//! Explicitly opted-in Qwen engineering execution; never a protected server.

mod target_decode_profile_contract;
mod tp_worker;

use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use ferric_m1_engineering_execution_v1::tp_artifact::EngineeringTpArtifactV1;
use ferric_m1_engineering_execution_v1::tp_execution::EngineeringTpExecutionV1;
use ferric_m1_engineering_execution_v1::tp_model::EngineeringQwenModelV1;
use serde::Serialize;
use sha2::{Digest, Sha256};
use target_decode_profile_contract::{Reference, validate_profile_options};
use tp_worker::{RuntimeOptions, Worker};

const USAGE: &str = "ferric-qwen3-tp-engineering --source DIR --artifact DIR --worker FILE --devices ID[,ID...] --allow-unauthenticated-machine-code [--prompt TEXT] [--new-tokens 32] [--repetitions 3] [--warmup 1] [--capacity 128] [--reference FILE --reference-sha256 SHA256] [--runtime-cache-admission] [--runtime-operational] [--runtime-profile]";

struct Options {
    source: PathBuf,
    artifact: PathBuf,
    worker: PathBuf,
    devices: Vec<u64>,
    prompt: String,
    new_tokens: u32,
    repetitions: u32,
    warmup: u32,
    capacity: u32,
    reference: Option<PathBuf>,
    reference_sha256: Option<String>,
    runtime: RuntimeOptions,
}

impl Options {
    fn parse(arguments: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut arguments = arguments.peekable();
        let mut seen = BTreeSet::new();
        let (mut source, mut artifact, mut worker, mut devices) = (None, None, None, None);
        let mut prompt = "The capital of France is".to_owned();
        let (mut new_tokens, mut repetitions, mut warmup, mut capacity) = (32, 3, 1, 128);
        let mut consent = false;
        let (mut reference, mut reference_sha256) = (None, None);
        let mut runtime = RuntimeOptions::default();
        while let Some(flag) = arguments.next() {
            if !seen.insert(flag.clone()) {
                return Err(format!("duplicate option {flag}"));
            }
            if flag == "--allow-unauthenticated-machine-code" {
                consent = true;
                continue;
            }
            if flag == "--help" {
                return Err(USAGE.into());
            }
            match flag.as_str() {
                "--runtime-cache-admission" => {
                    runtime.cache_admission = true;
                    continue;
                }
                "--runtime-operational" => {
                    runtime.operational = true;
                    continue;
                }
                "--runtime-profile" => {
                    runtime.profile = true;
                    continue;
                }
                _ => {}
            }
            let value = arguments
                .next()
                .ok_or_else(|| format!("missing value for {flag}"))?;
            match flag.as_str() {
                "--source" => source = Some(PathBuf::from(value)),
                "--artifact" => artifact = Some(PathBuf::from(value)),
                "--worker" => worker = Some(PathBuf::from(value)),
                "--devices" => {
                    devices = Some(
                        value
                            .split(',')
                            .map(str::parse::<u64>)
                            .collect::<Result<Vec<_>, _>>()
                            .map_err(|error| format!("device IDs: {error}"))?,
                    );
                }
                "--prompt" => prompt = value,
                "--new-tokens" => new_tokens = number(&flag, &value)?,
                "--repetitions" => repetitions = number(&flag, &value)?,
                "--warmup" => warmup = number(&flag, &value)?,
                "--capacity" => capacity = number(&flag, &value)?,
                "--reference" => reference = Some(PathBuf::from(value)),
                "--reference-sha256" => reference_sha256 = Some(value),
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
            return Err("devices must name 1, 2 or 8 distinct nonzero physical unique IDs".into());
        }
        let profile = validate_profile_options(
            reference.as_deref(),
            reference_sha256.as_deref(),
            devices.len(),
            &prompt,
            new_tokens,
            runtime.cache_admission || runtime.operational || runtime.profile,
        )?;
        if !(2..=256).contains(&new_tokens)
            || !(1..=if profile { 30 } else { 10 }).contains(&repetitions)
            || warmup > if profile { 10 } else { 3 }
            || !(1..=8192).contains(&capacity)
            || prompt.is_empty()
            || prompt.len() > 16_384
        {
            return Err("prompt or measurement bounds exceeded".into());
        }
        if profile {
            let steps = new_tokens + 4;
            if steps > capacity {
                return Err("target profile exceeds sequence capacity".into());
            }
            checked_dispatch_budget(steps, repetitions + warmup, 36)?;
        }
        Ok(Self {
            source: source.ok_or("--source is required")?,
            artifact: artifact.ok_or("--artifact is required")?,
            worker: worker.ok_or("--worker is required")?,
            devices,
            prompt,
            new_tokens,
            repetitions,
            warmup,
            capacity,
            reference,
            reference_sha256,
            runtime,
        })
    }
}

fn number(flag: &str, value: &str) -> Result<u32, String> {
    value.parse().map_err(|error| format!("{flag}: {error}"))
}

fn checked_dispatch_budget(steps: u32, runs: u32, layers: u32) -> Result<u64, String> {
    let budget = u64::from(steps)
        .checked_mul(u64::from(runs))
        .and_then(|tokens| tokens.checked_mul(u64::from(layers) * 15 + 4))
        .ok_or("measurement dispatch budget overflow")?;
    if steps == 0
        || runs == 0
        || layers == 0
        || budget > fe2o3_kfd::engineering_wire::MAX_UNRETIRED_RING_PACKETS_V1
    {
        return Err("measurement exceeds the conservative no-ring-rollover packet budget; reduce repetitions, warmup, or token count".into());
    }
    Ok(budget)
}

#[derive(Serialize)]
struct Measurement {
    schema: &'static str,
    authority: &'static str,
    run: u32,
    warmup: bool,
    world_size: usize,
    prompt_tokens: Vec<u32>,
    generated_tokens: Vec<u32>,
    generated_text: Option<String>,
    generated_utf8_bytes: Vec<u8>,
    ttft_seconds: f64,
    tpot_seconds: f64,
    decode_intervals_seconds: Vec<f64>,
    generation_seconds: f64,
    rank_dispatch_counts: Vec<u64>,
    kv_tokens_processed: u32,
}

fn emit(value: &impl Serialize) -> Result<(), String> {
    let mut output = std::io::stdout().lock();
    serde_json::to_writer(&mut output, value).map_err(|error| error.to_string())?;
    output.write_all(b"\n").map_err(|error| error.to_string())?;
    output.flush().map_err(|error| error.to_string())
}

fn run_sequence(
    engine: &mut EngineeringTpExecutionV1<Worker>,
    model: &EngineeringQwenModelV1,
    prompt: &[u32],
    options: &Options,
    reference: Option<&Reference>,
    run: u32,
    warmup: bool,
) -> Result<Measurement, String> {
    engine.reset_sequence()?;
    if engine.position() != 0 {
        return Err("fresh target sequence has a nonzero KV cursor".into());
    }
    let before = engine.dispatch_counts();
    let started = Instant::now();
    let mut next = None;
    for (position, &token) in prompt.iter().enumerate() {
        next = Some(engine.step(token)?);
        eprintln!(
            "run={run} warmup={warmup} prompt_position={position} elapsed_seconds={:.6}",
            started.elapsed().as_secs_f64()
        );
    }
    let first_at = Instant::now();
    let first = next.ok_or("empty tokenized prompt")?;
    if let Some(reference) = reference {
        reference.check_token(0, first)?;
    }
    let mut tokens = vec![first];
    let mut previous = first_at;
    let mut intervals = Vec::with_capacity(options.new_tokens as usize - 1);
    let mut token = first;
    for ordinal in 1..options.new_tokens {
        token = engine.step(token)?;
        let now = Instant::now();
        if let Some(reference) = reference {
            reference.check_token(ordinal as usize, token)?;
        }
        intervals.push(now.duration_since(previous).as_secs_f64());
        tokens.push(token);
        previous = now;
        eprintln!(
            "run={run} warmup={warmup} generated={ordinal} token={token} elapsed_seconds={:.6}",
            started.elapsed().as_secs_f64()
        );
    }
    let generation_seconds = previous.duration_since(started).as_secs_f64();
    let after = engine.dispatch_counts();
    let counts = after
        .iter()
        .zip(before)
        .map(|(after, before)| {
            after
                .checked_sub(before)
                .ok_or("dispatch counter regressed")
        })
        .collect::<Result<Vec<_>, _>>()?;
    let steps = u64::try_from(prompt.len()).map_err(|_| "prompt length overflow")?
        + u64::from(options.new_tokens)
        - 1;
    for (rank, count) in counts.iter().enumerate() {
        let per_token = u64::from(model.config().layers) * 15 + if rank == 0 { 4 } else { 0 };
        if *count != steps * per_token {
            return Err(format!(
                "rank {rank} did not execute the exact token/layer dispatch schedule"
            ));
        }
    }
    let decoded = model.decode(&tokens)?;
    if let Some(reference) = reference {
        reference.check_output(&tokens, &decoded)?;
        if u64::from(engine.position()) != steps {
            return Err("target profile final KV cursor differs from the exact schedule".into());
        }
    }
    let text = String::from_utf8(decoded.clone()).ok();
    let interval_count = u32::try_from(intervals.len()).map_err(|_| "interval count overflow")?;
    Ok(Measurement {
        schema: "FerricQwen3TpEngineeringMeasurementV1",
        authority: "none",
        run,
        warmup,
        world_size: options.devices.len(),
        prompt_tokens: prompt.to_vec(),
        generated_tokens: tokens,
        generated_text: text,
        generated_utf8_bytes: decoded,
        ttft_seconds: first_at.duration_since(started).as_secs_f64(),
        tpot_seconds: intervals.iter().sum::<f64>() / f64::from(interval_count),
        decode_intervals_seconds: intervals,
        generation_seconds,
        rank_dispatch_counts: counts,
        kv_tokens_processed: engine.position(),
    })
}

fn hash_file(path: &Path) -> Result<String, String> {
    let mut file =
        std::fs::File::open(path).map_err(|error| format!("open {}: {error}", path.display()))?;
    let metadata = file.metadata().map_err(|error| error.to_string())?;
    if !metadata.is_file() || metadata.len() > 512 * 1024 * 1024 {
        return Err("executable file bound".into());
    }
    let mut hash = Sha256::new();
    let mut buffer = vec![0; 65_536];
    loop {
        let read = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
    }
    Ok(hex(&hash.finalize()))
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

fn run(options: &Options) -> Result<(), String> {
    let whole = Instant::now();
    let reference = match (&options.reference, &options.reference_sha256) {
        (Some(path), Some(digest)) => Some(Reference::open(path, digest, options.new_tokens)?),
        (None, None) => None,
        _ => return Err("incomplete target reference options".into()),
    };
    let worker_hash = hash_file(&options.worker)?;
    let executable_hash = hash_file(Path::new("/proc/self/exe"))?;
    let roster = ferric_qwen3_tp_kernels_device_v1::compiler_expectation_roster_v1();
    let artifact = EngineeringTpArtifactV1::open(&options.artifact, &roster)
        .map_err(|error| error.to_string())?;
    eprintln!("artifact admitted; authenticating canonical Qwen source");
    let model = EngineeringQwenModelV1::open(&options.source)?;
    let prompt = model.encode(&options.prompt)?;
    if let Some(reference) = &reference {
        reference.bind(&hex(model.bundle_id().as_bytes()), &prompt)?;
    }
    let required = u32::try_from(prompt.len())
        .map_err(|_| "prompt token length")?
        .checked_add(options.new_tokens - 1)
        .ok_or("sequence capacity overflow")?;
    if prompt.is_empty() || required > options.capacity {
        return Err("tokenized prompt/output exceeds sequence capacity".into());
    }
    let dispatch_budget = checked_dispatch_budget(
        required,
        options.repetitions + options.warmup,
        model.config().layers,
    )?;
    let intake_seconds = whole.elapsed().as_secs_f64();
    eprintln!(
        "canonical model admitted in {intake_seconds:.6}s; starting {} independent GPU contexts",
        options.devices.len()
    );
    let workers = options
        .devices
        .iter()
        .map(|&device| {
            Worker::spawn_with_options(&options.worker, device, &artifact, options.runtime)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let pids = workers.iter().map(Worker::pid).collect::<Vec<_>>();
    let running_hashes = pids
        .iter()
        .map(|pid| hash_file(&PathBuf::from(format!("/proc/{pid}/exe"))))
        .collect::<Result<Vec<_>, _>>()?;
    if running_hashes.iter().any(|actual| actual != &worker_hash) {
        return Err("running worker executable differs from measured worker file".into());
    }
    let mut engine = EngineeringTpExecutionV1::new(
        workers,
        model.config(),
        model.target_weights(),
        model.layout(),
        options.capacity,
    )?;
    let setup_seconds = whole.elapsed().as_secs_f64();
    let measured = (|| {
        emit(&serde_json::json!({
            "schema": "FerricQwen3TpEngineeringSetupV1", "authority": "none",
            "model": "Qwen/Qwen3-8B", "dtype": "BF16", "target": "gfx950:xnack-",
            "tensor_parallel": options.devices.len(), "device_unique_ids": options.devices,
            "worker_pids": pids, "worker_sha256": worker_hash, "controller_sha256": executable_hash,
            "running_worker_sha256": running_hashes, "executable_identity": "live_proc_exe_sha256",
            "model_bundle_id": hex(model.bundle_id().as_bytes()),
            "artifact_hsaco_id": hex(artifact.hsaco_id().as_bytes()),
            "artifact_manifest_id": hex(artifact.manifest_id().as_bytes()),
            "artifact_handoff_id": hex(artifact.handoff_id().as_bytes()),
            "prompt": options.prompt, "prompt_tokens": prompt, "new_tokens": options.new_tokens,
            "capacity": options.capacity, "repetitions": options.repetitions, "warmup_runs": options.warmup,
            "rank_zero_dispatch_budget": dispatch_budget,
            "conservative_ring_packet_limit": fe2o3_kfd::engineering_wire::MAX_UNRETIRED_RING_PACKETS_V1,
            "model_intake_seconds": intake_seconds, "setup_seconds": setup_seconds,
            "collective": "host_staged_fp32_rank_order_reduce_bf16_residual",
            "prefill": "token_at_a_time_m1", "decoding": "greedy_lowest_id_fixed_length",
            "numerical_status": "Contracted; compare emitted token IDs independently",
            "timing": "monotonic controller clock; includes IPC, host collectives and per-token progress logging; excludes setup"
        }))?;
        if let Some(reference) = &reference {
            emit(&serde_json::json!({
                "schema": "FerricQwen3TargetDecodeProfileSetupV1", "authority": "none",
                "performance_qualified": false, "reference_sha256": target_decode_profile_contract::REFERENCE_SHA256,
                "reference_model_revision": target_decode_profile_contract::TARGET_REVISION,
                "reference_device": "AMD Instinct MI300X", "reference_passes": 2,
                "reference_tokens_available": 32, "reference_tokens_consumed": options.new_tokens,
                "reference_expected_tokens": reference.tokens,
                "reference_expected_utf8_bytes": reference.bytes,
                "precision": "BF16", "target_only": true, "speculation": false,
                "concurrent_requests": 1, "tensor_parallel": 1,
                "runtime_cache_admission": options.runtime.cache_admission,
                "runtime_operational": options.runtime.operational,
                "runtime_profile": options.runtime.profile,
                "runtime_sequences": false, "runtime_ordered_batches": false,
                "runtime_rollover": false, "kv_prefix_cache": false,
                "sequence_reuse": "reset logical cursor; each request overwrites every consumed KV position",
                "timing": "host Instant; includes IPC, host collectives, progress logging and reference checks; not GPU timestamps",
                "nonclaim": "Reference-checked bounded diagnostic; not model-wide numerical qualification, GPU overlap, megakernel execution, or a controlled benchmark"
            }))?;
        }
        for index in 0..options.warmup {
            let measurement = run_sequence(
                &mut engine,
                &model,
                &prompt,
                options,
                reference.as_ref(),
                index,
                true,
            )?;
            emit(&measurement)?;
            emit_profile_pass(reference.as_ref(), &measurement)?;
        }
        for index in 0..options.repetitions {
            let measurement = run_sequence(
                &mut engine,
                &model,
                &prompt,
                options,
                reference.as_ref(),
                index,
                false,
            )?;
            emit(&measurement)?;
            emit_profile_pass(reference.as_ref(), &measurement)?;
        }
        Ok::<(), String>(())
    })();
    let closed = engine.close();
    match (measured, closed) {
        (Ok(()), Ok(())) => emit(
            &serde_json::json!({ "schema": "FerricQwen3TpEngineeringClosedV1", "authority": "none", "worker_pids": pids, "all_workers_exited": true, "whole_seconds": whole.elapsed().as_secs_f64() }),
        ),
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Err(error)) => Err(format!("worker teardown failed: {error}")),
        (Err(error), Err(close)) => Err(format!("{error}; worker teardown failed: {close}")),
    }
}

fn emit_profile_pass(
    reference: Option<&Reference>,
    measurement: &Measurement,
) -> Result<(), String> {
    if reference.is_some() {
        emit(&serde_json::json!({
            "schema": "FerricQwen3TargetDecodeProfileRunV1", "authority": "none",
            "run": measurement.run, "warmup": measurement.warmup,
            "reference_passed": true, "performance_qualified": false,
            "generated_tokens_checked": measurement.generated_tokens.len(),
            "generated_utf8_bytes_checked": measurement.generated_utf8_bytes.len(),
            "kv_tokens_processed": measurement.kv_tokens_processed
        }))?;
    }
    Ok(())
}

fn main() -> std::process::ExitCode {
    match Options::parse(std::env::args().skip(1)).and_then(|options| run(&options)) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Qwen TP engineering failed: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(extra: &[&str]) -> Result<Options, String> {
        let mut args = vec![
            "--source",
            "/models",
            "--artifact",
            "/artifact",
            "--worker",
            "/worker",
            "--allow-unauthenticated-machine-code",
        ];
        args.extend(extra);
        Options::parse(args.into_iter().map(str::to_owned))
    }

    #[test]
    fn device_identity_and_measurement_bounds_are_closed() {
        assert_eq!(
            options(&["--devices", "1,2,3,4,5,6,7,8"])
                .unwrap()
                .devices
                .len(),
            8
        );
        for devices in ["", "0", "1,1", "1,2,3", "1,2,3,4,5,6,7,8,9", "-1"] {
            assert!(options(&["--devices", devices]).is_err());
        }
        for (flag, value) in [
            ("--new-tokens", "1"),
            ("--new-tokens", "257"),
            ("--repetitions", "0"),
            ("--warmup", "4"),
            ("--capacity", "8193"),
            ("--unknown", "1"),
        ] {
            assert!(options(&["--devices", "1", flag, value]).is_err());
        }
        assert!(options(&["--devices", "1", "--devices", "2"]).is_err());
        assert!(Options::parse(["--devices".into(), "1".into()].into_iter()).is_err());
    }

    #[test]
    fn digest_encoding_is_fixed_lowercase() {
        assert_eq!(hex(&[0, 15, 16, 255]), "000f10ff");
    }

    #[test]
    fn packet_budget_covers_default_runs_but_rejects_ring_exhaustion() {
        assert_eq!(checked_dispatch_budget(36, 4, 36), Ok(78_336));
        assert_eq!(checked_dispatch_budget(240, 1, 36), Ok(130_560));
        assert!(checked_dispatch_budget(241, 1, 36).is_err());
        assert!(checked_dispatch_budget(0, 1, 36).is_err());
        assert!(checked_dispatch_budget(u32::MAX, u32::MAX, u32::MAX).is_err());
    }

    #[test]
    fn profile_flags_preserve_legacy_bounds_and_close_runtime_ablations() {
        let digest = target_decode_profile_contract::REFERENCE_SHA256;
        let parsed = options(&[
            "--devices",
            "1",
            "--new-tokens",
            "2",
            "--repetitions",
            "30",
            "--warmup",
            "10",
            "--reference",
            "/reference",
            "--reference-sha256",
            digest,
            "--runtime-cache-admission",
            "--runtime-operational",
            "--runtime-profile",
        ])
        .unwrap();
        assert!(
            parsed.runtime.cache_admission && parsed.runtime.operational && parsed.runtime.profile
        );
        assert!(
            !parsed.runtime.sequences
                && !parsed.runtime.ordered_batches
                && !parsed.runtime.rollover
        );
        assert_eq!(
            checked_dispatch_budget(6, parsed.repetitions + parsed.warmup, 36),
            Ok(130_560)
        );
        assert!(checked_dispatch_budget(36, 7, 36).is_err());
        assert!(
            options(&[
                "--devices",
                "1",
                "--reference",
                "/reference",
                "--reference-sha256",
                digest,
                "--repetitions",
                "6",
                "--warmup",
                "1"
            ])
            .is_err()
        );
        assert!(
            options(&[
                "--devices",
                "1",
                "--reference",
                "/reference",
                "--reference-sha256",
                digest,
                "--capacity",
                "35"
            ])
            .is_err()
        );
        for flag in [
            "--runtime-cache-admission",
            "--runtime-operational",
            "--runtime-profile",
        ] {
            assert!(options(&["--devices", "1", flag]).is_err());
            assert!(
                options(&[
                    "--devices",
                    "1",
                    "--reference",
                    "/reference",
                    "--reference-sha256",
                    digest,
                    flag,
                    flag
                ])
                .is_err()
            );
        }
        for (flag, value) in [
            ("--repetitions", "31"),
            ("--warmup", "11"),
            ("--new-tokens", "4"),
        ] {
            assert!(
                options(&[
                    "--devices",
                    "1",
                    "--reference",
                    "/reference",
                    "--reference-sha256",
                    digest,
                    flag,
                    value
                ])
                .is_err()
            );
        }
        assert!(options(&["--devices", "1", "--repetitions", "11"]).is_err());
        assert!(options(&["--devices", "1", "--runtime-sequences"]).is_err());
        assert!(options(&["--devices", "1", "--runtime-ordered-batches"]).is_err());
        assert!(options(&["--devices", "1", "--runtime-rollover"]).is_err());
        assert!(options(&["--devices", "1", "--dtype", "FP8"]).is_err());
        assert!(options(&["--devices", "1", "--speculation"]).is_err());
    }
}
