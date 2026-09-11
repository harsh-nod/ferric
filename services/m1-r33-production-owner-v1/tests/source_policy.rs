//! Source-policy gates for the externally supervised production owner.

use std::fs;
use std::path::Path;

#[test]
fn production_owner_has_one_external_capability_chain_and_no_engineering_fallback() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_file = fs::read_to_string(root.join("src/main.rs")).unwrap();
    let source = source_file.split("#[cfg(test)]").next().unwrap();
    for required in [
        "COMPILER_CURRENT_RECORD_FD_V1: i32 = 195",
        "PROTECTED_VERIFIER_FD_V1: i32 = 196",
        "BEGIN_CHALLENGE_FD_V1: i32 = 197",
        "OWNER_PLAN_FD_V1: i32 = 198",
        "duplicate_inherited_fd",
        "libc::F_GETFD",
        "libc::F_DUPFD_CLOEXEC",
        "require_distinct_descriptors",
        "require_seqpacket",
        "read_sealed_owner_plan",
        "consume_canonical_slot",
        "admit_inherited_application_service",
        "admit_connected_path",
        "from_durable_reservation",
        "M1AllKernelsProductionProtectedVerifierV1::new",
        "bind_m1_authenticated_physical_runner_from_selector_v1",
        "initialize_m1_physical_runner_memory_v1",
        "new_with_s1_k4_resident_windows",
        "EXPECTED_SERVICE_EXCHANGES_V1",
        "coordinator.into_completed_backend()",
        "successful_ordered_measurements()",
        "terminate_with_quarantined_custody",
        "ManuallyDrop::new",
    ] {
        assert!(
            source.contains(required),
            "missing production owner step: {required}"
        );
    }
    for forbidden in [
        "M1AllKernelsProtectedVerifierV1::new",
        "reopen_m1_engineering_aggregate_artifact_v1",
        "bind_engineering_structural_m1_physical_runner_v1",
        "synthetic_for_test_only",
        "authority-free structural",
        "serve(Path::new(plan_path))",
    ] {
        assert!(
            !source.contains(forbidden),
            "forbidden fallback: {forbidden}"
        );
    }
}

#[test]
fn manifest_pins_all_direct_dependencies_and_release_abort_policy() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = fs::read_to_string(root.join("Cargo.toml")).unwrap();
    assert!(manifest.contains("panic = \"abort\""));
    assert!(manifest.contains("rust-version = \"1.97.1\""));
    assert!(manifest.contains("rev = \"3e3a77284a61654134211f8145dd0ddeebb2ff91\""));
    for dependency in ["rustix", "serde", "serde_json", "sha2"] {
        let line = manifest
            .lines()
            .find(|line| line.starts_with(dependency))
            .unwrap();
        assert!(line.contains('=') && line.contains('"'));
    }
}
