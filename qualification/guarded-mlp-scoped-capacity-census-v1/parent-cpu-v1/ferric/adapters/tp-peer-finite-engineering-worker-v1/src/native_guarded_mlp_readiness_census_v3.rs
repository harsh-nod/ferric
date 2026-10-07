//! Census subset accounting around the unchanged bank/layer ordering engine.
use super::bank_scoped;
use crate::finite_guarded_mlp_long_wire_v2::Profile;
use crate::finite_guarded_mlp_readiness_bank_scoped_census_v3::{CensusCounts, Counts};
use crate::state_roster::guarded_mlp_decode_v1::BankCompletion;
use fe2o3_kfd::{
    Gfx950EngineeringPeerScopedCapacityCensusObservationV1 as Census,
    Gfx950EngineeringPeerScopedCurrentnessCountsV1 as Currentness,
};
use std::io;
pub(super) struct State {
    bank: bank_scoped::State,
    census: CensusCounts,
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
fn add(total: u32, value: u32) -> io::Result<u32> {
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
    total.warm_layers = add(total.warm_layers, 1)?;
    total.preflights = add(total.preflights, c.preflights)?;
    total.rank_checkpoints = add(total.rank_checkpoints, c.rank_checkpoints)?;
    total.owner_counts = c.owner_counts;
    Ok(())
}
impl State {
    pub(super) fn new(profile: Profile) -> io::Result<Self> {
        Ok(Self {
            bank: bank_scoped::State::new(profile)?,
            census: CensusCounts::default(),
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
        require(!attempt.state.terminal, "census route terminal")?;
        attempt.state.bank.begin(position, call)?;
        attempt.committed = true;
        Ok(())
    }
    pub(super) fn dispatch<O>(
        &mut self,
        position: u32,
        layer: usize,
        call: impl FnOnce(bool) -> io::Result<(O, Option<(Currentness, Census)>)>,
    ) -> io::Result<O> {
        let mut attempt = Attempt {
            state: self,
            committed: false,
        };
        require(!attempt.state.terminal, "census route terminal")?;
        let mut census = attempt.state.census.clone();
        let output = attempt.state.bank.dispatch(position, layer, |warm| {
            let (output, observed) = call(warm)?;
            let currentness = match (warm, observed) {
                (false, None) => None,
                (true, Some((v, c))) => {
                    observe(&mut census, c, v)?;
                    Some(v)
                }
                _ => return Err(io::Error::other("census first-use/fallback refusal")),
            };
            Ok((output, currentness))
        })?;
        attempt.state.census = census;
        attempt.committed = true;
        Ok(output)
    }
    pub(super) fn closed_counts(&self) -> io::Result<Counts> {
        require(!self.terminal, "census route terminal Close")?;
        let bank = self.bank.closed_counts()?;
        let counts = Counts {
            layers: bank.layers,
            banks: bank.banks,
            census: self.census.clone(),
        };
        counts.validate_closed()?;
        Ok(counts)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    fn current(bank: bool) -> Currentness {
        if bank {
            Currentness {
                full_discoveries: 2,
                local_checkpoints: 5,
                before_calls: 14,
                after_calls: 14,
                generation_probes: 13,
            }
        } else {
            Currentness {
                full_discoveries: 2,
                local_checkpoints: 21,
                before_calls: 27,
                after_calls: 27,
                generation_probes: 45,
            }
        }
    }
    fn census() -> Census {
        Census {
            owner_counts: [787, 783],
            preflights: 2,
            rank_checkpoints: 16,
        }
    }
    fn begin(s: &mut State, p: u32) {
        s.begin(p, || {
            Ok(BankCompletion {
                generation: u64::from(p) / 2 + 1,
                currentness: (p >= 2).then(|| current(true)),
            })
        })
        .unwrap();
    }
    fn forward(s: &mut State, p: u32) {
        begin(s, p);
        for l in 0..36 {
            s.dispatch(p, l, |warm| {
                assert_eq!(warm, p >= 2);
                Ok(((), warm.then(|| (current(false), census()))))
            })
            .unwrap();
        }
    }
    #[test]
    fn census_state_runs_only_1368_warm_layers_and_keeps_nested_bank_totals() {
        let mut s = State::new(Profile::Readiness40Position5).unwrap();
        for p in 0..40 {
            forward(&mut s, p);
        }
        let c = s.closed_counts().unwrap();
        assert_eq!(c.census.warm_layers, 1368);
        assert_eq!(c.census.preflights, 2736);
        assert_eq!(c.census.rank_checkpoints, 21888);
        assert_eq!(c.census.owner_counts, [787, 783]);
        assert_eq!(c.layers.full_discoveries, 2736);
        assert_eq!(c.layers.local_checkpoints, 21 * 1368);
        assert_eq!(c.banks.scoped_rearms, 38);
        assert_eq!(c.banks.full_discoveries, 76);
    }
    #[test]
    fn census_state_rejects_fallback_and_counter_drift_after_runtime_return() {
        for mutation in 0..8 {
            let mut s = State::new(Profile::Readiness40Position5).unwrap();
            forward(&mut s, 0);
            forward(&mut s, 1);
            begin(&mut s, 2);
            let mut returned = false;
            assert!(
                s.dispatch(2, 0, |_| {
                    returned = true;
                    let mut c = census();
                    let mut v = current(false);
                    match mutation {
                        0 => c.preflights = 1,
                        1 => c.rank_checkpoints = 15,
                        2 => c.owner_counts[0] = 0,
                        3 => v.local_checkpoints = 16,
                        4 => v.before_calls = 0,
                        5 => v.after_calls += 1,
                        6 => v.generation_probes = u64::MAX,
                        _ => (),
                    }
                    Ok(((), (mutation != 7).then_some((v, c))))
                })
                .is_err()
            );
            assert!(returned && s.terminal && s.census.warm_layers == 0);
            assert!(
                s.dispatch(
                    2,
                    0,
                    |_| -> io::Result<((), Option<(Currentness, Census)>)> { panic!("no retry") }
                )
                .is_err()
            );
        }
    }
    #[test]
    fn census_state_refuses_owner_drift_order_other_profiles_and_unwind() {
        for p in [Profile::Readiness40, Profile::Full2303] {
            assert!(State::new(p).is_err());
        }
        let mut s = State::new(Profile::Readiness40Position5).unwrap();
        assert!(s.closed_counts().is_err());
        assert!(
            s.dispatch(
                0,
                0,
                |_| -> io::Result<((), Option<(Currentness, Census)>)> { panic!("before bank") }
            )
            .is_err()
        );
        let mut s = State::new(Profile::Readiness40Position5).unwrap();
        forward(&mut s, 0);
        forward(&mut s, 1);
        begin(&mut s, 2);
        s.dispatch(2, 0, |_| Ok(((), Some((current(false), census())))))
            .unwrap();
        let mut changed = census();
        changed.owner_counts.swap(0, 1);
        assert!(
            s.dispatch(2, 1, |_| Ok(((), Some((current(false), changed)))))
                .is_err()
        );
        assert!(s.terminal);
        let mut s = State::new(Profile::Readiness40Position5).unwrap();
        begin(&mut s, 0);
        assert!(
            catch_unwind(AssertUnwindSafe(|| {
                let _ = s.dispatch(
                    0,
                    0,
                    |_| -> io::Result<((), Option<(Currentness, Census)>)> {
                        panic!("native unwind")
                    },
                );
            }))
            .is_err()
        );
        assert!(s.terminal && s.closed_counts().is_err());
    }
    #[test]
    fn census_state_first_use_requires_no_census_and_checks_subset_overflow() {
        let mut s = State::new(Profile::Readiness40Position5).unwrap();
        begin(&mut s, 0);
        assert!(
            s.dispatch(0, 0, |_| Ok(((), Some((current(false), census())))))
                .is_err()
        );
        assert!(s.terminal);
        let mut c = CensusCounts {
            warm_layers: u32::MAX,
            owner_counts: [787, 783],
            ..CensusCounts::default()
        };
        assert!(observe(&mut c, census(), current(false)).is_err());
    }
}
