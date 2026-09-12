use fe2o3_device::{
    Gfx950Subgroup, Index1D, RowStriped2D, WriteOnlyDisjointSlice, kernel, memory, thread,
};

// Source-equivalence tests bind these host fixtures to the expanded device body.
#[cfg(test)]
macro_rules! lane_argmax {
    ($logits:expr, $base:expr, $lane:expr) => {{
        let first = memory::volatile_load($logits, $base + $lane);
        let mut invalid = if first.is_finite() { 0.0_f32 } else { 1.0 };
        let mut value = if first.is_finite() { first } else { 0.0 };
        let mut winner = $lane as u32;
        let mut step = 1_usize;
        while step < 2374 {
            let token = step * 64 + $lane;
            let candidate = memory::volatile_load($logits, $base + token);
            if !candidate.is_finite() {
                invalid = 1.0;
            } else if candidate > value {
                value = candidate;
                winner = token as u32;
            }
            step += 1;
        }
        (value, winner, invalid)
    }};
}

#[cfg(test)]
macro_rules! stable_key {
    ($value:expr, $maximum:expr, $winner:expr) => {{
        if $value == $maximum {
            151936.0_f32 - $winner as f32
        } else {
            0.0_f32
        }
    }};
}

#[cfg(test)]
macro_rules! decode_key {
    ($winning_key:expr) => {{
        let winning_key = $winning_key as u32;
        if winning_key < 151937 {
        } else {
            fe2o3_device::trap();
        }
        if winning_key == 0 {
            fe2o3_device::trap();
        }
        (151936.0_f32 - winning_key as f32) as u32
    }};
}

/// One Wave64 per row, with exact lowest-token ties and all-value finite rejection.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [32, 1, 1]), control_flow(loop_bounds(2374)))]
pub fn ferric_qwen3_tp_batch32_wave_argmax_f32_v11(
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
    let base = row * 151936;
    let subgroup = Gfx950Subgroup::current();
    let (value, winner, invalid) = {
        // BEGIN lane_argmax
        let first = memory::volatile_load(logits, base + lane);
        let mut invalid = if first.is_finite() { 0.0_f32 } else { 1.0 };
        let mut value = if first.is_finite() { first } else { 0.0 };
        let mut winner = lane as u32;
        let mut step = 1_usize;
        while step < 2374 {
            let token = step * 64 + lane;
            let candidate = memory::volatile_load(logits, base + token);
            if !candidate.is_finite() {
                invalid = 1.0;
            } else if candidate > value {
                value = candidate;
                winner = token as u32;
            }
            step += 1;
        }
        (value, winner, invalid)
        // END lane_argmax
    };
    let any_invalid = subgroup.reduce_max_f32::<64>(invalid);
    if any_invalid != 0.0 {
        fe2o3_device::trap();
    }
    let maximum = subgroup.reduce_max_f32::<64>(value);
    // Every key is an exact FP32 integer. Equal signed zeros select the same key set.
    let key = {
        // BEGIN stable_key
        if value == maximum {
            151936.0_f32 - winner as f32
        } else {
            0.0_f32
        }
        // END stable_key
    };
    let winning_key = subgroup.reduce_max_f32::<64>(key);
    if lane == 0 {
        let Some(stripe) = invocation.checked_row_striped_2d::<64, 1>() else {
            fe2o3_device::trap();
        };
        let winner = {
            // BEGIN decode_key
            let winning_key = winning_key as u32;
            if winning_key < 151937 {
            } else {
                fe2o3_device::trap();
            }
            if winning_key == 0 {
                fe2o3_device::trap();
            }
            (151936.0_f32 - winning_key as f32) as u32
            // END decode_key
        };
        if !choices.write_row_striped_2d(&stripe, 0, rows, 1, 1, winner) {
            fe2o3_device::trap();
        }
    }
}

#[cfg(test)]
mod tests;
