use super::*;

#[test]
fn prefix_host_worker_exact_optin_preserves_plain_parser_and_rejects_policy_flags() {
    let good = [
        "--engineering-native-prefix-decode-host-v1",
        "--allow-unauthenticated-machine-code",
        "--devices",
        "11,12",
        "--timeout-ms",
        "100",
        "--mode",
        "teacher-forced",
        "--host-sidecar",
        "/tmp/host.json",
    ];
    let args = good.iter().map(OsString::from).collect::<Vec<_>>();
    let result = parse_args(&args).unwrap();
    assert_eq!(result.path, PathBuf::from(good[9]));
    assert!(plain::parse_args(&args).is_err());
    for n in 0..10 {
        assert!(parse_args(&args[..n]).is_err());
    }
    for field in [
        "--cache-kernel-admission",
        "--profile",
        "--shared-currentness",
        "--raw-timestamps",
    ] {
        let mut bad = args.clone();
        bad.push(field.into());
        assert!(parse_args(&bad).is_err());
    }
    for change in [0, 8, 9] {
        let mut bad = args.clone();
        bad[change] = "relative-or-wrong".into();
        assert!(parse_args(&bad).is_err());
    }
}
#[test]
fn prefix_host_worker_new_canonical_sidecar_only_no_replacement() {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let base = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "host-diagnostic-{}-{}",
        std::process::id(),
        N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir(&base).unwrap();
    let path = base.join("sidecar.json");
    preflight_path(&path).unwrap();
    std::fs::write(&path, b"retained").unwrap();
    assert!(preflight_path(&path).is_err());
    std::fs::remove_file(&path).unwrap();
    std::os::unix::fs::symlink(base.join("missing"), &path).unwrap();
    assert!(preflight_path(&path).is_err());
    assert!(preflight_path(&base.join("missing-parent/file")).is_err());
    std::fs::remove_dir_all(base).unwrap();
}
#[test]
fn prefix_host_worker_duration_overflow_and_actual_executable_pin() {
    assert_eq!(ns(Duration::from_nanos(u64::MAX)).unwrap(), u64::MAX);
    assert!(ns(Duration::from_secs(u64::MAX)).is_err());
    let digest = executable_sha().unwrap();
    assert_ne!(digest, [0; 32]);
    assert_eq!(executable_sha().unwrap(), digest);
}
