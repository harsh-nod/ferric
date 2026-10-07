use super::*;
fn counts() -> Counts {
    Counts {
        ordinary_layers: 72,
        scoped_layers: 1368,
        scoped_layers_by_forward: (0..40).map(|p| if p < 2 { 0 } else { 36 }).collect(),
        full_discoveries: 2 * 1368,
        local_checkpoints: 5 * 1368,
        before_calls: 11 * 1368,
        after_calls: 11 * 1368,
        generation_probes: 13 * 1368,
    }
}
fn bootstrap() -> ready::Bootstrap {
    ready::Bootstrap {
        schema: ready::POSITION5_SCHEMA.into(),
        child_deadline_ms: 60_000,
        sequence: long::tests::bootstrap(long::Profile::Readiness40Position5),
    }
}
#[test]
fn scoped_warm_policy_exact_closed_record_roundtrips() {
    let b = bootstrap();
    let value = PolicyRecord::new(&b, [7; 32], [8; 32], counts()).unwrap();
    assert_eq!(
        PolicyRecord::decode(&value.encode().unwrap(), &b, [7; 32], [8; 32]).unwrap(),
        value
    );
    assert_eq!(value.capture_positions, [0, 5, 16, 39]);
    assert_eq!(value.completed_forwards, 40);
    assert!(value.generated_tokens.is_empty());
    assert!(
        crate::finite_guarded_mlp_readiness_shared_v1::PolicyRecord::decode(
            &value.encode().unwrap(),
            &b,
            [7; 32],
            [8; 32],
        )
        .is_err()
    );
    let shared =
        crate::finite_guarded_mlp_readiness_shared_v1::PolicyRecord::new(&b, [7; 32], [8; 32])
            .unwrap()
            .encode()
            .unwrap();
    assert!(PolicyRecord::decode(&shared, &b, [7; 32], [8; 32]).is_err());
}
#[test]
fn scoped_warm_policy_refuses_every_policy_and_authority_drift() {
    let b = bootstrap();
    let value = PolicyRecord::new(&b, [7; 32], [8; 32], counts()).unwrap();
    for key in [
        "scoped_warm_currentness",
        "full_entry_exit_per_scoped_layer",
        "participant_local_between_boundaries",
        "scope_includes_prefix_mlp_hidden",
        "temporal_equivalent_to_full",
        "default_group_policy_unchanged",
        "shared_full_currentness",
        "cache_kernel_admission",
        "operational_currentness",
        "host_observer",
        "paired_hidden_reads",
        "paired_terminal",
        "native_closed",
        "full_long_workload",
        "numerical_acceptance",
        "performance_claim",
        "production_authority",
    ] {
        let mut changed = serde_json::to_value(&value).unwrap();
        changed[key] = serde_json::json!(!changed[key].as_bool().unwrap());
        let changed: PolicyRecord = serde_json::from_value(changed).unwrap();
        assert!(changed.validate(&b, [7; 32], [8; 32]).is_err(), "{key}");
    }
}
#[test]
fn scoped_warm_policy_refuses_scope_worker_transcript_and_counts_drift() {
    let b = bootstrap();
    let value = PolicyRecord::new(&b, [7; 32], [8; 32], counts()).unwrap();
    for index in 0..9 {
        let mut changed = value.clone();
        match index {
            0 => changed.session[0] ^= 1,
            1 => changed.device_ids.swap(0, 1),
            2 => changed.child_pid += 1,
            3 => changed.profile_sha256[0] ^= 1,
            4 => changed.registration_sha256[0] ^= 1,
            5 => changed.completed_forwards = 39,
            6 => changed.generated_tokens.push(7),
            7 => changed.capture_positions[1] = 15,
            _ => changed.schema.push('x'),
        }
        assert!(changed.validate(&b, [7; 32], [8; 32]).is_err());
    }
    assert!(value.validate(&b, [6; 32], [8; 32]).is_err());
    assert!(value.validate(&b, [7; 32], [9; 32]).is_err());
}
#[test]
fn scoped_warm_policy_requires_one_exact_bounded_record() {
    let b = bootstrap();
    let raw = PolicyRecord::new(&b, [7; 32], [8; 32], counts())
        .unwrap()
        .encode()
        .unwrap();
    for bad in [
        Vec::new(),
        raw[..raw.len() - 1].to_vec(),
        [raw.clone(), raw.clone()].concat(),
        [raw.clone(), vec![b' ']].concat(),
        vec![b' '; MAX_BYTES + 1],
    ] {
        assert!(PolicyRecord::decode(&bad, &b, [7; 32], [8; 32]).is_err());
    }
    let mut extra: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    extra["extra"] = serde_json::json!(true);
    let mut bad = serde_json::to_vec(&extra).unwrap();
    bad.push(b'\n');
    assert!(PolicyRecord::decode(&bad, &b, [7; 32], [8; 32]).is_err());
    struct Refuse {
        flush: bool,
    }
    impl std::io::Write for Refuse {
        fn write(&mut self, raw: &[u8]) -> io::Result<usize> {
            if self.flush {
                Ok(raw.len())
            } else {
                Err(io::Error::other("write refused"))
            }
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("flush refused"))
        }
    }
    let record = PolicyRecord::new(&b, [7; 32], [8; 32], counts()).unwrap();
    assert!(record.write_to(&mut Refuse { flush: false }).is_err());
    assert!(record.write_to(&mut Refuse { flush: true }).is_err());
}
#[test]
fn scoped_warm_policy_refuses_other_profiles_and_missing_identity() {
    for profile in [long::Profile::Readiness40, long::Profile::Full2303] {
        let b = ready::Bootstrap {
            schema: ready::SCHEMA.into(),
            child_deadline_ms: 60_000,
            sequence: long::tests::bootstrap(profile),
        };
        assert!(PolicyRecord::new(&b, [7; 32], [8; 32], counts()).is_err());
    }
    assert!(PolicyRecord::new(&bootstrap(), [0; 32], [8; 32], counts()).is_err());
    assert!(PolicyRecord::new(&bootstrap(), [7; 32], [0; 32], counts()).is_err());
}
#[test]
fn scoped_warm_policy_refuses_boolean_integer_substitution() {
    let b = bootstrap();
    let mut value =
        serde_json::to_value(PolicyRecord::new(&b, [7; 32], [8; 32], counts()).unwrap()).unwrap();
    value["completed_forwards"] = serde_json::json!(true);
    let mut raw = serde_json::to_vec(&value).unwrap();
    raw.push(b'\n');
    assert!(PolicyRecord::decode(&raw, &b, [7; 32], [8; 32]).is_err());
}

#[test]
fn scoped_warm_policy_checks_every_call_count_and_unbounded_counter_overflow() {
    let b = bootstrap();
    for mutation in 0..9 {
        let mut c = counts();
        match mutation {
            0 => c.ordinary_layers -= 1,
            1 => c.scoped_layers -= 1,
            2 => c.scoped_layers_by_forward[1] = 36,
            3 => c.full_discoveries -= 1,
            4 => c.before_calls += 1,
            5 => c.generation_probes += 1,
            6 => c.local_checkpoints = u64::MAX,
            7 => {
                c.before_calls = 0;
                c.after_calls = 0;
            }
            _ => {
                c.before_calls = u64::MAX;
                c.after_calls = u64::MAX;
            }
        }
        assert!(
            PolicyRecord::new(&b, [7; 32], [8; 32], c).is_err(),
            "{mutation}"
        );
    }
    let mut changed = PolicyRecord::new(&b, [7; 32], [8; 32], counts()).unwrap();
    changed.execution_profile = crate::finite_guarded_mlp_readiness_shared_v1::SCHEMA.into();
    assert!(changed.validate(&b, [7; 32], [8; 32]).is_err());
}
