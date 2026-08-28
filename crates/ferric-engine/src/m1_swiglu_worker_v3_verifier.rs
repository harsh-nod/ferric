//! Ferric-owned production-verifier request contract for the first Worker V3 `SwiGLU` artifact.
//!
//! This module deliberately stops before verification. It binds the exact protected build
//! candidate, all target/draft inference profiles, the physical ABI, and every identity axis of a
//! future receipt-bearing compiler-execution carriage into one request identity. Only a protected
//! verifier may authenticate those inputs and promote them through fe2o3's unsafe verifier boundary.
//! This module does not decode a fe2o3 envelope and is not that future adapter.

use std::{error::Error, fmt};

use ferric_qwen_kernels::swiglu::{
    self, Qwen3SwiGluCatalogErrorV1, Qwen3SwiGluProfileCatalogIdentityV1,
    Qwen3SwiGluProfileCatalogV1,
};
use sha2::{Digest, Sha256};

const VERIFIER_REQUEST_DOMAIN_V1: &[u8] = b"ferric.m1.qwen3-swiglu.worker-v3-verifier-request.v1\0";
const DEVICE_SOURCE_NAMESPACE_V1: &str =
    "945c6bcdb6e275891490d8062a0b53b82f4b9d558ff2792d3fedfdf9c9bc0820";
const COMPILER_COMMIT_V1: &str = "21e4c10609a7b44687153fc3484d1156b4eb4def";
const DEVICE_PROVIDER_COMMIT_V1: &str = "06c74c64506f15883d64c5ab2ca476561909181d";
const ARTIFACT_SHA256_V1: [u8; 32] = [
    0x57, 0xec, 0xb8, 0x6b, 0x40, 0xdb, 0x13, 0x62, 0x37, 0xe6, 0x5a, 0x5f, 0xae, 0x04, 0xc9, 0x55,
    0xf2, 0xc9, 0x2f, 0xe3, 0x34, 0x7c, 0x08, 0x5e, 0xc5, 0xc8, 0x06, 0x98, 0x4f, 0xc6, 0xaf, 0xa7,
];
const DEVICE_SOURCE_SHA256_V1: [u8; 32] = [
    0x8f, 0xbb, 0xd8, 0x46, 0x4e, 0x2f, 0x6b, 0x66, 0xca, 0x43, 0xf2, 0x84, 0xd1, 0xf8, 0x94, 0xeb,
    0xa8, 0x8b, 0xe1, 0xeb, 0x84, 0x91, 0x3b, 0xcd, 0x00, 0x36, 0x0a, 0x6b, 0x7a, 0x20, 0x23, 0x9f,
];
const COMPILER_HANDOFF_SHA256_V1: [u8; 32] = [
    0xde, 0x56, 0x1a, 0x1e, 0xb2, 0xb6, 0x6a, 0x1b, 0x85, 0xb0, 0x5e, 0x6b, 0xda, 0x06, 0xc5, 0xe5,
    0x45, 0xc1, 0x7d, 0x64, 0x2f, 0xd0, 0xaa, 0x23, 0xf0, 0xa2, 0x45, 0x8f, 0xef, 0x53, 0x2b, 0x12,
];
const FINALIZATION_SHA256_V1: [u8; 32] = [
    0x37, 0xaa, 0x96, 0x5a, 0xf2, 0xc7, 0x71, 0xfc, 0xd4, 0xc1, 0x3f, 0x63, 0x56, 0x60, 0xd2, 0x59,
    0x61, 0x50, 0x9d, 0x37, 0xd0, 0xa0, 0x57, 0x2e, 0xfd, 0xb9, 0xec, 0x56, 0x9f, 0x53, 0xf8, 0x96,
];
const PUBLICATION_INTENT_SHA256_V1: [u8; 32] = [
    0x61, 0xdb, 0x6e, 0xf6, 0xf8, 0x0e, 0x89, 0xdc, 0x6a, 0xc5, 0x71, 0xf9, 0x9e, 0xdc, 0x57, 0x28,
    0xed, 0xc0, 0xa3, 0xde, 0xf3, 0xc4, 0xad, 0x1d, 0x11, 0x77, 0x87, 0xd4, 0xef, 0x74, 0x35, 0x65,
];
const NESTED_REPLAY_SHA256_V1: [u8; 32] = [
    0x09, 0x3b, 0x45, 0xda, 0x9d, 0xa3, 0xb6, 0x85, 0x95, 0x53, 0x34, 0x5a, 0xa3, 0x8e, 0x57, 0x89,
    0xaa, 0xd4, 0x94, 0x9b, 0x72, 0x5e, 0x33, 0xe4, 0xe4, 0xd6, 0x62, 0x00, 0x45, 0x45, 0x5e, 0xd1,
];
const PUBLISHED_CLAIM_SHA256_V1: [u8; 32] = [
    0x40, 0x1b, 0x5b, 0x2b, 0x54, 0x19, 0x0e, 0x7b, 0xd0, 0xe0, 0x11, 0x5d, 0xa9, 0xaa, 0x85, 0xb1,
    0x71, 0x87, 0x63, 0x1e, 0x9c, 0x9e, 0xe2, 0x05, 0x7b, 0xf4, 0x65, 0x5c, 0x45, 0x60, 0x83, 0xe0,
];

/// Identity field in the exact protected Worker V3 build candidate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum M1SwiGluProtectedWorkerV3BuildFieldV1 {
    ArtifactSha256,
    ArtifactLength,
    DeviceSourceSha256,
    DeviceSourceLength,
    CompilerCommit,
    DeviceProviderCommit,
    CompilerHandoffSha256,
    FinalizationSha256,
    PublicationIntentSha256,
    NestedReplaySha256,
    PublishedClaimSha256,
}

/// Schema carried by the checked-in protected-build observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum M1SwiGluCurrentEnvelopeSchemaV1 {
    /// Authority-free V1 replay and load-readiness receipt; no compiler-execution carriage.
    ReplayV1WithoutCompilerExecutionCarriage,
}

/// Exact identities of the protected Worker V3 artifact currently qualified by Ferric.
///
/// This is descriptive build evidence. It is not a verifier result and grants no load or dispatch
/// authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1SwiGluProtectedBuildIdentitiesV1 {
    artifact_sha256: [u8; 32],
    artifact_length: u64,
    device_source_sha256: [u8; 32],
    device_source_length: u64,
    compiler_commit: &'static str,
    device_provider_commit: &'static str,
    compiler_handoff_sha256: [u8; 32],
    finalization_sha256: [u8; 32],
    publication_intent_sha256: [u8; 32],
    nested_replay_sha256: [u8; 32],
    published_claim_sha256: [u8; 32],
}

impl M1SwiGluProtectedBuildIdentitiesV1 {
    /// Constructs an untrusted observation for comparison with Ferric's exact protected-build
    /// policy. This operation grants no authority.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub const fn from_untrusted_observation(
        artifact_sha256: [u8; 32],
        artifact_length: u64,
        device_source_sha256: [u8; 32],
        device_source_length: u64,
        compiler_commit: &'static str,
        device_provider_commit: &'static str,
        compiler_handoff_sha256: [u8; 32],
        finalization_sha256: [u8; 32],
        publication_intent_sha256: [u8; 32],
        nested_replay_sha256: [u8; 32],
        published_claim_sha256: [u8; 32],
    ) -> Self {
        Self {
            artifact_sha256,
            artifact_length,
            device_source_sha256,
            device_source_length,
            compiler_commit,
            device_provider_commit,
            compiler_handoff_sha256,
            finalization_sha256,
            publication_intent_sha256,
            nested_replay_sha256,
            published_claim_sha256,
        }
    }

    /// Exact finalized HSACO digest.
    #[must_use]
    pub const fn artifact_sha256(self) -> [u8; 32] {
        self.artifact_sha256
    }

    /// Exact finalized HSACO byte length.
    #[must_use]
    pub const fn artifact_length(self) -> u64 {
        self.artifact_length
    }

    /// Exact attributed Rust device-source digest.
    #[must_use]
    pub const fn device_source_sha256(self) -> [u8; 32] {
        self.device_source_sha256
    }

    /// Exact attributed Rust device-source byte length.
    #[must_use]
    pub const fn device_source_length(self) -> u64 {
        self.device_source_length
    }

    /// The current observation is not a receipt-bearing V2 envelope.
    #[must_use]
    pub const fn envelope_schema(self) -> M1SwiGluCurrentEnvelopeSchemaV1 {
        M1SwiGluCurrentEnvelopeSchemaV1::ReplayV1WithoutCompilerExecutionCarriage
    }

    /// Protected build identities alone do not authenticate compiler origin.
    #[must_use]
    pub const fn authenticates_compiler_origin(self) -> bool {
        false
    }

    /// Protected build identities alone grant no load authority.
    #[must_use]
    pub const fn grants_load_authority(self) -> bool {
        false
    }

    /// Protected build identities alone grant no launch authority.
    #[must_use]
    pub const fn grants_launch_authority(self) -> bool {
        false
    }
}

/// Returns the exact build axes recorded by `PROTECTED_WORKER_V3_SWIGLU_BUILD.json`.
#[must_use]
pub fn current_m1_swiglu_worker_v3_build_v1() -> M1SwiGluProtectedBuildIdentitiesV1 {
    M1SwiGluProtectedBuildIdentitiesV1::from_untrusted_observation(
        ARTIFACT_SHA256_V1,
        14_192,
        DEVICE_SOURCE_SHA256_V1,
        7_961,
        COMPILER_COMMIT_V1,
        DEVICE_PROVIDER_COMMIT_V1,
        COMPILER_HANDOFF_SHA256_V1,
        FINALIZATION_SHA256_V1,
        PUBLICATION_INTENT_SHA256_V1,
        NESTED_REPLAY_SHA256_V1,
        PUBLISHED_CLAIM_SHA256_V1,
    )
}

/// Complete identity projection of a strictly decoded compiler-execution carriage.
///
/// The future fe2o3 adapter must obtain every value from one strictly decoded V2 envelope; a
/// caller-authored instance is untrusted and cannot confer verifier authority. Ferric deliberately
/// imposes no extra zero/digest-distinctness rule beyond fe2o3's canonical protocol.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1SwiGluCompilerReceiptCarriageIdentitiesV1 {
    envelope_sha256: [u8; 32],
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

impl M1SwiGluCompilerReceiptCarriageIdentitiesV1 {
    /// Constructs an untrusted, authority-free projection of one complete V2 carriage.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub const fn from_untrusted_observation(
        envelope_sha256: [u8; 32],
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
    ) -> Self {
        Self {
            envelope_sha256,
            carriage_identity,
            issuer_policy_identity,
            compiler_subject_identity,
            attestation_request_identity,
            signed_receipt_identity,
            receipt_publication_identity,
            worker_acknowledgment_identity,
            worker_ledger_record_identity,
            sequence,
            next_rollback_anchor,
        }
    }

    fn identities(&self) -> [[u8; 32]; 10] {
        [
            self.envelope_sha256,
            self.carriage_identity,
            self.issuer_policy_identity,
            self.compiler_subject_identity,
            self.attestation_request_identity,
            self.signed_receipt_identity,
            self.receipt_publication_identity,
            self.worker_acknowledgment_identity,
            self.worker_ledger_record_identity,
            self.next_rollback_anchor,
        ]
    }

    /// Complete carriage identity, still without compiler authority.
    #[must_use]
    pub const fn carriage_identity(self) -> [u8; 32] {
        self.carriage_identity
    }

    /// Exact compiler subject joined by the receipt-bearing V2 envelope.
    #[must_use]
    pub const fn compiler_subject_identity(self) -> [u8; 32] {
        self.compiler_subject_identity
    }

    /// Durable Worker receipt sequence to be checked against protected monotonic state.
    #[must_use]
    pub const fn sequence(self) -> u64 {
        self.sequence
    }

    /// Carriage identities alone do not establish rollback currentness.
    #[must_use]
    pub const fn grants_compiler_authority(self) -> bool {
        false
    }
}

/// Pending, authority-free input for Ferric's future protected `SwiGLU` verifier.
///
/// The identity covers the exact protected build, complete Qwen target/draft profile catalog,
/// kernel ABI and launch constants, and every V2 receipt-carriage identity axis. This value cannot
/// be converted to fe2o3 load or KFD authority.
pub struct M1SwiGluProtectedVerifierRequestV1 {
    identity: [u8; 32],
    profile_catalog_identity: Qwen3SwiGluProfileCatalogIdentityV1,
    artifact_sha256: [u8; 32],
    carriage: M1SwiGluCompilerReceiptCarriageIdentitiesV1,
}

impl fmt::Debug for M1SwiGluProtectedVerifierRequestV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1SwiGluProtectedVerifierRequestV1")
            .field("identity", &self.identity)
            .field("artifact_sha256", &self.artifact_sha256)
            .field("carriage_identity", &self.carriage.carriage_identity)
            .field("authority", &"none")
            .finish_non_exhaustive()
    }
}

impl M1SwiGluProtectedVerifierRequestV1 {
    /// Domain-separated identity of the complete Ferric verifier request.
    #[must_use]
    pub const fn identity(&self) -> [u8; 32] {
        self.identity
    }

    /// Exact 22-profile target/draft inference catalog identity.
    #[must_use]
    pub const fn profile_catalog_identity(&self) -> Qwen3SwiGluProfileCatalogIdentityV1 {
        self.profile_catalog_identity
    }

    /// Exact protected artifact digest.
    #[must_use]
    pub const fn artifact_sha256(&self) -> [u8; 32] {
        self.artifact_sha256
    }

    /// Complete authority-free compiler-carriage identity.
    #[must_use]
    pub const fn carriage_identity(&self) -> [u8; 32] {
        self.carriage.carriage_identity
    }

    /// A protected verifier must compare policy and enforce external rollback currentness.
    #[must_use]
    pub const fn requires_protected_verifier(&self) -> bool {
        true
    }

    /// Request binding alone does not authenticate compiler origin.
    #[must_use]
    pub const fn authenticates_compiler_origin(&self) -> bool {
        false
    }

    /// Request binding alone grants no load authority.
    #[must_use]
    pub const fn grants_load_authority(&self) -> bool {
        false
    }

    /// Request binding alone grants no launch authority.
    #[must_use]
    pub const fn grants_launch_authority(&self) -> bool {
        false
    }
}

/// Failure while binding a Ferric production-verifier request.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum M1SwiGluProtectedVerifierRequestErrorV1 {
    /// The current evidence has only a V1 replay/load-readiness receipt.
    ReceiptBearingEnvelopeV2Required {
        current: M1SwiGluCurrentEnvelopeSchemaV1,
    },
    /// One exact protected-build identity axis drifted.
    BuildIdentityMismatch(M1SwiGluProtectedWorkerV3BuildFieldV1),
    /// The finite Qwen inference profile catalog could not be reconstructed.
    ProfileCatalog(Qwen3SwiGluCatalogErrorV1),
}

impl fmt::Display for M1SwiGluProtectedVerifierRequestErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReceiptBearingEnvelopeV2Required { current } => write!(
                formatter,
                "receipt-bearing Worker V3 envelope V2 is required; current evidence is {current:?}"
            ),
            Self::BuildIdentityMismatch(field) => {
                write!(
                    formatter,
                    "protected Worker V3 build identity drifted: {field:?}"
                )
            }
            Self::ProfileCatalog(error) => {
                write!(formatter, "SwiGLU profile binding failed: {error}")
            }
        }
    }
}

impl Error for M1SwiGluProtectedVerifierRequestErrorV1 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ProfileCatalog(error) => Some(error),
            _ => None,
        }
    }
}

impl From<Qwen3SwiGluCatalogErrorV1> for M1SwiGluProtectedVerifierRequestErrorV1 {
    fn from(error: Qwen3SwiGluCatalogErrorV1) -> Self {
        Self::ProfileCatalog(error)
    }
}

/// Fails closed for the checked-in current artifact because it has no compiler-execution carriage.
///
/// # Errors
///
/// Always returns [`M1SwiGluProtectedVerifierRequestErrorV1::ReceiptBearingEnvelopeV2Required`]
/// until a fresh protected build publishes and recovers the mandatory V2 envelope.
pub const fn require_current_m1_swiglu_receipt_bearing_envelope_v2(
) -> Result<(), M1SwiGluProtectedVerifierRequestErrorV1> {
    Err(
        M1SwiGluProtectedVerifierRequestErrorV1::ReceiptBearingEnvelopeV2Required {
            current: M1SwiGluCurrentEnvelopeSchemaV1::ReplayV1WithoutCompilerExecutionCarriage,
        },
    )
}

/// Binds exact build, inference, ABI, launch, and V2 carriage identities for protected review.
///
/// This accepts an untrusted identity projection. It neither decodes nor authenticates a fe2o3
/// carriage and must not be used as the future protected-envelope adapter.
///
/// # Errors
///
/// Returns [`M1SwiGluProtectedVerifierRequestErrorV1`] if any protected-build field drifts, the
/// finite 22-profile catalog cannot be reconstructed.
pub fn prepare_m1_swiglu_protected_verifier_request_v1(
    build: M1SwiGluProtectedBuildIdentitiesV1,
    carriage: M1SwiGluCompilerReceiptCarriageIdentitiesV1,
) -> Result<M1SwiGluProtectedVerifierRequestV1, M1SwiGluProtectedVerifierRequestErrorV1> {
    validate_build(&build)?;
    let catalog = Qwen3SwiGluProfileCatalogV1::canonical()?;
    let identity = verifier_request_identity(&build, &catalog, &carriage);
    Ok(M1SwiGluProtectedVerifierRequestV1 {
        identity,
        profile_catalog_identity: catalog.identity(),
        artifact_sha256: build.artifact_sha256,
        carriage,
    })
}

fn validate_build(
    actual: &M1SwiGluProtectedBuildIdentitiesV1,
) -> Result<(), M1SwiGluProtectedVerifierRequestErrorV1> {
    let expected = current_m1_swiglu_worker_v3_build_v1();
    for (matches, field) in [
        (
            actual.artifact_sha256 == expected.artifact_sha256,
            M1SwiGluProtectedWorkerV3BuildFieldV1::ArtifactSha256,
        ),
        (
            actual.artifact_length == expected.artifact_length,
            M1SwiGluProtectedWorkerV3BuildFieldV1::ArtifactLength,
        ),
        (
            actual.device_source_sha256 == expected.device_source_sha256,
            M1SwiGluProtectedWorkerV3BuildFieldV1::DeviceSourceSha256,
        ),
        (
            actual.device_source_length == expected.device_source_length,
            M1SwiGluProtectedWorkerV3BuildFieldV1::DeviceSourceLength,
        ),
        (
            actual.compiler_commit == expected.compiler_commit,
            M1SwiGluProtectedWorkerV3BuildFieldV1::CompilerCommit,
        ),
        (
            actual.device_provider_commit == expected.device_provider_commit,
            M1SwiGluProtectedWorkerV3BuildFieldV1::DeviceProviderCommit,
        ),
        (
            actual.compiler_handoff_sha256 == expected.compiler_handoff_sha256,
            M1SwiGluProtectedWorkerV3BuildFieldV1::CompilerHandoffSha256,
        ),
        (
            actual.finalization_sha256 == expected.finalization_sha256,
            M1SwiGluProtectedWorkerV3BuildFieldV1::FinalizationSha256,
        ),
        (
            actual.publication_intent_sha256 == expected.publication_intent_sha256,
            M1SwiGluProtectedWorkerV3BuildFieldV1::PublicationIntentSha256,
        ),
        (
            actual.nested_replay_sha256 == expected.nested_replay_sha256,
            M1SwiGluProtectedWorkerV3BuildFieldV1::NestedReplaySha256,
        ),
        (
            actual.published_claim_sha256 == expected.published_claim_sha256,
            M1SwiGluProtectedWorkerV3BuildFieldV1::PublishedClaimSha256,
        ),
    ] {
        if !matches {
            return Err(M1SwiGluProtectedVerifierRequestErrorV1::BuildIdentityMismatch(field));
        }
    }
    Ok(())
}

fn verifier_request_identity(
    build: &M1SwiGluProtectedBuildIdentitiesV1,
    catalog: &Qwen3SwiGluProfileCatalogV1,
    carriage: &M1SwiGluCompilerReceiptCarriageIdentitiesV1,
) -> [u8; 32] {
    let mut digest = Sha256::new();
    put_bytes(&mut digest, VERIFIER_REQUEST_DOMAIN_V1);
    put_bytes(
        &mut digest,
        swiglu::QWEN3_SWIGLU_KERNEL_SYMBOL_V1.as_bytes(),
    );
    put_bytes(
        &mut digest,
        swiglu::QWEN3_SWIGLU_KERNEL_DESCRIPTOR_SYMBOL_V1.as_bytes(),
    );
    put_bytes(&mut digest, swiglu::QWEN3_SWIGLU_TARGET_V1.as_bytes());
    digest.update([swiglu::QWEN3_SWIGLU_CODE_OBJECT_VERSION_V1]);
    for dimension in swiglu::QWEN3_SWIGLU_WORKGROUP_V1 {
        digest.update(dimension.to_le_bytes());
    }
    digest.update(swiglu::QWEN3_SWIGLU_ELEMENTS_PER_WORKITEM_V1.to_le_bytes());
    digest.update(swiglu::QWEN3_SWIGLU_ELEMENTS_PER_WORKGROUP_V1.to_le_bytes());
    digest.update(swiglu::QWEN3_SWIGLU_EXPLICIT_KERNARG_BYTES_V1.to_le_bytes());
    digest.update(swiglu::QWEN3_SWIGLU_TOTAL_KERNARG_BYTES_V1.to_le_bytes());
    digest.update(swiglu::QWEN3_SWIGLU_KERNARG_ALIGNMENT_V1.to_le_bytes());
    put_bytes(&mut digest, DEVICE_SOURCE_NAMESPACE_V1.as_bytes());
    for argument in swiglu::QWEN3_SWIGLU_GLOBAL_BUFFER_ABI_V1 {
        digest.update((argument.explicit_argument_index() as u64).to_le_bytes());
        put_bytes(&mut digest, argument.name().as_bytes());
        digest.update(argument.offset().to_le_bytes());
        digest.update(argument.pointee_alignment().to_le_bytes());
        digest.update([argument.access() as u8]);
    }
    put_bytes(&mut digest, catalog.canonical_bytes());
    digest.update(catalog.identity().as_bytes());
    digest.update(build.artifact_sha256);
    digest.update(build.artifact_length.to_le_bytes());
    digest.update(build.device_source_sha256);
    digest.update(build.device_source_length.to_le_bytes());
    put_bytes(&mut digest, build.compiler_commit.as_bytes());
    put_bytes(&mut digest, build.device_provider_commit.as_bytes());
    digest.update(build.compiler_handoff_sha256);
    digest.update(build.finalization_sha256);
    digest.update(build.publication_intent_sha256);
    digest.update(build.nested_replay_sha256);
    digest.update(build.published_claim_sha256);
    for identity in carriage.identities() {
        digest.update(identity);
    }
    digest.update(carriage.sequence.to_le_bytes());
    digest.finalize().into()
}

fn put_bytes(digest: &mut Sha256, bytes: &[u8]) {
    digest.update((bytes.len() as u64).to_le_bytes());
    digest.update(bytes);
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUILD_RECORD: &str =
        include_str!("../../../proofs/m1/evidence/PROTECTED_WORKER_V3_SWIGLU_BUILD.json");
    const DEVICE_SOURCE: &[u8] = include_bytes!("../../../device/qwen3-swiglu-v1/src/lib.rs");

    fn carriage(seed: u8) -> M1SwiGluCompilerReceiptCarriageIdentitiesV1 {
        M1SwiGluCompilerReceiptCarriageIdentitiesV1::from_untrusted_observation(
            [seed; 32],
            [seed.wrapping_add(1); 32],
            [seed.wrapping_add(2); 32],
            [seed.wrapping_add(3); 32],
            [seed.wrapping_add(4); 32],
            [seed.wrapping_add(5); 32],
            [seed.wrapping_add(6); 32],
            [seed.wrapping_add(7); 32],
            [seed.wrapping_add(8); 32],
            17,
            [seed.wrapping_add(9); 32],
        )
    }

    #[test]
    fn current_v1_evidence_is_explicitly_fail_closed() {
        assert_eq!(
            require_current_m1_swiglu_receipt_bearing_envelope_v2(),
            Err(
                M1SwiGluProtectedVerifierRequestErrorV1::ReceiptBearingEnvelopeV2Required {
                    current:
                        M1SwiGluCurrentEnvelopeSchemaV1::ReplayV1WithoutCompilerExecutionCarriage,
                }
            )
        );
        let build = current_m1_swiglu_worker_v3_build_v1();
        assert_eq!(
            build.envelope_schema(),
            M1SwiGluCurrentEnvelopeSchemaV1::ReplayV1WithoutCompilerExecutionCarriage
        );
        assert!(!build.authenticates_compiler_origin());
        assert!(!build.grants_load_authority());
        assert!(!build.grants_launch_authority());
    }

    #[test]
    fn frozen_build_policy_matches_checked_in_record_and_device_source() {
        let record: serde_json::Value = serde_json::from_str(BUILD_RECORD).unwrap();
        let build = current_m1_swiglu_worker_v3_build_v1();
        assert_eq!(hex(build.artifact_sha256), record["artifact"]["sha256"]);
        assert_eq!(build.artifact_length, record["artifact"]["size_bytes"]);
        assert_eq!(build.compiler_commit, record["compiler"]["commit"]);
        assert_eq!(
            build.device_provider_commit,
            record["source"]["device_provider_commit"]
        );
        assert_eq!(
            hex(build.compiler_handoff_sha256),
            record["publication"]["worker_v3_binding"]["compiler_handoff_sha256"]
        );
        assert_eq!(
            hex(build.finalization_sha256),
            record["publication"]["worker_v3_binding"]["finalization_sha256"]
        );
        assert_eq!(
            hex(build.publication_intent_sha256),
            record["publication"]["worker_v3_binding"]["publication_intent_sha256"]
        );

        let custody = record["custody_records"].as_array().unwrap();
        let envelope = custody
            .iter()
            .find(|entry| entry["kind"] == "envelope")
            .unwrap();
        let claim = custody
            .iter()
            .find(|entry| entry["kind"] == "claim")
            .unwrap();
        assert_eq!(hex(build.nested_replay_sha256), envelope["sha256"]);
        assert_eq!(hex(build.published_claim_sha256), claim["sha256"]);

        let source = record["source"]["device_files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["path"] == "device/qwen3-swiglu-v1/src/lib.rs")
            .unwrap();
        assert_eq!(hex(build.device_source_sha256), source["sha256"]);
        assert_eq!(build.device_source_length, source["size_bytes"]);
        assert_eq!(DEVICE_SOURCE.len() as u64, build.device_source_length);
        assert_eq!(
            <[u8; 32]>::from(Sha256::digest(DEVICE_SOURCE)),
            build.device_source_sha256
        );
        let source_text = std::str::from_utf8(DEVICE_SOURCE).unwrap();
        assert!(source_text.contains(DEVICE_SOURCE_NAMESPACE_V1));
        assert!(source_text.contains(swiglu::QWEN3_SWIGLU_KERNEL_SYMBOL_V1));

        assert_eq!(
            record["inspection"]["target"],
            swiglu::QWEN3_SWIGLU_TARGET_V1
        );
        assert_eq!(
            record["inspection"]["kernel"]["name"],
            swiglu::QWEN3_SWIGLU_KERNEL_SYMBOL_V1
        );
        assert_eq!(
            record["inspection"]["kernel"]["symbol"],
            swiglu::QWEN3_SWIGLU_KERNEL_DESCRIPTOR_SYMBOL_V1
        );
        assert_eq!(
            record["inspection"]["kernel"]["kernarg_size_bytes"],
            swiglu::QWEN3_SWIGLU_TOTAL_KERNARG_BYTES_V1
        );
        assert_eq!(
            record["inspection"]["kernel"]["kernarg_alignment_bytes"],
            swiglu::QWEN3_SWIGLU_KERNARG_ALIGNMENT_V1
        );
    }

    #[test]
    fn request_binds_exact_build_inference_abi_and_carriage_without_granting_authority() {
        let request = prepare_m1_swiglu_protected_verifier_request_v1(
            current_m1_swiglu_worker_v3_build_v1(),
            carriage(0x20),
        )
        .unwrap();
        let catalog = Qwen3SwiGluProfileCatalogV1::canonical().unwrap();
        assert_eq!(request.profile_catalog_identity(), catalog.identity());
        assert_eq!(catalog.profiles().len(), 22);
        assert_eq!(request.carriage_identity(), [0x21; 32]);
        assert_ne!(request.identity(), [0; 32]);
        assert!(request.requires_protected_verifier());
        assert!(!request.authenticates_compiler_origin());
        assert!(!request.grants_load_authority());
        assert!(!request.grants_launch_authority());
    }

    #[test]
    fn every_protected_build_axis_is_checked() {
        let expected = [
            M1SwiGluProtectedWorkerV3BuildFieldV1::ArtifactSha256,
            M1SwiGluProtectedWorkerV3BuildFieldV1::ArtifactLength,
            M1SwiGluProtectedWorkerV3BuildFieldV1::DeviceSourceSha256,
            M1SwiGluProtectedWorkerV3BuildFieldV1::DeviceSourceLength,
            M1SwiGluProtectedWorkerV3BuildFieldV1::CompilerCommit,
            M1SwiGluProtectedWorkerV3BuildFieldV1::DeviceProviderCommit,
            M1SwiGluProtectedWorkerV3BuildFieldV1::CompilerHandoffSha256,
            M1SwiGluProtectedWorkerV3BuildFieldV1::FinalizationSha256,
            M1SwiGluProtectedWorkerV3BuildFieldV1::PublicationIntentSha256,
            M1SwiGluProtectedWorkerV3BuildFieldV1::NestedReplaySha256,
            M1SwiGluProtectedWorkerV3BuildFieldV1::PublishedClaimSha256,
        ];
        for (index, field) in expected.into_iter().enumerate() {
            let mut changed = current_m1_swiglu_worker_v3_build_v1();
            match index {
                0 => changed.artifact_sha256[0] ^= 1,
                1 => changed.artifact_length += 1,
                2 => changed.device_source_sha256[0] ^= 1,
                3 => changed.device_source_length += 1,
                4 => changed.compiler_commit = DEVICE_PROVIDER_COMMIT_V1,
                5 => changed.device_provider_commit = COMPILER_COMMIT_V1,
                6 => changed.compiler_handoff_sha256[0] ^= 1,
                7 => changed.finalization_sha256[0] ^= 1,
                8 => changed.publication_intent_sha256[0] ^= 1,
                9 => changed.nested_replay_sha256[0] ^= 1,
                10 => changed.published_claim_sha256[0] ^= 1,
                _ => unreachable!(),
            }
            assert_eq!(
                prepare_m1_swiglu_protected_verifier_request_v1(changed, carriage(0x20))
                    .unwrap_err(),
                M1SwiGluProtectedVerifierRequestErrorV1::BuildIdentityMismatch(field)
            );
        }
    }

    #[test]
    fn request_identity_changes_for_every_carriage_axis_and_sequence() {
        let build = current_m1_swiglu_worker_v3_build_v1();
        let baseline = prepare_m1_swiglu_protected_verifier_request_v1(build, carriage(0x20))
            .unwrap()
            .identity();
        for index in 0..=10 {
            let mut changed = carriage(0x20);
            match index {
                0 => changed.envelope_sha256[0] ^= 0x80,
                1 => changed.carriage_identity[0] ^= 0x80,
                2 => changed.issuer_policy_identity[0] ^= 0x80,
                3 => changed.compiler_subject_identity[0] ^= 0x80,
                4 => changed.attestation_request_identity[0] ^= 0x80,
                5 => changed.signed_receipt_identity[0] ^= 0x80,
                6 => changed.receipt_publication_identity[0] ^= 0x80,
                7 => changed.worker_acknowledgment_identity[0] ^= 0x80,
                8 => changed.worker_ledger_record_identity[0] ^= 0x80,
                9 => changed.next_rollback_anchor[0] ^= 0x80,
                10 => changed.sequence += 1,
                _ => unreachable!(),
            }
            assert_ne!(
                prepare_m1_swiglu_protected_verifier_request_v1(build, changed)
                    .unwrap()
                    .identity(),
                baseline
            );
        }
    }

    fn hex(bytes: [u8; 32]) -> String {
        use std::fmt::Write as _;

        let mut output = String::with_capacity(64);
        for byte in bytes {
            write!(&mut output, "{byte:02x}").unwrap();
        }
        output
    }
}
