//! Source-level composition for one retained M1 model-bundle admission.
//!
//! The boundary recomputes the existing verified seal through shared borrows
//! and moves the original non-clone admission only after every check finishes.
//! A successful value is internal consistency authority, not independent
//! authentication. Both retained manifests are rechecked for canonical record
//! commitment and destination layout, without reading the weight images. This
//! does not prove tensor-name semantics or the runtime `BTreeSet` roster.
//! It grants no signature, provenance, artifact, plan join, load, launch,
//! machine, hardware, performance, or qualification authority.

use crate::auth::{
    revalidate_authenticated_bundle, AuthenticatedBundleAdmission, BundleAdmissionError,
};
use crate::weight_stream::{revalidate_weight_manifest_commitment, WeightSectionManifest};
use ferric_spec::{DeploymentBundle, Qwen3ModelRole};
use vstd::prelude::*;

verus! {

/// Exact source-level conclusion retained by the M1 composition boundary.
pub closed spec fn model_bundle_composition_spec(
    authority: AuthenticatedBundleAdmission,
) -> bool {
    &&& crate::auth::authenticated_bundle_admission_spec(authority)
    &&& crate::bundle::canonical_deployment_bundle_spec(authority.deployment_spec())
    &&& authority.target_manifest_spec().valid_commitment()
    &&& authority.draft_manifest_spec().valid_commitment()
    &&& crate::weight_stream::destination_layout_spec(
        authority.target_manifest_spec().role_spec(),
        authority.target_manifest_spec().output_bytes_spec(),
        authority.target_manifest_spec().sections_spec(),
    )
    &&& crate::weight_stream::destination_layout_spec(
        authority.draft_manifest_spec().role_spec(),
        authority.draft_manifest_spec().output_bytes_spec(),
        authority.draft_manifest_spec().sections_spec(),
    )
}

/// Non-clone custody of one source-level model-bundle composition result.
///
/// ```compile_fail
/// fn require_clone<T: Clone>() {}
/// require_clone::<ferric_build::ModelBundleProof>();
/// ```
///
/// ```compile_fail
/// # fn probe(admission: ferric_build::AuthenticatedBundleAdmission) {
/// let _ = ferric_build::ModelBundleProof { admission };
/// # }
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct ModelBundleProof {
    admission: AuthenticatedBundleAdmission,
}

impl ModelBundleProof {
    pub closed spec fn admission_spec(&self) -> AuthenticatedBundleAdmission {
        self.admission
    }

    /// Borrows the exact original admission retained by this proof boundary.
    #[must_use]
    pub const fn admission(&self) -> (admission: &AuthenticatedBundleAdmission)
        ensures *admission == self.admission_spec(),
    {
        &self.admission
    }

    /// Borrows the exact admitted deployment without decomposing its authority.
    #[must_use]
    pub const fn deployment(&self) -> (deployment: &DeploymentBundle)
        ensures *deployment == self.admission_spec().deployment_spec(),
    {
        let prepacked = self.admission.prepacked_exact();
        let deployment = prepacked.deployment_exact();
        proof {
            crate::auth::authenticated_bundle_deployment_is_prepacked(self.admission);
            reveal(ModelBundleProof::admission_spec);
        }
        deployment
    }

    /// Returns the original admission for the next consuming build stage.
    #[must_use]
    pub fn into_admission(self) -> (admission: AuthenticatedBundleAdmission)
        ensures admission == self.admission_spec(),
    {
        self.admission
    }
}

/// Retry-safe rejection retaining the exact original admission authority.
///
/// ```compile_fail
/// fn require_clone<T: Clone>() {}
/// require_clone::<ferric_build::ModelBundleProofFailure>();
/// ```
///
/// ```compile_fail
/// # fn probe(
/// #     error: ferric_build::BundleAdmissionError,
/// #     admission: ferric_build::AuthenticatedBundleAdmission,
/// # ) {
/// let _ = ferric_build::ModelBundleProofFailure { error, admission };
/// # }
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct ModelBundleProofFailure {
    error: BundleAdmissionError,
    admission: AuthenticatedBundleAdmission,
}

impl ModelBundleProofFailure {
    pub closed spec fn error_spec(&self) -> BundleAdmissionError {
        self.error
    }

    pub closed spec fn admission_spec(&self) -> AuthenticatedBundleAdmission {
        self.admission
    }

    /// Returns the exact fail-closed consistency error.
    #[must_use]
    pub const fn error(&self) -> (error: &BundleAdmissionError)
        ensures *error == self.error_spec(),
    {
        &self.error
    }

    /// Recovers the unchanged admission for diagnosis or retry.
    #[must_use]
    pub fn into_parts(
        self,
    ) -> (parts: (BundleAdmissionError, AuthenticatedBundleAdmission))
        ensures
            parts.0 == self.error_spec(),
            parts.1 == self.admission_spec(),
    {
        (self.error, self.admission)
    }
}

fn revalidate_model_bundle_manifests(
    target: &WeightSectionManifest,
    draft: &WeightSectionManifest,
) -> (result: Result<(), BundleAdmissionError>)
    ensures result.is_ok() ==> {
        &&& target.valid_commitment()
        &&& draft.valid_commitment()
        &&& crate::weight_stream::destination_layout_spec(
            target.role_spec(), target.output_bytes_spec(), target.sections_spec(),
        )
        &&& crate::weight_stream::destination_layout_spec(
            draft.role_spec(), draft.output_bytes_spec(), draft.sections_spec(),
        )
    },
{
    if !revalidate_weight_manifest_commitment(target) {
        return Err(BundleAdmissionError::InvalidManifest {
            role: Qwen3ModelRole::Target8B,
            reason: "retained manifest commitment or layout",
        });
    }
    if !revalidate_weight_manifest_commitment(draft) {
        return Err(BundleAdmissionError::InvalidManifest {
            role: Qwen3ModelRole::Draft06B,
            reason: "retained manifest commitment or layout",
        });
    }
    Ok(())
}

fn model_bundle_proof(
    admission: AuthenticatedBundleAdmission,
) -> (proof: ModelBundleProof)
    requires model_bundle_composition_spec(admission),
    ensures proof.admission_spec() == admission,
{
    ModelBundleProof { admission }
}

fn model_bundle_failure(
    error: BundleAdmissionError,
    admission: AuthenticatedBundleAdmission,
) -> (failure: Box<ModelBundleProofFailure>)
    ensures
        failure.error_spec() == error,
        failure.admission_spec() == admission,
{
    Box::new(ModelBundleProofFailure { error, admission })
}

/// Revalidates and consumes one sealed admission into proof-bearing custody.
///
/// All executable checks borrow `admission`. A rejection moves the original
/// value unchanged into [`ModelBundleProofFailure`]; it never decomposes or
/// reconstructs that authority. The exact retained target and draft manifests
/// are revalidated for canonical commitment and destination layout. This does
/// not read the weight images or prove tensor-name semantics, the runtime
/// `BTreeSet` roster, or a later plan join.
///
/// # Errors
///
/// Returns the exact consistency error together with the unchanged admission
/// authority for diagnosis or retry.
pub fn prove_model_bundle_composition(
    admission: AuthenticatedBundleAdmission,
) -> (result: Result<ModelBundleProof, Box<ModelBundleProofFailure>>)
    ensures match result {
        Ok(proof) => {
            &&& proof.admission_spec() == admission
            &&& model_bundle_composition_spec(proof.admission_spec())
        },
        Err(failure) => failure.admission_spec() == admission,
    },
{
    match revalidate_authenticated_bundle(&admission) {
        Ok(()) => {
            assert(crate::auth::authenticated_bundle_admission_spec(admission));
            let prepacked = admission.prepacked_exact();
            match revalidate_model_bundle_manifests(
                prepacked.target_manifest_exact(), prepacked.draft_manifest_exact(),
            ) {
                Ok(()) => {},
                Err(error) => return Err(model_bundle_failure(error, admission)),
            }
            proof {
                crate::auth::authenticated_bundle_manifests_are_prepacked(admission);
                crate::auth::authenticated_bundle_admission_retains_canonical_deployment(
                    admission,
                );
                reveal(model_bundle_composition_spec);
            }
            Ok(model_bundle_proof(admission))
        },
        Err(error) => Err(model_bundle_failure(error, admission)),
    }
}

} // verus!

#[cfg(test)]
mod tests {
    use super::{prove_model_bundle_composition, revalidate_model_bundle_manifests};
    use crate::{
        build_prepacked_deployment_bundle, seal_authenticated_bundle,
        tokenizer::tests::{authenticated_assets, test_tokenizer},
        weight_stream::tests::test_prepacked,
    };
    use ferric_spec::Qwen3ModelRole;

    fn admission() -> crate::AuthenticatedBundleAdmission {
        let prepacked = build_prepacked_deployment_bundle(
            authenticated_assets(),
            test_tokenizer(Qwen3ModelRole::Target8B),
            test_tokenizer(Qwen3ModelRole::Draft06B),
            test_prepacked(Qwen3ModelRole::Target8B),
            test_prepacked(Qwen3ModelRole::Draft06B),
        )
        .expect("complete test prepacked deployment");
        seal_authenticated_bundle(prepacked).expect("sealed admission")
    }

    #[test]
    fn exact_sealed_admission_enters_and_leaves_proof_custody_unchanged() {
        let admission = admission();
        let record_id = admission.record().record_id();
        let deployment = *admission.prepacked().deployment();
        let proof = prove_model_bundle_composition(admission).expect("composition proof");
        assert_eq!(proof.admission().record().record_id(), record_id);
        assert_eq!(*proof.deployment(), deployment);
        let admission = proof.into_admission();
        assert_eq!(admission.record().record_id(), record_id);
        assert_eq!(*admission.prepacked().deployment(), deployment);
    }

    #[test]
    fn retained_manifest_gate_checks_both_roles_without_changing_bytes() {
        for invalid_role in [Qwen3ModelRole::Target8B, Qwen3ModelRole::Draft06B] {
            let mut target = test_prepacked(Qwen3ModelRole::Target8B).into_parts().1;
            let mut draft = test_prepacked(Qwen3ModelRole::Draft06B).into_parts().1;
            let target_bytes = target.canonical_bytes().to_vec();
            let draft_bytes = draft.canonical_bytes().to_vec();
            assert_eq!(revalidate_model_bundle_manifests(&target, &draft), Ok(()));
            let invalid = match invalid_role {
                Qwen3ModelRole::Target8B => &mut target,
                Qwen3ModelRole::Draft06B => &mut draft,
            };
            invalid.test_sections_mut()[0].test_increment_destination_offset();
            assert_eq!(
                revalidate_model_bundle_manifests(&target, &draft),
                Err(crate::BundleAdmissionError::InvalidManifest {
                    role: invalid_role,
                    reason: "retained manifest commitment or layout",
                })
            );
            assert_eq!(target.canonical_bytes(), target_bytes);
            assert_eq!(draft.canonical_bytes(), draft_bytes);
        }
    }
}
