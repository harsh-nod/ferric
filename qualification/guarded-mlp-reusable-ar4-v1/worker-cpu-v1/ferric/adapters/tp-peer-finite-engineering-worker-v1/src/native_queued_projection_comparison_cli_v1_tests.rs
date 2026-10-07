use super::*;
use crate::finite_queued_projection_comparison_wire_v1::tests as fixtures;
use crate::forward_sequence::ForwardCompletion;
use crate::native_catalog::forward::ForwardRun;
use crate::resident_layer::LayerCompletion;

const REG: [u8; 32] = [9; 32];

fn args() -> Vec<OsString> {
    [
        "--engineering-native-queued-projection-comparison-v1",
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
        let mut comparison = queued::Comparison {
            profile: queued::ComparisonProfile::FiniteThenQueuedProjectionsLayerZeroV1,
            finite_states: value.finite_states,
            finite_prefix_host_ns: value.finite_prefix_host_ns,
            queued_stage_host_ns: value.queued_stage_host_ns,
            equality: value.equality.map(|pair| {
                pair.map(|row| queued::OutputEquality {
                    stage: match row.stage {
                        wire::Stage::Query => queued::Stage::Query,
                        wire::Stage::Key => queued::Stage::Key,
                        wire::Stage::Value => queued::Stage::Value,
                        wire::Stage::Output => queued::Stage::Output,
                    },
                    rank: row.rank as usize,
                    bytes: row.bytes as usize,
                    words: row.words as usize,
                    sha256: row.sha256,
                })
            }),
        };
        if self.malformed_report {
            comparison.finite_prefix_host_ns[0] += 1;
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
    assert!(crate::native_long_cli_v1::parse_args(&args()).is_err());
    assert!(crate::native_rearm_smoke_cli_v1::parse_args(&args()).is_err());
    assert!(crate::native_queued_mlp_comparison_cli_v1::parse_args(&args()).is_err());
    assert!(parse_args(&[]).is_err());
    for (index, value) in [
        (0, "--engineering-native-v1"),
        (0, "--engineering-native-queued-mlp-comparison-v1"),
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
    assert_eq!(report.finite_states, control.layers[0].prefix_states);
    assert_eq!(report.finite_prefix_host_ns, [3, 4]);
    assert_eq!(report.queued_stage_host_ns, [[21, 22]; 4]);
    assert!(!report.queued_semantic_state && !report.performance_claim);
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
fn retained_wave_extraction_uses_exact_typed_begin_extents_and_pin() {
    fn setup_part(bytes: &[u8]) -> setup::Part {
        let pin = part(bytes);
        setup::Part {
            bytes: pin.bytes,
            sha256: pin.sha256,
        }
    }
    let pieces: [&[u8]; 7] = [
        b"registration",
        b"program",
        b"uploads",
        b"prefix",
        b"mlp",
        b"residual",
        b"wave",
    ];
    let begin = setup::Begin {
        scope: fixtures::bootstrap().scope,
        registration: setup_part(pieces[0]),
        source_program: setup_part(pieces[1]),
        uploads: setup_part(pieces[2]),
        prefix_image: setup_part(pieces[3]),
        mlp_image: setup_part(pieces[4]),
        residual_image: setup_part(pieces[5]),
        tail_image: Some(setup_part(pieces[6])),
    };
    let request = setup::Request {
        protocol: 1,
        id: 1,
        device_ids: [10, 20],
        session: [3; 32],
        command: setup::Command::Begin(begin),
    };
    let bytes = pieces.concat();
    assert_eq!(retained_wave(&request, &bytes).unwrap(), b"wave");
    for changed in 0..5 {
        let mut bad = request.clone();
        let mut body = bytes.clone();
        let setup::Command::Begin(begin) = &mut bad.command else {
            panic!()
        };
        match changed {
            0 => begin.tail_image = None,
            1 => begin.tail_image.as_mut().unwrap().sha256[0] ^= 1,
            2 => begin.registration.bytes += 1,
            3 => body.push(0),
            _ => {
                body.pop();
            }
        }
        assert!(retained_wave(&bad, &body).is_err());
    }
    let mut bad = request;
    bad.command = setup::Command::AbortClose;
    assert!(retained_wave(&bad, &bytes).is_err());
    // This tests extraction only: none of these synthetic bytes admits an image.
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
fn wrong_actual_pid_bootstrap_is_rejected_before_reading_begin() {
    let options = parse_args(&args()).unwrap();
    let mut bootstrap = fixtures::bootstrap();
    bootstrap.scope.child_identity = if std::process::id() == 1 { 2 } else { 1 };
    let mut bytes = Vec::new();
    wire::write_bootstrap(&mut bytes, &mut FrameBudget::new(), &bootstrap).unwrap();
    bytes.extend_from_slice(b"unread Begin");
    let mut reader = bytes.as_slice();
    assert!(prepare_native(&options, &mut reader, &mut FrameBudget::new()).is_err());
    assert_eq!(reader, b"unread Begin");
}
