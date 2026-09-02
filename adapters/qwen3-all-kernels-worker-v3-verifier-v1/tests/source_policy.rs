//! Source-level policy checks for the fail-closed aggregate verifier adapter.

use fe2o3_host::CompilerGeneratedKernelExpectationRosterV1;
use ferric_qwen3_all_kernels_device_v1::M1AllKernelsWorkerV3RosterV1;

const SOURCE: &str = include_str!("../src/lib.rs");
const MANIFEST: &str = include_str!("../Cargo.toml");
const README: &str = include_str!("../README.md");

const FE2O3_REVISION: &str = "52815c9ed52a3075e26322cf506144cb22da12d2";

#[test]
fn backend_is_specific_to_the_exact_aggregate_roster() {
    assert!(SOURCE.contains(
        "unsafe impl WorkerV3ProtectedRosterVerifierBackendV1<\
M1AllKernelsWorkerV3RosterV1>"
    ));
    assert!(
        SOURCE.contains("WorkerV3RosterVerificationRequestV1<'_, M1AllKernelsWorkerV3RosterV1>")
    );
    assert!(SOURCE.contains("M1AllKernelsWorkerV3RosterV1::ENTRIES.len()"));
}

#[test]
fn pending_projection_is_private_and_typed_request_only() {
    assert!(SOURCE.contains("struct M1AllKernelsPendingRequestProjectionV1 {"));
    assert!(SOURCE.contains("struct M1AllKernelsPendingEntryProjectionV1 {"));
    assert!(SOURCE.contains("struct M1AllKernelsPendingDescriptorProjectionV1 {"));
    assert!(SOURCE.contains("struct M1AllKernelsPendingDescriptorBindingProjectionV1 {"));
    assert!(SOURCE.contains("struct M1AllKernelsPendingPhysicalKernelProjectionV1 {"));
    assert!(!SOURCE.contains("pub struct M1AllKernelsPendingRequestProjectionV1"));
    assert!(!SOURCE.contains("pub struct M1AllKernelsPendingEntryProjectionV1"));
    assert!(!SOURCE.contains("pub struct M1AllKernelsPendingDescriptorProjectionV1"));
    assert!(!SOURCE.contains("pub struct M1AllKernelsPendingDescriptorBindingProjectionV1"));
    assert!(!SOURCE.contains("pub struct M1AllKernelsPendingPhysicalKernelProjectionV1"));
    assert!(SOURCE.contains(
        "fn from_request(\n        request: &WorkerV3RosterVerificationRequestV1<'_, \
M1AllKernelsWorkerV3RosterV1>,"
    ));
    assert!(SOURCE.contains(
        "fn entry_from_request(\n        request: &WorkerV3RosterVerificationRequestV1<'_, \
M1AllKernelsWorkerV3RosterV1>,"
    ));
    for forbidden in [
        "from_untrusted",
        "from_parts",
        "from_json",
        "preflight",
        "serialize",
        "deserialize",
        ".expect(",
        ".unwrap(",
        "panic!(",
        "unreachable!(",
        "unimplemented!(",
    ] {
        assert!(
            !SOURCE.contains(forbidden),
            "production source contains forbidden parallel input surface {forbidden}"
        );
    }
}

#[test]
fn pending_projection_covers_every_request_identity_axis() {
    for getter in [
        "challenge_identity()",
        "roster_identity()",
        "lineage_identity()",
        "finalizer_derivation_sha256()",
        "compiler_execution_subject_sha256()",
        "compiler_execution_carriage_sha256()",
        "compiler_execution_policy_sha256()",
        "compiler_execution_issuer_journal_sha256()",
        "compiler_occurrence_sha256()",
        "compiler_execution_receipt_sha256()",
        "compiler_execution_publication_sha256()",
        "compiler_execution_acknowledgment_sha256()",
        "compiler_execution_worker_ledger_record_sha256()",
        "compiler_execution_sequence()",
        "compiler_execution_prior_rollback_anchor()",
        "compiler_execution_current_rollback_anchor()",
        "capsule_sha256()",
        "formal_memory_receipt_sha256()",
        "proof_binding_receipt_sha256()",
        "finalized_hsaco_sha256()",
        "finalized_hsaco_length()",
        "target()",
        "code_object_version()",
    ] {
        assert!(
            SOURCE.contains(getter),
            "pending request projection omits {getter}"
        );
    }
}

#[test]
fn pending_projection_has_exactly_twelve_ordered_complete_entry_rows() {
    assert!(SOURCE.contains("const M1_ALL_KERNELS_PENDING_ENTRY_COUNT_V1: usize = 12;"));
    assert!(SOURCE.contains("const _: [(); M1_ALL_KERNELS_PENDING_ENTRY_COUNT_V1]"));
    assert!(SOURCE.contains("[(); M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1];"));
    assert!(SOURCE.contains("entries: [M1AllKernelsPendingEntryProjectionV1;"));
    assert!(SOURCE.contains("M1_ALL_KERNELS_PENDING_ENTRY_COUNT_V1],"));
    assert!(SOURCE.contains("let marker_entries = request.marker_entries();"));
    assert!(SOURCE.contains("let entries = std::array::from_fn(|ordinal|"));
    assert!(SOURCE.contains(".entry_lineage_identity(ordinal)"));
    for field in [
        "ordinal,",
        "logical_name: marker.logical_name()",
        "export_name: marker.export_name()",
        "marker_binding_identity: marker.kernel_binding_id()",
        "generated_host_contract_identity: marker.generated_host_contract_identity()",
        ".map(|identity| *identity.as_bytes())",
        "lineage_identity: lineage",
        "descriptor,",
        "descriptor_binding,",
        "physical_kernel,",
    ] {
        assert!(
            SOURCE.contains(field),
            "ordered entry projection omits {field}"
        );
    }
}

#[test]
fn every_entry_projects_typed_descriptor_binding_and_physical_facts() {
    for getter in [
        "request.descriptor(ordinal)",
        "request.descriptor_binding(ordinal)",
        "request.physical_kernel(ordinal)",
    ] {
        assert!(SOURCE.contains(getter), "entry projection omits {getter}");
    }
    for field in [
        "kernel_id: *descriptor.kernel_id().as_bytes()",
        "logical_name: descriptor.logical_name().as_str().to_owned()",
        "entry_name: descriptor.entry_name().as_str().to_owned()",
        "descriptor_symbol: descriptor.descriptor_symbol().as_str().to_owned()",
        "source_evidence_identity:",
        "source_evidence_digest:",
        "executable_ir_evidence_identity:",
        "executable_ir_evidence_digest:",
        "explicit_argument_size:",
        "kernarg_segment_size:",
        "kernarg_segment_alignment:",
        "capability_count:",
        "logical_argument_count:",
        "kernel_index: binding.kernel_index()",
        "descriptor_address: binding.descriptor_address()",
        "descriptor_file_offset: binding.descriptor_file_offset()",
        "entry_address: binding.entry_address()",
        "entry_file_offset: binding.entry_file_offset()",
        "entry_size: binding.entry_size()",
        "kernel_code_entry_byte_offset:",
        "compute_pgm_rsrc3:",
        "compute_pgm_rsrc1:",
        "compute_pgm_rsrc2:",
        "kernel_code_properties:",
        "kernarg_preload: descriptor.kernarg_preload()",
        "name: physical.name().to_owned()",
        "symbol: physical.symbol().to_owned()",
        "group_segment_fixed_size:",
        "private_segment_fixed_size:",
        "wavefront_size:",
        "sgpr_count:",
        "vgpr_count:",
        "agpr_count:",
        "sgpr_spill_count:",
        "vgpr_spill_count:",
        "max_flat_workgroup_size:",
        "required_workgroup_size:",
        "max_workgroups:",
        "cluster_dims:",
        "uniform_work_group_size:",
        "uses_dynamic_stack:",
        "workgroup_processor_mode:",
        "implicit_argument_offset:",
        "implicit_argument_size:",
        "explicit_argument_count:",
        "hidden_argument_count:",
    ] {
        assert!(
            SOURCE.contains(field),
            "typed descriptor/physical projection omits {field}"
        );
    }
    for optional in [
        "descriptor: Option<M1AllKernelsPendingDescriptorProjectionV1>",
        "descriptor_binding: Option<M1AllKernelsPendingDescriptorBindingProjectionV1>",
        "physical_kernel: Option<M1AllKernelsPendingPhysicalKernelProjectionV1>",
    ] {
        assert!(
            SOURCE.contains(optional),
            "typed row absence is not explicit for {optional}"
        );
    }
}

#[test]
fn typed_roster_fixes_the_exact_twelve_projection_rows() {
    assert_eq!(
        M1AllKernelsWorkerV3RosterV1::ENTRIES
            .iter()
            .map(fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1::export_name)
            .collect::<Vec<_>>(),
        [
            "qwen3_swiglu_bf16_f32_v1",
            "qwen3_gqa_prefill_causal_bf16_f32_v1",
            "ferric_qwen3_lowest_id_argmax_bf16_v1",
            "qwen3_paged_kv_write_v1",
            "qwen3_paged_gqa_decode_bf16_f32_v1",
            "ferric_qwen3_speculative_token_assembly_v1",
            "ferric_qwen3_gemm_vector_a4_bf16_f32_bf16_v1",
            "ferric_qwen3_gemm_reference_bf16_f32_bf16_v1",
            "ferric_qwen3_token_embedding_bf16_copy_v1",
            "ferric_qwen3_compact_completion_v1",
            "qwen3_rope_v1",
            "qwen3_rmsnorm_v1",
        ]
    );
    assert!(M1AllKernelsWorkerV3RosterV1::ENTRIES.iter().all(|entry| {
        entry.kernel_binding_id() != [0; 32] && entry.generated_host_contract_identity() != [0; 32]
    }));
}

#[test]
fn production_backend_projects_then_has_one_fail_closed_return() {
    let method = SOURCE
        .split("unsafe fn verify_protected_roster")
        .nth(1)
        .expect("production verifier method must exist")
        .split("\n    }")
        .next()
        .expect("production verifier method must terminate");
    let method_body = method
        .split_once('{')
        .expect("production verifier method must have a body")
        .1
        .trim();
    assert!(method_body.contains(
        "let pending_request = M1AllKernelsPendingRequestProjectionV1::from_request(request);"
    ));
    assert!(method_body.ends_with("Self::reject_missing_protected_receipt(&pending_request)"));
    assert!(!method_body.contains("if "));
    assert!(!method_body.contains("match "));
    assert!(!method_body.contains("Ok("));

    let rejection = SOURCE
        .split("fn reject_missing_protected_receipt")
        .nth(1)
        .expect("production rejection helper must exist")
        .split("\n    }")
        .next()
        .expect("production rejection helper must terminate");
    assert!(rejection.contains("Err(missing_protected_verification_receipt_v1())"));
    assert!(!rejection.contains("if "));
    assert!(!rejection.contains("match "));
    assert!(!rejection.contains("Ok("));
    assert_eq!(SOURCE.matches("Err(").count(), 1);
    assert_eq!(SOURCE.matches("Ok(").count(), 0);
}

#[test]
fn no_synthetic_or_projected_identity_acceptance_surface_exists() {
    for forbidden in [
        "synthetic_for_test_only",
        "worker-v3-verifier-test-support",
        "Sha256",
        "use sha2",
        "sha2::",
        "sha2 =",
        "Digest",
        "verifier_measurement_sha256",
        "verification_transcript_sha256",
        "proof_executable_binding_sha256",
        "WorkerV3ProtectedRosterVerificationEvidenceV1::new",
        "WorkerV3ProtectedRosterEntryEvidenceV1::new",
        "WorkerV3ProtectedRosterEntryEvidenceV1",
        "AuthenticatedWorkerV3RosterV1",
        "AuthenticatedWorkerV3ExecutableV1",
    ] {
        assert!(
            !SOURCE.contains(forbidden),
            "production source contains forbidden acceptance surface {forbidden}"
        );
        assert!(
            !MANIFEST.contains(forbidden),
            "manifest contains forbidden acceptance surface {forbidden}"
        );
    }
}

#[test]
fn adapter_has_no_external_input_or_policy_authority_surface() {
    for forbidden in [
        "std::env",
        "std::fs",
        "std::net",
        "std::path",
        "std::process",
        "env!",
        "option_env!",
        "File::",
        "OpenOptions",
        "Path::",
        "PathBuf",
        "Command::",
        "TcpStream",
        "UnixStream",
        "serde",
        "serde_json",
        "json!",
        "clap",
        "argh",
        "lexopt",
        "fn main(",
        "policy_key",
        "policy_root",
        "trust_root",
        "root_key",
        "secret_key",
        "public_key",
        "keyring",
        "fe2o3-kfd",
        "fe2o3_kfd",
        "fe2o3-hsa-runtime",
        "fe2o3_hsa_runtime",
        "hip_runtime",
        "hip::",
        "ferric-engine",
        "ferric_engine",
        "launch(",
        ".load(",
        "::load(",
        "authorize_hsa_load",
    ] {
        assert!(!SOURCE.contains(forbidden), "source contains {forbidden}");
        assert!(
            !MANIFEST.contains(forbidden),
            "manifest contains {forbidden}"
        );
    }
}

#[test]
fn standalone_manifest_pins_the_current_generic_boundary() {
    assert!(MANIFEST.contains("[workspace]"));
    assert!(MANIFEST.contains(&format!("rev = \"{FE2O3_REVISION}\"")));
    assert!(MANIFEST.contains(
        "ferric-qwen3-all-kernels-device-v1 = { path = \"../../device/qwen3-all-kernels-v1\" }"
    ));
    assert!(!MANIFEST.contains("Cargo.lock"));
}

#[test]
fn documentation_states_the_non_authority_boundary() {
    for statement in [
        "private reject-only projection",
        "typed `WorkerV3RosterVerificationRequestV1`",
        "exactly 12 ordered entry rows",
        "typed descriptor, ELF-binding, and physical-kernel facts",
        "kernarg-preload field",
        "not runtime pointers or load authority",
        "lineage subprojection remains",
        "no public constructor, serializer, JSON preflight, environment, file, or CLI input",
        "no protected policy key or trust root",
        "neither panics nor invents a zero identity",
        "unconditional `Err(MissingProtectedVerificationReceipt)`",
        "does not accept hashes as a substitute",
        "grants no verification, load, launch, or inference authority",
        "no direct KFD, HSA, HIP, engine, or model import",
        "broader resolved runtime closure",
        "MissingProtectedVerificationReceipt",
    ] {
        assert!(
            README.contains(statement),
            "README is missing `{statement}`"
        );
    }
}
