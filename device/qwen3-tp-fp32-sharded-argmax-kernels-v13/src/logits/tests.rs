use super::*;
use std::vec;

const N: usize = 151936;
const SHARDS: usize = 64;

fn scalar(values: &[f32]) -> Result<u32, ()> {
    assert_eq!(values.len(), N);
    let mut maximum = values[0];
    let mut winner = 0;
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

// Numerical fixtures only; these do not construct device capabilities or model scheduling.
fn produce_shard(values: &[f32], base: usize, shard: usize) -> (f32, f32) {
    let view = StridedReadView2D::from_shared_slice(values, base, 1, N, N).unwrap();
    let lanes: [(f32, u32, f32); 64] =
        core::array::from_fn(|lane| shard_lane_argmax!(view, 0, shard, lane));
    let invalid = lanes.iter().map(|lane| lane.2).fold(0.0_f32, f32::max);
    let maximum = lanes.iter().map(|lane| lane.0).fold(f32::MIN, f32::max);
    let key = lanes
        .iter()
        .map(|&(value, winner, _)| stable_token_key!(value, maximum, winner))
        .fold(0.0_f32, f32::max);
    (maximum, if invalid == 0.0 { key } else { 0.0 })
}

fn produce(values: &[f32], base: usize) -> [(f32, f32); SHARDS] {
    core::array::from_fn(|shard| produce_shard(values, base, shard))
}

fn finalize(shards: &[(f32, f32); SHARDS]) -> Result<u32, ()> {
    let lanes: [(f32, f32, f32); 64] = core::array::from_fn(|lane| {
        let (value, key) = shards[lane];
        final_lane_input!(value, key)
    });
    let invalid = lanes.iter().map(|lane| lane.2).fold(0.0_f32, f32::max);
    if invalid != 0.0 {
        return Err(());
    }
    let maximum = lanes.iter().map(|lane| lane.0).fold(f32::MIN, f32::max);
    let key = lanes
        .iter()
        .map(|&(value, key, _)| winning_shard_key!(value, maximum, key))
        .fold(0.0_f32, f32::max);
    Ok(decode_key!(key))
}

fn parallel(values: &[f32], base: usize) -> Result<u32, ()> {
    finalize(&produce(values, base))
}

#[test]
fn exhaustive_token_ownership_has_exact_six_long_shards_and_no_tail_reads() {
    let mut owners = vec![0_u8; N];
    let mut counts = [[0_usize; 64]; SHARDS];
    for (shard, lanes) in counts.iter_mut().enumerate() {
        for (lane, count) in lanes.iter_mut().enumerate() {
            for step in 0..38 {
                let stripe = 64 * step + shard;
                if stripe < 2374 {
                    let token = 64 * stripe + lane;
                    assert!(token < N);
                    assert_eq!(token % 64, lane);
                    assert_eq!((token / 64) % 64, shard);
                    owners[token] += 1;
                    *count += 1;
                } else {
                    assert!(64 * stripe + lane >= N);
                    assert_eq!(step, 37);
                    assert!(shard >= 6);
                }
            }
            assert_eq!(*count, if shard < 6 { 38 } else { 37 });
        }
    }
    assert!(owners.iter().all(|&count| count == 1));
    assert_eq!(counts.iter().flatten().sum::<usize>(), N);
}

#[test]
fn every_token_key_is_exact_positive_ordered_and_decodes() {
    let mut previous = f32::INFINITY;
    for token in 0..N as u32 {
        let key = stable_token_key!(1.0, 1.0, token);
        assert_eq!(key as u32, N as u32 - token);
        assert!(key > 0.0 && key < previous);
        assert_eq!(final_lane_input!(1.0_f32, key), (1.0, key, 0.0));
        assert_eq!(decode_key!(key), token);
        previous = key;
    }
    assert_eq!(stable_token_key!(-1.0, 1.0, 0), 0.0);
}

#[test]
fn decode_rejects_zero_and_out_of_range_keys() {
    for key in [0.0, -0.0, -1.0, 151937.0, f32::NAN, f32::INFINITY] {
        assert!(std::panic::catch_unwind(|| decode_key!(key)).is_err());
    }
}

#[test]
fn every_shard_lane_and_scan_boundary_can_win() {
    let mut values = vec![-2.0_f32; N];
    let baseline = produce(&values, 0);
    for shard in 0..SHARDS {
        let last_step = if shard < 6 { 37 } else { 36 };
        for lane in 0..64 {
            for step in [0, 1, last_step] {
                let token = 64 * (64 * step + shard) + lane;
                values[token] = 3.0;
                let mut shards = baseline;
                shards[shard] = produce_shard(&values, 0, shard);
                assert_eq!(shards[shard], (3.0, (N - token) as f32));
                assert_eq!(finalize(&shards), Ok(token as u32));
                values[token] = -2.0;
            }
        }
    }
}

#[test]
fn ties_across_lanes_shards_and_steps_choose_lowest_id_in_both_zero_orders() {
    let mut values = vec![-2.0_f32; N];
    for (first, second) in [
        (0, 63),
        (63, 64),
        (64, 4096),
        (4095, 4096),
        (4096, 8192),
        (N - 65, N - 1),
        (0, N - 1),
    ] {
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
fn finite_extremes_uniform_rows_and_subnormals_match_scalar() {
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
    values[4097] = f32::from_bits(1);
    assert_eq!(parallel(&values, 0), Ok(4097));
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
fn random_finite_bit_patterns_match_scalar_without_narrowing() {
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
fn each_lane_keeps_nonfinite_flag_after_later_finite_candidates() {
    let mut values = vec![-1.0_f32; N];
    for shard in 0..SHARDS {
        let last_step = if shard < 6 { 37 } else { 36 };
        for lane in 0..64 {
            let last = 64 * (64 * last_step + shard) + lane;
            values[last] = f32::MAX;
            for step in [0, 1, last_step] {
                let token = 64 * (64 * step + shard) + lane;
                let saved = values[token];
                for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                    values[token] = invalid;
                    let view = StridedReadView2D::from_shared_slice(&values, 0, 1, N, N).unwrap();
                    let (_, _, flag) = shard_lane_argmax!(view, 0, shard, lane);
                    assert_eq!(flag, 1.0);
                }
                values[token] = saved;
            }
            values[last] = -1.0;
        }
    }
}

#[test]
fn each_shard_propagates_nonwinning_nan_or_infinity_as_zero_key() {
    let mut values = vec![-1.0_f32; N];
    values[0] = f32::MAX;
    let baseline = produce(&values, 0);
    for shard in 0..SHARDS {
        let token = shard * 64 + 63;
        for invalid in [
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::from_bits(0x7f80_0001),
            f32::from_bits(0xff80_0001),
            f32::from_bits(0xffc0_0000),
        ] {
            values[token] = invalid;
            let mut shards = baseline;
            shards[shard] = produce_shard(&values, 0, shard);
            assert_eq!(shards[shard].1.to_bits(), 0);
            assert_eq!(finalize(&shards), Err(()));
            values[token] = -1.0;
        }
    }
}

#[test]
fn finalizer_checks_every_key_even_when_its_maximum_loses() {
    let baseline = core::array::from_fn(|shard| (-1.0_f32, (N - shard * 64) as f32));
    for shard in 0..SHARDS {
        for key in [
            0.0,
            -0.0,
            -1.0,
            f32::from_bits(1),
            0.5,
            1.5,
            151935.5,
            151937.0,
            f32::MAX,
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
        ] {
            let mut shards = baseline;
            let other = (shard + 1) % SHARDS;
            shards[other].0 = f32::MAX;
            shards[shard].1 = key;
            assert_eq!(finalize(&shards), Err(()));
        }
    }
}

#[test]
fn finalizer_rejects_nonfinite_scratch_maxima_in_every_shard() {
    let baseline = core::array::from_fn(|shard| (-1.0_f32, (N - shard * 64) as f32));
    for shard in 0..SHARDS {
        for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut shards = baseline;
            shards[shard].0 = invalid;
            assert_eq!(finalize(&shards), Err(()));
        }
    }
}

#[test]
fn active_rows_preserve_scratch_choice_guards_and_nonfinite_capacity_tails() {
    for rows in [1, 2, 16, 17, 31, 32] {
        let guard = 0x7fc0_1234;
        let mut values = vec![f32::from_bits(guard); 32 * N + 2];
        let mut maxima = [f32::from_bits(guard); 32 * SHARDS + 2];
        let mut keys = maxima;
        let mut choices = [0xa5a5_a5a5_u32; 34];
        values[1..1 + rows * N].fill(-1.0);
        for row in 0..rows {
            values[1 + row * N + row * 997] = 1.0;
        }
        let before: std::vec::Vec<_> = values.iter().map(|value| value.to_bits()).collect();
        for row in 0..rows {
            let shards = produce(&values, 1 + row * N);
            for (shard, &(maximum, key)) in shards.iter().enumerate() {
                maxima[1 + row * SHARDS + shard] = maximum;
                keys[1 + row * SHARDS + shard] = key;
            }
        }
        let maxima_view =
            StridedReadView2D::from_shared_slice(&maxima, 1, rows, SHARDS, SHARDS).unwrap();
        let keys_view =
            StridedReadView2D::from_shared_slice(&keys, 1, rows, SHARDS, SHARDS).unwrap();
        for row in 0..rows {
            let shards = core::array::from_fn(|shard| {
                (
                    maxima_view.load_or(row, shard, 0.0),
                    keys_view.load_or(row, shard, 0.0),
                )
            });
            choices[1 + row] = finalize(&shards).unwrap();
            assert_eq!(choices[1 + row], (row * 997) as u32);
        }
        for scratch in [&maxima, &keys] {
            assert_eq!(scratch[0].to_bits(), guard);
            assert!(
                scratch[1 + rows * SHARDS..]
                    .iter()
                    .all(|value| value.to_bits() == guard)
            );
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
