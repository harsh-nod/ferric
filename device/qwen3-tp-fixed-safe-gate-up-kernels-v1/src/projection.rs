use fe2o3_device::{
    Bf16, Gfx950Subgroup, Index1D, RowStriped2D, StridedReadView2D, WriteOnlyDisjointSlice,
    kernel, thread,
};

/// Separate TP1/C1 gate/up experiment; no Q/K/V or generic-N routing.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [12288, 1, 1]), control_flow(loop_bounds(32)))]
#[allow(clippy::too_many_arguments)]
pub fn ferric_qwen3_c1_gate_up_fixed_safe_u32_bf16_r1(
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
        || n != 12_288
        || k != 4096
        || world_size != 1
        || !(projection == 4 || projection == 5)
        || a.len() < 2048
        || a.len() > 32 * 2048
        || weights.len() != 12_288 * 2048
        || output.len() < 12_288
        || output.len() > 32 * 12_288
        || thread::launch_extent_1d() != 12_288 * 64
    {
        fe2o3_device::trap();
    }
    let Ok(left_view) = StridedReadView2D::from_shared_slice(a, 0, 1, 2048, 2048) else {
        fe2o3_device::trap();
    };
    let Ok(right_view) = StridedReadView2D::from_shared_slice(weights, 0, 12_288, 2048, 2048) else {
        fe2o3_device::trap();
    };
    let invocation = thread::index_1d();
    let lane = invocation.get() % 64;
    let column = thread::block_idx_x() as usize;
    if column >= 12_288 {
        fe2o3_device::trap();
    }
    let subgroup = Gfx950Subgroup::current();
    let mut partial = 0.0_f32;
    let mut group = 0_usize;
    while group < 32 {
        let packed_inner = group * 64 + lane;
        // Total checked reads reconverge before the collective. Admission and
        // fixed coordinate bounds make the fallback unreachable for valid input.
        let left_bits = left_view.load_or(0, packed_inner, 0);
        let right_bits = right_view.load_or(column, packed_inner, 0);
        {
            let left = Bf16::from_bits(left_bits as u16).to_f32();
            let right = Bf16::from_bits(right_bits as u16).to_f32();
            let product = left * right;
            partial += product;
        }
        {
            let left = Bf16::from_bits((left_bits >> 16) as u16).to_f32();
            let right = Bf16::from_bits((right_bits >> 16) as u16).to_f32();
            let product = left * right;
            partial += product;
        }
        group += 1;
    }
    // With strict FP32 addition a nonfinite product or partial cannot recover.
    // Keep rejection after the collective so no lane exits before reduction.
    let finite = partial.is_finite();
    let sum = subgroup.reduce_sum_f32::<64>(partial);
    let narrowed = Bf16::from_f32(sum);
    if !finite || !sum.is_finite() || !narrowed.is_finite() {
        fe2o3_device::trap();
    }
    if lane == 0 {
        let Some(stripe) = invocation.checked_row_striped_2d::<64, 1>() else {
            fe2o3_device::trap();
        };
        if !output.write_row_striped_2d(&stripe, 0, 12_288, 1, 1, narrowed.to_bits()) {
            fe2o3_device::trap();
        }
    }
}
