use std::fs;
use std::os::unix::fs::DirBuilderExt;

use fe2o3_kfd::engineering_wire::{KernelMetadataV1, MAX_OBJECT_BYTES_V1};
use serde::{Deserialize, Serialize};

use crate::{
    artifact, create_private, json, kproj_artifact as abi, private_device_id, probe, read_bounded,
    session, write_private, Options, Result,
};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ArtifactRecord {
    schema: String,
    source_sha256: String,
    object_sha256: String,
    metadata: KernelMetadataV1,
    observed_launch_metadata: artifact::ObservedLaunchMetadata,
    observed_argument_qualifiers: Vec<artifact::ObservedArgumentQualifiers>,
    source_declared_abi: abi::DeclaredAbi,
}

#[derive(Serialize)]
struct Report<'a> {
    schema: &'static str,
    authority: &'static str,
    scope: &'static str,
    artifact: &'a ArtifactRecord,
    probe_sha256: String,
    worker_sha256: String,
    input_sha256: String,
    weights_sha256: String,
    output_sha256: String,
    workgroup: [u16; 3],
    grid_work_items: [u32; 3],
    completed_dispatches: u32,
    worker_elapsed_ns: u64,
    timing_boundary: &'static str,
    input_immutability_and_all_allocation_guards_passed: bool,
    free_close_and_worker_exit_passed: bool,
    numerical_validation: &'static str,
}

pub fn execute(options: &Options) -> Result<()> {
    let object = read_bounded(options.path("--object"), u64::from(MAX_OBJECT_BYTES_V1))?;
    let source = read_bounded(options.path("--source-file"), 1024 * 1024)?;
    let variant = if options.mode.starts_with("qwen3-kproj-wave64-") {
        abi::Variant::Wave64
    } else {
        abi::Variant::Scalar
    };
    let inspected = abi::inspect(&object, variant)?;
    let record = ArtifactRecord {
        schema: "ferric-qwen3-kproj-artifact-v1".into(),
        source_sha256: artifact::hex(&artifact::digest(&source)),
        object_sha256: artifact::hex(&artifact::digest(&object)),
        metadata: inspected.metadata,
        observed_launch_metadata: inspected.launch,
        observed_argument_qualifiers: inspected.qualifiers,
        source_declared_abi: abi::declared_abi(variant),
    };
    if options.mode.ends_with("-inspect") {
        return write_private(options.path("--metadata"), &json(&record)?);
    }
    let expected: ArtifactRecord =
        serde_json::from_slice(&read_bounded(options.path("--metadata"), 65_536)?)
            .map_err(|_| "invalid expected artifact metadata record")?;
    if expected != record {
        return Err("source, object or metadata differs from the expected artifact record".into());
    }
    let inputs = read_bounded(options.path("--inputs"), abi::BYTES[0] as u64)?;
    let weights = read_bounded(options.path("--weights"), abi::BYTES[1] as u64)?;
    if inputs.len() != abi::BYTES[0] || weights.len() != abi::BYTES[1] {
        return Err("Qwen3 projection input or weights size mismatch".into());
    }
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
        return Err("running worker executable differs from its expected digest".into());
    }
    let observation = probe::run_kproj(&mut session, object, &record.metadata, &inputs, &weights)?;
    session.finish()?;
    let report = Report {
        schema: "ferric-qwen3-kproj-probe-v1",
        authority: "none",
        scope: "unqualified real-weight single projection; not full-model inference or a performance claim",
        artifact: &record,
        probe_sha256,
        worker_sha256,
        input_sha256: artifact::hex(&artifact::digest(&inputs)),
        weights_sha256: artifact::hex(&artifact::digest(&weights)),
        output_sha256: artifact::hex(&artifact::digest(&observation.output)),
        workgroup: abi::WORKGROUP,
        grid_work_items: variant.grid(),
        completed_dispatches: 1,
        worker_elapsed_ns: observation.elapsed_ns,
        timing_boundary: "worker host interval: kernarg copy, signal reset, queue publication, completion polling and idle check; not GPU event time",
        input_immutability_and_all_allocation_guards_passed: true,
        free_close_and_worker_exit_passed: true,
        numerical_validation: "not evaluated here; compare output against independent FP64 reference and predetermined bounds",
    };
    write_private(&directory.join("output.f32le"), &observation.output)?;
    write_private(&directory.join("report.json"), &json(&report)?)
}
