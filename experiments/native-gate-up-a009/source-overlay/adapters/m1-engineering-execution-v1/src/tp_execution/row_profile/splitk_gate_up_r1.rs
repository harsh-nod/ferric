//! Closed C1 gate/up split-K4 partial and BF16 merge ABI.

use super::super::{
    EngineeringTpArgumentV1, EngineeringTpBufferAccessV1, EngineeringTpDispatchV1, TpResult,
};
use crate::tp_artifact::ENGINEERING_TP_SPLITK_GATE_UP_EXPORTS_R1 as ROOTS;

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
            command.grid_workgroups == 3072
                && command.arguments.len() == 8
                && command.arguments[3..7] == [U32(1), U32(12_288), U32(4096), U32(1)]
                && matches!(command.arguments[7], U32(4 | 5))
                && buffers(
                    &command.arguments[..3],
                    &[
                        (4096, 2, Read),
                        (4096 * 12_288, 2, Read),
                        (4 * 12_288, 4, Write),
                    ],
                )
        } else if command.kernel == ROOTS[1] {
            command.grid_workgroups == 192
                && buffers(
                    &command.arguments,
                    &[(4 * 12_288, 4, Read), (12_288, 2, Write)],
                )
        } else {
            false
        };
    if !valid {
        return Err("split-K gate/up role, buffer or launch geometry drifted".into());
    }
    Ok(command)
}

fn buffers(
    arguments: &[EngineeringTpArgumentV1],
    expected: &[(usize, u32, EngineeringTpBufferAccessV1)],
) -> bool {
    arguments.len() == expected.len()
        && arguments.iter().zip(expected).enumerate().all(|(index, (argument, shape))| {
            let EngineeringTpArgumentV1::Buffer {
                id, offset, elements, element_bytes, access,
            } = argument else {
                return false;
            };
            *id != 0 && *offset == 0
                && (*elements, *element_bytes, *access) == *shape
                && arguments[..index].iter().all(|other| {
                    !matches!(other, EngineeringTpArgumentV1::Buffer { id: previous, .. } if previous == id)
                })
        })
}

#[cfg(test)]
mod tests {
    use super::super::super::{Tensor, dispatch};
    use super::*;

    fn command(projection: Option<u32>) -> EngineeringTpDispatchV1 {
        let a = Tensor {
            id: 1,
            elements: 4096,
            element_bytes: 2,
        };
        let w = Tensor {
            id: 2,
            elements: 4096 * 12_288,
            element_bytes: 2,
        };
        let scratch = Tensor {
            id: 3,
            elements: 4 * 12_288,
            element_bytes: 4,
        };
        let out = Tensor {
            id: 4,
            elements: 12_288,
            element_bytes: 2,
        };
        if let Some(tag) = projection {
            let mut arguments = vec![a.read(), w.read(), scratch.write()];
            arguments.extend([1, 12_288, 4096, 1, tag].map(EngineeringTpArgumentV1::U32));
            dispatch(ROOTS[0], 3072, arguments)
        } else {
            dispatch(ROOTS[1], 192, vec![scratch.read(), out.write()])
        }
    }

    #[test]
    fn splitk_gate_up_router_closes_every_buffer_field_and_launch_dimension() {
        for tag in [Some(4), Some(5), None] {
            let good = command(tag);
            assert_eq!(bind(32, false, good.clone()).unwrap(), good);
            assert!(bind(16, false, good.clone()).is_err());
            assert!(bind(32, true, good.clone()).is_err());
            for index in 0..if tag.is_some() { 3 } else { 2 } {
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
            for mutation in 0..5 {
                let mut bad = good.clone();
                match mutation {
                    0 => bad.grid_workgroups += 1,
                    1 => bad.workgroup_size = 32,
                    2 => {
                        bad.arguments.pop();
                    }
                    3 => bad.arguments.push(EngineeringTpArgumentV1::U32(0)),
                    _ => bad.kernel = "unknown",
                }
                assert!(bind(32, false, bad).is_err());
            }
        }
    }

    #[test]
    fn splitk_gate_up_router_refuses_other_rows_tp_projection_and_scalar_types() {
        for tag in [4, 5] {
            for index in 3..8 {
                let mut bad = command(Some(tag));
                bad.arguments[index] = EngineeringTpArgumentV1::U32(0);
                assert!(bind(32, false, bad).is_err());
                let mut bad = command(Some(tag));
                bad.arguments[index] = EngineeringTpArgumentV1::F32(1.0);
                assert!(bind(32, false, bad).is_err());
            }
        }
        for tag in [0, 2, 3, 6] {
            assert!(bind(32, false, command(Some(tag))).is_err());
        }
    }

    #[test]
    fn splitk_gate_up_router_requires_short_views_not_capacity_tails() {
        for (tag, index) in [(Some(4), 0), (Some(5), 0), (None, 1)] {
            let mut bad = command(tag);
            let EngineeringTpArgumentV1::Buffer { elements, .. } = &mut bad.arguments[index] else {
                unreachable!()
            };
            *elements *= 32;
            assert!(bind(32, false, bad).is_err());
        }
    }
}
