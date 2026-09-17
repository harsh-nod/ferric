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
fn finite_resident_selection_is_only_from_the_sealed_canonical_owner_plan() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_file = fs::read_to_string(root.join("src/main.rs")).unwrap();
    let source = source_file.split("#[cfg(test)]").next().unwrap();
    for required in [
        "enum OwnerSpeculativeWindowV1",
        "#[serde(rename = \"s1-k4\")]",
        "#[serde(rename = \"s1-k8\")]",
        "#[serde(rename = \"s1-k16\")]",
        "#[serde(deny_unknown_fields)]",
        "skip_serializing_if = \"Option::is_none\"",
        "deserialize_with = \"deserialize_speculative_window\"",
        "let value = String::deserialize(deserializer)?;",
        "speculative_window: Option<OwnerSpeculativeWindowV1>",
        "if canonical != bytes",
        "let plan = decode_owner_plan(&owner_plan_bytes)?;",
        "let windows = resident_windows(&plan, owner_plan_sha256, &bundle)?;",
        "resident_window_input(plan, owner_plan_sha256, window_index, request)?",
        "M1R33AuthenticatedResidentWindowBindingV1::bind(window, input)",
        "new_with_s1_k4_resident_windows",
        "new_with_s1_finite_resident_windows",
    ] {
        assert!(
            source.contains(required),
            "lost checked selection path: {required}"
        );
    }
    let input = source
        .split_once("fn resident_window_input(")
        .unwrap()
        .1
        .split_once("struct SnapshotFileV1")
        .unwrap()
        .0;
    for required in [
        "let target_speculative = plan.target_speculative();",
        "M1AuthenticatedS1T128PrefillBootstrapInputV1::new(",
        "M1AuthenticatedS1T128PrefillBootstrapInputV1::new_with_speculative_successor(",
        "M1AuthenticatedResidentRoundPlansV1::new(speculative()?, speculative()?)",
        "M1AuthenticatedResidentWindowInputV1::new(",
        "M1AuthenticatedResidentWindowInputV1::new_with_speculative_successor(",
    ] {
        assert!(
            input.contains(required),
            "lost input selection binding: {required}"
        );
    }
    assert_eq!(input.matches("target_speculative,").count(), 2);
    assert_eq!(input.matches("match plan.speculative_window").count(), 2);
    for forbidden in ["std::env", "TARGET_SPECULATIVE", "unwrap_or("] {
        assert!(
            !input.contains(forbidden),
            "selection fallback in resident input: {forbidden}"
        );
    }
    let selection = source
        .split_once("fn target_speculative(&self)")
        .unwrap()
        .1
        .split_once("fn validate(&self)")
        .unwrap()
        .0;
    assert!(selection.contains("self.speculative_window"));
    assert!(selection.contains("unwrap_or(OwnerSpeculativeWindowV1::K4)"));
    assert!(!selection.contains("std::env"));
    let backend = source
        .split_once("let backend = match plan.speculative_window")
        .unwrap()
        .1
        .split_once("let server =")
        .unwrap()
        .0;
    let (legacy, finite) = backend.split_once("Some(_) =>").unwrap();
    assert!(legacy.contains("None =>"));
    assert!(legacy.contains("new_with_s1_k4_resident_windows"));
    assert!(!legacy.contains("new_with_s1_finite_resident_windows"));
    assert!(finite.contains("new_with_s1_finite_resident_windows"));
    assert!(!finite.contains("new_with_s1_k4_resident_windows"));
}

#[test]
fn manifest_pins_all_direct_dependencies_and_release_abort_policy() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = fs::read_to_string(root.join("Cargo.toml")).unwrap();
    assert!(manifest.contains("panic = \"abort\""));
    assert!(manifest.contains("rust-version = \"1.97.1\""));
    assert!(manifest.contains("rev = \"9989714525028db2e73bacfaa944f97fd3d5eeff\""));
    for dependency in ["rustix", "serde", "serde_json", "sha2"] {
        let line = manifest
            .lines()
            .find(|line| line.starts_with(dependency))
            .unwrap();
        assert!(line.contains('=') && line.contains('"'));
    }
}
