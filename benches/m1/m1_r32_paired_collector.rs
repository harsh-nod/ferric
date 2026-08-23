//! Real, policy-bound paired command collection for M1 speculation.
//!
//! Every counter in the published observation document comes from a distinct
//! successful child command. The collector supplies bindings, checks them on
//! return, and never derives or substitutes timing values.

use crate::m1_r32_speculation_records;
use ferric_m1_benchmarks::{
    encode_canonical_document, load_canonical_document_held, sha256_identity, BenchResult,
    SecureInputDirectory, SecureInputFile,
};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs::{File, Metadata};
use std::io::{Read, Seek};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub(super) const COMMAND: &str = "collect-comparison-observations";

const COMMAND_PLAN_FORMAT: &str = "FERRIC-M1-R32-PAIRED-COMMAND-PLAN-V1";
const COMMAND_PLAN_AUTHORITY: &str = "external-pre-execution-r32-paired-command-plan-only";
const COMMAND_PLAN_NONCLAIM: &str = "This command plan freezes one real paired R32 collection invocation. It binds executable bytes, exact environment, per-cell arguments, policy workloads, holdouts, deterministic fallback plan, implementations, and command identities before execution. It does not establish policy fairness, runner correctness, observation truth, hardware correctness, numerical correctness, independent reproduction, performance qualification, or close m1.r32 or M1.";
const RUN_RESULT_FORMAT: &str = "FERRIC-M1-R32-PAIRED-COMMAND-RESULT-V1";
const RUN_RESULT_AUTHORITY: &str = "ferric-r32-command-reported-raw-counters-only";
const TARGET: &str = "gfx942:xnack-";
const CELL_IDS: &[&str] = &["eligible-speculation", "low-acceptance"];
const ENGINES: &[&str] = &["speculative", "target-only"];
const WARMUP_PAIRS: usize = 10;
const RECORDED_PAIRS: usize = 30;
const MAX_RUNNER_OUTPUT_BYTES: usize = 64 * 1024;
const MAX_ARGUMENT_BYTES: usize = 1024 * 1024;
const MAX_TIMEOUT_SECONDS: u64 = 24 * 60 * 60;
const COLLECTOR_PROTOCOL_SHA256: &str =
    "94c1269fc68d2eacc139d89de942568ee3e02477fde741a509dd89aec3b28f2c";

const COMMAND_PLAN_KEYS: &[&str] = &[
    "authority",
    "cells",
    "environment",
    "executable",
    "format",
    "nonclaim",
    "obligation_id",
    "plan",
    "policy_sha256",
    "status",
    "target",
    "timeout_seconds",
    "working_directory",
];
const EXECUTABLE_KEYS: &[&str] = &["path", "sha256"];
const COMMAND_CELL_KEYS: &[&str] = &[
    "commands",
    "deterministic_admitted_plan_sha256",
    "holdout_member",
    "id",
    "workload",
];
const COMMAND_KEYS: &[&str] = &["arguments", "command_sha256", "engine", "implementation"];
const RUN_RESULT_KEYS: &[&str] = &[
    "artifact_sha256",
    "authority",
    "cell_id",
    "command_sha256",
    "config_sha256",
    "counters",
    "deterministic_admitted_plan_sha256",
    "engine",
    "engine_order",
    "format",
    "holdout_sha256",
    "implementation_sha256",
    "ordinal",
    "pair_id",
    "pair_index",
    "pairing_sha256",
    "phase",
    "policy_sha256",
    "protocol_sha256",
    "source_sha256",
    "status",
    "target",
    "version",
    "workload_sha256",
];
const SPECULATIVE_COUNTER_KEYS: &[&str] = &[
    "accepted_tokens",
    "duration_ns",
    "failed_requests",
    "p99_latency_ns",
    "successful_requests",
    "target_invocations",
    "total_tokens",
];
const TARGET_ONLY_COUNTER_KEYS: &[&str] = &[
    "duration_ns",
    "failed_requests",
    "p99_latency_ns",
    "successful_requests",
    "target_invocations",
    "total_tokens",
];

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

struct StablePath {
    file: File,
    initial: Metadata,
    path: PathBuf,
}

impl StablePath {
    fn executable(path: &str, expected_sha256: &str) -> BenchResult<Self> {
        let path = canonical_absolute_path(path, "R32 benchmark executable")?;
        let initial = std::fs::symlink_metadata(&path)
            .map_err(|error| format!("cannot inspect R32 benchmark executable: {error}"))?;
        if !initial.file_type().is_file() || initial.len() == 0 {
            return Err("R32 benchmark executable must be a nonempty regular file".to_owned());
        }
        let file = File::open(&path)
            .map_err(|error| format!("cannot open R32 benchmark executable: {error}"))?;
        require_same_metadata(
            &initial,
            &file
                .metadata()
                .map_err(|error| format!("cannot inspect held R32 executable: {error}"))?,
            "R32 benchmark executable",
        )?;
        if digest_file(&file, "R32 benchmark executable")? != expected_sha256 {
            return Err("R32 benchmark executable SHA-256 differs from the policy".to_owned());
        }
        let stable = Self {
            file,
            initial,
            path,
        };
        stable.revalidate("R32 benchmark executable")?;
        Ok(stable)
    }

    fn directory(path: &str) -> BenchResult<Self> {
        let path = canonical_absolute_path(path, "R32 working directory")?;
        let initial = std::fs::symlink_metadata(&path)
            .map_err(|error| format!("cannot inspect R32 working directory: {error}"))?;
        if !initial.file_type().is_dir() {
            return Err("R32 working directory must be a directory".to_owned());
        }
        let file = File::open(&path)
            .map_err(|error| format!("cannot open R32 working directory: {error}"))?;
        require_same_metadata(
            &initial,
            &file
                .metadata()
                .map_err(|error| format!("cannot inspect held R32 working directory: {error}"))?,
            "R32 working directory",
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
}

struct EngineCommand {
    arguments: Vec<OsString>,
    command_sha256: String,
}

struct CellCommandPlan {
    commands: Vec<EngineCommand>,
    deterministic_plan: Value,
    holdout_sha256: String,
    workload_sha256: String,
}

struct ExecutionPlan {
    cells: Vec<CellCommandPlan>,
    environment: Vec<(String, String)>,
    executable: StablePath,
    timeout: Duration,
    working_directory: StablePath,
}

struct RunContext<'a> {
    cell_id: &'a str,
    cell_plan: &'a CellCommandPlan,
    engine: &'a str,
    engine_command: &'a EngineCommand,
    engine_order: &'a [&'a str],
    implementation: &'a Map<String, Value>,
    ordinal: usize,
    pair_id: &'a str,
    pair_index: usize,
    pairing_sha256: &'a str,
    phase: &'a str,
    policy_sha256: &'a str,
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
            "usage: ferric-m1-speculation {COMMAND} POLICY COMMAND-PLAN OUTPUT-OBSERVATIONS"
        ));
    };
    let policy_path = Path::new(policy);
    let command_plan_path = Path::new(command_plan);
    let output_path = Path::new(output);
    if policy_path == command_plan_path
        || policy_path == output_path
        || command_plan_path == output_path
    {
        return Err("R32 collector inputs and output must be distinct paths".to_owned());
    }
    require_collector_protocol()?;
    let policy = HeldDocument::load(policy_path, "R32 comparison policy")?;
    let command_plan = HeldDocument::load(command_plan_path, "R32 paired command plan")?;
    if policy.file.identity() == command_plan.file.identity() {
        return Err("R32 comparison policy and command plan must not alias".to_owned());
    }
    m1_r32_speculation_records::validate_policy_for_collection(&policy.value)?;
    m1_r32_speculation_records::require_collected_output_absent(output_path)?;
    let policy_sha256 = sha256_identity(&policy.bytes);
    let execution = validate_command_plan(&command_plan.value, &policy.value, &policy_sha256)?;
    policy.revalidate("R32 comparison policy")?;
    command_plan.revalidate("R32 paired command plan")?;

    let rows = collect_rows(&policy, &command_plan, &policy_sha256, &execution)?;
    execution
        .executable
        .revalidate("R32 benchmark executable")?;
    let final_executable_sha256 = digest_file(
        &execution.executable.file,
        "R32 benchmark executable after collection",
    )?;
    let expected_executable_sha256 = policy.value["plan"]["benchmark_executable_sha256"]
        .as_str()
        .ok_or_else(|| "R32 policy executable identity is absent".to_owned())?;
    if final_executable_sha256 != expected_executable_sha256 {
        return Err("R32 benchmark executable bytes changed during collection".to_owned());
    }
    execution
        .executable
        .revalidate("R32 benchmark executable")?;
    execution
        .working_directory
        .revalidate("R32 working directory")?;
    policy.revalidate("R32 comparison policy")?;
    command_plan.revalidate("R32 paired command plan")?;

    let observations =
        m1_r32_speculation_records::collected_observations(&policy.value, &policy_sha256, rows)?;
    let bytes = encode_canonical_document(&observations)?;
    policy.revalidate("R32 comparison policy")?;
    command_plan.revalidate("R32 paired command plan")?;
    m1_r32_speculation_records::publish_collected_observations(output_path, &bytes)
}

fn validate_command_plan(
    value: &Value,
    policy: &Value,
    policy_sha256: &str,
) -> BenchResult<ExecutionPlan> {
    let object = exact_object(value, COMMAND_PLAN_KEYS, "R32 paired command plan")?;
    expect_string(
        object,
        "authority",
        COMMAND_PLAN_AUTHORITY,
        "R32 paired command plan",
    )?;
    expect_string(
        object,
        "format",
        COMMAND_PLAN_FORMAT,
        "R32 paired command plan",
    )?;
    expect_string(
        object,
        "nonclaim",
        COMMAND_PLAN_NONCLAIM,
        "R32 paired command plan",
    )?;
    expect_string(object, "obligation_id", "m1.r32", "R32 paired command plan")?;
    expect_string(
        object,
        "policy_sha256",
        policy_sha256,
        "R32 paired command plan",
    )?;
    expect_string(object, "status", "pre-execution", "R32 paired command plan")?;
    expect_string(object, "target", TARGET, "R32 paired command plan")?;

    let policy_object = policy
        .as_object()
        .ok_or_else(|| "R32 comparison policy must be an object".to_owned())?;
    if field(object, "plan", "R32 paired command plan")?
        != field(policy_object, "plan", "R32 comparison policy")?
    {
        return Err("R32 command plan policy plan binding drifted".to_owned());
    }
    let policy_plan = policy["plan"]
        .as_object()
        .ok_or_else(|| "R32 policy plan must be an object".to_owned())?;
    let environment_value = field(object, "environment", "R32 paired command plan")?;
    let environment = validate_environment(environment_value)?;
    let environment_sha256 = sha256_identity(&encode_canonical_document(environment_value)?);
    if policy_plan["environment_sha256"].as_str() != Some(&environment_sha256) {
        return Err("R32 command environment SHA-256 differs from the policy".to_owned());
    }

    let executable_object = exact_object(
        field(object, "executable", "R32 paired command plan")?,
        EXECUTABLE_KEYS,
        "R32 command executable",
    )?;
    let executable_path = safe_string(
        field(executable_object, "path", "R32 command executable")?,
        "R32 command executable path",
    )?;
    let executable_sha256 = sha_string(
        field(executable_object, "sha256", "R32 command executable")?,
        "R32 command executable SHA-256",
    )?;
    if policy_plan["benchmark_executable_sha256"].as_str() != Some(executable_sha256) {
        return Err("R32 command executable identity differs from the policy".to_owned());
    }
    let executable = StablePath::executable(executable_path, executable_sha256)?;
    let working_directory_path = safe_string(
        field(object, "working_directory", "R32 paired command plan")?,
        "R32 working directory",
    )?;
    let working_directory = StablePath::directory(working_directory_path)?;
    let timeout_seconds = positive_u64(
        field(object, "timeout_seconds", "R32 paired command plan")?,
        "R32 command timeout seconds",
    )?;
    if timeout_seconds > MAX_TIMEOUT_SECONDS {
        return Err("R32 command timeout exceeds one day".to_owned());
    }

    let policy_cells = policy["cells"]
        .as_array()
        .ok_or_else(|| "R32 policy cells must be an array".to_owned())?;
    let cells = field(object, "cells", "R32 paired command plan")?
        .as_array()
        .ok_or_else(|| "R32 command cells must be an array".to_owned())?;
    if cells.len() != CELL_IDS.len() || policy_cells.len() != CELL_IDS.len() {
        return Err("R32 command cell roster is incomplete".to_owned());
    }
    let mut parsed_cells = Vec::with_capacity(CELL_IDS.len());
    for ((cell, policy_cell), expected_id) in cells.iter().zip(policy_cells).zip(CELL_IDS) {
        parsed_cells.push(validate_command_cell(
            cell,
            policy_cell,
            &policy["implementations"],
            expected_id,
            executable_sha256,
            &environment_sha256,
        )?);
    }
    Ok(ExecutionPlan {
        cells: parsed_cells,
        environment,
        executable,
        timeout: Duration::from_secs(timeout_seconds),
        working_directory,
    })
}

fn validate_command_cell(
    value: &Value,
    policy_cell: &Value,
    policy_implementations: &Value,
    expected_id: &str,
    executable_sha256: &str,
    environment_sha256: &str,
) -> BenchResult<CellCommandPlan> {
    let object = exact_object(value, COMMAND_CELL_KEYS, "R32 command cell")?;
    expect_string(object, "id", expected_id, "R32 command cell")?;
    for key in [
        "deterministic_admitted_plan_sha256",
        "holdout_member",
        "workload",
    ] {
        if field(object, key, "R32 command cell")? != &policy_cell[key] {
            return Err(format!(
                "R32 command cell {key} binding drifted: {expected_id}"
            ));
        }
    }
    let workload_sha256 = policy_cell["workload"]["workload_sha256"]
        .as_str()
        .ok_or_else(|| format!("R32 policy workload identity is absent: {expected_id}"))?
        .to_owned();
    let holdout_sha256 = policy_cell["holdout_member"]["sha256"]
        .as_str()
        .ok_or_else(|| format!("R32 policy holdout identity is absent: {expected_id}"))?
        .to_owned();
    let implementations = policy_implementations
        .as_array()
        .ok_or_else(|| "R32 policy implementations must be an array".to_owned())?;
    let commands = field(object, "commands", "R32 command cell")?
        .as_array()
        .ok_or_else(|| "R32 cell commands must be an array".to_owned())?;
    if commands.len() != ENGINES.len() || implementations.len() != ENGINES.len() {
        return Err(format!("R32 command roster is incomplete: {expected_id}"));
    }
    let mut parsed_commands = Vec::with_capacity(ENGINES.len());
    for (((command, implementation), engine), implementation_index) in
        commands.iter().zip(implementations).zip(ENGINES).zip(0..)
    {
        let command_object = exact_object(command, COMMAND_KEYS, "R32 engine command")?;
        expect_string(command_object, "engine", engine, "R32 engine command")?;
        if field(command_object, "implementation", "R32 engine command")? != implementation {
            return Err(format!(
                "R32 command implementation binding drifted: {expected_id}/{engine}"
            ));
        }
        if implementation["id"].as_str() != Some(engine) {
            return Err(format!(
                "R32 policy implementation order drifted: {implementation_index}"
            ));
        }
        let arguments_value = field(command_object, "arguments", "R32 engine command")?;
        let arguments = validate_arguments(arguments_value, expected_id, engine)?;
        let identity = json!({
            "arguments": arguments_value,
            "cell_id": expected_id,
            "engine": engine,
            "environment_sha256": environment_sha256,
            "executable_sha256": executable_sha256,
            "implementation": implementation,
            "workload_sha256": workload_sha256,
        });
        let expected_command_sha256 = sha256_identity(&encode_canonical_document(&identity)?);
        expect_string(
            command_object,
            "command_sha256",
            &expected_command_sha256,
            "R32 engine command",
        )?;
        parsed_commands.push(EngineCommand {
            arguments,
            command_sha256: expected_command_sha256,
        });
    }
    Ok(CellCommandPlan {
        commands: parsed_commands,
        deterministic_plan: policy_cell["deterministic_admitted_plan_sha256"].clone(),
        holdout_sha256,
        workload_sha256,
    })
}

fn collect_rows(
    policy: &HeldDocument,
    command_plan: &HeldDocument,
    policy_sha256: &str,
    execution: &ExecutionPlan,
) -> BenchResult<Vec<Value>> {
    let policy_cells = policy.value["cells"]
        .as_array()
        .ok_or_else(|| "R32 policy cells must be an array".to_owned())?;
    let implementations = policy.value["implementations"]
        .as_array()
        .ok_or_else(|| "R32 policy implementations must be an array".to_owned())?;
    let mut rows = Vec::with_capacity(CELL_IDS.len() * (WARMUP_PAIRS + RECORDED_PAIRS));
    let mut ordinal = 0_usize;
    for (cell_index, cell_id) in CELL_IDS.iter().enumerate() {
        let policy_cell = &policy_cells[cell_index];
        let cell_plan = &execution.cells[cell_index];
        let pair_roster = policy_cell["pair_roster"]
            .as_array()
            .ok_or_else(|| format!("R32 policy pair roster is absent: {cell_id}"))?;
        let mut roster_index = 0_usize;
        for (phase, count) in [("warmup", WARMUP_PAIRS), ("recorded", RECORDED_PAIRS)] {
            for pair_index in 0..count {
                let pair = pair_roster.get(roster_index).ok_or_else(|| {
                    format!("R32 policy pair is absent: {cell_id}/{roster_index}")
                })?;
                let pair_id = pair["id"]
                    .as_str()
                    .ok_or_else(|| "R32 policy pair ID is absent".to_owned())?;
                let pairing_sha256 = pair["pairing_sha256"]
                    .as_str()
                    .ok_or_else(|| "R32 policy pairing identity is absent".to_owned())?;
                let engine_order = (0..ENGINES.len())
                    .map(|offset| ENGINES[(ordinal + offset) % ENGINES.len()])
                    .collect::<Vec<_>>();
                let mut values = Map::new();
                for engine in &engine_order {
                    let engine_index = ENGINES
                        .iter()
                        .position(|candidate| candidate == engine)
                        .ok_or_else(|| "R32 engine order is invalid".to_owned())?;
                    let engine_command = &cell_plan.commands[engine_index];
                    let implementation = implementations[engine_index]
                        .as_object()
                        .ok_or_else(|| "R32 policy implementation must be an object".to_owned())?;
                    policy.revalidate("R32 comparison policy")?;
                    command_plan.revalidate("R32 paired command plan")?;
                    execution
                        .executable
                        .revalidate("R32 benchmark executable")?;
                    execution
                        .working_directory
                        .revalidate("R32 working directory")?;
                    let context = RunContext {
                        cell_id,
                        cell_plan,
                        engine,
                        engine_command,
                        engine_order: &engine_order,
                        implementation,
                        ordinal,
                        pair_id,
                        pair_index,
                        pairing_sha256,
                        phase,
                        policy_sha256,
                    };
                    let counters = execute_engine(execution, &context)?;
                    execution
                        .executable
                        .revalidate("R32 benchmark executable")?;
                    policy.revalidate("R32 comparison policy")?;
                    command_plan.revalidate("R32 paired command plan")?;
                    values.insert((*engine).to_owned(), counters);
                }
                require_equal_work(&values, pair_id)?;
                rows.push(json!({
                    "cell_id": cell_id,
                    "engine_order": engine_order,
                    "faults": [],
                    "id": pair_id,
                    "ordinal": ordinal,
                    "pair_index": pair_index,
                    "pairing_sha256": pairing_sha256,
                    "phase": phase,
                    "status": "passed",
                    "values": values,
                }));
                ordinal += 1;
                roster_index += 1;
            }
        }
    }
    Ok(rows)
}

fn execute_engine(execution: &ExecutionPlan, context: &RunContext<'_>) -> BenchResult<Value> {
    let mut command = Command::new(&execution.executable.path);
    command
        .args(&context.engine_command.arguments)
        .current_dir(&execution.working_directory.path)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (name, value) in &execution.environment {
        command.env(name, value);
    }
    set_context_environment(&mut command, context)?;
    let mut child = command.spawn().map_err(|error| {
        format!(
            "cannot start R32 command {}/{}: {error}",
            context.pair_id, context.engine
        )
    })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "R32 command stdout pipe is absent".to_owned())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "R32 command stderr pipe is absent".to_owned())?;
    let stdout_reader = thread::spawn(move || read_capped(stdout));
    let stderr_reader = thread::spawn(move || read_capped(stderr));
    let deadline = Instant::now()
        .checked_add(execution.timeout)
        .ok_or_else(|| "R32 command deadline overflowed".to_owned())?;
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|error| {
            format!(
                "cannot inspect R32 command {}/{}: {error}",
                context.pair_id, context.engine
            )
        })? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err(format!(
                "R32 command timed out: {}/{}",
                context.pair_id, context.engine
            ));
        }
        thread::sleep(Duration::from_millis(10));
    };
    let stdout = join_reader(stdout_reader, "stdout", context)?;
    let stderr = join_reader(stderr_reader, "stderr", context)?;
    if !status.success() {
        return Err(format!(
            "R32 command failed: {}/{} status={} stderr={}",
            context.pair_id,
            context.engine,
            status,
            diagnostic(&stderr)
        ));
    }
    if !stderr.is_empty() {
        return Err(format!(
            "R32 command wrote stderr despite success: {}/{} stderr={}",
            context.pair_id,
            context.engine,
            diagnostic(&stderr)
        ));
    }
    let result = parse_canonical_runner_output(&stdout, context)?;
    validate_runner_result(&result, context)
}

fn set_context_environment(command: &mut Command, context: &RunContext<'_>) -> BenchResult<()> {
    let implementation = context.implementation;
    let bindings = [
        (
            "FERRIC_M1_R32_ARTIFACT_SHA256",
            implementation_string(implementation, "artifact_sha256")?,
        ),
        ("FERRIC_M1_R32_CELL_ID", context.cell_id),
        (
            "FERRIC_M1_R32_COMMAND_SHA256",
            &context.engine_command.command_sha256,
        ),
        (
            "FERRIC_M1_R32_CONFIG_SHA256",
            implementation_string(implementation, "config_sha256")?,
        ),
        ("FERRIC_M1_R32_ENGINE", context.engine),
        (
            "FERRIC_M1_R32_HOLDOUT_SHA256",
            &context.cell_plan.holdout_sha256,
        ),
        (
            "FERRIC_M1_R32_IMPLEMENTATION_SHA256",
            implementation_string(implementation, "implementation_sha256")?,
        ),
        ("FERRIC_M1_R32_PAIR_ID", context.pair_id),
        ("FERRIC_M1_R32_PAIRING_SHA256", context.pairing_sha256),
        ("FERRIC_M1_R32_PHASE", context.phase),
        ("FERRIC_M1_R32_POLICY_SHA256", context.policy_sha256),
        (
            "FERRIC_M1_R32_PROTOCOL_SHA256",
            implementation_string(implementation, "protocol_sha256")?,
        ),
        (
            "FERRIC_M1_R32_SOURCE_SHA256",
            implementation_string(implementation, "source_sha256")?,
        ),
        ("FERRIC_M1_R32_TARGET", TARGET),
        (
            "FERRIC_M1_R32_VERSION",
            implementation_string(implementation, "version")?,
        ),
        (
            "FERRIC_M1_R32_WORKLOAD_SHA256",
            &context.cell_plan.workload_sha256,
        ),
    ];
    for (name, value) in bindings {
        command.env(name, value);
    }
    command.env(
        "FERRIC_M1_R32_DETERMINISTIC_ADMITTED_PLAN_SHA256",
        context.cell_plan.deterministic_plan.as_str().unwrap_or(""),
    );
    command.env("FERRIC_M1_R32_ENGINE_ORDER", context.engine_order.join(","));
    command.env("FERRIC_M1_R32_ORDINAL", context.ordinal.to_string());
    command.env("FERRIC_M1_R32_PAIR_INDEX", context.pair_index.to_string());
    Ok(())
}

fn validate_runner_result(value: &Value, context: &RunContext<'_>) -> BenchResult<Value> {
    let object = exact_object(value, RUN_RESULT_KEYS, "R32 command result")?;
    let implementation = context.implementation;
    for (key, expected) in [
        (
            "artifact_sha256",
            implementation_string(implementation, "artifact_sha256")?,
        ),
        ("authority", RUN_RESULT_AUTHORITY),
        ("cell_id", context.cell_id),
        (
            "command_sha256",
            context.engine_command.command_sha256.as_str(),
        ),
        (
            "config_sha256",
            implementation_string(implementation, "config_sha256")?,
        ),
        ("engine", context.engine),
        ("format", RUN_RESULT_FORMAT),
        ("holdout_sha256", context.cell_plan.holdout_sha256.as_str()),
        (
            "implementation_sha256",
            implementation_string(implementation, "implementation_sha256")?,
        ),
        ("pair_id", context.pair_id),
        ("pairing_sha256", context.pairing_sha256),
        ("phase", context.phase),
        ("policy_sha256", context.policy_sha256),
        (
            "protocol_sha256",
            implementation_string(implementation, "protocol_sha256")?,
        ),
        (
            "source_sha256",
            implementation_string(implementation, "source_sha256")?,
        ),
        ("status", "passed"),
        ("target", TARGET),
        ("version", implementation_string(implementation, "version")?),
        (
            "workload_sha256",
            context.cell_plan.workload_sha256.as_str(),
        ),
    ] {
        expect_string(object, key, expected, "R32 command result")?;
    }
    if field(
        object,
        "deterministic_admitted_plan_sha256",
        "R32 command result",
    )? != &context.cell_plan.deterministic_plan
    {
        return Err(format!(
            "R32 command deterministic-plan binding drifted: {}/{}",
            context.pair_id, context.engine
        ));
    }
    if field(object, "engine_order", "R32 command result")? != &json!(context.engine_order) {
        return Err(format!(
            "R32 command engine-order binding drifted: {}/{}",
            context.pair_id, context.engine
        ));
    }
    expect_u64(
        object,
        "ordinal",
        context.ordinal as u64,
        "R32 command result",
    )?;
    expect_u64(
        object,
        "pair_index",
        context.pair_index as u64,
        "R32 command result",
    )?;
    let counter_keys = if context.engine == "speculative" {
        SPECULATIVE_COUNTER_KEYS
    } else {
        TARGET_ONLY_COUNTER_KEYS
    };
    let counters = exact_object(
        field(object, "counters", "R32 command result")?,
        counter_keys,
        "R32 command counters",
    )?;
    for key in [
        "duration_ns",
        "p99_latency_ns",
        "successful_requests",
        "target_invocations",
        "total_tokens",
    ] {
        let _ = positive_u64(
            field(counters, key, "R32 command counters")?,
            &format!("R32 command counter {key}"),
        )?;
    }
    expect_u64(counters, "failed_requests", 0, "R32 command counters")?;
    if context.engine == "speculative" {
        let accepted = unsigned_u64(
            field(counters, "accepted_tokens", "R32 command counters")?,
            "R32 accepted tokens",
        )?;
        let total = positive_u64(
            field(counters, "total_tokens", "R32 command counters")?,
            "R32 total tokens",
        )?;
        if accepted > total {
            return Err(format!(
                "R32 command accepted tokens exceed output: {}",
                context.pair_id
            ));
        }
    }
    Ok(Value::Object(counters.clone()))
}

fn require_equal_work(values: &Map<String, Value>, pair_id: &str) -> BenchResult<()> {
    let speculative = values
        .get("speculative")
        .and_then(Value::as_object)
        .ok_or_else(|| format!("R32 speculative counters are absent: {pair_id}"))?;
    let target_only = values
        .get("target-only")
        .and_then(Value::as_object)
        .ok_or_else(|| format!("R32 target-only counters are absent: {pair_id}"))?;
    for key in ["successful_requests", "total_tokens"] {
        if speculative.get(key) != target_only.get(key) {
            return Err(format!(
                "R32 pair does not contain equal work: {pair_id}/{key}"
            ));
        }
    }
    Ok(())
}

fn validate_environment(value: &Value) -> BenchResult<Vec<(String, String)>> {
    let object = value
        .as_object()
        .ok_or_else(|| "R32 command environment must be an object".to_owned())?;
    if object.len() > 128 {
        return Err("R32 command environment contains too many variables".to_owned());
    }
    let mut environment = Vec::with_capacity(object.len());
    for (name, value) in object {
        if !valid_environment_name(name) || name.starts_with("FERRIC_M1_R32_") {
            return Err(format!(
                "R32 command environment variable is not admitted: {name}"
            ));
        }
        let value = value
            .as_str()
            .ok_or_else(|| format!("R32 command environment value must be a string: {name}"))?;
        if value.len() > 16 * 1024 || value.contains('\0') {
            return Err(format!(
                "R32 command environment value is not admitted: {name}"
            ));
        }
        environment.push((name.clone(), value.to_owned()));
    }
    Ok(environment)
}

fn validate_arguments(value: &Value, cell: &str, engine: &str) -> BenchResult<Vec<OsString>> {
    let arguments = value
        .as_array()
        .ok_or_else(|| format!("R32 command arguments must be an array: {cell}/{engine}"))?;
    if arguments.len() > 256 {
        return Err(format!(
            "R32 command has too many arguments: {cell}/{engine}"
        ));
    }
    let mut total = 0_usize;
    let mut parsed = Vec::with_capacity(arguments.len());
    for argument in arguments {
        let argument = argument
            .as_str()
            .ok_or_else(|| format!("R32 command argument must be a string: {cell}/{engine}"))?;
        total = total
            .checked_add(argument.len())
            .ok_or_else(|| "R32 command argument extent overflowed".to_owned())?;
        if argument.contains('\0') || total > MAX_ARGUMENT_BYTES {
            return Err(format!(
                "R32 command arguments are too large: {cell}/{engine}"
            ));
        }
        parsed.push(OsString::from(argument));
    }
    Ok(parsed)
}

fn parse_canonical_runner_output(bytes: &[u8], context: &RunContext<'_>) -> BenchResult<Value> {
    if bytes.is_empty() || !bytes.is_ascii() {
        return Err(format!(
            "R32 command output must be nonempty ASCII JSON: {}/{}",
            context.pair_id, context.engine
        ));
    }
    let value: Value = serde_json::from_slice(bytes).map_err(|error| {
        format!(
            "cannot parse R32 command output {}/{}: {error}",
            context.pair_id, context.engine
        )
    })?;
    if encode_canonical_document(&value)? != bytes {
        return Err(format!(
            "R32 command output is not canonical JSON: {}/{}",
            context.pair_id, context.engine
        ));
    }
    Ok(value)
}

fn read_capped(mut reader: impl Read) -> BenchResult<Vec<u8>> {
    let mut bytes = Vec::new();
    Read::by_ref(&mut reader)
        .take((MAX_RUNNER_OUTPUT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("cannot read R32 command output: {error}"))?;
    if bytes.len() > MAX_RUNNER_OUTPUT_BYTES {
        return Err("R32 command output exceeded 64 KiB".to_owned());
    }
    Ok(bytes)
}

fn join_reader(
    handle: thread::JoinHandle<BenchResult<Vec<u8>>>,
    stream: &str,
    context: &RunContext<'_>,
) -> BenchResult<Vec<u8>> {
    handle
        .join()
        .map_err(|_| {
            format!(
                "R32 command {stream} reader panicked: {}/{}",
                context.pair_id, context.engine
            )
        })?
        .map_err(|error| {
            format!(
                "R32 command {stream} read failed {}/{}: {error}",
                context.pair_id, context.engine
            )
        })
}

fn diagnostic(bytes: &[u8]) -> String {
    String::from_utf8_lossy(&bytes[..bytes.len().min(256)]).replace(['\n', '\r'], " ")
}

fn require_collector_protocol() -> BenchResult<()> {
    let manifest = std::env::var_os("CARGO_MANIFEST_DIR")
        .ok_or_else(|| "R32 collector manifest directory is absent".to_owned())?;
    let path = PathBuf::from(manifest).join("m1_r32_paired_collector_protocol.json");
    let (_, value, bytes, file) =
        load_canonical_document_held(&path, "R32 paired collector protocol")?;
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
        "R32 paired collector protocol",
    )?;
    expect_string(
        object,
        "authority",
        "ferric-m1-r32-paired-collector-protocol-only",
        "R32 paired collector protocol",
    )?;
    expect_string(
        object,
        "format",
        "FERRIC-M1-R32-PAIRED-COLLECTOR-PROTOCOL-V1",
        "R32 paired collector protocol",
    )?;
    expect_string(
        object,
        "nonclaim",
        COMMAND_PLAN_NONCLAIM,
        "R32 paired collector protocol",
    )?;
    expect_string(
        object,
        "obligation_id",
        "m1.r32",
        "R32 paired collector protocol",
    )?;
    expect_string(
        object,
        "status",
        "collector-protocol",
        "R32 paired collector protocol",
    )?;
    expect_string(object, "target", TARGET, "R32 paired collector protocol")?;
    if sha256_identity(&bytes) != COLLECTOR_PROTOCOL_SHA256 {
        return Err("R32 paired collector protocol SHA-256 drifted".to_owned());
    }
    file.validate_snapshot("R32 paired collector protocol")
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
    let digest = digest.finalize();
    let mut identity = String::with_capacity(64);
    for byte in digest {
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

fn implementation_string<'a>(
    implementation: &'a Map<String, Value>,
    key: &str,
) -> BenchResult<&'a str> {
    implementation
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("R32 implementation binding is absent: {key}"))
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

    static NONCE: AtomicU64 = AtomicU64::new(0);

    struct Temporary(PathBuf);

    impl Temporary {
        fn new() -> Self {
            let nonce = NONCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "ferric-m1-r32-paired-collector-test.{}.{nonce}",
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
            "artifact_sha256": digest(&format!("{id}-artifact")),
            "config_sha256": digest(&format!("{id}-config")),
            "id": id,
            "implementation_sha256": digest(&format!("{id}-implementation")),
            "protocol_sha256": digest(&format!("{id}-protocol")),
            "source_sha256": digest("ferric-source"),
            "version": "pinned-ferric-version",
        })
    }

    fn workload(acceptance: &str) -> Value {
        let identity = json!({
            "acceptance": acceptance,
            "arrival": "closed-loop",
            "arrival_trace_sha256": digest(&format!("{acceptance}-arrival")),
            "batch": 1,
            "decode_kv_length": 8192,
            "draft_length": 4,
            "isl_osl": "4096x256",
            "output_limits_sha256": digest(&format!("{acceptance}-limits")),
            "prefix_sharing_percent": 0,
            "prefill_length": 128,
            "prompt_order_sha256": digest(&format!("{acceptance}-prompts")),
            "sampling_seed_sha256": digest(&format!("{acceptance}-seed")),
        });
        let mut workload = identity.as_object().unwrap().clone();
        workload.remove("acceptance");
        workload.insert(
            "workload_sha256".to_owned(),
            Value::String(document_digest(&identity)),
        );
        Value::Object(workload)
    }

    fn pair_roster(cell_id: &str) -> Vec<Value> {
        let mut pairs = Vec::new();
        for (phase, count) in [("warmup", WARMUP_PAIRS), ("recorded", RECORDED_PAIRS)] {
            for index in 0..count {
                let id = format!("{cell_id}.{phase}-{index:02}");
                pairs.push(json!({
                    "id": id,
                    "pairing_sha256": digest(&format!("pair/{id}")),
                }));
            }
        }
        pairs
    }

    fn cell(id: &str) -> Value {
        let acceptance = if id == "eligible-speculation" {
            "mixed"
        } else {
            "low"
        };
        json!({
            "acceptance": acceptance,
            "case_kind": "speculative-s1-k4-c8192",
            "deterministic_admitted_plan_sha256": if id == "low-acceptance" { Value::String(digest("low-plan")) } else { Value::Null },
            "eligible": true,
            "holdout_member": {"id": format!("{id}-member"), "sha256": digest(&format!("{id}-holdout"))},
            "id": id,
            "p99_slo_ns": 10_000,
            "pair_roster": pair_roster(id),
            "workload": workload(acceptance),
        })
    }

    fn runner_script() -> String {
        r#"#!/usr/bin/python3
import json
import os

engine = os.environ["FERRIC_M1_R32_ENGINE"]
cell = os.environ["FERRIC_M1_R32_CELL_ID"]
counters = {
    "duration_ns": 800 if engine == "speculative" and cell == "eligible-speculation" else (1050 if engine == "speculative" else 1000),
    "failed_requests": 0,
    "p99_latency_ns": 100,
    "successful_requests": 4,
    "target_invocations": 40 if engine == "speculative" else 100,
    "total_tokens": 100,
}
if engine == "speculative":
    counters["accepted_tokens"] = 80 if cell == "eligible-speculation" else 5
deterministic = os.environ["FERRIC_M1_R32_DETERMINISTIC_ADMITTED_PLAN_SHA256"]
value = {
    "artifact_sha256": os.environ["FERRIC_M1_R32_ARTIFACT_SHA256"],
    "authority": "ferric-r32-command-reported-raw-counters-only",
    "cell_id": cell,
    "command_sha256": os.environ["FERRIC_M1_R32_COMMAND_SHA256"],
    "config_sha256": os.environ["FERRIC_M1_R32_CONFIG_SHA256"],
    "counters": counters,
    "deterministic_admitted_plan_sha256": deterministic if deterministic else None,
    "engine": engine,
    "engine_order": os.environ["FERRIC_M1_R32_ENGINE_ORDER"].split(","),
    "format": "FERRIC-M1-R32-PAIRED-COMMAND-RESULT-V1",
    "holdout_sha256": os.environ["FERRIC_M1_R32_HOLDOUT_SHA256"],
    "implementation_sha256": os.environ["FERRIC_M1_R32_IMPLEMENTATION_SHA256"],
    "ordinal": int(os.environ["FERRIC_M1_R32_ORDINAL"]),
    "pair_id": os.environ["FERRIC_M1_R32_PAIR_ID"],
    "pair_index": int(os.environ["FERRIC_M1_R32_PAIR_INDEX"]),
    "pairing_sha256": os.environ["FERRIC_M1_R32_PAIRING_SHA256"],
    "phase": os.environ["FERRIC_M1_R32_PHASE"],
    "policy_sha256": os.environ["FERRIC_M1_R32_POLICY_SHA256"],
    "protocol_sha256": os.environ["FERRIC_M1_R32_PROTOCOL_SHA256"],
    "source_sha256": os.environ["FERRIC_M1_R32_SOURCE_SHA256"],
    "status": "passed",
    "target": os.environ["FERRIC_M1_R32_TARGET"],
    "version": os.environ["FERRIC_M1_R32_VERSION"],
    "workload_sha256": os.environ["FERRIC_M1_R32_WORKLOAD_SHA256"],
}
print(json.dumps(value, indent=2, sort_keys=True))
"#
        .to_owned()
    }

    fn fixtures(root: &Path) -> (Value, Value) {
        let executable_path = root.join("runner.py");
        fs::write(&executable_path, runner_script()).unwrap();
        let mut permissions = fs::metadata(&executable_path).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&executable_path, permissions).unwrap();
        let executable_sha256 = sha256_identity(&fs::read(&executable_path).unwrap());
        let environment = json!({});
        let implementations = ENGINES
            .iter()
            .map(|id| implementation(id))
            .collect::<Vec<_>>();
        let cells = CELL_IDS.iter().map(|id| cell(id)).collect::<Vec<_>>();
        let plan = json!({
            "benchmark_executable_sha256": executable_sha256,
            "benchmark_plan_sha256": digest("benchmark-plan"),
            "draft_artifact_sha256": digest("draft-artifact"),
            "environment_sha256": document_digest(&environment),
            "fe2o3_source_closure_sha256": digest("fe2o3-source"),
            "ferric_source_closure_sha256": digest("ferric-source"),
            "generated_plan_sha256": digest("generated-plan"),
            "model_sha256": digest("model"),
            "schedule_sha256": digest("schedule"),
            "target_artifact_sha256": digest("target-artifact"),
            "tokenizer_sha256": digest("tokenizer"),
            "weights_sha256": digest("weights"),
        });
        let policy = json!({
            "authority": "external-pre-observation-speculation-comparison-policy-only",
            "cells": cells,
            "engine_order": ENGINES,
            "format": "FERRIC-M1-R32-SPECULATION-COMPARISON-POLICY-V1",
            "implementations": implementations,
            "nonclaim": "This record authenticates an externally frozen eligible holdout, low-acceptance deterministic-plan cell, exact paired sample roster, Ferric speculative and target-only identities, and raw counters. It recomputes integer throughput, exact rational medians, the 1.10 eligible throughput gate, 1.05 eligible p99-latency ceiling, and 0.95 low-acceptance throughput floor. It does not validate external eligibility, holdout selection, plan admission, source or artifact correctness, collector behavior, observation truth, hardware behavior, numerical correctness, independent reproduction, or qualification; it is partial non-evidence and does not close m1.r32 or M1.",
            "obligation_id": "m1.r32",
            "plan": plan,
            "protocol_sha256": "26b7695b204f8994ddb61e9dfe860114a1ea8e628a4ccb991030a0ad06197ea0",
            "status": "pre-observation",
            "target": TARGET,
            "thresholds": {
                "eligible_latency_max_ratio_ppm": 1_050_000,
                "eligible_throughput_min_ratio_ppm": 1_100_000,
                "low_acceptance_throughput_min_ratio_ppm": 950_000,
                "recorded_pairs": RECORDED_PAIRS,
                "warmup_pairs": WARMUP_PAIRS,
            },
        });
        let policy_sha256 = document_digest(&policy);
        let command_cells = CELL_IDS
            .iter()
            .enumerate()
            .map(|(cell_index, id)| {
                let commands = ENGINES
                    .iter()
                    .enumerate()
                    .map(|(engine_index, engine)| {
                        let arguments = json!([]);
                        let identity = json!({
                            "arguments": arguments,
                            "cell_id": id,
                            "engine": engine,
                            "environment_sha256": policy["plan"]["environment_sha256"],
                            "executable_sha256": policy["plan"]["benchmark_executable_sha256"],
                            "implementation": policy["implementations"][engine_index],
                            "workload_sha256": policy["cells"][cell_index]["workload"]["workload_sha256"],
                        });
                        json!({
                            "arguments": [],
                            "command_sha256": document_digest(&identity),
                            "engine": engine,
                            "implementation": policy["implementations"][engine_index],
                        })
                    })
                    .collect::<Vec<_>>();
                json!({
                    "commands": commands,
                    "deterministic_admitted_plan_sha256": policy["cells"][cell_index]["deterministic_admitted_plan_sha256"],
                    "holdout_member": policy["cells"][cell_index]["holdout_member"],
                    "id": id,
                    "workload": policy["cells"][cell_index]["workload"],
                })
            })
            .collect::<Vec<_>>();
        let commands = json!({
            "authority": COMMAND_PLAN_AUTHORITY,
            "cells": command_cells,
            "environment": environment,
            "executable": {"path": executable_path, "sha256": policy["plan"]["benchmark_executable_sha256"]},
            "format": COMMAND_PLAN_FORMAT,
            "nonclaim": COMMAND_PLAN_NONCLAIM,
            "obligation_id": "m1.r32",
            "plan": policy["plan"],
            "policy_sha256": policy_sha256,
            "status": "pre-execution",
            "target": TARGET,
            "timeout_seconds": 5,
            "working_directory": root,
        });
        (policy, commands)
    }

    #[test]
    fn real_separate_commands_produce_exact_checker_observations() {
        let temporary = Temporary::new();
        let (policy, commands) = fixtures(&temporary.0);
        let policy_path = temporary.0.join("policy.json");
        let commands_path = temporary.0.join("commands.json");
        let output_path = temporary.0.join("observations.json");
        fs::write(&policy_path, encode_canonical_document(&policy).unwrap()).unwrap();
        fs::write(
            &commands_path,
            encode_canonical_document(&commands).unwrap(),
        )
        .unwrap();
        run(vec![
            policy_path.clone().into_os_string(),
            commands_path.into_os_string(),
            output_path.clone().into_os_string(),
        ])
        .unwrap();
        let (_, observations, _, file) =
            load_canonical_document_held(&output_path, "test R32 observations").unwrap();
        file.validate_snapshot("test R32 observations").unwrap();
        assert_eq!(observations["rows"].as_array().unwrap().len(), 80);
        assert_eq!(observations["rows"][0]["engine_order"], json!(ENGINES));
        assert_eq!(
            observations["rows"][1]["engine_order"],
            json!(["target-only", "speculative"])
        );
        assert_eq!(
            observations["rows"][0]["values"]["speculative"]["duration_ns"],
            800
        );
        assert_eq!(
            observations["rows"][0]["values"]["target-only"]["duration_ns"],
            1000
        );
        let record_path = temporary.0.join("record.json");
        assert_eq!(
            m1_r32_speculation_records::main_for_arguments(vec![
                policy_path.into_os_string(),
                output_path.into_os_string(),
                record_path.clone().into_os_string(),
            ]),
            ExitCode::SUCCESS
        );
        assert!(record_path.is_file());
    }

    #[test]
    fn command_plan_identity_and_environment_mutations_fail_closed() {
        let temporary = Temporary::new();
        let (policy, commands) = fixtures(&temporary.0);
        let policy_sha256 = document_digest(&policy);
        let mut command_identity = commands.clone();
        command_identity["cells"][0]["commands"][0]["arguments"] = json!(["--drift"]);
        let error = match validate_command_plan(&command_identity, &policy, &policy_sha256) {
            Err(error) => error,
            Ok(_) => panic!("mutated command identity was accepted"),
        };
        assert!(error.contains("command_sha256"));
        let mut environment = commands;
        environment["environment"]["UNBOUND"] = json!("value");
        let error = match validate_command_plan(&environment, &policy, &policy_sha256) {
            Err(error) => error,
            Ok(_) => panic!("mutated command environment was accepted"),
        };
        assert!(error.contains("environment SHA-256"));
    }

    #[test]
    fn unequal_work_is_rejected_before_publication() {
        let values = json!({
            "speculative": {
                "accepted_tokens": 1,
                "duration_ns": 1,
                "failed_requests": 0,
                "p99_latency_ns": 1,
                "successful_requests": 4,
                "target_invocations": 1,
                "total_tokens": 100,
            },
            "target-only": {
                "duration_ns": 1,
                "failed_requests": 0,
                "p99_latency_ns": 1,
                "successful_requests": 4,
                "target_invocations": 1,
                "total_tokens": 99,
            },
        });
        assert!(require_equal_work(values.as_object().unwrap(), "pair")
            .unwrap_err()
            .contains("equal work"));
    }
}
