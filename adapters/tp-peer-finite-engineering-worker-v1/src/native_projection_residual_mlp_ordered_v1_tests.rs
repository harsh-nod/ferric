use super::*;

fn args() -> Vec<OsString> {
    [
        Policy::OrderedSharedFull.worker_flag(),
        "--allow-unauthenticated-machine-code",
        "--devices",
        "16366993098680759275,10838076764495710945",
        "--timeout-ms",
        "10000",
        "--mode",
        "autoregressive",
        "--host-sidecar",
        "/tmp/projection-shared-host.json",
    ]
    .iter()
    .map(OsString::from)
    .collect()
}

#[test]
fn ordered_cli_is_distinct_and_rejects_extra_policies() {
    assert_eq!(
        parse_args(&args()).unwrap().policy,
        Policy::OrderedSharedFull
    );
    for flag in [
        "--engineering-native-projection-residual-decode-host-v1",
        "--engineering-native-projection-residual-decode-shared-host-v1",
        "--engineering-native-projection-residual-decode-v1",
        "--engineering-native-prefix-decode-device-clock-v2",
    ] {
        let mut old = args();
        old[0] = flag.into();
        assert!(parse_args(&old).is_err());
    }
    for n in 0..10 {
        assert!(parse_args(&args()[..n]).is_err());
    }
    for extra in [
        "--profile",
        "--host-policy",
        "--cache-kernel-admission",
        "--operational-currentness",
    ] {
        let mut bad = args();
        bad.push(extra.into());
        assert!(parse_args(&bad).is_err());
    }
}

#[test]
fn ordered_cli_retains_ar_device_and_path_bounds() {
    let good = parse_args(&args()).unwrap();
    assert_eq!(good.native.mode, InputMode::Autoregressive);
    assert_eq!(good.path, PathBuf::from("/tmp/projection-shared-host.json"));
    for (index, value) in [
        (7, "teacher-forced"),
        (3, "1,1"),
        (5, "10001"),
        (9, "relative"),
        (9, "/"),
    ] {
        let mut bad = args();
        bad[index] = value.into();
        assert!(parse_args(&bad).is_err());
    }
}

#[derive(Default)]
struct Setup {
    events: Vec<&'static str>,
    options: Vec<(bool, bool, bool)>,
    fail_configure: bool,
    fail_enable: bool,
    configured: bool,
    enabled: bool,
}
impl ObservationSetup for Setup {
    fn configure(&mut self, options: (bool, bool, bool)) -> io::Result<()> {
        self.events.push("configure");
        self.options.push(options);
        if self.fail_configure || self.configured || self.enabled {
            return Err(io::Error::other("injected configuration refusal"));
        }
        self.configured = true;
        Ok(())
    }
    fn enable(&mut self) -> io::Result<()> {
        self.events.push("enable");
        if self.fail_enable || self.enabled {
            return Err(io::Error::other("injected observation refusal"));
        }
        self.enabled = true;
        Ok(())
    }
}

#[test]
fn ordered_setup_configures_exact_policy_before_enable_once() {
    let mut setup = Setup::default();
    configure_observer(&mut setup, Policy::OrderedSharedFull).unwrap();
    assert_eq!(setup.events, ["configure", "enable"]);
    assert_eq!(setup.options, [(false, false, true)]);
    assert!(configure_observer(&mut setup, Policy::OrderedSharedFull).is_err());
    assert_eq!(setup.events, ["configure", "enable", "configure"]);
}

#[test]
fn ordered_setup_refusal_never_enables_after_failed_configuration() {
    let mut setup = Setup {
        fail_configure: true,
        ..Setup::default()
    };
    assert!(configure_observer(&mut setup, Policy::OrderedSharedFull).is_err());
    assert_eq!(setup.events, ["configure"]);
    assert!(!setup.enabled);
    let mut setup = Setup {
        fail_enable: true,
        ..Setup::default()
    };
    assert!(configure_observer(&mut setup, Policy::OrderedSharedFull).is_err());
    assert_eq!(setup.events, ["configure", "enable"]);
    assert!(!setup.enabled);
}

#[test]
fn ordered_recorder_retains_four_serializations_then_close() {
    let mut v = data::tests::report().observation;
    let completions = v.completions.clone();
    let snapshots = v.snapshots.clone();
    v.completions.clear();
    v.native_closed = false;
    v.serialization_host_ns = [0; 4];
    assert!(record_close(&mut v, 1).is_err());
    for (position, completion) in completions.iter().enumerate() {
        v.snapshots = snapshots[..position + 3].to_vec();
        record_completion(&mut v, completion, 20 + position as u64).unwrap();
        assert!(record_completion(&mut v, completion, 1).is_err());
    }
    assert!(record_close(&mut v, 1).is_err());
    v.snapshots = snapshots;
    record_close(&mut v, 30).unwrap();
    v.validate_for_policy(Policy::OrderedSharedFull).unwrap();
    v.validate().unwrap();
    assert!(record_close(&mut v, 30).is_err());
    assert_eq!(v.serialization_host_ns, [20, 21, 22, 23]);
}
