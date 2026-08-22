#![forbid(unsafe_code)]

//! Safe state machines used by the generated Ferric runtime.

#[allow(unused_imports)]
use vstd::prelude::*;

mod cache;
mod device_cache;
mod epoch;
mod physical_step;
mod runner;
mod scheduler;
mod speculative_graph;
mod system;

pub use cache::{KvError, PageId};
pub use device_cache::{
    bind_gfx942_device, ActiveDeviceKvCache, CancelledDeviceKvCache, DeviceKvAppendFailure,
    DeviceKvCacheError, DeviceKvCacheProjection, DeviceKvCancellationFailure,
    DeviceKvCancellationOutcome, DeviceKvPageLease, DeviceKvReadBinding, DeviceKvRetirementOutcome,
    Gfx942DeviceBinding, InitializedDeviceKvWrite, PendingDeviceKvWrite,
    PendingWriteCompletionFailure, PoisonedDeviceKvCache, QuiescenceFailure,
    QuiescentDeviceKvCache, RetirementCompletionFailure, WriteApplicationFailure, GFX942_PROCESSOR,
    GFX942_TARGET_FEATURES,
};
pub use epoch::ExactCompletion;
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
pub use system::{CompletionFailure, Engine, EngineError};
