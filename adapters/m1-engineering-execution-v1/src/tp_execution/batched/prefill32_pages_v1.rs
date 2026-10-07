//! Opt-in two-page copies over one genuine prepared 32-row forward.

use super::{
    EngineeringTpArgumentV1, EngineeringTpBatchExecutionV2, EngineeringTpDispatchV1,
    EngineeringTpExecutionV1, EngineeringTpPreparedBatchV1, EngineeringTpRankTransportV1,
    EngineeringTpReductionModeV3, Qwen3ModelRole, Rank, Tensor, TpResult, dispatch,
};
use crate::tp_artifact::{EngineeringTpArtifactV1, PrefillKvCopyBindingV27};
use crate::tp_execution::EngineeringTpBufferAccessV1::{Read, Write};

const HALF_ELEMENTS: usize = 16 * 1024;
const HALF_BYTES: usize = HALF_ELEMENTS * 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct PrefillPagePairV1 {
    first: u32,
    pages: [u32; 2],
    pool_pages: u32,
    terminal: bool,
}

impl PrefillPagePairV1 {
    pub(super) fn select(
        batch: &EngineeringTpPreparedBatchV1,
        output_rows: &[usize],
        pool_pages: u32,
        context: u32,
    ) -> TpResult<Option<Self>> {
        let rows = batch.rows();
        if rows.len() != 32 {
            return Ok(None);
        }
        if !output_rows.is_empty() && output_rows != [31] {
            return Err("prefill page pair requires no output or the terminal row 31".into());
        }
        if !(2..=512).contains(&pool_pages) || !(1..=8192).contains(&context) {
            return Err("prefill page pair storage bounds differ".into());
        }
        let first = rows[0].position();
        if first > 8160
            || !first.is_multiple_of(16)
            || rows.iter().zip(first..first + 32).any(|(row, position)| {
                row.sequence() != rows[0].sequence() || row.position() != position
            })
        {
            return Ok(None);
        }
        let pages = [
            rows[0].writable_physical_page(),
            rows[16].writable_physical_page(),
        ];
        if pages[0] == pages[1]
            || pages.iter().any(|page| *page >= pool_pages)
            || first + 31 >= context
            || rows.iter().enumerate().any(|(index, row)| {
                let page = pages[index / 16];
                row.writable_physical_page() != page
                    || row.writable_token_offset() != row.position() % 16
                    || row.physical_pages().get((row.position() / 16) as usize) != Some(&page)
            })
        {
            return Err(
                "prefill page pair requires two distinct exact prepared writable pages".into(),
            );
        }
        Ok(Some(Self {
            first,
            pages,
            pool_pages,
            terminal: !output_rows.is_empty(),
        }))
    }

    pub(super) fn execution_order(self) -> Vec<usize> {
        if self.terminal {
            std::iter::once(31).chain(16..31).chain(0..16).collect()
        } else {
            (0..32).collect()
        }
    }

    pub(super) fn commands(
        self,
        rank: &Rank,
        layer: usize,
    ) -> TpResult<[EngineeringTpDispatchV1; 2]> {
        let source = [rank.k_rotated, rank.v];
        let target = [rank.layers[layer].k_cache, rank.layers[layer].v_cache];
        if source
            .iter()
            .any(|tensor| tensor.elements != 32 * 1024 || tensor.element_bytes != 2)
            || target.iter().any(|tensor| {
                tensor.elements != self.pool_pages as usize * HALF_ELEMENTS
                    || tensor.element_bytes != 2
            })
        {
            return Err("prefill page pair allocation extents differ".into());
        }
        let ids = [source[0].id, source[1].id, target[0].id, target[1].id];
        if ids
            .iter()
            .enumerate()
            .any(|(index, id)| *id == 0 || ids[index + 1..].contains(id))
        {
            return Err("prefill page pair requires four distinct owned allocations".into());
        }
        let buffer = |tensor: Tensor, offset, access| EngineeringTpArgumentV1::Buffer {
            id: tensor.id,
            offset,
            elements: HALF_ELEMENTS,
            element_bytes: 2,
            access,
        };
        let command = |half: usize| {
            let logical = if self.terminal { 1 - half } else { half };
            let page = self.pages[logical];
            dispatch(
                crate::tp_artifact::ENGINEERING_TP_PREFILL_KV_COPY_EXPORTS_V27[0],
                256,
                vec![
                    buffer(source[0], half * HALF_BYTES, Read),
                    buffer(source[1], half * HALF_BYTES, Read),
                    buffer(target[0], page as usize * HALF_BYTES, Write),
                    buffer(target[1], page as usize * HALF_BYTES, Write),
                    EngineeringTpArgumentV1::U32(self.first + u32::from(logical == 1) * 16),
                    EngineeringTpArgumentV1::U32(page),
                    EngineeringTpArgumentV1::U32(self.pool_pages),
                    EngineeringTpArgumentV1::U32(u32::from(self.terminal && half == 0)),
                ],
            )
        };
        let bind = |half| {
            super::super::row_profile::bind_prefill_page_half_v1(
                32,
                false,
                [source[0].elements, source[1].elements],
                command(half),
            )
        };
        // Both exact views are admitted before either enters the producer group.
        Ok([bind(0)?, bind(1)?])
    }

    pub(super) fn enqueue<R: EngineeringTpRankTransportV1>(
        self,
        inner: &mut EngineeringTpExecutionV1<R>,
        layer: usize,
    ) -> TpResult<()> {
        let packed = inner.packed_c1;
        let program = packed.is_some_and(|state| state.prefill_program && state.prefill_rows == 32)
            && inner.transports.len() == 1
            && inner.transports[0].supports_prefill32_program();
        let expected = packed
            .map_or(inner.collective, |state| state.collective)
            .expected();
        if inner.closed
            || inner.draft_v10
            || inner.large_kv
            || inner.row_capacity != 32
            || inner.hidden.len() != 32 * 4096
            || inner.ranks.len() != 1
            || inner.transports.len() != 1
            || inner.sequences.is_some()
            || (inner.packed_c1.is_some() && !program)
            || (program && packed.is_none_or(|state| state.producer_packets != 7))
            || inner.reduction.mode() != EngineeringTpReductionModeV3::DeviceTp1V3
            || expected.layer as usize != layer
            || expected.operation != super::Qwen3TensorParallelCollectiveV1::AttentionOutputSum
            || inner
                .ordered_batches
                .as_ref()
                .is_none_or(|pending| pending.len() != if program { 1 + layer * 18 + 7 } else { 7 })
        {
            return Err(
                "prefill page pair requires the exact ordered 32-row producer prefix".into(),
            );
        }
        let commands = self.commands(&inner.ranks[0], layer)?;
        // This separate binder is the only route accepting the second half view.
        inner
            .ordered_batches
            .as_mut()
            .expect("validated producer group")
            .extend(commands);
        if program {
            inner
                .packed_c1
                .as_mut()
                .expect("validated prefill32 program")
                .producer_packets += 2;
        }
        Ok(())
    }
}

impl<R: EngineeringTpRankTransportV1> EngineeringTpBatchExecutionV2<R> {
    /// Selects the explicit slots512 transport without changing decode or images.
    /// # Errors
    /// Rejects nonfresh, incompatible or unadmitted two-page compositions.
    pub fn configure_prefill32_program_v1(
        &mut self,
        copy: &EngineeringTpArtifactV1,
    ) -> TpResult<()> {
        self.validate_prefill_profile_v1(true, true)?;
        let binding = copy
            .prefill_kv_copy_binding_v27()
            .ok_or("native prefill32 requires V27")?;
        self.configure_prefill32_pages_binding_v1(binding, true)
    }

    pub(super) fn prefill32_program_active_v1(
        &self,
        batch: &EngineeringTpPreparedBatchV1,
        output_rows: &[usize],
    ) -> TpResult<bool> {
        if !self.inner.transports[0].supports_prefill32_program() {
            return Ok(false);
        }
        self.validate_prefill_profile_v1(true, true)?;
        if self.prefill32_pages_v1 != Some(true) || self.prefill16_ordered_v1.is_some() {
            return Err("native prefill32 requires explicit two-page selection".into());
        }
        Ok(self.prefill32_plan_v1(batch, output_rows)?.is_some())
    }
    /// Selects a same-image two-page prefill experiment after the nine-image KV composition.
    /// The control and candidate retain the independently selected V19 decode policy.
    /// # Errors
    /// Rejects unadmitted images, nonfresh or incompatible storage before changing policy.
    pub fn configure_prefill32_pages_v1(
        &mut self,
        copy: &EngineeringTpArtifactV1,
        enabled: bool,
    ) -> TpResult<()> {
        let binding = copy
            .prefill_kv_copy_binding_v27()
            .ok_or("prefill page pair requires V27")?;
        self.configure_prefill32_pages_binding_v1(binding, enabled)
    }

    pub(super) fn configure_prefill32_pages_binding_v1(
        &mut self,
        binding: PrefillKvCopyBindingV27,
        enabled: bool,
    ) -> TpResult<()> {
        let program = self.inner.transports.len() == 1
            && self.inner.transports[0].supports_prefill32_program();
        if program {
            self.validate_prefill_profile_v1(true, true)?;
        }
        if !cfg!(feature = "c1-ordered64")
            || cfg!(feature = "model-timestamps")
            || self.prefill32_pages_v1.is_some()
            || self.packed_gate_up_r2.is_some()
            || self.packed_down_r1.is_some()
            || self.last_batch != 0
            || self.completed_batches != 0
            || self.poisoned
            || self.inner.closed
            || self.inner.draft_v10
            || self.inner.large_kv
            || self.inner.plan.world_size() != 1
            || self.inner.plan.model().role != Qwen3ModelRole::Target8B
            || self.inner.ranks.len() != 1
            || self.inner.transports.len() != 1
            || self.row_capacity != 32
            || self.inner.row_capacity != 32
            || !(2..=512).contains(&self.physical_pages)
            || !(32..=8192).contains(&self.context_tokens)
            || self.admitted_prefill_kv_copy_v27 != Some(binding)
            || self.prefill_kv_copy_v28 != Some(true)
            || self.c1_split_attention_v25 != Some(true)
            || self.split_attention_workspace_v25.is_none()
            || self.partial_gemv_v28 != Some(false)
            || self.admitted_partial_gemv_v20.is_none()
            || (program && !enabled)
            || self.admitted_c1_kv_copy_v19.is_none()
            || self
                .c1_kv_copy_v19
                .is_some_and(|copy| Some(copy) != self.admitted_c1_kv_copy_v19)
            || self.c1_packet_packing_v22 != Some(true)
            || !self.inner.ordered_batch_width.is_wide()
            || !self.inner.transports[0].supports_ordered_batches64()
            || self.inner.transports[0].peer_group_rank().is_some()
            || self.inner.ranks[0].dispatches != 0
            || self.inner.sequences.is_some()
            || self.inner.packed_c1.is_some()
            || self.numerical.is_some()
            || self.inner.residual_arithmetic.is_some()
            || self
                .inner
                .ordered_batches
                .as_ref()
                .is_none_or(|pending| !pending.is_empty())
            || self.reduction_mode() != EngineeringTpReductionModeV3::DeviceTp1V3
            || !self.c1_wave_layers
            || !self.wave_attention
            || !self.prune_output_head
            || self.projection.mode != super::super::EngineeringTpProjectionModeV3::Mfma
            || !self.head_profile_configured
            || self.fp32_logits.is_none()
            || self.fp32_argmax_v11.is_none()
            || self.fp32_argmax_v11 != self.admitted_argmax_v11
            || self.query_hoist_v14.is_none()
            || self.query_hoist_v14 != self.admitted_query_hoist_v14
            || self.wave_rmsnorm_v15.is_none()
            || self.wave_rmsnorm_v15 != self.admitted_wave_rmsnorm_v15
        {
            return Err(
                "prefill page pair requires the fresh exact ordered64 nine-image KV composition"
                    .into(),
            );
        }
        let rank = &self.inner.ranks[0];
        if rank.geometry.kv_channels.count != 1024
            || [rank.k_rotated, rank.v]
                .iter()
                .any(|tensor| tensor.elements != 32 * 1024 || tensor.element_bytes != 2)
            || rank.layers.iter().any(|layer| {
                [layer.k_cache, layer.v_cache].iter().any(|tensor| {
                    tensor.elements != self.physical_pages as usize * HALF_ELEMENTS
                        || tensor.element_bytes != 2
                })
            })
        {
            return Err("prefill page pair allocation geometry differs".into());
        }
        for layer in &rank.layers {
            let ids = [
                rank.k_rotated.id,
                rank.v.id,
                layer.k_cache.id,
                layer.v_cache.id,
            ];
            if ids
                .iter()
                .enumerate()
                .any(|(index, id)| *id == 0 || ids[index + 1..].contains(id))
            {
                return Err("prefill page pair requires four distinct owned allocations".into());
            }
        }
        self.prefill32_pages_v1 = Some(enabled);
        Ok(())
    }

    pub(super) fn prefill32_plan_v1(
        &self,
        batch: &EngineeringTpPreparedBatchV1,
        output_rows: &[usize],
    ) -> TpResult<Option<PrefillPagePairV1>> {
        if self.prefill32_pages_v1 != Some(true) {
            return Ok(None);
        }
        let plan = PrefillPagePairV1::select(
            batch,
            output_rows,
            self.physical_pages,
            self.context_tokens,
        )?;
        if let Some(plan) = plan {
            for layer in 0..self.inner.ranks[0].layers.len() {
                plan.commands(&self.inner.ranks[0], layer)?;
            }
        }
        Ok(plan)
    }

    pub(super) fn execution_order_v1(
        &self,
        batch: &EngineeringTpPreparedBatchV1,
        output_rows: &[usize],
        pair: Option<PrefillPagePairV1>,
    ) -> Vec<usize> {
        if let Some(pair) = pair {
            return pair.execution_order();
        }
        let mut order = Vec::with_capacity(batch.rows().len());
        if self.prune_output_head {
            order.extend_from_slice(output_rows);
        }
        order.extend(
            (0..batch.rows().len())
                .filter(|row| !self.prune_output_head || output_rows.binary_search(row).is_err()),
        );
        order
    }

    /// Selection-aware preflight; legacy policies retain their count-based behavior.
    /// # Errors
    /// Rejects invalid selection, prepared ownership or either page view before submission.
    pub fn expected_dispatch_counts_for_selection(
        &self,
        batch: &EngineeringTpPreparedBatchV1,
        output_rows: &[usize],
    ) -> TpResult<Vec<u64>> {
        if self.prefill32_pages_v1 != Some(true) {
            return self.expected_dispatch_counts_for_batch(batch, output_rows.len());
        }
        self.validate(batch)?;
        if output_rows.iter().any(|row| *row >= batch.rows().len())
            || output_rows.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err("output rows must be unique, ascending, and within the batch".into());
        }
        if batch.rows().len() != 32 {
            return self.expected_dispatch_counts_for_batch(batch, output_rows.len());
        }
        let extra = if self.prefill32_plan_v1(batch, output_rows)?.is_some() {
            36
        } else {
            0
        };
        self.expected_dispatch_counts(output_rows.len())
            .into_iter()
            .map(|count| {
                count
                    .checked_add(extra)
                    .ok_or_else(|| "prefill page pair packet count overflow".to_owned())
            })
            .collect()
    }

    /// Explicit prefill policy; decode selection is reported independently.
    #[must_use]
    pub const fn prefill32_pages_mode(&self) -> &'static str {
        if matches!(self.prefill32_pages_v1, Some(true)) {
            "parallel-prefill32-two-pages-v27"
        } else {
            "baseline"
        }
    }
}
