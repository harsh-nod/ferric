//! Exact dispatch routing for the separately admitted 32-row image.

use super::{EngineeringTpArgumentV1, EngineeringTpDispatchV1, TpResult};

pub(super) fn bind(
    capacity: u32,
    mut command: EngineeringTpDispatchV1,
) -> TpResult<EngineeringTpDispatchV1> {
    if capacity != 32 {
        return Ok(command);
    }
    let (kernel, tiled_projection) = match command.kernel {
        "ferric_qwen3_tp_peer_ordered_residual_bf16_v4" => (
            "ferric_qwen3_tp_batch32_peer_ordered_residual_bf16_v6",
            false,
        ),
        "ferric_qwen3_tp_peer_copy_bf16_v4" => ("ferric_qwen3_tp_batch32_peer_copy_bf16_v6", false),
        "qwen3_rmsnorm_v1" => ("qwen3_rmsnorm_v1", false),
        "ferric_qwen3_tp_batch_embedding_bf16_v2" => {
            ("ferric_qwen3_tp_batch32_embedding_bf16_v5", false)
        }
        "ferric_qwen3_tp_batch_gemm_bf16_f32_bf16_v2" => {
            ("ferric_qwen3_tp_batch32_gemm_bf16_f32_bf16_v5", true)
        }
        "ferric_qwen3_tp_batch_gemm_partial_bf16_f32_v2" => {
            ("ferric_qwen3_tp_batch32_gemm_partial_bf16_f32_v5", true)
        }
        "ferric_qwen3_tp_batch_swiglu_bf16_f32_v2" => {
            ("ferric_qwen3_tp_batch32_swiglu_bf16_f32_v5", false)
        }
        "ferric_qwen3_tp_batch_rope_v2" => ("ferric_qwen3_tp_batch32_rope_v5", false),
        "ferric_qwen3_tp_batch_paged_kv_append_v2" => {
            ("ferric_qwen3_tp_batch32_paged_kv_append_v5", false)
        }
        "ferric_qwen3_tp_batch_paged_gqa_bf16_f32_v2" => {
            ("ferric_qwen3_tp_batch32_paged_gqa_bf16_f32_v5", false)
        }
        "ferric_qwen3_tp_batch_argmax_bf16_v2" => ("ferric_qwen3_tp_batch32_argmax_bf16_v5", false),
        "ferric_qwen3_tp_batch_residual_bf16_v3" => {
            ("ferric_qwen3_tp_batch32_residual_bf16_v5", false)
        }
        "ferric_qwen3_tp_wave_gemv_bf16_v3" => ("ferric_qwen3_tp_batch32_wave_gemv_bf16_v5", false),
        "ferric_qwen3_tp_wave_gemv_partial_f32_v3" => {
            ("ferric_qwen3_tp_batch32_wave_gemv_partial_f32_v5", false)
        }
        "ferric_qwen3_tp_wave_paged_gqa_bf16_v3" => {
            ("ferric_qwen3_tp_batch32_wave_paged_gqa_bf16_v5", false)
        }
        "ferric_qwen3_tp_mfma_gemm_bf16_v3" => ("ferric_qwen3_tp_batch32_mfma_gemm_bf16_v5", true),
        "ferric_qwen3_tp_mfma_gemm_partial_f32_v3" => {
            ("ferric_qwen3_tp_batch32_mfma_gemm_partial_f32_v5", true)
        }
        _ => return Err("kernel is not in the separately admitted 32-row profile".into()),
    };
    command.kernel = kernel;
    if tiled_projection {
        let Some(EngineeringTpArgumentV1::U32(rows)) = command.arguments.get(3) else {
            return Err("32-row projection argument geometry drifted".into());
        };
        if !(1..=32).contains(rows) {
            return Err("32-row projection active extent exceeded".into());
        }
        command.grid_workgroups = command
            .grid_workgroups
            .checked_mul(rows.div_ceil(16))
            .ok_or("32-row projection grid overflow")?;
    }
    Ok(command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_tiles_are_one_physical_grid_and_old_profiles_are_unchanged() {
        for rows in [1, 16, 17, 31, 32] {
            let command = super::super::dispatch(
                "ferric_qwen3_tp_batch_gemm_bf16_f32_bf16_v2",
                32,
                vec![
                    EngineeringTpArgumentV1::U32(0),
                    EngineeringTpArgumentV1::U32(0),
                    EngineeringTpArgumentV1::U32(0),
                    EngineeringTpArgumentV1::U32(rows),
                ],
            );
            let legacy = bind(16, command.clone()).unwrap();
            assert_eq!(legacy.kernel, command.kernel);
            assert_eq!(legacy.grid_workgroups, 32);
            let wide = bind(32, command).unwrap();
            assert_eq!(wide.kernel, "ferric_qwen3_tp_batch32_gemm_bf16_f32_bf16_v5");
            assert_eq!(wide.grid_workgroups, 32 * rows.div_ceil(16));
        }
        assert!(bind(32, super::super::dispatch("foreign", 1, vec![])).is_err());
        assert!(
            bind(
                32,
                super::super::dispatch("ferric_qwen3_tp_mfma_gemm_bf16_v3", 1, vec![])
            )
            .is_err()
        );
    }
}
