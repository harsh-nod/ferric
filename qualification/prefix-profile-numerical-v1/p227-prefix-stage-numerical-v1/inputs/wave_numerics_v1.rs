// One textual numerical body for ordinary wrappers and claimed-task handlers.
// Loads are caller expressions; no launch/witness or mutable storage is issued.
#[allow(unused_macros)]
macro_rules! qwen_wave_square_sum_v1 {
    ($lane:expr, $column:ident, $load:expr, $subgroup:expr) => {{
        let mut partial = 0.0_f32;
        let mut finite = true;
        let mut component = 0_usize;
        while component < 64 {
            let $column = $lane + component * 64;
            let input = $load;
            let input_value = input.to_f32();
            let square = input_value * input_value;
            let next_sum = partial + square;
            finite &= input.is_finite() & square.is_finite() & next_sum.is_finite();
            partial = next_sum;
            component += 1;
        }
        let sum = $subgroup.reduce_sum_f32::<64>(partial);
        // Share lane zero's exact result bits, including exceptional values.
        let sum = $subgroup.broadcast_f32::<64>(sum, 0);
        let invalid = if finite { 0.0_f32 } else { 1.0_f32 };
        let any_invalid = $subgroup.reduce_max_f32::<64>(invalid);
        let any_invalid = $subgroup.broadcast_f32::<64>(any_invalid, 0);
        (sum, any_invalid)
    }};
}

#[allow(unused_macros)]
macro_rules! qwen_norm_narrow_v1 {
    ($input:expr, $inverse:expr) => {{
        let normalized = $input * $inverse;
        let narrowed = fe2o3_device::Bf16::from_f32(normalized);
        (normalized, narrowed)
    }};
}

#[allow(unused_macros)]
macro_rules! qwen_wave_dot_v1 {
    ($lane:expr, $inner:ident, $left:expr, $right:expr, $subgroup:expr) => {{
        let mut partial = 0.0_f32;
        let mut finite = true;
        let mut step = 0_usize;
        while step < 64 {
            let $inner = step * 64 + $lane;
            let left = $left;
            let right = $right;
            let product = left * right;
            partial += product;
            finite &= product.is_finite() & partial.is_finite();
            step += 1;
        }
        let sum = $subgroup.reduce_sum_f32::<64>(partial);
        let sum = $subgroup.broadcast_f32::<64>(sum, 0);
        let narrowed = fe2o3_device::Bf16::from_f32(sum);
        (sum, narrowed, finite)
    }};
}
