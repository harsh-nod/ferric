// Shared source for the claimed FP32 output projection and CPU numerical tests.
// Loads are caller expressions; these macros grant no storage or launch authority.
#[allow(unused_macros)]
macro_rules! qwen_wave_output_partial_f32_v1 {
    ($lane:expr, $inner:ident, $left:expr, $right:expr, $subgroup:expr) => {{
        let mut partial = 0.0_f32;
        let mut finite = true;
        let mut step = 0_usize;
        while step < 32 {
            let $inner = step * 64 + $lane;
            let left = $left;
            let right = $right;
            let product = left * right;
            partial += product;
            finite &= product.is_finite() & partial.is_finite();
            step += 1;
        }
        // One six-level XOR reduction, with no BF16 narrowing or broadcast.
        let sum = $subgroup.reduce_sum_f32::<64>(partial);
        (sum, finite)
    }};
}

#[allow(unused_macros)]
macro_rules! qwen_claimed_output_projection_v5 {
    ($task:ident, $subgroup:ident) => {{
        let lane = $task.lane();
        let mut column = 0;
        while column < 4096 {
            // Rejected lanes still execute every reduction. The outer worker
            // gathers failure before publishing completion of this task.
            let (sum, finite) = qwen_wave_output_partial_f32_v1!(
                lane,
                inner,
                fe2o3_device::Bf16::from_bits(match $task.input(inner) {
                    Some(value) => value,
                    None => 0x7fc0,
                })
                .to_f32(),
                fe2o3_device::Bf16::from_bits(match $task.weight(column, inner) {
                    Some(value) => value,
                    None => 0x7fc0,
                })
                .to_f32(),
                $subgroup
            );
            if !finite || !sum.is_finite() {
                $task.reject();
            } else if lane == 0 && !$task.write_output(column, sum) {
                $task.reject();
            }
            column += 1;
        }
    }};
}
