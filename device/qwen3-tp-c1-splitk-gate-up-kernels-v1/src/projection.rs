use fe2o3_device::{
    Bf16, Bf16MfmaAMatrix, Bf16MfmaBMatrix, DeviceMatrix, F32AccumulatorFragment, Index1D,
    RowStriped2D, StridedReadView2D, Wave64, WaveLane, WriteOnlyDisjointSlice, kernel, thread,
};

/// Four K1024 partitions of a fixed TP1/C1 gate or up projection.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [3072, 1, 1]), control_flow(loop_bounds(64)))]
#[allow(clippy::too_many_arguments)]
pub fn ferric_qwen3_c1_gate_up_splitk4_mfma_partial_f32_r1(
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
        || n != 12288
        || k != 4096
        || world_size != 1
        || !(projection == 4 || projection == 5)
        || a.len() < 4096
        || a.len() > 32 * 4096
        || weights_kn.len() != 4096 * 12288
        || partials.len() != 4 * 12288
        || thread::grid_dim_x() != 3072
        || thread::block_dim_x() != 64
    {
        fe2o3_device::trap();
    }
    let invocation = thread::index_1d();
    let raw = invocation.get();
    let group = raw / 64;
    let partition = group / 768;
    let tile_column = group % 768;
    if partition < 4 && tile_column < 768 {
    } else {
        fe2o3_device::trap();
    }
    let partition = partition as u8 as usize;
    // Unlike the down projection, 768 column tiles do not fit in u8.
    let tile_column = tile_column as u16 as usize;
    let lane = WaveLane::<Wave64>::current();
    let Ok(left) = Bf16MfmaAMatrix::row_major(a, 0, 1, 4096, 4096) else {
        fe2o3_device::trap();
    };
    let Ok(right) = Bf16MfmaBMatrix::row_major(weights_kn, 0, 4096, 12288, 12288) else {
        fe2o3_device::trap();
    };
    let matrix = DeviceMatrix::current();
    let mut accumulator = F32AccumulatorFragment::zero(&lane);
    let mut step = 0_usize;
    while step < 64 {
        let reduction_base = partition * 1024 + step * 16;
        let a_fragment = left.load_m16k16(&lane, 0, reduction_base);
        let b_fragment = right.load_k16n16(&lane, reduction_base, tile_column * 16);
        accumulator = matrix.multiply_accumulate(a_fragment, b_fragment, accumulator);
        step += 1;
    }
    let [value_0, _, _, _] = accumulator.into_values();
    // Only row zero is live; it occupies value_0 in lanes 0..15.
    if raw % 64 < 16 {
        let Some(stripe) = invocation.checked_row_striped_2d::<64, 1>() else {
            fe2o3_device::trap();
        };
        if !value_0.is_finite() || !partials.write_row_striped_2d(&stripe, 0, 3072, 16, 16, value_0)
        {
            fe2o3_device::trap();
        }
    }
}

/// Merge partitions in increasing order, then round once to BF16.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [192, 1, 1]), control_flow(loop_bounds(4)))]
pub fn ferric_qwen3_c1_gate_up_splitk4_merge_bf16_r1(
    partials: &[f32],
    mut output: WriteOnlyDisjointSlice<u16, Index1D>,
) {
    if partials.len() != 4 * 12288
        || output.len() < 12288
        || output.len() > 32 * 12288
        || thread::grid_dim_x() != 192
        || thread::block_dim_x() != 64
    {
        fe2o3_device::trap();
    }
    let Ok(view) = StridedReadView2D::from_shared_slice(partials, 0, 4, 12288, 12288) else {
        fe2o3_device::trap();
    };
    let invocation = thread::index_1d();
    let column = invocation.get();
    if column < 12288 {
    } else {
        fe2o3_device::trap();
    }
    let column = column as u16 as usize;
    let mut sum = 0.0_f32;
    let mut finite = true;
    let mut partition = 0_usize;
    while partition < 4 {
        let value = view.load_or(partition, column, f32::INFINITY);
        sum += value;
        finite &= value.is_finite() & sum.is_finite();
        partition += 1;
    }
    let value = Bf16::from_f32(sum);
    if !finite || !value.is_finite() || !output.write(invocation, value.to_bits()) {
        fe2o3_device::trap();
    }
}
