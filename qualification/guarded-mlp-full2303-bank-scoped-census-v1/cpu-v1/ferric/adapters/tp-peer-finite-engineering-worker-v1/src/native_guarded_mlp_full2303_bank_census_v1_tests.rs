use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind};

fn window(bank: bool) -> Currentness {
    if bank {
        Currentness {
            full_discoveries: 2,
            local_checkpoints: 361,
            before_calls: 726,
            after_calls: 726,
            generation_probes: 725,
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
            currentness: (p >= 2).then(|| window(true)),
        })
    })
    .unwrap();
}
fn layers(s: &mut State, p: u32) {
    for l in 0..36 {
        s.dispatch(p, l, |warm| {
            assert_eq!(warm, p >= 2);
            Ok(((), warm.then(|| (window(false), census()))))
        })
        .unwrap();
    }
}
fn warm() -> State {
    let mut s = State::new(Profile::Full2303).unwrap();
    for p in 0..2 {
        begin(&mut s, p);
        layers(&mut s, p);
    }
    s
}
#[test]
fn full_bank_census_state_runs_all_2303_banks_and_82836_warm_layers_in_order() {
    let mut s = State::new(Profile::Full2303).unwrap();
    for p in 0..2303 {
        begin(&mut s, p);
        assert_eq!(
            s.banks.final_generations[p as usize % 2],
            u64::from(p) / 2 + 1
        );
        layers(&mut s, p);
        if [39, 40, 2047, 2048, 2302].contains(&p) {
            assert_eq!(s.next_position, p + 1);
            assert!(!s.bank_active && s.next_layer == 0);
        }
    }
    let c = s.closed_counts().unwrap();
    assert_eq!(
        (c.layers.ordinary_layers, c.layers.scoped_layers),
        (72, 82_836)
    );
    assert_eq!(
        (c.banks.ordinary_initial_banks, c.banks.scoped_rearms),
        (2, 2301)
    );
    assert_eq!(c.banks.final_generations, [1152, 1151]);
    assert_eq!(c.banks.full_discoveries, 4602);
    assert_eq!(c.layers.full_discoveries, 165_672);
    assert_eq!(c.census.warm_layers, 82_836);
    assert_eq!(c.census.preflights, 165_672);
    assert_eq!(c.census.rank_checkpoints, 1_325_376);
    assert_eq!(c.census.owner_counts, [787, 783]);
    assert!(s.begin(2303, || panic!("after Full extent")).is_err());
    assert!(s.terminal);
}
#[test]
fn full_bank_census_state_refuses_wrong_profiles_order_and_first_use_substitution() {
    for p in [Profile::Readiness40, Profile::Readiness40Position5] {
        assert!(State::new(p).is_err());
    }
    for mutation in 0..5 {
        let mut s = State::new(Profile::Full2303).unwrap();
        assert!(s.closed_counts().is_err());
        match mutation {
            0 => assert!(
                s.dispatch(
                    0,
                    0,
                    |_| -> io::Result<((), Option<(Currentness, Census)>)> {
                        panic!("layer before bank")
                    }
                )
                .is_err()
            ),
            1 => assert!(s.begin(1, || panic!("wrong first position")).is_err()),
            2 => {
                begin(&mut s, 0);
                assert!(s.begin(0, || panic!("duplicate bank")).is_err());
            }
            3 => assert!(
                s.begin(0, || Ok(BankCompletion {
                    generation: 1,
                    currentness: Some(window(true)),
                }))
                .is_err()
            ),
            _ => {
                begin(&mut s, 0);
                assert!(
                    s.dispatch(0, 0, |_| Ok(((), Some((window(false), census())))))
                        .is_err()
                );
            }
        }
        assert!(s.terminal && s.closed_counts().is_err());
    }
}
#[test]
fn full_bank_census_state_post_runtime_bank_rejection_is_fatal_without_retry() {
    for mutation in 0..8 {
        let mut s = warm();
        let mut returned = false;
        assert!(
            s.begin(2, || {
                returned = true;
                let mut c = window(true);
                match mutation {
                    0 => c.full_discoveries = 1,
                    1 => c.local_checkpoints = 0,
                    2 => c.before_calls += 1,
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
        assert!(returned && s.terminal && s.banks.scoped_rearms == 0);
        assert!(s.begin(2, || panic!("failed bank retry")).is_err());
    }
}
#[test]
fn full_bank_census_state_post_runtime_layer_rejection_is_fatal_without_retry() {
    for mutation in 0..9 {
        let mut s = warm();
        begin(&mut s, 2);
        let mut returned = false;
        assert!(
            s.dispatch(2, 0, |_| {
                returned = true;
                let mut c = census();
                let mut v = window(false);
                match mutation {
                    0 => c.preflights = 1,
                    1 => c.rank_checkpoints = 15,
                    2 => c.owner_counts[0] = 0,
                    3 => c.owner_counts[1] = 2049,
                    4 => v.local_checkpoints = 16,
                    5 => v.before_calls = 0,
                    6 => v.after_calls += 1,
                    7 => v.generation_probes = u64::MAX,
                    _ => (),
                }
                Ok(((), (mutation != 8).then_some((v, c))))
            })
            .is_err()
        );
        assert!(returned && s.terminal && s.census.warm_layers == 0);
        assert!(
            s.dispatch(
                2,
                0,
                |_| -> io::Result<((), Option<(Currentness, Census)>)> {
                    panic!("failed layer retry")
                }
            )
            .is_err()
        );
    }
}
#[test]
fn full_bank_census_state_checks_rank_order_overflow_and_incomplete_close() {
    let mut s = warm();
    begin(&mut s, 2);
    s.dispatch(2, 0, |_| Ok(((), Some((window(false), census())))))
        .unwrap();
    let mut c = census();
    c.owner_counts.swap(0, 1);
    assert!(
        s.dispatch(2, 1, |_| Ok(((), Some((window(false), c)))))
            .is_err()
    );
    assert!(s.terminal && s.closed_counts().is_err());
    let mut subset = CensusCounts {
        warm_layers: u32::MAX,
        owner_counts: [787, 783],
        ..CensusCounts::default()
    };
    assert!(observe(&mut subset, census(), window(false)).is_err());
    let mut banks = BankCounts {
        full_discoveries: u64::MAX,
        ..BankCounts::default()
    };
    assert!(add_window(&mut banks, window(true)).is_err());
    let mut s = warm();
    begin(&mut s, 2);
    assert!(
        s.dispatch(
            2,
            1,
            |_| -> io::Result<((), Option<(Currentness, Census)>)> { panic!("wrong layer") }
        )
        .is_err()
    );
    assert!(s.terminal);
}
#[test]
fn full_bank_census_state_runtime_error_and_unwind_do_not_commit_or_fall_back() {
    for stage in 0..4 {
        let mut s = warm();
        if stage < 2 {
            if stage == 0 {
                assert!(
                    s.begin(2, || Err(io::Error::other("runtime bank")))
                        .is_err()
                );
            } else {
                assert!(
                    catch_unwind(AssertUnwindSafe(|| {
                        let _ = s.begin(2, || panic!("bank unwind"));
                    }))
                    .is_err()
                );
            }
        } else {
            begin(&mut s, 2);
            if stage == 2 {
                assert!(
                    s.dispatch(
                        2,
                        0,
                        |_| -> io::Result<((), Option<(Currentness, Census)>)> {
                            Err(io::Error::other("runtime layer"))
                        }
                    )
                    .is_err()
                );
            } else {
                assert!(
                    catch_unwind(AssertUnwindSafe(|| {
                        let _ = s.dispatch(
                            2,
                            0,
                            |_| -> io::Result<((), Option<(Currentness, Census)>)> {
                                panic!("layer unwind")
                            },
                        );
                    }))
                    .is_err()
                );
            }
        }
        assert!(s.terminal && s.next_position == 2 && s.next_layer == 0);
        assert!(s.closed_counts().is_err());
    }
}
