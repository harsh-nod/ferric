//! Canonical commitment for a byte-authenticated M1 deployment.
//!
//! This module consumes the sealed tokenizer and prepacked-weight path. The
//! resulting authority retains those manifests and binds them to one fixed
//! canonical record. It deliberately does not implement signatures or replace
//! the independent validator required by the M1 assurance contract.

use super::{
    decode_canonical_deployment_bundle, encode_canonical_deployment_bundle,
    safetensors::classify_tensor_name, sha256, CanonicalBundleError, PrepackedDeploymentBundle,
    WeightSectionManifest, WeightTransform, CANONICAL_DEPLOYMENT_BUNDLE_BYTES,
    PREPACKED_WEIGHT_MANIFEST_VERSION,
};
use ferric_spec::{DeploymentBundle, Identity, Qwen3ModelRole, Qwen3TensorMetadata, TensorDType};
use std::collections::BTreeSet;
use std::fmt;
use vstd::bytes::{u32_to_le_bytes, u64_to_le_bytes};
use vstd::prelude::*;

verus! {

const MAGIC: [u8; 16] = [70, 69, 82, 82, 73, 67, 45, 77, 49, 45, 65, 68, 77, 73, 84, 0];
const RECORD_DOMAIN: [u8; 41] = [
    102, 101, 114, 114, 105, 99, 46, 97, 117, 116, 104, 101, 110, 116, 105, 99,
    97, 116, 101, 100, 45, 98, 117, 110, 100, 108, 101, 45, 97, 100, 109, 105,
    115, 115, 105, 111, 110, 46, 118, 49, 0,
];
const ROLE_TARGET: u8 = 1;
const ROLE_DRAFT: u8 = 2;
/// Exact byte length of one manifest commitment within an admission record.
pub const MANIFEST_COMMITMENT_BYTES: usize = 101;
const MAX_MANIFEST_RECORD_BYTES: usize = 256 * 1_024;
const MAX_MANIFEST_RECORD_BYTES_U32: u32 = 256 * 1_024;

/// Version of the authenticated deployment commitment.
pub const BUNDLE_ADMISSION_RECORD_VERSION: u32 = 1;
/// Exact byte length of the authenticated deployment commitment.
pub const BUNDLE_ADMISSION_RECORD_BYTES: usize =
    16 + 4 + CANONICAL_DEPLOYMENT_BUNDLE_BYTES + 202;

/// Exact commitment to one retained prepacked-weight manifest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ManifestCommitment {
    /// Model role carried by the manifest.
    pub role: Qwen3ModelRole,
    /// Prepacked manifest format version.
    pub version: u32,
    /// Exact authenticated source weight-set identity.
    pub source_weights_id: [u8; 32],
    /// SHA-256 of the complete canonical manifest record.
    pub aggregate_id: [u8; 32],
    /// Complete source artifact bytes, including safetensors headers.
    pub source_artifact_bytes: u64,
    /// Source tensor bytes, excluding safetensors headers.
    pub tensor_data_bytes: u64,
    /// Exact emitted prepacked byte count.
    pub output_bytes: u64,
    /// Complete tensor-section count.
    pub section_count: u32,
    /// Length of the separately retained canonical manifest record.
    pub canonical_manifest_bytes: u32,
}

/// Decoded canonical record descriptor.
///
/// This is data, not authentication authority. Only
/// [`AuthenticatedBundleAdmission`] retains the sealed byte-backed inputs.
#[verifier::allow(autoderive_clone_without_spec)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleAdmissionDescriptor {
    /// Exact decoded deployment contract.
    pub deployment: DeploymentBundle,
    /// Exact target prepack commitment.
    pub target_manifest: ManifestCommitment,
    /// Exact draft prepack commitment.
    pub draft_manifest: ManifestCommitment,
    /// Domain-separated SHA-256 of the complete canonical record.
    pub record_id: Identity,
}

/// Fixed canonical commitment to an authenticated deployment.
#[verifier::allow(autoderive_clone_without_spec)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleAdmissionRecord {
    bytes: [u8; BUNDLE_ADMISSION_RECORD_BYTES],
    record_id: Identity,
}

impl BundleAdmissionRecord {
    /// Verifier view of the complete fixed-width record.
    pub closed spec fn bytes_spec(&self) -> Seq<u8> {
        self.bytes@
    }

    /// Verifier view of the domain-separated record identity.
    pub closed spec fn record_id_spec(&self) -> Identity {
        self.record_id
    }

    /// Returns the complete canonical bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> (bytes: &[u8; BUNDLE_ADMISSION_RECORD_BYTES])
        ensures bytes@ == self.bytes_spec(),
    {
        &self.bytes
    }

    /// Returns the domain-separated record identity.
    #[must_use]
    pub const fn record_id(&self) -> (identity: Identity)
        ensures identity == self.record_id_spec(),
    {
        self.record_id
    }
}

/// Non-clone authority retaining the exact authenticated deployment inputs.
#[derive(Debug, PartialEq, Eq)]
pub struct AuthenticatedBundleAdmission {
    prepacked: PrepackedDeploymentBundle,
    record: BundleAdmissionRecord,
}

impl AuthenticatedBundleAdmission {
    pub(crate) closed spec fn prepacked_spec(&self) -> PrepackedDeploymentBundle {
        self.prepacked
    }

    /// Verifier view of the exact retained deployment.
    pub closed spec fn deployment_spec(&self) -> DeploymentBundle {
        self.prepacked.deployment_spec()
    }

    /// Verifier view of the exact retained target manifest.
    pub closed spec fn target_manifest_spec(&self) -> WeightSectionManifest {
        self.prepacked.target_manifest_spec()
    }

    /// Verifier view of the exact retained draft manifest.
    pub closed spec fn draft_manifest_spec(&self) -> WeightSectionManifest {
        self.prepacked.draft_manifest_spec()
    }

    /// Verifier view of the exact canonical commitment record.
    pub closed spec fn record_spec(&self) -> BundleAdmissionRecord {
        self.record
    }

    /// Returns the retained exact deployment and prepacked manifests.
    #[must_use]
    pub const fn prepacked(&self) -> &PrepackedDeploymentBundle {
        self.prepacked_exact()
    }

    pub(crate) const fn prepacked_exact(&self) -> (prepacked: &PrepackedDeploymentBundle)
        ensures *prepacked == self.prepacked_spec(),
    {
        &self.prepacked
    }

    /// Returns the canonical commitment record.
    #[must_use]
    pub const fn record(&self) -> (record: &BundleAdmissionRecord)
        ensures *record == self.record_spec(),
    {
        &self.record
    }

    pub(crate) fn into_parts(
        self,
    ) -> (parts: (PrepackedDeploymentBundle, BundleAdmissionRecord))
        ensures parts.0 == self.prepacked_spec(), parts.1 == self.record_spec(),
    {
        (self.prepacked, self.record)
    }
}

/// Failure while sealing or decoding an authenticated bundle commitment.
#[verifier::allow(autoderive_clone_without_spec)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BundleAdmissionError {
    /// The fixed record length is wrong.
    InvalidLength,
    /// The fixed record discriminator is wrong.
    InvalidMagic,
    /// The record format version is unsupported.
    InvalidVersion,
    /// The embedded canonical deployment is invalid.
    CanonicalBundle(CanonicalBundleError),
    /// A retained manifest does not match the exact deployment.
    InvalidManifest {
        /// Manifest role under validation.
        role: Qwen3ModelRole,
        /// Stable fail-closed reason.
        reason: &'static str,
    },
    /// A decoded scalar or identity is not canonical for the selected role.
    InvalidCommitment {
        /// Commitment role under validation.
        role: Qwen3ModelRole,
        /// Stable fail-closed reason.
        reason: &'static str,
    },
}

closed spec fn u32_little_endian(value: u32) -> Seq<u8> {
    vstd::bytes::spec_u32_to_le_bytes(value)
}

closed spec fn u64_little_endian(value: u64) -> Seq<u8> {
    vstd::bytes::spec_u64_to_le_bytes(value)
}

closed spec fn u32_at(bytes: Seq<u8>, offset: int) -> u32
    recommends 0 <= offset, offset + 4 <= bytes.len(),
{
    (bytes[offset] as u32)
        | ((bytes[offset + 1] as u32) << 8)
        | ((bytes[offset + 2] as u32) << 16)
        | ((bytes[offset + 3] as u32) << 24)
}

closed spec fn u64_at(bytes: Seq<u8>, offset: int) -> u64
    recommends 0 <= offset, offset + 8 <= bytes.len(),
{
    (bytes[offset] as u64)
        | ((bytes[offset + 1] as u64) << 8)
        | ((bytes[offset + 2] as u64) << 16)
        | ((bytes[offset + 3] as u64) << 24)
        | ((bytes[offset + 4] as u64) << 32)
        | ((bytes[offset + 5] as u64) << 40)
        | ((bytes[offset + 6] as u64) << 48)
        | ((bytes[offset + 7] as u64) << 56)
}

closed spec fn role_wire(role: Qwen3ModelRole) -> Seq<u8> {
    seq![match role {
        Qwen3ModelRole::Target8B => ROLE_TARGET,
        Qwen3ModelRole::Draft06B => ROLE_DRAFT,
    }]
}

closed spec fn role_tensor_data_bytes(role: Qwen3ModelRole) -> u64 {
    match role {
        Qwen3ModelRole::Target8B => super::QWEN3_TARGET_TENSOR_DATA_BYTES,
        Qwen3ModelRole::Draft06B => super::QWEN3_DRAFT_TENSOR_DATA_BYTES,
    }
}

closed spec fn role_tensor_count(role: Qwen3ModelRole) -> u32 {
    match role {
        Qwen3ModelRole::Target8B => 399,
        Qwen3ModelRole::Draft06B => 311,
    }
}

/// Complete verifier-visible wire image for one manifest descriptor.
pub closed spec fn manifest_commitment_wire(value: ManifestCommitment) -> Seq<u8> {
    role_wire(value.role)
        + u32_little_endian(value.version)
        + value.source_weights_id@
        + value.aggregate_id@
        + u64_little_endian(value.source_artifact_bytes)
        + u64_little_endian(value.tensor_data_bytes)
        + u64_little_endian(value.output_bytes)
        + u32_little_endian(value.section_count)
        + u32_little_endian(value.canonical_manifest_bytes)
}

/// Exact structural commitment relation enforced by the descriptor decoder.
pub closed spec fn manifest_commitment_spec(
    deployment: DeploymentBundle,
    value: ManifestCommitment,
    expected_role: Qwen3ModelRole,
) -> bool {
    let model = match expected_role {
        Qwen3ModelRole::Target8B => deployment.target_model,
        Qwen3ModelRole::Draft06B => deployment.draft_model,
    };
    &&& value.role == expected_role
    &&& value.version == PREPACKED_WEIGHT_MANIFEST_VERSION
    &&& value.source_weights_id@ == model.weights.weights_id.bytes_spec()
    &&& value.aggregate_id@ != Seq::new(32, |index: int| 0u8)
    &&& value.source_artifact_bytes == model.weights.total_bytes
    &&& value.tensor_data_bytes == role_tensor_data_bytes(expected_role)
    &&& value.output_bytes == value.tensor_data_bytes
    &&& value.section_count == role_tensor_count(expected_role)
    &&& 0 < value.canonical_manifest_bytes
    &&& value.canonical_manifest_bytes <= MAX_MANIFEST_RECORD_BYTES_U32
}

/// Complete mixed-endian admission record for one exact descriptor value.
pub closed spec fn bundle_admission_record_wire(
    deployment: DeploymentBundle,
    target: ManifestCommitment,
    draft: ManifestCommitment,
) -> Seq<u8> {
    MAGIC@
        + u32_little_endian(BUNDLE_ADMISSION_RECORD_VERSION)
        + super::bundle::canonical_deployment_bundle_wire(deployment)
        + manifest_commitment_wire(target)
        + manifest_commitment_wire(draft)
}

/// Executable value relation for one canonical admission descriptor.
pub closed spec fn bundle_admission_descriptor_spec(
    deployment: DeploymentBundle,
    target: ManifestCommitment,
    draft: ManifestCommitment,
) -> bool {
    &&& super::bundle::canonical_deployment_bundle_spec(deployment)
    &&& manifest_commitment_spec(deployment, target, Qwen3ModelRole::Target8B)
    &&& manifest_commitment_spec(deployment, draft, Qwen3ModelRole::Draft06B)
}

/// A canonical record paired with the exact production values that encode it.
pub closed spec fn canonical_bundle_admission_values(
    bytes: Seq<u8>,
    deployment: DeploymentBundle,
    target: ManifestCommitment,
    draft: ManifestCommitment,
) -> bool {
    bundle_admission_descriptor_spec(deployment, target, draft)
        && bytes == bundle_admission_record_wire(deployment, target, draft)
}

closed spec fn commitment_matches_bytes(
    value: ManifestCommitment,
    bytes: Seq<u8>,
    offset: int,
) -> bool
    recommends 0 <= offset, offset + MANIFEST_COMMITMENT_BYTES <= bytes.len(),
{
    &&& bytes[offset] == match value.role {
        Qwen3ModelRole::Target8B => ROLE_TARGET,
        Qwen3ModelRole::Draft06B => ROLE_DRAFT,
    }
    &&& value.version == u32_at(bytes, offset + 1)
    &&& value.source_weights_id@ == bytes.subrange(offset + 5, offset + 37)
    &&& value.aggregate_id@ == bytes.subrange(offset + 37, offset + 69)
    &&& value.source_artifact_bytes == u64_at(bytes, offset + 69)
    &&& value.tensor_data_bytes == u64_at(bytes, offset + 77)
    &&& value.output_bytes == u64_at(bytes, offset + 85)
    &&& value.section_count == u32_at(bytes, offset + 93)
    &&& value.canonical_manifest_bytes == u32_at(bytes, offset + 97)
}

closed spec fn commitment_bytes_spec(
    bytes: Seq<u8>,
    offset: int,
    deployment: DeploymentBundle,
    role: Qwen3ModelRole,
) -> bool
    recommends 0 <= offset, offset + MANIFEST_COMMITMENT_BYTES <= bytes.len(),
{
    let expected_role = match role {
        Qwen3ModelRole::Target8B => ROLE_TARGET,
        Qwen3ModelRole::Draft06B => ROLE_DRAFT,
    };
    let model = match role {
        Qwen3ModelRole::Target8B => deployment.target_model,
        Qwen3ModelRole::Draft06B => deployment.draft_model,
    };
    &&& bytes[offset] == expected_role
    &&& u32_at(bytes, offset + 1) == PREPACKED_WEIGHT_MANIFEST_VERSION
    &&& bytes.subrange(offset + 5, offset + 37) == model.weights.weights_id.bytes_spec()
    &&& bytes.subrange(offset + 37, offset + 69) != Seq::new(32, |index: int| 0u8)
    &&& u64_at(bytes, offset + 69) == model.weights.total_bytes
    &&& u64_at(bytes, offset + 77) == role_tensor_data_bytes(role)
    &&& u64_at(bytes, offset + 85) == u64_at(bytes, offset + 77)
    &&& u32_at(bytes, offset + 93) == role_tensor_count(role)
    &&& 0 < u32_at(bytes, offset + 97)
    &&& u32_at(bytes, offset + 97) <= MAX_MANIFEST_RECORD_BYTES_U32
}

closed spec fn raw_commitment_wire(bytes: Seq<u8>, offset: int) -> Seq<u8>
    recommends 0 <= offset, offset + MANIFEST_COMMITMENT_BYTES <= bytes.len(),
{
    seq![bytes[offset]]
        + u32_little_endian(u32_at(bytes, offset + 1))
        + bytes.subrange(offset + 5, offset + 37)
        + bytes.subrange(offset + 37, offset + 69)
        + u64_little_endian(u64_at(bytes, offset + 69))
        + u64_little_endian(u64_at(bytes, offset + 77))
        + u64_little_endian(u64_at(bytes, offset + 85))
        + u32_little_endian(u32_at(bytes, offset + 93))
        + u32_little_endian(u32_at(bytes, offset + 97))
}

closed spec fn raw_admission_wire(bytes: Seq<u8>) -> Seq<u8>
    recommends bytes.len() == BUNDLE_ADMISSION_RECORD_BYTES,
{
    bytes.subrange(0, 16)
        + u32_little_endian(u32_at(bytes, 16))
        + bytes.subrange(20, 542)
        + raw_commitment_wire(bytes, 542)
        + raw_commitment_wire(bytes, 643)
}

/// Exact verifier-visible byte acceptance relation for the production decoder.
pub closed spec fn canonical_bundle_admission_bytes(bytes: Seq<u8>) -> bool {
    if bytes.len() != BUNDLE_ADMISSION_RECORD_BYTES {
        false
    } else {
        let bundle_bytes = bytes.subrange(20, 20 + CANONICAL_DEPLOYMENT_BUNDLE_BYTES);
        let deployment = super::bundle::parsed_bundle_spec(bundle_bytes);
        &&& bytes.subrange(0, 16) == MAGIC@
        &&& u32_at(bytes, 16) == BUNDLE_ADMISSION_RECORD_VERSION
        &&& super::bundle::canonical_deployment_bundle_bytes(bundle_bytes)
        &&& commitment_bytes_spec(bytes, 542, deployment, Qwen3ModelRole::Target8B)
        &&& commitment_bytes_spec(bytes, 643, deployment, Qwen3ModelRole::Draft06B)
        &&& bytes == raw_admission_wire(bytes)
    }
}

proof fn commitment_matches_reencodes(
    value: ManifestCommitment,
    bytes: Seq<u8>,
    offset: int,
)
    requires
        0 <= offset,
        offset + MANIFEST_COMMITMENT_BYTES <= bytes.len(),
        commitment_matches_bytes(value, bytes, offset),
    ensures manifest_commitment_wire(value) == raw_commitment_wire(bytes, offset),
{
}

proof fn parsed_admission_reencodes(
    bytes: Seq<u8>,
    deployment: DeploymentBundle,
    target: ManifestCommitment,
    draft: ManifestCommitment,
)
    requires
        bytes.len() == BUNDLE_ADMISSION_RECORD_BYTES,
        bytes.subrange(0, 16) == MAGIC@,
        u32_at(bytes, 16) == BUNDLE_ADMISSION_RECORD_VERSION,
        bytes.subrange(20, 542)
            == super::bundle::canonical_deployment_bundle_wire(deployment),
        commitment_matches_bytes(target, bytes, 542),
        commitment_matches_bytes(draft, bytes, 643),
    ensures
        bundle_admission_record_wire(deployment, target, draft)
            == raw_admission_wire(bytes),
{
    commitment_matches_reencodes(target, bytes, 542);
    commitment_matches_reencodes(draft, bytes, 643);
}

/// Exact relation between accepted bytes and every decoded descriptor field.
pub closed spec fn descriptor_matches_bytes(
    descriptor: BundleAdmissionDescriptor,
    bytes: Seq<u8>,
) -> bool {
    &&& bytes.len() == BUNDLE_ADMISSION_RECORD_BYTES
    &&& descriptor.deployment == super::bundle::parsed_bundle_spec(
        bytes.subrange(20, 20 + CANONICAL_DEPLOYMENT_BUNDLE_BYTES),
    )
    &&& commitment_matches_bytes(descriptor.target_manifest, bytes, 542)
    &&& commitment_matches_bytes(descriptor.draft_manifest, bytes, 643)
    &&& descriptor.record_id.bytes_spec()
        == sha256::digest_spec(RECORD_DOMAIN@ + bytes)
}

/// Establishes the exact 744-byte production record width from its fields.
pub proof fn bundle_admission_record_wire_len(
    deployment: DeploymentBundle,
    target: ManifestCommitment,
    draft: ManifestCommitment,
)
    ensures bundle_admission_record_wire(deployment, target, draft).len()
        == BUNDLE_ADMISSION_RECORD_BYTES,
{
    reveal(bundle_admission_record_wire);
    reveal(manifest_commitment_wire);
    reveal(role_wire);
    reveal(u32_little_endian);
    reveal(u64_little_endian);
    vstd::bytes::lemma_auto_spec_u32_to_from_le_bytes();
    vstd::bytes::lemma_auto_spec_u64_to_from_le_bytes();
    super::bundle::canonical_deployment_bundle_wire_len(deployment);
    assert(target.source_weights_id@.len() == 32);
    assert(target.aggregate_id@.len() == 32);
    assert(draft.source_weights_id@.len() == 32);
    assert(draft.aggregate_id@.len() == 32);
    assert(u32_little_endian(target.version).len() == 4);
    assert(u64_little_endian(target.source_artifact_bytes).len() == 8);
    assert(u32_little_endian(draft.version).len() == 4);
    assert(u64_little_endian(draft.source_artifact_bytes).len() == 8);
}

/// Accepted bytes are exactly the canonical re-encoding of retained values.
pub proof fn accepted_admission_record_reencodes(
    bytes: Seq<u8>,
    descriptor: BundleAdmissionDescriptor,
)
    requires
        canonical_bundle_admission_values(
            bytes,
            descriptor.deployment,
            descriptor.target_manifest,
            descriptor.draft_manifest,
        ),
    ensures bytes == bundle_admission_record_wire(
        descriptor.deployment,
        descriptor.target_manifest,
        descriptor.draft_manifest,
    ),
{
}

/// Equal decoded descriptors uniquely identify equal accepted records.
pub proof fn accepted_admission_record_injective(
    left: Seq<u8>,
    right: Seq<u8>,
    left_descriptor: BundleAdmissionDescriptor,
    right_descriptor: BundleAdmissionDescriptor,
)
    requires
        canonical_bundle_admission_values(
            left,
            left_descriptor.deployment,
            left_descriptor.target_manifest,
            left_descriptor.draft_manifest,
        ),
        canonical_bundle_admission_values(
            right,
            right_descriptor.deployment,
            right_descriptor.target_manifest,
            right_descriptor.draft_manifest,
        ),
        left_descriptor == right_descriptor,
    ensures left == right,
{
    accepted_admission_record_reencodes(left, left_descriptor);
    accepted_admission_record_reencodes(right, right_descriptor);
}

} // verus!

impl fmt::Display for BundleAdmissionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength => formatter.write_str("bundle admission record has wrong length"),
            Self::InvalidMagic => formatter.write_str("bundle admission record magic is invalid"),
            Self::InvalidVersion => {
                formatter.write_str("bundle admission record version is unsupported")
            }
            Self::CanonicalBundle(error) => {
                write!(formatter, "canonical bundle is invalid: {error}")
            }
            Self::InvalidManifest { role, reason } => {
                write!(
                    formatter,
                    "{role:?} prepacked manifest is invalid: {reason}"
                )
            }
            Self::InvalidCommitment { role, reason } => {
                write!(
                    formatter,
                    "{role:?} manifest commitment is invalid: {reason}"
                )
            }
        }
    }
}

impl std::error::Error for BundleAdmissionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CanonicalBundle(error) => Some(error),
            _ => None,
        }
    }
}

impl From<CanonicalBundleError> for BundleAdmissionError {
    fn from(error: CanonicalBundleError) -> Self {
        Self::CanonicalBundle(error)
    }
}

verus! {

/// Exact directly checked relation between one retained manifest and its
/// admission commitment. The digest equality is functional SHA-256 only; it
/// does not claim collision resistance, file provenance, or a signature.
pub closed spec fn retained_manifest_commitment_spec(
    manifest: WeightSectionManifest,
    deployment: DeploymentBundle,
    role: Qwen3ModelRole,
    commitment: ManifestCommitment,
) -> bool {
    &&& commitment.role == role
    &&& commitment.version == manifest.version_spec()
    &&& commitment.source_weights_id@ == manifest.source_weights_id_spec()
    &&& commitment.aggregate_id@ == manifest.aggregate_id_spec()
    &&& commitment.source_artifact_bytes == manifest.source_artifact_bytes_spec()
    &&& commitment.tensor_data_bytes == manifest.tensor_data_bytes_spec()
    &&& commitment.output_bytes == manifest.output_bytes_spec()
    &&& commitment.section_count == manifest.sections_spec().len()
    &&& commitment.canonical_manifest_bytes == manifest.canonical_bytes_spec().len()
    &&& manifest_commitment_spec(deployment, commitment, role)
    &&& manifest.aggregate_id_spec()
        == sha256::digest_spec(manifest.canonical_bytes_spec())
}

/// Exact success relation for the directly verified record-sealing core.
pub closed spec fn sealed_admission_record_spec(
    record: BundleAdmissionRecord,
    deployment: DeploymentBundle,
    target_manifest: WeightSectionManifest,
    draft_manifest: WeightSectionManifest,
    target: ManifestCommitment,
    draft: ManifestCommitment,
) -> bool {
    &&& canonical_bundle_admission_bytes(record.bytes_spec())
    &&& record.record_id_spec().bytes_spec()
        == sha256::digest_spec(RECORD_DOMAIN@ + record.bytes_spec())
    &&& retained_manifest_commitment_spec(
        target_manifest, deployment, Qwen3ModelRole::Target8B, target,
    )
    &&& retained_manifest_commitment_spec(
        draft_manifest, deployment, Qwen3ModelRole::Draft06B, draft,
    )
    &&& canonical_bundle_admission_values(record.bytes_spec(), deployment, target, draft)
}

/// Exact record composition retained by a sealed admission authority.
pub closed spec fn authenticated_bundle_admission_spec(
    authority: AuthenticatedBundleAdmission,
) -> bool {
    exists |target: ManifestCommitment, draft: ManifestCommitment|
        sealed_admission_record_spec(
            authority.record_spec(),
            authority.prepacked_spec().deployment_spec(),
            authority.prepacked_spec().target_manifest_spec(),
            authority.prepacked_spec().draft_manifest_spec(),
            target,
            draft,
        )
}

} // verus!

/// Consumes exact authenticated prepacked inputs and seals their canonical
/// deployment commitment.
///
/// # Errors
///
/// Returns [`BundleAdmissionError`] unless both manifests remain complete,
/// role-correct, digest-bound, gap-free, and semantically exact for the
/// canonical deployment. Success is not signature, artifact-load, launch, or
/// independent-validation authority.
///
/// The semantic section-roster checks are an explicit runtime gate. The
/// directly verified inner composition derives no roster fact from that gate;
/// it binds the actual retained scalar, digest, and record values after the
/// gate succeeds.
pub fn seal_authenticated_bundle(
    prepacked: PrepackedDeploymentBundle,
) -> Result<AuthenticatedBundleAdmission, BundleAdmissionError> {
    validate_manifest_sections(prepacked.target_manifest(), Qwen3ModelRole::Target8B)?;
    validate_manifest_sections(prepacked.draft_manifest(), Qwen3ModelRole::Draft06B)?;
    let sealed = seal_prepacked_record_verified(&prepacked)?;
    Ok(authenticated_bundle_admission(prepacked, sealed))
}

verus! {

type SealedAdmissionRecord = (
    BundleAdmissionRecord,
    ManifestCommitment,
    ManifestCommitment,
);

fn sealed_admission_record(
    sealed: SealedAdmissionRecord,
) -> (record: BundleAdmissionRecord)
    ensures record == sealed.0,
{
    let (record, _, _) = sealed;
    record
}

fn seal_prepacked_record_verified(
    prepacked: &PrepackedDeploymentBundle,
) -> (result: Result<SealedAdmissionRecord, BundleAdmissionError>)
    ensures result.is_ok() ==> {
        let sealed = result.get_Ok_0();
        sealed_admission_record_spec(
            sealed.0,
            prepacked.deployment_spec(),
            prepacked.target_manifest_spec(),
            prepacked.draft_manifest_spec(),
            sealed.1,
            sealed.2,
        )
    },
{
    let deployment = prepacked.deployment_exact();
    let target_manifest = prepacked.target_manifest_exact();
    let draft_manifest = prepacked.draft_manifest_exact();
    seal_admission_record_verified(deployment, target_manifest, draft_manifest)
}

fn authenticated_bundle_admission(
    prepacked: PrepackedDeploymentBundle,
    sealed: SealedAdmissionRecord,
) -> (authority: AuthenticatedBundleAdmission)
    requires sealed_admission_record_spec(
        sealed.0,
        prepacked.deployment_spec(),
        prepacked.target_manifest_spec(),
        prepacked.draft_manifest_spec(),
        sealed.1,
        sealed.2,
    ),
    ensures
        authority.prepacked_spec() == prepacked,
        authority.record_spec() == sealed.0,
    authenticated_bundle_admission_spec(authority),
{
    let ghost target = sealed.1;
    let ghost draft = sealed.2;
    let record = sealed_admission_record(sealed);
    let authority = AuthenticatedBundleAdmission { prepacked, record };
    assert(authenticated_bundle_admission_spec(authority)) by {
        assert(sealed_admission_record_spec(
            authority.record_spec(),
            authority.prepacked_spec().deployment_spec(),
            authority.prepacked_spec().target_manifest_spec(),
            authority.prepacked_spec().draft_manifest_spec(),
            target,
            draft,
        ));
    }
    authority
}

fn seal_admission_record_verified(
    deployment: &DeploymentBundle,
    target_manifest: &WeightSectionManifest,
    draft_manifest: &WeightSectionManifest,
) -> (result: Result<SealedAdmissionRecord, BundleAdmissionError>)
    ensures result.is_ok() ==> {
        let sealed = result.get_Ok_0();
        sealed_admission_record_spec(
            sealed.0,
            *deployment,
            *target_manifest,
            *draft_manifest,
            sealed.1,
            sealed.2,
        )
    },
{
    let deployment_record = match encode_canonical_deployment_bundle(deployment) {
        Ok(record) => record,
        Err(error) => return Err(BundleAdmissionError::CanonicalBundle(error)),
    };
    let target = validate_manifest_commitment_verified(
        target_manifest,
        deployment,
        Qwen3ModelRole::Target8B,
    )?;
    let draft = validate_manifest_commitment_verified(
        draft_manifest,
        deployment,
        Qwen3ModelRole::Draft06B,
    )?;
    let record = encode_record(deployment_record.as_bytes(), target, draft);
    match decode_bundle_admission_record(record.as_bytes()) {
        Ok(_) => {},
        Err(error) => return Err(error),
    }
    Ok((record, target, draft))
}

/// Decodes and revalidates a canonical admission commitment.
///
/// # Errors
///
/// Returns [`BundleAdmissionError`] for truncation, trailing bytes, tag or
/// identity drift, impossible lengths, or a noncanonical embedded deployment.
/// The returned descriptor cannot recreate the consumed authentication
/// authority.
pub fn decode_bundle_admission_record(
    bytes: &[u8],
) -> (result: Result<BundleAdmissionDescriptor, BundleAdmissionError>)
    ensures
        result.is_ok() == canonical_bundle_admission_bytes(bytes@),
        result.is_ok() ==> {
            &&& descriptor_matches_bytes(result.get_Ok_0(), bytes@)
            &&& bundle_admission_descriptor_spec(
                result.get_Ok_0().deployment,
                result.get_Ok_0().target_manifest,
                result.get_Ok_0().draft_manifest,
            )
            &&& canonical_bundle_admission_values(
                bytes@,
                result.get_Ok_0().deployment,
                result.get_Ok_0().target_manifest,
                result.get_Ok_0().draft_manifest,
            )
            &&& bytes@ == bundle_admission_record_wire(
                result.get_Ok_0().deployment,
                result.get_Ok_0().target_manifest,
                result.get_Ok_0().draft_manifest,
            )
        },
{
    if bytes.len() != BUNDLE_ADMISSION_RECORD_BYTES {
        assert(!canonical_bundle_admission_bytes(bytes@));
        return Err(BundleAdmissionError::InvalidLength);
    }
    let mut reader = Reader::new(bytes);
    let magic = reader.array::<16>()?;
    if !bytes_equal(&magic, &MAGIC) {
        return Err(BundleAdmissionError::InvalidMagic);
    }
    if reader.u32()? != BUNDLE_ADMISSION_RECORD_VERSION {
        return Err(BundleAdmissionError::InvalidVersion);
    }
    let bundle_bytes = reader.array::<CANONICAL_DEPLOYMENT_BUNDLE_BYTES>()?;
    let deployment = match decode_canonical_deployment_bundle(&bundle_bytes) {
        Ok(deployment) => deployment,
        Err(error) => return Err(BundleAdmissionError::CanonicalBundle(error)),
    };
    let target_manifest = reader.commitment(Qwen3ModelRole::Target8B)?;
    let draft_manifest = reader.commitment(Qwen3ModelRole::Draft06B)?;
    validate_commitment(&deployment, target_manifest)?;
    validate_commitment(&deployment, draft_manifest)?;
    let record = encode_record(&bundle_bytes, target_manifest, draft_manifest);
    proof {
        parsed_admission_reencodes(
            bytes@, deployment, target_manifest, draft_manifest,
        );
    }
    assert(record.bytes_spec() == raw_admission_wire(bytes@));
    let exact_reencoding = bytes_equal(record.as_bytes(), bytes);
    if !exact_reencoding {
        assert(!canonical_bundle_admission_bytes(bytes@));
        return Err(BundleAdmissionError::InvalidLength);
    }
    let descriptor = BundleAdmissionDescriptor {
        deployment,
        target_manifest,
        draft_manifest,
        record_id: record.record_id,
    };
    proof {
        accepted_admission_record_reencodes(bytes@, descriptor);
    }
    Ok(descriptor)
}

} // verus!

/// Runtime-only semantic section validation.
///
/// This body deliberately remains outside direct verification: tensor-name
/// parsing, `BTreeSet` roster construction, string handling, and the metadata
/// classifier are explicit trusted runtime dependencies. The directly verified
/// caller derives no ghost fact from this helper beyond observing `Ok(())`.
fn validate_manifest_sections(
    manifest: &WeightSectionManifest,
    role: Qwen3ModelRole,
) -> Result<(), BundleAdmissionError> {
    let invalid = |reason| BundleAdmissionError::InvalidManifest { role, reason };
    let mut ordinals = BTreeSet::new();
    let mut expected_destination = 0_u64;
    for section in manifest.sections() {
        if section.role() != role
            || section.dtype() != TensorDType::Bf16
            || section.transform() != WeightTransform::Bf16RowMajorIdentityV1
            || section.alignment() != 2
            || section.sha256() == [0; 32]
        {
            return Err(invalid("section authority"));
        }
        let (kind, layer, ordinal) = classify_tensor_name(role, section.tensor_name())
            .map_err(|_| invalid("tensor name"))?;
        let (rank, dimension_0, dimension_1) = section.shape();
        Qwen3TensorMetadata {
            role,
            kind,
            layer,
            dtype: section.dtype(),
            rank,
            dimension_0,
            dimension_1,
        }
        .validate()
        .map_err(|_| invalid("tensor schema"))?;
        if !ordinals.insert(ordinal) {
            return Err(invalid("tensor ordinal"));
        }
        let expected_length = u64::from(dimension_0)
            .checked_mul(u64::from(dimension_1))
            .and_then(|elements| elements.checked_mul(2))
            .ok_or_else(|| invalid("tensor byte arithmetic"))?;
        let (destination, length) = section.destination_range();
        if destination != expected_destination || length != expected_length {
            return Err(invalid("destination coverage"));
        }
        let (source, source_length) = section.source_range();
        if source_length != length
            || source
                .checked_add(source_length)
                .is_none_or(|end| end > manifest.source_artifact_bytes())
        {
            return Err(invalid("source coverage"));
        }
        expected_destination = destination
            .checked_add(length)
            .ok_or_else(|| invalid("destination arithmetic"))?;
    }
    if expected_destination != manifest.output_bytes()
        || ordinals.len() != role.tensor_count() as usize
        || ordinals.iter().copied().ne(0..role.tensor_count())
    {
        return Err(invalid("complete tensor roster"));
    }

    Ok(())
}

verus! {

fn validate_manifest_commitment_verified(
    manifest: &WeightSectionManifest,
    deployment: &DeploymentBundle,
    role: Qwen3ModelRole,
) -> (result: Result<ManifestCommitment, BundleAdmissionError>)
    ensures result.is_ok() ==> retained_manifest_commitment_spec(
        *manifest, *deployment, role, result.get_Ok_0(),
    ),
{
    let model = match role {
        Qwen3ModelRole::Target8B => deployment.target_model,
        Qwen3ModelRole::Draft06B => deployment.draft_model,
    };
    let (tensor_data_bytes, tensor_count) = match role {
        Qwen3ModelRole::Target8B => (super::QWEN3_TARGET_TENSOR_DATA_BYTES, 399u32),
        Qwen3ModelRole::Draft06B => (super::QWEN3_DRAFT_TENSOR_DATA_BYTES, 311u32),
    };
    let invalid = |reason| BundleAdmissionError::InvalidManifest { role, reason };
    if manifest.version() != PREPACKED_WEIGHT_MANIFEST_VERSION {
        return Err(invalid("version"));
    }
    if manifest.role() != role {
        return Err(invalid("role"));
    }
    if !bytes_equal(&manifest.source_weights_id(), model.weights.weights_id.as_bytes()) {
        return Err(invalid("source identity"));
    }
    if manifest.source_artifact_bytes() != model.weights.total_bytes {
        return Err(invalid("source byte count"));
    }
    if manifest.tensor_data_bytes() != tensor_data_bytes
        || manifest.output_bytes() != manifest.tensor_data_bytes()
    {
        return Err(invalid("tensor byte count"));
    }
    let section_count = manifest.sections().len();
    if section_count != tensor_count as usize {
        return Err(invalid("section count"));
    }
    let canonical_manifest_bytes = manifest.canonical_bytes().len();
    if canonical_manifest_bytes == 0
        || canonical_manifest_bytes > MAX_MANIFEST_RECORD_BYTES
    {
        return Err(invalid("canonical manifest digest"));
    }
    let digest = sha256::digest(manifest.canonical_bytes());
    if !bytes_equal(&digest, &manifest.aggregate_id()) {
        return Err(invalid("canonical manifest digest"));
    }
    let zero = [0u8; 32];
    assert(zero@ == Seq::new(32, |index: int| 0u8)) by {
        assert(zero@ =~= Seq::new(32, |index: int| 0u8)) by {
            assert forall|index: int| 0 <= index < 32 implies
                zero@[index] == Seq::new(32, |position: int| 0u8)[index] by {}
        }
    }
    if bytes_equal(&manifest.aggregate_id(), &zero) {
        return Err(invalid("canonical manifest digest"));
    }
    assert(section_count <= MAX_MANIFEST_RECORD_BYTES);
    assert(canonical_manifest_bytes <= MAX_MANIFEST_RECORD_BYTES);
    let section_count_u32 = match u32::try_from(section_count) {
        Ok(value) => value,
        Err(_) => return Err(invalid("section count")),
    };
    let canonical_manifest_bytes_u32 = match u32::try_from(canonical_manifest_bytes) {
        Ok(value) => value,
        Err(_) => return Err(invalid("canonical manifest digest")),
    };
    let commitment = ManifestCommitment {
        role,
        version: manifest.version(),
        source_weights_id: manifest.source_weights_id(),
        aggregate_id: manifest.aggregate_id(),
        source_artifact_bytes: manifest.source_artifact_bytes(),
        tensor_data_bytes: manifest.tensor_data_bytes(),
        output_bytes: manifest.output_bytes(),
        section_count: section_count_u32,
        canonical_manifest_bytes: canonical_manifest_bytes_u32,
    };
    assert(manifest_commitment_spec(*deployment, commitment, role));
    assert(manifest.aggregate_id_spec()
        == sha256::digest_spec(manifest.canonical_bytes_spec()));
    Ok(commitment)
}

fn validate_commitment(
    deployment: &DeploymentBundle,
    commitment: ManifestCommitment,
) -> (result: Result<(), BundleAdmissionError>)
    ensures result.is_ok() == manifest_commitment_spec(*deployment, commitment, commitment.role),
{
    let role = commitment.role;
    let model = match role {
        Qwen3ModelRole::Target8B => deployment.target_model,
        Qwen3ModelRole::Draft06B => deployment.draft_model,
    };
    let (tensor_data_bytes, tensor_count) = match role {
        Qwen3ModelRole::Target8B => (super::QWEN3_TARGET_TENSOR_DATA_BYTES, 399),
        Qwen3ModelRole::Draft06B => (super::QWEN3_DRAFT_TENSOR_DATA_BYTES, 311),
    };
    let invalid = |reason| BundleAdmissionError::InvalidCommitment { role, reason };
    if commitment.version != PREPACKED_WEIGHT_MANIFEST_VERSION {
        return Err(invalid("version"));
    }
    let source_matches = bytes_equal(
        &commitment.source_weights_id, model.weights.weights_id.as_bytes(),
    );
    let zero = [0u8; 32];
    assert(zero@ == Seq::new(32, |index: int| 0u8)) by {
        assert(zero@ =~= Seq::new(32, |index: int| 0u8)) by {
            assert forall|index: int| 0 <= index < 32 implies
                zero@[index] == Seq::new(32, |position: int| 0u8)[index] by {}
        }
    }
    let aggregate_is_zero = bytes_equal(&commitment.aggregate_id, &zero);
    if !source_matches || aggregate_is_zero {
        return Err(invalid("identity"));
    }
    if commitment.source_artifact_bytes != model.weights.total_bytes
        || commitment.tensor_data_bytes != tensor_data_bytes
        || commitment.output_bytes != commitment.tensor_data_bytes
    {
        return Err(invalid("byte count"));
    }
    if commitment.section_count != tensor_count
        || commitment.canonical_manifest_bytes == 0
        || commitment.canonical_manifest_bytes > MAX_MANIFEST_RECORD_BYTES_U32
    {
        return Err(invalid("manifest bound"));
    }
    assert(manifest_commitment_spec(*deployment, commitment, commitment.role));
    Ok(())
}

fn encode_record(
    bundle: &[u8; CANONICAL_DEPLOYMENT_BUNDLE_BYTES],
    target: ManifestCommitment,
    draft: ManifestCommitment,
) -> (record: BundleAdmissionRecord)
    ensures
        record.bytes_spec() == MAGIC@
            + u32_little_endian(BUNDLE_ADMISSION_RECORD_VERSION)
            + bundle@
            + manifest_commitment_wire(target)
            + manifest_commitment_wire(draft),
        record.record_id.bytes_spec()
            == sha256::digest_spec(RECORD_DOMAIN@ + record.bytes_spec()),
{
    let mut writer = Writer::new();
    writer.bytes(&MAGIC);
    writer.u32(BUNDLE_ADMISSION_RECORD_VERSION);
    writer.bytes(bundle);
    writer.commitment(target);
    writer.commitment(draft);
    proof {
        bundle_admission_record_wire_len(
            super::bundle::parsed_bundle_spec(bundle@), target, draft,
        );
    }
    assert(writer.offset == BUNDLE_ADMISSION_RECORD_BYTES);
    assert(writer.view() == writer.bytes@);
    let mut hasher = sha256::Sha256::new();
    assert(RECORD_DOMAIN@.len() == 41);
    proof { sha256::initial_view_is_valid(); }
    assert(hasher.view() == sha256::initial_view());
    assert(hasher.view().1 == 0);
    assert(sha256::can_update_view(hasher.view(), RECORD_DOMAIN@.len()));
    proof {
        hasher.derive_can_update(RECORD_DOMAIN@.len());
    }
    hasher.update(&RECORD_DOMAIN);
    proof {
        sha256::initial_view_is_valid();
        sha256::update_view_byte_len(sha256::initial_view(), RECORD_DOMAIN@);
    }
    assert(hasher.view().1 == RECORD_DOMAIN@.len());
    assert(RECORD_DOMAIN@.len() + writer.bytes@.len() <= u64::MAX / 8);
    assert(sha256::can_update_view(hasher.view(), writer.bytes@.len()));
    proof {
        hasher.derive_can_update(writer.bytes@.len());
    }
    hasher.update(&writer.bytes);
    proof {
        sha256::update_view_concat(sha256::initial_view(), RECORD_DOMAIN@, writer.bytes@);
    }
    assert(hasher.view() == sha256::update_view(
        sha256::initial_view(), RECORD_DOMAIN@ + writer.bytes@,
    ));
    let digest = hasher.finish();
    proof {
        sha256::digest_spec_definition(RECORD_DOMAIN@ + writer.bytes@);
    }
    BundleAdmissionRecord {
        bytes: writer.bytes,
        record_id: Identity::new(digest),
    }
}

struct Writer {
    bytes: [u8; BUNDLE_ADMISSION_RECORD_BYTES],
    offset: usize,
}

impl Writer {
    closed spec fn valid(&self) -> bool {
        self.offset <= BUNDLE_ADMISSION_RECORD_BYTES
    }

    closed spec fn view(&self) -> Seq<u8>
        recommends self.valid(),
    {
        self.bytes@.subrange(0, self.offset as int)
    }

    const fn new() -> (writer: Self)
        ensures writer.valid(), writer.offset == 0, writer.view() == Seq::<u8>::empty(),
    {
        Self {
            bytes: [0; BUNDLE_ADMISSION_RECORD_BYTES],
            offset: 0,
        }
    }

    fn bytes(&mut self, value: &[u8])
        requires
            old(self).valid(),
            old(self).offset + value@.len() <= BUNDLE_ADMISSION_RECORD_BYTES,
        ensures
            final(self).valid(),
            final(self).offset == old(self).offset + value@.len(),
            final(self).view() == old(self).view() + value@,
    {
        let ghost initial_view = self.view();
        let ghost initial_offset = self.offset;
        let mut index = 0;
        while index < value.len()
            invariant
                self.valid(),
                initial_offset + value@.len() <= BUNDLE_ADMISSION_RECORD_BYTES,
                0 <= index <= value@.len(),
                self.offset == initial_offset + index,
                self.view() == initial_view + value@.subrange(0, index as int),
            decreases value@.len() - index,
        {
            let byte = value[index];
            self.byte_with_capacity(byte);
            index += 1;
            assert(value@.subrange(0, index as int)
                == value@.subrange(0, index as int - 1).push(byte)) by {
                assert(value@.subrange(0, index as int) =~=
                    value@.subrange(0, index as int - 1).push(byte)) by {
                    assert forall|position: int| 0 <= position < index implies
                        value@.subrange(0, index as int)[position]
                            == value@.subrange(0, index as int - 1).push(byte)[position] by {
                        if position < index - 1 {} else {}
                    }
                }
            }
        }
        assert(value@.subrange(0, value@.len() as int) == value@);
    }

    fn u8(&mut self, value: u8)
        requires old(self).valid(), old(self).offset + 1 <= BUNDLE_ADMISSION_RECORD_BYTES,
        ensures
            final(self).valid(),
            final(self).offset == old(self).offset + 1,
            final(self).view() == old(self).view() + seq![value],
    {
        self.bytes(&[value]);
    }

    fn u32(&mut self, value: u32)
        requires old(self).valid(), old(self).offset + 4 <= BUNDLE_ADMISSION_RECORD_BYTES,
        ensures
            final(self).valid(),
            final(self).offset == old(self).offset + 4,
            final(self).view() == old(self).view() + u32_little_endian(value),
    {
        let encoded = u32_to_le_bytes(value);
        self.bytes(&encoded);
    }

    fn u64(&mut self, value: u64)
        requires old(self).valid(), old(self).offset + 8 <= BUNDLE_ADMISSION_RECORD_BYTES,
        ensures
            final(self).valid(),
            final(self).offset == old(self).offset + 8,
            final(self).view() == old(self).view() + u64_little_endian(value),
    {
        let encoded = u64_to_le_bytes(value);
        self.bytes(&encoded);
    }

    fn commitment(&mut self, value: ManifestCommitment)
        requires
            old(self).valid(),
            old(self).offset + MANIFEST_COMMITMENT_BYTES <= BUNDLE_ADMISSION_RECORD_BYTES,
        ensures
            final(self).valid(),
            final(self).offset == old(self).offset + MANIFEST_COMMITMENT_BYTES,
            final(self).view() == old(self).view() + manifest_commitment_wire(value),
    {
        self.u8(match value.role {
            Qwen3ModelRole::Target8B => ROLE_TARGET,
            Qwen3ModelRole::Draft06B => ROLE_DRAFT,
        });
        self.u32(value.version);
        self.bytes(&value.source_weights_id);
        self.bytes(&value.aggregate_id);
        self.u64(value.source_artifact_bytes);
        self.u64(value.tensor_data_bytes);
        self.u64(value.output_bytes);
        self.u32(value.section_count);
        self.u32(value.canonical_manifest_bytes);
    }

    fn byte_with_capacity(&mut self, value: u8)
        requires
            old(self).valid(),
            old(self).offset < BUNDLE_ADMISSION_RECORD_BYTES,
        ensures
            final(self).valid(),
            final(self).offset == old(self).offset + 1,
            final(self).view() == old(self).view().push(value),
    {
        let offset = self.offset;
        self.bytes[offset] = value;
        self.offset += 1;
        assert(self.view() =~= old(self).view().push(value)) by {
            assert forall|position: int| 0 <= position < self.view().len() implies
                self.view()[position] == old(self).view().push(value)[position] by {
                if position < offset {} else { assert(position == offset); }
            }
        }
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    closed spec fn valid(&self) -> bool {
        self.offset <= self.bytes@.len()
    }

    const fn new(bytes: &'a [u8]) -> (reader: Self)
        ensures reader.valid(), reader.offset == 0, reader.bytes@ == bytes@,
    {
        Self { bytes, offset: 0 }
    }

    fn array<const N: usize>(&mut self) -> (result: Result<[u8; N], BundleAdmissionError>)
        requires old(self).valid(),
        ensures
            final(self).bytes@ == old(self).bytes@,
            result.is_ok() == (N <= old(self).bytes@.len() - old(self).offset),
            result.is_ok() ==> {
                &&& final(self).offset == old(self).offset + N
                &&& result.get_Ok_0()@ == old(self).bytes@.subrange(
                    old(self).offset as int, (old(self).offset + N) as int,
                )
            },
            result.is_err() ==> final(self).offset == old(self).offset,
    {
        if N > self.bytes.len() - self.offset {
            return Err(BundleAdmissionError::InvalidLength);
        }
        let start = self.offset;
        let mut value = [0; N];
        let mut index = 0;
        while index < N
            invariant
                self.offset == start,
                start + N <= self.bytes@.len(),
                self.bytes@.len() <= usize::MAX as nat,
                0 <= index <= N,
                value@.len() == N,
                forall|prior: int| 0 <= prior < index ==>
                    value@[prior] == self.bytes@[start as int + prior],
            decreases N - index,
        {
            proof {
                assert(start as nat + index as nat <= self.bytes@.len());
                assert(start as nat + index as nat <= usize::MAX as nat);
            }
            let position = match start.checked_add(index) {
                Some(position) => position,
                None => return Err(BundleAdmissionError::InvalidLength),
            };
            value[index] = self.bytes[position];
            index += 1;
        }
        assert(value@ =~= self.bytes@.subrange(start as int, (start + N) as int)) by {
            assert forall|position: int| 0 <= position < N implies
                value@[position] == self.bytes@.subrange(start as int, (start + N) as int)[position]
                by {}
        }
        self.offset += N;
        Ok(value)
    }

    fn u8(&mut self) -> (result: Result<u8, BundleAdmissionError>)
        requires old(self).valid(),
        ensures
            final(self).bytes@ == old(self).bytes@,
            result.is_ok() == (1 <= old(self).bytes@.len() - old(self).offset),
            result.is_ok() ==> {
                &&& final(self).offset == old(self).offset + 1
                &&& result.get_Ok_0() == old(self).bytes@[old(self).offset as int]
            },
    {
        match self.array::<1>() {
            Ok(value) => Ok(value[0]),
            Err(error) => Err(error),
        }
    }

    fn u32(&mut self) -> (result: Result<u32, BundleAdmissionError>)
        requires old(self).valid(),
        ensures
            final(self).bytes@ == old(self).bytes@,
            result.is_ok() == (4 <= old(self).bytes@.len() - old(self).offset),
            result.is_ok() ==> {
                &&& final(self).offset == old(self).offset + 4
                &&& result.get_Ok_0() == u32_at(old(self).bytes@, old(self).offset as int)
            },
    {
        match self.array::<4>() {
            Ok(value) => Ok(
                u32::from(value[0])
                    | (u32::from(value[1]) << 8)
                    | (u32::from(value[2]) << 16)
                    | (u32::from(value[3]) << 24),
            ),
            Err(error) => Err(error),
        }
    }

    fn u64(&mut self) -> (result: Result<u64, BundleAdmissionError>)
        requires old(self).valid(),
        ensures
            final(self).bytes@ == old(self).bytes@,
            result.is_ok() == (8 <= old(self).bytes@.len() - old(self).offset),
            result.is_ok() ==> {
                &&& final(self).offset == old(self).offset + 8
                &&& result.get_Ok_0() == u64_at(old(self).bytes@, old(self).offset as int)
            },
    {
        match self.array::<8>() {
            Ok(value) => Ok(
                u64::from(value[0])
                    | (u64::from(value[1]) << 8)
                    | (u64::from(value[2]) << 16)
                    | (u64::from(value[3]) << 24)
                    | (u64::from(value[4]) << 32)
                    | (u64::from(value[5]) << 40)
                    | (u64::from(value[6]) << 48)
                    | (u64::from(value[7]) << 56),
            ),
            Err(error) => Err(error),
        }
    }

    fn commitment(
        &mut self,
        expected_role: Qwen3ModelRole,
    ) -> (result: Result<ManifestCommitment, BundleAdmissionError>)
        requires old(self).valid(),
        ensures
            final(self).bytes@ == old(self).bytes@,
            result.is_ok() == {
                &&& MANIFEST_COMMITMENT_BYTES <= old(self).bytes@.len() - old(self).offset
                &&& old(self).bytes@[old(self).offset as int] == match expected_role {
                    Qwen3ModelRole::Target8B => ROLE_TARGET,
                    Qwen3ModelRole::Draft06B => ROLE_DRAFT,
                }
            },
            result.is_ok() ==> {
                &&& final(self).offset == old(self).offset + MANIFEST_COMMITMENT_BYTES
                &&& commitment_matches_bytes(
                    result.get_Ok_0(), old(self).bytes@, old(self).offset as int,
                )
                &&& result.get_Ok_0().role == expected_role
            },
    {
        let tag = self.u8()?;
        let role = match expected_role {
            Qwen3ModelRole::Target8B => {
                if tag == ROLE_TARGET {
                    Qwen3ModelRole::Target8B
                } else if tag == ROLE_DRAFT {
                    return Err(BundleAdmissionError::InvalidCommitment {
                        role: expected_role,
                        reason: "role order",
                    });
                } else {
                    return Err(BundleAdmissionError::InvalidCommitment {
                        role: expected_role,
                        reason: "role tag",
                    });
                }
            }
            Qwen3ModelRole::Draft06B => {
                if tag == ROLE_DRAFT {
                    Qwen3ModelRole::Draft06B
                } else if tag == ROLE_TARGET {
                    return Err(BundleAdmissionError::InvalidCommitment {
                        role: expected_role,
                        reason: "role order",
                    });
                } else {
                    return Err(BundleAdmissionError::InvalidCommitment {
                        role: expected_role,
                        reason: "role tag",
                    });
                }
            }
        };
        Ok(ManifestCommitment {
            role,
            version: self.u32()?,
            source_weights_id: self.array()?,
            aggregate_id: self.array()?,
            source_artifact_bytes: self.u64()?,
            tensor_data_bytes: self.u64()?,
            output_bytes: self.u64()?,
            section_count: self.u32()?,
            canonical_manifest_bytes: self.u32()?,
        })
    }
}

fn bytes_equal(left: &[u8], right: &[u8]) -> (equal: bool)
    ensures equal == (left@ == right@),
{
    if left.len() != right.len() {
        return false;
    }
    let mut index = 0;
    while index < left.len()
        invariant
            left@.len() == right@.len(),
            0 <= index <= left@.len(),
            forall|prior: int| 0 <= prior < index ==> left@[prior] == right@[prior],
        decreases left@.len() - index,
    {
        if left[index] != right[index] {
            return false;
        }
        index += 1;
    }
    assert(left@ =~= right@) by {
        assert forall|position: int| 0 <= position < left@.len() implies
            left@[position] == right@[position] by {}
    }
    true
}

} // verus!

#[cfg(test)]
mod tests {
    use super::{
        decode_bundle_admission_record, encode_record, BundleAdmissionError, ManifestCommitment,
        BUNDLE_ADMISSION_RECORD_BYTES, BUNDLE_ADMISSION_RECORD_VERSION, MANIFEST_COMMITMENT_BYTES,
        MAX_MANIFEST_RECORD_BYTES_U32, RECORD_DOMAIN,
    };
    use crate::{
        bundle::tests::exact_bundle,
        encode_canonical_deployment_bundle, seal_authenticated_bundle,
        tokenizer::tests::{authenticated_assets, test_tokenizer},
        weight_stream::tests::test_prepacked,
        WeightSectionManifest, PREPACKED_WEIGHT_MANIFEST_VERSION,
    };
    use ferric_spec::Qwen3ModelRole;

    const BUNDLE_OFFSET: usize = 20;
    const TARGET_OFFSET: usize = BUNDLE_OFFSET + crate::CANONICAL_DEPLOYMENT_BUNDLE_BYTES;
    const DRAFT_OFFSET: usize = TARGET_OFFSET + MANIFEST_COMMITMENT_BYTES;
    const SOURCE_ID_OFFSET: usize = 5;
    const AGGREGATE_ID_OFFSET: usize = 37;
    const SOURCE_BYTES_OFFSET: usize = 69;
    const TENSOR_BYTES_OFFSET: usize = 77;
    const OUTPUT_BYTES_OFFSET: usize = 85;
    const SECTION_COUNT_OFFSET: usize = 93;
    const MANIFEST_BYTES_OFFSET: usize = 97;

    fn commitment(role: Qwen3ModelRole, byte: u8) -> ManifestCommitment {
        let deployment = exact_bundle();
        let model = match role {
            Qwen3ModelRole::Target8B => deployment.target_model,
            Qwen3ModelRole::Draft06B => deployment.draft_model,
        };
        ManifestCommitment {
            role,
            version: PREPACKED_WEIGHT_MANIFEST_VERSION,
            source_weights_id: *model.weights.weights_id.as_bytes(),
            aggregate_id: [byte; 32],
            source_artifact_bytes: model.weights.total_bytes,
            tensor_data_bytes: role.tensor_data_bytes(),
            output_bytes: role.tensor_data_bytes(),
            section_count: role.tensor_count(),
            canonical_manifest_bytes: 1024,
        }
    }

    fn record() -> super::BundleAdmissionRecord {
        let bundle = encode_canonical_deployment_bundle(&exact_bundle()).expect("exact bundle");
        encode_record(
            bundle.as_bytes(),
            commitment(Qwen3ModelRole::Target8B, 0x31),
            commitment(Qwen3ModelRole::Draft06B, 0x32),
        )
    }

    fn retained_commitment(
        manifest: &WeightSectionManifest,
        role: Qwen3ModelRole,
    ) -> ManifestCommitment {
        ManifestCommitment {
            role,
            version: manifest.version(),
            source_weights_id: manifest.source_weights_id(),
            aggregate_id: manifest.aggregate_id(),
            source_artifact_bytes: manifest.source_artifact_bytes(),
            tensor_data_bytes: manifest.tensor_data_bytes(),
            output_bytes: manifest.output_bytes(),
            section_count: manifest
                .sections()
                .len()
                .try_into()
                .expect("bounded section count"),
            canonical_manifest_bytes: manifest
                .canonical_bytes()
                .len()
                .try_into()
                .expect("bounded manifest record"),
        }
    }

    fn replace_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn replace_u64(bytes: &mut [u8], offset: usize, value: u64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    #[test]
    fn canonical_descriptor_round_trips_and_is_identity_sensitive() {
        let record = record();
        assert_eq!(record.as_bytes().len(), BUNDLE_ADMISSION_RECORD_BYTES);
        let decoded = decode_bundle_admission_record(record.as_bytes()).expect("canonical record");
        assert_eq!(decoded.record_id, record.record_id());
        assert_eq!(decoded.target_manifest.aggregate_id, [0x31; 32]);
        assert_eq!(decoded.draft_manifest.aggregate_id, [0x32; 32]);

        let reencoded = encode_record(
            &record.as_bytes()[BUNDLE_OFFSET..TARGET_OFFSET]
                .try_into()
                .expect("fixed embedded bundle"),
            decoded.target_manifest,
            decoded.draft_manifest,
        );
        assert_eq!(reencoded.as_bytes(), record.as_bytes());
        assert_eq!(reencoded.record_id(), decoded.record_id);

        for index in 0..BUNDLE_ADMISSION_RECORD_BYTES {
            let mut changed = record.as_bytes().to_owned();
            changed[index] ^= 1;
            if let Ok(changed_descriptor) = decode_bundle_admission_record(&changed) {
                assert_ne!(
                    changed_descriptor.record_id,
                    record.record_id(),
                    "byte {index} was not identity-sensitive"
                );
            }
        }
    }

    #[test]
    fn independent_wire_oracle_matches_every_field_offset_and_record_digest() {
        let record = record();
        let bytes = record.as_bytes();
        let bundle = encode_canonical_deployment_bundle(&exact_bundle()).expect("exact bundle");
        let target = commitment(Qwen3ModelRole::Target8B, 0x31);
        let draft = commitment(Qwen3ModelRole::Draft06B, 0x32);

        let mut oracle = Vec::with_capacity(BUNDLE_ADMISSION_RECORD_BYTES);
        oracle.extend_from_slice(b"FERRIC-M1-ADMIT\0");
        oracle.extend_from_slice(&BUNDLE_ADMISSION_RECORD_VERSION.to_le_bytes());
        oracle.extend_from_slice(bundle.as_bytes());
        for value in [target, draft] {
            oracle.push(match value.role {
                Qwen3ModelRole::Target8B => 1,
                Qwen3ModelRole::Draft06B => 2,
            });
            oracle.extend_from_slice(&value.version.to_le_bytes());
            oracle.extend_from_slice(&value.source_weights_id);
            oracle.extend_from_slice(&value.aggregate_id);
            oracle.extend_from_slice(&value.source_artifact_bytes.to_le_bytes());
            oracle.extend_from_slice(&value.tensor_data_bytes.to_le_bytes());
            oracle.extend_from_slice(&value.output_bytes.to_le_bytes());
            oracle.extend_from_slice(&value.section_count.to_le_bytes());
            oracle.extend_from_slice(&value.canonical_manifest_bytes.to_le_bytes());
        }
        assert_eq!(oracle.len(), BUNDLE_ADMISSION_RECORD_BYTES);
        assert_eq!(oracle, bytes);
        assert_eq!(&bytes[BUNDLE_OFFSET..TARGET_OFFSET], bundle.as_bytes());
        assert_eq!(bytes[TARGET_OFFSET], 1);
        assert_eq!(bytes[DRAFT_OFFSET], 2);
        assert_eq!(
            &bytes[TARGET_OFFSET + SOURCE_BYTES_OFFSET..TARGET_OFFSET + TENSOR_BYTES_OFFSET],
            &target.source_artifact_bytes.to_le_bytes()
        );
        assert_eq!(
            &bytes[DRAFT_OFFSET + SECTION_COUNT_OFFSET..DRAFT_OFFSET + MANIFEST_BYTES_OFFSET],
            &draft.section_count.to_le_bytes()
        );

        let mut identity_preimage = RECORD_DOMAIN.to_vec();
        identity_preimage.extend_from_slice(bytes);
        assert_eq!(
            record.record_id().as_bytes(),
            &crate::sha256::digest(&identity_preimage)
        );
    }

    #[test]
    fn sealed_authority_decode_gate_preserves_valid_record_and_complete_rosters() {
        let prepacked = crate::build_prepacked_deployment_bundle(
            authenticated_assets(),
            test_tokenizer(Qwen3ModelRole::Target8B),
            test_tokenizer(Qwen3ModelRole::Draft06B),
            test_prepacked(Qwen3ModelRole::Target8B),
            test_prepacked(Qwen3ModelRole::Draft06B),
        )
        .expect("complete test prepacked deployment");
        let deployment_record = encode_canonical_deployment_bundle(prepacked.deployment())
            .expect("exact retained deployment");
        let expected_record = encode_record(
            deployment_record.as_bytes(),
            retained_commitment(prepacked.target_manifest(), Qwen3ModelRole::Target8B),
            retained_commitment(prepacked.draft_manifest(), Qwen3ModelRole::Draft06B),
        );
        let authority = seal_authenticated_bundle(prepacked).expect("sealed admission authority");
        assert_eq!(authority.record().as_bytes(), expected_record.as_bytes());
        assert_eq!(authority.record().record_id(), expected_record.record_id());
        let decoded = decode_bundle_admission_record(authority.record().as_bytes())
            .expect("sealed record decodes");
        assert_eq!(decoded.record_id, authority.record().record_id());
        assert_eq!(decoded.target_manifest.section_count, 399);
        assert_eq!(decoded.draft_manifest.section_count, 311);
        assert_eq!(
            authority.prepacked().target_manifest().aggregate_id(),
            decoded.target_manifest.aggregate_id
        );
        assert_eq!(
            authority.prepacked().draft_manifest().aggregate_id(),
            decoded.draft_manifest.aggregate_id
        );
    }

    #[test]
    fn truncation_trailing_version_role_and_zero_identity_fail_closed() {
        let record = record();
        for length in 0..BUNDLE_ADMISSION_RECORD_BYTES {
            assert_eq!(
                decode_bundle_admission_record(&record.as_bytes()[..length]),
                Err(BundleAdmissionError::InvalidLength),
                "truncation length {length} was accepted"
            );
        }
        for extra in [1, 2, 17, MANIFEST_COMMITMENT_BYTES] {
            let mut trailing = record.as_bytes().to_vec();
            trailing.resize(BUNDLE_ADMISSION_RECORD_BYTES + extra, 0xa5);
            assert_eq!(
                decode_bundle_admission_record(&trailing),
                Err(BundleAdmissionError::InvalidLength),
                "{extra} trailing bytes were accepted"
            );
        }
        let mut version = record.as_bytes().to_owned();
        version[16..20].copy_from_slice(&(BUNDLE_ADMISSION_RECORD_VERSION + 1).to_le_bytes());
        assert_eq!(
            decode_bundle_admission_record(&version),
            Err(BundleAdmissionError::InvalidVersion)
        );
        let mut role = record.as_bytes().to_owned();
        role[TARGET_OFFSET] = 9;
        assert!(matches!(
            decode_bundle_admission_record(&role),
            Err(BundleAdmissionError::InvalidCommitment {
                reason: "role tag",
                ..
            })
        ));
        let mut zero = record.as_bytes().to_owned();
        let aggregate_start = TARGET_OFFSET + AGGREGATE_ID_OFFSET;
        zero[aggregate_start..aggregate_start + 32].fill(0);
        assert!(matches!(
            decode_bundle_admission_record(&zero),
            Err(BundleAdmissionError::InvalidCommitment {
                reason: "identity",
                ..
            })
        ));
    }

    #[test]
    fn role_order_and_little_endian_scalars_are_fail_closed() {
        let record = record();

        let mut swapped = record.as_bytes().to_owned();
        let target = swapped[TARGET_OFFSET..DRAFT_OFFSET].to_vec();
        let draft = swapped[DRAFT_OFFSET..].to_vec();
        swapped[TARGET_OFFSET..DRAFT_OFFSET].copy_from_slice(&draft);
        swapped[DRAFT_OFFSET..].copy_from_slice(&target);
        assert!(matches!(
            decode_bundle_admission_record(&swapped),
            Err(BundleAdmissionError::InvalidCommitment {
                reason: "role order",
                ..
            })
        ));

        for offset in [16, TARGET_OFFSET + 1, DRAFT_OFFSET + 1] {
            let mut big_endian = record.as_bytes().to_owned();
            big_endian[offset..offset + 4]
                .copy_from_slice(&BUNDLE_ADMISSION_RECORD_VERSION.to_be_bytes());
            assert!(decode_bundle_admission_record(&big_endian).is_err());
        }
        for offset in [
            TARGET_OFFSET + SOURCE_BYTES_OFFSET,
            TARGET_OFFSET + TENSOR_BYTES_OFFSET,
            DRAFT_OFFSET + SOURCE_BYTES_OFFSET,
            DRAFT_OFFSET + OUTPUT_BYTES_OFFSET,
        ] {
            let mut reversed = record.as_bytes().to_owned();
            reversed[offset..offset + 8].reverse();
            assert!(decode_bundle_admission_record(&reversed).is_err());
        }
    }

    #[test]
    fn structural_commitment_boundaries_and_arbitrary_nonzero_aggregates_are_exact() {
        let record = record();
        for offset in [TARGET_OFFSET, DRAFT_OFFSET] {
            for manifest_bytes in [1, 2, MAX_MANIFEST_RECORD_BYTES_U32] {
                let mut boundary = record.as_bytes().to_owned();
                replace_u32(
                    &mut boundary,
                    offset + MANIFEST_BYTES_OFFSET,
                    manifest_bytes,
                );
                let decoded = decode_bundle_admission_record(&boundary)
                    .expect("in-range canonical manifest length");
                let commitment = if offset == TARGET_OFFSET {
                    decoded.target_manifest
                } else {
                    decoded.draft_manifest
                };
                assert_eq!(commitment.canonical_manifest_bytes, manifest_bytes);
            }
            for manifest_bytes in [0, MAX_MANIFEST_RECORD_BYTES_U32 + 1, u32::MAX] {
                let mut boundary = record.as_bytes().to_owned();
                replace_u32(
                    &mut boundary,
                    offset + MANIFEST_BYTES_OFFSET,
                    manifest_bytes,
                );
                assert!(decode_bundle_admission_record(&boundary).is_err());
            }

            for (position, value) in [(0, 1), (15, 0x80), (31, 0xff)] {
                let mut arbitrary = record.as_bytes().to_owned();
                arbitrary[offset + AGGREGATE_ID_OFFSET..offset + AGGREGATE_ID_OFFSET + 32].fill(0);
                arbitrary[offset + AGGREGATE_ID_OFFSET + position] = value;
                let decoded = decode_bundle_admission_record(&arbitrary)
                    .expect("arbitrary nonzero aggregate commitment");
                assert_ne!(decoded.record_id, record.record_id());
            }
        }
    }

    #[test]
    fn every_structural_identity_count_and_length_drift_is_rejected() {
        let record = record();
        for offset in [TARGET_OFFSET, DRAFT_OFFSET] {
            let mut source_id = record.as_bytes().to_owned();
            source_id[offset + SOURCE_ID_OFFSET + 17] ^= 0x80;
            assert!(decode_bundle_admission_record(&source_id).is_err());

            let mut source_bytes = record.as_bytes().to_owned();
            let source_value = u64::from_le_bytes(
                source_bytes[offset + SOURCE_BYTES_OFFSET..offset + TENSOR_BYTES_OFFSET]
                    .try_into()
                    .expect("source byte scalar"),
            );
            replace_u64(
                &mut source_bytes,
                offset + SOURCE_BYTES_OFFSET,
                source_value + 1,
            );
            assert!(decode_bundle_admission_record(&source_bytes).is_err());

            let mut tensor_bytes = record.as_bytes().to_owned();
            replace_u64(&mut tensor_bytes, offset + TENSOR_BYTES_OFFSET, 2);
            assert!(decode_bundle_admission_record(&tensor_bytes).is_err());

            let mut output_bytes = record.as_bytes().to_owned();
            replace_u64(&mut output_bytes, offset + OUTPUT_BYTES_OFFSET, 4);
            assert!(decode_bundle_admission_record(&output_bytes).is_err());

            let mut sections = record.as_bytes().to_owned();
            replace_u32(&mut sections, offset + SECTION_COUNT_OFFSET, 0);
            assert!(decode_bundle_admission_record(&sections).is_err());
        }
    }
}
