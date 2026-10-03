use super::*;
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static SERIAL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "finite-prefix-decode-evidence-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn prefix_decode_worst_typed_capture_requests_and_stderr_fit_existing_output_cap() {
    let total = 4 * (wire::CONTROL_BYTES + old::OBSERVATION_BYTES) as u64
        + 4 * REQUEST_LIMIT
        + STDERR_LIMIT
        + SUMMARY_LIMIT as u64;
    assert_eq!(total, 5_623_968);
    assert!(total < OWN_LIMIT);
    assert!(OWN_LIMIT + SUMMARY_LIMIT as u64 + (512 << 10) < 32 << 20);
}
#[test]
fn prefix_decode_incomplete_exclusive_evidence_does_not_publish_or_replace() {
    let temp = Temp::new();
    let path = temp.0.join("run");
    let mut evidence = Evidence::create(&path).unwrap();
    evidence.add("provisional.bin", &[1, 2], 2).unwrap();
    assert!(evidence.add("provisional.bin", &[3], 2).is_err());
    assert!(evidence.finish(&[]).is_err());
    assert_eq!(std::fs::read(path.join("provisional.bin")).unwrap(), [1, 2]);
    assert!(!path.join("complete.json").exists());
    assert!(Evidence::create(&path).is_err());
}
#[test]
fn prefix_decode_output_pins_rehash_empty_and_nonempty_files_and_refuse_links() {
    let temp = Temp::new();
    let empty = write_file(temp.0.join("empty"), &[], 0).unwrap();
    verify(&empty, 0).unwrap();
    let pin = write_file(temp.0.join("bytes"), &[1, 2, 3], 3).unwrap();
    assert!(write_file(temp.0.join("oversize"), &[1, 2, 3], 2).is_err());
    let link = temp.0.join("link");
    std::os::unix::fs::symlink(&pin.path, &link).unwrap();
    assert!(write_file(link, &[0], 1).is_err());
    std::fs::write(&pin.path, [3, 2, 1]).unwrap();
    assert!(verify(&pin, 3).is_err());
}

fn completed_evidence(path: &Path) -> Observation {
    let b = wire::tests::bootstrap(wire::InputMode::TeacherForced);
    let mut ev = Evidence::create(path).unwrap();
    let mut chain = wire::Chain::new(b.registration, b.sha256().unwrap());
    for p in 0..4 {
        let r = wire::tests::request(&b, p, None);
        let (s, c, raw) = wire::tests::completed(&r, &mut chain, p + 7);
        ev.append(&r, &s, &c, &raw).unwrap();
    }
    let files = ev.finish(&[]).unwrap();
    let mut config = super::super::tests::config();
    config.evidence_directory = path.into();
    Observation {
        schema: "FerricFinitePrefixDecodeObservationV1",
        request: config,
        child_pid: b.scope.child_identity,
        registration_sha256: b.registration,
        source_program_sha256: b.begin.source_program.sha256,
        upload_manifest_sha256: b.begin.uploads.sha256,
        profile_sha256: b.sha256().unwrap(),
        setup_commands: 1,
        completed_forwards: 4,
        input_tokens: b.input_tokens.clone(),
        observed_output_tokens: vec![7, 8, 9, 10],
        page_permutation: (0..144).collect(),
        transcript_sha256: chain.digest(),
        request_stream_bytes: 1,
        response_stream_bytes: 3_723_444,
        close: wire::tests::close(&b, chain.digest()).1,
        bootstrap: b,
        files,
        child_exit_zero: true,
        process_group_absent: true,
        native_closed: true,
        gpu_execution: true,
        numerical_acceptance: false,
        performance_claim: false,
        production_authority: false,
        full_long_workload: false,
    }
}
#[test]
fn prefix_decode_exact_fourteen_files_empty_stderr_and_summary_accounting() {
    let t = Temp::new();
    let path = t.0.join("complete");
    let mut observation = completed_evidence(&path);
    assert_eq!(observation.files.child_stderr.bytes, 0);
    assert_eq!(std::fs::read_dir(&path).unwrap().count(), 13);
    publish(&mut observation).unwrap();
    assert_eq!(std::fs::read_dir(&path).unwrap().count(), 14);
    let raw = std::fs::read(path.join("complete.json")).unwrap();
    assert_eq!(raw.len() as u64, observation.files.summary_bytes);
    let total: u64 = std::fs::read_dir(&path)
        .unwrap()
        .map(|v| v.unwrap().metadata().unwrap().len())
        .sum();
    assert_eq!(total, observation.files.total_bytes);
    assert!(total < OWN_LIMIT);
}
#[test]
fn prefix_decode_publication_refuses_unclosed_unreaped_or_changed_evidence() {
    for change in 0..5 {
        let t = Temp::new();
        let path = t.0.join("complete");
        let mut o = completed_evidence(&path);
        match change {
            0 => o.native_closed = false,
            1 => o.child_exit_zero = false,
            2 => o.process_group_absent = false,
            3 => {
                std::fs::write(&o.files.frames[3].observation.path, [0]).unwrap();
            }
            _ => {
                std::fs::write(path.join("extra"), [0]).unwrap();
            }
        }
        assert!(publish(&mut o).is_err());
        assert!(!path.join("complete.json").exists());
    }
}
