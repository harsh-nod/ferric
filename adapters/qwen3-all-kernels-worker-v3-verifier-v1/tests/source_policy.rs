//! Source-level policy checks for the fail-closed aggregate verifier adapter.

const SOURCE: &str = include_str!("../src/lib.rs");
const MANIFEST: &str = include_str!("../Cargo.toml");
const README: &str = include_str!("../README.md");

const FE2O3_REVISION: &str = "2d275684d7a22f8f913114b51b1d1dd524d1ed9b";

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
fn production_backend_has_one_fail_closed_return() {
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
    assert_eq!(method_body, "Self::reject_missing_protected_receipt()");

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
fn no_synthetic_or_hash_only_acceptance_surface_exists() {
    for forbidden in [
        "synthetic_for_test_only",
        "worker-v3-verifier-test-support",
        "Sha256",
        "sha2",
        "[u8; 32]",
        "_sha256",
        "Digest",
        "verifier_measurement_sha256",
        "verification_transcript_sha256",
        "proof_executable_binding_sha256",
        "WorkerV3ProtectedRosterVerificationEvidenceV1::new",
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
fn adapter_imports_no_load_launch_or_inference_authority() {
    for forbidden in [
        "fe2o3-kfd",
        "fe2o3_kfd",
        "fe2o3-hsa-runtime",
        "fe2o3_hsa_runtime",
        "hip_runtime",
        "hip::",
        "ferric-engine",
        "ferric_engine",
        "launch(",
        "load(",
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
