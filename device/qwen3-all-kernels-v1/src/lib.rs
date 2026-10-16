#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)] // The kernel macro emits undocumented helper modules.

//! One selected compilation unit for all 13 attributed Ferric M1 device roots.

pub mod gemm;
pub mod logits;
pub mod paged_decode;
pub mod prefill;
pub mod rmsnorm;
pub mod rope_kv;
pub mod swiglu;
pub mod target;

#[cfg(not(target_arch = "amdgpu"))]
mod host_roster {
    type PagedKvWrite = super::rope_kv::qwen3_paged_kv_write_v1_gpu::Marker;
    type SwiGlu = super::swiglu::qwen3_swiglu_bf16_f32_v1_gpu::Marker;
    type LowestIdArgmax = super::logits::ferric_qwen3_lowest_id_argmax_bf16_v1_gpu::Marker;
    type GemmVectorized = super::gemm::ferric_qwen3_gemm_vector_a4_bf16_f32_bf16_v1_gpu::Marker;
    type GemmReference = super::gemm::ferric_qwen3_gemm_reference_bf16_f32_bf16_v1_gpu::Marker;
    type GemmMfma = super::gemm::ferric_qwen3_gemm_mfma_bf16_f32_bf16_v1_gpu::Marker;
    type Prefill = super::prefill::qwen3_gqa_prefill_causal_bf16_f32_v1_gpu::Marker;
    type PagedDecode = super::paged_decode::qwen3_paged_gqa_decode_bf16_f32_v1_gpu::Marker;
    type TokenEmbedding = super::gemm::ferric_qwen3_token_embedding_bf16_copy_v1_gpu::Marker;
    type SpeculativeAssembly =
        super::logits::ferric_qwen3_speculative_token_assembly_v1_gpu::Marker;
    type CompactCompletion = super::logits::ferric_qwen3_compact_completion_v1_gpu::Marker;
    type RmsNorm = super::rmsnorm::qwen3_rmsnorm_v1_gpu::Marker;
    type Rope = super::rope_kv::qwen3_rope_v1_gpu::Marker;

    use fe2o3_host::{
        CompilerGeneratedKernelExpectationRosterEntryV1 as Entry,
        CompilerGeneratedKernelExpectationRosterV1,
    };

    const fn binding_less(left: [u8; 32], right: [u8; 32]) -> bool {
        let mut byte = 0;
        while byte < 32 {
            if left[byte] != right[byte] {
                return left[byte] < right[byte];
            }
            byte += 1;
        }
        false
    }

    // Sort only producer-owned generated full tuples, never incoming evidence.
    const fn canonical_entries<const N: usize>(mut entries: [Entry; N]) -> [Entry; N] {
        let mut next = 1;
        while next < N {
            let mut current = next;
            while current > 0
                && binding_less(
                    entries[current].kernel_binding_id(),
                    entries[current - 1].kernel_binding_id(),
                )
            {
                let previous = entries[current - 1];
                entries[current - 1] = entries[current];
                entries[current] = previous;
                current -= 1;
            }
            next += 1;
        }
        let mut index = 1;
        while index < N {
            assert!(
                binding_less(
                    entries[index - 1].kernel_binding_id(),
                    entries[index].kernel_binding_id(),
                ),
                "duplicate generated kernel binding in producer roster"
            );
            index += 1;
        }
        entries
    }

    /// Exact producer-owned roster in ascending generated KernelId order.
    pub struct M1AllKernelsWorkerV3RosterV1;

    impl CompilerGeneratedKernelExpectationRosterV1 for M1AllKernelsWorkerV3RosterV1 {
        const ENTRIES: &'static [Entry] = &canonical_entries([
            Entry::for_marker::<GemmReference>(),
            Entry::for_marker::<SwiGlu>(),
            Entry::for_marker::<Rope>(),
            Entry::for_marker::<SpeculativeAssembly>(),
            Entry::for_marker::<TokenEmbedding>(),
            Entry::for_marker::<PagedDecode>(),
            Entry::for_marker::<Prefill>(),
            Entry::for_marker::<PagedKvWrite>(),
            Entry::for_marker::<CompactCompletion>(),
            Entry::for_marker::<GemmVectorized>(),
            Entry::for_marker::<LowestIdArgmax>(),
            Entry::for_marker::<RmsNorm>(),
        ]);
    }

    /// Exact producer-owned roster in ascending generated KernelId order.
    pub struct M1AllKernelsMfmaWorkerV3RosterV1;

    impl CompilerGeneratedKernelExpectationRosterV1 for M1AllKernelsMfmaWorkerV3RosterV1 {
        const ENTRIES: &'static [Entry] = &canonical_entries([
            Entry::for_marker::<GemmReference>(),
            Entry::for_marker::<SwiGlu>(),
            Entry::for_marker::<Rope>(),
            Entry::for_marker::<SpeculativeAssembly>(),
            Entry::for_marker::<TokenEmbedding>(),
            Entry::for_marker::<PagedDecode>(),
            Entry::for_marker::<Prefill>(),
            Entry::for_marker::<PagedKvWrite>(),
            Entry::for_marker::<CompactCompletion>(),
            Entry::for_marker::<GemmVectorized>(),
            Entry::for_marker::<LowestIdArgmax>(),
            Entry::for_marker::<RmsNorm>(),
            Entry::for_marker::<GemmMfma>(),
        ]);
    }

    #[cfg(test)]
    mod tests {
        extern crate std;
        use super::*;

        fn permutations_and_duplicates<const N: usize>(entries: &[Entry]) {
            let expected: [Entry; N] = entries.try_into().unwrap();
            let mut reversed = expected;
            reversed.reverse();
            let mut rotated = expected;
            rotated.rotate_left(3);
            for permutation in [reversed, rotated] {
                assert_eq!(canonical_entries(permutation), expected);
                for entry in expected {
                    assert_eq!(
                        permutation
                            .iter()
                            .filter(|actual| **actual == entry)
                            .count(),
                        1
                    );
                }
            }
            let mut duplicate = expected;
            duplicate[1] = duplicate[0];
            assert!(std::panic::catch_unwind(|| canonical_entries(duplicate)).is_err());
        }

        #[test]
        fn canonicalization_preserves_full_tuples_and_rejects_duplicates() {
            permutations_and_duplicates::<12>(M1AllKernelsWorkerV3RosterV1::ENTRIES);
            permutations_and_duplicates::<13>(M1AllKernelsMfmaWorkerV3RosterV1::ENTRIES);
            std::println!(
                "generated namespace witness: {:?}",
                M1AllKernelsMfmaWorkerV3RosterV1::ENTRIES[0].kernel_binding_id()
            );
        }
    }
}

#[cfg(not(target_arch = "amdgpu"))]
pub use host_roster::M1AllKernelsWorkerV3RosterV1;

#[cfg(not(target_arch = "amdgpu"))]
pub use host_roster::M1AllKernelsMfmaWorkerV3RosterV1;
