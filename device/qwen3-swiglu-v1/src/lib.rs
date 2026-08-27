#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)] // The kernel macro emits an undocumented helper module.

//! Attributed Rust source for Ferric's exact Qwen3 SwiGLU device kernel.
//!
//! This source contract carries no artifact, launch, numerical-qualification,
//! or M1 authority. Production Worker V3 compilation remains fail-closed until
//! fe2o3 admits the required BF16, gfx942 OCML, and verifier/runtime joins.

use fe2o3_device::{Bf16, Blocked, DisjointSlice, Index1D, Math, kernel, thread};

/// Exact exported kernel symbol retained from the direct-LLVM implementation.
pub const QWEN3_SWIGLU_KERNEL_SYMBOL_V1: &str = "qwen3_swiglu_bf16_f32_v1";
/// Exact workgroup size in workitems.
pub const QWEN3_SWIGLU_WORKGROUP_V1: [u32; 3] = [256, 1, 1];
/// Consecutive elements owned by one workitem.
pub const QWEN3_SWIGLU_ELEMENTS_PER_WORKITEM_V1: usize = 8;
/// Consecutive elements covered by one workgroup before tail masking.
pub const QWEN3_SWIGLU_ELEMENTS_PER_WORKGROUP_V1: usize = 2_048;
/// Largest admitted one-dimensional grid in workgroups.
pub const QWEN3_SWIGLU_MAX_GRID_WORKGROUPS_V1: u32 = 12_288;
/// Explicit kernarg bytes: three pointer-plus-`usize` slice records.
pub const QWEN3_SWIGLU_EXPLICIT_KERNARG_BYTES_V1: usize = 48;
/// The fifteen distinct element extents induced by the 22 M1 role/bucket profiles.
pub const QWEN3_SWIGLU_ADMITTED_EXTENTS_V1: [usize; 15] = [
    3_072, 12_288, 24_576, 49_152, 61_440, 98_304, 110_592, 208_896, 393_216, 491_520, 1_572_864,
    3_145_728, 6_291_456, 12_582_912, 25_165_824,
];

/// Returns whether `elements` is one of the exact Ferric M1 SwiGLU extents.
#[must_use]
pub const fn qwen3_swiglu_extent_is_admitted_v1(elements: usize) -> bool {
    elements == 3_072
        || elements == 12_288
        || elements == 24_576
        || elements == 49_152
        || elements == 61_440
        || elements == 98_304
        || elements == 110_592
        || elements == 208_896
        || elements == 393_216
        || elements == 491_520
        || elements == 1_572_864
        || elements == 3_145_728
        || elements == 6_291_456
        || elements == 12_582_912
        || elements == 25_165_824
}

/// Applies stable FP32 SiLU to BF16 gate values, multiplies BF16 up values, and
/// narrows the product to BF16 round-to-nearest, ties-to-even.
///
/// `gate`, `up`, and `output` use the retained physical `u16` BF16 carrier ABI.
/// Each workitem owns exactly eight contiguous elements. Non-finite input or
/// intermediate values trap before the current store; earlier stores are not
/// rolled back.
#[kernel(
    typed,
    launch(
        required = [256, 1, 1],
        max = [256, 1, 1],
        max_grid = [12288, 1, 1]
    ),
    control_flow(loop_bounds(8))
)]
pub fn qwen3_swiglu_bf16_f32_v1(
    gate: &[u16],
    up: &[u16],
    mut output: DisjointSlice<u16, Blocked<Index1D, 1, 8>>,
) {
    let elements = gate.len();
    if !qwen3_swiglu_extent_is_admitted_v1(elements)
        || up.len() != elements
        || output.len() != elements
    {
        fe2o3_device::trap();
    }

    let workitem = thread::index_1d();
    let base = workitem.get() * QWEN3_SWIGLU_ELEMENTS_PER_WORKITEM_V1;
    let Some(output_block) = workitem.checked_block::<1, 8>() else {
        fe2o3_device::trap();
    };
    let math = Math::current();

    let mut component = 0;
    while component < QWEN3_SWIGLU_ELEMENTS_PER_WORKITEM_V1 {
        let index = base + component;
        if index >= elements {
            return;
        }

        let gate_value = Bf16::from_bits(gate[index]);
        let up_value = Bf16::from_bits(up[index]);
        if !gate_value.is_finite() || !up_value.is_finite() {
            fe2o3_device::trap();
        }

        let gate_f32 = gate_value.to_f32();
        let up_f32 = up_value.to_f32();
        let nonnegative = gate_f32 >= 0.0;
        let exponent_argument = if nonnegative { -gate_f32 } else { gate_f32 };
        let exponent = math.exp_f32(exponent_argument);
        let denominator = 1.0 + exponent;
        if !exponent.is_finite() || !denominator.is_finite() || denominator <= 0.0 {
            fe2o3_device::trap();
        }

        let numerator = if nonnegative { 1.0 } else { exponent };
        let sigmoid = numerator / denominator;
        let silu = gate_f32 * sigmoid;
        let product = silu * up_f32;
        if !sigmoid.is_finite() || !silu.is_finite() || !product.is_finite() {
            fe2o3_device::trap();
        }

        let narrowed = Bf16::from_f32(product);
        if !narrowed.is_finite() {
            fe2o3_device::trap();
        }
        let Some(slot) = output.get_block_mut(&output_block, component) else {
            fe2o3_device::trap();
        };
        *slot = narrowed.to_bits();
        component += 1;
    }
}
