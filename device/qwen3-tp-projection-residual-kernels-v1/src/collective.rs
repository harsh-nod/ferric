use fe2o3_device::{Bf16, WriteOnlyDisjointSlice, kernel, memory, thread};

macro_rules! add_rank_v1 {
    ($sum:ident, $value:expr) => {{
        let value: f32 = $value;
        $sum += value;
        if !value.is_finite() || !$sum.is_finite() {
            fe2o3_device::trap();
        }
    }};
}

macro_rules! projection_residual_v1 {
    ($p0:expr, $p1:expr, $residual:expr) => {{
        let mut sum = 0.0_f32;
        add_rank_v1!(sum, $p0);
        add_rank_v1!(sum, $p1);
        // Materialize the combined projection, not either rank's partial.
        let projection = Bf16::from_f32(sum);
        if !projection.is_finite() {
            fe2o3_device::trap();
        }
        let residual = Bf16::from_bits($residual).to_f32();
        let value = projection.to_f32() + residual;
        let narrowed = Bf16::from_f32(value);
        if !residual.is_finite() || !value.is_finite() || !narrowed.is_finite() {
            fe2o3_device::trap();
        }
        narrowed.to_bits()
    }};
}

#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [64, 1, 1]))]
#[allow(clippy::too_many_arguments, clippy::len_zero)]
pub fn ferric_qwen3_tp_peer_tp2_projection_residual_bf16_v1(
    p0: &[f32],
    p1: &[f32],
    p2: &[f32],
    p3: &[f32],
    p4: &[f32],
    p5: &[f32],
    p6: &[f32],
    p7: &[f32],
    residual: &[u16],
    mut output: WriteOnlyDisjointSlice<u16>,
    rows: u32,
    world: u32,
) {
    if rows != 1 || world != 2 {
        fe2o3_device::trap();
    }
    let elements = 4096_usize;
    if p0.len() != elements
        || p1.len() != elements
        || residual.len() != elements
        || output.len() != elements
        || thread::launch_extent_1d() != elements
    {
        fe2o3_device::trap();
    }
    if p2.len() != 0
        || p3.len() != 0
        || p4.len() != 0
        || p5.len() != 0
        || p6.len() != 0
        || p7.len() != 0
    {
        fe2o3_device::trap();
    }
    let invocation = thread::index_1d();
    let index = invocation.get();
    if index < elements {
    } else {
        fe2o3_device::trap();
    }
    let value = projection_residual_v1!(
        memory::volatile_load(p0, index),
        memory::volatile_load(p1, index),
        memory::volatile_load(residual, index)
    );
    if !output.write(invocation, value) {
        fe2o3_device::trap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(p0: f32, p1: f32, residual: u16) -> u16 {
        projection_residual_v1!(p0, p1, residual)
    }

    // Independent of Bf16's implementation. Each addition rounds to FP32
    // before the next stage; BF16 rounding inspects the discarded bits.
    fn narrow(value: f32) -> Option<u16> {
        if !value.is_finite() {
            return None;
        }
        let bits = value.to_bits();
        let upper = bits >> 16;
        let lower = bits & 0xffff;
        let increment = u32::from(lower > 0x8000 || (lower == 0x8000 && upper & 1 != 0));
        let rounded = u16::try_from(upper + increment).unwrap();
        ((rounded & 0x7f80) != 0x7f80).then_some(rounded)
    }

    #[allow(clippy::cast_possible_truncation)]
    fn reference(p0: f32, p1: f32, residual: u16) -> Option<u16> {
        let first = (0.0_f64 + f64::from(p0)) as f32;
        let sum = (f64::from(first) + f64::from(p1)) as f32;
        if ![p0, p1, first, sum].into_iter().all(f32::is_finite) {
            return None;
        }
        let projection = f32::from_bits(u32::from(narrow(sum)?) << 16);
        let residual = f32::from_bits(u32::from(residual) << 16);
        if !residual.is_finite() {
            return None;
        }
        narrow((f64::from(projection) + f64::from(residual)) as f32)
    }

    #[test]
    fn analytical_pairs_match_independent_staged_reference() {
        for (p0, p1) in [
            (0.0, 0.0),
            (-0.0, -0.0),
            (0.5, 0.25),
            (1.0 / 256.0, 1.0 / 256.0),
            (16_777_216.0, -16_777_216.0),
            (16_777_216.0, 1.0),
            (f32::MIN_POSITIVE, -f32::MIN_POSITIVE),
        ] {
            for residual in [0, 0x8000, 0x3f80, 0xbf80, 0x0080, 0x7e00] {
                assert_eq!(Some(run(p0, p1, residual)), reference(p0, p1, residual));
            }
        }
    }

    #[test]
    fn two_generations_cover_all_4096_active_elements() {
        for generation in 1..=2_i32 {
            for index in 0..4096_u16 {
                let p0 = f32::from(
                    i16::try_from((i32::from(index) * 17 + generation * 13) % 257 - 128).unwrap(),
                ) / 32.0;
                let p1 = f32::from(
                    i16::try_from((i32::from(index) * 29 + generation * 7) % 251 - 125).unwrap(),
                ) / 64.0;
                let residual = [0, 0x8000, 0x3f80, 0xbf80, 0x3f81, 0x0080][usize::from(index) % 6];
                assert_eq!(Some(run(p0, p1, residual)), reference(p0, p1, residual));
            }
        }
    }

    #[test]
    fn projection_ties_round_before_residual_addition() {
        // Even lower BF16 significand: 1 + 1/256 materializes as 1.
        assert_eq!(run(1.0, 1.0 / 256.0, 0xbf80), 0);
        // Odd lower significand: 1 + 3/256 rounds upward to 1 + 2/128.
        assert_eq!(run(1.0, 3.0 / 256.0, 0xbf80), 0x3c80);
        assert_eq!(run(-1.0, -1.0 / 256.0, 0x3f80), 0);
        assert_eq!(run(-1.0, -3.0 / 256.0, 0x3f80), 0xbc80);
    }

    #[test]
    fn neither_rank_partial_is_narrowed() {
        let p0 = 1.0 + 1.0 / 512.0;
        assert_eq!(run(p0, -1.0, 0), 0x3b00);
        assert_ne!(run(p0, -1.0, 0), run(Bf16::from_f32(p0).to_f32(), -1.0, 0));
        assert_eq!(run(1.0 / 256.0, 1.0 / 256.0, 0x3f80), 0x3f81);
    }

    #[test]
    fn fp32_rank_sum_rounds_before_residual() {
        // The +1 is lost in FP32; a wide three-term sum would produce 1.
        assert_eq!(run(16_777_216.0, 1.0, 0xcb80), 0);
        assert_eq!(run(16_777_216.0, -16_777_216.0, 0x3f80), 0x3f80);
    }

    #[test]
    fn expressions_are_evaluated_once_in_rank_then_residual_order() {
        let order = std::cell::RefCell::new(std::vec::Vec::new());
        let value = projection_residual_v1!(
            {
                order.borrow_mut().push(0);
                0.5
            },
            {
                order.borrow_mut().push(1);
                0.25
            },
            {
                order.borrow_mut().push(2);
                0x3f80
            }
        );
        assert_eq!(value, 0x3fe0);
        assert_eq!(*order.borrow(), [0, 1, 2]);
    }

    #[test]
    fn leading_positive_zero_and_exact_cancellation_stay_positive() {
        assert_eq!(run(-0.0, -0.0, 0x8000), 0);
        assert_eq!(run(-1.0, 1.0, 0x8000), 0);
        assert_eq!(run(1.0, -1.0, 0), 0);
    }

    #[test]
    fn tiny_projection_uses_bf16_rne_without_flushing_subnormals() {
        assert_eq!(run(f32::from_bits(0x8000), 0.0, 0), 0);
        assert_eq!(run(f32::from_bits(0x8001), 0.0, 0), 1);
        assert_eq!(run(f32::from_bits(0x18000), 0.0, 0), 2);
        assert_eq!(run(f32::from_bits(0x80008001), 0.0, 0), 0x8001);
    }

    #[test]
    fn all_finite_bf16_residuals_match_zero_projection_reference() {
        for bits in 0..=u16::MAX {
            if bits & 0x7f80 != 0x7f80 {
                assert_eq!(Some(run(0.0, 0.0, bits)), reference(0.0, 0.0, bits));
            }
        }
    }

    #[test]
    fn active_nonfinite_inputs_trap() {
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(std::panic::catch_unwind(|| run(bad, 0.0, 0)).is_err());
            assert!(std::panic::catch_unwind(|| run(0.0, bad, 0)).is_err());
        }
        for bad in [0x7f80, 0xff80, 0x7fc0, 0x7fff] {
            assert!(std::panic::catch_unwind(|| run(0.0, 0.0, bad)).is_err());
        }
    }

    #[test]
    fn materialization_overflow_traps_before_cancelling_residual_is_evaluated() {
        let read_residual = std::cell::Cell::new(false);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            projection_residual_v1!(f32::MAX, 0.0, {
                read_residual.set(true);
                0xff7f
            })
        }));
        assert!(result.is_err());
        assert!(!read_residual.get());
        // The old non-materializing operation could remain finite here.
        assert!((f32::MAX + f32::from_bits(0xff7f0000)).is_finite());
    }

    #[test]
    fn rank_sum_and_final_output_overflows_trap() {
        assert!(std::panic::catch_unwind(|| run(f32::MAX, f32::MAX, 0)).is_err());
        let max_bf16 = f32::from_bits(0x7f7f0000);
        assert!(std::panic::catch_unwind(|| run(max_bf16, 0.0, 0x7f7f)).is_err());
        // FP32 sum stays finite but its final BF16 narrowing overflows.
        assert!(std::panic::catch_unwind(|| run(max_bf16, 0.0, 0x7b00)).is_err());
    }
}
