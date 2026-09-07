const MANIFEST: &str = include_str!("../Cargo.toml");
const SOURCE: &str = include_str!("../src/lib.rs");
const CLI_SOURCE: &str = include_str!("../src/bin/ferric-m1-engineering-target-smoke.rs");
const BOOTSTRAP_SOURCE: &str = include_str!("../src/bin/smoke_bootstrap.rs");
const R33_LIFECYCLE_SOURCE: &str = include_str!("../src/r33_lifecycle.rs");
const R33_PRODUCTION_BACKEND_SOURCE: &str = include_str!("../src/r33_production_backend.rs");
const R33_SERVICE_SOURCE: &str = include_str!("../src/r33_service.rs");
const R33_WIRE_SOURCE: &str = include_str!("../src/r33_wire.rs");
const R33_ADAPTER_SOURCE: &str = include_str!("../src/bin/ferric-m1-r33-adapter.rs");
const ROOT_MANIFEST: &str = include_str!("../../../Cargo.toml");
const ENGINE_MANIFEST: &str = include_str!("../../../crates/ferric-engine/Cargo.toml");
const ENGINE_LIB: &str = include_str!("../../../crates/ferric-engine/src/lib.rs");
const ENGINE_AUTHENTICATED_PREFILL_BOOTSTRAP_SOURCE: &str =
    include_str!("../../../crates/ferric-engine/src/authenticated_prefill_bootstrap.rs");
const ENGINE_AUTHENTICATED_TARGET_WINDOW_SOURCE: &str =
    include_str!("../../../crates/ferric-engine/src/authenticated_target_window_executor.rs");
const ENGINE_QUALIFICATION_CAPTURE_SOURCE: &str =
    include_str!("../../../crates/ferric-engine/src/bin/ferric-m1-qualification-capture.rs");
const CORE_SOURCE: &str =
    include_str!("../../../crates/ferric-engine/src/non_authoritative_program_artifact.rs");
const CAPABILITY_SOURCE: &str =
    include_str!("../../../crates/ferric-non-authoritative-program-source-v1/src/lib.rs");

const FE2O3_REVISION: &str = "be5668eaa71f8d60a0a5041891d25ce2ed9c2e6e";

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
    for required in [
        "ferric.m1-engineering-target-smoke-observation.v2",
        "ferric.m1-engineering-target-smoke-observation.v3",
        "@derive-engineering-identities-v1",
        "derived-engineering-observation-model-plan-v1",
        "execution.timing()",
        "\"benchmark_comparable\": false",
        "\"clock\": TIMING_CLOCK",
        "\"duration_boundary\": TIMING_BOUNDARY",
        "\"r33_tpot_eligible\": r33_tpot_eligible",
        "\"request_events\"",
        "not comparable to R33 serving, vLLM, or SGLang measurements",
    ] {
        assert!(
            CLI_SOURCE.contains(required),
            "engineering CLI is missing timing/nonclaim marker {required}"
        );
    }
    assert!(BOOTSTRAP_SOURCE.contains("M1PartitionedModelMemoryKvPoolV1"));
    for source in [CLI_SOURCE, BOOTSTRAP_SOURCE] {
        for forbidden in [
            "WorkerV3VerifierV1",
            "AuthenticatedWorkerV3ExecutableV1",
            "acquire_m1_all_kernels_authenticated_worker_v3_programs_v1",
            "\"benchmark_comparable\": true",
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
fn derived_engineering_identity_mode_is_coordinate_bound_and_authority_free() {
    let derived = BOOTSTRAP_SOURCE
        .split_once("fn derive_engineering_external_identity_inputs_v1")
        .map(|(_, tail)| tail)
        .and_then(|tail| {
            tail.split_once("impl SmokeBootstrapV1")
                .map(|(body, _)| body)
        })
        .expect("derived engineering identity implementation is a bounded source block");
    assert!(BOOTSTRAP_SOURCE.contains("ferric.m1.engineering-derived-external-identity.v1"));
    for required in [
        "DERIVED_ENGINEERING_COMPONENT_LABELS_V1",
        "observation.manifest.as_bytes()",
        "observation.hsaco.as_bytes()",
        "observation.compiler_handoff.as_bytes()",
        "observation.canonical_descriptor.as_bytes()",
        "observation.program_catalog.as_bytes()",
        "model.admission_record.as_bytes()",
        "model.model_bundle.as_bytes()",
        "model.target_prepacked.as_bytes()",
        "model.draft_prepacked.as_bytes()",
        "model.plan_catalog.as_bytes()",
        "expected_preliminary_kernel_catalog_identity",
        "expected_qwen3_gfx942_runner_source_identity",
    ] {
        assert!(
            derived.contains(required),
            "derived engineering identity block is missing {required}"
        );
    }
    assert_eq!(derived.matches("qualification_protocol").count(), 1);
    let unavoidable_field_removed = derived.replace("qualification_protocol", "");
    for forbidden in [
        "load_closure(",
        "OpenedKfd",
        "WorkerV3",
        "protected",
        "current",
        "qualification",
        "grants_load_authority",
        "grants_launch_authority",
        "bind_engineering_structural_m1_physical_runner_v1",
    ] {
        assert!(
            !unavoidable_field_removed.contains(forbidden),
            "derived engineering identity block contains forbidden authority marker {forbidden}"
        );
    }
    assert!(CLI_SOURCE.contains("\"authority\": \"none\""));
    assert!(CLI_SOURCE.contains("\"compiler_origin_authenticated\": false"));
    assert!(CLI_SOURCE.contains("\"current_publication_selected\": false"));
    assert!(CLI_SOURCE.contains("\"worker_v3_authenticated\": false"));
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

#[test]
fn r33_service_is_supervised_bounded_and_authority_free() {
    for required in [
        "socket_peercred",
        "expected_client_uid",
        "expected_daemon_uid",
        "service_plan_sha256",
        "M1_R33_SERVICE_PLAN_SHA256_ENV_V1",
        "response acknowledgement",
        "M1_R33_WINDOWS_PER_START_V1",
        "response_abandoned",
        "only-exact-stop-admitted",
        "HeldM1R33ServiceBundleV1",
        "M1R33AuthorityFreeBackendV1",
    ] {
        assert!(
            R33_SERVICE_SOURCE.contains(required),
            "R33 supervised service is missing {required}"
        );
    }
    for required in [
        "FRAME_MAGIC_V1",
        "Sha256::digest(&payload)",
        "M1_R33_MAX_WIRE_PAYLOAD_BYTES_V1",
        "NonCanonicalPayload",
        "trailing bytes",
    ] {
        assert!(
            R33_WIRE_SOURCE.contains(required),
            "R33 wire is missing {required}"
        );
    }
    assert!(R33_ADAPTER_SOURCE.contains("r33_service::adapter_main"));
    for source in [R33_SERVICE_SOURCE, R33_WIRE_SOURCE, R33_ADAPTER_SOURCE] {
        for forbidden in [
            "WorkerV3VerifierV1",
            "AuthenticatedWorkerV3ExecutableV1",
            "acquire_m1_all_kernels_authenticated_worker_v3_programs_v1",
            "TcpListener",
            "axum::",
            "hyper::",
        ] {
            assert!(
                !source.contains(forbidden),
                "R33 authority-free foundation contains forbidden marker {forbidden}"
            );
        }
    }
}

#[test]
fn r33_authenticated_backend_owns_only_pre_admitted_production_capabilities() {
    for required in [
        "M1AuthenticatedPhysicalRunnerV1",
        "M1PartitionedModelMemoryKvPoolV1",
        "BackendStateV1",
        "Dormant",
        "Active",
        "Faulted",
        "Stopped",
        "M1R33EngineV1::new",
        "authenticated-window-bootstrap-unavailable",
        "new_with_s1_t128_prefill_bootstrap",
        "new_with_s1_t128_target_window",
        "authenticated-window-execution-unavailable",
        "authenticated-window-execution-rejected",
        "prepare_m1_authenticated_s1_t128_prefill_prepublication_v1",
        "M1AuthenticatedTargetWindowClockStartV1::capture()",
        "execute_m1_authenticated_s1_t128_target_window_v1",
        "timing.first_token_offset_ns()",
        "timing.terminal_token_offset_ns()",
        "report.validate_against",
        "M1_R33_AUTHENTICATED_TARGET_WINDOWS_PER_INSTANCE_V1: usize = 1",
        "drop(custody)",
    ] {
        assert!(
            R33_PRODUCTION_BACKEND_SOURCE.contains(required),
            "R33 authenticated ownership backend is missing {required}"
        );
    }
    for forbidden in [
        "M1PhysicalRunnerV1",
        "M1QueuedServingPhysicalInputProviderV1",
        "execute_m1_target_smoke_v1",
        "M1EngineeringAggregateArtifactV1",
        "bind_engineering_structural_m1_physical_runner_v1",
        "M1AuthenticatedPhysicalQueueSessionV1",
        ".create(",
        ".submit(",
        ".wait(",
        ".wait_for(",
        ".recycle(",
        ".observe_completion(",
        ".check_completion(",
        "std::time::Instant",
        "TcpListener",
        "axum::",
        "hyper::",
    ] {
        assert!(
            !R33_PRODUCTION_BACKEND_SOURCE.contains(forbidden),
            "R33 authenticated ownership backend contains forbidden marker {forbidden}"
        );
    }
}

#[test]
fn authenticated_target_window_has_real_timing_and_checked_token_causality() {
    let production = ENGINE_AUTHENTICATED_TARGET_WINDOW_SOURCE
        .split_once("#[cfg(test)]")
        .map_or(ENGINE_AUTHENTICATED_TARGET_WINDOW_SOURCE, |(source, _)| {
            source
        });
    for required in [
        "ClockId::MonotonicRaw",
        "M1AuthenticatedTargetWindowClockStartV1",
        "execute_m1_authenticated_s1_t128_paired_prefill_v1",
        "observed.check_completion(&semantic)",
        "tokens.push(emitted)",
        "registry.preflight_publication",
        "registry.record_publication",
        "registry.preflight_completion_exact_for",
        "registry.apply_preflighted_completion",
        "released.shutdown_all_terminal_queue",
        "registry.remove_retired(request)",
    ] {
        assert!(
            production.contains(required),
            "authenticated target window is missing {required}"
        );
    }
    let adapter_clock = R33_PRODUCTION_BACKEND_SOURCE
        .find("M1AuthenticatedTargetWindowClockStartV1::capture()")
        .unwrap();
    let adapter_bootstrap = R33_PRODUCTION_BACKEND_SOURCE
        .find("prepare_m1_authenticated_s1_t128_prefill_prepublication_v1(")
        .unwrap();
    assert!(
        adapter_clock < adapter_bootstrap,
        "request-arrival clock must precede authenticated bootstrap"
    );
    for forbidden in [
        "std::time::Instant",
        "SystemTime",
        "execute_m1_target_smoke_v1",
        "M1PhysicalRunnerV1",
        ".expect(",
        "panic!(",
        "unreachable!(",
        "todo!(",
    ] {
        assert!(
            !production.contains(forbidden),
            "authenticated target window contains forbidden marker {forbidden}"
        );
    }
}

#[test]
fn authenticated_prefill_bootstrap_is_exact_owned_and_stops_before_execution() {
    let engine_production = ENGINE_AUTHENTICATED_PREFILL_BOOTSTRAP_SOURCE
        .split_once("#[cfg(test)]")
        .map_or(
            ENGINE_AUTHENTICATED_PREFILL_BOOTSTRAP_SOURCE,
            |(source, _)| source,
        );
    let backend_production = R33_PRODUCTION_BACKEND_SOURCE
        .split_once("#[cfg(test)]")
        .map_or(R33_PRODUCTION_BACKEND_SOURCE, |(source, _)| source);
    for required in [
        "M1AuthenticatedPhysicalRunnerV1",
        "M1PartitionedModelMemoryKvPoolV1",
        "M1CaptureQuarantinedEngineV1",
        "const PREFILL_WIDTH: usize = 128;",
        "PrefillS1T128",
        "SpeculativeS1K4C8192",
        "prompt_tokens.len() != PREFILL_WIDTH",
        "engine.admit()",
        "engine.append_tentative(request, 1)",
        "engine.dispatch_m1_ready()",
        "bind_m1_kv_workspace_table_v1",
        "reserve_step_write",
        "reserve_finite_speculative_rollover_outputs",
        "bind_m1_authenticated_speculative_rollover_intent_v1",
        "prepare_first_step",
        "into_m1_capture_quarantine",
    ] {
        assert!(
            ENGINE_AUTHENTICATED_PREFILL_BOOTSTRAP_SOURCE.contains(required),
            "authenticated prefill bootstrap is missing {required}"
        );
    }
    assert_eq!(
        R33_PRODUCTION_BACKEND_SOURCE
            .matches("authenticated-window-execution-unavailable")
            .count(),
        1,
        "R33 bootstrap must expose one stable no-execution fault"
    );
    for forbidden in [
        "M1PhysicalRunnerV1",
        "M1QueuedServingPhysicalInputProviderV1",
        "M1EngineeringAggregateArtifactV1",
        "M1AuthenticatedPhysicalQueueSessionV1",
        ".create(",
        ".submit(",
        ".wait(",
        ".wait_for(",
        ".recycle(",
        ".observe_completion(",
        ".check_completion(",
        "ClockId",
        "duration_ns",
        "M1R33MeasurementReportV1",
    ] {
        assert!(
            !ENGINE_AUTHENTICATED_PREFILL_BOOTSTRAP_SOURCE.contains(forbidden),
            "authenticated prefill bootstrap contains forbidden execution marker {forbidden}"
        );
    }
    let allowed_engine_calls = [
        ("self.engine.is_faulted()", 2),
        ("engine.into_m1_capture_quarantine()", 1),
        ("engine.is_faulted()", 3),
        ("engine.live_count()", 1),
        ("engine.completed_epoch()", 1),
        ("engine.admit()", 1),
        ("engine.append_tentative(request, 1)", 1),
        ("engine.dispatch_m1_ready()", 1),
    ];
    for (call, expected) in allowed_engine_calls {
        assert_eq!(
            engine_production.matches(call).count(),
            expected,
            "authenticated prefill bootstrap Engine call allowlist drifted at {call}"
        );
    }
    assert!(
        ENGINE_QUALIFICATION_CAPTURE_SOURCE
            .contains("fn admitted_mi300x_runs_public_authenticated_rollover_executor()")
    );
    assert!(
        ENGINE_QUALIFICATION_CAPTURE_SOURCE
            .contains("prepare_m1_authenticated_s1_t128_prefill_prepublication_v1(")
    );
    for forbidden in [".expect(", "panic!(", "unreachable!(", "todo!("] {
        assert!(
            !engine_production.contains(forbidden),
            "authenticated prefill production seam contains {forbidden}"
        );
        assert!(
            !backend_production.contains(forbidden),
            "R33 authenticated backend production contains {forbidden}"
        );
    }
}
