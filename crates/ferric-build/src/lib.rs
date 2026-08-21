#![forbid(unsafe_code)]

//! Offline admission of the pinned Qwen3 M1 deployment pair.
//!
//! This crate owns byte parsing and identity construction. The resulting
//! value is still admitted by the executable contract in `ferric-spec`.
//! Configuration, tokenizer metadata, tokenizer vocabulary, and safetensors
//! authentication remain separate sealed stages until the final bundle path
//! consumes their authorities.

mod bundle;
mod json;
mod plan;
mod safetensors;
mod sha256;
mod tokenizer;
mod weight_stream;

pub use bundle::{
    decode_canonical_deployment_bundle, encode_canonical_deployment_bundle, CanonicalBundleError,
    CanonicalDeploymentBundle, CANONICAL_DEPLOYMENT_BUNDLE_BYTES,
    CANONICAL_DEPLOYMENT_BUNDLE_VERSION,
};
pub use plan::{
    build_sequential_plan_catalog, SequentialPlanCatalog, SequentialPlanError,
    SEQUENTIAL_PLAN_CATALOG_ENTRIES, SEQUENTIAL_PLAN_CATALOG_VERSION,
};
pub use safetensors::{
    authenticate_qwen3_draft_weights, authenticate_qwen3_target_weights, AuthenticatedWeightSet,
    SafetensorsError, SafetensorsSource,
};
pub use tokenizer::{authenticate_qwen3_tokenizer, AuthenticatedTokenizer, TokenizerError};
pub use weight_stream::{
    prepack_qwen3_draft_weights, prepack_qwen3_target_weights, PrepackedWeightSet, WeightSection,
    WeightSectionManifest, WeightStreamError, WeightTransform, PREPACKED_WEIGHT_MANIFEST_VERSION,
};

use ferric_spec::{
    DeploymentBundle, EngineLimits, Identity, ModelArtifact, ModelConfig, NumericalPolicy,
    Qwen3ModelRole, SpecError, Target, TokenizerConfig, WeightManifest, QWEN3_END_OF_TEXT_TOKEN,
    QWEN3_IM_END_TOKEN, QWEN3_IM_START_TOKEN, QWEN3_VOCABULARY_SIZE,
};
use json::Value;
use sha256::Sha256;
use std::collections::BTreeMap;
use std::fmt;

const MAX_CONFIG_BYTES: usize = 16 * 1_024;
const MAX_TOKENIZER_METADATA_BYTES: usize = 64 * 1_024;
const MAX_CHAT_TEMPLATE_BYTES: usize = 32 * 1_024;

/// Pinned upstream repository for the M1 target model.
pub const TARGET_REPOSITORY: &str = "Qwen/Qwen3-8B";
/// Pinned upstream revision for the M1 target model.
pub const TARGET_REVISION: &str = "b968826d9c46dd6066d109eabc6255188de91218";
/// Pinned upstream repository for the M1 draft model.
pub const DRAFT_REPOSITORY: &str = "Qwen/Qwen3-0.6B";
/// Pinned upstream revision for the M1 draft model.
pub const DRAFT_REVISION: &str = "c1899de289a04d12100db370d81485cdf75e47ca";

/// Size of the shared upstream `tokenizer.json` payload.
pub const QWEN3_TOKENIZER_BYTES: u64 = 11_422_654;
/// Total file bytes across the five pinned Qwen3-8B safetensors shards.
pub const QWEN3_TARGET_WEIGHT_ARTIFACT_BYTES: u64 = 16_381_516_776;
/// Tensor-data bytes declared by the pinned Qwen3-8B safetensors index.
pub const QWEN3_TARGET_TENSOR_DATA_BYTES: u64 = 16_381_470_720;
/// Complete file bytes in the pinned Qwen3-0.6B safetensors artifact.
pub const QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES: u64 = 1_503_300_328;
/// Tensor-data bytes following the Qwen3-0.6B safetensors header.
pub const QWEN3_DRAFT_TENSOR_DATA_BYTES: u64 = 1_503_264_768;

const TARGET_CONFIG_SHA256: [u8; 32] =
    decode_hex_32(b"f7c4eadfbbf522470667b797a3c89be2524832d2d599797248dc304fff447c30");
const DRAFT_CONFIG_SHA256: [u8; 32] =
    decode_hex_32(b"660db3b73d788119c04535e48cf9be5f55bc3100841a718637ae695b442f27dd");
const TOKENIZER_METADATA_SHA256: [u8; 32] =
    decode_hex_32(b"d5d09f07b48c3086c508b30d1c9114bd1189145b74e982a265350c923acd8101");
/// SHA-256 identity of the shared upstream `tokenizer.json` LFS object.
pub const QWEN3_TOKENIZER_SHA256: [u8; 32] =
    decode_hex_32(b"aeb13307a71acd8fe81861d94ad54ab689df773318809eed3cbe794b4492dae4");
/// Canonical target weight-set descriptor identity.
///
/// This is the domain-separated, length-prefixed SHA-256 record over the
/// pinned index name/SHA-256/size followed by all five ordered shard
/// name/SHA-256/size tuples. It is not a hash of concatenated weight bytes.
pub const QWEN3_TARGET_WEIGHT_SET_SHA256: [u8; 32] =
    decode_hex_32(b"2e69c089ff9afcee264646cb8ea6344aa3c8cedbe8022d729889708204e32732");
/// SHA-256 of the complete pinned Qwen3-0.6B safetensors file.
pub const QWEN3_DRAFT_WEIGHT_SHA256: [u8; 32] =
    decode_hex_32(b"f47f71177f32bcd101b7573ec9171e6a57f4f4d31148d38e382306f42996874b");
/// Domain-separated identity of the exact admitted Qwen3-8B model inputs.
pub const QWEN3_TARGET_MODEL_ID: [u8; 32] =
    decode_hex_32(b"f18fc461576d1a3053a19aba5946ef7b3b45aaf7cbb45d77f5c276f18567224a");
/// Domain-separated identity of the exact admitted Qwen3-0.6B model inputs.
pub const QWEN3_DRAFT_MODEL_ID: [u8; 32] =
    decode_hex_32(b"351fc121a569f0a53e9bb5c98caaeff80d6f8d94737eecf5e179cfa54d9cf998");

const TARGET_WEIGHT_SET_COMPONENTS: [(&str, [u8; 32], u64); 6] = [
    (
        "model.safetensors.index.json",
        decode_hex_32(b"f9fdbcb91c23971c13ec5d5f2573d2349e8f61f2f049371ec699281748fdb1bc"),
        32_878,
    ),
    (
        "model-00001-of-00005.safetensors",
        decode_hex_32(b"31d6a825ae35f11fb85b195b4c42c146c051e446433125a215336abdf95cbf5f"),
        3_996_250_744,
    ),
    (
        "model-00002-of-00005.safetensors",
        decode_hex_32(b"5991236cea6fe21f3d43cab0f0e84448734fbbe0789816202989f2ddc9d18282"),
        3_993_160_032,
    ),
    (
        "model-00003-of-00005.safetensors",
        decode_hex_32(b"c5185c4794be2d8a9784d5753c9922db38df478ce11f9ed0b415b7304d896836"),
        3_959_604_768,
    ),
    (
        "model-00004-of-00005.safetensors",
        decode_hex_32(b"b5ee7de71fbf17db3d5704e0c8f2bc7d005ca9e1d7ca2aeb19827b0cfcaa917a"),
        3_187_841_392,
    ),
    (
        "model-00005-of-00005.safetensors",
        decode_hex_32(b"20c2d6366ab85c90786ccdd829cd2b9e7d30ef3b2ebbb998280e7e4014b542ff"),
        1_244_659_840,
    ),
];

const ADDED_TOKENS: [(&str, bool); 26] = [
    ("<|endoftext|>", true),
    ("<|im_start|>", true),
    ("<|im_end|>", true),
    ("<|object_ref_start|>", true),
    ("<|object_ref_end|>", true),
    ("<|box_start|>", true),
    ("<|box_end|>", true),
    ("<|quad_start|>", true),
    ("<|quad_end|>", true),
    ("<|vision_start|>", true),
    ("<|vision_end|>", true),
    ("<|vision_pad|>", true),
    ("<|image_pad|>", true),
    ("<|video_pad|>", true),
    ("<tool_call>", false),
    ("</tool_call>", false),
    ("<|fim_prefix|>", false),
    ("<|fim_middle|>", false),
    ("<|fim_suffix|>", false),
    ("<|fim_pad|>", false),
    ("<|repo_name|>", false),
    ("<|file_sep|>", false),
    ("<tool_response>", false),
    ("</tool_response>", false),
    ("<think>", false),
    ("</think>", false),
];

const ADDITIONAL_SPECIAL_TOKENS: [&str; 13] = [
    "<|im_start|>",
    "<|im_end|>",
    "<|object_ref_start|>",
    "<|object_ref_end|>",
    "<|box_start|>",
    "<|box_end|>",
    "<|quad_start|>",
    "<|quad_end|>",
    "<|vision_start|>",
    "<|vision_end|>",
    "<|vision_pad|>",
    "<|image_pad|>",
    "<|video_pad|>",
];

/// A caller-asserted identity and exact byte length for an opaque artifact.
///
/// The bundle builder compares vocabulary descriptors to the pinned Qwen3
/// identity. This type does not establish that a caller authenticated the
/// corresponding bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArtifactDigest {
    /// SHA-256 of the complete artifact bytes.
    pub sha256: [u8; 32],
    /// Exact length of the complete artifact bytes.
    pub byte_len: u64,
}

/// Opaque weight metadata supplied by a later safetensors authentication stage.
///
/// This slice compares every field to the pinned official descriptor. It does
/// not read, hash, or semantically parse the corresponding safetensors bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WeightDescriptor {
    /// Canonical identity of the pinned weight set.
    ///
    /// For the target this is a domain-separated record over the exact index
    /// and ordered shard descriptors, not a digest of concatenated bytes. For
    /// the single-file draft it equals the complete-file SHA-256.
    pub weights_id: [u8; 32],
    /// Complete safetensors file bytes, including all headers.
    pub artifact_bytes: u64,
    /// Bytes occupied by tensor payloads, excluding safetensors headers.
    pub tensor_data_bytes: u64,
    /// Number of bounded weight sections in the aggregate record.
    pub sections: u32,
}

/// Byte-backed inputs for one pinned Qwen3 model role.
#[derive(Clone, Copy, Debug)]
pub struct ModelAssets<'a> {
    /// Exact Hugging Face repository name.
    pub repository: &'a str,
    /// Exact immutable Hugging Face revision.
    pub revision: &'a str,
    /// Complete upstream `config.json` bytes.
    pub config_json: &'a [u8],
    /// Complete upstream `tokenizer_config.json` bytes.
    pub tokenizer_metadata_json: &'a [u8],
    /// Caller-asserted shared `tokenizer.json` descriptor.
    pub vocabulary: ArtifactDigest,
    /// Caller-asserted pinned weight descriptor.
    pub weights: WeightDescriptor,
}

/// Complete byte-backed inputs for the first M1 deployment bundle.
#[derive(Clone, Copy, Debug)]
pub struct DeploymentAssets<'a> {
    /// M1 target model artifacts.
    pub target: ModelAssets<'a>,
    /// M1 speculative draft model artifacts.
    pub draft: ModelAssets<'a>,
    /// Requested bounded engine limits.
    pub limits: EngineLimits,
}

/// Inputs whose weight descriptors come from streaming authenticated sets.
///
/// Configuration and tokenizer metadata bytes are authenticated here, while
/// `tokenizer.json` remains a pinned caller-asserted descriptor.
#[derive(Clone, Copy, Debug)]
pub struct WeightAuthenticatedModelAssets<'a> {
    /// Exact Hugging Face repository name.
    pub repository: &'a str,
    /// Exact immutable Hugging Face revision.
    pub revision: &'a str,
    /// Complete upstream `config.json` bytes.
    pub config_json: &'a [u8],
    /// Complete upstream `tokenizer_config.json` bytes.
    pub tokenizer_metadata_json: &'a [u8],
    /// Caller-asserted shared `tokenizer.json` descriptor.
    pub vocabulary: ArtifactDigest,
}

/// Deployment inputs paired with separately authenticated target/draft weights.
#[derive(Clone, Copy, Debug)]
pub struct WeightAuthenticatedDeploymentAssets<'a> {
    /// M1 target model metadata and tokenizer inputs.
    pub target: WeightAuthenticatedModelAssets<'a>,
    /// M1 speculative draft model metadata and tokenizer inputs.
    pub draft: WeightAuthenticatedModelAssets<'a>,
    /// Requested bounded engine limits.
    pub limits: EngineLimits,
}

/// Byte-backed model inputs whose tokenizer and weight payload authorities are
/// supplied separately to the fully authenticated builder.
#[derive(Clone, Copy, Debug)]
pub struct AuthenticatedModelAssets<'a> {
    /// Exact Hugging Face repository name.
    pub repository: &'a str,
    /// Exact immutable Hugging Face revision.
    pub revision: &'a str,
    /// Complete upstream `config.json` bytes.
    pub config_json: &'a [u8],
    /// Complete upstream `tokenizer_config.json` bytes.
    pub tokenizer_metadata_json: &'a [u8],
}

/// Inputs for a deployment whose opaque payloads arrive only as sealed
/// streaming-authenticated authorities.
#[derive(Clone, Copy, Debug)]
pub struct AuthenticatedDeploymentAssets<'a> {
    /// M1 target model configuration and tokenizer metadata.
    pub target: AuthenticatedModelAssets<'a>,
    /// M1 speculative draft configuration and tokenizer metadata.
    pub draft: AuthenticatedModelAssets<'a>,
    /// Requested bounded engine limits.
    pub limits: EngineLimits,
}

/// Validated deployment plus the two consumed prepacked-output authorities.
///
/// The `ferric-spec` deployment bundle binds the exact source weight-set
/// identities. The two manifests separately bind the emitted foundation
/// layouts because the current executable bundle schema has no prepacked
/// aggregate-identity field. This result is intentionally not `Clone`.
#[derive(Debug, PartialEq, Eq)]
pub struct PrepackedDeploymentBundle {
    deployment: DeploymentBundle,
    target_manifest: WeightSectionManifest,
    draft_manifest: WeightSectionManifest,
}

impl PrepackedDeploymentBundle {
    /// Returns the admitted executable deployment bundle.
    #[must_use]
    pub const fn deployment(&self) -> &DeploymentBundle {
        &self.deployment
    }

    /// Returns the exact target prepacked-output manifest.
    #[must_use]
    pub const fn target_manifest(&self) -> &WeightSectionManifest {
        &self.target_manifest
    }

    /// Returns the exact draft prepacked-output manifest.
    #[must_use]
    pub const fn draft_manifest(&self) -> &WeightSectionManifest {
        &self.draft_manifest
    }
}

/// Fail-closed bundle parsing or admission error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BuildError {
    /// An input artifact was empty.
    EmptyArtifact(&'static str),
    /// An input artifact exceeded its parser bound.
    ArtifactTooLarge(&'static str),
    /// An input was not structurally valid JSON.
    InvalidJson {
        /// Artifact being parsed.
        artifact: &'static str,
        /// Byte offset reported by the strict parser.
        offset: usize,
        /// Stable parser reason.
        reason: String,
    },
    /// A closed-schema field was absent.
    MissingField {
        /// Artifact being parsed.
        artifact: &'static str,
        /// Required field name.
        field: String,
    },
    /// A closed-schema object contained an unrecognized field.
    UnknownField {
        /// Artifact being parsed.
        artifact: &'static str,
        /// Rejected field name.
        field: String,
    },
    /// A field had the wrong JSON type.
    WrongType {
        /// Artifact being parsed.
        artifact: &'static str,
        /// Rejected field name.
        field: String,
    },
    /// A typed field differed from its pinned Qwen3 value.
    UnexpectedValue {
        /// Artifact being parsed.
        artifact: &'static str,
        /// Rejected field name.
        field: String,
    },
    /// Metadata requested model- or tokenizer-defined executable code.
    RemoteCode(String),
    /// The repository or revision did not match the selected role.
    SourceMismatch(Qwen3ModelRole),
    /// Parsed bytes did not match the pinned upstream artifact identity.
    DigestMismatch(&'static str),
    /// An opaque artifact descriptor did not match the pinned Qwen3 asset.
    DescriptorMismatch(&'static str),
    /// A streamed weight authority was supplied for the wrong model role.
    AuthenticatedWeightRole {
        /// Role required by the builder position.
        expected: Qwen3ModelRole,
        /// Role carried by the authenticated authority.
        actual: Qwen3ModelRole,
    },
    /// A streamed tokenizer authority was supplied for the wrong model role.
    AuthenticatedTokenizerRole {
        /// Role required by the builder position.
        expected: Qwen3ModelRole,
        /// Role carried by the authenticated authority.
        actual: Qwen3ModelRole,
    },
    /// Target and draft tokenizer semantics or metadata were not identical.
    TokenizerMismatch,
    /// The executable `ferric-spec` contract rejected the assembled bundle.
    Spec(SpecError),
}

impl fmt::Display for BuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyArtifact(artifact) => write!(formatter, "{artifact} is empty"),
            Self::ArtifactTooLarge(artifact) => {
                write!(formatter, "{artifact} exceeds its parser bound")
            }
            Self::InvalidJson {
                artifact,
                offset,
                reason,
            } => write!(
                formatter,
                "invalid {artifact} JSON at byte {offset}: {reason}"
            ),
            Self::MissingField { artifact, field } => {
                write!(formatter, "{artifact} is missing field {field:?}")
            }
            Self::UnknownField { artifact, field } => {
                write!(formatter, "{artifact} has unknown field {field:?}")
            }
            Self::WrongType { artifact, field } => {
                write!(formatter, "{artifact} field {field:?} has the wrong type")
            }
            Self::UnexpectedValue { artifact, field } => {
                write!(formatter, "{artifact} field {field:?} is not canonical")
            }
            Self::RemoteCode(field) => {
                write!(formatter, "remote code declaration {field:?} is forbidden")
            }
            Self::SourceMismatch(role) => write!(formatter, "source does not match {role:?}"),
            Self::DigestMismatch(artifact) => {
                write!(
                    formatter,
                    "{artifact} does not match the pinned byte identity"
                )
            }
            Self::DescriptorMismatch(artifact) => {
                write!(
                    formatter,
                    "{artifact} descriptor does not match the pinned asset"
                )
            }
            Self::AuthenticatedWeightRole { expected, actual } => write!(
                formatter,
                "authenticated weight role {actual:?} does not match {expected:?}"
            ),
            Self::AuthenticatedTokenizerRole { expected, actual } => write!(
                formatter,
                "authenticated tokenizer role {actual:?} does not match {expected:?}"
            ),
            Self::TokenizerMismatch => {
                formatter.write_str("target and draft tokenizers are not compatible")
            }
            Self::Spec(error) => write!(formatter, "bundle admission failed: {error}"),
        }
    }
}

impl std::error::Error for BuildError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Spec(error) => Some(error),
            _ => None,
        }
    }
}

/// Hashes complete artifact bytes into their canonical 32-byte identity.
#[must_use]
pub fn digest_bytes(bytes: &[u8]) -> ArtifactDigest {
    ArtifactDigest {
        sha256: sha256::digest(bytes),
        byte_len: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
    }
}

/// Preliminary descriptor-only configuration/tokenizer bundle admission.
///
/// This function checks pinned caller-asserted tokenizer and weight descriptors
/// but cannot construct [`AuthenticatedTokenizer`] or
/// [`AuthenticatedWeightSet`].
///
/// # Errors
///
/// Returns [`BuildError`] for malformed JSON, schema drift, remote-code
/// declarations, identity mismatches, or any `ferric-spec` admission failure.
pub fn build_preliminary_deployment_bundle(
    assets: DeploymentAssets<'_>,
) -> Result<DeploymentBundle, BuildError> {
    assemble_deployment_bundle(&assets)
}

/// Builds a bundle from configuration/tokenizer metadata plus two consumed
/// streaming-authenticated weight authorities.
///
/// This is weight-authenticated, not fully artifact-authenticated:
/// `tokenizer.json` remains the pinned caller-asserted vocabulary descriptor
/// in [`WeightAuthenticatedModelAssets`]. The private seals on both weight
/// inputs ensure descriptor-only code cannot enter this path.
///
/// # Errors
///
/// Returns [`BuildError`] for swapped weight roles, malformed JSON, schema
/// drift, remote-code declarations, identity mismatches, or any `ferric-spec`
/// admission failure.
pub fn build_weight_authenticated_deployment_bundle(
    assets: WeightAuthenticatedDeploymentAssets<'_>,
    target_weights: AuthenticatedWeightSet,
    draft_weights: AuthenticatedWeightSet,
) -> Result<DeploymentBundle, BuildError> {
    let target_role = target_weights.role();
    if target_role != Qwen3ModelRole::Target8B {
        return Err(BuildError::AuthenticatedWeightRole {
            expected: Qwen3ModelRole::Target8B,
            actual: target_role,
        });
    }
    let draft_role = draft_weights.role();
    if draft_role != Qwen3ModelRole::Draft06B {
        return Err(BuildError::AuthenticatedWeightRole {
            expected: Qwen3ModelRole::Draft06B,
            actual: draft_role,
        });
    }
    let admission_assets = DeploymentAssets {
        target: ModelAssets {
            repository: assets.target.repository,
            revision: assets.target.revision,
            config_json: assets.target.config_json,
            tokenizer_metadata_json: assets.target.tokenizer_metadata_json,
            vocabulary: assets.target.vocabulary,
            weights: target_weights.into_descriptor(),
        },
        draft: ModelAssets {
            repository: assets.draft.repository,
            revision: assets.draft.revision,
            config_json: assets.draft.config_json,
            tokenizer_metadata_json: assets.draft.tokenizer_metadata_json,
            vocabulary: assets.draft.vocabulary,
            weights: draft_weights.into_descriptor(),
        },
        limits: assets.limits,
    };
    assemble_deployment_bundle(&admission_assets)
}

/// Builds an M1 bundle by consuming exact target/draft tokenizer and weight
/// authorities produced only by their streaming authenticators.
///
/// The builder also authenticates the supplied configuration and tokenizer
/// metadata bytes, checks target/draft special-token and full chat-template
/// compatibility through their exact metadata identities, assembles the
/// deployment, and calls the executable `ferric-spec` validator.
///
/// # Errors
///
/// Returns [`BuildError`] for swapped authorities, target/draft tokenizer
/// mismatch, malformed or noncanonical metadata, source or identity mismatch,
/// or any `ferric-spec` admission failure.
pub fn build_authenticated_deployment_bundle(
    assets: AuthenticatedDeploymentAssets<'_>,
    target_tokenizer: AuthenticatedTokenizer,
    draft_tokenizer: AuthenticatedTokenizer,
    target_weights: AuthenticatedWeightSet,
    draft_weights: AuthenticatedWeightSet,
) -> Result<DeploymentBundle, BuildError> {
    let target_tokenizer_role = target_tokenizer.role();
    if target_tokenizer_role != Qwen3ModelRole::Target8B {
        return Err(BuildError::AuthenticatedTokenizerRole {
            expected: Qwen3ModelRole::Target8B,
            actual: target_tokenizer_role,
        });
    }
    let draft_tokenizer_role = draft_tokenizer.role();
    if draft_tokenizer_role != Qwen3ModelRole::Draft06B {
        return Err(BuildError::AuthenticatedTokenizerRole {
            expected: Qwen3ModelRole::Draft06B,
            actual: draft_tokenizer_role,
        });
    }
    if !target_tokenizer.compatible_with(&draft_tokenizer) {
        return Err(BuildError::TokenizerMismatch);
    }

    let target_weight_role = target_weights.role();
    if target_weight_role != Qwen3ModelRole::Target8B {
        return Err(BuildError::AuthenticatedWeightRole {
            expected: Qwen3ModelRole::Target8B,
            actual: target_weight_role,
        });
    }
    let draft_weight_role = draft_weights.role();
    if draft_weight_role != Qwen3ModelRole::Draft06B {
        return Err(BuildError::AuthenticatedWeightRole {
            expected: Qwen3ModelRole::Draft06B,
            actual: draft_weight_role,
        });
    }

    let admission_assets = DeploymentAssets {
        target: ModelAssets {
            repository: assets.target.repository,
            revision: assets.target.revision,
            config_json: assets.target.config_json,
            tokenizer_metadata_json: assets.target.tokenizer_metadata_json,
            vocabulary: target_tokenizer.into_descriptor(),
            weights: target_weights.into_descriptor(),
        },
        draft: ModelAssets {
            repository: assets.draft.repository,
            revision: assets.draft.revision,
            config_json: assets.draft.config_json,
            tokenizer_metadata_json: assets.draft.tokenizer_metadata_json,
            vocabulary: draft_tokenizer.into_descriptor(),
            weights: draft_weights.into_descriptor(),
        },
        limits: assets.limits,
    };
    assemble_deployment_bundle(&admission_assets)
}

/// Builds an M1 deployment while consuming tokenizer and prepacked authorities.
///
/// The prepacked authorities can be produced only by fresh source streaming;
/// neither caller-asserted descriptors nor a prior authentication pass can
/// enter this path. The result retains both canonical prepacked manifests next
/// to the executable bundle so their aggregate identities are not discarded.
///
/// # Errors
///
/// Returns [`BuildError`] for swapped authorities, target/draft tokenizer
/// mismatch, malformed or noncanonical metadata, source or identity mismatch,
/// or any `ferric-spec` admission failure.
pub fn build_prepacked_deployment_bundle(
    assets: AuthenticatedDeploymentAssets<'_>,
    target_tokenizer: AuthenticatedTokenizer,
    draft_tokenizer: AuthenticatedTokenizer,
    target_weights: PrepackedWeightSet,
    draft_weights: PrepackedWeightSet,
) -> Result<PrepackedDeploymentBundle, BuildError> {
    let target_tokenizer_role = target_tokenizer.role();
    if target_tokenizer_role != Qwen3ModelRole::Target8B {
        return Err(BuildError::AuthenticatedTokenizerRole {
            expected: Qwen3ModelRole::Target8B,
            actual: target_tokenizer_role,
        });
    }
    let draft_tokenizer_role = draft_tokenizer.role();
    if draft_tokenizer_role != Qwen3ModelRole::Draft06B {
        return Err(BuildError::AuthenticatedTokenizerRole {
            expected: Qwen3ModelRole::Draft06B,
            actual: draft_tokenizer_role,
        });
    }
    if !target_tokenizer.compatible_with(&draft_tokenizer) {
        return Err(BuildError::TokenizerMismatch);
    }

    let target_weight_role = target_weights.role();
    if target_weight_role != Qwen3ModelRole::Target8B {
        return Err(BuildError::AuthenticatedWeightRole {
            expected: Qwen3ModelRole::Target8B,
            actual: target_weight_role,
        });
    }
    let draft_weight_role = draft_weights.role();
    if draft_weight_role != Qwen3ModelRole::Draft06B {
        return Err(BuildError::AuthenticatedWeightRole {
            expected: Qwen3ModelRole::Draft06B,
            actual: draft_weight_role,
        });
    }

    let (target_descriptor, target_manifest) = target_weights.into_parts();
    let (draft_descriptor, draft_manifest) = draft_weights.into_parts();
    let admission_assets = DeploymentAssets {
        target: ModelAssets {
            repository: assets.target.repository,
            revision: assets.target.revision,
            config_json: assets.target.config_json,
            tokenizer_metadata_json: assets.target.tokenizer_metadata_json,
            vocabulary: target_tokenizer.into_descriptor(),
            weights: target_descriptor,
        },
        draft: ModelAssets {
            repository: assets.draft.repository,
            revision: assets.draft.revision,
            config_json: assets.draft.config_json,
            tokenizer_metadata_json: assets.draft.tokenizer_metadata_json,
            vocabulary: draft_tokenizer.into_descriptor(),
            weights: draft_descriptor,
        },
        limits: assets.limits,
    };
    let deployment = assemble_deployment_bundle(&admission_assets)?;
    Ok(PrepackedDeploymentBundle {
        deployment,
        target_manifest,
        draft_manifest,
    })
}

fn assemble_deployment_bundle(
    assets: &DeploymentAssets<'_>,
) -> Result<DeploymentBundle, BuildError> {
    let mut target_config = parse_config(Qwen3ModelRole::Target8B, &assets.target)?;
    let mut draft_config = parse_config(Qwen3ModelRole::Draft06B, &assets.draft)?;
    let target_tokenizer = parse_tokenizer(&assets.target)?;
    let draft_tokenizer = parse_tokenizer(&assets.draft)?;
    if target_tokenizer != draft_tokenizer {
        return Err(BuildError::TokenizerMismatch);
    }
    let target_weights = weight_manifest(Qwen3ModelRole::Target8B, assets.target.weights)?;
    let draft_weights = weight_manifest(Qwen3ModelRole::Draft06B, assets.draft.weights)?;

    target_config.model_id = model_identity(
        Qwen3ModelRole::Target8B,
        assets.target.repository,
        assets.target.revision,
        target_config.config_id,
        target_tokenizer,
        target_weights,
        assets.target.weights.tensor_data_bytes,
    );
    draft_config.model_id = model_identity(
        Qwen3ModelRole::Draft06B,
        assets.draft.repository,
        assets.draft.revision,
        draft_config.config_id,
        draft_tokenizer,
        draft_weights,
        assets.draft.weights.tensor_data_bytes,
    );

    let target_model = ModelArtifact {
        config: target_config,
        tokenizer: target_tokenizer,
        weights: target_weights,
    };
    let draft_model = ModelArtifact {
        config: draft_config,
        tokenizer: draft_tokenizer,
        weights: draft_weights,
    };
    let bundle = DeploymentBundle {
        bundle_id: bundle_identity(assets.limits, target_model, draft_model),
        target: Target::Gfx942XnackMinus,
        numerical_policy: NumericalPolicy::Bf16ParametersFp32Accumulation,
        limits: assets.limits,
        target_model,
        draft_model,
    };
    bundle.validate().map_err(BuildError::Spec)?;
    Ok(bundle)
}

fn parse_config(role: Qwen3ModelRole, assets: &ModelAssets<'_>) -> Result<ModelConfig, BuildError> {
    validate_source(role, assets.repository, assets.revision)?;
    let artifact = match role {
        Qwen3ModelRole::Target8B => "target config.json",
        Qwen3ModelRole::Draft06B => "draft config.json",
    };
    let value = parse_json(artifact, assets.config_json, MAX_CONFIG_BYTES)?;
    reject_remote_code(&value)?;
    let mut object = ObjectReader::new(artifact, expect_object(artifact, "$", value)?);

    expect_single_string_array(
        artifact,
        "architectures",
        object.take("architectures")?,
        "Qwen3ForCausalLM",
    )?;
    object.expect_bool("attention_bias", false)?;
    object.expect_number_literal("attention_dropout", "0.0")?;
    object.expect_u32("bos_token_id", QWEN3_END_OF_TEXT_TOKEN)?;
    object.expect_u32("eos_token_id", QWEN3_IM_END_TOKEN)?;
    let head_dim = object.u32("head_dim")?;
    object.expect_string("hidden_act", "silu")?;
    let hidden_size = object.u32("hidden_size")?;
    object.expect_number_literal("initializer_range", "0.02")?;
    let intermediate_size = object.u32("intermediate_size")?;
    let max_position_embeddings = object.u32("max_position_embeddings")?;
    let max_window_layers = object.u32("max_window_layers")?;
    object.expect_string("model_type", "qwen3")?;
    let query_heads = object.u32("num_attention_heads")?;
    let layers = object.u32("num_hidden_layers")?;
    let kv_heads = object.u32("num_key_value_heads")?;
    object.expect_number_literal("rms_norm_eps", "1e-06")?;
    object.expect_null("rope_scaling")?;
    let rope_theta = object.u32("rope_theta")?;
    object.expect_null("sliding_window")?;
    let tie_word_embeddings = object.bool("tie_word_embeddings")?;
    object.expect_string("torch_dtype", "bfloat16")?;
    object.expect_string("transformers_version", "4.51.0")?;
    object.expect_bool("use_cache", true)?;
    object.expect_bool("use_sliding_window", false)?;
    let vocabulary_size = object.u32("vocab_size")?;
    object.finish()?;
    if max_window_layers != layers {
        return Err(unexpected(artifact, "max_window_layers"));
    }

    let config_digest = digest_bytes(assets.config_json);
    let expected = match role {
        Qwen3ModelRole::Target8B => TARGET_CONFIG_SHA256,
        Qwen3ModelRole::Draft06B => DRAFT_CONFIG_SHA256,
    };
    if config_digest.sha256 != expected {
        return Err(BuildError::DigestMismatch(artifact));
    }

    Ok(ModelConfig {
        role,
        model_id: Identity::new([0; 32]),
        config_id: Identity::new(config_digest.sha256),
        vocabulary_size,
        layers,
        hidden_size,
        intermediate_size,
        query_heads,
        kv_heads,
        head_dim,
        max_position_embeddings,
        rope_theta,
        tie_word_embeddings,
    })
}

fn parse_tokenizer(assets: &ModelAssets<'_>) -> Result<TokenizerConfig, BuildError> {
    let artifact = "tokenizer_config.json";
    let value = parse_json(
        artifact,
        assets.tokenizer_metadata_json,
        MAX_TOKENIZER_METADATA_BYTES,
    )?;
    reject_remote_code(&value)?;
    let mut object = ObjectReader::new(artifact, expect_object(artifact, "$", value)?);
    object.expect_bool("add_bos_token", false)?;
    object.expect_bool("add_prefix_space", false)?;
    validate_added_tokens(artifact, object.take("added_tokens_decoder")?)?;
    expect_string_array(
        artifact,
        "additional_special_tokens",
        object.take("additional_special_tokens")?,
        &ADDITIONAL_SPECIAL_TOKENS,
    )?;
    object.expect_null("bos_token")?;
    let chat_template = object.string("chat_template")?;
    if chat_template.is_empty() || chat_template.len() > MAX_CHAT_TEMPLATE_BYTES {
        return Err(unexpected(artifact, "chat_template"));
    }
    object.expect_bool("clean_up_tokenization_spaces", false)?;
    object.expect_string("eos_token", "<|im_end|>")?;
    object.expect_string("errors", "replace")?;
    object.expect_u32("model_max_length", 131_072)?;
    object.expect_string("pad_token", "<|endoftext|>")?;
    object.expect_bool("split_special_tokens", false)?;
    object.expect_string("tokenizer_class", "Qwen2Tokenizer")?;
    object.expect_null("unk_token")?;
    object.finish()?;

    let metadata_digest = digest_bytes(assets.tokenizer_metadata_json);
    if metadata_digest.sha256 != TOKENIZER_METADATA_SHA256 {
        return Err(BuildError::DigestMismatch(artifact));
    }
    if assets.vocabulary.sha256 != QWEN3_TOKENIZER_SHA256
        || assets.vocabulary.byte_len != QWEN3_TOKENIZER_BYTES
    {
        return Err(BuildError::DescriptorMismatch("tokenizer.json"));
    }
    Ok(TokenizerConfig {
        tokenizer_id: Identity::new(metadata_digest.sha256),
        vocabulary_id: Identity::new(assets.vocabulary.sha256),
        vocabulary_size: QWEN3_VOCABULARY_SIZE,
        end_of_text_token: QWEN3_END_OF_TEXT_TOKEN,
        im_start_token: QWEN3_IM_START_TOKEN,
        im_end_token: QWEN3_IM_END_TOKEN,
    })
}

fn validate_added_tokens(artifact: &'static str, value: Value) -> Result<(), BuildError> {
    let mut tokens = expect_object(artifact, "added_tokens_decoder", value)?;
    for (offset, (content, special)) in ADDED_TOKENS.iter().enumerate() {
        let token_id = (151_643 + offset).to_string();
        let token = tokens
            .remove(&token_id)
            .ok_or_else(|| missing(artifact, &format!("added_tokens_decoder.{token_id}")))?;
        let field = format!("added_tokens_decoder.{token_id}");
        let mut object = ObjectReader::new(artifact, expect_object(artifact, &field, token)?);
        object.expect_string("content", content)?;
        object.expect_bool("lstrip", false)?;
        object.expect_bool("normalized", false)?;
        object.expect_bool("rstrip", false)?;
        object.expect_bool("single_word", false)?;
        object.expect_bool("special", *special)?;
        object.finish()?;
    }
    if let Some(field) = tokens.into_keys().next() {
        return Err(unknown(artifact, &format!("added_tokens_decoder.{field}")));
    }
    Ok(())
}

fn validate_source(
    role: Qwen3ModelRole,
    repository: &str,
    revision: &str,
) -> Result<(), BuildError> {
    let expected = match role {
        Qwen3ModelRole::Target8B => (TARGET_REPOSITORY, TARGET_REVISION),
        Qwen3ModelRole::Draft06B => (DRAFT_REPOSITORY, DRAFT_REVISION),
    };
    if (repository, revision) != expected {
        return Err(BuildError::SourceMismatch(role));
    }
    Ok(())
}

fn weight_manifest(
    role: Qwen3ModelRole,
    descriptor: WeightDescriptor,
) -> Result<WeightManifest, BuildError> {
    let (expected_sha256, artifact_bytes, tensor_data_bytes, sections, artifact) = match role {
        Qwen3ModelRole::Target8B => (
            target_weight_set_identity(),
            QWEN3_TARGET_WEIGHT_ARTIFACT_BYTES,
            QWEN3_TARGET_TENSOR_DATA_BYTES,
            5,
            "target weight set",
        ),
        Qwen3ModelRole::Draft06B => (
            QWEN3_DRAFT_WEIGHT_SHA256,
            QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES,
            QWEN3_DRAFT_TENSOR_DATA_BYTES,
            1,
            "draft weight file",
        ),
    };
    if descriptor.weights_id != expected_sha256
        || descriptor.artifact_bytes != artifact_bytes
        || descriptor.tensor_data_bytes != tensor_data_bytes
        || descriptor.sections != sections
    {
        return Err(BuildError::DescriptorMismatch(artifact));
    }
    Ok(WeightManifest {
        weights_id: Identity::new(descriptor.weights_id),
        total_bytes: descriptor.artifact_bytes,
        sections: descriptor.sections,
    })
}

fn model_identity(
    role: Qwen3ModelRole,
    repository: &str,
    revision: &str,
    config_id: Identity,
    tokenizer: TokenizerConfig,
    weights: WeightManifest,
    tensor_data_bytes: u64,
) -> Identity {
    let role_byte = [match role {
        Qwen3ModelRole::Target8B => 0,
        Qwen3ModelRole::Draft06B => 1,
    }];
    let weight_bytes = weights.total_bytes.to_be_bytes();
    let tensor_bytes = tensor_data_bytes.to_be_bytes();
    let weight_sections = weights.sections.to_be_bytes();
    record_identity(
        b"ferric.model.v1",
        &[
            &role_byte,
            repository.as_bytes(),
            revision.as_bytes(),
            config_id.as_bytes(),
            tokenizer.tokenizer_id.as_bytes(),
            tokenizer.vocabulary_id.as_bytes(),
            weights.weights_id.as_bytes(),
            &weight_bytes,
            &tensor_bytes,
            &weight_sections,
        ],
    )
}

fn bundle_identity(limits: EngineLimits, target: ModelArtifact, draft: ModelArtifact) -> Identity {
    let context = limits.max_context_tokens.to_be_bytes();
    let sequences = limits.max_active_sequences.to_be_bytes();
    let page = limits.kv_page_tokens.to_be_bytes();
    let draft_tokens = limits.max_draft_tokens.to_be_bytes();
    record_identity(
        b"ferric.deployment-bundle.v1",
        &[
            b"gfx942:xnack-",
            b"bf16-parameters:fp32-accumulation",
            &context,
            &sequences,
            &page,
            &draft_tokens,
            target.config.model_id.as_bytes(),
            draft.config.model_id.as_bytes(),
        ],
    )
}

fn record_identity(domain: &[u8], fields: &[&[u8]]) -> Identity {
    let mut hasher = Sha256::new();
    hash_field(&mut hasher, domain);
    for field in fields {
        hash_field(&mut hasher, field);
    }
    Identity::new(hasher.finish())
}

fn target_weight_set_identity() -> [u8; 32] {
    let mut hasher = Sha256::new();
    hash_field(&mut hasher, b"ferric.safetensors-set.v1");
    for (name, digest, byte_len) in TARGET_WEIGHT_SET_COMPONENTS {
        hash_field(&mut hasher, name.as_bytes());
        hash_field(&mut hasher, &digest);
        hash_field(&mut hasher, &byte_len.to_be_bytes());
    }
    hasher.finish()
}

fn hash_field(hasher: &mut Sha256, bytes: &[u8]) {
    let length = u64::try_from(bytes.len())
        .expect("identity field length fits u64")
        .to_be_bytes();
    hasher.update(&length);
    hasher.update(bytes);
}

fn parse_json(artifact: &'static str, bytes: &[u8], max_bytes: usize) -> Result<Value, BuildError> {
    if bytes.is_empty() {
        return Err(BuildError::EmptyArtifact(artifact));
    }
    if bytes.len() > max_bytes {
        return Err(BuildError::ArtifactTooLarge(artifact));
    }
    json::parse(bytes).map_err(|error| BuildError::InvalidJson {
        artifact,
        offset: error.offset,
        reason: error.kind.to_string(),
    })
}

fn reject_remote_code(value: &Value) -> Result<(), BuildError> {
    match value {
        Value::Array(values) => {
            for value in values {
                reject_remote_code(value)?;
            }
        }
        Value::Object(fields) => {
            for (field, value) in fields {
                if matches!(
                    field.as_str(),
                    "auto_map" | "trust_remote_code" | "custom_pipelines"
                ) {
                    return Err(BuildError::RemoteCode(field.clone()));
                }
                reject_remote_code(value)?;
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
    Ok(())
}

fn expect_object(
    artifact: &'static str,
    field: &str,
    value: Value,
) -> Result<BTreeMap<String, Value>, BuildError> {
    if let Value::Object(object) = value {
        Ok(object)
    } else {
        Err(wrong_type(artifact, field))
    }
}

fn expect_single_string_array(
    artifact: &'static str,
    field: &str,
    value: Value,
    expected: &str,
) -> Result<(), BuildError> {
    expect_string_array(artifact, field, value, &[expected])
}

fn expect_string_array(
    artifact: &'static str,
    field: &str,
    value: Value,
    expected: &[&str],
) -> Result<(), BuildError> {
    let Value::Array(values) = value else {
        return Err(wrong_type(artifact, field));
    };
    if values.len() != expected.len() {
        return Err(unexpected(artifact, field));
    }
    for (value, expected_value) in values.into_iter().zip(expected) {
        if value != Value::String((*expected_value).to_owned()) {
            return Err(unexpected(artifact, field));
        }
    }
    Ok(())
}

struct ObjectReader {
    artifact: &'static str,
    fields: BTreeMap<String, Value>,
}

impl ObjectReader {
    fn new(artifact: &'static str, fields: BTreeMap<String, Value>) -> Self {
        Self { artifact, fields }
    }

    fn take(&mut self, field: &str) -> Result<Value, BuildError> {
        self.fields
            .remove(field)
            .ok_or_else(|| missing(self.artifact, field))
    }

    fn bool(&mut self, field: &str) -> Result<bool, BuildError> {
        if let Value::Bool(value) = self.take(field)? {
            Ok(value)
        } else {
            Err(wrong_type(self.artifact, field))
        }
    }

    fn expect_bool(&mut self, field: &str, expected: bool) -> Result<(), BuildError> {
        if self.bool(field)? != expected {
            return Err(unexpected(self.artifact, field));
        }
        Ok(())
    }

    fn string(&mut self, field: &str) -> Result<String, BuildError> {
        if let Value::String(value) = self.take(field)? {
            Ok(value)
        } else {
            Err(wrong_type(self.artifact, field))
        }
    }

    fn expect_string(&mut self, field: &str, expected: &str) -> Result<(), BuildError> {
        if self.string(field)? != expected {
            return Err(unexpected(self.artifact, field));
        }
        Ok(())
    }

    fn u32(&mut self, field: &str) -> Result<u32, BuildError> {
        let Value::Number(value) = self.take(field)? else {
            return Err(wrong_type(self.artifact, field));
        };
        value.parse().map_err(|_| unexpected(self.artifact, field))
    }

    fn expect_u32(&mut self, field: &str, expected: u32) -> Result<(), BuildError> {
        if self.u32(field)? != expected {
            return Err(unexpected(self.artifact, field));
        }
        Ok(())
    }

    fn expect_number_literal(&mut self, field: &str, expected: &str) -> Result<(), BuildError> {
        let Value::Number(value) = self.take(field)? else {
            return Err(wrong_type(self.artifact, field));
        };
        if value != expected {
            return Err(unexpected(self.artifact, field));
        }
        Ok(())
    }

    fn expect_null(&mut self, field: &str) -> Result<(), BuildError> {
        if self.take(field)? != Value::Null {
            return Err(unexpected(self.artifact, field));
        }
        Ok(())
    }

    fn finish(self) -> Result<(), BuildError> {
        if let Some(field) = self.fields.into_keys().next() {
            return Err(unknown(self.artifact, &field));
        }
        Ok(())
    }
}

fn missing(artifact: &'static str, field: &str) -> BuildError {
    BuildError::MissingField {
        artifact,
        field: field.to_owned(),
    }
}

fn unknown(artifact: &'static str, field: &str) -> BuildError {
    BuildError::UnknownField {
        artifact,
        field: field.to_owned(),
    }
}

fn wrong_type(artifact: &'static str, field: &str) -> BuildError {
    BuildError::WrongType {
        artifact,
        field: field.to_owned(),
    }
}

fn unexpected(artifact: &'static str, field: &str) -> BuildError {
    BuildError::UnexpectedValue {
        artifact,
        field: field.to_owned(),
    }
}

const fn decode_hex_32(hex: &[u8; 64]) -> [u8; 32] {
    let mut bytes = [0; 32];
    let mut index = 0;
    while index < bytes.len() {
        bytes[index] = (hex_nibble(hex[index * 2]) << 4) | hex_nibble(hex[index * 2 + 1]);
        index += 1;
    }
    bytes
}

const fn hex_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_preliminary_deployment_bundle, digest_bytes, ArtifactDigest, BuildError,
        DeploymentAssets, ModelAssets, WeightDescriptor, DRAFT_REPOSITORY, DRAFT_REVISION,
        QWEN3_DRAFT_TENSOR_DATA_BYTES, QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES,
        QWEN3_DRAFT_WEIGHT_SHA256, QWEN3_TARGET_TENSOR_DATA_BYTES,
        QWEN3_TARGET_WEIGHT_ARTIFACT_BYTES, QWEN3_TARGET_WEIGHT_SET_SHA256, QWEN3_TOKENIZER_BYTES,
        QWEN3_TOKENIZER_SHA256, TARGET_REPOSITORY, TARGET_REVISION,
    };
    use ferric_spec::{EngineLimits, SpecError};

    const TARGET_CONFIG: &[u8] = include_bytes!("fixtures/qwen3-8b-config.json");
    const DRAFT_CONFIG: &[u8] = include_bytes!("fixtures/qwen3-06b-config.json");
    const TOKENIZER_METADATA: &[u8] = include_bytes!("fixtures/qwen3-tokenizer-config.json");

    fn pinned_config(bytes: &'static [u8]) -> &'static [u8] {
        assert_eq!(bytes.last(), Some(&b'\n'));
        &bytes[..bytes.len() - 1]
    }

    fn vocabulary() -> ArtifactDigest {
        ArtifactDigest {
            sha256: QWEN3_TOKENIZER_SHA256,
            byte_len: QWEN3_TOKENIZER_BYTES,
        }
    }

    fn weights(
        sha256: [u8; 32],
        byte_len: u64,
        tensor_data_bytes: u64,
        sections: u32,
    ) -> WeightDescriptor {
        WeightDescriptor {
            weights_id: sha256,
            artifact_bytes: byte_len,
            tensor_data_bytes,
            sections,
        }
    }

    fn target<'a>(config: &'a [u8], tokenizer: &'a [u8]) -> ModelAssets<'a> {
        ModelAssets {
            repository: TARGET_REPOSITORY,
            revision: TARGET_REVISION,
            config_json: config,
            tokenizer_metadata_json: tokenizer,
            vocabulary: vocabulary(),
            weights: weights(
                QWEN3_TARGET_WEIGHT_SET_SHA256,
                QWEN3_TARGET_WEIGHT_ARTIFACT_BYTES,
                QWEN3_TARGET_TENSOR_DATA_BYTES,
                5,
            ),
        }
    }

    fn draft<'a>(config: &'a [u8], tokenizer: &'a [u8]) -> ModelAssets<'a> {
        ModelAssets {
            repository: DRAFT_REPOSITORY,
            revision: DRAFT_REVISION,
            config_json: config,
            tokenizer_metadata_json: tokenizer,
            vocabulary: vocabulary(),
            weights: weights(
                QWEN3_DRAFT_WEIGHT_SHA256,
                QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES,
                QWEN3_DRAFT_TENSOR_DATA_BYTES,
                1,
            ),
        }
    }

    fn limits() -> EngineLimits {
        EngineLimits {
            max_context_tokens: 8_192,
            max_active_sequences: 32,
            kv_page_tokens: 256,
            max_draft_tokens: 16,
        }
    }

    fn canonical_assets() -> DeploymentAssets<'static> {
        DeploymentAssets {
            target: target(pinned_config(TARGET_CONFIG), TOKENIZER_METADATA),
            draft: draft(pinned_config(DRAFT_CONFIG), TOKENIZER_METADATA),
            limits: limits(),
        }
    }

    fn add_root_field(bytes: &[u8], field: &str) -> Vec<u8> {
        let mut changed = bytes.to_vec();
        assert_eq!(changed.pop(), Some(b'}'));
        changed.extend_from_slice(b",");
        changed.extend_from_slice(field.as_bytes());
        changed.push(b'}');
        changed
    }

    #[test]
    fn sha256_matches_standard_vectors_and_pinned_configs() {
        assert_eq!(
            digest_bytes(b"").sha256,
            super::decode_hex_32(
                b"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            )
        );
        assert_eq!(
            digest_bytes(b"abc").sha256,
            super::decode_hex_32(
                b"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
            )
        );
        let mut incremental = super::Sha256::new();
        incremental.update(b"a");
        incremental.update(b"b");
        incremental.update(b"c");
        assert_eq!(
            incremental.finish(),
            super::decode_hex_32(
                b"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
            )
        );
        assert_eq!(
            digest_bytes(pinned_config(TARGET_CONFIG)).sha256,
            super::TARGET_CONFIG_SHA256
        );
        assert_eq!(
            digest_bytes(pinned_config(DRAFT_CONFIG)).sha256,
            super::DRAFT_CONFIG_SHA256
        );
        assert_eq!(
            digest_bytes(TOKENIZER_METADATA).sha256,
            super::TOKENIZER_METADATA_SHA256
        );
        assert_eq!(
            super::target_weight_set_identity(),
            QWEN3_TARGET_WEIGHT_SET_SHA256
        );
    }

    #[test]
    fn canonical_bundle_has_one_deterministic_identity() {
        let first = build_preliminary_deployment_bundle(canonical_assets())
            .expect("canonical preliminary bundle");
        let second = build_preliminary_deployment_bundle(canonical_assets())
            .expect("canonical preliminary bundle");

        assert_eq!(first, second);
        assert_eq!(
            first.target_model.config.config_id.as_bytes(),
            &super::TARGET_CONFIG_SHA256
        );
        assert_eq!(
            first.draft_model.config.config_id.as_bytes(),
            &super::DRAFT_CONFIG_SHA256
        );
        assert_eq!(
            first.target_model.tokenizer.tokenizer_id.as_bytes(),
            &super::TOKENIZER_METADATA_SHA256
        );
        assert_eq!(
            first.target_model.tokenizer.vocabulary_id.as_bytes(),
            &QWEN3_TOKENIZER_SHA256
        );
        assert_eq!(
            first.target_model.config.model_id.as_bytes(),
            &super::decode_hex_32(
                b"f18fc461576d1a3053a19aba5946ef7b3b45aaf7cbb45d77f5c276f18567224a"
            )
        );
        assert_eq!(
            first.draft_model.config.model_id.as_bytes(),
            &super::decode_hex_32(
                b"351fc121a569f0a53e9bb5c98caaeff80d6f8d94737eecf5e179cfa54d9cf998"
            )
        );
        assert_eq!(
            first.bundle_id.as_bytes(),
            &super::decode_hex_32(
                b"6dfba0acd1c00ce13cec7b5eebb180691bdb8855a7eee89876df2a0a12a2802b"
            )
        );
        assert_eq!(
            first.target_model.weights.total_bytes,
            QWEN3_TARGET_WEIGHT_ARTIFACT_BYTES
        );
        assert_eq!(
            first.draft_model.weights.total_bytes,
            QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES
        );
    }

    #[test]
    fn unknown_and_duplicate_fields_are_rejected() {
        let unknown = add_root_field(pinned_config(TARGET_CONFIG), r#""future_field":1"#);
        let mut assets = canonical_assets();
        assets.target.config_json = &unknown;
        assert!(matches!(
            build_preliminary_deployment_bundle(assets),
            Err(BuildError::UnknownField { ref field, .. }) if field == "future_field"
        ));

        let tokenizer = std::str::from_utf8(TOKENIZER_METADATA).expect("fixture is UTF-8");
        let duplicate = tokenizer.replacen(
            r#""content": "<|endoftext|>","#,
            r#""content": "<|endoftext|>", "content": "<|endoftext|>","#,
            1,
        );
        assert_ne!(duplicate, tokenizer);
        let mut assets = canonical_assets();
        assets.target.tokenizer_metadata_json = duplicate.as_bytes();
        assert!(matches!(
            build_preliminary_deployment_bundle(assets),
            Err(BuildError::InvalidJson { ref reason, .. }) if reason.contains("duplicate field")
        ));
    }

    #[test]
    fn remote_code_is_rejected_at_any_depth() {
        let remote_config = add_root_field(
            pinned_config(TARGET_CONFIG),
            r#""auto_map":{"AutoModel":"modeling_qwen3.Custom"}"#,
        );
        let mut assets = canonical_assets();
        assets.target.config_json = &remote_config;
        assert_eq!(
            build_preliminary_deployment_bundle(assets),
            Err(BuildError::RemoteCode("auto_map".to_owned()))
        );

        let tokenizer = std::str::from_utf8(TOKENIZER_METADATA).expect("fixture is UTF-8");
        let nested = tokenizer.replacen(
            r#""special": true"#,
            r#""special": true, "trust_remote_code": false"#,
            1,
        );
        let mut assets = canonical_assets();
        assets.target.tokenizer_metadata_json = nested.as_bytes();
        assert_eq!(
            build_preliminary_deployment_bundle(assets),
            Err(BuildError::RemoteCode("trust_remote_code".to_owned()))
        );
    }

    #[test]
    fn schema_and_raw_byte_drift_are_distinct_failures() {
        let tokenizer = std::str::from_utf8(TOKENIZER_METADATA).expect("fixture is UTF-8");
        let custom_class = tokenizer.replacen("Qwen2Tokenizer", "CustomTokenizer", 1);
        let mut assets = canonical_assets();
        assets.target.tokenizer_metadata_json = custom_class.as_bytes();
        assert!(matches!(
            build_preliminary_deployment_bundle(assets),
            Err(BuildError::UnexpectedValue { ref field, .. }) if field == "tokenizer_class"
        ));

        let mut reencoded = pinned_config(TARGET_CONFIG).to_vec();
        reencoded.extend_from_slice(b" \n");
        let mut assets = canonical_assets();
        assets.target.config_json = &reencoded;
        assert_eq!(
            build_preliminary_deployment_bundle(assets),
            Err(BuildError::DigestMismatch("target config.json"))
        );
    }

    #[test]
    fn source_and_opaque_descriptors_are_pinned() {
        let mut assets = canonical_assets();
        assets.target.revision = DRAFT_REVISION;
        assert_eq!(
            build_preliminary_deployment_bundle(assets),
            Err(BuildError::SourceMismatch(
                ferric_spec::Qwen3ModelRole::Target8B
            ))
        );

        let mut assets = canonical_assets();
        assets.draft.vocabulary.sha256[0] ^= 1;
        assert_eq!(
            build_preliminary_deployment_bundle(assets),
            Err(BuildError::DescriptorMismatch("tokenizer.json"))
        );

        let mut assets = canonical_assets();
        assets.target.weights.artifact_bytes = QWEN3_TARGET_TENSOR_DATA_BYTES;
        assert_eq!(
            build_preliminary_deployment_bundle(assets),
            Err(BuildError::DescriptorMismatch("target weight set"))
        );

        let mut assets = canonical_assets();
        assets.draft.weights.tensor_data_bytes += 1;
        assert_eq!(
            build_preliminary_deployment_bundle(assets),
            Err(BuildError::DescriptorMismatch("draft weight file"))
        );
    }

    #[test]
    fn final_spec_validation_is_required() {
        let mut assets = canonical_assets();
        assets.limits.max_active_sequences = 33;
        assert_eq!(
            build_preliminary_deployment_bundle(assets),
            Err(BuildError::Spec(SpecError::ExceedsM1Envelope(
                "max_active_sequences"
            )))
        );
    }

    #[test]
    fn parser_bounds_and_trailing_data_fail_closed() {
        let oversized = vec![b' '; super::MAX_CONFIG_BYTES + 1];
        let mut assets = canonical_assets();
        assets.target.config_json = &oversized;
        assert_eq!(
            build_preliminary_deployment_bundle(assets),
            Err(BuildError::ArtifactTooLarge("target config.json"))
        );

        let mut trailing = pinned_config(DRAFT_CONFIG).to_vec();
        trailing.extend_from_slice(b" false");
        let mut assets = canonical_assets();
        assets.draft.config_json = &trailing;
        assert!(matches!(
            build_preliminary_deployment_bundle(assets),
            Err(BuildError::InvalidJson { ref reason, .. }) if reason.contains("trailing data")
        ));
    }
}
