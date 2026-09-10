#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)]

//! Contracted engineering kernels for genuine multi-row Qwen3-8B TP1/2/8.
//! This additive unit does not alter or qualify either earlier kernel roster.

#[cfg(not(target_arch = "amdgpu"))]
extern crate alloc;
#[cfg(test)]
extern crate std;

#[path = "../../qwen3-all-kernels-v1/src/rmsnorm.rs"]
pub mod rmsnorm;
#[path = "../../qwen3-all-kernels-v1/src/target.rs"]
pub mod target;

pub mod activation;
pub mod attention;
pub mod contract;
pub mod embedding;
pub mod logits;
pub mod projection;
pub mod rope_kv;

/// Exact current compiler-generated roster, sorted by binding identity.
#[cfg(not(target_arch = "amdgpu"))]
pub fn compiler_expectation_roster_v2()
-> alloc::vec::Vec<fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1> {
    use fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1 as Entry;
    let mut entries = alloc::vec![
        Entry::for_marker::<rmsnorm::qwen3_rmsnorm_v1_gpu::Marker>(),
        Entry::for_marker::<embedding::ferric_qwen3_tp_batch_embedding_bf16_v2_gpu::Marker>(),
        Entry::for_marker::<projection::ferric_qwen3_tp_batch_gemm_bf16_f32_bf16_v2_gpu::Marker>(),
        Entry::for_marker::<projection::ferric_qwen3_tp_batch_gemm_partial_bf16_f32_v2_gpu::Marker>(
        ),
        Entry::for_marker::<activation::ferric_qwen3_tp_batch_swiglu_bf16_f32_v2_gpu::Marker>(),
        Entry::for_marker::<rope_kv::ferric_qwen3_tp_batch_rope_v2_gpu::Marker>(),
        Entry::for_marker::<rope_kv::ferric_qwen3_tp_batch_paged_kv_append_v2_gpu::Marker>(),
        Entry::for_marker::<attention::ferric_qwen3_tp_batch_paged_gqa_bf16_f32_v2_gpu::Marker>(),
        Entry::for_marker::<logits::ferric_qwen3_tp_batch_argmax_bf16_v2_gpu::Marker>(),
    ];
    entries.sort_by_key(Entry::kernel_binding_id);
    entries
}
