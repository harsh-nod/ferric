#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)] // The kernel attribute emits an internal helper module.

//! Coalesced batch-one key projection using one full wave per output row.
//!
//! This is an engineering candidate, not a measured performance claim. Read
//! operands remain BF16 checkpoint bytes; arithmetic and outputs are FP32.

use fe2o3_device::{
    Bf16, Gfx950Subgroup, Index1D, Math, RowStriped2D, StridedReadView2D, WriteOnlyDisjointSlice,
    kernel, thread,
};

pub const KERNEL_SYMBOL: &str = "ferric_gfx950_qwen3_kproj_wave64_v1";
pub const INPUTS: usize = 1024;
pub const OUTPUTS: usize = 1024;
pub const WEIGHTS: usize = OUTPUTS * INPUTS;
pub const LANES_PER_ROW: usize = 64;
pub const TERMS_PER_LANE: usize = INPUTS / LANES_PER_ROW;
pub const WORKGROUP: [u32; 3] = [128, 1, 1];
pub const AQL_GRID: [u32; 3] = [65536, 1, 1];
pub const EXPLICIT_KERNARG_BYTES: usize = 48;

/// Compute the checkpoint projection with contiguous per-wave weight loads.
#[kernel(
    typed,
    launch(required = [128, 1, 1], max = [128, 1, 1], max_grid = [512, 1, 1]),
    control_flow(loop_bounds(16))
)]
pub fn ferric_gfx950_qwen3_kproj_wave64_v1(
    inputs: &[u16],
    weights: &[u16],
    mut output: WriteOnlyDisjointSlice<f32, RowStriped2D<Index1D, 64, 1>>,
) {
    if inputs.len() != INPUTS || weights.len() != WEIGHTS || output.len() != OUTPUTS {
        fe2o3_device::trap();
    }
    let input_view =
        if let Ok(view) = StridedReadView2D::from_shared_slice(inputs, 0, 1, INPUTS, INPUTS) {
            view
        } else {
            fe2o3_device::trap()
        };
    let weight_view = if let Ok(view) =
        StridedReadView2D::from_shared_slice(weights, 0, OUTPUTS, INPUTS, INPUTS)
    {
        view
    } else {
        fe2o3_device::trap()
    };
    let index = thread::index_1d();
    let raw = index.get();
    let row = raw / LANES_PER_ROW;
    let lane = raw % LANES_PER_ROW;
    let math = Math::current();
    let mut partial = 0.0f32;
    let mut term = 0usize;
    // Total checked loads use zero for out-of-view coordinates, never a
    // lane-dependent panic before the collective. Adjacent lanes load
    // adjacent columns in the checkpoint's unmodified row-major layout.
    while term < TERMS_PER_LANE {
        let column = lane + term * LANES_PER_ROW;
        let weight = Bf16::from_bits(weight_view.load_or(row, column, 0)).to_f32();
        let value = Bf16::from_bits(input_view.load_or(0, column, 0)).to_f32();
        partial = math.mul_add_f32(weight, value, partial);
        term += 1;
    }
    let subgroup = Gfx950Subgroup::current();
    let total = subgroup.reduce_sum_f32::<64>(partial);
    // Convergence ends before leader selection. RowStriped2D, not the integer
    // row, carries write authority; columns=1 permits only lane zero to write.
    if lane == 0 {
        if let Some(stripe) = index.checked_row_striped_2d::<64, 1>() {
            if !output.write_row_striped_2d(&stripe, 0, OUTPUTS, 1, 1, total) {
                fe2o3_device::trap();
            }
        } else {
            fe2o3_device::trap();
        }
    }
}
