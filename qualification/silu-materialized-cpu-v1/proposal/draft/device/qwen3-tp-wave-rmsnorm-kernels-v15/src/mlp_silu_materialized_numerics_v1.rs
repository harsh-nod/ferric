// Explicit experimental BF16 SiLU boundary. The original fused macro remains
// unchanged; only the separately selected candidate entry invokes this body.
#[allow(unused_macros)]
macro_rules! qwen_claimed_mlp_swiglu_materialized_v1 {
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
            let materialized_silu = fe2o3_device::Bf16::from_f32(silu);
            let product = materialized_silu.to_f32() * u;
            let output = fe2o3_device::Bf16::from_f32(product);
            if !gate.is_finite()
                || !up.is_finite()
                || !exponential.is_finite()
                || !sigmoid.is_finite()
                || !silu.is_finite()
                || !materialized_silu.is_finite()
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
