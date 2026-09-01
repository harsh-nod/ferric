#![forbid(unsafe_code)]

//! Safe state machines used by the generated Ferric runtime.

#[allow(unused_imports)]
use vstd::prelude::*;

mod authenticated_kernel_programs;
mod authenticated_physical_queue;
mod authenticated_physical_readback;
mod authenticated_queue_rearm;
mod bound_step_workspaces;
mod cache;
mod completed_readback_join;
mod completion_canary;
mod completion_output;
mod completion_wire;
mod device_cache;
mod direct_diagnostic_choices;
mod epoch;
mod initialized_model_memory;
mod initialized_step_workspaces;
mod kv_workspace_authority;
mod m1_completed_step;
mod m1_completed_step_release;
mod m1_packet_diagnostic;
mod m1_packet_diagnostic_execution;
mod m1_prepublication;
mod m1_queue_rearm;
mod m1_queue_rollover;
mod m1_serving_physical_bridge;
mod m1_serving_physical_input_provider;
mod m1_serving_physical_operations;
mod m1_serving_registry;
mod m1_swiglu_worker_v3_verifier;
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
mod speculative_diagnostic_choices;
mod speculative_generation_loop;
mod speculative_graph;
mod step_dispatch_composition;
mod step_workspace_composition;
mod step_workspace_images;
mod step_workspace_subleases;
mod system;

pub use authenticated_kernel_programs::{
    admit_m1_authenticated_worker_v3_programs_v1, require_m1_authenticated_roster_acquisition_v1,
    M1AuthenticatedProgramSetIntakeErrorV1, M1AuthenticatedProgramSetIntakeFailureV1,
    M1AuthenticatedProgramSetIntakePhaseV1, M1AuthenticatedRosterAcquisitionRequiredV1,
    M1AuthenticatedWorkerV3ProgramSetResidueV1, M1AuthenticatedWorkerV3ProgramSetV1,
    M1AuthenticatedWorkerV3RostersV1, M1GemmWorkerV3RosterV1, M1LogitsWorkerV3RosterV1,
    M1PagedDecodeWorkerV3RosterV1, M1PrefillWorkerV3RosterV1, M1RmsNormWorkerV3RosterV1,
    M1SwiGluWorkerV3RosterV1, M1_AUTHENTICATED_PROGRAM_TARGET_V1, M1_AUTHENTICATED_ROSTER_COUNT_V1,
};
pub use authenticated_physical_queue::{
    M1AuthenticatedPhysicalCompletedQueueSessionV1, M1AuthenticatedPhysicalDetachedQueueSessionV1,
    M1AuthenticatedPhysicalPublishedQueueSessionV1, M1AuthenticatedPhysicalQueueCreateDiagnosticV1,
    M1AuthenticatedPhysicalQueueCreateFailureV1, M1AuthenticatedPhysicalQueueCreateTerminalV1,
    M1AuthenticatedPhysicalQueueOperationFailureV1, M1AuthenticatedPhysicalQueuePhaseCaseV1,
    M1AuthenticatedPhysicalQueueReuseFailureV1, M1AuthenticatedPhysicalQueueSessionV1,
    M1AuthenticatedPhysicalQueueSubmitFailureV1, M1AuthenticatedPhysicalRecycledQueueSessionV1,
};
pub use authenticated_physical_readback::{
    M1AuthenticatedCompletedReadbackJoinFailureV1,
    M1AuthenticatedCompletionObservationFailureCustodyV1,
    M1AuthenticatedCompletionObservationFailureV1,
    M1AuthenticatedCompletionSnapshotReadFailedOutputV1, M1AuthenticatedObservedCompletionCaseV1,
    M1AuthenticatedObservedCompletionOutputV1, M1AuthenticatedPhysicalCompletedReadbackV1,
    M1AuthenticatedPhysicalPostReadbackQueueReleaseFailureV1,
    M1AuthenticatedPhysicalPostReadbackQueueReleaseResidueV1,
    M1AuthenticatedPhysicalReadbackDetachedQueueCaseV1,
    M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1,
    M1AuthenticatedPhysicalReadbackQueueCaseV1,
    M1AuthenticatedPhysicalReadbackQueueOperationFailureV1,
    M1AuthenticatedPhysicalReadbackQueueReleaseFailureV1,
    M1AuthenticatedPhysicalReadbackQueueSessionV1, M1AuthenticatedReadbackTeardownDiagnosticV1,
    M1AuthenticatedReadbackTeardownEvidenceV1, M1AuthenticatedReadbackTeardownFailureV1,
    M1AuthenticatedReadbackTeardownSuccessV1, M1AuthenticatedRejectedCompletionCaseV1,
    M1AuthenticatedRejectedCompletionOutputV1,
};
pub use authenticated_queue_rearm::{
    prepare_m1_authenticated_long_lived_queue_rearm_v1,
    reserve_m1_authenticated_long_lived_queue_rearm_kv_v1,
    schedule_m1_authenticated_long_lived_queue_rearm_exact_v1,
    schedule_m1_authenticated_long_lived_queue_rearm_v1,
    submit_m1_authenticated_long_lived_queue_rearm_v1,
    M1AuthenticatedLongLivedQueueRearmKvReservationFailureV1,
    M1AuthenticatedLongLivedQueueRearmPrepareErrorV1,
    M1AuthenticatedLongLivedQueueRearmPrepareFailureV1,
    M1AuthenticatedLongLivedQueueRearmScheduleErrorV1,
    M1AuthenticatedLongLivedQueueRearmScheduleFailureV1,
    M1AuthenticatedLongLivedQueueRearmScheduleRejectionV1,
    M1AuthenticatedLongLivedQueueRearmScheduleTerminalV1,
    M1AuthenticatedLongLivedQueueRearmSubmissionFailureV1,
    M1AuthenticatedLongLivedQueueRearmSubmissionPhaseV1,
    M1AuthenticatedLongLivedQueueRearmTeardownFailureV1,
    M1AuthenticatedLongLivedQueueRearmTeardownSuccessV1,
    M1AuthenticatedLongLivedQueueReleasedRoundV1, M1AuthenticatedPreparedLongLivedQueueRearmV1,
    M1AuthenticatedRearmedCompletedQueueV1, M1AuthenticatedRearmedCompletedReadbackV1,
    M1AuthenticatedRearmedCompletionOutcomeV1, M1AuthenticatedRearmedCompletionPreflightErrorV1,
    M1AuthenticatedRearmedCompletionPreflightFailureV1,
    M1AuthenticatedRearmedCompletionPreflightTeardownFailureV1,
    M1AuthenticatedRearmedCompletionPreflightTeardownSuccessV1,
    M1AuthenticatedRearmedObservedCompletionOutputV1, M1AuthenticatedRearmedPoisonedCompletionV1,
    M1AuthenticatedRearmedPublishedQueueV1, M1AuthenticatedRearmedQueueProgressFailureV1,
    M1AuthenticatedRearmedReadbackFailureSourceV1, M1AuthenticatedRearmedReadbackFailureV1,
    M1AuthenticatedRearmedReadbackTeardownFailureV1,
    M1AuthenticatedRearmedReadbackTeardownSuccessV1, M1AuthenticatedRearmedRecycledQueueV1,
    M1AuthenticatedRearmedRejectedCompletionTeardownFailureV1,
    M1AuthenticatedRearmedRejectedCompletionTeardownSuccessV1,
    M1AuthenticatedRearmedRoundPageReleaseFailureV1,
    M1AuthenticatedRearmedRoundPageReleaseTeardownFailureV1,
    M1AuthenticatedRearmedRoundPageReleaseTeardownSuccessV1,
    M1AuthenticatedRearmedRoundReleaseOutcomeV1, M1AuthenticatedReservedLongLivedQueueRearmV1,
    M1AuthenticatedScheduledLongLivedQueueRearmV1,
};
pub use bound_step_workspaces::{
    BoundM1FullStepWorkspaceSubleases, M1FullStepWorkspaceDispatchRangeError,
    M1FullStepWorkspaceSubleaseBindingError, M1FullStepWorkspaceSubleaseBindingFailure,
    M1FullStepWorkspaceSubleaseOwners,
};
pub use cache::{KvError, PageId};
pub use completed_readback_join::{M1CheckedCompletionOutputV1, M1CompletedOutputCheckErrorV1};
pub(crate) use completion_canary::{
    preflight_m1_completion_canary_v1, validate_m1_completion_canary_readback_v1,
    BoundM1CompletionCanaryV1, M1ValidatedCompletionCanaryReadbackV1,
};
pub use completion_canary::{
    M1CompletionCanaryErrorV1, M1CompletionCanaryLayoutV1, M1ObservedCompletionCanarySummaryV1,
    M1_COMPLETION_CANARY_GUARD_BYTES_V1, M1_COMPLETION_CANARY_PREFIX_BYTE_V1,
    M1_COMPLETION_CANARY_SUFFIX_BYTE_V1,
};
pub use completion_output::{
    allocate_m1_completion_output_v1, allocate_m1_guarded_completion_output_v1,
    m1_completion_output_shape_v1, BoundM1CompletionOutputV1, M1CompletionOutputErrorV1,
    M1CompletionOutputShapeV1, M1_COMPLETION_OUTPUT_ALIGNMENT_V1,
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
    M1FiniteSpeculativeRolloverOutputActivationErrorV1,
    M1FiniteSpeculativeRolloverOutputActivationFailureV1,
    M1FiniteSpeculativeRolloverOutputPortfolioStateV1,
    M1FiniteSpeculativeRolloverOutputReserveErrorV1, M1PartitionedModelMemoryKvPoolV1,
    M1PartitionedModelMemoryKvQueueCustodyV1, M1PendingQualificationContextStepWriteV1,
    M1QualificationContextStepAbortFailureV1, M1QualificationContextStepReservationFailureV1,
    M1QualificationPendingStepAbortFailureV1, M1QualificationTargetPagePreleaseCancellationErrorV1,
    M1QualificationTargetPagePreleaseCancellationExhaustedV1,
    M1QualificationTargetPagePreleaseCancellationFailureV1,
    M1QualificationTargetPagePreleaseCancellationPageErrorV1,
    M1QualificationTargetPagePreleaseCancellationSuccessV1,
    M1QualificationTargetPagePreleaseErrorV1, M1QualificationTargetPagePreleaseFailureV1,
    M1QualificationTargetPagePreleaseProgressV1, M1QualificationTargetPagePreleaseRecoveryV1,
    M1QualificationTargetPagePreleaseSuccessV1, M1QualificationTargetPageReserveV1,
    M1S1K4RolloverOutputActivationErrorV1, M1S1K4RolloverOutputActivationFailureV1,
    M1S1K4RolloverOutputPortfolioStateV1, M1S1K4RolloverOutputReserveErrorV1,
    M1S1K4RolloverOutputReserveV1, M1TargetPartitionedKvQuarantineV1,
    M1UnpartitionedModelMemoryKvRecoveryV1, M1UnpublishedKvPageReturnErrorV1,
    M1UnpublishedKvPageReturnFailureV1, PendingDeviceKvStepWrite, PendingDeviceKvWrite,
    PendingSpeculativeDraftKvRoundWrite, PendingWriteCompletionFailure, PoisonedDeviceKvCache,
    QuiescenceFailure, QuiescentDeviceKvCache, RetirementCompletionFailure,
    SettledQuiescentDeviceKvCache, WriteApplicationFailure, GFX942_PROCESSOR,
    GFX942_TARGET_FEATURES, M1_QUALIFICATION_TARGET_PAGE_COUNT_V1,
};
pub use direct_diagnostic_choices::{
    m1_direct_diagnostic_choices_shape_v1, BoundM1DirectDiagnosticChoicesV1,
    M1DirectDiagnosticChoicesAllocationFailureV1, M1DirectDiagnosticChoicesErrorV1,
    M1DirectDiagnosticChoicesShapeV1, M1ObservedDirectDiagnosticChoicesV1,
    M1_DIRECT_DIAGNOSTIC_CHOICE_ALIGNMENT_V1,
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
    complete_m1_authenticated_physical_step_v1, complete_m1_physical_step_v1,
    M1AuthenticatedCompletedStepOutcomeV1, M1AuthenticatedCompletedStepPoisonV1,
    M1AuthenticatedCompletedStepRejectionTeardownFailureV1,
    M1AuthenticatedCompletedStepRejectionTeardownSuccessV1,
    M1AuthenticatedCompletedStepRejectionV1, M1AuthenticatedCompletedStepSuccessV1,
    M1AuthenticatedCompletedStepTeardownFailureV1, M1AuthenticatedCompletedStepTeardownSuccessV1,
    M1CompletedDeviceKvMemberV1, M1CompletedStepErrorV1, M1CompletedStepOutcomeV1,
    M1CompletedStepPoisonV1, M1CompletedStepRejectionTeardownFailureV1,
    M1CompletedStepRejectionTeardownSuccessV1, M1CompletedStepRejectionV1,
    M1CompletedStepSuccessV1, M1CompletedStepTeardownFailureV1, M1CompletedStepTeardownSuccessV1,
    M1DeviceKvCompletionDispositionV1, M1DeviceKvCompletionMemberV1, M1DeviceKvCompletionRosterV1,
};
pub use m1_completed_step_release::{
    release_m1_authenticated_completed_step_kv_pages_v1, release_m1_completed_step_kv_pages_v1,
    M1AuthenticatedCompletedStepKvReleaseFailureV1, M1AuthenticatedReleasedCompletedStepV1,
    M1AuthenticatedReleasedQueueTeardownFailureV1, M1AuthenticatedReleasedQueueTeardownSuccessV1,
    M1CompletedKvPageIdentityErrorV1, M1CompletedKvPageReleaseCountsV1,
    M1CompletedStepKvReleaseErrorV1, M1CompletedStepKvReleaseFailureV1, M1ReleasedCompletedStepV1,
    M1ReleasedDeviceKvMemberV1, M1ReleasedQueueTeardownFailureV1, M1ReleasedQueueTeardownSuccessV1,
    M1ReleasedTerminalDeviceKvMemberV1,
};
pub use m1_packet_diagnostic::{
    m1_k1_target_s1t128_packet_diagnostic_spec_v1, m1_k7_s1k4_packet_diagnostic_spec_v1,
    M1PacketDiagnosticBufferAccessV1, M1PacketDiagnosticBufferV1, M1PacketDiagnosticKindV1,
    M1PacketDiagnosticSpecErrorV1, M1PacketDiagnosticSpecV1,
    M1_PACKET_DIAGNOSTIC_CONTENT_ROLE_IDENTITY_V1, M1_PACKET_DIAGNOSTIC_RING_BYTES_V1,
};
pub use m1_packet_diagnostic_execution::{
    execute_m1_k1_target_s1t128_packet_v1, execute_m1_k7_s1k4_packet_v1,
    M1K1S1T128PacketObservationV1, M1K7S1K4PacketObservationV1,
};
pub use m1_prepublication::{
    allocate_m1_prepublication_workspaces_v1, build_m1_prepublication_batch_v1,
    prepare_m1_scheduled_workspace_images_v1, M1AllocatedScheduledStepV1,
    M1AuthenticatedPrepublicationBatchBuildFailureV1, M1AuthenticatedPrepublicationBatchV1,
    M1FullStepKvReservationCustodyV1, M1FullStepKvWorkspaceTablesV1, M1PrepareFailureV1,
    M1PreparedScheduledWorkspaceImagesV1, M1PrepublicationAllocationFailureV1,
    M1PrepublicationBatchBuildDiagnosticV1, M1PrepublicationBatchBuildErrorKindV1,
    M1PrepublicationBatchBuildFailureV1, M1PrepublicationBatchV1,
    M1PrepublicationCompositionFailureV1, M1PrepublicationJoinErrorV1,
    M1PrepublicationJoinFailureV1, M1PrepublicationStepCustodyV1, M1WorkspaceImageResidueV1,
};
pub use m1_queue_rearm::{
    prepare_m1_long_lived_queue_rearm_v1, reserve_m1_long_lived_queue_rearm_kv_v1,
    schedule_m1_long_lived_queue_rearm_exact_v1, schedule_m1_long_lived_queue_rearm_v1,
    submit_m1_finite_speculative_queue_rollover_v1, submit_m1_long_lived_queue_rearm_v1,
    submit_m1_s1_k4_queue_rollover_v1, M1LongLivedQueueRearmKvInputsV1,
    M1LongLivedQueueRearmKvReservationFailureV1, M1LongLivedQueueRearmKvReservationPhaseV1,
    M1LongLivedQueueRearmPrepareFailureV1, M1LongLivedQueueRearmProgressPhaseV1,
    M1LongLivedQueueRearmScheduleClosureOutcomeV1, M1LongLivedQueueRearmScheduleDetachQuarantineV1,
    M1LongLivedQueueRearmScheduleDetachedTeardownFailureV1,
    M1LongLivedQueueRearmScheduleDetachedTeardownSuccessV1, M1LongLivedQueueRearmScheduleErrorV1,
    M1LongLivedQueueRearmScheduleFailureV1, M1LongLivedQueueRearmSchedulePhaseV1,
    M1LongLivedQueueRearmSubmissionFailureV1, M1LongLivedQueueRearmSubmissionPhaseV1,
    M1LongLivedQueueRearmTeardownFailureV1, M1LongLivedQueueRearmTeardownSuccessV1,
    M1LongLivedQueueReleasedRoundV1, M1LongLivedQueueUnscheduledRoundV1,
    M1PreparedLongLivedQueueRearmV1, M1QueueRolloverObservationV1, M1RearmRoundHistoryEntryV1,
    M1RearmedCompletedQueueV1, M1RearmedCompletedReadbackV1, M1RearmedCompletionOutcomeV1,
    M1RearmedCompletionPreflightErrorV1, M1RearmedCompletionPreflightFailureV1,
    M1RearmedCompletionPreflightTeardownFailureV1, M1RearmedCompletionPreflightTeardownSuccessV1,
    M1RearmedDirectDiagnosticCompletedReadbackV1, M1RearmedDirectDiagnosticReadbackFailureSourceV1,
    M1RearmedDirectDiagnosticReadbackFailureV1,
    M1RearmedDirectDiagnosticReadbackTeardownFailureSourceV1,
    M1RearmedDirectDiagnosticReadbackTeardownFailureV1,
    M1RearmedDirectDiagnosticReadbackTeardownSuccessSourceV1,
    M1RearmedDirectDiagnosticReadbackTeardownSuccessV1, M1RearmedDirectDiagnosticRetainedCustodyV1,
    M1RearmedObservedCompletionOutputV1, M1RearmedObservedQualificationOutputV1,
    M1RearmedObservedQualificationTeardownFailureV1,
    M1RearmedObservedQualificationTeardownSuccessV1, M1RearmedPoisonedCompletionV1,
    M1RearmedPublishedQueueV1, M1RearmedQualificationCompletedReadbackJoinFailureV1,
    M1RearmedQualificationObservationFailureV1, M1RearmedQualificationObservationTeardownFailureV1,
    M1RearmedQualificationObservationTeardownSuccessV1,
    M1RearmedQualificationSemanticTeardownFailureV1,
    M1RearmedQualificationSemanticTeardownSuccessV1, M1RearmedQualifiedCompletedReadbackV1,
    M1RearmedQualifiedCompletionOutcomeV1, M1RearmedQualifiedCompletionPreflightFailureV1,
    M1RearmedQualifiedCompletionPreflightTeardownFailureV1,
    M1RearmedQualifiedCompletionPreflightTeardownSuccessV1, M1RearmedQualifiedPoisonedCompletionV1,
    M1RearmedQualifiedReadbackTeardownFailureV1, M1RearmedQualifiedReadbackTeardownSuccessV1,
    M1RearmedQualifiedRejectedCompletionTeardownFailureV1,
    M1RearmedQualifiedRejectedCompletionTeardownSuccessV1, M1RearmedQualifiedReleasedRoundV1,
    M1RearmedQualifiedRoundPageReleaseFailureV1,
    M1RearmedQualifiedRoundPageReleaseTeardownFailureV1,
    M1RearmedQualifiedRoundPageReleaseTeardownSuccessV1, M1RearmedQualifiedRoundReleaseOutcomeV1,
    M1RearmedQualifiedTeardownFailureV1, M1RearmedQualifiedTeardownSuccessV1,
    M1RearmedQueueProgressFailureV1, M1RearmedReadbackCaptureReleaseStateV1,
    M1RearmedReadbackFailureSourceV1, M1RearmedReadbackFailureV1,
    M1RearmedReadbackTeardownDiagnosticV1, M1RearmedReadbackTeardownEvidenceV1,
    M1RearmedReadbackTeardownFailureV1, M1RearmedReadbackTeardownSuccessV1,
    M1RearmedRecycledQueueV1, M1RearmedRejectedCompletionTeardownFailureV1,
    M1RearmedRejectedCompletionTeardownSuccessV1, M1RearmedRoundPageReleaseFailureV1,
    M1RearmedRoundPageReleaseTeardownFailureV1, M1RearmedRoundPageReleaseTeardownSuccessV1,
    M1RearmedRoundReleaseOutcomeV1, M1RearmedSpeculativeDiagnosticCompletedReadbackV1,
    M1RearmedSpeculativeDiagnosticReadbackFailureSourceV1,
    M1RearmedSpeculativeDiagnosticReadbackFailureV1,
    M1RearmedSpeculativeDiagnosticReadbackTeardownFailureSourceV1,
    M1RearmedSpeculativeDiagnosticReadbackTeardownFailureV1,
    M1RearmedSpeculativeDiagnosticReadbackTeardownSuccessSourceV1,
    M1RearmedSpeculativeDiagnosticReadbackTeardownSuccessV1,
    M1RearmedSpeculativeDiagnosticRetainedCustodyV1, M1ReservedLongLivedQueueRearmV1,
    M1ScheduledLongLivedQueueRearmTeardownFailureV1,
    M1ScheduledLongLivedQueueRearmTeardownSuccessV1, M1ScheduledLongLivedQueueRearmV1,
    M1_MAX_REARM_ROUND_HISTORY_V1,
};
pub use m1_queue_rollover::{
    prepare_m1_finite_speculative_queue_rollover_v1, prepare_m1_s1_k4_queue_rollover_v1,
    reserve_m1_finite_speculative_queue_rollover_kv_v1, reserve_m1_s1_k4_queue_rollover_kv_v1,
    schedule_m1_finite_speculative_queue_rollover_v1, schedule_m1_s1_k4_queue_rollover_exact_v1,
    M1FiniteSpeculativeQueueRolloverKvInputsV1,
    M1FiniteSpeculativeQueueRolloverKvReservationFailureV1,
    M1FiniteSpeculativeQueueRolloverKvReservationPhaseV1,
    M1FiniteSpeculativeQueueRolloverPrepareFailureV1, M1FiniteSpeculativeQueueRolloverResidueV1,
    M1FiniteSpeculativeQueueRolloverScheduleErrorV1,
    M1FiniteSpeculativeQueueRolloverScheduleFailureCustodyV1,
    M1FiniteSpeculativeQueueRolloverScheduleFailureV1, M1PreparedFiniteSpeculativeQueueRolloverV1,
    M1PreparedS1K4QueueRolloverV1, M1ReservedFiniteSpeculativeQueueRolloverV1,
    M1ReservedS1K4QueueRolloverV1, M1S1K4QueueRolloverKvInputsV1,
    M1S1K4QueueRolloverKvReservationFailureV1, M1S1K4QueueRolloverKvReservationPhaseV1,
    M1S1K4QueueRolloverPrepareFailureV1, M1S1K4QueueRolloverResidueV1,
    M1S1K4QueueRolloverScheduleClosureOutcomeV1, M1S1K4QueueRolloverScheduleDetachQuarantineV1,
    M1S1K4QueueRolloverScheduleDetachedTeardownFailureV1,
    M1S1K4QueueRolloverScheduleDetachedTeardownSuccessV1, M1S1K4QueueRolloverScheduleErrorV1,
    M1S1K4QueueRolloverScheduleFailureCustodyV1, M1S1K4QueueRolloverScheduleFailureV1,
    M1ScheduledFiniteSpeculativeQueueRolloverV1, M1ScheduledS1K4QueueRolloverTeardownFailureV1,
    M1ScheduledS1K4QueueRolloverTeardownSuccessV1, M1ScheduledS1K4QueueRolloverV1,
};
pub use m1_serving_physical_bridge::{
    M1ServingCommittedSpeculativeRoundV1, M1ServingPhysicalAbortFailureV1,
    M1ServingPhysicalBridgeErrorV1, M1ServingPhysicalBridgeFailureV1,
    M1ServingPhysicalCompletionErrorV1, M1ServingPhysicalCompletionFailureCustodyV1,
    M1ServingPhysicalCompletionFailureV1, M1ServingPhysicalFailureCustodyV1,
    M1ServingPhysicalOperationFailureV1, M1ServingPhysicalOperationResultV1,
    M1ServingPhysicalOperationsV1, M1ServingPhysicalPublishResultV1, M1ServingPhysicalPublishedV1,
    M1ServingPhysicalQueueCustodyV1, M1ServingPhysicalReadbackResultV1,
    M1ServingPhysicalReadbackV1, M1ServingPhysicalRecordRetryFailureV1,
    M1ServingPhysicalRetryablePublicationV1, M1ServingPhysicalTerminalPublicationV1,
    M1ServingPhysicalUnmatchedPublishedV1, M1ServingPhysicalUnrecordedPublishedV1,
    M1ServingRegistryCompletionErrorV1, M1ServingRegistryCompletionFailureCustodyV1,
    M1ServingRegistryCompletionFailureV1, M1ServingRegistryCompletionResultV1,
    M1ServingSpeculativeCompletionErrorV1, M1ServingSpeculativeCompletionFailureCustodyV1,
    M1ServingSpeculativeCompletionFailureV1, M1ServingSpeculativeCompletionResultV1,
};
pub use m1_serving_physical_input_provider::{
    M1QueuedServingPhysicalInputProviderV1, M1ServingPhysicalInputEnqueueFailureV1,
    M1ServingPhysicalInputPreparationErrorV1, M1ServingPhysicalInputPreparationFailureV1,
    M1ServingPhysicalInputPreparationPhaseV1, M1ServingQueuedFiniteSpeculativeRolloverV1,
    M1ServingQueuedFirstPublicationV1, M1ServingQueuedGenerationBindingV1,
    M1ServingQueuedGenerationInputV1, M1ServingQueuedGenerationPhaseV1,
    M1ServingQueuedS1K4RolloverV1, M1ServingQueuedSameShapeRearmV1,
};
pub use m1_serving_physical_operations::{
    M1ServingFirstReadbackStateV1, M1ServingPhysicalInputProviderV1,
    M1ServingPhysicalRunnerDiagnosticBindingV1, M1ServingPhysicalRunnerDiagnosticHistoryV1,
    M1ServingPhysicalRunnerFiniteSpeculativeRolloverEnqueueFailureV1,
    M1ServingPhysicalRunnerGenerationEnqueueFailureV1,
    M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1, M1ServingPhysicalRunnerOperationErrorV1,
    M1ServingPhysicalRunnerOperationsCreateErrorV1, M1ServingPhysicalRunnerOperationsV1,
    M1ServingPhysicalRunnerPublishedV1, M1ServingPhysicalRunnerQuiescentV1,
    M1ServingPhysicalRunnerReadbackEvidenceV1, M1ServingPhysicalRunnerReadbackV1,
    M1ServingPhysicalRunnerS1K4RearmEnqueueFailureV1,
    M1ServingPhysicalRunnerSpeculativeRearmEnqueueFailureV1,
    M1ServingPhysicalRunnerTerminalCustodyV1, M1ServingPhysicalRunnerTerminalLowerCustodyV1,
    M1ServingPreparedFiniteSpeculativeRolloverV1, M1ServingPreparedFirstPublicationV1,
    M1ServingPreparedS1K4RolloverV1, M1ServingPreparedSameShapeRearmV1,
    M1ServingPreparedSemanticEvidenceV1, M1ServingRearmedReadbackStateV1,
};
pub use m1_serving_registry::{
    M1ServingBatchPlanV1, M1ServingCompletionDispositionV1, M1ServingPlanV1,
    M1ServingPublicationFailureV1, M1ServingPublicationReservationV1, M1ServingQueueActionV1,
    M1ServingQuiescenceV1, M1ServingQuiescentQueueActionV1, M1ServingRegistryErrorV1,
    M1ServingRegistryIdentityV1, M1ServingRegistryV1, M1ServingRequestPhaseV1,
    M1ServingRolloverReasonV1,
};
pub use m1_swiglu_worker_v3_verifier::{
    current_m1_swiglu_worker_v3_build_v1, prepare_m1_swiglu_protected_verifier_request_v1,
    require_current_m1_swiglu_receipt_bearing_envelope_v2,
    M1SwiGluCompilerReceiptCarriageIdentitiesV1, M1SwiGluCurrentEnvelopeSchemaV1,
    M1SwiGluProtectedBuildIdentitiesV1, M1SwiGluProtectedVerifierRequestErrorV1,
    M1SwiGluProtectedVerifierRequestV1, M1SwiGluProtectedWorkerV3BuildFieldV1,
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
    bind_declared_operation_kernel_plan, select_declared_operator_certificate,
    DeclaredKernelFamilyArtifact, DeclaredOperationIdentity, DeclaredOperationKernelBinding,
    DeclaredOperationKernelPlan, DeclaredOperatorCertificateError,
    DeclaredOperatorCertificateIdentityRole, OperationKernelIdentityComponent,
    OperationKernelPlanError, OperationKernelPlanFailure, OperationKernelPlanOutcome,
    ValidatedDeclaredOperatorCertificate,
};
pub use persisted_kernel_artifacts::{
    reopen_persisted_m1_kernel_artifacts_from_directory_v1,
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
    m1_completion_progress_total_scan_bound_v1, M1CompletedReadbackJoinErrorV1,
    M1CompletedReadbackJoinFailureV1, M1CompletionEvidenceTeardownDiagnosticV1,
    M1CompletionEvidenceTeardownEvidenceV1, M1CompletionEvidenceTeardownFailureV1,
    M1CompletionEvidenceTeardownSuccessV1, M1CompletionObservationErrorV1,
    M1CompletionObservationFailureCustodyV1, M1CompletionObservationFailureV1,
    M1CompletionProgressObservationV1, M1CompletionProgressWaitDiagnosticV1,
    M1CompletionProgressWaitTerminalReasonV1, M1CompletionSnapshotReadFailedOutputV1,
    M1DirectDiagnosticCompletedReadbackJoinFailureV1, M1DirectDiagnosticCompletedReadbackV1,
    M1DirectDiagnosticObservationErrorV1, M1DirectDiagnosticObservationFailureV1,
    M1DirectDiagnosticObservationTeardownFailureV1, M1DirectDiagnosticObservationTeardownSuccessV1,
    M1DirectDiagnosticSemanticTeardownFailureV1, M1DirectDiagnosticSemanticTeardownSuccessV1,
    M1EngineQuarantinedPhysicalQueueOperationFailureV1, M1ObservedCompletionCaseV1,
    M1ObservedCompletionOutputV1, M1ObservedDirectDiagnosticOutputV1,
    M1ObservedQualificationOutputV1, M1ObservedSpeculativeDiagnosticOutputV1,
    M1PhysicalCompletedQueueSessionV1, M1PhysicalCompletedReadbackV1,
    M1PhysicalDetachedQueueCaseV1, M1PhysicalDetachedQueueSessionV1,
    M1PhysicalPublishedQueueSessionV1, M1PhysicalQueueCreateFailureClassV1,
    M1PhysicalQueueCreateFailureV1, M1PhysicalQueueOperationFailureV1, M1PhysicalQueuePhaseCaseV1,
    M1PhysicalQueuePhaseV1, M1PhysicalQueueReleaseFailureV1, M1PhysicalQueueSessionV1,
    M1PhysicalReadbackDetachedQueueCaseV1, M1PhysicalReadbackDetachedQueueSessionV1,
    M1PhysicalReadbackQueueCaseV1, M1PhysicalReadbackQueueOperationFailureV1,
    M1PhysicalReadbackQueueReleaseFailureV1, M1PhysicalReadbackQueueSessionV1,
    M1PhysicalRecycledQueueSessionV1, M1QualificationCompletedReadbackJoinFailureV1,
    M1QualificationCompletionEvidenceV1, M1QualificationEvidenceTeardownFailureV1,
    M1QualificationEvidenceTeardownSuccessV1, M1QualificationObservationErrorV1,
    M1QualificationObservationFailureCustodyV1, M1QualificationObservationFailureV1,
    M1QualificationObservationTeardownEvidenceV1, M1QualificationObservationTeardownFailureV1,
    M1QualificationObservationTeardownSuccessV1, M1QualificationSemanticTeardownFailureV1,
    M1QualificationSemanticTeardownSuccessV1, M1QualifiedPhysicalCompletedReadbackV1,
    M1RejectedCompletionCaseV1, M1RejectedCompletionOutputV1,
    M1SpeculativeDiagnosticCompletedReadbackJoinFailureV1,
    M1SpeculativeDiagnosticCompletedReadbackV1, M1SpeculativeDiagnosticCompletedTeardownFailureV1,
    M1SpeculativeDiagnosticCompletedTeardownSuccessV1, M1SpeculativeDiagnosticObservationErrorV1,
    M1SpeculativeDiagnosticObservationFailureV1,
    M1SpeculativeDiagnosticObservationTeardownFailureV1,
    M1SpeculativeDiagnosticObservationTeardownSuccessV1,
    M1SpeculativeDiagnosticSemanticTeardownFailureV1,
    M1SpeculativeDiagnosticSemanticTeardownSuccessV1,
    M1_COMPLETION_PROGRESS_MAX_CONSECUTIVE_STALLED_SCANS_V1,
    M1_COMPLETION_PROGRESS_PENDING_SCAN_PAUSE_MICROS_V1, M1_COMPLETION_PROGRESS_WAIT_POLICY_ID_V2,
};
#[cfg(feature = "qualification-fault-injection")]
pub use physical_queue_lifecycle::{
    M1QualificationQueueTransitionFaultInjectionRejectionReasonV1,
    M1QualificationQueueTransitionFaultInjectionRejectionV1,
    M1QualificationQueueTransitionFaultSessionV1,
    M1QualificationQueueTransitionFaultTeardownFailureV1,
    M1QualificationQueueTransitionFaultTeardownSuccessV1,
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
    bind_m1_physical_runner_v1, bind_structural_m1_physical_runner_v1,
    initialize_m1_physical_runner_memory_v1, LogicalRunnerDeclaration, LogicalRunnerError,
    M1AuthenticatedPhysicalRunnerV1, M1PhysicalRunnerBindFailureV1,
    M1PhysicalRunnerFiniteSpeculativeRolloverSubmissionFailureV1,
    M1PhysicalRunnerFirstCompletionOutcomeV1, M1PhysicalRunnerFirstPublicationDiagnosticV1,
    M1PhysicalRunnerFirstPublicationExhaustedV1, M1PhysicalRunnerFirstPublicationFailureV1,
    M1PhysicalRunnerMemoryFailureV1, M1PhysicalRunnerQueueFailureStageV1,
    M1PhysicalRunnerRearmSubmissionExhaustedV1, M1PhysicalRunnerRearmSubmissionFailureV1,
    M1PhysicalRunnerRecipeFailureV1, M1PhysicalRunnerRecipeOutcomeV1,
    M1PhysicalRunnerS1K4RolloverSubmissionFailureV1, M1PhysicalRunnerV1,
    M1StructuralPhysicalRunnerBindFailureV1,
};
pub use scheduler::{DispatchBatch, M1ExactDispatchErrorV1, M1ScheduledDispatchV1, SchedulerError};
pub use speculative_diagnostic_choices::{
    m1_speculative_diagnostic_choices_shape_v1, BoundM1SpeculativeDiagnosticChoicesV1,
    M1ObservedSpeculativeDiagnosticChoicesV1, M1SpeculativeDiagnosticChoicesAllocationFailureV1,
    M1SpeculativeDiagnosticChoicesErrorV1, M1SpeculativeDiagnosticChoicesShapeV1,
    M1_SPECULATIVE_DIAGNOSTIC_CHOICE_ALIGNMENT_V1, M1_SPECULATIVE_DIAGNOSTIC_DRAFT_CHOICES_V1,
    M1_SPECULATIVE_DIAGNOSTIC_MAX_DRAFT_TOKENS_V1, M1_SPECULATIVE_DIAGNOSTIC_TARGET_CHOICES_V1,
};
pub use speculative_generation_loop::{
    M1SpeculativeCancellationReasonV1, M1SpeculativeGenerationLoopErrorV1,
    M1SpeculativeGenerationLoopV1, M1SpeculativeGenerationPolicyV1,
    M1SpeculativeKvRoleSettlementV1, M1SpeculativeMemberControlActionV1,
    M1SpeculativeMemberControlV1, M1SpeculativeMemberRoundOutcomeV1, M1SpeculativeMemberSeedV1,
    M1SpeculativeMemberSnapshotV1, M1SpeculativeMemberStatusV1, M1SpeculativePhysicalShapeV1,
    M1SpeculativePreflightedRoundV1, M1SpeculativePreparedRoundCommitFailureV1,
    M1SpeculativeRoundBindingV1, M1SpeculativeRoundMemberInputV1, M1SpeculativeRoundOutcomeV1,
    M1SpeculativeTerminalReasonV1, M1SpeculativeTokenBlockV1, M1SpeculativeVerificationChoiceV1,
};
pub use speculative_graph::{
    complete_single_member_speculative_graph, run_bounded_multi_member_speculative_graph_v1,
    M1SpeculativeGraphCommitFailureV1, M1SpeculativeGraphControlContextV1,
    M1SpeculativeGraphExecutionErrorV1, M1SpeculativeGraphExecutorV1,
    M1SpeculativeGraphFailureCustodyV1, M1SpeculativeGraphKvSettlementV1,
    M1SpeculativeGraphRoundContextV1, M1SpeculativeGraphRunFailureV1,
    M1SpeculativeGraphRunOutcomeV1, M1SpeculativeGraphStageV1, M1SpeculativeGraphStopV1,
    SingleMemberSpeculativeGraphError, SingleMemberSpeculativeGraphFailure,
    SingleMemberSpeculativeGraphInputs, SingleMemberSpeculativeGraphOutcome,
    M1_MAX_SPECULATIVE_GRAPH_ROUNDS_V1,
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
pub use system::{CompletionFailure, Engine, EngineError, M1CaptureQuarantinedEngineV1};

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
