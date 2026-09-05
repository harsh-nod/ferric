const MANIFEST: &str = include_str!("../Cargo.toml");
const SOURCE: &str = include_str!("../src/lib.rs");
const CLI_SOURCE: &str = include_str!("../src/bin/ferric-m1-engineering-target-smoke.rs");
const BOOTSTRAP_SOURCE: &str = include_str!("../src/bin/smoke_bootstrap.rs");
const R33_LIFECYCLE_SOURCE: &str = include_str!("../src/r33_lifecycle.rs");
const ROOT_MANIFEST: &str = include_str!("../../../Cargo.toml");
const ENGINE_MANIFEST: &str = include_str!("../../../crates/ferric-engine/Cargo.toml");
const ENGINE_LIB: &str = include_str!("../../../crates/ferric-engine/src/lib.rs");
const CORE_SOURCE: &str =
    include_str!("../../../crates/ferric-engine/src/non_authoritative_program_artifact.rs");
const CAPABILITY_SOURCE: &str =
    include_str!("../../../crates/ferric-non-authoritative-program-source-v1/src/lib.rs");

const FE2O3_REVISION: &str = "16da71edd823e0d5c16529bfbbedb4f9dd8e70c6";

#[test]
fn adapter_is_an_exact_standalone_workspace() {
    assert_eq!(MANIFEST.matches("[workspace]").count(), 1);
    let root = toml::from_str::<toml::Value>(ROOT_MANIFEST).unwrap();
    let workspace = root
        .get("workspace")
        .and_then(toml::Value::as_table)
        .unwrap();
    let members = workspace
        .get("members")
        .and_then(toml::Value::as_array)
        .unwrap();
    let exclude = workspace
        .get("exclude")
        .and_then(toml::Value::as_array)
        .unwrap();
    assert!(members.iter().any(|entry| {
        entry.as_str() == Some("crates/ferric-non-authoritative-program-source-v1")
    }));
    assert!(
        exclude
            .iter()
            .any(|entry| { entry.as_str() == Some("adapters/m1-engineering-execution-v1") })
    );
    assert!(
        !members
            .iter()
            .any(|entry| { entry.as_str() == Some("adapters/m1-engineering-execution-v1") })
    );
    assert!(!MANIFEST.contains("package.metadata.verus"));
    assert!(!MANIFEST.contains("optional = true"));
}

#[test]
fn adapter_and_observation_schema_pin_current_fe2o3() {
    assert_eq!(MANIFEST.matches(FE2O3_REVISION).count(), 4);
    assert!(SOURCE.contains(FE2O3_REVISION));
}

#[test]
fn production_engine_has_no_engineering_dependency_or_feature_edge() {
    for forbidden in [
        "engineering-non-authoritative-hsaco",
        "engineering-non-authoritative-execution",
        "dep:serde",
        "fe2o3-hsaco-finalize = { workspace = true",
    ] {
        assert!(
            !ENGINE_MANIFEST.contains(forbidden),
            "engine manifest contains forbidden engineering edge {forbidden}"
        );
    }
    assert!(!ENGINE_LIB.contains("mod engineering_aggregate_artifact"));
    assert!(!ENGINE_LIB.contains("pub use engineering_aggregate_artifact"));
}

#[test]
fn adapter_depends_one_way_on_authority_free_core() {
    let core_code = CORE_SOURCE
        .lines()
        .filter(|line| !line.trim_start().starts_with("///"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(MANIFEST.contains("ferric-engine = { path = \"../../crates/ferric-engine\""));
    assert!(MANIFEST.contains(
        "ferric-non-authoritative-program-source-v1 = { path = \"../../crates/ferric-non-authoritative-program-source-v1\""
    ));
    assert!(ENGINE_MANIFEST.contains(
        "ferric-non-authoritative-program-source-v1 = { path = \"../ferric-non-authoritative-program-source-v1\" }"
    ));
    assert!(!ENGINE_MANIFEST.contains("m1-engineering-execution-v1"));
    assert!(SOURCE.contains("admit_m1_non_authoritative_program_artifact_v1"));
    assert!(SOURCE.contains("bind_engineering_structural_m1_physical_runner_v1"));
    assert!(!SOURCE.contains("pub fn into_structural_artifact"));
    assert!(CORE_SOURCE.contains("pub struct M1NonAuthoritativeProgramArtifactV1"));
    assert!(CAPABILITY_SOURCE.contains("pub struct M1NonAuthoritativeProgramSourceCapabilityV1"));
    assert!(
        CORE_SOURCE.contains("source_capability: Box<M1NonAuthoritativeProgramSourceCapabilityV1>")
    );
    assert!(CORE_SOURCE.contains("pub const fn grants_publication_authority"));
    assert!(CORE_SOURCE.contains("pub const fn grants_load_authority"));
    assert!(CORE_SOURCE.contains("pub const fn grants_launch_authority"));
    assert!(!core_code.contains("M1AuthenticatedWorkerV3ProgramSetV1"));
    assert!(!core_code.contains("M1AuthenticatedPhysicalRunnerV1"));
    assert!(!ENGINE_LIB.contains("pub use ferric_non_authoritative_program_source_v1"));
    assert!(!ENGINE_LIB.contains("from_observed_engineering_parts_v1"));
}

#[test]
fn adapter_library_imports_no_kfd_or_authenticated_publication_authority() {
    for forbidden in [
        "fe2o3_kfd",
        "WorkerV3VerifierV1",
        "AuthenticatedWorkerV3ExecutableV1",
        "acquire_m1_all_kernels_authenticated_worker_v3_programs_v1",
    ] {
        assert!(
            !SOURCE.contains(forbidden),
            "adapter source contains forbidden authority marker {forbidden}"
        );
    }
}

#[test]
fn adapter_owned_cli_is_the_only_kfd_execution_boundary() {
    assert!(CLI_SOURCE.contains("ferric-m1-engineering-target-smoke"));
    assert!(CLI_SOURCE.contains("bind_engineering_structural_m1_physical_runner_v1"));
    assert!(CLI_SOURCE.contains("OpenedKfd::open_default"));
    assert!(BOOTSTRAP_SOURCE.contains("M1PartitionedModelMemoryKvPoolV1"));
    for source in [CLI_SOURCE, BOOTSTRAP_SOURCE] {
        for forbidden in [
            "WorkerV3VerifierV1",
            "AuthenticatedWorkerV3ExecutableV1",
            "acquire_m1_all_kernels_authenticated_worker_v3_programs_v1",
            "current_publication_selected\": true",
            "worker_v3_authenticated\": true",
        ] {
            assert!(
                !source.contains(forbidden),
                "engineering CLI contains forbidden authority marker {forbidden}"
            );
        }
    }
}

#[test]
fn r33_lifecycle_is_bounded_real_clocked_and_bound_to_production_operations() {
    for required in [
        "ClockId::MonotonicRaw",
        "M1_MAX_ACTIVE_SEQUENCES",
        "M1ServingPhysicalRunnerOperationsV1",
        "M1QueuedServingPhysicalInputProviderV1",
        "M1CheckedCompletionOutputV1",
        "engine.admit()",
        ".admit(request, prefill)",
        "preflight_first_publication_work",
        "checked_completion_for_readback",
        "observe_physical_readback",
        "observe_terminal_after_settlement",
        "arrival_offset_ns",
        "first_token_offset_ns",
        "terminal_offset_ns",
    ] {
        assert!(
            R33_LIFECYCLE_SOURCE.contains(required),
            "R33 lifecycle is missing required production marker {required}"
        );
    }
    assert_eq!(R33_LIFECYCLE_SOURCE.matches("pub fn admit(").count(), 1);
    assert!(!R33_LIFECYCLE_SOURCE.contains("pub fn observe_output("));
    assert!(!R33_LIFECYCLE_SOURCE.contains("pub fn observe_terminal("));
    for forbidden in [
        "std::time::Instant",
        "SystemTime",
        "thread::sleep",
        "TcpListener",
        "hyper::",
        "axum::",
    ] {
        assert!(
            !R33_LIFECYCLE_SOURCE.contains(forbidden),
            "R33 lifecycle contains forbidden timing or HTTP marker {forbidden}"
        );
    }
}
