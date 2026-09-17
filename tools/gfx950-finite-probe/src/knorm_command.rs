use std::{fs, os::unix::fs::DirBuilderExt};

use fe2o3_kfd::engineering_wire::MAX_OBJECT_BYTES_V1;
use serde::Serialize;

use crate::{
    artifact, create_private, json, knorm_artifact as abi, knorm_probe, private_device_id, probe,
    read_bounded, session, write_private, Options, Result,
};

#[derive(Serialize)]
// Independent evidence observations, not interchangeable runtime state flags.
#[allow(clippy::struct_excessive_bools)]
struct Report<'a> {
    schema: &'static str,
    authority: &'static str,
    scope: &'static str,
    artifact: &'a abi::Record,
    probe_sha256: String,
    worker_sha256: String,
    input_sha256: String,
    weights_sha256: String,
    norm_weights_sha256: String,
    projection_sha256: String,
    quantized_sha256: String,
    output_sha256: String,
    completed_dispatches: u32,
    worker_elapsed_ns: [u64; 2],
    timing_boundary: &'static str,
    input_immutability_and_all_allocation_guards_passed: bool,
    projection_unchanged_after_consumer: bool,
    producer_completion_before_consumer: bool,
    intermediate_allocation_reused_without_host_write: bool,
    free_close_and_worker_exit_passed: bool,
    numerical_validation: &'static str,
}

pub fn execute(options: &Options) -> Result<()> {
    let producer = read_bounded(
        options.path("--producer-object"),
        u64::from(MAX_OBJECT_BYTES_V1),
    )?;
    let consumer = read_bounded(
        options.path("--consumer-object"),
        u64::from(MAX_OBJECT_BYTES_V1),
    )?;
    let producer_source = read_bounded(options.path("--producer-source"), 1024 * 1024)?;
    let consumer_source = read_bounded(options.path("--consumer-source"), 1024 * 1024)?;
    let record = abi::inspect(&producer, &producer_source, &consumer, &consumer_source)?;
    if options.mode == "qwen3-knorm-chain-inspect" {
        return write_private(options.path("--metadata"), &json(&record)?);
    }
    let expected: abi::Record =
        serde_json::from_slice(&read_bounded(options.path("--metadata"), 131_072)?)
            .map_err(|_| "invalid expected Qwen3 chain artifact record")?;
    if record != expected {
        return Err("Qwen3 chain source/object/graph record substitution".into());
    }
    let inputs = read_bounded(options.path("--inputs"), abi::BYTES[0] as u64)?;
    let weights = read_bounded(options.path("--weights"), abi::BYTES[1] as u64)?;
    let norm_weights = read_bounded(options.path("--norm-weights"), abi::BYTES[3] as u64)?;
    let unique_id = private_device_id(options.path("--device-id-file"))?;
    let probe_sha256 = artifact::hex(&artifact::digest(&read_bounded(
        std::path::Path::new("/proc/self/exe"),
        256 * 1024 * 1024,
    )?));
    let worker_path = fs::canonicalize(options.path("--worker"))
        .map_err(|_| "worker executable path cannot be resolved")?;
    let worker_sha256 = artifact::hex(&artifact::digest(&read_bounded(
        &worker_path,
        256 * 1024 * 1024,
    )?));
    let directory = options.path("--run-dir");
    fs::DirBuilder::new()
        .mode(0o700)
        .create(directory)
        .map_err(|_| "run directory must be new and creatable")?;
    let stderr = create_private(&directory.join("worker-stderr.log"))?;
    let (mut session, packet) = session::Session::spawn(&worker_path, unique_id, stderr)?;
    probe::ready(&packet, unique_id)?;
    if artifact::hex(&session.executable_digest()?) != worker_sha256 {
        return Err("running Qwen3 chain worker differs from expected digest".into());
    }
    let observation = knorm_probe::run(
        &mut session,
        knorm_probe::Kernel {
            object: producer,
            metadata: &record.producer.metadata,
        },
        knorm_probe::Kernel {
            object: consumer,
            metadata: &record.consumer.metadata,
        },
        &knorm_probe::Inputs {
            hidden: &inputs,
            weights: &weights,
            norm_weights: &norm_weights,
        },
    )?;
    session.finish()?;
    let report = Report {
        schema: "ferric-qwen3-knorm-chain-probe-v1", authority: "none",
        scope: "real-weight completion-ordered projection/norm baseline; not a megakernel or full-model inference",
        artifact: &record, probe_sha256, worker_sha256,
        input_sha256: artifact::hex(&artifact::digest(&inputs)),
        weights_sha256: artifact::hex(&artifact::digest(&weights)),
        norm_weights_sha256: artifact::hex(&artifact::digest(&norm_weights)),
        projection_sha256: artifact::hex(&artifact::digest(&observation.projection)),
        quantized_sha256: artifact::hex(&artifact::digest(&observation.quantized)),
        output_sha256: artifact::hex(&artifact::digest(&observation.output)),
        completed_dispatches: 2, worker_elapsed_ns: observation.elapsed_ns,
        timing_boundary: "two worker host intervals; intermediate CPU readback is outside them; not GPU event or token timing",
        input_immutability_and_all_allocation_guards_passed: true,
        projection_unchanged_after_consumer: true, producer_completion_before_consumer: true,
        intermediate_allocation_reused_without_host_write: true,
        free_close_and_worker_exit_passed: true,
        numerical_validation: "not evaluated here; independently validate projection, BF16 keys and normalization",
    };
    write_private(&directory.join("projection.f32le"), &observation.projection)?;
    write_private(&directory.join("quantized.bf16le"), &observation.quantized)?;
    write_private(&directory.join("output.bf16le"), &observation.output)?;
    write_private(&directory.join("report.json"), &json(&report)?)
}
