#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)] // The kernel macro emits undocumented helper modules.

//! Attributed Rust source for Ferric's exact Qwen3 K3 device roots.

use fe2o3_device::{
    Bf16, GridExclusive, Index1D, RowStriped2D, WriteOnlyDisjointSlice, kernel, thread,
};

pub const QWEN3_ROPE_KERNEL_SYMBOL_V1: &str = "qwen3_rope_v1";
pub const QWEN3_PAGED_KV_WRITE_KERNEL_SYMBOL_V1: &str = "qwen3_paged_kv_write_v1";
pub const QWEN3_ROPE_KV_WORKGROUP_V1: [u32; 3] = [64, 1, 1];
pub const QWEN3_ROPE_KV_MAX_GRID_WORKGROUPS_V1: u32 = 2_048;
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
    let position = position_ids[token_index] as usize;
    if position >= context_tokens || position >= QWEN3_ROPE_KV_MAX_CONTEXT_TOKENS_V1 as usize {
        fe2o3_device::trap();
    }
    let trig_index = position * 64 + lane;
    let cos = cos_table_f32[trig_index];
    let sin = sin_table_f32[trig_index];
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
        let first_value = Bf16::from_bits(query_bf16[first]).to_f32();
        let second_value = Bf16::from_bits(query_bf16[second]).to_f32();
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
        let first_value = Bf16::from_bits(key_bf16[first]).to_f32();
        let second_value = Bf16::from_bits(key_bf16[second]).to_f32();
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
    launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [2048, 1, 1]),
    control_flow(loop_bounds(2048, 1024))
)]
#[allow(clippy::too_many_arguments)]
pub fn qwen3_paged_kv_write_v1(
    rotated_key_bf16: &[u16],
    value_bf16: &[u16],
    logical_starts: &[u32],
    page_indices: &[u32],
    mut key_cache_bf16: WriteOnlyDisjointSlice<u16, GridExclusive>,
    mut value_cache_bf16: WriteOnlyDisjointSlice<u16, GridExclusive>,
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
        || thread::launch_extent_1d() != rows * 64
    {
        fe2o3_device::trap();
    }
    let Some(leader) = thread::grid_leader() else {
        return;
    };

    let mut row = 0_usize;
    while row < rows {
        let sequence = row / active_tokens;
        let local_token = row % active_tokens;
        let logical_start = logical_starts[sequence] as usize;
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
        let physical_page = page_indices[table_index] as usize;
        if physical_page >= QWEN3_KV_PHYSICAL_PAGE_SLOTS_V1 as usize {
            fe2o3_device::trap();
        }
        let cache_token = physical_page * QWEN3_KV_PAGE_TOKENS_V1 as usize + token_in_page;
        let input_base = row * kv_columns;
        let cache_base = cache_token * kv_columns;

        let mut component = 0_usize;
        while component < kv_columns {
            if !key_cache_bf16.write_exclusive(
                &leader,
                cache_base + component,
                rotated_key_bf16[input_base + component],
            ) || !value_cache_bf16.write_exclusive(
                &leader,
                cache_base + component,
                value_bf16[input_base + component],
            ) {
                fe2o3_device::trap();
            }
            component += 1;
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
        assert_eq!(QWEN3_ROPE_KV_MAX_GRID_WORKGROUPS_V1, 2_048);
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
