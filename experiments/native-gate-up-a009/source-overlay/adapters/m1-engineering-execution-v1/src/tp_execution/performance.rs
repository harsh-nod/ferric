//! Bounded IPC sequences preserve rank-local dependencies and collective barriers.

use super::reduction::{
    AttentionProducerSchedule, FeedForwardProducerSchedule, ReductionWorkspace,
};
use super::{
    EngineeringTpDispatchV1, EngineeringTpExecutionV1, EngineeringTpRankTransportV1,
    OrderedBatchWidth, Qwen3TensorParallelCollectiveStateV1, Tensor, TpResult,
};

#[derive(Clone, Copy)]
pub(super) struct PackedC1State {
    pub(super) collective: Qwen3TensorParallelCollectiveStateV1,
    pub(super) hidden: Tensor,
    pub(super) scratch: Tensor,
    pub(super) producer_packets: usize,
    pub(super) attention_schedule: AttentionProducerSchedule,
    pub(super) feed_forward_schedule: FeedForwardProducerSchedule,
    pub(super) prefill_program: bool,
    pub(super) prefill_rows: usize,
}

impl<R: EngineeringTpRankTransportV1> EngineeringTpExecutionV1<R> {
    fn token_program_selected(&self) -> bool {
        #[cfg(all(feature = "c1-token-program", not(feature = "model-timestamps")))]
        {
            self.ordered_batch_width.is_wide()
                && self.transports.len() == 1
                && self.transports[0].supports_token_program()
                && self.packed_c1.is_some_and(|state| {
                    state.attention_schedule == AttentionProducerSchedule::Split8V21
                        && state.feed_forward_schedule.native_packets().is_some()
                })
        }
        #[cfg(not(all(feature = "c1-token-program", not(feature = "model-timestamps"))))]
        {
            let _ = self;
            false
        }
    }

    fn program_packets(&self) -> Option<usize> {
        if self.packed_c1.is_some_and(|state| state.prefill_program) {
            Some(
                if self.packed_c1.is_some_and(|state| state.prefill_rows == 32) {
                    649
                } else {
                    613
                },
            )
        } else if self.token_program_selected() {
            self.packed_c1
                .and_then(|state| state.feed_forward_schedule.native_packets())
        } else {
            None
        }
    }

    #[cfg(feature = "tp-batch-engineering")]
    pub(super) fn begin_prefill_program(&mut self) -> TpResult<()> {
        self.begin_prefill_program_rows(16)
    }

    #[cfg(feature = "tp-batch-engineering")]
    pub(super) fn begin_prefill_program_rows(&mut self, rows: usize) -> TpResult<()> {
        if !cfg!(all(
            feature = "c1-token-program",
            not(feature = "model-timestamps")
        )) || self.transports.len() != 1
            || !self.transports[0].supports_prefill_program()
            || !matches!(rows, 16 | 32)
            || (rows == 32 && !self.transports[0].supports_prefill32_program())
        {
            return Err("prefill program requires its explicit native transport".into());
        }
        self.begin_packed_tp1(
            rows,
            if rows == 32 {
                AttentionProducerSchedule::PrefillPagePairV1
            } else {
                AttentionProducerSchedule::Baseline
            },
            FeedForwardProducerSchedule::Baseline,
        )?;
        self.packed_c1
            .as_mut()
            .expect("new prefill scope")
            .prefill_program = true;
        self.packed_c1
            .as_mut()
            .expect("new prefill scope")
            .prefill_rows = rows;
        Ok(())
    }

    #[cfg(feature = "tp-batch-engineering")]
    pub(super) fn begin_packed_c1(
        &mut self,
        attention_schedule: AttentionProducerSchedule,
    ) -> TpResult<()> {
        self.begin_packed_c1_with_ffn(attention_schedule, FeedForwardProducerSchedule::Baseline)
    }

    #[cfg(feature = "tp-batch-engineering")]
    pub(super) fn begin_packed_c1_with_ffn(
        &mut self,
        attention_schedule: AttentionProducerSchedule,
        feed_forward_schedule: FeedForwardProducerSchedule,
    ) -> TpResult<()> {
        self.begin_packed_tp1(1, attention_schedule, feed_forward_schedule)
    }

    #[cfg(feature = "tp-batch-engineering")]
    pub(super) fn begin_packed_prefill16(&mut self) -> TpResult<()> {
        if !cfg!(feature = "c1-ordered64")
            || cfg!(feature = "model-timestamps")
            || !self.ordered_batch_width.is_wide()
            || self.transports.len() != 1
            || !self.transports[0].supports_ordered_batches64()
            || self.transports[0].supports_token_program()
        {
            return Err("prefill16 packing requires ordinary ordered64 without timestamps".into());
        }
        self.begin_packed_tp1(
            16,
            AttentionProducerSchedule::Baseline,
            FeedForwardProducerSchedule::Baseline,
        )
    }

    #[cfg(feature = "tp-batch-engineering")]
    fn begin_packed_tp1(
        &mut self,
        rows: usize,
        attention_schedule: AttentionProducerSchedule,
        feed_forward_schedule: FeedForwardProducerSchedule,
    ) -> TpResult<()> {
        let ReductionWorkspace::DeviceTp1(scratch) = self.reduction else {
            return Err("packed C1 requires device TP1 residual storage".into());
        };
        if self.packed_c1.is_some()
            || self.closed
            || self.ranks.len() != 1
            || self.transports.len() != 1
            || self.plan.world_size() != 1
            || self.plan.model().layers != 36
            || self.plan.model().hidden_size != 4096
            || self.draft_v10
            || self.large_kv
            || self.row_capacity != 32
            || self.hidden.len() != rows * 4096
            || self.sequences.is_some()
            || self
                .ordered_batches
                .as_ref()
                .is_none_or(|pending| !pending.is_empty())
            || !self.transports[0].supports_ordered_batches()
            || (self.transports[0].supports_token_program()
                && (!self.ordered_batch_width.is_wide()
                    || !matches!(
                        feed_forward_schedule,
                        FeedForwardProducerSchedule::Baseline
                            | FeedForwardProducerSchedule::SplitKGateUpR1
                    )))
            || (feed_forward_schedule == FeedForwardProducerSchedule::SplitKGateUpR1
                && (rows != 1
                    || attention_schedule != AttentionProducerSchedule::Split8V21
                    || !self.transports[0].supports_prefill32_program()))
        {
            return Err("packed TP1 requires a fresh bounded forward".into());
        }
        self.packed_c1 = Some(PackedC1State {
            collective: self.collective,
            hidden: self.ranks[0].hidden,
            scratch,
            producer_packets: 0,
            attention_schedule,
            feed_forward_schedule,
            prefill_program: false,
            prefill_rows: 0,
        });
        Ok(())
    }

    pub(super) fn enqueue_packed_c1(&mut self, command: EngineeringTpDispatchV1) -> TpResult<()> {
        let program = self.program_packets();
        let limit = program.unwrap_or_else(|| self.ordered_batch_width.bound());
        if self.packed_c1.is_none()
            || self
                .ordered_batches
                .as_ref()
                .is_none_or(|pending| pending.len() > limit)
        {
            return Err("packed C1 queue state drifted".into());
        }
        // Flush before the next append, so a residual's staged swap joins its packet.
        if self.ordered_batches.as_ref().expect("packed queue").len() == limit {
            if program.is_some() {
                return Err(
                    "fixed token program exceeded its complete graph before publication".into(),
                );
            }
            self.flush_ordered_batches()?;
        }
        self.ordered_batches
            .as_mut()
            .expect("packed queue")
            .push(command);
        Ok(())
    }

    #[cfg(feature = "tp-batch-engineering")]
    pub(super) fn finish_packed_c1(&mut self) -> TpResult<()> {
        if self
            .packed_c1
            .is_some_and(|state| state.producer_packets != 0)
        {
            return Err("packed C1 ended with an incomplete producer segment".into());
        }
        if self.packed_c1.is_some() {
            self.flush_ordered_batches()?;
            self.packed_c1 = None;
        }
        Ok(())
    }

    pub(super) fn discard_packed_c1(&mut self) {
        if self.packed_c1.take().is_some()
            && let Some(pending) = &mut self.ordered_batches
        {
            pending.clear();
        }
    }

    pub(super) fn flush_dispatch_groups(&mut self) -> TpResult<()> {
        self.flush_ordered_batches()?;
        self.flush_sequences()
    }

    fn flush_ordered_batches(&mut self) -> TpResult<()> {
        let program = self.program_packets();
        let prefill = self.packed_c1.is_some_and(|state| state.prefill_program);
        let limit = if let Some(count) = program {
            count
        } else if self.packed_c1.is_some() {
            self.ordered_batch_width.bound()
        } else {
            OrderedBatchWidth::Packets16.bound()
        };
        let Some(pending) = &mut self.ordered_batches else {
            return Ok(());
        };
        if pending.is_empty() {
            return Ok(());
        }
        let _timing = self.timing.span("flush_ordered_batches", None);
        let count = pending.len();
        if !(1..=limit).contains(&count)
            || program.is_some_and(|expected| count != expected)
            || self.ranks.len() != 1
            || self.transports.len() != 1
            || self.sequences.is_some()
            || (self.ordered_batch_width.is_wide()
                && !self.transports[0].supports_ordered_batches64())
            || self.ranks[0].dispatches.checked_add(count as u64).is_none()
        {
            return Err("ordered dispatch batch geometry/count overflow".into());
        }
        let result = if prefill {
            self.transports[0]
                .submit_prefill_program(pending)
                .and_then(|()| self.transports[0].wait_prefill_program(count))
        } else if program.is_some() {
            self.transports[0]
                .submit_token_program(pending)
                .and_then(|()| self.transports[0].wait_token_program(count))
        } else {
            self.transports[0]
                .submit_ordered_batch(pending)
                .and_then(|()| self.transports[0].wait_ordered_batch(count))
        };
        pending.clear();
        result?;
        self.ranks[0].dispatches += count as u64;
        if let Some(state) = self.packed_c1 {
            // Commit host metadata only after the entire ordered group completed.
            self.collective = state.collective;
            self.ranks[0].hidden = state.hidden;
            self.reduction = ReductionWorkspace::DeviceTp1(state.scratch);
        }
        Ok(())
    }

    pub(super) fn flush_sequences(&mut self) -> TpResult<()> {
        let Some(pending) = &mut self.sequences else {
            return Ok(());
        };
        if pending.iter().all(Vec::is_empty) {
            return Ok(());
        }
        let _timing = self.timing.span("flush_sequences", None);
        let count = pending[0].len();
        if !(1..=16).contains(&count)
            || pending.len() != self.ranks.len()
            || pending.iter().any(|rank| rank.len() != count)
            || self
                .ranks
                .iter()
                .any(|rank| rank.dispatches.checked_add(count as u64).is_none())
        {
            return Err("rank sequence geometry/count overflow".into());
        }
        let mut submitted = 0;
        let mut error = None;
        for (transport, commands) in self.transports.iter_mut().zip(pending.iter()) {
            match transport.submit_sequence(commands) {
                Ok(()) => submitted += 1,
                Err(message) => {
                    error = Some(message);
                    break;
                }
            }
        }
        // Submit every admitted rank before waiting; drain submitted responses
        // even when another rank has failed.
        for index in 0..submitted {
            match self.transports[index].wait_sequence(count) {
                Ok(()) => self.ranks[index].dispatches += count as u64,
                Err(message) => {
                    if error.is_none() {
                        error = Some(message);
                    }
                }
            }
        }
        for commands in pending {
            commands.clear();
        }
        error.map_or(Ok(()), Err)
    }
}
