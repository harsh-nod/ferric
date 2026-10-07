//! Engineering ABI and geometry only; these constants confer no authority.

pub const MAX_ROWS: u32 = 16;
pub const MAX_REQUESTS: u32 = 32;
pub const PAGE_TOKENS: u32 = 16;
pub const MAX_PAGES: u32 = 512;
pub const MAX_CONTEXT_TOKENS: u32 = 8192;
pub const HIDDEN: u32 = 4096;
pub const INTERMEDIATE: u32 = 12288;
pub const VOCABULARY: u32 = 151936;
pub const HEAD_DIMENSION: u32 = 128;
pub const WORKGROUP: [u32; 3] = [64, 1, 1];
pub const EMBEDDING_EXPLICIT_BYTES: u32 = 52;
pub const GEMM_EXPLICIT_BYTES: u32 = 68;
pub const SWIGLU_EXPLICIT_BYTES: u32 = 56;
pub const ROPE_EXPLICIT_BYTES: u32 = 120;
pub const APPEND_EXPLICIT_BYTES: u32 = 112;
pub const ATTENTION_EXPLICIT_BYTES: u32 = 116;
pub const ARGMAX_EXPLICIT_BYTES: u32 = 36;

pub const NEW_ROOTS: [&str; 8] = [
    "ferric_qwen3_tp_batch_embedding_bf16_v2",
    "ferric_qwen3_tp_batch_gemm_bf16_f32_bf16_v2",
    "ferric_qwen3_tp_batch_gemm_partial_bf16_f32_v2",
    "ferric_qwen3_tp_batch_swiglu_bf16_f32_v2",
    "ferric_qwen3_tp_batch_rope_v2",
    "ferric_qwen3_tp_batch_paged_kv_append_v2",
    "ferric_qwen3_tp_batch_paged_gqa_bf16_f32_v2",
    "ferric_qwen3_tp_batch_argmax_bf16_v2",
];

pub const fn world_is_supported(world: u32) -> bool {
    world == 1 || world == 2 || world == 8
}

pub const fn rows_are_supported(rows: u32) -> bool {
    rows > 0 && rows <= MAX_ROWS
}

pub const fn local_query_heads(world: u32) -> Option<u32> {
    match world {
        1 => Some(32),
        2 => Some(16),
        8 => Some(4),
        _ => None,
    }
}

pub const fn local_kv_heads(world: u32) -> Option<u32> {
    match world {
        1 => Some(8),
        2 => Some(4),
        8 => Some(1),
        _ => None,
    }
}

pub const fn local_intermediate(world: u32) -> Option<u32> {
    match world {
        1 => Some(12288),
        2 => Some(6144),
        8 => Some(1536),
        _ => None,
    }
}
