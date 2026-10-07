//! Narrow TP1/C1 two-packet attention ABI with owned, exact scratch views.

use super::{EngineeringTpArgumentV1, EngineeringTpDispatchV1, TpResult};
use crate::tp_execution::EngineeringTpBufferAccessV1::{Read, Write};

pub(super) fn bind(
    capacity: u32,
    large_kv: bool,
    command: EngineeringTpDispatchV1,
) -> TpResult<EngineeringTpDispatchV1> {
    let roots = crate::tp_artifact::ENGINEERING_TP_SPLIT_ATTENTION_EXPORTS_V21;
    let partial = command.kernel == roots[0];
    if capacity != 32
        || large_kv
        || !roots.contains(&command.kernel)
        || command.workgroup_size != 64
        || command.grid_workgroups != if partial { 256 } else { 32 }
    {
        return Err("V21 requires exact TP1/32-row storage and Wave64 partial/merge grids".into());
    }
    let shapes = if partial {
        let Some(
            [
                EngineeringTpArgumentV1::U32(rows),
                EngineeringTpArgumentV1::U32(world),
                EngineeringTpArgumentV1::U32(stride),
                EngineeringTpArgumentV1::U32(pages),
                EngineeringTpArgumentV1::U32(context),
            ],
        ) = command.arguments.get(7..)
        else {
            return Err("V21 partial requires seven slices and five u32 scalars".into());
        };
        if *rows != 1
            || *world != 1
            || !(1..=512).contains(stride)
            || !(1..=512).contains(pages)
            || !(128..=256).contains(context)
            || *context > *stride * 16
        {
            return Err("V21 partial requires one TP1 row and its actual bounded context".into());
        }
        vec![
            (32 * 4096, 2, Read),
            (*pages as usize * 16 * 1024, 2, Read),
            (*pages as usize * 16 * 1024, 2, Read),
            (32, 4, Read),
            (32 * *stride as usize, 4, Read),
            (512, 4, Write),
            (32_768, 4, Write),
        ]
    } else {
        if command.arguments.len() != 3 {
            return Err("V21 merge requires exactly three slices and no scalars".into());
        }
        vec![(512, 4, Read), (32_768, 4, Read), (32 * 4096, 2, Write)]
    };
    let mut buffers = Vec::with_capacity(shapes.len());
    for (argument, (expected_elements, expected_width, expected_access)) in
        command.arguments.iter().zip(shapes)
    {
        let EngineeringTpArgumentV1::Buffer {
            id,
            offset,
            elements,
            element_bytes,
            access,
        } = argument
        else {
            return Err("V21 requires exact typed buffer arguments".into());
        };
        if *offset != 0
            || *elements != expected_elements
            || *element_bytes != expected_width
            || *access != expected_access
        {
            return Err("V21 buffer extent, offset, element width or access drifted".into());
        }
        buffers.push((*id, *access));
    }
    for (index, (id, access)) in buffers.iter().enumerate() {
        if *access == Write
            && buffers
                .iter()
                .enumerate()
                .any(|(other, (other_id, _))| other != index && id == other_id)
        {
            return Err("V21 writable buffer aliases another argument".into());
        }
    }
    Ok(command)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tp_execution::{Tensor, dispatch};

    fn commands() -> [EngineeringTpDispatchV1; 2] {
        let tensor = |id, elements, element_bytes| Tensor {
            id,
            elements,
            element_bytes,
        };
        let stats = tensor(6, 512, 4);
        let numerators = tensor(7, 32_768, 4);
        let roots = crate::tp_artifact::ENGINEERING_TP_SPLIT_ATTENTION_EXPORTS_V21;
        [
            dispatch(
                roots[0],
                256,
                vec![
                    tensor(1, 32 * 4096, 2).read(),
                    tensor(2, 512 * 16 * 1024, 2).read(),
                    tensor(3, 512 * 16 * 1024, 2).read(),
                    tensor(4, 32, 4).read(),
                    tensor(5, 32 * 512, 4).read(),
                    stats.write(),
                    numerators.write(),
                    EngineeringTpArgumentV1::U32(1),
                    EngineeringTpArgumentV1::U32(1),
                    EngineeringTpArgumentV1::U32(512),
                    EngineeringTpArgumentV1::U32(512),
                    EngineeringTpArgumentV1::U32(192),
                ],
            ),
            dispatch(
                roots[1],
                32,
                vec![
                    stats.read(),
                    numerators.read(),
                    tensor(8, 32 * 4096, 2).write(),
                ],
            ),
        ]
    }

    #[test]
    fn both_packets_accept_exact_views_and_reject_every_buffer_mutation() {
        for (partial, command) in commands().into_iter().enumerate() {
            assert_eq!(bind(32, false, command.clone()).unwrap(), command);
            assert!(bind(16, false, command.clone()).is_err());
            assert!(bind(32, true, command.clone()).is_err());
            for index in 0..if partial == 0 { 7 } else { 3 } {
                for mutation in 0..5 {
                    let mut changed = command.clone();
                    let EngineeringTpArgumentV1::Buffer {
                        offset,
                        elements,
                        element_bytes,
                        access,
                        ..
                    } = &mut changed.arguments[index]
                    else {
                        unreachable!()
                    };
                    match mutation {
                        0 => *offset = 4,
                        1 => *elements = 0,
                        2 => *elements += 1,
                        3 => *element_bytes = 0,
                        4 => *access = if *access == Read { Write } else { Read },
                        _ => unreachable!(),
                    }
                    assert!(
                        bind(32, false, changed).is_err(),
                        "root={partial} arg={index} mutation={mutation}"
                    );
                }
            }
            for mutation in 0..4 {
                let mut changed = command.clone();
                match mutation {
                    0 => changed.grid_workgroups += 1,
                    1 => changed.workgroup_size = 32,
                    2 => {
                        changed.arguments.pop();
                    }
                    3 => changed.arguments.push(EngineeringTpArgumentV1::U32(0)),
                    _ => unreachable!(),
                }
                assert!(bind(32, false, changed).is_err());
            }
            let mut aliased = command;
            let index = if partial == 0 { 5 } else { 2 };
            let EngineeringTpArgumentV1::Buffer { id, .. } = &mut aliased.arguments[index] else {
                unreachable!()
            };
            *id = if partial == 0 { 1 } else { 6 };
            assert!(bind(32, false, aliased).is_err());
        }
    }

    #[test]
    fn actual_context_bounds_are_not_configured_context_capacity() {
        let original = commands()[0].clone();
        for context in [127, 128, 129, 191, 192, 193, 255, 256, 257, 8192] {
            let mut command = original.clone();
            command.arguments[11] = EngineeringTpArgumentV1::U32(context);
            assert_eq!(
                bind(32, false, command).is_ok(),
                (128..=256).contains(&context)
            );
        }
        for (index, value) in [(7, 2), (8, 8), (9, 0), (9, 513), (10, 0), (10, 513)] {
            let mut command = original.clone();
            command.arguments[index] = EngineeringTpArgumentV1::U32(value);
            assert!(bind(32, false, command).is_err());
        }
    }
}
