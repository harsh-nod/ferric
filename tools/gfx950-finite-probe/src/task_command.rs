use std::fs;
use std::os::unix::fs::DirBuilderExt;
use std::time::Instant;

use fe2o3_kfd::engineering_wire::MAX_OBJECT_BYTES_V1;
use serde::Serialize;

use crate::artifact::{digest, hex};
use crate::task_artifact::{self as abi, Record};
use crate::task_probe::{self, EpochObservation};
use crate::{
    create_private, json, private_device_id, read_bounded, write_private, Options, Result,
};

#[derive(Serialize)]
struct Report<'a> {
    schema: &'static str,
    authority: &'static str,
    scope: &'static str,
    artifact: &'a Record,
    probe_sha256: String,
    worker_sha256: String,
    input_sha256: String,
    states_sha256: String,
    completed_dispatches: usize,
    reused_worker_queue_and_allocations: bool,
    host_lifecycle_ns: u64,
    timing_semantics: &'static str,
    input_immutability_and_all_guards_passed: bool,
    free_close_and_worker_exit_passed: bool,
    independent_payload_validation: &'static str,
    epochs: &'a [EpochObservation],
}

pub fn execute(options: &Options) -> Result<()> {
    let started = Instant::now();
    let object = read_bounded(options.path("--object"), u64::from(MAX_OBJECT_BYTES_V1))?;
    let source = read_bounded(options.path("--source-file"), 1024 * 1024)?;
    let record = abi::inspect(&object, &source)?;
    if options.mode == "task-graph-inspect" {
        return write_private(options.path("--metadata"), &json(&record)?);
    }
    let expected: Record =
        serde_json::from_slice(&read_bounded(options.path("--metadata"), 65_536)?)
            .map_err(|_| "invalid task graph artifact record")?;
    if expected != record {
        return Err("task graph source, object or metadata identity mismatch".into());
    }
    let inputs = read_bounded(options.path("--inputs"), (abi::INPUT_WORDS * 4) as u64)?;
    abi::validate_inputs(&inputs)?;
    let unique_id = private_device_id(options.path("--device-id-file"))?;
    let worker = fs::canonicalize(options.path("--worker"))
        .map_err(|_| "worker executable path cannot be resolved")?;
    let worker_sha256 = hex(&digest(&read_bounded(&worker, 256 * 1024 * 1024)?));
    let executable = std::env::current_exe().map_err(|_| "probe executable path unavailable")?;
    let probe_sha256 = hex(&digest(&read_bounded(&executable, 256 * 1024 * 1024)?));
    let directory = options.path("--run-dir");
    fs::DirBuilder::new()
        .mode(0o700)
        .create(directory)
        .map_err(|_| "task graph run directory must be new and creatable")?;
    let stderr = create_private(&directory.join("worker-stderr.log"))?;
    let (mut session, packet) = crate::session::Session::spawn(&worker, unique_id, stderr)?;
    crate::probe::ready(&packet, unique_id)?;
    if hex(&session.executable_digest()?) != worker_sha256 {
        return Err("running task worker executable identity mismatch".into());
    }
    let epochs = task_probe::run(&mut session, object, &record.metadata, &inputs)?;
    session.finish()?;
    let host_lifecycle_ns = task_probe::elapsed(started)?;
    let states: Vec<u8> = epochs
        .iter()
        .flat_map(|epoch| epoch.state)
        .flat_map(u32::to_le_bytes)
        .collect();
    let report = Report {
        schema: "ferric-task-graph-probe-v1", authority: "none",
        scope: "bounded engineering scheduler micrograph; not model inference or production authority",
        artifact: &record, probe_sha256, worker_sha256, input_sha256: hex(&digest(&inputs)),
        states_sha256: hex(&digest(&states)), completed_dispatches: epochs.len(),
        reused_worker_queue_and_allocations: true, host_lifecycle_ns,
        timing_semantics: "host clocks only: lifecycle includes artifact checks through child exit, excluding report writes; epoch includes reset, dispatch roundtrip and all readbacks; worker interval includes kernarg copy, signal reset, publication, completion polling and idle checks, not GPU-only time",
        input_immutability_and_all_guards_passed: true, free_close_and_worker_exit_passed: true,
        independent_payload_validation: "not evaluated here; qualification reference must compare all seven payloads per epoch",
        epochs: &epochs,
    };
    write_private(&directory.join("states.u32le"), &states)?;
    write_private(&directory.join("report.json"), &json(&report)?)
}
