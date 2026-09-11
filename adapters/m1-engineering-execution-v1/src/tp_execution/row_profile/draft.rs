//! Resolves internal forward operations only to the independent closed draft image.

use super::{EngineeringTpArgumentV1, EngineeringTpDispatchV1, TpResult};
use ferric_qwen3_draft_batch32_kernels_device_v10::contract::{self, ProjectionRole, ROOTS_V10};

fn scalar(command: &EngineeringTpDispatchV1, index: usize) -> TpResult<u32> {
    match command.arguments.get(index) {
        Some(EngineeringTpArgumentV1::U32(value)) => Ok(*value),
        _ => Err("draft v10 scalar ABI mismatch".into()),
    }
}

pub(super) fn bind(mut command: EngineeringTpDispatchV1) -> TpResult<EngineeringTpDispatchV1> {
    let index = match command.kernel {
        "qwen3_rmsnorm_v1" => 0,
        "ferric_qwen3_tp_batch_embedding_bf16_v2" => 1,
        "ferric_qwen3_tp_batch_gemm_bf16_f32_bf16_v2" => 2,
        "ferric_qwen3_tp_mfma_gemm_bf16_v3" => 3,
        "ferric_qwen3_tp_batch_gemm_partial_bf16_f32_v2" => 4,
        "ferric_qwen3_tp_mfma_gemm_partial_f32_v3" => 5,
        "ferric_qwen3_tp_batch_swiglu_bf16_f32_v2" => 6,
        "ferric_qwen3_tp_batch_rope_v2" => 7,
        "ferric_qwen3_tp_batch_paged_kv_append_v2" => 8,
        "ferric_qwen3_tp_batch_paged_gqa_bf16_f32_v2" => 9,
        "ferric_qwen3_tp_batch_residual_bf16_v3" => 10,
        "ferric_qwen3_tp_head_bf16_f32_v7" => 11,
        "ferric_qwen3_tp_mfma_head_f32_v7" => 12,
        "ferric_qwen3_tp_argmax_f32_v7" => 13,
        _ => {
            return Err(
                "operation is absent from the separately admitted draft v10 profile".into(),
            );
        }
    };
    if command.workgroup_size != 64 {
        return Err("draft v10 workgroup mismatch".into());
    }
    let arguments = [9, 4, 8, 8, 8, 8, 5, 9, 10, 11, 4, 8, 8, 3][index];
    if command.arguments.len() != arguments {
        return Err("draft v10 argument count mismatch".into());
    }
    let grid = match index {
        2..=5 | 11 | 12 => {
            let rows = scalar(&command, 3)?;
            let n = scalar(&command, 4)?;
            let k = scalar(&command, 5)?;
            let world = scalar(&command, 6)?;
            let tag = scalar(&command, 7)?;
            let role = match (index, tag) {
                (2 | 3, 1) => ProjectionRole::Query,
                (2 | 3, 2) => ProjectionRole::Key,
                (2 | 3, 3) => ProjectionRole::Value,
                (2 | 3, 4) => ProjectionRole::Gate,
                (2 | 3, 5) => ProjectionRole::Up,
                (4 | 5, 1) => ProjectionRole::AttentionOutput,
                (4 | 5, 2) => ProjectionRole::Down,
                (11 | 12, 6) => ProjectionRole::Head,
                _ => return Err("draft v10 projection role mismatch".into()),
            };
            if !role.accepts(rows, world, n, k, tag) || command.grid_workgroups != n / 16 {
                return Err("draft v10 projection geometry mismatch".into());
            }
            role.grid(rows, world)
                .ok_or("draft v10 tiled grid overflow")?[0]
        }
        0 => {
            let rows = scalar(&command, 5)?;
            if !contract::norm_shape_is_supported(rows, scalar(&command, 6)?, scalar(&command, 8)?)
            {
                return Err("draft v10 pure normalization geometry mismatch".into());
            }
            rows
        }
        _ => {
            let row_index = match index {
                7 => 7,
                8 | 9 => 6,
                13 => 2,
                _ => 3,
            };
            let rows = scalar(&command, row_index)?;
            if !contract::rows_are_supported(rows) {
                return Err("draft v10 row limit exceeded".into());
            }
            match index {
                6 if scalar(&command, 4)? != 1 => return Err("draft v10 requires TP1".into()),
                7 if scalar(&command, 8)? != 1 => return Err("draft v10 requires TP1".into()),
                8 | 9 => {
                    if !contract::paged_limits_are_supported(
                        rows,
                        scalar(&command, 7)?,
                        scalar(&command, 9)?,
                        scalar(&command, 8)?,
                    ) {
                        return Err("draft v10 physical/logical page bounds mismatch".into());
                    }
                    if index == 9 && !(1..=8192).contains(&scalar(&command, 10)?) {
                        return Err("draft v10 context bound exceeded".into());
                    }
                }
                _ => {}
            }
            match index {
                1 | 9 | 10 => rows * 16,
                6 => rows * 48,
                8 => 1,
                _ => rows,
            }
        }
    };
    if !matches!(index, 2..=5 | 11 | 12) && command.grid_workgroups != grid {
        return Err("draft v10 exact launch grid mismatch".into());
    }
    if grid == 0 || grid > contract::MAX_GRID_WORKGROUPS[index] {
        return Err("draft v10 launch bound exceeded".into());
    }
    command.kernel = ROOTS_V10[index];
    command.grid_workgroups = grid;
    Ok(command)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn projection() -> EngineeringTpDispatchV1 {
        EngineeringTpDispatchV1 {
            kernel: "ferric_qwen3_tp_mfma_gemm_bf16_v3",
            grid_workgroups: 128,
            workgroup_size: 64,
            arguments: vec![EngineeringTpArgumentV1::U32(0); 3]
                .into_iter()
                .chain([17, 2048, 1024, 1, 1].map(EngineeringTpArgumentV1::U32))
                .collect(),
        }
    }

    #[test]
    fn projection_grid_and_role_are_validated_before_exact_draft_routing() {
        let command = bind(projection()).unwrap();
        assert_eq!(command.kernel, ROOTS_V10[3]);
        assert_eq!(command.grid_workgroups, 256);
        for (index, value) in [(3, 0), (3, 33), (4, 4096), (5, 4096), (6, 2), (7, 2)] {
            let mut command = projection();
            command.arguments[index] = EngineeringTpArgumentV1::U32(value);
            assert!(bind(command).is_err());
        }
        for case in 0..4 {
            let mut command = projection();
            match case {
                0 => command.kernel = "ferric_qwen3_tp_wave_gemv_bf16_v3",
                1 => command.grid_workgroups = 256,
                2 => command.workgroup_size = 128,
                _ => {
                    command.arguments.pop();
                }
            }
            assert!(bind(command).is_err());
        }
    }

    #[test]
    fn paged_geometry_never_inherits_target_or_large_pool_capabilities() {
        let mut command = EngineeringTpDispatchV1 {
            kernel: "ferric_qwen3_tp_batch_paged_gqa_bf16_f32_v2",
            grid_workgroups: 512,
            workgroup_size: 64,
            arguments: vec![EngineeringTpArgumentV1::U32(0); 6]
                .into_iter()
                .chain([32, 1, 512, 512, 8192].map(EngineeringTpArgumentV1::U32))
                .collect(),
        };
        assert_eq!(bind(command.clone()).unwrap().kernel, ROOTS_V10[9]);
        for (index, value) in [(6, 33), (7, 2), (8, 513), (9, 513), (10, 8193), (10, 0)] {
            let mut bad = command.clone();
            bad.arguments[index] = EngineeringTpArgumentV1::U32(value);
            assert!(bind(bad).is_err());
        }
        command.kernel = "ferric_qwen3_tp_wave_paged_gqa_bf16_v3";
        assert!(bind(command).is_err());
    }
}
