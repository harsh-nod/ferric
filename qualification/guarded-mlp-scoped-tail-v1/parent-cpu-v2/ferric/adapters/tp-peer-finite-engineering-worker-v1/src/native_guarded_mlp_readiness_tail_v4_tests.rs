use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind};
fn current(local: u64, extra: u64) -> Currentness {
    Currentness {
        full_discoveries: 2,
        local_checkpoints: local,
        before_calls: local + extra,
        after_calls: local + extra,
        generation_probes: 2 * local + 3,
    }
}
fn layers(s: &mut State, p: u32) {
    s.begin(p, || {
        Ok(BankCompletion {
            generation: u64::from(p) / 2 + 1,
            currentness: (p >= 2).then(|| current(5, 9)),
        })
    })
    .unwrap();
    for layer in 0..36 {
        s.dispatch(p, layer, |warm| {
            assert_eq!(warm, p >= 2);
            Ok((
                (),
                warm.then(|| {
                    (
                        current(21, 6),
                        Census {
                            preflights: 2,
                            rank_checkpoints: 16,
                            owner_counts: [787, 783],
                        },
                    )
                }),
            ))
        })
        .unwrap();
    }
}
fn forward(s: &mut State, p: u32) {
    layers(s, p);
    s.tail(p, |warm| {
        assert_eq!(warm, p >= 2);
        Ok(((), warm.then(|| current(27 + u64::from(p), 16))))
    })
    .unwrap();
}
#[test]
fn tail_state_counts_actual_two_ordinary_and_38_separate_scopes() {
    let mut s = State::new(Profile::Readiness40Position5).unwrap();
    for p in 0..40 {
        forward(&mut s, p);
    }
    let c = s.closed_counts().unwrap();
    assert_eq!(c.tails.ordinary_tails, 2);
    assert_eq!(c.tails.scoped_tails, 38);
    assert_eq!(c.tails.dispatches, 114);
    assert_eq!(c.tails.readbacks, 114);
    assert_eq!(c.tails.readback_bytes, 11_858_584);
    assert_eq!(
        c.tails.local_checkpoints,
        (2..40).map(|p| 27 + p).sum::<u64>()
    );
    assert_eq!(c.layers.local_checkpoints, 21 * 1368);
    assert_eq!(c.banks.scoped_rearms, 38);
    assert_eq!(c.census.rank_checkpoints, 21888);
    assert!(s.begin(40, || panic!("after Close extent")).is_err());
}
#[test]
fn tail_state_rejects_missing_malformed_return_and_overflow_without_retry() {
    for m in 0..16 {
        let mut s = State::new(Profile::Readiness40Position5).unwrap();
        forward(&mut s, 0);
        forward(&mut s, 1);
        layers(&mut s, 2);
        match m {
            6 => s.tails.scoped_tails = u32::MAX,
            7 => s.tails.dispatches = u32::MAX,
            8 => s.tails.readbacks = u32::MAX,
            9 => s.tails.readback_bytes = u64::MAX,
            10 => s.tails.generation_probes = u64::MAX,
            12 => s.tails.full_discoveries = u64::MAX,
            13 => s.tails.local_checkpoints = u64::MAX,
            14 => s.tails.before_calls = u64::MAX,
            15 => s.tails.after_calls = u64::MAX,
            _ => (),
        }
        let before = s.tails.clone();
        let mut returned = false;
        assert!(
            s.tail(2, |_| {
                returned = true;
                let mut c = current(27, 16);
                match m {
                    0 => c.full_discoveries = 1,
                    1 => c.local_checkpoints = 26,
                    2 => c.before_calls += 1,
                    3 => c.after_calls += 1,
                    4 => c.generation_probes += 1,
                    5 => c.local_checkpoints = u64::MAX,
                    _ => (),
                }
                Ok(((), (m != 11).then_some(c)))
            })
            .is_err(),
            "{m}"
        );
        assert!(returned && s.terminal);
        assert_eq!(s.tails, before);
        assert!(
            s.tail(2, |_| -> io::Result<((), Option<Currentness>)> {
                panic!("no retry")
            })
            .is_err()
        );
        assert!(s.begin(3, || panic!("no later forward")).is_err());
        assert!(s.closed_counts().is_err());
    }
}
#[test]
fn tail_state_refuses_order_other_profiles_first_use_scope_and_late_close() {
    let mut overflow = TailCounts {
        ordinary_tails: u32::MAX,
        ..TailCounts::default()
    };
    assert!(observe(&mut overflow, false, None).is_err());
    for p in [Profile::Readiness40, Profile::Full2303] {
        assert!(State::new(p).is_err());
    }
    for m in 0..5 {
        let mut s = State::new(Profile::Readiness40Position5).unwrap();
        if m != 0 {
            layers(&mut s, 0);
        }
        match m {
            0 => assert!(
                s.tail(0, |_| -> io::Result<((), Option<Currentness>)> {
                    panic!("before bank")
                })
                .is_err()
            ),
            1 => assert!(
                s.tail(1, |_| -> io::Result<((), Option<Currentness>)> {
                    panic!("wrong position")
                })
                .is_err()
            ),
            2 => assert!(s.begin(1, || panic!("missing tail")).is_err()),
            3 => assert!(s.tail(0, |_| Ok(((), Some(current(27, 16))))).is_err()),
            _ => assert!(
                s.dispatch(
                    0,
                    36,
                    |_| -> io::Result<((), Option<(Currentness, Census)>)> {
                        panic!("extra layer")
                    }
                )
                .is_err()
            ),
        }
        assert!(s.terminal && s.closed_counts().is_err());
    }
    let mut s = State::new(Profile::Readiness40Position5).unwrap();
    for p in 0..39 {
        forward(&mut s, p);
    }
    layers(&mut s, 39);
    assert!(s.closed_counts().is_err());
    s.tail(39, |_| Ok(((), Some(current(27, 16))))).unwrap();
    assert!(s.closed_counts().is_ok());
}
#[test]
fn tail_state_runtime_or_worker_refusal_and_unwind_are_terminal() {
    for unwind in [false, true] {
        let mut s = State::new(Profile::Readiness40Position5).unwrap();
        forward(&mut s, 0);
        forward(&mut s, 1);
        layers(&mut s, 2);
        let mut entered = false;
        let result = catch_unwind(AssertUnwindSafe(|| {
            s.tail(2, |_| -> io::Result<((), Option<Currentness>)> {
                entered = true;
                assert!(!unwind, "completed runtime followed by worker unwind");
                Err(io::Error::other(
                    "runtime refusal or completed runtime then numerical refusal",
                ))
            })
        }));
        assert!(if unwind {
            result.is_err()
        } else {
            result.unwrap().is_err()
        });
        assert!(entered && s.terminal && s.tails.scoped_tails == 0);
        assert!(s.closed_counts().is_err());
    }
}
