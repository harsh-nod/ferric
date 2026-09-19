//! Isolated V21 partial/merge selection; scratch retires at the existing residual frontier.

use super::{
    EngineeringTpBatchExecutionV2, EngineeringTpDispatchV1, EngineeringTpExecutionV1,
    EngineeringTpRankTransportV1, EngineeringTpWaveTargetArtifactsV17,
    EngineeringTpWaveTargetModeV17, Tensor, TpResult, allocate_tensor, dispatch,
};
use crate::tp_artifact::{
    ENGINEERING_TP_SPLIT_ATTENTION_EXPORTS_V21, EngineeringTpArtifactV1, SplitAttentionBindingV21,
};
use crate::tp_execution::EngineeringTpArgumentV1::U32;
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
        let context = batch.rows()[0]
            .position()
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
    ) -> TpResult<()> {
        if inner.packed_c1.is_some()
            || inner
                .ordered_batches
                .as_ref()
                .is_none_or(|pending| pending.len() != 8)
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
        for command in bound {
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
            || self.admitted_c1_kv_copy_v19.is_some()
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
        self.configure_ordered_c1_wave_target_bindings_v17(
            argmax,
            attention,
            rmsnorm,
            EngineeringTpWaveTargetModeV17::Combined,
        )?;
        self.c1_split_attention_v25 = Some(enabled);
        Ok(())
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
        if self.c1_split_attention_v25.is_none() {
            return Ok(self.expected_dispatch_counts(published));
        }
        self.validate(batch)?;
        if published > batch.rows().len() {
            return Err("published row count exceeds physical rows".into());
        }
        let split =
            SplitContext::select(self.c1_split_attention_v25 == Some(true), batch)?.is_some();
        self.expected_dispatch_counts(published)
            .into_iter()
            .map(|count| {
                count
                    .checked_add(if split { 36 } else { 0 })
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
