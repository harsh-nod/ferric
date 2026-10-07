//! Bank-before-layer ordering for the separate bank-scoped Position5 route.
use super::scoped;
use crate::finite_guarded_mlp_long_wire_v2::Profile;
use crate::finite_guarded_mlp_readiness_bank_scoped_v2::{BankCounts, Counts};
use crate::state_roster::guarded_mlp_decode_v1::BankCompletion;
use fe2o3_kfd::Gfx950EngineeringPeerScopedCurrentnessCountsV1 as Currentness;
use std::io;

pub(super) struct State {
    layers: scoped::State,
    banks: BankCounts,
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
impl State {
    pub(super) fn new(profile: Profile) -> io::Result<Self> {
        Ok(Self {
            layers: scoped::State::new(profile, false)?,
            banks: BankCounts {
                scoped_rearms_by_forward: vec![0; 40],
                ..BankCounts::default()
            },
            next_position: 0,
            next_layer: 0,
            bank_active: false,
            terminal: false,
        })
    }
    pub(super) fn begin(
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
                && position < 40
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
                counts.scoped_rearms_by_forward[position as usize] += 1;
            }
            _ => return Err(io::Error::other("bank scoped first-use/fallback refusal")),
        }
        counts.final_generations[bank] = output.generation;
        attempt.state.banks = counts;
        attempt.state.bank_active = true;
        attempt.committed = true;
        Ok(())
    }
    pub(super) fn dispatch<O>(
        &mut self,
        position: u32,
        layer: usize,
        call: impl FnOnce(bool) -> io::Result<(O, Option<Currentness>)>,
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
        let output = attempt.state.layers.dispatch(position, layer, call)?;
        attempt.state.next_layer += 1;
        if attempt.state.next_layer == 36 {
            attempt.state.next_layer = 0;
            attempt.state.next_position += 1;
            attempt.state.bank_active = false;
        }
        attempt.committed = true;
        Ok(output)
    }
    pub(super) fn closed_counts(&self) -> io::Result<Counts> {
        require(
            !self.terminal && !self.bank_active && self.next_position == 40 && self.next_layer == 0,
            "bank scoped incomplete/terminal Close",
        )?;
        let counts = Counts {
            layers: self.layers.closed_counts()?,
            banks: self.banks.clone(),
        };
        counts.validate_closed()?;
        Ok(counts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    fn window(bank: bool) -> Currentness {
        Currentness {
            full_discoveries: 2,
            local_checkpoints: 5,
            before_calls: if bank { 14 } else { 11 },
            after_calls: if bank { 14 } else { 11 },
            generation_probes: 13,
        }
    }
    fn begin(state: &mut State, position: u32) {
        state
            .begin(position, || {
                Ok(BankCompletion {
                    generation: position as u64 / 2 + 1,
                    currentness: (position >= 2).then(|| window(true)),
                })
            })
            .unwrap();
    }
    fn layers(state: &mut State, position: u32) {
        for layer in 0..36 {
            state
                .dispatch(position, layer, |warm| {
                    assert_eq!(warm, position >= 2);
                    Ok(((), warm.then(|| window(false))))
                })
                .unwrap();
        }
    }
    #[test]
    fn bank_scoped_route_runs_40_ordered_banks_with_separate_layer_counters() {
        let mut state = State::new(Profile::Readiness40Position5).unwrap();
        for position in 0..40 {
            begin(&mut state, position);
            layers(&mut state, position);
        }
        let counts = state.closed_counts().unwrap();
        assert_eq!(counts.banks.ordinary_initial_banks, 2);
        assert_eq!(counts.banks.scoped_rearms, 38);
        assert_eq!(counts.banks.final_generations, [20, 20]);
        assert_eq!(counts.banks.full_discoveries, 76);
        assert_eq!(counts.layers.full_discoveries, 2736);
        assert_eq!(counts.banks.before_calls, 14 * 38);
        assert_eq!(counts.layers.before_calls, 11 * 1368);
    }
    #[test]
    fn bank_scoped_route_refuses_early_layer_duplicate_begin_and_other_profiles() {
        for p in [Profile::Readiness40, Profile::Full2303] {
            assert!(State::new(p).is_err());
        }
        let mut state = State::new(Profile::Readiness40Position5).unwrap();
        assert!(
            state
                .dispatch(0, 0, |_| -> io::Result<((), Option<Currentness>)> {
                    panic!("layer before bank");
                })
                .is_err()
        );
        assert!(state.terminal);
        let mut state = State::new(Profile::Readiness40Position5).unwrap();
        begin(&mut state, 0);
        assert!(state.begin(0, || panic!("duplicate bank")).is_err());
        assert!(state.terminal);
        let mut state = State::new(Profile::Readiness40Position5).unwrap();
        begin(&mut state, 0);
        assert!(
            state
                .begin(1, || panic!("bank before completed layers"))
                .is_err()
        );
        assert!(state.closed_counts().is_err());
    }
    #[test]
    fn bank_scoped_route_refuses_generation_counters_and_warm_fallback_after_runtime() {
        for mutation in 0..8 {
            let mut state = State::new(Profile::Readiness40Position5).unwrap();
            for p in 0..2 {
                begin(&mut state, p);
                layers(&mut state, p);
            }
            let mut returned = false;
            assert!(
                state
                    .begin(2, || {
                        returned = true;
                        let mut c = window(true);
                        match mutation {
                            0 => c.full_discoveries = 1,
                            1 => c.local_checkpoints = 0,
                            2 => c.before_calls = 11,
                            3 => c.after_calls += 1,
                            4 => c.generation_probes += 1,
                            5 => c.local_checkpoints = u64::MAX,
                            _ => (),
                        }
                        Ok(BankCompletion {
                            generation: if mutation == 6 { 3 } else { 2 },
                            currentness: (mutation != 7).then_some(c),
                        })
                    })
                    .is_err()
            );
            assert!(returned && state.terminal);
            assert_eq!(state.banks.scoped_rearms, 0);
            assert!(state.begin(2, || panic!("invalid result retry")).is_err());
        }
    }
    #[test]
    fn bank_scoped_route_preserves_refusal_unwind_and_incomplete_close() {
        for stage in 0..3 {
            let mut state = State::new(Profile::Readiness40Position5).unwrap();
            assert!(state.closed_counts().is_err());
            if stage == 0 {
                let e = state
                    .begin(0, || Err(io::Error::other("bank runtime refusal")))
                    .unwrap_err();
                assert_eq!(e.to_string(), "bank runtime refusal");
            } else if stage == 1 {
                assert!(
                    catch_unwind(AssertUnwindSafe(|| {
                        state.begin(0, || panic!("bank unwind")).ok();
                    }))
                    .is_err()
                );
            } else {
                begin(&mut state, 0);
                assert!(
                    catch_unwind(AssertUnwindSafe(|| {
                        state
                            .dispatch(0, 0, |_| -> io::Result<((), Option<Currentness>)> {
                                panic!("layer unwind");
                            })
                            .ok();
                    }))
                    .is_err()
                );
            }
            assert!(state.terminal && state.closed_counts().is_err());
        }
    }
}
