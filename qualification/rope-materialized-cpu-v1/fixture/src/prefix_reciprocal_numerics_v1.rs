// Only the tiled V6 prefix opts into this arithmetic. The shared V3 head
// macros, including their native-division behavior, remain byte-identical.
macro_rules! qwen_prefix_reciprocal_rn_v1 {
    ($denominator:expr) => {{
        let bits = ($denominator).to_bits();
        // sqrt(mean + 1e-6) lies in this admitted positive-normal interval.
        // An unexpected Math result fails closed at each caller's finite check.
        let admitted = (bits.wrapping_sub(0x3a80_0000_u32) <= 0x2500_0000_u32) as u32;
        let mask = 0_u32.wrapping_sub(admitted);
        let exponent = bits >> 23;
        let significand = (bits & 0x007f_ffff_u32) | 0x0080_0000_u32;
        let mut remainder = 0x0080_0000_u32;
        let mut quotient = 0_u32;
        let mut step = 0_u32;
        // 2^(23+i) = quotient * significand + remainder, with remainder <=
        // significand. Equality persists only for significand = 2^23; RNE then
        // carries the final quotient into the exponent, giving the exact power.
        while step < 24_u32 {
            remainder <<= 1;
            let take = (remainder >= significand) as u32;
            remainder = remainder.wrapping_sub(
                significand & 0_u32.wrapping_sub(take),
            );
            quotient = (quotient << 1) | take;
            step = step.wrapping_add(1_u32);
        }
        let twice_remainder = remainder << 1;
        quotient = quotient.wrapping_add(
            ((twice_remainder > significand)
                | ((twice_remainder == significand)
                    & ((quotient & 1_u32) != 0))) as u32,
        );
        // Invalid inputs also take this bounded integer path. Wrapping exponent
        // packing is safe before the final mask replaces their result with NaN.
        let reciprocal_bits = (253_u32.wrapping_sub(exponent) << 23)
            .wrapping_add(quotient.wrapping_sub(0x0080_0000_u32));
        f32::from_bits((reciprocal_bits & mask) | (f32::NAN.to_bits() & !mask))
    }};
}

// Keep this small prefix-specific inverse separate from V3. The AST contract
// checks that its only arithmetic change is the guarded reciprocal above.
macro_rules! qwen_prefix_head_inverse_v6 {
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
        let inverse = qwen_prefix_reciprocal_rn_v1!(denominator);
        valid = valid && denominator.is_finite() && denominator > 0.0 && inverse.is_finite();
        (sum, inverse, valid)
    }};
}

// Deliberately retain the V3 rejection/write order without changing its other
// users. A source contract permits only the inverse macro's name to differ.
macro_rules! qwen_prefix_head_rope_post_v6 {
    ($task:expr, $stabilized:ident, $sqrt:expr) => {{
        let task = &mut *$task;
        let mut head = 0_usize;
        while head < 20 {
            let (_, inverse, inverse_valid) = qwen_prefix_head_inverse_v6!(task, head, $stabilized, $sqrt);
            let (a, b, norm_valid) = qwen_head_weighted_pair_v3!(task, head, inverse);
            let cosine = match task.rotary(0) {
                Some(value) => value,
                None => f32::NAN,
            };
            let sine = match task.rotary(1) {
                Some(value) => value,
                None => f32::NAN,
            };
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
            head += 1;
        }
    }};
}
