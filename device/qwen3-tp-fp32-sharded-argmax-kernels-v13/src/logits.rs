use fe2o3_device::{
    Gfx950Subgroup, Index1D, RowStriped2D, StridedReadView2D, WriteOnlyDisjointSlice, kernel,
    thread,
};

// Host arithmetic fixtures are checked against the inline device bodies.
#[cfg(test)]
macro_rules! shard_lane_argmax {
    ($logits_view:expr, $row:expr, $shard:expr, $lane:expr) => {{
        let first_token = $shard * 64 + $lane;
        let first = $logits_view.load_or($row, first_token, 0.0_f32);
        let mut invalid = if first.is_finite() { 0.0_f32 } else { 1.0 };
        let mut value = if first.is_finite() { first } else { 0.0 };
        let mut winner = first_token as u32;
        let mut step = 1_usize;
        while step < 38 {
            let stripe = 64 * step + $shard;
            if stripe < 2374 {
                let token = 64 * stripe + $lane;
                let candidate = $logits_view.load_or($row, token, 0.0_f32);
                if !candidate.is_finite() {
                    invalid = 1.0;
                } else if candidate > value {
                    value = candidate;
                    winner = token as u32;
                }
            }
            step += 1;
        }
        (value, winner, invalid)
    }};
}

#[cfg(test)]
macro_rules! stable_token_key {
    ($value:expr, $maximum:expr, $winner:expr) => {{
        if $value == $maximum {
            151936.0_f32 - $winner as f32
        } else {
            0.0_f32
        }
    }};
}

#[cfg(test)]
macro_rules! final_lane_input {
    ($value:expr, $key:expr) => {{
        let integer_key = $key as u32;
        let invalid = if !$value.is_finite()
            || !$key.is_finite()
            || integer_key == 0
            || integer_key > 151936
            || integer_key as f32 != $key
        {
            1.0_f32
        } else {
            0.0_f32
        };
        let value = if invalid == 0.0 { $value } else { 0.0 };
        let key = if invalid == 0.0 { $key } else { 0.0 };
        (value, key, invalid)
    }};
}

#[cfg(test)]
macro_rules! winning_shard_key {
    ($value:expr, $maximum:expr, $key:expr) => {{ if $value == $maximum { $key } else { 0.0_f32 } }};
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

/// Each Wave64 owns one shard and writes one maximum/key pair; key zero is invalid.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [2048, 1, 1]), control_flow(loop_bounds(38)))]
pub fn ferric_qwen3_tp_batch32_sharded_argmax_produce_f32_v13(
    logits: &[f32],
    mut maxima: WriteOnlyDisjointSlice<f32, RowStriped2D<Index1D, 64, 1>>,
    mut keys: WriteOnlyDisjointSlice<f32, RowStriped2D<Index1D, 64, 1>>,
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
        || maxima.len() != 32 * 64
        || keys.len() != 32 * 64
        || thread::launch_extent_1d() != rows * 4096
    {
        fe2o3_device::trap();
    }
    let Ok(logits_view) = StridedReadView2D::from_shared_slice(logits, 0, rows, 151936, 151936)
    else {
        fe2o3_device::trap();
    };
    let invocation = thread::index_1d();
    let raw = invocation.get();
    let group = thread::block_idx_x() as usize;
    let row = group / 64;
    let shard = group % 64;
    let lane = raw % 64;
    if row < rows {
    } else {
        fe2o3_device::trap();
    }
    let subgroup = Gfx950Subgroup::current();
    let (value, winner, invalid) = {
        // BEGIN shard_lane_argmax
        let first_token = shard * 64 + lane;
        let first = logits_view.load_or(row, first_token, 0.0_f32);
        let mut invalid = if first.is_finite() { 0.0_f32 } else { 1.0 };
        let mut value = if first.is_finite() { first } else { 0.0 };
        let mut winner = first_token as u32;
        let mut step = 1_usize;
        while step < 38 {
            let stripe = 64 * step + shard;
            if stripe < 2374 {
                let token = 64 * stripe + lane;
                let candidate = logits_view.load_or(row, token, 0.0_f32);
                if !candidate.is_finite() {
                    invalid = 1.0;
                } else if candidate > value {
                    value = candidate;
                    winner = token as u32;
                }
            }
            step += 1;
        }
        (value, winner, invalid)
        // END shard_lane_argmax
    };
    let any_invalid = subgroup.reduce_max_f32::<64>(invalid);
    let maximum = subgroup.reduce_max_f32::<64>(value);
    let key = {
        // BEGIN stable_token_key
        if value == maximum {
            151936.0_f32 - winner as f32
        } else {
            0.0_f32
        }
        // END stable_token_key
    };
    let winning_key = subgroup.reduce_max_f32::<64>(key);
    let shard_key = if any_invalid == 0.0 { winning_key } else { 0.0 };
    if lane == 0 {
        let Some(stripe) = invocation.checked_row_striped_2d::<64, 1>() else {
            fe2o3_device::trap();
        };
        // Flatten [rows, 64] into [rows * 64, 1] for the existing Wave64 owner witness.
        if !maxima.write_row_striped_2d(&stripe, 0, rows * 64, 1, 1, maximum) {
            fe2o3_device::trap();
        }
        if !keys.write_row_striped_2d(&stripe, 0, rows * 64, 1, 1, shard_key) {
            fe2o3_device::trap();
        }
    }
}

/// Requires completed private producer scratch; rejects every invalid shard before output.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [32, 1, 1]))]
pub fn ferric_qwen3_tp_batch32_sharded_argmax_finalize_f32_v13(
    maxima: &[f32],
    keys: &[f32],
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
    if maxima.len() != 32 * 64
        || keys.len() != 32 * 64
        || choices.len() < rows
        || choices.len() > 32
        || thread::launch_extent_1d() != rows * 64
    {
        fe2o3_device::trap();
    }
    let Ok(maxima_view) = StridedReadView2D::from_shared_slice(maxima, 0, rows, 64, 64) else {
        fe2o3_device::trap();
    };
    let Ok(keys_view) = StridedReadView2D::from_shared_slice(keys, 0, rows, 64, 64) else {
        fe2o3_device::trap();
    };
    let invocation = thread::index_1d();
    let raw = invocation.get();
    let row = thread::block_idx_x() as usize;
    let lane = raw % 64;
    if row < rows {
    } else {
        fe2o3_device::trap();
    }
    let subgroup = Gfx950Subgroup::current();
    let value = maxima_view.load_or(row, lane, 0.0_f32);
    let key = keys_view.load_or(row, lane, 0.0_f32);
    let (value, key, invalid) = {
        // BEGIN final_lane_input
        let integer_key = key as u32;
        let invalid = if !value.is_finite()
            || !key.is_finite()
            || integer_key == 0
            || integer_key > 151936
            || integer_key as f32 != key
        {
            1.0_f32
        } else {
            0.0_f32
        };
        let value = if invalid == 0.0 { value } else { 0.0 };
        let key = if invalid == 0.0 { key } else { 0.0 };
        (value, key, invalid)
        // END final_lane_input
    };
    let any_invalid = subgroup.reduce_max_f32::<64>(invalid);
    if any_invalid != 0.0 {
        fe2o3_device::trap();
    }
    let maximum = subgroup.reduce_max_f32::<64>(value);
    let key = {
        // BEGIN winning_shard_key
        if value == maximum { key } else { 0.0_f32 }
        // END winning_shard_key
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
