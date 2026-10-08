use super::*;

pub(crate) fn rows_for(callbacks: &old::Record) -> Vec<ForwardRow> {
    forward::tests::rows_for(callbacks)
        .into_iter()
        .map(|row| ForwardRow {
            position: row.position,
            phase_ns: row.phase_ns,
            forward_body_ns: row.forward_body_ns,
            layer_metrics: (row.position >= 2).then_some(LayerMetrics {
                layers: 36,
                phase_ns: [0, 0, row.phase_ns[4], 0, 0, 0],
                layer_body_ns: row.phase_ns[4],
                paired_mlp_phase_ns: [0, 0, 0, 0, row.phase_ns[4], 0, 0],
                paired_mlp_body_ns: row.phase_ns[4],
            }),
        })
        .collect()
}
pub(crate) fn fixture() -> (ready::Bootstrap, policy::PolicyRecord, old::Record, Record) {
    let (bootstrap, policy, callbacks) = old::tests::fixture();
    let record = Record::new(&policy, &callbacks, rows_for(&callbacks)).unwrap();
    (bootstrap, policy, callbacks, record)
}
fn stderr(policy: &policy::PolicyRecord, callbacks: &old::Record, record: &Record) -> Vec<u8> {
    [
        policy.encode().unwrap(),
        callbacks.encode().unwrap(),
        record.encode().unwrap(),
    ]
    .concat()
}

#[test]
fn layer_duration_roundtrip_preserves_two_original_records_and_rejects_v1() {
    let (bootstrap, policy, callbacks, record) = fixture();
    let prefix = [policy.encode().unwrap(), callbacks.encode().unwrap()].concat();
    let raw = stderr(&policy, &callbacks, &record);
    assert!(raw.starts_with(&prefix));
    assert_eq!(raw.iter().filter(|b| **b == b'\n').count(), 3);
    assert_eq!(
        decode_stderr(&raw, &bootstrap, [7; 32], [8; 32]).unwrap(),
        (policy.clone(), callbacks.clone(), record)
    );
    assert!(forward::decode_stderr(&raw, &bootstrap, [7; 32], [8; 32]).is_err());
    let old_record =
        forward::Record::new(&policy, &callbacks, forward::tests::rows_for(&callbacks)).unwrap();
    let old_raw = [prefix, old_record.encode().unwrap()].concat();
    assert!(decode_stderr(&old_raw, &bootstrap, [7; 32], [8; 32]).is_err());
}

#[test]
fn layer_duration_requires_fixed_six_and_seven_stage_order() {
    let (_, policy, callbacks, record) = fixture();
    assert_eq!(
        LAYER_STAGE_ORDER,
        [
            "enter_pre_census",
            "prefix",
            "mlp_retired_seal",
            "hidden_post_census",
            "full_exit",
            "commit_prepare"
        ]
    );
    assert_eq!(
        PAIRED_MLP_STAGE_ORDER,
        [
            "preflight",
            "consume",
            "reserve",
            "publish",
            "poll",
            "retire",
            "terminal"
        ]
    );
    for index in 0..6 {
        let mut changed = record.clone();
        changed.layer_stage_order[index] = "other".into();
        assert!(changed.validate(&policy, &callbacks).is_err());
    }
    for index in 0..7 {
        let mut changed = record.clone();
        changed.paired_mlp_stage_order[index] = "other".into();
        assert!(changed.validate(&policy, &callbacks).is_err());
    }
    let mut changed = record.clone();
    changed.phase_order.swap(3, 4);
    assert!(changed.validate(&policy, &callbacks).is_err());
}

#[test]
fn layer_duration_requires_cold_none_and_thirty_six_warm_layers() {
    let (_, policy, callbacks, record) = fixture();
    assert!(
        record.forwards[..2]
            .iter()
            .all(|r| r.layer_metrics.is_none())
    );
    assert!(
        record.forwards[2..]
            .iter()
            .all(|r| r.layer_metrics.unwrap().layers == 36)
    );
    for which in 0..6 {
        let mut rows = record.forwards.clone();
        match which {
            0 => rows[0].layer_metrics = rows[2].layer_metrics,
            1 => rows[2].layer_metrics = None,
            2 => rows[2].layer_metrics.as_mut().unwrap().layers = 0,
            3 => rows[2].layer_metrics.as_mut().unwrap().layers = 35,
            4 => rows[2].layer_metrics.as_mut().unwrap().layers = 37,
            _ => rows[2].layer_metrics.as_mut().unwrap().layers = u32::MAX,
        }
        assert!(Record::new(&policy, &callbacks, rows).is_err());
    }
}

#[test]
fn layer_duration_refuses_each_closed_sum_and_paired_sum_drift() {
    let (_, policy, callbacks, record) = fixture();
    for which in 0..15 {
        let mut rows = record.forwards.clone();
        let m = rows[2].layer_metrics.as_mut().unwrap();
        match which {
            0..=5 => m.phase_ns[which] += 1,
            6..=12 => m.paired_mlp_phase_ns[which - 6] += 1,
            13 => m.layer_body_ns += 1,
            _ => m.paired_mlp_body_ns += 1,
        }
        assert!(Record::new(&policy, &callbacks, rows).is_err());
    }
}

#[test]
fn layer_duration_refuses_both_nested_containment_violations() {
    let (_, policy, callbacks, record) = fixture();
    for which in 0..2 {
        let mut rows = record.forwards.clone();
        let m = rows[2].layer_metrics.as_mut().unwrap();
        if which == 0 {
            m.phase_ns[0] += 1;
            m.layer_body_ns += 1;
        } else {
            m.paired_mlp_phase_ns[0] += 1;
            m.paired_mlp_body_ns += 1;
        }
        assert!(Record::new(&policy, &callbacks, rows).is_err());
    }
}

#[test]
fn layer_duration_checks_overflow_and_independent_whole_case_bound() {
    let (_, policy, callbacks, record) = fixture();
    for which in 0..3 {
        let mut rows = record.forwards.clone();
        let m = rows[2].layer_metrics.as_mut().unwrap();
        match which {
            0 => m.phase_ns[0] = u64::MAX,
            1 => m.paired_mlp_phase_ns[0] = u64::MAX,
            _ => {
                m.phase_ns = [WHOLE_NS + 1, 0, 0, 0, 0, 0];
                m.layer_body_ns = WHOLE_NS + 1;
            }
        }
        assert!(Record::new(&policy, &callbacks, rows).is_err());
    }
    let mut rows = record.forwards.clone();
    rows[0].phase_ns = [WHOLE_NS, 0, 0, 0, 0, 0, 0, 0, 0];
    rows[0].forward_body_ns = WHOLE_NS;
    rows[0].validate().unwrap();
    assert!(Record::new(&policy, &callbacks, rows).is_err());
}

#[test]
fn layer_duration_reuses_original_callback_and_policy_predicates() {
    let (_, policy, callbacks, record) = fixture();
    let mut changed = callbacks.clone();
    changed.forwards[2]
        .measured
        .as_mut()
        .unwrap()
        .layers
        .before
        .calls += 1;
    assert!(Record::new(&policy, &changed, record.forwards.clone()).is_err());
    let mut changed_policy = policy.clone();
    changed_policy.native_closed = false;
    assert!(Record::new(&changed_policy, &callbacks, record.forwards.clone()).is_err());
    let mut rows = record.forwards.clone();
    rows[2].phase_ns[3] -= 1;
    rows[2].forward_body_ns -= 1;
    assert!(Record::new(&policy, &callbacks, rows).is_err());
}

#[test]
fn layer_duration_rejects_missing_duplicate_reordered_and_extra_rows() {
    let (_, policy, callbacks, record) = fixture();
    for which in 0..4 {
        let mut rows = record.forwards.clone();
        match which {
            0 => {
                rows.pop();
            }
            1 => rows[3].position = 2,
            2 => rows.swap(2, 3),
            _ => rows.push(rows[39]),
        }
        assert!(Record::new(&policy, &callbacks, rows).is_err());
    }
}

#[test]
fn layer_duration_authenticates_all_identities_and_nested_flags() {
    let (bootstrap, policy, callbacks, record) = fixture();
    for key in [
        "instrumented",
        "host_elapsed_nanoseconds",
        "disjoint_phases",
        "currentness_durations_nested",
        "layer_durations_nested",
        "paired_mlp_durations_nested",
        "gpu_timing",
        "numerical_acceptance",
        "performance_claim",
        "execution_authority",
    ] {
        let mut changed = serde_json::to_value(&record).unwrap();
        changed[key] = serde_json::json!(!changed[key].as_bool().unwrap());
        let changed: Record = serde_json::from_value(changed).unwrap();
        assert!(changed.validate(&policy, &callbacks).is_err());
    }
    for which in 0..6 {
        let mut changed = record.clone();
        match which {
            0 => changed.schema = forward::SCHEMA.into(),
            1 => changed.policy_sha256[0] ^= 1,
            2 => changed.currentness_record_sha256[0] ^= 1,
            3 => changed.session[0] ^= 1,
            4 => changed.worker_sha256[0] ^= 1,
            _ => changed.transcript_sha256[0] ^= 1,
        }
        assert!(
            decode_stderr(
                &stderr(&policy, &callbacks, &changed),
                &bootstrap,
                [7; 32],
                [8; 32]
            )
            .is_err()
        );
    }
}

#[test]
fn layer_duration_rejects_noncanonical_framing_unknown_and_duplicate_fields() {
    let (bootstrap, policy, callbacks, record) = fixture();
    let prefix = [policy.encode().unwrap(), callbacks.encode().unwrap()].concat();
    let third = record.encode().unwrap();
    for changed in [
        [b" ".to_vec(), third.clone()].concat(),
        [b"{\"instrumented\":true,".to_vec(), third[1..].to_vec()].concat(),
        [serde_json::to_vec_pretty(&record).unwrap(), b"\n".to_vec()].concat(),
        third[..third.len() - 1].to_vec(),
        [third.clone(), b"\n".to_vec()].concat(),
        [third.clone(), third.clone()].concat(),
    ] {
        assert!(
            decode_stderr(
                &[prefix.clone(), changed].concat(),
                &bootstrap,
                [7; 32],
                [8; 32]
            )
            .is_err()
        );
    }
    let mut changed = serde_json::to_value(&record).unwrap();
    changed["forwards"][2]["layer_metrics"]["extra"] = serde_json::json!(0);
    assert!(serde_json::from_value::<Record>(changed).is_err());
}

#[test]
fn layer_duration_rejects_integer_aliases_and_wrong_fixed_array_extents() {
    let (_, _, _, record) = fixture();
    for bad in [
        serde_json::json!(true),
        serde_json::json!(-1),
        serde_json::json!(1.5),
    ] {
        for field in ["phase_ns", "paired_mlp_phase_ns"] {
            let mut changed = serde_json::to_value(&record).unwrap();
            changed["forwards"][2]["layer_metrics"][field][0] = bad.clone();
            assert!(serde_json::from_value::<Record>(changed).is_err());
        }
    }
    for (field, len) in [
        ("phase_ns", 5),
        ("phase_ns", 7),
        ("paired_mlp_phase_ns", 6),
        ("paired_mlp_phase_ns", 8),
    ] {
        let mut changed = serde_json::to_value(&record).unwrap();
        changed["forwards"][2]["layer_metrics"][field] = serde_json::json!(vec![0_u64; len]);
        assert!(serde_json::from_value::<Record>(changed).is_err());
    }
}

#[test]
fn layer_duration_checked_aggregation_retains_count_and_stage_sums() {
    let one = LayerMetrics {
        layers: 1,
        phase_ns: [1, 2, 7, 3, 4, 5],
        layer_body_ns: 22,
        paired_mlp_phase_ns: [1; 7],
        paired_mlp_body_ns: 7,
    };
    let mut aggregate = LayerMetrics::default();
    for _ in 0..36 {
        aggregate = aggregate.checked_add(one).unwrap();
    }
    assert_eq!(aggregate.layers, 36);
    assert_eq!(aggregate.layer_body_ns, 792);
    assert_eq!(aggregate.phase_ns, [36, 72, 252, 108, 144, 180]);
    assert_eq!(aggregate.paired_mlp_phase_ns, [36; 7]);
    assert!(aggregate.checked_add(one).is_err());
    let corrupt = LayerMetrics {
        layer_body_ns: 1,
        ..LayerMetrics::default()
    };
    assert!(corrupt.checked_add(one).is_err());
}

#[test]
fn layer_duration_legal_one_hour_case_and_fixed_structural_ceiling_fit_caps() {
    let (bootstrap, policy, callbacks, mut record) = fixture();
    for row in &mut record.forwards {
        row.phase_ns = [10_000_000_000; 9];
        row.forward_body_ns = WHOLE_NS / 40;
        if let Some(m) = &mut row.layer_metrics {
            m.phase_ns = [
                1_000_000_000,
                1_000_000_000,
                5_000_000_000,
                1_000_000_000,
                1_000_000_000,
                1_000_000_000,
            ];
            m.layer_body_ns = 10_000_000_000;
            m.paired_mlp_phase_ns = [
                500_000_000,
                500_000_000,
                500_000_000,
                500_000_000,
                2_000_000_000,
                500_000_000,
                500_000_000,
            ];
            m.paired_mlp_body_ns = 5_000_000_000;
        }
    }
    record = Record::new(&policy, &callbacks, record.forwards).unwrap();
    assert_eq!(
        record
            .forwards
            .iter()
            .map(|r| r.forward_body_ns)
            .sum::<u64>(),
        WHOLE_NS
    );
    let raw = stderr(&policy, &callbacks, &record);
    assert!(raw.len() <= STDERR_MAX_BYTES);
    decode_stderr(&raw, &bootstrap, [7; 32], [8; 32]).unwrap();
    let mut maximum = record.clone();
    maximum.policy_sha256 = [255; 32];
    maximum.currentness_record_sha256 = [255; 32];
    maximum.session = [255; 32];
    maximum.worker_sha256 = [255; 32];
    maximum.transcript_sha256 = [255; 32];
    for row in &mut maximum.forwards {
        row.position = u32::MAX;
        row.phase_ns = [u64::MAX; 9];
        row.forward_body_ns = u64::MAX;
        row.layer_metrics = Some(LayerMetrics {
            layers: u32::MAX,
            phase_ns: [u64::MAX; 6],
            layer_body_ns: u64::MAX,
            paired_mlp_phase_ns: [u64::MAX; 7],
            paired_mlp_body_ns: u64::MAX,
        });
    }
    // Every integer and optional shape is at its serialized maximum; semantic validation still refuses it.
    assert!(maximum.encode().unwrap().len() <= MAX_BYTES);
    assert!(maximum.validate(&policy, &callbacks).is_err());
    maximum.forwards = vec![maximum.forwards[0]; 1368];
    assert!(maximum.encode().is_err());
}

#[test]
fn layer_duration_cap_refusals_keep_original_limits() {
    let (bootstrap, policy, callbacks, record) = fixture();
    assert_eq!(MAX_BYTES, 32768);
    assert_eq!(STDERR_MAX_BYTES, 69632);
    let mut oversized = record.clone();
    oversized.schema = "x".repeat(MAX_BYTES);
    assert!(oversized.encode().is_err());
    assert!(
        decode_stderr(
            &vec![b' '; STDERR_MAX_BYTES + 1],
            &bootstrap,
            [7; 32],
            [8; 32]
        )
        .is_err()
    );
    let raw = [
        policy.encode().unwrap(),
        callbacks.encode().unwrap(),
        vec![b' '; MAX_BYTES + 1],
    ]
    .concat();
    assert!(decode_stderr(&raw, &bootstrap, [7; 32], [8; 32]).is_err());
}

#[test]
fn layer_duration_zero_time_keeps_exact_warm_count_without_callback_subtraction() {
    let (_, policy, mut callbacks, _) = fixture();
    for row in &mut callbacks.forwards {
        if let Some(m) = &mut row.measured {
            for d in [&mut m.bank, &mut m.layers, &mut m.tail] {
                d.before.elapsed_ns = 0;
                d.discover.elapsed_ns = 0;
                d.after.elapsed_ns = 0;
                d.root_generation.elapsed_ns = 0;
            }
            m.bank_guarded_body_ns = 0;
        }
    }
    let rows = (0..40)
        .map(|position| ForwardRow {
            position,
            phase_ns: [0; 9],
            forward_body_ns: 0,
            layer_metrics: (position >= 2).then_some(LayerMetrics {
                layers: 36,
                ..LayerMetrics::default()
            }),
        })
        .collect();
    let record = Record::new(&policy, &callbacks, rows).unwrap();
    record.validate(&policy, &callbacks).unwrap();
    assert_eq!(record.forwards[2].layer_metrics.unwrap().layers, 36);
}

#[test]
fn layer_duration_rejects_closed_body_below_unchanged_callback_subtotal() {
    let (_, policy, callbacks, record) = fixture();
    let mut rows = record.forwards.clone();
    rows[2].layer_metrics = Some(LayerMetrics {
        layers: 36,
        ..LayerMetrics::default()
    });
    rows[2].validate().unwrap();
    rows[2]
        .original()
        .validate_against(&callbacks.forwards[2])
        .unwrap();
    assert!(
        callbacks.forwards[2]
            .measured
            .unwrap()
            .layers
            .elapsed_subtotal()
            .unwrap()
            > 0
    );
    assert!(rows[2].validate_against(&callbacks.forwards[2]).is_err());
    assert!(validate_rows(&rows, &callbacks).is_err());
    assert!(Record::new(&policy, &callbacks, rows).is_err());
}
