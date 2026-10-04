//! The actual wire error boundary must poison the clock route after run errors.
use super::*;
use std::cell::Cell;

struct Fake {
    calls: usize,
    poisoned: bool,
    run_error: bool,
}
impl Backend for Fake {
    fn run(&mut self, input: &ForwardInput) -> io::Result<Run> {
        self.calls += 1;
        if self.run_error {
            return Err(io::Error::other("synthetic sample/run failure"));
        }
        // Models a committed run with a malformed returned capture. The real
        // observation validator, not the clock recorder, detects this error.
        Ok(Run {
            completion: crate::native_catalog::forward::prefix_tiles_decode_v6::Completion {
                profile_sha256: wire::tests::bootstrap(InputMode::TeacherForced).sha256()?,
                generation: input.generation,
                position: input.cache_metadata[0],
                input_token: input.token,
                output_token: 100,
                embedding_ns: [0; 2],
                layers: Vec::new(),
                tail_ns: [0; 3],
            },
            layer_hidden: Vec::new(),
            final_normalized: Vec::new(),
            logits: Vec::new(),
        })
    }
    fn close(&mut self) -> io::Result<()> {
        panic!("failed forward must not Close");
    }
    fn failed(&mut self) {
        self.poisoned = true;
    }
}
#[test]
fn clock_wire_failure_boundary_poisoning_includes_post_run_capture_errors() {
    let b = wire::tests::bootstrap(InputMode::TeacherForced);
    let mut input = Vec::new();
    wire::write_request(
        &mut input,
        &mut FrameBudget::new(),
        &wire::tests::request(&b, 0, None),
    )
    .unwrap();
    for run_error in [false, true] {
        let mut fake = Fake {
            calls: 0,
            poisoned: false,
            run_error,
        };
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
        assert_eq!(
            error.to_string(),
            if run_error {
                "synthetic sample/run failure"
            } else {
                "tiles actual observation identity/extent"
            }
        );
        assert!(fake.poisoned && !published.get());
        assert_eq!(fake.calls, 1);
    }
}
#[test]
fn clock_wire_failure_boundary_poisoning_includes_eof_before_any_forward() {
    let b = wire::tests::bootstrap(InputMode::TeacherForced);
    let mut fake = Fake {
        calls: 0,
        poisoned: false,
        run_error: false,
    };
    let error = serve_and_finish(
        &mut fake,
        &mut &[][..],
        &mut Vec::new(),
        &b,
        &mut FrameBudget::new(),
        |_| panic!("EOF cannot publish"),
    )
    .unwrap_err();
    assert_eq!(error.to_string(), "tiles EOF before Close");
    assert!(fake.poisoned);
    assert_eq!(fake.calls, 0);
}
