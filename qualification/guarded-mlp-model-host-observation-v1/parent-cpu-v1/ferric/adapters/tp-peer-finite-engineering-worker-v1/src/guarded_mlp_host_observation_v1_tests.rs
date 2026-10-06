use super::*;
use crate::finite_guarded_mlp_decode_wire_v1::{self as wire, InputMode};
use crate::prefix_decode_host_observation_v1::Rank;

pub(crate) fn fixture(mode: InputMode) -> Report {
    let mut bootstrap = wire::tests::bootstrap(mode);
    bootstrap.decode.device_ids[0] = u64::from(u32::MAX) + 3;
    let snapshots = (0..SNAPSHOTS).map(|i| {
        let n = i as u64;
        Snapshot {
            phase: phase(i).unwrap(), group_incarnation: 13, shared_full_currentness: false,
            ranks: std::array::from_fn(|rank| {
                let mut counters = [0; 19];
                counters[0] = n * 2; counters[1] = n * 700;
                counters[2] = n; counters[3] = n * 500;
                counters[6] = n; counters[7] = n * 200;
                counters[13] = n; counters[14] = n * 8192; counters[15] = n * 100;
                Rank { rank: rank as u32, unique_id: bootstrap.decode.device_ids[rank],
                    queue_epoch: 7 + rank as u64, cache_kernel_admission: false,
                    raw_timestamp_queue: false, counters }
            }), shared: [n, n * 500, n, n * 200],
        }
    }).collect::<Vec<_>>();
    let intervals = snapshots.windows(2).map(|pair| Interval {
        host_elapsed_ns: 1000,
        shared: difference(&pair[1].shared, &pair[0].shared).unwrap(),
        ranks: std::array::from_fn(|rank| difference(&pair[1].ranks[rank].counters, &pair[0].ranks[rank].counters).unwrap()),
    }).collect();
    let mut chain = Chain::new(bootstrap.decode.registration, bootstrap.sha256().unwrap());
    let mut completions = Vec::new();
    let payload = vec![0; crate::finite_forward_wire_v1::OBSERVATION_BYTES];
    for position in 0..4 {
        let previous = completions.last().map(|c: &Completion| c.output_token);
        let mut done = Completion {
            generation: position as u64 + 1, position,
            input_token: bootstrap.input(position, previous).unwrap(), output_token: 20 + position,
            control: crate::finite_forward_wire_v1::part(&wire::tests::control(position as u64 + 1).encode()),
            observation: crate::finite_forward_wire_v1::part(&payload),
            capture: crate::finite_forward_wire_v1::Payload::from_bytes(&payload).unwrap(), chain: [0; 32],
        };
        done.chain = chain.advance(&done);
        completions.push(done);
    }
    Report {
        schema: SCHEMA.into(), worker_sha256: [42; 32], child_pid: bootstrap.decode.scope.child_identity,
        profile_sha256: bootstrap.sha256().unwrap(), bootstrap, snapshots, intervals,
        forward_host_ns: [144999; 4], close_host_ns: 100, completions,
        native_closed: true, inclusive_nested_host_scopes: true,
        paired_generic_dispatch_timers_complete: false, tensor_stage_capture: false,
        gpu_time: false, gpu_overlap: false, numerical_acceptance: false,
        full_model_acceptance: false, performance_claim: false, production_authority: false,
    }
}

#[test]
fn guarded_host_fixed_phase_roster_has_all_layer_and_forward_boundaries() {
    assert_eq!(SNAPSHOTS, 587);
    let names = (0..SNAPSHOTS).map(|i| phase(i).unwrap()).collect::<std::collections::BTreeSet<_>>();
    assert_eq!(names.len(), SNAPSHOTS);
    assert_eq!(phase(0).unwrap(), "fresh_enabled");
    assert_eq!(phase(1).unwrap(), "setup_sealed");
    assert_eq!(phase(2).unwrap(), "forward_0/begin");
    assert_eq!(phase(3).unwrap(), "forward_0/layer_00/begin");
    assert_eq!(phase(146).unwrap(), "forward_0/layer_35/hidden");
    assert_eq!(phase(147).unwrap(), "forward_0/done");
    assert_eq!(phase(585).unwrap(), "forward_3/done");
    assert_eq!(phase(586).unwrap(), "before_close");
    assert!(phase(587).is_err());
    assert!(phase(usize::MAX).is_err());
}

#[test]
fn guarded_host_tf_ar_and_high_u64_id_roundtrip_preserve_inclusive_counters() {
    for mode in [InputMode::TeacherForced, InputMode::Autoregressive] {
        let report = fixture(mode);
        report.validate().unwrap();
        let bytes = serde_json::to_vec(&report).unwrap();
        assert!(bytes.len() < MAX_BYTES);
        let decoded = Report::decode(&bytes).unwrap();
        assert_eq!(decoded.snapshots, report.snapshots);
        assert_eq!(decoded.intervals, report.intervals);
        assert_eq!(decoded.completions, report.completions);
        assert!(decoded.bootstrap.decode.device_ids[0] > u64::from(u32::MAX));
        // Inclusive nested counters may sum above elapsed; they are not a partition.
        let delta = &decoded.intervals[0];
        assert!(delta.ranks[0][1] + delta.ranks[0][3] + delta.shared[1] > delta.host_elapsed_ns);
        assert_eq!(delta.ranks[0][9..12], [0; 3]);
        assert!(!decoded.paired_generic_dispatch_timers_complete);
    }
}

#[test]
fn guarded_host_identity_policy_epoch_and_baseline_drift_refuse() {
    let report = fixture(InputMode::TeacherForced);
    for field in 0..9 {
        let mut changed = report.clone();
        match field {
            0 => changed.snapshots[1].group_incarnation += 1,
            1 => changed.snapshots[1].shared_full_currentness = true,
            2 => changed.snapshots[1].ranks[0].rank = 1,
            3 => changed.snapshots[1].ranks[0].unique_id += 1,
            4 => changed.snapshots[1].ranks[0].queue_epoch += 1,
            5 => changed.snapshots[1].ranks[0].cache_kernel_admission = true,
            6 => changed.snapshots[1].ranks[0].raw_timestamp_queue = true,
            7 => changed.snapshots[1].ranks[0].counters[4] = 1,
            _ => changed.snapshots[0].shared[0] = 1,
        }
        assert!(changed.validate().is_err(), "field {field}");
    }
}

#[test]
fn guarded_host_missing_reordered_decreasing_or_forged_intervals_refuse() {
    let report = fixture(InputMode::TeacherForced);
    for field in 0..9 {
        let mut changed = report.clone();
        match field {
            0 => { changed.snapshots.pop(); },
            1 => { changed.intervals.pop(); },
            2 => changed.snapshots.swap(3, 4),
            3 => changed.snapshots[3].ranks[0].counters[0] = 0,
            4 => changed.snapshots[3].shared[0] = 0,
            5 => changed.intervals[0].ranks[1][3] += 1,
            6 => changed.intervals[0].shared[0] += 1,
            7 => changed.forward_host_ns[0] = 145001,
            _ => changed.intervals[2].host_elapsed_ns = u64::MAX,
        }
        assert!(changed.validate().is_err(), "field {field}");
    }
}

#[test]
fn guarded_host_exact_parent_completion_join_rejects_other_worker_or_history() {
    let report = fixture(InputMode::Autoregressive);
    report.validate_expected(&report.bootstrap, report.worker_sha256, report.child_pid, &report.completions).unwrap();
    assert!(report.validate_expected(&report.bootstrap, [1;32], report.child_pid, &report.completions).is_err());
    assert!(report.validate_expected(&report.bootstrap, report.worker_sha256, report.child_pid + 1, &report.completions).is_err());
    let mut done = report.completions.clone(); done[1].observation.sha256[0] ^= 1;
    assert!(report.validate_expected(&report.bootstrap, report.worker_sha256, report.child_pid, &done).is_err());
    let mut wrong = report.clone(); wrong.completions[1].input_token ^= 1;
    assert!(wrong.validate().is_err());
    let mut wrong = report.clone(); wrong.completions[0].chain[0] ^= 1;
    assert!(wrong.validate().is_err());
}

#[test]
fn guarded_host_requires_healthy_close_closed_schema_and_observational_scope() {
    let report = fixture(InputMode::TeacherForced);
    for field in ["native_closed", "inclusive_nested_host_scopes", "paired_generic_dispatch_timers_complete",
        "tensor_stage_capture", "gpu_time", "gpu_overlap", "numerical_acceptance", "full_model_acceptance",
        "performance_claim", "production_authority"] {
        let mut value = serde_json::to_value(&report).unwrap();
        value[field] = serde_json::json!(!value[field].as_bool().unwrap());
        assert!(Report::decode(&serde_json::to_vec(&value).unwrap()).is_err(), "{field}");
    }
    let mut value = serde_json::to_value(&report).unwrap(); value["unknown"] = serde_json::json!(0);
    assert!(Report::decode(&serde_json::to_vec(&value).unwrap()).is_err());
    assert!(Report::decode(&[]).is_err());
    assert!(Report::decode(&vec![b' '; MAX_BYTES+1]).is_err());
}
