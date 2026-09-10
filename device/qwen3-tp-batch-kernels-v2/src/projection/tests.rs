use super::*;
use std::vec;

#[test]
fn exact_target_projection_shapes_cover_every_rank_width() {
    for world in [1_u32, 2, 8] {
        for (projection, n) in [
            (1, 4096 / world),
            (2, 1024 / world),
            (3, 1024 / world),
            (4, 12288 / world),
            (5, 12288 / world),
            (6, 151936),
        ] {
            assert!(batch_column_shape_v2!(n, 4096, world, projection));
            assert!(!batch_column_shape_v2!(n + 1, 4096, world, projection));
            assert!(!batch_column_shape_v2!(n, 1024, world, projection));
        }
        assert!(batch_partial_shape_v2!(4096, 4096 / world, world, 1));
        assert!(batch_partial_shape_v2!(4096, 12288 / world, world, 2));
        assert!(!batch_partial_shape_v2!(4095, 4096 / world, world, 1));
    }
    for world in [0, 3, 4, 16] {
        assert!(!batch_column_shape_v2!(151936, 4096, world, 6));
        assert!(!batch_partial_shape_v2!(4096, 4096, world, 1));
    }
}

#[test]
fn tiled_four_row_accumulation_matches_independent_ascending_dots() {
    let k = 37_usize;
    let columns = 7_usize;
    for rows in 1..=16 {
        let mut a = vec![0_u16; 16 * k];
        let mut weights = vec![0_u16; columns * k];
        for row in 0..rows {
            for inner in 0..k {
                a[row * k + inner] =
                    Bf16::from_f32(((row * 13 + inner * 7) % 31) as f32 / 16.0 - 1.0).to_bits();
            }
        }
        a[rows * k..].fill(Bf16::from_f32(f32::NAN).to_bits());
        for (index, value) in weights.iter_mut().enumerate() {
            *value = Bf16::from_f32((index % 17) as f32 / 8.0 - 1.0).to_bits();
        }
        for row_base in [0, 4, 8, 12] {
            for column in 0..columns {
                let sums = batch_four_dots_v2!(&a, &weights, rows, row_base, column, k);
                for (offset, actual) in [sums.0, sums.1, sums.2, sums.3].into_iter().enumerate() {
                    let row = row_base + offset;
                    let mut expected = 0.0_f32;
                    if row < rows {
                        for inner in 0..k {
                            expected += Bf16::from_bits(a[row * k + inner]).to_f32()
                                * Bf16::from_bits(weights[column * k + inner]).to_f32();
                        }
                    }
                    assert_eq!(actual.to_bits(), expected.to_bits());
                }
            }
        }
    }
}

#[test]
fn partial_outputs_preserve_information_before_host_reduction() {
    let a = [0x3f80, 0x3b80, 0x3f80, 0x3b80];
    let w = [0x3f80, 0x3f80];
    let sums = batch_four_dots_v2!(&a, &w, 2, 0, 0, 2);
    assert_eq!(sums.0, 1.0 + 1.0 / 256.0);
    assert_eq!(sums.1, 1.0 + 1.0 / 256.0);
    assert_ne!(sums.0, Bf16::from_f32(sums.0).to_f32());
}

#[test]
fn bf16_output_macro_performs_one_final_narrowing() {
    struct Capture(u16);
    impl Capture {
        fn write_tiled_2d(
            &mut self,
            _: &(),
            component: usize,
            rows: usize,
            n: usize,
            stride: usize,
            value: u16,
        ) -> bool {
            assert_eq!((component, rows, n, stride), (0, 1, 1, 1));
            self.0 = value;
            true
        }
    }
    let tile = ();
    let mut output = Capture(0);
    batch_write_bf16_v2!(output, tile, 0, 1, 1, 1.0 + 1.0 / 256.0);
    assert_eq!(output.0, Bf16::from_f32(1.0 + 1.0 / 256.0).to_bits());
}

#[test]
#[should_panic]
fn nonfinite_active_projection_input_traps() {
    let a = [0x7fc0];
    let w = [0x3f80];
    let _ = batch_four_dots_v2!(&a, &w, 1, 0, 0, 1);
}
