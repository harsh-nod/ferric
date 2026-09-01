//! Fail-closed Ferric adapter for the aggregate Qwen3 Worker V3 verifier boundary.
//!
//! M1 does not yet possess an independently produced protected-verification
//! receipt for the aggregate 12-marker artifact. This backend therefore makes
//! the integration boundary explicit while refusing every verification request.

#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

use std::error::Error;
use std::fmt;

use fe2o3_host::{
    CompilerGeneratedKernelExpectationRosterV1, WorkerV3ProtectedRosterVerificationEvidenceV1,
    WorkerV3ProtectedRosterVerifierBackendV1, WorkerV3RosterVerificationRequestV1,
};
use ferric_qwen3_all_kernels_device_v1::M1AllKernelsWorkerV3RosterV1;

/// Exact number of markers in Ferric's current aggregate M1 roster.
pub const M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1: usize = M1AllKernelsWorkerV3RosterV1::ENTRIES.len();

/// Failure returned by the aggregate M1 protected-verifier scaffold.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum M1AllKernelsProtectedVerifierErrorV1 {
    /// No independently authenticated protected-verification receipt exists.
    MissingProtectedVerificationReceipt {
        /// Number of ordered marker results the missing receipt must cover.
        expected_roster_entries: usize,
    },
}

impl fmt::Display for M1AllKernelsProtectedVerifierErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingProtectedVerificationReceipt {
                expected_roster_entries,
            } => write!(
                formatter,
                "missing protected verification receipt for all {expected_roster_entries} aggregate M1 roster entries"
            ),
        }
    }
}

impl Error for M1AllKernelsProtectedVerifierErrorV1 {}

/// Ferric's current aggregate M1 protected-verifier backend.
///
/// This zero-state scaffold owns no verifier service, protected receipt,
/// compiler authority, load authority, or launch authority. Until a reviewed
/// protected backend replaces it, every request fails closed.
#[derive(Clone, Copy, Debug, Default)]
pub struct M1AllKernelsProtectedVerifierV1;

impl M1AllKernelsProtectedVerifierV1 {
    /// Constructs the fail-closed backend.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    fn reject_missing_protected_receipt()
    -> Result<WorkerV3ProtectedRosterVerificationEvidenceV1, M1AllKernelsProtectedVerifierErrorV1>
    {
        Err(missing_protected_verification_receipt_v1())
    }
}

const fn missing_protected_verification_receipt_v1() -> M1AllKernelsProtectedVerifierErrorV1 {
    M1AllKernelsProtectedVerifierErrorV1::MissingProtectedVerificationReceipt {
        expected_roster_entries: M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1,
    }
}

// SAFETY: this backend cannot claim any of the trait's protected-verification
// obligations because it never constructs or returns verification evidence.
// Every request terminates with the explicit missing-receipt error below.
unsafe impl WorkerV3ProtectedRosterVerifierBackendV1<M1AllKernelsWorkerV3RosterV1>
    for M1AllKernelsProtectedVerifierV1
{
    type Error = M1AllKernelsProtectedVerifierErrorV1;

    unsafe fn verify_protected_roster(
        &mut self,
        _request: &WorkerV3RosterVerificationRequestV1<'_, M1AllKernelsWorkerV3RosterV1>,
    ) -> Result<WorkerV3ProtectedRosterVerificationEvidenceV1, Self::Error> {
        Self::reject_missing_protected_receipt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregate_roster_cardinality_is_exact() {
        assert_eq!(M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1, 12);
        assert_eq!(
            M1_ALL_KERNELS_ROSTER_ENTRY_COUNT_V1,
            M1AllKernelsWorkerV3RosterV1::ENTRIES.len()
        );
    }

    #[test]
    fn production_rejection_is_structured_and_unconditional() {
        let error = M1AllKernelsProtectedVerifierV1::reject_missing_protected_receipt()
            .err()
            .expect("the production backend must reject without a protected receipt");
        assert_eq!(
            error,
            M1AllKernelsProtectedVerifierErrorV1::MissingProtectedVerificationReceipt {
                expected_roster_entries: 12,
            }
        );
        assert_eq!(
            error.to_string(),
            "missing protected verification receipt for all 12 aggregate M1 roster entries"
        );
    }
}
