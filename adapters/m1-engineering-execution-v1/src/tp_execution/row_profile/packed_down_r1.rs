//! Exact down-only FP32 routing, with disjoint zero-offset typed slices.

use super::super::{
    EngineeringTpArgumentV1, EngineeringTpBufferAccessV1, EngineeringTpDispatchV1, TpResult,
};
use crate::tp_artifact::ENGINEERING_TP_PACKED_DOWN_EXPORTS_R1 as ROOTS;

pub(super) fn bind(
    capacity: u32,
    large_kv: bool,
    command: EngineeringTpDispatchV1,
) -> TpResult<EngineeringTpDispatchV1> {
    use EngineeringTpArgumentV1::U32;
    use EngineeringTpBufferAccessV1::{Read, Write};
    if capacity != 32 || large_kv || command.workgroup_size != 64 {
        return Err("packed down requires the separate TP1 capacity32 Wave64 profile".into());
    }
    let valid = if command.kernel == ROOTS[1] {
        command.grid_workgroups == 96
            && command.arguments.len() == 4
            && command.arguments[2..] == [U32(1), U32(12_288)]
            && buffers(
                &command.arguments[..2],
                &[(12_288, 2, Read), (6144, 4, Write)],
            )
    } else if command.kernel == ROOTS[0] {
        command.grid_workgroups == 4096
            && command.arguments.len() == 8
            && command.arguments[3..] == [U32(1), U32(4096), U32(12_288), U32(1), U32(2)]
            && buffers(
                &command.arguments[..3],
                &[(6144, 4, Read), (4096 * 6144, 4, Read), (4096, 4, Write)],
            )
    } else {
        false
    };
    if !valid {
        return Err("packed down role, slice, offset, alias or launch geometry drifted".into());
    }
    Ok(command)
}

fn buffers(
    arguments: &[EngineeringTpArgumentV1],
    expected: &[(usize, u32, EngineeringTpBufferAccessV1)],
) -> bool {
    arguments.len() == expected.len() && arguments.iter().zip(expected).enumerate().all(|(index, (argument, shape))| {
        let EngineeringTpArgumentV1::Buffer { id, offset, elements, element_bytes, access } = argument else { return false; };
        *offset == 0 && (*elements, *element_bytes, *access) == *shape && arguments[..index].iter().all(|other| {
            !matches!(other, EngineeringTpArgumentV1::Buffer { id: previous, .. } if previous == id)
        })
    })
}

#[cfg(test)]
mod tests {
    use super::super::super::{Tensor, dispatch};
    use super::*;

    fn command(pack: bool) -> EngineeringTpDispatchV1 {
        let a = Tensor {
            id: 1,
            elements: if pack { 12_288 } else { 6144 },
            element_bytes: if pack { 2 } else { 4 },
        };
        let b = Tensor {
            id: 2,
            elements: if pack { 6144 } else { 4096 * 6144 },
            element_bytes: 4,
        };
        let c = Tensor {
            id: 3,
            elements: 4096,
            element_bytes: 4,
        };
        let mut arguments = if pack {
            vec![a.read(), b.write()]
        } else {
            vec![a.read(), b.read(), c.write()]
        };
        arguments.extend(
            if pack {
                vec![1, 12_288]
            } else {
                vec![1, 4096, 12_288, 1, 2]
            }
            .into_iter()
            .map(EngineeringTpArgumentV1::U32),
        );
        dispatch(
            ROOTS[usize::from(pack)],
            if pack { 96 } else { 4096 },
            arguments,
        )
    }

    #[test]
    fn packed_down_router_rejects_all_extent_type_offset_access_alias_and_launch_mutations() {
        for pack in [false, true] {
            let good = command(pack);
            assert_eq!(bind(32, false, good.clone()).unwrap(), good);
            assert!(bind(16, false, good.clone()).is_err());
            assert!(bind(32, true, good.clone()).is_err());
            for index in 0..if pack { 2 } else { 3 } {
                for mutation in 0..6 {
                    let mut bad = good.clone();
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
                        0 => *offset = 4,
                        1 => *elements += 1,
                        2 => *elements -= 1,
                        3 => *element_bytes = 2,
                        4 => *access = EngineeringTpBufferAccessV1::ReadWrite,
                        _ => *id = if index == 0 { 2 } else { 1 },
                    }
                    if mutation == 3 && pack && index == 0 {
                        *element_bytes = 4;
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
    fn packed_down_router_accepts_only_tp1_c1_down_fp32() {
        for (index, value) in [
            (3, 2),
            (4, 12_288),
            (5, 4096),
            (6, 2),
            (7, 0),
            (7, 1),
            (7, 3),
            (7, 4),
            (7, 5),
            (7, 6),
        ] {
            let mut bad = command(false);
            bad.arguments[index] = EngineeringTpArgumentV1::U32(value);
            assert!(bind(32, false, bad).is_err());
        }
        for (index, value) in [(2, 2), (3, 4096)] {
            let mut bad = command(true);
            bad.arguments[index] = EngineeringTpArgumentV1::U32(value);
            assert!(bind(32, false, bad).is_err());
        }
    }
}
