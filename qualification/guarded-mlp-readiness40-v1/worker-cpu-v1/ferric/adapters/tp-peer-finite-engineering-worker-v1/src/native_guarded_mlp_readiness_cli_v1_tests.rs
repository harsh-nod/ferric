use super::*;
use crate::finite_guarded_mlp_long_wire_v2::tests as fixture;

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
            long::Profile::Readiness40,
            request,
            &control,
            &observation,
            7,
        )?;
        completion.chain = self.transcript.next_chain(request, &completion)?;
        let frame = long::Frame {
            schema: long::RESPONSE_SCHEMA.into(),
            profile: long::Profile::Readiness40,
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
