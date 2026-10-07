use fe2o3_device::{
    Bf16, Gfx950Subgroup, Index1D, RowStriped2D, StridedReadView2D, WriteOnlyDisjointSlice, kernel,
    thread,
};
#[cfg(feature = "mfma")]
use fe2o3_device::{
    Bf16MfmaAMatrix, Bf16MfmaBMatrix, DeviceMatrix, F32AccumulatorFragment, Tiled2D, Wave64,
    WaveLane,
};

/// FP32 TP partials retain precision until the separately checked reduction.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [131072, 1, 1]), control_flow(loop_bounds(192)))]
#[allow(clippy::too_many_arguments)]
pub fn ferric_qwen3_tp_batch32_wave_gemv_partial_f32_v5(
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
    let mut step = 0_usize;
    while step < 192 {
        let inner = step * 64 + lane;
        if inner < k {
            let inner = inner as u16 as usize;
            let left = Bf16::from_bits(left_view.load_or(row, inner, 0)).to_f32();
            let right = Bf16::from_bits(right_view.load_or(column, inner, 0)).to_f32();
            let product = left * right;
            partial += product;
            finite &= product.is_finite() & partial.is_finite();
        }
        step += 1;
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
