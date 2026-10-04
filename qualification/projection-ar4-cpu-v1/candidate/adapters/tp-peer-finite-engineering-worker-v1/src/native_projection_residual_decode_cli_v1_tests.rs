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
fn projection_decode_cli_accepts_only_explicit_plain_mode_invocation() {
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
        (7, "auto"),
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
fn projection_ar4_cli_selects_mode_without_changing_tf_argv_or_fallback() {
    let tf = args();
    let tf_options = parse_args(&tf).unwrap();
    assert_eq!(tf_options.mode, InputMode::TeacherForced);
    let mut ar = tf.clone();
    ar[7] = "autoregressive".into();
    assert_eq!(ar[..7], tf[..7]);
    let ar_options = parse_args(&ar).unwrap();
    assert_eq!(ar_options.mode, InputMode::Autoregressive);
    assert_eq!(
        admitted_mode(&tf_options, &wire::tests::bootstrap()).unwrap(),
        Mode::TeacherForced([9112, 2190, 3772, 220])
    );
    assert_eq!(
        admitted_mode(&ar_options, &wire::tests::ar_bootstrap()).unwrap(),
        Mode::Autoregressive { first: 9112 }
    );
    assert!(admitted_mode(&tf_options, &wire::tests::ar_bootstrap()).is_err());
    assert!(admitted_mode(&ar_options, &wire::tests::bootstrap()).is_err());
    for mode in ["", "AR", "teacher_forced", "teacher-forced "] {
        ar[7] = mode.into();
        assert!(parse_args(&ar).is_err());
    }
}

#[test]
fn projection_ar4_cli_mode_mismatch_is_refused_before_begin_is_read() {
    struct NoBegin;
    impl Read for NoBegin {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            panic!("mode mismatch must precede Begin or native setup")
        }
    }
    for (b, mode) in [
        (wire::tests::bootstrap(), "autoregressive"),
        (wire::tests::ar_bootstrap(), "teacher-forced"),
    ] {
        let mut argv = args();
        argv[7] = mode.into();
        let options = parse_args(&argv).unwrap();
        let mut bytes = Vec::new();
        wire::write_bootstrap(&mut bytes, &mut FrameBudget::new(), &b, &[8], &[9], &[10]).unwrap();
        let mut reader = io::Cursor::new(bytes).chain(NoBegin);
        assert!(prepare(&options, &mut reader, &mut FrameBudget::new()).is_err());
    }
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
