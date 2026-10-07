//! Readiness-only closed tail ordering around the unchanged V3 census engine.
use super::census_scoped;
use crate::finite_guarded_mlp_long_wire_v2::Profile;
use crate::finite_guarded_mlp_readiness_bank_scoped_census_tail_v4::{Counts, TailCounts};
use crate::state_roster::guarded_mlp_decode_v1::BankCompletion;
use fe2o3_kfd::{
    Gfx950EngineeringPeerScopedCapacityCensusObservationV1 as Census,
    Gfx950EngineeringPeerScopedCurrentnessCountsV1 as Currentness,
};
use std::io;

pub(super) struct State {
    census: census_scoped::State,
    tails: TailCounts,
    next: u32,
    layer: usize,
    active: bool,
    terminal: bool,
}
struct Attempt<'a> {
    state: &'a mut State,
    committed: bool,
}
impl Drop for Attempt<'_> {
    fn drop(&mut self) {
        if !self.committed {
            self.state.terminal = true;
        }
    }
}
fn require(ok: bool, why: &str) -> io::Result<()> {
    if ok {
        Ok(())
    } else {
        Err(io::Error::other(why))
    }
}
fn observe(total: &mut TailCounts, warm: bool, value: Option<Currentness>) -> io::Result<()> {
    let add32 = |a: u32, b: u32| {
        a.checked_add(b)
            .ok_or_else(|| io::Error::other("tail u32 overflow"))
    };
    let add64 = |a: u64, b: u64| {
        a.checked_add(b)
            .ok_or_else(|| io::Error::other("tail u64 overflow"))
    };
    match (warm, value) {
        (false, None) => total.ordinary_tails = add32(total.ordinary_tails, 1)?,
        (true, Some(v)) => {
            let l = v.local_checkpoints;
            require(
                v.full_discoveries == 2
                    && l >= 27
                    && l.checked_add(16) == Some(v.before_calls)
                    && v.before_calls == v.after_calls
                    && l.checked_mul(2).and_then(|n| n.checked_add(3)) == Some(v.generation_probes),
                "tail returned currentness census",
            )?;
            total.scoped_tails = add32(total.scoped_tails, 1)?;
            total.dispatches = add32(total.dispatches, 3)?;
            total.readbacks = add32(total.readbacks, 3)?;
            total.readback_bytes = add64(total.readback_bytes, 4 + 8192 + 303872)?;
            total.full_discoveries = add64(total.full_discoveries, v.full_discoveries)?;
            total.local_checkpoints = add64(total.local_checkpoints, l)?;
            total.before_calls = add64(total.before_calls, v.before_calls)?;
            total.after_calls = add64(total.after_calls, v.after_calls)?;
            total.generation_probes = add64(total.generation_probes, v.generation_probes)?;
        }
        _ => return Err(io::Error::other("tail first-use/fallback refusal")),
    }
    Ok(())
}
impl State {
    pub(super) fn new(profile: Profile) -> io::Result<Self> {
        Ok(Self {
            census: census_scoped::State::new(profile)?,
            tails: TailCounts::default(),
            next: 0,
            layer: 0,
            active: false,
            terminal: false,
        })
    }
    pub(super) fn begin(
        &mut self,
        position: u32,
        call: impl FnOnce() -> io::Result<BankCompletion>,
    ) -> io::Result<()> {
        let mut a = Attempt {
            state: self,
            committed: false,
        };
        require(
            !a.state.terminal && !a.state.active && position == a.state.next && position < 40,
            "tail bank ordering or terminal",
        )?;
        a.state.census.begin(position, call)?;
        a.state.active = true;
        a.state.layer = 0;
        a.committed = true;
        Ok(())
    }
    pub(super) fn dispatch<O>(
        &mut self,
        position: u32,
        layer: usize,
        call: impl FnOnce(bool) -> io::Result<(O, Option<(Currentness, Census)>)>,
    ) -> io::Result<O> {
        let mut a = Attempt {
            state: self,
            committed: false,
        };
        require(
            !a.state.terminal
                && a.state.active
                && position == a.state.next
                && layer == a.state.layer
                && layer < 36,
            "tail layer ordering or terminal",
        )?;
        let result = a.state.census.dispatch(position, layer, call)?;
        a.state.layer += 1;
        a.committed = true;
        Ok(result)
    }
    pub(super) fn tail<O>(
        &mut self,
        position: u32,
        call: impl FnOnce(bool) -> io::Result<(O, Option<Currentness>)>,
    ) -> io::Result<O> {
        let mut a = Attempt {
            state: self,
            committed: false,
        };
        require(
            !a.state.terminal && a.state.active && position == a.state.next && a.state.layer == 36,
            "tail completion ordering or terminal",
        )?;
        let warm = position >= 2;
        let (result, counters) = call(warm)?;
        let mut next = a.state.tails.clone();
        observe(&mut next, warm, counters)?;
        a.state.tails = next;
        a.state.next += 1;
        a.state.active = false;
        a.committed = true;
        Ok(result)
    }
    pub(super) fn closed_counts(&self) -> io::Result<Counts> {
        require(
            !self.terminal && !self.active && self.next == 40,
            "tail requires forty accepted completions before Close",
        )?;
        let c = self.census.closed_counts()?;
        let counts = Counts {
            layers: c.layers,
            banks: c.banks,
            census: c.census,
            tails: self.tails.clone(),
        };
        counts.validate_closed()?;
        Ok(counts)
    }
}
#[cfg(test)]
#[path = "native_guarded_mlp_readiness_tail_v4_tests.rs"]
mod tests;
