use super::*;
use crate::finite_rearm_smoke_wire_v1::Profile;
use crate::forward_sequence::ForwardCompletion;
use crate::native_catalog::forward::ForwardRun;
use crate::resident_layer::LayerCompletion;

const REG: [u8; 32] = [4; 32];
fn args(capture: bool) -> Vec<OsString> {
    let mut result: Vec<_> = [
        "--engineering-native-rearm-smoke-v1",
        "--allow-unauthenticated-machine-code",
        "--devices",
        "11,22",
        "--timeout-ms",
        "1000",
    ]
    .into_iter()
    .map(OsString::from)
    .collect();
    if capture {
        result.push("--capture-layer0".into());
    }
    result
}
fn bootstrap(capture_layer0: bool) -> Bootstrap {
    Bootstrap {
        protocol: 1,
        profile: Profile::RearmFourForwardV1,
        device_ids: [11, 22],
        timeout_ms: 1000,
        scope: setup::Scope {
            bundle_id: [1; 32],
            model_id: [2; 32],
            session: [3; 32],
            pool_identity: 1,
            group_id: 0,
            child_identity: std::process::id(),
        },
        prompt_tokens: vec![9112, 2190, 3772, 220],
        capture_layer0,
    }
}
fn request(bootstrap: &Bootstrap, id: u64) -> wire::Request {
    wire::Request {
        protocol: 1,
        id,
        device_ids: bootstrap.device_ids,
        session: bootstrap.scope.session,
        registration: REG,
        profile_sha256: bootstrap.sha256().unwrap(),
        command: if id == 5 {
            Command::Close
        } else {
            Command::Forward {
                generation: id,
                token: bootstrap.prompt_tokens[id as usize - 1],
                cache_metadata: core::iter::once(id as u32 - 1).chain(0..144).collect(),
                rotary_bits: vec![0; 128],
            }
        },
    }
}
fn state<const N: usize>() -> [u32; N] {
    let tasks = N - 6;
    let mut value = [64; N];
    value[..6].copy_from_slice(&[
        1,
        0,
        (1 << tasks) - 1,
        (1 << tasks) - 1,
        (0..tasks).fold(0, |owners, i| owners | (1 << (2 * i))),
        0,
    ]);
    value
}
#[derive(Default)]
struct Fake {
    generations: Vec<u64>,
    closes: usize,
    fail: Option<u64>,
    close_fail: bool,
}
impl Backend for Fake {
    fn run(&mut self, input: &ForwardInput) -> io::Result<SmokeRun> {
        self.generations.push(input.generation);
        if self.fail == Some(input.generation) {
            return Err(io::Error::other("injected forward failure"));
        }
        Ok(SmokeRun {
            layer0: None,
            forward: ForwardRun {
                completion: ForwardCompletion {
                    generation: input.generation,
                    position: input.cache_metadata[0],
                    input_token: input.token,
                    output_token: 0,
                    embedding_ns: [1; 2],
                    tail_ns: [2; 3],
                    layers: (0..36)
                        .map(|_| LayerCompletion {
                            prefix_states: [state(); 2],
                            mlp_states: [state(); 2],
                            paired_ns: [[1; 2]; 4],
                        })
                        .collect(),
                },
                layer_hidden: vec![vec![0; 8192]; 36],
                final_normalized: vec![0; 8192],
                logits: vec![0; 303872],
            },
        })
    }
    fn close(&mut self) -> io::Result<()> {
        self.closes += 1;
        if self.close_fail {
            Err(io::Error::other("injected close failure"))
        } else {
            Ok(())
        }
    }
}
fn input_bytes(bootstrap: &Bootstrap, count: u64) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut budget = FrameBudget::new();
    for id in 1..=count {
        wire::write_request(&mut bytes, &mut budget, &request(bootstrap, id)).unwrap();
    }
    bytes
}

#[test]
fn exact_smoke_args_do_not_select_old_or_long_cli() {
    for capture in [false, true] {
        assert_eq!(
            parse_args(&args(capture)).unwrap(),
            NativeOptions {
                devices: [11, 22],
                timeout_ms: 1000,
                capture_layer0: capture
            }
        );
        assert!(crate::native_cli_v1::parse_args(&args(capture)).is_err());
        assert!(crate::native_long_cli_v1::parse_args(&args(capture)).is_err());
    }
    assert!(parse_args(&[]).is_err());
    for (i, v) in [
        (0, "--engineering-native-long-v1"),
        (1, "--missing"),
        (3, "11,11"),
        (5, "10001"),
        (6, "--capture-any-layer"),
    ] {
        let mut bad = args(true);
        bad[i] = v.into();
        assert!(parse_args(&bad).is_err());
    }
}

#[test]
fn bootstrap_capture_option_and_actual_pid_are_checked_before_begin() {
    for wrong_pid in [false, true] {
        let mut value = bootstrap(true);
        if wrong_pid {
            value.scope.child_identity += 1;
        }
        let mut bytes = Vec::new();
        wire::write_bootstrap(&mut bytes, &mut FrameBudget::new(), &value).unwrap();
        let result = prepare_native(
            &parse_args(&args(wrong_pid)).unwrap(),
            &mut bytes.as_slice(),
            &mut FrameBudget::new(),
        );
        assert!(result.err().unwrap().to_string().contains("bootstrap"));
    }
}

#[test]
fn healthy_protocol_runs_exact_four_then_acknowledges_actual_close() {
    let bootstrap = bootstrap(false);
    let input = input_bytes(&bootstrap, 5);
    let mut backend = Fake::default();
    let mut output = Vec::new();
    serve_forwards(
        &mut backend,
        &mut input.as_slice(),
        &mut output,
        &bootstrap,
        REG,
        &mut FrameBudget::new(),
    )
    .unwrap();
    assert_eq!(backend.generations, [1, 2, 3, 4]);
    assert_eq!(backend.closes, 1);
    let mut reader = output.as_slice();
    let mut budget = FrameBudget::new();
    let mut chain = Chain::new(REG, bootstrap.sha256().unwrap());
    for id in 1..=4 {
        let (response, control, payload, stage) = wire::read_response(&mut reader, &mut budget)
            .unwrap()
            .unwrap();
        assert_eq!(response.id, id);
        assert!(control.is_some());
        assert_eq!(payload.len(), OBSERVATION_BYTES);
        assert!(stage.is_empty());
        let Event::Completed(c) = response.event else {
            panic!("expected completion")
        };
        assert_eq!(chain.advance(&c), c.chain);
    }
    let (response, _, _, _) = wire::read_response(&mut reader, &mut budget)
        .unwrap()
        .unwrap();
    assert_eq!(
        response.event,
        Event::Closed {
            completed_forwards: 4,
            transcript_sha256: chain.digest()
        }
    );
    assert!(reader.is_empty());
}

#[test]
fn fourth_failure_eof_or_failed_close_never_claims_healthy_close_or_retries() {
    for kind in 0..3 {
        let bootstrap = bootstrap(false);
        let input = input_bytes(&bootstrap, if kind == 1 { 4 } else { 5 });
        let mut backend = Fake {
            fail: (kind == 0).then_some(4),
            close_fail: kind == 2,
            ..Fake::default()
        };
        let mut output = Vec::new();
        assert!(
            serve_forwards(
                &mut backend,
                &mut input.as_slice(),
                &mut output,
                &bootstrap,
                REG,
                &mut FrameBudget::new()
            )
            .is_err()
        );
        assert_eq!(backend.generations, [1, 2, 3, 4]);
        assert_eq!(backend.closes, usize::from(kind == 2));
        let mut reader = output.as_slice();
        let mut budget = FrameBudget::new();
        while let Some((response, _, _, _)) = wire::read_response(&mut reader, &mut budget).unwrap()
        {
            assert!(matches!(response.event, Event::Completed(_)));
            assert!(!response.native_closed);
        }
    }
}

#[test]
fn requested_but_missing_actual_layer_zero_capture_is_fatal_before_response() {
    let bootstrap = bootstrap(true);
    let input = input_bytes(&bootstrap, 1);
    let mut backend = Fake::default();
    let mut output = Vec::new();
    assert!(
        serve_forwards(
            &mut backend,
            &mut input.as_slice(),
            &mut output,
            &bootstrap,
            REG,
            &mut FrameBudget::new()
        )
        .is_err()
    );
    assert_eq!(backend.generations, [1]);
    assert_eq!(backend.closes, 0);
    assert!(output.is_empty());
}

#[test]
fn stage_serialization_uses_same_two_mib_bound_and_rejects_non_capture_json() {
    let mut sink = StageJson(Vec::new());
    sink.write_all(&vec![0; wire::STAGE_JSON_BYTES]).unwrap();
    assert!(sink.write_all(&[0]).is_err());
    assert_eq!(sink.0.len(), wire::STAGE_JSON_BYTES);
    assert!(serialize_stage(&serde_json::json!({"payload":[]})).is_err());
}
