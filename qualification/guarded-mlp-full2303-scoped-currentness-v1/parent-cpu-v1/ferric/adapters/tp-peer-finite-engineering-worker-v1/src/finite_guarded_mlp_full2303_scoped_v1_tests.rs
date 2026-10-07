use super::*;
fn counts() -> Counts {
    Counts {
        ordinary_layers: 72,
        scoped_layers: 82_836,
        full_discoveries: 2 * 82_836,
        local_checkpoints: 5 * 82_836,
        before_calls: 11 * 82_836,
        after_calls: 11 * 82_836,
        generation_probes: 13 * 82_836,
    }
}
fn bootstrap() -> full::Bootstrap {
    full::Bootstrap {
        schema: full::SCHEMA.into(),
        child_deadline_ms: full::MAX_DEADLINE_MS,
        sequence: long::tests::bootstrap(long::Profile::Full2303),
    }
}
fn record() -> PolicyRecord {
    PolicyRecord::new(&bootstrap(), [7; 32], &[9; 256], [8; 32], counts()).unwrap()
}
#[test]
fn full_scoped_policy_compact_closed_record_and_exact_own_history_roundtrip() {
    let b = bootstrap();
    let value = record();
    let raw = value.encode().unwrap();
    assert!(raw.len() <= MAX_BYTES);
    assert_eq!(
        PolicyRecord::decode(&raw, &b, [7; 32], &[9; 256], [8; 32]).unwrap(),
        value
    );
    let bytes: Vec<_> = [9u32; 256].iter().flat_map(|n| n.to_le_bytes()).collect();
    assert_eq!(
        value.generated_tokens_sha256,
        <[u8; 32]>::from(Sha256::digest(bytes))
    );
    assert_eq!(value.capture_positions, [0, 2047, 2048, 2302]);
    assert_eq!(
        (
            value.completed_forwards,
            value.prompt_positions,
            value.generated_token_count
        ),
        (2303, 2048, 256)
    );
    assert_eq!(
        (value.first_scoped_position, value.layers_per_forward),
        (2, 36)
    );
    assert!(value.full_long_workload && !value.numerical_acceptance);
    for index in [0, 1, 254, 255] {
        let mut changed = [9; 256];
        changed[index] ^= 1;
        assert!(PolicyRecord::decode(&raw, &b, [7; 32], &changed, [8; 32]).is_err());
    }
    for bad in [vec![9; 255], vec![9; 257], vec![151_936; 256]] {
        assert!(PolicyRecord::new(&b, [7; 32], &bad, [8; 32], counts()).is_err());
    }
}
#[test]
fn full_scoped_policy_refuses_every_policy_and_authority_drift() {
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
        let mut value = serde_json::to_value(record()).unwrap();
        value[key] = serde_json::json!(!value[key].as_bool().unwrap());
        let changed: PolicyRecord = serde_json::from_value(value).unwrap();
        assert!(
            changed
                .validate(&bootstrap(), [7; 32], &[9; 256], [8; 32])
                .is_err(),
            "{key}"
        );
    }
}
#[test]
fn full_scoped_policy_refuses_scope_identity_extent_and_transcript_drift() {
    let b = bootstrap();
    for index in 0..15 {
        let mut changed = record();
        match index {
            0 => changed.session[0] ^= 1,
            1 => changed.device_ids.swap(0, 1),
            2 => changed.child_pid += 1,
            3 => changed.profile_sha256[0] ^= 1,
            4 => changed.registration_sha256[0] ^= 1,
            5 => changed.completed_forwards -= 1,
            6 => changed.generated_token_count -= 1,
            7 => changed.generated_tokens_sha256[0] ^= 1,
            8 => changed.capture_positions[1] = 5,
            9 => changed.prompt_positions -= 1,
            10 => changed.first_scoped_position = 1,
            11 => changed.layers_per_forward = 35,
            12 => changed.schema.push('x'),
            13 => changed.execution_profile.push('x'),
            _ => changed.transcript_sha256[0] ^= 1,
        }
        assert!(changed.validate(&b, [7; 32], &[9; 256], [8; 32]).is_err());
    }
    assert!(record().validate(&b, [6; 32], &[9; 256], [8; 32]).is_err());
    assert!(record().validate(&b, [7; 32], &[9; 256], [9; 32]).is_err());
    for profile in [
        long::Profile::Readiness40,
        long::Profile::Readiness40Position5,
    ] {
        let mut other = bootstrap();
        other.sequence = long::tests::bootstrap(profile);
        assert!(PolicyRecord::new(&other, [7; 32], &[9; 256], [8; 32], counts()).is_err());
    }
    assert!(PolicyRecord::new(&b, [0; 32], &[9; 256], [8; 32], counts()).is_err());
    assert!(PolicyRecord::new(&b, [7; 32], &[9; 256], [0; 32], counts()).is_err());
}
#[test]
fn full_scoped_policy_requires_canonical_single_record_and_refuses_write_flush_failures() {
    let raw = record().encode().unwrap();
    for bad in [
        Vec::new(),
        raw[..raw.len() - 1].to_vec(),
        [raw.clone(), raw.clone()].concat(),
        [raw.clone(), vec![b' ']].concat(),
        vec![b' '; MAX_BYTES + 1],
    ] {
        assert!(PolicyRecord::decode(&bad, &bootstrap(), [7; 32], &[9; 256], [8; 32]).is_err());
    }
    for (key, value) in [
        ("unknown", serde_json::json!(true)),
        ("completed_forwards", serde_json::json!(true)),
        ("generated_token_count", serde_json::json!(256.0)),
    ] {
        let mut object = serde_json::to_value(record()).unwrap();
        object[key] = value;
        let mut bad = serde_json::to_vec(&object).unwrap();
        bad.push(b'\n');
        assert!(PolicyRecord::decode(&bad, &bootstrap(), [7; 32], &[9; 256], [8; 32]).is_err());
    }
    struct Refuse(bool);
    impl io::Write for Refuse {
        fn write(&mut self, raw: &[u8]) -> io::Result<usize> {
            if self.0 {
                Ok(raw.len())
            } else {
                Err(io::Error::other("write"))
            }
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("flush"))
        }
    }
    assert!(record().write_to(&mut Refuse(false)).is_err());
    assert!(record().write_to(&mut Refuse(true)).is_err());
}
#[test]
fn full_scoped_policy_checks_all_window_counts_and_checked_arithmetic_overflow() {
    for mutation in 0..9 {
        let mut c = counts();
        match mutation {
            0 => c.ordinary_layers -= 1,
            1 => c.scoped_layers -= 1,
            2 => c.local_checkpoints = 0,
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
        assert!(c.validate_closed().is_err(), "{mutation}");
    }
}
