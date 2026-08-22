#![forbid(unsafe_code)]

//! Safe state machines used by the generated Ferric runtime.

#[allow(unused_imports)]
use vstd::prelude::*;

mod cache;
mod completion_wire;
mod device_cache;
mod epoch;
mod operation_dispatch_expansion;
mod operation_kernel_plan;
mod physical_program_catalog;
mod physical_step;
mod runner;
mod scheduler;
mod speculative_graph;
mod step_dispatch_composition;
mod system;

pub use cache::{KvError, PageId};
pub use completion_wire::{
    bind_inert_completion_epoch, check_inert_completion_record, CheckedCompletionSemantics,
    CompletionEpochJoinFailure, CompletionWireError, CompletionWireExpectation,
    CompletionWireSemanticExpectation, EpochJoinedCompletionRecord, InertCheckedCompletionRecord,
};
pub use device_cache::{
    bind_gfx942_device, AbortedDeviceKvStepWrite, ActiveDeviceKvCache, CancelledDeviceKvCache,
    DeviceKvAppendFailure, DeviceKvCacheError, DeviceKvCacheProjection,
    DeviceKvCancellationFailure, DeviceKvCancellationOutcome, DeviceKvPageLease,
    DeviceKvReadBinding, DeviceKvRetirementOutcome, DeviceKvStepAbortFailure,
    DeviceKvStepPageBinding, DeviceKvStepPageIdentity, DeviceKvStepReservationFailure,
    Gfx942DeviceBinding, InitializedDeviceKvWrite, PendingDeviceKvStepWrite, PendingDeviceKvWrite,
    PendingWriteCompletionFailure, PoisonedDeviceKvCache, QuiescenceFailure,
    QuiescentDeviceKvCache, RetirementCompletionFailure, WriteApplicationFailure, GFX942_PROCESSOR,
    GFX942_TARGET_FEATURES,
};
pub use epoch::ExactCompletion;
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
pub use physical_program_catalog::{
    bind_content_bound_m1_program_catalog_v1, ContentBoundM1ProgramCatalogV1,
    InspectedM1KernelArtifacts, M1PhysicalProgramCatalogErrorV1, M1PhysicalProgramFamilyV1,
    M1PhysicalProgramV1, M1_PHYSICAL_PROGRAM_COUNT_V1,
};
pub use physical_step::{
    bind_structural_physical_step, StructuralPhysicalStepBindingError,
    StructuralPhysicalStepBindingFailure, StructuralPhysicalStepBindingOutcome,
    StructurallyBoundPhysicalStep,
};
pub use runner::{LogicalRunnerDeclaration, LogicalRunnerError};
pub use scheduler::{DispatchBatch, SchedulerError};
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
pub use system::{CompletionFailure, Engine, EngineError};
