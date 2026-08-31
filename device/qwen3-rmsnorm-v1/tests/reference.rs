use ferric_qwen3_rmsnorm_device_v1::{
    QWEN3_RMSNORM_BEHAVIOR_PURE_V1, QWEN3_RMSNORM_BEHAVIOR_RESIDUAL_FUSED_V1,
    QWEN3_RMSNORM_EPSILON_BITS_V1,
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

fn xor_reduce_f32(mut values: [f32; 64]) -> f32 {
    let mut offset = 32;
    while offset != 0 {
        let previous = values;
        for lane in 0..64 {
            values[lane] = previous[lane] + previous[lane ^ offset];
        }
        offset >>= 1;
    }
    for value in values {
        assert_eq!(value.to_bits(), values[0].to_bits());
    }
    values[0]
}

fn reference_rmsnorm(
    input: &[u16],
    residual: &[u16],
    weight: &[u16],
    rows: usize,
    width: usize,
    behavior: u32,
) -> (Vec<u16>, Vec<u16>) {
    let fused_mode = behavior == QWEN3_RMSNORM_BEHAVIOR_RESIDUAL_FUSED_V1;
    assert!(fused_mode || behavior == QWEN3_RMSNORM_BEHAVIOR_PURE_V1);
    assert_eq!(input.len(), rows * width);
    assert_eq!(weight.len(), width);
    assert_eq!(residual.len(), if fused_mode { rows * width } else { 0 });
    let mut fused_output = if fused_mode {
        vec![0_u16; rows * width]
    } else {
        Vec::new()
    };
    let mut normalized_output = vec![0_u16; rows * width];
    let epsilon = f32::from_bits(QWEN3_RMSNORM_EPSILON_BITS_V1);
    for row in 0..rows {
        let row_base = row * width;
        let mut lane_sums = [0.0_f32; 64];
        for (lane, lane_sum) in lane_sums.iter_mut().enumerate() {
            for component in 0..64 {
                let column = lane + component * 64;
                if column < width {
                    let index = row_base + column;
                    let input_value = widen_bf16(input[index]);
                    let normalized_input = if fused_mode {
                        input_value + widen_bf16(residual[index])
                    } else {
                        input_value
                    };
                    *lane_sum += normalized_input * normalized_input;
                }
            }
        }
        let sum = xor_reduce_f32(lane_sums);
        let inverse_rms = 1.0 / (sum / width as f32 + epsilon).sqrt();
        for lane in 0..64 {
            for component in 0..64 {
                let column = lane + component * 64;
                if column < width {
                    let index = row_base + column;
                    let input_value = widen_bf16(input[index]);
                    let normalized_input = if fused_mode {
                        let fused = input_value + widen_bf16(residual[index]);
                        fused_output[index] = narrow_bf16_rne(fused);
                        fused
                    } else {
                        input_value
                    };
                    let normalized = normalized_input * inverse_rms;
                    let weighted = normalized * widen_bf16(weight[column]);
                    normalized_output[index] = narrow_bf16_rne(weighted);
                }
            }
        }
    }
    (fused_output, normalized_output)
}

#[test]
fn pure_mode_consumes_empty_auxiliaries_and_preserves_zero_rows_numerically() {
    for width in [128, 1_024, 4_096] {
        let input = vec![0_u16; width * 2];
        let weight = vec![0x3f80_u16; width];
        let (fused, normalized) = reference_rmsnorm(
            &input,
            &[],
            &weight,
            2,
            width,
            QWEN3_RMSNORM_BEHAVIOR_PURE_V1,
        );
        assert!(fused.is_empty());
        assert_eq!(normalized, input);
    }
}

#[test]
fn pure_uniform_unit_rows_round_back_to_bf16_one() {
    for width in [128, 1_024, 4_096] {
        let input = vec![0x3f80_u16; width];
        let weight = vec![0x3f80_u16; width];
        let (fused, normalized) = reference_rmsnorm(
            &input,
            &[],
            &weight,
            1,
            width,
            QWEN3_RMSNORM_BEHAVIOR_PURE_V1,
        );
        assert!(fused.is_empty());
        assert!(normalized.iter().all(|&value| value == 0x3f80));
    }
}

#[test]
fn fused_mode_stores_bf16_sum_but_normalizes_the_full_f32_sum() {
    for width in [1_024, 4_096] {
        let input = (0..width)
            .map(|index| if index % 2 == 0 { 0x3f80 } else { 0xbf00 })
            .collect::<Vec<_>>();
        let residual = (0..width)
            .map(|index| if index % 3 == 0 { 0x3e80 } else { 0x3f00 })
            .collect::<Vec<_>>();
        let weight = (0..width)
            .map(|index| if index % 5 == 0 { 0x4000 } else { 0x3f80 })
            .collect::<Vec<_>>();
        let (fused, normalized) = reference_rmsnorm(
            &input,
            &residual,
            &weight,
            1,
            width,
            QWEN3_RMSNORM_BEHAVIOR_RESIDUAL_FUSED_V1,
        );
        for index in 0..width {
            assert_eq!(
                fused[index],
                narrow_bf16_rne(widen_bf16(input[index]) + widen_bf16(residual[index]))
            );
        }
        assert_eq!(normalized.len(), width);
        assert!(normalized.iter().any(|&value| value != normalized[0]));
    }
}

#[test]
fn bf16_narrowing_is_round_to_nearest_ties_to_even() {
    assert_eq!(narrow_bf16_rne(f32::from_bits(0x3f80_7fff)), 0x3f80);
    assert_eq!(narrow_bf16_rne(f32::from_bits(0x3f80_8000)), 0x3f80);
    assert_eq!(narrow_bf16_rne(f32::from_bits(0x3f81_8000)), 0x3f82);
    assert_eq!(narrow_bf16_rne(f32::from_bits(0x3f80_8001)), 0x3f81);
    assert_eq!(narrow_bf16_rne(f32::NAN) & 0x7fc0, 0x7fc0);
}
