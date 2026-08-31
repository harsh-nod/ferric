use ferric_qwen3_rope_kv_device_v1::{
    QWEN3_KV_CACHE_ELEMENTS_V1, QWEN3_KV_PAGE_TABLE_ENTRIES_V1, QWEN3_KV_PAGE_TOKENS_V1,
    QWEN3_KV_PHYSICAL_PAGE_SLOTS_V1, QWEN3_ROPE_KV_MAX_GRID_WORKGROUPS_V1,
    qwen3_paged_kv_write_profile_is_admitted_v1, qwen3_rope_profile_is_admitted_v1,
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
        assert!(active * sequences <= QWEN3_ROPE_KV_MAX_GRID_WORKGROUPS_V1);
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
        rope_machine_profiles += 1;
    }
    assert_eq!(rope_machine_profiles, 22);

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
