#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)]

//! Opt-in gfx950 performance kernels; the original v2 bodies remain available.
//! Reassociated FP32 reductions require separate numerical qualification.

#[cfg(not(feature = "gfx950"))]
compile_error!("the v3 performance profile requires gfx950");

#[cfg(not(target_arch = "amdgpu"))]
extern crate alloc;
#[cfg(test)]
extern crate std;

#[path = "../../qwen3-tp-batch-kernels-v2/src/activation.rs"]
#[cfg(not(test))]
pub mod activation;
#[path = "../../qwen3-tp-batch-kernels-v2/src/attention.rs"]
#[cfg(not(test))]
pub mod baseline_attention;
#[path = "../../qwen3-tp-batch-kernels-v2/src/contract.rs"]
pub mod baseline_contract;
#[path = "../../qwen3-tp-batch-kernels-v2/src/projection.rs"]
#[cfg(not(test))]
pub mod baseline_projection;
#[path = "../../qwen3-tp-batch-kernels-v2/src/embedding.rs"]
#[cfg(not(test))]
pub mod embedding;
#[path = "../../qwen3-tp-batch-kernels-v2/src/logits.rs"]
#[cfg(not(test))]
pub mod logits;
#[path = "../../qwen3-all-kernels-v1/src/rmsnorm.rs"]
#[cfg(not(test))]
pub mod rmsnorm;
#[path = "../../qwen3-tp-batch-kernels-v2/src/rope_kv.rs"]
#[cfg(not(test))]
pub mod rope_kv;
#[path = "../../qwen3-all-kernels-v1/src/target.rs"]
pub mod target;

pub mod attention;
pub mod collective;
pub mod contract;
pub mod projection;

#[cfg(all(not(target_arch = "amdgpu"), not(test)))]
pub fn compiler_expectation_roster_v3()
-> alloc::vec::Vec<fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1> {
    use fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1 as Entry;
    let mut entries = alloc::vec![
        Entry::for_marker::<rmsnorm::qwen3_rmsnorm_v1_gpu::Marker>(),
        Entry::for_marker::<embedding::ferric_qwen3_tp_batch_embedding_bf16_v2_gpu::Marker>(),
        Entry::for_marker::<
            baseline_projection::ferric_qwen3_tp_batch_gemm_bf16_f32_bf16_v2_gpu::Marker,
        >(),
        Entry::for_marker::<
            baseline_projection::ferric_qwen3_tp_batch_gemm_partial_bf16_f32_v2_gpu::Marker,
        >(),
        Entry::for_marker::<activation::ferric_qwen3_tp_batch_swiglu_bf16_f32_v2_gpu::Marker>(),
        Entry::for_marker::<rope_kv::ferric_qwen3_tp_batch_rope_v2_gpu::Marker>(),
        Entry::for_marker::<rope_kv::ferric_qwen3_tp_batch_paged_kv_append_v2_gpu::Marker>(),
        Entry::for_marker::<
            baseline_attention::ferric_qwen3_tp_batch_paged_gqa_bf16_f32_v2_gpu::Marker,
        >(),
        Entry::for_marker::<logits::ferric_qwen3_tp_batch_argmax_bf16_v2_gpu::Marker>(),
        Entry::for_marker::<projection::ferric_qwen3_tp_wave_gemv_bf16_v3_gpu::Marker>(),
        Entry::for_marker::<projection::ferric_qwen3_tp_wave_gemv_partial_f32_v3_gpu::Marker>(),
        Entry::for_marker::<projection::ferric_qwen3_tp_mfma_gemm_bf16_v3_gpu::Marker>(),
        Entry::for_marker::<projection::ferric_qwen3_tp_mfma_gemm_partial_f32_v3_gpu::Marker>(),
        Entry::for_marker::<attention::ferric_qwen3_tp_wave_paged_gqa_bf16_v3_gpu::Marker>(),
        Entry::for_marker::<collective::ferric_qwen3_tp_batch_residual_bf16_v3_gpu::Marker>(),
    ];
    entries.sort_by_key(Entry::kernel_binding_id);
    entries
}
