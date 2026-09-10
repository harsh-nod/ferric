//! Ordered TP2/8 device reductions. Host tensor staging is never a fallback.

use super::reduction::ReductionWorkspace;
use super::{
    EngineeringTpArgumentV1, EngineeringTpDispatchV1, EngineeringTpExecutionV1,
    EngineeringTpRankTransportV1, Qwen3TensorParallelCollectiveV1, Tensor, TpResult, dispatch,
};

const REDUCE: &str = "ferric_qwen3_tp_peer_ordered_residual_bf16_v4";
const COPY: &str = "ferric_qwen3_tp_peer_copy_bf16_v4";

impl<R: EngineeringTpRankTransportV1> EngineeringTpExecutionV1<R> {
    fn check_peer_group(&self) -> TpResult<()> {
        let world = self.plan.world_size();
        let Some((pid, 0, reported_world)) = self.transports[0].peer_group_rank() else {
            return Err("peer reduction requires an explicit rank-zero group owner".into());
        };
        if !matches!(world, 2 | 8)
            || reported_world != world
            || pid == 0
            || self.plan.model().role != ferric_spec::Qwen3ModelRole::Target8B
            || self.plan.model().hidden_size != 4096
            || self.transports.iter().enumerate().any(|(rank, transport)| {
                u32::try_from(rank)
                    .ok()
                    .is_none_or(|rank| transport.peer_group_rank() != Some((pid, rank, world)))
            })
        {
            return Err("peer reduction requires exact Qwen3-8B TP2/8 in one owned group".into());
        }
        Ok(())
    }

    pub(super) fn configure_device_peer(&mut self) -> TpResult<()> {
        self.check_peer_group()?;
        let result = (|| {
            let mut scratch = Vec::with_capacity(self.ranks.len());
            for (rank, transport) in self.ranks.iter_mut().zip(&mut self.transports) {
                // Replace only before the first dispatch. Old private scratch remains
                // owned until child close; no initialized value is discarded here.
                rank.partial = shared_tensor(transport, rank.partial.elements, 4)?;
                rank.hidden = shared_tensor(transport, rank.hidden.elements, 2)?;
                scratch.push(shared_tensor(transport, rank.hidden.elements, 2)?);
            }
            let ids = self
                .ranks
                .iter()
                .flat_map(|rank| [rank.partial.id, rank.hidden.id])
                .chain(scratch.iter().map(|tensor| tensor.id))
                .collect::<std::collections::BTreeSet<_>>();
            if ids.len() != self.ranks.len() * 3 {
                return Err("peer workspace identities alias".into());
            }
            self.reduction = ReductionWorkspace::DevicePeer(scratch);
            Ok(())
        })();
        if let Err(error) = result {
            return Err(match self.close() {
                Ok(()) => error,
                Err(close) => format!("{error}; close: {close}"),
            });
        }
        Ok(())
    }

    fn peer_geometry(&self) -> TpResult<(usize, u32)> {
        self.check_peer_group()?;
        let elements = self.hidden.len();
        let rows = elements / 4096;
        if elements != rows * 4096 || !(1..=16).contains(&rows) {
            return Err("peer active row extent is outside the v4 contract".into());
        }
        Ok((
            elements,
            u32::try_from(rows).map_err(|_| "peer row count overflow")?,
        ))
    }

    pub(super) fn initialize_peer_hidden(&mut self) -> TpResult<()> {
        self.flush_sequences()?;
        let (elements, rows) = self.peer_geometry()?;
        let source = Tensor {
            elements,
            ..self.ranks[0].hidden
        };
        if elements > self.ranks[0].hidden.elements {
            return Err("peer embedding source extent drift".into());
        }
        let commands = self
            .ranks
            .iter()
            .skip(1)
            .map(|rank| {
                if elements > rank.hidden.elements || rank.hidden.id == source.id {
                    return Err("peer embedding destination extent or identity drift".into());
                }
                Ok(dispatch(
                    COPY,
                    u32::try_from(elements / 64).map_err(|_| "peer copy grid overflow")?,
                    vec![
                        source.read(),
                        Tensor {
                            elements,
                            ..rank.hidden
                        }
                        .write(),
                        EngineeringTpArgumentV1::U32(rows),
                    ],
                ))
            })
            .collect::<TpResult<Vec<_>>>()?;
        // Rank zero's embedding dispatch has completed before any peer reads it.
        self.dispatch_peer_commands(1, &commands)
    }

    pub(super) fn reduce_device_peer(
        &mut self,
        layer: u32,
        operation: Qwen3TensorParallelCollectiveV1,
    ) -> TpResult<()> {
        let (elements, rows) = self.peer_geometry()?;
        let key = self.collective.expected();
        if key.layer != layer || key.operation != operation {
            return Err("peer collective reordered".into());
        }
        let ReductionWorkspace::DevicePeer(scratch) = &self.reduction else {
            return Err("peer workspace unavailable".into());
        };
        if scratch.len() != self.ranks.len() {
            return Err("peer scratch roster drift".into());
        }
        let partials = self
            .ranks
            .iter()
            .map(|rank| {
                if elements > rank.partial.elements {
                    return Err("peer partial extent drift".into());
                }
                Ok(Tensor {
                    elements,
                    ..rank.partial
                })
            })
            .collect::<TpResult<Vec<_>>>()?;
        let commands = self
            .ranks
            .iter()
            .zip(scratch)
            .map(|(rank, &output)| {
                if elements > rank.hidden.elements
                    || elements > output.elements
                    || output.id == rank.hidden.id
                    || partials.iter().any(|p| p.id == output.id)
                {
                    return Err("peer output extent or identity drift".into());
                }
                let mut arguments = (0..8)
                    .map(|index| {
                        partials
                            .get(index)
                            .copied()
                            .unwrap_or(Tensor {
                                elements: 0,
                                ..partials[0]
                            })
                            .read()
                    })
                    .collect::<Vec<_>>();
                arguments.extend([
                    Tensor {
                        elements,
                        ..rank.hidden
                    }
                    .read(),
                    Tensor { elements, ..output }.write(),
                    EngineeringTpArgumentV1::U32(rows),
                    EngineeringTpArgumentV1::U32(self.plan.world_size()),
                ]);
                Ok(dispatch(
                    REDUCE,
                    u32::try_from(elements / 64).map_err(|_| "peer reduction grid overflow")?,
                    arguments,
                ))
            })
            .collect::<TpResult<Vec<_>>>()?;
        self.dispatch_peer_commands(0, &commands)?;
        for rank in &self.ranks {
            self.collective
                .arrive(rank.geometry.rank, key)
                .map_err(|e| format!("peer collective arrival: {e:?}"))?;
        }
        self.collective
            .advance()
            .map_err(|e| format!("peer collective advance: {e:?}"))?;
        let ReductionWorkspace::DevicePeer(scratch) = &mut self.reduction else {
            return Err("peer workspace disappeared".into());
        };
        // No rank publishes the new hidden state until every destination completed.
        for (rank, output) in self.ranks.iter_mut().zip(scratch) {
            std::mem::swap(&mut rank.hidden, output);
        }
        Ok(())
    }

    fn dispatch_peer_commands(
        &mut self,
        first_rank: usize,
        commands: &[EngineeringTpDispatchV1],
    ) -> TpResult<()> {
        if first_rank + commands.len() != self.ranks.len()
            || self.ranks[first_rank..]
                .iter()
                .any(|rank| rank.dispatches == u64::MAX)
        {
            return Err("peer dispatch roster or counter bound".into());
        }
        let mut submitted = 0;
        let mut failure = None;
        for (transport, command) in self.transports[first_rank..].iter_mut().zip(commands) {
            match transport.submit(command) {
                Ok(()) => submitted += 1,
                Err(error) => {
                    failure = Some(error);
                    break;
                }
            }
        }
        for rank in first_rank..first_rank + submitted {
            match self.transports[rank].wait() {
                Ok(()) => self.ranks[rank].dispatches += 1,
                Err(error) => {
                    if failure.is_none() {
                        failure = Some(error);
                    }
                }
            }
        }
        failure.map_or(Ok(()), Err)
    }
}

fn shared_tensor<R: EngineeringTpRankTransportV1>(
    transport: &mut R,
    elements: usize,
    element_bytes: u32,
) -> TpResult<Tensor> {
    let bytes = elements
        .checked_mul(element_bytes as usize)
        .ok_or("peer tensor extent overflow")?;
    Ok(Tensor {
        id: transport.allocate_peer_readable(bytes)?,
        elements,
        element_bytes,
    })
}

#[cfg(test)]
mod tests;
