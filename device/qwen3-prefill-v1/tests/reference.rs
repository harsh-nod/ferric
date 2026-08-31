use ferric_qwen3_prefill_device_v1::{
    QWEN3_PREFILL_ATTENTION_SCALE_BITS_V1, QWEN3_PREFILL_CACHE_POOL_PAGES_V1,
    QWEN3_PREFILL_HEAD_DIMENSION_V1, QWEN3_PREFILL_KV_HEADS_V1,
    QWEN3_PREFILL_PAGE_TABLE_ENTRIES_V1, QWEN3_PREFILL_PAGE_TOKENS_V1,
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
    (((physical_page * QWEN3_PREFILL_PAGE_TOKENS_V1 + token_in_page) * QWEN3_PREFILL_KV_HEADS_V1
        + kv_head)
        * QWEN3_PREFILL_HEAD_DIMENSION_V1)
        + feature
}

fn reference_pair(
    query: &[u16; 128],
    key: &[u16],
    value: &[u16],
    pages: &[u32],
    sequence: usize,
    query_token: usize,
    kv_head: usize,
    first_column: usize,
) -> Option<[u16; 2]> {
    if kv_head >= QWEN3_PREFILL_KV_HEADS_V1
        || first_column.checked_add(1)? >= QWEN3_PREFILL_HEAD_DIMENSION_V1
    {
        return None;
    }
    let mut running_max = 0.0_f32;
    let mut running_sum = 0.0_f32;
    let mut numerators = [0.0_f32; 2];
    for key_token in 0..=query_token {
        let logical_page = key_token / QWEN3_PREFILL_PAGE_TOKENS_V1;
        if logical_page >= QWEN3_PREFILL_PAGE_TABLE_ENTRIES_V1 {
            return None;
        }
        let token_in_page = key_token % QWEN3_PREFILL_PAGE_TOKENS_V1;
        let page_index = sequence * QWEN3_PREFILL_PAGE_TABLE_ENTRIES_V1 + logical_page;
        let physical_page = usize::try_from(*pages.get(page_index)?).ok()?;
        if physical_page >= QWEN3_PREFILL_CACHE_POOL_PAGES_V1 {
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
            dot += product;
            if !q.is_finite() || !k.is_finite() || !product.is_finite() || !dot.is_finite() {
                return None;
            }
        }
        let score = dot * f32::from_bits(QWEN3_PREFILL_ATTENTION_SCALE_BITS_V1);
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
            running_sum = running_sum * previous_weight + current_weight;
            numerators[0] = numerators[0] * previous_weight + values[0] * current_weight;
            numerators[1] = numerators[1] * previous_weight + values[1] * current_weight;
            running_max = next_max;
            if !previous_weight.is_finite()
                || !current_weight.is_finite()
                || !running_sum.is_finite()
                || running_sum <= 0.0
                || !numerators[0].is_finite()
                || !numerators[1].is_finite()
            {
                return None;
            }
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

#[test]
fn zero_scores_preserve_constant_values_across_the_causal_prefix() {
    let query = [0_u16; 128];
    let key = compact_cache(1);
    let mut value = compact_cache(1);
    value.fill(0x3f00); // 0.5
    let pages = [0_u32];
    for query_token in [0, 1, 7, 15] {
        assert_eq!(
            reference_pair(&query, &key, &value, &pages, 0, query_token, 7, 126),
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
    let mut pages = vec![0_u32; 2 * QWEN3_PREFILL_PAGE_TABLE_ENTRIES_V1];
    pages[QWEN3_PREFILL_PAGE_TABLE_ENTRIES_V1] = 1;
    assert_eq!(
        reference_pair(&query, &key, &value, &pages, 1, 0, 3, 10),
        Some([0x3f80, 0xc000])
    );
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
    let pages = [0_u32];
    let observed = reference_pair(&query, &key, &value, &pages, 0, 2, 0, 20).unwrap();

    let scale = f32::from_bits(QWEN3_PREFILL_ATTENTION_SCALE_BITS_V1);
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
fn future_key_and_value_mutations_do_not_affect_a_causal_pair() {
    let query = [0x3f80_u16; 128];
    let mut key = compact_cache(1);
    let mut value = compact_cache(1);
    for token in 0..16 {
        for feature in 0..128 {
            key[cache_index(0, token, 0, feature)] = if token <= 2 { 0x3c00 } else { 0x42c8 };
            value[cache_index(0, token, 0, feature)] = if token <= 2 { 0x3f00 } else { 0xc2c8 };
        }
    }
    let pages = [0_u32];
    let baseline = reference_pair(&query, &key, &value, &pages, 0, 2, 0, 0);
    for token in 3..16 {
        for feature in 0..128 {
            key[cache_index(0, token, 0, feature)] = 0x7f7f;
            value[cache_index(0, token, 0, feature)] = 0x7f7f;
        }
    }
    assert_eq!(
        reference_pair(&query, &key, &value, &pages, 0, 2, 0, 0),
        baseline
    );
}

#[test]
fn reference_rejects_nonfinite_query_key_value_and_result_paths() {
    let mut query = [0_u16; 128];
    let mut key = compact_cache(1);
    let mut value = compact_cache(1);
    let pages = [0_u32];
    query[17] = 0x7f80;
    assert_eq!(
        reference_pair(&query, &key, &value, &pages, 0, 0, 0, 0),
        None
    );
    query[17] = 0;
    key[17] = 0x7fc1;
    assert_eq!(
        reference_pair(&query, &key, &value, &pages, 0, 0, 0, 0),
        None
    );
    key[17] = 0;
    value[0] = 0xff80;
    assert_eq!(
        reference_pair(&query, &key, &value, &pages, 0, 0, 0, 0),
        None
    );

    value[0] = 0;
    query.fill(0x7f7f);
    key[..128].fill(0x7f7f);
    assert_eq!(
        reference_pair(&query, &key, &value, &pages, 0, 0, 0, 0),
        None
    );
}

#[test]
fn reference_rejects_unmapped_or_out_of_range_cache_access() {
    let query = [0_u16; 128];
    let key = compact_cache(1);
    let value = compact_cache(1);

    assert_eq!(reference_pair(&query, &key, &value, &[], 0, 0, 0, 0), None);
    assert_eq!(reference_pair(&query, &key, &value, &[1], 0, 0, 0, 0), None);
    assert_eq!(reference_pair(&query, &key, &value, &[0], 0, 0, 8, 0), None);
    assert_eq!(
        reference_pair(&query, &key, &value, &[0], 0, 0, 0, 127),
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
