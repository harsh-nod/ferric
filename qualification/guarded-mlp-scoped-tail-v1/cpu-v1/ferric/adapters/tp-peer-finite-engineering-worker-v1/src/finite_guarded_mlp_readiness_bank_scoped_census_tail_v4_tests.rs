use super::*;
fn counts() -> Counts {
    Counts {
        tails: TailCounts {
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
        layers: LayerCounts {
            ordinary_layers: 72,
            scoped_layers: 1368,
            scoped_layers_by_forward: (0..40).map(|p| if p < 2 { 0 } else { 36 }).collect(),
            full_discoveries: 2736,
            local_checkpoints: 6840 + 21888,
            before_calls: 15048 + 21888,
            after_calls: 15048 + 21888,
            generation_probes: 17784 + 43776,
        },
        census: CensusCounts {
            warm_layers: 1368,
            preflights: 2736,
            rank_checkpoints: 21888,
            owner_counts: [787, 783],
        },
        banks: BankCounts {
            ordinary_initial_banks: 2,
            scoped_rearms: 38,
            scoped_rearms_by_forward: (0..40).map(|p| if p < 2 { 0 } else { 1 }).collect(),
            final_generations: [20, 20],
            full_discoveries: 76,
            local_checkpoints: 190,
            before_calls: 532,
            after_calls: 532,
            generation_probes: 494,
        },
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
fn tail_scoped_policy_exact_closed_record_roundtrips() {
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
    assert!(
        crate::finite_guarded_mlp_readiness_scoped_v1::PolicyRecord::decode(
            &value.encode().unwrap(),
            &b,
            [7; 32],
            [8; 32]
        )
        .is_err()
    );
    assert!(value.encode().unwrap().len() <= MAX_BYTES);
    let shared =
        crate::finite_guarded_mlp_readiness_shared_v1::PolicyRecord::new(&b, [7; 32], [8; 32])
            .unwrap()
            .encode()
            .unwrap();
    assert!(PolicyRecord::decode(&shared, &b, [7; 32], [8; 32]).is_err());
}
#[test]
fn tail_scoped_policy_refuses_every_policy_and_authority_drift() {
    let b = bootstrap();
    let value = PolicyRecord::new(&b, [7; 32], [8; 32], counts()).unwrap();
    for key in [
        "scoped_warm_currentness",
        "scoped_bank_rearm",
        "full_entry_exit_per_scoped_bank",
        "allocation_preflights_outside_windows",
        "scoped_capacity_census",
        "allocation_preflights_changed",
        "full_entry_exit_per_scoped_layer",
        "participant_local_between_boundaries",
        "scope_includes_prefix_mlp_hidden",
        "scoped_tail",
        "full_entry_exit_per_scoped_tail",
        "scope_includes_tail_dispatch_and_readback",
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
fn tail_scoped_policy_refuses_scope_worker_transcript_and_counts_drift() {
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
fn tail_scoped_policy_requires_one_exact_bounded_record() {
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
fn tail_scoped_policy_refuses_other_profiles_and_missing_identity() {
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
fn tail_scoped_policy_refuses_boolean_integer_substitution() {
    let b = bootstrap();
    let mut value =
        serde_json::to_value(PolicyRecord::new(&b, [7; 32], [8; 32], counts()).unwrap()).unwrap();
    value["completed_forwards"] = serde_json::json!(true);
    let mut raw = serde_json::to_vec(&value).unwrap();
    raw.push(b'\n');
    assert!(PolicyRecord::decode(&raw, &b, [7; 32], [8; 32]).is_err());
}

#[test]
fn tail_scoped_policy_checks_every_call_count_and_unbounded_counter_overflow() {
    let b = bootstrap();
    for mutation in 0..9 {
        let mut c = counts();
        match mutation {
            0 => c.layers.ordinary_layers -= 1,
            1 => c.layers.scoped_layers -= 1,
            2 => c.layers.scoped_layers_by_forward[1] = 36,
            3 => c.layers.full_discoveries -= 1,
            4 => c.layers.before_calls += 1,
            5 => c.layers.generation_probes += 1,
            6 => c.layers.local_checkpoints = u64::MAX,
            7 => {
                c.layers.before_calls = 0;
                c.layers.after_calls = 0;
            }
            _ => {
                c.layers.before_calls = u64::MAX;
                c.layers.after_calls = u64::MAX;
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

#[test]
fn tail_scoped_policy_checks_separate_bank_census_generations_and_overflow() {
    let b = bootstrap();
    for mutation in 0..12 {
        let mut c = counts();
        match mutation {
            0 => c.banks.ordinary_initial_banks -= 1,
            1 => c.banks.scoped_rearms -= 1,
            2 => c.banks.scoped_rearms_by_forward[1] = 1,
            3 => c.banks.final_generations[0] -= 1,
            4 => c.banks.full_discoveries -= 1,
            5 => c.banks.local_checkpoints = 0,
            6 => c.banks.before_calls += 1,
            7 => c.banks.after_calls += 1,
            8 => c.banks.generation_probes += 1,
            9 => c.banks.local_checkpoints = u64::MAX,
            10 => c.banks.scoped_rearms_by_forward.pop().map(|_| ()).unwrap(),
            _ => c.banks.final_generations[1] += 1,
        }
        assert!(
            PolicyRecord::new(&b, [7; 32], [8; 32], c).is_err(),
            "{mutation}"
        );
    }
    let mut c = counts();
    c.layers.local_checkpoints = (u64::MAX - 4 * 1368 - 43776) / 2 + 21888;
    c.layers.before_calls = 2 * (c.layers.local_checkpoints - 21888) + 4 * 1368 + 21888;
    c.layers.after_calls = c.layers.before_calls;
    c.layers.generation_probes = 2 * c.layers.local_checkpoints + 3 * 1368;
    c.banks.local_checkpoints = (u64::MAX - 4 * 38) / 2;
    c.banks.before_calls = 2 * c.banks.local_checkpoints + 4 * 38;
    c.banks.after_calls = c.banks.before_calls;
    c.banks.generation_probes = 2 * c.banks.local_checkpoints + 3 * 38;
    c.tails.local_checkpoints = (u64::MAX - 608) / 2;
    c.tails.before_calls = c.tails.local_checkpoints + 608;
    c.tails.after_calls = c.tails.before_calls;
    c.tails.generation_probes = c.tails.local_checkpoints * 2 + 114;
    let raw = PolicyRecord::new(&b, [255; 32], [255; 32], c)
        .unwrap()
        .encode()
        .unwrap();
    assert!(raw.len() <= MAX_BYTES);
    assert!(PolicyRecord::decode(&raw, &b, [255; 32], [255; 32]).is_ok());
}

#[test]
fn tail_scoped_policy_rejects_subset_owner_and_legacy_bank_records() {
    let b = bootstrap();
    for mutation in 0..8 {
        let mut c = counts();
        match mutation {
            0 => c.census.warm_layers -= 1,
            1 => c.census.preflights -= 1,
            2 => c.census.rank_checkpoints += 1,
            3 => c.census.owner_counts[0] = 0,
            4 => c.census.owner_counts[1] = 2049,
            5 => c.layers.local_checkpoints = 21888,
            6 => c.layers.before_calls = 21887,
            _ => c.layers.generation_probes = 43775,
        }
        assert!(PolicyRecord::new(&b, [7; 32], [8; 32], c).is_err());
    }
    let c = counts();
    let old = crate::finite_guarded_mlp_readiness_bank_scoped_v2::PolicyRecord::new(
        &b,
        [7; 32],
        [8; 32],
        crate::finite_guarded_mlp_readiness_bank_scoped_v2::Counts {
            layers: c.layers,
            banks: c.banks,
        },
    )
    .unwrap()
    .encode()
    .unwrap();
    assert!(PolicyRecord::decode(&old, &b, [7; 32], [8; 32]).is_err());
    let new = PolicyRecord::new(&b, [7; 32], [8; 32], counts())
        .unwrap()
        .encode()
        .unwrap();
    assert!(
        crate::finite_guarded_mlp_readiness_bank_scoped_v2::PolicyRecord::decode(
            &new, &b, [7; 32], [8; 32]
        )
        .is_err()
    );
}

#[test]
fn tail_scoped_policy_checks_each_tail_count_overflow_and_variable_polls() {
    let b = bootstrap();
    for mutation in 0..11 {
        let mut c = counts();
        match mutation {
            0 => c.tails.ordinary_tails -= 1,
            1 => c.tails.scoped_tails -= 1,
            2 => c.tails.dispatches -= 1,
            3 => c.tails.readbacks -= 1,
            4 => c.tails.readback_bytes -= 1,
            5 => c.tails.full_discoveries -= 1,
            6 => c.tails.local_checkpoints -= 1,
            7 => c.tails.before_calls += 1,
            8 => c.tails.after_calls += 1,
            9 => c.tails.generation_probes += 1,
            _ => c.tails.local_checkpoints = u64::MAX,
        }
        assert!(
            PolicyRecord::new(&b, [7; 32], [8; 32], c).is_err(),
            "{mutation}"
        );
    }
    let mut c = counts();
    c.tails.local_checkpoints += 7;
    c.tails.before_calls += 7;
    c.tails.after_calls += 7;
    c.tails.generation_probes += 14;
    let record = PolicyRecord::new(&b, [7; 32], [8; 32], c).unwrap();
    assert!(PolicyRecord::decode(&record.encode().unwrap(), &b, [7; 32], [8; 32]).is_ok());
}
#[test]
fn tail_scoped_policy_refuses_census_v3_and_integer_type_or_nested_field_drift() {
    let b = bootstrap();
    let c = counts();
    let old = crate::finite_guarded_mlp_readiness_bank_scoped_census_v3::PolicyRecord::new(
        &b,
        [7; 32],
        [8; 32],
        crate::finite_guarded_mlp_readiness_bank_scoped_census_v3::Counts {
            layers: c.layers,
            banks: c.banks,
            census: c.census,
        },
    )
    .unwrap()
    .encode()
    .unwrap();
    assert!(PolicyRecord::decode(&old, &b, [7; 32], [8; 32]).is_err());
    let new = PolicyRecord::new(&b, [7; 32], [8; 32], counts())
        .unwrap()
        .encode()
        .unwrap();
    assert!(
        crate::finite_guarded_mlp_readiness_bank_scoped_census_v3::PolicyRecord::decode(
            &new, &b, [7; 32], [8; 32]
        )
        .is_err()
    );
    for v in [
        serde_json::json!(true),
        serde_json::json!(38.0),
        serde_json::json!(-1),
    ] {
        let mut body: serde_json::Value = serde_json::from_slice(&new).unwrap();
        body["counts"]["tails"]["scoped_tails"] = v;
        let mut raw = serde_json::to_vec(&body).unwrap();
        raw.push(b'\n');
        assert!(PolicyRecord::decode(&raw, &b, [7; 32], [8; 32]).is_err());
    }
    let mut body: serde_json::Value = serde_json::from_slice(&new).unwrap();
    body["counts"]["tails"]["extra"] = serde_json::json!(0);
    let mut raw = serde_json::to_vec(&body).unwrap();
    raw.push(b'\n');
    assert!(PolicyRecord::decode(&raw, &b, [7; 32], [8; 32]).is_err());
}
