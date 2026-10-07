//! Closed gate/up-only routing, narrower than the emitted five-role r2 image.

use super::super::{
    EngineeringTpArgumentV1, EngineeringTpBufferAccessV1, EngineeringTpDispatchV1, TpResult,
};
use crate::tp_artifact::ENGINEERING_TP_PACKED_BF16_EXPORTS_R2 as ROOTS;

pub(super) fn bind(
    capacity: u32,
    large_kv: bool,
    command: EngineeringTpDispatchV1,
) -> TpResult<EngineeringTpDispatchV1> {
    use EngineeringTpArgumentV1::U32;
    use EngineeringTpBufferAccessV1::{Read, Write};
    if capacity != 32 || large_kv || command.workgroup_size != 64 {
        return Err("packed gate/up requires the separate TP1 capacity32 Wave64 profile".into());
    }
    let valid = if command.kernel == ROOTS[1] {
        command.grid_workgroups == 32
            && command.arguments.len() == 4
            && command.arguments[2..] == [U32(1), U32(4096)]
            && buffers(
                &command.arguments[..2],
                &[(4096, 2, Read), (2048, 4, Write)],
            )
    } else if command.kernel == ROOTS[0] {
        command.grid_workgroups == 12_288
            && command.arguments.len() == 8
            && command.arguments[3..7] == [U32(1), U32(12_288), U32(4096), U32(1)]
            && matches!(command.arguments[7], U32(4 | 5))
            && buffers(
                &command.arguments[..3],
                &[
                    (2048, 4, Read),
                    (12_288 * 2048, 4, Read),
                    (12_288, 2, Write),
                ],
            )
    } else {
        false
    };
    if !valid {
        return Err("packed gate/up role, slice, offset, alias or launch geometry drifted".into());
    }
    Ok(command)
}

fn buffers(
    arguments: &[EngineeringTpArgumentV1],
    expected: &[(usize, u32, EngineeringTpBufferAccessV1)],
) -> bool {
    arguments.len() == expected.len()
        && arguments.iter().zip(expected).enumerate().all(|(index, (argument, shape))| {
            let EngineeringTpArgumentV1::Buffer { id, offset, elements, element_bytes, access } = argument else {
                return false;
            };
            *offset == 0 && (*elements, *element_bytes, *access) == *shape
                && arguments[..index].iter().all(|other| {
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
            elements: if pack { 4096 } else { 2048 },
            element_bytes: if pack { 2 } else { 4 },
        };
        let b = Tensor {
            id: 2,
            elements: if pack { 2048 } else { 12_288 * 2048 },
            element_bytes: 4,
        };
        let c = Tensor {
            id: 3,
            elements: 12_288,
            element_bytes: 2,
        };
        if pack {
            dispatch(
                ROOTS[1],
                32,
                vec![
                    a.read(),
                    b.write(),
                    EngineeringTpArgumentV1::U32(1),
                    EngineeringTpArgumentV1::U32(4096),
                ],
            )
        } else {
            let mut arguments = vec![a.read(), b.read(), c.write()];
            arguments.extend([1, 12_288, 4096, 1, 4].map(EngineeringTpArgumentV1::U32));
            dispatch(ROOTS[0], 12_288, arguments)
        }
    }

    #[test]
    fn packed_gate_up_router_admits_only_exact_zero_offset_disjoint_typed_slices() {
        for pack in [false, true] {
            let mut good = command(pack);
            if !pack {
                good.grid_workgroups = 12_288;
            }
            assert_eq!(bind(32, false, good.clone()).unwrap(), good);
            assert!(bind(16, false, good.clone()).is_err());
            assert!(bind(32, true, good.clone()).is_err());
            for index in 0..if pack { 2 } else { 3 } {
                for mutation in 0..5 {
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
                        2 => *element_bytes = 8,
                        3 => *access = EngineeringTpBufferAccessV1::ReadWrite,
                        _ => *id = if index == 0 { 2 } else { 1 },
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
    fn packed_gate_up_router_rejects_qkv_partial_head_tp2_and_multirow_shapes() {
        let mut good = command(false);
        good.grid_workgroups = 12_288;
        for tag in 0..8 {
            let mut changed = good.clone();
            changed.arguments[7] = EngineeringTpArgumentV1::U32(tag);
            assert_eq!(bind(32, false, changed).is_ok(), matches!(tag, 4 | 5));
        }
        for (index, value) in [(3, 2), (4, 4096), (5, 12_288), (6, 2)] {
            let mut changed = good.clone();
            changed.arguments[index] = EngineeringTpArgumentV1::U32(value);
            assert!(bind(32, false, changed).is_err());
        }
        let mut changed = command(true);
        changed.arguments[3] = EngineeringTpArgumentV1::U32(12_288);
        assert!(bind(32, false, changed).is_err());
    }
}
