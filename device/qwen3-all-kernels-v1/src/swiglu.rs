#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)] // The kernel macro emits an undocumented helper module.

//! Attributed Rust source for Ferric's exact Qwen3 SwiGLU device kernel.
//!
//! This source contract carries no artifact, launch, numerical-qualification,
//! or M1 authority. Production Worker V3 integration remains fail-closed until
//! an exact compiler run emits and verifies a replacement artifact.

use fe2o3_device::{Bf16, Blocked, Index1D, Math, WriteOnlyDisjointSlice, kernel, thread};

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

macro_rules! qwen3_swiglu_extent_is_admitted_expr_v1 {
    ($elements:expr) => {
        $elements == 3_072
            || $elements == 12_288
            || $elements == 24_576
            || $elements == 49_152
            || $elements == 61_440
            || $elements == 98_304
            || $elements == 110_592
            || $elements == 208_896
            || $elements == 393_216
            || $elements == 491_520
            || $elements == 1_572_864
            || $elements == 3_145_728
            || $elements == 6_291_456
            || $elements == 12_582_912
            || $elements == 25_165_824
    };
}

/// Returns whether `elements` is one of the exact Ferric M1 SwiGLU extents.
#[must_use]
pub const fn qwen3_swiglu_extent_is_admitted_v1(elements: usize) -> bool {
    qwen3_swiglu_extent_is_admitted_expr_v1!(elements)
}

#[inline(always)]
#[cfg(test)]
const fn f32_is_finite_v1(value: f32) -> bool {
    value >= f32::MIN && value <= f32::MAX
}

macro_rules! qwen3_swiglu_element_v1 {
    ($gate_bits:expr, $up_bits:expr) => {{
        let gate_value = Bf16::from_bits($gate_bits);
        let up_value = Bf16::from_bits($up_bits);
        if !gate_value.is_finite() || !up_value.is_finite() {
            fe2o3_device::trap();
        }

        let gate_f32 = gate_value.to_f32();
        let up_f32 = up_value.to_f32();
        let nonnegative = gate_f32 >= 0.0;
        let exponent_argument = if nonnegative { -gate_f32 } else { gate_f32 };
        let exponent = Math::current().exp_f32(exponent_argument);
        let denominator = 1.0 + exponent;
        if !(exponent >= f32::MIN && exponent <= f32::MAX)
            || exponent < 0.0
            || !(denominator >= f32::MIN && denominator <= f32::MAX)
            || denominator <= 0.0
        {
            fe2o3_device::trap();
        }

        let numerator = if nonnegative { 1.0 } else { exponent };
        let sigmoid = numerator / denominator;
        let silu = gate_f32 * sigmoid;
        let product = silu * up_f32;
        if !(sigmoid >= f32::MIN && sigmoid <= f32::MAX)
            || !(silu >= f32::MIN && silu <= f32::MAX)
            || !(product >= f32::MIN && product <= f32::MAX)
        {
            fe2o3_device::trap();
        }

        let narrowed = Bf16::from_f32(product);
        if !narrowed.is_finite() {
            fe2o3_device::trap();
        }
        narrowed.to_bits()
    }};
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
    )
)]
pub fn qwen3_swiglu_bf16_f32_v1(
    gate: &[u16],
    up: &[u16],
    mut output: WriteOnlyDisjointSlice<u16, Blocked<Index1D, 1, 8>>,
) {
    let elements = gate.len();
    if !qwen3_swiglu_extent_is_admitted_expr_v1!(elements)
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

    let index_0 = base;
    if index_0 < elements {
        let value = qwen3_swiglu_element_v1!(gate[index_0], up[index_0]);
        if !output.write_block(&output_block, 0, value) {
            fe2o3_device::trap();
        }
    }

    let index_1 = base + 1;
    if index_1 < elements {
        let value = qwen3_swiglu_element_v1!(gate[index_1], up[index_1]);
        if !output.write_block(&output_block, 1, value) {
            fe2o3_device::trap();
        }
    }

    let index_2 = base + 2;
    if index_2 < elements {
        let value = qwen3_swiglu_element_v1!(gate[index_2], up[index_2]);
        if !output.write_block(&output_block, 2, value) {
            fe2o3_device::trap();
        }
    }

    let index_3 = base + 3;
    if index_3 < elements {
        let value = qwen3_swiglu_element_v1!(gate[index_3], up[index_3]);
        if !output.write_block(&output_block, 3, value) {
            fe2o3_device::trap();
        }
    }

    let index_4 = base + 4;
    if index_4 < elements {
        let value = qwen3_swiglu_element_v1!(gate[index_4], up[index_4]);
        if !output.write_block(&output_block, 4, value) {
            fe2o3_device::trap();
        }
    }

    let index_5 = base + 5;
    if index_5 < elements {
        let value = qwen3_swiglu_element_v1!(gate[index_5], up[index_5]);
        if !output.write_block(&output_block, 5, value) {
            fe2o3_device::trap();
        }
    }

    let index_6 = base + 6;
    if index_6 < elements {
        let value = qwen3_swiglu_element_v1!(gate[index_6], up[index_6]);
        if !output.write_block(&output_block, 6, value) {
            fe2o3_device::trap();
        }
    }

    let index_7 = base + 7;
    if index_7 < elements {
        let value = qwen3_swiglu_element_v1!(gate[index_7], up[index_7]);
        if !output.write_block(&output_block, 7, value) {
            fe2o3_device::trap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::f32_is_finite_v1;

    #[test]
    fn explicit_f32_finiteness_rejects_infinities_and_nan_payloads() {
        for value in [0.0, -0.0, f32::MIN, f32::MAX, f32::MIN_POSITIVE] {
            assert!(f32_is_finite_v1(value));
        }
        for value in [
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
            f32::from_bits(0x7f80_0001),
            f32::from_bits(0xffc0_0001),
        ] {
            assert!(!f32_is_finite_v1(value));
        }
    }
}
