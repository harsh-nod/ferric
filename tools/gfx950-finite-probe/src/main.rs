mod artifact;
mod probe;
mod session;

#[cfg(test)]
mod process_tests;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use fe2o3_kfd::engineering_wire::{KernelMetadataV1, MAX_OBJECT_BYTES_V1};
use serde::{Deserialize, Serialize};

type Result<T> = std::result::Result<T, String>;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ArtifactRecord {
    schema: String,
    source_sha256: String,
    object_sha256: String,
    metadata: KernelMetadataV1,
    observed_launch_metadata: artifact::ObservedLaunchMetadata,
    observed_argument_qualifiers: Vec<artifact::ObservedArgumentQualifiers>,
    source_declared_abi: artifact::DeclaredAbi,
}

#[derive(Serialize)]
struct Report<'a> {
    schema: &'static str,
    authority: &'static str,
    scope: &'static str,
    artifact: &'a ArtifactRecord,
    worker_sha256: String,
    input_sha256: String,
    weights_sha256: String,
    output_sha256: String,
    workgroup: [u16; 3],
    grid_work_items: [u32; 3],
    completed_dispatches: u32,
    worker_elapsed_ns: u64,
    input_immutability_and_output_guards_passed: bool,
    free_close_and_worker_exit_passed: bool,
    numerical_validation: &'static str,
}

fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let file = File::open(path).map_err(|_| "input file could not be opened")?;
    if !file
        .metadata()
        .map_err(|_| "input metadata unavailable")?
        .is_file()
    {
        return Err("input must be a regular file".into());
    }
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "input file read failed")?;
    if bytes.len() as u64 > limit {
        return Err("input file exceeds its bounded size".into());
    }
    Ok(bytes)
}

fn create_private(path: &Path) -> Result<File> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|_| "output file already exists or cannot be created".into())
}

fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = create_private(path)?;
    file.write_all(bytes)
        .map_err(|_| "output file write failed")?;
    file.sync_all()
        .map_err(|_| "output file synchronization failed".into())
}

fn json(value: &impl Serialize) -> Result<Vec<u8>> {
    serde_json::to_vec_pretty(value).map_err(|_| "report serialization failed".into())
}

fn private_device_id(path: &Path) -> Result<u64> {
    let metadata =
        fs::symlink_metadata(path).map_err(|_| "private selector metadata unavailable")?;
    if !metadata.is_file() || metadata.permissions().mode() & 0o077 != 0 {
        return Err("device selector must be a private regular file (mode0600 or stricter)".into());
    }
    let bytes = read_bounded(path, 32)?;
    let value = std::str::from_utf8(&bytes)
        .map_err(|_| "invalid private selector encoding")?
        .trim()
        .parse::<u64>()
        .map_err(|_| "invalid private selector value")?;
    if value == 0 {
        return Err("private device selector cannot be zero".into());
    }
    Ok(value)
}

struct Options {
    mode: String,
    paths: BTreeMap<String, PathBuf>,
}

impl Options {
    fn parse(arguments: impl Iterator<Item = String>) -> Result<Self> {
        let mut arguments = arguments;
        let mode = arguments
            .next()
            .ok_or("expected inspect or run subcommand")?;
        let required = match mode.as_str() {
            "inspect" => vec!["--object", "--source-file", "--metadata"],
            "run" => vec![
                "--object",
                "--source-file",
                "--metadata",
                "--worker",
                "--inputs",
                "--weights",
                "--device-id-file",
                "--run-dir",
            ],
            _ => return Err("expected inspect or run subcommand".into()),
        };
        let mut paths = BTreeMap::new();
        let mut acknowledged = false;
        while let Some(flag) = arguments.next() {
            if flag == "--allow-unauthenticated-machine-code" && mode == "run" && !acknowledged {
                acknowledged = true;
                continue;
            }
            if !required.contains(&flag.as_str()) || paths.contains_key(&flag) {
                return Err("unknown or duplicate command option".into());
            }
            let value = arguments
                .next()
                .ok_or("command option requires a file path")?;
            if value.starts_with("--") {
                return Err("command option requires a file path".into());
            }
            paths.insert(flag, PathBuf::from(value));
        }
        if required.iter().any(|flag| !paths.contains_key(*flag)) {
            return Err("missing required command option; see README.md".into());
        }
        if mode == "run" && !acknowledged {
            return Err(
                "run requires explicit --allow-unauthenticated-machine-code acknowledgement".into(),
            );
        }
        Ok(Self { mode, paths })
    }

    fn path(&self, flag: &str) -> &Path {
        &self.paths[flag]
    }
}

fn execute(options: &Options) -> Result<()> {
    let object = read_bounded(options.path("--object"), u64::from(MAX_OBJECT_BYTES_V1))?;
    let source = read_bounded(options.path("--source-file"), 1024 * 1024)?;
    let inspected = artifact::inspect(&object)?;
    let record = ArtifactRecord {
        schema: "ferric-finite-decoder-artifact-v1".into(),
        source_sha256: artifact::hex(&artifact::digest(&source)),
        object_sha256: artifact::hex(&artifact::digest(&object)),
        metadata: inspected.metadata,
        observed_launch_metadata: inspected.launch,
        observed_argument_qualifiers: inspected.qualifiers,
        source_declared_abi: artifact::declared_abi(),
    };
    if options.mode == "inspect" {
        return write_private(options.path("--metadata"), &json(&record)?);
    }
    let expected: ArtifactRecord =
        serde_json::from_slice(&read_bounded(options.path("--metadata"), 65_536)?)
            .map_err(|_| "invalid expected artifact metadata record")?;
    if expected != record {
        return Err("source, object or metadata differs from the expected artifact record".into());
    }
    let inputs = read_bounded(options.path("--inputs"), artifact::BYTES[0] as u64)?;
    let weights = read_bounded(options.path("--weights"), artifact::BYTES[1] as u64)?;
    if inputs.len() != artifact::BYTES[0] || weights.len() != artifact::BYTES[1] {
        return Err("finite fixture input or weights size mismatch".into());
    }
    let unique_id = private_device_id(options.path("--device-id-file"))?;
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
    let observation = probe::run(&mut session, object, &record.metadata, &inputs, &weights)?;
    session.finish()?;
    let report = Report {
        schema: "ferric-finite-decoder-probe-v1",
        authority: "none",
        scope:
            "unqualified finite decoder fixture, not Ferric production or a persistent scheduler",
        artifact: &record,
        worker_sha256,
        input_sha256: artifact::hex(&artifact::digest(&inputs)),
        weights_sha256: artifact::hex(&artifact::digest(&weights)),
        output_sha256: artifact::hex(&artifact::digest(&observation.output)),
        workgroup: artifact::WORKGROUP,
        grid_work_items: artifact::GRID,
        completed_dispatches: 1,
        worker_elapsed_ns: observation.elapsed_ns,
        input_immutability_and_output_guards_passed: true,
        free_close_and_worker_exit_passed: true,
        numerical_validation: "not evaluated here; compare output with an independent reference",
    };
    write_private(&directory.join("output.f32le"), &observation.output)?;
    write_private(&directory.join("report.json"), &json(&report)?)
}

fn main() {
    let result = Options::parse(std::env::args().skip(1)).and_then(|options| execute(&options));
    match result {
        Ok(()) => {
            println!("finite probe completed; artifact inspection or engineering report written");
        }
        Err(error) => {
            eprintln!("finite probe failed: {error}");
            std::process::exit(1);
        }
    }
}
