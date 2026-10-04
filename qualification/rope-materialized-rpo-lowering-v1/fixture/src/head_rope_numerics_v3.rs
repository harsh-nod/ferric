// Shared source arithmetic for the GPU post task and its CPU differential
// harness. Only square root is supplied by the caller: authenticated device
// Math on AMDGPU, native f32 sqrt in explicitly CPU-only tests.
macro_rules! qwen_head_inverse_v3 {
    ($task:expr, $head:expr, $stabilized:ident, $sqrt:expr) => {{
        let mut sum = 0.0_f32;
        let mut valid = true;
        let mut column = 0_usize;
        while column < 128 {
            let bits = match $task.head_input($head, column) {
                Some(value) => value,
                None => 0x7fc0,
            };
            let input = Bf16::from_bits(bits);
            let value = input.to_f32();
            let square = value * value;
            sum = sum + square;
            valid = valid && input.is_finite() && square.is_finite();
            column += 1;
        }
        let mean = sum / 128.0_f32;
        let $stabilized = mean + 1e-6_f32;
        valid = valid
            && sum.is_finite()
            && mean.is_finite()
            && $stabilized.is_finite()
            && $stabilized > 0.0;
        let denominator = if valid { $sqrt } else { 1.0_f32 };
        let inverse = 1.0_f32 / denominator;
        valid = valid && denominator.is_finite() && denominator > 0.0 && inverse.is_finite();
        (sum, inverse, valid)
    }};
}

macro_rules! qwen_head_weighted_pair_v3 {
    ($task:expr, $head:expr, $inverse:expr) => {{
        let lane = $task.lane();
        let low = Bf16::from_bits(match $task.head_input($head, lane) {
            Some(value) => value,
            None => 0x7fc0,
        });
        let high = Bf16::from_bits(match $task.head_input($head, lane + 64) {
            Some(value) => value,
            None => 0x7fc0,
        });
        let weight_low = Bf16::from_bits(match $task.head_weight($head, lane) {
            Some(value) => value,
            None => 0x7fc0,
        });
        let weight_high = Bf16::from_bits(match $task.head_weight($head, lane + 64) {
            Some(value) => value,
            None => 0x7fc0,
        });
        // Keep both BF16 narrowing points around the learned norm weights.
        let scaled_low = low.to_f32() * $inverse;
        let scaled_high = high.to_f32() * $inverse;
        let norm_low = Bf16::from_f32(scaled_low);
        let norm_high = Bf16::from_f32(scaled_high);
        let weighted_low = norm_low.to_f32() * weight_low.to_f32();
        let weighted_high = norm_high.to_f32() * weight_high.to_f32();
        let a = Bf16::from_f32(weighted_low);
        let b = Bf16::from_f32(weighted_high);
        let valid = low.is_finite()
            && high.is_finite()
            && weight_low.is_finite()
            && weight_high.is_finite()
            && scaled_low.is_finite()
            && scaled_high.is_finite()
            && norm_low.is_finite()
            && norm_high.is_finite()
            && weighted_low.is_finite()
            && weighted_high.is_finite()
            && a.is_finite()
            && b.is_finite();
        (a, b, valid)
    }};
}

macro_rules! qwen_head_rope_post_v3 {
    ($task:expr, $stabilized:ident, $sqrt:expr) => {{
        let task = &mut *$task;
        let mut head = 0_usize;
        while head < 20 {
            let (_, inverse, inverse_valid) = qwen_head_inverse_v3!(task, head, $stabilized, $sqrt);
            let (a, b, norm_valid) = qwen_head_weighted_pair_v3!(task, head, inverse);
            let cosine = match task.rotary(0) {
                Some(value) => value,
                None => f32::NAN,
            };
            let sine = match task.rotary(1) {
                Some(value) => value,
                None => f32::NAN,
            };
            // Split-half pairs are lane and lane+64, not adjacent columns.
            // Separate operations preserve the existing no-contraction policy.
            let ac = a.to_f32() * cosine;
            let bs = b.to_f32() * sine;
            let bc = b.to_f32() * cosine;
            let asine = a.to_f32() * sine;
            let first = ac - bs;
            let second = bc + asine;
            let rotated_low = Bf16::from_f32(first);
            let rotated_high = Bf16::from_f32(second);
            let valid = inverse_valid
                && norm_valid
                && cosine.is_finite()
                && sine.is_finite()
                && ac.is_finite()
                && bs.is_finite()
                && bc.is_finite()
                && asine.is_finite()
                && first.is_finite()
                && second.is_finite()
                && rotated_low.is_finite()
                && rotated_high.is_finite();
            if !valid {
                task.reject();
            } else if head < 16 {
                if !task.write_query_head(head, rotated_low.to_bits(), rotated_high.to_bits()) {
                    task.reject();
                }
            } else {
                let kv_head = head - 16;
                let value_low = Bf16::from_bits(match task.value(kv_head, 0) {
                    Some(value) => value,
                    None => 0x7fc0,
                });
                let value_high = Bf16::from_bits(match task.value(kv_head, 1) {
                    Some(value) => value,
                    None => 0x7fc0,
                });
                if !value_low.is_finite()
                    || !value_high.is_finite()
                    || !task.write_key_value_head(
                        kv_head,
                        rotated_low.to_bits(),
                        rotated_high.to_bits(),
                        value_low.to_bits(),
                        value_high.to_bits(),
                    )
                {
                    task.reject();
                }
            }
            // Rejected lanes still reach the enclosing worker's finish exchange.
            head += 1;
        }
    }};
}
