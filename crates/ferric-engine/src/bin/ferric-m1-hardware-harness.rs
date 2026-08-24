//! Structured one-case MI300X hardware execution for M1 evidence production.

#![forbid(unsafe_code)]

use fe2o3_kfd::{DeviceSelector, OpenedKfd};
use ferric_engine::{
    execute_m1_k7_s1k4_packet_v1, reopen_persisted_m1_kernel_artifacts_from_directory_v1,
    reopen_persisted_m1_kernel_artifacts_v1,
};
use rustix::fd::OwnedFd;
use rustix::fs::{fstat, openat2, FileType, Mode, OFlags, ResolveFlags, Stat, CWD};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

type HarnessResult<T> = Result<T, String>;

// These literals are an IPC contract with produce-hardware-transcript.py.
const REQUEST_FORMAT: &str = "FERRIC-M1-HARDWARE-HARNESS-REQUEST-V1";
const RESULT_FORMAT: &str = "FERRIC-M1-HARDWARE-HARNESS-RESULT-V1";
const ENVIRONMENT_FORMAT: &str = "FERRIC-M1-HARDWARE-ENVIRONMENT-V1";
const PROTOCOL: &str = "ferric.m1.mi300x-hardware-test.v1";
const TARGET: &str = "gfx942:xnack-";
const PROGRAM: &str = "k7-speculative-token-assembly-s1k4";
const STATUS: &str = "pass";
const MARKETING_NAME: &str = "AMD Instinct MI300X";
const PROCESSOR: &str = "gfx942";
const VENDOR_ID: &str = "1002";
const XNACK: &str = "disabled";
const DRIVER_NAME: &str = "amdgpu";
const OBSERVATION_DOMAIN: &str = "ferric-m1-k7-observation-v1";
const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");
const MAX_JSON_BYTES: u64 = 64 * 1_024;
const USAGE: &str = "usage: ferric-m1-hardware-harness KERNEL_ARTIFACTS HARDWARE_ENVIRONMENT";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct HardwareHarnessRequestV1 {
    case: HardwareHarnessCaseV1,
    format: String,
    protocol: String,
    target: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct HardwareHarnessCaseV1 {
    binding_sha256: String,
    case_id: String,
    procedure_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct HardwareEnvironmentV1 {
    device: HardwareDeviceV1,
    driver: DriverEnvironmentV1,
    firmware: FirmwareEnvironmentV1,
    format: String,
    gpu_unique_id: u64,
    rocm: RocmEnvironmentV1,
    target: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct HardwareDeviceV1 {
    device_count: u32,
    device_uuid: String,
    marketing_name: String,
    pci_bdf: String,
    processor: String,
    vendor_id: String,
    xnack: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct DriverEnvironmentV1 {
    module_sha256: String,
    name: String,
    version: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct FirmwareEnvironmentV1 {
    bundle_sha256: String,
    package_version: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct RocmEnvironmentV1 {
    installation_sha256: String,
    version: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
struct HardwareHarnessResultV1 {
    case_result: HardwareCaseResultV1,
    device: HardwareDeviceV1,
    environment: ResultEnvironmentV1,
    finished_at_utc: String,
    format: &'static str,
    gpu_work_completed: bool,
    gpu_work_submitted: bool,
    kernel_catalog_sha256: String,
    kernel_manifest_sha256: String,
    no_gpu_work: bool,
    protocol: &'static str,
    run_id: String,
    started_at_utc: String,
    status: &'static str,
    target: &'static str,
    tool_source_sha256s: BTreeMap<&'static str, String>,
    tool_version: &'static str,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
struct HardwareCaseResultV1 {
    binding_sha256: String,
    case_id: String,
    completion_count: u32,
    generation: u64,
    gpu_observation_sha256: String,
    grid: [u32; 3],
    launch_count: u32,
    output_tokens: [u32; 5],
    output_verified: bool,
    procedure_sha256: String,
    program: &'static str,
    queue_released: bool,
    workgroup: [u32; 3],
}

#[derive(Debug, Eq, PartialEq, Serialize)]
struct ResultEnvironmentV1 {
    driver: DriverEnvironmentV1,
    firmware: FirmwareEnvironmentV1,
    rocm: RocmEnvironmentV1,
}

#[derive(Debug, Eq, PartialEq)]
struct Command {
    artifact_root: PathBuf,
    environment_path: PathBuf,
}

fn main() -> ExitCode {
    match parse_command(std::env::args_os().skip(1).collect()).and_then(execute) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ferric-m1-hardware-harness: {error}");
            ExitCode::FAILURE
        }
    }
}

fn parse_command(arguments: Vec<OsString>) -> HarnessResult<Command> {
    match arguments.as_slice() {
        [artifact_root, environment_path] => Ok(Command {
            artifact_root: PathBuf::from(artifact_root),
            environment_path: PathBuf::from(environment_path),
        }),
        _ => Err(USAGE.to_owned()),
    }
}

fn execute(command: Command) -> HarnessResult<()> {
    let started_seconds = unix_seconds()?;
    let run_nonce = unix_nanos()?;
    let started_at_utc = utc_timestamp(started_seconds)?;

    let request: HardwareHarnessRequestV1 = read_canonical_stdin("hardware request")?;
    validate_request(&request)?;
    let binding_id = request
        .case
        .case_id
        .strip_prefix("case.k7.")
        .ok_or_else(|| "validated case ID lost its binding prefix".to_owned())?;
    let run_id = format!("run.{binding_id}.{run_nonce}.{}", std::process::id());
    let environment: HardwareEnvironmentV1 =
        read_canonical_file(&command.environment_path, "hardware environment")?;
    validate_environment(&environment)?;

    let artifacts = if is_proc_self_fd_path(&command.artifact_root) {
        let root = open_descriptor_directory(&command.artifact_root)?;
        reopen_persisted_m1_kernel_artifacts_from_directory_v1(root)
    } else {
        reopen_persisted_m1_kernel_artifacts_v1(&command.artifact_root)
    }
    .map_err(|error| format!("cannot authenticate persisted kernel artifacts: {error}"))?;
    let kernel_manifest_sha256 = hex(artifacts.manifest().identity().sha256());
    let kernel_catalog_sha256 = hex(artifacts.program_catalog_id().as_bytes());
    require_sha256(&kernel_manifest_sha256, "kernel manifest identity")?;
    require_sha256(&kernel_catalog_sha256, "kernel catalog identity")?;

    let checked = OpenedKfd::open_default()
        .map_err(|error| format!("cannot open KFD: {error}"))?
        .admit_uapi()
        .map_err(|error| format!("cannot admit pinned KFD UAPI: {error}"))?
        .bind_gfx942_xnack_minus(DeviceSelector::UniqueId(environment.gpu_unique_id))
        .map_err(|error| format!("cannot bind selected gfx942:xnack- device: {error}"))?;
    let observation = checked.observation();
    if observation.unique_id() != environment.gpu_unique_id {
        return Err("KFD returned a device with a different GPU unique ID".to_owned());
    }
    let observed_bdf = observation.pci().to_string();
    if observed_bdf != environment.device.pci_bdf {
        return Err(format!(
            "KFD PCI BDF differs from hardware environment: expected {}, got {observed_bdf}",
            environment.device.pci_bdf
        ));
    }
    let derived_uuid = amd_smi_uuid(environment.gpu_unique_id);
    if derived_uuid != environment.device.device_uuid {
        return Err(format!(
            "AMD SMI UUID differs from KFD unique ID mapping: expected {derived_uuid}, got {}",
            environment.device.device_uuid
        ));
    }

    let packet_observation = artifacts
        .with_content_bound_program_catalog_v1(|catalog| {
            let mut no_report = |_| {};
            execute_m1_k7_s1k4_packet_v1(checked, catalog, &mut no_report)
        })
        .map_err(|error| format!("cannot bind content-bound program catalog: {error}"))??;
    let generation = packet_observation.dispatch_generation();
    if generation == 0 {
        return Err("completed K7 dispatch returned generation zero".to_owned());
    }
    let output_tokens = packet_observation.output_tokens();
    let gpu_observation_sha256 = observation_sha256(
        &request.case,
        &kernel_manifest_sha256,
        &kernel_catalog_sha256,
        &environment.device,
        generation,
        output_tokens,
    );

    let finished_seconds = finish_after(started_seconds)?;
    let result = HardwareHarnessResultV1 {
        case_result: HardwareCaseResultV1 {
            binding_sha256: request.case.binding_sha256,
            case_id: request.case.case_id,
            completion_count: 1,
            generation,
            gpu_observation_sha256,
            grid: [64, 1, 1],
            launch_count: 1,
            output_tokens,
            output_verified: true,
            procedure_sha256: request.case.procedure_sha256,
            program: PROGRAM,
            queue_released: true,
            workgroup: [64, 1, 1],
        },
        device: environment.device,
        environment: ResultEnvironmentV1 {
            driver: environment.driver,
            firmware: environment.firmware,
            rocm: environment.rocm,
        },
        finished_at_utc: utc_timestamp(finished_seconds)?,
        format: RESULT_FORMAT,
        gpu_work_completed: true,
        gpu_work_submitted: true,
        kernel_catalog_sha256,
        kernel_manifest_sha256,
        no_gpu_work: false,
        protocol: PROTOCOL,
        run_id,
        started_at_utc,
        status: STATUS,
        target: TARGET,
        tool_source_sha256s: tool_source_sha256s(),
        tool_version: TOOL_VERSION,
    };
    write_canonical_stdout(&result)
}

fn validate_request(request: &HardwareHarnessRequestV1) -> HarnessResult<()> {
    require_literal(&request.format, REQUEST_FORMAT, "request format")?;
    require_literal(&request.protocol, PROTOCOL, "request protocol")?;
    require_literal(&request.target, TARGET, "request target")?;
    require_case_id(&request.case.case_id)?;
    require_sha256(&request.case.binding_sha256, "binding SHA-256")?;
    require_sha256(&request.case.procedure_sha256, "procedure SHA-256")
}

fn validate_environment(environment: &HardwareEnvironmentV1) -> HarnessResult<()> {
    require_literal(
        &environment.format,
        ENVIRONMENT_FORMAT,
        "environment format",
    )?;
    require_literal(&environment.target, TARGET, "environment target")?;
    if environment.gpu_unique_id == 0 {
        return Err("environment GPU unique ID must be nonzero".to_owned());
    }
    let device = &environment.device;
    if device.device_count != 1 {
        return Err("environment must describe exactly one selected device".to_owned());
    }
    require_uuid(&device.device_uuid)?;
    require_pci_bdf(&device.pci_bdf)?;
    require_literal(
        &device.marketing_name,
        MARKETING_NAME,
        "device marketing name",
    )?;
    require_literal(&device.processor, PROCESSOR, "device processor")?;
    require_literal(&device.vendor_id, VENDOR_ID, "device vendor ID")?;
    require_literal(&device.xnack, XNACK, "device XNACK mode")?;
    require_literal(&environment.driver.name, DRIVER_NAME, "driver name")?;
    require_sha256(&environment.driver.module_sha256, "driver module SHA-256")?;
    require_sha256(
        &environment.firmware.bundle_sha256,
        "firmware bundle SHA-256",
    )?;
    require_sha256(
        &environment.rocm.installation_sha256,
        "ROCm installation SHA-256",
    )?;
    require_printable(&environment.driver.version, "driver version")?;
    require_printable(
        &environment.firmware.package_version,
        "firmware package version",
    )?;
    require_printable(&environment.rocm.version, "ROCm version")
}

fn require_literal(actual: &str, expected: &str, description: &str) -> HarnessResult<()> {
    if actual != expected {
        return Err(format!("{description} must be {expected:?}"));
    }
    Ok(())
}

fn require_case_id(value: &str) -> HarnessResult<()> {
    let Some(ordinal) = value.strip_prefix("case.k7.binding-") else {
        return Err("case ID must have shape case.k7.binding-NNNNN".to_owned());
    };
    if ordinal.len() != 5 || !ordinal.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("case ID must have shape case.k7.binding-NNNNN".to_owned());
    }
    Ok(())
}

fn require_sha256(value: &str, description: &str) -> HarnessResult<()> {
    let valid = value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    let nonplaceholder = value
        .as_bytes()
        .first()
        .is_some_and(|first| value.as_bytes().iter().any(|byte| byte != first));
    if !valid || !nonplaceholder {
        return Err(format!(
            "{description} must be a lowercase nonplaceholder SHA-256"
        ));
    }
    Ok(())
}

fn require_uuid(value: &str) -> HarnessResult<()> {
    let bytes = value.as_bytes();
    let hyphens = bytes.len() == 36
        && bytes.get(8) == Some(&b'-')
        && bytes.get(13) == Some(&b'-')
        && bytes.get(18) == Some(&b'-')
        && bytes.get(23) == Some(&b'-');
    let hex = bytes.iter().enumerate().all(|(index, byte)| {
        matches!(index, 8 | 13 | 18 | 23) || byte.is_ascii_digit() || (b'a'..=b'f').contains(byte)
    });
    let version = bytes
        .get(14)
        .is_some_and(|byte| (b'1'..=b'5').contains(byte));
    let variant = bytes
        .get(19)
        .is_some_and(|byte| matches!(byte, b'8' | b'9' | b'a' | b'b'));
    if !hyphens || !hex || !version || !variant {
        return Err("device UUID must be a lowercase RFC UUID".to_owned());
    }
    Ok(())
}

fn require_pci_bdf(value: &str) -> HarnessResult<()> {
    let bytes = value.as_bytes();
    let punctuation = bytes.len() == 12
        && bytes.get(4) == Some(&b':')
        && bytes.get(7) == Some(&b':')
        && bytes.get(10) == Some(&b'.');
    let hex = bytes.iter().enumerate().all(|(index, byte)| {
        matches!(index, 4 | 7 | 10) || byte.is_ascii_digit() || (b'a'..=b'f').contains(byte)
    });
    let device = value.get(8..10).and_then(parse_hex_byte);
    let function = value
        .get(11..12)
        .and_then(|part| u8::from_str_radix(part, 16).ok());
    if !punctuation
        || !hex
        || !device.is_some_and(|number| number <= 31)
        || !function.is_some_and(|number| number <= 7)
    {
        return Err("device PCI BDF must be canonical lowercase dddd:bb:ss.f".to_owned());
    }
    Ok(())
}

fn parse_hex_byte(value: &str) -> Option<u8> {
    u8::from_str_radix(value, 16).ok()
}

fn amd_smi_uuid(gpu_unique_id: u64) -> String {
    let top_byte = gpu_unique_id >> 56;
    let next_byte = (gpu_unique_id >> 48) & 0xff;
    let low_48_bits = gpu_unique_id & 0x0000_ffff_ffff_ffff;
    format!("{top_byte:02x}ff74a1-0000-1000-80{next_byte:02x}-{low_48_bits:012x}")
}

fn tool_source_sha256s() -> BTreeMap<&'static str, String> {
    [
        (
            "cargo_lock",
            include_bytes!("../../../../Cargo.lock").as_slice(),
        ),
        (
            "hardware_harness",
            include_bytes!("ferric-m1-hardware-harness.rs").as_slice(),
        ),
        (
            "package_manifest",
            include_bytes!("../../Cargo.toml").as_slice(),
        ),
        (
            "packet_execution",
            include_bytes!("../m1_packet_diagnostic_execution.rs").as_slice(),
        ),
        (
            "persisted_kernel_artifacts",
            include_bytes!("../persisted_kernel_artifacts.rs").as_slice(),
        ),
    ]
    .into_iter()
    .map(|(path, bytes)| (path, hex(&Sha256::digest(bytes))))
    .collect()
}

fn is_proc_self_fd_path(path: &Path) -> bool {
    path.as_os_str()
        .as_bytes()
        .strip_prefix(b"/proc/self/fd/")
        .is_some_and(|suffix| !suffix.is_empty() && suffix.iter().all(u8::is_ascii_digit))
}

fn open_descriptor_directory(path: &Path) -> HarnessResult<OwnedFd> {
    let descriptor = openat2(
        CWD,
        path,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::empty(),
    )
    .map_err(|error| format!("cannot duplicate held kernel artifact directory: {error}"))?;
    let metadata = fstat(&descriptor)
        .map_err(|error| format!("cannot inspect held kernel artifact directory: {error}"))?;
    if FileType::from_raw_mode(metadata.st_mode) != FileType::Directory {
        return Err("held kernel artifact descriptor must name a directory".to_owned());
    }
    Ok(descriptor)
}

fn require_printable(value: &str, description: &str) -> HarnessResult<()> {
    if value.is_empty()
        || value.len() > 256
        || !value.bytes().all(|byte| (0x20..=0x7e).contains(&byte))
    {
        return Err(format!(
            "{description} must be nonempty bounded printable ASCII"
        ));
    }
    Ok(())
}

fn observation_sha256(
    case: &HardwareHarnessCaseV1,
    kernel_manifest_sha256: &str,
    kernel_catalog_sha256: &str,
    device: &HardwareDeviceV1,
    generation: u64,
    output_tokens: [u32; 5],
) -> String {
    let tokens = format!(
        "{},{},{},{},{}",
        output_tokens[0], output_tokens[1], output_tokens[2], output_tokens[3], output_tokens[4]
    );
    let preimage = format!(
        "{OBSERVATION_DOMAIN}|{}|{}|{}|{kernel_manifest_sha256}|{kernel_catalog_sha256}|{}|{}|{generation}|{tokens}\n",
        case.binding_sha256,
        case.case_id,
        case.procedure_sha256,
        device.device_uuid,
        device.pci_bdf,
    );
    hex(&Sha256::digest(preimage.as_bytes()))
}

fn read_canonical_stdin<T>(description: &str) -> HarnessResult<T>
where
    T: for<'de> Deserialize<'de> + Serialize,
{
    let mut raw = Vec::new();
    std::io::stdin()
        .lock()
        .take(MAX_JSON_BYTES + 1)
        .read_to_end(&mut raw)
        .map_err(|error| format!("cannot read {description}: {error}"))?;
    decode_canonical_json(&raw, description)
}

fn read_canonical_file<T>(path: &Path, description: &str) -> HarnessResult<T>
where
    T: for<'de> Deserialize<'de> + Serialize,
{
    let descriptor = if is_proc_self_fd_path(path) {
        openat2(
            CWD,
            path,
            OFlags::RDONLY | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::empty(),
        )
        .map_err(|error| format!("cannot duplicate held {description}: {error}"))?
    } else {
        openat2(
            CWD,
            path,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )
        .map_err(|error| format!("cannot securely open {description}: {error}"))?
    };
    read_canonical_descriptor(descriptor, description)
}

fn read_canonical_descriptor<T>(descriptor: OwnedFd, description: &str) -> HarnessResult<T>
where
    T: for<'de> Deserialize<'de> + Serialize,
{
    let initial =
        fstat(&descriptor).map_err(|error| format!("cannot inspect {description}: {error}"))?;
    let initial_size = u64::try_from(initial.st_size)
        .map_err(|_| format!("{description} has a negative byte length"))?;
    if FileType::from_raw_mode(initial.st_mode) != FileType::RegularFile
        || initial.st_nlink != 1
        || initial_size == 0
        || initial_size > MAX_JSON_BYTES
    {
        return Err(format!(
            "{description} must be a nonempty bounded regular single-link file"
        ));
    }
    let mut file = File::from(descriptor);
    let mut raw = Vec::new();
    Read::by_ref(&mut file)
        .take(MAX_JSON_BYTES + 1)
        .read_to_end(&mut raw)
        .map_err(|error| format!("cannot read {description}: {error}"))?;
    let final_stat =
        fstat(&file).map_err(|error| format!("cannot reinspect {description}: {error}"))?;
    if u64::try_from(raw.len()).ok() != Some(initial_size) || !same_file(&initial, &final_stat) {
        return Err(format!("{description} changed while it was read"));
    }
    decode_canonical_json(&raw, description)
}

fn same_file(left: &Stat, right: &Stat) -> bool {
    left.st_dev == right.st_dev
        && left.st_ino == right.st_ino
        && left.st_mode == right.st_mode
        && left.st_nlink == right.st_nlink
        && left.st_size == right.st_size
        && left.st_mtime == right.st_mtime
        && left.st_mtime_nsec == right.st_mtime_nsec
        && left.st_ctime == right.st_ctime
        && left.st_ctime_nsec == right.st_ctime_nsec
}

fn decode_canonical_json<T>(raw: &[u8], description: &str) -> HarnessResult<T>
where
    T: for<'de> Deserialize<'de> + Serialize,
{
    if raw.is_empty() || raw.len() as u64 > MAX_JSON_BYTES {
        return Err(format!("{description} size is outside the admitted bound"));
    }
    let value: T = serde_json::from_slice(raw)
        .map_err(|error| format!("cannot decode {description}: {error}"))?;
    let expected = canonical_json(&value)?;
    if raw != expected {
        return Err(format!(
            "{description} must be canonical pretty ASCII JSON with one trailing newline"
        ));
    }
    Ok(value)
}

fn canonical_json<T: Serialize>(value: &T) -> HarnessResult<Vec<u8>> {
    let mut raw = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("cannot encode canonical JSON: {error}"))?;
    if !raw.is_ascii() {
        return Err("canonical JSON must be ASCII".to_owned());
    }
    raw.push(b'\n');
    Ok(raw)
}

fn write_canonical_stdout<T: Serialize>(value: &T) -> HarnessResult<()> {
    let raw = canonical_json(value)?;
    std::io::stdout()
        .lock()
        .write_all(&raw)
        .map_err(|error| format!("cannot write harness result: {error}"))
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from(DIGITS[usize::from(byte >> 4)]));
        encoded.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    encoded
}

fn unix_seconds() -> HarnessResult<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| "system clock precedes the Unix epoch".to_owned())
}

fn unix_nanos() -> HarnessResult<u128> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .map_err(|_| "system clock precedes the Unix epoch".to_owned())
}

fn finish_after(started_seconds: u64) -> HarnessResult<u64> {
    loop {
        let finished_seconds = unix_seconds()?;
        if finished_seconds > started_seconds {
            return Ok(finished_seconds);
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn utc_timestamp(seconds: u64) -> HarnessResult<String> {
    let days = i64::try_from(seconds / 86_400)
        .map_err(|_| "UTC timestamp day count exceeds the supported range".to_owned())?;
    let seconds_in_day = seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    if !(0..=9_999).contains(&year) {
        return Err("UTC timestamp year exceeds four decimal digits".to_owned());
    }
    let hour = seconds_in_day / 3_600;
    let minute = (seconds_in_day % 3_600) / 60;
    let second = seconds_in_day % 60;
    Ok(format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z"
    ))
}

fn civil_from_days(days_since_epoch: i64) -> (i64, i64, i64) {
    let shifted = days_since_epoch + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::os::fd::AsRawFd;

    const DIGEST_A: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const DIGEST_B: &str = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";

    fn case() -> HardwareHarnessCaseV1 {
        HardwareHarnessCaseV1 {
            binding_sha256: DIGEST_A.to_owned(),
            case_id: "case.k7.binding-00019".to_owned(),
            procedure_sha256: DIGEST_B.to_owned(),
        }
    }

    fn request() -> HardwareHarnessRequestV1 {
        HardwareHarnessRequestV1 {
            case: case(),
            format: REQUEST_FORMAT.to_owned(),
            protocol: PROTOCOL.to_owned(),
            target: TARGET.to_owned(),
        }
    }

    fn device() -> HardwareDeviceV1 {
        HardwareDeviceV1 {
            device_count: 1,
            device_uuid: "123e4567-e89b-42d3-a456-426614174000".to_owned(),
            marketing_name: MARKETING_NAME.to_owned(),
            pci_bdf: "0000:41:00.0".to_owned(),
            processor: PROCESSOR.to_owned(),
            vendor_id: VENDOR_ID.to_owned(),
            xnack: XNACK.to_owned(),
        }
    }

    fn environment() -> HardwareEnvironmentV1 {
        HardwareEnvironmentV1 {
            device: device(),
            driver: DriverEnvironmentV1 {
                module_sha256: DIGEST_A.to_owned(),
                name: DRIVER_NAME.to_owned(),
                version: "6.12.12".to_owned(),
            },
            firmware: FirmwareEnvironmentV1 {
                bundle_sha256: DIGEST_B.to_owned(),
                package_version: "20260824".to_owned(),
            },
            format: ENVIRONMENT_FORMAT.to_owned(),
            gpu_unique_id: 42,
            rocm: RocmEnvironmentV1 {
                installation_sha256: DIGEST_A.to_owned(),
                version: "7.1.0".to_owned(),
            },
            target: TARGET.to_owned(),
        }
    }

    #[test]
    fn request_accepts_only_exact_canonical_json() {
        let expected = request();
        let canonical = canonical_json(&expected).unwrap();
        assert_eq!(
            decode_canonical_json::<HardwareHarnessRequestV1>(&canonical, "request").unwrap(),
            expected
        );
        let compact = serde_json::to_vec(&expected).unwrap();
        assert!(decode_canonical_json::<HardwareHarnessRequestV1>(&compact, "request").is_err());
        let mut extra: serde_json::Value = serde_json::from_slice(&canonical).unwrap();
        extra["extra"] = json!(true);
        let extra = serde_json::to_vec_pretty(&extra).unwrap();
        assert!(decode_canonical_json::<HardwareHarnessRequestV1>(&extra, "request").is_err());
    }

    #[test]
    fn request_validation_is_single_case_and_fail_closed() {
        assert!(validate_request(&request()).is_ok());
        let mut invalid = request();
        invalid.case.case_id = "case.k7.binding-00019.extra".to_owned();
        assert!(validate_request(&invalid).is_err());
        let mut invalid = request();
        invalid.case.binding_sha256 = "0".repeat(64);
        assert!(validate_request(&invalid).is_err());
        let mut invalid = request();
        invalid.protocol.push_str(".drift");
        assert!(validate_request(&invalid).is_err());
    }

    #[test]
    fn device_identity_syntax_is_exact() {
        assert!(require_uuid("123e4567-e89b-42d3-a456-426614174000").is_ok());
        assert!(require_uuid("123E4567-e89b-42d3-a456-426614174000").is_err());
        assert!(require_uuid("00000000-0000-0000-0000-000000000000").is_err());
        assert!(require_pci_bdf("0000:41:00.0").is_ok());
        assert!(require_pci_bdf("0000:41:20.0").is_err());
        assert!(require_pci_bdf("0000:41:00.8").is_err());
        assert!(require_pci_bdf("0000:AF:00.0").is_err());
    }

    #[test]
    fn amd_smi_uuid_is_derived_from_all_unique_id_bytes() {
        assert_eq!(
            amd_smi_uuid(0x1234_5678_9abc_def0),
            "12ff74a1-0000-1000-8034-56789abcdef0"
        );
        assert_eq!(
            amd_smi_uuid(0xabcd_0000_0000_0001),
            "abff74a1-0000-1000-80cd-000000000001"
        );
        assert!(require_uuid(&amd_smi_uuid(0x1234_5678_9abc_def0)).is_ok());
    }

    #[test]
    fn tool_version_and_source_identities_are_binary_derived() {
        assert_eq!(TOOL_VERSION, env!("CARGO_PKG_VERSION"));
        let sources = tool_source_sha256s();
        let expected_sources = [
            ("cargo_lock", "Cargo.lock"),
            (
                "hardware_harness",
                "crates/ferric-engine/src/bin/ferric-m1-hardware-harness.rs",
            ),
            ("package_manifest", "crates/ferric-engine/Cargo.toml"),
            (
                "packet_execution",
                "crates/ferric-engine/src/m1_packet_diagnostic_execution.rs",
            ),
            (
                "persisted_kernel_artifacts",
                "crates/ferric-engine/src/persisted_kernel_artifacts.rs",
            ),
        ];
        assert_eq!(
            sources.keys().copied().collect::<Vec<_>>(),
            expected_sources.map(|(key, _)| key)
        );

        let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for (key, path) in expected_sources {
            let bytes = std::fs::read(repository.join(path)).unwrap();
            assert_eq!(sources[key], hex(&Sha256::digest(bytes)));
            require_sha256(&sources[key], key).unwrap();
        }
    }

    #[test]
    fn observation_digest_uses_the_locked_preimage() {
        let actual = observation_sha256(
            &case(),
            DIGEST_A,
            DIGEST_B,
            &device(),
            7,
            [10, 11, 12, 13, 14],
        );
        let expected = hex(&Sha256::digest(
            format!(
                "{OBSERVATION_DOMAIN}|{DIGEST_A}|case.k7.binding-00019|{DIGEST_B}|{DIGEST_A}|{DIGEST_B}|123e4567-e89b-42d3-a456-426614174000|0000:41:00.0|7|10,11,12,13,14\n"
            )
            .as_bytes(),
        ));
        assert_eq!(actual, expected);
    }

    #[test]
    fn utc_timestamp_is_second_precision() {
        assert_eq!(utc_timestamp(0).unwrap(), "1970-01-01T00:00:00Z");
        assert_eq!(utc_timestamp(951_827_696).unwrap(), "2000-02-29T12:34:56Z");
        assert_eq!(
            utc_timestamp(1_774_163_696).unwrap(),
            "2026-03-22T07:14:56Z"
        );
    }

    #[test]
    fn command_has_exactly_two_paths() {
        assert!(parse_command(vec!["artifacts".into(), "environment.json".into()]).is_ok());
        assert!(parse_command(Vec::new()).is_err());
        assert!(parse_command(vec![
            "run".into(),
            "artifacts".into(),
            "environment.json".into(),
        ])
        .is_err());
    }

    #[test]
    fn descriptor_directory_path_shape_is_exact() {
        assert!(is_proc_self_fd_path(Path::new("/proc/self/fd/17")));
        assert!(!is_proc_self_fd_path(Path::new("/proc/self/fd/")));
        assert!(!is_proc_self_fd_path(Path::new("/proc/self/fd/17/objects")));
        assert!(!is_proc_self_fd_path(Path::new("artifacts")));

        let held = File::open(".").unwrap();
        let path = PathBuf::from(format!("/proc/self/fd/{}", held.as_raw_fd()));
        let duplicate = open_descriptor_directory(&path).unwrap();
        let metadata = fstat(&duplicate).unwrap();
        assert_eq!(
            FileType::from_raw_mode(metadata.st_mode),
            FileType::Directory
        );
    }

    #[test]
    fn held_environment_descriptor_is_regular_stable_and_canonical() {
        let nonce = unix_nanos().unwrap();
        let file_path = std::env::temp_dir().join(format!(
            "ferric-m1-hardware-environment-{}-{nonce}.json",
            std::process::id()
        ));
        let expected = environment();
        std::fs::write(&file_path, canonical_json(&expected).unwrap()).unwrap();
        assert_eq!(
            read_canonical_file::<HardwareEnvironmentV1>(&file_path, "hardware environment")
                .unwrap(),
            expected
        );
        let held = File::open(&file_path).unwrap();
        let descriptor_path = PathBuf::from(format!("/proc/self/fd/{}", held.as_raw_fd()));
        let actual =
            read_canonical_file::<HardwareEnvironmentV1>(&descriptor_path, "hardware environment")
                .unwrap();
        assert_eq!(actual, expected);

        std::fs::write(&file_path, serde_json::to_vec(&expected).unwrap()).unwrap();
        assert!(read_canonical_file::<HardwareEnvironmentV1>(
            &descriptor_path,
            "hardware environment"
        )
        .is_err());
        std::fs::remove_file(&file_path).unwrap();

        let held_directory = File::open(".").unwrap();
        let directory_path = PathBuf::from(format!("/proc/self/fd/{}", held_directory.as_raw_fd()));
        assert!(read_canonical_file::<HardwareEnvironmentV1>(
            &directory_path,
            "hardware environment"
        )
        .unwrap_err()
        .contains("regular single-link file"));
    }
}
