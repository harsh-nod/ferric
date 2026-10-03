use fe2o3_device::{
    Bf16, Gfx950Subgroup, Index1D, Math, RowStriped2D, StridedReadView2D, WriteOnlyDisjointSlice,
    kernel, thread,
};

const ATTENTION_SCALE: f32 = f32::from_bits(0x3db5_04f3);

/// One Wave64 owns one head/chunk and writes [numerator128, max, sum, padding62].
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [288, 1, 1]), control_flow(loop_bounds(128)))]
#[allow(clippy::too_many_arguments)]
pub fn ferric_qwen3_tp_split_context_partial_bf16_v1(
    query: &[u16],
    key_cache: &[u16],
    value_cache: &[u16],
    positions: &[u32],
    page_table: &[u32],
    mut partials: WriteOnlyDisjointSlice<f32, RowStriped2D<Index1D, 64, 3>>,
    rows: u32,
    world_size: u32,
    max_pages_per_sequence: u32,
    physical_pages: u32,
    active_context: u32,
) {
    if max_pages_per_sequence < 145 {
    } else {
        fe2o3_device::trap();
    }
    if rows != 1
        || world_size != 2
        || !(max_pages_per_sequence == 4 || max_pages_per_sequence == 144)
        || physical_pages != max_pages_per_sequence
        || active_context == 0
        || active_context > max_pages_per_sequence * 16
    {
        fe2o3_device::trap();
    }
    let pages = max_pages_per_sequence as usize;
    let active_context = active_context as usize;
    if pages < 145 && active_context < 2305 {
    } else {
        fe2o3_device::trap();
    }
    let pages = pages as u16 as usize;
    let splits = if pages == 4 { 1_usize } else { 18 };
    if splits > 0 && splits < 19 {
    } else {
        fe2o3_device::trap();
    }
    let partial_rows = 16 * splits;
    if query.len() < 2048
        || query.len() > 32768
        || key_cache.len() != pages * 16 * 512
        || value_cache.len() != pages * 16 * 512
        || positions.len() < rows as usize
        || positions.len() > 16
        || page_table.len() < pages
        || page_table.len() > 16 * pages
        || partials.len() != partial_rows * 192
        || thread::launch_extent_1d() != partial_rows * 64
    {
        fe2o3_device::trap();
    }
    let Ok(query_view) = StridedReadView2D::from_shared_slice(query, 0, 16, 128, 128) else {
        fe2o3_device::trap();
    };
    let Ok(key_view) = StridedReadView2D::from_shared_slice(key_cache, 0, pages * 16, 512, 512)
    else {
        fe2o3_device::trap();
    };
    let Ok(value_view) = StridedReadView2D::from_shared_slice(value_cache, 0, pages * 16, 512, 512)
    else {
        fe2o3_device::trap();
    };
    let Ok(position_view) = StridedReadView2D::from_shared_slice(positions, 0, 1, 1, 1) else {
        fe2o3_device::trap();
    };
    let Ok(table_view) = StridedReadView2D::from_shared_slice(page_table, 0, 1, pages, pages)
    else {
        fe2o3_device::trap();
    };
    let invocation = thread::index_1d();
    let lane = invocation.get() % 64;
    let partial_row = thread::block_idx_x() as usize;
    if partial_row < partial_rows && partial_rows < 289 {
    } else {
        fe2o3_device::trap();
    }
    let head = if pages == 4 { partial_row } else { partial_row / 18 };
    let split = if pages == 4 { 0 } else { partial_row % 18 };
    if head < 16 && split < 18 {
    } else {
        fe2o3_device::trap();
    }
    let head = head as u16 as usize;
    let split = split as u16 as usize;
    let kv_head = head / 4;
    let subgroup = Gfx950Subgroup::current();
    // Valid metadata is <=2303; f32 preserves it exactly during wave broadcast.
    let position =
        subgroup.broadcast_f32::<64>(position_view.load_or(0, 0, u32::MAX) as f32, 0) as usize;
    if position < pages * 16 && position + 1 == active_context {
    } else {
        fe2o3_device::trap();
    }
    let query_0 = Bf16::from_bits(query_view.load_or(head, lane, 0)).to_f32();
    let query_1 = Bf16::from_bits(query_view.load_or(head, lane + 64, 0)).to_f32();
    let math = Math::current();
    let mut maximum = 0.0_f32;
    let mut denominator = 0.0_f32;
    let mut numerator_0 = 0.0_f32;
    let mut numerator_1 = 0.0_f32;
    let mut finite = query_0.is_finite() & query_1.is_finite();
    let start = split * 128;
    let mut offset = 0_usize;
    // Empty trailing chunks do not inspect future page-table entries or KV data.
    while offset < 128 {
        if start + offset < active_context {
            let token = start + offset;
            if token <= position && token / 16 < pages {
            } else {
                fe2o3_device::trap();
            }
            let physical_page = subgroup
                .broadcast_f32::<64>(table_view.load_or(0, token / 16, u32::MAX) as f32, 0)
                as usize;
            if physical_page < pages && physical_page < 144 {
            } else {
                fe2o3_device::trap();
            }
            let physical_page = physical_page as u16 as usize;
            let cache_row = physical_page * 16 + token % 16;
            let cache_column = kv_head * 128 + lane;
            let key_0 = Bf16::from_bits(key_view.load_or(cache_row, cache_column, 0)).to_f32();
            let key_1 = Bf16::from_bits(key_view.load_or(cache_row, cache_column + 64, 0)).to_f32();
            let product_0 = query_0 * key_0;
            let product_1 = query_1 * key_1;
            let pair = product_0 + product_1;
            let dot = subgroup.reduce_sum_f32::<64>(pair);
            let score = dot * ATTENTION_SCALE;
            let value_0 = Bf16::from_bits(value_view.load_or(cache_row, cache_column, 0)).to_f32();
            let value_1 = Bf16::from_bits(value_view.load_or(cache_row, cache_column + 64, 0)).to_f32();
            finite &= product_0.is_finite()
                & product_1.is_finite()
                & pair.is_finite()
                & dot.is_finite()
                & score.is_finite()
                & value_0.is_finite()
                & value_1.is_finite();
            if offset == 0 {
                maximum = score;
                denominator = 1.0;
                numerator_0 = value_0;
                numerator_1 = value_1;
            } else {
                let next_maximum = if score > maximum { score } else { maximum };
                let previous_weight = math.exp_f32(maximum - next_maximum);
                let current_weight = math.exp_f32(score - next_maximum);
                denominator = denominator * previous_weight + current_weight;
                numerator_0 = numerator_0 * previous_weight + value_0 * current_weight;
                numerator_1 = numerator_1 * previous_weight + value_1 * current_weight;
                finite &= previous_weight.is_finite()
                    & (previous_weight >= 0.0)
                    & current_weight.is_finite()
                    & (current_weight >= 0.0)
                    & denominator.is_finite()
                    & (denominator > 0.0)
                    & numerator_0.is_finite()
                    & numerator_1.is_finite();
                maximum = next_maximum;
            }
        }
        offset += 1;
    }
    // No lane-dependent numeric exit may precede a later wave collective.
    if !finite || !maximum.is_finite() {
        fe2o3_device::trap();
    }
    let header = if lane == 0 {
        maximum
    } else if lane == 1 {
        denominator
    } else {
        0.0
    };
    let Some(stripe) = invocation.checked_row_striped_2d::<64, 3>() else {
        fe2o3_device::trap();
    };
    if !partials.write_row_striped_2d(&stripe, 0, partial_rows, 192, 192, numerator_0)
        || !partials.write_row_striped_2d(&stripe, 1, partial_rows, 192, 192, numerator_1)
        || !partials.write_row_striped_2d(&stripe, 2, partial_rows, 192, 192, header)
    {
        fe2o3_device::trap();
    }
}

/// Merge FP32 chunk states in order, then narrow each output component once.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [16, 1, 1]), control_flow(loop_bounds(18)))]
pub fn ferric_qwen3_tp_split_context_merge_bf16_v1(
    partials: &[f32],
    positions: &[u32],
    mut output: WriteOnlyDisjointSlice<u16, RowStriped2D<Index1D, 64, 2>>,
    rows: u32,
    world_size: u32,
    max_pages_per_sequence: u32,
    active_context: u32,
) {
    if max_pages_per_sequence < 145 {
    } else {
        fe2o3_device::trap();
    }
    if rows != 1
        || world_size != 2
        || !(max_pages_per_sequence == 4 || max_pages_per_sequence == 144)
        || active_context == 0
        || active_context > max_pages_per_sequence * 16
    {
        fe2o3_device::trap();
    }
    let pages = max_pages_per_sequence as usize;
    let active_context = active_context as usize;
    if pages < 145 && active_context < 2305 {
    } else {
        fe2o3_device::trap();
    }
    let splits = if pages == 4 { 1_usize } else { 18 };
    if splits > 0 && splits < 19 {
    } else {
        fe2o3_device::trap();
    }
    let partial_rows = 16 * splits;
    if partials.len() != partial_rows * 192
        || positions.len() < rows as usize
        || positions.len() > 16
        || output.len() < 2048
        || output.len() > 32768
        || thread::launch_extent_1d() != 1024
    {
        fe2o3_device::trap();
    }
    let Ok(partial_view) =
        StridedReadView2D::from_shared_slice(partials, 0, partial_rows, 192, 192)
    else {
        fe2o3_device::trap();
    };
    let Ok(position_view) = StridedReadView2D::from_shared_slice(positions, 0, 1, 1, 1) else {
        fe2o3_device::trap();
    };
    let invocation = thread::index_1d();
    let lane = invocation.get() % 64;
    let head = thread::block_idx_x() as usize;
    if head < 16 {
    } else {
        fe2o3_device::trap();
    }
    let head = head as u16 as usize;
    let subgroup = Gfx950Subgroup::current();
    let position =
        subgroup.broadcast_f32::<64>(position_view.load_or(0, 0, u32::MAX) as f32, 0) as usize;
    if position < pages * 16 && position + 1 == active_context {
    } else {
        fe2o3_device::trap();
    }
    let math = Math::current();
    let mut maximum = 0.0_f32;
    let mut denominator = 0.0_f32;
    let mut numerator_0 = 0.0_f32;
    let mut numerator_1 = 0.0_f32;
    let mut finite = true;
    let mut split = 0_usize;
    while split < 18 {
        if split < splits {
            let partial_row = head * splits + split;
            let local_maximum = if lane == 0 {
                partial_view.load_or(partial_row, 128, 0.0)
            } else {
                0.0
            };
            let local_denominator = if lane == 0 {
                partial_view.load_or(partial_row, 129, 0.0)
            } else {
                0.0
            };
            let chunk_maximum = subgroup.broadcast_f32::<64>(local_maximum, 0);
            let chunk_denominator = subgroup.broadcast_f32::<64>(local_denominator, 0);
            let chunk_numerator_0 = partial_view.load_or(partial_row, lane, 0.0);
            let chunk_numerator_1 = partial_view.load_or(partial_row, lane + 64, 0.0);
            if split * 128 < active_context {
                finite &= chunk_maximum.is_finite()
                    & chunk_denominator.is_finite()
                    & (chunk_denominator >= 1.0)
                    & chunk_numerator_0.is_finite()
                    & chunk_numerator_1.is_finite();
                if split == 0 {
                    maximum = chunk_maximum;
                    denominator = chunk_denominator;
                    numerator_0 = chunk_numerator_0;
                    numerator_1 = chunk_numerator_1;
                } else {
                    let next_maximum = if chunk_maximum > maximum {
                        chunk_maximum
                    } else {
                        maximum
                    };
                    let previous_weight = math.exp_f32(maximum - next_maximum);
                    let current_weight = math.exp_f32(chunk_maximum - next_maximum);
                    denominator = denominator * previous_weight + chunk_denominator * current_weight;
                    numerator_0 = numerator_0 * previous_weight + chunk_numerator_0 * current_weight;
                    numerator_1 = numerator_1 * previous_weight + chunk_numerator_1 * current_weight;
                    finite &= previous_weight.is_finite()
                        & (previous_weight >= 0.0)
                        & current_weight.is_finite()
                        & (current_weight >= 0.0)
                        & denominator.is_finite()
                        & (denominator > 0.0)
                        & numerator_0.is_finite()
                        & numerator_1.is_finite();
                    maximum = next_maximum;
                }
            } else {
                // Neutral-state validation catches stale data in an inactive chunk.
                finite &= chunk_maximum == 0.0
                    && chunk_denominator == 0.0
                    && chunk_numerator_0 == 0.0
                    && chunk_numerator_1 == 0.0;
            }
        }
        split += 1;
    }
    let first = numerator_0 / denominator;
    let second = numerator_1 / denominator;
    let narrowed_first = Bf16::from_f32(first);
    let narrowed_second = Bf16::from_f32(second);
    if !finite
        || !first.is_finite()
        || !second.is_finite()
        || !narrowed_first.is_finite()
        || !narrowed_second.is_finite()
    {
        fe2o3_device::trap();
    }
    let Some(stripe) = invocation.checked_row_striped_2d::<64, 2>() else {
        fe2o3_device::trap();
    };
    if !output.write_row_striped_2d(&stripe, 0, 16, 128, 128, narrowed_first.to_bits())
        || !output.write_row_striped_2d(&stripe, 1, 16, 128, 128, narrowed_second.to_bits())
    {
        fe2o3_device::trap();
    }
}
