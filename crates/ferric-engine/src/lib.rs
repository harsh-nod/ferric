#![forbid(unsafe_code)]

//! Safe state machines used by the generated Ferric runtime.

#[allow(unused_imports)]
use vstd::prelude::*;

mod bound_step_workspaces;
mod cache;
mod completed_readback_join;
mod completion_output;
mod completion_wire;
mod device_cache;
mod epoch;
mod initialized_model_memory;
mod initialized_step_workspaces;
mod kv_workspace_authority;
mod m1_completed_step;
mod m1_completed_step_release;
mod m1_prepublication;
mod m1_queue_rearm;
mod model_memory_allocations;
mod observed_completion;
mod operation_dispatch_expansion;
mod operation_kernel_plan;
mod persisted_kernel_artifacts;
mod physical_buffer_bindings;
mod physical_buffer_recipe;
mod physical_device;
mod physical_dispatch_recipe;
mod physical_fixed_batch;
mod physical_kernarg_recipe;
mod physical_program_catalog;
mod physical_queue_lifecycle;
mod physical_step;
mod qualification_logits;
mod runner;
mod scheduler;
mod speculative_graph;
mod step_dispatch_composition;
mod step_workspace_composition;
mod step_workspace_images;
mod step_workspace_subleases;
mod system;

pub use bound_step_workspaces::{
    BoundM1FullStepWorkspaceSubleases, M1FullStepWorkspaceDispatchRangeError,
    M1FullStepWorkspaceSubleaseBindingError, M1FullStepWorkspaceSubleaseBindingFailure,
    M1FullStepWorkspaceSubleaseOwners,
};
pub use cache::{KvError, PageId};
pub use completed_readback_join::{M1CheckedCompletionOutputV1, M1CompletedOutputCheckErrorV1};
pub use completion_output::{
    allocate_m1_completion_output_v1, m1_completion_output_shape_v1, BoundM1CompletionOutputV1,
    M1CompletionOutputErrorV1, M1CompletionOutputShapeV1, M1_COMPLETION_OUTPUT_ALIGNMENT_V1,
};
pub use completion_wire::{
    bind_inert_completion_epoch, check_inert_completion_record,
    validate_m1_qualification_context_plan_v1, CheckedCompletionSemantics,
    CompletionEpochJoinFailure, CompletionWireError, CompletionWireExpectation,
    CompletionWireSemanticExpectation, EpochJoinedCompletionRecord, InertCheckedCompletionRecord,
    M1QualificationContextStepWitnessErrorV1, M1ValidatedQualificationContextPlanV1,
    M1ValidatedQualificationContextStepV1,
};
pub use device_cache::{
    bind_m1_partitioned_model_memory_kv_pool_v1, prelease_m1_qualification_target_pages_v1,
    AbortedDeviceKvStepWrite, ActiveDeviceKvCache, CancelledDeviceKvCache, DeviceKvAppendFailure,
    DeviceKvCacheError, DeviceKvCacheProjection, DeviceKvCancellationFailure,
    DeviceKvCancellationOutcome, DeviceKvPageLease, DeviceKvReadBinding, DeviceKvRetirementOutcome,
    DeviceKvStepAbortFailure, DeviceKvStepPageBinding, DeviceKvStepPageIdentity,
    DeviceKvStepReservationFailure, Gfx942DeviceBinding, InitializedDeviceKvWrite,
    M1DeviceKvArenaLeaseBindingFailureV1, M1DeviceKvArenaLeaseErrorV1,
    M1DeviceKvArenaLeaseRecoveryPhaseV1, M1DeviceKvArenaLeaseRecoveryV1,
    M1PartitionedModelMemoryKvPoolV1, M1PartitionedModelMemoryKvQueueCustodyV1,
    M1PendingQualificationContextStepWriteV1, M1QualificationContextStepAbortFailureV1,
    M1QualificationContextStepReservationFailureV1, M1QualificationTargetPagePreleaseErrorV1,
    M1QualificationTargetPagePreleaseFailureV1, M1QualificationTargetPagePreleaseProgressV1,
    M1QualificationTargetPagePreleaseRecoveryV1, M1QualificationTargetPagePreleaseSuccessV1,
    M1QualificationTargetPageReserveV1, M1TargetPartitionedKvQuarantineV1,
    M1UnpartitionedModelMemoryKvRecoveryV1, PendingDeviceKvStepWrite, PendingDeviceKvWrite,
    PendingSpeculativeDraftKvRoundWrite, PendingWriteCompletionFailure, PoisonedDeviceKvCache,
    QuiescenceFailure, QuiescentDeviceKvCache, RetirementCompletionFailure,
    SettledQuiescentDeviceKvCache, WriteApplicationFailure, GFX942_PROCESSOR,
    GFX942_TARGET_FEATURES, M1_QUALIFICATION_TARGET_PAGE_COUNT_V1,
};
pub use epoch::ExactCompletion;
pub use initialized_model_memory::{
    allocate_initialized_model_memory_v1, m1_model_memory_content_descriptor_v1,
    m1_model_memory_content_role_v1, InitializedModelMemoryAllocationErrorV1,
    InitializedModelMemoryAllocationFailureV1, InitializedModelMemoryPreflightErrorV1,
    M1_INITIALIZED_MODEL_MEMORY_CONTENT_ROLE_IDENTITY_V1,
};
pub use initialized_step_workspaces::{
    m1_step_workspace_content_descriptor_v1, m1_step_workspace_content_role_v1,
    InitializedM1FullStepWorkspaceAllocationErrorV1,
    InitializedM1FullStepWorkspaceAllocationFailureV1,
    InitializedM1FullStepWorkspacePreflightErrorV1, InitializedM1FullStepWorkspaceRuntimeErrorV1,
    M1FullStepWorkspaceImagesV1, M1InitializedWorkspaceSlotV1,
    M1_INITIALIZED_STEP_WORKSPACE_CONTENT_ROLE_IDENTITY_V1,
};
pub use kv_workspace_authority::{
    bind_m1_kv_workspace_table_v1, bind_m1_speculative_draft_kv_round_workspace_table_v1,
    BoundM1KvWorkspaceTableV1, BoundM1SpeculativeDraftKvRoundWorkspaceTableV1,
    M1KvWorkspaceReservationCustodyV1, M1KvWorkspaceTableBindingErrorV1,
    M1KvWorkspaceTableBindingFailureV1, M1SpeculativeDraftKvRoundBindingErrorV1,
    M1SpeculativeDraftKvRoundBindingFailureV1, M1SpeculativeDraftKvRoundReservationCustodyV1,
};
pub use m1_completed_step::{
    complete_m1_physical_step_v1, M1CompletedDeviceKvMemberV1, M1CompletedStepErrorV1,
    M1CompletedStepOutcomeV1, M1CompletedStepPoisonV1, M1CompletedStepRejectionV1,
    M1CompletedStepSuccessV1, M1DeviceKvCompletionDispositionV1, M1DeviceKvCompletionMemberV1,
    M1DeviceKvCompletionRosterV1,
};
pub use m1_completed_step_release::{
    release_m1_completed_step_kv_pages_v1, M1CompletedKvPageIdentityErrorV1,
    M1CompletedKvPageReleaseCountsV1, M1CompletedStepKvReleaseErrorV1,
    M1CompletedStepKvReleaseFailureV1, M1ReleasedCompletedStepV1, M1ReleasedDeviceKvMemberV1,
    M1ReleasedQueueTeardownFailureV1, M1ReleasedQueueTeardownSuccessV1,
    M1ReleasedTerminalDeviceKvMemberV1,
};
pub use m1_prepublication::{
    allocate_m1_prepublication_workspaces_v1, build_m1_prepublication_batch_v1,
    prepare_m1_scheduled_workspace_images_v1, M1AllocatedScheduledStepV1,
    M1FullStepKvReservationCustodyV1, M1FullStepKvWorkspaceTablesV1, M1PrepareFailureV1,
    M1PreparedScheduledWorkspaceImagesV1, M1PrepublicationAllocationFailureV1,
    M1PrepublicationBatchBuildDiagnosticV1, M1PrepublicationBatchBuildErrorKindV1,
    M1PrepublicationBatchBuildFailureV1, M1PrepublicationBatchV1,
    M1PrepublicationCompositionFailureV1, M1PrepublicationJoinErrorV1,
    M1PrepublicationJoinFailureV1, M1PrepublicationStepCustodyV1, M1WorkspaceImageResidueV1,
};
pub use m1_queue_rearm::{
    prepare_m1_long_lived_queue_rearm_v1, reserve_m1_long_lived_queue_rearm_kv_v1,
    schedule_m1_long_lived_queue_rearm_v1, submit_m1_long_lived_queue_rearm_v1,
    M1LongLivedQueueRearmKvInputsV1, M1LongLivedQueueRearmKvReservationFailureV1,
    M1LongLivedQueueRearmKvReservationPhaseV1, M1LongLivedQueueRearmPrepareFailureV1,
    M1LongLivedQueueRearmProgressPhaseV1, M1LongLivedQueueRearmScheduleErrorV1,
    M1LongLivedQueueRearmScheduleFailureV1, M1LongLivedQueueRearmSchedulePhaseV1,
    M1LongLivedQueueRearmSubmissionFailureV1, M1LongLivedQueueRearmSubmissionPhaseV1,
    M1LongLivedQueueRearmTeardownFailureV1, M1LongLivedQueueRearmTeardownSuccessV1,
    M1LongLivedQueueReleasedRoundV1, M1LongLivedQueueUnscheduledRoundV1,
    M1PreparedLongLivedQueueRearmV1, M1RearmedCompletedQueueV1, M1RearmedCompletedReadbackV1,
    M1RearmedCompletionOutcomeV1, M1RearmedCompletionPreflightErrorV1,
    M1RearmedCompletionPreflightFailureV1, M1RearmedObservedQualificationOutputV1,
    M1RearmedPublishedQueueV1, M1RearmedQualificationCompletedReadbackJoinFailureV1,
    M1RearmedQualificationObservationFailureV1, M1RearmedQualificationObservationTeardownFailureV1,
    M1RearmedQualificationObservationTeardownSuccessV1,
    M1RearmedQualificationSemanticTeardownFailureV1,
    M1RearmedQualificationSemanticTeardownSuccessV1, M1RearmedQualifiedCompletedReadbackV1,
    M1RearmedQualifiedCompletionOutcomeV1, M1RearmedQualifiedCompletionPreflightFailureV1,
    M1RearmedQualifiedReleasedRoundV1, M1RearmedQualifiedRoundPageReleaseFailureV1,
    M1RearmedQualifiedRoundReleaseOutcomeV1, M1RearmedQualifiedTeardownFailureV1,
    M1RearmedQualifiedTeardownSuccessV1, M1RearmedQueueProgressFailureV1,
    M1RearmedReadbackFailureSourceV1, M1RearmedReadbackFailureV1, M1RearmedRecycledQueueV1,
    M1RearmedRoundPageReleaseFailureV1, M1RearmedRoundReleaseOutcomeV1,
    M1ReservedLongLivedQueueRearmV1, M1ScheduledLongLivedQueueRearmV1,
};
pub use model_memory_allocations::{
    bind_addressless_model_memory_allocations_v1, BoundModelMemoryAllocationsV1,
    ModelMemoryAllocationBindingErrorV1, ModelMemoryAllocationBindingFailureV1,
    ModelMemoryDispatchRangeErrorV1, SelectedModelMemoryAllocationIdentitiesV1,
};
pub use observed_completion::{
    M1ObservedCompletionImageErrorV1, M1ObservedCompletionImageV1, M1ObservedCompletionRecordV1,
};
pub use operation_dispatch_expansion::{
    derive_m1_operation_dispatch_expansion, plan_m1_operation_dispatch_expansion,
    AddresslessM1OperationDispatchPlan, DeclaredM1OperationDispatchExpansion,
    M1OperationDispatchExpansionError, M1OperationDispatchExpansionFailure,
    M1OperationDispatchExpansionOutcome, M1OperationDispatchIdentityComponent,
    M1OperationDispatchKind, M1OperationDispatchRow, M1_MAX_OPERATION_DISPATCHES_V1,
    M1_OPERATION_DISPATCH_EXPANSION_VERSION,
};
pub use operation_kernel_plan::{
    bind_declared_operation_kernel_plan, DeclaredKernelFamilyArtifact, DeclaredOperationIdentity,
    DeclaredOperationKernelBinding, DeclaredOperationKernelPlan, OperationKernelIdentityComponent,
    OperationKernelPlanError, OperationKernelPlanFailure, OperationKernelPlanOutcome,
};
pub use persisted_kernel_artifacts::{
    reopen_persisted_m1_kernel_artifacts_v1, AdmittedPersistedM1KernelArtifactsV1,
    M1PersistedKernelArtifactFileV1, M1PersistedKernelArtifactOpenErrorV1,
};
pub use physical_buffer_bindings::{
    bind_m1_physical_buffer_ranges_v1, BoundM1PhysicalBufferBindingsV1, M1BoundPhysicalBufferRowV1,
    M1PhysicalBufferBindingErrorV1, M1PhysicalBufferBindingFailureV1,
    M1_PHYSICAL_BUFFER_BINDING_VERSION_V1,
};
pub use physical_buffer_recipe::{
    derive_m1_physical_buffer_recipe_v1, AddresslessM1PhysicalBufferRecipeV1,
    M1PhysicalBufferAccessV1, M1PhysicalBufferRecipeErrorV1, M1PhysicalBufferRecipeFailureV1,
    M1PhysicalBufferRecipeRowV1, M1PhysicalBufferSentinelV1, M1PhysicalBufferSourceV1,
    M1PhysicalExplicitBufferV1, M1_PHYSICAL_BUFFER_RECIPE_VERSION_V1,
};
pub use physical_device::{
    acquire_m1_checked_gfx942_service_device_v1, allocate_initialized_m1_model_memory_on_device_v1,
    M1CheckedGfx942ServiceDeviceAcquireFailureV1, M1CheckedGfx942ServiceDeviceV1,
    M1DeviceBoundModelMemoryV1, M1DeviceModelMemoryAllocationFailureClassV1,
    M1DeviceModelMemoryAllocationFailureV1, M1DeviceModelMemoryFailureReleaseFailureV1,
    M1DeviceModelMemoryFailureReleaseObservationV1, M1UnpublishedAllocationReleaseFailureV1,
    M1UnpublishedAllocationReleaseObservationV1,
};
pub use physical_dispatch_recipe::{
    derive_m1_physical_dispatch_recipe_v1, AddresslessM1PhysicalDispatchRecipeV1,
    M1PhysicalDispatchKindV1, M1PhysicalDispatchProfileV1, M1PhysicalDispatchRecipeErrorV1,
    M1PhysicalDispatchRecipeRowV1, M1PhysicalProfileFamilyV1,
    M1_PHYSICAL_DISPATCH_RECIPE_VERSION_V1,
};
pub use physical_fixed_batch::{
    build_m1_physical_fixed_batch_v1, M1PhysicalFixedBatchBuildErrorV1,
    M1PhysicalFixedBatchBuildFailureV1, M1PhysicalFixedBatchCaseV1, M1PhysicalFixedBatchCustodyV1,
    M1PhysicalFixedBatchRowSetV1, M1PhysicalFixedBatchShapeV1, M1PhysicalFixedBatchV1,
    M1PhysicalQueueBatchCustodyV1, M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1,
    M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1, M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1,
    M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1, M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1,
};
pub use physical_kernarg_recipe::{
    derive_m1_physical_kernarg_recipe_v1, AddresslessM1PhysicalKernargRecipeV1,
    M1PhysicalKernargImageV1, M1PhysicalKernargRecipeErrorV1, M1PhysicalKernargRecipeFailureV1,
    M1_COV6_HIDDEN_KERNARG_BYTES_V1, M1_PHYSICAL_KERNARG_RECIPE_VERSION_V1,
};
pub use physical_program_catalog::{
    bind_content_bound_m1_program_catalog_v1, ContentBoundM1ProgramCatalogV1,
    InspectedM1KernelArtifacts, M1PhysicalProgramCatalogErrorV1, M1PhysicalProgramFamilyV1,
    M1PhysicalProgramV1, M1_PHYSICAL_PROGRAM_COUNT_V1,
};
pub use physical_queue_lifecycle::{
    M1CompletedReadbackJoinErrorV1, M1CompletedReadbackJoinFailureV1,
    M1CompletionObservationErrorV1, M1CompletionObservationFailureCustodyV1,
    M1CompletionObservationFailureV1, M1ObservedCompletionCaseV1, M1ObservedCompletionOutputV1,
    M1ObservedQualificationOutputV1, M1PhysicalCompletedQueueSessionV1,
    M1PhysicalCompletedReadbackV1, M1PhysicalDetachedQueueCaseV1, M1PhysicalDetachedQueueSessionV1,
    M1PhysicalPublishedQueueSessionV1, M1PhysicalQueueCreateFailureClassV1,
    M1PhysicalQueueCreateFailureV1, M1PhysicalQueueOperationFailureV1, M1PhysicalQueuePhaseCaseV1,
    M1PhysicalQueuePhaseV1, M1PhysicalQueueReleaseFailureV1, M1PhysicalQueueSessionV1,
    M1PhysicalReadbackDetachedQueueCaseV1, M1PhysicalReadbackDetachedQueueSessionV1,
    M1PhysicalReadbackQueueCaseV1, M1PhysicalReadbackQueueOperationFailureV1,
    M1PhysicalReadbackQueueReleaseFailureV1, M1PhysicalReadbackQueueSessionV1,
    M1PhysicalRecycledQueueSessionV1, M1QualificationCompletedReadbackJoinFailureV1,
    M1QualificationCompletionEvidenceV1, M1QualificationObservationErrorV1,
    M1QualificationObservationFailureCustodyV1, M1QualificationObservationFailureV1,
    M1QualifiedPhysicalCompletedReadbackV1, M1RejectedCompletionCaseV1,
    M1RejectedCompletionOutputV1,
};
pub use physical_step::{
    bind_structural_physical_step, StructuralPhysicalStepBindingError,
    StructuralPhysicalStepBindingFailure, StructuralPhysicalStepBindingOutcome,
    StructurallyBoundPhysicalStep,
};
pub use qualification_logits::{
    m1_qualification_logits_shape_v1, BoundM1QualificationLogitsV1,
    M1ObservedQualificationLogitsRowV1, M1ObservedQualificationLogitsV1,
    M1QualificationFinalLogitsErrorV1, M1QualificationLogitsAllocationFailureV1,
    M1QualificationLogitsErrorV1, M1QualificationLogitsShapeV1,
    M1_QUALIFICATION_LOGITS_ALIGNMENT_V1, M1_QUALIFICATION_LOGITS_ELEMENT_BYTES_V1,
};
pub use runner::{
    bind_m1_physical_runner_v1, initialize_m1_physical_runner_memory_v1, LogicalRunnerDeclaration,
    LogicalRunnerError, M1PhysicalRunnerBindFailureV1, M1PhysicalRunnerFirstCompletionOutcomeV1,
    M1PhysicalRunnerFirstPublicationFailureV1, M1PhysicalRunnerMemoryFailureV1,
    M1PhysicalRunnerQueueFailureStageV1, M1PhysicalRunnerRearmSubmissionFailureV1,
    M1PhysicalRunnerRecipeFailureV1, M1PhysicalRunnerRecipeOutcomeV1, M1PhysicalRunnerV1,
};
pub use scheduler::{DispatchBatch, M1ScheduledDispatchV1, SchedulerError};
pub use speculative_graph::{
    complete_single_member_speculative_graph, SingleMemberSpeculativeGraphError,
    SingleMemberSpeculativeGraphFailure, SingleMemberSpeculativeGraphInputs,
    SingleMemberSpeculativeGraphOutcome,
};
pub use step_dispatch_composition::{
    derive_m1_step_dispatch_plan, AddresslessM1StepDispatchPlan, M1StepDispatchCompositionError,
    M1StepDispatchDependency, M1StepDispatchIntent, M1StepDispatchSegment, M1StepDispatchStage,
    M1_MAX_STEP_DISPATCHES_V1, M1_STEP_DISPATCH_COMPOSITION_VERSION,
};
pub use step_workspace_composition::{
    compose_addressless_m1_full_step_workspaces, AddresslessM1FullStepWorkspaceComposition,
    M1FullStepWorkspaceCompositionError, M1FullStepWorkspaceCompositionFailure,
    M1FullStepWorkspaceCompositionOutcome, M1FullStepWorkspaceInputKind, M1FullStepWorkspacePlans,
    M1FullStepWorkspaceRole, M1FullStepWorkspaceSegmentBinding, M1SpeculativeDraftChoiceSubrange,
    M1SpeculativeDraftMetadataSubrange,
};
pub use step_workspace_images::{
    compose_m1_step_workspace_image_v1, ComposedM1FullStepWorkspaceSetV1,
    ComposedM1StepWorkspaceImageV1, M1StepWorkspaceImageCompositionErrorV1,
    M1StepWorkspaceImageCompositionFailureV1, M1StepWorkspaceImageCompositionOutcomeV1,
    M1_KV_PAGE_TABLE_ENTRIES_PER_SEQUENCE_V1, M1_KV_PHYSICAL_PAGE_SLOTS_V1,
};
pub use step_workspace_subleases::{
    bind_addressless_m1_step_workspace_subleases, BoundM1StepWorkspaceSubleases,
    M1StepWorkspaceDispatchRangeError, M1StepWorkspaceSubleaseBindingError,
    M1StepWorkspaceSubleaseBindingFailure, M1_DRAFT_STEP_WORKSPACE_SUBLEASE_COUNT_V1,
    M1_TARGET_SPECULATIVE_STEP_WORKSPACE_SUBLEASE_COUNT_V1,
    M1_TARGET_STEP_WORKSPACE_SUBLEASE_COUNT_V1,
};
pub use system::{CompletionFailure, Engine, EngineError};

verus! {

/// Cross-crate verifier view of one generated operation's exact addressless
/// dispatch row kinds.
pub open spec fn m1_operation_dispatch_kinds_spec(
    role: ferric_spec::Qwen3ModelRole,
    operator: ferric_spec::Qwen3Operator,
) -> Seq<nat> {
    operation_dispatch_expansion::m1_operation_dispatch_kinds_spec(role, operator)
}

/// Cross-crate verifier view of target operation-expansion cardinality.
pub open spec fn m1_target_operation_dispatch_count_spec(
    logical_operation_count: nat,
    target_completion_operation_count: nat,
) -> nat {
    operation_dispatch_expansion::m1_target_operation_dispatch_count_spec(
        logical_operation_count,
        target_completion_operation_count,
    )
}

/// Exposes the exact reviewed target compact-completion expansion.
pub proof fn m1_target_completion_dispatch_shape()
    ensures
        m1_operation_dispatch_kinds_spec(
            ferric_spec::Qwen3ModelRole::Target8B,
            ferric_spec::Qwen3Operator::ArgmaxCompactCompletion,
        ) == seq![2nat, 3nat],
        m1_target_operation_dispatch_count_spec(544, 1) == 545,
{
    reveal(m1_operation_dispatch_kinds_spec);
    reveal(m1_target_operation_dispatch_count_spec);
    operation_dispatch_expansion::m1_target_completion_dispatch_shape();
}

/// Cross-crate verifier view of target-only addressless physical-recipe
/// cardinality.
pub open spec fn m1_target_only_physical_recipe_count_spec(
    operation_dispatch_count: nat,
) -> nat {
    physical_dispatch_recipe::m1_target_only_physical_recipe_count_spec(
        operation_dispatch_count,
    )
}

/// Exposes the exact reviewed target-only physical-recipe shape.
pub proof fn m1_target_only_physical_recipe_shape()
    ensures m1_target_only_physical_recipe_count_spec(545) == 545,
{
    reveal(m1_target_only_physical_recipe_count_spec);
    physical_dispatch_recipe::m1_target_only_physical_recipe_shape();
}

/// Cross-crate verifier view of target-only fixed-batch cardinality.
pub open spec fn m1_target_only_fixed_batch_packet_count_spec() -> nat {
    physical_fixed_batch::m1_target_only_fixed_batch_packet_count_spec()
}

/// Exposes the exact reviewed target-only fixed-batch cardinality.
pub proof fn m1_target_only_fixed_batch_shape()
    ensures m1_target_only_fixed_batch_packet_count_spec() == 545,
{
    reveal(m1_target_only_fixed_batch_packet_count_spec);
    physical_fixed_batch::m1_target_only_fixed_batch_shape();
}

} // verus!
