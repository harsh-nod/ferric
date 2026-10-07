use super::*;
use crate::finite_guarded_mlp_readiness_bank_scoped_v2::{BankCounts, Counts, LayerCounts};
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
                local_checkpoints: 5 * 1368,
                before_calls: 11 * 1368,
                after_calls: 11 * 1368,
                generation_probes: 13 * 1368,
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
        },
    )
    .unwrap()
}
struct PolicyFile(FilePin);
impl PolicyFile {
    fn new(raw: &[u8]) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "ferric-bank-scoped-policy-{}-{}",
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
fn bank_scoped_warm_parent_entry_preserves_legacy_signatures_and_exact_opt_in() {
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
fn bank_scoped_warm_parent_authenticates_original_file_scope_and_executable() {
    let b = bootstrap();
    let value = record(&b);
    let file = PolicyFile::new(&value.encode().unwrap());
    assert_eq!(read_policy(&file.0, &b, [7; 32], [8; 32]).unwrap(), value);
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
#[test]
fn bank_scoped_warm_parent_refuses_old_policies_duplicate_and_oversized_stderr() {
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
    for bad in [
        Vec::new(),
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
#[test]
fn bank_scoped_warm_parent_refuses_rehashed_counter_and_policy_drift() {
    let b = bootstrap();
    for mutation in 0..16 {
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
            14 => value.allocation_preflights_outside_windows = false,
            _ => value.counts.layers.scoped_layers -= 1,
        }
        let file = PolicyFile::new(&value.encode().unwrap());
        assert!(
            read_policy(&file.0, &b, [7; 32], [8; 32]).is_err(),
            "{mutation}"
        );
    }
}
