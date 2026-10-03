use super::*;
use std::{cmp::Ordering, vec, vec::Vec};

const N: usize = 151936;

// Independent numerical reference: widening BF16 bits, not the ordered-key map.
fn number(raw: u16) -> f32 {
    f32::from_bits(u32::from(raw) << 16)
}

fn scalar(values: &[u16]) -> Result<u32, ()> {
    assert_eq!(values.len(), N);
    let mut maximum = number(values[0]);
    let mut winner = 0;
    for (token, &raw) in values.iter().enumerate() {
        let value = number(raw);
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

// Pure numeric model only: no device capability or subgroup execution is emulated.
fn parallel(values: &[u16], base: usize) -> Result<u32, ()> {
    let logits_view = StridedReadView2D::from_shared_slice(values, base, 1, N, N).unwrap();
    let row = 0;
    let lanes: [(f32, u32, f32); 64] =
        core::array::from_fn(|lane| lane_argmax!(logits_view, row, lane));
    let invalid = lanes.iter().map(|lane| lane.2).fold(0.0_f32, f32::max);
    let maximum = lanes.iter().map(|lane| lane.0).fold(0.0_f32, f32::max);
    let key = lanes
        .iter()
        .map(|&(value, winner, _)| stable_key!(value, maximum, winner))
        .fold(0.0_f32, f32::max);
    if invalid != 0.0 {
        return Err(());
    }
    Ok(decode_key!(key))
}

#[test]
fn all_65536_patterns_match_independent_finite_classification_and_numeric_order() {
    let mut finite = Vec::new();
    let mut invalid_count = 0;
    for raw in 0..=u16::MAX {
        let key = ordered_bits!(u32::from(raw));
        let value = number(raw);
        assert_eq!(key != 0, value.is_finite(), "raw={raw:04x}");
        if value.is_finite() {
            assert_eq!(key as f32 as u32, key);
            finite.push((raw, value, key));
        } else {
            invalid_count += 1;
        }
    }
    assert_eq!(invalid_count, 256);
    assert_eq!(finite.len(), 65280);
    finite.sort_by(|left, right| left.1.partial_cmp(&right.1).unwrap());
    for pair in finite.windows(2) {
        let numeric_order = pair[0].1.partial_cmp(&pair[1].1).unwrap();
        assert_eq!(pair[0].2.cmp(&pair[1].2), numeric_order);
        if numeric_order == Ordering::Equal {
            assert_eq!(pair[0].1, 0.0);
            assert!(matches!(pair[0].0, 0 | 0x8000));
            assert!(matches!(pair[1].0, 0 | 0x8000));
        }
    }
    assert_eq!(ordered_bits!(0_u32), ordered_bits!(0x8000_u32));
    const {
        assert!(ordered_bits!(0x8001_u32) < ordered_bits!(0_u32));
        assert!(ordered_bits!(0x0001_u32) > ordered_bits!(0_u32));
    }
}

#[test]
fn each_active_logit_has_exactly_one_lane_owner() {
    let mut owners = vec![0_u8; N];
    for lane in 0..64 {
        for step in 0..2374 {
            owners[step * 64 + lane] += 1;
        }
    }
    assert!(owners.iter().all(|&count| count == 1));
}

#[test]
fn every_token_tie_key_is_exact_nonzero_and_strictly_ordered() {
    let mut previous = f32::INFINITY;
    for token in 0..N as u32 {
        let key = stable_key!(1.0, 1.0, token);
        assert_eq!(key as u32, N as u32 - token);
        assert!(key > 0.0 && key < previous);
        assert_eq!(decode_key!(key), token);
        previous = key;
    }
    assert_eq!(stable_key!(1.0, 2.0, 0), 0.0);
}

#[test]
fn decoding_rejects_invalid_integer_keys() {
    for key in [0.0, -0.0, -1.0, 151937.0, f32::NAN, f32::INFINITY] {
        assert!(std::panic::catch_unwind(|| decode_key!(key)).is_err());
    }
}

#[test]
fn winner_in_every_lane_and_scan_boundary_matches_serial() {
    let mut values = vec![0xbf80_u16; N];
    for lane in 0..64 {
        for step in [0, 1, 1187, 2373] {
            let token = step * 64 + lane;
            values[token] = 0x3f80;
            assert_eq!(parallel(&values, 0), Ok(token as u32));
            assert_eq!(parallel(&values, 0), scalar(&values));
            values[token] = 0xbf80;
        }
    }
}

#[test]
fn ties_and_signed_zero_orders_choose_lowest_token_across_lanes() {
    let mut values = vec![0xbf80_u16; N];
    for (first, second) in [(0, 63), (63, 64), (127, 128), (64, 128), (63, N - 1)] {
        for (left, right) in [(0x3f80, 0x3f80), (0, 0x8000), (0x8000, 0)] {
            values[first] = left;
            values[second] = right;
            assert_eq!(parallel(&values, 0), Ok(first as u32));
            assert_eq!(parallel(&values, 0), scalar(&values));
            values[first] = 0xbf80;
            values[second] = 0xbf80;
        }
    }
}

#[test]
fn uniform_rows_finite_extremes_and_subnormals_match_serial() {
    for raw in [0xff7f, 0xbf80, 0x8001, 0x8000, 0, 1, 0x007f, 0x0080, 0x7f7f] {
        let mut values = vec![raw; N];
        assert_eq!(parallel(&values, 0), Ok(0));
        assert_eq!(parallel(&values, 0), scalar(&values));
        if raw != 0x7f7f {
            values[N - 1] = 0x7f7f;
            assert_eq!(parallel(&values, 0), Ok((N - 1) as u32));
        }
    }
    let mut values = vec![0x8001; N];
    values[63] = 0x8000;
    values[64] = 0;
    values[N - 1] = 1;
    assert_eq!(parallel(&values, 0), Ok((N - 1) as u32));
    assert_eq!(parallel(&values, 0), scalar(&values));
}

#[test]
fn random_finite_bit_patterns_match_serial_without_narrowing() {
    let mut values = vec![0_u16; N];
    let mut state = 0x9e37_79b9_u32;
    for _ in 0..16 {
        for raw in &mut values {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            let candidate = (state & 0xffff) as u16;
            *raw = if number(candidate).is_finite() {
                candidate
            } else {
                candidate ^ 0x0080
            };
        }
        assert_eq!(parallel(&values, 0), scalar(&values));
    }
}

#[test]
fn every_lane_rejects_nonwinning_infinity_and_nan_before_publication() {
    let mut values = vec![0xbf80; N];
    values[0] = 0x7f7f;
    for lane in 0..64 {
        for step in [0, 1187, 2373] {
            let token = step * 64 + lane;
            for invalid in [0x7f80, 0xff80, 0x7f81, 0xff81, 0x7fc0, 0xffc0] {
                let saved = values[token];
                values[token] = invalid;
                assert_eq!(parallel(&values, 0), Err(()));
                values[token] = saved;
            }
        }
    }
    for invalid in [0x7f80, 0xff80, 0x7f81, 0xff81, 0x7fc0, 0xffc0] {
        values.fill(invalid);
        assert_eq!(parallel(&values, 0), Err(()));
    }
}

#[test]
fn exact_single_request_shape_rejects_under_and_oversized_extents() {
    const { assert!(!invalid_shape!(1_u32, 16 * N, 16_usize, 64_usize)) };
    for rows in [0, 2, 16, 17, u32::MAX] {
        assert!(invalid_shape!(rows, 16 * N, 16_usize, 64_usize));
    }
    for len in [0, N, 16 * N - 1, 16 * N + 1, usize::MAX] {
        assert!(invalid_shape!(1_u32, len, 16_usize, 64_usize));
    }
    for len in [0, 1, 15, 17, usize::MAX] {
        assert!(invalid_shape!(1_u32, 16 * N, len, 64_usize));
    }
    for extent in [0, 1, 32, 63, 65, 128, usize::MAX] {
        assert!(invalid_shape!(1_u32, 16 * N, 16_usize, extent));
    }
}

#[test]
fn inactive_fifteen_rows_nan_padding_and_choice_tail_remain_untouched() {
    let mut values = vec![0xff81_u16; 16 * N + 2];
    values[1..1 + N].fill(0x8001);
    values[1 + 64] = 0x0001;
    let before = values.clone();
    let mut choices = [0xa5a5_a5a5_u32; 18];
    choices[1] = parallel(&values, 1).unwrap();
    assert_eq!(choices[1], 64);
    assert_eq!(choices[0], 0xa5a5_a5a5);
    assert!(choices[2..].iter().all(|&value| value == 0xa5a5_a5a5));
    assert_eq!(values, before);
}
