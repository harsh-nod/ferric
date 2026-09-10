//! Exact canonical Qwen model intake for non-authoritative TP engineering runs.
//!
//! The retained M1 model bundle is used only for immutable weight geometry and
//! identities. Its target label is unchanged; it is not an MI350 runner or
//! device capability. The selected runtime must independently admit its target.

use ferric_build::{
    AuthenticatedDeploymentAssets, AuthenticatedModelAssets, AuthenticatedModelWeightLayout,
    AuthenticatedTokenizer, DRAFT_REPOSITORY, DRAFT_REVISION, SpecialTokenDecodePolicy,
    SpecialTokenEncodePolicy, TARGET_REPOSITORY, TARGET_REVISION, TokenizerExecutionLimits,
    authenticate_qwen3_tokenizer, build_authenticated_model_weight_layout,
    build_prepacked_deployment_bundle, open_canonical_qwen3_source_bundle,
    prepack_qwen3_draft_weights, prepack_qwen3_target_weights, seal_authenticated_bundle,
};
use ferric_spec::{
    EngineLimits, Identity, ModelConfig, QWEN3_TARGET_TENSOR_DATA_BYTES, Qwen3ModelRole,
};
use std::io::{self, Cursor, Write};
use std::path::Path;

/// Immutable target bytes and their authenticated model layout and tokenizer.
///
/// This owner grants no device allocation, dispatch, or serving authority.
pub struct EngineeringQwenModelV1 {
    target_weights: Box<[u8]>,
    layout: AuthenticatedModelWeightLayout,
    tokenizer: AuthenticatedTokenizer,
}

impl EngineeringQwenModelV1 {
    /// Reopens and authenticates the exact canonical target/draft source tree.
    ///
    /// Target payloads are retained in memory. The draft stream is authenticated
    /// for the unchanged canonical bundle but is not retained or executed.
    ///
    /// # Errors
    ///
    /// Rejects filesystem/schema/content drift, allocation failures, incomplete
    /// prepacking, incompatible tokenizers, or failed model-layout admission.
    pub fn open(source: &Path) -> Result<Self, String> {
        let source = open_canonical_qwen3_source_bundle(source)
            .map_err(|error| format!("cannot reopen canonical Qwen source: {error}"))?;
        let target_tokenizer =
            authenticate_qwen3_tokenizer(Qwen3ModelRole::Target8B, Cursor::new(&source.tokenizer))
                .map_err(|error| format!("cannot authenticate target tokenizer: {error}"))?;
        let draft_tokenizer =
            authenticate_qwen3_tokenizer(Qwen3ModelRole::Draft06B, Cursor::new(&source.tokenizer))
                .map_err(|error| format!("cannot authenticate draft tokenizer: {error}"))?;
        let mut target_output = ExactPrepackBuffer::new(QWEN3_TARGET_TENSOR_DATA_BYTES)
            .map_err(|error| format!("cannot reserve target model bytes: {error}"))?;
        let target = prepack_qwen3_target_weights(
            &source.target_index,
            source.target_shards,
            &mut target_output,
        )
        .map_err(|error| format!("cannot authenticate target weights: {error}"))?;
        let target_weights = target_output
            .finish()
            .map_err(|error| format!("target model payload is incomplete: {error}"))?;
        let draft = prepack_qwen3_draft_weights(source.draft_weights, &mut io::sink())
            .map_err(|error| format!("cannot authenticate draft weights: {error}"))?;
        let prepacked = build_prepacked_deployment_bundle(
            AuthenticatedDeploymentAssets {
                target: AuthenticatedModelAssets {
                    repository: TARGET_REPOSITORY,
                    revision: TARGET_REVISION,
                    config_json: &source.target_config,
                    tokenizer_metadata_json: &source.target_tokenizer_metadata,
                },
                draft: AuthenticatedModelAssets {
                    repository: DRAFT_REPOSITORY,
                    revision: DRAFT_REVISION,
                    config_json: &source.draft_config,
                    tokenizer_metadata_json: &source.draft_tokenizer_metadata,
                },
                limits: EngineLimits {
                    max_context_tokens: 8_192,
                    max_active_sequences: 32,
                    kv_page_tokens: 256,
                    max_draft_tokens: 16,
                },
            },
            target_tokenizer,
            draft_tokenizer,
            target,
            draft,
        )
        .map_err(|error| format!("cannot reconstruct canonical model bundle: {error}"))?;
        let admission = seal_authenticated_bundle(prepacked)
            .map_err(|error| format!("cannot seal canonical model bundle: {error}"))?;
        let layout = build_authenticated_model_weight_layout(admission)
            .map_err(|error| format!("cannot index authenticated target weights: {error}"))?;
        let tokenizer =
            authenticate_qwen3_tokenizer(Qwen3ModelRole::Target8B, Cursor::new(&source.tokenizer))
                .map_err(|error| format!("cannot retain target tokenizer: {error}"))?;
        Ok(Self {
            target_weights,
            layout,
            tokenizer,
        })
    }

    /// Borrows the authenticated header-free, row-major BF16 target payload.
    #[must_use]
    pub fn target_weights(&self) -> &[u8] {
        &self.target_weights
    }

    /// Borrows the typed manifest retained with these exact target bytes.
    #[must_use]
    pub const fn layout(&self) -> &AuthenticatedModelWeightLayout {
        &self.layout
    }

    /// Returns the canonical model geometry, not a GPU target declaration.
    #[must_use]
    pub fn config(&self) -> ModelConfig {
        self.layout
            .admission()
            .prepacked()
            .deployment()
            .target_model
            .config
    }

    /// Returns the unchanged canonical target/draft model bundle identity.
    #[must_use]
    pub fn bundle_id(&self) -> Identity {
        self.layout.admission().prepacked().deployment().bundle_id
    }

    /// Tokenizes a raw prompt without adding a chat template or special tokens.
    ///
    /// # Errors
    ///
    /// Rejects unsupported special tokens or tokenizer execution limits.
    pub fn encode(&self, prompt: &str) -> Result<Vec<u32>, String> {
        self.tokenizer
            .encode(
                prompt,
                TokenizerExecutionLimits::m1(),
                SpecialTokenEncodePolicy::Reject,
            )
            .map_err(|error| format!("cannot tokenize TP prompt: {error}"))
    }

    /// Decodes observed output tokens using the same authenticated tokenizer.
    ///
    /// # Errors
    ///
    /// Rejects invalid tokens or tokenizer execution limits.
    pub fn decode(&self, tokens: &[u32]) -> Result<Vec<u8>, String> {
        self.tokenizer
            .decode_to_bytes(
                tokens,
                TokenizerExecutionLimits::m1(),
                SpecialTokenDecodePolicy::Skip,
            )
            .map_err(|error| format!("cannot decode TP output: {error}"))
    }
}

struct ExactPrepackBuffer {
    bytes: Vec<u8>,
    expected: usize,
}

impl ExactPrepackBuffer {
    fn new(expected: u64) -> io::Result<Self> {
        let expected = usize::try_from(expected)
            .map_err(|_| io::Error::other("model byte length exceeds host address space"))?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(expected)
            .map_err(io::Error::other)?;
        Ok(Self { bytes, expected })
    }

    fn finish(self) -> io::Result<Box<[u8]>> {
        if self.bytes.len() != self.expected {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "short model prepack",
            ));
        }
        Ok(self.bytes.into_boxed_slice())
    }
}

impl Write for ExactPrepackBuffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.expected - self.bytes.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "oversized model prepack",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::ExactPrepackBuffer;
    use std::io::{ErrorKind, Write};

    #[test]
    fn bounded_prepack_preserves_chunk_order_and_bytes() {
        let mut output = ExactPrepackBuffer::new(6).unwrap();
        output.write_all(&[0, 1]).unwrap();
        output.write_all(&[]).unwrap();
        output.write_all(&[2, 3, 254, 255]).unwrap();
        assert_eq!(&*output.finish().unwrap(), &[0, 1, 2, 3, 254, 255]);
    }

    #[test]
    fn oversized_write_does_not_change_previous_bytes() {
        let mut output = ExactPrepackBuffer::new(4).unwrap();
        output.write_all(&[1, 2]).unwrap();
        assert_eq!(
            output.write(&[3, 4, 5]).unwrap_err().kind(),
            ErrorKind::InvalidData
        );
        assert_eq!(output.bytes, [1, 2]);
        output.write_all(&[3, 4]).unwrap();
        assert_eq!(&*output.finish().unwrap(), &[1, 2, 3, 4]);
    }

    #[test]
    fn short_prepack_cannot_be_published() {
        let mut output = ExactPrepackBuffer::new(4).unwrap();
        output.write_all(&[1, 2]).unwrap();
        assert_eq!(
            output.finish().unwrap_err().kind(),
            ErrorKind::UnexpectedEof
        );
    }

    #[test]
    fn exhausted_buffer_only_accepts_empty_writes() {
        let mut output = ExactPrepackBuffer::new(1).unwrap();
        output.write_all(&[42]).unwrap();
        assert_eq!(output.write(&[]).unwrap(), 0);
        assert_eq!(
            output.write(&[9]).unwrap_err().kind(),
            ErrorKind::InvalidData
        );
        assert_eq!(&*output.finish().unwrap(), &[42]);
    }

    #[test]
    fn impossible_reservation_is_fallible() {
        assert!(ExactPrepackBuffer::new(u64::MAX).is_err());
    }
}
