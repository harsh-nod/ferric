//! Exact `Target8B` recording with a deferred host-state commit.
//!
//! Rank clones contain only buffer identifiers and weight-layout metadata.
//! Allocation, mapping, transport and lifetime owners are never cloned.

use super::{
    EngineeringTpArgumentV1, EngineeringTpDispatchV1, EngineeringTpExecutionV1,
    EngineeringTpRankTransportV1, Qwen3ModelRole, Qwen3TensorParallelCollectiveStateV1,
    Qwen3TensorParallelCollectiveV1, Rank, ReductionWorkspace, Tensor, TpResult, dispatch,
    reduction::DEVICE_RESIDUAL, row_profile,
};

pub(super) const DISPATCHES: usize = 616;
const LAYERS_U32: u32 = 36;
const HIDDEN_U32: u32 = 4096;
const LAYERS: usize = LAYERS_U32 as usize;
const HIDDEN: usize = HIDDEN_U32 as usize;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Recording,
    Submitted,
    Completed,
}

pub(super) struct FullForwardRecording {
    pub(super) rank: Rank,
    scratch: Tensor,
    collective: Qwen3TensorParallelCollectiveStateV1,
    commands: Vec<EngineeringTpDispatchV1>,
    host_hidden: Vec<u16>,
    initial_hidden: Tensor,
    initial_scratch: Tensor,
    initial_collective: Qwen3TensorParallelCollectiveStateV1,
    initial_dispatches: u64,
    initial_host_elements: usize,
    next_dispatches: u64,
    residuals: usize,
    phase: Phase,
}

impl FullForwardRecording {
    fn push(&mut self, command: EngineeringTpDispatchV1) -> TpResult<()> {
        if self.phase != Phase::Recording || self.commands.len() >= DISPATCHES {
            return Err("full-forward command count or recording phase".into());
        }
        self.commands.push(command);
        Ok(())
    }

    fn seal(&mut self) -> TpResult<()> {
        let expected = self.collective.expected();
        if self.phase != Phase::Recording
            || self.commands.len() != DISPATCHES
            || self.residuals != LAYERS * 2
            || expected.epoch
                != self
                    .initial_collective
                    .expected()
                    .epoch
                    .checked_add(1)
                    .ok_or("full-forward collective epoch overflow")?
            || expected.layer != 0
            || expected.operation != Qwen3TensorParallelCollectiveV1::AttentionOutputSum
            || self.rank.hidden != self.initial_hidden
            || self.scratch != self.initial_scratch
        {
            return Err("incomplete full-forward recording or final binding parity".into());
        }
        self.phase = Phase::Submitted;
        Ok(())
    }
}

impl<R: EngineeringTpRankTransportV1> EngineeringTpExecutionV1<R> {
    pub(super) fn binding_rank_zero(&self) -> &Rank {
        self.full_forward
            .as_ref()
            .map_or(&self.ranks[0], |plan| &plan.rank)
    }

    pub(super) fn planned_collective(&self) -> Qwen3TensorParallelCollectiveStateV1 {
        self.full_forward
            .as_ref()
            .map_or(self.collective, |plan| plan.collective)
    }

    pub(super) fn begin_full_forward(&mut self, rows: u32) -> TpResult<()> {
        if !self.full_forward_enabled {
            return Ok(());
        }
        let ReductionWorkspace::DeviceTp1(scratch) = self.reduction else {
            return Err("full-forward device residual workspace unavailable".into());
        };
        if rows != 1
            || self.closed
            || self.full_forward.is_some()
            || self.ranks.len() != 1
            || self.transports.len() != 1
            || !self.transports[0].supports_full_forward()
            || self.transports[0].supports_queue_rollover()
            || self.ordered_batches.is_some()
            || self.sequences.is_some()
            || self.large_kv
            || self.draft_v10
            || self.row_capacity != 16
            || self.plan.model().role != Qwen3ModelRole::Target8B
            || self.plan.model().layers != LAYERS_U32
            || self.plan.model().hidden_size != HIDDEN_U32
            || self.plan.world_size() != 1
            || scratch.id == self.ranks[0].hidden.id
            || scratch.elements < HIDDEN
            || self.ranks[0].hidden.elements < HIDDEN
            || scratch.element_bytes != 2
            || self.ranks[0].hidden.element_bytes != 2
        {
            return Err("full-forward recording profile or ownership drifted".into());
        }
        let rank = &self.ranks[0];
        let next_dispatches = rank
            .dispatches
            .checked_add(DISPATCHES as u64)
            .ok_or("full-forward dispatch counter overflow")?;
        self.full_forward = Some(FullForwardRecording {
            rank: rank.clone(),
            scratch,
            collective: self.collective,
            commands: Vec::with_capacity(DISPATCHES),
            host_hidden: vec![0; HIDDEN],
            initial_hidden: rank.hidden,
            initial_scratch: scratch,
            initial_collective: self.collective,
            initial_dispatches: rank.dispatches,
            initial_host_elements: self.hidden.len(),
            next_dispatches,
            residuals: 0,
            phase: Phase::Recording,
        });
        Ok(())
    }

    pub(super) fn record_full_forward_command(
        &mut self,
        command: EngineeringTpDispatchV1,
    ) -> TpResult<()> {
        let command =
            row_profile::bind_mode(self.draft_v10, self.row_capacity, self.large_kv, command)?;
        self.full_forward
            .as_mut()
            .ok_or("full-forward recording is absent")?
            .push(command)
    }

    pub(super) fn record_full_forward_residual(
        &mut self,
        layer: u32,
        operation: Qwen3TensorParallelCollectiveV1,
    ) -> TpResult<()> {
        let plan = self
            .full_forward
            .as_mut()
            .ok_or("full-forward residual recording is absent")?;
        let key = plan.collective.expected();
        let preceding = match operation {
            Qwen3TensorParallelCollectiveV1::AttentionOutputSum => 10,
            Qwen3TensorParallelCollectiveV1::FeedForwardDownSum => 16,
        };
        if layer as usize >= LAYERS
            || key.layer != layer
            || key.operation != operation
            || plan.commands.len() != 1 + layer as usize * 17 + preceding
            || plan.phase != Phase::Recording
            || plan.rank.hidden.id == plan.scratch.id
            || plan.rank.partial.id == plan.scratch.id
            || plan.rank.partial.id == plan.rank.hidden.id
            || plan.rank.hidden.elements < HIDDEN
            || plan.scratch.elements < HIDDEN
            || plan.rank.partial.elements < HIDDEN
            || plan.rank.partial.element_bytes != 4
        {
            return Err("full-forward residual order, extent or ownership drifted".into());
        }
        plan.push(dispatch(
            DEVICE_RESIDUAL,
            HIDDEN_U32 / 64,
            vec![
                Tensor {
                    elements: HIDDEN,
                    ..plan.rank.partial
                }
                .read(),
                Tensor {
                    elements: HIDDEN,
                    ..plan.rank.hidden
                }
                .read(),
                Tensor {
                    elements: HIDDEN,
                    ..plan.scratch
                }
                .write(),
                EngineeringTpArgumentV1::U32(1),
            ],
        ))?;
        plan.collective
            .arrive(0, key)
            .map_err(|error| format!("planned collective arrival: {error:?}"))?;
        plan.collective
            .advance()
            .map_err(|error| format!("planned collective advance: {error:?}"))?;
        std::mem::swap(&mut plan.rank.hidden, &mut plan.scratch);
        plan.residuals += 1;
        Ok(())
    }

    fn validate_full_forward_live_state(&self) -> TpResult<()> {
        let plan = self
            .full_forward
            .as_ref()
            .ok_or("full-forward recording is absent")?;
        let ReductionWorkspace::DeviceTp1(scratch) = self.reduction else {
            return Err("full-forward live workspace changed".into());
        };
        if !self.full_forward_enabled
            || self.closed
            || self.ranks.len() != 1
            || self.transports.len() != 1
            || self.ranks[0].hidden != plan.initial_hidden
            || scratch != plan.initial_scratch
            || self.collective != plan.initial_collective
            || self.ranks[0].dispatches != plan.initial_dispatches
            || self.hidden.len() != plan.initial_host_elements
        {
            return Err("full-forward live state changed before commit".into());
        }
        Ok(())
    }

    pub(super) fn finish_full_forward(&mut self) -> TpResult<()> {
        if !self.full_forward_enabled {
            return Ok(());
        }
        self.validate_full_forward_live_state()?;
        let plan = self
            .full_forward
            .as_mut()
            .ok_or("full-forward recording is absent")?;
        plan.seal()?;
        self.transports[0].submit_full_forward(&plan.commands)?;
        self.transports[0].wait_full_forward(DISPATCHES)?;
        plan.phase = Phase::Completed;
        Ok(())
    }

    /// Choice readback and vocabulary validation must precede this infallible commit tail.
    pub(super) fn commit_full_forward(&mut self) -> TpResult<()> {
        if !self.full_forward_enabled {
            return Ok(());
        }
        self.validate_full_forward_live_state()?;
        if self
            .full_forward
            .as_ref()
            .is_none_or(|plan| plan.phase != Phase::Completed)
        {
            return Err("full-forward commit requires exact completed execution".into());
        }
        let plan = self
            .full_forward
            .take()
            .expect("validated full-forward completion");
        self.ranks[0].hidden = plan.rank.hidden;
        self.reduction = ReductionWorkspace::DeviceTp1(plan.scratch);
        self.collective = plan.collective;
        self.ranks[0].dispatches = plan.next_dispatches;
        self.hidden = plan.host_hidden;
        Ok(())
    }
}
