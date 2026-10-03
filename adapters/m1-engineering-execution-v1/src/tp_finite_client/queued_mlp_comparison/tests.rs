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
        schema: "FerricFiniteQueuedMlpComparisonRequestV1".into(),
        source: "/task/model".into(),
        worker: pin.clone(),
        images: ImagePins {
            prefix: pin.clone(),
            mlp: pin.clone(),
            residual: pin,
            tail: FilePin {
                path: "/task/wave-v3.hsaco".into(),
                bytes: 112_872,
                sha256: digest(WAVE_IMAGE_SHA),
            },
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
        norm_image: FilePin {
            path: "/task/norm-v15.hsaco".into(),
            bytes: wire::NORM_IMAGE_BYTES as u64,
            sha256: wire::NORM_IMAGE_SHA256,
        },
        evidence_directory: "/task/queued-comparison-evidence".into(),
        dispatch_timeout_ms: 10_000,
        child_deadline_ms: 3_600_000,
    }
}
fn request() -> wire::Request {
    wire::Request {
        protocol: wire::PROTOCOL,
        id: 1,
        device_ids: [7, 9],
        session: [3; 32],
        registration: [4; 32],
        profile_sha256: [5; 32],
        command: wire::Command::Forward {
            generation: 1,
            token: wire::TOKEN,
            cache_metadata: std::iter::once(0).chain(0..144).collect(),
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

// Synthetic custody fixture only, not an independent numerical implementation.
fn report(control: &wire::Control) -> wire::ComparisonReport {
    wire::ComparisonReport {
        schema: "FerricFiniteQueuedMlpComparisonV1".into(),
        generation: 1,
        position: 0,
        layer: 0,
        finite_states: control.layers[0].mlp_states,
        finite_queue_host_ns: control.layers[0].paired_ns[2],
        queued_stage_host_ns: [[17, 23]; 5],
        equality: core::array::from_fn(|stage| {
            core::array::from_fn(|rank| wire::OutputEquality {
                stage: wire::Stage::ALL[stage],
                rank: rank as u32,
                bytes: [8192, 12288, 12288, 12288, 16384][stage],
                words: [4096, 6144, 6144, 6144, 4096][stage],
                sha256: [stage as u8 + rank as u8 + 1; 32],
            })
        }),
        queued_semantic_state: false,
        numerical_acceptance: false,
        performance_claim: false,
        production_authority: false,
    }
}
fn response(control: &wire::Control, payload: &[u8], comparison: &[u8]) -> wire::Response {
    let request = request();
    let capture = old::Payload::from_bytes(payload).unwrap();
    let mut completed = wire::Completion {
        generation: 1,
        position: 0,
        input_token: wire::TOKEN,
        output_token: 0,
        control: old::part(&control.encode()),
        observation: capture.total,
        capture,
        comparison: old::part(comparison),
        chain: [0; 32],
    };
    completed.chain =
        wire::Chain::new(request.registration, request.profile_sha256).advance(&completed);
    wire::Response {
        protocol: request.protocol,
        id: request.id,
        device_ids: request.device_ids,
        session: request.session,
        registration: request.registration,
        profile_sha256: request.profile_sha256,
        event: wire::Event::Completed(completed),
        native_closed: false,
        gpu_execution: true,
        numerical_acceptance: false,
        performance_claim: false,
        production_authority: false,
    }
}
fn validate(
    response: &wire::Response,
    control: &wire::Control,
    payload: &[u8],
    comparison: &[u8],
) -> Result<u32> {
    let request = request();
    validate_completion(
        &request,
        response,
        control,
        payload,
        comparison,
        &mut wire::Chain::new(request.registration, request.profile_sha256),
    )
    .map(|c| c.output_token)
}

#[test]
fn distinct_config_requires_exact_norm_and_wave_images_before_open() {
    let original = config();
    original.validate().unwrap();
    Config::parse(&serde_json::to_vec(&original).unwrap()).unwrap();
    for mutation in 0..12 {
        let mut bad = original.clone();
        match mutation {
            0 => bad.schema = "FerricFiniteRearmSmokeRequestV1".into(),
            1 => bad.norm_image.sha256[0] ^= 1,
            2 => bad.norm_image.bytes -= 1,
            3 => bad.images.tail.sha256[0] ^= 1,
            4 => bad.images.tail.bytes = 115_432,
            5 => bad.device_ids[1] = bad.device_ids[0],
            6 => bad.prompt.tokens.sha256[0] ^= 1,
            7 => bad.expected_model_id = [0; 32],
            8 => bad.dispatch_timeout_ms = 10_001,
            9 => bad.child_deadline_ms = 3_600_001,
            10 => bad.evidence_directory = "relative".into(),
            _ => bad.session = [0; 32],
        }
        assert!(bad.validate().is_err(), "mutation {mutation}");
    }
}

#[test]
fn request_serde_rejects_unknown_missing_duplicate_and_wrong_typed_fields() {
    let original = serde_json::to_value(config()).unwrap();
    for field in [
        "capture_layer0",
        "token",
        "forwards",
        "fallback",
        "input_mode",
    ] {
        let mut bad = original.clone();
        bad[field] = 1.into();
        assert!(Config::parse(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
    for mutation in 0..5 {
        let mut bad = original.clone();
        match mutation {
            0 => {
                bad.as_object_mut().unwrap().remove("norm_image");
            }
            1 => bad["norm_image"]["unreviewed"] = true.into(),
            2 => bad["device_ids"] = serde_json::json!([7, 9, 11]),
            3 => bad["child_deadline_ms"] = true.into(),
            _ => bad["prompt"]["tokens"]["bytes"] = (-1).into(),
        }
        assert!(Config::parse(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
    let text =
        serde_json::to_string(&original)
            .unwrap()
            .replacen('{', "{\"schema\":\"duplicate\",", 1);
    assert!(Config::parse(text.as_bytes()).is_err());
    assert!(Config::parse(&[]).is_err());
    assert!(Config::parse(&vec![b' '; 65_537]).is_err());
}

#[test]
fn one_authentic_token_and_no_early_close_or_second_forward() {
    assert_eq!(input_token(0).unwrap(), 9112);
    assert!(input_token(1).is_err());
    let mut value = request();
    value.validate().unwrap();
    value.command = wire::Command::Close;
    assert!(value.validate().is_err());
    value.id = 2;
    value.validate().unwrap();
    let mut value = request();
    value.id = 2;
    assert!(value.validate().is_err());
    if let wire::Command::Forward { token, .. } = &mut value.command {
        *token = 2190;
    }
    assert!(value.validate().is_err());
}

#[test]
fn actual_prepared_position_zero_metadata_and_rotary_are_used() {
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
        .open_sequence(scope, &[wire::TOKEN], 0)
        .unwrap()
        .sequence();
    let batch = pool
        .reserve_batch(&[EngineeringTpPageRowV1 {
            sequence,
            token: wire::TOKEN,
            position: 0,
        }])
        .unwrap();
    let prepared = metadata(&batch, [4; 32], 1_000_000).unwrap();
    assert_eq!(prepared.cache_metadata()[0], 0);
    assert_eq!(
        prepared.cache_metadata()[1..].to_vec(),
        (0..144).collect::<Vec<_>>()
    );
    let (mut expected, sin) = crate::tp_execution::rope_bytes(0, 1_000_000);
    expected.extend(sin);
    assert_eq!(
        prepared
            .rotary()
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<_>>(),
        expected
    );
    assert!(metadata(&batch, [4; 32], 10_000).is_err());
    pool.begin_submission(&batch).unwrap();
    // CPU-only fixture completion, not a native execution claim.
    pool.commit_batch(
        &batch,
        EngineeringTpBatchCompletionV1::after_all_ranks(&batch),
    )
    .unwrap();
    let second = pool
        .reserve_batch(&[EngineeringTpPageRowV1 {
            sequence,
            token: 2190,
            position: 1,
        }])
        .unwrap();
    assert!(metadata(&second, [4; 32], 1_000_000).is_err());
}

#[test]
fn comparison_requires_genuine_control_states_and_finite_timing_join() {
    let control = control();
    let payload = vec![0; old::OBSERVATION_BYTES];
    let original = report(&control);
    let bytes = serde_json::to_vec(&original).unwrap();
    assert_eq!(
        validate(
            &response(&control, &payload, &bytes),
            &control,
            &payload,
            &bytes
        )
        .unwrap(),
        0
    );
    for mutation in 0..11 {
        let mut bad = original.clone();
        match mutation {
            0 => bad.finite_states[0][10] = 63,
            1 => bad.finite_queue_host_ns[1] = 1,
            2 => bad.queued_semantic_state = true,
            3 => bad.numerical_acceptance = true,
            4 => bad.performance_claim = true,
            5 => bad.production_authority = true,
            6 => bad.equality[4][1].words *= 2,
            7 => bad.equality[0][0].rank = 1,
            8 => bad.equality[2][0].stage = wire::Stage::Gate,
            9 => bad.equality[0][0].sha256 = [0; 32],
            _ => bad.layer = 1,
        }
        let bytes = serde_json::to_vec(&bad).unwrap();
        assert!(
            validate(
                &response(&control, &payload, &bytes),
                &control,
                &payload,
                &bytes
            )
            .is_err(),
            "mutation {mutation}"
        );
    }
    // Queued measurements may differ; they must never overwrite the finite pair.
    let mut changed = original;
    changed.queued_stage_host_ns = [[u64::MAX, 0]; 5];
    changed.validate(&control).unwrap();
}

#[test]
fn bounded_report_rejects_missing_unknown_and_duplicate_fields() {
    let control = control();
    let bytes = serde_json::to_vec(&report(&control)).unwrap();
    assert!(bytes.len() < wire::COMPARISON_JSON_BYTES);
    assert!(wire::validate_comparison(&[], &control).is_err());
    assert!(
        wire::validate_comparison(&vec![b' '; wire::COMPARISON_JSON_BYTES + 1], &control).is_err()
    );
    let mut unknown: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    unknown["accepted"] = true.into();
    assert!(wire::validate_comparison(&serde_json::to_vec(&unknown).unwrap(), &control).is_err());
    let text = String::from_utf8(bytes)
        .unwrap()
        .replacen('{', "{\"generation\":1,", 1);
    assert!(wire::validate_comparison(text.as_bytes(), &control).is_err());
}

#[test]
fn full_forward_capture_requires_all_layers_finite_values_and_own_argmax() {
    let control = control();
    let payload = vec![0; old::OBSERVATION_BYTES];
    let bytes = serde_json::to_vec(&report(&control)).unwrap();
    let original = response(&control, &payload, &bytes);
    let mut bad = original.clone();
    bad.profile_sha256[0] ^= 1;
    assert!(validate(&bad, &control, &payload, &bytes).is_err());
    let mut incomplete = control.clone();
    incomplete.layers[35].prefix_states[1][3] &= !0x8000;
    assert!(
        validate(
            &response(&incomplete, &payload, &bytes),
            &incomplete,
            &payload,
            &bytes
        )
        .is_err()
    );
    let mut not_a_number = payload.clone();
    not_a_number[..2].copy_from_slice(&0x7fc1_u16.to_le_bytes());
    assert!(
        validate(
            &response(&control, &not_a_number, &bytes),
            &control,
            &not_a_number,
            &bytes
        )
        .is_err()
    );
    let mut wrong_argmax = payload.clone();
    let at = old::OBSERVATION_BYTES - 303_872 + 2;
    wrong_argmax[at..at + 2].copy_from_slice(&0x3f80_u16.to_le_bytes());
    assert!(
        validate(
            &response(&control, &wrong_argmax, &bytes),
            &control,
            &wrong_argmax,
            &bytes
        )
        .is_err()
    );
}

#[test]
fn transcript_binds_comparison_and_refuses_replay() {
    let control = control();
    let payload = vec![0; old::OBSERVATION_BYTES];
    let original = report(&control);
    let bytes = serde_json::to_vec(&original).unwrap();
    let good = response(&control, &payload, &bytes);
    let mut changed = original;
    changed.queued_stage_host_ns[0][0] += 1;
    let changed = serde_json::to_vec(&changed).unwrap();
    wire::validate_comparison(&changed, &control).unwrap();
    assert!(validate(&good, &control, &payload, &changed).is_err());
    let mut bad = good.clone();
    if let wire::Event::Completed(c) = &mut bad.event {
        c.comparison = old::part(&changed);
    }
    assert!(validate(&bad, &control, &payload, &changed).is_err());
    let request = request();
    let mut chain = wire::Chain::new(request.registration, request.profile_sha256);
    validate_completion(&request, &good, &control, &payload, &bytes, &mut chain).unwrap();
    assert!(validate_completion(&request, &good, &control, &payload, &bytes, &mut chain).is_err());
}

#[test]
fn shared_wire_roundtrip_retains_typed_report_without_decimal_main_arrays() {
    let control = control();
    let payload = vec![0; old::OBSERVATION_BYTES];
    let comparison = serde_json::to_vec(&report(&control)).unwrap();
    let expected = response(&control, &payload, &comparison);
    let mut bytes = Vec::new();
    let mut sent = wire::FrameBudget::new();
    wire::write_response(
        &mut bytes,
        &mut sent,
        &expected,
        Some(&control),
        &payload,
        &comparison,
    )
    .unwrap();
    let mut received = wire::FrameBudget::new();
    let (actual, control, main, report) = wire::read_response(&mut bytes.as_slice(), &mut received)
        .unwrap()
        .unwrap();
    assert_eq!(actual, expected);
    assert_eq!(sent.used(), received.used());
    validate(&actual, &control.unwrap(), &main, &report).unwrap();
    assert!(
        wire::read_response(
            &mut &bytes[..bytes.len() - 1],
            &mut wire::FrameBudget::new()
        )
        .is_err()
    );
}

#[test]
fn close_is_exact_one_forward_and_refuses_report_control_or_capture() {
    let mut request = request();
    request.id = 2;
    request.command = wire::Command::Close;
    let control = control();
    let payload = vec![0; old::OBSERVATION_BYTES];
    let report = serde_json::to_vec(&report(&control)).unwrap();
    let mut response = response(&control, &payload, &report);
    response.id = 2;
    response.native_closed = true;
    response.event = wire::Event::Closed {
        completed_forwards: 1,
        transcript_sha256: [7; 32],
    };
    validate_close(&request, &response, None, &[], &[], [7; 32]).unwrap();
    assert!(validate_close(&request, &response, Some(&control), &[], &[], [7; 32]).is_err());
    assert!(validate_close(&request, &response, None, &[1], &[], [7; 32]).is_err());
    assert!(validate_close(&request, &response, None, &[], &report, [7; 32]).is_err());
    assert!(validate_close(&request, &response, None, &[], &[], [8; 32]).is_err());
    response.event = wire::Event::Closed {
        completed_forwards: 0,
        transcript_sha256: [7; 32],
    };
    assert!(validate_close(&request, &response, None, &[], &[], [7; 32]).is_err());
}

#[test]
fn absent_opt_in_refuses_before_process_files_or_model() {
    assert!(run(config(), false).err().unwrap().contains("opt-in"));
}

#[test]
fn exclusive_evidence_publication_requires_complete_close_and_reap_flags() {
    struct Temp(PathBuf);
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    static SERIAL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let temp = Temp(std::env::temp_dir().canonicalize().unwrap().join(format!(
        "finite-comparison-publication-{}-{}",
        std::process::id(),
        SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    )));
    std::fs::create_dir(&temp.0).unwrap();
    let mut config = config();
    config.evidence_directory = temp.0.join("run");
    let mut writer = evidence::Evidence::create(&config.evidence_directory).unwrap();
    let request = request();
    let control = control();
    let payload = vec![0; old::OBSERVATION_BYTES];
    let report = serde_json::to_vec(&report(&control)).unwrap();
    let response = response(&control, &payload, &report);
    writer
        .append(&request, &response, &control, &payload, &report)
        .unwrap();
    assert!(
        writer
            .append(&request, &response, &control, &payload, &report)
            .is_err()
    );
    let files = writer.finish(&[]).unwrap();
    assert_eq!(files.frames[0].comparison.sha256, old::part(&report).sha256);
    assert_eq!(files.frames[0].request, request);
    let wire::Event::Completed(c) = &response.event else {
        unreachable!()
    };
    let chain = c.chain;
    let mut close = response.clone();
    close.id = 2;
    close.native_closed = true;
    close.event = wire::Event::Closed {
        completed_forwards: 1,
        transcript_sha256: chain,
    };
    // This fixture checks publication policy only; no process or GPU was opened.
    let mut observation = Observation {
        schema: "FerricFiniteQueuedMlpComparisonObservationV1",
        request: config.clone(),
        child_pid: 123,
        registration_sha256: request.registration,
        source_program_sha256: [6; 32],
        upload_manifest_sha256: [7; 32],
        bootstrap: wire::Bootstrap {
            protocol: wire::PROTOCOL,
            profile: wire::Profile::FiniteThenQueuedLayerZeroV1,
            device_ids: config.device_ids,
            scope: setup_wire::Scope {
                bundle_id: config.expected_bundle_id,
                model_id: config.expected_model_id,
                session: config.session,
                pool_identity: 1,
                group_id: 0,
                child_identity: 123,
            },
            timeout_ms: config.dispatch_timeout_ms,
            token: wire::TOKEN,
            norm_image: setup_wire::Part {
                bytes: wire::NORM_IMAGE_BYTES as u32,
                sha256: wire::NORM_IMAGE_SHA256,
            },
        },
        profile_sha256: request.profile_sha256,
        setup_commands: 1,
        completed_forwards: 1,
        input_tokens: INPUT_TOKENS,
        observed_output_tokens: vec![0],
        page_permutation: (0..144).collect(),
        transcript_sha256: chain,
        request_stream_bytes: 0,
        response_stream_bytes: 0,
        files,
        close,
        child_exit_zero: true,
        process_group_absent: false,
        native_closed: true,
        gpu_execution: true,
        numerical_acceptance: false,
        performance_claim: false,
        production_authority: false,
        full_long_workload: false,
        queued_semantic_state: false,
    };
    assert!(evidence::publish(&mut observation).is_err());
    assert!(!config.evidence_directory.join("complete.json").exists());
    observation.process_group_absent = true;
    observation.numerical_acceptance = true;
    assert!(evidence::publish(&mut observation).is_err());
    observation.numerical_acceptance = false;
    observation.close.native_closed = false;
    assert!(evidence::publish(&mut observation).is_err());
    observation.close.native_closed = true;
    evidence::publish(&mut observation).unwrap();
    let summary = std::fs::read(config.evidence_directory.join("complete.json")).unwrap();
    assert_eq!(summary.len() as u64, observation.files.summary_bytes);
    assert_eq!(
        observation.files.total_bytes,
        observation.files.bytes_before_summary + summary.len() as u64
    );
    assert!(!config.evidence_directory.join("complete.pending").exists());
    assert!(evidence::publish(&mut observation).is_err());
    assert_eq!(
        std::fs::read(config.evidence_directory.join("complete.json")).unwrap(),
        summary
    );
}
