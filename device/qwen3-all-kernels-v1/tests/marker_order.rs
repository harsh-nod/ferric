use fe2o3_host::{
    CompilerGeneratedKernelExpectationRosterV1, CompilerGeneratedKernelExpectationV1,
};

const PRODUCTION_CRATE_BINDING_V1: &str =
    "cfa53c5dd7ab25966e45f74b5a7bb8cb2518f47d9599806177ba1dc949049f21";

#[test]
fn aggregate_roster_has_exact_global_marker_order() {
    use ferric_qwen3_all_kernels_device_v1::{
        M1AllKernelsWorkerV3RosterV1,
        gemm::{
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

    let expected = [
        GemmVectorized::KERNEL_BINDING_ID_V1,
        RmsNorm::KERNEL_BINDING_ID_V1,
        GemmReference::KERNEL_BINDING_ID_V1,
        PagedKvWrite::KERNEL_BINDING_ID_V1,
        SwiGlu::KERNEL_BINDING_ID_V1,
        Rope::KERNEL_BINDING_ID_V1,
        PagedDecode::KERNEL_BINDING_ID_V1,
        Prefill::KERNEL_BINDING_ID_V1,
        CompactCompletion::KERNEL_BINDING_ID_V1,
        SpeculativeAssembly::KERNEL_BINDING_ID_V1,
        TokenEmbedding::KERNEL_BINDING_ID_V1,
        LowestIdArgmax::KERNEL_BINDING_ID_V1,
    ];
    let entries = M1AllKernelsWorkerV3RosterV1::ENTRIES;
    assert_eq!(entries.len(), expected.len());
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.kernel_binding_id())
            .collect::<Vec<_>>(),
        expected
    );
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
    if option_env!("FE2O3_CRATE_BINDING_ID_V1") == Some(PRODUCTION_CRATE_BINDING_V1) {
        assert!(
            entries
                .windows(2)
                .all(|pair| pair[0].kernel_binding_id() < pair[1].kernel_binding_id()),
            "the production roster must follow canonical descriptor-table order"
        );
    }
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.export_name())
            .collect::<Vec<_>>(),
        vec![
            "ferric_qwen3_gemm_vector_a4_bf16_f32_bf16_v1",
            "qwen3_rmsnorm_v1",
            "ferric_qwen3_gemm_reference_bf16_f32_bf16_v1",
            "qwen3_paged_kv_write_v1",
            "qwen3_swiglu_bf16_f32_v1",
            "qwen3_rope_v1",
            "qwen3_paged_gqa_decode_bf16_f32_v1",
            "qwen3_gqa_prefill_causal_bf16_f32_v1",
            "ferric_qwen3_compact_completion_v1",
            "ferric_qwen3_speculative_token_assembly_v1",
            "ferric_qwen3_token_embedding_bf16_copy_v1",
            "ferric_qwen3_lowest_id_argmax_bf16_v1",
        ]
    );
}
