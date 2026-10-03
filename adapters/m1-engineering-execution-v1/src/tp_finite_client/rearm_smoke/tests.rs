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
        schema: "FerricFiniteRearmSmokeRequestV1".into(),
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
        capture_layer0: false,
        evidence_directory: "/task/smoke-evidence".into(),
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
            token: INPUT_TOKENS[position as usize],
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
fn response(
    position: u32,
    control: &wire::Control,
    payload: &[u8],
    stage: &[u8],
) -> wire::Response {
    let request = request(position);
    let capture = old::Payload::from_bytes(payload).unwrap();
    let mut c = wire::Completion {
        generation: request.id,
        position,
        input_token: INPUT_TOKENS[position as usize],
        output_token: 0,
        control: old::part(&control.encode()),
        observation: capture.total,
        capture,
        stage_capture: (!stage.is_empty()).then(|| old::part(stage)),
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
    stage: &[u8],
    capture_layer0: bool,
) -> Result<u32> {
    validate_completion(
        request,
        response,
        control,
        payload,
        stage,
        capture_layer0,
        &mut wire::Chain::new(request.registration, request.profile_sha256),
    )
    .map(|c| c.output_token)
}

// Synthetic data only: independent schema builder for the production parser and
// parent joins, not a repeated numerical implementation or actual capture.
fn stage_fixture() -> Vec<u8> {
    let specs: &[(&str, &[(&str, &str, usize)])] = &[
        (
            "before_prefix",
            &[
                ("input", "bf16", 8192),
                ("cache_metadata", "u32", 580),
                ("rotary", "f32", 512),
            ],
        ),
        (
            "after_prefix",
            &[
                ("input_normalized", "bf16", 8192),
                ("raw_qkv", "bf16", 6144),
                ("query", "bf16", 4096),
                ("current_key", "bf16", 1024),
                ("current_value", "bf16", 1024),
                ("attention", "bf16", 4096),
                ("output_partial", "f32", 16384),
            ],
        ),
        ("after_first_residual", &[("first_residual", "bf16", 8192)]),
        (
            "after_mlp",
            &[
                ("post_normalized", "bf16", 8192),
                ("gate", "bf16", 12288),
                ("up", "bf16", 12288),
                ("activation", "bf16", 12288),
                ("down_partial", "f32", 16384),
            ],
        ),
        ("after_final_residual", &[("final_hidden", "bf16", 8192)]),
    ];
    let mut payload = Vec::new();
    let mut parts = Vec::new();
    for &(boundary, roles) in specs {
        for rank in 0..2 {
            for &(role, scalar, bytes) in roles {
                let mut data = vec![0; bytes];
                if role == "cache_metadata" {
                    for (index, word) in data[4..].chunks_exact_mut(4).enumerate() {
                        word.copy_from_slice(&(index as u32).to_le_bytes());
                    }
                }
                parts.push(serde_json::json!({"boundary":boundary,"rank":rank,"role":role,"scalar":scalar,
                    "elements":bytes / if scalar == "bf16" {2} else {4},"offset":payload.len(),"bytes":bytes,
                    "source_byte_offset":0,"sha256":hash(&data)}));
                payload.extend_from_slice(&data);
            }
        }
    }
    serde_json::to_vec(&serde_json::json!({"schema":"FerricFiniteLayerZeroCaptureV1","generation":1,"position":0,"layer":0,
        "parts":parts,"payload_bytes":payload.len(),"payload_sha256":hash(&payload),"payload":payload,
        "full_cache_capture":false,"native_close_confirmed":false,"numerical_acceptance":false,
        "performance_claim":false,"production_authority":false})).unwrap()
}
fn change_stage_data(bytes: &[u8], role: &str, data: &[u8]) -> Vec<u8> {
    let mut value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    let mut payload: Vec<u8> = serde_json::from_value(value["payload"].clone()).unwrap();
    let mut parts = value["parts"].as_array().unwrap().clone();
    let part = parts
        .iter_mut()
        .find(|p| p["role"] == role && p["rank"] == 0)
        .unwrap();
    let offset = part["offset"].as_u64().unwrap() as usize;
    let size = part["bytes"].as_u64().unwrap() as usize;
    payload[offset..offset + data.len()].copy_from_slice(data);
    part["sha256"] = serde_json::to_value(hash(&payload[offset..offset + size])).unwrap();
    value["parts"] = serde_json::Value::Array(parts);
    value["payload_sha256"] = serde_json::to_value(hash(&payload)).unwrap();
    value["payload"] = serde_json::to_value(payload).unwrap();
    serde_json::to_vec(&value).unwrap()
}

#[test]
fn smoke_config_is_closed_distinct_and_requires_explicit_capture_boolean() {
    let original = config();
    original.validate().unwrap();
    Config::parse(&serde_json::to_vec(&original).unwrap()).unwrap();
    let mut captured = original.clone();
    captured.capture_layer0 = true;
    captured.validate().unwrap();
    for mutation in 0..8 {
        let mut bad = original.clone();
        match mutation {
            0 => bad.schema = "FerricFiniteLongRequestV1".into(),
            1 => bad.device_ids[1] = bad.device_ids[0],
            2 => bad.prompt.tokens.sha256[0] ^= 1,
            3 => bad.prompt.manifest.bytes -= 1,
            4 => bad.dispatch_timeout_ms = 10_001,
            5 => bad.child_deadline_ms = 3_600_001,
            6 => bad.expected_model_id = [0; 32],
            _ => bad.evidence_directory = "relative".into(),
        }
        assert!(bad.validate().is_err());
    }
    for field in ["input_tokens", "mode", "forwards", "fallback"] {
        let mut value = serde_json::to_value(&original).unwrap();
        value[field] = 4.into();
        assert!(Config::parse(&serde_json::to_vec(&value).unwrap()).is_err());
    }
    let mut value = serde_json::to_value(&original).unwrap();
    value.as_object_mut().unwrap().remove("capture_layer0");
    assert!(Config::parse(&serde_json::to_vec(&value).unwrap()).is_err());
    let bytes = String::from_utf8(serde_json::to_vec(&original).unwrap())
        .unwrap()
        .replacen('{', "{\"capture_layer0\":false,", 1);
    assert!(Config::parse(bytes.as_bytes()).is_err());
}

#[test]
fn exactly_four_authentic_teacher_forced_inputs_no_decode_or_early_close() {
    assert_eq!(
        (0..4).map(|p| input_token(p).unwrap()).collect::<Vec<_>>(),
        [9112, 2190, 3772, 220]
    );
    assert!(input_token(4).is_err());
    let mut close = request(3);
    close.command = wire::Command::Close;
    assert!(close.validate().is_err());
    close.id = 5;
    close.validate().unwrap();
}

#[test]
fn four_prepared_pool_rows_carry_real_history_and_rope_without_page_remap() {
    let scope = EngineeringTpPoolScopeV1 {
        model: [2; 32],
        session: [3; 32],
    };
    let mut pool = EngineeringTpPagedPoolV1::new(
        scope,
        EngineeringTpPagedLimitsV1::new(2304, 32, 144, 1024).unwrap(),
    )
    .unwrap();
    let sequence = pool
        .open_sequence(scope, &[INPUT_TOKENS[0]], 0)
        .unwrap()
        .sequence();
    let mut stable = None;
    for position in 0..4 {
        let batch = pool
            .reserve_batch(&[EngineeringTpPageRowV1 {
                sequence,
                token: input_token(position).unwrap(),
                position,
            }])
            .unwrap();
        let value = metadata(&batch, [4; 32], 1_000_000).unwrap();
        stable_pages(&mut stable, value.cache_metadata()).unwrap();
        let (mut expected, sin) = crate::tp_execution::rope_bytes(position, 1_000_000);
        expected.extend(sin);
        assert_eq!(
            value
                .rotary()
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect::<Vec<_>>(),
            expected
        );
        pool.begin_submission(&batch).unwrap();
        // CPU-only completion fixture, never actual native authority.
        pool.commit_batch(
            &batch,
            EngineeringTpBatchCompletionV1::after_all_ranks(&batch),
        )
        .unwrap();
    }
    assert_eq!(pool.committed_position(sequence).unwrap(), 4);
}

#[test]
fn every_frame_requires_all_states_finite_main_capture_argmax_and_profile() {
    let control = control();
    let payload = vec![0; old::OBSERVATION_BYTES];
    for position in 0..4 {
        let request = request(position);
        let original = response(position, &control, &payload, &[]);
        assert_eq!(
            validate(&request, &original, &control, &payload, &[], false).unwrap(),
            0
        );
        let mut bad = original.clone();
        bad.profile_sha256[0] ^= 1;
        assert!(validate(&request, &bad, &control, &payload, &[], false).is_err());
        let mut bad_control = control.clone();
        bad_control.layers[35].prefix_states[1][3] &= !0x8000;
        assert!(validate(&request, &original, &bad_control, &payload, &[], false).is_err());
        bad_control = control.clone();
        bad_control.layers[0].mlp_states[0][10] = 63;
        assert!(validate(&request, &original, &bad_control, &payload, &[], false).is_err());
    }
    let mut nonfinite = payload.clone();
    nonfinite[..2].copy_from_slice(&0x7f80_u16.to_le_bytes());
    assert!(
        validate(
            &request(0),
            &response(0, &control, &nonfinite, &[]),
            &control,
            &nonfinite,
            &[],
            false
        )
        .is_err()
    );
    let mut wrong_argmax = payload.clone();
    let at = old::OBSERVATION_BYTES - 303_872 + 2;
    wrong_argmax[at..at + 2].copy_from_slice(&0x3f80_u16.to_le_bytes());
    assert!(
        validate(
            &request(0),
            &response(0, &control, &wrong_argmax, &[]),
            &control,
            &wrong_argmax,
            &[],
            false
        )
        .is_err()
    );
}

#[test]
fn four_completed_frames_share_one_chain_and_replay_is_rejected() {
    let control = control();
    let payload = vec![0; old::OBSERVATION_BYTES];
    let mut child = wire::Chain::new([4; 32], [5; 32]);
    let mut parent = wire::Chain::new([4; 32], [5; 32]);
    let mut last = None;
    for position in 0..4 {
        let request = request(position);
        let mut response = response(position, &control, &payload, &[]);
        let wire::Event::Completed(c) = &mut response.event else {
            unreachable!()
        };
        c.chain = child.advance(c);
        validate_completion(
            &request,
            &response,
            &control,
            &payload,
            &[],
            false,
            &mut parent,
        )
        .unwrap();
        last = Some((request, response));
    }
    assert_eq!(parent.digest(), child.digest());
    let (request, response) = last.unwrap();
    assert!(
        validate_completion(
            &request,
            &response,
            &control,
            &payload,
            &[],
            false,
            &mut parent
        )
        .is_err()
    );
}

#[test]
fn optional_stage_is_explicit_only_generation1_and_joins_main_and_request() {
    let control = control();
    let payload = vec![0; old::OBSERVATION_BYTES];
    let stage = stage_fixture();
    wire::validate_stage_capture(&stage).unwrap();
    let original = response(0, &control, &payload, &stage);
    validate(&request(0), &original, &control, &payload, &stage, true).unwrap();
    assert!(validate(&request(0), &original, &control, &payload, &stage, false).is_err());
    assert!(
        validate(
            &request(0),
            &response(0, &control, &payload, &[]),
            &control,
            &payload,
            &[],
            true
        )
        .is_err()
    );
    assert!(
        validate(
            &request(1),
            &response(1, &control, &payload, &stage),
            &control,
            &payload,
            &stage,
            true
        )
        .is_err()
    );
    for (role, bytes) in [
        ("rotary", 1_f32.to_le_bytes().to_vec()),
        ("final_hidden", 0x3f80_u16.to_le_bytes().to_vec()),
        ("input", 0x3f80_u16.to_le_bytes().to_vec()),
    ] {
        let changed = change_stage_data(&stage, role, &bytes);
        wire::validate_stage_capture(&changed).unwrap(); // Still valid standalone diagnostic, wrong composition join.
        assert!(
            validate(
                &request(0),
                &response(0, &control, &payload, &changed),
                &control,
                &payload,
                &changed,
                true
            )
            .is_err()
        );
    }
    let mut malformed: serde_json::Value = serde_json::from_slice(&stage).unwrap();
    malformed["numerical_acceptance"] = true.into();
    let malformed = serde_json::to_vec(&malformed).unwrap();
    assert!(
        validate(
            &request(0),
            &response(0, &control, &payload, &malformed),
            &control,
            &payload,
            &malformed,
            true
        )
        .is_err()
    );
}

#[test]
fn actual_wire_annex_roundtrip_reaches_parent_validator_without_decimal_main_array() {
    let control = control();
    let payload = vec![0; old::OBSERVATION_BYTES];
    let stage = stage_fixture();
    let expected = response(0, &control, &payload, &stage);
    let mut bytes = Vec::new();
    let mut sent = wire::FrameBudget::new();
    wire::write_response(
        &mut bytes,
        &mut sent,
        &expected,
        Some(&control),
        &payload,
        &stage,
    )
    .unwrap();
    let mut received = wire::FrameBudget::new();
    let (decoded, actual_control, main, annex) =
        wire::read_response(&mut bytes.as_slice(), &mut received)
            .unwrap()
            .unwrap();
    assert_eq!(sent.used(), received.used());
    assert_eq!(decoded, expected);
    validate(
        &request(0),
        &decoded,
        &actual_control.unwrap(),
        &main,
        &annex,
        true,
    )
    .unwrap();
}

#[test]
fn close_joins_four_forward_chain_and_refuses_all_payloads_and_early_counts() {
    let mut request = request(3);
    request.id = 5;
    request.command = wire::Command::Close;
    let chain = [7; 32];
    let payload = vec![0; old::OBSERVATION_BYTES];
    let mut response = response(3, &control(), &payload, &[]);
    response.id = 5;
    response.native_closed = true;
    response.event = wire::Event::Closed {
        completed_forwards: 4,
        transcript_sha256: chain,
    };
    validate_close(&request, &response, None, &[], &[], chain).unwrap();
    assert!(validate_close(&request, &response, None, &[], &[1], chain).is_err());
    assert!(validate_close(&request, &response, Some(&control()), &[], &[], chain).is_err());
    assert!(validate_close(&request, &response, None, &[], &[], [8; 32]).is_err());
    response.event = wire::Event::Closed {
        completed_forwards: 2,
        transcript_sha256: chain,
    };
    assert!(validate_close(&request, &response, None, &[], &[], chain).is_err());
}

#[test]
fn absent_opt_in_refuses_before_process_files_or_model() {
    assert!(run(config(), false).err().unwrap().contains("opt-in"));
}
