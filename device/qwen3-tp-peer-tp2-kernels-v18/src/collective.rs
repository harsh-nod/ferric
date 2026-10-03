use fe2o3_device::{Bf16, WriteOnlyDisjointSlice, kernel, memory, thread};

macro_rules! add_rank_v18 {
    ($sum:ident, $value:expr) => {{
        let value: f32 = $value;
        $sum += value;
        if !value.is_finite() || !$sum.is_finite() {
            fe2o3_device::trap();
        }
    }};
}

macro_rules! ordered_residual_v18 {
    ($p0:expr, $p1:expr, $residual:expr) => {{
        let mut sum = 0.0_f32;
        add_rank_v18!(sum, $p0);
        add_rank_v18!(sum, $p1);
        let residual = Bf16::from_bits($residual).to_f32();
        let value = sum + residual;
        let narrowed = Bf16::from_f32(value);
        if !residual.is_finite() || !value.is_finite() || !narrowed.is_finite() {
            fe2o3_device::trap();
        }
        narrowed.to_bits()
    }};
}

#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [64, 1, 1]))]
#[allow(clippy::too_many_arguments, clippy::len_zero)]
pub fn ferric_qwen3_tp_peer_tp2_ordered_residual_bf16_v18(
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
    let value = ordered_residual_v18!(
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
        ordered_residual_v18!(p0, p1, residual)
    }

    #[allow(clippy::cast_possible_truncation)]
    fn reference(p0: f32, p1: f32, residual: u16) -> Option<u16> {
        let first = (0.0_f64 + f64::from(p0)) as f32;
        let sum = (f64::from(first) + f64::from(p1)) as f32;
        let residual = f32::from_bits(u32::from(residual) << 16);
        let value = (f64::from(sum) + f64::from(residual)) as f32;
        if ![p0, p1, first, sum, residual, value]
            .into_iter()
            .all(f32::is_finite)
        {
            return None;
        }
        let bits = value.to_bits();
        let upper = bits >> 16;
        let lower = bits & 0xffff;
        let increment = u32::from(lower > 0x8000 || (lower == 0x8000 && upper & 1 != 0));
        let rounded = u16::try_from(upper + increment).unwrap();
        ((rounded & 0x7f80) != 0x7f80).then_some(rounded)
    }

    #[test]
    fn analytical_pairs_match_independent_f64_staged_reference() {
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
    fn nonuniform_full_active_vectors_match_reference_for_two_generations() {
        for generation in 1..=2_i32 {
            for index in 0..4096_u16 {
                let p0 = f32::from(
                    i16::try_from((i32::from(index) * 17 + generation * 13) % 257 - 128).unwrap(),
                ) / 32.0;
                let p1 = f32::from(
                    i16::try_from((i32::from(index) * 29 + generation * 7) % 251 - 125).unwrap(),
                ) / 64.0;
                let residual =
                    [0x0000, 0x8000, 0x3f80, 0xbf80, 0x3f81, 0x0080][usize::from(index) % 6];
                assert_eq!(Some(run(p0, p1, residual)), reference(p0, p1, residual));
            }
        }
    }

    #[test]
    fn one_final_rounding_does_not_narrow_partials_or_each_addition() {
        assert_eq!(run(1.0 / 256.0, 1.0 / 256.0, 0x3f80), 0x3f81);
        let p0 = 1.0 + 1.0 / 512.0;
        let p1 = -1.0;
        assert_eq!(Some(run(p0, p1, 0)), reference(p0, p1, 0));
        assert_ne!(run(p0, p1, 0), run(Bf16::from_f32(p0).to_f32(), p1, 0));
    }

    #[test]
    fn residual_is_added_after_both_partials() {
        assert_eq!(run(16_777_216.0, -16_777_216.0, 0x3f80), 0x3f80);
        assert_ne!(run(16_777_216.0, -16_777_216.0, 0x3f80), 0);
    }

    #[test]
    fn rank_expressions_are_evaluated_once_in_order() {
        let order = std::cell::RefCell::new(std::vec::Vec::new());
        let result = ordered_residual_v18!(
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
        assert_eq!(result, 0x3fe0);
        assert_eq!(*order.borrow(), [0, 1, 2]);
    }

    #[test]
    fn all_finite_bf16_residuals_match_zero_partial_reference() {
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
    fn intermediate_final_and_narrowed_overflow_trap() {
        assert!(std::panic::catch_unwind(|| run(f32::MAX, f32::MAX, 0)).is_err());
        assert!(std::panic::catch_unwind(|| run(f32::MAX, 0.0, 0x7f7f)).is_err());
        assert!(std::panic::catch_unwind(|| run(f32::MAX, 0.0, 0)).is_err());
    }
}
