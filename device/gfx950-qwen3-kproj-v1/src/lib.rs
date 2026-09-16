#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)] // The kernel attribute emits an internal helper module.

//! Actual-weight Qwen3 layer-zero key projection, fixed batch-one baseline.
//!
//! Each invocation owns one output row. BF16 bit storage is widened exactly
//! before 1024 explicitly fused FP32 multiply-adds. This scalar baseline is
//! not a cooperative GEMV, an optimized kernel, model inference, or production
//! execution authority. No weight-layout rewrite or native-IR patch is used.

use fe2o3_device::{Bf16, Math, WriteOnlyDisjointSlice, kernel, thread};

pub const KERNEL_SYMBOL: &str = "ferric_gfx950_qwen3_kproj_v1";
pub const INPUTS: usize = 1024;
pub const OUTPUTS: usize = 1024;
pub const WEIGHTS: usize = OUTPUTS * INPUTS;
pub const WORKGROUP: [u32; 3] = [128, 1, 1];
pub const AQL_GRID: [u32; 3] = [1024, 1, 1];
pub const EXPLICIT_KERNARG_BYTES: usize = 48;

/// Compute row-major `model.layers.0.self_attn.k_proj.weight * input`.
///
/// The host binds the exact checkpoint tensor and freezes an error policy
/// before dispatch. The kernel itself receives only bounded slices, not a
/// model identifier or arbitrary runtime task schema.
#[kernel(
    typed,
    launch(required = [128, 1, 1], max = [128, 1, 1], max_grid = [8, 1, 1]),
    control_flow(loop_bounds(1024))
)]
pub fn ferric_gfx950_qwen3_kproj_v1(
    inputs: &[u16],
    weights: &[u16],
    mut output: WriteOnlyDisjointSlice<f32>,
) {
    if inputs.len() != INPUTS || weights.len() != WEIGHTS || output.len() != OUTPUTS {
        fe2o3_device::trap();
    }
    let index = thread::index_1d();
    let row = index.get();
    if row >= OUTPUTS {
        return;
    }
    let math = Math::current();
    let mut total = 0.0f32;
    let mut column = 0usize;
    while column < INPUTS {
        let weight = Bf16::from_bits(weights[row * INPUTS + column]).to_f32();
        let value = Bf16::from_bits(inputs[column]).to_f32();
        total = math.mul_add_f32(weight, value, total);
        column += 1;
    }
    if !output.write(index, total) {
        fe2o3_device::trap();
    }
}
