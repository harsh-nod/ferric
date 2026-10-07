//! Fail-closed collection of aggregate Worker V3 promotion prerequisites.
//!
//! This crate joins one exact durable V2 publication, an externally signed
//! aggregate verifier receipt, and a caller-challenged compiler-current
//! response. The live result deliberately remains a prerequisite: it cannot
//! publish `CURRENT` and grants no publication, load, or launch authority.

#![forbid(unsafe_code)]

use std::{error::Error, fmt};

use fe2o3_artifact_transaction::{
    BuildAttempt, DurableCurrentLinkPublicationTokenV1, DurableLinkPublicationError,
    RetainedDurableDirectoryV1,
};
use fe2o3_runtime_protocol::{
    CompilerExecutionCurrentRecordAttestationV3, CompilerExecutionCurrentRecordVerificationErrorV3,
    CompilerExecutionCurrentRecordVerificationV3, CompilerExecutionIssuerPolicyV1,
    RecoveredWorkerV3LoadEnvelopeV2, VerifiedCompilerExecutionCurrentRecordV3,
    WorkerV3LoadEnvelopeErrorV2, recover_worker_v3_load_envelope_from_retained_directory_v2,
};
use ferric_qwen3_all_kernels_worker_v3_source_pin_v1::{
    M1AggregateSourcePinErrorV1, M1AggregateSourcePinV1, extract_m1_aggregate_source_pin_v1,
};
use sha2::{Digest, Sha256};

use ferric_qwen3_all_kernels_worker_v3_verifier_v1::protected_receipt::{
    M1_ALL_KERNELS_PROTECTED_RECEIPT_BYTES_V1, M1AllKernelsAuthenticatedProtectedVerifierReceiptV1,
    M1AllKernelsProtectedReceiptCompilerClaimsV1, M1AllKernelsProtectedReceiptErrorV1,
    M1AllKernelsProtectedReceiptSourcePinV1, M1AllKernelsProtectedVerifierTrustPolicyV1,
};
use ferric_qwen3_all_kernels_worker_v3_verifier_v1::protected_verifier_service::{
    M1AllKernelsProtectedVerifierServiceEntryV1, M1AllKernelsProtectedVerifierServiceRequestV1,
};

mod refinement;

use refinement::{
    M1AllKernelsCollectorRefinementV1, M1AllKernelsOrderedEntryRefinementV1,
    validate_m1_all_kernels_collector_refinement_v1,
};

/// Number of logical kernel families represented by the aggregate M1 receipt.
pub const M1_ALL_KERNELS_PROMOTION_FAMILY_COUNT_V1: usize = 7;

/// Number of kernel programs represented by the aggregate M1 receipt.
pub const M1_ALL_KERNELS_PROMOTION_PROGRAM_COUNT_V1: usize = 12;

/// One externally supplied input required before collection may begin.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum M1AllKernelsPromotionExternalInputV1 {
    /// Canonical, externally signed protected-verifier receipt bytes.
    ProtectedVerifierReceipt,
    /// Canonical compiler-current verification bytes.
    CompilerCurrentVerification,
    /// Canonical signed compiler-current attestation bytes.
    CompilerCurrentAttestation,
}

/// One exact cross-record coordinate that did not match.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum M1AllKernelsPromotionBindingFieldV1 {
    /// Complete caller-retained protected-verifier intent and ordered roster.
    ProtectedServiceRequest,
    /// Independently supplied compiler issuer policy.
    CompilerIssuerPolicy,
    /// Exact source coordinates extracted from the durable V2 envelope.
    SourcePin,
    /// SHA-256 of the current durable HSACO bytes.
    FinalizedHsacoSha256,
    /// Exact length of the current durable HSACO bytes.
    FinalizedHsacoLength,
    /// Separately transported current verification and nested attestation verification.
    CurrentVerification,
    /// Compiler-execution subject identity.
    CompilerSubject,
    /// Complete compiler receipt-carriage identity.
    CompilerCarriage,
    /// Compiler issuer-policy identity.
    CompilerPolicy,
    /// Compiler issuer-journal identity.
    CompilerIssuerJournal,
    /// Compiler occurrence identity.
    CompilerOccurrence,
    /// Signed compiler receipt identity.
    CompilerReceipt,
    /// Compiler receipt-publication identity.
    CompilerPublication,
    /// Worker acknowledgment identity.
    CompilerAcknowledgment,
    /// Protected Worker-ledger record identity.
    CompilerWorkerLedger,
    /// Worker-ledger rollback sequence.
    CompilerSequence,
    /// Previous compiler rollback anchor.
    CompilerPriorRollbackAnchor,
    /// Current compiler rollback anchor.
    CompilerCurrentRollbackAnchor,
    /// Current-record verification identity.
    CurrentVerificationIdentity,
    /// Current-record attestation identity.
    CurrentAttestationIdentity,
    /// Protected compiler-policy verification identity.
    ProtectedPolicyVerification,
    /// Protected Worker-ledger verification identity.
    ProtectedWorkerLedgerVerification,
    /// External rollback-currentness verification identity.
    ExternalRollbackVerification,
}

/// Failure while collecting one aggregate promotion prerequisite.
#[derive(Debug)]
#[non_exhaustive]
pub enum M1AllKernelsPromotionPrerequisiteErrorV1 {
    /// One required external byte string was empty.
    MissingExternalInput(M1AllKernelsPromotionExternalInputV1),
    /// The caller-provided current-record challenge was zero.
    ZeroCallerChallenge,
    /// The protected-verifier receipt failed strict authentication.
    ProtectedReceipt(M1AllKernelsProtectedReceiptErrorV1),
    /// Exact V2 envelope/current-publication recovery failed.
    Recovery(Box<WorkerV3LoadEnvelopeErrorV2>),
    /// Aggregate source-pin decoding or policy checking failed.
    SourcePin(Box<M1AggregateSourcePinErrorV1>),
    /// The durable publication was busy, stale, replaced, or mutated.
    DurableLink(Box<DurableLinkPublicationError>),
    /// The separately transported current verification was invalid.
    CurrentVerification(CompilerExecutionCurrentRecordVerificationErrorV3),
    /// The signed current attestation was invalid or did not authenticate the explicit inputs.
    CurrentAttestation(CompilerExecutionCurrentRecordVerificationErrorV3),
    /// A signed coordinate differed from the exact recovered publication or current record.
    Binding(M1AllKernelsPromotionBindingFieldV1),
    /// The current artifact length could not be represented in the receipt schema.
    ArtifactLengthOverflow,
    /// The direct Verus refinement gate rejected the collected coordinate set.
    RefinementRejected,
}

impl fmt::Display for M1AllKernelsPromotionPrerequisiteErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingExternalInput(input) => {
                write!(formatter, "required external input is empty: {input:?}")
            }
            Self::ZeroCallerChallenge => {
                formatter.write_str("caller-provided compiler-current challenge is zero")
            }
            Self::ProtectedReceipt(error) => {
                write!(
                    formatter,
                    "protected-verifier receipt was rejected: {error}"
                )
            }
            Self::Recovery(error) => {
                write!(
                    formatter,
                    "exact Worker V3 V2 publication recovery failed: {error}"
                )
            }
            Self::SourcePin(error) => {
                write!(formatter, "aggregate source pin was rejected: {error}")
            }
            Self::DurableLink(error) => {
                write!(
                    formatter,
                    "durable current publication was rejected: {error}"
                )
            }
            Self::CurrentVerification(error) => {
                write!(
                    formatter,
                    "compiler-current verification was rejected: {error}"
                )
            }
            Self::CurrentAttestation(error) => {
                write!(
                    formatter,
                    "compiler-current attestation was rejected: {error}"
                )
            }
            Self::Binding(field) => write!(formatter, "promotion binding mismatch: {field:?}"),
            Self::ArtifactLengthOverflow => {
                formatter.write_str("current artifact length exceeds u64")
            }
            Self::RefinementRejected => {
                formatter.write_str("collector-success refinement rejected one coordinate")
            }
        }
    }
}

impl Error for M1AllKernelsPromotionPrerequisiteErrorV1 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ProtectedReceipt(error) => Some(error),
            Self::Recovery(error) => Some(error.as_ref()),
            Self::SourcePin(error) => Some(error.as_ref()),
            Self::DurableLink(error) => Some(error.as_ref()),
            Self::CurrentVerification(error) | Self::CurrentAttestation(error) => Some(error),
            Self::MissingExternalInput(_)
            | Self::ZeroCallerChallenge
            | Self::Binding(_)
            | Self::ArtifactLengthOverflow
            | Self::RefinementRejected => None,
        }
    }
}

/// Borrowed external verifier and currentness material required for collection.
///
/// There is no default or optional input. The caller must retain the exact
/// canonical verifier request whose complete intent and ordered roster are
/// expected in the signed receipt. Construction rejects missing byte strings
/// and a zero caller-provided challenge before durable recovery is attempted.
pub struct M1AllKernelsPromotionExternalInputsV1<'input> {
    protected_receipt_bytes: &'input [u8],
    protected_trust_policy: &'input M1AllKernelsProtectedVerifierTrustPolicyV1,
    protected_service_request: &'input M1AllKernelsProtectedVerifierServiceRequestV1,
    compiler_issuer_policy: &'input CompilerExecutionIssuerPolicyV1,
    caller_current_challenge: [u8; 32],
    current_verification_bytes: &'input [u8],
    current_attestation_bytes: &'input [u8],
}

impl<'input> M1AllKernelsPromotionExternalInputsV1<'input> {
    /// Admits the complete set of independently provisioned external inputs.
    ///
    /// # Errors
    ///
    /// Returns an error for an empty receipt/record or zero challenge. The
    /// retained service request, exact lengths, canonical encodings, policies,
    /// and signatures are checked by
    /// [`collect_m1_all_kernels_promotion_prerequisite_v1`].
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        protected_receipt_bytes: &'input [u8],
        protected_trust_policy: &'input M1AllKernelsProtectedVerifierTrustPolicyV1,
        protected_service_request: &'input M1AllKernelsProtectedVerifierServiceRequestV1,
        compiler_issuer_policy: &'input CompilerExecutionIssuerPolicyV1,
        caller_current_challenge: [u8; 32],
        current_verification_bytes: &'input [u8],
        current_attestation_bytes: &'input [u8],
    ) -> Result<Self, M1AllKernelsPromotionPrerequisiteErrorV1> {
        for (bytes, input) in [
            (
                protected_receipt_bytes,
                M1AllKernelsPromotionExternalInputV1::ProtectedVerifierReceipt,
            ),
            (
                current_verification_bytes,
                M1AllKernelsPromotionExternalInputV1::CompilerCurrentVerification,
            ),
            (
                current_attestation_bytes,
                M1AllKernelsPromotionExternalInputV1::CompilerCurrentAttestation,
            ),
        ] {
            if bytes.is_empty() {
                return Err(M1AllKernelsPromotionPrerequisiteErrorV1::MissingExternalInput(input));
            }
        }
        if caller_current_challenge == [0; 32] {
            return Err(M1AllKernelsPromotionPrerequisiteErrorV1::ZeroCallerChallenge);
        }
        Ok(Self {
            protected_receipt_bytes,
            protected_trust_policy,
            protected_service_request,
            compiler_issuer_policy,
            caller_current_challenge,
            current_verification_bytes,
            current_attestation_bytes,
        })
    }
}

/// Copyable descriptive identities for a separate external promotion service.
///
/// This value contains no file descriptor, lease, key, signature owner, or
/// currentness capability. It intentionally exposes no serialization API.
#[derive(Clone, Eq, PartialEq)]
pub struct M1AllKernelsPromotionRequestEvidenceV1 {
    attempt: BuildAttempt,
    envelope_sha256: [u8; 32],
    envelope_length: u64,
    finalized_hsaco_sha256: [u8; 32],
    finalized_hsaco_length: u64,
    protected_receipt_identity: [u8; 32],
    protected_trust_policy_identity: [u8; 32],
    protected_service_request_identity: [u8; 32],
    compiler_policy_identity: [u8; 32],
    compiler_carriage_identity: [u8; 32],
    current_verification_identity: [u8; 32],
    current_attestation_identity: [u8; 32],
    external_rollback_verification_identity: [u8; 32],
    caller_current_challenge: [u8; 32],
}

impl fmt::Debug for M1AllKernelsPromotionRequestEvidenceV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AllKernelsPromotionRequestEvidenceV1")
            .field("attempt", &self.attempt)
            .field("envelope_length", &self.envelope_length)
            .field("finalized_hsaco_length", &self.finalized_hsaco_length)
            .field("protected_receipt_identity", &"[redacted]")
            .field("protected_trust_policy_identity", &"[redacted]")
            .field("protected_service_request_identity", &"[redacted]")
            .field("compiler_policy_identity", &"[redacted]")
            .field("compiler_carriage_identity", &"[redacted]")
            .field("current_verification_identity", &"[redacted]")
            .field("current_attestation_identity", &"[redacted]")
            .field("external_rollback_verification_identity", &"[redacted]")
            .field("caller_current_challenge", &"[redacted]")
            .finish_non_exhaustive()
    }
}

impl M1AllKernelsPromotionRequestEvidenceV1 {
    /// Returns the exact durable build-attempt selector.
    #[must_use]
    pub const fn attempt(&self) -> BuildAttempt {
        self.attempt
    }

    /// Returns the exact recovered envelope SHA-256.
    #[must_use]
    pub const fn envelope_sha256(&self) -> [u8; 32] {
        self.envelope_sha256
    }

    /// Returns the exact recovered envelope length.
    #[must_use]
    pub const fn envelope_length(&self) -> u64 {
        self.envelope_length
    }

    /// Returns the SHA-256 of the exact current artifact bytes.
    #[must_use]
    pub const fn finalized_hsaco_sha256(&self) -> [u8; 32] {
        self.finalized_hsaco_sha256
    }

    /// Returns the length of the exact current artifact bytes.
    #[must_use]
    pub const fn finalized_hsaco_length(&self) -> u64 {
        self.finalized_hsaco_length
    }

    /// Returns the identity of the authenticated aggregate verifier receipt.
    #[must_use]
    pub const fn protected_receipt_identity(&self) -> [u8; 32] {
        self.protected_receipt_identity
    }

    /// Returns the explicit protected-verifier trust-policy identity.
    #[must_use]
    pub const fn protected_trust_policy_identity(&self) -> [u8; 32] {
        self.protected_trust_policy_identity
    }

    /// Returns the complete caller-retained verifier intent and roster identity.
    #[must_use]
    pub const fn protected_service_request_identity(&self) -> [u8; 32] {
        self.protected_service_request_identity
    }

    /// Returns the explicit compiler issuer-policy identity.
    #[must_use]
    pub const fn compiler_policy_identity(&self) -> [u8; 32] {
        self.compiler_policy_identity
    }

    /// Returns the complete compiler receipt-carriage identity.
    #[must_use]
    pub const fn compiler_carriage_identity(&self) -> [u8; 32] {
        self.compiler_carriage_identity
    }

    /// Returns the separately transported current-verification identity.
    #[must_use]
    pub const fn current_verification_identity(&self) -> [u8; 32] {
        self.current_verification_identity
    }

    /// Returns the signed current-attestation identity.
    #[must_use]
    pub const fn current_attestation_identity(&self) -> [u8; 32] {
        self.current_attestation_identity
    }

    /// Returns the external rollback-currentness verification identity.
    #[must_use]
    pub const fn external_rollback_verification_identity(&self) -> [u8; 32] {
        self.external_rollback_verification_identity
    }

    /// Returns the challenge provided by the caller for current-record verification.
    #[must_use]
    pub const fn caller_current_challenge(&self) -> [u8; 32] {
        self.caller_current_challenge
    }

    /// Descriptive request evidence never represents a `CURRENT` publication.
    #[must_use]
    pub const fn is_current(&self) -> bool {
        false
    }

    /// Descriptive request evidence grants no publication authority.
    #[must_use]
    pub const fn grants_publication_authority(&self) -> bool {
        false
    }

    /// Descriptive request evidence grants no GPU load authority.
    #[must_use]
    pub const fn grants_load_authority(&self) -> bool {
        false
    }

    /// Descriptive request evidence grants no GPU launch authority.
    #[must_use]
    pub const fn grants_launch_authority(&self) -> bool {
        false
    }

    /// A separately deployed promotion service must consume and revalidate the live owner.
    #[must_use]
    pub const fn requires_external_promotion_service(&self) -> bool {
        true
    }
}

/// Move-only live owner of all correlated aggregate promotion prerequisites.
///
/// The owner retains a locked current-publication token and both authenticated
/// external evidence owners. It is intentionally not `Clone` or serializable.
/// Even while held, it is not a production promotion or `CURRENT` authority.
pub struct M1AllKernelsWorkerV3PromotionPrerequisiteV1 {
    recovered: RecoveredWorkerV3LoadEnvelopeV2,
    current_publication: DurableCurrentLinkPublicationTokenV1,
    _protected_receipt: M1AllKernelsAuthenticatedProtectedVerifierReceiptV1,
    _compiler_current: VerifiedCompilerExecutionCurrentRecordV3,
    request_evidence: M1AllKernelsPromotionRequestEvidenceV1,
}

/// Opaque, move-only handoff produced immediately after locked-current revalidation.
///
/// A separately deployed promotion service consumes this value together with
/// authority that this crate never owns. The retained publication descriptor,
/// current token, and authenticated evidence owners remain private and cannot
/// be decomposed or serialized through this API.
///
/// ```compile_fail
/// use ferric_qwen3_all_kernels_worker_v3_promotion_prerequisite_v1::
///     M1AllKernelsWorkerV3SealedPromotionHandoffV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1AllKernelsWorkerV3SealedPromotionHandoffV1>();
/// ```
///
/// ```compile_fail
/// use ferric_qwen3_all_kernels_worker_v3_promotion_prerequisite_v1::
///     M1AllKernelsWorkerV3SealedPromotionHandoffV1;
/// fn reveal_owner(handoff: &M1AllKernelsWorkerV3SealedPromotionHandoffV1) {
///     let _ = &handoff.owner;
/// }
/// ```
#[must_use = "the locked authenticated handoff must be consumed or deliberately dropped"]
pub struct M1AllKernelsWorkerV3SealedPromotionHandoffV1 {
    owner: M1AllKernelsWorkerV3PromotionPrerequisiteV1,
}

impl fmt::Debug for M1AllKernelsWorkerV3SealedPromotionHandoffV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AllKernelsWorkerV3SealedPromotionHandoffV1")
            .field("request_evidence", &"[redacted]")
            .field("holds_current_publication_lock", &true)
            .field("is_current", &false)
            .field("grants_publication_authority", &false)
            .finish_non_exhaustive()
    }
}

impl M1AllKernelsWorkerV3SealedPromotionHandoffV1 {
    /// Borrows only the authority-free descriptive evidence.
    #[must_use]
    pub const fn request_evidence(&self) -> &M1AllKernelsPromotionRequestEvidenceV1 {
        &self.owner.request_evidence
    }

    /// A sealed handoff is not a `CURRENT` publication.
    #[must_use]
    pub const fn is_current(&self) -> bool {
        false
    }

    /// A sealed handoff grants no publication authority.
    #[must_use]
    pub const fn grants_publication_authority(&self) -> bool {
        false
    }

    /// A sealed handoff grants no GPU load authority.
    #[must_use]
    pub const fn grants_load_authority(&self) -> bool {
        false
    }

    /// A sealed handoff grants no GPU launch authority.
    #[must_use]
    pub const fn grants_launch_authority(&self) -> bool {
        false
    }
}

impl fmt::Debug for M1AllKernelsWorkerV3PromotionPrerequisiteV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AllKernelsWorkerV3PromotionPrerequisiteV1")
            .field("request_evidence", &"[redacted]")
            .field("holds_current_publication_lock", &true)
            .field("is_current", &false)
            .field("grants_publication_authority", &false)
            .finish_non_exhaustive()
    }
}

impl M1AllKernelsWorkerV3PromotionPrerequisiteV1 {
    /// Borrows copyable, authority-free promotion-request identities.
    #[must_use]
    pub const fn request_evidence(&self) -> &M1AllKernelsPromotionRequestEvidenceV1 {
        &self.request_evidence
    }

    /// Collection does not write or represent `CURRENT`.
    #[must_use]
    pub const fn is_current(&self) -> bool {
        false
    }

    /// Collection grants no publication authority.
    #[must_use]
    pub const fn grants_publication_authority(&self) -> bool {
        false
    }

    /// Collection grants no protected-verifier deployment authority.
    #[must_use]
    pub const fn grants_verifier_authority(&self) -> bool {
        false
    }

    /// Collection grants no GPU load authority.
    #[must_use]
    pub const fn grants_load_authority(&self) -> bool {
        false
    }

    /// Collection grants no GPU launch authority.
    #[must_use]
    pub const fn grants_launch_authority(&self) -> bool {
        false
    }

    /// A separately deployed authority must revalidate and consume this owner.
    #[must_use]
    pub const fn requires_external_promotion_service(&self) -> bool {
        true
    }

    /// Revalidates the retained locked `CURRENT` publication and consumes this
    /// owner into the only supported external-service handoff state.
    ///
    /// The returned value remains authority-free. A promotion service must
    /// supply its own publication capability and atomically consume its own
    /// challenge/current-ledger state around this handoff.
    ///
    /// # Errors
    ///
    /// Returns a typed durable-link error if `CURRENT`, its locked descriptor,
    /// or the retained publication changed after collection.
    pub fn revalidate_and_seal_for_external_promotion_v1(
        self,
    ) -> Result<
        M1AllKernelsWorkerV3SealedPromotionHandoffV1,
        M1AllKernelsPromotionPrerequisiteErrorV1,
    > {
        revalidate_retained_current_publication(&self.recovered, &self.current_publication)?;
        Ok(M1AllKernelsWorkerV3SealedPromotionHandoffV1 { owner: self })
    }
}

/// Collects one source/artifact/currentness-bound aggregate promotion prerequisite.
///
/// The caller must retain the exact durable root, select an exact attempt, and independently supply
/// both trust policies, the exact caller-retained verifier request, a caller-provided
/// nonzero challenge, and all three signed byte strings. There is no ambient
/// input discovery and this function never writes a selector or `CURRENT`
/// record.
///
/// # Errors
///
/// Fails closed if authentication, exact V2 recovery, source/artifact binding,
/// compiler-current verification, or any cross-record coordinate differs.
// The order is security-relevant: every authenticated owner and coordinate is retained until the
// final proof gate and move-only owner construction.
#[allow(clippy::too_many_lines)]
pub fn collect_m1_all_kernels_promotion_prerequisite_v1(
    directory: &RetainedDurableDirectoryV1,
    attempt: BuildAttempt,
    external: &M1AllKernelsPromotionExternalInputsV1<'_>,
) -> Result<M1AllKernelsWorkerV3PromotionPrerequisiteV1, M1AllKernelsPromotionPrerequisiteErrorV1> {
    let protected_receipt = external
        .protected_trust_policy
        .authenticate_canonical(external.protected_receipt_bytes)
        .map_err(M1AllKernelsPromotionPrerequisiteErrorV1::ProtectedReceipt)?;
    require_binding(
        external
            .protected_service_request
            .matches_receipt(protected_receipt.receipt()),
        M1AllKernelsPromotionBindingFieldV1::ProtectedServiceRequest,
    )?;
    let recovered = recover_worker_v3_load_envelope_from_retained_directory_v2(directory, attempt)
        .map_err(|error| M1AllKernelsPromotionPrerequisiteErrorV1::Recovery(Box::new(error)))?;

    let source_pin = extract_m1_aggregate_source_pin_v1(
        recovered.canonical_evidence_view().exact_canonical_bytes(),
    )
    .map_err(|error| M1AllKernelsPromotionPrerequisiteErrorV1::SourcePin(Box::new(error)))?
    .source_pin();
    let request_claims = protected_receipt.receipt().request_claims();
    require_binding(
        source_pin_matches(source_pin, request_claims.source_pin()),
        M1AllKernelsPromotionBindingFieldV1::SourcePin,
    )?;

    let artifact_length = u64::try_from(recovered.exact_artifact_bytes().len())
        .map_err(|_| M1AllKernelsPromotionPrerequisiteErrorV1::ArtifactLengthOverflow)?;
    let artifact_sha256: [u8; 32] = Sha256::digest(recovered.exact_artifact_bytes()).into();
    require_binding(
        request_claims.finalized_hsaco_sha256() == artifact_sha256,
        M1AllKernelsPromotionBindingFieldV1::FinalizedHsacoSha256,
    )?;
    require_binding(
        request_claims.finalized_hsaco_length() == artifact_length,
        M1AllKernelsPromotionBindingFieldV1::FinalizedHsacoLength,
    )?;

    let carriage = recovered.wire().compiler_execution_receipt();
    require_binding(
        carriage.policy() == external.compiler_issuer_policy,
        M1AllKernelsPromotionBindingFieldV1::CompilerIssuerPolicy,
    )?;
    let verification =
        CompilerExecutionCurrentRecordVerificationV3::decode(external.current_verification_bytes)
            .map_err(M1AllKernelsPromotionPrerequisiteErrorV1::CurrentVerification)?;
    let attestation =
        CompilerExecutionCurrentRecordAttestationV3::decode(external.current_attestation_bytes)
            .map_err(M1AllKernelsPromotionPrerequisiteErrorV1::CurrentAttestation)?;
    let current_verification_transport = attestation.verification() == &verification
        && attestation.verification().canonical_bytes().as_slice()
            == external.current_verification_bytes;
    require_binding(
        current_verification_transport,
        M1AllKernelsPromotionBindingFieldV1::CurrentVerification,
    )?;
    let compiler_current = attestation
        .verify(
            external.compiler_issuer_policy,
            carriage,
            external.caller_current_challenge,
        )
        .map_err(M1AllKernelsPromotionPrerequisiteErrorV1::CurrentAttestation)?;
    let compiler_checks = validate_compiler_claims(
        protected_receipt.receipt().compiler_claims(),
        recovered.wire(),
        &compiler_current,
    )?;

    let envelope_binding = recovered.canonical_evidence_view().binding();
    let request_evidence = M1AllKernelsPromotionRequestEvidenceV1 {
        attempt,
        envelope_sha256: envelope_binding.sha256(),
        envelope_length: envelope_binding.byte_length(),
        finalized_hsaco_sha256: artifact_sha256,
        finalized_hsaco_length: artifact_length,
        protected_receipt_identity: *protected_receipt.receipt().identity().as_bytes(),
        protected_trust_policy_identity: *protected_receipt.policy_identity().as_bytes(),
        protected_service_request_identity: *external
            .protected_service_request
            .identity()
            .as_bytes(),
        compiler_policy_identity: *carriage.policy().identity().as_bytes(),
        compiler_carriage_identity: *carriage.identity().as_bytes(),
        current_verification_identity: *compiler_current.verification().identity().as_bytes(),
        current_attestation_identity: *compiler_current.attestation().identity().as_bytes(),
        external_rollback_verification_identity: compiler_current
            .external_rollback_verification_identity(),
        caller_current_challenge: external.caller_current_challenge,
    };
    let current_publication = acquire_and_revalidate_current_publication(&recovered)?;
    let receipt_entries = protected_receipt.receipt().entries();
    let request_entries = external.protected_service_request.entries();
    let refinement = M1AllKernelsCollectorRefinementV1 {
        protected_receipt_authenticated: true,
        protected_service_request: external
            .protected_service_request
            .matches_receipt(protected_receipt.receipt()),
        recovered_current_publication: true,
        source_pin: source_pin_matches(source_pin, request_claims.source_pin()),
        finalized_hsaco_sha256: request_claims.finalized_hsaco_sha256() == artifact_sha256,
        finalized_hsaco_length: request_claims.finalized_hsaco_length() == artifact_length,
        compiler_issuer_policy: carriage.policy() == external.compiler_issuer_policy,
        current_verification_transport,
        current_attestation: true,
        compiler_subject: compiler_checks.subject,
        compiler_carriage: compiler_checks.carriage,
        compiler_policy: compiler_checks.policy,
        compiler_issuer_journal: compiler_checks.issuer_journal,
        compiler_occurrence: compiler_checks.occurrence,
        compiler_receipt: compiler_checks.receipt,
        compiler_publication: compiler_checks.publication,
        compiler_acknowledgment: compiler_checks.acknowledgment,
        compiler_worker_ledger: compiler_checks.worker_ledger,
        compiler_sequence: compiler_checks.sequence,
        compiler_prior_rollback_anchor: compiler_checks.prior_rollback_anchor,
        compiler_current_rollback_anchor: compiler_checks.current_rollback_anchor,
        current_verification_identity: compiler_checks.current_verification_identity,
        current_attestation_identity: compiler_checks.current_attestation_identity,
        protected_policy_verification: compiler_checks.protected_policy_verification,
        protected_worker_ledger_verification: compiler_checks.protected_worker_ledger_verification,
        external_rollback_verification: compiler_checks.external_rollback_verification,
        current_token_lease_binding: true,
        locked_currentness_revalidated: true,
        ordered_entries: M1AllKernelsOrderedEntryRefinementV1 {
            entry_00: service_entry_matches(&request_entries[0], &receipt_entries[0]),
            entry_01: service_entry_matches(&request_entries[1], &receipt_entries[1]),
            entry_02: service_entry_matches(&request_entries[2], &receipt_entries[2]),
            entry_03: service_entry_matches(&request_entries[3], &receipt_entries[3]),
            entry_04: service_entry_matches(&request_entries[4], &receipt_entries[4]),
            entry_05: service_entry_matches(&request_entries[5], &receipt_entries[5]),
            entry_06: service_entry_matches(&request_entries[6], &receipt_entries[6]),
            entry_07: service_entry_matches(&request_entries[7], &receipt_entries[7]),
            entry_08: service_entry_matches(&request_entries[8], &receipt_entries[8]),
            entry_09: service_entry_matches(&request_entries[9], &receipt_entries[9]),
            entry_10: service_entry_matches(&request_entries[10], &receipt_entries[10]),
            entry_11: service_entry_matches(&request_entries[11], &receipt_entries[11]),
        },
    };
    let refinement_outcome = validate_m1_all_kernels_collector_refinement_v1(&refinement);
    if !refinement_outcome.accepted || !refinement_outcome.is_authority_free() {
        return Err(M1AllKernelsPromotionPrerequisiteErrorV1::RefinementRejected);
    }
    Ok(M1AllKernelsWorkerV3PromotionPrerequisiteV1 {
        recovered,
        current_publication,
        _protected_receipt: protected_receipt,
        _compiler_current: compiler_current,
        request_evidence,
    })
}

fn acquire_and_revalidate_current_publication(
    recovered: &RecoveredWorkerV3LoadEnvelopeV2,
) -> Result<DurableCurrentLinkPublicationTokenV1, M1AllKernelsPromotionPrerequisiteErrorV1> {
    let lease = recovered.current_publication_lease();
    let token = lease
        .acquire_current_token()
        .map_err(|error| M1AllKernelsPromotionPrerequisiteErrorV1::DurableLink(Box::new(error)))?;
    revalidate_retained_current_publication(recovered, &token)?;
    Ok(token)
}

fn revalidate_retained_current_publication(
    recovered: &RecoveredWorkerV3LoadEnvelopeV2,
    token: &DurableCurrentLinkPublicationTokenV1,
) -> Result<(), M1AllKernelsPromotionPrerequisiteErrorV1> {
    let lease = recovered.current_publication_lease();
    lease
        .validate_current_token(token)
        .map_err(|error| M1AllKernelsPromotionPrerequisiteErrorV1::DurableLink(Box::new(error)))?;
    token
        .revalidate_locked_currentness()
        .map_err(|error| M1AllKernelsPromotionPrerequisiteErrorV1::DurableLink(Box::new(error)))?;
    Ok(())
}

// Each Boolean corresponds one-to-one with an independently authenticated compiler claim.
#[allow(clippy::struct_excessive_bools)]
struct M1CompilerClaimMatchesV1 {
    subject: bool,
    carriage: bool,
    policy: bool,
    issuer_journal: bool,
    occurrence: bool,
    receipt: bool,
    publication: bool,
    acknowledgment: bool,
    worker_ledger: bool,
    sequence: bool,
    prior_rollback_anchor: bool,
    current_rollback_anchor: bool,
    current_verification_identity: bool,
    current_attestation_identity: bool,
    protected_policy_verification: bool,
    protected_worker_ledger_verification: bool,
    external_rollback_verification: bool,
}

// Constructing and rejecting the complete coordinate set together prevents partial validation.
#[allow(clippy::too_many_lines)]
fn validate_compiler_claims(
    claims: &M1AllKernelsProtectedReceiptCompilerClaimsV1,
    wire: &fe2o3_runtime_protocol::WorkerV3LoadEnvelopeWireV2,
    current: &VerifiedCompilerExecutionCurrentRecordV3,
) -> Result<M1CompilerClaimMatchesV1, M1AllKernelsPromotionPrerequisiteErrorV1> {
    let carriage = wire.compiler_execution_receipt();
    let subject = wire
        .reconstructed_compiler_execution_subject_v1()
        .map_err(|error| M1AllKernelsPromotionPrerequisiteErrorV1::Recovery(Box::new(error)))?;
    let publication = carriage.publication();
    let acknowledgment = carriage.acknowledgment();
    let verification = current.verification();
    let matches = M1CompilerClaimMatchesV1 {
        subject: claims.subject_sha256() == *subject.identity().sha256(),
        carriage: claims.carriage_sha256() == *carriage.identity().as_bytes(),
        policy: claims.policy_sha256() == *carriage.policy().identity().as_bytes(),
        issuer_journal: claims.issuer_journal_sha256() == publication.issuer_journal_identity(),
        occurrence: claims.compiler_occurrence_sha256()
            == publication.compiler_occurrence_identity(),
        receipt: claims.receipt_sha256() == *publication.receipt_identity().as_bytes(),
        publication: claims.publication_sha256() == *publication.identity().as_bytes(),
        acknowledgment: claims.acknowledgment_sha256() == *acknowledgment.identity().as_bytes(),
        worker_ledger: claims.worker_ledger_record_sha256()
            == acknowledgment.worker_ledger_record_identity(),
        sequence: claims.sequence() == acknowledgment.sequence(),
        prior_rollback_anchor: claims.prior_rollback_anchor()
            == publication.receipt().prior_rollback_anchor(),
        current_rollback_anchor: claims.current_rollback_anchor()
            == acknowledgment.current_rollback_anchor(),
        current_verification_identity: claims.current_record_verification_sha256()
            == *verification.identity().as_bytes(),
        current_attestation_identity: claims.current_record_attestation_sha256()
            == *current.attestation().identity().as_bytes(),
        protected_policy_verification: claims.protected_policy_verification_sha256()
            == verification.protected_policy_verification_identity(),
        protected_worker_ledger_verification: claims.protected_worker_ledger_verification_sha256()
            == verification.protected_worker_ledger_verification_identity(),
        external_rollback_verification: claims.external_rollback_verification_sha256()
            == verification.external_rollback_verification_identity(),
    };
    for (coordinate_matches, field) in [
        (
            matches.subject,
            M1AllKernelsPromotionBindingFieldV1::CompilerSubject,
        ),
        (
            matches.carriage,
            M1AllKernelsPromotionBindingFieldV1::CompilerCarriage,
        ),
        (
            matches.policy,
            M1AllKernelsPromotionBindingFieldV1::CompilerPolicy,
        ),
        (
            matches.issuer_journal,
            M1AllKernelsPromotionBindingFieldV1::CompilerIssuerJournal,
        ),
        (
            matches.occurrence,
            M1AllKernelsPromotionBindingFieldV1::CompilerOccurrence,
        ),
        (
            matches.receipt,
            M1AllKernelsPromotionBindingFieldV1::CompilerReceipt,
        ),
        (
            matches.publication,
            M1AllKernelsPromotionBindingFieldV1::CompilerPublication,
        ),
        (
            matches.acknowledgment,
            M1AllKernelsPromotionBindingFieldV1::CompilerAcknowledgment,
        ),
        (
            matches.worker_ledger,
            M1AllKernelsPromotionBindingFieldV1::CompilerWorkerLedger,
        ),
        (
            matches.sequence,
            M1AllKernelsPromotionBindingFieldV1::CompilerSequence,
        ),
        (
            matches.prior_rollback_anchor,
            M1AllKernelsPromotionBindingFieldV1::CompilerPriorRollbackAnchor,
        ),
        (
            matches.current_rollback_anchor,
            M1AllKernelsPromotionBindingFieldV1::CompilerCurrentRollbackAnchor,
        ),
        (
            matches.current_verification_identity,
            M1AllKernelsPromotionBindingFieldV1::CurrentVerificationIdentity,
        ),
        (
            matches.current_attestation_identity,
            M1AllKernelsPromotionBindingFieldV1::CurrentAttestationIdentity,
        ),
        (
            matches.protected_policy_verification,
            M1AllKernelsPromotionBindingFieldV1::ProtectedPolicyVerification,
        ),
        (
            matches.protected_worker_ledger_verification,
            M1AllKernelsPromotionBindingFieldV1::ProtectedWorkerLedgerVerification,
        ),
        (
            matches.external_rollback_verification,
            M1AllKernelsPromotionBindingFieldV1::ExternalRollbackVerification,
        ),
    ] {
        require_binding(coordinate_matches, field)?;
    }
    Ok(matches)
}

fn source_pin_matches(
    projected: M1AggregateSourcePinV1,
    claimed: M1AllKernelsProtectedReceiptSourcePinV1,
) -> bool {
    projected.compiler_module_sha256() == claimed.compiler_module_sha256()
        && projected.compiler_module_length() == claimed.compiler_module_length()
        && projected.compiler_handoff_sha256() == claimed.compiler_handoff_sha256()
        && projected.compiler_handoff_length() == claimed.compiler_handoff_length()
        && projected.symbol_manifest_sha256() == claimed.symbol_manifest_sha256()
        && projected.symbol_manifest_length() == claimed.symbol_manifest_length()
}

fn service_entry_matches(
    expected: &M1AllKernelsProtectedVerifierServiceEntryV1,
    actual: &ferric_qwen3_all_kernels_worker_v3_verifier_v1::protected_receipt::M1AllKernelsProtectedReceiptEntryV1,
) -> bool {
    expected == &M1AllKernelsProtectedVerifierServiceEntryV1::from_receipt_entry(actual)
}

fn require_binding(
    matches: bool,
    field: M1AllKernelsPromotionBindingFieldV1,
) -> Result<(), M1AllKernelsPromotionPrerequisiteErrorV1> {
    if matches {
        Ok(())
    } else {
        Err(M1AllKernelsPromotionPrerequisiteErrorV1::Binding(field))
    }
}

const _: [(); M1_ALL_KERNELS_PROTECTED_RECEIPT_BYTES_V1] = [(); 3_552];
const _: [(); M1_ALL_KERNELS_PROMOTION_FAMILY_COUNT_V1] = [(); 7];
const _: [(); M1_ALL_KERNELS_PROMOTION_PROGRAM_COUNT_V1] = [(); 12];

#[cfg(test)]
mod tests {
    use fe2o3_artifact_transaction::BuildAttempt;

    use super::M1AllKernelsPromotionRequestEvidenceV1;

    fn evidence() -> M1AllKernelsPromotionRequestEvidenceV1 {
        M1AllKernelsPromotionRequestEvidenceV1 {
            attempt: BuildAttempt::from_env_value(
                "1:01010101010101010101010101010101:0202020202020202020202020202020202020202020202020202020202020202",
            )
            .expect("valid attempt"),
            envelope_sha256: [3; 32],
            envelope_length: 4,
            finalized_hsaco_sha256: [5; 32],
            finalized_hsaco_length: 6,
            protected_receipt_identity: [7; 32],
            protected_trust_policy_identity: [8; 32],
            protected_service_request_identity: [9; 32],
            compiler_policy_identity: [10; 32],
            compiler_carriage_identity: [11; 32],
            current_verification_identity: [12; 32],
            current_attestation_identity: [13; 32],
            external_rollback_verification_identity: [14; 32],
            caller_current_challenge: [15; 32],
        }
    }

    #[test]
    fn evidence_is_descriptive_nonserializable_and_authority_free() {
        let evidence = evidence();
        assert_eq!(evidence.envelope_sha256(), [3; 32]);
        assert_eq!(evidence.finalized_hsaco_length(), 6);
        assert_eq!(evidence.protected_receipt_identity(), [7; 32]);
        assert_eq!(evidence.protected_service_request_identity(), [9; 32]);
        assert_eq!(evidence.caller_current_challenge(), [15; 32]);
        let debug = format!("{evidence:?}");
        assert!(!debug.contains("0f0f0f0f"));
        assert!(debug.contains("caller_current_challenge: \"[redacted]\""));
        assert!(!evidence.is_current());
        assert!(!evidence.grants_publication_authority());
        assert!(!evidence.grants_load_authority());
        assert!(!evidence.grants_launch_authority());
        assert!(evidence.requires_external_promotion_service());
    }
}
