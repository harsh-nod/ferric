use fe2o3_device::{GridExclusive, WriteOnlyDisjointSlice, kernel, memory, thread};

#[cfg(test)]
macro_rules! batch_paged_slot_v9 {
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
macro_rules! batch_distinct_slots_v9 {
    ($positions:expr, $tables:expr, $rows:expr, $stride:expr, $pages:expr) => {{
        let mut row = 0_usize;
        while row < $rows {
            let slot = batch_paged_slot_v9!($positions, $tables, row, $stride, $pages);
            let mut other = 0_usize;
            while other < $rows {
                if other < row {
                    let previous =
                        batch_paged_slot_v9!($positions, $tables, other, $stride, $pages);
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

/// One grid leader scatters only selected rows after validating every slot.
/// Physical ownership and immutable-prefix copy-on-write remain host contracts.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [1, 1, 1]), control_flow(loop_bounds(32, 32, 32, 1024)))]
#[allow(clippy::too_many_arguments)]
pub fn ferric_qwen3_tp_batch32_large_kv_append_v9(
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
        || physical_pages > 16384
    {
        fe2o3_device::trap();
    }
    let kv_heads = if world_size == 1 {
        8_u32
    } else if world_size == 2 {
        4
    } else {
        1
    };
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
    if physical_pages < 16385 {
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
        // BEGIN batch_distinct_slots_v9
        let mut row = 0_usize;
        while row < rows {
            let slot = {
                // BEGIN batch_paged_slot_v9
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
                // END batch_paged_slot_v9
            };
            let mut other = 0_usize;
            while other < rows {
                if other < row {
                    let previous = {
                        // BEGIN batch_paged_slot_v9
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
                        // END batch_paged_slot_v9
                    };
                    if slot == previous {
                        fe2o3_device::trap();
                    }
                }
                other += 1;
            }
            row += 1;
        }
        // END batch_distinct_slots_v9
    };
    let mut row = 0_usize;
    while row < rows {
        let slot = {
            // BEGIN batch_paged_slot_v9
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
            // END batch_paged_slot_v9
        };
        if slot < 262144 {
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

#[cfg(test)]
mod tests;
