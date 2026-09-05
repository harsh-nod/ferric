#![forbid(unsafe_code)]

//! Explicit non-authoritative target-smoke execution for one fe2o3 engineering observation.

mod smoke_bootstrap;

use fe2o3_kfd::{DeviceSelector, OpenedKfd};
use ferric_build::{SpecialTokenDecodePolicy, TokenizerExecutionLimits};
use ferric_engine::{M1TargetSmokeExecutionV1, execute_m1_target_smoke_v1};
use ferric_m1_engineering_execution_v1::{
    bind_engineering_structural_m1_physical_runner_v1, reopen_m1_engineering_aggregate_artifact_v1,
};
use ferric_spec::{Identity, M1_QUALIFICATION_TOKENS_PER_LANE};
use serde_json::{Value, json};
use std::ffi::OsString;
use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

type SmokeResult<T> = Result<T, String>;

const STATUS: &str = "engineering-hardware-observation-non-evidence-non-qualification";
const NONCLAIM: &str = "Raw-prompt target-only execution of a structurally admitted fe2o3 engineering aggregate whose authority is none. Reported choices are raw device observations, not verified model answers. This output authenticates no compiler process or Worker V3 publication, selects no current protected publication, establishes no numerical or hardware correctness, is not benchmark evidence, and closes no M1 requirement.";
const TARGET: &str = "gfx942:xnack-";

#[derive(Clone, Copy)]
struct EngineeringObservationFacts {
    manifest: Identity,
    hsaco: Identity,
    compiler_handoff: Identity,
    canonical_descriptor: Identity,
    program_catalog: Identity,
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
    let [
        prepacked_root,
        observation_root,
        closure_path,
        gpu_unique_id,
        max_new_tokens,
        prompt,
    ] = arguments
    else {
        return Err("usage: ferric-m1-engineering-target-smoke PREPACKED-SNAPSHOT ENGINEERING-OBSERVATION-DIRECTORY CLOSURE GPU-UNIQUE-ID MAX-NEW-TOKENS RAW-PROMPT".to_owned());
    };
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
    let facts = EngineeringObservationFacts {
        manifest: artifact.manifest_id(),
        hsaco: artifact.hsaco_id(),
        compiler_handoff: artifact.compiler_handoff_id(),
        canonical_descriptor: artifact.canonical_descriptor_id(),
        program_catalog: artifact.program_catalog_id(),
    };
    let bootstrap = smoke_bootstrap::prepare(
        Path::new(prepacked_root),
        Path::new(closure_path),
        prompt,
        facts.program_catalog,
    )?;
    let bound = bootstrap.bind(|publication| {
        bind_engineering_structural_m1_physical_runner_v1(artifact, publication)
            .map_err(|error| format!("cannot bind engineering physical runner: {error:?}"))
    })?;
    let checked = OpenedKfd::open_default()
        .map_err(|error| format!("cannot open KFD: {error}"))?
        .admit_uapi()
        .map_err(|error| format!("cannot admit pinned KFD UAPI: {error}"))?
        .bind_gfx942_xnack_minus(DeviceSelector::UniqueId(gpu_unique_id))
        .map_err(|error| format!("cannot bind selected gfx942:xnack- device: {error}"))?;
    let initialized = bound.initialize_memory(checked)?;
    let execution = execute_m1_target_smoke_v1(
        &initialized.runner,
        initialized.memory,
        initialized.prompt_tokens,
        max_new_tokens,
    )?;
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
        &text,
        &text_bytes,
    );
    let mut stdout = std::io::stdout().lock();
    serde_json::to_writer(&mut stdout, &report)
        .map_err(|error| format!("cannot serialize smoke report: {error}"))?;
    stdout
        .write_all(b"\n")
        .map_err(|error| format!("cannot write smoke report: {error}"))?;
    Ok(())
}

fn engineering_report(
    execution: &M1TargetSmokeExecutionV1,
    facts: EngineeringObservationFacts,
    runner_declaration: Identity,
    model_bundle: Identity,
    text: &str,
    text_bytes: &[u8],
) -> Value {
    engineering_report_from_parts(
        execution.prompt_tokens(),
        execution.prompt_observations(),
        execution.generated_tokens(),
        execution.termination(),
        facts,
        runner_declaration,
        model_bundle,
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
    facts: EngineeringObservationFacts,
    runner_declaration: Identity,
    model_bundle: Identity,
    text: &str,
    text_bytes: &[u8],
) -> Value {
    let target_choice_observation_count = prompt_observations
        .len()
        .saturating_add(generated_tokens.len());
    json!({
        "artifact_authority": "none",
        "authority": "none",
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
        "schema": "ferric.m1-engineering-target-smoke-observation.v1",
        "status": STATUS,
        "target": TARGET,
        "target_choice_observation_count": target_choice_observation_count,
        "termination": termination,
        "text": text,
        "text_bytes_hex": hex_bytes(text_bytes),
        "text_utf8_policy": "lossy-replacement",
        "worker_v3_authenticated": false,
    })
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
    }

    #[test]
    fn engineering_report_policy_is_explicitly_non_authoritative() {
        assert_eq!(
            STATUS,
            "engineering-hardware-observation-non-evidence-non-qualification"
        );
        assert!(NONCLAIM.contains("authority is none"));
        assert!(NONCLAIM.contains("not verified model answers"));
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
            facts,
            Identity::new([6; 32]),
            Identity::new([7; 32]),
            "ok",
            &[0x6f, 0x6b],
        );
        let object = report.as_object().expect("report is an object");
        assert_eq!(object.len(), 27);
        assert_eq!(report["artifact_authority"], json!("none"));
        assert_eq!(report["authority"], json!("none"));
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
            required("FERRIC_M1_QUALIFICATION_CLOSURE"),
            required("FERRIC_M1_GPU_UNIQUE_ID"),
            OsString::from("1"),
            required("FERRIC_M1_ENGINEERING_SMOKE_PROMPT"),
        ];
        run(&arguments).expect("one real engineering target token completes");
    }
}
