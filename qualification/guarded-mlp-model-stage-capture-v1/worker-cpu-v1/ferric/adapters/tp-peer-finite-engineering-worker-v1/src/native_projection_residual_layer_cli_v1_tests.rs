use super::*;
fn args() -> Vec<OsString> {
    [
        "--engineering-native-projection-residual-layer-v1",
        "--allow-unauthenticated-machine-code",
        "--devices",
        "7,9",
        "--timeout-ms",
        "1000",
    ]
    .map(OsString::from)
    .to_vec()
}
#[test]
fn projection_cli_requires_separate_explicit_closed_selector() {
    assert_eq!(
        parse_args(&args()).unwrap(),
        NativeOptions {
            devices: [7, 9],
            timeout_ms: 1000
        }
    );
    for (index, value) in [
        (0, "--engineering-native-prefix-layer-v1"),
        (1, "--allow"),
        (2, "--device"),
        (3, "7,7"),
        (3, "0,9"),
        (3, "7,9,11"),
        (3, "-7,9"),
        (4, "--timeout"),
        (5, "0"),
        (5, "10001"),
        (5, "+1"),
    ] {
        let mut a = args();
        a[index] = value.into();
        assert!(parse_args(&a).is_err(), "{index}/{value}");
    }
    let mut a = args();
    a.push("--profile".into());
    assert!(parse_args(&a).is_err());
    assert!(parse_args(&args()[..5]).is_err());
}
#[test]
fn projection_prepare_rejects_bad_identity_before_begin_or_open() {
    let options = parse_args(&args()).unwrap();
    for field in 0..3 {
        let mut b = Bootstrap {
            schema: wire::SCHEMA.into(),
            layer: crate::finite_prefix_layer_wire_v1::tests::bootstrap(
                crate::finite_prefix_layer_wire_v1::Profile::Prefix284Mlp548,
            ),
            projection_residual_image: wire::part(&[2]),
        };
        match field {
            0 => b.layer.device_ids = [9, 7],
            1 => b.layer.timeout_ms = 999,
            _ => b.layer.begin.scope.child_identity = std::process::id().wrapping_add(1).max(1),
        }
        let mut raw = Vec::new();
        wire::write_bootstrap(&mut raw, &mut Budget::new(), &b, &[1], &[1], &[2]).unwrap();
        let error = prepare(&options, &mut raw.as_slice(), &mut Budget::new())
            .err()
            .unwrap();
        assert!(
            error
                .to_string()
                .contains("bootstrap/argv/process mismatch")
        );
    }
}
