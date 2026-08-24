//! Policy-bound D10 subprocess observation collection.
//!
//! The collector executes only exact held ELF files from a canonical manifest,
//! clears the inherited environment, sends one canonical request on stdin, and
//! accepts one bounded canonical JSON response on stdout. It records counters
//! and identities; it does not choose thresholds or synthesize measurements.

use crate::d10_observations::{
    validate_admission_binding, ExactBundle, HeldBundle, ADMISSION_FILES, CASE_ROSTER,
    IMPLEMENTATIONS, PROTOCOL_BYTES, PROTOCOL_SHA256,
};
use crate::d10_policy::{hold_validated_policy, HeldValidatedPolicy};
use ferric_m1_benchmarks::{encode_canonical_document, sha256_identity, BenchResult};
use rustix::fs::{fstat, openat2, FileType, Mode, OFlags, ResolveFlags, Stat, CWD};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::fd::AsRawFd;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub(super) const COMMAND: &str = "collect-policy-observations";

const MANIFEST_FORMAT: &str = "FERRIC-M1-D10-COLLECTION-MANIFEST-V1";
const MANIFEST_AUTHORITY: &str = "externally-supplied-pre-observation-d10-collection-manifest-only";
const OBSERVATION_FORMAT: &str = "FERRIC-M1-D10-POLICY-OBSERVATIONS-V2";
const OBSERVATION_AUTHORITY: &str = "ferric-collected-policy-bound-d10-observations-only";
const SAMPLE_REQUEST_FORMAT: &str = "FERRIC-M1-D10-SAMPLE-REQUEST-V1";
const SAMPLE_RESULT_FORMAT: &str = "FERRIC-M1-D10-SAMPLE-RESULT-V1";
const RESOURCE_REQUEST_FORMAT: &str = "FERRIC-M1-D10-RESOURCE-REQUEST-V1";
const RESOURCE_RESULT_FORMAT: &str = "FERRIC-M1-D10-RESOURCE-RESULT-V1";
const ENVIRONMENT_FORMAT: &str = "FERRIC-M1-D10-ENVIRONMENT-SNAPSHOT-V1";
const TARGET: &str = "gfx942:xnack-";
const WARMUPS: usize = 10;
const RECORDED: usize = 30;
const MAX_DOCUMENT_BYTES: usize = 8 * 1024 * 1024;
const MAX_BINARY_BYTES: usize = 512 * 1024 * 1024;
const MAX_SUBPROCESS_BYTES: usize = 1024 * 1024;
const MAX_TIMEOUT_MS: u64 = 3_600_000;
const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
const COLLECTED_FILES: &[&str] = &["observations.json", "protocol.json"];

struct HeldCanonicalFile {
    bytes: Vec<u8>,
    file: File,
    initial: Stat,
    path: PathBuf,
    value: Value,
}

impl HeldCanonicalFile {
    fn open(path: &Path, description: &str) -> BenchResult<Self> {
        require_safe_input_path(path, description)?;
        let descriptor = openat2(
            CWD,
            path,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )
        .map_err(|error| format!("cannot securely open {description}: {error}"))?;
        let initial =
            fstat(&descriptor).map_err(|error| format!("cannot inspect {description}: {error}"))?;
        if FileType::from_raw_mode(initial.st_mode) != FileType::RegularFile
            || initial.st_nlink != 1
        {
            return Err(format!("{description} must be a one-link regular file"));
        }
        let length = usize::try_from(initial.st_size)
            .map_err(|_| format!("{description} length is invalid"))?;
        if length == 0 || length > MAX_DOCUMENT_BYTES {
            return Err(format!(
                "{description} length is outside the admitted bound"
            ));
        }
        let mut file = File::from(descriptor);
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length.saturating_add(1))
            .map_err(|_| format!("cannot reserve {description} input buffer"))?;
        Read::by_ref(&mut file)
            .take(MAX_DOCUMENT_BYTES.saturating_add(1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("cannot read {description}: {error}"))?;
        let final_stat =
            fstat(&file).map_err(|error| format!("cannot reinspect {description}: {error}"))?;
        if bytes.len() != length || !same_snapshot(&initial, &final_stat) {
            return Err(format!("{description} changed while being read"));
        }
        if !bytes.is_ascii() {
            return Err(format!("{description} must be ASCII JSON"));
        }
        let value: Value = serde_json::from_slice(&bytes)
            .map_err(|error| format!("cannot parse {description}: {error}"))?;
        if encode_canonical_document(&value)? != bytes {
            return Err(format!("{description} must be canonical JSON"));
        }
        Ok(Self {
            bytes,
            file,
            initial,
            path: path.to_path_buf(),
            value,
        })
    }

    fn revalidate(&mut self, description: &str) -> BenchResult<()> {
        let held = fstat(&self.file)
            .map_err(|error| format!("cannot reinspect held {description}: {error}"))?;
        if !same_snapshot(&self.initial, &held) {
            return Err(format!("held {description} metadata changed"));
        }
        self.file
            .seek(SeekFrom::Start(0))
            .map_err(|error| format!("cannot rewind held {description}: {error}"))?;
        let mut bytes = Vec::new();
        Read::by_ref(&mut self.file)
            .take(MAX_DOCUMENT_BYTES.saturating_add(1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("cannot reread held {description}: {error}"))?;
        let reread = fstat(&self.file)
            .map_err(|error| format!("cannot inspect reread {description}: {error}"))?;
        if bytes != self.bytes || !same_snapshot(&self.initial, &reread) {
            return Err(format!("held {description} bytes changed"));
        }
        let rebound = openat2(
            CWD,
            &self.path,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )
        .map_err(|error| format!("cannot rebind held {description}: {error}"))?;
        let rebound = fstat(&rebound)
            .map_err(|error| format!("cannot inspect rebound {description}: {error}"))?;
        if !same_snapshot(&self.initial, &rebound) {
            return Err(format!(
                "{description} path no longer identifies the held file"
            ));
        }
        Ok(())
    }
}

struct HeldExecutable {
    digest: String,
    file: File,
    initial: Stat,
    path: PathBuf,
}

impl HeldExecutable {
    fn open(path: &Path, expected: &str, description: &str) -> BenchResult<Self> {
        if !path.is_absolute() {
            return Err(format!("{description} path must be absolute"));
        }
        require_safe_input_path(path, description)?;
        let descriptor = openat2(
            CWD,
            path,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )
        .map_err(|error| format!("cannot securely open {description}: {error}"))?;
        let initial =
            fstat(&descriptor).map_err(|error| format!("cannot inspect {description}: {error}"))?;
        let length = usize::try_from(initial.st_size)
            .map_err(|_| format!("{description} length is invalid"))?;
        if FileType::from_raw_mode(initial.st_mode) != FileType::RegularFile
            || initial.st_nlink != 1
            || initial.st_mode & 0o111 == 0
            || !(4..=MAX_BINARY_BYTES).contains(&length)
        {
            return Err(format!(
                "{description} must be one executable regular file within the size bound"
            ));
        }
        let mut file = File::from(descriptor);
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length)
            .map_err(|_| format!("cannot reserve {description} buffer"))?;
        Read::by_ref(&mut file)
            .take(MAX_BINARY_BYTES.saturating_add(1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("cannot read {description}: {error}"))?;
        if bytes.len() != length || !bytes.starts_with(b"\x7fELF") {
            return Err(format!("{description} must be one exact ELF binary"));
        }
        let digest = sha256_identity(&bytes);
        if digest != expected {
            return Err(format!("{description} SHA-256 drifted"));
        }
        file.seek(SeekFrom::Start(0))
            .map_err(|error| format!("cannot rewind {description}: {error}"))?;
        Ok(Self {
            digest,
            file,
            initial,
            path: path.to_path_buf(),
        })
    }

    fn proc_path(&self) -> PathBuf {
        PathBuf::from(format!("/proc/self/fd/{}", self.file.as_raw_fd()))
    }

    fn revalidate(&mut self, description: &str, reread: bool) -> BenchResult<()> {
        let held = fstat(&self.file)
            .map_err(|error| format!("cannot reinspect held {description}: {error}"))?;
        if !same_snapshot(&self.initial, &held) {
            return Err(format!("held {description} metadata changed"));
        }
        let rebound = openat2(
            CWD,
            &self.path,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )
        .map_err(|error| format!("cannot rebind held {description}: {error}"))?;
        let rebound = fstat(&rebound)
            .map_err(|error| format!("cannot inspect rebound {description}: {error}"))?;
        if !same_snapshot(&self.initial, &rebound) {
            return Err(format!(
                "{description} path no longer identifies the held binary"
            ));
        }
        if reread {
            self.file
                .seek(SeekFrom::Start(0))
                .map_err(|error| format!("cannot rewind held {description}: {error}"))?;
            let mut bytes = Vec::new();
            Read::by_ref(&mut self.file)
                .take(MAX_BINARY_BYTES.saturating_add(1) as u64)
                .read_to_end(&mut bytes)
                .map_err(|error| format!("cannot reread held {description}: {error}"))?;
            if sha256_identity(&bytes) != self.digest {
                return Err(format!("held {description} bytes changed"));
            }
        }
        Ok(())
    }
}

struct CommandSpec {
    arguments: Vec<String>,
    base_sha256: String,
    binary: HeldExecutable,
    protocol_sha256: String,
}

struct ImplementationPlan {
    command: CommandSpec,
    config: Value,
    config_sha256: String,
    name: String,
}

struct CasePlan {
    case_id: String,
    expected_resources: Value,
    expected_resources_sha256: String,
    holdout_member: Value,
    implementations: Vec<Option<ImplementationPlan>>,
    resource_command: CommandSpec,
}

struct CollectionPlan {
    cases: Vec<CasePlan>,
    collector_binary: HeldExecutable,
    environment: BTreeMap<String, String>,
    environment_sha256: String,
    manifest_sha256: String,
    timeout: Duration,
}

impl CollectionPlan {
    fn revalidate(&mut self, reread: bool) -> BenchResult<()> {
        self.collector_binary
            .revalidate("D10 collector binary", reread)?;
        for case in &mut self.cases {
            case.resource_command
                .binary
                .revalidate("D10 resource-inspection binary", reread)?;
            for implementation in case.implementations.iter_mut().flatten() {
                implementation
                    .command
                    .binary
                    .revalidate("D10 measurement binary", reread)?;
            }
        }
        Ok(())
    }
}

struct SubprocessResult {
    command_sha256: String,
    output_sha256: String,
    value: Value,
}

/// Collects exact K1-K7 observations from policy-bound external commands.
pub(super) fn collect_policy_observations(arguments: &[OsString]) -> BenchResult<()> {
    collect_policy_observations_with_hook(arguments, |_| Ok(()), || Ok(()))
}

fn collect_policy_observations_with_hook<F, G>(
    arguments: &[OsString],
    after_collection: F,
    before_publish: G,
) -> BenchResult<()>
where
    F: FnOnce(&Value) -> BenchResult<()>,
    G: FnOnce() -> BenchResult<()>,
{
    let [command, policy_root, admission_bundle, manifest_path, output] = arguments else {
        return Err("usage: ferric-m1-d10 collect-policy-observations POLICY-ROOT ADMISSION-BUNDLE COLLECTION-MANIFEST OUTPUT-BUNDLE".to_owned());
    };
    if command != COMMAND {
        return Err("D10 collection command drifted".to_owned());
    }
    let mut policy = hold_validated_policy(Path::new(policy_root))?;
    let mut admission = HeldBundle::open(
        Path::new(admission_bundle),
        ADMISSION_FILES,
        "D10 policy admission bundle",
    )?;
    validate_admission_binding(&policy, &admission)?;
    let mut manifest =
        HeldCanonicalFile::open(Path::new(manifest_path), "D10 collection manifest")?;
    let mut plan = parse_collection_plan(&policy, &manifest)?;
    preflight_order_bindings(&policy, &plan)?;
    let mut bundle = ExactBundle::create_with_hook(Path::new(output), COLLECTED_FILES, |_| Ok(()))?;
    let observations = collect_observations(&policy, &admission, &mut plan)?;
    after_collection(&observations)?;
    let observation_bytes = encode_canonical_document(&observations)?;
    let protocol_bytes = PROTOCOL_BYTES;
    if sha256_identity(protocol_bytes) != PROTOCOL_SHA256 {
        return Err("compiled D10 observation protocol identity drifted".to_owned());
    }
    policy.revalidate()?;
    admission.revalidate()?;
    manifest.revalidate("D10 collection manifest")?;
    plan.revalidate(true)?;
    bundle.write("observations.json", &observation_bytes)?;
    bundle.write("protocol.json", protocol_bytes)?;
    let expected = [
        ("observations.json", observation_bytes.as_slice()),
        ("protocol.json", protocol_bytes),
    ];
    let mut revalidate = || {
        policy.revalidate()?;
        admission.revalidate()?;
        manifest.revalidate("D10 collection manifest")?;
        plan.revalidate(true)
    };
    bundle.publish_exact(&expected, &mut revalidate, before_publish, || Ok(()))
}

fn parse_collection_plan(
    policy: &HeldValidatedPolicy,
    manifest: &HeldCanonicalFile,
) -> BenchResult<CollectionPlan> {
    let root = exact_object(
        &manifest.value,
        &[
            "authority",
            "cases",
            "collector_binary_path",
            "collector_binary_sha256",
            "environment",
            "format",
            "policy_sha256",
            "suite",
            "target",
            "timeout_ms",
        ],
        "D10 collection manifest",
    )?;
    expect_string(
        root,
        "authority",
        MANIFEST_AUTHORITY,
        "D10 collection manifest",
    )?;
    expect_string(root, "format", MANIFEST_FORMAT, "D10 collection manifest")?;
    expect_string(root, "suite", "d10", "D10 collection manifest")?;
    expect_string(root, "target", TARGET, "D10 collection manifest")?;
    expect_string(
        root,
        "policy_sha256",
        &sha256_identity(policy.document_bytes("policy.json")?),
        "D10 collection policy identity",
    )?;
    let timeout_ms = get_u64(root, "timeout_ms", "D10 collection manifest")?;
    if timeout_ms == 0 || timeout_ms > MAX_TIMEOUT_MS {
        return Err("D10 collection timeout is outside the admitted bound".to_owned());
    }
    let collector_path = Path::new(get_string(
        root,
        "collector_binary_path",
        "D10 collection manifest",
    )?);
    let collector_sha256 = get_string(root, "collector_binary_sha256", "D10 collection manifest")?;
    require_sha256(collector_sha256, "D10 collector binary identity")?;
    let current = std::env::current_exe()
        .map_err(|error| format!("cannot resolve D10 collector executable: {error}"))?;
    if current != collector_path {
        return Err("D10 collection manifest names a different collector executable".to_owned());
    }
    let collector_binary =
        HeldExecutable::open(collector_path, collector_sha256, "D10 collector binary")?;
    let (environment, environment_sha256) =
        parse_environment(policy, get(root, "environment", "D10 collection manifest")?)?;
    let cases_value = get(root, "cases", "D10 collection manifest")?
        .as_array()
        .ok_or_else(|| "D10 collection cases must be an array".to_owned())?;
    if cases_value.len() != CASE_ROSTER.len() {
        return Err("D10 collection manifest must cover the exact K1-K7 roster".to_owned());
    }
    let policy_cases = policy
        .document_value("policy.json")?
        .get("cases")
        .and_then(Value::as_array)
        .ok_or_else(|| "held D10 policy cases are unavailable".to_owned())?;
    let resource_cases = policy
        .document_value("resource-inspection.json")?
        .get("cases")
        .and_then(Value::as_array)
        .ok_or_else(|| "held D10 resource policy cases are unavailable".to_owned())?;
    let regression = policy
        .document_value("regression-reference.json")?
        .as_object()
        .ok_or_else(|| "held D10 regression reference must be an object".to_owned())?;
    let mut cases = Vec::with_capacity(CASE_ROSTER.len());
    for (((value, policy_case), resource_case), (expected_case, _)) in cases_value
        .iter()
        .zip(policy_cases)
        .zip(resource_cases)
        .zip(CASE_ROSTER)
    {
        let case = exact_object(
            value,
            &[
                "case_id",
                "expected_resources",
                "holdout_member_id",
                "implementations",
                "resource_command",
            ],
            "D10 collection case",
        )?;
        expect_string(case, "case_id", expected_case, "D10 collection case")?;
        let holdout_member = select_holdout_member(
            policy,
            get_string(case, "holdout_member_id", "D10 collection case")?,
        )?;
        let resource = resource_case
            .as_object()
            .ok_or_else(|| "held D10 resource case must be an object".to_owned())?;
        let expected_resources = get(case, "expected_resources", "D10 collection case")?.clone();
        validate_resource_map(&expected_resources, "D10 expected resources")?;
        let expected_resources_sha256 =
            sha256_identity(&encode_canonical_document(&expected_resources)?);
        expect_string(
            resource,
            "expected_resources_sha256",
            &expected_resources_sha256,
            "D10 expected-resource identity",
        )?;
        let resource_command = parse_command(
            get(case, "resource_command", "D10 collection case")?,
            Some(get_string(
                resource,
                "inspection_protocol_sha256",
                "held D10 resource policy",
            )?),
            "D10 resource command",
        )?;
        let implementations = parse_implementations(
            get(case, "implementations", "D10 collection case")?,
            policy_case,
            regression,
        )?;
        cases.push(CasePlan {
            case_id: (*expected_case).to_owned(),
            expected_resources,
            expected_resources_sha256,
            holdout_member,
            implementations,
            resource_command,
        });
    }
    Ok(CollectionPlan {
        cases,
        collector_binary,
        environment,
        environment_sha256,
        manifest_sha256: sha256_identity(&manifest.bytes),
        timeout: Duration::from_millis(timeout_ms),
    })
}

fn parse_environment(
    policy: &HeldValidatedPolicy,
    value: &Value,
) -> BenchResult<(BTreeMap<String, String>, String)> {
    let environment = exact_object(
        value,
        &["format", "variables"],
        "D10 collection environment",
    )?;
    expect_string(
        environment,
        "format",
        ENVIRONMENT_FORMAT,
        "D10 collection environment",
    )?;
    let variables = get(environment, "variables", "D10 collection environment")?
        .as_object()
        .ok_or_else(|| "D10 environment variables must be an object".to_owned())?;
    if variables.len() > 128 {
        return Err("D10 environment variable count exceeds the admitted bound".to_owned());
    }
    let mut parsed = BTreeMap::new();
    for (name, value) in variables {
        if name.is_empty()
            || name.len() > 128
            || !name.bytes().enumerate().all(|(index, byte)| {
                byte == b'_' || byte.is_ascii_uppercase() || (index > 0 && byte.is_ascii_digit())
            })
        {
            return Err(format!("invalid D10 environment variable name: {name}"));
        }
        let value = value
            .as_str()
            .ok_or_else(|| format!("D10 environment variable {name} must be a string"))?;
        if !value.is_ascii() || value.len() > 16 * 1024 || value.as_bytes().contains(&0) {
            return Err(format!("invalid D10 environment variable value: {name}"));
        }
        parsed.insert(name.clone(), value.to_owned());
    }
    let digest = sha256_identity(&encode_canonical_document(value)?);
    let telemetry = policy
        .document_value("telemetry.json")?
        .as_object()
        .ok_or_else(|| "held D10 telemetry policy must be an object".to_owned())?;
    expect_string(
        telemetry,
        "environment_snapshot_sha256",
        &digest,
        "D10 environment snapshot identity",
    )?;
    Ok((parsed, digest))
}

fn parse_implementations(
    value: &Value,
    policy_case: &Value,
    regression: &Map<String, Value>,
) -> BenchResult<Vec<Option<ImplementationPlan>>> {
    let values = value
        .as_array()
        .ok_or_else(|| "D10 collection implementations must be an array".to_owned())?;
    if values.len() != IMPLEMENTATIONS.len() {
        return Err("D10 collection implementation roster drifted".to_owned());
    }
    let policy_case = policy_case
        .as_object()
        .ok_or_else(|| "held D10 policy case must be an object".to_owned())?;
    let profile = get(policy_case, "profile", "held D10 policy case")?
        .as_object()
        .ok_or_else(|| "held D10 profile must be an object".to_owned())?;
    let vendor = get(policy_case, "vendor", "held D10 policy case")?
        .as_object()
        .ok_or_else(|| "held D10 vendor mapping must be an object".to_owned())?;
    let vendor_applicable = get_bool(vendor, "applicable", "held D10 vendor mapping")?;
    let expected_binaries = [
        Some(get_string(
            regression,
            "implementation_sha256",
            "held D10 regression reference",
        )?),
        Some(get_string(
            policy_case,
            "ferric_implementation_sha256",
            "held D10 policy case",
        )?),
        get(vendor, "implementation_sha256", "held D10 vendor mapping")?.as_str(),
    ];
    let expected_configs = [
        Some(get_string(
            regression,
            "config_sha256",
            "held D10 regression reference",
        )?),
        Some(get_string(profile, "sha256", "held D10 profile")?),
        get(vendor, "config_sha256", "held D10 vendor mapping")?.as_str(),
    ];
    let reference_protocol = get_string(
        regression,
        "measurement_protocol_sha256",
        "held D10 regression reference",
    )?;
    let mut implementations = Vec::with_capacity(IMPLEMENTATIONS.len());
    for ((((value, name), binary_sha256), config_sha256), index) in values
        .iter()
        .zip(IMPLEMENTATIONS)
        .zip(expected_binaries)
        .zip(expected_configs)
        .zip(0_usize..)
    {
        let applicable = *name != "vendor" || vendor_applicable;
        if !applicable {
            if !value.is_null() {
                return Err("inapplicable D10 vendor command must be null".to_owned());
            }
            implementations.push(None);
            continue;
        }
        let implementation = exact_object(
            value,
            &["command", "config", "implementation"],
            "D10 collection implementation",
        )?;
        expect_string(
            implementation,
            "implementation",
            name,
            "D10 collection implementation",
        )?;
        let config = get(implementation, "config", "D10 collection implementation")?.clone();
        let actual_config = sha256_identity(&encode_canonical_document(&config)?);
        if Some(actual_config.as_str()) != config_sha256 {
            return Err(format!("D10 {name} configuration SHA-256 drifted"));
        }
        let command = parse_command(
            get(implementation, "command", "D10 collection implementation")?,
            (index == 0).then_some(reference_protocol),
            "D10 measurement command",
        )?;
        if Some(command.binary.digest.as_str()) != binary_sha256 {
            return Err(format!("D10 {name} implementation binary drifted"));
        }
        implementations.push(Some(ImplementationPlan {
            command,
            config,
            config_sha256: actual_config,
            name: (*name).to_owned(),
        }));
    }
    Ok(implementations)
}

fn parse_command(
    value: &Value,
    expected_protocol: Option<&str>,
    description: &str,
) -> BenchResult<CommandSpec> {
    let command = exact_object(
        value,
        &[
            "arguments",
            "binary_path",
            "binary_sha256",
            "protocol_sha256",
        ],
        description,
    )?;
    let binary_sha256 = get_string(command, "binary_sha256", description)?;
    require_sha256(binary_sha256, description)?;
    let protocol_sha256 = get_string(command, "protocol_sha256", description)?;
    require_sha256(protocol_sha256, description)?;
    if expected_protocol.is_some_and(|expected| expected != protocol_sha256) {
        return Err(format!("{description} protocol identity drifted"));
    }
    let arguments = get(command, "arguments", description)?
        .as_array()
        .ok_or_else(|| format!("{description} arguments must be an array"))?;
    if arguments.len() > 128 {
        return Err(format!(
            "{description} argument count exceeds the admitted bound"
        ));
    }
    let arguments = arguments
        .iter()
        .map(|argument| {
            let argument = argument
                .as_str()
                .ok_or_else(|| format!("{description} arguments must be strings"))?;
            if !argument.is_ascii()
                || argument.len() > 64 * 1024
                || argument.as_bytes().contains(&0)
            {
                return Err(format!("{description} contains an invalid argument"));
            }
            Ok(argument.to_owned())
        })
        .collect::<BenchResult<Vec<_>>>()?;
    let path = Path::new(get_string(command, "binary_path", description)?);
    let binary = HeldExecutable::open(path, binary_sha256, description)?;
    Ok(CommandSpec {
        arguments,
        base_sha256: sha256_identity(&encode_canonical_document(value)?),
        binary,
        protocol_sha256: protocol_sha256.to_owned(),
    })
}

fn select_holdout_member(policy: &HeldValidatedPolicy, id: &str) -> BenchResult<Value> {
    require_safe_id(id, "D10 holdout member")?;
    let members = policy
        .document_value("holdout.json")?
        .get("members")
        .and_then(Value::as_array)
        .ok_or_else(|| "held D10 holdout members are unavailable".to_owned())?;
    let matches = members
        .iter()
        .filter(|member| member.get("id").and_then(Value::as_str) == Some(id))
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err("D10 collection holdout member is not uniquely admitted".to_owned());
    }
    Ok(matches[0].clone())
}

fn preflight_order_bindings(
    policy: &HeldValidatedPolicy,
    plan: &CollectionPlan,
) -> BenchResult<()> {
    let order_cases = policy
        .document_value("execution-order.json")?
        .get("cases")
        .and_then(Value::as_array)
        .ok_or_else(|| "held D10 execution-order cases are unavailable".to_owned())?;
    for ((case, order), (case_id, _)) in plan.cases.iter().zip(order_cases).zip(CASE_ROSTER) {
        if case.case_id != *case_id {
            return Err("D10 collection case order drifted".to_owned());
        }
        let mut warmup_projection = Vec::with_capacity(IMPLEMENTATIONS.len());
        let mut recorded_projection = Vec::with_capacity(IMPLEMENTATIONS.len());
        for (implementation, name) in case.implementations.iter().zip(IMPLEMENTATIONS) {
            let applicable = implementation.is_some();
            warmup_projection.push(json!({
                "holdout_member": if applicable { case.holdout_member.clone() } else { Value::Null },
                "implementation": name,
                "sample_ids": if applicable { sample_ids(case_id, name, "warmup", WARMUPS) } else { Vec::<String>::new() },
            }));
            recorded_projection.push(json!({
                "holdout_member": if applicable { case.holdout_member.clone() } else { Value::Null },
                "implementation": name,
                "sample_ids": if applicable { sample_ids(case_id, name, "recorded", RECORDED) } else { Vec::<String>::new() },
            }));
        }
        let order = order
            .as_object()
            .ok_or_else(|| "held D10 execution-order case must be an object".to_owned())?;
        expect_string(
            order,
            "warmup_order_sha256",
            &sha256_identity(&encode_canonical_document(&Value::Array(
                warmup_projection,
            ))?),
            "D10 warmup execution order",
        )?;
        expect_string(
            order,
            "recorded_order_sha256",
            &sha256_identity(&encode_canonical_document(&Value::Array(
                recorded_projection,
            ))?),
            "D10 recorded execution order",
        )?;
    }
    Ok(())
}

fn collect_observations(
    policy: &HeldValidatedPolicy,
    admission: &HeldBundle,
    plan: &mut CollectionPlan,
) -> BenchResult<Value> {
    let policy_value = policy
        .document_value("policy.json")?
        .as_object()
        .ok_or_else(|| "held D10 policy must be an object".to_owned())?;
    let policy_cases = get(policy_value, "cases", "held D10 policy")?
        .as_array()
        .ok_or_else(|| "held D10 policy cases must be an array".to_owned())?;
    let resources = policy
        .document_value("resource-inspection.json")?
        .get("cases")
        .and_then(Value::as_array)
        .ok_or_else(|| "held D10 resource policy cases are unavailable".to_owned())?;
    let regression = policy
        .document_value("regression-reference.json")?
        .as_object()
        .ok_or_else(|| "held D10 regression reference must be an object".to_owned())?;
    let tuning = policy
        .document_value("tuning.json")?
        .as_object()
        .ok_or_else(|| "held D10 tuning policy must be an object".to_owned())?;
    let bindings = execution_bindings(policy)?;
    let telemetry_protocols = telemetry_protocols(policy)?;
    let mut cases = Vec::with_capacity(CASE_ROSTER.len());
    for ((((case, policy_case), resource_policy), (_, family)), case_index) in plan
        .cases
        .iter_mut()
        .zip(policy_cases)
        .zip(resources)
        .zip(CASE_ROSTER)
        .zip(0_usize..)
    {
        let resource_policy = resource_policy
            .as_object()
            .ok_or_else(|| "held D10 resource case must be an object".to_owned())?;
        let resource_request = json!({
            "artifact_manifest_sha256": get_string(resource_policy, "artifact_manifest_sha256", "held D10 resource case")?,
            "case_id": case.case_id,
            "expected_resources": case.expected_resources,
            "expected_resources_sha256": case.expected_resources_sha256,
            "format": RESOURCE_REQUEST_FORMAT,
            "inspection_protocol_sha256": case.resource_command.protocol_sha256,
            "target": TARGET,
        });
        let resource_result = run_subprocess(
            &mut case.resource_command,
            &resource_request,
            &plan.environment,
            &plan.environment_sha256,
            plan.timeout,
            "D10 resource inspection",
        )?;
        let resource_observation = validate_resource_result(
            &resource_result,
            resource_policy,
            &case.expected_resources,
            &case.resource_command,
        )?;
        let policy_case = policy_case
            .as_object()
            .ok_or_else(|| "held D10 policy case must be an object".to_owned())?;
        let mut implementations = Vec::with_capacity(IMPLEMENTATIONS.len());
        for (implementation, name) in case.implementations.iter_mut().zip(IMPLEMENTATIONS) {
            let Some(implementation) = implementation else {
                implementations.push(inapplicable_vendor(&bindings));
                continue;
            };
            if implementation.name != *name {
                return Err("D10 collection implementation order drifted".to_owned());
            }
            let mut warmups = Vec::with_capacity(WARMUPS);
            for sequence in 0..WARMUPS {
                warmups.push(collect_sample(
                    &case.case_id,
                    family,
                    implementation,
                    &case.holdout_member,
                    "warmup",
                    sequence,
                    &plan.environment,
                    &plan.environment_sha256,
                    &telemetry_protocols,
                    plan.timeout,
                )?);
            }
            let mut recorded = Vec::with_capacity(RECORDED);
            for sequence in 0..RECORDED {
                recorded.push(collect_sample(
                    &case.case_id,
                    family,
                    implementation,
                    &case.holdout_member,
                    "recorded",
                    sequence,
                    &plan.environment,
                    &plan.environment_sha256,
                    &telemetry_protocols,
                    plan.timeout,
                )?);
            }
            let tuning_budget = match *name {
                "ferric-reference" => Value::Null,
                "ferric" => json!({
                    "budget": get_u64(tuning, "ferric_budget", "held D10 tuning policy")?,
                    "unit": get_string(tuning, "budget_unit", "held D10 tuning policy")?,
                }),
                "vendor" => json!({
                    "budget": get_u64(tuning, "vendor_budget", "held D10 tuning policy")?,
                    "unit": get_string(tuning, "budget_unit", "held D10 tuning policy")?,
                }),
                _ => return Err("unknown D10 implementation".to_owned()),
            };
            implementations.push(json!({
                "applicable": true,
                "base_command_sha256": implementation.command.base_sha256,
                "binary_sha256": implementation.command.binary.digest,
                "bindings": bindings,
                "command_protocol_sha256": implementation.command.protocol_sha256,
                "config_sha256": implementation.config_sha256,
                "environment_sha256": plan.environment_sha256,
                "holdout_member": case.holdout_member,
                "implementation": name,
                "implementation_sha256": implementation.command.binary.digest,
                "recorded": recorded,
                "regression_measurement_roster_sha256": if *name == "ferric-reference" {
                    get(regression, "measurement_roster_sha256", "held D10 regression reference")?.clone()
                } else {
                    Value::Null
                },
                "tuning_budget": tuning_budget,
                "warmups": warmups,
            }));
        }
        let profile = get(policy_case, "profile", "held D10 policy case")?
            .as_object()
            .ok_or_else(|| "held D10 profile must be an object".to_owned())?;
        let work_unit = get(policy_case, "work_unit", "held D10 policy case")?
            .as_object()
            .ok_or_else(|| "held D10 work unit must be an object".to_owned())?;
        cases.push(json!({
            "case_id": case.case_id,
            "implementations": implementations,
            "kernel_family": family,
            "profile_sha256": get_string(profile, "sha256", "held D10 profile")?,
            "resource_bindings": resource_policy,
            "resource_observation": resource_observation,
            "work_unit_semantics_sha256": get_string(work_unit, "semantics_sha256", "held D10 work unit")?,
        }));
        if case_index + 1 != cases.len() {
            return Err("D10 case collection order was not append-only".to_owned());
        }
    }
    let companion_sha256 = [
        ("calibration", "calibration.json"),
        ("execution-order", "execution-order.json"),
        ("holdout", "holdout.json"),
        ("regression-reference", "regression-reference.json"),
        ("resource-inspection", "resource-inspection.json"),
        ("telemetry", "telemetry.json"),
        ("timing", "timing.json"),
        ("tuning", "tuning.json"),
    ]
    .into_iter()
    .map(|(name, path)| {
        Ok((
            name.to_owned(),
            Value::String(sha256_identity(policy.document_bytes(path)?)),
        ))
    })
    .collect::<BenchResult<Map<_, _>>>()?;
    Ok(json!({
        "admission_sha256": sha256_identity(admission.document_bytes("admission.json")?),
        "authority": OBSERVATION_AUTHORITY,
        "cases": cases,
        "collection": {
            "collector_binary_sha256": plan.collector_binary.digest,
            "environment_sha256": plan.environment_sha256,
            "manifest_sha256": plan.manifest_sha256,
            "subprocess_contract": "held-elf-cleared-environment-canonical-stdin-canonical-stdout-empty-stderr-zero-exit-timeout-v1",
        },
        "companion_sha256": companion_sha256,
        "format": OBSERVATION_FORMAT,
        "policy_sha256": sha256_identity(policy.document_bytes("policy.json")?),
        "protocol_sha256": PROTOCOL_SHA256,
        "suite": "d10",
        "target": TARGET,
    }))
}

#[allow(clippy::too_many_arguments)]
fn collect_sample(
    case_id: &str,
    family: &str,
    implementation: &mut ImplementationPlan,
    holdout_member: &Value,
    phase: &str,
    sequence: usize,
    environment: &BTreeMap<String, String>,
    environment_sha256: &str,
    telemetry_protocols: &TelemetryProtocols,
    timeout: Duration,
) -> BenchResult<Value> {
    let sample_id = sample_id(case_id, &implementation.name, phase, sequence);
    let request = json!({
        "case_id": case_id,
        "config": implementation.config,
        "config_sha256": implementation.config_sha256,
        "format": SAMPLE_REQUEST_FORMAT,
        "holdout_member": holdout_member,
        "implementation": implementation.name,
        "kernel_family": family,
        "phase": phase,
        "sample_id": sample_id,
        "sequence": sequence,
        "target": TARGET,
        "timing": telemetry_protocols.timing,
    });
    let result = run_subprocess(
        &mut implementation.command,
        &request,
        environment,
        environment_sha256,
        timeout,
        "D10 measurement",
    )?;
    let (elapsed_ns, iterations, telemetry, timing) = validate_sample_result(
        &result.value,
        phase,
        telemetry_protocols,
        environment_sha256,
    )?;
    let mut sample = Map::new();
    sample.insert(
        "command_sha256".to_owned(),
        Value::String(result.command_sha256),
    );
    if let Some(elapsed_ns) = elapsed_ns {
        sample.insert("elapsed_ns".to_owned(), Value::from(elapsed_ns));
    }
    if let Some(iterations) = iterations {
        sample.insert("iterations".to_owned(), Value::from(iterations));
    }
    sample.insert(
        "runner_output_sha256".to_owned(),
        Value::String(result.output_sha256),
    );
    sample.insert("sample_id".to_owned(), Value::String(sample_id));
    sample.insert("sequence".to_owned(), Value::from(sequence));
    sample.insert(
        "subprocess".to_owned(),
        json!({"exit_code": 0, "stderr_sha256": EMPTY_SHA256}),
    );
    sample.insert("telemetry".to_owned(), telemetry);
    sample.insert("timing".to_owned(), timing);
    Ok(Value::Object(sample))
}

struct TelemetryProtocols {
    clock: String,
    errors: String,
    temperature: String,
    timing: Value,
}

fn telemetry_protocols(policy: &HeldValidatedPolicy) -> BenchResult<TelemetryProtocols> {
    let telemetry = policy
        .document_value("telemetry.json")?
        .as_object()
        .ok_or_else(|| "held D10 telemetry policy must be an object".to_owned())?;
    let timing = policy
        .document_value("timing.json")?
        .as_object()
        .ok_or_else(|| "held D10 timing policy must be an object".to_owned())?;
    Ok(TelemetryProtocols {
        clock: get_string(telemetry, "clock_trace_sha256", "held D10 telemetry policy")?.to_owned(),
        errors: get_string(telemetry, "error_trace_sha256", "held D10 telemetry policy")?
            .to_owned(),
        temperature: get_string(
            telemetry,
            "temperature_trace_sha256",
            "held D10 telemetry policy",
        )?
        .to_owned(),
        timing: json!({
            "clock_source_sha256": get_string(timing, "clock_source_sha256", "held D10 timing policy")?,
            "iteration_boundary_sha256": get_string(timing, "iteration_boundary_sha256", "held D10 timing policy")?,
            "synchronization_sha256": get_string(timing, "synchronization_sha256", "held D10 timing policy")?,
            "timer_overhead_sha256": get_string(timing, "timer_overhead_sha256", "held D10 timing policy")?,
        }),
    })
}

fn validate_sample_result(
    value: &Value,
    phase: &str,
    protocols: &TelemetryProtocols,
    environment_sha256: &str,
) -> BenchResult<(Option<u64>, Option<u64>, Value, Value)> {
    let result = exact_object(
        value,
        &["elapsed_ns", "format", "iterations", "telemetry", "timing"],
        "D10 sample result",
    )?;
    expect_string(result, "format", SAMPLE_RESULT_FORMAT, "D10 sample result")?;
    let elapsed_ns = optional_positive_u64(result, "elapsed_ns", "D10 sample result")?;
    let iterations = optional_positive_u64(result, "iterations", "D10 sample result")?;
    match phase {
        "warmup" if elapsed_ns.is_none() && iterations.is_none() => {}
        "recorded" if elapsed_ns.is_some() && iterations.is_some() => {}
        "warmup" => return Err("D10 warmup result must be untimed".to_owned()),
        "recorded" => {
            return Err("D10 recorded result must contain positive raw timing counters".to_owned())
        }
        _ => return Err("unknown D10 sample phase".to_owned()),
    }
    let telemetry = validate_telemetry_result(
        get(result, "telemetry", "D10 sample result")?,
        protocols,
        environment_sha256,
    )?;
    let timing = get(result, "timing", "D10 sample result")?;
    if timing != &protocols.timing {
        return Err("D10 sample timing identities drifted from policy".to_owned());
    }
    Ok((elapsed_ns, iterations, telemetry, timing.clone()))
}

fn validate_telemetry_result(
    value: &Value,
    protocols: &TelemetryProtocols,
    environment_sha256: &str,
) -> BenchResult<Value> {
    let telemetry = exact_object(
        value,
        &["clock", "environment_sha256", "errors", "temperature"],
        "D10 telemetry result",
    )?;
    expect_string(
        telemetry,
        "environment_sha256",
        environment_sha256,
        "D10 telemetry environment",
    )?;
    let clock = exact_object(
        get(telemetry, "clock", "D10 telemetry result")?,
        &["end_hz", "protocol_sha256", "start_hz"],
        "D10 clock telemetry",
    )?;
    expect_string(
        clock,
        "protocol_sha256",
        &protocols.clock,
        "D10 clock telemetry",
    )?;
    for field in ["start_hz", "end_hz"] {
        let value = get_u64(clock, field, "D10 clock telemetry")?;
        if value == 0 || value > 10_000_000_000 {
            return Err("D10 clock telemetry is outside the parseable physical bound".to_owned());
        }
    }
    let errors = exact_object(
        get(telemetry, "errors", "D10 telemetry result")?,
        &["events", "protocol_sha256"],
        "D10 error telemetry",
    )?;
    expect_string(
        errors,
        "protocol_sha256",
        &protocols.errors,
        "D10 error telemetry",
    )?;
    let events = get(errors, "events", "D10 error telemetry")?
        .as_array()
        .ok_or_else(|| "D10 error telemetry events must be an array".to_owned())?;
    if !events.is_empty() {
        return Err("D10 subprocess reported a telemetry error event".to_owned());
    }
    let temperature = exact_object(
        get(telemetry, "temperature", "D10 telemetry result")?,
        &["end_millicelsius", "protocol_sha256", "start_millicelsius"],
        "D10 temperature telemetry",
    )?;
    expect_string(
        temperature,
        "protocol_sha256",
        &protocols.temperature,
        "D10 temperature telemetry",
    )?;
    for field in ["start_millicelsius", "end_millicelsius"] {
        let value = get_u64(temperature, field, "D10 temperature telemetry")?;
        if value > 200_000 {
            return Err(
                "D10 temperature telemetry is outside the parseable physical bound".to_owned(),
            );
        }
    }
    Ok(value.clone())
}

fn validate_resource_result(
    result: &SubprocessResult,
    policy: &Map<String, Value>,
    expected_resources: &Value,
    command: &CommandSpec,
) -> BenchResult<Value> {
    let value = exact_object(
        &result.value,
        &[
            "artifact_manifest_sha256",
            "case_id",
            "format",
            "inspection_protocol_sha256",
            "observed_resources",
        ],
        "D10 resource result",
    )?;
    expect_string(
        value,
        "format",
        RESOURCE_RESULT_FORMAT,
        "D10 resource result",
    )?;
    for field in [
        "artifact_manifest_sha256",
        "case_id",
        "inspection_protocol_sha256",
    ] {
        expect_string(
            value,
            field,
            get_string(policy, field, "held D10 resource case")?,
            "D10 resource result",
        )?;
    }
    let observed = get(value, "observed_resources", "D10 resource result")?;
    validate_resource_map(observed, "D10 observed resources")?;
    if observed != expected_resources {
        return Err(
            "D10 observed resources differ from the exact policy-bound expectation".to_owned(),
        );
    }
    Ok(json!({
        "artifact_manifest_sha256": get_string(policy, "artifact_manifest_sha256", "held D10 resource case")?,
        "command_sha256": result.command_sha256,
        "expected_resources": expected_resources,
        "expected_resources_sha256": get_string(policy, "expected_resources_sha256", "held D10 resource case")?,
        "inspection_protocol_sha256": command.protocol_sha256,
        "inspector_binary_sha256": command.binary.digest,
        "observed_resources": observed,
        "runner_output_sha256": result.output_sha256,
        "subprocess": {"exit_code": 0, "stderr_sha256": EMPTY_SHA256},
    }))
}

fn validate_resource_map(value: &Value, description: &str) -> BenchResult<()> {
    let resources = value
        .as_object()
        .ok_or_else(|| format!("{description} must be an object"))?;
    if resources.is_empty() || resources.len() > 64 {
        return Err(format!(
            "{description} field count is outside the admitted bound"
        ));
    }
    for (name, value) in resources {
        require_safe_id(name, description)?;
        if value.as_u64().is_none() {
            return Err(format!("{description} values must be unsigned integers"));
        }
    }
    Ok(())
}

fn run_subprocess(
    command: &mut CommandSpec,
    request: &Value,
    environment: &BTreeMap<String, String>,
    environment_sha256: &str,
    timeout: Duration,
    description: &str,
) -> BenchResult<SubprocessResult> {
    command.binary.revalidate(description, false)?;
    let request_bytes = encode_canonical_document(request)?;
    let request_sha256 = sha256_identity(&request_bytes);
    let invocation = json!({
        "base_command_sha256": command.base_sha256,
        "environment_sha256": environment_sha256,
        "request_sha256": request_sha256,
    });
    let command_sha256 = sha256_identity(&encode_canonical_document(&invocation)?);
    let mut child = Command::new(command.binary.proc_path())
        .args(&command.arguments)
        .current_dir("/")
        .env_clear()
        .envs(environment)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("cannot launch {description}: {error}"))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| format!("cannot acquire {description} stdin"))?;
    stdin
        .write_all(&request_bytes)
        .map_err(|error| format!("cannot write canonical {description} request: {error}"))?;
    drop(stdin);
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| format!("cannot acquire {description} stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| format!("cannot acquire {description} stderr"))?;
    let stdout_reader = thread::spawn(move || read_bounded(stdout));
    let stderr_reader = thread::spawn(move || read_bounded(stderr));
    let start = Instant::now();
    let status = loop {
        match child
            .try_wait()
            .map_err(|error| format!("cannot poll {description}: {error}"))?
        {
            Some(status) => break status,
            None if start.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(format!("{description} timed out"));
            }
            None => thread::sleep(Duration::from_millis(1)),
        }
    };
    let stdout = stdout_reader
        .join()
        .map_err(|_| format!("{description} stdout reader panicked"))?
        .map_err(|error| format!("cannot read {description} stdout: {error}"))?;
    let stderr = stderr_reader
        .join()
        .map_err(|_| format!("{description} stderr reader panicked"))?
        .map_err(|error| format!("cannot read {description} stderr: {error}"))?;
    command.binary.revalidate(description, false)?;
    if status.code() != Some(0) {
        return Err(format!(
            "{description} did not exit normally with status zero"
        ));
    }
    if !stderr.is_empty() {
        return Err(format!("{description} emitted stderr"));
    }
    if stdout.is_empty() || !stdout.is_ascii() {
        return Err(format!("{description} stdout must be nonempty ASCII JSON"));
    }
    let value: Value = serde_json::from_slice(&stdout)
        .map_err(|error| format!("cannot parse {description} stdout: {error}"))?;
    if encode_canonical_document(&value)? != stdout {
        return Err(format!("{description} stdout must be canonical JSON"));
    }
    Ok(SubprocessResult {
        command_sha256,
        output_sha256: sha256_identity(&stdout),
        value,
    })
}

fn read_bounded(mut reader: impl Read) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .by_ref()
        .take(MAX_SUBPROCESS_BYTES.saturating_add(1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_SUBPROCESS_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "subprocess output exceeds bound",
        ));
    }
    Ok(bytes)
}

fn inapplicable_vendor(bindings: &Value) -> Value {
    json!({
        "applicable": false,
        "base_command_sha256": Value::Null,
        "binary_sha256": Value::Null,
        "bindings": bindings,
        "command_protocol_sha256": Value::Null,
        "config_sha256": Value::Null,
        "environment_sha256": Value::Null,
        "holdout_member": Value::Null,
        "implementation": "vendor",
        "implementation_sha256": Value::Null,
        "recorded": [],
        "regression_measurement_roster_sha256": Value::Null,
        "tuning_budget": Value::Null,
        "warmups": [],
    })
}

fn execution_bindings(policy: &HeldValidatedPolicy) -> BenchResult<Value> {
    let pairs = [
        ("calibration_sha256", "calibration.json"),
        ("execution_order_sha256", "execution-order.json"),
        ("holdout_sha256", "holdout.json"),
        ("regression_reference_sha256", "regression-reference.json"),
        ("resource_inspection_sha256", "resource-inspection.json"),
        ("telemetry_sha256", "telemetry.json"),
        ("timing_sha256", "timing.json"),
        ("tuning_sha256", "tuning.json"),
    ];
    let mut bindings = Map::new();
    for (field, path) in pairs {
        bindings.insert(
            field.to_owned(),
            Value::String(sha256_identity(policy.document_bytes(path)?)),
        );
    }
    Ok(Value::Object(bindings))
}

fn sample_id(case_id: &str, implementation: &str, phase: &str, sequence: usize) -> String {
    format!("{case_id}.{implementation}.{phase}.{sequence:02}")
}

fn sample_ids(case_id: &str, implementation: &str, phase: &str, count: usize) -> Vec<String> {
    (0..count)
        .map(|sequence| sample_id(case_id, implementation, phase, sequence))
        .collect()
}

fn exact_object<'a>(
    value: &'a Value,
    expected: &[&str],
    description: &str,
) -> BenchResult<&'a Map<String, Value>> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{description} must be an object"))?;
    if object.len() != expected.len() || !expected.iter().all(|field| object.contains_key(*field)) {
        return Err(format!("{description} fields drifted"));
    }
    Ok(object)
}

fn get<'a>(
    object: &'a Map<String, Value>,
    field: &str,
    description: &str,
) -> BenchResult<&'a Value> {
    object
        .get(field)
        .ok_or_else(|| format!("{description} is missing {field}"))
}

fn get_string<'a>(
    object: &'a Map<String, Value>,
    field: &str,
    description: &str,
) -> BenchResult<&'a str> {
    get(object, field, description)?
        .as_str()
        .ok_or_else(|| format!("{description} field {field} must be a string"))
}

fn get_u64(object: &Map<String, Value>, field: &str, description: &str) -> BenchResult<u64> {
    get(object, field, description)?
        .as_u64()
        .ok_or_else(|| format!("{description} field {field} must be an unsigned integer"))
}

fn get_bool(object: &Map<String, Value>, field: &str, description: &str) -> BenchResult<bool> {
    get(object, field, description)?
        .as_bool()
        .ok_or_else(|| format!("{description} field {field} must be boolean"))
}

fn expect_string(
    object: &Map<String, Value>,
    field: &str,
    expected: &str,
    description: &str,
) -> BenchResult<()> {
    if get_string(object, field, description)? != expected {
        return Err(format!("{description} drifted"));
    }
    Ok(())
}

fn optional_positive_u64(
    object: &Map<String, Value>,
    field: &str,
    description: &str,
) -> BenchResult<Option<u64>> {
    match get(object, field, description)? {
        Value::Null => Ok(None),
        Value::Number(value) => value
            .as_u64()
            .filter(|value| *value > 0)
            .map(Some)
            .ok_or_else(|| format!("{description} field {field} must be positive or null")),
        _ => Err(format!(
            "{description} field {field} must be positive or null"
        )),
    }
}

fn require_sha256(value: &str, description: &str) -> BenchResult<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || value.bytes().all(|byte| byte == value.as_bytes()[0])
    {
        return Err(format!("invalid {description} SHA-256"));
    }
    Ok(())
}

fn require_safe_id(value: &str, description: &str) -> BenchResult<()> {
    if value.is_empty()
        || value.len() > 160
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b".-".contains(&byte))
    {
        return Err(format!("invalid {description}: {value}"));
    }
    Ok(())
}

fn require_safe_input_path(path: &Path, description: &str) -> BenchResult<()> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
    {
        return Err(format!("{description} path is unsafe"));
    }
    Ok(())
}

fn same_snapshot(initial: &Stat, current: &Stat) -> bool {
    initial.st_dev == current.st_dev
        && initial.st_ino == current.st_ino
        && initial.st_mode == current.st_mode
        && initial.st_nlink == current.st_nlink
        && initial.st_uid == current.st_uid
        && initial.st_gid == current.st_gid
        && initial.st_size == current.st_size
        && initial.st_mtime == current.st_mtime
        && initial.st_mtime_nsec == current.st_mtime_nsec
        && initial.st_ctime == current.st_ctime
        && initial.st_ctime_nsec == current.st_ctime_nsec
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{d10_observations, d10_policy};
    use std::fs;

    struct Fixture {
        _temporary: d10_policy::tests::TestDirectory,
        admission: PathBuf,
        manifest: PathBuf,
        output: PathBuf,
        policy: PathBuf,
    }

    fn digest(label: &str) -> String {
        sha256_identity(label.as_bytes())
    }

    fn write_canonical(path: &Path, value: &Value) {
        fs::write(path, encode_canonical_document(value).unwrap()).unwrap();
    }

    fn read_value(path: &Path) -> Value {
        serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
    }

    fn python_binary() -> (PathBuf, String) {
        let path = fs::canonicalize("/usr/bin/python3").unwrap();
        let identity = sha256_identity(&fs::read(&path).unwrap());
        (path, identity)
    }

    fn awk_binary() -> (PathBuf, String) {
        let path = fs::canonicalize("/usr/bin/awk").unwrap();
        let identity = sha256_identity(&fs::read(&path).unwrap());
        (path, identity)
    }

    fn command(binary: &Path, binary_sha256: &str, protocol_sha256: &str, script: &str) -> Value {
        json!({
            "arguments": ["-c", script],
            "binary_path": binary,
            "binary_sha256": binary_sha256,
            "protocol_sha256": protocol_sha256,
        })
    }

    fn bind_companion(root: &Path, policy: &mut Value, name: &str, path: &str, value: &Value) {
        write_canonical(&root.join(path), value);
        let bytes = fs::read(root.join(path)).unwrap();
        policy["companions"][name]["bytes"] = json!(bytes.len());
        policy["companions"][name]["sha256"] = json!(sha256_identity(&bytes));
    }

    fn make_fixture() -> Fixture {
        let (temporary, policy_root, admission) = d10_policy::tests::fixture();
        let (python, python_sha256) = python_binary();
        let (awk, awk_sha256) = awk_binary();
        let current_exe = std::env::current_exe().unwrap();
        let collector_sha256 = sha256_identity(&fs::read(&current_exe).unwrap());
        let environment = json!({
            "format": ENVIRONMENT_FORMAT,
            "variables": {"FERRIC_D10_TEST": "1"},
        });
        let environment_sha256 = sha256_identity(&encode_canonical_document(&environment).unwrap());
        let mut policy = read_value(&policy_root.join("policy.json"));
        let mut telemetry = read_value(&policy_root.join("telemetry.json"));
        let timing = read_value(&policy_root.join("timing.json"));
        telemetry["environment_snapshot_sha256"] = json!(environment_sha256);
        bind_companion(
            &policy_root,
            &mut policy,
            "telemetry",
            "telemetry.json",
            &telemetry,
        );
        let reference_config = json!({"id": "reference-config"});
        let mut regression = read_value(&policy_root.join("regression-reference.json"));
        regression["implementation_sha256"] = json!(awk_sha256);
        regression["config_sha256"] = json!(sha256_identity(
            &encode_canonical_document(&reference_config).unwrap()
        ));
        bind_companion(
            &policy_root,
            &mut policy,
            "regression-reference",
            "regression-reference.json",
            &regression,
        );
        let holdout = read_value(&policy_root.join("holdout.json"));
        let holdout_member = holdout["members"][0].clone();
        let mut resources = read_value(&policy_root.join("resource-inspection.json"));
        let mut order = read_value(&policy_root.join("execution-order.json"));
        let mut manifest_cases = Vec::new();
        for (index, (case_id, _)) in CASE_ROSTER.iter().enumerate() {
            let ferric_config = json!({"case_id": case_id, "implementation": "ferric"});
            policy["cases"][index]["ferric_implementation_sha256"] = json!(awk_sha256);
            policy["cases"][index]["profile"]["sha256"] = json!(sha256_identity(
                &encode_canonical_document(&ferric_config).unwrap()
            ));
            let vendor_applicable = policy["cases"][index]["vendor"]["applicable"]
                .as_bool()
                .unwrap();
            let vendor_config = json!({"case_id": case_id, "implementation": "vendor"});
            if vendor_applicable {
                policy["cases"][index]["vendor"]["implementation_sha256"] = json!(awk_sha256);
                policy["cases"][index]["vendor"]["config_sha256"] = json!(sha256_identity(
                    &encode_canonical_document(&vendor_config).unwrap()
                ));
            }
            let expected_resources = json!({
                "group-segment-fixed-size": index,
                "private-segment-fixed-size": 0,
                "sgpr-count": 32,
                "vgpr-count": 16,
            });
            resources["cases"][index]["expected_resources_sha256"] = json!(sha256_identity(
                &encode_canonical_document(&expected_resources).unwrap()
            ));
            let warmup_projection = IMPLEMENTATIONS
                .iter()
                .map(|implementation| {
                    let applicable = *implementation != "vendor" || vendor_applicable;
                    json!({
                        "holdout_member": if applicable { holdout_member.clone() } else { Value::Null },
                        "implementation": implementation,
                        "sample_ids": if applicable { sample_ids(case_id, implementation, "warmup", WARMUPS) } else { Vec::<String>::new() },
                    })
                })
                .collect::<Vec<_>>();
            let recorded_projection = IMPLEMENTATIONS
                .iter()
                .map(|implementation| {
                    let applicable = *implementation != "vendor" || vendor_applicable;
                    json!({
                        "holdout_member": if applicable { holdout_member.clone() } else { Value::Null },
                        "implementation": implementation,
                        "sample_ids": if applicable { sample_ids(case_id, implementation, "recorded", RECORDED) } else { Vec::<String>::new() },
                    })
                })
                .collect::<Vec<_>>();
            order["cases"][index]["warmup_order_sha256"] = json!(sha256_identity(
                &encode_canonical_document(&Value::Array(warmup_projection)).unwrap()
            ));
            order["cases"][index]["recorded_order_sha256"] = json!(sha256_identity(
                &encode_canonical_document(&Value::Array(recorded_projection)).unwrap()
            ));
            let sample_output = |recorded: bool| {
                String::from_utf8(
                    encode_canonical_document(&json!({
                        "elapsed_ns": if recorded { Value::from(100_u64) } else { Value::Null },
                        "format": SAMPLE_RESULT_FORMAT,
                        "iterations": if recorded { Value::from(1_u64) } else { Value::Null },
                        "telemetry": {
                            "clock": {"end_hz": 1_500_000_000_u64, "protocol_sha256": telemetry["clock_trace_sha256"], "start_hz": 1_500_000_000_u64},
                            "environment_sha256": environment_sha256,
                            "errors": {"events": [], "protocol_sha256": telemetry["error_trace_sha256"]},
                            "temperature": {"end_millicelsius": 55_000, "protocol_sha256": telemetry["temperature_trace_sha256"], "start_millicelsius": 54_000},
                        },
                        "timing": {
                            "clock_source_sha256": timing["clock_source_sha256"],
                            "iteration_boundary_sha256": timing["iteration_boundary_sha256"],
                            "synchronization_sha256": timing["synchronization_sha256"],
                            "timer_overhead_sha256": timing["timer_overhead_sha256"],
                        },
                    }))
                    .unwrap(),
                )
                .unwrap()
            };
            let sample_arguments = json!([
                "-v",
                format!("warmup={}", sample_output(false)),
                "-v",
                format!("recorded={}", sample_output(true)),
                "/\"phase\": \"recorded\"/ { is_recorded=1 } END { printf \"%s\", is_recorded ? recorded : warmup }",
            ]);
            let resource_script = format!(
                "import json,sys\nr=json.load(sys.stdin)\no={{'artifact_manifest_sha256':r['artifact_manifest_sha256'],'case_id':r['case_id'],'format':'{RESOURCE_RESULT_FORMAT}','inspection_protocol_sha256':r['inspection_protocol_sha256'],'observed_resources':r['expected_resources']}}\njson.dump(o,sys.stdout,sort_keys=True,indent=2)\nsys.stdout.write('\\n')"
            );
            let implementations = vec![
                json!({
                    "command": {
                        "arguments": sample_arguments,
                        "binary_path": awk,
                        "binary_sha256": awk_sha256,
                        "protocol_sha256": regression["measurement_protocol_sha256"],
                    },
                    "config": reference_config,
                    "implementation": "ferric-reference",
                }),
                json!({
                    "command": {
                        "arguments": sample_arguments,
                        "binary_path": awk,
                        "binary_sha256": awk_sha256,
                        "protocol_sha256": digest(&format!("ferric-protocol:{case_id}")),
                    },
                    "config": ferric_config,
                    "implementation": "ferric",
                }),
                if vendor_applicable {
                    json!({
                        "command": {
                            "arguments": sample_arguments,
                            "binary_path": awk,
                            "binary_sha256": awk_sha256,
                            "protocol_sha256": digest(&format!("vendor-protocol:{case_id}")),
                        },
                        "config": vendor_config,
                        "implementation": "vendor",
                    })
                } else {
                    Value::Null
                },
            ];
            manifest_cases.push(json!({
                "case_id": case_id,
                "expected_resources": expected_resources,
                "holdout_member_id": holdout_member["id"],
                "implementations": implementations,
                "resource_command": command(
                    &python,
                    &python_sha256,
                    resources["cases"][index]["inspection_protocol_sha256"].as_str().unwrap(),
                    &resource_script,
                ),
            }));
        }
        bind_companion(
            &policy_root,
            &mut policy,
            "resource-inspection",
            "resource-inspection.json",
            &resources,
        );
        bind_companion(
            &policy_root,
            &mut policy,
            "execution-order",
            "execution-order.json",
            &order,
        );
        write_canonical(&policy_root.join("policy.json"), &policy);
        let admission_arguments = vec![
            OsString::from("admit-experiment-policy"),
            policy_root.as_os_str().to_os_string(),
            admission.as_os_str().to_os_string(),
        ];
        d10_policy::admit_experiment_policy(&admission_arguments).unwrap();
        let manifest = temporary.0.join("collection-manifest.json");
        write_canonical(
            &manifest,
            &json!({
                "authority": MANIFEST_AUTHORITY,
                "cases": manifest_cases,
                "collector_binary_path": current_exe,
                "collector_binary_sha256": collector_sha256,
                "environment": environment,
                "format": MANIFEST_FORMAT,
                "policy_sha256": sha256_identity(&fs::read(policy_root.join("policy.json")).unwrap()),
                "suite": "d10",
                "target": TARGET,
                "timeout_ms": 5_000,
            }),
        );
        let output = temporary.0.join("collected");
        Fixture {
            _temporary: temporary,
            admission,
            manifest,
            output,
            policy: policy_root,
        }
    }

    fn arguments(fixture: &Fixture) -> Vec<OsString> {
        vec![
            OsString::from(COMMAND),
            fixture.policy.as_os_str().to_os_string(),
            fixture.admission.as_os_str().to_os_string(),
            fixture.manifest.as_os_str().to_os_string(),
            fixture.output.as_os_str().to_os_string(),
        ]
    }

    #[test]
    fn exact_policy_collection_runs_real_commands_and_validates() {
        let fixture = make_fixture();
        collect_policy_observations(&arguments(&fixture)).unwrap();
        let observations = read_value(&fixture.output.join("observations.json"));
        assert_eq!(observations["cases"].as_array().unwrap().len(), 7);
        assert_eq!(
            observations["cases"][0]["implementations"][0]["warmups"]
                .as_array()
                .unwrap()
                .len(),
            WARMUPS
        );
        assert_eq!(
            observations["cases"][0]["implementations"][0]["recorded"]
                .as_array()
                .unwrap()
                .len(),
            RECORDED
        );
        assert_eq!(
            observations["cases"][0]["resource_observation"]["expected_resources"],
            observations["cases"][0]["resource_observation"]["observed_resources"]
        );
        let validated = fixture._temporary.0.join("validated");
        d10_observations::validate_policy_observations(&[
            OsString::from(d10_observations::COMMAND),
            fixture.policy.as_os_str().to_os_string(),
            fixture.admission.as_os_str().to_os_string(),
            fixture.output.as_os_str().to_os_string(),
            validated.as_os_str().to_os_string(),
        ])
        .unwrap();
        let result = read_value(&validated.join("validation.json"));
        assert_eq!(result["observation_counts_enforced"], true);
        assert_eq!(result["telemetry_resource_outputs_authenticated"], true);
        assert_eq!(result["qualification_evidence"], false);
    }

    #[test]
    fn manifest_order_drift_and_existing_output_fail_before_execution() {
        let fixture = make_fixture();
        let mut manifest = read_value(&fixture.manifest);
        manifest["cases"].as_array_mut().unwrap().swap(0, 1);
        write_canonical(&fixture.manifest, &manifest);
        assert!(collect_policy_observations(&arguments(&fixture)).is_err());
        assert!(!fixture.output.exists());

        let fixture = make_fixture();
        fs::create_dir(&fixture.output).unwrap();
        let error = collect_policy_observations(&arguments(&fixture)).unwrap_err();
        assert!(error.contains("already exists"));
        assert_eq!(fs::read_dir(&fixture.output).unwrap().count(), 0);
    }

    #[test]
    fn subprocess_nonzero_stderr_noncanonical_and_resource_mismatch_fail_closed() {
        let fixture = make_fixture();
        let mut manifest = read_value(&fixture.manifest);
        manifest["cases"][0]["resource_command"]["arguments"] =
            json!(["-c", "import sys;sys.exit(9)"]);
        write_canonical(&fixture.manifest, &manifest);
        let error = collect_policy_observations(&arguments(&fixture)).unwrap_err();
        assert!(error.contains("status zero"));
        assert!(!fixture.output.exists());

        let fixture = make_fixture();
        let mut manifest = read_value(&fixture.manifest);
        manifest["cases"][0]["resource_command"]["arguments"] =
            json!(["-c", "import sys;sys.stderr.write('bad')"]);
        write_canonical(&fixture.manifest, &manifest);
        let error = collect_policy_observations(&arguments(&fixture)).unwrap_err();
        assert!(error.contains("stderr"));
        assert!(!fixture.output.exists());

        let fixture = make_fixture();
        let mut manifest = read_value(&fixture.manifest);
        manifest["cases"][0]["resource_command"]["arguments"] = json!(["-c", "print('{}')"]);
        write_canonical(&fixture.manifest, &manifest);
        let error = collect_policy_observations(&arguments(&fixture)).unwrap_err();
        assert!(error.contains("canonical JSON") || error.contains("fields drifted"));
        assert!(!fixture.output.exists());

        let fixture = make_fixture();
        let mut manifest = read_value(&fixture.manifest);
        let code = "import json,sys\nr=json.load(sys.stdin)\no={'artifact_manifest_sha256':r['artifact_manifest_sha256'],'case_id':r['case_id'],'format':'FERRIC-M1-D10-RESOURCE-RESULT-V1','inspection_protocol_sha256':r['inspection_protocol_sha256'],'observed_resources':{'wrong':1}}\njson.dump(o,sys.stdout,sort_keys=True,indent=2)\nsys.stdout.write('\\n')";
        manifest["cases"][0]["resource_command"]["arguments"] = json!(["-c", code]);
        write_canonical(&fixture.manifest, &manifest);
        let error = collect_policy_observations(&arguments(&fixture)).unwrap_err();
        assert!(error.contains("differ from"));
        assert!(!fixture.output.exists());

        let fixture = make_fixture();
        let mut manifest = read_value(&fixture.manifest);
        manifest["timeout_ms"] = json!(1);
        manifest["cases"][0]["resource_command"]["arguments"] =
            json!(["-c", "import time;time.sleep(1)"]);
        write_canonical(&fixture.manifest, &manifest);
        let error = collect_policy_observations(&arguments(&fixture)).unwrap_err();
        assert!(error.contains("timed out"));
        assert!(!fixture.output.exists());

        let fixture = make_fixture();
        let mut manifest = read_value(&fixture.manifest);
        let command_arguments = manifest["cases"][0]["implementations"][0]["command"]["arguments"]
            .as_array_mut()
            .unwrap();
        for argument in command_arguments {
            if let Some(value) = argument.as_str() {
                for prefix in ["warmup=", "recorded="] {
                    if let Some(document) = value.strip_prefix(prefix) {
                        let mut result: Value = serde_json::from_str(document).unwrap();
                        result["telemetry"]["errors"]["events"] = json!(["fault"]);
                        let canonical =
                            String::from_utf8(encode_canonical_document(&result).unwrap()).unwrap();
                        *argument = Value::String(format!("{prefix}{canonical}"));
                        break;
                    }
                }
            }
        }
        write_canonical(&fixture.manifest, &manifest);
        let error = collect_policy_observations(&arguments(&fixture)).unwrap_err();
        assert!(error.contains("telemetry error event"), "{error}");
        assert!(!fixture.output.exists());
    }
}
