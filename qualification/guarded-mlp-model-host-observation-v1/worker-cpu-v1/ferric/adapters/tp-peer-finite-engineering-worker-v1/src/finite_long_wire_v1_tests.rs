use super::*;

fn bootstrap() -> Bootstrap {
    Bootstrap {
        protocol: 1,
        profile: Profile::Prompt2048Output256V1,
        device_ids: [11, 22],
        timeout_ms: 1000,
        scope: Scope {
            bundle_id: [1; 32],
            model_id: [2; 32],
            session: [3; 32],
            pool_identity: 1,
            group_id: 0,
            child_identity: 42,
        },
        prompt_tokens: vec![7; 2048],
    }
}
fn state<const N: usize>() -> [u32; N] {
    let tasks = N - 6;
    let mut result = [64; N];
    result[..6].copy_from_slice(&[
        1,
        0,
        (1 << tasks) - 1,
        (1 << tasks) - 1,
        (0..tasks).fold(0, |owners, i| owners | (1 << (2 * i))),
        0,
    ]);
    result
}
fn control() -> Control {
    Control {
        embedding_ns: [u64::MAX; 2],
        tail_ns: [u64::MAX; 3],
        layers: core::array::from_fn(|_| LayerObservation {
            prefix_states: [state(); 2],
            mlp_states: [state(); 2],
            paired_ns: [[u64::MAX; 2]; 4],
        }),
    }
}
fn request(id: u64) -> Request {
    Request {
        protocol: PROTOCOL,
        id,
        device_ids: [11, 22],
        session: [3; 32],
        registration: [4; 32],
        profile_sha256: [5; 32],
        command: if id == 2304 {
            Command::Close
        } else {
            Command::Forward {
                generation: id,
                token: 7,
                cache_metadata: core::iter::once(id as u32 - 1).chain(0..144).collect(),
                rotary_bits: vec![0; 128],
            }
        },
    }
}
fn response(position: u32, bytes: &[u8]) -> (Response, Control) {
    let control = control();
    let completion = Completion {
        generation: u64::from(position) + 1,
        position,
        input_token: 151_935,
        output_token: 151_935,
        control: part(&control.encode()),
        observation: part(bytes),
        capture: capture_position(position).then(|| Payload::from_bytes(bytes).unwrap()),
        chain: [255; 32],
    };
    (
        Response {
            protocol: 1,
            id: u64::from(position) + 1,
            device_ids: [u64::MAX, u64::MAX - 1],
            session: [255; 32],
            registration: [255; 32],
            profile_sha256: [255; 32],
            event: Event::Completed(completion),
            native_closed: false,
            gpu_execution: true,
            numerical_acceptance: false,
            performance_claim: false,
            production_authority: false,
        },
        control,
    )
}

#[test]
fn bootstrap_is_distinct_closed_profile_with_actual_pid_and_prompt_digest() {
    let good = bootstrap();
    good.validate([11, 22], 1000, 42).unwrap();
    assert_eq!(good.scope.group_id, 0);
    let hash = good.sha256().unwrap();
    let mut changed = good.clone();
    changed.prompt_tokens[2047] = 8;
    assert_ne!(changed.sha256().unwrap(), hash);
    assert!(good.validate([22, 11], 1000, 42).is_err());
    assert!(good.validate([11, 22], 1000, 43).is_err());
    for mutation in 0..5 {
        let mut bad = good.clone();
        match mutation {
            0 => {
                bad.prompt_tokens.pop();
            }
            1 => bad.prompt_tokens.push(7),
            2 => bad.prompt_tokens[0] = 151_936,
            3 => bad.protocol = 2,
            _ => bad.timeout_ms = 10_001,
        }
        assert!(bad.sha256().is_err());
    }
    let mut output = Vec::new();
    write_bootstrap(&mut output, &mut FrameBudget::new(), &good).unwrap();
    assert_eq!(
        read_bootstrap(&mut output.as_slice(), &mut FrameBudget::new()).unwrap(),
        Some(good.clone())
    );
    assert!(
        serde_json::from_slice::<crate::finite_forward_wire_v1::Bootstrap>(
            &header_bytes(&good).unwrap()
        )
        .is_err()
    );
}

#[test]
fn control_binary_roundtrip_retains_every_word_without_decimal_expansion() {
    let mut value = control();
    value.layers[35].prefix_states[1][4] = 0xaaaa_aaaa;
    value.layers[35].mlp_states[1][4] = 0x2aa;
    let bytes = value.encode();
    assert_eq!(bytes.len(), 11_848);
    assert_eq!(Control::decode(&bytes).unwrap(), value);
    assert!(Control::decode(&bytes[..CONTROL_BYTES - 1]).is_err());
    for word in 0..22 {
        let mut bad = value.clone();
        bad.layers[35].prefix_states[1][word] ^= 1;
        assert!(bad.validate().is_err());
    }
    for word in 0..11 {
        let mut bad = value.clone();
        bad.layers[35].mlp_states[1][word] ^= 1;
        assert!(bad.validate().is_err());
    }
    let mut bad = value;
    bad.layers[0].mlp_states[0][4] |= 1 << 10;
    assert!(bad.validate().is_err());
}

#[test]
fn all_request_boundaries_permutation_and_finite_rotary_are_checked() {
    for id in [1, 2048, 2049, 2303, 2304] {
        request(id).validate().unwrap();
    }
    for mutation in 0..8 {
        let mut bad = request(2303);
        let Command::Forward {
            generation,
            token,
            cache_metadata,
            rotary_bits,
        } = &mut bad.command
        else {
            unreachable!()
        };
        match mutation {
            0 => *generation = 2304,
            1 => *token = 151_936,
            2 => cache_metadata[0] = 2303,
            3 => cache_metadata[1] = 144,
            4 => cache_metadata[1] = cache_metadata[2],
            5 => rotary_bits[0] = f32::NAN.to_bits(),
            6 => {
                rotary_bits.pop();
            }
            _ => bad.profile_sha256 = [0; 32],
        }
        assert!(bad.validate().is_err());
    }
    let mut bad = request(1);
    bad.command = Command::Close;
    assert!(bad.validate().is_err());
}

#[test]
fn selected_and_uncaptured_frames_verify_body_and_exact_schedule() {
    let bytes = vec![0; OBSERVATION_BYTES];
    for position in [0, 1, 2047, 2048, 2302] {
        let (response, control) = response(position, &bytes);
        let payload = if capture_position(position) {
            bytes.as_slice()
        } else {
            &[]
        };
        let mut encoded = Vec::new();
        let mut write = FrameBudget::new();
        write_response(&mut encoded, &mut write, &response, Some(&control), payload).unwrap();
        let mut read = FrameBudget::new();
        let (decoded, decoded_control, decoded_payload) =
            read_response(&mut encoded.as_slice(), &mut read)
                .unwrap()
                .unwrap();
        assert_eq!(response, decoded);
        assert_eq!(decoded_control, Some(control));
        assert_eq!(decoded_payload, payload);
        assert_eq!(read.used(), write.used());
        *encoded.last_mut().unwrap() ^= 1;
        assert!(read_response(&mut encoded.as_slice(), &mut FrameBudget::new()).is_err());
    }
    let (mut bad, _) = response(1, &bytes);
    let Event::Completed(c) = &mut bad.event else {
        unreachable!()
    };
    c.capture = Some(Payload::from_bytes(&bytes).unwrap());
    assert!(bad.body_bytes().is_err());
}

#[test]
fn chain_binds_each_checked_observation_and_ignores_claimed_chain() {
    let bytes = vec![0; OBSERVATION_BYTES];
    let (r, _) = response(0, &bytes);
    let Event::Completed(c) = r.event else {
        unreachable!()
    };
    let mut first = Chain::new([1; 32], [2; 32]);
    let actual = first.advance(&c);
    let mut other = c.clone();
    other.chain = [3; 32];
    assert_eq!(Chain::new([1; 32], [2; 32]).advance(&other), actual);
    other.output_token -= 1;
    assert_ne!(Chain::new([1; 32], [2; 32]).advance(&other), actual);
    assert_ne!(Chain::new([1; 32], [9; 32]).advance(&c), actual);
}

#[test]
fn cumulative_budget_exact_one_short_overflow_and_malformed_frames_refuse() {
    let mut exact = FrameBudget {
        used: STREAM_BYTES - 1,
    };
    exact.charge(1).unwrap();
    assert!(exact.charge(1).is_err());
    assert!(exact.charge(0).is_err());
    let mut overflow = FrameBudget { used: usize::MAX };
    assert!(overflow.charge(1).is_err());
    for bytes in [
        vec![1],
        vec![0; 4],
        ((MAX_HEADER as u32) + 1).to_le_bytes().to_vec(),
    ] {
        assert!(read_request(&mut bytes.as_slice(), &mut FrameBudget::new()).is_err());
    }
    let mut value = serde_json::to_value(request(1)).unwrap();
    value["unexpected"] = 1.into();
    let mut bytes = Vec::new();
    write_header(&mut bytes, &mut FrameBudget::new(), &value).unwrap();
    assert!(read_request(&mut bytes.as_slice(), &mut FrameBudget::new()).is_err());
    let duplicate = String::from_utf8(header_bytes(&request(1)).unwrap())
        .unwrap()
        .replacen('{', "{\"protocol\":1,", 1);
    let mut bytes = (duplicate.len() as u32).to_le_bytes().to_vec();
    bytes.extend_from_slice(duplicate.as_bytes());
    assert!(read_request(&mut bytes.as_slice(), &mut FrameBudget::new()).is_err());
}

#[test]
fn full_2303_frame_shape_fits_existing_64mib_without_retaining_transcript() {
    struct Counter(usize);
    impl Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0 += bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let bytes = vec![0; OBSERVATION_BYTES];
    let mut sink = Counter(0);
    let mut budget = FrameBudget::new();
    let (mut response, control) = response(0, &bytes);
    let captured = Payload::from_bytes(&bytes).unwrap();
    for position in 0..FORWARDS {
        response.id = u64::from(position) + 1;
        let Event::Completed(c) = &mut response.event else {
            unreachable!()
        };
        c.generation = response.id;
        c.position = position;
        c.capture = capture_position(position).then(|| captured.clone());
        write_response(
            &mut sink,
            &mut budget,
            &response,
            Some(&control),
            if capture_position(position) {
                &bytes
            } else {
                &[]
            },
        )
        .unwrap();
    }
    assert_eq!(sink.0, budget.used());
    assert!(sink.0 < STREAM_BYTES);
    assert_eq!(
        CONTROL_BYTES * FORWARDS as usize + OBSERVATION_BYTES * CAPTURE_POSITIONS.len(),
        29_713_848
    );
}

#[test]
fn healthy_close_is_payload_free_and_has_no_numerical_or_performance_claim() {
    let (mut value, _) = response(0, &vec![0; OBSERVATION_BYTES]);
    value.id = 2304;
    value.native_closed = true;
    value.event = Event::Closed {
        completed_forwards: FORWARDS,
        transcript_sha256: [3; 32],
    };
    let mut bytes = Vec::new();
    write_response(&mut bytes, &mut FrameBudget::new(), &value, None, &[]).unwrap();
    assert_eq!(
        read_response(&mut bytes.as_slice(), &mut FrameBudget::new()).unwrap(),
        Some((value.clone(), None, vec![]))
    );
    value.performance_claim = true;
    assert!(value.body_bytes().is_err());
}
