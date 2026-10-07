use super::*;
fn bootstrap() -> ready::Bootstrap {
    ready::Bootstrap {
        schema: ready::POSITION5_SCHEMA.into(),
        child_deadline_ms: 60_000,
        sequence: long::tests::bootstrap(long::Profile::Readiness40Position5),
    }
}
#[test]
fn shared_full_parent_entry_preserves_existing_public_signatures_and_refuses_other_modes() {
    let _: fn(ReadinessConfig, bool) -> Result<Observation> = super::super::run;
    let _: fn(ReadinessConfig, bool) -> Result<Observation> = super::super::run_position5;
    let _: fn(ReadinessConfig, bool) -> Result<Observation> = super::super::run_causal;
    let _: fn(ReadinessConfig, bool) -> Result<super::super::TimedObservation> =
        super::super::run_position5_host_timing;
    let _: fn(ReadinessConfig, bool) -> Result<SharedFullObservation> = run_position5_shared_full;
    admit_entry(POSITION5_REQUEST_SCHEMA, true).unwrap();
    assert!(admit_entry(POSITION5_REQUEST_SCHEMA, false).is_err());
    for schema in [
        REQUEST_SCHEMA,
        CAUSAL_REQUEST_SCHEMA,
        "FerricGuardedMlpFull2303RequestV1",
        SCHEMA,
        "",
    ] {
        assert!(admit_entry(schema, true).is_err());
    }
}
#[test]
fn shared_full_parent_reads_retained_bytes_and_rejects_changed_or_absent_policy() {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "ferric-shared-policy-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let _cleanup = Cleanup(path.clone());
    let b = bootstrap();
    let record = PolicyRecord::new(&b, [7; 32], [8; 32]).unwrap();
    let raw = record.encode().unwrap();
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .unwrap();
    std::io::Write::write_all(&mut file, &raw).unwrap();
    drop(file);
    let pin = FilePin {
        path: path.clone(),
        bytes: raw.len() as u64,
        sha256: hash(&raw),
    };
    assert_eq!(read_policy(&pin, &b, [7; 32], [8; 32]).unwrap(), record);
    assert!(read_policy(&pin, &b, [7; 32], [9; 32]).is_err());
    let mut wrong = pin.clone();
    wrong.sha256[0] ^= 1;
    assert!(read_policy(&wrong, &b, [7; 32], [8; 32]).is_err());
    std::fs::write(&path, b"ordinary stderr is not a policy record\n").unwrap();
    assert!(read_policy(&pin, &b, [7; 32], [8; 32]).is_err());
    std::fs::remove_file(&path).unwrap();
    assert!(read_policy(&pin, &b, [7; 32], [8; 32]).is_err());
}
