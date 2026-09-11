use super::*;

#[test]
fn real_reduction_widths_match_independent_dyadic_fp64_and_masked_rows() {
    for k in [1024_usize, 2048, 3072] {
        for rows in [1_usize, 3, 5, 16, 17, 31, 32] {
            let a: std::vec::Vec<_> = (0..rows * k)
                .map(|i| Bf16::from_f32(((i * 17 % 37) as f32 - 18.0) / 32.0).to_bits())
                .collect();
            let weights: std::vec::Vec<_> = (0..3 * k)
                .map(|i| Bf16::from_f32(((i * 11 % 43) as f32 - 21.0) / 64.0).to_bits())
                .collect();
            for column in 0..3 {
                for base in (0..32).step_by(4) {
                    let (s0, s1, s2, s3) =
                        batch_four_dots_v10!(&a, &weights, rows, base, column, k);
                    for (offset, actual) in [s0, s1, s2, s3].into_iter().enumerate() {
                        let row = base + offset;
                        let mut expected = 0_f64;
                        if row < rows {
                            for inner in 0..k {
                                expected +=
                                    f64::from(f32::from_bits(u32::from(a[row * k + inner]) << 16))
                                        * f64::from(f32::from_bits(
                                            u32::from(weights[column * k + inner]) << 16,
                                        ));
                            }
                        }
                        assert_eq!(f64::from(actual), expected);
                    }
                }
            }
        }
    }
}

struct Writer(Option<u16>);
impl Writer {
    #[allow(clippy::too_many_arguments)]
    fn write_tiled_2d(
        &mut self,
        _tile: &(),
        _component: usize,
        _rows: usize,
        _n: usize,
        _stride: usize,
        value: u16,
    ) -> bool {
        self.0 = Some(value);
        true
    }
}

#[test]
fn narrowing_checks_and_nonfinite_dot_checks_are_not_disabled() {
    for value in [
        0_f32,
        -0.0,
        1.0,
        1.0 + 1.0 / 256.0,
        1.0 + 3.0 / 256.0,
        -3.25,
    ] {
        let mut output = Writer(None);
        batch_write_bf16_v10!(output, (), 0, 1, 1, value);
        let bits = value.to_bits();
        let expected = (bits.wrapping_add(0x7fff + ((bits >> 16) & 1)) >> 16) as u16;
        assert_eq!(output.0, Some(expected));
    }
    for k in [1024_usize, 2048, 3072] {
        for (left, right) in [(f32::NAN, 1.0), (1.0, f32::INFINITY), (f32::MAX, 2.0)] {
            let a = std::vec![Bf16::from_f32(left).to_bits();k];
            let w = std::vec![Bf16::from_f32(right).to_bits();k];
            assert!(std::panic::catch_unwind(|| batch_four_dots_v10!(&a, &w, 1, 0, 0, k)).is_err());
        }
    }
    for value in [f32::NAN, f32::INFINITY, f32::MAX] {
        assert!(
            std::panic::catch_unwind(|| {
                let mut output = Writer(None);
                batch_write_bf16_v10!(output, (), 0, 1, 1, value);
            })
            .is_err()
        );
    }
}
