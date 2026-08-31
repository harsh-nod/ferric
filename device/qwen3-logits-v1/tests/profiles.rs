use ferric_qwen3_logits_device_v1::*;

#[test]
fn argmax_rows_are_exact_sorted_and_fit_the_grid_cap() {
    for (index, rows) in QWEN3_LOGITS_ARGMAX_ROWS_V1.iter().copied().enumerate() {
        assert!(qwen3_logits_rows_are_admitted_v1(rows));
        if index > 0 {
            assert!(QWEN3_LOGITS_ARGMAX_ROWS_V1[index - 1] < rows);
        }
        assert!(rows <= QWEN3_LOGITS_ARGMAX_MAX_GRID_WORKGROUPS_V1 as usize);
    }
    for rejected in [
        0,
        2,
        3,
        6,
        15,
        18,
        31,
        33,
        127,
        129,
        2_047,
        2_049,
        usize::MAX,
    ] {
        assert!(!qwen3_logits_rows_are_admitted_v1(rejected));
    }
}

#[test]
fn compact_catalog_is_exact_distinct_and_physically_bounded() {
    let mut seen = std::collections::BTreeSet::new();
    for profile in QWEN3_LOGITS_COMPACT_PROFILES_V1 {
        assert!(qwen3_logits_compact_profile_is_admitted_v1(
            profile.sequences,
            profile.active_tokens,
            profile.speculative_k
        ));
        assert!(seen.insert((
            profile.sequences,
            profile.active_tokens,
            profile.speculative_k
        )));
        assert!(profile.sequences <= QWEN3_LOGITS_COMPACT_MAX_GRID_WORKGROUPS_V1 as usize);
        assert!(profile.speculative_k <= QWEN3_LOGITS_MAX_SPECULATIVE_K_V1);
        assert!(profile.active_tokens > profile.speculative_k);
        assert_eq!(
            profile.sequences * QWEN3_LOGITS_COMPACT_RECORD_WORDS_V1 * 4,
            profile.sequences * QWEN3_LOGITS_COMPACT_RECORD_BYTES_V1
        );
    }
    assert_eq!(seen.len(), 11);

    for rejected in [
        (0, 1, 0),
        (1, 0, 0),
        (1, 4, 4),
        (8, 9, 8),
        (32, 5, 4),
        (1, 18, 16),
        (1, 17, 15),
        (33, 1, 0),
    ] {
        assert!(!qwen3_logits_compact_profile_is_admitted_v1(
            rejected.0, rejected.1, rejected.2
        ));
    }
}

#[test]
fn compact_row_striped_64x2_owns_every_record_byte_exactly_once() {
    for sequences in [1_usize, 8, 32] {
        let mut owners = std::collections::BTreeMap::new();
        for sequence in 0..sequences {
            for lane in 0..64 {
                for component in 0..2 {
                    let byte = component * 64 + lane;
                    if byte < QWEN3_LOGITS_COMPACT_RECORD_BYTES_V1 {
                        let physical = sequence * QWEN3_LOGITS_COMPACT_RECORD_BYTES_V1 + byte;
                        assert_eq!(owners.insert(physical, (sequence, lane, component)), None);
                    }
                }
            }
        }
        assert_eq!(
            owners.len(),
            sequences * QWEN3_LOGITS_COMPACT_RECORD_BYTES_V1
        );
        assert_eq!(owners.keys().next().copied(), Some(0));
        assert_eq!(
            owners.keys().next_back().copied(),
            Some(sequences * QWEN3_LOGITS_COMPACT_RECORD_BYTES_V1 - 1)
        );
        assert!(
            (0..sequences * QWEN3_LOGITS_COMPACT_RECORD_BYTES_V1)
                .all(|byte| owners.contains_key(&byte))
        );
    }
}

#[test]
fn speculative_assembly_profiles_are_the_exact_four_target_shapes() {
    for profile in QWEN3_SPECULATIVE_TOKEN_ASSEMBLY_PROFILES_V1 {
        assert!(qwen3_speculative_token_assembly_profile_is_admitted_v1(
            profile.sequences,
            profile.speculative_k
        ));
        assert!(profile.sequences <= 8);
        assert!(profile.speculative_k <= 16);
    }
    for rejected in [(0, 4), (1, 0), (8, 8), (2, 4), (1, 5), (9, 4), (1, 17)] {
        assert!(!qwen3_speculative_token_assembly_profile_is_admitted_v1(
            rejected.0, rejected.1
        ));
    }
}

#[test]
fn explicit_kernarg_sizes_match_the_host_contract() {
    assert_eq!(QWEN3_LOGITS_ARGMAX_EXPLICIT_KERNARG_BYTES_V1, 40);
    assert_eq!(QWEN3_LOGITS_COMPACT_EXPLICIT_KERNARG_BYTES_V1, 144);
    assert_eq!(
        QWEN3_SPECULATIVE_TOKEN_ASSEMBLY_EXPLICIT_KERNARG_BYTES_V1,
        56
    );
    assert_eq!(QWEN3_LOGITS_COMPACT_RECORD_WORDS_V1, 30);
    assert_eq!(QWEN3_LOGITS_COMPACT_RECORD_BYTES_V1, 120);
    assert_eq!(QWEN3_LOGITS_MAX_EMITTED_TOKENS_V1, 17);
}
