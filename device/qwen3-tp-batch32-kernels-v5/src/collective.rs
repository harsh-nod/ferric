//! Exact TP1 residual arithmetic. This kernel does not claim cross-device access.

use fe2o3_device::{Bf16, WriteOnlyDisjointSlice, kernel, memory, thread};

macro_rules! residual_bf16_v5 {
    ($partial:expr, $residual:expr) => {{
        let partial: f32 = $partial;
        let residual = Bf16::from_bits($residual).to_f32();
        let sum = 0.0_f32 + partial;
        let value = sum + residual;
        let rounded = Bf16::from_f32(value);
        if !partial.is_finite()
            || !sum.is_finite()
            || !residual.is_finite()
            || !value.is_finite()
            || !rounded.is_finite()
        {
            fe2o3_device::trap();
        }
        rounded.to_bits()
    }};
}

#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [2048, 1, 1]))]
pub fn ferric_qwen3_tp_batch32_residual_bf16_v5(
    partial: &[f32],
    residual: &[u16],
    mut output: WriteOnlyDisjointSlice<u16>,
    rows: u32,
) {
    if rows == 0 || rows > 32 {
        fe2o3_device::trap();
    }
    let rows = rows as usize;
    if rows < 33 {
    } else {
        fe2o3_device::trap();
    }
    let elements = rows * 4096;
    if partial.len() != elements
        || residual.len() != elements
        || output.len() != elements
        || thread::launch_extent_1d() != elements
    {
        fe2o3_device::trap();
    }
    let invocation = thread::index_1d();
    let index = invocation.get();
    if index < elements {
    } else {
        fe2o3_device::trap();
    }
    let value = residual_bf16_v5!(
        memory::volatile_load(partial, index),
        memory::volatile_load(residual, index)
    );
    if !output.write(invocation, value) {
        fe2o3_device::trap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference(partial: f32, residual: u16) -> u16 {
        let sum = 0.0_f32 + partial;
        let value = sum + f32::from_bits(u32::from(residual) << 16);
        let bits = value.to_bits();
        (bits.wrapping_add(0x7fff + ((bits >> 16) & 1)) >> 16) as u16
    }

    #[test]
    fn exact_one_rounding_matches_independent_host_arithmetic() {
        for residual in [0_u16, 0x8000, 0x3f80, 0xbf80, 0x0080, 0x7f00, 0xff00] {
            for partial in [
                0.0_f32,
                -0.0,
                1.0,
                -1.0,
                0.00390625,
                0.01171875,
                f32::MIN_POSITIVE,
            ] {
                assert_eq!(
                    residual_bf16_v5!(partial, residual),
                    reference(partial, residual)
                );
            }
        }
    }

    #[test]
    fn every_finite_bf16_residual_survives_zero_partial_exactly() {
        for bits in 0_u16..=u16::MAX {
            if Bf16::from_bits(bits).is_finite() {
                assert_eq!(residual_bf16_v5!(0.0, bits), reference(0.0, bits));
            }
        }
    }

    #[test]
    fn nonfinite_inputs_and_rounding_overflow_trap() {
        for (partial, residual) in [
            (f32::NAN, 0_u16),
            (f32::INFINITY, 0),
            (f32::NEG_INFINITY, 0),
            (0.0, 0x7f80),
            (0.0, 0xff80),
            (0.0, 0x7fc1),
            (f32::MAX, 0),
            (f32::MAX, 0x7f7f),
        ] {
            assert!(std::panic::catch_unwind(|| residual_bf16_v5!(partial, residual)).is_err());
        }
    }
}
