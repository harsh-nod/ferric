use super::*;
use crate::finite_long_wire_v1::Profile;

fn args() -> Vec<OsString> {
    [
        "--engineering-native-long-v1",
        "--allow-unauthenticated-machine-code",
        "--devices",
        "11,22",
        "--timeout-ms",
        "1000",
    ]
    .into_iter()
    .map(OsString::from)
    .collect()
}
fn bootstrap() -> Bootstrap {
    Bootstrap {
        protocol: 1,
        profile: Profile::Prompt2048Output256V1,
        device_ids: [11, 22],
        timeout_ms: 1000,
        scope: setup::Scope {
            bundle_id: [1; 32],
            model_id: [2; 32],
            session: [3; 32],
            pool_identity: 1,
            group_id: 0,
            child_identity: std::process::id(),
        },
        prompt_tokens: vec![7; 2048],
    }
}
fn request() -> wire::Request {
    wire::Request {
        protocol: 1,
        id: 1,
        device_ids: [11, 22],
        session: [3; 32],
        registration: [4; 32],
        profile_sha256: bootstrap().sha256().unwrap(),
        command: Command::Forward {
            generation: 1,
            token: 7,
            cache_metadata: core::iter::once(0).chain(0..144).collect(),
            rotary_bits: vec![0; 128],
        },
    }
}
#[derive(Default)]
struct Fake {
    runs: usize,
    closes: usize,
    close_fail: bool,
}
impl Backend for Fake {
    fn run(&mut self, _: &ForwardInput) -> io::Result<ForwardRun> {
        self.runs += 1;
        Err(io::Error::other("injected native failure"))
    }
    fn close(&mut self) -> io::Result<()> {
        self.closes += 1;
        if self.close_fail {
            Err(io::Error::other("injected close failure"))
        } else {
            Ok(())
        }
    }
}

#[test]
fn long_cli_requires_separate_optin_and_exact_closed_arguments() {
    assert_eq!(
        parse_args(&args()).unwrap(),
        NativeOptions {
            devices: [11, 22],
            timeout_ms: 1000
        }
    );
    assert!(crate::native_cli_v1::parse_args(&args()).is_err());
    for (index, replacement) in [
        (0, "--engineering-native"),
        (1, "--missing"),
        (3, "11,11"),
        (3, "11,22,33"),
        (3, "+11,22"),
        (5, "0"),
        (5, "10001"),
    ] {
        let mut bad = args();
        bad[index] = replacement.into();
        assert!(parse_args(&bad).is_err());
    }
    let mut extra = args();
    extra.push("--input-mode".into());
    assert!(parse_args(&extra).is_err());
}

#[test]
fn wrong_bootstrap_pid_is_rejected_before_any_setup_or_opener() {
    let mut bad = bootstrap();
    bad.scope.child_identity = std::process::id().checked_add(1).unwrap();
    let mut bytes = Vec::new();
    wire::write_bootstrap(&mut bytes, &mut FrameBudget::new(), &bad).unwrap();
    let result = prepare_native(
        &parse_args(&args()).unwrap(),
        &mut bytes.as_slice(),
        &mut FrameBudget::new(),
    );
    assert!(result.err().unwrap().to_string().contains("bootstrap"));
}

#[test]
fn correct_bootstrap_without_begin_remains_cpu_only_failure() {
    let mut bytes = Vec::new();
    wire::write_bootstrap(&mut bytes, &mut FrameBudget::new(), &bootstrap()).unwrap();
    let result = prepare_native(
        &parse_args(&args()).unwrap(),
        &mut bytes.as_slice(),
        &mut FrameBudget::new(),
    );
    assert!(result.err().unwrap().to_string().contains("Begin"));
}

#[test]
fn eof_or_profile_prompt_identity_mismatch_cannot_dispatch_or_close() {
    for mutation in 0..5 {
        let mut backend = Fake::default();
        let mut input = Vec::new();
        if mutation != 0 {
            let mut r = request();
            match mutation {
                1 => r.profile_sha256 = [8; 32],
                2 => r.registration = [9; 32],
                3 => r.device_ids.swap(0, 1),
                _ => {
                    if let Command::Forward { token, .. } = &mut r.command {
                        *token = 8;
                    }
                }
            }
            wire::write_request(&mut input, &mut FrameBudget::new(), &r).unwrap();
        }
        let mut output = Vec::new();
        assert!(
            serve_forwards(
                &mut backend,
                &mut input.as_slice(),
                &mut output,
                &bootstrap(),
                [4; 32],
                &mut FrameBudget::new()
            )
            .is_err()
        );
        assert_eq!(backend.runs, 0);
        assert_eq!(backend.closes, 0);
        assert!(output.is_empty());
    }
}

#[test]
fn native_failure_is_not_retried_or_acknowledged_as_completed() {
    let mut input = Vec::new();
    wire::write_request(&mut input, &mut FrameBudget::new(), &request()).unwrap();
    let mut backend = Fake::default();
    let mut output = Vec::new();
    assert!(
        serve_forwards(
            &mut backend,
            &mut input.as_slice(),
            &mut output,
            &bootstrap(),
            [4; 32],
            &mut FrameBudget::new()
        )
        .is_err()
    );
    assert_eq!(backend.runs, 1);
    assert_eq!(backend.closes, 0);
    assert!(output.is_empty());
}

#[test]
fn final_ack_requires_actual_successful_close_and_preserves_transcript() {
    let mut r = request();
    r.id = 2304;
    r.command = Command::Close;
    let chain = Chain::new(r.registration, r.profile_sha256);
    for failed in [false, true] {
        let mut backend = Fake {
            close_fail: failed,
            ..Fake::default()
        };
        let mut output = Vec::new();
        let result = healthy_close(
            &mut backend,
            &mut output,
            &mut FrameBudget::new(),
            &r,
            &chain,
        );
        assert_eq!(backend.closes, 1);
        if failed {
            assert!(result.is_err());
            assert!(output.is_empty());
        } else {
            result.unwrap();
            let (response, control, payload) =
                wire::read_response(&mut output.as_slice(), &mut FrameBudget::new())
                    .unwrap()
                    .unwrap();
            assert!(control.is_none());
            assert!(payload.is_empty());
            assert!(response.native_closed);
            assert_eq!(
                response.event,
                Event::Closed {
                    completed_forwards: 2303,
                    transcript_sha256: chain.digest()
                }
            );
        }
    }
}
