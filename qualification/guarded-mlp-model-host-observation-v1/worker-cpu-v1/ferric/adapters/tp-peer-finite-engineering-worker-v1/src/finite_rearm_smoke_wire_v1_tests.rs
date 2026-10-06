use super::*;

fn bootstrap(capture_layer0: bool) -> Bootstrap {
    Bootstrap {
        protocol: 1,
        profile: Profile::RearmFourForwardV1,
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
        prompt_tokens: vec![9112, 2190, 3772, 220],
        capture_layer0,
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
fn control() -> Control {
    Control {
        embedding_ns: [1, 2],
        tail_ns: [3, 4, 5],
        layers: core::array::from_fn(|_| crate::finite_forward_wire_v1::LayerObservation {
            prefix_states: [state(); 2],
            mlp_states: [state(); 2],
            paired_ns: [[1; 2]; 4],
        }),
    }
}
fn response(position: u32, with_stage: bool) -> (Response, Control, Vec<u8>, Vec<u8>) {
    let control = control();
    let bytes = vec![0; OBSERVATION_BYTES];
    let stage = if with_stage {
        stage_capture::fixture()
    } else {
        vec![]
    };
    let c = Completion {
        generation: u64::from(position) + 1,
        position,
        input_token: 7,
        output_token: 0,
        control: part(&control.encode()),
        observation: part(&bytes),
        capture: Payload::from_bytes(&bytes).unwrap(),
        stage_capture: with_stage.then(|| part(&stage)),
        chain: [6; 32],
    };
    (
        Response {
            protocol: 1,
            id: u64::from(position) + 1,
            device_ids: [11, 22],
            session: [3; 32],
            registration: [4; 32],
            profile_sha256: [5; 32],
            event: Event::Completed(c),
            native_closed: false,
            gpu_execution: true,
            numerical_acceptance: false,
            performance_claim: false,
            production_authority: false,
        },
        control,
        bytes,
        stage,
    )
}

#[test]
fn smoke_bootstrap_is_distinct_and_binds_first_four_prompt_and_capture_flag() {
    let good = bootstrap(false);
    good.validate([11, 22], 1000, 42, false).unwrap();
    assert_ne!(good.sha256().unwrap(), bootstrap(true).sha256().unwrap());
    assert!(good.validate([11, 22], 1000, 42, true).is_err());
    assert!(good.validate([11, 22], 1000, 43, false).is_err());
    assert!(
        serde_json::from_slice::<crate::finite_long_wire_v1::Bootstrap>(
            &header_bytes(&good).unwrap()
        )
        .is_err()
    );
    for mutation in 0..5 {
        let mut bad = good.clone();
        match mutation {
            0 => {
                bad.prompt_tokens.pop();
            }
            1 => bad.prompt_tokens.push(7),
            2 => bad.prompt_tokens[3] = 151_936,
            3 => bad.protocol = 2,
            _ => bad.device_ids = [11, 11],
        }
        assert!(bad.sha256().is_err());
    }
}

#[test]
fn all_four_captures_roundtrip_and_only_generation_one_can_have_annex() {
    for position in 0..4 {
        for with_stage in [false, true] {
            let (response, control, payload, stage) = response(position, with_stage);
            let mut bytes = Vec::new();
            let mut sent = FrameBudget::new();
            let result = write_response(
                &mut bytes,
                &mut sent,
                &response,
                Some(&control),
                &payload,
                &stage,
            );
            if with_stage && position != 0 {
                assert!(result.is_err());
                assert!(bytes.is_empty());
                continue;
            }
            result.unwrap();
            let mut read = FrameBudget::new();
            let actual = read_response(&mut bytes.as_slice(), &mut read)
                .unwrap()
                .unwrap();
            assert_eq!(actual, (response, Some(control), payload, stage));
            assert_eq!(read.used(), sent.used());
        }
    }
}

#[test]
fn annex_must_be_declared_exact_hashed_and_bounded() {
    let (response, control, payload, stage) = response(0, true);
    assert!(
        write_response(
            &mut Vec::new(),
            &mut FrameBudget::new(),
            &response,
            Some(&control),
            &payload,
            &[]
        )
        .is_err()
    );
    let mut absent = response.clone();
    let Event::Completed(c) = &mut absent.event else {
        unreachable!()
    };
    c.stage_capture = None;
    assert!(
        write_response(
            &mut Vec::new(),
            &mut FrameBudget::new(),
            &absent,
            Some(&control),
            &payload,
            &stage
        )
        .is_err()
    );
    let mut too_large = response.clone();
    let Event::Completed(c) = &mut too_large.event else {
        unreachable!()
    };
    c.stage_capture.as_mut().unwrap().bytes = STAGE_JSON_BYTES as u32 + 1;
    assert!(too_large.body_bytes().is_err());
    let mut bad = stage.clone();
    bad[0] ^= 1;
    assert!(
        write_response(
            &mut Vec::new(),
            &mut FrameBudget::new(),
            &response,
            Some(&control),
            &payload,
            &bad
        )
        .is_err()
    );
    assert!(validate_stage_capture(&vec![0; STAGE_JSON_BYTES + 1]).is_err());
}

#[test]
fn strict_stage_roster_hashes_scope_and_false_authority_are_checked() {
    let fixture = stage_capture::fixture();
    validate_stage_capture(&fixture).unwrap();
    for mutation in 0..9 {
        let mut value: serde_json::Value = serde_json::from_slice(&fixture).unwrap();
        match mutation {
            0 => value["generation"] = 2.into(),
            1 => value["layer"] = 1.into(),
            2 => value["native_close_confirmed"] = true.into(),
            3 => value["production_authority"] = true.into(),
            4 => value["parts"][0]["role"] = "weight".into(),
            5 => value["parts"][0]["rank"] = 1.into(),
            6 => value["parts"][0]["source_byte_offset"] = 2.into(),
            7 => value["payload"][0] = 1.into(),
            _ => value["unknown"] = 1.into(),
        }
        assert!(validate_stage_capture(&serde_json::to_vec(&value).unwrap()).is_err());
    }
    let duplicate = String::from_utf8(fixture)
        .unwrap()
        .replacen('{', "{\"generation\":1,", 1);
    assert!(validate_stage_capture(duplicate.as_bytes()).is_err());
}

#[test]
fn stage_final_rows_join_both_ranks_to_main_observation() {
    let (mut response, control, mut payload, stage) = response(0, true);
    payload[0] = 1;
    let Event::Completed(c) = &mut response.event else {
        unreachable!()
    };
    c.observation = part(&payload);
    c.capture = Payload::from_bytes(&payload).unwrap();
    assert!(
        write_response(
            &mut Vec::new(),
            &mut FrameBudget::new(),
            &response,
            Some(&control),
            &payload,
            &stage
        )
        .is_err()
    );
}

#[test]
fn four_forward_close_is_not_accepted_as_long_close() {
    let request = Request {
        protocol: 1,
        id: 5,
        device_ids: [11, 22],
        session: [3; 32],
        registration: [4; 32],
        profile_sha256: [5; 32],
        command: Command::Close,
    };
    let mut bytes = Vec::new();
    write_request(&mut bytes, &mut FrameBudget::new(), &request).unwrap();
    assert!(
        crate::finite_long_wire_v1::read_request(&mut bytes.as_slice(), &mut FrameBudget::new())
            .is_err()
    );
    for id in [1, 3, 4, 6, 2304] {
        let mut bad = request.clone();
        bad.id = id;
        assert!(bad.validate().is_err());
    }
}

#[test]
fn transcript_binds_optional_annex_without_trusting_claimed_chain() {
    let (response, _, _, _) = response(0, true);
    let Event::Completed(c) = response.event else {
        unreachable!()
    };
    let expected = Chain::new([1; 32], [2; 32]).advance(&c);
    let mut different = c.clone();
    different.chain = [0; 32];
    assert_eq!(Chain::new([1; 32], [2; 32]).advance(&different), expected);
    different.stage_capture = None;
    assert_ne!(Chain::new([1; 32], [2; 32]).advance(&different), expected);
}

#[test]
fn entire_smoke_binary_stream_and_annex_fit_unchanged_budget() {
    let mut budget = FrameBudget::new();
    let mut bytes = Vec::new();
    for position in 0..4 {
        let (response, control, payload, stage) = response(position, position == 0);
        write_response(
            &mut bytes,
            &mut budget,
            &response,
            Some(&control),
            &payload,
            &stage,
        )
        .unwrap();
    }
    assert_eq!(budget.used(), bytes.len());
    assert!(bytes.len() < 4 * 1024 * 1024);
    let (response, control, payload, stage) = response(0, true);
    let mut full = FrameBudget::new();
    full.charge(crate::finite_long_wire_v1::STREAM_BYTES)
        .unwrap();
    let mut sink = Vec::new();
    assert!(
        write_response(
            &mut sink,
            &mut full,
            &response,
            Some(&control),
            &payload,
            &stage
        )
        .is_err()
    );
    assert!(sink.is_empty());
}
