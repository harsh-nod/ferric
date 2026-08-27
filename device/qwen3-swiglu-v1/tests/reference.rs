use ferric_qwen3_swiglu_device_v1::{
    QWEN3_SWIGLU_ADMITTED_EXTENTS_V1, QWEN3_SWIGLU_ELEMENTS_PER_WORKGROUP_V1,
    QWEN3_SWIGLU_ELEMENTS_PER_WORKITEM_V1, QWEN3_SWIGLU_MAX_GRID_WORKGROUPS_V1,
    qwen3_swiglu_extent_is_admitted_v1,
};

fn widen_bf16(bits: u16) -> f32 {
    f32::from_bits(u32::from(bits) << 16)
}

fn narrow_bf16_rne(value: f32) -> u16 {
    let bits = value.to_bits();
    if bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0 {
        return ((bits >> 16) as u16) | 0x0040;
    }
    let retained_lsb = (bits >> 16) & 1;
    ((bits + 0x7fff + retained_lsb) >> 16) as u16
}

fn reference_swiglu(gate_bits: u16, up_bits: u16) -> Option<u16> {
    let gate = widen_bf16(gate_bits);
    let up = widen_bf16(up_bits);
    if !gate.is_finite() || !up.is_finite() {
        return None;
    }
    let exponent = if gate >= 0.0 {
        (-gate).exp()
    } else {
        gate.exp()
    };
    let denominator = 1.0 + exponent;
    if !exponent.is_finite() || !denominator.is_finite() || denominator <= 0.0 {
        return None;
    }
    let numerator = if gate >= 0.0 { 1.0 } else { exponent };
    let sigmoid = numerator / denominator;
    let silu = gate * sigmoid;
    let product = silu * up;
    if !sigmoid.is_finite() || !silu.is_finite() || !product.is_finite() {
        return None;
    }
    let result = narrow_bf16_rne(product);
    widen_bf16(result).is_finite().then_some(result)
}

#[test]
fn exact_profile_extents_are_closed_and_fit_the_grid_bound() {
    for (index, elements) in QWEN3_SWIGLU_ADMITTED_EXTENTS_V1.iter().copied().enumerate() {
        assert!(qwen3_swiglu_extent_is_admitted_v1(elements));
        if index > 0 {
            assert!(QWEN3_SWIGLU_ADMITTED_EXTENTS_V1[index - 1] < elements);
        }
        assert_eq!(elements % QWEN3_SWIGLU_ELEMENTS_PER_WORKITEM_V1, 0);
        let workgroups = elements.div_ceil(QWEN3_SWIGLU_ELEMENTS_PER_WORKGROUP_V1);
        assert!(workgroups <= QWEN3_SWIGLU_MAX_GRID_WORKGROUPS_V1 as usize);
        assert!(workgroups * QWEN3_SWIGLU_ELEMENTS_PER_WORKGROUP_V1 >= elements);
    }

    for rejected in [0, 1, 3_071, 3_073, 25_165_823, 25_165_825, usize::MAX] {
        assert!(!qwen3_swiglu_extent_is_admitted_v1(rejected));
    }
}

#[test]
fn stable_reference_covers_sign_zero_extremes_and_nonfinite_rejection() {
    let finite_cases = [
        (0x0000, 0x4000),
        (0x8000, 0x4000),
        (0x3f80, 0x3f80),
        (0xbf80, 0x3f80),
        (0x42c8, 0x3f00),
        (0xc2c8, 0x3f00),
        (0x0001, 0x3f80),
        (0x7f7f, 0x0000),
    ];
    for (gate, up) in finite_cases {
        assert!(
            reference_swiglu(gate, up).is_some(),
            "gate={gate:#06x} up={up:#06x}"
        );
    }

    assert_eq!(reference_swiglu(0x0000, 0x4000), Some(0x0000));
    assert_eq!(reference_swiglu(0x8000, 0x4000), Some(0x8000));
    assert_eq!(reference_swiglu(0x7f80, 0x3f80), None);
    assert_eq!(reference_swiglu(0x3f80, 0xff80), None);
    assert_eq!(reference_swiglu(0x7fc1, 0x3f80), None);
}

#[test]
fn bf16_rounding_is_ties_to_even() {
    assert_eq!(narrow_bf16_rne(f32::from_bits(0x3f80_7fff)), 0x3f80);
    assert_eq!(narrow_bf16_rne(f32::from_bits(0x3f80_8000)), 0x3f80);
    assert_eq!(narrow_bf16_rne(f32::from_bits(0x3f81_8000)), 0x3f82);
    assert_eq!(narrow_bf16_rne(f32::from_bits(0x3f80_8001)), 0x3f81);
}
