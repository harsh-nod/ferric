//! Source-level policy checks for the standalone aggregate extractor.

const SOURCE: &str = include_str!("../src/lib.rs");
const MAIN: &str = include_str!("../src/main.rs");
const MANIFEST: &str = include_str!("../Cargo.toml");
const LOCKFILE: &str = include_str!("../Cargo.lock");
const README: &str = include_str!("../README.md");
const DEVICE_MARKER_ORDER: &str =
    include_str!("../../../device/qwen3-all-kernels-v1/tests/marker_order.rs");

const FE2O3_REVISION: &str = "5a503c04f5ae107a3b3e951ec970b36c5d6a9a79";

const TYPED_MARKERS: [(&str, &str); 13] = [
    (
        "ferric_qwen3_gemm_reference_bf16_f32_bf16_v1",
        "GemmReference",
    ),
    ("qwen3_swiglu_bf16_f32_v1", "SwiGlu"),
    ("qwen3_rope_v1", "Rope"),
    (
        "ferric_qwen3_speculative_token_assembly_v1",
        "SpeculativeAssembly",
    ),
    (
        "ferric_qwen3_token_embedding_bf16_copy_v1",
        "TokenEmbedding",
    ),
    ("qwen3_paged_gqa_decode_bf16_f32_v1", "PagedDecode"),
    ("qwen3_gqa_prefill_causal_bf16_f32_v1", "Prefill"),
    ("qwen3_paged_kv_write_v1", "PagedKvWrite"),
    ("ferric_qwen3_compact_completion_v1", "CompactCompletion"),
    (
        "ferric_qwen3_gemm_vector_a4_bf16_f32_bf16_v1",
        "GemmVectorized",
    ),
    ("ferric_qwen3_lowest_id_argmax_bf16_v1", "LowestIdArgmax"),
    ("qwen3_rmsnorm_v1", "RmsNorm"),
    ("ferric_qwen3_gemm_mfma_bf16_f32_bf16_v1", "GemmMfma"),
];

// This is a closed fixture contract, not a parser for arbitrary Rust or a new roster.
const TYPED_PREFIX: &str = r"
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
";

const TYPED_SUFFIX: &str = r#"
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
"#;

fn typed_fixture_is_closed(fixture: &str) -> bool {
    let entries = TYPED_MARKERS
        .map(|(_, alias)| format!("Entry::for_marker::<{alias}>(),"))
        .join("\n");
    let expected = format!("{TYPED_PREFIX}\n{entries}\n{TYPED_SUFFIX}");
    fixture.split_whitespace().eq(expected.split_whitespace())
}

fn reject_typed_fixture_mutation(needle: &str, replacement: &str) {
    assert_eq!(DEVICE_MARKER_ORDER.matches(needle).count(), 1, "{needle}");
    let hostile = DEVICE_MARKER_ORDER.replacen(needle, replacement, 1);
    assert_ne!(hostile, DEVICE_MARKER_ORDER);
    assert!(!typed_fixture_is_closed(&hostile), "accepted {needle}");
}

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
fn policy_roster_matches_the_aggregate_device_roster_set() {
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
            TYPED_MARKERS
                .iter()
                .filter(|(export, _)| *export == symbol)
                .count(),
            1,
            "typed fixture export drifted for {symbol}"
        );
    }
    assert!(typed_fixture_is_closed(DEVICE_MARKER_ORDER));
    let extra = TYPED_MARKERS
        .into_iter()
        .filter(|(export, _)| !symbols.contains(export))
        .collect::<Vec<_>>();
    assert_eq!(
        extra,
        vec![("ferric_qwen3_gemm_mfma_bf16_f32_bf16_v1", "GemmMfma")]
    );
    let source_positions = symbols.map(|symbol| SOURCE.find(&format!("\"{symbol}\"")).unwrap());
    assert!(source_positions.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(
        DEVICE_MARKER_ORDER
            .contains("the aggregate roster must follow canonical descriptor-table order")
    );
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

#[test]
fn typed_fixture_imports_aliases_and_entries_reject_mutations() {
    assert!(typed_fixture_is_closed(DEVICE_MARKER_ORDER));
    assert!(typed_fixture_is_closed(
        &DEVICE_MARKER_ORDER.replace('\n', "\n    ")
    ));
    for (symbol, alias) in TYPED_MARKERS {
        let import = format!("{symbol}_gpu::Marker as {alias}");
        reject_typed_fixture_mutation(&import, "");
        reject_typed_fixture_mutation(&import, &format!("{import}, {import}"));
        reject_typed_fixture_mutation(&import, &format!("{symbol}_gpu::Marker as Other{alias}"));
        reject_typed_fixture_mutation(&import, &format!("other_{symbol}_gpu::Marker as {alias}"));

        let entry = format!("Entry::for_marker::<{alias}>(),");
        reject_typed_fixture_mutation(&entry, "");
        reject_typed_fixture_mutation(&entry, &format!("{entry}{entry}"));
        reject_typed_fixture_mutation(&entry, "Entry::for_marker::<Wrong>(),");
    }
    reject_typed_fixture_mutation("gemm::{", "logits::{");
}

#[test]
fn typed_fixture_preserves_legacy_and_current_mfma_separation() {
    for (needle, replacement) in [
        (
            "let legacy = M1AllKernelsWorkerV3RosterV1::ENTRIES;",
            "let legacy = M1AllKernelsMfmaWorkerV3RosterV1::ENTRIES;",
        ),
        (
            "let entries = M1AllKernelsMfmaWorkerV3RosterV1::ENTRIES;",
            "let entries = M1AllKernelsWorkerV3RosterV1::ENTRIES;",
        ),
        (
            "assert_eq!(legacy.len(), 12);",
            "assert_eq!(legacy.len(), 13);",
        ),
        (
            ".filter(|current| *current == retained)",
            ".filter(|_current| true)",
        ),
        (
            "assert_ne!(retained.kernel_binding_id(), GemmMfma::KERNEL_BINDING_ID_V1);",
            "",
        ),
        (
            "expected.sort_unstable_by_key(Entry::kernel_binding_id);",
            "expected.sort_unstable_by_key(Entry::export_name);",
        ),
        ("assert_eq!(entries.len(), expected.len());", ""),
        ("assert_eq!(entries, expected);", ""),
        (
            "entry.kernel_binding_id() != [0; 32] && entry.generated_host_contract_identity() != [0; 32]",
            "true",
        ),
        (
            ".all(|other| entry.kernel_binding_id() != other.kernel_binding_id())",
            ".all(|_other| true)",
        ),
        ("assert!(index < 12);", "assert!(index <= 12);"),
        (
            "assert_ne!(M1AllKernelsWorkerV3RosterV1::ENTRIES, &mfma[..12]);",
            "",
        ),
    ] {
        reject_typed_fixture_mutation(needle, replacement);
    }
    let sorted = ".all(|pair| pair[0].kernel_binding_id() < pair[1].kernel_binding_id())";
    assert_eq!(DEVICE_MARKER_ORDER.matches(sorted).count(), 2);
    for occurrence in 0..2 {
        let offset = DEVICE_MARKER_ORDER
            .match_indices(sorted)
            .nth(occurrence)
            .unwrap()
            .0;
        let mut hostile = DEVICE_MARKER_ORDER.to_owned();
        hostile.replace_range(offset..offset + sorted.len(), ".all(|_pair| true)");
        assert!(!typed_fixture_is_closed(&hostile));
    }
}

#[test]
fn typed_fixture_rejects_commented_quoted_duplicate_or_extra_code() {
    for hostile in [
        format!("/*{DEVICE_MARKER_ORDER}*/"),
        format!("const TEXT: &str = {DEVICE_MARKER_ORDER:?};"),
        format!("{DEVICE_MARKER_ORDER}\n{DEVICE_MARKER_ORDER}"),
        format!("fn unrelated() {{}}\n{DEVICE_MARKER_ORDER}"),
    ] {
        assert!(!typed_fixture_is_closed(&hostile));
    }
}
