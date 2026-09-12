//! Separate exact-reference paged `Draft06B` canary. No speculative generation.

#![recursion_limit = "256"]

mod draft_paged_canary_contract;
mod tp_worker;

use std::io::Write;
use std::path::{Path, PathBuf};

use draft_paged_canary_contract::{
    ModelIdentity, Observation, ObservedStep, Options, PROMPT, Prefill, Reference, checked_choice,
    digest, finish_run, hash_file, hex, scheduled,
};
use ferric_build::{DRAFT_REPOSITORY, DRAFT_REVISION};
use ferric_m1_engineering_execution_v1::tp_artifact::EngineeringTpArtifactV1;
use ferric_m1_engineering_execution_v1::tp_execution::{
    EngineeringTpRankTransportV1, batched::EngineeringTpDraftBatchExecutionV10,
};
use ferric_m1_engineering_execution_v1::tp_model::EngineeringQwenModelV1;
use ferric_m1_engineering_execution_v1::tp_paged::{
    EngineeringTpPageRowV1, EngineeringTpPagedLimitsV1, EngineeringTpPagedPoolV1,
    EngineeringTpPoolScopeV1,
};
use serde::Serialize;
use tp_worker::Worker;

fn emit(value: &impl Serialize) -> Result<(), String> {
    let mut output = std::io::stdout().lock();
    serde_json::to_writer(&mut output, value).map_err(|e| e.to_string())?;
    output.write_all(b"\n").map_err(|e| e.to_string())?;
    output.flush().map_err(|e| e.to_string())
}

fn run_steps<R: EngineeringTpRankTransportV1>(
    engine: &mut EngineeringTpDraftBatchExecutionV10<R>,
    pool: &mut EngineeringTpPagedPoolV1,
    prompt: &[u32; 5],
    prefill: Prefill,
) -> Result<Observation, String> {
    if engine.completed_batches() != 0
        || engine.dispatch_counts() != [0]
        || !pool.is_empty()
        || engine.row_capacity() != 32
    {
        return Err("paged canary requires a fresh driver and pool".into());
    }
    let sequence = pool
        .open_sequence(pool.scope(), prompt, 0)
        .map_err(|e| format!("open: {e:?}"))?
        .sequence();
    let mut steps = Vec::new();
    let mut generated = Vec::new();
    let mut previous = None;
    for ordinal in 0..prefill.steps() {
        let schedule = scheduled(prefill, prompt, ordinal, previous)?;
        let rows = schedule
            .inputs
            .iter()
            .zip(&schedule.positions)
            .map(|(&token, &position)| EngineeringTpPageRowV1 {
                sequence,
                position,
                token,
            })
            .collect::<Vec<_>>();
        let batch = pool
            .reserve_batch(&rows)
            .map_err(|e| format!("reserve: {e:?}"))?;
        pool.begin_submission(&batch)
            .map_err(|e| format!("submission: {e:?}"))?;
        let observed = match engine.execute_selected(&batch, &[schedule.selected_row]) {
            Ok(value) => value,
            Err(error) => {
                let quarantine = pool.quarantine_batch(&batch);
                return Err(format!(
                    "paged forward failed: {error}; quarantine: {quarantine:?}"
                ));
            }
        };
        let choice = checked_choice(&observed.choices)?;
        pool.commit_batch(&batch, observed.completion)
            .map_err(|e| format!("commit: {e:?}"))?;
        let cursor = pool
            .committed_position(sequence)
            .map_err(|e| format!("cursor: {e:?}"))?;
        let completed = u64::try_from(ordinal + 1).map_err(|_| "ordinal")?;
        let counts = engine.dispatch_counts();
        if engine.completed_batches() != completed
            || counts != [completed * 480]
            || engine.expected_dispatch_counts(1) != [480]
            || cursor != schedule.positions.last().copied().ok_or("empty schedule")? + 1
        {
            return Err("exact paged completion/cursor/packet count differs".into());
        }
        if cursor >= 5 {
            generated.push(choice);
        }
        steps.push(ObservedStep {
            inputs: schedule.inputs,
            positions: schedule.positions,
            selected_row: schedule.selected_row,
            choice,
            cache_tokens: cursor,
            completed_dispatches: counts[0],
        });
        previous = Some(choice);
    }
    let generated_tokens: [u32; 2] = generated
        .try_into()
        .map_err(|_| "exact two output tokens required")?;
    if pool
        .committed_position(sequence)
        .map_err(|e| format!("final cursor: {e:?}"))?
        != 6
    {
        return Err("exact six resident inputs required".into());
    }
    pool.retire_sequence(sequence, false, 1)
        .map_err(|e| format!("retire: {e:?}"))?;
    pool.check_invariants()
        .map_err(|e| format!("pool invariants: {e:?}"))?;
    let stats = pool.stats();
    if stats.retained_pages != 0
        || stats.cached_pages != 0
        || stats.quarantined_pages != 0
        || stats.free_pages != 2
    {
        return Err("paged canary did not return its two private pages".into());
    }
    Ok(Observation {
        steps,
        generated_tokens,
    })
}

fn run(options: &Options) -> Result<(), String> {
    let reference = Reference::open(&options.reference, &options.reference_sha256)?;
    if reference.prefill != options.prefill.label() {
        return Err("reference schedule mismatch".into());
    }
    let controller_sha256 = hash_file(Path::new("/proc/self/exe"))?;
    if hash_file(&options.worker)? != options.worker_sha256 {
        return Err("worker external SHA-256 mismatch".into());
    }
    let artifact =
        EngineeringTpArtifactV1::open_draft32(&options.artifact).map_err(|e| e.to_string())?;
    let model = EngineeringQwenModelV1::open_with_draft(&options.source)?;
    let draft = model
        .draft()
        .ok_or("authenticated draft was not retained")?;
    let identity = ModelIdentity {
        model_bundle_id: hex(model.bundle_id().as_bytes()),
        draft_model_id: hex(draft.config().model_id.as_bytes()),
        draft_config_id: hex(draft.config().config_id.as_bytes()),
        draft_weights_sha256: digest(draft.weights()),
    };
    let prompt: [u32; 5] = model
        .encode(PROMPT)?
        .try_into()
        .map_err(|_| "canonical prompt is not five tokens")?;
    reference.bind(&identity, &prompt)?;
    let mut scope_session = [0_u8; 32];
    let mut entropy = std::fs::File::open("/dev/urandom").map_err(|e| e.to_string())?;
    std::io::Read::read_exact(&mut entropy, &mut scope_session).map_err(|e| e.to_string())?;
    let mut pool = EngineeringTpPagedPoolV1::new_wide32(
        EngineeringTpPoolScopeV1 {
            model: *draft.config().model_id.as_bytes(),
            session: scope_session,
        },
        EngineeringTpPagedLimitsV1::new(32, 1, 2, 100).map_err(|e| format!("limits: {e:?}"))?,
    )
    .map_err(|e| format!("pool: {e:?}"))?;
    let mut worker =
        Worker::spawn_with_options(&options.worker, options.device, &artifact, options.runtime)?;
    let pid = worker.pid();
    let running = hash_file(&PathBuf::from(format!("/proc/{pid}/exe")));
    let running_worker_sha256 = match running {
        Ok(hash) if hash == options.worker_sha256 => hash,
        other => {
            let close = worker.close();
            return Err(format!(
                "running worker identity: {other:?}; close: {close:?}"
            ));
        }
    };
    let mut engine = EngineeringTpDraftBatchExecutionV10::new(
        vec![worker],
        draft.config(),
        draft.weights(),
        draft.layout(),
        &pool,
        &artifact,
        options.projection,
    )?;
    let observed = (|| {
        emit(&serde_json::json!({
            "schema":"FerricDraftPagedCanarySetupV10", "authority":"none", "performance_qualified":false,
            "model":DRAFT_REPOSITORY, "model_revision":DRAFT_REVISION, "model_role":"Draft06B",
            "identity":identity, "projection":options.projection.label(), "attention":"baseline", "head_precision":"fp32-v10",
            "prefill":options.prefill.label(), "prompt":PROMPT, "prompt_tokens":prompt, "new_tokens":2,
            "tensor_parallel":1, "row_capacity":32, "context_tokens":32, "physical_pages":2,
            "draft_kv_payload_bytes":3_670_016_u64, "fp32_workspace_bytes":19_447_808_u64,
            "draft_payload_bytes":draft.weights().len(), "retained_target_payload_bytes":model.target_weights().len(),
            "transposed_weight_bytes":engine.transposed_weight_bytes(),
            "expected_steps":options.prefill.steps(), "expected_dispatches":options.prefill.steps()*480,
            "collective":"draft-device-tp1-v10", "output_head_pruning":true, "prefix_cache":false,
            "runtime_cache_admission":options.runtime.cache_admission, "runtime_operational":options.runtime.operational,
            "runtime_sequences":false, "runtime_ordered_batches":false, "runtime_rollover":false,
            "controller_sha256":controller_sha256, "worker_sha256":options.worker_sha256,
            "running_worker_sha256":running_worker_sha256, "worker_pid":pid, "device_unique_id":options.device,
            "artifact_hsaco_id":hex(artifact.hsaco_id().as_bytes()),
            "artifact_manifest_id":hex(artifact.manifest_id().as_bytes()),
            "artifact_handoff_id":hex(artifact.handoff_id().as_bytes()),
            "reference_sha256":options.reference_sha256, "reference_producer":reference.producer,
            "speculative_execution":false, "numerical_status":"fixed FP32-head reference canary only; no performance qualification"
        }))?;
        let observation = run_steps(&mut engine, &mut pool, &prompt, options.prefill)?;
        let bytes = model.decode(&observation.generated_tokens)?;
        let parity = reference.matches(&observation, &bytes);
        emit(&serde_json::json!({
            "schema":"FerricDraftPagedCanaryObservationV10", "authority":"none", "performance_qualified":false,
            "steps":observation.steps, "generated_tokens":observation.generated_tokens, "generated_utf8_bytes":bytes,
            "reference_passed":parity, "rank_dispatch_counts":engine.dispatch_counts(), "kv_tokens_processed":6,
            "free_pages":pool.stats().free_pages, "retained_pages":pool.stats().retained_pages, "cached_pages":pool.stats().cached_pages
        }))?;
        Ok::<_, String>(parity)
    })();
    let closed = engine.close();
    emit(&serde_json::json!({
        "schema":"FerricDraftPagedCanaryClosedV10", "authority":"none", "performance_qualified":false,
        "execution_completed":observed.is_ok(), "reference_passed":observed.as_ref().ok(),
        "all_workers_exited":closed.is_ok(), "worker_pid":pid, "rank_dispatch_counts":engine.dispatch_counts(),
        "completed_batches":engine.completed_batches()
    }))?;
    finish_run(observed, closed)
}

fn main() -> std::process::ExitCode {
    match Options::parse(std::env::args().skip(1)).and_then(|options| run(&options)) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Paged draft canary rejected: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
