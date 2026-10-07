//! Closed down-only partial/merge ABI; no generic projection aliasing.

use super::super::{
    EngineeringTpArgumentV1, EngineeringTpBufferAccessV1, EngineeringTpDispatchV1, TpResult,
};
use crate::tp_artifact::ENGINEERING_TP_SPLITK_DOWN_EXPORTS_R1 as ROOTS;

pub(super) fn bind(
    capacity: u32,
    large_kv: bool,
    command: EngineeringTpDispatchV1,
) -> TpResult<EngineeringTpDispatchV1> {
    use EngineeringTpArgumentV1::U32;
    use EngineeringTpBufferAccessV1::{Read, Write};
    let valid = capacity == 32
        && !large_kv
        && command.workgroup_size == 64
        && if command.kernel == ROOTS[0] {
            command.grid_workgroups == 2048
                && command.arguments.len() == 8
                && command.arguments[3..] == [U32(1), U32(4096), U32(12_288), U32(1), U32(2)]
                && buffers(
                    &command.arguments[..3],
                    &[
                        (12_288, 2, Read),
                        (4096 * 12_288, 2, Read),
                        (32_768, 4, Write),
                    ],
                )
        } else if command.kernel == ROOTS[1] {
            command.grid_workgroups == 64
                && buffers(&command.arguments, &[(32_768, 4, Read), (4096, 4, Write)])
        } else {
            false
        };
    if !valid {
        return Err("split-K down role, buffer or launch geometry drifted".into());
    }
    Ok(command)
}

fn buffers(
    arguments: &[EngineeringTpArgumentV1],
    expected: &[(usize, u32, EngineeringTpBufferAccessV1)],
) -> bool {
    arguments.len() == expected.len() && arguments.iter().zip(expected).enumerate().all(|(index, (argument, shape))| {
        let EngineeringTpArgumentV1::Buffer { id, offset, elements, element_bytes, access } = argument else { return false; };
        *id != 0 && *offset == 0 && (*elements, *element_bytes, *access) == *shape && arguments[..index].iter().all(|other| {
            !matches!(other, EngineeringTpArgumentV1::Buffer { id: previous, .. } if previous == id)
        })
    })
}

#[cfg(test)]
mod tests {
    use super::super::super::{Tensor, dispatch};
    use super::*;

    fn command(merge: bool) -> EngineeringTpDispatchV1 {
        let a = Tensor {
            id: 1,
            elements: 12_288,
            element_bytes: 2,
        };
        let w = Tensor {
            id: 2,
            elements: 4096 * 12_288,
            element_bytes: 2,
        };
        let scratch = Tensor {
            id: 3,
            elements: 32_768,
            element_bytes: 4,
        };
        let out = Tensor {
            id: 4,
            elements: 4096,
            element_bytes: 4,
        };
        if merge {
            dispatch(ROOTS[1], 64, vec![scratch.read(), out.write()])
        } else {
            let mut args = vec![a.read(), w.read(), scratch.write()];
            args.extend([1, 4096, 12_288, 1, 2].map(EngineeringTpArgumentV1::U32));
            dispatch(ROOTS[0], 2048, args)
        }
    }

    #[test]
    fn splitk_router_closes_every_buffer_field_and_launch_dimension() {
        for merge in [false, true] {
            let good = command(merge);
            assert_eq!(bind(32, false, good.clone()).unwrap(), good);
            assert!(bind(16, false, good.clone()).is_err());
            assert!(bind(32, true, good.clone()).is_err());
            for index in 0..if merge { 2 } else { 3 } {
                for mutation in 0..7 {
                    let mut bad = good.clone();
                    let alias = match good.arguments[usize::from(index == 0)] {
                        EngineeringTpArgumentV1::Buffer { id, .. } => id,
                        _ => unreachable!(),
                    };
                    let EngineeringTpArgumentV1::Buffer {
                        id,
                        offset,
                        elements,
                        element_bytes,
                        access,
                    } = &mut bad.arguments[index]
                    else {
                        unreachable!()
                    };
                    match mutation {
                        0 => *id = alias,
                        1 => *offset = 4,
                        2 => *elements -= 1,
                        3 => *elements += 1,
                        4 => *element_bytes = if *element_bytes == 2 { 4 } else { 2 },
                        5 => *access = EngineeringTpBufferAccessV1::ReadWrite,
                        _ => *id = 0,
                    }
                    assert!(bind(32, false, bad).is_err());
                }
            }
            for mutation in 0..4 {
                let mut bad = good.clone();
                match mutation {
                    0 => bad.grid_workgroups += 1,
                    1 => bad.workgroup_size = 32,
                    2 => {
                        bad.arguments.pop();
                    }
                    _ => bad.arguments.push(EngineeringTpArgumentV1::U32(0)),
                }
                assert!(bind(32, false, bad).is_err());
            }
        }
    }

    #[test]
    fn splitk_router_refuses_other_rows_tp_projection_and_scalar_types() {
        for index in 3..8 {
            let mut bad = command(false);
            bad.arguments[index] = EngineeringTpArgumentV1::U32(0);
            assert!(bind(32, false, bad).is_err());
            let mut bad = command(false);
            bad.arguments[index] = EngineeringTpArgumentV1::F32(1.0);
            assert!(bind(32, false, bad).is_err());
        }
    }
}
