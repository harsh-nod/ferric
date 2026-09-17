#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)] // The kernel attribute emits an internal helper module.

//! Per-head key RMSNorm after a separately completed key-projection dispatch.
//!
//! BF16 rounding boundaries follow the pinned eager Qwen3 implementation.
//! Square root followed by division is an explicit alternative to its rsqrt;
//! this source alone establishes neither numerical parity nor GPU correctness.

use fe2o3_device::{
    Bf16, Gfx950Subgroup, Index1D, Math, RowStriped2D, StridedReadView2D, WriteOnlyDisjointSlice,
    kernel, thread,
};

pub const KERNEL_SYMBOL: &str = "ferric_gfx950_qwen3_knorm_v1";
pub const HEADS: usize = 8;
pub const HEAD_DIM: usize = 128;
pub const VALUES: usize = HEADS * HEAD_DIM;
pub const WEIGHTS: usize = HEAD_DIM;
pub const LANES_PER_HEAD: usize = 64;
pub const COMPONENTS_PER_LANE: usize = 2;
pub const EPSILON: f32 = 1.0e-6;
pub const WORKGROUP: [u32; 3] = [128, 1, 1];
pub const AQL_GRID: [u32; 3] = [512, 1, 1];
pub const EXPLICIT_KERNARG_BYTES: usize = 64;

/// Normalize eight independent heads, with two full waves in each workgroup.
#[kernel(
    typed,
    launch(required = [128, 1, 1], max = [128, 1, 1], max_grid = [4, 1, 1])
)]
pub fn ferric_gfx950_qwen3_knorm_v1(
    inputs: &[f32],
    weights: &[u16],
    mut quantized: WriteOnlyDisjointSlice<u16, RowStriped2D<Index1D, 64, 2>>,
    mut output: WriteOnlyDisjointSlice<u16, RowStriped2D<Index1D, 64, 2>>,
) {
    if inputs.len() != VALUES
        || weights.len() != WEIGHTS
        || quantized.len() != VALUES
        || output.len() != VALUES
    {
        fe2o3_device::trap();
    }
    let input_view = if let Ok(view) =
        StridedReadView2D::from_shared_slice(inputs, 0, HEADS, HEAD_DIM, HEAD_DIM)
    {
        view
    } else {
        fe2o3_device::trap()
    };
    let weight_view =
        if let Ok(view) = StridedReadView2D::from_shared_slice(weights, 0, 1, HEAD_DIM, HEAD_DIM) {
            view
        } else {
            fe2o3_device::trap()
        };
    let index = thread::index_1d();
    let raw = index.get();
    let head = raw / LANES_PER_HEAD;
    let lane = raw % LANES_PER_HEAD;
    let second_column = lane + LANES_PER_HEAD;

    // Total reads avoid lane-dependent panic paths before the collective.
    // Round the completed FP32 projection to the model's BF16 key boundary.
    let first_key = Bf16::from_f32(input_view.load_or(head, lane, 0.0));
    let second_key = Bf16::from_f32(input_view.load_or(head, second_column, 0.0));
    let first = first_key.to_f32();
    let second = second_key.to_f32();
    let partial = first * first + second * second;
    let subgroup = Gfx950Subgroup::current();
    let sum = subgroup.reduce_sum_f32::<64>(partial);
    let mean = sum * (1.0f32 / 128.0f32);
    let math = Math::current();
    let scale = 1.0f32 / math.sqrt_f32(mean + EPSILON);

    // Qwen3 casts the normalized value before multiplying by BF16 gamma.
    // A second BF16 rounding stores that product; the two casts cannot merge.
    let first_normalized = Bf16::from_f32(first * scale).to_f32();
    let second_normalized = Bf16::from_f32(second * scale).to_f32();
    let first_weight = Bf16::from_bits(weight_view.load_or(0, lane, 0)).to_f32();
    let second_weight = Bf16::from_bits(weight_view.load_or(0, second_column, 0)).to_f32();
    let first_output = Bf16::from_f32(first_normalized * first_weight).to_bits();
    let second_output = Bf16::from_f32(second_normalized * second_weight).to_bits();

    // RowStriped2D owns columns lane and lane+64 in this wave's head. Selection
    // and checked writes occur after the last collective, preserving convergence.
    if let Some(stripe) = index.checked_row_striped_2d::<64, 2>() {
        if !quantized.write_row_striped_2d(
            &stripe,
            0,
            HEADS,
            HEAD_DIM,
            HEAD_DIM,
            first_key.to_bits(),
        ) || !quantized.write_row_striped_2d(
            &stripe,
            1,
            HEADS,
            HEAD_DIM,
            HEAD_DIM,
            second_key.to_bits(),
        ) || !output.write_row_striped_2d(&stripe, 0, HEADS, HEAD_DIM, HEAD_DIM, first_output)
            || !output.write_row_striped_2d(&stripe, 1, HEADS, HEAD_DIM, HEAD_DIM, second_output)
        {
            fe2o3_device::trap();
        }
    } else {
        fe2o3_device::trap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, size_of};

    #[test]
    fn fixed_shape_covers_each_component_once() {
        assert_eq!(AQL_GRID[0] as usize, HEADS * LANES_PER_HEAD);
        assert_eq!(WORKGROUP[0] as usize, 2 * LANES_PER_HEAD);
        assert_eq!(HEAD_DIM, LANES_PER_HEAD * COMPONENTS_PER_LANE);
        let mut visits = [0u8; VALUES];
        for raw in 0..AQL_GRID[0] as usize {
            let head = raw / LANES_PER_HEAD;
            let lane = raw % LANES_PER_HEAD;
            for component in 0..COMPONENTS_PER_LANE {
                visits[head * HEAD_DIM + component * LANES_PER_HEAD + lane] += 1;
            }
        }
        assert!(visits.iter().all(|count| *count == 1));
    }

    #[test]
    fn four_slice_abi_is_explicit_and_aligned() {
        type Output = WriteOnlyDisjointSlice<u16, RowStriped2D<Index1D, 64, 2>>;
        assert_eq!(size_of::<&[f32]>(), 16);
        assert_eq!(size_of::<&[u16]>(), 16);
        assert_eq!(size_of::<Output>(), 16);
        assert_eq!(align_of::<Output>(), 8);
        assert_eq!(EXPLICIT_KERNARG_BYTES, 4 * 16);
    }

    #[test]
    fn bf16_boundaries_use_ties_to_even_and_preserve_signed_zero() {
        assert_eq!(
            Bf16::from_f32(f32::from_bits(0x3f80_8000)).to_bits(),
            0x3f80
        );
        assert_eq!(
            Bf16::from_f32(f32::from_bits(0x3f81_8000)).to_bits(),
            0x3f82
        );
        assert_eq!(Bf16::from_f32(-0.0).to_bits(), 0x8000);
        let normalized = f32::from_bits(0x3f80_8000);
        let weight = Bf16::from_bits(0x3f81).to_f32();
        let staged = Bf16::from_f32(Bf16::from_f32(normalized).to_f32() * weight);
        let fused_cast = Bf16::from_f32(normalized * weight);
        assert_ne!(staged.to_bits(), fused_cast.to_bits());
    }
}
