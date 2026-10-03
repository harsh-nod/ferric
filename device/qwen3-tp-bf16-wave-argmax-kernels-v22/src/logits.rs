use fe2o3_device::{
    Gfx950Subgroup, Index1D, RowStriped2D, StridedReadView2D, WriteOnlyDisjointSlice, kernel,
    thread,
};

// Source-equivalence tests bind these host fixtures to the expanded device body.
#[cfg(test)]
macro_rules! invalid_shape {
    ($rows:expr, $logits_len:expr, $choices_len:expr, $launch_extent:expr) => {{ $rows != 1 || $logits_len != 16 * 151936 || $choices_len != 16 || $launch_extent != 64 }};
}

#[cfg(test)]
macro_rules! ordered_bits {
    ($raw:expr) => {{
        if $raw & 0x7f80 == 0x7f80 {
            0_u32
        } else if $raw & 0x7fff == 0 {
            32768
        } else if $raw & 0x8000 != 0 {
            $raw ^ 0xffff
        } else {
            $raw | 0x8000
        }
    }};
}

#[cfg(test)]
macro_rules! lane_argmax {
    ($logits_view:expr, $row:expr, $lane:expr) => {{
        let mut invalid = 0.0_f32;
        let mut ordered = 0_u32;
        let mut winner = $lane as u32;
        let mut step = 0_usize;
        while step < 2374 {
            let token = step * 64 + $lane;
            let raw = $logits_view.load_or($row, token, 0_u16) as u32;
            let candidate = {
                if raw & 0x7f80 == 0x7f80 {
                    0_u32
                } else if raw & 0x7fff == 0 {
                    32768
                } else if raw & 0x8000 != 0 {
                    raw ^ 0xffff
                } else {
                    raw | 0x8000
                }
            };
            if candidate == 0 {
                invalid = 1.0;
            } else if candidate > ordered {
                ordered = candidate;
                winner = token as u32;
            }
            step += 1;
        }
        (ordered as f32, winner, invalid)
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

/// One Wave64 scans the active BF16 row; only lane zero publishes the lowest ID.
/// Integer ordering preserves BF16 subnormals and folds both signed zeros together.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [1, 1, 1]), control_flow(loop_bounds(2374)))]
pub fn ferric_qwen3_tp_single_wave_argmax_bf16_v22(
    logits: &[u16],
    mut choices: WriteOnlyDisjointSlice<u32, RowStriped2D<Index1D, 64, 1>>,
    rows: u32,
) {
    let logits_len = logits.len();
    let choices_len = choices.len();
    let launch_extent = thread::launch_extent_1d();
    let invalid_shape = {
        // BEGIN invalid_shape
        rows != 1 || logits_len != 16 * 151936 || choices_len != 16 || launch_extent != 64
        // END invalid_shape
    };
    if invalid_shape {
        fe2o3_device::trap();
    }
    let rows = rows as usize;
    if rows < 2 {
    } else {
        fe2o3_device::trap();
    }
    let Ok(logits_view) = StridedReadView2D::from_shared_slice(logits, 0, rows, 151936, 151936)
    else {
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
    let (value, winner, invalid) = {
        // BEGIN lane_argmax
        let mut invalid = 0.0_f32;
        let mut ordered = 0_u32;
        let mut winner = lane as u32;
        let mut step = 0_usize;
        while step < 2374 {
            let token = step * 64 + lane;
            let raw = logits_view.load_or(row, token, 0_u16) as u32;
            let candidate = {
                // BEGIN ordered_bits
                if raw & 0x7f80 == 0x7f80 {
                    0_u32
                } else if raw & 0x7fff == 0 {
                    32768
                } else if raw & 0x8000 != 0 {
                    raw ^ 0xffff
                } else {
                    raw | 0x8000
                }
                // END ordered_bits
            };
            if candidate == 0 {
                invalid = 1.0;
            } else if candidate > ordered {
                ordered = candidate;
                winner = token as u32;
            }
            step += 1;
        }
        (ordered as f32, winner, invalid)
        // END lane_argmax
    };
    // Keep all three reductions ahead of data-dependent rejection and stores.
    let any_invalid = subgroup.reduce_max_f32::<64>(invalid);
    let maximum = subgroup.reduce_max_f32::<64>(value);
    // Both ordering values and token keys fit exactly in FP32 integer precision.
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
    if any_invalid != 0.0 {
        fe2o3_device::trap();
    }
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
