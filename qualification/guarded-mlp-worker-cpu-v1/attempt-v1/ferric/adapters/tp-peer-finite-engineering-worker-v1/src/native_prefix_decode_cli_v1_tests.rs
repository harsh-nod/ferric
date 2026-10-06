use super::*;
use crate::native_catalog::forward::prefix_tiles_decode_v6::{
    Completion as NativeCompletion, Profile,
};
use crate::resident_layer::prefix_tiles_decode_v6::Completion as LayerCompletion;
use wire::tests::{bootstrap, control, request};

fn args(mode: &str) -> Vec<OsString> {
    [
        "--engineering-native-prefix-decode-v1",
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
fn fake_run(b: &Bootstrap, input: &ForwardInput, winner: u32) -> Run {
    let c = control();
    let mut logits = vec![0; 303872];
    logits[winner as usize * 2..][..2].copy_from_slice(&0x3f80u16.to_le_bytes());
    Run {
        completion: NativeCompletion {
            profile_sha256: b.sha256().unwrap(),
            generation: input.generation,
            position: input.cache_metadata[0],
            input_token: input.token,
            output_token: winner,
            embedding_ns: c.embedding_ns,
            tail_ns: c.tail_ns,
            layers: c
                .layers
                .into_iter()
                .map(|l| LayerCompletion {
                    prefix_states: l.prefix_states,
                    mlp_states: l.tiles_states,
                    timing: crate::resident_layer::prefix_tiles_decode_v6::Timing::Paired(
                        l.paired_ns,
                    ),
                })
                .collect(),
        },
        layer_hidden: vec![vec![0; 8192]; 36],
        final_normalized: vec![0; 8192],
        logits,
    }
}
struct Fake {
    b: Bootstrap,
    calls: Vec<u32>,
    fail: Option<usize>,
    closes: usize,
    poisoned: bool,
}
impl Backend for Fake {
    fn run(&mut self, input: &ForwardInput) -> io::Result<Run> {
        assert!(!self.poisoned);
        self.calls.push(input.token);
        if self.fail == Some(self.calls.len()) {
            self.poisoned = true;
            return Err(io::Error::other("synthetic native failure"));
        }
        Ok(fake_run(&self.b, input, self.calls.len() as u32 + 6))
    }
    fn close(&mut self) -> io::Result<()> {
        self.closes += 1;
        if self.fail == Some(5) {
            self.poisoned = true;
            Err(io::Error::other("synthetic close failure"))
        } else {
            Ok(())
        }
    }
}
fn fake(b: Bootstrap, fail: Option<usize>) -> Fake {
    Fake {
        b,
        calls: Vec::new(),
        fail,
        closes: 0,
        poisoned: false,
    }
}
fn stream(b: &Bootstrap) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut budget = FrameBudget::new();
    for p in 0..4 {
        wire::write_request(&mut bytes, &mut budget, &request(b, p, Some(p + 6))).unwrap();
    }
    wire::write_request(&mut bytes, &mut budget, &wire::tests::close(b, [0; 32]).0).unwrap();
    bytes
}
fn replies(bytes: &[u8]) -> Vec<Response> {
    let mut input = bytes;
    let mut budget = FrameBudget::new();
    let mut out = Vec::new();
    while !input.is_empty() {
        match wire::read_response(&mut input, &mut budget) {
            Ok(Some((r, _, _))) => out.push(r),
            _ => break,
        }
    }
    out
}
#[test]
fn prefix_decode_cli_exact_mode_and_no_cached_or_legacy_fallback() {
    for m in ["teacher-forced", "autoregressive"] {
        let good = args(m);
        parse_args(&good).unwrap();
        for n in 0..good.len() {
            assert!(parse_args(&good[..n]).is_err());
        }
        let mut bad = good.clone();
        bad.extend(["--kernel-admission".into(), "cached-immutable".into()]);
        assert!(parse_args(&bad).is_err());
        let mut bad = good;
        bad[0] = "--engineering-native-tiles-decode-v1".into();
        assert!(parse_args(&bad).is_err());
    }
    assert!(parse_args(&args("auto")).is_err());
}
#[test]
fn prefix_decode_wire_profile_equals_actual_backend_for_both_modes() {
    for m in [InputMode::TeacherForced, InputMode::Autoregressive] {
        let b = bootstrap(m);
        let p = Profile::new(
            &b.scope,
            b.registration,
            b.prefix_image.sha256,
            b.tiles_image.sha256,
            mode(&b).unwrap(),
            b.timeout_ms,
            b.device_ids,
        )
        .unwrap();
        assert_eq!(b.sha256().unwrap(), p.sha256());
        let mut changed = b.clone();
        changed.prefix_image.sha256[0] ^= 1;
        assert_ne!(changed.sha256().unwrap(), p.sha256());
        changed = b.clone();
        changed.tiles_image.sha256[0] ^= 1;
        assert_ne!(changed.sha256().unwrap(), p.sha256());
    }
}
#[test]
fn prefix_decode_prepare_refuses_missing_tail_mode_and_changed_begin_before_open() {
    let b = bootstrap(InputMode::TeacherForced);
    let options = parse_args(&args("teacher-forced")).unwrap();
    let mut missing = b.clone();
    missing.begin.tail_image = None;
    assert!(
        wire::write_bootstrap(
            &mut Vec::new(),
            &mut FrameBudget::new(),
            &missing,
            &[8],
            &[9]
        )
        .is_err()
    );
    let mut bytes = Vec::new();
    wire::write_bootstrap(&mut bytes, &mut FrameBudget::new(), &b, &[8], &[9]).unwrap();
    assert!(prepare(&options, &mut &bytes[..], &mut FrameBudget::new()).is_err());
    let ar = parse_args(&args("autoregressive")).unwrap();
    assert!(prepare(&ar, &mut &bytes[..], &mut FrameBudget::new()).is_err());
}
#[test]
fn prefix_decode_serve_tf_ar_four_actual_inputs_and_close_five() {
    for m in [InputMode::TeacherForced, InputMode::Autoregressive] {
        let b = bootstrap(m);
        let mut f = fake(b.clone(), None);
        let mut output = Vec::new();
        serve(
            &mut f,
            &mut &stream(&b)[..],
            &mut output,
            &b,
            &mut FrameBudget::new(),
        )
        .unwrap();
        assert_eq!(
            f.calls,
            if m == InputMode::TeacherForced {
                vec![9112, 2190, 3772, 220]
            } else {
                vec![9112, 7, 8, 9]
            }
        );
        assert_eq!(f.closes, 1);
        assert!(!f.poisoned);
        let actual = replies(&output);
        assert_eq!(actual.len(), 5);
        assert!(actual.last().unwrap().native_closed);
        assert!(matches!(
            actual.last().unwrap().event,
            Event::Closed {
                completed_forwards: 4,
                ..
            }
        ));
    }
}
#[test]
fn prefix_decode_wrong_history_or_profile_stops_before_next_native_call() {
    let b = bootstrap(InputMode::Autoregressive);
    for change in 0..4 {
        let mut bytes = Vec::new();
        let mut budget = FrameBudget::new();
        wire::write_request(&mut bytes, &mut budget, &request(&b, 0, None)).unwrap();
        let mut r = request(&b, 1, Some(7));
        match change {
            0 => {
                if let Command::Forward { token, .. } = &mut r.command {
                    *token = 123;
                }
            }
            1 => r.profile_sha256[0] ^= 1,
            2 => r.session[0] ^= 1,
            _ => r.registration[0] ^= 1,
        }
        wire::write_request(&mut bytes, &mut budget, &r).unwrap();
        let mut f = fake(b.clone(), None);
        let mut output = Vec::new();
        assert!(
            serve(
                &mut f,
                &mut &bytes[..],
                &mut output,
                &b,
                &mut FrameBudget::new()
            )
            .is_err()
        );
        assert_eq!(f.calls.len(), 1);
        assert_eq!(f.closes, 0);
        assert!(!replies(&output).iter().any(|v| v.native_closed));
    }
}
#[test]
fn prefix_decode_native_failure_and_close_failure_never_retry_or_publish_close() {
    let b = bootstrap(InputMode::TeacherForced);
    for fail in 1..=5 {
        let mut f = fake(b.clone(), Some(fail));
        let mut output = Vec::new();
        assert!(
            serve(
                &mut f,
                &mut &stream(&b)[..],
                &mut output,
                &b,
                &mut FrameBudget::new()
            )
            .is_err()
        );
        assert!(f.poisoned);
        assert_eq!(f.calls.len(), fail.min(4));
        assert_eq!(f.closes, usize::from(fail == 5));
        assert!(!replies(&output).iter().any(|v| v.native_closed));
    }
}
struct Cut {
    remaining: usize,
    bytes: Vec<u8>,
}
impl Write for Cut {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.remaining == 0 {
            return Err(io::Error::other("synthetic write loss"));
        }
        let n = bytes.len().min(self.remaining);
        self.remaining -= n;
        self.bytes.extend_from_slice(&bytes[..n]);
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
#[test]
fn prefix_decode_each_response_write_loss_and_eof_withhold_success() {
    let b = bootstrap(InputMode::TeacherForced);
    let mut good = Vec::new();
    let mut f = fake(b.clone(), None);
    serve(
        &mut f,
        &mut &stream(&b)[..],
        &mut good,
        &b,
        &mut FrameBudget::new(),
    )
    .unwrap();
    let mut rest = &good[..];
    let mut budget = FrameBudget::new();
    let mut ends = Vec::new();
    while !rest.is_empty() {
        wire::read_response(&mut rest, &mut budget)
            .unwrap()
            .unwrap();
        ends.push(good.len() - rest.len());
    }
    for (i, end) in ends.into_iter().enumerate() {
        let mut cut = Cut {
            remaining: end - 1,
            bytes: Vec::new(),
        };
        let mut f = fake(b.clone(), None);
        assert!(
            serve(
                &mut f,
                &mut &stream(&b)[..],
                &mut cut,
                &b,
                &mut FrameBudget::new()
            )
            .is_err()
        );
        assert_eq!(f.calls.len(), (i + 1).min(4));
        assert_eq!(f.closes, usize::from(i == 4));
        assert!(!replies(&cut.bytes).iter().any(|v| v.native_closed));
    }
    let mut f = fake(b.clone(), None);
    assert!(
        serve(
            &mut f,
            &mut &[][..],
            &mut Vec::new(),
            &b,
            &mut FrameBudget::new()
        )
        .is_err()
    );
    assert!(f.calls.is_empty());
    assert_eq!(f.closes, 0);
}
#[test]
fn prefix_decode_observation_rejects_bad_generation_states_and_output_extent() {
    let b = bootstrap(InputMode::TeacherForced);
    let r = request(&b, 0, None);
    let input = ForwardInput {
        registration: b.registration,
        generation: 1,
        token: 9112,
        cache_metadata: core::array::from_fn(|i| if i == 0 { 0 } else { i as u32 - 1 }),
        rotary_bits: [0; 128],
    };
    for change in 0..6 {
        let mut run = fake_run(&b, &input, 7);
        match change {
            0 => run.completion.generation = 2,
            1 => run.completion.layers[35].prefix_states[1][0] = 2,
            2 => run.completion.layers[35].mlp_states[1][547] = 0,
            3 => {
                run.layer_hidden.pop();
            }
            4 => {
                run.logits.pop();
            }
            _ => run.completion.profile_sha256[0] ^= 1,
        }
        assert!(
            observation(
                run,
                &r,
                &mut Chain::new(b.registration, b.sha256().unwrap())
            )
            .is_err()
        );
    }
}
