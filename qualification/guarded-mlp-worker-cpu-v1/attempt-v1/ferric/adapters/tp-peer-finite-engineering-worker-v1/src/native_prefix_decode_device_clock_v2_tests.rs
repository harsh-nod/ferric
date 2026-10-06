use super::*;
use std::os::unix::ffi::OsStringExt;

fn args(mode: &str) -> Vec<OsString> {
    [
        "--engineering-native-prefix-decode-device-clock-v2",
        "--allow-unauthenticated-machine-code",
        "--devices",
        "11,12",
        "--timeout-ms",
        "100",
        "--mode",
        mode,
        "--device-sidecar",
        "/tmp/ferric-device-observation.json",
    ]
    .into_iter()
    .map(OsString::from)
    .collect()
}
#[test]
fn clock_cli_exact_selector_and_legacy_separation() {
    for mode in ["teacher-forced", "autoregressive"] {
        let good = args(mode);
        parse_args(&good).unwrap();
        assert!(plain::parse_args(&good).is_err());
        assert!(crate::native_prefix_decode_device_v1::parse_args(&good).is_err());
        assert!(crate::native_prefix_decode_host_v1::parse_args(&good).is_err());
        assert!(crate::native_prefix_decode_host_v2::parse_args(&good).is_err());
        for n in 0..good.len() {
            assert!(parse_args(&good[..n]).is_err());
        }
        for (index, value) in [
            (0, "--engineering-native-prefix-decode-host-v1"),
            (0, "--engineering-native-prefix-decode-device-v1"),
            (1, "--cached"),
            (8, "--host-sidecar"),
            (9, "relative.json"),
        ] {
            let mut bad = good.clone();
            bad[index] = value.into();
            assert!(parse_args(&bad).is_err());
        }
        let mut extra = good;
        extra.push("--force".into());
        assert!(parse_args(&extra).is_err());
    }
}
#[test]
fn clock_cli_rejects_malformed_device_deadline_mode_and_path() {
    for (index, value) in [
        (3, "11,11"),
        (3, "0,12"),
        (3, "11"),
        (5, "10001"),
        (5, "0"),
        (5, "-1"),
        (7, "auto"),
        (9, "/"),
    ] {
        let mut bad = args("teacher-forced");
        bad[index] = value.into();
        assert!(parse_args(&bad).is_err());
    }
    let mut bad = args("teacher-forced");
    bad[9] = format!("/{}", "x".repeat(512)).into();
    assert!(parse_args(&bad).is_err());
    bad = args("teacher-forced");
    bad[3] = OsString::from_vec(vec![255]);
    assert!(parse_args(&bad).is_err());
}
#[test]
#[allow(unsafe_code)]
fn clock_cli_incomplete_bootstrap_creates_no_sidecar_before_open() {
    let dir = std::env::temp_dir().join(format!("ferric-clock-cli-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.canonicalize().unwrap().join("raw.json");
    let mut args = args("teacher-forced");
    args[9] = path.clone().into_os_string();
    let mut output = Vec::new();
    // SAFETY: empty bootstrap refuses before opening any native group.
    assert!(unsafe { run_native(parse_args(&args).unwrap(), &mut &[][..], &mut output) }.is_err());
    assert!(output.is_empty());
    assert!(!path.exists());
    std::fs::remove_dir(dir).unwrap();
}
