//! Source-level policy checks for the protected-verifier service boundary.

const PROTOCOL_SOURCE: &str = include_str!("../src/protected_verifier_service.rs");
const CLIENT_SOURCE: &str = include_str!("../src/protected_verifier_client.rs");
const TEST_SUPPORT_SOURCE: &str = include_str!("../src/protected_verifier_test_support.rs");
const BACKEND_SOURCE: &str = include_str!("../src/lib.rs");
const MANIFEST: &str = include_str!("../Cargo.toml");
const README: &str = include_str!("../README.md");
const TEST_BOUNDARY: &str = "#[cfg(test)]\nmod tests {";

fn production_before_tests(source: &'static str) -> &'static str {
    source
        .split_once(TEST_BOUNDARY)
        .map_or(source, |(production, tests)| {
            assert!(!tests.trim().is_empty());
            production
        })
}

#[test]
fn packets_are_fixed_binary_bounded_and_domain_separated() {
    let protocol = production_before_tests(PROTOCOL_SOURCE);
    for required in [
        "const HEADER_BYTES: usize = 24;",
        "const TARGET_BLOCK_BYTES: usize = 24;",
        "const REQUEST_CLAIMS_BYTES: usize = 384;",
        "const COMPILER_CLAIMS_BYTES: usize = 520;",
        "const ENTRY_COORDINATES_BYTES: usize = 104;",
        "const REQUEST_MAGIC: [u8; 8] = *b\"FRW3VSQ1\";",
        "const RESPONSE_MAGIC: [u8; 8] = *b\"FRW3VSP1\";",
        "SERVICE-REQUEST/V1\\0",
        "SERVICE-RESPONSE/V1\\0",
        "const TARGET: &[u8] = b\"gfx942:xnack-\";",
        "const CODE_OBJECT_VERSION: u16 = 6;",
        "const _: [(); 2_304]",
        "const _: [(); 3_768]",
        "M1_ALL_KERNELS_PROTECTED_RECEIPT_BYTES_V1",
        "decoded.canonical_bytes != bytes",
        "#![forbid(unsafe_code)]",
        "This V1 transports coordinates, not their evidence payloads.",
        "must already hold, or authentically reacquire",
        "verify those payloads rather than sign a hash echo",
        "atomically",
        "protected live current-ledger state shared across instances and restarts",
    ] {
        assert!(
            protocol.contains(required),
            "missing protocol rule: {required}"
        );
    }
    for forbidden in [
        "serde",
        "serde_json",
        "http",
        "std::env",
        "std::fs",
        "std::net",
        "std::process",
        "Path",
        "unsafe {",
    ] {
        assert!(
            !protocol.contains(forbidden),
            "protocol contains forbidden surface {forbidden}",
        );
    }
}

#[test]
fn request_and_response_bind_every_caller_known_axis() {
    let protocol = production_before_tests(PROTOCOL_SOURCE);
    for coordinate in [
        "trust_policy_identity",
        "expected_sequence",
        "expected_current_rollback_anchor",
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
        "capsule_sha256",
        "formal_memory_receipt_sha256",
        "proof_binding_receipt_sha256",
        "finalized_hsaco_sha256",
        "finalized_hsaco_length",
        "subject_sha256",
        "carriage_sha256",
        "policy_sha256",
        "issuer_journal_sha256",
        "compiler_occurrence_sha256",
        "receipt_sha256",
        "publication_sha256",
        "acknowledgment_sha256",
        "worker_ledger_record_sha256",
        "prior_rollback_anchor",
        "current_rollback_anchor",
        "current_record_verification_sha256",
        "current_record_attestation_sha256",
        "protected_policy_verification_sha256",
        "protected_worker_ledger_verification_sha256",
        "external_rollback_verification_sha256",
        "lineage_identity",
        "marker_binding_identity",
        "generated_host_contract_identity",
    ] {
        assert!(
            protocol.contains(coordinate),
            "protocol omits coordinate {coordinate}",
        );
    }
    for binding in [
        "expected_sequence != compiler_claims.sequence()",
        "expected_current_rollback_anchor != compiler_claims.current_rollback_anchor()",
        "receipt.trust_policy_identity() == self.trust_policy_identity",
        "receipt.request_claims() == &self.request_claims",
        "receipt.compiler_claims() == &self.compiler_claims",
        "expected.matches_receipt(actual)",
        "self.request_identity == request.identity",
        "request.matches_receipt(&self.receipt)",
    ] {
        assert!(
            protocol.contains(binding),
            "missing cross-binding: {binding}"
        );
    }
}

#[test]
fn client_pins_peer_and_rejects_ambiguous_transport() {
    let client = production_before_tests(CLIENT_SOURCE);
    for required in [
        "SOCK_SEQPACKET",
        "SO_PEERCRED",
        "FD_CLOEXEC",
        "MSG_DONTWAIT | libc::MSG_NOSIGNAL",
        "MSG_DONTWAIT | libc::MSG_CMSG_CLOEXEC",
        "header.msg_flags & libc::MSG_CTRUNC",
        "header.msg_flags & libc::MSG_TRUNC",
        "header.msg_controllen != 0",
        "received != bytes.len()",
        "current.pid != expected_pid",
        "credentials.uid == client_uid",
        "wait_for_peer(peer, libc::POLLOUT, deadline)",
        "wait_for_peer(peer, libc::POLLIN, deadline)",
        ".authenticate_canonical(receipt.encode_canonical())",
        "require_deadline(self.deadline)?",
        "if !response.matches_request(request)",
        "into_peer(self) -> OwnedFd",
        "not replay across new",
        "locally retained request, evidence-custody, and audit owners",
    ] {
        assert!(client.contains(required), "missing client rule: {required}");
    }
    for forbidden in [
        "std::env",
        "std::fs",
        "std::net",
        "std::process",
        "UnixStream::connect",
        "TcpStream",
        "Path",
        "SigningKey",
        "VerifyingKey",
        "impl Default",
        "option_env!",
        "env!",
    ] {
        assert!(
            !client.contains(forbidden),
            "client production source contains forbidden surface {forbidden}",
        );
    }
}

#[test]
fn production_backend_remains_disconnected_and_fail_closed() {
    let backend = production_before_tests(BACKEND_SOURCE);
    for forbidden in [
        "M1AllKernelsProtectedVerifierClientV1",
        "M1AllKernelsProtectedVerifierServiceRequestV1",
        "M1AllKernelsProtectedVerifierTrustPolicyV1",
        "authenticate_canonical(",
        "WorkerV3ProtectedRosterVerificationEvidenceV1::",
    ] {
        assert!(
            !backend.contains(forbidden),
            "production backend reaches service authority surface {forbidden}",
        );
    }
    assert!(backend.contains("Err(missing_protected_verification_receipt_v1())"));
    assert!(backend.contains("Self::reject_missing_protected_receipt("));
    assert!(!TEST_SUPPORT_SOURCE.trim().is_empty());
    assert!(TEST_SUPPORT_SOURCE.contains("SigningKey"));
    assert!(!production_before_tests(CLIENT_SOURCE).contains("SigningKey"));
    assert!(!production_before_tests(PROTOCOL_SOURCE).contains("SigningKey"));
}

#[test]
fn dependency_and_documentation_boundaries_are_explicit() {
    assert!(MANIFEST.contains("libc = \"=0.2.189\""));
    for forbidden in ["serde =", "serde_json =", "reqwest =", "hyper ="] {
        assert!(
            !MANIFEST.contains(forbidden),
            "unexpected dependency {forbidden}"
        );
    }
    let normalized = README.split_whitespace().collect::<Vec<_>>().join(" ");
    for statement in [
        "request is exactly 2,304 bytes",
        "response is exactly 3,768 bytes",
        "binary, fixed-width protocol rather than HTTP, JSON, or a filesystem interchange",
        "caller-provisioned trust-policy identity",
        "requires a distinct production client/service UID",
        "returns the exact `OwnedFd`",
        "V1 is deliberately a coordinate protocol, not evidence transport",
        "exact receipt-bearing Worker V3 V2 envelope, finalized HSACO bytes, semantic/proof inputs, and protected current-record evidence",
        "must never be signed as a hash echo",
        "atomically consume every signed challenge",
        "protected live current-ledger state shared across all service instances and durable across restarts",
        "There is no production endpoint constructor, pathname, inherited descriptor, service process, or backend hookup",
        "still returns `MissingProtectedVerificationReceipt`",
        "locally bind the authenticated receipt to its retained request, evidence-custody owners, and audit result",
        "must never promote a hash echo",
        "grant no fe2o3 verifier, load, launch, inference, publication, or `CURRENT` authority",
    ] {
        assert!(
            normalized.contains(statement),
            "README is missing `{statement}`"
        );
    }
}
