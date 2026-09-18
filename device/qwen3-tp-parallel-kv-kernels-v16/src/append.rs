use fe2o3_device::{Index1D, RowStriped2D, WriteOnlyDisjointSlice, kernel, memory, thread};

/// One Wave64 owns each physical slot; only the selected slot copies u16 words.
/// Physical ownership and immutable-prefix copy-on-write remain host contracts.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [64, 1, 1]), control_flow(loop_bounds(16, 16, 16, 16)))]
#[allow(clippy::too_many_arguments)]
pub fn ferric_qwen3_tp_batch_parallel_kv_append_v16(
    key: &[u16],
    value: &[u16],
    positions: &[u32],
    page_table: &[u32],
    mut key_cache: WriteOnlyDisjointSlice<u16, RowStriped2D<Index1D, 64, 16>>,
    mut value_cache: WriteOnlyDisjointSlice<u16, RowStriped2D<Index1D, 64, 16>>,
    rows: u32,
    world_size: u32,
    max_pages_per_sequence: u32,
    physical_pages: u32,
) {
    if rows != 1
        || world_size != 1
        || max_pages_per_sequence != 4
        || physical_pages != 4
        || thread::grid_dim_x() != 64
        || thread::grid_dim_y() != 1
        || thread::grid_dim_z() != 1
    {
        fe2o3_device::trap();
    }
    if rows == 0
        || rows > 16
        || !(world_size == 1 || world_size == 2 || world_size == 8)
        || max_pages_per_sequence == 0
        || max_pages_per_sequence > 512
        || physical_pages == 0
        || physical_pages > 512
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
    if rows < 17 {
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
        || key.len() > 16 * columns
        || value.len() < rows * columns
        || value.len() > 16 * columns
        || positions.len() < rows
        || positions.len() > 16
        || page_table.len() < rows * max_pages_per_sequence
        || page_table.len() > 16 * max_pages_per_sequence
        || key_cache.len() != physical_pages * 16 * columns
        || value_cache.len() != physical_pages * 16 * columns
        || thread::launch_extent_1d() != 4096
    {
        fe2o3_device::trap();
    }
    {
        // BEGIN batch_distinct_slots_v2
        let mut row = 0_usize;
        while row < rows {
            let slot = {
                // BEGIN batch_paged_slot_v2
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
                // END batch_paged_slot_v2
            };
            let mut other = 0_usize;
            while other < rows {
                if other < row {
                    let previous = {
                        // BEGIN batch_paged_slot_v2
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
                        // END batch_paged_slot_v2
                    };
                    if slot == previous {
                        fe2o3_device::trap();
                    }
                }
                other += 1;
            }
            row += 1;
        }
        // END batch_distinct_slots_v2
    };
    let output_slot = thread::block_idx_x() as usize;
    let lane = thread::thread_idx_x() as usize;
    if lane < 64 {
    } else {
        fe2o3_device::trap();
    }
    let Some(output_row) = thread::index_1d().checked_row_striped_2d::<64, 16>() else {
        fe2o3_device::trap();
    };
    let physical_slots = physical_pages * 16;
    let chunks = columns / 64;
    let mut row = 0_usize;
    while row < rows {
        let slot = {
            // BEGIN batch_paged_slot_v2
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
            // END batch_paged_slot_v2
        };
        if slot < 8192 {
        } else {
            fe2o3_device::trap();
        }
        // The selected physical slot chooses a workgroup, never a dynamic
        // destination component. Every group completed the full prepass above.
        if slot == output_slot {
            let mut chunk = 0_usize;
            while chunk < 16 {
                if chunk < chunks {
                    let component = lane + chunk * 64;
                    let source_index = row * columns + component;
                    let key_value = memory::volatile_load(key, source_index);
                    let value_value = memory::volatile_load(value, source_index);
                    if !key_cache.write_row_striped_2d(
                        &output_row,
                        chunk,
                        physical_slots,
                        columns,
                        columns,
                        key_value,
                    ) || !value_cache.write_row_striped_2d(
                        &output_row,
                        chunk,
                        physical_slots,
                        columns,
                        columns,
                        value_value,
                    ) {
                        fe2o3_device::trap();
                    }
                }
                chunk += 1;
            }
        }
        row += 1;
    }
}
