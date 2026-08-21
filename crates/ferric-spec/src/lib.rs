#![forbid(unsafe_code)]

//! Executable sequential semantics used as Ferric's refinement target.
//!
//! This crate is not a serving fallback. Production runners must refine these
//! semantics and do not call them in the inference hot path.

#[allow(unused_imports)]
use vstd::prelude::*;

pub mod completion;
mod configuration;
pub mod continuous_batching;
mod graph;
mod identity;
mod m1_completion;
pub mod paged_kv_refinement;
mod qwen3;
pub mod scheduling;
mod speculation;
pub mod speculative_completion;
pub mod step_plan_publication;

pub use configuration::{
    DeploymentBundle, EngineLimits, ModelArtifact, ModelConfig, NumericalPolicy, Qwen3ModelRole,
    SpecError, Target, TokenizerConfig, WeightManifest, M1_MAX_ACTIVE_SEQUENCES,
    M1_MAX_CONTEXT_TOKENS, M1_MAX_DRAFT_TOKENS, M1_MAX_KV_PAGE_TOKENS, M1_MAX_WEIGHT_BYTES,
    M1_MAX_WEIGHT_SECTIONS, QWEN3_END_OF_TEXT_TOKEN, QWEN3_IM_END_TOKEN, QWEN3_IM_START_TOKEN,
    QWEN3_VOCABULARY_SIZE,
};
pub use continuous_batching::{
    apply_continuous_batch_step, ContinuousBatch, ContinuousBatchAction, ContinuousBatchError,
    ContinuousRequest, M1_CONTINUOUS_BATCH_CAPACITY,
};
pub use graph::{
    expected_step, geometry, plan_step_count, Qwen3BufferKind, Qwen3ExecutionMode,
    Qwen3GeneratedPlan, Qwen3Operator, Qwen3PlanAuthority, Qwen3PlanBucket, Qwen3PlanBuffer,
    Qwen3PlanDimensions, Qwen3PlanError, Qwen3PlanGeometry, Qwen3PlanSelection, Qwen3PlanShape,
    Qwen3PlanStep, QWEN3_DRAFT_PLAN_STEPS, QWEN3_LAYER_PLAN_STEPS, QWEN3_TARGET_PLAN_STEPS,
};
pub use identity::{Identity, RequestId};
pub use m1_completion::{
    select_lowest_argmax, validate_compact_completion, CompactCompletionError,
    CompactCompletionRecord, M1_MAX_COMPLETION_TOKENS,
};
pub use paged_kv_refinement::{
    append_physical_page, cancel_physical_kv, commit_physical_kv, map_initialized_token,
    release_retired_page, retire_cancelled_tail, rollback_physical_token, write_physical_token,
    LogicalKvState, PhysicalKvError, PhysicalKvLifecycle, PhysicalKvLocation, PhysicalKvState,
    PhysicalPageId, M1_KV_PAGE_TABLE_ENTRIES, M1_KV_PAGE_TOKENS, M1_KV_PHYSICAL_PAGE_SLOTS,
};
pub use qwen3::{
    Qwen3TensorError, Qwen3TensorKind, Qwen3TensorMetadata, TensorDType, QWEN3_DRAFT_TENSOR_COUNT,
    QWEN3_DRAFT_TENSOR_DATA_BYTES, QWEN3_NO_LAYER, QWEN3_TARGET_TENSOR_COUNT,
    QWEN3_TARGET_TENSOR_DATA_BYTES, QWEN3_TENSORS_PER_LAYER,
};
pub use speculation::{verify_greedy_round, GreedyCommit, GreedyVerificationError, TokenId};
pub use speculative_completion::{verify_speculative_completion, SpeculativeCompletionError};
pub use step_plan_publication::{
    discard_reserved_delta, publish_reserved_delta, validate_direct_publication,
    validate_speculative_publication, PublicationPhase, ReservedStateDelta, StepPlan,
    StepPublication, StepPublicationError,
};
