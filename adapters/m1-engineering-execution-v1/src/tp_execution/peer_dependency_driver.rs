//! One closed TP2 collective: defer only its two final partial projections.
//! Arithmetic and ordinary rank-local submission remain unchanged.

use super::peer_dependency::EngineeringTp2CollectiveRequestV1;
use super::{
    EngineeringTpDispatchV1, EngineeringTpExecutionV1, EngineeringTpRankTransportV1,
    EngineeringTpReductionModeV3, Qwen3TensorParallelCollectiveV1, Rank, ReductionWorkspace,
    TpResult,
};

impl<R: EngineeringTpRankTransportV1> EngineeringTpExecutionV1<R> {
    pub(super) fn check_peer_dependency_profile(&self) -> TpResult<()> {
        self.check_peer_dependency_source_profile()?;
        if self.transports.iter().any(|rank| {
            !rank.supports_peer_dependency_collectives()
                || rank.supports_concurrent_rounds()
                || rank.supports_queue_rollover()
        }) {
            return Err("peer dependency collective requires the explicit TP2/capacity16 target profile without batching, concurrent rounds or rollover".into());
        }
        Ok(())
    }

    /// Owner/geometry checks shared with inert recording; no dispatch capability.
    pub(super) fn check_peer_dependency_source_profile(&self) -> TpResult<()> {
        if self.ranks.len() != 2 || self.transports.len() != 2 {
            return Err("peer dependency collective requires exactly two retained ranks".into());
        }
        self.check_peer_group()?;
        if self.closed
            || self.plan.world_size() != 2
            || self.row_capacity != 16
            || self.plan.model().layers != 36
            || self.plan.model().intermediate_size != 12_288
            || self.large_kv
            || self.draft_v10
            || self.sequences.is_some()
            || self.ordered_batches.is_some()
            || self.full_forward_enabled
            || self.full_forward.is_some()
        {
            return Err("peer dependency collective requires the explicit TP2/capacity16 target profile without batching, concurrent rounds or rollover".into());
        }
        Ok(())
    }

    #[cfg_attr(not(feature = "tp-batch-engineering"), allow(dead_code))]
    pub(super) fn dispatch_collective_producers(
        &mut self,
        layer: u32,
        operation: Qwen3TensorParallelCollectiveV1,
        command: impl Fn(&Rank) -> EngineeringTpDispatchV1,
    ) -> TpResult<()> {
        if self.reduction.mode() != EngineeringTpReductionModeV3::DevicePeerDependencyV1 {
            return self.dispatch_each(command);
        }
        self.check_peer_dependency_profile()?;
        if self.peer_dependency_pending.is_some() || self.hidden.len() != 4096 {
            return Err(
                "peer dependency producers require one active row and no pending collective".into(),
            );
        }
        let request = EngineeringTp2CollectiveRequestV1 {
            key: self.collective.expected(),
            rows: 1,
            producers: [command(&self.ranks[0]), command(&self.ranks[1])],
            consumers: self
                .peer_reduction_commands(layer, operation)?
                .try_into()
                .map_err(|_| "peer dependency consumer cardinality")?,
        };
        if request
            .producers
            .iter()
            .any(|producer| producer.kernel != "ferric_qwen3_tp_mfma_gemm_partial_f32_v3")
        {
            return Err(
                "peer dependency collective admits only its explicit MFMA partial profile".into(),
            );
        }
        request.validate()?;
        // Finish earlier rank-local prerequisites before deferring the partials.
        // Neither producer is published until reduce submits the whole transaction.
        self.flush_dispatch_groups()?;
        self.peer_dependency_pending = Some(request);
        Ok(())
    }

    pub(super) fn reduce_peer_dependency(
        &mut self,
        layer: u32,
        operation: Qwen3TensorParallelCollectiveV1,
    ) -> TpResult<()> {
        self.check_peer_dependency_profile()?;
        let request = self
            .peer_dependency_pending
            .as_ref()
            .ok_or("peer dependency collective has no retained producer pair")?
            .clone();
        if request.key != self.collective.expected()
            || request.key.layer != layer
            || request.key.operation != operation
            || self.hidden.len() != 4096
            || request.consumers.as_slice()
                != self.peer_reduction_commands(layer, operation)?.as_slice()
        {
            return Err("peer dependency collective key or live tensor bindings changed".into());
        }
        request.validate()?;
        let completed = [
            self.ranks[0].dispatches.checked_add(2),
            self.ranks[1].dispatches.checked_add(2),
        ];
        let [Some(first), Some(second)] = completed else {
            return Err("peer dependency kernel dispatch counter overflow".into());
        };
        // Prepare every fallible host transition before publication. The live
        // cursor, counters and ping-pong buffers change together after acceptance.
        let mut collective = self.collective;
        for rank in 0..2 {
            collective
                .arrive(rank, request.key)
                .map_err(|error| format!("peer dependency arrival preflight: {error:?}"))?;
        }
        collective
            .advance()
            .map_err(|error| format!("peer dependency advance preflight: {error:?}"))?;
        let result = self.transports[0]
            .execute_peer_dependency_collective(&request)
            .and_then(|receipt| receipt.validate_for(&request));
        if let Err(error) = result {
            return Err(match self.close() {
                Ok(()) => error,
                Err(close) => format!("{error}; peer dependency close: {close}"),
            });
        }
        let ReductionWorkspace::DevicePeer(scratch, _) = &mut self.reduction else {
            unreachable!("workspace was validated before synchronous publication");
        };
        for (rank, output) in self.ranks.iter_mut().zip(scratch) {
            std::mem::swap(&mut rank.hidden, output);
        }
        self.ranks[0].dispatches = first;
        self.ranks[1].dispatches = second;
        self.collective = collective;
        self.peer_dependency_pending = None;
        Ok(())
    }
}
