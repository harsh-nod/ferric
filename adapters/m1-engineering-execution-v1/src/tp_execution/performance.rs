//! Bounded IPC sequences preserve rank-local dependencies and collective barriers.

use super::{EngineeringTpExecutionV1, EngineeringTpRankTransportV1, TpResult};

impl<R: EngineeringTpRankTransportV1> EngineeringTpExecutionV1<R> {
    pub(super) fn flush_dispatch_groups(&mut self) -> TpResult<()> {
        if self.full_forward_enabled {
            return Err("full-forward recording cannot use an intermediate dispatch flush".into());
        }
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
