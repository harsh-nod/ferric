//! Fixed target-only argmax comparison, not a serving endpoint or M1 authority.

#![recursion_limit = "256"]

mod argmax_canary_contract;
// Reuse the frozen reference parser and byte helpers without changing its paired CLI.
#[allow(dead_code)]
mod paired_paged_canary_contract;
mod tp_host_timing;
mod tp_worker;

use argmax_canary_contract::{
    ArgmaxMode, CHUNK, CONTEXT, Options, PAGES, PREFIX_REFERENCE_SHA256, PROMPT, Reference,
    workload_sha256,
};
use ferric_m1_engineering_execution_v1::tp_artifact::EngineeringTpArtifactV1;
use ferric_m1_engineering_execution_v1::tp_execution::batched::EngineeringTpBatchExecutionV2;
use ferric_m1_engineering_execution_v1::tp_execution::{
    EngineeringTpProjectionModeV3, EngineeringTpRankTransportV1, EngineeringTpReductionModeV3,
};
use ferric_m1_engineering_execution_v1::tp_model::EngineeringQwenModelV1;
use ferric_m1_engineering_execution_v1::tp_paged::{
    EngineeringTpPageRowV1, EngineeringTpPagedErrorV1, EngineeringTpPagedLimitsV1,
    EngineeringTpPagedPoolV1, EngineeringTpPoolScopeV1, EngineeringTpSequenceIdV1,
};
use paired_paged_canary_contract::{SOURCE_REFERENCE_SHA256, hash_file, hex};
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tp_host_timing::TimingFile;
use tp_worker::Worker;

fn emit(value: &Value) -> Result<(), String> {
    let mut output = std::io::stdout().lock();
    serde_json::to_writer(&mut output, value).map_err(|e| e.to_string())?;
    output
        .write_all(b"\n")
        .and_then(|()| output.flush())
        .map_err(|e| e.to_string())
}

fn count(values: &[u64]) -> Result<u64, String> {
    match values {
        [value] => Ok(*value),
        _ => Err("one TP1 packet counter required".into()),
    }
}

fn identity(artifact: &EngineeringTpArtifactV1) -> Value {
    json!({"hsaco":hex(artifact.hsaco_id().as_bytes()), "manifest":hex(artifact.manifest_id().as_bytes()),
        "handoff":hex(artifact.handoff_id().as_bytes())})
}

fn retire_no_cache(
    pool: &mut EngineeringTpPagedPoolV1,
    sequence: EngineeringTpSequenceIdV1,
) -> Result<(), String> {
    pool.retire_sequence(sequence, false, 1)
        .map_err(|e| format!("retire: {e:?}"))?;
    pool.check_invariants()
        .map_err(|e| format!("retired pool: {e:?}"))?;
    // is_empty is the virgin-driver admission guard; retirement never rewinds IDs.
    let stats = pool.stats();
    if stats.sequences != 0
        || stats.free_pages != PAGES
        || stats.retained_pages != 0
        || stats.cached_pages != 0
        || stats.quarantined_pages != 0
        || pool.committed_position(sequence) != Err(EngineeringTpPagedErrorV1::UnknownSequence)
    {
        return Err("retired no-cache pool must release every page and sequence".into());
    }
    Ok(())
}

fn execute_step<R: EngineeringTpRankTransportV1>(
    driver: &mut EngineeringTpBatchExecutionV2<R>,
    pool: &mut EngineeringTpPagedPoolV1,
    sequence: EngineeringTpSequenceIdV1,
    tokens: &[u32],
    publish: bool,
) -> Result<(Option<u32>, Value), String> {
    let start = pool
        .committed_position(sequence)
        .map_err(|e| format!("cursor: {e:?}"))?;
    let rows = tokens
        .iter()
        .enumerate()
        .map(|(index, &token)| {
            Ok(EngineeringTpPageRowV1 {
                sequence,
                token,
                position: start
                    .checked_add(u32::try_from(index).map_err(|_| "row index")?)
                    .ok_or("position overflow")?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let selected = if publish {
        vec![rows.len().checked_sub(1).ok_or("empty output batch")?]
    } else {
        Vec::new()
    };
    let batch = pool
        .reserve_batch(&rows)
        .map_err(|e| format!("reserve: {e:?}"))?;
    let before = count(&driver.dispatch_counts())?;
    let ordinal = driver
        .completed_batches()
        .checked_add(1)
        .ok_or("batch ordinal overflow")?;
    pool.begin_submission(&batch)
        .map_err(|e| format!("begin: {e:?}"))?;
    let submitted = (|| {
        let output = driver.execute_selected(&batch, &selected)?;
        let completed = count(&driver.dispatch_counts())?;
        let expected_delta = if publish { 616 } else { 613 };
        if output.choices.len() != usize::from(publish)
            || completed.checked_sub(before) != Some(expected_delta)
            || driver.completed_batches() != ordinal
            || batch.id() != ordinal
        {
            return Err("step packet/output/batch contract drift".into());
        }
        pool.commit_batch(&batch, output.completion)
            .map_err(|e| format!("commit: {e:?}"))?;
        pool.check_invariants()
            .map_err(|e| format!("pool: {e:?}"))?;
        let step = json!({"batch_id":batch.id(), "inputs":rows.iter().map(|row|
            json!({"token":row.token,"position":row.position})).collect::<Vec<_>>(),
            "selected_rows":selected,"head_rows":usize::from(publish),"choices":output.choices,
            "completed_packets":completed,"committed_position":pool.committed_position(sequence).map_err(|e| format!("committed cursor: {e:?}"))?});
        Ok((output.choices.first().copied(), step))
    })();
    submitted.map_err(|error: String| {
        let quarantine = pool.quarantine_batch(&batch);
        format!("{error}; quarantine: {quarantine:?}")
    })
}

fn observe<R: EngineeringTpRankTransportV1>(
    driver: &mut EngineeringTpBatchExecutionV2<R>,
    pool: &mut EngineeringTpPagedPoolV1,
    model: &EngineeringQwenModelV1,
    reference: &Reference,
    options: &Options,
) -> Result<bool, String> {
    if reference.source.prompt_token_ids.len() != PROMPT
        || !pool.is_empty()
        || driver.completed_batches() != 0
        || count(&driver.dispatch_counts())? != 0
    {
        return Err("fresh fixed prompt/driver/pool required".into());
    }
    let hit = pool
        .open_sequence(pool.scope(), &reference.source.prompt_token_ids, 0)
        .map_err(|e| format!("open sequence: {e:?}"))?;
    if hit.hit_pages() != 0 || hit.hit_tokens() != 0 {
        return Err("prefix reuse forbidden".into());
    }
    let sequence = hit.sequence();
    let mut generated = Vec::with_capacity(options.outputs);
    let mut elapsed_seconds = Vec::with_capacity(options.outputs);
    let mut prefill = Vec::with_capacity(PROMPT / CHUNK);
    let started = Instant::now();
    for (ordinal, tokens) in reference.source.prompt_token_ids.chunks(CHUNK).enumerate() {
        let publish = ordinal == PROMPT / CHUNK - 1;
        let (choice, step) = execute_step(driver, pool, sequence, tokens, publish)?;
        if let Some(token) = choice {
            generated.push(token);
            elapsed_seconds.push(started.elapsed().as_secs_f64());
        }
        prefill.push(step);
    }
    if generated.len() != 1 {
        return Err("prefill must produce exactly one choice".into());
    }
    emit(
        &json!({"schema":"FerricArgmaxCanaryPrefillV1","authority":"none","performance_qualified":false,
        "steps":prefill,"generated_token":generated[0],"elapsed_seconds":elapsed_seconds[0]}),
    )?;
    let mut decode = Vec::with_capacity(options.outputs - 1);
    while generated.len() < options.outputs {
        let token = *generated.last().ok_or("missing actual decode input")?;
        let (choice, step) = execute_step(driver, pool, sequence, &[token], true)?;
        generated.push(choice.ok_or("missing actual decode output")?);
        elapsed_seconds.push(started.elapsed().as_secs_f64());
        decode.push(step);
    }
    let workload_seconds = started.elapsed().as_secs_f64();
    let utf8 = hex(&model.decode(&generated)?);
    let parity = generated == reference.source.generated_token_ids[..options.outputs]
        && utf8 == reference.expected_utf8(options.outputs)?;
    let committed = pool
        .committed_position(sequence)
        .map_err(|e| format!("final cursor: {e:?}"))?;
    if count(&driver.dispatch_counts())? != options.expected_packets()?
        || driver.completed_batches()
            != u64::try_from(7 + options.outputs).map_err(|_| "batches")?
        || usize::try_from(committed).map_err(|_| "cursor")? != PROMPT - 1 + options.outputs
    {
        return Err("final packet/batch/resident-input contract drift".into());
    }
    retire_no_cache(pool, sequence)?;
    emit(
        &json!({"schema":"FerricArgmaxCanaryObservationV1","authority":"none","performance_qualified":false,
        "generated_token_ids":generated,"generated_utf8_hex":utf8,"reference_passed":parity,
        "elapsed_seconds":elapsed_seconds,"workload_seconds":workload_seconds,"decode_steps":decode,
        "completed_packets":count(&driver.dispatch_counts())?,"completed_batches":driver.completed_batches(),
        "committed_inputs_before_retirement":committed,"pool_retired":true}),
    )?;
    if !parity {
        return Err("exact unchanged target reference differs".into());
    }
    Ok(true)
}

fn run(options: &Options, timing: &mut TimingFile) -> Result<(), String> {
    let reference = Reference::open(options)?;
    let controller_sha256 = hash_file(Path::new("/proc/self/exe"))?;
    if hash_file(&options.worker)? != options.worker_sha256 {
        return Err("worker hash differs".into());
    }
    let target_artifact = EngineeringTpArtifactV1::open_batch32(
        &options.target_artifact,
        &ferric_qwen3_tp_batch32_kernels_device_v5::compiler_expectation_roster_v5(),
        true,
    )
    .map_err(|e| e.to_string())?;
    let head_artifact = EngineeringTpArtifactV1::open_fp32_head32(
        &options.target_head_artifact,
        &ferric_qwen3_tp_fp32_head32_kernels_device_v8::compiler_expectation_roster_v8(),
    )
    .map_err(|e| e.to_string())?;
    let argmax_artifact = EngineeringTpArtifactV1::open_fp32_argmax32_v11(&options.argmax_artifact)
        .map_err(|e| e.to_string())?;
    let model = EngineeringQwenModelV1::open(&options.source)?;
    let mut session = [0_u8; 32];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut session))
        .map_err(|e| e.to_string())?;
    let limits = EngineeringTpPagedLimitsV1::new(CONTEXT, 1, PAGES, 100)
        .map_err(|e| format!("limits: {e:?}"))?;
    let mut pool = EngineeringTpPagedPoolV1::new_wide32(
        EngineeringTpPoolScopeV1 {
            model: *model.bundle_id().as_bytes(),
            session,
        },
        limits,
    )
    .map_err(|e| format!("pool: {e:?}"))?;
    let mut worker = Worker::spawn_with_timing(
        &options.worker,
        options.device,
        &target_artifact,
        options.runtime,
        timing.timing.clone(),
        0,
    )?;
    let worker_pid = worker.pid();
    let admitted = (|| {
        if hash_file(&PathBuf::from(format!("/proc/{worker_pid}/exe")))? != options.worker_sha256 {
            return Err("running worker hash differs".into());
        }
        worker.load_additional_artifact(&head_artifact)?;
        worker.load_additional_artifact(&argmax_artifact)
    })();
    if let Err(error) = admitted {
        let close = worker.close();
        return Err(format!("{error}; worker close: {close:?}"));
    }
    let mut driver = EngineeringTpBatchExecutionV2::new_wide32_with_argmax_v11(
        vec![worker],
        model.config(),
        model.target_weights(),
        model.layout(),
        &pool,
        &argmax_artifact,
    )?;
    let observed = (|| {
        driver.configure_output_head_pruning(true)?;
        driver.configure_reduction(EngineeringTpReductionModeV3::DeviceTp1V3)?;
        driver.configure_projection(
            EngineeringTpProjectionModeV3::Mfma,
            model.target_weights(),
            model.layout(),
        )?;
        driver.configure_head_precision_v8(true)?;
        if options.mode == ArgmaxMode::WaveV11 {
            driver.configure_fp32_argmax_v11(&argmax_artifact)?;
        }
        driver.configure_host_timing(timing.timing.clone())?;
        if driver.expected_dispatch_counts(0) != [613]
            || driver.expected_dispatch_counts(1) != [616]
            || driver.fp32_argmax_mode() != options.mode.label()
        {
            return Err("configured selector/packet contract differs".into());
        }
        let workload_sha256 = workload_sha256(&reference.source.prompt_token_ids, options.outputs)?;
        timing.workload_sha256 = Some(workload_sha256.clone());
        let setup = json!({"schema":"FerricArgmaxCanarySetupV1","authority":"none","performance_qualified":false,
            "controller_sha256":controller_sha256,"worker_sha256":options.worker_sha256,"worker_pid":worker_pid,
            "device_unique_id":options.device,"session":hex(&session),"pool_identity":pool.identity(),
            "model_bundle_id":hex(model.bundle_id().as_bytes()),"target_model_id":hex(model.config().model_id.as_bytes()),
            "source_reference_sha256":SOURCE_REFERENCE_SHA256,"prefix_reference_sha256":PREFIX_REFERENCE_SHA256,
            "workload_sha256":workload_sha256,"target_artifact":identity(&target_artifact),
            "target_head_artifact":identity(&head_artifact),"argmax_artifact":identity(&argmax_artifact),
            "argmax_mode":options.mode.label(),"prompt_token_ids":reference.source.prompt_token_ids,
            "max_new_tokens":options.outputs,"row_capacity":32,"prefill_chunk":CHUNK,"context":CONTEXT,"pages":PAGES,
            "projection":"mfma","attention":"baseline","head_precision":"fp32-v8","collective":"device-tp1-v3",
            "prefix_cache":false,"runtime_cache_admission":true,"runtime_operational":true,"runtime_rollover":true,
            "runtime_ordered_batches":false,"runtime_sequences":false,"target_payload_bytes":model.target_weights().len(),
            "target_transposed_bytes":driver.transposed_weight_bytes(),"target_kv_payload_bytes":limits.target_kv_payload_bytes().map_err(|e| format!("KV: {e:?}"))?,
            "fp32_workspace_bytes":driver.fp32_head_workspace_bytes(),"expected_packets":options.expected_packets()?,
            "expected_batches":7+options.outputs,"timing_boundary":"host-prefill-start-through-generated-token-commit; excludes setup; not HTTP or GPU duration"});
        timing.setup = Some(setup.clone());
        emit(&setup)?;
        observe(&mut driver, &mut pool, &model, &reference, options)
    })();
    // Closure is attempted on every configuration, execution, reference or output failure.
    let close = driver.close();
    let closed = json!({"schema":"FerricArgmaxCanaryClosedV1","authority":"none","performance_qualified":false,
        "execution_completed":observed.is_ok(),"reference_passed":observed.as_ref().ok(),
        "worker_exited":close.is_ok(),"worker_pid":worker_pid,
        "completed_packets":driver.dispatch_counts(),"completed_batches":driver.completed_batches()});
    timing.closed = Some(closed.clone());
    let emitted = emit(&closed);
    match (observed, close, emitted) {
        (Ok(true), Ok(()), Ok(())) => Ok(()),
        (result, close, emitted) => Err(format!(
            "observation: {result:?}; close: {close:?}; closed record: {emitted:?}"
        )),
    }
}

fn main() -> std::process::ExitCode {
    let result = Options::parse(std::env::args().skip(1)).and_then(|options| {
        let mut timing = TimingFile::create(Some(&options.host_timing_output))?;
        let result = run(&options, &mut timing);
        let sidecar = timing.finish(&result);
        match (result, sidecar) {
            (Ok(()), Ok(())) => Ok(()),
            (result, sidecar) => Err(format!("run: {result:?}; timing: {sidecar:?}")),
        }
    });
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Argmax canary rejected: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod retirement_tests {
    use super::*;

    fn reserved() -> (
        EngineeringTpPagedPoolV1,
        EngineeringTpSequenceIdV1,
        ferric_m1_engineering_execution_v1::tp_paged::EngineeringTpPreparedBatchV1,
    ) {
        let scope = EngineeringTpPoolScopeV1 {
            model: [1; 32],
            session: [2; 32],
        };
        let limits = EngineeringTpPagedLimitsV1::new(CONTEXT, 1, PAGES, 100).unwrap();
        let mut pool = EngineeringTpPagedPoolV1::new_wide32(scope, limits).unwrap();
        assert!(pool.is_empty());
        let sequence = pool
            .open_sequence(scope, &[7; PROMPT], 0)
            .unwrap()
            .sequence();
        let batch = pool
            .reserve_batch(&[EngineeringTpPageRowV1 {
                sequence,
                token: 7,
                position: 0,
            }])
            .unwrap();
        (pool, sequence, batch)
    }

    #[test]
    fn no_cache_retirement_checks_ownership_without_resetting_freshness() {
        let (mut pool, sequence, batch) = reserved();
        pool.abort_batch(&batch).unwrap();
        retire_no_cache(&mut pool, sequence).unwrap();
        assert!(!pool.is_empty());
        assert_eq!(pool.stats().free_pages, PAGES);
        assert!(retire_no_cache(&mut pool, sequence).is_err());
        let next = pool.open_sequence(pool.scope(), &[7; PROMPT], 1).unwrap();
        assert_ne!(next.sequence(), sequence);
        assert_eq!((next.hit_tokens(), next.hit_pages()), (0, 0));
        retire_no_cache(&mut pool, next.sequence()).unwrap();
    }

    #[test]
    fn no_cache_retirement_rejects_pending_and_quarantined_work() {
        let (mut pool, sequence, batch) = reserved();
        assert!(retire_no_cache(&mut pool, sequence).is_err());
        assert_eq!(pool.stats().sequences, 1);
        pool.begin_submission(&batch).unwrap();
        assert!(retire_no_cache(&mut pool, sequence).is_err());
        pool.quarantine_batch(&batch).unwrap();
        assert!(retire_no_cache(&mut pool, sequence).is_err());
        assert_eq!(pool.stats().quarantined_pages, PAGES);
        assert_eq!(pool.stats().free_pages, 0);
    }
}
