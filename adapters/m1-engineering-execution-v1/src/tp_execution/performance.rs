//! Bounded IPC sequences preserve rank-local dependencies and collective barriers.

use super::{EngineeringTpExecutionV1, EngineeringTpRankTransportV1, TpResult};

impl<R: EngineeringTpRankTransportV1> EngineeringTpExecutionV1<R> {
    pub(super) fn flush_sequences(&mut self) -> TpResult<()> {
        let Some(pending) = &mut self.sequences else {
            return Ok(());
        };
        if pending.iter().all(Vec::is_empty) {
            return Ok(());
        }
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
