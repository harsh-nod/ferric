#![forbid(unsafe_code)]

//! Generated, inert Qwen3 target/draft runner declarations for gfx942.
//!
//! Regenerate with `ferric_build::render_qwen3_gfx942_runner_source`. These
//! declarations are request-independent data. They authorize no artifact,
//! allocation, address, queue, load, launch, completion, hardware, proof,
//! performance, or qualification action.

use ferric_spec::{Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection};

/// Canonical generated runner declaration format.
pub const GENERATED_RUNNER_TEMPLATE_VERSION: u32 = 1;
/// Exact target processor named by the declaration template.
pub const GENERATED_RUNNER_PROCESSOR: &str = "gfx942";
/// Exact target features named by the declaration template.
pub const GENERATED_RUNNER_TARGET_FEATURES: &str = "+wavefrontsize64,-xnack";
/// Exact number of finite target/draft B3 plan declarations.
pub const GENERATED_RUNNER_PLAN_COUNT: usize = 22;
/// Exact number of ordered operation declarations across all plans.
pub const GENERATED_RUNNER_OPERATION_COUNT: usize = 10_648;

/// One exact plan position in the generated target-then-draft declaration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GeneratedPlanTemplate {
    /// Zero-based target-then-draft plan index.
    pub plan_index: u16,
    /// Exact role, execution mode, and finite B3 bucket.
    pub selection: Qwen3PlanSelection,
    /// First operation in the flattened declaration sequence.
    pub operation_start: u32,
    /// Exact operation count for the selected model role.
    pub operation_count: u32,
}

/// Logical scalar input whose value may vary between admitted requests.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunnerPatchKind {
    /// Input token identifiers.
    TokenIds,
    /// Input position identifiers.
    PositionIds,
    /// Per-sequence active-token lengths.
    ActiveLengths,
    /// Per-sequence committed-context lengths.
    ContextLengths,
}

/// Logical element type of a request-independent patch slot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunnerPatchScalarType {
    /// Unsigned 32-bit scalar.
    U32,
}

/// Logical extent of a request-independent patch slot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunnerPatchExtent {
    /// One scalar for each active token in the selected finite bucket.
    ActiveTokens,
    /// One scalar for each sequence in the selected finite bucket.
    Sequences,
}

/// One logical input schema entry, never a value, pointer, or device address.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RunnerPatchSlotTemplate {
    /// Stable zero-based schema position.
    pub slot_index: u16,
    /// Logical value supplied later by an independently checked runtime.
    pub kind: RunnerPatchKind,
    /// Exact scalar representation.
    pub scalar_type: RunnerPatchScalarType,
    /// Bucket-relative logical element count.
    pub extent: RunnerPatchExtent,
}

/// Complete exact plan roster in target-then-draft B3 order.
pub const GENERATED_PLAN_TEMPLATES: [GeneratedPlanTemplate; GENERATED_RUNNER_PLAN_COUNT] = [
    GeneratedPlanTemplate {
        plan_index: 0,
        selection: Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Prefill,
            bucket: Qwen3PlanBucket::PrefillS1T128,
        },
        operation_start: 0,
        operation_count: 544,
    },
    GeneratedPlanTemplate {
        plan_index: 1,
        selection: Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Prefill,
            bucket: Qwen3PlanBucket::PrefillS8T128,
        },
        operation_start: 544,
        operation_count: 544,
    },
    GeneratedPlanTemplate {
        plan_index: 2,
        selection: Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Prefill,
            bucket: Qwen3PlanBucket::PrefillS1T512,
        },
        operation_start: 1088,
        operation_count: 544,
    },
    GeneratedPlanTemplate {
        plan_index: 3,
        selection: Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Prefill,
            bucket: Qwen3PlanBucket::PrefillS1T2048,
        },
        operation_start: 1632,
        operation_count: 544,
    },
    GeneratedPlanTemplate {
        plan_index: 4,
        selection: Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Decode,
            bucket: Qwen3PlanBucket::DecodeS1C8192,
        },
        operation_start: 2176,
        operation_count: 544,
    },
    GeneratedPlanTemplate {
        plan_index: 5,
        selection: Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Decode,
            bucket: Qwen3PlanBucket::DecodeS8C8192,
        },
        operation_start: 2720,
        operation_count: 544,
    },
    GeneratedPlanTemplate {
        plan_index: 6,
        selection: Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Decode,
            bucket: Qwen3PlanBucket::DecodeS32C8192,
        },
        operation_start: 3264,
        operation_count: 544,
    },
    GeneratedPlanTemplate {
        plan_index: 7,
        selection: Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Speculative,
            bucket: Qwen3PlanBucket::SpeculativeS1K4C8192,
        },
        operation_start: 3808,
        operation_count: 544,
    },
    GeneratedPlanTemplate {
        plan_index: 8,
        selection: Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Speculative,
            bucket: Qwen3PlanBucket::SpeculativeS8K4C8192,
        },
        operation_start: 4352,
        operation_count: 544,
    },
    GeneratedPlanTemplate {
        plan_index: 9,
        selection: Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Speculative,
            bucket: Qwen3PlanBucket::SpeculativeS1K8C8192,
        },
        operation_start: 4896,
        operation_count: 544,
    },
    GeneratedPlanTemplate {
        plan_index: 10,
        selection: Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Speculative,
            bucket: Qwen3PlanBucket::SpeculativeS1K16C8192,
        },
        operation_start: 5440,
        operation_count: 544,
    },
    GeneratedPlanTemplate {
        plan_index: 11,
        selection: Qwen3PlanSelection {
            role: Qwen3ModelRole::Draft06B,
            mode: Qwen3ExecutionMode::Prefill,
            bucket: Qwen3PlanBucket::PrefillS1T128,
        },
        operation_start: 5984,
        operation_count: 424,
    },
    GeneratedPlanTemplate {
        plan_index: 12,
        selection: Qwen3PlanSelection {
            role: Qwen3ModelRole::Draft06B,
            mode: Qwen3ExecutionMode::Prefill,
            bucket: Qwen3PlanBucket::PrefillS8T128,
        },
        operation_start: 6408,
        operation_count: 424,
    },
    GeneratedPlanTemplate {
        plan_index: 13,
        selection: Qwen3PlanSelection {
            role: Qwen3ModelRole::Draft06B,
            mode: Qwen3ExecutionMode::Prefill,
            bucket: Qwen3PlanBucket::PrefillS1T512,
        },
        operation_start: 6832,
        operation_count: 424,
    },
    GeneratedPlanTemplate {
        plan_index: 14,
        selection: Qwen3PlanSelection {
            role: Qwen3ModelRole::Draft06B,
            mode: Qwen3ExecutionMode::Prefill,
            bucket: Qwen3PlanBucket::PrefillS1T2048,
        },
        operation_start: 7256,
        operation_count: 424,
    },
    GeneratedPlanTemplate {
        plan_index: 15,
        selection: Qwen3PlanSelection {
            role: Qwen3ModelRole::Draft06B,
            mode: Qwen3ExecutionMode::Decode,
            bucket: Qwen3PlanBucket::DecodeS1C8192,
        },
        operation_start: 7680,
        operation_count: 424,
    },
    GeneratedPlanTemplate {
        plan_index: 16,
        selection: Qwen3PlanSelection {
            role: Qwen3ModelRole::Draft06B,
            mode: Qwen3ExecutionMode::Decode,
            bucket: Qwen3PlanBucket::DecodeS8C8192,
        },
        operation_start: 8104,
        operation_count: 424,
    },
    GeneratedPlanTemplate {
        plan_index: 17,
        selection: Qwen3PlanSelection {
            role: Qwen3ModelRole::Draft06B,
            mode: Qwen3ExecutionMode::Decode,
            bucket: Qwen3PlanBucket::DecodeS32C8192,
        },
        operation_start: 8528,
        operation_count: 424,
    },
    GeneratedPlanTemplate {
        plan_index: 18,
        selection: Qwen3PlanSelection {
            role: Qwen3ModelRole::Draft06B,
            mode: Qwen3ExecutionMode::Speculative,
            bucket: Qwen3PlanBucket::SpeculativeS1K4C8192,
        },
        operation_start: 8952,
        operation_count: 424,
    },
    GeneratedPlanTemplate {
        plan_index: 19,
        selection: Qwen3PlanSelection {
            role: Qwen3ModelRole::Draft06B,
            mode: Qwen3ExecutionMode::Speculative,
            bucket: Qwen3PlanBucket::SpeculativeS8K4C8192,
        },
        operation_start: 9376,
        operation_count: 424,
    },
    GeneratedPlanTemplate {
        plan_index: 20,
        selection: Qwen3PlanSelection {
            role: Qwen3ModelRole::Draft06B,
            mode: Qwen3ExecutionMode::Speculative,
            bucket: Qwen3PlanBucket::SpeculativeS1K8C8192,
        },
        operation_start: 9800,
        operation_count: 424,
    },
    GeneratedPlanTemplate {
        plan_index: 21,
        selection: Qwen3PlanSelection {
            role: Qwen3ModelRole::Draft06B,
            mode: Qwen3ExecutionMode::Speculative,
            bucket: Qwen3PlanBucket::SpeculativeS1K16C8192,
        },
        operation_start: 10224,
        operation_count: 424,
    },
];

/// Complete request-independent logical input schema.
pub const GENERATED_PATCH_SLOTS: [RunnerPatchSlotTemplate; 4] = [
    RunnerPatchSlotTemplate {
        slot_index: 0,
        kind: RunnerPatchKind::TokenIds,
        scalar_type: RunnerPatchScalarType::U32,
        extent: RunnerPatchExtent::ActiveTokens,
    },
    RunnerPatchSlotTemplate {
        slot_index: 1,
        kind: RunnerPatchKind::PositionIds,
        scalar_type: RunnerPatchScalarType::U32,
        extent: RunnerPatchExtent::ActiveTokens,
    },
    RunnerPatchSlotTemplate {
        slot_index: 2,
        kind: RunnerPatchKind::ActiveLengths,
        scalar_type: RunnerPatchScalarType::U32,
        extent: RunnerPatchExtent::Sequences,
    },
    RunnerPatchSlotTemplate {
        slot_index: 3,
        kind: RunnerPatchKind::ContextLengths,
        scalar_type: RunnerPatchScalarType::U32,
        extent: RunnerPatchExtent::Sequences,
    },
];
