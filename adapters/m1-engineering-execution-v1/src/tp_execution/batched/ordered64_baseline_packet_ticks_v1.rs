//! Explicit eight-image baseline-KV packet attribution; the V19 route is separate.

use super::{EngineeringTpBatchExecutionV2, EngineeringTpRankTransportV1, TpResult};

impl<R: EngineeringTpRankTransportV1> EngineeringTpBatchExecutionV2<R> {
    /// Marks the already configured default composition for raw packet attribution.
    ///
    /// This changes no kernel, allocation, upload, command, grouping or wait policy.
    /// The existing V19 timestamp path keeps requiring its separately admitted image.
    /// # Errors
    /// Rejects incomplete selection, unmarked transports, V19, peers or late/repeated calls.
    pub fn configure_ordered64_baseline_packet_ticks_v1(&mut self) -> TpResult<()> {
        if self.baseline_packet_ticks_v1
            || self.last_batch != 0
            || self.completed_batches != 0
            || self.poisoned
            || self.inner.closed
            || self.inner.transports.len() != 1
            || self.row_capacity != 32
            || self.inner.row_capacity != 32
            || self.inner.plan.world_size() != 1
            || self.inner.plan.model().role != super::Qwen3ModelRole::Target8B
            || self.inner.plan.model().layers != 36
            || self.inner.plan.model().hidden_size != 4096
            || self.inner.ranks.len() != 1
            || self.inner.draft_v10
            || self.inner.large_kv
            || self.inner.sequences.is_some()
            || self.inner.packed_c1.is_some()
            || self.inner.residual_arithmetic.is_some()
            || self.numerical.is_some()
            || self.inner.timing.is_enabled()
            || self
                .inner
                .ordered_batches
                .as_ref()
                .is_none_or(|pending| !pending.is_empty())
            || !self.inner.ordered_batch_width.is_wide()
            || self.c1_packet_packing_v22 != Some(true)
            || self.c1_split_attention_v25 != Some(true)
            || self.prefill_kv_copy_v28 != Some(true)
            || self.partial_gemv_v28 != Some(false)
            || self.prefill32_pages_v1.is_some()
            || self.prefill16_ordered_v1.is_some()
            || self.packed_gate_up_r2.is_some()
            || self.packed_down_r1.is_some()
            || self.admitted_c1_kv_copy_v19.is_some()
            || self.c1_kv_copy_v19.is_some()
            || self.admitted_prefill_kv_copy_v27.is_none()
            || self.admitted_partial_gemv_v20.is_none()
            || self.split_attention_workspace_v25.is_none()
            || self.fp32_argmax_v11.is_none()
            || self.fp32_argmax_v11 != self.admitted_argmax_v11
            || self.query_hoist_v14.is_none()
            || self.query_hoist_v14 != self.admitted_query_hoist_v14
            || self.wave_rmsnorm_v15.is_none()
            || self.wave_rmsnorm_v15 != self.admitted_wave_rmsnorm_v15
            || !self.inner.transports[0].supports_ordered_batches64()
            || self.inner.transports[0].supports_token_program()
            || !self.inner.transports[0].model_timestamps_enabled()
            || !self.inner.transports[0].model_timestamp_ordered64_enabled()
            || self.inner.transports[0].peer_group_rank().is_some()
        {
            return Err("baseline packet ticks require a fresh explicitly selected eight-image timestamp transport".into());
        }
        self.baseline_packet_ticks_v1 = true;
        Ok(())
    }
}
