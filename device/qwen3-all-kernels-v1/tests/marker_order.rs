use fe2o3_host::{
    CompilerGeneratedKernelExpectationRosterEntryV1 as Entry,
    CompilerGeneratedKernelExpectationRosterV1, CompilerGeneratedKernelExpectationV1,
};

#[test]
fn aggregate_roster_has_exact_global_marker_order() {
    use ferric_qwen3_all_kernels_device_v1::{
        M1AllKernelsMfmaWorkerV3RosterV1, M1AllKernelsWorkerV3RosterV1,
        gemm::{
            ferric_qwen3_gemm_mfma_bf16_f32_bf16_v1_gpu::Marker as GemmMfma,
            ferric_qwen3_gemm_reference_bf16_f32_bf16_v1_gpu::Marker as GemmReference,
            ferric_qwen3_gemm_vector_a4_bf16_f32_bf16_v1_gpu::Marker as GemmVectorized,
            ferric_qwen3_token_embedding_bf16_copy_v1_gpu::Marker as TokenEmbedding,
        },
        logits::{
            ferric_qwen3_compact_completion_v1_gpu::Marker as CompactCompletion,
            ferric_qwen3_lowest_id_argmax_bf16_v1_gpu::Marker as LowestIdArgmax,
            ferric_qwen3_speculative_token_assembly_v1_gpu::Marker as SpeculativeAssembly,
        },
        paged_decode::qwen3_paged_gqa_decode_bf16_f32_v1_gpu::Marker as PagedDecode,
        prefill::qwen3_gqa_prefill_causal_bf16_f32_v1_gpu::Marker as Prefill,
        rmsnorm::qwen3_rmsnorm_v1_gpu::Marker as RmsNorm,
        rope_kv::{
            qwen3_paged_kv_write_v1_gpu::Marker as PagedKvWrite, qwen3_rope_v1_gpu::Marker as Rope,
        },
        swiglu::qwen3_swiglu_bf16_f32_v1_gpu::Marker as SwiGlu,
    };

    let mut expected = [
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
    ];
    expected.sort_unstable_by_key(Entry::kernel_binding_id);
    let entries = M1AllKernelsMfmaWorkerV3RosterV1::ENTRIES;
    let legacy = M1AllKernelsWorkerV3RosterV1::ENTRIES;
    assert_eq!(legacy.len(), 12);
    for retained in legacy {
        assert_eq!(
            entries
                .iter()
                .filter(|current| *current == retained)
                .count(),
            1
        );
        assert_ne!(retained.kernel_binding_id(), GemmMfma::KERNEL_BINDING_ID_V1);
    }
    assert!(
        legacy
            .windows(2)
            .all(|pair| pair[0].kernel_binding_id() < pair[1].kernel_binding_id())
    );
    assert_eq!(entries.len(), expected.len());
    assert_eq!(entries, expected);
    assert!(entries.iter().all(|entry| {
        entry.kernel_binding_id() != [0; 32] && entry.generated_host_contract_identity() != [0; 32]
    }));
    assert!(
        entries.iter().enumerate().all(|(index, entry)| entries
            .iter()
            .skip(index + 1)
            .all(|other| entry.kernel_binding_id() != other.kernel_binding_id())),
        "aggregate marker identities must be unique"
    );
    assert!(
        entries
            .windows(2)
            .all(|pair| pair[0].kernel_binding_id() < pair[1].kernel_binding_id()),
        "the aggregate roster must follow canonical descriptor-table order: {:?}",
        entries
            .iter()
            .map(|entry| (entry.export_name(), entry.kernel_binding_id()))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.export_name())
            .collect::<Vec<_>>(),
        expected.iter().map(Entry::export_name).collect::<Vec<_>>()
    );
}

#[cfg(feature = "gfx942")]
#[test]
fn current_default_namespace_exercises_nonprefix_and_nonlast_mfma() {
    use ferric_qwen3_all_kernels_device_v1::{
        M1AllKernelsMfmaWorkerV3RosterV1, M1AllKernelsWorkerV3RosterV1,
        gemm::ferric_qwen3_gemm_mfma_bf16_f32_bf16_v1_gpu::Marker,
    };
    let mfma = M1AllKernelsMfmaWorkerV3RosterV1::ENTRIES;
    let index = mfma
        .iter()
        .position(|entry| *entry == Entry::for_marker::<Marker>())
        .unwrap();
    assert!(index < 12);
    assert_ne!(M1AllKernelsWorkerV3RosterV1::ENTRIES, &mfma[..12]);
}
