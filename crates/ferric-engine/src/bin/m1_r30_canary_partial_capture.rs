//! Canonical partial m1.r30 guarded physical K7 capture.
//!
//! This producer accepts only the retained output of one exact target-prefill
//! S1 lifecycle. It binds two adjacent 64-byte guards around the 120-byte K7
//! output and deliberately grants no broader memory-safety or evidence claim.

use super::{
    canonical_bytes, decode_identity, exact_object, expect_string, field, hex_bytes,
    parse_canonical, require_sha256, sha256_array, CaptureResult, R30PhysicalCaptureBindingsV1,
    StagingOutput, TARGET,
};
use ferric_engine::{
    CheckedCompletionSemantics, M1PhysicalRunnerV1, M1ReleasedDeviceKvMemberV1,
    M1ReleasedQueueTeardownSuccessV1, M1_COMPLETION_CANARY_GUARD_BYTES_V1,
    M1_COMPLETION_CANARY_PREFIX_BYTE_V1, M1_COMPLETION_CANARY_SUFFIX_BYTE_V1,
};
use ferric_spec::Identity;
use serde_json::{json, Map, Value};
use std::path::Path;

pub(super) const COMMAND: &str = "capture-r30-canary";
const CAPTURE_FORMAT: &str = "FERRIC-M1-R30-CANARY-PARTIAL-CAPTURE-V1";
const PROTOCOL_FORMAT: &str = "FERRIC-M1-R30-CANARY-PARTIAL-PROTOCOL-V1";
const STATUS: &str = "partial-non-evidence";
const CASE: &str = "target-prefill-s1-k7-adjacent-guards";
const INTERIOR_BYTES: u64 = 120;
const SNAPSHOT_BYTES: u64 = 248;
const SUFFIX_RELATIVE_OFFSET: u64 = 184;
const NONCLAIM: &str = "One Ferric-owned target-prefill S1 dispatch checked only the adjacent 64-byte prefix and suffix guards enclosing its single 120-byte K7 output. This partial capture does not establish K1-K6 bounds, general out-of-bounds safety, cancellation, exhaustion, rollback, injected-fault handling, external or independent validation, hardware or numerical correctness, evidence, performance, qualification, or m1.r30/M1 closure.";
const PROTOCOL_NONCLAIM: &str = "Partial one-case adjacent-guard protocol only: exactly 64 initialized bytes before and after one target-prefill S1 K7 output are checked after one completed generation. It grants no K1-K6 or general memory-safety proof, cancellation, exhaustion, rollback, fault-injection, independence, evidence, hardware-correctness, performance, qualification, or m1.r30/M1 closure authority.";

const EVENTS: &[&str] = &[
    "initialized-guarded-host-backing",
    "bound-single-k7-interior",
    "published-target-prefill-s1",
    "completed-and-recycled-generation",
    "copied-enclosing-snapshot-once",
    "validated-prefix-and-suffix-guards",
    "checked-existing-k7-semantics",
    "settled-single-engine-member",
    "released-single-target-page",
    "destroyed-physical-queue",
    "canonical-publication-ready",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ExpectedBindingsV1 {
    completion_epoch: u64,
    data_index: usize,
    device_id: [u8; 32],
    dispatch_generation: u64,
    emitted_token: u32,
    gpu_unique_id: u64,
    interior_offset: u64,
    interior_sha256: [u8; 32],
    kernel_catalog: [u8; 32],
    kernel_manifest: [u8; 32],
    plan_id: [u8; 32],
    prefix_sha256: [u8; 32],
    program_catalog: [u8; 32],
    protocol_sha256: [u8; 32],
    request_generation: u32,
    request_slot: u32,
    runner_declaration: [u8; 32],
    snapshot_offset: u64,
    snapshot_sha256: [u8; 32],
    suffix_sha256: [u8; 32],
}

pub(super) struct ClosedCaptureInputsV1<'a> {
    pub(super) closed: &'a M1ReleasedQueueTeardownSuccessV1,
    pub(super) device_id: Identity,
    pub(super) gpu_unique_id: u64,
    pub(super) runner: &'a M1PhysicalRunnerV1,
}

pub(super) struct CaptureArtifactV1 {
    bytes: Vec<u8>,
    expected: ExpectedBindingsV1,
}

impl CaptureArtifactV1 {
    pub(super) fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

pub(super) fn manifest(inputs: ClosedCaptureInputsV1<'_>) -> CaptureResult<CaptureArtifactV1> {
    require_protocol()?;
    let checked = inputs.closed.checked();
    let summary = checked
        .completion_canary()
        .ok_or_else(|| "partial canary checked output has no guarded snapshot".to_owned())?;
    let [record] = checked.records() else {
        return Err("partial canary requires exactly one checked S1 record".to_owned());
    };
    let CheckedCompletionSemantics::DirectFinalRow { token } = record.semantics() else {
        return Err("partial canary requires settled direct final-row semantics".to_owned());
    };
    let raw = record.record();
    let [member] = inputs.closed.members() else {
        return Err("partial canary requires exactly one released member".to_owned());
    };
    if !matches!(member, M1ReleasedDeviceKvMemberV1::Terminal(_))
        || member.request() != raw.request
        || checked.selection().role != ferric_spec::Qwen3ModelRole::Target8B
        || checked.selection().mode != ferric_spec::Qwen3ExecutionMode::Prefill
        || checked.selection().bucket != ferric_spec::Qwen3PlanBucket::PrefillS1T128
        || checked.epoch() != raw.epoch
        || checked.dispatch_generation() != summary.dispatch_generation()
        || checked.data_index() != summary.data_index()
        || checked.offset_bytes() != summary.interior_offset_bytes()
        || checked.extent_bytes() != INTERIOR_BYTES
        || checked.raw_sha256() != &summary.interior_sha256()
        || summary.snapshot_extent_bytes() != SNAPSHOT_BYTES
        || summary.interior_extent_bytes() != INTERIOR_BYTES
        || summary.interior_offset_bytes()
            != summary
                .snapshot_offset_bytes()
                .checked_add(M1_COMPLETION_CANARY_GUARD_BYTES_V1)
                .ok_or_else(|| "partial canary absolute interior offset overflowed".to_owned())?
        || summary.dispatch_generation() == 0
        || inputs.gpu_unique_id == 0
        || !inputs.device_id.is_present()
        || raw.request.generation() == 0
        || raw.emitted_token_count != 1
        || raw.emitted_tokens[0] != token
        || inputs.closed.completed_members() != 1
        || inputs.closed.logical_accepted_counts() != [1]
        || inputs.closed.externally_published_counts() != [1]
        || inputs.closed.total_released() != 1
        || inputs.closed.release_counts().len() != 1
        || inputs.closed.release_counts()[0].draft() != 0
        || inputs.closed.release_counts()[0].target() != 1
        || inputs.closed.queue_release().dispatch_generation() != summary.dispatch_generation()
    {
        return Err("partial canary retained lifecycle or layout custody drifted".to_owned());
    }

    let initialized_prefix = [M1_COMPLETION_CANARY_PREFIX_BYTE_V1; 64];
    let initialized_interior = [0_u8; 120];
    let initialized_suffix = [M1_COMPLETION_CANARY_SUFFIX_BYTE_V1; 64];
    let initialized_prefix_sha256 = sha256_array(&initialized_prefix);
    let initialized_interior_sha256 = sha256_array(&initialized_interior);
    let initialized_suffix_sha256 = sha256_array(&initialized_suffix);
    if summary.prefix_sha256() != initialized_prefix_sha256
        || summary.suffix_sha256() != initialized_suffix_sha256
    {
        return Err("partial canary completed guard digest differs from initialization".to_owned());
    }
    let protocol = protocol_bytes()?;
    let expected = ExpectedBindingsV1 {
        completion_epoch: checked.epoch().value(),
        data_index: checked.data_index(),
        device_id: *inputs.device_id.as_bytes(),
        dispatch_generation: checked.dispatch_generation(),
        emitted_token: token,
        gpu_unique_id: inputs.gpu_unique_id,
        interior_offset: summary.interior_offset_bytes(),
        interior_sha256: summary.interior_sha256(),
        kernel_catalog: *inputs.runner.kernel_catalog_id().as_bytes(),
        kernel_manifest: *inputs.runner.kernel_artifact_manifest_id().as_bytes(),
        plan_id: *raw.plan_id.as_bytes(),
        prefix_sha256: summary.prefix_sha256(),
        program_catalog: *inputs.runner.program_catalog_id().as_bytes(),
        protocol_sha256: sha256_array(&protocol),
        request_generation: raw.request.generation(),
        request_slot: raw.request.slot(),
        runner_declaration: *inputs.runner.declaration_id().as_bytes(),
        snapshot_offset: summary.snapshot_offset_bytes(),
        snapshot_sha256: summary.snapshot_sha256(),
        suffix_sha256: summary.suffix_sha256(),
    };
    let value = capture_value(
        &expected,
        initialized_prefix_sha256,
        initialized_interior_sha256,
        initialized_suffix_sha256,
    );
    let bytes = canonical_bytes(&value)?;
    validate_manifest(&bytes, &expected)?;
    Ok(CaptureArtifactV1 { bytes, expected })
}

fn capture_value(
    expected: &ExpectedBindingsV1,
    initialized_prefix_sha256: [u8; 32],
    initialized_interior_sha256: [u8; 32],
    initialized_suffix_sha256: [u8; 32],
) -> Value {
    json!({
        "authority": "ferric-physical-partial-capture-only",
        "case": CASE,
        "format": CAPTURE_FORMAT,
        "identities": {
            "device_id_sha256": hex_bytes(&expected.device_id),
            "gpu_unique_id": expected.gpu_unique_id,
            "kernel_catalog_sha256": hex_bytes(&expected.kernel_catalog),
            "kernel_manifest_sha256": hex_bytes(&expected.kernel_manifest),
            "program_catalog_sha256": hex_bytes(&expected.program_catalog),
            "protocol_sha256": hex_bytes(&expected.protocol_sha256),
            "runner_declaration_sha256": hex_bytes(&expected.runner_declaration),
        },
        "layout": {
            "interior": {
                "absolute_offset_bytes": expected.interior_offset,
                "completed_sha256": hex_bytes(&expected.interior_sha256),
                "extent_bytes": INTERIOR_BYTES,
                "initialized_byte": 0,
                "initialized_sha256": hex_bytes(&initialized_interior_sha256),
                "relative_offset_bytes": M1_COMPLETION_CANARY_GUARD_BYTES_V1,
            },
            "prefix_guard": {
                "completed_sha256": hex_bytes(&expected.prefix_sha256),
                "extent_bytes": M1_COMPLETION_CANARY_GUARD_BYTES_V1,
                "initialized_byte": M1_COMPLETION_CANARY_PREFIX_BYTE_V1,
                "initialized_sha256": hex_bytes(&initialized_prefix_sha256),
                "relative_offset_bytes": 0,
                "unchanged": true,
            },
            "snapshot": {
                "absolute_offset_bytes": expected.snapshot_offset,
                "completed_sha256": hex_bytes(&expected.snapshot_sha256),
                "extent_bytes": SNAPSHOT_BYTES,
            },
            "suffix_guard": {
                "completed_sha256": hex_bytes(&expected.suffix_sha256),
                "extent_bytes": M1_COMPLETION_CANARY_GUARD_BYTES_V1,
                "initialized_byte": M1_COMPLETION_CANARY_SUFFIX_BYTE_V1,
                "initialized_sha256": hex_bytes(&initialized_suffix_sha256),
                "relative_offset_bytes": SUFFIX_RELATIVE_OFFSET,
                "unchanged": true,
            },
        },
        "milestone": "M1",
        "nonclaim": NONCLAIM,
        "obligation_id": "m1.r30",
        "result": {
            "adjacent_guard_bytes_checked": 128,
            "completed_members": 1,
            "emitted_token": expected.emitted_token,
            "events": EVENTS,
            "externally_published_count": 1,
            "guard_corruptions": 0,
            "logical_accepted_count": 1,
            "queue_destroyed": true,
            "released_draft_pages": 0,
            "released_target_pages": 1,
            "single_k7_case_only": true,
        },
        "status": STATUS,
        "target": TARGET,
        "trace": {
            "completion_epoch": expected.completion_epoch,
            "data_index": expected.data_index,
            "dispatch_generation": expected.dispatch_generation,
            "plan_id_sha256": hex_bytes(&expected.plan_id),
            "request_generation": expected.request_generation,
            "request_slot": expected.request_slot,
        },
    })
}

pub(super) fn publish(output: &Path, capture: CaptureArtifactV1) -> CaptureResult<()> {
    validate_manifest(&capture.bytes, &capture.expected)?;
    let protocol = protocol_bytes()?;
    let mut staging = StagingOutput::create(output)?;
    staging.write("capture.json", &capture.bytes)?;
    staging.write("protocol.json", &protocol)?;
    staging.publish_exact(&[
        ("capture.json", capture.bytes.as_slice()),
        ("protocol.json", protocol.as_slice()),
    ])
}

pub(super) fn admit_persisted_bundle(
    capture: &[u8],
    protocol: &[u8],
) -> CaptureResult<R30PhysicalCaptureBindingsV1> {
    let expected_protocol = protocol_bytes()?;
    if protocol != expected_protocol {
        return Err("partial r30 canary protocol bytes drifted".to_owned());
    }
    let value = parse_canonical(capture, "persisted partial r30 canary capture")?;
    let root = value
        .as_object()
        .ok_or_else(|| "persisted partial r30 canary capture must be an object".to_owned())?;
    let identities = field(root, "identities")?
        .as_object()
        .ok_or_else(|| "persisted partial r30 canary identities must be an object".to_owned())?;
    let layout = field(root, "layout")?
        .as_object()
        .ok_or_else(|| "persisted partial r30 canary layout must be an object".to_owned())?;
    let interior = field(layout, "interior")?
        .as_object()
        .ok_or_else(|| "persisted partial r30 canary interior must be an object".to_owned())?;
    let prefix = field(layout, "prefix_guard")?
        .as_object()
        .ok_or_else(|| "persisted partial r30 canary prefix must be an object".to_owned())?;
    let snapshot = field(layout, "snapshot")?
        .as_object()
        .ok_or_else(|| "persisted partial r30 canary snapshot must be an object".to_owned())?;
    let suffix = field(layout, "suffix_guard")?
        .as_object()
        .ok_or_else(|| "persisted partial r30 canary suffix must be an object".to_owned())?;
    let result = field(root, "result")?
        .as_object()
        .ok_or_else(|| "persisted partial r30 canary result must be an object".to_owned())?;
    let trace = field(root, "trace")?
        .as_object()
        .ok_or_else(|| "persisted partial r30 canary trace must be an object".to_owned())?;
    let expected = ExpectedBindingsV1 {
        completion_epoch: persisted_u64(trace, "completion_epoch")?,
        data_index: usize::try_from(persisted_u64(trace, "data_index")?)
            .map_err(|_| "persisted canary data index does not fit usize".to_owned())?,
        device_id: persisted_digest(identities, "device_id_sha256")?,
        dispatch_generation: persisted_u64(trace, "dispatch_generation")?,
        emitted_token: u32::try_from(persisted_u64(result, "emitted_token")?)
            .map_err(|_| "persisted canary emitted token does not fit u32".to_owned())?,
        gpu_unique_id: persisted_u64(identities, "gpu_unique_id")?,
        interior_offset: persisted_u64(interior, "absolute_offset_bytes")?,
        interior_sha256: persisted_digest(interior, "completed_sha256")?,
        kernel_catalog: persisted_digest(identities, "kernel_catalog_sha256")?,
        kernel_manifest: persisted_digest(identities, "kernel_manifest_sha256")?,
        plan_id: persisted_digest(trace, "plan_id_sha256")?,
        prefix_sha256: persisted_digest(prefix, "completed_sha256")?,
        program_catalog: persisted_digest(identities, "program_catalog_sha256")?,
        protocol_sha256: sha256_array(protocol),
        request_generation: u32::try_from(persisted_u64(trace, "request_generation")?)
            .map_err(|_| "persisted canary request generation does not fit u32".to_owned())?,
        request_slot: u32::try_from(persisted_u64(trace, "request_slot")?)
            .map_err(|_| "persisted canary request slot does not fit u32".to_owned())?,
        runner_declaration: persisted_digest(identities, "runner_declaration_sha256")?,
        snapshot_offset: persisted_u64(snapshot, "absolute_offset_bytes")?,
        snapshot_sha256: persisted_digest(snapshot, "completed_sha256")?,
        suffix_sha256: persisted_digest(suffix, "completed_sha256")?,
    };
    validate_manifest(capture, &expected)?;
    Ok(R30PhysicalCaptureBindingsV1 {
        device_identity_sha256: hex_bytes(&expected.device_id),
        gpu_unique_id: expected.gpu_unique_id,
        kernel_artifact_manifest_sha256: hex_bytes(&expected.kernel_manifest),
        program_catalog_sha256: hex_bytes(&expected.program_catalog),
        runner_declaration_sha256: hex_bytes(&expected.runner_declaration),
    })
}

fn persisted_digest(object: &Map<String, Value>, name: &str) -> CaptureResult<[u8; 32]> {
    let value = field(object, name)?
        .as_str()
        .ok_or_else(|| format!("persisted canary {name} must be a string"))?;
    Ok(*decode_identity(value)?.as_bytes())
}

fn persisted_u64(object: &Map<String, Value>, name: &str) -> CaptureResult<u64> {
    field(object, name)?
        .as_u64()
        .ok_or_else(|| format!("persisted canary {name} must be a nonnegative integer"))
}

fn validate_manifest(bytes: &[u8], expected: &ExpectedBindingsV1) -> CaptureResult<()> {
    let value = parse_canonical(bytes, "partial r30 canary capture")?;
    let root = exact_object(
        &value,
        &[
            "authority",
            "case",
            "format",
            "identities",
            "layout",
            "milestone",
            "nonclaim",
            "obligation_id",
            "result",
            "status",
            "target",
            "trace",
        ],
        "partial r30 canary capture",
    )?;
    expect_string(root, "authority", "ferric-physical-partial-capture-only")?;
    expect_string(root, "case", CASE)?;
    expect_string(root, "format", CAPTURE_FORMAT)?;
    expect_string(root, "milestone", "M1")?;
    expect_string(root, "nonclaim", NONCLAIM)?;
    expect_string(root, "obligation_id", "m1.r30")?;
    expect_string(root, "status", STATUS)?;
    expect_string(root, "target", TARGET)?;

    let identities = exact_object(
        field(root, "identities")?,
        &[
            "device_id_sha256",
            "gpu_unique_id",
            "kernel_catalog_sha256",
            "kernel_manifest_sha256",
            "program_catalog_sha256",
            "protocol_sha256",
            "runner_declaration_sha256",
        ],
        "partial canary identities",
    )?;
    require_digest(identities, "device_id_sha256", &expected.device_id)?;
    require_u64(identities, "gpu_unique_id", expected.gpu_unique_id)?;
    require_digest(
        identities,
        "kernel_catalog_sha256",
        &expected.kernel_catalog,
    )?;
    require_digest(
        identities,
        "kernel_manifest_sha256",
        &expected.kernel_manifest,
    )?;
    require_digest(
        identities,
        "program_catalog_sha256",
        &expected.program_catalog,
    )?;
    require_digest(identities, "protocol_sha256", &expected.protocol_sha256)?;
    require_digest(
        identities,
        "runner_declaration_sha256",
        &expected.runner_declaration,
    )?;

    let initialized_prefix = sha256_array(&[M1_COMPLETION_CANARY_PREFIX_BYTE_V1; 64]);
    let initialized_interior = sha256_array(&[0_u8; 120]);
    let initialized_suffix = sha256_array(&[M1_COMPLETION_CANARY_SUFFIX_BYTE_V1; 64]);
    let layout = exact_object(
        field(root, "layout")?,
        &["interior", "prefix_guard", "snapshot", "suffix_guard"],
        "partial canary layout",
    )?;
    validate_guard(
        field(layout, "prefix_guard")?,
        0,
        M1_COMPLETION_CANARY_PREFIX_BYTE_V1,
        initialized_prefix,
        expected.prefix_sha256,
        "prefix",
    )?;
    validate_guard(
        field(layout, "suffix_guard")?,
        SUFFIX_RELATIVE_OFFSET,
        M1_COMPLETION_CANARY_SUFFIX_BYTE_V1,
        initialized_suffix,
        expected.suffix_sha256,
        "suffix",
    )?;
    let interior = exact_object(
        field(layout, "interior")?,
        &[
            "absolute_offset_bytes",
            "completed_sha256",
            "extent_bytes",
            "initialized_byte",
            "initialized_sha256",
            "relative_offset_bytes",
        ],
        "partial canary interior",
    )?;
    require_u64(interior, "absolute_offset_bytes", expected.interior_offset)?;
    require_u64(
        interior,
        "relative_offset_bytes",
        M1_COMPLETION_CANARY_GUARD_BYTES_V1,
    )?;
    require_u64(interior, "extent_bytes", INTERIOR_BYTES)?;
    require_u64(interior, "initialized_byte", 0)?;
    require_digest(interior, "initialized_sha256", &initialized_interior)?;
    require_digest(interior, "completed_sha256", &expected.interior_sha256)?;
    let snapshot = exact_object(
        field(layout, "snapshot")?,
        &["absolute_offset_bytes", "completed_sha256", "extent_bytes"],
        "partial canary snapshot",
    )?;
    require_u64(snapshot, "absolute_offset_bytes", expected.snapshot_offset)?;
    require_u64(snapshot, "extent_bytes", SNAPSHOT_BYTES)?;
    require_digest(snapshot, "completed_sha256", &expected.snapshot_sha256)?;
    if expected.interior_offset
        != expected
            .snapshot_offset
            .checked_add(M1_COMPLETION_CANARY_GUARD_BYTES_V1)
            .ok_or_else(|| "partial canary expected interior offset overflowed".to_owned())?
    {
        return Err("partial canary expected snapshot/interior coordinates drifted".to_owned());
    }

    let trace = exact_object(
        field(root, "trace")?,
        &[
            "completion_epoch",
            "data_index",
            "dispatch_generation",
            "plan_id_sha256",
            "request_generation",
            "request_slot",
        ],
        "partial canary trace",
    )?;
    require_u64(trace, "completion_epoch", expected.completion_epoch)?;
    require_u64(
        trace,
        "data_index",
        u64::try_from(expected.data_index).unwrap_or(u64::MAX),
    )?;
    require_u64(trace, "dispatch_generation", expected.dispatch_generation)?;
    require_digest(trace, "plan_id_sha256", &expected.plan_id)?;
    require_u64(
        trace,
        "request_generation",
        u64::from(expected.request_generation),
    )?;
    require_u64(trace, "request_slot", u64::from(expected.request_slot))?;
    if expected.completion_epoch == 0
        || expected.dispatch_generation == 0
        || expected.request_generation == 0
        || expected.gpu_unique_id == 0
    {
        return Err("partial canary trace identities must be nonzero".to_owned());
    }

    let result = exact_object(
        field(root, "result")?,
        &[
            "adjacent_guard_bytes_checked",
            "completed_members",
            "emitted_token",
            "events",
            "externally_published_count",
            "guard_corruptions",
            "logical_accepted_count",
            "queue_destroyed",
            "released_draft_pages",
            "released_target_pages",
            "single_k7_case_only",
        ],
        "partial canary result",
    )?;
    for (name, expected) in [
        ("adjacent_guard_bytes_checked", 128),
        ("completed_members", 1),
        ("externally_published_count", 1),
        ("guard_corruptions", 0),
        ("logical_accepted_count", 1),
        ("released_draft_pages", 0),
        ("released_target_pages", 1),
    ] {
        require_u64(result, name, expected)?;
    }
    require_u64(result, "emitted_token", u64::from(expected.emitted_token))?;
    for name in ["queue_destroyed", "single_k7_case_only"] {
        if field(result, name)?.as_bool() != Some(true) {
            return Err(format!("partial canary {name} must be true"));
        }
    }
    require_strings(result, "events", EVENTS)?;
    Ok(())
}

fn validate_guard(
    value: &Value,
    relative_offset: u64,
    initialized_byte: u8,
    initialized_sha256: [u8; 32],
    completed_sha256: [u8; 32],
    context: &str,
) -> CaptureResult<()> {
    let guard = exact_object(
        value,
        &[
            "completed_sha256",
            "extent_bytes",
            "initialized_byte",
            "initialized_sha256",
            "relative_offset_bytes",
            "unchanged",
        ],
        context,
    )?;
    require_u64(guard, "relative_offset_bytes", relative_offset)?;
    require_u64(guard, "extent_bytes", M1_COMPLETION_CANARY_GUARD_BYTES_V1)?;
    require_u64(guard, "initialized_byte", u64::from(initialized_byte))?;
    require_digest(guard, "initialized_sha256", &initialized_sha256)?;
    require_digest(guard, "completed_sha256", &completed_sha256)?;
    if initialized_sha256 != completed_sha256 || field(guard, "unchanged")?.as_bool() != Some(true)
    {
        return Err(format!("partial canary {context} guard changed"));
    }
    Ok(())
}

pub(super) fn require_protocol() -> CaptureResult<()> {
    validate_protocol(&protocol_bytes()?)
}

fn validate_protocol(bytes: &[u8]) -> CaptureResult<()> {
    let value = parse_canonical(bytes, "partial r30 canary protocol")?;
    let root = exact_object(
        &value,
        &[
            "authority",
            "bundle_files",
            "case",
            "format",
            "layout",
            "lifecycle",
            "milestone",
            "nonclaim",
            "obligation_id",
            "required_complete_case_roster",
            "status",
            "target",
        ],
        "partial r30 canary protocol",
    )?;
    expect_string(
        root,
        "authority",
        "ferric-m1-r30-canary-partial-protocol-only",
    )?;
    expect_string(root, "case", CASE)?;
    expect_string(root, "format", PROTOCOL_FORMAT)?;
    expect_string(root, "milestone", "M1")?;
    expect_string(root, "nonclaim", PROTOCOL_NONCLAIM)?;
    expect_string(root, "obligation_id", "m1.r30")?;
    expect_string(root, "status", STATUS)?;
    expect_string(root, "target", TARGET)?;
    require_strings(root, "bundle_files", &["capture.json", "protocol.json"])?;
    require_strings(root, "lifecycle", EVENTS)?;
    require_strings(
        root,
        "required_complete_case_roster",
        &[
            "canary",
            "cancellation",
            "exhaustion",
            "fault-injection",
            "rollback",
        ],
    )?;
    let layout = exact_object(
        field(root, "layout")?,
        &[
            "interior_bytes",
            "interior_relative_offset_bytes",
            "prefix_byte",
            "prefix_guard_bytes",
            "snapshot_bytes",
            "suffix_byte",
            "suffix_guard_bytes",
            "suffix_relative_offset_bytes",
        ],
        "partial canary protocol layout",
    )?;
    for (name, expected) in [
        ("interior_bytes", INTERIOR_BYTES),
        (
            "interior_relative_offset_bytes",
            M1_COMPLETION_CANARY_GUARD_BYTES_V1,
        ),
        (
            "prefix_byte",
            u64::from(M1_COMPLETION_CANARY_PREFIX_BYTE_V1),
        ),
        ("prefix_guard_bytes", M1_COMPLETION_CANARY_GUARD_BYTES_V1),
        ("snapshot_bytes", SNAPSHOT_BYTES),
        (
            "suffix_byte",
            u64::from(M1_COMPLETION_CANARY_SUFFIX_BYTE_V1),
        ),
        ("suffix_guard_bytes", M1_COMPLETION_CANARY_GUARD_BYTES_V1),
        ("suffix_relative_offset_bytes", SUFFIX_RELATIVE_OFFSET),
    ] {
        require_u64(layout, name, expected)?;
    }
    Ok(())
}

fn protocol_bytes() -> CaptureResult<Vec<u8>> {
    canonical_bytes(&json!({
        "authority": "ferric-m1-r30-canary-partial-protocol-only",
        "bundle_files": ["capture.json", "protocol.json"],
        "case": CASE,
        "format": PROTOCOL_FORMAT,
        "layout": {
            "interior_bytes": INTERIOR_BYTES,
            "interior_relative_offset_bytes": M1_COMPLETION_CANARY_GUARD_BYTES_V1,
            "prefix_byte": M1_COMPLETION_CANARY_PREFIX_BYTE_V1,
            "prefix_guard_bytes": M1_COMPLETION_CANARY_GUARD_BYTES_V1,
            "snapshot_bytes": SNAPSHOT_BYTES,
            "suffix_byte": M1_COMPLETION_CANARY_SUFFIX_BYTE_V1,
            "suffix_guard_bytes": M1_COMPLETION_CANARY_GUARD_BYTES_V1,
            "suffix_relative_offset_bytes": SUFFIX_RELATIVE_OFFSET,
        },
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
        "status": STATUS,
        "target": TARGET,
    }))
}

fn require_digest(
    object: &Map<String, Value>,
    name: &str,
    expected: &[u8; 32],
) -> CaptureResult<()> {
    let actual = field(object, name)?
        .as_str()
        .ok_or_else(|| format!("partial canary {name} must be a string"))?;
    require_sha256(actual)?;
    if actual != hex_bytes(expected) {
        return Err(format!("partial canary {name} digest drifted"));
    }
    Ok(())
}

fn u64_value(object: &Map<String, Value>, name: &str) -> CaptureResult<u64> {
    field(object, name)?
        .as_u64()
        .ok_or_else(|| format!("partial canary {name} must be a nonnegative integer"))
}

fn require_u64(object: &Map<String, Value>, name: &str, expected: u64) -> CaptureResult<()> {
    if u64_value(object, name)? != expected {
        return Err(format!("partial canary {name} value drifted"));
    }
    Ok(())
}

fn require_strings(
    object: &Map<String, Value>,
    name: &str,
    expected: &[&str],
) -> CaptureResult<()> {
    let actual = field(object, name)?
        .as_array()
        .and_then(|values| values.iter().map(Value::as_str).collect::<Option<Vec<_>>>())
        .ok_or_else(|| format!("partial canary {name} must be a string array"))?;
    if actual != expected {
        return Err(format!("partial canary {name} roster drifted"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expected() -> ExpectedBindingsV1 {
        ExpectedBindingsV1 {
            completion_epoch: 19,
            data_index: 4,
            device_id: [1; 32],
            dispatch_generation: 17,
            emitted_token: 41,
            gpu_unique_id: 23,
            interior_offset: 96,
            interior_sha256: [2; 32],
            kernel_catalog: [3; 32],
            kernel_manifest: [4; 32],
            plan_id: [5; 32],
            prefix_sha256: sha256_array(&[M1_COMPLETION_CANARY_PREFIX_BYTE_V1; 64]),
            program_catalog: [6; 32],
            protocol_sha256: sha256_array(&protocol_bytes().unwrap()),
            request_generation: 11,
            request_slot: 0,
            runner_declaration: [7; 32],
            snapshot_offset: 32,
            snapshot_sha256: [8; 32],
            suffix_sha256: sha256_array(&[M1_COMPLETION_CANARY_SUFFIX_BYTE_V1; 64]),
        }
    }

    fn fixture() -> Value {
        let expected = expected();
        capture_value(
            &expected,
            sha256_array(&[M1_COMPLETION_CANARY_PREFIX_BYTE_V1; 64]),
            sha256_array(&[0_u8; 120]),
            sha256_array(&[M1_COMPLETION_CANARY_SUFFIX_BYTE_V1; 64]),
        )
    }

    #[test]
    fn exact_partial_canary_capture_and_checked_protocol_are_accepted() {
        validate_manifest(&canonical_bytes(&fixture()).unwrap(), &expected()).unwrap();
        require_protocol().unwrap();
        let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR").unwrap();
        let checked_in = std::fs::read(
            std::path::PathBuf::from(manifest_dir)
                .join("src/bin/ferric-m1-r30-canary-partial-protocol.json"),
        )
        .unwrap();
        assert_eq!(protocol_bytes().unwrap(), checked_in);
    }

    #[test]
    fn persisted_bundle_admission_revalidates_capture_and_protocol() {
        let bytes = canonical_bytes(&fixture()).unwrap();
        let admitted = admit_persisted_bundle(&bytes, &protocol_bytes().unwrap()).unwrap();
        assert_eq!(admitted.device_identity_sha256, "01".repeat(32));
        assert_eq!(admitted.gpu_unique_id, 23);
        assert_eq!(admitted.kernel_artifact_manifest_sha256, "04".repeat(32));

        let mut hostile = fixture();
        hostile["layout"]["prefix_guard"]["unchanged"] = json!(false);
        assert!(admit_persisted_bundle(
            &canonical_bytes(&hostile).unwrap(),
            &protocol_bytes().unwrap()
        )
        .is_err());
    }

    #[test]
    fn hostile_layout_corruption_generation_and_promotion_reject() {
        let mutations: &[fn(&mut Value)] = &[
            |value| value["layout"]["prefix_guard"]["relative_offset_bytes"] = json!(1),
            |value| value["layout"]["prefix_guard"]["extent_bytes"] = json!(63),
            |value| value["layout"]["prefix_guard"]["initialized_byte"] = json!(90),
            |value| value["layout"]["prefix_guard"]["completed_sha256"] = json!("09".repeat(32)),
            |value| value["layout"]["suffix_guard"]["relative_offset_bytes"] = json!(183),
            |value| value["layout"]["suffix_guard"]["unchanged"] = json!(false),
            |value| value["layout"]["interior"]["extent_bytes"] = json!(119),
            |value| value["layout"]["interior"]["absolute_offset_bytes"] = json!(95),
            |value| value["layout"]["snapshot"]["extent_bytes"] = json!(247),
            |value| value["trace"]["dispatch_generation"] = json!(16),
            |value| value["trace"]["request_generation"] = json!(10),
            |value| value["result"]["emitted_token"] = json!(42),
            |value| value["result"]["guard_corruptions"] = json!(1),
            |value| value["result"]["events"][4] = json!("substituted"),
            |value| value["status"] = json!("evidence"),
            |value| value["identities"]["program_catalog_sha256"] = json!("09".repeat(32)),
            |value| value["extra"] = json!(true),
        ];
        for mutate in mutations {
            let mut value = fixture();
            mutate(&mut value);
            assert!(validate_manifest(&canonical_bytes(&value).unwrap(), &expected()).is_err());
        }
    }

    #[test]
    fn hostile_swapped_guard_and_interior_substitution_reject() {
        let mut swapped = fixture();
        let prefix = swapped["layout"]["prefix_guard"].clone();
        swapped["layout"]["prefix_guard"] = swapped["layout"]["suffix_guard"].clone();
        swapped["layout"]["suffix_guard"] = prefix;
        assert!(validate_manifest(&canonical_bytes(&swapped).unwrap(), &expected()).is_err());

        let mut substituted = fixture();
        substituted["layout"]["prefix_guard"]["completed_sha256"] =
            substituted["layout"]["interior"]["completed_sha256"].clone();
        assert!(validate_manifest(&canonical_bytes(&substituted).unwrap(), &expected()).is_err());
    }

    #[test]
    fn hostile_protocol_substitution_rejects() {
        let protocol = parse_canonical(&protocol_bytes().unwrap(), "fixture").unwrap();
        let mutations: &[fn(&mut Value)] = &[
            |value| value["status"] = json!("evidence"),
            |value| value["bundle_files"] = json!(["capture.json"]),
            |value| value["layout"]["prefix_guard_bytes"] = json!(63),
            |value| value["layout"]["interior_relative_offset_bytes"] = json!(63),
            |value| value["lifecycle"][5] = json!("substituted"),
            |value| value["required_complete_case_roster"][1] = json!("other"),
        ];
        for mutate in mutations {
            let mut value = protocol.clone();
            mutate(&mut value);
            assert!(validate_protocol(&canonical_bytes(&value).unwrap()).is_err());
        }
    }

    #[test]
    fn publisher_is_exact_and_no_replace() {
        let output = std::env::temp_dir().join(format!(
            "ferric-m1-r30-canary-publish-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let bytes = canonical_bytes(&fixture()).unwrap();
        publish(
            &output,
            CaptureArtifactV1 {
                bytes: bytes.clone(),
                expected: expected(),
            },
        )
        .unwrap();
        assert_eq!(std::fs::read(output.join("capture.json")).unwrap(), bytes);
        assert!(publish(
            &output,
            CaptureArtifactV1 {
                bytes: canonical_bytes(&fixture()).unwrap(),
                expected: expected(),
            },
        )
        .is_err());
        std::fs::remove_dir_all(output).unwrap();
    }
}
