//! Canonical, pre-observation producer for one partial m1.r33 serving diagnostic.

use ferric_m1_benchmarks::{
    encode_canonical_document, load_canonical_document_held, sha256_identity, BenchResult,
    SecureInputDirectory, SecureInputFile,
};
use rustix::fd::OwnedFd;
use rustix::fs::{
    fstat, fsync, inotify, mkdirat, openat2, renameat_with, unlinkat, AtFlags, Dir, FileType, Mode,
    OFlags, RenameFlags, ResolveFlags, Stat, CWD,
};
use rustix::process::{getegid, geteuid};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::mem::MaybeUninit;
use std::os::fd::AsRawFd;
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

pub(super) const COMMAND: &str = "partial-capture";

const EXPERIMENT_FORMAT: &str = "FERRIC-M1-R33-PARTIAL-EXPERIMENT-V1";
const EVENTS_FORMAT: &str = "FERRIC-M1-R33-PARTIAL-EVENTS-V1";
const CAPTURE_FORMAT: &str = "FERRIC-M1-R33-PARTIAL-CAPTURE-V1";
const PROTOCOL_FORMAT: &str = "FERRIC-M1-R33-PARTIAL-PROTOCOL-V1";
const INPUT_AUTHORITY: &str = "externally-admitted-m1-r33-partial-experiment-only";
const COMPANION_AUTHORITY: &str = "external-pre-observation-input-only";
const EVENTS_AUTHORITY: &str = "externally-collected-request-events-only";
const CAPTURE_AUTHORITY: &str = "ferric-computed-m1-r33-partial-diagnostic-only";
const STATUS: &str = "partial-non-evidence";
const TARGET: &str = "gfx942:xnack-";
const NONCLAIM: &str = "One fixed-roster Ferric externally declared target-load diagnostic computed from externally collected request events. This artifact is partial non-evidence: it does not establish a fresh server launch or server saturation, exercise continuous serving, compare measured vLLM or SGLang results, establish equal tuning, validate external policy, prove SLO compliance or hardware correctness, provide independent validation, or close m1.r33.";
const PROTOCOL_NONCLAIM: &str = "Partial externally declared target-load serving diagnostic protocol only. It is not continuous serving or qualification evidence, admits no baseline measurements or thresholds, cannot establish a fresh server launch or server saturation, and cannot establish baseline competitiveness, SLO compliance, hardware correctness, independent validation, or close m1.r33.";
const MAX_EVENTS: usize = 100_000;
const MAX_STRING_BYTES: usize = 256;
const FIXED_STARTS: u64 = 1;
const FIXED_WARMUP_WINDOWS: u64 = 1;
const FIXED_RECORDED_WINDOWS: u64 = 1;

const COMPANION_KINDS: &[&str] = &[
    "arrivals",
    "artifacts",
    "baselines",
    "environment",
    "model",
    "policy",
    "tuning",
    "workload",
];

const TIMING_TTFT: &str = "successful-request-arrival-to-first-output-token";
const TIMING_ITL: &str = "all-consecutive-output-token-intervals-from-successful-requests";
const TIMING_TPOT: &str = "floor-of-each-successful-request-first-to-last-output-token-nanoseconds-divided-by-output-token-count-minus-one";
const TIMING_E2E: &str = "successful-request-arrival-to-terminal-event";
const PERCENTILE_METHOD: &str = "nearest-rank-p50-p90-p99-over-the-declared-latency-population";
const RATE_INPUT_TOKENS: &str = "floor-of-successful-request-input-token-count-times-1e12-divided-by-exact-recorded-window-duration-nanoseconds";
const RATE_OUTPUT_TOKENS: &str = "floor-of-successful-request-output-token-count-times-1e12-divided-by-exact-recorded-window-duration-nanoseconds";
const RATE_REQUESTS: &str = "floor-of-all-recorded-request-count-including-failures-times-1e12-divided-by-exact-recorded-window-duration-nanoseconds";
const RATE_SUCCESSFUL_REQUESTS: &str = "floor-of-successful-recorded-request-count-times-1e12-divided-by-exact-recorded-window-duration-nanoseconds";
const RATE_TOTAL_TOKENS: &str = "floor-of-successful-request-input-plus-output-token-count-times-1e12-divided-by-exact-recorded-window-duration-nanoseconds";
const RATE_UNIT: &str = "integer-milli-units-per-second";
const TARGET_LOAD_PREDICATE: &str = "recorded-request-count-equals-offered-concurrency-and-peak-half-open-arrival-to-terminal-overlap-equals-offered-concurrency";

#[derive(Clone, Debug)]
struct CompanionSpec {
    bytes: u64,
    path: PathBuf,
    sha256: String,
}

#[derive(Debug)]
struct HeldInput {
    description: String,
    file: SecureInputFile,
    path: PathBuf,
}

#[derive(Debug)]
struct Experiment {
    case_id: String,
    companions: BTreeMap<String, CompanionSpec>,
    events_path: PathBuf,
}

#[derive(Clone, Debug)]
struct Workload {
    cell_id: String,
    input_tokens: u64,
    maximum_output_tokens: usize,
    offered_concurrency: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ScheduledRequest {
    arrival_offset_ns: u64,
    request_id: String,
}

#[derive(Clone, Debug)]
struct ScheduledWindow {
    end_offset_ns: u64,
    requests: Vec<ScheduledRequest>,
    start_offset_ns: u64,
}

#[derive(Clone, Debug)]
struct ArrivalRoster {
    recorded: ScheduledWindow,
    warmup: ScheduledWindow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Percentiles {
    p50: u64,
    p90: u64,
    p99: u64,
}

impl Percentiles {
    fn as_json(self) -> Value {
        json!({"p50": self.p50, "p90": self.p90, "p99": self.p99})
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Summary {
    end_to_end_ns: Percentiles,
    failures: u64,
    input_tokens: u64,
    input_tokens_per_second_milli: u64,
    itl_ns: Percentiles,
    output_tokens: u64,
    output_tokens_per_second_milli: u64,
    requests: u64,
    requests_per_second_milli: u64,
    successful_requests: u64,
    successful_requests_per_second_milli: u64,
    total_tokens: u64,
    total_tokens_per_second_milli: u64,
    tpot_ns: Percentiles,
    ttft_ns: Percentiles,
}

impl Summary {
    fn as_json(self) -> Value {
        json!({
            "failures": self.failures,
            "latency_ns": {
                "end_to_end": self.end_to_end_ns.as_json(),
                "itl": self.itl_ns.as_json(),
                "tpot": self.tpot_ns.as_json(),
                "ttft": self.ttft_ns.as_json(),
            },
            "rates_milli_per_second": {
                "input_tokens": self.input_tokens_per_second_milli,
                "output_tokens": self.output_tokens_per_second_milli,
                "requests": self.requests_per_second_milli,
                "successful_requests": self.successful_requests_per_second_milli,
                "total_tokens": self.total_tokens_per_second_milli,
            },
            "requests": self.requests,
            "successful_requests": self.successful_requests,
            "tokens": {
                "input": self.input_tokens,
                "output": self.output_tokens,
                "total": self.total_tokens,
            },
        })
    }
}

#[derive(Debug)]
struct StagedFile {
    file: File,
    name: OsString,
    snapshot: Stat,
}

#[derive(Debug)]
struct ExactBundle {
    armed: bool,
    files: Vec<StagedFile>,
    output_name: OsString,
    parent: OwnedFd,
    parent_snapshot: Stat,
    staging: OwnedFd,
    staging_name: OsString,
    staging_snapshot: Stat,
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
    let [experiment, output]: [OsString; 2] = arguments.try_into().map_err(|_| {
        "usage: ferric-m1-serving partial-capture EXPERIMENT OUTPUT-BUNDLE".to_owned()
    })?;
    produce(Path::new(&experiment), Path::new(&output))
}

fn produce(experiment_path: &Path, output: &Path) -> BenchResult<()> {
    require_protocol()?;
    let descriptor_name = experiment_path
        .file_name()
        .map(PathBuf::from)
        .ok_or_else(|| "experiment path has no file name".to_owned())?;
    let (root, descriptor, descriptor_bytes, descriptor_file) =
        load_canonical_document_held(experiment_path, "partial r33 experiment")?;
    let experiment = parse_experiment(&descriptor)?;
    let descriptor_sha256 = sha256_identity(&descriptor_bytes);

    let mut identities = BTreeSet::from([descriptor_file.identity()]);
    let mut held = vec![HeldInput {
        description: "partial r33 experiment".to_owned(),
        file: descriptor_file,
        path: descriptor_name,
    }];
    let mut payloads = BTreeMap::new();
    let mut companion_bindings = Map::new();
    for kind in COMPANION_KINDS {
        let spec = experiment
            .companions
            .get(*kind)
            .ok_or_else(|| format!("experiment companion is absent: {kind}"))?;
        let description = format!("partial r33 {kind} companion");
        let (value, bytes, file) = root.read_canonical_held(&spec.path, &description)?;
        if u64::try_from(bytes.len()).ok() != Some(spec.bytes)
            || sha256_identity(&bytes) != spec.sha256
        {
            return Err(format!("{description} identity drifted"));
        }
        if !identities.insert(file.identity()) {
            return Err("partial r33 input files must not alias each other".to_owned());
        }
        let payload = validate_companion(kind, &value, &experiment.case_id)?;
        payloads.insert((*kind).to_owned(), payload.clone());
        companion_bindings.insert(
            (*kind).to_owned(),
            json!({"bytes": spec.bytes, "path": path_string(&spec.path)?, "sha256": spec.sha256}),
        );
        held.push(HeldInput {
            description,
            file,
            path: spec.path.clone(),
        });
    }

    let workload = parse_workload(payload(&payloads, "workload")?, &experiment.case_id)?;
    let arrivals = parse_arrivals(payload(&payloads, "arrivals")?, &experiment.case_id)?;
    validate_external_inputs(&payloads)?;

    let (events, events_bytes, events_file) = root.read_canonical_held(
        &experiment.events_path,
        "partial r33 request event transcript",
    )?;
    if !identities.insert(events_file.identity()) {
        return Err("partial r33 event transcript aliases another input".to_owned());
    }
    held.push(HeldInput {
        description: "partial r33 request event transcript".to_owned(),
        file: events_file,
        path: experiment.events_path.clone(),
    });
    let summary = validate_events(
        &events,
        &experiment.case_id,
        &descriptor_sha256,
        &arrivals,
        &workload,
    )?;
    revalidate_inputs(&root, &held)?;

    let protocol = protocol_bytes()?;
    let capture = encode_canonical_document(&json!({
        "authority": CAPTURE_AUTHORITY,
        "case_id": experiment.case_id,
        "case_kind": "externally-declared-target-load",
        "companions": companion_bindings,
        "event_transcript": {
            "bytes": events_bytes.len(),
            "path": path_string(&experiment.events_path)?,
            "sha256": sha256_identity(&events_bytes),
        },
        "experiment_sha256": descriptor_sha256,
        "format": CAPTURE_FORMAT,
        "frozen_pre_observation_inputs": payloads,
        "nonclaim": NONCLAIM,
        "obligation_id": "m1.r33",
        "protocol_sha256": sha256_identity(&protocol),
        "roster": fixed_roster(),
        "status": STATUS,
        "summary": summary.as_json(),
        "target": TARGET,
        "timing_boundaries": timing_boundaries(),
        "workload_cell": workload.cell_id,
    }))?;
    validate_capture(&capture)?;

    let mut bundle = ExactBundle::create(output)?;
    bundle.write("capture.json", &capture)?;
    bundle.write("protocol.json", &protocol)?;
    bundle.publish_exact(
        &[("capture.json", &capture), ("protocol.json", &protocol)],
        || revalidate_inputs(&root, &held),
        || Ok(()),
    )
}

fn parse_experiment(value: &Value) -> BenchResult<Experiment> {
    let object = exact_object(
        value,
        &[
            "authority",
            "case_id",
            "case_kind",
            "companions",
            "event_transcript_path",
            "format",
            "nonclaim",
            "obligation_id",
            "protocol_sha256",
            "roster",
            "status",
            "target",
            "timing_boundaries",
        ],
        "partial r33 experiment",
    )?;
    expect_string(
        object,
        "authority",
        INPUT_AUTHORITY,
        "partial r33 experiment",
    )?;
    expect_string(
        object,
        "case_kind",
        "externally-declared-target-load",
        "partial r33 experiment",
    )?;
    expect_string(
        object,
        "format",
        EXPERIMENT_FORMAT,
        "partial r33 experiment",
    )?;
    expect_string(object, "nonclaim", NONCLAIM, "partial r33 experiment")?;
    expect_string(object, "obligation_id", "m1.r33", "partial r33 experiment")?;
    expect_string(
        object,
        "protocol_sha256",
        &sha256_identity(&protocol_bytes()?),
        "partial r33 experiment",
    )?;
    expect_string(
        object,
        "status",
        "pre-observation",
        "partial r33 experiment",
    )?;
    expect_string(object, "target", TARGET, "partial r33 experiment")?;
    if field(object, "roster", "partial r33 experiment")? != &fixed_roster()
        || field(object, "timing_boundaries", "partial r33 experiment")? != &timing_boundaries()
    {
        return Err("partial r33 fixed roster or timing boundaries drifted".to_owned());
    }
    let case_id = safe_string(
        field(object, "case_id", "partial r33 experiment")?,
        "case ID",
    )?;
    let companions = parse_companion_specs(field(object, "companions", "partial r33 experiment")?)?;
    let events_path = safe_path(
        field(object, "event_transcript_path", "partial r33 experiment")?,
        "event transcript",
    )?;
    if companions.values().any(|spec| spec.path == events_path) {
        return Err("event transcript path aliases a companion path".to_owned());
    }
    Ok(Experiment {
        case_id,
        companions,
        events_path,
    })
}

fn parse_companion_specs(value: &Value) -> BenchResult<BTreeMap<String, CompanionSpec>> {
    let object = value
        .as_object()
        .ok_or_else(|| "partial r33 companions must be an object".to_owned())?;
    exact_keys(object, COMPANION_KINDS, "partial r33 companions")?;
    let mut paths = BTreeSet::new();
    let mut specs = BTreeMap::new();
    for kind in COMPANION_KINDS {
        let entry = exact_object(
            field(object, kind, "partial r33 companions")?,
            &["bytes", "path", "sha256"],
            "partial r33 companion binding",
        )?;
        let bytes = positive_u64(
            field(entry, "bytes", "companion binding")?,
            "companion bytes",
        )?;
        let path = safe_path(field(entry, "path", "companion binding")?, "companion")?;
        let sha256 = sha_string(field(entry, "sha256", "companion binding")?)?;
        if !paths.insert(path.clone()) {
            return Err("partial r33 companion paths must be unique".to_owned());
        }
        specs.insert(
            (*kind).to_owned(),
            CompanionSpec {
                bytes,
                path,
                sha256,
            },
        );
    }
    Ok(specs)
}

fn validate_companion(kind: &str, value: &Value, case_id: &str) -> BenchResult<Value> {
    let object = exact_object(
        value,
        &["authority", "format", "kind", "payload", "status"],
        "partial r33 companion",
    )?;
    expect_string(
        object,
        "authority",
        COMPANION_AUTHORITY,
        "partial r33 companion",
    )?;
    expect_string(
        object,
        "format",
        &companion_format(kind),
        "partial r33 companion",
    )?;
    expect_string(object, "kind", kind, "partial r33 companion")?;
    expect_string(object, "status", "pre-observation", "partial r33 companion")?;
    let payload = field(object, "payload", "partial r33 companion")?.clone();
    match kind {
        "workload" => {
            parse_workload(&payload, case_id)?;
        }
        "arrivals" => {
            parse_arrivals(&payload, case_id)?;
        }
        "environment" => validate_identity_payload(
            &payload,
            &["environment_sha256", "hardware_sha256", "software_sha256"],
            "environment",
        )?,
        "model" => validate_identity_payload(
            &payload,
            &["model_sha256", "tokenizer_sha256", "weights_sha256"],
            "model",
        )?,
        "artifacts" => validate_identity_payload(
            &payload,
            &[
                "fe2o3_source_closure_sha256",
                "ferric_source_closure_sha256",
                "kernel_artifact_manifest_sha256",
                "runner_declaration_sha256",
            ],
            "artifact",
        )?,
        "tuning" => validate_identity_payload(
            &payload,
            &[
                "cache_policy_sha256",
                "ferric_config_sha256",
                "ferric_tuning_sha256",
                "sglang_tuning_sha256",
                "tuning_budget_sha256",
                "vllm_tuning_sha256",
            ],
            "tuning",
        )?,
        "baselines" => validate_baselines(&payload)?,
        "policy" => validate_policy(&payload)?,
        _ => return Err(format!("unsupported partial r33 companion: {kind}")),
    }
    Ok(payload)
}

fn parse_workload(value: &Value, case_id: &str) -> BenchResult<Workload> {
    let object = exact_object(
        value,
        &[
            "case_id",
            "cell_id",
            "input_tokens_per_request",
            "maximum_output_tokens_per_request",
            "offered_concurrency",
            "prefix_sharing",
            "prompt_roster_sha256",
            "sampling_seed",
            "target_mode",
        ],
        "partial r33 workload",
    )?;
    expect_string(object, "case_id", case_id, "partial r33 workload")?;
    expect_string(object, "target_mode", "target-only", "partial r33 workload")?;
    if field(object, "prefix_sharing", "partial r33 workload")?.as_bool() != Some(false) {
        return Err("partial r33 fixed workload requires prefix sharing disabled".to_owned());
    }
    let cell_id = safe_string(field(object, "cell_id", "partial r33 workload")?, "cell ID")?;
    let input_tokens = positive_u64(
        field(object, "input_tokens_per_request", "partial r33 workload")?,
        "input token count",
    )?;
    let maximum_output_tokens = usize::try_from(positive_u64(
        field(
            object,
            "maximum_output_tokens_per_request",
            "partial r33 workload",
        )?,
        "maximum output token count",
    )?)
    .map_err(|_| "maximum output token count does not fit this host".to_owned())?;
    if !(2..=MAX_EVENTS).contains(&maximum_output_tokens) {
        return Err("partial r33 output-token bound is outside the admitted range".to_owned());
    }
    let offered_concurrency = positive_u64(
        field(object, "offered_concurrency", "partial r33 workload")?,
        "offered concurrency",
    )?;
    if offered_concurrency < 2 {
        return Err("partial r33 target-load diagnostic requires concurrency above one".to_owned());
    }
    sha_string(field(
        object,
        "prompt_roster_sha256",
        "partial r33 workload",
    )?)?;
    field(object, "sampling_seed", "partial r33 workload")?
        .as_u64()
        .ok_or_else(|| "partial r33 sampling seed must be an unsigned integer".to_owned())?;
    Ok(Workload {
        cell_id,
        input_tokens,
        maximum_output_tokens,
        offered_concurrency,
    })
}

fn parse_arrivals(value: &Value, case_id: &str) -> BenchResult<ArrivalRoster> {
    let object = exact_object(
        value,
        &["case_id", "clock", "starts"],
        "partial r33 arrival roster",
    )?;
    expect_string(object, "case_id", case_id, "partial r33 arrival roster")?;
    expect_string(
        object,
        "clock",
        "monotonic-raw-nanoseconds",
        "partial r33 arrival roster",
    )?;
    let starts = field(object, "starts", "partial r33 arrival roster")?
        .as_array()
        .ok_or_else(|| "partial r33 starts must be an array".to_owned())?;
    if starts.len() != 1 {
        return Err("partial r33 diagnostic requires exactly one server start".to_owned());
    }
    let start = exact_object(
        &starts[0],
        &["recorded_windows", "start_index", "warmup_windows"],
        "partial r33 start roster",
    )?;
    if field(start, "start_index", "partial r33 start roster")?.as_u64() != Some(0) {
        return Err("partial r33 server-start index drifted".to_owned());
    }
    let warmups = field(start, "warmup_windows", "partial r33 start roster")?
        .as_array()
        .ok_or_else(|| "partial r33 warmup windows must be an array".to_owned())?;
    let recorded = field(start, "recorded_windows", "partial r33 start roster")?
        .as_array()
        .ok_or_else(|| "partial r33 recorded windows must be an array".to_owned())?;
    if warmups.len() != 1 || recorded.len() != 1 {
        return Err("partial r33 window roster drifted".to_owned());
    }
    let warmup = parse_scheduled_window(&warmups[0], 0, "warmup")?;
    let recorded = parse_scheduled_window(&recorded[0], 0, "recorded")?;
    if warmup.requests.is_empty()
        || recorded.requests.len() < 2
        || warmup.end_offset_ns > recorded.start_offset_ns
    {
        return Err("partial r33 warmup or target-load request roster is invalid".to_owned());
    }
    let mut request_ids = BTreeSet::new();
    for request in warmup.requests.iter().chain(&recorded.requests) {
        if !request_ids.insert(request.request_id.as_str()) {
            return Err("partial r33 request IDs must be globally unique".to_owned());
        }
    }
    Ok(ArrivalRoster { recorded, warmup })
}

fn parse_scheduled_window(
    value: &Value,
    expected_index: u64,
    phase: &str,
) -> BenchResult<ScheduledWindow> {
    let object = exact_object(
        value,
        &[
            "end_offset_ns",
            "requests",
            "start_offset_ns",
            "window_index",
        ],
        "partial r33 scheduled window",
    )?;
    if field(object, "window_index", "scheduled window")?.as_u64() != Some(expected_index) {
        return Err(format!("partial r33 {phase} window index drifted"));
    }
    let start_offset_ns = field(object, "start_offset_ns", "scheduled window")?
        .as_u64()
        .ok_or_else(|| "window start offset must be an unsigned integer".to_owned())?;
    let end_offset_ns = positive_u64(
        field(object, "end_offset_ns", "scheduled window")?,
        "window end offset",
    )?;
    if start_offset_ns >= end_offset_ns {
        return Err(format!("partial r33 {phase} window boundaries are invalid"));
    }
    let values = field(object, "requests", "scheduled window")?
        .as_array()
        .ok_or_else(|| "scheduled requests must be an array".to_owned())?;
    if values.len() > MAX_EVENTS {
        return Err("partial r33 scheduled request count exceeds the admitted bound".to_owned());
    }
    let mut requests = Vec::with_capacity(values.len());
    let mut previous = None;
    for value in values {
        let request = exact_object(
            value,
            &["arrival_offset_ns", "request_id"],
            "partial r33 scheduled request",
        )?;
        let arrival_offset_ns = field(request, "arrival_offset_ns", "scheduled request")?
            .as_u64()
            .ok_or_else(|| "arrival offset must be an unsigned integer".to_owned())?;
        if arrival_offset_ns < start_offset_ns
            || arrival_offset_ns >= end_offset_ns
            || previous.is_some_and(|prior| arrival_offset_ns < prior)
        {
            return Err(format!(
                "partial r33 {phase} arrival order or boundary drifted"
            ));
        }
        previous = Some(arrival_offset_ns);
        requests.push(ScheduledRequest {
            arrival_offset_ns,
            request_id: safe_string(
                field(request, "request_id", "scheduled request")?,
                "request ID",
            )?,
        });
    }
    Ok(ScheduledWindow {
        end_offset_ns,
        requests,
        start_offset_ns,
    })
}

fn validate_external_inputs(payloads: &BTreeMap<String, Value>) -> BenchResult<()> {
    for name in COMPANION_KINDS {
        if !payloads.contains_key(*name) {
            return Err(format!("partial r33 input is absent: {name}"));
        }
    }
    Ok(())
}

fn validate_identity_payload(value: &Value, names: &[&str], description: &str) -> BenchResult<()> {
    let object = exact_object(value, names, &format!("partial r33 {description} input"))?;
    for name in names {
        sha_string(field(object, name, description)?)?;
    }
    Ok(())
}

fn validate_baselines(value: &Value) -> BenchResult<()> {
    let object = exact_object(value, &["sglang", "vllm"], "partial r33 baseline input")?;
    for engine in ["sglang", "vllm"] {
        let baseline = exact_object(
            field(object, engine, "baseline input")?,
            &["config_sha256", "implementation_sha256", "version"],
            "partial r33 pinned baseline",
        )?;
        sha_string(field(baseline, "config_sha256", "pinned baseline")?)?;
        sha_string(field(baseline, "implementation_sha256", "pinned baseline")?)?;
        bounded_ascii_string(
            field(baseline, "version", "pinned baseline")?,
            "baseline version",
        )?;
    }
    Ok(())
}

fn validate_policy(value: &Value) -> BenchResult<()> {
    let object = exact_object(
        value,
        &[
            "itl_p99_slo_ns",
            "policy_identity_sha256",
            "ttft_p99_slo_ns",
        ],
        "partial r33 external policy",
    )?;
    positive_u64(
        field(object, "itl_p99_slo_ns", "external policy")?,
        "ITL p99 SLO",
    )?;
    positive_u64(
        field(object, "ttft_p99_slo_ns", "external policy")?,
        "TTFT p99 SLO",
    )?;
    sha_string(field(object, "policy_identity_sha256", "external policy")?)?;
    Ok(())
}

fn validate_events(
    value: &Value,
    case_id: &str,
    experiment_sha256: &str,
    arrivals: &ArrivalRoster,
    workload: &Workload,
) -> BenchResult<Summary> {
    let object = exact_object(
        value,
        &[
            "authority",
            "case_id",
            "experiment_sha256",
            "format",
            "starts",
            "status",
            "target",
        ],
        "partial r33 event transcript",
    )?;
    expect_string(object, "authority", EVENTS_AUTHORITY, "event transcript")?;
    expect_string(object, "case_id", case_id, "event transcript")?;
    expect_string(
        object,
        "experiment_sha256",
        experiment_sha256,
        "event transcript",
    )?;
    expect_string(object, "format", EVENTS_FORMAT, "event transcript")?;
    expect_string(
        object,
        "status",
        "collected-unvalidated",
        "event transcript",
    )?;
    expect_string(object, "target", TARGET, "event transcript")?;
    let starts = field(object, "starts", "event transcript")?
        .as_array()
        .ok_or_else(|| "event transcript starts must be an array".to_owned())?;
    if starts.len() != 1 {
        return Err("event transcript server-start roster drifted".to_owned());
    }
    let start = exact_object(
        &starts[0],
        &[
            "recorded_windows",
            "start_index",
            "start_time_ns",
            "warmup_windows",
        ],
        "partial r33 event start",
    )?;
    if field(start, "start_index", "event start")?.as_u64() != Some(0) {
        return Err("event transcript start index drifted".to_owned());
    }
    let start_time_ns = positive_u64(
        field(start, "start_time_ns", "event start")?,
        "server start time",
    )?;
    let warmups = field(start, "warmup_windows", "event start")?
        .as_array()
        .ok_or_else(|| "event warmup windows must be an array".to_owned())?;
    let recorded = field(start, "recorded_windows", "event start")?
        .as_array()
        .ok_or_else(|| "event recorded windows must be an array".to_owned())?;
    if warmups.len() != 1 || recorded.len() != 1 {
        return Err("event transcript window roster drifted".to_owned());
    }
    validate_raw_window(
        &warmups[0],
        &arrivals.warmup,
        start_time_ns,
        workload,
        false,
    )?;
    let summary = validate_raw_window(
        &recorded[0],
        &arrivals.recorded,
        start_time_ns,
        workload,
        true,
    )?
    .ok_or_else(|| "recorded event window did not produce a summary".to_owned())?;
    Ok(summary)
}

fn validate_raw_window(
    value: &Value,
    scheduled: &ScheduledWindow,
    start_time_ns: u64,
    workload: &Workload,
    recorded: bool,
) -> BenchResult<Option<Summary>> {
    let expected_keys = if recorded {
        &["end_ns", "requests", "start_ns", "summary", "window_index"][..]
    } else {
        &["end_ns", "requests", "start_ns", "window_index"][..]
    };
    let object = exact_object(value, expected_keys, "partial r33 raw window")?;
    if field(object, "window_index", "raw window")?.as_u64() != Some(0) {
        return Err("raw event window index drifted".to_owned());
    }
    let expected_start = start_time_ns
        .checked_add(scheduled.start_offset_ns)
        .ok_or_else(|| "raw window start overflowed".to_owned())?;
    let expected_end = start_time_ns
        .checked_add(scheduled.end_offset_ns)
        .ok_or_else(|| "raw window end overflowed".to_owned())?;
    if field(object, "start_ns", "raw window")?.as_u64() != Some(expected_start)
        || field(object, "end_ns", "raw window")?.as_u64() != Some(expected_end)
    {
        return Err("raw event window timing boundaries drifted".to_owned());
    }
    let requests = field(object, "requests", "raw window")?
        .as_array()
        .ok_or_else(|| "raw window requests must be an array".to_owned())?;
    if requests.len() != scheduled.requests.len() {
        return Err("raw event request roster length drifted".to_owned());
    }
    if recorded && u64::try_from(requests.len()).ok() != Some(workload.offered_concurrency) {
        return Err(
            "recorded request count differs from externally declared offered concurrency"
                .to_owned(),
        );
    }

    let mut ttft = Vec::new();
    let mut itl = Vec::new();
    let mut tpot = Vec::new();
    let mut end_to_end = Vec::new();
    let mut failures = 0_u64;
    let mut successes = 0_u64;
    let mut input_tokens = 0_u64;
    let mut output_tokens = 0_u64;
    let mut request_intervals = Vec::with_capacity(requests.len());
    for (raw, expected) in requests.iter().zip(&scheduled.requests) {
        let observation = validate_request(raw, expected, start_time_ns, expected_end, workload)?;
        request_intervals.push((observation.arrival_ns, observation.end_ns));
        if let Some(latencies) = observation.latencies {
            successes = successes
                .checked_add(1)
                .ok_or_else(|| "successful request count overflowed".to_owned())?;
            input_tokens = input_tokens
                .checked_add(workload.input_tokens)
                .ok_or_else(|| "input token count overflowed".to_owned())?;
            output_tokens = output_tokens
                .checked_add(latencies.output_tokens)
                .ok_or_else(|| "output token count overflowed".to_owned())?;
            ttft.push(latencies.ttft);
            itl.extend(latencies.itl);
            tpot.push(latencies.tpot);
            end_to_end.push(latencies.end_to_end);
        } else {
            failures = failures
                .checked_add(1)
                .ok_or_else(|| "failure count overflowed".to_owned())?;
        }
    }
    if !recorded {
        return Ok(None);
    }
    if peak_half_open_overlap(&request_intervals)? != workload.offered_concurrency {
        return Err(
            "recorded request intervals do not realize the externally declared target load"
                .to_owned(),
        );
    }
    if successes == 0 || itl.is_empty() {
        return Err("recorded window has no complete latency population".to_owned());
    }
    let requests =
        u64::try_from(requests.len()).map_err(|_| "request count does not fit u64".to_owned())?;
    let total_tokens = input_tokens
        .checked_add(output_tokens)
        .ok_or_else(|| "total token count overflowed".to_owned())?;
    let duration = expected_end
        .checked_sub(expected_start)
        .ok_or_else(|| "recorded window duration underflowed".to_owned())?;
    let summary = Summary {
        end_to_end_ns: percentiles(&mut end_to_end)?,
        failures,
        input_tokens,
        input_tokens_per_second_milli: rate_milli(input_tokens, duration)?,
        itl_ns: percentiles(&mut itl)?,
        output_tokens,
        output_tokens_per_second_milli: rate_milli(output_tokens, duration)?,
        requests,
        requests_per_second_milli: rate_milli(requests, duration)?,
        successful_requests: successes,
        successful_requests_per_second_milli: rate_milli(successes, duration)?,
        total_tokens,
        total_tokens_per_second_milli: rate_milli(total_tokens, duration)?,
        tpot_ns: percentiles(&mut tpot)?,
        ttft_ns: percentiles(&mut ttft)?,
    };
    if field(object, "summary", "recorded raw window")? != &summary.as_json() {
        return Err("reported raw summary differs from recomputed request events".to_owned());
    }
    Ok(Some(summary))
}

struct RequestLatencies {
    end_to_end: u64,
    itl: Vec<u64>,
    output_tokens: u64,
    tpot: u64,
    ttft: u64,
}

struct RequestObservation {
    arrival_ns: u64,
    end_ns: u64,
    latencies: Option<RequestLatencies>,
}

fn validate_request(
    value: &Value,
    expected: &ScheduledRequest,
    start_time_ns: u64,
    window_end_ns: u64,
    workload: &Workload,
) -> BenchResult<RequestObservation> {
    let object = exact_object(
        value,
        &[
            "arrival_ns",
            "end_ns",
            "outcome",
            "output_token_timestamps_ns",
            "prompt_tokens",
            "request_id",
        ],
        "partial r33 request event",
    )?;
    expect_string(object, "request_id", &expected.request_id, "request event")?;
    let arrival = start_time_ns
        .checked_add(expected.arrival_offset_ns)
        .ok_or_else(|| "request arrival time overflowed".to_owned())?;
    if field(object, "arrival_ns", "request event")?.as_u64() != Some(arrival) {
        return Err("request arrival time or order drifted".to_owned());
    }
    if field(object, "prompt_tokens", "request event")?.as_u64() != Some(workload.input_tokens) {
        return Err("request prompt-token count drifted from the workload".to_owned());
    }
    let end = field(object, "end_ns", "request event")?
        .as_u64()
        .ok_or_else(|| "request end time must be an unsigned integer".to_owned())?;
    if end <= arrival || end > window_end_ns {
        return Err("request terminal time is outside its admitted boundary".to_owned());
    }
    let timestamps = field(object, "output_token_timestamps_ns", "request event")?
        .as_array()
        .ok_or_else(|| "output token timestamps must be an array".to_owned())?;
    let outcome = field(object, "outcome", "request event")?
        .as_str()
        .ok_or_else(|| "request outcome must be a string".to_owned())?;
    if outcome == "failed" {
        if !timestamps.is_empty() {
            return Err("failed request must not report successful output tokens".to_owned());
        }
        return Ok(RequestObservation {
            arrival_ns: arrival,
            end_ns: end,
            latencies: None,
        });
    }
    if outcome != "completed"
        || timestamps.len() < 2
        || timestamps.len() > workload.maximum_output_tokens
    {
        return Err("completed request output-token roster is invalid".to_owned());
    }
    let mut parsed = Vec::with_capacity(timestamps.len());
    for timestamp in timestamps {
        let timestamp = timestamp
            .as_u64()
            .ok_or_else(|| "output token time must be an unsigned integer".to_owned())?;
        if timestamp < arrival
            || timestamp > end
            || parsed.last().is_some_and(|previous| timestamp <= *previous)
        {
            return Err("output token timestamps are nonmonotonic or out of bounds".to_owned());
        }
        parsed.push(timestamp);
    }
    let first = parsed[0];
    let last = *parsed
        .last()
        .ok_or_else(|| "completed request lost its token timestamps".to_owned())?;
    let mut intervals = Vec::with_capacity(parsed.len().saturating_sub(1));
    for pair in parsed.windows(2) {
        intervals.push(pair[1] - pair[0]);
    }
    let denominator = u64::try_from(parsed.len() - 1)
        .map_err(|_| "TPOT denominator does not fit u64".to_owned())?;
    Ok(RequestObservation {
        arrival_ns: arrival,
        end_ns: end,
        latencies: Some(RequestLatencies {
            end_to_end: end - arrival,
            itl: intervals,
            output_tokens: u64::try_from(parsed.len())
                .map_err(|_| "output token count does not fit u64".to_owned())?,
            tpot: (last - first) / denominator,
            ttft: first - arrival,
        }),
    })
}

fn peak_half_open_overlap(intervals: &[(u64, u64)]) -> BenchResult<u64> {
    let mut transitions = BTreeMap::<u64, (u64, u64)>::new();
    for &(arrival, end) in intervals {
        if arrival >= end {
            return Err("request interval is empty or reversed".to_owned());
        }
        transitions
            .entry(arrival)
            .and_modify(|counts| counts.1 += 1)
            .or_insert((0, 1));
        transitions
            .entry(end)
            .and_modify(|counts| counts.0 += 1)
            .or_insert((1, 0));
    }
    let mut active = 0_u64;
    let mut peak = 0_u64;
    for (departures, arrivals) in transitions.into_values() {
        active = active
            .checked_sub(departures)
            .ok_or_else(|| "request overlap departures exceed active requests".to_owned())?;
        active = active
            .checked_add(arrivals)
            .ok_or_else(|| "request overlap count overflowed".to_owned())?;
        peak = peak.max(active);
    }
    if active != 0 {
        return Err("request overlap accounting did not settle".to_owned());
    }
    Ok(peak)
}

fn percentiles(values: &mut [u64]) -> BenchResult<Percentiles> {
    if values.is_empty() {
        return Err("cannot compute a percentile from an empty population".to_owned());
    }
    values.sort_unstable();
    Ok(Percentiles {
        p50: nearest_rank(values, 50)?,
        p90: nearest_rank(values, 90)?,
        p99: nearest_rank(values, 99)?,
    })
}

fn nearest_rank(sorted: &[u64], percentile: usize) -> BenchResult<u64> {
    let numerator = sorted
        .len()
        .checked_mul(percentile)
        .and_then(|value| value.checked_add(99))
        .ok_or_else(|| "percentile rank overflowed".to_owned())?;
    let rank = numerator / 100;
    sorted
        .get(rank.saturating_sub(1))
        .copied()
        .ok_or_else(|| "percentile rank is outside the population".to_owned())
}

fn rate_milli(count: u64, duration_ns: u64) -> BenchResult<u64> {
    if duration_ns == 0 {
        return Err("cannot compute a rate over zero time".to_owned());
    }
    let scaled = u128::from(count)
        .checked_mul(1_000_000_000_000)
        .ok_or_else(|| "serving rate numerator overflowed".to_owned())?
        / u128::from(duration_ns);
    u64::try_from(scaled).map_err(|_| "serving rate does not fit u64".to_owned())
}

fn validate_capture(bytes: &[u8]) -> BenchResult<()> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|error| format!("cannot parse partial r33 capture: {error}"))?;
    if encode_canonical_document(&value)? != bytes {
        return Err("partial r33 capture is not canonical JSON".to_owned());
    }
    let object = exact_object(
        &value,
        &[
            "authority",
            "case_id",
            "case_kind",
            "companions",
            "event_transcript",
            "experiment_sha256",
            "format",
            "frozen_pre_observation_inputs",
            "nonclaim",
            "obligation_id",
            "protocol_sha256",
            "roster",
            "status",
            "summary",
            "target",
            "timing_boundaries",
            "workload_cell",
        ],
        "partial r33 capture",
    )?;
    expect_string(
        object,
        "authority",
        CAPTURE_AUTHORITY,
        "partial r33 capture",
    )?;
    expect_string(
        object,
        "case_kind",
        "externally-declared-target-load",
        "partial r33 capture",
    )?;
    expect_string(object, "format", CAPTURE_FORMAT, "partial r33 capture")?;
    expect_string(object, "nonclaim", NONCLAIM, "partial r33 capture")?;
    expect_string(object, "obligation_id", "m1.r33", "partial r33 capture")?;
    expect_string(object, "status", STATUS, "partial r33 capture")?;
    expect_string(object, "target", TARGET, "partial r33 capture")?;
    sha_string(field(object, "experiment_sha256", "partial r33 capture")?)?;
    sha_string(field(object, "protocol_sha256", "partial r33 capture")?)?;
    Ok(())
}

fn revalidate_inputs(root: &SecureInputDirectory, inputs: &[HeldInput]) -> BenchResult<()> {
    for input in inputs {
        root.validate_binding(&input.path, &input.file, &input.description)?;
    }
    Ok(())
}

fn require_protocol() -> BenchResult<()> {
    let bytes = protocol_bytes()?;
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("cannot parse partial r33 protocol: {error}"))?;
    if encode_canonical_document(&value)? != bytes {
        return Err("partial r33 protocol is not canonical JSON".to_owned());
    }
    let object = exact_object(
        &value,
        &[
            "authority",
            "bundle_files",
            "companion_kinds",
            "format",
            "metric_derivation",
            "milestone",
            "nonclaim",
            "obligation_id",
            "roster",
            "status",
            "target",
        ],
        "partial r33 protocol",
    )?;
    expect_string(
        object,
        "authority",
        "ferric-m1-r33-partial-protocol-only",
        "partial r33 protocol",
    )?;
    expect_string(object, "format", PROTOCOL_FORMAT, "partial r33 protocol")?;
    expect_string(object, "milestone", "M1", "partial r33 protocol")?;
    expect_string(
        object,
        "nonclaim",
        PROTOCOL_NONCLAIM,
        "partial r33 protocol",
    )?;
    expect_string(object, "obligation_id", "m1.r33", "partial r33 protocol")?;
    expect_string(object, "status", STATUS, "partial r33 protocol")?;
    expect_string(object, "target", TARGET, "partial r33 protocol")?;
    if field(object, "bundle_files", "partial r33 protocol")?
        != &json!(["capture.json", "protocol.json"])
        || field(object, "companion_kinds", "partial r33 protocol")? != &json!(COMPANION_KINDS)
        || field(object, "roster", "partial r33 protocol")? != &fixed_roster()
        || field(object, "metric_derivation", "partial r33 protocol")? != &timing_boundaries()
    {
        return Err("partial r33 protocol roster or metric derivation drifted".to_owned());
    }
    Ok(())
}

fn protocol_bytes() -> BenchResult<Vec<u8>> {
    encode_canonical_document(&json!({
        "authority": "ferric-m1-r33-partial-protocol-only",
        "bundle_files": ["capture.json", "protocol.json"],
        "companion_kinds": COMPANION_KINDS,
        "format": PROTOCOL_FORMAT,
        "metric_derivation": timing_boundaries(),
        "milestone": "M1",
        "nonclaim": PROTOCOL_NONCLAIM,
        "obligation_id": "m1.r33",
        "roster": fixed_roster(),
        "status": STATUS,
        "target": TARGET,
    }))
}

fn fixed_roster() -> Value {
    json!({
        "case_kinds": ["externally-declared-target-load"],
        "recorded_windows_per_start": FIXED_RECORDED_WINDOWS,
        "server_starts": FIXED_STARTS,
        "warmup_windows_per_start": FIXED_WARMUP_WINDOWS,
    })
}

fn timing_boundaries() -> Value {
    json!({
        "end_to_end": TIMING_E2E,
        "itl": TIMING_ITL,
        "percentiles": PERCENTILE_METHOD,
        "rates_milli_per_second": {
            "input_tokens": RATE_INPUT_TOKENS,
            "output_tokens": RATE_OUTPUT_TOKENS,
            "requests": RATE_REQUESTS,
            "successful_requests": RATE_SUCCESSFUL_REQUESTS,
            "total_tokens": RATE_TOTAL_TOKENS,
            "unit": RATE_UNIT,
        },
        "target_load_predicate": TARGET_LOAD_PREDICATE,
        "tpot": TIMING_TPOT,
        "ttft": TIMING_TTFT,
    })
}

fn companion_format(kind: &str) -> String {
    format!("FERRIC-M1-R33-PARTIAL-{}-V1", kind.to_ascii_uppercase())
}

fn payload<'a>(payloads: &'a BTreeMap<String, Value>, kind: &str) -> BenchResult<&'a Value> {
    payloads
        .get(kind)
        .ok_or_else(|| format!("partial r33 payload is absent: {kind}"))
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
    if object.len() != expected.len() || !expected.iter().all(|name| object.contains_key(*name)) {
        return Err(format!("{description} fields drifted"));
    }
    Ok(())
}

fn field<'a>(
    object: &'a Map<String, Value>,
    name: &str,
    description: &str,
) -> BenchResult<&'a Value> {
    object
        .get(name)
        .ok_or_else(|| format!("{description} is missing {name}"))
}

fn expect_string(
    object: &Map<String, Value>,
    name: &str,
    expected: &str,
    description: &str,
) -> BenchResult<()> {
    if field(object, name, description)?.as_str() != Some(expected) {
        return Err(format!("{description} {name} drifted"));
    }
    Ok(())
}

fn safe_string(value: &Value, description: &str) -> BenchResult<String> {
    let string = value
        .as_str()
        .ok_or_else(|| format!("{description} must be a string"))?;
    if string.is_empty()
        || string.len() > MAX_STRING_BYTES
        || !string.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b".-_".contains(&byte)
        })
    {
        return Err(format!("invalid {description}: {string}"));
    }
    Ok(string.to_owned())
}

fn bounded_ascii_string(value: &Value, description: &str) -> BenchResult<String> {
    let string = value
        .as_str()
        .ok_or_else(|| format!("{description} must be a string"))?;
    if string.is_empty()
        || string.len() > MAX_STRING_BYTES
        || !string.is_ascii()
        || string.bytes().any(|byte| byte.is_ascii_control())
    {
        return Err(format!("invalid {description}"));
    }
    Ok(string.to_owned())
}

fn sha_string(value: &Value) -> BenchResult<String> {
    let identity = value
        .as_str()
        .ok_or_else(|| "SHA-256 identity must be a string".to_owned())?;
    if identity.len() != 64
        || !identity
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || identity.bytes().all(|byte| byte == identity.as_bytes()[0])
    {
        return Err("invalid SHA-256 identity".to_owned());
    }
    Ok(identity.to_owned())
}

fn positive_u64(value: &Value, description: &str) -> BenchResult<u64> {
    let value = value
        .as_u64()
        .ok_or_else(|| format!("{description} must be an unsigned integer"))?;
    if value == 0 {
        return Err(format!("{description} must be positive"));
    }
    Ok(value)
}

fn safe_path(value: &Value, description: &str) -> BenchResult<PathBuf> {
    let path = PathBuf::from(
        value
            .as_str()
            .ok_or_else(|| format!("{description} path must be a string"))?,
    );
    require_relative(&path, description)?;
    Ok(path)
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

fn path_string(path: &Path) -> BenchResult<&str> {
    path.to_str()
        .filter(|value| value.is_ascii())
        .ok_or_else(|| "partial r33 path must be ASCII UTF-8".to_owned())
}

impl ExactBundle {
    fn create(output: &Path) -> BenchResult<Self> {
        Self::create_with_after_mkdir(output, |_| Ok(()))
    }

    fn create_with_after_mkdir(
        output: &Path,
        after_mkdir: impl FnOnce(&Path) -> BenchResult<()>,
    ) -> BenchResult<Self> {
        let output_name = output
            .file_name()
            .map(OsString::from)
            .ok_or_else(|| "output bundle path has no final component".to_owned())?;
        let parent_path = output
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let parent = openat2(
            CWD,
            parent_path,
            OFlags::RDONLY
                | OFlags::DIRECTORY
                | OFlags::NOFOLLOW
                | OFlags::NONBLOCK
                | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )
        .map_err(|error| format!("cannot securely open output parent: {error}"))?;
        let parent_stat =
            fstat(&parent).map_err(|error| format!("cannot inspect output parent: {error}"))?;
        validate_controlled_directory(&parent_stat, "output parent")?;
        if path_exists_at(&parent, &output_name)? {
            return Err("output bundle already exists".to_owned());
        }
        let mut after_mkdir = Some(after_mkdir);
        for nonce in 0..1_024_u16 {
            let mut staging_name = OsString::from(".");
            staging_name.push(&output_name);
            staging_name.push(format!(".staging.{}.{nonce}", std::process::id()));
            match mkdirat(
                &parent,
                staging_name.as_os_str(),
                Mode::RUSR | Mode::WUSR | Mode::XUSR,
            ) {
                Ok(()) => {
                    let staging_path = parent_path.join(&staging_name);
                    after_mkdir
                        .take()
                        .ok_or_else(|| "partial r33 mkdir hook was already consumed".to_owned())?(
                        &staging_path,
                    )?;
                    let staging = open_directory_at(&parent, Path::new(&staging_name), "staging")?;
                    let staging_snapshot = fstat(&staging)
                        .map_err(|error| format!("cannot inspect staging output: {error}"))?;
                    validate_adopted_directory(&staging_snapshot, "staging output")?;
                    if !directory_roster(&staging, "newly adopted staging")?.is_empty() {
                        return Err("newly adopted staging output must be exactly empty".to_owned());
                    }
                    let parent_snapshot = fstat(&parent).map_err(|error| {
                        format!("cannot reinspect output parent after staging creation: {error}")
                    })?;
                    validate_controlled_directory(&parent_snapshot, "output parent")?;
                    return Ok(Self {
                        armed: true,
                        files: Vec::new(),
                        output_name,
                        parent,
                        parent_snapshot,
                        staging,
                        staging_name,
                        staging_snapshot,
                    });
                }
                Err(error) if error == rustix::io::Errno::EXIST => {}
                Err(error) => return Err(format!("cannot create staging output: {error}")),
            }
        }
        Err("staging output namespace was exhausted".to_owned())
    }

    fn write(&mut self, name: &str, bytes: &[u8]) -> BenchResult<()> {
        let name = OsString::from(name);
        let descriptor = openat2(
            &self.staging,
            Path::new(&name),
            OFlags::RDWR
                | OFlags::CREATE
                | OFlags::EXCL
                | OFlags::NOFOLLOW
                | OFlags::NONBLOCK
                | OFlags::CLOEXEC,
            Mode::RUSR | Mode::WUSR,
            ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )
        .map_err(|error| format!("cannot create staged output {}: {error}", name.display()))?;
        let file = File::from(descriptor);
        let created = fstat(&file)
            .map_err(|error| format!("cannot inspect staged output {}: {error}", name.display()))?;
        if FileType::from_raw_mode(created.st_mode) != FileType::RegularFile
            || created.st_nlink != 1
        {
            return Err(format!(
                "created staged output must be a one-link regular file: {}",
                name.display()
            ));
        }
        self.files.push(StagedFile {
            file,
            name: name.clone(),
            snapshot: created,
        });
        let staged = self
            .files
            .last_mut()
            .ok_or_else(|| "staged output record disappeared during creation".to_owned())?;
        staged
            .file
            .write_all(bytes)
            .map_err(|error| format!("cannot write staged output {}: {error}", name.display()))?;
        staged
            .file
            .sync_all()
            .map_err(|error| format!("cannot sync staged output {}: {error}", name.display()))?;
        let snapshot = fstat(&staged.file)
            .map_err(|error| format!("cannot inspect staged output {}: {error}", name.display()))?;
        if !same_file_identity(&created, &snapshot)
            || FileType::from_raw_mode(snapshot.st_mode) != FileType::RegularFile
            || snapshot.st_nlink != 1
            || usize::try_from(snapshot.st_size).ok() != Some(bytes.len())
        {
            return Err(format!(
                "staged output metadata is invalid: {}",
                name.display()
            ));
        }
        staged.snapshot = snapshot;
        verify_held_file(staged, bytes, "written staged")?;
        Ok(())
    }

    fn publish_exact(
        mut self,
        expected: &[(&str, &[u8])],
        pre_publish: impl FnOnce() -> BenchResult<()>,
        after_published_verification: impl FnOnce() -> BenchResult<()>,
    ) -> BenchResult<()> {
        let expected_names = expected
            .iter()
            .map(|(name, _)| OsString::from(name))
            .collect::<Vec<_>>();
        if self.files.iter().map(|file| &file.name).ne(&expected_names) {
            return Err("staged output roster differs from the exact protocol".to_owned());
        }
        let staged = self.rebind_directory_identity(self.staging_name.as_os_str(), "staged")?;
        self.verify_exact_files(&staged, expected, "staged")?;
        fsync(&self.staging).map_err(|error| format!("cannot sync staging directory: {error}"))?;
        let settled_staging = fstat(&self.staging)
            .map_err(|error| format!("cannot snapshot settled staging directory: {error}"))?;
        validate_adopted_directory(&settled_staging, "settled staging output")?;
        self.staging_snapshot = settled_staging;
        self.rebind_directory_snapshot(
            self.staging_name.as_os_str(),
            &settled_staging,
            "settled staged",
        )?;
        let mutation_watch =
            inotify::init(inotify::CreateFlags::CLOEXEC | inotify::CreateFlags::NONBLOCK)
                .map_err(|error| format!("cannot create staging mutation watch: {error}"))?;
        let held_staging_path = format!("/proc/self/fd/{}", self.staging.as_raw_fd());
        inotify::add_watch(
            &mutation_watch,
            held_staging_path,
            inotify::WatchFlags::ATTRIB
                | inotify::WatchFlags::CLOSE_WRITE
                | inotify::WatchFlags::CREATE
                | inotify::WatchFlags::DELETE
                | inotify::WatchFlags::DELETE_SELF
                | inotify::WatchFlags::MODIFY
                | inotify::WatchFlags::MOVE_SELF
                | inotify::WatchFlags::MOVED_FROM
                | inotify::WatchFlags::MOVED_TO
                | inotify::WatchFlags::ONLYDIR,
        )
        .map_err(|error| format!("cannot watch held staging directory: {error}"))?;
        pre_publish()?;
        let mut mutation_events = [MaybeUninit::uninit(); 4096];
        match inotify::Reader::new(&mutation_watch, &mut mutation_events).next() {
            Err(rustix::io::Errno::AGAIN) => {}
            Err(error) => {
                return Err(format!("cannot inspect staging mutation watch: {error}"));
            }
            Ok(_) => return Err("staging output changed during input revalidation".to_owned()),
        }
        fsync(&self.staging)
            .map_err(|error| format!("cannot resync staging directory: {error}"))?;
        let final_staged = self.rebind_directory_snapshot(
            self.staging_name.as_os_str(),
            &settled_staging,
            "final pre-publication",
        )?;
        self.verify_exact_files(&final_staged, expected, "final pre-publication")?;
        self.validate_parent_snapshot(&self.parent_snapshot, "pre-publication")?;
        renameat_with(
            &self.parent,
            self.staging_name.as_os_str(),
            &self.parent,
            self.output_name.as_os_str(),
            RenameFlags::NOREPLACE,
        )
        .map_err(|error| format!("cannot publish output without replacement: {error}"))?;
        self.armed = false;
        let published =
            self.rebind_directory_identity(self.output_name.as_os_str(), "published")?;
        let published_snapshot = fstat(&published)
            .map_err(|error| format!("cannot snapshot published output directory: {error}"))?;
        validate_adopted_directory(&published_snapshot, "published output")?;
        self.verify_exact_files(&published, expected, "published")?;
        after_published_verification()?;
        let published_parent = fstat(&self.parent)
            .map_err(|error| format!("cannot snapshot published output parent: {error}"))?;
        validate_directory_transition(
            &self.parent_snapshot,
            &published_parent,
            "published output parent",
        )?;
        fsync(&self.parent)
            .map_err(|error| format!("cannot sync published output parent: {error}"))?;
        self.validate_parent_snapshot(&published_parent, "post-fsync published")?;
        let final_binding = self.rebind_directory_snapshot(
            self.output_name.as_os_str(),
            &published_snapshot,
            "final published",
        )?;
        self.verify_exact_files(&final_binding, expected, "final published")?;
        self.validate_parent_snapshot(&published_parent, "final published")?;
        let final_binding = self.rebind_directory_snapshot(
            self.output_name.as_os_str(),
            &published_snapshot,
            "final rebound published",
        )?;
        self.verify_exact_files(&final_binding, expected, "final rebound published")?;
        Ok(())
    }

    fn rebind_directory_identity(&self, name: &OsStr, phase: &str) -> BenchResult<OwnedFd> {
        let reopened = open_directory_at(&self.parent, Path::new(name), phase)?;
        let held = fstat(&self.staging)
            .map_err(|error| format!("cannot inspect held staging directory: {error}"))?;
        let rebound = fstat(&reopened)
            .map_err(|error| format!("cannot inspect rebound {phase} directory: {error}"))?;
        if !same_directory_identity(&self.staging_snapshot, &held)
            || !same_directory_identity(&self.staging_snapshot, &rebound)
        {
            return Err(format!(
                "{phase} output name does not bind the held directory"
            ));
        }
        Ok(reopened)
    }

    fn rebind_directory_snapshot(
        &self,
        name: &OsStr,
        expected: &Stat,
        phase: &str,
    ) -> BenchResult<OwnedFd> {
        let reopened = open_directory_at(&self.parent, Path::new(name), phase)?;
        let held = fstat(&self.staging)
            .map_err(|error| format!("cannot inspect held {phase} directory: {error}"))?;
        let rebound = fstat(&reopened)
            .map_err(|error| format!("cannot inspect rebound {phase} directory: {error}"))?;
        if !same_directory_snapshot(expected, &held) || !same_directory_snapshot(expected, &rebound)
        {
            return Err(format!("{phase} output name or directory metadata changed"));
        }
        Ok(reopened)
    }

    fn validate_parent_snapshot(&self, expected: &Stat, phase: &str) -> BenchResult<()> {
        let current = fstat(&self.parent)
            .map_err(|error| format!("cannot inspect {phase} output parent: {error}"))?;
        validate_controlled_directory(&current, &format!("{phase} output parent"))?;
        if !same_directory_snapshot(expected, &current) {
            return Err(format!("{phase} output parent metadata changed"));
        }
        Ok(())
    }

    fn verify_exact_files(
        &self,
        directory: &OwnedFd,
        expected: &[(&str, &[u8])],
        phase: &str,
    ) -> BenchResult<()> {
        let expected_names = expected
            .iter()
            .map(|(name, _)| (*name).to_owned())
            .collect::<BTreeSet<_>>();
        if directory_roster(directory, phase)? != expected_names {
            return Err(format!("{phase} output file roster drifted"));
        }
        for ((name, bytes), staged) in expected.iter().zip(&self.files) {
            if staged.name != OsStr::new(name) {
                return Err(format!("{phase} output order drifted"));
            }
            verify_exact_file(directory, name, bytes, staged, phase)?;
        }
        if directory_roster(directory, phase)? != expected_names {
            return Err(format!("{phase} output roster changed during verification"));
        }
        Ok(())
    }
}

impl Drop for ExactBundle {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        for file in &self.files {
            let held = fstat(&file.file).ok();
            let bound = openat2(
                &self.staging,
                Path::new(&file.name),
                OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
                Mode::empty(),
                ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
            )
            .ok()
            .and_then(|descriptor| fstat(&descriptor).ok())
            .zip(held)
            .is_some_and(|(current, held)| {
                same_file_identity(&file.snapshot, &held)
                    && held.st_dev == current.st_dev
                    && held.st_ino == current.st_ino
            });
            if bound {
                let _ = unlinkat(&self.staging, file.name.as_os_str(), AtFlags::empty());
            }
        }
    }
}

fn open_directory_at(parent: &OwnedFd, path: &Path, description: &str) -> BenchResult<OwnedFd> {
    openat2(
        parent,
        path,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )
    .map_err(|error| format!("cannot open {description} directory: {error}"))
}

fn path_exists_at(parent: &OwnedFd, name: &OsStr) -> BenchResult<bool> {
    match openat2(
        parent,
        Path::new(name),
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    ) {
        Ok(_) => Ok(true),
        Err(error) if error == rustix::io::Errno::NOENT => Ok(false),
        Err(error) => Err(format!("cannot inspect output path: {error}")),
    }
}

fn validate_controlled_directory(stat: &Stat, description: &str) -> BenchResult<()> {
    if FileType::from_raw_mode(stat.st_mode) != FileType::Directory
        || stat.st_uid != geteuid().as_raw()
        || stat.st_gid != getegid().as_raw()
        || stat.st_mode & 0o022 != 0
    {
        return Err(format!(
            "{description} must be owner-controlled without group/other writes"
        ));
    }
    Ok(())
}

// Safe Rust has no atomic directory-create-and-open operation. The publisher adopts
// this exact empty directory without claiming that `mkdirat` created its inode.
fn validate_adopted_directory(stat: &Stat, description: &str) -> BenchResult<()> {
    if FileType::from_raw_mode(stat.st_mode) != FileType::Directory
        || stat.st_mode & 0o7777 != 0o700
        || stat.st_uid != geteuid().as_raw()
        || stat.st_gid != getegid().as_raw()
    {
        return Err(format!(
            "adopted {description} must retain exact 0700 effective-owner/group custody"
        ));
    }
    Ok(())
}

fn validate_directory_transition(
    expected: &Stat,
    actual: &Stat,
    description: &str,
) -> BenchResult<()> {
    validate_controlled_directory(actual, description)?;
    if !same_directory_identity(expected, actual) {
        return Err(format!(
            "{description} identity or control metadata changed"
        ));
    }
    Ok(())
}

fn same_directory_identity(expected: &Stat, actual: &Stat) -> bool {
    expected.st_dev == actual.st_dev
        && expected.st_ino == actual.st_ino
        && expected.st_mode == actual.st_mode
        && expected.st_nlink == actual.st_nlink
        && expected.st_uid == actual.st_uid
        && expected.st_gid == actual.st_gid
        && FileType::from_raw_mode(actual.st_mode) == FileType::Directory
}

fn same_directory_snapshot(expected: &Stat, actual: &Stat) -> bool {
    same_directory_identity(expected, actual)
        && expected.st_size == actual.st_size
        && expected.st_mtime == actual.st_mtime
        && expected.st_mtime_nsec == actual.st_mtime_nsec
        && expected.st_ctime == actual.st_ctime
        && expected.st_ctime_nsec == actual.st_ctime_nsec
}

fn directory_roster(directory: &OwnedFd, phase: &str) -> BenchResult<BTreeSet<String>> {
    let mut entries = Dir::read_from(directory)
        .map_err(|error| format!("cannot enumerate {phase} output directory: {error}"))?;
    let mut names = BTreeSet::new();
    while let Some(entry) = entries.read() {
        let entry =
            entry.map_err(|error| format!("cannot enumerate {phase} output directory: {error}"))?;
        let bytes = entry.file_name().to_bytes();
        if matches!(bytes, b"." | b"..") {
            continue;
        }
        let name = std::str::from_utf8(bytes)
            .map_err(|_| format!("{phase} output filename must be UTF-8"))?;
        if !name.is_ascii() || !names.insert(name.to_owned()) {
            return Err(format!("{phase} output file roster is invalid"));
        }
    }
    Ok(names)
}

fn verify_exact_file(
    directory: &OwnedFd,
    name: &str,
    expected: &[u8],
    staged: &StagedFile,
    phase: &str,
) -> BenchResult<()> {
    verify_held_file(staged, expected, phase)?;
    let descriptor = openat2(
        directory,
        Path::new(name),
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )
    .map_err(|error| format!("cannot open {phase} output file {name}: {error}"))?;
    let initial = fstat(&descriptor)
        .map_err(|error| format!("cannot inspect {phase} output file {name}: {error}"))?;
    if !same_file(&staged.snapshot, &initial)
        || FileType::from_raw_mode(initial.st_mode) != FileType::RegularFile
        || initial.st_nlink != 1
        || usize::try_from(initial.st_size).ok() != Some(expected.len())
    {
        return Err(format!("{phase} output file metadata drifted: {name}"));
    }
    let mut file = File::from(descriptor);
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(
            u64::try_from(expected.len())
                .unwrap_or(u64::MAX)
                .saturating_add(1),
        )
        .read_to_end(&mut bytes)
        .map_err(|error| format!("cannot reread {phase} output file {name}: {error}"))?;
    let final_stat = fstat(&file)
        .map_err(|error| format!("cannot reinspect {phase} output file {name}: {error}"))?;
    if bytes != expected || !same_file(&initial, &final_stat) {
        return Err(format!("{phase} output bytes changed: {name}"));
    }
    let rebound = openat2(
        directory,
        Path::new(name),
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )
    .map_err(|error| format!("cannot rebind {phase} output file {name}: {error}"))?;
    let rebound_stat = fstat(&rebound)
        .map_err(|error| format!("cannot inspect rebound {phase} output file {name}: {error}"))?;
    if !same_file(&final_stat, &rebound_stat) {
        return Err(format!(
            "{phase} output filename changed during verification: {name}"
        ));
    }
    verify_held_file(staged, expected, phase)?;
    Ok(())
}

fn verify_held_file(staged: &StagedFile, expected: &[u8], phase: &str) -> BenchResult<()> {
    let initial = fstat(&staged.file).map_err(|error| {
        format!(
            "cannot inspect held {phase} output file {}: {error}",
            staged.name.display()
        )
    })?;
    if !same_file(&staged.snapshot, &initial)
        || FileType::from_raw_mode(initial.st_mode) != FileType::RegularFile
        || initial.st_nlink != 1
        || usize::try_from(initial.st_size).ok() != Some(expected.len())
    {
        return Err(format!(
            "held {phase} output metadata drifted: {}",
            staged.name.display()
        ));
    }
    let mut file = staged.file.try_clone().map_err(|error| {
        format!(
            "cannot duplicate held {phase} output file {}: {error}",
            staged.name.display()
        )
    })?;
    file.seek(SeekFrom::Start(0)).map_err(|error| {
        format!(
            "cannot rewind held {phase} output file {}: {error}",
            staged.name.display()
        )
    })?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(
            u64::try_from(expected.len())
                .unwrap_or(u64::MAX)
                .saturating_add(1),
        )
        .read_to_end(&mut bytes)
        .map_err(|error| {
            format!(
                "cannot reread held {phase} output file {}: {error}",
                staged.name.display()
            )
        })?;
    let final_stat = fstat(&staged.file).map_err(|error| {
        format!(
            "cannot reinspect held {phase} output file {}: {error}",
            staged.name.display()
        )
    })?;
    if bytes != expected || !same_file(&initial, &final_stat) {
        return Err(format!(
            "held {phase} output bytes changed: {}",
            staged.name.display()
        ));
    }
    Ok(())
}

fn same_file_identity(left: &Stat, right: &Stat) -> bool {
    left.st_dev == right.st_dev
        && left.st_ino == right.st_ino
        && left.st_mode == right.st_mode
        && left.st_nlink == right.st_nlink
        && left.st_uid == right.st_uid
        && left.st_gid == right.st_gid
}

fn same_file(left: &Stat, right: &Stat) -> bool {
    same_file_identity(left, right)
        && left.st_size == right.st_size
        && left.st_mtime == right.st_mtime
        && left.st_mtime_nsec == right.st_mtime_nsec
        && left.st_ctime == right.st_ctime
        && left.st_ctime_nsec == right.st_ctime_nsec
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::env;
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NONCE: AtomicU64 = AtomicU64::new(0);

    struct Temporary(PathBuf);

    impl Temporary {
        fn new() -> Self {
            let nonce = NONCE.fetch_add(1, Ordering::Relaxed);
            let path = env::temp_dir().join(format!(
                "ferric-m1-r33-partial-test.{}.{nonce}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap();
            fs::set_permissions(&path, std::os::unix::fs::PermissionsExt::from_mode(0o700))
                .unwrap();
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

    fn companion(kind: &str, payload: Value) -> Vec<u8> {
        encode_canonical_document(&json!({
            "authority": COMPANION_AUTHORITY,
            "format": companion_format(kind),
            "kind": kind,
            "payload": payload,
            "status": "pre-observation",
        }))
        .unwrap()
    }

    fn workload() -> Value {
        json!({
            "case_id": "target-load.001",
            "cell_id": "externally-declared-target-load.001",
            "input_tokens_per_request": 8,
            "maximum_output_tokens_per_request": 4,
            "offered_concurrency": 2,
            "prefix_sharing": false,
            "prompt_roster_sha256": digest("prompts"),
            "sampling_seed": 7,
            "target_mode": "target-only",
        })
    }

    fn arrivals() -> Value {
        json!({
            "case_id": "target-load.001",
            "clock": "monotonic-raw-nanoseconds",
            "starts": [{
                "recorded_windows": [{
                    "end_offset_ns": 300,
                    "requests": [
                        {"arrival_offset_ns": 110, "request_id": "request.001"},
                        {"arrival_offset_ns": 120, "request_id": "request.002"},
                    ],
                    "start_offset_ns": 100,
                    "window_index": 0,
                }],
                "start_index": 0,
                "warmup_windows": [{
                    "end_offset_ns": 90,
                    "requests": [{"arrival_offset_ns": 10, "request_id": "warmup.001"}],
                    "start_offset_ns": 0,
                    "window_index": 0,
                }],
            }],
        })
    }

    fn summary() -> Value {
        Summary {
            end_to_end_ns: Percentiles {
                p50: 50,
                p90: 50,
                p99: 50,
            },
            failures: 1,
            input_tokens: 8,
            input_tokens_per_second_milli: 40_000_000_000,
            itl_ns: Percentiles {
                p50: 10,
                p90: 10,
                p99: 10,
            },
            output_tokens: 2,
            output_tokens_per_second_milli: 10_000_000_000,
            requests: 2,
            requests_per_second_milli: 10_000_000_000,
            successful_requests: 1,
            successful_requests_per_second_milli: 5_000_000_000,
            total_tokens: 10,
            total_tokens_per_second_milli: 50_000_000_000,
            tpot_ns: Percentiles {
                p50: 10,
                p90: 10,
                p99: 10,
            },
            ttft_ns: Percentiles {
                p50: 20,
                p90: 20,
                p99: 20,
            },
        }
        .as_json()
    }

    fn events(experiment_sha256: &str) -> Value {
        json!({
            "authority": EVENTS_AUTHORITY,
            "case_id": "target-load.001",
            "experiment_sha256": experiment_sha256,
            "format": EVENTS_FORMAT,
            "starts": [{
                "recorded_windows": [{
                    "end_ns": 1300,
                    "requests": [
                        {
                            "arrival_ns": 1110,
                            "end_ns": 1160,
                            "outcome": "completed",
                            "output_token_timestamps_ns": [1130, 1140],
                            "prompt_tokens": 8,
                            "request_id": "request.001",
                        },
                        {
                            "arrival_ns": 1120,
                            "end_ns": 1150,
                            "outcome": "failed",
                            "output_token_timestamps_ns": [],
                            "prompt_tokens": 8,
                            "request_id": "request.002",
                        },
                    ],
                    "start_ns": 1100,
                    "summary": summary(),
                    "window_index": 0,
                }],
                "start_index": 0,
                "start_time_ns": 1000,
                "warmup_windows": [{
                    "end_ns": 1090,
                    "requests": [{
                        "arrival_ns": 1010,
                        "end_ns": 1040,
                        "outcome": "completed",
                        "output_token_timestamps_ns": [1020, 1030],
                        "prompt_tokens": 8,
                        "request_id": "warmup.001",
                    }],
                    "start_ns": 1000,
                    "window_index": 0,
                }],
            }],
            "status": "collected-unvalidated",
            "target": TARGET,
        })
    }

    fn build_fixture(root: &Path) -> (PathBuf, PathBuf) {
        let identity_payload = |names: &[&str]| {
            names
                .iter()
                .map(|name| ((*name).to_owned(), Value::String(digest(name))))
                .collect::<Map<_, _>>()
        };
        let documents = BTreeMap::from([
            ("arrivals", companion("arrivals", arrivals())),
            (
                "artifacts",
                companion(
                    "artifacts",
                    Value::Object(identity_payload(&[
                        "fe2o3_source_closure_sha256",
                        "ferric_source_closure_sha256",
                        "kernel_artifact_manifest_sha256",
                        "runner_declaration_sha256",
                    ])),
                ),
            ),
            (
                "baselines",
                companion(
                    "baselines",
                    json!({
                        "sglang": {"config_sha256": digest("sglang-config"), "implementation_sha256": digest("sglang-impl"), "version": "external-sglang-version"},
                        "vllm": {"config_sha256": digest("vllm-config"), "implementation_sha256": digest("vllm-impl"), "version": "external-vllm-version"},
                    }),
                ),
            ),
            (
                "environment",
                companion(
                    "environment",
                    Value::Object(identity_payload(&[
                        "environment_sha256",
                        "hardware_sha256",
                        "software_sha256",
                    ])),
                ),
            ),
            (
                "model",
                companion(
                    "model",
                    Value::Object(identity_payload(&[
                        "model_sha256",
                        "tokenizer_sha256",
                        "weights_sha256",
                    ])),
                ),
            ),
            (
                "policy",
                companion(
                    "policy",
                    json!({
                        "itl_p99_slo_ns": 10,
                        "policy_identity_sha256": digest("external-policy"),
                        "ttft_p99_slo_ns": 20,
                    }),
                ),
            ),
            (
                "tuning",
                companion(
                    "tuning",
                    Value::Object(identity_payload(&[
                        "cache_policy_sha256",
                        "ferric_config_sha256",
                        "ferric_tuning_sha256",
                        "sglang_tuning_sha256",
                        "tuning_budget_sha256",
                        "vllm_tuning_sha256",
                    ])),
                ),
            ),
            ("workload", companion("workload", workload())),
        ]);
        let mut bindings = Map::new();
        for (kind, bytes) in &documents {
            let name = format!("{kind}.json");
            fs::write(root.join(&name), bytes).unwrap();
            bindings.insert(
                (*kind).to_owned(),
                json!({"bytes": bytes.len(), "path": name, "sha256": sha256_identity(bytes)}),
            );
        }
        let experiment = encode_canonical_document(&json!({
            "authority": INPUT_AUTHORITY,
            "case_id": "target-load.001",
            "case_kind": "externally-declared-target-load",
            "companions": bindings,
            "event_transcript_path": "events.json",
            "format": EXPERIMENT_FORMAT,
            "nonclaim": NONCLAIM,
            "obligation_id": "m1.r33",
            "protocol_sha256": sha256_identity(&protocol_bytes().unwrap()),
            "roster": fixed_roster(),
            "status": "pre-observation",
            "target": TARGET,
            "timing_boundaries": timing_boundaries(),
        }))
        .unwrap();
        let experiment_path = root.join("experiment.json");
        fs::write(&experiment_path, &experiment).unwrap();
        let event_bytes =
            encode_canonical_document(&events(&sha256_identity(&experiment))).unwrap();
        let event_path = root.join("events.json");
        fs::write(&event_path, event_bytes).unwrap();
        (experiment_path, event_path)
    }

    #[test]
    fn protocol_is_checked_in_canonical_and_explicitly_partial() {
        require_protocol().unwrap();
        let manifest = env::var_os("CARGO_MANIFEST_DIR").unwrap();
        let checked_in =
            fs::read(PathBuf::from(manifest).join("ferric-m1-r33-partial-protocol.json")).unwrap();
        assert_eq!(protocol_bytes().unwrap(), checked_in);
        let text = String::from_utf8(checked_in).unwrap();
        assert!(text.contains("partial-non-evidence"));
        assert!(text.contains("not continuous serving"));
        assert!(text.contains("cannot establish a fresh server launch or server saturation"));
        assert!(text.contains("cannot establish baseline competitiveness"));
        assert!(text.contains(RATE_REQUESTS));
        assert!(text.contains(TARGET_LOAD_PREDICATE));
    }

    #[test]
    fn mixed_outcomes_define_all_rate_populations_and_units() {
        let roster = parse_arrivals(&arrivals(), "target-load.001").unwrap();
        let workload = parse_workload(&workload(), "target-load.001").unwrap();
        let experiment = digest("rate-test-experiment");
        let summary = validate_events(
            &events(&experiment),
            "target-load.001",
            &experiment,
            &roster,
            &workload,
        )
        .unwrap();

        assert_eq!(summary.requests, 2);
        assert_eq!(summary.successful_requests, 1);
        assert_eq!(summary.failures, 1);
        assert_eq!(summary.input_tokens, 8);
        assert_eq!(summary.output_tokens, 2);
        assert_eq!(summary.total_tokens, 10);
        assert_eq!(summary.requests_per_second_milli, 10_000_000_000);
        assert_eq!(summary.successful_requests_per_second_milli, 5_000_000_000);
        assert_eq!(summary.input_tokens_per_second_milli, 40_000_000_000);
        assert_eq!(summary.output_tokens_per_second_milli, 10_000_000_000);
        assert_eq!(summary.total_tokens_per_second_milli, 50_000_000_000);
    }

    #[test]
    fn nearest_rank_percentiles_cover_nontrivial_boundaries() {
        let mut hundred = (1_u64..=100).collect::<Vec<_>>();
        assert_eq!(
            percentiles(&mut hundred).unwrap(),
            Percentiles {
                p50: 50,
                p90: 90,
                p99: 99,
            }
        );
        let mut three = vec![30, 10, 20];
        assert_eq!(
            percentiles(&mut three).unwrap(),
            Percentiles {
                p50: 20,
                p90: 30,
                p99: 30,
            }
        );
    }

    #[test]
    fn serial_or_concurrency_mismatched_target_load_fails_closed() {
        let roster = parse_arrivals(&arrivals(), "target-load.001").unwrap();
        let workload = parse_workload(&workload(), "target-load.001").unwrap();
        let experiment = digest("target-load-test-experiment");

        let mut serial = events(&experiment);
        serial["starts"][0]["recorded_windows"][0]["requests"][0]["end_ns"] = json!(1115);
        serial["starts"][0]["recorded_windows"][0]["requests"][0]["output_token_timestamps_ns"] =
            json!([1111, 1112]);
        let error = validate_events(&serial, "target-load.001", &experiment, &roster, &workload)
            .unwrap_err();
        assert!(error.contains("do not realize the externally declared target load"));

        let mut mismatched = workload;
        mismatched.offered_concurrency = 3;
        let error = validate_events(
            &events(&experiment),
            "target-load.001",
            &experiment,
            &roster,
            &mismatched,
        )
        .unwrap_err();
        assert!(error.contains("differs from externally declared offered concurrency"));
    }

    #[test]
    fn valid_events_recompute_all_metrics_and_publish_exact_bundle() {
        let temporary = Temporary::new();
        let (experiment, _) = build_fixture(&temporary.0);
        let output = temporary.0.join("bundle");
        produce(&experiment, &output).unwrap();
        assert_eq!(
            fs::read_dir(&output)
                .unwrap()
                .map(|entry| entry.unwrap().file_name())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([
                OsString::from("capture.json"),
                OsString::from("protocol.json")
            ])
        );
        let capture: Value =
            serde_json::from_slice(&fs::read(output.join("capture.json")).unwrap()).unwrap();
        assert_eq!(capture["status"], STATUS);
        assert_eq!(capture["summary"], summary());
        assert_eq!(capture["summary"]["failures"], 1);
    }

    #[test]
    fn event_order_time_and_reported_summary_mutations_fail_closed() {
        let temporary = Temporary::new();
        let (experiment, events_path) = build_fixture(&temporary.0);
        let original: Value = serde_json::from_slice(&fs::read(&events_path).unwrap()).unwrap();

        let mut changed = original.clone();
        changed["starts"][0]["recorded_windows"][0]["requests"]
            .as_array_mut()
            .unwrap()
            .reverse();
        fs::write(&events_path, encode_canonical_document(&changed).unwrap()).unwrap();
        assert!(produce(&experiment, &temporary.0.join("order-output")).is_err());

        let mut changed = original.clone();
        changed["starts"][0]["recorded_windows"][0]["requests"][0]["output_token_timestamps_ns"]
            [1] = json!(1120);
        fs::write(&events_path, encode_canonical_document(&changed).unwrap()).unwrap();
        assert!(produce(&experiment, &temporary.0.join("time-output")).is_err());

        let mut changed = original;
        changed["starts"][0]["recorded_windows"][0]["summary"]["failures"] = json!(0);
        fs::write(&events_path, encode_canonical_document(&changed).unwrap()).unwrap();
        assert!(produce(&experiment, &temporary.0.join("summary-output")).is_err());
    }

    #[test]
    fn companion_path_and_alias_mutations_fail_closed() {
        let temporary = Temporary::new();
        let (experiment, _) = build_fixture(&temporary.0);
        fs::write(temporary.0.join("model.json"), b"{}\n").unwrap();
        assert!(produce(&experiment, &temporary.0.join("companion-output")).is_err());

        let alias_test = Temporary::new();
        let (experiment, _) = build_fixture(&alias_test.0);
        fs::remove_file(alias_test.0.join("model.json")).unwrap();
        fs::hard_link(
            alias_test.0.join("workload.json"),
            alias_test.0.join("model.json"),
        )
        .unwrap();
        assert!(produce(&experiment, &alias_test.0.join("alias-output")).is_err());

        let symlink_test = Temporary::new();
        let (experiment, _) = build_fixture(&symlink_test.0);
        fs::remove_file(symlink_test.0.join("model.json")).unwrap();
        symlink("workload.json", symlink_test.0.join("model.json")).unwrap();
        assert!(produce(&experiment, &symlink_test.0.join("symlink-output")).is_err());
    }

    #[test]
    fn unsafe_paths_and_replacement_publication_fail_closed() {
        let temporary = Temporary::new();
        let (experiment, _) = build_fixture(&temporary.0);
        let mut value: Value = serde_json::from_slice(&fs::read(&experiment).unwrap()).unwrap();
        value["event_transcript_path"] = json!("../events.json");
        fs::write(&experiment, encode_canonical_document(&value).unwrap()).unwrap();
        assert!(produce(&experiment, &temporary.0.join("path-output")).is_err());

        let replacement = Temporary::new();
        let (experiment, _) = build_fixture(&replacement.0);
        let output = replacement.0.join("bundle");
        fs::create_dir(&output).unwrap();
        fs::write(output.join("sentinel"), b"preserve").unwrap();
        assert!(produce(&experiment, &output).is_err());
        assert_eq!(fs::read(output.join("sentinel")).unwrap(), b"preserve");
    }

    #[test]
    fn mkdir_to_open_unopenable_file_substitution_is_retained() {
        let temporary = Temporary::new();
        let output = temporary.0.join("bundle");
        let replacement = RefCell::new(None);
        let result = ExactBundle::create_with_after_mkdir(&output, |staging| {
            fs::remove_dir(staging).map_err(|error| error.to_string())?;
            fs::write(staging, b"caller-owned-replacement").map_err(|error| error.to_string())?;
            fs::set_permissions(staging, std::os::unix::fs::PermissionsExt::from_mode(0o600))
                .map_err(|error| error.to_string())?;
            replacement.replace(Some(staging.to_path_buf()));
            Ok(())
        });
        assert!(result.is_err());
        let replacement = replacement.into_inner().unwrap();
        assert_eq!(fs::read(replacement).unwrap(), b"caller-owned-replacement");
        assert!(!output.exists());
    }

    #[test]
    fn exact_empty_mkdir_replacement_is_adopted_and_retained_after_cleanup() {
        let temporary = Temporary::new();
        let output = temporary.0.join("bundle");
        let adopted = RefCell::new(None);
        let mut bundle = ExactBundle::create_with_after_mkdir(&output, |staging| {
            fs::remove_dir(staging).map_err(|error| error.to_string())?;
            fs::create_dir(staging).map_err(|error| error.to_string())?;
            fs::set_permissions(staging, std::os::unix::fs::PermissionsExt::from_mode(0o700))
                .map_err(|error| error.to_string())?;
            adopted.replace(Some(staging.to_path_buf()));
            Ok(())
        })
        .unwrap();
        let capture = b"capture\n";
        let protocol = b"protocol\n";
        bundle.write("capture.json", capture).unwrap();
        bundle.write("protocol.json", protocol).unwrap();
        assert!(bundle
            .publish_exact(
                &[("capture.json", capture), ("protocol.json", protocol)],
                || Err("stop after adopted staging replacement".to_owned()),
                || Ok(()),
            )
            .is_err());

        let adopted = adopted.into_inner().unwrap();
        assert!(adopted.is_dir());
        assert_eq!(fs::read_dir(adopted).unwrap().count(), 0);
        assert!(!output.exists());
    }

    #[test]
    fn staged_name_substitution_during_prepublication_is_retained_and_rejected() {
        let temporary = Temporary::new();
        let output = temporary.0.join("bundle");
        let capture = b"capture\n";
        let protocol = b"protocol\n";
        let mut bundle = ExactBundle::create(&output).unwrap();
        bundle.write("capture.json", capture).unwrap();
        bundle.write("protocol.json", protocol).unwrap();
        let staging = temporary.0.join(&bundle.staging_name);
        let mutation_root = staging.clone();
        assert!(bundle
            .publish_exact(
                &[("capture.json", capture), ("protocol.json", protocol)],
                || {
                    fs::rename(
                        mutation_root.join("capture.json"),
                        mutation_root.join("held-capture.json"),
                    )
                    .map_err(|error| error.to_string())?;
                    fs::write(mutation_root.join("capture.json"), capture)
                        .map_err(|error| error.to_string())?;
                    Ok(())
                },
                || Ok(()),
            )
            .is_err());
        assert!(!output.exists());
        assert_eq!(fs::read(staging.join("capture.json")).unwrap(), capture);
        assert_eq!(
            fs::read(staging.join("held-capture.json")).unwrap(),
            capture
        );
    }

    #[test]
    fn transient_staging_directory_mutation_fails_metadata_custody() {
        let temporary = Temporary::new();
        let output = temporary.0.join("bundle");
        let capture = b"capture\n";
        let protocol = b"protocol\n";
        let mut bundle = ExactBundle::create(&output).unwrap();
        bundle.write("capture.json", capture).unwrap();
        bundle.write("protocol.json", protocol).unwrap();
        let staging = temporary.0.join(&bundle.staging_name);
        assert!(bundle
            .publish_exact(
                &[("capture.json", capture), ("protocol.json", protocol)],
                || {
                    let transient = staging.join("transient");
                    fs::write(&transient, b"transient").map_err(|error| error.to_string())?;
                    fs::remove_file(transient).map_err(|error| error.to_string())?;
                    Ok(())
                },
                || Ok(()),
            )
            .is_err());
        assert!(!output.exists());
    }

    #[test]
    fn transient_staged_name_round_trip_fails_custody() {
        let temporary = Temporary::new();
        let output = temporary.0.join("bundle");
        let capture = b"capture\n";
        let protocol = b"protocol\n";
        let mut bundle = ExactBundle::create(&output).unwrap();
        bundle.write("capture.json", capture).unwrap();
        bundle.write("protocol.json", protocol).unwrap();
        let staging = temporary.0.join(&bundle.staging_name);
        assert!(bundle
            .publish_exact(
                &[("capture.json", capture), ("protocol.json", protocol)],
                || {
                    let capture = staging.join("capture.json");
                    let transient = staging.join("transient");
                    fs::rename(&capture, &transient).map_err(|error| error.to_string())?;
                    fs::rename(transient, capture).map_err(|error| error.to_string())?;
                    Ok(())
                },
                || Ok(()),
            )
            .is_err());
        assert!(!output.exists());
    }

    #[test]
    fn output_parent_mode_drift_prevents_publication() {
        let temporary = Temporary::new();
        let output = temporary.0.join("bundle");
        let capture = b"capture\n";
        let protocol = b"protocol\n";
        let mut bundle = ExactBundle::create(&output).unwrap();
        bundle.write("capture.json", capture).unwrap();
        bundle.write("protocol.json", protocol).unwrap();
        let root = temporary.0.clone();
        let result = bundle.publish_exact(
            &[("capture.json", capture), ("protocol.json", protocol)],
            || {
                fs::set_permissions(&root, std::os::unix::fs::PermissionsExt::from_mode(0o777))
                    .map_err(|error| error.to_string())?;
                Ok(())
            },
            || Ok(()),
        );
        fs::set_permissions(
            &temporary.0,
            std::os::unix::fs::PermissionsExt::from_mode(0o700),
        )
        .unwrap();
        assert!(result.is_err());
        assert!(!output.exists());
    }

    #[test]
    fn published_content_mutation_after_first_verification_returns_failure() {
        let temporary = Temporary::new();
        let output = temporary.0.join("bundle");
        let capture = b"capture\n";
        let protocol = b"protocol\n";
        let mut bundle = ExactBundle::create(&output).unwrap();
        bundle.write("capture.json", capture).unwrap();
        bundle.write("protocol.json", protocol).unwrap();
        let published = output.clone();
        assert!(bundle
            .publish_exact(
                &[("capture.json", capture), ("protocol.json", protocol)],
                || Ok(()),
                || {
                    fs::write(published.join("capture.json"), b"mutated\n")
                        .map_err(|error| error.to_string())?;
                    Ok(())
                },
            )
            .is_err());
        assert_eq!(fs::read(output.join("capture.json")).unwrap(), b"mutated\n");
    }

    #[test]
    fn published_name_substitution_after_first_verification_returns_failure() {
        let temporary = Temporary::new();
        let output = temporary.0.join("bundle");
        let capture = b"capture\n";
        let protocol = b"protocol\n";
        let mut bundle = ExactBundle::create(&output).unwrap();
        bundle.write("capture.json", capture).unwrap();
        bundle.write("protocol.json", protocol).unwrap();
        let published = output.clone();
        assert!(bundle
            .publish_exact(
                &[("capture.json", capture), ("protocol.json", protocol)],
                || Ok(()),
                || {
                    fs::rename(
                        published.join("capture.json"),
                        published.join("held-capture.json"),
                    )
                    .map_err(|error| error.to_string())?;
                    fs::write(published.join("capture.json"), capture)
                        .map_err(|error| error.to_string())?;
                    Ok(())
                },
            )
            .is_err());
        assert_eq!(fs::read(output.join("capture.json")).unwrap(), capture);
        assert_eq!(fs::read(output.join("held-capture.json")).unwrap(), capture);
    }
}
