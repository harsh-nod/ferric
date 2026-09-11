use super::*;

fn draft_config() -> ModelConfig {
    ModelConfig {
        role: Qwen3ModelRole::Draft06B,
        model_id: ferric_spec::Identity::new([1; 32]),
        config_id: ferric_spec::Identity::new([2; 32]),
        vocabulary_size: 151_936,
        layers: 28,
        hidden_size: 1024,
        intermediate_size: 3072,
        query_heads: 16,
        kv_heads: 8,
        head_dim: 128,
        max_position_embeddings: 40_960,
        rope_theta: 1_000_000,
        tie_word_embeddings: true,
    }
}

fn identity() -> ModelIdentity {
    ModelIdentity {
        model_bundle_id: "01".repeat(32),
        draft_model_id: "02".repeat(32),
        draft_config_id: "03".repeat(32),
        draft_weights_sha256: "04".repeat(32),
    }
}

fn reference() -> Reference {
    // Synthetic protocol fixture only; these are not canonical model outputs.
    Reference {
        schema: "FerricDraftCanaryReferenceV1".into(),
        model: DRAFT_REPOSITORY.into(),
        model_revision: DRAFT_REVISION.into(),
        identity: identity(),
        prompt_tokens: [1, 2, 3, 4, 5],
        generated_tokens: [6, 7],
        generated_utf8_bytes: b"fixture".to_vec(),
        producer: "synthetic host protocol fixture, not model reference".into(),
    }
}

fn parse_options(extra: &[&str]) -> Result<Options, String> {
    let mut args = vec![
        "--source",
        "/source",
        "--artifact",
        "/artifact",
        "--worker",
        "/worker",
        "--worker-sha256",
        "0101010101010101010101010101010101010101010101010101010101010101",
        "--allow-unauthenticated-machine-code",
    ];
    args.extend(extra);
    Options::parse(args.into_iter().map(str::to_owned))
}

#[test]
fn canary_scope_is_fixed_tp1_and_no_fast_profile_options_are_accepted() {
    assert_eq!(
        parse_options(&["--device-unique-id", "7"]).unwrap().device,
        7
    );
    for id in ["0", "-1", "1,2", "", "18446744073709551616"] {
        assert!(parse_options(&["--device-unique-id", id]).is_err());
    }
    for (flag, value) in [
        ("--devices", "1,2"),
        ("--capacity", "32"),
        ("--new-tokens", "2"),
        ("--repetitions", "1"),
        ("--projection", "mfma"),
        ("--runtime-ordered-batches", "true"),
        ("--model-role", "target"),
        ("--prompt", ""),
    ] {
        assert!(parse_options(&["--device-unique-id", "7", flag, value]).is_err());
    }
    assert!(parse_options(&["--device-unique-id", "7", "--device-unique-id", "8"]).is_err());
    assert!(parse_options(&[]).is_err());
    assert!(Options::parse(["--device-unique-id".into(), "7".into()].into_iter()).is_err());
    assert!(parse_options(&["--device-unique-id", "7", "--reference", "/reference"]).is_err());
    assert!(
        parse_options(&[
            "--device-unique-id",
            "7",
            "--reference-sha256",
            &"01".repeat(32)
        ])
        .is_err()
    );
}

#[test]
fn canonical_geometry_role_tie_and_payload_are_required_without_allocating_weights() {
    let model = draft_config();
    assert!(
        validate_draft_shape(
            model,
            QWEN3_DRAFT_TENSOR_DATA_BYTES,
            QWEN3_DRAFT_TENSOR_COUNT
        )
        .is_ok()
    );
    assert_eq!(u64::from(model.layers) * 15 + 4, 424);
    assert_eq!(u64::from(STEPS) * 424, PACKETS);
    assert!(
        u64::from(STEPS) * (u64::from(model.layers) * 15 + 4)
            <= fe2o3_kfd::engineering_wire::MAX_UNRETIRED_RING_PACKETS_V1
    );
    for length in [
        0,
        QWEN3_DRAFT_TENSOR_DATA_BYTES - 1,
        QWEN3_DRAFT_TENSOR_DATA_BYTES + 1,
        ferric_spec::QWEN3_TARGET_TENSOR_DATA_BYTES,
        u64::MAX,
    ] {
        assert!(validate_draft_shape(model, length, QWEN3_DRAFT_TENSOR_COUNT).is_err());
    }
    for count in [0, 310, 312, 399] {
        assert!(validate_draft_shape(model, QWEN3_DRAFT_TENSOR_DATA_BYTES, count).is_err());
    }
    let mut mutations = Vec::new();
    let mut candidate = model;
    candidate.role = Qwen3ModelRole::Target8B;
    mutations.push(candidate);
    let mut candidate = model;
    candidate.tie_word_embeddings = false;
    mutations.push(candidate);
    let mut candidate = model;
    candidate.hidden_size = 4096;
    mutations.push(candidate);
    let mut candidate = model;
    candidate.intermediate_size = 12288;
    mutations.push(candidate);
    let mut candidate = model;
    candidate.query_heads = 32;
    mutations.push(candidate);
    let mut candidate = model;
    candidate.kv_heads = 4;
    mutations.push(candidate);
    let mut candidate = model;
    candidate.layers = 36;
    mutations.push(candidate);
    let mut candidate = model;
    candidate.vocabulary_size = 151_935;
    mutations.push(candidate);
    for candidate in mutations {
        assert!(
            validate_draft_shape(
                candidate,
                QWEN3_DRAFT_TENSOR_DATA_BYTES,
                QWEN3_DRAFT_TENSOR_COUNT
            )
            .is_err()
        );
    }
}

#[test]
fn tied_weight_check_uses_exact_bytes_and_each_authenticated_digest() {
    let bytes = [0, 1, 254, 255, 0x80, 0x3f];
    let hash: [u8; 32] = Sha256::digest(bytes).into();
    assert!(validate_tied_pair(&bytes, &bytes, hash, hash).is_ok());
    assert!(validate_tied_pair(&bytes, &bytes[..4], hash, hash).is_err());
    assert!(validate_tied_pair(&bytes, &[1, 0, 254, 255, 0x80, 0x3f], hash, hash).is_err());
    assert!(validate_tied_pair(&bytes, &bytes, [0; 32], hash).is_err());
    assert!(validate_tied_pair(&bytes, &bytes, [0; 32], [0; 32]).is_err());
    let empty_hash: [u8; 32] = Sha256::digest([]).into();
    assert!(validate_tied_pair(&[], &[], empty_hash, empty_hash).is_err());
}

#[test]
fn reference_requires_external_digest_exact_schema_and_draft_identity() {
    let expected = reference();
    let bytes = serde_json::to_vec(&expected).unwrap();
    let parsed = Reference::parse(&bytes, &digest(&bytes)).unwrap();
    parsed.bind(&identity(), &[1, 2, 3, 4, 5]).unwrap();
    assert!(parsed.matches([6, 7], b"fixture"));
    assert!(!parsed.matches([7, 6], b"fixture"));
    assert!(!parsed.matches([6, 7], b"changed"));
    assert!(Reference::parse(&bytes, &"00".repeat(32)).is_err());
    assert!(Reference::parse(&[], &digest(&[])).is_err());
    let oversized = vec![b' '; usize::try_from(MAX_REFERENCE_BYTES).unwrap() + 1];
    assert!(Reference::parse(&oversized, &digest(&oversized)).is_err());
    let mut wrong_identity = identity();
    wrong_identity.draft_model_id = "05".repeat(32);
    assert!(parsed.bind(&wrong_identity, &[1, 2, 3, 4, 5]).is_err());
    assert!(parsed.bind(&identity(), &[1, 2, 3, 4, 9]).is_err());
    let original = serde_json::to_value(&expected).unwrap();
    for (key, value) in [
        ("model", serde_json::json!("Qwen/Qwen3-8B")),
        ("model_revision", serde_json::json!("unknown")),
        ("schema", serde_json::json!("FerricQwen3TpBatchReferenceV1")),
        ("producer", serde_json::json!("")),
        ("prompt_tokens", serde_json::json!([1, 2, 3, 4])),
        ("generated_tokens", serde_json::json!([6, 151_936])),
        ("extra", serde_json::json!(true)),
    ] {
        let mut changed = original.clone();
        changed[key] = value;
        let bytes = serde_json::to_vec(&changed).unwrap();
        assert!(Reference::parse(&bytes, &digest(&bytes)).is_err(), "{key}");
    }
}

#[test]
fn prompt_rows_and_hex_encoding_are_bounded() {
    assert!(validate_prompt(&[1, 2, 3, 4, 5]).is_ok());
    for invalid in [vec![], vec![1; 4], vec![1; 6], vec![151_936; 5]] {
        assert!(validate_prompt(&invalid).is_err());
    }
    assert_eq!(hex(&[0, 15, 16, 255]), "000f10ff");
    for invalid in [
        "x".repeat(64),
        "A".repeat(64),
        "0".repeat(63),
        "0".repeat(65),
    ] {
        assert!(checked_sha256(&invalid).is_err());
    }
}

#[test]
fn close_or_observation_failure_never_returns_a_successful_completion() {
    assert_eq!(finish_run(Ok(42), Ok(())), Ok(42));
    assert!(
        finish_run(Ok(42), Err("close".into()))
            .unwrap_err()
            .contains("close")
    );
    assert!(finish_run::<u32>(Err("reference mismatch".into()), Ok(())).is_err());
    let both = finish_run::<u32>(Err("dispatch".into()), Err("close".into())).unwrap_err();
    assert!(both.contains("dispatch") && both.contains("close"));
}

#[test]
#[ignore = "requires FERRIC_DRAFT_TP_V1_ARTIFACT; exact current roster admission only, no GPU"]
fn retained_closed_tp_v1_image_must_match_current_host_roster() {
    let path = std::env::var_os("FERRIC_DRAFT_TP_V1_ARTIFACT").expect("explicit retained image");
    let roster = ferric_qwen3_tp_kernels_device_v1::compiler_expectation_roster_v1();
    let image = EngineeringTpArtifactV1::open(Path::new(&path), &roster).unwrap();
    assert_eq!(image.inspection().hsaco().kernels().len(), 13);
}
