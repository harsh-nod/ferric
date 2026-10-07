use super::*;
use crate::finite_guarded_mlp_long_wire_v2::tests as fixture;

fn digest(text: &str) -> [u8; 32] {
    std::array::from_fn(|i| u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).unwrap())
}
fn config() -> ReadinessConfig {
    let pin = FilePin {
        path: "/task/image".into(),
        bytes: 1,
        sha256: [1; 32],
    };
    ReadinessConfig {
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
            evidence_directory: "/task/readiness-evidence".into(),
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
fn readiness_parent_request_is_closed_and_keeps_authentic_full_prompt() {
    let value = config();
    ReadinessConfig::parse(&serde_json::to_vec(&value).unwrap()).unwrap();
    for edit in [
        |v: &mut ReadinessConfig| v.schema = "FerricFiniteLongRequestV1".into(),
        |v: &mut ReadinessConfig| v.base.prompt.tokens.bytes = 160,
        |v: &mut ReadinessConfig| v.base.child_deadline_ms = 3_600_001,
        |v: &mut ReadinessConfig| v.guarded_image.sha256[0] ^= 1,
        |v: &mut ReadinessConfig| v.projection_image = v.base.images.residual.clone(),
    ] {
        let mut bad = value.clone();
        edit(&mut bad);
        assert!(ReadinessConfig::parse(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
    let mut raw = serde_json::to_value(value).unwrap();
    raw["generated_tokens"] = 256.into();
    assert!(ReadinessConfig::parse(&serde_json::to_vec(&raw).unwrap()).is_err());
}
#[test]
fn readiness_parent_page_rows_cross_boundaries_with_exact_144_page_mapping() {
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
    for position in 0..40 {
        let batch = pool
            .reserve_batch(&[EngineeringTpPageRowV1 {
                sequence: sequence.sequence(),
                token: position + 2,
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
    assert_eq!(pool.committed_position(sequence.sequence()).unwrap(), 40);
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
            "ferric-ready-evidence-{}-{}",
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
fn frame(b: &long::Bootstrap, position: u32) -> (long::Frame, long::Capture) {
    let request = fixture::request(b, position, 7);
    let control = fixture::control(u64::from(position) + 1);
    let observation = fixture::payload(7);
    let completion =
        long::Completion::from_observation(b.profile, &request, &control, &observation, 7).unwrap();
    (
        long::Frame {
            schema: long::RESPONSE_SCHEMA.into(),
            profile: b.profile,
            request,
            completion,
        },
        long::Capture {
            control,
            parts: old::Payload::from_bytes(&observation).unwrap(),
            observation,
        },
    )
}
#[test]
fn readiness_parent_retention_is_compact_selected_and_not_published_early() {
    let temp = Temp::new();
    let mut evidence = retained::Evidence::create(&temp.0).unwrap();
    assert!(retained::Evidence::create(&temp.0).is_err());
    let b = fixture::bootstrap(long::Profile::Readiness40);
    let mut transcript = long::Transcript::new(b.clone()).unwrap();
    for position in 0..40 {
        let (mut frame, capture) = frame(&b, position);
        transcript.begin(&frame.request).unwrap();
        frame.completion.chain = transcript
            .next_chain(&frame.request, &frame.completion)
            .unwrap();
        evidence
            .append(&frame, b.profile.captures(position).then_some(&capture))
            .unwrap();
        transcript.advance(&frame).unwrap();
    }
    let files = evidence.finish(&[]).unwrap();
    assert_eq!(files.rows, 40);
    assert_eq!(
        files
            .captures
            .iter()
            .map(|c| c.position)
            .collect::<Vec<_>>(),
        [0, 15, 16, 39]
    );
    assert!(files.bytes_before_summary < 5 << 20);
    assert!(!temp.0.join("complete.json").exists());
    assert_eq!(std::fs::read_dir(&temp.0).unwrap().count(), 6);
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
    let close = ready::Closed::new(fixture::close_request(&b), digest).unwrap();
    let mut value = Observation {
        schema: "FerricGuardedMlpReadiness40ObservationV1",
        child_pid: b.scope.child_identity,
        profile_sha256: b.sha256().unwrap(),
        registration_sha256: b.registration,
        source_program_sha256: b.begin.source_program.sha256,
        upload_manifest_sha256: b.begin.uploads.sha256,
        bootstrap: ready::Bootstrap {
            schema: ready::SCHEMA.into(),
            sequence: b,
            child_deadline_ms: request.base.child_deadline_ms,
        },
        request,
        setup_commands: 1,
        completed_forwards: 40,
        prompt_positions_executed: 40,
        generated_tokens: Vec::new(),
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
        full_long_workload: false,
        numerical_acceptance: false,
        performance_claim: false,
        production_authority: false,
    };
    // This is synthetic serialization coverage, not a native observation.
    value.generated_tokens.push(7);
    assert!(retained::publish(&mut value).is_err());
    assert!(!temp.0.join("complete.json").exists());
    value.generated_tokens.clear();
    value.native_closed = false;
    assert!(retained::publish(&mut value).is_err());
    value.native_closed = true;
    retained::publish(&mut value).unwrap();
    assert!(temp.0.join("complete.json").is_file());
    assert!(retained::publish(&mut value).is_err());
}
#[test]
fn readiness_parent_retention_refuses_partial_reordered_and_changed_captures() {
    let b = fixture::bootstrap(long::Profile::Readiness40);
    let temp = Temp::new();
    let mut evidence = retained::Evidence::create(&temp.0).unwrap();
    let (first, mut capture) = frame(&b, 0);
    assert!(evidence.append(&first, None).is_err());
    capture.observation[0] ^= 1;
    assert!(evidence.append(&first, Some(&capture)).is_err());
    let (second, _) = frame(&b, 1);
    assert!(evidence.append(&second, None).is_err());
    assert!(evidence.finish(&[]).is_err());
    assert!(!temp.0.join("complete.json").exists());
}

#[test]
fn position5_parent_retention_keeps_distinct_profile_and_four_capture_bound() {
    let temp = Temp::new();
    let mut evidence =
        retained::Evidence::create_for(&temp.0, long::Profile::Readiness40Position5).unwrap();
    assert!(retained::Evidence::create_for(&temp.0, long::Profile::Readiness40Position5).is_err());
    let b = fixture::bootstrap(long::Profile::Readiness40Position5);
    let mut transcript = long::Transcript::new(b.clone()).unwrap();
    for position in 0..40 {
        let (mut frame, capture) = frame(&b, position);
        transcript.begin(&frame.request).unwrap();
        frame.completion.chain = transcript
            .next_chain(&frame.request, &frame.completion)
            .unwrap();
        evidence
            .append(&frame, b.profile.captures(position).then_some(&capture))
            .unwrap();
        transcript.advance(&frame).unwrap();
    }
    let files = evidence.finish(&[]).unwrap();
    assert_eq!(files.rows, 40);
    assert_eq!(
        files
            .captures
            .iter()
            .map(|c| c.position)
            .collect::<Vec<_>>(),
        [0, 5, 16, 39]
    );
    assert!(files.bytes_before_summary < 5 << 20);
    assert!(!temp.0.join("complete.json").exists());
    assert_eq!(std::fs::read_dir(&temp.0).unwrap().count(), 6);
    let mut request = config();
    request.schema = POSITION5_REQUEST_SCHEMA.into();
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
    let close = ready::Closed::new_for(b.profile, fixture::close_request(&b), digest).unwrap();
    let mut value = Observation {
        schema: "FerricGuardedMlpReadiness40Position5ObservationV1",
        child_pid: b.scope.child_identity,
        profile_sha256: b.sha256().unwrap(),
        registration_sha256: b.registration,
        source_program_sha256: b.begin.source_program.sha256,
        upload_manifest_sha256: b.begin.uploads.sha256,
        bootstrap: ready::Bootstrap {
            schema: ready::POSITION5_SCHEMA.into(),
            sequence: b,
            child_deadline_ms: request.base.child_deadline_ms,
        },
        request,
        setup_commands: 1,
        completed_forwards: 40,
        prompt_positions_executed: 40,
        generated_tokens: Vec::new(),
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
        full_long_workload: false,
        numerical_acceptance: false,
        performance_claim: false,
        production_authority: false,
    };
    // This is synthetic serialization coverage, not a native observation.
    value.generated_tokens.push(7);
    assert!(retained::publish(&mut value).is_err());
    assert!(!temp.0.join("complete.json").exists());
    value.generated_tokens.clear();
    value.native_closed = false;
    assert!(retained::publish(&mut value).is_err());
    value.native_closed = true;
    retained::publish(&mut value).unwrap();
    assert!(temp.0.join("complete.json").is_file());
    assert!(retained::publish(&mut value).is_err());
}

#[test]
fn position5_parent_selector_refuses_cross_profile_before_any_io() {
    let old = config();
    let mut diagnostic = old.clone();
    diagnostic.schema = POSITION5_REQUEST_SCHEMA.into();
    ReadinessConfig::parse(&serde_json::to_vec(&diagnostic).unwrap()).unwrap();
    assert_eq!(
        run(diagnostic, true).err().unwrap(),
        "readiness explicit parent entry/profile mismatch"
    );
    assert_eq!(
        run_position5(old, true).err().unwrap(),
        "readiness explicit parent entry/profile mismatch"
    );
}
