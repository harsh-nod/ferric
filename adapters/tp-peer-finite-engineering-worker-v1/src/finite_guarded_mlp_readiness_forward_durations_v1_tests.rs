use super::*;

pub(crate) fn rows_for(callbacks: &old::Record) -> Vec<ForwardRow> {
    callbacks
        .forwards
        .iter()
        .map(|row| {
            let mut phase_ns = [1; PHASE_COUNT];
            if let Some(value) = row.measured {
                phase_ns[3] = value.bank_guarded_body_ns.max(1);
                phase_ns[4] = value.layers.elapsed_subtotal().unwrap().max(1);
                phase_ns[5] = value.tail.elapsed_subtotal().unwrap().max(1);
            }
            ForwardRow {
                position: row.position,
                forward_body_ns: phase_ns.iter().copied().sum(),
                phase_ns,
            }
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

fn replace_third(policy: &policy::PolicyRecord, callbacks: &old::Record, raw: &[u8]) -> Vec<u8> {
    [
        policy.encode().unwrap(),
        callbacks.encode().unwrap(),
        raw.to_vec(),
    ]
    .concat()
}

#[test]
fn forward_duration_roundtrips_three_lines_preserving_original_prefix() {
    let (bootstrap, policy, callbacks, record) = fixture();
    let original = [policy.encode().unwrap(), callbacks.encode().unwrap()].concat();
    let raw = stderr(&policy, &callbacks, &record);
    assert!(raw.starts_with(&original));
    assert_eq!(raw.iter().filter(|byte| **byte == b'\n').count(), 3);
    assert_eq!(
        old::decode_stderr(&original, &bootstrap, [7; 32], [8; 32]).unwrap(),
        (policy.clone(), callbacks.clone())
    );
    assert!(old::decode_stderr(&raw, &bootstrap, [7; 32], [8; 32]).is_err());
    assert_eq!(
        decode_stderr(&raw, &bootstrap, [7; 32], [8; 32]).unwrap(),
        (policy, callbacks, record)
    );
}

#[test]
fn forward_duration_requires_fixed_nine_phase_order() {
    let (_, policy, callbacks, record) = fixture();
    assert_eq!(PHASE_COUNT, 9);
    assert_eq!(
        PHASE_ORDER,
        [
            "input",
            "metadata",
            "embedding",
            "bank",
            "layers",
            "tail",
            "frame",
            "fence",
            "commit"
        ]
    );
    for index in 0..PHASE_COUNT {
        let mut changed = record.clone();
        changed.phase_order[index] = "other".into();
        assert!(changed.validate(&policy, &callbacks).is_err());
    }
    let mut changed = record.clone();
    changed.phase_order.swap(3, 4);
    assert!(changed.validate(&policy, &callbacks).is_err());
}

#[test]
fn forward_duration_requires_forty_unique_ordered_rows_on_both_sides() {
    let (_, policy, callbacks, record) = fixture();
    for which in 0..6 {
        let mut rows = record.forwards.clone();
        let mut old = callbacks.clone();
        match which {
            0 => {
                rows.pop();
            }
            1 => rows.push(rows[39]),
            2 => rows[3].position = 2,
            3 => rows.swap(2, 3),
            4 => {
                old.forwards.pop();
            }
            _ => old.forwards[3].position = 2,
        }
        assert!(validate_rows(&rows, &old).is_err());
        assert!(Record::new(&policy, &old, rows).is_err());
    }
}

#[test]
fn forward_duration_times_first_use_but_does_not_create_first_use_callbacks() {
    let (_, policy, callbacks, record) = fixture();
    for index in 0..2 {
        assert!(callbacks.forwards[index].measured.is_none());
        assert!(
            record.forwards[index]
                .phase_ns
                .iter()
                .all(|value| *value == 1)
        );
        assert_eq!(record.forwards[index].forward_body_ns, 9);
    }
    for which in 0..2 {
        let mut changed = callbacks.clone();
        if which == 0 {
            changed.forwards[0].measured = changed.forwards[2].measured;
        } else {
            changed.forwards[2].measured = None;
        }
        assert!(validate_rows(&record.forwards, &changed).is_err());
        assert!(Record::new(&policy, &changed, record.forwards.clone()).is_err());
    }
}

#[test]
fn forward_duration_checks_exact_sum_overflow_position_and_row_bound() {
    let (_, _, _, record) = fixture();
    for which in 0..5 {
        let mut row = record.forwards[0];
        match which {
            0 => row.forward_body_ns -= 1,
            1 => row.forward_body_ns += 1,
            2 => row.phase_ns[0] = u64::MAX,
            3 => {
                row.phase_ns = [0; PHASE_COUNT];
                row.phase_ns[0] = WHOLE_NS + 1;
                row.forward_body_ns = WHOLE_NS + 1;
            }
            _ => row.position = 40,
        }
        assert!(row.validate().is_err());
    }
}

#[test]
fn forward_duration_checks_whole_case_sum_not_only_individual_bounds() {
    let (_, policy, callbacks, record) = fixture();
    let mut rows = record.forwards.clone();
    rows[0].phase_ns = [0; PHASE_COUNT];
    rows[0].phase_ns[0] = WHOLE_NS;
    rows[0].forward_body_ns = WHOLE_NS;
    for (row, callback) in rows.iter().zip(&callbacks.forwards) {
        row.validate_against(callback).unwrap();
    }
    assert!(validate_rows(&rows, &callbacks).is_err());
    assert!(Record::new(&policy, &callbacks, rows).is_err());
}

#[test]
fn forward_duration_checks_each_nested_scope_without_double_counting() {
    let (_, policy, callbacks, record) = fixture();
    let measured = callbacks.forwards[2].measured.unwrap();
    for (index, minimum) in [
        (3, measured.bank_guarded_body_ns),
        (4, measured.layers.elapsed_subtotal().unwrap()),
        (5, measured.tail.elapsed_subtotal().unwrap()),
    ] {
        let mut rows = record.forwards.clone();
        rows[2].phase_ns[index] = minimum - 1;
        rows[2].forward_body_ns = rows[2].phase_ns.iter().copied().sum();
        rows[2].validate().unwrap();
        assert!(rows[2].validate_against(&callbacks.forwards[2]).is_err());
        assert!(Record::new(&policy, &callbacks, rows).is_err());
    }
    assert_eq!(
        record.forwards[2].phase_ns[3],
        measured.bank_guarded_body_ns
    );
    assert!(measured.bank.elapsed_subtotal().unwrap() < record.forwards[2].phase_ns[3]);
    record.validate(&policy, &callbacks).unwrap();
}

#[test]
fn forward_duration_does_not_launder_invalid_original_callback_counts() {
    let (_, policy, callbacks, record) = fixture();
    let mut changed = callbacks.clone();
    changed.forwards[2]
        .measured
        .as_mut()
        .unwrap()
        .tail
        .root_generation
        .calls += 1;
    assert!(changed.validate(&policy).is_err());
    let mut repinned = record.clone();
    repinned.currentness_record_sha256 = Sha256::digest(changed.encode().unwrap()).into();
    assert!(repinned.validate(&policy, &changed).is_err());
    assert!(Record::new(&policy, &changed, rows_for(&changed)).is_err());
}

#[test]
fn forward_duration_authenticates_exact_lf_inclusive_callback_record() {
    let (_, policy, callbacks, record) = fixture();
    let raw = callbacks.encode().unwrap();
    let expected: [u8; 32] = Sha256::digest(&raw).into();
    assert_eq!(record.currentness_record_sha256, expected);
    let mut changed = record.clone();
    changed.currentness_record_sha256 = Sha256::digest(&raw[..raw.len() - 1]).into();
    assert!(changed.validate(&policy, &callbacks).is_err());
    let mut old = callbacks.clone();
    old.forwards[2]
        .measured
        .as_mut()
        .unwrap()
        .bank_guarded_body_ns += 1;
    old.validate(&policy).unwrap();
    let mut expanded = record.clone();
    expanded.forwards = rows_for(&old);
    assert!(expanded.validate(&policy, &old).is_err());
    expanded.currentness_record_sha256 = Sha256::digest(old.encode().unwrap()).into();
    expanded.validate(&policy, &old).unwrap();
}

#[test]
fn forward_duration_joins_all_record_and_external_close_identities() {
    let (bootstrap, policy, callbacks, record) = fixture();
    for which in 0..6 {
        let mut changed = record.clone();
        match which {
            0 => changed.schema = "other".into(),
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
    let raw = stderr(&policy, &callbacks, &record);
    assert!(decode_stderr(&raw, &bootstrap, [9; 32], [8; 32]).is_err());
    assert!(decode_stderr(&raw, &bootstrap, [7; 32], [9; 32]).is_err());
    let mut changed_bootstrap = bootstrap.clone();
    changed_bootstrap.sequence.scope.session[0] ^= 1;
    assert!(decode_stderr(&raw, &changed_bootstrap, [7; 32], [8; 32]).is_err());
}

#[test]
fn forward_duration_refuses_claim_and_instrumentation_flag_drift() {
    let (_, policy, callbacks, record) = fixture();
    for key in [
        "instrumented",
        "host_elapsed_nanoseconds",
        "disjoint_phases",
        "currentness_durations_nested",
        "gpu_timing",
        "numerical_acceptance",
        "performance_claim",
        "execution_authority",
    ] {
        let mut value = serde_json::to_value(&record).unwrap();
        value[key] = serde_json::json!(!value[key].as_bool().unwrap());
        let changed: Record = serde_json::from_value(value).unwrap();
        assert!(changed.validate(&policy, &callbacks).is_err());
    }
}

#[test]
fn forward_duration_refuses_noncanonical_duplicate_unknown_and_record_framing() {
    let (bootstrap, policy, callbacks, record) = fixture();
    let third = record.encode().unwrap();
    let mut unknown = serde_json::to_value(&record).unwrap();
    unknown["extra"] = serde_json::json!(0);
    assert!(serde_json::from_value::<Record>(unknown).is_err());
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
                &replace_third(&policy, &callbacks, &changed),
                &bootstrap,
                [7; 32],
                [8; 32]
            )
            .is_err()
        );
    }
    let original = [policy.encode().unwrap(), callbacks.encode().unwrap()].concat();
    assert!(decode_stderr(&original, &bootstrap, [7; 32], [8; 32]).is_err());
    assert!(
        decode_stderr(
            &[callbacks.encode().unwrap(), policy.encode().unwrap(), third].concat(),
            &bootstrap,
            [7; 32],
            [8; 32]
        )
        .is_err()
    );
}

#[test]
fn forward_duration_requires_integer_arrays_and_rejects_numeric_aliases() {
    let (_, _, _, record) = fixture();
    for bad in [
        serde_json::json!(true),
        serde_json::json!(-1),
        serde_json::json!(1.5),
    ] {
        let mut value = serde_json::to_value(&record).unwrap();
        value["forwards"][0]["phase_ns"][0] = bad;
        assert!(serde_json::from_value::<Record>(value).is_err());
    }
    for len in [PHASE_COUNT - 1, PHASE_COUNT + 1] {
        let mut value = serde_json::to_value(&record).unwrap();
        value["forwards"][0]["phase_ns"] = serde_json::json!(vec![1_u64; len]);
        assert!(serde_json::from_value::<Record>(value).is_err());
    }
    let mut value = serde_json::to_value(&record).unwrap();
    value["forwards"][0]["position"] = serde_json::json!(u64::MAX);
    assert!(serde_json::from_value::<Record>(value).is_err());
    let overflow = b"{\"position\":0,\"phase_ns\":[18446744073709551616,0,0,0,0,0,0,0,0],\"forward_body_ns\":0}";
    assert!(serde_json::from_slice::<ForwardRow>(overflow).is_err());
}

#[test]
fn forward_duration_preserves_unchanged_combined_and_third_record_caps() {
    let (bootstrap, policy, callbacks, record) = fixture();
    assert_eq!(STDERR_MAX_BYTES, old::STDERR_MAX_BYTES);
    assert_eq!(STDERR_MAX_BYTES, 69_632);
    assert_eq!(MAX_BYTES, 32_768);
    let mut oversized = record.clone();
    oversized.schema = "x".repeat(MAX_BYTES);
    assert!(oversized.encode().is_err());
    assert!(
        decode_stderr(
            &replace_third(&policy, &callbacks, &vec![b' '; MAX_BYTES + 1]),
            &bootstrap,
            [7; 32],
            [8; 32]
        )
        .is_err()
    );
    assert!(
        decode_stderr(
            &vec![b' '; STDERR_MAX_BYTES + 1],
            &bootstrap,
            [7; 32],
            [8; 32]
        )
        .is_err()
    );
}

#[test]
fn forward_duration_fixture_rows_and_record_cannot_alias_original_callback_data() {
    let (_, policy, callbacks, record) = fixture();
    let before = callbacks.encode().unwrap();
    let mut rows = rows_for(&callbacks);
    rows[2].phase_ns[0] += 10;
    rows[2].forward_body_ns += 10;
    let changed = Record::new(&policy, &callbacks, rows).unwrap();
    assert_ne!(changed.forwards, record.forwards);
    assert_eq!(callbacks.encode().unwrap(), before);
    record.validate(&policy, &callbacks).unwrap();
    changed.validate(&policy, &callbacks).unwrap();
}

#[test]
fn forward_duration_allows_zero_elapsed_without_inventing_zero_callback_calls() {
    let (_, policy, mut callbacks, _) = fixture();
    for row in &mut callbacks.forwards {
        if let Some(measured) = &mut row.measured {
            for scope in [&mut measured.bank, &mut measured.layers, &mut measured.tail] {
                scope.before.elapsed_ns = 0;
                scope.discover.elapsed_ns = 0;
                scope.after.elapsed_ns = 0;
                scope.root_generation.elapsed_ns = 0;
            }
            measured.bank_guarded_body_ns = 0;
        }
    }
    callbacks.validate(&policy).unwrap();
    let rows = (0..40)
        .map(|position| ForwardRow {
            position,
            phase_ns: [0; PHASE_COUNT],
            forward_body_ns: 0,
        })
        .collect();
    let record = Record::new(&policy, &callbacks, rows).unwrap();
    assert!(callbacks.forwards[2].measured.unwrap().bank.before.calls > 0);
    record.validate(&policy, &callbacks).unwrap();
}

#[test]
fn forward_duration_does_not_replace_original_policy_or_two_record_admission() {
    let (bootstrap, policy, callbacks, record) = fixture();
    let mut changed = policy.clone();
    changed.native_closed = false;
    assert!(
        decode_stderr(
            &stderr(&changed, &callbacks, &record),
            &bootstrap,
            [7; 32],
            [8; 32]
        )
        .is_err()
    );
    let mut changed_callbacks = callbacks.clone();
    changed_callbacks.forwards[2]
        .measured
        .as_mut()
        .unwrap()
        .layers
        .before
        .elapsed_ns = WHOLE_NS + 1;
    let mut repinned = record.clone();
    repinned.currentness_record_sha256 = Sha256::digest(changed_callbacks.encode().unwrap()).into();
    assert!(
        decode_stderr(
            &stderr(&policy, &changed_callbacks, &repinned),
            &bootstrap,
            [7; 32],
            [8; 32]
        )
        .is_err()
    );
}
