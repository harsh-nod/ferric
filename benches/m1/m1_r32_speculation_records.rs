//! Exact post-observation record construction for paired M1 speculation runs.
//!
//! This checker recomputes arithmetic over externally collected raw pair
//! counters. It authenticates declarations and byte identities, not the truth
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

const POLICY_FORMAT: &str = "FERRIC-M1-R32-SPECULATION-COMPARISON-POLICY-V1";
const OBSERVATIONS_FORMAT: &str = "FERRIC-M1-R32-SPECULATION-COMPARISON-OBSERVATIONS-V1";
const RECORD_FORMAT: &str = "FERRIC-M1-R32-SPECULATION-COMPARISON-RECORD-V1";
const POLICY_AUTHORITY: &str = "external-pre-observation-speculation-comparison-policy-only";
const OBSERVATIONS_AUTHORITY: &str = "externally-collected-paired-speculation-counters-only";
const RECORD_AUTHORITY: &str =
    "ferric-checked-speculation-comparison-structure-and-arithmetic-only";
const STATUS: &str = "PARTIAL_NON_EVIDENCE";
const TARGET: &str = "gfx942:xnack-";
const WARMUP_PAIRS: usize = 10;
const RECORDED_PAIRS: usize = 30;
const RATE_SCALE: u128 = 1_000_000_000;
const PPM_SCALE: u128 = 1_000_000;
const ELIGIBLE_THROUGHPUT_MIN_PPM: u64 = 1_100_000;
const ELIGIBLE_LATENCY_MAX_PPM: u64 = 1_050_000;
const LOW_ACCEPTANCE_THROUGHPUT_MIN_PPM: u64 = 950_000;
const ENGINES: &[&str] = &["speculative", "target-only"];
const CELL_IDS: &[&str] = &["eligible-speculation", "low-acceptance"];
const CASE_KINDS: &[&str] = &[
    "speculative-s1-k16-c8192",
    "speculative-s1-k4-c8192",
    "speculative-s1-k8-c8192",
    "speculative-s8-k4-c8192",
];
const PROTOCOL_SHA256: &str = "26b7695b204f8994ddb61e9dfe860114a1ea8e628a4ccb991030a0ad06197ea0";
const NONCLAIM: &str = "This record authenticates an externally frozen eligible holdout, low-acceptance deterministic-plan cell, exact paired sample roster, Ferric speculative and target-only identities, and raw counters. It recomputes integer throughput, exact rational medians, the 1.10 eligible throughput gate, 1.05 eligible p99-latency ceiling, and 0.95 low-acceptance throughput floor. It does not validate external eligibility, holdout selection, plan admission, source or artifact correctness, collector behavior, observation truth, hardware behavior, numerical correctness, independent reproduction, or qualification; it is partial non-evidence and does not close m1.r32 or M1.";

const POLICY_KEYS: &[&str] = &[
    "authority",
    "cells",
    "engine_order",
    "format",
    "implementations",
    "nonclaim",
    "obligation_id",
    "plan",
    "protocol_sha256",
    "status",
    "target",
    "thresholds",
];
const PLAN_KEYS: &[&str] = &[
    "benchmark_executable_sha256",
    "benchmark_plan_sha256",
    "draft_artifact_sha256",
    "environment_sha256",
    "fe2o3_source_closure_sha256",
    "ferric_source_closure_sha256",
    "generated_plan_sha256",
    "model_sha256",
    "schedule_sha256",
    "target_artifact_sha256",
    "tokenizer_sha256",
    "weights_sha256",
];
const IMPLEMENTATION_KEYS: &[&str] = &[
    "artifact_sha256",
    "config_sha256",
    "id",
    "implementation_sha256",
    "protocol_sha256",
    "source_sha256",
    "version",
];
const THRESHOLD_KEYS: &[&str] = &[
    "eligible_latency_max_ratio_ppm",
    "eligible_throughput_min_ratio_ppm",
    "low_acceptance_throughput_min_ratio_ppm",
    "recorded_pairs",
    "warmup_pairs",
];
const CELL_KEYS: &[&str] = &[
    "acceptance",
    "case_kind",
    "deterministic_admitted_plan_sha256",
    "eligible",
    "holdout_member",
    "id",
    "p99_slo_ns",
    "pair_roster",
    "workload",
];
const HOLDOUT_KEYS: &[&str] = &["id", "sha256"];
const PAIR_KEYS: &[&str] = &["id", "pairing_sha256"];
const WORKLOAD_KEYS: &[&str] = &[
    "arrival",
    "arrival_trace_sha256",
    "batch",
    "decode_kv_length",
    "draft_length",
    "isl_osl",
    "output_limits_sha256",
    "prefix_sharing_percent",
    "prefill_length",
    "prompt_order_sha256",
    "sampling_seed_sha256",
    "workload_sha256",
];
const OBSERVATION_KEYS: &[&str] = &[
    "authority",
    "cells",
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
    "thresholds",
];
const ROW_KEYS: &[&str] = &[
    "cell_id",
    "engine_order",
    "faults",
    "id",
    "ordinal",
    "pair_index",
    "pairing_sha256",
    "phase",
    "status",
    "values",
];
const SPECULATIVE_VALUE_KEYS: &[&str] = &[
    "accepted_tokens",
    "duration_ns",
    "failed_requests",
    "p99_latency_ns",
    "successful_requests",
    "target_invocations",
    "total_tokens",
];
const TARGET_ONLY_VALUE_KEYS: &[&str] = &[
    "duration_ns",
    "failed_requests",
    "p99_latency_ns",
    "successful_requests",
    "target_invocations",
    "total_tokens",
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
            "speculation comparison policy",
        )?;
        self.observations_root.validate_binding(
            &self.observations_name,
            &self.observations_file,
            "speculation comparison observations",
        )
    }
}

#[derive(Debug)]
struct EngineSamples {
    latency: Vec<u64>,
    throughput: Vec<u64>,
}

#[derive(Debug)]
struct CellSamples {
    accepted_tokens: u128,
    engines: Vec<EngineSamples>,
    target_invocations: Vec<u128>,
}

struct ExpectedRow<'a> {
    cell_id: &'a str,
    ordinal: usize,
    pair_index: usize,
    phase: &'a str,
    roster_pair: &'a Value,
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
            "usage: ferric-m1-speculation {COMMAND} POLICY OBSERVATIONS OUTPUT-RECORD"
        ));
    };
    let policy_path = Path::new(policy);
    let observations_path = Path::new(observations);
    let output_path = Path::new(output);
    if same_path(policy_path, observations_path)
        || same_path(policy_path, output_path)
        || same_path(observations_path, output_path)
    {
        return Err("speculation comparison inputs and output must be distinct paths".to_owned());
    }
    let record = validate(policy_path, observations_path)?;
    write_new(output_path, &encode_canonical_document(&record)?)
}

fn validate(policy_path: &Path, observations_path: &Path) -> BenchResult<Value> {
    require_protocol()?;
    let (policy_root, policy, policy_bytes, policy_file) =
        load_canonical_document_held(policy_path, "speculation comparison policy")?;
    let (observations_root, observations, observations_bytes, observations_file) =
        load_canonical_document_held(observations_path, "speculation comparison observations")?;
    let inputs = Inputs {
        observations_bytes,
        observations_file,
        observations_name: file_name(observations_path, "speculation comparison observations")?,
        observations_root,
        policy_bytes,
        policy_file,
        policy_name: file_name(policy_path, "speculation comparison policy")?,
        policy_root,
    };
    if inputs.policy_file.identity() == inputs.observations_file.identity() {
        return Err("speculation comparison policy and observations must not alias".to_owned());
    }
    validate_policy(&policy)?;
    let policy_sha256 = sha256_identity(&inputs.policy_bytes);
    let samples = validate_observations(&observations, &policy, &policy_sha256)?;
    let record = build_record(&policy, &observations, &inputs, samples)?;
    inputs.revalidate()?;
    Ok(record)
}

fn validate_policy(policy: &Value) -> BenchResult<()> {
    let object = exact_object(policy, POLICY_KEYS, "speculation comparison policy")?;
    expect_string(
        object,
        "authority",
        POLICY_AUTHORITY,
        "speculation comparison policy",
    )?;
    expect_string(
        object,
        "format",
        POLICY_FORMAT,
        "speculation comparison policy",
    )?;
    expect_string(
        object,
        "nonclaim",
        NONCLAIM,
        "speculation comparison policy",
    )?;
    expect_string(
        object,
        "obligation_id",
        "m1.r32",
        "speculation comparison policy",
    )?;
    expect_string(
        object,
        "protocol_sha256",
        PROTOCOL_SHA256,
        "speculation comparison policy",
    )?;
    expect_string(
        object,
        "status",
        "pre-observation",
        "speculation comparison policy",
    )?;
    expect_string(object, "target", TARGET, "speculation comparison policy")?;
    validate_engine_order(field(
        object,
        "engine_order",
        "speculation comparison policy",
    )?)?;
    let plan = field(object, "plan", "speculation comparison policy")?;
    validate_plan(plan)?;
    validate_implementations(
        field(object, "implementations", "speculation comparison policy")?,
        plan,
    )?;
    validate_thresholds(field(
        object,
        "thresholds",
        "speculation comparison policy",
    )?)?;
    validate_cells(field(object, "cells", "speculation comparison policy")?)
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
        .ok_or_else(|| "speculation comparison policy must be an object".to_owned())?;
    let observations = json!({
        "authority": OBSERVATIONS_AUTHORITY,
        "cells": field(policy_object, "cells", "speculation comparison policy")?,
        "engine_order": field(policy_object, "engine_order", "speculation comparison policy")?,
        "format": OBSERVATIONS_FORMAT,
        "implementations": field(policy_object, "implementations", "speculation comparison policy")?,
        "nonclaim": NONCLAIM,
        "obligation_id": "m1.r32",
        "plan": field(policy_object, "plan", "speculation comparison policy")?,
        "policy_sha256": policy_sha256,
        "rows": rows,
        "status": "externally-collected",
        "target": TARGET,
        "thresholds": field(policy_object, "thresholds", "speculation comparison policy")?,
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
    .map_err(|error| format!("cannot securely open R32 observation parent: {error}"))?;
    require_absent_at(&parent, &output_name)
}

fn validate_plan(value: &Value) -> BenchResult<()> {
    let object = exact_object(value, PLAN_KEYS, "speculation comparison plan binding")?;
    for key in PLAN_KEYS {
        sha_value(
            field(object, key, "speculation comparison plan binding")?,
            &format!("speculation comparison plan {key}"),
        )?;
    }
    Ok(())
}

fn validate_implementations(value: &Value, plan: &Value) -> BenchResult<()> {
    let implementations = value
        .as_array()
        .ok_or_else(|| "speculation comparison implementations must be an array".to_owned())?;
    if implementations.len() != ENGINES.len() {
        return Err("speculation comparison implementation roster is incomplete".to_owned());
    }
    let plan = exact_object(plan, PLAN_KEYS, "speculation comparison plan binding")?;
    let ferric_source = field(
        plan,
        "ferric_source_closure_sha256",
        "speculation comparison plan binding",
    )?;
    let mut ferric_version = None;
    for (implementation, expected_id) in implementations.iter().zip(ENGINES) {
        let object = exact_object(
            implementation,
            IMPLEMENTATION_KEYS,
            "speculation comparison implementation",
        )?;
        expect_string(
            object,
            "id",
            expected_id,
            "speculation comparison implementation",
        )?;
        for key in IMPLEMENTATION_KEYS {
            if *key != "id" && *key != "version" {
                sha_value(
                    field(object, key, "speculation comparison implementation")?,
                    &format!("{expected_id} {key}"),
                )?;
            }
        }
        let version = safe_string(
            field(object, "version", "speculation comparison implementation")?,
            &format!("{expected_id} version"),
        )?;
        if version.len() > 160 {
            return Err(format!("{expected_id} version is too long"));
        }
        if field(
            object,
            "source_sha256",
            "speculation comparison implementation",
        )? != ferric_source
        {
            return Err(format!(
                "{expected_id} source identity differs from the Ferric source closure"
            ));
        }
        if ferric_version.is_some_and(|expected| expected != version) {
            return Err("speculative and target-only Ferric versions differ".to_owned());
        }
        ferric_version = Some(version);
    }
    Ok(())
}

fn validate_thresholds(value: &Value) -> BenchResult<()> {
    let object = exact_object(value, THRESHOLD_KEYS, "speculation comparison thresholds")?;
    expect_u64(
        object,
        "eligible_latency_max_ratio_ppm",
        ELIGIBLE_LATENCY_MAX_PPM,
        "speculation comparison thresholds",
    )?;
    expect_u64(
        object,
        "eligible_throughput_min_ratio_ppm",
        ELIGIBLE_THROUGHPUT_MIN_PPM,
        "speculation comparison thresholds",
    )?;
    expect_u64(
        object,
        "low_acceptance_throughput_min_ratio_ppm",
        LOW_ACCEPTANCE_THROUGHPUT_MIN_PPM,
        "speculation comparison thresholds",
    )?;
    expect_u64(
        object,
        "recorded_pairs",
        RECORDED_PAIRS as u64,
        "speculation comparison thresholds",
    )?;
    expect_u64(
        object,
        "warmup_pairs",
        WARMUP_PAIRS as u64,
        "speculation comparison thresholds",
    )
}

fn validate_cells(value: &Value) -> BenchResult<()> {
    let cells = value
        .as_array()
        .ok_or_else(|| "speculation comparison cells must be an array".to_owned())?;
    if cells.len() != CELL_IDS.len() {
        return Err("speculation comparison cell roster is incomplete".to_owned());
    }
    let mut pair_ids = BTreeSet::new();
    let mut pairing_identities = BTreeSet::new();
    for (cell, expected_id) in cells.iter().zip(CELL_IDS) {
        validate_cell(cell, expected_id, &mut pair_ids, &mut pairing_identities)?;
    }
    Ok(())
}

fn validate_cell(
    value: &Value,
    expected_id: &str,
    pair_ids: &mut BTreeSet<String>,
    pairing_identities: &mut BTreeSet<String>,
) -> BenchResult<()> {
    let object = exact_object(value, CELL_KEYS, "speculation comparison cell")?;
    expect_string(object, "id", expected_id, "speculation comparison cell")?;
    if field(object, "eligible", "speculation comparison cell")?.as_bool() != Some(true) {
        return Err(format!(
            "speculation comparison cell is not eligible: {expected_id}"
        ));
    }
    let acceptance = safe_string(
        field(object, "acceptance", "speculation comparison cell")?,
        "speculation comparison acceptance",
    )?;
    match expected_id {
        "eligible-speculation" if !matches!(acceptance, "mixed" | "high") => {
            return Err("eligible speculation cell must use mixed or high acceptance".to_owned());
        }
        "low-acceptance" if acceptance != "low" => {
            return Err("low-acceptance cell must use the low acceptance class".to_owned());
        }
        _ => {}
    }
    let case_kind = safe_string(
        field(object, "case_kind", "speculation comparison cell")?,
        "speculation comparison case kind",
    )?;
    if !CASE_KINDS.contains(&case_kind) {
        return Err(format!(
            "unknown speculation comparison case kind: {case_kind}"
        ));
    }
    let plan = field(
        object,
        "deterministic_admitted_plan_sha256",
        "speculation comparison cell",
    )?;
    if expected_id == "low-acceptance" {
        sha_value(plan, "low-acceptance deterministic admitted plan")?;
    } else if !plan.is_null() {
        return Err(
            "eligible speculation cell unexpectedly names a deterministic fallback plan".to_owned(),
        );
    }
    let slo = positive_u64(
        field(object, "p99_slo_ns", "speculation comparison cell")?,
        "speculation comparison p99 SLO",
    )?;
    if slo == u64::MAX {
        return Err("speculation comparison p99 SLO is outside the admitted bound".to_owned());
    }
    validate_holdout(field(
        object,
        "holdout_member",
        "speculation comparison cell",
    )?)?;
    validate_workload(
        field(object, "workload", "speculation comparison cell")?,
        acceptance,
        case_kind,
    )?;
    validate_pair_roster(
        field(object, "pair_roster", "speculation comparison cell")?,
        expected_id,
        pair_ids,
        pairing_identities,
    )
}

fn validate_holdout(value: &Value) -> BenchResult<()> {
    let object = exact_object(value, HOLDOUT_KEYS, "speculation comparison holdout member")?;
    let id = safe_string(
        field(object, "id", "speculation comparison holdout member")?,
        "speculation comparison holdout member ID",
    )?;
    if id.len() > 160 {
        return Err("speculation comparison holdout member ID is too long".to_owned());
    }
    sha_value(
        field(object, "sha256", "speculation comparison holdout member")?,
        "speculation comparison holdout member",
    )
}

fn validate_workload(value: &Value, acceptance: &str, case_kind: &str) -> BenchResult<()> {
    let object = exact_object(value, WORKLOAD_KEYS, "speculation comparison workload")?;
    expect_string(
        object,
        "arrival",
        "closed-loop",
        "speculation comparison workload",
    )?;
    expect_u64(
        object,
        "prefix_sharing_percent",
        0,
        "speculation comparison workload",
    )?;
    for key in [
        "arrival_trace_sha256",
        "output_limits_sha256",
        "prompt_order_sha256",
        "sampling_seed_sha256",
    ] {
        sha_value(
            field(object, key, "speculation comparison workload")?,
            &format!("speculation comparison workload {key}"),
        )?;
    }
    let batch = positive_u64(
        field(object, "batch", "speculation comparison workload")?,
        "batch",
    )?;
    if ![1, 4, 8, 16, 32].contains(&batch) {
        return Err("speculation comparison batch is outside the M1 matrix".to_owned());
    }
    let prefill = positive_u64(
        field(object, "prefill_length", "speculation comparison workload")?,
        "prefill length",
    )?;
    if ![128, 512, 2_048, 8_192].contains(&prefill) {
        return Err("speculation comparison prefill length is outside the M1 matrix".to_owned());
    }
    let decode = positive_u64(
        field(
            object,
            "decode_kv_length",
            "speculation comparison workload",
        )?,
        "decode KV length",
    )?;
    if ![128, 1_024, 4_096, 8_192].contains(&decode) {
        return Err("speculation comparison decode KV length is outside the M1 matrix".to_owned());
    }
    let draft = positive_u64(
        field(object, "draft_length", "speculation comparison workload")?,
        "draft length",
    )?;
    if ![1, 2, 4, 8, 16].contains(&draft) {
        return Err("speculation comparison draft length is outside the M1 matrix".to_owned());
    }
    let (expected_batch, expected_draft) = match case_kind {
        "speculative-s1-k16-c8192" => (1, 16),
        "speculative-s1-k4-c8192" => (1, 4),
        "speculative-s1-k8-c8192" => (1, 8),
        "speculative-s8-k4-c8192" => (8, 4),
        _ => return Err("unknown speculation comparison case geometry".to_owned()),
    };
    if batch != expected_batch || draft != expected_draft {
        return Err("speculation comparison workload drifted from its case geometry".to_owned());
    }
    let isl_osl = safe_string(
        field(object, "isl_osl", "speculation comparison workload")?,
        "speculation comparison ISL/OSL",
    )?;
    if !["128x128", "1024x256", "4096x256", "512x2048"].contains(&isl_osl) {
        return Err("speculation comparison ISL/OSL is outside the M1 matrix".to_owned());
    }
    let identity = json!({
        "acceptance": acceptance,
        "arrival": field(object, "arrival", "speculation comparison workload")?,
        "arrival_trace_sha256": field(object, "arrival_trace_sha256", "speculation comparison workload")?,
        "batch": batch,
        "decode_kv_length": decode,
        "draft_length": draft,
        "isl_osl": isl_osl,
        "output_limits_sha256": field(object, "output_limits_sha256", "speculation comparison workload")?,
        "prefix_sharing_percent": 0,
        "prefill_length": prefill,
        "prompt_order_sha256": field(object, "prompt_order_sha256", "speculation comparison workload")?,
        "sampling_seed_sha256": field(object, "sampling_seed_sha256", "speculation comparison workload")?,
    });
    let expected = sha256_identity(&encode_canonical_document(&identity)?);
    expect_string(
        object,
        "workload_sha256",
        &expected,
        "speculation comparison workload",
    )
}

fn validate_pair_roster(
    value: &Value,
    cell_id: &str,
    pair_ids: &mut BTreeSet<String>,
    pairing_identities: &mut BTreeSet<String>,
) -> BenchResult<()> {
    let pairs = value
        .as_array()
        .ok_or_else(|| "speculation comparison pair roster must be an array".to_owned())?;
    if pairs.len() != WARMUP_PAIRS + RECORDED_PAIRS {
        return Err(format!(
            "speculation comparison pair roster is incomplete: {cell_id}"
        ));
    }
    let mut ordinal = 0;
    for (phase, count) in [("warmup", WARMUP_PAIRS), ("recorded", RECORDED_PAIRS)] {
        for index in 0..count {
            let object = exact_object(&pairs[ordinal], PAIR_KEYS, "speculation comparison pair")?;
            let expected_id = format!("{cell_id}.{phase}-{index:02}");
            expect_string(object, "id", &expected_id, "speculation comparison pair")?;
            let pairing = field(object, "pairing_sha256", "speculation comparison pair")?;
            sha_value(pairing, "speculation comparison pairing identity")?;
            let pairing = pairing
                .as_str()
                .ok_or_else(|| {
                    "speculation comparison pairing identity must be a string".to_owned()
                })?
                .to_owned();
            if !pair_ids.insert(expected_id) || !pairing_identities.insert(pairing) {
                return Err("speculation comparison pair roster contains a duplicate".to_owned());
            }
            ordinal += 1;
        }
    }
    Ok(())
}

fn validate_observations(
    observations: &Value,
    policy: &Value,
    policy_sha256: &str,
) -> BenchResult<Vec<CellSamples>> {
    let object = exact_object(
        observations,
        OBSERVATION_KEYS,
        "speculation comparison observations",
    )?;
    expect_string(
        object,
        "authority",
        OBSERVATIONS_AUTHORITY,
        "speculation comparison observations",
    )?;
    expect_string(
        object,
        "format",
        OBSERVATIONS_FORMAT,
        "speculation comparison observations",
    )?;
    expect_string(
        object,
        "nonclaim",
        NONCLAIM,
        "speculation comparison observations",
    )?;
    expect_string(
        object,
        "obligation_id",
        "m1.r32",
        "speculation comparison observations",
    )?;
    expect_string(
        object,
        "policy_sha256",
        policy_sha256,
        "speculation comparison observations",
    )?;
    expect_string(
        object,
        "status",
        "externally-collected",
        "speculation comparison observations",
    )?;
    expect_string(
        object,
        "target",
        TARGET,
        "speculation comparison observations",
    )?;

    let policy_object = policy
        .as_object()
        .ok_or_else(|| "speculation comparison policy must be an object".to_owned())?;
    for key in [
        "cells",
        "engine_order",
        "implementations",
        "plan",
        "thresholds",
    ] {
        if field(object, key, "speculation comparison observations")?
            != field(policy_object, key, "speculation comparison policy")?
        {
            return Err(format!(
                "speculation comparison observation {key} binding drifted"
            ));
        }
    }

    let rows = field(object, "rows", "speculation comparison observations")?
        .as_array()
        .ok_or_else(|| "speculation comparison rows must be an array".to_owned())?;
    let pairs_per_cell = WARMUP_PAIRS + RECORDED_PAIRS;
    if rows.len() != CELL_IDS.len() * pairs_per_cell {
        return Err("speculation comparison row roster is incomplete".to_owned());
    }
    let mut samples = CELL_IDS
        .iter()
        .map(|_| CellSamples {
            accepted_tokens: 0,
            engines: ENGINES
                .iter()
                .map(|_| EngineSamples {
                    latency: Vec::with_capacity(RECORDED_PAIRS),
                    throughput: Vec::with_capacity(RECORDED_PAIRS),
                })
                .collect(),
            target_invocations: vec![0; ENGINES.len()],
        })
        .collect::<Vec<_>>();
    let mut unique = BTreeSet::new();
    let mut ordinal = 0_usize;
    let policy_cells = field(policy_object, "cells", "speculation comparison policy")?
        .as_array()
        .ok_or_else(|| "speculation comparison policy cells must be an array".to_owned())?;
    for (cell_index, cell_id) in CELL_IDS.iter().enumerate() {
        let pair_roster = policy_cells[cell_index]["pair_roster"]
            .as_array()
            .ok_or_else(|| {
                "speculation comparison policy pair roster must be an array".to_owned()
            })?;
        let mut roster_index = 0;
        for (phase, count) in [("warmup", WARMUP_PAIRS), ("recorded", RECORDED_PAIRS)] {
            for pair_index in 0..count {
                validate_row(
                    &rows[ordinal],
                    ExpectedRow {
                        cell_id,
                        ordinal,
                        pair_index,
                        phase,
                        roster_pair: &pair_roster[roster_index],
                    },
                    &mut unique,
                    &mut samples[cell_index],
                )?;
                ordinal += 1;
                roster_index += 1;
            }
        }
    }
    Ok(samples)
}

fn validate_row(
    value: &Value,
    expected: ExpectedRow<'_>,
    unique: &mut BTreeSet<String>,
    samples: &mut CellSamples,
) -> BenchResult<()> {
    let object = exact_object(value, ROW_KEYS, "speculation comparison row")?;
    let expected_id = format!(
        "{}.{}-{:02}",
        expected.cell_id, expected.phase, expected.pair_index
    );
    expect_string(
        object,
        "cell_id",
        expected.cell_id,
        "speculation comparison row",
    )?;
    expect_string(object, "id", &expected_id, "speculation comparison row")?;
    if !unique.insert(expected_id.clone()) {
        return Err("speculation comparison row IDs are duplicated".to_owned());
    }
    expect_string(
        object,
        "phase",
        expected.phase,
        "speculation comparison row",
    )?;
    expect_string(object, "status", "passed", "speculation comparison row")?;
    expect_u64(
        object,
        "ordinal",
        expected.ordinal as u64,
        "speculation comparison row",
    )?;
    expect_u64(
        object,
        "pair_index",
        expected.pair_index as u64,
        "speculation comparison row",
    )?;
    let roster = exact_object(
        expected.roster_pair,
        PAIR_KEYS,
        "speculation comparison roster pair",
    )?;
    if field(object, "pairing_sha256", "speculation comparison row")?
        != field(
            roster,
            "pairing_sha256",
            "speculation comparison roster pair",
        )?
    {
        return Err(format!(
            "speculation comparison pairing identity drifted: {expected_id}"
        ));
    }
    let faults = field(object, "faults", "speculation comparison row")?
        .as_array()
        .ok_or_else(|| "speculation comparison faults must be an array".to_owned())?;
    if !faults.is_empty() {
        return Err(format!(
            "speculation comparison row retained a fault: {expected_id}"
        ));
    }
    let expected_order = (0..ENGINES.len())
        .map(|offset| ENGINES[(expected.ordinal + offset) % ENGINES.len()])
        .collect::<Vec<_>>();
    if field(object, "engine_order", "speculation comparison row")? != &json!(expected_order) {
        return Err(format!(
            "speculation comparison engine order drifted: {expected_id}"
        ));
    }
    let values = exact_object(
        field(object, "values", "speculation comparison row")?,
        ENGINES,
        "speculation comparison row values",
    )?;
    let mut paired_successful_requests = None;
    let mut paired_total_tokens = None;
    for (index, engine) in ENGINES.iter().enumerate() {
        let metric_keys = if *engine == "speculative" {
            SPECULATIVE_VALUE_KEYS
        } else {
            TARGET_ONLY_VALUE_KEYS
        };
        let metrics = exact_object(
            field(values, engine, "speculation comparison row values")?,
            metric_keys,
            &format!("speculation comparison {engine} pair counters"),
        )?;
        let duration = positive_u64(
            field(
                metrics,
                "duration_ns",
                "speculation comparison pair counters",
            )?,
            "speculation comparison duration",
        )?;
        let failed = unsigned_u64(
            field(
                metrics,
                "failed_requests",
                "speculation comparison pair counters",
            )?,
            "speculation comparison failed requests",
        )?;
        let latency = positive_u64(
            field(
                metrics,
                "p99_latency_ns",
                "speculation comparison pair counters",
            )?,
            "speculation comparison p99 latency",
        )?;
        let successful = positive_u64(
            field(
                metrics,
                "successful_requests",
                "speculation comparison pair counters",
            )?,
            "speculation comparison successful requests",
        )?;
        let tokens = positive_u64(
            field(
                metrics,
                "total_tokens",
                "speculation comparison pair counters",
            )?,
            "speculation comparison total tokens",
        )?;
        if paired_successful_requests.is_some_and(|expected| expected != successful)
            || paired_total_tokens.is_some_and(|expected| expected != tokens)
        {
            return Err(format!(
                "speculation comparison pair does not contain equal work: {expected_id}"
            ));
        }
        paired_successful_requests = Some(successful);
        paired_total_tokens = Some(tokens);
        if failed != 0 {
            return Err(format!(
                "speculation comparison row has a failed request: {expected_id}"
            ));
        }
        let throughput = rate(tokens, duration)?;
        let invocations = positive_u64(
            field(
                metrics,
                "target_invocations",
                "speculation comparison pair counters",
            )?,
            "speculation comparison target invocations",
        )?;
        let accepted = if *engine == "speculative" {
            let accepted = unsigned_u64(
                field(
                    metrics,
                    "accepted_tokens",
                    "speculation comparison pair counters",
                )?,
                "speculation comparison accepted tokens",
            )?;
            if accepted > tokens {
                return Err(format!(
                    "speculation comparison accepted tokens exceed paired output: {expected_id}"
                ));
            }
            Some(accepted)
        } else {
            None
        };
        if expected.phase == "recorded" {
            samples.target_invocations[index] = samples.target_invocations[index]
                .checked_add(u128::from(invocations))
                .ok_or_else(|| {
                    "speculation comparison target-invocation total overflowed".to_owned()
                })?;
            if let Some(accepted) = accepted {
                samples.accepted_tokens = samples
                    .accepted_tokens
                    .checked_add(u128::from(accepted))
                    .ok_or_else(|| {
                        "speculation comparison accepted-token total overflowed".to_owned()
                    })?;
            }
            samples.engines[index].latency.push(latency);
            samples.engines[index].throughput.push(throughput);
        }
    }
    Ok(())
}

fn build_record(
    policy: &Value,
    observations: &Value,
    inputs: &Inputs,
    samples: Vec<CellSamples>,
) -> BenchResult<Value> {
    let policy_object = policy
        .as_object()
        .ok_or_else(|| "speculation comparison policy must be an object".to_owned())?;
    let cells = field(policy_object, "cells", "speculation comparison policy")?
        .as_array()
        .ok_or_else(|| "speculation comparison policy cells must be an array".to_owned())?;
    let mut cell_summaries = Vec::with_capacity(CELL_IDS.len());
    for ((cell_id, cell), mut sample) in CELL_IDS.iter().zip(cells).zip(samples) {
        let slo = positive_u64(&cell["p99_slo_ns"], "speculation comparison p99 SLO")?;
        let mut engine_summaries = Vec::with_capacity(ENGINES.len());
        let mut throughputs = Vec::with_capacity(ENGINES.len());
        let mut latencies = Vec::with_capacity(ENGINES.len());
        for (index, (engine, engine_sample)) in ENGINES.iter().zip(&mut sample.engines).enumerate()
        {
            let throughput = median(&mut engine_sample.throughput)?;
            let latency = median(&mut engine_sample.latency)?;
            if latency.numerator > u128::from(slo) * latency.denominator {
                return Err(format!(
                    "speculation comparison median p99 latency exceeds the declared SLO: {cell_id}/{engine}"
                ));
            }
            engine_summaries.push(json!({
                "id": engine,
                "median_p99_latency_ns": latency.as_json(),
                "median_total_tokens_per_second": throughput.as_json(),
                "recorded_pairs": RECORDED_PAIRS,
                "target_invocations": sample.target_invocations[index].to_string(),
            }));
            throughputs.push(throughput);
            latencies.push(latency);
        }
        let throughput_ratio = ratio_ppm(&throughputs[0], &throughputs[1])?;
        let latency_ratio = ratio_ppm(&latencies[0], &latencies[1])?;
        let mean_accepted = Rational::new(sample.accepted_tokens, sample.target_invocations[0]);
        let gates = if *cell_id == "eligible-speculation" {
            if sample.accepted_tokens == 0 {
                return Err("eligible speculation cell accepted no speculative tokens".to_owned());
            }
            if !ratio_ge_scaled(
                &throughputs[0],
                &throughputs[1],
                ELIGIBLE_THROUGHPUT_MIN_PPM,
            )? {
                return Err(
                    "eligible speculation throughput improvement is below ten percent".to_owned(),
                );
            }
            if !ratio_le_scaled(&latencies[0], &latencies[1], ELIGIBLE_LATENCY_MAX_PPM)? {
                return Err(
                    "eligible speculation p99 latency regression exceeds five percent".to_owned(),
                );
            }
            json!({
                "latency_max_ratio_ppm": ELIGIBLE_LATENCY_MAX_PPM,
                "latency_passed": true,
                "throughput_min_ratio_ppm": ELIGIBLE_THROUGHPUT_MIN_PPM,
                "throughput_passed": true,
            })
        } else {
            if !ratio_ge_scaled(
                &throughputs[0],
                &throughputs[1],
                LOW_ACCEPTANCE_THROUGHPUT_MIN_PPM,
            )? {
                return Err(
                    "low-acceptance speculation throughput regression exceeds five percent"
                        .to_owned(),
                );
            }
            json!({
                "throughput_min_ratio_ppm": LOW_ACCEPTANCE_THROUGHPUT_MIN_PPM,
                "throughput_passed": true,
            })
        };
        cell_summaries.push(json!({
            "accepted_tokens": sample.accepted_tokens.to_string(),
            "engine_summaries": engine_summaries,
            "gates": gates,
            "id": cell_id,
            "mean_accepted_tokens_per_speculative_target_invocation": mean_accepted.as_json(),
            "speculative_to_target_only_latency_ratio_ppm": latency_ratio,
            "speculative_to_target_only_throughput_ratio_ppm": throughput_ratio,
        }));
    }
    let observation_object = observations
        .as_object()
        .ok_or_else(|| "speculation comparison observations must be an object".to_owned())?;
    Ok(json!({
        "authority": RECORD_AUTHORITY,
        "bindings": {
            "cells": field(policy_object, "cells", "speculation comparison policy")?,
            "engine_order": field(policy_object, "engine_order", "speculation comparison policy")?,
            "implementations": field(policy_object, "implementations", "speculation comparison policy")?,
            "plan": field(policy_object, "plan", "speculation comparison policy")?,
            "protocol_sha256": PROTOCOL_SHA256,
            "thresholds": field(policy_object, "thresholds", "speculation comparison policy")?,
        },
        "cell_summaries": cell_summaries,
        "format": RECORD_FORMAT,
        "nonclaim": NONCLAIM,
        "obligation_id": "m1.r32",
        "observations_sha256": sha256_identity(&inputs.observations_bytes),
        "policy_sha256": sha256_identity(&inputs.policy_bytes),
        "raw_rows": field(observation_object, "rows", "speculation comparison observations")?,
        "recorded_pairs_per_cell": RECORDED_PAIRS,
        "status": STATUS,
        "target": TARGET,
        "warmup_pairs_per_cell": WARMUP_PAIRS,
    }))
}

fn median(values: &mut [u64]) -> BenchResult<Rational> {
    if values.is_empty() {
        return Err("speculation comparison median population is empty".to_owned());
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
        .ok_or_else(|| "speculation comparison throughput numerator overflowed".to_owned())?;
    u64::try_from(scaled / u128::from(duration_ns))
        .map_err(|_| "speculation comparison throughput does not fit u64".to_owned())
        .and_then(|value| {
            if value == 0 {
                Err("speculation comparison throughput rounded to zero".to_owned())
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
        .ok_or_else(|| "speculation comparison ratio numerator overflowed".to_owned())?;
    let right = numerator
        .denominator
        .checked_mul(denominator.numerator)
        .ok_or_else(|| "speculation comparison ratio denominator overflowed".to_owned())?;
    u64::try_from(left / right)
        .map_err(|_| "speculation comparison ratio does not fit u64".to_owned())
}

fn ratio_ge_scaled(left: &Rational, right: &Rational, minimum_ppm: u64) -> BenchResult<bool> {
    let left_scaled = left
        .numerator
        .checked_mul(right.denominator)
        .and_then(|value| value.checked_mul(PPM_SCALE))
        .ok_or_else(|| "speculation comparison lower-gate numerator overflowed".to_owned())?;
    let right_scaled = right
        .numerator
        .checked_mul(left.denominator)
        .and_then(|value| value.checked_mul(u128::from(minimum_ppm)))
        .ok_or_else(|| "speculation comparison lower-gate denominator overflowed".to_owned())?;
    Ok(left_scaled >= right_scaled)
}

fn ratio_le_scaled(left: &Rational, right: &Rational, maximum_ppm: u64) -> BenchResult<bool> {
    let left_scaled = left
        .numerator
        .checked_mul(right.denominator)
        .and_then(|value| value.checked_mul(PPM_SCALE))
        .ok_or_else(|| "speculation comparison upper-gate numerator overflowed".to_owned())?;
    let right_scaled = right
        .numerator
        .checked_mul(left.denominator)
        .and_then(|value| value.checked_mul(u128::from(maximum_ppm)))
        .ok_or_else(|| "speculation comparison upper-gate denominator overflowed".to_owned())?;
    Ok(left_scaled <= right_scaled)
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
        return Err("speculation comparison engine roster or order drifted".to_owned());
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
        .ok_or_else(|| "speculation comparison manifest directory is absent".to_owned())?;
    let protocol = PathBuf::from(manifest).join("m1_r32_speculation_comparison_protocol.json");
    let (_, value, bytes, file) =
        load_canonical_document_held(&protocol, "speculation comparison protocol")?;
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
        "speculation comparison protocol",
    )?;
    expect_string(
        object,
        "authority",
        "ferric-m1-r32-speculation-comparison-protocol-only",
        "speculation comparison protocol",
    )?;
    expect_string(
        object,
        "format",
        "FERRIC-M1-R32-SPECULATION-COMPARISON-PROTOCOL-V1",
        "speculation comparison protocol",
    )?;
    expect_string(
        object,
        "nonclaim",
        NONCLAIM,
        "speculation comparison protocol",
    )?;
    expect_string(
        object,
        "obligation_id",
        "m1.r32",
        "speculation comparison protocol",
    )?;
    expect_string(object, "status", STATUS, "speculation comparison protocol")?;
    expect_string(object, "target", TARGET, "speculation comparison protocol")?;
    if sha256_identity(&bytes) != PROTOCOL_SHA256 {
        return Err("speculation comparison protocol SHA-256 drifted".to_owned());
    }
    file.validate_snapshot("speculation comparison protocol")
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
        .map_err(|error| format!("cannot securely open speculation record parent: {error}"))?;
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
                        "cannot create staged speculation comparison record: {error}"
                    ));
                }
            };
            let mut file = File::from(descriptor);
            let created = match fstat(&file) {
                Ok(created) if valid_record_file(&created, 0) => created,
                Ok(created) => {
                    cleanup_created_name(&parent, &staging_name, &created);
                    return Err(
                        "created speculation comparison staging entry is invalid".to_owned()
                    );
                }
                Err(error) => {
                    return Err(format!(
                        "cannot inspect staged speculation comparison record: {error}"
                    ));
                }
            };
            if let Err(error) = file.write_all(bytes) {
                drop(file);
                cleanup_created_name(&parent, &staging_name, &created);
                return Err(format!(
                    "cannot write staged speculation comparison record: {error}"
                ));
            }
            if let Err(error) = file.sync_all() {
                drop(file);
                cleanup_created_name(&parent, &staging_name, &created);
                return Err(format!(
                    "cannot synchronize staged speculation comparison record: {error}"
                ));
            }
            let settled = match fstat(&file) {
                Ok(settled) if valid_record_file(&settled, bytes.len()) => settled,
                Ok(_) => {
                    drop(file);
                    cleanup_created_name(&parent, &staging_name, &created);
                    return Err(
                        "settled speculation comparison staging entry is invalid".to_owned()
                    );
                }
                Err(error) => {
                    drop(file);
                    cleanup_created_name(&parent, &staging_name, &created);
                    return Err(format!(
                        "cannot reinspect staged speculation comparison record: {error}"
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
        Err("speculation comparison staging namespace was exhausted".to_owned())
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
            "staged speculation comparison record",
        )?;
        self.verify_bytes(staged, false, "staged speculation comparison record")?;
        renameat_with(
            &self.parent,
            self.staging_name.as_os_str(),
            &self.parent,
            self.output_name.as_os_str(),
            RenameFlags::NOREPLACE,
        )
        .map_err(|error| {
            if error == rustix::io::Errno::EXIST {
                "speculation comparison output appeared before no-replace publication".to_owned()
            } else {
                format!("cannot publish speculation comparison record without replacement: {error}")
            }
        })?;
        self.armed = false;
        let published = self.rebind(
            &self.output_name,
            true,
            "published speculation comparison record",
        )?;
        self.verify_bytes(published, true, "published speculation comparison record")?;
        after_first_published_verification()?;
        fsync(&self.parent).map_err(|error| {
            format!("cannot sync speculation comparison output parent: {error}")
        })?;
        let final_name = self.rebind(
            &self.output_name,
            true,
            "final published speculation comparison record",
        )?;
        self.verify_bytes(
            final_name,
            true,
            "final published speculation comparison record",
        )?;
        let final_rebound = self.rebind(
            &self.output_name,
            true,
            "final rebound speculation comparison record",
        )?;
        self.verify_bytes(
            final_rebound,
            true,
            "final rebound speculation comparison record",
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
        return Err("speculation comparison output parent path is not admitted".to_owned());
    }
    Ok(parent.to_path_buf())
}

fn safe_output_name(path: &Path) -> BenchResult<OsString> {
    let name = path
        .file_name()
        .ok_or_else(|| "speculation comparison output has no final component".to_owned())?;
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
        return Err("speculation comparison output name is invalid".to_owned());
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
        Ok(_) => Err("speculation comparison output already exists".to_owned()),
        Err(error) if error == rustix::io::Errno::NOENT => Ok(()),
        Err(error) => Err(format!(
            "cannot safely inspect speculation comparison output: {error}"
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
                "ferric-m1-r32-speculation-record-test.{}.{nonce}",
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
            "artifact_sha256": digest(&format!("{id}-artifact")),
            "config_sha256": digest(&format!("{id}-config")),
            "id": id,
            "implementation_sha256": digest(&format!("{id}-implementation")),
            "protocol_sha256": digest(&format!("{id}-protocol")),
            "source_sha256": digest("ferric_source_closure_sha256"),
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
        let mut object = identity.as_object().unwrap().clone();
        object.remove("acceptance");
        object.insert(
            "workload_sha256".to_owned(),
            Value::String(digest_document(&identity)),
        );
        Value::Object(object)
    }

    fn digest_document(value: &Value) -> String {
        sha256_identity(&encode_canonical_document(value).unwrap())
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
            "p99_slo_ns": 1_000,
            "pair_roster": pair_roster(id),
            "workload": workload(acceptance),
        })
    }

    fn policy() -> Value {
        json!({
            "authority": POLICY_AUTHORITY,
            "cells": CELL_IDS.iter().map(|id| cell(id)).collect::<Vec<_>>(),
            "engine_order": ENGINES,
            "format": POLICY_FORMAT,
            "implementations": ENGINES.iter().map(|id| implementation(id)).collect::<Vec<_>>(),
            "nonclaim": NONCLAIM,
            "obligation_id": "m1.r32",
            "plan": PLAN_KEYS.iter().map(|key| ((*key).to_owned(), Value::String(digest(key)))).collect::<Map<_, _>>(),
            "protocol_sha256": PROTOCOL_SHA256,
            "status": "pre-observation",
            "target": TARGET,
            "thresholds": {
                "eligible_latency_max_ratio_ppm": ELIGIBLE_LATENCY_MAX_PPM,
                "eligible_throughput_min_ratio_ppm": ELIGIBLE_THROUGHPUT_MIN_PPM,
                "low_acceptance_throughput_min_ratio_ppm": LOW_ACCEPTANCE_THROUGHPUT_MIN_PPM,
                "recorded_pairs": RECORDED_PAIRS,
                "warmup_pairs": WARMUP_PAIRS,
            },
        })
    }

    fn observations(policy: &Value) -> Value {
        let mut rows = Vec::new();
        let mut ordinal = 0_usize;
        for (cell_index, cell_id) in CELL_IDS.iter().enumerate() {
            let mut roster_index = 0;
            for (phase, count) in [("warmup", WARMUP_PAIRS), ("recorded", RECORDED_PAIRS)] {
                for pair_index in 0..count {
                    let order = (0..ENGINES.len())
                        .map(|offset| ENGINES[(ordinal + offset) % ENGINES.len()])
                        .collect::<Vec<_>>();
                    let (speculative_duration, speculative_latency) =
                        if *cell_id == "eligible-speculation" {
                            (900, 105)
                        } else {
                            (1_050, 100)
                        };
                    rows.push(json!({
                        "cell_id": cell_id,
                        "engine_order": order,
                        "faults": [],
                        "id": format!("{cell_id}.{phase}-{pair_index:02}"),
                        "ordinal": ordinal,
                        "pair_index": pair_index,
                        "pairing_sha256": policy["cells"][cell_index]["pair_roster"][roster_index]["pairing_sha256"],
                        "phase": phase,
                        "status": "passed",
                        "values": {
                            "speculative": {"accepted_tokens": if *cell_id == "eligible-speculation" { 80 } else { 5 }, "duration_ns": speculative_duration, "failed_requests": 0, "p99_latency_ns": speculative_latency, "successful_requests": 4, "target_invocations": 40, "total_tokens": 100},
                            "target-only": {"duration_ns": 1000, "failed_requests": 0, "p99_latency_ns": 100, "successful_requests": 4, "target_invocations": 100, "total_tokens": 100},
                        },
                    }));
                    ordinal += 1;
                    roster_index += 1;
                }
            }
        }
        let bytes = encode_canonical_document(policy).unwrap();
        json!({
            "authority": OBSERVATIONS_AUTHORITY,
            "cells": policy["cells"],
            "engine_order": policy["engine_order"],
            "format": OBSERVATIONS_FORMAT,
            "implementations": policy["implementations"],
            "nonclaim": NONCLAIM,
            "obligation_id": "m1.r32",
            "plan": policy["plan"],
            "policy_sha256": sha256_identity(&bytes),
            "rows": rows,
            "status": "externally-collected",
            "target": TARGET,
            "thresholds": policy["thresholds"],
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
        assert_eq!(record["cell_summaries"].as_array().unwrap().len(), 2);
        assert_eq!(
            record["cell_summaries"][0]["speculative_to_target_only_throughput_ratio_ppm"],
            1_111_111
        );
        assert_eq!(
            record["cell_summaries"][0]["speculative_to_target_only_latency_ratio_ppm"],
            1_050_000
        );
        assert_eq!(
            record["cell_summaries"][1]["speculative_to_target_only_throughput_ratio_ppm"],
            952_380
        );
        assert_eq!(record["raw_rows"].as_array().unwrap().len(), 80);
        assert_eq!(record["recorded_pairs_per_cell"], 30);
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
        failed["rows"][0]["values"]["speculative"]["failed_requests"] = json!(1);
        assert!(
            validate_observations(&failed, &policy, failed["policy_sha256"].as_str().unwrap())
                .is_err()
        );

        let mut submitted_summary = observations(&policy);
        submitted_summary["rows"][0]["values"]["speculative"]["throughput"] = json!(10_000);
        assert!(validate_observations(
            &submitted_summary,
            &policy,
            submitted_summary["policy_sha256"].as_str().unwrap()
        )
        .is_err());
    }

    #[test]
    fn unpaired_engine_count_identity_and_threshold_substitution_fail_closed() {
        let original = policy();
        let observations = observations(&original);

        let mut unpaired = observations.clone();
        unpaired["rows"][0]["pairing_sha256"] = json!(digest("substituted-pair"));
        assert!(validate_observations(
            &unpaired,
            &original,
            unpaired["policy_sha256"].as_str().unwrap()
        )
        .is_err());

        let mut missing_engine = observations.clone();
        missing_engine["rows"][0]["values"]
            .as_object_mut()
            .unwrap()
            .remove("target-only");
        assert!(validate_observations(
            &missing_engine,
            &original,
            missing_engine["policy_sha256"].as_str().unwrap()
        )
        .is_err());

        for key in ["successful_requests", "total_tokens"] {
            let mut unequal_work = observations.clone();
            unequal_work["rows"][0]["values"]["target-only"][key] = json!(101);
            assert!(validate_observations(
                &unequal_work,
                &original,
                unequal_work["policy_sha256"].as_str().unwrap()
            )
            .is_err());
        }

        let mut impossible_acceptance = observations.clone();
        impossible_acceptance["rows"][0]["values"]["speculative"]["accepted_tokens"] = json!(101);
        assert!(validate_observations(
            &impossible_acceptance,
            &original,
            impossible_acceptance["policy_sha256"].as_str().unwrap()
        )
        .is_err());

        let mut identity = original.clone();
        identity["implementations"][1]["source_sha256"] = json!(digest("other-target-source"));
        assert!(validate_observations(
            &observations,
            &identity,
            observations["policy_sha256"].as_str().unwrap()
        )
        .is_err());

        let mut weaker = original.clone();
        weaker["thresholds"]["eligible_throughput_min_ratio_ppm"] = json!(1_000_000);
        assert!(validate_policy(&weaker).is_err());

        let mut unrelated_source = original.clone();
        unrelated_source["implementations"][0]["source_sha256"] = json!(digest("other-source"));
        assert!(validate_policy(&unrelated_source).is_err());

        let mut unrelated_version = original.clone();
        unrelated_version["implementations"][1]["version"] = json!("other-ferric-version");
        assert!(validate_policy(&unrelated_version).is_err());

        let mut geometry = original.clone();
        geometry["cells"][0]["case_kind"] = json!("speculative-s1-k8-c8192");
        assert!(validate_policy(&geometry).is_err());

        let mut replay = observations.clone();
        replay["policy_sha256"] = json!(digest("different-policy"));
        let policy_bytes = encode_canonical_document(&original).unwrap();
        assert!(
            validate_observations(&replay, &original, &sha256_identity(&policy_bytes)).is_err()
        );
    }

    #[test]
    fn arithmetic_gates_reject_eligible_speed_latency_and_low_acceptance_regressions() {
        for (name, mutate) in [
            ("eligible-throughput", 0_u8),
            ("eligible-latency", 1_u8),
            ("low-throughput", 2_u8),
            ("eligible-zero-acceptance", 3_u8),
        ] {
            let temporary = Temporary::new();
            let policy = policy();
            let mut observations = observations(&policy);
            for row in observations["rows"].as_array_mut().unwrap() {
                match mutate {
                    0 if row["cell_id"] == "eligible-speculation" => {
                        row["values"]["speculative"]["duration_ns"] = json!(1_000);
                    }
                    1 if row["cell_id"] == "eligible-speculation" => {
                        row["values"]["speculative"]["p99_latency_ns"] = json!(106);
                    }
                    2 if row["cell_id"] == "low-acceptance" => {
                        row["values"]["speculative"]["duration_ns"] = json!(1_100);
                    }
                    3 if row["cell_id"] == "eligible-speculation" => {
                        row["values"]["speculative"]["accepted_tokens"] = json!(0);
                    }
                    _ => {}
                }
            }
            let (policy_path, observations_path) =
                write_fixture(&temporary.0, &policy, &observations);
            assert!(
                validate(&policy_path, &observations_path).is_err(),
                "{name}"
            );
        }
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
