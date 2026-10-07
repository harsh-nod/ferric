use fe2o3_device::{
    Bf16, Gfx950Subgroup, Index1D, RowStriped2D, StridedReadView2D, WriteOnlyDisjointSlice, kernel,
    thread,
};

/// Fixed TP1/C1 N12288/K4096 gate/up pair, with independent strict accumulators.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [12288, 1, 1]), control_flow(loop_bounds(32)))]
pub fn ferric_qwen3_c1_gate_up_packed_u32_bf16_v8(
    a: &[u32],
    gate_weights: &[u32],
    up_weights: &[u32],
    mut gate_output: WriteOnlyDisjointSlice<u16, RowStriped2D<Index1D, 64, 1>>,
    mut up_output: WriteOnlyDisjointSlice<u16, RowStriped2D<Index1D, 64, 1>>,
) {
    if a.len() != 2048
        || gate_weights.len() != 12288 * 2048
        || up_weights.len() != 12288 * 2048
        || gate_output.len() != 12288
        || up_output.len() != 12288
        || thread::launch_extent_1d() != 12288 * 64
    {
        fe2o3_device::trap();
    }
    let Ok(left_view) = StridedReadView2D::from_shared_slice(a, 0, 1, 2048, 2048) else {
        fe2o3_device::trap();
    };
    let Ok(gate_view) = StridedReadView2D::from_shared_slice(gate_weights, 0, 12288, 2048, 2048)
    else {
        fe2o3_device::trap();
    };
    let Ok(up_view) = StridedReadView2D::from_shared_slice(up_weights, 0, 12288, 2048, 2048) else {
        fe2o3_device::trap();
    };
    let invocation = thread::index_1d();
    let raw = invocation.get();
    let element = thread::block_idx_x() as usize;
    let lane = raw % 64;
    if element < 12288 {
    } else {
        fe2o3_device::trap();
    }
    let column = element as u16 as usize;
    let subgroup = Gfx950Subgroup::current();
    let mut gate_partial = 0.0_f32;
    let mut up_partial = 0.0_f32;
    let mut gate_finite = true;
    let mut up_finite = true;
    let mut group = 0_usize;
    while group < 32 {
        let packed_inner = group * 64 + lane;
        let left_bits = left_view.load_or(0, packed_inner, 0);
        let gate_bits = gate_view.load_or(column, packed_inner, 0);
        let up_bits = up_view.load_or(column, packed_inner, 0);
        {
            let left = Bf16::from_bits(left_bits as u16).to_f32();
            let gate = Bf16::from_bits(gate_bits as u16).to_f32();
            let gate_product = left * gate;
            gate_partial += gate_product;
            gate_finite &= gate_product.is_finite() & gate_partial.is_finite();
            let up = Bf16::from_bits(up_bits as u16).to_f32();
            let up_product = left * up;
            up_partial += up_product;
            up_finite &= up_product.is_finite() & up_partial.is_finite();
        }
        {
            let left = Bf16::from_bits((left_bits >> 16) as u16).to_f32();
            let gate = Bf16::from_bits((gate_bits >> 16) as u16).to_f32();
            let gate_product = left * gate;
            gate_partial += gate_product;
            gate_finite &= gate_product.is_finite() & gate_partial.is_finite();
            let up = Bf16::from_bits((up_bits >> 16) as u16).to_f32();
            let up_product = left * up;
            up_partial += up_product;
            up_finite &= up_product.is_finite() & up_partial.is_finite();
        }
        group += 1;
    }
    // Both collectives retain all lanes and each output's original XOR tree.
    let gate_sum = subgroup.reduce_sum_f32::<64>(gate_partial);
    let up_sum = subgroup.reduce_sum_f32::<64>(up_partial);
    let gate_narrowed = Bf16::from_f32(gate_sum);
    let up_narrowed = Bf16::from_f32(up_sum);
    if !gate_finite
        || !up_finite
        || !gate_sum.is_finite()
        || !up_sum.is_finite()
        || !gate_narrowed.is_finite()
        || !up_narrowed.is_finite()
    {
        fe2o3_device::trap();
    }
    if lane == 0 {
        let Some(stripe) = invocation.checked_row_striped_2d::<64, 1>() else {
            fe2o3_device::trap();
        };
        if !gate_output.write_row_striped_2d(&stripe, 0, 12288, 1, 1, gate_narrowed.to_bits()) {
            fe2o3_device::trap();
        }
        if !up_output.write_row_striped_2d(&stripe, 0, 12288, 1, 1, up_narrowed.to_bits()) {
            fe2o3_device::trap();
        }
    }
}
