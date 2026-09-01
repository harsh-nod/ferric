//! Canonical partial m1.r32 diagnostic capture.
//!
//! Construction accepts only typed, completed Ferric choice/readback and KV
//! settlement custody. The resulting two-file bundle remains explicitly
//! partial and non-evidentiary.

use super::{
    canonical_bytes, exact_object, expect_string, field, hex_bytes, parse_canonical,
    require_sha256, sha256_array, CaptureResult, StagingOutput, TARGET,
};
use ferric_engine::{
    CheckedCompletionSemantics, M1AuthenticatedReleasedCompletedStepV1,
    M1ObservedSpeculativeDiagnosticChoicesV1, M1StepDispatchIntent,
};
use ferric_spec::Identity;
use serde_json::{json, Value};
use std::path::Path;

const CAPTURE_FORMAT: &str = "FERRIC-M1-R32-AUTHENTICATED-PARTIAL-CAPTURE-V2";
const PROTOCOL_FORMAT: &str = "FERRIC-M1-R32-PARTIAL-PROTOCOL-V1";
const STATUS: &str = "partial-non-evidence";
const NONCLAIM: &str = "Authenticated Worker V3 program, Ferric queue, completion, diagnostic-choice, and KV custody for one first-publication physical S1/K4 speculative round only. The corresponding target token is the first target-verification choice from that same round, not a separately executed target-only queue. This partial capture does not establish holdout eligibility, sampling refinement, performance, external validation, hardware correctness, qualification, or close m1.r32.";
const PROTOCOL_NONCLAIM: &str = "Diagnostic S1/K4 physical choice capture only. This protocol does not establish an independent target-only dispatch, holdout eligibility, sampling refinement, performance, external validation, hardware correctness, qualification, or close m1.r32.";

const EVENTS: &[&str] = &[
    "queue-completed",
    "compact-readback-observed",
    "draft-choices-readback-observed",
    "target-choices-readback-observed",
    "maximal-prefix-checked",
    "corresponding-target-token-checked",
    "exact-completion-settled",
    "kv-pages-accounted",
];

pub(super) const COMMAND: &str = "capture-r32-speculative-k4";

pub(super) struct SettledCaptureInputsV1<'a> {
    pub(super) choices: &'a M1ObservedSpeculativeDiagnosticChoicesV1,
    pub(super) released: &'a M1AuthenticatedReleasedCompletedStepV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ExpectedBindingsV1 {
    compact_sha256: [u8; 32],
    completion_epoch: u64,
    device_id: [u8; 32],
    dispatch_generation: u64,
    draft_sha256: [u8; 32],
    gpu_unique_id: u64,
    kernel_catalog: [u8; 32],
    plan_id: [u8; 32],
    program_catalog: [u8; 32],
    request_generation: u32,
    request_slot: u32,
    runner_declaration: [u8; 32],
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

pub(super) fn manifest(inputs: SettledCaptureInputsV1<'_>) -> CaptureResult<CaptureArtifactV1> {
    let checked = inputs.released.checked();
    let [record] = checked.records() else {
        return Err("partial r32 capture requires exactly one checked S1 record".to_owned());
    };
    let CheckedCompletionSemantics::Speculative {
        accepted_draft_tokens,
        correction_or_bonus,
    } = record.semantics()
    else {
        return Err("partial r32 capture requires checked speculative semantics".to_owned());
    };
    if inputs.choices.dispatch_generation() != checked.dispatch_generation() {
        return Err("choice readback and compact dispatch generations differ".to_owned());
    }
    let raw = record.record();
    let emitted_count = usize::from(raw.emitted_token_count);
    let emitted = raw
        .emitted_tokens
        .get(..emitted_count)
        .ok_or_else(|| "checked emitted-token extent drifted".to_owned())?;
    if emitted.first().copied() != Some(inputs.choices.target_choices()[0]) {
        return Err(
            "speculative first token differs from the corresponding target choice".to_owned(),
        );
    }
    let expected_count = u32::from(accepted_draft_tokens) + 1;
    if inputs.released.completed_members() != 1
        || inputs.released.logical_accepted_counts() != [expected_count]
        || inputs.released.externally_published_counts() != [expected_count]
        || inputs.released.release_counts().len() != 1
    {
        return Err("partial r32 completion or KV settlement roster drifted".to_owned());
    }
    let release = inputs.released.release_counts()[0];
    let custody = inputs.released.queue().custody();
    let device = custody.device();
    let dispatch_plan = custody.workspace_composition().dispatch_plan();
    if custody.selection() != checked.selection()
        || dispatch_plan.intent() != M1StepDispatchIntent::SpeculativeRound(checked.selection())
    {
        return Err("partial r32 retained selection or dispatch intent drifted".to_owned());
    }
    let program_catalog = custody.catalog_id();
    let runner_declaration = dispatch_plan.runner_declaration_id();
    let kernel_catalog = dispatch_plan.kernel_catalog_id();
    let expected = ExpectedBindingsV1 {
        compact_sha256: *checked.raw_sha256(),
        completion_epoch: checked.epoch().value(),
        device_id: *device.device_id().as_bytes(),
        dispatch_generation: checked.dispatch_generation(),
        draft_sha256: *inputs.choices.draft_sha256(),
        gpu_unique_id: device.gpu_unique_id(),
        kernel_catalog: *kernel_catalog.as_bytes(),
        plan_id: *raw.plan_id.as_bytes(),
        program_catalog: *program_catalog.as_bytes(),
        request_generation: raw.request.generation(),
        request_slot: raw.request.slot(),
        runner_declaration: *runner_declaration.as_bytes(),
        target_sha256: *inputs.choices.target_sha256(),
    };
    let value = json!({
        "authority": "ferric-authenticated-worker-v3-partial-capture-only",
        "case": "speculative-s1-k4-c8192",
        "choices": {
            "draft": inputs.choices.draft_choices(),
            "draft_bytes": inputs.choices.draft_bytes().len(),
            "draft_sha256": hex_bytes(inputs.choices.draft_sha256()),
            "encoding": "u32-le",
            "target": inputs.choices.target_choices(),
            "target_bytes": inputs.choices.target_bytes().len(),
            "target_sha256": hex_bytes(inputs.choices.target_sha256()),
        },
        "format": CAPTURE_FORMAT,
        "identities": {
            "device_id_sha256": hex_identity(device.device_id()),
            "gpu_unique_id": device.gpu_unique_id(),
            "kernel_catalog_sha256": hex_identity(kernel_catalog),
            "program_catalog_sha256": hex_identity(program_catalog),
            "runner_declaration_sha256": hex_identity(runner_declaration),
        },
        "milestone": "M1",
        "nonclaim": NONCLAIM,
        "obligation_id": "m1.r32",
        "result": {
            "accepted_draft_tokens": accepted_draft_tokens,
            "completed_members": inputs.released.completed_members(),
            "correction_or_bonus": correction_or_bonus,
            "corresponding_target_token": inputs.choices.target_choices()[0],
            "emitted_tokens": emitted,
            "events": EVENTS,
            "externally_published_count": expected_count,
            "kv_pages_released": {
                "draft": release.draft(),
                "target": release.target(),
                "total": release.total(),
            },
            "kv_settled_and_release_accounted": true,
            "logical_accepted_count": expected_count,
            "maximal_prefix_verified": true,
            "positive_completion": true,
            "target_token_equal": true,
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

pub(super) fn publish(output: &Path, capture: &CaptureArtifactV1) -> CaptureResult<()> {
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

fn validate_manifest(bytes: &[u8], expected: &ExpectedBindingsV1) -> CaptureResult<()> {
    let value = parse_canonical(bytes, "partial r32 capture")?;
    let root = exact_object(
        &value,
        &[
            "authority",
            "case",
            "choices",
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
        "partial r32 capture",
    )?;
    expect_string(
        root,
        "authority",
        "ferric-authenticated-worker-v3-partial-capture-only",
    )?;
    expect_string(root, "case", "speculative-s1-k4-c8192")?;
    expect_string(root, "format", CAPTURE_FORMAT)?;
    expect_string(root, "milestone", "M1")?;
    expect_string(root, "nonclaim", NONCLAIM)?;
    expect_string(root, "obligation_id", "m1.r32")?;
    expect_string(root, "status", STATUS)?;
    expect_string(root, "target", TARGET)?;

    let identities = exact_object(
        field(root, "identities")?,
        &[
            "device_id_sha256",
            "gpu_unique_id",
            "kernel_catalog_sha256",
            "program_catalog_sha256",
            "runner_declaration_sha256",
        ],
        "partial r32 identities",
    )?;
    require_exact_sha256(identities, "device_id_sha256", &expected.device_id)?;
    require_exact_sha256(
        identities,
        "kernel_catalog_sha256",
        &expected.kernel_catalog,
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
    require_exact_u64(identities, "gpu_unique_id", expected.gpu_unique_id)?;

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
        "partial r32 trace",
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
        || expected.gpu_unique_id == 0
    {
        return Err("partial r32 expected trace identities must be nonzero".to_owned());
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
        "partial r32 choices",
    )?;
    expect_string(choices, "encoding", "u32-le")?;
    require_exact_u64(choices, "draft_bytes", 16)?;
    require_exact_u64(choices, "target_bytes", 20)?;
    require_exact_sha256(choices, "draft_sha256", &expected.draft_sha256)?;
    require_exact_sha256(choices, "target_sha256", &expected.target_sha256)?;
    let draft = token_array(field(choices, "draft")?, 4, "draft choices")?;
    let target = token_array(field(choices, "target")?, 5, "target choices")?;
    if sha256_array(&token_bytes(&draft)) != expected.draft_sha256
        || sha256_array(&token_bytes(&target)) != expected.target_sha256
    {
        return Err("partial r32 choice arrays differ from copied-byte digests".to_owned());
    }

    let result = exact_object(
        field(root, "result")?,
        &[
            "accepted_draft_tokens",
            "completed_members",
            "correction_or_bonus",
            "corresponding_target_token",
            "emitted_tokens",
            "events",
            "externally_published_count",
            "kv_pages_released",
            "kv_settled_and_release_accounted",
            "logical_accepted_count",
            "maximal_prefix_verified",
            "positive_completion",
            "target_token_equal",
        ],
        "partial r32 result",
    )?;
    let accepted = usize::try_from(u64_value(result, "accepted_draft_tokens")?)
        .map_err(|_| "accepted draft count does not fit usize".to_owned())?;
    if accepted > draft.len() {
        return Err("accepted draft count exceeds K4".to_owned());
    }
    let mismatch = draft
        .iter()
        .zip(&target)
        .position(|(draft, target)| draft != target)
        .unwrap_or(draft.len());
    if accepted != mismatch {
        return Err("partial r32 accepted prefix is not maximal".to_owned());
    }
    let expected_emitted = draft[..accepted]
        .iter()
        .copied()
        .chain([target[accepted]])
        .collect::<Vec<_>>();
    let emitted = token_array(
        field(result, "emitted_tokens")?,
        accepted + 1,
        "emitted tokens",
    )?;
    if emitted != expected_emitted
        || emitted[0] != target[0]
        || u64_value(result, "correction_or_bonus")? != u64::from(target[accepted])
        || u64_value(result, "corresponding_target_token")? != u64::from(target[0])
    {
        return Err("partial r32 token semantics drifted".to_owned());
    }
    let count = u64::try_from(accepted + 1).unwrap_or(u64::MAX);
    for name in ["externally_published_count", "logical_accepted_count"] {
        require_exact_u64(result, name, count)?;
    }
    require_exact_u64(result, "completed_members", 1)?;
    for name in [
        "kv_settled_and_release_accounted",
        "maximal_prefix_verified",
        "positive_completion",
        "target_token_equal",
    ] {
        if field(result, name)?.as_bool() != Some(true) {
            return Err(format!("partial r32 result {name} must be true"));
        }
    }
    let events = field(result, "events")?
        .as_array()
        .and_then(|events| events.iter().map(Value::as_str).collect::<Option<Vec<_>>>())
        .ok_or_else(|| "partial r32 events must be a string array".to_owned())?;
    if events != EVENTS {
        return Err("partial r32 event order drifted".to_owned());
    }
    let released = exact_object(
        field(result, "kv_pages_released")?,
        &["draft", "target", "total"],
        "partial r32 released pages",
    )?;
    let draft_pages = u64_value(released, "draft")?;
    let target_pages = u64_value(released, "target")?;
    if draft_pages.checked_add(target_pages) != Some(u64_value(released, "total")?) {
        return Err("partial r32 released-page total drifted".to_owned());
    }
    Ok(())
}

pub(super) fn require_protocol() -> CaptureResult<()> {
    let bytes = protocol_bytes()?;
    let value = parse_canonical(&bytes, "partial r32 protocol")?;
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
            "status",
            "target",
        ],
        "partial r32 protocol",
    )?;
    expect_string(root, "authority", "ferric-m1-r32-partial-protocol-only")?;
    expect_string(root, "case", "speculative-s1-k4-c8192")?;
    expect_string(root, "format", PROTOCOL_FORMAT)?;
    expect_string(root, "milestone", "M1")?;
    expect_string(root, "nonclaim", PROTOCOL_NONCLAIM)?;
    expect_string(root, "obligation_id", "m1.r32")?;
    expect_string(root, "status", STATUS)?;
    expect_string(root, "target", TARGET)?;
    let files = field(root, "bundle_files")?
        .as_array()
        .and_then(|items| items.iter().map(Value::as_str).collect::<Option<Vec<_>>>())
        .ok_or_else(|| "partial r32 bundle_files must be a string array".to_owned())?;
    if files != ["capture.json", "protocol.json"] {
        return Err("partial r32 bundle roster drifted".to_owned());
    }
    let lifecycle = field(root, "lifecycle")?
        .as_array()
        .and_then(|items| items.iter().map(Value::as_str).collect::<Option<Vec<_>>>())
        .ok_or_else(|| "partial r32 lifecycle must be a string array".to_owned())?;
    if lifecycle != EVENTS {
        return Err("partial r32 protocol lifecycle drifted".to_owned());
    }
    Ok(())
}

fn protocol_bytes() -> CaptureResult<Vec<u8>> {
    canonical_bytes(&json!({
        "authority": "ferric-m1-r32-partial-protocol-only",
        "bundle_files": ["capture.json", "protocol.json"],
        "case": "speculative-s1-k4-c8192",
        "format": PROTOCOL_FORMAT,
        "lifecycle": EVENTS,
        "milestone": "M1",
        "nonclaim": PROTOCOL_NONCLAIM,
        "obligation_id": "m1.r32",
        "status": STATUS,
        "target": TARGET,
    }))
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

fn string_value<'a>(
    object: &'a serde_json::Map<String, Value>,
    name: &str,
) -> CaptureResult<&'a str> {
    field(object, name)?
        .as_str()
        .ok_or_else(|| format!("partial r32 {name} must be a string"))
}

fn u64_value(object: &serde_json::Map<String, Value>, name: &str) -> CaptureResult<u64> {
    field(object, name)?
        .as_u64()
        .ok_or_else(|| format!("partial r32 {name} must be u64"))
}

fn require_exact_u64(
    object: &serde_json::Map<String, Value>,
    name: &str,
    expected: u64,
) -> CaptureResult<()> {
    if u64_value(object, name)? != expected {
        return Err(format!("partial r32 {name} differs from retained custody"));
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
        return Err(format!("partial r32 {name} differs from retained custody"));
    }
    Ok(())
}

fn hex_identity(identity: Identity) -> String {
    hex_bytes(identity.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: u8) -> String {
        hex_bytes(&[byte; 32])
    }

    fn expected() -> ExpectedBindingsV1 {
        ExpectedBindingsV1 {
            compact_sha256: [9; 32],
            completion_epoch: 31,
            device_id: [2; 32],
            dispatch_generation: 37,
            draft_sha256: sha256_array(&token_bytes(&[11, 12, 13, 14])),
            gpu_unique_id: 23,
            kernel_catalog: [6; 32],
            plan_id: [3; 32],
            program_catalog: [4; 32],
            request_generation: 7,
            request_slot: 0,
            runner_declaration: [8; 32],
            target_sha256: sha256_array(&token_bytes(&[11, 12, 99, 14, 15])),
        }
    }

    fn fixture() -> Value {
        json!({
            "authority": "ferric-authenticated-worker-v3-partial-capture-only",
            "case": "speculative-s1-k4-c8192",
            "choices": {
                "draft": [11, 12, 13, 14], "draft_bytes": 16,
                "draft_sha256": hex_bytes(&sha256_array(&token_bytes(&[11, 12, 13, 14]))),
                "encoding": "u32-le",
                "target": [11, 12, 99, 14, 15], "target_bytes": 20,
                "target_sha256": hex_bytes(&sha256_array(&token_bytes(&[11, 12, 99, 14, 15]))),
            },
            "format": CAPTURE_FORMAT,
            "identities": {
                "device_id_sha256": digest(2), "gpu_unique_id": 23,
                "kernel_catalog_sha256": digest(6),
                "program_catalog_sha256": digest(4),
                "runner_declaration_sha256": digest(8),
            },
            "milestone": "M1", "nonclaim": NONCLAIM, "obligation_id": "m1.r32",
            "result": {
                "accepted_draft_tokens": 2, "completed_members": 1,
                "correction_or_bonus": 99, "corresponding_target_token": 11,
                "emitted_tokens": [11, 12, 99], "events": EVENTS,
                "externally_published_count": 3,
                "kv_pages_released": {"draft": 2, "target": 1, "total": 3},
                "kv_settled_and_release_accounted": true, "logical_accepted_count": 3,
                "maximal_prefix_verified": true, "positive_completion": true,
                "target_token_equal": true,
            },
            "status": STATUS, "target": TARGET,
            "trace": {
                "compact_sha256": digest(9), "completion_epoch": 31,
                "dispatch_generation": 37, "plan_id_sha256": digest(3),
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
                .join("src/bin/ferric-m1-r32-partial-protocol.json"),
        )
        .unwrap();
        assert_eq!(protocol_bytes().unwrap(), checked_in);
        let bytes = canonical_bytes(&fixture()).unwrap();
        validate_manifest(&bytes, &expected()).unwrap();
    }

    #[test]
    fn hostile_trace_token_epoch_plan_device_and_completion_substitution_reject() {
        let mutations: &[fn(&mut Value)] = &[
            |value| value["result"]["events"][0] = json!("substituted"),
            |value| value["result"]["emitted_tokens"][0] = json!(77),
            |value| value["trace"]["completion_epoch"] = json!(32),
            |value| value["trace"]["plan_id_sha256"] = json!(digest(10)),
            |value| value["identities"]["device_id_sha256"] = json!(digest(11)),
            |value| value["identities"]["gpu_unique_id"] = json!(24),
            |value| value["identities"]["kernel_catalog_sha256"] = json!(digest(16)),
            |value| value["result"]["positive_completion"] = json!(false),
            |value| value["trace"]["compact_sha256"] = json!(digest(12)),
            |value| value["trace"]["dispatch_generation"] = json!(38),
            |value| value["identities"]["program_catalog_sha256"] = json!(digest(13)),
            |value| value["identities"]["runner_declaration_sha256"] = json!(digest(15)),
        ];
        for mutate in mutations {
            let mut value = fixture();
            mutate(&mut value);
            let bytes = canonical_bytes(&value).unwrap();
            assert!(validate_manifest(&bytes, &expected()).is_err());
        }
    }

    #[test]
    fn unknown_identity_member_rejects() {
        let mut value = fixture();
        value["identities"]["unexpected_identity_sha256"] = json!(digest(14));
        assert!(validate_manifest(&canonical_bytes(&value).unwrap(), &expected()).is_err());
    }

    #[test]
    fn nonmaximal_prefix_and_target_token_substitution_reject() {
        let mut nonmaximal = fixture();
        nonmaximal["result"]["accepted_draft_tokens"] = json!(1);
        assert!(validate_manifest(&canonical_bytes(&nonmaximal).unwrap(), &expected()).is_err());

        let mut target = fixture();
        target["choices"]["target"][0] = json!(77);
        assert!(validate_manifest(&canonical_bytes(&target).unwrap(), &expected()).is_err());

        let mut coherent_draft = fixture();
        coherent_draft["choices"]["draft"][3] = json!(77);
        assert!(
            validate_manifest(&canonical_bytes(&coherent_draft).unwrap(), &expected()).is_err()
        );

        let mut unused_target = fixture();
        unused_target["choices"]["target"][4] = json!(77);
        assert!(validate_manifest(&canonical_bytes(&unused_target).unwrap(), &expected()).is_err());
    }

    #[test]
    fn publisher_is_exact_and_no_replace() {
        let output = std::env::temp_dir().join(format!(
            "ferric-m1-r32-publish-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let capture = canonical_bytes(&fixture()).unwrap();
        let artifact = CaptureArtifactV1 {
            bytes: capture.clone(),
            expected: expected(),
        };
        publish(&output, &artifact).unwrap();
        assert_eq!(std::fs::read(output.join("capture.json")).unwrap(), capture);
        assert_eq!(
            std::fs::read(output.join("protocol.json")).unwrap(),
            protocol_bytes().unwrap()
        );
        let replacement = CaptureArtifactV1 {
            bytes: capture,
            expected: expected(),
        };
        assert!(publish(&output, &replacement).is_err());
        std::fs::remove_dir_all(output).unwrap();
    }
}
