use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind};
fn counts(kind: usize) -> Currentness {
    let (local, calls, probes) = match kind {
        0 => (361, 726, 725),
        1 => (21, 27, 45),
        _ => (27, 43, 57),
    };
    Currentness {
        full_discoveries: 2,
        local_checkpoints: local,
        before_calls: calls,
        after_calls: calls,
        generation_probes: probes,
    }
}
fn time(c: Currentness) -> RuntimeDurations {
    let p = |calls| fe2o3_kfd::Gfx950EngineeringCurrentnessCallDurationV1 {
        calls,
        elapsed_ns: 1,
    };
    RuntimeDurations {
        before: p(c.before_calls),
        discover: p(c.full_discoveries),
        after: p(c.after_calls),
        root_generation: p(c.generation_probes),
    }
}
fn begin(s: &mut State, p: u32) {
    s.begin_diagnostic(p, || {
        Ok((
            BankCompletion {
                generation: p as u64 / 2 + 1,
                currentness: (p >= 2).then(|| counts(0)),
            },
            (p >= 2).then(|| (time(counts(0)), 5)),
        ))
    })
    .unwrap();
}
fn layers(s: &mut State, p: u32) {
    for l in 0..36 {
        s.dispatch_diagnostic(p, l, |warm| {
            Ok((
                (),
                warm.then(|| {
                    (
                        counts(1),
                        Census {
                            owner_counts: [787, 783],
                            preflights: 2,
                            rank_checkpoints: 16,
                        },
                    )
                }),
                warm.then(|| time(counts(1))),
            ))
        })
        .unwrap();
    }
}
fn forward(s: &mut State, p: u32) {
    begin(s, p);
    layers(s, p);
    s.tail_diagnostic(p, |warm| {
        Ok(((), warm.then(|| counts(2)), warm.then(|| time(counts(2)))))
    })
    .unwrap();
    s.record_forward_timing(phase(p)).unwrap();
}
fn phase(position: u32) -> forward::ForwardRow {
    let phase_ns = [1, 1, 1, 5, 144, 4, 1, 1, 1];
    forward::ForwardRow {
        position,
        forward_body_ns: phase_ns.iter().sum(),
        phase_ns,
    }
}
fn warm() -> State {
    let mut s = State::new_diagnostic(Profile::Readiness40Position5).unwrap();
    forward(&mut s, 0);
    forward(&mut s, 1);
    s
}
#[test]
fn duration_state_records_exact_forty_rows_and_two_unmeasured_forwards() {
    let mut s = State::new_diagnostic(Profile::Readiness40Position5).unwrap();
    for p in 0..40 {
        forward(&mut s, p);
    }
    let c = s.closed_counts().unwrap();
    let rows = s.diagnostic_rows(&c).unwrap();
    assert_eq!(rows.len(), 40);
    assert!(rows[..2].iter().all(|r| r.measured.is_none()));
    assert!(rows[2..].iter().all(|r| r.measured.is_some()));
    wire::validate_rows(&rows, &c).unwrap();
}
#[test]
fn duration_state_bad_bank_duration_is_terminal_before_any_layer() {
    let mut s = warm();
    assert!(
        s.begin_diagnostic(2, || Ok((
            BankCompletion {
                generation: 2,
                currentness: Some(counts(0))
            },
            Some((time(counts(0)), 3))
        )))
        .is_err()
    );
    assert!(s.terminal);
    assert!(
        s.dispatch_diagnostic::<()>(2, 0, |_| panic!("no layer after bad bank"))
            .is_err()
    );
    assert!(s.closed_counts().is_err());
}
#[test]
fn duration_state_layer_count_mismatch_and_overflow_cannot_commit() {
    for overflow in [false, true] {
        let mut s = warm();
        begin(&mut s, 2);
        if overflow {
            s.diagnostic
                .as_mut()
                .unwrap()
                .pending
                .as_mut()
                .unwrap()
                .layers
                .before
                .calls = u64::MAX;
        }
        let mut value = time(counts(1));
        if !overflow {
            value.root_generation.calls += 1;
        }
        assert!(
            s.dispatch_diagnostic(2, 0, |_| Ok((
                (),
                Some((
                    counts(1),
                    Census {
                        owner_counts: [787, 783],
                        preflights: 2,
                        rank_checkpoints: 16
                    }
                )),
                Some(value)
            )))
            .is_err()
        );
        assert!(s.terminal && s.closed_counts().is_err());
    }
}
#[test]
fn duration_state_tail_metric_error_prevents_forward_and_close_publication() {
    let mut s = warm();
    begin(&mut s, 2);
    layers(&mut s, 2);
    let mut value = time(counts(2));
    value.before.elapsed_ns = u64::MAX;
    assert!(
        s.tail_diagnostic(2, |_| Ok(((), Some(counts(2)), Some(value))))
            .is_err()
    );
    assert!(s.terminal && s.closed_counts().is_err());
    assert!(s.tail_diagnostic::<()>(2, |_| panic!("no retry")).is_err());
}
#[test]
fn duration_state_unwind_at_each_collection_boundary_stays_terminal() {
    for stage in 0..3 {
        let mut s = warm();
        if stage > 0 {
            begin(&mut s, 2);
        }
        if stage > 1 {
            layers(&mut s, 2);
        }
        assert!(
            catch_unwind(AssertUnwindSafe(|| match stage {
                0 => {
                    s.begin_diagnostic(2, || panic!("bank unwind")).unwrap();
                }
                1 => {
                    s.dispatch_diagnostic::<()>(2, 0, |_| panic!("layer unwind"))
                        .unwrap();
                }
                _ => {
                    s.tail_diagnostic::<()>(2, |_| panic!("tail unwind"))
                        .unwrap();
                }
            }))
            .is_err()
        );
        assert!(s.terminal && s.closed_counts().is_err());
    }
}
#[test]
fn duration_state_missing_metrics_first_use_and_wrong_positions_refuse_without_fallback() {
    let mut first = State::new_diagnostic(Profile::Readiness40Position5).unwrap();
    assert!(
        first
            .begin_diagnostic(0, || Ok((
                BankCompletion {
                    generation: 1,
                    currentness: None
                },
                Some((time(counts(0)), 5))
            )))
            .is_err()
    );
    assert!(first.terminal && first.closed_counts().is_err());
    let mut s = State::new(Profile::Readiness40Position5).unwrap();
    assert!(
        s.begin_diagnostic(0, || panic!("diagnostics not selected"))
            .is_err()
    );
    assert!(s.terminal);
    let mut s = warm();
    assert!(
        s.begin_diagnostic(3, || panic!("wrong position before runtime"))
            .is_err()
    );
    assert!(s.terminal);
    let mut s = warm();
    assert!(
        s.begin_diagnostic(2, || Ok((
            BankCompletion {
                generation: 2,
                currentness: Some(counts(0))
            },
            None
        )))
        .is_err()
    );
    assert!(s.terminal);
}

#[test]
fn duration_phase_state_forty_rows_contain_original_callback_rows() {
    let mut s = State::new_diagnostic(Profile::Readiness40Position5).unwrap();
    for p in 0..40 {
        forward(&mut s, p);
    }
    let counts = s.closed_counts().unwrap();
    let callbacks = s.diagnostic_rows(&counts).unwrap();
    let phases = s.forward_rows().unwrap();
    assert_eq!(phases.len(), 40);
    for (position, (row, callback)) in phases.iter().zip(&callbacks).enumerate() {
        assert_eq!(*row, phase(position as u32));
        row.validate_against(callback).unwrap();
    }
    assert!(callbacks[..2].iter().all(|r| r.measured.is_none()));
}

#[test]
fn duration_phase_state_each_containment_refusal_is_terminal() {
    for index in [3, 4, 5] {
        let mut s = warm();
        begin(&mut s, 2);
        layers(&mut s, 2);
        s.tail_diagnostic(2, |_| Ok(((), Some(counts(2)), Some(time(counts(2))))))
            .unwrap();
        let mut row = phase(2);
        row.phase_ns[index] -= 1;
        row.forward_body_ns -= 1;
        assert!(s.record_forward_timing(row).is_err());
        assert!(s.terminal && s.closed_counts().is_err() && s.forward_rows().is_err());
        assert!(
            s.begin_diagnostic(3, || panic!("no bank after phase refusal"))
                .is_err()
        );
        assert!(s.record_forward_timing(phase(2)).is_err());
    }
}

#[test]
fn duration_phase_state_missing_prior_row_refuses_before_next_runtime_call() {
    let mut s = State::new_diagnostic(Profile::Readiness40Position5).unwrap();
    begin(&mut s, 0);
    layers(&mut s, 0);
    s.tail_diagnostic(0, |_| Ok(((), None, None))).unwrap();
    assert!(
        s.begin_diagnostic(1, || panic!("missing phases precede runtime"))
            .is_err()
    );
    assert!(s.terminal && s.forward_rows().is_err());
}

#[test]
fn duration_phase_state_duplicate_position_and_absent_callbacks_refuse() {
    let mut duplicate = State::new_diagnostic(Profile::Readiness40Position5).unwrap();
    forward(&mut duplicate, 0);
    assert!(duplicate.record_forward_timing(phase(0)).is_err());
    assert!(duplicate.terminal);
    for position in [0, 1, u32::MAX] {
        let mut s = State::new_diagnostic(Profile::Readiness40Position5).unwrap();
        assert!(s.record_forward_timing(phase(position)).is_err());
        assert!(s.terminal);
    }
}

#[test]
fn duration_phase_state_total_overflow_and_whole_bound_refuse_before_sink_commit() {
    for prior in [u64::MAX, 3_600_000_000_000] {
        let mut s = State::new_diagnostic(Profile::Readiness40Position5).unwrap();
        begin(&mut s, 0);
        layers(&mut s, 0);
        s.tail_diagnostic(0, |_| Ok(((), None, None))).unwrap();
        s.diagnostic.as_mut().unwrap().forward_body_ns = prior;
        assert!(s.record_forward_timing(phase(0)).is_err());
        assert!(s.terminal);
        assert!(s.diagnostic.as_ref().unwrap().forward_rows.is_empty());
    }
}
