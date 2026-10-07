use fe2o3_device::{Index1D, RowStriped2D, WriteOnlyDisjointSlice, kernel, memory, thread};

#[cfg(test)]
macro_rules! fp32_argmax {
    ($logits:expr, $base:expr) => {{
        let first = memory::volatile_load($logits, $base);
        if !first.is_finite() {
            fe2o3_device::trap();
        }
        let mut winner = 0_u32;
        let mut value = first;
        let mut token = 1_usize;
        while token < 151936 {
            let candidate = memory::volatile_load($logits, $base + token);
            if !candidate.is_finite() {
                fe2o3_device::trap();
            }
            if candidate > value {
                winner = token as u32;
                value = candidate;
            }
            token += 1;
        }
        winner
    }};
}

/// Ascending finite FP32 scan, retaining the lowest token ID for exact ties.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [32, 1, 1]), control_flow(loop_bounds(151936)))]
pub fn ferric_qwen3_tp_batch32_argmax_f32_v8(
    logits: &[f32],
    mut choices: WriteOnlyDisjointSlice<u32, RowStriped2D<Index1D, 64, 1>>,
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
    if logits.len() < rows * 151936
        || logits.len() > 32 * 151936
        || choices.len() < rows
        || choices.len() > 32
        || thread::launch_extent_1d() != rows * 64
    {
        fe2o3_device::trap();
    }
    let invocation = thread::index_1d();
    let raw = invocation.get();
    let row = raw / 64;
    let lane = raw % 64;
    if row < rows {
    } else {
        fe2o3_device::trap();
    }
    if lane != 0 {
        return;
    }
    let Some(stripe) = invocation.checked_row_striped_2d::<64, 1>() else {
        fe2o3_device::trap();
    };
    let base = row * 151936;
    let winner = {
        // BEGIN fp32_argmax
        let first = memory::volatile_load(logits, base);
        if !first.is_finite() {
            fe2o3_device::trap();
        }
        let mut winner = 0_u32;
        let mut value = first;
        let mut token = 1_usize;
        while token < 151936 {
            let candidate = memory::volatile_load(logits, base + token);
            if !candidate.is_finite() {
                fe2o3_device::trap();
            }
            if candidate > value {
                winner = token as u32;
                value = candidate;
            }
            token += 1;
        }
        winner
        // END fp32_argmax
    };
    if !choices.write_row_striped_2d(&stripe, 0, rows, 1, 1, winner) {
        fe2o3_device::trap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fp32_distinguishes_values_that_bf16_rounds_to_a_tie() {
        let mut values = std::vec![-1.0_f32; 151936];
        values[7] = 24.365_898;
        values[101] = 24.426_361;
        assert_eq!(
            fe2o3_device::Bf16::from_f32(values[7]).to_bits(),
            fe2o3_device::Bf16::from_f32(values[101]).to_bits()
        );
        assert_eq!(fp32_argmax!(&values, 0), 101);
    }

    #[test]
    fn ascending_ties_signed_zero_last_token_and_all_rows() {
        let mut values = std::vec![-1.0_f32; 32 * 151936];
        for row in 0..32 {
            let base = row * 151936;
            let token = if row == 31 { 151935 } else { row * 997 };
            values[base + token] = -0.0;
            values[base + 151935] = 0.0;
            assert_eq!(fp32_argmax!(&values, base), token as u32);
        }
    }

    #[test]
    fn nonfinite_values_at_all_scan_boundaries_trap() {
        for index in [0, 1, 75968, 151935] {
            for nonfinite in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                let mut values = std::vec![0.0_f32; 151936];
                values[index] = nonfinite;
                assert!(std::panic::catch_unwind(|| fp32_argmax!(&values, 0)).is_err());
            }
        }
    }
}
