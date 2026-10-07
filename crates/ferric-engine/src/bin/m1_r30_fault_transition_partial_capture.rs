//! Canonical partial m1.r30 physical queue-transition fault capture.
//!
//! The producer consumes one real completed-and-recycled target-prefill queue
//! before any completed read, terminalizes the Ferric Engine, proves retry
//! denial, and performs ordinary returning queue teardown. It deliberately
//! grants no native KFD or GPU fault authority.

use super::{
    canonical_bytes, completion_wait_policy_contract, decode_identity, exact_object, field,
    fixed_r30_prefill_input_bytes, hex_bytes, parse_canonical, sha256_array, sha256_hex,
    CaptureResult, R30FaultTransitionCaptureV1, R30PhysicalCaptureBindingsV1, StagingOutput,
    Workload, R30_PREFILL_ACTIVE_TOKENS, R30_PREFILL_INPUT_BYTES, R30_PREFILL_INPUT_TOKEN, TARGET,
};
use fe2o3_service_host::{
    ServiceQualificationQueueFaultPointV1, SERVICE_QUALIFICATION_QUEUE_FAULT_CONTRACT_SHA256_V1,
    SERVICE_QUEUE_OWNERSHIP_MANIFEST_SHA256_V1,
};
use ferric_engine::{M1PhysicalFixedBatchShapeV1, M1PhysicalRunnerV1};
use serde_json::{json, Map, Value};
use std::path::Path;

pub(super) const COMMAND: &str = "capture-r30-fault-transition";
const CAPTURE_FORMAT: &str = "FERRIC-M1-R30-FAULT-TRANSITION-PARTIAL-CAPTURE-V1";
const PROTOCOL_FORMAT: &str = "FERRIC-M1-R30-FAULT-TRANSITION-PARTIAL-PROTOCOL-V1";
const WORKLOAD_FORMAT: &str = "FERRIC-M1-R30-FAULT-TRANSITION-WORKLOAD-V1";
const AUTHORITY: &str = "ferric-physical-queue-transition-fault-capture-only";
const STATUS: &str = "partial-non-evidence";
const CASE: &str = "target-prefill-s1-post-recycle-before-completed-read-attempt";
const INJECTION_POINT: &str = "post-recycle-before-completed-read-attempt";
const NONCLAIM: &str = "One real target-prefill S1 queue was deliberately terminalized by Ferric after exact completion-signal recycle and before any completed-read attempt. The Engine denied a new admission and the native queue then followed ordinary returning teardown. This service transition is not a native KFD or GPU fault, did not reset the GPU, does not establish global resource reclamation, supplies no external or independent validation, is not benchmark evidence, and does not close m1.r30/M1.";
const PROTOCOL_NONCLAIM: &str = "Partial physical queue-transition protocol only. It establishes one Ferric-owned post-recycle service transition, logical Engine quarantine, retry denial, and ordinary queue teardown. It grants no native KFD/device fault, GPU-reset, global-resource, evidence, hardware-correctness, performance, qualification, or m1.r30/M1 closure authority.";

const EVENTS: &[&str] = &[
    "published-target-prefill-s1",
    "completed-fixed-batch",
    "recycled-all-completion-signals",
    "injected-service-queue-transition-fault",
    "quarantined-ferric-engine",
    "denied-post-fault-admission",
    "destroyed-native-queue",
    "released-service-allocation-roster",
    "canonical-publication-ready",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ExpectedBindingsV1 {
    allocation_count: usize,
    completion_epoch: u64,
    device_bytes: u64,
    device_id: [u8; 32],
    dispatch_generation: u64,
    gpu_unique_id: u64,
    host_bytes: u64,
    kernel_catalog: [u8; 32],
    kernel_manifest: [u8; 32],
    packet_count: usize,
    program_catalog: [u8; 32],
    protocol_sha256: [u8; 32],
    released_queue_resource_kinds: u8,
    runner_declaration: [u8; 32],
}

pub(super) struct ClosedCaptureInputsV1<'a> {
    pub(super) capture: &'a R30FaultTransitionCaptureV1,
    pub(super) gpu_unique_id: u64,
    pub(super) runner: &'a M1PhysicalRunnerV1,
    pub(super) workload: &'a Workload,
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
    validate_fixed_workload(inputs.workload)?;
    require_protocol()?;
    let teardown = &inputs.capture.teardown;
    let release = *teardown.release();
    let allocations = release.allocations_released();
    if teardown.shape() != M1PhysicalFixedBatchShapeV1::TargetOnly
        || teardown.fault_point()
            != ServiceQualificationQueueFaultPointV1::PostRecycleBeforeCompletedReadAttempt
        || teardown.queue_epoch().value() == 0
        || teardown.dispatch_generation() == 0
        || release.dispatch_generation() != teardown.dispatch_generation()
        || release.queue_destroyed().released_resources() == 0
        || allocations.allocation_count() == 0
        || !inputs.capture._engine.is_faulted()
        || inputs.capture.retry_denial != ferric_engine::EngineError::Faulted
        || inputs.gpu_unique_id == 0
        || !inputs.capture.device_id.is_present()
    {
        return Err("fault-transition physical lifecycle custody drifted".to_owned());
    }
    let protocol = protocol_bytes()?;
    let expected = ExpectedBindingsV1 {
        allocation_count: allocations.allocation_count(),
        completion_epoch: teardown.queue_epoch().value(),
        device_bytes: allocations.device_bytes(),
        device_id: *inputs.capture.device_id.as_bytes(),
        dispatch_generation: teardown.dispatch_generation(),
        gpu_unique_id: inputs.gpu_unique_id,
        host_bytes: allocations.host_bytes(),
        kernel_catalog: *inputs.runner.kernel_catalog_id().as_bytes(),
        kernel_manifest: *inputs.runner.kernel_artifact_manifest_id().as_bytes(),
        packet_count: teardown.shape().packet_count(),
        program_catalog: *inputs.runner.program_catalog_id().as_bytes(),
        protocol_sha256: sha256_array(&protocol),
        released_queue_resource_kinds: release.queue_destroyed().released_resources(),
        runner_declaration: *inputs.runner.declaration_id().as_bytes(),
    };
    let bytes = canonical_bytes(&capture_value(&expected))?;
    validate_manifest(&bytes, &expected)?;
    Ok(CaptureArtifactV1 { bytes, expected })
}

fn capture_value(expected: &ExpectedBindingsV1) -> Value {
    json!({
        "authority": AUTHORITY,
        "case": CASE,
        "format": CAPTURE_FORMAT,
        "hardware_claim": "none",
        "identities": {
            "device_id_sha256": hex_bytes(&expected.device_id),
            "fe2o3_qualification_fault_contract_sha256": SERVICE_QUALIFICATION_QUEUE_FAULT_CONTRACT_SHA256_V1,
            "fe2o3_service_queue_manifest_sha256": SERVICE_QUEUE_OWNERSHIP_MANIFEST_SHA256_V1,
            "gpu_unique_id": expected.gpu_unique_id,
            "kernel_catalog_sha256": hex_bytes(&expected.kernel_catalog),
            "kernel_manifest_sha256": hex_bytes(&expected.kernel_manifest),
            "program_catalog_sha256": hex_bytes(&expected.program_catalog),
            "protocol_sha256": hex_bytes(&expected.protocol_sha256),
            "runner_declaration_sha256": hex_bytes(&expected.runner_declaration),
        },
        "milestone": "M1",
        "nonclaim": NONCLAIM,
        "obligation_id": "m1.r30",
        "result": {
            "device_fault_observed": false,
            "engine_quarantined": true,
            "events": EVENTS,
            "injection_point": INJECTION_POINT,
            "native_kfd_fault_observed": false,
            "queue_destroyed": true,
            "queue_live_resources_after": 0,
            "released_allocation_count": expected.allocation_count,
            "released_device_bytes": expected.device_bytes,
            "released_host_bytes": expected.host_bytes,
            "released_queue_resource_kinds": expected.released_queue_resource_kinds,
            "retry_denied": true,
            "service_transition_injected": true,
            "synthetic_kfd_error": false,
        },
        "status": STATUS,
        "target": TARGET,
        "trace": {
            "completion_epoch": expected.completion_epoch,
            "dispatch_generation": expected.dispatch_generation,
            "fixed_batch_shape": "target-only",
            "packet_count": expected.packet_count,
        },
        "workload": fixed_workload_manifest(),
    })
}

fn fixed_workload_contract() -> Value {
    json!({
        "active_length": R30_PREFILL_ACTIVE_TOKENS,
        "case": "target-prefill-s1-t128-post-recycle-fault-transition",
        "completion_wait_policy": completion_wait_policy_contract(),
        "context_length": 0,
        "format": WORKLOAD_FORMAT,
        "input_bytes": R30_PREFILL_INPUT_BYTES,
        "input_token": R30_PREFILL_INPUT_TOKEN,
        "input_token_count": R30_PREFILL_ACTIVE_TOKENS,
        "lane_count": 1,
        "selection": "target-prefill-s1-t128",
    })
}

fn fixed_workload_manifest() -> Value {
    json!({
        "active_length": R30_PREFILL_ACTIVE_TOKENS,
        "completion_wait_policy": completion_wait_policy_contract(),
        "context_length": 0,
        "input_bytes": R30_PREFILL_INPUT_BYTES,
        "input_payload_sha256": sha256_hex(&fixed_r30_prefill_input_bytes()),
        "input_token": R30_PREFILL_INPUT_TOKEN,
        "input_token_count": R30_PREFILL_ACTIVE_TOKENS,
        "lane_count": 1,
        "selection": "target-prefill-s1-t128",
        "workload_sha256": sha256_hex(&canonical_bytes(&fixed_workload_contract()).expect("fixed workload is canonical")),
    })
}

fn validate_fixed_workload(workload: &Workload) -> CaptureResult<()> {
    if workload.bytes != canonical_bytes(&fixed_workload_contract())?
        || workload.input_path != Path::new("frozen-r30-fault-transition-input-u32le")
        || workload.input_bytes != R30_PREFILL_INPUT_BYTES
        || workload.input_sha256 != sha256_hex(&fixed_r30_prefill_input_bytes())
        || workload.kind != "prefill-s1-t128"
        || workload.lanes
            != [super::LaneInput {
                active_length: R30_PREFILL_ACTIVE_TOKENS,
                context_length: 0,
            }]
        || workload.selection.role != ferric_spec::Qwen3ModelRole::Target8B
        || workload.selection.mode != ferric_spec::Qwen3ExecutionMode::Prefill
        || workload.selection.bucket != ferric_spec::Qwen3PlanBucket::PrefillS1T128
    {
        return Err("fault-transition workload differs from the fixed S1/T128 contract".to_owned());
    }
    Ok(())
}

pub(super) fn publish(output: &Path, artifact: CaptureArtifactV1) -> CaptureResult<()> {
    validate_manifest(&artifact.bytes, &artifact.expected)?;
    let protocol = protocol_bytes()?;
    let mut staging = StagingOutput::create(output)?;
    staging.write("capture.json", &artifact.bytes)?;
    staging.write("protocol.json", &protocol)?;
    staging.publish_exact(&[
        ("capture.json", artifact.bytes.as_slice()),
        ("protocol.json", protocol.as_slice()),
    ])
}

pub(super) fn admit_persisted_bundle(
    capture: &[u8],
    protocol: &[u8],
) -> CaptureResult<R30PhysicalCaptureBindingsV1> {
    if protocol != protocol_bytes()? {
        return Err("fault-transition protocol bytes drifted".to_owned());
    }
    let value = parse_canonical(capture, "persisted r30 fault-transition capture")?;
    let root = value
        .as_object()
        .ok_or_else(|| "persisted fault-transition capture must be an object".to_owned())?;
    let identities = field(root, "identities")?
        .as_object()
        .ok_or_else(|| "persisted fault-transition identities must be an object".to_owned())?;
    let result = field(root, "result")?
        .as_object()
        .ok_or_else(|| "persisted fault-transition result must be an object".to_owned())?;
    let trace = field(root, "trace")?
        .as_object()
        .ok_or_else(|| "persisted fault-transition trace must be an object".to_owned())?;
    let expected = ExpectedBindingsV1 {
        allocation_count: persisted_usize(result, "released_allocation_count")?,
        completion_epoch: persisted_u64(trace, "completion_epoch")?,
        device_bytes: persisted_u64(result, "released_device_bytes")?,
        device_id: persisted_digest(identities, "device_id_sha256")?,
        dispatch_generation: persisted_u64(trace, "dispatch_generation")?,
        gpu_unique_id: persisted_u64(identities, "gpu_unique_id")?,
        host_bytes: persisted_u64(result, "released_host_bytes")?,
        kernel_catalog: persisted_digest(identities, "kernel_catalog_sha256")?,
        kernel_manifest: persisted_digest(identities, "kernel_manifest_sha256")?,
        packet_count: persisted_usize(trace, "packet_count")?,
        program_catalog: persisted_digest(identities, "program_catalog_sha256")?,
        protocol_sha256: sha256_array(protocol),
        released_queue_resource_kinds: u8::try_from(persisted_u64(
            result,
            "released_queue_resource_kinds",
        )?)
        .map_err(|_| "released queue-resource kind count does not fit u8".to_owned())?,
        runner_declaration: persisted_digest(identities, "runner_declaration_sha256")?,
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

fn validate_manifest(bytes: &[u8], expected: &ExpectedBindingsV1) -> CaptureResult<()> {
    let value = parse_canonical(bytes, "r30 fault-transition capture")?;
    let root = exact_object(
        &value,
        &[
            "authority",
            "case",
            "format",
            "hardware_claim",
            "identities",
            "milestone",
            "nonclaim",
            "obligation_id",
            "result",
            "status",
            "target",
            "trace",
            "workload",
        ],
        "r30 fault-transition capture",
    )?;
    require_string(root, "authority", AUTHORITY)?;
    require_string(root, "case", CASE)?;
    require_string(root, "format", CAPTURE_FORMAT)?;
    require_string(root, "hardware_claim", "none")?;
    require_string(root, "milestone", "M1")?;
    require_string(root, "nonclaim", NONCLAIM)?;
    require_string(root, "obligation_id", "m1.r30")?;
    require_string(root, "status", STATUS)?;
    require_string(root, "target", TARGET)?;
    if field(root, "workload")? != &fixed_workload_manifest() {
        return Err("fault-transition fixed workload manifest drifted".to_owned());
    }

    let identities = exact_object(
        field(root, "identities")?,
        &[
            "device_id_sha256",
            "fe2o3_qualification_fault_contract_sha256",
            "fe2o3_service_queue_manifest_sha256",
            "gpu_unique_id",
            "kernel_catalog_sha256",
            "kernel_manifest_sha256",
            "program_catalog_sha256",
            "protocol_sha256",
            "runner_declaration_sha256",
        ],
        "fault-transition identities",
    )?;
    require_digest(identities, "device_id_sha256", &expected.device_id)?;
    require_string(
        identities,
        "fe2o3_qualification_fault_contract_sha256",
        SERVICE_QUALIFICATION_QUEUE_FAULT_CONTRACT_SHA256_V1,
    )?;
    require_string(
        identities,
        "fe2o3_service_queue_manifest_sha256",
        SERVICE_QUEUE_OWNERSHIP_MANIFEST_SHA256_V1,
    )?;
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

    let result = exact_object(
        field(root, "result")?,
        &[
            "device_fault_observed",
            "engine_quarantined",
            "events",
            "injection_point",
            "native_kfd_fault_observed",
            "queue_destroyed",
            "queue_live_resources_after",
            "released_allocation_count",
            "released_device_bytes",
            "released_host_bytes",
            "released_queue_resource_kinds",
            "retry_denied",
            "service_transition_injected",
            "synthetic_kfd_error",
        ],
        "fault-transition result",
    )?;
    for name in [
        "engine_quarantined",
        "queue_destroyed",
        "retry_denied",
        "service_transition_injected",
    ] {
        require_bool(result, name, true)?;
    }
    for name in [
        "device_fault_observed",
        "native_kfd_fault_observed",
        "synthetic_kfd_error",
    ] {
        require_bool(result, name, false)?;
    }
    require_string(result, "injection_point", INJECTION_POINT)?;
    if field(result, "events")? != &json!(EVENTS) {
        return Err("fault-transition event trace drifted".to_owned());
    }
    require_u64(result, "queue_live_resources_after", 0)?;
    require_u64(
        result,
        "released_allocation_count",
        u64::try_from(expected.allocation_count).unwrap_or(u64::MAX),
    )?;
    require_u64(result, "released_device_bytes", expected.device_bytes)?;
    require_u64(result, "released_host_bytes", expected.host_bytes)?;
    require_u64(
        result,
        "released_queue_resource_kinds",
        u64::from(expected.released_queue_resource_kinds),
    )?;

    let trace = exact_object(
        field(root, "trace")?,
        &[
            "completion_epoch",
            "dispatch_generation",
            "fixed_batch_shape",
            "packet_count",
        ],
        "fault-transition trace",
    )?;
    require_u64(trace, "completion_epoch", expected.completion_epoch)?;
    require_u64(trace, "dispatch_generation", expected.dispatch_generation)?;
    require_string(trace, "fixed_batch_shape", "target-only")?;
    require_u64(
        trace,
        "packet_count",
        u64::try_from(expected.packet_count).unwrap_or(u64::MAX),
    )?;
    if expected.completion_epoch == 0
        || expected.dispatch_generation == 0
        || expected.gpu_unique_id == 0
        || expected.allocation_count == 0
        || expected.released_queue_resource_kinds == 0
    {
        return Err(
            "fault-transition retained identities and release counts must be nonzero".to_owned(),
        );
    }
    Ok(())
}

fn protocol_value() -> Value {
    json!({
        "authority": AUTHORITY,
        "feature_contract": {
            "cargo_feature": "qualification-fault-injection",
            "fe2o3_qualification_fault_contract_sha256": SERVICE_QUALIFICATION_QUEUE_FAULT_CONTRACT_SHA256_V1,
            "fe2o3_service_queue_manifest_sha256": SERVICE_QUEUE_OWNERSHIP_MANIFEST_SHA256_V1,
        },
        "format": PROTOCOL_FORMAT,
        "hardware_claim": "none",
        "injection_point": INJECTION_POINT,
        "nonclaim": PROTOCOL_NONCLAIM,
        "steps": [
            "authenticate fixed target-prefill S1/T128 inputs and physical runner",
            "publish and positively complete the real fixed batch",
            "recycle every completion signal",
            "consume recycled custody before any completed-read attempt",
            "terminalize the Ferric Engine and require admission retry denial",
            "destroy the native queue through ordinary returning teardown",
            "publish only addressless transition and release observations",
        ],
        "target": TARGET,
    })
}

fn protocol_bytes() -> CaptureResult<Vec<u8>> {
    canonical_bytes(&protocol_value())
}

fn require_protocol() -> CaptureResult<()> {
    let bytes = protocol_bytes()?;
    let parsed = parse_canonical(&bytes, "fault-transition protocol")?;
    if parsed != protocol_value() {
        return Err("fault-transition protocol failed canonical round trip".to_owned());
    }
    Ok(())
}

fn persisted_digest(object: &Map<String, Value>, name: &str) -> CaptureResult<[u8; 32]> {
    let value = field(object, name)?
        .as_str()
        .ok_or_else(|| format!("persisted fault-transition {name} must be a string"))?;
    Ok(*decode_identity(value)?.as_bytes())
}

fn persisted_u64(object: &Map<String, Value>, name: &str) -> CaptureResult<u64> {
    field(object, name)?
        .as_u64()
        .ok_or_else(|| format!("persisted fault-transition {name} must be a nonnegative integer"))
}

fn persisted_usize(object: &Map<String, Value>, name: &str) -> CaptureResult<usize> {
    usize::try_from(persisted_u64(object, name)?)
        .map_err(|_| format!("persisted fault-transition {name} does not fit usize"))
}

fn require_string(object: &Map<String, Value>, name: &str, expected: &str) -> CaptureResult<()> {
    if field(object, name)?.as_str() != Some(expected) {
        return Err(format!("fault-transition {name} drifted"));
    }
    Ok(())
}

fn require_bool(object: &Map<String, Value>, name: &str, expected: bool) -> CaptureResult<()> {
    if field(object, name)?.as_bool() != Some(expected) {
        return Err(format!("fault-transition {name} drifted"));
    }
    Ok(())
}

fn require_u64(object: &Map<String, Value>, name: &str, expected: u64) -> CaptureResult<()> {
    if field(object, name)?.as_u64() != Some(expected) {
        return Err(format!("fault-transition {name} drifted"));
    }
    Ok(())
}

fn require_digest(
    object: &Map<String, Value>,
    name: &str,
    expected: &[u8; 32],
) -> CaptureResult<()> {
    require_string(object, name, &hex_bytes(expected))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expected() -> ExpectedBindingsV1 {
        ExpectedBindingsV1 {
            allocation_count: 7,
            completion_epoch: 3,
            device_bytes: 16_384,
            device_id: [1; 32],
            dispatch_generation: 5,
            gpu_unique_id: 9,
            host_bytes: 8_192,
            kernel_catalog: [2; 32],
            kernel_manifest: [3; 32],
            packet_count: 545,
            program_catalog: [4; 32],
            protocol_sha256: sha256_array(&protocol_bytes().unwrap()),
            released_queue_resource_kinds: 5,
            runner_declaration: [6; 32],
        }
    }

    #[test]
    fn canonical_capture_and_persisted_admission_are_exact() {
        let expected = expected();
        let capture = canonical_bytes(&capture_value(&expected)).unwrap();
        let protocol = protocol_bytes().unwrap();
        validate_manifest(&capture, &expected).unwrap();
        let admitted = admit_persisted_bundle(&capture, &protocol).unwrap();
        assert_eq!(admitted.gpu_unique_id, expected.gpu_unique_id);
        assert_eq!(
            admitted.kernel_artifact_manifest_sha256,
            hex_bytes(&expected.kernel_manifest)
        );
    }

    #[test]
    fn hardware_promotion_and_transition_drift_fail_closed() {
        let expected = expected();
        let original = capture_value(&expected);
        for (path, value) in [
            (&["hardware_claim", ""] as &[&str], json!("gfx942-correct")),
            (&["result", "retry_denied"] as &[&str], json!(false)),
            (&["result", "queue_destroyed"] as &[&str], json!(false)),
            (
                &["result", "native_kfd_fault_observed"] as &[&str],
                json!(true),
            ),
            (&["trace", "dispatch_generation"] as &[&str], json!(6)),
        ] {
            let mut hostile = original.clone();
            if path[1].is_empty() {
                hostile[path[0]] = value;
            } else {
                hostile[path[0]][path[1]] = value;
            }
            let bytes = canonical_bytes(&hostile).unwrap();
            assert!(validate_manifest(&bytes, &expected).is_err());
        }
    }
}
