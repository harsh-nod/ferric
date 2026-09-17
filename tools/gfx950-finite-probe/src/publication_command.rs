use crate::{
    artifact, create_private, json, private_device_id, probe, publication_artifact as abi,
    publication_probe, read_bounded, session, write_private, Options, Result,
};
use fe2o3_kfd::engineering_wire::MAX_OBJECT_BYTES_V1;
use serde::Serialize;
use std::fs;
use std::os::unix::fs::DirBuilderExt;

#[derive(Serialize)]
struct Report<'a> {
    schema: &'static str,
    authority: &'static str,
    scope: &'static str,
    artifact: &'a abi::Record,
    case: &'a abi::Case,
    case_sha256: String,
    probe_sha256: String,
    worker_sha256: String,
    input_sha256: String,
    initial_flags_sha256: String,
    buffer_sha256: [String; 5],
    completed_dispatches: u32,
    fresh_worker_and_allocations: bool,
    input_immutability_and_all_guards_passed: bool,
    free_close_and_worker_exit_passed: bool,
    worker_elapsed_ns: u64,
    timing_boundary: &'static str,
    memory_eligibility: &'static str,
    numerical_validation: &'static str,
}

pub fn execute(options: &Options) -> Result<()> {
    let object = read_bounded(options.path("--object"), u64::from(MAX_OBJECT_BYTES_V1))?;
    let source = read_bounded(options.path("--source-file"), 1024 * 1024)?;
    let record = abi::inspect(&object, &source)?;
    if options.mode == "static-publication-inspect" {
        return write_private(options.path("--metadata"), &json(&record)?);
    }
    let expected: abi::Record =
        serde_json::from_slice(&read_bounded(options.path("--metadata"), 65_536)?)
            .map_err(|_| "invalid expected publication artifact")?;
    if expected != record {
        return Err("publication source, object or metadata substitution".into());
    }
    let case_bytes = read_bounded(options.path("--case-file"), 4096)?;
    let case: abi::Case =
        serde_json::from_slice(&case_bytes).map_err(|_| "invalid publication case")?;
    let inputs = read_bounded(options.path("--inputs"), abi::BYTES[2] as u64)?;
    let flags = read_bounded(options.path("--initial-flags"), abi::BYTES[1] as u64)?;
    abi::validate_inputs(&case, &inputs, &flags)?;
    let unique_id = private_device_id(options.path("--device-id-file"))?;
    let probe_sha256 = artifact::hex(&artifact::digest(&read_bounded(
        std::path::Path::new("/proc/self/exe"),
        256 * 1024 * 1024,
    )?));
    let worker_path =
        fs::canonicalize(options.path("--worker")).map_err(|_| "worker path cannot be resolved")?;
    let worker_sha256 = artifact::hex(&artifact::digest(&read_bounded(
        &worker_path,
        256 * 1024 * 1024,
    )?));
    let directory = options.path("--run-dir");
    fs::DirBuilder::new()
        .mode(0o700)
        .create(directory)
        .map_err(|_| "run directory must be new")?;
    let stderr = create_private(&directory.join("worker-stderr.log"))?;
    let (mut session, packet) = session::Session::spawn(&worker_path, unique_id, stderr)?;
    probe::ready(&packet, unique_id)?;
    if artifact::hex(&session.executable_digest()?) != worker_sha256 {
        return Err("running worker digest mismatch".into());
    }
    let observation = publication_probe::run(
        &mut session,
        object,
        &record.metadata,
        &case,
        &inputs,
        &flags,
    )?;
    session.finish()?;
    let report = Report {
        schema: "ferric-static-publication-probe-v1", authority: "none",
        scope: "fixed ordinary cross-workgroup consumer-clear observation; no progress or protected runtime authority",
        artifact: &record, case: &case, case_sha256: artifact::hex(&artifact::digest(&case_bytes)),
        probe_sha256, worker_sha256, input_sha256: artifact::hex(&artifact::digest(&inputs)),
        initial_flags_sha256: artifact::hex(&artifact::digest(&flags)),
        buffer_sha256: observation.data.each_ref().map(|data| artifact::hex(&artifact::digest(data))),
        completed_dispatches: 1, fresh_worker_and_allocations: true,
        input_immutability_and_all_guards_passed: true, free_close_and_worker_exit_passed: true,
        worker_elapsed_ns: observation.elapsed_ns,
        timing_boundary: "worker dispatch interval including kernarg copy, signal reset, currentness and observed completion/check_idle; not GPU event time",
        memory_eligibility: "unqualified DEVICE_LOCAL_PUBLIC (VRAM|WRITABLE|PUBLIC); not COHERENT or authenticated System atomic eligibility; operator-reviewed engineering assumption only",
        numerical_validation: "independent reference required; zero Ready cannot pass coverage",
    };
    for (name, bytes) in [
        "payload.f32le",
        "flags.u32le",
        "input.f32le",
        "statuses.u32le",
        "values.f32le",
    ]
    .iter()
    .zip(&observation.data)
    {
        write_private(&directory.join(name), bytes)?;
    }
    write_private(&directory.join("report.json"), &json(&report)?)
}
