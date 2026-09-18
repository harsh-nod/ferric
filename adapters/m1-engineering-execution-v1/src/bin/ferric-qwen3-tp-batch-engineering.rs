//! Bounded, explicitly non-authoritative continuous Qwen workload runner.

mod tp_benchmark_control;
mod tp_host_timing;
mod tp_peer_worker;
mod tp_rank_worker;
mod tp_worker;

use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use ferric_m1_engineering_execution_v1::tp_artifact::EngineeringTpArtifactV1;
use ferric_m1_engineering_execution_v1::tp_batch_runtime::EngineeringTpBatchRuntimeV2;
use ferric_m1_engineering_execution_v1::tp_execution::batched::EngineeringTpBatchExecutionV2;
use ferric_m1_engineering_execution_v1::tp_execution::numerical::{
    EngineeringTpNumericalCaptureV1, EngineeringTpNumericalProjectionV1 as NumericalRole,
};
use ferric_m1_engineering_execution_v1::tp_execution::{
    EngineeringTpProjectionModeV3 as ProjectionMode, EngineeringTpReductionModeV3,
};
use ferric_m1_engineering_execution_v1::tp_live_ingress;
use ferric_m1_engineering_execution_v1::tp_model::EngineeringQwenModelV1;
use ferric_m1_engineering_execution_v1::tp_paged::{
    EngineeringTpPagedLimitsV1, EngineeringTpPagedPoolV1, EngineeringTpPoolScopeV1,
};
use ferric_m1_engineering_execution_v1::tp_scheduler::{
    EngineeringTpSchedulerV1, TpRequestAdmissionV1, TpRequestIdV1, TpRequestRecordV1,
    TpRequestStateV1,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tp_benchmark_control::{BenchmarkClock, ControlConfig};
use tp_peer_worker::PeerWorker;
use tp_rank_worker::RankWorker;
use tp_worker::{RuntimeOptions, Worker};

const USAGE: &str = concat!(
    "ferric-qwen3-tp-batch-engineering --source DIR --artifact DIR --worker FILE ",
    "--devices ID[,ID...] (--requests FILE | --live-stdin --queue-rollover) --allow-unauthenticated-machine-code ",
    "[--batch-tokens 16] [--prefill-chunk 16] [--context 128] [--pages 64] ",
    "[--cache-ttl 1024] [--max-batches 240] [--disable-prefix-cache] [--prune-output-head] ",
    "[--runtime-cache-admission] [--runtime-operational] [--dispatch-sequences] [--queue-rollover] ",
    "[--runtime-profile] [--peer-shared-full-currentness] [--runtime-ordered-batches | --runtime-ordered-scalar-v3 | --runtime-full-forward | --runtime-full-forward-mfma-v7 | --runtime-full-forward-mfma-v7-wave] ",
    "[--collective host-staged-v1|host-staged-reuse-v3|device-tp1-v3|device-peer-serial-v4|device-peer-concurrent-round-v1] ",
    "[--peer-artifact DIR] [--kernel-profile v2|v3-wave|v3-mfma|v5-wave32|v5-mfma32] ",
    "[--projection baseline|wave|mfma|auto] [--attention baseline|wave] ",
    "[--head-precision bf16-v7-control|fp32-v7|bf16-v8-control|fp32-v8 --fp32-head-artifact DIR] ",
    "[--fp32-argmax serial-v7|wave-v11 --fp32-argmax-artifact DIR] ",
    "[--kv-pool-profile large-kv-v9 --large-kv-artifact DIR] ",
    "[--benchmark-control FILE] [--host-timing FILE] ",
    "[--numerical-capture DIR --numerical-batch N --numerical-layer N --numerical-projection q|k|v|o|gate|up|down]"
);

// Admission, arithmetic and submission are independently selected policies.
#[allow(clippy::struct_excessive_bools)]
struct Options {
    source: PathBuf,
    artifact: PathBuf,
    peer_artifact: Option<PathBuf>,
    fp32_head_artifact: Option<PathBuf>,
    fp32_argmax_artifact: Option<PathBuf>,
    fp32_argmax: Option<Fp32Argmax>,
    head_precision: Option<HeadPrecision>,
    large_kv_artifact: Option<PathBuf>,
    worker: PathBuf,
    requests: Option<PathBuf>,
    benchmark_control: Option<PathBuf>,
    host_timing: Option<PathBuf>,
    numerical: Option<NumericalOptions>,
    devices: Vec<u64>,
    rows: usize,
    chunk: usize,
    context: u32,
    pages: u32,
    ttl: u64,
    max_batches: u64,
    cache: bool,
    prune_output_head: bool,
    runtime: RuntimeOptions,
    ordered_scalar_v3: bool,
    full_forward_mfma_v7: bool,
    full_forward_mfma_v7_wave: bool,
    collective: EngineeringTpReductionModeV3,
    kernel_profile: KernelProfile,
    projection: ProjectionMode,
    wave_attention: bool,
}

struct NumericalOptions {
    directory: PathBuf,
    batch: u64,
    layer: u32,
    role: NumericalRole,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Fp32Argmax {
    SerialV7,
    WaveV11,
}

impl Fp32Argmax {
    const fn label(self) -> &'static str {
        match self {
            Self::SerialV7 => "serial-v7",
            Self::WaveV11 => "wave-v11",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HeadPrecision {
    Bf16Control,
    Fp32,
    Bf16ControlV8,
    Fp32V8,
}

impl HeadPrecision {
    const fn label(self) -> &'static str {
        match self {
            Self::Bf16Control => "bf16-v7-control",
            Self::Fp32 => "fp32-v7",
            Self::Bf16ControlV8 => "bf16-v8-control",
            Self::Fp32V8 => "fp32-v8",
        }
    }

    const fn wide32(self) -> bool {
        matches!(self, Self::Bf16ControlV8 | Self::Fp32V8)
    }

    const fn fp32(self) -> bool {
        matches!(self, Self::Fp32 | Self::Fp32V8)
    }
}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
enum KernelProfile {
    #[default]
    V2,
    Wave,
    Mfma,
    WideWave,
    WideMfma,
}

impl KernelProfile {
    fn wide32(self) -> bool {
        matches!(self, Self::WideWave | Self::WideMfma)
    }
    fn mfma(self) -> bool {
        matches!(self, Self::Mfma | Self::WideMfma)
    }
}

impl Options {
    fn paged_limits(&self) -> Result<EngineeringTpPagedLimitsV1, String> {
        let constructor = if self.large_kv_artifact.is_some() {
            EngineeringTpPagedLimitsV1::new_large_kv32
        } else {
            EngineeringTpPagedLimitsV1::new
        };
        constructor(self.context, 32, self.pages, self.ttl)
            .map_err(|e| format!("page limits: {e:?}"))
    }
    fn live_stdin(&self) -> bool {
        self.requests.is_none()
    }

    fn parse(args: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut args = args.peekable();
        let mut seen = BTreeSet::new();
        let (mut source, mut artifact, mut worker, mut requests, mut devices) =
            (None, None, None, None, None);
        let (mut rows, mut chunk, mut context, mut pages, mut ttl, mut max_batches) =
            (16, 16, 128, 64, 1024, 240);
        let (mut consent, mut cache) = (false, true);
        let mut prune_output_head = false;
        let mut live_stdin = false;
        let mut runtime = RuntimeOptions::default();
        let mut ordered_scalar_v3 = false;
        let mut full_forward_mfma_v7 = false;
        let mut full_forward_mfma_v7_wave = false;
        let mut collective = EngineeringTpReductionModeV3::HostStagedV1;
        let mut kernel_profile = KernelProfile::V2;
        let mut projection = ProjectionMode::Baseline;
        let mut wave_attention = false;
        let mut peer_artifact = None;
        let mut fp32_head_artifact = None;
        let mut fp32_argmax_artifact = None;
        let mut fp32_argmax = None;
        let mut head_precision = None;
        let mut large_kv = false;
        let mut large_kv_artifact = None;
        let mut benchmark_control = None;
        let mut host_timing = None;
        let (mut numerical_directory, mut numerical_batch, mut numerical_layer, mut numerical_role) =
            (None, None, None, None);
        while let Some(flag) = args.next() {
            if !seen.insert(flag.clone()) {
                return Err(format!("duplicate option {flag}"));
            }
            match flag.as_str() {
                "--allow-unauthenticated-machine-code" => {
                    consent = true;
                    continue;
                }
                "--disable-prefix-cache" => {
                    cache = false;
                    continue;
                }
                "--prune-output-head" => {
                    prune_output_head = true;
                    continue;
                }
                "--live-stdin" => {
                    live_stdin = true;
                    continue;
                }
                "--runtime-cache-admission" => {
                    runtime.cache_admission = true;
                    continue;
                }
                "--runtime-operational" => {
                    runtime.operational = true;
                    continue;
                }
                "--dispatch-sequences" => {
                    runtime.sequences = true;
                    continue;
                }
                "--runtime-ordered-batches" => {
                    runtime.ordered_batches = true;
                    continue;
                }
                "--runtime-ordered-scalar-v3" => {
                    ordered_scalar_v3 = true;
                    continue;
                }
                "--runtime-full-forward" => {
                    runtime.full_forward = true;
                    continue;
                }
                "--runtime-full-forward-mfma-v7" => {
                    runtime.full_forward = true;
                    full_forward_mfma_v7 = true;
                    continue;
                }
                "--runtime-full-forward-mfma-v7-wave" => {
                    runtime.full_forward = true;
                    full_forward_mfma_v7_wave = true;
                    continue;
                }
                "--queue-rollover" => {
                    runtime.rollover = true;
                    continue;
                }
                "--runtime-profile" => {
                    runtime.profile = true;
                    continue;
                }
                "--peer-shared-full-currentness" => {
                    runtime.shared_full_currentness = true;
                    continue;
                }
                "--help" => return Err(USAGE.into()),
                _ => {}
            }
            let value = args
                .next()
                .ok_or_else(|| format!("missing value for {flag}"))?;
            match flag.as_str() {
                "--source" => source = Some(PathBuf::from(value)),
                "--artifact" => artifact = Some(PathBuf::from(value)),
                "--peer-artifact" => peer_artifact = Some(PathBuf::from(value)),
                "--fp32-head-artifact" => fp32_head_artifact = Some(PathBuf::from(value)),
                "--fp32-argmax-artifact" => fp32_argmax_artifact = Some(PathBuf::from(value)),
                "--fp32-argmax" => {
                    fp32_argmax = Some(match value.as_str() {
                        "serial-v7" => Fp32Argmax::SerialV7,
                        "wave-v11" => Fp32Argmax::WaveV11,
                        _ => return Err("unknown explicit FP32 argmax profile".into()),
                    });
                }
                "--head-precision" => {
                    head_precision = Some(match value.as_str() {
                        "bf16-v7-control" => HeadPrecision::Bf16Control,
                        "fp32-v7" => HeadPrecision::Fp32,
                        "bf16-v8-control" => HeadPrecision::Bf16ControlV8,
                        "fp32-v8" => HeadPrecision::Fp32V8,
                        _ => return Err("unknown explicit head precision profile".into()),
                    });
                }
                "--worker" => worker = Some(PathBuf::from(value)),
                "--requests" => requests = Some(PathBuf::from(value)),
                "--benchmark-control" => benchmark_control = Some(PathBuf::from(value)),
                "--host-timing" => host_timing = Some(PathBuf::from(value)),
                "--numerical-capture" => numerical_directory = Some(PathBuf::from(value)),
                "--numerical-batch" => {
                    numerical_batch = Some(value.parse::<u64>().map_err(|e| e.to_string())?);
                }
                "--numerical-layer" => {
                    numerical_layer = Some(value.parse::<u32>().map_err(|e| e.to_string())?);
                }
                "--numerical-projection" => numerical_role = Some(NumericalRole::parse(&value)?),
                "--devices" => {
                    devices = Some(
                        value
                            .split(',')
                            .map(str::parse::<u64>)
                            .collect::<Result<Vec<_>, _>>()
                            .map_err(|e| e.to_string())?,
                    );
                }
                "--batch-tokens" => rows = value.parse::<usize>().map_err(|e| e.to_string())?,
                "--prefill-chunk" => chunk = value.parse::<usize>().map_err(|e| e.to_string())?,
                "--context" => context = value.parse::<u32>().map_err(|e| e.to_string())?,
                "--pages" => pages = value.parse::<u32>().map_err(|e| e.to_string())?,
                "--cache-ttl" => ttl = value.parse::<u64>().map_err(|e| e.to_string())?,
                "--max-batches" => max_batches = value.parse::<u64>().map_err(|e| e.to_string())?,
                "--collective" => {
                    collective = match value.as_str() {
                        "host-staged-v1" => EngineeringTpReductionModeV3::HostStagedV1,
                        "host-staged-reuse-v3" => EngineeringTpReductionModeV3::HostStagedReuseV3,
                        "device-tp1-v3" => EngineeringTpReductionModeV3::DeviceTp1V3,
                        "device-peer-serial-v4" => EngineeringTpReductionModeV3::DevicePeerV4,
                        "device-peer-concurrent-round-v1" => {
                            EngineeringTpReductionModeV3::DevicePeerConcurrentV1
                        }
                        _ => return Err("unsupported collective".into()),
                    }
                }
                "--kernel-profile" => {
                    kernel_profile = match value.as_str() {
                        "v2" => KernelProfile::V2,
                        "v3-wave" => KernelProfile::Wave,
                        "v3-mfma" => KernelProfile::Mfma,
                        "v5-wave32" => KernelProfile::WideWave,
                        "v5-mfma32" => KernelProfile::WideMfma,
                        _ => return Err("unknown closed kernel profile".into()),
                    }
                }
                "--projection" => {
                    projection = match value.as_str() {
                        "baseline" => ProjectionMode::Baseline,
                        "wave" => ProjectionMode::Wave,
                        "mfma" => ProjectionMode::Mfma,
                        "auto" => ProjectionMode::Auto,
                        _ => return Err("unknown projection mode".into()),
                    }
                }
                "--attention" => {
                    wave_attention = match value.as_str() {
                        "baseline" => false,
                        "wave" => true,
                        _ => return Err("unknown attention mode".into()),
                    }
                }
                "--kv-pool-profile" => {
                    if value != "large-kv-v9" {
                        return Err("unknown physical KV pool profile".into());
                    }
                    large_kv = true;
                }
                "--large-kv-artifact" => large_kv_artifact = Some(PathBuf::from(value)),
                _ => return Err(format!("unknown option {flag}")),
            }
        }
        if !consent {
            return Err("explicit --allow-unauthenticated-machine-code is required; this is not protected M1 execution".into());
        }
        let devices = devices.ok_or("--devices is required")?;
        if !matches!(devices.len(), 1 | 2 | 8)
            || devices.contains(&0)
            || devices.iter().copied().collect::<BTreeSet<_>>().len() != devices.len()
        {
            return Err("devices require 1, 2 or 8 distinct nonzero physical IDs".into());
        }
        if (kernel_profile == KernelProfile::V2
            && (projection != ProjectionMode::Baseline
                || wave_attention
                || collective == EngineeringTpReductionModeV3::DeviceTp1V3))
            || (projection.requires_mfma() && !kernel_profile.mfma())
            || (collective == EngineeringTpReductionModeV3::DeviceTp1V3 && devices.len() != 1)
        {
            return Err(
                "selected optimization is unavailable in the exact kernel profile or world size"
                    .into(),
            );
        }
        let peer = collective.is_peer();
        if runtime.shared_full_currentness && !peer {
            return Err(
                "--peer-shared-full-currentness requires an explicit peer transport".into(),
            );
        }
        if peer != peer_artifact.is_some()
            || (peer && !matches!(devices.len(), 2 | 8))
            || (peer && runtime.rollover)
            || (collective == EngineeringTpReductionModeV3::DevicePeerConcurrentV1
                && runtime.sequences)
        {
            return Err("peer transport requires TP2/8 and --peer-artifact; rollover and concurrent same-rank sequences are unsupported".into());
        }
        let packets_per_batch = 36 * (15 + collective.extra_dispatches_per_layer()) + 4;
        if !seen.contains("--max-batches") && !runtime.rollover {
            max_batches = max_batches.min(
                fe2o3_kfd::engineering_wire::MAX_UNRETIRED_RING_PACKETS_V1 / packets_per_batch,
            );
        }
        if !(1..=if kernel_profile.wide32() { 32 } else { 16 }).contains(&rows)
            || !(1..=rows).contains(&chunk)
            || max_batches == 0
            || max_batches > 1_000_000
            || (!runtime.rollover
                && max_batches
                    .checked_mul(packets_per_batch)
                    .is_none_or(|n| n > fe2o3_kfd::engineering_wire::MAX_UNRETIRED_RING_PACKETS_V1))
        {
            return Err("batch, chunk, or conservative packet bound exceeded".into());
        }
        let limits_constructor = if large_kv {
            EngineeringTpPagedLimitsV1::new_large_kv32
        } else {
            EngineeringTpPagedLimitsV1::new
        };
        limits_constructor(context, 32, pages, ttl).map_err(|e| format!("page limits: {e:?}"))?;
        if runtime.profile && (benchmark_control.is_some() || numerical_directory.is_some()) {
            return Err("--runtime-profile is diagnostic only and excludes replica benchmarks and numerical capture".into());
        }
        if live_stdin == requests.is_some()
            || (live_stdin
                && (!runtime.rollover
                    || benchmark_control.is_some()
                    || numerical_directory.is_some()))
        {
            return Err("exactly one of --requests or --live-stdin is required; live stdin requires --queue-rollover and excludes replica/numerical capture".into());
        }
        let numerical = match (numerical_directory, numerical_batch, numerical_layer, numerical_role) {
            (None, None, None, None) => None,
            (Some(directory), Some(batch), Some(layer), Some(role))
                if devices.len() == 1 && !kernel_profile.wide32() && !runtime.sequences
                    && !wave_attention && matches!(projection, ProjectionMode::Baseline | ProjectionMode::Mfma)
                    && benchmark_control.is_none() && host_timing.is_none()
                    && (1..=64).contains(&batch) && batch <= max_batches && layer < 36 =>
                Some(NumericalOptions { directory, batch, layer, role }),
            _ => return Err("numerical capture requires all four selection options, TP1/16 rows, baseline or MFMA, no wave attention, sequencing, timing or replica benchmark".into()),
        };
        if head_precision.is_some() != fp32_head_artifact.is_some()
            || head_precision.is_some_and(|precision| {
                devices.len() != 1
                    || kernel_profile.wide32() != precision.wide32()
                    || runtime.sequences
                    || (wave_attention
                        && !precision.wide32()
                        && !(precision == HeadPrecision::Fp32
                            && kernel_profile == KernelProfile::Mfma
                            && projection == ProjectionMode::Mfma
                            && collective == EngineeringTpReductionModeV3::DeviceTp1V3))
                    || numerical.is_some()
                    || benchmark_control.is_some()
                    || !matches!(projection, ProjectionMode::Baseline | ProjectionMode::Mfma)
            })
        {
            return Err("head requires both explicit options, TP1 with exact v7/16-row or v8/32-row profile, baseline or MFMA projection; wave attention needs v8 or v3-mfma/FP32-v7/device-tp1-v3, no sequences, numerical capture or replicas".into());
        }
        if large_kv != large_kv_artifact.is_some()
            || (large_kv
                && (devices.len() != 1
                    || !kernel_profile.wide32()
                    || !head_precision.is_some_and(HeadPrecision::wide32)
                    || wave_attention
                    || !matches!(projection, ProjectionMode::Baseline | ProjectionMode::Mfma)
                    || runtime.sequences
                    || peer
                    || benchmark_control.is_some()
                    || numerical.is_some()))
        {
            return Err("large-kv-v9 requires both explicit pool/image options, TP1/capacity32 with v8 head, baseline attention and baseline/MFMA projection; peers, sequences, capture and replicas are unsupported".into());
        }
        if ordered_scalar_v3 && runtime.ordered_batches {
            return Err(
                "ordered scalar-v3 and the wide ordered profile are mutually exclusive".into(),
            );
        }
        if runtime.ordered_batches
            && (devices.len() != 1
                || kernel_profile != KernelProfile::WideMfma
                || !head_precision.is_some_and(HeadPrecision::wide32)
                || projection != ProjectionMode::Mfma
                || collective != EngineeringTpReductionModeV3::DeviceTp1V3
                || !prune_output_head
                || runtime.sequences
                || runtime.shared_full_currentness
                || large_kv
                || numerical.is_some()
                || benchmark_control.is_some())
        {
            return Err("--runtime-ordered-batches requires TP1/v5-mfma32 with v8 head, MFMA projection, baseline or wave attention, device-tp1-v3 and pruning; legacy sequences, large KV, peers, numerical capture and replicas are unsupported".into());
        }
        if ordered_scalar_v3 {
            if devices.len() != 1
                || kernel_profile != KernelProfile::Wave
                || projection != ProjectionMode::Baseline
                || wave_attention
                || collective != EngineeringTpReductionModeV3::DeviceTp1V3
                || prune_output_head
                || head_precision.is_some()
                || fp32_head_artifact.is_some()
                || peer_artifact.is_some()
                || runtime.sequences
                || runtime.shared_full_currentness
                || runtime.rollover
                || large_kv
                || numerical.is_some()
                || benchmark_control.is_some()
                || live_stdin
            {
                return Err("--runtime-ordered-scalar-v3 requires TP1/v3-wave capacity16 with baseline projection/attention, device-tp1-v3 and unpruned BF16 head; secondary images, peers, sequences, rollover, live input, numerical capture and replicas are unsupported".into());
            }
            runtime.ordered_batches = true;
        }
        if [
            "--runtime-full-forward",
            "--runtime-full-forward-mfma-v7",
            "--runtime-full-forward-mfma-v7-wave",
        ]
        .iter()
        .filter(|flag| seen.contains(**flag))
        .count()
            > 1
        {
            return Err("full-forward profile selectors are mutually exclusive".into());
        }
        let full_forward_numerics = if full_forward_mfma_v7 || full_forward_mfma_v7_wave {
            kernel_profile == KernelProfile::Mfma
                && projection == ProjectionMode::Mfma
                && head_precision == Some(HeadPrecision::Fp32)
                && fp32_head_artifact.is_some()
        } else {
            kernel_profile == KernelProfile::Wave
                && projection == ProjectionMode::Baseline
                && head_precision.is_none()
                && fp32_head_artifact.is_none()
        };
        if runtime.full_forward
            && (devices.len() != 1
                || !full_forward_numerics
                || wave_attention != full_forward_mfma_v7_wave
                || collective != EngineeringTpReductionModeV3::DeviceTp1V3
                || rows != 1
                || chunk != 1
                || context != 64
                || pages != 4
                || cache
                || prune_output_head
                || peer_artifact.is_some()
                || runtime.sequences
                || runtime.ordered_batches
                || runtime.shared_full_currentness
                || runtime.rollover
                || runtime.profile
                || large_kv
                || numerical.is_some()
                || benchmark_control.is_some()
                || host_timing.is_some()
                || live_stdin)
        {
            return Err("full-forward requires its exact scalar-v3/BF16 or MFMA-v3/FP32-v7 selector and matching baseline/wave attention, TP1, device-tp1-v3, one row/chunk, context64/pages4, unpruned head and disabled prefix cache; other submission modes, extra sidecars, live input and instrumentation are unsupported".into());
        }
        if fp32_argmax.is_some() != fp32_argmax_artifact.is_some()
            || (fp32_argmax.is_some() && !full_forward_mfma_v7_wave)
        {
            return Err("FP32 argmax requires both explicit mode and v11 artifact with the exact MFMA-v7-wave full-forward profile".into());
        }
        Ok(Self {
            source: source.ok_or("--source is required")?,
            artifact: artifact.ok_or("--artifact is required")?,
            peer_artifact,
            fp32_head_artifact,
            fp32_argmax_artifact,
            fp32_argmax,
            head_precision,
            large_kv_artifact,
            worker: worker.ok_or("--worker is required")?,
            requests,
            benchmark_control,
            host_timing,
            numerical,
            devices,
            rows,
            chunk,
            context,
            pages,
            ttl,
            max_batches,
            cache,
            prune_output_head,
            runtime,
            ordered_scalar_v3,
            full_forward_mfma_v7,
            full_forward_mfma_v7_wave,
            collective,
            kernel_profile,
            projection,
            wave_attention,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Workload {
    schema: String,
    requests: Vec<Request>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    name: String,
    prompt: String,
    new_tokens: u32,
    arrival_tick: u64,
    #[serde(default)]
    cancel_tick: Option<u64>,
}

impl Workload {
    fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > 1_048_576 {
            return Err("workload exceeds 1 MiB".into());
        }
        let workload: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        if workload.schema != "FerricQwen3TpWorkloadV2"
            || !(1..=32).contains(&workload.requests.len())
        {
            return Err("wrong workload schema or request count outside 1..=32".into());
        }
        let mut names = BTreeSet::new();
        for request in &workload.requests {
            if request.name.is_empty()
                || request.name.len() > 64
                || !names.insert(&request.name)
                || request.prompt.is_empty()
                || request.prompt.len() > 16_384
                || !(1..=256).contains(&request.new_tokens)
                || request.arrival_tick > 10_000
                || request
                    .cancel_tick
                    .is_some_and(|tick| tick < request.arrival_tick || tick > 10_000)
            {
                return Err("invalid request identity, prompt, output, or tick bounds".into());
            }
        }
        Ok(workload)
    }
    fn open(path: &Path) -> Result<(Self, String), String> {
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let metadata = file.metadata().map_err(|e| e.to_string())?;
        if !metadata.is_file() || metadata.len() > 1_048_576 {
            return Err("workload must be a regular file <=1 MiB".into());
        }
        let mut bytes = Vec::new();
        file.take(1_048_577)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        let workload = Self::parse(&bytes)?;
        Ok((workload, hex(&Sha256::digest(&bytes))))
    }
}

fn emit(value: &impl Serialize) -> Result<(), String> {
    let mut out = std::io::stdout().lock();
    serde_json::to_writer(&mut out, value).map_err(|e| e.to_string())?;
    out.write_all(b"\n")
        .and_then(|()| out.flush())
        .map_err(|e| e.to_string())
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    bytes
        .iter()
        .flat_map(|b| {
            [
                char::from(DIGITS[usize::from(b >> 4)]),
                char::from(DIGITS[usize::from(b & 15)]),
            ]
        })
        .collect()
}

fn hash_file(path: &Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() > 512 * 1024 * 1024 {
        return Err("executable file bound".into());
    }
    let mut hash = Sha256::new();
    let mut bytes = vec![0; 65_536];
    loop {
        let n = file.read(&mut bytes).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&bytes[..n]);
    }
    Ok(hex(&hash.finalize()))
}

fn elapsed_ns(start: Instant) -> Result<u64, String> {
    u64::try_from(start.elapsed().as_nanos()).map_err(|_| "elapsed clock overflow".into())
}

fn timed_step<T>(
    mut clock: impl FnMut() -> Result<u64, String>,
    step: impl FnOnce(u64, &mut dyn FnMut() -> u64) -> Result<T, String>,
) -> Result<T, String> {
    let started = clock()?;
    let mut completion_error = None;
    let result = step(started, &mut || match clock() {
        Ok(now) => now,
        Err(error) => {
            // The coordinator completion callback cannot return a Result.
            // Discard its report before publication if the clock failed.
            completion_error = Some(error);
            started
        }
    });
    if let Some(error) = completion_error {
        return Err(error);
    }
    result
}

fn emit_request(
    name: &str,
    record: &TpRequestRecordV1,
    model: &EngineeringQwenModelV1,
) -> Result<(), String> {
    let decoded = model.decode(&record.generated_tokens)?;
    let intervals = record
        .output_timestamps_ns
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .collect::<Vec<_>>();
    let ttft = record
        .output_timestamps_ns
        .first()
        .map(|time| time - record.arrival_ns);
    emit(&serde_json::json!({
        "schema":"FerricQwen3TpBatchRequestV2", "authority":"none", "name":name,
        "slot":record.id.slot, "generation":record.id.generation,
        "state":format!("{:?}", record.state()), "prompt_tokens":record.prompt_tokens,
        "generated_tokens":record.generated_tokens, "generated_utf8_bytes":decoded,
        "generated_text":String::from_utf8(decoded.clone()).ok(),
        "cached_prefix_tokens":record.cached_prefix_tokens,
        "arrival_tick":record.arrival_tick, "arrival_ns":record.arrival_ns,
        "output_timestamps_ns":record.output_timestamps_ns, "cancelled_ns":record.cancelled_ns,
        "ttft_ns":ttft, "decode_intervals_ns":intervals,
        "tpot_ns":if intervals.is_empty() { None } else { Some(intervals.iter().sum::<u64>() / u64::try_from(intervals.len()).map_err(|_| "interval count")?) }
    }))
}

fn run_workload(
    runtime: &mut EngineeringTpBatchRuntimeV2<EngineeringTpBatchExecutionV2<RankWorker>>,
    model: &EngineeringQwenModelV1,
    workload: &Workload,
    prompts: &[Vec<u32>],
    options: &Options,
    benchmark_clock: Option<&BenchmarkClock>,
    timing: &ferric_m1_engineering_execution_v1::host_timing::HostTiming,
) -> Result<(), String> {
    let clock = Instant::now();
    let now = || benchmark_clock.map_or_else(|| elapsed_ns(clock), BenchmarkClock::elapsed_ns);
    let mut active: Vec<(usize, TpRequestIdV1)> = Vec::new();
    let mut admitted = vec![false; workload.requests.len()];
    let (mut tick, mut batches) = (0, 0);
    loop {
        for (index, request) in workload.requests.iter().enumerate() {
            if admitted[index] || request.arrival_tick > tick {
                continue;
            }
            let now = now()?;
            let hit = runtime.admit(
                TpRequestAdmissionV1 {
                    prompt_tokens: prompts[index].clone(),
                    max_new_tokens: request.new_tokens,
                    cached_prefix_tokens: 0,
                    arrival_tick: request.arrival_tick,
                    arrival_ns: now,
                },
                tick,
                now,
            )?;
            admitted[index] = true;
            active.push((index, hit.request));
            emit(
                &serde_json::json!({"schema":"FerricQwen3TpBatchAdmissionV2", "authority":"none",
                "name":request.name, "slot":hit.request.slot, "generation":hit.request.generation,
                "tick":tick, "arrival_ns":now, "prompt_tokens":prompts[index],
                "cached_tokens":hit.cached_tokens, "cached_pages":hit.cached_pages}),
            )?;
        }
        for &(index, id) in &active {
            if workload.requests[index]
                .cancel_tick
                .is_some_and(|cancel| cancel <= tick)
            {
                runtime.cancel(id, now()?)?;
            }
        }
        retire_finished(runtime, &mut active, workload, model, tick)?;
        if active.is_empty() {
            let next = workload
                .requests
                .iter()
                .enumerate()
                .filter(|(i, _)| !admitted[*i])
                .map(|(_, r)| r.arrival_tick)
                .min();
            if let Some(next) = next {
                tick = next;
                continue;
            }
            return Ok(());
        }
        if batches >= options.max_batches {
            return Err("workload exhausted its conservative batch/packet budget".into());
        }
        let batch_timing = timing.scope("controller_batch");
        let report = timed_step(now, |started, completed| {
            runtime.step(tick, started, completed)
        })?
        .ok_or("active requests produced no batch")?;
        drop(batch_timing);
        let rows = report.rows.iter().map(|row| serde_json::json!({
            "slot":row.request.slot, "generation":row.request.generation, "token":row.token_id,
            "position":row.absolute_position, "kind":format!("{:?}", row.kind)
        })).collect::<Vec<_>>();
        let outputs = report.outputs.iter().map(|out| serde_json::json!({
            "slot":out.request.slot, "generation":out.request.generation, "token":out.token_id,
            "index":out.output_index, "completed_ns":out.completed_ns, "finished":out.finished
        })).collect::<Vec<_>>();
        let stats = runtime.page_stats();
        emit(
            &serde_json::json!({"schema":"FerricQwen3TpBatchCompletedV2", "authority":"none",
            "tick":tick, "batch_id":report.batch_id, "pool_batch_id":report.pool_batch_id,
            "rows":rows, "outputs":outputs, "started_ns":report.started_ns, "completed_ns":report.completed_ns,
            "rank_dispatch_counts":report.rank_dispatch_counts,
            "output_head_rows":report.output_head_rows,
            "free_pages":stats.free_pages, "retained_pages":stats.retained_pages,
            "cached_pages":stats.cached_pages, "prefix_hits":stats.prefix_hits,
            "hit_tokens":stats.hit_tokens, "evicted_pages":stats.evicted_pages}),
        )?;
        retire_finished(runtime, &mut active, workload, model, tick)?;
        batches += 1;
        tick += 1;
    }
}

fn retire_finished(
    runtime: &mut EngineeringTpBatchRuntimeV2<EngineeringTpBatchExecutionV2<RankWorker>>,
    active: &mut Vec<(usize, TpRequestIdV1)>,
    workload: &Workload,
    model: &EngineeringQwenModelV1,
    tick: u64,
) -> Result<(), String> {
    let mut done = Vec::new();
    for &(index, id) in active.iter() {
        if matches!(
            runtime.request(id)?.state(),
            TpRequestStateV1::Completed | TpRequestStateV1::Cancelled
        ) {
            let record = runtime.retire(id, tick)?;
            emit_request(&workload.requests[index].name, &record, model)?;
            done.push(id);
        }
    }
    active.retain(|(_, id)| !done.contains(id));
    Ok(())
}

fn run(options: &Options) -> Result<(), String> {
    let mut diagnostics = tp_host_timing::TimingFile::create(options.host_timing.as_deref())?;
    let result = run_with_timing(options, &mut diagnostics);
    let output = diagnostics.finish(&result);
    match (result, output) {
        (Ok(()), output) => output,
        (Err(error), Ok(())) => Err(error),
        (Err(error), Err(output)) => Err(format!("{error}; {output}")),
    }
}

fn emit_runtime_diagnostic(
    runtime: &mut EngineeringTpBatchRuntimeV2<EngineeringTpBatchExecutionV2<RankWorker>>,
    phase: &str,
) -> Result<(), String> {
    let snapshots = runtime.runtime_diagnostic_snapshot()?;
    eprintln!("{}", serde_json::to_string(&serde_json::json!({
        "schema":"FerricQwen3TpRuntimeDiagnosticV1","authority":"none",
        "phase":phase,"measurement":"cumulative overlapping host-wall counters; not GPU timestamps",
        "performance_qualified":false,"ranks":snapshots,
    })).map_err(|e| e.to_string())?);
    Ok(())
}

fn run_with_timing(
    options: &Options,
    diagnostics: &mut tp_host_timing::TimingFile,
) -> Result<(), String> {
    let timing = diagnostics.timing.clone();
    let _controller_timing = timing.scope("controller");
    let setup_timing = timing.scope("setup");
    let whole = Instant::now();
    let admission_timing = timing.scope("setup_admission");
    let loaded = options
        .requests
        .as_deref()
        .map(Workload::open)
        .transpose()?;
    let (workload, loaded_requests_sha256) = loaded.map_or((None, None), |(workload, hash)| {
        (Some(workload), Some(hash))
    });
    if timing.is_enabled() {
        diagnostics
            .workload_sha256
            .clone_from(&loaded_requests_sha256);
    }
    let control = options
        .benchmark_control
        .as_ref()
        .map(|path| {
            let config = ControlConfig::open(
                path,
                options
                    .requests
                    .as_deref()
                    .ok_or("replica workload missing")?,
                &options.devices,
            )?;
            config.verify_loaded_requests_sha256(
                loaded_requests_sha256
                    .as_deref()
                    .ok_or("replica workload hash missing")?,
            )?;
            Ok::<_, String>(config)
        })
        .transpose()?;
    let worker_hash = hash_file(&options.worker)?;
    let controller_hash = hash_file(Path::new("/proc/self/exe"))?;
    let artifact = if options.kernel_profile == KernelProfile::V2 {
        EngineeringTpArtifactV1::open_batch(
            &options.artifact,
            &ferric_qwen3_tp_batch_kernels_device_v2::compiler_expectation_roster_v2(),
        )
    } else if options.kernel_profile.wide32() {
        let mfma = options.kernel_profile.mfma();
        let mut expected =
            ferric_qwen3_tp_batch32_kernels_device_v5::compiler_expectation_roster_v5();
        if !mfma {
            expected.retain(|entry| !entry.export_name().contains("_mfma_"));
        }
        EngineeringTpArtifactV1::open_batch32(&options.artifact, &expected, mfma)
    } else {
        let mfma = options.kernel_profile.mfma();
        let mut expected = ferric_qwen3_tp_perf_kernels_device_v3::compiler_expectation_roster_v3();
        if !mfma {
            expected.retain(|entry| !entry.export_name().contains("_mfma_"));
        }
        EngineeringTpArtifactV1::open_performance(&options.artifact, &expected, mfma)
    }
    .map_err(|e| e.to_string())?;
    let peer_artifact = options
        .peer_artifact
        .as_ref()
        .map(|path| {
            if options.kernel_profile.wide32() {
                EngineeringTpArtifactV1::open_peer32(
                    path,
                    &ferric_qwen3_tp_peer32_kernels_device_v6::compiler_expectation_roster_v6(),
                )
            } else {
                EngineeringTpArtifactV1::open_peer(
                    path,
                    &ferric_qwen3_tp_peer_kernels_device_v4::compiler_expectation_roster_v4(),
                )
            }
            .map_err(|error| error.to_string())
        })
        .transpose()?;
    let fp32_head_artifact = options
        .fp32_head_artifact
        .as_ref()
        .map(|path| {
            if options.head_precision.is_some_and(HeadPrecision::wide32) {
                EngineeringTpArtifactV1::open_fp32_head32(
                    path,
                    &ferric_qwen3_tp_fp32_head32_kernels_device_v8::compiler_expectation_roster_v8(
                    ),
                )
            } else {
                EngineeringTpArtifactV1::open_fp32_head(
                    path,
                    &ferric_qwen3_tp_fp32_head_kernels_device_v7::compiler_expectation_roster_v7(),
                )
            }
            .map_err(|error| error.to_string())
        })
        .transpose()?;
    let large_kv_artifact = options
        .large_kv_artifact
        .as_ref()
        .map(|path| {
            EngineeringTpArtifactV1::open_large_kv32(
                path,
                &ferric_qwen3_tp_large_kv_kernels_device_v9::compiler_expectation_roster_v9(),
            )
            .map_err(|error| error.to_string())
        })
        .transpose()?;
    let fp32_argmax_artifact = options
        .fp32_argmax_artifact
        .as_ref()
        .map(|path| {
            EngineeringTpArtifactV1::open_fp32_argmax32_v11(path).map_err(|error| error.to_string())
        })
        .transpose()?;
    drop(admission_timing);
    let model_timing = timing.scope("setup_model");
    let model = EngineeringQwenModelV1::open(&options.source)?;
    let prompts = workload
        .as_ref()
        .into_iter()
        .flat_map(|workload| &workload.requests)
        .map(|r| {
            let tokens = model.encode(&r.prompt)?;
            if tokens.is_empty()
                || tokens.len() + r.new_tokens as usize - 1 > options.context as usize
            {
                return Err("tokenized prompt/output exceeds context".into());
            }
            Ok(tokens)
        })
        .collect::<Result<Vec<_>, String>>()?;
    drop(model_timing);
    let scheduler_timing = timing.scope("setup_scheduler");
    let mut session = [0; 32];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut session))
        .map_err(|e| e.to_string())?;
    let limits = options.paged_limits()?;
    let kv_pool_payload_bytes = limits
        .target_kv_payload_bytes()
        .map_err(|e| format!("KV payload: {e:?}"))?;
    let scope = EngineeringTpPoolScopeV1 {
        model: *model.bundle_id().as_bytes(),
        session,
    };
    let pool = if let Some(large) = &large_kv_artifact {
        EngineeringTpPagedPoolV1::new_large_kv32(scope, limits, large)
    } else if options.kernel_profile.wide32() {
        EngineeringTpPagedPoolV1::new_wide32(scope, limits)
    } else {
        EngineeringTpPagedPoolV1::new(scope, limits)
    }
    .map_err(|e| format!("page pool: {e:?}"))?;
    let scheduler_constructor = if options.kernel_profile.wide32() {
        EngineeringTpSchedulerV1::new_wide32
    } else {
        EngineeringTpSchedulerV1::new
    };
    let scheduler = scheduler_constructor(
        model.config().vocabulary_size,
        options.context,
        options.rows,
        options.chunk,
    )
    .map_err(|e| format!("scheduler: {e:?}"))?;
    drop(scheduler_timing);
    let workers_timing = timing.scope("setup_workers");
    let workers = if let Some(peer) = &peer_artifact {
        PeerWorker::spawn_with_timing(
            &options.worker,
            &options.devices,
            &[&artifact, peer],
            options.runtime,
            options.collective == EngineeringTpReductionModeV3::DevicePeerConcurrentV1,
            timing.clone(),
        )?
        .into_iter()
        .map(RankWorker::Peer)
        .collect::<Vec<_>>()
    } else {
        options
            .devices
            .iter()
            .enumerate()
            .map(|(rank, &id)| {
                let mut worker = Worker::spawn_with_timing(
                    &options.worker,
                    id,
                    &artifact,
                    options.runtime,
                    timing.clone(),
                    u32::try_from(rank).map_err(|_| "timing rank overflow")?,
                )?;
                if let Some(head) = &fp32_head_artifact {
                    worker.load_additional_artifact(head)?;
                }
                if let Some(argmax) = &fp32_argmax_artifact {
                    worker.load_additional_artifact(argmax)?;
                }
                if let Some(large) = &large_kv_artifact {
                    worker.load_additional_artifact(large)?;
                }
                Ok::<_, String>(RankWorker::Independent(worker))
            })
            .collect::<Result<Vec<_>, _>>()?
    };
    let pids = workers.iter().map(RankWorker::pid).collect::<Vec<_>>();
    let running_hashes = pids
        .iter()
        .map(|pid| hash_file(&PathBuf::from(format!("/proc/{pid}/exe"))))
        .collect::<Result<Vec<_>, _>>()?;
    if running_hashes.iter().any(|hash| hash != &worker_hash) {
        return Err("running worker file identity drifted".into());
    }
    drop(workers_timing);
    let resident_timing = timing.scope("setup_resident_weights");
    let driver_constructor = if options.large_kv_artifact.is_some() {
        EngineeringTpBatchExecutionV2::new_large_kv32
    } else if options.kernel_profile.wide32() {
        EngineeringTpBatchExecutionV2::new_wide32
    } else {
        EngineeringTpBatchExecutionV2::new
    };
    let mut gpu = if let Some(argmax) = &fp32_argmax_artifact {
        EngineeringTpBatchExecutionV2::new_full_forward_with_argmax_v11(
            workers,
            model.config(),
            model.target_weights(),
            model.layout(),
            &pool,
            argmax,
        )?
    } else {
        driver_constructor(
            workers,
            model.config(),
            model.target_weights(),
            model.layout(),
            &pool,
        )?
    };
    drop(resident_timing);
    gpu.configure_host_timing(timing.clone())?;
    let policies_timing = timing.scope("setup_execution_policies");
    gpu.configure_output_head_pruning(options.prune_output_head)?;
    gpu.configure_reduction(options.collective)?;
    gpu.configure_dispatch_sequences(options.runtime.sequences)?;
    {
        let _projection_timing = timing.scope("setup_projection");
        gpu.configure_projection(options.projection, model.target_weights(), model.layout())?;
    }
    gpu.configure_wave_attention(options.wave_attention)?;
    if let Some(precision) = options.head_precision {
        if precision.wide32() {
            gpu.configure_head_precision_v8(precision.fp32())?;
        } else {
            gpu.configure_head_precision_v7(precision.fp32())?;
        }
    }
    if options.ordered_scalar_v3 {
        gpu.configure_scalar_v3_ordered_batches()?;
    } else {
        gpu.configure_ordered_batches(options.runtime.ordered_batches)?;
    }
    if options.runtime.full_forward {
        if let Some(argmax) = &fp32_argmax_artifact {
            gpu.configure_mfma_v7_wave_argmax_full_forward(
                argmax,
                options.fp32_argmax == Some(Fp32Argmax::WaveV11),
            )?;
        } else if options.full_forward_mfma_v7_wave {
            gpu.configure_mfma_v7_wave_full_forward()?;
        } else if options.full_forward_mfma_v7 {
            gpu.configure_mfma_v7_full_forward()?;
        } else {
            gpu.configure_scalar_v3_full_forward()?;
        }
    }
    let fp32_head_workspace_bytes = gpu.fp32_head_workspace_bytes();
    if let Some(selection) = &options.numerical {
        let identity = serde_json::json!({
            "controller_sha256":controller_hash,"worker_sha256":worker_hash,
            "artifact_hsaco_id":hex(artifact.hsaco_id().as_bytes()),
            "artifact_manifest_id":hex(artifact.manifest_id().as_bytes()),
            "artifact_handoff_id":hex(artifact.handoff_id().as_bytes()),
            "model_bundle_id":hex(model.bundle_id().as_bytes()),"requests_sha256":loaded_requests_sha256,
            "session_id":hex(&session),"tensor_parallel":1,"device_unique_id":options.devices[0],
            "projection":options.projection.label(),"output_head_pruning":options.prune_output_head,
            "runtime_operational":options.runtime.operational,"runtime_cache_admission":options.runtime.cache_admission,
            "queue_rollover":options.runtime.rollover,"collective":options.collective.label(),
            "running_worker_sha256":running_hashes,"batch_tokens":options.rows,"prefill_chunk":options.chunk,
            "prefix_cache":options.cache
        });
        gpu.configure_numerical_capture(EngineeringTpNumericalCaptureV1::new(
            &selection.directory,
            selection.batch,
            selection.layer,
            selection.role,
            identity,
        )?)?;
    }
    drop(policies_timing);
    eprintln!(
        "additional resident transposed weight bytes: {}",
        gpu.transposed_weight_bytes()
    );
    let weight_payload_bytes = if control.is_some() {
        Some(serde_json::json!({
            "host_target":u64::try_from(model.target_weights().len()).map_err(|_| "host weight byte count overflow")?,
            "device_base":gpu.resident_weight_bytes()?,
            "device_transposed":gpu.transposed_weight_bytes()
        }))
    } else {
        None
    };
    let runtime_constructor = if options.kernel_profile.wide32() {
        EngineeringTpBatchRuntimeV2::new_wide32
    } else {
        EngineeringTpBatchRuntimeV2::new
    };
    let mut runtime = runtime_constructor(gpu, pool, scheduler, options.rows, options.cache)?;
    let mut benchmark_clock = None;
    drop(setup_timing);
    let result = (|| {
        let setup_seconds = whole.elapsed().as_secs_f64();
        if let Some(config) = control {
            let _barrier_timing = timing.scope("ready_barrier");
            benchmark_clock = Some(config.ready_and_wait()?);
        }
        let mut setup = serde_json::json!({"schema":"FerricQwen3TpBatchSetupV2", "authority":"none",
            "model":"Qwen/Qwen3-8B", "dtype":"BF16", "target":"gfx950:xnack-",
            "tensor_parallel":options.devices.len(), "device_unique_ids":options.devices,
            "worker_pids":pids, "worker_sha256":worker_hash, "running_worker_sha256":running_hashes,
            "controller_sha256":controller_hash, "model_bundle_id":hex(model.bundle_id().as_bytes()),
            "artifact_hsaco_id":hex(artifact.hsaco_id().as_bytes()), "artifact_manifest_id":hex(artifact.manifest_id().as_bytes()),
            "artifact_handoff_id":hex(artifact.handoff_id().as_bytes()), "session_id":hex(&session),
            "batch_tokens":options.rows, "prefill_chunk":options.chunk, "page_tokens":16,
            "physical_pages":options.pages, "context_tokens":options.context, "cache_ttl_ticks":options.ttl,
            "prefix_cache":options.cache, "max_batches":options.max_batches, "setup_seconds":setup_seconds,
            "output_head_pruning":options.prune_output_head,
            "performance_profile": {
                "runtime_cache_admission":options.runtime.cache_admission,
                "runtime_operational":options.runtime.operational,
                "dispatch_sequences":options.runtime.sequences,
                "queue_rollover":options.runtime.rollover,
                "projection":options.projection.label(), "attention":if options.wave_attention { "wave" } else { "baseline" }, "runtime_profiling":options.runtime.profile
            },
            "collective":if options.collective == EngineeringTpReductionModeV3::HostStagedV1 {
                "host_staged_fp32_rank_order_reduce_bf16_residual"
            } else { options.collective.label() },
            "prefill":"true_multirow_chunked", "attention":"paged_causal_gqa", "cache":"complete_page_radix_after_retirement",
            "arrival_policy":"logical batch ticks; elapsed latency starts at admission",
            "numerical_status":"Contracted; independently compare emitted token IDs; not a serving qualification"});
        if let Some(clock) = &benchmark_clock {
            setup["replica_benchmark"] =
                serde_json::to_value(clock.metadata()).map_err(|e| e.to_string())?;
            setup["weight_payload_bytes"] =
                weight_payload_bytes.ok_or("missing replica weight accounting")?;
        }
        if options.live_stdin() {
            setup["arrival_policy"] =
                serde_json::json!("live monotonic ingress timestamps; includes pending wait");
            setup["live_protocol"] =
                serde_json::json!("FerricQwen3TpLiveCommandV1/FerricQwen3TpLiveEventV1");
        }
        if options.runtime.shared_full_currentness {
            setup["peer_shared_full_currentness"] = serde_json::json!(true);
        }
        if options.runtime.ordered_batches {
            setup["runtime_ordered_batches"] = serde_json::json!(true);
        }
        if options.ordered_scalar_v3 {
            setup["ordered_batch_profile"] = serde_json::json!("scalar-v3-tp1-bf16");
        }
        if options.runtime.full_forward {
            setup["runtime_full_forward"] = serde_json::json!(true);
            setup["full_forward_profile"] = serde_json::json!(if options.fp32_argmax.is_some() {
                "mfma-v3-fp32-v7-wave-attention-v11-sidecar-tp1-616"
            } else if options.full_forward_mfma_v7_wave {
                "mfma-v3-fp32-v7-wave-attention-tp1-616"
            } else if options.full_forward_mfma_v7 {
                "mfma-v3-fp32-v7-tp1-616"
            } else {
                "scalar-v3-tp1-bf16-616"
            });
        }
        if let Some(large) = &large_kv_artifact {
            setup["kv_pool_profile"] = serde_json::json!("large-kv-v9");
            setup["kv_pool_max_physical_pages"] = serde_json::json!(16384);
            setup["kv_pool_payload_bytes"] = serde_json::json!(kv_pool_payload_bytes);
            setup["kv_pool_artifact"] = serde_json::json!({
                "artifact_hsaco_id":hex(large.hsaco_id().as_bytes()),
                "artifact_manifest_id":hex(large.manifest_id().as_bytes()),
                "artifact_handoff_id":hex(large.handoff_id().as_bytes())
            });
        }
        if options.runtime.profile {
            setup["runtime_diagnostic_status"] = serde_json::json!(
                "unqualified cumulative overlapping host-wall snapshots; deltas include the earlier snapshot command"
            );
            setup["numerical_status"] = serde_json::json!(
                "Diagnostic runtime host-wall counters enabled; timings are not performance qualified"
            );
        }
        if let Some(selection) = &options.numerical {
            setup["numerical_capture"] = serde_json::json!({"schema":"FerricTpNumericalSelectionV1",
                "batch_ordinal":selection.batch,"layer":selection.layer,"role":format!("{:?}",selection.role),
                "directory":selection.directory,"performance_qualified":false});
            setup["numerical_status"] = serde_json::json!(
                "Diagnostic readback run; all timings unqualified; fixed-reference token checks remain unchanged"
            );
        }
        if options.kernel_profile.wide32() {
            setup["kernel_profile"] = serde_json::json!(if options.kernel_profile.mfma() {
                "v5-mfma32"
            } else {
                "v5-wave32"
            });
            setup["kernel_row_capacity"] = serde_json::json!(32);
        }
        if let Some(peer) = &peer_artifact {
            setup["peer_artifact"] = serde_json::json!({
                "artifact_hsaco_id":hex(peer.hsaco_id().as_bytes()),
                "artifact_manifest_id":hex(peer.manifest_id().as_bytes()),
                "artifact_handoff_id":hex(peer.handoff_id().as_bytes())
            });
        }
        if let Some(head) = &fp32_head_artifact {
            setup["head_precision"] = serde_json::json!(
                options
                    .head_precision
                    .ok_or("missing head precision")?
                    .label()
            );
            setup["fp32_head_artifact"] = serde_json::json!({
                "artifact_hsaco_id":hex(head.hsaco_id().as_bytes()),
                "artifact_manifest_id":hex(head.manifest_id().as_bytes()),
                "artifact_handoff_id":hex(head.handoff_id().as_bytes())
            });
            setup["fp32_head_workspace_bytes"] = serde_json::json!(fp32_head_workspace_bytes);
        }
        if let Some(argmax) = &fp32_argmax_artifact {
            setup["fp32_argmax"] = serde_json::json!(
                options
                    .fp32_argmax
                    .ok_or("missing FP32 argmax mode")?
                    .label()
            );
            setup["fp32_argmax_artifact"] = serde_json::json!({
                "artifact_hsaco_id":hex(argmax.hsaco_id().as_bytes()),
                "artifact_manifest_id":hex(argmax.manifest_id().as_bytes()),
                "artifact_handoff_id":hex(argmax.handoff_id().as_bytes())
            });
        }
        if timing.is_enabled() {
            diagnostics.setup = Some(setup.clone());
        }
        emit(&setup)?;
        if options.runtime.profile {
            emit_runtime_diagnostic(&mut runtime, "before_workload")?;
        }
        let result = {
            let _workload_timing = timing.scope("workload");
            if options.live_stdin() {
                tp_live_ingress::run(
                    &mut runtime,
                    &model,
                    options.context,
                    options.pages,
                    options.max_batches,
                    &timing,
                    emit,
                )
            } else {
                run_workload(
                    &mut runtime,
                    &model,
                    workload.as_ref().ok_or("static workload missing")?,
                    &prompts,
                    options,
                    benchmark_clock.as_ref(),
                    &timing,
                )
            }
        };
        if options.runtime.profile && result.is_ok() {
            emit_runtime_diagnostic(&mut runtime, "after_workload")?;
        }
        result
    })();
    let close = {
        let _close_timing = timing.scope("close");
        runtime.close()
    };
    match (result, close) {
        (Ok(()), Ok(())) => {
            let numerical_closed = runtime.finish_numerical_capture()?;
            let replica_closed = benchmark_clock.map(BenchmarkClock::finish).transpose()?;
            let mut closed = serde_json::json!({"schema":"FerricQwen3TpBatchClosedV2", "authority":"none",
            "worker_pids":pids, "all_workers_exited":true, "rank_dispatch_counts":runtime.dispatch_counts(),
            "whole_seconds":whole.elapsed().as_secs_f64()});
            if let Some(receipt) = replica_closed {
                closed["replica_benchmark"] =
                    serde_json::to_value(receipt).map_err(|e| e.to_string())?;
            }
            if let Some(receipt) = numerical_closed {
                closed["numerical_capture"] = receipt;
            }
            if timing.is_enabled() {
                diagnostics.closed = Some(closed.clone());
            }
            emit(&closed)
        }
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Err(error)) => Err(format!("worker teardown: {error}")),
        (Err(error), Err(close)) => Err(format!("{error}; worker teardown: {close}")),
    }
}

fn main() -> std::process::ExitCode {
    match Options::parse(std::env::args().skip(1)).and_then(|options| run(&options)) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Qwen TP batch engineering failed: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(extra: &[&str]) -> Result<Options, String> {
        let mut base = vec![
            "--source",
            "/models",
            "--artifact",
            "/artifact",
            "--worker",
            "/worker",
            "--requests",
            "/requests",
            "--allow-unauthenticated-machine-code",
        ];
        base.extend(extra);
        Options::parse(base.into_iter().map(str::to_owned))
    }

    #[test]
    fn v7_wave_attention_requires_fp32_mfma_device_tp1_without_other_profile_changes() {
        let base = [
            "--devices",
            "1",
            "--kernel-profile",
            "v3-mfma",
            "--projection",
            "mfma",
            "--attention",
            "wave",
            "--collective",
            "device-tp1-v3",
            "--head-precision",
            "fp32-v7",
            "--fp32-head-artifact",
            "/v7",
        ];
        for rows in ["1", "3", "16"] {
            for prune in [false, true] {
                let mut values = base.to_vec();
                values.extend(["--batch-tokens", rows, "--prefill-chunk", rows]);
                if prune {
                    values.push("--prune-output-head");
                }
                let options = args(&values).unwrap();
                assert!(options.wave_attention);
                assert_eq!(options.head_precision, Some(HeadPrecision::Fp32));
                assert_eq!(options.projection, ProjectionMode::Mfma);
                assert_eq!(
                    options.collective,
                    EngineeringTpReductionModeV3::DeviceTp1V3
                );
                assert_eq!(options.prune_output_head, prune);
                assert!(!options.runtime.ordered_batches && !options.ordered_scalar_v3);
            }
        }
        for (flag, value) in [
            ("--devices", "1,2"),
            ("--kernel-profile", "v2"),
            ("--kernel-profile", "v3-wave"),
            ("--kernel-profile", "v5-mfma32"),
            ("--projection", "baseline"),
            ("--projection", "wave"),
            ("--projection", "auto"),
            ("--collective", "host-staged-reuse-v3"),
            ("--head-precision", "bf16-v7-control"),
            ("--head-precision", "fp32-v8"),
        ] {
            let mut values = base.to_vec();
            let index = values.iter().position(|&value| value == flag).unwrap();
            values[index + 1] = value;
            assert!(args(&values).is_err(), "{flag}={value}");
        }
        for extra in [
            vec!["--dispatch-sequences"],
            vec!["--runtime-ordered-batches"],
            vec!["--runtime-ordered-scalar-v3"],
            vec!["--benchmark-control", "/replica"],
            vec![
                "--kv-pool-profile",
                "large-kv-v9",
                "--large-kv-artifact",
                "/v9",
            ],
            vec![
                "--numerical-capture",
                "/capture",
                "--numerical-batch",
                "2",
                "--numerical-layer",
                "0",
                "--numerical-projection",
                "q",
            ],
        ] {
            let mut values = base.to_vec();
            values.extend(extra);
            assert!(args(&values).is_err());
        }
    }

    #[test]
    fn mfma_v7_full_forward_policy_keeps_scalar_and_other_profiles_separate() {
        let base = [
            "--devices",
            "1",
            "--kernel-profile",
            "v3-mfma",
            "--projection",
            "mfma",
            "--collective",
            "device-tp1-v3",
            "--head-precision",
            "fp32-v7",
            "--fp32-head-artifact",
            "/head",
            "--batch-tokens",
            "1",
            "--prefill-chunk",
            "1",
            "--context",
            "64",
            "--pages",
            "4",
            "--disable-prefix-cache",
        ];
        let serial = args(&base).unwrap();
        assert!(!serial.runtime.full_forward && !serial.full_forward_mfma_v7);
        let mut enabled = base.to_vec();
        enabled.push("--runtime-full-forward-mfma-v7");
        let parsed = args(&enabled).unwrap();
        assert!(parsed.runtime.full_forward && parsed.full_forward_mfma_v7);
        assert_eq!(parsed.head_precision, Some(HeadPrecision::Fp32));
        let mut operational = enabled.clone();
        operational.extend(["--runtime-cache-admission", "--runtime-operational"]);
        assert!(args(&operational).unwrap().full_forward_mfma_v7);
        for extra in [
            vec!["--runtime-full-forward"],
            vec!["--runtime-full-forward-mfma-v7"],
            vec!["--runtime-ordered-batches"],
            vec!["--runtime-ordered-scalar-v3"],
            vec!["--dispatch-sequences"],
            vec!["--queue-rollover"],
            vec!["--runtime-profile"],
            vec!["--prune-output-head"],
            vec!["--attention", "wave"],
            vec!["--peer-shared-full-currentness"],
            vec!["--peer-artifact", "/peer"],
            vec!["--host-timing", "/timing"],
            vec!["--benchmark-control", "/benchmark"],
            vec![
                "--kv-pool-profile",
                "large-kv-v9",
                "--large-kv-artifact",
                "/kv",
            ],
            vec![
                "--numerical-capture",
                "/capture",
                "--numerical-batch",
                "1",
                "--numerical-layer",
                "0",
                "--numerical-projection",
                "q",
            ],
        ] {
            let mut values = enabled.clone();
            values.extend(extra);
            assert!(args(&values).is_err());
        }
        for (flag, bad) in [
            ("--devices", "1,2"),
            ("--kernel-profile", "v2"),
            ("--kernel-profile", "v3-wave"),
            ("--kernel-profile", "v5-mfma32"),
            ("--projection", "baseline"),
            ("--projection", "wave"),
            ("--projection", "auto"),
            ("--head-precision", "bf16-v7-control"),
            ("--head-precision", "fp32-v8"),
            ("--collective", "host-staged-v1"),
            ("--collective", "host-staged-reuse-v3"),
            ("--batch-tokens", "2"),
            ("--context", "128"),
            ("--pages", "8"),
        ] {
            let mut values = enabled.clone();
            let index = values.iter().position(|item| *item == flag).unwrap();
            values[index + 1] = bad;
            assert!(args(&values).is_err(), "{flag} {bad}");
        }
        let mut scalar_selector = base.to_vec();
        scalar_selector.push("--runtime-full-forward");
        assert!(args(&scalar_selector).is_err());
        enabled.retain(|flag| *flag != "--disable-prefix-cache");
        assert!(args(&enabled).is_err());
    }

    #[test]
    fn mfma_v7_wave_full_forward_policy_requires_its_exact_attention_selector() {
        let base = [
            "--devices",
            "1",
            "--kernel-profile",
            "v3-mfma",
            "--projection",
            "mfma",
            "--attention",
            "wave",
            "--collective",
            "device-tp1-v3",
            "--head-precision",
            "fp32-v7",
            "--fp32-head-artifact",
            "/head",
            "--batch-tokens",
            "1",
            "--prefill-chunk",
            "1",
            "--context",
            "64",
            "--pages",
            "4",
            "--disable-prefix-cache",
        ];
        let serial = args(&base).unwrap();
        assert!(!serial.runtime.full_forward && !serial.full_forward_mfma_v7_wave);
        let mut enabled = base.to_vec();
        enabled.push("--runtime-full-forward-mfma-v7-wave");
        let parsed = args(&enabled).unwrap();
        assert!(parsed.runtime.full_forward && parsed.full_forward_mfma_v7_wave);
        assert!(!parsed.full_forward_mfma_v7 && parsed.wave_attention);
        let mut operational = enabled.clone();
        operational.extend(["--runtime-cache-admission", "--runtime-operational"]);
        assert!(args(&operational).unwrap().full_forward_mfma_v7_wave);
        for extra in [
            vec!["--runtime-full-forward"],
            vec!["--runtime-full-forward-mfma-v7"],
            vec!["--runtime-full-forward-mfma-v7-wave"],
            vec!["--runtime-ordered-batches"],
            vec!["--runtime-ordered-scalar-v3"],
            vec!["--dispatch-sequences"],
            vec!["--queue-rollover"],
            vec!["--runtime-profile"],
            vec!["--prune-output-head"],
            vec!["--peer-shared-full-currentness"],
            vec!["--peer-artifact", "/peer"],
            vec!["--host-timing", "/timing"],
            vec!["--benchmark-control", "/benchmark"],
            vec![
                "--numerical-capture",
                "/capture",
                "--numerical-batch",
                "1",
                "--numerical-layer",
                "0",
                "--numerical-projection",
                "q",
            ],
        ] {
            let mut values = enabled.clone();
            values.extend(extra);
            assert!(args(&values).is_err());
        }
        for (flag, bad) in [
            ("--devices", "1,2"),
            ("--kernel-profile", "v3-wave"),
            ("--projection", "baseline"),
            ("--projection", "wave"),
            ("--attention", "baseline"),
            ("--head-precision", "bf16-v7-control"),
            ("--head-precision", "fp32-v8"),
            ("--collective", "host-staged-v1"),
            ("--batch-tokens", "2"),
            ("--prefill-chunk", "2"),
            ("--context", "128"),
            ("--pages", "8"),
        ] {
            let mut values = enabled.clone();
            let index = values.iter().position(|item| *item == flag).unwrap();
            values[index + 1] = bad;
            assert!(args(&values).is_err(), "{flag} {bad}");
        }
        for selector in ["--runtime-full-forward", "--runtime-full-forward-mfma-v7"] {
            let mut values = base.to_vec();
            values.push(selector);
            assert!(args(&values).is_err());
        }
        enabled.retain(|flag| *flag != "--disable-prefix-cache");
        assert!(args(&enabled).is_err());
    }

    #[test]
    fn full_forward_argmax_v11_requires_same_sidecar_and_exact_wave_profile_for_both_modes() {
        let base = [
            "--devices",
            "1",
            "--kernel-profile",
            "v3-mfma",
            "--projection",
            "mfma",
            "--attention",
            "wave",
            "--collective",
            "device-tp1-v3",
            "--head-precision",
            "fp32-v7",
            "--fp32-head-artifact",
            "/head",
            "--batch-tokens",
            "1",
            "--prefill-chunk",
            "1",
            "--context",
            "64",
            "--pages",
            "4",
            "--disable-prefix-cache",
            "--runtime-full-forward-mfma-v7-wave",
        ];
        assert!(args(&base).unwrap().fp32_argmax.is_none());
        for (mode, expected) in [
            ("serial-v7", Fp32Argmax::SerialV7),
            ("wave-v11", Fp32Argmax::WaveV11),
        ] {
            let mut enabled = base.to_vec();
            enabled.extend(["--fp32-argmax", mode, "--fp32-argmax-artifact", "/argmax"]);
            let parsed = args(&enabled).unwrap();
            assert_eq!(parsed.fp32_argmax, Some(expected));
            assert_eq!(parsed.fp32_argmax_artifact, Some(PathBuf::from("/argmax")));
            assert!(parsed.runtime.full_forward && parsed.full_forward_mfma_v7_wave);
            for flag in ["--fp32-argmax", "--fp32-argmax-artifact"] {
                let mut missing = enabled.clone();
                let position = missing.iter().position(|item| *item == flag).unwrap();
                missing.drain(position..position + 2);
                assert!(args(&missing).is_err());
            }
            for (flag, value) in [
                ("--attention", "baseline"),
                ("--head-precision", "bf16-v7-control"),
                ("--head-precision", "fp32-v8"),
                ("--kernel-profile", "v5-mfma32"),
                ("--projection", "baseline"),
                ("--batch-tokens", "2"),
                ("--context", "128"),
                ("--pages", "5"),
                ("--collective", "host-staged-v1"),
                ("--devices", "1,2"),
                ("--fp32-argmax", "auto"),
            ] {
                let mut changed = enabled.clone();
                let position = changed.iter().position(|item| *item == flag).unwrap();
                changed[position + 1] = value;
                assert!(args(&changed).is_err(), "{flag} {value}");
            }
            for extra in [
                vec!["--runtime-ordered-batches"],
                vec!["--runtime-profile"],
                vec!["--prune-output-head"],
                vec!["--queue-rollover"],
                vec!["--host-timing", "/timing"],
                vec!["--runtime-full-forward-mfma-v7"],
                vec!["--fp32-argmax", mode],
            ] {
                let mut changed = enabled.clone();
                changed.extend(extra);
                assert!(args(&changed).is_err());
            }
            let without_seal = enabled
                .iter()
                .copied()
                .filter(|flag| *flag != "--runtime-full-forward-mfma-v7-wave")
                .collect::<Vec<_>>();
            assert!(args(&without_seal).is_err());
        }
    }

    #[test]
    fn full_forward_policy_is_distinct_explicit_and_narrow() {
        let base = [
            "--devices",
            "1",
            "--kernel-profile",
            "v3-wave",
            "--projection",
            "baseline",
            "--collective",
            "device-tp1-v3",
            "--batch-tokens",
            "1",
            "--prefill-chunk",
            "1",
            "--context",
            "64",
            "--pages",
            "4",
            "--disable-prefix-cache",
        ];
        assert!(!args(&base).unwrap().runtime.full_forward);
        let mut enabled = base.to_vec();
        enabled.push("--runtime-full-forward");
        assert!(args(&enabled).unwrap().runtime.full_forward);
        let mut operational = enabled.clone();
        operational.extend(["--runtime-cache-admission", "--runtime-operational"]);
        let options = args(&operational).unwrap();
        assert!(
            options.runtime.full_forward
                && options.runtime.cache_admission
                && options.runtime.operational
        );
        for extra in [
            vec!["--runtime-full-forward"],
            vec!["--runtime-ordered-batches"],
            vec!["--runtime-ordered-scalar-v3"],
            vec!["--dispatch-sequences"],
            vec!["--queue-rollover"],
            vec!["--runtime-profile"],
            vec!["--prune-output-head"],
            vec!["--attention", "wave"],
            vec!["--peer-shared-full-currentness"],
            vec!["--peer-artifact", "/peer"],
            vec!["--host-timing", "/timing"],
            vec!["--benchmark-control", "/benchmark"],
            vec![
                "--head-precision",
                "fp32-v7",
                "--fp32-head-artifact",
                "/head",
            ],
            vec![
                "--head-precision",
                "bf16-v7-control",
                "--fp32-head-artifact",
                "/head",
            ],
            vec![
                "--kv-pool-profile",
                "large-kv-v9",
                "--large-kv-artifact",
                "/kv",
            ],
            vec![
                "--numerical-capture",
                "/capture",
                "--numerical-batch",
                "1",
                "--numerical-layer",
                "0",
                "--numerical-projection",
                "q",
            ],
        ] {
            let mut values = enabled.clone();
            values.extend(extra);
            assert!(args(&values).is_err());
        }
        for (flag, bad) in [
            ("--devices", "1,2"),
            ("--kernel-profile", "v2"),
            ("--kernel-profile", "v3-mfma"),
            ("--kernel-profile", "v5-wave32"),
            ("--projection", "wave"),
            ("--projection", "mfma"),
            ("--collective", "host-staged-v1"),
            ("--collective", "host-staged-reuse-v3"),
            ("--batch-tokens", "2"),
            ("--context", "128"),
            ("--pages", "8"),
        ] {
            let mut values = enabled.clone();
            let index = values.iter().position(|item| *item == flag).unwrap();
            values[index + 1] = bad;
            assert!(args(&values).is_err(), "{flag} {bad}");
        }
        enabled.retain(|flag| *flag != "--disable-prefix-cache");
        assert!(args(&enabled).is_err());
    }

    #[test]
    fn ordered_scalar_v3_policy_is_separate_explicit_and_narrow() {
        let base = [
            "--devices",
            "1",
            "--kernel-profile",
            "v3-wave",
            "--projection",
            "baseline",
            "--collective",
            "device-tp1-v3",
        ];
        let serial = args(&base).unwrap();
        assert!(!serial.ordered_scalar_v3 && !serial.runtime.ordered_batches);
        let mut enabled = base.to_vec();
        enabled.push("--runtime-ordered-scalar-v3");
        let parsed = args(&enabled).unwrap();
        assert!(parsed.ordered_scalar_v3 && parsed.runtime.ordered_batches);
        assert!(!parsed.runtime.sequences && !parsed.runtime.rollover);
        assert!(!parsed.prune_output_head && parsed.head_precision.is_none());
        for rows in ["1", "3", "16"] {
            let mut flags = enabled.clone();
            flags.extend(["--batch-tokens", rows, "--prefill-chunk", rows]);
            assert!(args(&flags).unwrap().ordered_scalar_v3);
        }
        let mut diagnostic = enabled.clone();
        diagnostic.extend([
            "--runtime-profile",
            "--runtime-cache-admission",
            "--runtime-operational",
        ]);
        assert!(args(&diagnostic).unwrap().runtime.profile);
        for extra in [
            vec!["--runtime-ordered-batches"],
            vec!["--runtime-ordered-scalar-v3"],
            vec!["--dispatch-sequences"],
            vec!["--queue-rollover"],
            vec!["--prune-output-head"],
            vec!["--attention", "wave"],
            vec!["--peer-shared-full-currentness"],
            vec!["--peer-artifact", "/peer"],
            vec![
                "--head-precision",
                "bf16-v7-control",
                "--fp32-head-artifact",
                "/head",
            ],
            vec![
                "--head-precision",
                "fp32-v7",
                "--fp32-head-artifact",
                "/head",
            ],
            vec![
                "--kv-pool-profile",
                "large-kv-v9",
                "--large-kv-artifact",
                "/kv",
            ],
            vec!["--benchmark-control", "/replica"],
            vec![
                "--numerical-capture",
                "/capture",
                "--numerical-batch",
                "1",
                "--numerical-layer",
                "0",
                "--numerical-projection",
                "q",
            ],
        ] {
            let mut flags = enabled.clone();
            flags.extend(extra);
            assert!(args(&flags).is_err());
        }
        for (flag, bad) in [
            ("--devices", "1,2"),
            ("--kernel-profile", "v2"),
            ("--kernel-profile", "v3-mfma"),
            ("--kernel-profile", "v5-wave32"),
            ("--kernel-profile", "v5-mfma32"),
            ("--projection", "wave"),
            ("--projection", "mfma"),
            ("--projection", "auto"),
            ("--collective", "host-staged-v1"),
            ("--collective", "host-staged-reuse-v3"),
        ] {
            let mut flags = enabled.clone();
            let index = flags.iter().position(|item| *item == flag).unwrap();
            flags[index + 1] = bad;
            assert!(args(&flags).is_err(), "{flag} {bad}");
        }
        let mut legacy = base.to_vec();
        legacy.push("--runtime-ordered-batches");
        assert!(args(&legacy).is_err());
    }

    #[test]
    fn ordered_batch_policy_is_distinct_explicit_and_narrow() {
        let base = [
            "--devices",
            "1",
            "--kernel-profile",
            "v5-mfma32",
            "--head-precision",
            "fp32-v8",
            "--fp32-head-artifact",
            "/head",
            "--projection",
            "mfma",
            "--collective",
            "device-tp1-v3",
            "--prune-output-head",
        ];
        assert!(!args(&base).unwrap().runtime.ordered_batches);
        let mut enabled = base.to_vec();
        enabled.push("--runtime-ordered-batches");
        let parsed = args(&enabled).unwrap();
        assert!(parsed.runtime.ordered_batches);
        assert!(!parsed.runtime.sequences);
        let mut rollover = enabled.clone();
        rollover.push("--queue-rollover");
        assert!(args(&rollover).unwrap().runtime.rollover);
        let mut wave = rollover.clone();
        wave.extend(["--attention", "wave"]);
        let parsed = args(&wave).unwrap();
        assert!(parsed.wave_attention && parsed.runtime.ordered_batches && parsed.runtime.rollover);
        for extra in [
            vec!["--dispatch-sequences"],
            vec!["--benchmark-control", "/replica"],
            vec![
                "--kv-pool-profile",
                "large-kv-v9",
                "--large-kv-artifact",
                "/kv",
            ],
            vec![
                "--numerical-capture",
                "/capture",
                "--numerical-batch",
                "1",
                "--numerical-layer",
                "0",
                "--numerical-projection",
                "q",
            ],
        ] {
            let mut flags = enabled.clone();
            flags.extend(extra);
            assert!(args(&flags).is_err());
        }
        for (flag, bad) in [
            ("--devices", "1,2"),
            ("--kernel-profile", "v5-wave32"),
            ("--head-precision", "fp32-v7"),
            ("--projection", "baseline"),
            ("--projection", "wave"),
            ("--collective", "host-staged-v1"),
        ] {
            let mut flags = enabled.clone();
            let index = flags.iter().position(|item| *item == flag).unwrap();
            flags[index + 1] = bad;
            assert!(args(&flags).is_err());
        }
        enabled.retain(|flag| *flag != "--prune-output-head");
        assert!(args(&enabled).is_err());
    }

    #[test]
    fn large_kv_profile_is_explicit_bounded_and_excludes_unsupported_combinations() {
        let base = [
            "--devices",
            "1",
            "--kernel-profile",
            "v5-mfma32",
            "--head-precision",
            "fp32-v8",
            "--fp32-head-artifact",
            "/head",
            "--kv-pool-profile",
            "large-kv-v9",
            "--large-kv-artifact",
            "/large",
        ];
        for pages in ["1", "512", "513", "8192", "8704", "16384"] {
            let mut flags = base.to_vec();
            flags.extend(["--pages", pages, "--context", "8192"]);
            let parsed = args(&flags).unwrap();
            assert!(parsed.large_kv_artifact.is_some());
            assert_eq!(
                parsed
                    .paged_limits()
                    .unwrap()
                    .physical_page_count()
                    .to_string(),
                pages
            );
            assert_eq!(parsed.paged_limits().unwrap().context_tokens(), 8192);
        }
        for extra in [
            vec!["--pages", "0"],
            vec!["--pages", "16385"],
            vec!["--pages", "4294967295"],
            vec!["--context", "8193"],
            vec!["--attention", "wave"],
            vec!["--projection", "wave"],
            vec!["--projection", "auto"],
            vec!["--dispatch-sequences"],
            vec!["--benchmark-control", "/replica"],
            vec!["--peer-artifact", "/peer"],
            vec!["--numerical-capture", "/capture"],
        ] {
            let mut flags = base.to_vec();
            flags.extend(extra);
            assert!(args(&flags).is_err());
        }
        for (index, replacement) in [
            (1, "1,2"),
            (1, "1,2,3,4,5,6,7,8"),
            (3, "v3-mfma"),
            (5, "fp32-v7"),
            (9, "unknown"),
        ] {
            let mut flags = base;
            flags[index] = replacement;
            assert!(args(&flags).is_err());
        }
        let mut mfma = base.to_vec();
        mfma.extend([
            "--projection",
            "mfma",
            "--collective",
            "device-tp1-v3",
            "--queue-rollover",
        ]);
        assert!(args(&mfma).is_ok());
        assert!(args(&base[..10]).is_err());
        let mut missing_profile = base[..8].to_vec();
        missing_profile.extend(["--large-kv-artifact", "/large"]);
        assert!(args(&missing_profile).is_err());
        let mut legacy = base[..8].to_vec();
        legacy.extend(["--pages", "513"]);
        assert!(args(&legacy).is_err());
    }

    #[test]
    fn live_stdin_is_explicit_exclusive_and_requires_rollover() {
        let parse = |extra: &[&str]| {
            let mut base = vec![
                "--source",
                "/models",
                "--artifact",
                "/artifact",
                "--worker",
                "/worker",
                "--devices",
                "1",
                "--allow-unauthenticated-machine-code",
            ];
            base.extend(extra);
            Options::parse(base.into_iter().map(str::to_owned))
        };
        let live = parse(&[
            "--live-stdin",
            "--queue-rollover",
            "--max-batches",
            "1000000",
        ])
        .unwrap();
        assert!(live.live_stdin());
        assert!(live.requests.is_none());
        assert_eq!(live.max_batches, 1_000_000);
        for extra in [
            vec![],
            vec!["--live-stdin"],
            vec![
                "--live-stdin",
                "--queue-rollover",
                "--requests",
                "/requests",
            ],
            vec![
                "--live-stdin",
                "--queue-rollover",
                "--benchmark-control",
                "/control",
            ],
            vec![
                "--live-stdin",
                "--queue-rollover",
                "--numerical-capture",
                "/capture",
            ],
            vec![
                "--live-stdin",
                "--queue-rollover",
                "--max-batches",
                "1000001",
            ],
        ] {
            assert!(parse(&extra).is_err());
        }
        let ordinary = args(&["--devices", "1"]).unwrap();
        assert!(!ordinary.live_stdin());
        assert_eq!(ordinary.requests.as_deref(), Some(Path::new("/requests")));
    }

    #[test]
    fn runtime_diagnostics_and_shared_peer_currentness_are_explicit() {
        let ordinary = args(&["--devices", "1"]).unwrap();
        assert!(!ordinary.runtime.profile);
        assert!(!ordinary.runtime.shared_full_currentness);
        assert!(
            args(&["--devices", "1", "--runtime-profile"])
                .unwrap()
                .runtime
                .profile
        );
        assert!(
            args(&[
                "--devices",
                "1",
                "--runtime-profile",
                "--benchmark-control",
                "/replica"
            ])
            .is_err()
        );
        assert!(
            args(&[
                "--devices",
                "1",
                "--runtime-profile",
                "--numerical-capture",
                "/capture"
            ])
            .is_err()
        );
        assert!(args(&["--devices", "1", "--peer-shared-full-currentness"]).is_err());
        assert!(args(&["--devices", "1,2", "--peer-shared-full-currentness"]).is_err());
        for collective in ["device-peer-serial-v4", "device-peer-concurrent-round-v1"] {
            let options = args(&[
                "--devices",
                "1,2",
                "--collective",
                collective,
                "--peer-artifact",
                "/peer",
                "--peer-shared-full-currentness",
                "--runtime-profile",
            ])
            .unwrap();
            assert!(options.runtime.shared_full_currentness);
            assert!(options.runtime.profile);
        }
    }

    #[test]
    fn v8_head_is_only_available_with_explicit_tp1_wide_profiles() {
        for mode in ["bf16-v8-control", "fp32-v8"] {
            for profile in ["v5-wave32", "v5-mfma32"] {
                let base = [
                    "--devices",
                    "1",
                    "--kernel-profile",
                    profile,
                    "--head-precision",
                    mode,
                    "--fp32-head-artifact",
                    "/v8",
                ];
                let options = args(&base).unwrap();
                let head = options.head_precision.unwrap();
                assert_eq!(head.label(), mode);
                assert!(head.wide32());
                assert_eq!(head.fp32(), mode == "fp32-v8");
                let mut wave = base.to_vec();
                wave.extend(["--attention", "wave"]);
                assert!(args(&wave).unwrap().wave_attention);
                for extra in [
                    vec!["--dispatch-sequences"],
                    vec!["--projection", "wave"],
                    vec!["--projection", "auto"],
                    vec!["--benchmark-control", "/control"],
                ] {
                    let mut flags = base.to_vec();
                    flags.extend(extra);
                    assert!(args(&flags).is_err());
                }
            }
            assert!(
                args(&[
                    "--devices",
                    "1",
                    "--kernel-profile",
                    "v5-mfma32",
                    "--projection",
                    "mfma",
                    "--head-precision",
                    mode,
                    "--fp32-head-artifact",
                    "/v8"
                ])
                .is_ok()
            );
            assert!(
                args(&[
                    "--devices",
                    "1",
                    "--kernel-profile",
                    "v5-mfma32",
                    "--projection",
                    "mfma",
                    "--attention",
                    "wave",
                    "--head-precision",
                    mode,
                    "--fp32-head-artifact",
                    "/v8",
                ])
                .is_ok()
            );
            for devices in ["1,2", "1,2,3,4,5,6,7,8"] {
                assert!(
                    args(&[
                        "--devices",
                        devices,
                        "--kernel-profile",
                        "v5-mfma32",
                        "--head-precision",
                        mode,
                        "--fp32-head-artifact",
                        "/v8"
                    ])
                    .is_err()
                );
            }
            for profile in ["v2", "v3-wave", "v3-mfma"] {
                assert!(
                    args(&[
                        "--devices",
                        "1",
                        "--kernel-profile",
                        profile,
                        "--head-precision",
                        mode,
                        "--fp32-head-artifact",
                        "/v8"
                    ])
                    .is_err()
                );
            }
            assert!(
                args(&[
                    "--devices",
                    "1",
                    "--kernel-profile",
                    "v5-mfma32",
                    "--head-precision",
                    mode
                ])
                .is_err()
            );
        }
    }

    #[test]
    fn fp32_head_and_explicit_control_require_closed_supported_profiles() {
        assert!(args(&["--devices", "1"]).unwrap().head_precision.is_none());
        for mode in ["fp32-v7", "bf16-v7-control"] {
            let base = [
                "--devices",
                "1",
                "--head-precision",
                mode,
                "--fp32-head-artifact",
                "/v7",
            ];
            let options = args(&base).unwrap();
            assert_eq!(options.head_precision.unwrap().label(), mode);
            assert_eq!(options.fp32_head_artifact, Some(PathBuf::from("/v7")));
            assert!(args(&["--devices", "1", "--head-precision", mode]).is_err());
            for extra in [
                vec!["--dispatch-sequences"],
                vec!["--benchmark-control", "/replica"],
                vec!["--kernel-profile", "v3-mfma", "--projection", "auto"],
                vec!["--kernel-profile", "v3-wave", "--projection", "wave"],
                vec!["--kernel-profile", "v3-wave", "--attention", "wave"],
                vec!["--kernel-profile", "v5-mfma32"],
                vec![
                    "--numerical-capture",
                    "/capture",
                    "--numerical-batch",
                    "2",
                    "--numerical-layer",
                    "0",
                    "--numerical-projection",
                    "q",
                ],
            ] {
                let mut values = base.to_vec();
                values.extend(extra);
                assert!(args(&values).is_err());
            }
            for devices in ["1,2", "1,2,3,4,5,6,7,8"] {
                assert!(
                    args(&[
                        "--devices",
                        devices,
                        "--head-precision",
                        mode,
                        "--fp32-head-artifact",
                        "/v7"
                    ])
                    .is_err()
                );
            }
            let mut mfma = base.to_vec();
            mfma.extend(["--kernel-profile", "v3-mfma", "--projection", "mfma"]);
            assert!(args(&mfma).is_ok());
        }
        assert!(args(&["--devices", "1", "--fp32-head-artifact", "/v7"]).is_err());
        assert!(
            args(&[
                "--devices",
                "1",
                "--head-precision",
                "fp64",
                "--fp32-head-artifact",
                "/v7"
            ])
            .is_err()
        );
    }

    #[test]
    fn host_timing_is_explicit_single_valued_and_does_not_change_policy() {
        let ordinary = args(&["--devices", "1"]).unwrap();
        assert!(ordinary.host_timing.is_none());
        let profiled = args(&["--devices", "1", "--host-timing", "/private/timing.json"]).unwrap();
        assert_eq!(
            profiled.host_timing,
            Some(PathBuf::from("/private/timing.json"))
        );
        assert_eq!(
            (
                profiled.runtime.cache_admission,
                profiled.runtime.operational,
                profiled.runtime.sequences,
                profiled.runtime.rollover
            ),
            (
                ordinary.runtime.cache_admission,
                ordinary.runtime.operational,
                ordinary.runtime.sequences,
                ordinary.runtime.rollover
            )
        );
        assert_eq!(profiled.collective, ordinary.collective);
        assert!(args(&["--devices", "1", "--host-timing"]).is_err());
        assert!(
            args(&[
                "--devices",
                "1",
                "--host-timing",
                "/a",
                "--host-timing",
                "/b"
            ])
            .is_err()
        );
    }

    #[test]
    fn numerical_capture_requires_explicit_bounded_selection_and_no_performance_modes() {
        assert!(args(&["--devices", "1"]).unwrap().numerical.is_none());
        let flags = [
            "--numerical-capture",
            "/tmp/capture",
            "--numerical-batch",
            "2",
            "--numerical-layer",
            "0",
            "--numerical-projection",
            "q",
            "--devices",
            "1",
        ];
        let selected = args(&flags).unwrap();
        assert_eq!(selected.numerical.unwrap().batch, 2);
        for extra in [
            vec!["--host-timing", "/tmp/timing"],
            vec!["--benchmark-control", "/tmp/control"],
            vec!["--dispatch-sequences"],
            vec!["--kernel-profile", "v5-mfma32"],
        ] {
            let mut input = flags.to_vec();
            input.extend(extra);
            assert!(args(&input).is_err());
        }
        for invalid in [
            vec!["--numerical-capture", "/tmp/capture"],
            vec!["--numerical-batch", "2"],
            vec!["--numerical-layer", "0"],
            vec!["--numerical-projection", "q"],
        ] {
            assert!(args(&invalid).is_err());
        }
        for (index, value) in [(3, "0"), (3, "65"), (5, "36"), (7, "unknown")] {
            let mut input = flags.to_vec();
            input[index] = value;
            assert!(args(&input).is_err());
        }
    }
    #[test]
    fn options_enforce_physical_roster_rows_and_ring_bound() {
        assert_eq!(
            args(&["--devices", "1,2,3,4,5,6,7,8"])
                .unwrap()
                .devices
                .len(),
            8
        );
        for extra in [
            vec!["--devices", "1,1"],
            vec!["--devices", "0"],
            vec!["--devices", "1,2,3"],
            vec!["--devices", "1", "--max-batches", "241"],
            vec!["--devices", "1", "--batch-tokens", "17"],
            vec!["--devices", "1", "--prefill-chunk", "0"],
            vec!["--devices", "1", "--pages", "513"],
            vec!["--devices", "1", "--devices", "2"],
        ] {
            assert!(args(&extra).is_err());
        }
        assert!(Options::parse(["--devices".into(), "1".into()].into_iter()).is_err());
    }

    #[test]
    fn replica_control_is_explicit_and_single_valued() {
        assert!(
            args(&["--devices", "1"])
                .unwrap()
                .benchmark_control
                .is_none()
        );
        assert_eq!(
            args(&[
                "--devices",
                "1",
                "--benchmark-control",
                "/private/control.json"
            ])
            .unwrap()
            .benchmark_control,
            Some(PathBuf::from("/private/control.json"))
        );
        assert!(args(&["--devices", "1", "--benchmark-control"]).is_err());
        assert!(
            args(&[
                "--devices",
                "1",
                "--benchmark-control",
                "/a",
                "--benchmark-control",
                "/b"
            ])
            .is_err()
        );
    }

    #[test]
    fn timed_step_uses_both_clock_samples_without_changing_origin() {
        let mut samples = [Ok(101), Ok(203)].into_iter();
        let result = timed_step(
            || samples.next().unwrap(),
            |started, completed| Ok((started, completed())),
        )
        .unwrap();
        assert_eq!(result, (101, 203));
        assert!(samples.next().is_none());
    }

    #[test]
    fn timed_step_rejects_start_clock_failure_before_dispatch() {
        let mut called = false;
        let result = timed_step(
            || Err("start clock failed".into()),
            |_, _| {
                called = true;
                Ok(())
            },
        );
        assert_eq!(result.unwrap_err(), "start clock failed");
        assert!(!called);
    }

    #[test]
    fn timed_step_never_accepts_a_report_after_completion_clock_failure() {
        for step_succeeds in [false, true] {
            let mut samples = [Ok(101), Err("completion clock failed".into())].into_iter();
            let result = timed_step(
                || samples.next().unwrap(),
                |_, completed| {
                    let value = completed();
                    if step_succeeds {
                        Ok(value)
                    } else {
                        Err("coordinator failed".into())
                    }
                },
            );
            assert_eq!(result.unwrap_err(), "completion clock failed");
            assert!(samples.next().is_none());
        }
    }

    #[test]
    fn workload_schema_and_request_bounds_are_closed() {
        let valid = serde_json::json!({"schema":"FerricQwen3TpWorkloadV2","requests":[
            {"name":"a","prompt":"The capital of France is","new_tokens":2,"arrival_tick":0}]});
        assert!(Workload::parse(&serde_json::to_vec(&valid).unwrap()).is_ok());
        let mut duplicate = valid.clone();
        duplicate["requests"]
            .as_array_mut()
            .unwrap()
            .push(valid["requests"][0].clone());
        assert!(Workload::parse(&serde_json::to_vec(&duplicate).unwrap()).is_err());
        for (key, value) in [
            ("new_tokens", serde_json::json!(0)),
            ("arrival_tick", serde_json::json!(10001)),
            ("prompt", serde_json::json!("")),
            ("unknown", serde_json::json!(1)),
        ] {
            let mut bad = valid.clone();
            bad["requests"][0][key] = value;
            assert!(Workload::parse(&serde_json::to_vec(&bad).unwrap()).is_err());
        }
        assert!(Workload::parse(&vec![0; 1_048_577]).is_err());
    }

    #[test]
    fn peer_mode_requires_its_own_image_world_and_supported_transport_options() {
        let base = [
            "--devices",
            "1,2",
            "--collective",
            "device-peer-serial-v4",
            "--peer-artifact",
            "/peer",
        ];
        let options = args(&base).unwrap();
        assert_eq!(options.max_batches, 212);
        assert_eq!(options.peer_artifact, Some(PathBuf::from("/peer")));
        for flag in [
            "--runtime-cache-admission",
            "--runtime-operational",
            "--dispatch-sequences",
        ] {
            let mut supplied = base.to_vec();
            supplied.push(flag);
            assert!(args(&supplied).is_ok());
        }
        let mut rollover = base.to_vec();
        rollover.push("--queue-rollover");
        assert!(args(&rollover).is_err());
        for supplied in [
            vec![
                "--devices",
                "1",
                "--collective",
                "device-peer-serial-v4",
                "--peer-artifact",
                "/peer",
            ],
            vec!["--devices", "1,2", "--collective", "device-peer-serial-v4"],
            vec!["--devices", "1,2", "--peer-artifact", "/peer"],
        ] {
            assert!(args(&supplied).is_err());
        }
    }

    #[test]
    fn concurrent_peer_profile_is_explicit_and_rejects_same_rank_sequences() {
        let base = [
            "--devices",
            "1,2",
            "--collective",
            "device-peer-concurrent-round-v1",
            "--peer-artifact",
            "/peer",
        ];
        let options = args(&base).unwrap();
        assert_eq!(
            options.collective.label(),
            "device-peer-concurrent-round-v1"
        );
        assert_eq!(options.max_batches, 212);
        for flag in ["--dispatch-sequences", "--queue-rollover"] {
            let mut invalid = base.to_vec();
            invalid.push(flag);
            assert!(args(&invalid).is_err());
        }
        let mut invalid = base;
        invalid[1] = "1";
        assert!(args(&invalid).is_err());
        assert!(args(&base[..4]).is_err());
    }

    #[test]
    fn thirty_two_rows_require_the_separate_image_profile() {
        for profile in ["v5-wave32", "v5-mfma32"] {
            let options = args(&[
                "--devices",
                "1,2,3,4,5,6,7,8",
                "--kernel-profile",
                profile,
                "--batch-tokens",
                "32",
                "--prefill-chunk",
                "32",
            ])
            .unwrap();
            assert!(options.kernel_profile.wide32());
            assert_eq!(options.rows, 32);
            assert!(
                args(&[
                    "--devices",
                    "1",
                    "--kernel-profile",
                    profile,
                    "--batch-tokens",
                    "33"
                ])
                .is_err()
            );
        }
        assert!(
            args(&[
                "--devices",
                "1",
                "--kernel-profile",
                "v3-mfma",
                "--batch-tokens",
                "32"
            ])
            .is_err()
        );
        assert!(
            args(&[
                "--devices",
                "1",
                "--kernel-profile",
                "v5-wave32",
                "--projection",
                "mfma"
            ])
            .is_err()
        );
        assert!(
            args(&[
                "--devices",
                "1,2",
                "--kernel-profile",
                "v5-wave32",
                "--collective",
                "device-peer-serial-v4",
                "--peer-artifact",
                "/peer32",
                "--batch-tokens",
                "32"
            ])
            .is_ok()
        );
    }

    #[test]
    fn performance_modes_require_exact_artifact_capability_and_world() {
        for options in [
            vec!["--projection", "wave"],
            vec!["--attention", "wave"],
            vec!["--kernel-profile", "v3-wave", "--projection", "auto"],
            vec!["--kernel-profile", "v3-wave", "--projection", "mfma"],
            vec!["--kernel-profile", "unknown"],
            vec!["--kernel-profile", "v3-wave", "--attention", "unknown"],
            vec!["--collective", "device-tp1-v3"],
        ] {
            let mut supplied = vec!["--devices", "1"];
            supplied.extend(options);
            assert!(args(&supplied).is_err());
        }
        assert!(
            args(&[
                "--devices",
                "1,2",
                "--kernel-profile",
                "v3-wave",
                "--collective",
                "device-tp1-v3"
            ])
            .is_err()
        );
        let valid = args(&[
            "--devices",
            "1",
            "--kernel-profile",
            "v3-wave",
            "--collective",
            "device-tp1-v3",
            "--projection",
            "wave",
            "--attention",
            "wave",
        ])
        .unwrap();
        assert_eq!(valid.max_batches, 212);
        assert!(
            args(&[
                "--devices",
                "1,2",
                "--kernel-profile",
                "v3-mfma",
                "--projection",
                "auto"
            ])
            .is_ok()
        );
        assert!(
            args(&[
                "--devices",
                "1,2",
                "--queue-rollover",
                "--max-batches",
                "1000000"
            ])
            .is_ok()
        );
        assert!(
            args(&[
                "--devices",
                "1,2",
                "--queue-rollover",
                "--max-batches",
                "1000001"
            ])
            .is_err()
        );
    }
}
