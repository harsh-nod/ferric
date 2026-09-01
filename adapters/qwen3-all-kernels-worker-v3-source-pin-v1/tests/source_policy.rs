//! Source-level policy checks for the standalone aggregate extractor.

const SOURCE: &str = include_str!("../src/lib.rs");
const MAIN: &str = include_str!("../src/main.rs");
const MANIFEST: &str = include_str!("../Cargo.toml");
const LOCKFILE: &str = include_str!("../Cargo.lock");
const README: &str = include_str!("../README.md");
const DEVICE_MARKER_ORDER: &str =
    include_str!("../../../device/qwen3-all-kernels-v1/tests/marker_order.rs");

const FE2O3_REVISION: &str = "52815c9ed52a3075e26322cf506144cb22da12d2";

#[test]
fn standalone_manifest_and_lock_pin_the_exact_fe2o3_revision() {
    assert!(MANIFEST.contains("[workspace]"));
    assert_eq!(
        MANIFEST
            .matches(&format!("rev = \"{FE2O3_REVISION}\""))
            .count(),
        2
    );
    assert!(LOCKFILE.contains(&format!("rev={FE2O3_REVISION}#{FE2O3_REVISION}")));
    assert!(README.contains(FE2O3_REVISION));
}

#[test]
fn extraction_uses_typed_v2_and_compiler_handoff_decoders() {
    for required in [
        "WorkerV3LoadEnvelopeWireV2::decode_canonical",
        "wire.replay().outer_handoff()",
        "InertSemanticCompilerModuleHandoffV3::decode",
        "outer.module_handoff()",
        "handoff.kind() != CompilerModuleKindV1::LlvmTextIr",
        "handoff.module_identity()",
        "handoff.identity()",
        "handoff.symbol_manifest().identity()",
    ] {
        assert!(
            SOURCE.contains(required),
            "missing typed extraction step: {required}"
        );
    }
    assert!(MAIN.contains("MAX_WORKER_V3_LOAD_ENVELOPE_BYTES_V2"));
}

#[test]
fn source_pin_surface_cannot_claim_runtime_authority() {
    for required in [
        "identity-observation-only",
        "authenticates_compiler_origin",
        "grants_verifier_authority",
        "grants_publication_authority",
        "grants_load_authority",
        "grants_launch_authority",
    ] {
        assert!(
            SOURCE.contains(required),
            "missing authority boundary: {required}"
        );
    }
    for forbidden in [
        "fe2o3_host",
        "fe2o3_kfd",
        "fe2o3_amdhsa_loader",
        "AuthenticatedWorkerV3",
        "WorkerV3ProtectedRosterVerifier",
        "hip_runtime",
    ] {
        assert!(
            !SOURCE.contains(forbidden),
            "forbidden authority dependency: {forbidden}"
        );
        assert!(
            !MANIFEST.contains(forbidden),
            "forbidden authority manifest dependency: {forbidden}"
        );
    }
}

#[test]
fn exact_aggregate_roster_and_target_are_source_pinned() {
    assert!(SOURCE.contains("gfx942:xnack-"));
    for symbol in [
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
    ] {
        assert_eq!(
            SOURCE.matches(&format!("\"{symbol}\"")).count(),
            1,
            "{symbol}"
        );
    }
    assert!(SOURCE.contains("CompilerModuleSymbolRoleV1::KernelEntry"));
    assert!(SOURCE.contains("CompilerModuleSymbolRoleV1::KernelDescriptor"));
    assert!(README.contains("does not claim compiler descriptor-table order"));
}

#[test]
fn policy_roster_matches_the_aggregate_device_roster_surface() {
    let symbols = [
        "ferric_qwen3_lowest_id_argmax_bf16_v1",
        "ferric_qwen3_gemm_vector_a4_bf16_f32_bf16_v1",
        "qwen3_rope_v1",
        "ferric_qwen3_compact_completion_v1",
        "qwen3_paged_kv_write_v1",
        "qwen3_paged_gqa_decode_bf16_f32_v1",
        "qwen3_swiglu_bf16_f32_v1",
        "ferric_qwen3_gemm_reference_bf16_f32_bf16_v1",
        "qwen3_rmsnorm_v1",
        "ferric_qwen3_token_embedding_bf16_copy_v1",
        "ferric_qwen3_speculative_token_assembly_v1",
        "qwen3_gqa_prefill_causal_bf16_f32_v1",
    ];
    for symbol in symbols {
        assert_eq!(SOURCE.matches(&format!("\"{symbol}\"")).count(), 1);
        assert_eq!(
            DEVICE_MARKER_ORDER
                .matches(&format!("\"{symbol}\""))
                .count(),
            1,
            "aggregate device roster drifted for {symbol}"
        );
    }
    let source_positions = symbols.map(|symbol| SOURCE.find(&format!("\"{symbol}\"")).unwrap());
    let device_positions =
        symbols.map(|symbol| DEVICE_MARKER_ORDER.find(&format!("\"{symbol}\"")).unwrap());
    assert!(source_positions.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(device_positions.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn canonical_output_exposes_exactly_six_source_pin_fields() {
    for field in [
        "compiler_handoff_length",
        "compiler_handoff_sha256",
        "compiler_module_length",
        "compiler_module_sha256",
        "symbol_manifest_length",
        "symbol_manifest_sha256",
    ] {
        assert!(SOURCE.contains(&format!("\"{field}\"")), "{field}");
    }
}
