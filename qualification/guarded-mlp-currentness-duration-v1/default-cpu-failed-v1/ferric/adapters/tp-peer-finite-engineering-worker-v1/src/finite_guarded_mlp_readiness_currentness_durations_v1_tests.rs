use super::*;
use crate::finite_guarded_mlp_long_wire_v2 as long;

pub(crate) fn fixture() -> (ready::Bootstrap, policy::PolicyRecord, Record) {
    let b = ready::Bootstrap {
        schema: ready::POSITION5_SCHEMA.into(),
        child_deadline_ms: 60_000,
        sequence: long::tests::bootstrap(long::Profile::Readiness40Position5),
    };
    let counts = policy::Counts {
        layers: policy::LayerCounts {
            ordinary_layers: 72,
            scoped_layers: 1368,
            scoped_layers_by_forward: (0..40).map(|p| if p < 2 { 0 } else { 36 }).collect(),
            full_discoveries: 2736,
            local_checkpoints: 21 * 1368,
            before_calls: 27 * 1368,
            after_calls: 27 * 1368,
            generation_probes: 45 * 1368,
        },
        banks: policy::BankCounts {
            ordinary_initial_banks: 2,
            scoped_rearms: 38,
            scoped_rearms_by_forward: (0..40).map(|p| if p < 2 { 0 } else { 1 }).collect(),
            final_generations: [20, 20],
            full_discoveries: 76,
            local_checkpoints: 361 * 38,
            before_calls: 726 * 38,
            after_calls: 726 * 38,
            generation_probes: 725 * 38,
        },
        census: policy::CensusCounts {
            warm_layers: 1368,
            preflights: 2736,
            rank_checkpoints: 21888,
            owner_counts: [787, 783],
        },
        tails: policy::TailCounts {
            ordinary_tails: 2,
            scoped_tails: 38,
            dispatches: 114,
            readbacks: 114,
            readback_bytes: 11_858_584,
            full_discoveries: 76,
            local_checkpoints: 27 * 38,
            before_calls: 43 * 38,
            after_calls: 43 * 38,
            generation_probes: 57 * 38,
        },
    };
    let policy = policy::PolicyRecord::new(&b, [7; 32], [8; 32], counts).unwrap();
    let d = |full, before, after, probes| Durations {
        discover: CallDuration {
            calls: full,
            elapsed_ns: 1,
        },
        before: CallDuration {
            calls: before,
            elapsed_ns: 1,
        },
        after: CallDuration {
            calls: after,
            elapsed_ns: 1,
        },
        root_generation: CallDuration {
            calls: probes,
            elapsed_ns: 1,
        },
    };
    let rows = (0..40)
        .map(|position| ForwardRow {
            position,
            measured: (position >= 2).then(|| MeasuredForward {
                bank: d(2, 726, 726, 725),
                layers: d(72, 972, 972, 1620),
                tail: d(2, 43, 43, 57),
                bank_guarded_body_ns: 5,
            }),
        })
        .collect();
    let record = Record::new(&policy, rows).unwrap();
    (b, policy, record)
}
fn stderr(policy: &policy::PolicyRecord, record: &Record) -> Vec<u8> {
    [policy.encode().unwrap(), record.encode().unwrap()].concat()
}
#[test]
fn duration_record_roundtrips_two_original_lines_without_policy_laundering() {
    let (b, policy, record) = fixture();
    let raw = stderr(&policy, &record);
    assert!(raw.starts_with(&policy.encode().unwrap()));
    assert_eq!(
        decode_stderr(&raw, &b, [7; 32], [8; 32]).unwrap(),
        (policy.clone(), record)
    );
    assert!(policy::PolicyRecord::decode(&raw, &b, [7; 32], [8; 32]).is_err());
}
#[test]
fn duration_record_requires_exact_two_canonical_bounded_records() {
    let (b, policy, record) = fixture();
    let raw = stderr(&policy, &record);
    for changed in [
        policy.encode().unwrap(),
        record.encode().unwrap(),
        [raw.clone(), b"\n".to_vec()].concat(),
        [raw.clone(), record.encode().unwrap()].concat(),
        raw[..raw.len() - 1].to_vec(),
        vec![b' '; STDERR_MAX_BYTES + 1],
    ] {
        assert!(decode_stderr(&changed, &b, [7; 32], [8; 32]).is_err());
    }
}
#[test]
fn duration_record_joins_policy_worker_transcript_and_session_identity() {
    let (b, policy, record) = fixture();
    for which in 0..4 {
        let mut r = record.clone();
        match which {
            0 => r.policy_sha256[0] ^= 1,
            1 => r.session[0] ^= 1,
            2 => r.worker_sha256[0] ^= 1,
            _ => r.transcript_sha256[0] ^= 1,
        }
        assert!(decode_stderr(&stderr(&policy, &r), &b, [7; 32], [8; 32]).is_err());
    }
    assert!(decode_stderr(&stderr(&policy, &record), &b, [9; 32], [8; 32]).is_err());
    assert!(decode_stderr(&stderr(&policy, &record), &b, [7; 32], [9; 32]).is_err());
}
#[test]
fn duration_record_refuses_missing_duplicate_reordered_or_measured_first_use_rows() {
    let (_, policy, record) = fixture();
    for which in 0..5 {
        let mut r = record.clone();
        match which {
            0 => {
                r.forwards.pop();
            }
            1 => r.forwards[3].position = 2,
            2 => r.forwards.swap(2, 3),
            3 => r.forwards[0].measured = r.forwards[2].measured,
            _ => r.forwards[2].measured = None,
        }
        assert!(r.validate(&policy).is_err());
    }
}
#[test]
fn duration_record_refuses_bank_row_drift_even_when_global_totals_cancel() {
    let (_, policy, mut record) = fixture();
    record.forwards[2]
        .measured
        .as_mut()
        .unwrap()
        .bank
        .before
        .calls += 1;
    record.forwards[3]
        .measured
        .as_mut()
        .unwrap()
        .bank
        .before
        .calls -= 1;
    assert!(record.validate(&policy).is_err());
}
#[test]
fn duration_record_checks_bank_subtotal_overflow_and_body_containment() {
    let (_, policy, record) = fixture();
    let mut r = record.clone();
    r.forwards[2]
        .measured
        .as_mut()
        .unwrap()
        .bank_guarded_body_ns = 3;
    assert!(r.validate(&policy).is_err());
    let mut d = record.forwards[2].measured.unwrap().bank;
    d.before.elapsed_ns = u64::MAX;
    assert!(d.elapsed_subtotal().is_err());
    d.before.calls = u64::MAX;
    assert!(
        d.checked_add(record.forwards[3].measured.unwrap().bank)
            .is_err()
    );
}
#[test]
fn duration_record_joins_each_scope_to_original_policy_counts() {
    let (_, policy, record) = fixture();
    for which in 0..4 {
        let mut r = record.clone();
        let v = r.forwards[2].measured.as_mut().unwrap();
        match which {
            0 => v.layers.discover.calls += 1,
            1 => v.layers.root_generation.calls += 1,
            2 => v.tail.before.calls += 1,
            _ => v.tail.root_generation.calls += 1,
        }
        assert!(r.validate(&policy).is_err());
    }
}
#[test]
fn duration_record_refuses_claim_type_unknown_duplicate_and_noncanonical_bytes() {
    let (b, policy, record) = fixture();
    for key in [
        "instrumented",
        "host_elapsed_nanoseconds",
        "bank_guarded_body_includes_callbacks",
        "numerical_acceptance",
        "performance_claim",
        "execution_authority",
    ] {
        let mut value = serde_json::to_value(&record).unwrap();
        value[key] = serde_json::json!(!value[key].as_bool().unwrap());
        let r: Record = serde_json::from_value(value).unwrap();
        assert!(r.validate(&policy).is_err());
    }
    let mut value = serde_json::to_value(&record).unwrap();
    value["extra"] = serde_json::json!(0);
    assert!(serde_json::from_value::<Record>(value).is_err());
    let raw = record.encode().unwrap();
    for changed in [
        b" {}".to_vec(),
        [b" ".to_vec(), raw.clone()].concat(),
        [b"{\"instrumented\":true,".to_vec(), raw[1..].to_vec()].concat(),
    ] {
        assert!(
            decode_stderr(
                &[policy.encode().unwrap(), changed].concat(),
                &b,
                [7; 32],
                [8; 32]
            )
            .is_err()
        );
    }
    let mut value = serde_json::to_value(&record).unwrap();
    value["forwards"][2]["measured"]["bank"]["before"]["calls"] = serde_json::json!(true);
    assert!(serde_json::from_value::<Record>(value).is_err());
}
#[test]
fn duration_record_does_not_accept_changed_original_policy_or_out_of_bound_ns() {
    let (b, mut policy, mut record) = fixture();
    policy.native_closed = false;
    assert!(decode_stderr(&stderr(&policy, &record), &b, [7; 32], [8; 32]).is_err());
    let (_, good, _) = fixture();
    record.forwards[2]
        .measured
        .as_mut()
        .unwrap()
        .layers
        .before
        .elapsed_ns = WHOLE_NS + 1;
    assert!(record.validate(&good).is_err());
}
