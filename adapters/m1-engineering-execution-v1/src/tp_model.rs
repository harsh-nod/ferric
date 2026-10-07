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
    EngineLimits, Identity, ModelConfig, QWEN3_DRAFT_TENSOR_DATA_BYTES,
    QWEN3_TARGET_TENSOR_DATA_BYTES, Qwen3ModelRole,
};
use std::io::{self, Cursor, Write};
use std::path::Path;

/// Immutable target bytes, optional draft bytes, and their authenticated layout.
///
/// This owner grants no device allocation, dispatch, or serving authority.
pub struct EngineeringQwenModelV1 {
    target_weights: Box<[u8]>,
    draft_weights: Option<Box<[u8]>>,
    layout: AuthenticatedModelWeightLayout,
    tokenizer: AuthenticatedTokenizer,
}

/// A borrowed canonical draft payload bound to its owner's authenticated layout.
///
/// This view does not copy weights or confer device, execution, or qualification
/// authority. Only successful opt-in intake can produce it.
pub struct EngineeringQwenDraftModelV1<'a> {
    weights: &'a [u8],
    layout: &'a AuthenticatedModelWeightLayout,
}

impl EngineeringQwenDraftModelV1<'_> {
    /// Borrows the authenticated header-free, row-major BF16 draft payload.
    #[must_use]
    pub const fn weights(&self) -> &[u8] {
        self.weights
    }

    /// Borrows the same target/draft layout retained by the model owner.
    #[must_use]
    pub const fn layout(&self) -> &AuthenticatedModelWeightLayout {
        self.layout
    }

    /// Returns the canonical `Draft06B` geometry from that retained layout.
    #[must_use]
    pub fn config(&self) -> ModelConfig {
        self.layout
            .admission()
            .prepacked()
            .deployment()
            .draft_model
            .config
    }
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
        Self::open_with_retention(source, false)
    }

    /// Authenticates the same canonical bundle and also retains draft payloads.
    ///
    /// This adds exactly `QWEN3_DRAFT_TENSOR_DATA_BYTES` of live payload to the
    /// target-only path. The target bytes, shared layout, bundle identity and
    /// tokenizer are unchanged. Neither model is allocated on a device here.
    ///
    /// # Errors
    ///
    /// Rejects the same source and authentication failures as [`Self::open`],
    /// plus draft allocation failure or incomplete draft output. No partially
    /// authenticated model or draft view is returned on failure.
    pub fn open_with_draft(source: &Path) -> Result<Self, String> {
        Self::open_with_retention(source, true)
    }

    fn open_with_retention(source: &Path, retain_draft: bool) -> Result<Self, String> {
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
        let mut draft_output = DraftPrepackOutput::new(retain_draft, QWEN3_DRAFT_TENSOR_DATA_BYTES)
            .map_err(|error| format!("cannot reserve draft model bytes: {error}"))?;
        let draft = prepack_qwen3_draft_weights(source.draft_weights, &mut draft_output)
            .map_err(|error| format!("cannot authenticate draft weights: {error}"))?;
        let draft_weights = draft_output
            .finish()
            .map_err(|error| format!("draft model payload is incomplete: {error}"))?;
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
            draft_weights,
            layout,
            tokenizer,
        })
    }

    /// Borrows the authenticated header-free, row-major BF16 target payload.
    #[must_use]
    pub fn target_weights(&self) -> &[u8] {
        &self.target_weights
    }

    /// Borrows the draft only when intake explicitly retained its payload.
    ///
    /// [`Self::open`] returns no draft view and allocates no draft payload buffer.
    /// [`Self::open_with_draft`] binds the view to the same canonical layout and
    /// tokenizer as the target, without copying either model's bytes.
    #[must_use]
    pub fn draft(&self) -> Option<EngineeringQwenDraftModelV1<'_>> {
        self.draft_weights
            .as_deref()
            .map(|weights| EngineeringQwenDraftModelV1 {
                weights,
                layout: &self.layout,
            })
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

struct DraftPrepackOutput {
    retained: Option<ExactPrepackBuffer>,
}

impl DraftPrepackOutput {
    fn new(retain: bool, expected: u64) -> io::Result<Self> {
        Ok(Self {
            retained: if retain {
                Some(ExactPrepackBuffer::new(expected)?)
            } else {
                None
            },
        })
    }

    fn finish(self) -> io::Result<Option<Box<[u8]>>> {
        self.retained.map(ExactPrepackBuffer::finish).transpose()
    }
}

impl Write for DraftPrepackOutput {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        match &mut self.retained {
            Some(output) => output.write(bytes),
            None => io::sink().write(bytes),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match &mut self.retained {
            Some(output) => output.flush(),
            None => io::sink().flush(),
        }
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
    use super::{DraftPrepackOutput, EngineeringQwenModelV1, ExactPrepackBuffer};
    use ferric_build::{
        QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES, SafetensorsError, SafetensorsSource, WeightStreamError,
        prepack_qwen3_draft_weights,
    };
    use ferric_spec::QWEN3_DRAFT_TENSOR_DATA_BYTES;
    use std::io::{Cursor, ErrorKind, Write};
    use std::path::Path;

    #[test]
    fn discarded_draft_output_never_allocates_a_payload_buffer() {
        // An impossible reservation would fail if the default path allocated.
        let mut output = DraftPrepackOutput::new(false, u64::MAX).unwrap();
        assert!(output.retained.is_none());
        output.write_all(&[1, 2, 3]).unwrap();
        output.write_all(&[]).unwrap();
        output.write_all(&[4, 5, 6, 7]).unwrap();
        output.flush().unwrap();
        assert!(output.retained.is_none());
        assert!(output.finish().unwrap().is_none());
    }

    #[test]
    fn retained_draft_output_is_exact_and_moved_without_copying() {
        let mut output = DraftPrepackOutput::new(true, 6).unwrap();
        let allocation = output.retained.as_ref().unwrap().bytes.as_ptr();
        output.write_all(&[0, 1]).unwrap();
        output.write_all(&[2, 3, 254, 255]).unwrap();
        output.flush().unwrap();
        let bytes = output.finish().unwrap().unwrap();
        assert_eq!(&*bytes, &[0, 1, 2, 3, 254, 255]);
        assert_eq!(bytes.as_ptr(), allocation);
    }

    #[test]
    fn retained_draft_output_rejects_incomplete_or_oversized_payloads() {
        let mut short = DraftPrepackOutput::new(true, 4).unwrap();
        short.write_all(&[1, 2]).unwrap();
        assert_eq!(short.finish().unwrap_err().kind(), ErrorKind::UnexpectedEof);

        let mut long = DraftPrepackOutput::new(true, 4).unwrap();
        long.write_all(&[1, 2]).unwrap();
        assert_eq!(
            long.write(&[3, 4, 5]).unwrap_err().kind(),
            ErrorKind::InvalidData
        );
        long.write_all(&[3, 4]).unwrap();
        assert_eq!(&*long.finish().unwrap().unwrap(), &[1, 2, 3, 4]);
        assert!(DraftPrepackOutput::new(true, u64::MAX).is_err());
    }

    #[test]
    fn both_intake_modes_reject_invalid_source_paths_before_allocating() {
        let source = Path::new("");
        let target_only = EngineeringQwenModelV1::open(source).err().unwrap();
        let with_draft = EngineeringQwenModelV1::open_with_draft(source)
            .err()
            .unwrap();
        assert!(target_only.starts_with("cannot reopen canonical Qwen source:"));
        assert_eq!(with_draft, target_only);
    }

    #[test]
    fn both_draft_output_modes_preserve_source_authentication_failures() {
        let header_bytes = QWEN3_DRAFT_WEIGHT_ARTIFACT_BYTES - QWEN3_DRAFT_TENSOR_DATA_BYTES - 8;
        let mut malformed_header = header_bytes.to_le_bytes().to_vec();
        malformed_header.resize(usize::try_from(header_bytes).unwrap() + 8, b' ');
        for (case, (name, bytes)) in [
            ("model-00001-of-00005.safetensors", vec![]),
            ("model.safetensors", vec![]),
            ("model.safetensors", 0_u64.to_le_bytes().to_vec()),
            ("model.safetensors", malformed_header),
        ]
        .into_iter()
        .enumerate()
        {
            let mut errors = Vec::new();
            for retain in [false, true] {
                let mut output = DraftPrepackOutput::new(retain, 16).unwrap();
                let error = prepack_qwen3_draft_weights(
                    SafetensorsSource {
                        name,
                        reader: Cursor::new(&bytes),
                    },
                    &mut output,
                )
                .unwrap_err();
                errors.push(error);
                assert!(
                    output
                        .retained
                        .as_ref()
                        .is_none_or(|buffer| buffer.bytes.is_empty())
                );
            }
            assert_eq!(errors[0], errors[1]);
            match (&errors[0], case) {
                (WeightStreamError::Source(SafetensorsError::ShardName { .. }), 0)
                | (WeightStreamError::Source(SafetensorsError::EarlyEof { .. }), 1)
                | (WeightStreamError::Source(SafetensorsError::HeaderLength { .. }), 2)
                | (WeightStreamError::Source(_), 3) => {}
                _ => panic!("unexpected draft authentication failure: {:?}", errors[0]),
            }
        }
    }

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
