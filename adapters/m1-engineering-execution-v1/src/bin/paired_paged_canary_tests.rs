//! Synthetic host contract checks only; these do not produce native evidence.

use super::*;

fn arguments() -> Vec<String> {
    [
        "--source",
        "/source",
        "--target-artifact",
        "/target",
        "--target-head-artifact",
        "/head",
        "--draft-artifact",
        "/draft",
        "--worker",
        "/worker",
        "--worker-sha256",
        SOURCE_REFERENCE_SHA256,
        "--device-unique-id",
        "1",
        "--reference",
        "/adapted",
        "--reference-sha256",
        SOURCE_REFERENCE_SHA256,
        "--target-reference",
        "/original",
        "--allow-unauthenticated-machine-code",
        "--runtime-cache-admission",
        "--runtime-operational",
        "--runtime-rollover",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

// Structural fixture intentionally cannot pass the immutable original-byte pin.
fn structural_reference() -> (SourceReference, Reference) {
    let source = SourceReference {
        schema: "FerricMatched128ReferenceV1".into(),
        generated_token_ids: (0..128).collect(),
        generated_utf8_hex: "61".repeat(128),
        independent_producer:
            "offline-stock-HF-Qwen3-BF16-eager-decoder-explicit-FP32-F.linear-head".into(),
        policy: "exact-greedy-token-ids-and-decoded-utf8-v1".into(),
        producer_evidence_sha256: "0".repeat(64),
        producer_source_sha256: "1".repeat(64),
        prompt_token_ids: vec![7; 128],
        target_manifest_sha256: "2".repeat(64),
        workload_sha256: "3".repeat(64),
    };
    let reference = Reference {
        schema: "FerricPairedPagedK4ReferenceV1".into(),
        source_reference_sha256: SOURCE_REFERENCE_SHA256.into(),
        prompt_token_ids: source.prompt_token_ids.clone(),
        generated_token_ids: source.generated_token_ids.clone(),
        prefix_utf8_hex: (1..=10).map(|n| "61".repeat(n)).collect(),
        tokenizer_sha256: "4".repeat(64),
        producer_source_sha256: "5".repeat(64),
    };
    (source, reference)
}

#[test]
fn closed_options_enable_only_the_fixed_supported_runtime() {
    let options = Options::parse(arguments().into_iter()).unwrap();
    assert_eq!(options.device, 1);
    assert!(
        options.runtime.cache_admission && options.runtime.operational && options.runtime.rollover
    );
    assert!(
        !options.runtime.sequences && !options.runtime.ordered_batches && !options.runtime.profile
    );
    for flag in [
        "--allow-unauthenticated-machine-code",
        "--runtime-cache-admission",
        "--runtime-operational",
        "--runtime-rollover",
    ] {
        let mut missing = arguments();
        missing.retain(|value| value != flag);
        assert!(Options::parse(missing.into_iter()).is_err(), "{flag}");
        let mut duplicate = arguments();
        duplicate.push(flag.into());
        assert!(Options::parse(duplicate.into_iter()).is_err(), "{flag}");
    }
}

#[test]
fn alternative_modes_choices_and_incomplete_options_are_rejected() {
    for extra in [
        "--runtime-ordered-batches",
        "--runtime-sequences",
        "--attention",
        "--prefix-cache",
        "--draft-tokens",
        "--accepted",
        "--rounds",
        "--k",
    ] {
        let mut args = arguments();
        args.extend([extra.into(), "4".into()]);
        assert!(Options::parse(args.into_iter()).is_err(), "{extra}");
    }
    for flag in [
        "--source",
        "--worker-sha256",
        "--reference",
        "--reference-sha256",
        "--target-reference",
        "--target-artifact",
        "--target-head-artifact",
        "--draft-artifact",
    ] {
        let mut args = arguments();
        let index = args.iter().position(|arg| arg == flag).unwrap();
        args.drain(index..index + 2);
        assert!(Options::parse(args.into_iter()).is_err(), "{flag}");
    }
    let mut args = arguments();
    let index = args
        .iter()
        .position(|arg| arg == "--device-unique-id")
        .unwrap();
    args[index + 1] = "0".into();
    assert!(Options::parse(args.into_iter()).is_err());
    assert!(Options::parse(["--source".into()].into_iter()).is_err());
}

#[test]
fn lowercase_exact_hash_and_independent_original_byte_pin_are_required() {
    assert_eq!(
        digest(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    for hash in ["", "ab", &"A".repeat(64), &"g".repeat(64), &"0".repeat(65)] {
        assert!(checked_sha(hash).is_err());
    }
    let (source, reference) = structural_reference();
    let original = serde_json::to_vec(&source).unwrap();
    let adapted = serde_json::to_vec(&reference).unwrap();
    assert!(Reference::parse(&original, &adapted, &digest(&adapted)).is_err());
    assert!(Reference::parse(&[], &adapted, &digest(&adapted)).is_err());
    assert!(Reference::parse(&original, &[], &digest(b"")).is_err());
}

#[test]
fn structural_reference_requires_unchanged_full_arrays_and_exact_prefixes() {
    let (source, reference) = structural_reference();
    reference.validate_source(&source).unwrap();
    for count in 2..=10 {
        assert!(reference.matches(&reference.generated_token_ids[..count], &vec![b'a'; count]));
        assert!(!reference.matches(&reference.generated_token_ids[..count], &vec![b'b'; count]));
    }
    assert!(!reference.matches(&[], b""));
    assert!(!reference.matches(&[0], b"a"));
    assert!(!reference.matches(&reference.generated_token_ids[..11], &[b'a'; 11]));
    for case in 0..13 {
        let mut changed = reference.clone();
        match case {
            0 => changed.schema.push('x'),
            1 => changed.source_reference_sha256 = "0".repeat(64),
            2 => {
                changed.prompt_token_ids.pop();
            }
            3 => changed.prompt_token_ids[127] += 1,
            4 => changed.generated_token_ids[127] += 1,
            5 => {
                changed.prefix_utf8_hex.pop();
            }
            6 => changed.prefix_utf8_hex[0].clear(),
            7 => changed.prefix_utf8_hex[0] = "6".into(),
            8 => changed.prefix_utf8_hex[0] = "6A".into(),
            9 => changed.prefix_utf8_hex[1] = "62".into(),
            10 => changed.prefix_utf8_hex[1] = "61".repeat(16_385),
            11 => changed.tokenizer_sha256 = "G".repeat(64),
            _ => changed.producer_source_sha256.clear(),
        }
        assert!(changed.validate_source(&source).is_err(), "mutation {case}");
    }
    let mut invalid = source.clone();
    let mut changed = reference;
    invalid.prompt_token_ids[0] = VOCABULARY;
    changed.prompt_token_ids[0] = VOCABULARY;
    assert!(changed.validate_source(&invalid).is_err());
}

#[test]
fn typed_reference_contract_rejects_unknown_and_duplicate_fields() {
    let (source, reference) = structural_reference();
    let mut value = serde_json::to_value(&reference).unwrap();
    value["draft_tokens"] = serde_json::json!([1, 2, 3, 4]);
    assert!(serde_json::from_value::<Reference>(value).is_err());
    let mut value = serde_json::to_value(&source).unwrap();
    value["force_accept"] = serde_json::json!(true);
    assert!(serde_json::from_value::<SourceReference>(value).is_err());
    assert!(serde_json::from_str::<Reference>(r#"{"schema":"a","schema":"b"}"#).is_err());
}

#[test]
fn packet_budget_is_derived_for_eight_prefills_eight_proposals_and_two_catchups() {
    let budget = PacketBudget::new(&[613], &[616], &[477], &[480]).unwrap();
    assert_eq!(
        (PREFIX_TOKENS, PREFILL_BATCHES, CONTEXT, PAGES),
        (127, 8, 160, 10)
    );
    assert_eq!(budget.target_total, 6136);
    assert_eq!(budget.draft_total_max, 8610);
    for count in [0, 1, 612, 614, u64::MAX] {
        assert!(PacketBudget::new(&[count], &[616], &[477], &[480]).is_err());
    }
    assert!(PacketBudget::new(&[], &[616], &[477], &[480]).is_err());
    assert!(PacketBudget::new(&[613, 613], &[616], &[477], &[480]).is_err());
    assert!(PacketBudget::new(&[613], &[613], &[477], &[480]).is_err());
    assert!(PacketBudget::new(&[613], &[616], &[480], &[480]).is_err());
    assert!(PacketBudget::new(&[613], &[616], &[477], &[477]).is_err());
}

#[test]
fn constructor_only_index_leaves_the_anchor_nonresident() {
    let plan_id = Identity::new([6; 32]);
    let index = bootstrap_index(323, plan_id).unwrap();
    assert_eq!(
        (index.target_pre_committed, index.draft_pre_committed),
        (127, 127)
    );
    assert_eq!(
        (index.target_tentative.end, index.draft_tentative.end),
        (132, 131)
    );
    assert_eq!(index.round_anchor, 323);
    assert_eq!(index.draft_token_count, 4);
    assert_eq!(index.draft_tokens, [0; 16]);
    assert_eq!(index.plan_id, plan_id);
    for accepted in 0..=4 {
        assert_eq!(
            index.target_commit_ends[accepted],
            128 + u32::try_from(accepted).unwrap()
        );
        assert_eq!(
            index.draft_commit_ends[accepted],
            127 + u32::try_from((accepted + 1).min(4)).unwrap()
        );
    }
}

#[test]
fn parity_and_both_unforced_closes_are_required_for_success() {
    assert!(finish_pair(Ok(true), Ok(()), Ok(())).is_ok());
    assert!(finish_pair(Ok(false), Ok(()), Ok(())).is_err());
    for target_ok in [false, true] {
        for draft_ok in [false, true] {
            let target = if target_ok {
                Ok(())
            } else {
                Err("target close failed".into())
            };
            let draft = if draft_ok {
                Ok(())
            } else {
                Err("draft close failed".into())
            };
            assert!(
                finish_pair(
                    Err("submitted execution failed".into()),
                    target.clone(),
                    draft.clone()
                )
                .is_err()
            );
            assert_eq!(
                finish_pair(Ok(true), target, draft).is_ok(),
                target_ok && draft_ok
            );
        }
    }
}
