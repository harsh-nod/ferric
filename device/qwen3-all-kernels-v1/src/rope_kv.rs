#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)] // The kernel macro emits undocumented helper modules.

//! Attributed Rust source for Ferric's exact Qwen3 K3 device roots.

use fe2o3_device::{
    Bf16, Blocked, Index1D, RowStriped2D, WriteOnlyDisjointSlice, kernel, memory, thread,
};

pub const QWEN3_ROPE_KERNEL_SYMBOL_V1: &str = "qwen3_rope_v1";
pub const QWEN3_PAGED_KV_WRITE_KERNEL_SYMBOL_V1: &str = "qwen3_paged_kv_write_v1";
pub const QWEN3_ROPE_KV_WORKGROUP_V1: [u32; 3] = [64, 1, 1];
pub const QWEN3_ROPE_MAX_GRID_WORKGROUPS_V1: u32 = 2_048;
pub const QWEN3_PAGED_KV_WRITE_GRID_WORKGROUPS_V1: u32 = 16_384;
pub const QWEN3_PAGED_KV_WRITE_GRID_WORKITEMS_V1: usize =
    QWEN3_PAGED_KV_WRITE_GRID_WORKGROUPS_V1 as usize * 64;
pub const QWEN3_ROPE_EXPLICIT_KERNARG_BYTES_V1: usize = 128;
pub const QWEN3_PAGED_KV_WRITE_EXPLICIT_KERNARG_BYTES_V1: usize = 112;
pub const QWEN3_ROPE_KV_HEAD_DIMENSION_V1: u32 = 128;
pub const QWEN3_ROPE_PAIR_COUNT_V1: u32 = 64;
pub const QWEN3_ROPE_KV_MAX_CONTEXT_TOKENS_V1: u32 = 8_192;
pub const QWEN3_KV_PAGE_TOKENS_V1: u32 = 16;
pub const QWEN3_KV_PAGE_TABLE_ENTRIES_V1: u32 = 512;
pub const QWEN3_KV_PHYSICAL_PAGE_SLOTS_V1: u32 = 16_384;
pub const QWEN3_KV_CACHE_ELEMENTS_V1: usize = 268_435_456;
pub const QWEN3_ROPE_TRIG_TABLE_ELEMENTS_V1: usize = 8_192 * 64;

macro_rules! common_profile_is_admitted_v1 {
    ($active:expr, $sequences:expr, $context:expr) => {
        (($active == 128 && ($sequences == 1 || $sequences == 8) && $context == 128)
            || ($active == 512 && $sequences == 1 && $context == 512)
            || ($active == 2_048 && $sequences == 1 && $context == 2_048)
            || ($active == 1
                && ($sequences == 1 || $sequences == 8 || $sequences == 32)
                && $context == 8_192))
    };
}

macro_rules! target_speculative_profile_is_admitted_v1 {
    ($active:expr, $sequences:expr, $context:expr) => {
        ($context == 8_192
            && (($active == 5 && ($sequences == 1 || $sequences == 8))
                || ($active == 9 && $sequences == 1)
                || ($active == 17 && $sequences == 1)))
    };
}

macro_rules! draft_speculative_profile_is_admitted_v1 {
    ($active:expr, $sequences:expr, $context:expr) => {
        ($context == 8_192
            && (($active == 4 && ($sequences == 1 || $sequences == 8))
                || ($active == 8 && $sequences == 1)
                || ($active == 16 && $sequences == 1)))
    };
}

macro_rules! rope_profile_is_admitted_v1 {
    ($active:expr, $sequences:expr, $query_heads:expr, $context:expr) => {
        ((common_profile_is_admitted_v1!($active, $sequences, $context)
            && ($query_heads == 16 || $query_heads == 32))
            || (target_speculative_profile_is_admitted_v1!($active, $sequences, $context)
                && $query_heads == 32)
            || (draft_speculative_profile_is_admitted_v1!($active, $sequences, $context)
                && $query_heads == 16))
    };
}

macro_rules! kv_profile_is_admitted_v1 {
    ($active:expr, $sequences:expr, $context:expr) => {
        (common_profile_is_admitted_v1!($active, $sequences, $context)
            || target_speculative_profile_is_admitted_v1!($active, $sequences, $context)
            || draft_speculative_profile_is_admitted_v1!($active, $sequences, $context))
    };
}

#[must_use]
pub const fn qwen3_rope_profile_is_admitted_v1(
    active_tokens: u32,
    sequences: u32,
    query_heads: u32,
    context_tokens: u32,
) -> bool {
    rope_profile_is_admitted_v1!(active_tokens, sequences, query_heads, context_tokens)
}

#[must_use]
pub const fn qwen3_paged_kv_write_profile_is_admitted_v1(
    active_tokens: u32,
    sequences: u32,
    context_tokens: u32,
) -> bool {
    kv_profile_is_admitted_v1!(active_tokens, sequences, context_tokens)
}

#[kernel(
    typed,
    launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [2048, 1, 1]),
    control_flow(loop_bounds(32, 8))
)]
#[allow(clippy::too_many_arguments)]
pub fn qwen3_rope_v1(
    query_bf16: &[u16],
    key_bf16: &[u16],
    position_ids: &[u32],
    cos_table_f32: &[f32],
    sin_table_f32: &[f32],
    mut rotated_query_bf16: WriteOnlyDisjointSlice<u16, RowStriped2D<Index1D, 64, 64>>,
    mut rotated_key_bf16: WriteOnlyDisjointSlice<u16, RowStriped2D<Index1D, 64, 16>>,
    active_tokens: u32,
    sequences: u32,
    query_heads: u32,
    context_tokens: u32,
) {
    let common_profile_is_admitted =
        (active_tokens == 128 && (sequences == 1 || sequences == 8) && context_tokens == 128)
            || (active_tokens == 512 && sequences == 1 && context_tokens == 512)
            || (active_tokens == 2_048 && sequences == 1 && context_tokens == 2_048)
            || (active_tokens == 1
                && (sequences == 1 || sequences == 8 || sequences == 32)
                && context_tokens == 8_192);
    let target_speculative_profile_is_admitted = context_tokens == 8_192
        && ((active_tokens == 5 && (sequences == 1 || sequences == 8))
            || (active_tokens == 9 && sequences == 1)
            || (active_tokens == 17 && sequences == 1));
    let draft_speculative_profile_is_admitted = context_tokens == 8_192
        && ((active_tokens == 4 && (sequences == 1 || sequences == 8))
            || (active_tokens == 8 && sequences == 1)
            || (active_tokens == 16 && sequences == 1));
    let profile_is_admitted = (common_profile_is_admitted
        && (query_heads == 16 || query_heads == 32))
        || (target_speculative_profile_is_admitted && query_heads == 32)
        || (draft_speculative_profile_is_admitted && query_heads == 16);
    if !profile_is_admitted {
        fe2o3_device::trap();
    }
    let active_tokens = active_tokens as usize;
    let sequences = sequences as usize;
    let query_heads = query_heads as usize;
    let context_tokens = context_tokens as usize;
    let rows = active_tokens * sequences;
    let query_columns = query_heads * 128;
    let key_columns = 8 * 128;
    if query_bf16.len() != rows * query_columns
        || key_bf16.len() != rows * key_columns
        || position_ids.len() != rows
        || cos_table_f32.len() != QWEN3_ROPE_TRIG_TABLE_ELEMENTS_V1
        || sin_table_f32.len() != QWEN3_ROPE_TRIG_TABLE_ELEMENTS_V1
        || rotated_query_bf16.len() != rows * query_columns
        || rotated_key_bf16.len() != rows * key_columns
        || thread::launch_extent_1d() != rows * 64
    {
        fe2o3_device::trap();
    }

    let query_index = thread::index_1d();
    let raw = query_index.get();
    let token_index = raw / 64;
    let lane = raw % 64;
    if token_index >= rows {
        fe2o3_device::trap();
    }
    let position = memory::volatile_load(position_ids, token_index) as usize;
    if position >= context_tokens || position >= QWEN3_ROPE_KV_MAX_CONTEXT_TOKENS_V1 as usize {
        fe2o3_device::trap();
    }
    let trig_index = position * 64 + lane;
    let cos = memory::volatile_load(cos_table_f32, trig_index);
    let sin = memory::volatile_load(sin_table_f32, trig_index);
    let Some(query_stripe) = query_index.checked_row_striped_2d::<64, 64>() else {
        fe2o3_device::trap();
    };
    let Some(key_stripe) = thread::index_1d().checked_row_striped_2d::<64, 16>() else {
        fe2o3_device::trap();
    };

    let mut head = 0_usize;
    while head < query_heads {
        let head_base = token_index * query_columns + head * 128;
        let first = head_base + lane;
        let second = first + 64;
        let first_value = Bf16::from_bits(memory::volatile_load(query_bf16, first)).to_f32();
        let second_value = Bf16::from_bits(memory::volatile_load(query_bf16, second)).to_f32();
        let first_cos = first_value * cos;
        let second_sin = second_value * sin;
        let rotated_first = first_cos - second_sin;
        let second_cos = second_value * cos;
        let first_sin = first_value * sin;
        let rotated_second = second_cos + first_sin;
        if !rotated_query_bf16.write_row_striped_2d(
            &query_stripe,
            head * 2,
            rows,
            query_columns,
            query_columns,
            Bf16::from_f32(rotated_first).to_bits(),
        ) || !rotated_query_bf16.write_row_striped_2d(
            &query_stripe,
            head * 2 + 1,
            rows,
            query_columns,
            query_columns,
            Bf16::from_f32(rotated_second).to_bits(),
        ) {
            fe2o3_device::trap();
        }
        head += 1;
    }

    let mut key_head = 0_usize;
    while key_head < 8 {
        let head_base = token_index * key_columns + key_head * 128;
        let first = head_base + lane;
        let second = first + 64;
        let first_value = Bf16::from_bits(memory::volatile_load(key_bf16, first)).to_f32();
        let second_value = Bf16::from_bits(memory::volatile_load(key_bf16, second)).to_f32();
        let first_cos = first_value * cos;
        let second_sin = second_value * sin;
        let rotated_first = first_cos - second_sin;
        let second_cos = second_value * cos;
        let first_sin = first_value * sin;
        let rotated_second = second_cos + first_sin;
        if !rotated_key_bf16.write_row_striped_2d(
            &key_stripe,
            key_head * 2,
            rows,
            key_columns,
            key_columns,
            Bf16::from_f32(rotated_first).to_bits(),
        ) || !rotated_key_bf16.write_row_striped_2d(
            &key_stripe,
            key_head * 2 + 1,
            rows,
            key_columns,
            key_columns,
            Bf16::from_f32(rotated_second).to_bits(),
        ) {
            fe2o3_device::trap();
        }
        key_head += 1;
    }
}

#[kernel(
    typed,
    launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [16384, 1, 1]),
    control_flow(loop_bounds(2048))
)]
#[allow(clippy::too_many_arguments)]
pub fn qwen3_paged_kv_write_v1(
    rotated_key_bf16: &[u16],
    value_bf16: &[u16],
    logical_starts: &[u32],
    page_indices: &[u32],
    mut key_cache_bf16: WriteOnlyDisjointSlice<u16, Blocked<Index1D, 64, 256>>,
    mut value_cache_bf16: WriteOnlyDisjointSlice<u16, Blocked<Index1D, 64, 256>>,
    active_tokens: u32,
    sequences: u32,
    context_tokens: u32,
) {
    let common_profile_is_admitted =
        (active_tokens == 128 && (sequences == 1 || sequences == 8) && context_tokens == 128)
            || (active_tokens == 512 && sequences == 1 && context_tokens == 512)
            || (active_tokens == 2_048 && sequences == 1 && context_tokens == 2_048)
            || (active_tokens == 1
                && (sequences == 1 || sequences == 8 || sequences == 32)
                && context_tokens == 8_192);
    let target_speculative_profile_is_admitted = context_tokens == 8_192
        && ((active_tokens == 5 && (sequences == 1 || sequences == 8))
            || (active_tokens == 9 && sequences == 1)
            || (active_tokens == 17 && sequences == 1));
    let draft_speculative_profile_is_admitted = context_tokens == 8_192
        && ((active_tokens == 4 && (sequences == 1 || sequences == 8))
            || (active_tokens == 8 && sequences == 1)
            || (active_tokens == 16 && sequences == 1));
    let profile_is_admitted = common_profile_is_admitted
        || target_speculative_profile_is_admitted
        || draft_speculative_profile_is_admitted;
    if !profile_is_admitted {
        fe2o3_device::trap();
    }
    let active_tokens = active_tokens as usize;
    let sequences = sequences as usize;
    let context_tokens = context_tokens as usize;
    let rows = active_tokens * sequences;
    let kv_columns = 8 * 128;
    if rotated_key_bf16.len() != rows * kv_columns
        || value_bf16.len() != rows * kv_columns
        || logical_starts.len() != sequences
        || page_indices.len() != sequences * QWEN3_KV_PAGE_TABLE_ENTRIES_V1 as usize
        || key_cache_bf16.len() != QWEN3_KV_CACHE_ELEMENTS_V1
        || value_cache_bf16.len() != QWEN3_KV_CACHE_ELEMENTS_V1
        || thread::launch_extent_1d() != QWEN3_PAGED_KV_WRITE_GRID_WORKITEMS_V1
    {
        fe2o3_device::trap();
    }

    let page_lane_index = thread::index_1d();
    let raw = page_lane_index.get();
    let owned_physical_page = raw / 64;
    let lane = raw % 64;
    if owned_physical_page >= QWEN3_KV_PHYSICAL_PAGE_SLOTS_V1 as usize {
        fe2o3_device::trap();
    }
    let Some(cache_block) = page_lane_index.checked_block::<64, 256>() else {
        fe2o3_device::trap();
    };

    let mut row = 0_usize;
    while row < rows {
        let sequence = row / active_tokens;
        let local_token = row % active_tokens;
        let logical_start = memory::volatile_load(logical_starts, sequence) as usize;
        if logical_start >= QWEN3_ROPE_KV_MAX_CONTEXT_TOKENS_V1 as usize {
            fe2o3_device::trap();
        }
        let logical_position = logical_start + local_token;
        if logical_position >= context_tokens
            || logical_position >= QWEN3_ROPE_KV_MAX_CONTEXT_TOKENS_V1 as usize
        {
            fe2o3_device::trap();
        }
        let logical_page = logical_position / QWEN3_KV_PAGE_TOKENS_V1 as usize;
        let token_in_page = logical_position % QWEN3_KV_PAGE_TOKENS_V1 as usize;
        if logical_page >= QWEN3_KV_PAGE_TABLE_ENTRIES_V1 as usize {
            fe2o3_device::trap();
        }
        let table_index = sequence * QWEN3_KV_PAGE_TABLE_ENTRIES_V1 as usize + logical_page;
        let physical_page = memory::volatile_load(page_indices, table_index) as usize;
        if physical_page >= QWEN3_KV_PHYSICAL_PAGE_SLOTS_V1 as usize {
            fe2o3_device::trap();
        }
        let input_base = row * kv_columns;

        if physical_page == owned_physical_page {
            let input_index_0 = input_base + lane;
            let key_component_0 = memory::volatile_load(rotated_key_bf16, input_index_0);
            let value_component_0 = memory::volatile_load(value_bf16, input_index_0);
            let input_index_1 = input_base + 64 + lane;
            let key_component_1 = memory::volatile_load(rotated_key_bf16, input_index_1);
            let value_component_1 = memory::volatile_load(value_bf16, input_index_1);
            let input_index_2 = input_base + 128 + lane;
            let key_component_2 = memory::volatile_load(rotated_key_bf16, input_index_2);
            let value_component_2 = memory::volatile_load(value_bf16, input_index_2);
            let input_index_3 = input_base + 192 + lane;
            let key_component_3 = memory::volatile_load(rotated_key_bf16, input_index_3);
            let value_component_3 = memory::volatile_load(value_bf16, input_index_3);
            let input_index_4 = input_base + 256 + lane;
            let key_component_4 = memory::volatile_load(rotated_key_bf16, input_index_4);
            let value_component_4 = memory::volatile_load(value_bf16, input_index_4);
            let input_index_5 = input_base + 320 + lane;
            let key_component_5 = memory::volatile_load(rotated_key_bf16, input_index_5);
            let value_component_5 = memory::volatile_load(value_bf16, input_index_5);
            let input_index_6 = input_base + 384 + lane;
            let key_component_6 = memory::volatile_load(rotated_key_bf16, input_index_6);
            let value_component_6 = memory::volatile_load(value_bf16, input_index_6);
            let input_index_7 = input_base + 448 + lane;
            let key_component_7 = memory::volatile_load(rotated_key_bf16, input_index_7);
            let value_component_7 = memory::volatile_load(value_bf16, input_index_7);
            let input_index_8 = input_base + 512 + lane;
            let key_component_8 = memory::volatile_load(rotated_key_bf16, input_index_8);
            let value_component_8 = memory::volatile_load(value_bf16, input_index_8);
            let input_index_9 = input_base + 576 + lane;
            let key_component_9 = memory::volatile_load(rotated_key_bf16, input_index_9);
            let value_component_9 = memory::volatile_load(value_bf16, input_index_9);
            let input_index_10 = input_base + 640 + lane;
            let key_component_10 = memory::volatile_load(rotated_key_bf16, input_index_10);
            let value_component_10 = memory::volatile_load(value_bf16, input_index_10);
            let input_index_11 = input_base + 704 + lane;
            let key_component_11 = memory::volatile_load(rotated_key_bf16, input_index_11);
            let value_component_11 = memory::volatile_load(value_bf16, input_index_11);
            let input_index_12 = input_base + 768 + lane;
            let key_component_12 = memory::volatile_load(rotated_key_bf16, input_index_12);
            let value_component_12 = memory::volatile_load(value_bf16, input_index_12);
            let input_index_13 = input_base + 832 + lane;
            let key_component_13 = memory::volatile_load(rotated_key_bf16, input_index_13);
            let value_component_13 = memory::volatile_load(value_bf16, input_index_13);
            let input_index_14 = input_base + 896 + lane;
            let key_component_14 = memory::volatile_load(rotated_key_bf16, input_index_14);
            let value_component_14 = memory::volatile_load(value_bf16, input_index_14);
            let input_index_15 = input_base + 960 + lane;
            let key_component_15 = memory::volatile_load(rotated_key_bf16, input_index_15);
            let value_component_15 = memory::volatile_load(value_bf16, input_index_15);
            if token_in_page == 0 {
                let component_0_written =
                    key_cache_bf16.write_block(&cache_block, 0, key_component_0)
                        & value_cache_bf16.write_block(&cache_block, 0, value_component_0);
                let component_1_written =
                    key_cache_bf16.write_block(&cache_block, 1, key_component_1)
                        & value_cache_bf16.write_block(&cache_block, 1, value_component_1);
                let component_2_written =
                    key_cache_bf16.write_block(&cache_block, 2, key_component_2)
                        & value_cache_bf16.write_block(&cache_block, 2, value_component_2);
                let component_3_written =
                    key_cache_bf16.write_block(&cache_block, 3, key_component_3)
                        & value_cache_bf16.write_block(&cache_block, 3, value_component_3);
                let component_4_written =
                    key_cache_bf16.write_block(&cache_block, 4, key_component_4)
                        & value_cache_bf16.write_block(&cache_block, 4, value_component_4);
                let component_5_written =
                    key_cache_bf16.write_block(&cache_block, 5, key_component_5)
                        & value_cache_bf16.write_block(&cache_block, 5, value_component_5);
                let component_6_written =
                    key_cache_bf16.write_block(&cache_block, 6, key_component_6)
                        & value_cache_bf16.write_block(&cache_block, 6, value_component_6);
                let component_7_written =
                    key_cache_bf16.write_block(&cache_block, 7, key_component_7)
                        & value_cache_bf16.write_block(&cache_block, 7, value_component_7);
                let component_8_written =
                    key_cache_bf16.write_block(&cache_block, 8, key_component_8)
                        & value_cache_bf16.write_block(&cache_block, 8, value_component_8);
                let component_9_written =
                    key_cache_bf16.write_block(&cache_block, 9, key_component_9)
                        & value_cache_bf16.write_block(&cache_block, 9, value_component_9);
                let component_10_written =
                    key_cache_bf16.write_block(&cache_block, 10, key_component_10)
                        & value_cache_bf16.write_block(&cache_block, 10, value_component_10);
                let component_11_written =
                    key_cache_bf16.write_block(&cache_block, 11, key_component_11)
                        & value_cache_bf16.write_block(&cache_block, 11, value_component_11);
                let component_12_written =
                    key_cache_bf16.write_block(&cache_block, 12, key_component_12)
                        & value_cache_bf16.write_block(&cache_block, 12, value_component_12);
                let component_13_written =
                    key_cache_bf16.write_block(&cache_block, 13, key_component_13)
                        & value_cache_bf16.write_block(&cache_block, 13, value_component_13);
                let component_14_written =
                    key_cache_bf16.write_block(&cache_block, 14, key_component_14)
                        & value_cache_bf16.write_block(&cache_block, 14, value_component_14);
                let component_15_written =
                    key_cache_bf16.write_block(&cache_block, 15, key_component_15)
                        & value_cache_bf16.write_block(&cache_block, 15, value_component_15);
                if !(component_0_written
                    & component_1_written
                    & component_2_written
                    & component_3_written
                    & component_4_written
                    & component_5_written
                    & component_6_written
                    & component_7_written
                    & component_8_written
                    & component_9_written
                    & component_10_written
                    & component_11_written
                    & component_12_written
                    & component_13_written
                    & component_14_written
                    & component_15_written)
                {
                    fe2o3_device::trap();
                }
            }
            if token_in_page == 1 {
                let component_0_written =
                    key_cache_bf16.write_block(&cache_block, 16, key_component_0)
                        & value_cache_bf16.write_block(&cache_block, 16, value_component_0);
                let component_1_written =
                    key_cache_bf16.write_block(&cache_block, 17, key_component_1)
                        & value_cache_bf16.write_block(&cache_block, 17, value_component_1);
                let component_2_written =
                    key_cache_bf16.write_block(&cache_block, 18, key_component_2)
                        & value_cache_bf16.write_block(&cache_block, 18, value_component_2);
                let component_3_written =
                    key_cache_bf16.write_block(&cache_block, 19, key_component_3)
                        & value_cache_bf16.write_block(&cache_block, 19, value_component_3);
                let component_4_written =
                    key_cache_bf16.write_block(&cache_block, 20, key_component_4)
                        & value_cache_bf16.write_block(&cache_block, 20, value_component_4);
                let component_5_written =
                    key_cache_bf16.write_block(&cache_block, 21, key_component_5)
                        & value_cache_bf16.write_block(&cache_block, 21, value_component_5);
                let component_6_written =
                    key_cache_bf16.write_block(&cache_block, 22, key_component_6)
                        & value_cache_bf16.write_block(&cache_block, 22, value_component_6);
                let component_7_written =
                    key_cache_bf16.write_block(&cache_block, 23, key_component_7)
                        & value_cache_bf16.write_block(&cache_block, 23, value_component_7);
                let component_8_written =
                    key_cache_bf16.write_block(&cache_block, 24, key_component_8)
                        & value_cache_bf16.write_block(&cache_block, 24, value_component_8);
                let component_9_written =
                    key_cache_bf16.write_block(&cache_block, 25, key_component_9)
                        & value_cache_bf16.write_block(&cache_block, 25, value_component_9);
                let component_10_written =
                    key_cache_bf16.write_block(&cache_block, 26, key_component_10)
                        & value_cache_bf16.write_block(&cache_block, 26, value_component_10);
                let component_11_written =
                    key_cache_bf16.write_block(&cache_block, 27, key_component_11)
                        & value_cache_bf16.write_block(&cache_block, 27, value_component_11);
                let component_12_written =
                    key_cache_bf16.write_block(&cache_block, 28, key_component_12)
                        & value_cache_bf16.write_block(&cache_block, 28, value_component_12);
                let component_13_written =
                    key_cache_bf16.write_block(&cache_block, 29, key_component_13)
                        & value_cache_bf16.write_block(&cache_block, 29, value_component_13);
                let component_14_written =
                    key_cache_bf16.write_block(&cache_block, 30, key_component_14)
                        & value_cache_bf16.write_block(&cache_block, 30, value_component_14);
                let component_15_written =
                    key_cache_bf16.write_block(&cache_block, 31, key_component_15)
                        & value_cache_bf16.write_block(&cache_block, 31, value_component_15);
                if !(component_0_written
                    & component_1_written
                    & component_2_written
                    & component_3_written
                    & component_4_written
                    & component_5_written
                    & component_6_written
                    & component_7_written
                    & component_8_written
                    & component_9_written
                    & component_10_written
                    & component_11_written
                    & component_12_written
                    & component_13_written
                    & component_14_written
                    & component_15_written)
                {
                    fe2o3_device::trap();
                }
            }
            if token_in_page == 2 {
                let component_0_written =
                    key_cache_bf16.write_block(&cache_block, 32, key_component_0)
                        & value_cache_bf16.write_block(&cache_block, 32, value_component_0);
                let component_1_written =
                    key_cache_bf16.write_block(&cache_block, 33, key_component_1)
                        & value_cache_bf16.write_block(&cache_block, 33, value_component_1);
                let component_2_written =
                    key_cache_bf16.write_block(&cache_block, 34, key_component_2)
                        & value_cache_bf16.write_block(&cache_block, 34, value_component_2);
                let component_3_written =
                    key_cache_bf16.write_block(&cache_block, 35, key_component_3)
                        & value_cache_bf16.write_block(&cache_block, 35, value_component_3);
                let component_4_written =
                    key_cache_bf16.write_block(&cache_block, 36, key_component_4)
                        & value_cache_bf16.write_block(&cache_block, 36, value_component_4);
                let component_5_written =
                    key_cache_bf16.write_block(&cache_block, 37, key_component_5)
                        & value_cache_bf16.write_block(&cache_block, 37, value_component_5);
                let component_6_written =
                    key_cache_bf16.write_block(&cache_block, 38, key_component_6)
                        & value_cache_bf16.write_block(&cache_block, 38, value_component_6);
                let component_7_written =
                    key_cache_bf16.write_block(&cache_block, 39, key_component_7)
                        & value_cache_bf16.write_block(&cache_block, 39, value_component_7);
                let component_8_written =
                    key_cache_bf16.write_block(&cache_block, 40, key_component_8)
                        & value_cache_bf16.write_block(&cache_block, 40, value_component_8);
                let component_9_written =
                    key_cache_bf16.write_block(&cache_block, 41, key_component_9)
                        & value_cache_bf16.write_block(&cache_block, 41, value_component_9);
                let component_10_written =
                    key_cache_bf16.write_block(&cache_block, 42, key_component_10)
                        & value_cache_bf16.write_block(&cache_block, 42, value_component_10);
                let component_11_written =
                    key_cache_bf16.write_block(&cache_block, 43, key_component_11)
                        & value_cache_bf16.write_block(&cache_block, 43, value_component_11);
                let component_12_written =
                    key_cache_bf16.write_block(&cache_block, 44, key_component_12)
                        & value_cache_bf16.write_block(&cache_block, 44, value_component_12);
                let component_13_written =
                    key_cache_bf16.write_block(&cache_block, 45, key_component_13)
                        & value_cache_bf16.write_block(&cache_block, 45, value_component_13);
                let component_14_written =
                    key_cache_bf16.write_block(&cache_block, 46, key_component_14)
                        & value_cache_bf16.write_block(&cache_block, 46, value_component_14);
                let component_15_written =
                    key_cache_bf16.write_block(&cache_block, 47, key_component_15)
                        & value_cache_bf16.write_block(&cache_block, 47, value_component_15);
                if !(component_0_written
                    & component_1_written
                    & component_2_written
                    & component_3_written
                    & component_4_written
                    & component_5_written
                    & component_6_written
                    & component_7_written
                    & component_8_written
                    & component_9_written
                    & component_10_written
                    & component_11_written
                    & component_12_written
                    & component_13_written
                    & component_14_written
                    & component_15_written)
                {
                    fe2o3_device::trap();
                }
            }
            if token_in_page == 3 {
                let component_0_written =
                    key_cache_bf16.write_block(&cache_block, 48, key_component_0)
                        & value_cache_bf16.write_block(&cache_block, 48, value_component_0);
                let component_1_written =
                    key_cache_bf16.write_block(&cache_block, 49, key_component_1)
                        & value_cache_bf16.write_block(&cache_block, 49, value_component_1);
                let component_2_written =
                    key_cache_bf16.write_block(&cache_block, 50, key_component_2)
                        & value_cache_bf16.write_block(&cache_block, 50, value_component_2);
                let component_3_written =
                    key_cache_bf16.write_block(&cache_block, 51, key_component_3)
                        & value_cache_bf16.write_block(&cache_block, 51, value_component_3);
                let component_4_written =
                    key_cache_bf16.write_block(&cache_block, 52, key_component_4)
                        & value_cache_bf16.write_block(&cache_block, 52, value_component_4);
                let component_5_written =
                    key_cache_bf16.write_block(&cache_block, 53, key_component_5)
                        & value_cache_bf16.write_block(&cache_block, 53, value_component_5);
                let component_6_written =
                    key_cache_bf16.write_block(&cache_block, 54, key_component_6)
                        & value_cache_bf16.write_block(&cache_block, 54, value_component_6);
                let component_7_written =
                    key_cache_bf16.write_block(&cache_block, 55, key_component_7)
                        & value_cache_bf16.write_block(&cache_block, 55, value_component_7);
                let component_8_written =
                    key_cache_bf16.write_block(&cache_block, 56, key_component_8)
                        & value_cache_bf16.write_block(&cache_block, 56, value_component_8);
                let component_9_written =
                    key_cache_bf16.write_block(&cache_block, 57, key_component_9)
                        & value_cache_bf16.write_block(&cache_block, 57, value_component_9);
                let component_10_written =
                    key_cache_bf16.write_block(&cache_block, 58, key_component_10)
                        & value_cache_bf16.write_block(&cache_block, 58, value_component_10);
                let component_11_written =
                    key_cache_bf16.write_block(&cache_block, 59, key_component_11)
                        & value_cache_bf16.write_block(&cache_block, 59, value_component_11);
                let component_12_written =
                    key_cache_bf16.write_block(&cache_block, 60, key_component_12)
                        & value_cache_bf16.write_block(&cache_block, 60, value_component_12);
                let component_13_written =
                    key_cache_bf16.write_block(&cache_block, 61, key_component_13)
                        & value_cache_bf16.write_block(&cache_block, 61, value_component_13);
                let component_14_written =
                    key_cache_bf16.write_block(&cache_block, 62, key_component_14)
                        & value_cache_bf16.write_block(&cache_block, 62, value_component_14);
                let component_15_written =
                    key_cache_bf16.write_block(&cache_block, 63, key_component_15)
                        & value_cache_bf16.write_block(&cache_block, 63, value_component_15);
                if !(component_0_written
                    & component_1_written
                    & component_2_written
                    & component_3_written
                    & component_4_written
                    & component_5_written
                    & component_6_written
                    & component_7_written
                    & component_8_written
                    & component_9_written
                    & component_10_written
                    & component_11_written
                    & component_12_written
                    & component_13_written
                    & component_14_written
                    & component_15_written)
                {
                    fe2o3_device::trap();
                }
            }
            if token_in_page == 4 {
                let component_0_written =
                    key_cache_bf16.write_block(&cache_block, 64, key_component_0)
                        & value_cache_bf16.write_block(&cache_block, 64, value_component_0);
                let component_1_written =
                    key_cache_bf16.write_block(&cache_block, 65, key_component_1)
                        & value_cache_bf16.write_block(&cache_block, 65, value_component_1);
                let component_2_written =
                    key_cache_bf16.write_block(&cache_block, 66, key_component_2)
                        & value_cache_bf16.write_block(&cache_block, 66, value_component_2);
                let component_3_written =
                    key_cache_bf16.write_block(&cache_block, 67, key_component_3)
                        & value_cache_bf16.write_block(&cache_block, 67, value_component_3);
                let component_4_written =
                    key_cache_bf16.write_block(&cache_block, 68, key_component_4)
                        & value_cache_bf16.write_block(&cache_block, 68, value_component_4);
                let component_5_written =
                    key_cache_bf16.write_block(&cache_block, 69, key_component_5)
                        & value_cache_bf16.write_block(&cache_block, 69, value_component_5);
                let component_6_written =
                    key_cache_bf16.write_block(&cache_block, 70, key_component_6)
                        & value_cache_bf16.write_block(&cache_block, 70, value_component_6);
                let component_7_written =
                    key_cache_bf16.write_block(&cache_block, 71, key_component_7)
                        & value_cache_bf16.write_block(&cache_block, 71, value_component_7);
                let component_8_written =
                    key_cache_bf16.write_block(&cache_block, 72, key_component_8)
                        & value_cache_bf16.write_block(&cache_block, 72, value_component_8);
                let component_9_written =
                    key_cache_bf16.write_block(&cache_block, 73, key_component_9)
                        & value_cache_bf16.write_block(&cache_block, 73, value_component_9);
                let component_10_written =
                    key_cache_bf16.write_block(&cache_block, 74, key_component_10)
                        & value_cache_bf16.write_block(&cache_block, 74, value_component_10);
                let component_11_written =
                    key_cache_bf16.write_block(&cache_block, 75, key_component_11)
                        & value_cache_bf16.write_block(&cache_block, 75, value_component_11);
                let component_12_written =
                    key_cache_bf16.write_block(&cache_block, 76, key_component_12)
                        & value_cache_bf16.write_block(&cache_block, 76, value_component_12);
                let component_13_written =
                    key_cache_bf16.write_block(&cache_block, 77, key_component_13)
                        & value_cache_bf16.write_block(&cache_block, 77, value_component_13);
                let component_14_written =
                    key_cache_bf16.write_block(&cache_block, 78, key_component_14)
                        & value_cache_bf16.write_block(&cache_block, 78, value_component_14);
                let component_15_written =
                    key_cache_bf16.write_block(&cache_block, 79, key_component_15)
                        & value_cache_bf16.write_block(&cache_block, 79, value_component_15);
                if !(component_0_written
                    & component_1_written
                    & component_2_written
                    & component_3_written
                    & component_4_written
                    & component_5_written
                    & component_6_written
                    & component_7_written
                    & component_8_written
                    & component_9_written
                    & component_10_written
                    & component_11_written
                    & component_12_written
                    & component_13_written
                    & component_14_written
                    & component_15_written)
                {
                    fe2o3_device::trap();
                }
            }
            if token_in_page == 5 {
                let component_0_written =
                    key_cache_bf16.write_block(&cache_block, 80, key_component_0)
                        & value_cache_bf16.write_block(&cache_block, 80, value_component_0);
                let component_1_written =
                    key_cache_bf16.write_block(&cache_block, 81, key_component_1)
                        & value_cache_bf16.write_block(&cache_block, 81, value_component_1);
                let component_2_written =
                    key_cache_bf16.write_block(&cache_block, 82, key_component_2)
                        & value_cache_bf16.write_block(&cache_block, 82, value_component_2);
                let component_3_written =
                    key_cache_bf16.write_block(&cache_block, 83, key_component_3)
                        & value_cache_bf16.write_block(&cache_block, 83, value_component_3);
                let component_4_written =
                    key_cache_bf16.write_block(&cache_block, 84, key_component_4)
                        & value_cache_bf16.write_block(&cache_block, 84, value_component_4);
                let component_5_written =
                    key_cache_bf16.write_block(&cache_block, 85, key_component_5)
                        & value_cache_bf16.write_block(&cache_block, 85, value_component_5);
                let component_6_written =
                    key_cache_bf16.write_block(&cache_block, 86, key_component_6)
                        & value_cache_bf16.write_block(&cache_block, 86, value_component_6);
                let component_7_written =
                    key_cache_bf16.write_block(&cache_block, 87, key_component_7)
                        & value_cache_bf16.write_block(&cache_block, 87, value_component_7);
                let component_8_written =
                    key_cache_bf16.write_block(&cache_block, 88, key_component_8)
                        & value_cache_bf16.write_block(&cache_block, 88, value_component_8);
                let component_9_written =
                    key_cache_bf16.write_block(&cache_block, 89, key_component_9)
                        & value_cache_bf16.write_block(&cache_block, 89, value_component_9);
                let component_10_written =
                    key_cache_bf16.write_block(&cache_block, 90, key_component_10)
                        & value_cache_bf16.write_block(&cache_block, 90, value_component_10);
                let component_11_written =
                    key_cache_bf16.write_block(&cache_block, 91, key_component_11)
                        & value_cache_bf16.write_block(&cache_block, 91, value_component_11);
                let component_12_written =
                    key_cache_bf16.write_block(&cache_block, 92, key_component_12)
                        & value_cache_bf16.write_block(&cache_block, 92, value_component_12);
                let component_13_written =
                    key_cache_bf16.write_block(&cache_block, 93, key_component_13)
                        & value_cache_bf16.write_block(&cache_block, 93, value_component_13);
                let component_14_written =
                    key_cache_bf16.write_block(&cache_block, 94, key_component_14)
                        & value_cache_bf16.write_block(&cache_block, 94, value_component_14);
                let component_15_written =
                    key_cache_bf16.write_block(&cache_block, 95, key_component_15)
                        & value_cache_bf16.write_block(&cache_block, 95, value_component_15);
                if !(component_0_written
                    & component_1_written
                    & component_2_written
                    & component_3_written
                    & component_4_written
                    & component_5_written
                    & component_6_written
                    & component_7_written
                    & component_8_written
                    & component_9_written
                    & component_10_written
                    & component_11_written
                    & component_12_written
                    & component_13_written
                    & component_14_written
                    & component_15_written)
                {
                    fe2o3_device::trap();
                }
            }
            if token_in_page == 6 {
                let component_0_written =
                    key_cache_bf16.write_block(&cache_block, 96, key_component_0)
                        & value_cache_bf16.write_block(&cache_block, 96, value_component_0);
                let component_1_written =
                    key_cache_bf16.write_block(&cache_block, 97, key_component_1)
                        & value_cache_bf16.write_block(&cache_block, 97, value_component_1);
                let component_2_written =
                    key_cache_bf16.write_block(&cache_block, 98, key_component_2)
                        & value_cache_bf16.write_block(&cache_block, 98, value_component_2);
                let component_3_written =
                    key_cache_bf16.write_block(&cache_block, 99, key_component_3)
                        & value_cache_bf16.write_block(&cache_block, 99, value_component_3);
                let component_4_written =
                    key_cache_bf16.write_block(&cache_block, 100, key_component_4)
                        & value_cache_bf16.write_block(&cache_block, 100, value_component_4);
                let component_5_written =
                    key_cache_bf16.write_block(&cache_block, 101, key_component_5)
                        & value_cache_bf16.write_block(&cache_block, 101, value_component_5);
                let component_6_written =
                    key_cache_bf16.write_block(&cache_block, 102, key_component_6)
                        & value_cache_bf16.write_block(&cache_block, 102, value_component_6);
                let component_7_written =
                    key_cache_bf16.write_block(&cache_block, 103, key_component_7)
                        & value_cache_bf16.write_block(&cache_block, 103, value_component_7);
                let component_8_written =
                    key_cache_bf16.write_block(&cache_block, 104, key_component_8)
                        & value_cache_bf16.write_block(&cache_block, 104, value_component_8);
                let component_9_written =
                    key_cache_bf16.write_block(&cache_block, 105, key_component_9)
                        & value_cache_bf16.write_block(&cache_block, 105, value_component_9);
                let component_10_written =
                    key_cache_bf16.write_block(&cache_block, 106, key_component_10)
                        & value_cache_bf16.write_block(&cache_block, 106, value_component_10);
                let component_11_written =
                    key_cache_bf16.write_block(&cache_block, 107, key_component_11)
                        & value_cache_bf16.write_block(&cache_block, 107, value_component_11);
                let component_12_written =
                    key_cache_bf16.write_block(&cache_block, 108, key_component_12)
                        & value_cache_bf16.write_block(&cache_block, 108, value_component_12);
                let component_13_written =
                    key_cache_bf16.write_block(&cache_block, 109, key_component_13)
                        & value_cache_bf16.write_block(&cache_block, 109, value_component_13);
                let component_14_written =
                    key_cache_bf16.write_block(&cache_block, 110, key_component_14)
                        & value_cache_bf16.write_block(&cache_block, 110, value_component_14);
                let component_15_written =
                    key_cache_bf16.write_block(&cache_block, 111, key_component_15)
                        & value_cache_bf16.write_block(&cache_block, 111, value_component_15);
                if !(component_0_written
                    & component_1_written
                    & component_2_written
                    & component_3_written
                    & component_4_written
                    & component_5_written
                    & component_6_written
                    & component_7_written
                    & component_8_written
                    & component_9_written
                    & component_10_written
                    & component_11_written
                    & component_12_written
                    & component_13_written
                    & component_14_written
                    & component_15_written)
                {
                    fe2o3_device::trap();
                }
            }
            if token_in_page == 7 {
                let component_0_written =
                    key_cache_bf16.write_block(&cache_block, 112, key_component_0)
                        & value_cache_bf16.write_block(&cache_block, 112, value_component_0);
                let component_1_written =
                    key_cache_bf16.write_block(&cache_block, 113, key_component_1)
                        & value_cache_bf16.write_block(&cache_block, 113, value_component_1);
                let component_2_written =
                    key_cache_bf16.write_block(&cache_block, 114, key_component_2)
                        & value_cache_bf16.write_block(&cache_block, 114, value_component_2);
                let component_3_written =
                    key_cache_bf16.write_block(&cache_block, 115, key_component_3)
                        & value_cache_bf16.write_block(&cache_block, 115, value_component_3);
                let component_4_written =
                    key_cache_bf16.write_block(&cache_block, 116, key_component_4)
                        & value_cache_bf16.write_block(&cache_block, 116, value_component_4);
                let component_5_written =
                    key_cache_bf16.write_block(&cache_block, 117, key_component_5)
                        & value_cache_bf16.write_block(&cache_block, 117, value_component_5);
                let component_6_written =
                    key_cache_bf16.write_block(&cache_block, 118, key_component_6)
                        & value_cache_bf16.write_block(&cache_block, 118, value_component_6);
                let component_7_written =
                    key_cache_bf16.write_block(&cache_block, 119, key_component_7)
                        & value_cache_bf16.write_block(&cache_block, 119, value_component_7);
                let component_8_written =
                    key_cache_bf16.write_block(&cache_block, 120, key_component_8)
                        & value_cache_bf16.write_block(&cache_block, 120, value_component_8);
                let component_9_written =
                    key_cache_bf16.write_block(&cache_block, 121, key_component_9)
                        & value_cache_bf16.write_block(&cache_block, 121, value_component_9);
                let component_10_written =
                    key_cache_bf16.write_block(&cache_block, 122, key_component_10)
                        & value_cache_bf16.write_block(&cache_block, 122, value_component_10);
                let component_11_written =
                    key_cache_bf16.write_block(&cache_block, 123, key_component_11)
                        & value_cache_bf16.write_block(&cache_block, 123, value_component_11);
                let component_12_written =
                    key_cache_bf16.write_block(&cache_block, 124, key_component_12)
                        & value_cache_bf16.write_block(&cache_block, 124, value_component_12);
                let component_13_written =
                    key_cache_bf16.write_block(&cache_block, 125, key_component_13)
                        & value_cache_bf16.write_block(&cache_block, 125, value_component_13);
                let component_14_written =
                    key_cache_bf16.write_block(&cache_block, 126, key_component_14)
                        & value_cache_bf16.write_block(&cache_block, 126, value_component_14);
                let component_15_written =
                    key_cache_bf16.write_block(&cache_block, 127, key_component_15)
                        & value_cache_bf16.write_block(&cache_block, 127, value_component_15);
                if !(component_0_written
                    & component_1_written
                    & component_2_written
                    & component_3_written
                    & component_4_written
                    & component_5_written
                    & component_6_written
                    & component_7_written
                    & component_8_written
                    & component_9_written
                    & component_10_written
                    & component_11_written
                    & component_12_written
                    & component_13_written
                    & component_14_written
                    & component_15_written)
                {
                    fe2o3_device::trap();
                }
            }
            if token_in_page == 8 {
                let component_0_written =
                    key_cache_bf16.write_block(&cache_block, 128, key_component_0)
                        & value_cache_bf16.write_block(&cache_block, 128, value_component_0);
                let component_1_written =
                    key_cache_bf16.write_block(&cache_block, 129, key_component_1)
                        & value_cache_bf16.write_block(&cache_block, 129, value_component_1);
                let component_2_written =
                    key_cache_bf16.write_block(&cache_block, 130, key_component_2)
                        & value_cache_bf16.write_block(&cache_block, 130, value_component_2);
                let component_3_written =
                    key_cache_bf16.write_block(&cache_block, 131, key_component_3)
                        & value_cache_bf16.write_block(&cache_block, 131, value_component_3);
                let component_4_written =
                    key_cache_bf16.write_block(&cache_block, 132, key_component_4)
                        & value_cache_bf16.write_block(&cache_block, 132, value_component_4);
                let component_5_written =
                    key_cache_bf16.write_block(&cache_block, 133, key_component_5)
                        & value_cache_bf16.write_block(&cache_block, 133, value_component_5);
                let component_6_written =
                    key_cache_bf16.write_block(&cache_block, 134, key_component_6)
                        & value_cache_bf16.write_block(&cache_block, 134, value_component_6);
                let component_7_written =
                    key_cache_bf16.write_block(&cache_block, 135, key_component_7)
                        & value_cache_bf16.write_block(&cache_block, 135, value_component_7);
                let component_8_written =
                    key_cache_bf16.write_block(&cache_block, 136, key_component_8)
                        & value_cache_bf16.write_block(&cache_block, 136, value_component_8);
                let component_9_written =
                    key_cache_bf16.write_block(&cache_block, 137, key_component_9)
                        & value_cache_bf16.write_block(&cache_block, 137, value_component_9);
                let component_10_written =
                    key_cache_bf16.write_block(&cache_block, 138, key_component_10)
                        & value_cache_bf16.write_block(&cache_block, 138, value_component_10);
                let component_11_written =
                    key_cache_bf16.write_block(&cache_block, 139, key_component_11)
                        & value_cache_bf16.write_block(&cache_block, 139, value_component_11);
                let component_12_written =
                    key_cache_bf16.write_block(&cache_block, 140, key_component_12)
                        & value_cache_bf16.write_block(&cache_block, 140, value_component_12);
                let component_13_written =
                    key_cache_bf16.write_block(&cache_block, 141, key_component_13)
                        & value_cache_bf16.write_block(&cache_block, 141, value_component_13);
                let component_14_written =
                    key_cache_bf16.write_block(&cache_block, 142, key_component_14)
                        & value_cache_bf16.write_block(&cache_block, 142, value_component_14);
                let component_15_written =
                    key_cache_bf16.write_block(&cache_block, 143, key_component_15)
                        & value_cache_bf16.write_block(&cache_block, 143, value_component_15);
                if !(component_0_written
                    & component_1_written
                    & component_2_written
                    & component_3_written
                    & component_4_written
                    & component_5_written
                    & component_6_written
                    & component_7_written
                    & component_8_written
                    & component_9_written
                    & component_10_written
                    & component_11_written
                    & component_12_written
                    & component_13_written
                    & component_14_written
                    & component_15_written)
                {
                    fe2o3_device::trap();
                }
            }
            if token_in_page == 9 {
                let component_0_written =
                    key_cache_bf16.write_block(&cache_block, 144, key_component_0)
                        & value_cache_bf16.write_block(&cache_block, 144, value_component_0);
                let component_1_written =
                    key_cache_bf16.write_block(&cache_block, 145, key_component_1)
                        & value_cache_bf16.write_block(&cache_block, 145, value_component_1);
                let component_2_written =
                    key_cache_bf16.write_block(&cache_block, 146, key_component_2)
                        & value_cache_bf16.write_block(&cache_block, 146, value_component_2);
                let component_3_written =
                    key_cache_bf16.write_block(&cache_block, 147, key_component_3)
                        & value_cache_bf16.write_block(&cache_block, 147, value_component_3);
                let component_4_written =
                    key_cache_bf16.write_block(&cache_block, 148, key_component_4)
                        & value_cache_bf16.write_block(&cache_block, 148, value_component_4);
                let component_5_written =
                    key_cache_bf16.write_block(&cache_block, 149, key_component_5)
                        & value_cache_bf16.write_block(&cache_block, 149, value_component_5);
                let component_6_written =
                    key_cache_bf16.write_block(&cache_block, 150, key_component_6)
                        & value_cache_bf16.write_block(&cache_block, 150, value_component_6);
                let component_7_written =
                    key_cache_bf16.write_block(&cache_block, 151, key_component_7)
                        & value_cache_bf16.write_block(&cache_block, 151, value_component_7);
                let component_8_written =
                    key_cache_bf16.write_block(&cache_block, 152, key_component_8)
                        & value_cache_bf16.write_block(&cache_block, 152, value_component_8);
                let component_9_written =
                    key_cache_bf16.write_block(&cache_block, 153, key_component_9)
                        & value_cache_bf16.write_block(&cache_block, 153, value_component_9);
                let component_10_written =
                    key_cache_bf16.write_block(&cache_block, 154, key_component_10)
                        & value_cache_bf16.write_block(&cache_block, 154, value_component_10);
                let component_11_written =
                    key_cache_bf16.write_block(&cache_block, 155, key_component_11)
                        & value_cache_bf16.write_block(&cache_block, 155, value_component_11);
                let component_12_written =
                    key_cache_bf16.write_block(&cache_block, 156, key_component_12)
                        & value_cache_bf16.write_block(&cache_block, 156, value_component_12);
                let component_13_written =
                    key_cache_bf16.write_block(&cache_block, 157, key_component_13)
                        & value_cache_bf16.write_block(&cache_block, 157, value_component_13);
                let component_14_written =
                    key_cache_bf16.write_block(&cache_block, 158, key_component_14)
                        & value_cache_bf16.write_block(&cache_block, 158, value_component_14);
                let component_15_written =
                    key_cache_bf16.write_block(&cache_block, 159, key_component_15)
                        & value_cache_bf16.write_block(&cache_block, 159, value_component_15);
                if !(component_0_written
                    & component_1_written
                    & component_2_written
                    & component_3_written
                    & component_4_written
                    & component_5_written
                    & component_6_written
                    & component_7_written
                    & component_8_written
                    & component_9_written
                    & component_10_written
                    & component_11_written
                    & component_12_written
                    & component_13_written
                    & component_14_written
                    & component_15_written)
                {
                    fe2o3_device::trap();
                }
            }
            if token_in_page == 10 {
                let component_0_written =
                    key_cache_bf16.write_block(&cache_block, 160, key_component_0)
                        & value_cache_bf16.write_block(&cache_block, 160, value_component_0);
                let component_1_written =
                    key_cache_bf16.write_block(&cache_block, 161, key_component_1)
                        & value_cache_bf16.write_block(&cache_block, 161, value_component_1);
                let component_2_written =
                    key_cache_bf16.write_block(&cache_block, 162, key_component_2)
                        & value_cache_bf16.write_block(&cache_block, 162, value_component_2);
                let component_3_written =
                    key_cache_bf16.write_block(&cache_block, 163, key_component_3)
                        & value_cache_bf16.write_block(&cache_block, 163, value_component_3);
                let component_4_written =
                    key_cache_bf16.write_block(&cache_block, 164, key_component_4)
                        & value_cache_bf16.write_block(&cache_block, 164, value_component_4);
                let component_5_written =
                    key_cache_bf16.write_block(&cache_block, 165, key_component_5)
                        & value_cache_bf16.write_block(&cache_block, 165, value_component_5);
                let component_6_written =
                    key_cache_bf16.write_block(&cache_block, 166, key_component_6)
                        & value_cache_bf16.write_block(&cache_block, 166, value_component_6);
                let component_7_written =
                    key_cache_bf16.write_block(&cache_block, 167, key_component_7)
                        & value_cache_bf16.write_block(&cache_block, 167, value_component_7);
                let component_8_written =
                    key_cache_bf16.write_block(&cache_block, 168, key_component_8)
                        & value_cache_bf16.write_block(&cache_block, 168, value_component_8);
                let component_9_written =
                    key_cache_bf16.write_block(&cache_block, 169, key_component_9)
                        & value_cache_bf16.write_block(&cache_block, 169, value_component_9);
                let component_10_written =
                    key_cache_bf16.write_block(&cache_block, 170, key_component_10)
                        & value_cache_bf16.write_block(&cache_block, 170, value_component_10);
                let component_11_written =
                    key_cache_bf16.write_block(&cache_block, 171, key_component_11)
                        & value_cache_bf16.write_block(&cache_block, 171, value_component_11);
                let component_12_written =
                    key_cache_bf16.write_block(&cache_block, 172, key_component_12)
                        & value_cache_bf16.write_block(&cache_block, 172, value_component_12);
                let component_13_written =
                    key_cache_bf16.write_block(&cache_block, 173, key_component_13)
                        & value_cache_bf16.write_block(&cache_block, 173, value_component_13);
                let component_14_written =
                    key_cache_bf16.write_block(&cache_block, 174, key_component_14)
                        & value_cache_bf16.write_block(&cache_block, 174, value_component_14);
                let component_15_written =
                    key_cache_bf16.write_block(&cache_block, 175, key_component_15)
                        & value_cache_bf16.write_block(&cache_block, 175, value_component_15);
                if !(component_0_written
                    & component_1_written
                    & component_2_written
                    & component_3_written
                    & component_4_written
                    & component_5_written
                    & component_6_written
                    & component_7_written
                    & component_8_written
                    & component_9_written
                    & component_10_written
                    & component_11_written
                    & component_12_written
                    & component_13_written
                    & component_14_written
                    & component_15_written)
                {
                    fe2o3_device::trap();
                }
            }
            if token_in_page == 11 {
                let component_0_written =
                    key_cache_bf16.write_block(&cache_block, 176, key_component_0)
                        & value_cache_bf16.write_block(&cache_block, 176, value_component_0);
                let component_1_written =
                    key_cache_bf16.write_block(&cache_block, 177, key_component_1)
                        & value_cache_bf16.write_block(&cache_block, 177, value_component_1);
                let component_2_written =
                    key_cache_bf16.write_block(&cache_block, 178, key_component_2)
                        & value_cache_bf16.write_block(&cache_block, 178, value_component_2);
                let component_3_written =
                    key_cache_bf16.write_block(&cache_block, 179, key_component_3)
                        & value_cache_bf16.write_block(&cache_block, 179, value_component_3);
                let component_4_written =
                    key_cache_bf16.write_block(&cache_block, 180, key_component_4)
                        & value_cache_bf16.write_block(&cache_block, 180, value_component_4);
                let component_5_written =
                    key_cache_bf16.write_block(&cache_block, 181, key_component_5)
                        & value_cache_bf16.write_block(&cache_block, 181, value_component_5);
                let component_6_written =
                    key_cache_bf16.write_block(&cache_block, 182, key_component_6)
                        & value_cache_bf16.write_block(&cache_block, 182, value_component_6);
                let component_7_written =
                    key_cache_bf16.write_block(&cache_block, 183, key_component_7)
                        & value_cache_bf16.write_block(&cache_block, 183, value_component_7);
                let component_8_written =
                    key_cache_bf16.write_block(&cache_block, 184, key_component_8)
                        & value_cache_bf16.write_block(&cache_block, 184, value_component_8);
                let component_9_written =
                    key_cache_bf16.write_block(&cache_block, 185, key_component_9)
                        & value_cache_bf16.write_block(&cache_block, 185, value_component_9);
                let component_10_written =
                    key_cache_bf16.write_block(&cache_block, 186, key_component_10)
                        & value_cache_bf16.write_block(&cache_block, 186, value_component_10);
                let component_11_written =
                    key_cache_bf16.write_block(&cache_block, 187, key_component_11)
                        & value_cache_bf16.write_block(&cache_block, 187, value_component_11);
                let component_12_written =
                    key_cache_bf16.write_block(&cache_block, 188, key_component_12)
                        & value_cache_bf16.write_block(&cache_block, 188, value_component_12);
                let component_13_written =
                    key_cache_bf16.write_block(&cache_block, 189, key_component_13)
                        & value_cache_bf16.write_block(&cache_block, 189, value_component_13);
                let component_14_written =
                    key_cache_bf16.write_block(&cache_block, 190, key_component_14)
                        & value_cache_bf16.write_block(&cache_block, 190, value_component_14);
                let component_15_written =
                    key_cache_bf16.write_block(&cache_block, 191, key_component_15)
                        & value_cache_bf16.write_block(&cache_block, 191, value_component_15);
                if !(component_0_written
                    & component_1_written
                    & component_2_written
                    & component_3_written
                    & component_4_written
                    & component_5_written
                    & component_6_written
                    & component_7_written
                    & component_8_written
                    & component_9_written
                    & component_10_written
                    & component_11_written
                    & component_12_written
                    & component_13_written
                    & component_14_written
                    & component_15_written)
                {
                    fe2o3_device::trap();
                }
            }
            if token_in_page == 12 {
                let component_0_written =
                    key_cache_bf16.write_block(&cache_block, 192, key_component_0)
                        & value_cache_bf16.write_block(&cache_block, 192, value_component_0);
                let component_1_written =
                    key_cache_bf16.write_block(&cache_block, 193, key_component_1)
                        & value_cache_bf16.write_block(&cache_block, 193, value_component_1);
                let component_2_written =
                    key_cache_bf16.write_block(&cache_block, 194, key_component_2)
                        & value_cache_bf16.write_block(&cache_block, 194, value_component_2);
                let component_3_written =
                    key_cache_bf16.write_block(&cache_block, 195, key_component_3)
                        & value_cache_bf16.write_block(&cache_block, 195, value_component_3);
                let component_4_written =
                    key_cache_bf16.write_block(&cache_block, 196, key_component_4)
                        & value_cache_bf16.write_block(&cache_block, 196, value_component_4);
                let component_5_written =
                    key_cache_bf16.write_block(&cache_block, 197, key_component_5)
                        & value_cache_bf16.write_block(&cache_block, 197, value_component_5);
                let component_6_written =
                    key_cache_bf16.write_block(&cache_block, 198, key_component_6)
                        & value_cache_bf16.write_block(&cache_block, 198, value_component_6);
                let component_7_written =
                    key_cache_bf16.write_block(&cache_block, 199, key_component_7)
                        & value_cache_bf16.write_block(&cache_block, 199, value_component_7);
                let component_8_written =
                    key_cache_bf16.write_block(&cache_block, 200, key_component_8)
                        & value_cache_bf16.write_block(&cache_block, 200, value_component_8);
                let component_9_written =
                    key_cache_bf16.write_block(&cache_block, 201, key_component_9)
                        & value_cache_bf16.write_block(&cache_block, 201, value_component_9);
                let component_10_written =
                    key_cache_bf16.write_block(&cache_block, 202, key_component_10)
                        & value_cache_bf16.write_block(&cache_block, 202, value_component_10);
                let component_11_written =
                    key_cache_bf16.write_block(&cache_block, 203, key_component_11)
                        & value_cache_bf16.write_block(&cache_block, 203, value_component_11);
                let component_12_written =
                    key_cache_bf16.write_block(&cache_block, 204, key_component_12)
                        & value_cache_bf16.write_block(&cache_block, 204, value_component_12);
                let component_13_written =
                    key_cache_bf16.write_block(&cache_block, 205, key_component_13)
                        & value_cache_bf16.write_block(&cache_block, 205, value_component_13);
                let component_14_written =
                    key_cache_bf16.write_block(&cache_block, 206, key_component_14)
                        & value_cache_bf16.write_block(&cache_block, 206, value_component_14);
                let component_15_written =
                    key_cache_bf16.write_block(&cache_block, 207, key_component_15)
                        & value_cache_bf16.write_block(&cache_block, 207, value_component_15);
                if !(component_0_written
                    & component_1_written
                    & component_2_written
                    & component_3_written
                    & component_4_written
                    & component_5_written
                    & component_6_written
                    & component_7_written
                    & component_8_written
                    & component_9_written
                    & component_10_written
                    & component_11_written
                    & component_12_written
                    & component_13_written
                    & component_14_written
                    & component_15_written)
                {
                    fe2o3_device::trap();
                }
            }
            if token_in_page == 13 {
                let component_0_written =
                    key_cache_bf16.write_block(&cache_block, 208, key_component_0)
                        & value_cache_bf16.write_block(&cache_block, 208, value_component_0);
                let component_1_written =
                    key_cache_bf16.write_block(&cache_block, 209, key_component_1)
                        & value_cache_bf16.write_block(&cache_block, 209, value_component_1);
                let component_2_written =
                    key_cache_bf16.write_block(&cache_block, 210, key_component_2)
                        & value_cache_bf16.write_block(&cache_block, 210, value_component_2);
                let component_3_written =
                    key_cache_bf16.write_block(&cache_block, 211, key_component_3)
                        & value_cache_bf16.write_block(&cache_block, 211, value_component_3);
                let component_4_written =
                    key_cache_bf16.write_block(&cache_block, 212, key_component_4)
                        & value_cache_bf16.write_block(&cache_block, 212, value_component_4);
                let component_5_written =
                    key_cache_bf16.write_block(&cache_block, 213, key_component_5)
                        & value_cache_bf16.write_block(&cache_block, 213, value_component_5);
                let component_6_written =
                    key_cache_bf16.write_block(&cache_block, 214, key_component_6)
                        & value_cache_bf16.write_block(&cache_block, 214, value_component_6);
                let component_7_written =
                    key_cache_bf16.write_block(&cache_block, 215, key_component_7)
                        & value_cache_bf16.write_block(&cache_block, 215, value_component_7);
                let component_8_written =
                    key_cache_bf16.write_block(&cache_block, 216, key_component_8)
                        & value_cache_bf16.write_block(&cache_block, 216, value_component_8);
                let component_9_written =
                    key_cache_bf16.write_block(&cache_block, 217, key_component_9)
                        & value_cache_bf16.write_block(&cache_block, 217, value_component_9);
                let component_10_written =
                    key_cache_bf16.write_block(&cache_block, 218, key_component_10)
                        & value_cache_bf16.write_block(&cache_block, 218, value_component_10);
                let component_11_written =
                    key_cache_bf16.write_block(&cache_block, 219, key_component_11)
                        & value_cache_bf16.write_block(&cache_block, 219, value_component_11);
                let component_12_written =
                    key_cache_bf16.write_block(&cache_block, 220, key_component_12)
                        & value_cache_bf16.write_block(&cache_block, 220, value_component_12);
                let component_13_written =
                    key_cache_bf16.write_block(&cache_block, 221, key_component_13)
                        & value_cache_bf16.write_block(&cache_block, 221, value_component_13);
                let component_14_written =
                    key_cache_bf16.write_block(&cache_block, 222, key_component_14)
                        & value_cache_bf16.write_block(&cache_block, 222, value_component_14);
                let component_15_written =
                    key_cache_bf16.write_block(&cache_block, 223, key_component_15)
                        & value_cache_bf16.write_block(&cache_block, 223, value_component_15);
                if !(component_0_written
                    & component_1_written
                    & component_2_written
                    & component_3_written
                    & component_4_written
                    & component_5_written
                    & component_6_written
                    & component_7_written
                    & component_8_written
                    & component_9_written
                    & component_10_written
                    & component_11_written
                    & component_12_written
                    & component_13_written
                    & component_14_written
                    & component_15_written)
                {
                    fe2o3_device::trap();
                }
            }
            if token_in_page == 14 {
                let component_0_written =
                    key_cache_bf16.write_block(&cache_block, 224, key_component_0)
                        & value_cache_bf16.write_block(&cache_block, 224, value_component_0);
                let component_1_written =
                    key_cache_bf16.write_block(&cache_block, 225, key_component_1)
                        & value_cache_bf16.write_block(&cache_block, 225, value_component_1);
                let component_2_written =
                    key_cache_bf16.write_block(&cache_block, 226, key_component_2)
                        & value_cache_bf16.write_block(&cache_block, 226, value_component_2);
                let component_3_written =
                    key_cache_bf16.write_block(&cache_block, 227, key_component_3)
                        & value_cache_bf16.write_block(&cache_block, 227, value_component_3);
                let component_4_written =
                    key_cache_bf16.write_block(&cache_block, 228, key_component_4)
                        & value_cache_bf16.write_block(&cache_block, 228, value_component_4);
                let component_5_written =
                    key_cache_bf16.write_block(&cache_block, 229, key_component_5)
                        & value_cache_bf16.write_block(&cache_block, 229, value_component_5);
                let component_6_written =
                    key_cache_bf16.write_block(&cache_block, 230, key_component_6)
                        & value_cache_bf16.write_block(&cache_block, 230, value_component_6);
                let component_7_written =
                    key_cache_bf16.write_block(&cache_block, 231, key_component_7)
                        & value_cache_bf16.write_block(&cache_block, 231, value_component_7);
                let component_8_written =
                    key_cache_bf16.write_block(&cache_block, 232, key_component_8)
                        & value_cache_bf16.write_block(&cache_block, 232, value_component_8);
                let component_9_written =
                    key_cache_bf16.write_block(&cache_block, 233, key_component_9)
                        & value_cache_bf16.write_block(&cache_block, 233, value_component_9);
                let component_10_written =
                    key_cache_bf16.write_block(&cache_block, 234, key_component_10)
                        & value_cache_bf16.write_block(&cache_block, 234, value_component_10);
                let component_11_written =
                    key_cache_bf16.write_block(&cache_block, 235, key_component_11)
                        & value_cache_bf16.write_block(&cache_block, 235, value_component_11);
                let component_12_written =
                    key_cache_bf16.write_block(&cache_block, 236, key_component_12)
                        & value_cache_bf16.write_block(&cache_block, 236, value_component_12);
                let component_13_written =
                    key_cache_bf16.write_block(&cache_block, 237, key_component_13)
                        & value_cache_bf16.write_block(&cache_block, 237, value_component_13);
                let component_14_written =
                    key_cache_bf16.write_block(&cache_block, 238, key_component_14)
                        & value_cache_bf16.write_block(&cache_block, 238, value_component_14);
                let component_15_written =
                    key_cache_bf16.write_block(&cache_block, 239, key_component_15)
                        & value_cache_bf16.write_block(&cache_block, 239, value_component_15);
                if !(component_0_written
                    & component_1_written
                    & component_2_written
                    & component_3_written
                    & component_4_written
                    & component_5_written
                    & component_6_written
                    & component_7_written
                    & component_8_written
                    & component_9_written
                    & component_10_written
                    & component_11_written
                    & component_12_written
                    & component_13_written
                    & component_14_written
                    & component_15_written)
                {
                    fe2o3_device::trap();
                }
            }
            if token_in_page == 15 {
                let component_0_written =
                    key_cache_bf16.write_block(&cache_block, 240, key_component_0)
                        & value_cache_bf16.write_block(&cache_block, 240, value_component_0);
                let component_1_written =
                    key_cache_bf16.write_block(&cache_block, 241, key_component_1)
                        & value_cache_bf16.write_block(&cache_block, 241, value_component_1);
                let component_2_written =
                    key_cache_bf16.write_block(&cache_block, 242, key_component_2)
                        & value_cache_bf16.write_block(&cache_block, 242, value_component_2);
                let component_3_written =
                    key_cache_bf16.write_block(&cache_block, 243, key_component_3)
                        & value_cache_bf16.write_block(&cache_block, 243, value_component_3);
                let component_4_written =
                    key_cache_bf16.write_block(&cache_block, 244, key_component_4)
                        & value_cache_bf16.write_block(&cache_block, 244, value_component_4);
                let component_5_written =
                    key_cache_bf16.write_block(&cache_block, 245, key_component_5)
                        & value_cache_bf16.write_block(&cache_block, 245, value_component_5);
                let component_6_written =
                    key_cache_bf16.write_block(&cache_block, 246, key_component_6)
                        & value_cache_bf16.write_block(&cache_block, 246, value_component_6);
                let component_7_written =
                    key_cache_bf16.write_block(&cache_block, 247, key_component_7)
                        & value_cache_bf16.write_block(&cache_block, 247, value_component_7);
                let component_8_written =
                    key_cache_bf16.write_block(&cache_block, 248, key_component_8)
                        & value_cache_bf16.write_block(&cache_block, 248, value_component_8);
                let component_9_written =
                    key_cache_bf16.write_block(&cache_block, 249, key_component_9)
                        & value_cache_bf16.write_block(&cache_block, 249, value_component_9);
                let component_10_written =
                    key_cache_bf16.write_block(&cache_block, 250, key_component_10)
                        & value_cache_bf16.write_block(&cache_block, 250, value_component_10);
                let component_11_written =
                    key_cache_bf16.write_block(&cache_block, 251, key_component_11)
                        & value_cache_bf16.write_block(&cache_block, 251, value_component_11);
                let component_12_written =
                    key_cache_bf16.write_block(&cache_block, 252, key_component_12)
                        & value_cache_bf16.write_block(&cache_block, 252, value_component_12);
                let component_13_written =
                    key_cache_bf16.write_block(&cache_block, 253, key_component_13)
                        & value_cache_bf16.write_block(&cache_block, 253, value_component_13);
                let component_14_written =
                    key_cache_bf16.write_block(&cache_block, 254, key_component_14)
                        & value_cache_bf16.write_block(&cache_block, 254, value_component_14);
                let component_15_written =
                    key_cache_bf16.write_block(&cache_block, 255, key_component_15)
                        & value_cache_bf16.write_block(&cache_block, 255, value_component_15);
                if !(component_0_written
                    & component_1_written
                    & component_2_written
                    & component_3_written
                    & component_4_written
                    & component_5_written
                    & component_6_written
                    & component_7_written
                    & component_8_written
                    & component_9_written
                    & component_10_written
                    & component_11_written
                    & component_12_written
                    & component_13_written
                    & component_14_written
                    & component_15_written)
                {
                    fe2o3_device::trap();
                }
            }
        }
        row += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifiers_accept_exact_boundary_profiles() {
        assert!(qwen3_rope_profile_is_admitted_v1(128, 8, 32, 128));
        assert!(qwen3_rope_profile_is_admitted_v1(2_048, 1, 16, 2_048));
        assert!(qwen3_rope_profile_is_admitted_v1(17, 1, 32, 8_192));
        assert!(qwen3_rope_profile_is_admitted_v1(16, 1, 16, 8_192));
        assert!(qwen3_paged_kv_write_profile_is_admitted_v1(5, 8, 8_192));
        assert!(qwen3_paged_kv_write_profile_is_admitted_v1(4, 8, 8_192));
    }

    #[test]
    fn classifiers_reject_cross_role_and_unknown_profiles() {
        assert!(!qwen3_rope_profile_is_admitted_v1(17, 1, 16, 8_192));
        assert!(!qwen3_rope_profile_is_admitted_v1(16, 1, 32, 8_192));
        assert!(!qwen3_rope_profile_is_admitted_v1(128, 8, 8, 128));
        assert!(!qwen3_paged_kv_write_profile_is_admitted_v1(17, 8, 8_192));
        assert!(!qwen3_paged_kv_write_profile_is_admitted_v1(
            2_048, 1, 8_192
        ));
    }

    #[test]
    fn launch_and_cache_maxima_are_exact() {
        assert_eq!(QWEN3_ROPE_MAX_GRID_WORKGROUPS_V1, 2_048);
        assert_eq!(QWEN3_PAGED_KV_WRITE_GRID_WORKGROUPS_V1, 16_384);
        assert_eq!(QWEN3_PAGED_KV_WRITE_GRID_WORKITEMS_V1, 1_048_576);
        assert_eq!(QWEN3_KV_CACHE_ELEMENTS_V1, 268_435_456);
        assert_eq!(
            (QWEN3_KV_PHYSICAL_PAGE_SLOTS_V1 as usize - 1)
                * QWEN3_KV_PAGE_TOKENS_V1 as usize
                * 1_024
                + (QWEN3_KV_PAGE_TOKENS_V1 as usize - 1) * 1_024
                + 1_023,
            QWEN3_KV_CACHE_ELEMENTS_V1 - 1
        );
    }
}
