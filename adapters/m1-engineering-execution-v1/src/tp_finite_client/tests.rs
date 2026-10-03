use super::*;

fn config() -> Config {
    let pin = FilePin {
        path: "/task/image".into(),
        bytes: 1,
        sha256: [1; 32],
    };
    Config {
        schema: "FerricFiniteTwoForwardRequestV1".into(),
        source: "/task/model".into(),
        worker: pin.clone(),
        images: ImagePins {
            prefix: pin.clone(),
            mlp: pin.clone(),
            residual: pin.clone(),
            tail: pin,
        },
        expected_bundle_id: [1; 32],
        expected_model_id: [2; 32],
        device_ids: [7, 9],
        session: [3; 32],
        mode: forward::InputMode::TeacherForced,
        input_tokens: vec![785, 6722],
        dispatch_timeout_ms: 10_000,
        child_deadline_ms: 60_000,
    }
}
fn request() -> forward::Request {
    forward::Request {
        protocol: forward::PROTOCOL,
        id: 1,
        device_ids: [7, 9],
        session: [3; 32],
        registration: [4; 32],
        command: forward::Command::Forward {
            generation: 1,
            token: 785,
            cache_metadata: std::iter::once(0).chain(0..144).collect(),
            rotary_bits: vec![0; 128],
        },
    }
}
fn prefix() -> [u32; 22] {
    let mut words = [64; 22];
    words[..6].copy_from_slice(&[1, 0, 0xffff, 0xffff, 0x55555555, 0]);
    words
}
fn mlp() -> [u32; 11] {
    let mut words = [64; 11];
    words[..6].copy_from_slice(&[1, 0, 31, 31, 0x155, 0]);
    words
}
fn response(payload: &[u8]) -> forward::Response {
    forward::Response {
        protocol: forward::PROTOCOL,
        id: 1,
        device_ids: [7, 9],
        session: [3; 32],
        registration: [4; 32],
        event: forward::Event::Completed(forward::Completion {
            generation: 1,
            position: 0,
            input_token: 785,
            output_token: 42,
            embedding_ns: [1, 2],
            layers: vec![
                forward::LayerObservation {
                    prefix_states: [prefix(); 2],
                    mlp_states: [mlp(); 2],
                    paired_ns: [[1, 2]; 4]
                };
                36
            ],
            tail_ns: [1, 2, 3],
            payload: forward::Payload::from_bytes(payload).unwrap(),
        }),
        native_closed: false,
        gpu_execution: true,
        numerical_acceptance: false,
        performance_claim: false,
        production_authority: false,
    }
}

#[test]
fn closed_configuration_rejects_mode_schema_device_and_resource_drift() {
    let original = config();
    original.validate().unwrap();
    Config::parse(&serde_json::to_vec(&original).unwrap()).unwrap();
    for mutation in 0..8 {
        let mut value = original.clone();
        match mutation {
            0 => value.schema.push('x'),
            1 => value.device_ids[1] = value.device_ids[0],
            2 => value.session = [0; 32],
            3 => value.input_tokens.pop().map(|_| ()).unwrap(),
            4 => value.input_tokens[1] = 151_936,
            5 => value.dispatch_timeout_ms = 10_001,
            6 => value.child_deadline_ms = 3_600_001,
            7 => value.expected_model_id = [0; 32],
            _ => unreachable!(),
        }
        assert!(value.validate().is_err());
    }
    let mut value = serde_json::to_value(original).unwrap();
    value["fallback"] = serde_json::json!(true);
    assert!(Config::parse(&serde_json::to_vec(&value).unwrap()).is_err());
    assert!(Config::parse(&vec![0; 65_537]).is_err());
}

#[test]
fn autoregressive_uses_actual_first_output_while_teacher_forcing_is_explicit() {
    let mut value = config();
    value.mode = forward::InputMode::Autoregressive;
    assert!(value.validate().is_err());
    value.input_tokens.truncate(1);
    value.validate().unwrap();
    assert_eq!(
        second_token(value.mode, &value.input_tokens, 42).unwrap(),
        42
    );
    assert_eq!(
        second_token(forward::InputMode::TeacherForced, &[785, 6722], 42).unwrap(),
        6722
    );
    assert!(second_token(value.mode, &value.input_tokens, 151_936).is_err());
    assert!(second_token(forward::InputMode::TeacherForced, &[785], 42).is_err());
}

#[test]
fn complete_reply_requires_every_layer_rank_and_exact_scope_before_pool_commit() {
    let payload = vec![0; forward::OBSERVATION_BYTES];
    let request = request();
    let original = response(&payload);
    assert_eq!(
        validate_completion(&request, &original, &payload).unwrap(),
        42
    );
    for mutation in 0..11 {
        let mut changed = original.clone();
        match mutation {
            0 => changed.device_ids.swap(0, 1),
            1 => changed.session[0] ^= 1,
            2 => changed.registration[0] ^= 1,
            3 => changed.numerical_acceptance = true,
            4 => changed.performance_claim = true,
            5 => changed.production_authority = true,
            6 => changed.native_closed = true,
            7 => changed.gpu_execution = false,
            8 => {
                if let forward::Event::Completed(c) = &mut changed.event {
                    c.layers.pop();
                }
            }
            9 => {
                if let forward::Event::Completed(c) = &mut changed.event {
                    c.input_token += 1;
                }
            }
            10 => {
                if let forward::Event::Completed(c) = &mut changed.event {
                    c.output_token = 151_936;
                }
            }
            _ => unreachable!(),
        }
        assert!(
            validate_completion(&request, &changed, &payload).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn state_error_arrival_owner_or_high_unused_mlp_bits_refuse_completion() {
    let payload = vec![0; forward::OBSERVATION_BYTES];
    for mutation in 0..7 {
        let mut changed = response(&payload);
        let forward::Event::Completed(c) = &mut changed.event else {
            unreachable!()
        };
        match mutation {
            0 => c.layers[35].prefix_states[1][5] = 1,
            1 => c.layers[35].prefix_states[1][21] = 63,
            2 => c.layers[0].prefix_states[0][4] |= 3,
            3 => c.layers[0].mlp_states[0][4] |= 1 << 10,
            4 => c.layers[0].mlp_states[1][3] = 15,
            5 => c.layers[0].mlp_states[1][0] = 2,
            6 => c.layers[0].mlp_states[1][4] &= !3,
            _ => unreachable!(),
        }
        assert!(validate_completion(&request(), &changed, &payload).is_err());
    }
}

#[test]
fn payload_mutation_and_nonfinite_words_never_become_numerical_success() {
    let mut payload = vec![0; forward::OBSERVATION_BYTES];
    let mut value = response(&payload);
    payload[0] = 1;
    assert!(validate_completion(&request(), &value, &payload).is_err());
    payload[..2].copy_from_slice(&0x7fc0_u16.to_le_bytes());
    let forward::Event::Completed(c) = &mut value.event else {
        unreachable!()
    };
    c.payload = forward::Payload::from_bytes(&payload).unwrap();
    assert!(validate_completion(&request(), &value, &payload).is_err());
}

#[test]
fn close_requires_actual_two_forward_closed_reply_not_eof_or_completion() {
    let mut request = request();
    request.id = 3;
    request.command = forward::Command::Close;
    let mut value = response(&vec![0; forward::OBSERVATION_BYTES]);
    value.id = 3;
    value.event = forward::Event::Closed {
        completed_forwards: 2,
    };
    value.native_closed = true;
    validate_close(&request, &value, &[]).unwrap();
    value.native_closed = false;
    assert!(validate_close(&request, &value, &[]).is_err());
    value.native_closed = true;
    value.event = forward::Event::Closed {
        completed_forwards: 1,
    };
    assert!(validate_close(&request, &value, &[]).is_err());
    value.event = forward::Event::Closed {
        completed_forwards: 2,
    };
    assert!(validate_close(&request, &value, &[0]).is_err());
}

#[test]
fn finite_metadata_reuses_real_rope_and_committed_pool_history() {
    let scope = EngineeringTpPoolScopeV1 {
        model: [2; 32],
        session: [3; 32],
    };
    let limits = EngineeringTpPagedLimitsV1::new(2304, 32, 144, 1024).unwrap();
    let mut pool = EngineeringTpPagedPoolV1::new(scope, limits).unwrap();
    let sequence = pool.open_sequence(scope, &[785], 0).unwrap().sequence();
    let first = pool
        .reserve_batch(&[EngineeringTpPageRowV1 {
            sequence,
            token: 785,
            position: 0,
        }])
        .unwrap();
    let m0 = metadata(&first, [4; 32], 1_000_000).unwrap();
    assert_eq!(&m0.rotary()[..64], &[1.0; 64]);
    assert_eq!(&m0.rotary()[64..], &[0.0; 64]);
    assert!(
        pool.reserve_batch(&[EngineeringTpPageRowV1 {
            sequence,
            token: 42,
            position: 1
        }])
        .is_err()
    );
    // CPU fixture completion only. The production path can mint this solely
    // after validate_completion accepted the actual child observation.
    pool.begin_submission(&first).unwrap();
    pool.commit_batch(
        &first,
        EngineeringTpBatchCompletionV1::after_all_ranks(&first),
    )
    .unwrap();
    let second = pool
        .reserve_batch(&[EngineeringTpPageRowV1 {
            sequence,
            token: 42,
            position: 1,
        }])
        .unwrap();
    let m1 = metadata(&second, [4; 32], 1_000_000).unwrap();
    assert_eq!(&m0.cache_metadata()[1..], &m1.cache_metadata()[1..]);
    assert_eq!(m1.cache_metadata()[0], 1);
    assert!(m1.rotary()[64..].iter().all(|v| *v > 0.0));
    assert!(metadata(&second, [4; 32], 10_000).is_err());
}

#[test]
fn absent_opt_in_refuses_before_any_files_process_or_model() {
    assert!(run(config(), false).err().unwrap().contains("opt-in"));
}

#[test]
fn file_pins_reject_wrong_bytes_hash_and_symlink_without_rewriting_inputs() {
    use std::io::Write;
    let path = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("finite-client-pin-{}", std::process::id()));
    let link = path.with_extension("link");
    struct Remove(PathBuf, PathBuf);
    impl Drop for Remove {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.1);
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    let _remove = Remove(path.clone(), link.clone());
    file.write_all(&[1, 2, 3]).unwrap();
    drop(file);
    let pin = FilePin {
        path,
        bytes: 3,
        sha256: hash(&[1, 2, 3]),
    };
    assert_eq!(pin.read(3, true).unwrap(), [1, 2, 3]);
    let mut bad = pin.clone();
    bad.sha256[0] ^= 1;
    assert!(bad.read(3, true).is_err());
    bad = pin.clone();
    bad.bytes = 2;
    assert!(bad.read(3, true).is_err());
    std::os::unix::fs::symlink(&pin.path, &link).unwrap();
    bad = pin.clone();
    bad.path = link;
    assert!(bad.read(3, true).is_err());
    assert_eq!(std::fs::read(&pin.path).unwrap(), [1, 2, 3]);
}
