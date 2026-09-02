//! Ferric-owned aggregate Qwen3 protected-verifier service foundation.
//!
//! This crate joins durable replay exclusion, service-owned current-record
//! challenges, fe2o3's V2 session transport, independent protected providers,
//! and Ferric's existing V1 signed receipt schema. It intentionally provides
//! no concrete production checker or signer and therefore does not close the
//! protected-service deployment gate by itself.

#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

mod durable;
mod service;

pub use durable::{
    DurableLedgerErrorV1, DurableReplayGuardV1, DurableReservationProviderV2,
    EntropyObjectIdentityV1, LedgerObjectIdentityV1, provision_empty_replay_ledger_v1,
    provision_empty_reservation_ledger_v2,
};
pub use service::{
    AbsoluteSessionDeadlineV1, AuthenticatedCompilerCurrentRecordV1,
    FerricProtectedVerifierServiceConfigErrorV1, FerricProtectedVerifierServiceConfigV1,
    FerricProtectedVerifierServiceFailureV1, FerricProtectedVerifierServiceOutcomeV1,
    IndependentCheckerInputV1, IndependentCheckerProviderV1, IndependentCheckerVerifiedClaimsV1,
    ProtectedCompilerCurrentRecordInputV1, ProtectedCompilerCurrentRecordProviderV1,
    ProtectedProviderClaimErrorV1, ProtectedReceiptSignerInputV1, ProtectedReceiptSignerProviderV1,
    ServiceApplicationRejectionV1, ServiceCallerPolicyV1, run_ferric_protected_verifier_session_v2,
};
