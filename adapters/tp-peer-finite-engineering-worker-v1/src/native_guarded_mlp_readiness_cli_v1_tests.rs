use super::*;
use crate::finite_guarded_mlp_long_wire_v2::tests as fixture;

#[test]
fn scoped_warm_cli_is_explicit_and_refuses_other_profiles_and_policy_combinations() {
    let good = [
        SCOPED_FLAG,
        "--allow-unauthenticated-machine-code",
        "--devices",
        "7,9",
        "--timeout-ms",
        "10000",
        "--mode",
        "autoregressive",
    ]
    .map(OsString::from);
    parse_scoped_args(&good).unwrap();
    for flag in [
        FLAG,
        POSITION5_FLAG,
        CAUSAL_FLAG,
        SHARED_FLAG,
        crate::native_guarded_mlp_full2303_cli_v1::FLAG,
        crate::native_guarded_mlp_decode_cli_v1::HOST_SHARED_FLAG,
        crate::native_guarded_mlp_decode_cli_v1::HOST_PAIRED_READ_FLAG,
    ] {
        let mut bad = good.clone();
        bad[0] = flag.into();
        assert!(parse_scoped_args(&bad).is_err());
    }
    for parse in [
        parse_args,
        parse_position5_args,
        parse_causal_args,
        parse_shared_args,
    ] {
        assert!(parse(&good).is_err());
    }
    let mut extra = good.to_vec();
    extra.push("--paired-terminal".into());
    assert!(parse_scoped_args(&extra).is_err());
    let mut bad = good.clone();
    bad[7] = "teacher-forced".into();
    assert!(parse_scoped_args(&bad).is_err());
    admit_scoped(long::Profile::Readiness40Position5, false, false, true).unwrap();
    for (profile, causal, shared) in [
        (long::Profile::Readiness40, false, false),
        (long::Profile::Full2303, false, false),
        (long::Profile::Readiness40Position5, true, false),
        (long::Profile::Readiness40Position5, false, true),
    ] {
        assert!(admit_scoped(profile, causal, shared, true).is_err());
        admit_scoped(profile, causal, shared, false).unwrap();
    }
}
#[test]
fn scoped_warm_publication_deadline_requires_both_pre_and_post_write_to_be_inside() {
    let before = Instant::now();
    let deadline = before + Duration::from_secs(1);
    scoped_deadline(before, deadline).unwrap();
    assert!(scoped_deadline(deadline, deadline).is_err());
    assert!(scoped_deadline(deadline + Duration::from_nanos(1), deadline).is_err());
    let scoped: unsafe fn(
        NativeOptions,
        &mut io::Cursor<Vec<u8>>,
        &mut Vec<u8>,
        &mut Vec<u8>,
    ) -> io::Result<()> = run_native_scoped;
    let ordinary: unsafe fn(
        NativeOptions,
        &mut io::Cursor<Vec<u8>>,
        &mut Vec<u8>,
    ) -> io::Result<()> = run_native_position5;
    let _ = (scoped, ordinary);
}

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

#[test]
fn bank_scoped_cli_has_one_explicit_flag_and_preserves_existing_route_signatures() {
    let args = [
        BANK_SCOPED_FLAG,
        "--allow-unauthenticated-machine-code",
        "--devices",
        "7,9",
        "--timeout-ms",
        "10000",
        "--mode",
        "autoregressive",
    ]
    .map(OsString::from);
    parse_bank_scoped_args(&args).unwrap();
    for parse in [
        parse_args,
        parse_position5_args,
        parse_causal_args,
        parse_shared_args,
        parse_scoped_args,
    ] {
        assert!(parse(&args).is_err());
    }
    for flag in [
        FLAG,
        POSITION5_FLAG,
        CAUSAL_FLAG,
        SHARED_FLAG,
        SCOPED_FLAG,
        crate::native_guarded_mlp_full2303_cli_v1::FLAG,
    ] {
        let mut bad = args.clone();
        bad[0] = flag.into();
        assert!(parse_bank_scoped_args(&bad).is_err());
    }
    for n in 0..args.len() {
        assert!(parse_bank_scoped_args(&args[..n]).is_err());
    }
    let mut bad = args.clone();
    bad[7] = "teacher-forced".into();
    assert!(parse_bank_scoped_args(&bad).is_err());
    let _bank: unsafe fn(
        NativeOptions,
        &mut io::Cursor<Vec<u8>>,
        &mut Vec<u8>,
        &mut Vec<u8>,
    ) -> io::Result<()> = run_native_bank_scoped;
    let _old: unsafe fn(
        NativeOptions,
        &mut io::Cursor<Vec<u8>>,
        &mut Vec<u8>,
        &mut Vec<u8>,
    ) -> io::Result<()> = run_native_scoped;
}
#[test]
#[allow(unsafe_code)]
fn bank_scoped_cli_refuses_policy_combinations_before_input_and_keeps_deadline() {
    admit_bank_scoped(
        long::Profile::Readiness40Position5,
        false,
        false,
        false,
        true,
    )
    .unwrap();
    for (profile, causal, shared, scoped) in [
        (long::Profile::Readiness40, false, false, false),
        (long::Profile::Full2303, false, false, false),
        (long::Profile::Readiness40Position5, true, false, false),
        (long::Profile::Readiness40Position5, false, true, false),
        (long::Profile::Readiness40Position5, false, false, true),
    ] {
        assert!(admit_bank_scoped(profile, causal, shared, scoped, true).is_err());
        admit_bank_scoped(profile, causal, shared, scoped, false).unwrap();
    }
    let options = parse_bank_scoped_args(
        &[
            BANK_SCOPED_FLAG,
            "--allow-unauthenticated-machine-code",
            "--devices",
            "7,9",
            "--timeout-ms",
            "10000",
            "--mode",
            "autoregressive",
        ]
        .map(OsString::from),
    )
    .unwrap();
    // SAFETY: the owned empty Cursor returns EOF before bootstrap admission,
    // setup preparation, native group opening, or any machine-code execution.
    let e = unsafe {
        run_native_bank_scoped(
            options,
            &mut io::Cursor::new(Vec::<u8>::new()),
            &mut Vec::new(),
            &mut Vec::new(),
        )
    }
    .unwrap_err();
    assert!(e.to_string().contains("readiness bootstrap absent"));
    let now = Instant::now();
    assert!(scoped_deadline(now, now).is_err());
    scoped_deadline(now, now + Duration::from_millis(1)).unwrap();
}

#[test]
fn census_scoped_cli_has_one_explicit_flag_and_preserves_existing_route_signatures() {
    let args = [
        CENSUS_SCOPED_FLAG,
        "--allow-unauthenticated-machine-code",
        "--devices",
        "7,9",
        "--timeout-ms",
        "10000",
        "--mode",
        "autoregressive",
    ]
    .map(OsString::from);
    parse_census_scoped_args(&args).unwrap();
    for parse in [
        parse_args,
        parse_position5_args,
        parse_causal_args,
        parse_shared_args,
        parse_scoped_args,
        parse_bank_scoped_args,
    ] {
        assert!(parse(&args).is_err());
    }
    for flag in [
        FLAG,
        POSITION5_FLAG,
        CAUSAL_FLAG,
        SHARED_FLAG,
        SCOPED_FLAG,
        BANK_SCOPED_FLAG,
        crate::native_guarded_mlp_full2303_cli_v1::FLAG,
        crate::native_guarded_mlp_full2303_cli_v1::SCOPED_FLAG,
    ] {
        let mut bad = args.clone();
        bad[0] = flag.into();
        assert!(parse_census_scoped_args(&bad).is_err());
    }
    for n in 0..args.len() {
        assert!(parse_census_scoped_args(&args[..n]).is_err());
    }
    let mut bad = args.clone();
    bad[7] = "teacher-forced".into();
    assert!(parse_census_scoped_args(&bad).is_err());
    let _census: unsafe fn(
        NativeOptions,
        &mut io::Cursor<Vec<u8>>,
        &mut Vec<u8>,
        &mut Vec<u8>,
    ) -> io::Result<()> = run_native_census_scoped;
    let _bank: unsafe fn(
        NativeOptions,
        &mut io::Cursor<Vec<u8>>,
        &mut Vec<u8>,
        &mut Vec<u8>,
    ) -> io::Result<()> = run_native_bank_scoped;
    let _old: unsafe fn(
        NativeOptions,
        &mut io::Cursor<Vec<u8>>,
        &mut Vec<u8>,
        &mut Vec<u8>,
    ) -> io::Result<()> = run_native_scoped;
}
#[test]
#[allow(unsafe_code)]
fn census_scoped_cli_refuses_policy_combinations_before_input_and_keeps_deadline() {
    admit_census_scoped(
        long::Profile::Readiness40Position5,
        false,
        false,
        false,
        false,
        true,
    )
    .unwrap();
    for (profile, causal, shared, scoped) in [
        (long::Profile::Readiness40, false, false, false),
        (long::Profile::Full2303, false, false, false),
        (long::Profile::Readiness40Position5, true, false, false),
        (long::Profile::Readiness40Position5, false, true, false),
        (long::Profile::Readiness40Position5, false, false, true),
    ] {
        assert!(admit_census_scoped(profile, causal, shared, scoped, false, true).is_err());
        admit_census_scoped(profile, causal, shared, scoped, false, false).unwrap();
    }
    assert!(
        admit_census_scoped(
            long::Profile::Readiness40Position5,
            false,
            false,
            false,
            true,
            true
        )
        .is_err()
    );
    let options = parse_census_scoped_args(
        &[
            CENSUS_SCOPED_FLAG,
            "--allow-unauthenticated-machine-code",
            "--devices",
            "7,9",
            "--timeout-ms",
            "10000",
            "--mode",
            "autoregressive",
        ]
        .map(OsString::from),
    )
    .unwrap();
    // SAFETY: the owned empty Cursor returns EOF before bootstrap admission,
    // setup preparation, native group opening, or any machine-code execution.
    let e = unsafe {
        run_native_census_scoped(
            options,
            &mut io::Cursor::new(Vec::<u8>::new()),
            &mut Vec::new(),
            &mut Vec::new(),
        )
    }
    .unwrap_err();
    assert!(e.to_string().contains("readiness bootstrap absent"));
    let now = Instant::now();
    assert!(scoped_deadline(now, now).is_err());
    scoped_deadline(now, now + Duration::from_millis(1)).unwrap();
}

#[test]
fn tail_scoped_cli_has_one_explicit_flag_and_preserves_existing_route_signatures() {
    let args = [
        TAIL_SCOPED_FLAG,
        "--allow-unauthenticated-machine-code",
        "--devices",
        "7,9",
        "--timeout-ms",
        "10000",
        "--mode",
        "autoregressive",
    ]
    .map(OsString::from);
    parse_tail_scoped_args(&args).unwrap();
    for parse in [
        parse_args,
        parse_position5_args,
        parse_causal_args,
        parse_shared_args,
        parse_scoped_args,
        parse_bank_scoped_args,
        parse_census_scoped_args,
    ] {
        assert!(parse(&args).is_err());
    }
    for flag in [
        FLAG,
        POSITION5_FLAG,
        CAUSAL_FLAG,
        SHARED_FLAG,
        SCOPED_FLAG,
        BANK_SCOPED_FLAG,
        CENSUS_SCOPED_FLAG,
        crate::native_guarded_mlp_full2303_cli_v1::FLAG,
        crate::native_guarded_mlp_full2303_cli_v1::SCOPED_FLAG,
    ] {
        let mut bad = args.clone();
        bad[0] = flag.into();
        assert!(parse_tail_scoped_args(&bad).is_err());
    }
    for n in 0..args.len() {
        assert!(parse_tail_scoped_args(&args[..n]).is_err());
    }
    let mut bad = args.clone();
    bad[7] = "teacher-forced".into();
    assert!(parse_tail_scoped_args(&bad).is_err());
    let _census: unsafe fn(
        NativeOptions,
        &mut io::Cursor<Vec<u8>>,
        &mut Vec<u8>,
        &mut Vec<u8>,
    ) -> io::Result<()> = run_native_tail_scoped;
    let _bank: unsafe fn(
        NativeOptions,
        &mut io::Cursor<Vec<u8>>,
        &mut Vec<u8>,
        &mut Vec<u8>,
    ) -> io::Result<()> = run_native_bank_scoped;
    let _old: unsafe fn(
        NativeOptions,
        &mut io::Cursor<Vec<u8>>,
        &mut Vec<u8>,
        &mut Vec<u8>,
    ) -> io::Result<()> = run_native_scoped;
}
#[test]
#[allow(unsafe_code)]
fn tail_scoped_cli_refuses_all_other_policies_before_io_and_keeps_deadline() {
    admit_tail_scoped(long::Profile::Readiness40Position5, [false; 5], true).unwrap();
    for p in [long::Profile::Readiness40, long::Profile::Full2303] {
        assert!(admit_tail_scoped(p, [false; 5], true).is_err());
        admit_tail_scoped(p, [false; 5], false).unwrap();
    }
    for i in 0..5 {
        let mut other = [false; 5];
        other[i] = true;
        assert!(admit_tail_scoped(long::Profile::Readiness40Position5, other, true).is_err());
        admit_tail_scoped(long::Profile::Readiness40Position5, other, false).unwrap();
    }
    let args = [
        TAIL_SCOPED_FLAG,
        "--allow-unauthenticated-machine-code",
        "--devices",
        "7,9",
        "--timeout-ms",
        "10000",
        "--mode",
        "autoregressive",
    ]
    .map(OsString::from);
    for flag in [
        "--full2303",
        "--raw-device-timestamps",
        CENSUS_SCOPED_FLAG,
        TAIL_SCOPED_FLAG,
    ] {
        let mut bad = args.to_vec();
        bad.push(flag.into());
        assert!(parse_tail_scoped_args(&bad).is_err());
    }
    let options = parse_tail_scoped_args(&args).unwrap();
    // SAFETY: owned empty Cursor returns EOF before bootstrap, Group or GPU work.
    let error = unsafe {
        run_native_tail_scoped(
            options,
            &mut io::Cursor::new(Vec::<u8>::new()),
            &mut Vec::new(),
            &mut Vec::new(),
        )
    }
    .unwrap_err();
    assert!(error.to_string().contains("readiness bootstrap absent"));
    let now = Instant::now();
    assert!(scoped_deadline(now, now).is_err());
    scoped_deadline(now, now + Duration::from_millis(1)).unwrap();
}

#[cfg(feature = "engineering-currentness-duration-diagnostics")]
#[test]
fn forward_phase_cli_three_records_preserve_original_first_two_bytes() {
    use crate::finite_guarded_mlp_readiness_forward_durations_v1 as phases;
    let (b, policy, old, record) = phases::tests::fixture();
    let original = [policy.encode().unwrap(), old.encode().unwrap()].concat();
    let mut output = Vec::new();
    write_forward_diagnostics(
        &mut output,
        &policy,
        old.forwards.clone(),
        record.forwards.clone(),
        Instant::now() + Duration::from_secs(1),
    )
    .unwrap();
    assert!(output.starts_with(&original));
    assert_eq!(output.iter().filter(|byte| **byte == b'\n').count(), 3);
    assert!(output.len() <= phases::STDERR_MAX_BYTES);
    let decoded =
        phases::decode_stderr(&output, &b, policy.transcript_sha256, policy.worker_sha256).unwrap();
    assert_eq!(decoded, (policy, old, record));
}

#[cfg(feature = "engineering-currentness-duration-diagnostics")]
#[test]
fn forward_phase_cli_invalid_late_record_writes_no_policy_prefix() {
    use crate::finite_guarded_mlp_readiness_forward_durations_v1 as phases;
    for case in 0..5 {
        let (_, policy, mut old, mut record) = phases::tests::fixture();
        match case {
            0 => {
                record.forwards.pop();
            }
            1 => {
                record.forwards.swap(38, 39);
            }
            2 => {
                record.forwards[39].forward_body_ns += 1;
            }
            3 => {
                let row = &mut record.forwards[39];
                row.phase_ns[4] = 0;
                row.forward_body_ns = row.phase_ns.iter().sum();
            }
            _ => {
                old.forwards.pop();
            }
        }
        let mut output = Vec::new();
        assert!(
            write_forward_diagnostics(
                &mut output,
                &policy,
                old.forwards,
                record.forwards,
                Instant::now() + Duration::from_secs(1),
            )
            .is_err()
        );
        assert!(output.is_empty());
    }
}

#[cfg(feature = "engineering-currentness-duration-diagnostics")]
#[test]
fn forward_phase_cli_writer_deadline_and_each_write_flush_refusal_propagate() {
    use crate::finite_guarded_mlp_readiness_forward_durations_v1 as phases;
    struct Output {
        fail_write: Option<usize>,
        fail_flush: bool,
        writes: usize,
        flushes: usize,
        bytes: Vec<u8>,
    }
    impl Write for Output {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            let ordinal = self.writes;
            self.writes += 1;
            if self.fail_write == Some(ordinal) {
                return Err(io::Error::other("injected diagnostic write failure"));
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            self.flushes += 1;
            if self.fail_flush {
                Err(io::Error::other("injected diagnostic flush failure"))
            } else {
                Ok(())
            }
        }
    }
    for case in 0..5 {
        let (_, policy, old, record) = phases::tests::fixture();
        let encoded = [
            policy.encode().unwrap(),
            old.encode().unwrap(),
            record.encode().unwrap(),
        ];
        let mut output = Output {
            fail_write: (case < 3).then_some(case),
            fail_flush: case == 3,
            writes: 0,
            flushes: 0,
            bytes: Vec::new(),
        };
        let deadline = if case == 4 {
            Instant::now()
        } else {
            Instant::now() + Duration::from_secs(1)
        };
        assert!(
            write_forward_diagnostics(
                &mut output,
                &policy,
                old.forwards,
                record.forwards,
                deadline,
            )
            .is_err()
        );
        if case < 3 {
            assert_eq!(output.writes, case + 1);
            assert_eq!(output.flushes, 0);
            assert_eq!(output.bytes, encoded[..case].concat());
        } else if case == 3 {
            assert_eq!((output.writes, output.flushes), (3, 1));
            assert_eq!(output.bytes, encoded.concat());
        } else {
            assert_eq!((output.writes, output.flushes), (0, 0));
            assert!(output.bytes.is_empty());
        }
    }
}
