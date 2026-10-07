//! Closed engineering geometry; no protected admission authority.

pub use crate::baseline_contract::*;

pub const PERFORMANCE_ROOTS: [&str; 6] = [
    "ferric_qwen3_tp_wave_gemv_bf16_v3",
    "ferric_qwen3_tp_wave_gemv_partial_f32_v3",
    "ferric_qwen3_tp_mfma_gemm_bf16_v3",
    "ferric_qwen3_tp_mfma_gemm_partial_f32_v3",
    "ferric_qwen3_tp_wave_paged_gqa_bf16_v3",
    "ferric_qwen3_tp_batch_residual_bf16_v3",
];
