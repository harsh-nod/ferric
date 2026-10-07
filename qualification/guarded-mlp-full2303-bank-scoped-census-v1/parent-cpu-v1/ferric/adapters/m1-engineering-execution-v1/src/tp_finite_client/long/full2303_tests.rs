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

fn completed_observation() -> (Temp, Observation) {
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
    let value = Observation {
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
    (temp, value)
}

#[test]
fn full2303_parent_retention_reconstructs_all_frames_tokens_and_original_captures() {
    let (temp, mut value) = completed_observation();
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

fn scoped_fixture() -> (
    full::Bootstrap,
    Vec<u32>,
    crate::finite_guarded_mlp_full2303_scoped_v1::PolicyRecord,
) {
    use crate::finite_guarded_mlp_full2303_scoped_v1::{Counts, PolicyRecord};
    let b = full::Bootstrap {
        schema: full::SCHEMA.into(),
        child_deadline_ms: 60_000,
        sequence: fixture::bootstrap(long::Profile::Full2303),
    };
    let tokens = vec![7; long::OUTPUT_TOKENS];
    let n = 82_836;
    let policy = PolicyRecord::new(
        &b,
        [7; 32],
        &tokens,
        [8; 32],
        Counts {
            ordinary_layers: 72,
            scoped_layers: n,
            full_discoveries: 2 * u64::from(n),
            local_checkpoints: 5 * u64::from(n),
            before_calls: 11 * u64::from(n),
            after_calls: 11 * u64::from(n),
            generation_probes: 13 * u64::from(n),
        },
    )
    .unwrap();
    (b, tokens, policy)
}
fn scoped_policy_file(temp: &Temp, raw: &[u8]) -> FilePin {
    std::fs::create_dir_all(&temp.0).unwrap();
    let path = temp.0.join("child-stderr.bin");
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    file.write_all(raw).unwrap();
    file.sync_all().unwrap();
    FilePin {
        path,
        bytes: raw.len() as u64,
        sha256: hash(raw),
    }
}

#[test]
fn full2303_scoped_parent_entry_preserves_ordinary_signature_and_closed_opt_in() {
    let _: fn(Full2303Config, bool) -> Result<Observation> = run;
    let _: fn(Full2303Config, bool) -> Result<ScopedWarmObservation> = run_scoped_warm;
    scoped::admit_entry(REQUEST_SCHEMA, true).unwrap();
    assert!(scoped::admit_entry(REQUEST_SCHEMA, false).is_err());
    for schema in [
        "FerricGuardedMlpReadiness40RequestV1",
        "FerricGuardedMlpReadiness40Position5RequestV1",
        "FerricGuardedMlpReadiness40CausalLayer0RequestV1",
        "FerricFiniteLongRequestV1",
        OBSERVATION_SCHEMA,
        scoped::SCHEMA,
        "",
    ] {
        assert!(scoped::admit_entry(schema, true).is_err());
        let mut bad = config();
        bad.schema = schema.into();
        assert_eq!(
            run_scoped_warm(bad, true).err().unwrap(),
            "full2303 scoped warm requires explicit Full2303 opt-in"
        );
    }
    assert_eq!(
        run_scoped_warm(config(), false).err().unwrap(),
        "full2303 scoped warm requires explicit Full2303 opt-in"
    );
    let mut admitted = config();
    admitted.base.prompt.tokens.bytes = 160;
    assert_eq!(
        run_scoped_warm(admitted, true).err().unwrap(),
        "long profile requires the exact retained raw 2048-token prompt"
    );
    let mut late = config();
    late.base.child_deadline_ms = full::MAX_DEADLINE_MS + 1;
    assert_eq!(
        run_scoped_warm(late, true).err().unwrap(),
        "long request scope or bounds"
    );
}

#[test]
fn full2303_scoped_parent_rechecks_original_policy_file_and_own_256_outputs() {
    let (b, tokens, policy) = scoped_fixture();
    let temp = Temp::new();
    let pin = scoped_policy_file(&temp, &policy.encode().unwrap());
    assert_eq!(
        scoped::read_policy(&pin, &b, [7; 32], &tokens, [8; 32]).unwrap(),
        policy
    );
    assert!(scoped::read_policy(&pin, &b, [7; 32], &tokens, [9; 32]).is_err());
    assert!(scoped::read_policy(&pin, &b, [6; 32], &tokens, [8; 32]).is_err());
    for index in [0, 255] {
        let mut changed = tokens.clone();
        changed[index] ^= 1;
        assert!(scoped::read_policy(&pin, &b, [7; 32], &changed, [8; 32]).is_err());
    }
    assert!(scoped::read_policy(&pin, &b, [7; 32], &tokens[..255], [8; 32]).is_err());
    for edit in [
        |v: &mut full::Bootstrap| v.sequence.scope.session[0] ^= 1,
        |v: &mut full::Bootstrap| v.sequence.device_ids.swap(0, 1),
        |v: &mut full::Bootstrap| v.sequence.scope.child_identity += 1,
        |v: &mut full::Bootstrap| v.sequence.registration[0] ^= 1,
        |v: &mut full::Bootstrap| v.sequence.begin.source_program.sha256[0] ^= 1,
        |v: &mut full::Bootstrap| v.sequence.scope.model_id[0] ^= 1,
        |v: &mut full::Bootstrap| v.sequence.scope.bundle_id[0] ^= 1,
    ] {
        let mut changed = b.clone();
        edit(&mut changed);
        assert!(scoped::read_policy(&pin, &changed, [7; 32], &tokens, [8; 32]).is_err());
    }
    let mut wrong = pin.clone();
    wrong.sha256[0] ^= 1;
    assert!(scoped::read_policy(&wrong, &b, [7; 32], &tokens, [8; 32]).is_err());
    std::fs::write(&pin.path, b"changed original policy\n").unwrap();
    assert!(scoped::read_policy(&pin, &b, [7; 32], &tokens, [8; 32]).is_err());
    std::fs::remove_file(&pin.path).unwrap();
    assert!(scoped::read_policy(&pin, &b, [7; 32], &tokens, [8; 32]).is_err());
}

#[test]
fn full2303_scoped_parent_refuses_missing_duplicate_malformed_and_oversized_policy() {
    use crate::finite_guarded_mlp_full2303_scoped_v1::MAX_BYTES;
    let (b, tokens, policy) = scoped_fixture();
    let raw = policy.encode().unwrap();
    for bad in [
        Vec::new(),
        b"not JSON\n".to_vec(),
        [raw.clone(), raw.clone()].concat(),
        raw[..raw.len() - 1].to_vec(),
        vec![b' '; MAX_BYTES + 1],
        br#"{"schema":"FerricReadiness40Position5ScopedWarmPolicyV1"}"#.to_vec(),
    ] {
        assert!(scoped::validate_original(&bad, &b, [7; 32], &tokens, [8; 32]).is_err());
        let temp = Temp::new();
        let pin = scoped_policy_file(&temp, &bad);
        assert!(scoped::read_policy(&pin, &b, [7; 32], &tokens, [8; 32]).is_err());
    }
}

#[test]
fn full2303_scoped_parent_refuses_rehashed_extent_counter_and_authority_drift() {
    let (b, tokens, policy) = scoped_fixture();
    for mutation in 0..12 {
        let mut value = policy.clone();
        match mutation {
            0 => value.counts.ordinary_layers -= 1,
            1 => value.counts.scoped_layers -= 1,
            2 => value.counts.full_discoveries -= 1,
            3 => value.counts.after_calls += 1,
            4 => value.counts.generation_probes += 1,
            5 => value.counts.local_checkpoints = u64::MAX,
            6 => value.temporal_equivalent_to_full = true,
            7 => value.shared_full_currentness = true,
            8 => value.native_closed = false,
            9 => value.generated_token_count = 255,
            10 => value.first_scoped_position = 1,
            _ => value.numerical_acceptance = true,
        }
        let temp = Temp::new();
        let pin = scoped_policy_file(&temp, &value.encode().unwrap());
        assert!(
            scoped::read_policy(&pin, &b, [7; 32], &tokens, [8; 32]).is_err(),
            "{mutation}"
        );
    }
}

#[test]
fn full2303_scoped_parent_publication_requires_original_policy_and_healthy_own_history() {
    let (temp, mut value) = completed_observation();
    let future = Instant::now() + Duration::from_secs(60);
    assert!(retained::publish_scoped(&mut value, future).is_err());
    assert!(!temp.0.join("complete.json").exists());
    let (_, _, fixture_policy) = scoped_fixture();
    let policy = crate::finite_guarded_mlp_full2303_scoped_v1::PolicyRecord::new(
        &value.bootstrap,
        value.transcript_sha256,
        &value.generated_tokens,
        value.request.base.worker.sha256,
        fixture_policy.counts,
    )
    .unwrap();
    let raw = policy.encode().unwrap();
    std::fs::write(&value.files.child_stderr.path, &raw).unwrap();
    value.files.child_stderr.bytes = raw.len() as u64;
    value.files.child_stderr.sha256 = hash(&raw);
    value.files.bytes_before_summary += raw.len() as u64;
    value.files.total_bytes += raw.len() as u64;
    assert_eq!(scoped::validate_file(&value).unwrap(), policy);
    assert!(scoped::validate_stdout_bound(&value).unwrap() <= 128 << 10);
    assert!(scoped::check_stdout_bytes(0).is_err());
    assert!(scoped::check_stdout_bytes((128 << 10) + 1).is_err());
    scoped::check_stdout_bytes(128 << 10).unwrap();
    assert!(retained::publish_scoped(&mut value, Instant::now()).is_err());
    assert!(!temp.0.join("complete.json").exists());
    value.native_closed = false;
    assert!(retained::publish_scoped(&mut value, future).is_err());
    value.native_closed = true;
    value.generated_tokens[255] ^= 1;
    assert!(retained::publish_scoped(&mut value, future).is_err());
    value.generated_tokens[255] ^= 1;
    value.request.base.worker.sha256[0] ^= 1;
    assert!(retained::publish_scoped(&mut value, future).is_err());
    value.request.base.worker.sha256[0] ^= 1;
    std::fs::write(
        &value.files.child_stderr.path,
        b"changed before publication\n",
    )
    .unwrap();
    assert!(retained::publish_scoped(&mut value, future).is_err());
    std::fs::write(&value.files.child_stderr.path, &raw).unwrap();
    assert!(!temp.0.join("complete.json").exists());
    retained::publish_scoped(&mut value, future).unwrap();
    assert!(temp.0.join("complete.json").is_file());
    assert_eq!(value.files.child_stderr.read(4096, true).unwrap(), raw);
    assert_eq!(scoped::validate_file(&value).unwrap(), policy);
    assert_eq!(
        value.files.total_bytes,
        value.files.bytes_before_summary + value.files.summary_bytes
    );
    let expected_stdout_bytes = scoped::validate_stdout_bound(&value).unwrap();
    let wrapper = ScopedWarmObservation {
        schema: scoped::SCHEMA,
        observation: value,
        currentness_policy: policy,
    };
    assert_eq!(
        serde_json::to_vec(&wrapper).unwrap().len() + 1,
        expected_stdout_bytes
    );
}

fn bank_census_fixture() -> (
    full::Bootstrap,
    Vec<u32>,
    crate::finite_guarded_mlp_full2303_bank_scoped_census_v1::PolicyRecord,
) {
    use crate::finite_guarded_mlp_full2303_bank_scoped_census_v1::{
        BankCounts, CensusCounts, Counts, LayerCounts, PolicyRecord,
    };
    let b = full::Bootstrap {
        schema: full::SCHEMA.into(),
        child_deadline_ms: 60_000,
        sequence: fixture::bootstrap(long::Profile::Full2303),
    };
    let tokens = vec![7; long::OUTPUT_TOKENS];
    let n = 82_836;
    let policy = PolicyRecord::new(
        &b,
        [7; 32],
        &tokens,
        [8; 32],
        Counts {
            layers: LayerCounts {
                ordinary_layers: 72,
                scoped_layers: n,
                full_discoveries: 2 * u64::from(n),
                local_checkpoints: 21 * u64::from(n),
                before_calls: 27 * u64::from(n),
                after_calls: 27 * u64::from(n),
                generation_probes: 45 * u64::from(n),
            },
            banks: BankCounts {
                ordinary_initial_banks: 2,
                scoped_rearms: 2301,
                final_generations: [1152, 1151],
                full_discoveries: 4602,
                local_checkpoints: 2301,
                before_calls: 6 * 2301,
                after_calls: 6 * 2301,
                generation_probes: 5 * 2301,
            },
            census: CensusCounts {
                warm_layers: n,
                preflights: 2 * n,
                rank_checkpoints: 16 * n,
                owner_counts: [787, 783],
            },
        },
    )
    .unwrap();
    (b, tokens, policy)
}
fn bank_census_policy_file(temp: &Temp, raw: &[u8]) -> FilePin {
    std::fs::create_dir_all(&temp.0).unwrap();
    let path = temp.0.join("child-stderr.bin");
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    file.write_all(raw).unwrap();
    file.sync_all().unwrap();
    FilePin {
        path,
        bytes: raw.len() as u64,
        sha256: hash(raw),
    }
}

#[test]
fn full2303_bank_scoped_census_parent_entry_preserves_ordinary_signature_and_closed_opt_in() {
    let _: fn(Full2303Config, bool) -> Result<Observation> = run;
    let _: fn(Full2303Config, bool) -> Result<ScopedWarmObservation> = run_scoped_warm;
    let _: fn(Full2303Config, bool) -> Result<BankScopedCensusObservation> = run_bank_scoped_census;
    bank_scoped_census::admit_entry(REQUEST_SCHEMA, true).unwrap();
    assert!(bank_scoped_census::admit_entry(REQUEST_SCHEMA, false).is_err());
    for schema in [
        "FerricGuardedMlpReadiness40RequestV1",
        "FerricGuardedMlpReadiness40Position5RequestV1",
        "FerricGuardedMlpReadiness40CausalLayer0RequestV1",
        "FerricFiniteLongRequestV1",
        OBSERVATION_SCHEMA,
        bank_scoped_census::SCHEMA,
        "",
    ] {
        assert!(bank_scoped_census::admit_entry(schema, true).is_err());
        let mut bad = config();
        bad.schema = schema.into();
        assert_eq!(
            run_bank_scoped_census(bad, true).err().unwrap(),
            "full2303 bank-scoped census requires explicit Full2303 opt-in"
        );
    }
    assert_eq!(
        run_bank_scoped_census(config(), false).err().unwrap(),
        "full2303 bank-scoped census requires explicit Full2303 opt-in"
    );
    let mut admitted = config();
    admitted.base.prompt.tokens.bytes = 160;
    assert_eq!(
        run_bank_scoped_census(admitted, true).err().unwrap(),
        "long profile requires the exact retained raw 2048-token prompt"
    );
    let mut late = config();
    late.base.child_deadline_ms = full::MAX_DEADLINE_MS + 1;
    assert_eq!(
        run_bank_scoped_census(late, true).err().unwrap(),
        "long request scope or bounds"
    );
}

#[test]
fn full2303_bank_scoped_census_parent_rechecks_original_policy_file_and_own_256_outputs() {
    let (b, tokens, policy) = bank_census_fixture();
    let temp = Temp::new();
    let pin = bank_census_policy_file(&temp, &policy.encode().unwrap());
    assert_eq!(
        bank_scoped_census::read_policy(&pin, &b, [7; 32], &tokens, [8; 32]).unwrap(),
        policy
    );
    assert!(bank_scoped_census::read_policy(&pin, &b, [7; 32], &tokens, [9; 32]).is_err());
    assert!(bank_scoped_census::read_policy(&pin, &b, [6; 32], &tokens, [8; 32]).is_err());
    for index in [0, 255] {
        let mut changed = tokens.clone();
        changed[index] ^= 1;
        assert!(bank_scoped_census::read_policy(&pin, &b, [7; 32], &changed, [8; 32]).is_err());
    }
    assert!(bank_scoped_census::read_policy(&pin, &b, [7; 32], &tokens[..255], [8; 32]).is_err());
    for edit in [
        |v: &mut full::Bootstrap| v.sequence.scope.session[0] ^= 1,
        |v: &mut full::Bootstrap| v.sequence.device_ids.swap(0, 1),
        |v: &mut full::Bootstrap| v.sequence.scope.child_identity += 1,
        |v: &mut full::Bootstrap| v.sequence.registration[0] ^= 1,
        |v: &mut full::Bootstrap| v.sequence.begin.source_program.sha256[0] ^= 1,
        |v: &mut full::Bootstrap| v.sequence.scope.model_id[0] ^= 1,
        |v: &mut full::Bootstrap| v.sequence.scope.bundle_id[0] ^= 1,
    ] {
        let mut changed = b.clone();
        edit(&mut changed);
        assert!(
            bank_scoped_census::read_policy(&pin, &changed, [7; 32], &tokens, [8; 32]).is_err()
        );
    }
    let mut wrong = pin.clone();
    wrong.sha256[0] ^= 1;
    assert!(bank_scoped_census::read_policy(&wrong, &b, [7; 32], &tokens, [8; 32]).is_err());
    std::fs::write(&pin.path, b"changed original policy\n").unwrap();
    assert!(bank_scoped_census::read_policy(&pin, &b, [7; 32], &tokens, [8; 32]).is_err());
    std::fs::remove_file(&pin.path).unwrap();
    assert!(bank_scoped_census::read_policy(&pin, &b, [7; 32], &tokens, [8; 32]).is_err());
}

#[test]
fn full2303_bank_scoped_census_parent_refuses_missing_duplicate_malformed_and_oversized_policy() {
    use crate::finite_guarded_mlp_full2303_bank_scoped_census_v1::MAX_BYTES;
    let (b, tokens, policy) = bank_census_fixture();
    let raw = policy.encode().unwrap();
    for bad in [
        Vec::new(),
        b"not JSON\n".to_vec(),
        [raw.clone(), raw.clone()].concat(),
        raw[..raw.len() - 1].to_vec(),
        vec![b' '; MAX_BYTES + 1],
        br#"{"schema":"FerricFull2303ScopedWarmPolicyV1"}"#.to_vec(),
    ] {
        assert!(
            bank_scoped_census::validate_original(&bad, &b, [7; 32], &tokens, [8; 32]).is_err()
        );
        let temp = Temp::new();
        let pin = bank_census_policy_file(&temp, &bad);
        assert!(bank_scoped_census::read_policy(&pin, &b, [7; 32], &tokens, [8; 32]).is_err());
    }
}

#[test]
fn full2303_bank_scoped_census_parent_refuses_rehashed_extent_counter_and_authority_drift() {
    let (b, tokens, policy) = bank_census_fixture();
    for mutation in 0..23 {
        let mut value = policy.clone();
        match mutation {
            0 => value.counts.layers.ordinary_layers -= 1,
            1 => value.counts.layers.scoped_layers -= 1,
            2 => value.counts.layers.full_discoveries -= 1,
            3 => value.counts.layers.after_calls += 1,
            4 => value.counts.layers.generation_probes += 1,
            5 => value.counts.layers.local_checkpoints = u64::MAX,
            6 => value.temporal_equivalent_to_full = true,
            7 => value.shared_full_currentness = true,
            8 => value.native_closed = false,
            9 => value.generated_token_count = 255,
            10 => value.first_scoped_position = 1,
            11 => value.numerical_acceptance = true,
            12 => value.counts.banks.scoped_rearms -= 1,
            13 => value.counts.banks.final_generations.swap(0, 1),
            14 => value.counts.banks.full_discoveries -= 1,
            15 => value.counts.banks.local_checkpoints = u64::MAX,
            16 => value.counts.banks.after_calls += 1,
            17 => value.counts.banks.generation_probes += 1,
            18 => value.counts.census.warm_layers -= 1,
            19 => value.counts.census.preflights -= 1,
            20 => value.counts.census.rank_checkpoints -= 1,
            21 => value.counts.census.owner_counts[1] = 0,
            _ => value.allocation_preflights_outside_windows = true,
        }
        let temp = Temp::new();
        let pin = bank_census_policy_file(&temp, &value.encode().unwrap());
        assert!(
            bank_scoped_census::read_policy(&pin, &b, [7; 32], &tokens, [8; 32]).is_err(),
            "{mutation}"
        );
    }
}

#[test]
fn full2303_bank_scoped_census_parent_publication_requires_original_policy_and_healthy_own_history()
{
    let (temp, mut value) = completed_observation();
    let future = Instant::now() + Duration::from_secs(60);
    assert!(retained::publish_bank_scoped_census(&mut value, future).is_err());
    assert!(!temp.0.join("complete.json").exists());
    let (_, _, fixture_policy) = bank_census_fixture();
    let policy = crate::finite_guarded_mlp_full2303_bank_scoped_census_v1::PolicyRecord::new(
        &value.bootstrap,
        value.transcript_sha256,
        &value.generated_tokens,
        value.request.base.worker.sha256,
        fixture_policy.counts,
    )
    .unwrap();
    let raw = policy.encode().unwrap();
    std::fs::write(&value.files.child_stderr.path, &raw).unwrap();
    value.files.child_stderr.bytes = raw.len() as u64;
    value.files.child_stderr.sha256 = hash(&raw);
    value.files.bytes_before_summary += raw.len() as u64;
    value.files.total_bytes += raw.len() as u64;
    assert_eq!(bank_scoped_census::validate_file(&value).unwrap(), policy);
    assert!(bank_scoped_census::validate_stdout_bound(&value).unwrap() <= 128 << 10);
    assert!(bank_scoped_census::check_stdout_bytes(0).is_err());
    assert!(bank_scoped_census::check_stdout_bytes((128 << 10) + 1).is_err());
    bank_scoped_census::check_stdout_bytes(128 << 10).unwrap();
    assert!(retained::publish_bank_scoped_census(&mut value, Instant::now()).is_err());
    assert!(!temp.0.join("complete.json").exists());
    value.native_closed = false;
    assert!(retained::publish_bank_scoped_census(&mut value, future).is_err());
    value.native_closed = true;
    value.generated_tokens[255] ^= 1;
    assert!(retained::publish_bank_scoped_census(&mut value, future).is_err());
    value.generated_tokens[255] ^= 1;
    value.request.base.worker.sha256[0] ^= 1;
    assert!(retained::publish_bank_scoped_census(&mut value, future).is_err());
    value.request.base.worker.sha256[0] ^= 1;
    std::fs::write(
        &value.files.child_stderr.path,
        b"changed before publication\n",
    )
    .unwrap();
    assert!(retained::publish_bank_scoped_census(&mut value, future).is_err());
    std::fs::write(&value.files.child_stderr.path, &raw).unwrap();
    assert!(!temp.0.join("complete.json").exists());
    retained::publish_bank_scoped_census(&mut value, future).unwrap();
    assert!(temp.0.join("complete.json").is_file());
    assert_eq!(value.files.child_stderr.read(4096, true).unwrap(), raw);
    assert_eq!(bank_scoped_census::validate_file(&value).unwrap(), policy);
    assert_eq!(
        value.files.total_bytes,
        value.files.bytes_before_summary + value.files.summary_bytes
    );
    let expected_stdout_bytes = bank_scoped_census::validate_stdout_bound(&value).unwrap();
    let wrapper = BankScopedCensusObservation {
        schema: bank_scoped_census::SCHEMA,
        observation: value,
        currentness_policy: policy,
    };
    assert_eq!(
        serde_json::to_vec(&wrapper).unwrap().len() + 1,
        expected_stdout_bytes
    );
}

#[test]
fn full2303_bank_scoped_census_parent_refuses_valid_totals_with_invalid_census_residue() {
    let (b, tokens, policy) = bank_census_fixture();
    let n = 82_836u64;
    // Whole-layer inequalities alone do not establish the proper census subset.
    for (local, before) in [(17 * n, 37 * n), (16 * n, 20 * n)] {
        let mut bad = policy.clone();
        bad.counts.layers.local_checkpoints = local;
        bad.counts.layers.before_calls = before;
        bad.counts.layers.after_calls = before;
        bad.counts.layers.generation_probes = 2 * local + 3 * n;
        bad.counts.layers.validate_closed().unwrap();
        let temp = Temp::new();
        let pin = bank_census_policy_file(&temp, &bad.encode().unwrap());
        assert!(bank_scoped_census::read_policy(&pin, &b, [7; 32], &tokens, [8; 32]).is_err());
    }
    let (_, _, old) = scoped_fixture();
    let raw = old.encode().unwrap();
    assert!(bank_scoped_census::validate_original(&raw, &b, [7; 32], &tokens, [8; 32]).is_err());
    let raw = policy.encode().unwrap();
    assert!(scoped::validate_original(&raw, &b, [7; 32], &tokens, [8; 32]).is_err());
}
