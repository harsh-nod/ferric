use fe2o3_device::{
    Bf16, Bf16MfmaAMatrix, Bf16MfmaBMatrix, DeviceMatrix, F32AccumulatorFragment, Index1D, Tiled2D,
    Wave64, WaveLane, WriteOnlyDisjointSlice, kernel, thread,
};

/// Two M16 tiles share one N16 workgroup; both output views are independently disjoint.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [768, 1, 1]), control_flow(loop_bounds(256)))]
#[allow(clippy::too_many_arguments)]
pub fn ferric_qwen3_prefill32_m2_gate_up_bf16_r1(
    a: &[u16],
    weights_kn: &[u16],
    mut output_low: WriteOnlyDisjointSlice<u16, Tiled2D<Index1D, 64, 16, 16, 4>>,
    mut output_high: WriteOnlyDisjointSlice<u16, Tiled2D<Index1D, 64, 16, 16, 4>>,
    rows: u32,
    n: u32,
    k: u32,
    world_size: u32,
    projection: u32,
) {
    if rows != 32
        || n != 12288
        || k != 4096
        || world_size != 1
        || !(projection == 4 || projection == 5)
    {
        fe2o3_device::trap();
    }
    if a.len() != 32 * 4096
        || weights_kn.len() != 4096 * 12288
        || output_low.len() != 16 * 12288
        || output_high.len() != 16 * 12288
    {
        fe2o3_device::trap();
    }
    if thread::grid_dim_x() != 768 || thread::block_dim_x() != 64 {
        fe2o3_device::trap();
    }
    let invocation = thread::index_1d();
    let raw = invocation.get();
    let group = raw / 64;
    if group < 768 {
    } else {
        fe2o3_device::trap();
    }
    // The guard makes this narrowing lossless and exposes a bounded column offset.
    let group = group as u16 as usize;
    let lane = WaveLane::<Wave64>::current();
    let Ok(left) = Bf16MfmaAMatrix::row_major(a, 0, 32, 4096, 4096) else {
        fe2o3_device::trap();
    };
    let Ok(right) = Bf16MfmaBMatrix::row_major(weights_kn, 0, 4096, 12288, 12288) else {
        fe2o3_device::trap();
    };
    let matrix = DeviceMatrix::current();
    let mut accumulator_low = F32AccumulatorFragment::zero(&lane);
    let mut accumulator_high = F32AccumulatorFragment::zero(&lane);
    let mut step = 0_usize;
    while step < 256 {
        let a_low = left.load_m16k16(&lane, 0, step * 16);
        let a_high = left.load_m16k16(&lane, 16, step * 16);
        let b_low = right.load_k16n16(&lane, step * 16, group * 16);
        let b_high = right.load_k16n16(&lane, step * 16, group * 16);
        accumulator_low = matrix.multiply_accumulate(a_low, b_low, accumulator_low);
        accumulator_high = matrix.multiply_accumulate(a_high, b_high, accumulator_high);
        step += 1;
    }
    let [low_0, low_1, low_2, low_3] = accumulator_low.into_values();
    let [high_0, high_1, high_2, high_3] = accumulator_high.into_values();
    let Some(tile) = invocation.checked_tiled_2d::<64, 16, 16, 4>() else {
        fe2o3_device::trap();
    };
    let value = Bf16::from_f32(low_0);
    if !low_0.is_finite()
        || !value.is_finite()
        || !output_low.write_tiled_2d(&tile, 0, 16, 12288, 12288, value.to_bits())
    {
        fe2o3_device::trap();
    }
    let value = Bf16::from_f32(low_1);
    if !low_1.is_finite()
        || !value.is_finite()
        || !output_low.write_tiled_2d(&tile, 1, 16, 12288, 12288, value.to_bits())
    {
        fe2o3_device::trap();
    }
    let value = Bf16::from_f32(low_2);
    if !low_2.is_finite()
        || !value.is_finite()
        || !output_low.write_tiled_2d(&tile, 2, 16, 12288, 12288, value.to_bits())
    {
        fe2o3_device::trap();
    }
    let value = Bf16::from_f32(low_3);
    if !low_3.is_finite()
        || !value.is_finite()
        || !output_low.write_tiled_2d(&tile, 3, 16, 12288, 12288, value.to_bits())
    {
        fe2o3_device::trap();
    }
    let value = Bf16::from_f32(high_0);
    if !high_0.is_finite()
        || !value.is_finite()
        || !output_high.write_tiled_2d(&tile, 0, 16, 12288, 12288, value.to_bits())
    {
        fe2o3_device::trap();
    }
    let value = Bf16::from_f32(high_1);
    if !high_1.is_finite()
        || !value.is_finite()
        || !output_high.write_tiled_2d(&tile, 1, 16, 12288, 12288, value.to_bits())
    {
        fe2o3_device::trap();
    }
    let value = Bf16::from_f32(high_2);
    if !high_2.is_finite()
        || !value.is_finite()
        || !output_high.write_tiled_2d(&tile, 2, 16, 12288, 12288, value.to_bits())
    {
        fe2o3_device::trap();
    }
    let value = Bf16::from_f32(high_3);
    if !high_3.is_finite()
        || !value.is_finite()
        || !output_high.write_tiled_2d(&tile, 3, 16, 12288, 12288, value.to_bits())
    {
        fe2o3_device::trap();
    }
}
