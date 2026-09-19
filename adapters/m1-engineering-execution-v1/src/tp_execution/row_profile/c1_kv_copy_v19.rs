use super::{EngineeringTpArgumentV1, EngineeringTpDispatchV1, TpResult};
use crate::tp_execution::EngineeringTpBufferAccessV1::{Read, Write};

pub(super) fn bind(
    capacity: u32,
    large_kv: bool,
    command: EngineeringTpDispatchV1,
) -> TpResult<EngineeringTpDispatchV1> {
    let Some(
        [
            EngineeringTpArgumentV1::U32(position),
            EngineeringTpArgumentV1::U32(page),
            EngineeringTpArgumentV1::U32(pages),
        ],
    ) = command.arguments.get(4..)
    else {
        return Err("C1 KV copy requires four slices and three bounded u32 scalars".into());
    };
    if capacity != 32
        || large_kv
        || *position >= 8192
        || !(1..=512).contains(pages)
        || *page >= *pages
        || command.grid_workgroups != 16
        || command.workgroup_size != 64
    {
        return Err("C1 KV copy requires the exact legacy TP1 slot and 16 Wave64 groups".into());
    }
    let expected_offset = (*page as usize * 16 + (*position % 16) as usize) * 1024 * 2;
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
            return Err("C1 KV copy requires exact buffer arguments".into());
        };
        if *offset != (if index < 2 { 0 } else { expected_offset })
            || *elements != 1024
            || *element_bytes != 2
            || *access != (if index < 2 { Read } else { Write })
        {
            return Err("C1 KV copy buffer extent/access/slot offset differs".into());
        }
        ids[index] = *id;
    }
    if ids[2] == ids[3] || ids[..2].contains(&ids[2]) || ids[..2].contains(&ids[3]) {
        return Err("C1 KV copy requires independently owned destination allocations".into());
    }
    Ok(command)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(position: u32, page: u32, pages: u32) -> EngineeringTpDispatchV1 {
        let offset = (page as usize * 16 + (position % 16) as usize) * 2048;
        let mut args = (0..4)
            .map(|index| EngineeringTpArgumentV1::Buffer {
                id: index + 1,
                offset: if index < 2 { 0 } else { offset },
                elements: 1024,
                element_bytes: 2,
                access: if index < 2 { Read } else { Write },
            })
            .collect::<Vec<_>>();
        args.extend([
            EngineeringTpArgumentV1::U32(position),
            EngineeringTpArgumentV1::U32(page),
            EngineeringTpArgumentV1::U32(pages),
        ]);
        crate::tp_execution::dispatch(
            crate::tp_artifact::ENGINEERING_TP_C1_KV_COPY_EXPORTS_V19[0],
            16,
            args,
        )
    }

    #[test]
    fn v19_route_checks_exact_slot_geometry_without_widening_other_profiles() {
        for (position, page, pages) in [(0, 0, 1), (15, 0, 1), (16, 1, 2), (8191, 511, 512)] {
            let command = command(position, page, pages);
            assert_eq!(
                super::super::bind_storage(32, false, command.clone()).unwrap(),
                command
            );
            assert!(super::super::bind_storage(16, false, command.clone()).is_err());
            assert!(super::super::bind_storage(32, true, command.clone()).is_err());
            assert!(super::super::bind_mode(true, 32, false, command).is_err());
        }
    }

    #[test]
    fn v19_route_rejects_scalar_launch_extent_access_alias_and_offset_mutations() {
        let valid = command(31, 2, 4);
        for mutation in 0..20 {
            let mut bad = valid.clone();
            match mutation {
                0 => bad.grid_workgroups = 1,
                1 => bad.workgroup_size = 32,
                2 => bad.arguments[4] = EngineeringTpArgumentV1::U32(8192),
                3 => bad.arguments[5] = EngineeringTpArgumentV1::U32(4),
                4 => bad.arguments[6] = EngineeringTpArgumentV1::U32(0),
                5 => bad.arguments[6] = EngineeringTpArgumentV1::U32(513),
                6 => {
                    bad.arguments.pop();
                }
                7 => bad.arguments.push(EngineeringTpArgumentV1::U32(1)),
                8 => bad.arguments[4] = EngineeringTpArgumentV1::F32(31.0),
                9 => bad.arguments[0] = EngineeringTpArgumentV1::U32(1),
                index => {
                    let arg = if index == 18 { 0 } else { 2 };
                    let EngineeringTpArgumentV1::Buffer {
                        id,
                        offset,
                        elements,
                        element_bytes,
                        access,
                    } = &mut bad.arguments[arg]
                    else {
                        unreachable!()
                    };
                    match index {
                        10 => *offset = 0,
                        11 => *offset += 2,
                        12 => *elements = 1023,
                        13 => *elements = 1025,
                        14 => *element_bytes = 4,
                        15 => *access = Read,
                        16 => *id = 4,
                        17 => *id = 1,
                        18 => *offset = 2,
                        19 => *offset = usize::MAX,
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
