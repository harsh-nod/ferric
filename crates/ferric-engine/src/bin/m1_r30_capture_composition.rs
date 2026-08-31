//! Fail-closed composition of the currently admitted m1.r30 capture roster.
//!
//! All five inputs are Ferric-owned physical partial captures bound to one
//! device and runtime identity. None is benchmark evidence or a hardware claim.

use super::{
    canonical_bytes, exact_object, expect_string, parse_canonical, require_relative,
    require_sha256, sha256_hex, CaptureResult, R30PhysicalCaptureBindingsV1, SecureDirectory,
    StagingOutput, TARGET,
};
use rustix::fs::Dir;
use serde_json::json;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::Path;

pub(super) const COMMAND: &str = "compose-r30-runner";

const RUNNER_FORMAT: &str = "FERRIC-M1-R30-COMPOSED-RUNNER-V5";
const PROTOCOL_FORMAT: &str = "FERRIC-M1-R30-COMPOSED-RUNNER-PROTOCOL-V6";
const STATUS: &str = "partial-non-evidence";
const AUTHORITY: &str = "ferric-r30-composed-runner-only";
const NONCLAIM: &str = "Canonical composition of five Ferric-owned physical partial captures. The composer authenticates source protocols, exact source bytes, and common physical runtime identities. The fault case establishes only a deliberate service queue transition, logical Engine quarantine, retry denial, and ordinary queue teardown; it is not a native KFD or GPU fault. This runner makes no hardware claim, supplies no external or independent validation, is not benchmark evidence, and does not close m1.r30/M1.";
const PROTOCOL_NONCLAIM: &str = "Composition and identity-join protocol for five physical partial captures only. The fault-transition source grants no native KFD/device fault or GPU-reset authority. This protocol makes no hardware, correctness, evidence, qualification, m1.r30, or M1 closure claim.";

#[derive(Clone, Debug, Eq, PartialEq)]
struct PhysicalSourceV1 {
    bindings: R30PhysicalCaptureBindingsV1,
    capture_sha256: String,
    kind: &'static str,
    protocol_sha256: String,
    source_format: &'static str,
}

pub(super) fn run(arguments: &[OsString]) -> CaptureResult<()> {
    let [canary, cancellation, exhaustion, rollback, fault_transition, output] = arguments else {
        return Err("usage: ferric-m1-qualification-capture compose-r30-runner CANARY-BUNDLE CANCELLATION-BUNDLE EXHAUSTION-BUNDLE ROLLBACK-BUNDLE FAULT-TRANSITION-BUNDLE OUTPUT-BUNDLE".to_owned());
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
    let fault_transition = read_physical_bundle(
        Path::new(fault_transition),
        "fault-transition",
        "FERRIC-M1-R30-FAULT-TRANSITION-PARTIAL-CAPTURE-V1",
    )?;
    publish(
        Path::new(output),
        &[canary, cancellation, exhaustion, fault_transition, rollback],
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
        #[cfg(feature = "qualification-fault-injection")]
        "fault-transition" => {
            super::m1_r30_fault_transition_partial_capture::admit_persisted_bundle(
                &capture, &protocol,
            )
        }
        #[cfg(not(feature = "qualification-fault-injection"))]
        "fault-transition" => Err(
            "r30 fault-transition composition requires qualification-fault-injection".to_owned(),
        ),
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

fn publish(output: &Path, physical: &[PhysicalSourceV1]) -> CaptureResult<()> {
    let executable_sha256 = super::current_executable_sha256()?;
    let runner = compose(physical, &executable_sha256)?;
    let protocol = protocol_bytes()?;
    validate_runner(&runner, physical, &executable_sha256, &protocol)?;
    let mut staging = StagingOutput::create(output)?;
    staging.write("runner.json", &runner)?;
    staging.write("protocol.json", &protocol)?;
    staging.publish_exact(&[
        ("runner.json", runner.as_slice()),
        ("protocol.json", protocol.as_slice()),
    ])
}

fn compose(physical: &[PhysicalSourceV1], executable_sha256: &str) -> CaptureResult<Vec<u8>> {
    require_sha256(executable_sha256)?;
    let [canary, cancellation, exhaustion, fault_transition, rollback] = physical else {
        return Err("r30 composer requires exactly five physical source captures".to_owned());
    };
    let expected_kinds = [
        "canary",
        "cancellation",
        "exhaustion",
        "fault-transition",
        "rollback",
    ];
    if physical.iter().map(|source| source.kind).ne(expected_kinds) {
        return Err("r30 physical source order or roster drifted".to_owned());
    }
    let common = &canary.bindings;
    if physical.iter().any(|source| &source.bindings != common) {
        return Err("r30 physical captures do not bind one device and runtime identity".to_owned());
    }
    let source = |item: &PhysicalSourceV1| {
        json!({
            "authority_class": if item.kind == "fault-transition" {
                "ferric-physical-queue-transition-fault-capture"
            } else {
                "ferric-physical-partial-capture"
            },
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
            source(fault_transition),
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
            "fault_transition_physical_queue_authority": true,
            "native_device_fault_authority": false,
            "physical_partial_capture_count": 5,
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
    executable_sha256: &str,
    protocol: &[u8],
) -> CaptureResult<()> {
    if bytes != compose(physical, executable_sha256)? {
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
            "fault-transition": "ferric-physical-queue-transition-fault-capture",
            "rollback": "ferric-physical-partial-capture",
        },
        "format": PROTOCOL_FORMAT,
        "hardware_claim": "none",
        "milestone": "M1",
        "nonclaim": PROTOCOL_NONCLAIM,
        "obligation_id": "m1.r30",
        "required_case_roster": ["canary", "cancellation", "exhaustion", "fault-transition", "rollback"],
        "source_capture_formats": [
            "FERRIC-M1-R30-CANARY-PARTIAL-CAPTURE-V4",
            "FERRIC-M1-R30-PARTIAL-CAPTURE-V5",
            "FERRIC-M1-R30-EXHAUSTION-PARTIAL-CAPTURE-V1",
            "FERRIC-M1-R30-FAULT-TRANSITION-PARTIAL-CAPTURE-V1",
            "FERRIC-M1-R30-ROLLBACK-PARTIAL-CAPTURE-V1",
        ],
        "status": STATUS,
        "target": TARGET,
    }))
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
            (
                "fault-transition",
                "FERRIC-M1-R30-FAULT-TRANSITION-PARTIAL-CAPTURE-V1",
            ),
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

    #[test]
    fn composition_preserves_the_native_fault_authority_gap() {
        let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR").unwrap();
        let checked_in = fs::read(
            PathBuf::from(manifest_dir).join("src/bin/ferric-m1-r30-composed-runner-protocol.json"),
        )
        .unwrap();
        assert_eq!(protocol_bytes().unwrap(), checked_in);
        let bytes = compose(&physical(), &"aa".repeat(32)).unwrap();
        let value = parse_canonical(&bytes, "test r30 composition").unwrap();
        assert_eq!(value["hardware_claim"], "none");
        assert_eq!(value["qualification"]["evidence_case_count"], 0);
        assert_eq!(
            value["qualification"]["fault_transition_physical_queue_authority"],
            true
        );
        assert_eq!(
            value["case_roster"][3]["authority_class"],
            "ferric-physical-queue-transition-fault-capture"
        );
        assert_eq!(value["case_roster"][3]["status"], STATUS);
        assert_eq!(
            value["qualification"]["native_device_fault_authority"],
            false
        );
    }

    #[test]
    fn composition_rejects_cross_device_capture_substitution() {
        let mut sources = physical();
        sources[2].bindings.device_identity_sha256 = "aa".repeat(32);
        assert!(compose(&sources, &"bb".repeat(32)).is_err());
    }

    #[test]
    fn publication_is_exact_and_no_replace() {
        let temporary = Temporary::new();
        let output = temporary.0.join("bundle");
        publish(&output, &physical()).unwrap();
        let mut names = fs::read_dir(&output)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<Vec<_>>();
        names.sort();
        assert_eq!(names, ["protocol.json", "runner.json"]);
        let before = fs::read(output.join("runner.json")).unwrap();
        assert!(publish(&output, &physical()).is_err());
        assert_eq!(fs::read(output.join("runner.json")).unwrap(), before);
    }
}
