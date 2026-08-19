use crate::Identity;
use core::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Gfx942XnackMinus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EngineLimits {
    pub max_context_tokens: u32,
    pub max_active_sequences: u32,
    pub kv_page_tokens: u32,
    pub max_draft_tokens: u32,
}

impl EngineLimits {
    /// Validates that every limit is usable and internally consistent.
    ///
    /// # Errors
    ///
    /// Returns [`SpecError`] when a limit is zero or a KV page is larger than
    /// the admitted context.
    pub fn validate(self) -> Result<(), SpecError> {
        if self.max_context_tokens == 0 {
            return Err(SpecError::ZeroLimit("max_context_tokens"));
        }
        if self.max_active_sequences == 0 {
            return Err(SpecError::ZeroLimit("max_active_sequences"));
        }
        if self.kv_page_tokens == 0 {
            return Err(SpecError::ZeroLimit("kv_page_tokens"));
        }
        if self.max_draft_tokens == 0 {
            return Err(SpecError::ZeroLimit("max_draft_tokens"));
        }
        if self.kv_page_tokens > self.max_context_tokens {
            return Err(SpecError::PageExceedsContext);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelConfig {
    pub model_id: Identity,
    pub tokenizer_id: Identity,
    pub target: Target,
    pub vocabulary_size: u32,
    pub layers: u32,
    pub hidden_size: u32,
    pub query_heads: u32,
    pub kv_heads: u32,
    pub head_dim: u32,
}

impl ModelConfig {
    /// Validates the model identities and grouped-query-attention dimensions.
    ///
    /// # Errors
    ///
    /// Returns [`SpecError`] when an identity or dimension is absent or the
    /// head geometry does not produce the declared hidden size.
    pub fn validate(self) -> Result<(), SpecError> {
        if !self.model_id.is_present() {
            return Err(SpecError::MissingIdentity("model_id"));
        }
        if !self.tokenizer_id.is_present() {
            return Err(SpecError::MissingIdentity("tokenizer_id"));
        }
        for (name, value) in [
            ("vocabulary_size", self.vocabulary_size),
            ("layers", self.layers),
            ("hidden_size", self.hidden_size),
            ("query_heads", self.query_heads),
            ("kv_heads", self.kv_heads),
            ("head_dim", self.head_dim),
        ] {
            if value == 0 {
                return Err(SpecError::ZeroLimit(name));
            }
        }
        if !self.query_heads.is_multiple_of(self.kv_heads) {
            return Err(SpecError::IncompatibleAttentionHeads);
        }
        if self.query_heads.checked_mul(self.head_dim) != Some(self.hidden_size) {
            return Err(SpecError::IncompatibleHiddenSize);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpecError {
    ZeroLimit(&'static str),
    MissingIdentity(&'static str),
    PageExceedsContext,
    IncompatibleAttentionHeads,
    IncompatibleHiddenSize,
}

impl fmt::Display for SpecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroLimit(name) => write!(formatter, "{name} must be nonzero"),
            Self::MissingIdentity(name) => write!(formatter, "{name} must be present"),
            Self::PageExceedsContext => {
                formatter.write_str("KV page size exceeds the maximum context")
            }
            Self::IncompatibleAttentionHeads => {
                formatter.write_str("query heads must be divisible by KV heads")
            }
            Self::IncompatibleHiddenSize => {
                formatter.write_str("hidden size must equal query head count times head dimension")
            }
        }
    }
}

impl std::error::Error for SpecError {}

#[cfg(test)]
mod tests {
    use super::{EngineLimits, ModelConfig, SpecError, Target};
    use crate::Identity;

    #[test]
    fn qwen_shaped_configuration_is_accepted() {
        let config = ModelConfig {
            model_id: Identity::new([1; 32]),
            tokenizer_id: Identity::new([2; 32]),
            target: Target::Gfx942XnackMinus,
            vocabulary_size: 151_936,
            layers: 36,
            hidden_size: 4_096,
            query_heads: 32,
            kv_heads: 8,
            head_dim: 128,
        };
        assert_eq!(config.validate(), Ok(()));
    }

    #[test]
    fn incompatible_grouped_query_attention_is_rejected() {
        let config = ModelConfig {
            model_id: Identity::new([1; 32]),
            tokenizer_id: Identity::new([2; 32]),
            target: Target::Gfx942XnackMinus,
            vocabulary_size: 100,
            layers: 1,
            hidden_size: 1_920,
            query_heads: 15,
            kv_heads: 8,
            head_dim: 128,
        };
        assert_eq!(
            config.validate(),
            Err(SpecError::IncompatibleAttentionHeads)
        );
    }

    #[test]
    fn invalid_limits_are_rejected() {
        let limits = EngineLimits {
            max_context_tokens: 8,
            max_active_sequences: 1,
            kv_page_tokens: 16,
            max_draft_tokens: 1,
        };
        assert_eq!(limits.validate(), Err(SpecError::PageExceedsContext));
    }
}
