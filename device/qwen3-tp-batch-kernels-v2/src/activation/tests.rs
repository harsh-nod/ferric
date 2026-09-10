use super::*;

struct HostMath;
impl HostMath {
    fn exp_f32(&self, value: f32) -> f32 {
        value.exp()
    }
}

#[test]
fn stable_swiglu_matches_original_order_for_each_batch_element() {
    for gate in [-100.0_f32, -4.0, -0.0, 0.0, 0.25, 8.0, 100.0] {
        for up in [-2.0_f32, 0.0, 0.75, 3.0] {
            let g = Bf16::from_f32(gate);
            let u = Bf16::from_f32(up);
            let actual = batch_swiglu_v2!(g.to_bits(), u.to_bits(), HostMath);
            let g = g.to_f32();
            let exponent = (if g >= 0.0 { -g } else { g }).exp();
            let sigmoid = (if g >= 0.0 { 1.0 } else { exponent }) / (1.0 + exponent);
            assert_eq!(actual, Bf16::from_f32((g * sigmoid) * u.to_f32()).to_bits());
        }
    }
}

#[test]
#[should_panic]
fn nonfinite_activation_traps() {
    let _ = batch_swiglu_v2!(0x7fc0, 0x3f80, HostMath);
}
