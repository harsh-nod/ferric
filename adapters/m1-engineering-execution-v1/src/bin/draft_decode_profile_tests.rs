use super::*;

fn arguments(prefill: &str, tokens: usize) -> Vec<String> {
    [
        "--source",
        "/source",
        "--artifact",
        "/artifact",
        "--worker",
        "/worker",
        "--worker-sha256",
        &"1".repeat(64),
        "--device-unique-id",
        "1",
        "--projection",
        "mfma",
        "--prefill",
        prefill,
        "--reference",
        "/reference",
        "--reference-sha256",
        &"2".repeat(64),
        "--allow-unauthenticated-machine-code",
        "--new-tokens",
        &tokens.to_string(),
        "--warmups",
        "0",
        "--samples",
        "1",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn options(prefill: &str, tokens: usize) -> Options {
    Options::parse(arguments(prefill, tokens).into_iter()).unwrap()
}

fn reference(options: &Options) -> Reference {
    let prompt = [1, 2, 3, 4, 5];
    let mut previous = None;
    let mut steps = Vec::new();
    let mut generated = Vec::new();
    for ordinal in 0..forward_count(options.common.prefill, options.new_tokens) {
        let plan = scheduled(
            options.common.prefill,
            &prompt,
            options.new_tokens,
            ordinal,
            previous,
        )
        .unwrap();
        let cursor = plan.positions.last().unwrap() + 1;
        let choice = 100 + cursor;
        if cursor >= 5 {
            generated.push(choice);
        }
        steps.push(ReferenceStep {
            inputs: plan.inputs,
            positions: plan.positions,
            selected_row: plan.selected_row,
            choice,
            cache_tokens: cursor,
        });
        previous = Some(choice);
    }
    Reference {
        schema: "FerricDraftDecodeProfileReferenceV1".into(),
        model: DRAFT_REPOSITORY.into(),
        model_revision: DRAFT_REVISION.into(),
        identity: ModelIdentity {
            model_bundle_id: "1".repeat(64),
            draft_model_id: "2".repeat(64),
            draft_config_id: "3".repeat(64),
            draft_weights_sha256: "4".repeat(64),
        },
        head_precision: "fp32-v10".into(),
        prefill: options.common.prefill.label().into(),
        prompt_tokens: prompt,
        steps,
        generated_tokens: generated,
        generated_utf8_bytes: b"synthetic".to_vec(),
        producer: "SYNTHETIC CPU fixture, not a model observation".into(),
    }
}

fn parse(value: &Reference, options: &Options) -> Result<Reference, String> {
    let bytes = serde_json::to_vec(value).unwrap();
    Reference::parse(&bytes, &digest(&bytes), options)
}

#[test]
fn every_supported_schedule_is_causal_bounded_and_includes_page_crossings() {
    for tokens in 2..=128 {
        for prefill in ["full", "tokenwise"] {
            let options = options(prefill, tokens);
            let value = reference(&options);
            parse(&value, &options).unwrap();
            assert_eq!(
                value.steps.last().unwrap().cache_tokens as usize,
                tokens + 4
            );
            assert_eq!(value.generated_tokens.len(), tokens);
            assert!(options.context_tokens() as usize >= tokens + 4);
            assert!((options.context_tokens() as usize) < tokens + 20);
            assert_eq!(options.sampling_class(), "diagnostic-only");
        }
    }
}

#[test]
fn explicit_sampling_bounds_and_no_implicit_warmup_exclusion() {
    for (flag, value) in [
        ("--new-tokens", "1"),
        ("--new-tokens", "129"),
        ("--warmups", "11"),
        ("--samples", "0"),
        ("--samples", "31"),
    ] {
        let mut args = arguments("full", 8);
        let index = args.iter().position(|arg| arg == flag).unwrap();
        args[index + 1] = value.into();
        assert!(Options::parse(args.into_iter()).is_err());
    }
    for flag in ["--new-tokens", "--warmups", "--samples"] {
        let mut args = arguments("full", 8);
        let index = args.iter().position(|arg| arg == flag).unwrap();
        args.drain(index..=index + 1);
        assert!(Options::parse(args.into_iter()).is_err());
        let mut args = arguments("full", 8);
        args.extend([flag.into(), "2".into()]);
        assert!(Options::parse(args.into_iter()).is_err());
    }
    let mut args = arguments("full", 4);
    for (flag, value) in [("--warmups", "10"), ("--samples", "30")] {
        let index = args.iter().position(|arg| arg == flag).unwrap();
        args[index + 1] = value.into();
    }
    assert_eq!(
        Options::parse(args.into_iter()).unwrap().sampling_class(),
        "benchmark-sized-unqualified"
    );
}

#[test]
fn combined_sampling_respects_existing_no_rollover_budget() {
    for (prefill, tokens, accepted) in [
        ("full", 6, true),
        ("full", 7, false),
        ("tokenwise", 2, true),
        ("tokenwise", 3, false),
    ] {
        let mut args = arguments(prefill, tokens);
        for (flag, value) in [("--warmups", "10"), ("--samples", "30")] {
            let index = args.iter().position(|arg| arg == flag).unwrap();
            args[index + 1] = value.into();
        }
        assert_eq!(Options::parse(args.into_iter()).is_ok(), accepted);
    }
    assert_eq!(options("full", 128).total_dispatches(), 61_440);
}

#[test]
fn retired_real_pool_is_reusable_without_being_a_fresh_pool() {
    use ferric_m1_engineering_execution_v1::tp_paged::{
        EngineeringTpPagedLimitsV1, EngineeringTpPoolScopeV1,
    };
    let scope = EngineeringTpPoolScopeV1 {
        model: [1; 32],
        session: [2; 32],
    };
    let mut pool = EngineeringTpPagedPoolV1::new_wide32(
        scope,
        EngineeringTpPagedLimitsV1::new(16, 1, 1, 100).unwrap(),
    )
    .unwrap();
    assert!(pool.is_empty());
    require_reusable_pool(&pool, 1).unwrap();
    for run in 0..40 {
        let sequence = pool
            .open_sequence(scope, &[1, 2, 3, 4, 5], run * 2)
            .unwrap()
            .sequence();
        assert!(require_reusable_pool(&pool, 1).is_err());
        pool.retire_sequence(sequence, false, run * 2 + 1).unwrap();
        assert!(!pool.is_empty());
        require_reusable_pool(&pool, 1).unwrap();
    }
    assert!(require_reusable_pool(&pool, 2).is_err());
}

#[test]
fn malformed_reference_fields_and_causal_mutants_reject() {
    let options = options("full", 8);
    let base = reference(&options);
    for mutation in 0..14 {
        let mut value = base.clone();
        match mutation {
            0 => value.schema = "unknown".into(),
            1 => value.model_revision = "main".into(),
            2 => value.head_precision = "bf16".into(),
            3 => value.prefill = "tokenwise".into(),
            4 => value.steps[1].inputs[0] += 1,
            5 => value.steps[1].positions[0] += 1,
            6 => value.steps[0].selected_row = 0,
            7 => value.steps[0].choice = 151_936,
            8 => value.steps[1].cache_tokens += 1,
            9 => value.generated_tokens[0] += 1,
            10 => {
                value.steps.pop();
            }
            11 => value.producer.clear(),
            12 => value.identity.draft_model_id = "not-an-identity".into(),
            13 => value.prompt_tokens[0] = 151_936,
            _ => unreachable!(),
        }
        assert!(parse(&value, &options).is_err(), "mutation {mutation}");
    }
    let mut json = serde_json::to_value(&base).unwrap();
    json["unknown"] = true.into();
    let bytes = serde_json::to_vec(&json).unwrap();
    assert!(Reference::parse(&bytes, &digest(&bytes), &options).is_err());
    let bytes = serde_json::to_vec(&base).unwrap();
    assert!(Reference::parse(&bytes, &"0".repeat(64), &options).is_err());
    assert!(Reference::parse(&[], &digest(&[]), &options).is_err());
}

#[test]
fn old_two_token_reference_is_not_silently_extended() {
    let two = options("full", 2);
    let mut value = reference(&two);
    value.schema = "FerricDraftPagedCanaryReferenceV10".into();
    parse(&value, &two).unwrap();
    assert!(parse(&value, &options("full", 8)).is_err());
    let eight = options("full", 8);
    let mut value = reference(&eight);
    value.schema = "FerricDraftPagedCanaryReferenceV10".into();
    assert!(parse(&value, &eight).is_err());
}

#[test]
fn actual_identity_and_prompt_must_match_before_worker_setup() {
    let options = options("tokenwise", 4);
    let value = reference(&options);
    value.bind(&value.identity, &value.prompt_tokens).unwrap();
    let mut other = value.identity.clone();
    other.draft_weights_sha256 = "5".repeat(64);
    assert!(value.bind(&other, &value.prompt_tokens).is_err());
    let mut prompt = value.prompt_tokens;
    prompt[4] += 1;
    assert!(value.bind(&value.identity, &prompt).is_err());
}

#[test]
fn decoder_never_invents_missing_previous_completion() {
    let prompt = [1, 2, 3, 4, 5];
    assert!(scheduled(Prefill::Full, &prompt, 8, 1, None).is_err());
    assert!(scheduled(Prefill::Tokenwise, &prompt, 8, 5, None).is_err());
    assert!(scheduled(Prefill::Full, &prompt, 8, 8, Some(7)).is_err());
    assert!(scheduled(Prefill::Full, &prompt, 8, 1, Some(151_936)).is_err());
    assert_eq!(
        scheduled(Prefill::Full, &prompt, 8, 1, Some(9))
            .unwrap()
            .inputs,
        [9]
    );
}

#[test]
fn raw_clock_is_real_monotonic_and_offsets_fail_closed() {
    let before = raw_ns().unwrap();
    assert!(raw_ns().unwrap() >= before);
    offset(before).unwrap();
    assert!(offset(u64::MAX).is_err());
}
