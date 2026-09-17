//! Bounded repeated greedy `Draft06B` execution on the existing engineering v10 path.

#![recursion_limit = "256"]

mod draft_decode_profile_contract;
// Reuse the unchanged canary helpers without exporting its unused fixed-run API.
#[allow(dead_code)]
mod draft_paged_canary_contract;
mod tp_worker;

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use draft_decode_profile_contract::{
    Options, PACKETS_PER_FORWARD, Reference, forward_count, identity_unchanged, offset, raw_ns,
    require_reusable_pool, scheduled,
};
use draft_paged_canary_contract::{ModelIdentity, PROMPT, checked_choice, digest, hash_file, hex};
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

#[derive(Clone, Copy)]
struct RunContext<'a> {
    options: &'a Options,
    reference: &'a Reference,
    model: &'a EngineeringQwenModelV1,
    origin: u64,
    run: usize,
}

fn run_once<R: EngineeringTpRankTransportV1>(
    engine: &mut EngineeringTpDraftBatchExecutionV10<R>,
    pool: &mut EngineeringTpPagedPoolV1,
    context: RunContext<'_>,
) -> Result<(), String> {
    let RunContext {
        options,
        reference,
        model,
        origin,
        run,
    } = context;
    require_reusable_pool(pool, options.physical_pages())?;
    if engine.row_capacity() != 32 {
        return Err("profile requires exact v10 row capacity".into());
    }
    let base_batches = engine.completed_batches();
    let base_dispatches = match engine.dispatch_counts().as_slice() {
        [count] => *count,
        _ => return Err("profile requires exactly one rank".into()),
    };
    let warmup = run < options.warmups;
    let run_start_ns = offset(origin)?;
    let sequence = pool
        .open_sequence(
            pool.scope(),
            &reference.prompt_tokens,
            u64::try_from(run * 2).map_err(|_| "run tick")?,
        )
        .map_err(|e| format!("open sequence: {e:?}"))?
        .sequence();
    let mut previous = None;
    let mut generated = Vec::with_capacity(options.new_tokens);
    let mut first_token_ns = None;
    let mut terminal_ns = None;
    for ordinal in 0..forward_count(options.common.prefill, options.new_tokens) {
        let plan = scheduled(
            options.common.prefill,
            &reference.prompt_tokens,
            options.new_tokens,
            ordinal,
            previous,
        )?;
        let expected = &reference.steps[ordinal];
        let reserve_start_ns = offset(origin)?;
        let rows = plan
            .inputs
            .iter()
            .zip(&plan.positions)
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
        let forward_start_ns = offset(origin)?;
        let observed = match engine.execute_selected(&batch, &[plan.selected_row]) {
            Ok(value) => value,
            Err(error) => {
                let quarantine = pool.quarantine_batch(&batch);
                return Err(format!(
                    "forward failed: {error}; quarantine: {quarantine:?}"
                ));
            }
        };
        let forward_complete_ns = offset(origin)?;
        let choice = match checked_choice(&observed.choices) {
            Ok(choice) => choice,
            Err(error) => {
                let quarantine = pool.quarantine_batch(&batch);
                return Err(format!(
                    "invalid choice: {error}; quarantine: {quarantine:?}"
                ));
            }
        };
        pool.commit_batch(&batch, observed.completion)
            .map_err(|e| format!("commit: {e:?}"))?;
        let cursor = pool
            .committed_position(sequence)
            .map_err(|e| format!("cursor: {e:?}"))?;
        let completed = u64::try_from(ordinal + 1).map_err(|_| "ordinal")?;
        let counts = engine.dispatch_counts();
        if engine.completed_batches() != base_batches + completed
            || counts != [base_dispatches + completed * PACKETS_PER_FORWARD]
            || engine.expected_dispatch_counts(1) != [PACKETS_PER_FORWARD]
            || cursor != plan.positions.last().ok_or("empty schedule")? + 1
        {
            return Err("profile exact completion/cursor/dispatch count mismatch".into());
        }
        let commit_complete_ns = offset(origin)?;
        let parity = choice == expected.choice
            && cursor == expected.cache_tokens
            && plan.inputs == expected.inputs
            && plan.positions == expected.positions
            && plan.selected_row == expected.selected_row;
        let output_index = if cursor >= 5 {
            Some(generated.len())
        } else {
            None
        };
        if cursor >= 5 {
            generated.push(choice);
            first_token_ns.get_or_insert(commit_complete_ns);
            terminal_ns = Some(commit_complete_ns);
        }
        emit(&serde_json::json!({
            "schema":"FerricDraftDecodeProfileStepV1", "authority":"none",
            "run":run, "warmup":warmup, "ordinal":ordinal, "output_index":output_index,
            "inputs":plan.inputs, "positions":plan.positions, "selected_row":plan.selected_row,
            "choice":choice, "expected_choice":expected.choice, "cache_tokens":cursor,
            "completed_dispatches":counts[0], "reserve_start_ns":reserve_start_ns,
            "forward_start_ns":forward_start_ns, "forward_complete_ns":forward_complete_ns,
            "commit_complete_ns":commit_complete_ns, "reference_passed":parity
        }))?;
        if !parity {
            return Err(format!(
                "run {run} step {ordinal} differs from independent reference"
            ));
        }
        previous = Some(choice);
    }
    let bytes = model.decode(&generated)?;
    let parity = generated == reference.generated_tokens && bytes == reference.generated_utf8_bytes;
    let cursor = pool
        .committed_position(sequence)
        .map_err(|e| format!("final cursor: {e:?}"))?;
    if cursor != u32::try_from(options.new_tokens + 4).map_err(|_| "cursor overflow")? {
        return Err("profile final KV cursor mismatch".into());
    }
    pool.retire_sequence(
        sequence,
        false,
        u64::try_from(run * 2 + 1).map_err(|_| "retire tick")?,
    )
    .map_err(|e| format!("retire: {e:?}"))?;
    pool.check_invariants()
        .map_err(|e| format!("pool invariants: {e:?}"))?;
    let stats = pool.stats();
    if stats.retained_pages != 0
        || stats.cached_pages != 0
        || stats.quarantined_pages != 0
        || stats.free_pages != options.physical_pages()
    {
        return Err("profile sequence did not return all private pages".into());
    }
    emit(&serde_json::json!({
        "schema":"FerricDraftDecodeProfileRunV1", "authority":"none", "performance_qualified":false,
        "run":run, "warmup":warmup, "run_start_ns":run_start_ns,
        "first_token_ns":first_token_ns.ok_or("missing first token")?,
        "terminal_ns":terminal_ns.ok_or("missing last token")?, "retired_ns":offset(origin)?,
        "generated_tokens":generated, "expected_tokens":reference.generated_tokens,
        "generated_utf8_bytes":bytes, "reference_passed":parity, "kv_tokens_processed":cursor,
        "steps":forward_count(options.common.prefill, options.new_tokens),
        "completed_dispatches":engine.dispatch_counts()[0] - base_dispatches,
        "free_pages":stats.free_pages, "retained_pages":stats.retained_pages,
        "cached_pages":stats.cached_pages, "quarantined_pages":stats.quarantined_pages
    }))?;
    if !parity {
        return Err(format!(
            "run {run} decoded bytes differ from independent reference"
        ));
    }
    Ok(())
}

fn run(options: &Options) -> Result<(), String> {
    let origin = raw_ns()?;
    let reference = Reference::open(options)?;
    let controller_sha256 = hash_file(Path::new("/proc/self/exe"))?;
    identity_unchanged(&options.common.worker, &options.common.worker_sha256)?;
    let artifact = EngineeringTpArtifactV1::open_draft32(&options.common.artifact)
        .map_err(|e| e.to_string())?;
    let model = EngineeringQwenModelV1::open_with_draft(&options.common.source)?;
    let draft = model.draft().ok_or("authenticated draft not retained")?;
    let identity = ModelIdentity {
        model_bundle_id: hex(model.bundle_id().as_bytes()),
        draft_model_id: hex(draft.config().model_id.as_bytes()),
        draft_config_id: hex(draft.config().config_id.as_bytes()),
        draft_weights_sha256: digest(draft.weights()),
    };
    let prompt: [u32; 5] = model
        .encode(PROMPT)?
        .try_into()
        .map_err(|_| "canonical prompt must have five tokens")?;
    reference.bind(&identity, &prompt)?;
    let mut session = [0_u8; 32];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut session))
        .map_err(|e| e.to_string())?;
    let mut pool = EngineeringTpPagedPoolV1::new_wide32(
        EngineeringTpPoolScopeV1 {
            model: *draft.config().model_id.as_bytes(),
            session,
        },
        EngineeringTpPagedLimitsV1::new(options.context_tokens(), 1, options.physical_pages(), 100)
            .map_err(|e| format!("limits: {e:?}"))?,
    )
    .map_err(|e| format!("pool: {e:?}"))?;
    let mut worker = Worker::spawn_with_options(
        &options.common.worker,
        options.common.device,
        &artifact,
        options.common.runtime,
    )?;
    let pid = worker.pid();
    if let Err(error) = identity_unchanged(
        &PathBuf::from(format!("/proc/{pid}/exe")),
        &options.common.worker_sha256,
    ) {
        let close = worker.close();
        return Err(format!("running worker: {error}; close: {close:?}"));
    }
    let mut engine = EngineeringTpDraftBatchExecutionV10::new(
        vec![worker],
        draft.config(),
        draft.weights(),
        draft.layout(),
        &pool,
        &artifact,
        options.common.projection,
    )?;
    let mut completed_runs = 0;
    let observed = (|| {
        emit(&serde_json::json!({
            "schema":"FerricDraftDecodeProfileSetupV1", "authority":"none", "performance_qualified":false,
            "clock":"CLOCK_MONOTONIC_RAW", "clock_origin_ns":origin, "setup_completed_ns":offset(origin)?,
            "timing_boundary":"host-reserve-through-checked-completion-and-KV-commit",
            "timing_exclusions":"model/artifact/worker setup; tokenization; retirement; teardown",
            "timing_includes":"controller/IPC/queue/waits; per-step observation emission between token completions",
            "device_timing":false, "device_overlap_claim":false, "sampling_class":options.sampling_class(),
            "warmups":options.warmups, "samples":options.samples, "new_tokens":options.new_tokens,
            "prompt":PROMPT, "prompt_tokens":prompt, "initial_kv_tokens":0, "prefill_tokens":5,
            "context_tokens":options.context_tokens(), "physical_pages":options.physical_pages(),
            "model":DRAFT_REPOSITORY, "model_revision":DRAFT_REVISION, "model_role":"Draft06B",
            "identity":identity, "projection":options.common.projection.label(), "prefill":options.common.prefill.label(),
            "head_precision":"fp32-v10", "collective":"draft-device-tp1-v10", "attention":"baseline",
            "tensor_parallel":1, "row_capacity":32, "prefix_cache":false, "eos_stopping":false,
            "reference_sha256":options.common.reference_sha256, "reference_producer":reference.producer,
            "controller_sha256":controller_sha256, "worker_sha256":options.common.worker_sha256,
            "running_worker_sha256":options.common.worker_sha256, "worker_pid":pid, "device_unique_id":options.common.device,
            "artifact_hsaco_id":hex(artifact.hsaco_id().as_bytes()), "artifact_manifest_id":hex(artifact.manifest_id().as_bytes()),
            "artifact_handoff_id":hex(artifact.handoff_id().as_bytes()),
            "runtime_cache_admission":options.common.runtime.cache_admission, "runtime_operational":options.common.runtime.operational,
            "runtime_sequences":false, "runtime_ordered_batches":false, "runtime_rollover":false,
            "expected_forwards_per_run":forward_count(options.common.prefill, options.new_tokens),
            "expected_dispatches_per_forward":PACKETS_PER_FORWARD,
            "expected_total_dispatches":options.total_dispatches(),
            "no_rollover_packet_budget":fe2o3_kfd::engineering_wire::MAX_UNRETIRED_RING_PACKETS_V1,
            "reused_worker_weights_and_pool":true, "retry_policy":"none-stop-on-first-error",
            "draft_payload_bytes":draft.weights().len(), "retained_target_payload_bytes":model.target_weights().len(),
            "transposed_weight_bytes":engine.transposed_weight_bytes()
        }))?;
        for ordinal in 0..options.warmups + options.samples {
            run_once(
                &mut engine,
                &mut pool,
                RunContext {
                    options,
                    reference: &reference,
                    model: &model,
                    origin,
                    run: ordinal,
                },
            )?;
            completed_runs += 1;
        }
        identity_unchanged(Path::new("/proc/self/exe"), &controller_sha256)?;
        identity_unchanged(&options.common.worker, &options.common.worker_sha256)?;
        identity_unchanged(
            &PathBuf::from(format!("/proc/{pid}/exe")),
            &options.common.worker_sha256,
        )?;
        Ok::<_, String>(())
    })();
    // Close even after clock, reference, output, or transport errors.
    let teardown_start = offset(origin);
    let closed = engine.close();
    let teardown_end = offset(origin);
    emit(&serde_json::json!({
        "schema":"FerricDraftDecodeProfileClosedV1", "authority":"none", "performance_qualified":false,
        "execution_completed":observed.is_ok(), "error":observed.as_ref().err(),
        "completed_runs":completed_runs, "all_workers_exited":closed.is_ok(), "close_error":closed.as_ref().err(),
        "teardown_start_ns":teardown_start.as_ref().ok(), "teardown_end_ns":teardown_end.as_ref().ok(),
        "worker_pid":pid, "rank_dispatch_counts":engine.dispatch_counts(), "completed_batches":engine.completed_batches()
    }))?;
    closed?;
    teardown_start?;
    teardown_end?;
    observed
}

fn main() -> std::process::ExitCode {
    match Options::parse(std::env::args().skip(1)).and_then(|options| run(&options)) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Draft decode profile rejected: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
