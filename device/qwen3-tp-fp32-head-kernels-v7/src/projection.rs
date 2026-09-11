use fe2o3_device::{
    Bf16, Bf16MfmaAMatrix, Bf16MfmaBMatrix, DeviceMatrix, F32AccumulatorFragment, Index1D, Tiled2D,
    Wave64, WaveLane, WriteOnlyDisjointSlice, kernel, memory, thread,
};

#[cfg(test)]
macro_rules! four_dots {
    ($a:expr, $weights:expr, $rows:expr, $row_base:expr, $column:expr) => {{
        let row_0 = $row_base;
        let row_1 = row_0 + 1;
        let row_2 = row_1 + 1;
        let row_3 = row_2 + 1;
        let mut sum_0 = 0.0_f32;
        let mut sum_1 = 0.0_f32;
        let mut sum_2 = 0.0_f32;
        let mut sum_3 = 0.0_f32;
        let mut inner = 0_usize;
        while inner < 4096 {
            let right =
                Bf16::from_bits(memory::volatile_load($weights, $column * 4096 + inner)).to_f32();
            if row_0 < $rows {
                let left =
                    Bf16::from_bits(memory::volatile_load($a, row_0 * 4096 + inner)).to_f32();
                let product = left * right;
                sum_0 += product;
                if !product.is_finite() || !sum_0.is_finite() {
                    fe2o3_device::trap();
                }
            }
            if row_1 < $rows {
                let left =
                    Bf16::from_bits(memory::volatile_load($a, row_1 * 4096 + inner)).to_f32();
                let product = left * right;
                sum_1 += product;
                if !product.is_finite() || !sum_1.is_finite() {
                    fe2o3_device::trap();
                }
            }
            if row_2 < $rows {
                let left =
                    Bf16::from_bits(memory::volatile_load($a, row_2 * 4096 + inner)).to_f32();
                let product = left * right;
                sum_2 += product;
                if !product.is_finite() || !sum_2.is_finite() {
                    fe2o3_device::trap();
                }
            }
            if row_3 < $rows {
                let left =
                    Bf16::from_bits(memory::volatile_load($a, row_3 * 4096 + inner)).to_f32();
                let product = left * right;
                sum_3 += product;
                if !product.is_finite() || !sum_3.is_finite() {
                    fe2o3_device::trap();
                }
            }
            inner += 1;
        }
        (sum_0, sum_1, sum_2, sum_3)
    }};
}

/// Unchanged serial FP32 accumulation over original [vocabulary, hidden] BF16 weights.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [9496, 1, 1]), control_flow(loop_bounds(4096)))]
#[allow(clippy::too_many_arguments)]
pub fn ferric_qwen3_tp_head_bf16_f32_v7(
    a: &[u16],
    weights: &[u16],
    mut output: WriteOnlyDisjointSlice<f32, Tiled2D<Index1D, 64, 16, 16, 4>>,
    rows: u32,
    n: u32,
    k: u32,
    world_size: u32,
    projection: u32,
) {
    if rows == 0 || rows > 16 || n != 151936 || k != 4096 || world_size != 1 || projection != 6 {
        fe2o3_device::trap();
    }
    let rows = rows as usize;
    if rows < 17 {
    } else {
        fe2o3_device::trap();
    }
    if a.len() < rows * 4096
        || a.len() > 16 * 4096
        || weights.len() != 151936 * 4096
        || output.len() < rows * 151936
        || output.len() > 16 * 151936
        || thread::launch_extent_1d() != 9496 * 64
    {
        fe2o3_device::trap();
    }
    let invocation = thread::index_1d();
    let raw = invocation.get();
    let tile_column = raw / 64;
    let lane = raw % 64;
    if tile_column < 9496 {
    } else {
        fe2o3_device::trap();
    }
    let column = tile_column * 16 + lane % 16;
    let row_base = (lane / 16) * 4;
    if row_base < 13 && column < 151936 {
    } else {
        fe2o3_device::trap();
    }
    let Some(tile) = invocation.checked_tiled_2d::<64, 16, 16, 4>() else {
        fe2o3_device::trap();
    };
    let (sum_0, sum_1, sum_2, sum_3) = {
        // BEGIN four_dots
        let row_0 = row_base;
        let row_1 = row_0 + 1;
        let row_2 = row_1 + 1;
        let row_3 = row_2 + 1;
        let mut sum_0 = 0.0_f32;
        let mut sum_1 = 0.0_f32;
        let mut sum_2 = 0.0_f32;
        let mut sum_3 = 0.0_f32;
        let mut inner = 0_usize;
        while inner < 4096 {
            let right =
                Bf16::from_bits(memory::volatile_load(weights, column * 4096 + inner)).to_f32();
            if row_0 < rows {
                let left = Bf16::from_bits(memory::volatile_load(a, row_0 * 4096 + inner)).to_f32();
                let product = left * right;
                sum_0 += product;
                if !product.is_finite() || !sum_0.is_finite() {
                    fe2o3_device::trap();
                }
            }
            if row_1 < rows {
                let left = Bf16::from_bits(memory::volatile_load(a, row_1 * 4096 + inner)).to_f32();
                let product = left * right;
                sum_1 += product;
                if !product.is_finite() || !sum_1.is_finite() {
                    fe2o3_device::trap();
                }
            }
            if row_2 < rows {
                let left = Bf16::from_bits(memory::volatile_load(a, row_2 * 4096 + inner)).to_f32();
                let product = left * right;
                sum_2 += product;
                if !product.is_finite() || !sum_2.is_finite() {
                    fe2o3_device::trap();
                }
            }
            if row_3 < rows {
                let left = Bf16::from_bits(memory::volatile_load(a, row_3 * 4096 + inner)).to_f32();
                let product = left * right;
                sum_3 += product;
                if !product.is_finite() || !sum_3.is_finite() {
                    fe2o3_device::trap();
                }
            }
            inner += 1;
        }
        (sum_0, sum_1, sum_2, sum_3)
        // END four_dots
    };
    if row_base < rows && !output.write_tiled_2d(&tile, 0, rows, 151936, 151936, sum_0) {
        fe2o3_device::trap();
    }
    if row_base + 1 < rows && !output.write_tiled_2d(&tile, 1, rows, 151936, 151936, sum_1) {
        fe2o3_device::trap();
    }
    if row_base + 2 < rows && !output.write_tiled_2d(&tile, 2, rows, 151936, 151936, sum_2) {
        fe2o3_device::trap();
    }
    if row_base + 3 < rows && !output.write_tiled_2d(&tile, 3, rows, 151936, 151936, sum_3) {
        fe2o3_device::trap();
    }
}

/// Existing Wave64 MFMA accumulation, retaining FP32 instead of narrowing the final head.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [9496, 1, 1]), control_flow(loop_bounds(256)))]
#[allow(clippy::too_many_arguments)]
pub fn ferric_qwen3_tp_mfma_head_f32_v7(
    a: &[u16],
    weights_kn: &[u16],
    mut output: WriteOnlyDisjointSlice<f32, Tiled2D<Index1D, 64, 16, 16, 4>>,
    rows: u32,
    n: u32,
    k: u32,
    world_size: u32,
    projection: u32,
) {
    if rows == 0 || rows > 16 || n != 151936 || k != 4096 || world_size != 1 || projection != 6 {
        fe2o3_device::trap();
    }
    let rows = rows as usize;
    if rows < 17 {
    } else {
        fe2o3_device::trap();
    }
    if a.len() < rows * 4096 || a.len() > 16 * 4096 || weights_kn.len() != 151936 * 4096 {
        fe2o3_device::trap();
    }
    let invocation = thread::index_1d();
    let raw = invocation.get();
    let tile_column = raw / 64;
    let row_base = (raw % 64 / 16) * 4;
    let lane = WaveLane::<Wave64>::current();
    let Ok(left) = Bf16MfmaAMatrix::row_major(a, 0, rows, 4096, 4096) else {
        fe2o3_device::trap();
    };
    let Ok(right) = Bf16MfmaBMatrix::row_major(weights_kn, 0, 4096, 151936, 151936) else {
        fe2o3_device::trap();
    };
    let matrix = DeviceMatrix::current();
    let mut accumulator = F32AccumulatorFragment::zero(&lane);
    let mut step = 0_usize;
    while step < 256 {
        let left_fragment = left.load_m16k16(&lane, 0, step * 16);
        let right_fragment = right.load_k16n16(&lane, step * 16, tile_column * 16);
        accumulator = matrix.multiply_accumulate(left_fragment, right_fragment, accumulator);
        step += 1;
    }
    let [value_0, value_1, value_2, value_3] = accumulator.into_values();
    if output.len() < rows * 151936 || output.len() > 16 * 151936 {
        fe2o3_device::trap();
    }
    if tile_column < 9496 {
    } else {
        fe2o3_device::trap();
    }
    if thread::grid_dim_x() != 9496 || thread::block_dim_x() != 64 {
        fe2o3_device::trap();
    }
    let Some(tile) = invocation.checked_tiled_2d::<64, 16, 16, 4>() else {
        fe2o3_device::trap();
    };
    if row_base < rows
        && (!value_0.is_finite() || !output.write_tiled_2d(&tile, 0, rows, 151936, 151936, value_0))
    {
        fe2o3_device::trap();
    }
    if row_base + 1 < rows
        && (!value_1.is_finite() || !output.write_tiled_2d(&tile, 1, rows, 151936, 151936, value_1))
    {
        fe2o3_device::trap();
    }
    if row_base + 2 < rows
        && (!value_2.is_finite() || !output.write_tiled_2d(&tile, 2, rows, 151936, 151936, value_2))
    {
        fe2o3_device::trap();
    }
    if row_base + 3 < rows
        && (!value_3.is_finite() || !output.write_tiled_2d(&tile, 3, rows, 151936, 151936, value_3))
    {
        fe2o3_device::trap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serial_body_matches_independent_real_width_reference_and_masks_inactive_rows() {
        for rows in [1_usize, 2, 3, 4, 5, 6, 15, 16] {
            let a: std::vec::Vec<_> = (0..rows * 4096)
                .map(|i| Bf16::from_f32(((i * 17 % 37) as f32 - 18.0) / 32.0).to_bits())
                .collect();
            let weights: std::vec::Vec<_> = (0..3 * 4096)
                .map(|i| Bf16::from_f32(((i * 11 % 43) as f32 - 21.0) / 64.0).to_bits())
                .collect();
            for column in 0..3 {
                for base in [0, 4, 8, 12] {
                    let (a0, a1, a2, a3) = four_dots!(&a, &weights, rows, base, column);
                    for (offset, actual) in [a0, a1, a2, a3].into_iter().enumerate() {
                        let row = base + offset;
                        let mut reference = 0.0_f64;
                        if row < rows {
                            for inner in 0..4096 {
                                reference +=
                                    f64::from(Bf16::from_bits(a[row * 4096 + inner]).to_f32())
                                        * f64::from(
                                            Bf16::from_bits(weights[column * 4096 + inner])
                                                .to_f32(),
                                        );
                            }
                        }
                        assert_eq!(f64::from(actual), reference);
                    }
                }
            }
        }
    }

    #[test]
    fn serial_nonfinite_and_overflow_trap() {
        for (left, right) in [(f32::NAN, 1.0), (1.0, f32::INFINITY), (f32::MAX, 2.0)] {
            let a = std::vec![Bf16::from_f32(left).to_bits(); 4096];
            let weights = std::vec![Bf16::from_f32(right).to_bits(); 4096];
            assert!(std::panic::catch_unwind(|| four_dots!(&a, &weights, 1, 0, 0)).is_err());
        }
    }
}
