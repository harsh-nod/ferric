use fe2o3_device::{
    Bf16, Gfx950Subgroup, Index1D, RowStriped2D, StridedReadView2D, WriteOnlyDisjointSlice, kernel,
    thread,
};

/// One Wave64 per draft Q/K/V/Gate/Up output; native row-major [N,K] weights.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [3072, 1, 1]), control_flow(loop_bounds(16)))]
#[allow(clippy::too_many_arguments)]
pub fn ferric_qwen3_draft_wave_m1_gemv_bf16_v16(
    a: &[u16],
    weights: &[u16],
    mut output: WriteOnlyDisjointSlice<u16, RowStriped2D<Index1D, 64, 1>>,
    rows: u32,
    n: u32,
    k: u32,
    world_size: u32,
    projection: u32,
) {
    if rows != 1
        || world_size != 1
        || k != 1024
        || !((projection == 1 && n == 2048)
            || ((projection == 2 || projection == 3) && n == 1024)
            || ((projection == 4 || projection == 5) && n == 3072))
    {
        fe2o3_device::trap();
    }
    let n = n as usize;
    if n > 0 && n < 3073 {
    } else {
        fe2o3_device::trap();
    }
    if a.len() < 1024
        || a.len() > 32 * 1024
        || weights.len() != n * 1024
        || output.len() < n
        || output.len() > 32 * n
        || thread::grid_dim_x() as usize != n
        || thread::grid_dim_y() != 1
        || thread::grid_dim_z() != 1
        || thread::block_dim_x() != 64
    {
        fe2o3_device::trap();
    }
    let Ok(left_view) = StridedReadView2D::from_shared_slice(a, 0, 1, 1024, 1024) else {
        fe2o3_device::trap();
    };
    let Ok(right_view) = StridedReadView2D::from_shared_slice(weights, 0, n, 1024, 1024) else {
        fe2o3_device::trap();
    };
    let invocation = thread::index_1d();
    let lane = invocation.get() % 64;
    let column = thread::block_idx_x() as usize;
    let subgroup = Gfx950Subgroup::current();
    let mut partial = 0.0_f32;
    let mut finite = true;
    let mut step = 0_usize;
    while step < 16 {
        let inner = step * 64 + lane;
        let left = Bf16::from_bits(left_view.load_or(0, inner, 0x7fc0)).to_f32();
        let right = Bf16::from_bits(right_view.load_or(column, inner, 0x7fc0)).to_f32();
        let product = left * right;
        partial += product;
        finite &= product.is_finite() & partial.is_finite();
        step += 1;
    }
    let sum = subgroup.reduce_sum_f32::<64>(partial);
    let invalid = if finite { 0.0_f32 } else { 1.0_f32 };
    let any_invalid = subgroup.reduce_max_f32::<64>(invalid);
    let narrowed = Bf16::from_f32(sum);
    if any_invalid != 0.0 || !sum.is_finite() || !narrowed.is_finite() {
        fe2o3_device::trap();
    }
    if lane == 0 {
        let Some(stripe) = invocation.checked_row_striped_2d::<64, 1>() else {
            fe2o3_device::trap();
        };
        if !output.write_row_striped_2d(&stripe, 0, n, 1, 1, narrowed.to_bits()) {
            fe2o3_device::trap();
        }
    }
}

/// FP32 O/Down partials and FP32 Head logits; no residual or BF16 output rounding.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [151936, 1, 1]), control_flow(loop_bounds(48)))]
#[allow(clippy::too_many_arguments)]
pub fn ferric_qwen3_draft_wave_m1_gemv_f32_v16(
    a: &[u16],
    weights: &[u16],
    mut output: WriteOnlyDisjointSlice<f32, RowStriped2D<Index1D, 64, 1>>,
    rows: u32,
    n: u32,
    k: u32,
    world_size: u32,
    projection: u32,
) {
    if rows != 1
        || world_size != 1
        || !((n == 1024 && ((projection == 1 && k == 2048) || (projection == 2 && k == 3072)))
            || (n == 151936 && projection == 6 && k == 1024))
    {
        fe2o3_device::trap();
    }
    let n = n as usize;
    let k = k as usize;
    if n > 0 && n < 151937 {
    } else {
        fe2o3_device::trap();
    }
    if k > 0 && k < 3073 {
    } else {
        fe2o3_device::trap();
    }
    if a.len() < k
        || a.len() > 32 * k
        || weights.len() != n * k
        || output.len() < n
        || output.len() > 32 * n
        || thread::grid_dim_x() as usize != n
        || thread::grid_dim_y() != 1
        || thread::grid_dim_z() != 1
        || thread::block_dim_x() != 64
    {
        fe2o3_device::trap();
    }
    let Ok(left_view) = StridedReadView2D::from_shared_slice(a, 0, 1, k, k) else {
        fe2o3_device::trap();
    };
    let Ok(right_view) = StridedReadView2D::from_shared_slice(weights, 0, n, k, k) else {
        fe2o3_device::trap();
    };
    let invocation = thread::index_1d();
    let lane = invocation.get() % 64;
    let column = thread::block_idx_x() as usize;
    let subgroup = Gfx950Subgroup::current();
    let mut partial = 0.0_f32;
    let mut finite = true;
    let mut step = 0_usize;
    while step < 48 {
        let inner = step * 64 + lane;
        if inner < k {
            let left = Bf16::from_bits(left_view.load_or(0, inner, 0x7fc0)).to_f32();
            let right = Bf16::from_bits(right_view.load_or(column, inner, 0x7fc0)).to_f32();
            let product = left * right;
            partial += product;
            finite &= product.is_finite() & partial.is_finite();
        }
        step += 1;
    }
    let sum = subgroup.reduce_sum_f32::<64>(partial);
    let invalid = if finite { 0.0_f32 } else { 1.0_f32 };
    let any_invalid = subgroup.reduce_max_f32::<64>(invalid);
    if any_invalid != 0.0 || !sum.is_finite() {
        fe2o3_device::trap();
    }
    if lane == 0 {
        let Some(stripe) = invocation.checked_row_striped_2d::<64, 1>() else {
            fe2o3_device::trap();
        };
        if !output.write_row_striped_2d(&stripe, 0, n, 1, 1, sum) {
            fe2o3_device::trap();
        }
    }
}
