use fe2o3_device::{Bf16, Index1D, Tiled2D, WriteOnlyDisjointSlice, kernel, memory, thread};

// Four distinct token rows share each weight load. Each row retains its own
// ascending FP32 multiply/add sequence, including the finite-result checks.
#[cfg(test)]
macro_rules! batch_four_dots_v10 {
    ($a:expr, $weights:expr, $rows:expr, $row_base:expr, $column:expr, $k:expr) => {{
        let row_0 = $row_base;
        let row_1 = row_0 + 1;
        let row_2 = row_1 + 1;
        let row_3 = row_2 + 1;
        let mut sum_0 = 0.0_f32;
        let mut sum_1 = 0.0_f32;
        let mut sum_2 = 0.0_f32;
        let mut sum_3 = 0.0_f32;
        let mut inner = 0_usize;
        while inner < $k {
            let right =
                Bf16::from_bits(memory::volatile_load($weights, $column * $k + inner)).to_f32();
            if row_0 < $rows {
                let left = Bf16::from_bits(memory::volatile_load($a, row_0 * $k + inner)).to_f32();
                let product = left * right;
                sum_0 += product;
                if !product.is_finite() || !sum_0.is_finite() {
                    fe2o3_device::trap();
                }
            }
            if row_1 < $rows {
                let left = Bf16::from_bits(memory::volatile_load($a, row_1 * $k + inner)).to_f32();
                let product = left * right;
                sum_1 += product;
                if !product.is_finite() || !sum_1.is_finite() {
                    fe2o3_device::trap();
                }
            }
            if row_2 < $rows {
                let left = Bf16::from_bits(memory::volatile_load($a, row_2 * $k + inner)).to_f32();
                let product = left * right;
                sum_2 += product;
                if !product.is_finite() || !sum_2.is_finite() {
                    fe2o3_device::trap();
                }
            }
            if row_3 < $rows {
                let left = Bf16::from_bits(memory::volatile_load($a, row_3 * $k + inner)).to_f32();
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

#[cfg(test)]
macro_rules! batch_write_bf16_v10 {
    ($output:expr, $tile:expr, $component:expr, $rows:expr, $n:expr, $value:expr) => {{
        let narrowed = Bf16::from_f32($value);
        if !narrowed.is_finite()
            || !$output.write_tiled_2d(&$tile, $component, $rows, $n, $n, narrowed.to_bits())
        {
            fe2o3_device::trap();
        }
    }};
}

#[cfg(test)]
mod tests;

/// Draft Q1/K2/V3/Gate4/Up5; weights are contiguous row-major [n,k].
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [384, 1, 1]), control_flow(loop_bounds(3072)))]
#[allow(clippy::too_many_arguments, clippy::manual_div_ceil)]
pub fn ferric_qwen3_draft_batch32_gemm_bf16_f32_bf16_v10(
    a: &[u16],
    weights: &[u16],
    mut output: WriteOnlyDisjointSlice<u16, Tiled2D<Index1D, 64, 16, 16, 4>>,
    rows: u32,
    n: u32,
    k: u32,
    world_size: u32,
    projection: u32,
) {
    if rows == 0
        || rows > 32
        || !{
            // BEGIN batch_column_shape_v10
            k == 1024
                && world_size == 1
                && ((projection == 1 && n == 2048)
                    || ((projection == 2 || projection == 3) && n == 1024)
                    || ((projection == 4 || projection == 5) && n == 3072))
            // END batch_column_shape_v10
        }
    {
        fe2o3_device::trap();
    }
    let rows = rows as usize;
    let n = n as usize;
    let k = k as usize;
    if rows < 33 {
    } else {
        fe2o3_device::trap();
    }
    if n < 3073 {
    } else {
        fe2o3_device::trap();
    }
    if k < 3073 {
    } else {
        fe2o3_device::trap();
    }
    if a.len() < rows * k
        || a.len() > 32 * k
        || weights.len() != n * k
        || output.len() < rows * n
        || output.len() > 32 * n
        || thread::grid_dim_x() as usize != ((rows + 15) / 16) * (n / 16)
        || thread::block_dim_x() != 64
    {
        fe2o3_device::trap();
    }
    let invocation = thread::index_1d();
    let raw = invocation.get();
    let tile_index = raw / 64;
    let (tile_row, tile_column) = if n == 1024 {
        (tile_index / 64, tile_index % 64)
    } else if n == 2048 {
        (tile_index / 128, tile_index % 128)
    } else if n == 3072 {
        (tile_index / 192, tile_index % 192)
    } else {
        fe2o3_device::trap()
    };
    if tile_row < 2 {
    } else {
        fe2o3_device::trap();
    }
    let tile_row = tile_row as u8 as usize;
    let lane = raw % 64;
    if tile_column < n / 16 {
    } else {
        fe2o3_device::trap();
    }
    let tile_column = tile_column as u16 as usize;
    let column = tile_column * 16 + lane % 16;
    let row_base = tile_row * 16 + (lane / 16) * 4;
    if row_base < 29 {
    } else {
        fe2o3_device::trap();
    }
    let row_base = row_base as u8 as usize;
    if column < n {
    } else {
        fe2o3_device::trap();
    }
    let Some(tile) = invocation.checked_tiled_2d::<64, 16, 16, 4>() else {
        fe2o3_device::trap();
    };
    let (sum_0, sum_1, sum_2, sum_3) = {
        // BEGIN batch_four_dots_v10
        let row_0 = row_base;
        let row_1 = row_0 + 1;
        let row_2 = row_1 + 1;
        let row_3 = row_2 + 1;
        let mut sum_0 = 0.0_f32;
        let mut sum_1 = 0.0_f32;
        let mut sum_2 = 0.0_f32;
        let mut sum_3 = 0.0_f32;
        let mut inner = 0_usize;
        while inner < k {
            let right =
                Bf16::from_bits(memory::volatile_load(weights, column * k + inner)).to_f32();
            if row_0 < rows {
                let left = Bf16::from_bits(memory::volatile_load(a, row_0 * k + inner)).to_f32();
                let product = left * right;
                sum_0 += product;
                if !product.is_finite() || !sum_0.is_finite() {
                    fe2o3_device::trap();
                }
            }
            if row_1 < rows {
                let left = Bf16::from_bits(memory::volatile_load(a, row_1 * k + inner)).to_f32();
                let product = left * right;
                sum_1 += product;
                if !product.is_finite() || !sum_1.is_finite() {
                    fe2o3_device::trap();
                }
            }
            if row_2 < rows {
                let left = Bf16::from_bits(memory::volatile_load(a, row_2 * k + inner)).to_f32();
                let product = left * right;
                sum_2 += product;
                if !product.is_finite() || !sum_2.is_finite() {
                    fe2o3_device::trap();
                }
            }
            if row_3 < rows {
                let left = Bf16::from_bits(memory::volatile_load(a, row_3 * k + inner)).to_f32();
                let product = left * right;
                sum_3 += product;
                if !product.is_finite() || !sum_3.is_finite() {
                    fe2o3_device::trap();
                }
            }
            inner += 1;
        }
        (sum_0, sum_1, sum_2, sum_3)
        // END batch_four_dots_v10
    };
    if row_base < rows {
        {
            // BEGIN batch_write_bf16_v10
            let narrowed = Bf16::from_f32(sum_0);
            if !narrowed.is_finite()
                || !output.write_tiled_2d(&tile, 0, rows, n, n, narrowed.to_bits())
            {
                fe2o3_device::trap();
            }
            // END batch_write_bf16_v10
        };
    }
    if row_base + 1 < rows {
        {
            // BEGIN batch_write_bf16_v10
            let narrowed = Bf16::from_f32(sum_1);
            if !narrowed.is_finite()
                || !output.write_tiled_2d(&tile, 1, rows, n, n, narrowed.to_bits())
            {
                fe2o3_device::trap();
            }
            // END batch_write_bf16_v10
        };
    }
    if row_base + 2 < rows {
        {
            // BEGIN batch_write_bf16_v10
            let narrowed = Bf16::from_f32(sum_2);
            if !narrowed.is_finite()
                || !output.write_tiled_2d(&tile, 2, rows, n, n, narrowed.to_bits())
            {
                fe2o3_device::trap();
            }
            // END batch_write_bf16_v10
        };
    }
    if row_base + 3 < rows {
        {
            // BEGIN batch_write_bf16_v10
            let narrowed = Bf16::from_f32(sum_3);
            if !narrowed.is_finite()
                || !output.write_tiled_2d(&tile, 3, rows, n, n, narrowed.to_bits())
            {
                fe2o3_device::trap();
            }
            // END batch_write_bf16_v10
        };
    }
}

/// O1/Down2 FP32 partials: no residual addition and no BF16 narrowing.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [128, 1, 1]), control_flow(loop_bounds(3072)))]
#[allow(clippy::too_many_arguments, clippy::manual_div_ceil)]
pub fn ferric_qwen3_draft_batch32_gemm_partial_bf16_f32_v10(
    a: &[u16],
    weights: &[u16],
    mut output: WriteOnlyDisjointSlice<f32, Tiled2D<Index1D, 64, 16, 16, 4>>,
    rows: u32,
    n: u32,
    k: u32,
    world_size: u32,
    projection: u32,
) {
    if rows == 0
        || rows > 32
        || !{
            // BEGIN batch_partial_shape_v10
            n == 1024
                && world_size == 1
                && ((projection == 1 && k == 2048) || (projection == 2 && k == 3072))
            // END batch_partial_shape_v10
        }
    {
        fe2o3_device::trap();
    }
    let rows = rows as usize;
    let n = n as usize;
    let k = k as usize;
    if rows < 33 {
    } else {
        fe2o3_device::trap();
    }
    if n < 1025 {
    } else {
        fe2o3_device::trap();
    }
    if k < 3073 {
    } else {
        fe2o3_device::trap();
    }
    if a.len() < rows * k
        || a.len() > 32 * k
        || weights.len() != n * k
        || output.len() < rows * n
        || output.len() > 32 * n
        || thread::grid_dim_x() as usize != ((rows + 15) / 16) * (n / 16)
        || thread::block_dim_x() != 64
    {
        fe2o3_device::trap();
    }
    let invocation = thread::index_1d();
    let raw = invocation.get();
    let tile_index = raw / 64;
    let (tile_row, tile_column) = if n == 1024 {
        (tile_index / 64, tile_index % 64)
    } else if n == 2048 {
        (tile_index / 128, tile_index % 128)
    } else if n == 3072 {
        (tile_index / 192, tile_index % 192)
    } else {
        fe2o3_device::trap()
    };
    if tile_row < 2 {
    } else {
        fe2o3_device::trap();
    }
    let tile_row = tile_row as u8 as usize;
    let lane = raw % 64;
    if tile_column < 64 {
    } else {
        fe2o3_device::trap();
    }
    let tile_column = tile_column as u16 as usize;
    let column = tile_column * 16 + lane % 16;
    let row_base = tile_row * 16 + (lane / 16) * 4;
    if row_base < 29 {
    } else {
        fe2o3_device::trap();
    }
    let row_base = row_base as u8 as usize;
    if column < n {
    } else {
        fe2o3_device::trap();
    }
    let Some(tile) = invocation.checked_tiled_2d::<64, 16, 16, 4>() else {
        fe2o3_device::trap();
    };
    let (sum_0, sum_1, sum_2, sum_3) = {
        // BEGIN batch_four_dots_v10
        let row_0 = row_base;
        let row_1 = row_0 + 1;
        let row_2 = row_1 + 1;
        let row_3 = row_2 + 1;
        let mut sum_0 = 0.0_f32;
        let mut sum_1 = 0.0_f32;
        let mut sum_2 = 0.0_f32;
        let mut sum_3 = 0.0_f32;
        let mut inner = 0_usize;
        while inner < k {
            let right =
                Bf16::from_bits(memory::volatile_load(weights, column * k + inner)).to_f32();
            if row_0 < rows {
                let left = Bf16::from_bits(memory::volatile_load(a, row_0 * k + inner)).to_f32();
                let product = left * right;
                sum_0 += product;
                if !product.is_finite() || !sum_0.is_finite() {
                    fe2o3_device::trap();
                }
            }
            if row_1 < rows {
                let left = Bf16::from_bits(memory::volatile_load(a, row_1 * k + inner)).to_f32();
                let product = left * right;
                sum_1 += product;
                if !product.is_finite() || !sum_1.is_finite() {
                    fe2o3_device::trap();
                }
            }
            if row_2 < rows {
                let left = Bf16::from_bits(memory::volatile_load(a, row_2 * k + inner)).to_f32();
                let product = left * right;
                sum_2 += product;
                if !product.is_finite() || !sum_2.is_finite() {
                    fe2o3_device::trap();
                }
            }
            if row_3 < rows {
                let left = Bf16::from_bits(memory::volatile_load(a, row_3 * k + inner)).to_f32();
                let product = left * right;
                sum_3 += product;
                if !product.is_finite() || !sum_3.is_finite() {
                    fe2o3_device::trap();
                }
            }
            inner += 1;
        }
        (sum_0, sum_1, sum_2, sum_3)
        // END batch_four_dots_v10
    };
    if row_base < rows && !output.write_tiled_2d(&tile, 0, rows, n, n, sum_0) {
        fe2o3_device::trap();
    }
    if row_base + 1 < rows && !output.write_tiled_2d(&tile, 1, rows, n, n, sum_1) {
        fe2o3_device::trap();
    }
    if row_base + 2 < rows && !output.write_tiled_2d(&tile, 2, rows, n, n, sum_2) {
        fe2o3_device::trap();
    }
    if row_base + 3 < rows && !output.write_tiled_2d(&tile, 3, rows, n, n, sum_3) {
        fe2o3_device::trap();
    }
}
