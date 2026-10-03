use super::*;

fn digest(text: &str) -> [u8; 32] {
    core::array::from_fn(|i| u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).unwrap())
}
fn config() -> Config {
    let pin = FilePin {
        path: "/task/image".into(),
        bytes: 1,
        sha256: [1; 32],
    };
    Config {
        schema: "FerricFiniteLongRequestV1".into(),
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
        prompt: PromptPins {
            manifest: FilePin {
                path: "/task/prompt-manifest.json".into(),
                bytes: 21_318,
                sha256: digest(MANIFEST_SHA),
            },
            text: FilePin {
                path: "/task/prompt.txt".into(),
                bytes: 11_224,
                sha256: digest(PROMPT_SHA),
            },
            tokens: FilePin {
                path: "/task/prompt.u32le".into(),
                bytes: 8192,
                sha256: digest(TOKENS_SHA),
            },
        },
        evidence_directory: "/task/long-evidence".into(),
        dispatch_timeout_ms: 10_000,
        child_deadline_ms: 3_600_000,
    }
}
fn request(position: u32) -> wire::Request {
    wire::Request {
        protocol: wire::PROTOCOL,
        id: u64::from(position) + 1,
        device_ids: [7, 9],
        session: [3; 32],
        registration: [4; 32],
        profile_sha256: [5; 32],
        command: wire::Command::Forward {
            generation: u64::from(position) + 1,
            token: 9112,
            cache_metadata: std::iter::once(position).chain(0..144).collect(),
            rotary_bits: vec![0; 128],
        },
    }
}
fn control() -> wire::Control {
    let mut prefix = [64; 22];
    prefix[..6].copy_from_slice(&[1, 0, 0xffff, 0xffff, 0x55555555, 0]);
    let mut mlp = [64; 11];
    mlp[..6].copy_from_slice(&[1, 0, 31, 31, 0x155, 0]);
    wire::Control {
        embedding_ns: [0; 2],
        layers: core::array::from_fn(|_| old::LayerObservation {
            prefix_states: [prefix; 2],
            mlp_states: [mlp; 2],
            paired_ns: [[0; 2]; 4],
        }),
        tail_ns: [0; 3],
    }
}
fn response(position: u32, control: &wire::Control, payload: &[u8]) -> wire::Response {
    let request = request(position);
    let capture =
        wire::capture_position(position).then(|| old::Payload::from_bytes(payload).unwrap());
    let observation = capture.as_ref().map_or(
        setup_wire::Part {
            bytes: old::OBSERVATION_BYTES as u32,
            sha256: [9; 32],
        },
        |p| p.total,
    );
    let mut c = wire::Completion {
        generation: request.id,
        position,
        input_token: 9112,
        output_token: 0,
        control: old::part(&control.encode()),
        observation,
        capture,
        chain: [0; 32],
    };
    c.chain = wire::Chain::new(request.registration, request.profile_sha256).advance(&c);
    wire::Response {
        protocol: request.protocol,
        id: request.id,
        device_ids: request.device_ids,
        session: request.session,
        registration: request.registration,
        profile_sha256: request.profile_sha256,
        event: wire::Event::Completed(c),
        native_closed: false,
        gpu_execution: true,
        numerical_acceptance: false,
        performance_claim: false,
        production_authority: false,
    }
}
fn validate(
    request: &wire::Request,
    response: &wire::Response,
    control: &wire::Control,
    payload: &[u8],
) -> Result<u32> {
    validate_completion(
        request,
        response,
        control,
        payload,
        &mut wire::Chain::new(request.registration, request.profile_sha256),
    )
    .map(|c| c.output_token)
}

#[test]
fn closed_config_pins_original_prompt_and_preserves_existing_resource_caps() {
    let original = config();
    original.validate().unwrap();
    Config::parse(&serde_json::to_vec(&original).unwrap()).unwrap();
    for mutation in 0..10 {
        let mut changed = original.clone();
        match mutation {
            0 => changed.schema = "FerricFiniteTwoForwardRequestV1".into(),
            1 => changed.device_ids[1] = changed.device_ids[0],
            2 => changed.session = [0; 32],
            3 => changed.expected_model_id = [0; 32],
            4 => changed.prompt.tokens.sha256[0] ^= 1,
            5 => changed.prompt.manifest.bytes -= 1,
            6 => changed.dispatch_timeout_ms = 10_001,
            7 => changed.child_deadline_ms = 3_600_001,
            8 => changed.evidence_directory = "relative".into(),
            9 => changed.prompt.text.sha256[0] ^= 1,
            _ => unreachable!(),
        }
        assert!(changed.validate().is_err());
    }
    for field in ["input_tokens", "mode", "forwards", "fallback"] {
        let mut value = serde_json::to_value(&original).unwrap();
        value[field] = serde_json::json!(4);
        assert!(Config::parse(&serde_json::to_vec(&value).unwrap()).is_err());
    }
    let json = String::from_utf8(serde_json::to_vec(&original).unwrap()).unwrap();
    let duplicate = json.replacen("{", "{\"schema\":\"FerricFiniteLongRequestV1\",", 1);
    assert!(Config::parse(duplicate.as_bytes()).is_err());
    assert!(Config::parse(&vec![0; 65_537]).is_err());
}

#[test]
fn prompt_then_actual_autoregressive_outputs_gives_exact_2303_to_256_schedule() {
    let prompt: Vec<u32> = (0..2048).collect();
    let mut previous = None;
    let mut generated = Vec::new();
    for position in 0..wire::FORWARDS {
        let input = input_token(&prompt, position, previous).unwrap();
        if position < 2048 {
            assert_eq!(input, position);
        } else {
            assert_eq!(input, 10_000 + position - 1);
        }
        let actual_output = 10_000 + position;
        if position >= 2047 {
            generated.push(actual_output);
        }
        previous = Some(actual_output);
    }
    assert_eq!(generated.len(), 256);
    assert_eq!((generated[0], generated[255]), (12_047, 12_302));
    assert!(input_token(&prompt, 2048, None).is_err());
    assert!(input_token(&prompt, 2303, previous).is_err());
    assert!(input_token(&prompt[..2047], 0, None).is_err());
    assert!(input_token(&prompt, 2048, Some(VOCABULARY)).is_err());
}

#[test]
fn real_paged_pool_preserves_full_permutation_across_all_context_boundaries() {
    let scope = EngineeringTpPoolScopeV1 {
        model: [2; 32],
        session: [3; 32],
    };
    let limits = EngineeringTpPagedLimitsV1::new(2304, 32, 144, 1024).unwrap();
    let mut pool = EngineeringTpPagedPoolV1::new(scope, limits).unwrap();
    let sequence = pool.open_sequence(scope, &[9112], 0).unwrap().sequence();
    let mut stable = None;
    for position in 0..wire::FORWARDS {
        assert_eq!(pool.committed_position(sequence).unwrap(), position);
        let batch = pool
            .reserve_batch(&[EngineeringTpPageRowV1 {
                sequence,
                token: 9112,
                position,
            }])
            .unwrap();
        let m = metadata(&batch, [4; 32], 1_000_000).unwrap();
        stable_pages(&mut stable, m.cache_metadata()).unwrap();
        if [0, 1, 15, 16, 2047, 2048, 2302].contains(&position) {
            assert_eq!(m.cache_metadata()[0], position);
            let (mut expected, sin) = crate::tp_execution::rope_bytes(position, 1_000_000);
            expected.extend(sin);
            let actual: Vec<u8> = m
                .rotary()
                .iter()
                .flat_map(|value| value.to_le_bytes())
                .collect();
            assert_eq!(actual, expected);
            assert_eq!(
                batch.rows()[0].physical_pages().len(),
                (position / 16 + 1) as usize
            );
        }
        // CPU-only completion fixture; production commits solely after wire/state validation.
        pool.begin_submission(&batch).unwrap();
        pool.commit_batch(
            &batch,
            EngineeringTpBatchCompletionV1::after_all_ranks(&batch),
        )
        .unwrap();
    }
    assert_eq!(pool.committed_position(sequence).unwrap(), 2303);
    let mut changed: [u32; 145] =
        core::array::from_fn(|i| if i == 0 { 2302 } else { stable.unwrap()[i - 1] });
    changed.swap(2, 3);
    assert!(stable_pages(&mut stable, &changed).is_err());
}

#[test]
fn uncaptured_completion_still_checks_every_terminal_state_scope_and_chain() {
    let request = request(1);
    let control = control();
    let original = response(1, &control, &[]);
    assert_eq!(validate(&request, &original, &control, &[]).unwrap(), 0);
    for mutation in 0..11 {
        let mut changed = original.clone();
        match mutation {
            0 => changed.device_ids.swap(0, 1),
            1 => changed.registration[0] ^= 1,
            2 => changed.profile_sha256[0] ^= 1,
            3 => changed.numerical_acceptance = true,
            4 => changed.performance_claim = true,
            5 => changed.production_authority = true,
            6 => changed.native_closed = true,
            7 => changed.gpu_execution = false,
            8..=10 => {
                if let wire::Event::Completed(c) = &mut changed.event {
                    match mutation {
                        8 => c.input_token += 1,
                        9 => c.chain[0] ^= 1,
                        _ => c.observation.bytes -= 1,
                    }
                }
            }
            _ => unreachable!(),
        }
        assert!(validate(&request, &changed, &control, &[]).is_err());
    }
    for layer in 0..36 {
        for rank in 0..2 {
            let mut bad = control.clone();
            bad.layers[layer].prefix_states[rank][3] &= !0x8000;
            assert!(validate(&request, &original, &bad, &[]).is_err());
            bad = control.clone();
            bad.layers[layer].mlp_states[rank][10] = 63;
            assert!(validate(&request, &original, &bad, &[]).is_err());
        }
    }
    assert!(validate(&request, &original, &control, &[0, 0]).is_err());
}

#[test]
fn selected_captures_join_finite_bf16_lowest_argmax_and_exact_bytes() {
    assert_eq!(wire::CAPTURE_POSITIONS, [0, 2047, 2048, 2302]);
    let control = control();
    let mut payload = vec![0; old::OBSERVATION_BYTES];
    let request = request(0);
    let original = response(0, &control, &payload);
    assert_eq!(
        validate(&request, &original, &control, &payload).unwrap(),
        0
    );
    payload[0..2].copy_from_slice(&0x7f80_u16.to_le_bytes());
    let nonfinite = response(0, &control, &payload);
    assert!(validate(&request, &nonfinite, &control, &payload).is_err());
    payload[0..2].copy_from_slice(&0_u16.to_le_bytes());
    let logit_one = old::OBSERVATION_BYTES - 303_872 + 2;
    payload[logit_one..logit_one + 2].copy_from_slice(&0x3f80_u16.to_le_bytes());
    let wrong_winner = response(0, &control, &payload);
    assert!(validate(&request, &wrong_winner, &control, &payload).is_err());
    assert!(validate(&request, &original, &control, &payload).is_err());
}

#[test]
fn close_requires_exact_completed_count_profile_transcript_and_empty_body() {
    let mut request = request(2302);
    request.id = 2304;
    request.command = wire::Command::Close;
    let chain = [7; 32];
    let mut response = response(1, &control(), &[]);
    response.id = 2304;
    response.native_closed = true;
    response.event = wire::Event::Closed {
        completed_forwards: 2303,
        transcript_sha256: chain,
    };
    validate_close(&request, &response, None, &[], chain).unwrap();
    assert!(validate_close(&request, &response, None, &[], [8; 32]).is_err());
    assert!(validate_close(&request, &response, Some(&control()), &[], chain).is_err());
    assert!(validate_close(&request, &response, None, &[0], chain).is_err());
    response.event = wire::Event::Closed {
        completed_forwards: 2,
        transcript_sha256: chain,
    };
    assert!(validate_close(&request, &response, None, &[], chain).is_err());
}

#[test]
fn absent_opt_in_never_opens_files_processes_or_model() {
    assert!(run(config(), false).err().unwrap().contains("opt-in"));
}
