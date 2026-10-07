use fe2o3_device::{
    Bf16MfmaAMatrix, Bf16MfmaBMatrix, DeviceMatrix, F32AccumulatorFragment, Index1D, RowStriped2D,
    StridedReadView2D, Wave64, WaveLane, WriteOnlyDisjointSlice, kernel, thread,
};

/// Eight disjoint K1536 partitions of the fixed TP1/C1 down projection.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [2048, 1, 1]), control_flow(loop_bounds(48)))]
#[allow(clippy::too_many_arguments)]
pub fn ferric_qwen3_c1_down_splitk8_mfma_partial_f32_r1(
    a: &[u16],
    weights_kn: &[u16],
    mut partials: WriteOnlyDisjointSlice<f32, RowStriped2D<Index1D, 64, 1>>,
    rows: u32,
    n: u32,
    k: u32,
    world_size: u32,
    projection: u32,
) {
    if rows != 1
        || n != 4096
        || k != 12288
        || world_size != 1
        || projection != 2
        || a.len() < 12288
        || a.len() > 32 * 12288
        || weights_kn.len() != 12288 * 4096
        || partials.len() != 8 * 4096
        || thread::grid_dim_x() != 2048
        || thread::block_dim_x() != 64
    {
        fe2o3_device::trap();
    }
    let invocation = thread::index_1d();
    let raw = invocation.get();
    let group = raw / 64;
    let partition = group / 256;
    let tile_column = group % 256;
    if partition < 8 && tile_column < 256 {
    } else {
        fe2o3_device::trap();
    }
    let partition = partition as u8 as usize;
    let tile_column = tile_column as u8 as usize;
    let lane = WaveLane::<Wave64>::current();
    let Ok(left) = Bf16MfmaAMatrix::row_major(a, 0, 1, 12288, 12288) else {
        fe2o3_device::trap();
    };
    let Ok(right) = Bf16MfmaBMatrix::row_major(weights_kn, 0, 12288, 4096, 4096) else {
        fe2o3_device::trap();
    };
    let matrix = DeviceMatrix::current();
    let mut accumulator = F32AccumulatorFragment::zero(&lane);
    let mut pair = 0_usize;
    while pair < 48 {
        let reduction_base = partition * 1536 + pair * 32;
        let next_reduction_base = reduction_base + 16;
        let a_fragment = left.load_m16k16(&lane, 0, reduction_base);
        let b_fragment = right.load_k16n16(&lane, reduction_base, tile_column * 16);
        let next_a_fragment = left.load_m16k16(&lane, 0, next_reduction_base);
        let next_b_fragment = right.load_k16n16(&lane, next_reduction_base, tile_column * 16);
        accumulator = matrix.multiply_accumulate(a_fragment, b_fragment, accumulator);
        accumulator = matrix.multiply_accumulate(next_a_fragment, next_b_fragment, accumulator);
        pair += 1;
    }
    let [value_0, _, _, _] = accumulator.into_values();
    // MFMA row zero is value_0 in lanes 0..15. Other rows are inactive.
    if raw % 64 < 16 {
        let Some(stripe) = invocation.checked_row_striped_2d::<64, 1>() else {
            fe2o3_device::trap();
        };
        if !value_0.is_finite() || !partials.write_row_striped_2d(&stripe, 0, 2048, 16, 16, value_0)
        {
            fe2o3_device::trap();
        }
    }
}

/// Merge K partitions in increasing order; keep the output FP32 for the residual.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [64, 1, 1]), control_flow(loop_bounds(8)))]
pub fn ferric_qwen3_c1_down_splitk8_merge_f32_r1(
    partials: &[f32],
    mut output: WriteOnlyDisjointSlice<f32, Index1D>,
) {
    if partials.len() != 8 * 4096
        || output.len() < 4096
        || output.len() > 32 * 4096
        || thread::grid_dim_x() != 64
        || thread::block_dim_x() != 64
    {
        fe2o3_device::trap();
    }
    let Ok(view) = StridedReadView2D::from_shared_slice(partials, 0, 8, 4096, 4096) else {
        fe2o3_device::trap();
    };
    let invocation = thread::index_1d();
    let column = invocation.get();
    if column < 4096 {
    } else {
        fe2o3_device::trap();
    }
    let column = column as u16 as usize;
    let mut sum = 0.0_f32;
    let mut finite = true;
    let mut partition = 0_usize;
    while partition < 8 {
        let value = view.load_or(partition, column, f32::INFINITY);
        sum += value;
        finite &= value.is_finite() & sum.is_finite();
        partition += 1;
    }
    if !finite || !output.write(invocation, sum) {
        fe2o3_device::trap();
    }
}
