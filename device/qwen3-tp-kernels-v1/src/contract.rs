//! Engineering ABI facts, not protected artifact or dispatch authority.

pub const KERNEL_SYMBOLS: [&str; 13] = [
    "ferric_qwen3_compact_completion_v1",
    "ferric_qwen3_gemm_reference_bf16_f32_bf16_v1",
    "ferric_qwen3_gemm_vector_a4_bf16_f32_bf16_v1",
    "ferric_qwen3_lowest_id_argmax_bf16_v1",
    "ferric_qwen3_speculative_token_assembly_v1",
    "ferric_qwen3_token_embedding_bf16_copy_v1",
    "ferric_qwen3_tp_gemv_bf16_f32_bf16_v1",
    "ferric_qwen3_tp_gemv_partial_bf16_f32_v1",
    "ferric_qwen3_tp_gqa_decode_bf16_f32_v1",
    "ferric_qwen3_tp_kv_append_v1",
    "ferric_qwen3_tp_rope_v1",
    "ferric_qwen3_tp_swiglu_bf16_f32_v1",
    "qwen3_rmsnorm_v1",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TpGeometry {
    pub hidden: u32,
    pub query_heads: u32,
    pub kv_heads: u32,
    pub intermediate: u32,
}

pub const fn geometry(model_role: u32, world_size: u32) -> Option<TpGeometry> {
    if !tp_role_world_valid_v1!(model_role, world_size) {
        return None;
    }
    let (hidden, query_heads, kv_heads, intermediate) = tp_sizes_v1!(model_role, world_size);
    Some(TpGeometry {
        hidden,
        query_heads,
        kv_heads,
        intermediate,
    })
}

pub const fn column_shape(n: u32, k: u32, role: u32, world: u32, op: u32) -> bool {
    if !tp_role_world_valid_v1!(role, world) {
        return false;
    }
    tp_column_shape_v1!(n, k, role, world, op)
}

pub const fn partial_shape(n: u32, k: u32, role: u32, world: u32, op: u32) -> bool {
    if !tp_role_world_valid_v1!(role, world) {
        return false;
    }
    tp_partial_shape_v1!(n, k, role, world, op)
}

pub const fn append_position(position: u32, capacity: u32) -> bool {
    tp_capacity_valid_v1!(capacity) && position < capacity
}

pub const fn attention_prefix(count: u32, capacity: u32) -> bool {
    tp_capacity_valid_v1!(capacity) && count > 0 && count <= capacity
}

/// Scalar prefix sizes; COV6 alignment and hidden arguments come from metadata.
pub const GEMV_EXPLICIT_BYTES: u32 = 68;
pub const SWIGLU_EXPLICIT_BYTES: u32 = 56;
pub const ROPE_EXPLICIT_BYTES: u32 = 108;
pub const KV_APPEND_EXPLICIT_BYTES: u32 = 80;
pub const ATTENTION_EXPLICIT_BYTES: u32 = 80;
pub const WORKGROUP: [u32; 3] = [64, 1, 1];

#[cfg(test)]
mod tests;
