//! Full-only bank-before-layer order; every late refusal or unwind is terminal.
use super::scoped;
use crate::finite_guarded_mlp_full2303_bank_scoped_census_v1::{BankCounts, CensusCounts, Counts};
use crate::finite_guarded_mlp_long_wire_v2::Profile;
use crate::state_roster::guarded_mlp_decode_v1::BankCompletion;
use fe2o3_kfd::Gfx950EngineeringPeerScopedCapacityCensusObservationV1 as Census;
use fe2o3_kfd::Gfx950EngineeringPeerScopedCurrentnessCountsV1 as Currentness;
use std::io;

pub(crate) struct State {
    layers: scoped::State,
    banks: BankCounts,
    census: CensusCounts,
    next_position: u32,
    next_layer: usize,
    bank_active: bool,
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
fn add(a: u64, b: u64) -> io::Result<u64> {
    a.checked_add(b)
        .ok_or_else(|| io::Error::other("bank scoped counter overflow"))
}
fn add_window(total: &mut BankCounts, c: Currentness) -> io::Result<()> {
    let calls = c
        .local_checkpoints
        .checked_mul(2)
        .and_then(|n| n.checked_add(4));
    let probes = c
        .local_checkpoints
        .checked_mul(2)
        .and_then(|n| n.checked_add(3));
    require(
        c.full_discoveries == 2
            && c.local_checkpoints > 0
            && calls == Some(c.before_calls)
            && c.before_calls == c.after_calls
            && probes == Some(c.generation_probes),
        "bank scoped observed window counters",
    )?;
    total.full_discoveries = add(total.full_discoveries, c.full_discoveries)?;
    total.local_checkpoints = add(total.local_checkpoints, c.local_checkpoints)?;
    total.before_calls = add(total.before_calls, c.before_calls)?;
    total.after_calls = add(total.after_calls, c.after_calls)?;
    total.generation_probes = add(total.generation_probes, c.generation_probes)?;
    Ok(())
}
fn add_subset(total: u32, value: u32) -> io::Result<u32> {
    total
        .checked_add(value)
        .ok_or_else(|| io::Error::other("census counter overflow"))
}
fn observe(total: &mut CensusCounts, c: Census, v: Currentness) -> io::Result<()> {
    require(
        c.preflights == 2
            && c.rank_checkpoints == 16
            && c.owner_counts.iter().all(|n| *n > 0 && *n <= 2048)
            && (total.warm_layers == 0 || total.owner_counts == c.owner_counts),
        "census returned subset/owner drift",
    )?;
    let local = v.local_checkpoints.checked_sub(16);
    let before = v.before_calls.checked_sub(16);
    let after = v.after_calls.checked_sub(16);
    let probes = v.generation_probes.checked_sub(32);
    require(
        match (local, before, after, probes) {
            (Some(l), Some(b), Some(a), Some(p)) => {
                l > 0
                    && b == a
                    && v.full_discoveries == 2
                    && l.checked_add(4).is_some_and(|n| b >= n)
                    && l.checked_mul(2)
                        .and_then(|n| n.checked_add(4))
                        .is_some_and(|n| b <= n)
                    && l.checked_mul(2).and_then(|n| n.checked_add(3)) == Some(p)
            }
            _ => false,
        },
        "census subset must already be in actual layer counters",
    )?;
    total.warm_layers = add_subset(total.warm_layers, 1)?;
    total.preflights = add_subset(total.preflights, c.preflights)?;
    total.rank_checkpoints = add_subset(total.rank_checkpoints, c.rank_checkpoints)?;
    total.owner_counts = c.owner_counts;
    Ok(())
}
impl State {
    pub(crate) fn new(profile: Profile) -> io::Result<Self> {
        Ok(Self {
            layers: scoped::State::new(profile)?,
            banks: BankCounts::default(),
            census: CensusCounts::default(),
            next_position: 0,
            next_layer: 0,
            bank_active: false,
            terminal: false,
        })
    }
    pub(crate) fn begin(
        &mut self,
        position: u32,
        call: impl FnOnce() -> io::Result<BankCompletion>,
    ) -> io::Result<()> {
        let mut attempt = Attempt {
            state: self,
            committed: false,
        };
        require(
            !attempt.state.terminal
                && !attempt.state.bank_active
                && position < 2303
                && position == attempt.state.next_position
                && attempt.state.next_layer == 0,
            "bank scoped exact forward begin order",
        )?;
        let output = call()?;
        let bank = position as usize % 2;
        require(
            output.generation == u64::from(position) / 2 + 1
                && attempt.state.banks.final_generations[bank].checked_add(1)
                    == Some(output.generation),
            "bank scoped actual bank-local generation",
        )?;
        let mut counts = attempt.state.banks.clone();
        match (position >= 2, output.currentness) {
            (false, None) => counts.ordinary_initial_banks += 1,
            (true, Some(c)) => {
                add_window(&mut counts, c)?;
                counts.scoped_rearms += 1;
            }
            _ => return Err(io::Error::other("bank scoped first-use/fallback refusal")),
        }
        counts.final_generations[bank] = output.generation;
        attempt.state.banks = counts;
        attempt.state.bank_active = true;
        attempt.committed = true;
        Ok(())
    }
    pub(crate) fn dispatch<O>(
        &mut self,
        position: u32,
        layer: usize,
        call: impl FnOnce(bool) -> io::Result<(O, Option<(Currentness, Census)>)>,
    ) -> io::Result<O> {
        let mut attempt = Attempt {
            state: self,
            committed: false,
        };
        require(
            !attempt.state.terminal
                && attempt.state.bank_active
                && position == attempt.state.next_position
                && layer < 36
                && layer == attempt.state.next_layer,
            "bank scoped exact bank/layer order",
        )?;
        let mut census = attempt.state.census.clone();
        let output = attempt.state.layers.dispatch(position, layer, |warm| {
            let (output, observed) = call(warm)?;
            let currentness = match (warm, observed) {
                (false, None) => None,
                (true, Some((v, c))) => {
                    observe(&mut census, c, v)?;
                    Some(v)
                }
                _ => return Err(io::Error::other("full census first-use/fallback refusal")),
            };
            Ok((output, currentness))
        })?;
        attempt.state.census = census;
        attempt.state.next_layer += 1;
        if attempt.state.next_layer == 36 {
            attempt.state.next_layer = 0;
            attempt.state.next_position += 1;
            attempt.state.bank_active = false;
        }
        attempt.committed = true;
        Ok(output)
    }
    pub(crate) fn closed_counts(&self) -> io::Result<Counts> {
        require(
            !self.terminal
                && !self.bank_active
                && self.next_position == 2303
                && self.next_layer == 0,
            "bank scoped incomplete/terminal Close",
        )?;
        let counts = Counts {
            layers: self.layers.closed_counts()?,
            banks: self.banks.clone(),
            census: self.census.clone(),
        };
        counts.validate_closed()?;
        Ok(counts)
    }
}

#[cfg(test)]
#[path = "native_guarded_mlp_full2303_bank_census_v1_tests.rs"]
mod tests;
