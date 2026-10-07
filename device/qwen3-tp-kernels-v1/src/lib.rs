#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)]

//! Engineering-only, single-sequence Qwen TP1/2/8 device compilation unit.
//! The protected M1 aggregate and its admission policy remain unchanged.

#[cfg(not(target_arch = "amdgpu"))]
extern crate alloc;
#[cfg(test)]
extern crate std;

#[path = "../../qwen3-all-kernels-v1/src/gemm.rs"]
pub mod gemm;
#[path = "../../qwen3-all-kernels-v1/src/logits.rs"]
pub mod logits;
#[path = "../../qwen3-all-kernels-v1/src/rmsnorm.rs"]
pub mod rmsnorm;
#[path = "../../qwen3-all-kernels-v1/src/target.rs"]
pub mod target;

#[macro_use]
mod geometry;
pub mod activation;
pub mod attention;
pub mod contract;
pub mod projection;
pub mod rope_kv;

/// Exact metadata from all fourteen current compiler-generated markers.
#[cfg(not(target_arch = "amdgpu"))]
pub fn compiler_expectation_roster_v1()
-> alloc::vec::Vec<fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1> {
    use fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1 as Entry;
    let mut entries = alloc::vec![
        Entry::for_marker::<gemm::ferric_qwen3_gemm_mfma_bf16_f32_bf16_v1_gpu::Marker>(),
        Entry::for_marker::<gemm::ferric_qwen3_gemm_reference_bf16_f32_bf16_v1_gpu::Marker>(),
        Entry::for_marker::<gemm::ferric_qwen3_gemm_vector_a4_bf16_f32_bf16_v1_gpu::Marker>(),
        Entry::for_marker::<gemm::ferric_qwen3_token_embedding_bf16_copy_v1_gpu::Marker>(),
        Entry::for_marker::<logits::ferric_qwen3_compact_completion_v1_gpu::Marker>(),
        Entry::for_marker::<logits::ferric_qwen3_lowest_id_argmax_bf16_v1_gpu::Marker>(),
        Entry::for_marker::<logits::ferric_qwen3_speculative_token_assembly_v1_gpu::Marker>(),
        Entry::for_marker::<rmsnorm::qwen3_rmsnorm_v1_gpu::Marker>(),
        Entry::for_marker::<projection::ferric_qwen3_tp_gemv_bf16_f32_bf16_v1_gpu::Marker>(),
        Entry::for_marker::<projection::ferric_qwen3_tp_gemv_partial_bf16_f32_v1_gpu::Marker>(),
        Entry::for_marker::<activation::ferric_qwen3_tp_swiglu_bf16_f32_v1_gpu::Marker>(),
        Entry::for_marker::<rope_kv::ferric_qwen3_tp_rope_v1_gpu::Marker>(),
        Entry::for_marker::<rope_kv::ferric_qwen3_tp_kv_append_v1_gpu::Marker>(),
        Entry::for_marker::<attention::ferric_qwen3_tp_gqa_decode_bf16_f32_v1_gpu::Marker>(),
    ];
    entries.sort_by_key(Entry::kernel_binding_id);
    entries
}
