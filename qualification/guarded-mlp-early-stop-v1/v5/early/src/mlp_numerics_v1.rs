// Source-visible arithmetic shared by the claimed MLP worker and CPU tests.
// These macros confer no storage, scheduling or launch authority. Host math
// stand-ins in tests are not evidence of the GPU sqrt/OCML implementation.
#[allow(unused_macros)]
macro_rules! qwen_claimed_mlp_norm_v1 {
    ($task:ident, $subgroup:ident, $stabilized:ident, $sqrt:expr) => {{
        let lane = $task.lane();
        let (sum, invalid) = qwen_wave_square_sum_v1!(
            lane,
            column,
            fe2o3_device::Bf16::from_bits(match $task.input(column) {
                Some(value) => value,
                None => 0x7fc0,
            }),
            $subgroup
        );
        // This branch is uniform because the invalid vote and sum were both
        // broadcast. Even a rejected task returns to the round finish exchange.
        if invalid != 0.0 || !sum.is_finite() {
            $task.reject();
        } else {
            let mean = sum / 4096.0_f32;
            let $stabilized = mean + 1e-6_f32;
            if !mean.is_finite() || !$stabilized.is_finite() || $stabilized <= 0.0 {
                $task.reject();
            } else {
                let denominator = $sqrt;
                if !denominator.is_finite() || denominator <= 0.0 {
                    $task.reject();
                } else {
                    let inverse = 1.0_f32 / denominator;
                    if !inverse.is_finite() {
                        $task.reject();
                    } else {
                        let mut component = 0;
                        while component < 64 {
                            let column = lane + 64 * component;
                            let input = fe2o3_device::Bf16::from_bits(match $task.input(column) {
                                Some(value) => value,
                                None => 0x7fc0,
                            });
                            let weight =
                                fe2o3_device::Bf16::from_bits(match $task.weight(column) {
                                    Some(value) => value,
                                    None => 0x7fc0,
                                });
                            let (normalized, narrowed) =
                                qwen_norm_narrow_v1!(input.to_f32(), inverse);
                            let weighted = narrowed.to_f32() * weight.to_f32();
                            let output = fe2o3_device::Bf16::from_f32(weighted);
                            if !input.is_finite()
                                || !weight.is_finite()
                                || !normalized.is_finite()
                                || !narrowed.is_finite()
                                || !weighted.is_finite()
                                || !output.is_finite()
                                || !$task.write_component(component, output.to_bits())
                            {
                                $task.reject();
                            }
                            component += 1;
                        }
                    }
                }
            }
        }
    }};
}

#[allow(unused_macros)]
macro_rules! qwen_claimed_mlp_projection_v1 {
    ($task:ident, $subgroup:ident) => {{
        let lane = $task.lane();
        let mut row = 0;
        while row < 6144 {
            // Rejecting a lane never skips a later row's subgroup operations.
            let (sum, narrowed, finite) = qwen_wave_dot_v1!(
                lane,
                inner,
                fe2o3_device::Bf16::from_bits(match $task.input(inner) {
                    Some(value) => value,
                    None => 0x7fc0,
                })
                .to_f32(),
                fe2o3_device::Bf16::from_bits(match $task.weight(row, inner) {
                    Some(value) => value,
                    None => 0x7fc0,
                })
                .to_f32(),
                $subgroup
            );
            if !finite || !sum.is_finite() || !narrowed.is_finite() {
                $task.reject();
            } else if lane == 0 && !$task.write_column(row, narrowed.to_bits()) {
                $task.reject();
            }
            row += 1;
        }
    }};
}

#[allow(unused_macros)]
macro_rules! qwen_claimed_mlp_swiglu_v1 {
    ($task:ident, $math:ident) => {{
        let lane = $task.lane();
        let mut component = 0;
        while component < 96 {
            let column = lane + 64 * component;
            let gate = fe2o3_device::Bf16::from_bits(match $task.gate(column) {
                Some(value) => value,
                None => 0x7fc0,
            });
            let up = fe2o3_device::Bf16::from_bits(match $task.up(column) {
                Some(value) => value,
                None => 0x7fc0,
            });
            let g = gate.to_f32();
            let u = up.to_f32();
            let exponential = $math.exp_f32(-g.abs());
            let numerator = if g >= 0.0 { 1.0_f32 } else { exponential };
            let sigmoid = numerator / (1.0_f32 + exponential);
            let silu = g * sigmoid;
            let product = silu * u;
            let output = fe2o3_device::Bf16::from_f32(product);
            if !gate.is_finite()
                || !up.is_finite()
                || !exponential.is_finite()
                || !sigmoid.is_finite()
                || !silu.is_finite()
                || !product.is_finite()
                || !output.is_finite()
                || !$task.write_component(component, output.to_bits())
            {
                $task.reject();
            }
            component += 1;
        }
    }};
}

#[allow(unused_macros)]
macro_rules! qwen_wave_mlp_down_partial_v1 {
    ($lane:expr, $inner:ident, $left:expr, $right:expr, $subgroup:expr) => {{
        let mut partial = 0.0_f32;
        let mut finite = true;
        let mut step = 0_usize;
        // Exactly the 96 active iterations of the queued 192-step/inner<6144
        // loop. Removing its inactive tail does not reorder any arithmetic.
        while step < 96 {
            let $inner = step * 64 + $lane;
            let left = $left;
            let right = $right;
            let product = left * right;
            partial += product;
            finite &= product.is_finite() & partial.is_finite();
            step += 1;
        }
        let sum = $subgroup.reduce_sum_f32::<64>(partial);
        (sum, finite)
    }};
}

#[allow(unused_macros)]
macro_rules! qwen_claimed_mlp_down_v1 {
    ($task:ident, $subgroup:ident) => {{
        let lane = $task.lane();
        let mut row = 0;
        while row < 4096 {
            let (sum, finite) = qwen_wave_mlp_down_partial_v1!(
                lane,
                inner,
                fe2o3_device::Bf16::from_bits(match $task.input(inner) {
                    Some(value) => value,
                    None => 0x7fc0,
                })
                .to_f32(),
                fe2o3_device::Bf16::from_bits(match $task.weight(row, inner) {
                    Some(value) => value,
                    None => 0x7fc0,
                })
                .to_f32(),
                $subgroup
            );
            if !finite || !sum.is_finite() {
                $task.reject();
            } else if lane == 0 && !$task.write_output(row, sum) {
                $task.reject();
            }
            row += 1;
        }
    }};
}
