#![forbid(unsafe_code)]

//! Shared, fail-closed ingestion for the Ferric M1 benchmark suites.
//!
//! The binaries in this package produce authenticated run plans and validate
//! the shape and identity of externally collected records. They deliberately
//! do not manufacture measurements or decide an M1 evidence gate.

use rustix::fd::OwnedFd;
use rustix::fs::{fstat, openat2, FileType, Mode, OFlags, ResolveFlags, Stat, CWD};
use rustix::io::fcntl_dupfd_cloexec;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;
#[allow(unused_imports)]
use vstd::prelude::*;

const TARGET: &str = "gfx942:xnack-";
const INPUT_FORMAT: &str = "FERRIC-M1-BENCHMARK-INPUT-V1";
const PLAN_FORMAT: &str = "FERRIC-M1-BENCHMARK-PLAN-V1";
const RECORDS_FORMAT: &str = "FERRIC-M1-BENCHMARK-RECORDS-V1";
const TRANSCRIPT_FORMAT: &str = "FERRIC-M1-BENCHMARK-INGESTION-V1";
const DESCRIPTOR_FORMAT: &str = "FERRIC-M1-BENCHMARK-SUITE-V1";
const PLAN_AUTHORITY: &str = "benchmark-run-plan-only";
const TRANSCRIPT_AUTHORITY: &str = "checked-benchmark-record-structure-only";
const MAX_DOCUMENT_BYTES: usize = 8 * 1024 * 1024;
const MAX_CASES: usize = 512;
const MAX_SAMPLES: u64 = 1_000_000;

const COMMON_IDENTITIES: &[&str] = &[
    "benchmark-executable",
    "benchmark-protocol",
    "config",
    "dispatch-graph",
    "environment",
    "fe2o3-source-closure",
    "ferric-source-closure",
    "generated-plan",
    "model",
    "schedule-catalog",
    "tokenizer",
    "weights",
    "workload-roster",
];

const COMMON_RECORD_ATTRIBUTES: &[&str] = &["raw-record-sha256", "runner-transcript-sha256"];

verus! {

fn metric_value_is_admitted(zero_allowed: bool, value: u64) -> (admitted: bool)
    ensures admitted == (zero_allowed || value != 0),
{
    zero_allowed || value != 0
}

/// Returns exactly whether an observed comparison value is within its maximum.
#[must_use]
pub fn comparison_within_threshold(value: u64, maximum: u64) -> (within: bool)
    ensures within == (value <= maximum),
{
    value <= maximum
}

} // verus!

/// One integer-valued raw metric required from every case observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Metric {
    /// Stable metric identifier and unit.
    pub id: &'static str,
    /// Whether zero is a legal raw observation for this metric.
    pub zero_allowed: bool,
}

/// Static, source-reviewed declaration for one M1 benchmark suite.
#[derive(Clone, Copy, Debug)]
pub struct Suite {
    /// Short CLI and protocol name.
    pub name: &'static str,
    /// Requirement bound by this suite.
    pub obligation_id: &'static str,
    /// Path obligation bound by this suite.
    pub path_id: &'static str,
    /// Repository-relative source path named by the requirements manifest.
    pub source_path: &'static str,
    /// Exact roster of case kinds that a run plan must cover.
    pub case_kinds: &'static [&'static str],
    /// Suite-specific identities added to the common identity roster.
    pub extra_identities: &'static [&'static str],
    /// Exact raw metric roster for every case.
    pub metrics: &'static [Metric],
    /// Suite-specific companion identities required from every observation.
    pub extra_record_attributes: &'static [&'static str],
    /// Minimum untimed warmups declared for each case.
    pub minimum_warmups: u64,
    /// Minimum recorded observations declared for each case.
    pub minimum_recorded_samples: u64,
    /// Exact statement of what this suite does not establish.
    pub nonclaim: &'static str,
}

/// Failure returned by a benchmark protocol command.
pub type BenchResult<T> = Result<T, String>;

/// Stable descriptor identity used to reject hard-link aliases before comparison.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SecureFileIdentity {
    device: u64,
    inode: u64,
}

/// Strict no-follow descriptor root for one externally supplied manifest tree.
#[derive(Debug)]
pub struct SecureInputDirectory {
    descriptor: OwnedFd,
}

/// One regular input held open from validation through its final snapshot check.
#[derive(Debug)]
pub struct SecureInputFile {
    file: File,
    initial: Stat,
}

impl SecureInputDirectory {
    /// Reads one bounded canonical document beneath this descriptor root.
    ///
    /// # Errors
    ///
    /// Returns an error for unsafe traversal, a nonregular file, an invalid size,
    /// concurrent metadata drift, an incomplete read, or noncanonical JSON.
    pub fn read_canonical(
        &self,
        relative: &Path,
        description: &str,
    ) -> BenchResult<(Value, Vec<u8>, SecureFileIdentity)> {
        let (bytes, identity) = self.read_bounded_bytes(relative, description)?;
        let value = parse_canonical(&bytes, description)?;
        Ok((value, bytes, identity))
    }

    fn read_bounded_bytes(
        &self,
        relative: &Path,
        description: &str,
    ) -> BenchResult<(Vec<u8>, SecureFileIdentity)> {
        let mut input = self.open_bounded(relative, MAX_DOCUMENT_BYTES, description)?;
        let initial_len = input.length(description)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(initial_len.saturating_add(1))
            .map_err(|_| format!("cannot reserve {description} read buffer"))?;
        let read_result = Read::by_ref(&mut input)
            .take(MAX_DOCUMENT_BYTES.saturating_add(1) as u64)
            .read_to_end(&mut bytes);
        let snapshot_result = input.validate_snapshot(description);
        if let Err(error) = read_result {
            snapshot_result?;
            return Err(format!("cannot read {description}: {error}"));
        }
        snapshot_result?;
        if bytes.len() != initial_len {
            return Err(format!("{description} changed during the bounded read"));
        }
        Ok((bytes, input.identity()))
    }

    /// Opens one exact-size regular file beneath this descriptor root.
    ///
    /// # Errors
    ///
    /// Returns an error for unsafe traversal, filesystem type drift, or a size mismatch.
    pub fn open_exact(
        &self,
        relative: &Path,
        expected_bytes: u64,
        description: &str,
    ) -> BenchResult<SecureInputFile> {
        let input = self.open_file(relative, description)?;
        let actual = u64::try_from(input.initial.st_size)
            .map_err(|_| format!("{description} size is invalid"))?;
        if actual != expected_bytes {
            return Err(format!("{description} length drifted"));
        }
        Ok(input)
    }

    fn open_bounded(
        &self,
        relative: &Path,
        maximum_bytes: usize,
        description: &str,
    ) -> BenchResult<SecureInputFile> {
        let input = self.open_file(relative, description)?;
        let length = usize::try_from(input.initial.st_size)
            .map_err(|_| format!("{description} is too large for this host"))?;
        if length == 0 || length > maximum_bytes {
            return Err(format!("{description} size is outside the admitted bound"));
        }
        Ok(input)
    }

    fn open_file(&self, relative: &Path, description: &str) -> BenchResult<SecureInputFile> {
        require_relative(relative, description)?;
        let descriptor = openat2(
            &self.descriptor,
            relative,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )
        .map_err(|error| format!("cannot securely open {description}: {error}"))?;
        let initial = fstat(&descriptor)
            .map_err(|error| format!("cannot inspect opened {description}: {error}"))?;
        if FileType::from_raw_mode(initial.st_mode) != FileType::RegularFile {
            return Err(format!("{description} must be a regular file"));
        }
        if initial.st_nlink != 1 {
            return Err(format!(
                "{description} must have exactly one filesystem link"
            ));
        }
        Ok(SecureInputFile {
            file: File::from(descriptor),
            initial,
        })
    }
}

impl SecureInputFile {
    /// Returns the stable device/inode identity captured on descriptor open.
    #[must_use]
    pub const fn identity(&self) -> SecureFileIdentity {
        SecureFileIdentity {
            device: self.initial.st_dev,
            inode: self.initial.st_ino,
        }
    }

    /// Revalidates descriptor metadata after an exact read.
    ///
    /// # Errors
    ///
    /// Returns an error if any identity, type, size, link, or timestamp field changed.
    pub fn validate_snapshot(&self, description: &str) -> BenchResult<()> {
        let final_stat = fstat(&self.file)
            .map_err(|error| format!("cannot reinspect {description}: {error}"))?;
        if !same_file_snapshot(&self.initial, &final_stat) {
            return Err(format!("{description} changed while being read"));
        }
        Ok(())
    }

    fn length(&self, description: &str) -> BenchResult<usize> {
        usize::try_from(self.initial.st_size)
            .map_err(|_| format!("{description} is too large for this host"))
    }
}

impl Read for SecureInputFile {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.file.read(buffer)
    }
}

/// Loads and validates one canonical benchmark plan without granting evidence authority.
///
/// # Errors
///
/// Returns an error for an unreadable, noncanonical, malformed, or suite-mismatched plan.
pub fn load_benchmark_plan(suite: &Suite, path: &Path) -> BenchResult<(Value, Vec<u8>)> {
    validate_definition(suite)?;
    let (root, relative) = secure_parent(path, "benchmark plan")?;
    let (value, bytes, _) = root.read_canonical(&relative, "benchmark plan")?;
    validate_plan(suite, &value)?;
    Ok((value, bytes))
}

/// Opens a strict descriptor root and loads one canonical document from it.
///
/// # Errors
///
/// Returns an error when the file or its canonical JSON representation is invalid.
pub fn load_canonical_document(
    path: &Path,
    description: &str,
) -> BenchResult<(SecureInputDirectory, Value, Vec<u8>)> {
    let (root, relative) = secure_parent(path, description)?;
    let (value, bytes, _) = root.read_canonical(&relative, description)?;
    Ok((root, value, bytes))
}

/// Duplicates one already-open directory as a secure input root.
///
/// # Errors
///
/// Returns an error if the held descriptor is not a directory or its identity
/// changes while the close-on-exec duplicate is created.
pub fn duplicate_secure_input_directory(
    descriptor: &OwnedFd,
    description: &str,
) -> BenchResult<SecureInputDirectory> {
    let initial = fstat(descriptor)
        .map_err(|error| format!("cannot inspect held {description} directory: {error}"))?;
    if FileType::from_raw_mode(initial.st_mode) != FileType::Directory {
        return Err(format!("held {description} descriptor must be a directory"));
    }
    let duplicate = fcntl_dupfd_cloexec(descriptor, 0)
        .map_err(|error| format!("cannot duplicate held {description} directory: {error}"))?;
    let final_stat = fstat(&duplicate)
        .map_err(|error| format!("cannot inspect duplicated {description} directory: {error}"))?;
    if !same_file_snapshot(&initial, &final_stat) {
        return Err(format!(
            "held {description} directory changed during duplication"
        ));
    }
    Ok(SecureInputDirectory {
        descriptor: duplicate,
    })
}

/// Serializes a value with the benchmark protocol's canonical JSON encoding.
///
/// # Errors
///
/// Returns an error when `value` cannot be represented as JSON.
pub fn encode_canonical_document(value: &Value) -> BenchResult<Vec<u8>> {
    canonical_bytes(value)
}

/// Returns the lowercase SHA-256 identity of an immutable byte sequence.
#[must_use]
pub fn sha256_identity(bytes: &[u8]) -> String {
    sha256(bytes)
}

/// Runs the common CLI for one source-reviewed suite definition.
#[must_use]
pub fn main_for(suite: &'static Suite) -> ExitCode {
    match run(suite, env::args_os().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("FAIL: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(suite: &Suite, arguments: Vec<OsString>) -> BenchResult<()> {
    validate_definition(suite)?;
    match arguments.as_slice() {
        [command] if command == "describe" => write_stdout(&descriptor(suite)),
        [command, input, output] if command == "plan" => {
            let input_path = Path::new(input);
            let input_bytes = read_regular(input_path, "benchmark input")?;
            let input_value = parse_canonical(&input_bytes, "benchmark input")?;
            let plan = build_plan(suite, &input_value, &input_bytes)?;
            write_new(Path::new(output), &canonical_bytes(&plan)?)
        }
        [command, plan, records, output] if command == "validate" => {
            let plan_bytes = read_regular(Path::new(plan), "benchmark plan")?;
            let plan_value = parse_canonical(&plan_bytes, "benchmark plan")?;
            validate_plan(suite, &plan_value)?;
            let records_bytes = read_regular(Path::new(records), "benchmark records")?;
            let records_value = parse_canonical(&records_bytes, "benchmark records")?;
            let transcript = validate_records(
                suite,
                &plan_value,
                &plan_bytes,
                &records_value,
                &records_bytes,
            )?;
            write_new(Path::new(output), &canonical_bytes(&transcript)?)
        }
        _ => Err(format!(
            "usage: ferric-m1-{} describe | plan INPUT OUTPUT | validate PLAN RECORDS OUTPUT",
            suite.name
        )),
    }
}

fn descriptor(suite: &Suite) -> Value {
    let metrics = suite
        .metrics
        .iter()
        .map(|metric| {
            json!({
                "id": metric.id,
                "zero_allowed": metric.zero_allowed,
            })
        })
        .collect::<Vec<_>>();
    json!({
        "authority": PLAN_AUTHORITY,
        "case_kinds": suite.case_kinds,
        "format": DESCRIPTOR_FORMAT,
        "minimum_recorded_samples": suite.minimum_recorded_samples,
        "minimum_warmups": suite.minimum_warmups,
        "nonclaim": suite.nonclaim,
        "obligation_id": suite.obligation_id,
        "path_id": suite.path_id,
        "raw_metrics": metrics,
        "required_identities": required_identities(suite),
        "required_record_attributes": required_record_attributes(suite),
        "source_path": suite.source_path,
        "suite": suite.name,
        "target": TARGET,
    })
}

fn build_plan(suite: &Suite, input: &Value, input_bytes: &[u8]) -> BenchResult<Value> {
    let object = exact_object(
        input,
        &["cases", "format", "identities", "suite", "target"],
        "benchmark input",
    )?;
    expect_string(object, "format", "benchmark input format", INPUT_FORMAT)?;
    expect_string(object, "suite", "benchmark input suite", suite.name)?;
    expect_string(object, "target", "benchmark input target", TARGET)?;
    validate_identities(suite, get(object, "identities", "benchmark input")?)?;
    validate_cases(suite, get(object, "cases", "benchmark input")?)?;
    Ok(json!({
        "authority": PLAN_AUTHORITY,
        "cases": get(object, "cases", "benchmark input")?,
        "format": PLAN_FORMAT,
        "identities": get(object, "identities", "benchmark input")?,
        "input_sha256": sha256(input_bytes),
        "milestone": "M1",
        "nonclaim": suite.nonclaim,
        "obligation_id": suite.obligation_id,
        "path_id": suite.path_id,
        "source_path": suite.source_path,
        "suite": suite.name,
        "target": TARGET,
    }))
}

fn validate_plan(suite: &Suite, plan: &Value) -> BenchResult<()> {
    let object = exact_object(
        plan,
        &[
            "authority",
            "cases",
            "format",
            "identities",
            "input_sha256",
            "milestone",
            "nonclaim",
            "obligation_id",
            "path_id",
            "source_path",
            "suite",
            "target",
        ],
        "benchmark plan",
    )?;
    expect_string(object, "authority", "plan authority", PLAN_AUTHORITY)?;
    expect_string(object, "format", "plan format", PLAN_FORMAT)?;
    expect_string(object, "milestone", "plan milestone", "M1")?;
    expect_string(object, "nonclaim", "plan nonclaim", suite.nonclaim)?;
    expect_string(
        object,
        "obligation_id",
        "plan obligation",
        suite.obligation_id,
    )?;
    expect_string(object, "path_id", "plan path", suite.path_id)?;
    expect_string(object, "source_path", "plan source", suite.source_path)?;
    expect_string(object, "suite", "plan suite", suite.name)?;
    expect_string(object, "target", "plan target", TARGET)?;
    require_sha256(get_string(object, "input_sha256", "plan input identity")?)?;
    validate_identities(suite, get(object, "identities", "benchmark plan")?)?;
    validate_cases(suite, get(object, "cases", "benchmark plan")?).map(drop)
}

fn validate_identities(suite: &Suite, value: &Value) -> BenchResult<()> {
    let object = value
        .as_object()
        .ok_or_else(|| "benchmark identities must be an object".to_owned())?;
    let expected = required_identities(suite);
    exact_keys(object, &expected, "benchmark identities")?;
    for (name, value) in object {
        let identity = value
            .as_str()
            .ok_or_else(|| format!("benchmark identity must be a string: {name}"))?;
        require_sha256(identity)?;
    }
    Ok(())
}

fn validate_cases(suite: &Suite, value: &Value) -> BenchResult<BTreeMap<String, String>> {
    let cases = value
        .as_array()
        .ok_or_else(|| "benchmark cases must be an array".to_owned())?;
    if cases.is_empty() || cases.len() > MAX_CASES {
        return Err("benchmark case count is outside the admitted bound".to_owned());
    }
    let mut prior: Option<&str> = None;
    let mut by_id = BTreeMap::new();
    let mut kinds = BTreeSet::new();
    for case in cases {
        let object = exact_object(
            case,
            &["id", "input_sha256", "kind", "workload_sha256"],
            "benchmark case",
        )?;
        let id = get_string(object, "id", "benchmark case id")?;
        require_safe_id(id, "benchmark case id")?;
        if prior.is_some_and(|previous| previous >= id) {
            return Err("benchmark cases must be uniquely sorted by id".to_owned());
        }
        prior = Some(id);
        let kind = get_string(object, "kind", "benchmark case kind")?;
        if !suite.case_kinds.contains(&kind) {
            return Err(format!("benchmark case has unknown kind: {kind}"));
        }
        require_sha256(get_string(object, "input_sha256", "case input identity")?)?;
        require_sha256(get_string(
            object,
            "workload_sha256",
            "case workload identity",
        )?)?;
        by_id.insert(id.to_owned(), kind.to_owned());
        kinds.insert(kind);
    }
    if kinds != suite.case_kinds.iter().copied().collect() {
        return Err("benchmark plan does not cover the exact required case-kind roster".to_owned());
    }
    Ok(by_id)
}

fn validate_records(
    suite: &Suite,
    plan: &Value,
    plan_bytes: &[u8],
    records: &Value,
    records_bytes: &[u8],
) -> BenchResult<Value> {
    let plan_object = plan
        .as_object()
        .ok_or_else(|| "benchmark plan must be an object".to_owned())?;
    let cases = validate_cases(suite, get(plan_object, "cases", "benchmark plan")?)?;
    let object = exact_object(
        records,
        &["format", "observations", "plan_sha256", "suite"],
        "benchmark records",
    )?;
    expect_string(object, "format", "records format", RECORDS_FORMAT)?;
    expect_string(object, "suite", "records suite", suite.name)?;
    let plan_sha256 = sha256(plan_bytes);
    expect_string(object, "plan_sha256", "records plan identity", &plan_sha256)?;
    let observations = get(object, "observations", "benchmark records")?
        .as_array()
        .ok_or_else(|| "benchmark observations must be an array".to_owned())?;
    if observations.len() != cases.len() {
        return Err("benchmark observations do not cover the exact plan".to_owned());
    }

    let mut prior: Option<&str> = None;
    let mut observed_ids = BTreeSet::new();
    let mut total_recorded = 0_u64;
    let mut total_warmups = 0_u64;
    for observation in observations {
        let object = exact_object(
            observation,
            &[
                "attributes",
                "case_id",
                "kind",
                "measurements",
                "recorded_samples",
                "status",
                "warmups",
            ],
            "benchmark observation",
        )?;
        let case_id = get_string(object, "case_id", "observation case id")?;
        if prior.is_some_and(|previous| previous >= case_id) {
            return Err("benchmark observations must be uniquely sorted by case id".to_owned());
        }
        prior = Some(case_id);
        let expected_kind = cases
            .get(case_id)
            .ok_or_else(|| format!("observation names an unknown case: {case_id}"))?;
        expect_string(object, "kind", "observation kind", expected_kind)?;
        expect_string(object, "status", "observation status", "completed")?;
        let warmups = bounded_count(
            get(object, "warmups", "benchmark observation")?,
            "observation warmups",
        )?;
        let recorded = bounded_count(
            get(object, "recorded_samples", "benchmark observation")?,
            "observation recorded samples",
        )?;
        if warmups < suite.minimum_warmups
            || recorded < suite.minimum_recorded_samples
            || recorded == 0
        {
            return Err(format!(
                "observation does not meet the declared sample floor: {case_id}"
            ));
        }
        validate_attributes(suite, get(object, "attributes", "benchmark observation")?)?;
        validate_measurements(
            suite,
            get(object, "measurements", "benchmark observation")?,
            recorded,
        )?;
        total_warmups = total_warmups
            .checked_add(warmups)
            .ok_or_else(|| "aggregate warmup count overflowed".to_owned())?;
        total_recorded = total_recorded
            .checked_add(recorded)
            .ok_or_else(|| "aggregate sample count overflowed".to_owned())?;
        observed_ids.insert(case_id);
    }
    if observed_ids != cases.keys().map(String::as_str).collect() {
        return Err("benchmark observation roster drifted from the plan".to_owned());
    }

    Ok(json!({
        "authority": TRANSCRIPT_AUTHORITY,
        "case_count": observations.len(),
        "format": TRANSCRIPT_FORMAT,
        "nonclaim": suite.nonclaim,
        "obligation_id": suite.obligation_id,
        "path_id": suite.path_id,
        "plan_sha256": plan_sha256,
        "recorded_samples": total_recorded,
        "records_sha256": sha256(records_bytes),
        "status": "RECORDS_ACCEPTED",
        "suite": suite.name,
        "target": TARGET,
        "warmups": total_warmups,
    }))
}

fn validate_attributes(suite: &Suite, value: &Value) -> BenchResult<()> {
    let object = value
        .as_object()
        .ok_or_else(|| "observation attributes must be an object".to_owned())?;
    let expected = required_record_attributes(suite);
    exact_keys(object, &expected, "observation attributes")?;
    for (name, value) in object {
        let identity = value
            .as_str()
            .ok_or_else(|| format!("observation attribute must be a string: {name}"))?;
        require_sha256(identity)?;
    }
    Ok(())
}

fn validate_measurements(suite: &Suite, value: &Value, samples: u64) -> BenchResult<()> {
    let object = value
        .as_object()
        .ok_or_else(|| "observation measurements must be an object".to_owned())?;
    let expected = suite
        .metrics
        .iter()
        .map(|metric| metric.id)
        .collect::<Vec<_>>();
    exact_keys(object, &expected, "observation measurements")?;
    let sample_count = usize::try_from(samples)
        .map_err(|_| "recorded sample count does not fit this host".to_owned())?;
    for metric in suite.metrics {
        let values = get(object, metric.id, "observation measurements")?
            .as_array()
            .ok_or_else(|| format!("metric must be an integer array: {}", metric.id))?;
        if values.len() != sample_count {
            return Err(format!("metric sample count drifted: {}", metric.id));
        }
        for value in values {
            let value = value
                .as_u64()
                .ok_or_else(|| format!("metric is not an unsigned integer: {}", metric.id))?;
            if !metric_value_is_admitted(metric.zero_allowed, value) {
                return Err(format!("metric must be positive: {}", metric.id));
            }
        }
    }
    Ok(())
}

fn validate_definition(suite: &Suite) -> BenchResult<()> {
    require_safe_id(suite.name, "suite name")?;
    require_safe_id(suite.obligation_id, "suite obligation")?;
    require_safe_id(suite.path_id, "suite path id")?;
    if suite.case_kinds.is_empty() || suite.metrics.is_empty() {
        return Err("suite definition has an empty required roster".to_owned());
    }
    unique_safe(suite.case_kinds, "suite case kind")?;
    unique_safe(suite.extra_identities, "suite identity")?;
    unique_safe(suite.extra_record_attributes, "record attribute")?;
    unique_safe(
        &suite
            .metrics
            .iter()
            .map(|metric| metric.id)
            .collect::<Vec<_>>(),
        "suite metric",
    )?;
    let identities = required_identities(suite);
    if identities.len() != identities.iter().collect::<BTreeSet<_>>().len() {
        return Err("suite identity rosters overlap".to_owned());
    }
    let attributes = required_record_attributes(suite);
    if attributes.len() != attributes.iter().collect::<BTreeSet<_>>().len() {
        return Err("suite record-attribute rosters overlap".to_owned());
    }
    Ok(())
}

fn unique_safe(values: &[&str], description: &str) -> BenchResult<()> {
    let mut unique = BTreeSet::new();
    for value in values {
        require_safe_id(value, description)?;
        if !unique.insert(*value) {
            return Err(format!("duplicate {description}: {value}"));
        }
    }
    Ok(())
}

fn required_identities(suite: &Suite) -> Vec<&str> {
    let mut values = COMMON_IDENTITIES.to_vec();
    values.extend_from_slice(suite.extra_identities);
    values.sort_unstable();
    values
}

fn required_record_attributes(suite: &Suite) -> Vec<&str> {
    let mut values = COMMON_RECORD_ATTRIBUTES.to_vec();
    values.extend_from_slice(suite.extra_record_attributes);
    values.sort_unstable();
    values
}

fn exact_object<'a>(
    value: &'a Value,
    expected: &[&str],
    description: &str,
) -> BenchResult<&'a Map<String, Value>> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{description} must be an object"))?;
    exact_keys(object, expected, description)?;
    Ok(object)
}

fn exact_keys(
    object: &Map<String, Value>,
    expected: &[&str],
    description: &str,
) -> BenchResult<()> {
    if object.len() != expected.len() || !expected.iter().all(|key| object.contains_key(*key)) {
        return Err(format!("{description} fields drifted"));
    }
    Ok(())
}

fn get<'a>(object: &'a Map<String, Value>, key: &str, description: &str) -> BenchResult<&'a Value> {
    object
        .get(key)
        .ok_or_else(|| format!("{description} is missing {key}"))
}

fn get_string<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    description: &str,
) -> BenchResult<&'a str> {
    get(object, key, description)?
        .as_str()
        .ok_or_else(|| format!("{description} must be a string: {key}"))
}

fn expect_string(
    object: &Map<String, Value>,
    key: &str,
    description: &str,
    expected: &str,
) -> BenchResult<()> {
    if get_string(object, key, description)? != expected {
        return Err(format!("{description} drifted"));
    }
    Ok(())
}

fn bounded_count(value: &Value, description: &str) -> BenchResult<u64> {
    let count = value
        .as_u64()
        .ok_or_else(|| format!("{description} must be an unsigned integer"))?;
    if count > MAX_SAMPLES {
        return Err(format!("{description} exceeds the admitted bound"));
    }
    Ok(count)
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

fn require_sha256(value: &str) -> BenchResult<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || value.bytes().all(|byte| byte == value.as_bytes()[0])
    {
        return Err("invalid SHA-256 identity".to_owned());
    }
    Ok(())
}

fn secure_parent(path: &Path, description: &str) -> BenchResult<(SecureInputDirectory, PathBuf)> {
    let relative = path
        .file_name()
        .map(PathBuf::from)
        .ok_or_else(|| format!("{description} path has no file name"))?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let descriptor = openat2(
        CWD,
        parent,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )
    .map_err(|error| format!("cannot securely open {description} parent: {error}"))?;
    Ok((SecureInputDirectory { descriptor }, relative))
}

fn read_regular(path: &Path, description: &str) -> BenchResult<Vec<u8>> {
    let (root, relative) = secure_parent(path, description)?;
    root.read_bounded_bytes(&relative, description)
        .map(|(bytes, _)| bytes)
}

fn require_relative(path: &Path, description: &str) -> BenchResult<()> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || !path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(format!("{description} path must be a safe relative path"));
    }
    Ok(())
}

fn same_file_snapshot(initial: &Stat, final_stat: &Stat) -> bool {
    initial.st_dev == final_stat.st_dev
        && initial.st_ino == final_stat.st_ino
        && initial.st_mode == final_stat.st_mode
        && initial.st_nlink == final_stat.st_nlink
        && initial.st_size == final_stat.st_size
        && initial.st_mtime == final_stat.st_mtime
        && initial.st_mtime_nsec == final_stat.st_mtime_nsec
        && initial.st_ctime == final_stat.st_ctime
        && initial.st_ctime_nsec == final_stat.st_ctime_nsec
}

fn parse_canonical(bytes: &[u8], description: &str) -> BenchResult<Value> {
    if !bytes.is_ascii() {
        return Err(format!("{description} must be ASCII JSON"));
    }
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|error| format!("cannot parse {description}: {error}"))?;
    if canonical_bytes(&value)? != bytes {
        return Err(format!("{description} is not canonical JSON"));
    }
    Ok(value)
}

fn canonical_bytes(value: &Value) -> BenchResult<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("cannot serialize canonical JSON: {error}"))?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn sha256(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

fn write_new(path: &Path, bytes: &[u8]) -> BenchResult<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("cannot create output without replacement: {error}"))?;
    file.write_all(bytes)
        .map_err(|error| format!("cannot write output: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("cannot synchronize output: {error}"))
}

fn write_stdout(value: &Value) -> BenchResult<()> {
    io::stdout()
        .write_all(&canonical_bytes(value)?)
        .map_err(|error| format!("cannot write descriptor: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_NONCE: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let nonce = TEST_NONCE.fetch_add(1, Ordering::Relaxed);
            let path = env::temp_dir().join(format!(
                "ferric-m1-support-test.{}.{nonce}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    const METRICS: &[Metric] = &[
        Metric {
            id: "failures",
            zero_allowed: true,
        },
        Metric {
            id: "throughput-units-per-second",
            zero_allowed: false,
        },
    ];
    const TEST_SUITE: Suite = Suite {
        name: "test-suite",
        obligation_id: "m1.r99",
        path_id: "test-bench",
        source_path: "benches/m1/test.rs",
        case_kinds: &["first", "second"],
        extra_identities: &["test-reference"],
        metrics: METRICS,
        extra_record_attributes: &["test-output-sha256"],
        minimum_warmups: 1,
        minimum_recorded_samples: 2,
        nonclaim: "Test fixtures are not benchmark evidence and close no obligation.",
    };

    fn digest(label: &str) -> String {
        sha256(label.as_bytes())
    }

    fn test_input() -> Value {
        let identities = required_identities(&TEST_SUITE)
            .into_iter()
            .map(|id| (id.to_owned(), Value::String(digest(id))))
            .collect::<Map<_, _>>();
        json!({
            "cases": [
                {
                    "id": "case.001",
                    "input_sha256": digest("input one"),
                    "kind": "first",
                    "workload_sha256": digest("workload one"),
                },
                {
                    "id": "case.002",
                    "input_sha256": digest("input two"),
                    "kind": "second",
                    "workload_sha256": digest("workload two"),
                },
            ],
            "format": INPUT_FORMAT,
            "identities": identities,
            "suite": TEST_SUITE.name,
            "target": TARGET,
        })
    }

    fn test_records(plan_bytes: &[u8]) -> Value {
        let attributes = required_record_attributes(&TEST_SUITE)
            .into_iter()
            .map(|id| (id.to_owned(), Value::String(digest(id))))
            .collect::<Map<_, _>>();
        let observation = |case_id: &str, kind: &str| {
            json!({
                "attributes": attributes,
                "case_id": case_id,
                "kind": kind,
                "measurements": {
                    "failures": [0, 0],
                    "throughput-units-per-second": [10, 11],
                },
                "recorded_samples": 2,
                "status": "completed",
                "warmups": 1,
            })
        };
        json!({
            "format": RECORDS_FORMAT,
            "observations": [
                observation("case.001", "first"),
                observation("case.002", "second"),
            ],
            "plan_sha256": sha256(plan_bytes),
            "suite": TEST_SUITE.name,
        })
    }

    #[test]
    fn plan_and_records_are_identity_bound() {
        let input = test_input();
        let input_bytes = canonical_bytes(&input).unwrap();
        let plan = build_plan(&TEST_SUITE, &input, &input_bytes).unwrap();
        validate_plan(&TEST_SUITE, &plan).unwrap();
        let plan_bytes = canonical_bytes(&plan).unwrap();
        let records = test_records(&plan_bytes);
        let records_bytes = canonical_bytes(&records).unwrap();
        let transcript =
            validate_records(&TEST_SUITE, &plan, &plan_bytes, &records, &records_bytes).unwrap();
        assert_eq!(transcript["status"], "RECORDS_ACCEPTED");
        assert_eq!(transcript["recorded_samples"], 4);
    }

    #[test]
    fn missing_kind_and_reordered_cases_fail_closed() {
        let mut missing = test_input();
        missing["cases"].as_array_mut().unwrap().pop();
        assert!(validate_cases(&TEST_SUITE, &missing["cases"]).is_err());

        let mut reordered = test_input();
        reordered["cases"].as_array_mut().unwrap().reverse();
        assert!(validate_cases(&TEST_SUITE, &reordered["cases"]).is_err());
    }

    #[test]
    fn record_identity_status_and_sample_floor_fail_closed() {
        let input = test_input();
        let input_bytes = canonical_bytes(&input).unwrap();
        let plan = build_plan(&TEST_SUITE, &input, &input_bytes).unwrap();
        let plan_bytes = canonical_bytes(&plan).unwrap();

        let mut records = test_records(&plan_bytes);
        records["plan_sha256"] = Value::String(digest("wrong plan"));
        let records_bytes = canonical_bytes(&records).unwrap();
        assert!(
            validate_records(&TEST_SUITE, &plan, &plan_bytes, &records, &records_bytes,).is_err()
        );

        let mut records = test_records(&plan_bytes);
        records["observations"][0]["status"] = Value::String("failed".to_owned());
        let records_bytes = canonical_bytes(&records).unwrap();
        assert!(
            validate_records(&TEST_SUITE, &plan, &plan_bytes, &records, &records_bytes,).is_err()
        );

        let mut records = test_records(&plan_bytes);
        records["observations"][0]["recorded_samples"] = Value::from(1);
        records["observations"][0]["measurements"]["failures"] = json!([0]);
        records["observations"][0]["measurements"]["throughput-units-per-second"] = json!([10]);
        let records_bytes = canonical_bytes(&records).unwrap();
        assert!(
            validate_records(&TEST_SUITE, &plan, &plan_bytes, &records, &records_bytes,).is_err()
        );
    }

    #[test]
    fn canonical_parser_rejects_duplicate_or_noncanonical_input() {
        assert!(parse_canonical(br#"{"b":1,"a":2}"#, "test input").is_err());
        assert!(parse_canonical(br#"{"a":1,"a":2}"#, "test input").is_err());
        assert!(require_sha256(&"0".repeat(64)).is_err());
    }

    #[test]
    fn metric_value_admission_matches_zero_policy() {
        assert!(!metric_value_is_admitted(false, 0));
        assert!(metric_value_is_admitted(false, 1));
        assert!(metric_value_is_admitted(true, 0));
    }

    #[test]
    fn secure_reads_reject_intermediate_and_final_symlinks() {
        let temporary = TestDirectory::new();
        let actual_directory = temporary.0.join("real");
        fs::create_dir(&actual_directory).unwrap();
        let document = canonical_bytes(&json!({"value": 1})).unwrap();
        fs::write(actual_directory.join("document.json"), document).unwrap();
        symlink(&actual_directory, temporary.0.join("directory-link")).unwrap();
        symlink(
            actual_directory.join("document.json"),
            temporary.0.join("document-link.json"),
        )
        .unwrap();

        let (root, _) = secure_parent(&temporary.0.join("root.json"), "test root").unwrap();
        assert!(root
            .read_canonical(Path::new("directory-link/document.json"), "linked document")
            .is_err());
        assert!(root
            .read_canonical(Path::new("document-link.json"), "linked document")
            .is_err());
        assert!(secure_parent(
            &temporary.0.join("directory-link/document.json"),
            "linked parent",
        )
        .is_err());
    }

    #[test]
    fn secure_file_snapshot_rejects_post_open_drift() {
        let temporary = TestDirectory::new();
        let path = temporary.0.join("payload.bin");
        fs::write(&path, b"original").unwrap();
        let (root, relative) = secure_parent(&path, "test payload").unwrap();
        let mut input = root.open_exact(&relative, 8, "test payload").unwrap();

        fs::write(&path, b"short").unwrap();
        let mut bytes = Vec::new();
        input.read_to_end(&mut bytes).unwrap();
        assert!(input.validate_snapshot("test payload").is_err());
    }
}
