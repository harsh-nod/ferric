use fe2o3_device::{Bf16, Gfx950Subgroup, Index1D, RowStriped2D, WriteOnlyDisjointSlice, kernel, memory, thread};
#[cfg(feature = "mfma")]
use fe2o3_device::{Bf16MfmaAMatrix, Bf16MfmaBMatrix, DeviceMatrix, F32AccumulatorFragment, Tiled2D, Wave64, WaveLane};

/// One Wave64 cooperatively computes each row/output-column dot product.
/// Weights are the unchanged v2 row-major [n,k] layout.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [2430976, 1, 1]), control_flow(loop_bounds(64)))]
#[allow(clippy::too_many_arguments)]
pub fn ferric_qwen3_tp_wave_gemv_bf16_v3(
    a: &[u16],
    weights: &[u16],
    mut output: WriteOnlyDisjointSlice<u16, RowStriped2D<Index1D, 64, 1>>,
    rows: u32,
    n: u32,
    k: u32,
    world_size: u32,
    projection: u32,
) {
    if rows == 0
        || rows > 16
        || k != 4096
        || !((world_size == 1
            && ((projection == 1 && n == 4096)
                || ((projection == 2 || projection == 3) && n == 1024)
                || ((projection == 4 || projection == 5) && n == 12288)))
            || (world_size == 2
                && ((projection == 1 && n == 2048)
                    || ((projection == 2 || projection == 3) && n == 512)
                    || ((projection == 4 || projection == 5) && n == 6144)))
            || (world_size == 8
                && ((projection == 1 && n == 512)
                    || ((projection == 2 || projection == 3) && n == 128)
                    || ((projection == 4 || projection == 5) && n == 1536)))
            || ((world_size == 1 || world_size == 2 || world_size == 8)
                && projection == 6
                && n == 151936))
    {
        fe2o3_device::trap();
    }
    let rows = rows as usize;
    let n = n as usize;
    if rows < 17 {
    } else {
        fe2o3_device::trap();
    }
    if n > 0 && n < 151937 {
    } else {
        fe2o3_device::trap();
    }
    if a.len() < rows * 4096
        || a.len() > 16 * 4096
        || weights.len() != n * 4096
        || output.len() < rows * n
        || output.len() > 16 * n
        || thread::launch_extent_1d() != rows * n * 64
    {
        fe2o3_device::trap();
    }
    let invocation = thread::index_1d();
    let raw = invocation.get();
    let element = raw / 64;
    let lane = raw % 64;
    if element < rows * n {
    } else {
        fe2o3_device::trap();
    }
    let (row, column) = if n == 128 {
        (element / 128, element % 128)
    } else if n == 512 {
        (element / 512, element % 512)
    } else if n == 1024 {
        (element / 1024, element % 1024)
    } else if n == 1536 {
        (element / 1536, element % 1536)
    } else if n == 2048 {
        (element / 2048, element % 2048)
    } else if n == 4096 {
        (element / 4096, element % 4096)
    } else if n == 6144 {
        (element / 6144, element % 6144)
    } else if n == 12288 {
        (element / 12288, element % 12288)
    } else if n == 151936 {
        (element / 151936, element % 151936)
    } else {
        fe2o3_device::trap()
    };
    if row < 16 {
    } else {
        fe2o3_device::trap();
    }
    if column < 151936 {
    } else {
        fe2o3_device::trap();
    }
    let row = row as u16 as usize;
    let column = column as u32 as usize;
    let subgroup = Gfx950Subgroup::current();
    let mut partial = 0.0_f32;
    let mut step = 0_usize;
    while step < 64 {
        let inner = step * 64 + lane;
        let left = Bf16::from_bits(memory::volatile_load(a, row * 4096 + inner)).to_f32();
        let right = Bf16::from_bits(memory::volatile_load(weights, column * 4096 + inner)).to_f32();
        let product = left * right;
        partial += product;
        if !product.is_finite() || !partial.is_finite() {
            fe2o3_device::trap();
        }
        step += 1;
    }
    let sum = subgroup.reduce_sum_f32::<64>(partial);
    let narrowed = Bf16::from_f32(sum);
    if !sum.is_finite() || !narrowed.is_finite() {
        fe2o3_device::trap();
    }
    if lane == 0 {
        let Some(stripe) = invocation.checked_row_striped_2d::<64, 1>() else {
            fe2o3_device::trap();
        };
        if !output.write_row_striped_2d(&stripe, 0, rows * n, 1, 1, narrowed.to_bits()) {
            fe2o3_device::trap();
        }
    }
}

/// FP32 TP partials retain precision until the separately checked reduction.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [65536, 1, 1]), control_flow(loop_bounds(192)))]
#[allow(clippy::too_many_arguments)]
pub fn ferric_qwen3_tp_wave_gemv_partial_f32_v3(
    a: &[u16],
    weights: &[u16],
    mut output: WriteOnlyDisjointSlice<f32, RowStriped2D<Index1D, 64, 1>>,
    rows: u32,
    n: u32,
    k: u32,
    world_size: u32,
    projection: u32,
) {
    if rows == 0
        || rows > 16
        || n != 4096
        || !((world_size == 1
            && ((projection == 1 && k == 4096) || (projection == 2 && k == 12288)))
            || (world_size == 2
                && ((projection == 1 && k == 2048) || (projection == 2 && k == 6144)))
            || (world_size == 8
                && ((projection == 1 && k == 512) || (projection == 2 && k == 1536))))
    {
        fe2o3_device::trap();
    }
    let rows = rows as usize;
    let k = k as usize;
    if rows < 17 {
    } else {
        fe2o3_device::trap();
    }
    if k > 0 && k < 12289 {
    } else {
        fe2o3_device::trap();
    }
    if a.len() < rows * k
        || a.len() > 16 * k
        || weights.len() != 4096 * k
        || output.len() < rows * 4096
        || output.len() > 16 * 4096
        || thread::launch_extent_1d() != rows * 4096 * 64
    {
        fe2o3_device::trap();
    }
    let invocation = thread::index_1d();
    let raw = invocation.get();
    let element = raw / 64;
    let lane = raw % 64;
    if element < rows * 4096 {
    } else {
        fe2o3_device::trap();
    }
    let row = element / 4096;
    let column = element % 4096;
    if row < 16 {
    } else {
        fe2o3_device::trap();
    }
    let row = row as u16 as usize;
    let k = k as u16 as usize;
    let column = column as u16 as usize;
    let subgroup = Gfx950Subgroup::current();
    let mut partial = 0.0_f32;
    let mut step = 0_usize;
    while step < 192 {
        let inner = step * 64 + lane;
        if inner < k {
            let inner = inner as u16 as usize;
            let left = Bf16::from_bits(memory::volatile_load(a, row * k + inner)).to_f32();
            let right = Bf16::from_bits(memory::volatile_load(weights, column * k + inner)).to_f32();
            let product = left * right;
            partial += product;
            if !product.is_finite() || !partial.is_finite() {
                fe2o3_device::trap();
            }
        }
        step += 1;
    }
    let sum = subgroup.reduce_sum_f32::<64>(partial);
    if !sum.is_finite() {
        fe2o3_device::trap();
    }
    if lane == 0 {
        let Some(stripe) = invocation.checked_row_striped_2d::<64, 1>() else {
            fe2o3_device::trap();
        };
        if !output.write_row_striped_2d(&stripe, 0, rows * 4096, 1, 1, sum) {
            fe2o3_device::trap();
        }
    }
}

/// One Wave64 owns a 16x16 tile, with zero-filled inactive activation rows.
/// `weights_kn` is a separately resident row-major [k,n] transpose of v2 weights.
#[cfg(feature = "mfma")]
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [9496, 1, 1]), control_flow(loop_bounds(256)))]
#[allow(clippy::too_many_arguments)]
pub fn ferric_qwen3_tp_mfma_gemm_bf16_v3(
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
        || rows > 16
        || k != 4096
        || !((world_size == 1
            && ((projection == 1 && n == 4096)
                || ((projection == 2 || projection == 3) && n == 1024)
                || ((projection == 4 || projection == 5) && n == 12288)))
            || (world_size == 2
                && ((projection == 1 && n == 2048)
                    || ((projection == 2 || projection == 3) && n == 512)
                    || ((projection == 4 || projection == 5) && n == 6144)))
            || (world_size == 8
                && ((projection == 1 && n == 512)
                    || ((projection == 2 || projection == 3) && n == 128)
                    || ((projection == 4 || projection == 5) && n == 1536)))
            || ((world_size == 1 || world_size == 2 || world_size == 8)
                && projection == 6
                && n == 151936))
    {
        fe2o3_device::trap();
    }
    let rows = rows as usize;
    let n = n as usize;
    if rows < 17 {
    } else {
        fe2o3_device::trap();
    }
    if n < 151937 {
    } else {
        fe2o3_device::trap();
    }
    if a.len() < rows * 4096
        || a.len() > 16 * 4096
        || weights_kn.len() != n * 4096
        || output.len() < rows * n
        || output.len() > 16 * n
    {
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
    let Ok(right) = Bf16MfmaBMatrix::row_major(weights_kn, 0, 4096, n, n) else {
        fe2o3_device::trap();
    };
    let matrix = DeviceMatrix::current();
    let mut accumulator = F32AccumulatorFragment::zero(&lane);
    let mut step = 0_usize;
    while step < 256 {
        let a_fragment = left.load_m16k16(&lane, 0, step * 16);
        let b_fragment = right.load_k16n16(&lane, step * 16, tile_column * 16);
        accumulator = matrix.multiply_accumulate(a_fragment, b_fragment, accumulator);
        step += 1;
    }
    let values = accumulator.into_values();
    if tile_column < n / 16 {
    } else {
        fe2o3_device::trap();
    }
    if thread::launch_extent_1d() != (n / 16) * 64 {
        fe2o3_device::trap();
    }
    let Some(tile) = invocation.checked_tiled_2d::<64, 16, 16, 4>() else {
        fe2o3_device::trap();
    };
    if row_base < rows {
        let value = Bf16::from_f32(values[0]);
        if !values[0].is_finite()
            || !value.is_finite()
            || !output.write_tiled_2d(&tile, 0, rows, n, n, value.to_bits())
        {
            fe2o3_device::trap();
        }
    }
    if row_base + 1 < rows {
        let value = Bf16::from_f32(values[1]);
        if !values[1].is_finite()
            || !value.is_finite()
            || !output.write_tiled_2d(&tile, 1, rows, n, n, value.to_bits())
        {
            fe2o3_device::trap();
        }
    }
    if row_base + 2 < rows {
        let value = Bf16::from_f32(values[2]);
        if !values[2].is_finite()
            || !value.is_finite()
            || !output.write_tiled_2d(&tile, 2, rows, n, n, value.to_bits())
        {
            fe2o3_device::trap();
        }
    }
    if row_base + 3 < rows {
        let value = Bf16::from_f32(values[3]);
        if !values[3].is_finite()
            || !value.is_finite()
            || !output.write_tiled_2d(&tile, 3, rows, n, n, value.to_bits())
        {
            fe2o3_device::trap();
        }
    }
}

/// MFMA FP32 partial projection over exactly the rank-local reduction width.
#[cfg(feature = "mfma")]
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [256, 1, 1]), control_flow(loop_bounds(768)))]
#[allow(clippy::too_many_arguments)]
pub fn ferric_qwen3_tp_mfma_gemm_partial_f32_v3(
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
        || rows > 16
        || n != 4096
        || !((world_size == 1
            && ((projection == 1 && k == 4096) || (projection == 2 && k == 12288)))
            || (world_size == 2
                && ((projection == 1 && k == 2048) || (projection == 2 && k == 6144)))
            || (world_size == 8
                && ((projection == 1 && k == 512) || (projection == 2 && k == 1536))))
    {
        fe2o3_device::trap();
    }
    let rows = rows as usize;
    let k = k as usize;
    if rows < 17 {
    } else {
        fe2o3_device::trap();
    }
    if k > 0 && k < 12289 {
    } else {
        fe2o3_device::trap();
    }
    if a.len() < rows * k
        || a.len() > 16 * k
        || weights_kn.len() != 4096 * k
        || output.len() < rows * 4096
        || output.len() > 16 * 4096
    {
        fe2o3_device::trap();
    }
    let invocation = thread::index_1d();
    let raw = invocation.get();
    let tile_column = raw / 64;
    let row_base = (raw % 64 / 16) * 4;
    let lane = WaveLane::<Wave64>::current();
    let Ok(left) = Bf16MfmaAMatrix::row_major(a, 0, rows, k, k) else {
        fe2o3_device::trap();
    };
    let Ok(right) = Bf16MfmaBMatrix::row_major(weights_kn, 0, k, 4096, 4096) else {
        fe2o3_device::trap();
    };
    let matrix = DeviceMatrix::current();
    let mut accumulator = F32AccumulatorFragment::zero(&lane);
    let mut step = 0_usize;
    while step < k / 16 {
        let a_fragment = left.load_m16k16(&lane, 0, step * 16);
        let b_fragment = right.load_k16n16(&lane, step * 16, tile_column * 16);
        accumulator = matrix.multiply_accumulate(a_fragment, b_fragment, accumulator);
        step += 1;
    }
    let values = accumulator.into_values();
    if tile_column < 256 {
    } else {
        fe2o3_device::trap();
    }
    if thread::launch_extent_1d() != 256 * 64 {
        fe2o3_device::trap();
    }
    let Some(tile) = invocation.checked_tiled_2d::<64, 16, 16, 4>() else {
        fe2o3_device::trap();
    };
    if row_base < rows
        && (!values[0].is_finite() || !output.write_tiled_2d(&tile, 0, rows, 4096, 4096, values[0]))
    {
        fe2o3_device::trap();
    }
    if row_base + 1 < rows
        && (!values[1].is_finite() || !output.write_tiled_2d(&tile, 1, rows, 4096, 4096, values[1]))
    {
        fe2o3_device::trap();
    }
    if row_base + 2 < rows
        && (!values[2].is_finite() || !output.write_tiled_2d(&tile, 2, rows, 4096, 4096, values[2]))
    {
        fe2o3_device::trap();
    }
    if row_base + 3 < rows
        && (!values[3].is_finite() || !output.write_tiled_2d(&tile, 3, rows, 4096, 4096, values[3]))
    {
        fe2o3_device::trap();
    }
}
