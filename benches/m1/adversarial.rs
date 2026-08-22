#![forbid(unsafe_code)]

//! Adversarial run-plan, execution, and exact observation ingestion boundary.

use ferric_engine::{Engine, EngineError, KvError};
use ferric_m1_benchmarks::{
    encode_canonical_document, load_benchmark_plan, load_canonical_document, main_for,
    sha256_identity, BenchResult, Metric, SecureFileIdentity, SecureInputDirectory, Suite,
};
use rustix::fd::OwnedFd;
use rustix::fs::{
    fsync, mkdirat, openat2, renameat_with, unlinkat, AtFlags, Mode, OFlags, RenameFlags,
    ResolveFlags, CWD,
};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsString;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
#[allow(unused_imports)]
use vstd::prelude::*;

const EXECUTION_FORMAT: &str = "FERRIC-M1-ADVERSARIAL-EXECUTION-V1";
const OBSERVATION_FORMAT: &str = "FERRIC-M1-ADVERSARIAL-OBSERVATION-V1";
const CANARY_LAYOUT_FORMAT: &str = "FERRIC-M1-ADVERSARIAL-CANARY-LAYOUT-V1";
const FAULT_PLAN_FORMAT: &str = "FERRIC-M1-ADVERSARIAL-FAULT-PLAN-V1";
const EXHAUSTION_WORKLOAD_FORMAT: &str = "FERRIC-M1-ADVERSARIAL-EXHAUSTION-V1";
const RUNNER_TRANSCRIPT_FORMAT: &str = "FERRIC-M1-ADVERSARIAL-RUNNER-TRANSCRIPT-V1";
const RAW_RECORD_FORMAT: &str = "FERRIC-M1-ADVERSARIAL-RAW-RECORD-V1";
const RECORDS_FORMAT: &str = "FERRIC-M1-BENCHMARK-RECORDS-V1";
const EXECUTION_AUTHORITY: &str = "externally-supplied-adversarial-execution-only";
const OBSERVATION_AUTHORITY: &str = "externally-collected-adversarial-observation-only";
const RAW_AUTHORITY: &str = "computed-adversarial-observation-only";
const MAX_COMPANION_BYTES: u64 = 64 * 1024 * 1024;
const MAX_EXHAUSTION_CONTEXT_TOKENS: u32 = 1_048_576;
const MAX_EXHAUSTION_PAGE_COUNT: u32 = 4_096;
const MAX_EXHAUSTION_PAGE_TOKENS: u32 = 65_536;

const METRICS: &[Metric] = &[
    Metric {
        id: "canary-corruptions",
        zero_allowed: true,
    },
    Metric {
        id: "faults-observed",
        zero_allowed: true,
    },
    Metric {
        id: "leaked-resources",
        zero_allowed: true,
    },
    Metric {
        id: "rollback-mismatches",
        zero_allowed: true,
    },
    Metric {
        id: "unexpected-errors",
        zero_allowed: true,
    },
];

const SUITE: Suite = Suite {
    name: "adversarial",
    obligation_id: "m1.r30",
    path_id: "adversarial-bench",
    source_path: "benches/m1/adversarial.rs",
    case_kinds: &[
        "canary",
        "cancellation",
        "exhaustion",
        "fault-injection",
        "rollback",
    ],
    extra_identities: &["canary-layout", "fault-plan"],
    metrics: METRICS,
    extra_record_attributes: &["fault-roster-sha256"],
    minimum_warmups: 0,
    minimum_recorded_samples: 1,
    nonclaim: "Structural acceptance authenticates externally collected adversarial records only. It does not establish canary integrity, cancellation safety, exhaustion handling, rollback refinement, fault coverage, hardware correctness, or close m1.r30.",
};

#[derive(Debug)]
struct PlanCase {
    input_sha256: String,
    kind: String,
    workload_sha256: String,
}

#[derive(Debug)]
struct ExecutionCase {
    case_id: String,
    input: PathBuf,
    kind: String,
    observation: Option<PathBuf>,
    workload: PathBuf,
}

#[derive(Debug)]
struct FaultCase {
    injection_point: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Measurements {
    canary_corruptions: u64,
    faults_observed: u64,
    leaked_resources: u64,
    rollback_mismatches: u64,
    unexpected_errors: u64,
}

impl Measurements {
    fn as_json(self) -> Value {
        json!({
            "canary-corruptions": [self.canary_corruptions],
            "faults-observed": [self.faults_observed],
            "leaked-resources": [self.leaked_resources],
            "rollback-mismatches": [self.rollback_mismatches],
            "unexpected-errors": [self.unexpected_errors],
        })
    }
}

#[derive(Debug)]
struct CaseResult {
    details: Value,
    measurements: Measurements,
    source_observation_sha256: String,
    transcript: Vec<u8>,
}

struct RecordContext<'a> {
    canary_layout: &'a str,
    execution: &'a str,
    fault_plan: &'a str,
    input: &'a str,
    plan: &'a str,
    transcript: &'a str,
    workload: &'a str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CanaryRegion {
    expected_byte: u8,
    length: usize,
    name: String,
    offset: usize,
}

#[derive(Debug)]
struct StagingBundle {
    armed: bool,
    output_name: OsString,
    parent: OwnedFd,
    raw: OwnedFd,
    raw_names: Vec<OsString>,
    records_written: bool,
    staging: OwnedFd,
    staging_name: OsString,
    transcript_names: Vec<OsString>,
    transcripts: OwnedFd,
}

impl StagingBundle {
    fn create(output: &Path) -> BenchResult<Self> {
        let output_name = output
            .file_name()
            .map(OsString::from)
            .ok_or_else(|| "output bundle path has no final component".to_owned())?;
        let parent_path = output
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
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
        match openat2(
            &parent,
            output_name.as_os_str(),
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        ) {
            Ok(_) => return Err("output bundle already exists".to_owned()),
            Err(error) if error == rustix::io::Errno::NOENT => {}
            Err(error) => return Err(format!("cannot safely inspect output bundle: {error}")),
        }

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
                    let staging =
                        match open_directory_at(&parent, Path::new(&staging_name), "staging") {
                            Ok(staging) => staging,
                            Err(error) => {
                                let _ =
                                    unlinkat(&parent, staging_name.as_os_str(), AtFlags::REMOVEDIR);
                                return Err(error);
                            }
                        };
                    if let Err(error) =
                        mkdirat(&staging, "raw", Mode::RUSR | Mode::WUSR | Mode::XUSR)
                    {
                        let _ = unlinkat(&parent, staging_name.as_os_str(), AtFlags::REMOVEDIR);
                        return Err(format!("cannot create staged raw directory: {error}"));
                    }
                    if let Err(error) = mkdirat(
                        &staging,
                        "transcripts",
                        Mode::RUSR | Mode::WUSR | Mode::XUSR,
                    ) {
                        let _ = unlinkat(&staging, "raw", AtFlags::REMOVEDIR);
                        let _ = unlinkat(&parent, staging_name.as_os_str(), AtFlags::REMOVEDIR);
                        return Err(format!(
                            "cannot create staged transcript directory: {error}"
                        ));
                    }
                    let raw = match open_directory_at(&staging, Path::new("raw"), "staged raw") {
                        Ok(raw) => raw,
                        Err(error) => {
                            cleanup_empty_staging(&parent, &staging, &staging_name);
                            return Err(error);
                        }
                    };
                    let transcripts = match open_directory_at(
                        &staging,
                        Path::new("transcripts"),
                        "staged transcript",
                    ) {
                        Ok(transcripts) => transcripts,
                        Err(error) => {
                            cleanup_empty_staging(&parent, &staging, &staging_name);
                            return Err(error);
                        }
                    };
                    return Ok(Self {
                        armed: true,
                        output_name,
                        parent,
                        raw,
                        raw_names: Vec::new(),
                        records_written: false,
                        staging,
                        staging_name,
                        transcript_names: Vec::new(),
                        transcripts,
                    });
                }
                Err(error) if error == rustix::io::Errno::EXIST => {}
                Err(error) => return Err(format!("cannot create staging bundle: {error}")),
            }
        }
        Err("staging bundle namespace was exhausted".to_owned())
    }

    fn write_raw(&mut self, name: &str, bytes: &[u8]) -> BenchResult<()> {
        let name = OsString::from(name);
        self.raw_names.push(name.clone());
        write_new_at(&self.raw, Path::new(&name), bytes, "raw adversarial record")?;
        Ok(())
    }

    fn write_transcript(&mut self, name: &str, bytes: &[u8]) -> BenchResult<()> {
        let name = OsString::from(name);
        self.transcript_names.push(name.clone());
        write_new_at(
            &self.transcripts,
            Path::new(&name),
            bytes,
            "runner transcript",
        )?;
        Ok(())
    }

    fn write_records(&mut self, bytes: &[u8]) -> BenchResult<()> {
        self.records_written = true;
        write_new_at(
            &self.staging,
            Path::new("records.json"),
            bytes,
            "benchmark records",
        )?;
        Ok(())
    }

    fn publish(mut self) -> BenchResult<()> {
        fsync(&self.raw).map_err(|error| format!("cannot sync staged raw directory: {error}"))?;
        fsync(&self.transcripts)
            .map_err(|error| format!("cannot sync staged transcript directory: {error}"))?;
        fsync(&self.staging).map_err(|error| format!("cannot sync staging bundle: {error}"))?;
        renameat_with(
            &self.parent,
            self.staging_name.as_os_str(),
            &self.parent,
            self.output_name.as_os_str(),
            RenameFlags::NOREPLACE,
        )
        .map_err(|error| {
            if error == rustix::io::Errno::EXIST {
                "output bundle appeared before no-replace publication".to_owned()
            } else {
                format!("cannot publish staging bundle without replacement: {error}")
            }
        })?;
        self.armed = false;
        if let Err(error) = fsync(&self.parent) {
            eprintln!("WARN: output bundle published but parent sync failed: {error}");
        }
        Ok(())
    }
}

impl Drop for StagingBundle {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        for name in &self.raw_names {
            let _ = unlinkat(&self.raw, name.as_os_str(), AtFlags::empty());
        }
        for name in &self.transcript_names {
            let _ = unlinkat(&self.transcripts, name.as_os_str(), AtFlags::empty());
        }
        if self.records_written {
            let _ = unlinkat(&self.staging, "records.json", AtFlags::empty());
        }
        let _ = unlinkat(&self.staging, "raw", AtFlags::REMOVEDIR);
        let _ = unlinkat(&self.staging, "transcripts", AtFlags::REMOVEDIR);
        let _ = unlinkat(
            &self.parent,
            self.staging_name.as_os_str(),
            AtFlags::REMOVEDIR,
        );
    }
}

fn main() -> ExitCode {
    let arguments = env::args_os().skip(1).collect::<Vec<_>>();
    if arguments
        .first()
        .is_some_and(|command| command == "produce")
    {
        return match produce_command(&arguments) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("FAIL: {error}");
                ExitCode::FAILURE
            }
        };
    }
    main_for(&SUITE)
}

fn produce_command(arguments: &[OsString]) -> BenchResult<()> {
    let [command, plan_path, execution_path, output_bundle] = arguments else {
        return Err("usage: ferric-m1-adversarial produce PLAN EXECUTION OUTPUT-BUNDLE".to_owned());
    };
    if command != "produce" {
        return Err("adversarial producer command drifted".to_owned());
    }
    let (plan, plan_bytes) = load_benchmark_plan(&SUITE, Path::new(plan_path))?;
    let plan_sha256 = sha256_identity(&plan_bytes);
    let plan_cases = exact_plan_cases(&plan)?;
    let plan_identities = plan_identities(&plan)?;
    let (root, execution, execution_bytes) =
        load_canonical_document(Path::new(execution_path), "adversarial execution manifest")?;
    let execution_sha256 = sha256_identity(&execution_bytes);
    let (cases, canary_layout_path, fault_plan_path) =
        parse_execution(&execution, &plan_cases, &plan_sha256)?;

    let mut input_identities = BTreeSet::new();
    let (canary_layout, canary_layout_bytes, canary_identity) =
        root.read_canonical(&canary_layout_path, "adversarial canary layout")?;
    admit_identity(
        &mut input_identities,
        canary_identity,
        "adversarial canary layout",
    )?;
    let canary_layout_sha256 = sha256_identity(&canary_layout_bytes);
    expect_identity(&plan_identities, "canary-layout", &canary_layout_sha256)?;
    let canary_regions = parse_canary_layout(&canary_layout)?;

    let (fault_plan, fault_plan_bytes, fault_identity) =
        root.read_canonical(&fault_plan_path, "adversarial fault plan")?;
    admit_identity(
        &mut input_identities,
        fault_identity,
        "adversarial fault plan",
    )?;
    let fault_plan_sha256 = sha256_identity(&fault_plan_bytes);
    expect_identity(&plan_identities, "fault-plan", &fault_plan_sha256)?;
    let fault_cases = parse_fault_plan(&fault_plan)?;

    let mut staging = StagingBundle::create(Path::new(output_bundle))?;
    let mut observations = Vec::with_capacity(cases.len());
    for case in cases {
        let plan_case = plan_cases
            .get(&case.case_id)
            .ok_or_else(|| format!("execution names unknown case: {}", case.case_id))?;
        let (_input, input_bytes, input_identity) =
            root.read_canonical(&case.input, "adversarial case input")?;
        admit_identity(
            &mut input_identities,
            input_identity,
            "adversarial case input",
        )?;
        let input_sha256 = sha256_identity(&input_bytes);
        if input_sha256 != plan_case.input_sha256 {
            return Err(format!("case input identity drifted: {}", case.case_id));
        }
        let (workload, workload_bytes, workload_identity) =
            root.read_canonical(&case.workload, "adversarial case workload")?;
        admit_identity(
            &mut input_identities,
            workload_identity,
            "adversarial case workload",
        )?;
        let workload_sha256 = sha256_identity(&workload_bytes);
        if workload_sha256 != plan_case.workload_sha256 {
            return Err(format!("case workload identity drifted: {}", case.case_id));
        }
        let fault_case = fault_cases
            .get(&case.kind)
            .ok_or_else(|| format!("fault plan omitted case kind: {}", case.kind))?;
        let context = RunContext {
            canary_regions: &canary_regions,
            case: &case,
            execution_sha256: &execution_sha256,
            fault_case,
            fault_plan_sha256: &fault_plan_sha256,
            input_sha256: &input_sha256,
            plan_sha256: &plan_sha256,
            root: &root,
            seen_identities: &mut input_identities,
            workload: &workload,
            workload_sha256: &workload_sha256,
        };
        let result = run_case(context)?;
        let transcript_sha256 = sha256_identity(&result.transcript);
        staging.write_transcript(
            &format!("{}.runner.transcript", case.case_id),
            &result.transcript,
        )?;
        let record_context = RecordContext {
            canary_layout: &canary_layout_sha256,
            execution: &execution_sha256,
            fault_plan: &fault_plan_sha256,
            input: &input_sha256,
            plan: &plan_sha256,
            transcript: &transcript_sha256,
            workload: &workload_sha256,
        };
        let raw = raw_record(&case, &result, &record_context);
        let raw_bytes = encode_canonical_document(&raw)?;
        staging.write_raw(
            &format!("{}.adversarial.raw.json", case.case_id),
            &raw_bytes,
        )?;
        observations.push(observation(
            &case,
            result.measurements,
            &fault_plan_sha256,
            &raw_bytes,
            &transcript_sha256,
        ));
    }
    let records = json!({
        "format": RECORDS_FORMAT,
        "observations": observations,
        "plan_sha256": plan_sha256,
        "suite": SUITE.name,
    });
    staging.write_records(&encode_canonical_document(&records)?)?;
    staging.publish()
}

struct RunContext<'a> {
    canary_regions: &'a [CanaryRegion],
    case: &'a ExecutionCase,
    execution_sha256: &'a str,
    fault_case: &'a FaultCase,
    fault_plan_sha256: &'a str,
    input_sha256: &'a str,
    plan_sha256: &'a str,
    root: &'a SecureInputDirectory,
    seen_identities: &'a mut BTreeSet<SecureFileIdentity>,
    workload: &'a Value,
    workload_sha256: &'a str,
}

fn run_case(mut context: RunContext<'_>) -> BenchResult<CaseResult> {
    match context.case.kind.as_str() {
        "canary" => run_canary(&mut context),
        "cancellation" => run_external_cancellation(&mut context),
        "exhaustion" => run_exhaustion(&context),
        "fault-injection" => run_external_fault_injection(&mut context),
        "rollback" => run_external_rollback(&mut context),
        kind => Err(format!("unsupported adversarial execution kind: {kind}")),
    }
}

fn run_canary(context: &mut RunContext<'_>) -> BenchResult<CaseResult> {
    let (result, transcript, observation_sha256) = load_external_observation(context)?;
    let result = object(&result, &["after", "before"], "canary observation result")?;
    let (before, before_identity) = read_companion(
        context.root,
        field(result, "before", "canary observation result")?,
        "canary before snapshot",
    )?;
    admit_identity(
        context.seen_identities,
        before_identity,
        "canary before snapshot",
    )?;
    let (after, after_identity) = read_companion(
        context.root,
        field(result, "after", "canary observation result")?,
        "canary after snapshot",
    )?;
    admit_identity(
        context.seen_identities,
        after_identity,
        "canary after snapshot",
    )?;
    if before.len() != after.len() {
        return Err("canary snapshots have different lengths".to_owned());
    }
    let corruptions = compare_canaries(context.canary_regions, &before, &after)?;
    let observed_outcome = if corruptions == 0 {
        "canary-intact"
    } else {
        "canary-corruption-detected"
    };
    let matched = outcome_matches(context.fault_case, observed_outcome);
    Ok(CaseResult {
        details: json!({
            "execution_domain": "external-snapshot-comparison",
            "hardware_claim": "none",
            "injection_point": context.fault_case.injection_point,
            "observed_outcome": observed_outcome,
            "snapshot_bytes": before.len(),
        }),
        measurements: Measurements {
            canary_corruptions: corruptions,
            faults_observed: u64::from(matched),
            leaked_resources: 0,
            rollback_mismatches: 0,
            unexpected_errors: u64::from(!matched),
        },
        source_observation_sha256: observation_sha256,
        transcript,
    })
}

fn run_external_cancellation(context: &mut RunContext<'_>) -> BenchResult<CaseResult> {
    let (result, transcript, observation_sha256) = load_external_observation(context)?;
    let result = object(
        &result,
        &[
            "completion_observed",
            "free_pages_after",
            "free_pages_before",
            "live_requests_after",
            "reclaimed_after_completion",
            "reclaimed_before_completion",
        ],
        "cancellation observation result",
    )?;
    let completion_observed = boolean(result, "completion_observed", "cancellation result")?;
    let free_before = unsigned(result, "free_pages_before", "cancellation result")?;
    let free_after = unsigned(result, "free_pages_after", "cancellation result")?;
    let live_after = unsigned(result, "live_requests_after", "cancellation result")?;
    let reclaimed_after = boolean(result, "reclaimed_after_completion", "cancellation result")?;
    let reclaimed_before = boolean(result, "reclaimed_before_completion", "cancellation result")?;
    let leaked = free_before.abs_diff(free_after).saturating_add(live_after);
    let violations = u64::from(!completion_observed)
        .saturating_add(u64::from(reclaimed_before))
        .saturating_add(u64::from(!reclaimed_after))
        .saturating_add(u64::from(leaked != 0));
    let observed_outcome = if violations == 0 {
        "cancelled-after-exact-completion"
    } else {
        "cancellation-boundary-violation"
    };
    let matched = outcome_matches(context.fault_case, observed_outcome);
    Ok(CaseResult {
        details: json!({
            "execution_domain": "external-physical-observation",
            "hardware_claim": "none",
            "injection_point": context.fault_case.injection_point,
            "observed_outcome": observed_outcome,
        }),
        measurements: Measurements {
            canary_corruptions: 0,
            faults_observed: u64::from(matched),
            leaked_resources: leaked,
            rollback_mismatches: 0,
            unexpected_errors: violations.saturating_add(u64::from(!matched)),
        },
        source_observation_sha256: observation_sha256,
        transcript,
    })
}

fn run_exhaustion(context: &RunContext<'_>) -> BenchResult<CaseResult> {
    if context.case.observation.is_some() {
        return Err("exhaustion case must not substitute an external observation".to_owned());
    }
    let workload = object(
        context.workload,
        &["format", "kind", "parameters"],
        "exhaustion workload",
    )?;
    expect(
        workload,
        "format",
        EXHAUSTION_WORKLOAD_FORMAT,
        "exhaustion workload format",
    )?;
    expect(workload, "kind", "exhaustion", "exhaustion workload kind")?;
    let parameters = object(
        field(workload, "parameters", "exhaustion workload")?,
        &[
            "first_append_tokens",
            "max_context_tokens",
            "page_count",
            "page_tokens",
            "rejected_append_tokens",
        ],
        "exhaustion parameters",
    )?;
    let page_count = unsigned_u32(parameters, "page_count", "exhaustion parameters")?;
    let page_tokens = unsigned_u32(parameters, "page_tokens", "exhaustion parameters")?;
    let max_context = unsigned_u32(parameters, "max_context_tokens", "exhaustion parameters")?;
    let first_append = unsigned_u32(parameters, "first_append_tokens", "exhaustion parameters")?;
    let rejected_append = unsigned_u32(
        parameters,
        "rejected_append_tokens",
        "exhaustion parameters",
    )?;
    let capacity = page_count
        .checked_mul(page_tokens)
        .ok_or_else(|| "exhaustion capacity overflowed".to_owned())?;
    if page_count == 0
        || page_tokens == 0
        || page_count > MAX_EXHAUSTION_PAGE_COUNT
        || page_tokens > MAX_EXHAUSTION_PAGE_TOKENS
        || max_context > MAX_EXHAUSTION_CONTEXT_TOKENS
        || first_append != capacity
        || rejected_append == 0
        || first_append
            .checked_add(rejected_append)
            .is_none_or(|total| total > max_context)
    {
        return Err("exhaustion workload does not force only the KV page boundary".to_owned());
    }

    let mut engine = Engine::<1>::new(page_count, page_tokens, max_context)
        .map_err(|error| format!("cannot construct exhaustion engine: {error:?}"))?;
    let free_before = engine.free_pages();
    let request = engine
        .admit()
        .map_err(|error| format!("cannot admit exhaustion request: {error:?}"))?;
    engine
        .append_tentative(request, first_append)
        .map_err(|error| format!("cannot fill exhaustion pages: {error:?}"))?;
    let resident_before = engine.resident_tokens(request);
    let free_at_boundary = engine.free_pages();
    let rejected = engine.append_tentative(request, rejected_append);
    let observed = rejected == Err(EngineError::Kv(KvError::OutOfPages));
    let transactional = engine.resident_tokens(request) == resident_before
        && engine.free_pages() == free_at_boundary
        && !engine.is_faulted();
    engine
        .retire(request)
        .map_err(|error| format!("cannot retire exhaustion request: {error:?}"))?;
    let reclaimed = engine
        .reclaim_one()
        .map_err(|error| format!("cannot reclaim exhaustion request: {error:?}"))?;
    let leaked = u64::try_from(engine.live_count())
        .unwrap_or(u64::MAX)
        .saturating_add(u64::from(free_before.abs_diff(engine.free_pages())));
    let violations = u64::from(!observed)
        .saturating_add(u64::from(!transactional))
        .saturating_add(u64::from(reclaimed != Some(request)))
        .saturating_add(u64::from(leaked != 0));
    let observed_outcome = if violations == 0 {
        "out-of-pages-transactional"
    } else {
        "exhaustion-boundary-violation"
    };
    let matched = outcome_matches(context.fault_case, observed_outcome);
    let details = json!({
        "execution_domain": "ferric-engine-logical",
        "hardware_claim": "none",
        "injection_point": context.fault_case.injection_point,
        "observed_outcome": observed_outcome,
        "page_count": page_count,
        "page_tokens": page_tokens,
    });
    let transcript = encode_canonical_document(&json!({
        "authority": RAW_AUTHORITY,
        "case_id": context.case.case_id,
        "details": details,
        "execution_sha256": context.execution_sha256,
        "fault_plan_sha256": context.fault_plan_sha256,
        "format": RUNNER_TRANSCRIPT_FORMAT,
        "input_sha256": context.input_sha256,
        "kind": context.case.kind,
        "plan_sha256": context.plan_sha256,
        "status": "completed",
        "workload_sha256": context.workload_sha256,
    }))?;
    Ok(CaseResult {
        details,
        measurements: Measurements {
            canary_corruptions: 0,
            faults_observed: u64::from(matched),
            leaked_resources: leaked,
            rollback_mismatches: 0,
            unexpected_errors: violations.saturating_add(u64::from(!matched)),
        },
        source_observation_sha256: sha256_identity(&transcript),
        transcript,
    })
}

fn run_external_fault_injection(context: &mut RunContext<'_>) -> BenchResult<CaseResult> {
    let (result, transcript, observation_sha256) = load_external_observation(context)?;
    let result = object(
        &result,
        &[
            "failures_observed",
            "faults_injected",
            "live_resources_after",
            "queue_quarantined",
            "retry_denied",
        ],
        "fault-injection observation result",
    )?;
    let injected = unsigned(result, "faults_injected", "fault-injection result")?;
    let observed = unsigned(result, "failures_observed", "fault-injection result")?;
    let live_resources = unsigned(result, "live_resources_after", "fault-injection result")?;
    let quarantined = boolean(result, "queue_quarantined", "fault-injection result")?;
    let retry_denied = boolean(result, "retry_denied", "fault-injection result")?;
    let violations = u64::from(injected == 0)
        .saturating_add(injected.abs_diff(observed))
        .saturating_add(u64::from(!quarantined))
        .saturating_add(u64::from(!retry_denied))
        .saturating_add(u64::from(live_resources != 0));
    let observed_outcome = if violations == 0 {
        "terminal-fault-quarantined"
    } else {
        "fault-injection-boundary-violation"
    };
    let matched = outcome_matches(context.fault_case, observed_outcome);
    Ok(CaseResult {
        details: json!({
            "execution_domain": "external-physical-observation",
            "hardware_claim": "none",
            "injection_point": context.fault_case.injection_point,
            "observed_outcome": observed_outcome,
        }),
        measurements: Measurements {
            canary_corruptions: 0,
            faults_observed: observed,
            leaked_resources: live_resources,
            rollback_mismatches: 0,
            unexpected_errors: violations.saturating_add(u64::from(!matched)),
        },
        source_observation_sha256: observation_sha256,
        transcript,
    })
}

fn run_external_rollback(context: &mut RunContext<'_>) -> BenchResult<CaseResult> {
    let (result, transcript, observation_sha256) = load_external_observation(context)?;
    let result = object(
        &result,
        &[
            "accepted_tokens",
            "committed_tokens_after",
            "committed_tokens_before",
            "free_pages_after_cleanup",
            "free_pages_before",
            "live_requests_after_cleanup",
            "resident_tokens_after",
            "resident_tokens_before",
        ],
        "rollback observation result",
    )?;
    let accepted = unsigned(result, "accepted_tokens", "rollback result")?;
    let committed_before = unsigned(result, "committed_tokens_before", "rollback result")?;
    let committed_after = unsigned(result, "committed_tokens_after", "rollback result")?;
    let resident_before = unsigned(result, "resident_tokens_before", "rollback result")?;
    let resident_after = unsigned(result, "resident_tokens_after", "rollback result")?;
    let free_before = unsigned(result, "free_pages_before", "rollback result")?;
    let free_after = unsigned(result, "free_pages_after_cleanup", "rollback result")?;
    let live_after = unsigned(result, "live_requests_after_cleanup", "rollback result")?;
    let expected_after = committed_before.checked_add(accepted);
    let tentative = resident_before.saturating_sub(committed_before);
    let mismatches = u64::from(resident_before <= committed_before)
        .saturating_add(u64::from(accepted >= tentative))
        .saturating_add(u64::from(expected_after != Some(committed_after)))
        .saturating_add(u64::from(resident_after != committed_after));
    let leaked = free_before.abs_diff(free_after).saturating_add(live_after);
    let observed_outcome = if mismatches == 0 && leaked == 0 {
        "strict-prefix-rollback-refined"
    } else {
        "rollback-boundary-violation"
    };
    let matched = outcome_matches(context.fault_case, observed_outcome);
    Ok(CaseResult {
        details: json!({
            "execution_domain": "external-physical-observation",
            "hardware_claim": "none",
            "injection_point": context.fault_case.injection_point,
            "observed_outcome": observed_outcome,
        }),
        measurements: Measurements {
            canary_corruptions: 0,
            faults_observed: u64::from(matched),
            leaked_resources: leaked,
            rollback_mismatches: mismatches,
            unexpected_errors: u64::from(!matched),
        },
        source_observation_sha256: observation_sha256,
        transcript,
    })
}

fn load_external_observation(
    context: &mut RunContext<'_>,
) -> BenchResult<(Value, Vec<u8>, String)> {
    let path = context.case.observation.as_ref().ok_or_else(|| {
        format!(
            "completion-dependent case requires an external observation: {}",
            context.case.case_id
        )
    })?;
    let (observation, observation_bytes, identity) = context
        .root
        .read_canonical(path, "adversarial external observation")?;
    admit_identity(
        context.seen_identities,
        identity,
        "adversarial external observation",
    )?;
    let document = object(
        &observation,
        &[
            "authority",
            "case_id",
            "format",
            "input_sha256",
            "kind",
            "plan_sha256",
            "result",
            "runner_transcript",
            "workload_sha256",
        ],
        "adversarial external observation",
    )?;
    expect(
        document,
        "authority",
        OBSERVATION_AUTHORITY,
        "observation authority",
    )?;
    expect(document, "format", OBSERVATION_FORMAT, "observation format")?;
    expect(
        document,
        "case_id",
        &context.case.case_id,
        "observation case id",
    )?;
    expect(document, "kind", &context.case.kind, "observation kind")?;
    expect(
        document,
        "plan_sha256",
        context.plan_sha256,
        "observation plan identity",
    )?;
    expect(
        document,
        "input_sha256",
        context.input_sha256,
        "observation input identity",
    )?;
    expect(
        document,
        "workload_sha256",
        context.workload_sha256,
        "observation workload identity",
    )?;
    let result = field(document, "result", "adversarial external observation")?.clone();
    let (transcript, transcript_identity) = read_companion(
        context.root,
        field(
            document,
            "runner_transcript",
            "adversarial external observation",
        )?,
        "external runner transcript",
    )?;
    admit_identity(
        context.seen_identities,
        transcript_identity,
        "external runner transcript",
    )?;
    Ok((result, transcript, sha256_identity(&observation_bytes)))
}

fn parse_execution(
    value: &Value,
    plan_cases: &BTreeMap<String, PlanCase>,
    plan_sha256: &str,
) -> BenchResult<(Vec<ExecutionCase>, PathBuf, PathBuf)> {
    let document = object(
        value,
        &[
            "authority",
            "canary_layout",
            "cases",
            "fault_plan",
            "format",
            "plan_sha256",
            "suite",
        ],
        "adversarial execution manifest",
    )?;
    expect(
        document,
        "authority",
        EXECUTION_AUTHORITY,
        "execution authority",
    )?;
    expect(document, "format", EXECUTION_FORMAT, "execution format")?;
    expect(
        document,
        "plan_sha256",
        plan_sha256,
        "execution plan identity",
    )?;
    expect(document, "suite", SUITE.name, "execution suite")?;
    let canary_layout = relative_path(string(
        document,
        "canary_layout",
        "adversarial execution manifest",
    )?)?;
    let fault_plan = relative_path(string(
        document,
        "fault_plan",
        "adversarial execution manifest",
    )?)?;
    let values = field(document, "cases", "adversarial execution manifest")?
        .as_array()
        .ok_or_else(|| "adversarial execution cases must be an array".to_owned())?;
    if values.len() != plan_cases.len() || values.len() != SUITE.case_kinds.len() {
        return Err("adversarial execution must contain exactly five cases".to_owned());
    }
    let mut prior: Option<&str> = None;
    let mut seen = BTreeSet::new();
    let mut paths = BTreeSet::new();
    let mut cases = Vec::with_capacity(values.len());
    for value in values {
        let case = object(
            value,
            &["case_id", "input", "kind", "observation", "workload"],
            "adversarial execution case",
        )?;
        let case_id = string(case, "case_id", "adversarial execution case")?;
        if prior.is_some_and(|previous| previous >= case_id) {
            return Err("adversarial execution cases must be uniquely sorted".to_owned());
        }
        prior = Some(case_id);
        let kind = string(case, "kind", "adversarial execution case")?;
        let planned = plan_cases
            .get(case_id)
            .ok_or_else(|| format!("execution names unknown case: {case_id}"))?;
        if planned.kind != kind {
            return Err(format!("execution case kind drifted: {case_id}"));
        }
        let input = relative_path(string(case, "input", "adversarial execution case")?)?;
        let workload = relative_path(string(case, "workload", "adversarial execution case")?)?;
        let observation = match field(case, "observation", "adversarial execution case")? {
            Value::Null => None,
            Value::String(path) => Some(relative_path(path)?),
            _ => return Err("execution observation must be a relative path or null".to_owned()),
        };
        for path in [&input, &workload].into_iter().chain(observation.iter()) {
            if !paths.insert(path.clone()) {
                return Err("adversarial execution paths must be unique".to_owned());
            }
        }
        seen.insert(kind);
        cases.push(ExecutionCase {
            case_id: case_id.to_owned(),
            input,
            kind: kind.to_owned(),
            observation,
            workload,
        });
    }
    if seen != SUITE.case_kinds.iter().copied().collect() {
        return Err("adversarial execution case-kind roster drifted".to_owned());
    }
    if !paths.insert(canary_layout.clone()) || !paths.insert(fault_plan.clone()) {
        return Err("adversarial execution companion paths must be unique".to_owned());
    }
    Ok((cases, canary_layout, fault_plan))
}

fn exact_plan_cases(plan: &Value) -> BenchResult<BTreeMap<String, PlanCase>> {
    let plan = plan
        .as_object()
        .ok_or_else(|| "benchmark plan must be an object".to_owned())?;
    let cases = field(plan, "cases", "benchmark plan")?
        .as_array()
        .ok_or_else(|| "benchmark plan cases must be an array".to_owned())?;
    if cases.len() != SUITE.case_kinds.len() {
        return Err("adversarial producer requires exactly one case of each kind".to_owned());
    }
    let mut by_id = BTreeMap::new();
    let mut kinds = BTreeSet::new();
    for case in cases {
        let case = object(
            case,
            &["id", "input_sha256", "kind", "workload_sha256"],
            "benchmark case",
        )?;
        let id = string(case, "id", "benchmark case")?;
        let kind = string(case, "kind", "benchmark case")?;
        by_id.insert(
            id.to_owned(),
            PlanCase {
                input_sha256: string(case, "input_sha256", "benchmark case")?.to_owned(),
                kind: kind.to_owned(),
                workload_sha256: string(case, "workload_sha256", "benchmark case")?.to_owned(),
            },
        );
        kinds.insert(kind);
    }
    if kinds != SUITE.case_kinds.iter().copied().collect() {
        return Err("adversarial plan case-kind roster drifted".to_owned());
    }
    Ok(by_id)
}

fn plan_identities(plan: &Value) -> BenchResult<BTreeMap<String, String>> {
    let plan = plan
        .as_object()
        .ok_or_else(|| "benchmark plan must be an object".to_owned())?;
    field(plan, "identities", "benchmark plan")?
        .as_object()
        .ok_or_else(|| "benchmark plan identities must be an object".to_owned())?
        .iter()
        .map(|(name, value)| {
            value
                .as_str()
                .map(|identity| (name.clone(), identity.to_owned()))
                .ok_or_else(|| format!("benchmark identity must be a string: {name}"))
        })
        .collect()
}

fn parse_canary_layout(value: &Value) -> BenchResult<Vec<CanaryRegion>> {
    let document = object(value, &["format", "regions"], "adversarial canary layout")?;
    expect(
        document,
        "format",
        CANARY_LAYOUT_FORMAT,
        "canary layout format",
    )?;
    let values = field(document, "regions", "adversarial canary layout")?
        .as_array()
        .ok_or_else(|| "canary regions must be an array".to_owned())?;
    if values.is_empty() || values.len() > 1_024 {
        return Err("canary region count is outside the admitted bound".to_owned());
    }
    let mut regions = Vec::with_capacity(values.len());
    let mut names = BTreeSet::new();
    let mut prior_end = 0_usize;
    for value in values {
        let region = object(
            value,
            &["expected_byte", "length", "name", "offset"],
            "canary region",
        )?;
        let name = string(region, "name", "canary region")?;
        require_safe_id(name, "canary region name")?;
        if !names.insert(name) {
            return Err("canary region names must be unique".to_owned());
        }
        let offset = unsigned_usize(region, "offset", "canary region")?;
        let length = unsigned_usize(region, "length", "canary region")?;
        let expected_byte = unsigned(region, "expected_byte", "canary region")?;
        if length == 0 || offset < prior_end || expected_byte > u64::from(u8::MAX) {
            return Err("canary regions must be nonempty, sorted, and disjoint".to_owned());
        }
        prior_end = offset
            .checked_add(length)
            .ok_or_else(|| "canary region range overflowed".to_owned())?;
        regions.push(CanaryRegion {
            expected_byte: u8::try_from(expected_byte)
                .map_err(|_| "canary expected byte does not fit u8".to_owned())?,
            length,
            name: name.to_owned(),
            offset,
        });
    }
    Ok(regions)
}

fn parse_fault_plan(value: &Value) -> BenchResult<BTreeMap<String, FaultCase>> {
    let document = object(value, &["faults", "format"], "adversarial fault plan")?;
    expect(document, "format", FAULT_PLAN_FORMAT, "fault plan format")?;
    let values = field(document, "faults", "adversarial fault plan")?
        .as_array()
        .ok_or_else(|| "fault plan faults must be an array".to_owned())?;
    if values.len() != SUITE.case_kinds.len() {
        return Err("fault plan must cover exactly five case kinds".to_owned());
    }
    let mut faults = BTreeMap::new();
    let mut ids = BTreeSet::new();
    let mut prior: Option<&str> = None;
    for value in values {
        let fault = object(
            value,
            &["case_kind", "expected_outcome", "id", "injection_point"],
            "fault plan entry",
        )?;
        let kind = string(fault, "case_kind", "fault plan entry")?;
        if prior.is_some_and(|previous| previous >= kind) {
            return Err("fault plan entries must be uniquely sorted by case kind".to_owned());
        }
        prior = Some(kind);
        if !SUITE.case_kinds.contains(&kind) {
            return Err(format!("fault plan contains unknown case kind: {kind}"));
        }
        let id = string(fault, "id", "fault plan entry")?;
        require_safe_id(id, "fault id")?;
        if !ids.insert(id) {
            return Err("fault plan ids must be unique".to_owned());
        }
        let point = string(fault, "injection_point", "fault plan entry")?;
        expect_value(
            point,
            expected_injection_point(kind),
            "fault injection point",
        )?;
        let expected = string(fault, "expected_outcome", "fault plan entry")?;
        expect_value(expected, expected_outcome(kind), "fault expected outcome")?;
        faults.insert(
            kind.to_owned(),
            FaultCase {
                injection_point: point.to_owned(),
            },
        );
    }
    if faults.keys().map(String::as_str).collect::<BTreeSet<_>>()
        != SUITE.case_kinds.iter().copied().collect()
    {
        return Err("fault plan case-kind roster drifted".to_owned());
    }
    Ok(faults)
}

fn expected_injection_point(kind: &str) -> &'static str {
    match kind {
        "canary" => "guard-bytes",
        "cancellation" => "in-flight-retirement",
        "exhaustion" => "kv-page-allocation",
        "fault-injection" => "queue-transition",
        "rollback" => "accepted-prefix",
        _ => "unknown",
    }
}

fn expected_outcome(kind: &str) -> &'static str {
    match kind {
        "canary" => "canary-intact",
        "cancellation" => "cancelled-after-exact-completion",
        "exhaustion" => "out-of-pages-transactional",
        "fault-injection" => "terminal-fault-quarantined",
        "rollback" => "strict-prefix-rollback-refined",
        _ => "unknown",
    }
}

fn outcome_matches(fault: &FaultCase, observed: &str) -> bool {
    observed == expected_outcome_from_point(&fault.injection_point)
}

fn expected_outcome_from_point(point: &str) -> &'static str {
    match point {
        "guard-bytes" => "canary-intact",
        "in-flight-retirement" => "cancelled-after-exact-completion",
        "kv-page-allocation" => "out-of-pages-transactional",
        "queue-transition" => "terminal-fault-quarantined",
        "accepted-prefix" => "strict-prefix-rollback-refined",
        _ => "unknown",
    }
}

fn compare_canaries(regions: &[CanaryRegion], before: &[u8], after: &[u8]) -> BenchResult<u64> {
    let mut corruptions = 0_u64;
    for region in regions {
        let end = region
            .offset
            .checked_add(region.length)
            .ok_or_else(|| format!("canary region overflowed: {}", region.name))?;
        let before_region = before
            .get(region.offset..end)
            .ok_or_else(|| format!("canary before snapshot is short: {}", region.name))?;
        let after_region = after
            .get(region.offset..end)
            .ok_or_else(|| format!("canary after snapshot is short: {}", region.name))?;
        if before_region
            .iter()
            .any(|byte| *byte != region.expected_byte)
        {
            return Err(format!(
                "canary baseline does not match its layout: {}",
                region.name
            ));
        }
        let changed = after_region
            .iter()
            .filter(|byte| **byte != region.expected_byte)
            .count();
        corruptions = corruptions
            .checked_add(
                u64::try_from(changed)
                    .map_err(|_| "canary corruption count does not fit u64".to_owned())?,
            )
            .ok_or_else(|| "canary corruption count overflowed".to_owned())?;
    }
    Ok(corruptions)
}

fn raw_record(case: &ExecutionCase, result: &CaseResult, context: &RecordContext<'_>) -> Value {
    json!({
        "authority": RAW_AUTHORITY,
        "canary_layout_sha256": context.canary_layout,
        "case_id": case.case_id,
        "details": result.details,
        "execution_sha256": context.execution,
        "fault_plan_sha256": context.fault_plan,
        "format": RAW_RECORD_FORMAT,
        "input_sha256": context.input,
        "kind": case.kind,
        "measurements": result.measurements.as_json(),
        "plan_sha256": context.plan,
        "runner_transcript_sha256": context.transcript,
        "source_observation_sha256": result.source_observation_sha256,
        "status": "observed",
        "workload_sha256": context.workload,
    })
}

fn observation(
    case: &ExecutionCase,
    measurements: Measurements,
    fault_plan_sha256: &str,
    raw_bytes: &[u8],
    transcript_sha256: &str,
) -> Value {
    json!({
        "attributes": {
            "fault-roster-sha256": fault_plan_sha256,
            "raw-record-sha256": sha256_identity(raw_bytes),
            "runner-transcript-sha256": transcript_sha256,
        },
        "case_id": case.case_id,
        "kind": case.kind,
        "measurements": measurements.as_json(),
        "recorded_samples": 1,
        "status": "completed",
        "warmups": 0,
    })
}

fn read_companion(
    root: &SecureInputDirectory,
    value: &Value,
    description: &str,
) -> BenchResult<(Vec<u8>, SecureFileIdentity)> {
    let companion = object(value, &["bytes", "path", "sha256"], description)?;
    let bytes = unsigned(companion, "bytes", description)?;
    if bytes == 0 || bytes > MAX_COMPANION_BYTES {
        return Err(format!("{description} size is outside the admitted bound"));
    }
    let path = relative_path(string(companion, "path", description)?)?;
    let expected_sha256 = string(companion, "sha256", description)?;
    require_sha256(expected_sha256, description)?;
    let mut input = root.open_exact(&path, bytes, description)?;
    let capacity =
        usize::try_from(bytes).map_err(|_| format!("{description} size does not fit this host"))?;
    let mut contents = Vec::new();
    contents
        .try_reserve_exact(capacity)
        .map_err(|_| format!("cannot reserve {description} buffer"))?;
    let read_result = input.read_to_end(&mut contents);
    let snapshot_result = input.validate_snapshot(description);
    if let Err(error) = read_result {
        snapshot_result?;
        return Err(format!("cannot read {description}: {error}"));
    }
    snapshot_result?;
    if contents.len() != capacity || sha256_identity(&contents) != expected_sha256 {
        return Err(format!("{description} content identity drifted"));
    }
    Ok((contents, input.identity()))
}

fn cleanup_empty_staging(parent: &OwnedFd, staging: &OwnedFd, staging_name: &OsString) {
    let _ = unlinkat(staging, "raw", AtFlags::REMOVEDIR);
    let _ = unlinkat(staging, "transcripts", AtFlags::REMOVEDIR);
    let _ = unlinkat(parent, staging_name.as_os_str(), AtFlags::REMOVEDIR);
}

fn open_directory_at(parent: &OwnedFd, relative: &Path, description: &str) -> BenchResult<OwnedFd> {
    openat2(
        parent,
        relative,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )
    .map_err(|error| format!("cannot securely open {description} directory: {error}"))
}

fn write_new_at(
    parent: &OwnedFd,
    relative: &Path,
    bytes: &[u8],
    description: &str,
) -> BenchResult<()> {
    let descriptor = openat2(
        parent,
        relative,
        OFlags::WRONLY
            | OFlags::CREATE
            | OFlags::EXCL
            | OFlags::NOFOLLOW
            | OFlags::NONBLOCK
            | OFlags::CLOEXEC,
        Mode::RUSR | Mode::WUSR,
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )
    .map_err(|error| format!("cannot create staged {description}: {error}"))?;
    let mut file = File::from(descriptor);
    file.write_all(bytes)
        .map_err(|error| format!("cannot write staged {description}: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("cannot sync staged {description}: {error}"))
}

fn admit_identity(
    identities: &mut BTreeSet<SecureFileIdentity>,
    identity: SecureFileIdentity,
    description: &str,
) -> BenchResult<()> {
    if !identities.insert(identity) {
        return Err(format!("{description} aliases another execution input"));
    }
    Ok(())
}

fn expect_identity(
    identities: &BTreeMap<String, String>,
    name: &str,
    actual: &str,
) -> BenchResult<()> {
    let expected = identities
        .get(name)
        .ok_or_else(|| format!("benchmark plan omitted identity: {name}"))?;
    if expected != actual {
        return Err(format!("benchmark identity drifted: {name}"));
    }
    Ok(())
}

fn object<'a>(
    value: &'a Value,
    keys: &[&str],
    description: &str,
) -> BenchResult<&'a Map<String, Value>> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{description} must be an object"))?;
    let actual = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
    let expected = keys.iter().copied().collect::<BTreeSet<_>>();
    if actual != expected || keys.len() != expected.len() {
        return Err(format!("{description} fields drifted"));
    }
    Ok(object)
}

fn field<'a>(
    object: &'a Map<String, Value>,
    name: &str,
    description: &str,
) -> BenchResult<&'a Value> {
    object
        .get(name)
        .ok_or_else(|| format!("{description} omitted {name}"))
}

fn string<'a>(
    object: &'a Map<String, Value>,
    name: &str,
    description: &str,
) -> BenchResult<&'a str> {
    field(object, name, description)?
        .as_str()
        .ok_or_else(|| format!("{description} {name} must be a string"))
}

fn boolean(object: &Map<String, Value>, name: &str, description: &str) -> BenchResult<bool> {
    field(object, name, description)?
        .as_bool()
        .ok_or_else(|| format!("{description} {name} must be a boolean"))
}

fn unsigned(object: &Map<String, Value>, name: &str, description: &str) -> BenchResult<u64> {
    field(object, name, description)?
        .as_u64()
        .ok_or_else(|| format!("{description} {name} must be an unsigned integer"))
}

fn unsigned_u32(object: &Map<String, Value>, name: &str, description: &str) -> BenchResult<u32> {
    u32::try_from(unsigned(object, name, description)?)
        .map_err(|_| format!("{description} {name} does not fit u32"))
}

fn unsigned_usize(
    object: &Map<String, Value>,
    name: &str,
    description: &str,
) -> BenchResult<usize> {
    usize::try_from(unsigned(object, name, description)?)
        .map_err(|_| format!("{description} {name} does not fit this host"))
}

fn expect(
    object: &Map<String, Value>,
    name: &str,
    expected: &str,
    description: &str,
) -> BenchResult<()> {
    expect_value(string(object, name, description)?, expected, description)
}

fn expect_value(actual: &str, expected: &str, description: &str) -> BenchResult<()> {
    if actual != expected {
        return Err(format!("{description} drifted"));
    }
    Ok(())
}

fn require_sha256(value: &str, description: &str) -> BenchResult<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!("{description} is not lowercase SHA-256"));
    }
    Ok(())
}

fn require_safe_id(value: &str, description: &str) -> BenchResult<()> {
    if value.is_empty()
        || value.len() > 96
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-' | b'_')
        })
    {
        return Err(format!("{description} is not a safe identifier"));
    }
    Ok(())
}

fn relative_path(value: &str) -> BenchResult<PathBuf> {
    let path = PathBuf::from(value);
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err("execution companion path must be nonempty and relative".to_owned());
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::{compare_canaries, parse_canary_layout, CanaryRegion};
    use serde_json::json;

    #[test]
    fn canary_comparison_counts_only_guard_corruption() {
        let regions = [
            CanaryRegion {
                expected_byte: 0xa5,
                length: 2,
                name: "prefix".to_owned(),
                offset: 0,
            },
            CanaryRegion {
                expected_byte: 0x5a,
                length: 2,
                name: "suffix".to_owned(),
                offset: 4,
            },
        ];
        let before = [0xa5, 0xa5, 1, 2, 0x5a, 0x5a];
        let after = [0xa5, 0, 9, 9, 0x5a, 1];
        assert_eq!(compare_canaries(&regions, &before, &after), Ok(2));
    }

    #[test]
    fn canary_comparison_rejects_invalid_baseline_and_short_snapshot() {
        let regions = [CanaryRegion {
            expected_byte: 0xa5,
            length: 2,
            name: "guard".to_owned(),
            offset: 1,
        }];
        assert!(compare_canaries(&regions, &[0, 0, 0], &[0, 0xa5, 0xa5]).is_err());
        assert!(compare_canaries(&regions, &[0, 0xa5, 0xa5], &[0, 0xa5]).is_err());
    }

    #[test]
    fn canary_layout_rejects_overlap_and_unknown_fields() {
        let overlap = json!({
            "format": "FERRIC-M1-ADVERSARIAL-CANARY-LAYOUT-V1",
            "regions": [
                {"expected_byte": 1, "length": 2, "name": "first", "offset": 0},
                {"expected_byte": 2, "length": 2, "name": "second", "offset": 1},
            ],
        });
        assert!(parse_canary_layout(&overlap).is_err());
        let unknown = json!({
            "format": "FERRIC-M1-ADVERSARIAL-CANARY-LAYOUT-V1",
            "regions": [{
                "expected_byte": 1,
                "extra": true,
                "length": 1,
                "name": "guard",
                "offset": 0,
            }],
        });
        assert!(parse_canary_layout(&unknown).is_err());
    }
}
