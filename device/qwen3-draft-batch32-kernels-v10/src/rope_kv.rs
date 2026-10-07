use fe2o3_device::{
    Bf16, GridExclusive, Index1D, RowStriped2D, WriteOnlyDisjointSlice, kernel, memory, thread,
};

#[cfg(test)]
macro_rules! batch_paged_slot_v10 {
    ($positions:expr, $tables:expr, $row:expr, $stride:expr, $pages:expr) => {{
        let row = $row;
        let stride = $stride;
        let pages = $pages;
        let position = memory::volatile_load($positions, row) as usize;
        if position < 8192 {
        } else {
            fe2o3_device::trap();
        }
        let logical_page = position / 16;
        if logical_page < stride {
        } else {
            fe2o3_device::trap();
        }
        let table_index = row * stride + logical_page;
        if table_index < $tables.len() {
        } else {
            fe2o3_device::trap();
        }
        let physical_page = memory::volatile_load($tables, table_index) as usize;
        if physical_page < pages {
        } else {
            fe2o3_device::trap();
        }
        physical_page * 16 + position % 16
    }};
}

#[cfg(test)]
macro_rules! batch_distinct_slots_v10 {
    ($positions:expr, $tables:expr, $rows:expr, $stride:expr, $pages:expr) => {{
        let mut row = 0_usize;
        while row < $rows {
            let slot = batch_paged_slot_v10!($positions, $tables, row, $stride, $pages);
            let mut other = 0_usize;
            while other < $rows {
                if other < row {
                    let previous =
                        batch_paged_slot_v10!($positions, $tables, other, $stride, $pages);
                    if slot == previous {
                        fe2o3_device::trap();
                    }
                }
                other += 1;
            }
            row += 1;
        }
    }};
}

#[cfg(test)]
macro_rules! batch_rope_pair_v10 {
    ($first:expr, $second:expr, $cos:expr, $sin:expr) => {{
        let first = Bf16::from_bits($first).to_f32();
        let second = Bf16::from_bits($second).to_f32();
        let cos = $cos;
        let sin = $sin;
        let rotated_first = first * cos - second * sin;
        let rotated_second = second * cos + first * sin;
        let first_out = Bf16::from_f32(rotated_first);
        let second_out = Bf16::from_f32(rotated_second);
        if !first.is_finite()
            || !second.is_finite()
            || !cos.is_finite()
            || !sin.is_finite()
            || !rotated_first.is_finite()
            || !rotated_second.is_finite()
            || !first_out.is_finite()
            || !second_out.is_finite()
        {
            fe2o3_device::trap();
        }
        (first_out.to_bits(), second_out.to_bits())
    }};
}

#[cfg(test)]
mod tests;

/// One wave per token row; trig tables contain that row's absolute position.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [32, 1, 1]), control_flow(loop_bounds(16, 8)))]
#[allow(clippy::too_many_arguments)]
pub fn ferric_qwen3_draft_batch32_rope_v10(
    query: &[u16],
    key: &[u16],
    cos: &[f32],
    sin: &[f32],
    positions: &[u32],
    mut rotated_query: WriteOnlyDisjointSlice<u16, RowStriped2D<Index1D, 64, 64>>,
    mut rotated_key: WriteOnlyDisjointSlice<u16, RowStriped2D<Index1D, 64, 64>>,
    rows: u32,
    world_size: u32,
) {
    if rows == 0 || rows > 32 || world_size != 1 {
        fe2o3_device::trap();
    }
    let query_heads = 16_u32;
    let kv_heads = 8_u32;
    let query_heads = query_heads as usize;
    let kv_heads = kv_heads as usize;
    let rows = rows as usize;
    if rows < 33 {
    } else {
        fe2o3_device::trap();
    }
    if query_heads < 17 {
    } else {
        fe2o3_device::trap();
    }
    if kv_heads < 9 {
    } else {
        fe2o3_device::trap();
    }
    let query_columns = query_heads * 128;
    let key_columns = kv_heads * 128;
    if query.len() < rows * query_columns
        || query.len() > 32 * query_columns
        || key.len() < rows * key_columns
        || key.len() > 32 * key_columns
        || rotated_query.len() < rows * query_columns
        || rotated_query.len() > 32 * query_columns
        || rotated_key.len() < rows * key_columns
        || rotated_key.len() > 32 * key_columns
        || cos.len() < rows * 64
        || cos.len() > 2048
        || sin.len() < rows * 64
        || sin.len() > 2048
        || positions.len() < rows
        || positions.len() > 32
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
    if memory::volatile_load(positions, row) >= 8192 {
        fe2o3_device::trap();
    }
    let cosine = memory::volatile_load(cos, row * 64 + lane);
    let sine = memory::volatile_load(sin, row * 64 + lane);
    let Some(stripe) = invocation.checked_row_striped_2d::<64, 64>() else {
        fe2o3_device::trap();
    };
    let mut head = 0_usize;
    while head < 16 {
        if head < query_heads {
            let index = row * query_columns + head * 128 + lane;
            if index < 65536 {
            } else {
                fe2o3_device::trap();
            }
            let (first, second) = {
                // BEGIN batch_rope_pair_v10
                let first = Bf16::from_bits(memory::volatile_load(query, index)).to_f32();
                let second = Bf16::from_bits(memory::volatile_load(query, index + 64)).to_f32();
                let cos = cosine;
                let sin = sine;
                let rotated_first = first * cos - second * sin;
                let rotated_second = second * cos + first * sin;
                let first_out = Bf16::from_f32(rotated_first);
                let second_out = Bf16::from_f32(rotated_second);
                if !first.is_finite()
                    || !second.is_finite()
                    || !cos.is_finite()
                    || !sin.is_finite()
                    || !rotated_first.is_finite()
                    || !rotated_second.is_finite()
                    || !first_out.is_finite()
                    || !second_out.is_finite()
                {
                    fe2o3_device::trap();
                }
                (first_out.to_bits(), second_out.to_bits())
                // END batch_rope_pair_v10
            };
            if !rotated_query.write_row_striped_2d(
                &stripe,
                head * 2,
                rows,
                query_columns,
                query_columns,
                first,
            ) || !rotated_query.write_row_striped_2d(
                &stripe,
                head * 2 + 1,
                rows,
                query_columns,
                query_columns,
                second,
            ) {
                fe2o3_device::trap();
            }
        }
        head += 1;
    }
    let mut head = 0_usize;
    while head < 8 {
        if head < kv_heads {
            let index = row * key_columns + head * 128 + lane;
            if index < 32768 {
            } else {
                fe2o3_device::trap();
            }
            let (first, second) = {
                // BEGIN batch_rope_pair_v10
                let first = Bf16::from_bits(memory::volatile_load(key, index)).to_f32();
                let second = Bf16::from_bits(memory::volatile_load(key, index + 64)).to_f32();
                let cos = cosine;
                let sin = sine;
                let rotated_first = first * cos - second * sin;
                let rotated_second = second * cos + first * sin;
                let first_out = Bf16::from_f32(rotated_first);
                let second_out = Bf16::from_f32(rotated_second);
                if !first.is_finite()
                    || !second.is_finite()
                    || !cos.is_finite()
                    || !sin.is_finite()
                    || !rotated_first.is_finite()
                    || !rotated_second.is_finite()
                    || !first_out.is_finite()
                    || !second_out.is_finite()
                {
                    fe2o3_device::trap();
                }
                (first_out.to_bits(), second_out.to_bits())
                // END batch_rope_pair_v10
            };
            if !rotated_key.write_row_striped_2d(
                &stripe,
                head * 2,
                rows,
                key_columns,
                key_columns,
                first,
            ) || !rotated_key.write_row_striped_2d(
                &stripe,
                head * 2 + 1,
                rows,
                key_columns,
                key_columns,
                second,
            ) {
                fe2o3_device::trap();
            }
        }
        head += 1;
    }
}

/// One grid leader scatters only selected rows after validating every slot.
/// Physical ownership and immutable-prefix copy-on-write remain host contracts.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [1, 1, 1]), control_flow(loop_bounds(32, 32, 32, 1024)))]
#[allow(clippy::too_many_arguments)]
pub fn ferric_qwen3_draft_batch32_paged_kv_append_v10(
    key: &[u16],
    value: &[u16],
    positions: &[u32],
    page_table: &[u32],
    mut key_cache: WriteOnlyDisjointSlice<u16, GridExclusive>,
    mut value_cache: WriteOnlyDisjointSlice<u16, GridExclusive>,
    rows: u32,
    world_size: u32,
    max_pages_per_sequence: u32,
    physical_pages: u32,
) {
    if rows == 0
        || rows > 32
        || world_size != 1
        || max_pages_per_sequence == 0
        || max_pages_per_sequence > 512
        || physical_pages == 0
        || physical_pages > 512
    {
        fe2o3_device::trap();
    }
    let kv_heads = 8_u32;
    let rows = rows as usize;
    let kv_heads = kv_heads as usize;
    let max_pages_per_sequence = max_pages_per_sequence as usize;
    let physical_pages = physical_pages as usize;
    if rows < 33 {
    } else {
        fe2o3_device::trap();
    }
    if kv_heads < 9 {
    } else {
        fe2o3_device::trap();
    }
    if max_pages_per_sequence < 513 {
    } else {
        fe2o3_device::trap();
    }
    if physical_pages < 513 {
    } else {
        fe2o3_device::trap();
    }
    let columns = kv_heads * 128;
    if columns < 1025 {
    } else {
        fe2o3_device::trap();
    }
    if key.len() < rows * columns
        || key.len() > 32 * columns
        || value.len() < rows * columns
        || value.len() > 32 * columns
        || positions.len() < rows
        || positions.len() > 32
        || page_table.len() < rows * max_pages_per_sequence
        || page_table.len() > 32 * max_pages_per_sequence
        || key_cache.len() != physical_pages * 16 * columns
        || value_cache.len() != physical_pages * 16 * columns
        || thread::launch_extent_1d() != 64
    {
        fe2o3_device::trap();
    }
    let Some(leader) = thread::grid_leader() else {
        return;
    };
    {
        // BEGIN batch_distinct_slots_v10
        let mut row = 0_usize;
        while row < rows {
            let slot = {
                // BEGIN batch_paged_slot_v10
                let row = row;
                let stride = max_pages_per_sequence;
                let pages = physical_pages;
                let position = memory::volatile_load(positions, row) as usize;
                if position < 8192 {
                } else {
                    fe2o3_device::trap();
                }
                let logical_page = position / 16;
                if logical_page < stride {
                } else {
                    fe2o3_device::trap();
                }
                let table_index = row * stride + logical_page;
                if table_index < page_table.len() {
                } else {
                    fe2o3_device::trap();
                }
                let physical_page = memory::volatile_load(page_table, table_index) as usize;
                if physical_page < pages {
                } else {
                    fe2o3_device::trap();
                }
                physical_page * 16 + position % 16
                // END batch_paged_slot_v10
            };
            let mut other = 0_usize;
            while other < rows {
                if other < row {
                    let previous = {
                        // BEGIN batch_paged_slot_v10
                        let row = other;
                        let stride = max_pages_per_sequence;
                        let pages = physical_pages;
                        let position = memory::volatile_load(positions, row) as usize;
                        if position < 8192 {
                        } else {
                            fe2o3_device::trap();
                        }
                        let logical_page = position / 16;
                        if logical_page < stride {
                        } else {
                            fe2o3_device::trap();
                        }
                        let table_index = row * stride + logical_page;
                        if table_index < page_table.len() {
                        } else {
                            fe2o3_device::trap();
                        }
                        let physical_page = memory::volatile_load(page_table, table_index) as usize;
                        if physical_page < pages {
                        } else {
                            fe2o3_device::trap();
                        }
                        physical_page * 16 + position % 16
                        // END batch_paged_slot_v10
                    };
                    if slot == previous {
                        fe2o3_device::trap();
                    }
                }
                other += 1;
            }
            row += 1;
        }
        // END batch_distinct_slots_v10
    };
    let mut row = 0_usize;
    while row < rows {
        let slot = {
            // BEGIN batch_paged_slot_v10
            let row = row;
            let stride = max_pages_per_sequence;
            let pages = physical_pages;
            let position = memory::volatile_load(positions, row) as usize;
            if position < 8192 {
            } else {
                fe2o3_device::trap();
            }
            let logical_page = position / 16;
            if logical_page < stride {
            } else {
                fe2o3_device::trap();
            }
            let table_index = row * stride + logical_page;
            if table_index < page_table.len() {
            } else {
                fe2o3_device::trap();
            }
            let physical_page = memory::volatile_load(page_table, table_index) as usize;
            if physical_page < pages {
            } else {
                fe2o3_device::trap();
            }
            physical_page * 16 + position % 16
            // END batch_paged_slot_v10
        };
        if slot < 8192 {
        } else {
            fe2o3_device::trap();
        }
        let base = slot * columns;
        let mut component = 0_usize;
        while component < 1024 {
            if component < columns {
                let source_index = row * columns + component;
                let key_value = memory::volatile_load(key, source_index);
                let value_value = memory::volatile_load(value, source_index);
                if !key_cache.write_exclusive(&leader, base + component, key_value)
                    || !value_cache.write_exclusive(&leader, base + component, value_value)
                {
                    fe2o3_device::trap();
                }
            }
            component += 1;
        }
        row += 1;
    }
}
