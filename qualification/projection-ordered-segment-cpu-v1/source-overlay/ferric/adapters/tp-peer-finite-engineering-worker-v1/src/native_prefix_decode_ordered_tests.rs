use super::*;
use crate::finite_projection_residual_mlp_ordered_wire_v1 as selected;
use crate::native_catalog::forward::prefix_tiles_decode_v6::Completion as NativeCompletion;
use crate::resident_layer::prefix_tiles_decode_v6::Completion as LayerCompletion;

struct Fake {
    profile: [u8; 32],
    calls: u64,
    closes: u64,
    fail: Option<u64>,
    poisoned: bool,
}
impl Backend for Fake {
    fn ordered(&self) -> bool {
        true
    }
    fn run(&mut self, input: &ForwardInput) -> io::Result<Run> {
        self.calls += 1;
        if self.fail == Some(self.calls) {
            return Err(io::Error::other("injected projection forward"));
        }
        let c = wire::tests::control();
        let mut logits = vec![0; 303872];
        let output = 6 + self.calls as u32;
        let offset = output as usize * 2;
        logits[offset..offset + 2].copy_from_slice(&0x3f80u16.to_le_bytes());
        Ok(Run {
            completion: NativeCompletion {
                profile_sha256: self.profile,
                generation: input.generation,
                position: input.cache_metadata[0],
                input_token: input.token,
                output_token: output,
                embedding_ns: c.embedding_ns,
                tail_ns: c.tail_ns,
                layers: c
                    .layers
                    .into_iter()
                    .map(|r| LayerCompletion {
                        prefix_states: r.prefix_states,
                        mlp_states: r.tiles_states,
                        timing: crate::resident_layer::prefix_tiles_decode_v6::Timing::Ordered {
                            prefix_ns: [1, 2],
                            segment_host_ns: 3,
                            final_residual_ns: [4, 5],
                        },
                    })
                    .collect(),
            },
            layer_hidden: vec![vec![0; 8192]; 36],
            final_normalized: vec![0; 8192],
            logits,
        })
    }
    fn close(&mut self) -> io::Result<()> {
        self.closes += 1;
        if self.fail == Some(5) {
            Err(io::Error::other("injected projection close"))
        } else {
            Ok(())
        }
    }
    fn failed(&mut self) {
        self.poisoned = true;
    }
}
fn backend(b: &selected::Bootstrap) -> Fake {
    Fake {
        profile: b.sha256().unwrap(),
        calls: 0,
        closes: 0,
        fail: None,
        poisoned: false,
    }
}
fn stream(b: &selected::Bootstrap, profile: [u8; 32]) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut budget = FrameBudget::new();
    for position in 0..4 {
        let previous = (position > 0).then_some(position + 6);
        let mut request = wire::tests::request(&b.decode, position, previous);
        request.profile_sha256 = profile;
        wire::write_request(&mut bytes, &mut budget, &request).unwrap();
    }
    let mut close = wire::tests::close(&b.decode, [0; 32]).0;
    close.profile_sha256 = profile;
    wire::write_request(&mut bytes, &mut budget, &close).unwrap();
    bytes
}

#[test]
fn ordered_ar4_protocol_four_own_outputs_have_exact_payloads_and_close_chain() {
    let b = selected::tests::ar_bootstrap();
    let profile = b.sha256().unwrap();
    let mut f = backend(&b);
    let mut output = Vec::new();
    serve_ordered(
        &mut f,
        &mut &stream(&b, profile)[..],
        &mut output,
        &b,
        &mut FrameBudget::new(),
    )
    .unwrap();
    assert_eq!((f.calls, f.closes, f.poisoned), (4, 1, false));
    let mut bytes = &output[..];
    let mut budget = FrameBudget::new();
    let mut chain = Chain::new(b.decode.registration, profile);
    let mut previous = None;
    for position in 0..4 {
        let (reply, control, payload) = selected::read_response(&mut bytes, &mut budget)
            .unwrap()
            .unwrap();
        assert_eq!(reply.profile_sha256, profile);
        let Event::Completed(completed) = reply.event else {
            panic!("expected forward")
        };
        assert_eq!(completed.input_token, b.input(position, previous).unwrap());
        assert_eq!(completed.output_token, position + 7);
        assert_eq!(completed.chain, chain.advance(&completed));
        assert_eq!(payload.len(), 606976);
        control.unwrap().validate().unwrap();
        previous = Some(completed.output_token);
    }
    let (reply, control, payload) = selected::read_response(&mut bytes, &mut budget)
        .unwrap()
        .unwrap();
    assert_eq!(reply.id, 5);
    assert!(reply.native_closed && control.is_none() && payload.is_empty());
    assert!(
        matches!(reply.event, Event::Closed { completed_forwards: 4, transcript_sha256 } if transcript_sha256 == chain.digest())
    );
    assert!(bytes.is_empty());
}

fn ar_requests(b: &selected::Bootstrap) -> Vec<wire::Request> {
    let profile = b.sha256().unwrap();
    (0..4)
        .map(|position| {
            let mut request =
                wire::tests::request(&b.decode, position, (position > 0).then_some(position + 6));
            request.profile_sha256 = profile;
            request
        })
        .collect()
}
fn request_bytes(requests: &[wire::Request]) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut budget = FrameBudget::new();
    for request in requests {
        wire::write_request(&mut bytes, &mut budget, request).unwrap();
    }
    bytes
}

#[test]
fn ordered_ar4_protocol_wrong_previous_token_or_profile_never_runs_that_forward() {
    let b = selected::tests::ar_bootstrap();
    for position in 0..4 {
        for wrong_profile in [false, true] {
            let mut requests = ar_requests(&b);
            if wrong_profile {
                requests[position].profile_sha256 = b.decode.sha256().unwrap();
            } else if let Command::Forward { token, .. } = &mut requests[position].command {
                *token ^= 1;
            }
            let mut f = backend(&b);
            let mut output = Vec::new();
            assert!(
                serve_ordered(
                    &mut f,
                    &mut &request_bytes(&requests)[..],
                    &mut output,
                    &b,
                    &mut FrameBudget::new()
                )
                .is_err()
            );
            assert_eq!((f.calls, f.closes), (position as u64, 0));
            assert!(f.poisoned);
        }
    }
}

#[test]
fn ordered_ar4_protocol_eof_and_early_close_poison_without_successful_close() {
    let b = selected::tests::ar_bootstrap();
    let requests = ar_requests(&b);
    for completed in 0..=4 {
        for early_close in [false, true] {
            if early_close && completed == 4 {
                continue;
            }
            let mut subset = requests[..completed].to_vec();
            if early_close {
                let mut close = wire::tests::close(&b.decode, [0; 32]).0;
                close.profile_sha256 = b.sha256().unwrap();
                subset.push(close);
            }
            let mut f = backend(&b);
            let mut output = Vec::new();
            assert!(
                serve_ordered(
                    &mut f,
                    &mut &request_bytes(&subset)[..],
                    &mut output,
                    &b,
                    &mut FrameBudget::new()
                )
                .is_err()
            );
            assert_eq!((f.calls, f.closes), (completed as u64, 0));
            assert!(f.poisoned);
            let mut bytes = &output[..];
            let mut budget = FrameBudget::new();
            while let Some((reply, _, _)) =
                selected::read_response(&mut bytes, &mut budget).unwrap()
            {
                assert!(!reply.native_closed);
            }
        }
    }
    let mut f = backend(&b);
    f.fail = Some(5);
    assert!(
        serve_ordered(
            &mut f,
            &mut &stream(&b, b.sha256().unwrap())[..],
            &mut Vec::new(),
            &b,
            &mut FrameBudget::new()
        )
        .is_err()
    );
    assert_eq!((f.calls, f.closes, f.poisoned), (4, 1, true));
}
#[test]
fn ordered_decode_protocol_keeps_four_payloads_controls_and_selected_close_chain() {
    let b = selected::tests::ar_bootstrap();
    let mut f = backend(&b);
    let mut output = Vec::new();
    serve_ordered(
        &mut f,
        &mut &stream(&b, b.sha256().unwrap())[..],
        &mut output,
        &b,
        &mut FrameBudget::new(),
    )
    .unwrap();
    assert_eq!((f.calls, f.closes, f.poisoned), (4, 1, false));
    let mut bytes = &output[..];
    let mut budget = FrameBudget::new();
    let mut chain = Chain::new(b.decode.registration, b.sha256().unwrap());
    for id in 1..=5 {
        let (reply, control, capture) = selected::read_response(&mut bytes, &mut budget)
            .unwrap()
            .unwrap();
        assert_eq!(reply.id, id);
        assert_eq!(reply.profile_sha256, b.sha256().unwrap());
        assert!(
            !reply.numerical_acceptance && !reply.performance_claim && !reply.production_authority
        );
        match reply.event {
            Event::Completed(c) => {
                assert!(id <= 4);
                assert_eq!(capture.len(), 606976);
                control.unwrap().validate().unwrap();
                assert_eq!(c.chain, chain.advance(&c));
                assert!(!reply.native_closed);
            }
            Event::Closed {
                completed_forwards,
                transcript_sha256,
            } => {
                assert_eq!((id, completed_forwards), (5, 4));
                assert_eq!(transcript_sha256, chain.digest());
                assert!(reply.native_closed && control.is_none() && capture.is_empty());
            }
        }
    }
    assert!(bytes.is_empty());
}
#[test]
fn ordered_decode_protocol_old_request_profile_is_refused_before_run() {
    let b = selected::tests::ar_bootstrap();
    let mut f = backend(&b);
    let mut output = Vec::new();
    assert!(
        serve_ordered(
            &mut f,
            &mut &stream(&b, b.decode.sha256().unwrap())[..],
            &mut output,
            &b,
            &mut FrameBudget::new()
        )
        .is_err()
    );
    assert_eq!((f.calls, f.closes), (0, 0));
    assert!(f.poisoned && output.is_empty());
}
#[test]
fn ordered_decode_protocol_wrong_completion_after_run_is_terminal() {
    let b = selected::tests::ar_bootstrap();
    let mut f = backend(&b);
    f.profile = b.decode.sha256().unwrap();
    let mut output = Vec::new();
    assert!(
        serve_ordered(
            &mut f,
            &mut &stream(&b, b.sha256().unwrap())[..],
            &mut output,
            &b,
            &mut FrameBudget::new()
        )
        .is_err()
    );
    assert_eq!((f.calls, f.closes), (1, 0));
    assert!(f.poisoned && output.is_empty());
}
#[test]
fn ordered_decode_protocol_forward_close_and_writer_failures_poison() {
    let b = selected::tests::ar_bootstrap();
    let input = stream(&b, b.sha256().unwrap());
    for fail in 1..=5 {
        let mut f = backend(&b);
        f.fail = Some(fail);
        let mut output = Vec::new();
        assert!(
            serve_ordered(
                &mut f,
                &mut &input[..],
                &mut output,
                &b,
                &mut FrameBudget::new()
            )
            .is_err()
        );
        assert!(f.poisoned);
        assert_eq!(f.calls, fail.min(4));
        assert_eq!(f.closes, u64::from(fail == 5));
        let mut bytes = &output[..];
        let mut budget = FrameBudget::new();
        while let Some((reply, _, _)) = selected::read_response(&mut bytes, &mut budget).unwrap() {
            assert!(!reply.native_closed);
        }
    }
    struct Broken;
    impl Write for Broken {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("injected write"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut f = backend(&b);
    assert!(
        serve_ordered(
            &mut f,
            &mut &input[..],
            &mut Broken,
            &b,
            &mut FrameBudget::new()
        )
        .is_err()
    );
    assert!(f.poisoned);
    assert_eq!((f.calls, f.closes), (1, 0));
}

fn serve_ordered(
    backend: &mut impl Backend,
    r: &mut impl Read,
    w: &mut impl Write,
    b: &selected::Bootstrap,
    incoming: &mut FrameBudget,
) -> io::Result<()> {
    let result = b
        .sha256()
        .and_then(|profile| serve_profile(backend, r, w, &b.decode, incoming, profile));
    if result.is_err() {
        backend.failed();
    }
    result
}
#[test]
fn ordered_completion_cannot_be_serialized_by_legacy_control() {
    let b = selected::tests::ar_bootstrap();
    let mut f = backend(&b);
    let request = ar_requests(&b).remove(0);
    let Command::Forward {
        generation,
        token,
        cache_metadata,
        rotary_bits,
    } = &request.command
    else {
        panic!()
    };
    let input = ForwardInput {
        registration: b.decode.registration,
        generation: *generation,
        token: *token,
        cache_metadata: cache_metadata.clone().try_into().unwrap(),
        rotary_bits: rotary_bits.clone().try_into().unwrap(),
    };
    let run = f.run(&input).unwrap();
    assert!(
        observation(
            run,
            &request,
            &mut Chain::new(b.decode.registration, b.sha256().unwrap())
        )
        .is_err()
    );
}
