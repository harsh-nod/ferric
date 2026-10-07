use super::*;

struct HostMath;
impl HostMath {
    fn exp_f32(&self, value: f32) -> f32 {
        value.exp()
    }
}

#[test]
fn shared_swiglu_formula_matches_independent_stable_reference() {
    let math = HostMath;
    for gate in [-100.0_f32, -7.0, -0.5, -0.0, 0.0, 0.5, 7.0, 100.0] {
        for up in [-2.0_f32, 0.0, 0.5, 3.0] {
            let gate = Bf16::from_f32(gate);
            let up = Bf16::from_f32(up);
            let got = tp_swiglu_element_v1!(gate.to_bits(), up.to_bits(), math);
            let x = f64::from(gate.to_f32());
            let expected = Bf16::from_f32((x / (1.0 + (-x).exp()) * f64::from(up.to_f32())) as f32);
            assert_eq!(got, expected.to_bits());
        }
    }
}

#[test]
#[should_panic]
fn nonfinite_swiglu_input_traps() {
    let math = HostMath;
    let _ = tp_swiglu_element_v1!(Bf16::from_f32(f32::INFINITY).to_bits(), 0, math);
}
