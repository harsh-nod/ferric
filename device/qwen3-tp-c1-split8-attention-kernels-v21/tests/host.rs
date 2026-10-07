use fe2o3_device::{Bf16, StridedReadView2D};
use ferric_qwen3_tp_c1_split8_attention_kernels_device_v21::{
    NUMERATOR_ELEMENTS_V21, STATS_ELEMENTS_V21,
};

#[path = "support/model.rs"]
mod model;

#[test]
fn merge_load_fallback_is_nonfinite_and_valid_coordinates_preserve_values() {
    let stats = vec![1.0_f32; STATS_ELEMENTS_V21];
    let numerators = vec![-2.0_f32; NUMERATOR_ELEMENTS_V21];
    let stats_view = StridedReadView2D::from_shared_slice(&stats, 0, 256, 2, 2).unwrap();
    let numerator_view =
        StridedReadView2D::from_shared_slice(&numerators, 0, 256, 128, 128).unwrap();
    for row in 0..256 {
        for column in 0..2 {
            assert_eq!(stats_view.load_or(row, column, f32::INFINITY), 1.0);
        }
        for column in 0..128 {
            assert_eq!(numerator_view.load_or(row, column, f32::INFINITY), -2.0);
        }
    }
    for (row, column) in [(256, 0), (0, 2), (usize::MAX, usize::MAX)] {
        assert!(!stats_view.load_or(row, column, f32::INFINITY).is_finite());
    }
    for (row, column) in [(256, 0), (0, 128), (usize::MAX, usize::MAX)] {
        assert!(
            !numerator_view
                .load_or(row, column, f32::INFINITY)
                .is_finite()
        );
    }
}

#[test]
fn every_bounded_context_partitions_once_with_at_most_32_tokens_per_wave() {
    for active in 128..=256 {
        let ranges = model::ranges(active).unwrap();
        let mut covered = vec![0_u8; active];
        for range in ranges {
            assert!(!range.is_empty() && range.len() <= 32);
            for token in range {
                covered[token] += 1;
            }
        }
        assert!(covered.iter().all(|count| *count == 1));
    }
    for bad in [0, 1, 127, 257, 8192, usize::MAX] {
        assert!(model::ranges(bad).is_err());
    }
    assert!(
        model::ranges(192)
            .unwrap()
            .iter()
            .all(|range| range.len() == 24)
    );
}

#[test]
fn wave_lane_ownership_covers_both_scratch_buffers_and_only_active_output() {
    let mut stats = vec![0_u8; STATS_ELEMENTS_V21];
    let mut numerators = vec![0_u8; NUMERATOR_ELEMENTS_V21];
    let mut output = vec![0_u8; 32 * 4096];
    for row in 0..256 {
        let query_head = row / 8;
        let partition = row % 8;
        assert!(query_head < 32 && partition < 8 && query_head / 4 < 8);
        for lane in 0..64 {
            numerators[row * 128 + lane] += 1;
            numerators[row * 128 + lane + 64] += 1;
            if lane < 2 {
                stats[row * 2 + lane] += 1;
            }
        }
    }
    for head in 0..32 {
        for lane in 0..64 {
            output[head * 128 + lane] += 1;
            output[head * 128 + lane + 64] += 1;
            for partition in 0..8 {
                let row = head * 8 + partition;
                assert!(row * 2 + 1 < stats.len());
                assert!(row * 128 + lane + 64 < numerators.len());
            }
        }
    }
    assert!(stats.iter().all(|count| *count == 1));
    assert!(numerators.iter().all(|count| *count == 1));
    assert!(output[..4096].iter().all(|count| *count == 1));
    assert!(output[4096..].iter().all(|count| *count == 0));
}

#[test]
fn noncontiguous_pages_map_only_active_tokens_even_with_poisoned_tail_and_padding() {
    for active in [128_usize, 129, 135, 191, 192, 193, 255, 256] {
        let mut table = vec![u32::MAX; 32 * 512];
        for (logical, entry) in table.iter_mut().take(active.div_ceil(16)).enumerate() {
            *entry = u32::try_from(511 - logical * 2).unwrap();
        }
        let mut cache_rows = vec![f32::NAN; 512 * 16];
        for token in 0..active {
            let page = usize::try_from(table[token / 16]).unwrap();
            cache_rows[page * 16 + token % 16] = f32::from(u16::try_from(token).unwrap());
        }
        let mut visits = vec![0_u8; active];
        for range in model::ranges(active).unwrap() {
            for token in range {
                let page = usize::try_from(table[token / 16]).unwrap();
                assert!(page < 512);
                let row = page * 16 + token % 16;
                assert_eq!(
                    cache_rows[row].to_bits(),
                    f32::from(u16::try_from(token).unwrap()).to_bits()
                );
                for head in 0..32 {
                    for lane in 0..64 {
                        let column = (head / 4) * 128 + lane;
                        assert!(column + 64 < 1024);
                        assert!(row * 1024 + column + 64 < 512 * 16 * 1024);
                    }
                }
                visits[token] += 1;
            }
        }
        assert!(visits.iter().all(|count| *count == 1));
        assert!(
            table[active.div_ceil(16)..]
                .iter()
                .all(|page| *page == u32::MAX)
        );
        assert_eq!(
            cache_rows.iter().filter(|value| value.is_finite()).count(),
            active
        );
    }
}

#[test]
fn constant_power_of_two_values_retain_exact_bf16_oracles_for_all_lengths() {
    for active in [128_usize, 129, 135, 191, 192, 193, 255, 256] {
        let scores: Vec<_> = (0..active)
            .map(|token| f32::from(u16::try_from(token % 5).unwrap()) / 8.0)
            .collect();
        let row = std::array::from_fn(|column| [0.125_f32, 0.25, 0.5, 1.0][column % 4]);
        let values = vec![row; active];
        let candidate = model::split(&scores, &values).unwrap();
        let baseline = model::serial(&scores, &values).unwrap();
        for ((actual, original), expected) in candidate.into_iter().zip(baseline).zip(row) {
            assert_eq!(
                Bf16::from_f32(actual).to_bits(),
                Bf16::from_f32(expected).to_bits()
            );
            assert_eq!(
                Bf16::from_f32(original).to_bits(),
                Bf16::from_f32(expected).to_bits()
            );
        }
    }
}

#[test]
fn variable_token_values_exercise_the_merge_corrections_against_a_double_precision_model() {
    for active in [128_usize, 129, 192, 255, 256] {
        let scores: Vec<_> = (0..active)
            .map(|token| f32::from(i16::try_from(token % 17).unwrap()) * 0.375 - 3.0)
            .collect();
        let values: Vec<[f32; 128]> = (0..active)
            .map(|token| {
                std::array::from_fn(|column| {
                    f32::from(i16::try_from((token * 11 + column * 3) % 23).unwrap()) / 16.0 - 0.75
                })
            })
            .collect();
        let candidate = model::split(&scores, &values).unwrap();
        let baseline = model::serial(&scores, &values).unwrap();
        let ideal = model::ideal(&scores, &values);
        for ((actual, original), expected) in candidate.into_iter().zip(baseline).zip(ideal) {
            // This threshold covers only these CPU fixtures, not GPU acceptance.
            assert!((f64::from(actual) - expected).abs() < 0.00001);
            assert!((f64::from(original) - expected).abs() < 0.00001);
        }
    }
}

#[test]
fn cancellation_demonstrates_that_the_new_route_does_not_claim_v14_bit_parity() {
    let scores = vec![0.0_f32; 128];
    let mut values = vec![[0.0_f32; 128]; 128];
    values[0].fill(16_777_216.0);
    values[16].fill(1.0);
    values[17].fill(-16_777_216.0);
    values[32].fill(1.0);
    let serial = model::serial(&scores, &values).unwrap();
    let split = model::split(&scores, &values).unwrap();
    assert_ne!(serial[0].to_bits(), split[0].to_bits());
    assert_ne!(
        Bf16::from_f32(serial[0]).to_bits(),
        Bf16::from_f32(split[0]).to_bits()
    );
}

#[test]
fn nonfinite_inputs_states_and_invalid_scratch_denominators_are_rejected() {
    let scores = vec![0.0_f32; 128];
    let values = vec![[1.0_f32; 128]; 128];
    let states: Vec<_> = model::ranges(128)
        .unwrap()
        .into_iter()
        .map(|range| model::partition(&scores, &values, range).unwrap())
        .collect();
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut changed = scores.clone();
        changed[17] = bad;
        assert!(model::split(&changed, &values).is_err());
        let mut changed = values.clone();
        changed[127][127] = bad;
        assert!(model::split(&scores, &changed).is_err());
        let mut changed = states.clone();
        changed[3].maximum = bad;
        assert!(model::merge(&changed).is_err());
        let mut changed = states.clone();
        changed[7].numerators[127] = bad;
        assert!(model::merge(&changed).is_err());
    }
    for bad in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        let mut changed = states.clone();
        changed[4].denominator = bad;
        assert!(model::merge(&changed).is_err());
    }
    assert!(model::merge(&states[..7]).is_err());
    assert!(model::partition(&scores, &values[..127], 0..16).is_err());
    assert!(model::partition(&scores, &values, 128..129).is_err());
    let overflowing = vec![[f32::MAX; 128]; 128];
    assert!(model::split(&scores, &overflowing).is_err());
}
