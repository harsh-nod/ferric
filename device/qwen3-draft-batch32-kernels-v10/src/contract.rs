//! Closed engineering ABI/shape metadata only, never model or completion authority.

pub const MAX_ROWS: u32 = 32;
pub const LAYERS: u32 = 28;
pub const HIDDEN: u32 = 1024;
pub const INTERMEDIATE: u32 = 3072;
pub const QUERY_HEADS: u32 = 16;
pub const KV_HEADS: u32 = 8;
pub const HEAD_DIMENSION: u32 = 128;
pub const VOCABULARY: u32 = 151936;
pub const PAGE_TOKENS: u32 = 16;
pub const MAX_PHYSICAL_PAGES: u32 = 512;
pub const MAX_CONTEXT_TOKENS: u32 = 8192;
pub const WORKGROUP: [u32; 3] = [64, 1, 1];
pub const KV_PAYLOAD_BYTES: u64 = 939_524_096;
pub const FP32_LOGITS_BYTES: u64 = 19_447_808;

pub const ROOTS_V10: [&str; 14] = [
    "ferric_qwen3_draft_batch32_rmsnorm_v10",
    "ferric_qwen3_draft_batch32_embedding_bf16_v10",
    "ferric_qwen3_draft_batch32_gemm_bf16_f32_bf16_v10",
    "ferric_qwen3_draft_batch32_mfma_gemm_bf16_v10",
    "ferric_qwen3_draft_batch32_gemm_partial_bf16_f32_v10",
    "ferric_qwen3_draft_batch32_mfma_gemm_partial_f32_v10",
    "ferric_qwen3_draft_batch32_swiglu_bf16_f32_v10",
    "ferric_qwen3_draft_batch32_rope_v10",
    "ferric_qwen3_draft_batch32_paged_kv_append_v10",
    "ferric_qwen3_draft_batch32_paged_gqa_bf16_f32_v10",
    "ferric_qwen3_draft_batch32_residual_bf16_v10",
    "ferric_qwen3_draft_batch32_head_bf16_f32_v10",
    "ferric_qwen3_draft_batch32_mfma_head_f32_v10",
    "ferric_qwen3_draft_batch32_argmax_f32_v10",
];
pub const EXPLICIT_ARGUMENT_BYTES: [u32; 14] =
    [96, 52, 68, 68, 68, 68, 56, 120, 112, 116, 52, 68, 68, 36];
pub const MAX_GRID_WORKGROUPS: [u32; 14] = [
    512, 512, 384, 384, 128, 128, 1536, 32, 1, 512, 512, 18992, 18992, 32,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectionRole {
    Query,
    Key,
    Value,
    Gate,
    Up,
    AttentionOutput,
    Down,
    Head,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectionMath {
    Scalar,
    Mfma,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputCarrier {
    Bf16,
    F32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProjectionShape {
    pub n: u32,
    pub k: u32,
    pub tag: u32,
    pub output: OutputCarrier,
}

impl ProjectionRole {
    pub const fn shape(self) -> ProjectionShape {
        let (n, k, tag, output) = match self {
            Self::Query => (2048, 1024, 1, OutputCarrier::Bf16),
            Self::Key => (1024, 1024, 2, OutputCarrier::Bf16),
            Self::Value => (1024, 1024, 3, OutputCarrier::Bf16),
            Self::Gate => (3072, 1024, 4, OutputCarrier::Bf16),
            Self::Up => (3072, 1024, 5, OutputCarrier::Bf16),
            Self::AttentionOutput => (1024, 2048, 1, OutputCarrier::F32),
            Self::Down => (1024, 3072, 2, OutputCarrier::F32),
            Self::Head => (151936, 1024, 6, OutputCarrier::F32),
        };
        ProjectionShape { n, k, tag, output }
    }

    pub const fn kernel(self, math: ProjectionMath) -> &'static str {
        match (self, math) {
            (Self::Head, ProjectionMath::Scalar) => ROOTS_V10[11],
            (Self::Head, ProjectionMath::Mfma) => ROOTS_V10[12],
            (Self::AttentionOutput | Self::Down, ProjectionMath::Scalar) => ROOTS_V10[4],
            (Self::AttentionOutput | Self::Down, ProjectionMath::Mfma) => ROOTS_V10[5],
            (_, ProjectionMath::Scalar) => ROOTS_V10[2],
            (_, ProjectionMath::Mfma) => ROOTS_V10[3],
        }
    }

    pub const fn grid(self, rows: u32, world: u32) -> Option<[u32; 3]> {
        if !rows_are_supported(rows) || world != 1 {
            return None;
        }
        Some([rows.div_ceil(16) * (self.shape().n / 16), 1, 1])
    }

    pub const fn accepts(self, rows: u32, world: u32, n: u32, k: u32, tag: u32) -> bool {
        let shape = self.shape();
        rows_are_supported(rows) && world == 1 && shape.n == n && shape.k == k && shape.tag == tag
    }
}

pub const fn rows_are_supported(rows: u32) -> bool {
    rows != 0 && rows <= MAX_ROWS
}

pub const fn norm_shape_is_supported(rows: u32, width: u32, behavior: u32) -> bool {
    behavior == 0
        && rows != 0
        && ((width == HIDDEN && rows <= MAX_ROWS) || (width == 128 && rows <= 512))
}

pub const fn query_to_kv_head(query: u32) -> Option<u32> {
    if query < QUERY_HEADS {
        Some(query / 2)
    } else {
        None
    }
}

pub const fn paged_limits_are_supported(rows: u32, world: u32, pages: u32, stride: u32) -> bool {
    rows_are_supported(rows)
        && world == 1
        && pages != 0
        && pages <= MAX_PHYSICAL_PAGES
        && stride != 0
        && stride <= 512
}
