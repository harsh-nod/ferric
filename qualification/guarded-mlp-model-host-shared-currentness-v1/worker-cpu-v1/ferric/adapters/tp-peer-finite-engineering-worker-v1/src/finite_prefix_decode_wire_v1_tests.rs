use super::*;

pub(crate) fn bootstrap(mode: InputMode) -> Bootstrap {
    let scope = Scope {
        bundle_id: [1; 32],
        model_id: [2; 32],
        session: [3; 32],
        pool_identity: 4,
        group_id: 5,
        child_identity: std::process::id(),
    };
    let p = part(&[7]);
    Bootstrap {
        protocol: PROTOCOL,
        profile: Profile::Prefix284Mlp548FourForwardV1,
        device_ids: [11, 12],
        scope: scope.clone(),
        registration: p.sha256,
        begin: Begin {
            scope,
            registration: p,
            source_program: p,
            uploads: p,
            prefix_image: p,
            mlp_image: p,
            residual_image: p,
            tail_image: Some(p),
        },
        timeout_ms: 100,
        mode,
        input_tokens: if mode == InputMode::TeacherForced {
            vec![9112, 2190, 3772, 220]
        } else {
            vec![9112]
        },
        tiles_image: part(&[8]),
        prefix_image: part(&[9]),
    }
}
pub(crate) fn request(b: &Bootstrap, position: u32, previous: Option<u32>) -> Request {
    Request {
        protocol: PROTOCOL,
        id: position as u64 + 1,
        device_ids: b.device_ids,
        session: b.scope.session,
        registration: b.registration,
        profile_sha256: b.sha256().unwrap(),
        command: Command::Forward {
            generation: position as u64 + 1,
            token: b.input(position, previous).unwrap(),
            cache_metadata: std::iter::once(position).chain(0..144).collect(),
            rotary_bits: vec![0; 128],
        },
    }
}
pub(crate) fn control() -> Control {
    let mut p = [64; 284];
    p[..4].copy_from_slice(&[1, 0, 0, 31]);
    p[4..9].copy_from_slice(&[1, 48, 1, 16, 64]);
    p[9..14].copy_from_slice(&[1, 48, 1, 16, 64]);
    p[14..19].copy_from_slice(&[u32::MAX, u32::MAX, u32::MAX, u32::MAX, 3]);
    p[19..24].copy_from_slice(&[u32::MAX, u32::MAX, u32::MAX, u32::MAX, 3]);
    let mut m = [64; 548];
    m[..4].copy_from_slice(&[1, 0, 0, 31]);
    m[4..9].copy_from_slice(&[1, 96, 96, 1, 64]);
    m[9..14].copy_from_slice(&[1, 96, 96, 1, 64]);
    m[14..32].fill(u32::MAX);
    m[22] = 3;
    m[31] = 3;
    Control {
        embedding_ns: [u64::MAX, 2],
        tail_ns: [3, 4, 5],
        layers: core::array::from_fn(|i| {
            let mut prefix_states = [p; 2];
            let mut tiles_states = [m; 2];
            for rank in 0..2 {
                for (j, owner) in prefix_states[rank][24..154].iter_mut().enumerate() {
                    *owner = ((i + rank + j) % 64 + 1) as u32;
                }
                for (j, owner) in tiles_states[rank][32..290].iter_mut().enumerate() {
                    *owner = ((i + 2 * rank + j + 3) % 64 + 1) as u32;
                }
            }
            LayerObservation {
                prefix_states,
                tiles_states,
                paired_ns: [[i as u64, u64::MAX - i as u64]; 4],
            }
        }),
    }
}
pub(crate) fn completed(
    r: &Request,
    chain: &mut Chain,
    winner: u32,
) -> (Response, Control, Vec<u8>) {
    let Command::Forward { token, .. } = r.command else {
        panic!("forward fixture")
    };
    let control = control();
    let mut bytes = vec![0; OBSERVATION_BYTES];
    bytes[37 * 8192 + winner as usize * 2..][..2].copy_from_slice(&0x3f80u16.to_le_bytes());
    let mut c = Completion {
        generation: r.id,
        position: r.id as u32 - 1,
        input_token: token,
        output_token: winner,
        control: part(&control.encode()),
        observation: part(&bytes),
        capture: Payload::from_bytes(&bytes).unwrap(),
        chain: [0; 32],
    };
    c.chain = chain.advance(&c);
    (
        Response {
            protocol: PROTOCOL,
            id: r.id,
            device_ids: r.device_ids,
            session: r.session,
            registration: r.registration,
            profile_sha256: r.profile_sha256,
            event: Event::Completed(c),
            native_closed: false,
            gpu_execution: true,
            numerical_acceptance: false,
            performance_claim: false,
            production_authority: false,
        },
        control,
        bytes,
    )
}
pub(crate) fn close(b: &Bootstrap, digest: [u8; 32]) -> (Request, Response) {
    let r = Request {
        protocol: PROTOCOL,
        id: 5,
        device_ids: b.device_ids,
        session: b.scope.session,
        registration: b.registration,
        profile_sha256: b.sha256().unwrap(),
        command: Command::Close,
    };
    let s = Response {
        protocol: PROTOCOL,
        id: 5,
        device_ids: r.device_ids,
        session: r.session,
        registration: r.registration,
        profile_sha256: r.profile_sha256,
        event: Event::Closed {
            completed_forwards: 4,
            transcript_sha256: digest,
        },
        native_closed: true,
        gpu_execution: true,
        numerical_acceptance: false,
        performance_claim: false,
        production_authority: false,
    };
    (r, s)
}

#[test]
fn prefix_decode_bootstrap_two_images_and_exact_begin_before_body() {
    for mode in [InputMode::TeacherForced, InputMode::Autoregressive] {
        let b = bootstrap(mode);
        let mut bytes = Vec::new();
        let mut sent = FrameBudget::new();
        write_bootstrap(&mut bytes, &mut sent, &b, &[8], &[9]).unwrap();
        account_begin(&mut sent, &b).unwrap();
        setup::write_request(&mut bytes, &begin_request(&b), &[7; 7]).unwrap();
        let mut input = &bytes[..];
        let mut budget = FrameBudget::new();
        let (actual, mlp, prefix) = read_bootstrap(&mut input, &mut budget).unwrap().unwrap();
        assert_eq!(actual, b);
        assert_eq!(mlp, [8]);
        assert_eq!(prefix, [9]);
        assert_eq!(read_begin(&mut input, &mut budget, &b).unwrap().1, [7; 7]);
        assert!(input.is_empty());
        assert_eq!(sent.used(), budget.used());
        let mut wrong = begin_request(&b);
        wrong.id = 2;
        let mut raw = Vec::new();
        write_header(&mut raw, &mut FrameBudget::new(), &wrong).unwrap();
        assert!(read_begin(&mut &raw[..], &mut FrameBudget::new(), &b).is_err());
        assert!(write_bootstrap(&mut Vec::new(), &mut FrameBudget::new(), &b, &[8], &[]).is_err());
        assert!(write_bootstrap(&mut Vec::new(), &mut FrameBudget::new(), &b, &[9], &[8]).is_err());
    }
}
#[test]
fn prefix_decode_bootstrap_closed_profile_mode_and_caps() {
    let b = bootstrap(InputMode::TeacherForced);
    for mutate in [
        |b: &mut Bootstrap| b.begin.tail_image = None,
        |b: &mut Bootstrap| b.begin.scope.group_id += 1,
        |b: &mut Bootstrap| b.begin.registration.sha256[0] ^= 1,
        |b: &mut Bootstrap| b.prefix_image.bytes = 0,
        |b: &mut Bootstrap| b.tiles_image.sha256 = [0; 32],
        |b: &mut Bootstrap| b.prefix_image.bytes = MAX_IMAGE_BYTES as u32 + 1,
        |b: &mut Bootstrap| b.begin.source_program.bytes = (4 << 20) + 1,
        |b: &mut Bootstrap| b.begin.prefix_image.bytes = u32::MAX,
        |b: &mut Bootstrap| b.input_tokens.pop().map(|_| ()).unwrap(),
    ] {
        let mut bad = b.clone();
        mutate(&mut bad);
        assert!(bad.sha256().is_err());
    }
    assert!(
        b.validate(
            b.device_ids,
            b.timeout_ms,
            b.scope.child_identity,
            InputMode::Autoregressive
        )
        .is_err()
    );
    let mut json = serde_json::to_value(&b).unwrap();
    json["profile"] = serde_json::json!("tiles_decode_four_forward_v1");
    assert!(serde_json::from_value::<Bootstrap>(json).is_err());
    let mut json = serde_json::to_value(&b).unwrap();
    json["kernel_admission"] = serde_json::json!("cached_immutable");
    assert!(serde_json::from_value::<Bootstrap>(json).is_err());
    assert_eq!(HEADER_BYTES, 65_536);
    assert!(4 * (CONTROL_BYTES + OBSERVATION_BYTES) + 5 * (HEADER_BYTES + 4) < STREAM_BYTES);
}
#[test]
fn prefix_decode_mode_history_requires_own_checked_previous_output() {
    let tf = bootstrap(InputMode::TeacherForced);
    let ar = bootstrap(InputMode::Autoregressive);
    for p in 0..4 {
        assert_eq!(tf.input(p, Some(13)).unwrap(), tf.input_tokens[p as usize]);
        assert_eq!(
            ar.input(p, Some(13)).unwrap(),
            if p == 0 { 9112 } else { 13 }
        );
    }
    assert!(ar.input(1, None).is_err());
    assert!(ar.input(1, Some(151936)).is_err());
    assert!(tf.input(4, None).is_err());
    assert_ne!(tf.sha256().unwrap(), ar.sha256().unwrap());
}
#[test]
fn prefix_decode_request_generation_bank_pages_and_rotary_are_closed() {
    let b = bootstrap(InputMode::TeacherForced);
    for p in 0..4 {
        let r = request(&b, p, None);
        r.validate().unwrap();
        for change in 0..6 {
            let mut bad = r.clone();
            if let Command::Forward {
                generation,
                token,
                cache_metadata,
                rotary_bits,
            } = &mut bad.command
            {
                match change {
                    0 => *generation += 1,
                    1 => *token = 151936,
                    2 => cache_metadata[0] += 1,
                    3 => cache_metadata[2] = cache_metadata[1],
                    4 => {
                        cache_metadata.pop();
                    }
                    _ => rotary_bits[127] = f32::INFINITY.to_bits(),
                }
            }
            assert!(bad.validate().is_err());
        }
    }
}
#[test]
fn prefix_decode_control_roundtrip_and_every_word_contract() {
    let c = control();
    c.validate().unwrap();
    let bytes = c.encode();
    assert_eq!(bytes.len(), 241_960);
    assert_eq!(Control::decode(&bytes).unwrap(), c);
    for layer in 0..36 {
        for rank in 0..2 {
            let base = 16 + layer * (2 * (284 + 548) * 4 + 64);
            let prefix = base + rank * 284 * 4 + 24 * 4;
            let mlp = base + 2 * 284 * 4 + rank * 548 * 4 + 32 * 4;
            assert_eq!(
                &bytes[prefix..prefix + 4],
                &c.layers[layer].prefix_states[rank][24].to_le_bytes()
            );
            assert_eq!(
                &bytes[mlp..mlp + 4],
                &c.layers[layer].tiles_states[rank][32].to_le_bytes()
            );
        }
    }
    for n in [0, 166504, CONTROL_BYTES - 1, CONTROL_BYTES + 1] {
        assert!(Control::decode(&vec![0; n]).is_err());
    }
    for word in 0..284 {
        let mut bad = c.clone();
        bad.layers[0].prefix_states[0][word] = if (24..154).contains(&word) {
            0
        } else {
            c.layers[0].prefix_states[0][word] ^ 1
        };
        assert!(bad.validate().is_err(), "prefix {word}");
    }
    for word in 0..548 {
        let mut bad = c.clone();
        bad.layers[0].tiles_states[0][word] = if (32..290).contains(&word) {
            65
        } else {
            c.layers[0].tiles_states[0][word] ^ 1
        };
        assert!(bad.validate().is_err(), "MLP {word}");
    }
    for owner in [1, 64] {
        let mut valid = c.clone();
        valid.layers[0].prefix_states[0][24..154].fill(owner);
        valid.layers[0].tiles_states[0][32..290].fill(owner);
        valid.validate().unwrap();
    }
}
#[test]
fn prefix_decode_every_layer_and_rank_requires_physical_epoch_one() {
    let c = control();
    for layer in 0..36 {
        for rank in 0..2 {
            for prefix in [false, true] {
                let mut bad = c.clone();
                if prefix {
                    bad.layers[layer].prefix_states[rank][0] = 3;
                } else {
                    bad.layers[layer].tiles_states[rank][0] = 3;
                }
                assert!(bad.validate().is_err());
            }
        }
    }
}
#[test]
fn prefix_decode_response_full_payload_and_lowest_index_argmax() {
    let b = bootstrap(InputMode::TeacherForced);
    let r = request(&b, 0, None);
    let (s, c, bytes) = completed(&r, &mut Chain::new(b.registration, b.sha256().unwrap()), 7);
    let mut encoded = Vec::new();
    write_response(&mut encoded, &mut FrameBudget::new(), &s, Some(&c), &bytes).unwrap();
    let (got, actual, raw) = read_response(&mut &encoded[..], &mut FrameBudget::new())
        .unwrap()
        .unwrap();
    assert_eq!(got, s);
    assert_eq!(actual.unwrap(), c);
    assert_eq!(raw, bytes);
    for offset in [0, 36 * 8192, 37 * 8192] {
        let mut bad = bytes.clone();
        bad[offset..offset + 2].copy_from_slice(&0x7f80u16.to_le_bytes());
        let mut s = s.clone();
        if let Event::Completed(c) = &mut s.event {
            c.observation = part(&bad);
            c.capture = Payload::from_bytes(&bad).unwrap();
        }
        assert!(
            write_response(&mut Vec::new(), &mut FrameBudget::new(), &s, Some(&c), &bad).is_err()
        );
    }
    let mut tie = bytes.clone();
    tie[37 * 8192..][..2].copy_from_slice(&0x3f80u16.to_le_bytes());
    let mut wrong = s.clone();
    if let Event::Completed(c) = &mut wrong.event {
        c.observation = part(&tie);
        c.capture = Payload::from_bytes(&tie).unwrap();
    }
    assert!(
        write_response(
            &mut Vec::new(),
            &mut FrameBudget::new(),
            &wrong,
            Some(&c),
            &tie
        )
        .is_err()
    );
    if let Event::Completed(c) = &mut wrong.event {
        c.output_token = 0;
    }
    write_response(
        &mut Vec::new(),
        &mut FrameBudget::new(),
        &wrong,
        Some(&c),
        &tie,
    )
    .unwrap();
}
#[test]
fn prefix_decode_close_and_claims_refuse_extra_body_or_wrong_count() {
    let b = bootstrap(InputMode::TeacherForced);
    let (r, s) = close(&b, [8; 32]);
    r.validate().unwrap();
    write_response(&mut Vec::new(), &mut FrameBudget::new(), &s, None, &[]).unwrap();
    for change in 0..5 {
        let mut bad = s.clone();
        match change {
            0 => bad.id = 4,
            1 => bad.native_closed = false,
            2 => {
                bad.event = Event::Closed {
                    completed_forwards: 3,
                    transcript_sha256: [8; 32],
                }
            }
            3 => bad.numerical_acceptance = true,
            _ => bad.production_authority = true,
        }
        assert!(write_response(&mut Vec::new(), &mut FrameBudget::new(), &bad, None, &[]).is_err());
    }
    assert!(
        write_response(
            &mut Vec::new(),
            &mut FrameBudget::new(),
            &s,
            Some(&control()),
            &[]
        )
        .is_err()
    );
    assert!(write_response(&mut Vec::new(), &mut FrameBudget::new(), &s, None, &[0]).is_err());
}
#[test]
fn prefix_decode_partial_eof_header_and_cumulative_budget_fail_closed() {
    let b = bootstrap(InputMode::TeacherForced);
    let (s, c, raw) = completed(
        &request(&b, 0, None),
        &mut Chain::new(b.registration, b.sha256().unwrap()),
        0,
    );
    let mut bytes = Vec::new();
    write_response(&mut bytes, &mut FrameBudget::new(), &s, Some(&c), &raw).unwrap();
    assert!(
        read_response(&mut &[][..], &mut FrameBudget::new())
            .unwrap()
            .is_none()
    );
    for n in [
        1,
        3,
        4,
        100,
        bytes.len() - OBSERVATION_BYTES,
        bytes.len() - 1,
    ] {
        assert!(read_response(&mut &bytes[..n], &mut FrameBudget::new()).is_err());
    }
    let oversize = (HEADER_BYTES as u32 + 1).to_le_bytes();
    assert!(read_request(&mut &oversize[..], &mut FrameBudget::new()).is_err());
    let mut budget = FrameBudget::new();
    budget.charge(STREAM_BYTES).unwrap();
    assert!(read_response(&mut &bytes[..], &mut budget).is_err());
}
