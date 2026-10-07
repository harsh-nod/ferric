use crate::Identity;
use core::fmt;
use vstd::prelude::*;

verus! {

pub const M1_MAX_CONTEXT_TOKENS: u32 = 8_192;
pub const M1_MAX_ACTIVE_SEQUENCES: u32 = 32;
pub const M1_MAX_KV_PAGE_TOKENS: u32 = 256;
pub const M1_MAX_DRAFT_TOKENS: u32 = 16;
pub const M1_MAX_WEIGHT_SECTIONS: u32 = 64;
pub const M1_MAX_WEIGHT_BYTES: u64 = 32 * 1_024 * 1_024 * 1_024;

pub const QWEN3_VOCABULARY_SIZE: u32 = 151_936;
pub const QWEN3_END_OF_TEXT_TOKEN: u32 = 151_643;
pub const QWEN3_IM_START_TOKEN: u32 = 151_644;
pub const QWEN3_IM_END_TOKEN: u32 = 151_645;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Gfx942XnackMinus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericalPolicy {
    Bf16ParametersFp32Accumulation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Qwen3ModelRole {
    Target8B,
    Draft06B,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EngineLimits {
    pub max_context_tokens: u32,
    pub max_active_sequences: u32,
    pub kv_page_tokens: u32,
    pub max_draft_tokens: u32,
}

impl EngineLimits {
    pub closed spec fn valid(self) -> bool {
        0 < self.max_context_tokens <= M1_MAX_CONTEXT_TOKENS
            && 0 < self.max_active_sequences <= M1_MAX_ACTIVE_SEQUENCES
            && 0 < self.kv_page_tokens <= M1_MAX_KV_PAGE_TOKENS
            && self.kv_page_tokens <= self.max_context_tokens
            && 0 < self.max_draft_tokens <= M1_MAX_DRAFT_TOKENS
            && self.max_draft_tokens <= self.max_context_tokens
    }

    /// Validates the bounded M1 runtime envelope.
    ///
    /// # Errors
    ///
    /// Returns [`SpecError`] when a limit is zero, exceeds the declared M1
    /// envelope, or cannot fit within the admitted context.
    pub fn validate(self) -> (result: Result<(), SpecError>)
        ensures result.is_ok() == self.valid(),
    {
        if self.max_context_tokens == 0 {
            return Err(SpecError::ZeroLimit("max_context_tokens"));
        }
        if self.max_context_tokens > M1_MAX_CONTEXT_TOKENS {
            return Err(SpecError::ExceedsM1Envelope("max_context_tokens"));
        }
        if self.max_active_sequences == 0 {
            return Err(SpecError::ZeroLimit("max_active_sequences"));
        }
        if self.max_active_sequences > M1_MAX_ACTIVE_SEQUENCES {
            return Err(SpecError::ExceedsM1Envelope("max_active_sequences"));
        }
        if self.kv_page_tokens == 0 {
            return Err(SpecError::ZeroLimit("kv_page_tokens"));
        }
        if self.kv_page_tokens > M1_MAX_KV_PAGE_TOKENS {
            return Err(SpecError::ExceedsM1Envelope("kv_page_tokens"));
        }
        if self.kv_page_tokens > self.max_context_tokens {
            return Err(SpecError::PageExceedsContext);
        }
        if self.max_draft_tokens == 0 {
            return Err(SpecError::ZeroLimit("max_draft_tokens"));
        }
        if self.max_draft_tokens > M1_MAX_DRAFT_TOKENS {
            return Err(SpecError::ExceedsM1Envelope("max_draft_tokens"));
        }
        if self.max_draft_tokens > self.max_context_tokens {
            return Err(SpecError::DraftExceedsContext);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelConfig {
    pub role: Qwen3ModelRole,
    pub model_id: Identity,
    pub config_id: Identity,
    pub vocabulary_size: u32,
    pub layers: u32,
    pub hidden_size: u32,
    pub intermediate_size: u32,
    pub query_heads: u32,
    pub kv_heads: u32,
    pub head_dim: u32,
    pub max_position_embeddings: u32,
    pub rope_theta: u32,
    pub tie_word_embeddings: bool,
}

impl ModelConfig {
    pub closed spec fn shape_matches_role(self) -> bool {
        match self.role {
            Qwen3ModelRole::Target8B => {
                self.vocabulary_size == QWEN3_VOCABULARY_SIZE
                    && self.layers == 36
                    && self.hidden_size == 4_096
                    && self.intermediate_size == 12_288
                    && self.query_heads == 32
                    && self.kv_heads == 8
                    && self.head_dim == 128
                    && self.max_position_embeddings == 40_960
                    && self.rope_theta == 1_000_000
                    && !self.tie_word_embeddings
            }
            Qwen3ModelRole::Draft06B => {
                self.vocabulary_size == QWEN3_VOCABULARY_SIZE
                    && self.layers == 28
                    && self.hidden_size == 1_024
                    && self.intermediate_size == 3_072
                    && self.query_heads == 16
                    && self.kv_heads == 8
                    && self.head_dim == 128
                    && self.max_position_embeddings == 40_960
                    && self.rope_theta == 1_000_000
                    && self.tie_word_embeddings
            }
        }
    }

    pub closed spec fn valid(self) -> bool {
        (exists|index: int|
            0 <= index < self.model_id.bytes_spec().len()
                && self.model_id.bytes_spec()[index] != 0)
            && (exists|index: int|
                0 <= index < self.config_id.bytes_spec().len()
                    && self.config_id.bytes_spec()[index] != 0)
            && self.shape_matches_role()
    }

    /// Validates exact Qwen3 target or draft geometry and identities.
    ///
    /// # Errors
    ///
    /// Returns [`SpecError`] when an identity is absent or any model field
    /// differs from the pinned Qwen3 role.
    pub fn validate(self) -> (result: Result<(), SpecError>)
        ensures result.is_ok() == self.valid(),
    {
        if !self.model_id.is_present() {
            return Err(SpecError::MissingIdentity("model_id"));
        }
        if !self.config_id.is_present() {
            return Err(SpecError::MissingIdentity("config_id"));
        }
        match self.role {
            Qwen3ModelRole::Target8B => {
                if self.vocabulary_size != QWEN3_VOCABULARY_SIZE
                    || self.layers != 36
                    || self.hidden_size != 4_096
                    || self.intermediate_size != 12_288
                    || self.query_heads != 32
                    || self.kv_heads != 8
                    || self.head_dim != 128
                    || self.max_position_embeddings != 40_960
                    || self.rope_theta != 1_000_000
                    || self.tie_word_embeddings
                {
                    return Err(SpecError::UnexpectedModelShape(Qwen3ModelRole::Target8B));
                }
            }
            Qwen3ModelRole::Draft06B => {
                if self.vocabulary_size != QWEN3_VOCABULARY_SIZE
                    || self.layers != 28
                    || self.hidden_size != 1_024
                    || self.intermediate_size != 3_072
                    || self.query_heads != 16
                    || self.kv_heads != 8
                    || self.head_dim != 128
                    || self.max_position_embeddings != 40_960
                    || self.rope_theta != 1_000_000
                    || !self.tie_word_embeddings
                {
                    return Err(SpecError::UnexpectedModelShape(Qwen3ModelRole::Draft06B));
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TokenizerConfig {
    pub tokenizer_id: Identity,
    pub vocabulary_id: Identity,
    pub vocabulary_size: u32,
    pub end_of_text_token: u32,
    pub im_start_token: u32,
    pub im_end_token: u32,
}

impl TokenizerConfig {
    pub closed spec fn valid(self) -> bool {
        (exists|index: int|
            0 <= index < self.tokenizer_id.bytes_spec().len()
                && self.tokenizer_id.bytes_spec()[index] != 0)
            && (exists|index: int|
                0 <= index < self.vocabulary_id.bytes_spec().len()
                    && self.vocabulary_id.bytes_spec()[index] != 0)
            && self.vocabulary_size == QWEN3_VOCABULARY_SIZE
            && self.end_of_text_token == QWEN3_END_OF_TEXT_TOKEN
            && self.im_start_token == QWEN3_IM_START_TOKEN
            && self.im_end_token == QWEN3_IM_END_TOKEN
    }

    pub closed spec fn compatible(self, other: Self) -> bool {
        self.tokenizer_id.bytes_spec() == other.tokenizer_id.bytes_spec()
            && self.vocabulary_id.bytes_spec() == other.vocabulary_id.bytes_spec()
            && self.vocabulary_size == other.vocabulary_size
            && self.end_of_text_token == other.end_of_text_token
            && self.im_start_token == other.im_start_token
            && self.im_end_token == other.im_end_token
    }

    /// Validates the pinned Qwen3 tokenizer identities and token numbers.
    ///
    /// # Errors
    ///
    /// Returns [`SpecError`] when an identity is absent or the tokenizer is
    /// outside the pinned Qwen3 vocabulary contract.
    pub fn validate(self) -> (result: Result<(), SpecError>)
        ensures result.is_ok() == self.valid(),
    {
        if !self.tokenizer_id.is_present() {
            return Err(SpecError::MissingIdentity("tokenizer_id"));
        }
        if !self.vocabulary_id.is_present() {
            return Err(SpecError::MissingIdentity("vocabulary_id"));
        }
        if self.vocabulary_size != QWEN3_VOCABULARY_SIZE
            || self.end_of_text_token != QWEN3_END_OF_TEXT_TOKEN
            || self.im_start_token != QWEN3_IM_START_TOKEN
            || self.im_end_token != QWEN3_IM_END_TOKEN
        {
            return Err(SpecError::UnexpectedTokenizer);
        }
        Ok(())
    }

    #[must_use]
    pub fn is_compatible_with(self, other: Self) -> (compatible: bool)
        ensures compatible == self.compatible(other),
    {
        if !self.tokenizer_id.equals(&other.tokenizer_id) {
            return false;
        }
        if !self.vocabulary_id.equals(&other.vocabulary_id) {
            return false;
        }
        self.vocabulary_size == other.vocabulary_size
            && self.end_of_text_token == other.end_of_text_token
            && self.im_start_token == other.im_start_token
            && self.im_end_token == other.im_end_token
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WeightManifest {
    pub weights_id: Identity,
    pub total_bytes: u64,
    pub sections: u32,
}

impl WeightManifest {
    pub closed spec fn valid(self) -> bool {
        (exists|index: int|
            0 <= index < self.weights_id.bytes_spec().len()
                && self.weights_id.bytes_spec()[index] != 0)
            && 0 < self.total_bytes <= M1_MAX_WEIGHT_BYTES
            && 0 < self.sections <= M1_MAX_WEIGHT_SECTIONS
    }

    /// Validates the bounded aggregate weight identity.
    ///
    /// # Errors
    ///
    /// Returns [`SpecError`] when the identity is absent or the declared
    /// storage exceeds the M1 bundle envelope.
    pub fn validate(self) -> (result: Result<(), SpecError>)
        ensures result.is_ok() == self.valid(),
    {
        if !self.weights_id.is_present() {
            return Err(SpecError::MissingIdentity("weights_id"));
        }
        if self.total_bytes == 0 {
            return Err(SpecError::ZeroLimit("weight_bytes"));
        }
        if self.total_bytes > M1_MAX_WEIGHT_BYTES {
            return Err(SpecError::ExceedsM1Envelope("weight_bytes"));
        }
        if self.sections == 0 {
            return Err(SpecError::ZeroLimit("weight_sections"));
        }
        if self.sections > M1_MAX_WEIGHT_SECTIONS {
            return Err(SpecError::ExceedsM1Envelope("weight_sections"));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelArtifact {
    pub config: ModelConfig,
    pub tokenizer: TokenizerConfig,
    pub weights: WeightManifest,
}

impl ModelArtifact {
    pub closed spec fn valid_for_role(self, role: Qwen3ModelRole) -> bool {
        self.config.role == role
            && self.config.valid()
            && self.tokenizer.valid()
            && self.weights.valid()
    }

    /// Validates an artifact against its required target or draft role.
    ///
    /// # Errors
    ///
    /// Returns [`SpecError`] when the role, model configuration, tokenizer,
    /// or weight manifest is not admitted by the M1 bundle contract.
    pub fn validate_for_role(self, role: Qwen3ModelRole) -> (result: Result<(), SpecError>)
        ensures result.is_ok() == self.valid_for_role(role),
    {
        match (role, self.config.role) {
            (Qwen3ModelRole::Target8B, Qwen3ModelRole::Target8B)
            | (Qwen3ModelRole::Draft06B, Qwen3ModelRole::Draft06B) => {}
            _ => return Err(SpecError::UnexpectedModelRole),
        }
        self.config.validate()?;
        self.tokenizer.validate()?;
        self.weights.validate()?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeploymentBundle {
    pub bundle_id: Identity,
    pub target: Target,
    pub numerical_policy: NumericalPolicy,
    pub limits: EngineLimits,
    pub target_model: ModelArtifact,
    pub draft_model: ModelArtifact,
}

impl DeploymentBundle {
    pub closed spec fn valid(self) -> bool {
        (exists|index: int|
            0 <= index < self.bundle_id.bytes_spec().len()
                && self.bundle_id.bytes_spec()[index] != 0)
            && self.limits.valid()
            && self.target_model.valid_for_role(Qwen3ModelRole::Target8B)
            && self.draft_model.valid_for_role(Qwen3ModelRole::Draft06B)
            && self.target_model.tokenizer.compatible(self.draft_model.tokenizer)
    }

    /// Validates the first bounded Qwen3 target/draft deployment bundle.
    ///
    /// # Errors
    ///
    /// Returns [`SpecError`] when the bundle identity, M1 limits, exact model
    /// roles, weight bounds, or target/draft tokenizer agreement is invalid.
    pub fn validate(self) -> (result: Result<(), SpecError>)
        ensures result.is_ok() == self.valid(),
    {
        if !self.bundle_id.is_present() {
            return Err(SpecError::MissingIdentity("bundle_id"));
        }
        self.limits.validate()?;
        self.target_model
            .validate_for_role(Qwen3ModelRole::Target8B)?;
        self.draft_model
            .validate_for_role(Qwen3ModelRole::Draft06B)?;
        if !self
            .target_model
            .tokenizer
            .is_compatible_with(self.draft_model.tokenizer)
        {
            return Err(SpecError::TokenizerMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpecError {
    ZeroLimit(&'static str),
    MissingIdentity(&'static str),
    ExceedsM1Envelope(&'static str),
    PageExceedsContext,
    DraftExceedsContext,
    UnexpectedModelShape(Qwen3ModelRole),
    UnexpectedModelRole,
    UnexpectedTokenizer,
    TokenizerMismatch,
}

} // verus!

impl fmt::Display for SpecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroLimit(name) => write!(formatter, "{name} must be nonzero"),
            Self::MissingIdentity(name) => write!(formatter, "{name} must be present"),
            Self::ExceedsM1Envelope(name) => {
                write!(formatter, "{name} exceeds the M1 deployment envelope")
            }
            Self::PageExceedsContext => {
                formatter.write_str("KV page size exceeds the maximum context")
            }
            Self::DraftExceedsContext => {
                formatter.write_str("draft length exceeds the maximum context")
            }
            Self::UnexpectedModelShape(role) => {
                write!(formatter, "model configuration does not match {role:?}")
            }
            Self::UnexpectedModelRole => {
                formatter.write_str("target and draft model roles are not canonical")
            }
            Self::UnexpectedTokenizer => {
                formatter.write_str("tokenizer is outside the pinned Qwen3 contract")
            }
            Self::TokenizerMismatch => {
                formatter.write_str("target and draft tokenizers are incompatible")
            }
        }
    }
}

impl std::error::Error for SpecError {}

#[cfg(test)]
mod tests {
    use super::{
        DeploymentBundle, EngineLimits, ModelArtifact, ModelConfig, NumericalPolicy,
        Qwen3ModelRole, SpecError, Target, TokenizerConfig, WeightManifest,
        QWEN3_END_OF_TEXT_TOKEN, QWEN3_IM_END_TOKEN, QWEN3_IM_START_TOKEN, QWEN3_VOCABULARY_SIZE,
    };
    use crate::Identity;

    const fn identity(byte: u8) -> Identity {
        Identity::new([byte; 32])
    }

    fn tokenizer() -> TokenizerConfig {
        TokenizerConfig {
            tokenizer_id: identity(3),
            vocabulary_id: identity(4),
            vocabulary_size: QWEN3_VOCABULARY_SIZE,
            end_of_text_token: QWEN3_END_OF_TEXT_TOKEN,
            im_start_token: QWEN3_IM_START_TOKEN,
            im_end_token: QWEN3_IM_END_TOKEN,
        }
    }

    fn target_config() -> ModelConfig {
        ModelConfig {
            role: Qwen3ModelRole::Target8B,
            model_id: identity(1),
            config_id: identity(2),
            vocabulary_size: QWEN3_VOCABULARY_SIZE,
            layers: 36,
            hidden_size: 4_096,
            intermediate_size: 12_288,
            query_heads: 32,
            kv_heads: 8,
            head_dim: 128,
            max_position_embeddings: 40_960,
            rope_theta: 1_000_000,
            tie_word_embeddings: false,
        }
    }

    fn draft_config() -> ModelConfig {
        ModelConfig {
            role: Qwen3ModelRole::Draft06B,
            model_id: identity(5),
            config_id: identity(6),
            vocabulary_size: QWEN3_VOCABULARY_SIZE,
            layers: 28,
            hidden_size: 1_024,
            intermediate_size: 3_072,
            query_heads: 16,
            kv_heads: 8,
            head_dim: 128,
            max_position_embeddings: 40_960,
            rope_theta: 1_000_000,
            tie_word_embeddings: true,
        }
    }

    fn bundle() -> DeploymentBundle {
        DeploymentBundle {
            bundle_id: identity(9),
            target: Target::Gfx942XnackMinus,
            numerical_policy: NumericalPolicy::Bf16ParametersFp32Accumulation,
            limits: EngineLimits {
                max_context_tokens: 8_192,
                max_active_sequences: 32,
                kv_page_tokens: 16,
                max_draft_tokens: 8,
            },
            target_model: ModelArtifact {
                config: target_config(),
                tokenizer: tokenizer(),
                weights: WeightManifest {
                    weights_id: identity(7),
                    total_bytes: 16_000_000_000,
                    sections: 5,
                },
            },
            draft_model: ModelArtifact {
                config: draft_config(),
                tokenizer: tokenizer(),
                weights: WeightManifest {
                    weights_id: identity(8),
                    total_bytes: 1_200_000_000,
                    sections: 2,
                },
            },
        }
    }

    #[test]
    fn canonical_qwen3_bundle_is_accepted() {
        assert_eq!(bundle().validate(), Ok(()));
    }

    #[test]
    fn model_shape_drift_is_rejected() {
        let mut candidate = bundle();
        candidate.target_model.config.layers -= 1;
        assert_eq!(
            candidate.validate(),
            Err(SpecError::UnexpectedModelShape(Qwen3ModelRole::Target8B))
        );
    }

    #[test]
    fn tokenizer_identity_mismatch_is_rejected() {
        let mut candidate = bundle();
        candidate.draft_model.tokenizer.tokenizer_id = identity(10);
        assert_eq!(candidate.validate(), Err(SpecError::TokenizerMismatch));
    }

    #[test]
    fn target_and_draft_roles_cannot_be_swapped() {
        let mut candidate = bundle();
        candidate.target_model.config = draft_config();
        candidate.draft_model.config = target_config();
        assert_eq!(candidate.validate(), Err(SpecError::UnexpectedModelRole));
    }

    #[test]
    fn bundle_limits_are_fail_closed() {
        let mut candidate = bundle();
        candidate.limits.max_active_sequences = 33;
        assert_eq!(
            candidate.validate(),
            Err(SpecError::ExceedsM1Envelope("max_active_sequences"))
        );
    }

    #[test]
    fn zero_or_oversized_weight_manifests_are_rejected() {
        let mut zero = bundle();
        zero.draft_model.weights.total_bytes = 0;
        assert_eq!(zero.validate(), Err(SpecError::ZeroLimit("weight_bytes")));

        let mut oversized = bundle();
        oversized.target_model.weights.sections = 65;
        assert_eq!(
            oversized.validate(),
            Err(SpecError::ExceedsM1Envelope("weight_sections"))
        );
    }
}
