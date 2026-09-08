#![forbid(unsafe_code)]

//! Explicit non-authoritative target-smoke execution for one fe2o3 engineering observation.

mod smoke_bootstrap;
mod startup_diagnostics;

use fe2o3_kfd::{DeviceSelector, OpenedKfd};
use ferric_build::{SpecialTokenDecodePolicy, TokenizerExecutionLimits};
use ferric_engine::{M1TargetSmokeExecutionV1, execute_m1_target_smoke_v1};
use ferric_m1_engineering_execution_v1::{
    bind_engineering_structural_m1_physical_runner_v1, reopen_m1_engineering_aggregate_artifact_v1,
};
use ferric_spec::{Identity, M1_QUALIFICATION_TOKENS_PER_LANE};
use serde_json::{Value, json};
use startup_diagnostics::{EngineeringStartupDiagnosticsV1, EngineeringStartupPhaseV1};
use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

type SmokeResult<T> = Result<T, String>;

const STATUS: &str = "engineering-hardware-observation-non-evidence-non-qualification";
const NONCLAIM: &str = "Raw-prompt target-only execution of a structurally admitted fe2o3 engineering aggregate whose authority is none. Reported choices and timing are raw device observations, not verified model answers or benchmark evidence. Timing starts after artifact, model-memory, and tokenizer setup and is not comparable to R33 serving, vLLM, or SGLang measurements. This output authenticates no compiler process or Worker V3 publication, selects no current protected publication, establishes no numerical or hardware correctness, is not a qualification result, and closes no M1 requirement.";
const EXTERNAL_FILE_SCHEMA: &str = "ferric.m1-engineering-target-smoke-observation.v2";
const DERIVED_ENGINEERING_SCHEMA: &str = "ferric.m1-engineering-target-smoke-observation.v3";
const DERIVED_ENGINEERING_IDENTITY_INPUT: &str = "@derive-engineering-identities-v1";
const DERIVED_ENGINEERING_IDENTITY_MODE: &str = "derived-engineering-observation-model-plan-v1";
const EXTERNAL_FILE_REPORT_FIELDS: [&str; 29] = [
    "artifact_authority",
    "authority",
    "benchmark_comparable",
    "canonical_descriptor_sha256",
    "compiler_handoff_sha256",
    "compiler_origin_authenticated",
    "current_publication_selected",
    "generated_runner_declaration_sha256",
    "generated_token_count",
    "generated_token_ids",
    "hardware_completion_observed",
    "hsaco_sha256",
    "model_bundle_sha256",
    "nonclaim",
    "observation_manifest_sha256",
    "program_catalog_sha256",
    "prompt_priming_choice_token_ids",
    "prompt_token_count",
    "prompt_token_ids",
    "schema",
    "status",
    "target",
    "target_choice_observation_count",
    "termination",
    "text",
    "text_bytes_hex",
    "text_utf8_policy",
    "timing",
    "worker_v3_authenticated",
];
const TARGET: &str = "gfx942:xnack-";
const TIMING_BOUNDARY: &str = "target-smoke-controller-entry-to-completed-device-teardown";
const TIMING_CLOCK: &str = "monotonic-raw-nanoseconds";
const TIMING_SCOPE: &str = "single-process-single-request-target-smoke";

#[derive(Clone, Copy)]
struct EngineeringObservationFacts {
    manifest: Identity,
    hsaco: Identity,
    compiler_handoff: Identity,
    canonical_descriptor: Identity,
    program_catalog: Identity,
}

#[derive(Clone, Copy)]
struct EngineeringTimingFacts {
    duration: u64,
    first_token_offset: u64,
    terminal_offset: u64,
}

enum EngineeringIdentityInputV1 {
    ExternalFile(PathBuf),
    DerivedEngineeringV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EngineeringIdentityReportModeV1 {
    ExternalFile,
    DerivedEngineeringV1,
}

impl EngineeringIdentityInputV1 {
    const fn report_mode(&self) -> EngineeringIdentityReportModeV1 {
        match self {
            Self::ExternalFile(_) => EngineeringIdentityReportModeV1::ExternalFile,
            Self::DerivedEngineeringV1 => EngineeringIdentityReportModeV1::DerivedEngineeringV1,
        }
    }
}

impl EngineeringIdentityReportModeV1 {
    const fn schema(self) -> &'static str {
        match self {
            Self::ExternalFile => EXTERNAL_FILE_SCHEMA,
            Self::DerivedEngineeringV1 => DERIVED_ENGINEERING_SCHEMA,
        }
    }
}

impl EngineeringTimingFacts {
    fn from_execution(execution: &M1TargetSmokeExecutionV1) -> Self {
        let timing = execution.timing();
        Self {
            duration: timing.duration_ns(),
            first_token_offset: timing.first_generated_token_offset_ns(),
            terminal_offset: timing.last_generated_token_offset_ns(),
        }
    }
}

fn main() -> ExitCode {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    match run(&arguments) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let _ = writeln!(std::io::stderr().lock(), "FAIL: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: &[OsString]) -> SmokeResult<()> {
    let diagnostics = EngineeringStartupDiagnosticsV1::from_process_environment();
    let [
        prepacked_root,
        observation_root,
        identity_input,
        gpu_unique_id,
        max_new_tokens,
        prompt,
    ] = arguments
    else {
        return Err("usage: ferric-m1-engineering-target-smoke PREPACKED-SNAPSHOT ENGINEERING-OBSERVATION-DIRECTORY EXTERNAL-CLOSURE-OR-@derive-engineering-identities-v1 GPU-UNIQUE-ID MAX-NEW-TOKENS RAW-PROMPT".to_owned());
    };
    let identity_input = parse_engineering_identity_input(identity_input);
    let gpu_unique_id = gpu_unique_id
        .to_str()
        .ok_or_else(|| "GPU unique ID must be UTF-8 decimal".to_owned())?
        .parse::<u64>()
        .map_err(|_| "GPU unique ID must be a decimal u64".to_owned())?;
    let max_new_tokens = max_new_tokens
        .to_str()
        .ok_or_else(|| "MAX-NEW-TOKENS must be UTF-8 decimal".to_owned())?
        .parse::<usize>()
        .map_err(|_| "MAX-NEW-TOKENS must be a decimal usize".to_owned())?;
    if max_new_tokens == 0 || max_new_tokens > M1_QUALIFICATION_TOKENS_PER_LANE as usize {
        return Err("MAX-NEW-TOKENS must be in 1..=8192".to_owned());
    }
    let prompt = prompt
        .to_str()
        .ok_or_else(|| "RAW-PROMPT must be UTF-8".to_owned())?;

    let artifact = reopen_m1_engineering_aggregate_artifact_v1(Path::new(observation_root))
        .map_err(|error| format!("cannot admit engineering aggregate: {error}"))?;
    diagnostics.completed(EngineeringStartupPhaseV1::ArtifactAdmission);
    let facts = EngineeringObservationFacts {
        manifest: artifact.manifest_id(),
        hsaco: artifact.hsaco_id(),
        compiler_handoff: artifact.compiler_handoff_id(),
        canonical_descriptor: artifact.canonical_descriptor_id(),
        program_catalog: artifact.program_catalog_id(),
    };
    let bootstrap =
        smoke_bootstrap::prepare(Path::new(prepacked_root), &identity_input, prompt, facts)?;
    diagnostics.completed(EngineeringStartupPhaseV1::CpuModelBootstrapPreparation);
    let bound = bootstrap.bind(|publication| {
        bind_engineering_structural_m1_physical_runner_v1(artifact, publication)
            .map_err(|error| format!("cannot bind engineering physical runner: {error:?}"))
    })?;
    diagnostics.completed(EngineeringStartupPhaseV1::RunnerBind);
    let checked = OpenedKfd::open_default()
        .map_err(|error| format!("cannot open KFD: {error}"))?
        .admit_uapi()
        .map_err(|error| format!("cannot admit pinned KFD UAPI: {error}"))?
        .bind_gfx942_xnack_minus(DeviceSelector::UniqueId(gpu_unique_id))
        .map_err(|error| format!("cannot bind selected gfx942:xnack- device: {error}"))?;
    diagnostics.completed(EngineeringStartupPhaseV1::KfdBind);
    let initialized = bound.initialize_memory(checked)?;
    diagnostics.completed(EngineeringStartupPhaseV1::InitializeMemoryAllocationUpload);
    let execution = execute_m1_target_smoke_v1(
        &initialized.runner,
        initialized.memory,
        initialized.prompt_tokens,
        max_new_tokens,
    )?;
    diagnostics.completed(EngineeringStartupPhaseV1::ControllerExecution);
    let text_bytes = initialized
        .tokenizer
        .decode_to_bytes(
            execution.generated_tokens(),
            TokenizerExecutionLimits::m1(),
            SpecialTokenDecodePolicy::Skip,
        )
        .map_err(|error| format!("cannot decode generated token bytes: {error}"))?;
    let text = String::from_utf8_lossy(&text_bytes).into_owned();
    let report = engineering_report(
        &execution,
        facts,
        initialized.runner.declaration_id(),
        initialized.runner.logical_runner().bundle_id(),
        identity_input.report_mode(),
        &text,
        &text_bytes,
    );
    validate_engineering_report(&report)?;
    let mut stdout = std::io::stdout().lock();
    serde_json::to_writer(&mut stdout, &report)
        .map_err(|error| format!("cannot serialize smoke report: {error}"))?;
    stdout
        .write_all(b"\n")
        .map_err(|error| format!("cannot write smoke report: {error}"))?;
    Ok(())
}

fn parse_engineering_identity_input(value: &OsStr) -> EngineeringIdentityInputV1 {
    if value == OsStr::new(DERIVED_ENGINEERING_IDENTITY_INPUT) {
        EngineeringIdentityInputV1::DerivedEngineeringV1
    } else {
        EngineeringIdentityInputV1::ExternalFile(PathBuf::from(value))
    }
}

fn engineering_report(
    execution: &M1TargetSmokeExecutionV1,
    facts: EngineeringObservationFacts,
    runner_declaration: Identity,
    model_bundle: Identity,
    identity_mode: EngineeringIdentityReportModeV1,
    text: &str,
    text_bytes: &[u8],
) -> Value {
    engineering_report_from_parts(
        execution.prompt_tokens(),
        execution.prompt_observations(),
        execution.generated_tokens(),
        execution.termination(),
        EngineeringTimingFacts::from_execution(execution),
        facts,
        runner_declaration,
        model_bundle,
        identity_mode,
        text,
        text_bytes,
    )
}

#[allow(clippy::too_many_arguments)]
fn engineering_report_from_parts(
    prompt_tokens: &[u32],
    prompt_observations: &[u32],
    generated_tokens: &[u32],
    termination: &str,
    timing: EngineeringTimingFacts,
    facts: EngineeringObservationFacts,
    runner_declaration: Identity,
    model_bundle: Identity,
    identity_mode: EngineeringIdentityReportModeV1,
    text: &str,
    text_bytes: &[u8],
) -> Value {
    let target_choice_observation_count = prompt_observations
        .len()
        .saturating_add(generated_tokens.len());
    let r33_tpot_eligible = generated_tokens.len() >= 2
        && timing.first_token_offset > 0
        && timing.terminal_offset > timing.first_token_offset;
    let mut report = json!({
        "artifact_authority": "none",
        "authority": "none",
        "benchmark_comparable": false,
        "canonical_descriptor_sha256": hex_bytes(facts.canonical_descriptor.as_bytes()),
        "compiler_handoff_sha256": hex_bytes(facts.compiler_handoff.as_bytes()),
        "compiler_origin_authenticated": false,
        "current_publication_selected": false,
        "generated_runner_declaration_sha256": hex_bytes(runner_declaration.as_bytes()),
        "generated_token_count": generated_tokens.len(),
        "generated_token_ids": generated_tokens,
        "hardware_completion_observed": true,
        "hsaco_sha256": hex_bytes(facts.hsaco.as_bytes()),
        "model_bundle_sha256": hex_bytes(model_bundle.as_bytes()),
        "nonclaim": NONCLAIM,
        "observation_manifest_sha256": hex_bytes(facts.manifest.as_bytes()),
        "program_catalog_sha256": hex_bytes(facts.program_catalog.as_bytes()),
        "prompt_priming_choice_token_ids": prompt_observations,
        "prompt_token_count": prompt_tokens.len(),
        "prompt_token_ids": prompt_tokens,
        "schema": identity_mode.schema(),
        "status": STATUS,
        "target": TARGET,
        "target_choice_observation_count": target_choice_observation_count,
        "termination": termination,
        "text": text,
        "text_bytes_hex": hex_bytes(text_bytes),
        "text_utf8_policy": "lossy-replacement",
        "timing": {
            "clock": TIMING_CLOCK,
            "duration_boundary": TIMING_BOUNDARY,
            "duration_ns": timing.duration,
            "r33_tpot_eligible": r33_tpot_eligible,
            "request_events": [{
                "arrival_offset_ns": 0,
                "first_token_offset_ns": timing.first_token_offset,
                "input_tokens": prompt_tokens.len(),
                "output_tokens": generated_tokens.len(),
                "request_ordinal": 0,
                "terminal_offset_ns": timing.terminal_offset,
            }],
            "scope": TIMING_SCOPE,
        },
        "worker_v3_authenticated": false,
    });
    if identity_mode == EngineeringIdentityReportModeV1::DerivedEngineeringV1 {
        report["identity_input_mode"] = json!(DERIVED_ENGINEERING_IDENTITY_MODE);
    }
    report
}

fn validate_engineering_report(report: &Value) -> SmokeResult<()> {
    let object = report
        .as_object()
        .ok_or_else(|| "engineering report is not an object".to_owned())?;
    for (field, expected) in [
        ("artifact_authority", json!("none")),
        ("authority", json!("none")),
        ("benchmark_comparable", json!(false)),
        ("compiler_origin_authenticated", json!(false)),
        ("current_publication_selected", json!(false)),
        ("status", json!(STATUS)),
        ("worker_v3_authenticated", json!(false)),
    ] {
        if object.get(field) != Some(&expected) {
            return Err(format!(
                "engineering report {field} attempted an unsupported claim"
            ));
        }
    }
    let mut expected_fields = EXTERNAL_FILE_REPORT_FIELDS
        .into_iter()
        .collect::<BTreeSet<_>>();
    match object.get("schema").and_then(Value::as_str) {
        Some(EXTERNAL_FILE_SCHEMA) => {}
        Some(DERIVED_ENGINEERING_SCHEMA)
            if object.get("identity_input_mode")
                == Some(&json!(DERIVED_ENGINEERING_IDENTITY_MODE)) =>
        {
            expected_fields.insert("identity_input_mode");
        }
        _ => return Err("engineering report identity-input provenance drifted".to_owned()),
    }
    if object.keys().map(String::as_str).collect::<BTreeSet<_>>() != expected_fields {
        return Err("engineering report field roster drifted".to_owned());
    }
    if object.get("nonclaim") != Some(&json!(NONCLAIM)) {
        return Err("engineering report nonclaim drifted".to_owned());
    }
    let timing = object
        .get("timing")
        .and_then(Value::as_object)
        .ok_or_else(|| "engineering report timing is missing".to_owned())?;
    if timing.get("clock") != Some(&json!(TIMING_CLOCK))
        || timing.get("duration_boundary") != Some(&json!(TIMING_BOUNDARY))
        || timing.get("scope") != Some(&json!(TIMING_SCOPE))
    {
        return Err("engineering report timing boundary drifted".to_owned());
    }
    let event = timing
        .get("request_events")
        .and_then(Value::as_array)
        .and_then(|events| <&[Value; 1]>::try_from(events.as_slice()).ok())
        .and_then(|events| events[0].as_object())
        .ok_or_else(|| "engineering report must contain one timing event".to_owned())?;
    let first = event.get("first_token_offset_ns").and_then(Value::as_u64);
    let terminal = event.get("terminal_offset_ns").and_then(Value::as_u64);
    let output_tokens = event.get("output_tokens").and_then(Value::as_u64);
    let eligible = matches!(
        (output_tokens, first, terminal),
        (Some(output_tokens), Some(first), Some(terminal))
            if output_tokens >= 2 && first > 0 && terminal > first
    );
    if timing.get("r33_tpot_eligible").and_then(Value::as_bool) != Some(eligible) {
        return Err("engineering report attempted to invent TPOT eligibility".to_owned());
    }
    Ok(())
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_is_the_explicit_adapter_owned_command() {
        let error = run(&[]).unwrap_err();
        assert!(error.starts_with("usage: ferric-m1-engineering-target-smoke"));
        assert!(error.contains("ENGINEERING-OBSERVATION-DIRECTORY"));
        assert!(error.contains(DERIVED_ENGINEERING_IDENTITY_INPUT));
    }

    #[test]
    fn reserved_identity_input_is_explicit_and_external_paths_remain_compatible() {
        assert!(matches!(
            parse_engineering_identity_input(OsStr::new(DERIVED_ENGINEERING_IDENTITY_INPUT)),
            EngineeringIdentityInputV1::DerivedEngineeringV1
        ));
        let external = parse_engineering_identity_input(OsStr::new("closure.json"));
        assert!(matches!(
            external,
            EngineeringIdentityInputV1::ExternalFile(path) if path == Path::new("closure.json")
        ));
    }

    #[test]
    fn engineering_report_policy_is_explicitly_non_authoritative() {
        assert_eq!(
            STATUS,
            "engineering-hardware-observation-non-evidence-non-qualification"
        );
        assert!(NONCLAIM.contains("authority is none"));
        assert!(NONCLAIM.contains("not verified model answers"));
        assert!(NONCLAIM.contains("not comparable to R33 serving"));
        assert!(NONCLAIM.contains("not a qualification result"));
        assert!(NONCLAIM.contains("closes no M1 requirement"));
        for forbidden in ["qualified", "correct", "worker_v3_authority"] {
            assert!(!STATUS.contains(forbidden));
        }
    }

    #[test]
    fn engineering_report_binds_every_authority_identity_token_and_count_field() {
        let facts = EngineeringObservationFacts {
            manifest: Identity::new([1; 32]),
            hsaco: Identity::new([2; 32]),
            compiler_handoff: Identity::new([3; 32]),
            canonical_descriptor: Identity::new([4; 32]),
            program_catalog: Identity::new([5; 32]),
        };
        let report = engineering_report_from_parts(
            &[10, 11],
            &[20],
            &[30, 31],
            "max-new-tokens",
            EngineeringTimingFacts {
                duration: 100,
                first_token_offset: 30,
                terminal_offset: 80,
            },
            facts,
            Identity::new([6; 32]),
            Identity::new([7; 32]),
            EngineeringIdentityReportModeV1::ExternalFile,
            "ok",
            &[0x6f, 0x6b],
        );
        validate_engineering_report(&report).expect("canonical engineering report");
        let object = report.as_object().expect("report is an object");
        assert_eq!(object.len(), 29);
        assert_eq!(report["artifact_authority"], json!("none"));
        assert_eq!(report["authority"], json!("none"));
        assert_eq!(report["benchmark_comparable"], json!(false));
        assert_eq!(report["compiler_origin_authenticated"], json!(false));
        assert_eq!(report["current_publication_selected"], json!(false));
        assert_eq!(report["worker_v3_authenticated"], json!(false));
        assert_eq!(report["hardware_completion_observed"], json!(true));
        assert_eq!(
            report["observation_manifest_sha256"],
            json!("01".repeat(32))
        );
        assert_eq!(report["hsaco_sha256"], json!("02".repeat(32)));
        assert_eq!(report["compiler_handoff_sha256"], json!("03".repeat(32)));
        assert_eq!(
            report["canonical_descriptor_sha256"],
            json!("04".repeat(32))
        );
        assert_eq!(report["program_catalog_sha256"], json!("05".repeat(32)));
        assert_eq!(
            report["generated_runner_declaration_sha256"],
            json!("06".repeat(32))
        );
        assert_eq!(report["model_bundle_sha256"], json!("07".repeat(32)));
        assert_eq!(report["prompt_token_count"], json!(2));
        assert_eq!(report["prompt_token_ids"], json!([10, 11]));
        assert_eq!(report["prompt_priming_choice_token_ids"], json!([20]));
        assert_eq!(report["generated_token_count"], json!(2));
        assert_eq!(report["generated_token_ids"], json!([30, 31]));
        assert_eq!(report["target_choice_observation_count"], json!(3));
        assert_eq!(report["termination"], json!("max-new-tokens"));
        assert_eq!(report["text"], json!("ok"));
        assert_eq!(report["text_bytes_hex"], json!("6f6b"));
        assert_eq!(report["schema"], json!(EXTERNAL_FILE_SCHEMA));
        assert!(report.get("identity_input_mode").is_none());
        assert_eq!(report["timing"]["clock"], json!(TIMING_CLOCK));
        assert_eq!(
            report["timing"]["duration_boundary"],
            json!(TIMING_BOUNDARY)
        );
        assert_eq!(report["timing"]["duration_ns"], json!(100));
        assert_eq!(report["timing"]["r33_tpot_eligible"], json!(true));
        assert_eq!(report["timing"]["scope"], json!(TIMING_SCOPE));
        assert_eq!(
            report["timing"]["request_events"],
            json!([{
                "arrival_offset_ns": 0,
                "first_token_offset_ns": 30,
                "input_tokens": 2,
                "output_tokens": 2,
                "request_ordinal": 0,
                "terminal_offset_ns": 80,
            }])
        );
        assert!(report["timing"].get("tpot_ns").is_none());
    }

    fn timing_report(generated_tokens: &[u32], timing: EngineeringTimingFacts) -> Value {
        timing_report_with_mode(
            generated_tokens,
            timing,
            EngineeringIdentityReportModeV1::ExternalFile,
        )
    }

    fn timing_report_with_mode(
        generated_tokens: &[u32],
        timing: EngineeringTimingFacts,
        identity_mode: EngineeringIdentityReportModeV1,
    ) -> Value {
        engineering_report_from_parts(
            &[10, 11],
            &[20],
            generated_tokens,
            "max-new-tokens",
            timing,
            EngineeringObservationFacts {
                manifest: Identity::new([1; 32]),
                hsaco: Identity::new([2; 32]),
                compiler_handoff: Identity::new([3; 32]),
                canonical_descriptor: Identity::new([4; 32]),
                program_catalog: Identity::new([5; 32]),
            },
            Identity::new([6; 32]),
            Identity::new([7; 32]),
            identity_mode,
            "ok",
            &[0x6f, 0x6b],
        )
    }

    #[test]
    fn zero_and_one_token_reports_do_not_invent_tpot() {
        let zero = timing_report(
            &[],
            EngineeringTimingFacts {
                duration: 100,
                first_token_offset: 0,
                terminal_offset: 0,
            },
        );
        validate_engineering_report(&zero).expect("zero-token schema remains explicit");
        assert_eq!(zero["timing"]["r33_tpot_eligible"], json!(false));
        assert!(zero["timing"].get("tpot_ns").is_none());

        let one = timing_report(
            &[30],
            EngineeringTimingFacts {
                duration: 100,
                first_token_offset: 40,
                terminal_offset: 40,
            },
        );
        validate_engineering_report(&one).expect("one-token schema remains explicit");
        assert_eq!(one["timing"]["r33_tpot_eligible"], json!(false));
        assert!(one["timing"].get("tpot_ns").is_none());
    }

    #[test]
    fn engineering_report_rejects_hostile_authority_and_comparison_claims() {
        let report = timing_report(
            &[30],
            EngineeringTimingFacts {
                duration: 100,
                first_token_offset: 40,
                terminal_offset: 40,
            },
        );
        for (field, hostile) in [
            ("artifact_authority", json!("production")),
            ("authority", json!("benchmark")),
            ("benchmark_comparable", json!(true)),
            ("compiler_origin_authenticated", json!(true)),
            ("current_publication_selected", json!(true)),
            ("status", json!("qualified")),
            ("worker_v3_authenticated", json!(true)),
        ] {
            let mut mutated = report.clone();
            mutated[field] = hostile;
            assert!(
                validate_engineering_report(&mutated).is_err(),
                "hostile {field} claim was accepted"
            );
        }

        let mut invented_tpot = report.clone();
        invented_tpot["timing"]["r33_tpot_eligible"] = json!(true);
        assert!(validate_engineering_report(&invented_tpot).is_err());
    }

    #[test]
    fn derived_identity_report_is_v3_and_rejects_provenance_substitution() {
        let mut report = timing_report_with_mode(
            &[30],
            EngineeringTimingFacts {
                duration: 100,
                first_token_offset: 40,
                terminal_offset: 40,
            },
            EngineeringIdentityReportModeV1::DerivedEngineeringV1,
        );
        validate_engineering_report(&report).expect("derived engineering report is explicit");
        assert_eq!(report.as_object().map(serde_json::Map::len), Some(30));
        assert_eq!(report["schema"], json!(DERIVED_ENGINEERING_SCHEMA));
        assert_eq!(
            report["identity_input_mode"],
            json!(DERIVED_ENGINEERING_IDENTITY_MODE)
        );

        for hostile in [
            "authenticated",
            "externally-qualified",
            "protected-publication",
            "current-publication",
        ] {
            report["identity_input_mode"] = json!(hostile);
            assert!(validate_engineering_report(&report).is_err());
        }
        report["identity_input_mode"] = json!(DERIVED_ENGINEERING_IDENTITY_MODE);
        report["schema"] = json!(EXTERNAL_FILE_SCHEMA);
        assert!(validate_engineering_report(&report).is_err());

        report["schema"] = json!(DERIVED_ENGINEERING_SCHEMA);
        report["unexpected"] = json!(false);
        assert!(validate_engineering_report(&report).is_err());
    }

    #[test]
    #[ignore = "requires a real fe2o3 observation, canonical prepacked snapshot, and exclusive MI300X"]
    fn configured_mi300x_engineering_target_first_token_observation() {
        let required = |name: &str| {
            std::env::var_os(name).unwrap_or_else(|| panic!("set {name} for the exact fixture"))
        };
        let arguments = [
            required("FERRIC_M1_OPERATIONAL_SNAPSHOT_ROOT"),
            required("FERRIC_M1_ENGINEERING_AGGREGATE_OBSERVATION_DIRECTORY"),
            OsString::from(DERIVED_ENGINEERING_IDENTITY_INPUT),
            required("FERRIC_M1_GPU_UNIQUE_ID"),
            OsString::from("1"),
            required("FERRIC_M1_ENGINEERING_SMOKE_PROMPT"),
        ];
        run(&arguments).expect("one real engineering target token completes");
    }
}
