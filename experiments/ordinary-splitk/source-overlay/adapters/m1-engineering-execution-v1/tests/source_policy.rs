const MANIFEST: &str = include_str!("../Cargo.toml");

#[test]
fn splitk_down_live_is_a_separate_v19_ordinary_same_image_selector() {
    let manifest = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    let mut selected_count = 0;
    for binary in manifest["bin"].as_array().unwrap() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(binary["path"].as_str().unwrap());
        let source = std::fs::read_to_string(path).unwrap();
        let selected = binary["name"].as_str() == Some("ferric-qwen3-splitk-down-r1-live");
        assert_eq!(source.contains("mod splitk_down_live_contract;"), selected);
        assert_eq!(
            source.contains("wave_target_v17_runner::run_splitk_down("),
            selected
        );
        if selected {
            selected_count += 1;
            assert_eq!(
                binary["required-features"].as_array().unwrap(),
                &[toml::Value::String("c1-ordered64".into())]
            );
        }
    }
    assert_eq!(selected_count, 1);
    assert!(!ENGINE_MANIFEST.contains("ferric-qwen3-splitk-down-r1-live"));
    let parser = include_str!("../src/bin/splitk_down_live_contract.rs");
    assert!(parser.contains("ordered64_kv_copy_live_contract::Options::parse"));
    assert!(parser.contains("\"baseline\" => false"));
    assert!(parser.contains("\"splitk8-down-mfma-r1\" => true"));
    let profile = include_str!("../src/bin/splitk_down_live_profile.rs");
    assert!(profile.contains("!copy.enabled || copy.prefill32_pages.is_some()"));
    let runner = include_str!("../src/bin/wave_target_v17_runner.rs")
        .split("#[cfg(test)]")
        .next()
        .unwrap()
        .split_whitespace()
        .collect::<String>()
        .replace(",)", ")");
    for marker in [
        "ExecutionMode::SplitKDown(_)=>(false,None)",
        "splitk_down.map(SplitKDown::open).transpose()?",
        "EngineeringTpBatchExecutionV2::new_wide32_with_ordered64_splitk_down_r1(",
        "driver.configure_ordered64_splitk_down_r1(&down.image,selection.enabled)?;",
        "run_with_timing(options,&muttiming,variant,None,false,Some(copy),ExecutionMode::SplitKDown(selection))",
        "setup[\"splitk_down\"]=observed.clone();",
        "closed[\"splitk_down\"]=observed.clone();",
    ] {
        assert!(runner.contains(marker), "{marker}");
    }
}

#[test]
fn packed_down_live_is_explicit_decode_only_and_preserves_ordinary_baseline() {
    let manifest = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    assert!(manifest["features"].get("default").is_none());
    assert!(!ENGINE_MANIFEST.contains("ferric-qwen3-packed-down-r1-live"));
    let mut selected_count = 0;
    for binary in manifest["bin"].as_array().unwrap() {
        let source = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(binary["path"].as_str().unwrap()),
        )
        .unwrap();
        let selected = binary["name"].as_str() == Some("ferric-qwen3-packed-down-r1-live");
        for marker in [
            "mod packed_down_live_contract;",
            "wave_target_v17_runner::run_packed_down(",
        ] {
            assert_eq!(source.contains(marker), selected, "{marker}");
        }
        if selected {
            selected_count += 1;
            assert_eq!(
                binary["required-features"].as_array().unwrap(),
                &[toml::Value::String("c1-ordered64".into())]
            );
        }
    }
    assert_eq!(selected_count, 1);
    let contract = include_str!("../src/bin/packed_down_live_contract.rs");
    let contract = contract.split_once("#[cfg(test)]").unwrap().0;
    for marker in [
        "const FLAGS: [&str; 7]",
        "enabled: match get(FLAGS[6])?",
        "\"baseline\" => false",
        "\"packed-down-u32-r1\" => true",
        "prefill_kv_copy_v28_live_contract::Options::parse(forwarded.into_iter())?",
    ] {
        assert!(contract.contains(marker), "{marker}");
    }
    for old in [
        include_str!("../src/bin/prefill_kv_copy_v28_live_contract.rs"),
        include_str!("../src/bin/ordered64_kv_copy_live_contract.rs"),
        include_str!("../src/bin/packed_gate_up_live_contract.rs"),
    ] {
        assert!(!old.contains("--packed-down-"));
    }
    let profile = include_str!("../src/bin/packed_down_live_profile.rs");
    let profile = profile.split_once("#[cfg(test)]").unwrap().0;
    let route = include_str!("../src/tp_execution/batched/packed_down_r1.rs");
    let route = route.split_once("#[cfg(test)]").unwrap().0;
    for source in [profile, route] {
        let compact = source
            .split_whitespace()
            .collect::<String>()
            .replace(",)", ")");
        assert!(compact.contains(concat!(
            "cfg!(all(feature=\"c1-ordered64\",not(feature=\"model-timestamps\"),",
            "not(feature=\"c1-token-program\")))",
        )));
    }
    let compact = profile.split_whitespace().collect::<String>();
    for marker in [
        "prefill16-decode-ordered64-packed-down-r1-live-v1",
        "gemv:Some((_,false))",
        "\"loaded_image_count\":9",
        "\"selected_phase\":\"decode\"",
        "\"prefill_unchanged\":true",
        "\"same_image_set_in_both_arms\":true",
        "\"native_qualified\":false",
        "\"performance_qualified\":false",
        "\"serving_qualified\":false",
    ] {
        assert!(compact.contains(marker), "{marker}");
    }
    let runner = include_str!("../src/bin/wave_target_v17_runner.rs");
    let production = runner.split_once("#[cfg(test)]").unwrap().0;
    let setup = &production[production.find("fn run_with_timing(").unwrap()..];
    let setup = setup.split_whitespace().collect::<String>();
    let ordered = [
        "ifletSome(selection)=packed_down{selection.validate(options,variant)?;",
        "letpacked_down_artifact=packed_down.map(PackedDown::open).transpose()?;",
        "EngineeringQwenModelV1::open",
        "Worker::spawn_with_timing",
        concat!(
            "ifletSome(artifact)=&packed_down_artifact{",
            "worker.load_additional_artifact(artifact.image.artifact())?;}",
        ),
        "EngineeringTpBatchExecutionV2::new_wide32_with_ordered64_packed_down_r1(",
        "driver.configure_ordered64_packed_down_r1(&packed.image,selection.enabled)?;",
    ]
    .map(|marker| setup.find(marker).expect(marker));
    assert!(ordered.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(setup[ordered[0]..ordered[1]].contains("ifcomposed_copy.is_some()"));
    assert!(setup.contains("ExecutionMode::PackedDown(_)=>(false,None)"));
    let entry = production
        .split_once("pub(super) fn run_packed_down(")
        .unwrap()
        .1
        .split_once("pub(super) fn validate_packed_host_diagnostic(")
        .unwrap()
        .0;
    let entry = entry
        .split_whitespace()
        .collect::<String>()
        .replace(",)", ")");
    assert!(entry.contains(concat!(
        "run_with_timing(options,&muttiming,variant,None,false,None,",
        "ExecutionMode::PackedDown(selection))",
    )));
    let route = route.split_whitespace().collect::<String>();
    for marker in [
        "selected:None",
        "ifself.packed_down_r1.is_none(){returnOk(());}",
        "self.rows.len()==1&&published==1&&self.rows[0].2==TpBatchRowKindV1::Decode",
        "self.partial_gemv_v28!=Some(false)",
        "self.c1_kv_copy_v19.is_some()",
        "self.admitted_c1_kv_copy_v19.is_some()",
        "self.packed_gate_up_r2.is_some()",
        "self.inner.timing.is_enabled()",
    ] {
        assert!(route.contains(marker), "{marker}");
    }
}

#[test]
fn packed_gate_up_live_is_explicit_preallocation_and_preserves_the_current_kv_profile() {
    let manifest = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    let binary = manifest["bin"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"].as_str() == Some("ferric-qwen3-packed-gate-up-kv-r2-live"))
        .unwrap();
    assert_eq!(
        binary["required-features"].as_array().unwrap(),
        &[toml::Value::String("c1-ordered64".into())]
    );
    assert!(!ENGINE_MANIFEST.contains("ferric-qwen3-packed-gate-up-kv-r2-live"));
    let entry = include_str!("../src/bin/ferric-qwen3-packed-gate-up-kv-r2-live.rs");
    assert!(entry.contains("wave_target_v17_runner::run_packed_gate_up("));
    assert!(!entry.contains("run_variant("));
    for old in [
        include_str!("../src/bin/prefill_kv_copy_v28_live_contract.rs"),
        include_str!("../src/bin/ordered64_kv_copy_live_contract.rs"),
    ] {
        assert!(!old.contains("--packed-gate-up-"));
    }
    let profile = include_str!("../src/bin/packed_gate_up_live_profile.rs");
    let profile = profile.split_once("#[cfg(test)]").unwrap().0;
    for marker in [
        "prefill16-decode-ordered64-kv-packed-gate-up-r2-live-v1",
        "gemv: Some((_, false))",
        "deny_unknown_fields",
        "roster_sha256",
        "EngineeringTpPackedBf16ArtifactR2::open",
        "authenticated_weight_sources",
        "additional_weight_bytes",
        "activation_scratch_bytes",
        "loaded_image_count",
        "extra_packets_per_selected_forward",
        "selected_published_rows",
    ] {
        assert!(profile.contains(marker), "{marker}");
    }
    let runner = include_str!("../src/bin/wave_target_v17_runner.rs");
    let production = runner.split_once("#[cfg(test)]").unwrap().0;
    let setup = &production[production.find("fn run_with_timing(").unwrap()..];
    let ordered = [
        "selection.validate(options, variant)?",
        "PackedGateUp::open",
        "EngineeringQwenModelV1::open",
        "Worker::spawn_with_timing",
        "worker.load_additional_artifact(artifact.image.artifact())",
        "EngineeringTpBatchExecutionV2::new_wide32_with_ordered64_kv_packed_gate_up_r2",
        "driver.configure_ordered64_kv_packed_gate_up_r2",
        "selection.metadata(",
    ]
    .map(|marker| setup.find(marker).unwrap());
    assert!(
        ordered
            .windows(2)
            .all(|positions| positions[0] < positions[1])
    );
    assert!(production.contains("prefill16-decode-ordered64-kv-copy-v1"));
    let compact = production.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(compact.contains("Some(selection), ExecutionMode::Ordinary"));
    assert!(compact.contains("false, None, ExecutionMode::Ordinary"));
    assert!(compact.contains("ExecutionMode::PackedGateUp(selection) => (false, Some(selection))"));
    assert!(compact.contains("ExecutionMode::TokenProgram(_) => (true, None)"));
    assert!(compact.contains("ExecutionMode::TokenProgramCounters(_) => (true, None)"));
    assert!(production.contains("setup[\"packed_gate_up\"] = observed.clone()"));
    assert!(production.contains("closed[\"packed_gate_up\"] = observed.clone()"));
    let route = include_str!("../src/tp_execution/batched/packed_gate_up_r2.rs");
    let admission = &route[route.find("pub(super) fn admit").unwrap()..];
    assert!(
        admission.find("supports_token_program()").unwrap()
            < admission.find("require_loaded_image(").unwrap()
    );
    let selector = &route[route
        .find("fn configure_packed_gate_up_binding_r2")
        .unwrap()..];
    assert!(
        selector.find("supports_token_program()").unwrap()
            < selector.find(".selected = Some(enabled)").unwrap()
    );
}

#[test]
fn ordered64_kv_copy_is_separate_atomic_and_reuses_existing_kernel_routes() {
    let cargo = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    let binary = cargo["bin"]
        .as_array()
        .unwrap()
        .iter()
        .find(|binary| binary["name"].as_str() == Some("ferric-qwen3-ordered64-kv-copy-live"))
        .unwrap();
    assert_eq!(
        binary["required-features"].as_array().unwrap(),
        &[toml::Value::String("c1-ordered64".into())]
    );
    let contract = include_str!("../src/bin/ordered64_kv_copy_live_contract.rs");
    let old = include_str!("../src/bin/prefill_kv_copy_v28_live_contract.rs");
    for flag in ["--ordered64-kv-copy-artifact", "--ordered64-kv-copy-mode"] {
        assert!(contract.contains(flag));
        assert!(!old.contains(flag));
    }
    let route = include_str!("../src/tp_execution/batched/ordered64_kv_copy_v1.rs");
    let admission = route.find("admit(&mut transports, binding)?").unwrap();
    let allocation = route
        .find("Self::new_wide32_with_prefill_decode_gemv_v28(")
        .unwrap();
    assert!(admission < allocation);
    assert!(route[..allocation].contains("let _ = transport.close();"));
    let selector = &route[route
        .find("pub(super) fn configure_ordered64_kv_copy_bindings_v1")
        .unwrap()..];
    let mutation = selector
        .find("self.prefill_kv_copy_v28 = Some(true)")
        .unwrap();
    for marker in [
        "self.validate_copy_storage_v19(copy)?",
        "self.validate_prefill_storage_with_c1_v1",
        "self.validate_split_storage_with_c1_v1",
        "self.validate_partial_gemv_v28(gemv)?",
        "self.validate_c1_packet_packing_idle()?",
        "self.configure_ordered_c1_wave_target_bindings_v17(",
    ] {
        assert!(selector.find(marker).unwrap() < mutation);
    }
    assert!(!selector[mutation..].contains('?'));
    assert!(selector.contains("self.partial_gemv_v28 = Some(false)"));
    assert!(selector.contains("self.c1_kv_copy_v19 = enabled.then_some(copy)"));
    let prefill = include_str!("../src/tp_execution/batched/prefill_kv_copy_v28.rs");
    let split = include_str!("../src/tp_execution/batched/c1_split_attention_v25.rs");
    assert!(
        prefill.contains("self.validate_prefill_storage_with_c1_v1(binding, split_admitted, None)")
    );
    assert!(
        split.contains("self.validate_split_storage_with_c1_v1(image, prefill_admitted, None)")
    );
    let runner = include_str!("../src/bin/wave_target_v17_runner.rs");
    assert!(runner.contains("prefill16-decode-ordered64-kv-copy-v1"));
    let compact = runner.split_whitespace().collect::<String>();
    assert!(compact.contains(
        "(model_timestamp_output.is_some()&&!ordered64_packet_ticks)||ordered64_runtime_counters"
    ));
    assert!(compact.contains("ifordered64_packet_ticks{selection.validate_packet_ticks(options,variant)?;}else{selection.validate(options,variant)?;}"));
    let ordinary_validation = &runner[runner
        .find("pub(super) fn validate(self, options:")
        .unwrap()
        ..runner.find("fn validate_packet_ticks(").unwrap()];
    assert!(ordinary_validation.contains("options.live.runtime.ordered64_packet_ticks"));
    let tick_validation = &runner
        [runner.find("fn validate_packet_ticks(").unwrap()..runner.find("fn annotate(").unwrap()];
    for marker in [
        "validate_ordered64_host_diagnostic",
        "!self.enabled",
        "self.prefill32_pages.is_some()",
    ] {
        assert!(tick_validation.contains(marker));
    }
    for marker in [
        "requested_kv_copy_mode",
        "kv_copy_mode",
        "kv_copy_artifact_path",
        "kv_copy_artifact",
    ] {
        assert!(runner.contains(marker));
    }
    assert!(!ENGINE_MANIFEST.contains("ordered64-kv-copy-live"));
}
const SOURCE: &str = include_str!("../src/lib.rs");

#[test]
fn prefill32_page_copy_is_opt_in_and_uses_selection_aware_preflight() {
    let manifest = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    let binary = manifest["bin"]
        .as_array()
        .unwrap()
        .iter()
        .find(|binary| binary["name"].as_str() == Some("ferric-qwen3-prefill32-pages-live"))
        .unwrap();
    assert_eq!(
        binary["required-features"].as_array().unwrap(),
        &[toml::Value::String("c1-ordered64".into())]
    );
    let contract = include_str!("../src/bin/prefill32_pages_live_contract.rs");
    assert!(contract.contains("required --prefill32-pages-mode"));
    assert!(
        !include_str!("../src/bin/ordered64_kv_copy_live_contract.rs")
            .contains("--prefill32-pages-mode")
    );
    let runner = include_str!("../src/bin/wave_target_v17_runner.rs");
    for marker in [
        "prefill32-pages-decode-ordered64-kv-copy-v1",
        "configure_prefill32_pages_v1",
        "requested_prefill32_pages_mode",
        "composed_copy.map_or(CHUNK, Ordered64KvCopy::prefill_chunk)",
    ] {
        assert!(runner.contains(marker), "{marker}");
    }
    let runtime = include_str!("../src/tp_batch_runtime.rs");
    assert!(runtime.contains("expected_dispatch_counts_for_selection"));
    let library = include_str!("../src/tp_execution/batched/prefill32_pages_v1.rs");
    assert!(library.contains("bind_prefill_page_half_v1"));
    assert!(!library.contains(".submit("));
    assert!(!ENGINE_MANIFEST.contains("prefill32-pages-live"));
}
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
const ENGINE_AUTHENTICATED_PACKET_SOURCE: &str =
    include_str!("../../../crates/ferric-engine/src/physical_fixed_batch.rs");
const ENGINE_SERVING_PHYSICAL_INPUT_PROVIDER_SOURCE: &str =
    include_str!("../../../crates/ferric-engine/src/m1_serving_physical_input_provider.rs");
const ENGINE_QUALIFICATION_CAPTURE_SOURCE: &str =
    include_str!("../../../crates/ferric-engine/src/bin/ferric-m1-qualification-capture.rs");
const CORE_SOURCE: &str =
    include_str!("../../../crates/ferric-engine/src/non_authoritative_program_artifact.rs");
const CAPABILITY_SOURCE: &str =
    include_str!("../../../crates/ferric-non-authoritative-program-source-v1/src/lib.rs");

const FE2O3_REVISION: &str = "5a503c04f5ae107a3b3e951ec970b36c5d6a9a79";

fn explicit_mfma_route_policy(source: &str, cli: &str) -> bool {
    let compact = source
        .split_whitespace()
        .collect::<String>()
        .replace(",)", ")");
    let cli_run = cli
        .split("fn run(arguments:")
        .nth(1)
        .and_then(|tail| tail.split("#[cfg(test)]").next());
    let Some(cli_run) = cli_run else {
        return false;
    };
    for (command, call) in [
        (
            "generate-engineering-mfma-inputs",
            "generate_engineering_r29_inputs::<EngineeringAggregateProgramSourceV1<true>>(&arguments[1..],false)",
        ),
        (
            "validate-engineering-mfma-inputs",
            "generate_engineering_r29_inputs::<EngineeringAggregateProgramSourceV1<true>>(&arguments[1..],true)",
        ),
        (
            "capture-engineering-mfma",
            "run_engineering_r29_capture::<EngineeringAggregateProgramSourceV1<true>>(&arguments[1..])",
        ),
    ] {
        let Some(arm) = cli_run
            .split(&format!("Some(\"{command}\")"))
            .nth(1)
            .and_then(|tail| tail.split("Some(").next())
        else {
            return false;
        };
        let arm = arm
            .split_whitespace()
            .collect::<String>()
            .replace(",>", ">");
        if !arm.contains(call) {
            return false;
        }
    }
    compact.contains("reopen_m1_engineering_aggregate_artifact_with_strategy_v1(root.as_ref(),M1PhysicalProgramStrategyV1::LegacyScalar12)")
        && compact.contains("reopen_m1_engineering_aggregate_artifact_with_strategy_v1(root.as_ref(),M1PhysicalProgramStrategyV1::AttributedMfma13)")
        && compact.contains("M1PhysicalProgramStrategyV1::LegacyScalar12=>{admit_m1_non_authoritative_program_artifact_v1(source_capability)}")
        && compact.contains("M1PhysicalProgramStrategyV1::AttributedMfma13=>{admit_m1_non_authoritative_mfma_program_artifact_v1(source_capability)}")
}

#[test]
fn mfma_capture_has_explicit_recorded_route_without_legacy_or_authority_fallback() {
    assert!(explicit_mfma_route_policy(SOURCE, R29_TECHNICAL_CLI_SOURCE));
    let input_source = include_str!("../../../crates/ferric-engine/src/bin/input_bundle.rs");
    assert!(
        input_source.contains("arguments.insert(0, S::ENGINEERING_CAPTURE_SUBCOMMAND.to_owned());")
    );
    assert!(
        ENGINE_QUALIFICATION_CAPTURE_SOURCE.contains(
            "const ENGINEERING_CAPTURE_SUBCOMMAND: &'static str = \"capture-engineering\";"
        )
    );
    let cli_provider = R29_TECHNICAL_CLI_SOURCE
        .split("    fn reopen(root:")
        .nth(1)
        .unwrap()
        .split("    fn program_catalog_id(")
        .next()
        .unwrap();
    assert!(cli_provider.contains("reopen_m1_engineering_mfma_aggregate_artifact_v1(root)"));
    assert!(cli_provider.contains("reopen_m1_engineering_aggregate_artifact_v1(root)"));
    assert!(!cli_provider.contains("or_else"));
    assert!(!cli_provider.contains("std::env"));
    for (old, replacement) in [
        (
            "EngineeringAggregateProgramSourceV1<true>",
            "EngineeringAggregateProgramSourceV1<false>",
        ),
        (
            "Some(\"capture-engineering-mfma\")",
            "Some(\"capture-unrecorded-mfma\")",
        ),
        (
            "generate_engineering_r29_inputs::<",
            "generate_technical_r29_inputs::<",
        ),
    ] {
        let hostile = R29_TECHNICAL_CLI_SOURCE.replacen(old, replacement, 1);
        assert_ne!(hostile, R29_TECHNICAL_CLI_SOURCE);
        assert!(!explicit_mfma_route_policy(SOURCE, &hostile));
    }
}

#[test]
fn paged_draft_canary_is_separate_and_non_authoritative() {
    let manifest = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    let binary = manifest["bin"]
        .as_array()
        .unwrap()
        .iter()
        .find(|binary| binary["name"].as_str() == Some("ferric-qwen3-draft-paged-canary"))
        .unwrap();
    assert_eq!(
        binary["path"].as_str(),
        Some("src/bin/ferric-qwen3-draft-paged-canary.rs")
    );
    assert_eq!(
        binary["required-features"]
            .as_array()
            .unwrap()
            .iter()
            .map(toml::Value::as_str)
            .collect::<Vec<_>>(),
        vec![Some("tp-batch-engineering")]
    );
    let source = include_str!("../src/bin/ferric-qwen3-draft-paged-canary.rs");
    assert!(source.contains("EngineeringTpArtifactV1::open_draft32"));
    assert!(source.contains("EngineeringTpDraftBatchExecutionV10::new"));
    assert!(source.contains("reference.bind(&identity, &prompt)"));
    assert!(source.contains("engine.execute_selected"));
    assert!(source.contains("pool.commit_batch(&batch, observed.completion)"));
    assert!(source.contains("FerricDraftPagedCanaryClosedV10"));
    for forbidden in [
        "execute_speculative_draft",
        "unsafe",
        "StepPublication",
        "verify_greedy_round",
    ] {
        assert!(!source.contains(forbidden));
    }
    assert!(!ENGINE_MANIFEST.contains("ferric-qwen3-draft-paged-canary"));
    assert!(!ROOT_MANIFEST.contains("ferric-qwen3-draft-paged-canary"));
}

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
            "fe2o3-kfd-current-wire",
            "fe2o3-service-host",
            "ferric-qwen3-draft-batch32-kernels-device-v10",
            "ferric-qwen3-tp-batch-kernels-device-v2",
            "ferric-qwen3-tp-batch32-roster-bridge-v5",
            "ferric-qwen3-tp-c1-kv-copy-kernels-device-v19",
            "ferric-qwen3-tp-fp32-argmax-kernels-device-v11",
            "ferric-qwen3-tp-fp32-head-kernels-device-v7",
            "ferric-qwen3-tp-fp32-head32-kernels-device-v8",
            "ferric-qwen3-tp-kernels-device-v1",
            "ferric-qwen3-tp-large-kv-kernels-device-v9",
            "ferric-qwen3-tp-peer-kernels-device-v4",
            "ferric-qwen3-tp-peer32-kernels-device-v6",
            "ferric-qwen3-tp-perf-kernels-device-v3",
            "ferric-qwen3-tp-wave-query-hoist-kernels-device-v14",
            "ferric-qwen3-tp-wave-rmsnorm-kernels-device-v15"
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
            Some("dep:ferric-qwen3-tp-batch32-roster-bridge-v5"),
            Some("dep:ferric-qwen3-tp-peer32-kernels-device-v6"),
            Some("dep:ferric-qwen3-tp-fp32-head-kernels-device-v7"),
            Some("dep:ferric-qwen3-tp-fp32-head32-kernels-device-v8"),
            Some("dep:ferric-qwen3-tp-large-kv-kernels-device-v9"),
            Some("dep:ferric-qwen3-draft-batch32-kernels-device-v10"),
            Some("dep:ferric-qwen3-tp-fp32-argmax-kernels-device-v11"),
            Some("dep:ferric-qwen3-tp-wave-query-hoist-kernels-device-v14"),
            Some("dep:ferric-qwen3-tp-wave-rmsnorm-kernels-device-v15"),
            Some("dep:ferric-qwen3-tp-c1-kv-copy-kernels-device-v19"),
            Some("dep:ferric-qwen3-tp-c1-split8-attention-kernels-device-v21"),
            Some("dep:ferric-qwen3-tp-gemv-prefetch-kernels-device-v20"),
            Some("dep:ferric-qwen3-tp-prefill-kv-roster-bridge-v27")
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
    let execution = include_str!("../src/tp_execution.rs");
    for module in ["batched", "numerical"] {
        assert!(execution.contains(&format!(
            "#[cfg(feature = \"tp-batch-engineering\")]\npub mod {module};"
        )));
    }
}

#[test]
fn argmax_canary_is_separately_opted_in_without_changing_frozen_clis() {
    let manifest = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    let binary = manifest["bin"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"].as_str() == Some("ferric-qwen3-argmax-canary"))
        .unwrap();
    assert_eq!(
        binary["required-features"].as_array().unwrap(),
        &vec![toml::Value::String("tp-batch-engineering".into())]
    );
    let dependency = &manifest["dependencies"]["ferric-qwen3-tp-fp32-argmax-kernels-device-v11"];
    assert_eq!(dependency["optional"].as_bool(), Some(true));
    assert_eq!(dependency["default-features"].as_bool(), Some(false));
    let canary = include_str!("../src/bin/ferric-qwen3-argmax-canary.rs");
    assert!(canary.contains("argmax_canary_contract::Options::parse"));
    assert!(canary.contains("CanaryProfile::LegacyArgmax"));
    assert!(!canary.contains("attention_argmax_canary_contract"));
    let shared = include_str!("../src/bin/argmax_canary_runtime.rs");
    assert!(shared.contains("new_wide32_with_argmax_v11"));
    assert!(shared.contains("configure_fp32_argmax_v11"));
    for frozen in [
        include_str!("../src/bin/ferric-qwen3-tp-batch-engineering.rs"),
        include_str!("../src/bin/ferric-qwen3-paired-paged-canary.rs"),
    ] {
        assert!(!frozen.contains("--argmax-mode"));
    }
}

#[test]
fn wave_rmsnorm_v15_route_keeps_defaults_and_all_legacy_entrypoints_closed() {
    let manifest = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    let dependency = &manifest["dependencies"]["ferric-qwen3-tp-wave-rmsnorm-kernels-device-v15"];
    assert_eq!(dependency["optional"].as_bool(), Some(true));
    assert_eq!(dependency["default-features"].as_bool(), Some(false));
    assert_eq!(
        dependency["features"].as_array().unwrap(),
        &[toml::Value::String("gfx950".into())]
    );
    assert!(
        !manifest["features"]
            .as_table()
            .unwrap()
            .contains_key("default")
    );
    let artifact = include_str!("../src/tp_artifact/wave_rmsnorm_v15.rs");
    assert!(artifact.contains("compiler_expectation_roster_v15()"));
    assert!(artifact.contains("if !metadata_matches("));
    let batched = include_str!("../src/tp_execution/batched.rs");
    assert!(batched.contains("new_wide32_with_argmax_v11_and_wave_rmsnorm_v15"));
    assert!(batched.contains("configure_ordered_c1_wave_rmsnorm_v15"));
    assert!(batched.contains("pub const fn rmsnorm_mode"));
    assert_eq!(batched.matches("target_norm(").count(), 4);
    assert!(!include_str!("../src/tp_execution.rs").contains("wave_rmsnorm_v15"));
    for binary in manifest["bin"].as_array().unwrap() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(binary["path"].as_str().unwrap());
        let source = std::fs::read_to_string(path).unwrap();
        if binary["name"].as_str() == Some("ferric-qwen3-wave-rmsnorm-v15-canary") {
            assert!(source.contains("wave_rmsnorm_v15_canary_contract::parse"));
            assert!(source.contains("argmax_canary_runtime::execute_wave_rmsnorm_v15"));
            assert_eq!(
                binary["required-features"].as_array().unwrap(),
                &[toml::Value::String("tp-batch-engineering".into())]
            );
        } else {
            assert!(!source.contains("wave_rmsnorm_v15_canary_contract"));
            assert!(!source.contains("execute_wave_rmsnorm_v15"));
        }
        for selector in ["--rmsnorm-mode", "--rmsnorm-artifact"] {
            assert!(!source.contains(selector));
        }
        if binary["name"].as_str() == Some("ferric-qwen3-wave-rmsnorm-v15-live") {
            assert!(source.contains("mod wave_rmsnorm_v15_live_contract;"));
            assert!(source.contains("configure_ordered_c1_wave_rmsnorm_v15"));
            assert!(source.contains("open_wave_rmsnorm_v15"));
        } else if matches!(
            binary["name"].as_str(),
            Some(
                "ferric-qwen3-wave-target-v17-live"
                    | "ferric-qwen3-wave-target-v17-runtime-diagnostic"
                    | "ferric-qwen3-c1-kv-copy-v19-live"
                    | "ferric-qwen3-c1-packed-v22-live"
                    | "ferric-qwen3-c1-split-attention-v25-live"
                    | "ferric-qwen3-prefill16-kv-copy-v28-live"
                    | "ferric-qwen3-prefill16-v28-model-timestamps"
                    | "ferric-qwen3-ordered64-host-diagnostic"
                    | "ferric-qwen3-ordered64-kv-copy-live"
                    | "ferric-qwen3-packed-gate-up-kv-r2-live"
                    | "ferric-qwen3-prefill32-pages-live"
                    | "ferric-qwen3-ordered64-packet-ticks"
                    | "ferric-qwen3-ordered64-baseline-packet-ticks"
                    | "ferric-qwen3-token-program-v1-live"
                    | "ferric-qwen3-token-program-native-v1-live"
                    | "ferric-qwen3-token-program-v1-counters"
                    | "ferric-qwen3-prefill-program-native-v1-live"
                    | "ferric-qwen3-prefill-program-native-v1-counters"
                    | "ferric-qwen3-prefill-width-native-live"
                    | "ferric-qwen3-prefill-width-native-counters"
            )
        ) {
            assert!(source.contains("mod wave_target_v17_live_contract;"));
            assert!(source.contains("mod wave_target_v17_runner;"));
            let runner = include_str!("../src/bin/wave_target_v17_runner.rs");
            assert!(runner.contains("configure_ordered_c1_wave_target_v17"));
            assert!(runner.contains("open_wave_rmsnorm_v15"));
            assert!(!source.contains("wave_rmsnorm_v15_live_contract"));
            assert!(!source.contains("configure_ordered_c1_wave_rmsnorm_v15"));
        } else {
            for selector in [
                "wave_rmsnorm_v15_live_contract",
                "configure_ordered_c1_wave_rmsnorm_v15",
                "open_wave_rmsnorm_v15",
            ] {
                assert!(!source.contains(selector));
            }
        }
    }
}

#[test]
fn wave_rmsnorm_v15_live_preserves_ingress_and_preloads_both_explicit_modes() {
    let manifest = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    let matches = manifest["bin"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|binary| binary["name"].as_str() == Some("ferric-qwen3-wave-rmsnorm-v15-live"))
        .collect::<Vec<_>>();
    assert_eq!(matches.len(), 1);
    assert_eq!(
        matches[0]["path"].as_str(),
        Some("src/bin/ferric-qwen3-wave-rmsnorm-v15-live.rs")
    );
    assert_eq!(
        matches[0]["required-features"].as_array().unwrap(),
        &[toml::Value::String("tp-batch-engineering".into())]
    );
    let source = include_str!("../src/bin/ferric-qwen3-wave-rmsnorm-v15-live.rs");
    let production = &source[..source.find("#[cfg(test)]").unwrap()];
    let ordered = [
        "EngineeringTpArtifactV1::open_wave_rmsnorm_v15",
        "EngineeringQwenModelV1::open",
        "Worker::spawn_with_timing",
        "worker.load_additional_artifact(&head_artifact)",
        "worker.load_additional_artifact(&argmax_artifact)",
        "worker.load_additional_artifact(&rmsnorm_artifact)",
        "new_wide32_with_argmax_v11_and_wave_rmsnorm_v15",
        "driver.configure_projection(",
        "driver.configure_head_precision_v8(true)",
        "match options.rmsnorm_mode",
        "driver.configure_ordered_c1_wave_layers_fp32_argmax_v11",
        "configure_ordered_c1_wave_rmsnorm_v15",
        "EngineeringTpBatchRuntimeV2::new_wide32",
    ]
    .map(|marker| production.find(marker).unwrap());
    assert!(ordered.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(production.contains("const LIVE_PROFILE: &str = \"wave-rmsnorm-v15-live-v1\";"));
    assert_eq!(
        production.matches("\"live_profile\":LIVE_PROFILE").count(),
        3
    );
    for marker in [
        "actual_layer != \"c1-wave\"",
        "actual_attention != \"wave\"",
        "actual_rmsnorm != options.rmsnorm_mode.label()",
        "driver.layer_projection_mode()",
        "driver.attention_mode()",
        "driver.rmsnorm_mode()",
        "\"schema\":\"FerricQwen3TpBatchSetupV2\"",
        "\"schema\":\"FerricQwen3TpBatchClosedV2\"",
        "\"live_protocol\":\"FerricQwen3TpLiveCommandV1/FerricQwen3TpLiveEventV1\"",
        "\"rmsnorm_artifact\":artifact_identity(&rmsnorm_artifact)",
        "\"rmsnorm_artifact_path\":options.rmsnorm_artifact",
        "\"benchmark_admitted\":false",
        "\"serving_admitted\":false",
        "\"prefix_cache\":false",
        "\"projection\":\"mfma\"",
        "let close = worker.close();",
        "let close = driver.close();",
        "let close = runtime.close();",
        "run_and_close(&mut runtime",
        "tp_live_ingress::run(",
        "check_retired(runtime, live.pages)",
    ] {
        assert!(production.contains(marker), "{marker}");
    }
    let run = &production[production.find("fn run(options:").unwrap()..];
    assert!(run.find("options.validate()?").unwrap() < run.find("TimingFile::create(").unwrap());
    let contract = include_str!("../src/bin/wave_rmsnorm_v15_live_contract.rs");
    for marker in [
        "layer_c1_wave_live_contract::Options::parse(forwarded.into_iter())?",
        "layer.layer_projection != LayerProjection::C1Wave",
        "required option --rmsnorm-mode",
        "required option --rmsnorm-artifact",
        "self.live.context != 8192",
        "self.live.pages != 512",
        "self.live.submission != Submission::Ordered",
    ] {
        assert!(contract.contains(marker), "{marker}");
    }
    for old in [
        include_str!("../src/bin/ferric-qwen3-wave-argmax-live.rs"),
        include_str!("../src/bin/wave_argmax_live_contract.rs"),
        include_str!("../src/bin/ferric-qwen3-layer-c1-wave-live.rs"),
        include_str!("../src/bin/layer_c1_wave_live_contract.rs"),
    ] {
        assert!(!old.contains("wave_rmsnorm_v15"));
        assert!(!old.contains("--rmsnorm-mode"));
    }
    assert!(!ENGINE_MANIFEST.contains("ferric-qwen3-wave-rmsnorm-v15-live"));
}

#[test]
fn wave_target_v17_keeps_legacy_binaries_closed_and_preloads_every_comparison_image() {
    let manifest = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    let mut count = 0;
    for binary in manifest["bin"].as_array().unwrap() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(binary["path"].as_str().unwrap());
        let source = std::fs::read_to_string(path).unwrap();
        let selected = matches!(
            binary["name"].as_str(),
            Some(
                "ferric-qwen3-wave-target-v17-live"
                    | "ferric-qwen3-wave-target-v17-runtime-diagnostic"
                    | "ferric-qwen3-c1-kv-copy-v19-live"
                    | "ferric-qwen3-c1-packed-v22-live"
                    | "ferric-qwen3-c1-split-attention-v25-live"
                    | "ferric-qwen3-prefill16-kv-copy-v28-live"
                    | "ferric-qwen3-prefill16-v28-model-timestamps"
                    | "ferric-qwen3-ordered64-host-diagnostic"
                    | "ferric-qwen3-ordered64-kv-copy-live"
                    | "ferric-qwen3-packed-gate-up-kv-r2-live"
                    | "ferric-qwen3-packed-down-r1-live"
                    | "ferric-qwen3-splitk-down-r1-live"
                    | "ferric-qwen3-prefill32-pages-live"
                    | "ferric-qwen3-ordered64-packet-ticks"
                    | "ferric-qwen3-ordered64-baseline-packet-ticks"
                    | "ferric-qwen3-token-program-v1-live"
                    | "ferric-qwen3-token-program-native-v1-live"
                    | "ferric-qwen3-token-program-v1-counters"
                    | "ferric-qwen3-prefill-program-native-v1-live"
                    | "ferric-qwen3-prefill-program-native-v1-counters"
                    | "ferric-qwen3-prefill-width-native-live"
                    | "ferric-qwen3-prefill-width-native-counters"
            )
        );
        for marker in [
            "mod wave_target_v17_live_contract;",
            "mod wave_target_v17_runner;",
        ] {
            assert_eq!(source.contains(marker), selected, "{marker}");
        }
        if selected {
            count += 1;
            if matches!(
                binary["name"].as_str(),
                Some(
                    "ferric-qwen3-ordered64-packet-ticks"
                        | "ferric-qwen3-ordered64-baseline-packet-ticks"
                )
            ) {
                assert_eq!(
                    binary["required-features"].as_array().unwrap(),
                    &[
                        toml::Value::String("c1-ordered64".into()),
                        toml::Value::String("model-timestamps".into()),
                    ]
                );
                continue;
            }
            let feature =
                if binary["name"].as_str() == Some("ferric-qwen3-prefill16-v28-model-timestamps") {
                    "model-timestamps"
                } else if matches!(
                    binary["name"].as_str(),
                    Some(
                        "ferric-qwen3-token-program-v1-live"
                            | "ferric-qwen3-token-program-native-v1-live"
                            | "ferric-qwen3-token-program-v1-counters"
                            | "ferric-qwen3-prefill-program-native-v1-live"
                            | "ferric-qwen3-prefill-program-native-v1-counters"
                            | "ferric-qwen3-prefill-width-native-live"
                            | "ferric-qwen3-prefill-width-native-counters"
                    )
                ) {
                    "c1-token-program"
                } else if matches!(
                    binary["name"].as_str(),
                    Some(
                        "ferric-qwen3-ordered64-host-diagnostic"
                            | "ferric-qwen3-ordered64-kv-copy-live"
                            | "ferric-qwen3-packed-gate-up-kv-r2-live"
                            | "ferric-qwen3-packed-down-r1-live"
                            | "ferric-qwen3-splitk-down-r1-live"
                            | "ferric-qwen3-prefill32-pages-live"
                    )
                ) {
                    "c1-ordered64"
                } else {
                    "tp-batch-engineering"
                };
            assert_eq!(
                binary["required-features"].as_array().unwrap(),
                &[toml::Value::String(feature.into())]
            );
        }
    }
    assert_eq!(count, 22);
    assert!(!ENGINE_MANIFEST.contains("ferric-qwen3-wave-target-v17-live"));
    let wrapper = include_str!("../src/bin/ferric-qwen3-wave-target-v17-live.rs");
    assert!(wrapper.contains("wave_target_v17_live_contract::Options::parse"));
    assert!(wrapper.contains("wave_target_v17_runner::run(&options)"));
    assert!(!wrapper.contains("RuntimeDiagnostic"));
    let diagnostic = include_str!("../src/bin/ferric-qwen3-wave-target-v17-runtime-diagnostic.rs");
    assert!(diagnostic.contains("wave_target_v17_runtime_diagnostic_contract::Options::parse"));
    assert!(diagnostic.contains("wave_target_v17_runner::Variant::RuntimeDiagnostic"));
    let source = include_str!("../src/bin/wave_target_v17_runner.rs");
    let production = &source[..source.find("#[cfg(test)]").unwrap()];
    let ordered = [
        "EngineeringTpArtifactV1::open_query_hoist_v14",
        "EngineeringTpArtifactV1::open_wave_rmsnorm_v15",
        "EngineeringQwenModelV1::open",
        "Worker::spawn_with_timing",
        "worker.load_additional_artifact(artifact)?",
        "new_wide32_with_wave_target_v17",
        "driver.configure_projection(",
        "driver.configure_head_precision_v8(true)",
        "driver.configure_ordered_c1_wave_target_v17",
        "EngineeringTpBatchRuntimeV2::new_wide32",
    ]
    .map(|marker| production.find(marker).unwrap());
    assert!(ordered.windows(2).all(|pair| pair[0] < pair[1]));
    for marker in [
        "attention != options.mode.attention()",
        "rmsnorm != options.mode.rmsnorm()",
        "\"wave_target_mode\":options.mode.label()",
        "\"attention_artifact\":artifact_identity(&attention_artifact)",
        "\"rmsnorm_artifact\":artifact_identity(&rmsnorm_artifact)",
        "\"benchmark_admitted\":false",
        "\"serving_admitted\":false",
        "\"prefix_cache\":false",
        "let close = worker.close();",
        "let close = driver.close();",
        "let close = runtime.close();",
        "run_and_close(&mut runtime",
        "tp_live_ingress::run(",
        "check_retired(runtime, live.pages)",
    ] {
        assert!(production.contains(marker), "{marker}");
    }
    let run = &production[production.find("fn run_variant(options:").unwrap()..];
    assert!(
        run.find("variant.validate(options)?").unwrap() < run.find("TimingFile::create(").unwrap()
    );
    for marker in [
        "options.validate()?",
        "runtime.profile = self == Self::RuntimeDiagnostic",
        "\"wave-target-v17-runtime-diagnostic-v1\"",
        "\"all live requests between snapshots, including any warmup; not a benchmark measurement\"",
        "fn checked_snapshot(",
        "runtime_snapshot_before",
        "runtime_snapshot_after",
    ] {
        assert!(production.contains(marker), "{marker}");
    }
    let contract = include_str!("../src/bin/wave_target_v17_live_contract.rs");
    for marker in [
        "layer.layer_projection != LayerProjection::C1Wave",
        "self.live.context != 8192",
        "self.live.pages != 512",
        "self.live.submission != Submission::Ordered",
    ] {
        assert!(contract.contains(marker), "{marker}");
    }
    let batched = include_str!("../src/tp_execution/batched.rs");
    let constructor = batched
        .split("pub fn new_wide32_with_wave_target_v17(")
        .nth(1)
        .unwrap()
        .split("pub fn new_large_kv32(")
        .next()
        .unwrap();
    for marker in [
        "argmax_v11: Some(argmax_v11)",
        "query_hoist_v14: Some(query_hoist_v14)",
        "wave_rmsnorm_v15: Some(&wave_rmsnorm_v15)",
        "let _ = transport.close();",
        "Self::new_profile(",
    ] {
        assert!(constructor.contains(marker), "{marker}");
    }
    let profile = batched.split("    fn new_profile(").nth(1).unwrap();
    let order = [
        "validate_pool_binding(",
        "validate_argmax_binding_v11(",
        "validate_query_hoist_binding_v14(",
        "validate_wave_rmsnorm_binding_v15(",
        "if let Err(error) = binding",
        "let _ = transport.close();",
        "EngineeringTpExecutionV1::new_with_storage(",
    ]
    .map(|marker| profile.find(marker).unwrap());
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn batch32_roster_bridge_keeps_migrated_sdk_out_of_client_units() {
    let manifest = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    let package = "ferric-qwen3-tp-batch32-kernels-device-v5";
    let bridge_package = "ferric-qwen3-tp-batch32-roster-bridge-v5";
    for section in ["dependencies", "build-dependencies", "dev-dependencies"] {
        assert!(manifest[section].get(package).is_none());
    }
    let dependency = &manifest["dependencies"][bridge_package];
    assert_eq!(
        dependency["path"].as_str(),
        Some("../qwen3-tp-batch32-roster-bridge-v5")
    );
    assert_eq!(dependency["version"].as_str(), Some("=0.1.0"));
    assert_eq!(dependency["optional"].as_bool(), Some(true));
    assert_eq!(dependency["default-features"].as_bool(), Some(false));
    assert!(dependency.get("features").is_none());
    assert_eq!(
        manifest["dependencies"]["fe2o3-host"]["rev"].as_str(),
        Some("5a503c04f5ae107a3b3e951ec970b36c5d6a9a79")
    );
    let device = toml::from_str::<toml::Value>(include_str!(
        "../../../device/qwen3-tp-batch32-kernels-v5/Cargo.toml"
    ))
    .unwrap();
    let sdk = Some("1a5999f6e1c5f2363bc2d525af65e84c46502ce6");
    assert_eq!(device["dependencies"]["fe2o3-device"]["rev"].as_str(), sdk);
    assert_eq!(device["target"][r#"cfg(not(target_arch = "amdgpu"))"#]["dependencies"]["fe2o3-host"]["rev"].as_str(), sdk);
    let bridge = toml::from_str::<toml::Value>(include_str!(
        "../../qwen3-tp-batch32-roster-bridge-v5/Cargo.toml"
    ))
    .unwrap();
    assert_eq!(bridge["package"]["name"].as_str(), Some(bridge_package));
    assert!(bridge["dependencies"].as_table().unwrap().is_empty());
    for forbidden in ["dev-dependencies", "target", "features"] {
        assert!(bridge.get(forbidden).is_none());
    }
    assert_eq!(bridge["build-dependencies"].as_table().unwrap().len(), 1);
    let dependency = &bridge["build-dependencies"][package];
    assert_eq!(
        dependency["path"].as_str(),
        Some("../../device/qwen3-tp-batch32-kernels-v5")
    );
    assert_eq!(dependency["version"].as_str(), Some("=0.1.0"));
    assert_eq!(dependency["default-features"].as_bool(), Some(false));
    assert_eq!(
        dependency["features"].as_array().unwrap(),
        &[
            toml::Value::String("gfx950".into()),
            toml::Value::String("mfma".into())
        ]
    );
    let build = include_str!("../../qwen3-tp-batch32-roster-bridge-v5/build.rs");
    for marker in [
        "compiler_expectation_roster_v5()",
        "entry.logical_name()",
        "entry.export_name()",
        "assert_eq!(names.len(), 15)",
        "baseline_contract::NEW_ROOTS",
        "contract::PERFORMANCE_ROOTS",
        "assert_eq!(exports, expected)",
        "({logical:?}, {export:?})",
    ] {
        assert!(build.contains(marker), "{marker}");
    }
    let bridge_lib = include_str!("../../qwen3-tp-batch32-roster-bridge-v5/src/lib.rs");
    assert!(bridge_lib.contains("include!(concat!(env!(\"OUT_DIR\"), \"/compiler_names.rs\"))"));
    assert!(!bridge_lib.contains("fe2o3"));
    let artifact = include_str!("../src/tp_artifact.rs");
    assert!(artifact.contains("expected: &[(&str, &str)]"));
    assert!(!artifact.contains("compiler_expectation_roster_v5()"));
    assert!(!include_str!("../build.rs").contains("ferric_qwen3_tp_batch32_kernels_device_v5::"));
}

#[test]
fn prefill_copy_v28_is_explicit_build_isolated_and_keeps_old_kernel_profiles() {
    let manifest = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    let package = "ferric-qwen3-tp-prefill-kv-copy-kernels-device-v27";
    let bridge_package = "ferric-qwen3-tp-prefill-kv-roster-bridge-v27";
    assert!(manifest["dependencies"].get(package).is_none());
    assert!(manifest["dependencies"].get(bridge_package).is_none());
    assert!(manifest["build-dependencies"].get(package).is_none());
    let dependency = &manifest["build-dependencies"][bridge_package];
    assert_eq!(
        dependency["path"].as_str(),
        Some("../qwen3-tp-prefill-kv-roster-bridge-v27")
    );
    assert_eq!(dependency["optional"].as_bool(), Some(true));
    assert_eq!(dependency["default-features"].as_bool(), Some(false));
    assert!(dependency.get("features").is_none());
    assert_eq!(manifest["build-dependencies"].as_table().unwrap().len(), 3);
    let device = toml::from_str::<toml::Value>(include_str!(
        "../../../device/qwen3-tp-prefill-kv-copy-kernels-v27/Cargo.toml"
    ))
    .unwrap();
    let sdk = Some("d10f49bfedc26848285d20ec1399c193b3340f47");
    assert_eq!(device["dependencies"]["fe2o3-device"]["rev"].as_str(), sdk);
    assert_eq!(
        device["target"][r#"cfg(not(target_arch = "amdgpu"))"#]["dependencies"]["fe2o3-host"]["rev"].as_str(),
        sdk,
    );
    let build = include_str!("../build.rs");
    assert!(!build.contains("ferric_qwen3_tp_prefill_kv_copy_kernels_device_v27::"));
    assert!(build.contains("ferric_qwen3_tp_prefill_kv_roster_bridge_v27::COMPILER_NAMES"));
    assert!(build.contains("v27_compiler_names.rs"));
    assert!(build.contains("#[cfg(not(feature = \"tp-batch-engineering\"))]"));
    let bridge = toml::from_str::<toml::Value>(include_str!(
        "../../qwen3-tp-prefill-kv-roster-bridge-v27/Cargo.toml"
    ))
    .unwrap();
    assert_eq!(bridge["package"]["name"].as_str(), Some(bridge_package));
    assert!(bridge["dependencies"].as_table().unwrap().is_empty());
    for forbidden in ["dev-dependencies", "target", "features"] {
        assert!(bridge.get(forbidden).is_none());
    }
    assert_eq!(bridge["build-dependencies"].as_table().unwrap().len(), 1);
    let dependency = &bridge["build-dependencies"][package];
    assert_eq!(
        dependency["path"].as_str(),
        Some("../../device/qwen3-tp-prefill-kv-copy-kernels-v27")
    );
    assert_eq!(dependency["version"].as_str(), Some("=0.1.0"));
    assert_eq!(dependency["default-features"].as_bool(), Some(false));
    assert_eq!(
        dependency["features"].as_array().unwrap(),
        &[toml::Value::String("gfx950".into())]
    );
    assert!(dependency.get("optional").is_none());
    let bridge_build = include_str!("../../qwen3-tp-prefill-kv-roster-bridge-v27/build.rs");
    for marker in [
        "compiler_expectation_roster_v27()",
        "entry.logical_name()",
        "entry.export_name()",
        "ROOTS_V27",
        "assert_eq!(names.len(), 1)",
        "({logical:?}, {export:?})",
        "pub const COMPILER_NAMES",
    ] {
        assert!(bridge_build.contains(marker), "{marker}");
    }
    let bridge_lib = include_str!("../../qwen3-tp-prefill-kv-roster-bridge-v27/src/lib.rs");
    assert!(bridge_lib.contains("include!(concat!(env!(\"OUT_DIR\"), \"/compiler_names.rs\"))"));
    assert!(!bridge_lib.contains("fe2o3"));
    let artifact = include_str!("../src/tp_artifact/prefill_kv_copy_v27.rs");
    assert!(artifact.contains("open_profile_names"));
    assert!(artifact.contains("&V27_COMPILER_NAMES"));
    assert!(!artifact.contains("compiler_expectation_roster_v27()"));
    assert!(!artifact.contains("for_marker"));
    let wrapper = include_str!("../src/bin/ferric-qwen3-prefill16-kv-copy-v28-live.rs");
    assert!(wrapper.contains("Variant::PrefillKvCopyV28"));
    assert!(wrapper.contains("prefill_kv_copy_v28_live_contract::Options::parse"));
    let runner = include_str!("../src/bin/wave_target_v17_runner.rs");
    let production = &runner[..runner.find("#[cfg(test)]").unwrap()];
    for marker in [
        "open_prefill_kv_copy_v27",
        "new_wide32_with_prefill_kv_copy_v28",
        "configure_ordered_prefill_kv_copy_v28",
        "driver.prefill_kv_mode()",
        "requested_prefill_kv_mode",
        "prefill_kv_artifact_path",
        "prefill_kv_artifact",
        "prefill16-kv-copy-v28-live-v1",
        "driver.expected_dispatch_counts(0) != [613]",
        "driver.expected_dispatch_counts(1) != [616]",
    ] {
        assert!(production.contains(marker), "{marker}");
    }
    assert!(
        production.find("open_prefill_kv_copy_v27").unwrap()
            < production.find("Worker::spawn_with_timing").unwrap()
    );
    let contract = include_str!("../src/bin/prefill_kv_copy_v28_live_contract.rs");
    for marker in [
        "required --prefill-kv-artifact",
        "required --prefill-kv-mode",
        "self.base.mode != Mode::Combined",
    ] {
        assert!(contract.contains(marker), "{marker}");
    }
    for old in [
        include_str!("../src/bin/wave_target_v17_live_contract.rs"),
        include_str!("../src/bin/c1_kv_copy_v19_live_contract.rs"),
        include_str!("../src/bin/c1_packet_packing_v22_live_contract.rs"),
    ] {
        assert!(!old.contains("--prefill-kv"));
    }
    assert!(!ENGINE_MANIFEST.contains(package));
    assert!(!ENGINE_MANIFEST.contains("prefill16-kv-copy-v28"));
}

#[test]
fn prefill_decode_composition_v28_is_explicit_atomic_and_excludes_diagnostics() {
    let runner = include_str!("../src/bin/wave_target_v17_runner.rs");
    let production = &runner[..runner.find("#[cfg(test)]").unwrap()];
    for marker in [
        "prefill16-decode-composed-v28-live-v1",
        "prefill16-kv-copy-v28-live-v1",
        "new_wide32_with_prefill_decode_v28",
        "configure_ordered_prefill_decode_v28",
        "driver.split_attention_workspace_bytes() != 133_120",
        "driver.split_attention_workspace_bytes() != 0",
    ] {
        assert!(production.contains(marker), "{marker}");
    }
    let diagnostic_gate = production[production
        .find("pub(super) fn run_model_timestamps(")
        .unwrap()..]
        .split_whitespace()
        .collect::<String>();
    assert!(diagnostic_gate.contains("Variant::PrefillKvCopyV28{decode:None,..}"));
    let contract = include_str!("../src/bin/prefill_kv_copy_v28_live_contract.rs");
    for marker in [
        "(None, None, None) => None",
        "(Some(artifact), Some(split), Some(packed))",
        "--split-attention-artifact",
        "--split-attention-mode",
        "--c1-packet-mode",
        "self.base.live.host_timing.is_some()",
    ] {
        assert!(contract.contains(marker), "{marker}");
    }
    let route = include_str!("../src/tp_execution/batched/prefill_kv_copy_v28.rs");
    let constructor = &route[route
        .find("pub fn new_wide32_with_prefill_decode_v28")
        .unwrap()
        ..route
            .find("pub fn configure_ordered_prefill_decode_v28")
            .unwrap()];
    let order = [
        "prefill_kv_copy_binding_v27()",
        "split_attention_binding_v21()",
        "admit(&mut transports, copy)?",
        "super::c1_split_attention_v25::admit(&mut transports, split)?",
        "Self::new_wide32_with_wave_target_v17(",
        "driver.allocate_split_workspace_v25(split)?",
    ]
    .map(|marker| constructor.find(marker).unwrap());
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(constructor.contains("let _ = transport.close();"));
    let selector = &route[route
        .find("pub(super) fn configure_prefill_decode_bindings_v28")
        .unwrap()
        ..route
            .find("pub fn new_wide32_with_prefill_kv_copy_v28")
            .unwrap()];
    let order = [
        "self.validate_prefill_storage_v28(copy, true)?",
        "self.validate_split_storage_v25(split, true)?",
        "self.validate_c1_packet_packing_idle()?",
        "self.configure_ordered_c1_wave_target_bindings_v17(",
        "self.prefill_kv_copy_v28 = Some(prefill_enabled)",
        "self.c1_split_attention_v25 = Some(split_enabled)",
        "self.c1_packet_packing_v22 = Some(packed)",
    ]
    .map(|marker| selector.find(marker).unwrap());
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(!selector[selector.find("self.prefill_kv_copy_v28 =").unwrap()..].contains('?'));
    assert!(route.contains("self.validate_prefill_storage_v28(copy, false)?"));
    let split = include_str!("../src/tp_execution/batched/c1_split_attention_v25.rs");
    assert!(split.contains("self.validate_split_storage_v25(image, false)?"));
    let diagnostic = include_str!("../src/bin/ferric-qwen3-prefill16-v28-model-timestamps.rs");
    for marker in [
        "options.decode.is_some()",
        "decode: None",
        "--split-attention-artifact",
        "--split-attention-mode",
        "--c1-packet-mode",
    ] {
        assert!(diagnostic.contains(marker), "{marker}");
    }
    for old in [
        include_str!("../src/bin/wave_target_v17_live_contract.rs"),
        include_str!("../src/bin/c1_kv_copy_v19_live_contract.rs"),
        include_str!("../src/bin/c1_packet_packing_v22_live_contract.rs"),
        include_str!("../src/bin/c1_split_attention_v25_live_contract.rs"),
    ] {
        assert!(!old.contains("--prefill-kv"));
    }
}

#[test]
fn partial_gemv_v28_is_build_isolated_explicit_and_only_routes_single_row_partials() {
    let manifest = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    let package = "ferric-qwen3-tp-gemv-prefetch-kernels-device-v20";
    assert!(manifest["dependencies"].get(package).is_none());
    let dependency = &manifest["build-dependencies"][package];
    assert_eq!(
        dependency["path"].as_str(),
        Some("../../device/qwen3-tp-gemv-prefetch-kernels-v20")
    );
    assert_eq!(dependency["optional"].as_bool(), Some(true));
    assert_eq!(dependency["default-features"].as_bool(), Some(false));
    assert_eq!(
        dependency["features"].as_array().unwrap(),
        &[toml::Value::String("gfx950".into())]
    );
    let device = toml::from_str::<toml::Value>(include_str!(
        "../../../device/qwen3-tp-gemv-prefetch-kernels-v20/Cargo.toml"
    ))
    .unwrap();
    assert_eq!(
        device["dependencies"]["fe2o3-device"]["rev"].as_str(),
        Some("c4c5cdd0f69f3844386440a5addb4d4c3dce0e4b")
    );
    assert_eq!(
        device["target"]["cfg(not(target_arch = \"amdgpu\"))"]["dependencies"]["fe2o3-host"]["rev"]
            .as_str(),
        Some("c4c5cdd0f69f3844386440a5addb4d4c3dce0e4b")
    );
    let build = include_str!("../build.rs");
    assert!(build.contains("compiler_expectation_roster_v20()"));
    assert!(build.contains("v20_compiler_names.rs"));
    let artifact = include_str!("../src/tp_artifact/gemv_prefetch_v20.rs");
    assert!(artifact.contains("Self::open_profile_names"));
    assert!(!artifact.contains("ferric_qwen3_tp_gemv_prefetch_kernels_device_v20::"));
    let route = include_str!("../src/tp_execution/batched/partial_gemv_v28.rs")
        .split_whitespace()
        .collect::<String>();
    assert!(route.contains("command.arguments.get(3)==Some(&EngineeringTpArgumentV1::U32(1))"));
    assert!(route.contains("command.kernel==\"ferric_qwen3_tp_wave_gemv_partial_f32_v3\""));
    assert!(route.contains("command.kernel=ENGINEERING_TP_GEMV_PREFETCH_EXPORTS_V20[1]"));
    assert!(
        route.find("admit(&muttransports,binding)").unwrap()
            < route
                .find("Self::new_wide32_with_prefill_decode_v28(")
                .unwrap()
    );
    assert!(route.contains("let_=transport.close();"));
    let selection = &route[route
        .find("pub(super)fnconfigure_partial_gemv_bindings_v28(")
        .unwrap()..];
    assert!(
        selection
            .find("self.validate_partial_gemv_v28(binding)?")
            .unwrap()
            < selection
                .find("self.configure_prefill_decode_bindings_v28(")
                .unwrap()
    );
    assert!(
        selection
            .find("self.configure_prefill_decode_bindings_v28(")
            .unwrap()
            < selection
                .find("self.partial_gemv_v28=Some(enabled)")
                .unwrap()
    );
    for forbidden in [
        "output_tokens",
        "prompt_tokens",
        "absolute_position",
        "128",
        "127",
    ] {
        assert!(!route.contains(forbidden));
    }
    let batches = include_str!("../src/tp_execution/batched.rs");
    assert_eq!(
        batches.matches("partial_gemv_v28::select_partial").count(),
        2
    );
    let contract = include_str!("../src/bin/prefill_kv_copy_v28_live_contract.rs");
    for marker in [
        "--gemv-artifact",
        "--gemv-mode",
        "partial-prefetch4-v20",
        "gemv.is_some() && decode.is_none()",
    ] {
        assert!(contract.contains(marker));
    }
    let runner = include_str!("../src/bin/wave_target_v17_runner.rs");
    let production = &runner[..runner.find("#[cfg(test)]").unwrap()];
    for marker in [
        "open_gemv_prefetch_v20",
        "new_wide32_with_prefill_decode_gemv_v28",
        "configure_ordered_prefill_decode_gemv_v28",
        "prefill16-decode-partial-gemv-v28-live-v1",
        "requested_gemv_mode",
        "gemv_artifact_path",
        "gemv_artifact",
    ] {
        assert!(production.contains(marker));
    }
    assert_eq!(production.matches("variant.annotate_gemv(").count(), 3);
    assert!(
        production
            .find("worker.load_additional_artifact(artifact)?")
            .unwrap()
            < production.find("let mut driver =").unwrap()
    );
    for old in [
        include_str!("../src/bin/wave_target_v17_live_contract.rs"),
        include_str!("../src/bin/c1_kv_copy_v19_live_contract.rs"),
        include_str!("../src/bin/c1_packet_packing_v22_live_contract.rs"),
        include_str!("../src/bin/c1_split_attention_v25_live_contract.rs"),
    ] {
        // Negative parser tests may name rejected flags without enabling them.
        let (production, _) = old
            .split_once("#[cfg(test)]")
            .expect("legacy contract test boundary");
        assert!(!production.contains("--gemv"));
    }
}

#[test]
fn wave_rmsnorm_v15_canary_preloads_both_arms_and_records_actual_mode() {
    let manifest = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    let binaries = manifest["bin"].as_array().unwrap();
    let matches = binaries
        .iter()
        .filter(|binary| binary["name"].as_str() == Some("ferric-qwen3-wave-rmsnorm-v15-canary"))
        .collect::<Vec<_>>();
    assert_eq!(matches.len(), 1);
    assert_eq!(
        matches[0]["path"].as_str(),
        Some("src/bin/ferric-qwen3-wave-rmsnorm-v15-canary.rs")
    );
    let contract = include_str!("../src/bin/wave_rmsnorm_v15_canary_contract.rs");
    assert!(contract.contains("layer_c1_wave_canary_contract::parse(forwarded.into_iter())"));
    assert!(contract.contains("layer != CanaryProfile::LayerC1Wave"));
    assert!(contract.contains("required option --rmsnorm-mode"));
    assert!(contract.contains("required option --rmsnorm-artifact"));
    let shared = include_str!("../src/bin/argmax_canary_runtime.rs");
    let run = &shared[shared.find("fn run(").unwrap()..shared.find("pub fn execute(").unwrap()];
    let ordered = [
        "EngineeringTpArtifactV1::open_wave_rmsnorm_v15",
        "EngineeringQwenModelV1::open",
        "Worker::spawn_with_timing",
        "worker.load_additional_artifact(&argmax_artifact)",
        "if let Some(artifact) = &wave_rmsnorm_artifact",
        "new_wide32_with_argmax_v11_and_wave_rmsnorm_v15",
        "driver.configure_projection(",
        "driver.configure_head_precision_v8(true)",
        "profile == CanaryProfile::WaveRmsNormV15",
        "driver.configure_ordered_c1_wave_rmsnorm_v15",
        "profile == CanaryProfile::WaveRmsNormBaseline",
        "driver.configure_ordered_c1_wave_layers_fp32_argmax_v11",
    ]
    .map(|marker| run.find(marker).unwrap());
    assert!(ordered.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(run.contains("driver.rmsnorm_mode() != mode"));
    assert!(run.contains("setup[\"rmsnorm_artifact\"] = identity(artifact)"));
    assert!(run.contains("annotate_wave_rmsnorm_record(&mut setup, driver.rmsnorm_mode())"));
    assert!(run.contains("annotate_wave_rmsnorm_record(&mut closed, driver.rmsnorm_mode())"));
    let observe = &shared[shared.find("fn observe<").unwrap()..shared.find("fn run(").unwrap()];
    assert_eq!(observe.matches("driver.rmsnorm_mode()").count(), 2);
    assert!(
        observe.contains("generated == reference.source.generated_token_ids[..options.outputs]")
    );
    assert!(observe.contains("utf8 == reference.expected_utf8(options.outputs)?"));
    let execution = &shared[shared.find("fn execute_with_query_hoist_path(").unwrap()
        ..shared.find("#[cfg(test)]").unwrap()];
    assert!(
        execution
            .find("profile.validate_wave_rmsnorm_path(")
            .unwrap()
            < execution.find("TimingFile::create(").unwrap()
    );
}

#[test]
fn query_hoist_v14_route_keeps_defaults_and_all_legacy_entrypoints_closed() {
    let manifest = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    let dependency =
        &manifest["dependencies"]["ferric-qwen3-tp-wave-query-hoist-kernels-device-v14"];
    assert_eq!(dependency["optional"].as_bool(), Some(true));
    assert_eq!(dependency["default-features"].as_bool(), Some(false));
    assert_eq!(
        dependency["features"].as_array().unwrap(),
        &[toml::Value::String("gfx950".into())]
    );
    assert!(
        !manifest["features"]
            .as_table()
            .unwrap()
            .contains_key("default")
    );
    let artifact = include_str!("../src/tp_artifact.rs");
    assert!(artifact.contains("compiler_expectation_roster_v14()"));
    assert!(artifact.contains("query_hoist_metadata_matches_v14"));
    let batched = include_str!("../src/tp_execution/batched.rs");
    assert!(batched.contains("new_wide32_with_argmax_v11_and_query_hoist_v14"));
    assert!(batched.contains("configure_ordered_c1_wave_query_hoist_v14"));
    for binary in manifest["bin"].as_array().unwrap() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(binary["path"].as_str().unwrap());
        let source = std::fs::read_to_string(path).unwrap();
        if binary["name"].as_str() == Some("ferric-qwen3-query-hoist-v14-canary") {
            assert!(source.contains("query_hoist_v14_canary_contract::parse"));
            assert!(source.contains("argmax_canary_runtime::execute_query_hoist_v14"));
            assert_eq!(
                binary["required-features"].as_array().unwrap(),
                &[toml::Value::String("tp-batch-engineering".into())]
            );
        } else {
            assert!(!source.contains("query_hoist_v14_canary_contract"));
            assert!(!source.contains("execute_query_hoist_v14"));
            assert!(!source.contains("--attention-mode"));
            assert!(!source.contains("--query-hoist-artifact"));
        }
        assert!(!source.contains("open_query_hoist_v14"));
        assert_eq!(
            source.contains("mod wave_target_v17_runner;"),
            matches!(
                binary["name"].as_str(),
                Some(
                    "ferric-qwen3-wave-target-v17-live"
                        | "ferric-qwen3-wave-target-v17-runtime-diagnostic"
                        | "ferric-qwen3-c1-kv-copy-v19-live"
                        | "ferric-qwen3-c1-packed-v22-live"
                        | "ferric-qwen3-c1-split-attention-v25-live"
                        | "ferric-qwen3-prefill16-kv-copy-v28-live"
                        | "ferric-qwen3-prefill16-v28-model-timestamps"
                        | "ferric-qwen3-ordered64-host-diagnostic"
                        | "ferric-qwen3-ordered64-kv-copy-live"
                        | "ferric-qwen3-packed-gate-up-kv-r2-live"
                        | "ferric-qwen3-packed-down-r1-live"
                        | "ferric-qwen3-splitk-down-r1-live"
                        | "ferric-qwen3-prefill32-pages-live"
                        | "ferric-qwen3-ordered64-packet-ticks"
                        | "ferric-qwen3-ordered64-baseline-packet-ticks"
                        | "ferric-qwen3-token-program-v1-live"
                        | "ferric-qwen3-token-program-native-v1-live"
                        | "ferric-qwen3-token-program-v1-counters"
                        | "ferric-qwen3-prefill-program-native-v1-live"
                        | "ferric-qwen3-prefill-program-native-v1-counters"
                        | "ferric-qwen3-prefill-width-native-live"
                        | "ferric-qwen3-prefill-width-native-counters"
                )
            )
        );
        assert!(
            include_str!("../src/bin/wave_target_v17_runner.rs").contains("open_query_hoist_v14")
        );
        assert!(!source.contains("configure_ordered_c1_wave_query_hoist_v14"));
        assert!(!source.contains("--attention-kernel"));
    }
}

#[test]
fn split_attention_v25_is_an_explicit_independent_image_with_closed_legacy_routes() {
    let manifest = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    let dependency =
        &manifest["build-dependencies"]["ferric-qwen3-tp-c1-split8-attention-kernels-device-v21"];
    assert_eq!(dependency["optional"].as_bool(), Some(true));
    assert_eq!(dependency["default-features"].as_bool(), Some(false));
    assert_eq!(
        dependency["features"].as_array().unwrap(),
        &[toml::Value::String("gfx950".into())]
    );
    assert!(!ENGINE_MANIFEST.contains("split-attention-v25"));
    for binary in manifest["bin"].as_array().unwrap() {
        let source = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(binary["path"].as_str().unwrap()),
        )
        .unwrap();
        let selected = binary["name"].as_str() == Some("ferric-qwen3-c1-split-attention-v25-live");
        assert_eq!(
            source.contains("mod c1_split_attention_v25_live_contract;"),
            selected
        );
        assert_eq!(source.contains("Variant::SplitAttentionV25"), selected);
        assert!(!source.contains("open_split_attention_v21"));
    }
    let runner = include_str!("../src/bin/wave_target_v17_runner.rs");
    let production = &runner[..runner.find("#[cfg(test)]").unwrap()];
    let ordered = [
        "EngineeringTpArtifactV1::open_split_attention_v21",
        "EngineeringQwenModelV1::open",
        "Worker::spawn_with_timing",
        "worker.load_additional_artifact(artifact)?",
        "new_wide32_with_c1_split_attention_v25",
        "configure_ordered_c1_split_attention_v25",
        "EngineeringTpBatchRuntimeV2::new_wide32",
    ]
    .map(|marker| production.find(marker).unwrap());
    assert!(ordered.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(
        production
            .matches("variant.annotate_split_attention(")
            .count(),
        3
    );
    let route = include_str!("../src/tp_execution/batched/c1_split_attention_v25.rs");
    for marker in [
        "batch.rows().len() != 1",
        "(128..=256).contains(&context)",
        "checked_add(1)",
        "Workspace::allocate",
        "self.c1_packet_packing_v22.is_some()",
        "self.admitted_c1_kv_copy_v19 != c1",
        "self.validate_split_storage_with_c1_v1(image, prefill_admitted, None)",
        "row_profile::bind_mode(false, 32, false, command)",
    ] {
        assert!(route.contains(marker), "{marker}");
    }
    let producer_guard = &route[route.find("let producer_ready =").unwrap()
        ..route.find("let rank = &inner.ranks[0];").unwrap()];
    for marker in [
        "inner.packed_c1.map_or_else(",
        ".is_some_and(|pending| pending.len() == 8)",
        "state.attention_schedule == AttentionProducerSchedule::Split8V21",
        "state.producer_packets == 8",
        "state.collective.expected().layer as usize == layer",
        "state.collective.expected().operation",
        "== super::Qwen3TensorParallelCollectiveV1::AttentionOutputSum",
        "if !producer_ready",
        "|| inner.ordered_batches.is_none()",
        "|| inner.ranks.len() != 1",
        "|| inner.transports.len() != 1",
        "|| inner.sequences.is_some()",
        "return Err(",
        "V25 partial/merge requires the exact ordered attention producer prefix",
    ] {
        assert!(producer_guard.contains(marker), "{marker}");
    }
    let artifact = include_str!("../src/tp_artifact/split_attention_v21.rs");
    for marker in [
        "V21_COMPILER_NAMES",
        "Self::open_profile_names(",
        "metadata_matches",
    ] {
        assert!(artifact.contains(marker), "{marker}");
    }
    assert!(!artifact.contains("compiler_expectation_roster_v21()"));
    assert!(!artifact.contains("ferric_qwen3_tp_c1_split8_attention_kernels_device_v21::"));
    let build = include_str!("../build.rs");
    for marker in [
        "compiler_expectation_roster_v21()",
        "entry.logical_name()",
        "entry.export_name()",
        "v21_compiler_names.rs",
        "#[cfg(feature = \"tp-batch-engineering\")]",
    ] {
        assert!(build.contains(marker), "{marker}");
    }
    assert!(
        !manifest["dependencies"]
            .as_table()
            .unwrap()
            .contains_key("ferric-qwen3-tp-c1-split8-attention-kernels-device-v21")
    );
    let build_dependencies = manifest["build-dependencies"].as_table().unwrap();
    assert_eq!(build_dependencies.len(), 3);
    for forbidden in ["transmute", "unsafe", "open_query_hoist_v14(root)"] {
        assert!(!artifact.contains(forbidden));
    }
    let runtime = include_str!("../src/tp_batch_runtime.rs");
    let compact = runtime.split_whitespace().collect::<String>();
    let step = compact
        .split_once("pubfnstep(")
        .expect("runtime step")
        .1
        .split_once("pubfncancel(")
        .expect("end of runtime step")
        .0;
    let selection = concat!(
        "letoutput_rows=scheduled.rows().iter().enumerate()",
        ".filter(|(_,row)|row.kind!=TpBatchRowKindV1::PrefillIntermediate)",
        ".map(|(index,_)|index).collect::<Vec<_>>();",
    );
    let binding = concat!(
        "letexpected_counts=matchself.gpu.",
        "bind_dispatch_rows(&prepared,scheduled.rows())",
    );
    let preflight = "self.gpu.expected_dispatch_counts_for_selection(&prepared,&output_rows)";
    let begin = "self.pool.begin_submission(&prepared)";
    let execute = "self.gpu.execute_batch(&prepared,&output_rows)";
    let positions = [selection, binding, preflight, begin, execute].map(|marker| {
        assert_eq!(step.matches(marker).count(), 1, "{marker}");
        step.find(marker).expect("unique runtime step marker")
    });
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(step[positions[1]..positions[2]].contains(".and_then(|()|"));
    for abort in [
        &step[positions[2]..positions[3]],
        &step[positions[3]..positions[4]],
    ] {
        assert!(abort.contains("self.gpu.abandon_dispatch_rows();"));
        assert!(abort.contains("self.pool.abort_batch(&prepared)"));
        assert!(abort.contains("self.scheduler.abort(scheduled.id())"));
    }
    assert!(step[positions[4]..].contains(concat!(
        "self.gpu.abandon_dispatch_rows();self.poisoned=true;",
        "let_=self.pool.quarantine_batch(&prepared);",
    )));
    let defaults = compact
        .split_once("pubtraitEngineeringTpBatchRunnerV2{")
        .unwrap()
        .1
        .split_once("fnruntime_diagnostic_snapshot(")
        .unwrap()
        .0
        .replace(",)", ")");
    assert!(defaults.contains(concat!(
        "fnbind_dispatch_rows(&mutself,_batch:&EngineeringTpPreparedBatchV1,",
        "_rows:&[TpBatchRowV1])->TpResult<()>{Ok(())}",
    )));
    assert!(defaults.contains("fnabandon_dispatch_rows(&mutself){}"));
    assert!(!step.contains("expected_dispatch_counts_for_batch("));
}

#[test]
fn query_hoist_v14_canary_preloads_both_arms_before_allocations_and_keeps_exact_reference() {
    let manifest = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    let binaries = manifest["bin"].as_array().unwrap();
    let matches = binaries
        .iter()
        .filter(|binary| binary["name"].as_str() == Some("ferric-qwen3-query-hoist-v14-canary"))
        .collect::<Vec<_>>();
    assert_eq!(matches.len(), 1);
    assert_eq!(
        matches[0]["path"].as_str(),
        Some("src/bin/ferric-qwen3-query-hoist-v14-canary.rs")
    );
    let contract = include_str!("../src/bin/query_hoist_v14_canary_contract.rs");
    assert!(contract.contains("layer_c1_wave_canary_contract::parse(forwarded.into_iter())"));
    assert!(contract.contains("layer != CanaryProfile::LayerC1Wave"));
    assert!(contract.contains("required option --attention-mode"));
    assert!(contract.contains("required option --query-hoist-artifact"));
    let shared = include_str!("../src/bin/argmax_canary_runtime.rs");
    let run = &shared[shared.find("fn run(").unwrap()..shared.find("pub fn execute(").unwrap()];
    let ordered = [
        "EngineeringTpArtifactV1::open_query_hoist_v14",
        "EngineeringQwenModelV1::open",
        "Worker::spawn_with_timing",
        "worker.load_additional_artifact(&argmax_artifact)",
        "worker.load_additional_artifact(artifact)",
        "new_wide32_with_argmax_v11_and_query_hoist_v14",
        "driver.configure_projection(",
        "driver.configure_head_precision_v8(true)",
        "profile == CanaryProfile::QueryHoistV14",
        "driver.configure_ordered_c1_wave_query_hoist_v14",
        "profile == CanaryProfile::QueryHoistResidentWave",
        "driver.configure_ordered_c1_wave_layers_fp32_argmax_v11",
    ]
    .map(|marker| run.find(marker).unwrap());
    assert!(ordered.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(run.contains("driver.attention_mode() != profile.attention()"));
    assert!(run.contains("setup[\"query_hoist_artifact\"] = identity(artifact)"));
    let execution = &shared[shared.find("fn execute_with_query_hoist_path(").unwrap()..];
    assert!(
        execution
            .find("profile.validate_options(&options)")
            .unwrap()
            < execution.find("TimingFile::create(").unwrap()
    );
    assert!(
        execution
            .find("profile.validate_query_hoist_path(artifact)")
            .unwrap()
            < execution.find("TimingFile::create(").unwrap()
    );
    assert!(
        shared.contains("generated == reference.source.generated_token_ids[..options.outputs]")
    );
    assert!(shared.contains("utf8 == reference.expected_utf8(options.outputs)?"));
    assert!(shared.contains("let close = driver.close()"));
    assert!(shared.contains("pool.quarantine_batch(&batch)"));
    for legacy in [
        include_str!("../src/bin/argmax_canary_contract.rs"),
        include_str!("../src/bin/attention_argmax_canary_contract.rs"),
        include_str!("../src/bin/wave_argmax_submission_canary_contract.rs"),
        include_str!("../src/bin/layer_c1_wave_canary_contract.rs"),
    ] {
        assert!(!legacy.contains("--attention-mode"));
        assert!(!legacy.contains("--query-hoist-artifact"));
    }
}

#[test]
fn attention_argmax_canary_is_closed_and_reuses_preallocation_admission() {
    let manifest = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    let binary = manifest["bin"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"].as_str() == Some("ferric-qwen3-attention-argmax-canary"))
        .unwrap();
    assert_eq!(
        binary["required-features"].as_array().unwrap(),
        &vec![toml::Value::String("tp-batch-engineering".into())]
    );
    let entry = include_str!("../src/bin/ferric-qwen3-attention-argmax-canary.rs");
    assert!(entry.contains("attention_argmax_canary_contract::parse"));
    assert!(entry.contains("argmax_canary_runtime::execute"));
    let contract = include_str!("../src/bin/attention_argmax_canary_contract.rs");
    assert!(contract.contains("Options::parse(forwarded.into_iter())"));
    assert!(contract.contains("options.mode != ArgmaxMode::WaveV11"));
    assert!(!include_str!("../src/bin/argmax_canary_contract.rs").contains("--attention"));
    let shared = include_str!("../src/bin/argmax_canary_runtime.rs");
    assert!(shared.contains("configure_wave_attention_fp32_argmax_v11"));
    assert!(
        shared
            .find("worker.load_additional_artifact(&argmax_artifact)")
            .unwrap()
            < shared
                .find("EngineeringTpBatchExecutionV2::new_wide32_with_argmax_v11")
                .unwrap()
    );
    assert!(
        shared
            .find("driver.configure_wave_attention(true)")
            .unwrap()
            < shared
                .find("driver.configure_head_precision_v8(true)")
                .unwrap()
    );
    for frozen in [
        include_str!("../src/bin/ferric-qwen3-tp-batch-engineering.rs"),
        include_str!("../src/bin/ferric-qwen3-paired-paged-canary.rs"),
    ] {
        assert!(!frozen.contains("configure_wave_attention_fp32_argmax_v11"));
    }
}

#[test]
fn layer_c1_wave_canary_is_additive_and_validates_before_effects() {
    let manifest = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    let binary = manifest["bin"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"].as_str() == Some("ferric-qwen3-layer-c1-wave-canary"))
        .unwrap();
    assert_eq!(
        binary["required-features"].as_array().unwrap(),
        &vec![toml::Value::String("tp-batch-engineering".into())]
    );
    let entry = include_str!("../src/bin/ferric-qwen3-layer-c1-wave-canary.rs");
    assert!(entry.contains("layer_c1_wave_canary_contract::parse"));
    assert!(entry.contains("argmax_canary_runtime::execute"));
    let contract = include_str!("../src/bin/layer_c1_wave_canary_contract.rs");
    assert!(
        contract.contains("wave_argmax_submission_canary_contract::parse(forwarded.into_iter())")
    );
    assert!(contract.contains("submission != CanaryProfile::SubmissionOrdered"));
    let shared = include_str!("../src/bin/argmax_canary_runtime.rs");
    let execution = &shared[shared.find("pub fn execute(").unwrap()..];
    assert!(
        execution
            .find("profile.validate_options(&options)")
            .unwrap()
            < execution.find("TimingFile::create(").unwrap()
    );
    let run = &shared[shared.find("fn run(").unwrap()..shared.find("pub fn execute(").unwrap()];
    let ordered = [
        "EngineeringTpArtifactV1::open_batch32",
        "Worker::spawn_with_timing",
        "worker.load_additional_artifact(&argmax_artifact)",
        "EngineeringTpBatchExecutionV2::new_wide32_with_argmax_v11",
        "driver.configure_projection(",
        "driver.configure_head_precision_v8(true)",
        "profile == CanaryProfile::LayerC1Wave",
        "driver.configure_ordered_c1_wave_layers_fp32_argmax_v11",
        "driver.configure_ordered_wave_attention_fp32_argmax_v11",
        "profile.annotate_setup(&mut setup)",
    ]
    .map(|marker| run.find(marker).unwrap());
    assert!(ordered.windows(2).all(|pair| pair[0] < pair[1]));
    for frozen in [
        include_str!("../src/bin/argmax_canary_contract.rs"),
        include_str!("../src/bin/attention_argmax_canary_contract.rs"),
        include_str!("../src/bin/wave_argmax_submission_canary_contract.rs"),
        include_str!("../src/bin/ferric-qwen3-argmax-canary.rs"),
        include_str!("../src/bin/ferric-qwen3-attention-argmax-canary.rs"),
        include_str!("../src/bin/ferric-qwen3-wave-argmax-submission-canary.rs"),
        include_str!("../src/bin/ferric-qwen3-wave-argmax-live.rs"),
    ] {
        assert!(!frozen.contains("--layer-projection"));
        assert!(!frozen.contains("LayerC1Wave"));
        assert!(!frozen.contains("configure_ordered_c1_wave_layers"));
    }
}

#[test]
fn wave_argmax_submission_canary_is_separate_and_validates_before_effects() {
    let manifest = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    let binary = manifest["bin"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"].as_str() == Some("ferric-qwen3-wave-argmax-submission-canary"))
        .unwrap();
    assert_eq!(
        binary["required-features"].as_array().unwrap(),
        &vec![toml::Value::String("tp-batch-engineering".into())]
    );
    let entry = include_str!("../src/bin/ferric-qwen3-wave-argmax-submission-canary.rs");
    assert!(entry.contains("wave_argmax_submission_canary_contract::parse"));
    assert!(entry.contains("argmax_canary_runtime::execute"));
    let contract = include_str!("../src/bin/wave_argmax_submission_canary_contract.rs");
    assert!(contract.contains("attention_argmax_canary_contract::parse(forwarded.into_iter())"));
    assert!(contract.contains("attention != CanaryProfile::AttentionWave"));
    let shared = include_str!("../src/bin/argmax_canary_runtime.rs");
    let execution = &shared[shared.find("pub fn execute(").unwrap()..];
    assert!(
        execution
            .find("profile.validate_options(&options)")
            .unwrap()
            < execution.find("TimingFile::create(").unwrap()
    );
    assert!(shared.contains("configure_ordered_wave_attention_fp32_argmax_v11"));
    for frozen in [
        include_str!("../src/bin/argmax_canary_contract.rs"),
        include_str!("../src/bin/attention_argmax_canary_contract.rs"),
        include_str!("../src/bin/ferric-qwen3-argmax-canary.rs"),
        include_str!("../src/bin/ferric-qwen3-attention-argmax-canary.rs"),
        include_str!("../src/bin/ferric-qwen3-tp-batch-engineering.rs"),
        include_str!("../src/bin/ferric-qwen3-paired-paged-canary.rs"),
    ] {
        assert!(!frozen.contains("--submission"));
        assert!(!frozen.contains("SubmissionOrdered"));
    }
}

#[test]
fn wave_argmax_live_is_separate_and_preserves_preallocation_and_terminal_selection() {
    let manifest = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    let binary = manifest["bin"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"].as_str() == Some("ferric-qwen3-wave-argmax-live"))
        .unwrap();
    assert_eq!(
        binary["required-features"].as_array().unwrap(),
        &vec![toml::Value::String("tp-batch-engineering".into())]
    );
    let source = include_str!("../src/bin/ferric-qwen3-wave-argmax-live.rs");
    let production = &source[..source.find("#[cfg(test)]").unwrap()];
    let execute = &production[production.find("fn run(options:").unwrap()..];
    assert!(
        execute.find("options.validate()").unwrap() < execute.find("TimingFile::create(").unwrap()
    );
    let ordered = [
        "EngineeringTpArtifactV1::open_fp32_argmax32_v11",
        "Worker::spawn_with_timing",
        "worker.load_additional_artifact(&head_artifact)",
        "worker.load_additional_artifact(&argmax_artifact)",
        "EngineeringTpBatchExecutionV2::new_wide32_with_argmax_v11",
        "driver.configure_output_head_pruning(true)",
        "driver.configure_reduction(EngineeringTpReductionModeV3::DeviceTp1V3)",
        "driver.configure_projection(",
        "driver.configure_wave_attention(true)",
        "driver.configure_head_precision_v8(true)",
        "match options.submission",
        "EngineeringTpBatchRuntimeV2::new_wide32(",
        "tp_live_ingress::run(",
    ]
    .map(|marker| production.find(marker).unwrap());
    assert!(
        ordered
            .windows(2)
            .all(|positions| positions[0] < positions[1])
    );
    let selection = &production[production.find("match options.submission").unwrap()
        ..production.find("driver.configure_host_timing(").unwrap()];
    let branches = [
        "Submission::Synchronous",
        "driver.configure_wave_attention_fp32_argmax_v11",
        "Submission::Ordered",
        "driver.configure_ordered_wave_attention_fp32_argmax_v11",
    ]
    .map(|marker| selection.find(marker).unwrap());
    assert!(
        branches
            .windows(2)
            .all(|positions| positions[0] < positions[1])
    );
    assert!(!production.contains("configure_ordered_batches("));
    assert!(!production.contains("configure_dispatch_sequences("));
    assert!(!production.contains("new_large_kv32"));
    assert!(production.contains("let result = body(runtime);\n    let close = runtime.close();"));
    assert!(production.contains("check_retired(runtime, options.pages)"));
    assert!(production.contains("FerricQwen3TpBatchSetupV2"));
    assert!(production.contains("FerricQwen3TpBatchClosedV2"));
    let contract = include_str!("../src/bin/wave_argmax_live_contract.rs");
    assert!(
        contract
            .contains("EngineeringTpPagedLimitsV1::new(self.context, 32, self.pages, CACHE_TTL)")
    );
    assert!(contract.contains("self.runtime.ordered_batches != self.submission.ordered()"));
    assert!(
        !include_str!("../src/bin/ferric-qwen3-tp-batch-engineering.rs")
            .contains("wave_argmax_live_contract")
    );
    assert!(!ENGINE_MANIFEST.contains("ferric-qwen3-wave-argmax-live"));
}

#[test]
fn layer_c1_wave_live_is_additive_ordered_and_preserves_pre_effect_admission() {
    let manifest = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    let binary = manifest["bin"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"].as_str() == Some("ferric-qwen3-layer-c1-wave-live"))
        .unwrap();
    assert_eq!(
        binary["required-features"].as_array().unwrap(),
        &vec![toml::Value::String("tp-batch-engineering".into())]
    );
    let source = include_str!("../src/bin/ferric-qwen3-layer-c1-wave-live.rs");
    let production = &source[..source.find("#[cfg(test)]").unwrap()];
    let execute = &production[production.find("fn run(options:").unwrap()..];
    assert!(
        execute.find("options.validate()").unwrap() < execute.find("TimingFile::create(").unwrap()
    );
    let setup = &production[production.find("fn run_with_timing(").unwrap()..];
    let ordered = [
        "options.validate()",
        "EngineeringTpArtifactV1::open_batch32",
        "EngineeringTpArtifactV1::open_fp32_head32",
        "EngineeringTpArtifactV1::open_fp32_argmax32_v11",
        "EngineeringQwenModelV1::open",
        "Worker::spawn_with_timing",
        "worker.load_additional_artifact(&head_artifact)",
        "worker.load_additional_artifact(&argmax_artifact)",
        "EngineeringTpBatchExecutionV2::new_wide32_with_argmax_v11",
        "driver.configure_output_head_pruning(true)",
        "driver.configure_reduction(EngineeringTpReductionModeV3::DeviceTp1V3)",
        "driver.configure_projection(",
        "EngineeringTpProjectionModeV3::Mfma",
        "driver.configure_wave_attention(true)",
        "driver.configure_head_precision_v8(true)",
        "match options.layer_projection",
        "profile_metadata(options, driver.layer_projection_mode())",
        "let layer_projection = driver.layer_projection_mode();",
        "EngineeringTpBatchRuntimeV2::new_wide32(",
        "tp_live_ingress::run(",
    ]
    .map(|marker| setup.find(marker).unwrap());
    assert!(
        ordered
            .windows(2)
            .all(|positions| positions[0] < positions[1])
    );
    let selection = &setup[setup.find("match options.layer_projection").unwrap()
        ..setup.find("driver.configure_host_timing(").unwrap()];
    let branches = [
        "LayerProjection::Mfma",
        "driver.configure_ordered_wave_attention_fp32_argmax_v11",
        "LayerProjection::C1Wave",
        "driver.configure_ordered_c1_wave_layers_fp32_argmax_v11",
    ]
    .map(|marker| selection.find(marker).unwrap());
    assert!(
        branches
            .windows(2)
            .all(|positions| positions[0] < positions[1])
    );
    assert_eq!(selection.matches("driver.configure_").count(), 2);
    for forbidden in [
        "configure_ordered_batches(",
        "configure_dispatch_sequences(",
        "configure_wave_attention_fp32_argmax_v11(",
        "new_large_kv32",
        "EngineeringTpProjectionModeV3::Auto",
    ] {
        assert!(!production.contains(forbidden), "{forbidden}");
    }
    assert!(production.contains("actual_layer != options.layer_projection.label()"));
    assert!(production.contains("\"projection\":\"mfma\""));
    assert!(production.contains("const LIVE_PROFILE: &str = \"layer-c1-wave-live-v1\";"));
    assert_eq!(
        production.matches("\"live_profile\":LIVE_PROFILE").count(),
        3
    );
    assert_eq!(
        production
            .matches("\"layer_projection\":layer_projection")
            .count(),
        2
    );
    assert!(production.contains("\"layer_projection\":actual_layer"));
    assert!(production.contains("let result = body(runtime);\n    let close = runtime.close();"));
    assert!(production.contains("check_retired(runtime, live.pages)"));
    assert!(production.contains("FerricQwen3TpBatchSetupV2"));
    assert!(production.contains("FerricQwen3TpBatchClosedV2"));
    assert!(production.contains("FerricQwen3TpLiveCommandV1/FerricQwen3TpLiveEventV1"));
    let contract = include_str!("../src/bin/layer_c1_wave_live_contract.rs");
    let contract = &contract[..contract.find("#[cfg(test)]").unwrap()];
    assert!(contract.contains("wave_argmax_live_contract::Options::parse(forwarded.into_iter())?"));
    assert!(contract.contains("self.live.validate()?;"));
    assert!(contract.contains("self.live.submission != Submission::Ordered"));
    assert!(contract.contains("!self.live.runtime.ordered_batches"));
    for frozen in [
        include_str!("../src/bin/ferric-qwen3-wave-argmax-live.rs"),
        include_str!("../src/bin/wave_argmax_live_contract.rs"),
        include_str!("../src/bin/ferric-qwen3-tp-batch-engineering.rs"),
    ] {
        assert!(!frozen.contains("--layer-projection"));
        assert!(!frozen.contains("layer_c1_wave_live_contract"));
        assert!(!frozen.contains("configure_ordered_c1_wave_layers"));
    }
    assert!(!ENGINE_MANIFEST.contains("ferric-qwen3-layer-c1-wave-live"));
}

#[test]
fn layer_c1_runtime_diagnostic_is_separate_profiled_and_closes_after_snapshots() {
    let manifest = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    let binary = manifest["bin"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| {
            entry["name"].as_str() == Some("ferric-qwen3-layer-c1-wave-runtime-diagnostic")
        })
        .unwrap();
    assert_eq!(
        binary["path"].as_str(),
        Some("src/bin/ferric-qwen3-layer-c1-wave-runtime-diagnostic.rs")
    );
    assert_eq!(
        binary["required-features"].as_array().unwrap(),
        &vec![toml::Value::String("tp-batch-engineering".into())]
    );
    let source = include_str!("../src/bin/ferric-qwen3-layer-c1-wave-runtime-diagnostic.rs");
    let production = &source[..source.find("#[cfg(test)]").unwrap()];
    let run = &production[production.find("fn run(options:").unwrap()..];
    assert!(run.find("options.validate()").unwrap() < run.find("TimingFile::create(").unwrap());
    let setup = &production[production.find("fn run_with_timing(").unwrap()..];
    let ordered = [
        "options.validate()",
        "let runtime_options = options.worker_runtime()?;",
        "EngineeringTpArtifactV1::open_batch32",
        "EngineeringTpArtifactV1::open_fp32_head32",
        "EngineeringTpArtifactV1::open_fp32_argmax32_v11",
        "EngineeringQwenModelV1::open",
        "Worker::spawn_with_timing",
        "worker.load_additional_artifact(&head_artifact)",
        "worker.load_additional_artifact(&argmax_artifact)",
        "EngineeringTpBatchExecutionV2::new_wide32_with_argmax_v11",
        "driver.configure_output_head_pruning(true)",
        "driver.configure_reduction(EngineeringTpReductionModeV3::DeviceTp1V3)",
        "driver.configure_projection(",
        "EngineeringTpProjectionModeV3::Mfma",
        "driver.configure_wave_attention(true)",
        "driver.configure_head_precision_v8(true)",
        "match options.base.layer_projection",
        "profile_metadata(options, driver.layer_projection_mode())",
        "EngineeringTpBatchRuntimeV2::new_wide32(",
        "run_diagnostic_and_close(",
        "emit(&setup)",
        "tp_live_ingress::run(",
        "check_retired(runtime, live.pages)",
    ]
    .map(|marker| setup.find(marker).unwrap());
    assert!(
        ordered
            .windows(2)
            .all(|positions| positions[0] < positions[1])
    );
    let selection = &setup[setup.find("match options.base.layer_projection").unwrap()
        ..setup.find("driver.configure_host_timing(").unwrap()];
    let selectors = [
        "LayerProjection::Mfma",
        "driver.configure_ordered_wave_attention_fp32_argmax_v11",
        "LayerProjection::C1Wave",
        "driver.configure_ordered_c1_wave_layers_fp32_argmax_v11",
    ]
    .map(|marker| selection.find(marker).unwrap());
    assert!(
        selectors
            .windows(2)
            .all(|positions| positions[0] < positions[1])
    );
    assert_eq!(selection.matches("driver.configure_").count(), 2);
    let lifecycle = &production[production.find("fn run_diagnostic_and_close<").unwrap()
        ..production.find("fn check_retired<").unwrap()];
    let phases = [
        "runtime_snapshot_before",
        "body(runtime)?",
        "runtime_snapshot_after",
        "runtime.close()",
    ]
    .map(|marker| lifecycle.find(marker).unwrap());
    assert!(
        phases
            .windows(2)
            .all(|positions| positions[0] < positions[1])
    );
    assert_eq!(
        lifecycle
            .matches("runtime.runtime_diagnostic_snapshot()?")
            .count(),
        2
    );
    assert!(lifecycle.contains("let result = (||"));
    assert!(lifecycle.contains("})();"));
    for marker in [
        "FerricQwen3TpBatchSetupV2",
        "FerricQwen3TpBatchClosedV2",
        "FerricQwen3TpLiveCommandV1/FerricQwen3TpLiveEventV1",
        "FerricLayerC1WaveRuntimeDiagnosticV1",
        "layer-c1-wave-runtime-diagnostic-v1",
        "one earlier snapshot command",
        "1..16 dispatches",
        "overlapping worker host-wall counters",
        "not GPU timestamps",
        "\"benchmark_qualified\":false",
        "\"serving_qualified\":false",
        "\"projection\":\"mfma\"",
        "std::io::stderr().lock()",
    ] {
        assert!(production.contains(marker), "{marker}");
    }
    for forbidden in [
        "configure_ordered_batches(",
        "configure_dispatch_sequences(",
        "new_large_kv32",
        "EngineeringTpProjectionModeV3::Auto",
        "\"performance_qualified\":true",
        "\"benchmark_qualified\":true",
        "\"serving_qualified\":true",
    ] {
        assert!(!production.contains(forbidden), "{forbidden}");
    }
    let contract = include_str!("../src/bin/layer_c1_wave_runtime_diagnostic_contract.rs");
    let contract = &contract[..contract.find("#[cfg(test)]").unwrap()];
    assert!(
        contract.contains("layer_c1_wave_live_contract::Options::parse(forwarded.into_iter())?")
    );
    assert!(contract.contains("self.base.validate()?;"));
    assert!(contract.contains("!self.profiling_requested"));
    assert!(contract.contains("self.base.live.host_timing.is_none()"));
    assert!(contract.contains("self.base.live.context != 8192"));
    assert!(contract.contains("self.base.live.pages != 512"));
    let derive = &contract[contract.find("pub fn worker_runtime(").unwrap()..];
    assert_eq!(derive.matches("runtime.profile = true;").count(), 1);
    assert!(derive.contains("let mut runtime = self.base.live.runtime;"));
    for frozen in [
        include_str!("../src/bin/ferric-qwen3-layer-c1-wave-live.rs"),
        include_str!("../src/bin/layer_c1_wave_live_contract.rs"),
        include_str!("../src/bin/wave_argmax_live_contract.rs"),
        include_str!("../src/bin/argmax_canary_runtime.rs"),
        include_str!("../src/bin/tp_worker.rs"),
    ] {
        assert!(!frozen.contains("layer_c1_wave_runtime_diagnostic_contract"));
        assert!(!frozen.contains("layer-c1-wave-runtime-diagnostic-v1"));
    }
    assert!(!ENGINE_MANIFEST.contains("ferric-qwen3-layer-c1-wave-runtime-diagnostic"));
}

#[test]
fn adapter_legacy_runtime_and_observation_schema_remain_pinned() {
    assert_eq!(MANIFEST.matches(FE2O3_REVISION).count(), 6);
    assert!(SOURCE.contains(FE2O3_REVISION));
    let manifest = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    assert_eq!(
        manifest["dependencies"]["fe2o3-kfd"]["rev"].as_str(),
        Some(FE2O3_REVISION)
    );
}

#[test]
fn ordered64_uses_the_current_optional_wire_without_a_default_or_peer_repin() {
    let manifest = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    let dependencies = manifest["dependencies"].as_table().unwrap();
    let dependency = &dependencies["fe2o3-kfd-current-wire"];
    assert_eq!(dependency["package"].as_str(), Some("fe2o3-kfd"));
    assert_eq!(
        dependency["rev"].as_str(),
        Some("55c1a9b6da5e835e4cec9e1d69609ce884c1a5c9")
    );
    assert_eq!(dependency["version"].as_str(), Some("=0.1.0"));
    assert_eq!(
        dependency["git"].as_str(),
        Some("https://github.com/harsh-nod/fe2o3.git")
    );
    assert_eq!(dependency["optional"].as_bool(), Some(true));
    assert_eq!(dependency["default-features"].as_bool(), Some(false));
    assert_eq!(
        dependency["features"].as_array().unwrap(),
        &vec![toml::Value::String("engineering-gfx950".into())]
    );
    assert_eq!(
        manifest["features"]["c1-ordered64"].as_array().unwrap(),
        &vec![
            toml::Value::String("tp-batch-engineering".into()),
            toml::Value::String("dep:fe2o3-kfd-current-wire".into())
        ]
    );
    for retired in [
        "fe2o3-kfd-ordered64",
        "fe2o3-kfd-profiled",
        "fe2o3-kfd-token-program",
    ] {
        assert!(!dependencies.contains_key(retired));
    }
    for feature in ["tp-engineering", "tp-batch-engineering", "model-timestamps"] {
        assert!(
            !manifest["features"][feature]
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| entry
                    .as_str()
                    .is_some_and(|name| name.contains("ordered64")))
        );
    }
    assert!(!ENGINE_MANIFEST.contains("c1-ordered64"));
    let peers = include_str!("../src/bin/tp_peer_worker.rs");
    assert!(peers.contains("use fe2o3_kfd::engineering_wire"));
    assert!(!peers.contains("fe2o3_kfd_current_wire"));
    assert!(peers.contains("if options.ordered_batches || options.ordered64"));
}

#[test]
fn model_timestamps_use_only_explicit_latest_wire_feature_and_dedicated_binary() {
    let manifest = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    let dependency = &manifest["dependencies"]["fe2o3-kfd-current-wire"];
    assert_eq!(dependency["package"].as_str(), Some("fe2o3-kfd"));
    assert_eq!(
        dependency["rev"].as_str(),
        Some("55c1a9b6da5e835e4cec9e1d69609ce884c1a5c9")
    );
    assert_eq!(dependency["optional"].as_bool(), Some(true));
    assert_eq!(dependency["default-features"].as_bool(), Some(false));
    assert_eq!(
        dependency["features"].as_array().unwrap(),
        &vec![toml::Value::String("engineering-gfx950".into())]
    );
    assert_eq!(
        manifest["features"]["model-timestamps"].as_array().unwrap(),
        &vec![
            toml::Value::String("tp-batch-engineering".into()),
            toml::Value::String("dep:fe2o3-kfd-current-wire".into()),
        ]
    );
    assert!(
        !manifest["features"]["tp-batch-engineering"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| {
                value.as_str().is_some_and(|name| {
                    name.contains("profiled") || name.contains("model-timestamps")
                })
            })
    );
    let binary = manifest["bin"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"].as_str() == Some("ferric-qwen3-prefill16-v28-model-timestamps"))
        .unwrap();
    assert_eq!(
        binary["required-features"].as_array().unwrap(),
        &vec![toml::Value::String("model-timestamps".into())]
    );
    assert!(!ENGINE_MANIFEST.contains("model-timestamps"));
    let worker = include_str!("../src/bin/tp_worker.rs");
    assert!(
        worker.contains(
            "#[cfg(not(any(feature = \"model-timestamps\", feature = \"c1-ordered64\")))]\nuse fe2o3_kfd::engineering_wire;"
        )
    );
    assert!(worker.contains(
        "#[cfg(any(feature = \"model-timestamps\", feature = \"c1-ordered64\"))]\nuse fe2o3_kfd_current_wire::engineering_wire;"
    ));
    for retired in [
        "fe2o3_kfd_ordered64",
        "fe2o3_kfd_profiled",
        "fe2o3_kfd_token_program",
    ] {
        assert!(!worker.contains(retired));
    }
    let collector = include_str!("../src/model_timestamps.rs");
    assert!(collector.contains("use fe2o3_kfd_current_wire::engineering_wire::"));
    assert!(collector.contains("MAX_GROUP_PACKETS: usize = 16"));
    assert!(
        collector.contains("only original single or selected ordered publications are eligible")
    );
    assert!(collector.contains("raw_device_ticks_frequency_unspecified"));
}

#[test]
fn token_program_is_explicit_unprofiled_and_preserves_ordinary_entries() {
    let manifest = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    let dependency = &manifest["dependencies"]["fe2o3-kfd-current-wire"];
    assert_eq!(dependency["package"].as_str(), Some("fe2o3-kfd"));
    assert_eq!(dependency["version"].as_str(), Some("=0.1.0"));
    assert_eq!(dependency["optional"].as_bool(), Some(true));
    assert_eq!(dependency["default-features"].as_bool(), Some(false));
    assert_eq!(
        dependency["features"].as_array().unwrap(),
        &[toml::Value::String("engineering-gfx950".into())]
    );
    assert_eq!(
        manifest["features"]["c1-token-program"].as_array().unwrap(),
        &[toml::Value::String("c1-ordered64".into())]
    );
    assert!(
        !manifest["features"]
            .as_table()
            .unwrap()
            .contains_key("default")
    );
    for name in [
        "tp-engineering",
        "tp-batch-engineering",
        "c1-ordered64",
        "model-timestamps",
    ] {
        assert!(
            !manifest["features"][name]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value.as_str().unwrap().contains("token-program"))
        );
    }
    let mut selected = 0;
    for binary in manifest["bin"].as_array().unwrap() {
        let source = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(binary["path"].as_str().unwrap()),
        )
        .unwrap();
        let token = binary["name"].as_str() == Some("ferric-qwen3-token-program-v1-live");
        let native = binary["name"].as_str() == Some("ferric-qwen3-token-program-native-v1-live");
        let counters = binary["name"].as_str() == Some("ferric-qwen3-token-program-v1-counters");
        assert_eq!(
            source.contains("wave_target_v17_runner::run_token_program("),
            token
        );
        assert_eq!(
            source.contains("wave_target_v17_runner::run_native_token_program("),
            native
        );
        assert_eq!(
            source.contains("wave_target_v17_runner::run_token_program_counters("),
            counters
        );
        if token || native || counters {
            selected += 1;
            assert_eq!(
                binary["required-features"].as_array().unwrap(),
                &[toml::Value::String("c1-token-program".into())]
            );
            assert!(source.contains("ordered64_kv_copy_live_contract::Options::parse"));
        }
    }
    assert_eq!(selected, 3);
    assert!(!ENGINE_MANIFEST.contains("token-program"));
    let worker = include_str!("../src/bin/tp_worker.rs")
        .split_whitespace()
        .collect::<String>()
        .replace(",)", ")");
    assert!(worker.contains("#[cfg(all(feature=\"c1-token-program\",not(feature=\"model-timestamps\")))]#[path=\"tp_worker_token_program.rs\"]modtoken_program;"));
    assert!(worker.contains("#[cfg(any(feature=\"model-timestamps\",all(feature=\"c1-ordered64\",not(feature=\"c1-token-program\"))))]ifmatches!(header,CommandV1::RegisterTokenProgram{..}|CommandV1::RegisterTokenProgramSlots512V1{..}|CommandV1::ExecuteTokenProgram{..}|CommandV1::ExecuteTokenProgramSlots512V1{..}|CommandV1::ReleaseTokenProgram{..})"));
    assert!(
        worker
            .find("returnself.reject(\"tokencommandsrequiretheexplicitunprofiledtokenmode\")")
            .unwrap()
            < worker
                .find("writer.try_send(Outgoing{header,payload})")
                .unwrap()
    );
    assert!(
        worker
            .contains("Self::spawn_mode(executable,unique_id,artifact,options,timing,rank,false)")
    );
    assert!(worker.contains("self.release_registered_token()?;"));
    let spawn = &worker[worker.find("fnspawn_entry(").unwrap()..];
    assert!(
        spawn
            .find("worker.verify_token_program_backend(backend)?")
            .unwrap()
            < spawn.find("worker.configure_options(options)?").unwrap()
    );
    let token = include_str!("../src/bin/tp_worker_token_program.rs");
    assert!(token.contains("CommandV1::DescribeTokenProgramBackendV1"));
    assert!(token.contains("backend == expected.identity()"));
    assert!(token.contains("fallback is forbidden"));
    assert!(token.contains("CommandV1::TokenProgramSnapshotV1"));
    let runner = include_str!("../src/bin/wave_target_v17_runner.rs")
        .split_whitespace()
        .collect::<String>();
    assert!(runner.contains("!selection.enabled||selection.prefill32_pages.is_some()"));
    assert!(runner.contains("options.live.runtime.ordered64_runtime_counters"));
    let dispatch = include_str!("../src/tp_execution/performance.rs")
        .split_whitespace()
        .collect::<String>()
        .replace(",)", ")");
    assert!(dispatch.contains("self.ordered_batch_width.is_wide()"));
    assert!(dispatch.contains("self.packed_c1.is_some_and(|state|{state.attention_schedule==AttentionProducerSchedule::Split8V21&&state.feed_forward_schedule==FeedForwardProducerSchedule::Baseline})"));
    assert!(dispatch.contains("feed_forward_schedule!=FeedForwardProducerSchedule::Baseline"));
    assert!(dispatch.contains("program.is_some_and(|expected|count!=expected)"));
    assert!(dispatch.contains("elseifself.token_program_selected(){Some(652)}"));
}

#[test]
fn native_prefill_program_is_separate_default_off_and_keeps_phase_validation() {
    let manifest = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    let entries = [
        (
            "ferric-qwen3-prefill-program-native-v1-live",
            include_str!("../src/bin/ferric-qwen3-prefill-program-native-v1-live.rs"),
        ),
        (
            "ferric-qwen3-prefill-program-native-v1-counters",
            include_str!("../src/bin/ferric-qwen3-prefill-program-native-v1-counters.rs"),
        ),
    ];
    for (name, source) in entries {
        let binary = manifest["bin"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["name"].as_str() == Some(name))
            .unwrap();
        assert_eq!(
            binary["required-features"].as_array().unwrap(),
            &[toml::Value::String("c1-token-program".into())]
        );
        assert!(source.contains("ordered64_kv_copy_live_contract::Options::parse"));
        assert!(source.contains("wave_target_v17_runner::run_native_prefill_program("));
        assert!(!ENGINE_MANIFEST.contains(name));
    }
    let worker = include_str!("../src/bin/tp_worker_token_program.rs");
    assert!(worker.contains("prefill_enabled: false"));
    assert!(worker.contains("registered.shape != shape"));
    assert!(worker.contains("registered.immutable != immutable"));
    assert!(worker.contains("self.release_registered_token()?;"));
    let client = include_str!("../src/bin/tp_worker_prefill_program.rs");
    assert!(client.contains("dispatches.len() != 613"));
    assert!(client.contains("result.definition.slots.len() != 216"));
    assert!(client.contains("page_shape.map(|shape| shape.0 + 16) != context"));
    let performance = include_str!("../src/tp_execution/performance.rs");
    assert!(performance.contains("supports_prefill_program()"));
    assert!(performance.contains(".wait_prefill_program(count)"));
}

#[test]
fn native_prefill_width_is_explicit_and_preserves_legacy_default_family() {
    let manifest = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    for (name, source) in [
        (
            "ferric-qwen3-prefill-width-native-live",
            include_str!("../src/bin/ferric-qwen3-prefill-width-native-live.rs"),
        ),
        (
            "ferric-qwen3-prefill-width-native-counters",
            include_str!("../src/bin/ferric-qwen3-prefill-width-native-counters.rs"),
        ),
    ] {
        let entry = manifest["bin"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["name"].as_str() == Some(name))
            .unwrap();
        assert_eq!(
            entry["required-features"].as_array().unwrap(),
            &[toml::Value::String("c1-token-program".into())]
        );
        assert!(source.contains("native_prefill_width_contract::parse_width"));
        assert!(source.contains("run_native_prefill_width_program"));
        assert!(!ENGINE_MANIFEST.contains(name));
    }
    let client = include_str!("../src/bin/tp_worker_token_program.rs");
    assert!(client.contains("shape == Shape::Prefill32"));
    assert!(client.contains("wire::encode_token_program_slots512_v1"));
    assert!(client.contains("wire::encode_token_program_v1"));
    assert!(client.contains("registered.immutable != immutable"));
    let planner = include_str!("../src/bin/tp_worker_prefill32_program.rs");
    assert!(planner.contains("dispatches.len() != 649"));
    assert!(planner.contains("result.definition.slots.len() != 396"));
    assert!(planner.contains("left.0.min(right.0) + 32"));
}

#[test]
fn baseline_packet_ticks_require_an_explicit_marker_without_weakening_v19_admission() {
    let manifest = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    let binary = manifest["bin"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| {
            entry["name"].as_str() == Some("ferric-qwen3-ordered64-baseline-packet-ticks")
        })
        .unwrap();
    assert_eq!(
        binary["required-features"].as_array().unwrap(),
        &[
            toml::Value::String("c1-ordered64".into()),
            toml::Value::String("model-timestamps".into()),
        ]
    );
    let contract = include_str!("../src/bin/ordered64_baseline_packet_ticks_contract.rs");
    assert!(contract.contains("--ordered64-baseline-packet-ticks"));
    for rejected in [
        "--ordered64-kv-copy-artifact",
        "--ordered64-kv-copy-mode",
        "--ordered64-packet-ticks",
    ] {
        assert!(contract.contains(rejected));
    }
    let route = include_str!("../src/tp_execution/batched/ordered64_baseline_packet_ticks_v1.rs");
    for required in [
        "self.admitted_c1_kv_copy_v19.is_some()",
        "self.c1_kv_copy_v19.is_some()",
        "!self.inner.transports[0].model_timestamp_ordered64_enabled()",
        "self.baseline_packet_ticks_v1 = true",
    ] {
        assert!(route.contains(required), "{required}");
    }
    for forbidden in ["allocate_tensor(", "load_additional_artifact(", "dispatch("] {
        assert!(!route.contains(forbidden));
    }
    let library = include_str!("../src/tp_execution/batched.rs");
    assert!(library.contains("if self.baseline_packet_ticks_v1"));
    assert!(library.contains("self.admitted_c1_kv_copy_v19.is_none()"));
    assert!(library.contains("self.c1_kv_copy_v19 != self.admitted_c1_kv_copy_v19"));
    let runner = include_str!("../src/bin/wave_target_v17_runner.rs");
    assert!(runner.contains("(composed_copy.is_none() != baseline_packet_ticks)"));
    assert!(runner.contains("driver.configure_ordered64_baseline_packet_ticks_v1()?"));
    assert!(!ENGINE_MANIFEST.contains("ferric-qwen3-ordered64-baseline-packet-ticks"));
}

#[test]
fn ordered64_packet_ticks_are_separate_bounded_raw_observations_not_a_core_rewrite() {
    let manifest = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    let binary = manifest["bin"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"].as_str() == Some("ferric-qwen3-ordered64-packet-ticks"))
        .unwrap();
    assert_eq!(
        binary["required-features"].as_array().unwrap(),
        &[
            toml::Value::String("c1-ordered64".into()),
            toml::Value::String("model-timestamps".into()),
        ]
    );
    assert!(!ENGINE_MANIFEST.contains("ferric-qwen3-ordered64-packet-ticks"));
    let contract = include_str!("../src/bin/ordered64_packet_ticks_contract.rs");
    for marker in [
        "--ordered64-packet-ticks",
        "--ordered64-runtime-counters",
        "--diagnostic-active-poll-10ms",
        "validate_ordered64_host_diagnostic",
    ] {
        assert!(contract.contains(marker));
    }
    for marker in [
        "--ordered64-kv-copy-mode",
        "--ordered64-kv-copy-artifact",
        "parallel-c1-v19",
        "prefill32_pages: None",
    ] {
        assert!(contract.contains(marker));
    }
    let ordinary_copy = include_str!("../src/bin/ordered64_kv_copy_live_contract.rs");
    assert!(!ordinary_copy.contains("parse_ordered64_packet_ticks"));
    assert!(!ordinary_copy.contains("run_ordered64_packet_ticks"));
    let ordinary_prefill32 = include_str!("../src/bin/prefill32_pages_live_contract.rs");
    assert!(!ordinary_prefill32.contains("parse_ordered64_packet_ticks"));
    let copy_driver =
        include_str!("../src/tp_execution/batched/ordered64_kv_copy_packet_ticks_v1.rs");
    for marker in [
        "model_timestamp_ordered64_enabled",
        "c1_kv_copy_v19::admit",
        "prefill32_pages_v1.is_some()",
        "configure_ordered64_kv_copy_checked_bindings_v1",
    ] {
        assert!(copy_driver.contains(marker));
    }
    let ordinary_driver = include_str!("../src/tp_execution/batched/ordered64_kv_copy_v1.rs");
    assert!(ordinary_driver.contains("cfg!(feature = \"model-timestamps\")"));
    let collector = include_str!("../src/model_timestamps.rs");
    for marker in [
        "MAX_GROUP_PACKETS: usize = 16",
        "MAX_ORDERED64_GROUP_PACKETS: usize = 64",
        "MAX_CAPTURE_PACKETS: usize = 100_000",
        "MAX_CAPTURE_BATCHES: usize = 256",
        "MAX_CAPTURE_BYTES: usize = 12 * 1024 * 1024",
        "FerricOrdered64RawPacketIntervalsV1",
        "raw_device_ticks_frequency_unspecified",
        "end_tick_minus_start_tick_not_shader_only_not_wall_time",
    ] {
        assert!(collector.contains(marker));
    }
    let runner = include_str!("../src/bin/wave_target_v17_runner.rs");
    for marker in [
        "prefill16-decode-ordered64-kv-copy-packet-ticks-v1",
        "FerricOrdered64PacketTicksClosedV1",
        "raw packet-processing",
        "OFlags::NOFOLLOW",
        "OFlags::EXCL",
        "collector.finish(counts[0])",
    ] {
        assert!(runner.contains(marker));
    }
    let worker = include_str!("../src/bin/tp_worker_model_timestamps.rs");
    assert!(worker.contains("Collector::new_ordered64"));
    assert!(worker.contains("ResponseV1::DispatchOrderedBatch64Completed"));
    assert!(
        worker.find("collector.complete_group").unwrap()
            < worker
                .find("ResponseV1::DispatchOrderedBatch64Completed")
                .unwrap()
    );
}

#[test]
fn peer_wire_helpers_are_instantiated_in_each_transport_namespace() {
    let worker = include_str!("../src/bin/tp_worker.rs");
    let peer = include_str!("../src/bin/tp_peer_worker.rs");
    let peer_wire = include_str!("../../tp-peer-engineering-worker-v4/src/wire.rs");
    let helper = include_str!("../src/bin/tp_worker_dispatch.rs");
    for source in [worker, peer] {
        assert_eq!(
            source
                .matches("include!(\"tp_worker_dispatch.rs\");")
                .count(),
            1
        );
        assert!(!source.contains("fn metadata_matches("));
        assert!(!source.contains("fn pack_dispatch("));
    }
    let peer = peer.split_whitespace().collect::<String>();
    assert!(peer.contains("usesuper::tp_worker::{LoadedKernel,RuntimeOptions};"));
    assert!(peer.contains("usefe2o3_kfd::engineering_wire::{selfaswire,"));
    assert!(!peer.contains("fe2o3_kfd_current_wire"));
    assert!(peer_wire.contains("use fe2o3_kfd::engineering_wire::"));
    assert!(!peer_wire.contains("fe2o3_kfd_current_wire"));
    for name in [
        "metadata_matches",
        "pack_dispatch",
        "pack_dispatch_into",
        "pack_dispatch_arguments",
    ] {
        assert_eq!(helper.matches(&format!("fn {name}(")).count(), 1);
    }
    assert_eq!(helper.matches("fn append_kernarg_payload<T>(").count(), 1);
    assert!(!helper.contains("fe2o3_kfd"));
    assert!(!helper.contains("serde_json"));
    assert!(helper.contains("timeout_ms: DISPATCH_TIMEOUT_MS"));
    assert!(helper.contains("const DISPATCH_TIMEOUT_MS: u32 = 60_000;"));
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

    for (source, artifact_operation) in [
        (CLI_SOURCE, OPERATIONS[0]),
        (
            SPECULATIVE_CLI_SOURCE,
            "let artifact = match program_strategy {",
        ),
    ] {
        assert!(source.contains("mod startup_diagnostics;"));
        assert!(source.contains("EngineeringStartupDiagnosticsV1::from_process_environment()"));
        assert_eq!(source.matches("diagnostics.completed(").count(), 6);

        let completion_offsets = COMPLETIONS.map(|completion| {
            source
                .find(completion)
                .unwrap_or_else(|| panic!("engineering smoke is missing {completion}"))
        });
        assert!(completion_offsets.windows(2).all(|pair| pair[0] < pair[1]));

        let mut operations = OPERATIONS;
        operations[0] = artifact_operation;
        for (operation, completion) in operations[..5].iter().zip(&COMPLETIONS[..5]) {
            assert_eq!(source.matches(operation).count(), 1);
            assert!(
                source.find(operation).unwrap() < source.find(completion).unwrap(),
                "completion {completion} must follow successful operation {operation}"
            );
        }
    }

    let speculative_admission = SPECULATIVE_CLI_SOURCE
        .split_once("let artifact = match program_strategy {")
        .unwrap()
        .1
        .split_once(COMPLETIONS[0])
        .unwrap()
        .0;
    for required in [
        "M1PhysicalProgramStrategyV1::LegacyScalar12 =>",
        "reopen_m1_engineering_aggregate_artifact_v1(observation_root)",
        "M1PhysicalProgramStrategyV1::AttributedMfma13 =>",
        "reopen_m1_engineering_mfma_aggregate_artifact_v1(observation_root)",
        ".map_err(|error| format!(\"cannot admit engineering aggregate: {error}\"))?;",
    ] {
        assert_eq!(speculative_admission.matches(required).count(), 1);
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
        let (terminal, success) = success
            .split_once("let (session, tokens, timing) = success.into_parts();")
            .unwrap();
        assert!(terminal.contains("M1AuthenticatedResidentWindowOutcomeV1::Terminal(terminal)"));
        let terminal_close = terminal.find("terminal.into_close()").unwrap();
        let terminal_fault = terminal.find("return Err(fault(").unwrap();
        assert!(terminal_close < terminal_fault);
        assert!(terminal.contains("ResidentClosed"));
        assert!(!terminal.contains("return Ok(report)"));
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
fn authenticated_resident_packet_storage_is_selected_and_consumed_without_fallback() {
    let production = ENGINE_AUTHENTICATED_PACKET_SOURCE
        .split_once("#[cfg(test)]")
        .map_or(ENGINE_AUTHENTICATED_PACKET_SOURCE, |(source, _)| source);
    for required in [
        "new_for_speculative_shape",
        "speculative_shape: M1PhysicalFixedBatchShapeV1",
        "fn has_case(",
        "fn ready_for_recipe(",
        "!storage.ready_for_recipe(shape, recipe.rows())",
        "storage.speculative_k4.take()",
        "storage.speculative_k8.take()",
        "storage.speculative_k16.take()",
        "storage.speculative_lowering.take()",
    ] {
        assert!(
            production.contains(required),
            "resident packet storage lost {required}"
        );
    }
    let lowering = production
        .split_once("fn build_m1_authenticated_rollover_packet_batch_core_v1(")
        .unwrap()
        .1
        .split_once("fn validate_authenticated_queue_packet_inputs(")
        .unwrap()
        .0;
    assert!(
        lowering
            .find("!storage.ready_for_recipe(shape, recipe.rows())")
            .unwrap()
            < lowering.find("match shape {").unwrap(),
        "resident storage must be checked before a typed slot is consumed",
    );
    for (start, end) in [
        (
            "M1PhysicalFixedBatchShapeV1::SpeculativeK4 =>",
            "M1PhysicalFixedBatchShapeV1::SpeculativeK8 =>",
        ),
        (
            "M1PhysicalFixedBatchShapeV1::SpeculativeK8 =>",
            "M1PhysicalFixedBatchShapeV1::SpeculativeK16 =>",
        ),
        (
            "M1PhysicalFixedBatchShapeV1::SpeculativeK16 =>",
            "\n    }\n}",
        ),
    ] {
        let body = lowering
            .split_once(start)
            .unwrap()
            .1
            .split_once(end)
            .unwrap()
            .0;
        assert!(body.contains("lower_authenticated_queue_packet_case_core("));
        assert!(body.contains("storage.speculative_lowering.take()"));
        assert!(!body.contains("lower_authenticated_queue_packet_case("));
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
        "SpeculativeS1K8C8192",
        "SpeculativeS1K16C8192",
        "new_with_speculative_successor",
        "admitted_s1_t128_speculative_successor_v1",
        "admit_m1_production_rollover_transition_v1(prefill, successor)",
        "reserve_finite_speculative_rollover_output(successor.target())",
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
        "a selected singleton bootstrap must not reserve every speculative output shape"
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

#[test]
fn ordered64_host_diagnostic_is_a_separate_closed_executable() {
    let manifest = toml::from_str::<toml::Value>(MANIFEST).unwrap();
    let binary = manifest["bin"]
        .as_array()
        .unwrap()
        .iter()
        .find(|binary| binary["name"].as_str() == Some("ferric-qwen3-ordered64-host-diagnostic"))
        .unwrap();
    assert_eq!(
        binary["required-features"].as_array().unwrap(),
        &[toml::Value::String("c1-ordered64".into())]
    );
    assert!(!ENGINE_MANIFEST.contains("ferric-qwen3-ordered64-host-diagnostic"));
    let entry = include_str!("../src/bin/ferric-qwen3-ordered64-host-diagnostic.rs");
    assert!(entry.contains("mod ordered64_host_diagnostic_contract;"));
    assert!(entry.contains("wave_target_v17_runner::run_ordered64_host_diagnostic_with_wait("));
    assert!(entry.contains("options.runtime_counters"));
    assert!(entry.contains("options.active_poll_10ms"));
    assert!(!entry.contains("run_variant("));
    let contract = include_str!("../src/bin/ordered64_host_diagnostic_contract.rs");
    let (production, _) = contract.split_once("#[cfg(test)]").unwrap();
    assert!(production.contains("--ordered64-host-timing"));
    assert!(production.contains("let mut active_poll_10ms = false;"));
    assert!(production.contains("--ordered64-active-poll-10ms"));
    assert!(production.contains("duplicate --ordered64-active-poll-10ms"));
    assert!(production.contains("self.active_poll_10ms && !self.runtime_counters"));
    assert!(production.contains("prefill_kv_copy_v28_live_contract::Options::parse"));
    assert!(production.contains("validate_ordered64_host_diagnostic"));
    for legacy in [
        include_str!("../src/bin/prefill_kv_copy_v28_live_contract.rs"),
        include_str!("../src/bin/ferric-qwen3-prefill16-kv-copy-v28-live.rs"),
    ] {
        assert!(!legacy.contains("--ordered64-host-timing"));
        assert!(!legacy.contains("--ordered64-active-poll-10ms"));
    }
    let legacy = include_str!("../src/bin/prefill_kv_copy_v28_live_contract.rs");
    let (production, _) = legacy.split_once("#[cfg(test)]").unwrap();
    assert!(production.contains("self.base.live.host_timing.is_some()"));
    let runner = include_str!("../src/bin/wave_target_v17_runner.rs");
    let (production, _) = runner.split_once("#[cfg(test)]").unwrap();
    assert!(production.contains("ORDERED64_HOST_MAX_MODEL_BATCHES: u64 = 256"));
    assert!(production.contains("prefill16-decode-ordered64-host-diagnostic-v1"));
    assert!(production.contains("prefill16-decode-ordered64-v29-live-v1"));
    assert!(production.contains("timing.timing.is_ordered64_diagnostic()"));
    assert!(production.contains("not GPU time"));
    let wrapper = production
        .split_once("pub(super) fn run_ordered64_host_diagnostic(")
        .unwrap()
        .1
        .split_once("pub(super) fn run_ordered64_host_diagnostic_with_wait(")
        .unwrap()
        .0;
    let wrapper = wrapper
        .split_whitespace()
        .collect::<String>()
        .replace(",)", ")");
    assert!(wrapper.contains(
        "run_ordered64_host_diagnostic_with_wait(options,variant,output,runtime_counters,false)"
    ));
    let timing = include_str!("../src/host_timing.rs");
    assert!(timing.contains("FerricOrdered64HostTimingV1"));
    assert!(timing.contains("worker_reported_elapsed"));
    let worker = include_str!("../src/bin/tp_worker.rs");
    assert!(worker.contains("ordered_packing_span"));
    assert!(worker.contains("ordered_worker_elapsed"));
}

#[test]
fn ordered64_runtime_counters_require_a_distinct_diagnostic_selector_and_profile() {
    let contract = include_str!("../src/bin/ordered64_host_diagnostic_contract.rs");
    let (production, _) = contract.split_once("#[cfg(test)]").unwrap();
    assert!(production.contains("--ordered64-runtime-counters"));
    assert!(production.contains("duplicate --ordered64-runtime-counters"));
    for legacy in [
        include_str!("../src/bin/prefill_kv_copy_v28_live_contract.rs"),
        include_str!("../src/bin/wave_target_v17_live_contract.rs"),
        include_str!("../src/bin/wave_argmax_live_contract.rs"),
        include_str!("../src/bin/ferric-qwen3-prefill16-kv-copy-v28-live.rs"),
    ] {
        assert!(!legacy.contains("--ordered64-runtime-counters"));
        assert!(!legacy.contains("--ordered64-active-poll-10ms"));
    }
    let runner = include_str!("../src/bin/wave_target_v17_runner.rs");
    let (production, _) = runner.split_once("#[cfg(test)]").unwrap();
    for marker in [
        "prefill16-decode-ordered64-runtime-counters-v1",
        "prefill16-decode-ordered64-active-poll-10ms-v1",
        "prefill16-decode-ordered64-host-diagnostic-v1",
        "prefill16-decode-ordered64-v29-live-v1",
        "FerricOrdered64RuntimeCountersV1",
        "ordered64_runtime_counters && !ordered64_host_diagnostic",
        "emit_diagnostic(&ordered64_counter_record_with_wait(value, active_poll))",
        "active_poll && !runtime_counters",
        "annotate_ordered64_active_poll(&mut value)",
        "Worker::spawn_active_poll_with_timing",
        "isolated IOCTL time",
    ] {
        assert!(production.contains(marker), "{marker}");
    }
    let worker = include_str!("../src/bin/tp_worker.rs");
    assert!(worker.contains("self.profile && !self.ordered64_runtime_counters"));
    assert!(
        worker.contains("self.ordered64_runtime_counters && (!self.ordered64 || !self.profile)")
    );
    let (worker, _) = worker.split_once("#[cfg(test)]").unwrap();
    let command = worker
        .split_once("impl WorkerEntry {")
        .unwrap()
        .1
        .split_once("struct Outgoing {")
        .unwrap()
        .0;
    let creation = command.find("Command::new(executable)").unwrap();
    assert!(command.find("options.validate_ordered64()?").unwrap() < creation);
    assert!(command.find("!timing.is_ordered64_diagnostic()").unwrap() < creation);
    assert!(
        command
            .find("active polling requires the separate profiled ordered64 TP1 entry")
            .unwrap()
            < creation
    );
    let command = command
        .split_whitespace()
        .collect::<String>()
        .replace(",)", ")");
    assert!(
        command.contains("matches!(self,Self::ActivePoll10ms)&&!timing.is_ordered64_diagnostic()")
    );
    assert!(
        command.contains("!cfg!(all(feature=\"c1-ordered64\",not(feature=\"model-timestamps\")))")
    );
    for required in [
        "!options.ordered64",
        "!options.profile",
        "!options.ordered64_runtime_counters",
        "options.ordered64_packet_ticks",
        "rank!=0",
    ] {
        assert!(command.contains(required), "{required}");
    }
    assert!(command.contains("Self::Ordinary=>{}"));
    assert!(
        command.contains("Self::ActivePoll10ms=>{command.arg(\"--diagnostic-active-poll-10ms\");}")
    );
    let worker = worker
        .split_whitespace()
        .collect::<String>()
        .replace(",)", ")");
    assert!(worker.contains("entry.command(executable,unique_id,options,&timing,rank)?"));
    assert!(
        worker
            .contains("Self::spawn_mode(executable,unique_id,artifact,options,timing,rank,false)")
    );
}
