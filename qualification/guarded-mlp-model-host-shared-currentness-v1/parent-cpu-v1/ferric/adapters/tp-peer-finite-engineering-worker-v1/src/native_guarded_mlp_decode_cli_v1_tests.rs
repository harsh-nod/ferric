use super::*;
use crate::native_catalog::forward::guarded_mlp_decode_v1::Completion as NativeCompletion;
use crate::resident_layer::guarded_mlp_decode_v1::Completion as LayerCompletion;
use fe2o3_kfd::Gfx950EngineeringPeerGuardedMlpObservationV1;
use wire::tests::{bootstrap, control, request};

struct Fake {
    profile: [u8; 32],
    calls: Vec<u64>,
    fail: Option<usize>,
    closed: bool,
    failed: usize,
    corrupt: bool,
}
impl Fake {
    fn new(b: &Bootstrap) -> Self {
        Self {
            profile: b.sha256().unwrap(),
            calls: Vec::new(),
            fail: None,
            closed: false,
            failed: 0,
            corrupt: false,
        }
    }
}
impl Backend for Fake {
    fn run(&mut self, input: &ForwardInput) -> io::Result<Run> {
        let at = self.calls.len();
        self.calls.push(input.generation);
        if self.fail == Some(at) {
            return Err(io::Error::other("injected forward"));
        }
        let c = control(input.generation);
        let output = 100 + input.generation as u32;
        let layers = c
            .layers
            .into_iter()
            .map(|l| LayerCompletion {
                prefix_states: l.prefix_states,
                prefix_ns: l.prefix_host_ns,
                guarded: Gfx950EngineeringPeerGuardedMlpObservationV1 {
                    prefixes: l.mlp_prefixes,
                    guards: l.guards,
                    observed_queue_frontiers: l.observed_queue_frontiers,
                    segment_host_ns: l.segment_host_ns,
                },
            })
            .collect();
        let mut logits = vec![0; 303872];
        logits[output as usize * 2..][..2].copy_from_slice(&0x3f80u16.to_le_bytes());
        let mut run = Run {
            completion: NativeCompletion {
                profile_sha256: self.profile,
                generation: input.generation,
                position: input.cache_metadata[0],
                input_token: input.token,
                output_token: output,
                embedding_ns: c.embedding_ns,
                layers,
                tail_ns: c.tail_ns,
            },
            layer_hidden: vec![vec![0; 8192]; 36],
            final_normalized: vec![0; 8192],
            logits,
        };
        if self.corrupt {
            run.completion.layers[0].guarded.guards[1][2] = 0;
        }
        Ok(run)
    }
    fn close(&mut self) -> io::Result<()> {
        if self.fail == Some(4) {
            return Err(io::Error::other("injected close"));
        }
        self.closed = true;
        Ok(())
    }
    fn failed(&mut self) {
        self.failed += 1;
    }
}
fn requests(b: &Bootstrap) -> Vec<wire::Request> {
    let mut values = Vec::new();
    for position in 0..4 {
        values.push(request(
            b,
            position,
            if position == 0 {
                None
            } else {
                Some(100 + position)
            },
        ));
    }
    values.push(wire::Request {
        protocol: wire::PROTOCOL,
        id: 5,
        device_ids: b.decode.device_ids,
        session: b.decode.scope.session,
        registration: b.decode.registration,
        profile_sha256: b.sha256().unwrap(),
        command: Command::Close,
    });
    values
}
fn stream(values: &[wire::Request]) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut budget = FrameBudget::new();
    for r in values {
        crate::finite_long_wire_v1::write_header(&mut bytes, &mut budget, r).unwrap();
    }
    bytes
}
fn args(mode: &str) -> Vec<OsString> {
    [
        FLAG,
        "--allow-unauthenticated-machine-code",
        "--devices",
        "11,12",
        "--timeout-ms",
        "100",
        "--mode",
        mode,
    ]
    .into_iter()
    .map(OsString::from)
    .collect()
}
#[test]
fn guarded_cli_is_explicit_plain_tf4_ar4_without_legacy_or_clock_fallback() {
    for mode in ["teacher-forced", "autoregressive"] {
        let a = args(mode);
        let parsed = parse_args(&a).unwrap();
        assert_eq!(parsed.devices, [11, 12]);
        assert_eq!(parsed.timeout_ms, 100);
        for (index, value) in [
            (0, "--engineering-native-projection-residual-decode-v1"),
            (1, "--production"),
            (3, "11,11"),
            (3, "0,12"),
            (3, "+11,12"),
            (5, "10001"),
            (7, "decode"),
        ] {
            let mut bad = a.clone();
            bad[index] = value.into();
            assert!(parse_args(&bad).is_err());
        }
        for n in 0..a.len() {
            assert!(parse_args(&a[..n]).is_err());
        }
        let mut extra = a.clone();
        extra.push("--clock".into());
        assert!(parse_args(&extra).is_err());
    }
}

#[test]
fn guarded_capture_cli_is_distinct_and_never_selected_by_default() {
    for mode in ["teacher-forced", "autoregressive"] {
        let original = args(mode);
        assert!(parse_capture_args(&original).is_err());
        let mut capture = original.clone();
        capture[0] = CAPTURE_FLAG.into();
        assert!(parse_args(&capture).is_err());
        let parsed = parse_capture_args(&capture).unwrap();
        assert_eq!(parsed.devices, [11, 12]);
        assert_eq!(parsed.timeout_ms, 100);
        for n in 0..capture.len() {
            assert!(parse_capture_args(&capture[..n]).is_err());
        }
        capture.push(FLAG.into());
        assert!(parse_capture_args(&capture).is_err());
    }
}

#[test]
fn guarded_host_cli_is_explicit_exclusive_and_preserves_runtime_options() {
    for mode in ["teacher-forced", "autoregressive"] {
        let original = args(mode);
        assert!(parse_host_args(&original).is_err());
        let mut host = original.clone();
        host[0] = HOST_FLAG.into();
        assert!(parse_args(&host).is_err());
        assert!(parse_capture_args(&host).is_err());
        let parsed = parse_host_args(&host).unwrap();
        assert_eq!(parsed.devices, [11, 12]);
        assert_eq!(parsed.timeout_ms, 100);
        for n in 0..host.len() {
            assert!(parse_host_args(&host[..n]).is_err());
        }
        for flag in [
            FLAG,
            CAPTURE_FLAG,
            "--shared-full",
            "--cache-kernel-admission",
        ] {
            let mut extra = host.clone();
            extra.push(flag.into());
            assert!(parse_host_args(&extra).is_err());
        }
    }
}

#[test]
fn guarded_shared_host_cli_is_distinct_and_preserves_closed_runtime_options() {
    assert_eq!(HOST_FLAG, HostPolicy::DefaultFull.worker_flag());
    for mode in ["teacher-forced", "autoregressive"] {
        let original = args(mode);
        assert!(parse_host_shared_args(&original).is_err());
        let mut selected = original.clone();
        selected[0] = HOST_SHARED_FLAG.into();
        assert!(parse_args(&selected).is_err());
        assert!(parse_host_args(&selected).is_err());
        assert!(parse_capture_args(&selected).is_err());
        let parsed = parse_host_shared_args(&selected).unwrap();
        let prior = parse_args(&original).unwrap();
        assert_eq!(parsed.devices, prior.devices);
        assert_eq!(parsed.timeout_ms, prior.timeout_ms);
        assert_eq!(parsed.mode, prior.mode);
        for length in 0..selected.len() {
            assert!(parse_host_shared_args(&selected[..length]).is_err());
        }
        for flag in [FLAG, CAPTURE_FLAG, HOST_FLAG, "--profile", "--operational-currentness",
            "--cache-kernel-admission"] {
            let mut extra = selected.clone();
            extra.push(flag.into());
            assert!(parse_host_shared_args(&extra).is_err());
        }
        selected[0] = HOST_FLAG.into();
        assert!(parse_host_shared_args(&selected).is_err());
        parse_host_args(&selected).unwrap();
    }
}
#[test]
fn guarded_cli_four_forwards_close_with_exact_observations_and_distinct_transcript() {
    for mode in [InputMode::TeacherForced, InputMode::Autoregressive] {
        let b = bootstrap(mode);
        let mut backend = Fake::new(&b);
        let mut output = Vec::new();
        serve(
            &mut backend,
            &mut &stream(&requests(&b))[..],
            &mut output,
            &b,
            &mut FrameBudget::new(),
        )
        .unwrap();
        assert_eq!(backend.calls, [1, 2, 3, 4]);
        assert!(backend.closed);
        assert_eq!(backend.failed, 0);
        let mut input = &output[..];
        let mut budget = FrameBudget::new();
        let mut chain = Chain::new(b.decode.registration, b.sha256().unwrap());
        for generation in 1..=4 {
            let (reply, c, bytes) = wire::read_response(&mut input, &mut budget)
                .unwrap()
                .unwrap();
            assert_eq!(reply.schema, wire::RESPONSE_SCHEMA);
            assert!(!reply.full_model_acceptance);
            assert!(!reply.performance_claim);
            assert!(!reply.production_authority);
            let Event::Completed(done) = reply.event else {
                panic!("completion")
            };
            assert_eq!(done.generation, generation);
            assert_eq!(done.output_token, 100 + generation as u32);
            assert_eq!(done.chain, chain.advance(&done));
            assert_eq!(bytes.len(), OBSERVATION_BYTES);
            assert_eq!(c.unwrap(), control(generation));
        }
        let (reply, c, bytes) = wire::read_response(&mut input, &mut budget)
            .unwrap()
            .unwrap();
        assert_eq!(
            reply.event,
            Event::Closed {
                completed_forwards: 4,
                transcript_sha256: chain.digest()
            }
        );
        assert!(reply.native_closed && c.is_none() && bytes.is_empty() && input.is_empty());
    }
}
#[test]
fn guarded_cli_forward_close_and_output_failures_are_terminal_without_extra_dispatch() {
    let b = bootstrap(InputMode::TeacherForced);
    for failure in 0..5 {
        let mut backend = Fake::new(&b);
        backend.fail = Some(failure);
        assert!(
            serve(
                &mut backend,
                &mut &stream(&requests(&b))[..],
                &mut Vec::new(),
                &b,
                &mut FrameBudget::new()
            )
            .is_err()
        );
        assert_eq!(backend.calls.len(), (failure + 1).min(4));
        assert_eq!(backend.failed, 1);
        assert!(!backend.closed);
    }
    let mut backend = Fake::new(&b);
    backend.corrupt = true;
    assert!(
        serve(
            &mut backend,
            &mut &stream(&requests(&b))[..],
            &mut Vec::new(),
            &b,
            &mut FrameBudget::new()
        )
        .is_err()
    );
    assert_eq!(backend.calls, [1]);
    assert_eq!(backend.failed, 1);
    assert!(!backend.closed);
    struct Denied;
    impl Write for Denied {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("output closed"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut backend = Fake::new(&b);
    assert!(
        serve(
            &mut backend,
            &mut &stream(&requests(&b))[..],
            &mut Denied,
            &b,
            &mut FrameBudget::new()
        )
        .is_err()
    );
    assert_eq!(backend.calls, [1]);
    assert_eq!(backend.failed, 1);
    assert!(!backend.closed);
}
#[test]
fn guarded_cli_bad_request_and_eof_refuse_before_any_next_gpu_effect() {
    let b = bootstrap(InputMode::TeacherForced);
    for edit in [
        |r: &mut wire::Request| r.profile_sha256[0] ^= 1,
        |r: &mut wire::Request| r.registration[0] ^= 1,
        |r: &mut wire::Request| r.session[0] ^= 1,
        |r: &mut wire::Request| r.device_ids.swap(0, 1),
        |r: &mut wire::Request| r.id = 2,
        |r: &mut wire::Request| r.command = Command::Close,
        |r: &mut wire::Request| {
            if let Command::Forward { token, .. } = &mut r.command {
                *token += 1
            }
        },
        |r: &mut wire::Request| {
            if let Command::Forward { cache_metadata, .. } = &mut r.command {
                cache_metadata[2] = cache_metadata[1]
            }
        },
        |r: &mut wire::Request| {
            if let Command::Forward { rotary_bits, .. } = &mut r.command {
                rotary_bits[0] = f32::NAN.to_bits()
            }
        },
    ] {
        let mut values = requests(&b);
        edit(&mut values[0]);
        let mut backend = Fake::new(&b);
        assert!(
            serve(
                &mut backend,
                &mut &stream(&values)[..],
                &mut Vec::new(),
                &b,
                &mut FrameBudget::new()
            )
            .is_err()
        );
        assert!(backend.calls.is_empty());
        assert_eq!(backend.failed, 1);
    }
    for count in 0..5 {
        let mut backend = Fake::new(&b);
        assert!(
            serve(
                &mut backend,
                &mut &stream(&requests(&b)[..count])[..],
                &mut Vec::new(),
                &b,
                &mut FrameBudget::new()
            )
            .is_err()
        );
        assert_eq!(backend.calls.len(), count);
        assert_eq!(backend.failed, 1);
        assert!(!backend.closed);
    }
    let b = bootstrap(InputMode::Autoregressive);
    let mut values = requests(&b);
    if let Command::Forward { token, .. } = &mut values[1].command {
        *token += 1;
    }
    let mut backend = Fake::new(&b);
    assert!(
        serve(
            &mut backend,
            &mut &stream(&values)[..],
            &mut Vec::new(),
            &b,
            &mut FrameBudget::new()
        )
        .is_err()
    );
    assert_eq!(backend.calls, [1]);
    assert_eq!(backend.failed, 1);
}
#[test]
fn guarded_cli_fake_image_and_wrong_invocation_cannot_reach_group_open() {
    let options = parse_args(&args("teacher-forced")).unwrap();
    let b = bootstrap(InputMode::TeacherForced);
    let mut raw = Vec::new();
    crate::finite_long_wire_v1::write_header(&mut raw, &mut FrameBudget::new(), &b).unwrap();
    raw.extend_from_slice(&[8, 9, 10]);
    raw.extend_from_slice(&vec![0; b.guarded_image.bytes as usize]);
    assert!(prepare(&options, &mut &raw[..], &mut FrameBudget::new()).is_err());
    let mut wrong = b.clone();
    wrong.decode.scope.child_identity += 1;
    assert!(admitted_mode(&options, &wrong).is_err());
    let mut wrong = b.clone();
    wrong.decode.timeout_ms += 1;
    assert!(admitted_mode(&options, &wrong).is_err());
    assert!(prepare(&options, &mut &[][..], &mut FrameBudget::new()).is_err());
}
