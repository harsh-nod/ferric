use fe2o3_device::{Bf16, StridedReadView2D};

#[derive(Debug, PartialEq, Eq)]
enum Rejection {
    Lane,
    Reduction,
    Narrowing,
}

// Arithmetic reference only; this does not emulate a device capability or prove parity.
fn wave_dot(a: &[u16], weights: &[u16], k: usize, column: usize) -> Result<f32, Rejection> {
    let left_view = StridedReadView2D::from_shared_slice(a, 0, 1, k, k).unwrap();
    let right_view =
        StridedReadView2D::from_shared_slice(weights, 0, weights.len() / k, k, k).unwrap();
    let mut partials = [0.0_f32; 64];
    let mut invalid = [false; 64];
    for lane in 0..64 {
        let mut partial = 0.0_f32;
        let mut finite = true;
        for step in 0..48 {
            let inner = step * 64 + lane;
            if inner < k {
                let left = Bf16::from_bits(left_view.load_or(0, inner, 0x7fc0)).to_f32();
                let right = Bf16::from_bits(right_view.load_or(column, inner, 0x7fc0)).to_f32();
                let product = left * right;
                partial += product;
                finite &= product.is_finite() & partial.is_finite();
            }
        }
        partials[lane] = partial;
        invalid[lane] = !finite;
    }
    // The six XOR levels used by the existing wave-reference tests.
    for mask in [1, 2, 4, 8, 16, 32] {
        let before = partials;
        for lane in 0..64 {
            partials[lane] = before[lane] + before[lane ^ mask];
        }
    }
    if invalid.into_iter().any(|value| value) {
        return Err(Rejection::Lane);
    }
    if !partials[0].is_finite() {
        return Err(Rejection::Reduction);
    }
    Ok(partials[0])
}

fn narrow(sum: f32) -> Result<u16, Rejection> {
    let value = Bf16::from_f32(sum);
    if !value.is_finite() {
        Err(Rejection::Narrowing)
    } else {
        Ok(value.to_bits())
    }
}

fn scalar_dot(a: &[u16], weights: &[u16]) -> f32 {
    let mut sum = 0.0_f32;
    for (&left, &right) in a.iter().zip(weights) {
        let product = Bf16::from_bits(left).to_f32() * Bf16::from_bits(right).to_f32();
        sum += product;
    }
    sum
}

#[test]
fn every_admitted_reduction_element_has_one_lane_owner() {
    for k in [1024_usize, 2048, 3072] {
        let mut owners = vec![0_u8; k];
        for lane in 0..64 {
            for step in 0..48 {
                let inner = step * 64 + lane;
                if inner < k {
                    owners[inner] += 1;
                }
            }
        }
        assert!(owners.into_iter().all(|count| count == 1));
    }
}

#[test]
fn signed_dyadic_dots_match_independent_fp64_reference_at_real_widths() {
    for k in [1024_usize, 2048, 3072] {
        let a: Vec<_> = (0..k)
            .map(|i| Bf16::from_f32(((i * 17 % 37) as f32 - 18.0) / 32.0).to_bits())
            .collect();
        let weights: Vec<_> = (0..3 * k)
            .map(|i| Bf16::from_f32(((i * 11 % 43) as f32 - 21.0) / 64.0).to_bits())
            .collect();
        for column in 0..3 {
            let expected: f64 = (0..k)
                .map(|inner| {
                    f64::from(f32::from_bits(u32::from(a[inner]) << 16))
                        * f64::from(f32::from_bits(u32::from(weights[column * k + inner]) << 16))
                })
                .sum();
            let actual = wave_dot(&a, &weights, k, column).unwrap();
            assert_eq!(f64::from(actual), expected);
            assert_eq!(
                actual,
                scalar_dot(&a, &weights[column * k..(column + 1) * k])
            );
        }
    }
}

#[test]
fn exponent_mixed_fixture_error_is_bounded_against_fp64_not_scalar_identity() {
    let mut state = 0x9e37_79b9_u32;
    for k in [1024_usize, 2048, 3072] {
        for _ in 0..8 {
            let mut next = || {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                let sign = (state >> 16) as u16 & 0x8000;
                let exponent = (118 + (state % 19) as u16) << 7;
                sign | exponent | (state as u16 & 0x007f)
            };
            let a: Vec<_> = (0..k).map(|_| next()).collect();
            let weights: Vec<_> = (0..k).map(|_| next()).collect();
            let products: Vec<_> = a
                .iter()
                .zip(&weights)
                .map(|(&a, &b)| {
                    f64::from(f32::from_bits(u32::from(a) << 16))
                        * f64::from(f32::from_bits(u32::from(b) << 16))
                })
                .collect();
            let expected: f64 = products.iter().sum();
            let absolute_sum: f64 = products.iter().map(|value| value.abs()).sum();
            let actual = f64::from(wave_dot(&a, &weights, k, 0).unwrap());
            // Fixture-only bound for <=48 local additions and six reduction levels.
            let bound = 64.0 * f64::from(f32::EPSILON) * absolute_sum;
            assert!((actual - expected).abs() <= bound);
        }
    }
}

#[test]
fn changed_association_is_observable_and_not_claimed_bitwise_equivalent() {
    let k = 1024;
    let mut a = vec![Bf16::from_f32(2.0_f32.powi(-19)).to_bits(); k];
    a[0] = Bf16::from_f32(256.0).to_bits();
    let weights = vec![Bf16::from_f32(1.0).to_bits(); k];
    let scalar = scalar_dot(&a, &weights);
    let wave = wave_dot(&a, &weights, k, 0).unwrap();
    assert_eq!(scalar, 256.0);
    assert!(wave > scalar);
    assert_ne!(wave.to_bits(), scalar.to_bits());
}

#[test]
fn every_lane_rejects_nonfinite_operands_and_product_overflow() {
    for k in [1024_usize, 2048, 3072] {
        let mut a = vec![Bf16::from_f32(1.0).to_bits(); k];
        let mut weights = a.clone();
        for lane in 0..64 {
            for inner in [lane, k - 64 + lane] {
                for bits in [0x7fc0, 0x7f81, 0xffc0, 0x7f80, 0xff80] {
                    a[inner] = bits;
                    assert_eq!(wave_dot(&a, &weights, k, 0), Err(Rejection::Lane));
                    a[inner] = 0x3f80;
                    weights[inner] = bits;
                    assert_eq!(wave_dot(&a, &weights, k, 0), Err(Rejection::Lane));
                    weights[inner] = 0x3f80;
                }
                a[inner] = 0x7f7f;
                weights[inner] = 0x4000;
                assert_eq!(wave_dot(&a, &weights, k, 0), Err(Rejection::Lane));
                a[inner] = 0x3f80;
                weights[inner] = 0x3f80;
            }
        }
    }
}

#[test]
fn lane_overflow_reduction_overflow_and_bf16_narrowing_are_distinct_failures() {
    let k = 1024;
    let mut a = vec![0_u16; k];
    let weights = vec![0x3f80_u16; k];
    a[0] = 0x7f7f;
    a[64] = 0x7f7f;
    assert_eq!(wave_dot(&a, &weights, k, 0), Err(Rejection::Lane));
    a[64] = 0;
    a[1] = 0x7f7f;
    assert_eq!(wave_dot(&a, &weights, k, 0), Err(Rejection::Reduction));
    a[1] = Bf16::from_f32(2.0_f32.powi(119)).to_bits();
    let sum = wave_dot(&a, &weights, k, 0).unwrap();
    assert!(sum.is_finite());
    assert_eq!(narrow(sum), Err(Rejection::Narrowing));
}

#[test]
fn guarded_read_fallback_rejects_instead_of_silently_using_zero() {
    let k = 1024;
    let a = vec![0x3f80_u16; k];
    let weights = a.clone();
    assert_eq!(wave_dot(&a, &weights, k, 1), Err(Rejection::Lane));
}

#[test]
fn single_row_ignores_poisoned_capacity_and_preserves_input_bits() {
    for k in [1024_usize, 2048, 3072] {
        let mut a = vec![0x7fc0_u16; 32 * k];
        a[..k].fill(0x3f80);
        let weights = vec![0x3f80_u16; k];
        let before = a.clone();
        assert_eq!(wave_dot(&a, &weights, k, 0), Ok(k as f32));
        assert_eq!(a, before);
    }
}

#[test]
fn bf16_rounding_uses_even_ties_but_fp32_head_does_not_narrow() {
    for (value, expected) in [
        (1.0, 0x3f80),
        (1.0 + 1.0 / 256.0, 0x3f80),
        (1.0 + 3.0 / 256.0, 0x3f82),
        (-1.0 - 1.0 / 256.0, 0xbf80),
    ] {
        assert_eq!(narrow(value), Ok(expected));
    }
    let k = 1024;
    let mut a = vec![0_u16; k];
    let weights = vec![0x3f80_u16; k];
    a[0] = 0x3f80;
    a[1] = Bf16::from_f32(1.0 / 256.0).to_bits();
    let head = wave_dot(&a, &weights, k, 0).unwrap();
    assert_eq!(head, 1.0 + 1.0 / 256.0);
    assert_ne!(head.to_bits(), u32::from(narrow(head).unwrap()) << 16);
}
