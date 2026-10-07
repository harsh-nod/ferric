use fe2o3_device::{
    Bf16, Gfx950Subgroup, Index1D, RowStriped2D, StridedReadView2D, WriteOnlyDisjointSlice, kernel,
    thread,
};

/// V5 geometry and arithmetic, with four raw input/weight pairs loaded ahead.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [4861952, 1, 1]), control_flow(loop_bounds(16)))]
#[allow(clippy::too_many_arguments)]
pub fn ferric_qwen3_tp_wave_gemv_prefetch4_bf16_v20(
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
        || rows > 32
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
    if rows < 33 {
    } else {
        fe2o3_device::trap();
    }
    if n > 0 && n < 151937 {
    } else {
        fe2o3_device::trap();
    }
    if a.len() < rows * 4096
        || a.len() > 32 * 4096
        || weights.len() != n * 4096
        || output.len() < rows * n
        || output.len() > 32 * n
        || thread::launch_extent_1d() != rows * n * 64
    {
        fe2o3_device::trap();
    }
    let Ok(left_view) = StridedReadView2D::from_shared_slice(a, 0, rows, 4096, 4096) else {
        fe2o3_device::trap();
    };
    let Ok(right_view) = StridedReadView2D::from_shared_slice(weights, 0, n, 4096, 4096) else {
        fe2o3_device::trap();
    };
    let invocation = thread::index_1d();
    let raw = invocation.get();
    let element = thread::block_idx_x() as usize;
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
    if row < 32 {
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
    let mut finite = true;
    let mut group = 0_usize;
    while group < 16 {
        let inner_0 = group * 256 + lane;
        let inner_1 = inner_0 + 64;
        let inner_2 = inner_0 + 128;
        let inner_3 = inner_0 + 192;
        let left_bits_0 = left_view.load_or(row, inner_0, 0);
        let right_bits_0 = right_view.load_or(column, inner_0, 0);
        let left_bits_1 = left_view.load_or(row, inner_1, 0);
        let right_bits_1 = right_view.load_or(column, inner_1, 0);
        let left_bits_2 = left_view.load_or(row, inner_2, 0);
        let right_bits_2 = right_view.load_or(column, inner_2, 0);
        let left_bits_3 = left_view.load_or(row, inner_3, 0);
        let right_bits_3 = right_view.load_or(column, inner_3, 0);

        // Keep V5's ascending multiply/add/check order; do not use four sums.
        let product_0 =
            Bf16::from_bits(left_bits_0).to_f32() * Bf16::from_bits(right_bits_0).to_f32();
        partial += product_0;
        finite &= product_0.is_finite() & partial.is_finite();
        let product_1 =
            Bf16::from_bits(left_bits_1).to_f32() * Bf16::from_bits(right_bits_1).to_f32();
        partial += product_1;
        finite &= product_1.is_finite() & partial.is_finite();
        let product_2 =
            Bf16::from_bits(left_bits_2).to_f32() * Bf16::from_bits(right_bits_2).to_f32();
        partial += product_2;
        finite &= product_2.is_finite() & partial.is_finite();
        let product_3 =
            Bf16::from_bits(left_bits_3).to_f32() * Bf16::from_bits(right_bits_3).to_f32();
        partial += product_3;
        finite &= product_3.is_finite() & partial.is_finite();
        group += 1;
    }
    let sum = subgroup.reduce_sum_f32::<64>(partial);
    let narrowed = Bf16::from_f32(sum);
    if !finite || !sum.is_finite() || !narrowed.is_finite() {
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

/// V5 FP32 partials, including every inactive-tail load/arithmetic predicate.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [131072, 1, 1]), control_flow(loop_bounds(48)))]
#[allow(clippy::too_many_arguments)]
pub fn ferric_qwen3_tp_wave_gemv_prefetch4_partial_f32_v20(
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
        || rows > 32
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
    if rows < 33 {
    } else {
        fe2o3_device::trap();
    }
    if k > 0 && k < 12289 {
    } else {
        fe2o3_device::trap();
    }
    if a.len() < rows * k
        || a.len() > 32 * k
        || weights.len() != 4096 * k
        || output.len() < rows * 4096
        || output.len() > 32 * 4096
        || thread::launch_extent_1d() != rows * 4096 * 64
    {
        fe2o3_device::trap();
    }
    let Ok(left_view) = StridedReadView2D::from_shared_slice(a, 0, rows, k, k) else {
        fe2o3_device::trap();
    };
    let Ok(right_view) = StridedReadView2D::from_shared_slice(weights, 0, 4096, k, k) else {
        fe2o3_device::trap();
    };
    let invocation = thread::index_1d();
    let raw = invocation.get();
    let element = thread::block_idx_x() as usize;
    let lane = raw % 64;
    if element < rows * 4096 {
    } else {
        fe2o3_device::trap();
    }
    let row = element / 4096;
    let column = element % 4096;
    if row < 32 {
    } else {
        fe2o3_device::trap();
    }
    let row = row as u16 as usize;
    let k = k as u16 as usize;
    let column = column as u16 as usize;
    let subgroup = Gfx950Subgroup::current();
    let mut partial = 0.0_f32;
    let mut finite = true;
    // These uniform endpoints expose the checked views' physical row extents.
    if row >= rows || column >= 4096 || row * k + k > a.len()
        || column * k + k > weights.len()
    {
        fe2o3_device::trap();
    }
    let mut group = 0_usize;
    while group < 48 {
        let inner_0 = group * 256 + lane;
        let inner_1 = inner_0 + 64;
        let inner_2 = inner_0 + 128;
        let inner_3 = inner_0 + 192;
        if inner_3 < k {
            let left_bits_0 = left_view.load_or(row, inner_0 as u16 as usize, 0);
            let right_bits_0 = right_view.load_or(column, inner_0 as u16 as usize, 0);
            let left_bits_1 = left_view.load_or(row, inner_1 as u16 as usize, 0);
            let right_bits_1 = right_view.load_or(column, inner_1 as u16 as usize, 0);
            let left_bits_2 = left_view.load_or(row, inner_2 as u16 as usize, 0);
            let right_bits_2 = right_view.load_or(column, inner_2 as u16 as usize, 0);
            let left_bits_3 = left_view.load_or(row, inner_3 as u16 as usize, 0);
            let right_bits_3 = right_view.load_or(column, inner_3 as u16 as usize, 0);
            let product_0 =
                Bf16::from_bits(left_bits_0).to_f32() * Bf16::from_bits(right_bits_0).to_f32();
            partial += product_0;
            finite &= product_0.is_finite() & partial.is_finite();
            let product_1 =
                Bf16::from_bits(left_bits_1).to_f32() * Bf16::from_bits(right_bits_1).to_f32();
            partial += product_1;
            finite &= product_1.is_finite() & partial.is_finite();
            let product_2 =
                Bf16::from_bits(left_bits_2).to_f32() * Bf16::from_bits(right_bits_2).to_f32();
            partial += product_2;
            finite &= product_2.is_finite() & partial.is_finite();
            let product_3 =
                Bf16::from_bits(left_bits_3).to_f32() * Bf16::from_bits(right_bits_3).to_f32();
            partial += product_3;
            finite &= product_3.is_finite() & partial.is_finite();
        }
        group += 1;
    }
    let sum = subgroup.reduce_sum_f32::<64>(partial);
    if !finite || !sum.is_finite() {
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
