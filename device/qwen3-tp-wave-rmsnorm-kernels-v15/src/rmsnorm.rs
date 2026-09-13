#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)]

//! Pure width4096 RMSNorm with a different FP32 sum association from v1.
//! Source only: no numerical equivalence, artifact or launch authority.

use fe2o3_device::{
    Bf16, Index1D, Math, RowStriped2D, Wave64, WaveLane, WriteOnlyDisjointSlice,
    gfx950::Gfx950Subgroup, kernel, memory, thread,
};

pub const QWEN3_RMSNORM_EPSILON_V1: f32 = 1e-6_f32;

#[allow(clippy::too_many_arguments, clippy::len_zero)]
#[kernel(
    typed,
    launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [32, 1, 1]),
    control_flow(loop_bounds(64, 64))
)]
pub fn ferric_qwen3_tp_batch32_wave_rmsnorm_bf16_v15(
    input_bf16: &[u16],
    residual_bf16: &[u16],
    weight_bf16: &[u16],
    fused_residual_bf16: WriteOnlyDisjointSlice<u16, RowStriped2D<Index1D, 64, 64>>,
    mut normalized_bf16: WriteOnlyDisjointSlice<u16, RowStriped2D<Index1D, 64, 64>>,
    rows: u32,
    width: u32,
    epsilon: f32,
    behavior: u32,
) {
    let shape_valid = rows != 0 && rows <= 32 && width == 4_096 && behavior == 0;
    let elements = rows as usize * width as usize;
    let required_lengths = input_bf16.len() == elements
        && weight_bf16.len() == width as usize
        && normalized_bf16.len() == elements;
    let auxiliary_lengths = residual_bf16.len() == 0 && fused_residual_bf16.len() == 0;
    let exact_grid =
        thread::grid_dim_x() == rows && thread::grid_dim_y() == 1 && thread::grid_dim_z() == 1;
    if !shape_valid
        || !required_lengths
        || !auxiliary_lengths
        || epsilon != QWEN3_RMSNORM_EPSILON_V1
        || !exact_grid
    {
        fe2o3_device::trap();
    }

    let row = thread::block_idx_x() as usize;
    let lane = WaveLane::<Wave64>::current();
    let lane_index = lane.into_lane_id() as usize;
    let row_base = row * width as usize;
    let subgroup = Gfx950Subgroup::current();
    let mut partial = 0.0_f32;
    let mut finite = true;
    let mut component = 0_usize;
    while component < 64 {
        let column = lane_index + component * 64;
        let index = row_base + column;
        let input = Bf16::from_bits(memory::volatile_load(input_bf16, index));
        let input_value = input.to_f32();
        let square = input_value * input_value;
        let next_sum = partial + square;
        finite &= input.is_finite() & square.is_finite() & next_sum.is_finite();
        partial = next_sum;
        component += 1;
    }
    // Every physical lane reaches both collectives, including invalid inputs.
    let sum = subgroup.reduce_sum_f32::<64>(partial);
    let invalid = if finite { 0.0_f32 } else { 1.0_f32 };
    let any_invalid = subgroup.reduce_max_f32::<64>(invalid);
    if any_invalid != 0.0 || !sum.is_finite() {
        fe2o3_device::trap();
    }
    let mean_square = sum / width as f32;
    let stabilized = mean_square + epsilon;
    if !mean_square.is_finite() || !stabilized.is_finite() || stabilized <= 0.0 {
        fe2o3_device::trap();
    }
    let denominator = Math::current().sqrt_f32(stabilized);
    if !denominator.is_finite() || denominator <= 0.0 {
        fe2o3_device::trap();
    }
    let inverse_rms = 1.0_f32 / denominator;
    if !inverse_rms.is_finite() {
        fe2o3_device::trap();
    }
    let Some(output_row) = thread::index_1d().checked_row_striped_2d::<64, 64>() else {
        fe2o3_device::trap();
    };

    let mut component = 0_usize;
    while component < 64 {
        let column = lane_index + component * 64;
        if column < width as usize {
            let index = row_base + column;
            let input = Bf16::from_bits(memory::volatile_load(input_bf16, index));
            if !input.is_finite() {
                fe2o3_device::trap();
            }
            let input_value = input.to_f32();
            let normalized_input = input_value;
            let normalized = normalized_input * inverse_rms;
            let weight = Bf16::from_bits(memory::volatile_load(weight_bf16, column));
            if !weight.is_finite() {
                fe2o3_device::trap();
            }
            let weighted = normalized * weight.to_f32();
            if !normalized.is_finite() || !weighted.is_finite() {
                fe2o3_device::trap();
            }
            let narrowed_weighted = Bf16::from_f32(weighted);
            if !narrowed_weighted.is_finite() {
                fe2o3_device::trap();
            }
            if !normalized_bf16.write_row_striped_2d(
                &output_row,
                component,
                rows as usize,
                width as usize,
                width as usize,
                narrowed_weighted.to_bits(),
            ) {
                fe2o3_device::trap();
            }
        }
        component += 1;
    }
}
