use std::fs;
use std::os::unix::fs::DirBuilderExt;

use fe2o3_kfd::engineering_wire::MAX_OBJECT_BYTES_V1;
use serde::Serialize;

use crate::{
    artifact, atomic_artifact as abi, atomic_probe, create_private, json, private_device_id, probe,
    read_bounded, session, write_private, Options, Result,
};

#[derive(Serialize)]
struct Report<'a> {
    schema: &'static str,
    authority: &'static str,
    scope: &'static str,
    artifact: &'a abi::Record,
    probe_sha256: String,
    worker_sha256: String,
    input_sha256: String,
    channels_sha256: String,
    output_sha256: String,
    workgroup: [u16; 3],
    grid_work_items: [u32; 3],
    completed_dispatches: u32,
    worker_elapsed_ns: u64,
    timing_boundary: &'static str,
    input_immutability_and_all_guards_passed: bool,
    free_close_and_worker_exit_passed: bool,
    numerical_validation: &'static str,
}

pub fn execute(options: &Options) -> Result<()> {
    let object = read_bounded(options.path("--object"), u64::from(MAX_OBJECT_BYTES_V1))?;
    let source = read_bounded(options.path("--source-file"), 1024 * 1024)?;
    let record = abi::inspect(&object, &source)?;
    if options.mode == "atomic-channel-inspect" {
        return write_private(options.path("--metadata"), &json(&record)?);
    }
    let expected: abi::Record =
        serde_json::from_slice(&read_bounded(options.path("--metadata"), 65_536)?)
            .map_err(|_| "invalid expected atomic channel artifact record")?;
    if expected != record {
        return Err("source, object or metadata differs from the expected artifact record".into());
    }
    let inputs = read_bounded(options.path("--inputs"), abi::BYTES as u64)?;
    abi::validate_inputs(&inputs)?;
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
    let observation = atomic_probe::run(&mut session, object, &record.metadata, &inputs)?;
    session.finish()?;
    let report = Report {
        schema: "ferric-atomic-channel-probe-v1",
        authority: "none",
        scope: "unqualified indexed atomic storage fixture; not cross-workgroup publication or model inference",
        artifact: &record,
        probe_sha256,
        worker_sha256,
        input_sha256: artifact::hex(&artifact::digest(&inputs)),
        channels_sha256: artifact::hex(&artifact::digest(&observation.channels)),
        output_sha256: artifact::hex(&artifact::digest(&observation.output)),
        workgroup: abi::WORKGROUP,
        grid_work_items: abi::GRID,
        completed_dispatches: 1,
        worker_elapsed_ns: observation.elapsed_ns,
        timing_boundary: "worker host interval including queue publication and completion polling; not GPU event time",
        input_immutability_and_all_guards_passed: true,
        free_close_and_worker_exit_passed: true,
        numerical_validation: "not evaluated here; compare both buffers against the independent exact reference",
    };
    write_private(&directory.join("channels.u32le"), &observation.channels)?;
    write_private(&directory.join("output.u32le"), &observation.output)?;
    write_private(&directory.join("report.json"), &json(&report)?)
}
