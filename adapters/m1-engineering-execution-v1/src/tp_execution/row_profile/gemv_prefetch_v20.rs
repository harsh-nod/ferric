//! Exact two-shape TP1/C1 partial ABI, retaining padded resident tensors.

use super::{EngineeringTpArgumentV1, EngineeringTpDispatchV1, TpResult};
use crate::tp_execution::EngineeringTpBufferAccessV1::{Read, Write};

pub(super) fn bind(capacity: u32, large_kv: bool, command: EngineeringTpDispatchV1) -> TpResult<EngineeringTpDispatchV1> {
    let roots = crate::tp_artifact::ENGINEERING_TP_GEMV_PREFETCH_EXPORTS_V20;
    let Some([EngineeringTpArgumentV1::U32(rows), EngineeringTpArgumentV1::U32(n), EngineeringTpArgumentV1::U32(k),
        EngineeringTpArgumentV1::U32(world), EngineeringTpArgumentV1::U32(tag)]) = command.arguments.get(3..) else {
        return Err("V20 requires three slices and five u32 scalars".into());
    };
    let partial = command.kernel == roots[1];
    let shape = partial && matches!((*n, *k, *tag), (4096, 4096, 1) | (4096, 12288, 2));
    if capacity != 32 || large_kv || *rows != 1 || *world != 1 || !shape
        || command.workgroup_size != 64 || command.grid_workgroups != *n {
        return Err("V20 route requires one TP1 row, exact partial shape and one Wave64 per output".into());
    }
    let mut ids = [0; 3];
    for (index, argument) in command.arguments[..3].iter().enumerate() {
        let elements_expected = match index { 0 => 32 * *k as usize, 1 => *n as usize * *k as usize, _ => 32 * *n as usize };
        let bytes_expected = if index == 2 && partial { 4 } else { 2 };
        let EngineeringTpArgumentV1::Buffer { id, offset, elements, element_bytes, access } = argument else {
            return Err("V20 requires buffer arguments".into());
        };
        if *offset != 0 || *elements != elements_expected || *element_bytes != bytes_expected
            || *access != if index == 2 { Write } else { Read } {
            return Err("V20 buffer extent, element size or access differs".into());
        }
        ids[index] = *id;
    }
    if ids[2] == ids[0] || ids[2] == ids[1] {
        return Err("V20 output aliases a read input".into());
    }
    Ok(command)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tp_execution::{Tensor, batched::matrix};

    #[test]
    fn only_two_partial_shapes_keep_exact_abi_and_reject_mutations() {
        for (partial, n, k, tag) in [(false,4096,4096,1), (false,1024,4096,2), (false,1024,4096,3),
            (false,12288,4096,4), (false,12288,4096,5), (true,4096,4096,1), (true,4096,12288,2)] {
            let root = crate::tp_artifact::ENGINEERING_TP_GEMV_PREFETCH_EXPORTS_V20[usize::from(partial)];
            let mut command = matrix(root, Tensor { id:1, elements:32*k as usize, element_bytes:2 },
                Tensor { id:2, elements:n as usize*k as usize, element_bytes:2 },
                Tensor { id:3, elements:32*n as usize, element_bytes:if partial {4} else {2} }, [1,n,k,1,tag]);
            command.grid_workgroups = n;
            if !partial {
                assert!(bind(32, false, command).is_err());
                continue;
            }
            assert_eq!(bind(32, false, command.clone()).unwrap(), command);
            assert!(bind(16, false, command.clone()).is_err());
            assert!(bind(32, true, command.clone()).is_err());
            for mutation in 0..13 {
                let mut bad = command.clone();
                match mutation {
                    0 => bad.workgroup_size = 32, 1 => bad.grid_workgroups += 1,
                    2 => bad.arguments[3] = EngineeringTpArgumentV1::U32(2), 3 => bad.arguments[6] = EngineeringTpArgumentV1::U32(2),
                    4 => bad.arguments[7] = EngineeringTpArgumentV1::U32(6), 5 => bad.arguments[5] = EngineeringTpArgumentV1::U32(0),
                    6 => { bad.arguments.pop(); },
                    other => {
                        let index = if other == 12 { 2 } else { (other - 7) % 3 };
                        let EngineeringTpArgumentV1::Buffer { id, offset, elements, element_bytes, access } = &mut bad.arguments[index] else { unreachable!() };
                        match other { 7 => *offset = 2, 8 => *elements -= 1, 9 => *element_bytes = 0,
                            10 => *access = Write, 11 => *elements = usize::MAX, 12 => *id = 1, _ => unreachable!() }
                    }
                }
                assert!(bind(32, false, bad).is_err(), "shape {n}/{k}/{tag}, mutation {mutation}");
            }
        }
    }
}
