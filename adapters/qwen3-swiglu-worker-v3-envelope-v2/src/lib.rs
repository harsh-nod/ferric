#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

use std::{error::Error, fmt};

use fe2o3_runtime_protocol::{
    RecoveredWorkerV3LoadEnvelopeV2, WorkerV3LoadEnvelopeErrorV2, WorkerV3LoadEnvelopeWireV2,
};
use ferric_engine::{
    M1SwiGluCompilerReceiptCarriageIdentitiesV1, M1SwiGluProtectedVerifierRequestErrorV1,
    M1SwiGluProtectedVerifierRequestV1, current_m1_swiglu_worker_v3_build_v1,
    prepare_m1_swiglu_protected_verifier_request_v1,
};
use sha2::{Digest, Sha256};

const EXPECTED_REPLAY_SHA256: [u8; 32] =
    hex32(b"093b45da9da3b6859553345aa38e5789aad4949b725e33e4e4d6620045455ed1");
const EXPECTED_REPLAY_LENGTH: u64 = 1_100_878;
const EXPECTED_CLAIM_SHA256: [u8; 32] =
    hex32(b"401b5b2b54190e7bd0e0115da9aa85b17187631e9c9ee2057bf4655c456083e0");
const EXPECTED_CLAIM_LENGTH: u64 = 1_219;
const EXPECTED_COMPILER_CLOSURE: [u8; 32] =
    hex32(b"97664a82bf361020647e36634e90afa30ccc4958c85b2da62baaa01303d75ef8");
const EXPECTED_PUBLICATION_INTENT: [u8; 32] =
    hex32(b"61db6ef6f80e89dc6ac571f99edc5728edc0a3def3c4ad1d117787d4ef743565");
const EXPECTED_FINALIZATION: [u8; 32] =
    hex32(b"37aa965af2c771fcd4c13f635660d25961509d37d0a0572efdb9ec569f53f896");
const EXPECTED_SOURCE_EVIDENCE: [u8; 32] =
    hex32(b"1ce1b7a5c834a14f0334ba75522e9f0aec31ce6761d4516ec36d45c72bfd839f");
const EXPECTED_COMPILER_HANDOFF: [u8; 32] =
    hex32(b"de561a1eb2b66a1b85b05e6bda06c5e545c17d642fd0aa23f0a2458fef532b12");
const EXPECTED_COMPILER_HANDOFF_LENGTH: u64 = 1_096_510;
const EXPECTED_RAW_INSPECTION: [u8; 32] =
    hex32(b"0397e40dc360f47c3b301c3b7aa8a1ce5342f862b7de8c0909c185179d49523c");
const EXPECTED_RAW_OUTPUT: [u8; 32] =
    hex32(b"af9dc3b58ff454dd78253cabbdd1bc2f114e1add2a16c995befbec5a3d50e2b2");
const EXPECTED_FINALIZED_OUTPUT: [u8; 32] =
    hex32(b"57ecb86b40db136237e65a5fae04c955f2c92fe3347c085ec5c806984fc6afa7");
const EXPECTED_OUTPUT_LENGTH: u64 = 14_192;

/// Exact Ferric build axis that did not match the protected `SwiGLU` build.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum M1SwiGluV2BuildFieldV1 {
    NestedReplaySha256,
    NestedReplayLength,
    PublishedClaimSha256,
    PublishedClaimLength,
    CompilerClosure,
    PublicationIntent,
    Finalization,
    SourceEvidence,
    CompilerHandoff,
    CompilerHandoffLength,
    RawInspection,
    RawOutputSha256,
    RawOutputLength,
    FinalizedOutputSha256,
    FinalizedOutputLength,
    RecoveredArtifactSha256,
    RecoveredArtifactLength,
    RecoveredEnvelopeSha256,
    RecoveredEnvelopeLength,
}

/// Failure while deriving a pending Ferric request from one fe2o3 V2 owner.
#[derive(Debug)]
#[non_exhaustive]
pub enum M1SwiGluV2ProjectionErrorV1 {
    Envelope(WorkerV3LoadEnvelopeErrorV2),
    LengthOverflow(&'static str),
    BuildIdentityMismatch(M1SwiGluV2BuildFieldV1),
    PendingRequest(M1SwiGluProtectedVerifierRequestErrorV1),
}

impl fmt::Display for M1SwiGluV2ProjectionErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Envelope(error) => write!(formatter, "invalid Worker V3 envelope V2: {error}"),
            Self::LengthOverflow(field) => write!(formatter, "{field} length exceeds u64"),
            Self::BuildIdentityMismatch(field) => {
                write!(
                    formatter,
                    "SwiGLU Worker V3 build identity drifted: {field:?}"
                )
            }
            Self::PendingRequest(error) => write!(formatter, "pending request rejected: {error}"),
        }
    }
}

impl Error for M1SwiGluV2ProjectionErrorV1 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Envelope(error) => Some(error),
            Self::PendingRequest(error) => Some(error),
            Self::LengthOverflow(_) | Self::BuildIdentityMismatch(_) => None,
        }
    }
}

impl From<WorkerV3LoadEnvelopeErrorV2> for M1SwiGluV2ProjectionErrorV1 {
    fn from(error: WorkerV3LoadEnvelopeErrorV2) -> Self {
        Self::Envelope(error)
    }
}

impl From<M1SwiGluProtectedVerifierRequestErrorV1> for M1SwiGluV2ProjectionErrorV1 {
    fn from(error: M1SwiGluProtectedVerifierRequestErrorV1) -> Self {
        Self::PendingRequest(error)
    }
}

/// Authority-free identity projection derived from one complete V2 carriage.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1SwiGluV2CarriageProjectionV1 {
    envelope_sha256: [u8; 32],
    envelope_length: u64,
    carriage_identity: [u8; 32],
    issuer_policy_identity: [u8; 32],
    compiler_subject_identity: [u8; 32],
    attestation_request_identity: [u8; 32],
    signed_receipt_identity: [u8; 32],
    receipt_publication_identity: [u8; 32],
    worker_acknowledgment_identity: [u8; 32],
    worker_ledger_record_identity: [u8; 32],
    sequence: u64,
    next_rollback_anchor: [u8; 32],
}

impl M1SwiGluV2CarriageProjectionV1 {
    #[must_use]
    pub const fn envelope_sha256(self) -> [u8; 32] {
        self.envelope_sha256
    }

    #[must_use]
    pub const fn envelope_length(self) -> u64 {
        self.envelope_length
    }

    #[must_use]
    pub const fn carriage_identity(self) -> [u8; 32] {
        self.carriage_identity
    }

    #[must_use]
    pub const fn issuer_policy_identity(self) -> [u8; 32] {
        self.issuer_policy_identity
    }

    #[must_use]
    pub const fn compiler_subject_identity(self) -> [u8; 32] {
        self.compiler_subject_identity
    }

    #[must_use]
    pub const fn attestation_request_identity(self) -> [u8; 32] {
        self.attestation_request_identity
    }

    #[must_use]
    pub const fn signed_receipt_identity(self) -> [u8; 32] {
        self.signed_receipt_identity
    }

    #[must_use]
    pub const fn receipt_publication_identity(self) -> [u8; 32] {
        self.receipt_publication_identity
    }

    #[must_use]
    pub const fn worker_acknowledgment_identity(self) -> [u8; 32] {
        self.worker_acknowledgment_identity
    }

    #[must_use]
    pub const fn worker_ledger_record_identity(self) -> [u8; 32] {
        self.worker_ledger_record_identity
    }

    #[must_use]
    pub const fn sequence(self) -> u64 {
        self.sequence
    }

    #[must_use]
    pub const fn next_rollback_anchor(self) -> [u8; 32] {
        self.next_rollback_anchor
    }

    /// Identity projection alone never authenticates compiler process origin.
    #[must_use]
    pub const fn authenticates_compiler_origin(self) -> bool {
        false
    }

    /// Protected policy and rollback verification remain mandatory.
    #[must_use]
    pub const fn requires_protected_verifier(self) -> bool {
        true
    }

    #[must_use]
    pub const fn grants_load_authority(self) -> bool {
        false
    }

    #[must_use]
    pub const fn grants_launch_authority(self) -> bool {
        false
    }
}

/// Complete pending Ferric request derived without caller-authored identity parallels.
pub struct M1SwiGluV2PendingRequestV1 {
    request: M1SwiGluProtectedVerifierRequestV1,
    carriage: M1SwiGluV2CarriageProjectionV1,
}

impl fmt::Debug for M1SwiGluV2PendingRequestV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1SwiGluV2PendingRequestV1")
            .field("request_identity", &self.request.identity())
            .field("carriage", &self.carriage)
            .field("authority", &"none")
            .finish()
    }
}

impl M1SwiGluV2PendingRequestV1 {
    #[must_use]
    pub const fn request(&self) -> &M1SwiGluProtectedVerifierRequestV1 {
        &self.request
    }

    #[must_use]
    pub const fn carriage(&self) -> M1SwiGluV2CarriageProjectionV1 {
        self.carriage
    }

    #[must_use]
    pub const fn authenticates_compiler_origin(&self) -> bool {
        false
    }

    /// V2 carriage does not authenticate Ferric or fe2o3 repository labels.
    #[must_use]
    pub const fn authenticates_repository_revision(&self) -> bool {
        false
    }

    /// Descriptor-source authentication requires the later consuming host-admission join.
    #[must_use]
    pub const fn authenticates_descriptor_source(&self) -> bool {
        false
    }

    #[must_use]
    pub const fn grants_verifier_authority(&self) -> bool {
        false
    }

    #[must_use]
    pub const fn grants_load_authority(&self) -> bool {
        false
    }

    #[must_use]
    pub const fn grants_launch_authority(&self) -> bool {
        false
    }
}

/// Linear precursor that retains recovered V2 custody beside the inert pending request.
///
/// This type is intentionally not `Clone`. Retention does not authenticate the compiler, make
/// currentness permanent, or grant verifier, load, or launch authority. A future consuming
/// verifier/admission join must revalidate currentness at that boundary.
pub struct M1SwiGluV2RecoveredPendingRequestV1 {
    pending: M1SwiGluV2PendingRequestV1,
    owner: RecoveredWorkerV3LoadEnvelopeV2,
}

impl fmt::Debug for M1SwiGluV2RecoveredPendingRequestV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1SwiGluV2RecoveredPendingRequestV1")
            .field("pending", &self.pending)
            .field("retains_recovered_owner", &true)
            .field("authority", &"none")
            .finish_non_exhaustive()
    }
}

impl M1SwiGluV2RecoveredPendingRequestV1 {
    #[must_use]
    pub const fn pending(&self) -> &M1SwiGluV2PendingRequestV1 {
        &self.pending
    }

    #[must_use]
    pub const fn recovered_owner(&self) -> &RecoveredWorkerV3LoadEnvelopeV2 {
        &self.owner
    }

    /// Returns the inert projection and restores linear ownership to the caller.
    #[must_use]
    pub fn into_parts(self) -> (M1SwiGluV2PendingRequestV1, RecoveredWorkerV3LoadEnvelopeV2) {
        (self.pending, self.owner)
    }

    #[must_use]
    pub const fn grants_verifier_authority(&self) -> bool {
        false
    }

    #[must_use]
    pub const fn authenticates_descriptor_source(&self) -> bool {
        false
    }

    #[must_use]
    pub const fn grants_load_authority(&self) -> bool {
        false
    }

    #[must_use]
    pub const fn grants_launch_authority(&self) -> bool {
        false
    }
}

/// Failed recovered projection that returns the still-linear V2 owner.
pub struct M1SwiGluV2RecoveredProjectionFailureV1 {
    error: M1SwiGluV2ProjectionErrorV1,
    owner: RecoveredWorkerV3LoadEnvelopeV2,
}

impl fmt::Debug for M1SwiGluV2RecoveredProjectionFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1SwiGluV2RecoveredProjectionFailureV1")
            .field("error", &self.error)
            .field("returns_recovered_owner", &true)
            .finish_non_exhaustive()
    }
}

impl fmt::Display for M1SwiGluV2RecoveredProjectionFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl Error for M1SwiGluV2RecoveredProjectionFailureV1 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.error)
    }
}

impl M1SwiGluV2RecoveredProjectionFailureV1 {
    #[must_use]
    pub const fn error(&self) -> &M1SwiGluV2ProjectionErrorV1 {
        &self.error
    }

    /// Returns the validation error and restores linear ownership to the caller.
    #[must_use]
    pub fn into_parts(self) -> (M1SwiGluV2ProjectionErrorV1, RecoveredWorkerV3LoadEnvelopeV2) {
        (self.error, self.owner)
    }
}

/// Strictly decodes a canonical V2 wire and derives the complete pending Ferric request.
///
/// V1 envelopes, noncanonical bytes, inconsistent receipt carriage, and exact `SwiGLU` build drift
/// are rejected. This decoded form is inert and does not establish durable currentness.
///
/// # Errors
///
/// Returns [`M1SwiGluV2ProjectionErrorV1`] for invalid V2 bytes or any exact build mismatch.
pub fn decode_m1_swiglu_pending_request_v2(
    bytes: &[u8],
) -> Result<M1SwiGluV2PendingRequestV1, M1SwiGluV2ProjectionErrorV1> {
    let wire = WorkerV3LoadEnvelopeWireV2::decode_canonical(bytes)?;
    project_wire(&wire)
}

/// Derives the complete pending Ferric request from one move-only recovered V2 owner.
///
/// Recovery was performed by fe2o3 and binds the canonical envelope to the publication current at
/// recovery. This adapter takes ownership, checks the exact artifact bytes and readiness-envelope
/// binding, and retains the owner beside the inert projection. Currentness must be revalidated by
/// the future consuming authority transition.
///
/// # Errors
///
/// Returns [`M1SwiGluV2RecoveredProjectionFailureV1`] with the original owner if the recovered
/// artifact, readiness binding, carried build, or pending request does not match exact policy.
pub fn project_m1_swiglu_pending_request_from_recovered_v2(
    owner: RecoveredWorkerV3LoadEnvelopeV2,
) -> Result<M1SwiGluV2RecoveredPendingRequestV1, Box<M1SwiGluV2RecoveredProjectionFailureV1>> {
    let result = (|| {
        let artifact = owner.exact_artifact_bytes();
        require_length(
            artifact.len(),
            EXPECTED_OUTPUT_LENGTH,
            M1SwiGluV2BuildFieldV1::RecoveredArtifactLength,
            "recovered artifact",
        )?;
        require_digest(
            sha256(artifact),
            EXPECTED_FINALIZED_OUTPUT,
            M1SwiGluV2BuildFieldV1::RecoveredArtifactSha256,
        )?;

        let pending = project_wire(owner.wire())?;
        let readiness = owner.receipt().envelope_binding();
        require_digest(
            readiness.sha256(),
            pending.carriage.envelope_sha256,
            M1SwiGluV2BuildFieldV1::RecoveredEnvelopeSha256,
        )?;
        require_equal_u64(
            readiness.byte_length(),
            pending.carriage.envelope_length,
            M1SwiGluV2BuildFieldV1::RecoveredEnvelopeLength,
        )?;
        Ok(pending)
    })();
    match result {
        Ok(pending) => Ok(M1SwiGluV2RecoveredPendingRequestV1 { pending, owner }),
        Err(error) => Err(Box::new(M1SwiGluV2RecoveredProjectionFailureV1 {
            error,
            owner,
        })),
    }
}

fn project_wire(
    wire: &WorkerV3LoadEnvelopeWireV2,
) -> Result<M1SwiGluV2PendingRequestV1, M1SwiGluV2ProjectionErrorV1> {
    let canonical = wire.encode_canonical()?;
    let envelope_length = u64::try_from(canonical.len())
        .map_err(|_| M1SwiGluV2ProjectionErrorV1::LengthOverflow("V2 envelope"))?;
    validate_build_projection(&build_projection(wire)?)?;

    let carriage = wire.compiler_execution_receipt();
    let subject = wire.reconstructed_compiler_execution_subject_v1()?;
    let publication = carriage.publication();
    let receipt = publication.receipt();
    let acknowledgment = carriage.acknowledgment();
    let projection = M1SwiGluV2CarriageProjectionV1 {
        envelope_sha256: sha256(&canonical),
        envelope_length,
        carriage_identity: *carriage.identity().as_bytes(),
        issuer_policy_identity: *carriage.policy().identity().as_bytes(),
        compiler_subject_identity: *subject.identity().sha256(),
        attestation_request_identity: *carriage.request().identity().as_bytes(),
        signed_receipt_identity: *receipt.identity().as_bytes(),
        receipt_publication_identity: *publication.identity().as_bytes(),
        worker_acknowledgment_identity: *acknowledgment.identity().as_bytes(),
        worker_ledger_record_identity: acknowledgment.worker_ledger_record_identity(),
        sequence: receipt.sequence(),
        next_rollback_anchor: receipt.next_rollback_anchor(),
    };

    let request_carriage = M1SwiGluCompilerReceiptCarriageIdentitiesV1::from_untrusted_observation(
        projection.envelope_sha256,
        projection.carriage_identity,
        projection.issuer_policy_identity,
        projection.compiler_subject_identity,
        projection.attestation_request_identity,
        projection.signed_receipt_identity,
        projection.receipt_publication_identity,
        projection.worker_acknowledgment_identity,
        projection.worker_ledger_record_identity,
        projection.sequence,
        projection.next_rollback_anchor,
    );
    let request = prepare_m1_swiglu_protected_verifier_request_v1(
        current_m1_swiglu_worker_v3_build_v1(),
        request_carriage,
    )?;
    Ok(M1SwiGluV2PendingRequestV1 {
        request,
        carriage: projection,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ExactBuildProjectionV1 {
    nested_replay_sha256: [u8; 32],
    nested_replay_length: u64,
    published_claim_sha256: [u8; 32],
    published_claim_length: u64,
    compiler_closure: [u8; 32],
    record_publication_intent: [u8; 32],
    binding_publication_intent: [u8; 32],
    finalization: [u8; 32],
    source_evidence: [u8; 32],
    binding_compiler_handoff: [u8; 32],
    record_compiler_handoff: [u8; 32],
    compiler_handoff_length: u64,
    raw_inspection: [u8; 32],
    raw_output_sha256: [u8; 32],
    raw_output_length: u64,
    binding_finalized_output_sha256: [u8; 32],
    record_finalized_output_sha256: [u8; 32],
    binding_finalized_output_length: u64,
    record_finalized_output_length: u64,
}

fn build_projection(
    wire: &WorkerV3LoadEnvelopeWireV2,
) -> Result<ExactBuildProjectionV1, M1SwiGluV2ProjectionErrorV1> {
    let replay = wire.replay();
    let replay_bytes = replay
        .encode_canonical()
        .map_err(WorkerV3LoadEnvelopeErrorV2::Replay)?;
    let nested_replay_length = u64::try_from(replay_bytes.len())
        .map_err(|_| M1SwiGluV2ProjectionErrorV1::LengthOverflow("nested replay"))?;

    let claim = wire.published_claim();
    let claim_bytes = claim.encode_canonical().map_err(|_| {
        M1SwiGluV2ProjectionErrorV1::BuildIdentityMismatch(
            M1SwiGluV2BuildFieldV1::PublishedClaimSha256,
        )
    })?;
    let published_claim_length = u64::try_from(claim_bytes.len())
        .map_err(|_| M1SwiGluV2ProjectionErrorV1::LengthOverflow("published claim"))?;

    let record = replay.publication_intent_record();
    let binding = claim.worker_v3_binding();
    let compiler_handoff_length = u64::try_from(record.outer_handoff_length())
        .map_err(|_| M1SwiGluV2ProjectionErrorV1::LengthOverflow("compiler handoff"))?;
    let record_finalized_output_length = u64::try_from(record.output_length())
        .map_err(|_| M1SwiGluV2ProjectionErrorV1::LengthOverflow("finalized output"))?;
    Ok(ExactBuildProjectionV1 {
        nested_replay_sha256: sha256(&replay_bytes),
        nested_replay_length,
        published_claim_sha256: sha256(&claim_bytes),
        published_claim_length,
        compiler_closure: binding.compiler_closure().identity_sha256(),
        record_publication_intent: record.identity().as_bytes(),
        binding_publication_intent: binding.publication_intent_record_identity(),
        finalization: binding.finalization_identity(),
        source_evidence: binding.source_evidence_identity(),
        binding_compiler_handoff: binding.compiler_handoff_binding_identity(),
        record_compiler_handoff: record.outer_handoff_sha256(),
        compiler_handoff_length,
        raw_inspection: binding.raw_inspection_identity(),
        raw_output_sha256: binding.raw_output_sha256(),
        raw_output_length: binding.raw_output_length(),
        binding_finalized_output_sha256: binding.finalized_output_sha256(),
        record_finalized_output_sha256: record.output_sha256(),
        binding_finalized_output_length: binding.finalized_output_length(),
        record_finalized_output_length,
    })
}

fn validate_build_projection(
    actual: &ExactBuildProjectionV1,
) -> Result<(), M1SwiGluV2ProjectionErrorV1> {
    for (observed, expected, field) in [
        (
            actual.nested_replay_sha256,
            EXPECTED_REPLAY_SHA256,
            M1SwiGluV2BuildFieldV1::NestedReplaySha256,
        ),
        (
            actual.published_claim_sha256,
            EXPECTED_CLAIM_SHA256,
            M1SwiGluV2BuildFieldV1::PublishedClaimSha256,
        ),
        (
            actual.compiler_closure,
            EXPECTED_COMPILER_CLOSURE,
            M1SwiGluV2BuildFieldV1::CompilerClosure,
        ),
        (
            actual.record_publication_intent,
            EXPECTED_PUBLICATION_INTENT,
            M1SwiGluV2BuildFieldV1::PublicationIntent,
        ),
        (
            actual.binding_publication_intent,
            EXPECTED_PUBLICATION_INTENT,
            M1SwiGluV2BuildFieldV1::PublicationIntent,
        ),
        (
            actual.finalization,
            EXPECTED_FINALIZATION,
            M1SwiGluV2BuildFieldV1::Finalization,
        ),
        (
            actual.source_evidence,
            EXPECTED_SOURCE_EVIDENCE,
            M1SwiGluV2BuildFieldV1::SourceEvidence,
        ),
        (
            actual.binding_compiler_handoff,
            EXPECTED_COMPILER_HANDOFF,
            M1SwiGluV2BuildFieldV1::CompilerHandoff,
        ),
        (
            actual.record_compiler_handoff,
            EXPECTED_COMPILER_HANDOFF,
            M1SwiGluV2BuildFieldV1::CompilerHandoff,
        ),
        (
            actual.raw_inspection,
            EXPECTED_RAW_INSPECTION,
            M1SwiGluV2BuildFieldV1::RawInspection,
        ),
        (
            actual.raw_output_sha256,
            EXPECTED_RAW_OUTPUT,
            M1SwiGluV2BuildFieldV1::RawOutputSha256,
        ),
        (
            actual.binding_finalized_output_sha256,
            EXPECTED_FINALIZED_OUTPUT,
            M1SwiGluV2BuildFieldV1::FinalizedOutputSha256,
        ),
        (
            actual.record_finalized_output_sha256,
            EXPECTED_FINALIZED_OUTPUT,
            M1SwiGluV2BuildFieldV1::FinalizedOutputSha256,
        ),
    ] {
        require_digest(observed, expected, field)?;
    }
    for (observed, expected, field) in [
        (
            actual.nested_replay_length,
            EXPECTED_REPLAY_LENGTH,
            M1SwiGluV2BuildFieldV1::NestedReplayLength,
        ),
        (
            actual.published_claim_length,
            EXPECTED_CLAIM_LENGTH,
            M1SwiGluV2BuildFieldV1::PublishedClaimLength,
        ),
        (
            actual.compiler_handoff_length,
            EXPECTED_COMPILER_HANDOFF_LENGTH,
            M1SwiGluV2BuildFieldV1::CompilerHandoffLength,
        ),
        (
            actual.raw_output_length,
            EXPECTED_OUTPUT_LENGTH,
            M1SwiGluV2BuildFieldV1::RawOutputLength,
        ),
        (
            actual.binding_finalized_output_length,
            EXPECTED_OUTPUT_LENGTH,
            M1SwiGluV2BuildFieldV1::FinalizedOutputLength,
        ),
        (
            actual.record_finalized_output_length,
            EXPECTED_OUTPUT_LENGTH,
            M1SwiGluV2BuildFieldV1::FinalizedOutputLength,
        ),
    ] {
        require_equal_u64(observed, expected, field)?;
    }
    Ok(())
}

fn require_digest(
    actual: [u8; 32],
    expected: [u8; 32],
    field: M1SwiGluV2BuildFieldV1,
) -> Result<(), M1SwiGluV2ProjectionErrorV1> {
    if actual == expected {
        Ok(())
    } else {
        Err(M1SwiGluV2ProjectionErrorV1::BuildIdentityMismatch(field))
    }
}

fn require_equal_u64(
    actual: u64,
    expected: u64,
    field: M1SwiGluV2BuildFieldV1,
) -> Result<(), M1SwiGluV2ProjectionErrorV1> {
    if actual == expected {
        Ok(())
    } else {
        Err(M1SwiGluV2ProjectionErrorV1::BuildIdentityMismatch(field))
    }
}

fn require_length(
    actual: usize,
    expected: u64,
    field: M1SwiGluV2BuildFieldV1,
    name: &'static str,
) -> Result<(), M1SwiGluV2ProjectionErrorV1> {
    let actual =
        u64::try_from(actual).map_err(|_| M1SwiGluV2ProjectionErrorV1::LengthOverflow(name))?;
    require_equal_u64(actual, expected, field)
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

const fn hex32(input: &[u8; 64]) -> [u8; 32] {
    let mut output = [0_u8; 32];
    let mut index = 0;
    while index < output.len() {
        output[index] = (hex_digit(input[index * 2]) << 4) | hex_digit(input[index * 2 + 1]);
        index += 1;
    }
    output
}

const fn hex_digit(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => panic!("invalid lowercase hexadecimal digit"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fe2o3_runtime_protocol::{
        COMPILER_EXECUTION_RECEIPT_CARRIAGE_BYTES_V1, MAX_WORKER_V3_LOAD_ENVELOPE_BYTES_V2,
        WORKER_V3_LOAD_ENVELOPE_MAGIC_V1, WORKER_V3_LOAD_ENVELOPE_MAGIC_V2,
    };

    #[test]
    fn v1_and_malformed_v2_inputs_fail_before_projection() {
        let mut v1 = vec![0_u8; 2_115];
        v1[..8].copy_from_slice(&WORKER_V3_LOAD_ENVELOPE_MAGIC_V1);
        assert!(matches!(
            decode_m1_swiglu_pending_request_v2(&v1),
            Err(M1SwiGluV2ProjectionErrorV1::Envelope(
                WorkerV3LoadEnvelopeErrorV2::WireLengthOutOfRange {
                    actual: 2_115,
                    minimum,
                    ..
                }
            )) if minimum > 2_115
        ));

        let current_v2_minimum = 24 + COMPILER_EXECUTION_RECEIPT_CARRIAGE_BYTES_V1 + 32 + 1;
        let mut wrong_version = vec![0_u8; current_v2_minimum];
        wrong_version[..8].copy_from_slice(&WORKER_V3_LOAD_ENVELOPE_MAGIC_V2);
        wrong_version[8..10].copy_from_slice(&1_u16.to_le_bytes());
        assert!(matches!(
            decode_m1_swiglu_pending_request_v2(&wrong_version),
            Err(M1SwiGluV2ProjectionErrorV1::Envelope(
                WorkerV3LoadEnvelopeErrorV2::UnsupportedVersion { actual: 1 }
            ))
        ));

        let oversized = vec![0_u8; MAX_WORKER_V3_LOAD_ENVELOPE_BYTES_V2 + 1];
        assert!(matches!(
            decode_m1_swiglu_pending_request_v2(&oversized),
            Err(M1SwiGluV2ProjectionErrorV1::Envelope(
                WorkerV3LoadEnvelopeErrorV2::WireLengthOutOfRange { .. }
            ))
        ));
    }

    fn expected_build_projection() -> ExactBuildProjectionV1 {
        ExactBuildProjectionV1 {
            nested_replay_sha256: EXPECTED_REPLAY_SHA256,
            nested_replay_length: EXPECTED_REPLAY_LENGTH,
            published_claim_sha256: EXPECTED_CLAIM_SHA256,
            published_claim_length: EXPECTED_CLAIM_LENGTH,
            compiler_closure: EXPECTED_COMPILER_CLOSURE,
            record_publication_intent: EXPECTED_PUBLICATION_INTENT,
            binding_publication_intent: EXPECTED_PUBLICATION_INTENT,
            finalization: EXPECTED_FINALIZATION,
            source_evidence: EXPECTED_SOURCE_EVIDENCE,
            binding_compiler_handoff: EXPECTED_COMPILER_HANDOFF,
            record_compiler_handoff: EXPECTED_COMPILER_HANDOFF,
            compiler_handoff_length: EXPECTED_COMPILER_HANDOFF_LENGTH,
            raw_inspection: EXPECTED_RAW_INSPECTION,
            raw_output_sha256: EXPECTED_RAW_OUTPUT,
            raw_output_length: EXPECTED_OUTPUT_LENGTH,
            binding_finalized_output_sha256: EXPECTED_FINALIZED_OUTPUT,
            record_finalized_output_sha256: EXPECTED_FINALIZED_OUTPUT,
            binding_finalized_output_length: EXPECTED_OUTPUT_LENGTH,
            record_finalized_output_length: EXPECTED_OUTPUT_LENGTH,
        }
    }

    #[test]
    fn exact_policy_rejects_every_carried_build_axis() {
        assert!(validate_build_projection(&expected_build_projection()).is_ok());
        let expected_fields = [
            M1SwiGluV2BuildFieldV1::NestedReplaySha256,
            M1SwiGluV2BuildFieldV1::NestedReplayLength,
            M1SwiGluV2BuildFieldV1::PublishedClaimSha256,
            M1SwiGluV2BuildFieldV1::PublishedClaimLength,
            M1SwiGluV2BuildFieldV1::CompilerClosure,
            M1SwiGluV2BuildFieldV1::PublicationIntent,
            M1SwiGluV2BuildFieldV1::PublicationIntent,
            M1SwiGluV2BuildFieldV1::Finalization,
            M1SwiGluV2BuildFieldV1::SourceEvidence,
            M1SwiGluV2BuildFieldV1::CompilerHandoff,
            M1SwiGluV2BuildFieldV1::CompilerHandoff,
            M1SwiGluV2BuildFieldV1::CompilerHandoffLength,
            M1SwiGluV2BuildFieldV1::RawInspection,
            M1SwiGluV2BuildFieldV1::RawOutputSha256,
            M1SwiGluV2BuildFieldV1::RawOutputLength,
            M1SwiGluV2BuildFieldV1::FinalizedOutputSha256,
            M1SwiGluV2BuildFieldV1::FinalizedOutputSha256,
            M1SwiGluV2BuildFieldV1::FinalizedOutputLength,
            M1SwiGluV2BuildFieldV1::FinalizedOutputLength,
        ];
        for (index, expected_field) in expected_fields.into_iter().enumerate() {
            let mut changed = expected_build_projection();
            match index {
                0 => changed.nested_replay_sha256[0] ^= 1,
                1 => changed.nested_replay_length += 1,
                2 => changed.published_claim_sha256[0] ^= 1,
                3 => changed.published_claim_length += 1,
                4 => changed.compiler_closure[0] ^= 1,
                5 => changed.record_publication_intent[0] ^= 1,
                6 => changed.binding_publication_intent[0] ^= 1,
                7 => changed.finalization[0] ^= 1,
                8 => changed.source_evidence[0] ^= 1,
                9 => changed.binding_compiler_handoff[0] ^= 1,
                10 => changed.record_compiler_handoff[0] ^= 1,
                11 => changed.compiler_handoff_length += 1,
                12 => changed.raw_inspection[0] ^= 1,
                13 => changed.raw_output_sha256[0] ^= 1,
                14 => changed.raw_output_length += 1,
                15 => changed.binding_finalized_output_sha256[0] ^= 1,
                16 => changed.record_finalized_output_sha256[0] ^= 1,
                17 => changed.binding_finalized_output_length += 1,
                18 => changed.record_finalized_output_length += 1,
                _ => unreachable!(),
            }
            assert!(matches!(
                validate_build_projection(&changed),
                Err(M1SwiGluV2ProjectionErrorV1::BuildIdentityMismatch(actual))
                    if actual == expected_field
            ));
        }
    }

    #[test]
    fn projection_types_make_no_authority_claim() {
        let projection = M1SwiGluV2CarriageProjectionV1 {
            envelope_sha256: [1; 32],
            envelope_length: 1,
            carriage_identity: [2; 32],
            issuer_policy_identity: [3; 32],
            compiler_subject_identity: [4; 32],
            attestation_request_identity: [5; 32],
            signed_receipt_identity: [6; 32],
            receipt_publication_identity: [7; 32],
            worker_acknowledgment_identity: [8; 32],
            worker_ledger_record_identity: [9; 32],
            sequence: 1,
            next_rollback_anchor: [10; 32],
        };
        assert!(!projection.authenticates_compiler_origin());
        assert!(projection.requires_protected_verifier());
        assert!(!projection.grants_load_authority());
        assert!(!projection.grants_launch_authority());
    }
}
