//! Explicit `Draft06B` driver over independently admitted paged kernels.

use super::{BatchedProfile, EngineeringTpBatchExecutionV2, EngineeringTpBatchOutputV2};
use crate::tp_artifact::{
    DraftBindingV10, ENGINEERING_DRAFT_BATCH32_EXPORTS_V10, EngineeringTpArtifactV1,
};
use crate::tp_execution::{
    EngineeringTpProjectionModeV3, EngineeringTpRankTransportV1, TpResult, allocate_tensor,
};
use crate::tp_paged::{EngineeringTpPagedPoolV1, EngineeringTpPreparedBatchV1};
use ferric_build::AuthenticatedModelWeightLayout;
use ferric_spec::{ModelConfig, Qwen3ModelRole};

/// Owns only the independent TP1, 32-row, FP32-head draft profile.
/// No target/wave/peer configuration or ordinary-output-to-speculation conversion exists.
///
/// ```compile_fail
/// use ferric_m1_engineering_execution_v1::tp_execution::{EngineeringTpRankTransportV1, batched::EngineeringTpDraftBatchExecutionV10};
/// fn no_target_mode<R: EngineeringTpRankTransportV1>(draft: &mut EngineeringTpDraftBatchExecutionV10<R>) {
///     draft.configure_wave_attention(true).unwrap();
/// }
/// ```
pub struct EngineeringTpDraftBatchExecutionV10<R: EngineeringTpRankTransportV1> {
    inner: EngineeringTpBatchExecutionV2<R>,
    image: DraftBindingV10,
}

#[cfg(test)]
impl<R: EngineeringTpRankTransportV1> EngineeringTpDraftBatchExecutionV10<R> {
    pub(super) fn recording(inner: EngineeringTpBatchExecutionV2<R>) -> Self {
        Self {
            inner,
            image: DraftBindingV10::recording(),
        }
    }

    pub(super) fn recording_inner(&mut self) -> &mut EngineeringTpBatchExecutionV2<R> {
        &mut self.inner
    }
}

impl<R: EngineeringTpRankTransportV1> EngineeringTpDraftBatchExecutionV10<R> {
    /// Admits the complete draft image before allocating any resident storage.
    /// Arithmetic is scalar or MFMA, with a fixed FP32 head and draft device residual.
    /// # Errors
    /// Rejects wrong model/image/scope, nonfresh storage, unsupported transport or
    /// mode, and every weight identity, allocation or upload failure.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mut transports: Vec<R>,
        model: ModelConfig,
        weights: &[u8],
        layout: &AuthenticatedModelWeightLayout,
        pool: &EngineeringTpPagedPoolV1,
        artifact: &EngineeringTpArtifactV1,
        projection: EngineeringTpProjectionModeV3,
    ) -> TpResult<Self> {
        let binding = artifact.draft_binding().filter(|_| {
            matches!(
                projection,
                EngineeringTpProjectionModeV3::Baseline | EngineeringTpProjectionModeV3::Mfma
            ) && model
                == layout
                    .admission()
                    .prepacked()
                    .deployment()
                    .draft_model
                    .config
        });
        let Some(image) = binding else {
            for transport in &mut transports {
                let _ = transport.close();
            }
            return Err(
                "draft v10 requires its exact admitted model/image and scalar or MFMA projection"
                    .into(),
            );
        };
        let tied = (|| {
            let embedding = layout
                .lookup(
                    Qwen3ModelRole::Draft06B,
                    ferric_spec::Qwen3TensorKind::TokenEmbedding,
                    ferric_spec::QWEN3_NO_LAYER,
                )
                .map_err(|e| e.to_string())?;
            let head = layout
                .lookup(
                    Qwen3ModelRole::Draft06B,
                    ferric_spec::Qwen3TensorKind::LanguageModelHead,
                    ferric_spec::QWEN3_NO_LAYER,
                )
                .map_err(|e| e.to_string())?;
            if embedding.sha256() != head.sha256()
                || crate::tp_execution::section_bytes(weights, embedding.destination_range())?
                    != crate::tp_execution::section_bytes(weights, head.destination_range())?
            {
                return Err("draft v10 authenticated tied embedding/head differ".into());
            }
            Ok::<_, String>(())
        })();
        if let Err(error) = tied {
            for transport in &mut transports {
                let _ = transport.close();
            }
            return Err(error);
        }
        let mut inner = EngineeringTpBatchExecutionV2::new_profile(
            transports,
            model,
            weights,
            layout,
            pool,
            BatchedProfile::Draft(image),
        )?;
        let configure = (|| {
            inner.configure_projection(projection, weights, layout)?;
            inner.configure_output_head_pruning(true)?;
            let elements = inner.inner.ranks[0].hidden.elements;
            let scratch = allocate_tensor(&mut inner.inner.transports[0], elements, 2)?;
            inner.inner.reduction = crate::tp_execution::ReductionWorkspace::DeviceTp1(scratch);
            let logits = 32_usize
                .checked_mul(model.vocabulary_size as usize)
                .ok_or("draft logits extent overflow")?;
            inner.fp32_logits = Some(allocate_tensor(&mut inner.inner.transports[0], logits, 4)?);
            inner.head_profile_configured = true;
            Ok::<_, String>(())
        })();
        if let Err(error) = configure {
            return Err(match inner.close() {
                Ok(()) => error,
                Err(close) => format!("{error}; draft setup close: {close}"),
            });
        }
        Ok(Self { inner, image })
    }

    /// Runs role-local prefill/proposal work; ordinary choices are not settlement evidence.
    /// # Errors
    /// Rejects stale/foreign metadata, invalid output rows or incomplete device work.
    pub fn execute_selected(
        &mut self,
        batch: &EngineeringTpPreparedBatchV1,
        output_rows: &[usize],
    ) -> TpResult<EngineeringTpBatchOutputV2> {
        self.inner.execute_selected(batch, output_rows)
    }

    /// Seals exact completed draft inputs, including the distinguished one-row catch-up.
    /// The ticket determines the consumed token; no caller-supplied completion is accepted.
    /// # Errors
    /// Rejects malformed/foreign/stale work and every incomplete device operation.
    /// The owner must call `fail_submitted` after an execution error; no KV is committed here.
    pub fn execute_speculative_draft(
        &mut self,
        work: crate::tp_paged::speculative::EngineeringTpSpeculativeDraftWorkV1<'_>,
    ) -> TpResult<crate::tp_paged::speculative::EngineeringTpSpeculativeDraftResultV1> {
        if !work.validate()
            || !self.inner.inner.draft_v10
            || self.inner.inner.plan.model().role != Qwen3ModelRole::Draft06B
        {
            return Err("speculative draft requires exact role-local consumed inputs".into());
        }
        let output = self.inner.execute_selected(work.batch(), &[])?;
        if !output.choices.is_empty() {
            self.inner.poisoned = true;
            return Err("draft consumed-input completion unexpectedly published choices".into());
        }
        work.seal(output.completion).map_err(|error| {
            self.inner.poisoned = true;
            format!("speculative draft completion binding: {error:?}")
        })
    }

    /// Exact admitted image identity retained for this driver's whole lifetime.
    #[must_use]
    pub const fn image_id(&self) -> [u8; 32] {
        self.image.hsaco
    }

    /// Cumulative completed packets, not per-kernel timing or published tokens.
    #[must_use]
    pub fn dispatch_counts(&self) -> Vec<u64> {
        self.inner.dispatch_counts()
    }

    /// Exact packet count for one forward with this output selection.
    #[must_use]
    pub fn expected_dispatch_counts(&self, published: usize) -> Vec<u64> {
        self.inner.expected_dispatch_counts(published)
    }

    /// Fixed active-row capacity of the separately admitted draft profile.
    #[must_use]
    pub const fn row_capacity(&self) -> usize {
        self.inner.row_capacity()
    }

    /// Additional authenticated transpose payload, excluding runtime overhead.
    #[must_use]
    pub const fn transposed_weight_bytes(&self) -> u64 {
        self.inner.transposed_weight_bytes()
    }

    /// Completed role-local batches, including completion-bound catch-up work.
    #[must_use]
    pub const fn completed_batches(&self) -> u64 {
        self.inner.completed_batches()
    }

    /// Closes the owned worker even after submitted uncertainty.
    /// # Errors
    /// Reports a teardown that cannot be confirmed.
    pub fn close(&mut self) -> TpResult<()> {
        self.inner.close()
    }
}

pub(super) fn validate_binding<R: EngineeringTpRankTransportV1>(
    transports: &mut [R],
    model: ModelConfig,
    pool: &EngineeringTpPagedPoolV1,
    image: DraftBindingV10,
) -> TpResult<()> {
    if model.validate().is_err()
        || model.role != Qwen3ModelRole::Draft06B
        || model.layers != 28
        || model.hidden_size != 1024
        || model.intermediate_size != 3072
        || model.query_heads != 16
        || model.kv_heads != 8
        || model.head_dim != 128
        || model.vocabulary_size != 151_936
        || !model.tie_word_embeddings
        || pool.scope().model != *model.model_id.as_bytes()
        || !pool.is_empty()
        || pool.row_capacity() != 32
        || pool.large_kv_binding().is_some()
        || pool.limits().physical_page_count() > 512
        || pool.limits().context_tokens() > 8192
        || transports.len() != 1
        || transports.iter().any(|transport| {
            transport.peer_group_rank().is_some()
                || transport.supports_concurrent_rounds()
                || transport.supports_sequences()
                || transport.supports_ordered_batches()
        })
    {
        return Err("draft v10 requires exact Draft06B TP1, fresh independent32-row legacy pool and no peer/sequence transport".into());
    }
    transports[0].require_loaded_image(image.hsaco, &ENGINEERING_DRAFT_BATCH32_EXPORTS_V10)
}
