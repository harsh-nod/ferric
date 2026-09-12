//! Shared fixed-target execution; profiles cannot override the workload or reference.

use super::argmax_canary_contract::{
    ArgmaxMode, CHUNK, CONTEXT, Options, PAGES, PREFIX_REFERENCE_SHA256, PROMPT, Reference,
    workload_sha256,
};
use super::paired_paged_canary_contract::{SOURCE_REFERENCE_SHA256, hash_file, hex};
use super::tp_host_timing::TimingFile;
use super::tp_worker::Worker;
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
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanaryProfile {
    LegacyArgmax,
    AttentionBaseline,
    AttentionWave,
    SubmissionSynchronous,
    SubmissionOrdered,
}

#[derive(Clone, Copy)]
enum RecordKind {
    Setup,
    Prefill,
    Observation,
    Closed,
}

impl CanaryProfile {
    const fn schema(self, record: RecordKind) -> &'static str {
        match (self, record) {
            (Self::LegacyArgmax, RecordKind::Setup) => "FerricArgmaxCanarySetupV1",
            (Self::LegacyArgmax, RecordKind::Prefill) => "FerricArgmaxCanaryPrefillV1",
            (Self::LegacyArgmax, RecordKind::Observation) => "FerricArgmaxCanaryObservationV1",
            (Self::LegacyArgmax, RecordKind::Closed) => "FerricArgmaxCanaryClosedV1",
            (Self::AttentionBaseline | Self::AttentionWave, RecordKind::Setup) => {
                "FerricAttentionArgmaxCanarySetupV1"
            }
            (Self::AttentionBaseline | Self::AttentionWave, RecordKind::Prefill) => {
                "FerricAttentionArgmaxCanaryPrefillV1"
            }
            (Self::AttentionBaseline | Self::AttentionWave, RecordKind::Observation) => {
                "FerricAttentionArgmaxCanaryObservationV1"
            }
            (Self::AttentionBaseline | Self::AttentionWave, RecordKind::Closed) => {
                "FerricAttentionArgmaxCanaryClosedV1"
            }
            (Self::SubmissionSynchronous | Self::SubmissionOrdered, RecordKind::Setup) => {
                "FerricWaveArgmaxSubmissionCanarySetupV1"
            }
            (Self::SubmissionSynchronous | Self::SubmissionOrdered, RecordKind::Prefill) => {
                "FerricWaveArgmaxSubmissionCanaryPrefillV1"
            }
            (Self::SubmissionSynchronous | Self::SubmissionOrdered, RecordKind::Observation) => {
                "FerricWaveArgmaxSubmissionCanaryObservationV1"
            }
            (Self::SubmissionSynchronous | Self::SubmissionOrdered, RecordKind::Closed) => {
                "FerricWaveArgmaxSubmissionCanaryClosedV1"
            }
        }
    }

    const fn attention(self) -> &'static str {
        match self {
            Self::AttentionWave | Self::SubmissionSynchronous | Self::SubmissionOrdered => "wave",
            Self::LegacyArgmax | Self::AttentionBaseline => "baseline",
        }
    }

    const fn wave_attention(self) -> bool {
        matches!(
            self,
            Self::AttentionWave | Self::SubmissionSynchronous | Self::SubmissionOrdered
        )
    }

    const fn ordered_batches(self) -> bool {
        matches!(self, Self::SubmissionOrdered)
    }

    const fn error_prefix(self) -> &'static str {
        match self {
            Self::LegacyArgmax => "Argmax canary rejected",
            Self::AttentionBaseline | Self::AttentionWave => "Attention argmax canary rejected",
            Self::SubmissionSynchronous | Self::SubmissionOrdered => {
                "Wave argmax submission canary rejected"
            }
        }
    }

    fn validate(self, mode: ArgmaxMode) -> Result<(), String> {
        if self != Self::LegacyArgmax && mode != ArgmaxMode::WaveV11 {
            return Err("attention comparison requires fixed argmax mode wave-v11".into());
        }
        Ok(())
    }

    fn validate_options(self, options: &Options) -> Result<(), String> {
        self.validate(options.mode)?;
        if matches!(self, Self::SubmissionSynchronous | Self::SubmissionOrdered)
            && (!matches!(options.outputs, 8 | 128)
                || !options.runtime.cache_admission
                || !options.runtime.operational
                || !options.runtime.rollover
                || options.runtime.sequences
                || options.runtime.profile
                || options.runtime.shared_full_currentness
                || options.runtime.ordered_batches != self.ordered_batches())
        {
            return Err("submission profile requires 8 or 128 outputs, matching ordered policy, cache admission, operational currentness and rollover, without sequences, runtime profiling or shared currentness".into());
        }
        Ok(())
    }
}

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
    profile: CanaryProfile,
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
        &json!({"schema":profile.schema(RecordKind::Prefill),"authority":"none","performance_qualified":false,
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
        &json!({"schema":profile.schema(RecordKind::Observation),"authority":"none","performance_qualified":false,
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

fn run(options: &Options, profile: CanaryProfile, timing: &mut TimingFile) -> Result<(), String> {
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
        if profile.wave_attention() {
            driver.configure_wave_attention(true)?;
        }
        driver.configure_head_precision_v8(true)?;
        if options.mode == ArgmaxMode::WaveV11 {
            if profile.ordered_batches() {
                driver.configure_ordered_wave_attention_fp32_argmax_v11(&argmax_artifact)?;
            } else if profile.wave_attention() {
                driver.configure_wave_attention_fp32_argmax_v11(&argmax_artifact)?;
            } else {
                driver.configure_fp32_argmax_v11(&argmax_artifact)?;
            }
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
        let setup = json!({"schema":profile.schema(RecordKind::Setup),"authority":"none","performance_qualified":false,
            "controller_sha256":controller_sha256,"worker_sha256":options.worker_sha256,"worker_pid":worker_pid,
            "device_unique_id":options.device,"session":hex(&session),"pool_identity":pool.identity(),
            "model_bundle_id":hex(model.bundle_id().as_bytes()),"target_model_id":hex(model.config().model_id.as_bytes()),
            "source_reference_sha256":SOURCE_REFERENCE_SHA256,"prefix_reference_sha256":PREFIX_REFERENCE_SHA256,
            "workload_sha256":workload_sha256,"target_artifact":identity(&target_artifact),
            "target_head_artifact":identity(&head_artifact),"argmax_artifact":identity(&argmax_artifact),
            "argmax_mode":options.mode.label(),"prompt_token_ids":reference.source.prompt_token_ids,
            "max_new_tokens":options.outputs,"row_capacity":32,"prefill_chunk":CHUNK,"context":CONTEXT,"pages":PAGES,
            "projection":"mfma","attention":profile.attention(),"head_precision":"fp32-v8","collective":"device-tp1-v3",
            "prefix_cache":false,"runtime_cache_admission":true,"runtime_operational":true,"runtime_rollover":true,
            "runtime_ordered_batches":profile.ordered_batches(),"runtime_sequences":false,"target_payload_bytes":model.target_weights().len(),
            "target_transposed_bytes":driver.transposed_weight_bytes(),"target_kv_payload_bytes":limits.target_kv_payload_bytes().map_err(|e| format!("KV: {e:?}"))?,
            "fp32_workspace_bytes":driver.fp32_head_workspace_bytes(),"expected_packets":options.expected_packets()?,
            "expected_batches":7+options.outputs,"timing_boundary":"host-prefill-start-through-generated-token-commit; excludes setup; not HTTP or GPU duration"});
        timing.setup = Some(setup.clone());
        emit(&setup)?;
        observe(&mut driver, &mut pool, &model, &reference, options, profile)
    })();
    // Closure is attempted on every configuration, execution, reference or output failure.
    let close = driver.close();
    let closed = json!({"schema":profile.schema(RecordKind::Closed),"authority":"none","performance_qualified":false,
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

pub fn execute(options: Result<Options, String>, profile: CanaryProfile) -> std::process::ExitCode {
    let result = options.and_then(|options| {
        profile.validate_options(&options)?;
        let mut timing = TimingFile::create(Some(&options.host_timing_output))?;
        let result = run(&options, profile, &mut timing);
        let sidecar = timing.finish(&result);
        match (result, sidecar) {
            (Ok(()), Ok(())) => Ok(()),
            (result, sidecar) => Err(format!("run: {result:?}; timing: {sidecar:?}")),
        }
    });
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{}: {error}", profile.error_prefix());
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

#[cfg(test)]
mod profile_tests {
    use super::*;

    #[test]
    fn closed_profiles_preserve_legacy_labels_and_distinguish_attention_records() {
        for (record, legacy, attention) in [
            (
                RecordKind::Setup,
                "FerricArgmaxCanarySetupV1",
                "FerricAttentionArgmaxCanarySetupV1",
            ),
            (
                RecordKind::Prefill,
                "FerricArgmaxCanaryPrefillV1",
                "FerricAttentionArgmaxCanaryPrefillV1",
            ),
            (
                RecordKind::Observation,
                "FerricArgmaxCanaryObservationV1",
                "FerricAttentionArgmaxCanaryObservationV1",
            ),
            (
                RecordKind::Closed,
                "FerricArgmaxCanaryClosedV1",
                "FerricAttentionArgmaxCanaryClosedV1",
            ),
        ] {
            assert_eq!(CanaryProfile::LegacyArgmax.schema(record), legacy);
            assert_eq!(CanaryProfile::AttentionBaseline.schema(record), attention);
            assert_eq!(CanaryProfile::AttentionWave.schema(record), attention);
            assert_ne!(legacy, attention);
        }
        assert_eq!(CanaryProfile::LegacyArgmax.attention(), "baseline");
        assert_eq!(CanaryProfile::AttentionBaseline.attention(), "baseline");
        assert_eq!(CanaryProfile::AttentionWave.attention(), "wave");
        assert_eq!(
            CanaryProfile::LegacyArgmax.error_prefix(),
            "Argmax canary rejected"
        );
        assert_eq!(
            CanaryProfile::AttentionWave.error_prefix(),
            "Attention argmax canary rejected"
        );
    }

    #[test]
    fn attention_runtime_rejects_serial_even_without_the_cli_parser() {
        for mode in [ArgmaxMode::Serial, ArgmaxMode::WaveV11] {
            assert!(CanaryProfile::LegacyArgmax.validate(mode).is_ok());
            for profile in [
                CanaryProfile::AttentionBaseline,
                CanaryProfile::AttentionWave,
            ] {
                assert_eq!(profile.validate(mode).is_ok(), mode == ArgmaxMode::WaveV11);
            }
        }
    }

    fn submission_options(profile: CanaryProfile) -> Options {
        Options {
            source: "source".into(),
            target_artifact: "target".into(),
            target_head_artifact: "head".into(),
            argmax_artifact: "argmax".into(),
            worker: "worker".into(),
            worker_sha256: "a".repeat(64),
            device: 1,
            reference: "reference".into(),
            prefix_reference: "prefix".into(),
            host_timing_output: "timing".into(),
            mode: ArgmaxMode::WaveV11,
            outputs: 8,
            runtime: super::super::tp_worker::RuntimeOptions {
                cache_admission: true,
                operational: true,
                rollover: true,
                ordered_batches: profile.ordered_batches(),
                ..super::super::tp_worker::RuntimeOptions::default()
            },
        }
    }

    fn runtime_bits(options: &Options) -> [bool; 7] {
        [
            options.runtime.cache_admission,
            options.runtime.operational,
            options.runtime.rollover,
            options.runtime.sequences,
            options.runtime.profile,
            options.runtime.shared_full_currentness,
            options.runtime.ordered_batches,
        ]
    }

    fn mismatch(options: &mut Options, mutation: usize) {
        match mutation {
            0 => options.runtime.cache_admission = false,
            1 => options.runtime.operational = false,
            2 => options.runtime.rollover = false,
            3 => options.runtime.sequences = true,
            4 => options.runtime.profile = true,
            5 => options.runtime.shared_full_currentness = true,
            6 => options.runtime.ordered_batches = !options.runtime.ordered_batches,
            7 => options.mode = ArgmaxMode::Serial,
            8 => options.outputs = 0,
            9 => options.outputs = 9,
            _ => unreachable!(),
        }
    }

    #[test]
    fn submission_profiles_have_closed_schemas_and_truthful_ordered_modes() {
        for (record, expected) in [
            (RecordKind::Setup, "FerricWaveArgmaxSubmissionCanarySetupV1"),
            (RecordKind::Prefill, "FerricWaveArgmaxSubmissionCanaryPrefillV1"),
            (RecordKind::Observation, "FerricWaveArgmaxSubmissionCanaryObservationV1"),
            (RecordKind::Closed, "FerricWaveArgmaxSubmissionCanaryClosedV1"),
        ] {
            for profile in [CanaryProfile::SubmissionSynchronous, CanaryProfile::SubmissionOrdered] {
                assert_eq!(profile.schema(record), expected);
                assert_eq!(profile.attention(), "wave");
                assert!(profile.wave_attention());
                assert_eq!(profile.ordered_batches(), profile == CanaryProfile::SubmissionOrdered);
                assert_eq!(profile.error_prefix(), "Wave argmax submission canary rejected");
            }
            assert_ne!(CanaryProfile::LegacyArgmax.schema(record), expected);
            assert_ne!(CanaryProfile::AttentionWave.schema(record), expected);
        }
        for profile in [CanaryProfile::LegacyArgmax, CanaryProfile::AttentionBaseline, CanaryProfile::AttentionWave] {
            assert!(!profile.ordered_batches());
            assert_eq!(profile.wave_attention(), profile == CanaryProfile::AttentionWave);
        }
    }

    #[test]
    fn submission_profiles_reject_mismatched_runtime_without_normalizing_options() {
        for profile in [CanaryProfile::SubmissionSynchronous, CanaryProfile::SubmissionOrdered] {
            for outputs in [8, 128] {
                let mut options = submission_options(profile);
                options.outputs = outputs;
                assert!(profile.validate_options(&options).is_ok());
            }
            for mutation in 0..10 {
                let mut options = submission_options(profile);
                mismatch(&mut options, mutation);
                let before = (runtime_bits(&options), options.mode, options.outputs);
                assert!(profile.validate_options(&options).is_err());
                assert_eq!((runtime_bits(&options), options.mode, options.outputs), before);
            }
        }
    }

    #[test]
    fn submission_profile_mismatches_create_no_sidecar_before_execution() {
        let root = std::env::temp_dir().join(format!("ferric-submission-profile-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        for profile in [CanaryProfile::SubmissionSynchronous, CanaryProfile::SubmissionOrdered] {
            for mutation in 0..10 {
                let mut options = submission_options(profile);
                options.reference = root.join("missing-reference");
                options.worker = root.join("missing-worker");
                let sidecar = root.join(format!("{}-{mutation}.json", profile.ordered_batches()));
                options.host_timing_output = sidecar.clone();
                mismatch(&mut options, mutation);
                assert_eq!(execute(Ok(options), profile), std::process::ExitCode::FAILURE);
                assert!(!sidecar.exists());
            }
        }
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);
        std::fs::remove_dir(root).unwrap();
    }
}
