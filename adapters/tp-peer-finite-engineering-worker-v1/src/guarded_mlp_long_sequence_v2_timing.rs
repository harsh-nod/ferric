//! Fixed timestamps inside Sequence custody, never a deadline or execution policy.
use crate::finite_guarded_mlp_readiness_forward_durations_v1::ForwardRow;
use std::io;
use std::time::Instant;

pub(super) trait Clock {
    fn now(&mut self) -> io::Result<u128>;
}
#[derive(Default)]
pub(super) struct NativeClock {
    origin: Option<Instant>,
}
impl Clock for NativeClock {
    fn now(&mut self) -> io::Result<u128> {
        let now = Instant::now();
        let origin = *self.origin.get_or_insert(now);
        now.checked_duration_since(origin)
            .map(|elapsed| elapsed.as_nanos())
            .ok_or_else(|| io::Error::other("forward timing clock reversed"))
    }
}
pub(super) struct Timing {
    selected: bool,
    position: u32,
    stamps: [u128; 10],
    next: usize,
}
impl Timing {
    pub(super) fn new(selected: bool, position: u32) -> Self {
        Self {
            selected,
            position,
            stamps: [0; 10],
            next: 0,
        }
    }
    pub(super) fn mark(&mut self, clock: &mut impl Clock) -> io::Result<()> {
        if self.selected {
            let slot = self
                .stamps
                .get_mut(self.next)
                .ok_or_else(|| io::Error::other("forward timing timestamp count"))?;
            *slot = clock.now()?;
            self.next += 1;
        }
        Ok(())
    }
    pub(super) fn finish(self) -> io::Result<Option<ForwardRow>> {
        if !self.selected {
            return Ok(None);
        }
        if self.next != 10 {
            return Err(io::Error::other("forward timing incomplete timestamps"));
        }
        let elapsed = |start: u128, end: u128| {
            let ns = end
                .checked_sub(start)
                .ok_or_else(|| io::Error::other("forward timing clock reversed"))?;
            u64::try_from(ns).map_err(io::Error::other)
        };
        let mut phase_ns = [0; 9];
        for (index, value) in phase_ns.iter_mut().enumerate() {
            *value = elapsed(self.stamps[index], self.stamps[index + 1])?;
        }
        let row = ForwardRow {
            position: self.position,
            phase_ns,
            forward_body_ns: elapsed(self.stamps[0], self.stamps[9])?,
        };
        row.validate()?;
        Ok(Some(row))
    }
}
