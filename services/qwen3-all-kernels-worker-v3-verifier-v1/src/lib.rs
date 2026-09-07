//! Ferric-owned aggregate Qwen3 protected-verifier service foundation.
//!
//! This crate joins durable replay exclusion, service-owned current-record
//! challenges, fe2o3's V2 unnamed and connected-path session transports, one
//! bounded single-session pathname listener, independent protected providers,
//! and Ferric's existing V1 signed receipt schema. It intentionally provides no
//! process launcher, connector, concrete production checker, or signer and
//! therefore does not close the protected-service deployment gate by itself.

#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

mod durable;
mod listener;
mod service;
mod signer_ipc;

pub use durable::{
    DurableLedgerErrorV1, DurableReplayGuardV1, DurableReservationProviderV2,
    EntropyObjectIdentityV1, LedgerObjectIdentityV1, MAX_DURABLE_LEDGER_RECORDS_V1,
    ProtectedLedgerExternalHeadV1, ProtectedLedgerHeadStoreFailureV1, ProtectedLedgerHeadStoreV1,
    ProtectedLedgerKindV1, ProtectedLedgerReplacementAuthorizationV1,
    ProtectedLedgerStorageCapabilityV1, ProtectedPolicyRevocationV1,
};
pub use listener::{
    FerricProtectedVerifierListenerFailureReasonV2, FerricProtectedVerifierListenerFailureV2,
    FerricProtectedVerifierPeerCredentialsV2, run_ferric_protected_verifier_listener_session_v2,
};
pub use service::{
    AbsoluteSessionDeadlineV1, AuthenticatedCompilerCurrentRecordV1,
    FerricProtectedVerifierServiceConfigErrorV1, FerricProtectedVerifierServiceConfigV1,
    FerricProtectedVerifierServiceFailureV1, FerricProtectedVerifierServiceOutcomeV1,
    IndependentCheckerInputV1, IndependentCheckerProviderV1, IndependentCheckerVerifiedClaimsV1,
    ProtectedCompilerCurrentRecordInputV1, ProtectedCompilerCurrentRecordProviderV1,
    ProtectedProviderClaimErrorV1, ProtectedReceiptSignerInputV1, ProtectedReceiptSignerProviderV1,
    ServiceApplicationRejectionV1, ServiceCallerPolicyV1,
    run_ferric_protected_verifier_accepted_session_v2, run_ferric_protected_verifier_session_v2,
};
pub use signer_ipc::{
    PREOPENED_PROTECTED_RECEIPT_SIGNER_REQUEST_BYTES_V1,
    PREOPENED_PROTECTED_RECEIPT_SIGNER_RESPONSE_BYTES_V1, PreopenedProtectedReceiptSignerClientV1,
    ProtectedReceiptSignerClientAdmissionErrorV1, ProtectedReceiptSignerClientAdmissionFailureV1,
    ProtectedReceiptSignerClientErrorV1, ProtectedReceiptSignerEndpointIdentityErrorV1,
    ProtectedReceiptSignerEndpointIdentityV1, ProtectedReceiptSignerPeerIdentityErrorV1,
    ProtectedReceiptSignerPeerIdentityV1, ProtectedReceiptSignerProtocolErrorV1,
    ProtectedReceiptSignerRequestV1, ProtectedReceiptSignerResponseStatusV1,
    ProtectedReceiptSignerResponseV1,
};
