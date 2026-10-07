use super::*;
use std::vec;

const N: usize = 151936;

fn scalar(values: &[f32]) -> Result<u32, ()> {
    assert_eq!(values.len(), N);
    let mut winner = 0;
    let mut maximum = values[0];
    for (token, &value) in values.iter().enumerate() {
        if !value.is_finite() {
            return Err(());
        }
        if value > maximum {
            maximum = value;
            winner = token as u32;
        }
    }
    Ok(winner)
}

// Numerical oracle only: this does not construct or emulate a device capability.
fn parallel(values: &[f32], base: usize) -> Result<u32, ()> {
    let logits_view = StridedReadView2D::from_shared_slice(values, base, 1, N, N).unwrap();
    let row = 0;
    let lanes: [(f32, u32, f32); 64] =
        core::array::from_fn(|lane| lane_argmax!(logits_view, row, lane));
    let invalid = lanes.iter().map(|lane| lane.2).fold(0.0_f32, f32::max);
    if invalid != 0.0 {
        return Err(());
    }
    let maximum = lanes.iter().map(|lane| lane.0).fold(f32::MIN, f32::max);
    let key = lanes
        .iter()
        .map(|&(value, winner, _)| stable_key!(value, maximum, winner))
        .fold(0.0_f32, f32::max);
    Ok(decode_key!(key))
}

#[test]
fn each_logit_has_exactly_one_lane_owner() {
    let mut owners = vec![0_u8; N];
    for lane in 0..64 {
        for step in 0..2374 {
            owners[step * 64 + lane] += 1;
        }
    }
    assert!(owners.iter().all(|&count| count == 1));
}

#[test]
fn every_token_key_is_exact_nonzero_and_strictly_ordered() {
    let mut previous = f32::INFINITY;
    for token in 0..N as u32 {
        let key = stable_key!(1.0, 1.0, token);
        assert_eq!(key as u32, N as u32 - token);
        assert!(key > 0.0 && key < previous);
        assert_eq!(N as u32 - key as u32, token);
        assert_eq!(decode_key!(key), token);
        previous = key;
    }
    assert_eq!(stable_key!(-1.0, 1.0, 0), 0.0);
}

#[test]
fn decoding_rejects_zero_and_out_of_range_integer_keys() {
    for key in [0.0, -0.0, -1.0, 151937.0, f32::NAN, f32::INFINITY] {
        assert!(std::panic::catch_unwind(|| decode_key!(key)).is_err());
    }
}

#[test]
fn winner_in_every_lane_and_scan_boundary_matches_serial() {
    let mut values = vec![-2.0_f32; N];
    for lane in 0..64 {
        for step in [0, 1, 1187, 2373] {
            let token = step * 64 + lane;
            values[token] = 3.0;
            assert_eq!(parallel(&values, 0), Ok(token as u32));
            assert_eq!(parallel(&values, 0), scalar(&values));
            values[token] = -2.0;
        }
    }
}

#[test]
fn exact_ties_and_both_signed_zero_orders_choose_lowest_id() {
    let mut values = vec![-2.0_f32; N];
    for (first, second) in [(0, 63), (63, 64), (127, 128), (64, 128), (0, N - 1)] {
        for (left, right) in [(2.0, 2.0), (-0.0, 0.0), (0.0, -0.0)] {
            values[first] = left;
            values[second] = right;
            assert_eq!(parallel(&values, 0), Ok(first as u32));
            assert_eq!(parallel(&values, 0), scalar(&values));
            values[first] = -2.0;
            values[second] = -2.0;
        }
    }
}

#[test]
fn finite_extremes_uniform_rows_and_subnormals_match_serial() {
    for value in [f32::MIN, -1.0, -0.0, 0.0, f32::from_bits(1), f32::MAX] {
        let mut values = vec![value; N];
        assert_eq!(parallel(&values, 0), Ok(0));
        assert_eq!(parallel(&values, 0), scalar(&values));
        if value != f32::MAX {
            values[N - 1] = f32::MAX;
            assert_eq!(parallel(&values, 0), Ok((N - 1) as u32));
        }
    }
    let mut values = vec![-f32::from_bits(1); N];
    values[64] = 0.0;
    values[129] = f32::from_bits(1);
    assert_eq!(parallel(&values, 0), Ok(129));
}

#[test]
fn fp32_values_collapsed_by_bf16_are_not_ties() {
    let mut values = vec![-1.0_f32; N];
    values[7] = 24.365_898;
    values[101] = 24.426_361;
    assert_eq!(
        fe2o3_device::Bf16::from_f32(values[7]).to_bits(),
        fe2o3_device::Bf16::from_f32(values[101]).to_bits()
    );
    assert_eq!(parallel(&values, 0), Ok(101));
}

#[test]
fn random_finite_bit_patterns_match_serial_without_narrowing() {
    let mut values = vec![0.0_f32; N];
    let mut state = 0x9e37_79b9_u32;
    for _ in 0..16 {
        for value in &mut values {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            let bits = if state & 0x7f80_0000 == 0x7f80_0000 {
                state ^ 0x0080_0000
            } else {
                state
            };
            *value = f32::from_bits(bits);
        }
        assert_eq!(parallel(&values, 0), scalar(&values));
    }
}

#[test]
fn every_lane_rejects_nonfinites_before_publishing_a_winner() {
    let mut values = vec![-1.0_f32; N];
    for lane in 0..64 {
        for step in [0, 1187, 2373] {
            let token = step * 64 + lane;
            for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                values[token] = invalid;
                assert_eq!(parallel(&values, 0), Err(()));
                values[token] = -1.0;
            }
        }
    }
}

#[test]
fn signaling_negative_nans_and_nonwinning_nonfinites_reject() {
    let mut values = vec![-1.0_f32; N];
    values[0] = f32::MAX;
    for bits in [0x7f80_0001, 0xff80_0001, 0xffc0_0000] {
        values[N - 1] = f32::from_bits(bits);
        assert_eq!(parallel(&values, 0), Err(()));
    }
}

#[test]
fn active_rows_do_not_read_nonfinite_capacity_tails_or_mutate_inputs() {
    for rows in [1, 2, 16, 17, 31, 32] {
        let mut values = vec![f32::from_bits(0x7fc0_1234); 32 * N + 2];
        let mut choices = [0xa5a5_a5a5_u32; 34];
        values[1..1 + rows * N].fill(-1.0);
        for row in 0..rows {
            values[1 + row * N + row * 997] = 1.0;
        }
        let before: std::vec::Vec<_> = values.iter().map(|value| value.to_bits()).collect();
        for row in 0..rows {
            choices[1 + row] = parallel(&values, 1 + row * N).unwrap();
            assert_eq!(choices[1 + row], (row * 997) as u32);
        }
        assert_eq!(choices[0], 0xa5a5_a5a5);
        assert!(
            choices[1 + rows..]
                .iter()
                .all(|&value| value == 0xa5a5_a5a5)
        );
        assert!(
            values
                .iter()
                .zip(&before)
                .all(|(value, &bits)| value.to_bits() == bits)
        );
    }
}
