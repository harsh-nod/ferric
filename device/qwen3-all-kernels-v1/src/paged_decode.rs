#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)] // The kernel macro emits an undocumented helper module.

//! Attributed Rust source for Ferric's exact Qwen3 paged-GQA decode kernel.
//!
//! This source contract carries no artifact, dispatch, numerical-qualification,
//! or M1 authority. Production integration remains fail-closed until an exact
//! compiler run emits and verifies a replacement artifact.

use fe2o3_device::{Bf16, Blocked, Index1D, Math, WriteOnlyDisjointSlice, kernel, memory, thread};

/// Exact exported kernel symbol retained from the direct-LLVM implementation.
pub const QWEN3_PAGED_DECODE_KERNEL_SYMBOL_V1: &str = "qwen3_paged_gqa_decode_bf16_f32_v1";
/// Exact Wave64 workgroup size in workitems.
pub const QWEN3_PAGED_DECODE_WORKGROUP_V1: [u32; 3] = [64, 1, 1];
/// Largest admitted one-dimensional grid in workgroups.
pub const QWEN3_PAGED_DECODE_MAX_GRID_WORKGROUPS_V1: u32 = 1_280;
/// Exact attention head dimension.
pub const QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1: usize = 128;
/// Exact shared KV-head count.
pub const QWEN3_PAGED_DECODE_KV_HEADS_V1: usize = 8;
/// Exact tokens in one physical cache page.
pub const QWEN3_PAGED_DECODE_PAGE_TOKENS_V1: usize = 16;
/// Exact logical-page entries reserved for one sequence.
pub const QWEN3_PAGED_DECODE_PAGE_TABLE_ENTRIES_V1: usize = 512;
/// Exact global physical-page pool size.
pub const QWEN3_PAGED_DECODE_CACHE_POOL_PAGES_V1: usize = 16_384;
/// Exact BF16 elements in each global K/V cache allocation.
pub const QWEN3_PAGED_DECODE_CACHE_ELEMENTS_V1: usize = 268_435_456;
/// Exact maximum committed-plus-active context length.
pub const QWEN3_PAGED_DECODE_CONTEXT_CAPACITY_V1: usize = 8_192;
/// Exact FP32 bits for `1 / sqrt(128)`.
pub const QWEN3_PAGED_DECODE_ATTENTION_SCALE_BITS_V1: u32 = 0x3db5_04f3;
/// Exact explicit kernarg bytes for six pointer-plus-`usize` slice records.
pub const QWEN3_PAGED_DECODE_EXPLICIT_KERNARG_BYTES_V1: usize = 96;
/// Number of closed target/draft role-and-bucket profiles.
pub const QWEN3_PAGED_DECODE_PROFILE_COUNT_V1: usize = 14;

/// One exact role-and-bucket profile inferred from the retained slice ABI.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Qwen3PagedDecodeProfileV1 {
    pub query_elements: usize,
    pub page_table_elements: usize,
    pub committed_elements: usize,
    pub sequences: usize,
    pub active_tokens: usize,
    pub query_heads: usize,
    pub gqa_group_size: usize,
}

/// Closed Ferric target/draft B3 profile catalog, role-major and bucket-major.
pub const QWEN3_PAGED_DECODE_PROFILES_V1: [Qwen3PagedDecodeProfileV1;
    QWEN3_PAGED_DECODE_PROFILE_COUNT_V1] = [
    Qwen3PagedDecodeProfileV1 {
        query_elements: 4_096,
        page_table_elements: 512,
        committed_elements: 1,
        sequences: 1,
        active_tokens: 1,
        query_heads: 32,
        gqa_group_size: 4,
    },
    Qwen3PagedDecodeProfileV1 {
        query_elements: 32_768,
        page_table_elements: 4_096,
        committed_elements: 8,
        sequences: 8,
        active_tokens: 1,
        query_heads: 32,
        gqa_group_size: 4,
    },
    Qwen3PagedDecodeProfileV1 {
        query_elements: 131_072,
        page_table_elements: 16_384,
        committed_elements: 32,
        sequences: 32,
        active_tokens: 1,
        query_heads: 32,
        gqa_group_size: 4,
    },
    Qwen3PagedDecodeProfileV1 {
        query_elements: 20_480,
        page_table_elements: 512,
        committed_elements: 1,
        sequences: 1,
        active_tokens: 5,
        query_heads: 32,
        gqa_group_size: 4,
    },
    Qwen3PagedDecodeProfileV1 {
        query_elements: 163_840,
        page_table_elements: 4_096,
        committed_elements: 8,
        sequences: 8,
        active_tokens: 5,
        query_heads: 32,
        gqa_group_size: 4,
    },
    Qwen3PagedDecodeProfileV1 {
        query_elements: 36_864,
        page_table_elements: 512,
        committed_elements: 1,
        sequences: 1,
        active_tokens: 9,
        query_heads: 32,
        gqa_group_size: 4,
    },
    Qwen3PagedDecodeProfileV1 {
        query_elements: 69_632,
        page_table_elements: 512,
        committed_elements: 1,
        sequences: 1,
        active_tokens: 17,
        query_heads: 32,
        gqa_group_size: 4,
    },
    Qwen3PagedDecodeProfileV1 {
        query_elements: 2_048,
        page_table_elements: 512,
        committed_elements: 1,
        sequences: 1,
        active_tokens: 1,
        query_heads: 16,
        gqa_group_size: 2,
    },
    Qwen3PagedDecodeProfileV1 {
        query_elements: 16_384,
        page_table_elements: 4_096,
        committed_elements: 8,
        sequences: 8,
        active_tokens: 1,
        query_heads: 16,
        gqa_group_size: 2,
    },
    Qwen3PagedDecodeProfileV1 {
        query_elements: 65_536,
        page_table_elements: 16_384,
        committed_elements: 32,
        sequences: 32,
        active_tokens: 1,
        query_heads: 16,
        gqa_group_size: 2,
    },
    Qwen3PagedDecodeProfileV1 {
        query_elements: 8_192,
        page_table_elements: 512,
        committed_elements: 1,
        sequences: 1,
        active_tokens: 4,
        query_heads: 16,
        gqa_group_size: 2,
    },
    Qwen3PagedDecodeProfileV1 {
        query_elements: 65_536,
        page_table_elements: 4_096,
        committed_elements: 8,
        sequences: 8,
        active_tokens: 4,
        query_heads: 16,
        gqa_group_size: 2,
    },
    Qwen3PagedDecodeProfileV1 {
        query_elements: 16_384,
        page_table_elements: 512,
        committed_elements: 1,
        sequences: 1,
        active_tokens: 8,
        query_heads: 16,
        gqa_group_size: 2,
    },
    Qwen3PagedDecodeProfileV1 {
        query_elements: 32_768,
        page_table_elements: 512,
        committed_elements: 1,
        sequences: 1,
        active_tokens: 16,
        query_heads: 16,
        gqa_group_size: 2,
    },
];

/// Infers one exact profile from the three ABI lengths that close all cases.
#[must_use]
pub const fn qwen3_paged_decode_profile_for_lengths_v1(
    query_elements: usize,
    page_table_elements: usize,
    committed_elements: usize,
) -> Option<Qwen3PagedDecodeProfileV1> {
    let mut index = 0;
    while index < QWEN3_PAGED_DECODE_PROFILES_V1.len() {
        let profile = QWEN3_PAGED_DECODE_PROFILES_V1[index];
        if profile.query_elements == query_elements
            && profile.page_table_elements == page_table_elements
            && profile.committed_elements == committed_elements
        {
            return Some(profile);
        }
        index += 1;
    }
    None
}

/// Computes exact paged-GQA causal decode for one output-vector pair per lane.
///
/// Q and output have logical shape `[S,A,QH,128]`; K and V are the shared
/// global `[16384,16,8,128]` P16 cache, pages has shape `[S,512]`, and
/// committed has shape `[S]`. All immutable loads are bounded volatile reads.
/// Each workitem owns two adjacent output features and traps before either
/// owned store on an exceptional path.
#[kernel(
    typed,
    launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [1280, 1, 1]),
    control_flow(loop_bounds(8192, 128))
)]
pub fn qwen3_paged_gqa_decode_bf16_f32_v1(
    q: &[u16],
    k: &[u16],
    v: &[u16],
    pages: &[u32],
    committed: &[u32],
    mut output: WriteOnlyDisjointSlice<u16, Blocked<Index1D, 1, 2>>,
) {
    let case_target_decode_s1 = q.len() == 4_096 && pages.len() == 512 && committed.len() == 1;
    let case_target_decode_s8 = q.len() == 32_768 && pages.len() == 4_096 && committed.len() == 8;
    let case_target_decode_s32 =
        q.len() == 131_072 && pages.len() == 16_384 && committed.len() == 32;
    let case_target_spec_s1k4 = q.len() == 20_480 && pages.len() == 512 && committed.len() == 1;
    let case_target_spec_s8k4 = q.len() == 163_840 && pages.len() == 4_096 && committed.len() == 8;
    let case_target_spec_s1k8 = q.len() == 36_864 && pages.len() == 512 && committed.len() == 1;
    let case_target_spec_s1k16 = q.len() == 69_632 && pages.len() == 512 && committed.len() == 1;
    let case_draft_decode_s1 = q.len() == 2_048 && pages.len() == 512 && committed.len() == 1;
    let case_draft_decode_s8 = q.len() == 16_384 && pages.len() == 4_096 && committed.len() == 8;
    let case_draft_decode_s32 = q.len() == 65_536 && pages.len() == 16_384 && committed.len() == 32;
    let case_draft_spec_s1k4 = q.len() == 8_192 && pages.len() == 512 && committed.len() == 1;
    let case_draft_spec_s8k4 = q.len() == 65_536 && pages.len() == 4_096 && committed.len() == 8;
    let case_draft_spec_s1k8 = q.len() == 16_384 && pages.len() == 512 && committed.len() == 1;
    let case_draft_spec_s1k16 = q.len() == 32_768 && pages.len() == 512 && committed.len() == 1;

    let target = case_target_decode_s1
        || case_target_decode_s8
        || case_target_decode_s32
        || case_target_spec_s1k4
        || case_target_spec_s8k4
        || case_target_spec_s1k8
        || case_target_spec_s1k16;
    let draft = case_draft_decode_s1
        || case_draft_decode_s8
        || case_draft_decode_s32
        || case_draft_spec_s1k4
        || case_draft_spec_s8k4
        || case_draft_spec_s1k8
        || case_draft_spec_s1k16;
    if !(target || draft)
        || k.len() != QWEN3_PAGED_DECODE_CACHE_ELEMENTS_V1
        || v.len() != QWEN3_PAGED_DECODE_CACHE_ELEMENTS_V1
        || output.len() != q.len()
    {
        fe2o3_device::trap();
    }

    let sequences = if case_target_decode_s32 || case_draft_decode_s32 {
        32
    } else if case_target_decode_s8
        || case_target_spec_s8k4
        || case_draft_decode_s8
        || case_draft_spec_s8k4
    {
        8
    } else {
        1
    };
    let query_heads = if target { 32 } else { 16 };
    let gqa_group_size = if target { 4 } else { 2 };
    let active_tokens = if case_target_decode_s1
        || case_target_decode_s8
        || case_target_decode_s32
        || case_draft_decode_s1
        || case_draft_decode_s8
        || case_draft_decode_s32
    {
        1
    } else if case_draft_spec_s1k4 || case_draft_spec_s8k4 {
        4
    } else if case_target_spec_s1k4 || case_target_spec_s8k4 {
        5
    } else if case_draft_spec_s1k8 {
        8
    } else if case_target_spec_s1k8 {
        9
    } else if case_draft_spec_s1k16 {
        16
    } else {
        17
    };
    if query_heads == 0 {
        fe2o3_device::trap();
    }
    if active_tokens == 0 {
        fe2o3_device::trap();
    }
    if gqa_group_size == 0 {
        fe2o3_device::trap();
    }

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
    let query_token = position % active_tokens;
    let sequence = position / active_tokens;
    let kv_head = query_head / gqa_group_size;
    if sequence >= sequences || kv_head >= QWEN3_PAGED_DECODE_KV_HEADS_V1 {
        fe2o3_device::trap();
    }

    let committed_tokens = memory::volatile_load(committed, sequence) as usize;
    if committed_tokens >= QWEN3_PAGED_DECODE_CONTEXT_CAPACITY_V1
        || active_tokens > QWEN3_PAGED_DECODE_CONTEXT_CAPACITY_V1 - committed_tokens
    {
        fe2o3_device::trap();
    }
    let query_position = committed_tokens + query_token;
    let query_base = vector * QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1;
    let column_0 = local * 2;
    let column_1 = column_0 + 1;
    let scale = f32::from_bits(QWEN3_PAGED_DECODE_ATTENTION_SCALE_BITS_V1);
    let math = Math::current();
    let mut key_token = 0;
    let mut running_max = 0.0_f32;
    let mut running_sum = 0.0_f32;
    let mut numerator_0 = 0.0_f32;
    let mut numerator_1 = 0.0_f32;

    while key_token <= query_position {
        let logical_page = key_token / QWEN3_PAGED_DECODE_PAGE_TOKENS_V1;
        let token_in_page = key_token % QWEN3_PAGED_DECODE_PAGE_TOKENS_V1;
        let page_table_index = sequence * QWEN3_PAGED_DECODE_PAGE_TABLE_ENTRIES_V1 + logical_page;
        if logical_page >= QWEN3_PAGED_DECODE_PAGE_TABLE_ENTRIES_V1
            || page_table_index >= pages.len()
        {
            fe2o3_device::trap();
        }
        let physical_page = memory::volatile_load(pages, page_table_index) as usize;
        if physical_page >= QWEN3_PAGED_DECODE_CACHE_POOL_PAGES_V1 {
            fe2o3_device::trap();
        }

        let cache_base = ((physical_page * QWEN3_PAGED_DECODE_PAGE_TOKENS_V1 + token_in_page)
            * QWEN3_PAGED_DECODE_KV_HEADS_V1
            + kv_head)
            * QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1;
        if cache_base + QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1 > k.len() {
            fe2o3_device::trap();
        }

        let mut feature = 0;
        let mut dot = 0.0_f32;
        while feature < QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1 {
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
