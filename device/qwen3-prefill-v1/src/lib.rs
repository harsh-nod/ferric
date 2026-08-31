#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)] // The kernel macro emits an undocumented helper module.

//! Attributed Rust source for Ferric's exact Qwen3 paged-GQA prefill kernel.
//!
//! This source contract carries no artifact, dispatch, numerical-qualification,
//! or M1 authority. Production integration remains fail-closed until an exact
//! compiler run emits and verifies a replacement artifact.

use fe2o3_device::{Bf16, Blocked, Index1D, Math, WriteOnlyDisjointSlice, kernel, memory, thread};

/// Exact exported kernel symbol retained from the direct-LLVM implementation.
pub const QWEN3_PREFILL_KERNEL_SYMBOL_V1: &str = "qwen3_gqa_prefill_causal_bf16_f32_v1";
/// Exact Wave64 workgroup size in workitems.
pub const QWEN3_PREFILL_WORKGROUP_V1: [u32; 3] = [64, 1, 1];
/// Largest admitted one-dimensional grid in workgroups.
pub const QWEN3_PREFILL_MAX_GRID_WORKGROUPS_V1: u32 = 65_536;
/// Exact attention head dimension.
pub const QWEN3_PREFILL_HEAD_DIMENSION_V1: usize = 128;
/// Exact shared KV-head count.
pub const QWEN3_PREFILL_KV_HEADS_V1: usize = 8;
/// Exact tokens in one physical cache page.
pub const QWEN3_PREFILL_PAGE_TOKENS_V1: usize = 16;
/// Exact logical-page entries reserved for one sequence.
pub const QWEN3_PREFILL_PAGE_TABLE_ENTRIES_V1: usize = 512;
/// Exact global physical-page pool size.
pub const QWEN3_PREFILL_CACHE_POOL_PAGES_V1: usize = 16_384;
/// Exact BF16 elements in each global K/V cache allocation.
pub const QWEN3_PREFILL_CACHE_ELEMENTS_V1: usize = 268_435_456;
/// Exact FP32 bits for `1 / sqrt(128)`.
pub const QWEN3_PREFILL_ATTENTION_SCALE_BITS_V1: u32 = 0x3db5_04f3;
/// Exact explicit kernarg bytes for five pointer-plus-`usize` slice records.
pub const QWEN3_PREFILL_EXPLICIT_KERNARG_BYTES_V1: usize = 80;
/// Number of closed target/draft role-and-bucket profiles.
pub const QWEN3_PREFILL_PROFILE_COUNT_V1: usize = 8;

/// One exact role-and-bucket profile inferred from the retained slice ABI.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Qwen3PrefillProfileV1 {
    pub query_elements: usize,
    pub page_table_elements: usize,
    pub sequences: usize,
    pub tokens: usize,
    pub query_heads: usize,
    pub gqa_group_size: usize,
}

/// Closed Ferric target/draft B3 profile catalog.
pub const QWEN3_PREFILL_PROFILES_V1: [Qwen3PrefillProfileV1; QWEN3_PREFILL_PROFILE_COUNT_V1] = [
    Qwen3PrefillProfileV1 {
        query_elements: 524_288,
        page_table_elements: 512,
        sequences: 1,
        tokens: 128,
        query_heads: 32,
        gqa_group_size: 4,
    },
    Qwen3PrefillProfileV1 {
        query_elements: 4_194_304,
        page_table_elements: 4_096,
        sequences: 8,
        tokens: 128,
        query_heads: 32,
        gqa_group_size: 4,
    },
    Qwen3PrefillProfileV1 {
        query_elements: 2_097_152,
        page_table_elements: 512,
        sequences: 1,
        tokens: 512,
        query_heads: 32,
        gqa_group_size: 4,
    },
    Qwen3PrefillProfileV1 {
        query_elements: 8_388_608,
        page_table_elements: 512,
        sequences: 1,
        tokens: 2_048,
        query_heads: 32,
        gqa_group_size: 4,
    },
    Qwen3PrefillProfileV1 {
        query_elements: 262_144,
        page_table_elements: 512,
        sequences: 1,
        tokens: 128,
        query_heads: 16,
        gqa_group_size: 2,
    },
    Qwen3PrefillProfileV1 {
        query_elements: 2_097_152,
        page_table_elements: 4_096,
        sequences: 8,
        tokens: 128,
        query_heads: 16,
        gqa_group_size: 2,
    },
    Qwen3PrefillProfileV1 {
        query_elements: 1_048_576,
        page_table_elements: 512,
        sequences: 1,
        tokens: 512,
        query_heads: 16,
        gqa_group_size: 2,
    },
    Qwen3PrefillProfileV1 {
        query_elements: 4_194_304,
        page_table_elements: 512,
        sequences: 1,
        tokens: 2_048,
        query_heads: 16,
        gqa_group_size: 2,
    },
];

/// Infers one exact profile from the two ABI lengths that distinguish all cases.
#[must_use]
pub const fn qwen3_prefill_profile_for_lengths_v1(
    query_elements: usize,
    page_table_elements: usize,
) -> Option<Qwen3PrefillProfileV1> {
    let mut index = 0;
    while index < QWEN3_PREFILL_PROFILES_V1.len() {
        let profile = QWEN3_PREFILL_PROFILES_V1[index];
        if profile.query_elements == query_elements
            && profile.page_table_elements == page_table_elements
        {
            return Some(profile);
        }
        index += 1;
    }
    None
}

/// Computes exact paged-GQA causal prefill for one output-vector pair per lane.
///
/// Q and output have logical shape `[S,T,QH,128]`; K and V are the shared
/// global `[16384,16,8,128]` P16 cache, and pages has shape `[S,512]`. All
/// immutable loads are bounded volatile reads. Each workitem owns two adjacent
/// output features and traps before either owned store on an exceptional path.
#[kernel(
    typed,
    launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [65536, 1, 1]),
    control_flow(loop_bounds(2048, 128))
)]
pub fn qwen3_gqa_prefill_causal_bf16_f32_v1(
    q: &[u16],
    k: &[u16],
    v: &[u16],
    pages: &[u32],
    mut output: WriteOnlyDisjointSlice<u16, Blocked<Index1D, 1, 2>>,
) {
    let case_target_s1t128 = q.len() == 524_288 && pages.len() == 512;
    let case_target_s8t128 = q.len() == 4_194_304 && pages.len() == 4_096;
    let case_target_s1t512 = q.len() == 2_097_152 && pages.len() == 512;
    let case_target_s1t2048 = q.len() == 8_388_608 && pages.len() == 512;
    let case_draft_s1t128 = q.len() == 262_144 && pages.len() == 512;
    let case_draft_s8t128 = q.len() == 2_097_152 && pages.len() == 4_096;
    let case_draft_s1t512 = q.len() == 1_048_576 && pages.len() == 512;
    let case_draft_s1t2048 = q.len() == 4_194_304 && pages.len() == 512;

    let target =
        case_target_s1t128 || case_target_s8t128 || case_target_s1t512 || case_target_s1t2048;
    let draft = case_draft_s1t128 || case_draft_s8t128 || case_draft_s1t512 || case_draft_s1t2048;
    if !(target || draft)
        || k.len() != QWEN3_PREFILL_CACHE_ELEMENTS_V1
        || v.len() != QWEN3_PREFILL_CACHE_ELEMENTS_V1
        || output.len() != q.len()
    {
        fe2o3_device::trap();
    }

    let sequences = if case_target_s8t128 || case_draft_s8t128 {
        8
    } else {
        1
    };
    let query_heads = if target { 32 } else { 16 };
    let gqa_group_size = if target { 4 } else { 2 };
    let tokens =
        if case_target_s1t128 || case_target_s8t128 || case_draft_s1t128 || case_draft_s8t128 {
            128
        } else if case_target_s1t512 || case_draft_s1t512 {
            512
        } else {
            2_048
        };

    let workitem = thread::index_1d();
    let global = workitem.get();
    if global >= q.len() / 2 {
        fe2o3_device::trap();
    }
    let Some(output_pair) = workitem.checked_block::<1, 2>() else {
        fe2o3_device::trap();
    };

    let vector = global / 64;
    let local = global % 64;
    let query_head = vector % query_heads;
    let position = vector / query_heads;
    let query_token = position % tokens;
    let sequence = position / tokens;
    let kv_head = query_head / gqa_group_size;
    if sequence >= sequences || kv_head >= QWEN3_PREFILL_KV_HEADS_V1 {
        fe2o3_device::trap();
    }

    let query_base = vector * QWEN3_PREFILL_HEAD_DIMENSION_V1;
    let column_0 = local * 2;
    let column_1 = column_0 + 1;
    let scale = f32::from_bits(QWEN3_PREFILL_ATTENTION_SCALE_BITS_V1);
    let math = Math::current();
    let mut key_token = 0;
    let mut running_max = 0.0_f32;
    let mut running_sum = 0.0_f32;
    let mut numerator_0 = 0.0_f32;
    let mut numerator_1 = 0.0_f32;

    while key_token <= query_token {
        let logical_page = key_token / QWEN3_PREFILL_PAGE_TOKENS_V1;
        let token_in_page = key_token % QWEN3_PREFILL_PAGE_TOKENS_V1;
        let page_table_index = sequence * QWEN3_PREFILL_PAGE_TABLE_ENTRIES_V1 + logical_page;
        if logical_page >= QWEN3_PREFILL_PAGE_TABLE_ENTRIES_V1 || page_table_index >= pages.len() {
            fe2o3_device::trap();
        }
        let physical_page = memory::volatile_load(pages, page_table_index) as usize;
        if physical_page >= QWEN3_PREFILL_CACHE_POOL_PAGES_V1 {
            fe2o3_device::trap();
        }

        let cache_base = ((physical_page * QWEN3_PREFILL_PAGE_TOKENS_V1 + token_in_page)
            * QWEN3_PREFILL_KV_HEADS_V1
            + kv_head)
            * QWEN3_PREFILL_HEAD_DIMENSION_V1;
        if cache_base + QWEN3_PREFILL_HEAD_DIMENSION_V1 > k.len() {
            fe2o3_device::trap();
        }

        let mut feature = 0;
        let mut dot = 0.0_f32;
        while feature < QWEN3_PREFILL_HEAD_DIMENSION_V1 {
            let query_value = Bf16::from_bits(memory::volatile_load(q, query_base + feature));
            let key_value = Bf16::from_bits(memory::volatile_load(k, cache_base + feature));
            if !query_value.is_finite() || !key_value.is_finite() {
                fe2o3_device::trap();
            }
            let product = query_value.to_f32() * key_value.to_f32();
            let next_dot = dot + product;
            if !(product >= f32::MIN && product <= f32::MAX)
                || !(next_dot >= f32::MIN && next_dot <= f32::MAX)
            {
                fe2o3_device::trap();
            }
            dot = next_dot;
            feature += 1;
        }

        let score = dot * scale;
        if !(score >= f32::MIN && score <= f32::MAX) {
            fe2o3_device::trap();
        }
        let value_0 = Bf16::from_bits(memory::volatile_load(v, cache_base + column_0));
        let value_1 = Bf16::from_bits(memory::volatile_load(v, cache_base + column_1));
        if !value_0.is_finite() || !value_1.is_finite() {
            fe2o3_device::trap();
        }
        let value_0 = value_0.to_f32();
        let value_1 = value_1.to_f32();

        if key_token == 0 {
            running_max = score;
            running_sum = 1.0;
            numerator_0 = value_0;
            numerator_1 = value_1;
        } else {
            let next_max = if score > running_max {
                score
            } else {
                running_max
            };
            let previous_weight = math.exp_f32(running_max - next_max);
            let current_weight = math.exp_f32(score - next_max);
            let next_sum = running_sum * previous_weight + current_weight;
            let next_numerator_0 = numerator_0 * previous_weight + value_0 * current_weight;
            let next_numerator_1 = numerator_1 * previous_weight + value_1 * current_weight;
            if !(previous_weight >= f32::MIN && previous_weight <= f32::MAX)
                || !(current_weight >= f32::MIN && current_weight <= f32::MAX)
                || !(next_sum >= f32::MIN && next_sum <= f32::MAX)
                || next_sum <= 0.0
                || !(next_numerator_0 >= f32::MIN && next_numerator_0 <= f32::MAX)
                || !(next_numerator_1 >= f32::MIN && next_numerator_1 <= f32::MAX)
            {
                fe2o3_device::trap();
            }
            running_max = next_max;
            running_sum = next_sum;
            numerator_0 = next_numerator_0;
            numerator_1 = next_numerator_1;
        }
        key_token += 1;
    }

    let output_0 = numerator_0 / running_sum;
    let output_1 = numerator_1 / running_sum;
    if !(output_0 >= f32::MIN && output_0 <= f32::MAX)
        || !(output_1 >= f32::MIN && output_1 <= f32::MAX)
    {
        fe2o3_device::trap();
    }
    let output_0 = Bf16::from_f32(output_0);
    let output_1 = Bf16::from_f32(output_1);
    if !output_0.is_finite() || !output_1.is_finite() {
        fe2o3_device::trap();
    }

    if !output.write_block(&output_pair, 0, output_0.to_bits()) {
        fe2o3_device::trap();
    }
    if !output.write_block(&output_pair, 1, output_1.to_bits()) {
        fe2o3_device::trap();
    }
}
