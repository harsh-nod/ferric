pub(crate) fn shared_report() -> SharedReport {
    let mut observation = report();
    observation.schema = SHARED_SCHEMA.into();
    for snapshot in &mut observation.snapshots {
        snapshot.shared_full_currentness = true;
    }
    SharedReport {
        schema: SHARED_ENVELOPE.into(),
        policy: Policy::SharedFull,
        configuration_host_ns: 9000,
        observation,
    }
}

#[test]
fn projection_shared_report_roundtrips_without_relaxing_v1() {
    let v = shared_report();
    let raw = serde_json::to_vec(&v).unwrap();
    let decoded = SharedReport::decode(&raw).unwrap();
    assert_eq!(decoded.observation.snapshots, v.observation.snapshots);
    assert_eq!(decoded.observation.completions, v.observation.completions);
    assert_eq!(decoded.configuration_host_ns, 9000);
    assert!(Report::decode(&raw).is_err());
    assert!(Report::decode(&serde_json::to_vec(&v.observation).unwrap()).is_err());
    let old = serde_json::to_vec(&report()).unwrap();
    Report::decode(&old).unwrap();
    assert!(SharedReport::decode(&old).is_err());
    let value: serde_json::Value = serde_json::from_slice(&old).unwrap();
    assert!(value.get("policy").is_none());
    assert!(value.get("configuration_host_ns").is_none());
}

#[test]
fn projection_shared_report_requires_fixed_policy_and_both_schemas() {
    for change in 0..5 {
        let mut v = shared_report();
        match change {
            0 => v.policy = Policy::DefaultFull,
            1 => v.schema = SCHEMA.into(),
            2 => v.observation.schema = SCHEMA.into(),
            3 => v.observation.schema = SHARED_ENVELOPE.into(),
            _ => v.observation.snapshots[0].shared_full_currentness = false,
        }
        assert!(v.validate().is_err());
    }
    assert_eq!(Policy::DefaultFull.schema(), SCHEMA);
    assert!(!Policy::DefaultFull.shared());
    assert!(Policy::SharedFull.shared());
}

#[test]
fn projection_shared_report_checks_every_snapshot_rank_and_full_policy() {
    for i in 0..7 {
        let mut v = shared_report();
        v.observation.snapshots[i].shared_full_currentness = false;
        assert!(v.validate().is_err());
        for rank in 0..2 {
            for change in 0..7 {
                let mut v = shared_report();
                let r = &mut v.observation.snapshots[i].ranks[rank];
                match change {
                    0 => r.cache_kernel_admission = true,
                    1 => r.raw_timestamp_queue = true,
                    2 => r.counters[4] = 1,
                    3 => r.counters[5] = 1,
                    4 => r.queue_epoch += 1,
                    5 => r.unique_id ^= 1,
                    _ => r.rank ^= 1,
                }
                assert!(v.validate().is_err(), "{i}/{rank}/{change}");
            }
        }
    }
}

#[test]
fn projection_shared_report_preserves_zero_baseline_and_checked_deltas() {
    for change in 0..5 {
        let mut v = shared_report();
        match change {
            0 => v.observation.snapshots[0].shared[0] = 1,
            1 => v.observation.snapshots[0].ranks[1].counters[2] = 1,
            2 => v.observation.intervals[2].shared[0] += 1,
            3 => v.observation.snapshots[4].shared[0] = 0,
            _ => v.observation.intervals[3].ranks[0][3] += 1,
        }
        assert!(v.validate().is_err());
    }
    assert!(difference(&[0_u64], &[u64::MAX]).is_err());
}

#[test]
fn projection_shared_report_configuration_time_is_outside_snapshot_intervals() {
    let mut v = shared_report();
    for configuration_host_ns in [0, 9000, u64::MAX] {
        v.configuration_host_ns = configuration_host_ns;
        let decoded = SharedReport::decode(&serde_json::to_vec(&v).unwrap()).unwrap();
        assert_eq!(decoded.configuration_host_ns, configuration_host_ns);
        assert_eq!(decoded.observation.intervals[0].host_elapsed_ns, 100);
    }
    v.observation.forward_host_ns[2] = 101;
    assert!(v.validate().is_err());
}

#[test]
fn projection_shared_report_retains_own_recurrence_projection_and_transcript() {
    for change in 0..7 {
        let mut v = shared_report();
        match change {
            0 => v.observation.completions[1].input_token ^= 1,
            1 => v.observation.completions[3].chain[0] ^= 1,
            2 => v.observation.bootstrap.projection_residual_image.sha256[0] ^= 1,
            3 => v.observation.profile_sha256[0] ^= 1,
            4 => v.observation.bootstrap.decode.mode = InputMode::TeacherForced,
            5 => v.observation.transcript_sha256[0] ^= 1,
            _ => v.observation.completions[2].observation.sha256[0] ^= 1,
        }
        assert!(v.validate().is_err());
    }
}

#[test]
fn projection_shared_report_keeps_close_extent_and_non_authority_checks() {
    for change in 0..8 {
        let mut v = shared_report();
        match change {
            0 => v.observation.native_closed = false,
            1 => v.observation.gpu_time = true,
            2 => v.observation.performance_claim = true,
            3 => v.observation.production_authority = true,
            4 => v.observation.numerical_acceptance = true,
            5 => {
                v.observation.completions.pop();
            }
            6 => {
                v.observation.snapshots.pop();
            }
            _ => {
                v.observation.intervals.pop();
            }
        }
        assert!(v.validate().is_err());
    }
    assert!(SharedReport::decode(&[]).is_err());
    assert!(SharedReport::decode(&vec![b' '; MAX_BYTES + 1]).is_err());
}

#[test]
fn projection_shared_report_rejects_unknown_fields_and_non_u64_configuration() {
    for value in [
        serde_json::json!(-1),
        serde_json::json!(1.0),
        serde_json::json!(true),
    ] {
        let mut v = serde_json::to_value(shared_report()).unwrap();
        v["configuration_host_ns"] = value;
        assert!(SharedReport::decode(&serde_json::to_vec(&v).unwrap()).is_err());
    }
    for policy in ["operational", "immutable-admission-cache", "default-full"] {
        let mut v = serde_json::to_value(shared_report()).unwrap();
        v["policy"] = policy.into();
        assert!(SharedReport::decode(&serde_json::to_vec(&v).unwrap()).is_err());
    }
    let mut v = serde_json::to_value(shared_report()).unwrap();
    v["cached_topology"] = true.into();
    assert!(SharedReport::decode(&serde_json::to_vec(&v).unwrap()).is_err());
    v.as_object_mut().unwrap().remove("cached_topology");
    v.as_object_mut().unwrap().remove("configuration_host_ns");
    assert!(SharedReport::decode(&serde_json::to_vec(&v).unwrap()).is_err());
}
