// Candidate-only RoPE arithmetic. The original V3/V6 macros stay unchanged.
// This matches BF16 tensor boundaries, not the framework's table generation.
macro_rules! qwen_prefix_rope_materialized_pair_v1 {
    ($a:expr, $b:expr, $cosine:expr, $sine:expr) => {{
        let a = $a;
        let b = $b;
        let cosine = $cosine;
        let sine = $sine;
        let cosine_bf16 = Bf16::from_f32(cosine);
        let sine_bf16 = Bf16::from_f32(sine);
        // rotate_half negates the BF16 high half before multiplying. In
        // particular, adding its negative-zero product is not subtraction.
        let negative_b = Bf16::from_bits(b.to_bits() ^ 0x8000);
        let ac = a.to_f32() * cosine_bf16.to_f32();
        let negative_bs = negative_b.to_f32() * sine_bf16.to_f32();
        let bc = b.to_f32() * cosine_bf16.to_f32();
        let asine = a.to_f32() * sine_bf16.to_f32();
        let ac_bf16 = Bf16::from_f32(ac);
        let negative_bs_bf16 = Bf16::from_f32(negative_bs);
        let bc_bf16 = Bf16::from_f32(bc);
        let asine_bf16 = Bf16::from_f32(asine);
        let first = ac_bf16.to_f32() + negative_bs_bf16.to_f32();
        let second = bc_bf16.to_f32() + asine_bf16.to_f32();
        let rotated_low = Bf16::from_f32(first);
        let rotated_high = Bf16::from_f32(second);
        let valid = a.is_finite()
            & b.is_finite()
            & cosine.is_finite()
            & sine.is_finite()
            & cosine_bf16.is_finite()
            & sine_bf16.is_finite()
            & ac.is_finite()
            & negative_bs.is_finite()
            & bc.is_finite()
            & asine.is_finite()
            & ac_bf16.is_finite()
            & negative_bs_bf16.is_finite()
            & bc_bf16.is_finite()
            & asine_bf16.is_finite()
            & first.is_finite()
            & second.is_finite()
            & rotated_low.is_finite()
            & rotated_high.is_finite();
        (rotated_low, rotated_high, valid)
    }};
}

macro_rules! qwen_prefix_head_rope_materialized_post_v1 {
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
            let (rotated_low, rotated_high, rotation_valid) =
                qwen_prefix_rope_materialized_pair_v1!(a, b, cosine, sine);
            let valid = inverse_valid && norm_valid && rotation_valid;
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
