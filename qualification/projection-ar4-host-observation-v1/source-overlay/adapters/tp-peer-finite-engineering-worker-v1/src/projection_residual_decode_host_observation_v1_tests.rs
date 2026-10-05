use super::*;
use crate::finite_prefix_decode_wire_v1 as wire;

pub(crate) fn report() -> Report {
    report_for(InputMode::Autoregressive)
}
fn report_for(mode: InputMode) -> Report {
    let old = crate::prefix_decode_host_observation_v1::tests::report(mode);
    let bootstrap = Bootstrap {
        schema: crate::finite_projection_residual_decode_wire_v1::SCHEMA.into(),
        decode: old.bootstrap,
        projection_residual_image: crate::finite_setup_wire_v1::Part {
            bytes: 17,
            sha256: [33; 32],
        },
    };
    let profile = bootstrap.sha256().unwrap();
    let mut chain = wire::Chain::new(bootstrap.decode.registration, profile);
    let mut completions = old.completions;
    for c in &mut completions {
        c.chain = chain.advance(c);
    }
    Report {
        schema: SCHEMA.into(),
        bootstrap,
        worker_sha256: old.worker_sha256,
        child_pid: old.child_pid,
        profile_sha256: profile,
        snapshots: old.snapshots,
        intervals: old.intervals,
        forward_host_ns: old.forward_host_ns,
        serialization_host_ns: [10; 4],
        close_host_ns: old.close_host_ns,
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
fn projection_host_report_roundtrips_and_rejects_legacy_schema() {
    let v = report();
    let bytes = serde_json::to_vec(&v).unwrap();
    let decoded = Report::decode(&bytes).unwrap();
    assert_eq!(decoded.bootstrap, v.bootstrap);
    assert_eq!(decoded.serialization_host_ns, [10; 4]);
    assert_eq!(decoded.completions, v.completions);
    assert!(crate::prefix_decode_host_observation_v1::Report::decode(&bytes).is_err());
    let old = crate::prefix_decode_host_observation_v1::tests::report(InputMode::Autoregressive);
    assert!(Report::decode(&serde_json::to_vec(&old).unwrap()).is_err());
}
#[test]
fn projection_host_report_requires_ar_and_selected_image_profile() {
    assert!(report_for(InputMode::TeacherForced).validate().is_err());
    for change in 0..3 {
        let mut v = report();
        match change {
            0 => v.bootstrap.projection_residual_image.sha256[0] ^= 1,
            1 => v.profile_sha256 = v.bootstrap.decode.sha256().unwrap(),
            _ => v.bootstrap.projection_residual_image.bytes = 0,
        }
        assert!(v.validate().is_err());
    }
}
#[test]
fn projection_host_report_retains_full_currentness_and_rank_epoch_policy() {
    for i in 0..7 {
        for rank in 0..2 {
            for change in 0..7 {
                let mut v = report();
                let r = &mut v.snapshots[i].ranks[rank];
                match change {
                    0 => r.queue_epoch += 1,
                    1 => r.unique_id += 1,
                    2 => r.rank ^= 1,
                    3 => r.cache_kernel_admission = true,
                    4 => r.raw_timestamp_queue = true,
                    5 => r.counters[4] = 1,
                    _ => r.counters[5] = 1,
                }
                assert!(v.validate().is_err());
            }
        }
    }
    let mut v = report();
    v.snapshots[3].shared_full_currentness = true;
    assert!(v.validate().is_err());
}
#[test]
fn projection_host_report_rejects_counter_delta_drift() {
    for i in 0..6 {
        for rank in 0..2 {
            for n in 0..19 {
                let mut v = report();
                v.intervals[i].ranks[rank][n] += 1;
                assert!(v.validate().is_err());
            }
        }
    }
    let mut v = report();
    v.intervals[0].shared[0] += 1;
    assert!(v.validate().is_err());
}
#[test]
fn projection_host_report_bounds_run_and_serialization_by_distinct_intervals() {
    for i in 0..4 {
        let mut v = report();
        v.forward_host_ns[i] = v.intervals[i + 1].host_elapsed_ns + 1;
        assert!(v.validate().is_err());
        let mut v = report();
        v.serialization_host_ns[i] = v.intervals[i + 2].host_elapsed_ns + 1;
        assert!(v.validate().is_err());
    }
}
#[test]
fn projection_host_report_requires_close_roster_and_false_claims() {
    for change in 0..9 {
        let mut v = report();
        match change {
            0 => v.native_closed = false,
            1 => v.gpu_time = true,
            2 => v.numerical_acceptance = true,
            3 => v.performance_claim = true,
            4 => v.production_authority = true,
            5 => v.inclusive_nested_host_scopes = false,
            6 => {
                v.snapshots.pop();
            }
            7 => {
                v.intervals.pop();
            }
            _ => {
                v.completions.pop();
            }
        }
        assert!(v.validate().is_err());
    }
}
#[test]
fn projection_host_report_rejects_trajectory_or_chain_mutation() {
    for i in 0..4 {
        for change in 0..4 {
            let mut v = report();
            match change {
                0 => v.completions[i].input_token ^= 1,
                1 => v.completions[i].chain[0] ^= 1,
                2 => v.completions[i].control.sha256[0] ^= 1,
                _ => v.completions[i].observation.sha256[0] ^= 1,
            }
            assert!(v.validate().is_err());
        }
    }
}
#[test]
fn projection_host_report_rejects_unknown_fields_and_missing_serialization() {
    let mut v = serde_json::to_value(report()).unwrap();
    v["host_policy"] = "operational".into();
    assert!(Report::decode(&serde_json::to_vec(&v).unwrap()).is_err());
    v.as_object_mut().unwrap().remove("host_policy");
    v.as_object_mut().unwrap().remove("serialization_host_ns");
    assert!(Report::decode(&serde_json::to_vec(&v).unwrap()).is_err());
}
