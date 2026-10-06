use super::*;
use crate::finite_prefix_decode_wire_v1::tests::{bootstrap, close, completed, request};

struct Mock {
    observer: bool,
    events: Vec<&'static str>,
    fail_receipt: bool,
    fail_close: bool,
}
impl Backend for Mock {
    fn run(&mut self, input: &ForwardInput) -> io::Result<Run> {
        self.events.push("run");
        let c = wire::tests::control();
        let mut logits = vec![0; 303872];
        logits[200..202].copy_from_slice(&0x3f80_u16.to_le_bytes());
        Ok(Run {
            completion: crate::native_catalog::forward::prefix_tiles_decode_v6::Completion {
                profile_sha256: bootstrap(InputMode::TeacherForced).sha256()?,
                generation: input.generation,
                position: input.cache_metadata[0],
                input_token: input.token,
                output_token: 100,
                embedding_ns: c.embedding_ns,
                tail_ns: c.tail_ns,
                layers: c
                    .layers
                    .into_iter()
                    .map(
                        |l| crate::resident_layer::prefix_tiles_decode_v6::Completion {
                            prefix_states: l.prefix_states,
                            mlp_states: l.tiles_states,
                            timing: crate::resident_layer::prefix_tiles_decode_v6::Timing::Paired(
                                l.paired_ns,
                            ),
                        },
                    )
                    .collect(),
            },
            layer_hidden: vec![vec![0; 8192]; 36],
            final_normalized: vec![0; 8192],
            logits,
        })
    }
    fn close(&mut self) -> io::Result<()> {
        self.events.push("close");
        if self.fail_close {
            Err(io::Error::other("injected Close"))
        } else {
            Ok(())
        }
    }
    fn completed(&mut self, _: &Completion) -> io::Result<()> {
        if self.observer {
            self.events.push("host-binding");
        }
        if self.fail_receipt {
            Err(io::Error::other("injected sidecar binding"))
        } else {
            Ok(())
        }
    }
}
fn requests() -> (Bootstrap, Vec<u8>) {
    let b = bootstrap(InputMode::TeacherForced);
    let mut bytes = Vec::new();
    let mut budget = FrameBudget::new();
    let mut chain = Chain::new(b.registration, b.sha256().unwrap());
    for position in 0..4 {
        let r = request(&b, position, None);
        wire::write_request(&mut bytes, &mut budget, &r).unwrap();
        completed(&r, &mut chain, 100);
    }
    wire::write_request(&mut bytes, &mut budget, &close(&b, chain.digest()).0).unwrap();
    (b, bytes)
}
#[test]
fn prefix_host_worker_optional_binding_keeps_four_wire_and_execution_trace() {
    let (b, input) = requests();
    let mut outputs = Vec::new();
    for observer in [false, true] {
        let mut backend = Mock {
            observer,
            events: Vec::new(),
            fail_receipt: false,
            fail_close: false,
        };
        let mut out = Vec::new();
        serve(
            &mut backend,
            &mut &input[..],
            &mut out,
            &b,
            &mut FrameBudget::new(),
        )
        .unwrap();
        assert_eq!(backend.events.iter().filter(|s| **s == "run").count(), 4);
        assert_eq!(
            backend
                .events
                .iter()
                .filter(|s| **s == "host-binding")
                .count(),
            if observer { 4 } else { 0 }
        );
        assert_eq!(backend.events.last(), Some(&"close"));
        outputs.push(out);
    }
    assert_eq!(outputs[0], outputs[1]);
}
#[test]
fn prefix_host_worker_failed_binding_or_close_has_no_healthy_final_response() {
    let (b, input) = requests();
    for fail_receipt in [false, true] {
        let mut backend = Mock {
            observer: true,
            events: Vec::new(),
            fail_receipt,
            fail_close: !fail_receipt,
        };
        let mut out = Vec::new();
        assert!(
            serve(
                &mut backend,
                &mut &input[..],
                &mut out,
                &b,
                &mut FrameBudget::new()
            )
            .is_err()
        );
        let mut cursor = &out[..];
        let mut budget = FrameBudget::new();
        let mut count = 0;
        while let Some((response, _, _)) = wire::read_response(&mut cursor, &mut budget).unwrap() {
            assert!(!response.native_closed);
            count += 1;
        }
        assert_eq!(count, if fail_receipt { 1 } else { 4 });
    }
}
