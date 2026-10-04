use super::*;
fn args() -> Vec<OsString> {
    [
        "--engineering-native-projection-residual-decode-v1",
        "--allow-unauthenticated-machine-code",
        "--devices",
        "11,12",
        "--timeout-ms",
        "100",
        "--mode",
        "teacher-forced",
    ]
    .into_iter()
    .map(OsString::from)
    .collect()
}
#[test]
fn projection_decode_cli_accepts_only_explicit_plain_tf_invocation() {
    let good = args();
    parse_args(&good).unwrap();
    for length in 0..good.len() {
        assert!(parse_args(&good[..length]).is_err());
    }
    for (index, replacement) in [
        (0, "--engineering-native-prefix-decode-v1"),
        (1, "--cached"),
        (3, "11,11"),
        (3, "0,12"),
        (3, "11,12,13"),
        (5, "10001"),
        (5, "0"),
        (7, "autoregressive"),
    ] {
        let mut bad = good.clone();
        bad[index] = replacement.into();
        assert!(parse_args(&bad).is_err());
    }
    let mut extra = good;
    extra.push("--device-sidecar".into());
    assert!(parse_args(&extra).is_err());
}
#[test]
fn projection_decode_prepare_rejects_wrong_identity_and_missing_begin_before_open() {
    let options = parse_args(&args()).unwrap();
    for changed in [false, true] {
        let mut b = wire::tests::bootstrap();
        if changed {
            b.decode.scope.child_identity += 1;
            b.decode.begin.scope = b.decode.scope.clone();
        }
        let mut bytes = Vec::new();
        wire::write_bootstrap(&mut bytes, &mut FrameBudget::new(), &b, &[8], &[9], &[10]).unwrap();
        assert!(prepare(&options, &mut &bytes[..], &mut FrameBudget::new()).is_err());
    }
}
