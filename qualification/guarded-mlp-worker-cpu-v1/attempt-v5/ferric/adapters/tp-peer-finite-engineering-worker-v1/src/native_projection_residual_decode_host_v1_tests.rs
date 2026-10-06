use super::*;
fn args() -> Vec<OsString> {
    [
        "--engineering-native-projection-residual-decode-host-v1",
        "--allow-unauthenticated-machine-code",
        "--devices",
        "1,2",
        "--timeout-ms",
        "10000",
        "--mode",
        "autoregressive",
        "--host-sidecar",
        "/tmp/projection-host.json",
    ]
    .iter()
    .map(OsString::from)
    .collect()
}
#[test]
fn projection_host_cli_is_distinct_and_ar_only() {
    parse_args(&args()).unwrap();
    let mut a = args();
    a[0] = "--engineering-native-projection-residual-decode-v1".into();
    assert!(parse_args(&a).is_err());
    let mut a = args();
    a[7] = "teacher-forced".into();
    assert!(parse_args(&a).is_err());
}
#[test]
fn projection_host_cli_rejects_optimized_policy_or_truncation() {
    for extra in [
        "--profile",
        "--shared-currentness",
        "--cache-kernel-admission",
        "--device-sidecar",
    ] {
        let mut a = args();
        a.push(extra.into());
        assert!(parse_args(&a).is_err());
    }
    for n in 0..10 {
        assert!(parse_args(&args()[..n]).is_err());
    }
}
#[test]
fn projection_host_cli_rejects_noncanonical_extent_and_duplicate_devices() {
    for path in ["relative", "/"] {
        let mut a = args();
        a[9] = path.into();
        assert!(parse_args(&a).is_err());
    }
    let mut a = args();
    a[3] = "1,1".into();
    assert!(parse_args(&a).is_err());
    let mut a = args();
    a[5] = "10001".into();
    assert!(parse_args(&a).is_err());
}
#[test]
fn projection_host_recorder_records_each_serialization_and_close_after_four() {
    let mut v = data::tests::report();
    let completions = v.completions.clone();
    let snapshots = v.snapshots.clone();
    v.completions.clear();
    v.native_closed = false;
    v.serialization_host_ns = [0; 4];
    for (position, c) in completions.iter().enumerate() {
        v.snapshots = snapshots[..position + 3].to_vec();
        record_completion(&mut v, c, 20 + position as u64).unwrap();
    }
    assert!(record_close(&mut v, 50).is_err());
    v.snapshots = snapshots;
    record_close(&mut v, 50).unwrap();
    assert_eq!(v.serialization_host_ns, [20, 21, 22, 23]);
    assert_eq!(v.close_host_ns, 50);
    v.validate().unwrap();
    assert!(record_close(&mut v, 51).is_err());
}
#[test]
fn projection_host_recorder_refuses_missing_snapshot_and_repeated_completion() {
    let mut v = data::tests::report();
    let c = v.completions[0].clone();
    v.completions.clear();
    v.native_closed = false;
    v.snapshots.truncate(2);
    assert!(record_completion(&mut v, &c, 1).is_err());
    v.snapshots = data::tests::report().snapshots[..3].to_vec();
    record_completion(&mut v, &c, 1).unwrap();
    assert!(record_completion(&mut v, &c, 1).is_err());
    assert!(record_close(&mut v, 1).is_err());
}
