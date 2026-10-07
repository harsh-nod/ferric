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
use tp_worker::{RuntimeOptions, TokenProgramBackend, Worker};
use wave_argmax_live_contract::{CACHE_TTL, CHUNK, ROWS};
use wave_target_v17_live_contract::Options;

#[path = "packed_gate_up_live_profile.rs"]
mod packed_gate_up_live_profile;
#[allow(unused_imports)] // Only the dedicated packed executable parses these inputs.
pub(super) use packed_gate_up_live_profile::{
    PackedGateUp, digest_argument as packed_digest_argument,
};

#[path = "packed_down_live_profile.rs"]
mod packed_down_live_profile;
pub(super) use packed_down_live_profile::PackedDown;

#[path = "native_gate_up_live_profile.rs"]
mod native_gate_up_live_profile;
#[allow(unused_imports)]
pub(super) use native_gate_up_live_profile::{NativeGateUp, parse as parse_native_gate_up};

const LIVE_PROFILE: &str = "wave-target-v17-live-v1";
const DIAGNOSTIC_PROFILE: &str = "wave-target-v17-runtime-diagnostic-v1";
const ORDERED64_HOST_DIAGNOSTIC_PROFILE: &str = "prefill16-decode-ordered64-host-diagnostic-v1";
const ORDERED64_RUNTIME_COUNTER_PROFILE: &str = "prefill16-decode-ordered64-runtime-counters-v1";
const ORDERED64_ACTIVE_POLL_PROFILE: &str = "prefill16-decode-ordered64-active-poll-10ms-v1";
const PREFILL16_ORDERED_PROFILE: &str = "prefill16-ordered64-host-diagnostic-v1";
const ORDERED64_PACKET_TICKS_PROFILE: &str = "prefill16-decode-ordered64-kv-copy-packet-ticks-v1";
const ORDERED64_BASELINE_PACKET_TICKS_PROFILE: &str =
    "prefill16-decode-ordered64-baseline-kv-packet-ticks-v1";
const ORDERED64_HOST_MAX_MODEL_BATCHES: u64 = 256;
const ORDERED64_KV_COPY_PROFILE: &str = "prefill16-decode-ordered64-kv-copy-v1";
const PREFILL32_PAGES_PROFILE: &str = "prefill32-pages-decode-ordered64-kv-copy-v1";

#[derive(Clone, Copy)]
pub(super) struct Ordered64KvCopy<'a> {
    pub artifact: &'a Path,
    pub enabled: bool,
    pub prefill32_pages: Option<bool>,
}

impl Ordered64KvCopy<'_> {
    fn live_profile(self) -> &'static str {
        if self.prefill32_pages.is_some() {
            PREFILL32_PAGES_PROFILE
        } else {
            ORDERED64_KV_COPY_PROFILE
        }
    }

    fn prefill_chunk(self) -> usize {
        if self.prefill32_pages.is_some() {
            32
        } else {
            CHUNK
        }
    }

    fn prefill32_mode(self) -> &'static str {
        if self.prefill32_pages == Some(true) {
            "parallel-prefill32-two-pages-v27"
        } else {
            "baseline"
        }
    }

    fn mode(self) -> &'static str {
        if self.enabled {
            "parallel-c1-v19"
        } else {
            "baseline"
        }
    }

    pub(super) fn validate(self, options: &Options, variant: Variant<'_>) -> Result<(), String> {
        variant.validate(options)?;
        if self.prefill32_pages.is_some()
            && (!self.enabled || options.live.context < 32 || options.live.pages < 2)
        {
            return Err(
                "prefill32 pages requires parallel V19 decode, context >=32 and at least two pages"
                    .into(),
            );
        }
        if self.artifact.as_os_str().is_empty()
            || options.live.host_timing.is_some()
            || options.live.runtime.profile
            || options.live.runtime.ordered64_packet_ticks
            || !matches!(
                variant,
                Variant::PrefillKvCopyV28 {
                    enabled: true,
                    decode: Some(DecodeComposition {
                        split: true,
                        packed: true,
                        ordered64: true,
                        gemv: Some((_, false)),
                        ..
                    }),
                    ..
                }
            )
        {
            return Err("ordered64 KV copy requires exact V27/split8/packed64/baseline-GEMV and no diagnostics".into());
        }
        Ok(())
    }

    fn validate_packet_ticks(self, options: &Options, variant: Variant<'_>) -> Result<(), String> {
        validate_ordered64_host_diagnostic(options, variant)?;
        if !cfg!(all(feature = "c1-ordered64", feature = "model-timestamps"))
            || !options.live.runtime.ordered64_packet_ticks
            || self.artifact.as_os_str().is_empty()
            || !self.enabled
            || self.prefill32_pages.is_some()
        {
            return Err(
                "packet ticks require the dedicated V19-enabled prefill16 composition".into(),
            );
        }
        Ok(())
    }

    fn annotate(self, value: &mut Value, artifact: &EngineeringTpArtifactV1) {
        value["requested_kv_copy_mode"] = json!(self.mode());
        value["kv_copy_mode"] = json!(self.mode());
        value["kv_append_mode"] = json!(self.mode());
        value["kv_copy_artifact_path"] = json!(self.artifact);
        value["kv_copy_artifact"] = artifact_identity(artifact);
        if self.prefill32_pages.is_some() {
            value["requested_prefill32_pages_mode"] = json!(self.prefill32_mode());
            value["prefill32_pages_mode"] = json!(self.prefill32_mode());
            value["prefill_chunk"] = json!(self.prefill_chunk());
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // Independent explicit kernel and packet selectors.
pub(super) struct DecodeComposition<'a> {
    pub artifact: &'a Path,
    pub split: bool,
    pub packed: bool,
    pub ordered64: bool,
    pub gemv: Option<(&'a Path, bool)>,
}

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
    #[allow(dead_code)] // Selected only by the separate V28 executable.
    PrefillKvCopyV28 {
        artifact: &'a Path,
        enabled: bool,
        decode: Option<DecodeComposition<'a>>,
    },
    #[allow(dead_code)] // Selected only by the separate V25 executable.
    SplitAttentionV25 {
        artifact: &'a Path,
        enabled: bool,
        packed: Option<bool>,
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
        if let Self::PrefillKvCopyV28 { artifact, .. } = self
            && (options.mode != wave_target_v17_live_contract::Mode::Combined
                || artifact.as_os_str().is_empty())
        {
            return Err("V28 requires combined V17 and an explicit V27 prefill copy image".into());
        }
        if let Self::PrefillKvCopyV28 {
            decode: Some(decode),
            ..
        } = self
            && decode.ordered64
            && (!cfg!(feature = "c1-ordered64")
                || (cfg!(feature = "model-timestamps")
                    && !options.live.runtime.ordered64_packet_ticks)
                || !decode.packed)
        {
            return Err(
                "packed64-v29 requires its explicit non-diagnostic build and packed C1 composition"
                    .into(),
            );
        }
        if let Self::PrefillKvCopyV28 {
            decode: Some(decode),
            ..
        } = self
            && (decode.artifact.as_os_str().is_empty()
                || options.live.host_timing.is_some()
                || decode
                    .gemv
                    .is_some_and(|(path, _)| path.as_os_str().is_empty()))
        {
            return Err(
                "V28 decode composition requires an explicit V21 image and no diagnostic timing"
                    .into(),
            );
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
            Self::PrefillKvCopyV28 {
                decode:
                    Some(DecodeComposition {
                        ordered64: true, ..
                    }),
                ..
            } => "prefill16-decode-ordered64-v29-live-v1",
            Self::PrefillKvCopyV28 {
                decode: Some(DecodeComposition { gemv: Some(_), .. }),
                ..
            } => "prefill16-decode-partial-gemv-v28-live-v1",
            Self::PrefillKvCopyV28 {
                decode: Some(_), ..
            } => "prefill16-decode-composed-v28-live-v1",
            Self::PrefillKvCopyV28 { decode: None, .. } => "prefill16-kv-copy-v28-live-v1",
        }
    }

    fn worker_runtime(self, options: &Options) -> Result<RuntimeOptions, String> {
        self.validate(options)?;
        let mut runtime = options.live.runtime;
        runtime.ordered64 = self.ordered64();
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
        if self.ordered64() {
            "packed64-v29"
        } else if matches!(
            self,
            Self::PackedC1V22 { enabled: true }
                | Self::SplitAttentionV25 {
                    packed: Some(true),
                    ..
                }
                | Self::PrefillKvCopyV28 {
                    decode: Some(DecodeComposition { packed: true, .. }),
                    ..
                }
        ) {
            "packed16-v22"
        } else {
            "baseline"
        }
    }

    fn ordered64(self) -> bool {
        matches!(
            self,
            Self::PrefillKvCopyV28 {
                decode: Some(DecodeComposition {
                    ordered64: true,
                    ..
                }),
                ..
            }
        )
    }

    fn annotate_packet_mode(self, value: &mut Value, actual: &str) -> Result<(), String> {
        if matches!(
            self,
            Self::PackedC1V22 { .. }
                | Self::SplitAttentionV25 {
                    packed: Some(_),
                    ..
                }
                | Self::PrefillKvCopyV28 {
                    decode: Some(_),
                    ..
                }
        ) {
            if actual != self.c1_packet_mode() {
                return Err("actual C1 packet policy differs from requested mode".into());
            }
            value["requested_c1_packet_mode"] = json!(self.c1_packet_mode());
            value["c1_packet_mode"] = json!(actual);
            if self.ordered64() {
                value["ordered_wire_mode"] = json!("ordered64");
                value["packed_c1_group_bound"] = json!(64);
                value["non_c1_group_bound"] = json!(16);
            }
        }
        Ok(())
    }

    fn gemv(&self) -> Option<(&Path, bool)> {
        match self {
            Self::PrefillKvCopyV28 {
                decode: Some(decode),
                ..
            } => decode.gemv,
            _ => None,
        }
    }

    fn annotate_gemv(
        self,
        value: &mut Value,
        image: Option<&EngineeringTpArtifactV1>,
        actual: &str,
    ) -> Result<(), String> {
        if let Some((path, enabled)) = self.gemv() {
            let expected = if enabled {
                "partial-prefetch4-v20"
            } else {
                "baseline"
            };
            if actual != expected {
                return Err("actual partial GEMV policy differs".into());
            }
            let image = image.ok_or("partial GEMV metadata requires the admitted V20 image")?;
            let roots = ferric_m1_engineering_execution_v1::tp_artifact::ENGINEERING_TP_GEMV_PREFETCH_EXPORTS_V20;
            let kernels = image.inspection().hsaco().kernels();
            if kernels.len() != 2
                || !roots
                    .iter()
                    .all(|root| kernels.iter().any(|kernel| kernel.name() == *root))
            {
                return Err("partial GEMV metadata image differs".into());
            }
            value["requested_gemv_mode"] = json!(expected);
            value["gemv_mode"] = json!(actual);
            value["gemv_artifact_path"] = json!(path);
            value["gemv_artifact"] = artifact_identity(image);
        }
        Ok(())
    }

    fn prefill_kv_mode(self) -> &'static str {
        if matches!(self, Self::PrefillKvCopyV28 { enabled: true, .. }) {
            "parallel-prefill16-v27"
        } else {
            "baseline"
        }
    }

    fn annotate_prefill_mode(self, value: &mut Value, actual: &str) -> Result<(), String> {
        if let Self::PrefillKvCopyV28 { .. } = self {
            if actual != self.prefill_kv_mode() {
                return Err("actual V28 prefill copy policy differs".into());
            }
            value["requested_prefill_kv_mode"] = json!(self.prefill_kv_mode());
            value["prefill_kv_mode"] = json!(actual);
        }
        Ok(())
    }

    fn split_attention_mode(self) -> &'static str {
        if matches!(
            self,
            Self::SplitAttentionV25 { enabled: true, .. }
                | Self::PrefillKvCopyV28 {
                    decode: Some(DecodeComposition { split: true, .. }),
                    ..
                }
        ) {
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
        if let Self::SplitAttentionV25 { artifact, .. }
        | Self::PrefillKvCopyV28 {
            decode: Some(DecodeComposition { artifact, .. }),
            ..
        } = self
        {
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

#[allow(dead_code)] // Only the separate ordered64 host executable selects this contract.
pub(super) fn validate_ordered64_host_diagnostic(
    options: &Options,
    variant: Variant<'_>,
) -> Result<(), String> {
    variant.validate(options)?;
    if options.live.max_batches > ORDERED64_HOST_MAX_MODEL_BATCHES
        || !matches!(
            variant,
            Variant::PrefillKvCopyV28 {
                enabled: true,
                decode: Some(DecodeComposition {
                    split: true,
                    packed: true,
                    ordered64: true,
                    gemv: Some((_, false)),
                    ..
                }),
                ..
            }
        )
    {
        return Err("ordered64 host diagnostic requires the exact baseline-GEMV composition and at most 256 physical model batches".into());
    }
    Ok(())
}

fn annotate_ordered64_host_diagnostic(value: &mut Value) {
    value["host_timing_schema"] = json!("FerricOrdered64HostTimingV1");
    value["diagnostic_max_model_batches"] = json!(ORDERED64_HOST_MAX_MODEL_BATCHES);
    value["host_diagnostic_scope"] = json!(
        "overlapping controller and worker wall durations; not GPU time; instrumentation perturbs execution"
    );
}

fn ordered64_counter_options(
    options: &Options,
    variant: Variant<'_>,
) -> Result<RuntimeOptions, String> {
    validate_ordered64_host_diagnostic(options, variant)?;
    variant
        .worker_runtime(options)?
        .with_ordered64_runtime_counters()
}

fn annotate_ordered64_runtime_counters(value: &mut Value) {
    value["runtime_profiling"] = json!(true);
    value["runtime_counter_schema"] = json!("FerricOrdered64RuntimeCountersV1");
    value["runtime_counter_scope"] = json!(
        "two cumulative worker host-wall snapshots around the complete workload; scopes overlap and snapshot boundaries differ; not GPU timestamps or isolated IOCTL time"
    );
}

fn ordered64_counter_record(value: &Value) -> Value {
    let mut value = value.clone();
    value["schema"] = json!("FerricOrdered64RuntimeCountersV1");
    value["live_profile"] = json!(ORDERED64_RUNTIME_COUNTER_PROFILE);
    value["command_accounting"] = json!(
        "commands and command_ns deltas include the earlier snapshot command and exclude the final snapshot command; currentness deltas exclude the earlier snapshot check_idle and include the final snapshot check_idle"
    );
    annotate_ordered64_host_diagnostic(&mut value);
    annotate_ordered64_runtime_counters(&mut value);
    value
}

fn annotate_ordered64_active_poll(value: &mut Value) {
    value["live_profile"] = json!(ORDERED64_ACTIVE_POLL_PROFILE);
    value["ordered64_wait_policy"] = json!({
        "policy":"ActivePoll10msV1", "worker_entry":"--diagnostic-active-poll-10ms",
        "active_window_ns":10_000_000_u64, "fallback_sleep_ns":50_000_u64,
        "scope":"ordinary ordered64 only; unchanged validation and original deadline",
        "terminal_schema":"Fe2o3Ordered64WaitPolicyDiagnosticV1",
        "terminal_stream":"worker stderr; independent native harness must bind PID/device/counts",
        "cpu_scope":"single worker thread may actively poll for up to 10ms per group; no aggregate CPU-time claim",
        "performance_qualified":false,
    });
}

fn ordered64_counter_record_with_wait(value: &Value, active_poll: bool) -> Value {
    let mut value = ordered64_counter_record(value);
    if active_poll {
        annotate_ordered64_active_poll(&mut value);
    }
    value
}

fn annotate_prefill16_ordered(value: &mut Value) {
    value["live_profile"] = json!(PREFILL16_ORDERED_PROFILE);
    value["non_c1_group_bound"] = json!(64);
    value["prefill_scheduling"] = json!({
        "mode":"prefill16-ordered64-v1",
        "eligible_rows":16,
        "eligible_output_rows":[[], [15]],
        "layer_commands_per_eligible_chunk":612,
        "ordered_groups_per_eligible_chunk":10,
        "fallback_non_c1_group_bound":16,
        "embedding_and_head_singletons_preserved":true,
        "flattened_gpu_commands_unchanged":true,
        "performance_qualified":false,
    });
}

#[derive(Clone, Copy)]
enum ExecutionMode<'a> {
    Ordinary,
    #[allow(dead_code)]
    BaselinePacketTicks,
    ActivePoll10ms,
    Prefill16Ordered,
    TokenProgram(TokenProgramBackend),
    TokenProgramCounters(TokenProgramBackend),
    NativePrefillProgram {
        counters: bool,
    },
    NativePrefillWidthProgram {
        rows: u32,
        counters: bool,
    },
    NativeGateUpProgram {
        selection: &'a NativeGateUp,
        counters: bool,
    },
    PackedGateUp(&'a PackedGateUp),
    PackedGateUpHostDiagnostic(&'a PackedGateUp),
    PackedDown(&'a PackedDown),
}

fn run_with_timing(
    options: &Options,
    timing: &mut TimingFile,
    variant: Variant<'_>,
    model_timestamp_output: Option<&Path>,
    ordered64_runtime_counters: bool,
    composed_copy: Option<Ordered64KvCopy<'_>>,
    execution: ExecutionMode<'_>,
) -> Result<(), String> {
    let packed_host_diagnostic = matches!(execution, ExecutionMode::PackedGateUpHostDiagnostic(_));
    let active_poll = matches!(execution, ExecutionMode::ActivePoll10ms);
    let prefill16_ordered = matches!(execution, ExecutionMode::Prefill16Ordered);
    let baseline_packet_ticks = matches!(execution, ExecutionMode::BaselinePacketTicks);
    let native_gate_up = if let ExecutionMode::NativeGateUpProgram { selection, .. } = execution {
        Some(selection)
    } else {
        None
    };
    let prefill_width = match execution {
        ExecutionMode::NativePrefillWidthProgram { rows, .. } => Some(rows),
        ExecutionMode::NativeGateUpProgram { .. } => Some(32),
        _ => None,
    };
    if prefill_width.is_some_and(|rows| !matches!(rows, 16 | 32))
        || (prefill_width == Some(32) && (options.live.context < 32 || options.live.pages < 2))
    {
        return Err(
            "native prefill width requires explicit 16/32 and complete-page storage".into(),
        );
    }
    let prefill_program = matches!(
        execution,
        ExecutionMode::NativePrefillProgram { .. }
            | ExecutionMode::NativePrefillWidthProgram { .. }
            | ExecutionMode::NativeGateUpProgram { .. }
    );
    let token_counters = matches!(
        execution,
        ExecutionMode::TokenProgramCounters(_)
            | ExecutionMode::NativePrefillProgram { counters: true }
            | ExecutionMode::NativePrefillWidthProgram { counters: true, .. }
            | ExecutionMode::NativeGateUpProgram { counters: true, .. }
    );
    let token_backend = match execution {
        ExecutionMode::NativePrefillWidthProgram { .. }
        | ExecutionMode::NativeGateUpProgram { .. } => {
            Some(TokenProgramBackend::NativeWholeProgramSlots512V1)
        }
        ExecutionMode::NativePrefillProgram { .. } => {
            Some(TokenProgramBackend::NativeWholeProgramV1)
        }
        ExecutionMode::TokenProgram(backend) | ExecutionMode::TokenProgramCounters(backend) => {
            Some(backend)
        }
        _ => None,
    };
    let packed_down = if let ExecutionMode::PackedDown(selection) = execution {
        Some(selection)
    } else {
        None
    };
    #[allow(clippy::match_same_arms)] // Keep the ordinary packed branch distinct from diagnostics.
    let (token_program, packed_gate_up) = match execution {
        ExecutionMode::Ordinary => (false, None),
        ExecutionMode::BaselinePacketTicks => (false, None),
        ExecutionMode::ActivePoll10ms => (false, None),
        ExecutionMode::Prefill16Ordered => (false, None),
        ExecutionMode::TokenProgram(_) => (true, None),
        ExecutionMode::TokenProgramCounters(_) => (true, None),
        ExecutionMode::NativePrefillProgram { .. } => (true, None),
        ExecutionMode::NativePrefillWidthProgram { .. } => (true, None),
        ExecutionMode::NativeGateUpProgram { .. } => (true, None),
        ExecutionMode::PackedGateUp(selection) => (false, Some(selection)),
        ExecutionMode::PackedGateUpHostDiagnostic(selection) => (false, Some(selection)),
        ExecutionMode::PackedDown(_) => (false, None),
    };
    variant.validate(options)?;
    if let Some(selection) = native_gate_up {
        selection.validate()?;
        validate_token_program(options, variant, composed_copy)?;
        if timing.timing.is_enabled()
            || model_timestamp_output.is_some()
            || ordered64_runtime_counters
            || options.live.runtime.ordered64_packet_ticks
        {
            return Err("native gate/up requires its separate latency or counter entry".into());
        }
    }
    if prefill16_ordered {
        validate_ordered64_host_diagnostic(options, variant)?;
        if !timing.timing.is_ordered64_diagnostic()
            || model_timestamp_output.is_some()
            || composed_copy.is_some()
            || options.live.runtime.ordered64_packet_ticks
        {
            return Err(
                "prefill16 scheduling requires its ordinary ordered64 host diagnostic".into(),
            );
        }
    }
    if active_poll
        && (!timing.timing.is_ordered64_diagnostic()
            || !ordered64_runtime_counters
            || model_timestamp_output.is_some()
            || composed_copy.is_some()
            || options.live.runtime.ordered64_packet_ticks)
    {
        return Err(
            "active polling requires the dedicated ordinary ordered64 counter diagnostic".into(),
        );
    }
    if token_program {
        validate_token_program(options, variant, composed_copy)?;
        if token_counters && options.live.max_batches > ORDERED64_HOST_MAX_MODEL_BATCHES {
            return Err("token counters require at most 256 physical model batches".into());
        }
        if timing.timing.is_enabled()
            || model_timestamp_output.is_some()
            || ordered64_runtime_counters
        {
            return Err("token program excludes instrumentation".into());
        }
    }
    let ordered64_packet_ticks = options.live.runtime.ordered64_packet_ticks;
    if baseline_packet_ticks && !ordered64_packet_ticks {
        return Err("baseline packet ticks require the explicit diagnostic parser".into());
    }
    if packed_host_diagnostic {
        validate_ordered64_host_diagnostic(options, variant)?;
        if !timing.timing.is_ordered64_diagnostic() {
            return Err("packed host diagnostic requires its dedicated recorder".into());
        }
    }
    if let Some(selection) = packed_gate_up {
        selection.validate(options, variant)?;
        if composed_copy.is_none_or(|copy| !copy.enabled || copy.prefill32_pages.is_some())
            || (timing.timing.is_enabled() && !packed_host_diagnostic)
            || model_timestamp_output.is_some()
            || ordered64_runtime_counters
            || ordered64_packet_ticks
        {
            return Err(
                "packed gate/up requires V19 and excludes prefill32 and instrumentation".into(),
            );
        }
    }
    if let Some(selection) = packed_down {
        selection.validate(options, variant)?;
        if composed_copy.is_some()
            || timing.timing.is_enabled()
            || model_timestamp_output.is_some()
            || ordered64_runtime_counters
            || ordered64_packet_ticks
        {
            return Err("packed down excludes V19, prefill32 and all instrumentation".into());
        }
    }
    if let Some(selection) = composed_copy {
        if ordered64_packet_ticks {
            selection.validate_packet_ticks(options, variant)?;
        } else {
            selection.validate(options, variant)?;
        }
        if (timing.timing.is_enabled() && !packed_host_diagnostic)
            || (model_timestamp_output.is_some() && !ordered64_packet_ticks)
            || ordered64_runtime_counters
        {
            return Err("ordered64 KV copy excludes all instrumentation".into());
        }
    }
    let ordered64_host_diagnostic = timing.timing.is_ordered64_diagnostic();
    if ordered64_packet_ticks {
        validate_ordered64_host_diagnostic(options, variant)?;
        if !cfg!(all(feature = "c1-ordered64", feature = "model-timestamps"))
            || model_timestamp_output.is_none()
            || (composed_copy.is_none() != baseline_packet_ticks)
            || ordered64_host_diagnostic
            || ordered64_runtime_counters
            || options.live.runtime.profile
            || options.live.runtime.ordered64_runtime_counters
        {
            return Err(
                "packet ticks require their separate diagnostic output and exclude counters".into(),
            );
        }
    }
    if ordered64_runtime_counters && !ordered64_host_diagnostic {
        return Err("ordered64 runtime counters require the dedicated host diagnostic".into());
    }
    if ordered64_host_diagnostic {
        validate_ordered64_host_diagnostic(options, variant)?;
        if model_timestamp_output.is_some() {
            return Err("ordered64 host diagnostic cannot include model timestamps".into());
        }
    }
    #[cfg(not(feature = "model-timestamps"))]
    if model_timestamp_output.is_some() {
        return Err("model timestamp feature is disabled".into());
    }
    let live_profile = if let Some(selection) = native_gate_up {
        native_gate_up_live_profile::profile(selection.enabled, token_counters)
    } else if let Some(rows) = prefill_width {
        prefill_width_profile(rows, token_counters)
    } else if prefill_program {
        prefill_program_profile(token_counters)
    } else if prefill16_ordered {
        PREFILL16_ORDERED_PROFILE
    } else if active_poll {
        ORDERED64_ACTIVE_POLL_PROFILE
    } else if let Some(backend) = token_backend {
        if token_counters {
            backend.counter_profile()
        } else {
            backend.profile()
        }
    } else if packed_host_diagnostic {
        packed_gate_up_live_profile::HOST_DIAGNOSTIC_PROFILE
    } else if packed_gate_up.is_some() {
        packed_gate_up_live_profile::PROFILE
    } else if packed_down.is_some() {
        packed_down_live_profile::PROFILE
    } else if baseline_packet_ticks {
        ORDERED64_BASELINE_PACKET_TICKS_PROFILE
    } else if ordered64_packet_ticks {
        ORDERED64_PACKET_TICKS_PROFILE
    } else if let Some(selection) = composed_copy {
        selection.live_profile()
    } else if ordered64_runtime_counters {
        ORDERED64_RUNTIME_COUNTER_PROFILE
    } else if ordered64_host_diagnostic {
        ORDERED64_HOST_DIAGNOSTIC_PROFILE
    } else if model_timestamp_output.is_some() {
        "prefill16-v28-model-timestamps-v1"
    } else {
        variant.live_profile()
    };
    #[cfg(feature = "model-timestamps")]
    let model_timestamp_file = model_timestamp_output
        .map(|path| {
            use std::os::unix::fs::OpenOptionsExt;

            if ordered64_packet_ticks {
                return create_ordered64_packet_tick_file(path);
            }
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(path)
                .map_err(|error| format!("fresh model timestamp output: {error}"))
        })
        .transpose()?;
    let runtime_options = if token_counters {
        variant
            .worker_runtime(options)?
            .with_ordered64_runtime_counters()?
    } else if ordered64_runtime_counters {
        ordered64_counter_options(options, variant)?
    } else {
        variant.worker_runtime(options)?
    };
    let live = &options.live;
    let whole = Instant::now();
    let setup_scope = timing.timing.scope("setup");
    let controller_sha256 = hash_file(Path::new("/proc/self/exe"))?;
    if hash_file(&live.worker)? != live.worker_sha256 {
        return Err("worker hash differs".into());
    }
    // Reopen the separately pinned roster/image before model setup or worker allocation.
    let packed_artifact = packed_gate_up.map(PackedGateUp::open).transpose()?;
    let packed_down_artifact = packed_down.map(PackedDown::open).transpose()?;
    let native_gate_up_artifact = native_gate_up.map(NativeGateUp::open).transpose()?;
    let target_artifact = EngineeringTpArtifactV1::open_batch32(
        &live.target_artifact,
        &ferric_qwen3_tp_batch32_roster_bridge_v5::COMPILER_NAMES,
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
    let copy_path = composed_copy
        .map(|selection| selection.artifact)
        .or(match variant {
            Variant::KvCopyV19 { artifact, .. } => Some(artifact),
            _ => None,
        });
    let copy_artifact = copy_path
        .map(|path| {
            EngineeringTpArtifactV1::open_c1_kv_copy_v19(path).map_err(|error| error.to_string())
        })
        .transpose()?;
    let split_artifact = match variant {
        Variant::SplitAttentionV25 { artifact, .. }
        | Variant::PrefillKvCopyV28 {
            decode: Some(DecodeComposition { artifact, .. }),
            ..
        } => Some(
            EngineeringTpArtifactV1::open_split_attention_v21(artifact)
                .map_err(|error| error.to_string())?,
        ),
        _ => None,
    };
    let prefill_artifact = match variant {
        Variant::PrefillKvCopyV28 { artifact, .. } => Some(
            EngineeringTpArtifactV1::open_prefill_kv_copy_v27(artifact)
                .map_err(|error| error.to_string())?,
        ),
        _ => None,
    };
    let artifacts = EngineeringTpWaveTargetArtifactsV17 {
        argmax: &argmax_artifact,
        attention: &attention_artifact,
        rmsnorm: &rmsnorm_artifact,
    };
    let gemv_artifact = variant
        .gemv()
        .map(|(path, _)| {
            EngineeringTpArtifactV1::open_gemv_prefetch_v20(path).map_err(|error| error.to_string())
        })
        .transpose()?;
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
        prefill_width.map_or_else(
            || composed_copy.map_or(CHUNK, Ordered64KvCopy::prefill_chunk),
            |rows| rows as usize,
        ),
    )
    .map_err(|e| format!("scheduler: {e:?}"))?;
    let spawn = if active_poll {
        Worker::spawn_active_poll_with_timing
    } else {
        Worker::spawn_with_timing
    };
    #[cfg(all(feature = "c1-token-program", not(feature = "model-timestamps")))]
    let spawn = match (token_backend, token_counters) {
        (Some(TokenProgramBackend::Ordered64GroupsV1), false) => {
            Worker::spawn_token_program_with_timing
        }
        (Some(TokenProgramBackend::NativeWholeProgramV1), false) => {
            Worker::spawn_native_token_program_with_timing
        }
        (Some(TokenProgramBackend::Ordered64GroupsV1), true) => {
            Worker::spawn_token_program_counters_with_timing
        }
        (Some(TokenProgramBackend::NativeWholeProgramV1), true) => {
            Worker::spawn_native_token_program_counters_with_timing
        }
        (Some(TokenProgramBackend::NativeWholeProgramSlots512V1) | None, _) => spawn,
    };
    #[cfg(all(feature = "c1-token-program", not(feature = "model-timestamps")))]
    let spawn = if prefill_program && prefill_width.is_none() {
        if token_counters {
            Worker::spawn_native_prefill_program_counters_with_timing
        } else {
            Worker::spawn_native_prefill_program_with_timing
        }
    } else {
        spawn
    };
    #[cfg(all(feature = "c1-token-program", not(feature = "model-timestamps")))]
    let width_worker = prefill_width.map(|rows| {
        if let Some(selection) = native_gate_up {
            return Worker::spawn_native_gate_up_program(
                &live.worker,
                live.device,
                &target_artifact,
                runtime_options,
                timing.timing.clone(),
                0,
                selection.enabled,
                token_counters,
            );
        }
        Worker::spawn_native_prefill_width_program(
            &live.worker,
            live.device,
            &target_artifact,
            runtime_options,
            timing.timing.clone(),
            0,
            rows,
            token_counters,
        )
    });
    #[cfg(not(all(feature = "c1-token-program", not(feature = "model-timestamps"))))]
    let width_worker: Option<Result<Worker, String>> = None;
    let mut worker = if let Some(worker) = width_worker {
        worker?
    } else {
        spawn(
            &live.worker,
            live.device,
            &target_artifact,
            runtime_options,
            timing.timing.clone(),
            0,
        )?
    };
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
        if let Some(artifact) = &prefill_artifact {
            worker.load_additional_artifact(artifact)?;
        }
        if let Some(artifact) = &gemv_artifact {
            worker.load_additional_artifact(artifact)?;
        }
        if let Some(artifact) = &packed_artifact {
            worker.load_additional_artifact(artifact.image.artifact())?;
        }
        if let Some(artifact) = &packed_down_artifact {
            worker.load_additional_artifact(artifact.image.artifact())?;
        }
        if let Some(artifact) = &native_gate_up_artifact {
            worker.load_additional_artifact(artifact.image.artifact())?;
        }
        Ok(())
    })();
    if let Err(error) = admitted {
        let close = worker.close();
        return Err(format!("{error}; worker close: {close:?}"));
    }
    #[cfg(feature = "model-timestamps")]
    let model_timestamps = if model_timestamp_output.is_some() {
        match worker.enable_model_timestamps(model.config().layers) {
            Ok(shared) => Some(shared),
            Err(error) => {
                let close = worker.close();
                return Err(format!("{error}; model timestamp worker close: {close:?}"));
            }
        }
    } else {
        None
    };
    let mut driver = if let Some(candidate) = &native_gate_up_artifact {
        EngineeringTpBatchExecutionV2::new_wide32_with_native_splitk_gate_up_r1(
            vec![worker],
            model.config(),
            model.target_weights(),
            model.layout(),
            &pool,
            &artifacts,
            prefill_artifact
                .as_ref()
                .ok_or("native gate/up V27 missing")?,
            split_artifact
                .as_ref()
                .ok_or("native gate/up V21 missing")?,
            gemv_artifact.as_ref().ok_or("native gate/up V20 missing")?,
            copy_artifact.as_ref().ok_or("native gate/up V19 missing")?,
            &candidate.image,
        )?
    } else if let (Some(packed), Some(prefill), Some(split), Some(gemv)) = (
        &packed_down_artifact,
        &prefill_artifact,
        &split_artifact,
        &gemv_artifact,
    ) {
        EngineeringTpBatchExecutionV2::new_wide32_with_ordered64_packed_down_r1(
            vec![worker],
            model.config(),
            model.target_weights(),
            model.layout(),
            &pool,
            &artifacts,
            prefill,
            split,
            gemv,
            &packed.image,
        )?
    } else if let (Some(packed), Some(prefill), Some(split), Some(gemv), Some(copy)) = (
        &packed_artifact,
        &prefill_artifact,
        &split_artifact,
        &gemv_artifact,
        &copy_artifact,
    ) {
        EngineeringTpBatchExecutionV2::new_wide32_with_ordered64_kv_packed_gate_up_r2(
            vec![worker],
            model.config(),
            model.target_weights(),
            model.layout(),
            &pool,
            &artifacts,
            prefill,
            split,
            gemv,
            copy,
            &packed.image,
        )?
    } else if let (Some(_), Some(copy), Some(prefill), Some(split), Some(gemv)) = (
        composed_copy,
        &copy_artifact,
        &prefill_artifact,
        &split_artifact,
        &gemv_artifact,
    ) {
        #[cfg(all(feature = "c1-ordered64", feature = "model-timestamps"))]
        let constructor = if ordered64_packet_ticks {
            EngineeringTpBatchExecutionV2::new_wide32_with_ordered64_kv_copy_packet_ticks_v1
        } else {
            EngineeringTpBatchExecutionV2::new_wide32_with_ordered64_kv_copy_v1
        };
        #[cfg(not(all(feature = "c1-ordered64", feature = "model-timestamps")))]
        let constructor = EngineeringTpBatchExecutionV2::new_wide32_with_ordered64_kv_copy_v1;
        constructor(
            vec![worker],
            model.config(),
            model.target_weights(),
            model.layout(),
            &pool,
            &artifacts,
            prefill,
            split,
            gemv,
            copy,
        )?
    } else if let (Some(copy), Some(split), Some(gemv)) =
        (&prefill_artifact, &split_artifact, &gemv_artifact)
    {
        EngineeringTpBatchExecutionV2::new_wide32_with_prefill_decode_gemv_v28(
            vec![worker],
            model.config(),
            model.target_weights(),
            model.layout(),
            &pool,
            &artifacts,
            copy,
            split,
            gemv,
        )?
    } else if let (Some(copy), Some(split)) = (&prefill_artifact, &split_artifact) {
        EngineeringTpBatchExecutionV2::new_wide32_with_prefill_decode_v28(
            vec![worker],
            model.config(),
            model.target_weights(),
            model.layout(),
            &pool,
            &artifacts,
            copy,
            split,
        )?
    } else if let Some(copy) = &prefill_artifact {
        EngineeringTpBatchExecutionV2::new_wide32_with_prefill_kv_copy_v28(
            vec![worker],
            model.config(),
            model.target_weights(),
            model.layout(),
            &pool,
            &artifacts,
            copy,
        )?
    } else if let Some(copy) = &copy_artifact {
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
        if let Some(selection) = composed_copy {
            #[cfg(all(feature = "c1-ordered64", feature = "model-timestamps"))]
            let configure = if ordered64_packet_ticks {
                EngineeringTpBatchExecutionV2::configure_ordered64_kv_copy_packet_ticks_v1
            } else {
                EngineeringTpBatchExecutionV2::configure_ordered64_kv_copy_v1
            };
            #[cfg(not(all(feature = "c1-ordered64", feature = "model-timestamps")))]
            let configure = EngineeringTpBatchExecutionV2::configure_ordered64_kv_copy_v1;
            configure(
                &mut driver,
                &artifacts,
                prefill_artifact.as_ref().ok_or("composed V27 missing")?,
                split_artifact.as_ref().ok_or("composed V21 missing")?,
                gemv_artifact.as_ref().ok_or("composed V20 missing")?,
                copy_artifact.as_ref().ok_or("composed V19 missing")?,
                selection.enabled,
            )?;
            if let Some(enabled) = selection.prefill32_pages {
                driver.configure_prefill32_pages_v1(
                    prefill_artifact.as_ref().ok_or("prefill32 V27 missing")?,
                    enabled,
                )?;
                if driver.prefill32_pages_mode() != selection.prefill32_mode() {
                    return Err("prefill32 page copy actual selection differs".into());
                }
            }
            if prefill_width == Some(32) {
                driver.configure_prefill32_program_v1(
                    prefill_artifact
                        .as_ref()
                        .ok_or("native prefill32 V27 missing")?,
                )?;
            }
            if let (Some(selection), Some(candidate)) = (native_gate_up, &native_gate_up_artifact) {
                driver.configure_native_splitk_gate_up_r1(&candidate.image, selection.enabled)?;
            }
            if driver.kv_append_mode() != selection.mode()
                || driver.split_attention_workspace_bytes() != 133_120
            {
                return Err("ordered64 KV copy actual selection differs".into());
            }
        } else {
            match (variant, &copy_artifact, &prefill_artifact) {
                (
                    Variant::SplitAttentionV25 {
                        enabled, packed, ..
                    },
                    None,
                    None,
                ) => {
                    driver.configure_ordered_c1_split_attention_v25(
                        &artifacts,
                        split_artifact
                            .as_ref()
                            .ok_or("V25 controller image missing")?,
                        enabled,
                    )?;
                    if let Some(packed) = packed {
                        driver.configure_c1_split_packet_packing_v25(packed)?;
                    }
                    if driver.split_attention_mode() != variant.split_attention_mode()
                        || driver.c1_packet_mode() != variant.c1_packet_mode()
                        || driver.kv_append_mode() != "baseline"
                    {
                        return Err(
                            "actual V25 attention or historical packet/copy route differs".into(),
                        );
                    }
                }
                (Variant::KvCopyV19 { enabled, .. }, Some(copy), None) => {
                    driver.configure_ordered_c1_kv_copy_v19(&artifacts, copy, enabled)?;
                    if driver.kv_append_mode() != variant.kv_append_mode() {
                        return Err("actual V19 copy policy differs from requested mode".into());
                    }
                }
                (
                    Variant::PrefillKvCopyV28 {
                        enabled,
                        decode: Some(decode),
                        ..
                    },
                    None,
                    Some(copy),
                ) => {
                    let split = split_artifact
                        .as_ref()
                        .ok_or("composed V21 image missing")?;
                    if let Some((_, gemv_enabled)) = decode.gemv {
                        driver.configure_ordered_prefill_decode_gemv_v28(
                            &artifacts,
                            copy,
                            split,
                            gemv_artifact.as_ref().ok_or("composed V20 image missing")?,
                            enabled,
                            decode.split,
                            decode.packed,
                            gemv_enabled.into(),
                        )?;
                    } else {
                        driver.configure_ordered_prefill_decode_v28(
                            &artifacts,
                            copy,
                            split,
                            enabled,
                            decode.split,
                            decode.packed,
                        )?;
                    }
                    if decode.ordered64 {
                        driver.configure_c1_ordered64()?;
                    }
                    if driver.prefill_kv_mode() != variant.prefill_kv_mode()
                        || driver.split_attention_mode() != variant.split_attention_mode()
                        || driver.c1_packet_mode() != variant.c1_packet_mode()
                        || driver.kv_append_mode() != "baseline"
                        || driver.split_attention_workspace_bytes() != 133_120
                    {
                        return Err("V28 composed selection or workspace differs".into());
                    }
                }
                (
                    Variant::PrefillKvCopyV28 {
                        enabled,
                        decode: None,
                        ..
                    },
                    None,
                    Some(copy),
                ) => {
                    driver.configure_ordered_prefill_kv_copy_v28(&artifacts, copy, enabled)?;
                    if driver.prefill_kv_mode() != variant.prefill_kv_mode()
                        || driver.kv_append_mode() != "baseline"
                        || driver.c1_packet_mode() != "baseline"
                        || driver.split_attention_mode() != "baseline"
                        || driver.split_attention_workspace_bytes() != 0
                    {
                        return Err(
                            "V28 copy selection differs or combines another candidate".into()
                        );
                    }
                }
                (
                    Variant::V17 | Variant::RuntimeDiagnostic | Variant::PackedC1V22 { .. },
                    None,
                    None,
                ) => {
                    driver.configure_ordered_c1_wave_target_v17(&artifacts, options.mode)?;
                    if let Variant::PackedC1V22 { enabled } = variant {
                        driver.configure_c1_packet_packing_v22(enabled)?;
                    }
                }
                _ => return Err("copy artifact does not match controller variant".into()),
            }
        }
        driver.configure_host_timing(timing.timing.clone())?;
        #[cfg(all(feature = "c1-ordered64", feature = "model-timestamps"))]
        if baseline_packet_ticks {
            driver.configure_ordered64_baseline_packet_ticks_v1()?;
        }
        if let (Some(selection), Some(packed)) = (packed_gate_up, &packed_artifact) {
            driver.configure_ordered64_kv_packed_gate_up_r2(&packed.image, selection.enabled)?;
        }
        if let (Some(selection), Some(packed)) = (packed_down, &packed_down_artifact) {
            driver.configure_ordered64_packed_down_r1(&packed.image, selection.enabled)?;
        }
        if prefill16_ordered {
            driver.configure_prefill16_ordered_v1(true)?;
            if driver.prefill16_ordered_mode_v1() != "prefill16-ordered64-v1" {
                return Err("prefill16 scheduling selection differs".into());
            }
        }
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
        variant.annotate_prefill_mode(&mut profile, driver.prefill_kv_mode())?;
        variant.annotate_gemv(
            &mut profile,
            gemv_artifact.as_ref(),
            driver.partial_gemv_mode(),
        )?;
        variant.annotate_split_attention(
            &mut profile,
            split_artifact.as_ref(),
            driver.split_attention_mode(),
            driver.split_attention_workspace_bytes(),
        )?;
        if let (Some(selection), Some(packed)) = (packed_gate_up, &packed_artifact) {
            profile["packed_gate_up"] = selection.metadata(
                packed,
                driver.packed_gate_up_mode(),
                driver.packed_gate_up_weight_bytes(),
                driver.packed_gate_up_activation_scratch_bytes(),
                &driver.packed_gate_up_source_identities(),
            )?;
        }
        if let (Some(selection), Some(packed)) = (packed_down, &packed_down_artifact) {
            profile["packed_down"] = selection.metadata(
                packed,
                driver.packed_down_mode(),
                driver.packed_down_weight_bytes(),
                driver.packed_down_activation_scratch_bytes(),
                &driver.packed_down_source_identities(),
            )?;
        }
        if let (Some(selection), Some(candidate)) = (native_gate_up, &native_gate_up_artifact) {
            profile["native_gate_up"] =
                selection.metadata(candidate, driver.native_splitk_gate_up_state_r1())?;
        }
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
        performance_profile["live_profile"] = json!(live_profile);
        performance_profile["runtime_profiling"] = json!(runtime_options.profile);
    }
    if ordered64_host_diagnostic {
        annotate_ordered64_host_diagnostic(&mut performance_profile);
    }
    if ordered64_runtime_counters {
        annotate_ordered64_runtime_counters(&mut performance_profile);
    }
    if active_poll {
        annotate_ordered64_active_poll(&mut performance_profile);
    }
    if prefill16_ordered {
        annotate_prefill16_ordered(&mut performance_profile);
    }
    if let Some(backend) = token_backend {
        annotate_token_program(&mut performance_profile, backend, token_counters);
        if prefill_program {
            annotate_prefill_program(&mut performance_profile, token_counters);
            if let Some(rows) = prefill_width {
                annotate_prefill_width(&mut performance_profile, rows, token_counters);
            }
        }
    }
    if let Some(selection) = native_gate_up {
        let observed = performance_profile["native_gate_up"].clone();
        native_gate_up_live_profile::annotate(
            &mut performance_profile,
            selection,
            &observed,
            token_counters,
        );
    }
    if model_timestamp_output.is_some() {
        performance_profile["model_timestamps"] = json!({
            "command":"dispatch_ordered_batch64_profiled", "max_group_packets":if ordered64_packet_ticks { 64 } else { 16 },
            "original_publication_boundaries_preserved":true,
            "unit":"raw_device_ticks_frequency_unspecified", "performance_qualified":false,
        });
        if ordered64_packet_ticks {
            performance_profile["model_timestamps"]["scope"] = json!(
                "raw packet-processing end_tick_minus_start_tick; not shader-only, calibrated nanoseconds, or wall time"
            );
        }
    }
    if let (Variant::KvCopyV19 { artifact, .. }, Some(copy)) = (variant, &copy_artifact) {
        performance_profile["kv_append_mode"] = json!(variant.kv_append_mode());
        performance_profile["kv_copy_artifact_path"] = json!(artifact);
        performance_profile["kv_copy_artifact"] = artifact_identity(copy);
    }
    if let (Some(selection), Some(copy)) = (composed_copy, &copy_artifact) {
        selection.annotate(&mut performance_profile, copy);
    }
    if let (Variant::PrefillKvCopyV28 { artifact, .. }, Some(copy)) = (variant, &prefill_artifact) {
        performance_profile["prefill_kv_artifact_path"] = json!(artifact);
        performance_profile["prefill_kv_artifact"] = artifact_identity(copy);
    }
    let layer_projection = driver.layer_projection_mode();
    let rmsnorm_mode = driver.rmsnorm_mode();
    let attention_mode = driver.attention_mode();
    let c1_packet_mode = driver.c1_packet_mode();
    let prefill_kv_mode = driver.prefill_kv_mode();
    let gemv_mode = driver.partial_gemv_mode();
    let split_attention_mode = driver.split_attention_mode();
    let split_attention_workspace_bytes = driver.split_attention_workspace_bytes();
    let transposed_weight_bytes = driver.transposed_weight_bytes();
    let fp32_workspace_bytes = driver.fp32_head_workspace_bytes();
    let packed_observation = performance_profile.get("packed_gate_up").cloned();
    let native_gate_up_observation = performance_profile.get("native_gate_up").cloned();
    let packed_down_observation = performance_profile.get("packed_down").cloned();
    let mut runtime =
        EngineeringTpBatchRuntimeV2::new_wide32(driver, pool, scheduler, ROWS, false)?;
    drop(setup_scope);
    let mut setup = json!({
        "schema":"FerricQwen3TpBatchSetupV2", "authority":"none", "performance_qualified":false,
        "serving_qualified":false, "live_profile":live_profile, "wave_target_mode":options.mode.label(),
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
    if let Some(observed) = &packed_observation {
        setup["packed_gate_up"] = observed.clone();
    }
    if let Some(observed) = &packed_down_observation {
        setup["packed_down"] = observed.clone();
    }
    if let (Variant::KvCopyV19 { artifact, .. }, Some(copy)) = (variant, &copy_artifact) {
        setup["kv_append_mode"] = json!(variant.kv_append_mode());
        setup["kv_copy_artifact_path"] = json!(artifact);
        setup["kv_copy_artifact"] = artifact_identity(copy);
    }
    if let (Some(selection), Some(copy)) = (composed_copy, &copy_artifact) {
        selection.annotate(&mut setup, copy);
    }
    if let (Variant::PrefillKvCopyV28 { artifact, .. }, Some(copy)) = (variant, &prefill_artifact) {
        setup["requested_prefill_kv_mode"] = json!(variant.prefill_kv_mode());
        setup["prefill_kv_mode"] = json!(prefill_kv_mode);
        setup["prefill_kv_artifact_path"] = json!(artifact);
        setup["prefill_kv_artifact"] = artifact_identity(copy);
    }
    variant.annotate_packet_mode(&mut setup, c1_packet_mode)?;
    variant.annotate_split_attention(
        &mut setup,
        split_artifact.as_ref(),
        split_attention_mode,
        split_attention_workspace_bytes,
    )?;
    variant.annotate_gemv(&mut setup, gemv_artifact.as_ref(), gemv_mode)?;
    if ordered64_host_diagnostic {
        annotate_ordered64_host_diagnostic(&mut setup);
    }
    if ordered64_runtime_counters {
        annotate_ordered64_runtime_counters(&mut setup);
    }
    if active_poll {
        annotate_ordered64_active_poll(&mut setup);
    }
    if prefill16_ordered {
        annotate_prefill16_ordered(&mut setup);
    }
    if let Some(backend) = token_backend {
        annotate_token_program(&mut setup, backend, token_counters);
        if prefill_program {
            annotate_prefill_program(&mut setup, token_counters);
            if let Some(rows) = prefill_width {
                annotate_prefill_width(&mut setup, rows, token_counters);
            }
        }
    }
    if let (Some(selection), Some(observed)) = (native_gate_up, &native_gate_up_observation) {
        native_gate_up_live_profile::annotate(&mut setup, selection, observed, token_counters);
    }
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
    let (result, close) = if ordered64_runtime_counters {
        run_diagnostic_and_close(
            &mut runtime,
            &timing.timing,
            live.device,
            worker_pid,
            |value| {
                if prefill16_ordered {
                    let mut record = ordered64_counter_record(value);
                    annotate_prefill16_ordered(&mut record);
                    emit_diagnostic(&record)
                } else {
                    emit_diagnostic(&ordered64_counter_record_with_wait(value, active_poll))
                }
            },
            body,
        )
    } else {
        match variant {
            Variant::V17
            | Variant::KvCopyV19 { .. }
            | Variant::PackedC1V22 { .. }
            | Variant::SplitAttentionV25 { .. }
            | Variant::PrefillKvCopyV28 { .. } => run_and_close(&mut runtime, body),
            Variant::RuntimeDiagnostic => run_diagnostic_and_close(
                &mut runtime,
                &timing.timing,
                live.device,
                worker_pid,
                emit_diagnostic,
                body,
            ),
        }
    };
    let mut closed = json!({
        "schema":"FerricQwen3TpBatchClosedV2", "authority":"none", "performance_qualified":false,
        "live_profile":live_profile, "wave_target_mode":options.mode.label(), "layer_projection":layer_projection,
        "submission":live.submission.label(), "rmsnorm_mode":rmsnorm_mode, "attention_mode":attention_mode,
        "benchmark_admitted":false, "serving_admitted":false, "worker_pids":[worker_pid],
        "all_workers_exited":close.is_ok(), "execution_completed":result.is_ok(),
        "rank_dispatch_counts":runtime.dispatch_counts(), "whole_seconds":whole.elapsed().as_secs_f64(),
    });
    if let Some(observed) = &packed_observation {
        closed["packed_gate_up"] = observed.clone();
    }
    if let Some(observed) = &packed_down_observation {
        closed["packed_down"] = observed.clone();
    }
    if let (Variant::KvCopyV19 { artifact, .. }, Some(copy)) = (variant, &copy_artifact) {
        closed["kv_append_mode"] = json!(variant.kv_append_mode());
        closed["kv_copy_artifact_path"] = json!(artifact);
        closed["kv_copy_artifact"] = artifact_identity(copy);
    }
    if let (Some(selection), Some(copy)) = (composed_copy, &copy_artifact) {
        selection.annotate(&mut closed, copy);
    }
    if let (Variant::PrefillKvCopyV28 { artifact, .. }, Some(copy)) = (variant, &prefill_artifact) {
        closed["requested_prefill_kv_mode"] = json!(variant.prefill_kv_mode());
        closed["prefill_kv_mode"] = json!(prefill_kv_mode);
        closed["prefill_kv_artifact_path"] = json!(artifact);
        closed["prefill_kv_artifact"] = artifact_identity(copy);
    }
    variant.annotate_packet_mode(&mut closed, c1_packet_mode)?;
    variant.annotate_split_attention(
        &mut closed,
        split_artifact.as_ref(),
        split_attention_mode,
        split_attention_workspace_bytes,
    )?;
    variant.annotate_gemv(&mut closed, gemv_artifact.as_ref(), gemv_mode)?;
    if ordered64_host_diagnostic {
        annotate_ordered64_host_diagnostic(&mut closed);
    }
    if ordered64_runtime_counters {
        annotate_ordered64_runtime_counters(&mut closed);
    }
    if active_poll {
        annotate_ordered64_active_poll(&mut closed);
    }
    if prefill16_ordered {
        annotate_prefill16_ordered(&mut closed);
    }
    if let Some(backend) = token_backend {
        annotate_token_program(&mut closed, backend, token_counters);
        if prefill_program {
            annotate_prefill_program(&mut closed, token_counters);
            if let Some(rows) = prefill_width {
                annotate_prefill_width(&mut closed, rows, token_counters);
            }
        }
    }
    if let (Some(selection), Some(observed)) = (native_gate_up, &native_gate_up_observation) {
        native_gate_up_live_profile::annotate(&mut closed, selection, observed, token_counters);
    }
    timing.closed = Some(closed.clone());
    let emitted = emit(&closed);
    #[cfg(feature = "model-timestamps")]
    if let (Some(shared), Some(mut file), Some(path)) = (
        model_timestamps,
        model_timestamp_file,
        model_timestamp_output,
    ) {
        if result.is_err() || close.is_err() || emitted.is_err() {
            if let Some(collector) = shared.borrow_mut().as_mut() {
                collector.poison();
            }
        } else {
            let counts = runtime.dispatch_counts();
            if counts.len() != 1 {
                return Err("model timestamp completion rank count differs".into());
            }
            let collector = shared
                .borrow_mut()
                .take()
                .ok_or("model timestamp collector missing")?;
            let capture = collector.finish(counts[0])?;
            let bytes = capture.to_bounded_json()?;
            file.write_all(&bytes)
                .and_then(|()| file.sync_all())
                .map_err(|error| format!("model timestamp output: {error}"))?;
            let raw_sha256 = hex(&Sha256::digest(&bytes));
            let raw_bytes = u64::try_from(bytes.len()).map_err(|_| "model timestamp byte count")?;
            let metadata = std::fs::symlink_metadata(path).map_err(|error| error.to_string())?;
            if !metadata.is_file() || metadata.len() != raw_bytes || hash_file(path)? != raw_sha256
            {
                return Err(
                    "retained model timestamp bytes differ from the completed collector".into(),
                );
            }
            let mut terminal = json!({
                "schema":if ordered64_packet_ticks { "FerricOrdered64PacketTicksClosedV1" } else { "FerricModelTimestampDiagnosticClosedV1" }, "authority":"none",
                "live_profile":live_profile, "performance_qualified":false, "serving_qualified":false,
                "raw_capture_path":path, "raw_capture_sha256":raw_sha256, "raw_capture_bytes":raw_bytes,
                "core_wire_revision":ferric_m1_engineering_execution_v1::model_timestamps::CORE_REVISION,
                "scope":if ordered64_packet_ticks {
                    "raw packet-processing ticks, frequency unspecified; not shader-only time, calibrated nanoseconds, or performance qualification"
                } else {
                    "raw dispatch intervals; not shader-only time, nanoseconds, or performance qualification"
                },
                "numerical_status":"independently compare every emitted model token ID",
                "rank_dispatch_counts":counts, "all_workers_exited":true,
                "summary":capture.summarize(),
            });
            if let (Some(selection), Some(copy)) = (composed_copy, &copy_artifact) {
                selection.annotate(&mut terminal, copy);
            }
            emit(&terminal)?;
        }
    }
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
    if options.live.runtime.ordered64_packet_ticks {
        return Err("packet ticks require their dedicated entry".into());
    }
    variant.validate(options)?;
    let mut timing = TimingFile::create(options.live.host_timing.as_deref())?;
    let result = run_with_timing(
        options,
        &mut timing,
        variant,
        None,
        false,
        None,
        ExecutionMode::Ordinary,
    );
    let sidecar = timing.finish(&result);
    match (result, sidecar) {
        (Ok(()), Ok(())) => Ok(()),
        (result, sidecar) => Err(format!("live run: {result:?}; timing: {sidecar:?}")),
    }
}

#[allow(dead_code)] // Only the separate ordered64 host diagnostic selects this entry.
pub(super) fn run_ordered64_host_diagnostic(
    options: &Options,
    variant: Variant<'_>,
    output: &Path,
    runtime_counters: bool,
) -> Result<(), String> {
    run_ordered64_host_diagnostic_with_wait(options, variant, output, runtime_counters, false)
}

#[allow(dead_code)] // Only the separate ordered64 host diagnostic selects this entry.
pub(super) fn run_ordered64_host_diagnostic_with_wait(
    options: &Options,
    variant: Variant<'_>,
    output: &Path,
    runtime_counters: bool,
    active_poll: bool,
) -> Result<(), String> {
    if active_poll && !runtime_counters {
        return Err("ordered64 active polling requires explicit runtime counters".into());
    }
    run_ordered64_host_diagnostic_with_execution(
        options,
        variant,
        output,
        runtime_counters,
        if active_poll {
            ExecutionMode::ActivePoll10ms
        } else {
            ExecutionMode::Ordinary
        },
    )
}

#[allow(dead_code)] // Only the separate ordered64 host diagnostic selects this entry.
pub(super) fn run_prefill16_ordered_host_diagnostic(
    options: &Options,
    variant: Variant<'_>,
    output: &Path,
    runtime_counters: bool,
) -> Result<(), String> {
    run_ordered64_host_diagnostic_with_execution(
        options,
        variant,
        output,
        runtime_counters,
        ExecutionMode::Prefill16Ordered,
    )
}

fn run_ordered64_host_diagnostic_with_execution(
    options: &Options,
    variant: Variant<'_>,
    output: &Path,
    runtime_counters: bool,
    execution: ExecutionMode<'_>,
) -> Result<(), String> {
    if options.live.runtime.ordered64_packet_ticks {
        return Err("host diagnostic cannot include packet ticks".into());
    }
    validate_ordered64_host_diagnostic(options, variant)?;
    if runtime_counters {
        ordered64_counter_options(options, variant)?;
    }
    if output.as_os_str().is_empty() {
        return Err("ordered64 host diagnostic requires a fresh sidecar path".into());
    }
    let mut timing = TimingFile::create_ordered64(output)?;
    let result = run_with_timing(
        options,
        &mut timing,
        variant,
        None,
        runtime_counters,
        None,
        execution,
    );
    let sidecar = timing.finish(&result);
    match (result, sidecar) {
        (Ok(()), Ok(())) => Ok(()),
        (result, sidecar) => Err(format!(
            "ordered64 host diagnostic: {result:?}; sidecar: {sidecar:?}"
        )),
    }
}

#[cfg(feature = "model-timestamps")]
#[allow(dead_code)] // Only the separate diagnostic executable selects this entry.
pub(super) fn run_model_timestamps(
    options: &Options,
    variant: Variant<'_>,
    output: &Path,
) -> Result<(), String> {
    if options.live.runtime.ordered64_packet_ticks
        || !matches!(variant, Variant::PrefillKvCopyV28 { decode: None, .. })
        || options.live.max_batches > 256
        || output.as_os_str().is_empty()
    {
        return Err(
            "model timestamp diagnostic requires bounded V28 and a fresh output path".into(),
        );
    }
    variant.validate(options)?;
    let mut timing = TimingFile::create(options.live.host_timing.as_deref())?;
    let result = run_with_timing(
        options,
        &mut timing,
        variant,
        Some(output),
        false,
        None,
        ExecutionMode::Ordinary,
    );
    let sidecar = timing.finish(&result);
    match (result, sidecar) {
        (Ok(()), Ok(())) => Ok(()),
        (result, sidecar) => Err(format!(
            "model timestamp run: {result:?}; host timing: {sidecar:?}"
        )),
    }
}

#[allow(dead_code)] // Only the dedicated composed-copy executable selects this entry.
pub(super) fn run_ordered64_kv_copy(
    options: &Options,
    variant: Variant<'_>,
    selection: Ordered64KvCopy<'_>,
) -> Result<(), String> {
    selection.validate(options, variant)?;
    let mut timing = TimingFile::create(None)?;
    let result = run_with_timing(
        options,
        &mut timing,
        variant,
        None,
        false,
        Some(selection),
        ExecutionMode::Ordinary,
    );
    let sidecar = timing.finish(&result);
    match (result, sidecar) {
        (Ok(()), Ok(())) => Ok(()),
        (result, sidecar) => Err(format!(
            "ordered64 KV copy: {result:?}; timing: {sidecar:?}"
        )),
    }
}

#[allow(dead_code)] // Only the separate explicit packed-gate/up executable selects this entry.
pub(super) fn run_packed_gate_up(
    options: &Options,
    variant: Variant<'_>,
    copy: Ordered64KvCopy<'_>,
    selection: &PackedGateUp,
) -> Result<(), String> {
    selection.validate(options, variant)?;
    let mut timing = TimingFile::create(None)?;
    let result = run_with_timing(
        options,
        &mut timing,
        variant,
        None,
        false,
        Some(copy),
        ExecutionMode::PackedGateUp(selection),
    );
    let sidecar = timing.finish(&result);
    match (result, sidecar) {
        (Ok(()), Ok(())) => Ok(()),
        (result, sidecar) => Err(format!("packed gate/up: {result:?}; timing: {sidecar:?}")),
    }
}

#[allow(dead_code)] // Only the separate explicit packed-down executable selects this entry.
pub(super) fn run_packed_down(
    options: &Options,
    variant: Variant<'_>,
    selection: &PackedDown,
) -> Result<(), String> {
    selection.validate(options, variant)?;
    let mut timing = TimingFile::create(None)?;
    let result = run_with_timing(
        options,
        &mut timing,
        variant,
        None,
        false,
        None,
        ExecutionMode::PackedDown(selection),
    );
    let sidecar = timing.finish(&result);
    match (result, sidecar) {
        (Ok(()), Ok(())) => Ok(()),
        (result, sidecar) => Err(format!("packed down: {result:?}; timing: {sidecar:?}")),
    }
}

pub(super) fn validate_packed_host_diagnostic(
    options: &Options,
    variant: Variant<'_>,
    copy: Ordered64KvCopy<'_>,
    selection: &PackedGateUp,
    output: &Path,
) -> Result<(), String> {
    selection.validate(options, variant)?;
    copy.validate(options, variant)?;
    validate_ordered64_host_diagnostic(options, variant)?;
    if !copy.enabled || copy.prefill32_pages.is_some() || !output.is_absolute() {
        return Err(
            "packed host diagnostic requires V19, prefill16 and an absolute sidecar path".into(),
        );
    }
    Ok(())
}

#[allow(dead_code)] // Selected only by the explicit packed host diagnostic argument.
pub(super) fn run_packed_host_diagnostic(
    options: &Options,
    variant: Variant<'_>,
    copy: Ordered64KvCopy<'_>,
    selection: &PackedGateUp,
    output: &Path,
) -> Result<(), String> {
    validate_packed_host_diagnostic(options, variant, copy, selection, output)?;
    let mut timing = TimingFile::create_ordered64(output)?;
    let result = run_with_timing(
        options,
        &mut timing,
        variant,
        None,
        false,
        Some(copy),
        ExecutionMode::PackedGateUpHostDiagnostic(selection),
    );
    let sidecar = timing.finish(&result);
    match (result, sidecar) {
        (Ok(()), Ok(())) => Ok(()),
        (result, sidecar) => Err(format!(
            "packed host diagnostic: {result:?}; timing: {sidecar:?}"
        )),
    }
}

fn validate_token_program(
    options: &Options,
    variant: Variant<'_>,
    selection: Option<Ordered64KvCopy<'_>>,
) -> Result<(), String> {
    let selection = selection.ok_or("token program requires V19 composition")?;
    selection.validate(options, variant)?;
    if !cfg!(all(
        feature = "c1-token-program",
        not(feature = "model-timestamps")
    )) || !selection.enabled
        || selection.prefill32_pages.is_some()
        || options.live.runtime.profile
        || options.live.runtime.ordered64_runtime_counters
        || options.live.runtime.ordered64_packet_ticks
        || options.live.host_timing.is_some()
    {
        return Err(
            "token program requires its separate fixed unprofiled V19/split8 build and entry"
                .into(),
        );
    }
    Ok(())
}

fn prefill_program_profile(counters: bool) -> &'static str {
    if counters {
        "prefill16-native613-decode-native652-counters-v1"
    } else {
        "prefill16-native613-decode-native652-v1"
    }
}

fn prefill_width_profile(rows: u32, counters: bool) -> &'static str {
    match (rows, counters) {
        (16, false) => "prefill16-native613-slots512-worker-decode652-v1",
        (32, false) => "prefill32-native649-slots512-worker-decode652-v1",
        (16, true) => "prefill16-native613-slots512-worker-decode652-counters-v1",
        (32, true) => "prefill32-native649-slots512-worker-decode652-counters-v1",
        _ => "invalid-prefill-width",
    }
}

fn annotate_prefill_width(value: &mut Value, rows: u32, counters: bool) {
    value["live_profile"] = json!(prefill_width_profile(rows, counters));
    value["prefill_chunk"] = json!(rows);
    value["token_program"]["prefill_unchanged"] = json!(false);
    if counters {
        value["token_program"]["counter_schema"] = json!("FerricPrefillWidthProgramCountersV1");
    }
    value["prefill_program"] = json!({
        "mode":"native-prefill-width-slots512-worker-v1", "enabled":true, "rows":rows,
        "dispatches":if rows == 32 { 649 } else { 613 },
        "dynamic_slots":if rows == 32 { 396 } else { 216 },
        "command_family":if rows == 32 { "slots512-v1" } else { "legacy256-v1" },
        "decode_command_family":"legacy256-v1", "output_rows":[[],[rows - 1]],
        "head_singletons_preserved":true,"same_kernel_images":true,
        "transition_policy":"idle-explicit-shape-release-register-v1",
        "fallback":"ordinary for valid ineligible batches", "decode_unchanged":true,
        "performance_qualified":false,"counter_diagnostic":counters
    });
}

#[allow(dead_code)]
pub(super) fn run_native_prefill_width_program(
    options: &Options,
    variant: Variant<'_>,
    selection: Ordered64KvCopy<'_>,
    rows: u32,
    counters: bool,
) -> Result<(), String> {
    validate_token_program(options, variant, Some(selection))?;
    if !matches!(rows, 16 | 32) {
        return Err("native prefill rows must be 16 or 32".into());
    }
    let mut timing = TimingFile::create(None)?;
    let result = run_with_timing(
        options,
        &mut timing,
        variant,
        None,
        false,
        Some(selection),
        ExecutionMode::NativePrefillWidthProgram { rows, counters },
    );
    let sidecar = timing.finish(&result);
    match (result, sidecar) {
        (Ok(()), Ok(())) => Ok(()),
        (result, sidecar) => Err(format!(
            "native prefill width: {result:?}; timing: {sidecar:?}"
        )),
    }
}

#[allow(dead_code)]
pub(super) fn run_native_gate_up_program(
    options: &Options,
    variant: Variant<'_>,
    copy: Ordered64KvCopy<'_>,
    selection: &NativeGateUp,
    counters: bool,
) -> Result<(), String> {
    selection.validate()?;
    validate_token_program(options, variant, Some(copy))?;
    let mut timing = TimingFile::create(None)?;
    let result = run_with_timing(
        options,
        &mut timing,
        variant,
        None,
        false,
        Some(copy),
        ExecutionMode::NativeGateUpProgram {
            selection,
            counters,
        },
    );
    let sidecar = timing.finish(&result);
    match (result, sidecar) {
        (Ok(()), Ok(())) => Ok(()),
        (result, sidecar) => Err(format!("native gate/up: {result:?}; timing: {sidecar:?}")),
    }
}

fn annotate_prefill_program(value: &mut Value, counters: bool) {
    value["live_profile"] = json!(prefill_program_profile(counters));
    value["token_program"]["prefill_unchanged"] = json!(false);
    if counters {
        value["token_program"]["counter_schema"] = json!("FerricPrefillProgramCountersV1");
    }
    value["prefill_program"] = json!({
        "mode":"native-prefill16-program-v1","enabled":true,
        "dispatches":613,"dynamic_slots":216,"rows":16,"output_rows":[[],[15]],
        "head_singletons_preserved":true,"flattened_gpu_commands_unchanged":true,
        "transition_policy":"idle-explicit-shape-release-register-v1",
        "fallback":"ordinary for valid ineligible batches",
        "decode_unchanged":true,"performance_qualified":false,
        "counter_diagnostic":counters
    });
}

#[allow(dead_code)]
pub(super) fn run_native_prefill_program(
    options: &Options,
    variant: Variant<'_>,
    selection: Ordered64KvCopy<'_>,
    counters: bool,
) -> Result<(), String> {
    validate_token_program(options, variant, Some(selection))?;
    let mut timing = TimingFile::create(None)?;
    let result = run_with_timing(
        options,
        &mut timing,
        variant,
        None,
        false,
        Some(selection),
        ExecutionMode::NativePrefillProgram { counters },
    );
    let sidecar = timing.finish(&result);
    match (result, sidecar) {
        (Ok(()), Ok(())) => Ok(()),
        (result, sidecar) => Err(format!(
            "native prefill program: {result:?}; timing: {sidecar:?}"
        )),
    }
}

fn annotate_token_program(value: &mut Value, backend: TokenProgramBackend, counters: bool) {
    value["live_profile"] = json!(if counters {
        backend.counter_profile()
    } else {
        backend.profile()
    });
    value["runtime_profiling"] = json!(counters);
    value["token_program"] = json!({"command":"execute_token_program", "c1_dispatches":652,
        "dynamic_slots":180, "worker_entry":backend.argument(),
        "backend":backend.identity(), "backend_identity_checked":true,
        "backend_fallback":false,
        "counter_diagnostic":counters,
        "context_range":[128, 256], "ordinary_fallback":true,
        "prefill_unchanged":true, "active_poll":false, "performance_qualified":false});
    if counters {
        value["token_program"]["counter_schema"] = json!("FerricTokenProgramCountersV1");
        value["token_program"]["counter_stream"] = json!("controller stderr");
        value["token_program"]["counter_phases"] = json!(["worker_start", "before_close"]);
        value["token_program"]["latency_sample_admitted"] = json!(false);
        value["token_program"]["max_model_batches"] = json!(ORDERED64_HOST_MAX_MODEL_BATCHES);
    }
}

#[allow(dead_code)] // Only the separately named opt-in executable calls this entry.
pub(super) fn run_token_program(
    options: &Options,
    variant: Variant<'_>,
    selection: Ordered64KvCopy<'_>,
) -> Result<(), String> {
    run_token_program_with_backend(
        options,
        variant,
        selection,
        TokenProgramBackend::Ordered64GroupsV1,
        false,
    )
}

#[allow(dead_code)] // Only the separately named whole-token executable calls this entry.
pub(super) fn run_native_token_program(
    options: &Options,
    variant: Variant<'_>,
    selection: Ordered64KvCopy<'_>,
) -> Result<(), String> {
    run_token_program_with_backend(
        options,
        variant,
        selection,
        TokenProgramBackend::NativeWholeProgramV1,
        false,
    )
}

#[allow(dead_code)] // Counter runs are never latency samples.
pub(super) fn run_token_program_counters(
    options: &Options,
    variant: Variant<'_>,
    selection: Ordered64KvCopy<'_>,
    backend: TokenProgramBackend,
) -> Result<(), String> {
    run_token_program_with_backend(options, variant, selection, backend, true)
}

fn run_token_program_with_backend(
    options: &Options,
    variant: Variant<'_>,
    selection: Ordered64KvCopy<'_>,
    backend: TokenProgramBackend,
    counters: bool,
) -> Result<(), String> {
    validate_token_program(options, variant, Some(selection))?;
    let mut timing = TimingFile::create(None)?;
    let result = run_with_timing(
        options,
        &mut timing,
        variant,
        None,
        false,
        Some(selection),
        if counters {
            ExecutionMode::TokenProgramCounters(backend)
        } else {
            ExecutionMode::TokenProgram(backend)
        },
    );
    let sidecar = timing.finish(&result);
    match (result, sidecar) {
        (Ok(()), Ok(())) => Ok(()),
        (result, sidecar) => Err(format!(
            "fixed token program: {result:?}; timing: {sidecar:?}"
        )),
    }
}

#[cfg(feature = "model-timestamps")]
fn create_ordered64_packet_tick_file(path: &Path) -> Result<std::fs::File, String> {
    use rustix::fs::{Mode, OFlags};
    use std::os::unix::fs::MetadataExt;
    let name = path.file_name().ok_or("packet ticks require a file name")?;
    let parent = path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let directory = std::fs::File::from(
        rustix::fs::open(
            parent,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|error| format!("open packet tick parent: {error}"))?,
    );
    if directory
        .metadata()
        .map_err(|error| error.to_string())?
        .uid()
        != rustix::process::geteuid().as_raw()
    {
        return Err("packet tick parent must be owned".into());
    }
    rustix::fs::openat(
        &directory,
        name,
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::RUSR | Mode::WUSR,
    )
    .map(std::fs::File::from)
    .map_err(|error| format!("create packet ticks: {error}"))
}

#[cfg(feature = "model-timestamps")]
#[allow(dead_code)] // Only the separate ordered64 packet-tick executable selects this entry.
pub(super) fn run_ordered64_packet_ticks(
    options: &Options,
    variant: Variant<'_>,
    selection: Ordered64KvCopy<'_>,
    output: &Path,
) -> Result<(), String> {
    if !options.live.runtime.ordered64_packet_ticks || output.as_os_str().is_empty() {
        return Err("packet ticks require an explicit selector and fresh sidecar".into());
    }
    selection.validate_packet_ticks(options, variant)?;
    let mut timing = TimingFile::create(None)?;
    let result = run_with_timing(
        options,
        &mut timing,
        variant,
        Some(output),
        false,
        Some(selection),
        ExecutionMode::Ordinary,
    );
    let sidecar = timing.finish(&result);
    match (result, sidecar) {
        (Ok(()), Ok(())) => Ok(()),
        (result, sidecar) => Err(format!("packet ticks: {result:?}; timing: {sidecar:?}")),
    }
}

#[cfg(feature = "model-timestamps")]
#[allow(dead_code)] // Only the separately named baseline-KV diagnostic selects this entry.
pub(super) fn run_ordered64_baseline_packet_ticks(
    options: &Options,
    variant: Variant<'_>,
    output: &Path,
) -> Result<(), String> {
    validate_ordered64_host_diagnostic(options, variant)?;
    if !options.live.runtime.ordered64_packet_ticks || output.as_os_str().is_empty() {
        return Err("baseline packet ticks require an explicit selector and fresh sidecar".into());
    }
    let mut timing = TimingFile::create(None)?;
    let result = run_with_timing(
        options,
        &mut timing,
        variant,
        Some(output),
        false,
        None,
        ExecutionMode::BaselinePacketTicks,
    );
    let sidecar = timing.finish(&result);
    match (result, sidecar) {
        (Ok(()), Ok(())) => Ok(()),
        (result, sidecar) => Err(format!(
            "baseline packet ticks: {result:?}; timing: {sidecar:?}"
        )),
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

    fn ordered64_copy_variant() -> Variant<'static> {
        Variant::PrefillKvCopyV28 {
            artifact: Path::new("prefill"),
            enabled: true,
            decode: Some(DecodeComposition {
                artifact: Path::new("split"),
                split: true,
                packed: true,
                ordered64: true,
                gemv: Some((Path::new("gemv"), false)),
            }),
        }
    }

    #[test]
    fn width_metadata_binds_chunk_graph_family_and_same_decode() {
        for rows in [16, 32] {
            for counters in [false, true] {
                let mut value = json!({});
                annotate_token_program(
                    &mut value,
                    TokenProgramBackend::NativeWholeProgramSlots512V1,
                    counters,
                );
                annotate_prefill_width(&mut value, rows, counters);
                assert_eq!(value["prefill_chunk"], rows);
                assert_eq!(value["live_profile"], prefill_width_profile(rows, counters));
                assert_eq!(
                    value["token_program"]["backend"],
                    "native-whole-program-slots512-v1"
                );
                assert_eq!(value["token_program"]["command"], "execute_token_program");
                assert_eq!(value["token_program"]["c1_dispatches"], 652);
                assert_eq!(value["token_program"]["dynamic_slots"], 180);
                assert_eq!(
                    value["prefill_program"]["dispatches"],
                    if rows == 32 { 649 } else { 613 }
                );
                assert_eq!(
                    value["prefill_program"]["dynamic_slots"],
                    if rows == 32 { 396 } else { 216 }
                );
                assert_eq!(
                    value["prefill_program"]["command_family"],
                    if rows == 32 {
                        "slots512-v1"
                    } else {
                        "legacy256-v1"
                    }
                );
                assert_eq!(
                    value["prefill_program"]["decode_command_family"],
                    "legacy256-v1"
                );
                assert_eq!(value["prefill_program"]["performance_qualified"], false);
                assert_eq!(value["prefill_program"]["counter_diagnostic"], counters);
            }
        }
    }

    #[test]
    fn token_program_entry_requires_its_feature_and_uninstrumented_v19_split8_composition() {
        let mut options = fixture(wave_target_v17_live_contract::Mode::Combined);
        let variant = ordered64_copy_variant();
        let selection = Ordered64KvCopy {
            artifact: Path::new("copy"),
            enabled: true,
            prefill32_pages: None,
        };
        assert_eq!(
            validate_token_program(&options, variant, Some(selection)).is_ok(),
            cfg!(all(
                feature = "c1-token-program",
                not(feature = "model-timestamps")
            ))
        );
        assert!(validate_token_program(&options, variant, None).is_err());
        assert!(
            validate_token_program(
                &options,
                variant,
                Some(Ordered64KvCopy {
                    enabled: false,
                    ..selection
                })
            )
            .is_err()
        );
        assert!(
            validate_token_program(
                &options,
                variant,
                Some(Ordered64KvCopy {
                    prefill32_pages: Some(true),
                    ..selection
                })
            )
            .is_err()
        );
        options.live.host_timing = Some("unused-timing".into());
        assert!(validate_token_program(&options, variant, Some(selection)).is_err());
        options.live.host_timing = None;
        options.live.runtime.ordered64_runtime_counters = true;
        assert!(validate_token_program(&options, variant, Some(selection)).is_err());
        let mut record = json!({});
        annotate_token_program(&mut record, TokenProgramBackend::Ordered64GroupsV1, false);
        assert_eq!(record["token_program"]["context_range"], json!([128, 256]));
        assert_eq!(record["token_program"]["ordinary_fallback"], true);
        assert_eq!(record["token_program"]["performance_qualified"], false);
    }

    #[test]
    fn prefill_program_metadata_is_explicit_and_preserves_decode_identity() {
        for counters in [false, true] {
            let mut baseline = json!({});
            annotate_token_program(
                &mut baseline,
                TokenProgramBackend::NativeWholeProgramV1,
                counters,
            );
            let mut candidate = baseline.clone();
            annotate_prefill_program(&mut candidate, counters);
            assert_ne!(candidate["live_profile"], baseline["live_profile"]);
            assert_eq!(baseline["token_program"]["prefill_unchanged"], true);
            assert_eq!(candidate["token_program"]["prefill_unchanged"], false);
            for field in [
                "backend",
                "worker_entry",
                "c1_dispatches",
                "dynamic_slots",
                "context_range",
                "backend_identity_checked",
                "backend_fallback",
                "active_poll",
            ] {
                assert_eq!(
                    candidate["token_program"][field],
                    baseline["token_program"][field]
                );
            }
            assert_eq!(candidate["prefill_program"]["dispatches"], 613);
            assert_eq!(candidate["prefill_program"]["dynamic_slots"], 216);
            assert_eq!(candidate["prefill_program"]["counter_diagnostic"], counters);
            assert_eq!(candidate["prefill_program"]["performance_qualified"], false);
            if counters {
                assert_eq!(
                    candidate["token_program"]["counter_schema"],
                    "FerricPrefillProgramCountersV1"
                );
                assert_eq!(candidate["token_program"]["latency_sample_admitted"], false);
            }
        }
    }

    #[test]
    fn native_token_metadata_changes_only_backend_and_profile_not_the_model_graph() {
        let mut baseline = json!({"dispatches":652, "source":"unchanged"});
        let mut candidate = baseline.clone();
        annotate_token_program(&mut baseline, TokenProgramBackend::Ordered64GroupsV1, false);
        annotate_token_program(
            &mut candidate,
            TokenProgramBackend::NativeWholeProgramV1,
            false,
        );
        assert_ne!(baseline["live_profile"], candidate["live_profile"]);
        assert_ne!(
            baseline["token_program"]["backend"],
            candidate["token_program"]["backend"]
        );
        assert_ne!(
            baseline["token_program"]["worker_entry"],
            candidate["token_program"]["worker_entry"]
        );
        assert_eq!(candidate["token_program"]["backend_identity_checked"], true);
        assert_eq!(candidate["token_program"]["backend_fallback"], false);
        assert_eq!(candidate["token_program"]["active_poll"], false);
        assert_eq!(candidate["token_program"]["c1_dispatches"], 652);
        assert_eq!(candidate["token_program"]["dynamic_slots"], 180);
        candidate["live_profile"] = baseline["live_profile"].clone();
        candidate["token_program"]["backend"] = baseline["token_program"]["backend"].clone();
        candidate["token_program"]["worker_entry"] =
            baseline["token_program"]["worker_entry"].clone();
        assert_eq!(baseline, candidate);
    }

    #[test]
    fn token_counter_metadata_is_explicitly_not_an_uninstrumented_latency_sample() {
        for backend in [
            TokenProgramBackend::Ordered64GroupsV1,
            TokenProgramBackend::NativeWholeProgramV1,
        ] {
            let mut value = json!({});
            annotate_token_program(&mut value, backend, true);
            assert_eq!(value["live_profile"], backend.counter_profile());
            assert_ne!(value["live_profile"], backend.profile());
            assert_eq!(value["runtime_profiling"], true);
            assert_eq!(value["token_program"]["counter_diagnostic"], true);
            assert_eq!(value["token_program"]["latency_sample_admitted"], false);
            assert_eq!(
                value["token_program"]["counter_phases"],
                json!(["worker_start", "before_close"])
            );
            assert_eq!(value["token_program"]["max_model_batches"], 256);
        }
    }

    #[test]
    fn ordered64_copy_runtime_is_uninstrumented_and_has_no_diagnostic_batch_cap() {
        let options = fixture(wave_target_v17_live_contract::Mode::Combined);
        let variant = ordered64_copy_variant();
        for enabled in [false, true] {
            let selection = Ordered64KvCopy {
                artifact: Path::new("copy"),
                enabled,
                prefill32_pages: None,
            };
            let result = selection.validate(&options, variant);
            assert_eq!(
                result.is_ok(),
                cfg!(all(
                    feature = "c1-ordered64",
                    not(feature = "model-timestamps")
                ))
            );
            if result.is_ok() {
                let runtime = variant.worker_runtime(&options).unwrap();
                assert!(runtime.ordered64 && !runtime.profile);
                assert!(options.live.max_batches > ORDERED64_HOST_MAX_MODEL_BATCHES);
                assert_eq!(
                    selection.mode(),
                    if enabled {
                        "parallel-c1-v19"
                    } else {
                        "baseline"
                    }
                );
                assert_ne!(variant.live_profile(), ORDERED64_KV_COPY_PROFILE);
            }
            assert!(selection.validate(&options, Variant::V17).is_err());
            let mut diagnostic = fixture(wave_target_v17_live_contract::Mode::Combined);
            diagnostic.live.host_timing = Some("forbidden".into());
            assert!(selection.validate(&diagnostic, variant).is_err());
            diagnostic.live.host_timing = None;
            diagnostic.live.runtime.profile = true;
            assert!(selection.validate(&diagnostic, variant).is_err());
        }
    }

    #[test]
    fn prefill32_copy_profile_selects_genuine_chunk32_and_rejects_hybrids() {
        let mut options = fixture(wave_target_v17_live_contract::Mode::Combined);
        let variant = ordered64_copy_variant();
        for enabled in [false, true] {
            let selection = Ordered64KvCopy {
                artifact: Path::new("copy"),
                enabled: true,
                prefill32_pages: Some(enabled),
            };
            assert_eq!(selection.prefill_chunk(), 32);
            assert_eq!(selection.live_profile(), PREFILL32_PAGES_PROFILE);
            assert_eq!(
                selection.prefill32_mode(),
                if enabled {
                    "parallel-prefill32-two-pages-v27"
                } else {
                    "baseline"
                }
            );
            assert_eq!(
                selection.validate(&options, variant).is_ok(),
                cfg!(all(
                    feature = "c1-ordered64",
                    not(feature = "model-timestamps")
                ))
            );
            let legacy = Ordered64KvCopy {
                prefill32_pages: None,
                ..selection
            };
            assert_eq!(legacy.prefill_chunk(), CHUNK);
            assert_eq!(legacy.live_profile(), ORDERED64_KV_COPY_PROFILE);
            assert!(
                Ordered64KvCopy {
                    enabled: false,
                    ..selection
                }
                .validate(&options, variant)
                .is_err()
            );
            options.live.context = 31;
            assert!(selection.validate(&options, variant).is_err());
            options.live.context = 8192;
            options.live.pages = 1;
            assert!(selection.validate(&options, variant).is_err());
            options.live.pages = 512;
        }
    }

    #[test]
    #[ignore = "requires retained actual V19 image; CPU artifact metadata only"]
    fn ordered64_copy_metadata_binds_actual_v19_in_both_modes() {
        let path = PathBuf::from(
            std::env::var_os("FERRIC_TEST_C1_KV_COPY_V19_ARTIFACT").expect("explicit V19 artifact"),
        );
        let image = EngineeringTpArtifactV1::open_c1_kv_copy_v19(&path).unwrap();
        for enabled in [false, true] {
            let selection = Ordered64KvCopy {
                artifact: &path,
                enabled,
                prefill32_pages: None,
            };
            let mut value = json!({"authority":"none", "performance_qualified":false, "live_profile":ORDERED64_KV_COPY_PROFILE});
            selection.annotate(&mut value, &image);
            assert_eq!(value["requested_kv_copy_mode"], selection.mode());
            assert_eq!(value["kv_copy_mode"], selection.mode());
            assert_eq!(value["kv_append_mode"], selection.mode());
            assert_eq!(value["kv_copy_artifact_path"], json!(path));
            assert_eq!(value["kv_copy_artifact"], artifact_identity(&image));
            assert_eq!(value["performance_qualified"], false);
            assert!(value.get("prefill32_pages_mode").is_none());
            assert!(value.get("requested_prefill32_pages_mode").is_none());
            assert!(value.get("prefill_chunk").is_none());
            if enabled {
                for prefill_enabled in [false, true] {
                    let selection = Ordered64KvCopy {
                        prefill32_pages: Some(prefill_enabled),
                        ..selection
                    };
                    let mut selected = json!({"authority":"none", "performance_qualified":false,
                        "live_profile":selection.live_profile(), "prefill_kv_mode":"parallel-prefill16-v27"});
                    selection.annotate(&mut selected, &image);
                    assert_eq!(
                        selected["requested_prefill32_pages_mode"],
                        selection.prefill32_mode()
                    );
                    assert_eq!(selected["prefill32_pages_mode"], selection.prefill32_mode());
                    assert_eq!(selected["prefill_chunk"], 32);
                    assert_eq!(selected["live_profile"], PREFILL32_PAGES_PROFILE);
                    assert_eq!(selected["prefill_kv_mode"], "parallel-prefill16-v27");
                    assert_eq!(selected["kv_copy_mode"], "parallel-c1-v19");
                    assert_eq!(selected["kv_copy_artifact"], artifact_identity(&image));
                    assert_eq!(selected["performance_qualified"], false);
                }
            }
        }
    }

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
                packed: None,
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
                    enabled,
                    packed: None,
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
    fn v25_packet_composition_records_both_selectors_without_changing_legacy_receipts() {
        let options = fixture(wave_target_v17_live_contract::Mode::Combined);
        for enabled in [false, true] {
            for packed in [None, Some(false), Some(true)] {
                let variant = Variant::SplitAttentionV25 {
                    artifact: Path::new("split"),
                    enabled,
                    packed,
                };
                assert!(variant.validate(&options).is_ok());
                let mode = if packed == Some(true) {
                    "packed16-v22"
                } else {
                    "baseline"
                };
                assert_eq!(variant.c1_packet_mode(), mode);
                assert_eq!(
                    variant.split_attention_mode(),
                    if enabled { "split8-v21" } else { "baseline" }
                );
                assert_eq!(variant.live_profile(), "c1-split-attention-v25-live-v1");
                let mut event = json!({"authority":"none"});
                variant.annotate_packet_mode(&mut event, mode).unwrap();
                if packed.is_some() {
                    assert_eq!(event["requested_c1_packet_mode"], mode);
                    assert_eq!(event["c1_packet_mode"], mode);
                    let before = event.clone();
                    assert!(variant.annotate_packet_mode(&mut event, "wrong").is_err());
                    assert_eq!(event, before);
                } else {
                    assert_eq!(event, json!({"authority":"none"}));
                }
                let runtime = variant.worker_runtime(&options).unwrap();
                assert!(!runtime.profile && runtime.ordered_batches && !runtime.sequences);
            }
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
                packed: None,
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
    #[test]
    fn v28_runner_is_separate_unprofiled_and_preserves_legacy_metadata() {
        for enabled in [false, true] {
            let variant = Variant::PrefillKvCopyV28 {
                artifact: Path::new("copy"),
                enabled,
                decode: None,
            };
            let options = fixture(wave_target_v17_live_contract::Mode::Combined);
            assert_eq!(variant.live_profile(), "prefill16-kv-copy-v28-live-v1");
            assert!(!variant.worker_runtime(&options).unwrap().profile);
            assert_eq!(variant.kv_append_mode(), "baseline");
            assert_eq!(variant.c1_packet_mode(), "baseline");
            let actual = if enabled {
                "parallel-prefill16-v27"
            } else {
                "baseline"
            };
            let mut value = json!({"existing":true});
            variant.annotate_prefill_mode(&mut value, actual).unwrap();
            assert_eq!(value["requested_prefill_kv_mode"], actual);
            assert_eq!(value["prefill_kv_mode"], actual);
            assert!(variant.annotate_prefill_mode(&mut value, "other").is_err());
            for old in [
                Variant::V17,
                Variant::RuntimeDiagnostic,
                Variant::PackedC1V22 { enabled: true },
                Variant::KvCopyV19 {
                    artifact: Path::new("old"),
                    enabled: true,
                },
                Variant::SplitAttentionV25 {
                    artifact: Path::new("old"),
                    enabled: true,
                    packed: Some(true),
                },
            ] {
                let mut legacy = json!({"existing":true});
                old.annotate_prefill_mode(&mut legacy, "baseline").unwrap();
                assert_eq!(legacy, json!({"existing":true}));
            }
            for mode in MODES {
                assert_eq!(
                    variant.validate(&fixture(mode)).is_ok(),
                    mode == wave_target_v17_live_contract::Mode::Combined
                );
            }
        }
        assert!(
            Variant::PrefillKvCopyV28 {
                artifact: Path::new(""),
                enabled: true,
                decode: None,
            }
            .validate(&fixture(wave_target_v17_live_contract::Mode::Combined))
            .is_err()
        );
    }

    #[test]
    fn v28_composition_runner_has_distinct_profile_and_three_explicit_modes() {
        let options = fixture(wave_target_v17_live_contract::Mode::Combined);
        for enabled in [false, true] {
            for split in [false, true] {
                for packed in [false, true] {
                    let variant = Variant::PrefillKvCopyV28 {
                        artifact: Path::new("copy"),
                        enabled,
                        decode: Some(DecodeComposition {
                            artifact: Path::new("split"),
                            split,
                            packed,
                            ordered64: false,
                            gemv: None,
                        }),
                    };
                    assert_eq!(
                        variant.live_profile(),
                        "prefill16-decode-composed-v28-live-v1"
                    );
                    let runtime = variant.worker_runtime(&options).unwrap();
                    assert!(!runtime.profile && runtime.ordered_batches && !runtime.sequences);
                    assert_eq!(variant.kv_append_mode(), "baseline");
                    assert_eq!(
                        variant.prefill_kv_mode(),
                        if enabled {
                            "parallel-prefill16-v27"
                        } else {
                            "baseline"
                        }
                    );
                    assert_eq!(
                        variant.split_attention_mode(),
                        if split { "split8-v21" } else { "baseline" }
                    );
                    assert_eq!(
                        variant.c1_packet_mode(),
                        if packed { "packed16-v22" } else { "baseline" }
                    );
                    let mut value = json!({"authority":"none"});
                    variant
                        .annotate_prefill_mode(&mut value, variant.prefill_kv_mode())
                        .unwrap();
                    variant
                        .annotate_packet_mode(&mut value, variant.c1_packet_mode())
                        .unwrap();
                    assert_eq!(
                        value["requested_prefill_kv_mode"],
                        variant.prefill_kv_mode()
                    );
                    assert_eq!(value["requested_c1_packet_mode"], variant.c1_packet_mode());
                    assert!(
                        variant
                            .annotate_split_attention(
                                &mut value,
                                None,
                                variant.split_attention_mode(),
                                133_120
                            )
                            .is_err()
                    );
                    let mut timed = fixture(wave_target_v17_live_contract::Mode::Combined);
                    timed.live.host_timing = Some("timing".into());
                    assert!(variant.validate(&timed).is_err());
                    #[cfg(feature = "model-timestamps")]
                    assert!(
                        run_model_timestamps(&options, variant, Path::new("must-not-be-created"))
                            .is_err()
                    );
                }
            }
        }
        let legacy = Variant::PrefillKvCopyV28 {
            artifact: Path::new("copy"),
            enabled: true,
            decode: None,
        };
        let mut value = json!({"existing":true});
        legacy.annotate_packet_mode(&mut value, "baseline").unwrap();
        legacy
            .annotate_split_attention(&mut value, None, "baseline", 0)
            .unwrap();
        assert_eq!(value, json!({"existing":true}));
        let missing = Variant::PrefillKvCopyV28 {
            artifact: Path::new("copy"),
            enabled: false,
            decode: Some(DecodeComposition {
                artifact: Path::new(""),
                split: false,
                packed: false,
                ordered64: false,
                gemv: None,
            }),
        };
        assert!(missing.validate(&options).is_err());
    }

    #[test]
    #[ignore = "requires both actual canonical V21 and V27 images; CPU metadata validation only"]
    fn v28_composition_metadata_binds_both_actual_images_for_all_eight_modes() {
        let split_path = PathBuf::from(
            std::env::var_os("FERRIC_TEST_SPLIT_ATTENTION_V21_ARTIFACT")
                .expect("explicit V21 image"),
        );
        let copy_path = PathBuf::from(
            std::env::var_os("FERRIC_TEST_PREFILL_COPY_V27_ARTIFACT").expect("explicit V27 image"),
        );
        let split_image = EngineeringTpArtifactV1::open_split_attention_v21(&split_path).unwrap();
        let copy_image = EngineeringTpArtifactV1::open_prefill_kv_copy_v27(&copy_path).unwrap();
        for enabled in [false, true] {
            for split in [false, true] {
                for packed in [false, true] {
                    let variant = Variant::PrefillKvCopyV28 {
                        artifact: &copy_path,
                        enabled,
                        decode: Some(DecodeComposition {
                            artifact: &split_path,
                            split,
                            packed,
                            ordered64: false,
                            gemv: None,
                        }),
                    };
                    let mut value = json!({"live_profile":variant.live_profile(), "prefill_kv_artifact":artifact_identity(&copy_image), "prefill_kv_artifact_path":copy_path});
                    variant
                        .annotate_prefill_mode(&mut value, variant.prefill_kv_mode())
                        .unwrap();
                    variant
                        .annotate_packet_mode(&mut value, variant.c1_packet_mode())
                        .unwrap();
                    variant
                        .annotate_split_attention(
                            &mut value,
                            Some(&split_image),
                            variant.split_attention_mode(),
                            133_120,
                        )
                        .unwrap();
                    assert_eq!(
                        value["split_attention_artifact"],
                        artifact_identity(&split_image)
                    );
                    assert_eq!(value["split_attention_artifact_path"], json!(split_path));
                    assert_eq!(value["split_attention_workspace_bytes"], 133_120);
                    assert_eq!(
                        value["requested_split_attention_mode"],
                        value["split_attention_mode"]
                    );
                    assert_eq!(value["requested_prefill_kv_mode"], value["prefill_kv_mode"]);
                    assert_eq!(value["requested_c1_packet_mode"], value["c1_packet_mode"]);
                    assert_eq!(
                        value["split_attention_policy"]["split_packets_with_head"],
                        652
                    );
                    assert!(
                        variant
                            .annotate_split_attention(
                                &mut value,
                                Some(&split_image),
                                "wrong",
                                133_120
                            )
                            .is_err()
                    );
                    assert!(
                        variant
                            .annotate_split_attention(
                                &mut value,
                                Some(&split_image),
                                variant.split_attention_mode(),
                                0
                            )
                            .is_err()
                    );
                }
            }
        }
    }

    #[test]
    fn v28_partial_gemv_runner_is_explicit_unprofiled_and_keeps_legacy_metadata() {
        let options = fixture(wave_target_v17_live_contract::Mode::Combined);
        for enabled in [false, true] {
            let variant = Variant::PrefillKvCopyV28 {
                artifact: Path::new("copy"),
                enabled: true,
                decode: Some(DecodeComposition {
                    artifact: Path::new("split"),
                    split: true,
                    packed: true,
                    ordered64: false,
                    gemv: Some((Path::new("gemv"), enabled)),
                }),
            };
            assert_eq!(
                variant.live_profile(),
                "prefill16-decode-partial-gemv-v28-live-v1"
            );
            assert!(!variant.worker_runtime(&options).unwrap().profile);
            assert_eq!(variant.prefill_kv_mode(), "parallel-prefill16-v27");
            assert_eq!(variant.split_attention_mode(), "split8-v21");
            assert_eq!(variant.c1_packet_mode(), "packed16-v22");
            assert!(
                variant
                    .annotate_gemv(
                        &mut json!({}),
                        None,
                        if enabled {
                            "partial-prefetch4-v20"
                        } else {
                            "baseline"
                        }
                    )
                    .is_err()
            );
            let mut timed = fixture(wave_target_v17_live_contract::Mode::Combined);
            timed.live.host_timing = Some("timing".into());
            assert!(variant.validate(&timed).is_err());
            #[cfg(feature = "model-timestamps")]
            assert!(
                run_model_timestamps(&options, variant, Path::new("must-not-be-created")).is_err()
            );
        }
        for variant in [
            Variant::V17,
            Variant::RuntimeDiagnostic,
            Variant::PackedC1V22 { enabled: true },
            Variant::KvCopyV19 {
                artifact: Path::new("old"),
                enabled: true,
            },
            Variant::SplitAttentionV25 {
                artifact: Path::new("old"),
                enabled: true,
                packed: Some(true),
            },
            Variant::PrefillKvCopyV28 {
                artifact: Path::new("copy"),
                enabled: true,
                decode: None,
            },
            Variant::PrefillKvCopyV28 {
                artifact: Path::new("copy"),
                enabled: true,
                decode: Some(DecodeComposition {
                    artifact: Path::new("split"),
                    split: true,
                    packed: true,
                    ordered64: false,
                    gemv: None,
                }),
            },
        ] {
            let mut value = json!({"existing":true});
            variant.annotate_gemv(&mut value, None, "baseline").unwrap();
            assert_eq!(value, json!({"existing":true}));
        }
        let empty = Variant::PrefillKvCopyV28 {
            artifact: Path::new("copy"),
            enabled: true,
            decode: Some(DecodeComposition {
                artifact: Path::new("split"),
                split: true,
                packed: true,
                ordered64: false,
                gemv: Some((Path::new(""), true)),
            }),
        };
        assert!(empty.validate(&options).is_err());
    }

    #[test]
    #[ignore = "requires actual canonical V20, V21 and V27 images; CPU metadata validation only"]
    fn v28_partial_gemv_metadata_binds_actual_eighth_image_for_both_modes() {
        let gemv_path = PathBuf::from(
            std::env::var_os("FERRIC_TEST_GEMV_PREFETCH_V20_ARTIFACT").expect("explicit V20 image"),
        );
        let split_path = PathBuf::from(
            std::env::var_os("FERRIC_TEST_SPLIT_ATTENTION_V21_ARTIFACT")
                .expect("explicit V21 image"),
        );
        let copy_path = PathBuf::from(
            std::env::var_os("FERRIC_TEST_PREFILL_COPY_V27_ARTIFACT").expect("explicit V27 image"),
        );
        let gemv = EngineeringTpArtifactV1::open_gemv_prefetch_v20(&gemv_path).unwrap();
        let split = EngineeringTpArtifactV1::open_split_attention_v21(&split_path).unwrap();
        let copy = EngineeringTpArtifactV1::open_prefill_kv_copy_v27(&copy_path).unwrap();
        for enabled in [false, true] {
            let variant = Variant::PrefillKvCopyV28 {
                artifact: &copy_path,
                enabled: true,
                decode: Some(DecodeComposition {
                    artifact: &split_path,
                    split: true,
                    packed: true,
                    ordered64: false,
                    gemv: Some((&gemv_path, enabled)),
                }),
            };
            let actual = if enabled {
                "partial-prefetch4-v20"
            } else {
                "baseline"
            };
            let mut value =
                json!({"prefill_kv_artifact":artifact_identity(&copy), "existing":true});
            variant
                .annotate_prefill_mode(&mut value, "parallel-prefill16-v27")
                .unwrap();
            variant
                .annotate_packet_mode(&mut value, "packed16-v22")
                .unwrap();
            variant
                .annotate_split_attention(&mut value, Some(&split), "split8-v21", 133_120)
                .unwrap();
            variant
                .annotate_gemv(&mut value, Some(&gemv), actual)
                .unwrap();
            assert_eq!(value["requested_gemv_mode"], actual);
            assert_eq!(value["gemv_mode"], actual);
            assert_eq!(value["gemv_artifact_path"], json!(gemv_path));
            assert_eq!(value["gemv_artifact"], artifact_identity(&gemv));
            assert_eq!(value["split_attention_workspace_bytes"], 133_120);
            assert_eq!(value["existing"], true);
            assert!(
                variant
                    .annotate_gemv(&mut value, Some(&gemv), "wrong")
                    .is_err()
            );
            assert!(
                variant
                    .annotate_gemv(&mut value, Some(&split), actual)
                    .is_err()
            );
            assert!(
                variant
                    .annotate_gemv(&mut value, Some(&copy), actual)
                    .is_err()
            );
        }
    }

    #[test]
    fn ordered64_profile_is_distinct_without_changing_kernel_selectors() {
        let options = fixture(wave_target_v17_live_contract::Mode::Combined);
        assert!(!Variant::V17.worker_runtime(&options).unwrap().ordered64);
        for gemv in [
            None,
            Some((Path::new("gemv"), false)),
            Some((Path::new("gemv"), true)),
        ] {
            let variant = Variant::PrefillKvCopyV28 {
                artifact: Path::new("copy"),
                enabled: true,
                decode: Some(DecodeComposition {
                    artifact: Path::new("split"),
                    split: true,
                    packed: true,
                    ordered64: true,
                    gemv,
                }),
            };
            assert_eq!(
                variant.live_profile(),
                "prefill16-decode-ordered64-v29-live-v1"
            );
            assert_eq!(variant.c1_packet_mode(), "packed64-v29");
            assert_eq!(variant.prefill_kv_mode(), "parallel-prefill16-v27");
            assert_eq!(variant.split_attention_mode(), "split8-v21");
            assert_eq!(variant.gemv(), gemv);
            let result = variant.worker_runtime(&options);
            if cfg!(feature = "c1-ordered64") && !cfg!(feature = "model-timestamps") {
                let runtime = result.unwrap();
                assert!(runtime.ordered64 && runtime.ordered_batches);
                assert!(!runtime.sequences && !runtime.profile);
            } else {
                assert!(result.is_err());
            }
            let mut value = json!({});
            variant
                .annotate_packet_mode(&mut value, "packed64-v29")
                .unwrap();
            assert_eq!(value["ordered_wire_mode"], "ordered64");
            assert_eq!(value["packed_c1_group_bound"], 64);
            assert_eq!(value["non_c1_group_bound"], 16);
            assert!(
                variant
                    .annotate_packet_mode(&mut json!({}), "packed16-v22")
                    .is_err()
            );
            let Variant::PrefillKvCopyV28 {
                artifact,
                enabled,
                decode: Some(mut decode),
            } = variant
            else {
                unreachable!()
            };
            decode.packed = false;
            assert!(
                Variant::PrefillKvCopyV28 {
                    artifact,
                    enabled,
                    decode: Some(decode)
                }
                .worker_runtime(&options)
                .is_err()
            );
        }
    }

    #[test]
    fn ordered64_host_diagnostic_is_closed_without_changing_live_or_runtime_options() {
        let mut options = fixture(wave_target_v17_live_contract::Mode::Combined);
        options.live.max_batches = 135;
        let variant = Variant::PrefillKvCopyV28 {
            artifact: Path::new("copy"),
            enabled: true,
            decode: Some(DecodeComposition {
                artifact: Path::new("split"),
                split: true,
                packed: true,
                ordered64: true,
                gemv: Some((Path::new("gemv"), false)),
            }),
        };
        if !cfg!(feature = "c1-ordered64") || cfg!(feature = "model-timestamps") {
            assert!(validate_ordered64_host_diagnostic(&options, variant).is_err());
            return;
        }
        validate_ordered64_host_diagnostic(&options, variant).unwrap();
        assert_eq!(
            variant.live_profile(),
            "prefill16-decode-ordered64-v29-live-v1"
        );
        assert_ne!(variant.live_profile(), ORDERED64_HOST_DIAGNOSTIC_PROFILE);
        let runtime = variant.worker_runtime(&options).unwrap();
        assert!(runtime.ordered64 && runtime.ordered_batches);
        assert!(!runtime.profile && !runtime.sequences);
        assert!(options.live.host_timing.is_none());
        for count in [0, 257, u64::MAX] {
            options.live.max_batches = count;
            assert!(validate_ordered64_host_diagnostic(&options, variant).is_err());
        }
        options.live.max_batches = 256;
        validate_ordered64_host_diagnostic(&options, variant).unwrap();
        options.live.host_timing = Some("legacy-timing".into());
        assert!(variant.validate(&options).is_err());
    }

    #[test]
    #[cfg(all(feature = "model-timestamps", feature = "c1-ordered64"))]
    fn ordered64_packet_tick_profile_retains_composition_and_capture_bound() {
        let mut options = fixture(wave_target_v17_live_contract::Mode::Combined);
        options.live.max_batches = 135;
        let variant = Variant::PrefillKvCopyV28 {
            artifact: Path::new("copy"),
            enabled: true,
            decode: Some(DecodeComposition {
                artifact: Path::new("split"),
                split: true,
                packed: true,
                ordered64: true,
                gemv: Some((Path::new("gemv"), false)),
            }),
        };
        assert!(validate_ordered64_host_diagnostic(&options, variant).is_err());
        options.live.runtime.ordered64_packet_ticks = true;
        validate_ordered64_host_diagnostic(&options, variant).unwrap();
        let runtime = variant.worker_runtime(&options).unwrap();
        assert!(runtime.ordered64 && runtime.ordered_batches && runtime.ordered64_packet_ticks);
        assert!(!runtime.profile && !runtime.ordered64_runtime_counters);
        assert_ne!(
            ORDERED64_PACKET_TICKS_PROFILE,
            ORDERED64_HOST_DIAGNOSTIC_PROFILE
        );
        assert_ne!(ORDERED64_PACKET_TICKS_PROFILE, variant.live_profile());
        for count in [0, 257, u64::MAX] {
            options.live.max_batches = count;
            assert!(validate_ordered64_host_diagnostic(&options, variant).is_err());
        }
        options.live.max_batches = 135;
        assert!(run_variant(&options, variant).is_err());
        assert!(run_model_timestamps(&options, variant, Path::new("unused")).is_err());
        assert!(runtime.with_ordered64_runtime_counters().is_err());
    }

    #[test]
    #[cfg(feature = "model-timestamps")]
    fn packet_tick_sidecar_is_fresh_private_and_does_not_follow_symlinks() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let parent =
            std::env::temp_dir().join(format!("ferric-packet-ticks-{}", std::process::id()));
        std::fs::create_dir(&parent).unwrap();
        let path = parent.join("capture.json");
        let mut file = create_ordered64_packet_tick_file(&path).unwrap();
        file.write_all(b"retained").unwrap();
        drop(file);
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert!(create_ordered64_packet_tick_file(&path).is_err());
        let link = parent.join("link.json");
        symlink(&path, &link).unwrap();
        assert!(create_ordered64_packet_tick_file(&link).is_err());
        let linked_parent = parent.join("linked-parent");
        symlink(&parent, &linked_parent).unwrap();
        assert!(create_ordered64_packet_tick_file(&linked_parent.join("new.json")).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"retained");
        std::fs::remove_file(linked_parent).unwrap();
        std::fs::remove_file(link).unwrap();
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(parent).unwrap();
    }

    #[test]
    #[cfg(all(feature = "model-timestamps", feature = "c1-ordered64"))]
    fn ordered64_packet_tick_profile_requires_v19_without_relaxing_live_entries() {
        let mut options = fixture(wave_target_v17_live_contract::Mode::Combined);
        options.live.max_batches = 135;
        options.live.runtime.ordered64_packet_ticks = true;
        let variant = ordered64_copy_variant();
        validate_ordered64_host_diagnostic(&options, variant).unwrap();
        assert_ne!(ORDERED64_PACKET_TICKS_PROFILE, ORDERED64_KV_COPY_PROFILE);
        assert_eq!(variant.kv_append_mode(), "baseline");
        for enabled in [false, true] {
            let selection = Ordered64KvCopy {
                artifact: Path::new("copy"),
                enabled,
                prefill32_pages: None,
            };
            assert!(selection.validate(&options, variant).is_err());
            assert!(run_ordered64_kv_copy(&options, variant, selection).is_err());
            assert_eq!(
                selection.validate_packet_ticks(&options, variant).is_ok(),
                enabled
            );
            for prefill32_pages in [Some(false), Some(true)] {
                assert!(
                    Ordered64KvCopy {
                        prefill32_pages,
                        ..selection
                    }
                    .validate_packet_ticks(&options, variant)
                    .is_err()
                );
            }
        }
        let selection = Ordered64KvCopy {
            artifact: Path::new(""),
            enabled: true,
            prefill32_pages: None,
        };
        assert!(selection.validate_packet_ticks(&options, variant).is_err());
        assert_ne!(ORDERED64_PACKET_TICKS_PROFILE, PREFILL32_PAGES_PROFILE);
    }

    #[test]
    fn ordered64_host_diagnostic_rejects_other_variants_and_labels_its_scope() {
        let mut options = fixture(wave_target_v17_live_contract::Mode::Combined);
        options.live.max_batches = 135;
        for variant in [
            Variant::V17,
            Variant::RuntimeDiagnostic,
            Variant::PackedC1V22 { enabled: true },
            Variant::PrefillKvCopyV28 {
                artifact: Path::new("copy"),
                enabled: true,
                decode: None,
            },
        ] {
            assert!(validate_ordered64_host_diagnostic(&options, variant).is_err());
        }
        let mut value = json!({"runtime_profiling":false});
        annotate_ordered64_host_diagnostic(&mut value);
        assert_eq!(value["host_timing_schema"], "FerricOrdered64HostTimingV1");
        assert_eq!(value["diagnostic_max_model_batches"], 256);
        assert_eq!(value["runtime_profiling"], false);
        assert!(
            value["host_diagnostic_scope"]
                .as_str()
                .unwrap()
                .contains("not GPU time")
        );
    }

    #[test]
    fn ordered64_counter_options_keep_the_matched_selector_contract() {
        let mut options = fixture(wave_target_v17_live_contract::Mode::Combined);
        options.live.max_batches = 135;
        let variant = Variant::PrefillKvCopyV28 {
            artifact: Path::new("copy"),
            enabled: true,
            decode: Some(DecodeComposition {
                artifact: Path::new("split"),
                split: true,
                packed: true,
                ordered64: true,
                gemv: Some((Path::new("gemv"), false)),
            }),
        };
        let counters = ordered64_counter_options(&options, variant);
        if !cfg!(feature = "c1-ordered64") || cfg!(feature = "model-timestamps") {
            assert!(counters.is_err());
            return;
        }
        let counters = counters.unwrap();
        assert!(counters.ordered64 && counters.ordered_batches && counters.profile);
        assert!(counters.ordered64_runtime_counters);
        let plain = variant.worker_runtime(&options).unwrap();
        assert!(!plain.profile && !plain.ordered64_runtime_counters);
        assert!(!options.live.runtime.profile && !options.live.runtime.ordered64_runtime_counters);
        assert_eq!(
            variant.live_profile(),
            "prefill16-decode-ordered64-v29-live-v1"
        );
        assert_ne!(ORDERED64_RUNTIME_COUNTER_PROFILE, variant.live_profile());
        assert_ne!(
            ORDERED64_RUNTIME_COUNTER_PROFILE,
            ORDERED64_HOST_DIAGNOSTIC_PROFILE
        );
        options.live.max_batches = 257;
        assert!(ordered64_counter_options(&options, variant).is_err());
        options.live.max_batches = 135;
        assert!(ordered64_counter_options(&options, Variant::V17).is_err());
        options.live.host_timing = Some("legacy.json".into());
        assert!(ordered64_counter_options(&options, variant).is_err());
    }

    #[test]
    fn active_poll_counter_record_preserves_observations_and_default_record() {
        let original = json!({"observation":"unchanged", "counters":{"dispatches":652}});
        let control = ordered64_counter_record_with_wait(&original, false);
        assert_eq!(control, ordered64_counter_record(&original));
        assert!(control.get("ordered64_wait_policy").is_none());
        let active = ordered64_counter_record_with_wait(&original, true);
        assert_eq!(active["observation"], original["observation"]);
        assert_eq!(active["counters"], original["counters"]);
        assert_eq!(active["live_profile"], ORDERED64_ACTIVE_POLL_PROFILE);
        assert_ne!(active["live_profile"], control["live_profile"]);
        assert_eq!(
            active["ordered64_wait_policy"]["policy"],
            "ActivePoll10msV1"
        );
        assert_eq!(
            active["ordered64_wait_policy"]["active_window_ns"],
            10_000_000_u64
        );
        assert_eq!(
            active["ordered64_wait_policy"]["fallback_sleep_ns"],
            50_000_u64
        );
        assert_eq!(
            active["ordered64_wait_policy"]["performance_qualified"],
            false
        );
        assert_eq!(
            original,
            json!({"observation":"unchanged", "counters":{"dispatches":652}})
        );
    }

    #[test]
    fn prefill16_ordered_metadata_discloses_eligible_and_fallback_group_bounds() {
        let mut value = json!({"non_c1_group_bound":16, "dispatches":87711});
        annotate_prefill16_ordered(&mut value);
        assert_eq!(value["live_profile"], PREFILL16_ORDERED_PROFILE);
        assert_eq!(value["non_c1_group_bound"], 64);
        assert_eq!(value["dispatches"], 87711);
        assert_eq!(
            value["prefill_scheduling"]["fallback_non_c1_group_bound"],
            16
        );
        assert_eq!(value["prefill_scheduling"]["eligible_rows"], 16);
        assert_eq!(
            value["prefill_scheduling"]["ordered_groups_per_eligible_chunk"],
            10
        );
        assert_eq!(value["prefill_scheduling"]["performance_qualified"], false);
        assert!(value.get("ordered64_wait_policy").is_none());
    }

    #[test]
    fn ordered64_counter_record_is_distinct_and_preserves_checked_observations() {
        let snapshots = [snapshot(0)];
        let original = diagnostic_record("before_workload", &snapshots, &json!({}));
        let value = ordered64_counter_record(&original);
        assert_eq!(original["schema"], "FerricWaveTargetV17RuntimeDiagnosticV1");
        assert_eq!(original["live_profile"], DIAGNOSTIC_PROFILE);
        assert_eq!(value["schema"], "FerricOrdered64RuntimeCountersV1");
        assert_eq!(value["live_profile"], ORDERED64_RUNTIME_COUNTER_PROFILE);
        assert_eq!(value["host_timing_schema"], "FerricOrdered64HostTimingV1");
        assert_eq!(value["diagnostic_max_model_batches"], 256);
        assert_eq!(value["runtime_profiling"], true);
        for key in [
            "phase",
            "snapshots",
            "counter_delta",
            "authority",
            "performance_qualified",
            "benchmark_admitted",
            "serving_admitted",
            "workload_scope",
            "dispatch_prepare_scope",
        ] {
            assert_eq!(value[key], original[key]);
        }
        assert_eq!(
            value["command_accounting"],
            "commands and command_ns deltas include the earlier snapshot command and exclude the final snapshot command; currentness deltas exclude the earlier snapshot check_idle and include the final snapshot check_idle"
        );
        assert_eq!(
            original["command_accounting"],
            "delta includes the earlier snapshot command; the final snapshot excludes its own command"
        );
        assert!(
            value["runtime_counter_scope"]
                .as_str()
                .unwrap()
                .contains("scopes overlap")
        );
        assert!(
            value["runtime_counter_scope"]
                .as_str()
                .unwrap()
                .contains("not GPU timestamps")
        );
    }

    #[test]
    fn ordered64_counter_output_reuses_two_snapshot_and_single_close_lifecycle() {
        for fail_output in [None, Some(0), Some(1)] {
            let (mut runtime, events) = runtime(None, false);
            let mut records = Vec::new();
            let (result, closed) = run_diagnostic_and_close(
                &mut runtime,
                &HostTiming::ordered64_diagnostic(),
                9,
                7,
                |value| {
                    let record = ordered64_counter_record(value);
                    if fail_output == Some(records.len()) {
                        return Err("injected counter output failure".into());
                    }
                    records.push(record);
                    Ok(())
                },
                |_| Ok(()),
            );
            assert_eq!(result.is_ok(), fail_output.is_none());
            assert!(closed.is_ok());
            assert_eq!(
                events
                    .borrow()
                    .iter()
                    .filter(|event| event.as_str() == "close")
                    .count(),
                1
            );
            if fail_output.is_none() {
                assert_eq!(records.len(), 2);
                assert_eq!(records[0]["snapshots"][0]["ordinal"], 0);
                assert_eq!(records[1]["snapshots"][0]["ordinal"], 1);
                assert_eq!(records[1]["counter_delta"]["commands"], 1);
                assert_eq!(records[1]["counter_delta"]["command_ns"], 1);
            }
        }
    }
}
