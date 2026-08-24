//! Fail-closed composition of the currently admitted m1.r30 capture roster.
//!
//! Four inputs are Ferric-owned physical partial captures. Fault-injection is
//! intentionally a structurally admitted external report until the physical
//! queue API exposes a production fault-injection authority.

use super::{
    canonical_bytes, exact_object, expect_string, field, parse_canonical, require_relative,
    require_safe_id, require_sha256, secure_parent, sha256_hex, CaptureResult,
    R30PhysicalCaptureBindingsV1, SecureDirectory, StagingOutput, TARGET,
};
use rustix::fs::Dir;
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::Path;

pub(super) const COMMAND: &str = "compose-r30-runner";

const RUNNER_FORMAT: &str = "FERRIC-M1-R30-COMPOSED-RUNNER-V4";
const PROTOCOL_FORMAT: &str = "FERRIC-M1-R30-COMPOSED-RUNNER-PROTOCOL-V5";
const FAULT_OBSERVATION_FORMAT: &str = "FERRIC-M1-R30-FAULT-OBSERVATION-V1";
const STATUS: &str = "partial-non-evidence";
const AUTHORITY: &str = "ferric-r30-composed-runner-only";
const FAULT_AUTHORITY: &str = "externally-reported-r30-fault-observation-only";
const FAULT_NONCLAIM: &str = "Structurally admitted external fault-injection report only. The reported observations are not a Ferric physical queue fault authority, make no hardware claim, and do not establish fault occurrence, queue quarantine, resource reclamation, hardware correctness, qualification, m1.r30, or M1 closure.";
const NONCLAIM: &str = "Canonical composition of four Ferric-owned physical partial captures and one structurally admitted external fault-injection report. The composer authenticates source protocols, exact source bytes, and common physical runtime identities, but does not convert the external report into a physical observation, make a hardware claim, establish general canary or device-memory safety, supply independent validation, qualify performance, or close m1.r30/M1.";
const PROTOCOL_NONCLAIM: &str = "Composition and identity-join protocol only. Fault injection remains externally reported and unvalidated because the public physical queue API supplies no production fault-injection authority. This protocol makes no hardware, correctness, evidence, qualification, m1.r30, or M1 closure claim.";
const MAX_FAULT_OBSERVATIONS: usize = 64;

#[derive(Clone, Debug, Eq, PartialEq)]
struct PhysicalSourceV1 {
    bindings: R30PhysicalCaptureBindingsV1,
    capture_sha256: String,
    kind: &'static str,
    protocol_sha256: String,
    source_format: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FaultObservationV1 {
    observation_count: usize,
    source_environment_sha256: String,
    source_executable_sha256: String,
    source_protocol_sha256: String,
    source_sha256: String,
    source_transcript_sha256: String,
}

pub(super) fn run(arguments: &[OsString]) -> CaptureResult<()> {
    let [canary, cancellation, exhaustion, rollback, fault_observation, output] = arguments else {
        return Err("usage: ferric-m1-qualification-capture compose-r30-runner CANARY-BUNDLE CANCELLATION-BUNDLE EXHAUSTION-BUNDLE ROLLBACK-BUNDLE FAULT-OBSERVATION OUTPUT-BUNDLE".to_owned());
    };
    let canary = read_physical_bundle(
        Path::new(canary),
        "canary",
        "FERRIC-M1-R30-CANARY-PARTIAL-CAPTURE-V4",
    )?;
    let cancellation = read_physical_bundle(
        Path::new(cancellation),
        "cancellation",
        "FERRIC-M1-R30-PARTIAL-CAPTURE-V5",
    )?;
    let exhaustion = read_physical_bundle(
        Path::new(exhaustion),
        "exhaustion",
        "FERRIC-M1-R30-EXHAUSTION-PARTIAL-CAPTURE-V1",
    )?;
    let rollback = read_physical_bundle(
        Path::new(rollback),
        "rollback",
        "FERRIC-M1-R30-ROLLBACK-PARTIAL-CAPTURE-V1",
    )?;
    let fault = read_fault_observation(Path::new(fault_observation))?;
    publish(
        Path::new(output),
        &[canary, cancellation, exhaustion, rollback],
        &fault,
    )
}

fn read_physical_bundle(
    path: &Path,
    kind: &'static str,
    source_format: &'static str,
) -> CaptureResult<PhysicalSourceV1> {
    let root = SecureDirectory::open(path, &format!("r30 {kind} capture bundle"))?;
    require_bundle_roster(&root, &format!("r30 {kind} capture bundle"))?;
    let capture = root.read_bounded(
        Path::new("capture.json"),
        super::MAX_DOCUMENT_BYTES as u64,
        &format!("r30 {kind} capture"),
    )?;
    let protocol = root.read_bounded(
        Path::new("protocol.json"),
        super::MAX_DOCUMENT_BYTES as u64,
        &format!("r30 {kind} protocol"),
    )?;
    let bindings = match kind {
        "canary" => {
            super::m1_r30_canary_partial_capture::admit_persisted_bundle(&capture, &protocol)
        }
        "cancellation" => {
            super::m1_r30_partial_capture::admit_persisted_bundle(&capture, &protocol)
        }
        "exhaustion" => {
            super::m1_r30_exhaustion_partial_capture::admit_persisted_bundle(&capture, &protocol)
        }
        "rollback" => {
            super::m1_r30_rollback_partial_capture::admit_persisted_bundle(&capture, &protocol)
        }
        _ => Err(format!("unsupported r30 physical capture kind: {kind}")),
    }?;
    require_bundle_roster(&root, &format!("r30 {kind} capture bundle"))?;
    Ok(PhysicalSourceV1 {
        bindings,
        capture_sha256: sha256_hex(&capture),
        kind,
        protocol_sha256: sha256_hex(&protocol),
        source_format,
    })
}

fn require_bundle_roster(root: &SecureDirectory, description: &str) -> CaptureResult<()> {
    let mut directory = Dir::read_from(&root.descriptor)
        .map_err(|error| format!("cannot enumerate {description}: {error}"))?;
    let mut actual = BTreeSet::new();
    while let Some(entry) = directory.read() {
        let entry = entry.map_err(|error| format!("cannot enumerate {description}: {error}"))?;
        let name = entry.file_name().to_bytes();
        if matches!(name, b"." | b"..") {
            continue;
        }
        if !name.is_ascii() {
            return Err(format!("{description} filename must be ASCII"));
        }
        let name = std::str::from_utf8(name)
            .map_err(|_| format!("{description} filename must be UTF-8"))?;
        require_relative(Path::new(name), &format!("{description} member"))?;
        if !actual.insert(name.to_owned()) {
            return Err(format!("{description} repeats a filename"));
        }
    }
    let expected = BTreeSet::from(["capture.json".to_owned(), "protocol.json".to_owned()]);
    if actual != expected {
        return Err(format!("{description} exact file roster drifted"));
    }
    Ok(())
}

fn read_fault_observation(path: &Path) -> CaptureResult<FaultObservationV1> {
    let (parent, relative) = secure_parent(path, "r30 fault observation parent")?;
    let (value, bytes) = parent.read_canonical(&relative, "r30 fault observation")?;
    let root = exact_object(
        &value,
        &[
            "authority",
            "format",
            "hardware_claim",
            "milestone",
            "nonclaim",
            "obligation_id",
            "observations",
            "source",
            "status",
            "target",
        ],
        "r30 fault observation",
    )?;
    expect_string(root, "authority", FAULT_AUTHORITY)?;
    expect_string(root, "format", FAULT_OBSERVATION_FORMAT)?;
    expect_string(root, "hardware_claim", "none")?;
    expect_string(root, "milestone", "M1")?;
    expect_string(root, "nonclaim", FAULT_NONCLAIM)?;
    expect_string(root, "obligation_id", "m1.r30")?;
    expect_string(root, "status", "reported-unvalidated")?;
    expect_string(root, "target", TARGET)?;
    let source = exact_object(
        field(root, "source")?,
        &[
            "environment_sha256",
            "executable_sha256",
            "protocol_sha256",
            "transcript_sha256",
        ],
        "r30 fault observation source",
    )?;
    for name in [
        "environment_sha256",
        "executable_sha256",
        "protocol_sha256",
        "transcript_sha256",
    ] {
        require_sha256(string(source, name, "r30 fault observation source")?)?;
    }
    let observations = field(root, "observations")?
        .as_array()
        .filter(|values| !values.is_empty() && values.len() <= MAX_FAULT_OBSERVATIONS)
        .ok_or_else(|| "r30 fault observation roster is outside the admitted bound".to_owned())?;
    let mut ids = BTreeSet::new();
    for observation in observations {
        validate_fault_member(observation, &mut ids)?;
    }
    Ok(FaultObservationV1 {
        observation_count: observations.len(),
        source_environment_sha256: string(source, "environment_sha256", "fault source")?.to_owned(),
        source_executable_sha256: string(source, "executable_sha256", "fault source")?.to_owned(),
        source_protocol_sha256: string(source, "protocol_sha256", "fault source")?.to_owned(),
        source_sha256: sha256_hex(&bytes),
        source_transcript_sha256: string(source, "transcript_sha256", "fault source")?.to_owned(),
    })
}

fn validate_fault_member(value: &Value, ids: &mut BTreeSet<String>) -> CaptureResult<()> {
    let member = exact_object(
        value,
        &[
            "failure_observed",
            "id",
            "injection_point",
            "live_resources_after",
            "queue_quarantined",
            "retry_denied",
        ],
        "r30 fault observation member",
    )?;
    let id = string(member, "id", "r30 fault observation member")?;
    require_safe_id(id, "r30 fault observation ID")?;
    if !ids.insert(id.to_owned()) {
        return Err("r30 fault observation IDs must be unique".to_owned());
    }
    require_safe_id(
        string(member, "injection_point", "r30 fault observation member")?,
        "r30 fault injection point",
    )?;
    for name in ["failure_observed", "queue_quarantined", "retry_denied"] {
        if field(member, name)?.as_bool().is_none() {
            return Err(format!("r30 fault observation {name} must be boolean"));
        }
    }
    if field(member, "live_resources_after")?.as_u64().is_none() {
        return Err("r30 fault live-resource count must be a nonnegative integer".to_owned());
    }
    Ok(())
}

fn publish(
    output: &Path,
    physical: &[PhysicalSourceV1],
    fault: &FaultObservationV1,
) -> CaptureResult<()> {
    let executable_sha256 = super::current_executable_sha256()?;
    let runner = compose(physical, fault, &executable_sha256)?;
    let protocol = protocol_bytes()?;
    validate_runner(&runner, physical, fault, &executable_sha256, &protocol)?;
    let mut staging = StagingOutput::create(output)?;
    staging.write("runner.json", &runner)?;
    staging.write("protocol.json", &protocol)?;
    staging.publish_exact(&[
        ("runner.json", runner.as_slice()),
        ("protocol.json", protocol.as_slice()),
    ])
}

fn compose(
    physical: &[PhysicalSourceV1],
    fault: &FaultObservationV1,
    executable_sha256: &str,
) -> CaptureResult<Vec<u8>> {
    require_sha256(executable_sha256)?;
    let [canary, cancellation, exhaustion, rollback] = physical else {
        return Err("r30 composer requires exactly four physical source captures".to_owned());
    };
    let expected_kinds = ["canary", "cancellation", "exhaustion", "rollback"];
    if physical.iter().map(|source| source.kind).ne(expected_kinds) {
        return Err("r30 physical source order or roster drifted".to_owned());
    }
    let common = &canary.bindings;
    if physical.iter().any(|source| &source.bindings != common) {
        return Err("r30 physical captures do not bind one device and runtime identity".to_owned());
    }
    let source = |item: &PhysicalSourceV1| {
        json!({
            "authority_class": "ferric-physical-partial-capture",
            "capture_sha256": item.capture_sha256,
            "hardware_claim": "none",
            "kind": item.kind,
            "protocol_sha256": item.protocol_sha256,
            "source_format": item.source_format,
            "status": STATUS,
        })
    };
    canonical_bytes(&json!({
        "authority": AUTHORITY,
        "bindings": {
            "device_identity_sha256": common.device_identity_sha256,
            "gpu_unique_id": common.gpu_unique_id,
            "kernel_artifact_manifest_sha256": common.kernel_artifact_manifest_sha256,
            "program_catalog_sha256": common.program_catalog_sha256,
            "runner_declaration_sha256": common.runner_declaration_sha256,
        },
        "case_roster": [
            source(canary),
            source(cancellation),
            source(exhaustion),
            json!({
                "authority_class": "external-report",
                "hardware_claim": "none",
                "kind": "fault-injection",
                "observation_count": fault.observation_count,
                "source_environment_sha256": fault.source_environment_sha256,
                "source_executable_sha256": fault.source_executable_sha256,
                "source_format": FAULT_OBSERVATION_FORMAT,
                "source_observation_sha256": fault.source_sha256,
                "source_protocol_sha256": fault.source_protocol_sha256,
                "source_transcript_sha256": fault.source_transcript_sha256,
                "status": "reported-unvalidated",
            }),
            source(rollback),
        ],
        "composer_executable_sha256": executable_sha256,
        "format": RUNNER_FORMAT,
        "hardware_claim": "none",
        "milestone": "M1",
        "nonclaim": NONCLAIM,
        "obligation_id": "m1.r30",
        "protocol_sha256": sha256_hex(&protocol_bytes()?),
        "qualification": {
            "evidence_case_count": 0,
            "fault_injection_physical_authority": false,
            "physical_partial_capture_count": 4,
            "required_case_count": 5,
            "roster_complete": true,
        },
        "status": STATUS,
        "target": TARGET,
    }))
}

fn validate_runner(
    bytes: &[u8],
    physical: &[PhysicalSourceV1],
    fault: &FaultObservationV1,
    executable_sha256: &str,
    protocol: &[u8],
) -> CaptureResult<()> {
    if bytes != compose(physical, fault, executable_sha256)? {
        return Err("r30 composed runner bytes do not recompute exactly".to_owned());
    }
    let value = parse_canonical(bytes, "r30 composed runner")?;
    let root = exact_object(
        &value,
        &[
            "authority",
            "bindings",
            "case_roster",
            "composer_executable_sha256",
            "format",
            "hardware_claim",
            "milestone",
            "nonclaim",
            "obligation_id",
            "protocol_sha256",
            "qualification",
            "status",
            "target",
        ],
        "r30 composed runner",
    )?;
    expect_string(root, "authority", AUTHORITY)?;
    expect_string(root, "composer_executable_sha256", executable_sha256)?;
    expect_string(root, "format", RUNNER_FORMAT)?;
    expect_string(root, "hardware_claim", "none")?;
    expect_string(root, "milestone", "M1")?;
    expect_string(root, "nonclaim", NONCLAIM)?;
    expect_string(root, "obligation_id", "m1.r30")?;
    expect_string(root, "protocol_sha256", &sha256_hex(protocol))?;
    expect_string(root, "status", STATUS)?;
    expect_string(root, "target", TARGET)
}

fn protocol_bytes() -> CaptureResult<Vec<u8>> {
    canonical_bytes(&json!({
        "authority": "ferric-m1-r30-composed-runner-protocol-only",
        "bundle_files": ["runner.json", "protocol.json"],
        "case_authorities": {
            "canary": "ferric-physical-partial-capture",
            "cancellation": "ferric-physical-partial-capture",
            "exhaustion": "ferric-physical-partial-capture",
            "fault-injection": "external-report",
            "rollback": "ferric-physical-partial-capture",
        },
        "format": PROTOCOL_FORMAT,
        "hardware_claim": "none",
        "milestone": "M1",
        "nonclaim": PROTOCOL_NONCLAIM,
        "obligation_id": "m1.r30",
        "required_case_roster": ["canary", "cancellation", "exhaustion", "fault-injection", "rollback"],
        "source_capture_formats": [
            "FERRIC-M1-R30-CANARY-PARTIAL-CAPTURE-V4",
            "FERRIC-M1-R30-PARTIAL-CAPTURE-V5",
            "FERRIC-M1-R30-EXHAUSTION-PARTIAL-CAPTURE-V1",
            FAULT_OBSERVATION_FORMAT,
            "FERRIC-M1-R30-ROLLBACK-PARTIAL-CAPTURE-V1",
        ],
        "status": STATUS,
        "target": TARGET,
    }))
}

fn string<'a>(object: &'a Map<String, Value>, name: &str, context: &str) -> CaptureResult<&'a str> {
    field(object, name)?
        .as_str()
        .ok_or_else(|| format!("{context} {name} must be a string"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NONCE: AtomicU64 = AtomicU64::new(0);

    struct Temporary(PathBuf);

    impl Temporary {
        fn new() -> Self {
            let nonce = NONCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "ferric-m1-r30-composition-test.{}.{nonce}",
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

    fn bindings() -> R30PhysicalCaptureBindingsV1 {
        R30PhysicalCaptureBindingsV1 {
            device_identity_sha256: "11".repeat(32),
            gpu_unique_id: 17,
            kernel_artifact_manifest_sha256: "22".repeat(32),
            program_catalog_sha256: "33".repeat(32),
            runner_declaration_sha256: "44".repeat(32),
        }
    }

    fn physical() -> Vec<PhysicalSourceV1> {
        [
            ("canary", "FERRIC-M1-R30-CANARY-PARTIAL-CAPTURE-V4"),
            ("cancellation", "FERRIC-M1-R30-PARTIAL-CAPTURE-V5"),
            ("exhaustion", "FERRIC-M1-R30-EXHAUSTION-PARTIAL-CAPTURE-V1"),
            ("rollback", "FERRIC-M1-R30-ROLLBACK-PARTIAL-CAPTURE-V1"),
        ]
        .into_iter()
        .map(|(kind, source_format)| PhysicalSourceV1 {
            bindings: bindings(),
            capture_sha256: sha256_hex(format!("{kind}:capture").as_bytes()),
            kind,
            protocol_sha256: sha256_hex(format!("{kind}:protocol").as_bytes()),
            source_format,
        })
        .collect()
    }

    fn fault() -> FaultObservationV1 {
        FaultObservationV1 {
            observation_count: 2,
            source_environment_sha256: "55".repeat(32),
            source_executable_sha256: "66".repeat(32),
            source_protocol_sha256: "77".repeat(32),
            source_sha256: "88".repeat(32),
            source_transcript_sha256: "99".repeat(32),
        }
    }

    fn fault_value() -> Value {
        json!({
            "authority": FAULT_AUTHORITY,
            "format": FAULT_OBSERVATION_FORMAT,
            "hardware_claim": "none",
            "milestone": "M1",
            "nonclaim": FAULT_NONCLAIM,
            "obligation_id": "m1.r30",
            "observations": [{
                "failure_observed": true,
                "id": "queue-submit.001",
                "injection_point": "queue-submit",
                "live_resources_after": 0,
                "queue_quarantined": true,
                "retry_denied": true,
            }],
            "source": {
                "environment_sha256": "55".repeat(32),
                "executable_sha256": "66".repeat(32),
                "protocol_sha256": "77".repeat(32),
                "transcript_sha256": "88".repeat(32),
            },
            "status": "reported-unvalidated",
            "target": TARGET,
        })
    }

    fn write_fault(temporary: &Temporary, value: &Value) -> PathBuf {
        let path = temporary.0.join("fault.json");
        fs::write(&path, canonical_bytes(value).unwrap()).unwrap();
        path
    }

    #[test]
    fn composition_preserves_the_external_fault_authority_gap() {
        let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR").unwrap();
        let checked_in = fs::read(
            PathBuf::from(manifest_dir).join("src/bin/ferric-m1-r30-composed-runner-protocol.json"),
        )
        .unwrap();
        assert_eq!(protocol_bytes().unwrap(), checked_in);
        let bytes = compose(&physical(), &fault(), &"aa".repeat(32)).unwrap();
        let value = parse_canonical(&bytes, "test r30 composition").unwrap();
        assert_eq!(value["hardware_claim"], "none");
        assert_eq!(value["qualification"]["evidence_case_count"], 0);
        assert_eq!(
            value["qualification"]["fault_injection_physical_authority"],
            false
        );
        assert_eq!(
            value["case_roster"][3]["authority_class"],
            "external-report"
        );
        assert_eq!(value["case_roster"][3]["status"], "reported-unvalidated");
    }

    #[test]
    fn composition_rejects_cross_device_capture_substitution() {
        let mut sources = physical();
        sources[2].bindings.device_identity_sha256 = "aa".repeat(32);
        assert!(compose(&sources, &fault(), &"bb".repeat(32)).is_err());
    }

    #[test]
    fn fault_admission_rejects_hardware_promotion_and_duplicate_occurrences() {
        let temporary = Temporary::new();
        let path = write_fault(&temporary, &fault_value());
        let admitted = read_fault_observation(&path).unwrap();
        assert_eq!(admitted.observation_count, 1);

        let mut promoted = fault_value();
        promoted["hardware_claim"] = json!("validated");
        fs::write(&path, canonical_bytes(&promoted).unwrap()).unwrap();
        assert!(read_fault_observation(&path).is_err());

        let mut duplicate = fault_value();
        let occurrence = duplicate["observations"][0].clone();
        duplicate["observations"]
            .as_array_mut()
            .unwrap()
            .push(occurrence);
        fs::write(&path, canonical_bytes(&duplicate).unwrap()).unwrap();
        assert!(read_fault_observation(&path).is_err());
    }

    #[test]
    fn publication_is_exact_and_no_replace() {
        let temporary = Temporary::new();
        let output = temporary.0.join("bundle");
        publish(&output, &physical(), &fault()).unwrap();
        let mut names = fs::read_dir(&output)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<Vec<_>>();
        names.sort();
        assert_eq!(names, ["protocol.json", "runner.json"]);
        let before = fs::read(output.join("runner.json")).unwrap();
        assert!(publish(&output, &physical(), &fault()).is_err());
        assert_eq!(fs::read(output.join("runner.json")).unwrap(), before);
    }
}
