use super::*;
use crate::finite_guarded_mlp_long_wire_v2::tests as fixture;
struct Mock {
    transcript: long::Transcript,
    runs: u32,
    closed: bool,
    cancelled: usize,
    fail: Option<u32>,
    panic: bool,
    altered: Option<Vec<u32>>,
    scoped: Option<crate::native_catalog::forward::guarded_mlp_decode_v1::full2303::scoped::State>,
    bank_census:
        Option<crate::native_catalog::forward::guarded_mlp_decode_v1::full2303::bank_census::State>,
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
            altered: None,
            scoped: None,
            bank_census: None,
        }
    }
}
impl Backend for Mock {
    fn run(&mut self, request: &long::Request) -> io::Result<Produced> {
        self.transcript.begin(request)?;
        assert!(!self.panic, "injected native unwind");
        if self.fail == Some(self.runs) {
            return Err(io::Error::other("injected failure"));
        }
        if let Some(state) = self.scoped.as_mut() {
            for layer in 0..36 {
                state.dispatch(self.runs, layer, |warm| {
                    Ok((
                        (),
                        warm.then_some(fe2o3_kfd::Gfx950EngineeringPeerScopedCurrentnessCountsV1 {
                            full_discoveries: 2,
                            local_checkpoints: 5,
                            before_calls: 11,
                            after_calls: 11,
                            generation_probes: 13,
                        }),
                    ))
                })?;
            }
        }
        if let Some(state) = self.bank_census.as_mut() {
            state.begin(self.runs, || {
                Ok(crate::state_roster::guarded_mlp_decode_v1::BankCompletion {
                    generation: u64::from(self.runs) / 2 + 1,
                    currentness: (self.runs >= 2).then_some(
                        fe2o3_kfd::Gfx950EngineeringPeerScopedCurrentnessCountsV1 {
                            full_discoveries: 2,
                            local_checkpoints: 361,
                            before_calls: 726,
                            after_calls: 726,
                            generation_probes: 725,
                        },
                    ),
                })
            })?;
            for layer in 0..36 {
                state.dispatch(self.runs, layer, |warm| {
                    Ok((
                        (),
                        warm.then_some((
                            fe2o3_kfd::Gfx950EngineeringPeerScopedCurrentnessCountsV1 {
                                full_discoveries: 2,
                                local_checkpoints: 21,
                                before_calls: 27,
                                after_calls: 27,
                                generation_probes: 45,
                            },
                            fe2o3_kfd::Gfx950EngineeringPeerScopedCapacityCensusObservationV1 {
                                owner_counts: [787, 783],
                                preflights: 2,
                                rank_checkpoints: 16,
                            },
                        )),
                    ))
                })?;
            }
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
    fn output_tokens(&self) -> &[u32] {
        self.altered
            .as_deref()
            .unwrap_or_else(|| self.transcript.output_tokens())
    }
    fn close(&mut self, request: &long::Request, digest: [u8; 32]) -> io::Result<()> {
        if let Some(state) = self.scoped.as_ref() {
            state.closed_counts()?;
        }
        if let Some(state) = self.bank_census.as_ref() {
            state.closed_counts()?;
        }
        self.transcript.close(request, digest)?;
        self.closed = true;
        Ok(())
    }
    fn cancel(&mut self) {
        self.cancelled += 1;
        self.transcript.cancel();
    }
}

#[test]
fn full_scoped_cli_closed_route_joins_all_own_outputs_and_compact_policy() {
    let b = bootstrap();
    let mut backend = Mock::new(&b);
    backend.scoped = Some(
        crate::native_catalog::forward::guarded_mlp_decode_v1::full2303::scoped::State::new(
            long::Profile::Full2303,
        )
        .unwrap(),
    );
    let closed = serve_closed(
        &mut backend,
        &mut input(&b, 2303, true).as_slice(),
        &mut Vec::new(),
        &b,
        &mut long::FrameBudget::new(),
    )
    .unwrap();
    assert_eq!(closed.generated_tokens, vec![7; 256]);
    assert!(backend.closed);
    let counts = backend.scoped.as_ref().unwrap().closed_counts().unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut policy = Vec::new();
    publish_policy(
        &mut policy,
        &b,
        &closed,
        counts.clone(),
        [8; 32],
        deadline,
        || Ok([8; 32]),
    )
    .unwrap();
    scoped::PolicyRecord::decode(
        &policy,
        &b,
        closed.transcript_sha256,
        &closed.generated_tokens,
        [8; 32],
    )
    .unwrap();
    let mut wrong = closed.clone();
    wrong.native_closed = false;
    assert!(
        publish_policy(
            &mut Vec::new(),
            &b,
            &wrong,
            counts.clone(),
            [8; 32],
            deadline,
            || Ok([8; 32])
        )
        .is_err()
    );
    let mut output = Vec::new();
    assert!(
        publish_policy(
            &mut output,
            &b,
            &closed,
            counts.clone(),
            [8; 32],
            deadline,
            || Ok([9; 32])
        )
        .is_err()
    );
    assert!(output.is_empty());
    struct Refuse(bool);
    impl Write for Refuse {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.0 {
                Ok(bytes.len())
            } else {
                Err(io::Error::other("policy write"))
            }
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("policy flush"))
        }
    }
    for flush in [false, true] {
        assert!(
            publish_policy(
                &mut Refuse(flush),
                &b,
                &closed,
                counts.clone(),
                [8; 32],
                deadline,
                || Ok([8; 32])
            )
            .is_err()
        );
    }
    assert!(
        publish_policy(
            &mut output,
            &b,
            &closed,
            counts,
            [8; 32],
            Instant::now(),
            || panic!("expired")
        )
        .is_err()
    );
    assert!(output.is_empty());
}

#[test]
fn full_scoped_cli_selector_deadline_and_concrete_entry_signatures_are_distinct() {
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
    assert!(parse_args(&good).is_err());
    for flag in [
        FLAG,
        crate::native_guarded_mlp_readiness_cli_v1::SCOPED_FLAG,
        crate::native_guarded_mlp_readiness_cli_v1::POSITION5_FLAG,
        crate::native_guarded_mlp_decode_cli_v1::REUSE_FLAG,
    ] {
        let mut bad = good.clone();
        bad[0] = flag.into();
        assert!(parse_scoped_args(&bad).is_err());
    }
    for n in 0..good.len() {
        assert!(parse_scoped_args(&good[..n]).is_err());
    }
    let mut bad = good.to_vec();
    bad.push("--readiness40".into());
    assert!(parse_scoped_args(&bad).is_err());
    let now = Instant::now();
    assert!(publication_deadline(now, now + Duration::from_millis(1)).is_ok());
    assert!(publication_deadline(now, now).is_err());
    assert!(publication_deadline(now + Duration::from_millis(1), now).is_err());
    let _: unsafe fn(NativeOptions, &mut io::Cursor<Vec<u8>>, &mut Vec<u8>) -> io::Result<()> =
        run_native;
    let _: unsafe fn(
        NativeOptions,
        &mut io::Cursor<Vec<u8>>,
        &mut Vec<u8>,
        &mut Vec<u8>,
    ) -> io::Result<()> = run_native_scoped;
}
fn bootstrap() -> wire::Bootstrap {
    wire::Bootstrap {
        schema: wire::SCHEMA.into(),
        child_deadline_ms: 60_000,
        sequence: fixture::bootstrap(long::Profile::Full2303),
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
fn full2303_cli_selector_refuses_other_modes_missing_opt_in_and_extra_arguments() {
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
        crate::native_guarded_mlp_readiness_cli_v1::FLAG,
        crate::native_guarded_mlp_readiness_cli_v1::POSITION5_FLAG,
        crate::native_guarded_mlp_decode_cli_v1::REUSE_FLAG,
        crate::native_guarded_mlp_decode_cli_v1::PAIRED_TERMINAL_FLAG,
    ] {
        let mut bad = good.clone();
        bad[0] = flag.into();
        assert!(parse_args(&bad).is_err());
    }
    for n in 0..good.len() {
        assert!(parse_args(&good[..n]).is_err());
    }
    let mut bad = good.clone();
    bad[7] = "teacher-forced".into();
    assert!(parse_args(&bad).is_err());
    let mut bad = good.to_vec();
    bad.remove(1);
    assert!(parse_args(&bad).is_err());
    let mut bad = good.to_vec();
    bad.push("--readiness40".into());
    assert!(parse_args(&bad).is_err());
}
#[test]
fn full2303_cli_streams_2303_owned_calls_256_outputs_four_captures_and_real_close() {
    let b = bootstrap();
    let mut backend = Mock::new(&b);
    let mut output = Vec::new();
    serve(
        &mut backend,
        &mut input(&b, 2303, true).as_slice(),
        &mut output,
        &b,
        &mut long::FrameBudget::new(),
    )
    .unwrap();
    assert_eq!(
        (backend.runs, backend.closed, backend.cancelled),
        (2303, true, 0)
    );
    assert_eq!(backend.output_tokens(), vec![7; 256]);
    let mut transcript = long::Transcript::new(b.sequence.clone()).unwrap();
    let mut raw = output.as_slice();
    let mut budget = long::FrameBudget::new();
    let mut captures = Vec::new();
    for position in 0..2303 {
        let (frame, capture) = long::read_frame(&mut raw, &mut budget, long::Profile::Full2303)
            .unwrap()
            .unwrap();
        transcript.begin(&frame.request).unwrap();
        transcript.advance(&frame).unwrap();
        assert_eq!(frame.completion.position, position);
        assert_eq!(frame.completion.bank.local_generation, position / 2 + 1);
        if position >= 2048 {
            let long::Command::Forward { token, .. } = frame.request.command else {
                panic!("forward");
            };
            assert_eq!(token, 7);
        }
        if capture.is_some() {
            captures.push(position);
        }
    }
    assert_eq!(captures, [0, 2047, 2048, 2302]);
    let closed = long::read_record::<wire::Closed>(&mut raw, &mut budget)
        .unwrap()
        .unwrap();
    let request = fixture::close_request(&b.sequence);
    transcript.close(&request, transcript.digest()).unwrap();
    closed
        .validate(&request, transcript.digest(), transcript.output_tokens())
        .unwrap();
    join_completed(&backend, &transcript).unwrap();
    for index in [0, 255] {
        let mut wrong = backend.transcript.output_tokens().to_vec();
        wrong[index] ^= 1;
        backend.altered = Some(wrong);
        assert!(join_completed(&backend, &transcript).is_err());
    }
    assert!(raw.is_empty());
    assert!(output.len() <= long::MAX_RETAINED_BYTES);
    assert!(long::MAX_RETAINED_BYTES < long::EVIDENCE_BYTES);
}
#[test]
fn full2303_cli_eof_wrong_prompt_early_close_and_failure_cancel_without_success() {
    let b = bootstrap();
    for (count, close, fail) in [
        (0, false, None),
        (1, true, None),
        (3, false, Some(2)),
        (2, false, None),
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
    let mut request = fixture::request(&b.sequence, 0, 7);
    let long::Command::Forward { token, .. } = &mut request.command else {
        panic!("forward");
    };
    *token ^= 1;
    let mut raw = Vec::new();
    long::write_record(&mut raw, &mut long::FrameBudget::new(), &request).unwrap();
    let mut backend = Mock::new(&b);
    assert!(
        serve(
            &mut backend,
            &mut raw.as_slice(),
            &mut Vec::new(),
            &b,
            &mut long::FrameBudget::new()
        )
        .is_err()
    );
    assert_eq!((backend.runs, backend.cancelled), (0, 1));
}
#[test]
fn full2303_cli_publication_failure_and_unwind_quarantine_owner() {
    struct Reject;
    impl Write for Reject {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("write failure"))
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
            &mut input(&b, 1, false).as_slice(),
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
    let mut backend = Mock::new(&b);
    backend.panic = true;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        serve(
            &mut backend,
            &mut input(&b, 1, false).as_slice(),
            &mut Vec::new(),
            &b,
            &mut long::FrameBudget::new(),
        )
    }));
    assert!(result.is_err());
    assert_eq!(backend.cancelled, 1);
    assert!(!backend.closed);
}

#[test]
fn full_bank_census_cli_closed_route_joins_all_own_outputs_and_compact_policy() {
    let b = bootstrap();
    let mut backend = Mock::new(&b);
    backend.bank_census = Some(
        crate::native_catalog::forward::guarded_mlp_decode_v1::full2303::bank_census::State::new(
            long::Profile::Full2303,
        )
        .unwrap(),
    );
    let closed = serve_closed(
        &mut backend,
        &mut input(&b, 2303, true).as_slice(),
        &mut Vec::new(),
        &b,
        &mut long::FrameBudget::new(),
    )
    .unwrap();
    assert_eq!(closed.generated_tokens, vec![7; 256]);
    assert!(backend.closed && backend.runs == 2303);
    assert_eq!(closed.completed_forwards, 2303);
    let counts = backend
        .bank_census
        .as_ref()
        .unwrap()
        .closed_counts()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut policy = Vec::new();
    publish_bank_census_policy(
        &mut policy,
        &b,
        &closed,
        counts.clone(),
        [8; 32],
        deadline,
        || Ok([8; 32]),
    )
    .unwrap();
    bank_census::PolicyRecord::decode(
        &policy,
        &b,
        closed.transcript_sha256,
        &closed.generated_tokens,
        [8; 32],
    )
    .unwrap();
    let mut wrong = closed.clone();
    wrong.native_closed = false;
    assert!(
        publish_bank_census_policy(
            &mut Vec::new(),
            &b,
            &wrong,
            counts.clone(),
            [8; 32],
            deadline,
            || Ok([8; 32])
        )
        .is_err()
    );
    let mut output = Vec::new();
    assert!(
        publish_bank_census_policy(
            &mut output,
            &b,
            &closed,
            counts.clone(),
            [8; 32],
            deadline,
            || Ok([9; 32])
        )
        .is_err()
    );
    assert!(output.is_empty());
    struct Refuse(bool);
    impl Write for Refuse {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.0 {
                Ok(bytes.len())
            } else {
                Err(io::Error::other("policy write"))
            }
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("policy flush"))
        }
    }
    for flush in [false, true] {
        assert!(
            publish_bank_census_policy(
                &mut Refuse(flush),
                &b,
                &closed,
                counts.clone(),
                [8; 32],
                deadline,
                || Ok([8; 32])
            )
            .is_err()
        );
    }
    assert!(
        publish_bank_census_policy(
            &mut output,
            &b,
            &closed,
            counts,
            [8; 32],
            Instant::now(),
            || panic!("expired")
        )
        .is_err()
    );
    assert!(output.is_empty());
}

#[test]
fn full_bank_census_cli_selector_refuses_all_legacy_flags_and_preserves_entry_type() {
    let good = [
        BANK_CENSUS_FLAG,
        "--allow-unauthenticated-machine-code",
        "--devices",
        "7,9",
        "--timeout-ms",
        "10000",
        "--mode",
        "autoregressive",
    ]
    .map(OsString::from);
    parse_bank_scoped_census_args(&good).unwrap();
    assert!(parse_args(&good).is_err() && parse_scoped_args(&good).is_err());
    for flag in [
        FLAG,
        SCOPED_FLAG,
        crate::native_guarded_mlp_readiness_cli_v1::CENSUS_SCOPED_FLAG,
        crate::native_guarded_mlp_readiness_cli_v1::BANK_SCOPED_FLAG,
        crate::native_guarded_mlp_readiness_cli_v1::POSITION5_FLAG,
    ] {
        let mut bad = good.clone();
        bad[0] = flag.into();
        assert!(parse_bank_scoped_census_args(&bad).is_err());
    }
    for n in 0..good.len() {
        assert!(parse_bank_scoped_census_args(&good[..n]).is_err());
    }
    let mut extra = good.to_vec();
    extra.push("--readiness40".into());
    assert!(parse_bank_scoped_census_args(&extra).is_err());
    let mut wrong = good.clone();
    wrong[7] = "teacher-forced".into();
    assert!(parse_bank_scoped_census_args(&wrong).is_err());
    let _: unsafe fn(
        NativeOptions,
        &mut io::Cursor<Vec<u8>>,
        &mut Vec<u8>,
        &mut Vec<u8>,
    ) -> io::Result<()> = run_native_bank_scoped_census;
}
#[test]
fn full_bank_census_cli_refuses_changed_first_or_last_genuine_output_before_close() {
    let b = bootstrap();
    for index in [0, 255] {
        let mut backend = Mock::new(&b);
        backend.bank_census = Some(crate::native_catalog::forward::guarded_mlp_decode_v1::full2303::bank_census::State::new(long::Profile::Full2303).unwrap());
        let mut changed = vec![7; 256];
        changed[index] = 8;
        backend.altered = Some(changed);
        assert!(
            serve_closed(
                &mut backend,
                &mut input(&b, 2303, true).as_slice(),
                &mut Vec::new(),
                &b,
                &mut long::FrameBudget::new()
            )
            .is_err()
        );
        assert!(!backend.closed && backend.cancelled == 1 && backend.runs == 2303);
    }
}
