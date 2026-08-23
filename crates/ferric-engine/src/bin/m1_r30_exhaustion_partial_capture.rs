//! Canonical partial m1.r30 physical KV exhaustion capture.
//!
//! This capture authenticates one initialized gfx942 model-memory owner and
//! exercises the exact per-request target-page ledger boundary. It deliberately
//! does not claim kernel dispatch, physical memory pressure, or M1 evidence.

use super::{
    canonical_bytes, decode_identity, exact_object, expect_string, field, hex_bytes,
    parse_canonical, require_sha256, CaptureResult, R30PhysicalCaptureBindingsV1, StagingOutput,
    TARGET,
};
use ferric_engine::{Gfx942DeviceBinding, M1PhysicalRunnerV1};
use ferric_spec::{Identity, RequestId, M1_KV_PHYSICAL_PAGE_SLOTS};
use serde_json::{json, Map, Value};
use std::path::Path;

pub(super) const COMMAND: &str = "capture-r30-exhaustion";
const CAPTURE_FORMAT: &str = "FERRIC-M1-R30-EXHAUSTION-PARTIAL-CAPTURE-V1";
const PROTOCOL_FORMAT: &str = "FERRIC-M1-R30-EXHAUSTION-PARTIAL-PROTOCOL-V1";
const STATUS: &str = "partial-non-evidence";
const CASE: &str = "exhaustion-target-kv-p512";
const NONCLAIM: &str = "Authenticated Ferric custody for one initialized physical target-KV page ledger only. After all 512 request-local slots are leased, the case establishes transactional rejection of occupied slot 0, separately establishes the static P512 index bound, completely returns the unpublished roster, and observes generation-advanced reuse in the source runtime. It does not establish device-memory exhaustion or queue pressure, dispatch a kernel, create a queue, establish canary integrity or injected device-fault handling, supply external or independent validation, prove general hardware correctness, qualify performance, or close m1.r30/M1.";
const PROTOCOL_NONCLAIM: &str = "Partial physical target-KV ledger saturation protocol only: occupied-slot rejection after all 512 request-local slots are leased is distinct from the static P512 index bound. It grants no kernel-dispatch, queue, device-memory-exhaustion or pressure, canary, fault-injection, evidence, hardware-correctness, performance, qualification, or m1.r30/M1 closure authority.";

const EVENTS: &[&str] = &[
    "physical-model-memory-initialized",
    "engine-request-admitted",
    "exact-target-page-capacity-leased",
    "occupied-target-page-rejected",
    "out-of-range-target-page-rejected",
    "leased-page-identities-revalidated",
    "unpublished-pages-returned",
    "page-zero-released-at-next-generation",
    "reused-page-returned",
    "engine-request-retired",
    "engine-request-reclaimed",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ExpectedBindingsV1 {
    device_id: [u8; 32],
    draft_arena: [u8; 32],
    first_generation: u32,
    gpu_unique_id: u64,
    kernel_catalog: [u8; 32],
    kernel_manifest: [u8; 32],
    program_catalog: [u8; 32],
    request_generation: u32,
    request_slot: u32,
    reused_generation: u32,
    runner_declaration: [u8; 32],
    target_arena: [u8; 32],
}

pub(super) struct CaptureInputsV1<'a> {
    pub(super) checked_leases: usize,
    pub(super) device: Gfx942DeviceBinding,
    pub(super) draft_arena: Identity,
    pub(super) engine_reclaimed: RequestId,
    pub(super) first_generation: u32,
    pub(super) occupied_page_rejected: bool,
    pub(super) out_of_range_page_rejected: bool,
    pub(super) request: RequestId,
    pub(super) returned_pages: usize,
    pub(super) reused_generation: u32,
    pub(super) reused_returned_pages: usize,
    pub(super) runner: &'a M1PhysicalRunnerV1,
    pub(super) target_arena: Identity,
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

pub(super) fn manifest(inputs: CaptureInputsV1<'_>) -> CaptureResult<CaptureArtifactV1> {
    require_protocol()?;
    let page_capacity = M1_KV_PHYSICAL_PAGE_SLOTS;
    let reused_expected = inputs
        .first_generation
        .checked_add(1)
        .ok_or_else(|| "partial exhaustion page generation overflowed".to_owned())?;
    if inputs.checked_leases != page_capacity
        || inputs.returned_pages != page_capacity
        || inputs.reused_returned_pages != 1
        || inputs.reused_generation != reused_expected
        || inputs.engine_reclaimed != inputs.request
        || !inputs.occupied_page_rejected
        || !inputs.out_of_range_page_rejected
        || inputs.device.gpu_unique_id() == 0
        || !inputs.device.device_id().is_present()
        || !inputs.target_arena.is_present()
        || !inputs.draft_arena.is_present()
        || inputs.target_arena == inputs.draft_arena
    {
        return Err("partial exhaustion retained custody is incomplete or inconsistent".to_owned());
    }
    let expected = ExpectedBindingsV1 {
        device_id: *inputs.device.device_id().as_bytes(),
        draft_arena: *inputs.draft_arena.as_bytes(),
        first_generation: inputs.first_generation,
        gpu_unique_id: inputs.device.gpu_unique_id(),
        kernel_catalog: *inputs.runner.kernel_catalog_id().as_bytes(),
        kernel_manifest: *inputs.runner.kernel_artifact_manifest_id().as_bytes(),
        program_catalog: *inputs.runner.program_catalog_id().as_bytes(),
        request_generation: inputs.request.generation(),
        request_slot: inputs.request.slot(),
        reused_generation: inputs.reused_generation,
        runner_declaration: *inputs.runner.declaration_id().as_bytes(),
        target_arena: *inputs.target_arena.as_bytes(),
    };
    let value = json!({
        "authority": "ferric-physical-partial-capture-only",
        "case": CASE,
        "format": CAPTURE_FORMAT,
        "identities": {
            "device_id_sha256": hex_bytes(&expected.device_id),
            "draft_kv_arena_sha256": hex_bytes(&expected.draft_arena),
            "gpu_unique_id": expected.gpu_unique_id,
            "kernel_catalog_sha256": hex_bytes(&expected.kernel_catalog),
            "kernel_manifest_sha256": hex_bytes(&expected.kernel_manifest),
            "program_catalog_sha256": hex_bytes(&expected.program_catalog),
            "runner_declaration_sha256": hex_bytes(&expected.runner_declaration),
            "target_kv_arena_sha256": hex_bytes(&expected.target_arena),
        },
        "milestone": "M1",
        "nonclaim": NONCLAIM,
        "obligation_id": "m1.r30",
        "result": {
            "checked_leases": inputs.checked_leases,
            "engine_request_reclaimed": true,
            "events": EVENTS,
            "first_generation": expected.first_generation,
            "kernel_dispatch_performed": false,
            "occupied_page_index": 0,
            "occupied_page_rejection": "page-already-leased",
            "occupied_rejection_transactional": inputs.occupied_page_rejected,
            "out_of_range_page_index": page_capacity,
            "out_of_range_rejection": "page-out-of-range",
            "out_of_range_rejection_transactional": inputs.out_of_range_page_rejected,
            "page_capacity": page_capacity,
            "returned_pages": inputs.returned_pages,
            "reuse_page_index": 0,
            "reused_generation": expected.reused_generation,
            "reused_returned_pages": inputs.reused_returned_pages,
            "role": "target-8b",
        },
        "status": STATUS,
        "target": TARGET,
        "trace": {
            "request_generation": expected.request_generation,
            "request_slot": expected.request_slot,
        },
    });
    let bytes = canonical_bytes(&value)?;
    validate_manifest(&bytes, &expected)?;
    Ok(CaptureArtifactV1 { bytes, expected })
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
        return Err("partial r30 exhaustion protocol bytes drifted".to_owned());
    }
    let value = parse_canonical(capture, "persisted partial r30 exhaustion capture")?;
    let root = value
        .as_object()
        .ok_or_else(|| "persisted partial r30 exhaustion capture must be an object".to_owned())?;
    let identities = field(root, "identities")?.as_object().ok_or_else(|| {
        "persisted partial r30 exhaustion identities must be an object".to_owned()
    })?;
    let result = field(root, "result")?
        .as_object()
        .ok_or_else(|| "persisted partial r30 exhaustion result must be an object".to_owned())?;
    let trace = field(root, "trace")?
        .as_object()
        .ok_or_else(|| "persisted partial r30 exhaustion trace must be an object".to_owned())?;
    let expected = ExpectedBindingsV1 {
        device_id: persisted_digest(identities, "device_id_sha256")?,
        draft_arena: persisted_digest(identities, "draft_kv_arena_sha256")?,
        first_generation: u32::try_from(persisted_u64(result, "first_generation")?)
            .map_err(|_| "persisted exhaustion first generation does not fit u32".to_owned())?,
        gpu_unique_id: persisted_u64(identities, "gpu_unique_id")?,
        kernel_catalog: persisted_digest(identities, "kernel_catalog_sha256")?,
        kernel_manifest: persisted_digest(identities, "kernel_manifest_sha256")?,
        program_catalog: persisted_digest(identities, "program_catalog_sha256")?,
        request_generation: u32::try_from(persisted_u64(trace, "request_generation")?)
            .map_err(|_| "persisted exhaustion request generation does not fit u32".to_owned())?,
        request_slot: u32::try_from(persisted_u64(trace, "request_slot")?)
            .map_err(|_| "persisted exhaustion request slot does not fit u32".to_owned())?,
        reused_generation: u32::try_from(persisted_u64(result, "reused_generation")?)
            .map_err(|_| "persisted exhaustion reused generation does not fit u32".to_owned())?,
        runner_declaration: persisted_digest(identities, "runner_declaration_sha256")?,
        target_arena: persisted_digest(identities, "target_kv_arena_sha256")?,
    };
    if expected.reused_generation
        != expected
            .first_generation
            .checked_add(1)
            .ok_or_else(|| "persisted exhaustion page generation overflowed".to_owned())?
        || expected.target_arena == expected.draft_arena
        || expected.request_generation == 0
    {
        return Err("persisted exhaustion generation or arena relation drifted".to_owned());
    }
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
        .ok_or_else(|| format!("persisted exhaustion {name} must be a string"))?;
    Ok(*decode_identity(value)?.as_bytes())
}

fn persisted_u64(object: &Map<String, Value>, name: &str) -> CaptureResult<u64> {
    field(object, name)?
        .as_u64()
        .ok_or_else(|| format!("persisted exhaustion {name} must be a nonnegative integer"))
}

fn validate_manifest(bytes: &[u8], expected: &ExpectedBindingsV1) -> CaptureResult<()> {
    let value = parse_canonical(bytes, "partial r30 exhaustion capture")?;
    let root = exact_object(
        &value,
        &[
            "authority",
            "case",
            "format",
            "identities",
            "milestone",
            "nonclaim",
            "obligation_id",
            "result",
            "status",
            "target",
            "trace",
        ],
        "partial r30 exhaustion capture",
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
            "draft_kv_arena_sha256",
            "gpu_unique_id",
            "kernel_catalog_sha256",
            "kernel_manifest_sha256",
            "program_catalog_sha256",
            "runner_declaration_sha256",
            "target_kv_arena_sha256",
        ],
        "partial exhaustion identities",
    )?;
    require_exact_sha256(identities, "device_id_sha256", &expected.device_id)?;
    require_exact_sha256(identities, "draft_kv_arena_sha256", &expected.draft_arena)?;
    require_exact_u64(identities, "gpu_unique_id", expected.gpu_unique_id)?;
    require_exact_sha256(
        identities,
        "kernel_catalog_sha256",
        &expected.kernel_catalog,
    )?;
    require_exact_sha256(
        identities,
        "kernel_manifest_sha256",
        &expected.kernel_manifest,
    )?;
    require_exact_sha256(
        identities,
        "program_catalog_sha256",
        &expected.program_catalog,
    )?;
    require_exact_sha256(
        identities,
        "runner_declaration_sha256",
        &expected.runner_declaration,
    )?;
    require_exact_sha256(identities, "target_kv_arena_sha256", &expected.target_arena)?;

    let result = exact_object(
        field(root, "result")?,
        &[
            "checked_leases",
            "engine_request_reclaimed",
            "events",
            "first_generation",
            "kernel_dispatch_performed",
            "occupied_page_index",
            "occupied_page_rejection",
            "occupied_rejection_transactional",
            "out_of_range_page_index",
            "out_of_range_rejection",
            "out_of_range_rejection_transactional",
            "page_capacity",
            "returned_pages",
            "reuse_page_index",
            "reused_generation",
            "reused_returned_pages",
            "role",
        ],
        "partial exhaustion result",
    )?;
    let capacity = u64::try_from(M1_KV_PHYSICAL_PAGE_SLOTS)
        .map_err(|_| "partial exhaustion page capacity exceeds u64".to_owned())?;
    for (name, expected_value) in [
        ("checked_leases", capacity),
        ("first_generation", u64::from(expected.first_generation)),
        ("occupied_page_index", 0),
        ("out_of_range_page_index", capacity),
        ("page_capacity", capacity),
        ("returned_pages", capacity),
        ("reuse_page_index", 0),
        ("reused_generation", u64::from(expected.reused_generation)),
        ("reused_returned_pages", 1),
    ] {
        require_exact_u64(result, name, expected_value)?;
    }
    expect_string(result, "occupied_page_rejection", "page-already-leased")?;
    expect_string(result, "out_of_range_rejection", "page-out-of-range")?;
    expect_string(result, "role", "target-8b")?;
    for name in [
        "engine_request_reclaimed",
        "occupied_rejection_transactional",
        "out_of_range_rejection_transactional",
    ] {
        if field(result, name)?.as_bool() != Some(true) {
            return Err(format!("partial exhaustion {name} must be true"));
        }
    }
    if field(result, "kernel_dispatch_performed")?.as_bool() != Some(false) {
        return Err("partial exhaustion cannot claim kernel dispatch".to_owned());
    }
    require_exact_strings(result, "events", EVENTS, "lifecycle")?;

    let trace = exact_object(
        field(root, "trace")?,
        &["request_generation", "request_slot"],
        "partial exhaustion trace",
    )?;
    require_exact_u64(
        trace,
        "request_generation",
        u64::from(expected.request_generation),
    )?;
    require_exact_u64(trace, "request_slot", u64::from(expected.request_slot))
}

pub(super) fn require_protocol() -> CaptureResult<()> {
    validate_protocol(&protocol_bytes()?)
}

fn validate_protocol(bytes: &[u8]) -> CaptureResult<()> {
    let value = parse_canonical(bytes, "partial r30 exhaustion protocol")?;
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
        "partial r30 exhaustion protocol",
    )?;
    expect_string(
        root,
        "authority",
        "ferric-m1-r30-exhaustion-partial-protocol-only",
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
        "authority": "ferric-m1-r30-exhaustion-partial-protocol-only",
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

fn require_exact_strings(
    object: &Map<String, Value>,
    name: &str,
    expected: &[&str],
    context: &str,
) -> CaptureResult<()> {
    let actual = field(object, name)?
        .as_array()
        .and_then(|items| items.iter().map(Value::as_str).collect::<Option<Vec<_>>>())
        .ok_or_else(|| format!("partial exhaustion {context} must be a string array"))?;
    if actual != expected {
        return Err(format!("partial exhaustion {context} drifted"));
    }
    Ok(())
}

fn require_exact_u64(object: &Map<String, Value>, name: &str, expected: u64) -> CaptureResult<()> {
    if field(object, name)?.as_u64() != Some(expected) {
        return Err(format!(
            "partial exhaustion {name} differs from retained custody"
        ));
    }
    Ok(())
}

fn require_exact_sha256(
    object: &Map<String, Value>,
    name: &str,
    expected: &[u8; 32],
) -> CaptureResult<()> {
    let actual = field(object, name)?
        .as_str()
        .ok_or_else(|| format!("partial exhaustion {name} must be a string"))?;
    require_sha256(actual)?;
    if actual != hex_bytes(expected) {
        return Err(format!(
            "partial exhaustion {name} differs from retained custody"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expected() -> ExpectedBindingsV1 {
        ExpectedBindingsV1 {
            device_id: [1; 32],
            draft_arena: [2; 32],
            first_generation: 7,
            gpu_unique_id: 23,
            kernel_catalog: [3; 32],
            kernel_manifest: [4; 32],
            program_catalog: [5; 32],
            request_generation: 11,
            request_slot: 0,
            reused_generation: 8,
            runner_declaration: [6; 32],
            target_arena: [7; 32],
        }
    }

    fn fixture() -> Value {
        let expected = expected();
        json!({
            "authority": "ferric-physical-partial-capture-only",
            "case": CASE,
            "format": CAPTURE_FORMAT,
            "identities": {
                "device_id_sha256": hex_bytes(&expected.device_id),
                "draft_kv_arena_sha256": hex_bytes(&expected.draft_arena),
                "gpu_unique_id": expected.gpu_unique_id,
                "kernel_catalog_sha256": hex_bytes(&expected.kernel_catalog),
                "kernel_manifest_sha256": hex_bytes(&expected.kernel_manifest),
                "program_catalog_sha256": hex_bytes(&expected.program_catalog),
                "runner_declaration_sha256": hex_bytes(&expected.runner_declaration),
                "target_kv_arena_sha256": hex_bytes(&expected.target_arena),
            },
            "milestone": "M1", "nonclaim": NONCLAIM, "obligation_id": "m1.r30",
            "result": {
                "checked_leases": 512, "engine_request_reclaimed": true,
                "events": EVENTS, "first_generation": 7,
                "kernel_dispatch_performed": false,
                "occupied_page_index": 0,
                "occupied_page_rejection": "page-already-leased",
                "occupied_rejection_transactional": true,
                "out_of_range_page_index": 512,
                "out_of_range_rejection": "page-out-of-range",
                "out_of_range_rejection_transactional": true,
                "page_capacity": 512,
                "returned_pages": 512, "reuse_page_index": 0,
                "reused_generation": 8, "reused_returned_pages": 1,
                "role": "target-8b",
            },
            "status": STATUS, "target": TARGET,
            "trace": {"request_generation": 11, "request_slot": 0},
        })
    }

    #[test]
    fn exact_partial_exhaustion_capture_and_protocol_are_accepted() {
        validate_manifest(&canonical_bytes(&fixture()).unwrap(), &expected()).unwrap();
        require_protocol().unwrap();
        let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR").unwrap();
        let checked_in = std::fs::read(
            std::path::PathBuf::from(manifest_dir)
                .join("src/bin/ferric-m1-r30-exhaustion-partial-protocol.json"),
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
        assert_eq!(admitted.runner_declaration_sha256, "06".repeat(32));

        let mut hostile = fixture();
        hostile["result"]["returned_pages"] = json!(511);
        assert!(admit_persisted_bundle(
            &canonical_bytes(&hostile).unwrap(),
            &protocol_bytes().unwrap()
        )
        .is_err());

        let mut hostile_generation = fixture();
        hostile_generation["result"]["reused_generation"] = json!(9);
        assert!(admit_persisted_bundle(
            &canonical_bytes(&hostile_generation).unwrap(),
            &protocol_bytes().unwrap()
        )
        .is_err());
    }

    #[test]
    fn hostile_capacity_return_generation_and_promotion_reject() {
        let mutations: &[fn(&mut Value)] = &[
            |value| value["result"]["checked_leases"] = json!(511),
            |value| value["result"]["returned_pages"] = json!(511),
            |value| value["result"]["reused_generation"] = json!(7),
            |value| value["result"]["occupied_page_index"] = json!(1),
            |value| value["result"]["occupied_page_rejection"] = json!("other"),
            |value| value["result"]["occupied_rejection_transactional"] = json!(false),
            |value| value["result"]["out_of_range_rejection"] = json!("other"),
            |value| value["result"]["out_of_range_rejection_transactional"] = json!(false),
            |value| value["result"]["kernel_dispatch_performed"] = json!(true),
            |value| value["result"]["events"][3] = json!("substituted"),
            |value| value["status"] = json!("evidence"),
            |value| value["identities"]["target_kv_arena_sha256"] = json!("09".repeat(32)),
        ];
        for mutate in mutations {
            let mut value = fixture();
            mutate(&mut value);
            assert!(validate_manifest(&canonical_bytes(&value).unwrap(), &expected()).is_err());
        }
    }

    #[test]
    fn hostile_protocol_substitution_rejects() {
        let protocol = parse_canonical(&protocol_bytes().unwrap(), "fixture").unwrap();
        let mutations: &[fn(&mut Value)] = &[
            |value| value["status"] = json!("evidence"),
            |value| value["case"] = json!("canary"),
            |value| value["bundle_files"] = json!(["capture.json"]),
            |value| value["lifecycle"][2] = json!("substituted"),
            |value| value["required_complete_case_roster"][2] = json!("other"),
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
            "ferric-m1-r30-exhaustion-publish-{}-{}",
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
