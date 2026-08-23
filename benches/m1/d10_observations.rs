//! Policy-bound validation of exact canonical raw D10 observations.
//!
//! This is a structural and arithmetic checker, not an observation-truth or
//! qualification authority. It retains every input descriptor through final
//! no-replace publication and recomputes every reported metric from raw rows.

use crate::d10_policy::{hold_validated_policy, HeldValidatedPolicy};
use ferric_m1_benchmarks::{encode_canonical_document, sha256_identity, BenchResult};
use num_bigint::BigUint;
use rustix::fd::OwnedFd;
use rustix::fs::{
    fstat, fsync, mkdirat, openat2, renameat_with, unlinkat, AtFlags, Dir, FileType, Mode, OFlags,
    RenameFlags, ResolveFlags, Stat, CWD,
};
use rustix::process::{getegid, geteuid};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::ffi::OsStrExt;
use std::path::{Component, Path, PathBuf};

pub(super) const COMMAND: &str = "validate-policy-observations";

const INPUT_FORMAT: &str = "FERRIC-M1-D10-POLICY-OBSERVATIONS-V1";
const OUTPUT_FORMAT: &str = "FERRIC-M1-D10-POLICY-OBSERVATION-VALIDATION-V1";
const INPUT_AUTHORITY: &str = "externally-collected-policy-bound-d10-observations-only";
const OUTPUT_AUTHORITY: &str = "checked-policy-bound-d10-observation-structure-and-arithmetic-only";
const STATUS: &str = "PARTIAL_NON_EVIDENCE";
const TARGET: &str = "gfx942:xnack-";
const WARMUPS: usize = 10;
const RECORDED: usize = 30;
const RATE_SCALE: u128 = 1_000_000_000;
const PPM_SCALE: u64 = 1_000_000;
const MAX_DOCUMENT_BYTES: usize = 8 * 1024 * 1024;
const MAX_EXACT_AGGREGATE_BITS: u64 = 8 * 1024 * 1024;
const MAX_EXACT_AGGREGATE_CASE_WEIGHT: u64 = 8_192;
const PROTOCOL_SHA256: &str = "758485a3166ebaa0723c444862e988f3c5b1b2cf3069dd4804ac1464e13eedec";
const NONCLAIM: &str = "This artifact checks exact canonical policy-bound raw D10 timing rows, order, holdout membership, tuning budgets, identity bindings, and recomputes integer throughput medians, regression gates, applicable-vendor gates, and the applicable-vendor weighted geometric aggregate. Telemetry and resource companions expose identities but no parseable raw-output schema, so this validator binds those identities without authenticating telemetry/resource output bytes or semantics. It does not validate externally supplied policy values, prove that observations or hardware telemetry are truthful, independently reproduce results, establish kernel correctness, constitute qualification evidence, or close m1.r31.";

const CASE_ROSTER: &[(&str, &str)] = &[
    ("flash-attention-prefill", "k4-gqa-prefill"),
    ("gemm-gemv", "k1-gemm-gemv"),
    ("gqa-paged-decode", "k5-paged-gqa-decode"),
    ("logits-argmax", "k7-logits-compact"),
    ("rmsnorm-residual", "k2-rmsnorm-residual"),
    ("rope-paged-kv", "k3-rope-paged-kv"),
    ("swiglu-projection", "k6-swiglu"),
];
const IMPLEMENTATIONS: &[&str] = &["ferric-reference", "ferric", "vendor"];
const COMPANIONS: &[(&str, &str)] = &[
    ("calibration", "calibration.json"),
    ("execution-order", "execution-order.json"),
    ("holdout", "holdout.json"),
    ("regression-reference", "regression-reference.json"),
    ("resource-inspection", "resource-inspection.json"),
    ("telemetry", "telemetry.json"),
    ("timing", "timing.json"),
    ("tuning", "tuning.json"),
];
const ADMISSION_FILES: &[&str] = &["admission.json", "protocol.json"];
const OBSERVATION_FILES: &[&str] = &["observations.json", "protocol.json"];
const OUTPUT_FILES: &[&str] = &["observations.json", "protocol.json", "validation.json"];

#[derive(Clone, Debug)]
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
struct HeldDocument {
    bytes: Vec<u8>,
    file: File,
    initial: Stat,
    name: &'static str,
    value: Value,
}

impl HeldDocument {
    fn open(root: &OwnedFd, name: &'static str, description: &str) -> BenchResult<Self> {
        let descriptor = openat2(
            root,
            Path::new(name),
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )
        .map_err(|error| format!("cannot securely open {description} {name}: {error}"))?;
        let initial = fstat(&descriptor)
            .map_err(|error| format!("cannot inspect {description} {name}: {error}"))?;
        if FileType::from_raw_mode(initial.st_mode) != FileType::RegularFile
            || initial.st_nlink != 1
        {
            return Err(format!(
                "{description} {name} must be a one-link regular file"
            ));
        }
        let length = usize::try_from(initial.st_size)
            .map_err(|_| format!("{description} {name} length is invalid"))?;
        if length == 0 || length > MAX_DOCUMENT_BYTES {
            return Err(format!(
                "{description} {name} length is outside the admitted bound"
            ));
        }
        let mut file = File::from(descriptor);
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length.saturating_add(1))
            .map_err(|_| format!("cannot reserve {description} {name} buffer"))?;
        Read::by_ref(&mut file)
            .take(MAX_DOCUMENT_BYTES.saturating_add(1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("cannot read {description} {name}: {error}"))?;
        let final_stat = fstat(&file)
            .map_err(|error| format!("cannot reinspect {description} {name}: {error}"))?;
        if bytes.len() != length || !same_snapshot(&initial, &final_stat) {
            return Err(format!("{description} {name} changed while being read"));
        }
        if !bytes.is_ascii() {
            return Err(format!("{description} {name} must be ASCII JSON"));
        }
        let value: Value = serde_json::from_slice(&bytes)
            .map_err(|error| format!("cannot parse {description} {name}: {error}"))?;
        if encode_canonical_document(&value)? != bytes {
            return Err(format!("{description} {name} is not canonical JSON"));
        }
        Ok(Self {
            bytes,
            file,
            initial,
            name,
            value,
        })
    }

    fn revalidate(&mut self, root: &OwnedFd, description: &str) -> BenchResult<()> {
        let held = fstat(&self.file).map_err(|error| {
            format!("cannot reinspect held {description} {}: {error}", self.name)
        })?;
        if !same_snapshot(&self.initial, &held) {
            return Err(format!("held {description} {} changed", self.name));
        }
        self.file
            .seek(SeekFrom::Start(0))
            .map_err(|error| format!("cannot rewind held {description} {}: {error}", self.name))?;
        let mut bytes = Vec::new();
        Read::by_ref(&mut self.file)
            .take(MAX_DOCUMENT_BYTES.saturating_add(1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("cannot reread held {description} {}: {error}", self.name))?;
        let reread = fstat(&self.file).map_err(|error| {
            format!(
                "cannot reinspect reread {description} {}: {error}",
                self.name
            )
        })?;
        if bytes != self.bytes || !same_snapshot(&self.initial, &reread) {
            return Err(format!("held {description} {} bytes changed", self.name));
        }
        let rebound = openat2(
            root,
            Path::new(self.name),
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )
        .map_err(|error| format!("cannot rebind {description} {}: {error}", self.name))?;
        let rebound = fstat(&rebound).map_err(|error| {
            format!(
                "cannot inspect rebound {description} {}: {error}",
                self.name
            )
        })?;
        if !same_snapshot(&self.initial, &rebound) {
            return Err(format!(
                "{description} name {} no longer binds held input",
                self.name
            ));
        }
        Ok(())
    }
}

struct HeldBundle {
    description: &'static str,
    documents: BTreeMap<&'static str, HeldDocument>,
    initial: Stat,
    root: OwnedFd,
    roster: &'static [&'static str],
}

impl HeldBundle {
    fn open(
        path: &Path,
        roster: &'static [&'static str],
        description: &'static str,
    ) -> BenchResult<Self> {
        require_safe_path(path, description)?;
        let root = openat2(
            CWD,
            path,
            OFlags::RDONLY
                | OFlags::DIRECTORY
                | OFlags::NOFOLLOW
                | OFlags::NONBLOCK
                | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )
        .map_err(|error| format!("cannot securely open {description}: {error}"))?;
        let initial =
            fstat(&root).map_err(|error| format!("cannot inspect {description}: {error}"))?;
        if FileType::from_raw_mode(initial.st_mode) != FileType::Directory {
            return Err(format!("{description} must be a directory"));
        }
        validate_roster(&root, roster, description)?;
        let mut documents = BTreeMap::new();
        let mut identities = BTreeSet::new();
        for name in roster {
            let document = HeldDocument::open(&root, name, description)?;
            if !identities.insert((document.initial.st_dev, document.initial.st_ino)) {
                return Err(format!("{description} files must not alias"));
            }
            documents.insert(*name, document);
        }
        let current =
            fstat(&root).map_err(|error| format!("cannot reinspect {description}: {error}"))?;
        if !same_snapshot(&initial, &current) {
            return Err(format!("{description} changed while opening inputs"));
        }
        Ok(Self {
            description,
            documents,
            initial,
            root,
            roster,
        })
    }

    fn document(&self, name: &str) -> BenchResult<&HeldDocument> {
        self.documents
            .get(name)
            .ok_or_else(|| format!("missing held {} {name}", self.description))
    }

    fn revalidate(&mut self) -> BenchResult<()> {
        validate_roster(&self.root, self.roster, self.description)?;
        for document in self.documents.values_mut() {
            document.revalidate(&self.root, self.description)?;
        }
        let current = fstat(&self.root)
            .map_err(|error| format!("cannot reinspect {}: {error}", self.description))?;
        if !same_snapshot(&self.initial, &current) {
            return Err(format!("{} changed after validation", self.description));
        }
        Ok(())
    }
}

#[derive(Debug)]
struct CaseMetric {
    case_id: String,
    ferric: Rational,
    reference: Rational,
    regression_gate: bool,
    regression_ppm: BigUint,
    vendor: Option<Rational>,
    vendor_gate: Option<bool>,
    vendor_ratio_ppm: Option<BigUint>,
    weight: u64,
}

/// Validates one exact raw bundle against its exact admitted policy and publishes a non-evidence result.
pub(super) fn validate_policy_observations(arguments: &[OsString]) -> BenchResult<()> {
    validate_policy_observations_with_hooks(
        arguments,
        || Ok(()),
        |_| Ok(()),
        |_| Ok(()),
        || Ok(()),
        || Ok(()),
    )
}

fn validate_policy_observations_with_hooks<F, G, H, I, J>(
    arguments: &[OsString],
    after_inputs: F,
    after_mkdir: G,
    after_staging: H,
    before_rename: I,
    after_published_verification: J,
) -> BenchResult<()>
where
    F: FnOnce() -> BenchResult<()>,
    G: FnOnce(&Path) -> BenchResult<()>,
    H: FnOnce(&Path) -> BenchResult<()>,
    I: FnOnce() -> BenchResult<()>,
    J: FnOnce() -> BenchResult<()>,
{
    let [command, policy_root, admission_bundle, observation_bundle, output] = arguments else {
        return Err("usage: ferric-m1-d10 validate-policy-observations POLICY-ROOT ADMISSION-BUNDLE OBSERVATION-BUNDLE OUTPUT-BUNDLE".to_owned());
    };
    if command != COMMAND {
        return Err("D10 observation-validation command drifted".to_owned());
    }
    let mut policy = hold_validated_policy(Path::new(policy_root))?;
    let mut admission = HeldBundle::open(
        Path::new(admission_bundle),
        ADMISSION_FILES,
        "D10 policy admission bundle",
    )?;
    let mut observations = HeldBundle::open(
        Path::new(observation_bundle),
        OBSERVATION_FILES,
        "D10 observation bundle",
    )?;
    validate_admission_binding(&policy, &admission)?;
    validate_observation_protocol(&observations)?;
    let validation = validate_observations(&policy, &admission, &observations)?;
    let validation_bytes = encode_canonical_document(&validation)?;
    let observation_bytes = observations.document("observations.json")?.bytes.clone();
    let protocol_bytes = observations.document("protocol.json")?.bytes.clone();
    after_inputs()?;
    revalidate_inputs(&mut policy, &mut admission, &mut observations)?;
    let mut bundle = ExactBundle::create_with_hook(Path::new(output), after_mkdir)?;
    bundle.write("observations.json", &observation_bytes)?;
    bundle.write("protocol.json", &protocol_bytes)?;
    bundle.write("validation.json", &validation_bytes)?;
    after_staging(&bundle.staging_path)?;
    let expected = [
        ("observations.json", observation_bytes.as_slice()),
        ("protocol.json", protocol_bytes.as_slice()),
        ("validation.json", validation_bytes.as_slice()),
    ];
    let mut revalidate = || revalidate_inputs(&mut policy, &mut admission, &mut observations);
    bundle.publish_exact(
        &expected,
        &mut revalidate,
        || {
            before_rename()?;
            Ok(())
        },
        || {
            after_published_verification()?;
            Ok(())
        },
    )
}

fn revalidate_inputs(
    policy: &mut HeldValidatedPolicy,
    admission: &mut HeldBundle,
    observations: &mut HeldBundle,
) -> BenchResult<()> {
    policy.revalidate()?;
    admission.revalidate()?;
    observations.revalidate()
}

fn validate_admission_binding(
    policy: &HeldValidatedPolicy,
    admission: &HeldBundle,
) -> BenchResult<()> {
    let admission_document = admission.document("admission.json")?;
    if admission_document.bytes != policy.admission_bytes()
        || &admission_document.value != policy.admission()
    {
        return Err(
            "D10 policy admission is not the exact admission recomputed from the held policy root"
                .to_owned(),
        );
    }
    if admission.document("protocol.json")?.bytes != policy.document_bytes("protocol.json")? {
        return Err(
            "D10 policy admission protocol is not the held policy-root protocol".to_owned(),
        );
    }
    Ok(())
}

fn validate_observation_protocol(observations: &HeldBundle) -> BenchResult<()> {
    let protocol = observations.document("protocol.json")?;
    if sha256_identity(&protocol.bytes) != PROTOCOL_SHA256 {
        return Err("D10 observation protocol was substituted".to_owned());
    }
    Ok(())
}

fn validate_observations(
    policy: &HeldValidatedPolicy,
    admission: &HeldBundle,
    observations: &HeldBundle,
) -> BenchResult<Value> {
    let document = observations.document("observations.json")?;
    let root = exact_object(
        &document.value,
        &[
            "admission_sha256",
            "authority",
            "cases",
            "companion_sha256",
            "format",
            "policy_sha256",
            "protocol_sha256",
            "suite",
            "target",
        ],
        "D10 observations",
    )?;
    expect_string(root, "authority", INPUT_AUTHORITY, "D10 observations")?;
    expect_string(root, "format", INPUT_FORMAT, "D10 observations")?;
    expect_string(root, "suite", "d10", "D10 observations")?;
    expect_string(root, "target", TARGET, "D10 observations")?;
    expect_string(
        root,
        "admission_sha256",
        &sha256_identity(&admission.document("admission.json")?.bytes),
        "D10 observation admission identity",
    )?;
    let admission_value = policy
        .admission()
        .as_object()
        .ok_or_else(|| "held D10 admission must be an object".to_owned())?;
    expect_string(
        root,
        "policy_sha256",
        get_string(admission_value, "policy_sha256", "held D10 admission")?,
        "D10 observation policy identity",
    )?;
    expect_string(
        root,
        "protocol_sha256",
        PROTOCOL_SHA256,
        "D10 observation protocol identity",
    )?;
    validate_companion_identities(policy, get(root, "companion_sha256", "D10 observations")?)?;

    let policy_value = policy.document_value("policy.json")?;
    let policy_object = policy_value
        .as_object()
        .ok_or_else(|| "held D10 policy must be an object".to_owned())?;
    let policy_cases = get(policy_object, "cases", "held D10 policy")?
        .as_array()
        .ok_or_else(|| "held D10 policy cases must be an array".to_owned())?;
    let thresholds = get(policy_object, "thresholds", "held D10 policy")?
        .as_object()
        .ok_or_else(|| "held D10 thresholds must be an object".to_owned())?;
    let max_regression = get_u64(thresholds, "maximum_regression_ppm", "held D10 thresholds")?;
    let min_vendor = get_u64(
        thresholds,
        "minimum_per_case_vendor_ratio_ppm",
        "held D10 thresholds",
    )?;
    let min_weighted = get_u64(
        thresholds,
        "minimum_weighted_vendor_ratio_ppm",
        "held D10 thresholds",
    )?;
    let cases = get(root, "cases", "D10 observations")?
        .as_array()
        .ok_or_else(|| "D10 observation cases must be an array".to_owned())?;
    if cases.len() != CASE_ROSTER.len() {
        return Err("D10 observations do not cover the exact K1-K7 roster".to_owned());
    }
    let mut sample_ids = BTreeSet::new();
    let mut metrics = Vec::new();
    for (((case, policy_case), (case_id, family)), index) in cases
        .iter()
        .zip(policy_cases)
        .zip(CASE_ROSTER)
        .zip(0_usize..)
    {
        metrics.push(validate_case(
            policy,
            case,
            policy_case,
            case_id,
            family,
            index,
            max_regression,
            min_vendor,
            &mut sample_ids,
        )?);
    }
    let aggregate = weighted_aggregate(&metrics, min_weighted)?;
    let all_checked_gates_pass = metrics
        .iter()
        .all(|metric| metric.regression_gate && metric.vendor_gate.unwrap_or(true))
        && aggregate.as_ref().is_none_or(|(_, gate)| *gate);
    let result_cases = metrics.iter().map(case_metric_json).collect::<Vec<_>>();
    let aggregate_json = aggregate.map_or(Value::Null, |(value, gate)| {
        json!({
            "gate_pass": gate,
            "minimum_ratio_ppm": min_weighted,
            "ratio_power": value,
        })
    });
    Ok(json!({
        "all_checked_gates_pass": all_checked_gates_pass,
        "authority": OUTPUT_AUTHORITY,
        "cases": result_cases,
        "closes": [],
        "format": OUTPUT_FORMAT,
        "independent_validation": false,
        "milestone": "M1",
        "nonclaim": NONCLAIM,
        "obligation_id": "m1.r31",
        "observation_bundle_protocol_sha256": PROTOCOL_SHA256,
        "observation_counts_enforced": true,
        "observations_sha256": sha256_identity(&document.bytes),
        "observations_structurally_admitted": true,
        "path_id": "d10-bench",
        "policy_admission_sha256": sha256_identity(&admission.document("admission.json")?.bytes),
        "policy_sha256": get_string(admission_value, "policy_sha256", "held D10 admission")?,
        "policy_values_validated": false,
        "qualification_evidence": false,
        "r31_closed": false,
        "rate_formula": "floor(work_unit.count_per_iteration * iterations * 1000000000 / elapsed_ns)",
        "rate_unit": "integer-policy-work-units-per-second",
        "recorded_samples_per_applicable_implementation": RECORDED,
        "status": STATUS,
        "suite": "d10",
        "target": TARGET,
        "telemetry_resource_identity_bindings_enforced": true,
        "telemetry_resource_outputs_authenticated": false,
        "holdout_membership_enforced": true,
        "warmups_per_applicable_implementation": WARMUPS,
        "weighted_applicable_vendor_aggregate": aggregate_json,
    }))
}

fn validate_companion_identities(policy: &HeldValidatedPolicy, value: &Value) -> BenchResult<()> {
    let object = value
        .as_object()
        .ok_or_else(|| "D10 companion identities must be an object".to_owned())?;
    exact_keys(
        object,
        &COMPANIONS.iter().map(|(name, _)| *name).collect::<Vec<_>>(),
        "D10 companion identities",
    )?;
    for (name, path) in COMPANIONS {
        expect_string(
            object,
            name,
            &sha256_identity(policy.document_bytes(path)?),
            "D10 companion identity",
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_case(
    policy: &HeldValidatedPolicy,
    value: &Value,
    policy_value: &Value,
    case_id: &str,
    family: &str,
    case_index: usize,
    maximum_regression_ppm: u64,
    minimum_vendor_ppm: u64,
    global_sample_ids: &mut BTreeSet<String>,
) -> BenchResult<CaseMetric> {
    let case = exact_object(
        value,
        &[
            "case_id",
            "implementations",
            "kernel_family",
            "profile_sha256",
            "resource_bindings",
            "work_unit_semantics_sha256",
        ],
        "D10 observation case",
    )?;
    expect_string(case, "case_id", case_id, "D10 observation case id")?;
    expect_string(
        case,
        "kernel_family",
        family,
        "D10 observation kernel family",
    )?;
    let policy_case = policy_value
        .as_object()
        .ok_or_else(|| "held D10 policy case must be an object".to_owned())?;
    let weight = get_u64(policy_case, "weight", "held D10 policy case")?;
    if weight > MAX_EXACT_AGGREGATE_CASE_WEIGHT {
        return Err(format!(
            "D10 case weight exceeds the observation validator's exact aggregate-computability bound of {MAX_EXACT_AGGREGATE_CASE_WEIGHT}"
        ));
    }
    let profile = get(policy_case, "profile", "held D10 policy case")?
        .as_object()
        .ok_or_else(|| "held D10 profile must be an object".to_owned())?;
    expect_string(
        case,
        "profile_sha256",
        get_string(profile, "sha256", "held D10 profile")?,
        "D10 observation profile identity",
    )?;
    let work_unit = get(policy_case, "work_unit", "held D10 policy case")?
        .as_object()
        .ok_or_else(|| "held D10 work unit must be an object".to_owned())?;
    expect_string(
        case,
        "work_unit_semantics_sha256",
        get_string(work_unit, "semantics_sha256", "held D10 work unit")?,
        "D10 work-unit semantics identity",
    )?;
    let count_per_iteration = get_u64(work_unit, "count_per_iteration", "held D10 work unit")?;
    validate_resource_binding(
        policy,
        get(case, "resource_bindings", "D10 observation case")?,
        case_index,
    )?;
    let implementations = get(case, "implementations", "D10 observation case")?
        .as_array()
        .ok_or_else(|| "D10 implementations must be an array".to_owned())?;
    if implementations.len() != IMPLEMENTATIONS.len() {
        return Err(format!("D10 implementation roster drifted for {case_id}"));
    }
    let vendor = get(policy_case, "vendor", "held D10 policy case")?
        .as_object()
        .ok_or_else(|| "held D10 vendor mapping must be an object".to_owned())?;
    let vendor_applicable = get(vendor, "applicable", "held D10 vendor mapping")?
        .as_bool()
        .ok_or_else(|| "held D10 vendor applicability must be boolean".to_owned())?;
    let expected_identities = implementation_identities(policy, policy_case, vendor, profile)?;
    let expected_configs = implementation_configs(policy, vendor, profile)?;
    let mut medians = Vec::new();
    let mut warmup_projection = Vec::new();
    let mut recorded_projection = Vec::new();
    let mut shared_holdout_member = None;
    for (((implementation, implementation_name), expected_identity), expected_config) in
        implementations
            .iter()
            .zip(IMPLEMENTATIONS)
            .zip(expected_identities)
            .zip(expected_configs)
    {
        let applicable = implementation_name != &"vendor" || vendor_applicable;
        let validated = validate_implementation(
            policy,
            implementation,
            implementation_name,
            applicable,
            expected_identity,
            expected_config,
            count_per_iteration,
            global_sample_ids,
        )?;
        if let Some(member) = &validated.holdout_member {
            if let Some(expected) = &shared_holdout_member {
                if member != expected {
                    return Err(
                        "all applicable D10 implementations in a case must use one exact shared holdout member"
                            .to_owned(),
                    );
                }
            } else {
                shared_holdout_member = Some(member.clone());
            }
        }
        medians.push(validated.median);
        warmup_projection.push(json!({
            "holdout_member": validated.holdout_member,
            "implementation": implementation_name,
            "sample_ids": validated.warmup_ids,
        }));
        recorded_projection.push(json!({
            "holdout_member": validated.holdout_member,
            "implementation": implementation_name,
            "sample_ids": validated.recorded_ids,
        }));
    }
    validate_order_binding(policy, case_index, &warmup_projection, &recorded_projection)?;
    let reference = medians[0]
        .clone()
        .ok_or_else(|| "Ferric reference observations cannot be inapplicable".to_owned())?;
    let ferric = medians[1]
        .clone()
        .ok_or_else(|| "Ferric observations cannot be inapplicable".to_owned())?;
    let regression_ppm = regression_ppm(&ferric, &reference);
    let regression_gate = regression_gate(&ferric, &reference, maximum_regression_ppm);
    let vendor_median = medians[2].clone();
    let (vendor_ratio_ppm, vendor_gate) =
        vendor_median
            .as_ref()
            .map_or((None, None), |vendor_median| {
                (
                    Some(ratio_ppm(&ferric, vendor_median)),
                    Some(ratio_gate(&ferric, vendor_median, minimum_vendor_ppm)),
                )
            });
    Ok(CaseMetric {
        case_id: case_id.to_owned(),
        ferric,
        reference,
        regression_gate,
        regression_ppm,
        vendor: vendor_median,
        vendor_gate,
        vendor_ratio_ppm,
        weight,
    })
}

struct ValidatedImplementation {
    holdout_member: Option<Value>,
    median: Option<Rational>,
    recorded_ids: Vec<String>,
    warmup_ids: Vec<String>,
}

fn implementation_identities<'a>(
    policy: &'a HeldValidatedPolicy,
    policy_case: &'a Map<String, Value>,
    vendor: &'a Map<String, Value>,
    _profile: &'a Map<String, Value>,
) -> BenchResult<[Option<&'a str>; 3]> {
    let regression = policy
        .document_value("regression-reference.json")?
        .as_object()
        .ok_or_else(|| "held regression reference must be an object".to_owned())?;
    Ok([
        Some(get_string(
            regression,
            "implementation_sha256",
            "held regression reference",
        )?),
        Some(get_string(
            policy_case,
            "ferric_implementation_sha256",
            "held D10 policy case",
        )?),
        get(vendor, "implementation_sha256", "held D10 vendor mapping")?.as_str(),
    ])
}

fn implementation_configs<'a>(
    policy: &'a HeldValidatedPolicy,
    vendor: &'a Map<String, Value>,
    profile: &'a Map<String, Value>,
) -> BenchResult<[Option<&'a str>; 3]> {
    let regression = policy
        .document_value("regression-reference.json")?
        .as_object()
        .ok_or_else(|| "held regression reference must be an object".to_owned())?;
    Ok([
        Some(get_string(
            regression,
            "config_sha256",
            "held regression reference",
        )?),
        Some(get_string(profile, "sha256", "held D10 profile")?),
        get(vendor, "config_sha256", "held D10 vendor mapping")?.as_str(),
    ])
}

#[allow(clippy::too_many_arguments)]
fn validate_implementation(
    policy: &HeldValidatedPolicy,
    value: &Value,
    name: &str,
    applicable: bool,
    expected_identity: Option<&str>,
    expected_config: Option<&str>,
    count_per_iteration: u64,
    global_sample_ids: &mut BTreeSet<String>,
) -> BenchResult<ValidatedImplementation> {
    let implementation = exact_object(
        value,
        &[
            "applicable",
            "bindings",
            "config_sha256",
            "implementation",
            "implementation_sha256",
            "recorded",
            "regression_measurement_roster_sha256",
            "holdout_member",
            "tuning_budget",
            "warmups",
        ],
        "D10 observation implementation",
    )?;
    expect_string(
        implementation,
        "implementation",
        name,
        "D10 observation implementation",
    )?;
    expect_bool(
        implementation,
        "applicable",
        applicable,
        "D10 observation applicability",
    )?;
    expect_optional_string(
        implementation,
        "implementation_sha256",
        expected_identity,
        "D10 observation implementation identity",
    )?;
    expect_optional_string(
        implementation,
        "config_sha256",
        expected_config,
        "D10 observation config identity",
    )?;
    validate_execution_bindings(
        policy,
        get(implementation, "bindings", "D10 observation implementation")?,
    )?;
    let holdout_member = validate_holdout_member(
        policy,
        get(
            implementation,
            "holdout_member",
            "D10 observation implementation",
        )?,
        applicable,
    )?;
    validate_regression_roster_binding(policy, implementation, name, applicable)?;
    validate_tuning_budget(policy, implementation, name, applicable)?;
    let warmups = get(implementation, "warmups", "D10 observation implementation")?
        .as_array()
        .ok_or_else(|| "D10 warmups must be an array".to_owned())?;
    let recorded = get(implementation, "recorded", "D10 observation implementation")?
        .as_array()
        .ok_or_else(|| "D10 recorded samples must be an array".to_owned())?;
    if !applicable {
        if !warmups.is_empty() || !recorded.is_empty() {
            return Err(
                "inapplicable D10 vendor must have exact empty warmup and recorded rosters"
                    .to_owned(),
            );
        }
        return Ok(ValidatedImplementation {
            holdout_member,
            median: None,
            recorded_ids: Vec::new(),
            warmup_ids: Vec::new(),
        });
    }
    if warmups.len() != WARMUPS || recorded.len() != RECORDED {
        return Err(format!(
            "D10 {name} must have exactly {WARMUPS} warmups and {RECORDED} recorded samples"
        ));
    }
    let mut warmup_ids = Vec::with_capacity(WARMUPS);
    for (sequence, sample) in warmups.iter().enumerate() {
        let sample = exact_object(sample, &["sample_id", "sequence"], "D10 untimed warmup")?;
        expect_u64(sample, "sequence", sequence as u64, "D10 warmup sequence")?;
        warmup_ids.push(insert_sample_id(sample, global_sample_ids, "D10 warmup")?);
    }
    let mut recorded_ids = Vec::with_capacity(RECORDED);
    let mut rates = Vec::with_capacity(RECORDED);
    for (sequence, sample) in recorded.iter().enumerate() {
        let sample = exact_object(
            sample,
            &["elapsed_ns", "iterations", "sample_id", "sequence"],
            "D10 recorded sample",
        )?;
        expect_u64(sample, "sequence", sequence as u64, "D10 recorded sequence")?;
        recorded_ids.push(insert_sample_id(
            sample,
            global_sample_ids,
            "D10 recorded sample",
        )?);
        let elapsed = get_u64(sample, "elapsed_ns", "D10 recorded sample")?;
        let iterations = get_u64(sample, "iterations", "D10 recorded sample")?;
        if elapsed == 0 || iterations == 0 {
            return Err("D10 recorded elapsed time and iterations must be positive".to_owned());
        }
        let numerator = u128::from(count_per_iteration)
            .checked_mul(u128::from(iterations))
            .and_then(|value| value.checked_mul(RATE_SCALE))
            .ok_or_else(|| {
                "D10 raw throughput numerator overflowed exact u128 arithmetic".to_owned()
            })?;
        let rate = numerator / u128::from(elapsed);
        let rate = u64::try_from(rate).map_err(|_| {
            "D10 recomputed throughput does not fit the u64 Metric record domain".to_owned()
        })?;
        if rate == 0 {
            return Err("D10 recomputed throughput must be positive".to_owned());
        }
        rates.push(rate);
    }
    Ok(ValidatedImplementation {
        holdout_member,
        median: Some(median(&mut rates)),
        recorded_ids,
        warmup_ids,
    })
}

fn validate_holdout_member(
    policy: &HeldValidatedPolicy,
    value: &Value,
    applicable: bool,
) -> BenchResult<Option<Value>> {
    if !applicable {
        if !value.is_null() {
            return Err("inapplicable D10 vendor holdout member must be null".to_owned());
        }
        return Ok(None);
    }
    let member = exact_object(value, &["id", "sha256"], "D10 observed holdout member")?;
    let holdout = policy
        .document_value("holdout.json")?
        .as_object()
        .ok_or_else(|| "held D10 holdout policy must be an object".to_owned())?;
    let members = get(holdout, "members", "held D10 holdout policy")?
        .as_array()
        .ok_or_else(|| "held D10 holdout members must be an array".to_owned())?;
    if !members.iter().any(|candidate| candidate == value) {
        return Err("D10 observed member is not in the exact admitted holdout roster".to_owned());
    }
    require_safe_id(
        get_string(member, "id", "D10 observed holdout member")?,
        "D10 observed holdout member",
    )?;
    Ok(Some(value.clone()))
}

fn validate_regression_roster_binding(
    policy: &HeldValidatedPolicy,
    implementation: &Map<String, Value>,
    name: &str,
    applicable: bool,
) -> BenchResult<()> {
    let expected = if name == "ferric-reference" && applicable {
        let regression = policy
            .document_value("regression-reference.json")?
            .as_object()
            .ok_or_else(|| "held D10 regression reference must be an object".to_owned())?;
        Some(get_string(
            regression,
            "measurement_roster_sha256",
            "held D10 regression reference",
        )?)
    } else {
        None
    };
    expect_optional_string(
        implementation,
        "regression_measurement_roster_sha256",
        expected,
        "D10 regression measurement-roster binding",
    )
}

fn validate_execution_bindings(policy: &HeldValidatedPolicy, value: &Value) -> BenchResult<()> {
    let bindings = exact_object(
        value,
        &[
            "calibration_sha256",
            "execution_order_sha256",
            "holdout_sha256",
            "regression_reference_sha256",
            "resource_inspection_sha256",
            "telemetry_sha256",
            "timing_sha256",
            "tuning_sha256",
        ],
        "D10 implementation bindings",
    )?;
    for (field, path) in [
        ("calibration_sha256", "calibration.json"),
        ("execution_order_sha256", "execution-order.json"),
        ("holdout_sha256", "holdout.json"),
        ("regression_reference_sha256", "regression-reference.json"),
        ("resource_inspection_sha256", "resource-inspection.json"),
        ("telemetry_sha256", "telemetry.json"),
        ("timing_sha256", "timing.json"),
        ("tuning_sha256", "tuning.json"),
    ] {
        expect_string(
            bindings,
            field,
            &sha256_identity(policy.document_bytes(path)?),
            "D10 implementation binding",
        )?;
    }
    Ok(())
}

fn validate_tuning_budget(
    policy: &HeldValidatedPolicy,
    implementation: &Map<String, Value>,
    name: &str,
    applicable: bool,
) -> BenchResult<()> {
    let budget = get(
        implementation,
        "tuning_budget",
        "D10 observation implementation",
    )?;
    if name == "ferric-reference" || !applicable {
        if !budget.is_null() {
            return Err(format!("D10 {name} tuning budget must be null"));
        }
        return Ok(());
    }
    let budget = exact_object(budget, &["budget", "unit"], "D10 observed tuning budget")?;
    let tuning = policy
        .document_value("tuning.json")?
        .as_object()
        .ok_or_else(|| "held D10 tuning policy must be an object".to_owned())?;
    expect_string(
        budget,
        "unit",
        get_string(tuning, "budget_unit", "held D10 tuning policy")?,
        "D10 observed tuning budget unit",
    )?;
    let field = if name == "ferric" {
        "ferric_budget"
    } else {
        "vendor_budget"
    };
    expect_u64(
        budget,
        "budget",
        get_u64(tuning, field, "held D10 tuning policy")?,
        "D10 observed tuning budget",
    )
}

fn validate_resource_binding(
    policy: &HeldValidatedPolicy,
    value: &Value,
    case_index: usize,
) -> BenchResult<()> {
    let resources = policy
        .document_value("resource-inspection.json")?
        .as_object()
        .ok_or_else(|| "held D10 resource policy must be an object".to_owned())?;
    let cases = get(resources, "cases", "held D10 resource policy")?
        .as_array()
        .ok_or_else(|| "held D10 resource cases must be an array".to_owned())?;
    if value != &cases[case_index] {
        return Err(
            "D10 observation resource binding drifted from the held policy companion".to_owned(),
        );
    }
    Ok(())
}

fn validate_order_binding(
    policy: &HeldValidatedPolicy,
    case_index: usize,
    warmups: &[Value],
    recorded: &[Value],
) -> BenchResult<()> {
    let order = policy
        .document_value("execution-order.json")?
        .as_object()
        .ok_or_else(|| "held D10 execution-order policy must be an object".to_owned())?;
    let cases = get(order, "cases", "held D10 execution-order policy")?
        .as_array()
        .ok_or_else(|| "held D10 execution-order cases must be an array".to_owned())?;
    let case = cases[case_index]
        .as_object()
        .ok_or_else(|| "held D10 execution-order case must be an object".to_owned())?;
    let warmup_sha256 =
        sha256_identity(&encode_canonical_document(&Value::Array(warmups.to_vec()))?);
    let recorded_sha256 = sha256_identity(&encode_canonical_document(&Value::Array(
        recorded.to_vec(),
    ))?);
    expect_string(
        case,
        "warmup_order_sha256",
        &warmup_sha256,
        "D10 warmup order projection",
    )?;
    expect_string(
        case,
        "recorded_order_sha256",
        &recorded_sha256,
        "D10 recorded order projection",
    )
}

fn insert_sample_id(
    sample: &Map<String, Value>,
    global: &mut BTreeSet<String>,
    description: &str,
) -> BenchResult<String> {
    let id = get_string(sample, "sample_id", description)?;
    require_safe_id(id, description)?;
    if !global.insert(id.to_owned()) {
        return Err(format!("{description} identities must be globally unique"));
    }
    Ok(id.to_owned())
}

fn median(values: &mut [u64]) -> Rational {
    values.sort_unstable();
    let upper = u128::from(values[values.len() / 2]);
    let lower = u128::from(values[values.len() / 2 - 1]);
    Rational::new(lower + upper, 2)
}

fn ratio_ppm(numerator: &Rational, denominator: &Rational) -> BigUint {
    let top = BigUint::from(numerator.numerator)
        * BigUint::from(denominator.denominator)
        * BigUint::from(PPM_SCALE);
    let bottom = BigUint::from(numerator.denominator) * BigUint::from(denominator.numerator);
    top / bottom
}

fn regression_ppm(ferric: &Rational, reference: &Rational) -> BigUint {
    if ratio_gate(ferric, reference, PPM_SCALE) {
        return BigUint::from(0_u8);
    }
    let reference_scaled = BigUint::from(reference.numerator) * BigUint::from(ferric.denominator);
    let ferric_scaled = BigUint::from(ferric.numerator) * BigUint::from(reference.denominator);
    (reference_scaled - ferric_scaled) * BigUint::from(PPM_SCALE)
        / (BigUint::from(reference.numerator) * BigUint::from(ferric.denominator))
}

fn ratio_gate(numerator: &Rational, denominator: &Rational, minimum_ppm: u64) -> bool {
    BigUint::from(numerator.numerator)
        * BigUint::from(denominator.denominator)
        * BigUint::from(PPM_SCALE)
        >= BigUint::from(denominator.numerator)
            * BigUint::from(numerator.denominator)
            * BigUint::from(minimum_ppm)
}

fn regression_gate(ferric: &Rational, reference: &Rational, maximum_ppm: u64) -> bool {
    if maximum_ppm >= PPM_SCALE {
        return true;
    }
    ratio_gate(ferric, reference, PPM_SCALE - maximum_ppm)
}

fn weighted_aggregate(
    metrics: &[CaseMetric],
    minimum_ppm: u64,
) -> BenchResult<Option<(Value, bool)>> {
    let applicable = metrics
        .iter()
        .filter(|metric| metric.vendor.is_some())
        .collect::<Vec<_>>();
    if applicable.is_empty() {
        return Ok(None);
    }
    let total_weight = applicable.iter().try_fold(0_u64, |total, metric| {
        total
            .checked_add(metric.weight)
            .ok_or_else(|| "D10 aggregate weight overflowed".to_owned())
    })?;
    let mut numerator = BigUint::from(1_u8);
    let mut denominator = BigUint::from(1_u8);
    for metric in applicable {
        let vendor = metric
            .vendor
            .as_ref()
            .ok_or_else(|| "applicable D10 vendor disappeared".to_owned())?;
        let ratio_numerator =
            BigUint::from(metric.ferric.numerator) * BigUint::from(vendor.denominator);
        let ratio_denominator =
            BigUint::from(metric.ferric.denominator) * BigUint::from(vendor.numerator);
        numerator *= pow_checked(&ratio_numerator, metric.weight)?;
        denominator *= pow_checked(&ratio_denominator, metric.weight)?;
        if numerator.bits() > MAX_EXACT_AGGREGATE_BITS
            || denominator.bits() > MAX_EXACT_AGGREGATE_BITS
        {
            return Err(
                "D10 exact weighted aggregate exceeds the documented host representation bound"
                    .to_owned(),
            );
        }
    }
    let gate = numerator.clone() * pow_checked(&BigUint::from(PPM_SCALE), total_weight)?
        >= denominator.clone() * pow_checked(&BigUint::from(minimum_ppm), total_weight)?;
    Ok(Some((
        json!({
            "degree": total_weight,
            "denominator": denominator.to_str_radix(10),
            "numerator": numerator.to_str_radix(10),
            "semantics": "exact-weighted-geometric-mean-ratio-raised-to-total-applicable-weight",
        }),
        gate,
    )))
}

fn pow_checked(base: &BigUint, exponent: u64) -> BenchResult<BigUint> {
    if base.bits().saturating_mul(exponent) > MAX_EXACT_AGGREGATE_BITS {
        return Err(
            "D10 exact weighted aggregate exceeds the documented host representation bound"
                .to_owned(),
        );
    }
    let mut result = BigUint::from(1_u8);
    let mut factor = base.clone();
    let mut remaining = exponent;
    while remaining != 0 {
        if remaining & 1 == 1 {
            result *= &factor;
        }
        remaining >>= 1;
        if remaining != 0 {
            factor = &factor * &factor;
        }
    }
    Ok(result)
}

fn case_metric_json(metric: &CaseMetric) -> Value {
    json!({
        "case_id": metric.case_id,
        "ferric_median": metric.ferric.as_json(),
        "ferric_reference_median": metric.reference.as_json(),
        "regression_gate_pass": metric.regression_gate,
        "regression_ppm": metric.regression_ppm.to_str_radix(10),
        "vendor_applicable": metric.vendor.is_some(),
        "vendor_gate_pass": metric.vendor_gate,
        "vendor_median": metric.vendor.as_ref().map(Rational::as_json),
        "vendor_ratio_ppm": metric.vendor_ratio_ppm.as_ref().map(|value| value.to_str_radix(10)),
        "weight": metric.weight,
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
    if object.len() != expected.len() || !expected.iter().all(|field| object.contains_key(*field)) {
        return Err(format!("{description} fields drifted"));
    }
    Ok(())
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

fn expect_optional_string(
    object: &Map<String, Value>,
    field: &str,
    expected: Option<&str>,
    description: &str,
) -> BenchResult<()> {
    match (get(object, field, description)?, expected) {
        (Value::String(actual), Some(expected)) if actual == expected => Ok(()),
        (Value::Null, None) => Ok(()),
        _ => Err(format!("{description} drifted")),
    }
}

fn expect_u64(
    object: &Map<String, Value>,
    field: &str,
    expected: u64,
    description: &str,
) -> BenchResult<()> {
    if get_u64(object, field, description)? != expected {
        return Err(format!("{description} drifted"));
    }
    Ok(())
}

fn expect_bool(
    object: &Map<String, Value>,
    field: &str,
    expected: bool,
    description: &str,
) -> BenchResult<()> {
    if get(object, field, description)?.as_bool() != Some(expected) {
        return Err(format!("{description} drifted"));
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

fn require_safe_path(path: &Path, description: &str) -> BenchResult<()> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
    {
        return Err(format!("{description} path is unsafe"));
    }
    Ok(())
}

fn validate_roster(root: &OwnedFd, expected: &[&str], description: &str) -> BenchResult<()> {
    let expected = expected
        .iter()
        .map(|name| (*name).to_owned())
        .collect::<BTreeSet<_>>();
    let actual = directory_roster(root, description)?;
    if actual != expected {
        return Err(format!("{description} file roster drifted"));
    }
    Ok(())
}

fn directory_roster(root: &OwnedFd, description: &str) -> BenchResult<BTreeSet<String>> {
    let mut entries =
        Dir::read_from(root).map_err(|error| format!("cannot enumerate {description}: {error}"))?;
    let mut names = BTreeSet::new();
    while let Some(entry) = entries.read() {
        let entry = entry.map_err(|error| format!("cannot enumerate {description}: {error}"))?;
        let bytes = entry.file_name().to_bytes();
        if matches!(bytes, b"." | b"..") {
            continue;
        }
        let name = std::str::from_utf8(bytes)
            .map_err(|_| format!("{description} filename must be UTF-8"))?;
        if !name.is_ascii() || !names.insert(name.to_owned()) {
            return Err(format!("{description} filename roster is invalid"));
        }
    }
    Ok(names)
}

fn same_snapshot(initial: &Stat, final_stat: &Stat) -> bool {
    initial.st_dev == final_stat.st_dev
        && initial.st_ino == final_stat.st_ino
        && initial.st_mode == final_stat.st_mode
        && initial.st_nlink == final_stat.st_nlink
        && initial.st_uid == final_stat.st_uid
        && initial.st_gid == final_stat.st_gid
        && initial.st_size == final_stat.st_size
        && initial.st_mtime == final_stat.st_mtime
        && initial.st_mtime_nsec == final_stat.st_mtime_nsec
        && initial.st_ctime == final_stat.st_ctime
        && initial.st_ctime_nsec == final_stat.st_ctime_nsec
}

#[derive(Debug)]
struct StagedFile {
    bytes: Vec<u8>,
    file: File,
    name: OsString,
    snapshot: Stat,
}

struct ExactBundle {
    armed: bool,
    files: Vec<StagedFile>,
    output_name: OsString,
    parent: OwnedFd,
    parent_snapshot: Stat,
    staging: OwnedFd,
    staging_name: OsString,
    staging_path: PathBuf,
    staging_snapshot: Stat,
}

impl ExactBundle {
    fn create_with_hook(
        output: &Path,
        after_mkdir: impl FnOnce(&Path) -> BenchResult<()>,
    ) -> BenchResult<Self> {
        let output_name = safe_output_name(output)?;
        let parent_path = output
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        require_safe_parent_path(parent_path)?;
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
        .map_err(|error| format!("cannot securely open D10 observation output parent: {error}"))?;
        let parent_snapshot = fstat(&parent)
            .map_err(|error| format!("cannot inspect D10 observation output parent: {error}"))?;
        validate_controlled_directory(&parent_snapshot, "D10 observation output parent")?;
        require_absent(&parent, &output_name, "D10 observation output")?;
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
                    after_mkdir.take().ok_or_else(|| {
                        "D10 observation mkdir hook was already consumed".to_owned()
                    })?(&staging_path)?;
                    let staging = open_directory_at(
                        &parent,
                        Path::new(&staging_name),
                        "D10 observation staging",
                    )?;
                    let staging_snapshot = fstat(&staging).map_err(|error| {
                        format!("cannot inspect D10 observation staging: {error}")
                    })?;
                    validate_adopted_directory(&staging_snapshot, "D10 observation staging")?;
                    validate_roster(&staging, &[], "newly adopted D10 observation staging")?;
                    let current_parent = fstat(&parent).map_err(|error| {
                        format!("cannot reinspect D10 observation output parent: {error}")
                    })?;
                    validate_controlled_directory(
                        &current_parent,
                        "D10 observation output parent",
                    )?;
                    return Ok(Self {
                        armed: true,
                        files: Vec::new(),
                        output_name,
                        parent,
                        parent_snapshot: current_parent,
                        staging,
                        staging_name,
                        staging_path,
                        staging_snapshot,
                    });
                }
                Err(error) if error == rustix::io::Errno::EXIST => {}
                Err(error) => {
                    return Err(format!("cannot create D10 observation staging: {error}"))
                }
            }
        }
        Err("D10 observation staging namespace was exhausted".to_owned())
    }

    fn write(&mut self, name: &str, bytes: &[u8]) -> BenchResult<()> {
        if !OUTPUT_FILES.contains(&name) || self.files.iter().any(|file| file.name == name) {
            return Err("D10 observation output name or order drifted".to_owned());
        }
        let descriptor = openat2(
            &self.staging,
            Path::new(name),
            OFlags::RDWR
                | OFlags::CREATE
                | OFlags::EXCL
                | OFlags::NOFOLLOW
                | OFlags::NONBLOCK
                | OFlags::CLOEXEC,
            Mode::RUSR | Mode::WUSR,
            ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )
        .map_err(|error| format!("cannot create staged D10 observation output {name}: {error}"))?;
        let created = fstat(&descriptor).map_err(|error| {
            format!("cannot inspect staged D10 observation output {name}: {error}")
        })?;
        if let Err(error) = validate_created_file(&created, 0, name) {
            cleanup_created_name(&self.staging, OsStr::new(name), &created);
            return Err(error);
        }
        let mut staged = StagedFile {
            bytes: bytes.to_vec(),
            file: File::from(descriptor),
            name: OsString::from(name),
            snapshot: created,
        };
        if let Err(error) = staged
            .file
            .write_all(bytes)
            .and_then(|()| staged.file.sync_all())
        {
            cleanup_created_name(&self.staging, OsStr::new(name), &created);
            return Err(format!(
                "cannot write staged D10 observation output {name}: {error}"
            ));
        }
        staged.snapshot = match fstat(&staged.file) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                cleanup_created_name(&self.staging, OsStr::new(name), &created);
                return Err(format!(
                    "cannot reinspect staged D10 observation output {name}: {error}"
                ));
            }
        };
        if let Err(error) = validate_created_file(&staged.snapshot, bytes.len() as u64, name) {
            cleanup_created_name(&self.staging, OsStr::new(name), &created);
            return Err(error);
        }
        if let Err(error) = verify_held_file(&mut staged, "written staged") {
            cleanup_created_name(&self.staging, OsStr::new(name), &created);
            return Err(error);
        }
        self.files.push(staged);
        Ok(())
    }

    fn publish_exact(
        mut self,
        expected: &[(&str, &[u8])],
        revalidate_inputs: &mut impl FnMut() -> BenchResult<()>,
        pre_publish: impl FnOnce() -> BenchResult<()>,
        after_published_verification: impl FnOnce() -> BenchResult<()>,
    ) -> BenchResult<()> {
        if expected
            .iter()
            .map(|(name, _)| *name)
            .ne(OUTPUT_FILES.iter().copied())
        {
            return Err("D10 observation publication roster drifted".to_owned());
        }
        Self::verify_directory(&mut self.files, &self.staging, expected, "staged")?;
        fsync(&self.staging)
            .map_err(|error| format!("cannot sync D10 observation staging: {error}"))?;
        let settled = fstat(&self.staging)
            .map_err(|error| format!("cannot snapshot D10 observation staging: {error}"))?;
        validate_adopted_directory(&settled, "settled D10 observation staging")?;
        self.staging_snapshot = settled;
        self.rebind_directory(&self.staging_name, &settled, "settled staged")?;
        pre_publish()?;
        revalidate_inputs()?;
        Self::verify_directory(
            &mut self.files,
            &self.staging,
            expected,
            "final prepublication",
        )?;
        self.validate_parent(&self.parent_snapshot, "prepublication")?;
        renameat_with(
            &self.parent,
            &self.staging_name,
            &self.parent,
            &self.output_name,
            RenameFlags::NOREPLACE,
        )
        .map_err(|error| {
            format!("cannot publish D10 observation output without replacement: {error}")
        })?;
        self.armed = false;
        let published = self.rebind_published_directory(&settled)?;
        let published_snapshot = fstat(&published).map_err(|error| {
            format!("cannot snapshot published D10 observation output: {error}")
        })?;
        Self::verify_directory(&mut self.files, &published, expected, "published")?;
        after_published_verification()?;
        revalidate_inputs()?;
        let parent_published = fstat(&self.parent)
            .map_err(|error| format!("cannot inspect published D10 observation parent: {error}"))?;
        validate_directory_transition(
            &self.parent_snapshot,
            &parent_published,
            "published D10 observation parent",
        )?;
        fsync(&self.parent)
            .map_err(|error| format!("cannot sync published D10 observation parent: {error}"))?;
        self.validate_parent(&parent_published, "post-fsync")?;
        let final_directory =
            self.rebind_directory(&self.output_name, &published_snapshot, "final published")?;
        Self::verify_directory(
            &mut self.files,
            &final_directory,
            expected,
            "final published",
        )?;
        self.validate_parent(&parent_published, "final published")
    }

    fn validate_parent(&self, expected: &Stat, phase: &str) -> BenchResult<()> {
        let current = fstat(&self.parent)
            .map_err(|error| format!("cannot inspect {phase} D10 observation parent: {error}"))?;
        validate_controlled_directory(&current, &format!("{phase} D10 observation parent"))?;
        if !same_snapshot(expected, &current) {
            return Err(format!("{phase} D10 observation parent metadata changed"));
        }
        Ok(())
    }

    fn rebind_directory(&self, name: &OsStr, expected: &Stat, phase: &str) -> BenchResult<OwnedFd> {
        let directory = open_directory_at(&self.parent, Path::new(name), phase)?;
        let held = fstat(&self.staging)
            .map_err(|error| format!("cannot inspect held {phase} directory: {error}"))?;
        let rebound = fstat(&directory)
            .map_err(|error| format!("cannot inspect rebound {phase} directory: {error}"))?;
        if !same_snapshot(expected, &held) || !same_snapshot(expected, &rebound) {
            return Err(format!("{phase} D10 observation directory custody changed"));
        }
        Ok(directory)
    }

    fn rebind_published_directory(&self, expected: &Stat) -> BenchResult<OwnedFd> {
        let directory = open_directory_at(&self.parent, Path::new(&self.output_name), "published")?;
        let held = fstat(&self.staging)
            .map_err(|error| format!("cannot inspect held published directory: {error}"))?;
        let rebound = fstat(&directory)
            .map_err(|error| format!("cannot inspect rebound published directory: {error}"))?;
        if !same_directory_identity(expected, &held) || !same_directory_identity(expected, &rebound)
        {
            return Err("published D10 observation directory identity changed".to_owned());
        }
        Ok(directory)
    }

    fn verify_directory(
        files: &mut [StagedFile],
        directory: &OwnedFd,
        expected: &[(&str, &[u8])],
        phase: &str,
    ) -> BenchResult<()> {
        validate_roster(
            directory,
            OUTPUT_FILES,
            &format!("{phase} D10 observation output"),
        )?;
        for ((name, bytes), staged) in expected.iter().zip(files) {
            if staged.name != *name || staged.bytes != *bytes {
                return Err(format!(
                    "{phase} D10 observation output expectation drifted"
                ));
            }
            verify_held_file(staged, phase)?;
            let descriptor = openat2(
                directory,
                Path::new(name),
                OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
                Mode::empty(),
                ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
            )
            .map_err(|error| {
                format!("cannot rebind {phase} D10 observation output {name}: {error}")
            })?;
            let stat = fstat(&descriptor).map_err(|error| {
                format!("cannot inspect rebound {phase} output {name}: {error}")
            })?;
            if !same_snapshot(&staged.snapshot, &stat) {
                return Err(format!(
                    "{phase} D10 observation output name {name} drifted"
                ));
            }
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
            if name_has_identity(&self.staging, &file.name, &file.snapshot) {
                let _ = unlinkat(&self.staging, &file.name, AtFlags::empty());
            }
        }
    }
}

fn verify_held_file(file: &mut StagedFile, phase: &str) -> BenchResult<()> {
    let initial = fstat(&file.file)
        .map_err(|error| format!("cannot inspect held {phase} output: {error}"))?;
    if !same_snapshot(&file.snapshot, &initial) {
        return Err(format!(
            "held {phase} D10 observation output metadata changed"
        ));
    }
    file.file
        .seek(SeekFrom::Start(0))
        .map_err(|error| format!("cannot rewind held {phase} output: {error}"))?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file.file)
        .take(file.bytes.len().saturating_add(1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("cannot reread held {phase} output: {error}"))?;
    let final_stat = fstat(&file.file)
        .map_err(|error| format!("cannot reinspect held {phase} output: {error}"))?;
    if bytes != file.bytes || !same_snapshot(&file.snapshot, &final_stat) {
        return Err(format!("held {phase} D10 observation output bytes changed"));
    }
    Ok(())
}

fn safe_output_name(output: &Path) -> BenchResult<OsString> {
    if output.as_os_str().is_empty()
        || output.file_name().is_none()
        || output
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
    {
        return Err("D10 observation output path is unsafe".to_owned());
    }
    let name = output
        .file_name()
        .ok_or_else(|| "D10 observation output has no final component".to_owned())?;
    let bytes = name.as_bytes();
    if bytes.is_empty()
        || bytes.len() > 255
        || !bytes.is_ascii()
        || matches!(bytes, b"." | b"..")
        || Path::new(name).components().count() != 1
    {
        return Err("D10 observation output name is unsafe".to_owned());
    }
    Ok(name.to_os_string())
}

fn require_safe_parent_path(path: &Path) -> BenchResult<()> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        return Err("D10 observation output parent path is unsafe".to_owned());
    }
    Ok(())
}

fn open_directory_at(parent: &OwnedFd, path: &Path, description: &str) -> BenchResult<OwnedFd> {
    openat2(
        parent,
        path,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )
    .map_err(|error| format!("cannot securely open {description} directory: {error}"))
}

fn require_absent(parent: &OwnedFd, name: &OsStr, description: &str) -> BenchResult<()> {
    match openat2(
        parent,
        Path::new(name),
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    ) {
        Ok(_) => Err(format!("{description} already exists")),
        Err(error) if error == rustix::io::Errno::NOENT => Ok(()),
        Err(error) => Err(format!("cannot safely inspect {description}: {error}")),
    }
}

fn validate_created_file(stat: &Stat, size: u64, description: &str) -> BenchResult<()> {
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile
        || stat.st_mode & 0o7777 != 0o600
        || stat.st_nlink != 1
        || stat.st_uid != geteuid().as_raw()
        || stat.st_gid != getegid().as_raw()
        || u64::try_from(stat.st_size).ok() != Some(size)
    {
        return Err(format!(
            "created {description} lost exact 0600 owner/group/link/size custody"
        ));
    }
    Ok(())
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

// Safe Rust cannot atomically create and open a directory. The publisher adopts
// only this exact empty directory and makes no claim about its inode provenance.
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
    initial: &Stat,
    current: &Stat,
    description: &str,
) -> BenchResult<()> {
    validate_controlled_directory(current, description)?;
    if initial.st_dev != current.st_dev
        || initial.st_ino != current.st_ino
        || initial.st_mode != current.st_mode
        || initial.st_nlink != current.st_nlink
        || initial.st_uid != current.st_uid
        || initial.st_gid != current.st_gid
    {
        return Err(format!(
            "{description} identity or control metadata changed"
        ));
    }
    Ok(())
}

fn same_directory_identity(initial: &Stat, current: &Stat) -> bool {
    initial.st_dev == current.st_dev
        && initial.st_ino == current.st_ino
        && initial.st_mode == current.st_mode
        && initial.st_nlink == current.st_nlink
        && initial.st_uid == current.st_uid
        && initial.st_gid == current.st_gid
        && initial.st_size == current.st_size
        && initial.st_mtime == current.st_mtime
        && initial.st_mtime_nsec == current.st_mtime_nsec
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::d10_policy;
    use std::cell::RefCell;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::rc::Rc;

    struct Fixture {
        _temporary: d10_policy::tests::TestDirectory,
        admission: PathBuf,
        observations: PathBuf,
        output: PathBuf,
        policy: PathBuf,
    }

    fn write_canonical(path: &Path, value: &Value) {
        fs::write(path, encode_canonical_document(value).unwrap()).unwrap();
    }

    fn protocol_bytes() -> Vec<u8> {
        fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("d10_observation_protocol.json"))
            .unwrap()
    }

    fn sample_id(case_id: &str, implementation: &str, phase: &str, sequence: usize) -> String {
        format!("{case_id}-{implementation}-{phase}-{sequence:02}")
    }

    fn make_fixture() -> Fixture {
        make_fixture_with_case_weight(None)
    }

    fn make_fixture_with_case_weight(case_weight: Option<u64>) -> Fixture {
        let (temporary, policy, admission) = d10_policy::tests::fixture();
        let mut policy_value: Value =
            serde_json::from_slice(&fs::read(policy.join("policy.json")).unwrap()).unwrap();
        if let Some(weight) = case_weight {
            policy_value["cases"][0]["weight"] = json!(weight);
        }
        let regression: Value =
            serde_json::from_slice(&fs::read(policy.join("regression-reference.json")).unwrap())
                .unwrap();
        let resources: Value =
            serde_json::from_slice(&fs::read(policy.join("resource-inspection.json")).unwrap())
                .unwrap();
        let holdout: Value =
            serde_json::from_slice(&fs::read(policy.join("holdout.json")).unwrap()).unwrap();
        let tuning: Value =
            serde_json::from_slice(&fs::read(policy.join("tuning.json")).unwrap()).unwrap();
        let holdout_member = holdout["members"][0].clone();
        let mut observation_cases = Vec::new();
        let mut order_cases = Vec::new();
        for (case_index, (case_id, family)) in CASE_ROSTER.iter().enumerate() {
            let policy_case = &policy_value["cases"][case_index];
            let vendor_applicable = policy_case["vendor"]["applicable"].as_bool().unwrap();
            let identities = [
                Some(regression["implementation_sha256"].as_str().unwrap()),
                Some(
                    policy_case["ferric_implementation_sha256"]
                        .as_str()
                        .unwrap(),
                ),
                policy_case["vendor"]["implementation_sha256"].as_str(),
            ];
            let configs = [
                Some(regression["config_sha256"].as_str().unwrap()),
                Some(policy_case["profile"]["sha256"].as_str().unwrap()),
                policy_case["vendor"]["config_sha256"].as_str(),
            ];
            let mut implementations = Vec::new();
            let mut warmup_projection = Vec::new();
            let mut recorded_projection = Vec::new();
            for ((implementation, identity), config) in
                IMPLEMENTATIONS.iter().zip(identities).zip(configs)
            {
                let applicable = *implementation != "vendor" || vendor_applicable;
                let warmups = if applicable {
                    (0..WARMUPS)
                        .map(|sequence| {
                            json!({
                                "sample_id": sample_id(case_id, implementation, "warmup", sequence),
                                "sequence": sequence,
                            })
                        })
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                };
                let recorded = if applicable {
                    (0..RECORDED)
                        .map(|sequence| {
                            json!({
                                "elapsed_ns": if *implementation == "vendor" { 200 } else { 100 },
                                "iterations": 1,
                                "sample_id": sample_id(case_id, implementation, "recorded", sequence),
                                "sequence": sequence,
                            })
                        })
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                };
                warmup_projection.push(json!({
                    "holdout_member": if applicable { holdout_member.clone() } else { Value::Null },
                    "implementation": implementation,
                    "sample_ids": warmups.iter().map(|sample| sample["sample_id"].clone()).collect::<Vec<_>>(),
                }));
                recorded_projection.push(json!({
                    "holdout_member": if applicable { holdout_member.clone() } else { Value::Null },
                    "implementation": implementation,
                    "sample_ids": recorded.iter().map(|sample| sample["sample_id"].clone()).collect::<Vec<_>>(),
                }));
                let tuning_budget = match (*implementation, applicable) {
                    ("ferric", true) => json!({
                        "budget": tuning["ferric_budget"],
                        "unit": tuning["budget_unit"],
                    }),
                    ("vendor", true) => json!({
                        "budget": tuning["vendor_budget"],
                        "unit": tuning["budget_unit"],
                    }),
                    _ => Value::Null,
                };
                implementations.push(json!({
                    "applicable": applicable,
                    "bindings": {},
                    "config_sha256": config,
                    "holdout_member": if applicable { holdout_member.clone() } else { Value::Null },
                    "implementation": implementation,
                    "implementation_sha256": identity,
                    "recorded": recorded,
                    "regression_measurement_roster_sha256": if *implementation == "ferric-reference" { regression["measurement_roster_sha256"].clone() } else { Value::Null },
                    "tuning_budget": tuning_budget,
                    "warmups": warmups,
                }));
            }
            order_cases.push(json!({
                "case_id": case_id,
                "recorded_order_sha256": sha256_identity(&encode_canonical_document(&Value::Array(recorded_projection)).unwrap()),
                "warmup_order_sha256": sha256_identity(&encode_canonical_document(&Value::Array(warmup_projection)).unwrap()),
            }));
            observation_cases.push(json!({
                "case_id": case_id,
                "implementations": implementations,
                "kernel_family": family,
                "profile_sha256": policy_case["profile"]["sha256"],
                "resource_bindings": resources["cases"][case_index],
                "work_unit_semantics_sha256": policy_case["work_unit"]["semantics_sha256"],
            }));
        }
        let mut order: Value =
            serde_json::from_slice(&fs::read(policy.join("execution-order.json")).unwrap())
                .unwrap();
        order["cases"] = Value::Array(order_cases);
        write_canonical(&policy.join("execution-order.json"), &order);
        let order_bytes = fs::read(policy.join("execution-order.json")).unwrap();
        policy_value["companions"]["execution-order"]["bytes"] = json!(order_bytes.len());
        policy_value["companions"]["execution-order"]["sha256"] =
            json!(sha256_identity(&order_bytes));
        write_canonical(&policy.join("policy.json"), &policy_value);

        let admission_arguments = vec![
            OsString::from("admit-experiment-policy"),
            policy.as_os_str().to_os_string(),
            admission.as_os_str().to_os_string(),
        ];
        d10_policy::admit_experiment_policy(&admission_arguments).unwrap();

        let mut companion_sha256 = Map::new();
        for (name, path) in COMPANIONS {
            companion_sha256.insert(
                (*name).to_owned(),
                Value::String(sha256_identity(&fs::read(policy.join(path)).unwrap())),
            );
        }
        let binding_template = json!({
            "calibration_sha256": companion_sha256["calibration"],
            "execution_order_sha256": companion_sha256["execution-order"],
            "holdout_sha256": companion_sha256["holdout"],
            "regression_reference_sha256": companion_sha256["regression-reference"],
            "resource_inspection_sha256": companion_sha256["resource-inspection"],
            "telemetry_sha256": companion_sha256["telemetry"],
            "timing_sha256": companion_sha256["timing"],
            "tuning_sha256": companion_sha256["tuning"],
        });
        for case in &mut observation_cases {
            for implementation in case["implementations"].as_array_mut().unwrap() {
                implementation["bindings"] = binding_template.clone();
            }
        }
        let observations = temporary.0.join("observations");
        fs::create_dir(&observations).unwrap();
        let observation_value = json!({
            "admission_sha256": sha256_identity(&fs::read(admission.join("admission.json")).unwrap()),
            "authority": INPUT_AUTHORITY,
            "cases": observation_cases,
            "companion_sha256": companion_sha256,
            "format": INPUT_FORMAT,
            "policy_sha256": sha256_identity(&fs::read(policy.join("policy.json")).unwrap()),
            "protocol_sha256": PROTOCOL_SHA256,
            "suite": "d10",
            "target": TARGET,
        });
        write_canonical(&observations.join("observations.json"), &observation_value);
        fs::write(observations.join("protocol.json"), protocol_bytes()).unwrap();
        let output = temporary.0.join("validated");
        Fixture {
            _temporary: temporary,
            admission,
            observations,
            output,
            policy,
        }
    }

    fn arguments(fixture: &Fixture) -> Vec<OsString> {
        vec![
            OsString::from(COMMAND),
            fixture.policy.as_os_str().to_os_string(),
            fixture.admission.as_os_str().to_os_string(),
            fixture.observations.as_os_str().to_os_string(),
            fixture.output.as_os_str().to_os_string(),
        ]
    }

    fn mutate_observations(fixture: &Fixture, mutation: impl FnOnce(&mut Value)) {
        let path = fixture.observations.join("observations.json");
        let mut value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        mutation(&mut value);
        write_canonical(&path, &value);
    }

    #[test]
    fn canonical_mixed_applicability_recomputes_metrics_and_stays_non_evidence() {
        let fixture = make_fixture();
        validate_policy_observations(&arguments(&fixture)).unwrap();
        let result: Value =
            serde_json::from_slice(&fs::read(fixture.output.join("validation.json")).unwrap())
                .unwrap();
        assert_eq!(result["authority"], OUTPUT_AUTHORITY);
        assert_eq!(result["status"], STATUS);
        assert_eq!(result["r31_closed"], false);
        assert_eq!(result["qualification_evidence"], false);
        assert_eq!(result["independent_validation"], false);
        assert_eq!(result["observation_counts_enforced"], true);
        assert_eq!(result["holdout_membership_enforced"], true);
        assert_eq!(result["telemetry_resource_outputs_authenticated"], false);
        assert_eq!(
            result["cases"][0]["ferric_median"]["numerator"],
            "110000000"
        );
        assert_eq!(result["cases"][0]["vendor_ratio_ppm"], "2000000");
        assert_eq!(result["cases"][1]["vendor_applicable"], false);
        assert_eq!(result["cases"][1]["vendor_median"], Value::Null);
        assert_eq!(
            result["weighted_applicable_vendor_aggregate"]["ratio_power"]["degree"],
            16
        );
        assert_eq!(result["all_checked_gates_pass"], true);
        assert_eq!(
            fs::read(fixture.output.join("observations.json")).unwrap(),
            fs::read(fixture.observations.join("observations.json")).unwrap()
        );
    }

    #[test]
    fn exact_counts_order_and_raw_arithmetic_fail_closed() {
        let fixture = make_fixture();
        mutate_observations(&fixture, |value| {
            value["cases"][0]["implementations"][0]["warmups"]
                .as_array_mut()
                .unwrap()
                .pop();
        });
        assert!(validate_policy_observations(&arguments(&fixture)).is_err());

        let fixture = make_fixture();
        mutate_observations(&fixture, |value| {
            value["cases"][0]["implementations"][1]["recorded"]
                .as_array_mut()
                .unwrap()
                .swap(0, 1);
        });
        assert!(validate_policy_observations(&arguments(&fixture)).is_err());

        let fixture = make_fixture();
        mutate_observations(&fixture, |value| {
            value["cases"][0]["implementations"][1]["recorded"][0]["elapsed_ns"] = json!(0);
        });
        assert!(validate_policy_observations(&arguments(&fixture)).is_err());

        let fixture = make_fixture();
        mutate_observations(&fixture, |value| {
            value["cases"][0]["implementations"][1]["recorded"][0]["summary"] = json!(7);
        });
        assert!(validate_policy_observations(&arguments(&fixture)).is_err());
    }

    #[test]
    fn even_sample_median_uses_the_two_exact_boundary_rates() {
        let fixture = make_fixture();
        mutate_observations(&fixture, |value| {
            let samples = value["cases"][0]["implementations"][1]["recorded"]
                .as_array_mut()
                .unwrap();
            for (index, sample) in samples.iter_mut().enumerate() {
                sample["elapsed_ns"] = json!(if index < 15 { 101 } else { 100 });
            }
        });
        validate_policy_observations(&arguments(&fixture)).unwrap();
        let result: Value =
            serde_json::from_slice(&fs::read(fixture.output.join("validation.json")).unwrap())
                .unwrap();
        assert_eq!(
            result["cases"][0]["ferric_median"]["numerator"],
            "218910891"
        );
        assert_eq!(result["cases"][0]["ferric_median"]["denominator"], "2");
        assert_eq!(result["cases"][0]["regression_gate_pass"], false);
        assert_eq!(result["all_checked_gates_pass"], false);
    }

    #[test]
    fn observation_protocol_roster_alias_and_noncanonical_inputs_fail_closed() {
        let fixture = make_fixture();
        fs::write(
            fixture.observations.join("protocol.json"),
            encode_canonical_document(&json!({"substituted": true})).unwrap(),
        )
        .unwrap();
        assert!(validate_policy_observations(&arguments(&fixture)).is_err());

        let fixture = make_fixture();
        fs::write(fixture.observations.join("extra.json"), b"{}\n").unwrap();
        assert!(validate_policy_observations(&arguments(&fixture)).is_err());

        let fixture = make_fixture();
        fs::remove_file(fixture.observations.join("protocol.json")).unwrap();
        fs::hard_link(
            fixture.observations.join("observations.json"),
            fixture.observations.join("protocol.json"),
        )
        .unwrap();
        assert!(validate_policy_observations(&arguments(&fixture)).is_err());

        let fixture = make_fixture();
        let path = fixture.observations.join("observations.json");
        let value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(validate_policy_observations(&arguments(&fixture)).is_err());
    }

    #[test]
    fn policy_admission_companion_holdout_and_regression_bindings_fail_closed() {
        for mutation in 0..5 {
            let fixture = make_fixture();
            mutate_observations(&fixture, |value| match mutation {
                0 => value["policy_sha256"] = json!(sha256_identity(b"wrong-policy")),
                1 => value["admission_sha256"] = json!(sha256_identity(b"wrong-admission")),
                2 => {
                    value["companion_sha256"]["timing"] = json!(sha256_identity(b"wrong-timing"));
                }
                3 => {
                    value["cases"][0]["implementations"][0]["holdout_member"]["id"] =
                        json!("calibration-a");
                }
                4 => {
                    value["cases"][0]["implementations"][0]
                        ["regression_measurement_roster_sha256"] =
                        json!(sha256_identity(b"wrong-reference-roster"));
                }
                _ => unreachable!(),
            });
            assert!(validate_policy_observations(&arguments(&fixture)).is_err());
        }
    }

    #[test]
    fn all_applicable_streams_must_share_one_exact_holdout_member() {
        let fixture = make_fixture();
        let holdout: Value =
            serde_json::from_slice(&fs::read(fixture.policy.join("holdout.json")).unwrap())
                .unwrap();
        let alternate = holdout["members"][1].clone();
        mutate_observations(&fixture, |value| {
            value["cases"][0]["implementations"][1]["holdout_member"] = alternate;
        });
        let error = validate_policy_observations(&arguments(&fixture)).unwrap_err();
        assert!(error.contains("one exact shared holdout member"));
    }

    #[test]
    fn switching_every_applicable_stream_to_another_admitted_member_breaks_order_binding() {
        let fixture = make_fixture();
        let holdout: Value =
            serde_json::from_slice(&fs::read(fixture.policy.join("holdout.json")).unwrap())
                .unwrap();
        let alternate = holdout["members"][1].clone();
        mutate_observations(&fixture, |value| {
            for implementation in value["cases"][0]["implementations"].as_array_mut().unwrap() {
                implementation["holdout_member"] = alternate.clone();
            }
        });
        let error = validate_policy_observations(&arguments(&fixture)).unwrap_err();
        assert!(error.contains("warmup order projection"));
    }

    #[test]
    fn observation_case_weight_bound_preserves_v1_admission_compatibility() {
        let fixture = make_fixture_with_case_weight(Some(MAX_EXACT_AGGREGATE_CASE_WEIGHT));
        validate_policy_observations(&arguments(&fixture)).unwrap();

        let fixture = make_fixture_with_case_weight(Some(MAX_EXACT_AGGREGATE_CASE_WEIGHT + 1));
        let held_policy = hold_validated_policy(&fixture.policy).unwrap();
        assert_eq!(
            fs::read(fixture.admission.join("admission.json")).unwrap(),
            held_policy.admission_bytes()
        );
        let error = validate_policy_observations(&arguments(&fixture)).unwrap_err();
        assert!(error.contains("exact aggregate-computability bound"));
    }

    #[test]
    fn inapplicable_vendor_cannot_supply_samples_or_budget() {
        let fixture = make_fixture();
        mutate_observations(&fixture, |value| {
            value["cases"][1]["implementations"][2]["warmups"] =
                value["cases"][1]["implementations"][1]["warmups"].clone();
        });
        assert!(validate_policy_observations(&arguments(&fixture)).is_err());

        let fixture = make_fixture();
        mutate_observations(&fixture, |value| {
            value["cases"][1]["implementations"][2]["tuning_budget"] =
                json!({"budget": 17, "unit": "candidate-builds"});
        });
        assert!(validate_policy_observations(&arguments(&fixture)).is_err());

        let fixture = make_fixture();
        let holdout: Value =
            serde_json::from_slice(&fs::read(fixture.policy.join("holdout.json")).unwrap())
                .unwrap();
        mutate_observations(&fixture, |value| {
            value["cases"][1]["implementations"][2]["holdout_member"] =
                holdout["members"][0].clone();
        });
        assert!(validate_policy_observations(&arguments(&fixture)).is_err());
    }

    #[test]
    fn held_input_mutation_and_no_replace_publication_fail_closed() {
        let fixture = make_fixture();
        let observations = fixture.observations.clone();
        let result = validate_policy_observations_with_hooks(
            &arguments(&fixture),
            || {
                let path = observations.join("observations.json");
                let mut bytes = fs::read(&path).unwrap();
                bytes[0] = b'[';
                fs::write(path, bytes).unwrap();
                Ok(())
            },
            |_| Ok(()),
            |_| Ok(()),
            || Ok(()),
            || Ok(()),
        );
        assert!(result.is_err());
        assert!(!fixture.output.exists());

        let fixture = make_fixture();
        let output = fixture.output.clone();
        let result = validate_policy_observations_with_hooks(
            &arguments(&fixture),
            || Ok(()),
            |_| Ok(()),
            |_| Ok(()),
            move || {
                fs::create_dir(&output).map_err(|error| error.to_string())?;
                Ok(())
            },
            || Ok(()),
        );
        assert!(result.is_err());
        assert!(fixture.output.is_dir());
    }

    #[test]
    fn staged_and_published_file_substitution_fail_custody() {
        let fixture = make_fixture();
        let replacement = Rc::new(RefCell::new(None));
        let replacement_path = Rc::clone(&replacement);
        let result = validate_policy_observations_with_hooks(
            &arguments(&fixture),
            || Ok(()),
            |_| Ok(()),
            move |staging| {
                let path = staging.join("validation.json");
                fs::remove_file(&path).map_err(|error| error.to_string())?;
                fs::write(&path, b"caller replacement\n").map_err(|error| error.to_string())?;
                replacement_path.replace(Some(path));
                Ok(())
            },
            || Ok(()),
            || Ok(()),
        );
        assert!(result.is_err());
        assert_eq!(
            fs::read(replacement.borrow().as_ref().unwrap()).unwrap(),
            b"caller replacement\n"
        );

        let fixture = make_fixture();
        let output = fixture.output.clone();
        let result = validate_policy_observations_with_hooks(
            &arguments(&fixture),
            || Ok(()),
            |_| Ok(()),
            |_| Ok(()),
            || Ok(()),
            move || {
                let path = output.join("validation.json");
                fs::remove_file(&path).map_err(|error| error.to_string())?;
                fs::write(path, b"published replacement\n").map_err(|error| error.to_string())?;
                Ok(())
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn mkdir_gap_substitution_adopts_only_exact_empty_directory_without_claimed_cleanup() {
        let fixture = make_fixture();
        let replacement = Rc::new(RefCell::new(None));
        let replacement_path = Rc::clone(&replacement);
        let result = validate_policy_observations_with_hooks(
            &arguments(&fixture),
            || Ok(()),
            move |staging| {
                fs::remove_dir(staging).map_err(|error| error.to_string())?;
                fs::write(staging, b"caller-owned").map_err(|error| error.to_string())?;
                fs::set_permissions(staging, fs::Permissions::from_mode(0o600))
                    .map_err(|error| error.to_string())?;
                replacement_path.replace(Some(staging.to_path_buf()));
                Ok(())
            },
            |_| Ok(()),
            || Ok(()),
            || Ok(()),
        );
        assert!(result.is_err());
        assert_eq!(
            fs::read(replacement.borrow().as_ref().unwrap()).unwrap(),
            b"caller-owned"
        );

        let fixture = make_fixture();
        let adopted = Rc::new(RefCell::new(None));
        let adopted_path = Rc::clone(&adopted);
        let result = validate_policy_observations_with_hooks(
            &arguments(&fixture),
            || Ok(()),
            move |staging| {
                fs::remove_dir(staging).map_err(|error| error.to_string())?;
                fs::create_dir(staging).map_err(|error| error.to_string())?;
                fs::set_permissions(staging, fs::Permissions::from_mode(0o700))
                    .map_err(|error| error.to_string())?;
                adopted_path.replace(Some(staging.to_path_buf()));
                Ok(())
            },
            |_| Err("stop after writing adopted staging".to_owned()),
            || Ok(()),
            || Ok(()),
        );
        assert!(result.is_err());
        let adopted = adopted.borrow();
        let adopted = adopted.as_ref().unwrap();
        assert!(adopted.is_dir());
        assert_eq!(fs::read_dir(adopted).unwrap().count(), 0);
    }

    #[test]
    fn published_directory_name_substitution_is_retained_and_reported() {
        let fixture = make_fixture();
        let output = fixture.output.clone();
        let moved = fixture.output.with_extension("held-original");
        let result = validate_policy_observations_with_hooks(
            &arguments(&fixture),
            || Ok(()),
            |_| Ok(()),
            |_| Ok(()),
            || Ok(()),
            move || {
                fs::rename(&output, &moved).map_err(|error| error.to_string())?;
                fs::create_dir(&output).map_err(|error| error.to_string())?;
                fs::set_permissions(&output, fs::Permissions::from_mode(0o700))
                    .map_err(|error| error.to_string())?;
                Ok(())
            },
        );
        assert!(result.is_err());
        assert!(fixture.output.is_dir());
        assert_eq!(fs::read_dir(&fixture.output).unwrap().count(), 0);
    }
}
