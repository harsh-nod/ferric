use super::*;

struct HostMath;
impl HostMath {
    fn exp_f32(&self, value: f32) -> f32 {
        value.exp()
    }
}

#[test]
fn final_physical_element_and_logical_position_have_exact_uniform_reference() {
    let query = std::vec![0_u16; 4096];
    let keys = std::vec![0_u16; 16384 * 16 * 1024];
    let mut values = std::vec![0_u16; keys.len()];
    let end = values.len();
    values[end - 128..].fill(Bf16::from_f32(32.0).to_bits());
    let mut tables = std::vec![512_u32; 512];
    tables[511] = 16383;
    let result = batch_paged_attention_pair_v9!(
        &query, &keys, &values, &tables, 0, 31 * 128, 7, 1024, 63,
        8191, 8192, 512, 16384, HostMath
    );
    let expected = Bf16::from_f32(32.0 / 8192.0).to_bits();
    assert_eq!(result, (expected, expected));
    assert_eq!(ATTENTION_SCALE.to_bits(), 0x3db5_04f3);
}

#[test]
fn nonzero_qk_crosses_old_physical_boundary_without_reading_future_poison() {
    let mut query = std::vec![0_u16; 4096];
    query[0] = Bf16::from_f32(1.0).to_bits();
    let mut keys = std::vec![0x7fc0_u16; 513 * 16 * 1024];
    let mut values = std::vec![0x7fc0_u16; keys.len()];
    let tables = [512_u32, 511, u32::MAX];
    let mut scores = std::vec::Vec::new();
    let mut dense_values = std::vec::Vec::new();
    for token in 0..17_usize {
        let base = (tables[token / 16] as usize * 16 + token % 16) * 1024;
        let key = (token as f32 - 8.0) * 0.25;
        let value = token as f32 * 0.0625;
        keys[base..base + 1024].fill(0);
        keys[base] = Bf16::from_f32(key).to_bits();
        values[base..base + 1024].fill(Bf16::from_f32(value).to_bits());
        scores.push(f64::from(key) * f64::from(ATTENTION_SCALE));
        dense_values.push(f64::from(value));
    }
    let maximum = scores.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let denominator: f64 = scores.iter().map(|score| (score - maximum).exp()).sum();
    let numerator: f64 = scores.iter().zip(&dense_values)
        .map(|(score, value)| (score - maximum).exp() * value).sum();
    let expected = Bf16::from_f32((numerator / denominator) as f32).to_bits();
    for lane in 0..64 {
        let actual = batch_paged_attention_pair_v9!(
            &query, &keys, &values, &tables, 0, 0, 0, 1024, lane,
            16, 33, 3, 513, HostMath
        );
        assert_eq!(actual, (expected, expected));
    }
}

#[test]
fn causally_required_invalid_page_and_nonfinite_values_trap() {
    let query = std::vec![0_u16; 4096];
    let keys = std::vec![0_u16; 16 * 1024];
    for page in [1_u32, 16384, u32::MAX] {
        assert!(std::panic::catch_unwind(|| {
            batch_paged_attention_pair_v9!(
                &query, &keys, &keys, &[page], 0, 0, 0, 1024, 0,
                0, 1, 1, 1, HostMath
            )
        }).is_err());
    }
    for nonfinite in [0x7fc0_u16, 0x7f80, 0xff80] {
        let mut values = keys.clone();
        values[0] = nonfinite;
        assert!(std::panic::catch_unwind(|| {
            batch_paged_attention_pair_v9!(
                &query, &keys, &values, &[0_u32], 0, 0, 0, 1024, 0,
                0, 1, 1, 1, HostMath
            )
        }).is_err());
    }
}
