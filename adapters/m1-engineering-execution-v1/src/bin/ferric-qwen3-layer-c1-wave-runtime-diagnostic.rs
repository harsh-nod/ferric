//! Separate ordered C1/v11 host-wall diagnostic; no benchmark or serving grant.

#![recursion_limit = "256"]

mod layer_c1_wave_live_contract;
mod layer_c1_wave_runtime_diagnostic_contract;
mod tp_host_timing;
mod tp_worker;
mod wave_argmax_live_contract;

use ferric_m1_engineering_execution_v1::host_timing::HostTiming;
use ferric_m1_engineering_execution_v1::tp_artifact::EngineeringTpArtifactV1;
use ferric_m1_engineering_execution_v1::tp_batch_runtime::{
    EngineeringTpBatchRunnerV2, EngineeringTpBatchRuntimeV2,
};
use ferric_m1_engineering_execution_v1::tp_execution::batched::EngineeringTpBatchExecutionV2;
use ferric_m1_engineering_execution_v1::tp_execution::{
    EngineeringTpProjectionModeV3, EngineeringTpRankTransportV1, EngineeringTpReductionModeV3,
};
use ferric_m1_engineering_execution_v1::tp_live_ingress;
use ferric_m1_engineering_execution_v1::tp_model::EngineeringQwenModelV1;
use ferric_m1_engineering_execution_v1::tp_paged::{
    EngineeringTpPagedPoolV1, EngineeringTpPoolScopeV1,
};
use ferric_m1_engineering_execution_v1::tp_scheduler::EngineeringTpSchedulerV1;
use layer_c1_wave_live_contract::LayerProjection;
use layer_c1_wave_runtime_diagnostic_contract::Options;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tp_host_timing::TimingFile;
use tp_worker::Worker;
use wave_argmax_live_contract::{CACHE_TTL, CHUNK, ROWS};

const LIVE_PROFILE: &str = "layer-c1-wave-runtime-diagnostic-v1";
const COUNTER_SCOPE: &str = "cumulative overlapping worker host-wall counters; not GPU timestamps";
const COMMAND_ACCOUNTING: &str = "workload command delta equals one earlier snapshot command plus successful workload IPC round trips; an ordered command carries 1..16 dispatches";

fn emit(value: &Value) -> Result<(), String> {
    let mut output = std::io::stdout().lock();
    serde_json::to_writer(&mut output, value).map_err(|e| e.to_string())?;
    output
        .write_all(b"\n")
        .and_then(|()| output.flush())
        .map_err(|e| e.to_string())
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    bytes
        .iter()
        .flat_map(|byte| {
            [
                char::from(DIGITS[usize::from(byte >> 4)]),
                char::from(DIGITS[usize::from(byte & 15)]),
            ]
        })
        .collect()
}

fn hash_file(path: &Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let before = file.metadata().map_err(|e| e.to_string())?;
    if !before.is_file() || before.len() == 0 || before.len() > 512 * 1024 * 1024 {
        return Err("nonempty bounded executable required".into());
    }
    let mut hash = Sha256::new();
    let mut buffer = vec![0; 65_536];
    let mut length = 0_u64;
    loop {
        let bytes = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if bytes == 0 {
            break;
        }
        length = length
            .checked_add(u64::try_from(bytes).map_err(|_| "executable read length")?)
            .ok_or("executable extent overflow")?;
        if length > before.len() {
            return Err("executable grew during hashing".into());
        }
        hash.update(&buffer[..bytes]);
    }
    let after = file.metadata().map_err(|e| e.to_string())?;
    if length != before.len()
        || after.len() != before.len()
        || before.modified().map_err(|e| e.to_string())?
            != after.modified().map_err(|e| e.to_string())?
    {
        return Err("executable changed during hashing".into());
    }
    Ok(hex(&hash.finalize()))
}

fn artifact_identity(artifact: &EngineeringTpArtifactV1) -> Value {
    json!({
        "artifact_hsaco_id":hex(artifact.hsaco_id().as_bytes()),
        "artifact_manifest_id":hex(artifact.manifest_id().as_bytes()),
        "artifact_handoff_id":hex(artifact.handoff_id().as_bytes()),
    })
}

fn profile_metadata(options: &Options, actual_layer: &str) -> Result<Value, String> {
    options.validate()?;
    if actual_layer != options.base.layer_projection.label() {
        return Err("actual layer projection differs from requested live profile".into());
    }
    let live = &options.base.live;
    let runtime = options.worker_runtime()?;
    Ok(json!({
        "runtime_cache_admission":runtime.cache_admission,
        "runtime_operational":runtime.operational,
        "dispatch_sequences":runtime.sequences,
        "queue_rollover":runtime.rollover,
        "runtime_profiling":runtime.profile,
        "projection":"mfma", "attention":"wave", "argmax_mode":"wave-v11",
        "submission":live.submission.label(),
        "runtime_ordered_batches":runtime.ordered_batches,
        "live_profile":LIVE_PROFILE, "layer_projection":actual_layer,
        "benchmark_qualified":false, "serving_qualified":false, "performance_qualified":false,
    }))
}

fn diagnostic_record(options: &Options, phase: &str, snapshots: &[Value]) -> Result<Value, String> {
    options.validate()?;
    if !matches!(phase, "before_workload" | "after_workload") {
        return Err("unsupported runtime snapshot phase".into());
    }
    Ok(json!({
        "schema":"FerricLayerC1WaveRuntimeDiagnosticV1", "authority":"none",
        "performance_qualified":false, "benchmark_qualified":false, "serving_qualified":false,
        "live_profile":LIVE_PROFILE, "layer_projection":options.base.layer_projection.label(),
        "runtime_profiling":options.worker_runtime()?.profile,
        "phase":phase, "measurement":COUNTER_SCOPE, "command_accounting":COMMAND_ACCOUNTING,
        "snapshots":snapshots,
    }))
}

fn emit_diagnostic(options: &Options, phase: &str, snapshots: &[Value]) -> Result<(), String> {
    let value = diagnostic_record(options, phase, snapshots)?;
    let mut output = std::io::stderr().lock();
    serde_json::to_writer(&mut output, &value).map_err(|e| e.to_string())?;
    output.write_all(b"\n").and_then(|()| output.flush()).map_err(|e| e.to_string())
}

fn run_diagnostic_and_close<G: EngineeringTpBatchRunnerV2>(
    runtime: &mut EngineeringTpBatchRuntimeV2<G>,
    timing: &HostTiming,
    mut snapshot_output: impl FnMut(&str, &[Value]) -> Result<(), String>,
    body: impl FnOnce(&mut EngineeringTpBatchRuntimeV2<G>) -> Result<(), String>,
) -> (Result<(), String>, Result<(), String>) {
    let result = (|| {
        {
            let _scope = timing.scope("runtime_snapshot_before");
            snapshot_output("before_workload", &runtime.runtime_diagnostic_snapshot()?)?;
        }
        body(runtime)?;
        {
            let _scope = timing.scope("runtime_snapshot_after");
            snapshot_output("after_workload", &runtime.runtime_diagnostic_snapshot()?)?;
        }
        Ok(())
    })();
    let _close_scope = timing.scope("runtime_diagnostic_close");
    let close = runtime.close();
    (result, close)
}

fn check_retired<G: EngineeringTpBatchRunnerV2>(
    runtime: &EngineeringTpBatchRuntimeV2<G>,
    pages: u32,
) -> Result<(), String> {
    let stats = runtime.page_stats();
    if runtime.retained_requests() != 0
        || stats.sequences != 0
        || stats.free_pages != pages
        || stats.retained_pages != 0
        || stats.cached_pages != 0
        || stats.quarantined_pages != 0
    {
        return Err("drained no-cache live runtime must retire every request and page".into());
    }
    Ok(())
}

fn run_with_timing(options: &Options, timing: &mut TimingFile) -> Result<(), String> {
    options.validate()?;
    let live = &options.base.live;
    let runtime_options = options.worker_runtime()?;
    let whole = Instant::now();
    let setup_scope = timing.timing.scope("setup");
    let controller_sha256 = hash_file(Path::new("/proc/self/exe"))?;
    if hash_file(&live.worker)? != live.worker_sha256 {
        return Err("worker hash differs".into());
    }
    let target_artifact = EngineeringTpArtifactV1::open_batch32(
        &live.target_artifact,
        &ferric_qwen3_tp_batch32_kernels_device_v5::compiler_expectation_roster_v5(),
        true,
    )
    .map_err(|e| e.to_string())?;
    let head_artifact = EngineeringTpArtifactV1::open_fp32_head32(
        &live.target_head_artifact,
        &ferric_qwen3_tp_fp32_head32_kernels_device_v8::compiler_expectation_roster_v8(),
    )
    .map_err(|e| e.to_string())?;
    let argmax_artifact = EngineeringTpArtifactV1::open_fp32_argmax32_v11(&live.argmax_artifact)
        .map_err(|e| e.to_string())?;
    let model = EngineeringQwenModelV1::open(&live.source)?;
    let mut session = [0_u8; 32];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut session))
        .map_err(|e| e.to_string())?;
    let limits = live.limits()?;
    let kv_pool_payload_bytes = limits
        .target_kv_payload_bytes()
        .map_err(|e| format!("KV payload: {e:?}"))?;
    let pool = EngineeringTpPagedPoolV1::new_wide32(
        EngineeringTpPoolScopeV1 {
            model: *model.bundle_id().as_bytes(),
            session,
        },
        limits,
    )
    .map_err(|e| format!("pool: {e:?}"))?;
    let scheduler = EngineeringTpSchedulerV1::new_wide32(
        model.config().vocabulary_size,
        live.context,
        ROWS,
        CHUNK,
    )
    .map_err(|e| format!("scheduler: {e:?}"))?;
    let mut worker = Worker::spawn_with_timing(
        &live.worker,
        live.device,
        &target_artifact,
        runtime_options,
        timing.timing.clone(),
        0,
    )?;
    let worker_pid = worker.pid();
    let admitted = (|| {
        if hash_file(&PathBuf::from(format!("/proc/{worker_pid}/exe")))? != live.worker_sha256 {
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
    let configured = (|| {
        driver.configure_output_head_pruning(true)?;
        driver.configure_reduction(EngineeringTpReductionModeV3::DeviceTp1V3)?;
        driver.configure_projection(
            EngineeringTpProjectionModeV3::Mfma,
            model.target_weights(),
            model.layout(),
        )?;
        driver.configure_wave_attention(true)?;
        driver.configure_head_precision_v8(true)?;
        match options.base.layer_projection {
            LayerProjection::Mfma => {
                driver.configure_ordered_wave_attention_fp32_argmax_v11(&argmax_artifact)?;
            }
            LayerProjection::C1Wave => {
                driver.configure_ordered_c1_wave_layers_fp32_argmax_v11(&argmax_artifact)?;
            }
        }
        driver.configure_host_timing(timing.timing.clone())?;
        if driver.expected_dispatch_counts(0) != [613]
            || driver.expected_dispatch_counts(1) != [616]
            || driver.fp32_argmax_mode() != "wave-v11"
        {
            return Err("live selector or packet contract differs".into());
        }
        profile_metadata(options, driver.layer_projection_mode())
    })();
    let performance_profile = match configured {
        Ok(profile) => profile,
        Err(error) => {
            let close = driver.close();
            return Err(format!("{error}; driver close: {close:?}"));
        }
    };
    let layer_projection = driver.layer_projection_mode();
    let transposed_weight_bytes = driver.transposed_weight_bytes();
    let fp32_workspace_bytes = driver.fp32_head_workspace_bytes();
    let argmax_mode = driver.fp32_argmax_mode();
    let mut runtime =
        EngineeringTpBatchRuntimeV2::new_wide32(driver, pool, scheduler, ROWS, false)?;
    drop(setup_scope);
    let setup = json!({
        "schema":"FerricQwen3TpBatchSetupV2", "authority":"none", "performance_qualified":false,
        "serving_qualified":false, "benchmark_qualified":false,
        "runtime_profiling":runtime_options.profile,
        "live_profile":LIVE_PROFILE, "layer_projection":layer_projection,
        "model":"Qwen/Qwen3-8B", "dtype":"BF16", "target":"gfx950:xnack-",
        "tensor_parallel":1, "device_unique_ids":[live.device], "worker_pids":[worker_pid],
        "worker_sha256":live.worker_sha256, "running_worker_sha256":[live.worker_sha256],
        "controller_sha256":controller_sha256, "model_bundle_id":hex(model.bundle_id().as_bytes()),
        "target_model_id":hex(model.config().model_id.as_bytes()), "session_id":hex(&session),
        "artifact_hsaco_id":hex(target_artifact.hsaco_id().as_bytes()),
        "artifact_manifest_id":hex(target_artifact.manifest_id().as_bytes()),
        "artifact_handoff_id":hex(target_artifact.handoff_id().as_bytes()),
        "fp32_head_artifact":artifact_identity(&head_artifact), "argmax_artifact":artifact_identity(&argmax_artifact),
        "head_precision":"fp32-v8", "argmax_mode":argmax_mode, "submission":live.submission.label(),
        "runtime_ordered_batches":live.runtime.ordered_batches, "performance_profile":performance_profile,
        "kernel_profile":"v5-mfma32", "kernel_row_capacity":ROWS, "batch_tokens":ROWS,
        "prefill_chunk":CHUNK, "page_tokens":16, "physical_pages":live.pages,
        "context_tokens":live.context, "cache_ttl_ticks":CACHE_TTL, "prefix_cache":false,
        "kv_pool_profile":"legacy-v5", "kv_pool_max_physical_pages":512,
        "kv_pool_payload_bytes":kv_pool_payload_bytes,
        "target_payload_bytes":model.target_weights().len(), "target_transposed_bytes":transposed_weight_bytes,
        "fp32_head_workspace_bytes":fp32_workspace_bytes, "max_batches":live.max_batches,
        "output_head_pruning":true, "collective":"device-tp1-v3", "prefill":"true_multirow_chunked",
        "attention":"paged_causal_gqa", "cache":"disabled", "setup_seconds":whole.elapsed().as_secs_f64(),
        "arrival_policy":"live monotonic ingress timestamps; includes pending wait",
        "live_protocol":"FerricQwen3TpLiveCommandV1/FerricQwen3TpLiveEventV1",
        "numerical_status":"Runtime diagnostic only; independently replay exact token IDs and bytes; no benchmark or serving admission",
        "runtime_diagnostic_scope":COUNTER_SCOPE, "command_accounting":COMMAND_ACCOUNTING,
    });
    timing.setup = Some(setup.clone());
    let (result, close) = run_diagnostic_and_close(
        &mut runtime,
        &timing.timing,
        |phase, snapshots| emit_diagnostic(options, phase, snapshots),
        |runtime| {
            emit(&setup)?;
            let _workload_scope = timing.timing.scope("workload");
            tp_live_ingress::run(
                runtime,
                &model,
                live.context,
                live.pages,
                live.max_batches,
                &timing.timing,
                emit,
            )?;
            check_retired(runtime, live.pages)
        },
    );
    let closed = json!({
        "schema":"FerricQwen3TpBatchClosedV2", "authority":"none", "performance_qualified":false,
        "live_profile":LIVE_PROFILE, "layer_projection":layer_projection, "submission":live.submission.label(),
        "runtime_profiling":runtime_options.profile, "benchmark_qualified":false, "serving_qualified":false,
        "runtime_diagnostics_completed":result.is_ok(),
        "worker_pids":[worker_pid], "all_workers_exited":close.is_ok(), "execution_completed":result.is_ok(),
        "rank_dispatch_counts":runtime.dispatch_counts(), "whole_seconds":whole.elapsed().as_secs_f64(),
    });
    timing.closed = Some(closed.clone());
    let emitted = emit(&closed);
    match (result, close, emitted) {
        (Ok(()), Ok(()), Ok(())) => Ok(()),
        (result, close, emitted) => Err(format!(
            "live execution: {result:?}; close: {close:?}; closed record: {emitted:?}"
        )),
    }
}

fn run(options: &Options) -> Result<(), String> {
    options.validate()?;
    let mut timing = TimingFile::create(options.base.live.host_timing.as_deref())?;
    let result = run_with_timing(options, &mut timing);
    let sidecar = timing.finish(&result);
    match (result, sidecar) {
        (Ok(()), Ok(())) => Ok(()),
        (result, sidecar) => Err(format!("live run: {result:?}; timing: {sidecar:?}")),
    }
}

fn main() -> std::process::ExitCode {
    match Options::parse(std::env::args().skip(1)).and_then(|options| run(&options)) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Layer C1 Wave runtime diagnostic rejected: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod diagnostic_tests {
    use super::*;
    use ferric_m1_engineering_execution_v1::tp_execution::batched::EngineeringTpBatchOutputV2;
    use ferric_m1_engineering_execution_v1::tp_paged::EngineeringTpPreparedBatchV1;
    use std::cell::RefCell;
    use std::rc::Rc;

    type Events = Rc<RefCell<Vec<String>>>;

    struct DiagnosticRunner {
        events: Events,
        timing: HostTiming,
        snapshots: usize,
        fail_snapshot: Option<usize>,
        fail_close: bool,
    }

    impl EngineeringTpBatchRunnerV2 for DiagnosticRunner {
        fn runtime_diagnostic_snapshot(&mut self) -> Result<Vec<Value>, String> {
            let _span = self.timing.span("snapshot_fixture", Some(0));
            let ordinal = self.snapshots;
            self.snapshots += 1;
            self.events.borrow_mut().push(format!("snapshot{ordinal}"));
            if self.fail_snapshot == Some(ordinal) {
                return Err("injected snapshot failure".into());
            }
            Ok(vec![json!({"ordinal":ordinal})])
        }
        fn row_capacity(&self) -> usize {
            ROWS
        }
        fn execute_batch(
            &mut self,
            _: &EngineeringTpPreparedBatchV1,
            _: &[usize],
        ) -> Result<EngineeringTpBatchOutputV2, String> {
            Err("CPU lifecycle fixture must not execute GPU work".into())
        }
        fn dispatch_counts(&self) -> Vec<u64> {
            vec![0]
        }
        fn close(&mut self) -> Result<(), String> {
            let _span = self.timing.span("close_fixture", Some(0));
            self.events.borrow_mut().push("close".into());
            if self.fail_close {
                Err("injected close failure".into())
            } else {
                Ok(())
            }
        }
    }

    fn runtime(
        fail_snapshot: Option<usize>,
        fail_close: bool,
    ) -> (EngineeringTpBatchRuntimeV2<DiagnosticRunner>, Events, HostTiming) {
        let options = layer_c1_wave_runtime_diagnostic_contract::fixture(LayerProjection::C1Wave);
        let pool = EngineeringTpPagedPoolV1::new_wide32(
            EngineeringTpPoolScopeV1 { model: [1; 32], session: [2; 32] },
            options.base.live.limits().unwrap(),
        ).unwrap();
        let scheduler = EngineeringTpSchedulerV1::new_wide32(100, 8192, ROWS, CHUNK).unwrap();
        let events = Rc::new(RefCell::new(Vec::new()));
        let timing = HostTiming::enabled();
        let runtime = EngineeringTpBatchRuntimeV2::new_wide32(
            DiagnosticRunner {
                events: events.clone(), timing: timing.clone(), snapshots: 0,
                fail_snapshot, fail_close,
            },
            pool, scheduler, ROWS, false,
        ).unwrap();
        (runtime, events, timing)
    }

    #[test]
    fn metadata_is_direct_profiled_ordered_and_layer_specific() {
        for layer in [LayerProjection::Mfma, LayerProjection::C1Wave] {
            let options = layer_c1_wave_runtime_diagnostic_contract::fixture(layer);
            let profile = profile_metadata(&options, layer.label()).unwrap();
            assert_eq!(profile, json!({
                "runtime_cache_admission":true, "runtime_operational":true,
                "dispatch_sequences":false, "queue_rollover":true, "runtime_profiling":true,
                "projection":"mfma", "attention":"wave", "argmax_mode":"wave-v11",
                "submission":"ordered", "runtime_ordered_batches":true,
                "live_profile":LIVE_PROFILE, "layer_projection":layer.label(),
                "benchmark_qualified":false, "serving_qualified":false, "performance_qualified":false,
            }));
            for actual in ["auto", "wave", "", if layer == LayerProjection::Mfma { "c1-wave" } else { "mfma" }] {
                assert!(profile_metadata(&options, actual).is_err());
            }
        }
    }

    #[test]
    fn diagnostic_envelope_preserves_raw_snapshots_and_nonexclusive_scope() {
        let snapshots = vec![json!({"schema":"FerricRuntimeDiagnosticSnapshotV1", "ordinal":0,
            "process_id":123, "device_unique_id":456, "rank":0, "counters":{"commands":11}})];
        for layer in [LayerProjection::Mfma, LayerProjection::C1Wave] {
            let options = layer_c1_wave_runtime_diagnostic_contract::fixture(layer);
            let record = diagnostic_record(&options, "before_workload", &snapshots).unwrap();
            assert_eq!(record, json!({
                "schema":"FerricLayerC1WaveRuntimeDiagnosticV1", "authority":"none",
                "performance_qualified":false, "benchmark_qualified":false, "serving_qualified":false,
                "live_profile":LIVE_PROFILE, "layer_projection":layer.label(), "runtime_profiling":true,
                "phase":"before_workload", "measurement":COUNTER_SCOPE,
                "command_accounting":COMMAND_ACCOUNTING, "snapshots":snapshots,
            }));
            assert!(COUNTER_SCOPE.contains("overlapping"));
            assert!(COMMAND_ACCOUNTING.contains("one earlier snapshot command"));
            assert!(COMMAND_ACCOUNTING.contains("1..16 dispatches"));
            assert!(diagnostic_record(&options, "during_workload", &snapshots).is_err());
        }
    }

    #[test]
    fn invalid_direct_diagnostic_options_create_no_timing_file() {
        let path = std::env::temp_dir().join(format!("ferric-layer-diagnostic-reject-{}.json", std::process::id()));
        assert!(!path.exists());
        for layer in [LayerProjection::Mfma, LayerProjection::C1Wave] {
            for mutation in 0..12 {
                let mut options = layer_c1_wave_runtime_diagnostic_contract::fixture(layer);
                options.base.live.host_timing = Some(path.clone());
                match mutation {
                    0 => options.base.live.runtime.cache_admission = false,
                    1 => options.base.live.runtime.operational = false,
                    2 => options.base.live.runtime.rollover = false,
                    3 => options.base.live.runtime.ordered_batches = false,
                    4 => options.base.live.runtime.sequences = true,
                    5 => options.base.live.runtime.profile = true,
                    6 => options.base.live.runtime.shared_full_currentness = true,
                    7 => options.base.live.host_timing = None,
                    8 => options.base.live.context = 256,
                    9 => options.base.live.pages = 16,
                    10 => options.base.live.max_batches = 0,
                    11 => options.base.live.device = 0,
                    _ => unreachable!(),
                }
                assert!(run(&options).is_err(), "mutation {mutation}");
                assert!(!path.exists());
            }
        }
    }

    #[test]
    fn snapshot_order_brackets_ready_body_retirement_and_restores_host_phase() {
        let (mut runtime, events, timing) = runtime(None, false);
        let (result, close) = run_diagnostic_and_close(&mut runtime, &timing,
            |phase, _| { events.borrow_mut().push(phase.into()); Ok(()) },
            |runtime| {
                events.borrow_mut().push("setup_ready".into());
                let _scope = timing.scope("workload");
                let _span = timing.span("body_fixture", None);
                events.borrow_mut().push("body".into());
                check_retired(runtime, 512)?;
                events.borrow_mut().push("retired".into());
                Ok(())
            });
        assert!(result.is_ok() && close.is_ok());
        assert_eq!(*events.borrow(), ["snapshot0", "before_workload", "setup_ready", "body", "retired",
            "snapshot1", "after_workload", "close"]);
        {
            let _span = timing.span("restored_fixture", None);
        }
        let recorded = timing.snapshot();
        assert_eq!(recorded["incomplete"], false);
        assert_eq!(recorded["active_records"], 0);
        let records = recorded["records"].as_array().unwrap();
        for (label, phase) in [("snapshot_fixture", "runtime_snapshot_before"),
            ("snapshot_fixture", "runtime_snapshot_after"), ("body_fixture", "workload"),
            ("close_fixture", "runtime_diagnostic_close"), ("restored_fixture", "controller")] {
            assert_eq!(records.iter().filter(|record| record["label"] == label && record["phase"] == phase)
                .map(|record| record["count"].as_u64().unwrap()).sum::<u64>(), 1);
        }
        assert!(runtime.runtime_diagnostic_snapshot().is_err());
    }

    #[test]
    fn all_snapshot_output_body_and_close_failures_attempt_close_once() {
        for failure in 0..7 {
            let snapshot_failure = match failure { 1 => Some(0), 4 => Some(1), _ => None };
            let (mut runtime, events, timing) = runtime(snapshot_failure, failure == 6);
            let (result, close) = run_diagnostic_and_close(&mut runtime, &timing,
                |phase, _| {
                    events.borrow_mut().push(phase.into());
                    if (failure == 2 && phase == "before_workload") || (failure == 5 && phase == "after_workload") {
                        Err("injected snapshot output failure".into())
                    } else { Ok(()) }
                },
                |_| {
                    events.borrow_mut().push("body".into());
                    if failure == 3 { Err("injected ingress, output or retirement failure".into()) }
                    else { Ok(()) }
                });
            assert_eq!(result.is_err(), (1..=5).contains(&failure));
            assert_eq!(close.is_err(), failure == 6);
            let recorded = events.borrow();
            assert_eq!(recorded.iter().filter(|event| *event == "close").count(), 1);
            assert_eq!(recorded.last().map(String::as_str), Some("close"));
            if matches!(failure, 1 | 2) {
                assert!(!recorded.iter().any(|event| event == "body"));
            }
            if matches!(failure, 1..=3) {
                assert!(!recorded.iter().any(|event| event == "snapshot1"));
            }
            assert!(runtime.runtime_diagnostic_snapshot().is_err());
            assert_eq!(timing.snapshot()["incomplete"], false);
        }
    }

    #[test]
    fn diagnostic_after_snapshot_requires_complete_no_cache_retirement() {
        use ferric_m1_engineering_execution_v1::tp_scheduler::TpRequestAdmissionV1;
        let (mut runtime, events, timing) = runtime(None, false);
        let request = runtime.admit(TpRequestAdmissionV1 {
            prompt_tokens: vec![7; 128], max_new_tokens: 128, cached_prefix_tokens: 0,
            arrival_tick: 0, arrival_ns: 0,
        }, 0, 0).unwrap().request;
        runtime.cancel(request, 1).unwrap();
        let (result, close) = run_diagnostic_and_close(&mut runtime, &timing, |_, _| Ok(()),
            |runtime| check_retired(runtime, 512));
        assert!(result.is_err() && close.is_ok());
        assert_eq!(*events.borrow(), ["snapshot0", "close"]);
    }
}
