//! Default-off decode-only split-K, reusing the authenticated resident KxN weights.

use super::{
    AuthenticatedModelWeightLayout, EngineeringTpArgumentV1, EngineeringTpBatchExecutionV2,
    EngineeringTpDispatchV1, EngineeringTpPagedPoolV1, EngineeringTpRankTransportV1,
    EngineeringTpReductionModeV3, EngineeringTpWaveTargetArtifactsV17, ModelConfig, Qwen3ModelRole,
    Qwen3TensorKind, Rank, Tensor, TpResult, allocate_tensor, dispatch,
};
use crate::tp_artifact::{
    ENGINEERING_TP_SPLITK_DOWN_EXPORTS_R1 as ROOTS, EngineeringTpArtifactV1,
    EngineeringTpSplitKDownArtifactR1, SplitKDownBindingR1,
};
use crate::tp_paged::EngineeringTpPreparedBatchV1;
use crate::tp_scheduler::{TpBatchRowKindV1, TpBatchRowV1};
use std::collections::BTreeSet;

const N: usize = 4096;
const K: usize = 12_288;
const PARTIALS: usize = 8 * N;

pub(super) struct BatchBinding {
    pool: u64,
    batch: u64,
    rows: Vec<(u32, u32, TpBatchRowKindV1)>,
    outputs: Vec<usize>,
}

impl BatchBinding {
    fn validate(&self, batch: &EngineeringTpPreparedBatchV1, published: usize) -> TpResult<bool> {
        if self.pool != batch.pool_identity()
            || self.batch != batch.id()
            || self.rows.len() != batch.rows().len()
            || self.outputs.len() != published
            || self
                .rows
                .iter()
                .zip(batch.rows())
                .any(|(&(token, position, _), row)| {
                    token != row.token() || position != row.position()
                })
        {
            return Err("split-K scheduler/prepared identity drifted".into());
        }
        Ok(self.rows.len() == 1
            && self.outputs == [0]
            && self.rows[0].2 == TpBatchRowKindV1::Decode)
    }
}

pub(super) struct Workspace {
    pub(super) binding: SplitKDownBindingR1,
    pub(super) scratch: Tensor,
    pub(super) selected: Option<bool>,
    pub(super) weights: Vec<(Tensor, Tensor)>,
    pub(super) batch: Option<BatchBinding>,
}

impl Workspace {
    pub(super) fn commands(
        &self,
        rank: &Rank,
        projection: &super::super::projection::ProjectionPolicy,
        layer: u32,
    ) -> TpResult<[EngineeringTpDispatchV1; 2]> {
        let (original, transposed) = self
            .weights
            .get(layer as usize)
            .copied()
            .ok_or("split-K layer missing")?;
        if self.selected != Some(true)
            || rank.geometry.rank != 0
            || layer >= 36
            || rank.activation.element_bytes != 2
            || rank.activation.elements < K
            || rank.partial.element_bytes != 4
            || rank.partial.elements < N
            || self.scratch.elements != PARTIALS
            || self.scratch.element_bytes != 4
            || rank.layers.get(layer as usize).is_none_or(|weights| {
                let current = weights.weight(Qwen3TensorKind::DownProjection);
                (current.id, current.elements, current.element_bytes) != (original.id, N * K, 2)
            })
        {
            return Err("split-K active layer/storage identity drifted".into());
        }
        let current = projection.resident_transposed(0, original)?;
        if (current.id, current.elements, current.element_bytes) != (transposed.id, N * K, 2)
            || [
                rank.activation.id,
                transposed.id,
                rank.partial.id,
                self.scratch.id,
            ]
            .into_iter()
            .collect::<BTreeSet<_>>()
            .len()
                != 4
        {
            return Err("split-K transposed/scratch/output binding drifted".into());
        }
        let activation = Tensor {
            elements: K,
            ..rank.activation
        };
        let output = Tensor {
            elements: N,
            ..rank.partial
        };
        let mut arguments = vec![activation.read(), transposed.read(), self.scratch.write()];
        arguments.extend([1, 4096, 12_288, 1, 2].map(EngineeringTpArgumentV1::U32));
        let partial = dispatch(ROOTS[0], 2048, arguments);
        let merge = dispatch(ROOTS[1], 64, vec![self.scratch.read(), output.write()]);
        Ok([
            super::super::row_profile::bind_storage(32, false, partial)?,
            super::super::row_profile::bind_storage(32, false, merge)?,
        ])
    }
}

pub(super) fn admit<R: EngineeringTpRankTransportV1>(
    transports: &mut [R],
    binding: SplitKDownBindingR1,
) -> TpResult<()> {
    if !cfg!(all(
        feature = "c1-ordered64",
        not(feature = "model-timestamps"),
        not(feature = "c1-token-program")
    )) || transports.len() != 1
        || transports[0].peer_group_rank().is_some()
        || transports[0].supports_token_program()
        || !transports[0].supports_ordered_batches64()
    {
        return Err("split-K requires ordinary nonpeer TP1 ordered64".into());
    }
    transports[0].require_loaded_image(binding.hsaco(), &ROOTS)
}

pub(super) fn admit_owned<R: EngineeringTpRankTransportV1>(
    transports: &mut [R],
    binding: SplitKDownBindingR1,
) -> TpResult<()> {
    if let Err(mut error) = admit(transports, binding) {
        for transport in transports {
            if let Err(close) = transport.close() {
                error.push_str(&format!("; split-K admission close: {close}"));
            }
        }
        return Err(error);
    }
    Ok(())
}

impl<R: EngineeringTpRankTransportV1> EngineeringTpBatchExecutionV2<R> {
    /// Admits the extra image and one128KiB scratch in both experimental arms.
    /// # Errors
    /// Rejects unsupported image/profile/storage and closes owned transports on failure.
    #[allow(clippy::too_many_arguments)]
    pub fn new_wide32_with_ordered64_splitk_down_r1(
        mut transports: Vec<R>,
        model: ModelConfig,
        weights: &[u8],
        layout: &AuthenticatedModelWeightLayout,
        pool: &EngineeringTpPagedPoolV1,
        artifacts: &EngineeringTpWaveTargetArtifactsV17<'_>,
        prefill: &EngineeringTpArtifactV1,
        split: &EngineeringTpArtifactV1,
        gemv: &EngineeringTpArtifactV1,
        copy: &EngineeringTpArtifactV1,
        down: &EngineeringTpSplitKDownArtifactR1,
    ) -> TpResult<Self> {
        admit_owned(&mut transports, down.binding())?;
        let mut driver = Self::new_wide32_with_ordered64_kv_copy_v1(
            transports, model, weights, layout, pool, artifacts, prefill, split, gemv, copy,
        )?;
        driver.allocate_splitk_down_workspace(down.binding())?;
        Ok(driver)
    }

    pub(super) fn allocate_splitk_down_workspace(
        &mut self,
        binding: SplitKDownBindingR1,
    ) -> TpResult<()> {
        let prepared =
            allocate_tensor(&mut self.inner.transports[0], PARTIALS, 4).and_then(|scratch| {
                if self.splitk_retained_ids().contains(&scratch.id) {
                    return Err("split-K allocator aliased resident storage".into());
                }
                Ok(Workspace {
                    binding,
                    scratch,
                    selected: None,
                    weights: Vec::new(),
                    batch: None,
                })
            });
        match prepared {
            Ok(workspace) => self.splitk_down_r1 = Some(workspace),
            Err(error) => {
                self.poisoned = true;
                return Err(match self.close() {
                    Ok(()) => error,
                    Err(close) => format!("{error}; split-K setup close: {close}"),
                });
            }
        }
        Ok(())
    }

    fn splitk_retained_ids(&self) -> BTreeSet<u64> {
        let mut ids = BTreeSet::new();
        for rank in &self.inner.ranks {
            ids.extend(
                [
                    rank.hidden,
                    rank.normalized,
                    rank.q,
                    rank.k,
                    rank.v,
                    rank.q_normalized,
                    rank.k_normalized,
                    rank.q_rotated,
                    rank.k_rotated,
                    rank.attention,
                    rank.gate,
                    rank.up,
                    rank.activation,
                    rank.partial,
                    rank.cos,
                    rank.sin,
                    rank.empty,
                    rank.token,
                    rank.logits,
                    rank.choice,
                ]
                .map(|tensor| tensor.id),
            );
            ids.extend(rank.globals.iter().map(|(_, tensor)| tensor.id));
            for layer in &rank.layers {
                ids.extend(layer.weights.iter().map(|(_, tensor)| tensor.id));
                ids.extend([layer.k_cache.id, layer.v_cache.id]);
            }
        }
        ids.extend(
            self.positions
                .iter()
                .chain(&self.page_tables)
                .map(|tensor| tensor.id),
        );
        ids.extend(self.fp32_logits.map(|tensor| tensor.id));
        ids.extend(self.projection.retained_tensor_ids());
        if let super::super::ReductionWorkspace::DeviceTp1(tensor) = self.inner.reduction {
            ids.insert(tensor.id);
        }
        if let Some(split) = self.split_attention_workspace_v25 {
            ids.extend([split.stats.id, split.numerators.id]);
        }
        ids
    }

    pub(super) fn splitk_profile_valid(&self) -> bool {
        cfg!(all(
            feature = "c1-ordered64",
            not(feature = "model-timestamps"),
            not(feature = "c1-token-program")
        )) && !self.poisoned
            && !self.inner.closed
            && self.row_capacity == 32
            && self.inner.row_capacity == 32
            && self.inner.plan.world_size() == 1
            && self.inner.plan.model().role == Qwen3ModelRole::Target8B
            && self.inner.plan.model().layers == 36
            && self.inner.plan.model().hidden_size == 4096
            && self.inner.ranks.len() == 1
            && self.inner.transports.len() == 1
            && self.inner.ranks[0].geometry.intermediate.count == 12_288
            && self.inner.transports[0].peer_group_rank().is_none()
            && !self.inner.transports[0].supports_token_program()
            && self.inner.sequences.is_none()
            && !self.inner.large_kv
            && !self.inner.draft_v10
            && self.numerical.is_none()
            && !self.inner.timing.is_enabled()
            && self.inner.residual_arithmetic.is_none()
            && self.inner.packed_c1.is_none()
            && self.head_profile_configured
            && self.prune_output_head
            && self.projection_configured
            && self.c1_wave_layers
            && self.wave_attention
            && self.fp32_logits.is_some()
            && self.fp32_argmax_v11.is_some()
            && self.fp32_argmax_v11 == self.admitted_argmax_v11
            && self.query_hoist_v14.is_some()
            && self.query_hoist_v14 == self.admitted_query_hoist_v14
            && self.wave_rmsnorm_v15.is_some()
            && self.wave_rmsnorm_v15 == self.admitted_wave_rmsnorm_v15
            && self.projection.mode == super::super::EngineeringTpProjectionModeV3::Mfma
            && self.reduction_mode() == EngineeringTpReductionModeV3::DeviceTp1V3
            && self.c1_packet_packing_v22 == Some(true)
            && self.c1_split_attention_v25 == Some(true)
            && self.prefill_kv_copy_v28 == Some(true)
            && self.partial_gemv_v28 == Some(false)
            && self.prefill32_pages_v1.is_none()
            && self.prefill16_ordered_v1.is_none()
            && self.packed_gate_up_r2.is_none()
            && self.packed_down_r1.is_none()
            && self.c1_kv_copy_v19.is_some()
            && self.c1_kv_copy_v19 == self.admitted_c1_kv_copy_v19
            && self.admitted_prefill_kv_copy_v27.is_some()
            && self.admitted_partial_gemv_v20.is_some()
            && self.split_attention_workspace_v25.is_some()
            && self.inner.ordered_batch_width.is_wide()
            && self.inner.transports[0].supports_ordered_batches64()
            && self
                .inner
                .ordered_batches
                .as_ref()
                .is_some_and(Vec::is_empty)
    }

    /// Selects the down-only experiment after ordinary composed-profile setup.
    /// # Errors
    /// Rejects repeated/late selection, missing transposes, aliases or other experiments.
    pub fn configure_ordered64_splitk_down_r1(
        &mut self,
        artifact: &EngineeringTpSplitKDownArtifactR1,
        enabled: bool,
    ) -> TpResult<()> {
        self.configure_splitk_down_binding_r1(artifact.binding(), enabled)
    }

    pub(super) fn configure_splitk_down_binding_r1(
        &mut self,
        binding: SplitKDownBindingR1,
        enabled: bool,
    ) -> TpResult<()> {
        if !self.splitk_profile_valid()
            || self.last_batch != 0
            || self.completed_batches != 0
            || self.inner.ranks[0].dispatches != 0
            || self.splitk_down_r1.as_ref().is_none_or(|workspace| {
                workspace.binding != binding
                    || workspace.selected.is_some()
                    || workspace.batch.is_some()
                    || !workspace.weights.is_empty()
                    || workspace.scratch.elements != PARTIALS
                    || workspace.scratch.element_bytes != 4
                    || self.splitk_retained_ids().contains(&workspace.scratch.id)
            })
        {
            return Err("split-K requires its fresh ordinary ordered64 TP1 composition".into());
        }
        let rank = &self.inner.ranks[0];
        if rank.layers.len() != 36 {
            return Err("split-K requires all36 layer weights".into());
        }
        let mut weights = Vec::with_capacity(36);
        let mut unique = BTreeSet::new();
        for layer in &rank.layers {
            let original = layer.weight(Qwen3TensorKind::DownProjection);
            let transposed = self.projection.resident_transposed(0, original)?;
            if original.elements != N * K
                || !unique.insert(original.id)
                || !unique.insert(transposed.id)
            {
                return Err(
                    "split-K requires distinct complete resident NxK/KxN down weights".into(),
                );
            }
            weights.push((original, transposed));
        }
        let workspace = self
            .splitk_down_r1
            .as_mut()
            .ok_or("split-K workspace disappeared")?;
        workspace.weights = weights;
        workspace.selected = Some(enabled);
        Ok(())
    }

    pub(super) fn bind_splitk_dispatch_rows(
        &mut self,
        batch: &EngineeringTpPreparedBatchV1,
        rows: &[TpBatchRowV1],
    ) -> TpResult<()> {
        if self
            .splitk_down_r1
            .as_mut()
            .and_then(|workspace| workspace.batch.take())
            .is_some()
        {
            return Err("split-K duplicate binding abandoned its prior selection".into());
        }
        self.validate(batch)?;
        if rows.is_empty()
            || rows.len() != batch.rows().len()
            || rows.iter().any(|row| row.request != rows[0].request)
            || batch
                .rows()
                .iter()
                .any(|row| row.sequence() != batch.rows()[0].sequence())
            || rows.iter().zip(batch.rows()).any(|(scheduled, prepared)| {
                scheduled.token_id != prepared.token()
                    || scheduled.absolute_position != prepared.position()
            })
            || (rows.iter().any(|row| row.kind == TpBatchRowKindV1::Decode) && rows.len() != 1)
        {
            return Err("split-K requires one exact scheduler request/phase binding".into());
        }
        self.splitk_down_r1
            .as_mut()
            .ok_or("split-K workspace missing")?
            .batch = Some(BatchBinding {
            pool: batch.pool_identity(),
            batch: batch.id(),
            rows: rows
                .iter()
                .map(|row| (row.token_id, row.absolute_position, row.kind))
                .collect(),
            outputs: rows
                .iter()
                .enumerate()
                .filter_map(|(index, row)| {
                    (row.kind != TpBatchRowKindV1::PrefillIntermediate).then_some(index)
                })
                .collect(),
        });
        Ok(())
    }

    pub(super) fn splitk_down_active_for_batch(
        &self,
        batch: &EngineeringTpPreparedBatchV1,
        published: usize,
    ) -> TpResult<bool> {
        let Some(workspace) = &self.splitk_down_r1 else {
            return Ok(false);
        };
        let active = workspace
            .batch
            .as_ref()
            .ok_or("split-K scheduler binding missing")?
            .validate(batch, published)?;
        Ok(workspace.selected == Some(true) && active)
    }

    pub(super) fn take_splitk_down_selection(
        &mut self,
        batch: &EngineeringTpPreparedBatchV1,
        outputs: &[usize],
    ) -> TpResult<bool> {
        let Some(workspace) = &mut self.splitk_down_r1 else {
            return Ok(false);
        };
        let binding = workspace
            .batch
            .take()
            .ok_or("split-K requires a fresh scheduler binding")?;
        let active = binding.validate(batch, outputs.len())?;
        if binding.outputs != outputs {
            return Err("split-K output selection differs from scheduler phase".into());
        }
        Ok(workspace.selected == Some(true) && active)
    }

    /// Actual selected route; ordinary engines remain baseline.
    #[must_use]
    pub fn splitk_down_mode(&self) -> &'static str {
        match self
            .splitk_down_r1
            .as_ref()
            .map(|workspace| workspace.selected)
        {
            Some(Some(true)) => "splitk8-down-mfma-r1",
            Some(None) => "unconfigured-splitk-down-r1",
            _ => "baseline",
        }
    }

    /// Additional scratch in either experimental arm; no new weight allocation.
    #[must_use]
    pub fn splitk_down_scratch_bytes(&self) -> u64 {
        self.splitk_down_r1.as_ref().map_or(0, |_| 131_072)
    }

    /// Existing down transposes retained by this selection, not additional memory.
    /// # Errors
    /// Rejects a missing selection or unrepresentable retained storage size.
    pub fn splitk_down_resident_weight_bytes(&self) -> TpResult<u64> {
        let workspace = self
            .splitk_down_r1
            .as_ref()
            .ok_or("split-K workspace missing")?;
        if workspace.selected.is_none() || workspace.weights.len() != 36 {
            return Err("split-K weight selection missing".into());
        }
        workspace
            .weights
            .iter()
            .try_fold(0_u64, |total, (_, tensor)| {
                let elements =
                    u64::try_from(tensor.elements).map_err(|_| "split-K weight extent overflow")?;
                let bytes = elements
                    .checked_mul(u64::from(tensor.element_bytes))
                    .ok_or("split-K weight byte overflow")?;
                total
                    .checked_add(bytes)
                    .ok_or_else(|| "split-K weight total overflow".into())
            })
    }
}
