use super::*;

fn arguments() -> Vec<String> {
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
        "full",
        "--reference",
        "/reference",
        "--reference-sha256",
        &"2".repeat(64),
        "--allow-unauthenticated-machine-code",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn reference(prefill: Prefill) -> Reference {
    let prompt = [1, 2, 3, 4, 5];
    let mut previous = None;
    let mut steps = Vec::new();
    for ordinal in 0..prefill.steps() {
        let plan = scheduled(prefill, &prompt, ordinal, previous).unwrap();
        let cursor = plan.positions.last().unwrap() + 1;
        let choice = 100 + cursor;
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
        schema: "FerricDraftPagedCanaryReferenceV10".into(),
        model: DRAFT_REPOSITORY.into(),
        model_revision: DRAFT_REVISION.into(),
        identity: ModelIdentity {
            model_bundle_id: "1".repeat(64),
            draft_model_id: "2".repeat(64),
            draft_config_id: "3".repeat(64),
            draft_weights_sha256: "4".repeat(64),
        },
        head_precision: "fp32-v10".into(),
        prefill: prefill.label().into(),
        prompt_tokens: prompt,
        steps,
        generated_tokens: [105, 106],
        generated_utf8_bytes: b" output".to_vec(),
        producer: "independent pinned CPU fixture; no GPU claim".into(),
    }
}

fn parse_reference(value: &Reference) -> Result<Reference, String> {
    let bytes = serde_json::to_vec(value).unwrap();
    Reference::parse(&bytes, &digest(&bytes))
}

#[test]
fn options_require_explicit_profiles_reference_and_consent() {
    let options = Options::parse(arguments().into_iter()).unwrap();
    assert_eq!(options.prefill, Prefill::Full);
    assert_eq!(options.projection, EngineeringTpProjectionModeV3::Mfma);
    assert!(!options.runtime.cache_admission && !options.runtime.operational);
    for flag in [
        "--reference",
        "--reference-sha256",
        "--projection",
        "--prefill",
        "--allow-unauthenticated-machine-code",
    ] {
        let mut args = arguments();
        let index = args.iter().position(|s| s == flag).unwrap();
        args.remove(index);
        if flag != "--allow-unauthenticated-machine-code" {
            args.remove(index);
        }
        assert!(Options::parse(args.into_iter()).is_err());
    }
}

#[test]
fn unsupported_modes_zero_device_duplicate_options_and_hashes_reject() {
    for (flag, value) in [
        ("--projection", "auto"),
        ("--projection", "wave"),
        ("--prefill", "other"),
        ("--device-unique-id", "0"),
        ("--worker-sha256", "ABC"),
    ] {
        let mut args = arguments();
        let index = args.iter().position(|s| s == flag).unwrap();
        args[index + 1] = value.into();
        assert!(Options::parse(args.into_iter()).is_err());
    }
    for extra in [
        vec!["--prefill", "full"],
        vec!["--numerical-capture", "yes"],
        vec!["--runtime-rollover", "true"],
        vec!["--dispatch-sequences", "true"],
    ] {
        let mut args = arguments();
        args.extend(extra.into_iter().map(str::to_owned));
        assert!(Options::parse(args.into_iter()).is_err());
    }
}

#[test]
fn only_named_existing_runtime_options_can_be_enabled() {
    let mut args = arguments();
    args.extend([
        "--runtime-cache-admission".into(),
        "--runtime-operational".into(),
    ]);
    let options = Options::parse(args.clone().into_iter()).unwrap();
    assert!(options.runtime.cache_admission && options.runtime.operational);
    assert!(
        !options.runtime.rollover
            && !options.runtime.sequences
            && !options.runtime.ordered_batches
            && !options.runtime.profile
            && !options.runtime.shared_full_currentness
    );
    args.push("--runtime-operational".into());
    assert!(Options::parse(args.into_iter()).is_err());
}

#[test]
fn both_schedules_are_frozen_and_decode_requires_a_completed_prior_choice() {
    let prompt = [1, 2, 3, 4, 5];
    assert_eq!(
        scheduled(Prefill::Full, &prompt, 0, None).unwrap(),
        Scheduled {
            inputs: prompt.to_vec(),
            positions: vec![0, 1, 2, 3, 4],
            selected_row: 4
        }
    );
    assert_eq!(
        scheduled(Prefill::Full, &prompt, 1, Some(77)).unwrap(),
        Scheduled {
            inputs: vec![77],
            positions: vec![5],
            selected_row: 0
        }
    );
    for ordinal in 0..5 {
        let schedule = scheduled(Prefill::Tokenwise, &prompt, ordinal, None).unwrap();
        assert_eq!(schedule.inputs, [prompt[ordinal]]);
        assert_eq!(schedule.positions, [u32::try_from(ordinal).unwrap()]);
    }
    for prefill in [Prefill::Full, Prefill::Tokenwise] {
        assert!(scheduled(prefill, &prompt, prefill.steps() - 1, None).is_err());
        assert!(scheduled(prefill, &prompt, prefill.steps(), Some(7)).is_err());
        assert!(scheduled(prefill, &prompt, 0, Some(151_936)).is_err());
    }
}

#[test]
fn valid_references_bind_every_selected_row_and_generated_input() {
    for prefill in [Prefill::Full, Prefill::Tokenwise] {
        let expected = reference(prefill);
        let parsed = parse_reference(&expected).unwrap();
        parsed
            .bind(&expected.identity, &expected.prompt_tokens)
            .unwrap();
        let observation = Observation {
            generated_tokens: expected.generated_tokens,
            steps: expected
                .steps
                .iter()
                .enumerate()
                .map(|(ordinal, step)| ObservedStep {
                    inputs: step.inputs.clone(),
                    positions: step.positions.clone(),
                    selected_row: step.selected_row,
                    choice: step.choice,
                    cache_tokens: step.cache_tokens,
                    completed_dispatches: u64::try_from(ordinal + 1).unwrap() * 480,
                })
                .collect(),
        };
        assert!(parsed.matches(&observation, &expected.generated_utf8_bytes));
        assert!(!parsed.matches(&observation, b"wrong"));
        let mut wrong = observation;
        wrong.steps[0].choice += 1;
        assert!(!parsed.matches(&wrong, &expected.generated_utf8_bytes));
    }
}

#[test]
fn mutated_reference_schedule_profile_and_identity_are_rejected() {
    for prefill in [Prefill::Full, Prefill::Tokenwise] {
        for case in 0..12 {
            let mut value = reference(prefill);
            match case {
                0 => value.schema = "FerricDraftCanaryReferenceV1".into(),
                1 => value.head_precision = "bf16".into(),
                2 => value.model = "other".into(),
                3 => value.prefill = "unknown".into(),
                4 => value.steps[0].inputs[0] += 1,
                5 => value.steps[0].positions[0] += 1,
                6 => value.steps[0].selected_row += 1,
                7 => value.steps[0].cache_tokens += 1,
                8 => value.steps[0].choice = 151_936,
                9 => value.generated_tokens[0] += 1,
                10 => {
                    value.steps.pop();
                }
                _ => value.identity.draft_model_id = "not a hash".into(),
            }
            assert!(parse_reference(&value).is_err(), "case{case}");
        }
    }
}

#[test]
fn reference_rejects_raw_hash_drift_extra_fields_and_wrong_runtime_binding() {
    let expected = reference(Prefill::Full);
    let bytes = serde_json::to_vec(&expected).unwrap();
    assert!(Reference::parse(&bytes, &"0".repeat(64)).is_err());
    let mut value = serde_json::to_value(&expected).unwrap();
    value["extra"] = serde_json::json!(true);
    let bytes = serde_json::to_vec(&value).unwrap();
    assert!(Reference::parse(&bytes, &digest(&bytes)).is_err());
    let mut identity = expected.identity.clone();
    identity.model_bundle_id = "5".repeat(64);
    assert!(expected.bind(&identity, &expected.prompt_tokens).is_err());
    assert!(expected.bind(&expected.identity, &[9, 2, 3, 4, 5]).is_err());
}

#[test]
fn malformed_choices_and_failed_close_never_qualify() {
    for values in [&[][..], &[1, 2][..], &[151_936][..]] {
        assert!(checked_choice(values).is_err());
    }
    assert_eq!(checked_choice(&[7]).unwrap(), 7);
    assert!(finish_run(Ok(true), Ok(())).is_ok());
    assert!(finish_run(Ok(false), Ok(())).is_err());
    assert!(finish_run(Ok(true), Err("close".into())).is_err());
    assert!(finish_run(Err("submit".into()), Ok(())).is_err());
    assert!(finish_run(Err("submit".into()), Err("close".into())).is_err());
}
