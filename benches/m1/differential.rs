#![forbid(unsafe_code)]

//! Target-only differential run-plan, comparison, and record-ingestion boundary.

use ferric_m1_benchmarks::{
    encode_canonical_document, load_benchmark_plan, load_canonical_document, main_for,
    sha256_identity, BenchResult, Metric, SecureFileIdentity, SecureInputDirectory,
    SecureInputFile, Suite,
};
use rustix::fd::OwnedFd;
use rustix::fs::{
    fsync, mkdirat, openat2, renameat_with, unlinkat, AtFlags, Mode, OFlags, RenameFlags,
    ResolveFlags, CWD,
};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsString;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;
#[allow(unused_imports)]
use vstd::prelude::*;

const PAIRS_FORMAT: &str = "FERRIC-M1-DIFFERENTIAL-PAIRS-V1";
const OUTPUT_FORMAT: &str = "FERRIC-M1-DIFFERENTIAL-OUTPUT-V1";
const RAW_RECORD_FORMAT: &str = "FERRIC-M1-DIFFERENTIAL-RAW-RECORD-V1";
const ACCEPTANCE_POLICY_FORMAT: &str = "FERRIC-M1-DIFFERENTIAL-ACCEPTANCE-POLICY-V1";
const ACCEPTANCE_RESULT_FORMAT: &str = "FERRIC-M1-DIFFERENTIAL-ACCEPTANCE-RESULT-V1";
const RECORDS_FORMAT: &str = "FERRIC-M1-BENCHMARK-RECORDS-V1";
const PAIRS_AUTHORITY: &str = "externally-collected-differential-pairs-only";
const OUTPUT_AUTHORITY: &str = "externally-collected-model-output-only";
const RAW_AUTHORITY: &str = "computed-differential-comparison-only";
const ACCEPTANCE_POLICY_AUTHORITY: &str = "externally-admitted-differential-threshold-policy-only";
const ACCEPTANCE_RESULT_AUTHORITY: &str = "checked-differential-policy-conformance-only";
const ACCEPTANCE_POLICY_IDENTITY: &str = "differential-acceptance-policy";
const DIFFERENTIAL_IDENTITIES: &[&str] = &[
    ACCEPTANCE_POLICY_IDENTITY,
    "dispatch-graph-decode-s1-c8192",
    "dispatch-graph-decode-s32-c8192",
    "dispatch-graph-decode-s8-c8192",
    "dispatch-graph-prefill-s1-t128",
    "dispatch-graph-prefill-s1-t2048",
    "dispatch-graph-prefill-s1-t512",
    "dispatch-graph-prefill-s8-t128",
    "reference-implementation",
    "reference-protocol",
];
const TARGET: &str = "gfx942:xnack-";
const VOCABULARY_SIZE: u64 = 151_936;
const BF16_BYTES: u64 = 2;
const TOKEN_BYTES: u64 = 4;
const ACCEPTANCE_POLICY_NONCLAIM: &str = "This artifact supplies plan-admitted differential thresholds only. It does not establish independent review, numerical correctness, hardware correctness, qualification authority, or close m1.r29.";
const ACCEPTANCE_RESULT_NONCLAIM: &str = "This result authenticates exact target-only differential comparisons against one plan-admitted threshold policy only. It does not establish an independently reviewed threshold, prove operator or graph refinement, establish hardware correctness, grant qualification authority, or close m1.r29.";

const METRICS: &[Metric] = &[
    Metric {
        id: "compared-logits",
        zero_allowed: false,
    },
    Metric {
        id: "compared-tokens",
        zero_allowed: false,
    },
    Metric {
        id: "maximum-logit-ulp-error",
        zero_allowed: true,
    },
    Metric {
        id: "token-mismatches",
        zero_allowed: true,
    },
];

const SUITE: Suite = Suite {
    name: "differential",
    obligation_id: "m1.r29",
    path_id: "differential-bench",
    source_path: "benches/m1/differential.rs",
    case_kinds: &[
        "decode-s1-c8192",
        "decode-s32-c8192",
        "decode-s8-c8192",
        "prefill-s1-t128",
        "prefill-s1-t2048",
        "prefill-s1-t512",
        "prefill-s8-t128",
    ],
    extra_identities: DIFFERENTIAL_IDENTITIES,
    metrics: METRICS,
    extra_record_attributes: &["ferric-output-sha256", "reference-output-sha256"],
    minimum_warmups: 0,
    minimum_recorded_samples: 1,
    nonclaim: "Structural acceptance authenticates externally collected target-only differential records only. It does not validate a logit tolerance, prove token equality, establish numerical or hardware correctness, qualify performance, or close m1.r29.",
};

#[derive(Debug)]
struct Payload {
    bytes: u64,
    input: Option<SecureInputFile>,
    path: PathBuf,
    sha256: String,
}

#[derive(Debug)]
struct Output {
    manifest_identity: SecureFileIdentity,
    manifest_sha256: String,
    logits: Payload,
    tokens: Payload,
}

struct OutputContext<'a> {
    case_id: &'a str,
    case: &'a PlanCase,
    identities: &'a BTreeMap<String, String>,
    plan_sha256: &'a str,
    runner_transcript_sha256: &'a str,
}

#[derive(Clone, Copy)]
struct PayloadExpectation<'a> {
    bytes: u64,
    sha256: &'a str,
}

#[derive(Debug)]
struct Pair {
    case_id: String,
    kind: String,
    ferric: Output,
    reference: Output,
    runner_transcript_sha256: String,
}

#[derive(Debug)]
struct PlanCase {
    input_sha256: String,
    kind: String,
    workload_sha256: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Comparison {
    compared_logits: u64,
    compared_tokens: u64,
    maximum_logit_ulp_error: u64,
    token_mismatches: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct AcceptanceThreshold {
    maximum_logit_ulp_error: u64,
    maximum_token_mismatches: u64,
}

struct CheckedReader<R> {
    inner: R,
    sha256: Sha256,
    bytes: u64,
}

trait SnapshotRead: Read {
    fn validate_snapshot(&self, description: &str) -> BenchResult<()>;
}

impl SnapshotRead for SecureInputFile {
    fn validate_snapshot(&self, description: &str) -> BenchResult<()> {
        SecureInputFile::validate_snapshot(self, description)
    }
}

#[cfg(test)]
impl<T: AsRef<[u8]>> SnapshotRead for std::io::Cursor<T> {
    fn validate_snapshot(&self, _description: &str) -> BenchResult<()> {
        Ok(())
    }
}

impl<R: SnapshotRead> CheckedReader<R> {
    fn new(inner: R) -> Self {
        Self {
            inner,
            sha256: Sha256::new(),
            bytes: 0,
        }
    }

    fn read_exact(&mut self, buffer: &mut [u8], description: &str) -> BenchResult<()> {
        if let Err(error) = self.inner.read_exact(buffer) {
            self.inner.validate_snapshot(description)?;
            return Err(format!("cannot read {description}: {error}"));
        }
        self.sha256.update(&*buffer);
        self.bytes = self
            .bytes
            .checked_add(
                u64::try_from(buffer.len())
                    .map_err(|_| format!("{description} read size does not fit u64"))?,
            )
            .ok_or_else(|| format!("{description} read length overflowed"))?;
        Ok(())
    }

    fn finish(
        mut self,
        expected_bytes: u64,
        expected_sha256: &str,
        description: &str,
    ) -> BenchResult<()> {
        let mut trailing = [0_u8; 1];
        let trailing_bytes = match self.inner.read(&mut trailing) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.inner.validate_snapshot(description)?;
                return Err(format!("cannot finish {description}: {error}"));
            }
        };
        if trailing_bytes != 0 || self.bytes != expected_bytes {
            self.inner.validate_snapshot(description)?;
            return Err(format!("{description} length drifted"));
        }
        self.inner.validate_snapshot(description)?;
        let actual = hex_digest(self.sha256.finalize().as_slice());
        if actual != expected_sha256 {
            return Err(format!("{description} SHA-256 drifted"));
        }
        Ok(())
    }
}

struct StagingBundle {
    parent: OwnedFd,
    staging: OwnedFd,
    raw: OwnedFd,
    staging_name: OsString,
    output_name: OsString,
    raw_names: Vec<OsString>,
    records_written: bool,
    armed: bool,
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
                    let raw = match open_directory_at(&staging, Path::new("raw"), "staged raw") {
                        Ok(raw) => raw,
                        Err(error) => {
                            let _ = unlinkat(&staging, "raw", AtFlags::REMOVEDIR);
                            let _ = unlinkat(&parent, staging_name.as_os_str(), AtFlags::REMOVEDIR);
                            return Err(error);
                        }
                    };
                    return Ok(Self {
                        parent,
                        staging,
                        raw,
                        staging_name,
                        output_name,
                        raw_names: Vec::new(),
                        records_written: false,
                        armed: true,
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
        write_new_at(&self.raw, Path::new(&name), bytes, "raw comparison record")
    }

    fn write_records(&mut self, bytes: &[u8]) -> BenchResult<()> {
        self.records_written = true;
        write_new_at(
            &self.staging,
            Path::new("records.json"),
            bytes,
            "benchmark records",
        )
    }

    fn publish(mut self) -> BenchResult<()> {
        fsync(&self.raw).map_err(|error| format!("cannot sync staged raw directory: {error}"))?;
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
        if self.records_written {
            let _ = unlinkat(&self.staging, "records.json", AtFlags::empty());
        }
        let _ = unlinkat(&self.staging, "raw", AtFlags::REMOVEDIR);
        let _ = unlinkat(
            &self.parent,
            self.staging_name.as_os_str(),
            AtFlags::REMOVEDIR,
        );
    }
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
    if arguments
        .first()
        .is_some_and(|command| command == "check-acceptance")
    {
        return match check_acceptance_command(&arguments) {
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
    let [command, plan_path, pairs_path, output_bundle] = arguments else {
        return Err("usage: ferric-m1-differential produce PLAN PAIRS OUTPUT-BUNDLE".to_owned());
    };
    if command != "produce" {
        return Err("differential producer command drifted".to_owned());
    }
    let plan_path = Path::new(plan_path);
    let pairs_path = Path::new(pairs_path);
    let output_bundle = Path::new(output_bundle);

    let (plan, plan_bytes) = load_benchmark_plan(&SUITE, plan_path)?;
    let plan_sha256 = sha256_identity(&plan_bytes);
    let plan_cases = exact_plan_cases(&plan)?;
    let identities = plan_identities(&plan)?;
    let (input_root, pairs_value, pairs_bytes) =
        load_canonical_document(pairs_path, "differential pairs manifest")?;
    let pairs = parse_pairs(
        &input_root,
        &pairs_value,
        &plan_cases,
        &identities,
        &plan_sha256,
    )?;
    let pairs_sha256 = sha256_identity(&pairs_bytes);
    let mut staging = StagingBundle::create(output_bundle)?;

    let mut observations = Vec::with_capacity(pairs.len());
    for mut pair in pairs {
        let comparison = compare_pair(&mut pair)?;
        let raw = raw_record(&pair, comparison, &plan_sha256, &pairs_sha256)?;
        let raw_bytes = encode_canonical_document(&raw)?;
        let raw_name = format!("{}.differential.raw.json", pair.case_id);
        staging.write_raw(&raw_name, &raw_bytes)?;
        observations.push(observation(&pair, comparison, &raw_bytes));
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

fn check_acceptance_command(arguments: &[OsString]) -> BenchResult<()> {
    let [command, plan_path, pairs_path, policy_path] = arguments else {
        return Err("usage: ferric-m1-differential check-acceptance PLAN PAIRS POLICY".to_owned());
    };
    if command != "check-acceptance" {
        return Err("differential acceptance command drifted".to_owned());
    }
    let (plan, plan_bytes) = load_benchmark_plan(&SUITE, Path::new(plan_path))?;
    let plan_sha256 = sha256_identity(&plan_bytes);
    let plan_cases = exact_plan_cases(&plan)?;
    let identities = plan_identities(&plan)?;
    let (_, policy_value, policy_bytes) =
        load_canonical_document(Path::new(policy_path), "differential acceptance policy")?;
    let policy_sha256 = sha256_identity(&policy_bytes);
    if policy_sha256 != identity(&identities, ACCEPTANCE_POLICY_IDENTITY)? {
        return Err("differential acceptance policy was not admitted by the plan".to_owned());
    }
    let thresholds = parse_acceptance_policy(&policy_value)?;
    let (input_root, pairs_value, pairs_bytes) =
        load_canonical_document(Path::new(pairs_path), "differential pairs manifest")?;
    let mut pairs = parse_pairs(
        &input_root,
        &pairs_value,
        &plan_cases,
        &identities,
        &plan_sha256,
    )?;
    let pairs_sha256 = sha256_identity(&pairs_bytes);
    let mut cases = Vec::with_capacity(pairs.len());
    for pair in &mut pairs {
        let comparison = compare_pair(pair)?;
        let threshold = thresholds
            .get(&pair.kind)
            .ok_or_else(|| format!("acceptance policy omitted case kind: {}", pair.kind))?;
        require_comparison_within_policy(&pair.case_id, comparison, *threshold)?;
        cases.push(acceptance_case_record(pair, comparison, *threshold));
    }
    let result = json!({
        "authority": ACCEPTANCE_RESULT_AUTHORITY,
        "cases": cases,
        "format": ACCEPTANCE_RESULT_FORMAT,
        "nonclaim": ACCEPTANCE_RESULT_NONCLAIM,
        "obligation_id": SUITE.obligation_id,
        "pairs_sha256": pairs_sha256,
        "path_id": SUITE.path_id,
        "plan_sha256": plan_sha256,
        "policy_sha256": policy_sha256,
        "status": "POLICY_CONFORMING",
        "suite": SUITE.name,
        "target": TARGET,
    });
    let bytes = encode_canonical_document(&result)?;
    let mut stdout = std::io::stdout().lock();
    stdout
        .write_all(&bytes)
        .map_err(|error| format!("cannot write differential acceptance result: {error}"))?;
    stdout
        .flush()
        .map_err(|error| format!("cannot flush differential acceptance result: {error}"))
}

fn parse_acceptance_policy(value: &Value) -> BenchResult<BTreeMap<String, AcceptanceThreshold>> {
    let policy = object(
        value,
        &[
            "authority",
            "cases",
            "finite_logits_required",
            "format",
            "logit_metric",
            "nonclaim",
            "obligation_id",
            "path_id",
            "suite",
            "target",
            "token_metric",
            "token_selection",
        ],
        "differential acceptance policy",
    )?;
    expect(
        policy,
        "authority",
        ACCEPTANCE_POLICY_AUTHORITY,
        "acceptance policy authority",
    )?;
    expect(
        policy,
        "format",
        ACCEPTANCE_POLICY_FORMAT,
        "acceptance policy format",
    )?;
    expect(
        policy,
        "logit_metric",
        "maximum-monotonic-bf16-ulp-distance-signed-zero-equal",
        "acceptance policy logit metric",
    )?;
    expect(
        policy,
        "nonclaim",
        ACCEPTANCE_POLICY_NONCLAIM,
        "acceptance policy nonclaim",
    )?;
    expect(
        policy,
        "obligation_id",
        SUITE.obligation_id,
        "acceptance policy obligation",
    )?;
    expect(policy, "path_id", SUITE.path_id, "acceptance policy path")?;
    expect(policy, "suite", SUITE.name, "acceptance policy suite")?;
    expect(policy, "target", TARGET, "acceptance policy target")?;
    expect(
        policy,
        "token_metric",
        "ferric-reference-greedy-token-mismatch-count",
        "acceptance policy token metric",
    )?;
    expect(
        policy,
        "token_selection",
        "lowest-token-id-bf16-argmax",
        "acceptance policy token selection",
    )?;
    if field(
        policy,
        "finite_logits_required",
        "differential acceptance policy",
    )?
    .as_bool()
        != Some(true)
    {
        return Err("differential acceptance policy must require finite logits".to_owned());
    }
    let cases = field(policy, "cases", "differential acceptance policy")?
        .as_array()
        .ok_or_else(|| "differential acceptance policy cases must be an array".to_owned())?;
    if cases.len() != SUITE.case_kinds.len() {
        return Err(
            "differential acceptance policy must cover exactly seven case kinds".to_owned(),
        );
    }
    let mut thresholds = BTreeMap::new();
    let mut prior: Option<&str> = None;
    for case in cases {
        let case = object(
            case,
            &[
                "kind",
                "maximum_logit_ulp_error",
                "maximum_token_mismatches",
            ],
            "differential acceptance policy case",
        )?;
        let kind = string(case, "kind", "differential acceptance policy case")?;
        if prior.is_some_and(|previous| previous >= kind) {
            return Err(
                "differential acceptance policy cases must be uniquely sorted by kind".to_owned(),
            );
        }
        prior = Some(kind);
        if !SUITE.case_kinds.contains(&kind) {
            return Err(format!(
                "differential acceptance policy has unknown kind: {kind}"
            ));
        }
        let maximum_logit_ulp_error = field(
            case,
            "maximum_logit_ulp_error",
            "differential acceptance policy case",
        )?
        .as_u64()
        .ok_or_else(|| "maximum logit ULP threshold must be an unsigned integer".to_owned())?;
        let maximum_token_mismatches = field(
            case,
            "maximum_token_mismatches",
            "differential acceptance policy case",
        )?
        .as_u64()
        .ok_or_else(|| "maximum token mismatch threshold must be an unsigned integer".to_owned())?;
        if maximum_token_mismatches > rows_for_kind(kind)? {
            return Err(format!(
                "token mismatch threshold exceeds the row count for {kind}"
            ));
        }
        thresholds.insert(
            kind.to_owned(),
            AcceptanceThreshold {
                maximum_logit_ulp_error,
                maximum_token_mismatches,
            },
        );
    }
    if thresholds
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>()
        != SUITE.case_kinds.iter().copied().collect()
    {
        return Err("differential acceptance policy case-kind roster drifted".to_owned());
    }
    Ok(thresholds)
}

fn require_comparison_within_policy(
    case_id: &str,
    comparison: Comparison,
    threshold: AcceptanceThreshold,
) -> BenchResult<()> {
    if comparison.maximum_logit_ulp_error > threshold.maximum_logit_ulp_error {
        return Err(format!(
            "maximum logit ULP error exceeds the admitted policy for {case_id}"
        ));
    }
    if comparison.token_mismatches > threshold.maximum_token_mismatches {
        return Err(format!(
            "token mismatches exceed the admitted policy for {case_id}"
        ));
    }
    Ok(())
}

fn acceptance_case_record(
    pair: &Pair,
    comparison: Comparison,
    threshold: AcceptanceThreshold,
) -> Value {
    json!({
        "case_id": pair.case_id,
        "comparison": {
            "compared_logits": comparison.compared_logits,
            "compared_tokens": comparison.compared_tokens,
            "maximum_logit_ulp_error": comparison.maximum_logit_ulp_error,
            "token_mismatches": comparison.token_mismatches,
        },
        "ferric_output": output_record(&pair.ferric),
        "kind": pair.kind,
        "reference_output": output_record(&pair.reference),
        "runner_transcript_sha256": pair.runner_transcript_sha256,
        "status": "within-policy",
        "threshold": {
            "maximum_logit_ulp_error": threshold.maximum_logit_ulp_error,
            "maximum_token_mismatches": threshold.maximum_token_mismatches,
        },
    })
}

fn exact_plan_cases(plan: &Value) -> BenchResult<BTreeMap<String, PlanCase>> {
    let plan_object = object(
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
    let cases = field(plan_object, "cases", "benchmark plan")?
        .as_array()
        .ok_or_else(|| "benchmark plan cases must be an array".to_owned())?;
    if cases.len() != SUITE.case_kinds.len() {
        return Err(
            "differential producer requires exactly one case for each of seven kinds".to_owned(),
        );
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
        return Err("differential producer case-kind roster drifted".to_owned());
    }
    Ok(by_id)
}

fn plan_identities(plan: &Value) -> BenchResult<BTreeMap<String, String>> {
    let plan = plan
        .as_object()
        .ok_or_else(|| "benchmark plan must be an object".to_owned())?;
    let identities = field(plan, "identities", "benchmark plan")?
        .as_object()
        .ok_or_else(|| "benchmark identities must be an object".to_owned())?;
    identities
        .iter()
        .map(|(name, identity)| {
            identity
                .as_str()
                .map(|identity| (name.clone(), identity.to_owned()))
                .ok_or_else(|| format!("benchmark identity must be a string: {name}"))
        })
        .collect()
}

fn parse_pairs(
    root: &SecureInputDirectory,
    value: &Value,
    plan_cases: &BTreeMap<String, PlanCase>,
    identities: &BTreeMap<String, String>,
    plan_sha256: &str,
) -> BenchResult<Vec<Pair>> {
    let document = object(
        value,
        &["authority", "format", "pairs", "plan_sha256", "suite"],
        "differential pairs manifest",
    )?;
    expect(document, "authority", PAIRS_AUTHORITY, "pairs authority")?;
    expect(document, "format", PAIRS_FORMAT, "pairs format")?;
    expect(document, "plan_sha256", plan_sha256, "pairs plan identity")?;
    expect(document, "suite", SUITE.name, "pairs suite")?;
    let values = field(document, "pairs", "differential pairs manifest")?
        .as_array()
        .ok_or_else(|| "differential pairs must be an array".to_owned())?;
    if values.len() != plan_cases.len() {
        return Err("differential pairs do not cover the exact seven-case plan".to_owned());
    }
    let mut prior: Option<&str> = None;
    let mut seen = BTreeSet::new();
    let mut manifest_paths = BTreeSet::new();
    let mut input_identities = BTreeSet::new();
    let mut pairs = Vec::with_capacity(values.len());
    for value in values {
        let pair = object(
            value,
            &[
                "case_id",
                "ferric_output_manifest",
                "kind",
                "reference_output_manifest",
                "runner_transcript",
            ],
            "differential pair",
        )?;
        let case_id = string(pair, "case_id", "differential pair")?;
        if prior.is_some_and(|previous| previous >= case_id) {
            return Err("differential pairs must be uniquely sorted by case id".to_owned());
        }
        prior = Some(case_id);
        let kind = string(pair, "kind", "differential pair")?;
        let plan_case = plan_cases
            .get(case_id)
            .ok_or_else(|| format!("differential pair names an unknown case: {case_id}"))?;
        if plan_case.kind != kind {
            return Err(format!("differential pair drifted from plan: {case_id}"));
        }
        let (runner_transcript_sha256, runner_identity) = parse_companion(
            root,
            field(pair, "runner_transcript", "differential pair")?,
            "runner transcript",
        )?;
        admit_input_identity(&mut input_identities, runner_identity, "runner transcript")?;
        let ferric_path = relative_path(
            Path::new(""),
            string(pair, "ferric_output_manifest", "differential pair")?,
            "Ferric output manifest",
        )?;
        let reference_path = relative_path(
            Path::new(""),
            string(pair, "reference_output_manifest", "differential pair")?,
            "reference output manifest",
        )?;
        if !manifest_paths.insert(ferric_path.clone())
            || !manifest_paths.insert(reference_path.clone())
        {
            return Err("differential output manifest was reused".to_owned());
        }
        let output_context = OutputContext {
            case_id,
            case: plan_case,
            identities,
            plan_sha256,
            runner_transcript_sha256: &runner_transcript_sha256,
        };
        let ferric = parse_output(root, &ferric_path, "ferric", &output_context)?;
        let reference = parse_output(root, &reference_path, "reference", &output_context)?;
        for (identity, description) in [
            (ferric.manifest_identity, "Ferric output manifest"),
            (
                ferric
                    .logits
                    .input
                    .as_ref()
                    .ok_or_else(|| "Ferric logit payload was not opened".to_owned())?
                    .identity(),
                "Ferric logit payload",
            ),
            (
                ferric
                    .tokens
                    .input
                    .as_ref()
                    .ok_or_else(|| "Ferric token payload was not opened".to_owned())?
                    .identity(),
                "Ferric token payload",
            ),
            (reference.manifest_identity, "reference output manifest"),
            (
                reference
                    .logits
                    .input
                    .as_ref()
                    .ok_or_else(|| "reference logit payload was not opened".to_owned())?
                    .identity(),
                "reference logit payload",
            ),
            (
                reference
                    .tokens
                    .input
                    .as_ref()
                    .ok_or_else(|| "reference token payload was not opened".to_owned())?
                    .identity(),
                "reference token payload",
            ),
        ] {
            admit_input_identity(&mut input_identities, identity, description)?;
        }
        seen.insert(case_id);
        pairs.push(Pair {
            case_id: case_id.to_owned(),
            kind: kind.to_owned(),
            ferric,
            reference,
            runner_transcript_sha256,
        });
    }
    if seen != plan_cases.keys().map(String::as_str).collect() {
        return Err("differential pair roster drifted from the plan".to_owned());
    }
    Ok(pairs)
}

fn parse_companion(
    root: &SecureInputDirectory,
    value: &Value,
    description: &str,
) -> BenchResult<(String, SecureFileIdentity)> {
    let companion = object(value, &["bytes", "path", "sha256"], description)?;
    let bytes = field(companion, "bytes", description)?
        .as_u64()
        .ok_or_else(|| format!("{description} length must be an unsigned integer"))?;
    if bytes == 0 {
        return Err(format!("{description} must not be empty"));
    }
    let expected_sha256 = string(companion, "sha256", description)?;
    require_sha256(expected_sha256, &format!("{description} identity"))?;
    let path = relative_path(
        Path::new(""),
        string(companion, "path", description)?,
        description,
    )?;
    let (_, actual, identity) = root.read_canonical(&path, description)?;
    if u64::try_from(actual.len()) != Ok(bytes) {
        return Err(format!("{description} length drifted"));
    }
    if sha256_identity(&actual) != expected_sha256 {
        return Err(format!("{description} SHA-256 drifted"));
    }
    Ok((expected_sha256.to_owned(), identity))
}

fn parse_output(
    root: &SecureInputDirectory,
    path: &Path,
    producer: &str,
    context: &OutputContext<'_>,
) -> BenchResult<Output> {
    let description = format!("{producer} output manifest");
    let (value, bytes, manifest_identity) = root.read_canonical(path, &description)?;
    let output = object(
        &value,
        &[
            "authority",
            "case_id",
            "environment_sha256",
            "format",
            "input_sha256",
            "kind",
            "logits",
            "plan_sha256",
            "producer",
            "producer_sha256",
            "protocol_sha256",
            "runner_transcript_sha256",
            "shape",
            "tokens",
            "workload_sha256",
        ],
        &description,
    )?;
    expect(output, "authority", OUTPUT_AUTHORITY, "output authority")?;
    expect(output, "case_id", context.case_id, "output case id")?;
    expect(
        output,
        "environment_sha256",
        identity(context.identities, "environment")?,
        "output environment identity",
    )?;
    expect(output, "format", OUTPUT_FORMAT, "output format")?;
    expect(
        output,
        "input_sha256",
        &context.case.input_sha256,
        "output input identity",
    )?;
    expect(output, "kind", &context.case.kind, "output kind")?;
    expect(
        output,
        "plan_sha256",
        context.plan_sha256,
        "output plan identity",
    )?;
    expect(output, "producer", producer, "output producer")?;

    let (producer_identity, protocol_identity) = if producer == "ferric" {
        ("benchmark-executable", "benchmark-protocol")
    } else {
        ("reference-implementation", "reference-protocol")
    };
    expect(
        output,
        "producer_sha256",
        identity(context.identities, producer_identity)?,
        "output producer identity",
    )?;
    expect(
        output,
        "protocol_sha256",
        identity(context.identities, protocol_identity)?,
        "output protocol identity",
    )?;
    expect(
        output,
        "runner_transcript_sha256",
        context.runner_transcript_sha256,
        "output runner transcript identity",
    )?;
    expect(
        output,
        "workload_sha256",
        &context.case.workload_sha256,
        "output workload identity",
    )?;

    let rows = rows_for_kind(&context.case.kind)?;
    let shape = object(
        field(output, "shape", &description)?,
        &["rows", "vocabulary_size"],
        "differential output shape",
    )?;
    expect_u64(shape, "rows", rows, "output row count")?;
    expect_u64(
        shape,
        "vocabulary_size",
        VOCABULARY_SIZE,
        "output vocabulary size",
    )?;
    let logits_bytes = rows
        .checked_mul(VOCABULARY_SIZE)
        .and_then(|elements| elements.checked_mul(BF16_BYTES))
        .ok_or_else(|| "logit payload length overflowed".to_owned())?;
    let token_bytes = rows
        .checked_mul(TOKEN_BYTES)
        .ok_or_else(|| "token payload length overflowed".to_owned())?;
    let base = path.parent().unwrap_or_else(|| Path::new("."));
    let logits = parse_payload(
        root,
        field(output, "logits", &description)?,
        base,
        "bf16-le",
        logits_bytes,
        "logit payload",
    )?;
    let tokens = parse_payload(
        root,
        field(output, "tokens", &description)?,
        base,
        "u32-le",
        token_bytes,
        "token payload",
    )?;
    if logits.path == tokens.path {
        return Err(format!("{producer} output reuses one payload path"));
    }
    Ok(Output {
        manifest_identity,
        manifest_sha256: sha256_identity(&bytes),
        logits,
        tokens,
    })
}

fn parse_payload(
    root: &SecureInputDirectory,
    value: &Value,
    base: &Path,
    encoding: &str,
    expected_bytes: u64,
    description: &str,
) -> BenchResult<Payload> {
    let payload = object(value, &["bytes", "encoding", "path", "sha256"], description)?;
    expect(payload, "encoding", encoding, "payload encoding")?;
    expect_u64(payload, "bytes", expected_bytes, "payload length")?;
    let sha256 = string(payload, "sha256", description)?;
    require_sha256(sha256, "payload identity")?;
    let path = relative_path(base, string(payload, "path", description)?, description)?;
    let input = root.open_exact(&path, expected_bytes, description)?;
    Ok(Payload {
        bytes: expected_bytes,
        input: Some(input),
        path,
        sha256: sha256.to_owned(),
    })
}

fn compare_pair(pair: &mut Pair) -> BenchResult<Comparison> {
    let rows = rows_for_kind(&pair.kind)?;
    let ferric_logits_bytes = pair.ferric.logits.bytes;
    let ferric_logits_sha256 = pair.ferric.logits.sha256.clone();
    let reference_logits_bytes = pair.reference.logits.bytes;
    let reference_logits_sha256 = pair.reference.logits.sha256.clone();
    let ferric_expected = PayloadExpectation {
        bytes: ferric_logits_bytes,
        sha256: &ferric_logits_sha256,
    };
    let reference_expected = PayloadExpectation {
        bytes: reference_logits_bytes,
        sha256: &reference_logits_sha256,
    };
    let (maximum_logit_ulp_error, ferric_argmax, reference_argmax) = compare_logits(
        CheckedReader::new(
            pair.ferric
                .logits
                .input
                .take()
                .ok_or_else(|| "Ferric logit payload was already consumed".to_owned())?,
        ),
        CheckedReader::new(
            pair.reference
                .logits
                .input
                .take()
                .ok_or_else(|| "reference logit payload was already consumed".to_owned())?,
        ),
        rows,
        VOCABULARY_SIZE,
        ferric_expected,
        reference_expected,
    )?;
    let ferric_token_bytes = pair.ferric.tokens.bytes;
    let ferric_token_sha256 = pair.ferric.tokens.sha256.clone();
    let ferric_tokens = read_tokens(
        CheckedReader::new(
            pair.ferric
                .tokens
                .input
                .take()
                .ok_or_else(|| "Ferric token payload was already consumed".to_owned())?,
        ),
        ferric_token_bytes,
        &ferric_token_sha256,
        rows,
        "Ferric token payload",
    )?;
    let reference_token_bytes = pair.reference.tokens.bytes;
    let reference_token_sha256 = pair.reference.tokens.sha256.clone();
    let reference_tokens = read_tokens(
        CheckedReader::new(
            pair.reference
                .tokens
                .input
                .take()
                .ok_or_else(|| "reference token payload was already consumed".to_owned())?,
        ),
        reference_token_bytes,
        &reference_token_sha256,
        rows,
        "reference token payload",
    )?;
    validate_argmax(&ferric_tokens, &ferric_argmax, "Ferric", &pair.case_id)?;
    validate_argmax(
        &reference_tokens,
        &reference_argmax,
        "reference",
        &pair.case_id,
    )?;
    let token_mismatches = ferric_tokens
        .iter()
        .zip(&reference_tokens)
        .filter(|(ferric, reference)| ferric != reference)
        .count();
    Ok(Comparison {
        compared_logits: rows
            .checked_mul(VOCABULARY_SIZE)
            .ok_or_else(|| "compared logit count overflowed".to_owned())?,
        compared_tokens: rows,
        maximum_logit_ulp_error,
        token_mismatches: u64::try_from(token_mismatches)
            .map_err(|_| "token mismatch count does not fit u64".to_owned())?,
    })
}

fn compare_logits<FR: SnapshotRead, RR: SnapshotRead>(
    mut ferric: CheckedReader<FR>,
    mut reference: CheckedReader<RR>,
    rows: u64,
    vocabulary: u64,
    ferric_expected: PayloadExpectation<'_>,
    reference_expected: PayloadExpectation<'_>,
) -> BenchResult<(u64, Vec<u32>, Vec<u32>)> {
    let row_capacity = usize::try_from(rows)
        .map_err(|_| "differential row count does not fit this host".to_owned())?;
    let mut ferric_argmax = Vec::with_capacity(row_capacity);
    let mut reference_argmax = Vec::with_capacity(row_capacity);
    let mut maximum_ulp = 0_u64;
    for row in 0..rows {
        let mut ferric_best = None;
        let mut reference_best = None;
        for token in 0..vocabulary {
            let ferric_bits = read_bf16(&mut ferric, "Ferric logit payload")?;
            let reference_bits = read_bf16(&mut reference, "reference logit payload")?;
            require_finite(ferric_bits, "Ferric", row, token)?;
            require_finite(reference_bits, "reference", row, token)?;
            maximum_ulp = maximum_ulp.max(u64::from(bf16_ulp(ferric_bits, reference_bits)));
            update_argmax(&mut ferric_best, ferric_bits, token)?;
            update_argmax(&mut reference_best, reference_bits, token)?;
        }
        ferric_argmax.push(
            ferric_best
                .map(|(_, token)| token)
                .ok_or_else(|| "Ferric logit row was empty".to_owned())?,
        );
        reference_argmax.push(
            reference_best
                .map(|(_, token)| token)
                .ok_or_else(|| "reference logit row was empty".to_owned())?,
        );
    }
    ferric.finish(
        ferric_expected.bytes,
        ferric_expected.sha256,
        "Ferric logit payload",
    )?;
    reference.finish(
        reference_expected.bytes,
        reference_expected.sha256,
        "reference logit payload",
    )?;
    Ok((maximum_ulp, ferric_argmax, reference_argmax))
}

fn read_tokens<R: SnapshotRead>(
    mut reader: CheckedReader<R>,
    expected_bytes: u64,
    expected_sha256: &str,
    rows: u64,
    description: &str,
) -> BenchResult<Vec<u32>> {
    let capacity = usize::try_from(rows)
        .map_err(|_| "differential row count does not fit this host".to_owned())?;
    let mut tokens = Vec::with_capacity(capacity);
    for _ in 0..rows {
        let mut bytes = [0_u8; 4];
        reader.read_exact(&mut bytes, description)?;
        tokens.push(u32::from_le_bytes(bytes));
    }
    reader.finish(expected_bytes, expected_sha256, description)?;
    Ok(tokens)
}

fn read_bf16<R: SnapshotRead>(
    reader: &mut CheckedReader<R>,
    description: &str,
) -> BenchResult<u16> {
    let mut bytes = [0_u8; 2];
    reader.read_exact(&mut bytes, description)?;
    Ok(u16::from_le_bytes(bytes))
}

fn require_finite(bits: u16, producer: &str, row: u64, token: u64) -> BenchResult<()> {
    if bits & 0x7f80 == 0x7f80 {
        return Err(format!(
            "{producer} output contains a nonfinite BF16 logit at row {row}, token {token}"
        ));
    }
    Ok(())
}

fn update_argmax(best: &mut Option<(f32, u32)>, bits: u16, token: u64) -> BenchResult<()> {
    let token = u32::try_from(token).map_err(|_| "token id does not fit u32".to_owned())?;
    let value = f32::from_bits(u32::from(bits) << 16);
    if best.is_none_or(|(prior, _)| value > prior) {
        *best = Some((value, token));
    }
    Ok(())
}

fn bf16_ulp(left: u16, right: u16) -> u32 {
    ordered_bf16(left).abs_diff(ordered_bf16(right))
}

fn ordered_bf16(bits: u16) -> u32 {
    let magnitude = u32::from(bits & 0x7fff);
    if bits & 0x8000 == 0 {
        0x8000 + magnitude
    } else {
        0x8000 - magnitude
    }
}

fn validate_argmax(
    tokens: &[u32],
    argmax: &[u32],
    producer: &str,
    case_id: &str,
) -> BenchResult<()> {
    for (row, (actual, expected)) in tokens.iter().zip(argmax).enumerate() {
        if actual != expected {
            return Err(format!(
                "{producer} token is not the lowest-ID BF16 argmax for {case_id} row {row}"
            ));
        }
    }
    Ok(())
}

fn raw_record(
    pair: &Pair,
    comparison: Comparison,
    plan_sha256: &str,
    pairs_sha256: &str,
) -> BenchResult<Value> {
    Ok(json!({
        "authority": RAW_AUTHORITY,
        "case_id": pair.case_id,
        "comparison": {
            "compared_logits": comparison.compared_logits,
            "compared_tokens": comparison.compared_tokens,
            "maximum_logit_ulp_error": comparison.maximum_logit_ulp_error,
            "token_mismatches": comparison.token_mismatches,
        },
        "ferric_output": output_record(&pair.ferric),
        "format": RAW_RECORD_FORMAT,
        "kind": pair.kind,
        "nonclaim": SUITE.nonclaim,
        "pairs_sha256": pairs_sha256,
        "plan_sha256": plan_sha256,
        "reference_output": output_record(&pair.reference),
        "runner_transcript_sha256": pair.runner_transcript_sha256,
        "shape": {
            "rows": rows_for_kind(&pair.kind)?,
            "vocabulary_size": VOCABULARY_SIZE,
        },
        "status": "compared",
    }))
}

fn output_record(output: &Output) -> Value {
    json!({
        "logits_bytes": output.logits.bytes,
        "logits_sha256": output.logits.sha256,
        "manifest_sha256": output.manifest_sha256,
        "tokens_bytes": output.tokens.bytes,
        "tokens_sha256": output.tokens.sha256,
    })
}

fn observation(pair: &Pair, comparison: Comparison, raw_bytes: &[u8]) -> Value {
    json!({
        "attributes": {
            "ferric-output-sha256": pair.ferric.manifest_sha256,
            "raw-record-sha256": sha256_identity(raw_bytes),
            "reference-output-sha256": pair.reference.manifest_sha256,
            "runner-transcript-sha256": pair.runner_transcript_sha256,
        },
        "case_id": pair.case_id,
        "kind": pair.kind,
        "measurements": {
            "compared-logits": [comparison.compared_logits],
            "compared-tokens": [comparison.compared_tokens],
            "maximum-logit-ulp-error": [comparison.maximum_logit_ulp_error],
            "token-mismatches": [comparison.token_mismatches],
        },
        "recorded_samples": 1,
        "status": "completed",
        "warmups": 0,
    })
}

fn rows_for_kind(kind: &str) -> BenchResult<u64> {
    match kind {
        "decode-s1-c8192" | "prefill-s1-t128" | "prefill-s1-t2048" | "prefill-s1-t512" => Ok(1),
        "decode-s8-c8192" | "prefill-s8-t128" => Ok(8),
        "decode-s32-c8192" => Ok(32),
        _ => Err(format!("unknown differential case kind: {kind}")),
    }
}

fn relative_path(base: &Path, value: &str, description: &str) -> BenchResult<PathBuf> {
    if value.is_empty() || value.len() > 1024 || !value.is_ascii() {
        return Err(format!("invalid {description} path"));
    }
    let relative = Path::new(value);
    if relative.is_absolute()
        || !relative
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(format!("{description} path must be a safe relative path"));
    }
    Ok(base.join(relative))
}

fn admit_input_identity(
    identities: &mut BTreeSet<SecureFileIdentity>,
    identity: SecureFileIdentity,
    description: &str,
) -> BenchResult<()> {
    if !identities.insert(identity) {
        return Err(format!("{description} aliases another differential input"));
    }
    Ok(())
}

fn object<'a>(
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

fn string<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    description: &str,
) -> BenchResult<&'a str> {
    field(object, key, description)?
        .as_str()
        .ok_or_else(|| format!("{description} field must be a string: {key}"))
}

fn expect(
    object: &Map<String, Value>,
    key: &str,
    expected: &str,
    description: &str,
) -> BenchResult<()> {
    if string(object, key, description)? != expected {
        return Err(format!("{description} drifted"));
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
        return Err(format!("{description} drifted"));
    }
    Ok(())
}

fn identity<'a>(identities: &'a BTreeMap<String, String>, name: &str) -> BenchResult<&'a str> {
    identities
        .get(name)
        .map(String::as_str)
        .ok_or_else(|| format!("benchmark plan is missing identity: {name}"))
}

fn require_sha256(value: &str, description: &str) -> BenchResult<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || value.bytes().all(|byte| byte == value.as_bytes()[0])
    {
        return Err(format!("invalid {description}"));
    }
    Ok(())
}

fn hex_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Cursor;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_NONCE: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let nonce = TEST_NONCE.fetch_add(1, Ordering::Relaxed);
            let path = env::temp_dir().join(format!(
                "ferric-m1-differential-test.{}.{nonce}",
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

    fn bf16(values: &[u16]) -> Vec<u8> {
        values
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect()
    }

    fn payload(bytes: &[u8]) -> Payload {
        Payload {
            bytes: u64::try_from(bytes.len()).unwrap(),
            input: None,
            path: PathBuf::new(),
            sha256: sha256_identity(bytes),
        }
    }

    fn acceptance_policy_fixture() -> Value {
        json!({
            "authority": ACCEPTANCE_POLICY_AUTHORITY,
            "cases": SUITE.case_kinds.iter().map(|kind| json!({
                "kind": kind,
                "maximum_logit_ulp_error": 0,
                "maximum_token_mismatches": 0,
            })).collect::<Vec<_>>(),
            "finite_logits_required": true,
            "format": ACCEPTANCE_POLICY_FORMAT,
            "logit_metric": "maximum-monotonic-bf16-ulp-distance-signed-zero-equal",
            "nonclaim": ACCEPTANCE_POLICY_NONCLAIM,
            "obligation_id": SUITE.obligation_id,
            "path_id": SUITE.path_id,
            "suite": SUITE.name,
            "target": TARGET,
            "token_metric": "ferric-reference-greedy-token-mismatch-count",
            "token_selection": "lowest-token-id-bf16-argmax",
        })
    }

    fn compare(
        ferric: &[u16],
        reference: &[u16],
        rows: u64,
        vocabulary: u64,
    ) -> BenchResult<(u64, Vec<u32>, Vec<u32>)> {
        let ferric = bf16(ferric);
        let reference = bf16(reference);
        let ferric_payload = payload(&ferric);
        let reference_payload = payload(&reference);
        compare_logits(
            CheckedReader::new(Cursor::new(&ferric)),
            CheckedReader::new(Cursor::new(&reference)),
            rows,
            vocabulary,
            PayloadExpectation {
                bytes: ferric_payload.bytes,
                sha256: &ferric_payload.sha256,
            },
            PayloadExpectation {
                bytes: reference_payload.bytes,
                sha256: &reference_payload.sha256,
            },
        )
    }

    #[test]
    fn streaming_comparison_uses_monotonic_bf16_ulp_and_signed_zero_equality() {
        let (maximum, ferric, reference) = compare(
            &[0xbf80, 0x8000, 0x3f80, 0x4000],
            &[0xbf81, 0x0000, 0x3f81, 0x4000],
            1,
            4,
        )
        .unwrap();
        assert_eq!(maximum, 1);
        assert_eq!(ferric, vec![3]);
        assert_eq!(reference, vec![3]);
        assert_eq!(bf16_ulp(0x8000, 0x0000), 0);
        assert_eq!(bf16_ulp(0x8001, 0x0001), 2);
        assert_eq!(bf16_ulp(0xbf80, 0xbf81), 1);
    }

    #[test]
    fn lowest_id_argmax_is_required_for_each_output() {
        let (_, ferric, reference) =
            compare(&[0x3f80, 0x4000, 0x4000], &[0x4000, 0x4000, 0x3f80], 1, 3).unwrap();
        assert_eq!(ferric, vec![1]);
        assert_eq!(reference, vec![0]);
        assert!(validate_argmax(&[2], &ferric, "Ferric", "case.001").is_err());
    }

    #[test]
    fn nonfinite_truncated_and_substituted_payloads_fail_closed() {
        assert!(compare(&[0x7f80], &[0x3f80], 1, 1).is_err());
        assert!(compare(&[0x7fc1], &[0x3f80], 1, 1).is_err());
        assert!(compare(&[], &[0x3f80], 1, 1).is_err());

        let ferric = bf16(&[0x3f80]);
        let reference = bf16(&[0x3f80]);
        let reference_sha256 = sha256_identity(&reference);
        let substituted = Payload {
            bytes: 2,
            input: None,
            path: PathBuf::new(),
            sha256: sha256_identity(b"substituted"),
        };
        assert!(compare_logits(
            CheckedReader::new(Cursor::new(&ferric)),
            CheckedReader::new(Cursor::new(&reference)),
            1,
            1,
            PayloadExpectation {
                bytes: substituted.bytes,
                sha256: &substituted.sha256,
            },
            PayloadExpectation {
                bytes: 2,
                sha256: &reference_sha256,
            },
        )
        .is_err());
    }

    #[test]
    fn seven_case_roster_and_shapes_are_fixed() {
        assert_eq!(SUITE.case_kinds.len(), 7);
        assert_eq!(
            &SUITE.extra_identities[1..8],
            &[
                "dispatch-graph-decode-s1-c8192",
                "dispatch-graph-decode-s32-c8192",
                "dispatch-graph-decode-s8-c8192",
                "dispatch-graph-prefill-s1-t128",
                "dispatch-graph-prefill-s1-t2048",
                "dispatch-graph-prefill-s1-t512",
                "dispatch-graph-prefill-s8-t128",
            ]
        );
        assert_eq!(rows_for_kind("decode-s1-c8192").unwrap(), 1);
        assert_eq!(rows_for_kind("decode-s32-c8192").unwrap(), 32);
        assert_eq!(rows_for_kind("decode-s8-c8192").unwrap(), 8);
        assert_eq!(rows_for_kind("prefill-s8-t128").unwrap(), 8);
        assert!(rows_for_kind("decode-s2-c8192").is_err());
    }

    #[test]
    fn acceptance_policy_requires_exact_typed_semantics_and_roster() {
        let policy = acceptance_policy_fixture();
        let thresholds = parse_acceptance_policy(&policy).unwrap();
        assert_eq!(thresholds.len(), 7);

        let mut missing = policy.clone();
        missing["cases"].as_array_mut().unwrap().pop();
        assert!(parse_acceptance_policy(&missing).is_err());

        let mut nonfinite = policy.clone();
        nonfinite["finite_logits_required"] = Value::Bool(false);
        assert!(parse_acceptance_policy(&nonfinite).is_err());

        let mut unknown = policy.clone();
        unknown["cases"][0]["kind"] = Value::String("decode-s2-c8192".to_owned());
        assert!(parse_acceptance_policy(&unknown).is_err());

        let mut vacuous_tokens = policy;
        vacuous_tokens["cases"][0]["maximum_token_mismatches"] = Value::from(2_u64);
        assert!(parse_acceptance_policy(&vacuous_tokens).is_err());
    }

    #[test]
    fn acceptance_applies_only_explicit_logit_and_token_thresholds() {
        let comparison = Comparison {
            compared_logits: VOCABULARY_SIZE,
            compared_tokens: 1,
            maximum_logit_ulp_error: 1,
            token_mismatches: 1,
        };
        assert!(require_comparison_within_policy(
            "decode-s1-c8192.001",
            comparison,
            AcceptanceThreshold {
                maximum_logit_ulp_error: 0,
                maximum_token_mismatches: 1,
            },
        )
        .is_err());
        assert!(require_comparison_within_policy(
            "decode-s1-c8192.001",
            comparison,
            AcceptanceThreshold {
                maximum_logit_ulp_error: 1,
                maximum_token_mismatches: 0,
            },
        )
        .is_err());
        assert!(require_comparison_within_policy(
            "decode-s1-c8192.001",
            comparison,
            AcceptanceThreshold {
                maximum_logit_ulp_error: 1,
                maximum_token_mismatches: 1,
            },
        )
        .is_ok());
    }

    #[test]
    fn paths_reject_absolute_parent_and_non_ascii_components() {
        let base = Path::new("/tmp/base");
        assert!(relative_path(base, "case/output.json", "test").is_ok());
        assert!(relative_path(base, "/tmp/output.json", "test").is_err());
        assert!(relative_path(base, "../output.json", "test").is_err());
        assert!(relative_path(base, "case/\u{2603}.json", "test").is_err());
    }

    #[test]
    fn staged_bundle_failure_is_retry_safe_and_publication_is_no_replace() {
        let temporary = TestDirectory::new();
        let output = temporary.0.join("comparison.bundle");

        {
            let mut staging = StagingBundle::create(&output).unwrap();
            staging.write_raw("case.raw.json", b"raw\n").unwrap();
            assert!(staging.write_raw("case.raw.json", b"duplicate\n").is_err());
        }
        assert!(!output.exists());
        assert!(fs::read_dir(&temporary.0).unwrap().next().is_none());

        let mut staging = StagingBundle::create(&output).unwrap();
        staging.write_raw("case.raw.json", b"raw\n").unwrap();
        staging.write_records(b"records\n").unwrap();
        fs::write(&output, b"caller-owned\n").unwrap();
        assert!(staging.publish().is_err());
        assert_eq!(fs::read(&output).unwrap(), b"caller-owned\n");
        assert_eq!(fs::read_dir(&temporary.0).unwrap().count(), 1);

        fs::remove_file(&output).unwrap();
        let mut retry = StagingBundle::create(&output).unwrap();
        retry.write_raw("case.raw.json", b"raw\n").unwrap();
        retry.write_records(b"records\n").unwrap();
        retry.publish().unwrap();
        assert_eq!(
            fs::read(output.join("raw/case.raw.json")).unwrap(),
            b"raw\n"
        );
        assert_eq!(fs::read(output.join("records.json")).unwrap(), b"records\n");
        assert!(StagingBundle::create(&output).is_err());
        assert_eq!(fs::read(output.join("records.json")).unwrap(), b"records\n");
    }
}
