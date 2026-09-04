use ferric_qwen3_rope_kv_device_v1::{
    QWEN3_KV_CACHE_ELEMENTS_V1, QWEN3_KV_PAGE_TABLE_ENTRIES_V1, QWEN3_KV_PAGE_TOKENS_V1,
    QWEN3_KV_PHYSICAL_PAGE_SLOTS_V1, QWEN3_PAGED_KV_WRITE_GRID_WORKGROUPS_V1,
    QWEN3_PAGED_KV_WRITE_GRID_WORKITEMS_V1, QWEN3_ROPE_KV_MAX_PROFILE_ROWS_V1,
    QWEN3_ROPE_MAX_GRID_WORKGROUPS_V1, qwen3_paged_kv_write_profile_is_admitted_v1,
    qwen3_rope_profile_is_admitted_v1,
};

fn widen_bf16(bits: u16) -> f32 {
    f32::from_bits(u32::from(bits) << 16)
}

fn narrow_bf16_rne(value: f32) -> u16 {
    let bits = value.to_bits();
    if bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0 {
        return ((bits >> 16) as u16) | 0x0040;
    }
    let retained_lsb = (bits >> 16) & 1;
    ((bits + 0x7fff + retained_lsb) >> 16) as u16
}

fn reference_rope_pair(first: u16, second: u16, cos: f32, sin: f32) -> [u16; 2] {
    let first = widen_bf16(first);
    let second = widen_bf16(second);
    let first_cos = first * cos;
    let second_sin = second * sin;
    let rotated_first = first_cos - second_sin;
    let second_cos = second * cos;
    let first_sin = first * sin;
    let rotated_second = second_cos + first_sin;
    [
        narrow_bf16_rne(rotated_first),
        narrow_bf16_rne(rotated_second),
    ]
}

fn cache_index(physical_page: usize, token_in_page: usize, component: usize) -> usize {
    ((physical_page * QWEN3_KV_PAGE_TOKENS_V1 as usize + token_in_page) * 8 * 128) + component
}

fn blocked_cache_index(physical_page: usize, lane: usize, blocked_component: usize) -> usize {
    physical_page * 64 * 256 + blocked_component * 64 + lane
}

fn checked_kv_destination(
    logical_start: usize,
    local_token: usize,
    context_tokens: usize,
    physical_page: usize,
    component: usize,
) -> Option<usize> {
    if logical_start >= 8_192 || physical_page >= QWEN3_KV_PHYSICAL_PAGE_SLOTS_V1 as usize {
        return None;
    }
    let logical_position = logical_start.checked_add(local_token)?;
    if logical_position >= context_tokens || logical_position >= 8_192 || component >= 1_024 {
        return None;
    }
    Some(cache_index(
        physical_page,
        logical_position % QWEN3_KV_PAGE_TOKENS_V1 as usize,
        component,
    ))
}

#[test]
fn exact_profile_rosters_are_finite_and_fit_the_flat_grid_maximum() {
    const COMMON: [[u32; 3]; 7] = [
        [128, 1, 128],
        [128, 8, 128],
        [512, 1, 512],
        [2_048, 1, 2_048],
        [1, 1, 8_192],
        [1, 8, 8_192],
        [1, 32, 8_192],
    ];
    const TARGET_SPECULATIVE: [[u32; 3]; 4] =
        [[5, 1, 8_192], [5, 8, 8_192], [9, 1, 8_192], [17, 1, 8_192]];
    const DRAFT_SPECULATIVE: [[u32; 3]; 4] =
        [[4, 1, 8_192], [4, 8, 8_192], [8, 1, 8_192], [16, 1, 8_192]];

    let mut rope_machine_profiles = 0;
    for [active, sequences, context] in COMMON {
        for query_heads in [16, 32] {
            assert!(qwen3_rope_profile_is_admitted_v1(
                active,
                sequences,
                query_heads,
                context
            ));
            rope_machine_profiles += 1;
        }
        assert!(qwen3_paged_kv_write_profile_is_admitted_v1(
            active, sequences, context
        ));
        assert!(active * sequences <= QWEN3_ROPE_MAX_GRID_WORKGROUPS_V1);
    }
    for [active, sequences, context] in TARGET_SPECULATIVE {
        assert!(qwen3_rope_profile_is_admitted_v1(
            active, sequences, 32, context
        ));
        assert!(!qwen3_rope_profile_is_admitted_v1(
            active, sequences, 16, context
        ));
        assert!(qwen3_paged_kv_write_profile_is_admitted_v1(
            active, sequences, context
        ));
        assert!(active * sequences <= QWEN3_ROPE_KV_MAX_PROFILE_ROWS_V1);
        rope_machine_profiles += 1;
    }
    for [active, sequences, context] in DRAFT_SPECULATIVE {
        assert!(qwen3_rope_profile_is_admitted_v1(
            active, sequences, 16, context
        ));
        assert!(!qwen3_rope_profile_is_admitted_v1(
            active, sequences, 32, context
        ));
        assert!(qwen3_paged_kv_write_profile_is_admitted_v1(
            active, sequences, context
        ));
        assert!(active * sequences <= QWEN3_ROPE_KV_MAX_PROFILE_ROWS_V1);
        rope_machine_profiles += 1;
    }
    assert_eq!(rope_machine_profiles, 22);
    assert_eq!(QWEN3_ROPE_KV_MAX_PROFILE_ROWS_V1, 2_048);
    assert_eq!(
        QWEN3_ROPE_MAX_GRID_WORKGROUPS_V1,
        QWEN3_ROPE_KV_MAX_PROFILE_ROWS_V1
    );
    assert_eq!(QWEN3_PAGED_KV_WRITE_GRID_WORKGROUPS_V1, 16_384);
    assert_eq!(QWEN3_PAGED_KV_WRITE_GRID_WORKITEMS_V1, 1_048_576);

    for [active, sequences, query_heads, context] in [
        [0, 1, 32, 128],
        [127, 1, 32, 128],
        [128, 2, 32, 128],
        [128, 1, 8, 128],
        [17, 8, 32, 8_192],
        [17, 1, 32, 8_191],
        [2_048, 1, 32, 8_192],
    ] {
        assert!(!qwen3_rope_profile_is_admitted_v1(
            active,
            sequences,
            query_heads,
            context
        ));
    }
    for [active, sequences, context] in [
        [0, 1, 128],
        [127, 1, 128],
        [128, 2, 128],
        [17, 8, 8_192],
        [17, 1, 8_191],
        [2_048, 1, 8_192],
    ] {
        assert!(!qwen3_paged_kv_write_profile_is_admitted_v1(
            active, sequences, context
        ));
    }
}

#[test]
fn split_half_rope_uses_ordered_fp32_operations_and_bf16_rne_storage() {
    assert_eq!(
        reference_rope_pair(0x3f80, 0x4000, 1.0, 0.0),
        [0x3f80, 0x4000]
    );
    assert_eq!(
        reference_rope_pair(0x3f80, 0x4000, 0.0, 1.0),
        [0xc000, 0x3f80]
    );
    assert_eq!(
        reference_rope_pair(0x3f80, 0x4000, 0.5, 0.25),
        [0x0000, 0x3fa0]
    );
    assert_eq!(narrow_bf16_rne(f32::from_bits(0x3f80_8000)), 0x3f80);
    assert_eq!(narrow_bf16_rne(f32::from_bits(0x3f81_8000)), 0x3f82);
}

#[test]
fn paged_kv_mapping_crosses_page_boundaries_and_stays_in_the_global_pool() {
    let pages = [7_usize, 3, 16_383];
    let first = cache_index(pages[0], 15, 0);
    let second = cache_index(pages[1], 0, 0);
    assert_eq!(first, (7 * 16 + 15) * 1_024);
    assert_eq!(second, 3 * 16 * 1_024);
    assert_ne!(first, second);

    let final_index = cache_index(
        QWEN3_KV_PHYSICAL_PAGE_SLOTS_V1 as usize - 1,
        QWEN3_KV_PAGE_TOKENS_V1 as usize - 1,
        1_023,
    );
    assert_eq!(final_index, QWEN3_KV_CACHE_ELEMENTS_V1 - 1);
    assert_eq!(
        QWEN3_KV_PAGE_TABLE_ENTRIES_V1 * QWEN3_KV_PAGE_TOKENS_V1,
        8_192
    );
}

#[test]
fn hostile_logical_physical_and_component_boundaries_fail_closed() {
    assert_eq!(checked_kv_destination(0, 0, 8_192, 0, 0), Some(0));
    assert_eq!(
        checked_kv_destination(8_191, 0, 8_192, 16_383, 1_023),
        Some(QWEN3_KV_CACHE_ELEMENTS_V1 - 1)
    );
    for rejected in [
        checked_kv_destination(8_192, 0, 8_192, 0, 0),
        checked_kv_destination(8_191, 1, 8_192, 0, 0),
        checked_kv_destination(127, 1, 128, 0, 0),
        checked_kv_destination(0, 0, 8_192, 16_384, 0),
        checked_kv_destination(0, 0, 8_192, 0, 1_024),
        checked_kv_destination(usize::MAX, 1, 8_192, 0, 0),
    ] {
        assert_eq!(rejected, None);
    }
}

#[test]
fn reverse_page_block_mapping_is_identical_and_injective() {
    assert_eq!(
        QWEN3_KV_PHYSICAL_PAGE_SLOTS_V1 as usize * 256 * 64,
        QWEN3_KV_CACHE_ELEMENTS_V1
    );
    for physical_page in 0..QWEN3_KV_PHYSICAL_PAGE_SLOTS_V1 as usize {
        for blocked_component in 0..256 {
            for lane in [0_usize, 1, 63] {
                let index = blocked_cache_index(physical_page, lane, blocked_component);
                assert!(index < QWEN3_KV_CACHE_ELEMENTS_V1);
                assert_eq!(index / (256 * 64), physical_page);
                let page_offset = index % (256 * 64);
                assert_eq!(page_offset / 64, blocked_component);
                assert_eq!(page_offset % 64, lane);

                let token_in_page = blocked_component / 16;
                let input_component = blocked_component % 16;
                assert_eq!(
                    index,
                    cache_index(physical_page, token_in_page, input_component * 64 + lane)
                );
            }
        }
    }
    assert_eq!(
        blocked_cache_index(16_383, 63, 255),
        QWEN3_KV_CACHE_ELEMENTS_V1 - 1
    );
}

#[test]
fn aliasing_rows_share_one_page_owner_and_preserve_untouched_seed_bits() {
    let first_key = (0..1_024)
        .map(|index| (index as u16).wrapping_mul(17) ^ 0x7fc1)
        .collect::<Vec<_>>();
    let second_key = (0..1_024)
        .map(|index| (index as u16).wrapping_mul(29) ^ 0x8001)
        .collect::<Vec<_>>();
    let first_value = (0..1_024)
        .map(|index| (index as u16).wrapping_mul(31) ^ 0x3f80)
        .collect::<Vec<_>>();
    let second_value = (0..1_024)
        .map(|index| (index as u16).wrapping_mul(43) ^ 0xbf80)
        .collect::<Vec<_>>();
    let other_page = vec![0x1234_u16; 1_024];
    let rows = [
        (7_usize, 3_usize, &first_key, &first_value),
        (8, 3, &other_page, &other_page),
        (7, 3, &second_key, &second_value),
    ];
    let mut key_page = vec![0xa5a5_u16; 16 * 1_024];
    let mut value_page = vec![0x5a5a_u16; 16 * 1_024];

    for (physical_page, token_in_page, key, value) in rows {
        if physical_page != 7 {
            continue;
        }
        for input_component in 0..16 {
            let blocked_component = token_in_page * 16 + input_component;
            for lane in 0..64 {
                let source = input_component * 64 + lane;
                let destination = blocked_component * 64 + lane;
                assert_eq!(7 * 64 + lane, physical_page * 64 + lane);
                key_page[destination] = key[source];
                value_page[destination] = value[source];
            }
        }
    }

    assert_eq!(&key_page[3 * 1_024..4 * 1_024], second_key.as_slice());
    assert_eq!(&value_page[3 * 1_024..4 * 1_024], second_value.as_slice());
    assert!(key_page[..3 * 1_024].iter().all(|value| *value == 0xa5a5));
    assert!(key_page[4 * 1_024..].iter().all(|value| *value == 0xa5a5));
    assert!(value_page[..3 * 1_024].iter().all(|value| *value == 0x5a5a));
    assert!(value_page[4 * 1_024..].iter().all(|value| *value == 0x5a5a));
}
