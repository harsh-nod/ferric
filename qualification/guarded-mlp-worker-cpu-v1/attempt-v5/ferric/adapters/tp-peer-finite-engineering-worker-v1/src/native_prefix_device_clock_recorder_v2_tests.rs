//! Synthetic private-state tests. No successful native sampling is claimed.
use super::*;
use std::cell::Cell;

fn recorder() -> Recorder {
    let mut report = data::tests::report().raw;
    report.rows.clear();
    report.completions.clear();
    report.transcript_sha256 = [0; 32];
    report.final_dispatches = [0; 2];
    report.native_closed = false;
    Recorder {
        raw: RawRecorder {
            report,
            bound: true,
            failed: false,
            packets: [0; 2],
            pending_control: None,
        },
        origin: Instant::now(),
        samples: Vec::new(),
        failed: false,
    }
}
fn raw_forward(recorder: &mut Recorder, n: usize) {
    let fixture = data::tests::report();
    for row in &fixture.raw.rows[n * raw_data::PER_FORWARD..(n + 1) * raw_data::PER_FORWARD] {
        recorder.raw.record_row(row.clone()).unwrap();
    }
}
fn complete(recorder: &mut Recorder, n: usize) {
    recorder.ready_control().unwrap();
    recorder
        .raw
        .control(&crate::finite_prefix_decode_wire_v1::tests::control())
        .unwrap();
    recorder
        .raw
        .completed(&data::tests::report().raw.completions[n])
        .unwrap();
}
fn fill(recorder: &mut Recorder) {
    let fixture = data::tests::report();
    for n in 0..4 {
        recorder
            .append_pair(fixture.samples[n * 4..n * 4 + 2].to_vec())
            .unwrap();
        raw_forward(recorder, n);
        recorder
            .append_pair(fixture.samples[n * 4 + 2..n * 4 + 4].to_vec())
            .unwrap();
        complete(recorder, n);
    }
}
#[test]
fn clock_recorder_full_synthetic_route_requires_consuming_raw_close() {
    let mut value = recorder();
    fill(&mut value);
    let closes = Cell::new(0);
    let closed = value
        .close_with(|raw| {
            raw.close_with(raw_data::RANK_PACKETS, || {
                closes.set(closes.get() + 1);
                Ok(())
            })
        })
        .unwrap();
    assert_eq!(closes.get(), 1);
    let decoded = Report::decode(&closed.encode().unwrap()).unwrap();
    assert_eq!(decoded.samples.len(), 16);
    assert_eq!(decoded.raw.rows.len(), 1172);
    assert!(!decoded.clock_domain_validated);
}
#[test]
fn clock_recorder_raw_close_failure_cannot_construct_clock_report() {
    let mut value = recorder();
    fill(&mut value);
    assert!(
        value
            .close_with(
                |raw| raw.close_with(raw_data::RANK_PACKETS, || Err(io::Error::other(
                    "synthetic consuming Close failure"
                )))
            )
            .is_err()
    );
}
#[test]
fn clock_recorder_missing_samples_refuse_close_without_calling_owner() {
    for keep in [0, 1, 8, 15] {
        let mut value = recorder();
        fill(&mut value);
        value.samples.truncate(keep);
        let called = Cell::new(false);
        assert!(
            value
                .close_with(|raw| {
                    called.set(true);
                    raw.close_with(raw_data::RANK_PACKETS, || Ok(()))
                })
                .is_err()
        );
        assert!(!called.get());
    }
}
#[test]
fn clock_recorder_bad_pair_is_atomic_and_terminalizes_both_recorders() {
    for target in [0, 6, 14] {
        let fixture = data::tests::report();
        let mut value = recorder();
        for n in 0..target / 4 {
            value
                .append_pair(fixture.samples[n * 4..n * 4 + 2].to_vec())
                .unwrap();
            raw_forward(&mut value, n);
            value
                .append_pair(fixture.samples[n * 4 + 2..n * 4 + 4].to_vec())
                .unwrap();
            complete(&mut value, n);
        }
        if target % 4 == 2 {
            value
                .append_pair(fixture.samples[target - 2..target].to_vec())
                .unwrap();
            raw_forward(&mut value, target / 4);
        }
        let mut pair = fixture.samples[target..target + 2].to_vec();
        pair[1].queue_epoch ^= 1;
        let result = value.append_pair(pair);
        assert_eq!(value.samples.len(), target);
        let owner_poisoned = Cell::new(false);
        assert!(value.finish(result, || owner_poisoned.set(true)).is_err());
        assert!(owner_poisoned.get() && value.failed && value.raw.failed);
        assert!(
            value
                .append_pair(fixture.samples[target..target + 2].to_vec())
                .is_err()
        );
        assert!(value.ready_close().is_err());
    }
}
#[test]
fn clock_recorder_post_sample_requires_exact_forward_packet_boundary() {
    let fixture = data::tests::report();
    for rows in [0, raw_data::PER_FORWARD - 1] {
        let mut value = recorder();
        value.append_pair(fixture.samples[..2].to_vec()).unwrap();
        for row in &fixture.raw.rows[..rows] {
            value.raw.record_row(row.clone()).unwrap();
        }
        assert!(value.append_pair(fixture.samples[2..4].to_vec()).is_err());
        assert!(value.ready_control().is_err());
    }
}
#[test]
fn clock_recorder_raw_control_error_runs_terminal_callback_after_post_samples() {
    let fixture = data::tests::report();
    let mut value = recorder();
    value.append_pair(fixture.samples[..2].to_vec()).unwrap();
    raw_forward(&mut value, 0);
    value.append_pair(fixture.samples[2..4].to_vec()).unwrap();
    let mut control = crate::finite_prefix_decode_wire_v1::tests::control();
    control.tail_ns[0] ^= 1;
    let result = value
        .ready_control()
        .and_then(|()| value.raw.control(&control));
    let owner_poisoned = Cell::new(false);
    assert!(value.finish(result, || owner_poisoned.set(true)).is_err());
    assert!(owner_poisoned.get() && value.failed && value.raw.failed);
}
#[test]
fn clock_recorder_forward_scope_and_incomplete_previous_forward_are_refused() {
    let fixture = data::tests::report();
    let mut value = recorder();
    let mut input = ForwardInput {
        registration: fixture.raw.bootstrap.registration,
        generation: 1,
        token: fixture.raw.bootstrap.input_tokens[0],
        cache_metadata: [0; 145],
        rotary_bits: [0; 128],
    };
    let profile = fixture.raw.profile_sha256;
    assert_eq!(value.before_forward(profile, &input).unwrap(), 0);
    assert!(value.before_forward([0; 32], &input).is_err());
    input.generation = 2;
    assert!(value.before_forward(profile, &input).is_err());
    input.generation = 1;
    input.cache_metadata[0] = 1;
    assert!(value.before_forward(profile, &input).is_err());
    input.cache_metadata[0] = 0;
    value.append_pair(fixture.samples[..2].to_vec()).unwrap();
    assert!(value.before_forward(profile, &input).is_err());
}
#[test]
fn clock_recorder_host_offset_refuses_before_origin_and_overflow() {
    let start = Instant::now();
    let later = start.checked_add(Duration::from_nanos(5)).unwrap();
    assert_eq!(offset(start, later).unwrap(), 5);
    assert!(offset(later, start).is_err());
    assert_eq!(
        duration_ns(Duration::from_nanos(u64::MAX)).unwrap(),
        u64::MAX
    );
    assert!(duration_ns(Duration::from_secs(u64::MAX / 1_000_000_000 + 1)).is_err());
}
#[test]
fn clock_recorder_sample_failure_callback_poisoning_forbids_retry() {
    let mut value = recorder();
    let owner_poisoned = Cell::new(false);
    let result: io::Result<()> = Err(io::Error::other("synthetic native sample failure"));
    assert!(value.finish(result, || owner_poisoned.set(true)).is_err());
    assert!(owner_poisoned.get());
    assert!(value.active().is_err() && value.raw.active().is_err());
}
