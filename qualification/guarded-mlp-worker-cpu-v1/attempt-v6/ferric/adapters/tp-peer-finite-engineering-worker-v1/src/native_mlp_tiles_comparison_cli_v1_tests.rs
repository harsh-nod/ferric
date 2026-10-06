use super::*;
use crate::finite_mlp_tiles_comparison_wire_v1::tests as fixtures;
use crate::forward_sequence::ForwardCompletion;
use crate::native_catalog::forward::ForwardRun;
use crate::resident_layer::LayerCompletion;

const REG: [u8; 32] = [9; 32];

fn args() -> Vec<OsString> {
    [
        "--engineering-native-mlp-tiles-comparison-v1",
        "--allow-unauthenticated-machine-code",
        "--devices",
        "10,20",
        "--timeout-ms",
        "1000",
    ]
    .into_iter()
    .map(OsString::from)
    .collect()
}

#[derive(Default)]
struct Fake {
    calls: usize,
    closes: usize,
    fail_run: bool,
    fail_close: bool,
    malformed_report: bool,
}
impl Backend for Fake {
    fn run(&mut self, input: &ForwardInput) -> io::Result<ComparisonRun> {
        self.calls += 1;
        if self.fail_run {
            return Err(io::Error::other("injected comparison failure"));
        }
        let control = fixtures::control();
        let value = fixtures::report(&control);
        let mut comparison = tiles::Comparison {
            finite_states: value.finite_states,
            finite_queue_host_ns: value.finite_queue_host_ns,
            tiles_states: value.tiles_states.map(|v| v.try_into().unwrap()),
            tiles_queue_host_ns: value.tiles_queue_host_ns,
            equality: value.equality,
        };
        if self.malformed_report {
            comparison.finite_queue_host_ns[0] += 1;
        }
        Ok(ComparisonRun {
            comparison,
            forward: ForwardRun {
                completion: ForwardCompletion {
                    generation: input.generation,
                    position: input.cache_metadata[0],
                    input_token: input.token,
                    output_token: 0,
                    embedding_ns: control.embedding_ns,
                    tail_ns: control.tail_ns,
                    layers: control
                        .layers
                        .into_iter()
                        .map(|layer| LayerCompletion {
                            prefix_states: layer.prefix_states,
                            mlp_states: layer.mlp_states,
                            paired_ns: layer.paired_ns,
                        })
                        .collect(),
                },
                layer_hidden: vec![vec![0; 8192]; 36],
                final_normalized: vec![0; 8192],
                logits: vec![0; 303872],
            },
        })
    }
    fn close(&mut self) -> io::Result<()> {
        self.closes += 1;
        if self.fail_close {
            Err(io::Error::other("injected close failure"))
        } else {
            Ok(())
        }
    }
}

fn input(include_close: bool) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut budget = FrameBudget::new();
    wire::write_request(&mut bytes, &mut budget, &fixtures::request(false)).unwrap();
    if include_close {
        wire::write_request(&mut bytes, &mut budget, &fixtures::request(true)).unwrap();
    }
    bytes
}
fn serve(fake: &mut Fake, bytes: &[u8], output: &mut impl Write) -> io::Result<()> {
    let mut reader = bytes;
    serve_forwards(
        fake,
        &mut reader,
        output,
        &fixtures::bootstrap(),
        REG,
        &mut FrameBudget::new(),
    )
}

#[test]
fn comparison_cli_is_explicit_closed_and_does_not_select_an_old_route() {
    assert_eq!(
        parse_args(&args()).unwrap(),
        NativeOptions {
            devices: [10, 20],
            timeout_ms: 1000
        }
    );
    assert!(crate::native_cli_v1::parse_args(&args()).is_err());
    assert!(crate::native_queued_mlp_comparison_cli_v1::parse_args(&args()).is_err());
    assert!(crate::native_long_cli_v1::parse_args(&args()).is_err());
    assert!(crate::native_rearm_smoke_cli_v1::parse_args(&args()).is_err());
    assert!(parse_args(&[]).is_err());
    for (index, value) in [
        (0, "--engineering-native-v1"),
        (1, "--missing-opt-in"),
        (3, "10,10"),
        (3, "0,20"),
        (3, "10,20,30"),
        (3, "+10,20"),
        (5, "0"),
        (5, "10001"),
        (5, "4294967296"),
    ] {
        let mut bad = args();
        bad[index] = value.into();
        assert!(parse_args(&bad).is_err());
    }
    let mut extra = args();
    extra.push("--capture-layer0".into());
    assert!(parse_args(&extra).is_err());
}

#[test]
fn healthy_one_forward_report_is_followed_by_actual_close_acknowledgement() {
    let mut fake = Fake::default();
    let mut output = Vec::new();
    serve(&mut fake, &input(true), &mut output).unwrap();
    assert_eq!((fake.calls, fake.closes), (1, 1));
    let mut reader = output.as_slice();
    let mut budget = FrameBudget::new();
    let (response, control, main, comparison) = wire::read_response(&mut reader, &mut budget)
        .unwrap()
        .unwrap();
    let Event::Completed(c) = response.event else {
        panic!("missing actual forward");
    };
    assert!(!response.native_closed);
    assert_eq!(main.len(), OBSERVATION_BYTES);
    let control = control.unwrap();
    let report = wire::validate_comparison(&comparison, &control).unwrap();
    assert_eq!(report.finite_states, control.layers[0].mlp_states);
    assert_eq!(report.finite_queue_host_ns, [7, 8]);
    assert_eq!(report.tiles_queue_host_ns, [21, 22]);
    assert_eq!(report.tiles_states[0].len(), 548);
    assert!(!report.v2_full_model && !report.performance_claim);
    let mut chain = Chain::new(REG, fixtures::bootstrap().sha256().unwrap());
    assert_eq!(chain.advance(&c), c.chain);
    let (closed, state, main, report) = wire::read_response(&mut reader, &mut budget)
        .unwrap()
        .unwrap();
    assert_eq!(
        closed.event,
        Event::Closed {
            completed_forwards: 1,
            transcript_sha256: chain.digest()
        }
    );
    assert!(closed.native_closed && state.is_none() && main.is_empty() && report.is_empty());
    assert!(reader.is_empty());
}

#[test]
fn failed_forward_eof_and_failed_close_never_emit_closed_or_retry() {
    for failure in 0..3 {
        let mut fake = Fake {
            fail_run: failure == 0,
            fail_close: failure == 2,
            ..Fake::default()
        };
        let mut output = Vec::new();
        assert!(serve(&mut fake, &input(failure != 1), &mut output).is_err());
        assert_eq!(fake.calls, 1);
        assert_eq!(fake.closes, usize::from(failure == 2));
        let mut reader = output.as_slice();
        let mut budget = FrameBudget::new();
        let mut completed = 0;
        while let Some((response, _, _, _)) = wire::read_response(&mut reader, &mut budget).unwrap()
        {
            assert!(matches!(response.event, Event::Completed(_)));
            assert!(!response.native_closed);
            completed += 1;
        }
        assert_eq!(completed, usize::from(failure != 0));
    }
}

#[test]
fn wrong_custody_or_repeated_forward_is_rejected_before_another_backend_call() {
    for changed in 0..4 {
        let mut request = fixtures::request(false);
        match changed {
            0 => request.device_ids = [10, 21],
            1 => request.session[0] ^= 1,
            2 => request.registration[0] ^= 1,
            _ => request.profile_sha256[0] ^= 1,
        }
        let mut bytes = Vec::new();
        wire::write_request(&mut bytes, &mut FrameBudget::new(), &request).unwrap();
        let mut fake = Fake::default();
        let mut output = Vec::new();
        assert!(serve(&mut fake, &bytes, &mut output).is_err());
        assert_eq!((fake.calls, fake.closes), (0, 0));
        assert!(output.is_empty());
    }
    let mut bytes = input(false);
    wire::write_request(
        &mut bytes,
        &mut FrameBudget::new(),
        &fixtures::request(false),
    )
    .unwrap();
    let mut fake = Fake::default();
    assert!(serve(&mut fake, &bytes, &mut Vec::new()).is_err());
    assert_eq!((fake.calls, fake.closes), (1, 0));
}

#[test]
fn report_must_join_actual_finite_control_before_any_response() {
    let mut fake = Fake {
        malformed_report: true,
        ..Fake::default()
    };
    let mut output = Vec::new();
    assert!(serve(&mut fake, &input(true), &mut output).is_err());
    assert_eq!((fake.calls, fake.closes), (1, 0));
    assert!(output.is_empty());
}

#[test]
fn lost_response_is_fatal_without_retry_or_claimed_close() {
    struct Broken;
    impl Write for Broken {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "lost parent"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut fake = Fake::default();
    assert!(serve(&mut fake, &input(true), &mut Broken).is_err());
    assert_eq!((fake.calls, fake.closes), (1, 0));
}

#[test]
fn incomplete_whole_forward_setup_refuses_in_the_preopener() {
    let mut boot = fixtures::bootstrap();
    boot.scope.child_identity = std::process::id();
    let image = b"synthetic custody only";
    boot.tiles_image = part(image);
    let pin = part(b"x");
    let begin = setup::Begin {
        scope: boot.scope.clone(),
        registration: pin,
        source_program: pin,
        uploads: pin,
        prefix_image: pin,
        mlp_image: pin,
        residual_image: pin,
        tail_image: None,
    };
    let request = setup::Request {
        protocol: 1,
        id: 1,
        device_ids: boot.device_ids,
        session: boot.scope.session,
        command: setup::Command::Begin(begin),
    };
    let mut input = Vec::new();
    wire::write_bootstrap(&mut input, &mut FrameBudget::new(), &boot, image).unwrap();
    setup::write_request(&mut input, &request, b"xxxxxx").unwrap();
    let error = match prepare_native(
        &parse_args(&args()).unwrap(),
        &mut input.as_slice(),
        &mut FrameBudget::new(),
    ) {
        Ok(_) => panic!("incomplete setup unexpectedly prepared"),
        Err(error) => error,
    };
    assert!(
        error
            .to_string()
            .contains("complete tail setup before open")
    );
}
