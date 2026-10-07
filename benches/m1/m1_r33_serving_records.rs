//! Exact post-observation record construction for one M1 serving comparison.
//!
//! This checker recomputes arithmetic over externally collected raw window
//! events. It authenticates declarations and byte identities, not the truth
//! of a measurement or the hardware that allegedly produced it.

use ferric_m1_benchmarks::{
    encode_canonical_document, load_canonical_document_held, sha256_identity, BenchResult,
    SecureInputDirectory, SecureInputFile,
};
use rustix::fd::OwnedFd;
use rustix::fs::{
    fstat, fsync, openat2, renameat_with, unlinkat, AtFlags, FileType, Mode, OFlags, RenameFlags,
    ResolveFlags, Stat, CWD,
};
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

pub(super) const COMMAND: &str = "validate-comparison-observations";

const POLICY_FORMAT: &str = "FERRIC-M1-R33-SERVING-COMPARISON-POLICY-V3";
const OBSERVATIONS_FORMAT: &str = "FERRIC-M1-R33-SERVING-COMPARISON-OBSERVATIONS-V3";
const RECORD_FORMAT: &str = "FERRIC-M1-R33-SERVING-COMPARISON-RECORD-V3";
const POLICY_AUTHORITY: &str = "external-pre-observation-serving-comparison-policy-v3-only";
const OBSERVATIONS_AUTHORITY: &str = "externally-collected-serving-request-events-v3-only";
const RECORD_AUTHORITY: &str = "ferric-checked-serving-comparison-v3-structure-and-arithmetic-only";
const PROTOCOL_AUTHORITY: &str = "ferric-m1-r33-serving-comparison-protocol-v3-only";
const PROTOCOL_FORMAT: &str = "FERRIC-M1-R33-SERVING-COMPARISON-PROTOCOL-V3";
const STATUS: &str = "PARTIAL_NON_EVIDENCE";
const TARGET: &str = "gfx942:xnack-";
const WARMUPS_PER_START: usize = 10;
const RECORDED_PER_START: usize = 10;
const SERVER_STARTS: usize = 3;
const BOOTSTRAP_RESAMPLES: usize = 10_000;
const BOOTSTRAP_CONFIDENCE_PPM: u64 = 950_000;
const BOOTSTRAP_LOWER_RANK: usize = 250;
const BOOTSTRAP_UPPER_RANK: usize = 9_750;
const COMPETITIVENESS_GATE_PPM: u64 = 950_000;
const RATE_SCALE: u128 = 1_000_000_000;
const PPM_SCALE: u128 = 1_000_000;
const ENGINES: &[&str] = &["ferric", "vllm", "sglang"];
const PROTOCOL_SHA256: &str = "2f6a720b2512623332e26f77d4bbaeb42b289ab946dcbf0a56a3e3eca2aca662";
const NONCLAIM: &str = "This V3 record checks one exact externally frozen Ferric, tuned vLLM, and tuned SGLang comparison roster, retains an ordered event for every successful request, requires identical per-request input/output work across engines in each aligned window, and recomputes end-to-end latency, TTFT, TPOT, token throughputs, nearest-rank percentiles, exact medians, fastest-baseline selection, and a deterministic paired-percentile-bootstrap 95% throughput interval. TPOT is floor((terminal-first-token)/(output-tokens-1)) nanoseconds per output token and therefore requires at least two output tokens per request. The record enforces declared p99 timing SLOs and a 0.95 throughput lower confidence bound. It does not validate the external plan, versions, sources, tuning choices, budget, SLO choice, event truth, server freshness, hardware behavior, numerical correctness, or independent reproduction; it is not qualification evidence and does not close m1.r33 or M1.";
const TIMING_CLOCK: &str = "monotonic-raw-nanoseconds";
const TIMING_BOUNDARIES: &str = "request-arrival-to-first-output-token-observed-to-terminal-event";
const TIMING_PERCENTILE_METHOD: &str = "nearest-rank-ceil-percent-times-population";
const TIMING_SOURCE: &str = "record-recomputed-from-retained-per-request-events";

const POLICY_KEYS: &[&str] = &[
    "authority",
    "engine_order",
    "format",
    "implementations",
    "nonclaim",
    "obligation_id",
    "p99_end_to_end_slo_ns",
    "p99_tpot_slo_ns_per_output_token",
    "p99_ttft_slo_ns",
    "plan",
    "protocol_sha256",
    "sample_roster",
    "status",
    "target",
];
const PLAN_KEYS: &[&str] = &[
    "arrival_trace_sha256",
    "benchmark_executable_sha256",
    "benchmark_plan_sha256",
    "environment_sha256",
    "fe2o3_source_closure_sha256",
    "ferric_source_closure_sha256",
    "generated_plan_sha256",
    "model_sha256",
    "output_limits_sha256",
    "sampling_seed_sha256",
    "schedule_sha256",
    "server_start_roster_sha256",
    "tokenizer_sha256",
    "weights_sha256",
    "workload_sha256",
];
const IMPLEMENTATION_KEYS: &[&str] = &[
    "config_sha256",
    "id",
    "implementation_sha256",
    "protocol_sha256",
    "source_sha256",
    "tuning_budget_sha256",
    "tuning_sha256",
    "version",
];
const ROSTER_KEYS: &[&str] = &[
    "recorded_windows_per_start",
    "server_starts",
    "warmup_windows_per_start",
];
const OBSERVATION_KEYS: &[&str] = &[
    "authority",
    "engine_order",
    "format",
    "implementations",
    "nonclaim",
    "obligation_id",
    "plan",
    "policy_sha256",
    "rows",
    "status",
    "target",
];
const ROW_KEYS: &[&str] = &[
    "engine_order",
    "faults",
    "id",
    "ordinal",
    "phase",
    "server_start",
    "status",
    "values",
    "window",
];
const VALUE_KEYS: &[&str] = &[
    "duration_ns",
    "failed_requests",
    "input_tokens",
    "output_tokens",
    "p50_end_to_end_latency_ns",
    "p50_tpot_ns_per_output_token",
    "p50_ttft_ns",
    "p90_end_to_end_latency_ns",
    "p90_tpot_ns_per_output_token",
    "p90_ttft_ns",
    "p99_end_to_end_latency_ns",
    "p99_tpot_ns_per_output_token",
    "p99_ttft_ns",
    "request_events",
    "successful_requests",
    "total_tokens",
];
const REQUEST_EVENT_KEYS: &[&str] = &[
    "arrival_offset_ns",
    "first_token_offset_ns",
    "input_tokens",
    "output_tokens",
    "request_ordinal",
    "terminal_offset_ns",
];

#[derive(Clone, Debug, Eq, PartialEq)]
struct Rational {
    denominator: u128,
    numerator: u128,
}

impl Rational {
    fn new(numerator: u128, denominator: u128) -> Self {
        let divisor = gcd(numerator, denominator);
        Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        }
    }

    fn as_json(&self) -> Value {
        json!({
            "denominator": self.denominator.to_string(),
            "numerator": self.numerator.to_string(),
        })
    }
}

#[derive(Debug)]
struct Inputs {
    observations_bytes: Vec<u8>,
    observations_file: SecureInputFile,
    observations_name: PathBuf,
    observations_root: SecureInputDirectory,
    policy_bytes: Vec<u8>,
    policy_file: SecureInputFile,
    policy_name: PathBuf,
    policy_root: SecureInputDirectory,
}

impl Inputs {
    fn revalidate(&self) -> BenchResult<()> {
        self.policy_root.validate_binding(
            &self.policy_name,
            &self.policy_file,
            "serving comparison policy",
        )?;
        self.observations_root.validate_binding(
            &self.observations_name,
            &self.observations_file,
            "serving comparison observations",
        )
    }
}

#[derive(Debug)]
struct EngineSamples {
    end_to_end: PercentileSamples,
    input_throughput: Vec<u64>,
    output_throughput: Vec<u64>,
    tpot: PercentileSamples,
    total_throughput: Vec<u64>,
    ttft: PercentileSamples,
}

#[derive(Debug)]
struct PercentileSamples {
    p50: Vec<u64>,
    p90: Vec<u64>,
    p99: Vec<u64>,
}

impl PercentileSamples {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            p50: Vec::with_capacity(capacity),
            p90: Vec::with_capacity(capacity),
            p99: Vec::with_capacity(capacity),
        }
    }

    fn push(&mut self, percentiles: TimingPercentiles) {
        self.p50.push(percentiles.p50);
        self.p90.push(percentiles.p90);
        self.p99.push(percentiles.p99);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TimingPercentiles {
    p50: u64,
    p90: u64,
    p99: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RequestWork {
    input_tokens: u64,
    output_tokens: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct WindowWork {
    input_tokens: u64,
    output_tokens: u64,
    successful_requests: u64,
    total_tokens: u64,
}

#[derive(Debug, Eq, PartialEq)]
struct BootstrapInterval {
    lower_ppm: u64,
    seed_sha256: String,
    upper_ppm: u64,
}

#[derive(Debug)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    fn index(&mut self, bound: usize) -> BenchResult<usize> {
        let bound = u64::try_from(bound)
            .map_err(|_| "serving bootstrap population does not fit u64".to_owned())?;
        if bound == 0 {
            return Err("serving bootstrap population is empty".to_owned());
        }
        let rejection_floor = bound.wrapping_neg() % bound;
        loop {
            let value = self.next();
            if value >= rejection_floor {
                return usize::try_from(value % bound)
                    .map_err(|_| "serving bootstrap index does not fit usize".to_owned());
            }
        }
    }
}

struct RecordPublication {
    armed: bool,
    expected_sha256: String,
    file: File,
    output_name: OsString,
    parent: OwnedFd,
    settled: Stat,
    staging_name: OsString,
}

pub(super) fn main_for_arguments(arguments: Vec<OsString>) -> ExitCode {
    match run(arguments) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("FAIL: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: Vec<OsString>) -> BenchResult<()> {
    let [policy, observations, output] = arguments.as_slice() else {
        return Err(format!(
            "usage: ferric-m1-serving {COMMAND} POLICY OBSERVATIONS OUTPUT-RECORD"
        ));
    };
    let policy_path = Path::new(policy);
    let observations_path = Path::new(observations);
    let output_path = Path::new(output);
    if same_path(policy_path, observations_path)
        || same_path(policy_path, output_path)
        || same_path(observations_path, output_path)
    {
        return Err("serving comparison inputs and output must be distinct paths".to_owned());
    }
    let record = validate(policy_path, observations_path)?;
    write_new(output_path, &encode_canonical_document(&record)?)
}

fn validate(policy_path: &Path, observations_path: &Path) -> BenchResult<Value> {
    require_protocol()?;
    let (policy_root, policy, policy_bytes, policy_file) =
        load_canonical_document_held(policy_path, "serving comparison policy")?;
    let (observations_root, observations, observations_bytes, observations_file) =
        load_canonical_document_held(observations_path, "serving comparison observations")?;
    let inputs = Inputs {
        observations_bytes,
        observations_file,
        observations_name: file_name(observations_path, "serving comparison observations")?,
        observations_root,
        policy_bytes,
        policy_file,
        policy_name: file_name(policy_path, "serving comparison policy")?,
        policy_root,
    };
    if inputs.policy_file.identity() == inputs.observations_file.identity() {
        return Err("serving comparison policy and observations must not alias".to_owned());
    }
    validate_policy(&policy)?;
    let policy_sha256 = sha256_identity(&inputs.policy_bytes);
    let samples = validate_observations(&observations, &policy, &policy_sha256)?;
    let record = build_record(&policy, &observations, &inputs, samples)?;
    inputs.revalidate()?;
    Ok(record)
}

fn validate_policy(policy: &Value) -> BenchResult<()> {
    let object = exact_object(policy, POLICY_KEYS, "serving comparison policy")?;
    expect_string(
        object,
        "authority",
        POLICY_AUTHORITY,
        "serving comparison policy",
    )?;
    expect_string(object, "format", POLICY_FORMAT, "serving comparison policy")?;
    expect_string(object, "nonclaim", NONCLAIM, "serving comparison policy")?;
    expect_string(
        object,
        "obligation_id",
        "m1.r33",
        "serving comparison policy",
    )?;
    expect_string(
        object,
        "protocol_sha256",
        PROTOCOL_SHA256,
        "serving comparison policy",
    )?;
    expect_string(
        object,
        "status",
        "pre-observation",
        "serving comparison policy",
    )?;
    expect_string(object, "target", TARGET, "serving comparison policy")?;
    validate_engine_order(field(object, "engine_order", "serving comparison policy")?)?;
    for (key, description) in [
        ("p99_end_to_end_slo_ns", "p99 end-to-end SLO"),
        ("p99_ttft_slo_ns", "p99 TTFT SLO"),
        ("p99_tpot_slo_ns_per_output_token", "p99 TPOT SLO"),
    ] {
        let slo = positive_u64(
            field(object, key, "serving comparison policy")?,
            description,
        )?;
        if slo == u64::MAX {
            return Err(format!(
                "serving comparison {description} is outside the admitted bound"
            ));
        }
    }
    validate_plan(field(object, "plan", "serving comparison policy")?)?;
    validate_implementations(field(
        object,
        "implementations",
        "serving comparison policy",
    )?)?;
    validate_roster(field(object, "sample_roster", "serving comparison policy")?)
}

pub(super) fn validate_policy_for_collection(policy: &Value) -> BenchResult<()> {
    require_protocol()?;
    validate_policy(policy)
}

pub(super) fn collected_observations(
    policy: &Value,
    policy_sha256: &str,
    rows: Vec<Value>,
) -> BenchResult<Value> {
    let policy_object = policy
        .as_object()
        .ok_or_else(|| "serving comparison policy must be an object".to_owned())?;
    let observations = json!({
        "authority": OBSERVATIONS_AUTHORITY,
        "engine_order": field(policy_object, "engine_order", "serving comparison policy")?,
        "format": OBSERVATIONS_FORMAT,
        "implementations": field(policy_object, "implementations", "serving comparison policy")?,
        "nonclaim": NONCLAIM,
        "obligation_id": "m1.r33",
        "plan": field(policy_object, "plan", "serving comparison policy")?,
        "policy_sha256": policy_sha256,
        "rows": rows,
        "status": "externally-collected",
        "target": TARGET,
    });
    let _ = validate_observations(&observations, policy, policy_sha256)?;
    Ok(observations)
}

pub(super) fn publish_collected_observations(path: &Path, bytes: &[u8]) -> BenchResult<()> {
    write_new(path, bytes)
}

pub(super) fn require_collected_output_absent(path: &Path) -> BenchResult<()> {
    let output_name = safe_output_name(path)?;
    let parent_path = admitted_output_parent(path)?;
    let parent = openat2(
        CWD,
        &parent_path,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )
    .map_err(|error| format!("cannot securely open serving observation parent: {error}"))?;
    require_absent_at(&parent, &output_name)
}

fn validate_plan(value: &Value) -> BenchResult<()> {
    let object = exact_object(value, PLAN_KEYS, "serving comparison plan binding")?;
    for key in PLAN_KEYS {
        sha_value(
            field(object, key, "serving comparison plan binding")?,
            &format!("serving comparison plan {key}"),
        )?;
    }
    Ok(())
}

fn validate_implementations(value: &Value) -> BenchResult<()> {
    let implementations = value
        .as_array()
        .ok_or_else(|| "serving comparison implementations must be an array".to_owned())?;
    if implementations.len() != ENGINES.len() {
        return Err("serving comparison implementation roster is incomplete".to_owned());
    }
    let mut tuning_budget: Option<&str> = None;
    for (implementation, expected_id) in implementations.iter().zip(ENGINES) {
        let object = exact_object(
            implementation,
            IMPLEMENTATION_KEYS,
            "serving comparison implementation",
        )?;
        expect_string(
            object,
            "id",
            expected_id,
            "serving comparison implementation",
        )?;
        for key in IMPLEMENTATION_KEYS {
            if *key != "id" && *key != "version" {
                sha_value(
                    field(object, key, "serving comparison implementation")?,
                    &format!("{expected_id} {key}"),
                )?;
            }
        }
        let version = safe_string(
            field(object, "version", "serving comparison implementation")?,
            &format!("{expected_id} version"),
        )?;
        if version.len() > 160 {
            return Err(format!("{expected_id} version is too long"));
        }
        let budget = field(
            object,
            "tuning_budget_sha256",
            "serving comparison implementation",
        )?
        .as_str()
        .ok_or_else(|| "serving comparison tuning budget must be a string".to_owned())?;
        if tuning_budget.is_some_and(|expected| expected != budget) {
            return Err("Ferric, vLLM, and SGLang tuning-budget identities differ".to_owned());
        }
        tuning_budget = Some(budget);
    }
    Ok(())
}

fn validate_roster(value: &Value) -> BenchResult<()> {
    let object = exact_object(value, ROSTER_KEYS, "serving comparison sample roster")?;
    expect_u64(
        object,
        "recorded_windows_per_start",
        RECORDED_PER_START as u64,
        "serving comparison sample roster",
    )?;
    expect_u64(
        object,
        "server_starts",
        SERVER_STARTS as u64,
        "serving comparison sample roster",
    )?;
    expect_u64(
        object,
        "warmup_windows_per_start",
        WARMUPS_PER_START as u64,
        "serving comparison sample roster",
    )
}

fn validate_observations(
    observations: &Value,
    policy: &Value,
    policy_sha256: &str,
) -> BenchResult<Vec<EngineSamples>> {
    let object = exact_object(
        observations,
        OBSERVATION_KEYS,
        "serving comparison observations",
    )?;
    expect_string(
        object,
        "authority",
        OBSERVATIONS_AUTHORITY,
        "serving comparison observations",
    )?;
    expect_string(
        object,
        "format",
        OBSERVATIONS_FORMAT,
        "serving comparison observations",
    )?;
    expect_string(
        object,
        "nonclaim",
        NONCLAIM,
        "serving comparison observations",
    )?;
    expect_string(
        object,
        "obligation_id",
        "m1.r33",
        "serving comparison observations",
    )?;
    expect_string(
        object,
        "policy_sha256",
        policy_sha256,
        "serving comparison observations",
    )?;
    expect_string(
        object,
        "status",
        "externally-collected",
        "serving comparison observations",
    )?;
    expect_string(object, "target", TARGET, "serving comparison observations")?;

    let policy_object = policy
        .as_object()
        .ok_or_else(|| "serving comparison policy must be an object".to_owned())?;
    for key in ["engine_order", "implementations", "plan"] {
        if field(object, key, "serving comparison observations")?
            != field(policy_object, key, "serving comparison policy")?
        {
            return Err(format!(
                "serving comparison observation {key} binding drifted"
            ));
        }
    }

    let rows = field(object, "rows", "serving comparison observations")?
        .as_array()
        .ok_or_else(|| "serving comparison rows must be an array".to_owned())?;
    let per_start = WARMUPS_PER_START + RECORDED_PER_START;
    if rows.len() != SERVER_STARTS * per_start {
        return Err("serving comparison row roster is incomplete".to_owned());
    }
    let sample_capacity = SERVER_STARTS * RECORDED_PER_START;
    let mut samples = ENGINES
        .iter()
        .map(|_| EngineSamples {
            end_to_end: PercentileSamples::with_capacity(sample_capacity),
            input_throughput: Vec::with_capacity(sample_capacity),
            output_throughput: Vec::with_capacity(sample_capacity),
            tpot: PercentileSamples::with_capacity(sample_capacity),
            total_throughput: Vec::with_capacity(sample_capacity),
            ttft: PercentileSamples::with_capacity(sample_capacity),
        })
        .collect::<Vec<_>>();
    let mut unique = BTreeSet::new();
    let mut ordinal = 0_usize;
    for start in 0..SERVER_STARTS {
        for (phase, count) in [
            ("warmup", WARMUPS_PER_START),
            ("recorded", RECORDED_PER_START),
        ] {
            for window in 0..count {
                validate_row(
                    &rows[ordinal],
                    start,
                    phase,
                    window,
                    ordinal,
                    &mut unique,
                    &mut samples,
                )?;
                ordinal += 1;
            }
        }
    }
    Ok(samples)
}

fn validate_row(
    value: &Value,
    start: usize,
    phase: &str,
    window: usize,
    ordinal: usize,
    unique: &mut BTreeSet<String>,
    samples: &mut [EngineSamples],
) -> BenchResult<()> {
    let object = exact_object(value, ROW_KEYS, "serving comparison row")?;
    let expected_id = format!("start-{start}.{phase}-{window:02}");
    expect_string(object, "id", &expected_id, "serving comparison row")?;
    if !unique.insert(expected_id.clone()) {
        return Err("serving comparison row IDs are duplicated".to_owned());
    }
    expect_string(object, "phase", phase, "serving comparison row")?;
    expect_string(object, "status", "passed", "serving comparison row")?;
    expect_u64(object, "ordinal", ordinal as u64, "serving comparison row")?;
    expect_u64(
        object,
        "server_start",
        start as u64,
        "serving comparison row",
    )?;
    expect_u64(object, "window", window as u64, "serving comparison row")?;
    let faults = field(object, "faults", "serving comparison row")?
        .as_array()
        .ok_or_else(|| "serving comparison faults must be an array".to_owned())?;
    if !faults.is_empty() {
        return Err(format!(
            "serving comparison row retained a fault: {expected_id}"
        ));
    }
    let expected_order = (0..ENGINES.len())
        .map(|offset| ENGINES[(ordinal + offset) % ENGINES.len()])
        .collect::<Vec<_>>();
    if field(object, "engine_order", "serving comparison row")? != &json!(expected_order) {
        return Err(format!(
            "serving comparison engine order drifted: {expected_id}"
        ));
    }
    let values = exact_object(
        field(object, "values", "serving comparison row")?,
        ENGINES,
        "serving comparison row values",
    )?;
    let mut expected_work: Option<WindowWork> = None;
    let mut expected_request_work: Option<Vec<RequestWork>> = None;
    for (index, engine) in ENGINES.iter().enumerate() {
        let metrics = exact_object(
            field(values, engine, "serving comparison row values")?,
            VALUE_KEYS,
            &format!("serving comparison {engine} window counters"),
        )?;
        let duration = positive_u64(
            field(metrics, "duration_ns", "serving comparison window counters")?,
            "serving comparison duration",
        )?;
        let failed = unsigned_u64(
            field(
                metrics,
                "failed_requests",
                "serving comparison window counters",
            )?,
            "serving comparison failed requests",
        )?;
        let input_tokens = positive_u64(
            field(
                metrics,
                "input_tokens",
                "serving comparison window counters",
            )?,
            "serving comparison input tokens",
        )?;
        let output_tokens = positive_u64(
            field(
                metrics,
                "output_tokens",
                "serving comparison window counters",
            )?,
            "serving comparison output tokens",
        )?;
        let successful_requests = positive_u64(
            field(
                metrics,
                "successful_requests",
                "serving comparison window counters",
            )?,
            "serving comparison successful requests",
        )?;
        let total_tokens = positive_u64(
            field(
                metrics,
                "total_tokens",
                "serving comparison window counters",
            )?,
            "serving comparison total tokens",
        )?;
        if failed != 0 {
            return Err(format!(
                "serving comparison row has a failed request: {expected_id}"
            ));
        }
        let checked_total = input_tokens.checked_add(output_tokens).ok_or_else(|| {
            format!(
                "serving comparison input-plus-output token count overflowed: {expected_id}/{engine}"
            )
        })?;
        if total_tokens != checked_total {
            return Err(format!(
                "serving comparison total token count differs from checked input-plus-output work: {expected_id}/{engine}"
            ));
        }
        let work = WindowWork {
            input_tokens,
            output_tokens,
            successful_requests,
            total_tokens,
        };
        if let Some(expected) = expected_work {
            if work.successful_requests != expected.successful_requests {
                return Err(format!(
                    "serving comparison successful-request work differs across engines: {expected_id}/{engine}"
                ));
            }
            if work.input_tokens != expected.input_tokens {
                return Err(format!(
                    "serving comparison input-token work differs across engines: {expected_id}/{engine}"
                ));
            }
            if work.output_tokens != expected.output_tokens {
                return Err(format!(
                    "serving comparison output-token work differs across engines: {expected_id}/{engine}"
                ));
            }
            if work.total_tokens != expected.total_tokens {
                return Err(format!(
                    "serving comparison total-token work differs across engines: {expected_id}/{engine}"
                ));
            }
        } else {
            expected_work = Some(work);
        }
        let (end_to_end, ttft, tpot, request_work) = validate_request_timings(
            metrics,
            duration,
            successful_requests,
            input_tokens,
            output_tokens,
            &expected_id,
            engine,
        )?;
        if let Some(expected) = &expected_request_work {
            if request_work != *expected {
                return Err(format!(
                    "serving comparison per-request input/output work differs across engines: {expected_id}/{engine}"
                ));
            }
        } else {
            expected_request_work = Some(request_work);
        }
        let input_throughput = rate(input_tokens, duration)?;
        let output_throughput = rate(output_tokens, duration)?;
        let total_throughput = rate(total_tokens, duration)?;
        if phase == "recorded" {
            samples[index].end_to_end.push(end_to_end);
            samples[index].input_throughput.push(input_throughput);
            samples[index].output_throughput.push(output_throughput);
            samples[index].tpot.push(tpot);
            samples[index].total_throughput.push(total_throughput);
            samples[index].ttft.push(ttft);
        }
    }
    Ok(())
}

fn validate_request_timings(
    metrics: &Map<String, Value>,
    duration_ns: u64,
    successful_requests: u64,
    input_tokens: u64,
    output_tokens: u64,
    row_id: &str,
    engine: &str,
) -> BenchResult<(
    TimingPercentiles,
    TimingPercentiles,
    TimingPercentiles,
    Vec<RequestWork>,
)> {
    let events = field(
        metrics,
        "request_events",
        "serving comparison window counters",
    )?
    .as_array()
    .ok_or_else(|| "serving comparison request events must be an array".to_owned())?;
    if events.len() != usize::try_from(successful_requests).unwrap_or(usize::MAX) {
        return Err(format!(
            "serving comparison request-event population differs from successful requests: {row_id}/{engine}"
        ));
    }
    let mut end_to_end = Vec::with_capacity(events.len());
    let mut ttft = Vec::with_capacity(events.len());
    let mut tpot = Vec::with_capacity(events.len());
    let mut request_work = Vec::with_capacity(events.len());
    let mut checked_input_tokens = 0_u64;
    let mut checked_output_tokens = 0_u64;
    for (ordinal, event) in events.iter().enumerate() {
        let event = exact_object(
            event,
            REQUEST_EVENT_KEYS,
            "serving comparison request event",
        )?;
        expect_u64(
            event,
            "request_ordinal",
            u64::try_from(ordinal)
                .map_err(|_| "serving comparison request ordinal does not fit u64".to_owned())?,
            "serving comparison request event",
        )?;
        let arrival = unsigned_u64(
            field(
                event,
                "arrival_offset_ns",
                "serving comparison request event",
            )?,
            "serving comparison request arrival offset",
        )?;
        let first = positive_u64(
            field(
                event,
                "first_token_offset_ns",
                "serving comparison request event",
            )?,
            "serving comparison first-token offset",
        )?;
        let terminal = positive_u64(
            field(
                event,
                "terminal_offset_ns",
                "serving comparison request event",
            )?,
            "serving comparison terminal offset",
        )?;
        if !(arrival < first && first < terminal && terminal <= duration_ns) {
            return Err(format!(
                "serving comparison request timing order or window bound is invalid: {row_id}/{engine}/{ordinal}"
            ));
        }
        let request_input = positive_u64(
            field(event, "input_tokens", "serving comparison request event")?,
            "serving comparison request input tokens",
        )?;
        let request_output = positive_u64(
            field(event, "output_tokens", "serving comparison request event")?,
            "serving comparison request output tokens",
        )?;
        if request_output < 2 {
            return Err(format!(
                "serving comparison request has fewer than two output tokens required for TPOT: {row_id}/{engine}/{ordinal}"
            ));
        }
        checked_input_tokens =
            checked_input_tokens
                .checked_add(request_input)
                .ok_or_else(|| {
                    "serving comparison per-request input-token sum overflowed".to_owned()
                })?;
        checked_output_tokens = checked_output_tokens
            .checked_add(request_output)
            .ok_or_else(|| {
                "serving comparison per-request output-token sum overflowed".to_owned()
            })?;
        let end_to_end_ns = terminal - arrival;
        let ttft_ns = first - arrival;
        let tpot_ns = (terminal - first) / (request_output - 1);
        if tpot_ns == 0 {
            return Err(format!(
                "serving comparison request TPOT rounded to zero: {row_id}/{engine}/{ordinal}"
            ));
        }
        end_to_end.push(end_to_end_ns);
        ttft.push(ttft_ns);
        tpot.push(tpot_ns);
        request_work.push(RequestWork {
            input_tokens: request_input,
            output_tokens: request_output,
        });
    }
    if checked_input_tokens != input_tokens || checked_output_tokens != output_tokens {
        return Err(format!(
            "serving comparison per-request token sums differ from window work: {row_id}/{engine}"
        ));
    }
    let end_to_end = timing_percentiles(&mut end_to_end, "end-to-end")?;
    let ttft = timing_percentiles(&mut ttft, "TTFT")?;
    let tpot = timing_percentiles(&mut tpot, "TPOT")?;
    validate_submitted_percentiles(metrics, "end_to_end_latency_ns", end_to_end)?;
    validate_submitted_percentiles(metrics, "ttft_ns", ttft)?;
    validate_submitted_percentiles(metrics, "tpot_ns_per_output_token", tpot)?;
    Ok((end_to_end, ttft, tpot, request_work))
}

fn validate_submitted_percentiles(
    metrics: &Map<String, Value>,
    suffix: &str,
    expected: TimingPercentiles,
) -> BenchResult<()> {
    for (percentile, value) in [
        ("p50", expected.p50),
        ("p90", expected.p90),
        ("p99", expected.p99),
    ] {
        let key = format!("{percentile}_{suffix}");
        expect_u64(
            metrics,
            &key,
            value,
            "serving comparison recomputed timing percentile",
        )?;
    }
    Ok(())
}

fn timing_percentiles(values: &mut [u64], description: &str) -> BenchResult<TimingPercentiles> {
    if values.is_empty() {
        return Err(format!(
            "serving comparison {description} population is empty"
        ));
    }
    values.sort_unstable();
    Ok(TimingPercentiles {
        p50: nearest_rank(values, 50, description)?,
        p90: nearest_rank(values, 90, description)?,
        p99: nearest_rank(values, 99, description)?,
    })
}

fn nearest_rank(values: &[u64], percentile: usize, description: &str) -> BenchResult<u64> {
    let rank = values
        .len()
        .checked_mul(percentile)
        .and_then(|value| value.checked_add(99))
        .ok_or_else(|| {
            format!("serving comparison {description} nearest-rank calculation overflowed")
        })?
        / 100;
    values.get(rank - 1).copied().ok_or_else(|| {
        format!("serving comparison {description} nearest rank is outside the population")
    })
}

fn build_record(
    policy: &Value,
    observations: &Value,
    inputs: &Inputs,
    samples: Vec<EngineSamples>,
) -> BenchResult<Value> {
    let policy_object = policy
        .as_object()
        .ok_or_else(|| "serving comparison policy must be an object".to_owned())?;
    let end_to_end_slo = positive_u64(
        field(
            policy_object,
            "p99_end_to_end_slo_ns",
            "serving comparison policy",
        )?,
        "serving comparison p99 end-to-end SLO",
    )?;
    let ttft_slo = positive_u64(
        field(
            policy_object,
            "p99_ttft_slo_ns",
            "serving comparison policy",
        )?,
        "serving comparison p99 TTFT SLO",
    )?;
    let tpot_slo = positive_u64(
        field(
            policy_object,
            "p99_tpot_slo_ns_per_output_token",
            "serving comparison policy",
        )?,
        "serving comparison p99 TPOT SLO",
    )?;
    let mut summaries = Vec::with_capacity(ENGINES.len());
    let mut total_throughput_medians = Vec::with_capacity(ENGINES.len());
    let mut total_throughput_samples = Vec::with_capacity(ENGINES.len());
    for (engine, mut sample) in ENGINES.iter().zip(samples) {
        let input_throughput = median(&mut sample.input_throughput)?;
        let output_throughput = median(&mut sample.output_throughput)?;
        let mut total_throughput_for_median = sample.total_throughput.clone();
        let total_throughput = median(&mut total_throughput_for_median)?;
        let end_to_end = median_timing(&mut sample.end_to_end)?;
        let ttft = median_timing(&mut sample.ttft)?;
        let tpot = median_timing(&mut sample.tpot)?;
        for (metric, value, slo) in [
            ("end-to-end latency", &end_to_end.2, end_to_end_slo),
            ("TTFT", &ttft.2, ttft_slo),
            ("TPOT", &tpot.2, tpot_slo),
        ] {
            if value.numerator > u128::from(slo) * value.denominator {
                return Err(format!(
                    "serving comparison median p99 {metric} exceeds the declared SLO: {engine}"
                ));
            }
        }
        summaries.push(json!({
            "id": engine,
            "median_input_tokens_per_second": input_throughput.as_json(),
            "median_p50_end_to_end_latency_ns": end_to_end.0.as_json(),
            "median_p50_tpot_ns_per_output_token": tpot.0.as_json(),
            "median_p50_ttft_ns": ttft.0.as_json(),
            "median_p90_end_to_end_latency_ns": end_to_end.1.as_json(),
            "median_p90_tpot_ns_per_output_token": tpot.1.as_json(),
            "median_p90_ttft_ns": ttft.1.as_json(),
            "median_p99_end_to_end_latency_ns": end_to_end.2.as_json(),
            "median_p99_tpot_ns_per_output_token": tpot.2.as_json(),
            "median_p99_ttft_ns": ttft.2.as_json(),
            "median_output_tokens_per_second": output_throughput.as_json(),
            "median_total_tokens_per_second": total_throughput.as_json(),
            "recorded_windows": SERVER_STARTS * RECORDED_PER_START,
        }));
        total_throughput_medians.push(total_throughput);
        total_throughput_samples.push(sample.total_throughput);
    }
    let fastest_baseline_index =
        if ratio_ge(&total_throughput_medians[1], &total_throughput_medians[2]) {
            1
        } else {
            2
        };
    let ratio_ppm = ratio_ppm(
        &total_throughput_medians[0],
        &total_throughput_medians[fastest_baseline_index],
    )?;
    let bootstrap = paired_bootstrap_interval(
        &total_throughput_samples[0],
        &total_throughput_samples[fastest_baseline_index],
        &sha256_identity(&inputs.policy_bytes),
        &sha256_identity(&inputs.observations_bytes),
    )?;
    if bootstrap.lower_ppm < COMPETITIVENESS_GATE_PPM {
        return Err(format!(
            "serving comparison paired-bootstrap lower bound {} is below the {} gate",
            bootstrap.lower_ppm, COMPETITIVENESS_GATE_PPM
        ));
    }
    let observation_object = observations
        .as_object()
        .ok_or_else(|| "serving comparison observations must be an object".to_owned())?;
    Ok(json!({
        "authority": RECORD_AUTHORITY,
        "bindings": {
            "engine_order": field(policy_object, "engine_order", "serving comparison policy")?,
            "implementations": field(policy_object, "implementations", "serving comparison policy")?,
            "p99_end_to_end_slo_ns": end_to_end_slo,
            "p99_tpot_slo_ns_per_output_token": tpot_slo,
            "p99_ttft_slo_ns": ttft_slo,
            "plan": field(policy_object, "plan", "serving comparison policy")?,
            "protocol_sha256": PROTOCOL_SHA256,
        },
        "competitiveness_metric": "median-total-tokens-per-second",
        "engine_summaries": summaries,
        "fastest_baseline": ENGINES[fastest_baseline_index],
        "ferric_to_fastest_baseline_ratio_ppm": ratio_ppm,
        "format": RECORD_FORMAT,
        "timing_semantics": {
            "clock": TIMING_CLOCK,
            "end_to_end_unit": "nanoseconds",
            "event_boundaries": TIMING_BOUNDARIES,
            "percentile_method": TIMING_PERCENTILE_METHOD,
            "source": TIMING_SOURCE,
            "tpot_arithmetic": "floor((terminal_offset_ns-first_token_offset_ns)/(output_tokens-1))",
            "tpot_unit": "nanoseconds-per-output-token-after-first",
            "ttft_unit": "nanoseconds",
        },
        "nonclaim": NONCLAIM,
        "obligation_id": "m1.r33",
        "observations_sha256": sha256_identity(&inputs.observations_bytes),
        "policy_sha256": sha256_identity(&inputs.policy_bytes),
        "paired_bootstrap_95_percent": {
            "algorithm": "paired-percentile-bootstrap-splitmix64-v3",
            "confidence_level_ppm": BOOTSTRAP_CONFIDENCE_PPM,
            "gate_lower_bound_ppm": COMPETITIVENESS_GATE_PPM,
            "lower_bound_ppm": bootstrap.lower_ppm,
            "metric": "total-tokens-per-second",
            "resamples": BOOTSTRAP_RESAMPLES,
            "seed_sha256": bootstrap.seed_sha256,
            "upper_bound_ppm": bootstrap.upper_ppm,
        },
        "raw_rows": field(observation_object, "rows", "serving comparison observations")?,
        "recorded_windows_per_engine": SERVER_STARTS * RECORDED_PER_START,
        "server_starts": SERVER_STARTS,
        "status": STATUS,
        "target": TARGET,
        "warmup_windows_per_engine": SERVER_STARTS * WARMUPS_PER_START,
    }))
}

fn median_timing(samples: &mut PercentileSamples) -> BenchResult<(Rational, Rational, Rational)> {
    Ok((
        median(&mut samples.p50)?,
        median(&mut samples.p90)?,
        median(&mut samples.p99)?,
    ))
}

fn median(values: &mut [u64]) -> BenchResult<Rational> {
    if values.is_empty() {
        return Err("serving comparison median population is empty".to_owned());
    }
    values.sort_unstable();
    let middle = values.len() / 2;
    if values.len() % 2 == 1 {
        Ok(Rational::new(u128::from(values[middle]), 1))
    } else {
        Ok(Rational::new(
            u128::from(values[middle - 1]) + u128::from(values[middle]),
            2,
        ))
    }
}

fn rate(tokens: u64, duration_ns: u64) -> BenchResult<u64> {
    let scaled = u128::from(tokens)
        .checked_mul(RATE_SCALE)
        .ok_or_else(|| "serving comparison throughput numerator overflowed".to_owned())?;
    u64::try_from(scaled / u128::from(duration_ns))
        .map_err(|_| "serving comparison throughput does not fit u64".to_owned())
        .and_then(|value| {
            if value == 0 {
                Err("serving comparison throughput rounded to zero".to_owned())
            } else {
                Ok(value)
            }
        })
}

fn ratio_ppm(numerator: &Rational, denominator: &Rational) -> BenchResult<u64> {
    let left = numerator
        .numerator
        .checked_mul(denominator.denominator)
        .and_then(|value| value.checked_mul(PPM_SCALE))
        .ok_or_else(|| "serving comparison ratio numerator overflowed".to_owned())?;
    let right = numerator
        .denominator
        .checked_mul(denominator.numerator)
        .ok_or_else(|| "serving comparison ratio denominator overflowed".to_owned())?;
    u64::try_from(left / right).map_err(|_| "serving comparison ratio does not fit u64".to_owned())
}

fn ratio_ge(left: &Rational, right: &Rational) -> bool {
    left.numerator * right.denominator >= right.numerator * left.denominator
}

fn paired_bootstrap_interval(
    ferric: &[u64],
    baseline: &[u64],
    policy_sha256: &str,
    observations_sha256: &str,
) -> BenchResult<BootstrapInterval> {
    if ferric.len() != baseline.len() || ferric.len() != SERVER_STARTS * RECORDED_PER_START {
        return Err("serving bootstrap paired sample roster is incomplete".to_owned());
    }
    if BOOTSTRAP_LOWER_RANK == 0
        || BOOTSTRAP_LOWER_RANK > BOOTSTRAP_UPPER_RANK
        || BOOTSTRAP_UPPER_RANK > BOOTSTRAP_RESAMPLES
    {
        return Err("serving bootstrap percentile ranks are invalid".to_owned());
    }
    let seed_material =
        format!("ferric-m1-r33-paired-bootstrap-v3|{policy_sha256}|{observations_sha256}");
    let seed_sha256 = sha256_identity(seed_material.as_bytes());
    let seed = u64::from_str_radix(&seed_sha256[..16], 16)
        .map_err(|_| "serving bootstrap seed digest is invalid".to_owned())?;
    let mut generator = SplitMix64 { state: seed };
    let mut estimates = Vec::new();
    estimates
        .try_reserve_exact(BOOTSTRAP_RESAMPLES)
        .map_err(|_| "cannot reserve serving bootstrap estimates".to_owned())?;
    let mut ferric_resample = vec![0_u64; ferric.len()];
    let mut baseline_resample = vec![0_u64; baseline.len()];
    for _ in 0..BOOTSTRAP_RESAMPLES {
        for pair in 0..ferric.len() {
            let index = generator.index(ferric.len())?;
            ferric_resample[pair] = ferric[index];
            baseline_resample[pair] = baseline[index];
        }
        let ferric_median = median(&mut ferric_resample)?;
        let baseline_median = median(&mut baseline_resample)?;
        estimates.push(ratio_ppm(&ferric_median, &baseline_median)?);
    }
    estimates.sort_unstable();
    Ok(BootstrapInterval {
        lower_ppm: estimates[BOOTSTRAP_LOWER_RANK - 1],
        seed_sha256,
        upper_ppm: estimates[BOOTSTRAP_UPPER_RANK - 1],
    })
}

fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

fn validate_engine_order(value: &Value) -> BenchResult<()> {
    if value != &json!(ENGINES) {
        return Err("serving comparison engine roster or order drifted".to_owned());
    }
    Ok(())
}

fn exact_object<'a>(
    value: &'a Value,
    expected: &[&str],
    description: &str,
) -> BenchResult<&'a Map<String, Value>> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{description} must be an object"))?;
    if object.len() != expected.len() || !expected.iter().all(|key| object.contains_key(*key)) {
        return Err(format!("{description} fields drifted"));
    }
    Ok(object)
}

fn field<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    description: &str,
) -> BenchResult<&'a Value> {
    object
        .get(key)
        .ok_or_else(|| format!("{description} is missing {key}"))
}

fn expect_string(
    object: &Map<String, Value>,
    key: &str,
    expected: &str,
    description: &str,
) -> BenchResult<()> {
    if field(object, key, description)?.as_str() != Some(expected) {
        return Err(format!("{description} {key} drifted"));
    }
    Ok(())
}

fn expect_u64(
    object: &Map<String, Value>,
    key: &str,
    expected: u64,
    description: &str,
) -> BenchResult<()> {
    if field(object, key, description)?.as_u64() != Some(expected) {
        return Err(format!("{description} {key} drifted"));
    }
    Ok(())
}

fn unsigned_u64(value: &Value, description: &str) -> BenchResult<u64> {
    value
        .as_u64()
        .ok_or_else(|| format!("{description} must be an unsigned integer"))
}

fn positive_u64(value: &Value, description: &str) -> BenchResult<u64> {
    let value = unsigned_u64(value, description)?;
    if value == 0 {
        return Err(format!("{description} must be positive"));
    }
    Ok(value)
}

fn safe_string<'a>(value: &'a Value, description: &str) -> BenchResult<&'a str> {
    let value = value
        .as_str()
        .ok_or_else(|| format!("{description} must be a string"))?;
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_graphic() && byte != b'"' && byte != b'\\')
    {
        return Err(format!("{description} must be nonempty printable ASCII"));
    }
    Ok(value)
}

fn sha_value(value: &Value, description: &str) -> BenchResult<()> {
    let value = value
        .as_str()
        .ok_or_else(|| format!("{description} must be a string"))?;
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || value.bytes().all(|byte| byte == value.as_bytes()[0])
    {
        return Err(format!("{description} is not a valid SHA-256 identity"));
    }
    Ok(())
}

fn file_name(path: &Path, description: &str) -> BenchResult<PathBuf> {
    path.file_name()
        .map(PathBuf::from)
        .ok_or_else(|| format!("{description} path has no filename"))
}

fn same_path(left: &Path, right: &Path) -> bool {
    left == right
}

fn require_protocol() -> BenchResult<()> {
    let manifest = std::env::var_os("CARGO_MANIFEST_DIR")
        .ok_or_else(|| "serving comparison manifest directory is absent".to_owned())?;
    let protocol = PathBuf::from(manifest).join("m1_r33_serving_comparison_protocol.json");
    let (_, value, bytes, file) =
        load_canonical_document_held(&protocol, "serving comparison protocol")?;
    let object = exact_object(
        &value,
        &[
            "authority",
            "format",
            "nonclaim",
            "obligation_id",
            "schema",
            "status",
            "target",
        ],
        "serving comparison protocol",
    )?;
    expect_string(
        object,
        "authority",
        PROTOCOL_AUTHORITY,
        "serving comparison protocol",
    )?;
    expect_string(
        object,
        "format",
        PROTOCOL_FORMAT,
        "serving comparison protocol",
    )?;
    expect_string(object, "nonclaim", NONCLAIM, "serving comparison protocol")?;
    expect_string(
        object,
        "obligation_id",
        "m1.r33",
        "serving comparison protocol",
    )?;
    expect_string(object, "status", STATUS, "serving comparison protocol")?;
    expect_string(object, "target", TARGET, "serving comparison protocol")?;
    if sha256_identity(&bytes) != PROTOCOL_SHA256 {
        return Err("serving comparison protocol SHA-256 drifted".to_owned());
    }
    file.validate_snapshot("serving comparison protocol")
}

impl RecordPublication {
    fn create(path: &Path, bytes: &[u8]) -> BenchResult<Self> {
        let output_name = safe_output_name(path)?;
        let parent_path = admitted_output_parent(path)?;
        let parent = openat2(
            CWD,
            &parent_path,
            OFlags::RDONLY
                | OFlags::DIRECTORY
                | OFlags::NOFOLLOW
                | OFlags::NONBLOCK
                | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )
        .map_err(|error| format!("cannot securely open serving record parent: {error}"))?;
        require_absent_at(&parent, &output_name)?;

        for nonce in 0..1_024_u16 {
            let mut staging_name = OsString::from(".");
            staging_name.push(&output_name);
            staging_name.push(format!(".staging.{}.{nonce}", std::process::id()));
            let descriptor = match openat2(
                &parent,
                Path::new(&staging_name),
                OFlags::RDWR
                    | OFlags::CREATE
                    | OFlags::EXCL
                    | OFlags::NOFOLLOW
                    | OFlags::NONBLOCK
                    | OFlags::CLOEXEC,
                Mode::RUSR | Mode::WUSR,
                ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
            ) {
                Ok(descriptor) => descriptor,
                Err(error) if error == rustix::io::Errno::EXIST => continue,
                Err(error) => {
                    return Err(format!(
                        "cannot create staged serving comparison record: {error}"
                    ));
                }
            };
            let mut file = File::from(descriptor);
            let created = match fstat(&file) {
                Ok(created) if valid_record_file(&created, 0) => created,
                Ok(created) => {
                    cleanup_created_name(&parent, &staging_name, &created);
                    return Err("created serving comparison staging entry is invalid".to_owned());
                }
                Err(error) => {
                    return Err(format!(
                        "cannot inspect staged serving comparison record: {error}"
                    ));
                }
            };
            if let Err(error) = file.write_all(bytes) {
                drop(file);
                cleanup_created_name(&parent, &staging_name, &created);
                return Err(format!(
                    "cannot write staged serving comparison record: {error}"
                ));
            }
            if let Err(error) = file.sync_all() {
                drop(file);
                cleanup_created_name(&parent, &staging_name, &created);
                return Err(format!(
                    "cannot synchronize staged serving comparison record: {error}"
                ));
            }
            let settled = match fstat(&file) {
                Ok(settled) if valid_record_file(&settled, bytes.len()) => settled,
                Ok(_) => {
                    drop(file);
                    cleanup_created_name(&parent, &staging_name, &created);
                    return Err("settled serving comparison staging entry is invalid".to_owned());
                }
                Err(error) => {
                    drop(file);
                    cleanup_created_name(&parent, &staging_name, &created);
                    return Err(format!(
                        "cannot reinspect staged serving comparison record: {error}"
                    ));
                }
            };
            return Ok(Self {
                armed: true,
                expected_sha256: sha256_identity(bytes),
                file,
                output_name,
                parent,
                settled,
                staging_name,
            });
        }
        Err("serving comparison staging namespace was exhausted".to_owned())
    }

    fn rebind(&self, name: &OsStr, published: bool, description: &str) -> BenchResult<File> {
        let held = fstat(&self.file)
            .map_err(|error| format!("cannot inspect held {description}: {error}"))?;
        let held_matches = if published {
            same_publication_snapshot(&self.settled, &held)
        } else {
            same_file_snapshot(&self.settled, &held)
        };
        if !held_matches {
            return Err(format!("held {description} metadata changed"));
        }
        let descriptor = openat2(
            &self.parent,
            Path::new(name),
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )
        .map_err(|error| format!("cannot rebind {description}: {error}"))?;
        let rebound = File::from(descriptor);
        let rebound_stat = fstat(&rebound)
            .map_err(|error| format!("cannot inspect rebound {description}: {error}"))?;
        let rebound_matches = if published {
            same_publication_snapshot(&self.settled, &rebound_stat)
        } else {
            same_file_snapshot(&self.settled, &rebound_stat)
        };
        let expected_size = usize::try_from(self.settled.st_size)
            .map_err(|_| format!("held {description} length is invalid"))?;
        if !rebound_matches || !valid_record_file(&rebound_stat, expected_size) {
            return Err(format!("{description} name does not bind the held record"));
        }
        Ok(rebound)
    }

    fn verify_bytes(&self, mut file: File, published: bool, description: &str) -> BenchResult<()> {
        let initial = fstat(&file)
            .map_err(|error| format!("cannot inspect {description} before reread: {error}"))?;
        let length = usize::try_from(self.settled.st_size)
            .map_err(|_| format!("{description} length is invalid"))?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length.saturating_add(1))
            .map_err(|_| format!("cannot reserve {description} verification buffer"))?;
        Read::by_ref(&mut file)
            .take(length.saturating_add(1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("cannot reread {description}: {error}"))?;
        let final_stat =
            fstat(&file).map_err(|error| format!("cannot reinspect {description}: {error}"))?;
        let snapshot_matches = if published {
            same_publication_snapshot(&self.settled, &initial)
                && same_file_snapshot(&initial, &final_stat)
        } else {
            same_file_snapshot(&self.settled, &initial) && same_file_snapshot(&initial, &final_stat)
        };
        if bytes.len() != length
            || sha256_identity(&bytes) != self.expected_sha256
            || !snapshot_matches
        {
            return Err(format!("{description} bytes or metadata changed"));
        }
        Ok(())
    }

    fn publish_with_hook(
        mut self,
        after_first_published_verification: impl FnOnce() -> BenchResult<()>,
    ) -> BenchResult<()> {
        let staged = self.rebind(
            &self.staging_name,
            false,
            "staged serving comparison record",
        )?;
        self.verify_bytes(staged, false, "staged serving comparison record")?;
        renameat_with(
            &self.parent,
            self.staging_name.as_os_str(),
            &self.parent,
            self.output_name.as_os_str(),
            RenameFlags::NOREPLACE,
        )
        .map_err(|error| {
            if error == rustix::io::Errno::EXIST {
                "serving comparison output appeared before no-replace publication".to_owned()
            } else {
                format!("cannot publish serving comparison record without replacement: {error}")
            }
        })?;
        self.armed = false;
        let published = self.rebind(
            &self.output_name,
            true,
            "published serving comparison record",
        )?;
        self.verify_bytes(published, true, "published serving comparison record")?;
        after_first_published_verification()?;
        fsync(&self.parent)
            .map_err(|error| format!("cannot sync serving comparison output parent: {error}"))?;
        let final_name = self.rebind(
            &self.output_name,
            true,
            "final published serving comparison record",
        )?;
        self.verify_bytes(
            final_name,
            true,
            "final published serving comparison record",
        )?;
        let final_rebound = self.rebind(
            &self.output_name,
            true,
            "final rebound serving comparison record",
        )?;
        self.verify_bytes(
            final_rebound,
            true,
            "final rebound serving comparison record",
        )
    }
}

impl Drop for RecordPublication {
    fn drop(&mut self) {
        if self.armed && name_has_identity(&self.parent, &self.staging_name, &self.settled) {
            let _ = unlinkat(
                &self.parent,
                self.staging_name.as_os_str(),
                AtFlags::empty(),
            );
        }
    }
}

fn write_new(path: &Path, bytes: &[u8]) -> BenchResult<()> {
    write_new_with_hook(path, bytes, || Ok(()))
}

fn write_new_with_hook(
    path: &Path,
    bytes: &[u8],
    after_first_published_verification: impl FnOnce() -> BenchResult<()>,
) -> BenchResult<()> {
    RecordPublication::create(path, bytes)?.publish_with_hook(after_first_published_verification)
}

fn admitted_output_parent(path: &Path) -> BenchResult<PathBuf> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    if !parent.as_os_str().as_encoded_bytes().is_ascii()
        || parent
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
    {
        return Err("serving comparison output parent path is not admitted".to_owned());
    }
    Ok(parent.to_path_buf())
}

fn safe_output_name(path: &Path) -> BenchResult<OsString> {
    let name = path
        .file_name()
        .ok_or_else(|| "serving comparison output has no final component".to_owned())?;
    let bytes = name.as_encoded_bytes();
    if bytes.is_empty()
        || bytes.len() > 255
        || !bytes.is_ascii()
        || !matches!(
            Path::new(name).components().next(),
            Some(Component::Normal(_))
        )
        || Path::new(name).components().count() != 1
    {
        return Err("serving comparison output name is invalid".to_owned());
    }
    Ok(name.to_os_string())
}

fn require_absent_at(parent: &OwnedFd, name: &OsStr) -> BenchResult<()> {
    match openat2(
        parent,
        Path::new(name),
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    ) {
        Ok(_) => Err("serving comparison output already exists".to_owned()),
        Err(error) if error == rustix::io::Errno::NOENT => Ok(()),
        Err(error) => Err(format!(
            "cannot safely inspect serving comparison output: {error}"
        )),
    }
}

fn valid_record_file(stat: &Stat, expected_size: usize) -> bool {
    FileType::from_raw_mode(stat.st_mode) == FileType::RegularFile
        && stat.st_nlink == 1
        && usize::try_from(stat.st_size) == Ok(expected_size)
}

fn name_has_identity(parent: &OwnedFd, name: &OsStr, identity: &Stat) -> bool {
    let Ok(descriptor) = openat2(
        parent,
        Path::new(name),
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    ) else {
        return false;
    };
    fstat(&descriptor)
        .is_ok_and(|current| current.st_dev == identity.st_dev && current.st_ino == identity.st_ino)
}

fn cleanup_created_name(parent: &OwnedFd, name: &OsStr, identity: &Stat) {
    if name_has_identity(parent, name, identity) {
        let _ = unlinkat(parent, name, AtFlags::empty());
    }
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

fn same_publication_snapshot(initial: &Stat, published: &Stat) -> bool {
    initial.st_dev == published.st_dev
        && initial.st_ino == published.st_ino
        && initial.st_mode == published.st_mode
        && initial.st_nlink == published.st_nlink
        && initial.st_size == published.st_size
        && initial.st_mtime == published.st_mtime
        && initial.st_mtime_nsec == published.st_mtime_nsec
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NONCE: AtomicU64 = AtomicU64::new(0);

    struct Temporary(PathBuf);

    impl Temporary {
        fn new() -> Self {
            let nonce = NONCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "ferric-m1-r33-serving-record-test.{}.{nonce}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for Temporary {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn digest(label: &str) -> String {
        sha256_identity(label.as_bytes())
    }

    fn implementation(id: &str) -> Value {
        json!({
            "config_sha256": digest(&format!("{id}-config")),
            "id": id,
            "implementation_sha256": digest(&format!("{id}-implementation")),
            "protocol_sha256": digest(&format!("{id}-protocol")),
            "source_sha256": digest(&format!("{id}-source")),
            "tuning_budget_sha256": digest("equal-budget"),
            "tuning_sha256": digest(&format!("{id}-tuning")),
            "version": format!("pinned-{id}-version"),
        })
    }

    fn metrics(duration_ns: u64, ttft_ns: u64, tpot_ns: u64) -> Value {
        let events = (0..4_u64)
            .map(|request_ordinal| {
                let arrival = request_ordinal * 100;
                json!({
                    "arrival_offset_ns": arrival,
                    "first_token_offset_ns": arrival + ttft_ns,
                    "input_tokens": 2,
                    "output_tokens": 2,
                    "request_ordinal": request_ordinal,
                    "terminal_offset_ns": arrival + ttft_ns + tpot_ns,
                })
            })
            .collect::<Vec<_>>();
        let end_to_end = ttft_ns + tpot_ns;
        json!({
            "duration_ns": duration_ns,
            "failed_requests": 0,
            "input_tokens": 8,
            "output_tokens": 8,
            "p50_end_to_end_latency_ns": end_to_end,
            "p50_tpot_ns_per_output_token": tpot_ns,
            "p50_ttft_ns": ttft_ns,
            "p90_end_to_end_latency_ns": end_to_end,
            "p90_tpot_ns_per_output_token": tpot_ns,
            "p90_ttft_ns": ttft_ns,
            "p99_end_to_end_latency_ns": end_to_end,
            "p99_tpot_ns_per_output_token": tpot_ns,
            "p99_ttft_ns": ttft_ns,
            "request_events": events,
            "successful_requests": 4,
            "total_tokens": 16,
        })
    }

    fn policy() -> Value {
        json!({
            "authority": POLICY_AUTHORITY,
            "engine_order": ENGINES,
            "format": POLICY_FORMAT,
            "implementations": ENGINES.iter().map(|id| implementation(id)).collect::<Vec<_>>(),
            "nonclaim": NONCLAIM,
            "obligation_id": "m1.r33",
            "p99_end_to_end_slo_ns": 1_000_000,
            "p99_tpot_slo_ns_per_output_token": 1_000_000,
            "p99_ttft_slo_ns": 1_000_000,
            "plan": PLAN_KEYS.iter().map(|key| ((*key).to_owned(), Value::String(digest(key)))).collect::<Map<_, _>>(),
            "protocol_sha256": PROTOCOL_SHA256,
            "sample_roster": {
                "recorded_windows_per_start": RECORDED_PER_START,
                "server_starts": SERVER_STARTS,
                "warmup_windows_per_start": WARMUPS_PER_START,
            },
            "status": "pre-observation",
            "target": TARGET,
        })
    }

    fn observations(policy: &Value) -> Value {
        let mut rows = Vec::new();
        let mut ordinal = 0_usize;
        for start in 0..SERVER_STARTS {
            for (phase, count) in [
                ("warmup", WARMUPS_PER_START),
                ("recorded", RECORDED_PER_START),
            ] {
                for window in 0..count {
                    let order = (0..ENGINES.len())
                        .map(|offset| ENGINES[(ordinal + offset) % ENGINES.len()])
                        .collect::<Vec<_>>();
                    rows.push(json!({
                        "engine_order": order,
                        "faults": [],
                        "id": format!("start-{start}.{phase}-{window:02}"),
                        "ordinal": ordinal,
                        "phase": phase,
                        "server_start": start,
                        "status": "passed",
                        "values": {
                            "ferric": metrics(1000, 10, 20),
                            "vllm": metrics(1000, 12, 24),
                            "sglang": metrics(1000, 14, 26),
                        },
                        "window": window,
                    }));
                    ordinal += 1;
                }
            }
        }
        let bytes = encode_canonical_document(policy).unwrap();
        json!({
            "authority": OBSERVATIONS_AUTHORITY,
            "engine_order": policy["engine_order"],
            "format": OBSERVATIONS_FORMAT,
            "implementations": policy["implementations"],
            "nonclaim": NONCLAIM,
            "obligation_id": "m1.r33",
            "plan": policy["plan"],
            "policy_sha256": sha256_identity(&bytes),
            "rows": rows,
            "status": "externally-collected",
            "target": TARGET,
        })
    }

    fn write_fixture(root: &Path, policy: &Value, observations: &Value) -> (PathBuf, PathBuf) {
        let policy_path = root.join("policy.json");
        let observations_path = root.join("observations.json");
        fs::write(&policy_path, encode_canonical_document(policy).unwrap()).unwrap();
        fs::write(
            &observations_path,
            encode_canonical_document(observations).unwrap(),
        )
        .unwrap();
        (policy_path, observations_path)
    }

    #[test]
    fn exact_roster_recomputes_comparison_and_carries_raw_rows() {
        let temporary = Temporary::new();
        let policy = policy();
        let observations = observations(&policy);
        let (policy_path, observations_path) = write_fixture(&temporary.0, &policy, &observations);
        let record = validate(&policy_path, &observations_path).unwrap();
        assert_eq!(record["authority"], RECORD_AUTHORITY);
        assert_eq!(record["format"], RECORD_FORMAT);
        assert_eq!(record["fastest_baseline"], "vllm");
        assert_eq!(record["ferric_to_fastest_baseline_ratio_ppm"], 1_000_000);
        assert_eq!(
            record["paired_bootstrap_95_percent"]["lower_bound_ppm"],
            1_000_000
        );
        assert_eq!(
            record["paired_bootstrap_95_percent"]["upper_bound_ppm"],
            1_000_000
        );
        assert_eq!(record["raw_rows"].as_array().unwrap().len(), 60);
        assert_eq!(record["recorded_windows_per_engine"], 30);
        assert_eq!(
            record["engine_summaries"][0]["median_input_tokens_per_second"]["numerator"],
            "8000000"
        );
        assert_eq!(
            record["engine_summaries"][0]["median_output_tokens_per_second"]["numerator"],
            "8000000"
        );
        assert_eq!(
            record["engine_summaries"][0]["median_total_tokens_per_second"]["numerator"],
            "16000000"
        );
        assert_eq!(
            record["engine_summaries"][0]["median_p99_ttft_ns"]["numerator"],
            "10"
        );
        assert_eq!(
            record["engine_summaries"][0]["median_p99_tpot_ns_per_output_token"]["numerator"],
            "20"
        );
        assert_eq!(
            record["timing_semantics"]["percentile_method"],
            TIMING_PERCENTILE_METHOD
        );
        assert_eq!(record["timing_semantics"]["source"], TIMING_SOURCE);
        assert_eq!(
            record["paired_bootstrap_95_percent"]["algorithm"],
            "paired-percentile-bootstrap-splitmix64-v3"
        );
        assert_eq!(
            record["paired_bootstrap_95_percent"]["metric"],
            "total-tokens-per-second"
        );
        assert_eq!(record["status"], STATUS);
    }

    #[test]
    fn missing_reordered_failed_and_summary_like_rows_fail_closed() {
        let policy = policy();
        let mut missing = observations(&policy);
        missing["rows"].as_array_mut().unwrap().pop();
        assert!(validate_observations(
            &missing,
            &policy,
            missing["policy_sha256"].as_str().unwrap()
        )
        .is_err());

        let mut reordered = observations(&policy);
        reordered["rows"].as_array_mut().unwrap().swap(0, 1);
        assert!(validate_observations(
            &reordered,
            &policy,
            reordered["policy_sha256"].as_str().unwrap()
        )
        .is_err());

        let mut failed = observations(&policy);
        failed["rows"][0]["values"]["ferric"]["failed_requests"] = json!(1);
        assert!(
            validate_observations(&failed, &policy, failed["policy_sha256"].as_str().unwrap())
                .is_err()
        );

        let mut submitted_summary = observations(&policy);
        submitted_summary["rows"][0]["values"]["ferric"]["throughput"] = json!(10_000);
        assert!(validate_observations(
            &submitted_summary,
            &policy,
            submitted_summary["policy_sha256"].as_str().unwrap()
        )
        .is_err());
    }

    #[test]
    fn unequal_successful_request_and_token_work_fail_closed() {
        let policy = policy();

        let mut requests = observations(&policy);
        requests["rows"][0]["values"]["vllm"]["successful_requests"] = json!(5);
        let error = validate_observations(
            &requests,
            &policy,
            requests["policy_sha256"].as_str().unwrap(),
        )
        .unwrap_err();
        assert!(error.contains("successful-request work differs"));

        let mut input = observations(&policy);
        input["rows"][0]["values"]["vllm"]["input_tokens"] = json!(9);
        input["rows"][0]["values"]["vllm"]["total_tokens"] = json!(17);
        let error =
            validate_observations(&input, &policy, input["policy_sha256"].as_str().unwrap())
                .unwrap_err();
        assert!(error.contains("input-token work differs"));

        let mut output = observations(&policy);
        output["rows"][0]["values"]["sglang"]["output_tokens"] = json!(9);
        output["rows"][0]["values"]["sglang"]["total_tokens"] = json!(17);
        let error =
            validate_observations(&output, &policy, output["policy_sha256"].as_str().unwrap())
                .unwrap_err();
        assert!(error.contains("output-token work differs"));
    }

    #[test]
    fn inconsistent_and_overflowing_token_arithmetic_fail_closed() {
        let policy = policy();

        let mut inconsistent = observations(&policy);
        inconsistent["rows"][0]["values"]["sglang"]["total_tokens"] = json!(17);
        let error = validate_observations(
            &inconsistent,
            &policy,
            inconsistent["policy_sha256"].as_str().unwrap(),
        )
        .unwrap_err();
        assert!(error.contains("differs from checked input-plus-output work"));

        let mut overflow = observations(&policy);
        overflow["rows"][0]["values"]["ferric"]["input_tokens"] = json!(u64::MAX);
        overflow["rows"][0]["values"]["ferric"]["output_tokens"] = json!(1);
        overflow["rows"][0]["values"]["ferric"]["total_tokens"] = json!(u64::MAX);
        let error = validate_observations(
            &overflow,
            &policy,
            overflow["policy_sha256"].as_str().unwrap(),
        )
        .unwrap_err();
        assert!(error.contains("input-plus-output token count overflowed"));
    }

    #[test]
    fn request_timing_and_pairing_mutations_fail_closed() {
        let policy = policy();
        for (mutation, expected) in [
            ("extra-field", "request event fields drifted"),
            ("nonmonotonic", "timing order or window bound is invalid"),
            ("single-output", "fewer than two output tokens"),
            ("event-sum", "per-request token sums differ"),
            ("percentile", "p99_ttft_ns drifted"),
            ("paired-work", "per-request input/output work differs"),
            ("ordinal", "request_ordinal drifted"),
        ] {
            let mut candidate = observations(&policy);
            match mutation {
                "extra-field" => {
                    candidate["rows"][0]["values"]["ferric"]["request_events"][0]
                        ["submitted_ttft_ns"] = json!(10);
                }
                "nonmonotonic" => {
                    candidate["rows"][0]["values"]["ferric"]["request_events"][0]
                        ["first_token_offset_ns"] = json!(30);
                    candidate["rows"][0]["values"]["ferric"]["request_events"][0]
                        ["terminal_offset_ns"] = json!(30);
                }
                "single-output" => {
                    candidate["rows"][0]["values"]["ferric"]["request_events"][0]
                        ["output_tokens"] = json!(1);
                }
                "event-sum" => {
                    candidate["rows"][0]["values"]["ferric"]["request_events"][0]["input_tokens"] =
                        json!(3);
                }
                "percentile" => {
                    candidate["rows"][0]["values"]["ferric"]["p99_ttft_ns"] = json!(11);
                }
                "paired-work" => {
                    candidate["rows"][0]["values"]["vllm"]["request_events"][0]["input_tokens"] =
                        json!(3);
                    candidate["rows"][0]["values"]["vllm"]["request_events"][1]["input_tokens"] =
                        json!(1);
                }
                "ordinal" => {
                    candidate["rows"][0]["values"]["ferric"]["request_events"][1]
                        ["request_ordinal"] = json!(0);
                }
                _ => unreachable!(),
            }
            let error = validate_observations(
                &candidate,
                &policy,
                candidate["policy_sha256"].as_str().unwrap(),
            )
            .unwrap_err();
            assert!(error.contains(expected), "{mutation}: {error}");
        }
    }

    #[test]
    fn older_schemas_and_noncanonical_bytes_are_not_reinterpreted() {
        let policy = policy();

        let mut v1_policy = policy.clone();
        v1_policy["format"] = json!("FERRIC-M1-R33-SERVING-COMPARISON-POLICY-V1");
        assert!(validate_policy(&v1_policy)
            .unwrap_err()
            .contains("format drifted"));

        let mut v2_policy = policy.clone();
        v2_policy["format"] = json!("FERRIC-M1-R33-SERVING-COMPARISON-POLICY-V2");
        assert!(validate_policy(&v2_policy)
            .unwrap_err()
            .contains("format drifted"));

        let mut v1_observations = observations(&policy);
        v1_observations["format"] = json!("FERRIC-M1-R33-SERVING-COMPARISON-OBSERVATIONS-V1");
        let error = validate_observations(
            &v1_observations,
            &policy,
            v1_observations["policy_sha256"].as_str().unwrap(),
        )
        .unwrap_err();
        assert!(error.contains("format drifted"));

        let mut v2_observations = observations(&policy);
        v2_observations["format"] = json!("FERRIC-M1-R33-SERVING-COMPARISON-OBSERVATIONS-V2");
        let error = validate_observations(
            &v2_observations,
            &policy,
            v2_observations["policy_sha256"].as_str().unwrap(),
        )
        .unwrap_err();
        assert!(error.contains("format drifted"));

        let mut total_only = observations(&policy);
        let metrics = total_only["rows"][0]["values"]["ferric"]
            .as_object_mut()
            .unwrap();
        metrics.remove("input_tokens");
        metrics.remove("output_tokens");
        metrics.remove("p99_end_to_end_latency_ns");
        metrics.insert("p99_latency_ns".to_owned(), json!(100));
        let error = validate_observations(
            &total_only,
            &policy,
            total_only["policy_sha256"].as_str().unwrap(),
        )
        .unwrap_err();
        assert!(error.contains("fields drifted"));

        let temporary = Temporary::new();
        let observations = observations(&policy);
        let policy_path = temporary.0.join("policy.json");
        let observations_path = temporary.0.join("observations.json");
        fs::write(&policy_path, serde_json::to_vec_pretty(&policy).unwrap()).unwrap();
        fs::write(
            &observations_path,
            encode_canonical_document(&observations).unwrap(),
        )
        .unwrap();
        assert!(validate(&policy_path, &observations_path)
            .unwrap_err()
            .contains("not canonical JSON"));
    }

    #[test]
    fn identity_tuning_policy_and_slo_substitution_fail_closed() {
        let original = policy();
        let observations = observations(&original);

        let mut identity = original.clone();
        identity["implementations"][1]["source_sha256"] = json!(digest("other-vllm-source"));
        assert!(validate_observations(
            &observations,
            &identity,
            observations["policy_sha256"].as_str().unwrap()
        )
        .is_err());

        let mut unequal = original.clone();
        unequal["implementations"][2]["tuning_budget_sha256"] = json!(digest("larger-budget"));
        assert!(validate_policy(&unequal).is_err());

        let mut replay = observations.clone();
        replay["policy_sha256"] = json!(digest("different-policy"));
        let policy_bytes = encode_canonical_document(&original).unwrap();
        assert!(
            validate_observations(&replay, &original, &sha256_identity(&policy_bytes)).is_err()
        );

        let temporary = Temporary::new();
        let mut too_slow = observations;
        for row in too_slow["rows"].as_array_mut().unwrap() {
            row["values"]["sglang"]["p99_end_to_end_latency_ns"] = json!(2_000_000);
        }
        let (policy_path, observations_path) = write_fixture(&temporary.0, &original, &too_slow);
        assert!(validate(&policy_path, &observations_path).is_err());
    }

    #[test]
    fn crossed_window_record_bootstrap_preserves_original_pairing() {
        let temporary = Temporary::new();
        let policy = policy();
        let mut observations = observations(&policy);
        let mut ferric_samples = Vec::new();
        let mut vllm_samples = Vec::new();
        let mut recorded = 0_usize;
        for row in observations["rows"].as_array_mut().unwrap() {
            if row["phase"] != "recorded" {
                continue;
            }
            let ferric_duration = if recorded.is_multiple_of(2) {
                1_020
            } else {
                1_000
            };
            let vllm_duration = if recorded.is_multiple_of(2) {
                1_000
            } else {
                1_020
            };
            row["values"]["ferric"]["duration_ns"] = json!(ferric_duration);
            row["values"]["vllm"]["duration_ns"] = json!(vllm_duration);
            row["values"]["sglang"]["duration_ns"] = json!(1_030);
            ferric_samples.push(rate(16, ferric_duration).unwrap());
            vllm_samples.push(rate(16, vllm_duration).unwrap());
            recorded += 1;
        }
        assert_eq!(recorded, SERVER_STARTS * RECORDED_PER_START);

        let policy_bytes = encode_canonical_document(&policy).unwrap();
        let observations_bytes = encode_canonical_document(&observations).unwrap();
        let policy_sha256 = sha256_identity(&policy_bytes);
        let observations_sha256 = sha256_identity(&observations_bytes);
        let aligned = paired_bootstrap_interval(
            &ferric_samples,
            &vllm_samples,
            &policy_sha256,
            &observations_sha256,
        )
        .unwrap();
        let mut independently_sorted_ferric = ferric_samples.clone();
        let mut independently_sorted_vllm = vllm_samples.clone();
        independently_sorted_ferric.sort_unstable();
        independently_sorted_vllm.sort_unstable();
        let independently_sorted = paired_bootstrap_interval(
            &independently_sorted_ferric,
            &independently_sorted_vllm,
            &policy_sha256,
            &observations_sha256,
        )
        .unwrap();
        assert_ne!(aligned, independently_sorted);
        assert!(aligned.lower_ppm >= COMPETITIVENESS_GATE_PPM);

        let (policy_path, observations_path) = write_fixture(&temporary.0, &policy, &observations);
        let record = validate(&policy_path, &observations_path).unwrap();
        assert_eq!(record["fastest_baseline"], "vllm");
        assert_eq!(
            record["paired_bootstrap_95_percent"]["lower_bound_ppm"],
            aligned.lower_ppm
        );
        assert_eq!(
            record["paired_bootstrap_95_percent"]["seed_sha256"],
            aligned.seed_sha256
        );
        assert_eq!(
            record["paired_bootstrap_95_percent"]["upper_bound_ppm"],
            aligned.upper_ppm
        );
        assert_ne!(
            record["paired_bootstrap_95_percent"]["lower_bound_ppm"],
            independently_sorted.lower_ppm
        );
    }

    #[test]
    fn paired_bootstrap_is_reproducible_and_enforces_the_lower_bound() {
        let ferric = vec![12_000_000; SERVER_STARTS * RECORDED_PER_START];
        let baseline = vec![12_000_000; SERVER_STARTS * RECORDED_PER_START];
        let first =
            paired_bootstrap_interval(&ferric, &baseline, &digest("policy"), &digest("rows"))
                .unwrap();
        let replay =
            paired_bootstrap_interval(&ferric, &baseline, &digest("policy"), &digest("rows"))
                .unwrap();
        assert_eq!(first, replay);
        assert_eq!(first.lower_ppm, 1_000_000);
        assert_eq!(first.upper_ppm, 1_000_000);

        let temporary = Temporary::new();
        let policy = policy();
        let mut below_gate = observations(&policy);
        for row in below_gate["rows"].as_array_mut().unwrap() {
            row["values"]["ferric"]["duration_ns"] = json!(2_000);
        }
        let (policy_path, observations_path) = write_fixture(&temporary.0, &policy, &below_gate);
        let error = validate(&policy_path, &observations_path).unwrap_err();
        assert!(error.contains("paired-bootstrap lower bound"));
    }

    #[test]
    fn record_publication_never_replaces_an_existing_path() {
        let temporary = Temporary::new();
        let policy = policy();
        let observations = observations(&policy);
        let (policy_path, observations_path) = write_fixture(&temporary.0, &policy, &observations);
        let output = temporary.0.join("record.json");
        fs::write(&output, b"preserve\n").unwrap();
        assert!(run(vec![
            policy_path.into_os_string(),
            observations_path.into_os_string(),
            output.clone().into_os_string(),
        ])
        .is_err());
        assert_eq!(fs::read(output).unwrap(), b"preserve\n");
    }

    #[test]
    fn published_name_substitution_fails_final_custody() {
        let temporary = Temporary::new();
        let output = temporary.0.join("record.json");
        let displaced = temporary.0.join("displaced.json");
        let expected = encode_canonical_document(&json!({"record": "expected"})).unwrap();
        let substituted = encode_canonical_document(&json!({"record": "substituted"})).unwrap();
        let output_for_hook = output.clone();
        let displaced_for_hook = displaced.clone();
        let result = write_new_with_hook(&output, &expected, || {
            fs::rename(&output_for_hook, &displaced_for_hook)
                .map_err(|error| format!("cannot displace published test record: {error}"))?;
            fs::write(&output_for_hook, &substituted)
                .map_err(|error| format!("cannot substitute published test record: {error}"))
        });
        assert!(result.is_err());
        assert_eq!(fs::read(&output).unwrap(), substituted);
        assert_eq!(fs::read(&displaced).unwrap(), expected);
    }
}
