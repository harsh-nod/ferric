use super::*;
use crate::finite_prefix_layer_wire_v1::tests::{bootstrap, control};
use crate::native_catalog::forward::prefix_tiles_layer_v6::Profile as ActualProfile;
use crate::resident_layer::prefix_tiles_v6::{Capture, Completion, Run};
fn args(profile: Profile) -> Vec<OsString> {
    [
        "--engineering-native-prefix-layer-v1",
        "--allow-unauthenticated-machine-code",
        "--devices",
        "7,9",
        "--timeout-ms",
        "1000",
        "--profile",
        if profile == Profile::Baseline22Mlp548 {
            "baseline22-mlp548"
        } else {
            "prefix284-mlp548"
        },
    ]
    .into_iter()
    .map(OsString::from)
    .collect()
}
fn closed(b: &Bootstrap) -> ClosedRun {
    let c = control(b.profile);
    let prefix = if b.profile == Profile::Baseline22Mlp548 {
        PrefixObservation::Baseline22(c.prefix.map(|v| v.try_into().unwrap()))
    } else {
        PrefixObservation::Tiles284(c.prefix.map(|v| v.try_into().unwrap()))
    };
    ClosedRun {
        profile_sha256: b.sha256().unwrap(),
        generation: 1,
        position: 0,
        input_token: 9112,
        embedding_ns: [0; 2],
        layer: Run {
            completion: Completion {
                prefix,
                mlp: c.mlp.map(|v| v.try_into().unwrap()),
                paired_ns: [[0; 2]; 4],
            },
            capture: Capture {
                prefix: core::array::from_fn(|_| {
                    core::array::from_fn(|i| vec![0; wire::STAGES[i].1])
                }),
                first_residual: core::array::from_fn(|_| vec![0; 8192]),
                mlp: core::array::from_fn(|_| {
                    core::array::from_fn(|i| vec![0; wire::STAGES[8 + i].1])
                }),
                final_hidden: core::array::from_fn(|_| vec![0; 8192]),
            },
        },
    }
}
struct Fake {
    b: Bootstrap,
    runs: usize,
    closes: usize,
    run_fail: bool,
    close_fail: bool,
    bad_state: bool,
}
impl Fake {
    fn new(b: &Bootstrap) -> Self {
        Self {
            b: b.clone(),
            runs: 0,
            closes: 0,
            run_fail: false,
            close_fail: false,
            bad_state: false,
        }
    }
}
impl Backend for Fake {
    fn run(&mut self, i: &ForwardInput) -> io::Result<()> {
        self.runs += 1;
        assert_eq!(i.registration, self.b.begin.registration.sha256);
        assert_eq!(i.token, 9112);
        if self.run_fail {
            Err(io::Error::other("run failure"))
        } else {
            Ok(())
        }
    }
    fn close(&mut self) -> io::Result<ClosedRun> {
        self.closes += 1;
        if self.close_fail {
            return Err(io::Error::other("close failure"));
        }
        let mut c = closed(&self.b);
        if self.bad_state {
            c.layer.completion.mlp[1][547] = 63;
        }
        Ok(c)
    }
}
fn requests(b: &Bootstrap) -> Vec<u8> {
    let mut out = Vec::new();
    let mut budget = Budget::new();
    for (id, command) in [(1, Command::Run), (2, Command::Close)] {
        wire::write_request(
            &mut out,
            &mut budget,
            &wire::Request {
                protocol: 1,
                id,
                profile_sha256: b.sha256().unwrap(),
                command,
            },
        )
        .unwrap();
    }
    out
}
#[test]
fn wire_digest_matches_actual_backend_for_both_profiles() {
    for p in [Profile::Baseline22Mlp548, Profile::Prefix284Mlp548] {
        let b = bootstrap(p);
        let actual = ActualProfile::new(
            &b.begin.scope,
            b.begin.registration.sha256,
            b.prefix_image.map(|v| v.sha256),
            b.mlp_image.sha256,
            &input(&b).unwrap(),
            b.timeout_ms,
            b.device_ids,
        )
        .unwrap();
        assert_eq!(b.sha256().unwrap(), actual.sha256());
    }
}
#[test]
fn argv_has_no_fallback_cache_or_mode_expansion() {
    for p in [Profile::Baseline22Mlp548, Profile::Prefix284Mlp548] {
        let good = args(p);
        assert_eq!(
            parse_args(&good).unwrap(),
            NativeOptions {
                devices: [7, 9],
                timeout_ms: 1000,
                profile: p
            }
        );
        for n in 0..good.len() {
            assert!(parse_args(&good[..n]).is_err());
        }
        for v in ["--kernel-admission", "--mode", "--forwards"] {
            let mut bad = good.clone();
            bad.push(v.into());
            assert!(parse_args(&bad).is_err());
        }
        for (i, v) in [
            (3, "7,7"),
            (3, "0,9"),
            (5, "10001"),
            (5, "+1"),
            (7, "baseline"),
        ] {
            let mut bad = good.clone();
            bad[i] = v.into();
            assert!(parse_args(&bad).is_err());
        }
    }
}
#[test]
fn bootstrap_refusals_precede_setup_and_device_open() {
    let b = bootstrap(Profile::Prefix284Mlp548);
    let options = parse_args(&args(b.profile)).unwrap();
    for which in 0..3 {
        let mut bad = b.clone();
        match which {
            0 => bad.begin.tail_image = None,
            1 => bad.begin.scope.child_identity = 0,
            _ => bad.profile = Profile::Baseline22Mlp548,
        };
        let json = serde_json::to_vec(&bad).unwrap();
        let mut raw = (json.len() as u32).to_le_bytes().to_vec();
        raw.extend(json);
        // No bodies, Begin or opener exist in this fixture.
        assert!(prepare(&options, &mut raw.as_slice(), &mut Budget::new()).is_err());
    }
}
#[test]
fn one_run_then_close_emits_exact_closed_capture_and_control() {
    for p in [Profile::Baseline22Mlp548, Profile::Prefix284Mlp548] {
        let b = bootstrap(p);
        let mut fake = Fake::new(&b);
        let mut output = Vec::new();
        serve(
            &mut fake,
            &mut requests(&b).as_slice(),
            &mut output,
            &b,
            &mut Budget::new(),
        )
        .unwrap();
        assert_eq!((fake.runs, fake.closes), (1, 1));
        let mut input = output.as_slice();
        let mut budget = Budget::new();
        let (first, body) = wire::read_response(&mut input, &mut budget).unwrap();
        assert!(!first.native_closed);
        assert!(body.is_empty());
        let (last, body) = wire::read_response(&mut input, &mut budget).unwrap();
        assert!(last.native_closed);
        assert_eq!(body.len(), wire::CAPTURE_BYTES);
        assert_eq!(last.control, Some(control(p)));
        assert!(input.is_empty());
    }
}
#[test]
fn run_close_and_postclose_validation_errors_never_emit_success_or_retry() {
    let b = bootstrap(Profile::Prefix284Mlp548);
    for which in 0..3 {
        let mut f = Fake::new(&b);
        match which {
            0 => f.run_fail = true,
            1 => f.close_fail = true,
            _ => f.bad_state = true,
        };
        let mut out = Vec::new();
        assert!(
            serve(
                &mut f,
                &mut requests(&b).as_slice(),
                &mut out,
                &b,
                &mut Budget::new()
            )
            .is_err()
        );
        assert_eq!(f.runs, 1);
        assert_eq!(f.closes, usize::from(which != 0));
        if which == 0 {
            assert!(out.is_empty());
        } else {
            let mut bytes = out.as_slice();
            assert_eq!(
                wire::read_response(&mut bytes, &mut Budget::new())
                    .unwrap()
                    .0
                    .id,
                1
            );
            assert!(bytes.is_empty());
        }
    }
}
#[test]
fn partial_request_eof_does_not_guess_close() {
    let b = bootstrap(Profile::Baseline22Mlp548);
    let raw = requests(&b);
    for n in [0, 3, raw.len() - 1] {
        let mut f = Fake::new(&b);
        let mut output = Vec::new();
        let mut truncated = &raw[..n];
        assert!(serve(&mut f, &mut truncated, &mut output, &b, &mut Budget::new()).is_err());
        assert_eq!(f.closes, 0);
    }
}
#[test]
fn response_write_loss_is_fatal_and_never_retries_close() {
    struct Fail {
        remaining: usize,
    }
    impl Write for Fail {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            if self.remaining == 0 {
                return Err(io::Error::other("write loss"));
            }
            let n = b.len().min(self.remaining);
            self.remaining -= n;
            Ok(n)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let b = bootstrap(Profile::Prefix284Mlp548);
    let mut f = Fake::new(&b);
    assert!(
        serve(
            &mut f,
            &mut requests(&b).as_slice(),
            &mut Fail { remaining: 0 },
            &b,
            &mut Budget::new()
        )
        .is_err()
    );
    assert_eq!((f.runs, f.closes), (1, 0));
    let first = wire::Response {
        protocol: 1,
        id: 1,
        profile_sha256: b.sha256().unwrap(),
        profile: b.profile,
        native_closed: false,
        completed_layers: 1,
        control: None,
        capture: None,
        gpu_execution: true,
        numerical_acceptance: false,
        performance_claim: false,
        production_authority: false,
    };
    let mut raw = Vec::new();
    wire::write_response(&mut raw, &mut Budget::new(), &first, &[]).unwrap();
    let mut f = Fake::new(&b);
    assert!(
        serve(
            &mut f,
            &mut requests(&b).as_slice(),
            &mut Fail {
                remaining: raw.len() + 4
            },
            &b,
            &mut Budget::new()
        )
        .is_err()
    );
    assert_eq!((f.runs, f.closes), (1, 1));
}
#[test]
fn wrong_profile_before_run_and_wrong_closed_identity_are_refused() {
    let b = bootstrap(Profile::Prefix284Mlp548);
    let mut raw = Vec::new();
    wire::write_request(
        &mut raw,
        &mut Budget::new(),
        &wire::Request {
            protocol: 1,
            id: 1,
            profile_sha256: [9; 32],
            command: Command::Run,
        },
    )
    .unwrap();
    let mut f = Fake::new(&b);
    assert!(
        serve(
            &mut f,
            &mut raw.as_slice(),
            &mut Vec::new(),
            &b,
            &mut Budget::new()
        )
        .is_err()
    );
    assert_eq!((f.runs, f.closes), (0, 0));
    let mut c = closed(&b);
    c.position = 1;
    assert!(encode_closed(&b, c).is_err());
    let mut c = closed(&b);
    c.layer.capture.mlp[1][4].pop();
    assert!(encode_closed(&b, c).is_err());
}
