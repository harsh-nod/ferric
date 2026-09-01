#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)] // The kernel macro emits undocumented helper modules.

//! One selected compilation unit for all 12 attributed Ferric M1 device roots.

pub mod gemm;
pub mod logits;
pub mod paged_decode;
pub mod prefill;
pub mod rmsnorm;
pub mod rope_kv;
pub mod swiglu;

#[cfg(not(target_arch = "amdgpu"))]
mod host_roster {
    type PagedKvWrite = super::rope_kv::qwen3_paged_kv_write_v1_gpu::Marker;
    type SwiGlu = super::swiglu::qwen3_swiglu_bf16_f32_v1_gpu::Marker;
    type LowestIdArgmax = super::logits::ferric_qwen3_lowest_id_argmax_bf16_v1_gpu::Marker;
    type GemmVectorized = super::gemm::ferric_qwen3_gemm_vector_a4_bf16_f32_bf16_v1_gpu::Marker;
    type GemmReference = super::gemm::ferric_qwen3_gemm_reference_bf16_f32_bf16_v1_gpu::Marker;
    type Prefill = super::prefill::qwen3_gqa_prefill_causal_bf16_f32_v1_gpu::Marker;
    type PagedDecode = super::paged_decode::qwen3_paged_gqa_decode_bf16_f32_v1_gpu::Marker;
    type TokenEmbedding = super::gemm::ferric_qwen3_token_embedding_bf16_copy_v1_gpu::Marker;
    type SpeculativeAssembly =
        super::logits::ferric_qwen3_speculative_token_assembly_v1_gpu::Marker;
    type CompactCompletion = super::logits::ferric_qwen3_compact_completion_v1_gpu::Marker;
    type RmsNorm = super::rmsnorm::qwen3_rmsnorm_v1_gpu::Marker;
    type Rope = super::rope_kv::qwen3_rope_v1_gpu::Marker;

    fe2o3_host::compiler_generated_kernel_expectation_roster_v1! {
        /// All 12 aggregate markers in exact compiler descriptor-table order.
        pub struct M1AllKernelsWorkerV3RosterV1 = [
            SwiGlu,
            Prefill,
            LowestIdArgmax,
            PagedKvWrite,
            PagedDecode,
            SpeculativeAssembly,
            GemmVectorized,
            GemmReference,
            TokenEmbedding,
            CompactCompletion,
            Rope,
            RmsNorm,
        ];
    }
}

#[cfg(not(target_arch = "amdgpu"))]
pub use host_roster::M1AllKernelsWorkerV3RosterV1;
