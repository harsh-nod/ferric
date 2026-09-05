use ferric_qwen3_paged_decode_device_v1::{
    QWEN3_PAGED_DECODE_CACHE_ELEMENTS_V1, QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1,
    QWEN3_PAGED_DECODE_KV_HEADS_V1, QWEN3_PAGED_DECODE_MAX_GRID_WORKGROUPS_V1,
    QWEN3_PAGED_DECODE_PAGE_TABLE_ENTRIES_V1, QWEN3_PAGED_DECODE_PROFILE_COUNT_V1,
    QWEN3_PAGED_DECODE_PROFILES_V1, qwen3_paged_decode_profile_for_lengths_v1,
};

#[test]
fn exact_fourteen_profile_catalog_closes_shape_gqa_and_launch_arithmetic() {
    assert_eq!(
        QWEN3_PAGED_DECODE_PROFILES_V1.len(),
        QWEN3_PAGED_DECODE_PROFILE_COUNT_V1
    );
    for (index, profile) in QWEN3_PAGED_DECODE_PROFILES_V1.iter().copied().enumerate() {
        assert_ne!(profile.query_heads, 0);
        assert_ne!(profile.active_tokens, 0);
        assert_ne!(profile.gqa_group_size, 0);
        assert_eq!(
            profile.query_elements,
            profile.sequences
                * profile.active_tokens
                * profile.query_heads
                * QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1
        );
        assert_eq!(
            profile.page_table_elements,
            profile.sequences * QWEN3_PAGED_DECODE_PAGE_TABLE_ENTRIES_V1
        );
        assert_eq!(profile.committed_elements, profile.sequences);
        assert_eq!(
            profile.query_heads / profile.gqa_group_size,
            QWEN3_PAGED_DECODE_KV_HEADS_V1
        );
        let workitems = profile.query_elements / 2;
        assert_eq!(workitems % 64, 0);
        assert!(workitems / 64 <= QWEN3_PAGED_DECODE_MAX_GRID_WORKGROUPS_V1 as usize);
        assert_eq!(
            qwen3_paged_decode_profile_for_lengths_v1(
                profile.query_elements,
                profile.page_table_elements,
                profile.committed_elements,
            ),
            Some(profile)
        );
        assert!(
            QWEN3_PAGED_DECODE_PROFILES_V1[index + 1..]
                .iter()
                .all(|other| other != &profile)
        );
    }
    assert_eq!(QWEN3_PAGED_DECODE_CACHE_ELEMENTS_V1, 16_384 * 16 * 8 * 128);
}

#[test]
fn catalog_order_and_speculative_widths_match_the_host_contract() {
    let observed = QWEN3_PAGED_DECODE_PROFILES_V1.map(|profile| {
        (
            profile.sequences,
            profile.active_tokens,
            profile.query_heads,
            profile.gqa_group_size,
            profile.query_elements,
            profile.page_table_elements,
            profile.committed_elements,
        )
    });
    assert_eq!(
        observed,
        [
            (1, 1, 32, 4, 4_096, 512, 1),
            (8, 1, 32, 4, 32_768, 4_096, 8),
            (32, 1, 32, 4, 131_072, 16_384, 32),
            (1, 5, 32, 4, 20_480, 512, 1),
            (8, 5, 32, 4, 163_840, 4_096, 8),
            (1, 9, 32, 4, 36_864, 512, 1),
            (1, 17, 32, 4, 69_632, 512, 1),
            (1, 1, 16, 2, 2_048, 512, 1),
            (8, 1, 16, 2, 16_384, 4_096, 8),
            (32, 1, 16, 2, 65_536, 16_384, 32),
            (1, 4, 16, 2, 8_192, 512, 1),
            (8, 4, 16, 2, 65_536, 4_096, 8),
            (1, 8, 16, 2, 16_384, 512, 1),
            (1, 16, 16, 2, 32_768, 512, 1),
        ]
    );
}

#[test]
fn page_and_committed_lengths_disambiguate_shared_query_extents() {
    let target_s8_decode = qwen3_paged_decode_profile_for_lengths_v1(32_768, 4_096, 8).unwrap();
    let draft_s1k16 = qwen3_paged_decode_profile_for_lengths_v1(32_768, 512, 1).unwrap();
    assert_eq!(
        (
            target_s8_decode.sequences,
            target_s8_decode.active_tokens,
            target_s8_decode.query_heads,
        ),
        (8, 1, 32)
    );
    assert_eq!(
        (
            draft_s1k16.sequences,
            draft_s1k16.active_tokens,
            draft_s1k16.query_heads,
        ),
        (1, 16, 16)
    );

    let draft_s32_decode = qwen3_paged_decode_profile_for_lengths_v1(65_536, 16_384, 32).unwrap();
    let draft_s8k4 = qwen3_paged_decode_profile_for_lengths_v1(65_536, 4_096, 8).unwrap();
    assert_eq!(
        (draft_s32_decode.sequences, draft_s32_decode.active_tokens),
        (32, 1)
    );
    assert_eq!((draft_s8k4.sequences, draft_s8k4.active_tokens), (8, 4));
}

#[test]
fn profile_admission_rejects_near_misses_and_crossed_axes() {
    for (query, pages, committed) in [
        (0, 0, 0),
        (4_095, 512, 1),
        (4_097, 512, 1),
        (4_096, 4_096, 1),
        (32_768, 512, 8),
        (65_536, 16_384, 8),
        (163_840, 4_096, 1),
        (usize::MAX, usize::MAX, usize::MAX),
    ] {
        assert_eq!(
            qwen3_paged_decode_profile_for_lengths_v1(query, pages, committed),
            None
        );
    }
}
