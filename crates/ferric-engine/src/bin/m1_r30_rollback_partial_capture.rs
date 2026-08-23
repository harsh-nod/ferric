//! Canonical partial m1.r30 strict-prefix rollback capture.
//!
//! This two-file diagnostic covers only the rollback member of the five-case
//! r30 roster. It records source-level physical-KV metadata around exact Engine
//! completion and deliberately grants no physical subpage-return authority.

use super::{
    canonical_bytes, decode_identity, exact_object, expect_string, field, hex_bytes,
    parse_canonical, require_sha256, sha256_array, CaptureResult, R30PhysicalCaptureBindingsV1,
    StagingOutput, TARGET,
};
use ferric_engine::{
    CheckedCompletionSemantics, DeviceKvCacheProjection, M1CompletedStepSuccessV1,
    M1ObservedSpeculativeDiagnosticChoicesV1, M1PhysicalRunnerV1, M1ReleasedDeviceKvMemberV1,
    M1ReleasedQueueTeardownSuccessV1, M1StepDispatchIntent,
};
use ferric_spec::{Identity, PhysicalKvLifecycle, Qwen3ModelRole, RequestId};
use serde_json::{json, Map, Value};
use std::path::Path;

pub(super) const COMMAND: &str = "capture-r30-rollback";
const CAPTURE_FORMAT: &str = "FERRIC-M1-R30-ROLLBACK-PARTIAL-CAPTURE-V1";
const PROTOCOL_FORMAT: &str = "FERRIC-M1-R30-ROLLBACK-PARTIAL-PROTOCOL-V1";
const STATUS: &str = "partial-non-evidence";
const CASE: &str = "rollback-strict-prefix-s1-k4-c8192";
const NONCLAIM: &str = "Authenticated Ferric custody for one physical S1/K4 strict-prefix rollback diagnostic only. It covers only rollback among the five required m1.r30 cases. It records source-level physical-KV metadata before and after Engine completion, but does not establish physical subpage return or reuse, canary integrity, exhaustion handling, injected device-fault coverage, benchmark evidence, external or independent validation, hardware correctness, performance, qualification, or m1.r30/M1 closure.";
const PROTOCOL_NONCLAIM: &str = "Partial physical S1/K4 strict-prefix rollback protocol only. It covers only rollback among the five required m1.r30 cases and grants no physical page or subpage return/reuse authority, evidence authority, hardware correctness, performance, qualification, or m1.r30/M1 closure.";

const EVENTS: &[&str] = &[
    "pre-completion-kv-projection-captured",
    "queue-completed",
    "compact-readback-observed",
    "draft-choices-readback-observed",
    "target-choices-readback-observed",
    "strict-maximal-prefix-checked",
    "engine-completion-settled",
    "post-completion-pre-release-kv-projection-captured",
    "zero-retired-page-return-accounted",
    "queue-destroyed",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct QueueBindingsV1 {
    device_id: [u8; 32],
    gpu_unique_id: u64,
    kernel_catalog: [u8; 32],
    kernel_manifest: [u8; 32],
    program_catalog: [u8; 32],
    runner_declaration: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RoleProjectionV1 {
    active_pages: usize,
    arena_allocation_id: Option<[u8; 32]>,
    committed_tokens: u32,
    quiescent_retired_pages: usize,
    resident_tokens: u32,
    retired_pages: usize,
    role: Qwen3ModelRole,
    write_pending: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ProjectionV1 {
    device_id: [u8; 32],
    draft: RoleProjectionV1,
    request_generation: u32,
    request_slot: u32,
    target: RoleProjectionV1,
    target_qualification_future_pages: usize,
}

impl ProjectionV1 {
    fn capture(source: &DeviceKvCacheProjection) -> CaptureResult<Self> {
        if source.target.lifecycle != PhysicalKvLifecycle::Active
            || source.draft.lifecycle != PhysicalKvLifecycle::Active
        {
            return Err("partial rollback projection requires active role custody".to_owned());
        }
        if source.target.request != source.request
            || source.draft.request != source.request
            || source.target.role != Qwen3ModelRole::Target8B
            || source.draft.role != Qwen3ModelRole::Draft06B
        {
            return Err("partial rollback projection request or role drifted".to_owned());
        }
        Ok(Self {
            device_id: *source.device.device_id().as_bytes(),
            draft: RoleProjectionV1 {
                active_pages: source.draft_active_pages,
                arena_allocation_id: source
                    .draft_arena_allocation_id
                    .map(|identity| *identity.as_bytes()),
                committed_tokens: source.draft.committed_tokens,
                quiescent_retired_pages: source.draft_quiescent_retired_pages,
                resident_tokens: source.draft.resident_tokens,
                retired_pages: source.draft_retired_pages,
                role: source.draft.role,
                write_pending: source.draft_write_pending,
            },
            request_generation: source.request.generation(),
            request_slot: source.request.slot(),
            target: RoleProjectionV1 {
                active_pages: source.target_active_pages,
                arena_allocation_id: source
                    .target_arena_allocation_id
                    .map(|identity| *identity.as_bytes()),
                committed_tokens: source.target.committed_tokens,
                quiescent_retired_pages: source.target_quiescent_retired_pages,
                resident_tokens: source.target.resident_tokens,
                retired_pages: source.target_retired_pages,
                role: source.target.role,
                write_pending: source.target_write_pending,
            },
            target_qualification_future_pages: source.target_qualification_future_pages,
        })
    }

    fn as_json(self) -> Value {
        json!({
            "device_id_sha256": hex_bytes(&self.device_id),
            "draft": role_json(self.draft),
            "request_generation": self.request_generation,
            "request_slot": self.request_slot,
            "target": role_json(self.target),
            "target_qualification_future_pages": self.target_qualification_future_pages,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ExpectedBindingsV1 {
    compact_sha256: [u8; 32],
    completion_epoch: u64,
    dispatch_generation: u64,
    draft_sha256: [u8; 32],
    plan_id: [u8; 32],
    post: ProjectionV1,
    pre: ProjectionV1,
    queue: QueueBindingsV1,
    request_generation: u32,
    request_slot: u32,
    target_sha256: [u8; 32],
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

pub(super) struct ClosedCaptureInputsV1<'a> {
    pub(super) choices: &'a M1ObservedSpeculativeDiagnosticChoicesV1,
    pub(super) closed: &'a M1ReleasedQueueTeardownSuccessV1,
    pub(super) post_completion_pre_release: DeviceKvCacheProjection,
    pub(super) pre_completion: DeviceKvCacheProjection,
    pub(super) queue: QueueBindingsV1,
}

pub(super) fn capture_queue_bindings(
    completed: &M1CompletedStepSuccessV1,
    runner: &M1PhysicalRunnerV1,
) -> CaptureResult<QueueBindingsV1> {
    let custody = completed.queue().custody();
    let dispatch_plan = custody.workspace_composition().dispatch_plan();
    if custody.selection() != completed.checked().selection()
        || dispatch_plan.intent()
            != M1StepDispatchIntent::SpeculativeRound(completed.checked().selection())
    {
        return Err("partial rollback retained selection or dispatch intent drifted".to_owned());
    }
    let program_catalog = custody.catalog_id();
    let runner_declaration = dispatch_plan.runner_declaration_id();
    let kernel_catalog = dispatch_plan.kernel_catalog_id();
    if runner.program_catalog_id() != program_catalog
        || runner.declaration_id() != runner_declaration
        || runner.kernel_catalog_id() != kernel_catalog
    {
        return Err("partial rollback queue and runner identities differ".to_owned());
    }
    Ok(QueueBindingsV1 {
        device_id: *custody.device().device_id().as_bytes(),
        gpu_unique_id: custody.device().gpu_unique_id(),
        kernel_catalog: *kernel_catalog.as_bytes(),
        kernel_manifest: *runner.kernel_artifact_manifest_id().as_bytes(),
        program_catalog: *program_catalog.as_bytes(),
        runner_declaration: *runner_declaration.as_bytes(),
    })
}

pub(super) fn manifest(inputs: ClosedCaptureInputsV1<'_>) -> CaptureResult<CaptureArtifactV1> {
    require_protocol()?;
    let checked = inputs.closed.checked();
    let [record] = checked.records() else {
        return Err("partial rollback capture requires exactly one checked S1 record".to_owned());
    };
    let CheckedCompletionSemantics::Speculative {
        accepted_draft_tokens,
        correction_or_bonus,
    } = record.semantics()
    else {
        return Err("partial rollback capture requires checked speculative semantics".to_owned());
    };
    if accepted_draft_tokens >= 4 {
        return Err("partial rollback capture requires a strict K4 prefix".to_owned());
    }
    if inputs.choices.dispatch_generation() != checked.dispatch_generation() {
        return Err("rollback choice and compact dispatch generations differ".to_owned());
    }
    let draft = inputs.choices.draft_choices();
    let target = inputs.choices.target_choices();
    let maximal_prefix = draft
        .iter()
        .zip(target)
        .position(|(draft, target)| draft != target)
        .unwrap_or(draft.len());
    if usize::from(accepted_draft_tokens) != maximal_prefix {
        return Err("partial rollback accepted prefix is not maximal".to_owned());
    }
    let raw = record.record();
    let emitted_count = usize::from(raw.emitted_token_count);
    let emitted = raw
        .emitted_tokens
        .get(..emitted_count)
        .ok_or_else(|| "partial rollback emitted-token extent drifted".to_owned())?;
    let accepted = usize::from(accepted_draft_tokens);
    let expected_emitted = draft[..accepted]
        .iter()
        .copied()
        .chain([target[accepted]])
        .collect::<Vec<_>>();
    if emitted != expected_emitted || correction_or_bonus != target[accepted] {
        return Err("partial rollback emitted prefix differs from exact choices".to_owned());
    }
    let logical_prefix = u32::from(accepted_draft_tokens) + 1;
    let pre = ProjectionV1::capture(&inputs.pre_completion)?;
    let post = ProjectionV1::capture(&inputs.post_completion_pre_release)?;
    validate_projection_transition(
        pre,
        post,
        raw.request,
        logical_prefix,
        inputs.queue.device_id,
    )?;

    let [M1ReleasedDeviceKvMemberV1::Active(active)] = inputs.closed.members() else {
        return Err("partial rollback closed custody must retain one active cache".to_owned());
    };
    if ProjectionV1::capture(&active.projection())? != post {
        return Err("partial rollback closed cache differs from pre-release projection".to_owned());
    }
    if inputs.closed.completed_members() != 1
        || inputs.closed.logical_accepted_counts() != [logical_prefix]
        || inputs.closed.externally_published_counts() != [logical_prefix]
        || inputs.closed.release_counts().len() != 1
        || inputs.closed.release_counts()[0].draft() != 0
        || inputs.closed.release_counts()[0].target() != 0
        || inputs.closed.total_released() != 0
    {
        return Err("partial rollback completion or zero-return accounting drifted".to_owned());
    }

    let expected = ExpectedBindingsV1 {
        compact_sha256: *checked.raw_sha256(),
        completion_epoch: checked.epoch().value(),
        dispatch_generation: checked.dispatch_generation(),
        draft_sha256: *inputs.choices.draft_sha256(),
        plan_id: *raw.plan_id.as_bytes(),
        post,
        pre,
        queue: inputs.queue,
        request_generation: raw.request.generation(),
        request_slot: raw.request.slot(),
        target_sha256: *inputs.choices.target_sha256(),
    };
    let target_rejected = 4usize - accepted;
    let draft_rejected = 3usize - accepted;
    let release = inputs.closed.release_counts()[0];
    let value = json!({
        "authority": "ferric-physical-partial-capture-only",
        "case": CASE,
        "choices": {
            "draft": draft,
            "draft_bytes": inputs.choices.draft_bytes().len(),
            "draft_sha256": hex_bytes(inputs.choices.draft_sha256()),
            "encoding": "u32-le",
            "target": target,
            "target_bytes": inputs.choices.target_bytes().len(),
            "target_sha256": hex_bytes(inputs.choices.target_sha256()),
        },
        "format": CAPTURE_FORMAT,
        "identities": {
            "device_id_sha256": hex_bytes(&inputs.queue.device_id),
            "gpu_unique_id": inputs.queue.gpu_unique_id,
            "kernel_catalog_sha256": hex_bytes(&inputs.queue.kernel_catalog),
            "kernel_manifest_sha256": hex_bytes(&inputs.queue.kernel_manifest),
            "program_catalog_sha256": hex_bytes(&inputs.queue.program_catalog),
            "runner_declaration_sha256": hex_bytes(&inputs.queue.runner_declaration),
        },
        "kv_projections": {
            "post_engine_completion_pre_release": post.as_json(),
            "pre_completion": pre.as_json(),
        },
        "milestone": "M1",
        "nonclaim": NONCLAIM,
        "obligation_id": "m1.r30",
        "result": {
            "accepted_draft_tokens": accepted_draft_tokens,
            "completed_members": inputs.closed.completed_members(),
            "correction_or_bonus": correction_or_bonus,
            "emitted_tokens": emitted,
            "events": EVENTS,
            "externally_published_count": logical_prefix,
            "logical_accepted_count": logical_prefix,
            "maximal_prefix_verified": true,
            "physical_page_returns": {
                "draft": release.draft(),
                "target": release.target(),
                "total": release.total(),
            },
            "physical_subpage_return_or_reuse_claimed": false,
            "positive_completion": true,
            "queue_destroyed": true,
            "rejected_suffix_tokens": {
                "draft": draft_rejected,
                "target": target_rejected,
            },
            "rollback_case_only": true,
            "strict_prefix_verified": true,
        },
        "status": STATUS,
        "target": TARGET,
        "trace": {
            "compact_sha256": hex_bytes(checked.raw_sha256()),
            "completion_epoch": checked.epoch().value(),
            "dispatch_generation": checked.dispatch_generation(),
            "plan_id_sha256": hex_identity(raw.plan_id),
            "request_generation": raw.request.generation(),
            "request_slot": raw.request.slot(),
        },
    });
    let bytes = canonical_bytes(&value)?;
    validate_manifest(&bytes, &expected)?;
    Ok(CaptureArtifactV1 { bytes, expected })
}

pub(super) fn publish(output: &Path, capture: CaptureArtifactV1) -> CaptureResult<()> {
    validate_manifest(&capture.bytes, &capture.expected)?;
    require_protocol()?;
    let protocol = protocol_bytes()?;
    let mut staging = StagingOutput::create(output)?;
    staging.write("capture.json", &capture.bytes)?;
    staging.write("protocol.json", &protocol)?;
    staging.publish_exact(&[
        ("capture.json", &capture.bytes),
        ("protocol.json", &protocol),
    ])
}

pub(super) fn admit_persisted_bundle(
    capture: &[u8],
    protocol: &[u8],
) -> CaptureResult<R30PhysicalCaptureBindingsV1> {
    let expected_protocol = protocol_bytes()?;
    if protocol != expected_protocol {
        return Err("partial r30 rollback protocol bytes drifted".to_owned());
    }
    let value = parse_canonical(capture, "persisted partial r30 rollback capture")?;
    let root = value
        .as_object()
        .ok_or_else(|| "persisted partial r30 rollback capture must be an object".to_owned())?;
    let identities = persisted_object(root, "identities", "rollback identities")?;
    let trace = persisted_object(root, "trace", "rollback trace")?;
    let choices = persisted_object(root, "choices", "rollback choices")?;
    let projections = persisted_object(root, "kv_projections", "rollback projections")?;
    let result = persisted_object(root, "result", "rollback result")?;
    let request_generation = u32::try_from(persisted_u64(trace, "request_generation")?)
        .map_err(|_| "persisted rollback request generation does not fit u32".to_owned())?;
    let request_slot = u32::try_from(persisted_u64(trace, "request_slot")?)
        .map_err(|_| "persisted rollback request slot does not fit u32".to_owned())?;
    let queue = QueueBindingsV1 {
        device_id: persisted_digest(identities, "device_id_sha256")?,
        gpu_unique_id: persisted_u64(identities, "gpu_unique_id")?,
        kernel_catalog: persisted_digest(identities, "kernel_catalog_sha256")?,
        kernel_manifest: persisted_digest(identities, "kernel_manifest_sha256")?,
        program_catalog: persisted_digest(identities, "program_catalog_sha256")?,
        runner_declaration: persisted_digest(identities, "runner_declaration_sha256")?,
    };
    let expected = ExpectedBindingsV1 {
        compact_sha256: persisted_digest(trace, "compact_sha256")?,
        completion_epoch: persisted_u64(trace, "completion_epoch")?,
        dispatch_generation: persisted_u64(trace, "dispatch_generation")?,
        draft_sha256: persisted_digest(choices, "draft_sha256")?,
        plan_id: persisted_digest(trace, "plan_id_sha256")?,
        post: persisted_projection(
            field(projections, "post_engine_completion_pre_release")?,
            "post-completion",
        )?,
        pre: persisted_projection(field(projections, "pre_completion")?, "pre-completion")?,
        queue,
        request_generation,
        request_slot,
        target_sha256: persisted_digest(choices, "target_sha256")?,
    };
    validate_manifest(capture, &expected)?;
    let accepted = u32::try_from(persisted_u64(result, "accepted_draft_tokens")?)
        .map_err(|_| "persisted rollback accepted count does not fit u32".to_owned())?;
    let logical_prefix = accepted
        .checked_add(1)
        .ok_or_else(|| "persisted rollback logical prefix overflowed".to_owned())?;
    validate_projection_transition(
        expected.pre,
        expected.post,
        RequestId::new(request_slot, request_generation),
        logical_prefix,
        expected.queue.device_id,
    )?;
    Ok(R30PhysicalCaptureBindingsV1 {
        device_identity_sha256: hex_bytes(&expected.queue.device_id),
        gpu_unique_id: expected.queue.gpu_unique_id,
        kernel_artifact_manifest_sha256: hex_bytes(&expected.queue.kernel_manifest),
        program_catalog_sha256: hex_bytes(&expected.queue.program_catalog),
        runner_declaration_sha256: hex_bytes(&expected.queue.runner_declaration),
    })
}

fn persisted_projection(value: &Value, context: &str) -> CaptureResult<ProjectionV1> {
    let projection = value
        .as_object()
        .ok_or_else(|| format!("persisted rollback {context} projection must be an object"))?;
    Ok(ProjectionV1 {
        device_id: persisted_digest(projection, "device_id_sha256")?,
        draft: persisted_role(
            field(projection, "draft")?,
            Qwen3ModelRole::Draft06B,
            context,
        )?,
        request_generation: u32::try_from(persisted_u64(projection, "request_generation")?)
            .map_err(|_| format!("persisted rollback {context} generation does not fit u32"))?,
        request_slot: u32::try_from(persisted_u64(projection, "request_slot")?)
            .map_err(|_| format!("persisted rollback {context} slot does not fit u32"))?,
        target: persisted_role(
            field(projection, "target")?,
            Qwen3ModelRole::Target8B,
            context,
        )?,
        target_qualification_future_pages: usize::try_from(persisted_u64(
            projection,
            "target_qualification_future_pages",
        )?)
        .map_err(|_| {
            format!("persisted rollback {context} future-page count does not fit usize")
        })?,
    })
}

fn persisted_role(
    value: &Value,
    role: Qwen3ModelRole,
    context: &str,
) -> CaptureResult<RoleProjectionV1> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("persisted rollback {context} role must be an object"))?;
    let arena = match field(object, "arena_allocation_id_sha256")? {
        Value::Null => None,
        Value::String(value) => Some(*decode_identity(value)?.as_bytes()),
        _ => {
            return Err(format!(
                "persisted rollback {context} arena identity is invalid"
            ))
        }
    };
    Ok(RoleProjectionV1 {
        active_pages: usize::try_from(persisted_u64(object, "active_pages")?)
            .map_err(|_| format!("persisted rollback {context} active pages do not fit usize"))?,
        arena_allocation_id: arena,
        committed_tokens: u32::try_from(persisted_u64(object, "committed_tokens")?).map_err(
            |_| format!("persisted rollback {context} committed count does not fit u32"),
        )?,
        quiescent_retired_pages: usize::try_from(persisted_u64(object, "quiescent_retired_pages")?)
            .map_err(|_| {
                format!("persisted rollback {context} quiescent pages do not fit usize")
            })?,
        resident_tokens: u32::try_from(persisted_u64(object, "resident_tokens")?)
            .map_err(|_| format!("persisted rollback {context} resident count does not fit u32"))?,
        retired_pages: usize::try_from(persisted_u64(object, "retired_pages")?)
            .map_err(|_| format!("persisted rollback {context} retired pages do not fit usize"))?,
        role,
        write_pending: field(object, "write_pending")?
            .as_bool()
            .ok_or_else(|| format!("persisted rollback {context} write-pending flag is invalid"))?,
    })
}

fn persisted_object<'a>(
    object: &'a Map<String, Value>,
    name: &str,
    context: &str,
) -> CaptureResult<&'a Map<String, Value>> {
    field(object, name)?
        .as_object()
        .ok_or_else(|| format!("persisted {context} must be an object"))
}

fn persisted_digest(object: &Map<String, Value>, name: &str) -> CaptureResult<[u8; 32]> {
    let value = field(object, name)?
        .as_str()
        .ok_or_else(|| format!("persisted rollback {name} must be a string"))?;
    Ok(*decode_identity(value)?.as_bytes())
}

fn persisted_u64(object: &Map<String, Value>, name: &str) -> CaptureResult<u64> {
    field(object, name)?
        .as_u64()
        .ok_or_else(|| format!("persisted rollback {name} must be a nonnegative integer"))
}

fn validate_projection_transition(
    pre: ProjectionV1,
    post: ProjectionV1,
    request: ferric_spec::RequestId,
    logical_prefix: u32,
    device_id: [u8; 32],
) -> CaptureResult<()> {
    if pre.device_id != device_id
        || post.device_id != device_id
        || pre.request_generation != request.generation()
        || post.request_generation != request.generation()
        || pre.request_slot != request.slot()
        || post.request_slot != request.slot()
    {
        return Err("partial rollback projection identity drifted".to_owned());
    }
    if pre.target.committed_tokens != 0
        || pre.target.resident_tokens != 0
        || pre.draft.committed_tokens != 0
        || pre.draft.resident_tokens != 0
        || !pre.target.write_pending
        || !pre.draft.write_pending
        || pre.target.active_pages != 0
        || pre.draft.active_pages != 0
        || pre.target.retired_pages != 0
        || pre.draft.retired_pages != 0
        || pre.target.quiescent_retired_pages != 0
        || pre.draft.quiescent_retired_pages != 0
        || pre.target.arena_allocation_id.is_some()
        || pre.draft.arena_allocation_id.is_some()
        || pre.target_qualification_future_pages != 0
    {
        return Err("partial rollback pre-completion projection is not exact".to_owned());
    }
    if post.target.committed_tokens != logical_prefix
        || post.target.resident_tokens != logical_prefix
        || post.draft.committed_tokens != logical_prefix
        || post.draft.resident_tokens != logical_prefix
        || post.target.write_pending
        || post.draft.write_pending
        || post.target.active_pages != 1
        || post.draft.active_pages != 1
        || post.target.retired_pages != 0
        || post.draft.retired_pages != 0
        || post.target.quiescent_retired_pages != 0
        || post.draft.quiescent_retired_pages != 0
        || post.target.arena_allocation_id.is_none()
        || post.draft.arena_allocation_id.is_none()
        || post.target.arena_allocation_id == post.draft.arena_allocation_id
        || post.target_qualification_future_pages != 0
    {
        return Err(
            "partial rollback post-completion projection is not the exact prefix".to_owned(),
        );
    }
    Ok(())
}

fn validate_manifest(bytes: &[u8], expected: &ExpectedBindingsV1) -> CaptureResult<()> {
    let value = parse_canonical(bytes, "partial r30 rollback capture")?;
    let root = exact_object(
        &value,
        &[
            "authority",
            "case",
            "choices",
            "format",
            "identities",
            "kv_projections",
            "milestone",
            "nonclaim",
            "obligation_id",
            "result",
            "status",
            "target",
            "trace",
        ],
        "partial r30 rollback capture",
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
            "runner_declaration_sha256",
        ],
        "partial rollback identities",
    )?;
    require_exact_sha256(identities, "device_id_sha256", &expected.queue.device_id)?;
    require_exact_u64(identities, "gpu_unique_id", expected.queue.gpu_unique_id)?;
    require_exact_sha256(
        identities,
        "kernel_catalog_sha256",
        &expected.queue.kernel_catalog,
    )?;
    require_exact_sha256(
        identities,
        "kernel_manifest_sha256",
        &expected.queue.kernel_manifest,
    )?;
    require_exact_sha256(
        identities,
        "program_catalog_sha256",
        &expected.queue.program_catalog,
    )?;
    require_exact_sha256(
        identities,
        "runner_declaration_sha256",
        &expected.queue.runner_declaration,
    )?;

    let trace = exact_object(
        field(root, "trace")?,
        &[
            "compact_sha256",
            "completion_epoch",
            "dispatch_generation",
            "plan_id_sha256",
            "request_generation",
            "request_slot",
        ],
        "partial rollback trace",
    )?;
    require_exact_sha256(trace, "compact_sha256", &expected.compact_sha256)?;
    require_exact_sha256(trace, "plan_id_sha256", &expected.plan_id)?;
    require_exact_u64(trace, "completion_epoch", expected.completion_epoch)?;
    require_exact_u64(trace, "dispatch_generation", expected.dispatch_generation)?;
    require_exact_u64(
        trace,
        "request_generation",
        u64::from(expected.request_generation),
    )?;
    require_exact_u64(trace, "request_slot", u64::from(expected.request_slot))?;
    if expected.completion_epoch == 0
        || expected.dispatch_generation == 0
        || expected.request_generation == 0
        || expected.queue.gpu_unique_id == 0
    {
        return Err("partial rollback trace identities must be nonzero".to_owned());
    }

    let choices = exact_object(
        field(root, "choices")?,
        &[
            "draft",
            "draft_bytes",
            "draft_sha256",
            "encoding",
            "target",
            "target_bytes",
            "target_sha256",
        ],
        "partial rollback choices",
    )?;
    expect_string(choices, "encoding", "u32-le")?;
    require_exact_u64(choices, "draft_bytes", 16)?;
    require_exact_u64(choices, "target_bytes", 20)?;
    require_exact_sha256(choices, "draft_sha256", &expected.draft_sha256)?;
    require_exact_sha256(choices, "target_sha256", &expected.target_sha256)?;
    let draft = token_array(field(choices, "draft")?, 4, "rollback draft choices")?;
    let target = token_array(field(choices, "target")?, 5, "rollback target choices")?;
    if sha256_array(&token_bytes(&draft)) != expected.draft_sha256
        || sha256_array(&token_bytes(&target)) != expected.target_sha256
    {
        return Err("partial rollback choices differ from copied-byte digests".to_owned());
    }

    let projections = exact_object(
        field(root, "kv_projections")?,
        &["post_engine_completion_pre_release", "pre_completion"],
        "partial rollback KV projections",
    )?;
    validate_projection(
        field(projections, "pre_completion")?,
        expected.pre,
        "pre-completion",
    )?;
    validate_projection(
        field(projections, "post_engine_completion_pre_release")?,
        expected.post,
        "post-completion",
    )?;

    let result = exact_object(
        field(root, "result")?,
        &[
            "accepted_draft_tokens",
            "completed_members",
            "correction_or_bonus",
            "emitted_tokens",
            "events",
            "externally_published_count",
            "logical_accepted_count",
            "maximal_prefix_verified",
            "physical_page_returns",
            "physical_subpage_return_or_reuse_claimed",
            "positive_completion",
            "queue_destroyed",
            "rejected_suffix_tokens",
            "rollback_case_only",
            "strict_prefix_verified",
        ],
        "partial rollback result",
    )?;
    let accepted = usize::try_from(u64_value(result, "accepted_draft_tokens")?)
        .map_err(|_| "partial rollback accepted count does not fit usize".to_owned())?;
    let mismatch = draft
        .iter()
        .zip(&target)
        .position(|(draft, target)| draft != target)
        .unwrap_or(draft.len());
    if accepted >= 4 || accepted != mismatch {
        return Err("partial rollback result is not a strict maximal prefix".to_owned());
    }
    let expected_emitted = draft[..accepted]
        .iter()
        .copied()
        .chain([target[accepted]])
        .collect::<Vec<_>>();
    if token_array(
        field(result, "emitted_tokens")?,
        accepted + 1,
        "rollback emitted tokens",
    )? != expected_emitted
        || u64_value(result, "correction_or_bonus")? != u64::from(target[accepted])
    {
        return Err("partial rollback result token semantics drifted".to_owned());
    }
    let logical_prefix = u64::try_from(accepted + 1).unwrap_or(u64::MAX);
    require_exact_u64(result, "completed_members", 1)?;
    require_exact_u64(result, "logical_accepted_count", logical_prefix)?;
    require_exact_u64(result, "externally_published_count", logical_prefix)?;
    for name in [
        "maximal_prefix_verified",
        "positive_completion",
        "queue_destroyed",
        "rollback_case_only",
        "strict_prefix_verified",
    ] {
        if field(result, name)?.as_bool() != Some(true) {
            return Err(format!("partial rollback {name} must be true"));
        }
    }
    if field(result, "physical_subpage_return_or_reuse_claimed")?.as_bool() != Some(false) {
        return Err("partial rollback must not claim physical subpage return or reuse".to_owned());
    }
    let suffix = exact_object(
        field(result, "rejected_suffix_tokens")?,
        &["draft", "target"],
        "partial rollback rejected suffix",
    )?;
    require_exact_u64(
        suffix,
        "draft",
        u64::try_from(3 - accepted).unwrap_or(u64::MAX),
    )?;
    require_exact_u64(
        suffix,
        "target",
        u64::try_from(4 - accepted).unwrap_or(u64::MAX),
    )?;
    let returns = exact_object(
        field(result, "physical_page_returns")?,
        &["draft", "target", "total"],
        "partial rollback physical page returns",
    )?;
    for name in ["draft", "target", "total"] {
        require_exact_u64(returns, name, 0)?;
    }
    let events = field(result, "events")?
        .as_array()
        .and_then(|events| events.iter().map(Value::as_str).collect::<Option<Vec<_>>>())
        .ok_or_else(|| "partial rollback events must be a string array".to_owned())?;
    if events != EVENTS {
        return Err("partial rollback event order drifted".to_owned());
    }
    Ok(())
}

fn validate_projection(value: &Value, expected: ProjectionV1, context: &str) -> CaptureResult<()> {
    let projection = exact_object(
        value,
        &[
            "device_id_sha256",
            "draft",
            "request_generation",
            "request_slot",
            "target",
            "target_qualification_future_pages",
        ],
        context,
    )?;
    require_exact_sha256(projection, "device_id_sha256", &expected.device_id)?;
    require_exact_u64(
        projection,
        "request_generation",
        u64::from(expected.request_generation),
    )?;
    require_exact_u64(projection, "request_slot", u64::from(expected.request_slot))?;
    require_exact_u64(
        projection,
        "target_qualification_future_pages",
        u64::try_from(expected.target_qualification_future_pages).unwrap_or(u64::MAX),
    )?;
    validate_role(field(projection, "target")?, expected.target, context)?;
    validate_role(field(projection, "draft")?, expected.draft, context)
}

fn validate_role(value: &Value, expected: RoleProjectionV1, context: &str) -> CaptureResult<()> {
    let role = exact_object(
        value,
        &[
            "active_pages",
            "arena_allocation_id_sha256",
            "committed_tokens",
            "lifecycle",
            "quiescent_retired_pages",
            "resident_tokens",
            "retired_pages",
            "role",
            "write_pending",
        ],
        context,
    )?;
    expect_string(role, "lifecycle", "active")?;
    expect_string(
        role,
        "role",
        match expected.role {
            Qwen3ModelRole::Target8B => "target-8b",
            Qwen3ModelRole::Draft06B => "draft-0.6b",
        },
    )?;
    for (name, expected) in [
        ("active_pages", expected.active_pages),
        ("quiescent_retired_pages", expected.quiescent_retired_pages),
        ("retired_pages", expected.retired_pages),
    ] {
        require_exact_u64(role, name, u64::try_from(expected).unwrap_or(u64::MAX))?;
    }
    require_exact_u64(
        role,
        "committed_tokens",
        u64::from(expected.committed_tokens),
    )?;
    require_exact_u64(role, "resident_tokens", u64::from(expected.resident_tokens))?;
    if field(role, "write_pending")?.as_bool() != Some(expected.write_pending) {
        return Err(format!(
            "partial rollback {context} pending-write state drifted"
        ));
    }
    let arena = field(role, "arena_allocation_id_sha256")?;
    match (arena.as_str(), expected.arena_allocation_id) {
        (Some(actual), Some(expected)) => {
            require_sha256(actual)?;
            if actual != hex_bytes(&expected) {
                return Err(format!("partial rollback {context} arena identity drifted"));
            }
        }
        (None, None) if arena.is_null() => {}
        _ => return Err(format!("partial rollback {context} arena presence drifted")),
    }
    Ok(())
}

pub(super) fn require_protocol() -> CaptureResult<()> {
    let bytes = protocol_bytes()?;
    validate_protocol(&bytes)
}

fn validate_protocol(bytes: &[u8]) -> CaptureResult<()> {
    let value = parse_canonical(bytes, "partial r30 rollback protocol")?;
    let root = exact_object(
        &value,
        &[
            "authority",
            "bundle_files",
            "case",
            "format",
            "lifecycle",
            "milestone",
            "nonclaim",
            "obligation_id",
            "required_complete_case_roster",
            "status",
            "target",
        ],
        "partial r30 rollback protocol",
    )?;
    expect_string(
        root,
        "authority",
        "ferric-m1-r30-rollback-partial-protocol-only",
    )?;
    expect_string(root, "case", CASE)?;
    expect_string(root, "format", PROTOCOL_FORMAT)?;
    expect_string(root, "milestone", "M1")?;
    expect_string(root, "nonclaim", PROTOCOL_NONCLAIM)?;
    expect_string(root, "obligation_id", "m1.r30")?;
    expect_string(root, "status", STATUS)?;
    expect_string(root, "target", TARGET)?;
    require_exact_strings(
        root,
        "bundle_files",
        &["capture.json", "protocol.json"],
        "bundle roster",
    )?;
    require_exact_strings(root, "lifecycle", EVENTS, "lifecycle")?;
    require_exact_strings(
        root,
        "required_complete_case_roster",
        &[
            "canary",
            "cancellation",
            "exhaustion",
            "fault-injection",
            "rollback",
        ],
        "complete-case roster",
    )
}

fn protocol_bytes() -> CaptureResult<Vec<u8>> {
    canonical_bytes(&json!({
        "authority": "ferric-m1-r30-rollback-partial-protocol-only",
        "bundle_files": ["capture.json", "protocol.json"],
        "case": CASE,
        "format": PROTOCOL_FORMAT,
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

fn role_json(role: RoleProjectionV1) -> Value {
    json!({
        "active_pages": role.active_pages,
        "arena_allocation_id_sha256": role.arena_allocation_id.map(|identity| hex_bytes(&identity)),
        "committed_tokens": role.committed_tokens,
        "lifecycle": "active",
        "quiescent_retired_pages": role.quiescent_retired_pages,
        "resident_tokens": role.resident_tokens,
        "retired_pages": role.retired_pages,
        "role": match role.role {
            Qwen3ModelRole::Target8B => "target-8b",
            Qwen3ModelRole::Draft06B => "draft-0.6b",
        },
        "write_pending": role.write_pending,
    })
}

fn token_array(value: &Value, expected: usize, context: &str) -> CaptureResult<Vec<u32>> {
    let values = value
        .as_array()
        .ok_or_else(|| format!("{context} must be an array"))?;
    if values.len() != expected {
        return Err(format!("{context} must contain exactly {expected} tokens"));
    }
    values
        .iter()
        .map(|value| {
            let token = value
                .as_u64()
                .and_then(|value| u32::try_from(value).ok())
                .ok_or_else(|| format!("{context} token must be u32"))?;
            if token >= ferric_spec::QWEN3_VOCABULARY_SIZE {
                return Err(format!("{context} token is outside the Qwen vocabulary"));
            }
            Ok(token)
        })
        .collect()
}

fn token_bytes(tokens: &[u32]) -> Vec<u8> {
    tokens
        .iter()
        .flat_map(|token| token.to_le_bytes())
        .collect()
}

fn require_exact_strings(
    object: &serde_json::Map<String, Value>,
    name: &str,
    expected: &[&str],
    context: &str,
) -> CaptureResult<()> {
    let actual = field(object, name)?
        .as_array()
        .and_then(|items| items.iter().map(Value::as_str).collect::<Option<Vec<_>>>())
        .ok_or_else(|| format!("partial rollback {context} must be a string array"))?;
    if actual != expected {
        return Err(format!("partial rollback {context} drifted"));
    }
    Ok(())
}

fn string_value<'a>(
    object: &'a serde_json::Map<String, Value>,
    name: &str,
) -> CaptureResult<&'a str> {
    field(object, name)?
        .as_str()
        .ok_or_else(|| format!("partial rollback {name} must be a string"))
}

fn u64_value(object: &serde_json::Map<String, Value>, name: &str) -> CaptureResult<u64> {
    field(object, name)?
        .as_u64()
        .ok_or_else(|| format!("partial rollback {name} must be u64"))
}

fn require_exact_u64(
    object: &serde_json::Map<String, Value>,
    name: &str,
    expected: u64,
) -> CaptureResult<()> {
    if u64_value(object, name)? != expected {
        return Err(format!(
            "partial rollback {name} differs from retained custody"
        ));
    }
    Ok(())
}

fn require_exact_sha256(
    object: &serde_json::Map<String, Value>,
    name: &str,
    expected: &[u8; 32],
) -> CaptureResult<()> {
    let actual = string_value(object, name)?;
    require_sha256(actual)?;
    if actual != hex_bytes(expected) {
        return Err(format!(
            "partial rollback {name} differs from retained custody"
        ));
    }
    Ok(())
}

fn hex_identity(identity: Identity) -> String {
    hex_bytes(identity.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn role(role: Qwen3ModelRole, committed: u32, pending: bool, post: bool) -> RoleProjectionV1 {
        RoleProjectionV1 {
            active_pages: usize::from(post),
            arena_allocation_id: post.then_some(match role {
                Qwen3ModelRole::Target8B => [21; 32],
                Qwen3ModelRole::Draft06B => [22; 32],
            }),
            committed_tokens: committed,
            quiescent_retired_pages: 0,
            resident_tokens: committed,
            retired_pages: 0,
            role,
            write_pending: pending,
        }
    }

    fn projection(committed: u32, pending: bool, post: bool) -> ProjectionV1 {
        ProjectionV1 {
            device_id: [2; 32],
            draft: role(Qwen3ModelRole::Draft06B, committed, pending, post),
            request_generation: 7,
            request_slot: 0,
            target: role(Qwen3ModelRole::Target8B, committed, pending, post),
            target_qualification_future_pages: 0,
        }
    }

    fn expected() -> ExpectedBindingsV1 {
        ExpectedBindingsV1 {
            compact_sha256: [9; 32],
            completion_epoch: 31,
            dispatch_generation: 37,
            draft_sha256: sha256_array(&token_bytes(&[11, 12, 13, 14])),
            plan_id: [3; 32],
            post: projection(3, false, true),
            pre: projection(0, true, false),
            queue: QueueBindingsV1 {
                device_id: [2; 32],
                gpu_unique_id: 23,
                kernel_catalog: [6; 32],
                kernel_manifest: [7; 32],
                program_catalog: [4; 32],
                runner_declaration: [8; 32],
            },
            request_generation: 7,
            request_slot: 0,
            target_sha256: sha256_array(&token_bytes(&[11, 12, 99, 14, 15])),
        }
    }

    fn fixture() -> Value {
        let expected = expected();
        json!({
            "authority": "ferric-physical-partial-capture-only",
            "case": CASE,
            "choices": {
                "draft": [11, 12, 13, 14], "draft_bytes": 16,
                "draft_sha256": hex_bytes(&expected.draft_sha256), "encoding": "u32-le",
                "target": [11, 12, 99, 14, 15], "target_bytes": 20,
                "target_sha256": hex_bytes(&expected.target_sha256),
            },
            "format": CAPTURE_FORMAT,
            "identities": {
                "device_id_sha256": hex_bytes(&expected.queue.device_id),
                "gpu_unique_id": 23,
                "kernel_catalog_sha256": hex_bytes(&expected.queue.kernel_catalog),
                "kernel_manifest_sha256": hex_bytes(&expected.queue.kernel_manifest),
                "program_catalog_sha256": hex_bytes(&expected.queue.program_catalog),
                "runner_declaration_sha256": hex_bytes(&expected.queue.runner_declaration),
            },
            "kv_projections": {
                "post_engine_completion_pre_release": expected.post.as_json(),
                "pre_completion": expected.pre.as_json(),
            },
            "milestone": "M1", "nonclaim": NONCLAIM, "obligation_id": "m1.r30",
            "result": {
                "accepted_draft_tokens": 2, "completed_members": 1,
                "correction_or_bonus": 99, "emitted_tokens": [11, 12, 99],
                "events": EVENTS, "externally_published_count": 3,
                "logical_accepted_count": 3, "maximal_prefix_verified": true,
                "physical_page_returns": {"draft": 0, "target": 0, "total": 0},
                "physical_subpage_return_or_reuse_claimed": false,
                "positive_completion": true, "queue_destroyed": true,
                "rejected_suffix_tokens": {"draft": 1, "target": 2},
                "rollback_case_only": true, "strict_prefix_verified": true,
            },
            "status": STATUS, "target": TARGET,
            "trace": {
                "compact_sha256": hex_bytes(&expected.compact_sha256),
                "completion_epoch": 31, "dispatch_generation": 37,
                "plan_id_sha256": hex_bytes(&expected.plan_id),
                "request_generation": 7, "request_slot": 0,
            },
        })
    }

    #[test]
    fn protocol_and_exact_partial_capture_are_accepted() {
        require_protocol().unwrap();
        let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR").unwrap();
        let checked_in = std::fs::read(
            std::path::PathBuf::from(manifest_dir)
                .join("src/bin/ferric-m1-r30-rollback-partial-protocol.json"),
        )
        .unwrap();
        assert_eq!(protocol_bytes().unwrap(), checked_in);
        validate_manifest(&canonical_bytes(&fixture()).unwrap(), &expected()).unwrap();
    }

    #[test]
    fn persisted_bundle_admission_revalidates_rollback_transition() {
        let bytes = canonical_bytes(&fixture()).unwrap();
        let admitted = admit_persisted_bundle(&bytes, &protocol_bytes().unwrap()).unwrap();
        assert_eq!(admitted.device_identity_sha256, "02".repeat(32));
        assert_eq!(admitted.gpu_unique_id, 23);
        assert_eq!(admitted.program_catalog_sha256, "04".repeat(32));

        let mut hostile = fixture();
        hostile["kv_projections"]["post_engine_completion_pre_release"]["target"]
            ["committed_tokens"] = json!(2);
        hostile["kv_projections"]["post_engine_completion_pre_release"]["target"]
            ["resident_tokens"] = json!(2);
        assert!(admit_persisted_bundle(
            &canonical_bytes(&hostile).unwrap(),
            &protocol_bytes().unwrap()
        )
        .is_err());
    }

    #[test]
    fn full_or_nonmaximal_prefix_and_suffix_drift_reject() {
        for mutate in [
            |value: &mut Value| value["result"]["accepted_draft_tokens"] = json!(1),
            |value: &mut Value| value["result"]["accepted_draft_tokens"] = json!(4),
            |value: &mut Value| value["result"]["rejected_suffix_tokens"]["draft"] = json!(2),
            |value: &mut Value| value["result"]["rejected_suffix_tokens"]["target"] = json!(1),
        ] {
            let mut value = fixture();
            mutate(&mut value);
            assert!(validate_manifest(&canonical_bytes(&value).unwrap(), &expected()).is_err());
        }
    }

    #[test]
    fn hostile_projection_release_claim_and_identity_substitution_reject() {
        let mutations: &[fn(&mut Value)] = &[
            |value| {
                value["kv_projections"]["pre_completion"]["target"]["write_pending"] = json!(false)
            },
            |value| {
                value["kv_projections"]["post_engine_completion_pre_release"]["target"]
                    ["write_pending"] = json!(true)
            },
            |value| {
                value["kv_projections"]["post_engine_completion_pre_release"]["draft"]
                    ["committed_tokens"] = json!(2)
            },
            |value| {
                value["kv_projections"]["post_engine_completion_pre_release"]["draft"]
                    ["resident_tokens"] = json!(2)
            },
            |value| {
                value["kv_projections"]["post_engine_completion_pre_release"]["target"]
                    ["active_pages"] = json!(0)
            },
            |value| value["result"]["physical_page_returns"]["target"] = json!(1),
            |value| value["result"]["physical_subpage_return_or_reuse_claimed"] = json!(true),
            |value| value["result"]["queue_destroyed"] = json!(false),
            |value| value["trace"]["dispatch_generation"] = json!(38),
            |value| value["identities"]["device_id_sha256"] = json!(hex_bytes(&[44; 32])),
            |value| value["result"]["events"][0] = json!("substituted"),
        ];
        for mutate in mutations {
            let mut value = fixture();
            mutate(&mut value);
            assert!(validate_manifest(&canonical_bytes(&value).unwrap(), &expected()).is_err());
        }
    }

    #[test]
    fn hostile_protocol_substitutions_reject() {
        let protocol = parse_canonical(&protocol_bytes().unwrap(), "test protocol").unwrap();
        let mutations: &[fn(&mut Value)] = &[
            |value| value["status"] = json!("evidence"),
            |value| value["case"] = json!("cancellation"),
            |value| value["bundle_files"] = json!(["capture.json"]),
            |value| value["lifecycle"][8] = json!("physical-page-released"),
            |value| value["required_complete_case_roster"][4] = json!("other"),
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
            "ferric-m1-r30-rollback-publish-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let capture = canonical_bytes(&fixture()).unwrap();
        publish(
            &output,
            CaptureArtifactV1 {
                bytes: capture.clone(),
                expected: expected(),
            },
        )
        .unwrap();
        assert_eq!(std::fs::read(output.join("capture.json")).unwrap(), capture);
        assert_eq!(
            std::fs::read(output.join("protocol.json")).unwrap(),
            protocol_bytes().unwrap()
        );
        assert!(publish(
            &output,
            CaptureArtifactV1 {
                bytes: capture,
                expected: expected(),
            },
        )
        .is_err());
        std::fs::remove_dir_all(output).unwrap();
    }
}
