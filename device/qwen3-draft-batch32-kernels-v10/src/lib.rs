#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)]

//! Independent Draft06B TP1/32-row engineering kernels, not target aliases.
//! A closed roster and geometry metadata do not confer model or KV authority.

#[cfg(not(feature = "gfx950"))]
compile_error!("the v10 draft32 profile requires gfx950");

#[cfg(not(target_arch = "amdgpu"))]
extern crate alloc;
#[cfg(test)]
extern crate std;

#[cfg(not(test))]
pub mod activation;
pub mod attention;
pub mod collective;
pub mod contract;
#[cfg(not(test))]
pub mod embedding;
pub mod head;
pub mod logits;
pub mod mfma;
pub mod projection;
#[cfg(not(test))]
pub mod rmsnorm;
pub mod rope_kv;

pub use contract::ROOTS_V10;

#[cfg(all(not(target_arch = "amdgpu"), not(test)))]
pub fn compiler_expectation_roster_v10()
-> alloc::vec::Vec<fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1> {
    use fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1 as Entry;
    let mut entries = alloc::vec![
        Entry::for_marker::<rmsnorm::ferric_qwen3_draft_batch32_rmsnorm_v10_gpu::Marker>(),
        Entry::for_marker::<embedding::ferric_qwen3_draft_batch32_embedding_bf16_v10_gpu::Marker>(),
        Entry::for_marker::<
            projection::ferric_qwen3_draft_batch32_gemm_bf16_f32_bf16_v10_gpu::Marker,
        >(),
        Entry::for_marker::<mfma::ferric_qwen3_draft_batch32_mfma_gemm_bf16_v10_gpu::Marker>(),
        Entry::for_marker::<
            projection::ferric_qwen3_draft_batch32_gemm_partial_bf16_f32_v10_gpu::Marker,
        >(),
        Entry::for_marker::<mfma::ferric_qwen3_draft_batch32_mfma_gemm_partial_f32_v10_gpu::Marker>(
        ),
        Entry::for_marker::<activation::ferric_qwen3_draft_batch32_swiglu_bf16_f32_v10_gpu::Marker>(
        ),
        Entry::for_marker::<rope_kv::ferric_qwen3_draft_batch32_rope_v10_gpu::Marker>(),
        Entry::for_marker::<rope_kv::ferric_qwen3_draft_batch32_paged_kv_append_v10_gpu::Marker>(),
        Entry::for_marker::<attention::ferric_qwen3_draft_batch32_paged_gqa_bf16_f32_v10_gpu::Marker>(
        ),
        Entry::for_marker::<collective::ferric_qwen3_draft_batch32_residual_bf16_v10_gpu::Marker>(),
        Entry::for_marker::<head::ferric_qwen3_draft_batch32_head_bf16_f32_v10_gpu::Marker>(),
        Entry::for_marker::<head::ferric_qwen3_draft_batch32_mfma_head_f32_v10_gpu::Marker>(),
        Entry::for_marker::<logits::ferric_qwen3_draft_batch32_argmax_f32_v10_gpu::Marker>(),
    ];
    entries.sort_by_key(Entry::kernel_binding_id);
    entries
}
