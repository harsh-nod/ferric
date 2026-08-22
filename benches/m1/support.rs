#![forbid(unsafe_code)]

//! Shared, fail-closed ingestion for the Ferric M1 benchmark suites.
//!
//! The binaries in this package produce authenticated run plans and validate
//! the shape and identity of externally collected records. They deliberately
//! do not manufacture measurements or decide an M1 evidence gate.

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
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

/// Loads and validates one canonical benchmark plan without granting evidence authority.
///
/// # Errors
///
/// Returns an error for an unreadable, noncanonical, malformed, or suite-mismatched plan.
pub fn load_benchmark_plan(suite: &Suite, path: &Path) -> BenchResult<(Value, Vec<u8>)> {
    validate_definition(suite)?;
    let bytes = read_regular(path, "benchmark plan")?;
    let value = parse_canonical(&bytes, "benchmark plan")?;
    validate_plan(suite, &value)?;
    Ok((value, bytes))
}

/// Loads one bounded canonical JSON document from a regular nonsymlink file.
///
/// # Errors
///
/// Returns an error when the file or its canonical JSON representation is invalid.
pub fn load_canonical_document(path: &Path, description: &str) -> BenchResult<(Value, Vec<u8>)> {
    let bytes = read_regular(path, description)?;
    let value = parse_canonical(&bytes, description)?;
    Ok((value, bytes))
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

/// Creates and synchronizes a protocol output without replacing an existing path.
///
/// # Errors
///
/// Returns an error when the path exists or the new file cannot be written and synchronized.
pub fn create_new_output(path: &Path, bytes: &[u8]) -> BenchResult<()> {
    write_new(path, bytes)
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
            if value == 0 && !metric.zero_allowed {
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

fn read_regular(path: &Path, description: &str) -> BenchResult<Vec<u8>> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("cannot inspect {description}: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!("{description} must be a regular nonsymlink file"));
    }
    let size = usize::try_from(metadata.len())
        .map_err(|_| format!("{description} is too large for this host"))?;
    if size == 0 || size > MAX_DOCUMENT_BYTES {
        return Err(format!("{description} size is outside the admitted bound"));
    }
    let bytes = fs::read(path).map_err(|error| format!("cannot read {description}: {error}"))?;
    if bytes.len() != size {
        return Err(format!("{description} changed during the bounded read"));
    }
    Ok(bytes)
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
}
