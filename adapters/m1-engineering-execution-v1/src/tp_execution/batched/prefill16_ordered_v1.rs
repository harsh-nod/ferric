//! Same-command prefill packing; only completed ordered groups commit host state.

use super::{
    EngineeringTpBatchExecutionV2, EngineeringTpPreparedBatchV1, EngineeringTpRankTransportV1,
    EngineeringTpReductionModeV3, Qwen3ModelRole, TpResult,
};

impl<R: EngineeringTpRankTransportV1> EngineeringTpBatchExecutionV2<R> {
    /// Selects a default-off, same-kernel prefill16 completion-group experiment.
    /// Configure after the ordinary ordered64 prefill/decode composition.
    /// # Errors
    /// Rejects repeated/late selection, changed profiles, token programs or capture.
    pub fn configure_prefill16_ordered_v1(&mut self, enabled: bool) -> TpResult<()> {
        if self.prefill16_ordered_v1.is_some()
            || self.last_batch != 0
            || self.completed_batches != 0
            || self.inner.ranks.iter().any(|rank| rank.dispatches != 0)
        {
            return Err("prefill16 scheduling requires fresh explicit selection".into());
        }
        self.validate_prefill16_ordered_profile_v1()?;
        self.prefill16_ordered_v1 = Some(enabled);
        Ok(())
    }

    /// Selected completion policy, not a kernel or performance qualification.
    #[must_use]
    pub const fn prefill16_ordered_mode_v1(&self) -> &'static str {
        if matches!(self.prefill16_ordered_v1, Some(true)) {
            "prefill16-ordered64-v1"
        } else {
            "residual-frontiers-v1"
        }
    }

    fn validate_prefill16_ordered_profile_v1(&self) -> TpResult<()> {
        self.validate_prefill16_profile_v1(false)
    }

    fn validate_prefill16_profile_v1(&self, program: bool) -> TpResult<()> {
        self.validate_prefill_profile_v1(program, false)
    }

    pub(super) fn validate_prefill_profile_v1(&self, program: bool, width32: bool) -> TpResult<()> {
        if !cfg!(feature = "c1-ordered64")
            || cfg!(feature = "model-timestamps")
            || self.poisoned
            || self.inner.closed
            || self.row_capacity != 32
            || self.inner.row_capacity != 32
            || self.inner.plan.world_size() != 1
            || self.inner.plan.model().role != Qwen3ModelRole::Target8B
            || self.inner.plan.model().layers != 36
            || self.inner.plan.model().hidden_size != 4096
            || self.inner.ranks.len() != 1
            || self.inner.transports.len() != 1
            || self.inner.draft_v10
            || self.inner.large_kv
            || self.inner.sequences.is_some()
            || self.inner.packed_c1.is_some()
            || !self.inner.ordered_batch_width.is_wide()
            || self
                .inner
                .ordered_batches
                .as_ref()
                .is_none_or(|pending| !pending.is_empty())
            || self.inner.residual_arithmetic.is_some()
            || self.reduction_mode() != EngineeringTpReductionModeV3::DeviceTp1V3
            || self.numerical.is_some()
            || !self.c1_wave_layers
            || !self.wave_attention
            || !self.prune_output_head
            || !self.head_profile_configured
            || self.projection.mode != super::super::EngineeringTpProjectionModeV3::Mfma
            || self.fp32_logits.is_none()
            || self.fp32_argmax_v11.is_none()
            || self.fp32_argmax_v11 != self.admitted_argmax_v11
            || self.query_hoist_v14.is_none()
            || self.query_hoist_v14 != self.admitted_query_hoist_v14
            || self.wave_rmsnorm_v15.is_none()
            || self.wave_rmsnorm_v15 != self.admitted_wave_rmsnorm_v15
            || self.c1_packet_packing_v22 != Some(true)
            || self.c1_split_attention_v25 != Some(true)
            || self.split_attention_workspace_v25.is_none()
            || self.prefill_kv_copy_v28 != Some(true)
            || self.admitted_prefill_kv_copy_v27.is_none()
            || (!width32 && self.prefill32_pages_v1.is_some())
            || (width32 && (!program || !self.inner.transports[0].supports_prefill32_program()))
            || self.partial_gemv_v28 == Some(true)
            || self.packed_gate_up_r2.is_some()
            || self.packed_down_r1.is_some()
            || if program {
                self.admitted_c1_kv_copy_v19.is_none()
                    || self.c1_kv_copy_v19 != self.admitted_c1_kv_copy_v19
            } else {
                self.c1_kv_copy_v19.is_some() || self.admitted_c1_kv_copy_v19.is_some()
            }
            || !self.inner.transports[0].supports_ordered_batches()
            || !self.inner.transports[0].supports_ordered_batches64()
            || self.inner.transports[0].supports_token_program() != program
            || (program && !self.inner.transports[0].supports_prefill_program())
            || self.inner.transports[0].peer_group_rank().is_some()
        {
            return Err("prefill16 scheduling requires the ordinary baseline-GEMV TP1 ordered64 composition".into());
        }
        Ok(())
    }

    pub(super) fn prefill16_program_active_v1(
        &self,
        batch: &EngineeringTpPreparedBatchV1,
        output_rows: &[usize],
    ) -> TpResult<bool> {
        if !self.inner.transports[0].supports_prefill_program() {
            return Ok(false);
        }
        if self.inner.transports[0].supports_prefill32_program() {
            return Ok(false);
        }
        if !cfg!(all(
            feature = "c1-token-program",
            not(feature = "model-timestamps")
        )) || self.prefill16_ordered_v1.is_some()
        {
            return Err("native prefill program excludes other prefill scheduling".into());
        }
        self.validate_prefill16_profile_v1(true)?;
        if batch.rows().len() != 16 || !(output_rows.is_empty() || output_rows == [15]) {
            return Ok(false);
        }
        let order = self.execution_order_v1(batch, output_rows, None);
        Ok(self.prepare_prefill_copy_v28(batch, &order)?.is_some())
    }

    pub(super) fn prefill16_ordered_active_v1(
        &self,
        batch: &EngineeringTpPreparedBatchV1,
        output_rows: &[usize],
    ) -> TpResult<bool> {
        if self.prefill16_ordered_v1 != Some(true) {
            return Ok(false);
        }
        self.validate_prefill16_ordered_profile_v1()?;
        if batch.rows().len() != 16 || !(output_rows.is_empty() || output_rows == [15]) {
            return Ok(false);
        }
        // Reuse the exact page, row permutation and prepared-write ownership checks.
        let order = self.execution_order_v1(batch, output_rows, None);
        Ok(self.prepare_prefill_copy_v28(batch, &order)?.is_some())
    }
}
