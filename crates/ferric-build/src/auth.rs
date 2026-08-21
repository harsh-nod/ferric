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

const MAGIC: [u8; 16] = *b"FERRIC-M1-ADMIT\0";
const RECORD_DOMAIN: &[u8] = b"ferric.authenticated-bundle-admission.v1\0";
const ROLE_TARGET: u8 = 1;
const ROLE_DRAFT: u8 = 2;
const MANIFEST_COMMITMENT_BYTES: usize = 101;
const MAX_MANIFEST_RECORD_BYTES: usize = 256 * 1_024;

/// Version of the authenticated deployment commitment.
pub const BUNDLE_ADMISSION_RECORD_VERSION: u32 = 1;
/// Exact byte length of the authenticated deployment commitment.
pub const BUNDLE_ADMISSION_RECORD_BYTES: usize =
    MAGIC.len() + 4 + CANONICAL_DEPLOYMENT_BUNDLE_BYTES + 2 * MANIFEST_COMMITMENT_BYTES;

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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleAdmissionRecord {
    bytes: [u8; BUNDLE_ADMISSION_RECORD_BYTES],
    record_id: Identity,
}

impl BundleAdmissionRecord {
    /// Returns the complete canonical bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; BUNDLE_ADMISSION_RECORD_BYTES] {
        &self.bytes
    }

    /// Returns the domain-separated record identity.
    #[must_use]
    pub const fn record_id(&self) -> Identity {
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
    /// Returns the retained exact deployment and prepacked manifests.
    #[must_use]
    pub const fn prepacked(&self) -> &PrepackedDeploymentBundle {
        &self.prepacked
    }

    /// Returns the canonical commitment record.
    #[must_use]
    pub const fn record(&self) -> &BundleAdmissionRecord {
        &self.record
    }

    pub(crate) fn into_parts(self) -> (PrepackedDeploymentBundle, BundleAdmissionRecord) {
        (self.prepacked, self.record)
    }
}

/// Failure while sealing or decoding an authenticated bundle commitment.
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

/// Consumes exact authenticated prepacked inputs and seals their canonical
/// deployment commitment.
///
/// # Errors
///
/// Returns [`BundleAdmissionError`] unless both manifests remain complete,
/// role-correct, digest-bound, gap-free, and semantically exact for the
/// canonical deployment. Success is not signature, artifact-load, launch, or
/// independent-validation authority.
pub fn seal_authenticated_bundle(
    prepacked: PrepackedDeploymentBundle,
) -> Result<AuthenticatedBundleAdmission, BundleAdmissionError> {
    let deployment_record = encode_canonical_deployment_bundle(prepacked.deployment())?;
    let target = validate_manifest(
        prepacked.target_manifest(),
        prepacked.deployment(),
        Qwen3ModelRole::Target8B,
    )?;
    let draft = validate_manifest(
        prepacked.draft_manifest(),
        prepacked.deployment(),
        Qwen3ModelRole::Draft06B,
    )?;
    let record = encode_record(deployment_record.as_bytes(), target, draft);
    Ok(AuthenticatedBundleAdmission { prepacked, record })
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
) -> Result<BundleAdmissionDescriptor, BundleAdmissionError> {
    if bytes.len() != BUNDLE_ADMISSION_RECORD_BYTES {
        return Err(BundleAdmissionError::InvalidLength);
    }
    let mut reader = Reader::new(bytes);
    if reader.array::<16>() != MAGIC {
        return Err(BundleAdmissionError::InvalidMagic);
    }
    if reader.u32() != BUNDLE_ADMISSION_RECORD_VERSION {
        return Err(BundleAdmissionError::InvalidVersion);
    }
    let bundle_bytes = reader.array::<CANONICAL_DEPLOYMENT_BUNDLE_BYTES>();
    let deployment = decode_canonical_deployment_bundle(&bundle_bytes)?;
    let target_manifest = reader.commitment(Qwen3ModelRole::Target8B)?;
    let draft_manifest = reader.commitment(Qwen3ModelRole::Draft06B)?;
    validate_commitment(&deployment, target_manifest)?;
    validate_commitment(&deployment, draft_manifest)?;
    let record = encode_record(&bundle_bytes, target_manifest, draft_manifest);
    if record.bytes.as_slice() != bytes {
        return Err(BundleAdmissionError::InvalidLength);
    }
    Ok(BundleAdmissionDescriptor {
        deployment,
        target_manifest,
        draft_manifest,
        record_id: record.record_id,
    })
}

fn validate_manifest(
    manifest: &WeightSectionManifest,
    deployment: &DeploymentBundle,
    role: Qwen3ModelRole,
) -> Result<ManifestCommitment, BundleAdmissionError> {
    let model = match role {
        Qwen3ModelRole::Target8B => deployment.target_model,
        Qwen3ModelRole::Draft06B => deployment.draft_model,
    };
    let invalid = |reason| BundleAdmissionError::InvalidManifest { role, reason };
    if manifest.version() != PREPACKED_WEIGHT_MANIFEST_VERSION {
        return Err(invalid("version"));
    }
    if manifest.role() != role {
        return Err(invalid("role"));
    }
    if manifest.source_weights_id() != *model.weights.weights_id.as_bytes() {
        return Err(invalid("source identity"));
    }
    if manifest.source_artifact_bytes() != model.weights.total_bytes {
        return Err(invalid("source byte count"));
    }
    if manifest.tensor_data_bytes() != role.tensor_data_bytes()
        || manifest.output_bytes() != manifest.tensor_data_bytes()
    {
        return Err(invalid("tensor byte count"));
    }
    if manifest.sections().len() != role.tensor_count() as usize {
        return Err(invalid("section count"));
    }
    if manifest.canonical_bytes().is_empty()
        || manifest.canonical_bytes().len() > MAX_MANIFEST_RECORD_BYTES
        || sha256::digest(manifest.canonical_bytes()) != manifest.aggregate_id()
    {
        return Err(invalid("canonical manifest digest"));
    }

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

    Ok(ManifestCommitment {
        role,
        version: manifest.version(),
        source_weights_id: manifest.source_weights_id(),
        aggregate_id: manifest.aggregate_id(),
        source_artifact_bytes: manifest.source_artifact_bytes(),
        tensor_data_bytes: manifest.tensor_data_bytes(),
        output_bytes: manifest.output_bytes(),
        section_count: u32::try_from(manifest.sections().len()).unwrap_or(u32::MAX),
        canonical_manifest_bytes: u32::try_from(manifest.canonical_bytes().len())
            .unwrap_or(u32::MAX),
    })
}

fn validate_commitment(
    deployment: &DeploymentBundle,
    commitment: ManifestCommitment,
) -> Result<(), BundleAdmissionError> {
    let role = commitment.role;
    let model = match role {
        Qwen3ModelRole::Target8B => deployment.target_model,
        Qwen3ModelRole::Draft06B => deployment.draft_model,
    };
    let invalid = |reason| BundleAdmissionError::InvalidCommitment { role, reason };
    if commitment.version != PREPACKED_WEIGHT_MANIFEST_VERSION {
        return Err(invalid("version"));
    }
    if commitment.source_weights_id != *model.weights.weights_id.as_bytes()
        || commitment.aggregate_id == [0; 32]
    {
        return Err(invalid("identity"));
    }
    if commitment.source_artifact_bytes != model.weights.total_bytes
        || commitment.tensor_data_bytes != role.tensor_data_bytes()
        || commitment.output_bytes != commitment.tensor_data_bytes
    {
        return Err(invalid("byte count"));
    }
    if commitment.section_count != role.tensor_count()
        || commitment.canonical_manifest_bytes == 0
        || commitment.canonical_manifest_bytes as usize > MAX_MANIFEST_RECORD_BYTES
    {
        return Err(invalid("manifest bound"));
    }
    Ok(())
}

fn encode_record(
    bundle: &[u8; CANONICAL_DEPLOYMENT_BUNDLE_BYTES],
    target: ManifestCommitment,
    draft: ManifestCommitment,
) -> BundleAdmissionRecord {
    let mut writer = Writer::new();
    writer.bytes(&MAGIC);
    writer.u32(BUNDLE_ADMISSION_RECORD_VERSION);
    writer.bytes(bundle);
    writer.commitment(target);
    writer.commitment(draft);
    let mut identity_bytes = Vec::with_capacity(RECORD_DOMAIN.len() + writer.bytes.len());
    identity_bytes.extend_from_slice(RECORD_DOMAIN);
    identity_bytes.extend_from_slice(&writer.bytes);
    BundleAdmissionRecord {
        bytes: writer.bytes,
        record_id: Identity::new(sha256::digest(&identity_bytes)),
    }
}

struct Writer {
    bytes: [u8; BUNDLE_ADMISSION_RECORD_BYTES],
    offset: usize,
}

impl Writer {
    const fn new() -> Self {
        Self {
            bytes: [0; BUNDLE_ADMISSION_RECORD_BYTES],
            offset: 0,
        }
    }

    fn bytes(&mut self, value: &[u8]) {
        let end = self.offset + value.len();
        self.bytes[self.offset..end].copy_from_slice(value);
        self.offset = end;
    }

    fn u8(&mut self, value: u8) {
        self.bytes(&[value]);
    }

    fn u32(&mut self, value: u32) {
        self.bytes(&value.to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes(&value.to_le_bytes());
    }

    fn commitment(&mut self, value: ManifestCommitment) {
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
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn array<const N: usize>(&mut self) -> [u8; N] {
        let end = self.offset + N;
        let mut value = [0; N];
        value.copy_from_slice(&self.bytes[self.offset..end]);
        self.offset = end;
        value
    }

    fn u8(&mut self) -> u8 {
        self.array::<1>()[0]
    }

    fn u32(&mut self) -> u32 {
        u32::from_le_bytes(self.array())
    }

    fn u64(&mut self) -> u64 {
        u64::from_le_bytes(self.array())
    }

    fn commitment(
        &mut self,
        expected_role: Qwen3ModelRole,
    ) -> Result<ManifestCommitment, BundleAdmissionError> {
        let role = match self.u8() {
            ROLE_TARGET => Qwen3ModelRole::Target8B,
            ROLE_DRAFT => Qwen3ModelRole::Draft06B,
            _ => {
                return Err(BundleAdmissionError::InvalidCommitment {
                    role: expected_role,
                    reason: "role tag",
                });
            }
        };
        if role != expected_role {
            return Err(BundleAdmissionError::InvalidCommitment {
                role: expected_role,
                reason: "role order",
            });
        }
        Ok(ManifestCommitment {
            role,
            version: self.u32(),
            source_weights_id: self.array(),
            aggregate_id: self.array(),
            source_artifact_bytes: self.u64(),
            tensor_data_bytes: self.u64(),
            output_bytes: self.u64(),
            section_count: self.u32(),
            canonical_manifest_bytes: self.u32(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        decode_bundle_admission_record, encode_record, BundleAdmissionError, ManifestCommitment,
        BUNDLE_ADMISSION_RECORD_BYTES, BUNDLE_ADMISSION_RECORD_VERSION,
    };
    use crate::{
        bundle::tests::exact_bundle,
        encode_canonical_deployment_bundle, seal_authenticated_bundle,
        tokenizer::tests::{authenticated_assets, test_tokenizer},
        weight_stream::tests::test_prepacked,
        PREPACKED_WEIGHT_MANIFEST_VERSION,
    };
    use ferric_spec::Qwen3ModelRole;

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

    #[test]
    fn canonical_descriptor_round_trips_and_is_identity_sensitive() {
        let record = record();
        assert_eq!(record.as_bytes().len(), BUNDLE_ADMISSION_RECORD_BYTES);
        let decoded = decode_bundle_admission_record(record.as_bytes()).expect("canonical record");
        assert_eq!(decoded.record_id, record.record_id());
        assert_eq!(decoded.target_manifest.aggregate_id, [0x31; 32]);
        assert_eq!(decoded.draft_manifest.aggregate_id, [0x32; 32]);

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
    fn sealed_authority_consumes_complete_official_tensor_rosters() {
        let prepacked = crate::build_prepacked_deployment_bundle(
            authenticated_assets(),
            test_tokenizer(Qwen3ModelRole::Target8B),
            test_tokenizer(Qwen3ModelRole::Draft06B),
            test_prepacked(Qwen3ModelRole::Target8B),
            test_prepacked(Qwen3ModelRole::Draft06B),
        )
        .expect("complete test prepacked deployment");
        let authority = seal_authenticated_bundle(prepacked).expect("sealed admission authority");
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
        assert_eq!(
            decode_bundle_admission_record(&record.as_bytes()[..BUNDLE_ADMISSION_RECORD_BYTES - 1]),
            Err(BundleAdmissionError::InvalidLength)
        );
        let mut trailing = record.as_bytes().to_vec();
        trailing.push(0);
        assert_eq!(
            decode_bundle_admission_record(&trailing),
            Err(BundleAdmissionError::InvalidLength)
        );
        let mut version = record.as_bytes().to_owned();
        version[16..20].copy_from_slice(&(BUNDLE_ADMISSION_RECORD_VERSION + 1).to_le_bytes());
        assert_eq!(
            decode_bundle_admission_record(&version),
            Err(BundleAdmissionError::InvalidVersion)
        );
        let mut role = record.as_bytes().to_owned();
        role[20 + crate::CANONICAL_DEPLOYMENT_BUNDLE_BYTES] = 9;
        assert!(matches!(
            decode_bundle_admission_record(&role),
            Err(BundleAdmissionError::InvalidCommitment {
                reason: "role tag",
                ..
            })
        ));
        let mut zero = record.as_bytes().to_owned();
        let aggregate_start = 20 + crate::CANONICAL_DEPLOYMENT_BUNDLE_BYTES + 1 + 4 + 32;
        zero[aggregate_start..aggregate_start + 32].fill(0);
        assert!(matches!(
            decode_bundle_admission_record(&zero),
            Err(BundleAdmissionError::InvalidCommitment {
                reason: "identity",
                ..
            })
        ));
    }
}
