use fe2o3_device::{
    Bf16, Gfx950Subgroup, Index1D, Math, RowStriped2D, StridedReadView2D, WriteOnlyDisjointSlice,
    kernel, thread,
};

const ATTENTION_SCALE: f32 = f32::from_bits(0x3db5_04f3);

/// One Wave64 per query-head/contiguous-context-partition pair.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [256, 1, 1]), control_flow(loop_bounds(32)))]
#[allow(
    clippy::too_many_arguments,
    clippy::manual_div_ceil,
    clippy::manual_range_contains,
    clippy::len_zero
)]
pub fn ferric_qwen3_tp_c1_split8_attention_partial_f32_v21(
    query: &[u16],
    key_cache: &[u16],
    value_cache: &[u16],
    positions: &[u32],
    page_table: &[u32],
    mut stats: WriteOnlyDisjointSlice<f32, RowStriped2D<Index1D, 64, 1>>,
    mut numerators: WriteOnlyDisjointSlice<f32, RowStriped2D<Index1D, 64, 2>>,
    rows: u32,
    world_size: u32,
    max_pages_per_sequence: u32,
    physical_pages: u32,
    max_context_tokens: u32,
) {
    if rows != 1
        || world_size != 1
        || max_pages_per_sequence == 0
        || max_pages_per_sequence > 512
        || physical_pages == 0
        || physical_pages > 512
        || max_context_tokens < 128
        || max_context_tokens > 256
    {
        fe2o3_device::trap();
    }
    let max_pages_per_sequence = max_pages_per_sequence as usize;
    let physical_pages = physical_pages as usize;
    let max_context_tokens = max_context_tokens as usize;
    if max_pages_per_sequence < 513 && physical_pages < 513 && max_context_tokens < 257 {
    } else {
        fe2o3_device::trap();
    }
    if query.len() < 4096
        || query.len() > 32 * 4096
        || key_cache.len() != physical_pages * 16 * 1024
        || value_cache.len() != physical_pages * 16 * 1024
        || positions.len() < 1
        || positions.len() > 32
        || page_table.len() < max_pages_per_sequence
        || page_table.len() > 32 * max_pages_per_sequence
        || max_context_tokens > max_pages_per_sequence * 16
        || stats.len() != 512
        || numerators.len() != 32768
        || thread::launch_extent_1d() != 256 * 64
    {
        fe2o3_device::trap();
    }
    let Ok(query_view) = StridedReadView2D::from_shared_slice(query, 0, 32, 128, 128) else {
        fe2o3_device::trap();
    };
    let Ok(key_view) =
        StridedReadView2D::from_shared_slice(key_cache, 0, physical_pages * 16, 1024, 1024)
    else {
        fe2o3_device::trap();
    };
    let Ok(value_view) =
        StridedReadView2D::from_shared_slice(value_cache, 0, physical_pages * 16, 1024, 1024)
    else {
        fe2o3_device::trap();
    };
    let Ok(position_view) = StridedReadView2D::from_shared_slice(positions, 0, 1, 1, 1) else {
        fe2o3_device::trap();
    };
    let Ok(table_view) = StridedReadView2D::from_shared_slice(
        page_table,
        0,
        1,
        max_pages_per_sequence,
        max_pages_per_sequence,
    ) else {
        fe2o3_device::trap();
    };
    let invocation = thread::index_1d();
    let lane = invocation.get() % 64;
    let partition_row = thread::block_idx_x() as usize;
    if partition_row < 256 {
    } else {
        fe2o3_device::trap();
    }
    let query_head = partition_row / 8;
    let partition = partition_row % 8;
    let kv_head = query_head / 4;
    if query_head < 32 && kv_head < 8 {
    } else {
        fe2o3_device::trap();
    }
    let query_head = query_head as u16 as usize;
    let kv_head = kv_head as u16 as usize;
    let subgroup = Gfx950Subgroup::current();
    let position =
        subgroup.broadcast_f32::<64>(position_view.load_or(0, 0, u32::MAX) as f32, 0) as usize;
    if position < 256 && position + 1 == max_context_tokens {
    } else {
        fe2o3_device::trap();
    }
    let span = (max_context_tokens + 7) / 8;
    if span > 0 && span < 33 {
    } else {
        fe2o3_device::trap();
    }
    let begin = partition * span;
    if begin < max_context_tokens {
    } else {
        fe2o3_device::trap();
    }
    let query_0 = Bf16::from_bits(query_view.load_or(query_head, lane, 0)).to_f32();
    let query_1 = Bf16::from_bits(query_view.load_or(query_head, lane + 64, 0)).to_f32();
    let math = Math::current();
    let mut maximum = 0.0_f32;
    let mut denominator = 0.0_f32;
    let mut numerator_0 = 0.0_f32;
    let mut numerator_1 = 0.0_f32;
    let mut finite = true;
    let mut offset = 0_usize;
    while offset < span {
        let token = begin + offset;
        if token < max_context_tokens {
            let physical_page = subgroup
                .broadcast_f32::<64>(table_view.load_or(0, token / 16, u32::MAX) as f32, 0)
                as usize;
            if physical_page < physical_pages && physical_page < 512 {
            } else {
                fe2o3_device::trap();
            }
            let cache_row = physical_page * 16 + token % 16;
            let cache_column = kv_head * 128 + lane;
            let key_0 = Bf16::from_bits(key_view.load_or(cache_row, cache_column, 0)).to_f32();
            let key_1 = Bf16::from_bits(key_view.load_or(cache_row, cache_column + 64, 0)).to_f32();
            let product_0 = query_0 * key_0;
            let product_1 = query_1 * key_1;
            let partial = product_0 + product_1;
            finite &= product_0.is_finite() & product_1.is_finite() & partial.is_finite();
            let dot = subgroup.reduce_sum_f32::<64>(partial);
            if !dot.is_finite() {
                fe2o3_device::trap();
            }
            let score = dot * ATTENTION_SCALE;
            let value_0 = Bf16::from_bits(value_view.load_or(cache_row, cache_column, 0)).to_f32();
            let value_1 =
                Bf16::from_bits(value_view.load_or(cache_row, cache_column + 64, 0)).to_f32();
            finite &= score.is_finite() & value_0.is_finite() & value_1.is_finite();
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
    if !finite
        || !maximum.is_finite()
        || !denominator.is_finite()
        || denominator <= 0.0
        || !numerator_0.is_finite()
        || !numerator_1.is_finite()
    {
        fe2o3_device::trap();
    }
    let Some(numerator_stripe) = invocation.checked_row_striped_2d::<64, 2>() else {
        fe2o3_device::trap();
    };
    if !numerators.write_row_striped_2d(&numerator_stripe, 0, 256, 128, 128, numerator_0)
        || !numerators.write_row_striped_2d(&numerator_stripe, 1, 256, 128, 128, numerator_1)
    {
        fe2o3_device::trap();
    }
    // Lanes 0/1 own the two scalar columns; all other lanes write numerators only.
    if lane < 2 {
        let Some(stats_stripe) = thread::index_1d().checked_row_striped_2d::<64, 1>() else {
            fe2o3_device::trap();
        };
        let value = if lane == 0 { maximum } else { denominator };
        if !stats.write_row_striped_2d(&stats_stripe, 0, 256, 2, 2, value) {
            fe2o3_device::trap();
        }
    }
}

/// Merge eight FP32 online-softmax states; this reassociates V14 arithmetic.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [32, 1, 1]), control_flow(loop_bounds(8)))]
#[allow(clippy::manual_range_contains)]
pub fn ferric_qwen3_tp_c1_split8_attention_merge_bf16_v21(
    stats: &[f32],
    numerators: &[f32],
    mut output: WriteOnlyDisjointSlice<u16, RowStriped2D<Index1D, 64, 2>>,
) {
    if stats.len() != 512
        || numerators.len() != 32768
        || output.len() < 4096
        || output.len() > 32 * 4096
        || thread::launch_extent_1d() != 32 * 64
    {
        fe2o3_device::trap();
    }
    let Ok(stats_view) = StridedReadView2D::from_shared_slice(stats, 0, 256, 2, 2) else {
        fe2o3_device::trap();
    };
    let Ok(numerator_view) = StridedReadView2D::from_shared_slice(numerators, 0, 256, 128, 128)
    else {
        fe2o3_device::trap();
    };
    let invocation = thread::index_1d();
    let lane = invocation.get() % 64;
    let query_head = thread::block_idx_x() as usize;
    if query_head < 32 {
    } else {
        fe2o3_device::trap();
    }
    let math = Math::current();
    let mut maximum = 0.0_f32;
    let mut denominator = 0.0_f32;
    let mut numerator_0 = 0.0_f32;
    let mut numerator_1 = 0.0_f32;
    let mut finite = true;
    let mut partition = 0_usize;
    while partition < 8 {
        let partition_row = query_head * 8 + partition;
        // Infinity remains fail-closed without requiring a NaN payload constant.
        let part_maximum = stats_view.load_or(partition_row, 0, f32::INFINITY);
        let part_denominator = stats_view.load_or(partition_row, 1, f32::INFINITY);
        let part_numerator_0 = numerator_view.load_or(partition_row, lane, f32::INFINITY);
        let part_numerator_1 = numerator_view.load_or(partition_row, lane + 64, f32::INFINITY);
        finite &= part_maximum.is_finite()
            & part_denominator.is_finite()
            & (part_denominator > 0.0)
            & part_numerator_0.is_finite()
            & part_numerator_1.is_finite();
        if partition == 0 {
            maximum = part_maximum;
            denominator = part_denominator;
            numerator_0 = part_numerator_0;
            numerator_1 = part_numerator_1;
        } else {
            let next_maximum = if part_maximum > maximum {
                part_maximum
            } else {
                maximum
            };
            let previous_weight = math.exp_f32(maximum - next_maximum);
            let current_weight = math.exp_f32(part_maximum - next_maximum);
            denominator = denominator * previous_weight + part_denominator * current_weight;
            numerator_0 = numerator_0 * previous_weight + part_numerator_0 * current_weight;
            numerator_1 = numerator_1 * previous_weight + part_numerator_1 * current_weight;
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
        partition += 1;
    }
    let result_0 = numerator_0 / denominator;
    let result_1 = numerator_1 / denominator;
    let narrowed_0 = Bf16::from_f32(result_0);
    let narrowed_1 = Bf16::from_f32(result_1);
    if !finite
        || !result_0.is_finite()
        || !result_1.is_finite()
        || !narrowed_0.is_finite()
        || !narrowed_1.is_finite()
    {
        fe2o3_device::trap();
    }
    let Some(stripe) = invocation.checked_row_striped_2d::<64, 2>() else {
        fe2o3_device::trap();
    };
    if !output.write_row_striped_2d(&stripe, 0, 32, 128, 128, narrowed_0.to_bits())
        || !output.write_row_striped_2d(&stripe, 1, 32, 128, 128, narrowed_1.to_bits())
    {
        fe2o3_device::trap();
    }
}
