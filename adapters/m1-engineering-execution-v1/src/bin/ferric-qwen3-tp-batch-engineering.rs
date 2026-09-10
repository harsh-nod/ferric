//! Bounded, explicitly non-authoritative continuous Qwen workload runner.

mod tp_benchmark_control;
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
use ferric_m1_engineering_execution_v1::tp_execution::{
    EngineeringTpProjectionModeV3 as ProjectionMode, EngineeringTpReductionModeV3,
};
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
    "--devices ID[,ID...] --requests FILE --allow-unauthenticated-machine-code ",
    "[--batch-tokens 16] [--prefill-chunk 16] [--context 128] [--pages 64] ",
    "[--cache-ttl 1024] [--max-batches 240] [--disable-prefix-cache] [--prune-output-head] ",
    "[--runtime-cache-admission] [--runtime-operational] [--dispatch-sequences] [--queue-rollover] ",
    "[--collective host-staged-v1|host-staged-reuse-v3|device-tp1-v3|device-peer-serial-v4] ",
    "[--peer-artifact DIR] [--kernel-profile v2|v3-wave|v3-mfma|v5-wave32|v5-mfma32] ",
    "[--projection baseline|wave|mfma|auto] [--attention baseline|wave] ",
    "[--benchmark-control FILE]"
);

struct Options {
    source: PathBuf,
    artifact: PathBuf,
    peer_artifact: Option<PathBuf>,
    worker: PathBuf,
    requests: PathBuf,
    benchmark_control: Option<PathBuf>,
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
    collective: EngineeringTpReductionModeV3,
    kernel_profile: KernelProfile,
    projection: ProjectionMode,
    wave_attention: bool,
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
    fn parse(args: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut args = args.peekable();
        let mut seen = BTreeSet::new();
        let (mut source, mut artifact, mut worker, mut requests, mut devices) =
            (None, None, None, None, None);
        let (mut rows, mut chunk, mut context, mut pages, mut ttl, mut max_batches) =
            (16, 16, 128, 64, 1024, 240);
        let (mut consent, mut cache) = (false, true);
        let mut prune_output_head = false;
        let mut runtime = RuntimeOptions::default();
        let mut collective = EngineeringTpReductionModeV3::HostStagedV1;
        let mut kernel_profile = KernelProfile::V2;
        let mut projection = ProjectionMode::Baseline;
        let mut wave_attention = false;
        let mut peer_artifact = None;
        let mut benchmark_control = None;
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
                "--queue-rollover" => {
                    runtime.rollover = true;
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
                "--worker" => worker = Some(PathBuf::from(value)),
                "--requests" => requests = Some(PathBuf::from(value)),
                "--benchmark-control" => benchmark_control = Some(PathBuf::from(value)),
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
        let peer = collective == EngineeringTpReductionModeV3::DevicePeerV4;
        if peer != peer_artifact.is_some()
            || (peer && !matches!(devices.len(), 2 | 8))
            || (peer && runtime.rollover)
        {
            return Err("serial peer transport requires TP2/8 and --peer-artifact; peer queue rollover is not supported".into());
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
        EngineeringTpPagedLimitsV1::new(context, 32, pages, ttl)
            .map_err(|e| format!("page limits: {e:?}"))?;
        Ok(Self {
            source: source.ok_or("--source is required")?,
            artifact: artifact.ok_or("--artifact is required")?,
            peer_artifact,
            worker: worker.ok_or("--worker is required")?,
            requests: requests.ok_or("--requests is required")?,
            benchmark_control,
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
        let report = timed_step(now, |started, completed| {
            runtime.step(tick, started, completed)
        })?
        .ok_or("active requests produced no batch")?;
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
    let whole = Instant::now();
    let (workload, loaded_requests_sha256) = Workload::open(&options.requests)?;
    let control = options
        .benchmark_control
        .as_ref()
        .map(|path| {
            let config = ControlConfig::open(path, &options.requests, &options.devices)?;
            config.verify_loaded_requests_sha256(&loaded_requests_sha256)?;
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
    let model = EngineeringQwenModelV1::open(&options.source)?;
    let prompts = workload
        .requests
        .iter()
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
    let mut session = [0; 32];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut session))
        .map_err(|e| e.to_string())?;
    let pool_constructor = if options.kernel_profile.wide32() {
        EngineeringTpPagedPoolV1::new_wide32
    } else {
        EngineeringTpPagedPoolV1::new
    };
    let pool = pool_constructor(
        EngineeringTpPoolScopeV1 {
            model: *model.bundle_id().as_bytes(),
            session,
        },
        EngineeringTpPagedLimitsV1::new(options.context, 32, options.pages, options.ttl)
            .map_err(|e| format!("page limits: {e:?}"))?,
    )
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
    let workers = if let Some(peer) = &peer_artifact {
        PeerWorker::spawn_with_options(
            &options.worker,
            &options.devices,
            &[&artifact, peer],
            options.runtime,
        )?
        .into_iter()
        .map(RankWorker::Peer)
        .collect::<Vec<_>>()
    } else {
        options
            .devices
            .iter()
            .map(|&id| {
                Worker::spawn_with_options(&options.worker, id, &artifact, options.runtime)
                    .map(RankWorker::Independent)
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
    let driver_constructor = if options.kernel_profile.wide32() {
        EngineeringTpBatchExecutionV2::new_wide32
    } else {
        EngineeringTpBatchExecutionV2::new
    };
    let mut gpu = driver_constructor(
        workers,
        model.config(),
        model.target_weights(),
        model.layout(),
        &pool,
    )?;
    gpu.configure_output_head_pruning(options.prune_output_head)?;
    gpu.configure_reduction(options.collective)?;
    gpu.configure_dispatch_sequences(options.runtime.sequences)?;
    gpu.configure_projection(options.projection, model.target_weights(), model.layout())?;
    gpu.configure_wave_attention(options.wave_attention)?;
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
    let result = (|| {
        let setup_seconds = whole.elapsed().as_secs_f64();
        if let Some(config) = control {
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
                "projection":options.projection.label(), "attention":if options.wave_attention { "wave" } else { "baseline" }, "runtime_profiling":false
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
        emit(&setup)?;
        run_workload(
            &mut runtime,
            &model,
            &workload,
            &prompts,
            options,
            benchmark_clock.as_ref(),
        )
    })();
    let close = runtime.close();
    match (result, close) {
        (Ok(()), Ok(())) => {
            let replica_closed = benchmark_clock.map(BenchmarkClock::finish).transpose()?;
            let mut closed = serde_json::json!({"schema":"FerricQwen3TpBatchClosedV2", "authority":"none",
            "worker_pids":pids, "all_workers_exited":true, "rank_dispatch_counts":runtime.dispatch_counts(),
            "whole_seconds":whole.elapsed().as_secs_f64()});
            if let Some(receipt) = replica_closed {
                closed["replica_benchmark"] =
                    serde_json::to_value(receipt).map_err(|e| e.to_string())?;
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
