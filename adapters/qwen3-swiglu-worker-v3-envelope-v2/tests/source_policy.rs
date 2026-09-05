const SOURCE: &str = include_str!("../src/lib.rs");
const MANIFEST: &str = include_str!("../Cargo.toml");
const LOCKFILE: &str = include_str!("../Cargo.lock");
const ADMISSION_DOC: &str = include_str!("../../../docs/M1_QWEN3_SWIGLU_PRODUCTION_ADMISSION.md");
const BUILD_EVIDENCE: &str =
    include_str!("../../../proofs/m1/evidence/PROTECTED_WORKER_V3_SWIGLU_BUILD.json");

const FE2O3_REVISION: &str = "4413b086482f2f4ad218f28e4485dc089d6cc020";

#[test]
fn public_adapter_surface_has_no_parallel_identity_inputs() {
    for function in [
        "decode_m1_swiglu_pending_request_v2",
        "project_m1_swiglu_pending_request_from_recovered_v2",
    ] {
        assert!(SOURCE.contains(&format!("pub fn {function}")));
    }
    assert!(SOURCE.contains("bytes: &[u8]"));
    assert!(SOURCE.contains("owner: RecoveredWorkerV3LoadEnvelopeV2"));
    assert!(SOURCE.contains("M1SwiGluV2RecoveredPendingRequestV1 { pending, owner }"));
    assert!(SOURCE.contains("Box::new(M1SwiGluV2RecoveredProjectionFailureV1"));

    let public_prefix = SOURCE.split("fn project_wire").next().unwrap();
    assert!(!public_prefix.contains("from_untrusted_observation"));
    assert!(!public_prefix.contains("envelope_sha256: [u8; 32],\n) -> Result"));
    assert!(!public_prefix.contains("carriage_identity: [u8; 32],\n) -> Result"));
}

#[test]
fn adapter_does_not_import_runtime_or_qualification_authority() {
    for forbidden in [
        "fe2o3_kfd",
        "fe2o3_host",
        "WorkerV3VerifierV1",
        "AuthenticatedWorkerV3ExecutableV1",
        "hip::",
        "hip_runtime",
        "qualification/qwen3-swiglu-v1/hip_numeric.cpp",
    ] {
        assert!(
            !SOURCE.contains(forbidden),
            "adapter source contains forbidden authority marker {forbidden}"
        );
    }
}

#[test]
fn standalone_manifest_and_lock_pin_the_exact_current_fe2o3_revision() {
    assert!(MANIFEST.contains("[workspace]"));
    assert!(MANIFEST.contains(&format!("rev = \"{FE2O3_REVISION}\"")));
    assert!(LOCKFILE.contains(&format!("rev={FE2O3_REVISION}#{FE2O3_REVISION}")));
    assert!(ADMISSION_DOC.contains(FE2O3_REVISION));
}

#[test]
fn admission_doc_distinguishes_compiler_completion_from_deployment_authority() {
    let normalized = ADMISSION_DOC
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    for boundary in [
        "Cargo now transfers one fixed inherited receipt descriptor to rustc",
        "the backend acquires the subject-bound receipt",
        "Cargo reconstructs and admits the durable V2 carriage under currentness",
        "Those generic mechanisms do not provision Ferric's production authority",
    ] {
        assert!(
            normalized.contains(boundary),
            "admission document is missing boundary: {boundary}"
        );
    }
}

#[test]
fn exact_build_policy_constants_are_grounded_in_checked_in_evidence() {
    for identity in [
        "093b45da9da3b6859553345aa38e5789aad4949b725e33e4e4d6620045455ed1",
        "401b5b2b54190e7bd0e0115da9aa85b17187631e9c9ee2057bf4655c456083e0",
        "97664a82bf361020647e36634e90afa30ccc4958c85b2da62baaa01303d75ef8",
        "61db6ef6f80e89dc6ac571f99edc5728edc0a3def3c4ad1d117787d4ef743565",
        "37aa965af2c771fcd4c13f635660d25961509d37d0a0572efdb9ec569f53f896",
        "1ce1b7a5c834a14f0334ba75522e9f0aec31ce6761d4516ec36d45c72bfd839f",
        "de561a1eb2b66a1b85b05e6bda06c5e545c17d642fd0aa23f0a2458fef532b12",
        "0397e40dc360f47c3b301c3b7aa8a1ce5342f862b7de8c0909c185179d49523c",
        "af9dc3b58ff454dd78253cabbdd1bc2f114e1add2a16c995befbec5a3d50e2b2",
        "57ecb86b40db136237e65a5fae04c955f2c92fe3347c085ec5c806984fc6afa7",
    ] {
        assert!(SOURCE.contains(identity), "source is missing {identity}");
        assert!(
            BUILD_EVIDENCE.contains(identity),
            "evidence is missing {identity}"
        );
    }
    for length in ["1_100_878", "1_096_510", "14_192"] {
        assert!(SOURCE.contains(length), "source is missing length {length}");
    }
}
