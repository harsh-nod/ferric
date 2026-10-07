use super::*;
use crate::forward_sequence::ForwardCompletion;
use crate::resident_layer::LayerCompletion;

fn args() -> Vec<OsString> {
    [
        "--engineering-native",
        "--allow-unauthenticated-machine-code",
        "--devices",
        "11,22",
        "--input-mode",
        "teacher-forced",
        "--timeout-ms",
        "1000",
    ]
    .into_iter()
    .map(Into::into)
    .collect()
}
fn request(id: u64) -> wire::Request {
    let mut metadata = vec![id as u32 - 1];
    metadata.extend(0..144);
    wire::Request {
        protocol: wire::PROTOCOL,
        id,
        device_ids: [11, 22],
        session: [3; 32],
        registration: [4; 32],
        command: if id == 3 {
            Command::Close
        } else {
            Command::Forward {
                generation: id,
                token: 7,
                cache_metadata: metadata,
                rotary_bits: vec![0; 128],
            }
        },
    }
}
fn input(ids: &[u64]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for &id in ids {
        wire::write_request(&mut bytes, &request(id)).unwrap();
    }
    bytes
}
#[derive(Default)]
struct Fake {
    runs: usize,
    closes: usize,
    fail_run: bool,
    fail_close: bool,
    bad_shape: bool,
}
impl Backend for Fake {
    fn run(&mut self, input: &ForwardInput) -> io::Result<ForwardRun> {
        self.runs += 1;
        if self.fail_run {
            return Err(io::Error::other("injected run failure"));
        }
        Ok(ForwardRun {
            completion: ForwardCompletion {
                generation: input.generation,
                position: input.cache_metadata[0],
                input_token: input.token,
                output_token: 9,
                embedding_ns: [1, 2],
                tail_ns: [3, 4, 5],
                layers: (0..36)
                    .map(|_| LayerCompletion {
                        prefix_states: [[0; 22]; 2],
                        mlp_states: [[0; 11]; 2],
                        paired_ns: [[1; 2]; 4],
                    })
                    .collect(),
            },
            layer_hidden: vec![vec![0; 8192]; if self.bad_shape { 35 } else { 36 }],
            final_normalized: vec![0; 8192],
            logits: vec![0; wire::LOGIT_BYTES],
        })
    }
    fn close(&mut self) -> io::Result<()> {
        self.closes += 1;
        if self.fail_close {
            Err(io::Error::other("injected close failure"))
        } else {
            Ok(())
        }
    }
}

#[test]
fn arguments_require_opt_in_and_preserve_check_wire() {
    assert_eq!(
        parse_args(&["--check-wire".into()]).unwrap(),
        Invocation::CheckWire
    );
    assert!(matches!(
        parse_args(&args()).unwrap(),
        Invocation::Native(_)
    ));
    for index in [0, 1, 2, 4, 6] {
        let mut bad = args();
        bad[index] = "--unsupported".into();
        assert!(parse_args(&bad).is_err());
    }
    assert!(parse_args(&[]).is_err());
    let mut bad = args();
    bad.push("extra".into());
    assert!(parse_args(&bad).is_err());
    for devices in ["11", "11,11", "0,22", "11,22,33", "-1,22", "+1,22", "1, 2"] {
        let mut bad = args();
        bad[3] = devices.into();
        assert!(parse_args(&bad).is_err());
    }
    for timeout in ["0", "10001", "-1", "18446744073709551616"] {
        let mut bad = args();
        bad[7] = timeout.into();
        assert!(parse_args(&bad).is_err());
    }
    let mut good = args();
    good[5] = "autoregressive".into();
    assert!(parse_args(&good).is_ok());
}

#[test]
fn two_forwards_then_actual_close_produces_three_bounded_acknowledgements() {
    let mut fake = Fake::default();
    let mut output = Vec::new();
    serve_forwards(
        &mut fake,
        &mut input(&[1, 2, 3]).as_slice(),
        &mut output,
        [11, 22],
        [3; 32],
        [4; 32],
    )
    .unwrap();
    assert_eq!((fake.runs, fake.closes), (2, 1));
    let mut reader = output.as_slice();
    for id in 1..=2 {
        let (response, bytes) = wire::read_response(&mut reader).unwrap().unwrap();
        assert_eq!(response.id, id);
        assert!(!response.native_closed);
        assert_eq!(bytes.len(), wire::OBSERVATION_BYTES);
    }
    let (closed, bytes) = wire::read_response(&mut reader).unwrap().unwrap();
    assert!(closed.native_closed);
    assert!(bytes.is_empty());
    assert!(reader.is_empty());
}

#[test]
fn eof_never_implicitly_closes_or_retries() {
    for ids in [vec![], vec![1], vec![1, 2]] {
        let mut fake = Fake::default();
        assert!(
            serve_forwards(
                &mut fake,
                &mut input(&ids).as_slice(),
                &mut Vec::new(),
                [11, 22],
                [3; 32],
                [4; 32]
            )
            .is_err()
        );
        assert_eq!(fake.runs, ids.len());
        assert_eq!(fake.closes, 0);
    }
}

#[test]
fn wrong_sequence_session_devices_registration_stop_before_backend() {
    for index in 0..5 {
        let mut req = request(if index == 0 {
            3
        } else if index == 4 {
            2
        } else {
            1
        });
        match index {
            1 => req.device_ids.swap(0, 1),
            2 => req.session = [7; 32],
            3 => req.registration = [8; 32],
            _ => {}
        }
        let mut bytes = Vec::new();
        wire::write_request(&mut bytes, &req).unwrap();
        let mut fake = Fake::default();
        assert!(
            serve_forwards(
                &mut fake,
                &mut bytes.as_slice(),
                &mut Vec::new(),
                [11, 22],
                [3; 32],
                [4; 32]
            )
            .is_err()
        );
        assert_eq!((fake.runs, fake.closes), (0, 0));
    }
}

#[test]
fn native_failure_and_malformed_observation_publish_nothing() {
    for bad_shape in [false, true] {
        let mut fake = Fake {
            fail_run: !bad_shape,
            bad_shape,
            ..Default::default()
        };
        let mut output = Vec::new();
        assert!(
            serve_forwards(
                &mut fake,
                &mut input(&[1, 2, 3]).as_slice(),
                &mut output,
                [11, 22],
                [3; 32],
                [4; 32]
            )
            .is_err()
        );
        assert_eq!((fake.runs, fake.closes), (1, 0));
        assert!(output.is_empty());
    }
}

struct FailingWriter {
    calls: usize,
    fail_on: usize,
}
impl Write for FailingWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.calls += 1;
        if self.calls == self.fail_on {
            Err(io::Error::other("injected lost pipe"))
        } else {
            Ok(bytes.len())
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn response_transport_loss_after_commit_is_fatal_no_retry_or_close() {
    let mut fake = Fake::default();
    let mut writer = FailingWriter {
        calls: 0,
        fail_on: 1,
    };
    assert!(
        serve_forwards(
            &mut fake,
            &mut input(&[1, 2, 3]).as_slice(),
            &mut writer,
            [11, 22],
            [3; 32],
            [4; 32]
        )
        .is_err()
    );
    assert_eq!((fake.runs, fake.closes), (1, 0));
}

#[test]
fn failed_close_or_close_ack_is_not_success() {
    let mut fake = Fake {
        fail_close: true,
        ..Default::default()
    };
    let mut output = Vec::new();
    assert!(
        serve_forwards(
            &mut fake,
            &mut input(&[1, 2, 3]).as_slice(),
            &mut output,
            [11, 22],
            [3; 32],
            [4; 32]
        )
        .is_err()
    );
    assert_eq!((fake.runs, fake.closes), (2, 1));
    let mut reader = output.as_slice();
    assert!(wire::read_response(&mut reader).unwrap().is_some());
    assert!(wire::read_response(&mut reader).unwrap().is_some());
    assert!(reader.is_empty());
    // Each completion writes prefix, header, payload. Reject the close prefix.
    let mut fake = Fake::default();
    let mut writer = FailingWriter {
        calls: 0,
        fail_on: 7,
    };
    assert!(
        serve_forwards(
            &mut fake,
            &mut input(&[1, 2, 3]).as_slice(),
            &mut writer,
            [11, 22],
            [3; 32],
            [4; 32]
        )
        .is_err()
    );
    assert_eq!((fake.runs, fake.closes), (2, 1));
}

#[test]
fn preparation_rejects_missing_begin_or_wrong_actual_pid_without_native_open() {
    let Invocation::Native(options) = parse_args(&args()).unwrap() else {
        unreachable!()
    };
    assert!(prepare_native(&options, &mut [].as_slice()).is_err());
    let mut bootstrap = wire::Bootstrap {
        protocol: wire::PROTOCOL,
        device_ids: [11, 22],
        mode: InputMode::TeacherForced,
        timeout_ms: 1000,
        scope: setup::Scope {
            bundle_id: [1; 32],
            model_id: [2; 32],
            session: [3; 32],
            pool_identity: 1,
            group_id: 0,
            child_identity: std::process::id(),
        },
    };
    let mut bytes = Vec::new();
    wire::write_bootstrap(&mut bytes, &bootstrap).unwrap();
    assert!(prepare_native(&options, &mut bytes.as_slice()).is_err());
    bootstrap.scope.child_identity = std::process::id().checked_add(1).unwrap();
    let mut bytes = Vec::new();
    wire::write_bootstrap(&mut bytes, &bootstrap).unwrap();
    assert!(prepare_native(&options, &mut bytes.as_slice()).is_err());
}
