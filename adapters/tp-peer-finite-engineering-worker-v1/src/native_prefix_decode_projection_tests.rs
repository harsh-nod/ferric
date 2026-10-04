use super::*;
use crate::finite_projection_residual_decode_wire_v1 as selected;
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
    fn run(&mut self, input: &ForwardInput) -> io::Result<Run> {
        self.calls += 1;
        if self.fail == Some(self.calls) {
            return Err(io::Error::other("injected projection forward"));
        }
        let c = wire::tests::control();
        let mut logits = vec![0; 303872];
        logits[14..16].copy_from_slice(&0x3f80u16.to_le_bytes());
        Ok(Run {
            completion: NativeCompletion {
                profile_sha256: self.profile,
                generation: input.generation,
                position: input.cache_metadata[0],
                input_token: input.token,
                output_token: 7,
                embedding_ns: c.embedding_ns,
                tail_ns: c.tail_ns,
                layers: c
                    .layers
                    .into_iter()
                    .map(|r| LayerCompletion {
                        prefix_states: r.prefix_states,
                        mlp_states: r.tiles_states,
                        paired_ns: r.paired_ns,
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
        let mut request = wire::tests::request(&b.decode, position, None);
        request.profile_sha256 = profile;
        wire::write_request(&mut bytes, &mut budget, &request).unwrap();
    }
    let mut close = wire::tests::close(&b.decode, [0; 32]).0;
    close.profile_sha256 = profile;
    wire::write_request(&mut bytes, &mut budget, &close).unwrap();
    bytes
}
#[test]
fn projection_decode_protocol_keeps_four_payloads_controls_and_selected_close_chain() {
    let b = selected::tests::bootstrap();
    let mut f = backend(&b);
    let mut output = Vec::new();
    serve_projection(
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
        let (reply, control, capture) = wire::read_response(&mut bytes, &mut budget)
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
fn projection_decode_protocol_old_request_profile_is_refused_before_run() {
    let b = selected::tests::bootstrap();
    let mut f = backend(&b);
    let mut output = Vec::new();
    assert!(
        serve_projection(
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
fn projection_decode_protocol_wrong_completion_after_run_is_terminal() {
    let b = selected::tests::bootstrap();
    let mut f = backend(&b);
    f.profile = b.decode.sha256().unwrap();
    let mut output = Vec::new();
    assert!(
        serve_projection(
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
fn projection_decode_protocol_forward_close_and_writer_failures_poison() {
    let b = selected::tests::bootstrap();
    let input = stream(&b, b.sha256().unwrap());
    for fail in 1..=5 {
        let mut f = backend(&b);
        f.fail = Some(fail);
        let mut output = Vec::new();
        assert!(
            serve_projection(
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
        while let Some((reply, _, _)) = wire::read_response(&mut bytes, &mut budget).unwrap() {
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
        serve_projection(
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
