use super::*;
use crate::finite_guarded_mlp_readiness_bank_scoped_census_tail_v4::{
    BankCounts, CensusCounts, Counts, LayerCounts, TailCounts,
};
#[cfg(feature = "engineering-currentness-duration-diagnostics")]
use crate::finite_guarded_mlp_readiness_currentness_durations_v1 as diagnostic;
#[cfg(feature = "engineering-currentness-duration-diagnostics")]
use crate::finite_guarded_mlp_readiness_layer_durations_v2 as phases;
use std::sync::atomic::{AtomicU64, Ordering};

fn bootstrap() -> ready::Bootstrap {
    ready::Bootstrap {
        schema: ready::POSITION5_SCHEMA.into(),
        child_deadline_ms: 60_000,
        sequence: long::tests::bootstrap(long::Profile::Readiness40Position5),
    }
}
fn record(b: &ready::Bootstrap) -> PolicyRecord {
    PolicyRecord::new(
        b,
        [7; 32],
        [8; 32],
        Counts {
            layers: LayerCounts {
                ordinary_layers: 72,
                scoped_layers: 1368,
                scoped_layers_by_forward: (0..40).map(|p| if p < 2 { 0 } else { 36 }).collect(),
                full_discoveries: 2736,
                local_checkpoints: 21 * 1368,
                before_calls: 27 * 1368,
                after_calls: 27 * 1368,
                generation_probes: 45 * 1368,
            },
            banks: BankCounts {
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
            census: CensusCounts {
                warm_layers: 1368,
                preflights: 2736,
                rank_checkpoints: 21888,
                owner_counts: [787, 783],
            },
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
        },
    )
    .unwrap()
}
struct PolicyFile(FilePin);
#[cfg(feature = "engineering-currentness-duration-diagnostics")]
fn diagnostic_records(value: &PolicyRecord) -> (diagnostic::Record, phases::Record) {
    let (_, _, mut report) = diagnostic::tests::fixture();
    let tail = &mut report.forwards[2].measured.as_mut().unwrap().tail;
    tail.before.calls = value.counts.tails.before_calls - 37 * 43;
    tail.after.calls = value.counts.tails.after_calls - 37 * 43;
    tail.root_generation.calls = value.counts.tails.generation_probes - 37 * 57;
    let record = diagnostic::Record::new(value, report.forwards).unwrap();
    let phases = phases::Record::new(value, &record, phases::tests::rows_for(&record)).unwrap();
    (record, phases)
}
fn policy_file(value: &PolicyRecord) -> PolicyFile {
    #[cfg(not(feature = "engineering-currentness-duration-diagnostics"))]
    {
        PolicyFile::new(&value.encode().unwrap())
    }
    #[cfg(feature = "engineering-currentness-duration-diagnostics")]
    {
        let (record, phases) = diagnostic_records(value);
        PolicyFile::new(
            &[
                value.encode().unwrap(),
                record.encode().unwrap(),
                phases.encode().unwrap(),
            ]
            .concat(),
        )
    }
}
impl PolicyFile {
    fn new(raw: &[u8]) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "ferric-bank-scoped-census-tail-policy-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .unwrap();
        std::io::Write::write_all(&mut file, raw).unwrap();
        Self(FilePin {
            path,
            bytes: raw.len() as u64,
            sha256: hash(raw),
        })
    }
}
impl Drop for PolicyFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0.path);
    }
}

#[test]
fn bank_scoped_census_tail_parent_entry_preserves_legacy_signatures_and_exact_opt_in() {
    let _: fn(ReadinessConfig, bool) -> Result<Observation> = super::super::run;
    let _: fn(ReadinessConfig, bool) -> Result<Observation> = super::super::run_position5;
    let _: fn(ReadinessConfig, bool) -> Result<Observation> = super::super::run_causal;
    let _: fn(ReadinessConfig, bool) -> Result<super::super::TimedObservation> =
        super::super::run_position5_host_timing;
    let _: fn(ReadinessConfig, bool) -> Result<super::super::SharedFullObservation> =
        super::super::run_position5_shared_full;
    let _: fn(ReadinessConfig, bool) -> Result<super::super::SharedFullTimedObservation> =
        super::super::run_position5_shared_full_host_timing;
    let _: fn(ReadinessConfig, bool) -> Result<super::super::ScopedWarmObservation> =
        super::super::run_position5_scoped_warm;
    let _: fn(ReadinessConfig, bool) -> Result<super::super::ScopedWarmTimedObservation> =
        super::super::run_position5_scoped_warm_host_timing;
    let _: fn(ReadinessConfig, bool) -> Result<super::super::BankScopedWarmTimedObservation> =
        super::super::run_position5_bank_scoped_warm_host_timing;
    let _: fn(ReadinessConfig, bool) -> Result<super::super::BankScopedWarmCensusTimedObservation> =
        super::super::run_position5_bank_scoped_census_host_timing;
    let _: fn(
        ReadinessConfig,
        bool,
    ) -> Result<super::super::BankScopedWarmCensusTailTimedObservation> =
        super::super::run_position5_bank_scoped_census_tail_host_timing;
    admit_entry(POSITION5_REQUEST_SCHEMA, true).unwrap();
    assert!(admit_entry(POSITION5_REQUEST_SCHEMA, false).is_err());
    for schema in [
        REQUEST_SCHEMA,
        CAUSAL_REQUEST_SCHEMA,
        "FerricGuardedMlpFull2303RequestV1",
        TIMED_SCHEMA,
        super::super::shared_full::SCHEMA,
        "",
    ] {
        assert!(admit_entry(schema, true).is_err());
    }
}
#[test]
fn bank_scoped_census_tail_parent_authenticates_original_file_scope_and_executable() {
    let b = bootstrap();
    let value = record(&b);
    let file = policy_file(&value);
    assert_eq!(read_policy(&file.0, &b, [7; 32], [8; 32]).unwrap(), value);
    let mut polled = value.clone();
    polled.counts.tails.local_checkpoints += 1;
    polled.counts.tails.before_calls += 1;
    polled.counts.tails.after_calls += 1;
    polled.counts.tails.generation_probes += 2;
    let polled_file = policy_file(&polled);
    assert_eq!(
        read_policy(&polled_file.0, &b, [7; 32], [8; 32]).unwrap(),
        polled,
    );
    assert!(read_policy(&file.0, &b, [7; 32], [9; 32]).is_err());
    assert!(read_policy(&file.0, &b, [6; 32], [8; 32]).is_err());
    let mut changed = b.clone();
    changed.sequence.scope.session[0] ^= 1;
    assert!(read_policy(&file.0, &changed, [7; 32], [8; 32]).is_err());
    let mut wrong = file.0.clone();
    wrong.sha256[0] ^= 1;
    assert!(read_policy(&wrong, &b, [7; 32], [8; 32]).is_err());
    std::fs::write(&file.0.path, b"not the retained policy\n").unwrap();
    assert!(read_policy(&file.0, &b, [7; 32], [8; 32]).is_err());
    std::fs::remove_file(&file.0.path).unwrap();
    assert!(read_policy(&file.0, &b, [7; 32], [8; 32]).is_err());
}
#[cfg(feature = "engineering-currentness-duration-diagnostics")]
#[test]
fn duration_parent_requires_second_record_and_original_entire_file_pin() {
    use crate::finite_guarded_mlp_readiness_currentness_durations_v1 as diagnostic;
    let b = bootstrap();
    let policy = record(&b);
    let old = PolicyFile::new(&policy.encode().unwrap());
    assert!(read_policy(&old.0, &b, [7; 32], [8; 32]).is_err());
    let file = policy_file(&policy);
    read_policy(&file.0, &b, [7; 32], [8; 32]).unwrap();
    let mut changed = file.0.clone();
    changed.bytes -= 1;
    assert!(read_policy(&changed, &b, [7; 32], [8; 32]).is_err());
    let raw = std::fs::read(&file.0.path).unwrap();
    std::fs::write(&file.0.path, &policy.encode().unwrap()).unwrap();
    assert!(read_policy(&file.0, &b, [7; 32], [8; 32]).is_err());
    let duplicate = PolicyFile::new(&[raw.clone(), raw].concat());
    assert!(read_policy(&duplicate.0, &b, [7; 32], [8; 32]).is_err());
    let oversized = PolicyFile::new(&vec![b' '; diagnostic::STDERR_MAX_BYTES + 1]);
    assert!(read_policy(&oversized.0, &b, [7; 32], [8; 32]).is_err());
}
#[cfg(feature = "engineering-currentness-duration-diagnostics")]
#[test]
fn duration_parent_refuses_rehashed_report_counter_identity_and_containment_drift() {
    use crate::finite_guarded_mlp_readiness_currentness_durations_v1 as diagnostic;
    let (b, policy, record) = diagnostic::tests::fixture();
    let phases = phases::Record::new(&policy, &record, phases::tests::rows_for(&record)).unwrap();
    for mutation in 0..4 {
        let mut record = record.clone();
        match mutation {
            0 => record.policy_sha256[0] ^= 1,
            1 => {
                record.forwards[2]
                    .measured
                    .as_mut()
                    .unwrap()
                    .bank_guarded_body_ns = 3
            }
            2 => {
                record.forwards[2]
                    .measured
                    .as_mut()
                    .unwrap()
                    .bank
                    .before
                    .calls += 1
            }
            _ => record.execution_authority = true,
        }
        let file = PolicyFile::new(
            &[
                policy.encode().unwrap(),
                record.encode().unwrap(),
                phases.encode().unwrap(),
            ]
            .concat(),
        );
        assert!(read_policy(&file.0, &b, [7; 32], [8; 32]).is_err());
    }
}
#[cfg(feature = "engineering-currentness-duration-diagnostics")]
#[test]
fn forward_duration_parent_requires_three_canonical_ordered_records() {
    let (b, policy, record) = diagnostic::tests::fixture();
    let phases = phases::Record::new(&policy, &record, phases::tests::rows_for(&record)).unwrap();
    let first = policy.encode().unwrap();
    let second = record.encode().unwrap();
    let third = phases.encode().unwrap();
    let raw = [first.clone(), second.clone(), third.clone()].concat();
    let file = PolicyFile::new(&raw);
    assert_eq!(read_policy(&file.0, &b, [7; 32], [8; 32]).unwrap(), policy);
    let prior = [first.clone(), second.clone()].concat();
    diagnostic::decode_stderr(&prior, &b, [7; 32], [8; 32]).unwrap();
    assert_eq!(phases::STDERR_MAX_BYTES, diagnostic::STDERR_MAX_BYTES);
    for bad in [
        prior,
        [first.clone(), third.clone(), second.clone()].concat(),
        [first.clone(), second.clone(), third.clone(), third.clone()].concat(),
        [first.clone(), second.clone(), b"{}\n".to_vec()].concat(),
        [
            first.clone(),
            second.clone(),
            third[..third.len() - 1].to_vec(),
        ]
        .concat(),
        [first.clone(), second.clone(), b" ".to_vec(), third.clone()].concat(),
        [
            first,
            second,
            vec![b' '; phases::MAX_BYTES + 1],
            b"\n".to_vec(),
        ]
        .concat(),
        [raw, b"\n".to_vec()].concat(),
    ] {
        let file = PolicyFile::new(&bad);
        assert!(read_policy(&file.0, &b, [7; 32], [8; 32]).is_err());
    }
}

#[cfg(feature = "engineering-currentness-duration-diagnostics")]
#[test]
fn forward_duration_parent_refuses_rehashed_third_identity_rows_and_containment_drift() {
    let (b, policy, record) = diagnostic::tests::fixture();
    let original = phases::Record::new(&policy, &record, phases::tests::rows_for(&record)).unwrap();
    for mutation in 0..22 {
        let mut phases = original.clone();
        match mutation {
            0 => phases.policy_sha256[0] ^= 1,
            1 => phases.currentness_record_sha256[0] ^= 1,
            2 => phases.session[0] ^= 1,
            3 => phases.worker_sha256[0] ^= 1,
            4 => phases.transcript_sha256[0] ^= 1,
            5 => phases.phase_order.swap(0, 1),
            6 => {
                phases.forwards.pop();
            }
            7 => phases.forwards[3].position = 2,
            8..=10 => {
                let row = &mut phases.forwards[2];
                row.phase_ns[mutation - 5] = 0;
                row.forward_body_ns = row.phase_ns.iter().sum();
            }
            11 => phases.forwards[2].forward_body_ns += 1,
            12 => {
                phases.forwards[2].phase_ns[0] = u64::MAX;
                phases.forwards[2].forward_body_ns = u64::MAX;
            }
            13 => {
                phases.forwards[0].phase_ns[0] = 3_600_000_000_001;
                phases.forwards[0].forward_body_ns = 3_600_000_000_001;
            }
            14 => phases.instrumented = false,
            15 => phases.execution_authority = true,
            16 => phases.numerical_acceptance = true,
            17 => phases.performance_claim = true,
            18 => phases.gpu_timing = true,
            19 => phases.disjoint_phases = false,
            20 => phases.host_elapsed_nanoseconds = false,
            _ => phases.currentness_durations_nested = false,
        }
        let file = PolicyFile::new(
            &[
                policy.encode().unwrap(),
                record.encode().unwrap(),
                phases.encode().unwrap(),
            ]
            .concat(),
        );
        assert!(
            read_policy(&file.0, &b, [7; 32], [8; 32]).is_err(),
            "{mutation}",
        );
    }
}

#[cfg(feature = "engineering-currentness-duration-diagnostics")]
#[test]
fn forward_duration_parent_preserves_first_two_records_and_whole_stderr_identity() {
    let b = bootstrap();
    let policy = record(&b);
    let (record, phases) = diagnostic_records(&policy);
    let prefix = [policy.encode().unwrap(), record.encode().unwrap()].concat();
    let expected = [prefix.clone(), phases.encode().unwrap()].concat();
    let file = policy_file(&policy);
    let raw = std::fs::read(&file.0.path).unwrap();
    assert_eq!(raw, expected);
    assert_eq!(&raw[..prefix.len()], prefix.as_slice());
    assert_eq!(file.0.bytes, raw.len() as u64);
    assert_eq!(file.0.sha256, hash(&raw));
    let original_pin = file.0.clone();
    read_policy(&file.0, &b, [7; 32], [8; 32]).unwrap();
    assert_eq!(file.0, original_pin);
    let mut prefix_pin = file.0.clone();
    prefix_pin.bytes = prefix.len() as u64;
    prefix_pin.sha256 = hash(&prefix);
    assert!(read_policy(&prefix_pin, &b, [7; 32], [8; 32]).is_err());
    let mut changed = phases.clone();
    changed.forwards[2].phase_ns[0] += 1;
    changed.forwards[2].forward_body_ns += 1;
    let changed_raw = [prefix, changed.encode().unwrap()].concat();
    std::fs::write(&file.0.path, &changed_raw).unwrap();
    assert!(read_policy(&file.0, &b, [7; 32], [8; 32]).is_err());
    let repinned = PolicyFile::new(&changed_raw);
    assert_eq!(
        read_policy(&repinned.0, &b, [7; 32], [8; 32]).unwrap(),
        policy
    );
}

#[test]
fn bank_scoped_census_tail_parent_refuses_old_policies_duplicate_and_oversized_stderr() {
    let b = bootstrap();
    let raw = record(&b).encode().unwrap();
    let shared =
        crate::finite_guarded_mlp_readiness_shared_v1::PolicyRecord::new(&b, [7; 32], [8; 32])
            .unwrap()
            .encode()
            .unwrap();
    let old_scoped = crate::finite_guarded_mlp_readiness_scoped_v1::PolicyRecord::new(
        &b,
        [7; 32],
        [8; 32],
        record(&b).counts.layers,
    )
    .unwrap()
    .encode()
    .unwrap();
    let old_bank = crate::finite_guarded_mlp_readiness_bank_scoped_v2::PolicyRecord::new(
        &b,
        [7; 32],
        [8; 32],
        crate::finite_guarded_mlp_readiness_bank_scoped_v2::Counts {
            layers: record(&b).counts.layers,
            banks: record(&b).counts.banks,
        },
    )
    .unwrap()
    .encode()
    .unwrap();
    let prior_counts = record(&b).counts;
    let old_census = crate::finite_guarded_mlp_readiness_bank_scoped_census_v3::PolicyRecord::new(
        &b,
        [7; 32],
        [8; 32],
        crate::finite_guarded_mlp_readiness_bank_scoped_census_v3::Counts {
            layers: prior_counts.layers,
            banks: prior_counts.banks,
            census: prior_counts.census,
        },
    )
    .unwrap()
    .encode()
    .unwrap();
    for bad in [
        Vec::new(),
        old_census,
        old_bank,
        old_scoped,
        shared,
        [raw.clone(), raw.clone()].concat(),
        raw[..raw.len() - 1].to_vec(),
        vec![b' '; MAX_BYTES + 1],
    ] {
        let file = PolicyFile::new(&bad);
        assert!(read_policy(&file.0, &b, [7; 32], [8; 32]).is_err());
    }
}
#[cfg(feature = "engineering-currentness-duration-diagnostics")]
#[test]
fn layer_duration_parent_refuses_prior_forward_schema_without_relabeling_it() {
    use crate::finite_guarded_mlp_readiness_forward_durations_v1 as prior;
    let (b, policy, callbacks) = diagnostic::tests::fixture();
    let old = prior::Record::new(&policy, &callbacks, prior::tests::rows_for(&callbacks)).unwrap();
    let raw = [
        policy.encode().unwrap(),
        callbacks.encode().unwrap(),
        old.encode().unwrap(),
    ]
    .concat();
    prior::decode_stderr(&raw, &b, [7; 32], [8; 32]).unwrap();
    let file = PolicyFile::new(&raw);
    assert!(read_policy(&file.0, &b, [7; 32], [8; 32]).is_err());
    let current = policy_file(&policy);
    assert_eq!(
        read_policy(&current.0, &b, [7; 32], [8; 32]).unwrap(),
        policy
    );
}

#[cfg(feature = "engineering-currentness-duration-diagnostics")]
#[test]
fn layer_duration_parent_refuses_rehashed_presence_count_and_stage_order_drift() {
    let (b, policy, callbacks) = diagnostic::tests::fixture();
    let original =
        phases::Record::new(&policy, &callbacks, phases::tests::rows_for(&callbacks)).unwrap();
    for mutation in 0..9 {
        let mut changed = original.clone();
        match mutation {
            0 => changed.forwards[0].layer_metrics = changed.forwards[2].layer_metrics,
            1 => changed.forwards[2].layer_metrics = None,
            2 => changed.forwards[2].layer_metrics.as_mut().unwrap().layers = 35,
            3 => changed.forwards[2].layer_metrics.as_mut().unwrap().layers = 37,
            4 => changed.layer_stage_order.swap(0, 1),
            5 => changed.paired_mlp_stage_order.swap(0, 1),
            6 => changed.forwards[39].layer_metrics = None,
            7 => changed.layer_durations_nested = false,
            _ => changed.paired_mlp_durations_nested = false,
        }
        let file = PolicyFile::new(
            &[
                policy.encode().unwrap(),
                callbacks.encode().unwrap(),
                changed.encode().unwrap(),
            ]
            .concat(),
        );
        assert!(
            read_policy(&file.0, &b, [7; 32], [8; 32]).is_err(),
            "{mutation}"
        );
    }
}

#[cfg(feature = "engineering-currentness-duration-diagnostics")]
#[test]
fn layer_duration_parent_refuses_rehashed_nested_sum_overflow_and_containment_drift() {
    let (b, policy, callbacks) = diagnostic::tests::fixture();
    let original =
        phases::Record::new(&policy, &callbacks, phases::tests::rows_for(&callbacks)).unwrap();
    for mutation in 0..8 {
        let mut changed = original.clone();
        let row = &mut changed.forwards[2];
        let enclosing = row.phase_ns[4];
        let metrics = row.layer_metrics.as_mut().unwrap();
        match mutation {
            0 => metrics.layer_body_ns += 1,
            1 => metrics.paired_mlp_body_ns += 1,
            2 => {
                metrics.phase_ns[0] = u64::MAX;
                metrics.layer_body_ns = u64::MAX;
            }
            3 => {
                metrics.paired_mlp_phase_ns[0] = u64::MAX;
                metrics.paired_mlp_body_ns = u64::MAX;
            }
            4 => {
                metrics.phase_ns = [0; 6];
                metrics.phase_ns[2] = enclosing + 1;
                metrics.layer_body_ns = enclosing + 1;
            }
            5 => {
                metrics.paired_mlp_phase_ns = [0; 7];
                metrics.paired_mlp_phase_ns[4] = metrics.phase_ns[2] + 1;
                metrics.paired_mlp_body_ns = metrics.phase_ns[2] + 1;
            }
            6 => {
                metrics.phase_ns[0] = 3_600_000_000_001;
                metrics.layer_body_ns = 3_600_000_000_001;
            }
            _ => {
                metrics.phase_ns = [0; 6];
                metrics.layer_body_ns = 0;
                metrics.paired_mlp_phase_ns = [0; 7];
                metrics.paired_mlp_body_ns = 0;
            }
        }
        let file = PolicyFile::new(
            &[
                policy.encode().unwrap(),
                callbacks.encode().unwrap(),
                changed.encode().unwrap(),
            ]
            .concat(),
        );
        assert!(
            read_policy(&file.0, &b, [7; 32], [8; 32]).is_err(),
            "{mutation}"
        );
    }
}

#[test]
fn bank_scoped_census_tail_parent_refuses_rehashed_counter_and_policy_drift() {
    let b = bootstrap();
    for mutation in 0..43 {
        let mut value = record(&b);
        match mutation {
            0 => value.counts.layers.scoped_layers_by_forward[1] = 36,
            1 => value.counts.layers.full_discoveries -= 1,
            2 => value.counts.layers.after_calls += 1,
            3 => value.counts.layers.generation_probes += 1,
            4 => value.counts.layers.local_checkpoints = u64::MAX,
            5 => value.temporal_equivalent_to_full = true,
            6 => value.shared_full_currentness = true,
            7 => value.native_closed = false,
            8 => value.counts.banks.scoped_rearms -= 1,
            9 => value.counts.banks.scoped_rearms_by_forward[1] = 1,
            10 => value.counts.banks.final_generations[1] -= 1,
            11 => value.counts.banks.local_checkpoints = u64::MAX,
            12 => value.counts.banks.before_calls += 1,
            13 => value.scoped_bank_rearm = false,
            14 => value.allocation_preflights_outside_windows = true,
            15 => value.counts.layers.scoped_layers -= 1,
            16 => value.counts.census.warm_layers -= 1,
            17 => value.counts.census.preflights -= 1,
            18 => value.counts.census.rank_checkpoints -= 1,
            19 => value.counts.census.owner_counts[0] = 0,
            20 => value.counts.census.owner_counts[1] = 2049,
            21 => value.scoped_capacity_census = false,
            22 => value.allocation_preflights_changed = false,
            23 => value.default_group_policy_unchanged = false,
            24 => value.counts.layers.local_checkpoints = 5 * 1368,
            25 => value.counts.layers.before_calls -= 21888,
            26 => value.counts.layers.generation_probes -= 2 * 21888,
            27 => value.counts.census.owner_counts[1] = u64::MAX,
            28 => {
                value.counts.layers.local_checkpoints = 5 * 1368;
                value.counts.layers.before_calls = 11 * 1368;
                value.counts.layers.after_calls = 11 * 1368;
                value.counts.layers.generation_probes = 13 * 1368;
            }
            29 => value.counts.tails.ordinary_tails += 1,
            30 => value.counts.tails.scoped_tails -= 1,
            31 => value.counts.tails.dispatches -= 1,
            32 => value.counts.tails.readbacks -= 1,
            33 => value.counts.tails.readback_bytes -= 1,
            34 => value.counts.tails.full_discoveries -= 1,
            35 => value.counts.tails.local_checkpoints = 27 * 38 - 1,
            36 => value.counts.tails.before_calls += 1,
            37 => value.counts.tails.after_calls += 1,
            38 => value.counts.tails.generation_probes += 1,
            39 => value.counts.tails.local_checkpoints = u64::MAX,
            40 => value.scoped_tail = false,
            41 => value.full_entry_exit_per_scoped_tail = false,
            _ => value.scope_includes_tail_dispatch_and_readback = false,
        }
        let file = PolicyFile::new(&value.encode().unwrap());
        assert!(
            read_policy(&file.0, &b, [7; 32], [8; 32]).is_err(),
            "{mutation}"
        );
    }
}
