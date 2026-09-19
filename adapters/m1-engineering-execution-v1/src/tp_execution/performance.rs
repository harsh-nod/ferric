//! Bounded IPC sequences preserve rank-local dependencies and collective barriers.

use super::reduction::ReductionWorkspace;
use super::{
    EngineeringTpDispatchV1, EngineeringTpExecutionV1, EngineeringTpRankTransportV1,
    Qwen3TensorParallelCollectiveStateV1, Tensor, TpResult,
};

#[derive(Clone, Copy)]
pub(super) struct PackedC1State {
    pub(super) collective: Qwen3TensorParallelCollectiveStateV1,
    pub(super) hidden: Tensor,
    pub(super) scratch: Tensor,
    pub(super) producer_packets: usize,
}

impl<R: EngineeringTpRankTransportV1> EngineeringTpExecutionV1<R> {
    #[cfg(feature = "tp-batch-engineering")]
    pub(super) fn begin_packed_c1(&mut self) -> TpResult<()> {
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
            || self.hidden.len() != 4096
            || self.sequences.is_some()
            || self
                .ordered_batches
                .as_ref()
                .is_none_or(|pending| !pending.is_empty())
            || !self.transports[0].supports_ordered_batches()
        {
            return Err("packed C1 requires a fresh bounded one-row TP1 forward".into());
        }
        self.packed_c1 = Some(PackedC1State {
            collective: self.collective,
            hidden: self.ranks[0].hidden,
            scratch,
            producer_packets: 0,
        });
        Ok(())
    }

    pub(super) fn enqueue_packed_c1(&mut self, command: EngineeringTpDispatchV1) -> TpResult<()> {
        if self.packed_c1.is_none()
            || self
                .ordered_batches
                .as_ref()
                .is_none_or(|pending| pending.len() > 16)
        {
            return Err("packed C1 queue state drifted".into());
        }
        // Flush before the next append, so a residual's staged swap joins its packet.
        if self.ordered_batches.as_ref().expect("packed queue").len() == 16 {
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
        let Some(pending) = &mut self.ordered_batches else {
            return Ok(());
        };
        if pending.is_empty() {
            return Ok(());
        }
        let _timing = self.timing.span("flush_ordered_batches", None);
        let count = pending.len();
        if !(1..=16).contains(&count)
            || self.ranks.len() != 1
            || self.transports.len() != 1
            || self.sequences.is_some()
            || self.ranks[0].dispatches.checked_add(count as u64).is_none()
        {
            return Err("ordered dispatch batch geometry/count overflow".into());
        }
        let result = self.transports[0]
            .submit_ordered_batch(pending)
            .and_then(|()| self.transports[0].wait_ordered_batch(count));
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
