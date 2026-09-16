//! Draft-only pure RMSNorm admission with unchanged generic arithmetic.
#![allow(clippy::let_and_return)]

use fe2o3_device::{
    Bf16, Index1D, Math, RowStriped2D, Wave64, WaveLane, WriteOnlyDisjointSlice, kernel, memory,
    thread,
};
const QWEN3_RMSNORM_BEHAVIOR_PURE_V1: u32 = 0;
const QWEN3_RMSNORM_BEHAVIOR_RESIDUAL_FUSED_V1: u32 = 1;
const QWEN3_RMSNORM_MAX_GRID_WORKGROUPS_V1: u32 = 512;
const QWEN3_RMSNORM_EPSILON_V1: f32 = 1e-6_f32;

/// Computes RMSNorm with one wave64 workgroup per row.
///
/// Inputs and outputs retain physical `u16` BF16 carriers. Pure mode requires
/// genuinely empty residual and fused-output slices. The stronger entry guard
/// excludes fused mode; the original arithmetic remains intact below it.
/// Every lane owns columns `lane + component * 64` in its row.
#[allow(clippy::too_many_arguments, clippy::len_zero)]
#[kernel(
    typed,
    launch(
        required = [64, 1, 1],
        max = [64, 1, 1],
        max_grid = [512, 1, 1]
    ),
    control_flow(loop_bounds(4096, 64))
)]
pub fn ferric_qwen3_draft_batch32_rmsnorm_v10(
    input_bf16: &[u16],
    residual_bf16: &[u16],
    weight_bf16: &[u16],
    mut fused_residual_bf16: WriteOnlyDisjointSlice<u16, RowStriped2D<Index1D, 64, 64>>,
    mut normalized_bf16: WriteOnlyDisjointSlice<u16, RowStriped2D<Index1D, 64, 64>>,
    rows: u32,
    width: u32,
    epsilon: f32,
    behavior: u32,
) {
    if behavior != 0 || !((width == 1024 && rows <= 32) || (width == 128 && rows <= 512)) {
        fe2o3_device::trap();
    }
    let pure_mode = behavior == QWEN3_RMSNORM_BEHAVIOR_PURE_V1;
    let fused_mode = behavior == QWEN3_RMSNORM_BEHAVIOR_RESIDUAL_FUSED_V1;
    let pure_width = width == 128 || width == 1_024 || width == 4_096;
    let fused_width = width == 1_024 || width == 4_096;
    let shape_valid = rows != 0
        && rows <= QWEN3_RMSNORM_MAX_GRID_WORKGROUPS_V1
        && ((pure_mode && pure_width) || (fused_mode && fused_width));
    let elements = rows as usize * width as usize;
    let required_lengths = input_bf16.len() == elements
        && weight_bf16.len() == width as usize
        && normalized_bf16.len() == elements;
    let auxiliary_lengths = (pure_mode
        && residual_bf16.len() == 0
        && fused_residual_bf16.len() == 0)
        || (fused_mode && residual_bf16.len() == elements && fused_residual_bf16.len() == elements);
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
    let mut sum = 0.0_f32;
    let mut column = 0_usize;
    while column < width as usize {
        let index = row_base + column;
        let input = Bf16::from_bits(memory::volatile_load(input_bf16, index));
        let input_value = input.to_f32();
        let normalized_input = if fused_mode {
            let residual = Bf16::from_bits(memory::volatile_load(residual_bf16, index));
            let fused = input_value + residual.to_f32();
            fused
        } else {
            input_value
        };
        let square = normalized_input * normalized_input;
        let next_sum = sum + square;
        sum = next_sum;
        column += 1;
    }
    if !sum.is_finite() {
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
            let normalized_input = if fused_mode {
                let residual = Bf16::from_bits(memory::volatile_load(residual_bf16, index));
                if !residual.is_finite() {
                    fe2o3_device::trap();
                }
                let fused = input_value + residual.to_f32();
                if !fused.is_finite() {
                    fe2o3_device::trap();
                }
                let narrowed_fused = Bf16::from_f32(fused);
                if !narrowed_fused.is_finite() {
                    fe2o3_device::trap();
                }
                if !fused_residual_bf16.write_row_striped_2d(
                    &output_row,
                    component,
                    rows as usize,
                    width as usize,
                    width as usize,
                    narrowed_fused.to_bits(),
                ) {
                    fe2o3_device::trap();
                }
                fused
            } else {
                input_value
            };
            let normalized = normalized_input * inverse_rms;
            let narrowed_normalized = Bf16::from_f32(normalized);
            if !narrowed_normalized.is_finite() {
                fe2o3_device::trap();
            }
            let weight = Bf16::from_bits(memory::volatile_load(weight_bf16, column));
            if !weight.is_finite() {
                fe2o3_device::trap();
            }
            let weighted = narrowed_normalized.to_f32() * weight.to_f32();
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
