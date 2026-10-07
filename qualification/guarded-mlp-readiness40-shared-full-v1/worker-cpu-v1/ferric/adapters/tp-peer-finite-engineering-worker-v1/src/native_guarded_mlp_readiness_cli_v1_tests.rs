use super::*;
use crate::finite_guarded_mlp_long_wire_v2::tests as fixture;

#[test]
fn shared_full_cli_is_explicit_and_never_selects_causal_or_full() {
    let good = [
        SHARED_FLAG,
        "--allow-unauthenticated-machine-code",
        "--devices",
        "7,9",
        "--timeout-ms",
        "10000",
        "--mode",
        "autoregressive",
    ]
    .map(OsString::from);
    parse_shared_args(&good).unwrap();
    for flag in [
        FLAG,
        POSITION5_FLAG,
        CAUSAL_FLAG,
        crate::native_guarded_mlp_full2303_cli_v1::FLAG,
        crate::native_guarded_mlp_decode_cli_v1::HOST_SHARED_FLAG,
        crate::native_guarded_mlp_decode_cli_v1::HOST_PAIRED_READ_FLAG,
    ] {
        let mut bad = good.clone();
        bad[0] = flag.into();
        assert!(parse_shared_args(&bad).is_err());
    }
    for parse in [parse_args, parse_position5_args, parse_causal_args] {
        assert!(parse(&good).is_err());
    }
    let mut bad = good.to_vec();
    bad.push("--paired-terminal".into());
    assert!(parse_shared_args(&bad).is_err());
    let mut bad = good.clone();
    bad[7] = "teacher-forced".into();
    assert!(parse_shared_args(&bad).is_err());
    assert!(admit_policy(long::Profile::Readiness40Position5, false, true).is_ok());
    assert!(admit_policy(long::Profile::Readiness40Position5, true, true).is_err());
    for profile in [long::Profile::Readiness40, long::Profile::Full2303] {
        assert!(admit_policy(profile, false, true).is_err());
    }
}
#[test]
fn shared_full_configures_fresh_group_before_install_with_only_shared_true() {
    use std::{cell::RefCell, rc::Rc};
    struct Fake {
        events: Rc<RefCell<Vec<&'static str>>>,
        fresh: bool,
    }
    impl FreshPolicy for Fake {
        fn configure(&mut self, cache: bool, operational: bool, shared: bool) -> io::Result<()> {
            assert_eq!((cache, operational, shared), (false, false, true));
            self.events.borrow_mut().push("configure");
            if !self.fresh {
                return Err(io::Error::other("not fresh"));
            }
            self.fresh = false;
            Ok(())
        }
    }
    for selected in [false, true] {
        let events = Rc::new(RefCell::new(Vec::new()));
        let (_, applied) = install_policy(
            Fake {
                events: events.clone(),
                fresh: true,
            },
            selected,
            |group| {
                group.events.borrow_mut().push("install");
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(applied.is_some(), selected);
        assert_eq!(
            *events.borrow(),
            if selected {
                vec!["configure", "install"]
            } else {
                vec!["install"]
            }
        );
    }
    let events = Rc::new(RefCell::new(Vec::new()));
    assert!(
        install_policy(
            Fake {
                events: events.clone(),
                fresh: false
            },
            true,
            |_| -> io::Result<()> { panic!("failed fresh policy must not install") }
        )
        .is_err()
    );
    assert_eq!(*events.borrow(), vec!["configure"]);
    assert!(
        install_policy(
            Fake {
                events,
                fresh: true
            },
            true,
            |_| -> io::Result<()> { Err(io::Error::other("partial setup failed")) }
        )
        .is_err()
    );
    let before = Instant::now();
    let deadline = before.checked_add(Duration::from_secs(1)).unwrap();
    shared_deadline(before, deadline).unwrap();
    assert!(shared_deadline(deadline, deadline).is_err());
    assert!(
        shared_deadline(
            deadline.checked_add(Duration::from_nanos(1)).unwrap(),
            deadline
        )
        .is_err()
    );
}
#[test]
fn shared_full_policy_uses_actual_closed_forty_frame_digest() {
    let b = wire::Bootstrap {
        schema: wire::POSITION5_SCHEMA.into(),
        child_deadline_ms: 60_000,
        sequence: fixture::bootstrap(long::Profile::Readiness40Position5),
    };
    let mut backend = Mock::new(&b);
    let mut output = Vec::new();
    let digest = serve(
        &mut backend,
        &mut input(&b, 40, true).as_slice(),
        &mut output,
        &b,
        &mut long::FrameBudget::new(),
    )
    .unwrap();
    assert!(backend.closed);
    assert_eq!(digest, backend.transcript.digest());
    let record = shared::PolicyRecord::new(&b, digest, [8; 32]).unwrap();
    let mut receiver = long::Transcript::new(b.sequence.clone()).unwrap();
    let mut raw = output.as_slice();
    let mut budget = long::FrameBudget::new();
    let mut captures = Vec::new();
    for _ in 0..40 {
        let (frame, capture) = long::read_frame(&mut raw, &mut budget, b.sequence.profile)
            .unwrap()
            .unwrap();
        receiver.begin(&frame.request).unwrap();
        receiver.advance(&frame).unwrap();
        if capture.is_some() {
            captures.push(frame.completion.position);
        }
    }
    let close = long::read_record::<wire::Closed>(&mut raw, &mut budget)
        .unwrap()
        .unwrap();
    receiver
        .close(&close.request, close.transcript_sha256)
        .unwrap();
    assert!(raw.is_empty());
    assert_eq!(captures, [0, 5, 16, 39]);
    record.validate(&b, receiver.digest(), [8; 32]).unwrap();
    assert!(record.validate(&b, [0; 32], [8; 32]).is_err());
}

struct Mock {
    transcript: long::Transcript,
    runs: u32,
    closed: bool,
    cancelled: usize,
    fail: Option<u32>,
    panic: bool,
}
impl Mock {
    fn new(b: &wire::Bootstrap) -> Self {
        Self {
            transcript: long::Transcript::new(b.sequence.clone()).unwrap(),
            runs: 0,
            closed: false,
            cancelled: 0,
            fail: None,
            panic: false,
        }
    }
}
impl Backend for Mock {
    fn run(&mut self, request: &long::Request) -> io::Result<Produced> {
        self.transcript.begin(request)?;
        assert!(!self.panic, "injected native unwind");
        if self.fail == Some(self.runs) {
            return Err(io::Error::other("injected late failure"));
        }
        let control = fixture::control(request.id);
        let observation = fixture::payload(7);
        let mut completion = long::Completion::from_observation(
            self.transcript.profile(),
            request,
            &control,
            &observation,
            7,
        )?;
        completion.chain = self.transcript.next_chain(request, &completion)?;
        let frame = long::Frame {
            schema: long::RESPONSE_SCHEMA.into(),
            profile: self.transcript.profile(),
            request: request.clone(),
            completion,
        };
        self.transcript.advance(&frame)?;
        self.runs += 1;
        Ok(Produced {
            frame,
            control,
            observation,
        })
    }
    fn digest(&self) -> [u8; 32] {
        self.transcript.digest()
    }
    fn close(&mut self, request: &long::Request, digest: [u8; 32]) -> io::Result<()> {
        self.transcript.close(request, digest)?;
        self.closed = true;
        Ok(())
    }
    fn cancel(&mut self) {
        self.cancelled += 1;
        self.transcript.cancel();
    }
}
fn bootstrap() -> wire::Bootstrap {
    wire::Bootstrap {
        schema: wire::SCHEMA.into(),
        child_deadline_ms: 60_000,
        sequence: fixture::bootstrap(long::Profile::Readiness40),
    }
}
fn input(b: &wire::Bootstrap, count: u32, close: bool) -> Vec<u8> {
    let mut raw = Vec::new();
    let mut budget = long::FrameBudget::new();
    for i in 0..count {
        long::write_record(&mut raw, &mut budget, &fixture::request(&b.sequence, i, 7)).unwrap();
    }
    if close {
        long::write_record(&mut raw, &mut budget, &fixture::close_request(&b.sequence)).unwrap();
    }
    raw
}
#[test]
fn readiness_cli_selector_is_explicit_and_preserves_all_existing_modes() {
    let good = [
        FLAG,
        "--allow-unauthenticated-machine-code",
        "--devices",
        "7,9",
        "--timeout-ms",
        "10000",
        "--mode",
        "autoregressive",
    ]
    .map(OsString::from);
    parse_args(&good).unwrap();
    for flag in [
        crate::native_guarded_mlp_decode_cli_v1::FLAG,
        crate::native_guarded_mlp_decode_cli_v1::REUSE_FLAG,
        crate::native_guarded_mlp_decode_cli_v1::HOST_PAIRED_READ_FLAG,
        "--full2303",
    ] {
        let mut bad = good.clone();
        bad[0] = flag.into();
        assert!(parse_args(&bad).is_err());
    }
    for index in 0..good.len() {
        assert!(parse_args(&good[..index]).is_err());
    }
    let mut bad = good.clone();
    bad[7] = "teacher-forced".into();
    assert!(parse_args(&bad).is_err());
    let mut bad = good.to_vec();
    bad.push("--full2303".into());
    assert!(parse_args(&bad).is_err());
}
#[test]
fn readiness_cli_streams_forty_actual_calls_four_captures_and_real_close() {
    let b = bootstrap();
    let mut backend = Mock::new(&b);
    let mut output = Vec::new();
    serve(
        &mut backend,
        &mut input(&b, 40, true).as_slice(),
        &mut output,
        &b,
        &mut long::FrameBudget::new(),
    )
    .unwrap();
    assert_eq!(
        (backend.runs, backend.closed, backend.cancelled),
        (40, true, 0)
    );
    assert!(backend.transcript.output_tokens().is_empty());
    let mut raw = output.as_slice();
    let mut budget = long::FrameBudget::new();
    let mut captures = Vec::new();
    for position in 0..40 {
        let (frame, capture) = long::read_frame(&mut raw, &mut budget, long::Profile::Readiness40)
            .unwrap()
            .unwrap();
        assert_eq!(frame.completion.position, position);
        assert_eq!(frame.completion.bank.local_generation, position / 2 + 1);
        if capture.is_some() {
            captures.push(position);
        }
    }
    assert_eq!(captures, [0, 15, 16, 39]);
    let closed = long::read_record::<wire::Closed>(&mut raw, &mut budget)
        .unwrap()
        .unwrap();
    closed
        .validate(&fixture::close_request(&b.sequence), backend.digest())
        .unwrap();
    assert!(raw.is_empty());
    assert!(output.len() < 5 << 20);
}
#[test]
fn readiness_cli_eof_late_failure_and_early_close_cancel_without_success() {
    let b = bootstrap();
    for (count, close, fail) in [
        (0, false, None),
        (1, true, None),
        (40, true, Some(37)),
        (40, false, None),
    ] {
        let mut backend = Mock::new(&b);
        backend.fail = fail;
        assert!(
            serve(
                &mut backend,
                &mut input(&b, count, close).as_slice(),
                &mut Vec::new(),
                &b,
                &mut long::FrameBudget::new()
            )
            .is_err()
        );
        assert_eq!(backend.cancelled, 1);
        assert!(!backend.closed);
    }
}
#[test]
fn readiness_cli_publication_failure_and_unwind_cancel_owner() {
    struct Reject;
    impl Write for Reject {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("rejected output"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let b = bootstrap();
    let mut backend = Mock::new(&b);
    assert!(
        serve(
            &mut backend,
            &mut input(&b, 40, true).as_slice(),
            &mut Reject,
            &b,
            &mut long::FrameBudget::new()
        )
        .is_err()
    );
    assert_eq!(
        (backend.runs, backend.cancelled, backend.closed),
        (1, 1, false)
    );
    struct RejectClose;
    impl Write for RejectClose {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if bytes
                .windows(wire::CLOSE_SCHEMA.len())
                .any(|w| w == wire::CLOSE_SCHEMA.as_bytes())
            {
                Err(io::Error::other("rejected Close publication"))
            } else {
                Ok(bytes.len())
            }
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut backend = Mock::new(&b);
    assert!(
        serve(
            &mut backend,
            &mut input(&b, 40, true).as_slice(),
            &mut RejectClose,
            &b,
            &mut long::FrameBudget::new()
        )
        .is_err()
    );
    assert_eq!(
        (backend.runs, backend.cancelled, backend.closed),
        (40, 1, true)
    );
    let mut backend = Mock::new(&b);
    backend.panic = true;
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            serve(
                &mut backend,
                &mut input(&b, 40, true).as_slice(),
                &mut Vec::new(),
                &b,
                &mut long::FrameBudget::new(),
            )
        }))
        .is_err()
    );
    assert_eq!(backend.cancelled, 1);
}

#[test]
fn position5_cli_is_explicit_and_streams_exactly_four_selected_captures() {
    let args = [
        POSITION5_FLAG,
        "--allow-unauthenticated-machine-code",
        "--devices",
        "7,9",
        "--timeout-ms",
        "10000",
        "--mode",
        "autoregressive",
    ]
    .map(OsString::from);
    parse_position5_args(&args).unwrap();
    assert!(parse_args(&args).is_err());
    let mut old = args.clone();
    old[0] = FLAG.into();
    assert!(parse_position5_args(&old).is_err());
    let mut bad = args.clone();
    bad[7] = "teacher-forced".into();
    assert!(parse_position5_args(&bad).is_err());
    let mut b = bootstrap();
    b.schema = wire::POSITION5_SCHEMA.into();
    b.sequence.profile = long::Profile::Readiness40Position5;
    let mut backend = Mock::new(&b);
    let mut output = Vec::new();
    serve(
        &mut backend,
        &mut input(&b, 40, true).as_slice(),
        &mut output,
        &b,
        &mut long::FrameBudget::new(),
    )
    .unwrap();
    assert_eq!(
        (backend.runs, backend.closed, backend.cancelled),
        (40, true, 0)
    );
    let mut raw = output.as_slice();
    let mut budget = long::FrameBudget::new();
    let mut captures = Vec::new();
    for position in 0..40 {
        let (frame, capture) = long::read_frame(&mut raw, &mut budget, b.sequence.profile)
            .unwrap()
            .unwrap();
        assert_eq!(frame.completion.position, position);
        if capture.is_some() {
            captures.push(position);
        }
    }
    assert_eq!(captures, [0, 5, 16, 39]);
    let close = long::read_record::<wire::Closed>(&mut raw, &mut budget)
        .unwrap()
        .unwrap();
    close
        .validate_for(
            b.sequence.profile,
            &fixture::close_request(&b.sequence),
            backend.digest(),
        )
        .unwrap();
    assert!(raw.is_empty());
    assert!(output.len() < 5 << 20);
}
