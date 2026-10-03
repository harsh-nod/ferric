use super::*;
use wire::tests::{bootstrap, completed, request};
pub(super) fn config() -> Config {
    fn digest(s: &str) -> [u8; 32] {
        core::array::from_fn(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap())
    }
    let pin = FilePin {
        path: "/task/image".into(),
        bytes: 1,
        sha256: [1; 32],
    };
    Config {
        schema: "FerricFinitePrefixDecodeRequestV1".into(),
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
        device_ids: [11, 12],
        session: [3; 32],
        prompt: PromptPins {
            manifest: FilePin {
                path: "/task/prompt-manifest.json".into(),
                bytes: 21318,
                sha256: digest(MANIFEST_SHA),
            },
            text: FilePin {
                path: "/task/prompt.txt".into(),
                bytes: 11224,
                sha256: digest(PROMPT_SHA),
            },
            tokens: FilePin {
                path: "/task/tokens".into(),
                bytes: 8192,
                sha256: digest(TOKENS_SHA),
            },
        },
        mode: wire::InputMode::TeacherForced,
        tiles_image: pin.clone(),
        prefix_image: pin,
        evidence_directory: "/task/evidence".into(),
        dispatch_timeout_ms: 100,
        child_deadline_ms: 3_600_000,
    }
}
pub(super) struct Temp(pub(super) PathBuf);
impl Temp {
    pub(super) fn new() -> Self {
        static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "prefix-decode-parent-{}-{}",
            std::process::id(),
            N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn prefix_decode_parent_closed_config_no_old_profile_or_cache_options() {
    for mode in [
        wire::InputMode::TeacherForced,
        wire::InputMode::Autoregressive,
    ] {
        let mut c = config();
        c.mode = mode;
        Config::parse(&serde_json::to_vec(&c).unwrap()).unwrap();
        for change in 0..9 {
            let mut bad = c.clone();
            match change {
                0 => bad.schema = "FerricFiniteTilesDecodeRequestV1".into(),
                1 => bad.prefix_image.bytes = 0,
                2 => bad.prefix_image.bytes = wire::MAX_IMAGE_BYTES as u64 + 1,
                3 => bad.prefix_image.sha256 = [0; 32],
                4 => bad.tiles_image.path = "relative".into(),
                5 => bad.device_ids[1] = bad.device_ids[0],
                6 => bad.prompt.tokens.sha256[0] ^= 1,
                7 => bad.child_deadline_ms += 1,
                _ => bad.dispatch_timeout_ms = 10001,
            }
            assert!(bad.validate().is_err());
        }
        for name in ["prefix_image", "tiles_image", "mode"] {
            let mut v = serde_json::to_value(&c).unwrap();
            v.as_object_mut().unwrap().remove(name);
            assert!(Config::parse(&serde_json::to_vec(&v).unwrap()).is_err());
        }
        for name in ["kernel_admission", "fallback", "forwards", "input_tokens"] {
            let mut v = serde_json::to_value(&c).unwrap();
            v[name] = true.into();
            assert!(Config::parse(&serde_json::to_vec(&v).unwrap()).is_err());
        }
    }
}
#[test]
fn prefix_decode_parent_retains_both_image_bodies_and_rejects_mutation() {
    let t = Temp::new();
    let mut c = config();
    for (name, bytes, pin) in [
        ("prefix", vec![1, 2, 3], &mut c.prefix_image),
        ("mlp", vec![4, 5], &mut c.tiles_image),
    ] {
        pin.path = t.0.join(name);
        pin.bytes = bytes.len() as u64;
        pin.sha256 = hash(&bytes);
        std::fs::write(&pin.path, bytes).unwrap();
    }
    assert_eq!(c.read_prefix_image().unwrap(), [1, 2, 3]);
    assert_eq!(c.read_tiles_image().unwrap(), [4, 5]);
    std::fs::write(&c.prefix_image.path, [1, 2, 4]).unwrap();
    assert!(c.read_prefix_image().is_err());
    c.tiles_image.bytes += 1;
    assert!(c.read_tiles_image().is_err());
}
#[test]
fn prefix_decode_parent_real_pool_tf_ar_commits_only_after_retained_completion() {
    for mode in [
        wire::InputMode::TeacherForced,
        wire::InputMode::Autoregressive,
    ] {
        let b = bootstrap(mode);
        let t = Temp::new();
        let directory = t.0.join("evidence");
        let mut evidence = evidence::Evidence::create(&directory).unwrap();
        let scope = EngineeringTpPoolScopeV1 {
            model: [2; 32],
            session: [3; 32],
        };
        let mut pool = EngineeringTpPagedPoolV1::new(
            scope,
            EngineeringTpPagedLimitsV1::new(2304, 32, 144, 1024).unwrap(),
        )
        .unwrap();
        let sequence = pool.open_sequence(scope, &[9112], 0).unwrap().sequence();
        let mut last = None;
        let mut stable = None;
        let mut produced = wire::Chain::new(b.registration, b.sha256().unwrap());
        let mut checked = wire::Chain::new(b.registration, b.sha256().unwrap());
        for p in 0..4 {
            let token = b.input(p, last).unwrap();
            assert_eq!(
                token,
                if mode == wire::InputMode::TeacherForced {
                    INPUT_TOKENS[p as usize]
                } else if p == 0 {
                    9112
                } else {
                    p + 6
                }
            );
            let batch = pool
                .reserve_batch(&[EngineeringTpPageRowV1 {
                    sequence,
                    token,
                    position: p,
                }])
                .unwrap();
            pool.begin_submission(&batch).unwrap();
            let md = metadata(&batch, [4; 32], 1_000_000).unwrap();
            stable_pages(&mut stable, md.cache_metadata()).unwrap();
            let mut r = request(&b, p, last);
            if let wire::Command::Forward {
                cache_metadata,
                rotary_bits,
                ..
            } = &mut r.command
            {
                *cache_metadata = md.cache_metadata().to_vec();
                *rotary_bits = md.rotary().iter().map(|v| v.to_bits()).collect();
            }
            let (s, c, raw) = completed(&r, &mut produced, p + 7);
            let output = validate_completion(&r, &s, &c, &raw, &mut checked)
                .unwrap()
                .output_token;
            assert_eq!(pool.committed_position(sequence).unwrap(), p);
            evidence.append(&r, &s, &c, &raw).unwrap();
            assert_eq!(
                std::fs::read(directory.join(format!("observation-{p}.bin"))).unwrap(),
                raw
            );
            assert_eq!(pool.committed_position(sequence).unwrap(), p);
            // Synthetic CPU completion is not a native execution assertion.
            last = Some(commit_completed(&mut pool, &batch, Ok(output)).unwrap());
            assert_eq!(pool.committed_position(sequence).unwrap(), p + 1);
        }
        let mut changed = [0; 145];
        changed[1..].copy_from_slice(&stable.unwrap());
        changed.swap(1, 2);
        assert!(stable_pages(&mut stable, &changed).is_err());
        let files = evidence.finish(&[]).unwrap();
        assert_eq!(files.frames.len(), 4);
        assert!(!directory.join("complete.json").exists());
    }
}
#[test]
fn prefix_decode_parent_failed_retention_quarantines_without_frontier_commit() {
    let b = bootstrap(wire::InputMode::TeacherForced);
    let t = Temp::new();
    let directory = t.0.join("evidence");
    let mut evidence = evidence::Evidence::create(&directory).unwrap();
    let scope = EngineeringTpPoolScopeV1 {
        model: [2; 32],
        session: [3; 32],
    };
    let mut pool = EngineeringTpPagedPoolV1::new(
        scope,
        EngineeringTpPagedLimitsV1::new(2304, 32, 144, 1024).unwrap(),
    )
    .unwrap();
    let sequence = pool.open_sequence(scope, &[9112], 0).unwrap().sequence();
    let batch = pool
        .reserve_batch(&[EngineeringTpPageRowV1 {
            sequence,
            token: 9112,
            position: 0,
        }])
        .unwrap();
    pool.begin_submission(&batch).unwrap();
    let r = request(&b, 0, None);
    let (s, c, raw) = completed(
        &r,
        &mut wire::Chain::new(b.registration, b.sha256().unwrap()),
        7,
    );
    std::fs::write(directory.join("request-0.json"), b"occupied").unwrap();
    let result = (|| {
        let output = validate_completion(
            &r,
            &s,
            &c,
            &raw,
            &mut wire::Chain::new(b.registration, b.sha256().unwrap()),
        )?
        .output_token;
        evidence.append(&r, &s, &c, &raw)?;
        Ok(output)
    })();
    assert_eq!(pool.committed_position(sequence).unwrap(), 0);
    assert!(commit_completed(&mut pool, &batch, result).is_err());
    assert!(pool.committed_position(sequence).is_err());
    assert_eq!(pool.stats().quarantined_pages, 144);
    assert!(
        pool.commit_batch(
            &batch,
            EngineeringTpBatchCompletionV1::after_all_ranks(&batch)
        )
        .is_err()
    );
    assert!(evidence.finish(&[]).is_err());
    assert!(!directory.join("complete.json").exists());
}
#[test]
fn prefix_decode_parent_rejects_wrong_mode_identity_states_output_and_chain() {
    let b = bootstrap(wire::InputMode::TeacherForced);
    let r = request(&b, 0, None);
    let (s, c, raw) = completed(
        &r,
        &mut wire::Chain::new(b.registration, b.sha256().unwrap()),
        7,
    );
    for change in 0..9 {
        let mut s = s.clone();
        let mut c = c.clone();
        let mut raw = raw.clone();
        match change {
            0 => s.profile_sha256[0] ^= 1,
            1 => {
                if let wire::Event::Completed(v) = &mut s.event {
                    v.generation = 3;
                }
            }
            2 => c.layers[35].prefix_states[1][283] = 0,
            3 => c.layers[35].tiles_states[1][547] = 0,
            4 => {
                raw.pop();
            }
            5 => {
                if let wire::Event::Completed(v) = &mut s.event {
                    v.output_token = 8;
                }
            }
            6 => {
                if let wire::Event::Completed(v) = &mut s.event {
                    v.chain[0] ^= 1;
                }
            }
            7 => s.native_closed = true,
            _ => {
                if let wire::Event::Completed(v) = &mut s.event {
                    v.input_token = 0;
                }
            }
        }
        assert!(
            validate_completion(
                &r,
                &s,
                &c,
                &raw,
                &mut wire::Chain::new(b.registration, b.sha256().unwrap())
            )
            .is_err()
        );
    }
}
#[test]
fn prefix_decode_parent_close_requires_four_transcript_no_body_and_true_native_close() {
    let b = bootstrap(wire::InputMode::TeacherForced);
    let digest = [8; 32];
    let (r, s) = wire::tests::close(&b, digest);
    validate_close(&r, &s, None, &[], digest).unwrap();
    for change in 0..4 {
        let mut s = s.clone();
        match change {
            0 => s.id = 4,
            1 => s.native_closed = false,
            2 => {
                s.event = wire::Event::Closed {
                    completed_forwards: 3,
                    transcript_sha256: digest,
                }
            }
            _ => {
                s.event = wire::Event::Closed {
                    completed_forwards: 4,
                    transcript_sha256: [9; 32],
                }
            }
        }
        assert!(validate_close(&r, &s, None, &[], digest).is_err());
    }
    assert!(validate_close(&r, &s, None, &[0], digest).is_err());
    assert!(validate_close(&r, &s, Some(&wire::tests::control()), &[], digest).is_err());
}
#[test]
fn prefix_decode_parent_optin_refuses_before_any_files_or_child() {
    assert!(run(config(), false).err().unwrap().contains("opt-in"));
}
