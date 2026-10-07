use super::{EngineeringTpArgumentV1, EngineeringTpDispatchV1, TpResult};
use crate::tp_execution::EngineeringTpBufferAccessV1::{Read, Write};

pub(super) fn bind(
    capacity: u32,
    large_kv: bool,
    command: EngineeringTpDispatchV1,
) -> TpResult<EngineeringTpDispatchV1> {
    let Some(
        [
            EngineeringTpArgumentV1::U32(first),
            EngineeringTpArgumentV1::U32(page),
            EngineeringTpArgumentV1::U32(pages),
            EngineeringTpArgumentV1::U32(rotation),
        ],
    ) = command.arguments.get(4..)
    else {
        return Err("prefill copy requires four slices and four bounded u32 scalars".into());
    };
    if capacity != 32
        || large_kv
        || *first > 8176
        || !first.is_multiple_of(16)
        || !(1..=512).contains(pages)
        || *page >= *pages
        || *rotation > 1
        || command.grid_workgroups != 256
        || command.workgroup_size != 64
    {
        return Err(
            "prefill copy requires an aligned bounded full page and 256 Wave64 groups".into(),
        );
    }
    let expected_offset = *page as usize * 16 * 1024 * 2;
    let mut ids = [0_u64; 4];
    for (index, argument) in command.arguments[..4].iter().enumerate() {
        let EngineeringTpArgumentV1::Buffer {
            id,
            offset,
            elements,
            element_bytes,
            access,
        } = argument
        else {
            return Err("prefill copy requires exact buffer arguments".into());
        };
        if *id == 0
            || *offset != (if index < 2 { 0 } else { expected_offset })
            || *elements != 16_384
            || *element_bytes != 2
            || *access != (if index < 2 { Read } else { Write })
        {
            return Err("prefill copy buffer extent/access/page offset differs".into());
        }
        ids[index] = *id;
    }
    if ids
        .iter()
        .enumerate()
        .any(|(index, id)| ids[index + 1..].contains(id))
    {
        return Err("prefill copy requires four distinct allocations".into());
    }
    Ok(command)
}

pub(super) fn bind_half(
    capacity: u32,
    large_kv: bool,
    source_elements: [usize; 2],
    command: EngineeringTpDispatchV1,
) -> TpResult<EngineeringTpDispatchV1> {
    if command.kernel != crate::tp_artifact::ENGINEERING_TP_PREFILL_KV_COPY_EXPORTS_V27[0]
        || source_elements != [32_768; 2]
    {
        return Err("prefill page half requires the actual 32-row source allocations".into());
    }
    let mut normalized = command.clone();
    let mut source_offset = None;
    for argument in normalized.arguments.iter_mut().take(2) {
        let EngineeringTpArgumentV1::Buffer { offset, .. } = argument else {
            return Err("prefill page half requires buffer sources".into());
        };
        if !matches!(*offset, 0 | 32_768)
            || source_offset.is_some_and(|previous| previous != *offset)
        {
            return Err("prefill page half requires equal exact half offsets".into());
        }
        source_offset = Some(*offset);
        *offset = 0;
    }
    // The legacy binder still independently requires offset zero. Only this
    // explicit half-view admission normalizes already bounded source views.
    bind(capacity, large_kv, normalized)?;
    Ok(command)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(first: u32, page: u32, pages: u32, rotation: u32) -> EngineeringTpDispatchV1 {
        let mut arguments = (0..4)
            .map(|index| EngineeringTpArgumentV1::Buffer {
                id: index + 1,
                offset: if index < 2 { 0 } else { page as usize * 32_768 },
                elements: 16_384,
                element_bytes: 2,
                access: if index < 2 { Read } else { Write },
            })
            .collect::<Vec<_>>();
        arguments.extend([first, page, pages, rotation].map(EngineeringTpArgumentV1::U32));
        crate::tp_execution::dispatch(
            crate::tp_artifact::ENGINEERING_TP_PREFILL_KV_COPY_EXPORTS_V27[0],
            256,
            arguments,
        )
    }

    #[test]
    fn v27_prefill_row_profile_preserves_exact_page_views_and_both_orders() {
        for (first, page, pages) in [(0, 0, 1), (16, 2, 3), (8176, 511, 512)] {
            for rotation in [0, 1] {
                let command = command(first, page, pages, rotation);
                assert_eq!(
                    super::super::bind_storage(32, false, command.clone()).unwrap(),
                    command
                );
                assert!(super::super::bind_storage(16, false, command.clone()).is_err());
                assert!(super::super::bind_storage(32, true, command.clone()).is_err());
                assert!(super::super::bind_mode(true, 32, false, command).is_err());
            }
        }
    }

    #[test]
    fn v27_prefill_row_profile_rejects_all_scalar_launch_view_and_alias_drift() {
        for mutation in 0..24 {
            let mut bad = command(16, 2, 4, 1);
            match mutation {
                0 => bad.grid_workgroups = 1,
                1 => bad.workgroup_size = 32,
                2 => bad.arguments[4] = EngineeringTpArgumentV1::U32(17),
                3 => bad.arguments[4] = EngineeringTpArgumentV1::U32(8192),
                4 => bad.arguments[5] = EngineeringTpArgumentV1::U32(4),
                5 => bad.arguments[6] = EngineeringTpArgumentV1::U32(0),
                6 => bad.arguments[6] = EngineeringTpArgumentV1::U32(513),
                7 => bad.arguments[7] = EngineeringTpArgumentV1::U32(2),
                8 => {
                    bad.arguments.pop();
                }
                9 => bad.arguments.push(EngineeringTpArgumentV1::U32(1)),
                10 => bad.arguments[4] = EngineeringTpArgumentV1::F32(16.0),
                11 => bad.arguments[0] = EngineeringTpArgumentV1::U32(1),
                index => {
                    let argument = if index >= 21 { 0 } else { 2 };
                    let EngineeringTpArgumentV1::Buffer {
                        id,
                        offset,
                        elements,
                        element_bytes,
                        access,
                    } = &mut bad.arguments[argument]
                    else {
                        unreachable!()
                    };
                    match index {
                        12 => *offset = 0,
                        13 => *offset += 2,
                        14 => *elements = 16_383,
                        15 => *elements = 16_385,
                        16 => *element_bytes = 4,
                        17 => *access = Read,
                        18 => *id = 4,
                        19 => *id = 1,
                        20 => *id = 0,
                        21 => *offset = 2,
                        22 => *id = 2,
                        23 => *elements = 0,
                        _ => unreachable!(),
                    }
                }
            }
            assert!(
                super::super::bind_storage(32, false, bad).is_err(),
                "mutation {mutation}"
            );
        }
    }
}
