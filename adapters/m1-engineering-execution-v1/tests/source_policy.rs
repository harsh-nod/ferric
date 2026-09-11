const MANIFEST: &str = include_str!("../Cargo.toml");
const SOURCE: &str = include_str!("../src/lib.rs");
const CLI_SOURCE: &str = include_str!("../src/bin/ferric-m1-engineering-target-smoke.rs");
const SPECULATIVE_CLI_SOURCE: &str =
    include_str!("../src/bin/ferric-m1-engineering-speculative-smoke.rs");
const R29_TECHNICAL_CLI_SOURCE: &str =
    include_str!("../src/bin/ferric-m1-engineering-r29-capture.rs");
const BOOTSTRAP_SOURCE: &str = include_str!("../src/bin/smoke_bootstrap.rs");
const STARTUP_DIAGNOSTICS_SOURCE: &str = include_str!("../src/bin/startup_diagnostics.rs");
const ADAPTER_README: &str = include_str!("../README.md");
const R33_LIFECYCLE_SOURCE: &str = include_str!("../src/r33_lifecycle.rs");
const R33_PRODUCTION_BACKEND_SOURCE: &str = include_str!("../src/r33_production_backend.rs");
const R33_RESIDENT_SESSION_SOURCE: &str = include_str!("../src/r33_resident_session.rs");
const R33_RESIDENT_VAULT_SOURCE: &str = include_str!("../src/r33_resident_session/vault.rs");
const R33_SERVICE_SOURCE: &str = include_str!("../src/r33_service.rs");
const R33_WIRE_SOURCE: &str = include_str!("../src/r33_wire.rs");
const R33_ADAPTER_SOURCE: &str = include_str!("../src/bin/ferric-m1-r33-adapter.rs");
const R33_VAULT_ABORT_PROBE_SOURCE: &str =
    include_str!("../src/bin/ferric-r33-custody-vault-abort-probe.rs");
const ROOT_MANIFEST: &str = include_str!("../../../Cargo.toml");
const ENGINE_MANIFEST: &str = include_str!("../../../crates/ferric-engine/Cargo.toml");
const ENGINE_LIB: &str = include_str!("../../../crates/ferric-engine/src/lib.rs");
const ENGINE_AUTHENTICATED_PREFILL_BOOTSTRAP_SOURCE: &str =
    include_str!("../../../crates/ferric-engine/src/authenticated_prefill_bootstrap.rs");
const ENGINE_AUTHENTICATED_TARGET_WINDOW_SOURCE: &str =
    include_str!("../../../crates/ferric-engine/src/authenticated_target_window_executor.rs");
const ENGINE_AUTHENTICATED_RESIDENT_SOURCE: &str =
    include_str!("../../../crates/ferric-engine/src/authenticated_resident_session.rs");
const ENGINE_SERVING_PHYSICAL_INPUT_PROVIDER_SOURCE: &str =
    include_str!("../../../crates/ferric-engine/src/m1_serving_physical_input_provider.rs");
const ENGINE_QUALIFICATION_CAPTURE_SOURCE: &str =
    include_str!("../../../crates/ferric-engine/src/bin/ferric-m1-qualification-capture.rs");
const CORE_SOURCE: &str =
    include_str!("../../../crates/ferric-engine/src/non_authoritative_program_artifact.rs");
const CAPABILITY_SOURCE: &str =
    include_str!("../../../crates/ferric-non-authoritative-program-source-v1/src/lib.rs");

const FE2O3_REVISION: &str = "21682228486f7186cc3c37ddf165fffc438d8b6a";

#[test]
fn adapter_is_an_exact_standalone_workspace() {
    assert_eq!(MANIFEST.matches("[workspace]").count(), 1);
    assert!(MANIFEST.contains("edition = \"2024\""));
    assert!(ROOT_MANIFEST.contains("edition = \"2021\""));
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
    let manifest = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    let dependencies = manifest
        .get("dependencies")
        .and_then(toml::Value::as_table)
        .unwrap();
    let service_host = dependencies
        .get("fe2o3-service-host")
        .and_then(toml::Value::as_table)
        .unwrap();
    assert_eq!(
        service_host.get("optional").and_then(toml::Value::as_bool),
        Some(true)
    );
    let mut optional_dependencies = dependencies
        .iter()
        .filter_map(|(name, dependency)| {
            dependency
                .as_table()
                .and_then(|details| details.get("optional"))
                .and_then(toml::Value::as_bool)
                .filter(|optional| *optional)
                .map(|_| name.as_str())
        })
        .collect::<Vec<_>>();
    optional_dependencies.sort_unstable();
    assert_eq!(
        optional_dependencies,
        [
            "fe2o3-hsaco",
            "fe2o3-service-host",
            "ferric-qwen3-tp-batch-kernels-device-v2",
            "ferric-qwen3-tp-batch32-kernels-device-v5",
            "ferric-qwen3-tp-fp32-head-kernels-device-v7",
            "ferric-qwen3-tp-fp32-head32-kernels-device-v8",
            "ferric-qwen3-tp-kernels-device-v1",
            "ferric-qwen3-tp-large-kv-kernels-device-v9",
            "ferric-qwen3-tp-peer-kernels-device-v4",
            "ferric-qwen3-tp-peer32-kernels-device-v6",
            "ferric-qwen3-tp-perf-kernels-device-v3"
        ]
    );
    let features = manifest
        .get("features")
        .and_then(toml::Value::as_table)
        .unwrap();
    assert!(!features.contains_key("default"));
    assert_eq!(
        features
            .get("qualification-fault-injection")
            .and_then(toml::Value::as_array)
            .unwrap()
            .iter()
            .map(toml::Value::as_str)
            .collect::<Vec<_>>(),
        vec![
            Some("ferric-engine/qualification-fault-injection"),
            Some("dep:fe2o3-service-host"),
        ]
    );
}

#[test]
fn batched_paged_runtime_requires_a_separate_engineering_opt_in() {
    let manifest = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    let feature = manifest["features"]["tp-batch-engineering"]
        .as_array()
        .unwrap();
    assert_eq!(
        feature.iter().map(toml::Value::as_str).collect::<Vec<_>>(),
        vec![
            Some("tp-engineering"),
            Some("dep:ferric-qwen3-tp-batch-kernels-device-v2"),
            Some("dep:ferric-qwen3-tp-perf-kernels-device-v3"),
            Some("dep:ferric-qwen3-tp-peer-kernels-device-v4"),
            Some("dep:ferric-qwen3-tp-batch32-kernels-device-v5"),
            Some("dep:ferric-qwen3-tp-peer32-kernels-device-v6"),
            Some("dep:ferric-qwen3-tp-fp32-head-kernels-device-v7"),
            Some("dep:ferric-qwen3-tp-fp32-head32-kernels-device-v8"),
            Some("dep:ferric-qwen3-tp-large-kv-kernels-device-v9")
        ]
    );
    let bin = manifest["bin"]
        .as_array()
        .unwrap()
        .iter()
        .find(|bin| bin["name"].as_str() == Some("ferric-qwen3-tp-batch-engineering"))
        .unwrap();
    assert_eq!(
        bin["required-features"].as_array().unwrap(),
        &vec![toml::Value::String("tp-batch-engineering".into())]
    );
    for module in ["tp_paged", "tp_scheduler", "tp_batch_runtime"] {
        assert!(SOURCE.contains(&format!(
            "#[cfg(feature = \"tp-batch-engineering\")]\npub mod {module};"
        )));
    }
}

#[test]
fn adapter_and_observation_schema_pin_current_fe2o3() {
    assert_eq!(MANIFEST.matches(FE2O3_REVISION).count(), 6);
    assert!(SOURCE.contains(FE2O3_REVISION));
}

#[test]
fn standalone_draft_canary_is_separately_opted_in_without_new_kernel_dependencies() {
    let manifest = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    let binary = manifest["bin"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"].as_str() == Some("ferric-qwen3-draft-canary"))
        .unwrap();
    assert_eq!(
        binary["required-features"].as_array().unwrap(),
        &vec![toml::Value::String("tp-batch-engineering".into())]
    );
    let canary = include_str!("../src/bin/ferric-qwen3-draft-canary.rs");
    assert!(canary.contains("EngineeringQwenModelV1::open_with_draft"));
    assert!(canary.contains("compiler_expectation_roster_v1()"));
    assert!(!canary.contains("configure_projection"));
    assert!(!canary.contains("configure_ordered_batches"));
    assert!(!ENGINE_MANIFEST.contains("ferric-qwen3-draft-canary"));
}

#[test]
fn tensor_parallel_runtime_is_opt_in_and_confined_to_its_engineering_binary() {
    let manifest = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    let feature = manifest["features"]["tp-engineering"].as_array().unwrap();
    assert_eq!(
        feature.iter().map(toml::Value::as_str).collect::<Vec<_>>(),
        vec![
            Some("fe2o3-kfd/engineering-gfx950"),
            Some("dep:fe2o3-hsaco"),
            Some("dep:ferric-qwen3-tp-kernels-device-v1"),
        ]
    );
    let binary = manifest["bin"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"].as_str() == Some("ferric-qwen3-tp-engineering"))
        .unwrap();
    assert_eq!(
        binary["required-features"].as_array().unwrap()[0].as_str(),
        Some("tp-engineering")
    );
    assert!(!SOURCE.contains("engineering_wire"));
    assert!(!ENGINE_MANIFEST.contains("engineering-gfx950"));
    assert!(!ENGINE_MANIFEST.contains("ferric-qwen3-tp-kernels-device-v1"));
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
fn engineering_startup_diagnostics_are_opt_in_stderr_only_and_non_authoritative() {
    for required in [
        "FERRIC_M1_ENGINEERING_STARTUP_PHASE_DIAGNOSTICS_V1",
        "FERRIC_M1_ENGINEERING_STARTUP_PHASE_V1",
        "std::env::var_os",
        "Some(OsStr::new(STARTUP_PHASE_DIAGNOSTICS_OPT_IN_V1))",
        "std::io::stderr().lock()",
        "scope=engineering-only authority=none evidence=false",
        "benchmark_comparable=false clock=std-instant-cumulative",
        "let _ = writeln!",
    ] {
        assert!(
            STARTUP_DIAGNOSTICS_SOURCE.contains(required),
            "startup diagnostic helper is missing {required}"
        );
    }
    for phase in [
        "artifact-admission",
        "cpu-model-bootstrap-preparation",
        "runner-bind",
        "kfd-bind",
        "initialize-memory-allocation-upload",
        "controller-execution",
    ] {
        assert!(
            STARTUP_DIAGNOSTICS_SOURCE.contains(phase),
            "startup diagnostic helper is missing phase ID {phase}"
        );
    }
    for forbidden in ["stdout", "serde_json", "prompt", "token", "model_bundle"] {
        assert!(
            !STARTUP_DIAGNOSTICS_SOURCE.contains(forbidden),
            "startup diagnostic helper contains payload or stdout marker {forbidden}"
        );
    }
    assert!(ADAPTER_README.contains("`FERRIC_M1_ENGINEERING_STARTUP_PHASE_DIAGNOSTICS_V1=1`"));
    assert!(ADAPTER_README.contains("do not change"));
    assert!(ADAPTER_README.contains("the stdout observation schema"));
}

#[test]
fn both_engineering_smokes_complete_the_same_six_startup_boundaries_in_order() {
    const COMPLETIONS: [&str; 6] = [
        "diagnostics.completed(EngineeringStartupPhaseV1::ArtifactAdmission)",
        "diagnostics.completed(EngineeringStartupPhaseV1::CpuModelBootstrapPreparation)",
        "diagnostics.completed(EngineeringStartupPhaseV1::RunnerBind)",
        "diagnostics.completed(EngineeringStartupPhaseV1::KfdBind)",
        "diagnostics.completed(EngineeringStartupPhaseV1::InitializeMemoryAllocationUpload)",
        "diagnostics.completed(EngineeringStartupPhaseV1::ControllerExecution)",
    ];
    const OPERATIONS: [&str; 6] = [
        "let artifact = reopen_m1_engineering_aggregate_artifact_v1",
        "smoke_bootstrap::prepare(",
        "let bound = bootstrap.bind(",
        "let checked = OpenedKfd::open_default()",
        "let initialized = bound.initialize_memory(checked)?",
        "shutdown_all_terminal_queue",
    ];

    for source in [CLI_SOURCE, SPECULATIVE_CLI_SOURCE] {
        assert!(source.contains("mod startup_diagnostics;"));
        assert!(source.contains("EngineeringStartupDiagnosticsV1::from_process_environment()"));
        assert_eq!(source.matches("diagnostics.completed(").count(), 6);

        let completion_offsets = COMPLETIONS.map(|completion| {
            source
                .find(completion)
                .unwrap_or_else(|| panic!("engineering smoke is missing {completion}"))
        });
        assert!(completion_offsets.windows(2).all(|pair| pair[0] < pair[1]));

        for (operation, completion) in OPERATIONS[..5].iter().zip(&COMPLETIONS[..5]) {
            assert!(
                source.find(operation).unwrap() < source.find(completion).unwrap(),
                "completion {completion} must follow successful operation {operation}"
            );
        }
    }

    let target_controller = CLI_SOURCE
        .find("let execution = execute_m1_target_smoke_v1")
        .unwrap();
    assert!(target_controller < CLI_SOURCE.find(COMPLETIONS[5]).unwrap());
    let speculative_shutdown = SPECULATIVE_CLI_SOURCE.find(OPERATIONS[5]).unwrap();
    let speculative_duration = SPECULATIVE_CLI_SOURCE
        .find("let speculative_duration_ns =")
        .unwrap();
    let speculative_completion = SPECULATIVE_CLI_SOURCE.find(COMPLETIONS[5]).unwrap();
    assert!(speculative_shutdown < speculative_duration);
    assert!(speculative_duration < speculative_completion);
}

#[test]
fn engineering_r29_capture_is_aggregate_only_and_explicitly_non_authoritative() {
    assert!(MANIFEST.contains("name = \"ferric-m1-engineering-r29-capture\""));
    assert_eq!(
        R29_TECHNICAL_CLI_SOURCE.matches("#[rustfmt::skip]").count(),
        1
    );
    for allowed_root_lint in [
        "clippy::manual_let_else",
        "clippy::needless_pass_by_value",
        "clippy::semicolon_if_nothing_returned",
        "clippy::used_underscore_binding",
    ] {
        assert_eq!(
            R29_TECHNICAL_CLI_SOURCE.matches(allowed_root_lint).count(),
            1,
            "engineering R29 capture must mirror the owning root lint exactly once: {allowed_root_lint}"
        );
    }
    for required in [
        "Mirror only the owning root workspace's lint policy for this shared module.",
        "#[rustfmt::skip] // Skip cross-edition traversal only; the root workspace formats this shared module.",
        "reopen_m1_engineering_aggregate_artifact_v1",
        "bind_engineering_structural_m1_physical_runner_v1",
        "run_technical_r29_capture",
        "generate_technical_r29_inputs",
        "validate_technical_r29_inputs",
        "EngineeringAggregateProgramSourceV1",
        "ferric-m1-engineering-r29-capture",
        "../../../../crates/ferric-engine/src/bin/ferric-m1-qualification-capture.rs",
    ] {
        assert!(
            R29_TECHNICAL_CLI_SOURCE.contains(required),
            "engineering R29 capture is missing {required}"
        );
    }
    for forbidden in [
        "clippy::all",
        "clippy::pedantic",
        "reopen_persisted_m1_kernel_artifacts_v1",
        "require_m1_authenticated_roster_acquisition_v1",
        "M1AuthenticatedWorkerV3ProgramSetV1",
        "M1AuthenticatedPhysicalRunnerV1",
        "AuthenticatedWorkerV3ExecutableV1",
    ] {
        assert!(
            !R29_TECHNICAL_CLI_SOURCE.contains(forbidden),
            "engineering R29 capture contains forbidden authority marker {forbidden}"
        );
    }

    for required in [
        "TECHNICAL_TRANSCRIPT_FORMAT",
        "TECHNICAL_TRANSCRIPT_NONCLAIM",
        "CapturePurposeV1::TechnicalPrequalification",
        "ENGINEERING-OBSERVATION-DIRECTORY",
        "\"artifact_authority\".to_owned(), json!(\"none\")",
        "require_m1_authenticated_roster_acquisition_v1(root)",
        "reopen_persisted_m1_kernel_artifacts_v1(root)",
        "prefill_semantic_join_diagnostic={}",
        "failure.destroy_queue_and_retain_evidence(engine)",
    ] {
        assert!(
            ENGINE_QUALIFICATION_CAPTURE_SOURCE.contains(required),
            "shared R29 capture is missing separation marker {required}"
        );
    }
    assert!(ENGINE_QUALIFICATION_CAPTURE_SOURCE.contains(
        "run_capture_with_program_source::<PersistedM1R29CaptureProgramSourceV1>(arguments, purpose)"
    ));
}

#[test]
fn engineering_speculative_smoke_is_inventory_bound_and_authority_free() {
    assert!(MANIFEST.contains("name = \"ferric-m1-engineering-speculative-smoke\""));
    assert!(MANIFEST.contains("path = \"src/bin/ferric-m1-engineering-speculative-smoke.rs\""));
    for required in [
        "ferric.m1-engineering-speculative-smoke-observation.v1",
        "bind_engineering_structural_m1_physical_runner_v1",
        "reserve_s1_k4_rollover_output",
        "GFX942_MAX_FIXED_DISPATCH_DATA_V1",
        "schedule_m1_finite_speculative_queue_rollover_v1",
        "reserve_m1_finite_speculative_queue_rollover_kv_v1",
        "prepare_m1_finite_speculative_queue_rollover_v1",
        "submit_finite_speculative_rollover",
        "observe_direct_diagnostic_choices",
        "read_and_check_speculative_k4_diagnostic_completion",
        "\"authority\": \"none\"",
        "\"artifact_authority\": \"none\"",
        "\"benchmark_comparable\": false",
        "\"authenticated_AB_exercised\": false",
        "\"compiler_origin_authenticated\": false",
        "\"current_publication_selected\": false",
        "\"worker_v3_authenticated\": false",
        "structural-host-fixture-only-no-token-or-completion-oracle",
        "active-token-fill-not-attention-mask-padding",
        "4-model-memory+2-paired-workspaces+3-k4-successor-output+1-prefill-compact+1-prefill-direct-choice",
    ] {
        assert!(
            SPECULATIVE_CLI_SOURCE.contains(required),
            "engineering speculative smoke is missing {required}"
        );
    }
    for forbidden in [
        "M1AuthenticatedPhysicalRunnerV1",
        "acquire_m1_all_kernels_authenticated_worker_v3_programs_v1",
        "WorkerV3VerifierV1",
        "AuthenticatedWorkerV3ExecutableV1",
        "prepare_m1_swiglu_protected_verifier_request_v1",
        "reserve_finite_speculative_rollover_outputs",
        "require_current_",
        "\"benchmark_comparable\": true",
        "\"authenticated_AB_exercised\": true",
        "\"current_publication_selected\": true",
        "\"worker_v3_authenticated\": true",
        ".expect(",
        "panic!(",
        "unreachable!(",
        "todo!(",
    ] {
        assert!(
            !SPECULATIVE_CLI_SOURCE.contains(forbidden),
            "engineering speculative smoke contains forbidden marker {forbidden}"
        );
    }
}

#[test]
fn engineering_smoke_runs_two_independent_admissions_once_and_joins_fail_closed() {
    let parallel = BOOTSTRAP_SOURCE
        .split_once("fn authenticate_model_inputs_in_parallel_v1")
        .map(|(_, tail)| tail)
        .and_then(|tail| {
            tail.split_once("fn derive_engineering_external_identity_inputs_v1")
                .map(|(body, _)| body)
        })
        .expect("parallel authentication helper is a bounded source block");

    assert_eq!(
        parallel
            .matches("authenticate(ModelAuthenticationPurposeV1::Runner)")
            .count(),
        1
    );
    assert_eq!(
        parallel
            .matches("authenticate(ModelAuthenticationPurposeV1::Memory)")
            .count(),
        1
    );
    for required in [
        ".spawn_scoped(scope,",
        "let runner_result =",
        ".join()",
        "match runner_result",
        "Ok(runner) => memory_result.map(|memory| (runner, memory))",
        "Err(error) => Err(error)",
        "memory model authentication worker panicked",
    ] {
        assert!(
            parallel.contains(required),
            "parallel authentication helper is missing {required}"
        );
    }
    assert!(
        parallel.find(".join()").unwrap() < parallel.find("match runner_result").unwrap(),
        "memory authentication must be joined before returning the runner result"
    );
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
fn r33_resident_custody_vault_is_private_held_and_abort_on_unwind() {
    let production_session_source = R33_RESIDENT_SESSION_SOURCE
        .split("#[cfg(test)]")
        .next()
        .unwrap();
    assert!(SOURCE.contains("pub(crate) mod r33_resident_session;"));
    for required in [
        "HeldM1R33ServiceBundleV1",
        "M1R33OutstandingWindowV1<'_",
        "session: &'a mut M1R33ResidentSessionV1",
        "vault::ExecutionCapability<'_",
        "bundle.revalidate()",
        "expected_start + sequence as u64",
        "quarantine_input",
        "cancel_bound",
    ] {
        assert!(
            R33_RESIDENT_SESSION_SOURCE.contains(required),
            "R33 resident shell is missing {required}"
        );
    }
    for required in [
        "ManuallyDrop<C>",
        "ManuallyDrop<I>",
        "struct ExecutionCapability",
        "std::process::abort()",
        "run_external_abort_probe_v1",
        "let _quarantined_input = self.input.take();",
        "let _quarantined_custody = self.custody.take();",
    ] {
        assert!(
            R33_RESIDENT_VAULT_SOURCE.contains(required),
            "R33 private custody vault is missing {required}"
        );
    }
    assert!(!R33_RESIDENT_VAULT_SOURCE.contains("core::mem::forget("));
    assert!(!production_session_source.contains("FnOnce"));
    assert!(!production_session_source.contains("AtomicU64"));
    assert!(R33_VAULT_ABORT_PROBE_SOURCE.contains("run_external_abort_probe_v1"));
    assert!(MANIFEST.contains("name = \"ferric-r33-custody-vault-abort-probe\""));
    let manifest = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    assert_eq!(
        manifest
            .get("profile")
            .and_then(|profile| profile.get("release"))
            .and_then(|release| release.get("panic"))
            .and_then(toml::Value::as_str),
        Some("abort")
    );
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
        "new_with_s1_k4_resident_windows",
        "M1_R33_AUTHENTICATED_RESIDENT_WINDOWS_PER_INSTANCE_V1",
        "execute_m1_authenticated_resident_first_window_v1",
        "execute_m1_authenticated_resident_next_window_v1",
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
fn authenticated_resident_path_is_exact_bounded_and_phase_ordered() {
    let engine = ENGINE_AUTHENTICATED_RESIDENT_SOURCE
        .split_once("#[cfg(test)]")
        .map_or(ENGINE_AUTHENTICATED_RESIDENT_SOURCE, |(source, _)| source);
    for required in [
        "M1_MAX_AUTHENTICATED_SPECULATIVE_WINDOWS_V1",
        "reserve_completed_window_replacement_with_storage",
        "schedule_m1_authenticated_speculative_new_window_resident_v1",
        "submit_m1_authenticated_speculative_new_window_v1",
        "registry.record_new_window_publication",
        "M1AuthenticatedSpeculativeNewWindowMemberDispositionV1",
        "M1SpeculativeGenerationLoopV1::new_with_storage",
        "prepare_m1_authenticated_speculative_rollover_retained_v1",
        "submit_m1_authenticated_speculative_rollover_v1",
        "execute_m1_authenticated_resident_same_shape_round_core_v1",
        "execute_same_shape_round_with_join_and_deadline",
        "clock_start.elapsed_ns()",
        "cancel_and_close",
        "quarantine_m1_queue_rearm_failure",
    ] {
        assert!(
            engine.contains(required),
            "resident engine path is missing {required}"
        );
    }
    let (first, next) = engine
        .split_once("pub fn execute_m1_authenticated_resident_first_window_v1")
        .unwrap()
        .1
        .split_once("pub fn execute_m1_authenticated_resident_next_window_v1")
        .unwrap();
    for (name, body) in [("first", first), ("next", next)] {
        assert!(
            body.contains("execute_m1_authenticated_resident_same_shape_round_v1("),
            "resident {name}-window loop bypasses the shared physical settlement core"
        );
    }
    assert!(
        next.find("submit_m1_authenticated_speculative_new_window_v1")
            .unwrap()
            < next.find("registry.record_new_window_publication").unwrap()
    );
    for forbidden in [
        "TcpListener",
        "axum::",
        "hyper::",
        "M1EngineeringAggregateArtifactV1",
        "bind_engineering_structural_m1_physical_runner_v1",
        "std::time::Instant",
    ] {
        assert!(
            !engine.contains(forbidden),
            "resident engine path contains {forbidden}"
        );
    }
    for required in [
        "new_with_s1_k4_resident_windows",
        "exact_resident_roster",
        "pending.front().is_some_and(|bound| bound.matches(window))",
        "resident_report(window, tokens.len(), timing)",
        "session.completed_windows() != expected_completed",
        "session.close()",
    ] {
        assert!(
            R33_PRODUCTION_BACKEND_SOURCE.contains(required),
            "resident backend wiring is missing {required}"
        );
    }
    for entry in [
        "match execute_m1_authenticated_resident_first_window_v1(",
        "match execute_m1_authenticated_resident_next_window_v1(",
    ] {
        let success = R33_PRODUCTION_BACKEND_SOURCE
            .split_once(entry)
            .unwrap()
            .1
            .split_once("Ok(success) =>")
            .unwrap()
            .1;
        let report = success
            .find("resident_report(window, tokens.len(), timing)")
            .unwrap();
        let deadline = success.find("deadline.expired()").unwrap();
        let close = success.find("session.close()").unwrap();
        assert!(
            report < deadline && deadline < close,
            "resident backend must reject an expired start deadline and close custody before reporting"
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
        "reserve_s1_k4_rollover_output",
        "const EXPECTED_PREFILL_QUEUE_DATA_ALLOCATIONS: usize = 11;",
        "GFX942_MAX_FIXED_DISPATCH_DATA_V1",
        ".retained_allocation_count()",
        "prepublication_allocation_count != EXPECTED_PREFILL_QUEUE_DATA_ALLOCATIONS",
        "prepublication_allocation_count > GFX942_MAX_FIXED_DISPATCH_DATA_V1",
        "FixedDispatchDataRosterMismatch",
        "bind_m1_authenticated_speculative_rollover_intent_v1",
        "prepare_first_step",
        "into_m1_capture_quarantine",
    ] {
        assert!(
            ENGINE_AUTHENTICATED_PREFILL_BOOTSTRAP_SOURCE.contains(required),
            "authenticated prefill bootstrap is missing {required}"
        );
    }
    assert!(
        !ENGINE_AUTHENTICATED_PREFILL_BOOTSTRAP_SOURCE
            .contains("reserve_finite_speculative_rollover_outputs"),
        "fixed S1/K4 bootstrap must not reserve every speculative output shape"
    );
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

#[test]
fn generic_serving_provider_binds_one_exact_finite_successor_output() {
    let production = ENGINE_SERVING_PHYSICAL_INPUT_PROVIDER_SOURCE
        .split_once("#[cfg(test)]")
        .map_or(
            ENGINE_SERVING_PHYSICAL_INPUT_PROVIDER_SOURCE,
            |(source, _)| source,
        );
    for required in [
        "new_with_finite_speculative_successor",
        "exact_finite_speculative_successor",
        "reserve_finite_speculative_rollover_output(successor)",
        "retained_allocation_count()",
        "GFX942_MAX_FIXED_DISPATCH_DATA_V1",
        "actual != expected || actual > GFX942_MAX_FIXED_DISPATCH_DATA_V1",
        "FiniteSpeculativeSuccessorUnbound",
        "FiniteSpeculativeSuccessorBindingMismatch",
        "UnexpectedFiniteSpeculativeSuccessorBinding",
        "FixedDispatchDataRosterMismatch",
    ] {
        assert!(
            production.contains(required),
            "generic serving provider is missing {required}"
        );
    }
    assert!(
        !production.contains("reserve_finite_speculative_rollover_outputs"),
        "generic serving provider must not reserve every finite output shape"
    );
    let prepare = production
        .split_once("    fn prepare_first_publication(")
        .and_then(|(_, tail)| {
            tail.split_once("    fn prepare_same_shape_rearm(")
                .map(|(body, _)| body)
        })
        .expect("generic first-publication provider remains a bounded source block");
    let successor_preflight = prepare
        .find("first_physical_preflight(front, batch)")
        .expect("exact successor preflight remains present");
    let dequeue = prepare
        .find("self.pending.pop_front()")
        .expect("first-publication dequeue remains present");
    let allocation = prepare
        .find("runner.allocate_scheduled_workspaces")
        .expect("first-publication allocation remains present");
    assert!(successor_preflight < dequeue);
    assert!(dequeue < allocation);
}
