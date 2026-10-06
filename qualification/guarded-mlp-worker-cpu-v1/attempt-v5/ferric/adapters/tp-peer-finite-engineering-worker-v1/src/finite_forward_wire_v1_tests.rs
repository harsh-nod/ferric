use super::*;

fn bootstrap() -> Bootstrap {
    Bootstrap {
        protocol: PROTOCOL,
        device_ids: [11, 22],
        mode: InputMode::TeacherForced,
        timeout_ms: 1000,
        scope: Scope {
            bundle_id: [1; 32],
            model_id: [2; 32],
            session: [3; 32],
            pool_identity: 1,
            group_id: 0,
            child_identity: 42,
        },
    }
}
fn request() -> Request {
    let mut metadata = vec![0];
    metadata.extend(0..144);
    Request {
        protocol: PROTOCOL,
        id: 1,
        device_ids: [11, 22],
        session: [3; 32],
        registration: [4; 32],
        command: Command::Forward {
            generation: 1,
            token: 7,
            cache_metadata: metadata,
            rotary_bits: vec![0; 128],
        },
    }
}
fn encoded(value: &impl Serialize) -> Vec<u8> {
    let mut bytes = Vec::new();
    write_header(&mut bytes, value).unwrap();
    bytes
}
fn response() -> (Response, Vec<u8>) {
    let bytes = vec![0; OBSERVATION_BYTES];
    let c = Completion {
        generation: 1,
        position: 0,
        input_token: 7,
        output_token: 9,
        embedding_ns: [1, 2],
        tail_ns: [3, 4, 5],
        layers: (0..LAYERS)
            .map(|_| LayerObservation {
                prefix_states: [[0; 22]; 2],
                mlp_states: [[0; 11]; 2],
                paired_ns: [[0; 2]; 4],
            })
            .collect(),
        payload: Payload::from_bytes(&bytes).unwrap(),
    };
    (
        Response {
            protocol: PROTOCOL,
            id: 1,
            device_ids: [11, 22],
            session: [3; 32],
            registration: [4; 32],
            event: Event::Completed(c),
            native_closed: false,
            gpu_execution: true,
            numerical_acceptance: false,
            performance_claim: false,
            production_authority: false,
        },
        bytes,
    )
}

#[test]
fn bootstrap_binds_actual_pid_devices_scope_mode_and_timeout() {
    let good = bootstrap();
    good.validate([11, 22], InputMode::TeacherForced, 1000, 42)
        .unwrap();
    assert_eq!(good.scope.group_id, 0);
    assert!(good.validate([22, 11], good.mode, 1000, 42).is_err());
    assert!(
        good.validate([11, 22], InputMode::Autoregressive, 1000, 42)
            .is_err()
    );
    assert!(good.validate([11, 22], good.mode, 999, 42).is_err());
    assert!(good.validate([11, 22], good.mode, 1000, 43).is_err());
    for index in 0..7 {
        let mut bad = good.clone();
        match index {
            0 => bad.scope.bundle_id = [0; 32],
            1 => bad.scope.model_id = [0; 32],
            2 => bad.scope.session = [0; 32],
            3 => bad.scope.pool_identity = 0,
            4 => bad.scope.child_identity = 0,
            5 => bad.protocol = 2,
            _ => bad.device_ids = [11, 11],
        }
        assert!(bad.validate([11, 22], good.mode, 1000, 42).is_err());
    }
    let mut bytes = Vec::new();
    write_bootstrap(&mut bytes, &good).unwrap();
    assert_eq!(read_bootstrap(&mut bytes.as_slice()).unwrap(), Some(good));
}

#[test]
fn requests_have_closed_exact_array_bounds_and_permutation() {
    let good = request();
    good.validate().unwrap();
    let mut bytes = Vec::new();
    write_request(&mut bytes, &good).unwrap();
    assert_eq!(
        read_request(&mut bytes.as_slice()).unwrap(),
        Some(good.clone())
    );
    for index in 0..9 {
        let mut bad = good.clone();
        let Command::Forward {
            generation,
            token,
            cache_metadata,
            rotary_bits,
        } = &mut bad.command
        else {
            unreachable!()
        };
        match index {
            0 => {
                cache_metadata.pop();
            }
            1 => cache_metadata.push(0),
            2 => rotary_bits.push(0),
            3 => {
                rotary_bits.pop();
            }
            4 => cache_metadata[1] = 144,
            5 => cache_metadata[2] = cache_metadata[1],
            6 => rotary_bits[0] = f32::INFINITY.to_bits(),
            7 => *generation = 2,
            _ => *token = 151_936,
        }
        assert!(bad.validate().is_err());
        assert!(read_request(&mut encoded(&bad).as_slice()).is_err());
    }
}

#[test]
fn unknown_duplicate_fields_and_truncated_or_oversized_frames_reject() {
    let mut value = serde_json::to_value(request()).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("ready".into(), serde_json::json!(true));
    assert!(read_request(&mut encoded(&value).as_slice()).is_err());
    let text = serde_json::to_string(&request()).unwrap();
    let text = text.replacen("\"protocol\":1", "\"protocol\":1,\"protocol\":1", 1);
    let mut duplicate = (text.len() as u32).to_le_bytes().to_vec();
    duplicate.extend(text.bytes());
    assert!(read_request(&mut duplicate.as_slice()).is_err());
    assert!(read_request(&mut [].as_slice()).unwrap().is_none());
    assert!(read_request(&mut [1_u8, 0].as_slice()).is_err());
    assert!(read_request(&mut ((MAX_HEADER + 1) as u32).to_le_bytes().as_slice()).is_err());
    let mut short = encoded(&request());
    short.pop();
    assert!(read_request(&mut short.as_slice()).is_err());
}

#[test]
fn response_roundtrip_binds_every_payload_and_forbids_authority_claims() {
    let (response, payload) = response();
    let mut bytes = Vec::new();
    write_response(&mut bytes, &response, &payload).unwrap();
    assert_eq!(
        read_response(&mut bytes.as_slice()).unwrap(),
        Some((response.clone(), payload.clone()))
    );
    for offset in [0, ROW_BYTES, LAYERS * ROW_BYTES, OBSERVATION_BYTES - 1] {
        let mut changed = payload.clone();
        changed[offset] ^= 1;
        assert!(write_response(&mut Vec::new(), &response, &changed).is_err());
    }
    let mut corrupted = bytes.clone();
    *corrupted.last_mut().unwrap() ^= 1;
    assert!(read_response(&mut corrupted.as_slice()).is_err());
    for index in 0..5 {
        let mut bad = response.clone();
        match index {
            0 => bad.production_authority = true,
            1 => bad.numerical_acceptance = true,
            2 => bad.performance_claim = true,
            3 => bad.native_closed = true,
            _ => bad.gpu_execution = false,
        }
        assert!(write_response(&mut Vec::new(), &bad, &payload).is_err());
    }
    let mut short = bytes;
    short.pop();
    assert!(read_response(&mut short.as_slice()).is_err());
}

#[test]
fn response_rosters_and_close_are_exactly_bounded() {
    let (response, payload) = response();
    for index in 0..4 {
        let mut bad = response.clone();
        let Event::Completed(c) = &mut bad.event else {
            unreachable!()
        };
        match index {
            0 => {
                c.layers.pop();
            }
            1 => {
                c.payload.layer_hidden.pop();
            }
            2 => c.payload.total.bytes -= 1,
            _ => c.output_token = 151_936,
        }
        assert!(write_response(&mut Vec::new(), &bad, &payload).is_err());
    }
    let mut closed = response;
    closed.id = 3;
    closed.native_closed = true;
    closed.event = Event::Closed {
        completed_forwards: 2,
    };
    let mut bytes = Vec::new();
    write_response(&mut bytes, &closed, &[]).unwrap();
    assert_eq!(
        read_response(&mut bytes.as_slice()).unwrap(),
        Some((closed.clone(), vec![]))
    );
    closed.event = Event::Closed {
        completed_forwards: 1,
    };
    assert!(write_response(&mut Vec::new(), &closed, &[]).is_err());
}
