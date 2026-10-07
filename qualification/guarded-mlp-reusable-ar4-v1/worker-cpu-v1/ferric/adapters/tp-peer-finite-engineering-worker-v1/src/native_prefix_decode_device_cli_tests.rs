//! The actual serve/publication gate, with an explicitly synthetic backend.
use super::*;
use std::cell::Cell;

struct Fake {
    calls: usize,
    controls: usize,
    completed: usize,
    closed: bool,
    fail_control: Option<usize>,
    fail_completion: Option<usize>,
    fail_close: bool,
}
impl Fake {
    fn new() -> Self {
        Self {
            calls: 0,
            controls: 0,
            completed: 0,
            closed: false,
            fail_control: None,
            fail_completion: None,
            fail_close: false,
        }
    }
}
impl Backend for Fake {
    fn run(&mut self, input: &ForwardInput) -> io::Result<Run> {
        assert_eq!(self.controls, self.completed);
        assert_eq!(self.calls, self.completed);
        self.calls += 1;
        let c = wire::tests::control();
        let mut logits = vec![0; 303872];
        logits[200..202].copy_from_slice(&0x3f80_u16.to_le_bytes());
        Ok(Run {
            completion: crate::native_catalog::forward::prefix_tiles_decode_v6::Completion {
                profile_sha256: wire::tests::bootstrap(InputMode::TeacherForced).sha256()?,
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
    fn control(&mut self, control: &Control) -> io::Result<()> {
        assert_eq!(self.calls, self.controls + 1);
        assert_eq!(control.encode(), wire::tests::control().encode());
        if self.fail_control == Some(self.controls) {
            return Err(io::Error::other("injected interval join"));
        }
        self.controls += 1;
        Ok(())
    }
    fn completed(&mut self, completion: &Completion) -> io::Result<()> {
        assert_eq!(self.controls, self.completed + 1);
        assert_eq!(completion.control, part(&wire::tests::control().encode()));
        if self.fail_completion == Some(self.completed) {
            return Err(io::Error::other("injected completion join"));
        }
        self.completed += 1;
        Ok(())
    }
    fn close(&mut self) -> io::Result<()> {
        assert_eq!(self.completed, 4);
        if self.fail_close {
            return Err(io::Error::other("injected Close"));
        }
        self.closed = true;
        Ok(())
    }
}
fn input() -> (Bootstrap, Vec<u8>) {
    let b = wire::tests::bootstrap(InputMode::TeacherForced);
    let mut bytes = Vec::new();
    let mut budget = FrameBudget::new();
    for position in 0..4 {
        wire::write_request(
            &mut bytes,
            &mut budget,
            &wire::tests::request(&b, position, None),
        )
        .unwrap();
    }
    wire::write_request(&mut bytes, &mut budget, &wire::tests::close(&b, [0; 32]).0).unwrap();
    (b, bytes)
}
#[test]
fn device_cli_gate_joins_four_controls_before_publication_after_close() {
    let (b, input) = input();
    let mut fake = Fake::new();
    let mut output = Vec::new();
    let published = Cell::new(false);
    serve_and_finish(
        &mut fake,
        &mut &input[..],
        &mut output,
        &b,
        &mut FrameBudget::new(),
        |f| {
            assert!(f.closed);
            assert_eq!(f.completed, 4);
            published.set(true);
            Ok(())
        },
    )
    .unwrap();
    assert!(published.get());
    let mut bytes = &output[..];
    let mut budget = FrameBudget::new();
    for i in 0..5 {
        let (response, _, _) = wire::read_response(&mut bytes, &mut budget)
            .unwrap()
            .unwrap();
        assert_eq!(response.native_closed, i == 4);
    }
    assert!(bytes.is_empty());
}
#[test]
fn device_cli_gate_recording_completion_and_close_failures_never_publish() {
    let (b, input) = input();
    for index in [0, 2, 3] {
        for kind in 0..3 {
            let mut fake = Fake::new();
            match kind {
                0 => fake.fail_control = Some(index),
                1 => fake.fail_completion = Some(index),
                _ => fake.fail_close = true,
            }
            let published = Cell::new(false);
            let error = serve_and_finish(
                &mut fake,
                &mut &input[..],
                &mut Vec::new(),
                &b,
                &mut FrameBudget::new(),
                |_| {
                    published.set(true);
                    Ok(())
                },
            )
            .unwrap_err();
            let (message, calls, controls, completed) = match kind {
                0 => ("injected interval join", index + 1, index, index),
                1 => ("injected completion join", index + 1, index + 1, index),
                _ => ("injected Close", 4, 4, 4),
            };
            assert_eq!(error.to_string(), message);
            assert_eq!(
                (fake.calls, fake.controls, fake.completed),
                (calls, controls, completed)
            );
            assert!(!published.get());
            assert!(!fake.closed);
        }
    }
}
struct Cut {
    limit: usize,
    written: usize,
}
impl Write for Cut {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.written == self.limit {
            return Err(io::Error::other("injected wire write"));
        }
        let n = bytes.len().min(self.limit - self.written);
        self.written += n;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
#[test]
fn device_cli_gate_each_response_write_failure_withholds_sidecar() {
    let (b, input) = input();
    let mut output = Vec::new();
    serve(
        &mut Fake::new(),
        &mut &input[..],
        &mut output,
        &b,
        &mut FrameBudget::new(),
    )
    .unwrap();
    let mut bytes = &output[..];
    let mut budget = FrameBudget::new();
    for index in 0..5 {
        wire::read_response(&mut bytes, &mut budget)
            .unwrap()
            .unwrap();
        let mut cut = Cut {
            limit: output.len() - bytes.len() - 1,
            written: 0,
        };
        let published = Cell::new(false);
        let mut fake = Fake::new();
        assert!(
            serve_and_finish(
                &mut fake,
                &mut &input[..],
                &mut cut,
                &b,
                &mut FrameBudget::new(),
                |_| {
                    published.set(true);
                    Ok(())
                }
            )
            .is_err()
        );
        assert!(!published.get());
        assert_eq!(fake.closed, index == 4);
        assert_eq!(fake.completed, index);
    }
}
