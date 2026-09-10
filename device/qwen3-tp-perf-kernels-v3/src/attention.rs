use fe2o3_device::{
    Bf16, Gfx950Subgroup, Index1D, Math, RowStriped2D, WriteOnlyDisjointSlice, kernel, memory,
    thread,
};

const ATTENTION_SCALE: f32 = f32::from_bits(0x3db5_04f3);

/// One Wave64 per token-row/query-head pair over its own logical page table.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [512, 1, 1]), control_flow(loop_bounds(8192)))]
#[allow(clippy::too_many_arguments)]
pub fn ferric_qwen3_tp_wave_paged_gqa_bf16_v3(
    query: &[u16],
    key_cache: &[u16],
    value_cache: &[u16],
    positions: &[u32],
    page_table: &[u32],
    mut output: WriteOnlyDisjointSlice<u16, RowStriped2D<Index1D, 64, 2>>,
    rows: u32,
    world_size: u32,
    max_pages_per_sequence: u32,
    physical_pages: u32,
    max_context_tokens: u32,
) {
    if rows == 0
        || rows > 16
        || !(world_size == 1 || world_size == 2 || world_size == 8)
        || max_pages_per_sequence == 0
        || max_pages_per_sequence > 512
        || physical_pages == 0
        || physical_pages > 512
        || max_context_tokens == 0
        || max_context_tokens > 8192
    {
        fe2o3_device::trap();
    }
    let query_heads = if world_size == 1 {
        32_u32
    } else if world_size == 2 {
        16
    } else {
        4
    };
    let kv_heads = if world_size == 1 {
        8_u32
    } else if world_size == 2 {
        4
    } else {
        1
    };
    let rows = rows as usize;
    let query_heads = query_heads as usize;
    let kv_heads = kv_heads as usize;
    let max_pages_per_sequence = max_pages_per_sequence as usize;
    let physical_pages = physical_pages as usize;
    let max_context_tokens = max_context_tokens as usize;
    if rows < 17 {
    } else {
        fe2o3_device::trap();
    }
    if query_heads < 33 {
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
    if max_context_tokens < 8193 {
    } else {
        fe2o3_device::trap();
    }
    let columns = kv_heads * 128;
    let head_rows = rows * query_heads;
    if query.len() < head_rows * 128
        || query.len() > 16 * query_heads * 128
        || output.len() < head_rows * 128
        || output.len() > 16 * query_heads * 128
        || key_cache.len() != physical_pages * 16 * columns
        || value_cache.len() != physical_pages * 16 * columns
        || positions.len() < rows
        || positions.len() > 16
        || page_table.len() < rows * max_pages_per_sequence
        || page_table.len() > 16 * max_pages_per_sequence
        || max_context_tokens > max_pages_per_sequence * 16
        || thread::launch_extent_1d() != head_rows * 64
    {
        fe2o3_device::trap();
    }
    if columns < 1025 {
    } else {
        fe2o3_device::trap();
    }
    // The validated TP geometry fits u16; retain that bound in loop fragments.
    let columns = columns as u16 as usize;
    let invocation = thread::index_1d();
    let raw = invocation.get();
    let head_row = thread::block_idx_x() as usize;
    let lane = raw % 64;
    if head_row < head_rows {
    } else {
        fe2o3_device::trap();
    }
    let row = if world_size == 1 {
        head_row / 32
    } else if world_size == 2 {
        head_row / 16
    } else {
        head_row / 4
    };
    let query_head = if world_size == 1 {
        head_row % 32
    } else if world_size == 2 {
        head_row % 16
    } else {
        head_row % 4
    };
    if row < rows {
    } else {
        fe2o3_device::trap();
    }
    let kv_head = query_head / 4;
    if kv_head < kv_heads {
    } else {
        fe2o3_device::trap();
    }
    let kv_head = kv_head as u16 as usize;
    let subgroup = Gfx950Subgroup::current();
    // Broadcast exact bits so metadata guards are explicitly Wave64-uniform.
    let position = subgroup
        .broadcast_f32::<64>(f32::from_bits(memory::volatile_load(positions, row)), 0)
        .to_bits() as usize;
    if position < max_context_tokens {
    } else {
        fe2o3_device::trap();
    }
    let query_base = head_row * 128;
    if query_base < 65536 {
    } else {
        fe2o3_device::trap();
    }
    let math = Math::current();
    let (first, second) = {
        // Each lane computes two QK products; six shuffles share the full score.
        let mut maximum = 0.0_f32;
        let mut denominator = 0.0_f32;
        let mut numerator_0 = 0.0_f32;
        let mut numerator_1 = 0.0_f32;
        let mut finite = true;
        let mut token = 0_usize;
        while token < max_context_tokens {
            if token <= position {
                let table_index = row * max_pages_per_sequence + token / 16;
                if table_index < page_table.len() {
                } else {
                    fe2o3_device::trap();
                }
                let physical_page = subgroup
                    .broadcast_f32::<64>(
                        f32::from_bits(memory::volatile_load(page_table, table_index)),
                        0,
                    )
                    .to_bits() as usize;
                if physical_page < physical_pages {
                } else {
                    fe2o3_device::trap();
                }
                if physical_page < 512 {
                } else {
                    fe2o3_device::trap();
                }
                let cache_base = (physical_page * 16 + token % 16) * columns + kv_head * 128;
                if cache_base < 8388481 {
                } else {
                    fe2o3_device::trap();
                }
                let query_0 =
                    Bf16::from_bits(memory::volatile_load(query, query_base + lane)).to_f32();
                let query_1 =
                    Bf16::from_bits(memory::volatile_load(query, query_base + lane + 64)).to_f32();
                let key_0 =
                    Bf16::from_bits(memory::volatile_load(key_cache, cache_base + lane)).to_f32();
                let key_1 =
                    Bf16::from_bits(memory::volatile_load(key_cache, cache_base + lane + 64))
                        .to_f32();
                let product_0 = query_0 * key_0;
                let product_1 = query_1 * key_1;
                let partial = product_0 + product_1;
                finite &= product_0.is_finite() & product_1.is_finite() & partial.is_finite();
                let dot = subgroup.reduce_sum_f32::<64>(partial);
                if !dot.is_finite() {
                    fe2o3_device::trap();
                }
                let score = dot * ATTENTION_SCALE;
                let value_0 =
                    Bf16::from_bits(memory::volatile_load(value_cache, cache_base + lane)).to_f32();
                let value_1 =
                    Bf16::from_bits(memory::volatile_load(value_cache, cache_base + lane + 64))
                        .to_f32();
                finite &= score.is_finite() & value_0.is_finite() & value_1.is_finite();
                if token == 0 {
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
            token += 1;
        }
        let output_0 = numerator_0 / denominator;
        let output_1 = numerator_1 / denominator;
        let narrowed_0 = Bf16::from_f32(output_0);
        let narrowed_1 = Bf16::from_f32(output_1);
        // Numeric failures cannot terminate individual lanes before a collective.
        if !finite
            || !output_0.is_finite()
            || !output_1.is_finite()
            || !narrowed_0.is_finite()
            || !narrowed_1.is_finite()
        {
            fe2o3_device::trap();
        }
        (narrowed_0.to_bits(), narrowed_1.to_bits())
    };
    let Some(stripe) = invocation.checked_row_striped_2d::<64, 2>() else {
        fe2o3_device::trap();
    };
    if !output.write_row_striped_2d(&stripe, 0, head_rows, 128, 128, first)
        || !output.write_row_striped_2d(&stripe, 1, head_rows, 128, 128, second)
    {
        fe2o3_device::trap();
    }
}
