use super::*;
use crate::finite_guarded_mlp_long_wire_v2::tests as fixture;

fn digest(text: &str) -> [u8; 32] {
    std::array::from_fn(|i| u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).unwrap())
}
fn config() -> Full2303Config {
    let pin = FilePin {
        path: "/task/image".into(),
        bytes: 1,
        sha256: [1; 32],
    };
    Full2303Config {
        schema: REQUEST_SCHEMA.into(),
        base: super::super::Config {
            schema: "FerricFiniteLongRequestV1".into(),
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
            evidence_directory: "/task/full2303-evidence".into(),
            dispatch_timeout_ms: 10_000,
            child_deadline_ms: 3_600_000,
        },
        tiles_image: pin.clone(),
        prefix_image: pin.clone(),
        projection_image: FilePin {
            sha256: [2; 32],
            ..pin.clone()
        },
        guarded_image: FilePin {
            sha256: four::GUARDED_IMAGE,
            ..pin
        },
    }
}
#[test]
fn full2303_parent_request_is_closed_and_keeps_authentic_full_prompt() {
    let value = config();
    Full2303Config::parse(&serde_json::to_vec(&value).unwrap()).unwrap();
    for edit in [
        |v: &mut Full2303Config| v.schema = "FerricGuardedMlpReadiness40RequestV1".into(),
        |v: &mut Full2303Config| v.schema = "FerricFiniteLongRequestV1".into(),
        |v: &mut Full2303Config| v.base.prompt.tokens.bytes = 160,
        |v: &mut Full2303Config| v.base.child_deadline_ms = full::MAX_DEADLINE_MS + 1,
        |v: &mut Full2303Config| v.guarded_image.sha256[0] ^= 1,
        |v: &mut Full2303Config| v.projection_image = v.base.images.residual.clone(),
    ] {
        let mut bad = value.clone();
        edit(&mut bad);
        assert!(Full2303Config::parse(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
    let mut raw = serde_json::to_value(value).unwrap();
    raw["generated_tokens"] = 256.into();
    assert!(Full2303Config::parse(&serde_json::to_vec(&raw).unwrap()).is_err());
    assert_eq!(
        run(config(), false).err().unwrap(),
        "full2303 explicit machine-code opt-in required"
    );
    let mut old = config();
    old.schema = "FerricGuardedMlpReadiness40Position5RequestV1".into();
    assert_eq!(
        run(old, true).err().unwrap(),
        "full2303 closed request/profile/image"
    );
}
#[test]
fn full2303_parent_schedule_uses_prompt_then_actual_outputs_without_early_eos() {
    let prompt = (0..2048).map(|i| i + 3).collect::<Vec<_>>();
    for position in 0..2048 {
        assert_eq!(
            super::super::input_token(&prompt, position, Some(99)).unwrap(),
            position + 3
        );
    }
    for position in 2048..2303 {
        assert_eq!(
            super::super::input_token(&prompt, position, Some(151_645)).unwrap(),
            151_645
        );
        assert!(super::super::input_token(&prompt, position, None).is_err());
    }
    assert!(super::super::input_token(&prompt, 2303, Some(7)).is_err());
    assert!(super::super::input_token(&prompt[..40], 0, None).is_err());
}
#[test]
fn full2303_parent_page_rows_commit_all_2303_with_exact_144_page_mapping() {
    let scope = EngineeringTpPoolScopeV1 {
        model: [1; 32],
        session: [2; 32],
    };
    let mut pool = EngineeringTpPagedPoolV1::new(
        scope,
        EngineeringTpPagedLimitsV1::new(2304, 32, 144, 1024).unwrap(),
    )
    .unwrap();
    let sequence = pool.open_sequence(scope, &[2], 0).unwrap();
    let mut stable = None;
    for position in 0..long::FORWARDS {
        let batch = pool
            .reserve_batch(&[EngineeringTpPageRowV1 {
                sequence: sequence.sequence(),
                token: 7,
                position,
            }])
            .unwrap();
        pool.begin_submission(&batch).unwrap();
        let metadata = super::super::metadata(&batch, [3; 32], 1_000_000).unwrap();
        assert_eq!(metadata.cache_metadata()[0], position);
        assert_eq!(metadata.cache_metadata().len(), 145);
        super::super::stable_pages(&mut stable, metadata.cache_metadata()).unwrap();
        assert_eq!(
            batch.rows()[0].physical_pages().len(),
            (position / 16 + 1) as usize
        );
        pool.commit_batch(
            &batch,
            EngineeringTpBatchCompletionV1::after_all_ranks(&batch),
        )
        .unwrap();
    }
    assert_eq!(pool.committed_position(sequence.sequence()).unwrap(), 2303);
    assert_eq!(
        stable
            .unwrap()
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>(),
        (0..144).collect()
    );
}
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "ferric-full2303-evidence-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        if self.0.exists() {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
}

// Uncaptured records below are symbolic test data, not evidence of native execution.
fn frame(b: &long::Bootstrap, position: u32) -> (long::Frame, Option<long::Capture>) {
    let request = fixture::request(b, position, 7);
    let capture = if b.profile.captures(position) {
        let observation = fixture::payload(7);
        Some(long::Capture {
            control: fixture::control(u64::from(position) + 1),
            parts: old::Payload::from_bytes(&observation).unwrap(),
            observation,
        })
    } else {
        None
    };
    let completion = match &capture {
        Some(c) => {
            long::Completion::from_observation(b.profile, &request, &c.control, &c.observation, 7)
                .unwrap()
        }
        None => long::Completion {
            generation: u64::from(position) + 1,
            position,
            input_token: if position < 2048 {
                b.prompt_tokens[position as usize]
            } else {
                7
            },
            output_token: 7,
            bank: long::BankStep::at(b.profile, position).unwrap(),
            control: setup_wire::Part {
                bytes: four::CONTROL_BYTES as u32,
                sha256: [1; 32],
            },
            observation: setup_wire::Part {
                bytes: old::OBSERVATION_BYTES as u32,
                sha256: [2; 32],
            },
            logits: setup_wire::Part {
                bytes: old::LOGIT_BYTES as u32,
                sha256: [3; 32],
            },
            captured: false,
            first_frontiers: [(
                u64::from(position) * 1000 + 10,
                u64::from(position) * 1000 + 8,
            ); 2],
            final_frontiers: [(
                u64::from(position) * 1000 + 360,
                u64::from(position) * 1000 + 358,
            ); 2],
            chain: [0; 32],
        },
    };
    (
        long::Frame {
            schema: long::RESPONSE_SCHEMA.into(),
            profile: b.profile,
            request,
            completion,
        },
        capture,
    )
}

#[test]
fn full2303_parent_retention_reconstructs_all_frames_tokens_and_original_captures() {
    let temp = Temp::new();
    let mut evidence = retained::Evidence::create(&temp.0).unwrap();
    assert!(retained::Evidence::create(&temp.0).is_err());
    let b = fixture::bootstrap(long::Profile::Full2303);
    let mut transcript = long::Transcript::new(b.clone()).unwrap();
    for position in 0..long::FORWARDS {
        let (mut frame, capture) = frame(&b, position);
        transcript.begin(&frame.request).unwrap();
        frame.completion.chain = transcript
            .next_chain(&frame.request, &frame.completion)
            .unwrap();
        evidence.append(&frame, capture.as_ref()).unwrap();
        transcript.advance(&frame).unwrap();
    }
    let files = evidence.finish(&[], b"synthetic decoded bytes").unwrap();
    assert_eq!(files.rows, 2303);
    assert_eq!(
        files
            .captures
            .iter()
            .map(|c| c.position)
            .collect::<Vec<_>>(),
        [0, 2047, 2048, 2302]
    );
    assert_eq!(
        files.decoded_output.read(128 << 10, true).unwrap(),
        b"synthetic decoded bytes"
    );
    assert!(files.bytes_before_summary + (128 << 10) + (512 << 10) < 32 << 20);
    assert!(!temp.0.join("complete.json").exists());
    assert_eq!(std::fs::read_dir(&temp.0).unwrap().count(), 7);
    let mut request = config();
    request.base.evidence_directory = temp.0.clone();
    request.base.device_ids = b.device_ids;
    request.base.dispatch_timeout_ms = b.timeout_ms;
    request.base.session = b.scope.session;
    request.base.expected_model_id = b.scope.model_id;
    request.base.expected_bundle_id = b.scope.bundle_id;
    request.base.images.residual.sha256 = [99; 32];
    for (pin, part) in [
        (&mut request.tiles_image, b.mlp_image),
        (&mut request.prefix_image, b.prefix_image),
        (&mut request.projection_image, b.projection_image),
        (&mut request.guarded_image, b.guarded_image),
    ] {
        pin.bytes = u64::from(part.bytes);
        pin.sha256 = part.sha256;
    }
    let digest = transcript.digest();
    let close_request = fixture::close_request(&b);
    transcript.close(&close_request, digest).unwrap();
    let close = full::Closed::from_transcript(close_request, &transcript).unwrap();
    let mut value = Observation {
        schema: OBSERVATION_SCHEMA,
        child_pid: b.scope.child_identity,
        profile_sha256: b.sha256().unwrap(),
        registration_sha256: b.registration,
        source_program_sha256: b.begin.source_program.sha256,
        upload_manifest_sha256: b.begin.uploads.sha256,
        bootstrap: full::Bootstrap {
            schema: full::SCHEMA.into(),
            sequence: b,
            child_deadline_ms: request.base.child_deadline_ms,
        },
        request,
        setup_commands: 1,
        completed_forwards: long::FORWARDS,
        prompt_positions_executed: 2048,
        decode_positions_executed: 255,
        decoded_special_token_policy: "skip",
        generated_tokens: transcript.output_tokens().to_vec(),
        page_permutation: (0..144).collect(),
        transcript_sha256: digest,
        request_stream_bytes: 1,
        response_stream_bytes: 1,
        files,
        close,
        child_exit_zero: true,
        process_group_absent: true,
        native_closed: true,
        gpu_execution: true,
        full_long_workload: true,
        numerical_acceptance: false,
        performance_claim: false,
        production_authority: false,
    };
    assert_eq!(value.generated_tokens, vec![7; 256]);
    // These are serialization tests only; decoded bytes were not produced by a model.
    value.generated_tokens[0] = 8;
    assert!(retained::publish(&mut value).is_err());
    value.generated_tokens[0] = 7;
    value.close.generated_tokens[255] = 8;
    assert!(retained::publish(&mut value).is_err());
    value.close.generated_tokens[255] = 7;
    value.native_closed = false;
    assert!(retained::publish(&mut value).is_err());
    value.native_closed = true;
    value.numerical_acceptance = true;
    assert!(retained::publish(&mut value).is_err());
    value.numerical_acceptance = false;
    let decoded = value.files.decoded_output.read(128 << 10, true).unwrap();
    std::fs::write(&value.files.decoded_output.path, b"changed").unwrap();
    assert!(retained::publish(&mut value).is_err());
    std::fs::write(&value.files.decoded_output.path, decoded).unwrap();
    let saved = value.files.frames.read(32 << 20, true).unwrap();
    let mut rows = saved
        .split_inclusive(|b| *b == b'\n')
        .map(|row| row.to_vec())
        .collect::<Vec<_>>();
    rows.swap(2047, 2048);
    let changed = rows.concat();
    std::fs::write(&value.files.frames.path, &changed).unwrap();
    let original_hash = value.files.frames.sha256;
    value.files.frames.sha256 = hash(&changed);
    assert!(retained::publish(&mut value).is_err());
    std::fs::write(&value.files.frames.path, &saved).unwrap();
    value.files.frames.sha256 = original_hash;
    assert!(!temp.0.join("complete.json").exists());
    retained::publish(&mut value).unwrap();
    assert!(temp.0.join("complete.json").is_file());
    assert_eq!(std::fs::read_dir(&temp.0).unwrap().count(), 8);
    assert!(retained::publish(&mut value).is_err());
}

#[test]
fn full2303_parent_retention_refuses_partial_reordered_and_changed_captures() {
    let b = fixture::bootstrap(long::Profile::Full2303);
    let temp = Temp::new();
    let mut evidence = retained::Evidence::create(&temp.0).unwrap();
    let (first, mut capture) = frame(&b, 0);
    assert!(evidence.append(&first, None).is_err());
    capture.as_mut().unwrap().observation[0] ^= 1;
    assert!(evidence.append(&first, capture.as_ref()).is_err());
    let (second, _) = frame(&b, 1);
    assert!(evidence.append(&second, None).is_err());
    assert!(evidence.finish(&[], &[]).is_err());
    assert!(!temp.0.join("complete.json").exists());
}
#[test]
fn full2303_parent_compact_bound_does_not_require_all_logit_payloads() {
    let frames = u64::from(long::FORWARDS) * (long::RECORD_BYTES as u64 + 1);
    let captures = long::CAPTURE_BYTES as u64;
    let worst_case = frames + captures + (2 << 20) + (128 << 10) + (128 << 10) + (512 << 10);
    assert!(worst_case < long::EVIDENCE_BYTES as u64);
    assert_eq!(long::STREAM_BYTES, 64 << 20);
    assert!(long::FORWARDS as usize * old::LOGIT_BYTES > long::STREAM_BYTES);
    assert_eq!(
        long::Profile::Full2303.capture_positions(),
        [0, 2047, 2048, 2302]
    );
}
