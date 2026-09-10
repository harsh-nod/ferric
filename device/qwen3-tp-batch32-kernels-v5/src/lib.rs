#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)]

//! Independent gfx950 32-row kernels; the original 16-row profiles remain frozen.
//! Reassociated FP32 reductions require separate numerical qualification.

#[cfg(not(feature = "gfx950"))]
compile_error!("the v5 batch32 profile requires gfx950");

#[cfg(not(target_arch = "amdgpu"))]
extern crate alloc;
#[cfg(test)]
extern crate std;

#[cfg(not(test))]
pub mod activation;
#[cfg(not(test))]
pub mod baseline_attention;
pub mod baseline_contract;
#[cfg(not(test))]
pub mod baseline_projection;
#[cfg(not(test))]
pub mod embedding;
#[cfg(not(test))]
pub mod logits;
#[path = "../../qwen3-all-kernels-v1/src/rmsnorm.rs"]
#[cfg(not(test))]
pub mod rmsnorm;
#[cfg(not(test))]
pub mod rope_kv;
#[path = "../../qwen3-all-kernels-v1/src/target.rs"]
pub mod target;

pub mod attention;
pub mod collective;
pub mod contract;
pub mod projection;

#[cfg(all(not(target_arch = "amdgpu"), not(test)))]
pub fn compiler_expectation_roster_v5()
-> alloc::vec::Vec<fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1> {
    use fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1 as Entry;
    let mut entries = alloc::vec![
        Entry::for_marker::<rmsnorm::qwen3_rmsnorm_v1_gpu::Marker>(),
        Entry::for_marker::<embedding::ferric_qwen3_tp_batch32_embedding_bf16_v5_gpu::Marker>(),
        Entry::for_marker::<
            baseline_projection::ferric_qwen3_tp_batch32_gemm_bf16_f32_bf16_v5_gpu::Marker,
        >(),
        Entry::for_marker::<
            baseline_projection::ferric_qwen3_tp_batch32_gemm_partial_bf16_f32_v5_gpu::Marker,
        >(),
        Entry::for_marker::<activation::ferric_qwen3_tp_batch32_swiglu_bf16_f32_v5_gpu::Marker>(),
        Entry::for_marker::<rope_kv::ferric_qwen3_tp_batch32_rope_v5_gpu::Marker>(),
        Entry::for_marker::<rope_kv::ferric_qwen3_tp_batch32_paged_kv_append_v5_gpu::Marker>(),
        Entry::for_marker::<
            baseline_attention::ferric_qwen3_tp_batch32_paged_gqa_bf16_f32_v5_gpu::Marker,
        >(),
        Entry::for_marker::<logits::ferric_qwen3_tp_batch32_argmax_bf16_v5_gpu::Marker>(),
        Entry::for_marker::<projection::ferric_qwen3_tp_batch32_wave_gemv_bf16_v5_gpu::Marker>(),
        Entry::for_marker::<projection::ferric_qwen3_tp_batch32_wave_gemv_partial_f32_v5_gpu::Marker>(
        ),
        Entry::for_marker::<attention::ferric_qwen3_tp_batch32_wave_paged_gqa_bf16_v5_gpu::Marker>(
        ),
        Entry::for_marker::<collective::ferric_qwen3_tp_batch32_residual_bf16_v5_gpu::Marker>(),
    ];
    #[cfg(feature = "mfma")]
    {
        entries.push(Entry::for_marker::<
            projection::ferric_qwen3_tp_batch32_mfma_gemm_bf16_v5_gpu::Marker,
        >());
        entries.push(Entry::for_marker::<
            projection::ferric_qwen3_tp_batch32_mfma_gemm_partial_f32_v5_gpu::Marker,
        >());
    }
    entries.sort_by_key(Entry::kernel_binding_id);
    entries
}
