//! Source-level composition for one retained M1 model-bundle admission.
//!
//! The boundary recomputes the existing verified seal through shared borrows
//! and moves the original non-clone admission only after every check finishes.
//! A successful value is internal consistency authority, not independent
//! authentication. It also does not prove `WeightSectionManifest::valid_commitment`,
//! destination layout, tensor-name semantics, or the runtime `BTreeSet` roster.
//! It grants no signature, provenance, artifact, plan join, load, launch,
//! machine, hardware, performance, or qualification authority.

use crate::auth::{
    revalidate_authenticated_bundle, AuthenticatedBundleAdmission, BundleAdmissionError,
};
use ferric_spec::DeploymentBundle;
use vstd::prelude::*;

verus! {

/// Exact source-level conclusion retained by the M1 composition boundary.
pub closed spec fn model_bundle_composition_spec(
    authority: AuthenticatedBundleAdmission,
) -> bool {
    &&& crate::auth::authenticated_bundle_admission_spec(authority)
    &&& crate::bundle::canonical_deployment_bundle_spec(authority.deployment_spec())
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

proof fn authenticated_admission_composes(
    authority: AuthenticatedBundleAdmission,
)
    requires crate::auth::authenticated_bundle_admission_spec(authority),
    ensures model_bundle_composition_spec(authority),
{
    crate::auth::authenticated_bundle_admission_retains_canonical_deployment(authority);
    reveal(model_bundle_composition_spec);
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
/// reconstructs that authority. This does not prove
/// `WeightSectionManifest::valid_commitment`, manifest destination layout,
/// tensor-name semantics, the runtime `BTreeSet` roster, or a later plan join.
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
            proof { authenticated_admission_composes(admission); }
            Ok(model_bundle_proof(admission))
        },
        Err(error) => Err(model_bundle_failure(error, admission)),
    }
}

} // verus!

#[cfg(test)]
mod tests {
    use super::prove_model_bundle_composition;
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
}
