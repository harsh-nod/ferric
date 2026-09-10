use fe2o3_host::CompilerGeneratedKernelExpectationV1;
use ferric_qwen3_tp_kernels_device_v1::{
    activation, attention, gemm, logits, projection, rmsnorm, rope_kv,
};

#[test]
fn marker_inventory_is_unique() {
    macro_rules! marker {
        ($module:ident, $marker:ident) => {
            (
                $module::$marker::Marker::KERNEL_BINDING_ID_V1,
                stringify!($marker),
            )
        };
    }
    let mut entries = [
        marker!(gemm, ferric_qwen3_gemm_reference_bf16_f32_bf16_v1_gpu),
        marker!(gemm, ferric_qwen3_gemm_vector_a4_bf16_f32_bf16_v1_gpu),
        marker!(gemm, ferric_qwen3_token_embedding_bf16_copy_v1_gpu),
        marker!(logits, ferric_qwen3_compact_completion_v1_gpu),
        marker!(logits, ferric_qwen3_lowest_id_argmax_bf16_v1_gpu),
        marker!(logits, ferric_qwen3_speculative_token_assembly_v1_gpu),
        marker!(rmsnorm, qwen3_rmsnorm_v1_gpu),
        marker!(projection, ferric_qwen3_tp_gemv_bf16_f32_bf16_v1_gpu),
        marker!(projection, ferric_qwen3_tp_gemv_partial_bf16_f32_v1_gpu),
        marker!(activation, ferric_qwen3_tp_swiglu_bf16_f32_v1_gpu),
        marker!(rope_kv, ferric_qwen3_tp_rope_v1_gpu),
        marker!(rope_kv, ferric_qwen3_tp_kv_append_v1_gpu),
        marker!(attention, ferric_qwen3_tp_gqa_decode_bf16_f32_v1_gpu),
    ];
    entries.sort();
    let roster = ferric_qwen3_tp_kernels_device_v1::compiler_expectation_roster_v1();
    assert_eq!(
        roster
            .iter()
            .map(|entry| entry.kernel_binding_id())
            .collect::<Vec<_>>(),
        entries.iter().map(|entry| entry.0).collect::<Vec<_>>()
    );
    let mut symbols = roster
        .iter()
        .map(|entry| entry.export_name())
        .collect::<Vec<_>>();
    symbols.sort();
    assert_eq!(
        symbols,
        ferric_qwen3_tp_kernels_device_v1::contract::KERNEL_SYMBOLS
    );
    assert!(
        roster
            .iter()
            .all(|entry| entry.generated_host_contract_identity() != [0; 32]
                && !entry.logical_name().is_empty())
    );
    for entry in &roster {
        println!(
            "logical={} entry={} descriptor={}.kd",
            entry.logical_name(),
            entry.export_name(),
            entry.export_name()
        );
    }
    for (id, name) in entries {
        println!("{name}: {id:02x?}");
    }
    assert!(entries.windows(2).all(|pair| pair[0].0 < pair[1].0));
    assert!(entries.iter().all(|entry| entry.0 != [0; 32]));
}
