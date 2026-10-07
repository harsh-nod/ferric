use fe2o3_device::{
    Bf16, Bf16MfmaAMatrix, Bf16MfmaBMatrix, DeviceMatrix, F32AccumulatorFragment, Index1D, Tiled2D,
    Wave64, WaveLane, WriteOnlyDisjointSlice, kernel, thread,
};

/// One Wave64 owns a 16x16 tile, with zero-filled inactive activation rows.
/// `weights_kn` is a separately resident row-major [k,n] transpose of draft weights.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [384, 1, 1]), control_flow(loop_bounds(64)))]
#[allow(clippy::too_many_arguments, clippy::manual_div_ceil)]
pub fn ferric_qwen3_draft_batch32_mfma_gemm_bf16_v10(
    a: &[u16],
    weights_kn: &[u16],
    mut output: WriteOnlyDisjointSlice<u16, Tiled2D<Index1D, 64, 16, 16, 4>>,
    rows: u32,
    n: u32,
    k: u32,
    world_size: u32,
    projection: u32,
) {
    if rows == 0
        || rows > 32
        || !(k == 1024
            && world_size == 1
            && ((projection == 1 && n == 2048)
                || ((projection == 2 || projection == 3) && n == 1024)
                || ((projection == 4 || projection == 5) && n == 3072)))
    {
        fe2o3_device::trap();
    }
    let rows = rows as usize;
    let n = n as usize;
    if rows < 33 {
    } else {
        fe2o3_device::trap();
    }
    if n < 3073 {
    } else {
        fe2o3_device::trap();
    }
    if a.len() < rows * 1024 || a.len() > 32 * 1024 || weights_kn.len() != n * 1024 {
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
    // The guard makes this narrowing lossless and exposes a bounded offset.
    let tile_row = tile_row as u8 as usize;
    if tile_column < n / 16 {
    } else {
        fe2o3_device::trap();
    }
    let tile_column = tile_column as u16 as usize;
    let row_base = tile_row * 16 + (raw % 64 / 16) * 4;
    let lane = WaveLane::<Wave64>::current();
    let Ok(left) = Bf16MfmaAMatrix::row_major(a, 0, rows, 1024, 1024) else {
        fe2o3_device::trap();
    };
    let Ok(right) = Bf16MfmaBMatrix::row_major(weights_kn, 0, 1024, n, n) else {
        fe2o3_device::trap();
    };
    let matrix = DeviceMatrix::current();
    let mut accumulator = F32AccumulatorFragment::zero(&lane);
    let mut step = 0_usize;
    while step < 64 {
        let a_fragment = left.load_m16k16(&lane, tile_row * 16, step * 16);
        let b_fragment = right.load_k16n16(&lane, step * 16, tile_column * 16);
        accumulator = matrix.multiply_accumulate(a_fragment, b_fragment, accumulator);
        step += 1;
    }
    let [value_0, value_1, value_2, value_3] = accumulator.into_values();
    if output.len() < rows * n || output.len() > 32 * n {
        fe2o3_device::trap();
    }
    if thread::grid_dim_x() as usize != ((rows + 15) / 16) * (n / 16) || thread::block_dim_x() != 64
    {
        fe2o3_device::trap();
    }
    let Some(tile) = invocation.checked_tiled_2d::<64, 16, 16, 4>() else {
        fe2o3_device::trap();
    };
    if row_base < rows {
        let value = Bf16::from_f32(value_0);
        if !value_0.is_finite()
            || !value.is_finite()
            || !output.write_tiled_2d(&tile, 0, rows, n, n, value.to_bits())
        {
            fe2o3_device::trap();
        }
    }
    if row_base + 1 < rows {
        let value = Bf16::from_f32(value_1);
        if !value_1.is_finite()
            || !value.is_finite()
            || !output.write_tiled_2d(&tile, 1, rows, n, n, value.to_bits())
        {
            fe2o3_device::trap();
        }
    }
    if row_base + 2 < rows {
        let value = Bf16::from_f32(value_2);
        if !value_2.is_finite()
            || !value.is_finite()
            || !output.write_tiled_2d(&tile, 2, rows, n, n, value.to_bits())
        {
            fe2o3_device::trap();
        }
    }
    if row_base + 3 < rows {
        let value = Bf16::from_f32(value_3);
        if !value_3.is_finite()
            || !value.is_finite()
            || !output.write_tiled_2d(&tile, 3, rows, n, n, value.to_bits())
        {
            fe2o3_device::trap();
        }
    }
}

/// MFMA FP32 partial projection over exactly the rank-local reduction width.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [128, 1, 1]), control_flow(loop_bounds(128, 192)))]
#[allow(clippy::too_many_arguments, clippy::manual_div_ceil)]
pub fn ferric_qwen3_draft_batch32_mfma_gemm_partial_f32_v10(
    a: &[u16],
    weights_kn: &[u16],
    mut output: WriteOnlyDisjointSlice<f32, Tiled2D<Index1D, 64, 16, 16, 4>>,
    rows: u32,
    n: u32,
    k: u32,
    world_size: u32,
    projection: u32,
) {
    if rows == 0
        || rows > 32
        || !(n == 1024
            && world_size == 1
            && ((projection == 1 && k == 2048) || (projection == 2 && k == 3072)))
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
    if k > 0 && k < 3073 {
    } else {
        fe2o3_device::trap();
    }
    if n == 1024 {
    } else {
        fe2o3_device::trap();
    }
    if a.len() < rows * k || a.len() > 32 * k || weights_kn.len() != 1024 * k {
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
    // The guard makes this narrowing lossless and exposes a bounded offset.
    let tile_row = tile_row as u8 as usize;
    if tile_column < 64 {
    } else {
        fe2o3_device::trap();
    }
    let tile_column = tile_column as u16 as usize;
    let row_base = tile_row * 16 + (raw % 64 / 16) * 4;
    let lane = WaveLane::<Wave64>::current();
    let Ok(left) = Bf16MfmaAMatrix::row_major(a, 0, rows, k, k) else {
        fe2o3_device::trap();
    };
    let Ok(right) = Bf16MfmaBMatrix::row_major(weights_kn, 0, k, 1024, 1024) else {
        fe2o3_device::trap();
    };
    let matrix = DeviceMatrix::current();
    let mut accumulator = F32AccumulatorFragment::zero(&lane);
    // Static loops retain uniform MFMA control for both draft reduction widths.
    if k == 2048 {
        let mut step = 0_usize;
        while step < 128 {
            let a_fragment = left.load_m16k16(&lane, tile_row * 16, step * 16);
            let b_fragment = right.load_k16n16(&lane, step * 16, tile_column * 16);
            accumulator = matrix.multiply_accumulate(a_fragment, b_fragment, accumulator);
            step += 1;
        }
    } else {
        let mut step = 0_usize;
        while step < 192 {
            let a_fragment = left.load_m16k16(&lane, tile_row * 16, step * 16);
            let b_fragment = right.load_k16n16(&lane, step * 16, tile_column * 16);
            accumulator = matrix.multiply_accumulate(a_fragment, b_fragment, accumulator);
            step += 1;
        }
    }
    let values = accumulator.into_values();
    if output.len() < rows * 1024 || output.len() > 32 * 1024 {
        fe2o3_device::trap();
    }
    if thread::grid_dim_x() as usize != ((rows + 15) / 16) * 64 || thread::block_dim_x() != 64 {
        fe2o3_device::trap();
    }
    let Some(tile) = invocation.checked_tiled_2d::<64, 16, 16, 4>() else {
        fe2o3_device::trap();
    };
    if row_base < rows
        && (!values[0].is_finite() || !output.write_tiled_2d(&tile, 0, rows, n, n, values[0]))
    {
        fe2o3_device::trap();
    }
    if row_base + 1 < rows
        && (!values[1].is_finite() || !output.write_tiled_2d(&tile, 1, rows, n, n, values[1]))
    {
        fe2o3_device::trap();
    }
    if row_base + 2 < rows
        && (!values[2].is_finite() || !output.write_tiled_2d(&tile, 2, rows, n, n, values[2]))
    {
        fe2o3_device::trap();
    }
    if row_base + 3 < rows
        && (!values[3].is_finite() || !output.write_tiled_2d(&tile, 3, rows, n, n, values[3]))
    {
        fe2o3_device::trap();
    }
}
