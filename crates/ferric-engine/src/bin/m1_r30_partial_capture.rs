//! Canonical partial m1.r30 cancellation capture protocol.
//!
//! This module deliberately accepts only the first physical cancellation
//! slice. It cannot mint benchmark evidence or represent the other four r30
//! cases.

use super::{
    canonical_bytes, exact_object, expect_string, field, hex_bytes, parse_canonical,
    require_sha256, sha256_hex, CaptureResult, CapturedExecutionV1, R30PhysicalCaptureBindingsV1,
    StagingOutput, Workload, MAX_POLLS, TARGET,
};
use ferric_engine::M1PhysicalRunnerV1;
use ferric_spec::{Identity, RequestId};
use serde_json::{json, Value};
use std::path::Path;

pub(super) const COMMAND: &str = "capture-r30-cancellation";
const CAPTURE_FORMAT: &str = "FERRIC-M1-R30-PARTIAL-CAPTURE-V2";
const PROTOCOL_FORMAT: &str = "FERRIC-M1-R30-PARTIAL-PROTOCOL-V2";
const AUTHORITY: &str = "ferric-physical-partial-capture-only";
const STATUS: &str = "partial-non-evidence";
const CASE: &str = "target-prefill-s1-t128-retirement-before-observation";
const WORKLOAD_FORMAT: &str = "FERRIC-M1-R30-CANCELLATION-WORKLOAD-V2";
const NONCLAIM: &str = "Authenticated Ferric scheduler retirement before positive physical completion observation and exact settlement for one fixed target-prefill S1/T128 workload only. Retirement is not GPU work preemption. This partial capture is not benchmark evidence, makes no hardware claim, does not establish general hardware correctness, does not cover canary, exhaustion, rollback, or injected device-fault cases, supplies none of the required external or independent validation evidence, and does not close m1.r30.";
const PROTOCOL_NONCLAIM: &str = "Partial fixed-workload physical cancellation capture only. Scheduler retirement before completion observation is not GPU work preemption. This protocol makes no hardware claim, does not establish canary integrity, exhaustion handling, rollback refinement, injected device-fault coverage, required external evidence, independent validation, hardware correctness, or close m1.r30.";

const EVENTS: &[&str] = &[
    "queue-published",
    "scheduler-retirement-requested",
    "precompletion-reclaim-rejected",
    "queue-completion-wait-satisfied",
    "signals-recycled",
    "physical-completion-readback-observed",
    "semantic-completion-joined",
    "exact-completion-settled",
    "retired-pages-released",
    "queue-released",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct RequestIdentityV1 {
    pub(super) generation: u32,
    pub(super) slot: u32,
}

impl From<RequestId> for RequestIdentityV1 {
    fn from(request: RequestId) -> Self {
        Self {
            generation: request.generation(),
            slot: request.slot(),
        }
    }
}

#[derive(Debug)]
pub(super) struct PreCompletionCancellationV1 {
    pub(super) in_flight_count: usize,
    pub(super) precompletion_reclaim_count: usize,
    pub(super) requests: Vec<RequestIdentityV1>,
    pub(super) retiring_count: usize,
}

#[derive(Debug)]
pub(super) struct CancellationSettlementV1 {
    pub(super) checked_records: usize,
    pub(super) completed_members: usize,
    pub(super) dispatch_generation: u64,
    pub(super) epoch: u64,
    pub(super) expected_target_pages: Vec<usize>,
    pub(super) expected_total_target_pages: usize,
    pub(super) externally_published_counts: Vec<u32>,
    pub(super) final_absent_count: usize,
    pub(super) in_flight_count: usize,
    pub(super) logical_accepted_counts: Vec<u32>,
    pub(super) plan_id: Identity,
    pub(super) precompletion_reclaim_count: usize,
    pub(super) released_pages: Vec<(usize, usize)>,
    pub(super) requests: Vec<RequestIdentityV1>,
    pub(super) retiring_count: usize,
    pub(super) terminal_members: usize,
    pub(super) total_released_pages: usize,
}

impl CancellationSettlementV1 {
    fn validate(&self) -> CaptureResult<()> {
        let count = self.requests.len();
        if count != 1 {
            return Err(
                "fixed partial cancellation capture requires exactly one request".to_owned(),
            );
        }
        if self.dispatch_generation == 0 || self.epoch == 0 || !self.plan_id.is_present() {
            return Err(
                "partial cancellation capture requires nonzero physical identities".to_owned(),
            );
        }
        if self.in_flight_count != count {
            return Err("cancellation was not requested for an exact in-flight roster".to_owned());
        }
        if self.retiring_count != count {
            return Err("cancellation did not move the exact roster into retirement".to_owned());
        }
        if self.precompletion_reclaim_count != 0 {
            return Err("a request reclaimed before exact completion settlement".to_owned());
        }
        if self.checked_records != count || self.completed_members != count {
            return Err("positive physical completion roster is incomplete".to_owned());
        }
        if self.terminal_members != count || self.final_absent_count != count {
            return Err("cancelled roster did not reach exact terminal settlement".to_owned());
        }
        if self.logical_accepted_counts.len() != count
            || self.externally_published_counts.len() != count
            || self.expected_target_pages.len() != count
            || self.released_pages.len() != count
        {
            return Err("completion or page-release roster length drifted".to_owned());
        }
        if self.logical_accepted_counts.iter().any(|count| *count != 1)
            || self
                .externally_published_counts
                .iter()
                .any(|count| *count != 1)
        {
            return Err("target-prefill completion counts are not exact".to_owned());
        }
        if self.expected_target_pages != [1] || self.released_pages != [(0, 1)] {
            return Err(
                "released pages differ from the fixed target-prefill S1 contract".to_owned(),
            );
        }
        let expected_total = self
            .expected_target_pages
            .iter()
            .try_fold(0usize, |total, expected| total.checked_add(*expected))
            .ok_or_else(|| "expected target-page count overflowed".to_owned())?;
        if expected_total != 1
            || expected_total != self.expected_total_target_pages
            || expected_total != self.total_released_pages
        {
            return Err("released-page total differs from the nonzero target contract".to_owned());
        }
        Ok(())
    }

    fn as_json(&self) -> Value {
        let requests = self
            .requests
            .iter()
            .map(|request| {
                json!({
                    "generation": request.generation,
                    "slot": request.slot,
                })
            })
            .collect::<Vec<_>>();
        let released_pages = self
            .released_pages
            .iter()
            .zip(&self.expected_target_pages)
            .map(|((draft, target), expected_target)| {
                json!({
                    "draft": draft,
                    "expected_target": expected_target,
                    "target": target,
                    "total": draft + target,
                })
            })
            .collect::<Vec<_>>();
        json!({
            "checked_records": self.checked_records,
            "completed_members": self.completed_members,
            "dispatch_generation": self.dispatch_generation,
            "epoch": self.epoch,
            "expected_total_target_pages": self.expected_total_target_pages,
            "events": EVENTS,
            "externally_published_counts": self.externally_published_counts,
            "final_absent_count": self.final_absent_count,
            "in_flight_count_before_retirement": self.in_flight_count,
            "logical_accepted_counts": self.logical_accepted_counts,
            "physical_completion_observed": true,
            "precompletion_reclaim_count": self.precompletion_reclaim_count,
            "released_pages": released_pages,
            "requests": requests,
            "retiring_count_after_request": self.retiring_count,
            "terminal_members": self.terminal_members,
            "total_released_pages": self.total_released_pages,
        })
    }
}

pub(super) struct CaptureManifestInputsV2<'a> {
    pub(super) capture: &'a super::CapturedOutput,
    pub(super) closure_sha256: &'a str,
    pub(super) environment_sha256: &'a str,
    pub(super) executable_sha256: &'a str,
    pub(super) gpu_unique_id: u64,
    pub(super) runner: &'a M1PhysicalRunnerV1,
    pub(super) settlement: &'a CancellationSettlementV1,
    pub(super) workload: &'a Workload,
}

pub(super) fn require_protocol() -> CaptureResult<()> {
    let protocol = protocol_bytes()?;
    let value = parse_canonical(&protocol, "partial r30 protocol")?;
    let object = exact_object(
        &value,
        &[
            "authority",
            "bundle_files",
            "case",
            "fixed_workload",
            "format",
            "hardware_claim",
            "lifecycle",
            "milestone",
            "nonclaim",
            "obligation_id",
            "required_complete_case_roster",
            "retirement_semantics",
            "status",
            "target",
        ],
        "partial r30 protocol",
    )?;
    expect_string(object, "authority", "ferric-m1-r30-partial-protocol-only")?;
    expect_string(object, "case", "cancellation")?;
    expect_string(object, "format", PROTOCOL_FORMAT)?;
    expect_string(object, "hardware_claim", "none")?;
    expect_string(object, "milestone", "M1")?;
    expect_string(object, "nonclaim", PROTOCOL_NONCLAIM)?;
    expect_string(object, "obligation_id", "m1.r30")?;
    expect_string(object, "status", STATUS)?;
    expect_string(object, "target", TARGET)?;
    if field(object, "fixed_workload")? != &fixed_workload_contract() {
        return Err("partial r30 fixed workload protocol drifted".to_owned());
    }
    if field(object, "retirement_semantics")?
        != &json!({
            "device_preemption": false,
            "operation": "scheduler-retirement-before-positive-physical-completion-observation",
        })
    {
        return Err("partial r30 retirement semantics drifted".to_owned());
    }
    if field(object, "bundle_files")?
        .as_array()
        .map(|files| files.iter().map(Value::as_str).collect::<Option<Vec<_>>>())
        != Some(Some(vec!["capture.json", "protocol.json"]))
    {
        return Err("partial r30 protocol bundle roster drifted".to_owned());
    }
    if field(object, "required_complete_case_roster")?
        .as_array()
        .map(|cases| cases.iter().map(Value::as_str).collect::<Option<Vec<_>>>())
        != Some(Some(vec![
            "canary",
            "cancellation",
            "exhaustion",
            "fault-injection",
            "rollback",
        ]))
    {
        return Err("partial r30 complete-case roster drifted".to_owned());
    }
    let lifecycle = field(object, "lifecycle")?
        .as_array()
        .ok_or_else(|| "partial r30 lifecycle must be an array".to_owned())?;
    if lifecycle
        .iter()
        .map(Value::as_str)
        .collect::<Option<Vec<_>>>()
        != Some(EVENTS.to_vec())
    {
        return Err("partial r30 protocol lifecycle ordering drifted".to_owned());
    }
    Ok(())
}

fn protocol_bytes() -> CaptureResult<Vec<u8>> {
    canonical_bytes(&json!({
        "authority": "ferric-m1-r30-partial-protocol-only",
        "bundle_files": ["capture.json", "protocol.json"],
        "case": "cancellation",
        "fixed_workload": fixed_workload_contract(),
        "format": PROTOCOL_FORMAT,
        "hardware_claim": "none",
        "lifecycle": EVENTS,
        "milestone": "M1",
        "nonclaim": PROTOCOL_NONCLAIM,
        "obligation_id": "m1.r30",
        "required_complete_case_roster": [
            "canary",
            "cancellation",
            "exhaustion",
            "fault-injection",
            "rollback",
        ],
        "retirement_semantics": {
            "device_preemption": false,
            "operation": "scheduler-retirement-before-positive-physical-completion-observation",
        },
        "status": STATUS,
        "target": TARGET,
    }))
}

fn fixed_workload_contract() -> Value {
    json!({
        "active_length": 1,
        "case": CASE,
        "context_length": 0,
        "format": WORKLOAD_FORMAT,
        "input_token": 1,
        "lane_count": 1,
        "max_polls": MAX_POLLS,
        "selection": "target-prefill-s1-t128",
    })
}

pub(super) fn protocol_sha256() -> CaptureResult<String> {
    Ok(sha256_hex(&protocol_bytes()?))
}

pub(super) fn manifest(inputs: CaptureManifestInputsV2<'_>) -> CaptureResult<Vec<u8>> {
    require_protocol()?;
    inputs.settlement.validate()?;
    validate_fixed_workload(inputs.workload)?;
    for identity in [
        inputs.closure_sha256,
        inputs.environment_sha256,
        inputs.executable_sha256,
    ] {
        require_sha256(identity)?;
    }
    if inputs.gpu_unique_id == 0 || !inputs.capture.device_id.is_present() {
        return Err("partial cancellation capture requires nonzero device identities".to_owned());
    }
    let capture = inputs.capture;
    let CapturedExecutionV1::OneShotPrefill {
        dispatch_generation,
        epoch,
    } = capture.execution
    else {
        return Err("fixed cancellation capture requires one-shot prefill execution".to_owned());
    };
    if dispatch_generation != inputs.settlement.dispatch_generation
        || epoch != inputs.settlement.epoch
        || capture.tokens.len() != 4
        || capture.logits_row_sha256.len() != 1
    {
        return Err("fixed cancellation output differs from settled physical execution".to_owned());
    }
    let protocol_sha256 = protocol_sha256()?;
    canonical_bytes(&json!({
        "authority": AUTHORITY,
        "case": CASE,
        "format": CAPTURE_FORMAT,
        "hardware_claim": "none",
        "identities": {
            "closure_document_sha256": inputs.closure_sha256,
            "device_identity_sha256": hex_identity(capture.device_id),
            "environment_sha256": inputs.environment_sha256,
            "executable_sha256": inputs.executable_sha256,
            "gpu_unique_id": inputs.gpu_unique_id,
            "kernel_artifact_manifest_sha256": hex_identity(inputs.runner.kernel_artifact_manifest_id()),
            "kernel_catalog_sha256": hex_identity(inputs.runner.kernel_catalog_id()),
            "plan_id_sha256": hex_identity(inputs.settlement.plan_id),
            "program_catalog_sha256": hex_identity(inputs.runner.program_catalog_id()),
            "runner_declaration_sha256": hex_identity(inputs.runner.declaration_id()),
        },
        "lifecycle": inputs.settlement.as_json(),
        "nonclaim": NONCLAIM,
        "obligation_id": "m1.r30",
        "protocol_sha256": protocol_sha256,
        "source_compact_sha256": hex_bytes(&capture.compact_sha256),
        "status": STATUS,
        "target": TARGET,
        "workload": {
            "active_length": 1,
            "context_length": 0,
            "input_payload_sha256": inputs.workload.input_sha256,
            "input_token": 1,
            "lane_count": 1,
            "max_polls": inputs.workload.max_polls,
            "selection": "target-prefill-s1-t128",
            "workload_sha256": sha256_hex(&inputs.workload.bytes),
        },
    }))
}

fn validate_fixed_workload(workload: &Workload) -> CaptureResult<()> {
    let expected_bytes = canonical_bytes(&fixed_workload_contract())?;
    let expected_input_sha256 = sha256_hex(&1_u32.to_le_bytes());
    if workload.bytes != expected_bytes
        || workload.input_path != Path::new("frozen-r30-cancellation-input-u32le")
        || workload.input_bytes != 4
        || workload.input_sha256 != expected_input_sha256
        || workload.kind != "prefill-s1-t128"
        || workload.lanes
            != [super::LaneInput {
                active_length: 1,
                context_length: 0,
            }]
        || u64::from(workload.max_polls) != MAX_POLLS
        || workload.selection.role != ferric_spec::Qwen3ModelRole::Target8B
        || workload.selection.mode != ferric_spec::Qwen3ExecutionMode::Prefill
        || workload.selection.bucket != ferric_spec::Qwen3PlanBucket::PrefillS1T128
    {
        return Err(
            "partial cancellation workload differs from the fixed S1/T128 contract".to_owned(),
        );
    }
    Ok(())
}

pub(super) fn publish(output: &Path, capture: &[u8]) -> CaptureResult<()> {
    validate_manifest(capture)?;
    let protocol = protocol_bytes()?;
    let mut staging = StagingOutput::create(output)?;
    staging.write("capture.json", capture)?;
    staging.write("protocol.json", &protocol)?;
    staging.publish_exact(&[("capture.json", capture), ("protocol.json", &protocol)])
}

pub(super) fn admit_persisted_bundle(
    capture: &[u8],
    protocol: &[u8],
) -> CaptureResult<R30PhysicalCaptureBindingsV1> {
    let expected_protocol = protocol_bytes()?;
    if protocol != expected_protocol {
        return Err("partial r30 cancellation protocol bytes drifted".to_owned());
    }
    validate_manifest(capture)?;
    let value = parse_canonical(capture, "persisted partial r30 cancellation capture")?;
    let root = value
        .as_object()
        .ok_or_else(|| "persisted partial r30 cancellation capture must be an object".to_owned())?;
    let identities = field(root, "identities")?.as_object().ok_or_else(|| {
        "persisted partial r30 cancellation identities must be an object".to_owned()
    })?;
    Ok(R30PhysicalCaptureBindingsV1 {
        device_identity_sha256: identity_string(identities, "device_identity_sha256")?.to_owned(),
        gpu_unique_id: field(identities, "gpu_unique_id")?
            .as_u64()
            .filter(|value| *value != 0)
            .ok_or_else(|| "persisted cancellation GPU identity is invalid".to_owned())?,
        kernel_artifact_manifest_sha256: identity_string(
            identities,
            "kernel_artifact_manifest_sha256",
        )?
        .to_owned(),
        program_catalog_sha256: identity_string(identities, "program_catalog_sha256")?.to_owned(),
        runner_declaration_sha256: identity_string(identities, "runner_declaration_sha256")?
            .to_owned(),
    })
}

fn identity_string<'a>(
    object: &'a serde_json::Map<String, Value>,
    name: &str,
) -> CaptureResult<&'a str> {
    let value = field(object, name)?
        .as_str()
        .ok_or_else(|| format!("persisted cancellation identity {name} must be a string"))?;
    require_sha256(value)?;
    Ok(value)
}

fn validate_manifest(bytes: &[u8]) -> CaptureResult<()> {
    let value = parse_canonical(bytes, "partial r30 capture")?;
    let object = exact_object(
        &value,
        &[
            "authority",
            "case",
            "format",
            "hardware_claim",
            "identities",
            "lifecycle",
            "nonclaim",
            "obligation_id",
            "protocol_sha256",
            "source_compact_sha256",
            "status",
            "target",
            "workload",
        ],
        "partial r30 capture",
    )?;
    expect_string(object, "authority", AUTHORITY)?;
    expect_string(object, "case", CASE)?;
    expect_string(object, "format", CAPTURE_FORMAT)?;
    expect_string(object, "hardware_claim", "none")?;
    expect_string(object, "nonclaim", NONCLAIM)?;
    expect_string(object, "obligation_id", "m1.r30")?;
    expect_string(object, "protocol_sha256", &protocol_sha256()?)?;
    expect_string(object, "status", STATUS)?;
    expect_string(object, "target", TARGET)?;
    for name in ["protocol_sha256", "source_compact_sha256"] {
        let identity = field(object, name)?
            .as_str()
            .ok_or_else(|| format!("partial r30 identity must be a string: {name}"))?;
        require_sha256(identity)?;
    }
    validate_identities(field(object, "identities")?)?;
    validate_workload(field(object, "workload")?)?;
    validate_lifecycle(field(object, "lifecycle")?)
}

fn validate_identities(value: &Value) -> CaptureResult<()> {
    let object = exact_object(
        value,
        &[
            "closure_document_sha256",
            "device_identity_sha256",
            "environment_sha256",
            "executable_sha256",
            "gpu_unique_id",
            "kernel_artifact_manifest_sha256",
            "kernel_catalog_sha256",
            "plan_id_sha256",
            "program_catalog_sha256",
            "runner_declaration_sha256",
        ],
        "partial r30 identities",
    )?;
    if field(object, "gpu_unique_id")?
        .as_u64()
        .is_none_or(|id| id == 0)
    {
        return Err("partial r30 GPU identity is invalid".to_owned());
    }
    for (name, value) in object {
        if name == "gpu_unique_id" {
            continue;
        }
        let identity = value
            .as_str()
            .ok_or_else(|| format!("partial r30 identity must be a string: {name}"))?;
        require_sha256(identity)?;
    }
    Ok(())
}

fn validate_workload(value: &Value) -> CaptureResult<()> {
    let object = exact_object(
        value,
        &[
            "active_length",
            "context_length",
            "input_payload_sha256",
            "input_token",
            "lane_count",
            "max_polls",
            "selection",
            "workload_sha256",
        ],
        "partial r30 workload",
    )?;
    let input_sha256 = field(object, "input_payload_sha256")?
        .as_str()
        .ok_or_else(|| "partial r30 workload input identity must be a string".to_owned())?;
    let workload_sha256 = field(object, "workload_sha256")?
        .as_str()
        .ok_or_else(|| "partial r30 workload identity must be a string".to_owned())?;
    require_sha256(input_sha256)?;
    require_sha256(workload_sha256)?;
    if field(object, "active_length")?.as_u64() != Some(1)
        || field(object, "context_length")?.as_u64() != Some(0)
        || field(object, "input_token")?.as_u64() != Some(1)
        || field(object, "lane_count")?.as_u64() != Some(1)
        || field(object, "max_polls")?.as_u64() != Some(MAX_POLLS)
    {
        return Err("partial r30 fixed workload dimensions drifted".to_owned());
    }
    if input_sha256 != sha256_hex(&1_u32.to_le_bytes())
        || workload_sha256 != sha256_hex(&canonical_bytes(&fixed_workload_contract())?)
    {
        return Err("partial r30 fixed workload identity drifted".to_owned());
    }
    expect_string(object, "selection", "target-prefill-s1-t128")
}

fn validate_lifecycle(value: &Value) -> CaptureResult<()> {
    let object = exact_object(
        value,
        &[
            "checked_records",
            "completed_members",
            "dispatch_generation",
            "epoch",
            "expected_total_target_pages",
            "events",
            "externally_published_counts",
            "final_absent_count",
            "in_flight_count_before_retirement",
            "logical_accepted_counts",
            "physical_completion_observed",
            "precompletion_reclaim_count",
            "released_pages",
            "requests",
            "retiring_count_after_request",
            "terminal_members",
            "total_released_pages",
        ],
        "partial r30 lifecycle",
    )?;
    let events = field(object, "events")?
        .as_array()
        .ok_or_else(|| "partial r30 lifecycle events must be an array".to_owned())?;
    if events.iter().map(Value::as_str).collect::<Option<Vec<_>>>() != Some(EVENTS.to_vec()) {
        return Err("partial r30 capture lifecycle ordering drifted".to_owned());
    }
    if field(object, "physical_completion_observed")?.as_bool() != Some(true) {
        return Err("partial r30 capture omitted positive physical completion".to_owned());
    }
    if field(object, "precompletion_reclaim_count")?.as_u64() != Some(0) {
        return Err("partial r30 capture reports precompletion reclamation".to_owned());
    }
    let requests = field(object, "requests")?
        .as_array()
        .ok_or_else(|| "partial r30 request roster must be an array".to_owned())?;
    let request_count = requests.len();
    if request_count != 1 {
        return Err("fixed partial r30 request roster must contain exactly one member".to_owned());
    }
    let mut request_ids = std::collections::BTreeSet::new();
    for request in requests {
        let request = exact_object(
            request,
            &["generation", "slot"],
            "partial r30 request identity",
        )?;
        let generation = field(request, "generation")?
            .as_u64()
            .filter(|generation| *generation != 0 && u32::try_from(*generation).is_ok())
            .ok_or_else(|| "partial r30 request generation is invalid".to_owned())?;
        let slot = field(request, "slot")?
            .as_u64()
            .filter(|slot| u32::try_from(*slot).is_ok())
            .ok_or_else(|| "partial r30 request slot is invalid".to_owned())?;
        if !request_ids.insert((slot, generation)) {
            return Err("partial r30 request roster repeats an identity".to_owned());
        }
    }
    for name in [
        "checked_records",
        "completed_members",
        "final_absent_count",
        "in_flight_count_before_retirement",
        "retiring_count_after_request",
        "terminal_members",
    ] {
        if field(object, name)?.as_u64() != u64::try_from(request_count).ok() {
            return Err(format!("partial r30 lifecycle count drifted: {name}"));
        }
    }
    for name in ["dispatch_generation", "epoch"] {
        if field(object, name)?.as_u64().is_none_or(|value| value == 0) {
            return Err(format!("partial r30 lifecycle identity is invalid: {name}"));
        }
    }
    for name in ["externally_published_counts", "logical_accepted_counts"] {
        let counts = field(object, name)?
            .as_array()
            .ok_or_else(|| format!("partial r30 {name} must be an array"))?;
        if counts.len() != request_count || counts.iter().any(|count| count.as_u64() != Some(1)) {
            return Err(format!("partial r30 {name} roster drifted"));
        }
    }
    let releases = field(object, "released_pages")?
        .as_array()
        .ok_or_else(|| "partial r30 released pages must be an array".to_owned())?;
    if releases.len() != request_count {
        return Err("partial r30 released-page roster drifted".to_owned());
    }
    let mut released_total = 0_u64;
    for release in releases {
        let release = exact_object(
            release,
            &["draft", "expected_target", "target", "total"],
            "partial r30 released-page entry",
        )?;
        let draft = field(release, "draft")?
            .as_u64()
            .ok_or_else(|| "partial r30 draft release count is invalid".to_owned())?;
        let target = field(release, "target")?
            .as_u64()
            .ok_or_else(|| "partial r30 target release count is invalid".to_owned())?;
        let expected_target = field(release, "expected_target")?
            .as_u64()
            .filter(|expected| *expected != 0)
            .ok_or_else(|| "partial r30 expected target release count is invalid".to_owned())?;
        let total = field(release, "total")?
            .as_u64()
            .ok_or_else(|| "partial r30 total release count is invalid".to_owned())?;
        if draft != 0
            || target != 1
            || expected_target != 1
            || total != 1
            || draft.checked_add(target) != Some(total)
        {
            return Err("partial r30 released-page entry differs from target contract".to_owned());
        }
        released_total = released_total
            .checked_add(total)
            .ok_or_else(|| "partial r30 released-page total overflowed".to_owned())?;
    }
    if released_total != 1
        || field(object, "expected_total_target_pages")?.as_u64() != Some(released_total)
        || field(object, "total_released_pages")?.as_u64() != Some(released_total)
    {
        return Err("partial r30 released-page aggregate drifted".to_owned());
    }
    Ok(())
}

fn hex_identity(identity: Identity) -> String {
    hex_bytes(identity.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Map;
    use std::ffi::OsString;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct Temporary(PathBuf);

    impl Temporary {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "ferric-m1-r30-partial-test.{}.{nonce}",
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

    fn lifecycle() -> Value {
        json!({
            "checked_records": 1,
            "completed_members": 1,
            "dispatch_generation": 1,
            "epoch": 1,
            "expected_total_target_pages": 1,
            "events": EVENTS,
            "externally_published_counts": [1],
            "final_absent_count": 1,
            "in_flight_count_before_retirement": 1,
            "logical_accepted_counts": [1],
            "physical_completion_observed": true,
            "precompletion_reclaim_count": 0,
            "released_pages": [{"draft": 0, "expected_target": 1, "target": 1, "total": 1}],
            "requests": [{"generation": 1, "slot": 0}],
            "retiring_count_after_request": 1,
            "terminal_members": 1,
            "total_released_pages": 1,
        })
    }

    fn identities() -> Value {
        let identity = "11".repeat(32);
        json!({
            "closure_document_sha256": identity,
            "device_identity_sha256": identity,
            "environment_sha256": identity,
            "executable_sha256": identity,
            "gpu_unique_id": 1,
            "kernel_artifact_manifest_sha256": identity,
            "kernel_catalog_sha256": identity,
            "plan_id_sha256": identity,
            "program_catalog_sha256": identity,
            "runner_declaration_sha256": identity,
        })
    }

    fn workload() -> Value {
        json!({
            "active_length": 1,
            "context_length": 0,
            "input_payload_sha256": sha256_hex(&1_u32.to_le_bytes()),
            "input_token": 1,
            "lane_count": 1,
            "max_polls": MAX_POLLS,
            "selection": "target-prefill-s1-t128",
            "workload_sha256": sha256_hex(&canonical_bytes(&fixed_workload_contract()).unwrap()),
        })
    }

    fn capture(lifecycle: Value) -> Vec<u8> {
        canonical_bytes(&json!({
            "authority": AUTHORITY,
            "case": CASE,
            "format": CAPTURE_FORMAT,
            "hardware_claim": "none",
            "identities": identities(),
            "lifecycle": lifecycle,
            "nonclaim": NONCLAIM,
            "obligation_id": "m1.r30",
            "protocol_sha256": protocol_sha256().unwrap(),
            "source_compact_sha256": "11".repeat(32),
            "status": STATUS,
            "target": TARGET,
            "workload": workload(),
        }))
        .unwrap()
    }

    #[test]
    fn protocol_is_canonical_and_partial() {
        require_protocol().unwrap();
        let protocol = protocol_bytes().unwrap();
        let manifest = std::env::var_os("CARGO_MANIFEST_DIR").unwrap();
        let checked_in =
            fs::read(PathBuf::from(manifest).join("src/bin/ferric-m1-r30-partial-protocol.json"))
                .unwrap();
        assert_eq!(protocol, checked_in);
        assert!(String::from_utf8_lossy(&protocol).contains("partial-non-evidence"));
        assert!(String::from_utf8_lossy(&protocol).contains("does not establish"));
        assert!(
            EVENTS
                .iter()
                .position(|event| *event == "scheduler-retirement-requested")
                < EVENTS
                    .iter()
                    .position(|event| *event == "physical-completion-readback-observed")
        );
    }

    #[test]
    fn persisted_bundle_admission_recovers_only_checked_runtime_bindings() {
        let bytes = capture(lifecycle());
        let admitted = admit_persisted_bundle(&bytes, &protocol_bytes().unwrap()).unwrap();
        assert_eq!(admitted.device_identity_sha256, "11".repeat(32));
        assert_eq!(admitted.gpu_unique_id, 1);
        assert_eq!(admitted.kernel_artifact_manifest_sha256, "11".repeat(32));

        let mut wrong_protocol = protocol_bytes().unwrap();
        wrong_protocol.push(b'\n');
        assert!(admit_persisted_bundle(&bytes, &wrong_protocol).is_err());
    }

    #[test]
    fn parser_rejects_reordered_lifecycle_and_premature_reclaim() {
        let mut reordered = lifecycle();
        reordered["events"].as_array_mut().unwrap().swap(4, 6);
        assert!(validate_manifest(&capture(reordered)).is_err());

        let mut reclaimed = lifecycle();
        reclaimed["precompletion_reclaim_count"] = json!(1);
        assert!(validate_manifest(&capture(reclaimed)).is_err());
    }

    #[test]
    fn settlement_rejects_late_or_incomplete_cancellation() {
        let mut settlement = CancellationSettlementV1 {
            checked_records: 1,
            completed_members: 1,
            dispatch_generation: 1,
            epoch: 1,
            expected_target_pages: vec![1],
            expected_total_target_pages: 1,
            externally_published_counts: vec![1],
            final_absent_count: 1,
            in_flight_count: 0,
            logical_accepted_counts: vec![1],
            plan_id: Identity::new([7; 32]),
            precompletion_reclaim_count: 0,
            released_pages: vec![(0, 1)],
            requests: vec![RequestIdentityV1 {
                generation: 1,
                slot: 0,
            }],
            retiring_count: 1,
            terminal_members: 1,
            total_released_pages: 1,
        };
        assert!(settlement.validate().is_err());
        settlement.in_flight_count = 1;
        settlement.precompletion_reclaim_count = 1;
        assert!(settlement.validate().is_err());
    }

    #[test]
    fn parser_rejects_zero_or_mismatched_target_page_release() {
        let mut zero = lifecycle();
        zero["expected_total_target_pages"] = json!(0);
        zero["total_released_pages"] = json!(0);
        zero["released_pages"][0] =
            json!({"draft": 0, "expected_target": 0, "target": 0, "total": 0});
        assert!(validate_manifest(&capture(zero)).is_err());

        let mut mismatched = lifecycle();
        mismatched["released_pages"][0]["target"] = json!(2);
        mismatched["released_pages"][0]["total"] = json!(2);
        mismatched["total_released_pages"] = json!(2);
        assert!(validate_manifest(&capture(mismatched)).is_err());
    }

    #[test]
    fn publisher_is_no_replace_and_has_exact_roster() {
        let temporary = Temporary::new();
        let output = temporary.0.join("partial.bundle");
        let bytes = capture(lifecycle());
        publish(&output, &bytes).unwrap();
        let mut names = fs::read_dir(&output)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<Vec<OsString>>();
        names.sort();
        assert_eq!(
            names,
            [
                OsString::from("capture.json"),
                OsString::from("protocol.json")
            ]
        );
        assert!(publish(&output, &bytes).is_err());
    }

    #[test]
    fn publisher_rejects_mutated_or_extra_staged_files() {
        let capture_bytes = b"capture\n";
        let protocol_bytes = b"protocol\n";

        let mutation = Temporary::new();
        let mutation_output = mutation.0.join("partial.bundle");
        let mut staging = StagingOutput::create(&mutation_output).unwrap();
        staging.write("capture.json", capture_bytes).unwrap();
        staging.write("protocol.json", protocol_bytes).unwrap();
        let staging_root = mutation.0.join(&staging.staging_name);
        fs::write(staging_root.join("capture.json"), b"mutated\n").unwrap();
        assert!(staging
            .publish_exact(&[
                ("capture.json", capture_bytes),
                ("protocol.json", protocol_bytes),
            ])
            .is_err());
        assert!(!mutation_output.exists());

        let extra = Temporary::new();
        let extra_output = extra.0.join("partial.bundle");
        let mut staging = StagingOutput::create(&extra_output).unwrap();
        staging.write("capture.json", capture_bytes).unwrap();
        staging.write("protocol.json", protocol_bytes).unwrap();
        let staging_root = extra.0.join(&staging.staging_name);
        fs::write(staging_root.join("extra.json"), b"{}\n").unwrap();
        assert!(staging
            .publish_exact(&[
                ("capture.json", capture_bytes),
                ("protocol.json", protocol_bytes),
            ])
            .is_err());
        assert!(!extra_output.exists());
    }

    #[test]
    fn publisher_rejects_replaced_or_chmod_staged_custody() {
        let capture_bytes = b"capture\n";
        let protocol_bytes = b"protocol\n";
        let expected = [
            ("capture.json", capture_bytes.as_slice()),
            ("protocol.json", protocol_bytes.as_slice()),
        ];

        let replacement = Temporary::new();
        let replacement_output = replacement.0.join("partial.bundle");
        let mut staging = StagingOutput::create(&replacement_output).unwrap();
        staging.write("capture.json", capture_bytes).unwrap();
        staging.write("protocol.json", protocol_bytes).unwrap();
        let staging_root = replacement.0.join(&staging.staging_name);
        let capture_path = staging_root.join("capture.json");
        fs::rename(&capture_path, replacement.0.join("displaced.json")).unwrap();
        fs::write(&capture_path, capture_bytes).unwrap();
        fs::set_permissions(&capture_path, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(staging.publish_exact(&expected).is_err());
        assert!(!replacement_output.exists());

        let file_chmod = Temporary::new();
        let file_chmod_output = file_chmod.0.join("partial.bundle");
        let mut staging = StagingOutput::create(&file_chmod_output).unwrap();
        staging.write("capture.json", capture_bytes).unwrap();
        staging.write("protocol.json", protocol_bytes).unwrap();
        let staging_root = file_chmod.0.join(&staging.staging_name);
        fs::set_permissions(
            staging_root.join("capture.json"),
            fs::Permissions::from_mode(0o400),
        )
        .unwrap();
        assert!(staging.publish_exact(&expected).is_err());
        assert!(!file_chmod_output.exists());

        let directory_chmod = Temporary::new();
        let directory_chmod_output = directory_chmod.0.join("partial.bundle");
        let mut staging = StagingOutput::create(&directory_chmod_output).unwrap();
        staging.write("capture.json", capture_bytes).unwrap();
        staging.write("protocol.json", protocol_bytes).unwrap();
        let staging_root = directory_chmod.0.join(&staging.staging_name);
        fs::set_permissions(&staging_root, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(staging.publish_exact(&expected).is_err());
        assert!(!directory_chmod_output.exists());
    }

    #[test]
    fn parser_rejects_evidence_promotion() {
        let bytes = capture(lifecycle());
        let mut value = parse_canonical(&bytes, "fixture").unwrap();
        let object: &mut Map<String, Value> = value.as_object_mut().unwrap();
        object.insert("status".to_owned(), json!("observed"));
        assert!(validate_manifest(&canonical_bytes(&value).unwrap()).is_err());

        let mut value = parse_canonical(&bytes, "fixture").unwrap();
        value["hardware_claim"] = json!("validated");
        assert!(validate_manifest(&canonical_bytes(&value).unwrap()).is_err());

        let mut value = parse_canonical(&bytes, "fixture").unwrap();
        value["workload"]["input_token"] = json!(2);
        assert!(validate_manifest(&canonical_bytes(&value).unwrap()).is_err());
    }
}
