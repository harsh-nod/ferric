use super::*;

#[test]
fn tiles_cached_request_is_explicit_and_does_not_change_legacy_serialization() {
    let mut c = config();
    let raw = serde_json::to_vec(&c).unwrap();
    assert!(
        !String::from_utf8(raw.clone())
            .unwrap()
            .contains("kernel_admission")
    );
    assert_eq!(
        Config::parse(&raw).unwrap().kernel_admission,
        wire::KernelAdmission::Baseline
    );
    for mode in [
        wire::InputMode::TeacherForced,
        wire::InputMode::Autoregressive,
    ] {
        c.mode = mode;
        c.kernel_admission = wire::KernelAdmission::CachedImmutable;
        assert!(c.validate().is_err());
        c.schema = "FerricFiniteTilesDecodeCachedAdmissionRequestV1".into();
        c.validate().unwrap();
        let value = serde_json::to_value(&c).unwrap();
        assert_eq!(value["kernel_admission"], "cached_immutable");
        Config::parse(&serde_json::to_vec(&value).unwrap()).unwrap();
        let mut missing = value.clone();
        missing.as_object_mut().unwrap().remove("kernel_admission");
        assert!(Config::parse(&serde_json::to_vec(&missing).unwrap()).is_err());
        for bad in ["operational", "shared_full", "cached", "baseline"] {
            let mut bad_value = value.clone();
            bad_value["kernel_admission"] = bad.into();
            assert!(Config::parse(&serde_json::to_vec(&bad_value).unwrap()).is_err());
        }
        let mut extra = value.clone();
        extra["operational_currentness"] = true.into();
        assert!(Config::parse(&serde_json::to_vec(&extra).unwrap()).is_err());
        c.kernel_admission = wire::KernelAdmission::Baseline;
        assert!(c.validate().is_err());
        c.schema = "FerricFiniteTilesDecodeRequestV1".into();
    }
}

#[test]
fn tiles_parent_requires_exact_applied_policy_for_every_profile() {
    let c = control();
    let payload = vec![0; old::OBSERVATION_BYTES];
    let mut r = response(0, &c, &payload);
    let baseline = wire::KernelAdmission::Baseline.profile();
    let cached = wire::KernelAdmission::CachedImmutable.profile();
    applied_admission(baseline, &r).unwrap();
    assert!(applied_admission(cached, &r).is_err());
    r.applied_admission = wire::KernelAdmission::CachedImmutable.receipt();
    applied_admission(cached, &r).unwrap();
    assert!(applied_admission(baseline, &r).is_err());
    for field in 0..3 {
        let mut receipt = wire::KernelAdmission::CachedImmutable.receipt().unwrap();
        match field {
            0 => receipt.cache_kernel_admission = false,
            1 => receipt.operational_currentness = true,
            _ => receipt.shared_full_currentness = true,
        }
        r.applied_admission = Some(receipt);
        assert!(applied_admission(cached, &r).is_err());
    }
}

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
        schema: "FerricFiniteTilesDecodeRequestV1".into(),
        source: "/task/model".into(),
        worker: pin.clone(),
        images: ImagePins {
            prefix: pin.clone(),
            mlp: pin.clone(),
            residual: pin.clone(),
            tail: pin.clone(),
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
        mode: wire::InputMode::TeacherForced,
        kernel_admission: wire::KernelAdmission::Baseline,
        tiles_image: pin,
        evidence_directory: "/task/tiles-evidence".into(),
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
    let mut mlp = [0; 548];
    mlp[..4].copy_from_slice(&[1, 0, 0, 31]);
    mlp[4..9].copy_from_slice(&[1, 96, 96, 1, 64]);
    mlp[9..14].copy_from_slice(&[1, 96, 96, 1, 64]);
    mlp[14..23].fill(u32::MAX);
    mlp[22] = 3;
    mlp[23..32].fill(u32::MAX);
    mlp[31] = 3;
    mlp[32..290].fill(1);
    mlp[290..].fill(64);
    wire::Control {
        embedding_ns: [0; 2],
        layers: core::array::from_fn(|_| wire::LayerObservation {
            prefix_states: [prefix; 2],
            tiles_states: [mlp; 2],
            paired_ns: [[0; 2]; 4],
        }),
        tail_ns: [0; 3],
    }
}
fn response(position: u32, control: &wire::Control, payload: &[u8]) -> wire::Response {
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
        applied_admission: None,
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
fn tiles_parent_config_is_closed_explicit_and_bounds_image() {
    let original = config();
    original.validate().unwrap();
    for mode in [
        wire::InputMode::TeacherForced,
        wire::InputMode::Autoregressive,
    ] {
        let mut c = original.clone();
        c.mode = mode;
        Config::parse(&serde_json::to_vec(&c).unwrap()).unwrap();
    }
    for change in 0..9 {
        let mut bad = original.clone();
        match change {
            0 => bad.schema = "FerricFiniteRearmSmokeRequestV1".into(),
            1 => bad.device_ids[1] = bad.device_ids[0],
            2 => bad.prompt.tokens.sha256[0] ^= 1,
            3 => bad.dispatch_timeout_ms = 10001,
            4 => bad.tiles_image.bytes = 0,
            5 => bad.tiles_image.bytes = wire::MAX_IMAGE_BYTES as u64 + 1,
            6 => bad.tiles_image.sha256 = [0; 32],
            7 => bad.tiles_image.path = "relative".into(),
            _ => bad.evidence_directory = "relative".into(),
        }
        assert!(bad.validate().is_err());
    }
    for field in ["mode", "tiles_image"] {
        let mut value = serde_json::to_value(&original).unwrap();
        value.as_object_mut().unwrap().remove(field);
        assert!(Config::parse(&serde_json::to_vec(&value).unwrap()).is_err());
    }
    for field in ["capture_layer0", "input_tokens", "forwards", "fallback"] {
        let mut value = serde_json::to_value(&original).unwrap();
        value[field] = true.into();
        assert!(Config::parse(&serde_json::to_vec(&value).unwrap()).is_err());
    }
}
fn bootstrap(mode: wire::InputMode) -> wire::Bootstrap {
    wire::Bootstrap {
        protocol: 1,
        profile: wire::Profile::TilesDecodeFourForwardV1,
        device_ids: [7, 9],
        scope: setup_wire::Scope {
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
        input_tokens: if mode == wire::InputMode::TeacherForced {
            INPUT_TOKENS.to_vec()
        } else {
            vec![INPUT_TOKENS[0]]
        },
        tiles_image: old::part(&[1]),
    }
}
#[test]
fn tiles_parent_file_backed_image_is_retained_for_bootstrap_in_both_modes() {
    struct Temp(PathBuf);
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    static SERIAL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let temp = Temp(std::env::temp_dir().canonicalize().unwrap().join(format!(
        "tiles-image-{}-{}",
        std::process::id(),
        SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    )));
    std::fs::create_dir(&temp.0).unwrap();
    let bytes = [0x7f, b'E', b'L', b'F', 0, 255, 37];
    let path = temp.0.join("image.hsaco");
    std::fs::write(&path, bytes).unwrap();
    let mut c = config();
    c.tiles_image = FilePin {
        path,
        bytes: bytes.len() as u64,
        sha256: hash(&bytes),
    };
    for mode in [
        wire::InputMode::TeacherForced,
        wire::InputMode::Autoregressive,
    ] {
        c.mode = mode;
        let retained = c.read_tiles_image().unwrap();
        assert_eq!(retained, bytes);
        let mut expected = bootstrap(mode);
        expected.tiles_image = old::part(&retained);
        assert_eq!(expected.tiles_image.bytes as u64, c.tiles_image.bytes);
        assert_eq!(expected.tiles_image.sha256, c.tiles_image.sha256);
        let mut framed = Vec::new();
        wire::write_bootstrap(
            &mut framed,
            &mut wire::FrameBudget::new(),
            &expected,
            &retained,
        )
        .unwrap();
        let (observed, body) =
            wire::read_bootstrap(&mut framed.as_slice(), &mut wire::FrameBudget::new())
                .unwrap()
                .unwrap();
        assert_eq!(observed, expected);
        assert_eq!(body, bytes);
        assert!(
            c.tiles_image
                .read(wire::MAX_IMAGE_BYTES as u64, false)
                .unwrap()
                .is_empty()
        );
    }
    let original = c.tiles_image.clone();
    c.tiles_image.sha256[0] ^= 1;
    assert!(c.read_tiles_image().is_err());
    c.tiles_image = original.clone();
    c.tiles_image.bytes += 1;
    assert!(c.read_tiles_image().is_err());
    c.tiles_image = original;
    std::fs::write(&c.tiles_image.path, [0; 7]).unwrap();
    assert!(c.read_tiles_image().is_err());
}
#[test]
fn tiles_parent_real_pool_carries_history_in_both_input_modes() {
    for mode in [
        wire::InputMode::TeacherForced,
        wire::InputMode::Autoregressive,
    ] {
        let b = bootstrap(mode);
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
        let mut previous = None;
        for position in 0..4 {
            let token = b.input(position, previous).unwrap();
            assert_eq!(
                token,
                if mode == wire::InputMode::TeacherForced {
                    INPUT_TOKENS[position as usize]
                } else if position == 0 {
                    9112
                } else {
                    67
                }
            );
            let batch = pool
                .reserve_batch(&[EngineeringTpPageRowV1 {
                    sequence,
                    token,
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
            // CPU-only commit fixture; never a native completion assertion.
            pool.commit_batch(
                &batch,
                EngineeringTpBatchCompletionV1::after_all_ranks(&batch),
            )
            .unwrap();
            previous = Some(67);
        }
        assert_eq!(pool.committed_position(sequence).unwrap(), 4);
        let mut changed = [0; 145];
        changed[1..].copy_from_slice(&stable.unwrap());
        changed.swap(1, 2);
        assert!(stable_pages(&mut stable, &changed).is_err());
    }
}
#[test]
fn tiles_parent_every_layer_and_rank_terminal_is_required_before_evidence() {
    let c = control();
    let main = vec![0; old::OBSERVATION_BYTES];
    for position in 0..4 {
        let r = request(position);
        let original = response(position, &c, &main);
        assert_eq!(validate(&r, &original, &c, &main).unwrap(), 0);
        let mut bad = original.clone();
        bad.profile_sha256[0] ^= 1;
        assert!(validate(&r, &bad, &c, &main).is_err());
        let mut bad = c.clone();
        bad.layers[35].tiles_states[1][547] = 63;
        assert!(validate(&r, &response(position, &bad, &main), &bad, &main).is_err());
        let mut bad = c.clone();
        bad.layers[35].prefix_states[1][21] = 63;
        assert!(validate(&r, &response(position, &bad, &main), &bad, &main).is_err());
    }
}
#[test]
fn tiles_parent_capture_finiteness_argmax_and_chain_are_mandatory() {
    let c = control();
    let mut main = vec![0; old::OBSERVATION_BYTES];
    main[..2].copy_from_slice(&0x7f80u16.to_le_bytes());
    assert!(validate(&request(0), &response(0, &c, &main), &c, &main).is_err());
    main.fill(0);
    let at = old::OBSERVATION_BYTES - 303872 + 2;
    main[at..at + 2].copy_from_slice(&0x3f80u16.to_le_bytes());
    assert!(validate(&request(0), &response(0, &c, &main), &c, &main).is_err());
    main.fill(0);
    let mut child = wire::Chain::new([4; 32], [5; 32]);
    let mut parent = wire::Chain::new([4; 32], [5; 32]);
    for pos in 0..4 {
        let r = request(pos);
        let mut reply = response(pos, &c, &main);
        let wire::Event::Completed(v) = &mut reply.event else {
            unreachable!()
        };
        v.chain = child.advance(v);
        validate_completion(&r, &reply, &c, &main, &mut parent).unwrap();
        assert!(
            validate_completion(
                &r,
                &reply,
                &c,
                &main,
                &mut wire::Chain::new([9; 32], [5; 32])
            )
            .is_err()
        );
    }
    assert_eq!(parent.digest(), child.digest());
}
#[test]
fn tiles_parent_close_requires_exact_four_chain_and_empty_bodies() {
    let c = control();
    let main = vec![0; old::OBSERVATION_BYTES];
    let mut r = request(0);
    r.id = 5;
    r.command = wire::Command::Close;
    let mut reply = response(0, &c, &main);
    reply.id = 5;
    reply.native_closed = true;
    reply.event = wire::Event::Closed {
        completed_forwards: 4,
        transcript_sha256: [8; 32],
    };
    validate_close(&r, &reply, None, &[], [8; 32]).unwrap();
    assert!(validate_close(&r, &reply, Some(&c), &[], [8; 32]).is_err());
    assert!(validate_close(&r, &reply, None, &main, [8; 32]).is_err());
    assert!(validate_close(&r, &reply, None, &[], [9; 32]).is_err());
    reply.native_closed = false;
    assert!(validate_close(&r, &reply, None, &[], [8; 32]).is_err());
}
#[test]
fn tiles_parent_actual_wire_decodes_typed_control_and_checks_custody() {
    let c = control();
    let main = vec![0; old::OBSERVATION_BYTES];
    let expected = response(0, &c, &main);
    let mut bytes = Vec::new();
    wire::write_response(
        &mut bytes,
        &mut wire::FrameBudget::new(),
        &expected,
        Some(&c),
        &main,
    )
    .unwrap();
    let (reply, decoded, payload) =
        wire::read_response(&mut bytes.as_slice(), &mut wire::FrameBudget::new())
            .unwrap()
            .unwrap();
    assert_eq!(decoded.as_ref(), Some(&c));
    validate(&request(0), &reply, decoded.as_ref().unwrap(), &payload).unwrap();
    let mut legacy = serde_json::to_value(&expected).unwrap();
    legacy["event"]["stage_capture"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<wire::Response>(legacy).is_err());
}
#[test]
fn tiles_parent_missing_optin_refuses_before_any_file_or_device_open() {
    assert!(run(config(), false).err().unwrap().contains("opt-in"));
}
#[test]
fn tiles_parent_four_request_pins_binary_controls_and_summary_fit_closed_census() {
    for admission in [
        wire::KernelAdmission::Baseline,
        wire::KernelAdmission::CachedImmutable,
    ] {
        summary_fits(admission);
    }
}
fn summary_fits(admission: wire::KernelAdmission) {
    struct Temp(PathBuf);
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    static SERIAL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let temp = Temp(std::env::temp_dir().canonicalize().unwrap().join(format!(
        "tiles-summary-{}-{}",
        std::process::id(),
        SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    )));
    std::fs::create_dir(&temp.0).unwrap();
    let directory = temp.0.join("evidence");
    let mut e = evidence::Evidence::create(&directory).unwrap();
    let mut c = control();
    c.embedding_ns = [u64::MAX; 2];
    c.tail_ns = [u64::MAX; 3];
    for layer in &mut c.layers {
        layer.paired_ns = [[u64::MAX; 2]; 4];
    }
    let main = vec![0; old::OBSERVATION_BYTES];
    let mut chain = wire::Chain::new([4; 32], [5; 32]);
    for pos in 0..4 {
        let r = request(pos);
        let mut reply = response(pos, &c, &main);
        reply.applied_admission = admission.receipt();
        let wire::Event::Completed(v) = &mut reply.event else {
            unreachable!()
        };
        v.chain = chain.advance(v);
        e.append(&r, &reply, &c, &main).unwrap();
    }
    let files = e.finish(&vec![0; 2 << 20]).unwrap();
    assert_eq!(files.frames.len(), 4);
    for (pos, frame) in files.frames.iter().enumerate() {
        assert_eq!(frame.control.bytes, 166504);
        assert_eq!(frame.observation.bytes, 606976);
        let raw = frame.request.read(16 << 10, true).unwrap();
        let r: wire::Request = serde_json::from_slice(&raw).unwrap();
        assert_eq!(r, request(pos as u32));
    }
    let mut cfg = config();
    cfg.kernel_admission = admission;
    if !admission.is_baseline() {
        cfg.schema = "FerricFiniteTilesDecodeCachedAdmissionRequestV1".into();
    }
    cfg.evidence_directory = directory.clone();
    let mut close = response(0, &c, &main);
    close.applied_admission = admission.receipt();
    close.id = 5;
    close.native_closed = true;
    close.event = wire::Event::Closed {
        completed_forwards: 4,
        transcript_sha256: chain.digest(),
    };
    let mut b = bootstrap(wire::InputMode::TeacherForced);
    b.profile = admission.profile();
    let mut observation = Observation {
        schema: if admission.is_baseline() {
            "FerricFiniteTilesDecodeObservationV1"
        } else {
            "FerricFiniteTilesDecodeCachedAdmissionObservationV1"
        },
        request: cfg,
        child_pid: 17,
        registration_sha256: [4; 32],
        source_program_sha256: [255; 32],
        upload_manifest_sha256: [255; 32],
        bootstrap: b,
        profile_sha256: [5; 32],
        setup_commands: 16384,
        completed_forwards: 4,
        input_tokens: INPUT_TOKENS.to_vec(),
        observed_output_tokens: vec![0; 4],
        page_permutation: (0..144).collect(),
        transcript_sha256: chain.digest(),
        request_stream_bytes: 64 << 20,
        response_stream_bytes: 64 << 20,
        files,
        close,
        child_exit_zero: true,
        process_group_absent: true,
        native_closed: true,
        gpu_execution: true,
        numerical_acceptance: false,
        performance_claim: false,
        production_authority: false,
        full_long_workload: false,
    };
    assert!(!directory.join("complete.json").exists());
    evidence::publish(&mut observation).unwrap();
    let raw = std::fs::read(directory.join("complete.json")).unwrap();
    assert!(raw.len() <= 65536);
    assert_eq!(observation.files.summary_bytes, raw.len() as u64);
    let actual = std::fs::read_dir(&directory)
        .unwrap()
        .map(|entry| entry.unwrap().metadata().unwrap().len())
        .sum::<u64>();
    assert_eq!(actual, observation.files.total_bytes);
    assert!(actual < 8 << 20);
    assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 14);
}
