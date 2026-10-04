// Source-visible numerical core shared by launch-index and claimed-task wrappers.
// The caller owns geometry, checked loads, reductions, rejection, and stores.
// Expand the whole kernel item before #[kernel] examines its control flow.
// An expression macro inside an already attributed kernel would be opaque.
macro_rules! qwen_attention_emit_kernel_v1 {
    (([$($header:tt)*] [$($prelude:tt)*] [$($suffix:tt)*]) { $($body:tt)* }) => {
        $($header)* { $($prelude)* { $($body)* }; $($suffix)* }
    };
}

macro_rules! qwen_attention_emit_pair_v1 {
    (() { $($body:tt)* }) => { { $($body)* } };
}

macro_rules! qwen_attention_online_pair_v1 {
    ($emit:ident, $state:tt, $context:expr, $position:expr, $token:ident, $load:block, $math:ident) => {
        $emit! { $state {
        let mut maximum = 0.0_f32;
        let mut denominator = 0.0_f32;
        let mut numerator_0 = 0.0_f32;
        let mut numerator_1 = 0.0_f32;
        let mut finite = true;
        let mut $token = 0_usize;
        while $token < $context {
            // Never load masked tokens: future pages need not be initialized.
            if $token <= $position {
                let (score, value_0, value_1, product_finite) = $load;
                finite &= product_finite;
                finite &= score.is_finite() & value_0.is_finite() & value_1.is_finite();
                if $token == 0 {
                    maximum = score;
                    denominator = 1.0;
                    numerator_0 = value_0;
                    numerator_1 = value_1;
                } else {
                    let next_maximum = if score > maximum { score } else { maximum };
                    let previous_weight = $math.exp_f32(maximum - next_maximum);
                    let current_weight = $math.exp_f32(score - next_maximum);
                    denominator = denominator * previous_weight + current_weight;
                    numerator_0 = numerator_0 * previous_weight + value_0 * current_weight;
                    numerator_1 = numerator_1 * previous_weight + value_1 * current_weight;
                    finite &= previous_weight.is_finite()
                        & (previous_weight >= 0.0)
                        & current_weight.is_finite()
                        & (current_weight >= 0.0)
                        & denominator.is_finite()
                        & (denominator > 0.0)
                        & numerator_0.is_finite()
                        & numerator_1.is_finite();
                    maximum = next_maximum;
                }
            }
            $token += 1;
        }
        let output_0 = numerator_0 / denominator;
        let output_1 = numerator_1 / denominator;
        let narrowed_0 = fe2o3_device::Bf16::from_f32(output_0);
        let narrowed_1 = fe2o3_device::Bf16::from_f32(output_1);
        finite &= output_0.is_finite()
            & output_1.is_finite()
            & narrowed_0.is_finite()
            & narrowed_1.is_finite();
        (narrowed_0.to_bits(), narrowed_1.to_bits(), finite)
        } }
    };
}
