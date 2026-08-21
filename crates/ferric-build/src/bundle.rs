use super::{
    bundle_identity, model_identity, DRAFT_CONFIG_SHA256, DRAFT_REPOSITORY, DRAFT_REVISION,
    QWEN3_DRAFT_MODEL_ID, QWEN3_DRAFT_TENSOR_DATA_BYTES, QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES,
    QWEN3_DRAFT_WEIGHT_SHA256, QWEN3_TARGET_MODEL_ID, QWEN3_TARGET_TENSOR_DATA_BYTES,
    QWEN3_TARGET_WEIGHT_ARTIFACT_BYTES, QWEN3_TARGET_WEIGHT_SET_SHA256, QWEN3_TOKENIZER_SHA256,
    TARGET_CONFIG_SHA256, TARGET_REPOSITORY, TARGET_REVISION, TOKENIZER_METADATA_SHA256,
};
use ferric_spec::{
    DeploymentBundle, EngineLimits, Identity, ModelArtifact, ModelConfig, NumericalPolicy,
    Qwen3ModelRole, SpecError, Target, TokenizerConfig, WeightManifest,
};
use std::fmt;

const MAGIC: [u8; 16] = *b"FERRIC-M1-BUNDLE";
const TARGET_GFX942_XNACK_MINUS: u8 = 1;
const NUMERICAL_BF16_FP32: u8 = 1;
const ROLE_TARGET_8B: u8 = 1;
const ROLE_DRAFT_06B: u8 = 2;

/// Version of the fixed-width canonical M1 deployment-bundle record.
pub const CANONICAL_DEPLOYMENT_BUNDLE_VERSION: u32 = 1;
/// Exact byte length of a canonical M1 deployment-bundle record.
pub const CANONICAL_DEPLOYMENT_BUNDLE_BYTES: usize = 522;

/// A fixed-width canonical record for the exact first M1 model pair.
///
/// This value binds already-admitted identities and geometry. It does not
/// authenticate the external files named by those identities and is not a
/// signature or a device-load authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalDeploymentBundle {
    bytes: [u8; CANONICAL_DEPLOYMENT_BUNDLE_BYTES],
}

impl CanonicalDeploymentBundle {
    /// Returns the complete canonical record bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; CANONICAL_DEPLOYMENT_BUNDLE_BYTES] {
        &self.bytes
    }
}

/// Failure while encoding or decoding the fixed M1 bundle record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CanonicalBundleError {
    /// The record is truncated or has trailing bytes.
    InvalidLength,
    /// The fixed format discriminator is incorrect.
    InvalidMagic,
    /// The fixed format version is unsupported.
    InvalidVersion,
    /// An enum discriminator is not canonical for M1.
    InvalidTag(&'static str),
    /// A boolean byte is not the canonical zero or one representation.
    InvalidBoolean(&'static str),
    /// A field differs from the exact pinned Qwen3 pair.
    PinnedFieldMismatch(&'static str),
    /// The executable sequential bundle contract rejected the decoded value.
    Spec(SpecError),
}

impl fmt::Display for CanonicalBundleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength => formatter.write_str("canonical bundle has the wrong length"),
            Self::InvalidMagic => formatter.write_str("canonical bundle magic is invalid"),
            Self::InvalidVersion => formatter.write_str("canonical bundle version is unsupported"),
            Self::InvalidTag(field) => write!(formatter, "canonical bundle tag {field} is invalid"),
            Self::InvalidBoolean(field) => {
                write!(formatter, "canonical bundle boolean {field} is invalid")
            }
            Self::PinnedFieldMismatch(field) => {
                write!(formatter, "canonical bundle field {field} is not pinned")
            }
            Self::Spec(error) => write!(formatter, "canonical bundle is invalid: {error}"),
        }
    }
}

impl std::error::Error for CanonicalBundleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Spec(error) => Some(error),
            _ => None,
        }
    }
}

/// Encodes one exact admitted Qwen3 target/draft bundle.
///
/// # Errors
///
/// Returns [`CanonicalBundleError`] when any identity, geometry, tokenizer,
/// weight descriptor, limit, or derived model/bundle identity is not exact.
pub fn encode_canonical_deployment_bundle(
    bundle: &DeploymentBundle,
) -> Result<CanonicalDeploymentBundle, CanonicalBundleError> {
    validate_exact_bundle(bundle)?;
    let mut writer = Writer::new();
    writer.bytes(&MAGIC);
    writer.u32(CANONICAL_DEPLOYMENT_BUNDLE_VERSION);
    writer.identity(bundle.bundle_id);
    writer.u8(TARGET_GFX942_XNACK_MINUS);
    writer.u8(NUMERICAL_BF16_FP32);
    writer.limits(bundle.limits);
    writer.model(bundle.target_model);
    writer.model(bundle.draft_model);
    Ok(CanonicalDeploymentBundle {
        bytes: writer.bytes,
    })
}

/// Decodes and revalidates one exact fixed-width Qwen3 bundle record.
///
/// # Errors
///
/// Returns [`CanonicalBundleError`] for truncation, trailing bytes,
/// noncanonical scalar encodings, pin drift, or derived-identity mismatch.
pub fn decode_canonical_deployment_bundle(
    bytes: &[u8],
) -> Result<DeploymentBundle, CanonicalBundleError> {
    if bytes.len() != CANONICAL_DEPLOYMENT_BUNDLE_BYTES {
        return Err(CanonicalBundleError::InvalidLength);
    }
    let mut reader = Reader::new(bytes);
    if reader.array::<16>() != MAGIC {
        return Err(CanonicalBundleError::InvalidMagic);
    }
    if reader.u32() != CANONICAL_DEPLOYMENT_BUNDLE_VERSION {
        return Err(CanonicalBundleError::InvalidVersion);
    }
    let bundle_id = reader.identity();
    if reader.u8() != TARGET_GFX942_XNACK_MINUS {
        return Err(CanonicalBundleError::InvalidTag("target"));
    }
    if reader.u8() != NUMERICAL_BF16_FP32 {
        return Err(CanonicalBundleError::InvalidTag("numerical_policy"));
    }
    let limits = reader.limits();
    let target_model = reader.model("target_model")?;
    let draft_model = reader.model("draft_model")?;
    let bundle = DeploymentBundle {
        bundle_id,
        target: Target::Gfx942XnackMinus,
        numerical_policy: NumericalPolicy::Bf16ParametersFp32Accumulation,
        limits,
        target_model,
        draft_model,
    };
    validate_exact_bundle(&bundle)?;
    let canonical = encode_canonical_deployment_bundle(&bundle)?;
    if canonical.as_bytes().as_slice() != bytes {
        return Err(CanonicalBundleError::PinnedFieldMismatch(
            "canonical_encoding",
        ));
    }
    Ok(bundle)
}

fn validate_exact_bundle(bundle: &DeploymentBundle) -> Result<(), CanonicalBundleError> {
    bundle.validate().map_err(CanonicalBundleError::Spec)?;
    validate_model(bundle.target_model, Qwen3ModelRole::Target8B)?;
    validate_model(bundle.draft_model, Qwen3ModelRole::Draft06B)?;
    let expected_bundle_id =
        bundle_identity(bundle.limits, bundle.target_model, bundle.draft_model);
    if bundle.bundle_id.as_bytes() != expected_bundle_id.as_bytes() {
        return Err(CanonicalBundleError::PinnedFieldMismatch("bundle_id"));
    }
    Ok(())
}

fn validate_model(model: ModelArtifact, role: Qwen3ModelRole) -> Result<(), CanonicalBundleError> {
    let (
        config_id,
        model_id,
        repository,
        revision,
        weight_id,
        artifact_bytes,
        tensor_bytes,
        sections,
    ) = match role {
        Qwen3ModelRole::Target8B => (
            TARGET_CONFIG_SHA256,
            QWEN3_TARGET_MODEL_ID,
            TARGET_REPOSITORY,
            TARGET_REVISION,
            QWEN3_TARGET_WEIGHT_SET_SHA256,
            QWEN3_TARGET_WEIGHT_ARTIFACT_BYTES,
            QWEN3_TARGET_TENSOR_DATA_BYTES,
            5,
        ),
        Qwen3ModelRole::Draft06B => (
            DRAFT_CONFIG_SHA256,
            QWEN3_DRAFT_MODEL_ID,
            DRAFT_REPOSITORY,
            DRAFT_REVISION,
            QWEN3_DRAFT_WEIGHT_SHA256,
            QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES,
            QWEN3_DRAFT_TENSOR_DATA_BYTES,
            1,
        ),
    };
    exact_identity(model.config.config_id, config_id, "config_id")?;
    exact_identity(model.config.model_id, model_id, "model_id")?;
    exact_identity(
        model.tokenizer.tokenizer_id,
        TOKENIZER_METADATA_SHA256,
        "tokenizer_id",
    )?;
    exact_identity(
        model.tokenizer.vocabulary_id,
        QWEN3_TOKENIZER_SHA256,
        "vocabulary_id",
    )?;
    exact_identity(model.weights.weights_id, weight_id, "weights_id")?;
    if model.weights.total_bytes != artifact_bytes {
        return Err(CanonicalBundleError::PinnedFieldMismatch("weight_bytes"));
    }
    if model.weights.sections != sections {
        return Err(CanonicalBundleError::PinnedFieldMismatch("weight_sections"));
    }
    let expected_model_id = model_identity(
        role,
        repository,
        revision,
        model.config.config_id,
        model.tokenizer,
        model.weights,
        tensor_bytes,
    );
    if model.config.model_id.as_bytes() != expected_model_id.as_bytes() {
        return Err(CanonicalBundleError::PinnedFieldMismatch(
            "derived_model_id",
        ));
    }
    Ok(())
}

fn exact_identity(
    actual: Identity,
    expected: [u8; 32],
    field: &'static str,
) -> Result<(), CanonicalBundleError> {
    if actual.as_bytes() != &expected {
        return Err(CanonicalBundleError::PinnedFieldMismatch(field));
    }
    Ok(())
}

struct Writer {
    bytes: [u8; CANONICAL_DEPLOYMENT_BUNDLE_BYTES],
    offset: usize,
}

impl Writer {
    fn new() -> Self {
        Self {
            bytes: [0; CANONICAL_DEPLOYMENT_BUNDLE_BYTES],
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

    fn bool(&mut self, value: bool) {
        self.u8(u8::from(value));
    }

    fn u32(&mut self, value: u32) {
        self.bytes(&value.to_be_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes(&value.to_be_bytes());
    }

    fn identity(&mut self, value: Identity) {
        self.bytes(value.as_bytes());
    }

    fn limits(&mut self, value: EngineLimits) {
        self.u32(value.max_context_tokens);
        self.u32(value.max_active_sequences);
        self.u32(value.kv_page_tokens);
        self.u32(value.max_draft_tokens);
    }

    fn model(&mut self, value: ModelArtifact) {
        self.u8(match value.config.role {
            Qwen3ModelRole::Target8B => ROLE_TARGET_8B,
            Qwen3ModelRole::Draft06B => ROLE_DRAFT_06B,
        });
        self.identity(value.config.model_id);
        self.identity(value.config.config_id);
        self.u32(value.config.vocabulary_size);
        self.u32(value.config.layers);
        self.u32(value.config.hidden_size);
        self.u32(value.config.intermediate_size);
        self.u32(value.config.query_heads);
        self.u32(value.config.kv_heads);
        self.u32(value.config.head_dim);
        self.u32(value.config.max_position_embeddings);
        self.u32(value.config.rope_theta);
        self.bool(value.config.tie_word_embeddings);
        self.identity(value.tokenizer.tokenizer_id);
        self.identity(value.tokenizer.vocabulary_id);
        self.u32(value.tokenizer.vocabulary_size);
        self.u32(value.tokenizer.end_of_text_token);
        self.u32(value.tokenizer.im_start_token);
        self.u32(value.tokenizer.im_end_token);
        self.identity(value.weights.weights_id);
        self.u64(value.weights.total_bytes);
        self.u32(value.weights.sections);
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
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

    fn bool(&mut self, field: &'static str) -> Result<bool, CanonicalBundleError> {
        match self.u8() {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(CanonicalBundleError::InvalidBoolean(field)),
        }
    }

    fn u32(&mut self) -> u32 {
        u32::from_be_bytes(self.array())
    }

    fn u64(&mut self) -> u64 {
        u64::from_be_bytes(self.array())
    }

    fn identity(&mut self) -> Identity {
        Identity::new(self.array())
    }

    fn limits(&mut self) -> EngineLimits {
        EngineLimits {
            max_context_tokens: self.u32(),
            max_active_sequences: self.u32(),
            kv_page_tokens: self.u32(),
            max_draft_tokens: self.u32(),
        }
    }

    fn model(&mut self, field: &'static str) -> Result<ModelArtifact, CanonicalBundleError> {
        let role = match self.u8() {
            ROLE_TARGET_8B => Qwen3ModelRole::Target8B,
            ROLE_DRAFT_06B => Qwen3ModelRole::Draft06B,
            _ => return Err(CanonicalBundleError::InvalidTag(field)),
        };
        let config = ModelConfig {
            role,
            model_id: self.identity(),
            config_id: self.identity(),
            vocabulary_size: self.u32(),
            layers: self.u32(),
            hidden_size: self.u32(),
            intermediate_size: self.u32(),
            query_heads: self.u32(),
            kv_heads: self.u32(),
            head_dim: self.u32(),
            max_position_embeddings: self.u32(),
            rope_theta: self.u32(),
            tie_word_embeddings: self.bool("tie_word_embeddings")?,
        };
        let tokenizer = TokenizerConfig {
            tokenizer_id: self.identity(),
            vocabulary_id: self.identity(),
            vocabulary_size: self.u32(),
            end_of_text_token: self.u32(),
            im_start_token: self.u32(),
            im_end_token: self.u32(),
        };
        let weights = WeightManifest {
            weights_id: self.identity(),
            total_bytes: self.u64(),
            sections: self.u32(),
        };
        Ok(ModelArtifact {
            config,
            tokenizer,
            weights,
        })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::{
        decode_canonical_deployment_bundle, encode_canonical_deployment_bundle,
        CanonicalBundleError, CANONICAL_DEPLOYMENT_BUNDLE_BYTES,
    };
    use crate::{
        build_preliminary_deployment_bundle, ArtifactDigest, DeploymentAssets, ModelAssets,
        WeightDescriptor, DRAFT_REPOSITORY, DRAFT_REVISION, QWEN3_DRAFT_TENSOR_DATA_BYTES,
        QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES, QWEN3_DRAFT_WEIGHT_SHA256,
        QWEN3_TARGET_TENSOR_DATA_BYTES, QWEN3_TARGET_WEIGHT_ARTIFACT_BYTES,
        QWEN3_TARGET_WEIGHT_SET_SHA256, QWEN3_TOKENIZER_BYTES, QWEN3_TOKENIZER_SHA256,
        TARGET_REPOSITORY, TARGET_REVISION,
    };
    use ferric_spec::EngineLimits;

    const TARGET_CONFIG: &[u8] = include_bytes!("fixtures/qwen3-8b-config.json");
    const DRAFT_CONFIG: &[u8] = include_bytes!("fixtures/qwen3-06b-config.json");
    const TOKENIZER_METADATA: &[u8] = include_bytes!("fixtures/qwen3-tokenizer-config.json");

    fn pinned_config(bytes: &'static [u8]) -> &'static [u8] {
        assert_eq!(bytes.last(), Some(&b'\n'));
        &bytes[..bytes.len() - 1]
    }

    fn model_assets(
        repository: &'static str,
        revision: &'static str,
        config_json: &'static [u8],
        weights: WeightDescriptor,
    ) -> ModelAssets<'static> {
        ModelAssets {
            repository,
            revision,
            config_json,
            tokenizer_metadata_json: TOKENIZER_METADATA,
            vocabulary: ArtifactDigest {
                sha256: QWEN3_TOKENIZER_SHA256,
                byte_len: QWEN3_TOKENIZER_BYTES,
            },
            weights,
        }
    }

    pub(crate) fn exact_bundle() -> ferric_spec::DeploymentBundle {
        build_preliminary_deployment_bundle(DeploymentAssets {
            target: model_assets(
                TARGET_REPOSITORY,
                TARGET_REVISION,
                pinned_config(TARGET_CONFIG),
                WeightDescriptor {
                    weights_id: QWEN3_TARGET_WEIGHT_SET_SHA256,
                    artifact_bytes: QWEN3_TARGET_WEIGHT_ARTIFACT_BYTES,
                    tensor_data_bytes: QWEN3_TARGET_TENSOR_DATA_BYTES,
                    sections: 5,
                },
            ),
            draft: model_assets(
                DRAFT_REPOSITORY,
                DRAFT_REVISION,
                pinned_config(DRAFT_CONFIG),
                WeightDescriptor {
                    weights_id: QWEN3_DRAFT_WEIGHT_SHA256,
                    artifact_bytes: QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES,
                    tensor_data_bytes: QWEN3_DRAFT_TENSOR_DATA_BYTES,
                    sections: 1,
                },
            ),
            limits: EngineLimits {
                max_context_tokens: 8_192,
                max_active_sequences: 32,
                kv_page_tokens: 16,
                max_draft_tokens: 8,
            },
        })
        .expect("exact preliminary bundle")
    }

    #[test]
    fn exact_bundle_round_trips_canonically() {
        let bundle = exact_bundle();
        let encoded = encode_canonical_deployment_bundle(&bundle).expect("canonical encoding");
        assert_eq!(encoded.as_bytes().len(), CANONICAL_DEPLOYMENT_BUNDLE_BYTES);
        assert_eq!(
            decode_canonical_deployment_bundle(encoded.as_bytes()),
            Ok(bundle)
        );
        assert_eq!(
            encode_canonical_deployment_bundle(
                &decode_canonical_deployment_bundle(encoded.as_bytes()).expect("canonical decode")
            )
            .expect("canonical re-encode"),
            encoded
        );
    }

    #[test]
    fn truncation_and_trailing_bytes_are_rejected() {
        let encoded = encode_canonical_deployment_bundle(&exact_bundle()).expect("encoding");
        assert_eq!(
            decode_canonical_deployment_bundle(
                &encoded.as_bytes()[..CANONICAL_DEPLOYMENT_BUNDLE_BYTES - 1]
            ),
            Err(CanonicalBundleError::InvalidLength)
        );
        let mut trailing = encoded.as_bytes().to_vec();
        trailing.push(0);
        assert_eq!(
            decode_canonical_deployment_bundle(&trailing),
            Err(CanonicalBundleError::InvalidLength)
        );
    }

    #[test]
    fn every_single_byte_drift_is_rejected() {
        let encoded = encode_canonical_deployment_bundle(&exact_bundle()).expect("encoding");
        for index in 0..CANONICAL_DEPLOYMENT_BUNDLE_BYTES {
            let mut changed = encoded.as_bytes().to_owned();
            changed[index] ^= 1;
            assert!(
                decode_canonical_deployment_bundle(&changed).is_err(),
                "byte {index} was not identity-sensitive"
            );
        }
    }

    #[test]
    fn descriptor_values_cannot_claim_canonical_authority() {
        let mut bundle = exact_bundle();
        bundle.target_model.weights.total_bytes -= 1;
        assert_eq!(
            encode_canonical_deployment_bundle(&bundle),
            Err(CanonicalBundleError::PinnedFieldMismatch("weight_bytes"))
        );
    }
}
