//! Exercises the private state machine with explicitly synthetic reports/rows.
use super::*;
use crate::finite_prefix_decode_wire_v1::InputMode;
use crate::prefix_decode_device_observation_v1::tests::report;
use std::cell::Cell;

fn recorder() -> Recorder {
    let mut value = report(InputMode::TeacherForced);
    value.rows.clear();
    value.completions.clear();
    value.transcript_sha256 = [0; 32];
    value.final_dispatches = [0; 2];
    value.native_closed = false;
    Recorder {
        report: value,
        bound: true,
        failed: false,
        packets: [0; 2],
        pending_control: None,
    }
}
fn completed(recorder: &mut Recorder, completion: &Completion) {
    recorder
        .control(&crate::finite_prefix_decode_wire_v1::tests::control())
        .unwrap();
    recorder.completed(completion).unwrap();
}
fn fill(recorder: &mut Recorder) {
    let value = report(InputMode::TeacherForced);
    for (i, row) in value.rows.into_iter().enumerate() {
        recorder.record_row(row).unwrap();
        if (i + 1) % data::PER_FORWARD == 0 {
            completed(recorder, &value.completions[i / data::PER_FORWARD]);
        }
    }
}
#[test]
fn device_recorder_synthetic_full_census_requires_successful_consuming_close() {
    let mut recorder = recorder();
    fill(&mut recorder);
    let called = Cell::new(0);
    let closed = recorder
        .close_with(data::RANK_PACKETS, || {
            called.set(called.get() + 1);
            Ok(())
        })
        .unwrap();
    assert_eq!(called.get(), 1);
    let value = Report::decode(&closed.encode().unwrap()).unwrap();
    assert_eq!(value.rows.len(), 1172);
    assert!(!value.calibrated_nanoseconds && !value.performance_claim);
}
#[test]
fn device_recorder_close_error_cannot_return_a_report() {
    let mut recorder = recorder();
    fill(&mut recorder);
    assert!(
        recorder
            .close_with(data::RANK_PACKETS, || Err(io::Error::other(
                "synthetic native Close failure"
            )))
            .is_err()
    );
}
#[test]
fn device_recorder_missing_binding_is_terminal_before_any_row() {
    let mut recorder = recorder();
    recorder.bound = false;
    assert!(
        recorder
            .record_row(report(InputMode::TeacherForced).rows[0].clone())
            .is_err()
    );
    assert!(recorder.failed);
    assert!(recorder.report.rows.is_empty());
    recorder.bound = true;
    assert!(
        recorder
            .record_row(report(InputMode::TeacherForced).rows[0].clone())
            .is_err()
    );
}
#[test]
fn device_recorder_first_middle_last_bad_row_poison_and_forbid_retry() {
    for failure in [0, 586, data::MAX_ROWS - 1] {
        let fixture = report(InputMode::TeacherForced);
        let mut recorder = recorder();
        for i in 0..failure {
            recorder.record_row(fixture.rows[i].clone()).unwrap();
            if (i + 1) % data::PER_FORWARD == 0 {
                completed(&mut recorder, &fixture.completions[i / data::PER_FORWARD]);
            }
        }
        let mut bad = fixture.rows[failure].clone();
        bad.queue_epoch += 1;
        assert!(recorder.record_row(bad).is_err());
        assert_eq!(recorder.report.rows.len(), failure);
        assert!(recorder.failed);
        assert!(recorder.record_row(fixture.rows[failure].clone()).is_err());
        let called = Cell::new(false);
        assert!(
            recorder
                .close_with(data::RANK_PACKETS, || {
                    called.set(true);
                    Ok(())
                })
                .is_err()
        );
        assert!(!called.get());
    }
}
#[test]
fn device_recorder_forward_transition_requires_the_prior_completion() {
    let fixture = report(InputMode::TeacherForced);
    let mut recorder = recorder();
    for row in &fixture.rows[..data::PER_FORWARD] {
        recorder.record_row(row.clone()).unwrap();
    }
    assert!(
        recorder
            .record_row(fixture.rows[data::PER_FORWARD].clone())
            .is_err()
    );
    assert!(recorder.completed(&fixture.completions[0]).is_err());
}
#[test]
fn device_recorder_early_duplicate_wrong_completion_is_terminal() {
    for kind in 0..3 {
        let fixture = report(InputMode::TeacherForced);
        let mut recorder = recorder();
        if kind != 0 {
            for row in &fixture.rows[..data::PER_FORWARD] {
                recorder.record_row(row.clone()).unwrap();
            }
        }
        if kind == 1 {
            completed(&mut recorder, &fixture.completions[0]);
        }
        if kind == 2 {
            recorder
                .control(&crate::finite_prefix_decode_wire_v1::tests::control())
                .unwrap();
        }
        let mut c = fixture.completions[0].clone();
        if kind == 2 {
            c.chain[0] ^= 1;
        }
        assert!(recorder.completed(&c).is_err());
        assert!(recorder.failed);
    }
}
#[test]
fn device_recorder_missing_rows_or_wrong_native_counts_do_not_close() {
    for kind in 0..3 {
        let mut recorder = recorder();
        if kind > 0 {
            fill(&mut recorder);
        }
        if kind == 2 {
            recorder.report.completions.pop();
        }
        let called = Cell::new(false);
        let counts = if kind == 1 {
            [593, 580]
        } else {
            data::RANK_PACKETS
        };
        assert!(
            recorder
                .close_with(counts, || {
                    called.set(true);
                    Ok(())
                })
                .is_err()
        );
        assert!(!called.get());
    }
}
#[test]
fn device_recorder_extra_packet_after_four_forwards_is_terminal() {
    let mut recorder = recorder();
    fill(&mut recorder);
    assert!(
        recorder
            .record_row(report(InputMode::TeacherForced).rows[0].clone())
            .is_err()
    );
    assert_eq!(recorder.report.rows.len(), data::MAX_ROWS);
    assert!(recorder.failed);
}

#[test]
fn device_recorder_control_is_required_once_and_bad_join_is_terminal() {
    let fixture = report(InputMode::TeacherForced);
    for kind in 0..5 {
        let mut recorder = recorder();
        if kind != 0 {
            for row in &fixture.rows[..data::PER_FORWARD] {
                recorder.record_row(row.clone()).unwrap();
            }
        }
        let mut control = crate::finite_prefix_decode_wire_v1::tests::control();
        match kind {
            0 => assert!(recorder.control(&control).is_err()),
            1 => assert!(recorder.completed(&fixture.completions[0]).is_err()),
            2 => {
                recorder.control(&control).unwrap();
                assert!(recorder.control(&control).is_err());
            }
            3 => {
                control.tail_ns[2] ^= 1;
                assert!(recorder.control(&control).is_err());
            }
            _ => {
                recorder.control(&control).unwrap();
                let mut c = fixture.completions[0].clone();
                c.control.sha256[0] ^= 1;
                assert!(recorder.completed(&c).is_err());
            }
        }
        assert!(recorder.failed);
        assert!(
            recorder
                .control(&crate::finite_prefix_decode_wire_v1::tests::control())
                .is_err()
        );
        assert!(recorder.completed(&fixture.completions[0]).is_err());
    }
}
