use super::*;

#[test]
fn tiles_admission_keeps_baseline_hashes_and_separates_cached_modes() {
    let golden = [
        [
            118, 18, 233, 48, 128, 150, 81, 197, 60, 79, 206, 109, 60, 28, 25, 220, 165, 21, 234,
            205, 125, 199, 101, 117, 137, 108, 179, 209, 44, 90, 78, 77,
        ],
        [
            169, 108, 178, 174, 45, 137, 63, 236, 54, 115, 163, 254, 45, 172, 201, 127, 195, 232,
            211, 156, 35, 33, 24, 249, 149, 20, 152, 194, 13, 170, 239, 201,
        ],
    ];
    let mut cached = Vec::new();
    for (i, mode) in [InputMode::TeacherForced, InputMode::Autoregressive]
        .into_iter()
        .enumerate()
    {
        let mut b = bootstrap(mode);
        assert_eq!(b.sha256().unwrap(), golden[i]);
        b.profile = Profile::TilesDecodeFourForwardCachedAdmissionV1;
        let digest = b.sha256().unwrap();
        assert!(!golden.contains(&digest));
        cached.push(digest);
        let mut bytes = Vec::new();
        write_bootstrap(&mut bytes, &mut FrameBudget::new(), &b, &[1, 2, 3]).unwrap();
        assert_eq!(
            read_bootstrap(&mut bytes.as_slice(), &mut FrameBudget::new()).unwrap(),
            Some((b, vec![1, 2, 3]))
        );
    }
    assert_ne!(cached[0], cached[1]);
}

#[test]
fn tiles_applied_admission_is_closed_and_absent_on_legacy_responses() {
    let (mut r, c, bytes) = completed();
    let baseline = serde_json::to_vec(&r).unwrap();
    assert!(
        !String::from_utf8(baseline.clone())
            .unwrap()
            .contains("applied_admission")
    );
    assert_eq!(serde_json::from_slice::<Response>(&baseline).unwrap(), r);
    r.applied_admission = KernelAdmission::CachedImmutable.receipt();
    let mut output = Vec::new();
    write_response(&mut output, &mut FrameBudget::new(), &r, Some(&c), &bytes).unwrap();
    assert_eq!(
        read_response(&mut output.as_slice(), &mut FrameBudget::new())
            .unwrap()
            .unwrap()
            .0,
        r
    );
    for bad in [
        AdmissionReceipt {
            cache_kernel_admission: false,
            operational_currentness: false,
            shared_full_currentness: false,
        },
        AdmissionReceipt {
            cache_kernel_admission: true,
            operational_currentness: true,
            shared_full_currentness: false,
        },
        AdmissionReceipt {
            cache_kernel_admission: true,
            operational_currentness: false,
            shared_full_currentness: true,
        },
    ] {
        r.applied_admission = Some(bad);
        assert!(
            write_response(
                &mut Vec::new(),
                &mut FrameBudget::new(),
                &r,
                Some(&c),
                &bytes
            )
            .is_err()
        );
    }
    let mut value =
        serde_json::to_value(KernelAdmission::CachedImmutable.receipt().unwrap()).unwrap();
    value["profile"] = true.into();
    assert!(serde_json::from_value::<AdmissionReceipt>(value).is_err());
}
fn bootstrap(mode: InputMode) -> Bootstrap {
    Bootstrap {
        protocol: 1,
        profile: Profile::TilesDecodeFourForwardV1,
        device_ids: [7, 9],
        scope: Scope {
            bundle_id: [1; 32],
            model_id: [2; 32],
            session: [3; 32],
            pool_identity: 1,
            group_id: 0,
            child_identity: 17,
        },
        registration: [4; 32],
        timeout_ms: 1000,
        mode,
        input_tokens: if mode == InputMode::TeacherForced {
            vec![9112, 2190, 3772, 220]
        } else {
            vec![9112]
        },
        tiles_image: part(&[1, 2, 3]),
    }
}
fn control() -> Control {
    let mut prefix = [64; 22];
    prefix[..6].copy_from_slice(&[1, 0, 65535, 65535, 0x55555555, 0]);
    let mut tiles = [0; 548];
    tiles[..4].copy_from_slice(&[1, 0, 0, 31]);
    tiles[4..9].copy_from_slice(&[1, 96, 96, 1, 64]);
    tiles[9..14].copy_from_slice(&[1, 96, 96, 1, 64]);
    tiles[14..23].fill(u32::MAX);
    tiles[22] = 3;
    tiles[23..32].fill(u32::MAX);
    tiles[31] = 3;
    tiles[32..290].fill(64);
    tiles[290..548].fill(64);
    Control {
        embedding_ns: [u64::MAX; 2],
        layers: core::array::from_fn(|_| LayerObservation {
            prefix_states: [prefix; 2],
            tiles_states: [tiles; 2],
            paired_ns: [[u64::MAX; 2]; 4],
        }),
        tail_ns: [u64::MAX; 3],
    }
}
fn request(id: u64) -> Request {
    Request {
        protocol: 1,
        id,
        device_ids: [7, 9],
        session: [3; 32],
        registration: [4; 32],
        profile_sha256: [5; 32],
        command: if id == 5 {
            Command::Close
        } else {
            Command::Forward {
                generation: id,
                token: 9112,
                cache_metadata: std::iter::once(id as u32 - 1).chain(0..144).collect(),
                rotary_bits: vec![0; 128],
            }
        },
    }
}
fn completed() -> (Response, Control, Vec<u8>) {
    let control = control();
    let main = vec![0; OBSERVATION_BYTES];
    let mut c = Completion {
        generation: 1,
        position: 0,
        input_token: 9112,
        output_token: 0,
        control: part(&control.encode()),
        observation: part(&main),
        capture: Payload::from_bytes(&main).unwrap(),
        chain: [0; 32],
    };
    c.chain = Chain::new([4; 32], [5; 32]).advance(&c);
    (
        Response {
            protocol: 1,
            id: 1,
            device_ids: [7, 9],
            session: [3; 32],
            registration: [4; 32],
            profile_sha256: [5; 32],
            event: Event::Completed(c),
            native_closed: false,
            gpu_execution: true,
            numerical_acceptance: false,
            performance_claim: false,
            production_authority: false,
            applied_admission: None,
        },
        control,
        main,
    )
}
#[test]
fn tiles_wire_bootstrap_exact_modes_image_and_frame_roundtrip() {
    for mode in [InputMode::TeacherForced, InputMode::Autoregressive] {
        let b = bootstrap(mode);
        let mut bytes = Vec::new();
        let mut sent = FrameBudget::new();
        write_bootstrap(&mut bytes, &mut sent, &b, &[1, 2, 3]).unwrap();
        let mut received = FrameBudget::new();
        assert_eq!(
            read_bootstrap(&mut bytes.as_slice(), &mut received).unwrap(),
            Some((b.clone(), vec![1, 2, 3]))
        );
        assert_eq!(sent.used(), received.used());
        assert!(write_bootstrap(&mut Vec::new(), &mut FrameBudget::new(), &b, &[1, 2, 4]).is_err());
        bytes.pop();
        assert!(read_bootstrap(&mut bytes.as_slice(), &mut FrameBudget::new()).is_err());
        let mut bad = b;
        bad.input_tokens.push(1);
        assert!(bad.sha256().is_err());
    }
}
#[test]
fn tiles_wire_mode_identity_and_own_output_trajectory_are_distinct() {
    let tf = bootstrap(InputMode::TeacherForced);
    let ar = bootstrap(InputMode::Autoregressive);
    assert_ne!(tf.sha256().unwrap(), ar.sha256().unwrap());
    assert_eq!(tf.input(1, Some(67)).unwrap(), 2190);
    assert_eq!(ar.input(0, None).unwrap(), 9112);
    assert_eq!(ar.input(1, Some(67)).unwrap(), 67);
    assert!(ar.input(1, None).is_err());
    assert!(ar.input(1, Some(151936)).is_err());
    assert!(tf.input(4, None).is_err());
    assert!(
        tf.validate([7, 9], 1000, 17, InputMode::Autoregressive)
            .is_err()
    );
    for change in 0..5 {
        let mut bad = tf.clone();
        match change {
            0 => bad.registration[0] ^= 1,
            1 => bad.tiles_image.sha256[0] ^= 1,
            2 => bad.scope.session[0] ^= 1,
            3 => bad.device_ids.swap(0, 1),
            _ => bad.input_tokens[3] += 1,
        };
        assert_ne!(bad.sha256().unwrap(), tf.sha256().unwrap());
    }
}
#[test]
fn tiles_wire_exact_all_layer_terminal_states_and_no_v1_control() {
    let c = control();
    assert_eq!(c.encode().len(), CONTROL_BYTES);
    assert_eq!(Control::decode(&c.encode()).unwrap(), c);
    assert!(Control::decode(&vec![0; 11848]).is_err());
    for (layer, rank, word) in [(0, 0, 0), (35, 1, 547), (35, 0, 22), (0, 1, 32)] {
        let mut bad = c.clone();
        bad.layers[layer].tiles_states[rank][word] = 0;
        assert!(bad.validate().is_err());
        assert!(Control::decode(&bad.encode()).is_err());
    }
    let mut bad = c;
    bad.layers[35].prefix_states[1][21] = 63;
    assert!(bad.validate().is_err());
}
#[test]
fn tiles_wire_request_order_metadata_rotary_and_closed_schema() {
    for id in 1..=5 {
        let r = request(id);
        let mut bytes = Vec::new();
        write_request(&mut bytes, &mut FrameBudget::new(), &r).unwrap();
        assert_eq!(
            read_request(&mut bytes.as_slice(), &mut FrameBudget::new()).unwrap(),
            Some(r)
        );
    }
    let mut early = request(1);
    early.command = Command::Close;
    assert!(early.validate().is_err());
    let mut late = request(6);
    assert!(late.validate().is_err());
    late.id = 4;
    assert!(late.validate().is_err());
    for change in 0..4 {
        let mut bad = request(1);
        if let Command::Forward {
            generation,
            cache_metadata,
            rotary_bits,
            ..
        } = &mut bad.command
        {
            match change {
                0 => *generation = 2,
                1 => cache_metadata[2] = 0,
                2 => cache_metadata[0] = 1,
                _ => rotary_bits[127] = f32::NAN.to_bits(),
            }
        }
        assert!(bad.validate().is_err());
    }
    let mut json = serde_json::to_value(request(1)).unwrap();
    json["old_control"] = true.into();
    assert!(serde_json::from_value::<Request>(json).is_err());
}
#[test]
fn tiles_wire_completion_roundtrip_and_partial_eof_refusal() {
    let (response, control, main) = completed();
    let mut bytes = Vec::new();
    let mut sent = FrameBudget::new();
    write_response(&mut bytes, &mut sent, &response, Some(&control), &main).unwrap();
    let mut received = FrameBudget::new();
    let actual = read_response(&mut bytes.as_slice(), &mut received)
        .unwrap()
        .unwrap();
    assert_eq!(actual, (response, Some(control), main));
    assert_eq!(sent.used(), received.used());
    bytes.pop();
    assert!(read_response(&mut bytes.as_slice(), &mut FrameBudget::new()).is_err());
    assert!(
        read_response(&mut &[][..], &mut FrameBudget::new())
            .unwrap()
            .is_none()
    );
}
#[test]
fn tiles_wire_control_payload_hash_finite_and_argmax_refusals() {
    let (r, c, main) = completed();
    let mut changed = c.clone();
    changed.tail_ns[0] = 0;
    assert!(
        write_response(
            &mut Vec::new(),
            &mut FrameBudget::new(),
            &r,
            Some(&changed),
            &main
        )
        .is_err()
    );
    for change in 0..3 {
        let mut bad = r.clone();
        let mut bytes = main.clone();
        if let Event::Completed(v) = &mut bad.event {
            match change {
                0 => v.output_token = 1,
                1 => {
                    bytes[0..2].copy_from_slice(&0x7f80u16.to_le_bytes());
                    v.capture = Payload::from_bytes(&bytes).unwrap();
                    v.observation = part(&bytes);
                }
                _ => v.control.bytes = 11848,
            }
        }
        assert!(
            write_response(
                &mut Vec::new(),
                &mut FrameBudget::new(),
                &bad,
                Some(&c),
                &bytes
            )
            .is_err()
        );
    }
}
#[test]
fn tiles_wire_chain_binds_every_forward_identity_and_body_digest() {
    let (r, _, _) = completed();
    let Event::Completed(c) = r.event else {
        unreachable!()
    };
    let expected = Chain::new([4; 32], [5; 32]).advance(&c);
    for change in 0..6 {
        let mut v = c.clone();
        match change {
            0 => v.generation += 1,
            1 => v.input_token += 1,
            2 => v.output_token += 1,
            3 => v.control.sha256[0] ^= 1,
            4 => v.observation.sha256[0] ^= 1,
            _ => v.position += 1,
        }
        assert_ne!(Chain::new([4; 32], [5; 32]).advance(&v), expected);
    }
    let mut v = c.clone();
    v.chain = [99; 32];
    assert_eq!(Chain::new([4; 32], [5; 32]).advance(&v), expected);
}
#[test]
fn tiles_wire_close_has_no_body_or_authority_and_budget_is_cumulative() {
    let (mut r, c, main) = completed();
    r.id = 5;
    r.native_closed = true;
    r.event = Event::Closed {
        completed_forwards: 4,
        transcript_sha256: [6; 32],
    };
    let mut bytes = Vec::new();
    write_response(&mut bytes, &mut FrameBudget::new(), &r, None, &[]).unwrap();
    assert_eq!(
        read_response(&mut bytes.as_slice(), &mut FrameBudget::new()).unwrap(),
        Some((r.clone(), None, vec![]))
    );
    assert!(
        write_response(
            &mut Vec::new(),
            &mut FrameBudget::new(),
            &r,
            Some(&c),
            &main
        )
        .is_err()
    );
    r.performance_claim = true;
    assert!(r.body_bytes().is_err());
    let mut b = FrameBudget::new();
    b.charge(64 << 20).unwrap();
    assert!(write_request(&mut Vec::new(), &mut b, &request(1)).is_err());
    assert_eq!(4 * (CONTROL_BYTES + OBSERVATION_BYTES), 3_093_920);
}
