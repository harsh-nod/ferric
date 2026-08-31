use ferric_qwen3_prefill_device_v1::{
    QWEN3_PREFILL_ATTENTION_SCALE_BITS_V1, QWEN3_PREFILL_CACHE_ELEMENTS_V1,
    QWEN3_PREFILL_CACHE_POOL_PAGES_V1, QWEN3_PREFILL_EXPLICIT_KERNARG_BYTES_V1,
    QWEN3_PREFILL_HEAD_DIMENSION_V1, QWEN3_PREFILL_KERNEL_SYMBOL_V1, QWEN3_PREFILL_KV_HEADS_V1,
    QWEN3_PREFILL_MAX_GRID_WORKGROUPS_V1, QWEN3_PREFILL_PAGE_TABLE_ENTRIES_V1,
    QWEN3_PREFILL_PAGE_TOKENS_V1, QWEN3_PREFILL_PROFILE_COUNT_V1, QWEN3_PREFILL_PROFILES_V1,
    QWEN3_PREFILL_WORKGROUP_V1, qwen3_prefill_profile_for_lengths_v1,
};

#[test]
fn constants_retain_the_authoritative_host_symbol_abi_and_cache_geometry() {
    assert_eq!(
        QWEN3_PREFILL_KERNEL_SYMBOL_V1,
        "qwen3_gqa_prefill_causal_bf16_f32_v1"
    );
    assert_eq!(QWEN3_PREFILL_WORKGROUP_V1, [64, 1, 1]);
    assert_eq!(QWEN3_PREFILL_MAX_GRID_WORKGROUPS_V1, 65_536);
    assert_eq!(QWEN3_PREFILL_HEAD_DIMENSION_V1, 128);
    assert_eq!(QWEN3_PREFILL_KV_HEADS_V1, 8);
    assert_eq!(QWEN3_PREFILL_PAGE_TOKENS_V1, 16);
    assert_eq!(QWEN3_PREFILL_PAGE_TABLE_ENTRIES_V1, 512);
    assert_eq!(QWEN3_PREFILL_CACHE_POOL_PAGES_V1, 16_384);
    assert_eq!(QWEN3_PREFILL_ATTENTION_SCALE_BITS_V1, 0x3db5_04f3);
    assert_eq!(QWEN3_PREFILL_EXPLICIT_KERNARG_BYTES_V1, 80);
}

#[test]
fn exact_eight_profile_catalog_closes_shape_and_launch_arithmetic() {
    assert_eq!(
        QWEN3_PREFILL_PROFILES_V1.len(),
        QWEN3_PREFILL_PROFILE_COUNT_V1
    );
    for profile in QWEN3_PREFILL_PROFILES_V1 {
        assert_eq!(
            profile.query_elements,
            profile.sequences
                * profile.tokens
                * profile.query_heads
                * QWEN3_PREFILL_HEAD_DIMENSION_V1
        );
        assert_eq!(
            profile.page_table_elements,
            profile.sequences * QWEN3_PREFILL_PAGE_TABLE_ENTRIES_V1
        );
        assert_eq!(
            profile.query_heads / profile.gqa_group_size,
            QWEN3_PREFILL_KV_HEADS_V1
        );
        let workitems = profile.query_elements / 2;
        assert_eq!(workitems % 64, 0);
        assert!(workitems / 64 <= QWEN3_PREFILL_MAX_GRID_WORKGROUPS_V1 as usize);
        assert_eq!(
            qwen3_prefill_profile_for_lengths_v1(
                profile.query_elements,
                profile.page_table_elements
            ),
            Some(profile)
        );
    }
    assert_eq!(QWEN3_PREFILL_CACHE_ELEMENTS_V1, 16_384 * 16 * 8 * 128);
}

#[test]
fn page_length_disambiguates_the_two_shared_query_extents() {
    let target_s1t512 = qwen3_prefill_profile_for_lengths_v1(2_097_152, 512).unwrap();
    let draft_s8t128 = qwen3_prefill_profile_for_lengths_v1(2_097_152, 4_096).unwrap();
    assert_eq!(
        (
            target_s1t512.sequences,
            target_s1t512.tokens,
            target_s1t512.query_heads
        ),
        (1, 512, 32)
    );
    assert_eq!(
        (
            draft_s8t128.sequences,
            draft_s8t128.tokens,
            draft_s8t128.query_heads
        ),
        (8, 128, 16)
    );

    let target_s8t128 = qwen3_prefill_profile_for_lengths_v1(4_194_304, 4_096).unwrap();
    let draft_s1t2048 = qwen3_prefill_profile_for_lengths_v1(4_194_304, 512).unwrap();
    assert_eq!(
        (
            target_s8t128.sequences,
            target_s8t128.tokens,
            target_s8t128.query_heads
        ),
        (8, 128, 32)
    );
    assert_eq!(
        (
            draft_s1t2048.sequences,
            draft_s1t2048.tokens,
            draft_s1t2048.query_heads
        ),
        (1, 2_048, 16)
    );
}

#[test]
fn profile_admission_rejects_near_misses_and_crossed_axes() {
    for (query, pages) in [
        (0, 0),
        (524_287, 512),
        (524_289, 512),
        (524_288, 4_096),
        (2_097_152, 511),
        (2_097_152, 513),
        (8_388_608, 4_096),
        (usize::MAX, usize::MAX),
    ] {
        assert_eq!(qwen3_prefill_profile_for_lengths_v1(query, pages), None);
    }
}
