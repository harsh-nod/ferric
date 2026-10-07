use super::*;

struct HostMath;
impl HostMath {
    fn exp_f32(&self, value: f32) -> f32 {
        value.exp()
    }
}

#[test]
fn ratio_two_distinct_heads_and_causal_poisoned_future_pages() {
    let query = std::vec![0_u16;16*128];
    let keys = std::vec![0_u16;2*16*1024];
    let mut values = std::vec![0_u16;2*16*1024];
    for token in 0..32 {
        for head in 0..8 {
            for column in 0..128 {
                values[token * 1024 + head * 128 + column] =
                    Bf16::from_f32(head as f32 + token as f32 / 32.0).to_bits();
            }
        }
    }
    for position in [0_usize, 15, 16, 31] {
        let tables = [0_u32, if position < 16 { u32::MAX } else { 1 }];
        for head in 0..16 {
            for lane in [0_usize, 31, 63] {
                let (first, second) = batch_paged_attention_pair_v10!(
                    &query,
                    &keys,
                    &values,
                    &tables,
                    0,
                    head * 128,
                    head / 2,
                    1024,
                    lane,
                    position,
                    32,
                    2,
                    2,
                    HostMath
                );
                let mut sum = 0_f32;
                for token in 0..=position {
                    sum += Bf16::from_bits(values[token * 1024 + (head / 2) * 128 + lane]).to_f32();
                }
                let expected = Bf16::from_f32(sum / (position + 1) as f32).to_bits();
                assert_eq!((first, second), (expected, expected));
            }
        }
    }
}

#[test]
fn invalid_reachable_page_or_nonfinite_values_trap_on_host() {
    let query = std::vec![0_u16;128];
    let keys = std::vec![0_u16;16*1024];
    let mut values = keys.clone();
    let tables = [1_u32];
    assert!(
        std::panic::catch_unwind(|| batch_paged_attention_pair_v10!(
            &query, &keys, &values, &tables, 0, 0, 0, 1024, 0, 0, 1, 1, 1, HostMath
        ))
        .is_err()
    );
    values[0] = 0x7f80;
    let tables = [0_u32];
    assert!(
        std::panic::catch_unwind(|| batch_paged_attention_pair_v10!(
            &query, &keys, &values, &tables, 0, 0, 0, 1024, 0, 0, 1, 1, 1, HostMath
        ))
        .is_err()
    );
}
