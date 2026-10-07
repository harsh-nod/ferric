use fe2o3_device::{
    Bf16MfmaAMatrix, Bf16MfmaBMatrix, DeviceMatrix, F32AccumulatorFragment, Index1D, Tiled2D,
    Wave64, WaveLane, WriteOnlyDisjointSlice, kernel, thread,
};

/// Fixed-shape control: the original ascending K16 updates and FP32 output.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [512, 1, 1]), control_flow(loop_bounds(768)))]
#[allow(clippy::too_many_arguments)]
pub fn ferric_qwen3_prefill32_down_k16_control_f32_r1(
    a: &[u16],
    weights_kn: &[u16],
    mut output: WriteOnlyDisjointSlice<f32, Tiled2D<Index1D, 64, 16, 16, 4>>,
    rows: u32,
    n: u32,
    k: u32,
    world_size: u32,
    projection: u32,
) {
    if rows != 32 || n != 4096 || k != 12288 || world_size != 1 || projection != 2 {
        fe2o3_device::trap();
    }
    if a.len() != 32 * 12288
        || weights_kn.len() != 12288 * 4096
        || output.len() != 32 * 4096
    {
        fe2o3_device::trap();
    }
    if thread::grid_dim_x() != 512 || thread::block_dim_x() != 64 {
        fe2o3_device::trap();
    }
    let invocation = thread::index_1d();
    let group = invocation.get() / 64;
    if group < 512 {
    } else {
        fe2o3_device::trap();
    }
    let tile_row = (group / 256) as u8 as usize;
    let tile_column = (group % 256) as u8 as usize;
    let lane = WaveLane::<Wave64>::current();
    let Ok(left) = Bf16MfmaAMatrix::row_major(a, 0, 32, 12288, 12288) else {
        fe2o3_device::trap();
    };
    let Ok(right) = Bf16MfmaBMatrix::row_major(weights_kn, 0, 12288, 4096, 4096) else {
        fe2o3_device::trap();
    };
    let matrix = DeviceMatrix::current();
    let mut accumulator = F32AccumulatorFragment::zero(&lane);
    let mut step = 0_usize;
    while step < 768 {
        let a_fragment = left.load_m16k16(&lane, tile_row * 16, step * 16);
        let b_fragment = right.load_k16n16(&lane, step * 16, tile_column * 16);
        accumulator = matrix.multiply_accumulate(a_fragment, b_fragment, accumulator);
        step += 1;
    }
    let [value_0, value_1, value_2, value_3] = accumulator.into_values();
    let Some(tile) = invocation.checked_tiled_2d::<64, 16, 16, 4>() else {
        fe2o3_device::trap();
    };
    if !value_0.is_finite()
        || !output.write_tiled_2d(&tile, 0, 32, 4096, 4096, value_0)
    {
        fe2o3_device::trap();
    }
    if !value_1.is_finite()
        || !output.write_tiled_2d(&tile, 1, 32, 4096, 4096, value_1)
    {
        fe2o3_device::trap();
    }
    if !value_2.is_finite()
        || !output.write_tiled_2d(&tile, 2, 32, 4096, 4096, value_2)
    {
        fe2o3_device::trap();
    }
    if !value_3.is_finite()
        || !output.write_tiled_2d(&tile, 3, 32, 4096, 4096, value_3)
    {
        fe2o3_device::trap();
    }
}

/// Load adjacent K16 fragments before two ordered updates of one accumulator.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [512, 1, 1]), control_flow(loop_bounds(384)))]
#[allow(clippy::too_many_arguments)]
pub fn ferric_qwen3_prefill32_down_k16_paired_f32_r1(
    a: &[u16],
    weights_kn: &[u16],
    mut output: WriteOnlyDisjointSlice<f32, Tiled2D<Index1D, 64, 16, 16, 4>>,
    rows: u32,
    n: u32,
    k: u32,
    world_size: u32,
    projection: u32,
) {
    if rows != 32 || n != 4096 || k != 12288 || world_size != 1 || projection != 2 {
        fe2o3_device::trap();
    }
    if a.len() != 32 * 12288
        || weights_kn.len() != 12288 * 4096
        || output.len() != 32 * 4096
    {
        fe2o3_device::trap();
    }
    if thread::grid_dim_x() != 512 || thread::block_dim_x() != 64 {
        fe2o3_device::trap();
    }
    let invocation = thread::index_1d();
    let group = invocation.get() / 64;
    if group < 512 {
    } else {
        fe2o3_device::trap();
    }
    let tile_row = (group / 256) as u8 as usize;
    let tile_column = (group % 256) as u8 as usize;
    let lane = WaveLane::<Wave64>::current();
    let Ok(left) = Bf16MfmaAMatrix::row_major(a, 0, 32, 12288, 12288) else {
        fe2o3_device::trap();
    };
    let Ok(right) = Bf16MfmaBMatrix::row_major(weights_kn, 0, 12288, 4096, 4096) else {
        fe2o3_device::trap();
    };
    let matrix = DeviceMatrix::current();
    let mut accumulator = F32AccumulatorFragment::zero(&lane);
    let mut pair = 0_usize;
    while pair < 384 {
        let reduction_base = pair * 32;
        let next_reduction_base = reduction_base + 16;
        let a_fragment = left.load_m16k16(&lane, tile_row * 16, reduction_base);
        let b_fragment = right.load_k16n16(&lane, reduction_base, tile_column * 16);
        let next_a_fragment = left.load_m16k16(&lane, tile_row * 16, next_reduction_base);
        let next_b_fragment = right.load_k16n16(&lane, next_reduction_base, tile_column * 16);
        accumulator = matrix.multiply_accumulate(a_fragment, b_fragment, accumulator);
        accumulator = matrix.multiply_accumulate(next_a_fragment, next_b_fragment, accumulator);
        pair += 1;
    }
    let [value_0, value_1, value_2, value_3] = accumulator.into_values();
    let Some(tile) = invocation.checked_tiled_2d::<64, 16, 16, 4>() else {
        fe2o3_device::trap();
    };
    if !value_0.is_finite()
        || !output.write_tiled_2d(&tile, 0, 32, 4096, 4096, value_0)
    {
        fe2o3_device::trap();
    }
    if !value_1.is_finite()
        || !output.write_tiled_2d(&tile, 1, 32, 4096, 4096, value_1)
    {
        fe2o3_device::trap();
    }
    if !value_2.is_finite()
        || !output.write_tiled_2d(&tile, 2, 32, 4096, 4096, value_2)
    {
        fe2o3_device::trap();
    }
    if !value_3.is_finite()
        || !output.write_tiled_2d(&tile, 3, 32, 4096, 4096, value_3)
    {
        fe2o3_device::trap();
    }
}
