use fe2o3_device::{Bf16, WriteOnlyDisjointSlice, kernel, memory, thread};

macro_rules! add_rank_v4 {
    ($sum:ident, $value:expr) => {{
        let value: f32 = $value;
        $sum += value;
        if !value.is_finite() || !$sum.is_finite() {
            fe2o3_device::trap();
        }
    }};
}

macro_rules! ordered_residual_v4 {
    ($world:expr, $p0:expr, $p1:expr, $p2:expr, $p3:expr, $p4:expr, $p5:expr, $p6:expr, $p7:expr, $residual:expr) => {{
        let mut sum = 0.0_f32;
        add_rank_v4!(sum, $p0);
        add_rank_v4!(sum, $p1);
        if $world == 8 {
            add_rank_v4!(sum, $p2);
            add_rank_v4!(sum, $p3);
            add_rank_v4!(sum, $p4);
            add_rank_v4!(sum, $p5);
            add_rank_v4!(sum, $p6);
            add_rank_v4!(sum, $p7);
        }
        let residual = Bf16::from_bits($residual).to_f32();
        let value = sum + residual;
        let narrowed = Bf16::from_f32(value);
        if !residual.is_finite() || !value.is_finite() || !narrowed.is_finite() {
            fe2o3_device::trap();
        }
        narrowed.to_bits()
    }};
}

#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [1024, 1, 1]))]
#[allow(clippy::too_many_arguments, clippy::len_zero)]
pub fn ferric_qwen3_tp_peer_ordered_residual_bf16_v4(
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
    if rows == 0 || rows > 16 || !(world == 2 || world == 8) {
        fe2o3_device::trap();
    }
    let rows = rows as usize;
    if rows < 17 {
    } else {
        fe2o3_device::trap();
    }
    let elements = rows * 4096;
    if p0.len() != elements
        || p1.len() != elements
        || residual.len() != elements
        || output.len() != elements
        || thread::launch_extent_1d() != elements
    {
        fe2o3_device::trap();
    }
    if world == 8 {
        if p2.len() != elements
            || p3.len() != elements
            || p4.len() != elements
            || p5.len() != elements
            || p6.len() != elements
            || p7.len() != elements
        {
            fe2o3_device::trap();
        }
    } else if p2.len() != 0
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
    let value = ordered_residual_v4!(
        world,
        memory::volatile_load(p0, index),
        memory::volatile_load(p1, index),
        memory::volatile_load(p2, index),
        memory::volatile_load(p3, index),
        memory::volatile_load(p4, index),
        memory::volatile_load(p5, index),
        memory::volatile_load(p6, index),
        memory::volatile_load(p7, index),
        memory::volatile_load(residual, index)
    );
    if !output.write(invocation, value) {
        fe2o3_device::trap();
    }
}

#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [1024, 1, 1]))]
pub fn ferric_qwen3_tp_peer_copy_bf16_v4(
    source: &[u16],
    mut output: WriteOnlyDisjointSlice<u16>,
    rows: u32,
) {
    if rows == 0 || rows > 16 {
        fe2o3_device::trap();
    }
    let rows = rows as usize;
    if rows < 17 {
    } else {
        fe2o3_device::trap();
    }
    let elements = rows * 4096;
    if source.len() != elements
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
    let bits = memory::volatile_load(source, index);
    if !output.write(invocation, bits) {
        fe2o3_device::trap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(values: [f32; 8], residual: u16, world: u32) -> u16 {
        ordered_residual_v4!(
            world, values[0], values[1], values[2], values[3], values[4], values[5], values[6],
            values[7], residual
        )
    }

    fn reference(values: [f32; 8], residual: u16, world: usize) -> u16 {
        let sum = values[..world]
            .iter()
            .fold(0.0_f32, |sum, value| sum + value);
        let value = sum + f32::from_bits(u32::from(residual) << 16);
        let bits = value.to_bits();
        (bits.wrapping_add(0x7fff + ((bits >> 16) & 1)) >> 16) as u16
    }

    #[test]
    fn ordered_rank_sum_and_single_rounding_match_host_reference() {
        for world in [2, 8] {
            for values in [
                [0.0; 8],
                [-0.0; 8],
                [0.000_976_562_5; 8],
                [
                    16_777_216.0,
                    1.0,
                    -16_777_216.0,
                    0.5,
                    -0.25,
                    0.125,
                    0.0625,
                    -0.03125,
                ],
            ] {
                for residual in [0, 0x8000, 0x3f80, 0xbf80, 0x0080, 0x7e00] {
                    assert_eq!(
                        run(values, residual, world),
                        reference(values, residual, world as usize)
                    );
                }
            }
        }
    }

    #[test]
    fn disabled_tp2_rank_expressions_are_not_evaluated() {
        let disabled = || -> f32 { panic!("disabled rank was read") };
        assert_eq!(
            ordered_residual_v4!(
                2,
                0.5,
                0.25,
                disabled(),
                disabled(),
                disabled(),
                disabled(),
                disabled(),
                disabled(),
                0x3f80
            ),
            0x3fe0
        );
    }

    #[test]
    fn all_finite_bf16_residuals_match_zero_partial_reference() {
        for bits in 0..=u16::MAX {
            if Bf16::from_bits(bits).is_finite() {
                assert_eq!(run([0.0; 8], bits, 8), reference([0.0; 8], bits, 8));
            }
        }
    }

    #[test]
    fn active_nonfinite_inputs_and_intermediate_overflow_trap() {
        for rank in 0..8 {
            for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                let mut values = [0.0; 8];
                values[rank] = invalid;
                assert!(std::panic::catch_unwind(|| run(values, 0, 8)).is_err());
            }
        }
        assert!(std::panic::catch_unwind(|| run([f32::MAX; 8], 0, 8)).is_err());
        assert!(std::panic::catch_unwind(|| run([0.0; 8], 0x7f80, 8)).is_err());
        assert!(std::panic::catch_unwind(|| run([0.0; 8], 0x7fc0, 8)).is_err());
        let mut values = [0.0; 8];
        values[0] = f32::MAX;
        assert!(std::panic::catch_unwind(|| run(values, 0, 8)).is_err());
    }
}
