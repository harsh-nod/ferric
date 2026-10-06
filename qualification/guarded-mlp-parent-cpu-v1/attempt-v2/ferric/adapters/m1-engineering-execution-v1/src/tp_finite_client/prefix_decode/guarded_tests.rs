use super::super::tests::Temp;
use super::*;

fn config() -> GuardedConfig {
    let mut decode = super::super::tests::config();
    for pin in [
        &mut decode.images.prefix,
        &mut decode.images.mlp,
        &mut decode.images.residual,
        &mut decode.images.tail,
    ] {
        pin.sha256 = hash(&[7]);
    }
    decode.tiles_image.sha256 = hash(&[8]);
    decode.prefix_image.sha256 = hash(&[9]);
    GuardedConfig {
        schema: REQUEST_SCHEMA.into(),
        decode,
        projection_image: FilePin {
            path: "/task/projection.hsaco".into(),
            bytes: 1,
            sha256: hash(&[10]),
        },
        guarded_image: FilePin {
            path: "/task/guarded.hsaco".into(),
            bytes: 28440,
            sha256: gwire::GUARDED_IMAGE,
        },
    }
}
fn frame(
    b: &gwire::Bootstrap,
    position: u32,
    previous: Option<u32>,
    chain: &mut gwire::Chain,
) -> (gwire::Request, gwire::Response, gwire::Control, Vec<u8>) {
    let request = gwire::tests::request(b, position, previous);
    let control = gwire::tests::control(u64::from(position) + 1);
    let mut payload = vec![0; old::OBSERVATION_BYTES];
    let output = position + 7;
    payload[37 * 8192 + output as usize * 2..][..2].copy_from_slice(&0x3f80u16.to_le_bytes());
    let gwire::Command::Forward { token, .. } = request.command else {
        panic!("fixture")
    };
    let mut done = gwire::Completion {
        generation: u64::from(position) + 1,
        position,
        input_token: token,
        output_token: output,
        control: old::part(&control.encode()),
        observation: old::part(&payload),
        capture: old::Payload::from_bytes(&payload).unwrap(),
        chain: [0; 32],
    };
    done.chain = chain.advance(&done);
    let reply = gwire::Response {
        schema: gwire::RESPONSE_SCHEMA.into(),
        protocol: gwire::PROTOCOL,
        id: request.id,
        device_ids: request.device_ids,
        session: request.session,
        registration: request.registration,
        profile_sha256: request.profile_sha256,
        event: gwire::Event::Completed(done),
        native_closed: false,
        gpu_execution: true,
        numerical_acceptance: false,
        full_model_acceptance: false,
        performance_claim: false,
        production_authority: false,
    };
    (request, reply, control, payload)
}
fn completed(path: &std::path::Path, mode: wire::InputMode) -> GuardedObservation {
    let mut cfg = config();
    cfg.decode.evidence_directory = path.into();
    cfg.decode.mode = mode;
    let b = bootstrap(wire::tests::bootstrap(mode), &cfg).unwrap();
    let profile = b.sha256().unwrap();
    let mut evidence = evidence::Evidence::create(path).unwrap();
    let mut chain = gwire::Chain::new(b.decode.registration, profile);
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    for position in 0..4 {
        let (request, reply, control, payload) =
            frame(&b, position, outputs.last().copied(), &mut chain);
        let gwire::Event::Completed(c) = &reply.event else {
            panic!("fixture")
        };
        inputs.push(c.input_token);
        outputs.push(c.output_token);
        evidence
            .append(&request, &reply, &control, &payload)
            .unwrap();
    }
    let close = gwire::Response {
        schema: gwire::RESPONSE_SCHEMA.into(),
        protocol: gwire::PROTOCOL,
        id: 5,
        device_ids: b.decode.device_ids,
        session: b.decode.scope.session,
        registration: b.decode.registration,
        profile_sha256: profile,
        event: gwire::Event::Closed {
            completed_forwards: 4,
            transcript_sha256: chain.digest(),
        },
        native_closed: true,
        gpu_execution: true,
        numerical_acceptance: false,
        full_model_acceptance: false,
        performance_claim: false,
        production_authority: false,
    };
    GuardedObservation {
        schema: "FerricFiniteGuardedMlpDecodeObservationV1",
        request: cfg,
        child_pid: b.decode.scope.child_identity,
        registration_sha256: b.decode.registration,
        source_program_sha256: b.decode.begin.source_program.sha256,
        upload_manifest_sha256: b.decode.begin.uploads.sha256,
        bootstrap: b,
        profile_sha256: profile,
        setup_commands: 1,
        completed_forwards: 4,
        input_tokens: inputs,
        observed_output_tokens: outputs,
        page_permutation: (0..144).collect(),
        transcript_sha256: chain.digest(),
        request_stream_bytes: 1,
        response_stream_bytes: 4 * (gwire::CONTROL_BYTES + old::OBSERVATION_BYTES),
        files: evidence.finish(&[]).unwrap(),
        close,
        child_exit_zero: true,
        process_group_absent: true,
        native_closed: true,
        gpu_execution: true,
        numerical_acceptance: false,
        full_model_acceptance: false,
        performance_claim: false,
        production_authority: false,
        full_long_workload: false,
        native_attempts: 1,
        retries: 0,
    }
}
#[test]
fn guarded_parent_request_is_closed_image_bound_and_requires_explicit_opt_in() {
    let cfg = config();
    GuardedConfig::parse(&serde_json::to_vec(&cfg).unwrap()).unwrap();
    assert!(Config::parse(&serde_json::to_vec(&cfg).unwrap()).is_err());
    assert!(GuardedConfig::parse(&serde_json::to_vec(&cfg.decode).unwrap()).is_err());
    for key in [
        "fallback",
        "performance_policy",
        "quantization",
        "speculative",
        "forwards",
    ] {
        let mut value = serde_json::to_value(&cfg).unwrap();
        value[key] = true.into();
        assert!(GuardedConfig::parse(&serde_json::to_vec(&value).unwrap()).is_err());
    }
    for edit in [
        |c: &mut GuardedConfig| c.schema.push('x'),
        |c: &mut GuardedConfig| c.guarded_image.sha256[0] ^= 1,
        |c: &mut GuardedConfig| c.projection_image.bytes = 0,
        |c: &mut GuardedConfig| c.guarded_image.path = "relative".into(),
        |c: &mut GuardedConfig| c.projection_image.sha256 = c.decode.images.residual.sha256,
    ] {
        let mut bad = cfg.clone();
        edit(&mut bad);
        assert!(bad.validate().is_err());
    }
    assert!(run_guarded(cfg, false).is_err());
}
#[test]
fn guarded_parent_retains_tf4_and_own_output_ar4_without_model_or_performance_claims() {
    let temp = Temp::new();
    for (i, mode) in [
        wire::InputMode::TeacherForced,
        wire::InputMode::Autoregressive,
    ]
    .into_iter()
    .enumerate()
    {
        let mut observation = completed(&temp.0.join(i.to_string()), mode);
        if mode == wire::InputMode::Autoregressive {
            assert_eq!(observation.input_tokens, [9112, 7, 8, 9]);
        } else {
            assert_eq!(observation.input_tokens, INPUT_TOKENS);
        }
        evidence::publish(&mut observation).unwrap();
        let value: serde_json::Value = serde_json::from_slice(
            &std::fs::read(
                observation
                    .request
                    .decode
                    .evidence_directory
                    .join("complete.json"),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(value["schema"], "FerricFiniteGuardedMlpDecodeObservationV1");
        assert_eq!(value["completed_forwards"], 4);
        assert_eq!(value["files"]["frames"].as_array().unwrap().len(), 4);
        for name in [
            "numerical_acceptance",
            "full_model_acceptance",
            "performance_claim",
            "production_authority",
            "full_long_workload",
        ] {
            assert_eq!(value[name], false);
        }
        assert_eq!(value["native_attempts"], 1);
        assert_eq!(value["retries"], 0);
        assert_eq!(
            std::fs::read_dir(&observation.request.decode.evidence_directory)
                .unwrap()
                .count(),
            14
        );
    }
}
#[test]
fn guarded_parent_completion_refuses_wrong_root_profile_guard_argmax_and_chain() {
    let cfg = config();
    let b = bootstrap(wire::tests::bootstrap(wire::InputMode::TeacherForced), &cfg).unwrap();
    let (request, response, control, payload) = frame(
        &b,
        0,
        None,
        &mut gwire::Chain::new(b.decode.registration, b.sha256().unwrap()),
    );
    for edit in [
        |r: &mut gwire::Response| r.schema.push('x'),
        |r: &mut gwire::Response| r.profile_sha256[0] ^= 1,
        |r: &mut gwire::Response| r.session[0] ^= 1,
        |r: &mut gwire::Response| r.full_model_acceptance = true,
        |r: &mut gwire::Response| {
            if let gwire::Event::Completed(c) = &mut r.event {
                c.chain[0] ^= 1
            }
        },
        |r: &mut gwire::Response| {
            if let gwire::Event::Completed(c) = &mut r.event {
                c.output_token += 1
            }
        },
    ] {
        let mut bad = response.clone();
        edit(&mut bad);
        assert!(
            validate_completion(
                &request,
                &bad,
                &control,
                &payload,
                &mut gwire::Chain::new(b.decode.registration, b.sha256().unwrap())
            )
            .is_err()
        );
    }
    let mut bad = control.clone();
    bad.layers[35].guards[1][0] = 2;
    assert!(
        validate_completion(
            &request,
            &response,
            &bad,
            &payload,
            &mut gwire::Chain::new(b.decode.registration, b.sha256().unwrap())
        )
        .is_err()
    );
    let mut frontiers = [(0, 0); 2];
    check_frontiers(&mut frontiers, &control).unwrap();
    assert!(check_frontiers(&mut frontiers, &control).is_err());
    assert!(trajectory(&b, &[9112, 2190, 3772, 220], &[7, 8, 9, 151936]).is_err());
    let ar = bootstrap(
        wire::tests::bootstrap(wire::InputMode::Autoregressive),
        &cfg,
    )
    .unwrap_err();
    assert!(!ar.is_empty());
}
#[test]
fn guarded_parent_publication_requires_healthy_close_scope_and_unchanged_retained_frames() {
    let temp = Temp::new();
    let mut observation = completed(&temp.0.join("capture"), wire::InputMode::Autoregressive);
    observation.native_closed = false;
    assert!(evidence::validate_closed(&observation).is_err());
    observation.native_closed = true;
    observation.full_model_acceptance = true;
    assert!(evidence::validate_closed(&observation).is_err());
    observation.full_model_acceptance = false;
    observation.request.guarded_image.sha256[0] ^= 1;
    assert!(evidence::validate_closed(&observation).is_err());
    observation.request.guarded_image.sha256[0] ^= 1;
    observation.observed_output_tokens[0] += 1;
    assert!(evidence::validate_closed(&observation).is_err());
    observation.observed_output_tokens[0] -= 1;
    observation.retries = 1;
    assert!(evidence::validate_closed(&observation).is_err());
    observation.retries = 0;
    evidence::validate_closed(&observation).unwrap();
    std::fs::write(&observation.files.frames[3].control.path, [0]).unwrap();
    assert!(evidence::publish(&mut observation).is_err());
    assert!(
        !observation
            .request
            .decode
            .evidence_directory
            .join("complete.json")
            .exists()
    );
}
#[test]
fn guarded_parent_evidence_never_publishes_partial_or_extra_file_captures() {
    let temp = Temp::new();
    let path = temp.0.join("partial");
    let evidence = evidence::Evidence::create(&path).unwrap();
    assert!(evidence.finish(&[]).is_err());
    assert!(!path.join("complete.json").exists());
    let mut observation = completed(&temp.0.join("extra"), wire::InputMode::TeacherForced);
    std::fs::write(
        observation
            .request
            .decode
            .evidence_directory
            .join("unexpected"),
        [],
    )
    .unwrap();
    assert!(evidence::publish(&mut observation).is_err());
    assert!(
        !observation
            .request
            .decode
            .evidence_directory
            .join("complete.json")
            .exists()
    );
}
