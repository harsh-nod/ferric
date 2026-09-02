//! Source-level policy checks for the aggregate protected-receipt boundary.

const RECEIPT_SOURCE: &str = include_str!("../src/protected_receipt.rs");
const BACKEND_SOURCE: &str = include_str!("../src/lib.rs");
const MANIFEST: &str = include_str!("../Cargo.toml");
const README: &str = include_str!("../README.md");
const TEST_BOUNDARY: &str = "#[cfg(test)]\nmod tests {";

fn production_receipt_source() -> &'static str {
    let (production, tests) = RECEIPT_SOURCE
        .split_once(TEST_BOUNDARY)
        .expect("receipt unit-test boundary");
    assert!(!tests.trim().is_empty());
    production
}

#[test]
fn wire_is_fixed_bounded_domain_separated_and_strictly_reencoded() {
    let production = production_receipt_source();
    for required in [
        "const SIGNATURE_BYTES: usize = 64;",
        "const HEADER_BYTES: usize = 32;",
        "const ENTRY_BYTES: usize = 200;",
        "const COMMON_BYTES: usize = 1_056;",
        "const UNSIGNED_BYTES: usize =\n    HEADER_BYTES",
        "M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1 * ENTRY_BYTES",
        "pub const M1_ALL_KERNELS_PROTECTED_RECEIPT_BYTES_V1",
        "const MAGIC: [u8; 8] = *b\"FRW3PR1\\0\";",
        "const VERSION: u16 = 1;",
        "const TARGET: &[u8] = b\"gfx942:xnack-\";",
        "const CODE_OBJECT_VERSION: u16 = 6;",
        "RECEIPT-SIGNATURE/V1\\0",
        "RECEIPT-IDENTITY/V1\\0",
        "TRUST-POLICY/V1\\0",
        "#![forbid(unsafe_code)]",
        "if bytes.len() != M1_ALL_KERNELS_PROTECTED_RECEIPT_BYTES_V1",
        "if unsigned.encode_canonical() != &bytes[..UNSIGNED_BYTES]",
        "if !reader.is_finished()",
    ] {
        assert!(
            production.contains(required),
            "missing wire rule: {required}"
        );
    }
    for forbidden in [
        "serde",
        "serde_json",
        "std::env",
        "std::fs",
        "std::net",
        "std::process",
        "unsafe {",
        "from_utf8_unchecked",
    ] {
        assert!(
            !production.contains(forbidden),
            "receipt production source contains forbidden surface {forbidden}"
        );
    }
}

#[test]
fn trust_policy_is_caller_provisioned_without_embedded_signing_authority() {
    let production = production_receipt_source();
    for required in [
        "pub struct M1AllKernelsProtectedVerifierTrustPolicyV1",
        "VerifyingKey::from_bytes(&verifying_key)",
        "if verifying_key.is_weak()",
        "verify_strict(&receipt.signing_bytes(), &signature)",
        "verifier == checker",
        "M1AllKernelsProtectedReceiptErrorV1::AliasedVerifierAndCheckerMeasurements",
        "receipt.trust_policy_identity() != self.identity",
        "receipt.verifier_measurement_sha256() != self.verifier_measurement_sha256",
        "receipt.checker_measurement_sha256() != self.checker_measurement_sha256",
    ] {
        assert!(
            production.contains(required),
            "missing trust-policy rule: {required}"
        );
    }
    for forbidden in [
        "SigningKey",
        "impl Default for M1AllKernelsProtectedVerifierTrustPolicyV1",
        "M1AllKernelsProtectedVerifierTrustPolicyV1::default",
        "secret_key",
        "private_key",
        "option_env!",
        "env!",
    ] {
        assert!(
            !production.contains(forbidden),
            "production trust policy embeds or discovers authority through {forbidden}"
        );
    }
}

#[test]
fn receipt_carries_every_common_source_and_current_record_axis() {
    let production = production_receipt_source();
    for coordinate in [
        "challenge_identity",
        "roster_identity",
        "host_lineage_identity",
        "finalizer_derivation_sha256",
        "compiler_module_sha256",
        "compiler_module_length",
        "compiler_handoff_sha256",
        "compiler_handoff_length",
        "symbol_manifest_sha256",
        "symbol_manifest_length",
        "subject_sha256",
        "carriage_sha256",
        "policy_sha256",
        "issuer_journal_sha256",
        "compiler_occurrence_sha256",
        "receipt_sha256",
        "publication_sha256",
        "acknowledgment_sha256",
        "worker_ledger_record_sha256",
        "sequence",
        "prior_rollback_anchor",
        "current_rollback_anchor",
        "capsule_sha256",
        "formal_memory_receipt_sha256",
        "proof_binding_receipt_sha256",
        "finalized_hsaco_sha256",
        "finalized_hsaco_length",
        "const TARGET: &[u8] = b\"gfx942:xnack-\"",
        "const CODE_OBJECT_VERSION: u16 = 6",
    ] {
        assert!(
            production.contains(coordinate),
            "receipt omits signed coordinate {coordinate}"
        );
    }
    for protected_result in [
        "current_record_verification_sha256",
        "current_record_attestation_sha256",
        "protected_policy_verification_sha256",
        "protected_worker_ledger_verification_sha256",
        "external_rollback_verification_sha256",
        "verifier_measurement_sha256",
        "checker_measurement_sha256",
        "verification_transcript_sha256",
    ] {
        assert!(
            production.contains(protected_result),
            "receipt omits protected result {protected_result}"
        );
    }
}

#[test]
fn exactly_twelve_ordered_complete_entry_results_are_required() {
    let production = production_receipt_source();
    for required in [
        "entries: [M1AllKernelsProtectedReceiptEntryV1; M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1]",
        "if entry.ordinal != expected",
        "DuplicateEntryIdentity",
        "prior.lineage_identity == entry.lineage_identity",
        "prior.marker_binding_identity == entry.marker_binding_identity",
        "safety_properties != WorkerV3SafetyPropertiesV1::required()",
        "lineage_identity",
        "marker_binding_identity",
        "pub const fn generated_host_contract_identity",
        "generated_host_contract_identity",
        "proof_executable_binding_sha256",
        "rust_type_layout_contract_sha256",
        "rust_effect_contract_sha256",
    ] {
        assert!(
            production.contains(required),
            "missing entry rule: {required}"
        );
    }
}

#[test]
fn codec_cannot_construct_host_evidence_or_change_default_backend_rejection() {
    let production = production_receipt_source();
    for forbidden in [
        "WorkerV3ProtectedRosterVerificationEvidenceV1",
        "WorkerV3ProtectedRosterEntryEvidenceV1",
        "AuthenticatedWorkerV3RosterV1",
        "authorize_hsa_load",
        "fe2o3_kfd",
        "fe2o3_hsa_runtime",
    ] {
        assert!(
            !production.contains(forbidden),
            "receipt codec reaches authority surface {forbidden}"
        );
    }
    assert!(BACKEND_SOURCE.contains("pub struct M1AllKernelsProtectedVerifierV1;"));
    assert!(BACKEND_SOURCE.contains("Err(missing_protected_verification_receipt_v1())"));
    assert!(BACKEND_SOURCE.contains("Self::reject_missing_protected_receipt("));
    assert!(!BACKEND_SOURCE.contains("M1AllKernelsProtectedVerifierTrustPolicyV1"));
    assert!(!BACKEND_SOURCE.contains("authenticate_canonical("));
    assert!(!production.contains("bind_request"));
    assert!(!production.contains("WorkerV3RosterVerificationRequestV1"));
    assert!(!production.contains("RequestBoundProtectedVerifierReceipt"));
}

#[test]
fn direct_crypto_dependencies_are_exact_and_minimal() {
    assert!(MANIFEST.contains(
        "ed25519-dalek = { version = \"=2.2.0\", default-features = false, features = [\"fast\", \"zeroize\"] }"
    ));
    assert!(MANIFEST.contains("sha2 = { version = \"=0.11.0\", default-features = false }"));
    for forbidden in [
        "rand =",
        "rand_core =",
        "base64 =",
        "serde =",
        "serde_json =",
    ] {
        assert!(
            !MANIFEST.contains(forbidden),
            "unexpected dependency {forbidden}"
        );
    }
}

#[test]
fn documentation_keeps_receipt_authentication_below_authority_promotion() {
    let normalized = README.split_whitespace().collect::<Vec<_>>().join(" ");
    for statement in [
        "fixed-width, 3,552-byte little-endian frame",
        "3,488-byte signed preimage",
        "domain-separated Ed25519 signing message",
        "distinct protected-verifier and independent-checker measurements",
        "all 12 ordered entry coordinates",
        "has no default and embeds no key or measurement",
        "must independently provision an exact non-weak Ed25519 public key",
        "still explicitly grants no verifier, load, launch, or inference authority",
        "The production backend does not read, embed, or instantiate this policy or receipt",
        "`MissingProtectedVerificationReceipt`",
    ] {
        assert!(
            normalized.contains(statement),
            "README is missing `{statement}`"
        );
    }
}
