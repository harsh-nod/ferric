use super::*;
use std::vec;

struct HostMath;
impl HostMath {
    fn exp_f32(&self, value: f32) -> f32 {
        value.exp()
    }
}

#[test]
fn each_row_masks_future_tokens_before_reading_pages_or_stale_nan_cache() {
    let columns = 128_usize;
    let query = vec![0_u16; 2 * 4 * 128];
    let mut keys = vec![0x7fc0_u16; 4 * 16 * columns];
    let mut values = vec![0x7fc0_u16; keys.len()];
    keys[2 * 16 * columns..(2 * 16 + 1) * columns].fill(0);
    values[2 * 16 * columns..(2 * 16 + 1) * columns].fill(0x3f80);
    keys[16 * columns..32 * columns].fill(0);
    values[16 * columns..32 * columns].fill(0x4000);
    keys[3 * 16 * columns..(3 * 16 + 1) * columns].fill(0);
    values[3 * 16 * columns..(3 * 16 + 1) * columns].fill(0x4080);
    let tables = [2_u32, u32::MAX, 1, 3];
    for lane in 0..64 {
        let first = batch_paged_attention_pair_v2!(
            &query, &keys, &values, &tables, 0, 0, 0, columns, lane, 0, 17, 2, 4, HostMath
        );
        assert_eq!(first, (0x3f80, 0x3f80));
        let second = batch_paged_attention_pair_v2!(
            &query,
            &keys,
            &values,
            &tables,
            1,
            4 * 128,
            0,
            columns,
            lane,
            16,
            17,
            2,
            4,
            HostMath
        );
        let expected = Bf16::from_f32(36.0 / 17.0).to_bits();
        assert_eq!(second, (expected, expected));
    }
}

#[test]
fn local_grouped_query_mapping_covers_all_tp1_tp2_tp8_heads() {
    for world in [1_usize, 2, 8] {
        let query_heads = 32 / world;
        let kv_heads = 8 / world;
        let columns = kv_heads * 128;
        let query = vec![0_u16; query_heads * 128];
        let keys = vec![0_u16; 16 * columns];
        let mut values = vec![0_u16; keys.len()];
        for rank in 0..world {
            for head in 0..kv_heads {
                values[head * 128..(head + 1) * 128]
                    .fill(Bf16::from_f32((rank * kv_heads + head + 1) as f32).to_bits());
            }
            for query_head in 0..query_heads {
                let kv_head = query_head / 4;
                let pair = batch_paged_attention_pair_v2!(
                    &query,
                    &keys,
                    &values,
                    &[0_u32],
                    0,
                    query_head * 128,
                    kv_head,
                    columns,
                    3,
                    0,
                    1,
                    1,
                    1,
                    HostMath
                );
                let expected = Bf16::from_f32((rank * kv_heads + kv_head + 1) as f32).to_bits();
                assert_eq!(pair, (expected, expected));
            }
        }
    }
}

#[test]
fn mixed_positions_nonzero_qk_match_independent_dense_softmax() {
    let columns = 128_usize;
    let mut query = vec![0_u16; 2 * 4 * 128];
    query[0] = Bf16::from_f32(1.0).to_bits();
    query[4 * 128] = Bf16::from_f32(-1.5).to_bits();
    let mut keys = vec![0x7fc0_u16; 4 * 16 * columns];
    let mut values = vec![0x7fc0_u16; keys.len()];
    let tables = [2_u32, u32::MAX, 1, 3];
    for (row, position) in [(0_usize, 1_usize), (1, 16)] {
        let mut dense_scores = vec![];
        let mut dense_values = vec![];
        for token in 0..=position {
            let page = tables[row * 2 + token / 16] as usize;
            let base = (page * 16 + token % 16) * columns;
            let key = if row == 0 {
                token as f32 * 8.0
            } else {
                (token as f32 - 8.0) * 0.25
            };
            let value = if row == 0 {
                1.0 + token as f32 * 2.0
            } else {
                token as f32 * 0.0625
            };
            keys[base..base + columns].fill(0);
            keys[base] = Bf16::from_f32(key).to_bits();
            values[base..base + columns].fill(Bf16::from_f32(value).to_bits());
            let q = f64::from(Bf16::from_bits(query[row * 4 * 128]).to_f32());
            dense_scores.push(q * f64::from(key) * f64::from(ATTENTION_SCALE));
            dense_values.push(f64::from(value));
        }
        let maximum = dense_scores
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let denominator: f64 = dense_scores
            .iter()
            .map(|score| (score - maximum).exp())
            .sum();
        let numerator: f64 = dense_scores
            .iter()
            .zip(&dense_values)
            .map(|(score, value)| (score - maximum).exp() * value)
            .sum();
        let expected = Bf16::from_f32((numerator / denominator) as f32).to_bits();
        let uniform =
            Bf16::from_f32((dense_values.iter().sum::<f64>() / dense_values.len() as f64) as f32)
                .to_bits();
        assert_ne!(expected, uniform);
        for lane in 0..64 {
            let pair = batch_paged_attention_pair_v2!(
                &query,
                &keys,
                &values,
                &tables,
                row,
                row * 4 * 128,
                0,
                columns,
                lane,
                position,
                17,
                2,
                4,
                HostMath
            );
            assert_eq!(pair, (expected, expected));
        }
    }
}

#[test]
#[should_panic]
fn missing_causally_required_page_traps() {
    let data = vec![0_u16; 16 * 128];
    let _ = batch_paged_attention_pair_v2!(
        &data,
        &data,
        &data,
        &[u32::MAX],
        0,
        0,
        0,
        128,
        0,
        0,
        1,
        1,
        1,
        HostMath
    );
}

#[test]
fn attention_scale_preserves_the_prior_exact_bits() {
    assert_eq!(ATTENTION_SCALE.to_bits(), 0x3db5_04f3);
}
