//! Source policy for the default rejection backend and configured binder.

use fe2o3_host::CompilerGeneratedKernelExpectationRosterV1;
use ferric_qwen3_all_kernels_device_v1::M1AllKernelsWorkerV3RosterV1;

const SOURCE: &str = include_str!("../src/lib.rs");
const MANIFEST: &str = include_str!("../Cargo.toml");
const README: &str = include_str!("../README.md");
const ZERO_IMPL: &str = "unsafe impl WorkerV3ProtectedRosterVerifierBackendV1<M1AllKernelsWorkerV3RosterV1>\n    for M1AllKernelsProtectedVerifierV1";
const CONFIGURED_IMPL: &str = "unsafe impl WorkerV3ProtectedRosterVerifierBackendV1<M1AllKernelsWorkerV3RosterV1>\n    for M1AllKernelsProductionProtectedVerifierV1";
const FE2O3_REVISION: &str = "57d2d9ced5c113d40546ea1dee603e8ba499cf40";

fn between<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let (_, tail) = source.split_once(start).expect("start boundary");
    tail.split_once(end).expect("end boundary").0
}

fn appears_in_order(source: &str, needles: &[&str]) -> bool {
    let mut cursor = 0;
    for needle in needles {
        let Some(offset) = source[cursor..].find(needle) else {
            return false;
        };
        cursor += offset + needle.len();
    }
    true
}

fn zero_state_policy(source: &str) -> bool {
    let zero = between(
        source,
        ZERO_IMPL,
        "/// Failure returned by the configured aggregate production verifier binder.",
    );
    zero.contains("locally_revalidate_request_v1(request, &pending_request)?")
        && zero.contains("Self::reject_missing_protected_receipt(")
        && !zero.contains("request_receipt")
        && !zero.contains("WorkerV3ProtectedRosterVerificationEvidenceV1::new")
}

#[test]
fn roster_is_the_exact_twelve_entry_aggregate() {
    assert_eq!(M1AllKernelsWorkerV3RosterV1::ENTRIES.len(), 12);
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
}

#[test]
fn zero_state_backend_remains_unconditionally_fail_closed() {
    assert!(zero_state_policy(SOURCE));
    assert!(SOURCE.contains("Err(missing_protected_verification_receipt_v1())"));
    assert!(!zero_state_policy(&SOURCE.replacen(
        "Self::reject_missing_protected_receipt(",
        "unreachable_success(",
        1,
    )));
    assert!(!zero_state_policy(&SOURCE.replacen(
        "locally_revalidate_request_v1(request, &pending_request)?",
        "Default::default()",
        1,
    )));
}

#[test]
fn both_backends_share_one_local_owner_revalidation_and_association_path() {
    assert_eq!(
        SOURCE.matches("fn locally_revalidate_request_v1(").count(),
        1
    );
    assert_eq!(SOURCE.matches("locally_revalidate_request_v1(").count(), 3);
    assert_eq!(
        SOURCE
            .matches("fn validate_local_request_associations_v1(")
            .count(),
        1
    );
    assert_eq!(
        SOURCE
            .matches("::fe2o3_hsaco_finalize::verify_finalized(")
            .count(),
        1
    );
    assert_eq!(
        SOURCE
            .matches(".independently_revalidate_finalizer_derivation()")
            .count(),
        1
    );
    assert_eq!(
        SOURCE
            .matches(".validate_compiler_multi_root_proof_inputs_v1()")
            .count(),
        1
    );
    for required in [
        "finalizer_identity.as_bytes() == &pending.finalizer_derivation_sha256",
        "finalized_hsaco.sha256() == &pending.finalized_hsaco_sha256",
        "final_llvm.sha256() == *finalizer_module.sha256()",
        "final_llvm.sha256() == *semantic_module.sha256()",
        "pending.target == \"gfx942:xnack-\"",
        "pending.code_object_version == 6",
        "proof_roots.len() == M1_ALL_KERNELS_PENDING_ENTRY_COUNT_V1",
        "matched_reinspected_kernels",
        "descriptor_workgroup == root.workgroup()",
    ] {
        assert!(SOURCE.contains(required), "missing local join `{required}`");
    }
}

#[test]
fn configured_binder_orders_all_one_shot_transitions_before_promotion() {
    let configured = between(SOURCE, CONFIGURED_IMPL, "#[cfg(test)]\nmod tests {");
    assert!(
        SOURCE
            .contains("pub unsafe fn new(\n        client: M1AllKernelsProtectedVerifierClientV1")
    );
    assert!(SOURCE.contains(
        "# Safety\n    ///\n    /// The caller must have independently reviewed the service"
    ));
    assert!(appears_in_order(
        configured,
        &[
            "M1AllKernelsPendingRequestProjectionV1::from_request(request)",
            "locally_revalidate_request_v1(request, &pending)",
            ".audit_roster(request)",
            ".bind_exact_compiler_execution_v1(",
            "protected_service_request_v1(",
            "self.client.take()",
            ".request_receipt(&self.trust_policy, &service_request)",
            "authenticated_receipt_associates_v1(",
            "authenticated_entry_evidence_v1(",
            "WorkerV3ProtectedRosterVerificationEvidenceV1::new(",
        ]
    ));
    for forbidden in [
        "admit_inherited_application_service",
        "M1AllKernelsProtectedVerifierClientV1::admit",
        "M1AllKernelsProtectedVerifierTrustPolicyV1::new",
        "SigningKey",
        "synthetic_for_test_only",
    ] {
        assert!(
            !configured.contains(forbidden),
            "configured binder discovers or fabricates `{forbidden}`"
        );
    }
}

#[test]
fn service_claims_come_only_from_typed_source_and_bound_current_owners() {
    let builder = between(
        SOURCE,
        "fn protected_service_request_v1(",
        "fn authenticated_entry_evidence_v1(",
    );
    for required in [
        "project_m1_aggregate_module_handoff_v1(",
        "request.semantic_compiler_handoff().module_handoff()",
        "M1AllKernelsProtectedReceiptSourcePinV1::new(",
        "M1AllKernelsProtectedReceiptRequestClaimsV1::new(",
        "compiler.subject_sha256() == pending.compiler_execution_subject_sha256",
        "compiler.authenticates_signed_currentness_evidence()",
        "M1AllKernelsProtectedReceiptCompilerClaimsV1::new(",
        "compiler.current_record_verification_sha256()",
        "compiler.current_record_attestation_sha256()",
        "compiler.protected_policy_verification_sha256()",
        "compiler.protected_worker_ledger_verification_sha256()",
        "compiler.external_rollback_verification_sha256()",
        "M1AllKernelsProtectedVerifierServiceEntryV1::new(",
        "M1AllKernelsProtectedVerifierServiceRequestV1::new(",
    ] {
        assert!(
            builder.contains(required),
            "missing request derivation `{required}`"
        );
    }
    for forbidden in [
        "[1; 32]",
        "[0; 32]",
        "Sha256::digest",
        "from_json",
        "std::env",
    ] {
        assert!(
            !builder.contains(forbidden),
            "fabricated request input `{forbidden}`"
        );
    }
}

#[test]
fn final_join_rechecks_and_maps_every_signed_entry_result() {
    let coordinates = between(
        SOURCE,
        "fn authenticated_entry_coordinates_associate_v1(",
        "fn authenticated_entry_evidence_v1(",
    );
    let mapper = between(
        SOURCE,
        "fn authenticated_entry_evidence_v1(",
        "fn authenticated_receipt_associates_v1(",
    );
    for required in [
        ".zip(authenticated.receipt().entries())",
        "authenticated_entry_coordinates_associate_v1(",
        "signed.generated_host_contract_identity()",
        "signed.proof_executable_binding_sha256()",
        "signed.rust_type_layout_contract_sha256()",
        "signed.rust_effect_contract_sha256()",
        "signed.safety_properties()",
        "evidence.len() == M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1",
    ] {
        assert!(
            mapper.contains(required),
            "missing signed-entry join `{required}`"
        );
    }
    for required in [
        "usize::from(signed.ordinal()) == ordinal",
        "expected_ordinal == ordinal",
        "expected_lineage == Some(typed_lineage)",
        "signed.lineage_identity() == typed_lineage",
        "signed.marker_binding_identity() == expected_marker",
        "signed.generated_host_contract_identity() == expected_generated_host",
    ] {
        assert!(
            coordinates.contains(required),
            "missing coordinate join `{required}`"
        );
    }
}

#[test]
fn production_source_contains_no_deployment_values_or_runtime_surface() {
    for forbidden in [
        "std::env",
        "std::fs",
        "UnixStream::connect",
        "SigningKey",
        "CURRENT.json",
        "fe2o3_kfd",
        "fe2o3_hsa_runtime",
        "authorize_hsa_load",
        "synthetic_for_test_only",
    ] {
        assert!(
            !SOURCE.contains(forbidden),
            "forbidden surface `{forbidden}`"
        );
    }
    assert!(!MANIFEST.contains("worker-v3-verifier-test-support"));
    assert!(MANIFEST.contains("[workspace]"));
    assert!(MANIFEST.contains("fe2o3-verifier ="));
    assert!(MANIFEST.contains(
        "ferric-qwen3-all-kernels-worker-v3-source-pin-v1 = { path = \"../qwen3-all-kernels-worker-v3-source-pin-v1\" }"
    ));
    let revisions = MANIFEST
        .lines()
        .filter(|line| line.starts_with("fe2o3-"))
        .map(|line| {
            line.split_once("rev = \"")
                .and_then(|(_, tail)| tail.split_once('"'))
                .map(|(revision, _)| revision)
                .expect("every direct fe2o3 dependency is pinned")
        })
        .collect::<Vec<_>>();
    assert_eq!(revisions.len(), 3);
    assert!(
        revisions
            .iter()
            .all(|revision| *revision == FE2O3_REVISION)
    );
}

#[test]
fn documentation_states_prerequisites_and_nonclaims() {
    let normalized = README.split_whitespace().collect::<Vec<_>>().join(" ");
    for statement in [
        "zero-state default",
        "always returns `MissingProtectedVerificationReceipt`",
        "previously admitted one-shot service client",
        "caller-provisioned trust policy",
        "inherited FD195 compiler-current auditor",
        "All request-known compiler coordinates are compared back",
        "maps all 12 signed proof-to-executable, Rust type-layout, Rust effect",
        "coordinate protocol, not evidence transport",
        "Signing caller-supplied hash echoes does not satisfy",
        "A production deployment still must provide",
        "embeds none of those deployment values",
        "does not provide a service process, signing key, real receipt, model bundle, `CURRENT` record, qualification result, or GPU result",
        "grants no publication, load, launch, or inference authority by itself",
    ] {
        assert!(
            normalized.contains(statement),
            "README missing `{statement}`"
        );
    }
}
