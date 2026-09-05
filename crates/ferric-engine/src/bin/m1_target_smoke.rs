//! Persisted-artifact target-only prompt-to-text smoke wrapper.

use super::{
    build_authenticated_sequential_plan_catalog, build_preliminary_identity_closure,
    complete_closure, generate_qwen3_gfx942_runner_declaration, hex_bytes, load_closure,
    load_model_inputs, model_memory_plan, publish_qwen3_gfx942_runner_declaration, CaptureResult,
    DeviceSelector, OpenedKfd, OsString, Path, SecureDirectory,
};
use ferric_build::{
    authenticate_qwen3_tokenizer, SpecialTokenDecodePolicy, SpecialTokenEncodePolicy,
    TokenizerExecutionLimits,
};
use ferric_engine::{
    bind_structural_m1_physical_runner_v1, execute_m1_target_smoke_v1,
    initialize_m1_physical_runner_memory_v1, reopen_persisted_m1_kernel_artifacts_v1,
    require_m1_authenticated_roster_acquisition_v1,
};
use ferric_spec::{Qwen3ModelRole, M1_QUALIFICATION_TOKENS_PER_LANE};
use serde_json::json;
use std::io::{Cursor, Write};

pub(super) const COMMAND: &str = "run-target-smoke";

const STATUS: &str = "smoke-non-evidence-non-qualification";
const AUTHORITY: &str = "ferric-target-only-smoke-only";
const NONCLAIM: &str = "Raw-prompt target-only text smoke only. Every prompt-priming and generation choice is reported as a non-evidence diagnostic and settled from the same inert physical K7 observation. This output is not evidence, is not a qualification result, does not establish numerical or hardware correctness, and closes no M1 requirement.";
const TARGET: &str = "gfx942:xnack-";

pub(super) fn run(arguments: &[OsString]) -> CaptureResult<()> {
    let [prepacked_root, artifact_root, closure_path, gpu_unique_id, max_new_tokens, prompt] =
        arguments
    else {
        return Err("usage: ferric-m1-qualification-capture run-target-smoke PREPACKED-SNAPSHOT KERNEL-ARTIFACTS CLOSURE GPU-UNIQUE-ID MAX-NEW-TOKENS RAW-PROMPT".to_owned());
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

    require_m1_authenticated_roster_acquisition_v1(Path::new(artifact_root))
        .map_err(|error| error.to_string())?;
    let closure = load_closure(Path::new(closure_path))?;
    let artifacts = reopen_persisted_m1_kernel_artifacts_v1(Path::new(artifact_root))
        .map_err(|error| format!("cannot authenticate persisted kernel artifacts: {error}"))?;
    let executable_catalog_id = artifacts.program_catalog_id();
    let snapshot = SecureDirectory::open(Path::new(prepacked_root), "prepacked snapshot root")?;
    let model = load_model_inputs(&snapshot)?;
    let tokenizer =
        authenticate_qwen3_tokenizer(Qwen3ModelRole::Target8B, Cursor::new(&model.tokenizer))
            .map_err(|error| format!("cannot authenticate target tokenizer: {error}"))?;
    let prompt_tokens = tokenizer
        .encode(
            prompt,
            TokenizerExecutionLimits::m1(),
            SpecialTokenEncodePolicy::Reject,
        )
        .map_err(|error| format!("cannot encode raw prompt: {error}"))?;

    let runner_admission = model.authenticate()?;
    let plan_catalog = build_authenticated_sequential_plan_catalog(runner_admission)
        .map_err(|error| format!("cannot build authenticated plan catalog: {error:?}"))?;
    let external = complete_closure(&closure, &plan_catalog, executable_catalog_id)?;
    let identity_closure = build_preliminary_identity_closure(plan_catalog, external)
        .map_err(|error| format!("cannot build runner identity closure: {error:?}"))?;
    let declaration = generate_qwen3_gfx942_runner_declaration(identity_closure)
        .map_err(|error| format!("cannot generate authenticated runner declaration: {error:?}"))?;
    let publication = publish_qwen3_gfx942_runner_declaration(declaration)
        .map_err(|error| format!("cannot publish runner declaration: {error:?}"))?;
    let runner = bind_structural_m1_physical_runner_v1(artifacts, publication)
        .map_err(|error| format!("cannot bind physical runner: {error:?}"))?;
    if runner.program_catalog_id() != executable_catalog_id {
        return Err("bound physical runner executable catalog drifted".to_owned());
    }

    let memory_admission = model.authenticate()?;
    let memory_plan = model_memory_plan(memory_admission)?;
    let checked = OpenedKfd::open_default()
        .map_err(|error| format!("cannot open KFD: {error}"))?
        .admit_uapi()
        .map_err(|error| format!("cannot admit pinned KFD UAPI: {error}"))?
        .bind_gfx942_xnack_minus(DeviceSelector::UniqueId(gpu_unique_id))
        .map_err(|error| format!("cannot bind selected gfx942:xnack- device: {error}"))?;
    let memory = initialize_m1_physical_runner_memory_v1(
        checked,
        memory_plan,
        model.target_weights,
        model.draft_weights,
    )
    .map_err(|error| format!("cannot initialize physical model memory: {error:?}"))?;

    let execution = execute_m1_target_smoke_v1(&runner, memory, prompt_tokens, max_new_tokens)?;
    let text_bytes = tokenizer
        .decode_to_bytes(
            execution.generated_tokens(),
            TokenizerExecutionLimits::m1(),
            SpecialTokenDecodePolicy::Skip,
        )
        .map_err(|error| format!("cannot decode generated token bytes: {error}"))?;
    let text = String::from_utf8_lossy(&text_bytes).into_owned();
    let direct_published_token_count = execution
        .prompt_observations()
        .len()
        .saturating_add(execution.generated_tokens().len());
    let report = json!({
        "authority": AUTHORITY,
        "direct_published_token_count": direct_published_token_count,
        "generated_token_count": execution.generated_tokens().len(),
        "generated_token_ids": execution.generated_tokens(),
        "nonclaim": NONCLAIM,
        "prompt_priming_published_choice_token_ids": execution.prompt_observations(),
        "status": STATUS,
        "target": TARGET,
        "termination": execution.termination(),
        "text": text,
        "text_bytes_hex": hex_bytes(&text_bytes),
        "text_utf8_policy": "lossy-replacement",
    });
    let mut stdout = std::io::stdout().lock();
    serde_json::to_writer(&mut stdout, &report)
        .map_err(|error| format!("cannot serialize smoke report: {error}"))?;
    stdout
        .write_all(b"\n")
        .map_err(|error| format!("cannot write smoke report: {error}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncated_utf8_is_preserved_in_hex_and_displayed_lossily() {
        let bytes = [0xf0, 0x9f];
        assert_eq!(hex_bytes(&bytes), "f09f");
        assert_eq!(String::from_utf8_lossy(&bytes), "\u{fffd}");
    }

    #[test]
    fn report_authority_is_explicitly_non_evidentiary() {
        assert_eq!(STATUS, "smoke-non-evidence-non-qualification");
        assert!(NONCLAIM.contains("not evidence"));
        assert!(NONCLAIM.contains("not a qualification result"));
        assert!(NONCLAIM.contains("closes no M1 requirement"));
        assert!(NONCLAIM.contains("Every prompt-priming and generation choice is reported"));
    }

    #[test]
    fn command_reports_the_exact_positional_usage() {
        let error = run(&[]).unwrap_err();
        assert!(error.contains("run-target-smoke PREPACKED-SNAPSHOT"));
        assert!(!error.contains("MODEL-SOURCE"));
        assert!(error.ends_with("MAX-NEW-TOKENS RAW-PROMPT"));
    }
}
