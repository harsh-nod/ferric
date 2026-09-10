//! Source-level authority policy for the promotion-prerequisite collector.

use std::fs;
use std::path::PathBuf;

const SOURCE: &str = include_str!("../src/lib.rs");
const REFINEMENT: &str = include_str!("../src/refinement.rs");
const MANIFEST: &str = include_str!("../Cargo.toml");
const ROOT_MANIFEST: &str = include_str!("../../../Cargo.toml");
const QUALIFIER: &str = include_str!("../../../proofs/qualify-release.sh");
const SOURCE_GATE: &str = include_str!("../../../proofs/source-gate/src/main.rs");
const VERIFIED_MODULES: &str = include_str!("../../../proofs/VERIFIED_MODULES");
const UNVERIFIED_BODIES: &str = include_str!("../../../proofs/UNVERIFIED_BODIES");
const BEHAVIOR_HARNESS: &str = include_str!("behavioral-harness/run.sh");
const BEHAVIOR_PATCH: &str = include_str!("behavioral-harness/patches/collector-test.patch");
const BEHAVIOR_README: &str = include_str!("behavioral-harness/README.md");
const FE2O3_REV: &str = "1b262ac3dd23ee63067e40587d62a124f40b9fc9";
const PACKAGE: &str = "ferric-qwen3-all-kernels-worker-v3-promotion-prerequisite-v1";
const ADAPTER: &str = "adapters/qwen3-all-kernels-worker-v3-promotion-prerequisite-v1";

#[test]
fn promotion_prerequisite_requires_external_inputs_and_denies_authority() {
    for required in [
        "protected_receipt_bytes",
        "protected_trust_policy",
        "protected_service_request",
        "compiler_issuer_policy",
        "caller_current_challenge",
        "current_verification_bytes",
        "current_attestation_bytes",
        "authenticate_canonical",
        "recover_worker_v3_load_envelope_from_retained_directory_v2",
        "RetainedDurableDirectoryV1",
        "extract_m1_aggregate_source_pin_v1",
        "requires_external_promotion_service",
        "protected_receipt_identity",
        "protected_service_request_identity",
    ] {
        assert!(
            SOURCE.contains(required),
            "missing policy anchor {required}"
        );
    }
    for forbidden in [
        "std::env",
        "write_all",
        "File::create",
        "OpenOptions",
        "publish_worker",
        "CURRENT_PATH",
        "set_current",
        "recover_worker_v3_load_envelope_v2",
        "encode_canonical",
        "to_canonical_json",
    ] {
        assert!(
            !SOURCE.contains(forbidden),
            "forbidden authority path {forbidden}"
        );
    }
    assert!(!SOURCE.contains("impl Clone for M1AllKernelsWorkerV3PromotionPrerequisiteV1"));
    assert!(!SOURCE.contains("impl Clone for M1AllKernelsWorkerV3SealedPromotionHandoffV1"));
    assert!(!SOURCE.contains("Serialize for M1AllKernelsWorkerV3PromotionPrerequisiteV1"));
    assert!(!SOURCE.contains("Serialize for M1AllKernelsWorkerV3SealedPromotionHandoffV1"));
    assert!(!SOURCE.contains("pub const fn recovered_publication"));
    assert!(!SOURCE.contains("pub const fn current_publication"));
    assert!(!SOURCE.contains("pub const fn protected_receipt(&self)"));
    assert!(!SOURCE.contains("pub const fn compiler_current(&self)"));
    assert!(!SOURCE.contains("serde"));
    assert!(
        !SOURCE.contains("#[derive(Debug)]\npub struct M1AllKernelsPromotionRequestEvidenceV1")
    );
    assert!(
        !SOURCE
            .contains("#[derive(Debug)]\npub struct M1AllKernelsWorkerV3PromotionPrerequisiteV1")
    );
    assert!(SOURCE.matches("\"[redacted]\"").count() >= 2);
    assert!(SOURCE.contains("revalidate_and_seal_for_external_promotion_v1"));
    assert!(SOURCE.contains("let _ = &handoff.owner;"));
    assert!(!SOURCE.contains("pub fn into_parts"));
    assert!(!SOURCE.contains("pub owner:"));
    let production = SOURCE
        .split("#[cfg(test)]")
        .next()
        .expect("production source");
    assert!(!production.contains(".expect("));
    assert!(!production.contains("unwrap("));
}

#[test]
fn direct_refinement_binds_all_coordinates_and_denies_authority() {
    for coordinate in [
        "protected_receipt_authenticated",
        "protected_service_request",
        "recovered_current_publication",
        "source_pin",
        "finalized_hsaco_sha256",
        "finalized_hsaco_length",
        "compiler_issuer_policy",
        "current_verification_transport",
        "current_attestation",
        "compiler_subject",
        "compiler_carriage",
        "compiler_policy",
        "compiler_issuer_journal",
        "compiler_occurrence",
        "compiler_receipt",
        "compiler_publication",
        "compiler_acknowledgment",
        "compiler_worker_ledger",
        "compiler_sequence",
        "compiler_prior_rollback_anchor",
        "compiler_current_rollback_anchor",
        "current_verification_identity",
        "current_attestation_identity",
        "protected_policy_verification",
        "protected_worker_ledger_verification",
        "external_rollback_verification",
        "current_token_lease_binding",
        "locked_currentness_revalidated",
    ] {
        assert!(
            REFINEMENT.contains(coordinate),
            "missing refinement coordinate {coordinate}"
        );
        assert!(
            SOURCE.contains(coordinate),
            "collector does not populate {coordinate}"
        );
    }
    for index in 0..12 {
        let entry = format!("entry_{index:02}");
        assert_eq!(
            REFINEMENT.matches(&entry).count(),
            4,
            "unexpected proof coverage for {entry}"
        );
        assert!(
            SOURCE.contains(&entry),
            "collector does not populate {entry}"
        );
    }
    assert!(REFINEMENT.contains("successful_refinement_binds_every_coordinate_v1"));
    assert!(REFINEMENT.contains("outcome.is_authority_free_spec()"));
    assert!(SOURCE.contains("validate_m1_all_kernels_collector_refinement_v1(&refinement)"));
    assert!(!REFINEMENT.contains("external_body"));
    assert!(!REFINEMENT.contains("assume("));
}

#[test]
fn hostile_request_or_stale_publication_cannot_produce_a_prerequisite() {
    let collection_start = SOURCE
        .find("pub fn collect_m1_all_kernels_promotion_prerequisite_v1(")
        .expect("collector");
    let helper_start = SOURCE
        .find("fn acquire_and_revalidate_current_publication(")
        .expect("currentness helper");
    let collection = &SOURCE[collection_start..helper_start];
    let helper = &SOURCE[helper_start..];
    let authentication = collection
        .find(".authenticate_canonical(external.protected_receipt_bytes)")
        .expect("protected receipt authentication");
    let full_request_binding = collection
        .find(".matches_receipt(protected_receipt.receipt())")
        .expect("complete intent and ordered-roster binding");
    let recovery = collection
        .find("recover_worker_v3_load_envelope_from_retained_directory_v2(directory, attempt)")
        .expect("retained-directory recovery");
    let currentness = collection
        .find("acquire_and_revalidate_current_publication(&recovered)")
        .expect("currentness acquisition and revalidation");
    let success = collection
        .find("Ok(M1AllKernelsWorkerV3PromotionPrerequisiteV1 {")
        .expect("successful owner construction");
    let token_acquisition = helper
        .find(".acquire_current_token()")
        .expect("current-token acquisition");
    let token_binding = helper
        .find(".validate_current_token(token)")
        .expect("exact-lease token validation");
    let locked_revalidation = helper
        .find(".revalidate_locked_currentness()")
        .expect("locked currentness revalidation");

    assert!(authentication < full_request_binding);
    assert!(full_request_binding < recovery);
    assert!(recovery < currentness);
    assert!(currentness < success);
    assert!(token_acquisition < token_binding);
    assert!(token_binding < locked_revalidation);
    assert!(SOURCE.contains("DurableLink(Box<DurableLinkPublicationError>)"));
    assert!(SOURCE.contains("current_publication: DurableCurrentLinkPublicationTokenV1"));
}

#[test]
fn promotion_prerequisite_uses_only_the_checked_fe2o3_revision() {
    let manifest: toml::Value = toml::from_str(MANIFEST).expect("parse collector manifest");
    let root: toml::Value = toml::from_str(ROOT_MANIFEST).expect("parse root manifest");
    let dependencies = manifest
        .get("dependencies")
        .and_then(toml::Value::as_table)
        .expect("dependencies table");
    let root_dependencies = root
        .get("workspace")
        .and_then(|workspace| workspace.get("dependencies"))
        .and_then(toml::Value::as_table)
        .expect("root workspace dependencies");
    for (name, dependency) in dependencies {
        if name.starts_with("fe2o3-") {
            assert_eq!(
                dependency.get("workspace").and_then(toml::Value::as_bool),
                Some(true)
            );
            let root_dependency = root_dependencies
                .get(name)
                .and_then(toml::Value::as_table)
                .expect("inherited fe2o3 dependency");
            assert_eq!(
                root_dependency.get("rev").and_then(toml::Value::as_str),
                Some(FE2O3_REV)
            );
        }
    }
}

#[test]
fn promotion_prerequisite_is_not_a_current_publisher() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source =
        fs::read_to_string(root.join("src/lib.rs")).expect("read promotion prerequisite source");
    assert_eq!(source, SOURCE);
    assert!(
        source
            .matches("pub const fn is_current(&self) -> bool")
            .count()
            >= 2
    );
    assert!(
        source
            .matches("pub const fn grants_publication_authority(&self) -> bool")
            .count()
            >= 2
    );
    assert!(source.matches("false").count() >= 8);
}

#[test]
fn promotion_prerequisite_is_in_workspace_qualification_and_source_inventory() {
    let root: toml::Value = toml::from_str(ROOT_MANIFEST).expect("parse root manifest");
    let members = root
        .get("workspace")
        .and_then(|workspace| workspace.get("members"))
        .and_then(toml::Value::as_array)
        .expect("workspace members");
    assert!(
        members
            .iter()
            .any(|member| member.as_str() == Some(ADAPTER))
    );

    let manifest: toml::Value = toml::from_str(MANIFEST).expect("parse adapter manifest");
    assert_eq!(
        manifest
            .get("package")
            .and_then(|package| package.get("metadata"))
            .and_then(|metadata| metadata.get("verus"))
            .and_then(|verus| verus.get("verify"))
            .and_then(toml::Value::as_bool),
        Some(true)
    );
    assert_eq!(QUALIFIER.matches(ADAPTER).count(), 4);
    for admitted in [
        format!("package={PACKAGE}|"),
        format!("module={PACKAGE}|{ADAPTER}/src/lib.rs|"),
        format!("module={PACKAGE}|{ADAPTER}/src/refinement.rs|"),
        format!("unverified={PACKAGE}|{ADAPTER}/src/lib.rs|"),
        format!("verified={PACKAGE}|{ADAPTER}/src/refinement.rs|"),
    ] {
        assert!(
            VERIFIED_MODULES.contains(&admitted),
            "missing coverage row {admitted}"
        );
    }
    assert!(
        UNVERIFIED_BODIES.contains(&format!("unverified={PACKAGE}|{ADAPTER}/src/lib.rs|")),
        "promotion executable bodies are absent from the explicit unverified inventory"
    );
    assert!(SOURCE_GATE.matches("PROMOTION_PACKAGE_NAME").count() >= 4);
    assert!(SOURCE_GATE.contains("PROMOTION_NORMAL_DEPENDENCIES"));
    assert!(SOURCE_GATE.contains("PROMOTION_DEV_DEPENDENCIES"));
    assert!(SOURCE_GATE.contains("validate_promotion_resolved_dependencies("));
    assert!(SOURCE_GATE.contains("validate_promotion_proof_only_vstd("));
    assert!(SOURCE_GATE.contains("SOURCE_PIN_CRATE_NAME"));
    assert!(SOURCE_GATE.contains("validate_source_pin_package("));
    assert!(SOURCE_GATE.contains("validate_local_runtime_owner_binding("));
}

#[test]
fn committed_behavior_harness_is_a_full_qualification_gate() {
    assert!(QUALIFIER.contains("FERRIC_QUALITY_GATE=worker-v3-promotion-behavior:BEGIN"));
    assert!(QUALIFIER.contains("FERRIC_QUALITY_GATE=worker-v3-promotion-behavior:PASS"));
    assert!(BEHAVIOR_HARNESS.contains(FE2O3_REV));
    assert!(BEHAVIOR_HARNESS.contains("FERRIC_BEHAVIOR_TARGET_DIR"));
    assert!(
        BEHAVIOR_HARNESS
            .contains("ferric_all12_collector_accepts_exact_live_inputs_and_rejects_hostile_state")
    );
    for rejection in [
        "M1AllKernelsPromotionBindingFieldV1::",
        "CurrentVerification(_)",
        "CurrentAttestation(_)",
        "ProtectedReceipt(_)",
        "Recovery(_)",
        "DurableLink(_)",
        "revalidate_and_seal_for_external_promotion_v1",
        "publish_ferric_all12_worker_v3_fixture_in_directory",
        "libc::flock",
    ] {
        assert!(
            BEHAVIOR_PATCH.contains(rejection),
            "behavior matrix omitted {rejection}"
        );
    }
    for nonclaim in [
        "synthetic",
        "non-production",
        "not compiler-produced evidence",
        "artifact, promotion",
        "V77 strict rejection",
        "external promotion service",
    ] {
        assert!(
            BEHAVIOR_README.contains(nonclaim),
            "behavior nonclaim omitted {nonclaim}"
        );
    }
}
