use fe2o3_device::{Bf16, Index1D, RowStriped2D, WriteOnlyDisjointSlice, kernel, memory, thread};

#[cfg(test)]
macro_rules! batch_argmax_v2 {
    ($logits:expr, $row_base:expr) => {{
        let first = Bf16::from_bits(memory::volatile_load($logits, $row_base));
        if !first.is_finite() {
            fe2o3_device::trap();
        }
        let mut winner_token = 0_u32;
        let mut winner_value = first.to_f32();
        let mut token = 1_usize;
        while token < 151936 {
            let candidate = Bf16::from_bits(memory::volatile_load($logits, $row_base + token));
            if !candidate.is_finite() {
                fe2o3_device::trap();
            }
            let candidate_value = candidate.to_f32();
            if candidate_value > winner_value {
                winner_token = token as u32;
                winner_value = candidate_value;
            }
            token += 1;
        }
        winner_token
    }};
}

/// The ascending finite-logit scan preserves the earlier lowest-ID tie rule.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [16, 1, 1]), control_flow(loop_bounds(151936)))]
pub fn ferric_qwen3_tp_batch_argmax_bf16_v2(
    logits: &[u16],
    mut choices: WriteOnlyDisjointSlice<u32, RowStriped2D<Index1D, 64, 1>>,
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
    if logits.len() < rows * 151936
        || logits.len() > 16 * 151936
        || choices.len() < rows
        || choices.len() > 16
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
    let row_base = row * 151936;
    let winner = {
        // BEGIN batch_argmax_v2
        let first = Bf16::from_bits(memory::volatile_load(logits, row_base));
        if !first.is_finite() {
            fe2o3_device::trap();
        }
        let mut winner_token = 0_u32;
        let mut winner_value = first.to_f32();
        let mut token = 1_usize;
        while token < 151936 {
            let candidate = Bf16::from_bits(memory::volatile_load(logits, row_base + token));
            if !candidate.is_finite() {
                fe2o3_device::trap();
            }
            let candidate_value = candidate.to_f32();
            if candidate_value > winner_value {
                winner_token = token as u32;
                winner_value = candidate_value;
            }
            token += 1;
        }
        winner_token
        // END batch_argmax_v2
    };
    if !choices.write_row_striped_2d(&stripe, 0, rows, 1, 1, winner) {
        fe2o3_device::trap();
    }
}

#[cfg(test)]
mod tests;
