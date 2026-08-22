use crate::{Identity, Qwen3ModelRole, QWEN3_NO_LAYER, QWEN3_VOCABULARY_SIZE};
use vstd::prelude::*;

verus! {

pub const QWEN3_LAYER_PLAN_STEPS: u32 = 15;
pub const QWEN3_TARGET_PLAN_STEPS: u32 = 544;
pub const QWEN3_DRAFT_PLAN_STEPS: u32 = 424;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Qwen3ExecutionMode {
    Prefill,
    Decode,
    Speculative,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Qwen3PlanBucket {
    PrefillS1T128,
    PrefillS8T128,
    PrefillS1T512,
    PrefillS1T2048,
    DecodeS1C8192,
    DecodeS8C8192,
    DecodeS32C8192,
    SpeculativeS1K4C8192,
    SpeculativeS8K4C8192,
    SpeculativeS1K8C8192,
    SpeculativeS1K16C8192,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Qwen3PlanDimensions {
    pub sequences: u32,
    pub active_tokens: u32,
    pub context_tokens: u32,
}

impl Qwen3PlanBucket {
    pub closed spec fn dimensions_spec(
        self,
        role: Qwen3ModelRole,
        mode: Qwen3ExecutionMode,
    ) -> Option<Qwen3PlanDimensions> {
        match (mode, self) {
            (Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128) => {
                Some(Qwen3PlanDimensions { sequences: 1, active_tokens: 128, context_tokens: 128 })
            },
            (Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS8T128) => {
                Some(Qwen3PlanDimensions { sequences: 8, active_tokens: 128, context_tokens: 128 })
            },
            (Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T512) => {
                Some(Qwen3PlanDimensions { sequences: 1, active_tokens: 512, context_tokens: 512 })
            },
            (Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T2048) => {
                Some(Qwen3PlanDimensions { sequences: 1, active_tokens: 2_048, context_tokens: 2_048 })
            },
            (Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS1C8192) => {
                Some(Qwen3PlanDimensions { sequences: 1, active_tokens: 1, context_tokens: 8_192 })
            },
            (Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS8C8192) => {
                Some(Qwen3PlanDimensions { sequences: 8, active_tokens: 1, context_tokens: 8_192 })
            },
            (Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS32C8192) => {
                Some(Qwen3PlanDimensions { sequences: 32, active_tokens: 1, context_tokens: 8_192 })
            },
            (Qwen3ExecutionMode::Speculative, Qwen3PlanBucket::SpeculativeS1K4C8192) => {
                Some(Qwen3PlanDimensions {
                    sequences: 1,
                    active_tokens: match role { Qwen3ModelRole::Target8B => 5, Qwen3ModelRole::Draft06B => 4 },
                    context_tokens: 8_192,
                })
            },
            (Qwen3ExecutionMode::Speculative, Qwen3PlanBucket::SpeculativeS8K4C8192) => {
                Some(Qwen3PlanDimensions {
                    sequences: 8,
                    active_tokens: match role { Qwen3ModelRole::Target8B => 5, Qwen3ModelRole::Draft06B => 4 },
                    context_tokens: 8_192,
                })
            },
            (Qwen3ExecutionMode::Speculative, Qwen3PlanBucket::SpeculativeS1K8C8192) => {
                Some(Qwen3PlanDimensions {
                    sequences: 1,
                    active_tokens: match role { Qwen3ModelRole::Target8B => 9, Qwen3ModelRole::Draft06B => 8 },
                    context_tokens: 8_192,
                })
            },
            (Qwen3ExecutionMode::Speculative, Qwen3PlanBucket::SpeculativeS1K16C8192) => {
                Some(Qwen3PlanDimensions {
                    sequences: 1,
                    active_tokens: match role { Qwen3ModelRole::Target8B => 17, Qwen3ModelRole::Draft06B => 16 },
                    context_tokens: 8_192,
                })
            },
            _ => None,
        }
    }

    #[must_use]
    pub fn dimensions(
        self,
        role: Qwen3ModelRole,
        mode: Qwen3ExecutionMode,
    ) -> (dimensions: Option<Qwen3PlanDimensions>)
        ensures dimensions == self.dimensions_spec(role, mode),
    {
        match (mode, self) {
            (Qwen3ExecutionMode::Prefill, Self::PrefillS1T128) => {
                Some(Qwen3PlanDimensions { sequences: 1, active_tokens: 128, context_tokens: 128 })
            }
            (Qwen3ExecutionMode::Prefill, Self::PrefillS8T128) => {
                Some(Qwen3PlanDimensions { sequences: 8, active_tokens: 128, context_tokens: 128 })
            }
            (Qwen3ExecutionMode::Prefill, Self::PrefillS1T512) => {
                Some(Qwen3PlanDimensions { sequences: 1, active_tokens: 512, context_tokens: 512 })
            }
            (Qwen3ExecutionMode::Prefill, Self::PrefillS1T2048) => {
                Some(Qwen3PlanDimensions { sequences: 1, active_tokens: 2_048, context_tokens: 2_048 })
            }
            (Qwen3ExecutionMode::Decode, Self::DecodeS1C8192) => {
                Some(Qwen3PlanDimensions { sequences: 1, active_tokens: 1, context_tokens: 8_192 })
            }
            (Qwen3ExecutionMode::Decode, Self::DecodeS8C8192) => {
                Some(Qwen3PlanDimensions { sequences: 8, active_tokens: 1, context_tokens: 8_192 })
            }
            (Qwen3ExecutionMode::Decode, Self::DecodeS32C8192) => {
                Some(Qwen3PlanDimensions { sequences: 32, active_tokens: 1, context_tokens: 8_192 })
            }
            (Qwen3ExecutionMode::Speculative, Self::SpeculativeS1K4C8192) => {
                Some(Qwen3PlanDimensions {
                    sequences: 1,
                    active_tokens: match role { Qwen3ModelRole::Target8B => 5, Qwen3ModelRole::Draft06B => 4 },
                    context_tokens: 8_192,
                })
            }
            (Qwen3ExecutionMode::Speculative, Self::SpeculativeS8K4C8192) => {
                Some(Qwen3PlanDimensions {
                    sequences: 8,
                    active_tokens: match role { Qwen3ModelRole::Target8B => 5, Qwen3ModelRole::Draft06B => 4 },
                    context_tokens: 8_192,
                })
            }
            (Qwen3ExecutionMode::Speculative, Self::SpeculativeS1K8C8192) => {
                Some(Qwen3PlanDimensions {
                    sequences: 1,
                    active_tokens: match role { Qwen3ModelRole::Target8B => 9, Qwen3ModelRole::Draft06B => 8 },
                    context_tokens: 8_192,
                })
            }
            (Qwen3ExecutionMode::Speculative, Self::SpeculativeS1K16C8192) => {
                Some(Qwen3PlanDimensions {
                    sequences: 1,
                    active_tokens: match role { Qwen3ModelRole::Target8B => 17, Qwen3ModelRole::Draft06B => 16 },
                    context_tokens: 8_192,
                })
            }
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Qwen3PlanGeometry {
    pub hidden_size: u32,
    pub intermediate_size: u32,
    pub query_heads: u32,
    pub kv_heads: u32,
    pub head_dim: u32,
    pub gqa_group_size: u32,
}

pub closed spec fn geometry_spec(role: Qwen3ModelRole) -> Qwen3PlanGeometry {
    match role {
        Qwen3ModelRole::Target8B => Qwen3PlanGeometry {
            hidden_size: 4_096,
            intermediate_size: 12_288,
            query_heads: 32,
            kv_heads: 8,
            head_dim: 128,
            gqa_group_size: 4,
        },
        Qwen3ModelRole::Draft06B => Qwen3PlanGeometry {
            hidden_size: 1_024,
            intermediate_size: 3_072,
            query_heads: 16,
            kv_heads: 8,
            head_dim: 128,
            gqa_group_size: 2,
        },
    }
}

#[must_use]
pub fn geometry(role: Qwen3ModelRole) -> (result: Qwen3PlanGeometry)
    ensures result == geometry_spec(role),
{
    match role {
        Qwen3ModelRole::Target8B => Qwen3PlanGeometry {
            hidden_size: 4_096,
            intermediate_size: 12_288,
            query_heads: 32,
            kv_heads: 8,
            head_dim: 128,
            gqa_group_size: 4,
        },
        Qwen3ModelRole::Draft06B => Qwen3PlanGeometry {
            hidden_size: 1_024,
            intermediate_size: 3_072,
            query_heads: 16,
            kv_heads: 8,
            head_dim: 128,
            gqa_group_size: 2,
        },
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Qwen3PlanShape {
    pub rank: u32,
    pub dimension_0: u32,
    pub dimension_1: u32,
    pub dimension_2: u32,
    pub dimension_3: u32,
}

impl Qwen3PlanShape {
    #[must_use]
    pub fn matches(self, other: Self) -> (matches: bool)
        ensures matches == (self == other),
    {
        self.rank == other.rank
            && self.dimension_0 == other.dimension_0
            && self.dimension_1 == other.dimension_1
            && self.dimension_2 == other.dimension_2
            && self.dimension_3 == other.dimension_3
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Qwen3BufferKind {
    Absent,
    TokenIds,
    PositionIds,
    Hidden,
    NormalizedHidden,
    Query,
    Key,
    Value,
    NormalizedQuery,
    NormalizedKey,
    RotatedQuery,
    RotatedKey,
    KvKeys,
    KvValues,
    AttentionOutput,
    HiddenAfterAttention,
    PostAttentionNormalized,
    Gate,
    Up,
    Activated,
    FinalNormalized,
    Logits,
    CompactCompletion,
}

impl Qwen3BufferKind {
    #[must_use]
    pub fn matches(self, other: Self) -> (matches: bool)
        ensures matches == (self == other),
    {
        matches!((self, other),
            (Self::Absent, Self::Absent)
            | (Self::TokenIds, Self::TokenIds)
            | (Self::PositionIds, Self::PositionIds)
            | (Self::Hidden, Self::Hidden)
            | (Self::NormalizedHidden, Self::NormalizedHidden)
            | (Self::Query, Self::Query)
            | (Self::Key, Self::Key)
            | (Self::Value, Self::Value)
            | (Self::NormalizedQuery, Self::NormalizedQuery)
            | (Self::NormalizedKey, Self::NormalizedKey)
            | (Self::RotatedQuery, Self::RotatedQuery)
            | (Self::RotatedKey, Self::RotatedKey)
            | (Self::KvKeys, Self::KvKeys)
            | (Self::KvValues, Self::KvValues)
            | (Self::AttentionOutput, Self::AttentionOutput)
            | (Self::HiddenAfterAttention, Self::HiddenAfterAttention)
            | (Self::PostAttentionNormalized, Self::PostAttentionNormalized)
            | (Self::Gate, Self::Gate)
            | (Self::Up, Self::Up)
            | (Self::Activated, Self::Activated)
            | (Self::FinalNormalized, Self::FinalNormalized)
            | (Self::Logits, Self::Logits)
            | (Self::CompactCompletion, Self::CompactCompletion)
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Qwen3PlanBuffer {
    pub kind: Qwen3BufferKind,
    pub layer: u32,
    pub shape: Qwen3PlanShape,
}

impl Qwen3PlanBuffer {
    #[must_use]
    pub fn matches(self, other: Self) -> (matches: bool)
        ensures matches == (self == other),
    {
        self.kind.matches(other.kind)
            && self.layer == other.layer
            && self.shape.matches(other.shape)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Qwen3Operator {
    TokenEmbedding,
    InputRmsNorm,
    QueryProjection,
    KeyProjection,
    ValueProjection,
    QueryRmsNorm,
    KeyRmsNorm,
    Rope,
    KvWrite,
    Attention,
    AttentionOutputResidual,
    PostAttentionRmsNorm,
    GateProjection,
    UpProjection,
    SwiGlu,
    DownResidual,
    FinalRmsNorm,
    LogitsProjection,
    ArgmaxCompactCompletion,
}

impl Qwen3Operator {
    #[must_use]
    pub fn matches(self, other: Self) -> (matches: bool)
        ensures matches == (self == other),
    {
        matches!((self, other),
            (Self::TokenEmbedding, Self::TokenEmbedding)
            | (Self::InputRmsNorm, Self::InputRmsNorm)
            | (Self::QueryProjection, Self::QueryProjection)
            | (Self::KeyProjection, Self::KeyProjection)
            | (Self::ValueProjection, Self::ValueProjection)
            | (Self::QueryRmsNorm, Self::QueryRmsNorm)
            | (Self::KeyRmsNorm, Self::KeyRmsNorm)
            | (Self::Rope, Self::Rope)
            | (Self::KvWrite, Self::KvWrite)
            | (Self::Attention, Self::Attention)
            | (Self::AttentionOutputResidual, Self::AttentionOutputResidual)
            | (Self::PostAttentionRmsNorm, Self::PostAttentionRmsNorm)
            | (Self::GateProjection, Self::GateProjection)
            | (Self::UpProjection, Self::UpProjection)
            | (Self::SwiGlu, Self::SwiGlu)
            | (Self::DownResidual, Self::DownResidual)
            | (Self::FinalRmsNorm, Self::FinalRmsNorm)
            | (Self::LogitsProjection, Self::LogitsProjection)
            | (Self::ArgmaxCompactCompletion, Self::ArgmaxCompactCompletion)
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Qwen3PlanStep {
    pub ordinal: u32,
    pub layer: u32,
    pub operator: Qwen3Operator,
    pub geometry: Qwen3PlanGeometry,
    pub input_0: Qwen3PlanBuffer,
    pub input_1: Qwen3PlanBuffer,
    pub input_2: Qwen3PlanBuffer,
    pub output_0: Qwen3PlanBuffer,
    pub output_1: Qwen3PlanBuffer,
}

impl Qwen3PlanGeometry {
    #[must_use]
    pub fn matches(self, other: Self) -> (matches: bool)
        ensures matches == (self == other),
    {
        self.hidden_size == other.hidden_size
            && self.intermediate_size == other.intermediate_size
            && self.query_heads == other.query_heads
            && self.kv_heads == other.kv_heads
            && self.head_dim == other.head_dim
            && self.gqa_group_size == other.gqa_group_size
    }
}

impl Qwen3PlanStep {
    #[must_use]
    pub fn matches(self, other: Self) -> (matches: bool)
        ensures matches == (self == other),
    {
        self.ordinal == other.ordinal
            && self.layer == other.layer
            && self.operator.matches(other.operator)
            && self.geometry.matches(other.geometry)
            && self.input_0.matches(other.input_0)
            && self.input_1.matches(other.input_1)
            && self.input_2.matches(other.input_2)
            && self.output_0.matches(other.output_0)
            && self.output_1.matches(other.output_1)
    }
}

pub open spec fn zero_shape_spec() -> Qwen3PlanShape {
    Qwen3PlanShape { rank: 0, dimension_0: 0, dimension_1: 0, dimension_2: 0, dimension_3: 0 }
}

pub open spec fn shape_2_spec(dimension_0: u32, dimension_1: u32) -> Qwen3PlanShape {
    Qwen3PlanShape { rank: 2, dimension_0, dimension_1, dimension_2: 1, dimension_3: 1 }
}

pub open spec fn shape_3_spec(
    dimension_0: u32,
    dimension_1: u32,
    dimension_2: u32,
) -> Qwen3PlanShape {
    Qwen3PlanShape { rank: 3, dimension_0, dimension_1, dimension_2, dimension_3: 1 }
}

pub open spec fn shape_4_spec(
    dimension_0: u32,
    dimension_1: u32,
    dimension_2: u32,
    dimension_3: u32,
) -> Qwen3PlanShape {
    Qwen3PlanShape { rank: 4, dimension_0, dimension_1, dimension_2, dimension_3 }
}

fn zero_shape() -> (shape: Qwen3PlanShape)
    ensures shape == zero_shape_spec(),
{
    Qwen3PlanShape { rank: 0, dimension_0: 0, dimension_1: 0, dimension_2: 0, dimension_3: 0 }
}

fn shape_2(dimension_0: u32, dimension_1: u32) -> (shape: Qwen3PlanShape)
    ensures shape == shape_2_spec(dimension_0, dimension_1),
{
    Qwen3PlanShape { rank: 2, dimension_0, dimension_1, dimension_2: 1, dimension_3: 1 }
}

fn shape_3(
    dimension_0: u32,
    dimension_1: u32,
    dimension_2: u32,
) -> (shape: Qwen3PlanShape)
    ensures shape == shape_3_spec(dimension_0, dimension_1, dimension_2),
{
    Qwen3PlanShape { rank: 3, dimension_0, dimension_1, dimension_2, dimension_3: 1 }
}

fn shape_4(
    dimension_0: u32,
    dimension_1: u32,
    dimension_2: u32,
    dimension_3: u32,
) -> (shape: Qwen3PlanShape)
    ensures shape == shape_4_spec(dimension_0, dimension_1, dimension_2, dimension_3),
{
    Qwen3PlanShape { rank: 4, dimension_0, dimension_1, dimension_2, dimension_3 }
}

pub open spec fn absent_buffer_spec() -> Qwen3PlanBuffer {
    Qwen3PlanBuffer { kind: Qwen3BufferKind::Absent, layer: QWEN3_NO_LAYER, shape: zero_shape_spec() }
}

fn absent_buffer() -> (buffer: Qwen3PlanBuffer)
    ensures buffer == absent_buffer_spec(),
{
    Qwen3PlanBuffer { kind: Qwen3BufferKind::Absent, layer: QWEN3_NO_LAYER, shape: zero_shape() }
}

pub open spec fn buffer_spec(
    kind: Qwen3BufferKind,
    layer: u32,
    shape: Qwen3PlanShape,
) -> Qwen3PlanBuffer {
    Qwen3PlanBuffer { kind, layer, shape }
}

fn buffer(
    kind: Qwen3BufferKind,
    layer: u32,
    shape: Qwen3PlanShape,
) -> (result: Qwen3PlanBuffer)
    ensures result == buffer_spec(kind, layer, shape),
{
    Qwen3PlanBuffer { kind, layer, shape }
}

pub open spec fn next_layer_spec(layer: u32) -> u32 {
    if layer == u32::MAX { layer } else { (layer as int + 1) as u32 }
}

fn next_layer(layer: u32) -> (result: u32)
    ensures result == next_layer_spec(layer),
{
    if layer == u32::MAX { layer } else { layer + 1 }
}

pub open spec fn canonical_step_spec(
    role: Qwen3ModelRole,
    dimensions: Qwen3PlanDimensions,
    ordinal: u32,
    layer: u32,
    operator: Qwen3Operator,
) -> Qwen3PlanStep {
    let geometry = geometry_spec(role);
    let hidden = shape_3_spec(dimensions.sequences, dimensions.active_tokens, geometry.hidden_size);
    let query = shape_4_spec(
        dimensions.sequences,
        dimensions.active_tokens,
        geometry.query_heads,
        geometry.head_dim,
    );
    let attention = shape_3_spec(
        dimensions.sequences,
        dimensions.active_tokens,
        (geometry.query_heads * geometry.head_dim) as u32,
    );
    let kv = shape_4_spec(
        dimensions.sequences,
        dimensions.active_tokens,
        geometry.kv_heads,
        geometry.head_dim,
    );
    let cache = shape_4_spec(
        dimensions.sequences,
        dimensions.context_tokens,
        geometry.kv_heads,
        geometry.head_dim,
    );
    let intermediate = shape_3_spec(
        dimensions.sequences,
        dimensions.active_tokens,
        geometry.intermediate_size,
    );
    let none = absent_buffer_spec();
    match operator {
        Qwen3Operator::TokenEmbedding => Qwen3PlanStep {
            ordinal,
            layer: QWEN3_NO_LAYER,
            operator,
            geometry,
            input_0: buffer_spec(Qwen3BufferKind::TokenIds, QWEN3_NO_LAYER, shape_2_spec(dimensions.sequences, dimensions.active_tokens)),
            input_1: none,
            input_2: none,
            output_0: buffer_spec(Qwen3BufferKind::Hidden, 0, hidden),
            output_1: none,
        },
        Qwen3Operator::InputRmsNorm => Qwen3PlanStep {
            ordinal, layer, operator, geometry,
            input_0: buffer_spec(Qwen3BufferKind::Hidden, layer, hidden),
            input_1: none, input_2: none,
            output_0: buffer_spec(Qwen3BufferKind::NormalizedHidden, layer, hidden),
            output_1: none,
        },
        Qwen3Operator::QueryProjection => Qwen3PlanStep {
            ordinal, layer, operator, geometry,
            input_0: buffer_spec(Qwen3BufferKind::NormalizedHidden, layer, hidden),
            input_1: none, input_2: none,
            output_0: buffer_spec(Qwen3BufferKind::Query, layer, query), output_1: none,
        },
        Qwen3Operator::KeyProjection => Qwen3PlanStep {
            ordinal, layer, operator, geometry,
            input_0: buffer_spec(Qwen3BufferKind::NormalizedHidden, layer, hidden),
            input_1: none, input_2: none,
            output_0: buffer_spec(Qwen3BufferKind::Key, layer, kv), output_1: none,
        },
        Qwen3Operator::ValueProjection => Qwen3PlanStep {
            ordinal, layer, operator, geometry,
            input_0: buffer_spec(Qwen3BufferKind::NormalizedHidden, layer, hidden),
            input_1: none, input_2: none,
            output_0: buffer_spec(Qwen3BufferKind::Value, layer, kv), output_1: none,
        },
        Qwen3Operator::QueryRmsNorm => Qwen3PlanStep {
            ordinal, layer, operator, geometry,
            input_0: buffer_spec(Qwen3BufferKind::Query, layer, query),
            input_1: none, input_2: none,
            output_0: buffer_spec(Qwen3BufferKind::NormalizedQuery, layer, query), output_1: none,
        },
        Qwen3Operator::KeyRmsNorm => Qwen3PlanStep {
            ordinal, layer, operator, geometry,
            input_0: buffer_spec(Qwen3BufferKind::Key, layer, kv),
            input_1: none, input_2: none,
            output_0: buffer_spec(Qwen3BufferKind::NormalizedKey, layer, kv), output_1: none,
        },
        Qwen3Operator::Rope => Qwen3PlanStep {
            ordinal, layer, operator, geometry,
            input_0: buffer_spec(Qwen3BufferKind::NormalizedQuery, layer, query),
            input_1: buffer_spec(Qwen3BufferKind::NormalizedKey, layer, kv),
            input_2: buffer_spec(Qwen3BufferKind::PositionIds, QWEN3_NO_LAYER, shape_2_spec(dimensions.sequences, dimensions.active_tokens)),
            output_0: buffer_spec(Qwen3BufferKind::RotatedQuery, layer, query),
            output_1: buffer_spec(Qwen3BufferKind::RotatedKey, layer, kv),
        },
        Qwen3Operator::KvWrite => Qwen3PlanStep {
            ordinal, layer, operator, geometry,
            input_0: buffer_spec(Qwen3BufferKind::RotatedKey, layer, kv),
            input_1: buffer_spec(Qwen3BufferKind::Value, layer, kv), input_2: none,
            output_0: buffer_spec(Qwen3BufferKind::KvKeys, layer, cache),
            output_1: buffer_spec(Qwen3BufferKind::KvValues, layer, cache),
        },
        Qwen3Operator::Attention => Qwen3PlanStep {
            ordinal, layer, operator, geometry,
            input_0: buffer_spec(Qwen3BufferKind::RotatedQuery, layer, query),
            input_1: buffer_spec(Qwen3BufferKind::KvKeys, layer, cache),
            input_2: buffer_spec(Qwen3BufferKind::KvValues, layer, cache),
            output_0: buffer_spec(Qwen3BufferKind::AttentionOutput, layer, attention), output_1: none,
        },
        Qwen3Operator::AttentionOutputResidual => Qwen3PlanStep {
            ordinal, layer, operator, geometry,
            input_0: buffer_spec(Qwen3BufferKind::AttentionOutput, layer, attention),
            input_1: buffer_spec(Qwen3BufferKind::Hidden, layer, hidden), input_2: none,
            output_0: buffer_spec(Qwen3BufferKind::HiddenAfterAttention, layer, hidden), output_1: none,
        },
        Qwen3Operator::PostAttentionRmsNorm => Qwen3PlanStep {
            ordinal, layer, operator, geometry,
            input_0: buffer_spec(Qwen3BufferKind::HiddenAfterAttention, layer, hidden),
            input_1: none, input_2: none,
            output_0: buffer_spec(Qwen3BufferKind::PostAttentionNormalized, layer, hidden), output_1: none,
        },
        Qwen3Operator::GateProjection => Qwen3PlanStep {
            ordinal, layer, operator, geometry,
            input_0: buffer_spec(Qwen3BufferKind::PostAttentionNormalized, layer, hidden),
            input_1: none, input_2: none,
            output_0: buffer_spec(Qwen3BufferKind::Gate, layer, intermediate), output_1: none,
        },
        Qwen3Operator::UpProjection => Qwen3PlanStep {
            ordinal, layer, operator, geometry,
            input_0: buffer_spec(Qwen3BufferKind::PostAttentionNormalized, layer, hidden),
            input_1: none, input_2: none,
            output_0: buffer_spec(Qwen3BufferKind::Up, layer, intermediate), output_1: none,
        },
        Qwen3Operator::SwiGlu => Qwen3PlanStep {
            ordinal, layer, operator, geometry,
            input_0: buffer_spec(Qwen3BufferKind::Gate, layer, intermediate),
            input_1: buffer_spec(Qwen3BufferKind::Up, layer, intermediate), input_2: none,
            output_0: buffer_spec(Qwen3BufferKind::Activated, layer, intermediate), output_1: none,
        },
        Qwen3Operator::DownResidual => Qwen3PlanStep {
            ordinal, layer, operator, geometry,
            input_0: buffer_spec(Qwen3BufferKind::Activated, layer, intermediate),
            input_1: buffer_spec(Qwen3BufferKind::HiddenAfterAttention, layer, hidden), input_2: none,
            output_0: buffer_spec(Qwen3BufferKind::Hidden, next_layer_spec(layer), hidden), output_1: none,
        },
        Qwen3Operator::FinalRmsNorm => Qwen3PlanStep {
            ordinal, layer: QWEN3_NO_LAYER, operator, geometry,
            input_0: buffer_spec(Qwen3BufferKind::Hidden, layer, hidden),
            input_1: none, input_2: none,
            output_0: buffer_spec(Qwen3BufferKind::FinalNormalized, QWEN3_NO_LAYER, hidden), output_1: none,
        },
        Qwen3Operator::LogitsProjection => Qwen3PlanStep {
            ordinal, layer: QWEN3_NO_LAYER, operator, geometry,
            input_0: buffer_spec(Qwen3BufferKind::FinalNormalized, QWEN3_NO_LAYER, hidden),
            input_1: none, input_2: none,
            output_0: buffer_spec(Qwen3BufferKind::Logits, QWEN3_NO_LAYER, shape_3_spec(dimensions.sequences, dimensions.active_tokens, QWEN3_VOCABULARY_SIZE)),
            output_1: none,
        },
        Qwen3Operator::ArgmaxCompactCompletion => Qwen3PlanStep {
            ordinal, layer: QWEN3_NO_LAYER, operator, geometry,
            input_0: buffer_spec(Qwen3BufferKind::Logits, QWEN3_NO_LAYER, shape_3_spec(dimensions.sequences, dimensions.active_tokens, QWEN3_VOCABULARY_SIZE)),
            input_1: none, input_2: none,
            output_0: buffer_spec(Qwen3BufferKind::CompactCompletion, QWEN3_NO_LAYER, shape_2_spec(dimensions.sequences, dimensions.active_tokens)),
            output_1: none,
        },
    }
}

fn canonical_step(
    role: Qwen3ModelRole,
    dimensions: Qwen3PlanDimensions,
    ordinal: u32,
    layer: u32,
    operator: Qwen3Operator,
) -> (step: Qwen3PlanStep)
    ensures step == canonical_step_spec(role, dimensions, ordinal, layer, operator),
{
    let geometry = geometry(role);
    let hidden = shape_3(dimensions.sequences, dimensions.active_tokens, geometry.hidden_size);
    let query = shape_4(
        dimensions.sequences,
        dimensions.active_tokens,
        geometry.query_heads,
        geometry.head_dim,
    );
    let attention = shape_3(
        dimensions.sequences,
        dimensions.active_tokens,
        geometry.query_heads * geometry.head_dim,
    );
    let kv = shape_4(
        dimensions.sequences,
        dimensions.active_tokens,
        geometry.kv_heads,
        geometry.head_dim,
    );
    let cache = shape_4(
        dimensions.sequences,
        dimensions.context_tokens,
        geometry.kv_heads,
        geometry.head_dim,
    );
    let intermediate = shape_3(
        dimensions.sequences,
        dimensions.active_tokens,
        geometry.intermediate_size,
    );
    let none = absent_buffer();
    match operator {
        Qwen3Operator::TokenEmbedding => Qwen3PlanStep {
            ordinal, layer: QWEN3_NO_LAYER, operator, geometry,
            input_0: buffer(Qwen3BufferKind::TokenIds, QWEN3_NO_LAYER, shape_2(dimensions.sequences, dimensions.active_tokens)),
            input_1: none, input_2: none,
            output_0: buffer(Qwen3BufferKind::Hidden, 0, hidden), output_1: none,
        },
        Qwen3Operator::InputRmsNorm => Qwen3PlanStep {
            ordinal, layer, operator, geometry,
            input_0: buffer(Qwen3BufferKind::Hidden, layer, hidden), input_1: none, input_2: none,
            output_0: buffer(Qwen3BufferKind::NormalizedHidden, layer, hidden), output_1: none,
        },
        Qwen3Operator::QueryProjection => Qwen3PlanStep {
            ordinal, layer, operator, geometry,
            input_0: buffer(Qwen3BufferKind::NormalizedHidden, layer, hidden), input_1: none, input_2: none,
            output_0: buffer(Qwen3BufferKind::Query, layer, query), output_1: none,
        },
        Qwen3Operator::KeyProjection => Qwen3PlanStep {
            ordinal, layer, operator, geometry,
            input_0: buffer(Qwen3BufferKind::NormalizedHidden, layer, hidden), input_1: none, input_2: none,
            output_0: buffer(Qwen3BufferKind::Key, layer, kv), output_1: none,
        },
        Qwen3Operator::ValueProjection => Qwen3PlanStep {
            ordinal, layer, operator, geometry,
            input_0: buffer(Qwen3BufferKind::NormalizedHidden, layer, hidden), input_1: none, input_2: none,
            output_0: buffer(Qwen3BufferKind::Value, layer, kv), output_1: none,
        },
        Qwen3Operator::QueryRmsNorm => Qwen3PlanStep {
            ordinal, layer, operator, geometry,
            input_0: buffer(Qwen3BufferKind::Query, layer, query), input_1: none, input_2: none,
            output_0: buffer(Qwen3BufferKind::NormalizedQuery, layer, query), output_1: none,
        },
        Qwen3Operator::KeyRmsNorm => Qwen3PlanStep {
            ordinal, layer, operator, geometry,
            input_0: buffer(Qwen3BufferKind::Key, layer, kv), input_1: none, input_2: none,
            output_0: buffer(Qwen3BufferKind::NormalizedKey, layer, kv), output_1: none,
        },
        Qwen3Operator::Rope => Qwen3PlanStep {
            ordinal, layer, operator, geometry,
            input_0: buffer(Qwen3BufferKind::NormalizedQuery, layer, query),
            input_1: buffer(Qwen3BufferKind::NormalizedKey, layer, kv),
            input_2: buffer(Qwen3BufferKind::PositionIds, QWEN3_NO_LAYER, shape_2(dimensions.sequences, dimensions.active_tokens)),
            output_0: buffer(Qwen3BufferKind::RotatedQuery, layer, query),
            output_1: buffer(Qwen3BufferKind::RotatedKey, layer, kv),
        },
        Qwen3Operator::KvWrite => Qwen3PlanStep {
            ordinal, layer, operator, geometry,
            input_0: buffer(Qwen3BufferKind::RotatedKey, layer, kv),
            input_1: buffer(Qwen3BufferKind::Value, layer, kv), input_2: none,
            output_0: buffer(Qwen3BufferKind::KvKeys, layer, cache),
            output_1: buffer(Qwen3BufferKind::KvValues, layer, cache),
        },
        Qwen3Operator::Attention => Qwen3PlanStep {
            ordinal, layer, operator, geometry,
            input_0: buffer(Qwen3BufferKind::RotatedQuery, layer, query),
            input_1: buffer(Qwen3BufferKind::KvKeys, layer, cache),
            input_2: buffer(Qwen3BufferKind::KvValues, layer, cache),
            output_0: buffer(Qwen3BufferKind::AttentionOutput, layer, attention), output_1: none,
        },
        Qwen3Operator::AttentionOutputResidual => Qwen3PlanStep {
            ordinal, layer, operator, geometry,
            input_0: buffer(Qwen3BufferKind::AttentionOutput, layer, attention),
            input_1: buffer(Qwen3BufferKind::Hidden, layer, hidden), input_2: none,
            output_0: buffer(Qwen3BufferKind::HiddenAfterAttention, layer, hidden), output_1: none,
        },
        Qwen3Operator::PostAttentionRmsNorm => Qwen3PlanStep {
            ordinal, layer, operator, geometry,
            input_0: buffer(Qwen3BufferKind::HiddenAfterAttention, layer, hidden), input_1: none, input_2: none,
            output_0: buffer(Qwen3BufferKind::PostAttentionNormalized, layer, hidden), output_1: none,
        },
        Qwen3Operator::GateProjection => Qwen3PlanStep {
            ordinal, layer, operator, geometry,
            input_0: buffer(Qwen3BufferKind::PostAttentionNormalized, layer, hidden), input_1: none, input_2: none,
            output_0: buffer(Qwen3BufferKind::Gate, layer, intermediate), output_1: none,
        },
        Qwen3Operator::UpProjection => Qwen3PlanStep {
            ordinal, layer, operator, geometry,
            input_0: buffer(Qwen3BufferKind::PostAttentionNormalized, layer, hidden), input_1: none, input_2: none,
            output_0: buffer(Qwen3BufferKind::Up, layer, intermediate), output_1: none,
        },
        Qwen3Operator::SwiGlu => Qwen3PlanStep {
            ordinal, layer, operator, geometry,
            input_0: buffer(Qwen3BufferKind::Gate, layer, intermediate),
            input_1: buffer(Qwen3BufferKind::Up, layer, intermediate), input_2: none,
            output_0: buffer(Qwen3BufferKind::Activated, layer, intermediate), output_1: none,
        },
        Qwen3Operator::DownResidual => Qwen3PlanStep {
            ordinal, layer, operator, geometry,
            input_0: buffer(Qwen3BufferKind::Activated, layer, intermediate),
            input_1: buffer(Qwen3BufferKind::HiddenAfterAttention, layer, hidden), input_2: none,
            output_0: buffer(Qwen3BufferKind::Hidden, next_layer(layer), hidden), output_1: none,
        },
        Qwen3Operator::FinalRmsNorm => Qwen3PlanStep {
            ordinal, layer: QWEN3_NO_LAYER, operator, geometry,
            input_0: buffer(Qwen3BufferKind::Hidden, layer, hidden), input_1: none, input_2: none,
            output_0: buffer(Qwen3BufferKind::FinalNormalized, QWEN3_NO_LAYER, hidden), output_1: none,
        },
        Qwen3Operator::LogitsProjection => Qwen3PlanStep {
            ordinal, layer: QWEN3_NO_LAYER, operator, geometry,
            input_0: buffer(Qwen3BufferKind::FinalNormalized, QWEN3_NO_LAYER, hidden), input_1: none, input_2: none,
            output_0: buffer(Qwen3BufferKind::Logits, QWEN3_NO_LAYER, shape_3(dimensions.sequences, dimensions.active_tokens, QWEN3_VOCABULARY_SIZE)),
            output_1: none,
        },
        Qwen3Operator::ArgmaxCompactCompletion => Qwen3PlanStep {
            ordinal, layer: QWEN3_NO_LAYER, operator, geometry,
            input_0: buffer(Qwen3BufferKind::Logits, QWEN3_NO_LAYER, shape_3(dimensions.sequences, dimensions.active_tokens, QWEN3_VOCABULARY_SIZE)),
            input_1: none, input_2: none,
            output_0: buffer(Qwen3BufferKind::CompactCompletion, QWEN3_NO_LAYER, shape_2(dimensions.sequences, dimensions.active_tokens)),
            output_1: none,
        },
    }
}

#[must_use]
pub const fn plan_step_count(role: Qwen3ModelRole) -> (count: u32)
    ensures count == plan_step_count_spec(role),
{
    match role {
        Qwen3ModelRole::Target8B => QWEN3_TARGET_PLAN_STEPS,
        Qwen3ModelRole::Draft06B => QWEN3_DRAFT_PLAN_STEPS,
    }
}

pub closed spec fn plan_step_count_spec(role: Qwen3ModelRole) -> u32 {
    match role {
        Qwen3ModelRole::Target8B => QWEN3_TARGET_PLAN_STEPS,
        Qwen3ModelRole::Draft06B => QWEN3_DRAFT_PLAN_STEPS,
    }
}

pub(crate) proof fn plan_step_count_is_role_exact(role: Qwen3ModelRole)
    ensures
        plan_step_count_spec(role) == match role {
            Qwen3ModelRole::Target8B => QWEN3_TARGET_PLAN_STEPS,
            Qwen3ModelRole::Draft06B => QWEN3_DRAFT_PLAN_STEPS,
        },
{
    reveal(plan_step_count_spec);
}

pub open spec fn expected_step_spec(
    role: Qwen3ModelRole,
    mode: Qwen3ExecutionMode,
    bucket: Qwen3PlanBucket,
    ordinal: u32,
) -> Option<Qwen3PlanStep> {
    match bucket.dimensions_spec(role, mode) {
        None => None,
        Some(dimensions) => {
            let layer_steps: int = match role {
                Qwen3ModelRole::Target8B => 540,
                Qwen3ModelRole::Draft06B => 420,
            };
            let layers = match role {
                Qwen3ModelRole::Target8B => 36u32,
                Qwen3ModelRole::Draft06B => 28u32,
            };
            if ordinal as int == 0 {
                Some(canonical_step_spec(role, dimensions, ordinal, QWEN3_NO_LAYER, Qwen3Operator::TokenEmbedding))
            } else if ordinal as int <= layer_steps {
                let offset = ordinal as int - 1;
                let layer = (offset / QWEN3_LAYER_PLAN_STEPS as int) as u32;
                let slot = offset % QWEN3_LAYER_PLAN_STEPS as int;
                let operator = if slot == 0 {
                    Qwen3Operator::InputRmsNorm
                } else if slot == 1 {
                    Qwen3Operator::QueryProjection
                } else if slot == 2 {
                    Qwen3Operator::KeyProjection
                } else if slot == 3 {
                    Qwen3Operator::ValueProjection
                } else if slot == 4 {
                    Qwen3Operator::QueryRmsNorm
                } else if slot == 5 {
                    Qwen3Operator::KeyRmsNorm
                } else if slot == 6 {
                    Qwen3Operator::Rope
                } else if slot == 7 {
                    Qwen3Operator::KvWrite
                } else if slot == 8 {
                    Qwen3Operator::Attention
                } else if slot == 9 {
                    Qwen3Operator::AttentionOutputResidual
                } else if slot == 10 {
                    Qwen3Operator::PostAttentionRmsNorm
                } else if slot == 11 {
                    Qwen3Operator::GateProjection
                } else if slot == 12 {
                    Qwen3Operator::UpProjection
                } else if slot == 13 {
                    Qwen3Operator::SwiGlu
                } else {
                    Qwen3Operator::DownResidual
                };
                Some(canonical_step_spec(role, dimensions, ordinal, layer, operator))
            } else if ordinal as int == layer_steps + 1 {
                Some(canonical_step_spec(role, dimensions, ordinal, layers, Qwen3Operator::FinalRmsNorm))
            } else if ordinal as int == layer_steps + 2 {
                Some(canonical_step_spec(role, dimensions, ordinal, QWEN3_NO_LAYER, Qwen3Operator::LogitsProjection))
            } else if ordinal as int == layer_steps + 3 {
                Some(canonical_step_spec(role, dimensions, ordinal, QWEN3_NO_LAYER, Qwen3Operator::ArgmaxCompactCompletion))
            } else {
                None
            }
        },
    }
}

#[must_use]
pub fn expected_step(
    role: Qwen3ModelRole,
    mode: Qwen3ExecutionMode,
    bucket: Qwen3PlanBucket,
    ordinal: u32,
) -> (step: Option<Qwen3PlanStep>)
    ensures step == expected_step_spec(role, mode, bucket, ordinal),
{
    let dimensions = bucket.dimensions(role, mode)?;
    let (layer_steps, layers) = match role {
        Qwen3ModelRole::Target8B => (540u32, 36u32),
        Qwen3ModelRole::Draft06B => (420u32, 28u32),
    };
    if ordinal == 0 {
        return Some(canonical_step(role, dimensions, ordinal, QWEN3_NO_LAYER, Qwen3Operator::TokenEmbedding));
    }
    if ordinal <= layer_steps {
        let offset = ordinal - 1;
        let layer = offset / QWEN3_LAYER_PLAN_STEPS;
        let operator = match offset % QWEN3_LAYER_PLAN_STEPS {
            0 => Qwen3Operator::InputRmsNorm,
            1 => Qwen3Operator::QueryProjection,
            2 => Qwen3Operator::KeyProjection,
            3 => Qwen3Operator::ValueProjection,
            4 => Qwen3Operator::QueryRmsNorm,
            5 => Qwen3Operator::KeyRmsNorm,
            6 => Qwen3Operator::Rope,
            7 => Qwen3Operator::KvWrite,
            8 => Qwen3Operator::Attention,
            9 => Qwen3Operator::AttentionOutputResidual,
            10 => Qwen3Operator::PostAttentionRmsNorm,
            11 => Qwen3Operator::GateProjection,
            12 => Qwen3Operator::UpProjection,
            13 => Qwen3Operator::SwiGlu,
            _ => Qwen3Operator::DownResidual,
        };
        return Some(canonical_step(role, dimensions, ordinal, layer, operator));
    }
    if ordinal == layer_steps + 1 {
        return Some(canonical_step(role, dimensions, ordinal, layers, Qwen3Operator::FinalRmsNorm));
    }
    if ordinal == layer_steps + 2 {
        return Some(canonical_step(role, dimensions, ordinal, QWEN3_NO_LAYER, Qwen3Operator::LogitsProjection));
    }
    if ordinal == layer_steps + 3 {
        return Some(canonical_step(role, dimensions, ordinal, QWEN3_NO_LAYER, Qwen3Operator::ArgmaxCompactCompletion));
    }
    None
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Qwen3PlanSelection {
    pub role: Qwen3ModelRole,
    pub mode: Qwen3ExecutionMode,
    pub bucket: Qwen3PlanBucket,
}

impl Qwen3PlanSelection {
    pub closed spec fn valid(self) -> bool {
        self.bucket.dimensions_spec(self.role, self.mode).is_some()
    }

    #[must_use]
    pub fn matches(self, other: Self) -> (matches: bool)
        ensures matches == (self == other),
    {
        let role_matches = matches!((self.role, other.role),
            (Qwen3ModelRole::Target8B, Qwen3ModelRole::Target8B)
                | (Qwen3ModelRole::Draft06B, Qwen3ModelRole::Draft06B)
        );
        let mode_matches = matches!((self.mode, other.mode),
            (Qwen3ExecutionMode::Prefill, Qwen3ExecutionMode::Prefill)
                | (Qwen3ExecutionMode::Decode, Qwen3ExecutionMode::Decode)
                | (Qwen3ExecutionMode::Speculative, Qwen3ExecutionMode::Speculative)
        );
        let bucket_matches = matches!((self.bucket, other.bucket),
            (Qwen3PlanBucket::PrefillS1T128, Qwen3PlanBucket::PrefillS1T128)
                | (Qwen3PlanBucket::PrefillS8T128, Qwen3PlanBucket::PrefillS8T128)
                | (Qwen3PlanBucket::PrefillS1T512, Qwen3PlanBucket::PrefillS1T512)
                | (Qwen3PlanBucket::PrefillS1T2048, Qwen3PlanBucket::PrefillS1T2048)
                | (Qwen3PlanBucket::DecodeS1C8192, Qwen3PlanBucket::DecodeS1C8192)
                | (Qwen3PlanBucket::DecodeS8C8192, Qwen3PlanBucket::DecodeS8C8192)
                | (Qwen3PlanBucket::DecodeS32C8192, Qwen3PlanBucket::DecodeS32C8192)
                | (Qwen3PlanBucket::SpeculativeS1K4C8192, Qwen3PlanBucket::SpeculativeS1K4C8192)
                | (Qwen3PlanBucket::SpeculativeS8K4C8192, Qwen3PlanBucket::SpeculativeS8K4C8192)
                | (Qwen3PlanBucket::SpeculativeS1K8C8192, Qwen3PlanBucket::SpeculativeS1K8C8192)
                | (Qwen3PlanBucket::SpeculativeS1K16C8192, Qwen3PlanBucket::SpeculativeS1K16C8192)
        );
        role_matches && mode_matches && bucket_matches
    }

    /// Validates an admitted execution mode and finite plan bucket pair.
    ///
    /// # Errors
    ///
    /// Returns [`Qwen3PlanError::ModeBucketMismatch`] when the bucket belongs
    /// to a different execution mode.
    pub fn validate(self) -> (result: Result<(), Qwen3PlanError>)
        ensures result.is_ok() == self.valid(),
    {
        if self.bucket.dimensions(self.role, self.mode).is_none() {
            return Err(Qwen3PlanError::ModeBucketMismatch);
        }
        Ok(())
    }

    fn validate_against(self, expected: Self) -> (result: Result<(), Qwen3PlanError>)
        ensures result.is_ok() == (self.valid() && expected.valid() && self == expected),
    {
        self.validate()?;
        expected.validate()?;
        if !self.matches(expected) {
            return Err(Qwen3PlanError::SelectionMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Qwen3PlanAuthority {
    pub bundle_id: Identity,
    pub model_id: Identity,
    pub config_id: Identity,
    pub graph_id: Identity,
    pub plan_id: Identity,
    pub revision: u64,
}

impl Qwen3PlanAuthority {
    pub closed spec fn valid(self) -> bool {
        (exists|index: int| 0 <= index < self.bundle_id.bytes_spec().len() && self.bundle_id.bytes_spec()[index] != 0)
            && (exists|index: int| 0 <= index < self.model_id.bytes_spec().len() && self.model_id.bytes_spec()[index] != 0)
            && (exists|index: int| 0 <= index < self.config_id.bytes_spec().len() && self.config_id.bytes_spec()[index] != 0)
            && (exists|index: int| 0 <= index < self.graph_id.bytes_spec().len() && self.graph_id.bytes_spec()[index] != 0)
            && (exists|index: int| 0 <= index < self.plan_id.bytes_spec().len() && self.plan_id.bytes_spec()[index] != 0)
            && self.revision > 0
    }

    pub closed spec fn matches(self, expected: Self) -> bool {
        self.valid()
            && expected.valid()
            && self.bundle_id.bytes_spec() == expected.bundle_id.bytes_spec()
            && self.model_id.bytes_spec() == expected.model_id.bytes_spec()
            && self.config_id.bytes_spec() == expected.config_id.bytes_spec()
            && self.graph_id.bytes_spec() == expected.graph_id.bytes_spec()
            && self.plan_id.bytes_spec() == expected.plan_id.bytes_spec()
            && self.revision == expected.revision
    }

    fn validate_present(self) -> (result: Result<(), Qwen3PlanError>)
        ensures result.is_ok() == self.valid(),
    {
        if !self.bundle_id.is_present() { return Err(Qwen3PlanError::MissingIdentity("bundle_id")); }
        if !self.model_id.is_present() { return Err(Qwen3PlanError::MissingIdentity("model_id")); }
        if !self.config_id.is_present() { return Err(Qwen3PlanError::MissingIdentity("config_id")); }
        if !self.graph_id.is_present() { return Err(Qwen3PlanError::MissingIdentity("graph_id")); }
        if !self.plan_id.is_present() { return Err(Qwen3PlanError::MissingIdentity("plan_id")); }
        if self.revision == 0 { return Err(Qwen3PlanError::ZeroRevision); }
        Ok(())
    }

    fn validate_against(self, expected: Self) -> (result: Result<(), Qwen3PlanError>)
        ensures result.is_ok() == self.matches(expected),
    {
        self.validate_present()?;
        expected.validate_present()?;
        if !self.bundle_id.equals(&expected.bundle_id) { return Err(Qwen3PlanError::StaleIdentity("bundle_id")); }
        if !self.model_id.equals(&expected.model_id) { return Err(Qwen3PlanError::StaleIdentity("model_id")); }
        if !self.config_id.equals(&expected.config_id) { return Err(Qwen3PlanError::StaleIdentity("config_id")); }
        if !self.graph_id.equals(&expected.graph_id) { return Err(Qwen3PlanError::StaleIdentity("graph_id")); }
        if !self.plan_id.equals(&expected.plan_id) { return Err(Qwen3PlanError::StaleIdentity("plan_id")); }
        if self.revision != expected.revision { return Err(Qwen3PlanError::StaleRevision); }
        Ok(())
    }
}

#[verifier::allow(autoderive_clone_without_spec)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Qwen3GeneratedPlan {
    pub authority: Qwen3PlanAuthority,
    pub selection: Qwen3PlanSelection,
    pub steps: Vec<Qwen3PlanStep>,
}

impl Qwen3GeneratedPlan {
    pub closed spec fn valid_for(
        &self,
        expected_authority: Qwen3PlanAuthority,
        expected_selection: Qwen3PlanSelection,
    ) -> bool {
        self.authority.matches(expected_authority)
            && self.selection.valid()
            && expected_selection.valid()
            && self.selection == expected_selection
            && self.steps@.len() == plan_step_count_spec(self.selection.role) as nat
            && forall|index: int| 0 <= index < self.steps@.len() ==> expected_step_spec(
                self.selection.role,
                self.selection.mode,
                self.selection.bucket,
                index as u32,
            ) == Some(self.steps@[index])
    }

    /// Validates an exact finite generated-plan input against its authority.
    ///
    /// This closes only the sequential Qwen3 graph-input contract. It does not
    /// establish kernel, generated-runner, or machine-code refinement.
    ///
    /// # Errors
    ///
    /// Returns [`Qwen3PlanError`] for an absent or stale identity, a selection
    /// that differs from the independently expected role/mode/bucket, an
    /// invalid mode/bucket pair, omitted step, extra step, reordered operator,
    /// or any layer, geometry, GQA, buffer-edge, or shape substitution.
    pub fn validate(
        &self,
        expected_authority: Qwen3PlanAuthority,
        expected_selection: Qwen3PlanSelection,
    ) -> (result: Result<(), Qwen3PlanError>)
        ensures result.is_ok() == self.valid_for(expected_authority, expected_selection),
    {
        self.authority.validate_against(expected_authority)?;
        self.selection.validate_against(expected_selection)?;
        let expected_count = plan_step_count(self.selection.role);
        if self.steps.len() != expected_count as usize {
            return Err(Qwen3PlanError::StepCount {
                expected: expected_count,
                actual: self.steps.len(),
            });
        }
        let mut ordinal = 0u32;
        while ordinal < expected_count
            invariant
                self.authority.matches(expected_authority),
                self.selection.valid(),
                expected_selection.valid(),
                self.selection == expected_selection,
                self.steps@.len() == plan_step_count_spec(self.selection.role) as nat,
                expected_count == plan_step_count_spec(self.selection.role),
                0 <= ordinal <= expected_count,
                forall|prior: int| 0 <= prior < ordinal ==> expected_step_spec(
                    self.selection.role,
                    self.selection.mode,
                    self.selection.bucket,
                    prior as u32,
                ) == Some(self.steps@[prior]),
            decreases expected_count - ordinal,
        {
            let expected = expected_step(
                self.selection.role,
                self.selection.mode,
                self.selection.bucket,
                ordinal,
            );
            let actual_step = self.steps[ordinal as usize];
            let Some(expected_step) = expected else {
                assert(expected_step_spec(
                    self.selection.role,
                    self.selection.mode,
                    self.selection.bucket,
                    ordinal,
                ).is_none());
                return Err(Qwen3PlanError::StepMismatch { ordinal });
            };
            if !actual_step.matches(expected_step) {
                assert(expected_step_spec(
                    self.selection.role,
                    self.selection.mode,
                    self.selection.bucket,
                    ordinal,
                ) != Some(self.steps@[ordinal as int]));
                return Err(Qwen3PlanError::StepMismatch { ordinal });
            }
            assert(expected_step_spec(
                self.selection.role,
                self.selection.mode,
                self.selection.bucket,
                ordinal,
            ) == Some(self.steps@[ordinal as int]));
            assert forall|prior: int| 0 <= prior < ordinal + 1 implies expected_step_spec(
                self.selection.role,
                self.selection.mode,
                self.selection.bucket,
                prior as u32,
            ) == Some(self.steps@[prior]) by {
                if prior < ordinal {
                    assert(expected_step_spec(
                        self.selection.role,
                        self.selection.mode,
                        self.selection.bucket,
                        prior as u32,
                    ) == Some(self.steps@[prior]));
                } else {
                    assert(prior == ordinal);
                }
            }
            ordinal += 1;
        }
        Ok(())
    }

    /// Exposes exact ordered logical steps and the plan identity carried by a
    /// valid generated Qwen3 plan.
    ///
    /// This source-level graph fact grants no kernel, address, dispatch,
    /// queue, numerical, or machine-execution authority.
    pub proof fn expose_valid_steps(
        &self,
        expected_authority: Qwen3PlanAuthority,
        expected_selection: Qwen3PlanSelection,
    )
        requires self.valid_for(expected_authority, expected_selection),
        ensures
            self.selection == expected_selection,
            self.steps@.len() == match expected_selection.role {
                Qwen3ModelRole::Target8B => QWEN3_TARGET_PLAN_STEPS as nat,
                Qwen3ModelRole::Draft06B => QWEN3_DRAFT_PLAN_STEPS as nat,
            },
            self.authority.plan_id.bytes_spec() == expected_authority.plan_id.bytes_spec(),
            forall|index: int| 0 <= index < self.steps@.len()
                ==> crate::canonical_expected_step_spec(
                    expected_selection.role,
                    expected_selection.mode,
                    expected_selection.bucket,
                    index as u32,
                ) == Some(self.steps@[index]),
    {
        reveal(Qwen3GeneratedPlan::valid_for);
        reveal(Qwen3PlanAuthority::matches);
        reveal(crate::canonical_expected_step_spec);
        plan_step_count_is_role_exact(expected_selection.role);
    }

    /// A valid target graph has exactly one compact-completion operation, at
    /// its final logical ordinal.
    pub proof fn expose_unique_target_completion(
        &self,
        expected_authority: Qwen3PlanAuthority,
        expected_selection: Qwen3PlanSelection,
    )
        requires
            self.valid_for(expected_authority, expected_selection),
            expected_selection.role == Qwen3ModelRole::Target8B,
        ensures
            self.steps@.len() == QWEN3_TARGET_PLAN_STEPS as nat,
            self.steps@[543].operator == Qwen3Operator::ArgmaxCompactCompletion,
            forall|index: int| 0 <= index < 543
                ==> self.steps@[index].operator != Qwen3Operator::ArgmaxCompactCompletion,
    {
        self.expose_valid_steps(expected_authority, expected_selection);
        reveal(expected_step_spec);
        reveal(Qwen3PlanBucket::dimensions_spec);
        reveal(canonical_step_spec);
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Qwen3PlanError {
    MissingIdentity(&'static str),
    ZeroRevision,
    StaleIdentity(&'static str),
    StaleRevision,
    ModeBucketMismatch,
    SelectionMismatch,
    StepCount { expected: u32, actual: usize },
    StepMismatch { ordinal: u32 },
}

} // verus!

#[cfg(test)]
mod tests {
    use super::{
        expected_step, plan_step_count, Qwen3BufferKind, Qwen3ExecutionMode, Qwen3GeneratedPlan,
        Qwen3Operator, Qwen3PlanAuthority, Qwen3PlanBucket, Qwen3PlanError, Qwen3PlanSelection,
        QWEN3_DRAFT_PLAN_STEPS, QWEN3_TARGET_PLAN_STEPS,
    };
    use crate::{Identity, Qwen3ModelRole};

    const fn identity(byte: u8) -> Identity {
        Identity::new([byte; 32])
    }

    const fn authority() -> Qwen3PlanAuthority {
        Qwen3PlanAuthority {
            bundle_id: identity(1),
            model_id: identity(2),
            config_id: identity(3),
            graph_id: identity(4),
            plan_id: identity(5),
            revision: 7,
        }
    }

    fn canonical_plan(
        role: Qwen3ModelRole,
        mode: Qwen3ExecutionMode,
        bucket: Qwen3PlanBucket,
    ) -> Qwen3GeneratedPlan {
        let count = plan_step_count(role);
        let steps = (0..count)
            .map(|ordinal| expected_step(role, mode, bucket, ordinal).unwrap())
            .collect();
        Qwen3GeneratedPlan {
            authority: authority(),
            selection: Qwen3PlanSelection { role, mode, bucket },
            steps,
        }
    }

    #[test]
    fn exact_target_and_draft_graphs_validate() {
        let target = canonical_plan(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T512,
        );
        assert_eq!(target.steps.len(), QWEN3_TARGET_PLAN_STEPS as usize);
        assert_eq!(target.validate(authority(), target.selection), Ok(()));

        let draft = canonical_plan(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS8C8192,
        );
        assert_eq!(draft.steps.len(), QWEN3_DRAFT_PLAN_STEPS as usize);
        assert_eq!(draft.validate(authority(), draft.selection), Ok(()));
    }

    #[test]
    fn target_draft_heads_gqa_and_speculative_width_are_distinct() {
        let target = canonical_plan(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K8C8192,
        );
        let draft = canonical_plan(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K8C8192,
        );
        assert_eq!(target.steps[2].operator, Qwen3Operator::QueryProjection);
        assert_eq!(target.steps[2].geometry.query_heads, 32);
        assert_eq!(target.steps[2].geometry.gqa_group_size, 4);
        assert_eq!(target.steps[0].input_0.shape.dimension_1, 9);
        assert_eq!(target.steps[9].operator, Qwen3Operator::Attention);
        assert_eq!(target.steps[9].output_0.shape.dimension_2, 4_096);
        assert_eq!(
            target.steps[10].operator,
            Qwen3Operator::AttentionOutputResidual
        );
        assert_eq!(target.steps[10].input_0.shape.dimension_2, 4_096);
        assert_eq!(target.steps[10].output_0.shape.dimension_2, 4_096);
        assert_eq!(draft.steps[2].geometry.query_heads, 16);
        assert_eq!(draft.steps[2].geometry.gqa_group_size, 2);
        assert_eq!(draft.steps[0].input_0.shape.dimension_1, 8);
        assert_eq!(draft.steps[9].operator, Qwen3Operator::Attention);
        assert_eq!(draft.steps[9].output_0.shape.dimension_2, 2_048);
        assert_eq!(
            draft.steps[10].operator,
            Qwen3Operator::AttentionOutputResidual
        );
        assert_eq!(draft.steps[10].input_0.shape.dimension_2, 2_048);
        assert_eq!(draft.steps[10].output_0.shape.dimension_2, 1_024);
    }

    #[test]
    fn draft_attention_hidden_width_substitution_fails_closed() {
        let base = canonical_plan(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        );

        let mut wrong_attention_output = base.clone();
        wrong_attention_output.steps[9].output_0.shape.dimension_2 = 1_024;
        assert_eq!(
            wrong_attention_output.validate(authority(), wrong_attention_output.selection),
            Err(Qwen3PlanError::StepMismatch { ordinal: 9 })
        );

        let mut wrong_output_projection_input = base;
        wrong_output_projection_input.steps[10]
            .input_0
            .shape
            .dimension_2 = 1_024;
        assert_eq!(
            wrong_output_projection_input
                .validate(authority(), wrong_output_projection_input.selection),
            Err(Qwen3PlanError::StepMismatch { ordinal: 10 })
        );
    }

    #[test]
    fn omitted_extra_and_reordered_operators_fail_closed() {
        let mut omitted = canonical_plan(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        );
        omitted.steps.remove(8);
        assert!(matches!(
            omitted.validate(authority(), omitted.selection),
            Err(Qwen3PlanError::StepCount { .. })
        ));

        let mut extra = canonical_plan(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        );
        extra.steps.push(extra.steps[0]);
        assert!(matches!(
            extra.validate(authority(), extra.selection),
            Err(Qwen3PlanError::StepCount { .. })
        ));

        let mut reordered = canonical_plan(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T128,
        );
        reordered.steps.swap(2, 3);
        assert_eq!(
            reordered.validate(authority(), reordered.selection),
            Err(Qwen3PlanError::StepMismatch { ordinal: 2 })
        );
    }

    #[test]
    fn layer_head_gqa_buffer_and_shape_substitutions_fail_closed() {
        let base = canonical_plan(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS8C8192,
        );

        let mut wrong_layer = base.clone();
        wrong_layer.steps[1].layer = 1;
        assert_eq!(
            wrong_layer.validate(authority(), wrong_layer.selection),
            Err(Qwen3PlanError::StepMismatch { ordinal: 1 })
        );

        let mut wrong_heads = base.clone();
        wrong_heads.steps[2].geometry.query_heads = 16;
        assert_eq!(
            wrong_heads.validate(authority(), wrong_heads.selection),
            Err(Qwen3PlanError::StepMismatch { ordinal: 2 })
        );

        let mut wrong_gqa = base.clone();
        wrong_gqa.steps[2].geometry.gqa_group_size = 2;
        assert_eq!(
            wrong_gqa.validate(authority(), wrong_gqa.selection),
            Err(Qwen3PlanError::StepMismatch { ordinal: 2 })
        );

        let mut wrong_kv_heads = base.clone();
        wrong_kv_heads.steps[3].geometry.kv_heads = 4;
        assert_eq!(
            wrong_kv_heads.validate(authority(), wrong_kv_heads.selection),
            Err(Qwen3PlanError::StepMismatch { ordinal: 3 })
        );

        let mut wrong_buffer = base.clone();
        wrong_buffer.steps[9].input_1.kind = Qwen3BufferKind::KvValues;
        assert_eq!(
            wrong_buffer.validate(authority(), wrong_buffer.selection),
            Err(Qwen3PlanError::StepMismatch { ordinal: 9 })
        );

        let mut wrong_shape = base;
        wrong_shape.steps[2].output_0.shape.dimension_2 = 8;
        assert_eq!(
            wrong_shape.validate(authority(), wrong_shape.selection),
            Err(Qwen3PlanError::StepMismatch { ordinal: 2 })
        );
    }

    #[test]
    fn wrong_or_incompatible_mode_and_bucket_fail_before_steps() {
        let mut plan = canonical_plan(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS8T128,
        );
        let expected_selection = plan.selection;
        plan.selection.mode = Qwen3ExecutionMode::Decode;
        assert_eq!(
            plan.validate(authority(), expected_selection),
            Err(Qwen3PlanError::ModeBucketMismatch)
        );

        plan.selection.mode = Qwen3ExecutionMode::Prefill;
        plan.selection.bucket = Qwen3PlanBucket::PrefillS1T512;
        assert_eq!(
            plan.validate(authority(), expected_selection),
            Err(Qwen3PlanError::SelectionMismatch)
        );

        plan.selection.bucket = Qwen3PlanBucket::SpeculativeS8K4C8192;
        assert_eq!(
            plan.validate(authority(), plan.selection),
            Err(Qwen3PlanError::ModeBucketMismatch)
        );
    }

    #[test]
    fn absent_and_stale_plan_authority_fail_closed() {
        let plan = canonical_plan(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        );
        let expected_selection = plan.selection;
        let mut missing = authority();
        missing.graph_id = identity(0);
        assert_eq!(
            plan.validate(missing, expected_selection),
            Err(Qwen3PlanError::MissingIdentity("graph_id"))
        );

        let mut stale = authority();
        stale.plan_id = identity(9);
        assert_eq!(
            plan.validate(stale, expected_selection),
            Err(Qwen3PlanError::StaleIdentity("plan_id"))
        );

        let mut stale_revision = authority();
        stale_revision.revision += 1;
        assert_eq!(
            plan.validate(stale_revision, expected_selection),
            Err(Qwen3PlanError::StaleRevision)
        );

        let mut absent_plan_identity = plan;
        absent_plan_identity.authority.plan_id = identity(0);
        assert_eq!(
            absent_plan_identity.validate(authority(), expected_selection),
            Err(Qwen3PlanError::MissingIdentity("plan_id"))
        );
    }

    #[test]
    fn target_plan_cannot_be_relabelled_as_draft() {
        let mut plan = canonical_plan(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        );
        let expected_selection = plan.selection;
        plan.selection.role = Qwen3ModelRole::Draft06B;
        assert_eq!(
            plan.validate(authority(), expected_selection),
            Err(Qwen3PlanError::SelectionMismatch)
        );
    }
}
