use super::*;
use crate::finite_prefix_decode_wire_v1::{
    self as wire,
    tests::{bootstrap, completed, request},
};

pub(crate) fn report(mode: wire::InputMode) -> Report {
    let b = bootstrap(mode);
    let mut chain = wire::Chain::new(b.registration, b.sha256().unwrap());
    let mut previous = None;
    let mut completions = Vec::new();
    for position in 0..4 {
        let (response, _, _) =
            completed(&request(&b, position, previous), &mut chain, 100 + position);
        let wire::Event::Completed(c) = response.event else {
            panic!("fixture completion")
        };
        previous = Some(c.output_token);
        completions.push(c);
    }
    let snapshots = PHASES
        .iter()
        .enumerate()
        .map(|(i, phase)| Snapshot {
            phase: (*phase).into(),
            group_incarnation: 7,
            shared_full_currentness: false,
            ranks: core::array::from_fn(|rank| Rank {
                rank: rank as u32,
                unique_id: b.device_ids[rank],
                queue_epoch: 11 + rank as u64,
                cache_kernel_admission: false,
                raw_timestamp_queue: false,
                counters: core::array::from_fn(|n| {
                    if n == 4 || n == 5 {
                        0
                    } else {
                        (i * (rank + 1) * (n + 1)) as u64
                    }
                }),
            }),
            shared: core::array::from_fn(|n| (i * (n + 1)) as u64),
        })
        .collect::<Vec<_>>();
    let intervals = snapshots
        .windows(2)
        .map(|v| Interval {
            host_elapsed_ns: 100,
            ranks: core::array::from_fn(|r| {
                difference(&v[1].ranks[r].counters, &v[0].ranks[r].counters).unwrap()
            }),
            shared: difference(&v[1].shared, &v[0].shared).unwrap(),
        })
        .collect();
    Report {
        schema: SCHEMA.into(),
        profile_sha256: b.sha256().unwrap(),
        child_pid: b.scope.child_identity,
        bootstrap: b,
        worker_sha256: [9; 32],
        snapshots,
        intervals,
        forward_host_ns: [80; 4],
        close_host_ns: 90,
        completions,
        transcript_sha256: chain.digest(),
        native_closed: true,
        inclusive_nested_host_scopes: true,
        gpu_time: false,
        numerical_acceptance: false,
        performance_claim: false,
        production_authority: false,
    }
}

#[test]
fn prefix_host_data_roundtrip_tf_ar_exact_counters_and_bound() {
    for mode in [
        wire::InputMode::TeacherForced,
        wire::InputMode::Autoregressive,
    ] {
        let value = report(mode);
        value.validate().unwrap();
        let bytes = serde_json::to_vec(&value).unwrap();
        assert!(bytes.len() < MAX_BYTES);
        let decoded = Report::decode(&bytes).unwrap();
        assert_eq!(decoded.snapshots, value.snapshots);
        assert_eq!(decoded.intervals, value.intervals);
        assert_eq!(decoded.completions, value.completions);
        assert_eq!(
            COUNTER_NAMES[9..12],
            [
                "dispatch_prepare_ns",
                "dispatch_publish_ns",
                "dispatch_wait_ns"
            ]
        );
        assert_eq!(COUNTER_NAMES[13..16], ["reads", "read_bytes", "read_ns"]);
    }
}
#[test]
fn prefix_host_data_rejects_every_rank_policy_identity_and_epoch_change() {
    for snapshot in 0..7 {
        for rank in 0..2 {
            for change in 0..7 {
                let mut value = report(wire::InputMode::TeacherForced);
                let v = &mut value.snapshots[snapshot].ranks[rank];
                match change {
                    0 => v.rank ^= 1,
                    1 => v.unique_id += 1,
                    2 => v.queue_epoch += 1,
                    3 => v.cache_kernel_admission = true,
                    4 => v.raw_timestamp_queue = true,
                    5 => v.counters[4] = 1,
                    _ => v.counters[5] = 1,
                }
                assert!(value.validate().is_err(), "{snapshot}/{rank}/{change}");
            }
        }
    }
}
#[test]
fn prefix_host_data_rejects_snapshot_roster_order_and_missing_close() {
    for change in 0..9 {
        let mut v = report(wire::InputMode::TeacherForced);
        match change {
            0 => {
                v.snapshots.pop();
            }
            1 => v.snapshots.push(v.snapshots[6].clone()),
            2 => v.snapshots.swap(2, 3),
            3 => v.snapshots[2].group_incarnation += 1,
            4 => v.snapshots[2].shared_full_currentness = true,
            5 => v.snapshots[0].shared[0] = 1,
            6 => {
                v.intervals.pop();
            }
            7 => v.native_closed = false,
            _ => v.snapshots[0].group_incarnation = 0,
        }
        assert!(v.validate().is_err());
    }
}
#[test]
fn prefix_host_data_checked_delta_rejects_every_counter_mutation_and_underflow() {
    for interval in 0..6 {
        for rank in 0..2 {
            for n in 0..19 {
                let mut v = report(wire::InputMode::TeacherForced);
                v.intervals[interval].ranks[rank][n] += 1;
                assert!(v.validate().is_err());
            }
        }
        for n in 0..4 {
            let mut v = report(wire::InputMode::TeacherForced);
            v.intervals[interval].shared[n] += 1;
            assert!(v.validate().is_err());
        }
    }
    assert!(difference(&[0], &[1]).is_err());
    assert_eq!(difference(&[u64::MAX], &[0]).unwrap(), [u64::MAX]);
    let mut v = report(wire::InputMode::TeacherForced);
    v.snapshots[4].ranks[0].counters[1] = 0;
    assert!(v.validate().is_err());
}
#[test]
fn prefix_host_data_nested_host_scopes_are_not_summed_into_gpu_time() {
    let mut v = report(wire::InputMode::TeacherForced);
    for i in 1..7 {
        v.snapshots[i].ranks[0].counters[1] = (i as u64) * 1000;
        v.intervals[i - 1].ranks[0][1] = 1000;
    }
    v.validate().unwrap();
    v.forward_host_ns[3] = 101;
    assert!(v.validate().is_err());
    for field in [
        "gpu_time",
        "performance_claim",
        "numerical_acceptance",
        "production_authority",
    ] {
        let mut json = serde_json::to_value(report(wire::InputMode::TeacherForced)).unwrap();
        json[field] = true.into();
        assert!(Report::decode(&serde_json::to_vec(&json).unwrap()).is_err());
    }
}
#[test]
fn prefix_host_data_binds_complete_four_trajectory_and_control_payload_digests() {
    for change in 0..10 {
        let mut v = report(wire::InputMode::Autoregressive);
        match change {
            0 => v.child_pid += 1,
            1 => v.profile_sha256[0] ^= 1,
            2 => v.worker_sha256 = [0; 32],
            3 => {
                v.completions.pop();
            }
            4 => v.completions.swap(0, 1),
            5 => v.completions[2].input_token += 1,
            6 => v.completions[2].control.bytes += 1,
            7 => v.completions[2].observation.sha256[0] ^= 1,
            8 => v.completions[2].chain[0] ^= 1,
            _ => v.transcript_sha256[0] ^= 1,
        }
        assert!(v.validate().is_err());
    }
}
#[test]
fn prefix_host_data_closed_json_schema_and_extent() {
    let bytes = serde_json::to_vec(&report(wire::InputMode::TeacherForced)).unwrap();
    assert!(Report::decode(&[]).is_err());
    assert!(Report::decode(&vec![b' '; MAX_BYTES + 1]).is_err());
    assert!(Report::decode(&bytes[..bytes.len() - 1]).is_err());
    let mut json = serde_json::from_slice::<serde_json::Value>(&bytes).unwrap();
    json["gpu_duration_ns"] = 1.into();
    assert!(Report::decode(&serde_json::to_vec(&json).unwrap()).is_err());
}
