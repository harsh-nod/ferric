//! Policy-bound command collection for the complete M1 serving comparison.
//!
//! One externally frozen adapter controls all three engines. The collector
//! fixes lifecycle and measurement command identities, rotates three hardware
//! slots across starts, recomputes per-window timing percentiles from raw
//! request events, and delegates final observation validation to the V3 checker.

use crate::m1_r33_serving_records;
use ferric_m1_benchmarks::{
    encode_canonical_document, load_canonical_document_held, sha256_identity, BenchResult,
    SecureInputDirectory, SecureInputFile,
};
use rustix::fs::{fcntl_getfl, fcntl_setfl, OFlags};
use rustix::process::{kill_process_group, waitid, Pid, Signal, WaitId, WaitIdOptions};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs::{File, Metadata};
use std::io::{Read, Seek};
use std::os::fd::{AsFd, AsRawFd};
use std::os::unix::fs::MetadataExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub(super) const COMMAND: &str = "collect-comparison-observations";

const COMMAND_PLAN_FORMAT: &str = "FERRIC-M1-R33-SERVING-COLLECTOR-PLAN-V2";
const COMMAND_PLAN_AUTHORITY: &str = "external-pre-execution-r33-serving-collector-plan-v2-only";
const COMMAND_PLAN_NONCLAIM: &str = "This plan freezes one R33 V3 comparison collection over three externally assigned exclusive gfx942 hardware slots and external Ferric, vLLM, and SGLang lifecycle adapters. Measure adapters must return ordered per-request arrival, first-token, terminal, and token-work events; adapter reports remain observations, not independently validated facts. The collector does not validate tuning fairness, server freshness, slot exclusivity, hardware identity, model answers, numerical or hardware correctness, performance qualification, or independent reproduction, and it does not close m1.r33 or M1.";
const RESULT_FORMAT: &str = "FERRIC-M1-R33-SERVING-ADAPTER-RESULT-V2";
const RESULT_AUTHORITY: &str = "external-r33-serving-adapter-report-only";
const PROTOCOL_AUTHORITY: &str = "ferric-m1-r33-serving-collector-protocol-only";
const PROTOCOL_FORMAT: &str = "FERRIC-M1-R33-SERVING-COLLECTOR-PROTOCOL-V2";
const TARGET: &str = "gfx942:xnack-";
const ENGINES: &[&str] = &["ferric", "vllm", "sglang"];
const ACTIONS: &[&str] = &["start", "ready", "measure", "stop"];
const SERVER_STARTS: usize = 3;
const WARMUPS_PER_START: usize = 10;
const RECORDED_PER_START: usize = 10;
const MAX_TIMEOUT_SECONDS: u64 = 24 * 60 * 60;
const MAX_ARGUMENT_BYTES: usize = 1024 * 1024;
const MAX_RUNNER_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
const MAX_REQUESTS_PER_WINDOW: usize = 100_000;
const CLOCK: &str = "monotonic-raw-nanoseconds";
const DURATION_BOUNDARY: &str = "declared-window-start-to-declared-window-end";
const TIMING_BOUNDARIES: &str = "request-arrival-to-first-output-token-observed-to-terminal-event";
const COLLECTOR_PROTOCOL_SHA256: &str =
    "43975ba0b3dbaf5b5d880f70f879dcc26d47e8c89372790fee383a141199fe03";

const COMMAND_PLAN_KEYS: &[&str] = &[
    "adapters",
    "authority",
    "benchmark_executable",
    "environment",
    "format",
    "hardware_slots",
    "nonclaim",
    "obligation_id",
    "plan",
    "policy_sha256",
    "status",
    "target",
    "window_roster",
];
const ADAPTER_KEYS: &[&str] = &[
    "commands",
    "engine",
    "implementation",
    "timeout_seconds",
    "working_directory",
];
const COMMAND_KEYS: &[&str] = &["arguments", "command_sha256"];
const SLOT_KEYS: &[&str] = &[
    "hardware_configuration_sha256",
    "hardware_sha256",
    "id",
    "target",
];
const WINDOW_KEYS: &[&str] = &[
    "expected_work",
    "id",
    "ordinal",
    "phase",
    "server_start",
    "window",
];
const WORK_KEYS: &[&str] = &[
    "input_tokens",
    "output_tokens",
    "successful_requests",
    "total_tokens",
];
const RESULT_KEYS: &[&str] = &[
    "action",
    "authority",
    "command_sha256",
    "engine",
    "engine_order",
    "format",
    "implementation",
    "policy_sha256",
    "reported",
    "row",
    "server_instance_sha256",
    "server_start",
    "slot",
    "status",
    "target",
];
const MEASUREMENT_KEYS: &[&str] = &[
    "clock",
    "duration_boundary",
    "duration_ns",
    "failed_requests",
    "input_tokens",
    "output_tokens",
    "request_events",
    "request_timing_boundaries",
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Work {
    input_tokens: u64,
    output_tokens: u64,
    successful_requests: u64,
    total_tokens: u64,
}

#[derive(Debug)]
struct WindowPlan {
    expected_work: Work,
    id: String,
    ordinal: usize,
    phase: &'static str,
    server_start: usize,
    value: Value,
    window: usize,
}

#[derive(Debug)]
struct HeldDocument {
    bytes: Vec<u8>,
    file: SecureInputFile,
    name: PathBuf,
    root: SecureInputDirectory,
    value: Value,
}

impl HeldDocument {
    fn load(path: &Path, description: &str) -> BenchResult<Self> {
        let (root, value, bytes, file) = load_canonical_document_held(path, description)?;
        let name = path
            .file_name()
            .map(PathBuf::from)
            .ok_or_else(|| format!("{description} path has no filename"))?;
        Ok(Self {
            bytes,
            file,
            name,
            root,
            value,
        })
    }

    fn revalidate(&self, description: &str) -> BenchResult<()> {
        self.root
            .validate_binding(&self.name, &self.file, description)
    }
}

#[derive(Debug)]
struct StablePath {
    file: File,
    initial: Metadata,
    path: PathBuf,
}

impl StablePath {
    fn executable(path: &str, expected_sha256: &str) -> BenchResult<Self> {
        let path = canonical_absolute_path(path, "R33 benchmark executable")?;
        let initial = std::fs::symlink_metadata(&path)
            .map_err(|error| format!("cannot inspect R33 benchmark executable: {error}"))?;
        if !initial.file_type().is_file() || initial.len() == 0 {
            return Err("R33 benchmark executable must be a nonempty regular file".to_owned());
        }
        let file = File::open(&path)
            .map_err(|error| format!("cannot open R33 benchmark executable: {error}"))?;
        require_same_metadata(
            &initial,
            &file
                .metadata()
                .map_err(|error| format!("cannot inspect held R33 executable: {error}"))?,
            "R33 benchmark executable",
        )?;
        if digest_file(&file, "R33 benchmark executable")? != expected_sha256 {
            return Err("R33 benchmark executable SHA-256 differs from the policy".to_owned());
        }
        let stable = Self {
            file,
            initial,
            path,
        };
        stable.revalidate("R33 benchmark executable")?;
        Ok(stable)
    }

    fn directory(path: &str, engine: &str) -> BenchResult<Self> {
        let description = format!("R33 {engine} working directory");
        let path = canonical_absolute_path(path, &description)?;
        let initial = std::fs::symlink_metadata(&path)
            .map_err(|error| format!("cannot inspect {description}: {error}"))?;
        if !initial.file_type().is_dir() {
            return Err(format!("{description} must be a directory"));
        }
        let file =
            File::open(&path).map_err(|error| format!("cannot open {description}: {error}"))?;
        require_same_metadata(
            &initial,
            &file
                .metadata()
                .map_err(|error| format!("cannot inspect held {description}: {error}"))?,
            &description,
        )?;
        Ok(Self {
            file,
            initial,
            path,
        })
    }

    fn revalidate(&self, description: &str) -> BenchResult<()> {
        let path_metadata = std::fs::symlink_metadata(&self.path)
            .map_err(|error| format!("cannot reinspect {description}: {error}"))?;
        let held_metadata = self
            .file
            .metadata()
            .map_err(|error| format!("cannot reinspect held {description}: {error}"))?;
        require_same_metadata(&self.initial, &path_metadata, description)?;
        require_same_metadata(&self.initial, &held_metadata, description)
    }

    fn proc_fd_path(&self) -> PathBuf {
        PathBuf::from(format!(
            "/proc/{}/fd/{}",
            std::process::id(),
            self.file.as_raw_fd()
        ))
    }
}

#[derive(Debug)]
struct AdapterCommand {
    arguments: Vec<OsString>,
    command_sha256: String,
}

#[derive(Debug)]
struct AdapterPlan {
    commands: Vec<AdapterCommand>,
    implementation: Value,
    timeout: Duration,
    working_directory: StablePath,
}

#[derive(Debug)]
struct ExecutionPlan {
    adapters: Vec<AdapterPlan>,
    environment: Vec<(String, String)>,
    executable: StablePath,
    slots: Vec<Value>,
    windows: Vec<WindowPlan>,
}

struct RunContext<'a> {
    action: &'static str,
    adapter: &'a AdapterPlan,
    command: &'a AdapterCommand,
    engine: &'static str,
    engine_order: &'a [&'static str],
    policy_sha256: &'a str,
    row: Option<&'a WindowPlan>,
    server_instance_sha256: Option<&'a str>,
    server_start: usize,
    slot: &'a Value,
}

#[derive(Debug)]
struct ActiveServer {
    engine_index: usize,
    instance_sha256: String,
    slot_index: usize,
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
    let [policy, command_plan, output] = arguments.as_slice() else {
        return Err(format!(
            "usage: ferric-m1-serving {COMMAND} POLICY COMMAND-PLAN OUTPUT-OBSERVATIONS"
        ));
    };
    let policy_path = Path::new(policy);
    let command_plan_path = Path::new(command_plan);
    let output_path = Path::new(output);
    if policy_path == command_plan_path
        || policy_path == output_path
        || command_plan_path == output_path
    {
        return Err("R33 collector inputs and output must be distinct paths".to_owned());
    }
    require_collector_protocol()?;
    let policy = HeldDocument::load(policy_path, "R33 comparison policy")?;
    let command_plan = HeldDocument::load(command_plan_path, "R33 collector plan")?;
    if policy.file.identity() == command_plan.file.identity() {
        return Err("R33 comparison policy and collector plan must not alias".to_owned());
    }
    m1_r33_serving_records::validate_policy_for_collection(&policy.value)?;
    m1_r33_serving_records::require_collected_output_absent(output_path)?;
    let policy_sha256 = sha256_identity(&policy.bytes);
    let execution = validate_command_plan(&command_plan.value, &policy.value, &policy_sha256)?;
    revalidate_inputs(&policy, &command_plan, &execution)?;

    let rows = collect_rows(&policy, &command_plan, &policy_sha256, &execution)?;
    revalidate_inputs(&policy, &command_plan, &execution)?;
    let expected_executable_sha256 = policy.value["plan"]["benchmark_executable_sha256"]
        .as_str()
        .ok_or_else(|| "R33 policy benchmark executable identity is absent".to_owned())?;
    if digest_file(
        &execution.executable.file,
        "R33 benchmark executable after collection",
    )? != expected_executable_sha256
    {
        return Err("R33 benchmark executable bytes changed during collection".to_owned());
    }

    let observations =
        m1_r33_serving_records::collected_observations(&policy.value, &policy_sha256, rows)?;
    let bytes = encode_canonical_document(&observations)?;
    revalidate_inputs(&policy, &command_plan, &execution)?;
    m1_r33_serving_records::publish_collected_observations(output_path, &bytes)
}

fn validate_command_plan(
    value: &Value,
    policy: &Value,
    policy_sha256: &str,
) -> BenchResult<ExecutionPlan> {
    let object = exact_object(value, COMMAND_PLAN_KEYS, "R33 collector plan")?;
    expect_string(
        object,
        "authority",
        COMMAND_PLAN_AUTHORITY,
        "R33 collector plan",
    )?;
    expect_string(object, "format", COMMAND_PLAN_FORMAT, "R33 collector plan")?;
    expect_string(
        object,
        "nonclaim",
        COMMAND_PLAN_NONCLAIM,
        "R33 collector plan",
    )?;
    expect_string(object, "obligation_id", "m1.r33", "R33 collector plan")?;
    expect_string(object, "policy_sha256", policy_sha256, "R33 collector plan")?;
    expect_string(object, "status", "pre-execution", "R33 collector plan")?;
    expect_string(object, "target", TARGET, "R33 collector plan")?;
    if object["plan"] != policy["plan"] {
        return Err("R33 collector plan policy binding drifted".to_owned());
    }

    let executable_object = exact_object(
        &object["benchmark_executable"],
        &["path", "sha256"],
        "R33 benchmark executable",
    )?;
    let executable_sha256 = sha_string(
        &executable_object["sha256"],
        "R33 benchmark executable SHA-256",
    )?;
    if policy["plan"]["benchmark_executable_sha256"].as_str() != Some(executable_sha256) {
        return Err("R33 collector executable identity differs from the policy".to_owned());
    }
    let executable = StablePath::executable(
        safe_string(&executable_object["path"], "R33 benchmark executable path")?,
        executable_sha256,
    )?;

    let environment = validate_environment(&object["environment"])?;
    let environment_sha256 = sha256_identity(&encode_canonical_document(&object["environment"])?);
    if policy["plan"]["environment_sha256"].as_str() != Some(&environment_sha256) {
        return Err("R33 collector environment identity differs from the policy".to_owned());
    }
    let slots = validate_slots(&object["hardware_slots"])?;
    let roster_sha256 = sha256_identity(&encode_canonical_document(&server_start_roster(&slots))?);
    if policy["plan"]["server_start_roster_sha256"].as_str() != Some(&roster_sha256) {
        return Err("R33 hardware slots and rotation differ from the policy roster".to_owned());
    }
    let adapters = validate_adapters(
        &object["adapters"],
        policy,
        policy_sha256,
        executable_sha256,
        &environment_sha256,
    )?;
    let windows = validate_window_roster(&object["window_roster"])?;
    Ok(ExecutionPlan {
        adapters,
        environment,
        executable,
        slots,
        windows,
    })
}

fn validate_slots(value: &Value) -> BenchResult<Vec<Value>> {
    let slots = value
        .as_array()
        .ok_or_else(|| "R33 hardware slots must be an array".to_owned())?;
    if slots.len() != ENGINES.len() {
        return Err("R33 requires exactly three hardware slots".to_owned());
    }
    let mut ids = BTreeSet::new();
    let mut identities = BTreeSet::new();
    let mut configuration = None;
    for (index, slot) in slots.iter().enumerate() {
        let object = exact_object(slot, SLOT_KEYS, "R33 hardware slot")?;
        let expected_id = format!("slot-{index}");
        expect_string(object, "id", &expected_id, "R33 hardware slot")?;
        expect_string(object, "target", TARGET, "R33 hardware slot")?;
        let slot_configuration = sha_string(
            &object["hardware_configuration_sha256"],
            "R33 hardware configuration identity",
        )?;
        if configuration.is_some_and(|expected| expected != slot_configuration) {
            return Err("R33 hardware slot configurations must be identical".to_owned());
        }
        configuration = Some(slot_configuration);
        let identity = sha_string(&object["hardware_sha256"], "R33 hardware identity")?;
        if !ids.insert(expected_id) || !identities.insert(identity.to_owned()) {
            return Err("R33 hardware slot identities must be distinct".to_owned());
        }
    }
    Ok(slots.clone())
}

fn server_start_roster(slots: &[Value]) -> Value {
    let assignments = (0..SERVER_STARTS)
        .map(|server_start| {
            json!({
                "engines": ENGINES.iter().enumerate().map(|(engine_index, engine)| json!({
                    "engine": engine,
                    "slot": slots[(engine_index + server_start) % slots.len()],
                })).collect::<Vec<_>>(),
                "server_start": server_start,
            })
        })
        .collect::<Vec<_>>();
    json!({
        "assignments": assignments,
        "hardware_slots": slots,
        "rotation": "slot-index-equals-engine-index-plus-server-start-modulo-three",
    })
}

fn validate_adapters(
    value: &Value,
    policy: &Value,
    policy_sha256: &str,
    executable_sha256: &str,
    environment_sha256: &str,
) -> BenchResult<Vec<AdapterPlan>> {
    let adapters = value
        .as_array()
        .ok_or_else(|| "R33 adapters must be an array".to_owned())?;
    let implementations = policy["implementations"]
        .as_array()
        .ok_or_else(|| "R33 policy implementations must be an array".to_owned())?;
    if adapters.len() != ENGINES.len() || implementations.len() != ENGINES.len() {
        return Err("R33 adapter roster is incomplete".to_owned());
    }
    let mut parsed = Vec::with_capacity(ENGINES.len());
    for ((adapter, implementation), engine) in adapters.iter().zip(implementations).zip(ENGINES) {
        let object = exact_object(adapter, ADAPTER_KEYS, "R33 adapter")?;
        expect_string(object, "engine", engine, "R33 adapter")?;
        if object["implementation"] != *implementation {
            return Err(format!(
                "R33 {engine} adapter implementation binding drifted"
            ));
        }
        let timeout_seconds = positive_u64(&object["timeout_seconds"], "R33 adapter timeout")?;
        if timeout_seconds > MAX_TIMEOUT_SECONDS {
            return Err(format!("R33 {engine} adapter timeout exceeds one day"));
        }
        let working_directory_value = safe_string(
            &object["working_directory"],
            "R33 adapter working directory",
        )?;
        let working_directory = StablePath::directory(working_directory_value, engine)?;
        let commands_object = exact_object(&object["commands"], ACTIONS, "R33 adapter commands")?;
        let mut commands = Vec::with_capacity(ACTIONS.len());
        for action in ACTIONS {
            let command = exact_object(
                &commands_object[*action],
                COMMAND_KEYS,
                "R33 adapter command",
            )?;
            let arguments = validate_arguments(&command["arguments"], engine, action)?;
            let identity = json!({
                "action": action,
                "arguments": command["arguments"],
                "engine": engine,
                "environment_sha256": environment_sha256,
                "executable_sha256": executable_sha256,
                "implementation": implementation,
                "policy_sha256": policy_sha256,
                "working_directory": working_directory_value,
            });
            let expected = sha256_identity(&encode_canonical_document(&identity)?);
            expect_string(command, "command_sha256", &expected, "R33 adapter command")?;
            commands.push(AdapterCommand {
                arguments,
                command_sha256: expected,
            });
        }
        parsed.push(AdapterPlan {
            commands,
            implementation: implementation.clone(),
            timeout: Duration::from_secs(timeout_seconds),
            working_directory,
        });
    }
    Ok(parsed)
}

fn validate_window_roster(value: &Value) -> BenchResult<Vec<WindowPlan>> {
    let windows = value
        .as_array()
        .ok_or_else(|| "R33 window roster must be an array".to_owned())?;
    let per_start = WARMUPS_PER_START + RECORDED_PER_START;
    if windows.len() != SERVER_STARTS * per_start {
        return Err("R33 window roster is incomplete".to_owned());
    }
    let mut parsed = Vec::with_capacity(windows.len());
    let mut ordinal = 0_usize;
    for server_start in 0..SERVER_STARTS {
        for (phase, count) in [
            ("warmup", WARMUPS_PER_START),
            ("recorded", RECORDED_PER_START),
        ] {
            for window in 0..count {
                let value = &windows[ordinal];
                let object = exact_object(value, WINDOW_KEYS, "R33 planned window")?;
                let id = format!("start-{server_start}.{phase}-{window:02}");
                expect_string(object, "id", &id, "R33 planned window")?;
                expect_string(object, "phase", phase, "R33 planned window")?;
                expect_u64(object, "ordinal", ordinal as u64, "R33 planned window")?;
                expect_u64(
                    object,
                    "server_start",
                    server_start as u64,
                    "R33 planned window",
                )?;
                expect_u64(object, "window", window as u64, "R33 planned window")?;
                let expected_work = validate_work(&object["expected_work"], &id)?;
                parsed.push(WindowPlan {
                    expected_work,
                    id,
                    ordinal,
                    phase,
                    server_start,
                    value: value.clone(),
                    window,
                });
                ordinal += 1;
            }
        }
    }
    Ok(parsed)
}

fn validate_work(value: &Value, id: &str) -> BenchResult<Work> {
    let object = exact_object(value, WORK_KEYS, "R33 expected work")?;
    let work = Work {
        input_tokens: positive_u64(&object["input_tokens"], "R33 expected input tokens")?,
        output_tokens: positive_u64(&object["output_tokens"], "R33 expected output tokens")?,
        successful_requests: positive_u64(
            &object["successful_requests"],
            "R33 expected successful requests",
        )?,
        total_tokens: positive_u64(&object["total_tokens"], "R33 expected total tokens")?,
    };
    if usize::try_from(work.successful_requests)
        .map_or(true, |requests| requests > MAX_REQUESTS_PER_WINDOW)
    {
        return Err(format!(
            "R33 expected request count exceeds the bound: {id}"
        ));
    }
    let total = work
        .input_tokens
        .checked_add(work.output_tokens)
        .ok_or_else(|| format!("R33 expected token work overflowed: {id}"))?;
    if total != work.total_tokens {
        return Err(format!("R33 expected token work is inconsistent: {id}"));
    }
    Ok(work)
}

fn collect_rows(
    policy: &HeldDocument,
    command_plan: &HeldDocument,
    policy_sha256: &str,
    execution: &ExecutionPlan,
) -> BenchResult<Vec<Value>> {
    let mut rows = Vec::with_capacity(execution.windows.len());
    let mut seen_instances = BTreeSet::new();
    let per_start = WARMUPS_PER_START + RECORDED_PER_START;
    for server_start in 0..SERVER_STARTS {
        let lifecycle_order = engine_order(server_start * per_start);
        let mut active = Vec::new();
        let collection = (|| -> BenchResult<Vec<Value>> {
            for engine in &lifecycle_order {
                let engine_index = engine_index(engine)?;
                let slot_index = (engine_index + server_start) % execution.slots.len();
                let context = context(
                    execution,
                    policy_sha256,
                    "start",
                    engine_index,
                    &lifecycle_order,
                    server_start,
                    slot_index,
                    None,
                    None,
                );
                revalidate_inputs(policy, command_plan, execution)?;
                let result = execute_adapter(execution, &context)?;
                let provisional_instance = result
                    .get("server_instance_sha256")
                    .and_then(|value| sha_string(value, "R33 server-instance identity").ok())
                    .map(str::to_owned);
                if let Some(instance) = &provisional_instance {
                    active.push(ActiveServer {
                        engine_index,
                        instance_sha256: instance.clone(),
                        slot_index,
                    });
                }
                let instance = validate_lifecycle_result(&result, &context)?;
                if provisional_instance.as_deref() != Some(&instance) {
                    return Err(
                        "R33 start result did not yield a stoppable server identity".to_owned()
                    );
                }
                if !seen_instances.insert(instance) {
                    return Err(
                        "R33 server-instance identities must be unique across starts and engines"
                            .to_owned(),
                    );
                }
            }
            for server in &active {
                let context = context(
                    execution,
                    policy_sha256,
                    "ready",
                    server.engine_index,
                    &lifecycle_order,
                    server_start,
                    server.slot_index,
                    None,
                    Some(&server.instance_sha256),
                );
                revalidate_inputs(policy, command_plan, execution)?;
                let result = execute_adapter(execution, &context)?;
                let _ = validate_lifecycle_result(&result, &context)?;
            }

            let mut start_rows = Vec::with_capacity(per_start);
            for window in execution
                .windows
                .iter()
                .skip(server_start * per_start)
                .take(per_start)
            {
                let order = engine_order(window.ordinal);
                let mut values = Map::new();
                for engine in &order {
                    let engine_index = engine_index(engine)?;
                    let server = active
                        .iter()
                        .find(|server| server.engine_index == engine_index)
                        .ok_or_else(|| format!("R33 active server is absent: {engine}"))?;
                    let context = context(
                        execution,
                        policy_sha256,
                        "measure",
                        engine_index,
                        &order,
                        server_start,
                        server.slot_index,
                        Some(window),
                        Some(&server.instance_sha256),
                    );
                    revalidate_inputs(policy, command_plan, execution)?;
                    let result = execute_adapter(execution, &context)?;
                    let counters = validate_measurement_result(&result, &context)?;
                    values.insert((*engine).to_owned(), counters);
                }
                start_rows.push(json!({
                    "engine_order": order,
                    "faults": [],
                    "id": window.id,
                    "ordinal": window.ordinal,
                    "phase": window.phase,
                    "server_start": window.server_start,
                    "status": "passed",
                    "values": values,
                    "window": window.window,
                }));
            }
            Ok(start_rows)
        })();
        let stop = stop_active(
            policy,
            command_plan,
            execution,
            policy_sha256,
            server_start,
            &lifecycle_order,
            &active,
        );
        match (collection, stop) {
            (Ok(start_rows), Ok(())) => rows.extend(start_rows),
            (Err(error), Ok(())) => return Err(error),
            (Ok(_), Err(stop_error)) => return Err(stop_error),
            (Err(error), Err(stop_error)) => return Err(format!("{error}; {stop_error}")),
        }
    }
    Ok(rows)
}

fn stop_active(
    policy: &HeldDocument,
    command_plan: &HeldDocument,
    execution: &ExecutionPlan,
    policy_sha256: &str,
    server_start: usize,
    lifecycle_order: &[&'static str],
    active: &[ActiveServer],
) -> BenchResult<()> {
    let mut failures = Vec::new();
    for server in active.iter().rev() {
        let context = context(
            execution,
            policy_sha256,
            "stop",
            server.engine_index,
            lifecycle_order,
            server_start,
            server.slot_index,
            None,
            Some(&server.instance_sha256),
        );
        if let Err(error) = revalidate_inputs(policy, command_plan, execution) {
            failures.push(format!(
                "pre-stop custody failure for {}: {error}",
                context.engine
            ));
        }
        match execute_adapter(execution, &context) {
            Ok(result) => {
                if let Err(error) = validate_lifecycle_result(&result, &context) {
                    failures.push(error);
                }
            }
            Err(error) => failures.push(error),
        }
        if let Err(error) = revalidate_inputs(policy, command_plan, execution) {
            failures.push(format!(
                "post-stop custody failure for {}: {error}",
                context.engine
            ));
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "R33 stop adapter failure(s): {}",
            failures.join(" | ")
        ))
    }
}

#[allow(clippy::too_many_arguments)]
fn context<'a>(
    execution: &'a ExecutionPlan,
    policy_sha256: &'a str,
    action: &'static str,
    engine_index: usize,
    engine_order: &'a [&'static str],
    server_start: usize,
    slot_index: usize,
    row: Option<&'a WindowPlan>,
    server_instance_sha256: Option<&'a str>,
) -> RunContext<'a> {
    let adapter = &execution.adapters[engine_index];
    let action_index = ACTIONS
        .iter()
        .position(|candidate| *candidate == action)
        .expect("fixed R33 action is present");
    RunContext {
        action,
        adapter,
        command: &adapter.commands[action_index],
        engine: ENGINES[engine_index],
        engine_order,
        policy_sha256,
        row,
        server_instance_sha256,
        server_start,
        slot: &execution.slots[slot_index],
    }
}

fn engine_order(ordinal: usize) -> Vec<&'static str> {
    (0..ENGINES.len())
        .map(|offset| ENGINES[(ordinal + offset) % ENGINES.len()])
        .collect()
}

fn engine_index(engine: &str) -> BenchResult<usize> {
    ENGINES
        .iter()
        .position(|candidate| *candidate == engine)
        .ok_or_else(|| format!("R33 engine is not admitted: {engine}"))
}

fn execute_adapter(execution: &ExecutionPlan, context: &RunContext<'_>) -> BenchResult<Value> {
    let mut command = Command::new(execution.executable.proc_fd_path());
    command
        .args(&context.command.arguments)
        .current_dir(context.adapter.working_directory.proc_fd_path())
        .env_clear()
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (name, value) in &execution.environment {
        command.env(name, value);
    }
    set_context_environment(&mut command, context)?;
    let deadline = Instant::now()
        .checked_add(context.adapter.timeout)
        .ok_or_else(|| "R33 adapter deadline overflowed".to_owned())?;
    let mut child = command.spawn().map_err(|error| {
        format!(
            "cannot start R33 adapter {}/{}/{}: {error}",
            context.server_start, context.engine, context.action
        )
    })?;
    let process_group = Pid::from_child(&child);
    let Some(mut stdout) = child.stdout.take() else {
        return Err(terminate_with_error(
            &mut child,
            process_group,
            "R33 adapter stdout pipe is absent".to_owned(),
        ));
    };
    let Some(mut stderr) = child.stderr.take() else {
        return Err(terminate_with_error(
            &mut child,
            process_group,
            "R33 adapter stderr pipe is absent".to_owned(),
        ));
    };
    for (descriptor, stream) in [(stdout.as_fd(), "stdout"), (stderr.as_fd(), "stderr")] {
        if let Err(error) = set_nonblocking(descriptor, stream, context) {
            return Err(terminate_with_error(&mut child, process_group, error));
        }
    }
    let mut stdout_bytes = Vec::new();
    let mut stderr_bytes = Vec::new();
    let mut stdout_eof = false;
    let mut stderr_eof = false;
    let mut exited = false;
    loop {
        if Instant::now() >= deadline {
            return Err(terminate_with_error(
                &mut child,
                process_group,
                format!(
                    "R33 adapter timed out: {}/{}/{}",
                    context.server_start, context.engine, context.action
                ),
            ));
        }
        if let Err(error) = drain_capped(
            &mut stdout,
            &mut stdout_bytes,
            &mut stdout_eof,
            "stdout",
            context,
        )
        .and_then(|()| {
            drain_capped(
                &mut stderr,
                &mut stderr_bytes,
                &mut stderr_eof,
                "stderr",
                context,
            )
        }) {
            return Err(terminate_with_error(&mut child, process_group, error));
        }
        if !exited {
            match waitid(
                WaitId::Pid(process_group),
                WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT,
            ) {
                Ok(Some(_)) => {
                    if let Err(error) = kill_process_group_if_present(process_group) {
                        return Err(terminate_with_error(
                            &mut child,
                            process_group,
                            format!("cannot terminate R33 adapter descendants: {error}"),
                        ));
                    }
                    exited = true;
                }
                Ok(None) => {}
                Err(error) => {
                    return Err(terminate_with_error(
                        &mut child,
                        process_group,
                        format!("cannot inspect R33 adapter: {error}"),
                    ));
                }
            }
        }
        if exited && stdout_eof && stderr_eof {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    let status = child
        .wait()
        .map_err(|error| format!("cannot reap R33 adapter: {error}"))?;
    if !status.success() {
        return Err(format!(
            "R33 adapter failed: {}/{}/{} status={} stderr={}",
            context.server_start,
            context.engine,
            context.action,
            status,
            diagnostic(&stderr_bytes)
        ));
    }
    if !stderr_bytes.is_empty() {
        return Err(format!(
            "R33 adapter wrote stderr despite success: {}/{}/{} stderr={}",
            context.server_start,
            context.engine,
            context.action,
            diagnostic(&stderr_bytes)
        ));
    }
    parse_canonical_runner_output(&stdout_bytes, context)
}

fn set_context_environment(command: &mut Command, context: &RunContext<'_>) -> BenchResult<()> {
    command.env("FERRIC_M1_R33_ACTION", context.action);
    command.env(
        "FERRIC_M1_R33_COMMAND_SHA256",
        &context.command.command_sha256,
    );
    command.env("FERRIC_M1_R33_ENGINE", context.engine);
    command.env("FERRIC_M1_R33_ENGINE_ORDER", context.engine_order.join(","));
    command.env(
        "FERRIC_M1_R33_IMPLEMENTATION_JSON",
        serde_json::to_string(&context.adapter.implementation)
            .map_err(|error| format!("cannot serialize R33 implementation binding: {error}"))?,
    );
    command.env("FERRIC_M1_R33_POLICY_SHA256", context.policy_sha256);
    command.env(
        "FERRIC_M1_R33_SERVER_START",
        context.server_start.to_string(),
    );
    command.env(
        "FERRIC_M1_R33_SLOT_CONFIGURATION_SHA256",
        sha_string(
            &context.slot["hardware_configuration_sha256"],
            "R33 hardware configuration identity",
        )?,
    );
    command.env(
        "FERRIC_M1_R33_SLOT_ID",
        safe_string(&context.slot["id"], "R33 hardware slot ID")?,
    );
    command.env(
        "FERRIC_M1_R33_SLOT_SHA256",
        sha_string(&context.slot["hardware_sha256"], "R33 hardware identity")?,
    );
    command.env("FERRIC_M1_R33_TARGET", TARGET);
    if let Some(instance) = context.server_instance_sha256 {
        command.env("FERRIC_M1_R33_SERVER_INSTANCE_SHA256", instance);
    }
    if let Some(row) = context.row {
        command.env("FERRIC_M1_R33_ROW_ID", &row.id);
        command.env("FERRIC_M1_R33_ORDINAL", row.ordinal.to_string());
        command.env("FERRIC_M1_R33_PHASE", row.phase);
        command.env("FERRIC_M1_R33_WINDOW", row.window.to_string());
        command.env(
            "FERRIC_M1_R33_EXPECTED_SUCCESSFUL_REQUESTS",
            row.expected_work.successful_requests.to_string(),
        );
        command.env(
            "FERRIC_M1_R33_EXPECTED_INPUT_TOKENS",
            row.expected_work.input_tokens.to_string(),
        );
        command.env(
            "FERRIC_M1_R33_EXPECTED_OUTPUT_TOKENS",
            row.expected_work.output_tokens.to_string(),
        );
        command.env(
            "FERRIC_M1_R33_EXPECTED_TOTAL_TOKENS",
            row.expected_work.total_tokens.to_string(),
        );
    }
    Ok(())
}

fn validate_common_result<'a>(
    value: &'a Value,
    context: &RunContext<'_>,
) -> BenchResult<&'a Map<String, Value>> {
    let object = exact_object(value, RESULT_KEYS, "R33 adapter result")?;
    for (key, expected) in [
        ("action", context.action),
        ("authority", RESULT_AUTHORITY),
        ("command_sha256", context.command.command_sha256.as_str()),
        ("engine", context.engine),
        ("format", RESULT_FORMAT),
        ("policy_sha256", context.policy_sha256),
        ("status", "passed"),
        ("target", TARGET),
    ] {
        expect_string(object, key, expected, "R33 adapter result")?;
    }
    if object["engine_order"] != json!(context.engine_order) {
        return Err("R33 adapter engine order binding drifted".to_owned());
    }
    if object["implementation"] != context.adapter.implementation {
        return Err(format!(
            "R33 {} adapter implementation binding drifted",
            context.engine
        ));
    }
    expect_u64(
        object,
        "server_start",
        context.server_start as u64,
        "R33 adapter result",
    )?;
    if object["slot"] != *context.slot {
        return Err("R33 adapter hardware slot binding drifted".to_owned());
    }
    let instance = sha_string(
        &object["server_instance_sha256"],
        "R33 server-instance identity",
    )?;
    if context
        .server_instance_sha256
        .is_some_and(|expected| instance != expected)
    {
        return Err("R33 adapter server-instance binding drifted".to_owned());
    }
    let expected_row = context.row.map_or(Value::Null, |row| row.value.clone());
    if object["row"] != expected_row {
        return Err("R33 adapter row binding drifted".to_owned());
    }
    Ok(object)
}

fn validate_lifecycle_result(value: &Value, context: &RunContext<'_>) -> BenchResult<String> {
    let object = validate_common_result(value, context)?;
    let reported = exact_object(&object["reported"], &["kind"], "R33 lifecycle report")?;
    expect_string(reported, "kind", "lifecycle", "R33 lifecycle report")?;
    Ok(sha_string(
        &object["server_instance_sha256"],
        "R33 server-instance identity",
    )?
    .to_owned())
}

fn validate_measurement_result(value: &Value, context: &RunContext<'_>) -> BenchResult<Value> {
    let object = validate_common_result(value, context)?;
    let row = context
        .row
        .ok_or_else(|| "R33 measurement context row is absent".to_owned())?;
    let reported = exact_object(
        &object["reported"],
        MEASUREMENT_KEYS,
        "R33 measurement report",
    )?;
    expect_string(reported, "clock", CLOCK, "R33 measurement report")?;
    expect_string(
        reported,
        "duration_boundary",
        DURATION_BOUNDARY,
        "R33 measurement report",
    )?;
    expect_string(
        reported,
        "request_timing_boundaries",
        TIMING_BOUNDARIES,
        "R33 measurement report",
    )?;
    let duration_ns = positive_u64(&reported["duration_ns"], "R33 measurement duration")?;
    expect_u64(reported, "failed_requests", 0, "R33 measurement report")?;
    let work = Work {
        input_tokens: positive_u64(&reported["input_tokens"], "R33 measured input tokens")?,
        output_tokens: positive_u64(&reported["output_tokens"], "R33 measured output tokens")?,
        successful_requests: positive_u64(
            &reported["successful_requests"],
            "R33 measured successful requests",
        )?,
        total_tokens: positive_u64(&reported["total_tokens"], "R33 measured total tokens")?,
    };
    if work != row.expected_work {
        return Err(format!(
            "R33 measured work differs from the pre-execution roster: {}/{}",
            row.id, context.engine
        ));
    }
    if work.input_tokens.checked_add(work.output_tokens) != Some(work.total_tokens) {
        return Err(format!(
            "R33 measured token arithmetic is inconsistent: {}/{}",
            row.id, context.engine
        ));
    }
    let event_values = reported["request_events"]
        .as_array()
        .ok_or_else(|| "R33 successful-request event population must be an array".to_owned())?;
    if event_values.len() != usize::try_from(work.successful_requests).unwrap_or(usize::MAX) {
        return Err(format!(
            "R33 successful-request event population differs from the request count: {}/{}",
            row.id, context.engine
        ));
    }
    let mut end_to_end = Vec::with_capacity(event_values.len());
    let mut ttft = Vec::with_capacity(event_values.len());
    let mut tpot = Vec::with_capacity(event_values.len());
    let mut checked_input_tokens = 0_u64;
    let mut checked_output_tokens = 0_u64;
    for (ordinal, event) in event_values.iter().enumerate() {
        let event = exact_object(event, REQUEST_EVENT_KEYS, "R33 request timing event")?;
        expect_u64(
            event,
            "request_ordinal",
            u64::try_from(ordinal)
                .map_err(|_| "R33 request ordinal does not fit u64".to_owned())?,
            "R33 request timing event",
        )?;
        let arrival = unsigned_u64(&event["arrival_offset_ns"], "R33 request arrival offset")?;
        let first = positive_u64(
            &event["first_token_offset_ns"],
            "R33 request first-token offset",
        )?;
        let terminal = positive_u64(&event["terminal_offset_ns"], "R33 request terminal offset")?;
        if !(arrival < first && first < terminal && terminal <= duration_ns) {
            return Err(format!(
                "R33 request timing order or window bound is invalid: {}/{}/{}",
                row.id, context.engine, ordinal
            ));
        }
        let request_input = positive_u64(&event["input_tokens"], "R33 request input tokens")?;
        let request_output = positive_u64(&event["output_tokens"], "R33 request output tokens")?;
        if request_output < 2 {
            return Err(format!(
                "R33 request has fewer than two output tokens required for TPOT: {}/{}/{}",
                row.id, context.engine, ordinal
            ));
        }
        checked_input_tokens = checked_input_tokens
            .checked_add(request_input)
            .ok_or_else(|| "R33 per-request input-token sum overflowed".to_owned())?;
        checked_output_tokens = checked_output_tokens
            .checked_add(request_output)
            .ok_or_else(|| "R33 per-request output-token sum overflowed".to_owned())?;
        end_to_end.push(terminal - arrival);
        ttft.push(first - arrival);
        let per_token = (terminal - first) / (request_output - 1);
        if per_token == 0 {
            return Err(format!(
                "R33 request TPOT rounded to zero: {}/{}/{}",
                row.id, context.engine, ordinal
            ));
        }
        tpot.push(per_token);
    }
    if checked_input_tokens != work.input_tokens || checked_output_tokens != work.output_tokens {
        return Err(format!(
            "R33 per-request token sums differ from measured window work: {}/{}",
            row.id, context.engine
        ));
    }
    let end_to_end = timing_percentiles(&mut end_to_end, "end-to-end")?;
    let ttft = timing_percentiles(&mut ttft, "TTFT")?;
    let tpot = timing_percentiles(&mut tpot, "TPOT")?;
    Ok(json!({
        "duration_ns": duration_ns,
        "failed_requests": 0,
        "input_tokens": work.input_tokens,
        "output_tokens": work.output_tokens,
        "p50_end_to_end_latency_ns": end_to_end[0],
        "p50_tpot_ns_per_output_token": tpot[0],
        "p50_ttft_ns": ttft[0],
        "p90_end_to_end_latency_ns": end_to_end[1],
        "p90_tpot_ns_per_output_token": tpot[1],
        "p90_ttft_ns": ttft[1],
        "p99_end_to_end_latency_ns": end_to_end[2],
        "p99_tpot_ns_per_output_token": tpot[2],
        "p99_ttft_ns": ttft[2],
        "request_events": event_values,
        "successful_requests": work.successful_requests,
        "total_tokens": work.total_tokens,
    }))
}

fn timing_percentiles(values: &mut [u64], description: &str) -> BenchResult<[u64; 3]> {
    if values.is_empty() {
        return Err(format!("R33 {description} timing population is empty"));
    }
    values.sort_unstable();
    Ok([
        nearest_rank(values, 50, description)?,
        nearest_rank(values, 90, description)?,
        nearest_rank(values, 99, description)?,
    ])
}

fn nearest_rank(values: &[u64], percentile: usize, description: &str) -> BenchResult<u64> {
    let rank = values
        .len()
        .checked_mul(percentile)
        .and_then(|value| value.checked_add(99))
        .ok_or_else(|| format!("R33 {description} nearest-rank calculation overflowed"))?
        / 100;
    values
        .get(rank - 1)
        .copied()
        .ok_or_else(|| format!("R33 {description} nearest rank is outside the population"))
}

fn revalidate_inputs(
    policy: &HeldDocument,
    command_plan: &HeldDocument,
    execution: &ExecutionPlan,
) -> BenchResult<()> {
    policy.revalidate("R33 comparison policy")?;
    command_plan.revalidate("R33 collector plan")?;
    execution
        .executable
        .revalidate("R33 benchmark executable")?;
    for (adapter, engine) in execution.adapters.iter().zip(ENGINES) {
        adapter
            .working_directory
            .revalidate(&format!("R33 {engine} working directory"))?;
    }
    Ok(())
}

fn validate_environment(value: &Value) -> BenchResult<Vec<(String, String)>> {
    let object = value
        .as_object()
        .ok_or_else(|| "R33 collector environment must be an object".to_owned())?;
    if object.len() > 128 {
        return Err("R33 collector environment contains too many variables".to_owned());
    }
    let mut environment = Vec::with_capacity(object.len());
    for (name, value) in object {
        if !valid_environment_name(name) || name.starts_with("FERRIC_M1_R33_") {
            return Err(format!(
                "R33 collector environment variable is not admitted: {name}"
            ));
        }
        let value = value
            .as_str()
            .ok_or_else(|| format!("R33 collector environment value must be a string: {name}"))?;
        if value.len() > 16 * 1024 || value.contains('\0') {
            return Err(format!(
                "R33 collector environment value is not admitted: {name}"
            ));
        }
        environment.push((name.clone(), value.to_owned()));
    }
    Ok(environment)
}

fn validate_arguments(value: &Value, engine: &str, action: &str) -> BenchResult<Vec<OsString>> {
    let arguments = value
        .as_array()
        .ok_or_else(|| format!("R33 adapter arguments must be an array: {engine}/{action}"))?;
    if arguments.len() > 256 {
        return Err(format!(
            "R33 adapter has too many arguments: {engine}/{action}"
        ));
    }
    let mut total = 0_usize;
    let mut parsed = Vec::with_capacity(arguments.len());
    for argument in arguments {
        let argument = argument
            .as_str()
            .ok_or_else(|| format!("R33 adapter argument must be a string: {engine}/{action}"))?;
        total = total
            .checked_add(argument.len())
            .ok_or_else(|| "R33 adapter argument extent overflowed".to_owned())?;
        if argument.contains('\0') || total > MAX_ARGUMENT_BYTES {
            return Err(format!(
                "R33 adapter arguments are too large: {engine}/{action}"
            ));
        }
        parsed.push(OsString::from(argument));
    }
    Ok(parsed)
}

fn parse_canonical_runner_output(bytes: &[u8], context: &RunContext<'_>) -> BenchResult<Value> {
    if bytes.is_empty() || !bytes.is_ascii() {
        return Err(format!(
            "R33 adapter output must be nonempty ASCII JSON: {}/{}/{}",
            context.server_start, context.engine, context.action
        ));
    }
    let value: Value = serde_json::from_slice(bytes).map_err(|error| {
        format!(
            "cannot parse R33 adapter output {}/{}/{}: {error}",
            context.server_start, context.engine, context.action
        )
    })?;
    if encode_canonical_document(&value)? != bytes {
        return Err(format!(
            "R33 adapter output is not canonical JSON: {}/{}/{}",
            context.server_start, context.engine, context.action
        ));
    }
    Ok(value)
}

fn set_nonblocking(
    descriptor: impl AsFd,
    stream: &str,
    context: &RunContext<'_>,
) -> BenchResult<()> {
    let flags = fcntl_getfl(descriptor.as_fd()).map_err(|error| {
        format!(
            "cannot inspect R33 adapter {stream} flags {}/{}/{}: {error}",
            context.server_start, context.engine, context.action
        )
    })?;
    fcntl_setfl(descriptor.as_fd(), flags | OFlags::NONBLOCK).map_err(|error| {
        format!(
            "cannot make R33 adapter {stream} nonblocking {}/{}/{}: {error}",
            context.server_start, context.engine, context.action
        )
    })
}

fn drain_capped(
    reader: &mut impl Read,
    bytes: &mut Vec<u8>,
    eof: &mut bool,
    stream: &str,
    context: &RunContext<'_>,
) -> BenchResult<()> {
    let mut chunk = [0_u8; 16 * 1024];
    loop {
        match reader.read(&mut chunk) {
            Ok(0) => {
                *eof = true;
                return Ok(());
            }
            Ok(read) => {
                if bytes.len().saturating_add(read) > MAX_RUNNER_OUTPUT_BYTES {
                    return Err(format!(
                        "R33 adapter {stream} exceeded 8 MiB: {}/{}/{}",
                        context.server_start, context.engine, context.action
                    ));
                }
                bytes
                    .try_reserve(read)
                    .map_err(|_| format!("cannot reserve R33 adapter {stream} buffer"))?;
                bytes.extend_from_slice(&chunk[..read]);
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => return Ok(()),
            Err(error) => return Err(format!("cannot read R33 adapter {stream}: {error}")),
        }
    }
}

fn kill_process_group_if_present(process_group: Pid) -> rustix::io::Result<()> {
    match kill_process_group(process_group, Signal::KILL) {
        Ok(()) | Err(rustix::io::Errno::SRCH) => Ok(()),
        Err(error) => Err(error),
    }
}

fn terminate_subprocess(child: &mut std::process::Child, process_group: Pid) -> BenchResult<()> {
    let group = kill_process_group_if_present(process_group)
        .map_err(|error| format!("cannot terminate R33 adapter group: {error}"));
    let _ = child.kill();
    let reap = child
        .wait()
        .map(|_| ())
        .map_err(|error| format!("cannot reap terminated R33 adapter: {error}"));
    group.and(reap)
}

fn terminate_with_error(
    child: &mut std::process::Child,
    process_group: Pid,
    error: String,
) -> String {
    match terminate_subprocess(child, process_group) {
        Ok(()) => error,
        Err(cleanup) => format!("{error}; {cleanup}"),
    }
}

fn diagnostic(bytes: &[u8]) -> String {
    String::from_utf8_lossy(&bytes[..bytes.len().min(256)]).replace(['\n', '\r'], " ")
}

fn require_collector_protocol() -> BenchResult<()> {
    let manifest = std::env::var_os("CARGO_MANIFEST_DIR")
        .ok_or_else(|| "R33 collector manifest directory is absent".to_owned())?;
    let path = PathBuf::from(manifest).join("m1_r33_serving_collector_protocol.json");
    let (_, value, bytes, file) =
        load_canonical_document_held(&path, "R33 serving collector protocol")?;
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
        "R33 serving collector protocol",
    )?;
    expect_string(
        object,
        "authority",
        PROTOCOL_AUTHORITY,
        "R33 serving collector protocol",
    )?;
    expect_string(
        object,
        "format",
        PROTOCOL_FORMAT,
        "R33 serving collector protocol",
    )?;
    expect_string(
        object,
        "nonclaim",
        COMMAND_PLAN_NONCLAIM,
        "R33 serving collector protocol",
    )?;
    expect_string(
        object,
        "obligation_id",
        "m1.r33",
        "R33 serving collector protocol",
    )?;
    expect_string(
        object,
        "status",
        "collector-protocol",
        "R33 serving collector protocol",
    )?;
    expect_string(object, "target", TARGET, "R33 serving collector protocol")?;
    if sha256_identity(&bytes) != COLLECTOR_PROTOCOL_SHA256 {
        return Err("R33 serving collector protocol SHA-256 drifted".to_owned());
    }
    file.validate_snapshot("R33 serving collector protocol")
}

fn canonical_absolute_path(value: &str, description: &str) -> BenchResult<PathBuf> {
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err(format!("{description} path must be absolute"));
    }
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("cannot canonicalize {description}: {error}"))?;
    if canonical != path {
        return Err(format!("{description} path must already be canonical"));
    }
    Ok(path)
}

fn digest_file(file: &File, description: &str) -> BenchResult<String> {
    let mut clone = file
        .try_clone()
        .map_err(|error| format!("cannot duplicate held {description}: {error}"))?;
    clone
        .rewind()
        .map_err(|error| format!("cannot rewind held {description}: {error}"))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 8 * 1024];
    loop {
        let read = clone
            .read(&mut buffer)
            .map_err(|error| format!("cannot hash held {description}: {error}"))?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    let mut identity = String::with_capacity(64);
    for byte in digest.finalize() {
        write!(&mut identity, "{byte:02x}")
            .map_err(|_| format!("cannot encode held {description} SHA-256"))?;
    }
    Ok(identity)
}

fn require_same_metadata(
    initial: &Metadata,
    current: &Metadata,
    description: &str,
) -> BenchResult<()> {
    if initial.dev() != current.dev()
        || initial.ino() != current.ino()
        || initial.mode() != current.mode()
        || initial.nlink() != current.nlink()
        || initial.len() != current.len()
        || initial.mtime() != current.mtime()
        || initial.mtime_nsec() != current.mtime_nsec()
        || initial.ctime() != current.ctime()
        || initial.ctime_nsec() != current.ctime_nsec()
    {
        return Err(format!("{description} metadata changed"));
    }
    Ok(())
}

fn valid_environment_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte == b'_' || byte.is_ascii_uppercase())
        && bytes.all(|byte| byte == b'_' || byte.is_ascii_uppercase() || byte.is_ascii_digit())
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

fn expect_string(
    object: &Map<String, Value>,
    key: &str,
    expected: &str,
    description: &str,
) -> BenchResult<()> {
    if object.get(key).and_then(Value::as_str) != Some(expected) {
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
    if object.get(key).and_then(Value::as_u64) != Some(expected) {
        return Err(format!("{description} {key} drifted"));
    }
    Ok(())
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

fn unsigned_u64(value: &Value, description: &str) -> BenchResult<u64> {
    value
        .as_u64()
        .ok_or_else(|| format!("{description} must be an unsigned integer"))
}

fn safe_string<'a>(value: &'a Value, description: &str) -> BenchResult<&'a str> {
    let value = value
        .as_str()
        .ok_or_else(|| format!("{description} must be a string"))?;
    if value.is_empty()
        || !value.is_ascii()
        || value
            .chars()
            .any(|character| matches!(character, '\0' | '\n' | '\r'))
    {
        return Err(format!("{description} is not an admitted string"));
    }
    Ok(value)
}

fn sha_string<'a>(value: &'a Value, description: &str) -> BenchResult<&'a str> {
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
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicU64, Ordering};

    const COMPARISON_NONCLAIM: &str = "This V3 record checks one exact externally frozen Ferric, tuned vLLM, and tuned SGLang comparison roster, retains an ordered event for every successful request, requires identical per-request input/output work across engines in each aligned window, and recomputes end-to-end latency, TTFT, TPOT, token throughputs, nearest-rank percentiles, exact medians, fastest-baseline selection, and a deterministic paired-percentile-bootstrap 95% throughput interval. TPOT is floor((terminal-first-token)/(output-tokens-1)) nanoseconds per output token and therefore requires at least two output tokens per request. The record enforces declared p99 timing SLOs and a 0.95 throughput lower confidence bound. It does not validate the external plan, versions, sources, tuning choices, budget, SLO choice, event truth, server freshness, hardware behavior, numerical correctness, or independent reproduction; it is not qualification evidence and does not close m1.r33 or M1.";
    const COMPARISON_PROTOCOL_SHA256: &str =
        "2f6a720b2512623332e26f77d4bbaeb42b289ab946dcbf0a56a3e3eca2aca662";
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

    static NONCE: AtomicU64 = AtomicU64::new(0);

    struct Temporary(PathBuf);

    impl Temporary {
        fn new() -> Self {
            let nonce = NONCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "ferric-m1-r33-serving-collector-test.{}.{nonce}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap();
            Self(path.canonicalize().unwrap())
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

    fn document_digest(value: &Value) -> String {
        sha256_identity(&encode_canonical_document(value).unwrap())
    }

    fn implementation(id: &str) -> Value {
        json!({
            "config_sha256": digest(&format!("{id}-config")),
            "id": id,
            "implementation_sha256": digest(&format!("{id}-implementation")),
            "protocol_sha256": digest(&format!("{id}-protocol")),
            "source_sha256": digest(&format!("{id}-source")),
            "tuning_budget_sha256": digest("equal-tuning-budget"),
            "tuning_sha256": digest(&format!("{id}-tuning")),
            "version": format!("pinned-{id}-version"),
        })
    }

    fn runner_script() -> &'static str {
        r#"#!/usr/bin/python3
import hashlib
import json
import os
import sys
import time

action = os.environ["FERRIC_M1_R33_ACTION"]
engine = os.environ["FERRIC_M1_R33_ENGINE"]
start = int(os.environ["FERRIC_M1_R33_SERVER_START"])
slot_id = os.environ["FERRIC_M1_R33_SLOT_ID"]
slot_sha = os.environ["FERRIC_M1_R33_SLOT_SHA256"]
instance = os.environ.get("FERRIC_M1_R33_SERVER_INSTANCE_SHA256")
if instance is None:
    instance = hashlib.sha256(f"{engine}/{start}/{slot_id}/{slot_sha}".encode()).hexdigest()
row_id = os.environ.get("FERRIC_M1_R33_ROW_ID", "-")
with open(os.environ["TEST_LOG"], "a", encoding="ascii") as log:
    log.write(f"{start}/{action}/{engine}/{slot_id}/{row_id}\n")

fault = os.environ["TEST_FAULT"]
if fault == "duplicate-instance":
    instance = hashlib.sha256(b"duplicate-instance").hexdigest()
if fault == "stop" and action == "stop" and engine == "sglang":
    print("injected stop failure", file=sys.stderr)
    sys.exit(7)

row = None
reported = {"kind": "lifecycle"}
if action == "measure":
    if fault == "timeout" and engine == "ferric" and row_id == "start-0.warmup-00":
        time.sleep(3)
    successful = int(os.environ["FERRIC_M1_R33_EXPECTED_SUCCESSFUL_REQUESTS"])
    input_tokens = int(os.environ["FERRIC_M1_R33_EXPECTED_INPUT_TOKENS"])
    output_tokens = int(os.environ["FERRIC_M1_R33_EXPECTED_OUTPUT_TOKENS"])
    total_tokens = int(os.environ["FERRIC_M1_R33_EXPECTED_TOTAL_TOKENS"])
    if fault == "work" and engine == "ferric" and row_id == "start-0.warmup-00":
        output_tokens += 1
    engine_offset = ["ferric", "vllm", "sglang"].index(engine)
    events = []
    for request_ordinal in range(successful):
        arrival = request_ordinal * 100
        first = arrival + 10 + engine_offset
        terminal = first + 20 + engine_offset
        events.append({
            "arrival_offset_ns": arrival,
            "first_token_offset_ns": first,
            "input_tokens": input_tokens // successful,
            "output_tokens": output_tokens // successful,
            "request_ordinal": request_ordinal,
            "terminal_offset_ns": terminal,
        })
    if fault == "event-population" and engine == "ferric" and row_id == "start-0.warmup-00":
        events.pop()
    if fault == "timing-bound" and engine == "ferric" and row_id == "start-0.warmup-00":
        events[-1]["terminal_offset_ns"] = 2000
    if fault == "timing-order" and engine == "ferric" and row_id == "start-0.warmup-00":
        events[-1]["first_token_offset_ns"] = events[-1]["terminal_offset_ns"]
    if fault == "event-work" and engine == "ferric" and row_id == "start-0.warmup-00":
        events[-1]["input_tokens"] += 1
    if fault == "single-output" and engine == "ferric" and row_id == "start-0.warmup-00":
        events[-1]["output_tokens"] = 1
    if fault == "event-extra" and engine == "ferric" and row_id == "start-0.warmup-00":
        events[-1]["submitted_ttft_ns"] = 1
    if fault == "event-ordinal" and engine == "ferric" and row_id == "start-0.warmup-00":
        events[-1]["request_ordinal"] = 0
    reported = {
        "clock": "monotonic-raw-nanoseconds",
        "duration_boundary": "declared-window-start-to-declared-window-end",
        "duration_ns": 1000 + engine_offset,
        "failed_requests": 0,
        "input_tokens": input_tokens,
        "output_tokens": output_tokens,
        "request_events": events,
        "request_timing_boundaries": "request-arrival-to-first-output-token-observed-to-terminal-event",
        "successful_requests": successful,
        "total_tokens": total_tokens,
    }
    row = {
        "expected_work": {
            "input_tokens": int(os.environ["FERRIC_M1_R33_EXPECTED_INPUT_TOKENS"]),
            "output_tokens": int(os.environ["FERRIC_M1_R33_EXPECTED_OUTPUT_TOKENS"]),
            "successful_requests": successful,
            "total_tokens": int(os.environ["FERRIC_M1_R33_EXPECTED_TOTAL_TOKENS"]),
        },
        "id": row_id,
        "ordinal": int(os.environ["FERRIC_M1_R33_ORDINAL"]),
        "phase": os.environ["FERRIC_M1_R33_PHASE"],
        "server_start": start,
        "window": int(os.environ["FERRIC_M1_R33_WINDOW"]),
    }

value = {
    "action": action,
    "authority": "external-r33-serving-adapter-report-only",
    "command_sha256": os.environ["FERRIC_M1_R33_COMMAND_SHA256"],
    "engine": engine,
    "engine_order": os.environ["FERRIC_M1_R33_ENGINE_ORDER"].split(","),
    "format": "FERRIC-M1-R33-SERVING-ADAPTER-RESULT-V2",
    "implementation": json.loads(os.environ["FERRIC_M1_R33_IMPLEMENTATION_JSON"]),
    "policy_sha256": os.environ["FERRIC_M1_R33_POLICY_SHA256"],
    "reported": reported,
    "row": row,
    "server_instance_sha256": instance,
    "server_start": start,
    "slot": {"hardware_configuration_sha256": os.environ["FERRIC_M1_R33_SLOT_CONFIGURATION_SHA256"], "hardware_sha256": slot_sha, "id": slot_id, "target": os.environ["FERRIC_M1_R33_TARGET"]},
    "status": "passed",
    "target": os.environ["FERRIC_M1_R33_TARGET"],
}
if fault == "binding" and action == "measure" and engine == "ferric" and row_id == "start-0.warmup-00":
    value["command_sha256"] = hashlib.sha256(b"wrong-command").hexdigest()
if fault == "stderr" and action == "measure" and engine == "ferric" and row_id == "start-0.warmup-00":
    print("injected stderr", file=sys.stderr)
print(json.dumps(value, indent=2, sort_keys=True))
"#
    }

    fn fixtures(root: &Path, fault: &str) -> (Value, Value, PathBuf) {
        let executable_path = root.join("adapter.py");
        fs::write(&executable_path, runner_script()).unwrap();
        let mut permissions = fs::metadata(&executable_path).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&executable_path, permissions).unwrap();
        let executable_sha256 = sha256_identity(&fs::read(&executable_path).unwrap());
        let working_directory = root.join("work");
        fs::create_dir(&working_directory).unwrap();
        let log = root.join("adapter.log");
        let environment = json!({
            "TEST_FAULT": fault,
            "TEST_LOG": log,
        });
        let environment_sha256 = document_digest(&environment);
        let implementations = ENGINES
            .iter()
            .map(|engine| implementation(engine))
            .collect::<Vec<_>>();
        let hardware_slots = (0..ENGINES.len())
            .map(|index| {
                json!({
                    "hardware_configuration_sha256": digest("identical-gfx942-configuration"),
                    "hardware_sha256": digest(&format!("hardware-slot-{index}")),
                    "id": format!("slot-{index}"),
                    "target": TARGET,
                })
            })
            .collect::<Vec<_>>();
        let mut plan = PLAN_KEYS
            .iter()
            .map(|key| ((*key).to_owned(), Value::String(digest(key))))
            .collect::<Map<_, _>>();
        plan.insert(
            "benchmark_executable_sha256".to_owned(),
            Value::String(executable_sha256.clone()),
        );
        plan.insert(
            "environment_sha256".to_owned(),
            Value::String(environment_sha256.clone()),
        );
        plan.insert(
            "server_start_roster_sha256".to_owned(),
            Value::String(document_digest(&server_start_roster(&hardware_slots))),
        );
        let policy = json!({
            "authority": "external-pre-observation-serving-comparison-policy-v3-only",
            "engine_order": ENGINES,
            "format": "FERRIC-M1-R33-SERVING-COMPARISON-POLICY-V3",
            "implementations": implementations,
            "nonclaim": COMPARISON_NONCLAIM,
            "obligation_id": "m1.r33",
            "p99_end_to_end_slo_ns": 1_000_000,
            "p99_tpot_slo_ns_per_output_token": 1_000_000,
            "p99_ttft_slo_ns": 1_000_000,
            "plan": plan,
            "protocol_sha256": COMPARISON_PROTOCOL_SHA256,
            "sample_roster": {
                "recorded_windows_per_start": RECORDED_PER_START,
                "server_starts": SERVER_STARTS,
                "warmup_windows_per_start": WARMUPS_PER_START,
            },
            "status": "pre-observation",
            "target": TARGET,
        });
        let policy_sha256 = document_digest(&policy);
        let adapters = ENGINES
            .iter()
            .enumerate()
            .map(|(engine_index, engine)| {
                let mut commands = Map::new();
                for action in ACTIONS {
                    let arguments = json!([]);
                    let identity = json!({
                        "action": action,
                        "arguments": arguments,
                        "engine": engine,
                        "environment_sha256": environment_sha256,
                        "executable_sha256": executable_sha256,
                        "implementation": policy["implementations"][engine_index],
                        "policy_sha256": policy_sha256,
                        "working_directory": working_directory,
                    });
                    commands.insert(
                        (*action).to_owned(),
                        json!({
                            "arguments": [],
                            "command_sha256": document_digest(&identity),
                        }),
                    );
                }
                json!({
                    "commands": commands,
                    "engine": engine,
                    "implementation": policy["implementations"][engine_index],
                    "timeout_seconds": if fault == "timeout" { 1 } else { 5 },
                    "working_directory": working_directory,
                })
            })
            .collect::<Vec<_>>();
        let mut window_roster = Vec::new();
        let mut ordinal = 0_usize;
        for server_start in 0..SERVER_STARTS {
            for (phase, count) in [
                ("warmup", WARMUPS_PER_START),
                ("recorded", RECORDED_PER_START),
            ] {
                for window in 0..count {
                    window_roster.push(json!({
                        "expected_work": {
                            "input_tokens": 8,
                            "output_tokens": 8,
                            "successful_requests": 4,
                            "total_tokens": 16,
                        },
                        "id": format!("start-{server_start}.{phase}-{window:02}"),
                        "ordinal": ordinal,
                        "phase": phase,
                        "server_start": server_start,
                        "window": window,
                    }));
                    ordinal += 1;
                }
            }
        }
        let command_plan = json!({
            "adapters": adapters,
            "authority": COMMAND_PLAN_AUTHORITY,
            "benchmark_executable": {
                "path": executable_path,
                "sha256": executable_sha256,
            },
            "environment": environment,
            "format": COMMAND_PLAN_FORMAT,
            "hardware_slots": hardware_slots,
            "nonclaim": COMMAND_PLAN_NONCLAIM,
            "obligation_id": "m1.r33",
            "plan": policy["plan"],
            "policy_sha256": policy_sha256,
            "status": "pre-execution",
            "target": TARGET,
            "window_roster": window_roster,
        });
        (policy, command_plan, log)
    }

    fn write_inputs(root: &Path, policy: &Value, command_plan: &Value) -> (PathBuf, PathBuf) {
        let policy_path = root.join("policy.json");
        let command_plan_path = root.join("command-plan.json");
        fs::write(&policy_path, encode_canonical_document(policy).unwrap()).unwrap();
        fs::write(
            &command_plan_path,
            encode_canonical_document(command_plan).unwrap(),
        )
        .unwrap();
        (policy_path, command_plan_path)
    }

    #[test]
    fn full_lifecycle_produces_exact_v3_timing_observations() {
        let temporary = Temporary::new();
        let (policy, command_plan, log) = fixtures(&temporary.0, "");
        let (policy_path, command_plan_path) = write_inputs(&temporary.0, &policy, &command_plan);
        let output = temporary.0.join("observations.json");
        run(vec![
            policy_path.clone().into_os_string(),
            command_plan_path.clone().into_os_string(),
            output.clone().into_os_string(),
        ])
        .unwrap();
        let (_, observations, _, file) =
            load_canonical_document_held(&output, "test R33 observations").unwrap();
        file.validate_snapshot("test R33 observations").unwrap();
        assert_eq!(observations["rows"].as_array().unwrap().len(), 60);
        assert_eq!(observations["rows"][0]["engine_order"], json!(ENGINES));
        assert_eq!(
            observations["rows"][1]["engine_order"],
            json!(["vllm", "sglang", "ferric"])
        );
        assert_eq!(
            observations["rows"][0]["values"]["ferric"]["p99_end_to_end_latency_ns"],
            30
        );
        assert_eq!(
            observations["rows"][0]["values"]["ferric"]["p99_ttft_ns"],
            10
        );
        assert_eq!(
            observations["rows"][0]["values"]["ferric"]["p99_tpot_ns_per_output_token"],
            20
        );
        assert_eq!(
            observations["rows"][0]["values"]["ferric"]["request_events"]
                .as_array()
                .unwrap()
                .len(),
            4
        );
        let lines = fs::read_to_string(log).unwrap();
        let lines = lines.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 207);
        assert_eq!(lines[0], "0/start/ferric/slot-0/-");
        assert_eq!(lines[6], "0/measure/ferric/slot-0/start-0.warmup-00");
        assert!(lines.contains(&"1/start/sglang/slot-0/-"));
        assert!(lines.contains(&"2/start/vllm/slot-0/-"));

        let record = temporary.0.join("record.json");
        assert_eq!(
            m1_r33_serving_records::main_for_arguments(vec![
                policy_path.into_os_string(),
                output.clone().into_os_string(),
                record.into_os_string(),
            ]),
            ExitCode::SUCCESS
        );
        assert!(run(vec![
            temporary.0.join("policy.json").into_os_string(),
            command_plan_path.into_os_string(),
            output.into_os_string(),
        ])
        .unwrap_err()
        .contains("already exists"));
    }

    #[test]
    fn roster_drift_is_rejected_before_execution() {
        let temporary = Temporary::new();
        let (policy, mut command_plan, log) = fixtures(&temporary.0, "");
        command_plan["window_roster"][1]["id"] = json!("wrong");
        let (policy_path, command_plan_path) = write_inputs(&temporary.0, &policy, &command_plan);
        let error = run(vec![
            policy_path.into_os_string(),
            command_plan_path.into_os_string(),
            temporary.0.join("observations.json").into_os_string(),
        ])
        .unwrap_err();
        assert!(error.contains("planned window id drifted"));
        assert!(!log.exists());
    }

    #[test]
    fn hardware_configuration_and_policy_roster_drift_fail_before_execution() {
        for mutation in ["configuration", "assignment"] {
            let temporary = Temporary::new();
            let (policy, mut command_plan, log) = fixtures(&temporary.0, "");
            if mutation == "configuration" {
                command_plan["hardware_slots"][1]["hardware_configuration_sha256"] =
                    json!(digest("different-gfx942-configuration"));
            } else {
                command_plan["hardware_slots"][1]["hardware_sha256"] =
                    json!(digest("different-slot-identity"));
            }
            let (policy_path, command_plan_path) =
                write_inputs(&temporary.0, &policy, &command_plan);
            let error = run(vec![
                policy_path.into_os_string(),
                command_plan_path.into_os_string(),
                temporary.0.join("observations.json").into_os_string(),
            ])
            .unwrap_err();
            assert!(
                error.contains("configurations must be identical")
                    || error.contains("rotation differ from the policy roster"),
                "{mutation}: {error}"
            );
            assert!(!log.exists());
        }
    }

    #[test]
    fn measured_work_drift_stops_every_active_engine_and_publishes_nothing() {
        let temporary = Temporary::new();
        let (policy, command_plan, log) = fixtures(&temporary.0, "work");
        let (policy_path, command_plan_path) = write_inputs(&temporary.0, &policy, &command_plan);
        let output = temporary.0.join("observations.json");
        let error = run(vec![
            policy_path.into_os_string(),
            command_plan_path.into_os_string(),
            output.clone().into_os_string(),
        ])
        .unwrap_err();
        assert!(error.contains("measured work differs"));
        assert!(!output.exists());
        let lines = fs::read_to_string(log).unwrap();
        let lines = lines.lines().collect::<Vec<_>>();
        assert_eq!(
            &lines[lines.len() - 3..],
            [
                "0/stop/sglang/slot-2/-",
                "0/stop/vllm/slot-1/-",
                "0/stop/ferric/slot-0/-",
            ]
        );
    }

    #[test]
    fn stop_failure_attempts_remaining_stops_and_publishes_nothing() {
        let temporary = Temporary::new();
        let (policy, command_plan, log) = fixtures(&temporary.0, "stop");
        let (policy_path, command_plan_path) = write_inputs(&temporary.0, &policy, &command_plan);
        let output = temporary.0.join("observations.json");
        let error = run(vec![
            policy_path.into_os_string(),
            command_plan_path.into_os_string(),
            output.clone().into_os_string(),
        ])
        .unwrap_err();
        assert!(error.contains("stop adapter failure"));
        assert!(!output.exists());
        let lines = fs::read_to_string(log).unwrap();
        let lines = lines.lines().collect::<Vec<_>>();
        assert_eq!(
            &lines[lines.len() - 3..],
            [
                "0/stop/sglang/slot-2/-",
                "0/stop/vllm/slot-1/-",
                "0/stop/ferric/slot-0/-",
            ]
        );
    }

    #[test]
    fn hostile_adapter_reports_fail_closed_after_best_effort_stop() {
        for (fault, expected) in [
            ("binding", "command_sha256 drifted"),
            ("stderr", "wrote stderr despite success"),
            ("event-population", "event population differs"),
            ("timing-bound", "timing order or window bound is invalid"),
            ("timing-order", "timing order or window bound is invalid"),
            ("event-work", "per-request token sums differ"),
            ("single-output", "fewer than two output tokens"),
            ("event-extra", "request timing event fields drifted"),
            ("event-ordinal", "request_ordinal drifted"),
            (
                "duplicate-instance",
                "server-instance identities must be unique",
            ),
        ] {
            let temporary = Temporary::new();
            let (policy, command_plan, log) = fixtures(&temporary.0, fault);
            let (policy_path, command_plan_path) =
                write_inputs(&temporary.0, &policy, &command_plan);
            let output = temporary.0.join("observations.json");
            let error = run(vec![
                policy_path.into_os_string(),
                command_plan_path.into_os_string(),
                output.clone().into_os_string(),
            ])
            .unwrap_err();
            assert!(error.contains(expected), "{fault}: {error}");
            assert!(!output.exists());
            let lines = fs::read_to_string(log).unwrap();
            assert!(lines.lines().any(|line| line.contains("/stop/")));
        }
    }

    #[test]
    fn timeout_kills_measurement_and_stops_every_active_engine() {
        let temporary = Temporary::new();
        let (policy, command_plan, log) = fixtures(&temporary.0, "timeout");
        let (policy_path, command_plan_path) = write_inputs(&temporary.0, &policy, &command_plan);
        let output = temporary.0.join("observations.json");
        let error = run(vec![
            policy_path.into_os_string(),
            command_plan_path.into_os_string(),
            output.clone().into_os_string(),
        ])
        .unwrap_err();
        assert!(error.contains("timed out"));
        assert!(!output.exists());
        let lines = fs::read_to_string(log).unwrap();
        assert_eq!(
            lines.lines().filter(|line| line.contains("/stop/")).count(),
            3
        );
    }

    #[test]
    fn timing_percentiles_use_nearest_rank_ceil() {
        let mut one_to_one_hundred = (1..=100).collect::<Vec<_>>();
        assert_eq!(
            timing_percentiles(&mut one_to_one_hundred, "test").unwrap(),
            [50, 90, 99]
        );
        let mut crossed = vec![5, 1, 100, 2];
        assert_eq!(
            timing_percentiles(&mut crossed, "test").unwrap(),
            [2, 100, 100]
        );
        assert!(timing_percentiles(&mut [], "test").is_err());
    }
}
