//! Exact dispatch routing for the separately admitted 32-row image.

use super::{EngineeringTpArgumentV1, EngineeringTpDispatchV1, TpResult};

#[cfg(feature = "tp-batch-engineering")]
mod draft;

pub(super) fn bind_mode(
    draft_v10: bool,
    capacity: u32,
    large_kv: bool,
    command: EngineeringTpDispatchV1,
) -> TpResult<EngineeringTpDispatchV1> {
    if draft_v10 {
        if capacity != 32 || large_kv {
            return Err("draft v10 requires independent legacy-storage 32-row routing".into());
        }
        #[cfg(feature = "tp-batch-engineering")]
        return draft::bind(command);
        #[cfg(not(feature = "tp-batch-engineering"))]
        return Err("draft v10 requires explicit batched engineering support".into());
    }
    bind_storage(capacity, large_kv, command)
}

pub(super) fn bind_storage(
    capacity: u32,
    large_kv: bool,
    command: EngineeringTpDispatchV1,
) -> TpResult<EngineeringTpDispatchV1> {
    if large_kv && capacity != 32 {
        return Err("large KV routing requires the separately admitted 32-row profile".into());
    }
    let mut command = bind(capacity, command)?;
    if large_kv {
        command.kernel = match command.kernel {
            "ferric_qwen3_tp_batch32_paged_kv_append_v5" => {
                "ferric_qwen3_tp_batch32_large_kv_append_v9"
            }
            "ferric_qwen3_tp_batch32_paged_gqa_bf16_f32_v5" => {
                "ferric_qwen3_tp_batch32_large_kv_paged_gqa_bf16_f32_v9"
            }
            name if name.contains("_wave_") || name.contains("_peer_") => {
                return Err("large KV does not support wave or peer dispatches".into());
            }
            name => name,
        };
    }
    Ok(command)
}

pub(super) fn bind(
    capacity: u32,
    mut command: EngineeringTpDispatchV1,
) -> TpResult<EngineeringTpDispatchV1> {
    #[cfg(feature = "tp-batch-engineering")]
    if command.kernel == crate::tp_artifact::ENGINEERING_TP_WAVE_RMSNORM_EXPORTS_V15[0] {
        use super::EngineeringTpBufferAccessV1::{Read, Write};
        let Some(
            [
                EngineeringTpArgumentV1::U32(rows),
                EngineeringTpArgumentV1::U32(width),
                EngineeringTpArgumentV1::F32(epsilon),
                EngineeringTpArgumentV1::U32(behavior),
            ],
        ) = command.arguments.get(5..)
        else {
            return Err("wave RMSNorm v15 requires five slices and four exact scalars".into());
        };
        if capacity != 32 || !(1..=32).contains(rows) || *width != 4096
            || epsilon.to_bits() != 1.0e-6_f32.to_bits() || *behavior != 0
            || command.workgroup_size != 64 || command.grid_workgroups != *rows
            || command.arguments[..5].iter().enumerate().any(|(index, arg)| {
                let extent = match index { 0 | 4 => *rows as usize * 4096, 2 => 4096, _ => 0 };
                !matches!(arg, EngineeringTpArgumentV1::Buffer { offset, elements, element_bytes, access, .. }
                    if *offset == 0 && *elements == extent && *element_bytes == 2
                    && *access == (if index >= 3 { Write } else { Read }))
            })
        {
            return Err("wave RMSNorm v15 requires pure width4096/rows1..32, empty auxiliaries and exact Wave64 geometry".into());
        }
        return Ok(command);
    }
    #[cfg(feature = "tp-batch-engineering")]
    if command.kernel == crate::tp_artifact::ENGINEERING_TP_QUERY_HOIST_EXPORTS_V14[0] {
        use super::EngineeringTpBufferAccessV1::{Read, Write};
        let Some(
            [
                EngineeringTpArgumentV1::U32(rows),
                EngineeringTpArgumentV1::U32(world),
                EngineeringTpArgumentV1::U32(stride),
                EngineeringTpArgumentV1::U32(pages),
                EngineeringTpArgumentV1::U32(context),
            ],
        ) = command.arguments.get(6..)
        else {
            return Err("query hoist v14 requires six slices and five u32 scalars".into());
        };
        if capacity != 32
            || *world != 1
            || !(1..=32).contains(rows)
            || !(1..=512).contains(stride)
            || !(1..=512).contains(pages)
            || !(1..=8192).contains(context)
            || *context > *stride * 16
            || command.workgroup_size != 64
            || command.grid_workgroups != *rows * 32
            || command.arguments[..6]
                .iter()
                .enumerate()
                .any(|(index, arg)| {
                    !matches!(arg, EngineeringTpArgumentV1::Buffer { element_bytes, access, .. }
                    if *element_bytes == (if matches!(index, 3 | 4) { 4 } else { 2 })
                    && *access == (if index == 5 { Write } else { Read }))
                })
        {
            return Err(
                "query hoist v14 requires the exact TP1/capacity32 Wave64 GQA geometry".into(),
            );
        }
        return Ok(command);
    }
    if command.kernel == "ferric_qwen3_tp_batch32_wave_argmax_f32_v11" {
        let Some(EngineeringTpArgumentV1::U32(rows)) = command.arguments.get(2) else {
            return Err("argmax v11 row argument is missing".into());
        };
        if capacity != 32
            || command.workgroup_size != 64
            || command.arguments.len() != 3
            || !(1..=32).contains(rows)
            || command.grid_workgroups != *rows
        {
            return Err("argmax v11 requires exactly one Wave64 group per active row".into());
        }
        return Ok(command);
    }
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
        "ferric_qwen3_tp_head_bf16_f32_v7" => ("ferric_qwen3_tp_batch32_head_bf16_f32_v8", true),
        "ferric_qwen3_tp_mfma_head_f32_v7" => ("ferric_qwen3_tp_batch32_mfma_head_f32_v8", true),
        "ferric_qwen3_tp_argmax_f32_v7" => ("ferric_qwen3_tp_batch32_argmax_f32_v8", false),
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
    fn v11_route_requires_exact_wave64_rows_and_legacy_target_storage() {
        const ROOT: &str = "ferric_qwen3_tp_batch32_wave_argmax_f32_v11";
        for rows in [1, 16, 17, 32] {
            let command = super::super::dispatch(
                ROOT,
                rows,
                vec![
                    EngineeringTpArgumentV1::U32(0),
                    EngineeringTpArgumentV1::U32(0),
                    EngineeringTpArgumentV1::U32(rows),
                ],
            );
            assert_eq!(bind_storage(32, false, command.clone()).unwrap(), command);
            assert!(bind_storage(16, false, command.clone()).is_err());
            assert!(bind_storage(32, true, command.clone()).is_err());
            #[cfg(feature = "tp-batch-engineering")]
            assert!(bind_mode(true, 32, false, command.clone()).is_err());
            for mutation in 0..4 {
                let mut bad = command.clone();
                match mutation {
                    0 => bad.workgroup_size = 32,
                    1 => bad.grid_workgroups += 1,
                    2 => bad.arguments[2] = EngineeringTpArgumentV1::U32(0),
                    3 => {
                        bad.arguments.pop();
                    }
                    _ => unreachable!(),
                }
                assert!(bind_storage(32, false, bad).is_err());
            }
        }
    }

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

    #[test]
    fn fp32_head32_uses_two_tiles_only_above_sixteen_rows() {
        for (old, wide) in [
            (
                "ferric_qwen3_tp_head_bf16_f32_v7",
                "ferric_qwen3_tp_batch32_head_bf16_f32_v8",
            ),
            (
                "ferric_qwen3_tp_mfma_head_f32_v7",
                "ferric_qwen3_tp_batch32_mfma_head_f32_v8",
            ),
        ] {
            for rows in [0, 1, 16, 17, 31, 32, 33] {
                let command = super::super::dispatch(
                    old,
                    9496,
                    vec![
                        EngineeringTpArgumentV1::U32(0),
                        EngineeringTpArgumentV1::U32(0),
                        EngineeringTpArgumentV1::U32(0),
                        EngineeringTpArgumentV1::U32(rows),
                    ],
                );
                assert_eq!(bind(16, command.clone()).unwrap(), command);
                let result = bind(32, command);
                if (1..=32).contains(&rows) {
                    let result = result.unwrap();
                    assert_eq!(result.kernel, wide);
                    assert_eq!(result.grid_workgroups, rows.div_ceil(16) * 9496);
                } else {
                    assert!(result.is_err());
                }
            }
        }
        let command = super::super::dispatch("ferric_qwen3_tp_argmax_f32_v7", 32, vec![]);
        assert_eq!(bind(16, command.clone()).unwrap(), command);
        let wide = bind(32, command).unwrap();
        assert_eq!(wide.kernel, "ferric_qwen3_tp_batch32_argmax_f32_v8");
        assert_eq!(wide.grid_workgroups, 32);
    }
}
