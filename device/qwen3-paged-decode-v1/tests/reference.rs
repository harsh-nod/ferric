use ferric_qwen3_paged_decode_device_v1::{
    QWEN3_PAGED_DECODE_ATTENTION_SCALE_BITS_V1, QWEN3_PAGED_DECODE_CACHE_POOL_PAGES_V1,
    QWEN3_PAGED_DECODE_CONTEXT_CAPACITY_V1, QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1,
    QWEN3_PAGED_DECODE_KV_HEADS_V1, QWEN3_PAGED_DECODE_PAGE_TABLE_ENTRIES_V1,
    QWEN3_PAGED_DECODE_PAGE_TOKENS_V1, QWEN3_PAGED_DECODE_PROFILES_V1, Qwen3PagedDecodeProfileV1,
};

fn widen_bf16(bits: u16) -> f32 {
    f32::from_bits(u32::from(bits) << 16)
}

fn narrow_bf16_rne(value: f32) -> Option<u16> {
    if !value.is_finite() {
        return None;
    }
    let bits = value.to_bits();
    let retained_lsb = (bits >> 16) & 1;
    let narrowed = ((bits + 0x7fff + retained_lsb) >> 16) as u16;
    widen_bf16(narrowed).is_finite().then_some(narrowed)
}

fn cache_index(
    physical_page: usize,
    token_in_page: usize,
    kv_head: usize,
    feature: usize,
) -> usize {
    (((physical_page * QWEN3_PAGED_DECODE_PAGE_TOKENS_V1 + token_in_page)
        * QWEN3_PAGED_DECODE_KV_HEADS_V1
        + kv_head)
        * QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1)
        + feature
}

#[allow(clippy::too_many_arguments)]
fn reference_pair(
    query: &[u16; 128],
    key: &[u16],
    value: &[u16],
    pages: &[u32],
    sequence: usize,
    committed_tokens: usize,
    active_tokens: usize,
    query_token: usize,
    kv_head: usize,
    first_column: usize,
) -> Option<[u16; 2]> {
    if committed_tokens >= QWEN3_PAGED_DECODE_CONTEXT_CAPACITY_V1
        || active_tokens == 0
        || query_token >= active_tokens
        || active_tokens > QWEN3_PAGED_DECODE_CONTEXT_CAPACITY_V1 - committed_tokens
        || kv_head >= QWEN3_PAGED_DECODE_KV_HEADS_V1
        || first_column + 1 >= QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1
    {
        return None;
    }
    let query_position = committed_tokens.checked_add(query_token)?;
    let mut running_max = 0.0_f32;
    let mut running_sum = 0.0_f32;
    let mut numerators = [0.0_f32; 2];
    for key_token in 0..=query_position {
        let logical_page = key_token / QWEN3_PAGED_DECODE_PAGE_TOKENS_V1;
        let token_in_page = key_token % QWEN3_PAGED_DECODE_PAGE_TOKENS_V1;
        if logical_page >= QWEN3_PAGED_DECODE_PAGE_TABLE_ENTRIES_V1 {
            return None;
        }
        let page_index = sequence
            .checked_mul(QWEN3_PAGED_DECODE_PAGE_TABLE_ENTRIES_V1)?
            .checked_add(logical_page)?;
        let physical_page = usize::try_from(*pages.get(page_index)?).ok()?;
        if physical_page >= QWEN3_PAGED_DECODE_CACHE_POOL_PAGES_V1 {
            return None;
        }
        let mut dot = 0.0_f32;
        for (feature, query_bits) in query.iter().copied().enumerate() {
            let q = widen_bf16(query_bits);
            let k = widen_bf16(*key.get(cache_index(
                physical_page,
                token_in_page,
                kv_head,
                feature,
            ))?);
            let product = q * k;
            let next_dot = dot + product;
            if !q.is_finite() || !k.is_finite() || !product.is_finite() || !next_dot.is_finite() {
                return None;
            }
            dot = next_dot;
        }
        let score = dot * f32::from_bits(QWEN3_PAGED_DECODE_ATTENTION_SCALE_BITS_V1);
        let values = [
            widen_bf16(*value.get(cache_index(
                physical_page,
                token_in_page,
                kv_head,
                first_column,
            ))?),
            widen_bf16(*value.get(cache_index(
                physical_page,
                token_in_page,
                kv_head,
                first_column + 1,
            ))?),
        ];
        if !score.is_finite() || !values[0].is_finite() || !values[1].is_finite() {
            return None;
        }
        if key_token == 0 {
            running_max = score;
            running_sum = 1.0;
            numerators = values;
        } else {
            let next_max = score.max(running_max);
            let previous_weight = (running_max - next_max).exp();
            let current_weight = (score - next_max).exp();
            let next_sum = running_sum * previous_weight + current_weight;
            let next_numerator_0 = numerators[0] * previous_weight + values[0] * current_weight;
            let next_numerator_1 = numerators[1] * previous_weight + values[1] * current_weight;
            if !previous_weight.is_finite()
                || !current_weight.is_finite()
                || !next_sum.is_finite()
                || next_sum <= 0.0
                || !next_numerator_0.is_finite()
                || !next_numerator_1.is_finite()
            {
                return None;
            }
            running_max = next_max;
            running_sum = next_sum;
            numerators = [next_numerator_0, next_numerator_1];
        }
    }
    Some([
        narrow_bf16_rne(numerators[0] / running_sum)?,
        narrow_bf16_rne(numerators[1] / running_sum)?,
    ])
}

fn compact_cache(pages: usize) -> Vec<u16> {
    vec![0; pages * 16 * 8 * 128]
}

fn reference_coordinates(profile: Qwen3PagedDecodeProfileV1, global: usize) -> Option<[usize; 5]> {
    if profile.query_heads == 0 || profile.active_tokens == 0 || profile.gqa_group_size == 0 {
        return None;
    }
    let vector = global / 64;
    let local = global % 64;
    let query_head = vector % profile.query_heads;
    let position = vector / profile.query_heads;
    let query_token = position % profile.active_tokens;
    let sequence = position / profile.active_tokens;
    let kv_head = query_head / profile.gqa_group_size;
    Some([sequence, query_token, query_head, kv_head, local])
}

#[test]
fn closed_profiles_make_divisor_guards_unreachable_and_coordinates_round_trip() {
    for profile in QWEN3_PAGED_DECODE_PROFILES_V1 {
        let workitems = profile.query_elements / 2;
        for global in [0, workitems - 1] {
            let [sequence, query_token, query_head, kv_head, local] =
                reference_coordinates(profile, global)
                    .expect("closed profiles have nonzero divisors");
            assert!(sequence < profile.sequences);
            assert!(query_token < profile.active_tokens);
            assert!(query_head < profile.query_heads);
            assert!(kv_head < QWEN3_PAGED_DECODE_KV_HEADS_V1);
            assert!(local < 64);

            let rebuilt_vector =
                (sequence * profile.active_tokens + query_token) * profile.query_heads + query_head;
            assert_eq!(rebuilt_vector * 64 + local, global);
            assert_eq!(kv_head, query_head / profile.gqa_group_size);
        }
    }
}

#[test]
fn zero_coordinate_divisors_fail_before_division() {
    let profile = QWEN3_PAGED_DECODE_PROFILES_V1[0];
    for hostile in [
        Qwen3PagedDecodeProfileV1 {
            query_heads: 0,
            ..profile
        },
        Qwen3PagedDecodeProfileV1 {
            active_tokens: 0,
            ..profile
        },
        Qwen3PagedDecodeProfileV1 {
            gqa_group_size: 0,
            ..profile
        },
    ] {
        assert_eq!(reference_coordinates(hostile, 0), None);
    }
}

#[test]
fn zero_scores_preserve_constant_values_across_committed_and_active_tokens() {
    let query = [0_u16; 128];
    let key = compact_cache(2);
    let mut value = compact_cache(2);
    value.fill(0x3f00); // 0.5
    let mut pages = [0_u32; 512];
    pages[1] = 1;
    for (committed, active, query_token) in [(0, 1, 0), (13, 4, 3), (16, 1, 0)] {
        assert_eq!(
            reference_pair(
                &query,
                &key,
                &value,
                &pages,
                0,
                committed,
                active,
                query_token,
                7,
                126,
            ),
            Some([0x3f00, 0x3f00])
        );
    }
}

#[test]
fn page_table_selects_global_physical_pages_without_a_sequence_cache_stride() {
    let query = [0_u16; 128];
    let key = compact_cache(2);
    let mut value = compact_cache(2);
    let mapped = cache_index(1, 0, 3, 10);
    value[mapped] = 0x3f80;
    value[mapped + 1] = 0xc000;
    let mut pages = vec![0_u32; 2 * QWEN3_PAGED_DECODE_PAGE_TABLE_ENTRIES_V1];
    pages[QWEN3_PAGED_DECODE_PAGE_TABLE_ENTRIES_V1] = 1;
    assert_eq!(
        reference_pair(&query, &key, &value, &pages, 1, 0, 1, 0, 3, 10),
        Some([0x3f80, 0xc000])
    );
}

#[test]
fn quotient_gqa_maps_target_and_draft_query_heads_to_shared_kv_heads() {
    for query_head in 0..32 {
        assert_eq!(query_head / 4, (query_head >> 2).min(7));
    }
    for query_head in 0..16 {
        assert_eq!(query_head / 2, (query_head >> 1).min(7));
    }
    assert_eq!(31 / 4, 7);
    assert_eq!(15 / 2, 7);
}

#[test]
fn online_recurrence_agrees_with_an_independent_batch_softmax() {
    let mut query = [0_u16; 128];
    query[0] = 0x3f80;
    let mut key = compact_cache(1);
    let mut value = compact_cache(1);
    let key_bits = [0xbf80, 0x0000, 0x4000];
    let value_bits = [[0x3f80, 0xc000], [0x4040, 0x4080], [0xc0a0, 0x40c0]];
    for token in 0..3 {
        key[cache_index(0, token, 0, 0)] = key_bits[token];
        value[cache_index(0, token, 0, 20)] = value_bits[token][0];
        value[cache_index(0, token, 0, 21)] = value_bits[token][1];
    }
    let pages = [0_u32; 512];
    let observed = reference_pair(&query, &key, &value, &pages, 0, 2, 1, 0, 0, 20).unwrap();

    let scale = f32::from_bits(QWEN3_PAGED_DECODE_ATTENTION_SCALE_BITS_V1);
    let scores = [-scale, 0.0, 2.0 * scale];
    let maximum = scores.into_iter().reduce(f32::max).unwrap();
    let weights = scores.map(|score| (score - maximum).exp());
    let denominator: f32 = weights.into_iter().sum();
    let expected = [
        narrow_bf16_rne((1.0 * weights[0] + 3.0 * weights[1] - 5.0 * weights[2]) / denominator)
            .unwrap(),
        narrow_bf16_rne((-2.0 * weights[0] + 4.0 * weights[1] + 6.0 * weights[2]) / denominator)
            .unwrap(),
    ];
    assert_eq!(observed, expected);
}

#[test]
fn future_key_and_value_mutations_do_not_affect_a_committed_causal_pair() {
    let query = [0x3f80_u16; 128];
    let mut key = compact_cache(1);
    let mut value = compact_cache(1);
    for token in 0..16 {
        for feature in 0..128 {
            key[cache_index(0, token, 0, feature)] = if token <= 2 { 0x3c00 } else { 0x42c8 };
            value[cache_index(0, token, 0, feature)] = if token <= 2 { 0x3f00 } else { 0xc2c8 };
        }
    }
    let pages = [0_u32; 512];
    let baseline = reference_pair(&query, &key, &value, &pages, 0, 2, 1, 0, 0, 0);
    for token in 3..16 {
        for feature in 0..128 {
            key[cache_index(0, token, 0, feature)] = 0x7f7f;
            value[cache_index(0, token, 0, feature)] = 0x7f7f;
        }
    }
    assert_eq!(
        reference_pair(&query, &key, &value, &pages, 0, 2, 1, 0, 0, 0),
        baseline
    );
}

#[test]
fn reference_rejects_invalid_context_page_and_nonfinite_inputs() {
    let mut query = [0_u16; 128];
    let mut key = compact_cache(1);
    let mut value = compact_cache(1);
    let mut pages = [0_u32; 512];
    assert_eq!(
        reference_pair(&query, &key, &value, &pages, 0, 8_191, 2, 0, 0, 0),
        None
    );
    pages[0] = 16_384;
    assert_eq!(
        reference_pair(&query, &key, &value, &pages, 0, 0, 1, 0, 0, 0),
        None
    );
    pages[0] = 0;
    query[17] = 0x7f80;
    assert_eq!(
        reference_pair(&query, &key, &value, &pages, 0, 0, 1, 0, 0, 0),
        None
    );
    query[17] = 0;
    key[17] = 0x7fc1;
    assert_eq!(
        reference_pair(&query, &key, &value, &pages, 0, 0, 1, 0, 0, 0),
        None
    );
    key[17] = 0;
    value[0] = 0xff80;
    assert_eq!(
        reference_pair(&query, &key, &value, &pages, 0, 0, 1, 0, 0, 0),
        None
    );
}

#[test]
fn bf16_output_rounding_is_rne() {
    assert_eq!(narrow_bf16_rne(f32::from_bits(0x3f80_7fff)), Some(0x3f80));
    assert_eq!(narrow_bf16_rne(f32::from_bits(0x3f80_8000)), Some(0x3f80));
    assert_eq!(narrow_bf16_rne(f32::from_bits(0x3f81_8000)), Some(0x3f82));
    assert_eq!(narrow_bf16_rne(f32::from_bits(0x3f80_8001)), Some(0x3f81));
}
