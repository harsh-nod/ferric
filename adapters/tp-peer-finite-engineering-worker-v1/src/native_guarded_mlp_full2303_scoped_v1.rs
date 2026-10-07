//! Private ordered route ledger; any late rejection or unwind is terminal.
use crate::finite_guarded_mlp_full2303_scoped_v1::Counts;
use crate::finite_guarded_mlp_long_wire_v2::Profile;
use fe2o3_kfd::Gfx950EngineeringPeerScopedCurrentnessCountsV1 as Currentness;
use std::io;

pub(crate) struct State {
    next: usize,
    terminal: bool,
    counts: Counts,
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
fn checked_add(left: u64, right: u64) -> io::Result<u64> {
    left.checked_add(right)
        .ok_or_else(|| io::Error::other("scoped warm counter overflow"))
}
fn add_counts(total: &mut Counts, observed: Currentness) -> io::Result<()> {
    let lower = observed.local_checkpoints.checked_add(4);
    let upper = observed
        .local_checkpoints
        .checked_mul(2)
        .and_then(|v| v.checked_add(4));
    let probes = observed
        .local_checkpoints
        .checked_mul(2)
        .and_then(|v| v.checked_add(3));
    require(
        observed.full_discoveries == 2
            && observed.local_checkpoints > 0
            && observed.before_calls == observed.after_calls
            && lower.is_some_and(|n| observed.before_calls >= n)
            && upper.is_some_and(|n| observed.before_calls <= n)
            && probes == Some(observed.generation_probes),
        "scoped warm returned currentness counts",
    )?;
    total.full_discoveries = checked_add(total.full_discoveries, observed.full_discoveries)?;
    total.local_checkpoints = checked_add(total.local_checkpoints, observed.local_checkpoints)?;
    total.before_calls = checked_add(total.before_calls, observed.before_calls)?;
    total.after_calls = checked_add(total.after_calls, observed.after_calls)?;
    total.generation_probes = checked_add(total.generation_probes, observed.generation_probes)?;
    Ok(())
}
impl State {
    pub(crate) fn new(profile: Profile) -> io::Result<Self> {
        require(
            profile == Profile::Full2303,
            "scoped warm only explicit Full2303",
        )?;
        Ok(Self {
            next: 0,
            terminal: false,
            counts: Counts::default(),
        })
    }
    pub(crate) fn dispatch<O>(
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
                && position < 2303
                && layer < 36
                && attempt.state.next == position as usize * 36 + layer,
            "scoped warm exact forward/layer order",
        )?;
        let warm = position >= 2;
        let (output, currentness) = call(warm)?;
        let mut counts = attempt.state.counts.clone();
        match (warm, currentness) {
            (false, None) => counts.ordinary_layers += 1,
            (true, Some(observed)) => {
                add_counts(&mut counts, observed)?;
                counts.scoped_layers += 1;
            }
            _ => {
                return Err(io::Error::other(
                    "scoped warm route cannot fall back or replace first use",
                ));
            }
        }
        attempt.state.counts = counts;
        attempt.state.next += 1;
        attempt.committed = true;
        Ok(output)
    }
    pub(crate) fn closed_counts(&self) -> io::Result<Counts> {
        require(
            !self.terminal && self.next == 2303 * 36,
            "scoped warm incomplete or terminal route",
        )?;
        self.counts.validate_closed()?;
        Ok(self.counts.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    fn counts() -> Currentness {
        Currentness {
            full_discoveries: 2,
            local_checkpoints: 5,
            before_calls: 11,
            after_calls: 11,
            generation_probes: 13,
        }
    }
    fn first_uses(state: &mut State) {
        for position in 0..2 {
            for layer in 0..36 {
                state
                    .dispatch(position, layer, |warm| {
                        assert!(!warm);
                        Ok(((), None))
                    })
                    .unwrap();
            }
        }
    }
    #[test]
    fn scoped_route_calls_every_layer_with_two_ordinary_first_uses_and_no_fallback() {
        let mut state = State::new(Profile::Full2303).unwrap();
        let mut ordinary = 0;
        let mut scoped = 0;
        for position in 0..2303 {
            for layer in 0..36 {
                let result = state
                    .dispatch(position, layer, |warm| {
                        assert_eq!(warm, position >= 2);
                        if warm {
                            scoped += 1;
                        } else {
                            ordinary += 1;
                        }
                        Ok(((position, layer), warm.then(counts)))
                    })
                    .unwrap();
                assert_eq!(result, (position, layer));
            }
        }
        assert_eq!((ordinary, scoped), (72, 82_836));
        let closed = state.closed_counts().unwrap();
        assert_eq!(closed.full_discoveries, 165_672);
        assert_eq!(closed.local_checkpoints, 5 * 82_836);
    }
    #[test]
    fn scoped_route_rejects_wrong_profile_causal_order_and_incomplete_close() {
        assert!(State::new(Profile::Readiness40).is_err());
        assert!(State::new(Profile::Readiness40Position5).is_err());
        for (position, layer) in [(1, 0), (0, 1), (2303, 0), (0, 36)] {
            let mut state = State::new(Profile::Full2303).unwrap();
            assert!(state.closed_counts().is_err());
            assert!(
                state
                    .dispatch(
                        position,
                        layer,
                        |_| -> io::Result<((), Option<Currentness>)> {
                            panic!("wrong order must not execute");
                        }
                    )
                    .is_err()
            );
            assert!(state.terminal);
            assert_eq!(state.next, 0);
        }
    }
    #[test]
    fn scoped_route_rejects_first_use_scoped_and_warm_ordinary_results_fatally() {
        let mut state = State::new(Profile::Full2303).unwrap();
        assert!(state.dispatch(0, 0, |_| Ok(((), Some(counts())))).is_err());
        assert!(state.terminal);
        let mut state = State::new(Profile::Full2303).unwrap();
        first_uses(&mut state);
        assert!(state.dispatch(2, 0, |_| Ok(((), None))).is_err());
        assert!(state.terminal);
        assert_eq!(state.next, 72);
    }
    #[test]
    fn scoped_route_post_runtime_completion_counter_rejection_cannot_retry_or_commit() {
        for mutation in 0..6 {
            let mut state = State::new(Profile::Full2303).unwrap();
            first_uses(&mut state);
            let mut returned = false;
            assert!(
                state
                    .dispatch(2, 0, |warm| {
                        assert!(warm);
                        returned = true;
                        let mut c = counts();
                        match mutation {
                            0 => c.full_discoveries = 1,
                            1 => c.local_checkpoints = 0,
                            2 => c.after_calls += 1,
                            3 => c.before_calls = 99,
                            4 => c.generation_probes += 1,
                            _ => c.local_checkpoints = u64::MAX,
                        }
                        Ok(((), Some(c)))
                    })
                    .is_err()
            );
            assert!(returned && state.terminal);
            assert_eq!(state.next, 72);
            assert_eq!(state.counts.scoped_layers, 0);
            assert!(state.closed_counts().is_err());
            assert!(
                state
                    .dispatch(2, 0, |_| -> io::Result<((), Option<Currentness>)> {
                        panic!("post-runtime worker refusal must not retry");
                    })
                    .is_err()
            );
        }
    }
    #[test]
    fn scoped_route_runtime_error_and_unwind_keep_original_failure_terminal() {
        let mut state = State::new(Profile::Full2303).unwrap();
        first_uses(&mut state);
        let error = state
            .dispatch(2, 0, |_| -> io::Result<((), Option<Currentness>)> {
                Err(io::Error::other("runtime exit failed"))
            })
            .unwrap_err();
        assert_eq!(error.to_string(), "runtime exit failed");
        assert!(state.terminal);
        assert_eq!(state.next, 72);
        let mut state = State::new(Profile::Full2303).unwrap();
        first_uses(&mut state);
        assert!(
            catch_unwind(AssertUnwindSafe(|| {
                let _: io::Result<()> = state.dispatch(2, 0, |_| {
                    panic!("runtime or later worker unwind");
                });
            }))
            .is_err()
        );
        assert!(state.terminal);
        assert_eq!(state.next, 72);
    }
}
