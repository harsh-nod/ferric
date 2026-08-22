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
pub mod m1_foundation_theorems;
mod m1_step_inputs;
pub mod paged_kv_refinement;
pub mod physical_plan;
mod qwen3;
pub mod request_isolation;
pub mod scheduling;
mod speculation;
pub mod speculative_completion;
pub mod speculative_kv_indexing;
pub mod speculative_step_composition;
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
pub use m1_step_inputs::{
    validate_m1_step_inputs, M1StepInputCandidate, M1StepInputError, M1StepInputParts,
    M1StepInputRejection, M1StepInputValidationOutcome, ValidatedM1StepInputs,
};
pub use paged_kv_refinement::{
    append_physical_page, cancel_physical_kv, commit_physical_kv, map_initialized_token,
    release_retired_page, retire_cancelled_tail, rollback_physical_token, write_physical_token,
    LogicalKvState, PhysicalKvError, PhysicalKvLifecycle, PhysicalKvLocation, PhysicalKvState,
    PhysicalPageId, M1_KV_PAGE_TABLE_ENTRIES, M1_KV_PAGE_TOKENS, M1_KV_PHYSICAL_PAGE_SLOTS,
};
pub use physical_plan::{
    physical_plan_structural_validation_theorem, validate_physical_plan_declaration,
    DeclaredFusionRefinementPremise, PhysicalCapacityExpectation, PhysicalCapacitySource,
    PhysicalCompletionDeclaration, PhysicalIdentityRole, PhysicalPacketIdentityBinding,
    PhysicalPacketSpanDeclaration, PhysicalPlanDeclaration, PhysicalPlanError,
    PhysicalPlanExpectation, PhysicalPublicationDeclaration, StructurallyValidatedPhysicalPlan,
    M1_MAX_DECLARED_RING_PACKETS_V1, M1_MAX_UNTRUSTED_PACKET_CAPACITY_V1,
    M1_MIN_DECLARED_RING_PACKETS_V1, M1_PHYSICAL_PLAN_DECLARATION_VERSION,
    M1_REVIEWED_BATCH_PACKET_CAPACITY_V1, M1_REVIEWED_BATCH_PACKET_CAPACITY_V2,
};
pub use qwen3::{
    Qwen3TensorError, Qwen3TensorKind, Qwen3TensorMetadata, TensorDType, QWEN3_DRAFT_TENSOR_COUNT,
    QWEN3_DRAFT_TENSOR_DATA_BYTES, QWEN3_NO_LAYER, QWEN3_TARGET_TENSOR_COUNT,
    QWEN3_TARGET_TENSOR_DATA_BYTES, QWEN3_TENSORS_PER_LAYER,
};
pub use request_isolation::{
    apply_isolated_kv_action, apply_isolated_scheduler_step, cancel_isolated_request,
    detach_isolated_request, map_isolated_token, release_isolated_page,
    settle_isolated_speculative_kv, IsolatedKvAction, IsolatedRequestKv, IsolatedRequestProjection,
    IsolatedSchedulerAction, IsolatedSpeculativeKvExpectation, IsolatedSpeculativeKvSettlement,
    RequestIsolationError,
};
pub use speculation::{verify_greedy_round, GreedyCommit, GreedyVerificationError, TokenId};
pub use speculative_completion::{verify_speculative_completion, SpeculativeCompletionError};
pub use speculative_kv_indexing::{
    CorrectionBonusKvDisposition, SpeculativeKvIndexError, SpeculativeKvInputBinding,
    SpeculativeKvInputSource, SpeculativeKvInterval, SpeculativeKvRoundIndex, TargetChoiceBinding,
    TargetChoiceUse, M1_MAX_SPECULATIVE_KV_DRAFT_TOKENS, M1_MAX_SPECULATIVE_KV_TARGET_INPUTS,
};
pub use speculative_step_composition::{
    apply_preflighted_speculative_step, preflight_speculative_step,
    required_single_member_accepted_count, settle_and_publish_speculative_step,
    AtomicSpeculativeStepError, AtomicSpeculativeStepOutcome, SpeculativeStepPreflight,
};
pub use step_plan_publication::{
    discard_reserved_delta, publish_reserved_delta, validate_direct_publication,
    validate_speculative_publication, PublicationPhase, ReservedStateDelta, SpeculativeTokenInputs,
    StepPlan, StepPublication, StepPublicationError,
};

verus! {

/// Cross-crate verifier view of the exact finite Qwen3 graph-step lookup.
pub open spec fn canonical_expected_step_spec(
    role: Qwen3ModelRole,
    mode: Qwen3ExecutionMode,
    bucket: Qwen3PlanBucket,
    ordinal: u32,
) -> Option<Qwen3PlanStep> {
    graph::expected_step_spec(role, mode, bucket, ordinal)
}

/// Cross-crate verifier view of exact logical M1 step-input validity.
pub open spec fn m1_step_input_candidate_valid_spec(
    candidate: &M1StepInputCandidate,
) -> bool {
    m1_step_inputs::m1_step_input_candidate_valid(candidate)
}

/// Cross-crate verifier view of the exact structural rejection relation.
pub open spec fn m1_step_input_error_matches_spec(
    error: M1StepInputError,
    candidate: &M1StepInputCandidate,
) -> bool {
    m1_step_inputs::m1_step_input_error_matches(error, candidate)
}

} // verus!
