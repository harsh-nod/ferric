#![allow(unused_imports)]

//! M1 source-level model-bundle composition theorem.
//!
//! This exact roadmap path lifts the executable `ferric-build` foundation. It
//! proves that successful revalidation retains the exact sealed admission and
//! its canonical deployment consequence. It does not close
//! `model_bundle_well_formed`: signatures, independent provenance, complete
//! weight-manifest validity and layout, runtime roster and plan joins, artifact
//! loading, launch, hardware, numerical, performance, and qualification
//! evidence remain separate Open obligations.

use ferric_build::{AuthenticatedBundleAdmission, ModelBundleProof, ModelBundleProofFailure};
use vstd::prelude::*;

verus! {

/// Exact source-level authority composed by the M1 model-bundle path.
pub open spec fn m1_model_bundle_composition_spec(
    authority: AuthenticatedBundleAdmission,
) -> bool {
    ferric_build::model_bundle::model_bundle_composition_spec(authority)
}

/// Revalidates one exact sealed admission through the compiler-rooted roadmap
/// path while retaining custody on both success and failure.
///
/// # Errors
///
/// Returns the exact `ferric-build` consistency failure with the unchanged
/// admission available for diagnosis or retry.
pub fn model_bundle_well_formed_composition_theorem(
    admission: AuthenticatedBundleAdmission,
) -> (result: Result<ModelBundleProof, Box<ModelBundleProofFailure>>)
    ensures match result {
        Ok(proof) => {
            &&& proof.admission_spec() == admission
            &&& m1_model_bundle_composition_spec(proof.admission_spec())
        },
        Err(failure) => failure.admission_spec() == admission,
    },
{
    let result = ferric_build::prove_model_bundle_composition(admission);
    proof {
        reveal(m1_model_bundle_composition_spec);
    }
    result
}

} // verus!
