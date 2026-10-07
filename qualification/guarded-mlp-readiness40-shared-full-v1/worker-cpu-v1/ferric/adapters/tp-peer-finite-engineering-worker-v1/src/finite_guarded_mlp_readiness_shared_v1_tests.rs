use super::*;
fn bootstrap() -> ready::Bootstrap {
    ready::Bootstrap {
        schema: ready::POSITION5_SCHEMA.into(),
        child_deadline_ms: 60_000,
        sequence: long::tests::bootstrap(long::Profile::Readiness40Position5),
    }
}
#[test]
fn shared_full_policy_exact_closed_record_roundtrips() {
    let b = bootstrap();
    let value = PolicyRecord::new(&b, [7; 32], [8; 32]).unwrap();
    assert_eq!(
        PolicyRecord::decode(&value.encode().unwrap(), &b, [7; 32], [8; 32]).unwrap(),
        value
    );
    assert_eq!(value.capture_positions, [0, 5, 16, 39]);
    assert_eq!(value.completed_forwards, 40);
    assert!(value.generated_tokens.is_empty());
}
#[test]
fn shared_full_policy_refuses_every_policy_and_authority_drift() {
    let b = bootstrap();
    let value = PolicyRecord::new(&b, [7; 32], [8; 32]).unwrap();
    for key in [
        "shared_full_currentness",
        "cache_kernel_admission",
        "operational_currentness",
        "legacy_profile",
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
fn shared_full_policy_refuses_scope_worker_transcript_and_counts_drift() {
    let b = bootstrap();
    let value = PolicyRecord::new(&b, [7; 32], [8; 32]).unwrap();
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
fn shared_full_policy_requires_one_exact_bounded_record() {
    let b = bootstrap();
    let raw = PolicyRecord::new(&b, [7; 32], [8; 32])
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
    let record = PolicyRecord::new(&b, [7; 32], [8; 32]).unwrap();
    assert!(record.write_to(&mut Refuse { flush: false }).is_err());
    assert!(record.write_to(&mut Refuse { flush: true }).is_err());
}
#[test]
fn shared_full_policy_refuses_other_profiles_and_missing_identity() {
    for profile in [long::Profile::Readiness40, long::Profile::Full2303] {
        let b = ready::Bootstrap {
            schema: ready::SCHEMA.into(),
            child_deadline_ms: 60_000,
            sequence: long::tests::bootstrap(profile),
        };
        assert!(PolicyRecord::new(&b, [7; 32], [8; 32]).is_err());
    }
    assert!(PolicyRecord::new(&bootstrap(), [0; 32], [8; 32]).is_err());
    assert!(PolicyRecord::new(&bootstrap(), [7; 32], [0; 32]).is_err());
}
#[test]
fn shared_full_policy_refuses_boolean_integer_substitution() {
    let b = bootstrap();
    let mut value = serde_json::to_value(PolicyRecord::new(&b, [7; 32], [8; 32]).unwrap()).unwrap();
    value["completed_forwards"] = serde_json::json!(true);
    let mut raw = serde_json::to_vec(&value).unwrap();
    raw.push(b'\n');
    assert!(PolicyRecord::decode(&raw, &b, [7; 32], [8; 32]).is_err());
}
