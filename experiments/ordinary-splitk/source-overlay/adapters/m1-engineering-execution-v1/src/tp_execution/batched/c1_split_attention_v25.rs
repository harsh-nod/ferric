//! V21 partial/merge selection with optional ordered C1 completion packing.

use super::{
    EngineeringTpBatchExecutionV2, EngineeringTpDispatchV1, EngineeringTpExecutionV1,
    EngineeringTpRankTransportV1, EngineeringTpWaveTargetArtifactsV17,
    EngineeringTpWaveTargetModeV17, Tensor, TpResult, allocate_tensor, dispatch,
};
use crate::tp_artifact::{
    ENGINEERING_TP_SPLIT_ATTENTION_EXPORTS_V21, EngineeringTpArtifactV1, SplitAttentionBindingV21,
};
use crate::tp_execution::EngineeringTpArgumentV1::U32;
use crate::tp_execution::reduction::AttentionProducerSchedule;
use crate::tp_paged::{EngineeringTpPagedPoolV1, EngineeringTpPreparedBatchV1};
use ferric_build::AuthenticatedModelWeightLayout;
use ferric_spec::{ModelConfig, Qwen3ModelRole};

#[derive(Clone, Copy)]
pub(super) struct Workspace {
    pub(super) image: SplitAttentionBindingV21,
    pub(super) stats: Tensor,
    pub(super) numerators: Tensor,
}

impl Workspace {
    pub(super) fn allocate<R: EngineeringTpRankTransportV1>(
        transport: &mut R,
        image: SplitAttentionBindingV21,
    ) -> TpResult<Self> {
        let stats = allocate_tensor(transport, 512, 4)?;
        let numerators = allocate_tensor(transport, 32_768, 4)?;
        if stats.id == numerators.id {
            return Err("V21 scratch allocations must be distinct".into());
        }
        Ok(Self {
            image,
            stats,
            numerators,
        })
    }
}

#[derive(Clone, Copy)]
pub(super) struct SplitContext(u32);

impl SplitContext {
    pub(super) fn select(
        enabled: bool,
        batch: &EngineeringTpPreparedBatchV1,
    ) -> TpResult<Option<Self>> {
        if !enabled || batch.rows().len() != 1 {
            return Ok(None);
        }
        Self::from_position(enabled, batch.rows().len(), batch.rows()[0].position())
    }

    pub(super) fn from_position(
        enabled: bool,
        rows: usize,
        position: u32,
    ) -> TpResult<Option<Self>> {
        if !enabled || rows != 1 {
            return Ok(None);
        }
        let context = position
            .checked_add(1)
            .ok_or("V25 actual context overflow")?;
        Ok((128..=256).contains(&context).then_some(Self(context)))
    }

    pub(super) const fn tokens(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy)]
pub(super) struct SplitDispatch {
    workspace: Workspace,
    positions: Tensor,
    table: Tensor,
    stride: u32,
    pages: u32,
    context: SplitContext,
}

impl SplitDispatch {
    pub(super) const fn tokens(self) -> u32 {
        self.context.tokens()
    }

    pub(super) fn enqueue<R: EngineeringTpRankTransportV1>(
        self,
        inner: &mut EngineeringTpExecutionV1<R>,
        layer: usize,
        batch_ordinal: u64,
    ) -> TpResult<()> {
        #[cfg(not(feature = "model-timestamps"))]
        let _ = batch_ordinal;
        let producer_ready = inner.packed_c1.map_or_else(
            || {
                inner
                    .ordered_batches
                    .as_ref()
                    .is_some_and(|pending| pending.len() == 8)
            },
            |state| {
                state.attention_schedule == AttentionProducerSchedule::Split8V21
                    && state.producer_packets == 8
                    && state.collective.expected().layer as usize == layer
                    && state.collective.expected().operation
                        == super::Qwen3TensorParallelCollectiveV1::AttentionOutputSum
            },
        );
        if !producer_ready
            || inner.ordered_batches.is_none()
            || inner.ranks.len() != 1
            || inner.transports.len() != 1
            || inner.sequences.is_some()
        {
            return Err(
                "V25 partial/merge requires the exact ordered attention producer prefix".into(),
            );
        }
        let rank = &inner.ranks[0];
        let roots = ENGINEERING_TP_SPLIT_ATTENTION_EXPORTS_V21;
        let commands = [
            dispatch(
                roots[0],
                256,
                vec![
                    rank.q_rotated.read(),
                    rank.layers[layer].k_cache.read(),
                    rank.layers[layer].v_cache.read(),
                    self.positions.read(),
                    self.table.read(),
                    self.workspace.stats.write(),
                    self.workspace.numerators.write(),
                    U32(1),
                    U32(1),
                    U32(self.stride),
                    U32(self.pages),
                    U32(self.tokens()),
                ],
            ),
            dispatch(
                roots[1],
                32,
                vec![
                    self.workspace.stats.read(),
                    self.workspace.numerators.read(),
                    rank.attention.write(),
                ],
            ),
        ];
        // Validate both packets before either can enter the pending producer group.
        let bound = commands
            .into_iter()
            .map(|command| super::super::row_profile::bind_mode(false, 32, false, command))
            .collect::<TpResult<Vec<EngineeringTpDispatchV1>>>()?;
        #[cfg(feature = "model-timestamps")]
        let mut first = true;
        for command in bound {
            #[cfg(feature = "model-timestamps")]
            {
                if !first {
                    // The producer already supplied the partial packet's tag.
                    inner.timestamp_operation(
                        batch_ordinal,
                        Some(u32::try_from(layer).map_err(|_| "timestamp layer conversion")?),
                        crate::model_timestamps::Operation::Attention,
                    )?;
                }
                first = false;
            }
            inner.dispatch_each(|_| command.clone())?;
        }
        Ok(())
    }
}

pub(super) fn admit<R: EngineeringTpRankTransportV1>(
    transports: &mut [R],
    image: SplitAttentionBindingV21,
) -> TpResult<()> {
    if transports.len() != 1
        || transports[0].peer_group_rank().is_some()
        || !transports[0].supports_ordered_batches()
    {
        return Err("V25 requires one nonpeer ordered TP1 transport".into());
    }
    transports[0].require_loaded_image(image.hsaco, &ENGINEERING_TP_SPLIT_ATTENTION_EXPORTS_V21)
}

impl<R: EngineeringTpRankTransportV1> EngineeringTpBatchExecutionV2<R> {
    /// Admits all six images before allocation and owns one reusable scratch pair in both arms.
    /// # Errors
    /// Rejects missing image, unsupported base state, allocation failure or teardown failure.
    pub fn new_wide32_with_c1_split_attention_v25(
        mut transports: Vec<R>,
        model: ModelConfig,
        weights: &[u8],
        layout: &AuthenticatedModelWeightLayout,
        pool: &EngineeringTpPagedPoolV1,
        artifacts: &EngineeringTpWaveTargetArtifactsV17<'_>,
        split: &EngineeringTpArtifactV1,
    ) -> TpResult<Self> {
        let image = split
            .split_attention_binding_v21()
            .ok_or_else(|| String::from("V25 requires its exact separate V21 image"))
            .and_then(|image| admit(&mut transports, image).map(|()| image));
        let image = match image {
            Ok(image) => image,
            Err(error) => {
                for transport in &mut transports {
                    let _ = transport.close();
                }
                return Err(error);
            }
        };
        let mut driver = Self::new_wide32_with_wave_target_v17(
            transports, model, weights, layout, pool, artifacts,
        )?;
        driver.allocate_split_workspace_v25(image)?;
        Ok(driver)
    }

    pub(super) fn allocate_split_workspace_v25(
        &mut self,
        image: SplitAttentionBindingV21,
    ) -> TpResult<()> {
        match Workspace::allocate(&mut self.inner.transports[0], image) {
            Ok(workspace) => self.split_attention_workspace_v25 = Some(workspace),
            Err(error) => {
                self.poisoned = true;
                return Err(match self.inner.close() {
                    Ok(()) => error,
                    Err(close) => format!("{error}; V21 scratch setup close: {close}"),
                });
            }
        }
        Ok(())
    }

    /// Selects the control/candidate once; only bounded one-row contexts use the split image.
    /// # Errors
    /// Rejects late/repeated configuration, wrong bindings or another experimental route.
    pub fn configure_ordered_c1_split_attention_v25(
        &mut self,
        artifacts: &EngineeringTpWaveTargetArtifactsV17<'_>,
        split: &EngineeringTpArtifactV1,
        enabled: bool,
    ) -> TpResult<()> {
        let image = split
            .split_attention_binding_v21()
            .ok_or("V25 requires the exact V21 image")?;
        let ((argmax, attention), rmsnorm) = artifacts
            .argmax
            .fp32_argmax_binding_v11()
            .zip(artifacts.attention.query_hoist_binding_v14())
            .zip(artifacts.rmsnorm.wave_rmsnorm_binding_v15())
            .ok_or("V25 requires all exact historical V17 images")?;
        self.configure_split_bindings_v25(argmax, attention, rmsnorm, image, enabled)
    }

    pub(super) fn configure_split_bindings_v25(
        &mut self,
        argmax: crate::tp_artifact::Fp32ArgmaxBindingV11,
        attention: crate::tp_artifact::QueryHoistBindingV14,
        rmsnorm: crate::tp_artifact::WaveRmsNormBindingV15,
        image: SplitAttentionBindingV21,
        enabled: bool,
    ) -> TpResult<()> {
        self.validate_split_storage_v25(image, false)?;
        self.configure_ordered_c1_wave_target_bindings_v17(
            argmax,
            attention,
            rmsnorm,
            EngineeringTpWaveTargetModeV17::Combined,
        )?;
        self.c1_split_attention_v25 = Some(enabled);
        Ok(())
    }

    pub(super) fn validate_split_storage_v25(
        &self,
        image: SplitAttentionBindingV21,
        prefill_admitted: bool,
    ) -> TpResult<()> {
        self.validate_split_storage_with_c1_v1(image, prefill_admitted, None)
    }

    pub(super) fn validate_split_storage_with_c1_v1(
        &self,
        image: SplitAttentionBindingV21,
        prefill_admitted: bool,
        c1: Option<crate::tp_artifact::C1KvCopyBindingV19>,
    ) -> TpResult<()> {
        let workspace = self
            .split_attention_workspace_v25
            .ok_or("V25 scratch was not allocated")?;
        if workspace.image != image
            || workspace.stats.id == workspace.numerators.id
            || workspace.stats.elements != 512
            || workspace.stats.element_bytes != 4
            || workspace.numerators.elements != 32_768
            || workspace.numerators.element_bytes != 4
            || self.c1_split_attention_v25.is_some()
            || self.c1_packet_packing_v22.is_some()
            || self.c1_kv_copy_v19.is_some()
            || self.admitted_c1_kv_copy_v19 != c1
            || self.prefill_kv_copy_v28.is_some()
            || (!prefill_admitted && self.admitted_prefill_kv_copy_v27.is_some())
            || self.inner.plan.world_size() != 1
            || self.inner.plan.model().role != Qwen3ModelRole::Target8B
            || self.inner.plan.model().layers != 36
            || self.inner.plan.model().hidden_size != 4096
            || self.inner.ranks.len() != 1
            || self.inner.transports.len() != 1
            || self.row_capacity != 32
            || self.inner.large_kv
            || self.inner.draft_v10
            || self.inner.closed
            || self.poisoned
            || self.last_batch != 0
            || self.completed_batches != 0
        {
            return Err("V25 requires fresh exact V21 scratch, target TP1/32-row V17 and no other candidate".into());
        }
        Ok(())
    }

    /// Explicitly composes V22 grouping with an already selected V25 control/candidate.
    /// Partial and merge retain queue order; scratch reuse follows merge consumption even
    /// when a completion boundary divides them. Multirow and headless batches stay unpacked.
    /// # Errors
    /// Rejects absent V25 selection, repeated/late selection or incompatible execution.
    pub fn configure_c1_split_packet_packing_v25(&mut self, enabled: bool) -> TpResult<()> {
        if self.c1_split_attention_v25.is_none() || self.split_attention_workspace_v25.is_none() {
            return Err("V25 packing requires an explicitly selected V25 attention policy".into());
        }
        self.configure_c1_packet_packing(enabled)
    }

    /// Actual configured policy; multirow or out-of-domain batches still use V14.
    #[must_use]
    pub const fn split_attention_mode(&self) -> &'static str {
        if matches!(self.c1_split_attention_v25, Some(true)) {
            "split8-v21"
        } else {
            "baseline"
        }
    }

    /// Extra payload bytes, excluding allocator rounding; allocated equally in both V25 arms.
    #[must_use]
    pub const fn split_attention_workspace_bytes(&self) -> u64 {
        if self.split_attention_workspace_v25.is_some() {
            133_120
        } else {
            0
        }
    }

    /// Expected completed packets for this immutable prepared batch, before publication.
    /// # Errors
    /// Rejects invalid prepared metadata or arithmetic overflow.
    pub fn expected_dispatch_counts_for_batch(
        &self,
        batch: &EngineeringTpPreparedBatchV1,
        published: usize,
    ) -> TpResult<Vec<u64>> {
        if self.prefill32_pages_v1 == Some(true) && batch.rows().len() == 32 {
            return Err("prefill page pair requires selection-aware packet preflight".into());
        }
        if self.c1_split_attention_v25.is_none()
            && self.packed_gate_up_r2.is_none()
            && self.packed_down_r1.is_none()
            && self.splitk_down_r1.is_none()
        {
            return Ok(self.expected_dispatch_counts(published));
        }
        self.validate(batch)?;
        if published > batch.rows().len() {
            return Err("published row count exceeds physical rows".into());
        }
        let split =
            SplitContext::select(self.c1_split_attention_v25 == Some(true), batch)?.is_some();
        let packed_gate_up = self.packed_gate_up_active(batch.rows().len(), published);
        let packed_down = self.packed_down_active_for_batch(batch, published)?;
        let splitk_down = self.splitk_down_active_for_batch(batch, published)?;
        self.expected_dispatch_counts(published)
            .into_iter()
            .map(|count| {
                count
                    .checked_add(if split { 36 } else { 0 })
                    .and_then(|count| {
                        count.checked_add(if packed_gate_up || packed_down || splitk_down {
                            36
                        } else {
                            0
                        })
                    })
                    .ok_or_else(|| "V25 expected packet overflow".to_owned())
            })
            .collect()
    }

    pub(super) fn prepare_split_attention_v25(
        &self,
        batch: &EngineeringTpPreparedBatchV1,
    ) -> TpResult<Option<SplitDispatch>> {
        let Some(context) = SplitContext::select(self.c1_split_attention_v25 == Some(true), batch)?
        else {
            return Ok(None);
        };
        let workspace = self
            .split_attention_workspace_v25
            .ok_or("V25 scratch unavailable")?;
        Ok(Some(SplitDispatch {
            workspace,
            positions: self.positions[0],
            table: self.page_tables[0],
            stride: self.table_stride,
            pages: self.physical_pages,
            context,
        }))
    }
}
