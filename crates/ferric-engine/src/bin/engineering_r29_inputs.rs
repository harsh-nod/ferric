//! Authority-none reproducibility coordinates for the explicit engineering route.

use super::{
    canonical_bytes, domain_identity, exact_object, expected_preliminary_kernel_catalog_identity,
    expected_qwen3_gfx942_runner_source_identity, hex_identity, identity_field, CaptureResult,
    ExternalIdentityClosureInputs, Identity, PlanIdentityBindingsV1, DIFFERENTIAL_KINDS, TARGET,
};
use serde_json::{json, Value};

pub(super) const CLOSURE_FORMAT: &str = "FERRIC-M1-ENGINEERING-REPRODUCIBILITY-CLOSURE-V1";
pub(super) const POLICY_FORMAT: &str = "FERRIC-M1-ENGINEERING-DIAGNOSTIC-POLICY-V1";
pub(super) const DERIVATION: &str = "ferric.m1.r29-engineering-coordinates.v1";
pub(super) const NONCLAIM: &str = "Authority-none reproducibility coordinates derived from admitted engineering observation and model-plan bytes only. Runner identity slots are inert engineering coordinates, not compiler-origin, proof, runtime-contract, validator, TCB, or current-publication evidence. No qualification authority or M1 gate closure is supplied.";
pub(super) const POLICY_NONCLAIM: &str = "Engineering numerical diagnostics only. This policy specifies finite BF16 and token-comparison metrics without thresholds or acceptance, and supplies no qualification authority or M1 gate closure.";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct EngineeringArtifactCoordinatesV1 {
    pub(crate) manifest: Identity,
    pub(crate) hsaco: Identity,
    pub(crate) compiler_handoff: Identity,
    pub(crate) canonical_descriptor: Identity,
    pub(crate) program_catalog: Identity,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct EngineeringCoordinatesV1 {
    artifact: EngineeringArtifactCoordinatesV1,
    admission_record: Identity,
    model_bundle: Identity,
    target_prepacked: Identity,
    draft_prepacked: Identity,
    plan_catalog: Identity,
}

impl EngineeringCoordinatesV1 {
    pub(super) fn from_catalog(
        catalog: &ferric_build::SequentialPlanCatalog,
        artifact: EngineeringArtifactCoordinatesV1,
    ) -> CaptureResult<Self> {
        Ok(Self {
            artifact,
            admission_record: catalog
                .admission_record()
                .ok_or_else(|| "engineering model plan has no admission record".to_owned())?
                .record_id(),
            model_bundle: catalog.deployment().bundle_id,
            target_prepacked: Identity::new(catalog.prepacked().target_manifest().aggregate_id()),
            draft_prepacked: Identity::new(catalog.prepacked().draft_manifest().aggregate_id()),
            plan_catalog: catalog.catalog_id(),
        })
    }

    pub(super) fn bytes(self) -> CaptureResult<Vec<u8>> {
        canonical_bytes(&json!({
            "authority": "none",
            "derivation": DERIVATION,
            "format": CLOSURE_FORMAT,
            "model": {
                "admission_record_sha256": hex_identity(self.admission_record),
                "bundle_sha256": hex_identity(self.model_bundle),
                "draft_prepacked_sha256": hex_identity(self.draft_prepacked),
                "plan_catalog_sha256": hex_identity(self.plan_catalog),
                "target_prepacked_sha256": hex_identity(self.target_prepacked),
            },
            "nonclaim": NONCLAIM,
            "observation": {
                "canonical_descriptor_sha256": hex_identity(self.artifact.canonical_descriptor),
                "compiler_handoff_sha256": hex_identity(self.artifact.compiler_handoff),
                "hsaco_sha256": hex_identity(self.artifact.hsaco),
                "manifest_sha256": hex_identity(self.artifact.manifest),
                "program_catalog_sha256": hex_identity(self.artifact.program_catalog),
            },
            "qualification": false,
            "target": TARGET,
        }))
    }

    pub(super) fn parse(value: &Value) -> CaptureResult<Self> {
        let root = exact_object(
            value,
            &[
                "authority",
                "derivation",
                "format",
                "model",
                "nonclaim",
                "observation",
                "qualification",
                "target",
            ],
            "engineering reproducibility closure",
        )?;
        let model = exact_object(
            &root["model"],
            &[
                "admission_record_sha256",
                "bundle_sha256",
                "draft_prepacked_sha256",
                "plan_catalog_sha256",
                "target_prepacked_sha256",
            ],
            "engineering model coordinates",
        )?;
        let artifact = exact_object(
            &root["observation"],
            &[
                "canonical_descriptor_sha256",
                "compiler_handoff_sha256",
                "hsaco_sha256",
                "manifest_sha256",
                "program_catalog_sha256",
            ],
            "engineering artifact coordinates",
        )?;
        let parsed = Self {
            artifact: EngineeringArtifactCoordinatesV1 {
                manifest: identity_field(artifact, "manifest_sha256")?,
                hsaco: identity_field(artifact, "hsaco_sha256")?,
                compiler_handoff: identity_field(artifact, "compiler_handoff_sha256")?,
                canonical_descriptor: identity_field(artifact, "canonical_descriptor_sha256")?,
                program_catalog: identity_field(artifact, "program_catalog_sha256")?,
            },
            admission_record: identity_field(model, "admission_record_sha256")?,
            model_bundle: identity_field(model, "bundle_sha256")?,
            target_prepacked: identity_field(model, "target_prepacked_sha256")?,
            draft_prepacked: identity_field(model, "draft_prepacked_sha256")?,
            plan_catalog: identity_field(model, "plan_catalog_sha256")?,
        };
        if canonical_bytes(value)? != parsed.bytes()? {
            return Err(
                "engineering closure schema, authority or coordinate binding drifted".to_owned(),
            );
        }
        Ok(parsed)
    }

    fn component(self, label: &[u8]) -> Identity {
        domain_identity(
            DERIVATION.as_bytes(),
            &[
                label,
                self.artifact.manifest.as_bytes(),
                self.artifact.hsaco.as_bytes(),
                self.artifact.compiler_handoff.as_bytes(),
                self.artifact.canonical_descriptor.as_bytes(),
                self.artifact.program_catalog.as_bytes(),
                self.admission_record.as_bytes(),
                self.model_bundle.as_bytes(),
                self.target_prepacked.as_bytes(),
                self.draft_prepacked.as_bytes(),
                self.plan_catalog.as_bytes(),
            ],
        )
    }

    pub(super) fn plan_bindings(self) -> PlanIdentityBindingsV1 {
        PlanIdentityBindingsV1 {
            ferric_source: self.component(b"ferric-source-coordinate"),
            fe2o3_source: self.component(b"fe2o3-source-coordinate"),
            protocol: self.component(b"protocol-coordinate"),
        }
    }

    pub(super) fn external_inputs(
        self,
        catalog: &ferric_build::SequentialPlanCatalog,
    ) -> CaptureResult<ExternalIdentityClosureInputs> {
        if Self::from_catalog(catalog, self.artifact)? != self {
            return Err(
                "engineering coordinates no longer bind the authenticated model plan".to_owned(),
            );
        }
        let bindings = self.plan_bindings();
        let mut external = ExternalIdentityClosureInputs {
            ferric_source: bindings.ferric_source,
            fe2o3_source: bindings.fe2o3_source,
            compiler: self.component(b"compiler-coordinate"),
            compiler_configuration: self.component(b"compiler-configuration-coordinate"),
            target_contract: self.component(b"target-contract-coordinate"),
            kernel_catalog: self.component(b"pending-kernel-catalog-coordinate"),
            kernel_proof_set: self.component(b"kernel-proof-slot-coordinate"),
            kernel_abi_catalog: self.component(b"kernel-abi-coordinate"),
            executable_catalog: self.artifact.program_catalog,
            runtime_contract: self.component(b"runtime-contract-coordinate"),
            runtime_abi: self.component(b"runtime-abi-coordinate"),
            generated_runner: expected_qwen3_gfx942_runner_source_identity(),
            validator_registry: self.component(b"validator-slot-coordinate"),
            qualification_protocol: bindings.protocol,
            tcb_report: self.component(b"tcb-slot-coordinate"),
        };
        external.kernel_catalog = expected_preliminary_kernel_catalog_identity(catalog, &external)
            .map_err(|error| {
                format!("cannot derive engineering kernel catalog coordinate: {error:?}")
            })?;
        Ok(external)
    }
}

pub(super) fn policy_value() -> Value {
    json!({
        "authority": "none",
        "case_kinds": DIFFERENTIAL_KINDS,
        "finite_logits_required": true,
        "format": POLICY_FORMAT,
        "logit_metric": "maximum-monotonic-bf16-ulp-distance-signed-zero-equal",
        "nonclaim": POLICY_NONCLAIM,
        "qualification": false,
        "target": TARGET,
        "token_metric": "ferric-reference-greedy-token-mismatch-count",
        "token_selection": "lowest-token-id-bf16-argmax",
    })
}

pub(super) fn validate_policy(value: &Value) -> CaptureResult<()> {
    if value != &policy_value() {
        return Err(
            "engineering diagnostic policy drifted; thresholds and acceptance are forbidden"
                .to_owned(),
        );
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn coordinate_fixture() -> EngineeringCoordinatesV1 {
    EngineeringCoordinatesV1 {
        artifact: EngineeringArtifactCoordinatesV1 {
            manifest: Identity::new([1; 32]),
            hsaco: Identity::new([2; 32]),
            compiler_handoff: Identity::new([3; 32]),
            canonical_descriptor: Identity::new([4; 32]),
            program_catalog: Identity::new([5; 32]),
        },
        admission_record: Identity::new([6; 32]),
        model_bundle: Identity::new([7; 32]),
        target_prepacked: Identity::new([8; 32]),
        draft_prepacked: Identity::new([9; 32]),
        plan_catalog: Identity::new([10; 32]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engineering_record_is_distinct_from_qualification_and_binds_all_ten_coordinates() {
        let coordinates = coordinate_fixture();
        let value: Value = serde_json::from_slice(&coordinates.bytes().unwrap()).unwrap();
        assert_eq!(
            EngineeringCoordinatesV1::parse(&value).unwrap(),
            coordinates
        );
        assert!(super::super::parse_closure_document(&value).is_err());
        let bindings = coordinates.plan_bindings();
        assert_ne!(bindings.ferric_source, bindings.fe2o3_source);
        assert_ne!(bindings.protocol, bindings.ferric_source);
        for group in ["model", "observation"] {
            for key in value[group].as_object().unwrap().keys() {
                let mut changed = value.clone();
                changed[group][key] = json!(hex_identity(Identity::new([240; 32])));
                let parsed = EngineeringCoordinatesV1::parse(&changed).unwrap();
                assert_ne!(parsed, coordinates, "coordinate {group}/{key}");
                let actual = parsed.plan_bindings();
                assert_ne!(actual.ferric_source, bindings.ferric_source);
                assert_ne!(actual.fe2o3_source, bindings.fe2o3_source);
                assert_ne!(actual.protocol, bindings.protocol);
            }
        }
    }

    #[test]
    fn engineering_record_rejects_authority_theorem_schema_and_target_claims() {
        let value: Value = serde_json::from_slice(&coordinate_fixture().bytes().unwrap()).unwrap();
        for (key, replacement) in [
            ("authority", json!("qualified")),
            ("format", json!(super::super::CLOSURE_FORMAT)),
            ("qualification", json!(true)),
            ("derivation", json!("unbound")),
            ("target", json!("gfx950:xnack-")),
            ("kernel_proof_set", json!("f".repeat(64))),
            ("theorem_result", json!("verified")),
            ("nonclaim", json!("")),
        ] {
            let mut changed = value.clone();
            changed[key] = replacement;
            assert!(
                EngineeringCoordinatesV1::parse(&changed).is_err(),
                "accepted {key}"
            );
        }
    }

    #[test]
    fn diagnostic_policy_has_exact_seven_kinds_and_no_threshold_acceptance() {
        let policy = policy_value();
        validate_policy(&policy).unwrap();
        assert_eq!(policy["case_kinds"].as_array().unwrap().len(), 7);
        for (key, value) in [
            ("maximum_logit_ulp_error", json!(0)),
            ("maximum_token_mismatches", json!(0)),
            ("accepted", json!(true)),
            ("qualification", json!(true)),
            ("finite_logits_required", json!(false)),
            ("case_kinds", json!(["prefill-s1-t128"])),
        ] {
            let mut changed = policy.clone();
            changed[key] = value;
            assert!(validate_policy(&changed).is_err(), "accepted {key}");
        }
    }

    #[test]
    fn qualification_capture_rejects_engineering_mode_before_any_input_or_gpu_access() {
        let error = super::super::run_capture_with_input_purpose::<
            super::super::PersistedM1R29CaptureProgramSourceV1,
        >(
            &[],
            super::super::CapturePurposeV1::Qualification,
            super::super::input_bundle::InputDocumentPurposeV1::EngineeringCoordinates,
        )
        .unwrap_err();
        assert_eq!(
            error,
            "engineering inputs cannot enter qualification capture"
        );
    }
}
