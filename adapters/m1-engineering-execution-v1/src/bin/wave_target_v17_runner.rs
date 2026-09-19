//! Same-image target attention/normalization ablations; no benchmark or serving grant.

use super::{tp_host_timing, tp_worker, wave_argmax_live_contract, wave_target_v17_live_contract};
use ferric_m1_engineering_execution_v1::host_timing::HostTiming;

use ferric_m1_engineering_execution_v1::tp_artifact::EngineeringTpArtifactV1;
use ferric_m1_engineering_execution_v1::tp_batch_runtime::{
    EngineeringTpBatchRunnerV2, EngineeringTpBatchRuntimeV2,
};
use ferric_m1_engineering_execution_v1::tp_execution::batched::{
    EngineeringTpBatchExecutionV2, EngineeringTpWaveTargetArtifactsV17,
};
use ferric_m1_engineering_execution_v1::tp_execution::{
    EngineeringTpProjectionModeV3, EngineeringTpRankTransportV1, EngineeringTpReductionModeV3,
};
use ferric_m1_engineering_execution_v1::tp_live_ingress;
use ferric_m1_engineering_execution_v1::tp_model::EngineeringQwenModelV1;
use ferric_m1_engineering_execution_v1::tp_paged::{
    EngineeringTpPagedPoolV1, EngineeringTpPoolScopeV1,
};
use ferric_m1_engineering_execution_v1::tp_scheduler::EngineeringTpSchedulerV1;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tp_host_timing::TimingFile;
use tp_worker::{RuntimeOptions, Worker};
use wave_argmax_live_contract::{CACHE_TTL, CHUNK, ROWS};
use wave_target_v17_live_contract::Options;

const LIVE_PROFILE: &str = "wave-target-v17-live-v1";
const DIAGNOSTIC_PROFILE: &str = "wave-target-v17-runtime-diagnostic-v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Variant<'a> {
    V17,
    #[allow(dead_code)] // Selected only by the separate diagnostic executable.
    RuntimeDiagnostic,
    #[allow(dead_code)] // Selected only by the separate V19 executable.
    KvCopyV19 {
        artifact: &'a Path,
        enabled: bool,
    },
    #[allow(dead_code)] // Selected only by the separate V22 executable.
    PackedC1V22 {
        enabled: bool,
    },
    #[allow(dead_code)] // Selected only by the separate V25 executable.
    SplitAttentionV25 {
        artifact: &'a Path,
        enabled: bool,
    },
}

impl Variant<'_> {
    fn validate(self, options: &Options) -> Result<(), String> {
        options.validate()?;
        if self == Self::RuntimeDiagnostic
            && (options.mode != wave_target_v17_live_contract::Mode::Combined
                || options.live.host_timing.is_none())
        {
            return Err("V17 runtime diagnostic requires combined mode and host timing".into());
        }
        if let Self::KvCopyV19 { artifact, .. } = self
            && (options.mode != wave_target_v17_live_contract::Mode::Combined
                || artifact.as_os_str().is_empty())
        {
            return Err("V19 requires combined V17 and an explicit copy image".into());
        }
        if matches!(self, Self::PackedC1V22 { .. })
            && options.mode != wave_target_v17_live_contract::Mode::Combined
        {
            return Err("V22 packet packing requires combined V17".into());
        }
        if let Self::SplitAttentionV25 { artifact, .. } = self
            && (options.mode != wave_target_v17_live_contract::Mode::Combined
                || artifact.as_os_str().is_empty()
                || options.live.host_timing.is_some())
        {
            return Err(
                "V25 requires combined V17, its explicit V21 image and no diagnostic timing".into(),
            );
        }
        Ok(())
    }

    fn live_profile(self) -> &'static str {
        match self {
            Self::V17 => LIVE_PROFILE,
            Self::RuntimeDiagnostic => DIAGNOSTIC_PROFILE,
            Self::KvCopyV19 { .. } => "c1-kv-copy-v19-live-v1",
            Self::PackedC1V22 { .. } => "c1-packed-v22-live-v1",
            Self::SplitAttentionV25 { .. } => "c1-split-attention-v25-live-v1",
        }
    }

    fn worker_runtime(self, options: &Options) -> Result<RuntimeOptions, String> {
        self.validate(options)?;
        let mut runtime = options.live.runtime;
        runtime.profile = self == Self::RuntimeDiagnostic;
        Ok(runtime)
    }

    fn kv_append_mode(self) -> &'static str {
        if matches!(self, Self::KvCopyV19 { enabled: true, .. }) {
            "parallel-c1-v19"
        } else {
            "baseline"
        }
    }

    fn c1_packet_mode(self) -> &'static str {
        if matches!(self, Self::PackedC1V22 { enabled: true }) {
            "packed16-v22"
        } else {
            "baseline"
        }
    }

    fn annotate_packet_mode(self, value: &mut Value, actual: &str) -> Result<(), String> {
        if let Self::PackedC1V22 { .. } = self {
            if actual != self.c1_packet_mode() {
                return Err("actual V22 packet policy differs from requested mode".into());
            }
            value["requested_c1_packet_mode"] = json!(self.c1_packet_mode());
            value["c1_packet_mode"] = json!(actual);
        }
        Ok(())
    }

    fn split_attention_mode(self) -> &'static str {
        if matches!(self, Self::SplitAttentionV25 { enabled: true, .. }) {
            "split8-v21"
        } else {
            "baseline"
        }
    }

    fn annotate_split_attention(
        self,
        value: &mut Value,
        image: Option<&EngineeringTpArtifactV1>,
        actual: &str,
        bytes: u64,
    ) -> Result<(), String> {
        if let Self::SplitAttentionV25 { artifact, .. } = self {
            if actual != self.split_attention_mode() || bytes != 133_120 {
                return Err("actual V25 attention policy or workspace differs".into());
            }
            let image = image.ok_or("V25 image missing from controller receipt")?;
            value["requested_split_attention_mode"] = json!(self.split_attention_mode());
            value["split_attention_mode"] = json!(actual);
            value["split_attention_artifact_path"] = json!(artifact);
            value["split_attention_artifact"] = artifact_identity(image);
            value["split_attention_workspace_bytes"] = json!(bytes);
            value["split_attention_policy"] = json!({"physical_rows":1, "actual_context_min":128, "actual_context_max":256,
                "partitions":8, "fallback":"query-hoist-v14", "fallback_packets_no_head":613, "fallback_packets_with_head":616,
                "split_packets_no_head":649, "split_packets_with_head":652});
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
    json!({"artifact_hsaco_id":hex(artifact.hsaco_id().as_bytes()),
        "artifact_manifest_id":hex(artifact.manifest_id().as_bytes()),
        "artifact_handoff_id":hex(artifact.handoff_id().as_bytes())})
}

fn profile_metadata(
    options: &Options,
    layer: &str,
    attention: &str,
    rmsnorm: &str,
) -> Result<Value, String> {
    options.validate()?;
    if layer != "c1-wave"
        || attention != options.mode.attention()
        || rmsnorm != options.mode.rmsnorm()
    {
        return Err("actual execution policy differs from requested wave target profile".into());
    }
    let live = &options.live;
    Ok(json!({
        "runtime_cache_admission":live.runtime.cache_admission, "runtime_operational":live.runtime.operational,
        "dispatch_sequences":live.runtime.sequences, "queue_rollover":live.runtime.rollover,
        "runtime_profiling":live.runtime.profile, "projection":"mfma", "attention":attention,
        "argmax_mode":"wave-v11", "submission":live.submission.label(),
        "runtime_ordered_batches":live.runtime.ordered_batches, "live_profile":LIVE_PROFILE,
        "layer_projection":layer, "wave_target_mode":options.mode.label(), "rmsnorm_mode":rmsnorm,
        "attention_artifact_path":options.attention_artifact, "rmsnorm_artifact_path":options.rmsnorm_artifact,
        "benchmark_admitted":false, "serving_admitted":false,
    }))
}

fn run_and_close<G: EngineeringTpBatchRunnerV2>(
    runtime: &mut EngineeringTpBatchRuntimeV2<G>,
    body: impl FnOnce(&mut EngineeringTpBatchRuntimeV2<G>) -> Result<(), String>,
) -> (Result<(), String>, Result<(), String>) {
    let result = body(runtime);
    let close = runtime.close();
    (result, close)
}

fn checked_snapshot(
    snapshots: &[Value],
    previous: Option<&Value>,
    device: u64,
    worker_pid: u32,
    dispatches: &[u64],
) -> Result<Value, String> {
    let [snapshot] = snapshots else {
        return Err("TP1 runtime diagnostic requires one complete rank snapshot".into());
    };
    if snapshot["schema"] != "FerricRuntimeDiagnosticSnapshotV1"
        || snapshot["authority"] != "none"
        || snapshot["performance_qualified"] != false
        || snapshot["rank"].as_u64() != Some(0)
        || snapshot["process_id"].as_u64() != Some(u64::from(worker_pid))
        || snapshot["device_unique_id"].as_u64() != Some(device)
        || snapshot["ordinal"].as_u64() != Some(u64::from(previous.is_some()))
    {
        return Err("runtime diagnostic snapshot identity differs".into());
    }
    let counters: fe2o3_kfd::engineering_wire::PerformanceCountersV1 =
        serde_json::from_value(snapshot["counters"].clone()).map_err(|error| error.to_string())?;
    if dispatches != [counters.dispatches] {
        return Err("runtime diagnostic snapshot dispatch count differs".into());
    }
    let mut delta = serde_json::Map::new();
    if let Some(previous) = previous {
        let current = snapshot["counters"]
            .as_object()
            .ok_or("runtime counter object")?;
        let previous = previous["counters"]
            .as_object()
            .ok_or("prior runtime counter object")?;
        if current.len() != previous.len() {
            return Err("runtime diagnostic counter roster differs".into());
        }
        for (key, value) in current {
            let value = value.as_u64().ok_or("runtime counter integer")?;
            let old = previous
                .get(key)
                .and_then(Value::as_u64)
                .ok_or("prior runtime counter integer")?;
            delta.insert(
                key.clone(),
                json!(
                    value
                        .checked_sub(old)
                        .ok_or("runtime diagnostic counters regressed")?
                ),
            );
        }
    }
    Ok(Value::Object(delta))
}

fn diagnostic_record(phase: &str, snapshots: &[Value], delta: &Value) -> Value {
    json!({
        "schema":"FerricWaveTargetV17RuntimeDiagnosticV1", "authority":"none",
        "performance_qualified":false, "benchmark_admitted":false, "serving_admitted":false,
        "live_profile":DIAGNOSTIC_PROFILE, "wave_target_mode":"combined", "runtime_profiling":true,
        "phase":phase, "measurement":"cumulative overlapping worker host-wall counters; not GPU timestamps",
        "command_accounting":"delta includes the earlier snapshot command; the final snapshot excludes its own command",
        "workload_scope":"all live requests between snapshots, including any warmup; not a benchmark measurement",
        "dispatch_prepare_scope":"pure preparation only; excludes aggregate fences, payload copy and staging",
        "snapshots":snapshots, "counter_delta":delta,
    })
}

fn emit_diagnostic(value: &Value) -> Result<(), String> {
    let worker_pid = value["snapshots"][0]["process_id"]
        .as_u64()
        .and_then(|pid| u32::try_from(pid).ok())
        .ok_or("runtime diagnostic worker PID")?;
    let mut value = value.clone();
    value["placement"] = json!({
        "scope":"read-only Linux thread-group leader observations; placement is not changed",
        "controller":read_placement(std::process::id())?,
        "worker":read_placement(worker_pid)?,
    });
    let mut output = std::io::stderr().lock();
    serde_json::to_writer(&mut output, &value).map_err(|error| error.to_string())?;
    output
        .write_all(b"\n")
        .and_then(|()| output.flush())
        .map_err(|error| error.to_string())
}

fn parse_placement(pid: u32, stat: &str, status: &str) -> Result<Value, String> {
    let open = stat.find('(').ok_or("process stat name")?;
    let close = stat
        .rfind(')')
        .filter(|close| *close > open)
        .ok_or("process stat name end")?;
    if stat[..open]
        .trim()
        .parse::<u32>()
        .map_err(|error| error.to_string())?
        != pid
    {
        return Err("process placement PID differs".into());
    }
    let fields: Vec<_> = stat[close + 1..].split_whitespace().collect();
    let integer = |index: usize| -> Result<i64, String> {
        fields
            .get(index)
            .ok_or("process stat extent")?
            .parse::<i64>()
            .map_err(|error| error.to_string())
    };
    let affinity: Vec<_> = status
        .lines()
        .filter_map(|line| line.strip_prefix("Cpus_allowed_list:"))
        .collect();
    let [affinity] = affinity.as_slice() else {
        return Err("process CPU affinity observation is absent or repeated".into());
    };
    let affinity = affinity.trim();
    if affinity.is_empty()
        || !affinity
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b',' | b'-'))
    {
        return Err("process CPU affinity observation is malformed".into());
    }
    let priority = integer(15)?;
    let nice = integer(16)?;
    let start_time_ticks = integer(19)?;
    let processor = integer(36)?;
    if !(-20..=19).contains(&nice) || start_time_ticks < 0 || processor < 0 {
        return Err("process scheduling observation is malformed".into());
    }
    Ok(
        json!({"process_id":pid, "start_time_ticks":start_time_ticks,
        "cpus_allowed_list":affinity, "nice":nice, "priority":priority,
        "last_processor":processor}),
    )
}

fn read_placement(pid: u32) -> Result<Value, String> {
    let read = |name: &str| -> Result<String, String> {
        let mut text = String::new();
        std::fs::File::open(format!("/proc/{pid}/{name}"))
            .map_err(|error| error.to_string())?
            .take(16_385)
            .read_to_string(&mut text)
            .map_err(|error| error.to_string())?;
        if text.is_empty() || text.len() > 16_384 {
            return Err("bounded process placement observation required".into());
        }
        Ok(text)
    };
    let stat = read("stat")?;
    let status = read("status")?;
    let before = parse_placement(pid, &stat, &status)?;
    let after = parse_placement(pid, &read("stat")?, &status)?;
    if before["start_time_ticks"] != after["start_time_ticks"] {
        return Err("process identity changed during placement observation".into());
    }
    Ok(before)
}

fn run_diagnostic_and_close<G: EngineeringTpBatchRunnerV2>(
    runtime: &mut EngineeringTpBatchRuntimeV2<G>,
    timing: &HostTiming,
    device: u64,
    worker_pid: u32,
    mut output: impl FnMut(&Value) -> Result<(), String>,
    body: impl FnOnce(&mut EngineeringTpBatchRuntimeV2<G>) -> Result<(), String>,
) -> (Result<(), String>, Result<(), String>) {
    let result = (|| {
        let before = {
            let _scope = timing.scope("runtime_snapshot_before");
            let snapshots = runtime.runtime_diagnostic_snapshot()?;
            let delta = checked_snapshot(
                &snapshots,
                None,
                device,
                worker_pid,
                &runtime.dispatch_counts(),
            )?;
            output(&diagnostic_record("before_workload", &snapshots, &delta))?;
            snapshots[0].clone()
        };
        body(runtime)?;
        {
            let _scope = timing.scope("runtime_snapshot_after");
            let snapshots = runtime.runtime_diagnostic_snapshot()?;
            let delta = checked_snapshot(
                &snapshots,
                Some(&before),
                device,
                worker_pid,
                &runtime.dispatch_counts(),
            )?;
            output(&diagnostic_record("after_workload", &snapshots, &delta))?;
        }
        Ok(())
    })();
    let _scope = timing.scope("runtime_diagnostic_close");
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

fn run_with_timing(
    options: &Options,
    timing: &mut TimingFile,
    variant: Variant<'_>,
) -> Result<(), String> {
    variant.validate(options)?;
    let runtime_options = variant.worker_runtime(options)?;
    let live = &options.live;
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
    let attention_artifact =
        EngineeringTpArtifactV1::open_query_hoist_v14(&options.attention_artifact)
            .map_err(|e| e.to_string())?;
    let rmsnorm_artifact =
        EngineeringTpArtifactV1::open_wave_rmsnorm_v15(&options.rmsnorm_artifact)
            .map_err(|e| e.to_string())?;
    let copy_artifact = match variant {
        Variant::KvCopyV19 { artifact, .. } => Some(
            EngineeringTpArtifactV1::open_c1_kv_copy_v19(artifact)
                .map_err(|error| error.to_string())?,
        ),
        _ => None,
    };
    let split_artifact = match variant {
        Variant::SplitAttentionV25 { artifact, .. } => Some(
            EngineeringTpArtifactV1::open_split_attention_v21(artifact)
                .map_err(|error| error.to_string())?,
        ),
        _ => None,
    };
    let artifacts = EngineeringTpWaveTargetArtifactsV17 {
        argmax: &argmax_artifact,
        attention: &attention_artifact,
        rmsnorm: &rmsnorm_artifact,
    };
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
    let admitted: Result<(), String> = (|| {
        if hash_file(&PathBuf::from(format!("/proc/{worker_pid}/exe")))? != live.worker_sha256 {
            return Err("running worker hash differs".into());
        }
        for artifact in [
            &head_artifact,
            &argmax_artifact,
            &attention_artifact,
            &rmsnorm_artifact,
        ] {
            worker.load_additional_artifact(artifact)?;
        }
        if let Some(artifact) = &copy_artifact {
            worker.load_additional_artifact(artifact)?;
        }
        if let Some(artifact) = &split_artifact {
            worker.load_additional_artifact(artifact)?;
        }
        Ok(())
    })();
    if let Err(error) = admitted {
        let close = worker.close();
        return Err(format!("{error}; worker close: {close:?}"));
    }
    let mut driver = if let Some(copy) = &copy_artifact {
        EngineeringTpBatchExecutionV2::new_wide32_with_c1_kv_copy_v19(
            vec![worker],
            model.config(),
            model.target_weights(),
            model.layout(),
            &pool,
            &artifacts,
            copy,
        )?
    } else if let Some(split) = &split_artifact {
        EngineeringTpBatchExecutionV2::new_wide32_with_c1_split_attention_v25(
            vec![worker],
            model.config(),
            model.target_weights(),
            model.layout(),
            &pool,
            &artifacts,
            split,
        )?
    } else {
        EngineeringTpBatchExecutionV2::new_wide32_with_wave_target_v17(
            vec![worker],
            model.config(),
            model.target_weights(),
            model.layout(),
            &pool,
            &artifacts,
        )?
    };
    let configured: Result<Value, String> = (|| {
        driver.configure_output_head_pruning(true)?;
        driver.configure_reduction(EngineeringTpReductionModeV3::DeviceTp1V3)?;
        driver.configure_projection(
            EngineeringTpProjectionModeV3::Mfma,
            model.target_weights(),
            model.layout(),
        )?;
        driver.configure_wave_attention(true)?;
        driver.configure_head_precision_v8(true)?;
        match (variant, &copy_artifact) {
            (Variant::SplitAttentionV25 { enabled, .. }, None) => {
                driver.configure_ordered_c1_split_attention_v25(
                    &artifacts,
                    split_artifact
                        .as_ref()
                        .ok_or("V25 controller image missing")?,
                    enabled,
                )?;
                if driver.split_attention_mode() != variant.split_attention_mode()
                    || driver.c1_packet_mode() != "baseline"
                    || driver.kv_append_mode() != "baseline"
                {
                    return Err(
                        "actual V25 attention or historical packet/copy route differs".into(),
                    );
                }
            }
            (Variant::KvCopyV19 { enabled, .. }, Some(copy)) => {
                driver.configure_ordered_c1_kv_copy_v19(&artifacts, copy, enabled)?;
                if driver.kv_append_mode() != variant.kv_append_mode() {
                    return Err("actual V19 copy policy differs from requested mode".into());
                }
            }
            (Variant::V17 | Variant::RuntimeDiagnostic | Variant::PackedC1V22 { .. }, None) => {
                driver.configure_ordered_c1_wave_target_v17(&artifacts, options.mode)?;
                if let Variant::PackedC1V22 { enabled } = variant {
                    driver.configure_c1_packet_packing_v22(enabled)?;
                }
            }
            _ => return Err("copy artifact does not match controller variant".into()),
        }
        driver.configure_host_timing(timing.timing.clone())?;
        if driver.expected_dispatch_counts(0) != [613]
            || driver.expected_dispatch_counts(1) != [616]
            || driver.fp32_argmax_mode() != "wave-v11"
        {
            return Err("live selector or packet contract differs".into());
        }
        let mut profile = profile_metadata(
            options,
            driver.layer_projection_mode(),
            driver.attention_mode(),
            driver.rmsnorm_mode(),
        )?;
        variant.annotate_packet_mode(&mut profile, driver.c1_packet_mode())?;
        variant.annotate_split_attention(
            &mut profile,
            split_artifact.as_ref(),
            driver.split_attention_mode(),
            driver.split_attention_workspace_bytes(),
        )?;
        Ok(profile)
    })();
    let mut performance_profile = match configured {
        Ok(profile) => profile,
        Err(error) => {
            let close = driver.close();
            return Err(format!("{error}; driver close: {close:?}"));
        }
    };
    if variant != Variant::V17 {
        performance_profile["live_profile"] = json!(variant.live_profile());
        performance_profile["runtime_profiling"] = json!(runtime_options.profile);
    }
    if let (Variant::KvCopyV19 { artifact, .. }, Some(copy)) = (variant, &copy_artifact) {
        performance_profile["kv_append_mode"] = json!(variant.kv_append_mode());
        performance_profile["kv_copy_artifact_path"] = json!(artifact);
        performance_profile["kv_copy_artifact"] = artifact_identity(copy);
    }
    let layer_projection = driver.layer_projection_mode();
    let rmsnorm_mode = driver.rmsnorm_mode();
    let attention_mode = driver.attention_mode();
    let c1_packet_mode = driver.c1_packet_mode();
    let split_attention_mode = driver.split_attention_mode();
    let split_attention_workspace_bytes = driver.split_attention_workspace_bytes();
    let transposed_weight_bytes = driver.transposed_weight_bytes();
    let fp32_workspace_bytes = driver.fp32_head_workspace_bytes();
    let mut runtime =
        EngineeringTpBatchRuntimeV2::new_wide32(driver, pool, scheduler, ROWS, false)?;
    drop(setup_scope);
    let mut setup = json!({
        "schema":"FerricQwen3TpBatchSetupV2", "authority":"none", "performance_qualified":false,
        "serving_qualified":false, "live_profile":variant.live_profile(), "wave_target_mode":options.mode.label(),
        "layer_projection":layer_projection, "rmsnorm_mode":rmsnorm_mode, "attention_mode":attention_mode,
        "benchmark_admitted":false, "serving_admitted":false,
        "model":"Qwen/Qwen3-8B", "dtype":"BF16", "target":"gfx950:xnack-",
        "tensor_parallel":1, "device_unique_ids":[live.device], "worker_pids":[worker_pid],
        "worker_sha256":live.worker_sha256, "running_worker_sha256":[live.worker_sha256],
        "controller_sha256":controller_sha256, "model_bundle_id":hex(model.bundle_id().as_bytes()),
        "target_model_id":hex(model.config().model_id.as_bytes()), "session_id":hex(&session),
        "artifact_hsaco_id":hex(target_artifact.hsaco_id().as_bytes()),
        "artifact_manifest_id":hex(target_artifact.manifest_id().as_bytes()),
        "artifact_handoff_id":hex(target_artifact.handoff_id().as_bytes()),
        "fp32_head_artifact":artifact_identity(&head_artifact), "argmax_artifact":artifact_identity(&argmax_artifact),
        "attention_artifact":artifact_identity(&attention_artifact), "rmsnorm_artifact":artifact_identity(&rmsnorm_artifact),
        "attention_artifact_path":options.attention_artifact, "rmsnorm_artifact_path":options.rmsnorm_artifact,
        "head_precision":"fp32-v8", "argmax_mode":"wave-v11", "submission":live.submission.label(),
        "runtime_ordered_batches":live.runtime.ordered_batches, "performance_profile":performance_profile,
        "kernel_profile":"v5-mfma32", "kernel_row_capacity":ROWS, "batch_tokens":ROWS,
        "prefill_chunk":CHUNK, "page_tokens":16, "physical_pages":live.pages,
        "context_tokens":live.context, "cache_ttl_ticks":CACHE_TTL, "prefix_cache":false,
        "kv_pool_profile":"legacy-v5", "kv_pool_max_physical_pages":512, "kv_pool_payload_bytes":kv_pool_payload_bytes,
        "target_payload_bytes":model.target_weights().len(), "target_transposed_bytes":transposed_weight_bytes,
        "fp32_head_workspace_bytes":fp32_workspace_bytes, "max_batches":live.max_batches,
        "output_head_pruning":true, "collective":"device-tp1-v3", "prefill":"true_multirow_chunked",
        "attention":"paged_causal_gqa", "cache":"disabled", "setup_seconds":whole.elapsed().as_secs_f64(),
        "arrival_policy":"live monotonic ingress timestamps; includes pending wait",
        "live_protocol":"FerricQwen3TpLiveCommandV1/FerricQwen3TpLiveEventV1",
        "numerical_status":"Contracted; independently compare emitted token IDs; not a serving qualification",
    });
    if let (Variant::KvCopyV19 { artifact, .. }, Some(copy)) = (variant, &copy_artifact) {
        setup["kv_append_mode"] = json!(variant.kv_append_mode());
        setup["kv_copy_artifact_path"] = json!(artifact);
        setup["kv_copy_artifact"] = artifact_identity(copy);
    }
    if matches!(variant, Variant::PackedC1V22 { .. }) {
        setup["requested_c1_packet_mode"] = json!(variant.c1_packet_mode());
        setup["c1_packet_mode"] = json!(c1_packet_mode);
    }
    variant.annotate_split_attention(
        &mut setup,
        split_artifact.as_ref(),
        split_attention_mode,
        split_attention_workspace_bytes,
    )?;
    timing.setup = Some(setup.clone());
    let body = |runtime: &mut EngineeringTpBatchRuntimeV2<_>| {
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
    };
    let (result, close) = match variant {
        Variant::V17
        | Variant::KvCopyV19 { .. }
        | Variant::PackedC1V22 { .. }
        | Variant::SplitAttentionV25 { .. } => run_and_close(&mut runtime, body),
        Variant::RuntimeDiagnostic => run_diagnostic_and_close(
            &mut runtime,
            &timing.timing,
            live.device,
            worker_pid,
            emit_diagnostic,
            body,
        ),
    };
    let mut closed = json!({
        "schema":"FerricQwen3TpBatchClosedV2", "authority":"none", "performance_qualified":false,
        "live_profile":variant.live_profile(), "wave_target_mode":options.mode.label(), "layer_projection":layer_projection,
        "submission":live.submission.label(), "rmsnorm_mode":rmsnorm_mode, "attention_mode":attention_mode,
        "benchmark_admitted":false, "serving_admitted":false, "worker_pids":[worker_pid],
        "all_workers_exited":close.is_ok(), "execution_completed":result.is_ok(),
        "rank_dispatch_counts":runtime.dispatch_counts(), "whole_seconds":whole.elapsed().as_secs_f64(),
    });
    if let (Variant::KvCopyV19 { artifact, .. }, Some(copy)) = (variant, &copy_artifact) {
        closed["kv_append_mode"] = json!(variant.kv_append_mode());
        closed["kv_copy_artifact_path"] = json!(artifact);
        closed["kv_copy_artifact"] = artifact_identity(copy);
    }
    if matches!(variant, Variant::PackedC1V22 { .. }) {
        closed["requested_c1_packet_mode"] = json!(variant.c1_packet_mode());
        closed["c1_packet_mode"] = json!(c1_packet_mode);
    }
    variant.annotate_split_attention(
        &mut closed,
        split_artifact.as_ref(),
        split_attention_mode,
        split_attention_workspace_bytes,
    )?;
    timing.closed = Some(closed.clone());
    let emitted = emit(&closed);
    match (result, close, emitted) {
        (Ok(()), Ok(()), Ok(())) => Ok(()),
        (result, close, emitted) => Err(format!(
            "live execution: {result:?}; close: {close:?}; closed record: {emitted:?}"
        )),
    }
}

#[allow(dead_code)] // The diagnostic executable selects run_variant directly.
pub(super) fn run(options: &Options) -> Result<(), String> {
    run_variant(options, Variant::V17)
}

pub(super) fn run_variant(options: &Options, variant: Variant<'_>) -> Result<(), String> {
    variant.validate(options)?;
    let mut timing = TimingFile::create(options.live.host_timing.as_deref())?;
    let result = run_with_timing(options, &mut timing, variant);
    let sidecar = timing.finish(&result);
    match (result, sidecar) {
        (Ok(()), Ok(())) => Ok(()),
        (result, sidecar) => Err(format!("live run: {result:?}; timing: {sidecar:?}")),
    }
}

#[cfg(test)]
mod live_tests {
    use super::*;
    use ferric_m1_engineering_execution_v1::tp_execution::batched::EngineeringTpBatchOutputV2;
    use ferric_m1_engineering_execution_v1::tp_paged::EngineeringTpPreparedBatchV1;
    use std::cell::RefCell;
    use std::rc::Rc;
    use wave_target_v17_live_contract::{MODES, fixture};

    fn snapshot(ordinal: u64) -> Value {
        let counters = fe2o3_kfd::engineering_wire::PerformanceCountersV1 {
            commands: ordinal + 2,
            command_ns: ordinal + 10,
            ..Default::default()
        };
        json!({"schema":"FerricRuntimeDiagnosticSnapshotV1", "authority":"none",
            "performance_qualified":false, "rank":0, "process_id":7,
            "device_unique_id":9, "ordinal":ordinal, "counters":counters})
    }

    type Events = Rc<RefCell<Vec<String>>>;

    struct DiagnosticRunner {
        events: Events,
        snapshots: usize,
        fail_snapshot: Option<usize>,
        fail_close: bool,
    }

    impl EngineeringTpBatchRunnerV2 for DiagnosticRunner {
        fn runtime_diagnostic_snapshot(&mut self) -> Result<Vec<Value>, String> {
            let ordinal = self.snapshots;
            self.snapshots += 1;
            self.events.borrow_mut().push(format!("snapshot{ordinal}"));
            if self.fail_snapshot == Some(ordinal) {
                return Err("injected snapshot failure".into());
            }
            Ok(vec![snapshot(ordinal as u64)])
        }
        fn row_capacity(&self) -> usize {
            ROWS
        }
        fn execute_batch(
            &mut self,
            _: &EngineeringTpPreparedBatchV1,
            _: &[usize],
        ) -> Result<EngineeringTpBatchOutputV2, String> {
            Err("CPU lifecycle fixture does not execute kernels".into())
        }
        fn dispatch_counts(&self) -> Vec<u64> {
            vec![0]
        }
        fn close(&mut self) -> Result<(), String> {
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
    ) -> (EngineeringTpBatchRuntimeV2<DiagnosticRunner>, Events) {
        let options = fixture(wave_target_v17_live_contract::Mode::Combined);
        let pool = EngineeringTpPagedPoolV1::new_wide32(
            EngineeringTpPoolScopeV1 {
                model: [1; 32],
                session: [2; 32],
            },
            options.live.limits().unwrap(),
        )
        .unwrap();
        let scheduler = EngineeringTpSchedulerV1::new_wide32(100, 8192, ROWS, CHUNK).unwrap();
        let events = Rc::new(RefCell::new(Vec::new()));
        let runtime = EngineeringTpBatchRuntimeV2::new_wide32(
            DiagnosticRunner {
                events: events.clone(),
                snapshots: 0,
                fail_snapshot,
                fail_close,
            },
            pool,
            scheduler,
            ROWS,
            false,
        )
        .unwrap();
        (runtime, events)
    }

    #[test]
    fn diagnostic_derives_only_profile_and_preserves_frozen_v17_identity() {
        let mut options = fixture(wave_target_v17_live_contract::Mode::Combined);
        assert!(Variant::RuntimeDiagnostic.worker_runtime(&options).is_err());
        options.live.host_timing = Some("host.json".into());
        let base = Variant::V17.worker_runtime(&options).unwrap();
        let diagnostic = Variant::RuntimeDiagnostic.worker_runtime(&options).unwrap();
        assert!(!base.profile && diagnostic.profile && !options.live.runtime.profile);
        assert_eq!(base.cache_admission, diagnostic.cache_admission);
        assert_eq!(base.operational, diagnostic.operational);
        assert_eq!(base.sequences, diagnostic.sequences);
        assert_eq!(base.ordered_batches, diagnostic.ordered_batches);
        assert_eq!(base.rollover, diagnostic.rollover);
        assert_eq!(
            base.shared_full_currentness,
            diagnostic.shared_full_currentness
        );
        assert_eq!(Variant::V17.live_profile(), LIVE_PROFILE);
        assert_ne!(
            Variant::V17.live_profile(),
            Variant::RuntimeDiagnostic.live_profile()
        );
        for mode in MODES {
            options.mode = mode;
            assert_eq!(
                Variant::RuntimeDiagnostic.validate(&options).is_ok(),
                mode == wave_target_v17_live_contract::Mode::Combined
            );
        }
        options.mode = wave_target_v17_live_contract::Mode::Combined;
        options.live.runtime.profile = true;
        assert!(Variant::V17.validate(&options).is_err());
        assert!(Variant::RuntimeDiagnostic.validate(&options).is_err());
    }

    #[test]
    fn v19_runner_selection_is_separate_combined_only_and_unprofiled() {
        let mut options = fixture(wave_target_v17_live_contract::Mode::Combined);
        for enabled in [false, true] {
            let variant = Variant::KvCopyV19 {
                artifact: Path::new("copy"),
                enabled,
            };
            assert!(variant.validate(&options).is_ok());
            assert!(!variant.worker_runtime(&options).unwrap().profile);
            assert_eq!(variant.live_profile(), "c1-kv-copy-v19-live-v1");
            assert_eq!(
                variant.kv_append_mode(),
                if enabled {
                    "parallel-c1-v19"
                } else {
                    "baseline"
                }
            );
            assert!(
                Variant::KvCopyV19 {
                    artifact: Path::new(""),
                    enabled
                }
                .validate(&options)
                .is_err()
            );
            options.mode = wave_target_v17_live_contract::Mode::Baseline;
            assert!(variant.validate(&options).is_err());
            options.mode = wave_target_v17_live_contract::Mode::Combined;
        }
        assert_eq!(Variant::V17.kv_append_mode(), "baseline");
    }

    #[test]
    fn v22_runner_is_explicit_combined_only_and_binds_requested_actual_metadata() {
        let mut options = fixture(wave_target_v17_live_contract::Mode::Combined);
        for enabled in [false, true] {
            let variant = Variant::PackedC1V22 { enabled };
            assert!(variant.validate(&options).is_ok());
            let base = Variant::V17.worker_runtime(&options).unwrap();
            let candidate = variant.worker_runtime(&options).unwrap();
            assert!(!candidate.profile);
            assert_eq!(base.cache_admission, candidate.cache_admission);
            assert_eq!(base.operational, candidate.operational);
            assert_eq!(base.sequences, candidate.sequences);
            assert_eq!(base.ordered_batches, candidate.ordered_batches);
            assert_eq!(base.rollover, candidate.rollover);
            assert_eq!(
                base.shared_full_currentness,
                candidate.shared_full_currentness
            );
            assert_eq!(variant.live_profile(), "c1-packed-v22-live-v1");
            let mode = if enabled { "packed16-v22" } else { "baseline" };
            let mut record = json!({"authority":"none"});
            variant.annotate_packet_mode(&mut record, mode).unwrap();
            assert_eq!(record["requested_c1_packet_mode"], mode);
            assert_eq!(record["c1_packet_mode"], mode);
            let before = record.clone();
            assert!(
                variant
                    .annotate_packet_mode(&mut record, "unknown")
                    .is_err()
            );
            assert_eq!(record, before);
            for other in MODES {
                options.mode = other;
                assert_eq!(
                    variant.validate(&options).is_ok(),
                    other == wave_target_v17_live_contract::Mode::Combined
                );
            }
            options.mode = wave_target_v17_live_contract::Mode::Combined;
        }
        let mut frozen = json!({"live_profile":LIVE_PROFILE});
        let before = frozen.clone();
        Variant::V17
            .annotate_packet_mode(&mut frozen, "baseline")
            .unwrap();
        assert_eq!(frozen, before);
    }

    #[test]
    fn v25_selects_only_the_separate_split_route_and_preserves_historical_metadata() {
        let combined = wave_target_v17_live_contract::Mode::Combined;
        for enabled in [false, true] {
            let variant = Variant::SplitAttentionV25 {
                artifact: Path::new("split"),
                enabled,
            };
            let options = fixture(combined);
            assert!(variant.validate(&options).is_ok());
            assert_eq!(variant.live_profile(), "c1-split-attention-v25-live-v1");
            assert_eq!(
                variant.split_attention_mode(),
                if enabled { "split8-v21" } else { "baseline" }
            );
            assert_eq!(variant.kv_append_mode(), "baseline");
            assert_eq!(variant.c1_packet_mode(), "baseline");
            let runtime = variant.worker_runtime(&options).unwrap();
            assert!(!runtime.profile && runtime.ordered_batches && !runtime.sequences);
            assert!(
                variant
                    .annotate_split_attention(
                        &mut json!({}),
                        None,
                        variant.split_attention_mode(),
                        133_120
                    )
                    .is_err()
            );
            for mode in MODES {
                assert_eq!(variant.validate(&fixture(mode)).is_ok(), mode == combined);
            }
            let mut timed = fixture(combined);
            timed.live.host_timing = Some(PathBuf::from("timing"));
            assert!(variant.validate(&timed).is_err());
            assert!(
                Variant::SplitAttentionV25 {
                    artifact: Path::new(""),
                    enabled
                }
                .validate(&options)
                .is_err()
            );
        }
        for variant in [
            Variant::V17,
            Variant::RuntimeDiagnostic,
            Variant::KvCopyV19 {
                artifact: Path::new("copy"),
                enabled: true,
            },
            Variant::PackedC1V22 { enabled: true },
        ] {
            let mut metadata = json!({"original":true});
            variant
                .annotate_split_attention(&mut metadata, None, "unused", 0)
                .unwrap();
            assert_eq!(metadata, json!({"original":true}));
        }
    }

    #[test]
    #[ignore = "requires actual emitted V21 image; CPU admission and event-schema validation only"]
    fn v25_event_metadata_binds_actual_v21_image_and_exact_counts_in_both_arms() {
        let path = PathBuf::from(
            std::env::var_os("FERRIC_TEST_SPLIT_ATTENTION_V21_ARTIFACT")
                .expect("explicit V21 image"),
        );
        let image = EngineeringTpArtifactV1::open_split_attention_v21(&path).unwrap();
        for enabled in [false, true] {
            let variant = Variant::SplitAttentionV25 {
                artifact: &path,
                enabled,
            };
            let mut event = json!({"performance_qualified":false});
            variant
                .annotate_split_attention(
                    &mut event,
                    Some(&image),
                    variant.split_attention_mode(),
                    133_120,
                )
                .unwrap();
            assert_eq!(
                event["requested_split_attention_mode"],
                variant.split_attention_mode()
            );
            assert_eq!(
                event["split_attention_mode"],
                variant.split_attention_mode()
            );
            assert_eq!(event["split_attention_artifact"], artifact_identity(&image));
            assert_eq!(event["split_attention_workspace_bytes"], 133_120);
            assert_eq!(
                event["split_attention_policy"],
                json!({"physical_rows":1,"actual_context_min":128,"actual_context_max":256,
                "partitions":8,"fallback":"query-hoist-v14","fallback_packets_no_head":613,"fallback_packets_with_head":616,
                "split_packets_no_head":649,"split_packets_with_head":652})
            );
            assert!(
                variant
                    .annotate_split_attention(&mut event, Some(&image), "wrong", 133_120)
                    .is_err()
            );
            assert!(
                variant
                    .annotate_split_attention(
                        &mut event,
                        Some(&image),
                        variant.split_attention_mode(),
                        133_119
                    )
                    .is_err()
            );
        }
    }

    #[test]
    fn diagnostic_snapshot_requires_exact_complete_monotonic_bound_counters() {
        let before = snapshot(0);
        let after = snapshot(1);
        assert_eq!(
            checked_snapshot(std::slice::from_ref(&before), None, 9, 7, &[0]).unwrap(),
            json!({})
        );
        let delta =
            checked_snapshot(std::slice::from_ref(&after), Some(&before), 9, 7, &[0]).unwrap();
        assert_eq!(delta["commands"], 1);
        assert_eq!(delta["command_ns"], 1);
        assert_eq!(delta["dispatches"], 0);
        assert!(checked_snapshot(&[], None, 9, 7, &[0]).is_err());
        assert!(checked_snapshot(&[before.clone(), before.clone()], None, 9, 7, &[0]).is_err());
        for mutation in 0..11 {
            let mut wrong = after.clone();
            match mutation {
                0 => wrong["rank"] = json!(1),
                1 => wrong["ordinal"] = json!(0),
                2 => wrong["process_id"] = json!(8),
                3 => wrong["device_unique_id"] = json!(10),
                4 => wrong["authority"] = json!("granted"),
                5 => wrong["performance_qualified"] = json!(true),
                6 => wrong["counters"]["commands"] = json!(0),
                7 => {
                    wrong["counters"]
                        .as_object_mut()
                        .unwrap()
                        .remove("dispatch_wait_ns");
                }
                8 => wrong["counters"]["unexpected"] = json!(0),
                9 => wrong["counters"]["dispatches"] = json!(1),
                10 => wrong["counters"]["dispatch_wait_ns"] = json!("0"),
                _ => unreachable!(),
            }
            assert!(
                checked_snapshot(&[wrong], Some(&before), 9, 7, &[0]).is_err(),
                "mutation {mutation}"
            );
        }
    }

    #[test]
    fn diagnostic_lifecycle_closes_once_on_success_and_every_fallible_boundary() {
        for failure in 0..7 {
            let fail_snapshot = match failure {
                1 => Some(0),
                4 => Some(1),
                _ => None,
            };
            let (mut runtime, events) = runtime(fail_snapshot, failure == 6);
            let timing = HostTiming::enabled();
            let mut outputs = 0;
            let (result, close) = run_diagnostic_and_close(
                &mut runtime,
                &timing,
                9,
                7,
                |value| {
                    events.borrow_mut().push(format!("output{outputs}"));
                    let should_fail =
                        (failure == 2 && outputs == 0) || (failure == 5 && outputs == 1);
                    outputs += 1;
                    assert_eq!(value["performance_qualified"], false);
                    assert_eq!(value["live_profile"], DIAGNOSTIC_PROFILE);
                    if should_fail {
                        Err("injected output failure".into())
                    } else {
                        Ok(())
                    }
                },
                |_| {
                    events.borrow_mut().push("body".into());
                    if failure == 3 {
                        Err("injected workload failure".into())
                    } else {
                        Ok(())
                    }
                },
            );
            assert_eq!(result.is_ok(), failure == 0 || failure == 6);
            assert_eq!(close.is_ok(), failure != 6);
            let events = events.borrow();
            assert_eq!(events.last().unwrap(), "close");
            assert_eq!(
                events
                    .iter()
                    .filter(|event| event.as_str() == "close")
                    .count(),
                1
            );
            if failure == 0 || failure == 6 {
                assert_eq!(
                    *events,
                    [
                        "snapshot0",
                        "output0",
                        "body",
                        "snapshot1",
                        "output1",
                        "close"
                    ]
                );
            }
            assert!(runtime.runtime_diagnostic_snapshot().is_err());
            assert_eq!(timing.snapshot()["active_records"], 0);
        }
    }

    #[test]
    fn placement_parser_handles_parenthesized_names_and_rejects_incomplete_observations() {
        let mut fields = vec!["0"; 37];
        fields[0] = "S";
        fields[15] = "39";
        fields[16] = "19";
        fields[19] = "100";
        fields[36] = "3";
        let stat = format!("7 (worker ) name) {}", fields.join(" "));
        let value = parse_placement(7, &stat, "Name:\tworker\nCpus_allowed_list:\t2-3\n").unwrap();
        assert_eq!(value["nice"], 19);
        assert_eq!(value["priority"], 39);
        assert_eq!(value["last_processor"], 3);
        assert_eq!(value["cpus_allowed_list"], "2-3");
        assert!(parse_placement(8, &stat, "Cpus_allowed_list: 2-3").is_err());
        assert!(parse_placement(7, "7 (worker) S", "Cpus_allowed_list: 2-3").is_err());
        for status in [
            "",
            "Cpus_allowed_list: ",
            "Cpus_allowed_list: unknown",
            "Cpus_allowed_list: 0\nCpus_allowed_list: 1",
        ] {
            assert!(parse_placement(7, &stat, status).is_err());
        }
    }

    #[test]
    fn v17_metadata_binds_each_actual_route_and_never_grants_qualification() {
        for mode in MODES {
            let options = fixture(mode);
            let profile =
                profile_metadata(&options, "c1-wave", mode.attention(), mode.rmsnorm()).unwrap();
            assert_eq!(profile["wave_target_mode"], mode.label());
            assert_eq!(profile["attention"], mode.attention());
            assert_eq!(profile["rmsnorm_mode"], mode.rmsnorm());
            assert_eq!(profile["benchmark_admitted"], false);
            assert_eq!(profile["serving_admitted"], false);
            assert_eq!(profile["runtime_ordered_batches"], true);
            for other in MODES {
                assert_eq!(
                    profile_metadata(&options, "c1-wave", other.attention(), other.rmsnorm())
                        .is_ok(),
                    mode == other
                );
            }
            assert!(profile_metadata(&options, "mfma", mode.attention(), mode.rmsnorm()).is_err());
            assert!(profile_metadata(&options, "c1-wave", "auto", mode.rmsnorm()).is_err());
        }
    }

    #[test]
    fn v17_invalid_options_reject_before_creating_timing_or_spawning_workers() {
        let path = std::env::temp_dir().join(format!(
            "ferric-wave-target-v17-reject-{}.json",
            std::process::id()
        ));
        assert!(!path.exists());
        for mode in MODES {
            for mutation in 0..5 {
                let mut options = fixture(mode);
                options.live.host_timing = Some(path.clone());
                match mutation {
                    0 => options.live.context = 256,
                    1 => options.live.runtime.ordered_batches = false,
                    2 => options.attention_artifact = PathBuf::new(),
                    3 => options.rmsnorm_artifact = PathBuf::new(),
                    4 => options.live.runtime.shared_full_currentness = true,
                    _ => unreachable!(),
                }
                assert!(run(&options).is_err());
                assert!(!path.exists());
            }
        }
    }
}
