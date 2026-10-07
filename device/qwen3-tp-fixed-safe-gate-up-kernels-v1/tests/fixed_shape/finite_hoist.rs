use ferric_qwen3_tp_fixed_safe_gate_up_kernels_v1::{host, reference_bf16::Bf16};

fn recorded_bits(value: f32) -> u32 {
    if value.is_nan() { f32::NAN.to_bits() } else { value.to_bits() }
}

fn compare_row(a: &[u16], weights: &[u16]) -> (Vec<host::LaneTrace>, host::RowResult) {
    let packed_a = host::pack_rows(a, 1, host::K).unwrap();
    let packed_weights = host::pack_rows(weights, 1, host::K).unwrap();
    let baseline = (0..host::LANES)
        .map(|lane| host::packed_lane(&packed_a, &packed_weights, lane).unwrap())
        .collect::<Vec<_>>();
    let mut candidate = baseline.clone();
    for (lane, state) in candidate.iter_mut().enumerate() {
        let mut partial = 0.0_f32;
        for group in 0..host::GROUPS {
            let index = group * host::LANES + lane;
            let left = host::unpack_word(packed_a[index]);
            let right = host::unpack_word(packed_weights[index]);
            for half in 0..2 {
                let product = Bf16::from_bits(left[half]).to_f32()
                    * Bf16::from_bits(right[half]).to_f32();
                partial += product;
                let checkpoint = &baseline[lane].steps[group * 2 + half];
                assert_eq!(recorded_bits(product), checkpoint.product_bits);
                assert_eq!(recorded_bits(partial), checkpoint.partial_bits);
            }
        }
        state.partial_bits = recorded_bits(partial);
        state.finite = partial.is_finite();
        assert_eq!(state.partial_bits, baseline[lane].partial_bits);
        assert_eq!(state.finite, baseline[lane].finite, "lane {lane}");
    }
    let result = host::finish_row(&candidate).unwrap();
    assert_eq!(result, host::finish_row(&baseline).unwrap());
    (candidate, result)
}

#[test]
fn every_bf16_pattern_preserves_the_one_step_finite_invariant() {
    let partials = [0.0, -0.0, f32::from_bits(1), -f32::from_bits(1), f32::MAX, -f32::MAX,
        f32::INFINITY, f32::NEG_INFINITY, f32::NAN];
    for bits in 0..=u16::MAX {
        let left = Bf16::from_bits(bits).to_f32();
        for right_bits in [0x0000, 0x8000, 0x0001, 0x3f80, 0xbf80, 0x7f7f, 0x7f80, 0x7fc1] {
            let product = left * Bf16::from_bits(right_bits).to_f32();
            for partial in partials {
                let next = partial + product;
                let sticky = partial.is_finite() & product.is_finite() & next.is_finite();
                assert_eq!(sticky, next.is_finite(), "left {bits:04x}, right {right_bits:04x}");
            }
        }
    }
}

#[test]
fn nonfinite_products_at_early_middle_and_final_steps_keep_rejection() {
    for lane in [0, 7, 63] {
        for step in [0, 1, 31, 32, 62, 63] {
            for (left, right) in [
                (0x7fc1, 0x3f80), (0xff81, 0xbf80), (0x7f80, 0x3f80),
                (0x7f80, 0x0000), (0xff80, 0xbf80), (0x7f7f, 0x4000),
            ] {
                let mut a = vec![0; host::K];
                let mut weights = vec![0x3f80; host::K];
                let inner = step * host::LANES + lane;
                a[inner] = left;
                weights[inner] = right;
                let (states, result) = compare_row(&a, &weights);
                assert!(!states[lane].finite);
                assert!(!result.accepted);
            }
        }
    }
}

#[test]
fn overflow_cannot_be_repaired_by_later_opposite_sign_products() {
    for product_overflow in [false, true] {
        let mut a = vec![0; host::K];
        let mut weights = vec![0x3f80; host::K];
        for (step, bits) in [0x7f7f, 0x7f7f, 0xff7f, 0xff7f].into_iter().enumerate() {
            a[step * host::LANES] = bits;
            if product_overflow {
                weights[step * host::LANES] = 0x4000;
            }
        }
        let (states, result) = compare_row(&a, &weights);
        assert!(!states[0].finite && !result.accepted);
        assert_eq!(states[0].steps.iter().all(|step| f32::from_bits(step.product_bits).is_finite()), !product_overflow);
        assert!(!f32::from_bits(states[0].partial_bits).is_finite());
    }
}

#[test]
fn signed_zero_subnormal_underflow_and_cancellation_keep_exact_bits() {
    for (left, right) in [(0x0000, 0xbf80), (0x8000, 0x3f80), (0x0001, 0x3f80),
        (0x8001, 0x3f80), (0x0080, 0x0001)] {
        let (states, result) = compare_row(&[left; host::K], &[right; host::K]);
        assert!(states.iter().all(|lane| lane.finite) && result.accepted);
    }
    let mut a = vec![0; host::K];
    a[0] = 0x4b80;
    a[64] = 0x3f80;
    a[128] = 0xcb80;
    let (states, result) = compare_row(&a, &[0x3f80; host::K]);
    assert_eq!(states[0].steps[0].partial_bits, 0x4b80_0000);
    assert_eq!(states[0].steps[1].partial_bits, 0x4b80_0000);
    assert_eq!(states[0].steps[2].partial_bits, 0);
    assert_eq!(states[0].partial_bits, 0);
    assert!(result.accepted);
}

#[test]
fn reduction_overflow_and_bf16_narrowing_overflow_still_reject() {
    for (second, sum_is_finite) in [(0x7f7f, false), (0x7b00, true)] {
        let mut a = vec![0; host::K];
        a[0] = 0x7f7f;
        a[1] = second;
        let (states, result) = compare_row(&a, &[0x3f80; host::K]);
        assert!(states.iter().all(|lane| lane.finite));
        assert_eq!(f32::from_bits(result.sums[0]).is_finite(), sum_is_finite);
        assert!(!Bf16::from_bits(result.narrowed[0]).is_finite());
        assert!(!result.accepted);
    }
}
