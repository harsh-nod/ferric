use super::*;

fn args(policy: Policy) -> Vec<OsString> {
    [
        "--engineering-native-prefix-decode-host-v2",
        "--allow-unauthenticated-machine-code",
        "--devices",
        "11,12",
        "--timeout-ms",
        "100",
        "--mode",
        "teacher-forced",
        "--host-sidecar",
        "/tmp/host-v2.json",
        "--host-policy",
        policy.name(),
    ]
    .into_iter()
    .map(OsString::from)
    .collect()
}

#[test]
fn prefix_host_v2_worker_exact_single_factor_policy_required() {
    for policy in [
        Policy::Baseline,
        Policy::ImmutableAdmissionCache,
        Policy::SharedFullCurrentness,
    ] {
        let good = args(policy);
        let value = parse_args(&good).unwrap();
        assert_eq!(value.policy, policy);
        assert_eq!(value.path, PathBuf::from("/tmp/host-v2.json"));
        assert!(plain::parse_args(&good).is_err());
        assert!(crate::native_prefix_decode_host_v1::parse_args(&good).is_err());
        for n in 0..good.len() {
            assert!(parse_args(&good[..n]).is_err());
        }
        for flag in [
            "--profile",
            "--operational-currentness",
            "--raw-timestamps",
            "--host-policy",
        ] {
            let mut bad = good.clone();
            bad.push(flag.into());
            assert!(parse_args(&bad).is_err());
        }
        for n in [0, 8, 9, 10, 11] {
            let mut bad = good.clone();
            bad[n] = "wrong-or-relative".into();
            assert!(parse_args(&bad).is_err());
        }
        let mut ar = good;
        ar[7] = "autoregressive".into();
        assert_eq!(parse_args(&ar).unwrap().policy, policy);
    }
}

#[test]
fn prefix_host_v2_worker_rejects_combined_or_non_utf8_policy() {
    use std::os::unix::ffi::OsStringExt;
    let mut bad = args(Policy::Baseline);
    for name in [
        "both",
        "auto",
        "baseline,immutable-admission-cache",
        "immutable-admission-cache+shared-full-currentness",
    ] {
        bad[11] = name.into();
        assert!(parse_args(&bad).is_err());
    }
    bad[11] = OsString::from_vec(vec![0xff]);
    assert!(parse_args(&bad).is_err());
}
