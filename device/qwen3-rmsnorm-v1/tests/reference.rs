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

fn serial_square_sum(input: &[u16]) -> f32 {
    input.iter().fold(0.0_f32, |sum, bits| {
        let value = widen_bf16(*bits);
        sum + value * value
    })
}

fn legacy_xor_square_sum(input: &[u16]) -> f32 {
    let mut lanes = [0.0_f32; 64];
    for (lane, sum) in lanes.iter_mut().enumerate() {
        for component in 0..64 {
            let column = lane + component * 64;
            if let Some(bits) = input.get(column) {
                let value = widen_bf16(*bits);
                *sum += value * value;
            }
        }
    }
    let mut offset = 32;
    while offset != 0 {
        let previous = lanes;
        for lane in 0..64 {
            lanes[lane] = previous[lane] + previous[lane ^ offset];
        }
        offset >>= 1;
    }
    lanes[0]
}

#[test]
fn reassociation_sensitive_row_pins_authoritative_serial_fp32_order() {
    let input = [
        16295, 16734, 15610, 16917, 16782, 17267, 16001, 15365, 17397, 15627, 16562, 15898, 15809,
        16186, 17041, 16392, 16277, 16725, 15587, 16964, 16853, 17263, 16043, 15361, 17318, 15708,
        16557, 15923, 15850, 16176, 17133, 16399, 16327, 16704, 15535, 16903, 16800, 17182, 16120,
        15441, 17306, 15648, 16523, 15968, 15767, 16218, 17132, 16489, 16317, 16670, 15584, 16991,
        16879, 17154, 16106, 15478, 17362, 15705, 16590, 15906, 15816, 16184, 17040, 16407, 16375,
        16753, 15605, 16970, 16834, 17178, 16127, 15470, 17358, 15622, 16628, 15991, 15869, 16201,
        17112, 16410, 16373, 16696, 15597, 16969, 16793, 17254, 16057, 15418, 17294, 15655, 16639,
        15968, 15798, 16143, 17091, 16496, 16311, 16755, 15562, 16989, 16884, 17254, 16022, 15451,
        17298, 15676, 16621, 15966, 15860, 16137, 17107, 16410, 16317, 16674, 15498, 16900, 16852,
        17177, 16024, 15439, 17370, 15685, 16576, 15983, 15797, 16183, 17030, 16408,
    ];
    assert_eq!(serial_square_sum(&input).to_bits(), 0x49be_1c17);
    assert_eq!(legacy_xor_square_sum(&input).to_bits(), 0x49be_1c1a);
    assert_ne!(
        serial_square_sum(&input).to_bits(),
        legacy_xor_square_sum(&input).to_bits()
    );
}

fn reference_rmsnorm(
    input: &[u16],
    residual: &[u16],
    weight: &[u16],
    rows: usize,
    width: usize,
    behavior: u32,
) -> Option<(Vec<u16>, Vec<u16>)> {
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
        let mut sum = 0.0_f32;
        for column in 0..width {
            let index = row_base + column;
            let input_value = widen_bf16(input[index]);
            if !input_value.is_finite() {
                return None;
            }
            let normalized_input = if fused_mode {
                let residual_value = widen_bf16(residual[index]);
                let fused = input_value + residual_value;
                if !residual_value.is_finite() || !fused.is_finite() {
                    return None;
                }
                fused
            } else {
                input_value
            };
            let square = normalized_input * normalized_input;
            let next_sum = sum + square;
            if !square.is_finite() || !next_sum.is_finite() {
                return None;
            }
            sum = next_sum;
        }
        let mean_square = sum / width as f32;
        let stabilized = mean_square + epsilon;
        let denominator = stabilized.sqrt();
        let inverse_rms = 1.0 / denominator;
        if !mean_square.is_finite()
            || !stabilized.is_finite()
            || stabilized <= 0.0
            || !denominator.is_finite()
            || denominator <= 0.0
            || !inverse_rms.is_finite()
        {
            return None;
        }
        for lane in 0..64 {
            for component in 0..64 {
                let column = lane + component * 64;
                if column < width {
                    let index = row_base + column;
                    let input_value = widen_bf16(input[index]);
                    if !input_value.is_finite() {
                        return None;
                    }
                    let normalized_input = if fused_mode {
                        let residual_value = widen_bf16(residual[index]);
                        let fused = input_value + residual_value;
                        let narrowed = narrow_bf16_rne(fused);
                        if !residual_value.is_finite()
                            || !fused.is_finite()
                            || !widen_bf16(narrowed).is_finite()
                        {
                            return None;
                        }
                        fused_output[index] = narrowed;
                        fused
                    } else {
                        input_value
                    };
                    let normalized = normalized_input * inverse_rms;
                    let weight_value = widen_bf16(weight[column]);
                    let weighted = normalized * weight_value;
                    let narrowed = narrow_bf16_rne(weighted);
                    if !weight_value.is_finite()
                        || !normalized.is_finite()
                        || !weighted.is_finite()
                        || !widen_bf16(narrowed).is_finite()
                    {
                        return None;
                    }
                    normalized_output[index] = narrowed;
                }
            }
        }
    }
    Some((fused_output, normalized_output))
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
        )
        .unwrap();
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
        )
        .unwrap();
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
        )
        .unwrap();
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

#[test]
fn nonfinite_inputs_intermediates_and_narrowing_fail_closed() {
    let finite_input = vec![0x3f80_u16; 128];
    let finite_weight = vec![0x3f80_u16; 128];

    let mut nonfinite_input = finite_input.clone();
    nonfinite_input[17] = 0x7f80;
    assert!(reference_rmsnorm(&nonfinite_input, &[], &finite_weight, 1, 128, 0).is_none());

    let mut nonfinite_weight = finite_weight.clone();
    nonfinite_weight[31] = 0x7fc1;
    assert!(reference_rmsnorm(&finite_input, &[], &nonfinite_weight, 1, 128, 0).is_none());

    let overflow_input = vec![0x7f7f_u16; 128];
    assert!(reference_rmsnorm(&overflow_input, &[], &finite_weight, 1, 128, 0).is_none());

    let fused_input = vec![0x3f80_u16; 1_024];
    let mut nonfinite_residual = vec![0_u16; 1_024];
    nonfinite_residual[257] = 0xff80;
    let fused_weight = vec![0x3f80_u16; 1_024];
    assert!(
        reference_rmsnorm(
            &fused_input,
            &nonfinite_residual,
            &fused_weight,
            1,
            1_024,
            QWEN3_RMSNORM_BEHAVIOR_RESIDUAL_FUSED_V1,
        )
        .is_none()
    );
}
