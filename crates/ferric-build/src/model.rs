//! Exact Qwen3 target/draft model admission and deployment assembly.

use crate::json::{self, Value};
use crate::safetensors::AuthenticatedWeightSet;
use crate::sha256::{self, Sha256};
use crate::tokenizer::AuthenticatedTokenizer;
use crate::weight_stream::{PrepackedWeightSet, WeightSectionManifest};
use ferric_spec::{
    DeploymentBundle, EngineLimits, Identity, ModelArtifact, ModelConfig, NumericalPolicy,
    Qwen3ModelRole, SpecError, Target, TokenizerConfig, WeightManifest, QWEN3_END_OF_TEXT_TOKEN,
    QWEN3_IM_END_TOKEN, QWEN3_IM_START_TOKEN, QWEN3_VOCABULARY_SIZE,
};
use std::collections::BTreeMap;
use std::fmt;
use vstd::prelude::*;
#[allow(unused_imports)]
use vstd::seq_lib::*;
#[allow(unused_imports)]
use vstd::string::StringSliceAdditionalSpecFns;

const MAX_CONFIG_BYTES: usize = 16 * 1_024;
const MAX_TOKENIZER_METADATA_BYTES: usize = 64 * 1_024;
const MAX_CHAT_TEMPLATE_BYTES: usize = 32 * 1_024;

verus! {

type PinnedStr = &'static str;

/// Pinned upstream repository for the M1 target model.
pub const TARGET_REPOSITORY: PinnedStr = "Qwen/Qwen3-8B";
/// Pinned upstream revision for the M1 target model.
pub const TARGET_REVISION: PinnedStr = "b968826d9c46dd6066d109eabc6255188de91218";
/// Pinned upstream repository for the M1 draft model.
pub const DRAFT_REPOSITORY: PinnedStr = "Qwen/Qwen3-0.6B";
/// Pinned upstream revision for the M1 draft model.
pub const DRAFT_REVISION: PinnedStr = "c1899de289a04d12100db370d81485cdf75e47ca";

/// Size of the shared upstream `tokenizer.json` payload.
pub const QWEN3_TOKENIZER_BYTES: u64 = 11_422_654;
/// Complete pinned Qwen3-8B upstream `config.json` bytes.
pub const QWEN3_TARGET_CONFIG_BYTES: u64 = 728;
/// Complete pinned Qwen3-0.6B upstream `config.json` bytes.
pub const QWEN3_DRAFT_CONFIG_BYTES: u64 = 726;
/// Complete pinned shared upstream `tokenizer_config.json` bytes.
pub const QWEN3_TOKENIZER_METADATA_BYTES: u64 = 9_732;
/// Total file bytes across the five pinned Qwen3-8B safetensors shards.
pub const QWEN3_TARGET_WEIGHT_ARTIFACT_BYTES: u64 = 16_381_516_776;
/// Tensor-data bytes declared by the pinned Qwen3-8B safetensors index.
pub const QWEN3_TARGET_TENSOR_DATA_BYTES: u64 = 16_381_470_720;
/// Complete file bytes in the pinned Qwen3-0.6B safetensors artifact.
pub const QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES: u64 = 1_503_300_328;
/// Tensor-data bytes following the Qwen3-0.6B safetensors header.
pub const QWEN3_DRAFT_TENSOR_DATA_BYTES: u64 = 1_503_264_768;

pub(crate) const TARGET_CONFIG_SHA256: [u8; 32] = [
    247, 196, 234, 223, 187, 245, 34, 71, 6, 103, 183, 151, 163, 200, 155, 226, 82, 72, 50,
    210, 213, 153, 121, 114, 72, 220, 48, 79, 255, 68, 124, 48,
];
pub(crate) const DRAFT_CONFIG_SHA256: [u8; 32] = [
    102, 13, 179, 183, 61, 120, 129, 25, 192, 69, 53, 228, 140, 249, 190, 95, 85, 188, 49, 0,
    132, 26, 113, 134, 55, 174, 105, 91, 68, 47, 39, 221,
];
pub(crate) const TOKENIZER_METADATA_SHA256: [u8; 32] = [
    213, 208, 159, 7, 180, 140, 48, 134, 197, 8, 179, 13, 28, 145, 20, 189, 17, 137, 20, 91,
    116, 233, 130, 162, 101, 53, 12, 146, 58, 205, 129, 1,
];
/// SHA-256 identity of the shared upstream `tokenizer.json` LFS object.
pub const QWEN3_TOKENIZER_SHA256: [u8; 32] = [
    174, 177, 51, 7, 167, 26, 205, 143, 232, 24, 97, 217, 74, 213, 74, 182, 137, 223, 119,
    51, 24, 128, 158, 237, 60, 190, 121, 75, 68, 146, 218, 228,
];
/// Canonical target weight-set descriptor identity.
///
/// This is the domain-separated, length-prefixed SHA-256 record over the
/// pinned index name/SHA-256/size followed by all five ordered shard
/// name/SHA-256/size tuples. It is not a hash of concatenated weight bytes.
pub const QWEN3_TARGET_WEIGHT_SET_SHA256: [u8; 32] = [
    46, 105, 192, 137, 255, 154, 252, 238, 38, 70, 70, 203, 142, 166, 52, 74, 163, 200, 206,
    219, 232, 2, 45, 114, 152, 137, 112, 130, 4, 227, 39, 50,
];
/// SHA-256 of the complete pinned Qwen3-0.6B safetensors file.
pub const QWEN3_DRAFT_WEIGHT_SHA256: [u8; 32] = [
    244, 127, 113, 23, 127, 50, 188, 209, 1, 183, 87, 62, 201, 23, 30, 106, 87, 244, 244,
    211, 17, 72, 211, 142, 56, 35, 6, 244, 41, 150, 135, 75,
];
/// Domain-separated identity of the exact admitted Qwen3-8B model inputs.
pub const QWEN3_TARGET_MODEL_ID: [u8; 32] = [
    241, 143, 196, 97, 87, 109, 26, 48, 83, 161, 154, 186, 89, 70, 239, 123, 59, 69, 170,
    247, 203, 180, 93, 119, 245, 194, 118, 241, 133, 103, 34, 74,
];
/// Domain-separated identity of the exact admitted Qwen3-0.6B model inputs.
pub const QWEN3_DRAFT_MODEL_ID: [u8; 32] = [
    53, 31, 193, 33, 165, 105, 240, 165, 62, 155, 181, 201, 140, 170, 239, 248, 13, 111, 141,
    148, 115, 126, 236, 245, 225, 121, 207, 165, 77, 156, 249, 152,
];

} // verus!

pub(crate) const ADDED_TOKENS: [(&str, bool); 26] = [
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

verus! {

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

verus! {

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
    pub(crate) closed spec fn deployment_spec(&self) -> DeploymentBundle {
        self.deployment
    }

    pub(crate) closed spec fn target_manifest_spec(&self) -> WeightSectionManifest {
        self.target_manifest
    }

    pub(crate) closed spec fn draft_manifest_spec(&self) -> WeightSectionManifest {
        self.draft_manifest
    }

    /// Returns the admitted executable deployment bundle.
    #[must_use]
    pub const fn deployment(&self) -> &DeploymentBundle {
        self.deployment_exact()
    }

    pub(crate) const fn deployment_exact(&self) -> (deployment: &DeploymentBundle)
        ensures *deployment == self.deployment_spec(),
    {
        &self.deployment
    }

    /// Returns the exact target prepacked-output manifest.
    #[must_use]
    pub const fn target_manifest(&self) -> &WeightSectionManifest {
        self.target_manifest_exact()
    }

    pub(crate) const fn target_manifest_exact(&self) -> (manifest: &WeightSectionManifest)
        ensures *manifest == self.target_manifest_spec(),
    {
        &self.target_manifest
    }

    /// Returns the exact draft prepacked-output manifest.
    #[must_use]
    pub const fn draft_manifest(&self) -> &WeightSectionManifest {
        self.draft_manifest_exact()
    }

    pub(crate) const fn draft_manifest_exact(&self) -> (manifest: &WeightSectionManifest)
        ensures *manifest == self.draft_manifest_spec(),
    {
        &self.draft_manifest
    }
}

fn prepacked_deployment_bundle(
    deployment: &DeploymentBundle,
    target_manifest: WeightSectionManifest,
    draft_manifest: WeightSectionManifest,
) -> (prepacked: PrepackedDeploymentBundle)
    ensures
        prepacked.deployment_spec() == *deployment,
        prepacked.target_manifest_spec() == target_manifest,
        prepacked.draft_manifest_spec() == draft_manifest,
{
    PrepackedDeploymentBundle {
        deployment: *deployment,
        target_manifest,
        draft_manifest,
    }
}

} // verus!

verus! {

/// Fail-closed bundle parsing or admission error.
#[derive(Clone, Debug, PartialEq, Eq)]
#[verifier::allow(autoderive_clone_without_spec)]
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

} // verus!

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
/// The prepacked authorities can be produced by fresh source streaming or by
/// reopening bytes under the exact compiled canonical-manifest trust anchors;
/// caller-asserted descriptors cannot enter this path. The result retains both
/// canonical prepacked manifests next to the executable bundle so their
/// aggregate identities are not discarded.
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
    Ok(prepacked_deployment_bundle(
        &deployment,
        target_manifest,
        draft_manifest,
    ))
}

fn assemble_deployment_bundle(
    assets: &DeploymentAssets<'_>,
) -> Result<DeploymentBundle, BuildError> {
    let target_config = parse_config(Qwen3ModelRole::Target8B, &assets.target)?;
    let draft_config = parse_config(Qwen3ModelRole::Draft06B, &assets.draft)?;
    let target_tokenizer = parse_tokenizer(&assets.target)?;
    let draft_tokenizer = parse_tokenizer(&assets.draft)?;
    admit_parsed_deployment(ParsedDeploymentInputs {
        limits: assets.limits,
        target_repository: assets.target.repository,
        target_revision: assets.target.revision,
        draft_repository: assets.draft.repository,
        draft_revision: assets.draft.revision,
        target_config,
        draft_config,
        target_tokenizer,
        draft_tokenizer,
        target_descriptor: assets.target.weights,
        draft_descriptor: assets.draft.weights,
        target_tensor_data_bytes: assets.target.weights.tensor_data_bytes,
        draft_tensor_data_bytes: assets.draft.weights.tensor_data_bytes,
    })
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

verus! {

closed spec fn byte_slices_equal_spec(left: &[u8], right: &[u8]) -> bool {
    left@ == right@
}

fn byte_slices_equal(left: &[u8], right: &[u8]) -> (equal: bool)
    ensures equal == byte_slices_equal_spec(left, right),
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
        assert forall|position: int| 0 <= position < left@.len()
            implies left@[position] == right@[position] by {}
    }
    true
}

closed spec fn source_matches_role(
    role: Qwen3ModelRole,
    repository: &str,
    revision: &str,
) -> bool {
    &&& repository.spec_bytes().len() <= 64
    &&& revision.spec_bytes().len() <= 64
    &&& match role {
        Qwen3ModelRole::Target8B => {
            &&& repository.spec_bytes() == TARGET_REPOSITORY.spec_bytes()
            &&& revision.spec_bytes() == TARGET_REVISION.spec_bytes()
        }
        Qwen3ModelRole::Draft06B => {
            &&& repository.spec_bytes() == DRAFT_REPOSITORY.spec_bytes()
            &&& revision.spec_bytes() == DRAFT_REVISION.spec_bytes()
        }
    }
}

fn validate_source(
    role: Qwen3ModelRole,
    repository: &str,
    revision: &str,
) -> (result: Result<(), BuildError>)
    ensures result.is_ok() == source_matches_role(role, repository, revision),
{
    let expected = match role {
        Qwen3ModelRole::Target8B => (TARGET_REPOSITORY, TARGET_REVISION),
        Qwen3ModelRole::Draft06B => (DRAFT_REPOSITORY, DRAFT_REVISION),
    };
    let repository_bytes = repository.as_bytes();
    let revision_bytes = revision.as_bytes();
    let repository_matches = byte_slices_equal(repository_bytes, expected.0.as_bytes());
    let revision_matches = byte_slices_equal(revision_bytes, expected.1.as_bytes());
    if !repository_matches || !revision_matches {
        return Err(BuildError::SourceMismatch(role));
    }
    let repository_len = repository_bytes.len();
    let revision_len = revision_bytes.len();
    if repository_len > 64 || revision_len > 64 {
        return Err(BuildError::SourceMismatch(role));
    }
    assert(repository_len <= 64);
    assert(revision_len <= 64);
    assert(repository_len as nat == repository_bytes@.len());
    assert(revision_len as nat == revision_bytes@.len());
    assert(repository.spec_bytes().len() <= 64);
    assert(revision.spec_bytes().len() <= 64);
    proof {
        match role {
            Qwen3ModelRole::Target8B => {
                assert(repository.spec_bytes() == TARGET_REPOSITORY.spec_bytes());
                assert(revision.spec_bytes() == TARGET_REVISION.spec_bytes());
            }
            Qwen3ModelRole::Draft06B => {
                assert(repository.spec_bytes() == DRAFT_REPOSITORY.spec_bytes());
                assert(revision.spec_bytes() == DRAFT_REVISION.spec_bytes());
            }
        }
    }
    assert(source_matches_role(role, repository, revision));
    Ok(())
}

closed spec fn weight_descriptor_matches_role(
    role: Qwen3ModelRole,
    descriptor: WeightDescriptor,
) -> bool {
    match role {
        Qwen3ModelRole::Target8B => {
            &&& descriptor.weights_id@ == QWEN3_TARGET_WEIGHT_SET_SHA256@
            &&& descriptor.artifact_bytes == QWEN3_TARGET_WEIGHT_ARTIFACT_BYTES
            &&& descriptor.tensor_data_bytes == QWEN3_TARGET_TENSOR_DATA_BYTES
            &&& descriptor.sections == 5
        }
        Qwen3ModelRole::Draft06B => {
            &&& descriptor.weights_id@ == QWEN3_DRAFT_WEIGHT_SHA256@
            &&& descriptor.artifact_bytes == QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES
            &&& descriptor.tensor_data_bytes == QWEN3_DRAFT_TENSOR_DATA_BYTES
            &&& descriptor.sections == 1
        }
    }
}

closed spec fn weight_manifest_refines_descriptor(
    descriptor: WeightDescriptor,
    manifest: WeightManifest,
) -> bool {
    &&& manifest.weights_id.bytes_spec() == descriptor.weights_id@
    &&& manifest.total_bytes == descriptor.artifact_bytes
    &&& manifest.sections == descriptor.sections
}

fn weight_manifest(
    role: Qwen3ModelRole,
    descriptor: WeightDescriptor,
) -> (result: Result<WeightManifest, BuildError>)
    ensures match result {
        Ok(manifest) => {
            &&& weight_descriptor_matches_role(role, descriptor)
            &&& weight_manifest_refines_descriptor(descriptor, manifest)
        }
        Err(_) => !weight_descriptor_matches_role(role, descriptor),
    },
{
    let (expected_sha256, artifact_bytes, tensor_data_bytes, sections, artifact) = match role {
        Qwen3ModelRole::Target8B => (
            QWEN3_TARGET_WEIGHT_SET_SHA256,
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
    if !byte_slices_equal(&descriptor.weights_id, &expected_sha256)
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

} // verus!

verus! {

const MODEL_IDENTITY_DOMAIN: [u8; 15] = [
    102, 101, 114, 114, 105, 99, 46, 109, 111, 100, 101, 108, 46, 118, 49,
];
const BUNDLE_IDENTITY_DOMAIN: [u8; 27] = [
    102, 101, 114, 114, 105, 99, 46, 100, 101, 112, 108, 111, 121, 109, 101, 110, 116, 45, 98,
    117, 110, 100, 108, 101, 46, 118, 49,
];
const GFX942_XNACK_MINUS_IDENTITY_FIELD: [u8; 13] = [
    103, 102, 120, 57, 52, 50, 58, 120, 110, 97, 99, 107, 45,
];
const BF16_FP32_IDENTITY_FIELD: [u8; 33] = [
    98, 102, 49, 54, 45, 112, 97, 114, 97, 109, 101, 116, 101, 114, 115, 58, 102, 112, 51, 50,
    45, 97, 99, 99, 117, 109, 117, 108, 97, 116, 105, 111, 110,
];

pub(crate) open spec fn u32_big_endian(value: u32) -> Seq<u8> {
    seq![
        ((value >> 24) % 256) as u8,
        ((value >> 16) % 256) as u8,
        ((value >> 8) % 256) as u8,
        (value % 256) as u8,
    ]
}

pub(crate) open spec fn u64_big_endian(value: u64) -> Seq<u8> {
    seq![
        ((value >> 56) % 256) as u8,
        ((value >> 48) % 256) as u8,
        ((value >> 40) % 256) as u8,
        ((value >> 32) % 256) as u8,
        ((value >> 24) % 256) as u8,
        ((value >> 16) % 256) as u8,
        ((value >> 8) % 256) as u8,
        (value % 256) as u8,
    ]
}

pub(crate) fn encode_u32_big_endian(value: u32) -> (encoded: [u8; 4])
    ensures encoded@ == u32_big_endian(value),
{
    [
        u8::try_from((value >> 24) % 256).expect("big-endian byte fits u8"),
        u8::try_from((value >> 16) % 256).expect("big-endian byte fits u8"),
        u8::try_from((value >> 8) % 256).expect("big-endian byte fits u8"),
        u8::try_from(value % 256).expect("big-endian byte fits u8"),
    ]
}

pub(crate) fn encode_u64_big_endian(value: u64) -> (encoded: [u8; 8])
    ensures encoded@ == u64_big_endian(value),
{
    [
        u8::try_from((value >> 56) % 256).expect("big-endian byte fits u8"),
        u8::try_from((value >> 48) % 256).expect("big-endian byte fits u8"),
        u8::try_from((value >> 40) % 256).expect("big-endian byte fits u8"),
        u8::try_from((value >> 32) % 256).expect("big-endian byte fits u8"),
        u8::try_from((value >> 24) % 256).expect("big-endian byte fits u8"),
        u8::try_from((value >> 16) % 256).expect("big-endian byte fits u8"),
        u8::try_from((value >> 8) % 256).expect("big-endian byte fits u8"),
        u8::try_from(value % 256).expect("big-endian byte fits u8"),
    ]
}

pub(crate) closed spec fn identity_field_preimage(bytes: Seq<u8>) -> Seq<u8> {
    u64_big_endian(bytes.len() as u64) + bytes
}

closed spec fn field_views(fields: &[&[u8]]) -> Seq<Seq<u8>> {
    Seq::new(fields@.len(), |index: int| fields@[index]@)
}

closed spec fn identity_fields_preimage(fields: Seq<Seq<u8>>, count: nat) -> Seq<u8>
    recommends count <= fields.len(),
    decreases count,
{
    if count == 0 {
        Seq::empty()
    } else {
        identity_fields_preimage(fields, (count - 1) as nat)
            + identity_field_preimage(fields[(count - 1) as int])
    }
}

closed spec fn identity_record_preimage(domain: Seq<u8>, fields: Seq<Seq<u8>>) -> Seq<u8> {
    identity_field_preimage(domain) + identity_fields_preimage(fields, fields.len())
}

proof fn identity_field_preimage_len(bytes: Seq<u8>)
    ensures identity_field_preimage(bytes).len() == 8 + bytes.len(),
{
}

proof fn identity_fields_preimage_len(fields: Seq<Seq<u8>>, count: nat)
    requires count <= fields.len(),
    ensures
        identity_fields_preimage(fields, count).len()
            == if count == 0 {
                0
            } else {
                identity_fields_preimage(fields, (count - 1) as nat).len()
                    + 8
                    + fields[(count - 1) as int].len()
            },
{
    if count > 0 {
        identity_field_preimage_len(fields[(count - 1) as int]);
    }
}

proof fn identity_fields_prefix_is_bounded(fields: Seq<Seq<u8>>, count: nat)
    requires count <= fields.len(),
    ensures
        identity_fields_preimage(fields, count).len()
            <= identity_fields_preimage(fields, fields.len()).len(),
    decreases fields.len() - count,
{
    if count < fields.len() {
        identity_fields_prefix_is_bounded(fields, count + 1);
    }
}

proof fn identity_record_prefix_is_bounded(
    domain: Seq<u8>,
    fields: Seq<Seq<u8>>,
    count: nat,
)
    requires count <= fields.len(),
    ensures
        (identity_field_preimage(domain) + identity_fields_preimage(fields, count)).len()
            <= identity_record_preimage(domain, fields).len(),
{
    identity_fields_prefix_is_bounded(fields, count);
}

closed spec fn model_identity_fields(
    role: Qwen3ModelRole,
    repository: &str,
    revision: &str,
    config_id: Identity,
    tokenizer: TokenizerConfig,
    weights: WeightManifest,
    tensor_data_bytes: u64,
) -> Seq<Seq<u8>> {
    seq![
        seq![match role {
            Qwen3ModelRole::Target8B => 0,
            Qwen3ModelRole::Draft06B => 1,
        }],
        repository.spec_bytes(),
        revision.spec_bytes(),
        config_id.bytes_spec(),
        tokenizer.tokenizer_id.bytes_spec(),
        tokenizer.vocabulary_id.bytes_spec(),
        weights.weights_id.bytes_spec(),
        u64_big_endian(weights.total_bytes),
        u64_big_endian(tensor_data_bytes),
        u32_big_endian(weights.sections),
    ]
}

pub(crate) closed spec fn model_identity_preimage(
    role: Qwen3ModelRole,
    repository: &str,
    revision: &str,
    config_id: Identity,
    tokenizer: TokenizerConfig,
    weights: WeightManifest,
    tensor_data_bytes: u64,
) -> Seq<u8> {
    identity_record_preimage(
        MODEL_IDENTITY_DOMAIN@,
        model_identity_fields(
            role,
            repository,
            revision,
            config_id,
            tokenizer,
            weights,
            tensor_data_bytes,
        ),
    )
}

pub(crate) proof fn model_identity_preimage_len(
    role: Qwen3ModelRole,
    repository: &str,
    revision: &str,
    config_id: Identity,
    tokenizer: TokenizerConfig,
    weights: WeightManifest,
    tensor_data_bytes: u64,
)
    ensures
        model_identity_preimage(
            role,
            repository,
            revision,
            config_id,
            tokenizer,
            weights,
            tensor_data_bytes,
        ).len() == 252 + repository.spec_bytes().len() + revision.spec_bytes().len(),
{
    config_id.bytes_spec_len();
    tokenizer.tokenizer_id.bytes_spec_len();
    tokenizer.vocabulary_id.bytes_spec_len();
    weights.weights_id.bytes_spec_len();
    let fields = model_identity_fields(
        role,
        repository,
        revision,
        config_id,
        tokenizer,
        weights,
        tensor_data_bytes,
    );
    assert(fields.len() == 10);
    assert(fields[0].len() == 1);
    assert(fields[1].len() == repository.spec_bytes().len());
    assert(fields[2].len() == revision.spec_bytes().len());
    assert(fields[3].len() == 32);
    assert(fields[4].len() == 32);
    assert(fields[5].len() == 32);
    assert(fields[6].len() == 32);
    assert(fields[7].len() == 8);
    assert(fields[8].len() == 8);
    assert(fields[9].len() == 4);
    identity_field_preimage_len(MODEL_IDENTITY_DOMAIN@);
    identity_fields_preimage_len(fields, 1);
    identity_fields_preimage_len(fields, 2);
    identity_fields_preimage_len(fields, 3);
    identity_fields_preimage_len(fields, 4);
    identity_fields_preimage_len(fields, 5);
    identity_fields_preimage_len(fields, 6);
    identity_fields_preimage_len(fields, 7);
    identity_fields_preimage_len(fields, 8);
    identity_fields_preimage_len(fields, 9);
    identity_fields_preimage_len(fields, 10);
}

pub(crate) fn model_identity(
    role: Qwen3ModelRole,
    repository: &str,
    revision: &str,
    config_id: Identity,
    tokenizer: TokenizerConfig,
    weights: WeightManifest,
    tensor_data_bytes: u64,
) -> (identity: Identity)
    requires
        model_identity_preimage(
            role,
            repository,
            revision,
            config_id,
            tokenizer,
            weights,
            tensor_data_bytes,
        ).len() <= u64::MAX / 8,
    ensures
        identity.bytes_spec() == sha256::digest_spec(model_identity_preimage(
            role,
            repository,
            revision,
            config_id,
            tokenizer,
            weights,
            tensor_data_bytes,
        )),
{
    let role_byte = [match role {
        Qwen3ModelRole::Target8B => 0,
        Qwen3ModelRole::Draft06B => 1,
    }];
    let weight_bytes = encode_u64_big_endian(weights.total_bytes);
    let tensor_bytes = encode_u64_big_endian(tensor_data_bytes);
    let weight_sections = encode_u32_big_endian(weights.sections);
    let fields: [&[u8]; 10] = [
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
    ];
    assert(field_views(&fields) =~= model_identity_fields(
        role,
        repository,
        revision,
        config_id,
        tokenizer,
        weights,
        tensor_data_bytes,
    )) by {
        assert forall|index: int| 0 <= index < 10 implies
            field_views(&fields)[index] == model_identity_fields(
                role,
                repository,
                revision,
                config_id,
                tokenizer,
                weights,
                tensor_data_bytes,
            )[index] by {
            if index == 0 {} else if index == 1 {} else if index == 2 {}
            else if index == 3 {} else if index == 4 {} else if index == 5 {}
            else if index == 6 {} else if index == 7 {} else if index == 8 {}
            else {}
        }
    }
    record_identity(&MODEL_IDENTITY_DOMAIN, &fields)
}

closed spec fn bundle_identity_fields(
    limits: EngineLimits,
    target: ModelArtifact,
    draft: ModelArtifact,
) -> Seq<Seq<u8>> {
    seq![
        GFX942_XNACK_MINUS_IDENTITY_FIELD@,
        BF16_FP32_IDENTITY_FIELD@,
        u32_big_endian(limits.max_context_tokens),
        u32_big_endian(limits.max_active_sequences),
        u32_big_endian(limits.kv_page_tokens),
        u32_big_endian(limits.max_draft_tokens),
        target.config.model_id.bytes_spec(),
        draft.config.model_id.bytes_spec(),
    ]
}

pub(crate) closed spec fn bundle_identity_preimage(
    limits: EngineLimits,
    target: ModelArtifact,
    draft: ModelArtifact,
) -> Seq<u8> {
    identity_record_preimage(
        BUNDLE_IDENTITY_DOMAIN@,
        bundle_identity_fields(limits, target, draft),
    )
}

proof fn bundle_identity_preimage_len(
    limits: EngineLimits,
    target: ModelArtifact,
    draft: ModelArtifact,
)
    ensures bundle_identity_preimage(limits, target, draft).len() == 225,
{
    target.config.model_id.bytes_spec_len();
    draft.config.model_id.bytes_spec_len();
    let fields = bundle_identity_fields(limits, target, draft);
    assert(fields.len() == 8);
    assert(fields[0].len() == 13);
    assert(fields[1].len() == 33);
    assert(fields[2].len() == 4);
    assert(fields[3].len() == 4);
    assert(fields[4].len() == 4);
    assert(fields[5].len() == 4);
    assert(fields[6].len() == 32);
    assert(fields[7].len() == 32);
    identity_field_preimage_len(BUNDLE_IDENTITY_DOMAIN@);
    identity_fields_preimage_len(fields, 1);
    identity_fields_preimage_len(fields, 2);
    identity_fields_preimage_len(fields, 3);
    identity_fields_preimage_len(fields, 4);
    identity_fields_preimage_len(fields, 5);
    identity_fields_preimage_len(fields, 6);
    identity_fields_preimage_len(fields, 7);
    identity_fields_preimage_len(fields, 8);
}

pub(crate) fn bundle_identity(
    limits: EngineLimits,
    target: ModelArtifact,
    draft: ModelArtifact,
) -> (identity: Identity)
    ensures
        identity.bytes_spec()
            == sha256::digest_spec(bundle_identity_preimage(limits, target, draft)),
{
    proof {
        bundle_identity_preimage_len(limits, target, draft);
    }
    let context = encode_u32_big_endian(limits.max_context_tokens);
    let sequences = encode_u32_big_endian(limits.max_active_sequences);
    let page = encode_u32_big_endian(limits.kv_page_tokens);
    let draft_tokens = encode_u32_big_endian(limits.max_draft_tokens);
    let fields: [&[u8]; 8] = [
        &GFX942_XNACK_MINUS_IDENTITY_FIELD,
        &BF16_FP32_IDENTITY_FIELD,
        &context,
        &sequences,
        &page,
        &draft_tokens,
        target.config.model_id.as_bytes(),
        draft.config.model_id.as_bytes(),
    ];
    assert(field_views(&fields) =~= bundle_identity_fields(limits, target, draft)) by {
        assert forall|index: int| 0 <= index < 8 implies
            field_views(&fields)[index] == bundle_identity_fields(limits, target, draft)[index] by {
            if index == 0 {} else if index == 1 {} else if index == 2 {}
            else if index == 3 {} else if index == 4 {} else if index == 5 {}
            else if index == 6 {} else {}
        }
    }
    record_identity(&BUNDLE_IDENTITY_DOMAIN, &fields)
}

closed spec fn config_preserved_except_model_id(
    admitted: ModelConfig,
    parsed: ModelConfig,
) -> bool {
    &&& admitted.role == parsed.role
    &&& admitted.config_id.bytes_spec() == parsed.config_id.bytes_spec()
    &&& admitted.vocabulary_size == parsed.vocabulary_size
    &&& admitted.layers == parsed.layers
    &&& admitted.hidden_size == parsed.hidden_size
    &&& admitted.intermediate_size == parsed.intermediate_size
    &&& admitted.query_heads == parsed.query_heads
    &&& admitted.kv_heads == parsed.kv_heads
    &&& admitted.head_dim == parsed.head_dim
    &&& admitted.max_position_embeddings == parsed.max_position_embeddings
    &&& admitted.rope_theta == parsed.rope_theta
    &&& admitted.tie_word_embeddings == parsed.tie_word_embeddings
}

fn config_with_model_identity(
    parsed: ModelConfig,
    model_id: Identity,
) -> (admitted: ModelConfig)
    ensures
        config_preserved_except_model_id(admitted, parsed),
        admitted.role == parsed.role,
        admitted.config_id.bytes_spec() == parsed.config_id.bytes_spec(),
        admitted.model_id.bytes_spec() == model_id.bytes_spec(),
{
    ModelConfig {
        role: parsed.role,
        model_id,
        config_id: parsed.config_id,
        vocabulary_size: parsed.vocabulary_size,
        layers: parsed.layers,
        hidden_size: parsed.hidden_size,
        intermediate_size: parsed.intermediate_size,
        query_heads: parsed.query_heads,
        kv_heads: parsed.kv_heads,
        head_dim: parsed.head_dim,
        max_position_embeddings: parsed.max_position_embeddings,
        rope_theta: parsed.rope_theta,
        tie_word_embeddings: parsed.tie_word_embeddings,
    }
}

closed spec fn admitted_bundle_refines_inputs(
    bundle: DeploymentBundle,
    limits: EngineLimits,
    target_repository: &str,
    target_revision: &str,
    draft_repository: &str,
    draft_revision: &str,
    target_config: ModelConfig,
    draft_config: ModelConfig,
    target_tokenizer: TokenizerConfig,
    draft_tokenizer: TokenizerConfig,
    target_descriptor: WeightDescriptor,
    draft_descriptor: WeightDescriptor,
    target_tensor_data_bytes: u64,
    draft_tensor_data_bytes: u64,
) -> bool {
    &&& source_matches_role(
        Qwen3ModelRole::Target8B,
        target_repository,
        target_revision,
    )
    &&& source_matches_role(
        Qwen3ModelRole::Draft06B,
        draft_repository,
        draft_revision,
    )
    &&& weight_descriptor_matches_role(Qwen3ModelRole::Target8B, target_descriptor)
    &&& weight_descriptor_matches_role(Qwen3ModelRole::Draft06B, draft_descriptor)
    &&& target_tokenizer.compatible(draft_tokenizer)
    &&& bundle.target == Target::Gfx942XnackMinus
    &&& bundle.numerical_policy == NumericalPolicy::Bf16ParametersFp32Accumulation
    &&& bundle.limits == limits
    &&& bundle.target_model.config.role == Qwen3ModelRole::Target8B
    &&& bundle.draft_model.config.role == Qwen3ModelRole::Draft06B
    &&& weight_manifest_refines_descriptor(
        target_descriptor,
        bundle.target_model.weights,
    )
    &&& weight_manifest_refines_descriptor(
        draft_descriptor,
        bundle.draft_model.weights,
    )
    &&& bundle.target_model.config.model_id.bytes_spec() == sha256::digest_spec(
        model_identity_preimage(
            Qwen3ModelRole::Target8B,
            target_repository,
            target_revision,
            target_config.config_id,
            target_tokenizer,
            bundle.target_model.weights,
            target_tensor_data_bytes,
        ),
    )
    &&& bundle.draft_model.config.model_id.bytes_spec() == sha256::digest_spec(
        model_identity_preimage(
            Qwen3ModelRole::Draft06B,
            draft_repository,
            draft_revision,
            draft_config.config_id,
            draft_tokenizer,
            bundle.draft_model.weights,
            draft_tensor_data_bytes,
        ),
    )
    &&& bundle.bundle_id.bytes_spec() == sha256::digest_spec(bundle_identity_preimage(
        limits,
        bundle.target_model,
        bundle.draft_model,
    ))
    &&& bundle.valid()
}

struct ParsedDeploymentInputs<'a> {
    limits: EngineLimits,
    target_repository: &'a str,
    target_revision: &'a str,
    draft_repository: &'a str,
    draft_revision: &'a str,
    target_config: ModelConfig,
    draft_config: ModelConfig,
    target_tokenizer: TokenizerConfig,
    draft_tokenizer: TokenizerConfig,
    target_descriptor: WeightDescriptor,
    draft_descriptor: WeightDescriptor,
    target_tensor_data_bytes: u64,
    draft_tensor_data_bytes: u64,
}

fn admit_parsed_deployment(
    input: ParsedDeploymentInputs<'_>,
) -> (result: Result<DeploymentBundle, BuildError>)
    ensures match result {
        Ok(bundle) => admitted_bundle_refines_inputs(
            bundle,
            input.limits,
            input.target_repository,
            input.target_revision,
            input.draft_repository,
            input.draft_revision,
            input.target_config,
            input.draft_config,
            input.target_tokenizer,
            input.draft_tokenizer,
            input.target_descriptor,
            input.draft_descriptor,
            input.target_tensor_data_bytes,
            input.draft_tensor_data_bytes,
        ),
        Err(_) => true,
    },
{
    let limits = input.limits;
    let target_repository = input.target_repository;
    let target_revision = input.target_revision;
    let draft_repository = input.draft_repository;
    let draft_revision = input.draft_revision;
    let target_config = input.target_config;
    let draft_config = input.draft_config;
    let target_tokenizer = input.target_tokenizer;
    let draft_tokenizer = input.draft_tokenizer;
    let target_descriptor = input.target_descriptor;
    let draft_descriptor = input.draft_descriptor;
    let target_tensor_data_bytes = input.target_tensor_data_bytes;
    let draft_tensor_data_bytes = input.draft_tensor_data_bytes;

    validate_source(
        Qwen3ModelRole::Target8B,
        target_repository,
        target_revision,
    )?;
    validate_source(
        Qwen3ModelRole::Draft06B,
        draft_repository,
        draft_revision,
    )?;
    match target_config.role {
        Qwen3ModelRole::Target8B => {}
        Qwen3ModelRole::Draft06B => {
            return Err(BuildError::Spec(SpecError::UnexpectedModelRole));
        }
    }
    match draft_config.role {
        Qwen3ModelRole::Draft06B => {}
        Qwen3ModelRole::Target8B => {
            return Err(BuildError::Spec(SpecError::UnexpectedModelRole));
        }
    }
    if !target_tokenizer.is_compatible_with(draft_tokenizer) {
        return Err(BuildError::TokenizerMismatch);
    }
    let target_weights = weight_manifest(Qwen3ModelRole::Target8B, target_descriptor)?;
    let draft_weights = weight_manifest(Qwen3ModelRole::Draft06B, draft_descriptor)?;

    proof {
        model_identity_preimage_len(
            Qwen3ModelRole::Target8B,
            target_repository,
            target_revision,
            target_config.config_id,
            target_tokenizer,
            target_weights,
            target_tensor_data_bytes,
        );
        model_identity_preimage_len(
            Qwen3ModelRole::Draft06B,
            draft_repository,
            draft_revision,
            draft_config.config_id,
            draft_tokenizer,
            draft_weights,
            draft_tensor_data_bytes,
        );
    }
    assert(target_repository.spec_bytes().len() == TARGET_REPOSITORY.spec_bytes().len());
    assert(target_revision.spec_bytes().len() == TARGET_REVISION.spec_bytes().len());
    assert(draft_repository.spec_bytes().len() == DRAFT_REPOSITORY.spec_bytes().len());
    assert(draft_revision.spec_bytes().len() == DRAFT_REVISION.spec_bytes().len());
    assert(target_repository.spec_bytes().len() <= 64);
    assert(target_revision.spec_bytes().len() <= 64);
    assert(draft_repository.spec_bytes().len() <= 64);
    assert(draft_revision.spec_bytes().len() <= 64);
    assert(model_identity_preimage(
        Qwen3ModelRole::Target8B,
        target_repository,
        target_revision,
        target_config.config_id,
        target_tokenizer,
        target_weights,
        target_tensor_data_bytes,
    ).len() <= u64::MAX / 8);
    assert(model_identity_preimage(
        Qwen3ModelRole::Draft06B,
        draft_repository,
        draft_revision,
        draft_config.config_id,
        draft_tokenizer,
        draft_weights,
        draft_tensor_data_bytes,
    ).len() <= u64::MAX / 8);
    let target_model_id = model_identity(
        Qwen3ModelRole::Target8B,
        target_repository,
        target_revision,
        target_config.config_id,
        target_tokenizer,
        target_weights,
        target_tensor_data_bytes,
    );
    let draft_model_id = model_identity(
        Qwen3ModelRole::Draft06B,
        draft_repository,
        draft_revision,
        draft_config.config_id,
        draft_tokenizer,
        draft_weights,
        draft_tensor_data_bytes,
    );

    let target_model = ModelArtifact {
        config: config_with_model_identity(target_config, target_model_id),
        tokenizer: target_tokenizer,
        weights: target_weights,
    };
    let draft_model = ModelArtifact {
        config: config_with_model_identity(draft_config, draft_model_id),
        tokenizer: draft_tokenizer,
        weights: draft_weights,
    };
    let bundle = DeploymentBundle {
        bundle_id: bundle_identity(limits, target_model, draft_model),
        target: Target::Gfx942XnackMinus,
        numerical_policy: NumericalPolicy::Bf16ParametersFp32Accumulation,
        limits,
        target_model,
        draft_model,
    };
    match bundle.validate() {
        Ok(()) => {
            assert(source_matches_role(
                Qwen3ModelRole::Target8B,
                target_repository,
                target_revision,
            ));
            assert(source_matches_role(
                Qwen3ModelRole::Draft06B,
                draft_repository,
                draft_revision,
            ));
            assert(weight_descriptor_matches_role(
                Qwen3ModelRole::Target8B,
                target_descriptor,
            ));
            assert(weight_descriptor_matches_role(
                Qwen3ModelRole::Draft06B,
                draft_descriptor,
            ));
            assert(target_tokenizer.compatible(draft_tokenizer));
            assert(bundle.target == Target::Gfx942XnackMinus);
            assert(bundle.numerical_policy == NumericalPolicy::Bf16ParametersFp32Accumulation);
            assert(bundle.limits == limits);
            assert(bundle.target_model.config.role == Qwen3ModelRole::Target8B);
            assert(bundle.draft_model.config.role == Qwen3ModelRole::Draft06B);
            assert(weight_manifest_refines_descriptor(
                target_descriptor,
                bundle.target_model.weights,
            ));
            assert(weight_manifest_refines_descriptor(
                draft_descriptor,
                bundle.draft_model.weights,
            ));
            assert(bundle.target_model.config.model_id.bytes_spec() == sha256::digest_spec(
                model_identity_preimage(
                    Qwen3ModelRole::Target8B,
                    target_repository,
                    target_revision,
                    target_config.config_id,
                    target_tokenizer,
                    bundle.target_model.weights,
                    target_tensor_data_bytes,
                ),
            ));
            assert(bundle.draft_model.config.model_id.bytes_spec() == sha256::digest_spec(
                model_identity_preimage(
                    Qwen3ModelRole::Draft06B,
                    draft_repository,
                    draft_revision,
                    draft_config.config_id,
                    draft_tokenizer,
                    bundle.draft_model.weights,
                    draft_tensor_data_bytes,
                ),
            ));
            assert(bundle.bundle_id.bytes_spec() == sha256::digest_spec(bundle_identity_preimage(
                limits,
                bundle.target_model,
                bundle.draft_model,
            )));
            assert(bundle.valid());
            Ok(bundle)
        }
        Err(error) => Err(BuildError::Spec(error)),
    }
}

fn record_identity(domain: &[u8], fields: &[&[u8]]) -> (identity: Identity)
    requires
        identity_record_preimage(domain@, field_views(fields)).len() <= u64::MAX / 8,
    ensures
        identity.bytes_spec()
            == sha256::digest_spec(identity_record_preimage(domain@, field_views(fields))),
{
    let mut hasher = Sha256::new();
    proof {
        sha256::initial_view_is_valid();
        identity_record_prefix_is_bounded(domain@, field_views(fields), 0);
    }
    assert(hasher.view().1 == 0);
    assert(identity_field_preimage(domain@).len()
        <= identity_record_preimage(domain@, field_views(fields)).len());
    assert(sha256::can_update_view(
        hasher.view(), identity_field_preimage(domain@).len(),
    ));
    proof {
        hasher.derive_can_update(identity_field_preimage(domain@).len());
    }
    hash_field(&mut hasher, domain);
    let mut index = 0;
    while index < fields.len()
        invariant
            index <= fields.len(),
            hasher.valid(),
            sha256::valid_view(hasher.view()),
            sha256::valid_view(sha256::initial_view()),
            sha256::initial_view().1 == 0,
            hasher.view() == sha256::update_view(
                sha256::initial_view(),
                identity_field_preimage(domain@)
                    + identity_fields_preimage(field_views(fields), index as nat),
            ),
            identity_record_preimage(domain@, field_views(fields)).len() <= u64::MAX / 8,
        decreases fields.len() - index,
    {
        let field = fields[index];
        assert(field_views(fields)[index as int] == field@);
        proof {
            identity_fields_prefix_is_bounded(field_views(fields), (index + 1) as nat);
            identity_record_prefix_is_bounded(
                domain@,
                field_views(fields),
                (index + 1) as nat,
            );
        }
        assert(identity_fields_preimage(field_views(fields), (index + 1) as nat)
            == identity_fields_preimage(field_views(fields), index as nat)
                + identity_field_preimage(field@));
        proof {
            sha256::update_view_byte_len(
                sha256::initial_view(),
                identity_field_preimage(domain@)
                    + identity_fields_preimage(field_views(fields), index as nat),
            );
        }
        assert(hasher.view().1
            == (identity_field_preimage(domain@)
                + identity_fields_preimage(field_views(fields), index as nat)).len());
        assert((identity_field_preimage(domain@)
            + identity_fields_preimage(field_views(fields), (index + 1) as nat)).len()
                == hasher.view().1 + identity_field_preimage(field@).len());
        assert(sha256::can_update_view(
            hasher.view(), identity_field_preimage(field@).len(),
        ));
        proof {
            hasher.derive_can_update(identity_field_preimage(field@).len());
        }
        hash_field(&mut hasher, field);
        proof {
            sha256::update_view_concat(
                sha256::initial_view(),
                identity_field_preimage(domain@)
                    + identity_fields_preimage(field_views(fields), index as nat),
                identity_field_preimage(field@),
            );
            lemma_concat_associative(
                identity_field_preimage(domain@),
                identity_fields_preimage(field_views(fields), index as nat),
                identity_field_preimage(field@),
            );
        }
        index += 1;
    }
    assert(index == fields.len());
    assert(identity_record_preimage(domain@, field_views(fields))
        == identity_field_preimage(domain@)
            + identity_fields_preimage(field_views(fields), fields@.len()));
    assert(hasher.view() == sha256::update_view(
        sha256::initial_view(),
        identity_record_preimage(domain@, field_views(fields)),
    ));
    let digest = hasher.finish();
    proof {
        sha256::digest_spec_definition(
            identity_record_preimage(domain@, field_views(fields)),
        );
    }
    assert(digest@ == sha256::digest_spec(
        identity_record_preimage(domain@, field_views(fields)),
    ));
    Identity::new(digest)
}

} // verus!

verus! {

pub(crate) fn hash_field(hasher: &mut Sha256, bytes: &[u8])
    requires
        old(hasher).valid(),
        sha256::valid_view(old(hasher).view()),
        old(hasher).can_update(identity_field_preimage(bytes@).len()),
    ensures
        final(hasher).valid(),
        sha256::valid_view(final(hasher).view()),
        final(hasher).view() == sha256::update_view(
            old(hasher).view(),
            identity_field_preimage(bytes@),
        ),
{
    let length_value = u64::try_from(bytes.len()).expect("identity field length fits u64");
    let length = encode_u64_big_endian(length_value);
    let ghost initial_view = hasher.view();
    proof {
        identity_field_preimage_len(bytes@);
    }
    assert(length_value as nat == bytes@.len());
    assert(length@.len() == 8);
    assert(identity_field_preimage(bytes@).len() == length@.len() + bytes@.len());
    proof {
        hasher.expose_can_update(identity_field_preimage(bytes@).len());
    }
    assert(sha256::can_update_view(initial_view, length@.len()));
    proof {
        hasher.derive_can_update(length@.len());
    }
    hasher.update(&length);
    proof {
        sha256::update_view_byte_len(initial_view, length@);
    }
    assert(hasher.view().1 == initial_view.1 + length@.len());
    assert(sha256::can_update_view(hasher.view(), bytes@.len()));
    proof {
        hasher.derive_can_update(bytes@.len());
    }
    hasher.update(bytes);
    proof {
        sha256::update_view_concat(initial_view, length@, bytes@);
    }
    assert(length@ + bytes@ == identity_field_preimage(bytes@));
}

} // verus!

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

pub(crate) const fn decode_hex_32(hex: &[u8; 64]) -> [u8; 32] {
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

    fn replace_once(bytes: &[u8], from: &str, to: &str) -> Vec<u8> {
        let text = std::str::from_utf8(bytes).expect("fixture is UTF-8");
        let changed = text.replacen(from, to, 1);
        assert_ne!(changed, text, "fixture mutation must replace one field");
        changed.into_bytes()
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
        assert_eq!(
            digest_bytes(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq").sha256,
            super::decode_hex_32(
                b"248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
            )
        );
        let million_a = vec![b'a'; 1_000_000];
        assert_eq!(
            digest_bytes(&million_a).sha256,
            super::decode_hex_32(
                b"cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
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
    }

    #[test]
    fn sha256_chunk_boundaries_match_one_shot_digest() {
        for byte_len in [0, 1, 55, 56, 63, 64, 65, 119, 120, 127, 128, 129, 257] {
            let bytes = (0..byte_len)
                .map(|index| u8::try_from((index * 131 + 17) % 256).expect("fixture byte fits u8"))
                .collect::<Vec<_>>();
            let expected = super::sha256::digest(&bytes);
            for chunk_len in [1, 2, 7, 31, 63, 64, 65, 127] {
                let mut incremental = super::Sha256::new();
                for chunk in bytes.chunks(chunk_len) {
                    incremental.update(chunk);
                }
                assert_eq!(
                    incremental.finish(),
                    expected,
                    "byte_len={byte_len}, chunk_len={chunk_len}"
                );
            }
        }
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

        let draft_duplicate =
            add_root_field(pinned_config(DRAFT_CONFIG), r#""num_hidden_layers":28"#);
        let mut assets = canonical_assets();
        assets.draft.config_json = &draft_duplicate;
        assert!(matches!(
            build_preliminary_deployment_bundle(assets),
            Err(BuildError::InvalidJson { ref reason, .. }) if reason.contains("duplicate field")
        ));

        let tokenizer_unknown = add_root_field(
            pinned_config(TOKENIZER_METADATA),
            r#""future_tokenizer_field":1"#,
        );
        let mut assets = canonical_assets();
        assets.draft.tokenizer_metadata_json = &tokenizer_unknown;
        assert!(matches!(
            build_preliminary_deployment_bundle(assets),
            Err(BuildError::UnknownField { ref field, .. }) if field == "future_tokenizer_field"
        ));
    }

    #[test]
    fn missing_fields_are_rejected_for_both_roles_and_tokenizer_metadata() {
        for target_role in [true, false] {
            let config = if target_role {
                pinned_config(TARGET_CONFIG)
            } else {
                pinned_config(DRAFT_CONFIG)
            };
            let missing = replace_once(config, "  \"attention_bias\": false,\n", "");
            let mut assets = canonical_assets();
            if target_role {
                assets.target.config_json = &missing;
            } else {
                assets.draft.config_json = &missing;
            }
            assert!(matches!(
                build_preliminary_deployment_bundle(assets),
                Err(BuildError::MissingField { ref field, .. }) if field == "attention_bias"
            ));
        }

        let missing = replace_once(TOKENIZER_METADATA, "  \"add_bos_token\": false,\n", "");
        let mut assets = canonical_assets();
        assets.target.tokenizer_metadata_json = &missing;
        assert!(matches!(
            build_preliminary_deployment_bundle(assets),
            Err(BuildError::MissingField { ref field, .. }) if field == "add_bos_token"
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

        let target_geometry = replace_once(
            pinned_config(TARGET_CONFIG),
            "\"hidden_size\": 4096",
            "\"hidden_size\": 4097",
        );
        let mut assets = canonical_assets();
        assets.target.config_json = &target_geometry;
        assert_eq!(
            build_preliminary_deployment_bundle(assets),
            Err(BuildError::DigestMismatch("target config.json"))
        );

        let draft_geometry = replace_once(
            pinned_config(DRAFT_CONFIG),
            "\"hidden_size\": 1024",
            "\"hidden_size\": 1025",
        );
        let mut assets = canonical_assets();
        assets.draft.config_json = &draft_geometry;
        assert_eq!(
            build_preliminary_deployment_bundle(assets),
            Err(BuildError::DigestMismatch("draft config.json"))
        );

        let mut draft_reencoded = pinned_config(DRAFT_CONFIG).to_vec();
        draft_reencoded.push(b' ');
        let mut assets = canonical_assets();
        assets.draft.config_json = &draft_reencoded;
        assert_eq!(
            build_preliminary_deployment_bundle(assets),
            Err(BuildError::DigestMismatch("draft config.json"))
        );

        let mut tokenizer_reencoded = TOKENIZER_METADATA.to_vec();
        tokenizer_reencoded.push(b' ');
        let mut assets = canonical_assets();
        assets.target.tokenizer_metadata_json = &tokenizer_reencoded;
        assert_eq!(
            build_preliminary_deployment_bundle(assets),
            Err(BuildError::DigestMismatch("tokenizer_config.json"))
        );
    }

    #[test]
    fn source_and_opaque_descriptors_are_pinned() {
        for (target_role, repository) in [(true, DRAFT_REPOSITORY), (false, TARGET_REPOSITORY)] {
            let mut assets = canonical_assets();
            if target_role {
                assets.target.repository = repository;
            } else {
                assets.draft.repository = repository;
            }
            let expected_role = if target_role {
                ferric_spec::Qwen3ModelRole::Target8B
            } else {
                ferric_spec::Qwen3ModelRole::Draft06B
            };
            assert_eq!(
                build_preliminary_deployment_bundle(assets),
                Err(BuildError::SourceMismatch(expected_role))
            );
        }

        for (target_role, revision) in [(true, DRAFT_REVISION), (false, TARGET_REVISION)] {
            let mut assets = canonical_assets();
            if target_role {
                assets.target.revision = revision;
            } else {
                assets.draft.revision = revision;
            }
            let expected_role = if target_role {
                ferric_spec::Qwen3ModelRole::Target8B
            } else {
                ferric_spec::Qwen3ModelRole::Draft06B
            };
            assert_eq!(
                build_preliminary_deployment_bundle(assets),
                Err(BuildError::SourceMismatch(expected_role))
            );
        }

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
    fn every_weight_descriptor_field_is_role_pinned() {
        for target_role in [true, false] {
            for field in 0..4 {
                let mut assets = canonical_assets();
                let descriptor = if target_role {
                    &mut assets.target.weights
                } else {
                    &mut assets.draft.weights
                };
                match field {
                    0 => descriptor.weights_id[31] ^= 1,
                    1 => descriptor.artifact_bytes += 1,
                    2 => descriptor.tensor_data_bytes += 1,
                    3 => descriptor.sections += 1,
                    _ => unreachable!(),
                }
                let artifact = if target_role {
                    "target weight set"
                } else {
                    "draft weight file"
                };
                assert_eq!(
                    build_preliminary_deployment_bundle(assets),
                    Err(BuildError::DescriptorMismatch(artifact)),
                    "target_role={target_role}, field={field}"
                );
            }
        }
    }

    #[test]
    fn tokenizer_substitutions_fail_for_both_roles() {
        for target_role in [true, false] {
            for mutate_length in [false, true] {
                let mut assets = canonical_assets();
                let vocabulary = if target_role {
                    &mut assets.target.vocabulary
                } else {
                    &mut assets.draft.vocabulary
                };
                if mutate_length {
                    vocabulary.byte_len += 1;
                } else {
                    vocabulary.sha256[0] ^= 1;
                }
                assert_eq!(
                    build_preliminary_deployment_bundle(assets),
                    Err(BuildError::DescriptorMismatch("tokenizer.json")),
                    "target_role={target_role}, mutate_length={mutate_length}"
                );
            }

            let changed_metadata = replace_once(
                TOKENIZER_METADATA,
                "\"clean_up_tokenization_spaces\": false",
                "\"clean_up_tokenization_spaces\": true",
            );
            let mut assets = canonical_assets();
            if target_role {
                assets.target.tokenizer_metadata_json = &changed_metadata;
            } else {
                assets.draft.tokenizer_metadata_json = &changed_metadata;
            }
            assert!(matches!(
                build_preliminary_deployment_bundle(assets),
                Err(BuildError::UnexpectedValue { ref field, .. })
                    if field == "clean_up_tokenization_spaces"
            ));
        }
    }

    #[test]
    fn every_admitted_limit_changes_the_final_bundle_identity() {
        let canonical = build_preliminary_deployment_bundle(canonical_assets())
            .expect("canonical preliminary bundle");
        for field in 0..4 {
            let mut assets = canonical_assets();
            match field {
                0 => assets.limits.max_context_tokens -= 1,
                1 => assets.limits.max_active_sequences -= 1,
                2 => assets.limits.kv_page_tokens /= 2,
                3 => assets.limits.max_draft_tokens -= 1,
                _ => unreachable!(),
            }
            let changed = build_preliminary_deployment_bundle(assets)
                .expect("changed limits remain in the M1 envelope");
            assert_ne!(
                changed.bundle_id, canonical.bundle_id,
                "limit field {field} must enter the final identity"
            );
        }
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
