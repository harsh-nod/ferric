use fe2o3_device::{
    Bf16, Gfx950Subgroup, Index1D, RowStriped2D, StridedReadView2D,
    WriteOnlyDisjointSlice, kernel, thread,
};

/// C1 TP1 Q/K/V and gate/up projections with lane-time-packed u32 inputs.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [12288, 1, 1]), control_flow(loop_bounds(32)))]
#[allow(clippy::too_many_arguments)]
pub fn ferric_qwen3_c1_wave_gemv_packed_u32_bf16_r2(
    a: &[u32],
    weights: &[u32],
    mut output: WriteOnlyDisjointSlice<u16, RowStriped2D<Index1D, 64, 1>>,
    rows: u32,
    n: u32,
    k: u32,
    world_size: u32,
    projection: u32,
) {
    if rows != 1
        || k != 4096
        || world_size != 1
        || !((projection == 1 && n == 4096)
            || ((projection == 2 || projection == 3) && n == 1024)
            || ((projection == 4 || projection == 5) && n == 12288))
    {
        fe2o3_device::trap();
    }
    let rows = rows as usize;
    let n = n as usize;
    if rows < 2 && n > 0 && n < 12289 {
    } else {
        fe2o3_device::trap();
    }
    if a.len() < 2048
        || a.len() > 32 * 2048
        || weights.len() != n * 2048
        || output.len() < rows * n
        || output.len() > 32 * n
        || thread::launch_extent_1d() != rows * n * 64
    {
        fe2o3_device::trap();
    }
    let Ok(left_view) = StridedReadView2D::from_shared_slice(a, 0, rows, 2048, 2048) else {
        fe2o3_device::trap();
    };
    let Ok(right_view) = StridedReadView2D::from_shared_slice(weights, 0, n, 2048, 2048) else {
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
    let column = element as u16 as usize;
    let subgroup = Gfx950Subgroup::current();
    let mut partial = 0.0_f32;
    let mut finite = true;
    let mut group = 0_usize;
    while group < 32 {
        let packed_inner = group * 64 + lane;
        let left_bits = left_view.load_or(0, packed_inner, 0);
        let right_bits = right_view.load_or(column, packed_inner, 0);
        {
            let left = Bf16::from_bits(left_bits as u16).to_f32();
            let right = Bf16::from_bits(right_bits as u16).to_f32();
            let product = left * right;
            partial += product;
            finite &= product.is_finite() & partial.is_finite();
        }
        {
            let left = Bf16::from_bits((left_bits >> 16) as u16).to_f32();
            let right = Bf16::from_bits((right_bits >> 16) as u16).to_f32();
            let product = left * right;
            partial += product;
            finite &= product.is_finite() & partial.is_finite();
        }
        group += 1;
    }
    let sum = subgroup.reduce_sum_f32::<64>(partial);
    let narrowed = Bf16::from_f32(sum);
    // Keep every lane active until the collective; reject before any store.
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
