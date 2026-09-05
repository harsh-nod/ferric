//! Closed repeated-step rearm for one long-lived M1 physical queue.
//!
//! This Ferric-only bridge consumes a released completed step, detaches the
//! recycled generic queue, captures exactly one next scheduler dispatch, and
//! replaces every request-specific workspace allocation before binding and
//! submitting the same native queue. This direct path admits only an unchanged
//! decode or speculative selection and fixed-batch shape. The serving registry
//! routes paired prefill, mode/shape changes, admissions, cancellation, and
//! roster changes through homogeneous planning and explicit quiescent rollover
//! instead of weakening this same-plan boundary. This module makes no hardware,
//! numerical, or performance claim. A qualification capture attached to target
//! decode remains physically bound and is overwritten by every admitted
//! generation; Ferric observes it only after the caller selects the terminal
//! generation. Completion custody retains Engine logical acceptance
//! independently from external publication, including qualification prompt
//! commits with logical one and external zero.

use core::fmt;

use fe2o3_kfd::{
    ComputeAqlQueueDestroyedV1, ComputeAqlQueueObservationV1, Gfx942DeviceContentDescriptorV1,
};
use fe2o3_service_host::{
    DeviceWorkspaceRoleV1, HostDownloadRoleV1, ServiceCompletedReadbackV1,
    ServiceDeviceDispatchRangeV1, ServiceDispatchRangeV1, ServiceFixedBatchV1,
    ServiceFixedDispatchBufferV1, ServiceFixedDispatchPacketV1, ServiceHostDispatchRangeV1,
    ServiceHostDispatchSnapshotRangeV1, ServiceQueueDataUpdateFailureV1,
    ServiceQueueReleaseFailureV1, ServiceQueueReleaseObservationV1, ServiceQueueSessionV1,
    ServiceQueueUnboundSessionV1,
};
use ferric_build::{AddresslessM1StepWorkspacePlan, M1StepWorkspaceRange};
use ferric_spec::{
    completion::CompletionEpoch, scheduling::RequestState, Qwen3ExecutionMode, Qwen3PlanSelection,
    RequestId, M1_MAX_ACTIVE_SEQUENCES,
};

use crate::physical_buffer_bindings::{
    speculative_diagnostic_choice_source_route, SpeculativeDiagnosticChoiceSourceRouteV1,
};
use crate::physical_fixed_batch::M1PhysicalQueueBatchRearmPartsV1;
use crate::step_workspace_subleases::{
    bind_queue_replaced_m1_step_workspace, M1QueueReplacedWorkspaceBindingFailureV1,
};
use crate::{
    prepare_m1_scheduled_workspace_images_v1, ActiveDeviceKvCache,
    AddresslessM1FullStepWorkspaceComposition, AddresslessM1PhysicalBufferRecipeV1,
    BoundM1StepWorkspaceSubleases, ContentBoundM1ProgramCatalogV1, Engine, EngineError,
    Gfx942DeviceBinding, LogicalRunnerDeclaration, M1BoundPhysicalBufferRowV1,
    M1CompletedKvPageReleaseCountsV1, M1ExactDispatchErrorV1,
    M1FiniteSpeculativeRolloverOutputPortfolioStateV1, M1FullStepKvWorkspaceTablesV1,
    M1FullStepWorkspaceImagesV1, M1FullStepWorkspaceInputKind, M1FullStepWorkspacePlans,
    M1FullStepWorkspaceRole, M1FullStepWorkspaceSubleaseOwners, M1InitializedWorkspaceSlotV1,
    M1PhysicalBufferRecipeRowV1, M1PhysicalBufferSourceV1, M1PhysicalFixedBatchShapeV1,
    M1PhysicalPublishedQueueSessionV1, M1PhysicalQueueBatchCustodyV1, M1PhysicalQueuePhaseCaseV1,
    M1PhysicalQueueSessionV1, M1PhysicalReadbackDetachedQueueSessionV1,
    M1PhysicalReadbackQueueOperationFailureV1, M1PrepareFailureV1,
    M1PreparedFiniteSpeculativeQueueRolloverV1, M1PreparedS1K4QueueRolloverV1,
    M1PreparedScheduledWorkspaceImagesV1, M1PrepublicationStepCustodyV1, M1ReleasedCompletedStepV1,
    M1ReleasedDeviceKvMemberV1, M1ReleasedTerminalDeviceKvMemberV1, M1ScheduledDispatchV1,
    M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1, M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1,
    M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1, M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1,
};

/// Stable rejection before a fresh workspace replacement begins.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1LongLivedQueueRearmScheduleErrorV1 {
    UnsupportedPriorShape,
    NoContinuingRequests,
    Detach,
    Scheduler,
    ExactScheduler(M1ExactDispatchErrorV1),
    EmptySchedulerBatch,
    EpochExhausted,
    EpochNotExactNext {
        expected: CompletionEpoch,
        actual: CompletionEpoch,
    },
    MalformedSchedulerBatch {
        lane: usize,
    },
    DuplicateScheduledRequest {
        first_lane: usize,
        lane: usize,
    },
    UnownedScheduledRequest {
        lane: usize,
    },
    RoundHistoryCapacity {
        maximum: usize,
    },
    HostAllocation,
}

/// Exact ownership phase at which scheduling rejected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1LongLivedQueueRearmSchedulePhaseV1 {
    /// The intact released-step owner remains retryable or tear-down capable.
    Released,
    /// Queue detach failed and retained terminal lower-layer quarantine.
    QueueDetach,
    /// The queue detached, but no scheduler dispatch owner was returned.
    Detached,
    /// Engine dispatch authority moved into one retained scheduler owner.
    PostDispatch,
}

#[derive(Debug)]
struct ReleasedStepResidueV1 {
    checked: crate::M1CheckedCompletionOutputV1,
    members: Vec<M1ReleasedDeviceKvMemberV1>,
    logical_accepted_counts: Box<[u32]>,
    externally_published_counts: Box<[u32]>,
    release_counts: Box<[M1CompletedKvPageReleaseCountsV1]>,
    completed_members: usize,
    total_released: usize,
    history: M1RearmRoundHistoryV1,
}

/// Maximum number of append-only rearm round records retained by one queue.
pub const M1_MAX_REARM_ROUND_HISTORY_V1: usize = 8192;

/// One immutable rearm-round record retained for the life of the native queue.
#[derive(Debug)]
pub struct M1RearmRoundHistoryEntryV1 {
    checked: crate::M1CheckedCompletionOutputV1,
    logical_accepted_counts: Box<[u32]>,
    externally_published_counts: Box<[u32]>,
    release_counts: Box<[M1CompletedKvPageReleaseCountsV1]>,
    completed_members: usize,
    total_released: usize,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
    rollover: Option<M1QueueRolloverObservationV1>,
}

impl M1RearmRoundHistoryEntryV1 {
    #[allow(clippy::too_many_arguments)]
    pub(crate) const fn from_same_native_queue(
        checked: crate::M1CheckedCompletionOutputV1,
        logical_accepted_counts: Box<[u32]>,
        externally_published_counts: Box<[u32]>,
        release_counts: Box<[M1CompletedKvPageReleaseCountsV1]>,
        completed_members: usize,
        total_released: usize,
        queue_observation: ComputeAqlQueueObservationV1,
        device: Gfx942DeviceBinding,
    ) -> Self {
        Self {
            checked,
            logical_accepted_counts,
            externally_published_counts,
            release_counts,
            completed_members,
            total_released,
            queue_observation,
            device,
            rollover: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) const fn from_queue_transition(
        checked: crate::M1CheckedCompletionOutputV1,
        logical_accepted_counts: Box<[u32]>,
        externally_published_counts: Box<[u32]>,
        release_counts: Box<[M1CompletedKvPageReleaseCountsV1]>,
        completed_members: usize,
        total_released: usize,
        queue_observation: ComputeAqlQueueObservationV1,
        device: Gfx942DeviceBinding,
        rollover: Option<M1QueueRolloverObservationV1>,
    ) -> Self {
        Self {
            checked,
            logical_accepted_counts,
            externally_published_counts,
            release_counts,
            completed_members,
            total_released,
            queue_observation,
            device,
            rollover,
        }
    }

    pub const fn checked(&self) -> &crate::M1CheckedCompletionOutputV1 {
        &self.checked
    }

    #[must_use]
    pub fn logical_accepted_counts(&self) -> &[u32] {
        &self.logical_accepted_counts
    }

    #[must_use]
    pub fn externally_published_counts(&self) -> &[u32] {
        &self.externally_published_counts
    }

    #[must_use]
    pub fn release_counts(&self) -> &[M1CompletedKvPageReleaseCountsV1] {
        &self.release_counts
    }

    #[must_use]
    pub const fn completed_members(&self) -> usize {
        self.completed_members
    }

    #[must_use]
    pub const fn total_released(&self) -> usize {
        self.total_released
    }

    #[must_use]
    pub const fn queue_observation(&self) -> ComputeAqlQueueObservationV1 {
        self.queue_observation
    }

    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.device
    }

    /// Native rollover evidence when this round replaced its predecessor queue.
    #[must_use]
    pub const fn rollover_observation(&self) -> Option<M1QueueRolloverObservationV1> {
        self.rollover
    }
}

#[derive(Debug)]
pub(crate) struct M1NonEmptyRearmRoundHistoryV1<T = M1RearmRoundHistoryEntryV1> {
    earlier: Vec<T>,
    latest: T,
}

#[derive(Debug)]
struct TerminalLineageJoinV1<Source, Parked, Terminal, History> {
    source: Source,
    parked: Parked,
    terminal: Terminal,
    history: History,
}

type TerminalLineageJoinResultV1<Success, Failure, Parked, Terminal, History> = Result<
    TerminalLineageJoinV1<Success, Parked, Terminal, History>,
    TerminalLineageJoinV1<Failure, Parked, Terminal, History>,
>;

fn join_terminal_lineage<Success, Failure, Parked, Terminal, History>(
    result: Result<Success, Failure>,
    parked: Parked,
    terminal: Terminal,
    history: History,
) -> TerminalLineageJoinResultV1<Success, Failure, Parked, Terminal, History> {
    match result {
        Ok(source) => Ok(TerminalLineageJoinV1 {
            source,
            parked,
            terminal,
            history,
        }),
        Err(source) => Err(TerminalLineageJoinV1 {
            source,
            parked,
            terminal,
            history,
        }),
    }
}

#[derive(Debug)]
struct QualificationEvidenceJoinV1<Source, Evidence> {
    source: Source,
    evidence: Evidence,
}

type QualificationEvidenceJoinResultV1<Success, Failure, Evidence> = Result<
    QualificationEvidenceJoinV1<Success, Evidence>,
    QualificationEvidenceJoinV1<Failure, Evidence>,
>;

fn join_qualification_evidence<Success, Failure, Evidence>(
    result: Result<Success, Failure>,
    evidence: Evidence,
) -> QualificationEvidenceJoinResultV1<Success, Failure, Evidence> {
    match result {
        Ok(source) => Ok(QualificationEvidenceJoinV1 { source, evidence }),
        Err(source) => Err(QualificationEvidenceJoinV1 { source, evidence }),
    }
}

impl<T> M1NonEmptyRearmRoundHistoryV1<T> {
    pub(crate) const fn len(&self) -> usize {
        self.earlier.len() + 1
    }

    pub(crate) fn get(&self, index: usize) -> Option<&T> {
        if index == self.earlier.len() {
            Some(&self.latest)
        } else {
            self.earlier.get(index)
        }
    }

    pub(crate) const fn latest(&self) -> &T {
        &self.latest
    }
}

#[derive(Debug)]
pub(crate) enum M1RearmRoundHistoryV1<T = M1RearmRoundHistoryEntryV1> {
    Empty,
    NonEmpty(M1NonEmptyRearmRoundHistoryV1<T>),
}

impl<T> M1RearmRoundHistoryV1<T> {
    pub(crate) const fn len(&self) -> usize {
        match self {
            Self::Empty => 0,
            Self::NonEmpty(history) => history.len(),
        }
    }

    pub(crate) fn get(&self, index: usize) -> Option<&T> {
        match self {
            Self::Empty => None,
            Self::NonEmpty(history) => history.get(index),
        }
    }

    pub(crate) fn try_reserve_append(&mut self) -> Result<(), M1RearmedCompletionPreflightErrorV1> {
        if self.len() >= M1_MAX_REARM_ROUND_HISTORY_V1 {
            return Err(M1RearmedCompletionPreflightErrorV1::RoundHistoryCapacity {
                maximum: M1_MAX_REARM_ROUND_HISTORY_V1,
            });
        }
        match self {
            Self::Empty => Ok(()),
            Self::NonEmpty(history) => history
                .earlier
                .try_reserve(1)
                .map_err(|_| M1RearmedCompletionPreflightErrorV1::HostAllocation),
        }
    }

    pub(crate) fn append(self, entry: T) -> M1NonEmptyRearmRoundHistoryV1<T> {
        match self {
            Self::Empty => M1NonEmptyRearmRoundHistoryV1 {
                earlier: Vec::new(),
                latest: entry,
            },
            Self::NonEmpty(mut history) => {
                history.earlier.push(history.latest);
                history.latest = entry;
                history
            }
        }
    }
}

fn validate_rearm_round_history_schedule_capacity<T>(
    history: &M1RearmRoundHistoryV1<T>,
) -> Result<(), M1LongLivedQueueRearmScheduleErrorV1> {
    if history.len() >= M1_MAX_REARM_ROUND_HISTORY_V1 {
        Err(M1LongLivedQueueRearmScheduleErrorV1::RoundHistoryCapacity {
            maximum: M1_MAX_REARM_ROUND_HISTORY_V1,
        })
    } else {
        Ok(())
    }
}

#[derive(Debug)]
enum ScheduleFailureCustodyV1 {
    ReleasedWithLineage {
        released: Box<M1ReleasedCompletedStepV1>,
        parked: Vec<ActiveDeviceKvCache>,
        terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
        history: Box<M1RearmRoundHistoryV1>,
    },
    Detach {
        queue: Box<M1PhysicalReadbackQueueOperationFailureV1>,
        residue: Box<ReleasedStepResidueV1>,
    },
    Detached {
        queue: Box<M1PhysicalReadbackDetachedQueueSessionV1>,
        residue: Box<ReleasedStepResidueV1>,
        scheduler_error: Option<EngineError>,
        scheduled: Option<Box<M1ScheduledDispatchV1>>,
    },
}

/// Closed failure retaining every released, detached, or scheduler-issued owner.
#[must_use = "queue-rearm failure custody must remain retained"]
#[derive(Debug)]
pub struct M1LongLivedQueueRearmScheduleFailureV1 {
    error: M1LongLivedQueueRearmScheduleErrorV1,
    custody: ScheduleFailureCustodyV1,
}

impl M1LongLivedQueueRearmScheduleFailureV1 {
    #[must_use]
    pub const fn error(&self) -> M1LongLivedQueueRearmScheduleErrorV1 {
        self.error
    }

    /// Exact-roster scheduler rejection retained after queue detach.
    #[must_use]
    pub const fn exact_scheduler_error(&self) -> Option<M1ExactDispatchErrorV1> {
        match self.error {
            M1LongLivedQueueRearmScheduleErrorV1::ExactScheduler(error) => Some(error),
            _ => None,
        }
    }

    #[must_use]
    pub const fn phase(&self) -> M1LongLivedQueueRearmSchedulePhaseV1 {
        match &self.custody {
            ScheduleFailureCustodyV1::ReleasedWithLineage { .. } => {
                M1LongLivedQueueRearmSchedulePhaseV1::Released
            }
            ScheduleFailureCustodyV1::Detach { .. } => {
                M1LongLivedQueueRearmSchedulePhaseV1::QueueDetach
            }
            ScheduleFailureCustodyV1::Detached {
                scheduled: Some(_), ..
            } => M1LongLivedQueueRearmSchedulePhaseV1::PostDispatch,
            ScheduleFailureCustodyV1::Detached {
                scheduled: None, ..
            } => M1LongLivedQueueRearmSchedulePhaseV1::Detached,
        }
    }

    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        !matches!(self.phase(), M1LongLivedQueueRearmSchedulePhaseV1::Released)
    }

    #[must_use]
    pub fn retained_owner_count(&self) -> usize {
        match &self.custody {
            ScheduleFailureCustodyV1::ReleasedWithLineage {
                released,
                parked,
                terminal,
                ..
            } => released.members().len() + parked.len() + terminal.len() + 1,
            ScheduleFailureCustodyV1::Detach { queue, residue } => {
                let _ = queue.error();
                residue.members.len() + 1
            }
            ScheduleFailureCustodyV1::Detached {
                queue,
                residue,
                scheduler_error,
                scheduled,
            } => {
                let _ = queue.detached_dispatch_generation();
                usize::from(scheduler_error.is_some())
                    + usize::from(scheduled.is_some())
                    + residue.members.len()
                    + 1
            }
        }
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        match &self.custody {
            ScheduleFailureCustodyV1::ReleasedWithLineage { history, .. } => history.len(),
            ScheduleFailureCustodyV1::Detach { residue, .. }
            | ScheduleFailureCustodyV1::Detached { residue, .. } => residue.history.len(),
        }
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        match &self.custody {
            ScheduleFailureCustodyV1::ReleasedWithLineage { history, .. } => history.get(index),
            ScheduleFailureCustodyV1::Detach { residue, .. }
            | ScheduleFailureCustodyV1::Detached { residue, .. } => residue.history.get(index),
        }
    }

    /// Recovers the exact intact released owner and its separate lineage as a
    /// closed retry-or-teardown round.
    ///
    /// Detach and detached failures are terminal and cannot be reconstructed
    /// as a released queue session.
    ///
    /// # Errors
    ///
    /// Returns the unchanged failure if its queue was already consumed by a
    /// detach attempt.
    #[must_use = "an intact released owner remains linear"]
    pub fn into_unscheduled(self) -> Result<M1LongLivedQueueUnscheduledRoundV1, Self> {
        match self {
            Self {
                custody:
                    ScheduleFailureCustodyV1::ReleasedWithLineage {
                        released,
                        parked,
                        terminal,
                        history,
                    },
                ..
            } => Ok(M1LongLivedQueueUnscheduledRoundV1 {
                released: *released,
                parked,
                terminal,
                history: *history,
            }),
            failure => Err(failure),
        }
    }

    /// Closes every non-released scheduling failure without relabeling an
    /// intact released retry owner as terminal.
    pub fn close_terminal(self) -> M1LongLivedQueueRearmScheduleClosureOutcomeV1 {
        match self {
            failure @ Self {
                custody: ScheduleFailureCustodyV1::ReleasedWithLineage { .. },
                ..
            } => M1LongLivedQueueRearmScheduleClosureOutcomeV1::Released(failure),
            Self {
                error,
                custody: ScheduleFailureCustodyV1::Detach { queue, residue },
            } => M1LongLivedQueueRearmScheduleClosureOutcomeV1::QueueDetach(
                M1LongLivedQueueRearmScheduleDetachQuarantineV1 {
                    error,
                    queue,
                    residue,
                },
            ),
            Self {
                error,
                custody:
                    ScheduleFailureCustodyV1::Detached {
                        queue,
                        residue,
                        scheduler_error,
                        scheduled,
                    },
            } => {
                let (shape, lower, batch_custody) = (*queue).into_rearm_parts();
                let custody = M1LongLivedQueueRearmScheduleDetachedCustodyV1 {
                    error,
                    shape,
                    batch_custody,
                    residue,
                    scheduler_error,
                    scheduled,
                };
                M1LongLivedQueueRearmScheduleClosureOutcomeV1::Detached(Box::new(
                    match lower.destroy_and_release() {
                        Ok(queue_release) => {
                            Ok(M1LongLivedQueueRearmScheduleDetachedTeardownSuccessV1 {
                                queue_release,
                                custody,
                            })
                        }
                        Err(source) => Err(Box::new(
                            M1LongLivedQueueRearmScheduleDetachedTeardownFailureV1 {
                                source,
                                custody,
                            },
                        )),
                    },
                ))
            }
        }
    }
}

/// Exhaustive closure of a failed scheduling transition.
#[must_use = "schedule closure retains every phase-local owner"]
#[derive(Debug)]
pub enum M1LongLivedQueueRearmScheduleClosureOutcomeV1 {
    Released(M1LongLivedQueueRearmScheduleFailureV1),
    QueueDetach(M1LongLivedQueueRearmScheduleDetachQuarantineV1),
    Detached(
        Box<
            Result<
                M1LongLivedQueueRearmScheduleDetachedTeardownSuccessV1,
                Box<M1LongLivedQueueRearmScheduleDetachedTeardownFailureV1>,
            >,
        >,
    ),
}

/// Lower detach quarantine retaining exact released-step residue.
#[must_use = "detach quarantine and released-step residue remain retained"]
#[derive(Debug)]
pub struct M1LongLivedQueueRearmScheduleDetachQuarantineV1 {
    error: M1LongLivedQueueRearmScheduleErrorV1,
    queue: Box<M1PhysicalReadbackQueueOperationFailureV1>,
    residue: Box<ReleasedStepResidueV1>,
}

impl M1LongLivedQueueRearmScheduleDetachQuarantineV1 {
    #[must_use]
    pub const fn error(&self) -> M1LongLivedQueueRearmScheduleErrorV1 {
        self.error
    }

    pub const fn source(&self) -> &M1PhysicalReadbackQueueOperationFailureV1 {
        &self.queue
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.residue.history.len()
    }
}

#[derive(Debug)]
struct M1LongLivedQueueRearmScheduleDetachedCustodyV1 {
    error: M1LongLivedQueueRearmScheduleErrorV1,
    shape: M1PhysicalFixedBatchShapeV1,
    batch_custody: M1PhysicalQueueBatchCustodyV1,
    residue: Box<ReleasedStepResidueV1>,
    scheduler_error: Option<EngineError>,
    scheduled: Option<Box<M1ScheduledDispatchV1>>,
}

/// Clean release after a detached scheduling failure.
#[must_use = "detached scheduling residue remains retained"]
#[derive(Debug)]
pub struct M1LongLivedQueueRearmScheduleDetachedTeardownSuccessV1 {
    queue_release: ServiceQueueReleaseObservationV1,
    custody: M1LongLivedQueueRearmScheduleDetachedCustodyV1,
}

/// Terminal lower release failure after detached scheduling failure.
#[must_use = "detached scheduling release quarantine remains retained"]
#[derive(Debug)]
pub struct M1LongLivedQueueRearmScheduleDetachedTeardownFailureV1 {
    source: ServiceQueueReleaseFailureV1,
    custody: M1LongLivedQueueRearmScheduleDetachedCustodyV1,
}

impl M1LongLivedQueueRearmScheduleDetachedTeardownSuccessV1 {
    #[must_use]
    pub const fn error(&self) -> M1LongLivedQueueRearmScheduleErrorV1 {
        self.custody.error
    }

    #[must_use]
    pub const fn exact_scheduler_error(&self) -> Option<M1ExactDispatchErrorV1> {
        match self.custody.error {
            M1LongLivedQueueRearmScheduleErrorV1::ExactScheduler(error) => Some(error),
            _ => None,
        }
    }

    #[must_use]
    pub const fn shape(&self) -> M1PhysicalFixedBatchShapeV1 {
        self.custody.shape
    }

    pub const fn batch_custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        &self.custody.batch_custody
    }

    #[must_use]
    pub const fn retained_member_count(&self) -> usize {
        self.custody.residue.members.len()
    }

    #[must_use]
    pub const fn scheduler_error(&self) -> Option<&EngineError> {
        self.custody.scheduler_error.as_ref()
    }

    #[must_use]
    pub const fn scheduled_dispatch(&self) -> Option<&M1ScheduledDispatchV1> {
        match &self.custody.scheduled {
            Some(scheduled) => Some(scheduled),
            None => None,
        }
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.custody.residue.history.len()
    }

    #[must_use]
    pub const fn queue_release(&self) -> ServiceQueueReleaseObservationV1 {
        self.queue_release
    }
}

impl M1LongLivedQueueRearmScheduleDetachedTeardownFailureV1 {
    #[must_use]
    pub const fn error(&self) -> M1LongLivedQueueRearmScheduleErrorV1 {
        self.custody.error
    }

    #[must_use]
    pub const fn exact_scheduler_error(&self) -> Option<M1ExactDispatchErrorV1> {
        match self.custody.error {
            M1LongLivedQueueRearmScheduleErrorV1::ExactScheduler(error) => Some(error),
            _ => None,
        }
    }

    #[must_use]
    pub const fn shape(&self) -> M1PhysicalFixedBatchShapeV1 {
        self.custody.shape
    }

    pub const fn batch_custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        &self.custody.batch_custody
    }

    #[must_use]
    pub const fn retained_member_count(&self) -> usize {
        self.custody.residue.members.len()
    }

    #[must_use]
    pub const fn scheduler_error(&self) -> Option<&EngineError> {
        self.custody.scheduler_error.as_ref()
    }

    #[must_use]
    pub const fn scheduled_dispatch(&self) -> Option<&M1ScheduledDispatchV1> {
        match &self.custody.scheduled {
            Some(scheduled) => Some(scheduled),
            None => None,
        }
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.custody.residue.history.len()
    }

    pub const fn source(&self) -> &ServiceQueueReleaseFailureV1 {
        &self.source
    }
}

/// Intact released round that can retry scheduling or cleanly release its queue.
#[must_use = "an unscheduled round must retry or retain/release every owner"]
#[derive(Debug)]
pub struct M1LongLivedQueueUnscheduledRoundV1 {
    released: M1ReleasedCompletedStepV1,
    parked: Vec<ActiveDeviceKvCache>,
    terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
    history: M1RearmRoundHistoryV1,
}

impl M1LongLivedQueueUnscheduledRoundV1 {
    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.history.get(index)
    }

    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.parked.len()
    }

    #[must_use]
    pub const fn terminal_count(&self) -> usize {
        self.terminal.len()
    }

    /// Retries the exact same closed scheduling transition with all lineage.
    ///
    /// # Errors
    ///
    /// Returns the same phase-tagged closed scheduling failure contract as the
    /// initial scheduling entry point.
    pub fn retry<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<M1ScheduledLongLivedQueueRearmV1, M1LongLivedQueueRearmScheduleFailureV1> {
        schedule_m1_long_lived_queue_rearm_with_lineage_v1(
            engine,
            self.released,
            self.parked,
            self.terminal,
            self.history,
        )
    }

    /// Retries intact released custody with one exact caller-named roster.
    ///
    /// Exact scheduler rejection happens only after queue detach and is
    /// therefore terminal under the same irreversible-phase policy as the
    /// automatic scheduling path.
    ///
    /// # Errors
    ///
    /// Returns the same phase-tagged exhaustive custody as the initial exact
    /// scheduling entry point.
    pub fn retry_exact<const C: usize>(
        self,
        engine: &mut Engine<C>,
        expected_epoch: CompletionEpoch,
        requests: &[RequestId],
    ) -> Result<M1ScheduledLongLivedQueueRearmV1, M1LongLivedQueueRearmScheduleFailureV1> {
        schedule_m1_long_lived_queue_rearm_exact_with_lineage_v1(
            engine,
            self.released,
            self.parked,
            self.terminal,
            self.history,
            expected_epoch,
            requests,
        )
    }

    /// Destroys and releases the intact queue while retaining every lineage owner.
    ///
    /// # Errors
    ///
    /// Returns terminal lower-layer queue release quarantine together with all
    /// parked and terminal lineage custody.
    pub fn destroy_queue_and_retain_round<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<M1LongLivedQueueRearmTeardownSuccessV1, Box<M1LongLivedQueueRearmTeardownFailureV1>>
    {
        let Self {
            released,
            parked,
            terminal,
            history,
        } = self;
        match released.destroy_queue_and_retain_step(engine) {
            Ok(released) => Ok(M1LongLivedQueueRearmTeardownSuccessV1 {
                released,
                parked,
                terminal,
                history,
            }),
            Err(released) => Err(Box::new(M1LongLivedQueueRearmTeardownFailureV1 {
                released,
                parked,
                terminal,
                history,
            })),
        }
    }
}

/// Clean queue release retaining current and historical round custody.
#[must_use = "queue release observations and lineage custody remain owned"]
#[derive(Debug)]
pub struct M1LongLivedQueueRearmTeardownSuccessV1 {
    released: crate::M1ReleasedQueueTeardownSuccessV1,
    parked: Vec<ActiveDeviceKvCache>,
    terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
    history: M1RearmRoundHistoryV1,
}

impl M1LongLivedQueueRearmTeardownSuccessV1 {
    pub const fn released(&self) -> &crate::M1ReleasedQueueTeardownSuccessV1 {
        &self.released
    }

    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.parked.len()
    }

    #[must_use]
    pub const fn terminal_count(&self) -> usize {
        self.terminal.len()
    }

    /// Prior-round completed member count when teardown followed a rearm.
    #[must_use]
    pub const fn prior_completed_members(&self) -> Option<usize> {
        match &self.history {
            M1RearmRoundHistoryV1::Empty => None,
            M1RearmRoundHistoryV1::NonEmpty(history) => Some(history.latest().completed_members()),
        }
    }

    /// Prior-round Engine logical acceptance when teardown followed a rearm.
    #[must_use]
    pub fn prior_logical_accepted_counts(&self) -> Option<&[u32]> {
        match &self.history {
            M1RearmRoundHistoryV1::Empty => None,
            M1RearmRoundHistoryV1::NonEmpty(history) => {
                Some(history.latest().logical_accepted_counts())
            }
        }
    }

    /// Prior-round externally published counts when teardown followed a rearm.
    #[must_use]
    pub fn prior_externally_published_counts(&self) -> Option<&[u32]> {
        match &self.history {
            M1RearmRoundHistoryV1::Empty => None,
            M1RearmRoundHistoryV1::NonEmpty(history) => {
                Some(history.latest().externally_published_counts())
            }
        }
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.history.get(index)
    }
}

/// Terminal queue-release failure retaining current and historical custody.
#[must_use = "terminal queue release failure retains every available owner"]
#[derive(Debug)]
pub struct M1LongLivedQueueRearmTeardownFailureV1 {
    released: Box<crate::M1ReleasedQueueTeardownFailureV1>,
    parked: Vec<ActiveDeviceKvCache>,
    terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
    history: M1RearmRoundHistoryV1,
}

impl M1LongLivedQueueRearmTeardownFailureV1 {
    pub const fn released(&self) -> &crate::M1ReleasedQueueTeardownFailureV1 {
        &self.released
    }

    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.parked.len()
    }

    #[must_use]
    pub const fn terminal_count(&self) -> usize {
        self.terminal.len()
    }

    /// Prior-round completed member count retained after a rearm teardown failure.
    #[must_use]
    pub const fn prior_completed_members(&self) -> Option<usize> {
        match &self.history {
            M1RearmRoundHistoryV1::Empty => None,
            M1RearmRoundHistoryV1::NonEmpty(history) => Some(history.latest().completed_members()),
        }
    }

    /// Prior-round Engine logical acceptance retained on teardown failure.
    #[must_use]
    pub fn prior_logical_accepted_counts(&self) -> Option<&[u32]> {
        match &self.history {
            M1RearmRoundHistoryV1::Empty => None,
            M1RearmRoundHistoryV1::NonEmpty(history) => {
                Some(history.latest().logical_accepted_counts())
            }
        }
    }

    /// Prior-round external publication retained on teardown failure.
    #[must_use]
    pub fn prior_externally_published_counts(&self) -> Option<&[u32]> {
        match &self.history {
            M1RearmRoundHistoryV1::Empty => None,
            M1RearmRoundHistoryV1::NonEmpty(history) => {
                Some(history.latest().externally_published_counts())
            }
        }
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.history.get(index)
    }
}

/// One detached same-queue owner paired with exactly one next scheduler batch.
///
/// ```compile_fail
/// use ferric_engine::M1ScheduledLongLivedQueueRearmV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1ScheduledLongLivedQueueRearmV1>();
/// ```
///
/// ```compile_fail
/// use ferric_engine::M1ScheduledLongLivedQueueRearmV1;
/// fn steal_cache(mut scheduled: M1ScheduledLongLivedQueueRearmV1) {
///     let _owned = scheduled.with_selected_caches(|_, caches| caches.to_vec());
/// }
/// ```
#[must_use = "scheduled rearm custody must proceed to preparation or remain retained"]
#[derive(Debug)]
pub struct M1ScheduledLongLivedQueueRearmV1 {
    queue: M1PhysicalReadbackDetachedQueueSessionV1,
    scheduled: M1ScheduledDispatchV1,
    selected: Vec<ActiveDeviceKvCache>,
    parked: Vec<ActiveDeviceKvCache>,
    terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
    prior_checked: crate::M1CheckedCompletionOutputV1,
    logical_accepted_counts: Box<[u32]>,
    externally_published_counts: Box<[u32]>,
    release_counts: Box<[M1CompletedKvPageReleaseCountsV1]>,
    completed_members: usize,
    total_released: usize,
    history: M1RearmRoundHistoryV1,
}

impl M1ScheduledLongLivedQueueRearmV1 {
    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        &self.scheduled
    }

    #[must_use]
    pub fn selected_requests(&self) -> impl ExactSizeIterator<Item = RequestId> + '_ {
        self.selected.iter().map(|cache| cache.projection().request)
    }

    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.parked.len()
    }

    #[must_use]
    pub const fn terminal_count(&self) -> usize {
        self.terminal.len()
    }

    #[must_use]
    pub fn prior_logical_accepted_counts(&self) -> &[u32] {
        &self.logical_accepted_counts
    }

    #[must_use]
    pub fn prior_externally_published_counts(&self) -> &[u32] {
        &self.externally_published_counts
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.history.get(index)
    }

    /// Returns immutable selected-cache projections in scheduler order.
    pub fn selected_cache_projections(
        &self,
    ) -> impl ExactSizeIterator<Item = crate::DeviceKvCacheProjection> + '_ {
        self.selected.iter().map(ActiveDeviceKvCache::projection)
    }

    /// Quarantines the Engine and destroys the detached queue while retaining
    /// the exact scheduler batch, caches, prior completion, and round history.
    ///
    /// # Errors
    ///
    /// Returns lower queue-release quarantine joined to every retained round
    /// owner when physical queue destruction fails.
    pub fn destroy_queue_and_retain_round<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1ScheduledLongLivedQueueRearmTeardownSuccessV1,
        Box<M1ScheduledLongLivedQueueRearmTeardownFailureV1>,
    > {
        engine.quarantine_m1_queue_rearm_failure();
        let Self {
            queue,
            scheduled,
            selected,
            parked,
            terminal,
            prior_checked,
            logical_accepted_counts,
            externally_published_counts,
            release_counts,
            completed_members,
            total_released,
            history,
        } = self;
        let (shape, lower, batch_custody) = queue.into_rearm_parts();
        let custody = M1ScheduledLongLivedQueueRearmTeardownCustodyV1 {
            shape,
            batch_custody,
            scheduled,
            selected,
            parked,
            terminal,
            prior_checked,
            logical_accepted_counts,
            externally_published_counts,
            release_counts,
            completed_members,
            total_released,
            history,
        };
        match lower.destroy_and_release() {
            Ok(queue_release) => Ok(M1ScheduledLongLivedQueueRearmTeardownSuccessV1 {
                queue_release,
                custody,
            }),
            Err(source) => Err(Box::new(M1ScheduledLongLivedQueueRearmTeardownFailureV1 {
                source,
                custody,
            })),
        }
    }
}

#[derive(Debug)]
struct M1ScheduledLongLivedQueueRearmTeardownCustodyV1 {
    shape: M1PhysicalFixedBatchShapeV1,
    batch_custody: M1PhysicalQueueBatchCustodyV1,
    scheduled: M1ScheduledDispatchV1,
    selected: Vec<ActiveDeviceKvCache>,
    parked: Vec<ActiveDeviceKvCache>,
    terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
    prior_checked: crate::M1CheckedCompletionOutputV1,
    logical_accepted_counts: Box<[u32]>,
    externally_published_counts: Box<[u32]>,
    release_counts: Box<[M1CompletedKvPageReleaseCountsV1]>,
    completed_members: usize,
    total_released: usize,
    history: M1RearmRoundHistoryV1,
}

/// Clean teardown of an already scheduled next round.
#[must_use = "scheduled round custody remains retained"]
#[derive(Debug)]
pub struct M1ScheduledLongLivedQueueRearmTeardownSuccessV1 {
    queue_release: ServiceQueueReleaseObservationV1,
    custody: M1ScheduledLongLivedQueueRearmTeardownCustodyV1,
}

/// Terminal lower release quarantine retaining an already scheduled round.
#[must_use = "scheduled round release quarantine remains retained"]
#[derive(Debug)]
pub struct M1ScheduledLongLivedQueueRearmTeardownFailureV1 {
    source: ServiceQueueReleaseFailureV1,
    custody: M1ScheduledLongLivedQueueRearmTeardownCustodyV1,
}

impl M1ScheduledLongLivedQueueRearmTeardownSuccessV1 {
    #[must_use]
    pub const fn shape(&self) -> M1PhysicalFixedBatchShapeV1 {
        self.custody.shape
    }

    pub const fn batch_custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        &self.custody.batch_custody
    }

    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        &self.custody.scheduled
    }

    #[must_use]
    pub fn selected_requests(&self) -> impl ExactSizeIterator<Item = RequestId> + '_ {
        self.custody
            .selected
            .iter()
            .map(|cache| cache.projection().request)
    }

    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.custody.parked.len()
    }

    #[must_use]
    pub const fn terminal_count(&self) -> usize {
        self.custody.terminal.len()
    }

    pub const fn prior_checked(&self) -> &crate::M1CheckedCompletionOutputV1 {
        &self.custody.prior_checked
    }

    #[must_use]
    pub fn prior_logical_accepted_counts(&self) -> &[u32] {
        &self.custody.logical_accepted_counts
    }

    #[must_use]
    pub fn prior_externally_published_counts(&self) -> &[u32] {
        &self.custody.externally_published_counts
    }

    #[must_use]
    pub fn prior_release_counts(&self) -> &[M1CompletedKvPageReleaseCountsV1] {
        &self.custody.release_counts
    }

    #[must_use]
    pub const fn prior_completed_members(&self) -> usize {
        self.custody.completed_members
    }

    #[must_use]
    pub const fn prior_total_released(&self) -> usize {
        self.custody.total_released
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.custody.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.custody.history.get(index)
    }

    #[must_use]
    pub const fn queue_release(&self) -> ServiceQueueReleaseObservationV1 {
        self.queue_release
    }
}

impl M1ScheduledLongLivedQueueRearmTeardownFailureV1 {
    #[must_use]
    pub const fn shape(&self) -> M1PhysicalFixedBatchShapeV1 {
        self.custody.shape
    }

    pub const fn batch_custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        &self.custody.batch_custody
    }

    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        &self.custody.scheduled
    }

    #[must_use]
    pub fn selected_requests(&self) -> impl ExactSizeIterator<Item = RequestId> + '_ {
        self.custody
            .selected
            .iter()
            .map(|cache| cache.projection().request)
    }

    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.custody.parked.len()
    }

    #[must_use]
    pub const fn terminal_count(&self) -> usize {
        self.custody.terminal.len()
    }

    pub const fn prior_checked(&self) -> &crate::M1CheckedCompletionOutputV1 {
        &self.custody.prior_checked
    }

    #[must_use]
    pub fn prior_logical_accepted_counts(&self) -> &[u32] {
        &self.custody.logical_accepted_counts
    }

    #[must_use]
    pub fn prior_externally_published_counts(&self) -> &[u32] {
        &self.custody.externally_published_counts
    }

    #[must_use]
    pub fn prior_release_counts(&self) -> &[M1CompletedKvPageReleaseCountsV1] {
        &self.custody.release_counts
    }

    #[must_use]
    pub const fn prior_completed_members(&self) -> usize {
        self.custody.completed_members
    }

    #[must_use]
    pub const fn prior_total_released(&self) -> usize {
        self.custody.total_released
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.custody.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.custody.history.get(index)
    }

    pub const fn source(&self) -> &ServiceQueueReleaseFailureV1 {
        &self.source
    }
}

fn validate_rearm_eligibility(
    shape: M1PhysicalFixedBatchShapeV1,
    selection: Qwen3PlanSelection,
    qualification_logits_enabled: bool,
) -> Result<(), M1LongLivedQueueRearmScheduleErrorV1> {
    let shape_is_supported = match shape {
        M1PhysicalFixedBatchShapeV1::TargetOnly => selection.mode == Qwen3ExecutionMode::Decode,
        M1PhysicalFixedBatchShapeV1::SpeculativeK4
        | M1PhysicalFixedBatchShapeV1::SpeculativeK8
        | M1PhysicalFixedBatchShapeV1::SpeculativeK16 => {
            selection.mode == Qwen3ExecutionMode::Speculative
        }
        M1PhysicalFixedBatchShapeV1::PairedPrefill => false,
    };
    let qualification_shape_is_supported = !qualification_logits_enabled
        || (shape == M1PhysicalFixedBatchShapeV1::TargetOnly
            && selection.mode == Qwen3ExecutionMode::Decode);
    if !qualification_shape_is_supported || !shape_is_supported {
        Err(M1LongLivedQueueRearmScheduleErrorV1::UnsupportedPriorShape)
    } else {
        Ok(())
    }
}

fn exact_next_epoch(previous: CompletionEpoch) -> Option<CompletionEpoch> {
    previous.value().checked_add(1).map(CompletionEpoch::new)
}

fn validate_request_partition(
    available: &[RequestId],
    scheduled: &[RequestId],
) -> Result<(), M1LongLivedQueueRearmScheduleErrorV1> {
    validate_request_partition_by(scheduled, |request| available.contains(&request))
}

fn validate_request_partition_by(
    scheduled: &[RequestId],
    mut is_available: impl FnMut(RequestId) -> bool,
) -> Result<(), M1LongLivedQueueRearmScheduleErrorV1> {
    for (lane, request) in scheduled.iter().copied().enumerate() {
        if let Some(first_lane) = scheduled[..lane]
            .iter()
            .position(|prior| prior.slot() == request.slot())
        {
            return Err(
                M1LongLivedQueueRearmScheduleErrorV1::DuplicateScheduledRequest {
                    first_lane,
                    lane,
                },
            );
        }
        if !is_available(request) {
            return Err(M1LongLivedQueueRearmScheduleErrorV1::UnownedScheduledRequest { lane });
        }
    }
    Ok(())
}

fn validate_exact_rearm_preflight<const C: usize>(
    engine: &Engine<C>,
    released: &M1ReleasedCompletedStepV1,
    parked: &[ActiveDeviceKvCache],
    expected_epoch: CompletionEpoch,
    requests: &[RequestId],
) -> Result<(), M1LongLivedQueueRearmScheduleErrorV1> {
    if engine.is_faulted() {
        return Err(M1LongLivedQueueRearmScheduleErrorV1::ExactScheduler(
            M1ExactDispatchErrorV1::Faulted,
        ));
    }
    if requests.is_empty() {
        return Err(M1LongLivedQueueRearmScheduleErrorV1::ExactScheduler(
            M1ExactDispatchErrorV1::EmptyRoster,
        ));
    }
    if requests.len() > M1_MAX_ACTIVE_SEQUENCES as usize {
        return Err(M1LongLivedQueueRearmScheduleErrorV1::ExactScheduler(
            M1ExactDispatchErrorV1::RosterTooLarge {
                maximum: M1_MAX_ACTIVE_SEQUENCES as usize,
                actual: requests.len(),
            },
        ));
    }
    let Some(next_epoch) = exact_next_epoch(released.checked().epoch()) else {
        return Err(M1LongLivedQueueRearmScheduleErrorV1::EpochExhausted);
    };
    if expected_epoch != next_epoch {
        return Err(M1LongLivedQueueRearmScheduleErrorV1::EpochNotExactNext {
            expected: next_epoch,
            actual: expected_epoch,
        });
    }
    validate_request_partition_by(requests, |request| {
        released.members().iter().any(|member| {
            matches!(
                member,
                M1ReleasedDeviceKvMemberV1::Active(cache)
                    if cache.projection().request == request
            )
        }) || parked
            .iter()
            .any(|cache| cache.projection().request == request)
    })?;
    for (lane, request) in requests.iter().copied().enumerate() {
        match engine.state(request) {
            Some(RequestState::Ready) => {}
            None => {
                return Err(M1LongLivedQueueRearmScheduleErrorV1::ExactScheduler(
                    M1ExactDispatchErrorV1::MissingRequest { lane, request },
                ));
            }
            Some(state) => {
                return Err(M1LongLivedQueueRearmScheduleErrorV1::ExactScheduler(
                    M1ExactDispatchErrorV1::RequestNotReady {
                        lane,
                        request,
                        state,
                    },
                ));
            }
        }
    }
    Ok(())
}

fn schedule_failure(
    error: M1LongLivedQueueRearmScheduleErrorV1,
    custody: ScheduleFailureCustodyV1,
) -> M1LongLivedQueueRearmScheduleFailureV1 {
    M1LongLivedQueueRearmScheduleFailureV1 { error, custody }
}

fn schedule_phase_failure(
    phase: M1LongLivedQueueRearmSchedulePhaseV1,
    error: M1LongLivedQueueRearmScheduleErrorV1,
    custody: ScheduleFailureCustodyV1,
) -> (
    M1LongLivedQueueRearmSchedulePhaseV1,
    M1LongLivedQueueRearmScheduleFailureV1,
) {
    (phase, schedule_failure(error, custody))
}

fn finish_schedule_transition<const C: usize, T, E>(
    engine: &mut Engine<C>,
    result: Result<T, (M1LongLivedQueueRearmSchedulePhaseV1, E)>,
) -> Result<T, E> {
    match result {
        Ok(value) => Ok(value),
        Err((phase, error)) => {
            if !matches!(phase, M1LongLivedQueueRearmSchedulePhaseV1::Released) {
                engine.quarantine_m1_queue_rearm_failure();
            }
            Err(error)
        }
    }
}

#[derive(Clone, Copy)]
enum M1LongLivedQueueRearmDispatchV1<'a> {
    Automatic,
    Exact {
        expected_epoch: CompletionEpoch,
        requests: &'a [RequestId],
    },
}

#[derive(Debug)]
enum M1LongLivedQueueRearmDispatchFailureV1 {
    EmptyAutomatic,
    Automatic(EngineError),
    Exact(M1ExactDispatchErrorV1),
}

fn dispatch_m1_long_lived_queue_rearm_v1<const C: usize>(
    engine: &mut Engine<C>,
    dispatch: M1LongLivedQueueRearmDispatchV1<'_>,
) -> Result<M1ScheduledDispatchV1, M1LongLivedQueueRearmDispatchFailureV1> {
    match dispatch {
        M1LongLivedQueueRearmDispatchV1::Automatic => engine
            .dispatch_m1_ready()
            .map_err(M1LongLivedQueueRearmDispatchFailureV1::Automatic)?
            .ok_or(M1LongLivedQueueRearmDispatchFailureV1::EmptyAutomatic),
        M1LongLivedQueueRearmDispatchV1::Exact {
            expected_epoch,
            requests,
        } => engine
            .dispatch_m1_exact_ready(expected_epoch, requests)
            .map_err(M1LongLivedQueueRearmDispatchFailureV1::Exact),
    }
}

/// Detaches one released queue and captures exactly one next scheduler batch.
///
/// Scheduler order is authoritative. Every selected request must be one of the
/// released continuing caches; other continuing caches remain parked. New
/// requests, prefill transitions, and shape changes are rejected by this slice.
/// An admitted target-decode qualification buffer is the same allocation
/// already attached before first publication: each rearmed generation
/// physically overwrites it. Rearm does not allocate or attach a final-only
/// buffer and does not itself identify which generation is terminal.
///
/// # Errors
///
/// Returns closed failure custody for unsupported prior shape, absent active
/// custody, detach or scheduler rejection, non-successor epoch, malformed,
/// duplicate, or unowned scheduler members, and host reservation failure.
pub fn schedule_m1_long_lived_queue_rearm_v1<const C: usize>(
    engine: &mut Engine<C>,
    released: M1ReleasedCompletedStepV1,
) -> Result<M1ScheduledLongLivedQueueRearmV1, M1LongLivedQueueRearmScheduleFailureV1> {
    schedule_m1_long_lived_queue_rearm_with_lineage_v1(
        engine,
        released,
        Vec::new(),
        Vec::new(),
        M1RearmRoundHistoryV1::Empty,
    )
}

/// Detaches one released queue and captures exactly the caller-named ready
/// roster in caller-provided lane order at `expected_epoch`.
///
/// Other ready Engine requests remain unchanged. Duplicate or unowned requests
/// are rejected against retained cache custody before physical queue detach.
/// Other exact scheduler rejection is pre-mutation for the Engine, but occurs
/// after detach, so the returned custody is terminal and the Engine is
/// quarantined. Once dispatch succeeds, every later failure retains its
/// [`M1ScheduledDispatchV1`] and no retry path can dispatch the roster twice.
///
/// # Errors
///
/// Returns the same closed physical custody as the automatic path, including
/// [`M1LongLivedQueueRearmScheduleErrorV1::ExactScheduler`] for exact-roster
/// scheduler rejection.
pub fn schedule_m1_long_lived_queue_rearm_exact_v1<const C: usize>(
    engine: &mut Engine<C>,
    released: M1ReleasedCompletedStepV1,
    expected_epoch: CompletionEpoch,
    requests: &[RequestId],
) -> Result<M1ScheduledLongLivedQueueRearmV1, M1LongLivedQueueRearmScheduleFailureV1> {
    schedule_m1_long_lived_queue_rearm_exact_with_lineage_v1(
        engine,
        released,
        Vec::new(),
        Vec::new(),
        M1RearmRoundHistoryV1::Empty,
        expected_epoch,
        requests,
    )
}

fn schedule_m1_long_lived_queue_rearm_with_lineage_v1<const C: usize>(
    engine: &mut Engine<C>,
    released: M1ReleasedCompletedStepV1,
    parked_lineage: Vec<ActiveDeviceKvCache>,
    terminal_lineage: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
    history: M1RearmRoundHistoryV1,
) -> Result<M1ScheduledLongLivedQueueRearmV1, M1LongLivedQueueRearmScheduleFailureV1> {
    let result = schedule_m1_long_lived_queue_rearm_inner_v1(
        engine,
        released,
        parked_lineage,
        terminal_lineage,
        history,
        M1LongLivedQueueRearmDispatchV1::Automatic,
    );
    finish_schedule_transition(engine, result)
}

fn schedule_m1_long_lived_queue_rearm_exact_with_lineage_v1<const C: usize>(
    engine: &mut Engine<C>,
    released: M1ReleasedCompletedStepV1,
    parked_lineage: Vec<ActiveDeviceKvCache>,
    terminal_lineage: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
    history: M1RearmRoundHistoryV1,
    expected_epoch: CompletionEpoch,
    requests: &[RequestId],
) -> Result<M1ScheduledLongLivedQueueRearmV1, M1LongLivedQueueRearmScheduleFailureV1> {
    let result = schedule_m1_long_lived_queue_rearm_inner_v1(
        engine,
        released,
        parked_lineage,
        terminal_lineage,
        history,
        M1LongLivedQueueRearmDispatchV1::Exact {
            expected_epoch,
            requests,
        },
    );
    finish_schedule_transition(engine, result)
}

fn schedule_m1_long_lived_queue_rearm_inner_v1<const C: usize>(
    engine: &mut Engine<C>,
    released: M1ReleasedCompletedStepV1,
    parked_lineage: Vec<ActiveDeviceKvCache>,
    terminal_lineage: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
    history: M1RearmRoundHistoryV1,
    dispatch: M1LongLivedQueueRearmDispatchV1<'_>,
) -> Result<
    M1ScheduledLongLivedQueueRearmV1,
    (
        M1LongLivedQueueRearmSchedulePhaseV1,
        M1LongLivedQueueRearmScheduleFailureV1,
    ),
> {
    if let Err(error) = validate_rearm_round_history_schedule_capacity(&history) {
        return Err(schedule_phase_failure(
            M1LongLivedQueueRearmSchedulePhaseV1::Released,
            error,
            ScheduleFailureCustodyV1::ReleasedWithLineage {
                released: Box::new(released),
                parked: parked_lineage,
                terminal: terminal_lineage,
                history: Box::new(history),
            },
        ));
    }
    let shape = released.queue().shape();
    let selection = released.queue().custody().selection();
    if let Err(error) = validate_rearm_eligibility(
        shape,
        selection,
        released
            .queue()
            .custody()
            .completion_output()
            .qualification_logits()
            .is_some(),
    ) {
        return Err(schedule_phase_failure(
            M1LongLivedQueueRearmSchedulePhaseV1::Released,
            error,
            ScheduleFailureCustodyV1::ReleasedWithLineage {
                released: Box::new(released),
                parked: parked_lineage,
                terminal: terminal_lineage,
                history: Box::new(history),
            },
        ));
    }
    if !released
        .members()
        .iter()
        .any(|member| matches!(member, M1ReleasedDeviceKvMemberV1::Active(_)))
        && parked_lineage.is_empty()
    {
        return Err(schedule_phase_failure(
            M1LongLivedQueueRearmSchedulePhaseV1::Released,
            M1LongLivedQueueRearmScheduleErrorV1::NoContinuingRequests,
            ScheduleFailureCustodyV1::ReleasedWithLineage {
                released: Box::new(released),
                parked: parked_lineage,
                terminal: terminal_lineage,
                history: Box::new(history),
            },
        ));
    }

    if let M1LongLivedQueueRearmDispatchV1::Exact {
        expected_epoch,
        requests,
    } = dispatch
    {
        if let Err(error) = validate_exact_rearm_preflight(
            engine,
            &released,
            &parked_lineage,
            expected_epoch,
            requests,
        ) {
            return Err(schedule_phase_failure(
                M1LongLivedQueueRearmSchedulePhaseV1::Released,
                error,
                ScheduleFailureCustodyV1::ReleasedWithLineage {
                    released: Box::new(released),
                    parked: parked_lineage,
                    terminal: terminal_lineage,
                    history: Box::new(history),
                },
            ));
        }
    }

    let additional_members = parked_lineage.len() + terminal_lineage.len();
    let mut released = released;
    if released
        .try_reserve_rearm_members(additional_members)
        .is_err()
    {
        return Err(schedule_phase_failure(
            M1LongLivedQueueRearmSchedulePhaseV1::Released,
            M1LongLivedQueueRearmScheduleErrorV1::HostAllocation,
            ScheduleFailureCustodyV1::ReleasedWithLineage {
                released: Box::new(released),
                parked: parked_lineage,
                terminal: terminal_lineage,
                history: Box::new(history),
            },
        ));
    }

    let (
        queue,
        checked,
        mut members,
        logical_accepted_counts,
        externally_published_counts,
        release_counts,
        completed_members,
        total_released,
    ) = released.into_rearm_parts();
    members.extend(
        parked_lineage
            .into_iter()
            .map(M1ReleasedDeviceKvMemberV1::Active),
    );
    members.extend(
        terminal_lineage
            .into_iter()
            .map(M1ReleasedDeviceKvMemberV1::Terminal),
    );
    let residue = ReleasedStepResidueV1 {
        checked,
        members,
        logical_accepted_counts,
        externally_published_counts,
        release_counts,
        completed_members,
        total_released,
        history,
    };
    let queue = match queue.detach() {
        Ok(queue) => queue,
        Err(queue) => {
            return Err(schedule_phase_failure(
                M1LongLivedQueueRearmSchedulePhaseV1::QueueDetach,
                M1LongLivedQueueRearmScheduleErrorV1::Detach,
                ScheduleFailureCustodyV1::Detach {
                    queue: Box::new(queue),
                    residue: Box::new(residue),
                },
            ));
        }
    };
    let scheduled = match dispatch_m1_long_lived_queue_rearm_v1(engine, dispatch) {
        Ok(scheduled) => scheduled,
        Err(error) => {
            let (error, scheduler_error) = match error {
                M1LongLivedQueueRearmDispatchFailureV1::EmptyAutomatic => (
                    M1LongLivedQueueRearmScheduleErrorV1::EmptySchedulerBatch,
                    None,
                ),
                M1LongLivedQueueRearmDispatchFailureV1::Automatic(error) => {
                    (M1LongLivedQueueRearmScheduleErrorV1::Scheduler, Some(error))
                }
                M1LongLivedQueueRearmDispatchFailureV1::Exact(error) => (
                    M1LongLivedQueueRearmScheduleErrorV1::ExactScheduler(error),
                    None,
                ),
            };
            return Err(schedule_phase_failure(
                M1LongLivedQueueRearmSchedulePhaseV1::Detached,
                error,
                ScheduleFailureCustodyV1::Detached {
                    queue: Box::new(queue),
                    residue: Box::new(residue),
                    scheduler_error,
                    scheduled: None,
                },
            ));
        }
    };

    let Some(expected_epoch) = exact_next_epoch(residue.checked.epoch()) else {
        return Err(schedule_phase_failure(
            M1LongLivedQueueRearmSchedulePhaseV1::PostDispatch,
            M1LongLivedQueueRearmScheduleErrorV1::EpochExhausted,
            ScheduleFailureCustodyV1::Detached {
                queue: Box::new(queue),
                residue: Box::new(residue),
                scheduler_error: None,
                scheduled: Some(Box::new(scheduled)),
            },
        ));
    };
    if scheduled.epoch() != expected_epoch {
        let actual = scheduled.epoch();
        return Err(schedule_phase_failure(
            M1LongLivedQueueRearmSchedulePhaseV1::PostDispatch,
            M1LongLivedQueueRearmScheduleErrorV1::EpochNotExactNext {
                expected: expected_epoch,
                actual,
            },
            ScheduleFailureCustodyV1::Detached {
                queue: Box::new(queue),
                residue: Box::new(residue),
                scheduler_error: None,
                scheduled: Some(Box::new(scheduled)),
            },
        ));
    }

    let mut scheduled_requests = Vec::new();
    if scheduled_requests
        .try_reserve_exact(scheduled.member_count())
        .is_err()
    {
        return Err(schedule_phase_failure(
            M1LongLivedQueueRearmSchedulePhaseV1::PostDispatch,
            M1LongLivedQueueRearmScheduleErrorV1::HostAllocation,
            ScheduleFailureCustodyV1::Detached {
                queue: Box::new(queue),
                residue: Box::new(residue),
                scheduler_error: None,
                scheduled: Some(Box::new(scheduled)),
            },
        ));
    }
    for lane in 0..scheduled.member_count() {
        let Some(request) = scheduled.member(lane) else {
            return Err(schedule_phase_failure(
                M1LongLivedQueueRearmSchedulePhaseV1::PostDispatch,
                M1LongLivedQueueRearmScheduleErrorV1::MalformedSchedulerBatch { lane },
                ScheduleFailureCustodyV1::Detached {
                    queue: Box::new(queue),
                    residue: Box::new(residue),
                    scheduler_error: None,
                    scheduled: Some(Box::new(scheduled)),
                },
            ));
        };
        scheduled_requests.push(request);
    }
    let mut available_requests = Vec::new();
    if available_requests
        .try_reserve_exact(residue.members.len())
        .is_err()
    {
        return Err(schedule_phase_failure(
            M1LongLivedQueueRearmSchedulePhaseV1::PostDispatch,
            M1LongLivedQueueRearmScheduleErrorV1::HostAllocation,
            ScheduleFailureCustodyV1::Detached {
                queue: Box::new(queue),
                residue: Box::new(residue),
                scheduler_error: None,
                scheduled: Some(Box::new(scheduled)),
            },
        ));
    }
    available_requests.extend(residue.members.iter().filter_map(|member| match member {
        M1ReleasedDeviceKvMemberV1::Active(cache) => Some(cache.projection().request),
        M1ReleasedDeviceKvMemberV1::Terminal(_) => None,
    }));
    if let Err(error) = validate_request_partition(&available_requests, &scheduled_requests) {
        return Err(schedule_phase_failure(
            M1LongLivedQueueRearmSchedulePhaseV1::PostDispatch,
            error,
            ScheduleFailureCustodyV1::Detached {
                queue: Box::new(queue),
                residue: Box::new(residue),
                scheduler_error: None,
                scheduled: Some(Box::new(scheduled)),
            },
        ));
    }

    let mut selected_slots = Vec::new();
    let mut selected = Vec::new();
    let mut parked = Vec::new();
    let mut terminal = Vec::new();
    if selected_slots
        .try_reserve_exact(scheduled.member_count())
        .is_err()
        || selected
            .try_reserve_exact(scheduled.member_count())
            .is_err()
        || parked.try_reserve_exact(residue.members.len()).is_err()
        || terminal.try_reserve_exact(residue.members.len()).is_err()
    {
        return Err(schedule_phase_failure(
            M1LongLivedQueueRearmSchedulePhaseV1::PostDispatch,
            M1LongLivedQueueRearmScheduleErrorV1::HostAllocation,
            ScheduleFailureCustodyV1::Detached {
                queue: Box::new(queue),
                residue: Box::new(residue),
                scheduler_error: None,
                scheduled: Some(Box::new(scheduled)),
            },
        ));
    }
    selected_slots.resize_with(scheduled.member_count(), || None);
    for member in residue.members {
        match member {
            M1ReleasedDeviceKvMemberV1::Active(cache) => {
                let request = cache.projection().request;
                if let Some(lane) = scheduled_requests
                    .iter()
                    .position(|scheduled| *scheduled == request)
                {
                    selected_slots[lane] = Some(cache);
                } else {
                    parked.push(cache);
                }
            }
            M1ReleasedDeviceKvMemberV1::Terminal(observation) => terminal.push(observation),
        }
    }
    selected.extend(selected_slots.into_iter().flatten());

    Ok(M1ScheduledLongLivedQueueRearmV1 {
        queue,
        scheduled,
        selected,
        parked,
        terminal,
        prior_checked: residue.checked,
        logical_accepted_counts: residue.logical_accepted_counts,
        externally_published_counts: residue.externally_published_counts,
        release_counts: residue.release_counts,
        completed_members: residue.completed_members,
        total_released: residue.total_released,
        history: residue.history,
    })
}

#[derive(Debug)]
struct ScheduledRemainderV1 {
    queue: M1PhysicalReadbackDetachedQueueSessionV1,
    selected: Vec<ActiveDeviceKvCache>,
    parked: Vec<ActiveDeviceKvCache>,
    terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
    prior_checked: crate::M1CheckedCompletionOutputV1,
    logical_accepted_counts: Box<[u32]>,
    externally_published_counts: Box<[u32]>,
    release_counts: Box<[M1CompletedKvPageReleaseCountsV1]>,
    completed_members: usize,
    total_released: usize,
    history: M1RearmRoundHistoryV1,
}

/// Fresh page leases and validated workspace inputs for the next same-shape round.
#[must_use = "KV reservation inputs contain linear page leases"]
#[derive(Debug, Eq, PartialEq)]
pub enum M1LongLivedQueueRearmKvInputsV1 {
    TargetOnly {
        target: ferric_spec::ValidatedM1StepInputs,
        target_page_leases: Vec<Vec<crate::DeviceKvPageLease>>,
    },
    QualificationTargetOnly {
        target: ferric_spec::ValidatedM1StepInputs,
        contexts: Vec<crate::M1ValidatedQualificationContextStepV1>,
    },
    SpeculativeRound {
        draft_decode: ferric_spec::ValidatedM1StepInputs,
        target_speculative: ferric_spec::ValidatedM1StepInputs,
        draft_page_leases: Vec<Vec<crate::DeviceKvPageLease>>,
        target_page_leases: Vec<Vec<crate::DeviceKvPageLease>>,
    },
}

impl M1LongLivedQueueRearmKvInputsV1 {
    pub const fn target_only(
        target: ferric_spec::ValidatedM1StepInputs,
        target_page_leases: Vec<Vec<crate::DeviceKvPageLease>>,
    ) -> Self {
        Self::TargetOnly {
            target,
            target_page_leases,
        }
    }

    /// Binds one C8192 teacher-forced or terminal step to the exact validated
    /// context witnesses whose attached cache reserves own all future pages.
    pub const fn qualification_target_only(
        target: ferric_spec::ValidatedM1StepInputs,
        contexts: Vec<crate::M1ValidatedQualificationContextStepV1>,
    ) -> Self {
        Self::QualificationTargetOnly { target, contexts }
    }

    pub const fn speculative_round(
        draft_decode: ferric_spec::ValidatedM1StepInputs,
        target_speculative: ferric_spec::ValidatedM1StepInputs,
        draft_page_leases: Vec<Vec<crate::DeviceKvPageLease>>,
        target_page_leases: Vec<Vec<crate::DeviceKvPageLease>>,
    ) -> Self {
        Self::SpeculativeRound {
            draft_decode,
            target_speculative,
            draft_page_leases,
            target_page_leases,
        }
    }
}

/// Closed reservation stage used before workspace-image preparation.
#[must_use = "reserved caches and KV tables must proceed together"]
#[derive(Debug)]
pub struct M1ReservedLongLivedQueueRearmV1 {
    scheduled: M1ScheduledLongLivedQueueRearmV1,
    tables: M1FullStepKvWorkspaceTablesV1,
}

impl M1ReservedLongLivedQueueRearmV1 {
    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.scheduled.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.scheduled.history.get(index)
    }
}

/// Reservation/binding stage for an explicitly terminal closed failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1LongLivedQueueRearmKvReservationPhaseV1 {
    Preflight,
    DraftReservation,
    TargetReservation,
    DraftTableBinding,
    TargetTableBinding,
}

/// Fail-stop custody after next-round KV reservation or table binding rejects.
///
/// The first version conservatively classifies every such failure terminal,
/// including pure preflight rejection, so no caller can incorrectly retry a
/// partially installed cache marker. All supplied leases, reservations,
/// selected/parked caches, and the detached queue remain retained internally.
#[must_use = "terminal reservation failure requires process-level quarantine"]
#[derive(Debug)]
pub struct M1LongLivedQueueRearmKvReservationFailureV1 {
    phase: M1LongLivedQueueRearmKvReservationPhaseV1,
    retained: OpaqueRearmCustodyV1<'static>,
}

impl M1LongLivedQueueRearmKvReservationFailureV1 {
    #[must_use]
    pub const fn phase(&self) -> M1LongLivedQueueRearmKvReservationPhaseV1 {
        self.phase
    }

    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        true
    }

    #[must_use]
    pub fn retains_custody(&self) -> bool {
        let _ = &self.retained.0;
        true
    }
}

fn kv_reservation_failure(
    phase: M1LongLivedQueueRearmKvReservationPhaseV1,
    retained: impl fmt::Debug + 'static,
) -> M1LongLivedQueueRearmKvReservationFailureV1 {
    M1LongLivedQueueRearmKvReservationFailureV1 {
        phase,
        retained: OpaqueRearmCustodyV1(Box::new(retained)),
    }
}

fn input_roster_matches(
    scheduled: &M1ScheduledLongLivedQueueRearmV1,
    inputs: &ferric_spec::ValidatedM1StepInputs,
) -> bool {
    let Ok(live) = usize::try_from(inputs.live_lane_count()) else {
        return false;
    };
    live == scheduled.selected.len()
        && inputs.selection() == scheduled.queue.custody().selection()
        && inputs
            .lanes()
            .iter()
            .take(live)
            .zip(&scheduled.selected)
            .all(|(plan, cache)| {
                plan.is_some_and(|plan| {
                    plan.request() == cache.projection().request
                        && plan.completion_epoch() == scheduled.scheduled.epoch()
                })
            })
}

fn qualification_logits_preflight(
    selection: Qwen3PlanSelection,
    logits: Option<&crate::BoundM1QualificationLogitsV1>,
) -> bool {
    let Some(logits) = logits else {
        return false;
    };
    let Ok(expected) = crate::m1_qualification_logits_shape_v1(selection) else {
        return false;
    };
    logits.shape() == expected
        && logits.retained_host_dispatch_range().extent_bytes() == expected.extent_bytes()
}

/// Installs exact next-epoch KV reservations inside the retained selected caches
/// and binds the closed target-only or speculative workspace tables.
///
/// # Errors
///
/// Returns explicitly terminal closed custody for shape/roster drift, page
/// reservation failure, table binding rejection, or host reservation failure.
fn reserve_m1_long_lived_queue_rearm_kv_inner_v1(
    mut scheduled: M1ScheduledLongLivedQueueRearmV1,
    inputs: M1LongLivedQueueRearmKvInputsV1,
) -> Result<M1ReservedLongLivedQueueRearmV1, M1LongLivedQueueRearmKvReservationFailureV1> {
    match inputs {
        M1LongLivedQueueRearmKvInputsV1::TargetOnly {
            target,
            mut target_page_leases,
        } => {
            if scheduled.queue.shape() != M1PhysicalFixedBatchShapeV1::TargetOnly
                || !input_roster_matches(&scheduled, &target)
                || target_page_leases.len() != scheduled.selected.len()
            {
                return Err(kv_reservation_failure(
                    M1LongLivedQueueRearmKvReservationPhaseV1::Preflight,
                    (scheduled, target, target_page_leases),
                ));
            }
            let mut reservations = Vec::new();
            if reservations
                .try_reserve_exact(scheduled.selected.len())
                .is_err()
            {
                return Err(kv_reservation_failure(
                    M1LongLivedQueueRearmKvReservationPhaseV1::Preflight,
                    (scheduled, target, target_page_leases),
                ));
            }
            for lane in 0..scheduled.selected.len() {
                let request = scheduled.selected[lane].projection().request;
                let leases = core::mem::take(&mut target_page_leases[lane]);
                match scheduled.selected[lane].reserve_step_write(
                    request,
                    ferric_spec::Qwen3ModelRole::Target8B,
                    target.context_lengths()[lane],
                    target.active_lengths()[lane],
                    scheduled.scheduled.epoch(),
                    leases,
                ) {
                    Ok(reservation) => reservations.push(reservation),
                    Err(failure) => {
                        let (error, leases) = (*failure).into_parts();
                        target_page_leases[lane] = leases;
                        return Err(kv_reservation_failure(
                            M1LongLivedQueueRearmKvReservationPhaseV1::TargetReservation,
                            (
                                scheduled,
                                target,
                                target_page_leases,
                                reservations,
                                lane,
                                error,
                            ),
                        ));
                    }
                }
            }
            let target = match crate::bind_m1_kv_workspace_table_v1(target, reservations) {
                Ok(table) => table,
                Err(failure) => {
                    return Err(kv_reservation_failure(
                        M1LongLivedQueueRearmKvReservationPhaseV1::TargetTableBinding,
                        (scheduled, target_page_leases, failure),
                    ));
                }
            };
            Ok(M1ReservedLongLivedQueueRearmV1 {
                scheduled,
                tables: M1FullStepKvWorkspaceTablesV1::TargetOnly { target },
            })
        }
        M1LongLivedQueueRearmKvInputsV1::QualificationTargetOnly { target, contexts } => {
            let custody = scheduled.queue.custody();
            if scheduled.queue.shape() != M1PhysicalFixedBatchShapeV1::TargetOnly
                || !input_roster_matches(&scheduled, &target)
                || contexts.len() != scheduled.selected.len()
                || !qualification_logits_preflight(
                    custody.selection(),
                    custody.completion_output().qualification_logits(),
                )
            {
                return Err(kv_reservation_failure(
                    M1LongLivedQueueRearmKvReservationPhaseV1::Preflight,
                    (scheduled, target, contexts),
                ));
            }
            let mut reservations = Vec::new();
            if reservations
                .try_reserve_exact(scheduled.selected.len())
                .is_err()
            {
                return Err(kv_reservation_failure(
                    M1LongLivedQueueRearmKvReservationPhaseV1::Preflight,
                    (scheduled, target, contexts),
                ));
            }
            for (lane, (cache, context)) in scheduled
                .selected
                .iter_mut()
                .zip(contexts.iter().copied())
                .enumerate()
            {
                let request = cache.projection().request;
                match cache.reserve_m1_qualification_context_step_write_v1(
                    request,
                    u32::try_from(lane).unwrap_or(u32::MAX),
                    context,
                    scheduled.scheduled.epoch(),
                ) {
                    Ok(reservation) => reservations.push(reservation.into_pending_step_write()),
                    Err(failure) => {
                        return Err(kv_reservation_failure(
                            M1LongLivedQueueRearmKvReservationPhaseV1::TargetReservation,
                            (scheduled, target, contexts, reservations, lane, failure),
                        ));
                    }
                }
            }
            let target = match crate::bind_m1_kv_workspace_table_v1(target, reservations) {
                Ok(table) => table,
                Err(failure) => {
                    return Err(kv_reservation_failure(
                        M1LongLivedQueueRearmKvReservationPhaseV1::TargetTableBinding,
                        (scheduled, contexts, failure),
                    ));
                }
            };
            Ok(M1ReservedLongLivedQueueRearmV1 {
                scheduled,
                tables: M1FullStepKvWorkspaceTablesV1::TargetOnly { target },
            })
        }
        M1LongLivedQueueRearmKvInputsV1::SpeculativeRound {
            draft_decode,
            target_speculative,
            mut draft_page_leases,
            mut target_page_leases,
        } => {
            let speculative_shape = matches!(
                scheduled.queue.shape(),
                M1PhysicalFixedBatchShapeV1::SpeculativeK4
                    | M1PhysicalFixedBatchShapeV1::SpeculativeK8
                    | M1PhysicalFixedBatchShapeV1::SpeculativeK16
            );
            let draft_live = usize::try_from(draft_decode.live_lane_count()).ok();
            if !speculative_shape
                || !input_roster_matches(&scheduled, &target_speculative)
                || draft_live != Some(scheduled.selected.len())
                || draft_page_leases.len() != scheduled.selected.len()
                || target_page_leases.len() != scheduled.selected.len()
            {
                return Err(kv_reservation_failure(
                    M1LongLivedQueueRearmKvReservationPhaseV1::Preflight,
                    (
                        scheduled,
                        draft_decode,
                        target_speculative,
                        draft_page_leases,
                        target_page_leases,
                    ),
                ));
            }
            let draft_selection = draft_decode.selection();
            let target_selection = target_speculative.selection();
            let draft_roster_matches = draft_decode
                .lanes()
                .iter()
                .take(scheduled.selected.len())
                .zip(&scheduled.selected)
                .all(|(plan, cache)| {
                    plan.is_some_and(|plan| {
                        plan.request() == cache.projection().request
                            && plan.completion_epoch() == scheduled.scheduled.epoch()
                    })
                });
            if !draft_roster_matches {
                return Err(kv_reservation_failure(
                    M1LongLivedQueueRearmKvReservationPhaseV1::Preflight,
                    (
                        scheduled,
                        draft_decode,
                        target_speculative,
                        draft_page_leases,
                        target_page_leases,
                    ),
                ));
            }
            let mut draft_reservations = Vec::new();
            let mut target_reservations = Vec::new();
            if draft_reservations
                .try_reserve_exact(scheduled.selected.len())
                .is_err()
                || target_reservations
                    .try_reserve_exact(scheduled.selected.len())
                    .is_err()
            {
                return Err(kv_reservation_failure(
                    M1LongLivedQueueRearmKvReservationPhaseV1::Preflight,
                    (
                        scheduled,
                        draft_decode,
                        target_speculative,
                        draft_page_leases,
                        target_page_leases,
                    ),
                ));
            }
            for lane in 0..scheduled.selected.len() {
                let request = scheduled.selected[lane].projection().request;
                let leases = core::mem::take(&mut draft_page_leases[lane]);
                match scheduled.selected[lane].reserve_speculative_draft_round_write(
                    request,
                    target_selection,
                    draft_selection,
                    draft_decode.context_lengths()[lane],
                    scheduled.scheduled.epoch(),
                    leases,
                ) {
                    Ok(reservation) => draft_reservations.push(reservation),
                    Err(failure) => {
                        let (error, leases) = (*failure).into_parts();
                        draft_page_leases[lane] = leases;
                        return Err(kv_reservation_failure(
                            M1LongLivedQueueRearmKvReservationPhaseV1::DraftReservation,
                            (
                                scheduled,
                                draft_decode,
                                target_speculative,
                                draft_page_leases,
                                target_page_leases,
                                draft_reservations,
                                lane,
                                error,
                            ),
                        ));
                    }
                }
            }
            for lane in 0..scheduled.selected.len() {
                let request = scheduled.selected[lane].projection().request;
                let leases = core::mem::take(&mut target_page_leases[lane]);
                match scheduled.selected[lane].reserve_step_write(
                    request,
                    ferric_spec::Qwen3ModelRole::Target8B,
                    target_speculative.context_lengths()[lane],
                    target_speculative.active_lengths()[lane],
                    scheduled.scheduled.epoch(),
                    leases,
                ) {
                    Ok(reservation) => target_reservations.push(reservation),
                    Err(failure) => {
                        let (error, leases) = (*failure).into_parts();
                        target_page_leases[lane] = leases;
                        return Err(kv_reservation_failure(
                            M1LongLivedQueueRearmKvReservationPhaseV1::TargetReservation,
                            (
                                scheduled,
                                draft_decode,
                                target_speculative,
                                draft_page_leases,
                                target_page_leases,
                                draft_reservations,
                                target_reservations,
                                lane,
                                error,
                            ),
                        ));
                    }
                }
            }
            let target =
                match crate::bind_m1_kv_workspace_table_v1(target_speculative, target_reservations)
                {
                    Ok(table) => table,
                    Err(failure) => {
                        return Err(kv_reservation_failure(
                            M1LongLivedQueueRearmKvReservationPhaseV1::TargetTableBinding,
                            (
                                scheduled,
                                draft_decode,
                                draft_page_leases,
                                target_page_leases,
                                draft_reservations,
                                failure,
                            ),
                        ));
                    }
                };
            let draft_decode = match crate::bind_m1_speculative_draft_kv_round_workspace_table_v1(
                target_selection,
                draft_decode,
                draft_reservations,
            ) {
                Ok(table) => table,
                Err(failure) => {
                    return Err(kv_reservation_failure(
                        M1LongLivedQueueRearmKvReservationPhaseV1::DraftTableBinding,
                        (
                            scheduled,
                            target,
                            draft_page_leases,
                            target_page_leases,
                            failure,
                        ),
                    ));
                }
            };
            Ok(M1ReservedLongLivedQueueRearmV1 {
                scheduled,
                tables: M1FullStepKvWorkspaceTablesV1::SpeculativeRound {
                    draft_decode,
                    target_speculative: target,
                },
            })
        }
    }
}

/// Installs exact next-round reservations and faults the in-flight Engine on
/// any closed failure.
///
/// # Errors
///
/// Returns terminal retained custody after permanently faulting `engine`.
pub fn reserve_m1_long_lived_queue_rearm_kv_v1<const C: usize>(
    engine: &mut Engine<C>,
    scheduled: M1ScheduledLongLivedQueueRearmV1,
    inputs: M1LongLivedQueueRearmKvInputsV1,
) -> Result<M1ReservedLongLivedQueueRearmV1, M1LongLivedQueueRearmKvReservationFailureV1> {
    match reserve_m1_long_lived_queue_rearm_kv_inner_v1(scheduled, inputs) {
        Ok(reserved) => Ok(reserved),
        Err(failure) => {
            engine.quarantine_m1_queue_rearm_failure();
            Err(failure)
        }
    }
}

/// Preparation rejection retaining the detached queue, every cache, and all
/// scheduler/KV/image failure custody.
#[must_use = "prepared-rearm rejection retains all linear authority"]
#[derive(Debug)]
pub struct M1LongLivedQueueRearmPrepareFailureV1 {
    source: M1PrepareFailureV1,
    remainder: Box<ScheduledRemainderV1>,
}

impl M1LongLivedQueueRearmPrepareFailureV1 {
    pub const fn source(&self) -> &M1PrepareFailureV1 {
        &self.source
    }

    #[must_use]
    pub fn retained_cache_count(&self) -> usize {
        self.remainder.selected.len() + self.remainder.parked.len()
    }

    /// Preparation rejection permanently faults the in-flight Engine.
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        true
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.remainder.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.remainder.history.get(index)
    }
}

/// Fresh scheduler-bound workspace bytes retained beside the detached queue.
#[must_use = "prepared rearm custody must replace workspaces and submit or remain retained"]
#[derive(Debug)]
pub struct M1PreparedLongLivedQueueRearmV1 {
    prepared: M1PreparedScheduledWorkspaceImagesV1,
    remainder: ScheduledRemainderV1,
}

impl M1PreparedLongLivedQueueRearmV1 {
    #[must_use]
    pub const fn kind(&self) -> M1FullStepWorkspaceInputKind {
        self.prepared.kind()
    }

    #[must_use]
    pub const fn next_epoch(&self) -> CompletionEpoch {
        self.prepared.step().scheduled_dispatch().epoch()
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.remainder.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.remainder.history.get(index)
    }
}

/// Joins next-step plans/tables to the once-selected scheduler roster.
///
/// # Errors
///
/// Returns closed preparation failure retaining the detached physical queue,
/// every selected and parked cache, scheduler custody, KV reservations, plans,
/// tables, and any composed workspace residue.
fn prepare_m1_long_lived_queue_rearm_inner_v1(
    reserved: M1ReservedLongLivedQueueRearmV1,
    runner: &LogicalRunnerDeclaration,
    plans: M1FullStepWorkspacePlans,
) -> Result<M1PreparedLongLivedQueueRearmV1, Box<M1LongLivedQueueRearmPrepareFailureV1>> {
    let M1ReservedLongLivedQueueRearmV1 { scheduled, tables } = reserved;
    let M1ScheduledLongLivedQueueRearmV1 {
        queue,
        scheduled,
        selected,
        parked,
        terminal,
        prior_checked,
        logical_accepted_counts,
        externally_published_counts,
        release_counts,
        completed_members,
        total_released,
        history,
    } = scheduled;
    let remainder = ScheduledRemainderV1 {
        queue,
        selected,
        parked,
        terminal,
        prior_checked,
        logical_accepted_counts,
        externally_published_counts,
        release_counts,
        completed_members,
        total_released,
        history,
    };
    match prepare_m1_scheduled_workspace_images_v1(scheduled, runner, plans, tables) {
        Ok(prepared) => Ok(M1PreparedLongLivedQueueRearmV1 {
            prepared,
            remainder,
        }),
        Err(source) => Err(Box::new(M1LongLivedQueueRearmPrepareFailureV1 {
            source,
            remainder: Box::new(remainder),
        })),
    }
}

/// Prepares fresh images or permanently faults the in-flight Engine while
/// retaining all detached queue, cache, reservation, and image custody.
///
/// # Errors
///
/// Returns terminal preparation custody after permanently faulting `engine`.
pub fn prepare_m1_long_lived_queue_rearm_v1<const C: usize>(
    engine: &mut Engine<C>,
    reserved: M1ReservedLongLivedQueueRearmV1,
    runner: &LogicalRunnerDeclaration,
    plans: M1FullStepWorkspacePlans,
) -> Result<M1PreparedLongLivedQueueRearmV1, Box<M1LongLivedQueueRearmPrepareFailureV1>> {
    match prepare_m1_long_lived_queue_rearm_inner_v1(reserved, runner, plans) {
        Ok(prepared) => Ok(prepared),
        Err(failure) => {
            engine.quarantine_m1_queue_rearm_failure();
            Err(failure)
        }
    }
}

/// Stable submission stage for a closed rearm failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1LongLivedQueueRearmSubmissionPhaseV1 {
    Preflight,
    DraftWorkspaceReplacement,
    TargetWorkspaceReplacement,
    DirectDiagnosticChoiceReplacement,
    SpeculativeDraftChoiceReplacement,
    SpeculativeTargetChoiceReplacement,
    WorkspaceRangeRebinding,
    RolloverOutputActivation,
    FixedBatchRebuild,
    QueueBind,
    QueueRollover,
    QueueObservation,
    QueueSubmit,
}

#[derive(Debug)]
struct OpaqueRearmCustodyV1<'a>(Box<dyn fmt::Debug + 'a>);

/// Closed failure retaining every generic/Ferric owner available at its stage.
#[must_use = "failed rearm custody must remain retained or be explicitly torn down"]
#[derive(Debug)]
pub struct M1LongLivedQueueRearmSubmissionFailureV1<'a> {
    phase: M1LongLivedQueueRearmSubmissionPhaseV1,
    retained: OpaqueRearmCustodyV1<'a>,
}

impl M1LongLivedQueueRearmSubmissionFailureV1<'_> {
    #[must_use]
    pub const fn phase(&self) -> M1LongLivedQueueRearmSubmissionPhaseV1 {
        self.phase
    }

    #[must_use]
    pub fn retains_custody(&self) -> bool {
        let _ = &self.retained.0;
        true
    }

    /// Submission rejection permanently faults the in-flight Engine.
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        true
    }
}

fn submission_failure<'a>(
    phase: M1LongLivedQueueRearmSubmissionPhaseV1,
    retained: impl fmt::Debug + 'a,
) -> M1LongLivedQueueRearmSubmissionFailureV1<'a> {
    M1LongLivedQueueRearmSubmissionFailureV1 {
        phase,
        retained: OpaqueRearmCustodyV1(Box::new(retained)),
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct FreshWorkspaceRangeV1 {
    pub(crate) workspace: M1FullStepWorkspaceRole,
    pub(crate) semantic: M1StepWorkspaceRange,
    pub(crate) dispatch: ServiceDeviceDispatchRangeV1,
}

pub(crate) fn member_layout<const N: usize>(
    plan: &AddresslessM1StepWorkspacePlan,
) -> [(u64, u64, u64); N] {
    core::array::from_fn(|index| {
        let range = plan.ranges()[index];
        (range.offset(), range.byte_len(), range.alignment())
    })
}

pub(crate) fn append_workspace_ranges<const N: usize>(
    destination: &mut Vec<FreshWorkspaceRangeV1>,
    workspace: M1FullStepWorkspaceRole,
    owner: &BoundM1StepWorkspaceSubleases<N>,
    ranges: [ServiceDeviceDispatchRangeV1; N],
) {
    destination.extend(ranges.into_iter().enumerate().map(|(index, dispatch)| {
        FreshWorkspaceRangeV1 {
            workspace,
            semantic: owner.plan().ranges()[index],
            dispatch,
        }
    }));
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RearmRangeRequestV1 {
    FreshWorkspace(M1FullStepWorkspaceRole, M1StepWorkspaceRange),
    RetainedCompletionOutput,
    RetainedQualificationLogits,
    RetainedDirectDiagnosticChoices,
    RetainedSpeculativeDraftChoices,
    RetainedSpeculativeDraftChoice { iteration: u8 },
    RetainedSpeculativeTargetChoices,
    Unchanged,
}

#[derive(Clone, Copy, Debug)]
enum RetainedSemanticCaptureRangesV1<T> {
    Ordinary,
    Qualification {
        logits: T,
    },
    DirectDiagnostic {
        choices: T,
    },
    SpeculativeDiagnostic {
        draft: T,
        draft_tokens: u8,
        draft_rows: [Option<T>; crate::M1_SPECULATIVE_DIAGNOSTIC_MAX_DRAFT_TOKENS_V1],
        target: T,
    },
}

impl<T> RetainedSemanticCaptureRangesV1<T> {
    const fn qualification_enabled(&self) -> bool {
        matches!(self, Self::Qualification { .. })
    }

    const fn direct_diagnostic_enabled(&self) -> bool {
        matches!(self, Self::DirectDiagnostic { .. })
    }

    const fn speculative_diagnostic_enabled(&self) -> bool {
        matches!(self, Self::SpeculativeDiagnostic { .. })
    }
}

#[derive(Clone, Copy, Debug)]
struct RetainedCaptureRangesV1<T> {
    completion_output: T,
    semantic: RetainedSemanticCaptureRangesV1<T>,
}

impl<T> RetainedCaptureRangesV1<T> {
    fn map<U>(self, mut map: impl FnMut(T) -> U) -> RetainedCaptureRangesV1<U> {
        let semantic = match self.semantic {
            RetainedSemanticCaptureRangesV1::Ordinary => RetainedSemanticCaptureRangesV1::Ordinary,
            RetainedSemanticCaptureRangesV1::Qualification { logits } => {
                RetainedSemanticCaptureRangesV1::Qualification {
                    logits: map(logits),
                }
            }
            RetainedSemanticCaptureRangesV1::DirectDiagnostic { choices } => {
                RetainedSemanticCaptureRangesV1::DirectDiagnostic {
                    choices: map(choices),
                }
            }
            RetainedSemanticCaptureRangesV1::SpeculativeDiagnostic {
                draft,
                draft_tokens,
                draft_rows,
                target,
            } => RetainedSemanticCaptureRangesV1::SpeculativeDiagnostic {
                draft: map(draft),
                draft_tokens,
                draft_rows: draft_rows.map(|row| row.map(&mut map)),
                target: map(target),
            },
        };
        RetainedCaptureRangesV1 {
            completion_output: map(self.completion_output),
            semantic,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct RetainedHostCaptureRangesV1 {
    ranges: RetainedCaptureRangesV1<ServiceHostDispatchRangeV1>,
    completion_snapshot: Option<ServiceHostDispatchSnapshotRangeV1>,
}

fn retained_capture_range_request(
    source: M1PhysicalBufferSourceV1,
    semantic: &RetainedSemanticCaptureRangesV1<impl Copy>,
) -> Option<RearmRangeRequestV1> {
    let diagnostic_route = speculative_diagnostic_choice_source_route(
        source,
        semantic.speculative_diagnostic_enabled(),
        semantic.direct_diagnostic_enabled(),
    )
    .ok()?;
    match diagnostic_route {
        SpeculativeDiagnosticChoiceSourceRouteV1::DirectTargetWholeHost => {
            return Some(RearmRangeRequestV1::RetainedDirectDiagnosticChoices)
        }
        SpeculativeDiagnosticChoiceSourceRouteV1::DraftWholeHost => {
            return Some(RearmRangeRequestV1::RetainedSpeculativeDraftChoices)
        }
        SpeculativeDiagnosticChoiceSourceRouteV1::TargetWholeHost => {
            return Some(RearmRangeRequestV1::RetainedSpeculativeTargetChoices)
        }
        SpeculativeDiagnosticChoiceSourceRouteV1::DraftScalarHost { iteration, .. } => {
            return Some(RearmRangeRequestV1::RetainedSpeculativeDraftChoice { iteration })
        }
        SpeculativeDiagnosticChoiceSourceRouteV1::OrdinaryDevice => {}
    }
    match source {
        M1PhysicalBufferSourceV1::Workspace { workspace, range }
            if semantic.qualification_enabled()
                && workspace == M1FullStepWorkspaceRole::Target
                && range == ferric_build::M1StepWorkspaceRangeRole::Logits =>
        {
            Some(RearmRangeRequestV1::RetainedQualificationLogits)
        }
        M1PhysicalBufferSourceV1::CompletionOutput { .. } => {
            Some(RearmRangeRequestV1::RetainedCompletionOutput)
        }
        _ => None,
    }
}

fn requested_workspace_range(
    source: M1PhysicalBufferSourceV1,
    composition: &AddresslessM1FullStepWorkspaceComposition,
    semantic: &RetainedSemanticCaptureRangesV1<impl Copy>,
) -> Result<RearmRangeRequestV1, ()> {
    if let Some(request) = retained_capture_range_request(source, semantic) {
        return Ok(request);
    }
    match source {
        M1PhysicalBufferSourceV1::Workspace { workspace, range }
        | M1PhysicalBufferSourceV1::WorkspaceSentinel {
            workspace, range, ..
        }
        | M1PhysicalBufferSourceV1::SpeculativeDraftAnchorTokenIds {
            workspace, range, ..
        }
        | M1PhysicalBufferSourceV1::SpeculativeTargetTokenIds {
            workspace, range, ..
        } => composition
            .workspace_plans()
            .workspace(workspace)
            .and_then(|plan| plan.range(range))
            .map(|range| RearmRangeRequestV1::FreshWorkspace(workspace, range))
            .ok_or(()),
        M1PhysicalBufferSourceV1::SpeculativeDraftChoices(row) => Ok(
            RearmRangeRequestV1::FreshWorkspace(M1FullStepWorkspaceRole::Target, row.range()),
        ),
        M1PhysicalBufferSourceV1::SpeculativeDraftIterationMetadata {
            workspace,
            range,
            draft_segment,
            ..
        } => {
            let binding = composition.segment_binding(draft_segment).ok_or(())?;
            let row = match range {
                ferric_build::M1StepWorkspaceRangeRole::DraftPositionIds => {
                    binding.draft_position_ids_subrange().ok_or(())?.range()
                }
                ferric_build::M1StepWorkspaceRangeRole::DraftContextLengths => {
                    binding.draft_context_lengths_subrange().ok_or(())?.range()
                }
                _ => return Err(()),
            };
            Ok(RearmRangeRequestV1::FreshWorkspace(workspace, row))
        }
        M1PhysicalBufferSourceV1::CompletionOutput { .. } => Err(()),
        M1PhysicalBufferSourceV1::ModelWeight { .. }
        | M1PhysicalBufferSourceV1::KvCachePlane { .. } => Ok(RearmRangeRequestV1::Unchanged),
    }
}

fn select_rearm_completed_snapshot<T: Copy + Eq>(
    source: M1PhysicalBufferSourceV1,
    old: Option<T>,
    retained_completion: Option<T>,
) -> Result<Option<T>, ()> {
    if matches!(source, M1PhysicalBufferSourceV1::CompletionOutput { .. }) {
        (old == retained_completion)
            .then_some(retained_completion)
            .ok_or(())
    } else {
        old.is_none().then_some(None).ok_or(())
    }
}

fn resolve_fresh_workspace_range(
    workspace: M1FullStepWorkspaceRole,
    requested: M1StepWorkspaceRange,
    ranges: &[FreshWorkspaceRangeV1],
) -> Result<ServiceDeviceDispatchRangeV1, ()> {
    let parent = ranges
        .iter()
        .find(|entry| entry.workspace == workspace && entry.semantic.role() == requested.role())
        .ok_or(())?;
    let relative = requested
        .offset()
        .checked_sub(parent.semantic.offset())
        .ok_or(())?;
    parent
        .dispatch
        .checked_subrange(relative, requested.byte_len(), requested.alignment())
        .map_err(|_| ())
}

#[derive(Debug)]
struct RearmRangeSelectionV1 {
    completion_output_sources: usize,
    qualification_logits_sources: usize,
    direct_diagnostic_sources: usize,
    speculative_draft_sources: usize,
    speculative_draft_scalar_sources: usize,
    speculative_target_sources: usize,
    semantic: RetainedSemanticCaptureRangesV1<()>,
}

impl RearmRangeSelectionV1 {
    const fn new<T>(semantic: &RetainedSemanticCaptureRangesV1<T>) -> Self {
        let semantic = match semantic {
            RetainedSemanticCaptureRangesV1::Ordinary => RetainedSemanticCaptureRangesV1::Ordinary,
            RetainedSemanticCaptureRangesV1::Qualification { .. } => {
                RetainedSemanticCaptureRangesV1::Qualification { logits: () }
            }
            RetainedSemanticCaptureRangesV1::DirectDiagnostic { .. } => {
                RetainedSemanticCaptureRangesV1::DirectDiagnostic { choices: () }
            }
            RetainedSemanticCaptureRangesV1::SpeculativeDiagnostic { draft_tokens, .. } => {
                RetainedSemanticCaptureRangesV1::SpeculativeDiagnostic {
                    draft: (),
                    draft_tokens: *draft_tokens,
                    draft_rows: [None; crate::M1_SPECULATIVE_DIAGNOSTIC_MAX_DRAFT_TOKENS_V1],
                    target: (),
                }
            }
        };
        Self {
            completion_output_sources: 0,
            qualification_logits_sources: 0,
            direct_diagnostic_sources: 0,
            speculative_draft_sources: 0,
            speculative_draft_scalar_sources: 0,
            speculative_target_sources: 0,
            semantic,
        }
    }

    fn select<T: Copy + Eq>(
        &mut self,
        request: RearmRangeRequestV1,
        old: T,
        fresh: Option<T>,
        previous: RetainedCaptureRangesV1<T>,
        retained: RetainedCaptureRangesV1<T>,
    ) -> Result<T, ()> {
        match request {
            RearmRangeRequestV1::FreshWorkspace(_, _) => fresh.ok_or(()),
            RearmRangeRequestV1::RetainedCompletionOutput => {
                self.completion_output_sources += 1;
                (old == previous.completion_output)
                    .then_some(retained.completion_output)
                    .ok_or(())
            }
            RearmRangeRequestV1::RetainedQualificationLogits => {
                self.qualification_logits_sources += 1;
                let (
                    RetainedSemanticCaptureRangesV1::Qualification { logits: previous },
                    RetainedSemanticCaptureRangesV1::Qualification { logits: retained },
                ) = (&previous.semantic, &retained.semantic)
                else {
                    return Err(());
                };
                (old == *previous).then_some(*retained).ok_or(())
            }
            RearmRangeRequestV1::RetainedDirectDiagnosticChoices => {
                self.direct_diagnostic_sources += 1;
                let (
                    RetainedSemanticCaptureRangesV1::DirectDiagnostic { choices: previous },
                    RetainedSemanticCaptureRangesV1::DirectDiagnostic { choices: retained },
                ) = (&previous.semantic, &retained.semantic)
                else {
                    return Err(());
                };
                (old == *previous).then_some(*retained).ok_or(())
            }
            RearmRangeRequestV1::RetainedSpeculativeDraftChoices => {
                self.speculative_draft_sources += 1;
                let (
                    RetainedSemanticCaptureRangesV1::SpeculativeDiagnostic {
                        draft: previous, ..
                    },
                    RetainedSemanticCaptureRangesV1::SpeculativeDiagnostic {
                        draft: retained, ..
                    },
                ) = (&previous.semantic, &retained.semantic)
                else {
                    return Err(());
                };
                (old == *previous).then_some(*retained).ok_or(())
            }
            RearmRangeRequestV1::RetainedSpeculativeDraftChoice { iteration } => {
                self.speculative_draft_scalar_sources += 1;
                let (
                    RetainedSemanticCaptureRangesV1::SpeculativeDiagnostic {
                        draft_rows: previous,
                        ..
                    },
                    RetainedSemanticCaptureRangesV1::SpeculativeDiagnostic {
                        draft_rows: retained,
                        ..
                    },
                ) = (&previous.semantic, &retained.semantic)
                else {
                    return Err(());
                };
                let index = usize::from(iteration);
                let previous = previous.get(index).copied().flatten().ok_or(())?;
                let retained = retained.get(index).copied().flatten().ok_or(())?;
                (old == previous).then_some(retained).ok_or(())
            }
            RearmRangeRequestV1::RetainedSpeculativeTargetChoices => {
                self.speculative_target_sources += 1;
                let (
                    RetainedSemanticCaptureRangesV1::SpeculativeDiagnostic {
                        target: previous, ..
                    },
                    RetainedSemanticCaptureRangesV1::SpeculativeDiagnostic {
                        target: retained, ..
                    },
                ) = (&previous.semantic, &retained.semantic)
                else {
                    return Err(());
                };
                (old == *previous).then_some(*retained).ok_or(())
            }
            RearmRangeRequestV1::Unchanged => Ok(old),
        }
    }

    fn select_fresh<T: Copy>(
        &mut self,
        request: RearmRangeRequestV1,
        fresh: Option<T>,
        retained: RetainedCaptureRangesV1<T>,
    ) -> Result<T, ()> {
        match request {
            RearmRangeRequestV1::FreshWorkspace(_, _) | RearmRangeRequestV1::Unchanged => {
                fresh.ok_or(())
            }
            RearmRangeRequestV1::RetainedCompletionOutput => {
                self.completion_output_sources += 1;
                Ok(retained.completion_output)
            }
            RearmRangeRequestV1::RetainedQualificationLogits => {
                self.qualification_logits_sources += 1;
                let RetainedSemanticCaptureRangesV1::Qualification { logits } = retained.semantic
                else {
                    return Err(());
                };
                Ok(logits)
            }
            RearmRangeRequestV1::RetainedDirectDiagnosticChoices => {
                self.direct_diagnostic_sources += 1;
                let RetainedSemanticCaptureRangesV1::DirectDiagnostic { choices } =
                    retained.semantic
                else {
                    return Err(());
                };
                Ok(choices)
            }
            RearmRangeRequestV1::RetainedSpeculativeDraftChoices => {
                self.speculative_draft_sources += 1;
                let RetainedSemanticCaptureRangesV1::SpeculativeDiagnostic { draft, .. } =
                    retained.semantic
                else {
                    return Err(());
                };
                Ok(draft)
            }
            RearmRangeRequestV1::RetainedSpeculativeDraftChoice { iteration } => {
                self.speculative_draft_scalar_sources += 1;
                let RetainedSemanticCaptureRangesV1::SpeculativeDiagnostic { draft_rows, .. } =
                    retained.semantic
                else {
                    return Err(());
                };
                draft_rows
                    .get(usize::from(iteration))
                    .copied()
                    .flatten()
                    .ok_or(())
            }
            RearmRangeRequestV1::RetainedSpeculativeTargetChoices => {
                self.speculative_target_sources += 1;
                let RetainedSemanticCaptureRangesV1::SpeculativeDiagnostic { target, .. } =
                    retained.semantic
                else {
                    return Err(());
                };
                Ok(target)
            }
        }
    }

    fn validate(self) -> Result<(), ()> {
        let expected = match self.semantic {
            RetainedSemanticCaptureRangesV1::Ordinary => (0, 0, 0, 0, 0),
            RetainedSemanticCaptureRangesV1::Qualification { .. } => (2, 0, 0, 0, 0),
            RetainedSemanticCaptureRangesV1::DirectDiagnostic { .. } => (0, 2, 0, 0, 0),
            RetainedSemanticCaptureRangesV1::SpeculativeDiagnostic { draft_tokens, .. } => (
                0,
                0,
                2,
                usize::from(draft_tokens.saturating_mul(2).saturating_sub(1)),
                2,
            ),
        };
        (self.completion_output_sources == 1
            && self.qualification_logits_sources == expected.0
            && self.direct_diagnostic_sources == expected.1
            && self.speculative_draft_sources == expected.2
            && self.speculative_draft_scalar_sources == expected.3
            && self.speculative_target_sources == expected.4)
            .then_some(())
            .ok_or(())
    }
}

pub(crate) fn rebuild_bound_rows(
    source_rows: &[M1PhysicalBufferRecipeRowV1],
    old_bound_rows: &[M1BoundPhysicalBufferRowV1],
    composition: &AddresslessM1FullStepWorkspaceComposition,
    workspace_ranges: &[FreshWorkspaceRangeV1],
    previous_capture: &RetainedHostCaptureRangesV1,
    retained_capture: &RetainedHostCaptureRangesV1,
) -> Result<Box<[M1BoundPhysicalBufferRowV1]>, ()> {
    let mut old_rows = Vec::new();
    old_rows
        .try_reserve_exact(old_bound_rows.len())
        .map_err(|_| ())?;
    for old in old_bound_rows {
        let mut buffers = Vec::new();
        buffers
            .try_reserve_exact(old.buffers().len())
            .map_err(|_| ())?;
        buffers.extend(old.buffers().iter().map(|buffer| RearmBoundRangeV1 {
            explicit_argument_index: buffer.explicit_argument_index(),
            range: buffer.range(),
        }));
        old_rows.push(RearmBoundRowV1 {
            dispatch_index: old.dispatch_index(),
            profile_id: old.profile_id(),
            program: old.program(),
            buffers,
        });
    }
    let rebuilt = rebuild_bound_row_ranges(
        source_rows,
        &old_rows,
        composition,
        previous_capture
            .ranges
            .map(ServiceDispatchRangeV1::HostVisible),
        retained_capture
            .ranges
            .map(ServiceDispatchRangeV1::HostVisible),
        |workspace, requested| {
            Ok((
                workspace,
                ServiceDispatchRangeV1::Device(resolve_fresh_workspace_range(
                    workspace,
                    requested,
                    workspace_ranges,
                )?),
            ))
        },
    )?;
    let mut rows = Vec::new();
    rows.try_reserve_exact(rebuilt.len()).map_err(|_| ())?;
    for ((source, rebuilt), old) in source_rows.iter().zip(rebuilt).zip(old_bound_rows) {
        let mut buffers = Vec::new();
        buffers
            .try_reserve_exact(rebuilt.buffers.len())
            .map_err(|_| ())?;
        for ((semantic, buffer), old_buffer) in source
            .buffers()
            .iter()
            .zip(rebuilt.buffers)
            .zip(old.buffers())
        {
            let completed_snapshot = select_rearm_completed_snapshot(
                semantic.source(),
                old_buffer.completed_snapshot(),
                retained_capture.completion_snapshot,
            )?;
            buffers.push(match buffer.range {
                ServiceDispatchRangeV1::Device(range) => {
                    if completed_snapshot.is_some() {
                        return Err(());
                    }
                    ServiceFixedDispatchBufferV1::new(buffer.explicit_argument_index, range)
                }
                ServiceDispatchRangeV1::HostVisible(range) => {
                    if matches!(
                        semantic.source(),
                        M1PhysicalBufferSourceV1::CompletionOutput { .. }
                    ) {
                        match completed_snapshot {
                            Some(snapshot) => ServiceFixedDispatchBufferV1::new_host_visible_with_completed_snapshot(
                                buffer.explicit_argument_index,
                                range,
                                snapshot,
                            )
                            .map_err(|_| ())?,
                            None => ServiceFixedDispatchBufferV1::new_host_visible(
                                buffer.explicit_argument_index,
                                range,
                            ),
                        }
                    } else {
                        debug_assert!(completed_snapshot.is_none());
                        ServiceFixedDispatchBufferV1::new_host_visible(
                            buffer.explicit_argument_index,
                            range,
                        )
                    }
                }
            });
        }
        rows.push(M1BoundPhysicalBufferRowV1::from_queue_rearm(
            source,
            buffers.into_boxed_slice(),
        ));
    }
    Ok(rows.into_boxed_slice())
}

fn retained_unchanged_range(
    source: M1PhysicalBufferSourceV1,
    old_source_rows: &[M1PhysicalBufferRecipeRowV1],
    old_bound_rows: &[M1BoundPhysicalBufferRowV1],
) -> Result<ServiceDispatchRangeV1, ()> {
    if old_source_rows.len() != old_bound_rows.len() {
        return Err(());
    }
    let mut found = None;
    for (semantic_row, bound_row) in old_source_rows.iter().zip(old_bound_rows) {
        if semantic_row.buffers().len() != bound_row.buffers().len() {
            return Err(());
        }
        for (semantic, bound) in semantic_row.buffers().iter().zip(bound_row.buffers()) {
            if semantic.source() == source {
                let candidate = bound.range();
                if found.is_some_and(|retained| retained != candidate) {
                    return Err(());
                }
                found = Some(candidate);
            }
        }
    }
    found.ok_or(())
}

pub(crate) fn build_rollover_bound_rows(
    source_rows: &[M1PhysicalBufferRecipeRowV1],
    old_source_rows: &[M1PhysicalBufferRecipeRowV1],
    old_bound_rows: &[M1BoundPhysicalBufferRowV1],
    composition: &AddresslessM1FullStepWorkspaceComposition,
    workspace_ranges: &[FreshWorkspaceRangeV1],
    retained_capture: &RetainedHostCaptureRangesV1,
) -> Result<Box<[M1BoundPhysicalBufferRowV1]>, ()> {
    let retained = retained_capture
        .ranges
        .map(ServiceDispatchRangeV1::HostVisible);
    let mut selection = RearmRangeSelectionV1::new(&retained_capture.ranges.semantic);
    let mut rows = Vec::new();
    rows.try_reserve_exact(source_rows.len()).map_err(|_| ())?;
    for source_row in source_rows {
        let mut buffers = Vec::new();
        buffers
            .try_reserve_exact(source_row.buffers().len())
            .map_err(|_| ())?;
        for semantic in source_row.buffers() {
            let request = requested_workspace_range(
                semantic.source(),
                composition,
                &retained_capture.ranges.semantic,
            )?;
            let fresh = match request {
                RearmRangeRequestV1::FreshWorkspace(workspace, range) => {
                    Some(ServiceDispatchRangeV1::Device(
                        resolve_fresh_workspace_range(workspace, range, workspace_ranges)?,
                    ))
                }
                RearmRangeRequestV1::Unchanged => Some(retained_unchanged_range(
                    semantic.source(),
                    old_source_rows,
                    old_bound_rows,
                )?),
                RearmRangeRequestV1::RetainedCompletionOutput
                | RearmRangeRequestV1::RetainedQualificationLogits
                | RearmRangeRequestV1::RetainedDirectDiagnosticChoices
                | RearmRangeRequestV1::RetainedSpeculativeDraftChoices
                | RearmRangeRequestV1::RetainedSpeculativeDraftChoice { .. }
                | RearmRangeRequestV1::RetainedSpeculativeTargetChoices => None,
            };
            let range = selection.select_fresh(request, fresh, retained)?;
            let argument = semantic.explicit_argument_index();
            buffers.push(match range {
                ServiceDispatchRangeV1::Device(range) => {
                    ServiceFixedDispatchBufferV1::new(argument, range)
                }
                ServiceDispatchRangeV1::HostVisible(range) => {
                    if matches!(
                        semantic.source(),
                        M1PhysicalBufferSourceV1::CompletionOutput { .. }
                    ) {
                        match retained_capture.completion_snapshot {
                            Some(snapshot) => {
                                ServiceFixedDispatchBufferV1::new_host_visible_with_completed_snapshot(
                                    argument, range, snapshot,
                                )
                                .map_err(|_| ())?
                            }
                            None => ServiceFixedDispatchBufferV1::new_host_visible(argument, range),
                        }
                    } else {
                        ServiceFixedDispatchBufferV1::new_host_visible(argument, range)
                    }
                }
            });
        }
        rows.push(M1BoundPhysicalBufferRowV1::from_queue_rearm(
            source_row,
            buffers.into_boxed_slice(),
        ));
    }
    selection.validate()?;
    Ok(rows.into_boxed_slice())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RearmBoundRangeV1<T> {
    explicit_argument_index: usize,
    range: T,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RearmBoundRowV1<T> {
    dispatch_index: u32,
    profile_id: ferric_spec::Identity,
    program: crate::M1PhysicalProgramV1,
    buffers: Vec<RearmBoundRangeV1<T>>,
}

fn rebuild_bound_row_ranges<T: Copy + Eq>(
    source_rows: &[M1PhysicalBufferRecipeRowV1],
    old_bound_rows: &[RearmBoundRowV1<T>],
    composition: &AddresslessM1FullStepWorkspaceComposition,
    previous: RetainedCaptureRangesV1<T>,
    retained: RetainedCaptureRangesV1<T>,
    fresh_range: impl FnMut(
        M1FullStepWorkspaceRole,
        M1StepWorkspaceRange,
    ) -> Result<(M1FullStepWorkspaceRole, T), ()>,
) -> Result<Vec<RearmBoundRowV1<T>>, ()> {
    rebuild_bound_row_ranges_with_requests(
        source_rows,
        old_bound_rows,
        previous,
        retained,
        |source| requested_workspace_range(source, composition, &retained.semantic),
        fresh_range,
    )
}

fn rebuild_bound_row_ranges_with_requests<T: Copy + Eq>(
    source_rows: &[M1PhysicalBufferRecipeRowV1],
    old_bound_rows: &[RearmBoundRowV1<T>],
    previous: RetainedCaptureRangesV1<T>,
    retained: RetainedCaptureRangesV1<T>,
    mut range_request: impl FnMut(M1PhysicalBufferSourceV1) -> Result<RearmRangeRequestV1, ()>,
    mut fresh_range: impl FnMut(
        M1FullStepWorkspaceRole,
        M1StepWorkspaceRange,
    ) -> Result<(M1FullStepWorkspaceRole, T), ()>,
) -> Result<Vec<RearmBoundRowV1<T>>, ()> {
    if source_rows.len() != old_bound_rows.len() {
        return Err(());
    }
    let mut rows = Vec::new();
    let mut range_selection = RearmRangeSelectionV1::new(&retained.semantic);
    rows.try_reserve_exact(source_rows.len()).map_err(|_| ())?;
    for (source, old) in source_rows.iter().zip(old_bound_rows) {
        if source.dispatch_index() != old.dispatch_index
            || source.profile_id() != old.profile_id
            || source.program() != old.program
            || source.buffers().len() != old.buffers.len()
        {
            return Err(());
        }
        let mut buffers = Vec::new();
        buffers
            .try_reserve_exact(source.buffers().len())
            .map_err(|_| ())?;
        for (semantic, old_buffer) in source.buffers().iter().zip(&old.buffers) {
            if semantic.explicit_argument_index() != old_buffer.explicit_argument_index {
                return Err(());
            }
            let request = range_request(semantic.source())?;
            let fresh = match request {
                RearmRangeRequestV1::FreshWorkspace(workspace, requested) => {
                    let (bound_workspace, range) = fresh_range(workspace, requested)?;
                    if bound_workspace != workspace {
                        return Err(());
                    }
                    Some(range)
                }
                RearmRangeRequestV1::RetainedCompletionOutput
                | RearmRangeRequestV1::RetainedQualificationLogits
                | RearmRangeRequestV1::RetainedDirectDiagnosticChoices
                | RearmRangeRequestV1::RetainedSpeculativeDraftChoices
                | RearmRangeRequestV1::RetainedSpeculativeDraftChoice { .. }
                | RearmRangeRequestV1::RetainedSpeculativeTargetChoices
                | RearmRangeRequestV1::Unchanged => None,
            };
            let selected =
                range_selection.select(request, old_buffer.range, fresh, previous, retained)?;
            buffers.push(RearmBoundRangeV1 {
                explicit_argument_index: semantic.explicit_argument_index(),
                range: selected,
            });
        }
        rows.push(RearmBoundRowV1 {
            dispatch_index: source.dispatch_index(),
            profile_id: source.profile_id(),
            program: source.program(),
            buffers,
        });
    }
    range_selection.validate()?;
    Ok(rows)
}

fn preflight_rearm(
    prepared: &M1PreparedLongLivedQueueRearmV1,
    recipe: &AddresslessM1PhysicalBufferRecipeV1,
    catalog: &ContentBoundM1ProgramCatalogV1<'_>,
) -> Result<(), ()> {
    let old = prepared.remainder.queue.custody();
    if catalog.catalog_id() != old.catalog_id()
        || old.selection()
            != prepared
                .prepared
                .step()
                .kv_reservations()
                .target_selection()
        || recipe.kernarg_recipe().source_recipe() != old.physical_recipe()
        || recipe.workspace_composition() != old.workspace_composition()
        || recipe.rows() != old.source_rows()
        || prepared.prepared.kind() != old.workspace_owners().kind()
        || old.retained_intent_shape() != Some(prepared.remainder.queue.shape())
        || recipe.requires_future_materialization()
        || recipe.rows().len() != prepared.remainder.queue.shape().packet_count()
        || recipe.kernarg_recipe().images().len() != prepared.remainder.queue.shape().packet_count()
    {
        return Err(());
    }
    let plans_match = match (old.workspace_owners(), prepared.prepared.plans()) {
        (
            M1FullStepWorkspaceSubleaseOwners::TargetOnly { target: old },
            M1FullStepWorkspacePlans::TargetOnly { target: new },
        ) => old.plan() == &**new,
        (
            M1FullStepWorkspaceSubleaseOwners::SpeculativeRound {
                draft_decode: old_draft,
                target_speculative: old_target,
            },
            M1FullStepWorkspacePlans::SpeculativeRound {
                draft_decode: new_draft,
                target_speculative: new_target,
            },
        ) => old_draft.plan() == &**new_draft && old_target.plan() == &**new_target,
        _ => false,
    };
    if !plans_match {
        return Err(());
    }
    match (prepared.remainder.queue.shape(), prepared.prepared.kind()) {
        (M1PhysicalFixedBatchShapeV1::TargetOnly, M1FullStepWorkspaceInputKind::TargetOnly)
        | (
            M1PhysicalFixedBatchShapeV1::SpeculativeK4
            | M1PhysicalFixedBatchShapeV1::SpeculativeK8
            | M1PhysicalFixedBatchShapeV1::SpeculativeK16,
            M1FullStepWorkspaceInputKind::SpeculativeRound,
        ) => {}
        _ => return Err(()),
    }
    let expected_device = old.device();
    if prepared
        .remainder
        .selected
        .iter()
        .chain(&prepared.remainder.parked)
        .any(|cache| cache.projection().device != expected_device)
    {
        return Err(());
    }
    Ok(())
}

enum RebuiltBatchV1<'a> {
    TargetOnly(Box<ServiceFixedBatchV1<'a, M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1>>),
    SpeculativeK4(Box<ServiceFixedBatchV1<'a, M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1>>),
    SpeculativeK8(Box<ServiceFixedBatchV1<'a, M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1>>),
    SpeculativeK16(Box<ServiceFixedBatchV1<'a, M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1>>),
}

#[derive(Debug)]
enum WorkspaceReplacementFailureV1<const N: usize> {
    Update(ServiceQueueDataUpdateFailureV1),
    Binding(Box<M1QueueReplacedWorkspaceBindingFailureV1<N>>),
}

impl<const N: usize> WorkspaceReplacementFailureV1<N> {
    fn retained_owner_count(&self) -> usize {
        match self {
            Self::Update(failure) => {
                let _ = failure.error();
                1
            }
            Self::Binding(failure) => failure.retained_owner_count(),
        }
    }
}

struct LowerBatchFailureV1<'a> {
    catalog: ContentBoundM1ProgramCatalogV1<'a>,
    images: Box<[crate::M1PhysicalKernargImageV1]>,
}

struct LowerBatchInputV1 {
    physical: crate::M1PhysicalDispatchRecipeRowV1,
    image: crate::M1PhysicalKernargImageV1,
    buffers: Box<[fe2o3_service_host::ServiceFixedDispatchBufferV1]>,
}

// Keep each const-cardinality array construction out of the shape dispatcher.
#[inline(never)]
fn lower_boxed_rearm_batch<'a, const N: usize>(
    catalog: ContentBoundM1ProgramCatalogV1<'a>,
    physical: &crate::AddresslessM1PhysicalDispatchRecipeV1,
    images: Box<[crate::M1PhysicalKernargImageV1]>,
    bound: &[M1BoundPhysicalBufferRowV1],
) -> Result<Box<ServiceFixedBatchV1<'a, N>>, Box<LowerBatchFailureV1<'a>>> {
    lower_batch(catalog, physical, images, bound).map(Box::new)
}

fn lower_batch<'a, const N: usize>(
    catalog: ContentBoundM1ProgramCatalogV1<'a>,
    physical: &crate::AddresslessM1PhysicalDispatchRecipeV1,
    images: Box<[crate::M1PhysicalKernargImageV1]>,
    bound: &[M1BoundPhysicalBufferRowV1],
) -> Result<ServiceFixedBatchV1<'a, N>, Box<LowerBatchFailureV1<'a>>> {
    if physical.rows().len() != N || images.len() != N || bound.len() != N {
        return Err(Box::new(LowerBatchFailureV1 { catalog, images }));
    }
    let mut inputs = Vec::new();
    if inputs.try_reserve_exact(N).is_err() {
        return Err(Box::new(LowerBatchFailureV1 { catalog, images }));
    }
    for ((image, physical), bound) in images
        .into_vec()
        .into_iter()
        .zip(physical.rows().iter().copied())
        .zip(bound)
    {
        inputs.push(LowerBatchInputV1 {
            physical,
            image,
            buffers: bound.buffers().to_vec().into_boxed_slice(),
        });
    }
    let inputs: [LowerBatchInputV1; N] = match inputs.try_into() {
        Ok(inputs) => inputs,
        Err(inputs) => {
            return Err(Box::new(LowerBatchFailureV1 {
                catalog,
                images: inputs
                    .into_iter()
                    .map(|input| input.image)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            }));
        }
    };
    let packets = inputs.map(|input| {
        let LowerBatchInputV1 {
            physical,
            image,
            buffers,
        } = input;
        ServiceFixedDispatchPacketV1::new(
            physical.program_index(),
            physical.geometry(),
            physical.dynamic_group_segment_bytes(),
            image.into_bytes(),
            buffers,
        )
    });
    Ok(ServiceFixedBatchV1::new(catalog.into_programs(), packets))
}

/// Confirmed native predecessor destruction and replacement queue identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1QueueRolloverObservationV1 {
    previous_queue_destroyed: ComputeAqlQueueDestroyedV1,
    previous_dispatch_generation: u64,
    replacement_queue_observation: ComputeAqlQueueObservationV1,
    replacement_dispatch_generation: u64,
}

impl M1QueueRolloverObservationV1 {
    pub(crate) const fn new(
        previous_queue_destroyed: ComputeAqlQueueDestroyedV1,
        previous_dispatch_generation: u64,
        replacement_queue_observation: ComputeAqlQueueObservationV1,
        replacement_dispatch_generation: u64,
    ) -> Self {
        Self {
            previous_queue_destroyed,
            previous_dispatch_generation,
            replacement_queue_observation,
            replacement_dispatch_generation,
        }
    }

    /// Returns the lower observation proving predecessor queue destruction.
    #[must_use]
    pub const fn previous_queue_destroyed(&self) -> ComputeAqlQueueDestroyedV1 {
        self.previous_queue_destroyed
    }

    /// Returns the predecessor queue's final dispatch generation.
    #[must_use]
    pub const fn previous_dispatch_generation(&self) -> u64 {
        self.previous_dispatch_generation
    }

    /// Returns the native identity of the replacement queue.
    #[must_use]
    pub const fn replacement_queue_observation(&self) -> ComputeAqlQueueObservationV1 {
        self.replacement_queue_observation
    }

    /// Returns the replacement queue's first dispatch generation.
    #[must_use]
    pub const fn replacement_dispatch_generation(&self) -> u64 {
        self.replacement_dispatch_generation
    }
}

#[derive(Debug)]
struct M1RearmContinuationCustodyV1 {
    selected: Vec<ActiveDeviceKvCache>,
    parked: Vec<ActiveDeviceKvCache>,
    terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
    previous_epoch: CompletionEpoch,
    prior_checked: crate::M1CheckedCompletionOutputV1,
    logical_accepted_counts: Box<[u32]>,
    externally_published_counts: Box<[u32]>,
    release_counts: Box<[M1CompletedKvPageReleaseCountsV1]>,
    completed_members: usize,
    total_released: usize,
    history: M1RearmRoundHistoryV1,
    rollover: Option<M1QueueRolloverObservationV1>,
}

/// Published next generation on the same native queue, paired with all cache custody.
///
/// This owner has consuming [`Self::wait`] continuation; neither the physical
/// queue nor any cache owner can be extracted directly.
///
/// ```compile_fail
/// use ferric_engine::M1RearmedPublishedQueueV1;
/// fn extract_raw(published: M1RearmedPublishedQueueV1) {
///     let _raw = published.queue();
/// }
/// ```
///
/// ```compile_fail
/// use ferric_engine::{Engine, M1RearmedPublishedQueueV1};
/// fn wait_twice<const C: usize>(engine: &mut Engine<C>, published: M1RearmedPublishedQueueV1) {
///     let _first = published.wait(engine);
///     let _second = published.wait(engine);
/// }
/// ```
#[must_use = "published rearm custody must enter the completion pipeline"]
#[derive(Debug)]
pub struct M1RearmedPublishedQueueV1 {
    queue: M1PhysicalPublishedQueueSessionV1,
    carry: M1RearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

impl M1RearmedPublishedQueueV1 {
    /// Exact scheduler authority retained through physical publication.
    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        self.queue.scheduled_dispatch()
    }

    /// Exact closed physical shape retained by this publication.
    #[must_use]
    pub const fn shape(&self) -> M1PhysicalFixedBatchShapeV1 {
        self.queue.shape()
    }

    #[must_use]
    pub const fn queue_observation(&self) -> ComputeAqlQueueObservationV1 {
        self.queue_observation
    }

    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.device
    }

    /// Native rollover evidence when this generation replaced another queue.
    #[must_use]
    pub const fn rollover_observation(&self) -> Option<M1QueueRolloverObservationV1> {
        self.carry.rollover
    }

    #[must_use]
    pub fn selected_requests(&self) -> impl ExactSizeIterator<Item = RequestId> + '_ {
        self.carry
            .selected
            .iter()
            .map(|cache| cache.projection().request)
    }

    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.carry.parked.len()
    }

    #[must_use]
    pub const fn terminal_count(&self) -> usize {
        self.carry.terminal.len()
    }

    #[must_use]
    pub const fn previous_epoch(&self) -> CompletionEpoch {
        self.carry.previous_epoch
    }

    pub const fn prior_checked(&self) -> &crate::M1CheckedCompletionOutputV1 {
        &self.carry.prior_checked
    }

    #[must_use]
    pub fn prior_logical_accepted_counts(&self) -> &[u32] {
        &self.carry.logical_accepted_counts
    }

    #[must_use]
    pub fn prior_externally_published_counts(&self) -> &[u32] {
        &self.carry.externally_published_counts
    }

    #[must_use]
    pub fn prior_release_counts(&self) -> &[M1CompletedKvPageReleaseCountsV1] {
        &self.carry.release_counts
    }

    #[must_use]
    pub const fn prior_completed_members(&self) -> usize {
        self.carry.completed_members
    }

    #[must_use]
    pub const fn prior_total_released(&self) -> usize {
        self.carry.total_released
    }

    /// Waits for the exact rearmed generation while retaining every cache.
    ///
    /// # Errors
    ///
    /// Returns terminal queue quarantine paired with all continuation custody.
    pub fn wait<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<M1RearmedCompletedQueueV1, Box<M1RearmedQueueProgressFailureV1>> {
        let Self {
            queue,
            carry,
            queue_observation,
            device,
        } = self;
        match queue.wait() {
            Ok(queue) => Ok(M1RearmedCompletedQueueV1 {
                queue,
                carry,
                queue_observation,
                device,
            }),
            Err(source) => {
                engine.quarantine_m1_queue_rearm_failure();
                Err(Box::new(M1RearmedQueueProgressFailureV1 {
                    phase: M1LongLivedQueueRearmProgressPhaseV1::QueueWait,
                    source,
                    carry,
                    queue_observation,
                    device,
                }))
            }
        }
    }
}

/// Runtime phase for a rearmed queue continuation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1LongLivedQueueRearmProgressPhaseV1 {
    QueueWait,
    SignalRecycle,
}

/// Terminal queue-operation failure retaining rearm cache and prior-step custody.
#[must_use = "terminal queue failure and cache custody must remain retained"]
#[derive(Debug)]
pub struct M1RearmedQueueProgressFailureV1 {
    phase: M1LongLivedQueueRearmProgressPhaseV1,
    source: crate::M1PhysicalQueueOperationFailureV1,
    carry: M1RearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

impl M1RearmedQueueProgressFailureV1 {
    #[must_use]
    pub const fn phase(&self) -> M1LongLivedQueueRearmProgressPhaseV1 {
        self.phase
    }

    pub const fn source(&self) -> &crate::M1PhysicalQueueOperationFailureV1 {
        &self.source
    }

    #[must_use]
    pub const fn retained_cache_count(&self) -> usize {
        self.carry.selected.len() + self.carry.parked.len()
    }

    #[must_use]
    pub const fn queue_observation(&self) -> ComputeAqlQueueObservationV1 {
        self.queue_observation
    }

    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.device
    }
}

/// Completed rearmed generation before exact signal recycle.
#[must_use = "completed rearm custody must recycle or remain retained"]
#[derive(Debug)]
pub struct M1RearmedCompletedQueueV1 {
    queue: crate::M1PhysicalCompletedQueueSessionV1,
    carry: M1RearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

impl M1RearmedCompletedQueueV1 {
    /// Recycles the exact completion signals while retaining all rearm custody.
    ///
    /// # Errors
    ///
    /// Returns terminal queue quarantine paired with all continuation custody.
    pub fn recycle<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<M1RearmedRecycledQueueV1, Box<M1RearmedQueueProgressFailureV1>> {
        let Self {
            queue,
            carry,
            queue_observation,
            device,
        } = self;
        match queue.recycle() {
            Ok(queue) => Ok(M1RearmedRecycledQueueV1 {
                queue,
                carry,
                queue_observation,
                device,
            }),
            Err(source) => {
                engine.quarantine_m1_queue_rearm_failure();
                Err(Box::new(M1RearmedQueueProgressFailureV1 {
                    phase: M1LongLivedQueueRearmProgressPhaseV1::SignalRecycle,
                    source,
                    carry,
                    queue_observation,
                    device,
                }))
            }
        }
    }
}

/// Recycled rearmed queue ready for one exact observation and semantic join.
#[must_use = "recycled rearm custody must observe exact completion or remain retained"]
#[derive(Debug)]
pub struct M1RearmedRecycledQueueV1 {
    queue: crate::M1PhysicalRecycledQueueSessionV1,
    carry: M1RearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

impl M1RearmedRecycledQueueV1 {
    /// Copies and structurally observes the exact fresh K7 completion once.
    ///
    /// The returned move-only owner exposes the inert compact image before any
    /// semantic authority exists. Callers may inspect the observed token and
    /// must then consume the owner through
    /// [`M1RearmedObservedCompletionOutputV1::check_completion`].
    ///
    /// ```compile_fail
    /// use ferric_engine::M1RearmedRecycledQueueV1;
    /// fn observe_twice(recycled: M1RearmedRecycledQueueV1) {
    ///     let _first = recycled.observe_completion();
    ///     let _second = recycled.observe_completion();
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns the existing phase-accurate observation failure joined to every
    /// selected or parked cache owner. A retryable lower pre-copy rejection
    /// remains retryable; no path can repeat a successful physical copy.
    pub fn observe_completion(
        self,
    ) -> Result<M1RearmedObservedCompletionOutputV1, Box<M1RearmedReadbackFailureV1>> {
        let Self {
            queue,
            carry,
            queue_observation,
            device,
        } = self;
        match queue.observe_completion() {
            Ok(observed) => Ok(M1RearmedObservedCompletionOutputV1 {
                observed,
                carry,
                queue_observation,
                device,
            }),
            Err(source) => Err(Box::new(M1RearmedReadbackFailureV1 {
                source: M1RearmedReadbackFailureSourceV1::Observation(source),
                carry,
                queue_observation,
                device,
            })),
        }
    }

    /// Copies one rearmed target-decode compact output and its final live logits.
    ///
    /// This consuming path is intended only after the caller identifies the
    /// terminal qualification generation. The same attached qualification
    /// buffer is physically overwritten by every admitted target-decode
    /// generation; this method merely defers its host observation until the
    /// terminal one. Intermediate prompt priming uses
    /// [`Self::read_and_check_completion`] with the typed qualification
    /// prompt-commit expectation: K7's compact choice is structurally checked,
    /// logically accepted, and suppressed from external publication. This path
    /// is reserved for the terminal qualification expectation and full-logits
    /// observation.
    /// Observation failure retains both the lower phase-local queue custody and
    /// every selected or parked cache.
    ///
    /// ```compile_fail
    /// use ferric_engine::M1RearmedRecycledQueueV1;
    /// fn observe_twice(recycled: M1RearmedRecycledQueueV1) {
    ///     let _first = recycled.observe_qualification_completion();
    ///     let _second = recycled.observe_qualification_completion();
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns the existing qualification observation rejection paired with
    /// the complete rearm continuation custody.
    pub fn observe_qualification_completion(
        self,
    ) -> Result<
        M1RearmedObservedQualificationOutputV1,
        Box<M1RearmedQualificationObservationFailureV1>,
    > {
        let Self {
            queue,
            carry,
            queue_observation,
            device,
        } = self;
        match queue.observe_qualification_completion() {
            Ok(observed) => Ok(M1RearmedObservedQualificationOutputV1 {
                observed,
                carry,
                queue_observation,
                device,
            }),
            Err(source) => Err(Box::new(M1RearmedQualificationObservationFailureV1 {
                source,
                carry,
                queue_observation,
                device,
            })),
        }
    }

    /// Observes and checks the exact completion bytes for the fresh generation.
    ///
    /// # Errors
    ///
    /// A lower read rejection retains retryable recycled custody. Rejection
    /// after a successful copy retains either closed rejected-observation
    /// custody or the semantic observation, so no second physical read occurs.
    pub fn read_and_check_completion(
        self,
        expectations: &[crate::CompletionWireSemanticExpectation<'_>],
    ) -> Result<M1RearmedCompletedReadbackV1, Box<M1RearmedReadbackFailureV1>> {
        let Self {
            queue,
            carry,
            queue_observation,
            device,
        } = self;
        if let Err(gate) = validate_rearmed_generic_semantics(
            queue
                .custody()
                .completion_output()
                .qualification_logits()
                .is_some(),
            queue
                .custody()
                .completion_output()
                .direct_diagnostic_choices()
                .is_some(),
            queue
                .custody()
                .completion_output()
                .speculative_diagnostic_choices()
                .is_some(),
            expectations,
        ) {
            let source = match gate {
                M1RearmedGenericSemanticGateV1::Qualification { lane } => {
                    M1RearmedReadbackFailureSourceV1::QualificationCaptureRequiresEvidence {
                        lane,
                        queue: Box::new(queue),
                    }
                }
                M1RearmedGenericSemanticGateV1::DirectDiagnostic => {
                    M1RearmedReadbackFailureSourceV1::DirectDiagnosticCaptureRequiresEvidence {
                        queue: Box::new(queue),
                    }
                }
                M1RearmedGenericSemanticGateV1::SpeculativeDiagnostic => {
                    M1RearmedReadbackFailureSourceV1::SpeculativeDiagnosticCaptureRequiresEvidence {
                        queue: Box::new(queue),
                    }
                }
            };
            return Err(Box::new(M1RearmedReadbackFailureV1 {
                source,
                carry,
                queue_observation,
                device,
            }));
        }
        let observed = match queue.observe_completion() {
            Ok(observed) => observed,
            Err(source) => {
                return Err(Box::new(M1RearmedReadbackFailureV1 {
                    source: M1RearmedReadbackFailureSourceV1::Observation(source),
                    carry,
                    queue_observation,
                    device,
                }))
            }
        };
        match observed.check_completion(expectations) {
            Ok(readback) => Ok(M1RearmedCompletedReadbackV1 {
                readback,
                carry,
                queue_observation,
                device,
            }),
            Err(source) => Err(Box::new(M1RearmedReadbackFailureV1 {
                source: M1RearmedReadbackFailureSourceV1::Join(source),
                carry,
                queue_observation,
                device,
            })),
        }
    }

    /// Copies and joins independent final-row target choices for one rearmed
    /// `TargetOnly` completion.
    ///
    /// # Errors
    ///
    /// Returns every selected/parked cache and prior-round owner beside the
    /// exact compact, direct-choice observation, or semantic-join failure.
    pub fn read_and_check_direct_diagnostic_completion(
        self,
    ) -> Result<
        M1RearmedDirectDiagnosticCompletedReadbackV1,
        Box<M1RearmedDirectDiagnosticReadbackFailureV1>,
    > {
        let Self {
            queue,
            carry,
            queue_observation,
            device,
        } = self;
        let observed = match queue.observe_completion() {
            Ok(observed) => observed,
            Err(source) => {
                return Err(Box::new(M1RearmedDirectDiagnosticReadbackFailureV1 {
                    source: M1RearmedDirectDiagnosticReadbackFailureSourceV1::Compact(source),
                    carry,
                    queue_observation,
                    device,
                }))
            }
        };
        let diagnostic = match observed.observe_direct_diagnostic_choices() {
            Ok(diagnostic) => diagnostic,
            Err(source) => {
                return Err(Box::new(M1RearmedDirectDiagnosticReadbackFailureV1 {
                    source: M1RearmedDirectDiagnosticReadbackFailureSourceV1::Choices(source),
                    carry,
                    queue_observation,
                    device,
                }))
            }
        };
        let joined = match diagnostic.check_completion() {
            Ok(joined) => joined,
            Err(source) => {
                return Err(Box::new(M1RearmedDirectDiagnosticReadbackFailureV1 {
                    source: M1RearmedDirectDiagnosticReadbackFailureSourceV1::Join(source),
                    carry,
                    queue_observation,
                    device,
                }))
            }
        };
        let (readback, choices) = joined.into_parts();
        Ok(M1RearmedDirectDiagnosticCompletedReadbackV1 {
            readback: M1RearmedCompletedReadbackV1 {
                readback,
                carry,
                queue_observation,
                device,
            },
            choices,
        })
    }

    /// Source-compatible S1/K4 entry point for evidence-bearing rearmed readback.
    ///
    /// Production serving currently calls this entry point only for S1/K4.
    ///
    /// # Errors
    ///
    /// Returns every selected/parked cache and prior-round owner beside the
    /// exact diagnostic observation or semantic-join failure.
    pub fn read_and_check_speculative_k4_diagnostic_completion(
        self,
    ) -> Result<
        M1RearmedSpeculativeDiagnosticCompletedReadbackV1,
        Box<M1RearmedSpeculativeDiagnosticReadbackFailureV1>,
    > {
        let Self {
            queue,
            carry,
            queue_observation,
            device,
        } = self;
        let observed = match queue.observe_completion() {
            Ok(observed) => observed,
            Err(source) => {
                return Err(Box::new(M1RearmedSpeculativeDiagnosticReadbackFailureV1 {
                    source: M1RearmedSpeculativeDiagnosticReadbackFailureSourceV1::Compact(source),
                    carry,
                    queue_observation,
                    device,
                }));
            }
        };
        let diagnostic = match observed.observe_speculative_k4_diagnostic_choices() {
            Ok(diagnostic) => diagnostic,
            Err(source) => {
                return Err(Box::new(M1RearmedSpeculativeDiagnosticReadbackFailureV1 {
                    source: M1RearmedSpeculativeDiagnosticReadbackFailureSourceV1::Choices(source),
                    carry,
                    queue_observation,
                    device,
                }));
            }
        };
        let joined = match diagnostic.check_completion() {
            Ok(joined) => joined,
            Err(source) => {
                return Err(Box::new(M1RearmedSpeculativeDiagnosticReadbackFailureV1 {
                    source: M1RearmedSpeculativeDiagnosticReadbackFailureSourceV1::Join(source),
                    carry,
                    queue_observation,
                    device,
                }));
            }
        };
        let (readback, choices) = joined.into_parts();
        Ok(M1RearmedSpeculativeDiagnosticCompletedReadbackV1 {
            readback: M1RearmedCompletedReadbackV1 {
                readback,
                carry,
                queue_observation,
                device,
            },
            choices,
        })
    }

    /// Copies, independently observes, and semantically joins one rearmed
    /// finite M1 speculative completion.
    ///
    /// # Errors
    ///
    /// Returns exact continuation custody beside compact-copy, choice-copy, or
    /// semantic-join failure.
    pub fn read_and_check_speculative_diagnostic_completion(
        self,
    ) -> Result<
        M1RearmedSpeculativeDiagnosticCompletedReadbackV1,
        Box<M1RearmedSpeculativeDiagnosticReadbackFailureV1>,
    > {
        let Self {
            queue,
            carry,
            queue_observation,
            device,
        } = self;
        let observed = match queue.observe_completion() {
            Ok(observed) => observed,
            Err(source) => {
                return Err(Box::new(M1RearmedSpeculativeDiagnosticReadbackFailureV1 {
                    source: M1RearmedSpeculativeDiagnosticReadbackFailureSourceV1::Compact(source),
                    carry,
                    queue_observation,
                    device,
                }));
            }
        };
        let diagnostic = match observed.observe_speculative_diagnostic_choices() {
            Ok(diagnostic) => diagnostic,
            Err(source) => {
                return Err(Box::new(M1RearmedSpeculativeDiagnosticReadbackFailureV1 {
                    source: M1RearmedSpeculativeDiagnosticReadbackFailureSourceV1::Choices(source),
                    carry,
                    queue_observation,
                    device,
                }));
            }
        };
        let joined = match diagnostic.check_completion() {
            Ok(joined) => joined,
            Err(source) => {
                return Err(Box::new(M1RearmedSpeculativeDiagnosticReadbackFailureV1 {
                    source: M1RearmedSpeculativeDiagnosticReadbackFailureSourceV1::Join(source),
                    carry,
                    queue_observation,
                    device,
                }));
            }
        };
        let (readback, choices) = joined.into_parts();
        Ok(M1RearmedSpeculativeDiagnosticCompletedReadbackV1 {
            readback: M1RearmedCompletedReadbackV1 {
                readback,
                carry,
                queue_observation,
                device,
            },
            choices,
        })
    }
}

/// Phase-local failure for evidence-bearing rearmed direct readback.
#[must_use = "failed direct diagnostic readback retains queue, cache, and evidence custody"]
#[derive(Debug)]
pub enum M1RearmedDirectDiagnosticReadbackFailureSourceV1 {
    Compact(crate::M1CompletionObservationFailureV1),
    Choices(Box<crate::M1DirectDiagnosticObservationFailureV1>),
    Join(Box<crate::M1DirectDiagnosticCompletedReadbackJoinFailureV1>),
}

/// Failed rearmed direct diagnostic join with complete continuation custody.
#[must_use = "failed direct diagnostic readback must be retained or torn down"]
#[derive(Debug)]
pub struct M1RearmedDirectDiagnosticReadbackFailureV1 {
    source: M1RearmedDirectDiagnosticReadbackFailureSourceV1,
    carry: M1RearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

impl M1RearmedDirectDiagnosticReadbackFailureV1 {
    pub const fn source(&self) -> &M1RearmedDirectDiagnosticReadbackFailureSourceV1 {
        &self.source
    }

    #[must_use]
    pub const fn retained_cache_count(&self) -> usize {
        self.carry.selected.len() + self.carry.parked.len()
    }

    #[must_use]
    pub const fn queue_observation(&self) -> ComputeAqlQueueObservationV1 {
        self.queue_observation
    }

    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.device
    }

    /// Separates exact lower failure from opaque retained rearm lineage.
    #[must_use = "both direct failure and retained rearm lineage remain linear"]
    pub fn into_parts(
        self: Box<Self>,
    ) -> (
        M1RearmedDirectDiagnosticReadbackFailureSourceV1,
        M1RearmedDirectDiagnosticRetainedCustodyV1,
    ) {
        let Self {
            source,
            carry,
            queue_observation,
            device,
        } = *self;
        (
            source,
            M1RearmedDirectDiagnosticRetainedCustodyV1 {
                carry,
                queue_observation,
                device,
            },
        )
    }

    /// Fail-stops `engine`, releases the failed physical queue, and retains all
    /// direct evidence and rearm continuation custody.
    ///
    /// # Errors
    ///
    /// Returns exact lower release quarantine joined to unchanged evidence.
    pub fn destroy_queue_and_retain_custody<const C: usize>(
        self: Box<Self>,
        engine: &mut Engine<C>,
    ) -> Result<
        M1RearmedDirectDiagnosticReadbackTeardownSuccessV1,
        Box<M1RearmedDirectDiagnosticReadbackTeardownFailureV1>,
    > {
        let (source, retained) = self.into_parts();
        let teardown = match source {
            M1RearmedDirectDiagnosticReadbackFailureSourceV1::Compact(source) => source
                .destroy_queue_and_retain_evidence(engine)
                .map(M1RearmedDirectDiagnosticReadbackTeardownSuccessSourceV1::Compact)
                .map_err(|source| {
                    M1RearmedDirectDiagnosticReadbackTeardownFailureSourceV1::Compact(source)
                }),
            M1RearmedDirectDiagnosticReadbackFailureSourceV1::Choices(source) => (*source)
                .destroy_queue_and_retain_evidence(engine)
                .map(M1RearmedDirectDiagnosticReadbackTeardownSuccessSourceV1::Choices)
                .map_err(|source| {
                    M1RearmedDirectDiagnosticReadbackTeardownFailureSourceV1::Choices(source)
                }),
            M1RearmedDirectDiagnosticReadbackFailureSourceV1::Join(source) => (*source)
                .destroy_queue_and_retain_evidence(engine)
                .map(M1RearmedDirectDiagnosticReadbackTeardownSuccessSourceV1::Join)
                .map_err(|source| {
                    M1RearmedDirectDiagnosticReadbackTeardownFailureSourceV1::Join(source)
                }),
        };
        match teardown {
            Ok(source) => {
                Ok(M1RearmedDirectDiagnosticReadbackTeardownSuccessV1 { source, retained })
            }
            Err(source) => Err(Box::new(
                M1RearmedDirectDiagnosticReadbackTeardownFailureV1 { source, retained },
            )),
        }
    }
}

/// Opaque rearm continuation custody retained independently of direct readback.
#[must_use = "rearm continuation custody must remain retained"]
#[derive(Debug)]
pub struct M1RearmedDirectDiagnosticRetainedCustodyV1 {
    carry: M1RearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

impl M1RearmedDirectDiagnosticRetainedCustodyV1 {
    #[must_use]
    pub fn selected_requests(&self) -> impl ExactSizeIterator<Item = RequestId> + '_ {
        self.carry
            .selected
            .iter()
            .map(|cache| cache.projection().request)
    }

    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.carry.parked.len()
    }

    #[must_use]
    pub const fn terminal_count(&self) -> usize {
        self.carry.terminal.len()
    }

    #[must_use]
    pub const fn previous_epoch(&self) -> CompletionEpoch {
        self.carry.previous_epoch
    }

    pub const fn prior_checked(&self) -> &crate::M1CheckedCompletionOutputV1 {
        &self.carry.prior_checked
    }

    #[must_use]
    pub const fn queue_observation(&self) -> ComputeAqlQueueObservationV1 {
        self.queue_observation
    }

    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.device
    }
}

/// Exact lower success after terminal direct diagnostic teardown.
#[must_use = "direct diagnostic teardown evidence must remain retained"]
#[derive(Debug)]
pub enum M1RearmedDirectDiagnosticReadbackTeardownSuccessSourceV1 {
    Compact(crate::M1CompletionEvidenceTeardownSuccessV1),
    Choices(crate::M1DirectDiagnosticObservationTeardownSuccessV1),
    Join(crate::M1DirectDiagnosticSemanticTeardownSuccessV1),
}

/// Exact lower release quarantine after terminal direct diagnostic teardown.
#[must_use = "direct diagnostic teardown quarantine must remain retained"]
#[derive(Debug)]
pub enum M1RearmedDirectDiagnosticReadbackTeardownFailureSourceV1 {
    Compact(Box<crate::M1CompletionEvidenceTeardownFailureV1>),
    Choices(Box<crate::M1DirectDiagnosticObservationTeardownFailureV1>),
    Join(Box<crate::M1DirectDiagnosticSemanticTeardownFailureV1>),
}

/// Clean queue release retaining direct evidence and all rearm lineage.
#[must_use = "direct evidence and rearm lineage must remain retained"]
#[derive(Debug)]
pub struct M1RearmedDirectDiagnosticReadbackTeardownSuccessV1 {
    source: M1RearmedDirectDiagnosticReadbackTeardownSuccessSourceV1,
    retained: M1RearmedDirectDiagnosticRetainedCustodyV1,
}

impl M1RearmedDirectDiagnosticReadbackTeardownSuccessV1 {
    pub const fn source(&self) -> &M1RearmedDirectDiagnosticReadbackTeardownSuccessSourceV1 {
        &self.source
    }

    pub const fn retained(&self) -> &M1RearmedDirectDiagnosticRetainedCustodyV1 {
        &self.retained
    }

    #[must_use = "teardown evidence and retained lineage remain linear"]
    pub fn into_parts(
        self,
    ) -> (
        M1RearmedDirectDiagnosticReadbackTeardownSuccessSourceV1,
        M1RearmedDirectDiagnosticRetainedCustodyV1,
    ) {
        (self.source, self.retained)
    }
}

/// Failed release retaining direct evidence and all rearm lineage.
#[must_use = "direct release quarantine and rearm lineage must remain retained"]
#[derive(Debug)]
pub struct M1RearmedDirectDiagnosticReadbackTeardownFailureV1 {
    source: M1RearmedDirectDiagnosticReadbackTeardownFailureSourceV1,
    retained: M1RearmedDirectDiagnosticRetainedCustodyV1,
}

impl M1RearmedDirectDiagnosticReadbackTeardownFailureV1 {
    pub const fn source(&self) -> &M1RearmedDirectDiagnosticReadbackTeardownFailureSourceV1 {
        &self.source
    }

    pub const fn retained(&self) -> &M1RearmedDirectDiagnosticRetainedCustodyV1 {
        &self.retained
    }

    #[must_use = "release quarantine and retained lineage remain linear"]
    pub fn into_parts(
        self,
    ) -> (
        M1RearmedDirectDiagnosticReadbackTeardownFailureSourceV1,
        M1RearmedDirectDiagnosticRetainedCustodyV1,
    ) {
        (self.source, self.retained)
    }
}

/// Rearmed direct completed readback retaining independent choice evidence.
#[must_use = "direct readback and choice evidence must remain retained"]
#[derive(Debug)]
pub struct M1RearmedDirectDiagnosticCompletedReadbackV1 {
    readback: M1RearmedCompletedReadbackV1,
    choices: crate::M1ObservedDirectDiagnosticChoicesV1,
}

impl M1RearmedDirectDiagnosticCompletedReadbackV1 {
    pub const fn checked(&self) -> &crate::M1CheckedCompletionOutputV1 {
        self.readback.checked()
    }

    pub const fn choices(&self) -> &crate::M1ObservedDirectDiagnosticChoicesV1 {
        &self.choices
    }

    /// Separates normal completion custody from independent direct evidence.
    #[must_use = "both normal readback and direct evidence must remain retained"]
    pub fn into_parts(
        self,
    ) -> (
        M1RearmedCompletedReadbackV1,
        crate::M1ObservedDirectDiagnosticChoicesV1,
    ) {
        (self.readback, self.choices)
    }
}

/// Phase-local failure for an evidence-bearing rearmed speculative readback.
#[must_use = "failed diagnostic readback retains queue, cache, and evidence custody"]
#[derive(Debug)]
pub enum M1RearmedSpeculativeDiagnosticReadbackFailureSourceV1 {
    Compact(crate::M1CompletionObservationFailureV1),
    Choices(Box<crate::M1SpeculativeDiagnosticObservationFailureV1>),
    Join(Box<crate::M1SpeculativeDiagnosticCompletedReadbackJoinFailureV1>),
}

/// Failed rearmed speculative diagnostic join with complete continuation custody.
#[must_use = "failed diagnostic readback must be retained or torn down"]
#[derive(Debug)]
pub struct M1RearmedSpeculativeDiagnosticReadbackFailureV1 {
    source: M1RearmedSpeculativeDiagnosticReadbackFailureSourceV1,
    carry: M1RearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

impl M1RearmedSpeculativeDiagnosticReadbackFailureV1 {
    pub const fn source(&self) -> &M1RearmedSpeculativeDiagnosticReadbackFailureSourceV1 {
        &self.source
    }

    #[must_use]
    pub const fn retained_cache_count(&self) -> usize {
        self.carry.selected.len() + self.carry.parked.len()
    }

    #[must_use]
    pub const fn queue_observation(&self) -> ComputeAqlQueueObservationV1 {
        self.queue_observation
    }

    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.device
    }

    /// Separates the exact lower failure from opaque retained rearm lineage.
    ///
    /// This does not reopen serving or scheduler authority. The originating
    /// serving adapter and Engine remain sealed; the split owners are only for
    /// phase-accurate diagnosis and terminal cleanup.
    #[must_use = "both diagnostic failure and retained rearm lineage remain linear"]
    pub fn into_parts(
        self: Box<Self>,
    ) -> (
        M1RearmedSpeculativeDiagnosticReadbackFailureSourceV1,
        M1RearmedSpeculativeDiagnosticRetainedCustodyV1,
    ) {
        let Self {
            source,
            carry,
            queue_observation,
            device,
        } = *self;
        (
            source,
            M1RearmedSpeculativeDiagnosticRetainedCustodyV1 {
                carry,
                queue_observation,
                device,
            },
        )
    }

    /// Fail-stops `engine`, releases the failed physical queue, and retains all
    /// diagnostic evidence and rearm continuation custody.
    ///
    /// # Errors
    ///
    /// Returns the exact lower release quarantine joined to the same evidence
    /// and lineage when queue release fails.
    pub fn destroy_queue_and_retain_custody<const C: usize>(
        self: Box<Self>,
        engine: &mut Engine<C>,
    ) -> Result<
        M1RearmedSpeculativeDiagnosticReadbackTeardownSuccessV1,
        Box<M1RearmedSpeculativeDiagnosticReadbackTeardownFailureV1>,
    > {
        let (source, retained) = self.into_parts();
        let teardown = match source {
            M1RearmedSpeculativeDiagnosticReadbackFailureSourceV1::Compact(source) => source
                .destroy_queue_and_retain_evidence(engine)
                .map(M1RearmedSpeculativeDiagnosticReadbackTeardownSuccessSourceV1::Compact)
                .map_err(|source| {
                    Box::new(
                        M1RearmedSpeculativeDiagnosticReadbackTeardownFailureSourceV1::Compact(
                            source,
                        ),
                    )
                }),
            M1RearmedSpeculativeDiagnosticReadbackFailureSourceV1::Choices(source) => (*source)
                .destroy_queue_and_retain_evidence(engine)
                .map(M1RearmedSpeculativeDiagnosticReadbackTeardownSuccessSourceV1::Choices)
                .map_err(|source| {
                    Box::new(
                        M1RearmedSpeculativeDiagnosticReadbackTeardownFailureSourceV1::Choices(
                            source,
                        ),
                    )
                }),
            M1RearmedSpeculativeDiagnosticReadbackFailureSourceV1::Join(source) => (*source)
                .destroy_queue_and_retain_evidence(engine)
                .map(M1RearmedSpeculativeDiagnosticReadbackTeardownSuccessSourceV1::Join)
                .map_err(|source| {
                    Box::new(
                        M1RearmedSpeculativeDiagnosticReadbackTeardownFailureSourceV1::Join(source),
                    )
                }),
        };
        match teardown {
            Ok(source) => {
                Ok(M1RearmedSpeculativeDiagnosticReadbackTeardownSuccessV1 { source, retained })
            }
            Err(source) => Err(Box::new(
                M1RearmedSpeculativeDiagnosticReadbackTeardownFailureV1 {
                    source: *source,
                    retained,
                },
            )),
        }
    }
}

/// Opaque rearm continuation custody retained independently of readback phase.
#[must_use = "rearm continuation custody must remain retained"]
#[derive(Debug)]
pub struct M1RearmedSpeculativeDiagnosticRetainedCustodyV1 {
    carry: M1RearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

impl M1RearmedSpeculativeDiagnosticRetainedCustodyV1 {
    #[must_use]
    pub fn selected_requests(&self) -> impl ExactSizeIterator<Item = RequestId> + '_ {
        self.carry
            .selected
            .iter()
            .map(|cache| cache.projection().request)
    }

    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.carry.parked.len()
    }

    #[must_use]
    pub const fn terminal_count(&self) -> usize {
        self.carry.terminal.len()
    }

    #[must_use]
    pub const fn previous_epoch(&self) -> CompletionEpoch {
        self.carry.previous_epoch
    }

    pub const fn prior_checked(&self) -> &crate::M1CheckedCompletionOutputV1 {
        &self.carry.prior_checked
    }

    #[must_use]
    pub const fn queue_observation(&self) -> ComputeAqlQueueObservationV1 {
        self.queue_observation
    }

    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.device
    }
}

/// Exact lower success after terminal diagnostic readback teardown.
#[must_use = "diagnostic teardown evidence must remain retained"]
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum M1RearmedSpeculativeDiagnosticReadbackTeardownSuccessSourceV1 {
    Compact(crate::M1CompletionEvidenceTeardownSuccessV1),
    Choices(crate::M1SpeculativeDiagnosticObservationTeardownSuccessV1),
    Join(crate::M1SpeculativeDiagnosticSemanticTeardownSuccessV1),
}

/// Exact lower release quarantine after terminal diagnostic readback teardown.
#[must_use = "diagnostic teardown quarantine must remain retained"]
#[derive(Debug)]
pub enum M1RearmedSpeculativeDiagnosticReadbackTeardownFailureSourceV1 {
    Compact(Box<crate::M1CompletionEvidenceTeardownFailureV1>),
    Choices(Box<crate::M1SpeculativeDiagnosticObservationTeardownFailureV1>),
    Join(Box<crate::M1SpeculativeDiagnosticSemanticTeardownFailureV1>),
}

/// Clean queue release retaining diagnostic evidence and all rearm lineage.
#[must_use = "diagnostic evidence and rearm lineage must remain retained"]
#[derive(Debug)]
pub struct M1RearmedSpeculativeDiagnosticReadbackTeardownSuccessV1 {
    source: M1RearmedSpeculativeDiagnosticReadbackTeardownSuccessSourceV1,
    retained: M1RearmedSpeculativeDiagnosticRetainedCustodyV1,
}

impl M1RearmedSpeculativeDiagnosticReadbackTeardownSuccessV1 {
    pub const fn source(&self) -> &M1RearmedSpeculativeDiagnosticReadbackTeardownSuccessSourceV1 {
        &self.source
    }

    pub const fn retained(&self) -> &M1RearmedSpeculativeDiagnosticRetainedCustodyV1 {
        &self.retained
    }

    #[must_use = "teardown evidence and retained lineage remain linear"]
    pub fn into_parts(
        self,
    ) -> (
        M1RearmedSpeculativeDiagnosticReadbackTeardownSuccessSourceV1,
        M1RearmedSpeculativeDiagnosticRetainedCustodyV1,
    ) {
        (self.source, self.retained)
    }
}

/// Failed queue release retaining diagnostic evidence and all rearm lineage.
#[must_use = "diagnostic release quarantine and rearm lineage must remain retained"]
#[derive(Debug)]
pub struct M1RearmedSpeculativeDiagnosticReadbackTeardownFailureV1 {
    source: M1RearmedSpeculativeDiagnosticReadbackTeardownFailureSourceV1,
    retained: M1RearmedSpeculativeDiagnosticRetainedCustodyV1,
}

impl M1RearmedSpeculativeDiagnosticReadbackTeardownFailureV1 {
    pub const fn source(&self) -> &M1RearmedSpeculativeDiagnosticReadbackTeardownFailureSourceV1 {
        &self.source
    }

    pub const fn retained(&self) -> &M1RearmedSpeculativeDiagnosticRetainedCustodyV1 {
        &self.retained
    }

    #[must_use = "release quarantine and retained lineage remain linear"]
    pub fn into_parts(
        self,
    ) -> (
        M1RearmedSpeculativeDiagnosticReadbackTeardownFailureSourceV1,
        M1RearmedSpeculativeDiagnosticRetainedCustodyV1,
    ) {
        (self.source, self.retained)
    }
}

/// Rearmed speculative completed readback retaining independent choice evidence.
#[must_use = "diagnostic readback and choice evidence must remain retained"]
#[derive(Debug)]
pub struct M1RearmedSpeculativeDiagnosticCompletedReadbackV1 {
    readback: M1RearmedCompletedReadbackV1,
    choices: crate::M1ObservedSpeculativeDiagnosticChoicesV1,
}

impl M1RearmedSpeculativeDiagnosticCompletedReadbackV1 {
    pub const fn checked(&self) -> &crate::M1CheckedCompletionOutputV1 {
        self.readback.checked()
    }

    pub const fn choices(&self) -> &crate::M1ObservedSpeculativeDiagnosticChoicesV1 {
        &self.choices
    }

    /// Separates normal completion custody from independent choice evidence.
    #[must_use = "both normal readback and diagnostic evidence must remain retained"]
    pub fn into_parts(
        self,
    ) -> (
        M1RearmedCompletedReadbackV1,
        crate::M1ObservedSpeculativeDiagnosticChoicesV1,
    ) {
        (self.readback, self.choices)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum M1RearmedGenericSemanticGateV1 {
    Qualification { lane: usize },
    DirectDiagnostic,
    SpeculativeDiagnostic,
}

fn validate_rearmed_generic_semantics(
    qualification_capture_enabled: bool,
    direct_diagnostic_capture_enabled: bool,
    speculative_diagnostic_capture_enabled: bool,
    expectations: &[crate::CompletionWireSemanticExpectation<'_>],
) -> Result<(), M1RearmedGenericSemanticGateV1> {
    if direct_diagnostic_capture_enabled {
        return Err(M1RearmedGenericSemanticGateV1::DirectDiagnostic);
    }
    if speculative_diagnostic_capture_enabled {
        return Err(M1RearmedGenericSemanticGateV1::SpeculativeDiagnostic);
    }
    if qualification_capture_enabled {
        if let Some(lane) = expectations.iter().position(|semantic| {
            !matches!(
                semantic,
                crate::CompletionWireSemanticExpectation::QualificationPromptCommit { .. }
            )
        }) {
            return Err(M1RearmedGenericSemanticGateV1::Qualification { lane });
        }
    }
    Ok(())
}

/// Qualification-copy rejection retaining every rearm continuation owner.
///
/// ```compile_fail
/// use ferric_engine::M1RearmedQualificationObservationFailureV1;
/// fn retry_twice(failure: M1RearmedQualificationObservationFailureV1) {
///     let _first = failure.retry();
///     let _second = failure.retry();
/// }
/// ```
#[must_use = "qualification observation failure retains queue and cache custody"]
#[derive(Debug)]
pub struct M1RearmedQualificationObservationFailureV1 {
    source: Box<crate::M1QualificationObservationFailureV1>,
    carry: M1RearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

impl M1RearmedQualificationObservationFailureV1 {
    /// Exact lower qualification-copy rejection.
    pub const fn source(&self) -> &crate::M1QualificationObservationFailureV1 {
        &self.source
    }

    /// Number of active cache owners retained across selected and parked lanes.
    #[must_use]
    pub const fn retained_cache_count(&self) -> usize {
        self.carry.selected.len() + self.carry.parked.len()
    }

    /// Exact completed queue generation observation.
    #[must_use]
    pub const fn queue_observation(&self) -> ComputeAqlQueueObservationV1 {
        self.queue_observation
    }

    /// Checked physical-device receipt retained through failure.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.device
    }

    /// Exact predecessor completion epoch.
    #[must_use]
    pub const fn previous_epoch(&self) -> CompletionEpoch {
        self.carry.previous_epoch
    }

    /// Retries only a lower pre-copy `Recycled` failure and rejoins success to
    /// this exact selected/parked cache lineage.
    ///
    /// Terminal and partial-copy failures return unchanged joined custody; no
    /// completed read that may already have succeeded is reopened.
    /// A true lower observation-success retry requires native recycled queue
    /// custody and is therefore exercised by hardware integration; unit tests
    /// cover this transition's exact success/failure rejoin core.
    ///
    /// # Errors
    ///
    /// Returns the same joined owner when retry is not admitted or when the
    /// lower retry rejects again.
    pub fn retry(self) -> Result<M1RearmedObservedQualificationOutputV1, Box<Self>> {
        let Self {
            source,
            carry,
            queue_observation,
            device,
        } = self;
        let retry = retry_physical_qualification_observation_source(*source);
        match rejoin_qualification_observation_retry(retry, carry, queue_observation, device) {
            Ok(joined) => Ok(M1RearmedObservedQualificationOutputV1 {
                observed: joined.source,
                carry: joined.carry,
                queue_observation: joined.queue_observation,
                device: joined.device,
            }),
            Err(joined) => {
                let joined = *joined;
                Err(Box::new(Self {
                    source: joined.source,
                    carry: joined.carry,
                    queue_observation: joined.queue_observation,
                    device: joined.device,
                }))
            }
        }
    }

    /// Permanently quarantines `engine`, then destroys the failed physical queue
    /// while retaining its diagnostic, partial-copy evidence, and all
    /// selected/parked cache lineage together.
    ///
    /// # Errors
    ///
    /// Returns terminal lower release quarantine joined to the same rearm
    /// lineage and partial completed-copy custody.
    pub fn destroy_queue_and_retain_custody<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1RearmedQualificationObservationTeardownSuccessV1,
        Box<M1RearmedQualificationObservationTeardownFailureV1>,
    > {
        quarantine_qualification_teardown(engine);
        let Self {
            source,
            carry,
            queue_observation,
            device,
        } = self;
        match teardown_physical_qualification_observation_source(*source) {
            Ok(source) => Ok(M1RearmedQualificationObservationTeardownSuccessV1 {
                source,
                carry,
                queue_observation,
                device,
            }),
            Err(source) => Err(Box::new(
                M1RearmedQualificationObservationTeardownFailureV1 {
                    source,
                    carry,
                    queue_observation,
                    device,
                },
            )),
        }
    }
}

fn quarantine_qualification_teardown<const C: usize>(engine: &mut Engine<C>) {
    engine.quarantine_m1_queue_rearm_failure();
}

#[derive(Debug)]
struct M1RearmedQualificationObservationRetryJoinV1<T, Q> {
    source: T,
    carry: M1RearmContinuationCustodyV1,
    queue_observation: Q,
    device: Gfx942DeviceBinding,
}

fn rejoin_qualification_observation_retry<T, E, Q>(
    source: Result<T, E>,
    carry: M1RearmContinuationCustodyV1,
    queue_observation: Q,
    device: Gfx942DeviceBinding,
) -> Result<
    M1RearmedQualificationObservationRetryJoinV1<T, Q>,
    Box<M1RearmedQualificationObservationRetryJoinV1<E, Q>>,
> {
    match source {
        Ok(source) => Ok(M1RearmedQualificationObservationRetryJoinV1 {
            source,
            carry,
            queue_observation,
            device,
        }),
        Err(source) => Err(Box::new(M1RearmedQualificationObservationRetryJoinV1 {
            source,
            carry,
            queue_observation,
            device,
        })),
    }
}

fn retry_physical_qualification_observation_source(
    source: crate::M1QualificationObservationFailureV1,
) -> Result<crate::M1ObservedQualificationOutputV1, Box<crate::M1QualificationObservationFailureV1>>
{
    let (error, custody) = source.into_parts();
    match custody {
        crate::M1QualificationObservationFailureCustodyV1::Recycled(queue) => {
            queue.observe_qualification_completion()
        }
        custody => Err(Box::new(
            crate::M1QualificationObservationFailureV1::from_parts(error, custody),
        )),
    }
}

#[derive(Debug)]
struct QualificationObservationTeardownSuccessV1 {
    error: crate::M1QualificationObservationErrorV1,
    queue_release: ServiceQueueReleaseObservationV1,
    compact_evidence: M1RearmedReadbackTeardownEvidenceV1,
    partial_logits: Box<[ServiceCompletedReadbackV1]>,
}

#[derive(Debug)]
struct QualificationObservationTeardownFailureV1 {
    error: crate::M1QualificationObservationErrorV1,
    source: crate::M1PhysicalQueueReleaseFailureV1,
    compact_evidence: M1RearmedReadbackTeardownEvidenceV1,
    partial_logits: Box<[ServiceCompletedReadbackV1]>,
}

fn teardown_physical_qualification_observation_source(
    source: crate::M1QualificationObservationFailureV1,
) -> Result<QualificationObservationTeardownSuccessV1, Box<QualificationObservationTeardownFailureV1>>
{
    let (error, custody) = source.into_parts();
    match custody {
        crate::M1QualificationObservationFailureCustodyV1::Recycled(queue) => {
            match queue.destroy_and_release() {
                Ok(queue_release) => Ok(QualificationObservationTeardownSuccessV1 {
                    error,
                    queue_release,
                    compact_evidence: M1RearmedReadbackTeardownEvidenceV1::None,
                    partial_logits: Box::new([]),
                }),
                Err(source) => Err(Box::new(QualificationObservationTeardownFailureV1 {
                    error,
                    source,
                    compact_evidence: M1RearmedReadbackTeardownEvidenceV1::None,
                    partial_logits: Box::new([]),
                })),
            }
        }
        crate::M1QualificationObservationFailureCustodyV1::CompactRejected(output) => {
            match output.destroy_and_release_retaining_readback() {
                Ok((queue_release, readback)) => Ok(QualificationObservationTeardownSuccessV1 {
                    error,
                    queue_release,
                    compact_evidence: M1RearmedReadbackTeardownEvidenceV1::RejectedCompact(
                        Box::new(readback),
                    ),
                    partial_logits: Box::new([]),
                }),
                Err(failure) => {
                    let (source, readback) = *failure;
                    Err(Box::new(QualificationObservationTeardownFailureV1 {
                        error,
                        source,
                        compact_evidence: M1RearmedReadbackTeardownEvidenceV1::RejectedCompact(
                            Box::new(readback),
                        ),
                        partial_logits: Box::new([]),
                    }))
                }
            }
        }
        crate::M1QualificationObservationFailureCustodyV1::CompactSnapshotReadFailed(output) => {
            match output.destroy_and_release() {
                Ok(queue_release) => Ok(QualificationObservationTeardownSuccessV1 {
                    error,
                    queue_release,
                    compact_evidence: M1RearmedReadbackTeardownEvidenceV1::None,
                    partial_logits: Box::new([]),
                }),
                Err(source) => Err(Box::new(QualificationObservationTeardownFailureV1 {
                    error,
                    source,
                    compact_evidence: M1RearmedReadbackTeardownEvidenceV1::None,
                    partial_logits: Box::new([]),
                })),
            }
        }
        crate::M1QualificationObservationFailureCustodyV1::Observed {
            completion,
            partial_logits,
        } => match completion.destroy_and_release_retaining_image() {
            Ok((queue_release, image)) => Ok(QualificationObservationTeardownSuccessV1 {
                error,
                queue_release,
                compact_evidence: M1RearmedReadbackTeardownEvidenceV1::ObservedCompact(Box::new(
                    image,
                )),
                partial_logits,
            }),
            Err(failure) => {
                let (source, image) = *failure;
                Err(Box::new(QualificationObservationTeardownFailureV1 {
                    error,
                    source,
                    compact_evidence: M1RearmedReadbackTeardownEvidenceV1::ObservedCompact(
                        Box::new(image),
                    ),
                    partial_logits,
                }))
            }
        },
    }
}

/// Clean queue teardown after qualification observation failed.
///
/// Cache lineage is intentionally retained as terminal quarantine, not made
/// schedulable: the destroyed physical step still owned an uncommitted KV write
/// whose semantic completion could not be established. There is no sound page
/// return or active-cache recovery transition after that ambiguity. `engine` is
/// already fail-stopped before this value can be constructed.
#[must_use = "queue release and all failure custody remain retained"]
#[derive(Debug)]
pub struct M1RearmedQualificationObservationTeardownSuccessV1 {
    source: QualificationObservationTeardownSuccessV1,
    carry: M1RearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

impl M1RearmedQualificationObservationTeardownSuccessV1 {
    /// Original qualification-copy rejection retained through teardown.
    #[must_use]
    pub const fn error(&self) -> &crate::M1QualificationObservationErrorV1 {
        &self.source.error
    }

    /// Exact generic queue release observation.
    #[must_use]
    pub const fn queue_release(&self) -> ServiceQueueReleaseObservationV1 {
        self.source.queue_release
    }

    /// Compact readback or checked compact image retained after release.
    #[must_use = "compact qualification evidence remains retained"]
    pub const fn compact_evidence(&self) -> &M1RearmedReadbackTeardownEvidenceV1 {
        &self.source.compact_evidence
    }

    #[must_use]
    pub const fn capture_release_state(&self) -> M1RearmedReadbackCaptureReleaseStateV1 {
        M1RearmedReadbackCaptureReleaseStateV1::Released
    }

    /// Number of final-logits rows copied before the original rejection.
    #[must_use]
    pub const fn partial_logits_count(&self) -> usize {
        self.source.partial_logits.len()
    }

    /// Exact final-logits row readbacks copied before the rejection.
    #[must_use = "partial qualification logits remain retained"]
    pub fn partial_logits(&self) -> &[ServiceCompletedReadbackV1] {
        &self.source.partial_logits
    }

    #[must_use]
    pub fn partial_logits_row_bytes(&self, index: usize) -> Option<&[u8]> {
        self.source
            .partial_logits
            .get(index)
            .map(ServiceCompletedReadbackV1::bytes)
    }

    /// Number of selected and parked caches retained after teardown.
    #[must_use]
    pub const fn retained_cache_count(&self) -> usize {
        self.carry.selected.len() + self.carry.parked.len()
    }

    /// Selected requests retained in terminal scheduler order.
    #[must_use]
    pub fn selected_requests(&self) -> impl ExactSizeIterator<Item = RequestId> + '_ {
        self.carry
            .selected
            .iter()
            .map(|cache| cache.projection().request)
    }

    /// Active caches parked outside the failed selected roster.
    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.carry.parked.len()
    }

    #[must_use]
    pub const fn terminal_lineage_count(&self) -> usize {
        self.carry.terminal.len()
    }

    #[must_use]
    pub const fn previous_epoch(&self) -> CompletionEpoch {
        self.carry.previous_epoch
    }

    pub const fn prior_checked(&self) -> &crate::M1CheckedCompletionOutputV1 {
        &self.carry.prior_checked
    }

    #[must_use]
    pub fn prior_logical_accepted_counts(&self) -> &[u32] {
        &self.carry.logical_accepted_counts
    }

    #[must_use]
    pub fn prior_externally_published_counts(&self) -> &[u32] {
        &self.carry.externally_published_counts
    }

    #[must_use]
    pub fn prior_release_counts(&self) -> &[M1CompletedKvPageReleaseCountsV1] {
        &self.carry.release_counts
    }

    #[must_use]
    pub const fn prior_completed_members(&self) -> usize {
        self.carry.completed_members
    }

    #[must_use]
    pub const fn prior_total_released(&self) -> usize {
        self.carry.total_released
    }

    /// Exact completed queue generation observed before teardown.
    #[must_use]
    pub const fn queue_observation(&self) -> ComputeAqlQueueObservationV1 {
        self.queue_observation
    }

    /// Checked physical-device receipt retained through teardown.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.device
    }
}

/// Terminal queue-release failure retaining qualification and rearm custody.
///
/// As with the clean queue-release result, selected and parked caches remain
/// deliberately inert because no semantic completion exists for their pending
/// write. The additionally failed native release remains quarantined alongside
/// that fail-stopped lineage.
#[must_use = "terminal release quarantine and cache lineage remain retained"]
#[derive(Debug)]
pub struct M1RearmedQualificationObservationTeardownFailureV1 {
    source: Box<QualificationObservationTeardownFailureV1>,
    carry: M1RearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

impl M1RearmedQualificationObservationTeardownFailureV1 {
    /// Original qualification-copy rejection retained through release failure.
    #[must_use]
    pub const fn error(&self) -> &crate::M1QualificationObservationErrorV1 {
        &self.source.error
    }

    /// Terminal lower queue release failure.
    pub const fn source(&self) -> &crate::M1PhysicalQueueReleaseFailureV1 {
        &self.source.source
    }

    /// Compact readback or checked compact image retained beside release failure.
    #[must_use = "compact qualification evidence remains retained"]
    pub const fn compact_evidence(&self) -> &M1RearmedReadbackTeardownEvidenceV1 {
        &self.source.compact_evidence
    }

    #[must_use]
    pub const fn capture_release_state(&self) -> M1RearmedReadbackCaptureReleaseStateV1 {
        M1RearmedReadbackCaptureReleaseStateV1::LowerReleaseFailure
    }

    /// Number of final-logits rows copied before the original rejection.
    #[must_use]
    pub const fn partial_logits_count(&self) -> usize {
        self.source.partial_logits.len()
    }

    /// Exact final-logits row readbacks retained beside release failure.
    #[must_use = "partial qualification logits remain retained"]
    pub fn partial_logits(&self) -> &[ServiceCompletedReadbackV1] {
        &self.source.partial_logits
    }

    #[must_use]
    pub fn partial_logits_row_bytes(&self, index: usize) -> Option<&[u8]> {
        self.source
            .partial_logits
            .get(index)
            .map(ServiceCompletedReadbackV1::bytes)
    }

    /// Number of selected and parked caches retained beside quarantine.
    #[must_use]
    pub const fn retained_cache_count(&self) -> usize {
        self.carry.selected.len() + self.carry.parked.len()
    }

    /// Selected requests retained in terminal scheduler order.
    #[must_use]
    pub fn selected_requests(&self) -> impl ExactSizeIterator<Item = RequestId> + '_ {
        self.carry
            .selected
            .iter()
            .map(|cache| cache.projection().request)
    }

    /// Active caches parked outside the failed selected roster.
    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.carry.parked.len()
    }

    #[must_use]
    pub const fn terminal_lineage_count(&self) -> usize {
        self.carry.terminal.len()
    }

    #[must_use]
    pub const fn previous_epoch(&self) -> CompletionEpoch {
        self.carry.previous_epoch
    }

    pub const fn prior_checked(&self) -> &crate::M1CheckedCompletionOutputV1 {
        &self.carry.prior_checked
    }

    #[must_use]
    pub fn prior_logical_accepted_counts(&self) -> &[u32] {
        &self.carry.logical_accepted_counts
    }

    #[must_use]
    pub fn prior_externally_published_counts(&self) -> &[u32] {
        &self.carry.externally_published_counts
    }

    #[must_use]
    pub fn prior_release_counts(&self) -> &[M1CompletedKvPageReleaseCountsV1] {
        &self.carry.release_counts
    }

    #[must_use]
    pub const fn prior_completed_members(&self) -> usize {
        self.carry.completed_members
    }

    #[must_use]
    pub const fn prior_total_released(&self) -> usize {
        self.carry.total_released
    }

    /// Exact completed queue generation retained beside release quarantine.
    #[must_use]
    pub const fn queue_observation(&self) -> ComputeAqlQueueObservationV1 {
        self.queue_observation
    }

    /// Checked physical-device receipt retained beside release quarantine.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.device
    }
}

/// Move-only structural completion observation with complete rearm lineage.
///
/// The copied K7 image is inert: inspecting its records grants no completion,
/// KV, inference, or publication authority. Semantic rejection retains this
/// same owner, so corrected expectations never repeat the physical read.
///
/// ```compile_fail
/// use ferric_engine::M1RearmedObservedCompletionOutputV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1RearmedObservedCompletionOutputV1>();
/// ```
#[must_use = "observed completion must be checked, destroyed, or retained"]
#[derive(Debug)]
pub struct M1RearmedObservedCompletionOutputV1 {
    observed: crate::M1ObservedCompletionOutputV1,
    carry: M1RearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

impl M1RearmedObservedCompletionOutputV1 {
    /// Borrows the inert copied K7 image and structurally decoded records.
    #[must_use = "the observed image remains paired with rearm custody"]
    pub const fn image(&self) -> &crate::M1ObservedCompletionImageV1 {
        self.observed.image()
    }

    /// Selected request owners in exact scheduler order.
    #[must_use]
    pub fn selected_requests(&self) -> impl ExactSizeIterator<Item = RequestId> + '_ {
        self.carry
            .selected
            .iter()
            .map(|cache| cache.projection().request)
    }

    /// Number of active caches parked outside the observed selected roster.
    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.carry.parked.len()
    }

    /// Number of terminal members retained from the predecessor round.
    #[must_use]
    pub const fn terminal_count(&self) -> usize {
        self.carry.terminal.len()
    }

    /// Exact predecessor completion epoch.
    #[must_use]
    pub const fn previous_epoch(&self) -> CompletionEpoch {
        self.carry.previous_epoch
    }

    /// Exact completed queue generation observation.
    #[must_use]
    pub const fn queue_observation(&self) -> ComputeAqlQueueObservationV1 {
        self.queue_observation
    }

    /// Checked physical-device receipt retained through observation.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.device
    }

    /// Fail-stops `engine`, destroys the unchecked observed queue, and retains
    /// the exact copied image with all continuation lineage.
    ///
    /// This path is for caller-side invariant failures after inspection but
    /// before semantic join. It does not mint completion or KV authority.
    ///
    /// # Errors
    ///
    /// Returns terminal lower release quarantine paired with the same copied
    /// image, device receipt, queue observation, and cache owners.
    pub fn destroy_queue_and_retain_custody<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<M1RearmedReadbackTeardownSuccessV1, Box<M1RearmedReadbackTeardownFailureV1>> {
        quarantine_readback_teardown(engine);
        let Self {
            observed,
            carry,
            queue_observation,
            device,
        } = self;
        match teardown_unchecked_rearmed_observation(observed) {
            Ok(source) => Ok(M1RearmedReadbackTeardownSuccessV1 {
                diagnostic: source.diagnostic,
                queue_release: source.queue_release,
                evidence: source.evidence,
                carry,
                queue_observation,
                device,
            }),
            Err(source) => {
                let source = *source;
                Err(Box::new(M1RearmedReadbackTeardownFailureV1 {
                    diagnostic: source.diagnostic,
                    source: source.source,
                    evidence: source.evidence,
                    carry,
                    queue_observation,
                    device,
                }))
            }
        }
    }

    /// Consumes the copied image through the existing roster and semantic join.
    ///
    /// Qualification-capture queues retain their existing restriction to prompt
    /// commits; a rejected expectation preserves the already-copied observation
    /// and cannot return to recycled readback custody.
    ///
    /// # Errors
    ///
    /// Returns the exact semantic rejection together with the same observation
    /// and every rearm continuation owner. Corrected expectations can retry the
    /// join without issuing another physical read.
    pub fn check_completion(
        self,
        expectations: &[crate::CompletionWireSemanticExpectation<'_>],
    ) -> Result<M1RearmedCompletedReadbackV1, Box<M1RearmedReadbackFailureV1>> {
        if let Err(gate) = validate_rearmed_generic_semantics(
            self.observed.qualification_logits_enabled(),
            self.observed.direct_diagnostic_choices_enabled(),
            self.observed.speculative_diagnostic_choices_enabled(),
            expectations,
        ) {
            let Self {
                observed,
                carry,
                queue_observation,
                device,
            } = self;
            let source = match gate {
                M1RearmedGenericSemanticGateV1::Qualification { lane } => {
                    M1RearmedReadbackFailureSourceV1::ObservedQualificationCaptureRequiresEvidence {
                        lane,
                        observed: Box::new(observed),
                    }
                }
                M1RearmedGenericSemanticGateV1::DirectDiagnostic => {
                    M1RearmedReadbackFailureSourceV1::ObservedDirectDiagnosticCaptureRequiresEvidence {
                        observed: Box::new(observed),
                    }
                }
                M1RearmedGenericSemanticGateV1::SpeculativeDiagnostic => {
                    M1RearmedReadbackFailureSourceV1::ObservedSpeculativeDiagnosticCaptureRequiresEvidence {
                        observed: Box::new(observed),
                    }
                }
            };
            return Err(Box::new(M1RearmedReadbackFailureV1 {
                source,
                carry,
                queue_observation,
                device,
            }));
        }
        let Self {
            observed,
            carry,
            queue_observation,
            device,
        } = self;
        rejoin_rearmed_readback(observed, expectations, carry, queue_observation, device)
    }
}

/// Move-only final qualification observation with complete rearm lineage.
///
/// Semantic rejection returns this same owner, so corrected expectations never
/// repeat either the compact-output read or a logits-row read.
///
/// ```compile_fail
/// use ferric_engine::M1RearmedObservedQualificationOutputV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1RearmedObservedQualificationOutputV1>();
/// ```
#[must_use = "final qualification observation must be checked or retained"]
#[derive(Debug)]
pub struct M1RearmedObservedQualificationOutputV1 {
    observed: crate::M1ObservedQualificationOutputV1,
    carry: M1RearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

impl M1RearmedObservedQualificationOutputV1 {
    /// Already-copied compact and final-logits evidence.
    #[must_use = "qualification evidence remains retained by this observation"]
    pub const fn evidence(&self) -> &crate::M1QualificationCompletionEvidenceV1 {
        self.observed.evidence()
    }

    /// Selected request owners in exact scheduler order.
    #[must_use]
    pub fn selected_requests(&self) -> impl ExactSizeIterator<Item = RequestId> + '_ {
        self.carry
            .selected
            .iter()
            .map(|cache| cache.projection().request)
    }

    /// Number of active caches parked outside the retried selected roster.
    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.carry.parked.len()
    }

    /// Exact completed queue generation observation.
    #[must_use]
    pub const fn queue_observation(&self) -> ComputeAqlQueueObservationV1 {
        self.queue_observation
    }

    /// Checked physical-device receipt retained through observation.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.device
    }

    /// Exact predecessor completion epoch.
    #[must_use]
    pub const fn previous_epoch(&self) -> CompletionEpoch {
        self.carry.previous_epoch
    }

    /// Quarantines the Engine and destroys the observed physical queue while
    /// retaining the copied qualification evidence and complete round lineage.
    ///
    /// This transition is reserved for caller-side invariant failures that
    /// occur after a successful terminal observation but before semantic join.
    ///
    /// # Errors
    ///
    /// Returns terminal lower queue-release quarantine joined to the same
    /// evidence and lineage.
    pub fn destroy_queue_and_retain_custody<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1RearmedObservedQualificationTeardownSuccessV1,
        Box<M1RearmedObservedQualificationTeardownFailureV1>,
    > {
        quarantine_qualification_teardown(engine);
        let Self {
            observed,
            carry,
            queue_observation,
            device,
        } = self;
        let (completion, evidence) = observed.into_teardown_parts();
        let custody = M1RearmedObservedQualificationTeardownCustodyV1 {
            evidence,
            carry,
            queue_observation,
            device,
        };
        match completion.destroy_and_release() {
            Ok(queue_release) => Ok(M1RearmedObservedQualificationTeardownSuccessV1 {
                queue_release,
                custody,
            }),
            Err(source) => Err(Box::new(M1RearmedObservedQualificationTeardownFailureV1 {
                source,
                custody,
            })),
        }
    }

    /// Derives and joins the terminal finite BF16 argmax for every live lane.
    ///
    /// This is the only rearmed transition that admits
    /// `QualificationFinalRow`. Callers supply validated context-step witnesses
    /// but no token choices; choices come exclusively from the retained copied
    /// logits evidence.
    ///
    /// # Errors
    ///
    /// Returns the same already-copied qualification observation when numerical,
    /// context-roster, or compact K7 checking rejects.
    pub fn check_final_completion(
        self,
        contexts: &[crate::M1ValidatedQualificationContextStepV1],
    ) -> Result<
        M1RearmedQualifiedCompletedReadbackV1,
        M1RearmedQualificationCompletedReadbackJoinFailureV1,
    > {
        let Self {
            observed,
            carry,
            queue_observation,
            device,
        } = self;
        match observed.check_final_completion(contexts) {
            Ok(qualified) => {
                let (readback, evidence) = qualified.into_parts();
                Ok(M1RearmedQualifiedCompletedReadbackV1 {
                    readback: M1RearmedCompletedReadbackV1 {
                        readback,
                        carry,
                        queue_observation,
                        device,
                    },
                    evidence,
                })
            }
            Err(source) => {
                let (error, observed) = source.into_parts();
                Err(M1RearmedQualificationCompletedReadbackJoinFailureV1 {
                    error,
                    observed: Box::new(Self {
                        observed,
                        carry,
                        queue_observation,
                        device,
                    }),
                })
            }
        }
    }
}

#[derive(Debug)]
struct M1RearmedObservedQualificationTeardownCustodyV1 {
    evidence: crate::M1QualificationCompletionEvidenceV1,
    carry: M1RearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

/// Clean queue teardown after a caller-side terminal observation rejection.
#[must_use = "terminal observation evidence and lineage remain retained"]
#[derive(Debug)]
pub struct M1RearmedObservedQualificationTeardownSuccessV1 {
    queue_release: ServiceQueueReleaseObservationV1,
    custody: M1RearmedObservedQualificationTeardownCustodyV1,
}

/// Terminal queue-release quarantine after caller-side observation rejection.
#[must_use = "terminal observation release quarantine remains retained"]
#[derive(Debug)]
pub struct M1RearmedObservedQualificationTeardownFailureV1 {
    source: crate::M1PhysicalQueueReleaseFailureV1,
    custody: M1RearmedObservedQualificationTeardownCustodyV1,
}

impl M1RearmedObservedQualificationTeardownSuccessV1 {
    #[must_use = "qualification evidence remains retained"]
    pub const fn evidence(&self) -> &crate::M1QualificationCompletionEvidenceV1 {
        &self.custody.evidence
    }

    #[must_use]
    pub fn selected_requests(&self) -> impl ExactSizeIterator<Item = RequestId> + '_ {
        self.custody
            .carry
            .selected
            .iter()
            .map(|cache| cache.projection().request)
    }

    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.custody.carry.parked.len()
    }

    #[must_use]
    pub const fn terminal_count(&self) -> usize {
        self.custody.carry.terminal.len()
    }

    #[must_use]
    pub const fn previous_epoch(&self) -> CompletionEpoch {
        self.custody.carry.previous_epoch
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.custody.carry.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.custody.carry.history.get(index)
    }

    #[must_use]
    pub const fn queue_observation(&self) -> ComputeAqlQueueObservationV1 {
        self.custody.queue_observation
    }

    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.custody.device
    }

    #[must_use]
    pub const fn queue_release(&self) -> ServiceQueueReleaseObservationV1 {
        self.queue_release
    }
}

impl M1RearmedObservedQualificationTeardownFailureV1 {
    #[must_use = "qualification evidence remains retained"]
    pub const fn evidence(&self) -> &crate::M1QualificationCompletionEvidenceV1 {
        &self.custody.evidence
    }

    #[must_use]
    pub fn selected_requests(&self) -> impl ExactSizeIterator<Item = RequestId> + '_ {
        self.custody
            .carry
            .selected
            .iter()
            .map(|cache| cache.projection().request)
    }

    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.custody.carry.parked.len()
    }

    #[must_use]
    pub const fn terminal_count(&self) -> usize {
        self.custody.carry.terminal.len()
    }

    #[must_use]
    pub const fn previous_epoch(&self) -> CompletionEpoch {
        self.custody.carry.previous_epoch
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.custody.carry.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.custody.carry.history.get(index)
    }

    #[must_use]
    pub const fn queue_observation(&self) -> ComputeAqlQueueObservationV1 {
        self.custody.queue_observation
    }

    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.custody.device
    }

    pub const fn source(&self) -> &crate::M1PhysicalQueueReleaseFailureV1 {
        &self.source
    }
}

/// Semantic rejection retaining the same final qualification observation.
///
/// ```compile_fail
/// use ferric_engine::M1RearmedQualificationCompletedReadbackJoinFailureV1;
/// fn recover_twice(failure: M1RearmedQualificationCompletedReadbackJoinFailureV1) {
///     let _first = failure.into_parts();
///     let _second = failure.into_parts();
/// }
/// ```
#[must_use = "qualification join failure retains copied evidence and cache custody"]
#[derive(Debug)]
pub struct M1RearmedQualificationCompletedReadbackJoinFailureV1 {
    error: crate::M1CompletedReadbackJoinErrorV1,
    observed: Box<M1RearmedObservedQualificationOutputV1>,
}

impl M1RearmedQualificationCompletedReadbackJoinFailureV1 {
    /// Exact compact semantic rejection.
    #[must_use]
    pub const fn error(&self) -> &crate::M1CompletedReadbackJoinErrorV1 {
        &self.error
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.observed.carry.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.observed.carry.history.get(index)
    }

    /// Same already-copied qualification observation.
    #[must_use = "the rejected observation retains every linear owner"]
    pub const fn observed(&self) -> &M1RearmedObservedQualificationOutputV1 {
        &self.observed
    }

    /// Recovers the diagnostic and unchanged observation for corrected retry.
    #[must_use = "the observation remains the sole semantic retry owner"]
    pub fn into_parts(
        self,
    ) -> (
        crate::M1CompletedReadbackJoinErrorV1,
        M1RearmedObservedQualificationOutputV1,
    ) {
        (self.error, *self.observed)
    }

    /// Permanently quarantines `engine`, then destroys the semantically
    /// rejected physical queue while retaining the diagnostic, copied
    /// qualification evidence, and exact selected/parked cache lineage.
    ///
    /// A semantic mismatch cannot authorize the pending KV write even though
    /// its bytes were physically observed. The returned custody is therefore
    /// terminal and deliberately cannot re-enter scheduling.
    ///
    /// ```compile_fail
    /// use ferric_engine::{Engine, M1RearmedQualificationCompletedReadbackJoinFailureV1};
    /// fn teardown_twice(
    ///     engine: &mut Engine<32>,
    ///     failure: M1RearmedQualificationCompletedReadbackJoinFailureV1,
    /// ) {
    ///     let _first = failure.destroy_queue_and_retain_custody(engine);
    ///     let _second = failure.destroy_queue_and_retain_custody(engine);
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns terminal lower queue-release quarantine joined to the same
    /// semantic diagnostic, evidence, and cache lineage.
    pub fn destroy_queue_and_retain_custody<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1RearmedQualificationSemanticTeardownSuccessV1,
        Box<M1RearmedQualificationSemanticTeardownFailureV1>,
    > {
        quarantine_qualification_teardown(engine);
        let Self { error, observed } = self;
        let M1RearmedObservedQualificationOutputV1 {
            observed,
            carry,
            queue_observation,
            device,
        } = *observed;
        let (completion, evidence) = observed.into_teardown_parts();
        match completion.destroy_and_release() {
            Ok(queue_release) => Ok(M1RearmedQualificationSemanticTeardownSuccessV1 {
                error,
                evidence,
                queue_release,
                carry,
                queue_observation,
                device,
            }),
            Err(source) => Err(Box::new(M1RearmedQualificationSemanticTeardownFailureV1 {
                error,
                evidence,
                source,
                carry,
                queue_observation,
                device,
            })),
        }
    }
}

/// Clean queue teardown after terminal qualification semantic rejection.
///
/// The copied evidence and exact diagnostic remain inspectable, while selected
/// and parked caches are intentionally inert beside the fail-stopped Engine.
#[must_use = "semantic rejection evidence and cache quarantine remain retained"]
#[derive(Debug)]
pub struct M1RearmedQualificationSemanticTeardownSuccessV1 {
    error: crate::M1CompletedReadbackJoinErrorV1,
    evidence: crate::M1QualificationCompletionEvidenceV1,
    queue_release: ServiceQueueReleaseObservationV1,
    carry: M1RearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

impl M1RearmedQualificationSemanticTeardownSuccessV1 {
    /// Exact semantic rejection retained through teardown.
    #[must_use]
    pub const fn error(&self) -> &crate::M1CompletedReadbackJoinErrorV1 {
        &self.error
    }

    /// Copied compact and final-logits qualification evidence.
    #[must_use = "qualification evidence remains retained"]
    pub const fn evidence(&self) -> &crate::M1QualificationCompletionEvidenceV1 {
        &self.evidence
    }

    /// Exact generic queue release observation.
    #[must_use]
    pub const fn queue_release(&self) -> ServiceQueueReleaseObservationV1 {
        self.queue_release
    }

    /// Number of selected and parked caches retained in terminal quarantine.
    #[must_use]
    pub const fn retained_cache_count(&self) -> usize {
        self.carry.selected.len() + self.carry.parked.len()
    }

    /// Selected requests retained in exact scheduler order.
    #[must_use]
    pub fn selected_requests(&self) -> impl ExactSizeIterator<Item = RequestId> + '_ {
        self.carry
            .selected
            .iter()
            .map(|cache| cache.projection().request)
    }

    /// Active caches parked outside the rejected selected roster.
    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.carry.parked.len()
    }

    /// Exact predecessor completion epoch.
    #[must_use]
    pub const fn previous_epoch(&self) -> CompletionEpoch {
        self.carry.previous_epoch
    }

    /// Exact completed queue generation observed before teardown.
    #[must_use]
    pub const fn queue_observation(&self) -> ComputeAqlQueueObservationV1 {
        self.queue_observation
    }

    /// Checked physical-device receipt retained through teardown.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.device
    }
}

/// Terminal queue-release failure after qualification semantic rejection.
///
/// The failed lower release remains joined to the semantic diagnostic, copied
/// evidence, and exact selected/parked cache quarantine.
#[must_use = "semantic and queue-release quarantine remain retained"]
#[derive(Debug)]
pub struct M1RearmedQualificationSemanticTeardownFailureV1 {
    error: crate::M1CompletedReadbackJoinErrorV1,
    evidence: crate::M1QualificationCompletionEvidenceV1,
    source: crate::M1PhysicalQueueReleaseFailureV1,
    carry: M1RearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

impl M1RearmedQualificationSemanticTeardownFailureV1 {
    /// Exact semantic rejection retained beside release quarantine.
    #[must_use]
    pub const fn error(&self) -> &crate::M1CompletedReadbackJoinErrorV1 {
        &self.error
    }

    /// Copied compact and final-logits qualification evidence.
    #[must_use = "qualification evidence remains retained"]
    pub const fn evidence(&self) -> &crate::M1QualificationCompletionEvidenceV1 {
        &self.evidence
    }

    /// Terminal lower queue-release failure.
    pub const fn source(&self) -> &crate::M1PhysicalQueueReleaseFailureV1 {
        &self.source
    }

    /// Number of selected and parked caches retained in terminal quarantine.
    #[must_use]
    pub const fn retained_cache_count(&self) -> usize {
        self.carry.selected.len() + self.carry.parked.len()
    }

    /// Selected requests retained in exact scheduler order.
    #[must_use]
    pub fn selected_requests(&self) -> impl ExactSizeIterator<Item = RequestId> + '_ {
        self.carry
            .selected
            .iter()
            .map(|cache| cache.projection().request)
    }

    /// Active caches parked outside the rejected selected roster.
    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.carry.parked.len()
    }

    /// Exact predecessor completion epoch.
    #[must_use]
    pub const fn previous_epoch(&self) -> CompletionEpoch {
        self.carry.previous_epoch
    }

    /// Exact completed queue generation retained beside release quarantine.
    #[must_use]
    pub const fn queue_observation(&self) -> ComputeAqlQueueObservationV1 {
        self.queue_observation
    }

    /// Checked physical-device receipt retained beside release quarantine.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.device
    }
}

/// Exact observation or semantic-join failure retaining all linear custody.
#[derive(Debug)]
pub enum M1RearmedReadbackFailureSourceV1 {
    /// Generic checking cannot retire a queue that still owns qualification
    /// logits; only a prompt commit may bypass the terminal evidence join.
    QualificationCaptureRequiresEvidence {
        lane: usize,
        queue: Box<crate::M1PhysicalRecycledQueueSessionV1>,
    },
    /// Post-copy semantic gate rejection retaining the inert observation.
    ObservedQualificationCaptureRequiresEvidence {
        lane: usize,
        observed: Box<crate::M1ObservedCompletionOutputV1>,
    },
    /// Generic checking cannot retire a queue with attached direct-choice evidence.
    DirectDiagnosticCaptureRequiresEvidence {
        queue: Box<crate::M1PhysicalRecycledQueueSessionV1>,
    },
    /// Post-copy direct-choice gate rejection retaining the inert observation.
    ObservedDirectDiagnosticCaptureRequiresEvidence {
        observed: Box<crate::M1ObservedCompletionOutputV1>,
    },
    /// Generic checking cannot retire a queue with attached speculative-choice evidence.
    SpeculativeDiagnosticCaptureRequiresEvidence {
        queue: Box<crate::M1PhysicalRecycledQueueSessionV1>,
    },
    /// Post-copy speculative-choice gate rejection retaining the inert observation.
    ObservedSpeculativeDiagnosticCaptureRequiresEvidence {
        observed: Box<crate::M1ObservedCompletionOutputV1>,
    },
    /// Physical copy or structural observation rejection.
    Observation(crate::M1CompletionObservationFailureV1),
    /// Scheduler-roster, plan, wire-identity, or token-semantic rejection.
    Join(crate::M1CompletedReadbackJoinFailureV1),
}

/// Exact-readback rejection retaining recycled or observed queue custody.
#[must_use = "readback rejection and all continuation custody must remain retained"]
#[derive(Debug)]
pub struct M1RearmedReadbackFailureV1 {
    source: M1RearmedReadbackFailureSourceV1,
    carry: M1RearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

impl M1RearmedReadbackFailureV1 {
    /// Returns the exact observation or semantic rejection by borrow.
    #[must_use]
    pub const fn source(&self) -> &M1RearmedReadbackFailureSourceV1 {
        &self.source
    }

    /// Recovers untouched recycled custody after a pre-read semantic rejection.
    ///
    /// Observation and post-copy join failures remain in their existing closed
    /// states. Only the qualification-capture gate runs before a physical read,
    /// so only that variant can safely return to recycled custody.
    ///
    /// # Errors
    ///
    /// Returns the unchanged failure for every post-read rejection.
    pub fn recover_recycled_after_semantic_rejection(
        self: Box<Self>,
    ) -> Result<M1RearmedRecycledQueueV1, Box<Self>> {
        let Self {
            source,
            carry,
            queue_observation,
            device,
        } = *self;
        match source {
            M1RearmedReadbackFailureSourceV1::QualificationCaptureRequiresEvidence {
                queue,
                ..
            }
            | M1RearmedReadbackFailureSourceV1::DirectDiagnosticCaptureRequiresEvidence { queue }
            | M1RearmedReadbackFailureSourceV1::SpeculativeDiagnosticCaptureRequiresEvidence {
                queue,
            } => Ok(M1RearmedRecycledQueueV1 {
                queue: *queue,
                carry,
                queue_observation,
                device,
            }),
            source => Err(Box::new(Self {
                source,
                carry,
                queue_observation,
                device,
            })),
        }
    }

    /// Recovers an already-copied observation after a semantic rejection.
    ///
    /// Both the post-copy qualification gate and the ordinary completion join
    /// retain the same inert image. Pre-copy and structurally rejected states
    /// remain in their existing phase-accurate failure owner.
    ///
    /// # Errors
    ///
    /// Returns the unchanged failure when no complete structural observation
    /// exists.
    pub fn recover_observed_after_semantic_rejection(
        self: Box<Self>,
    ) -> Result<M1RearmedObservedCompletionOutputV1, Box<Self>> {
        let Self {
            source,
            carry,
            queue_observation,
            device,
        } = *self;
        match source {
            M1RearmedReadbackFailureSourceV1::ObservedQualificationCaptureRequiresEvidence {
                observed,
                ..
            }
            | M1RearmedReadbackFailureSourceV1::ObservedDirectDiagnosticCaptureRequiresEvidence {
                observed,
            }
            | M1RearmedReadbackFailureSourceV1::ObservedSpeculativeDiagnosticCaptureRequiresEvidence {
                observed,
            } => Ok(M1RearmedObservedCompletionOutputV1 {
                observed: *observed,
                carry,
                queue_observation,
                device,
            }),
            M1RearmedReadbackFailureSourceV1::Join(source) => {
                let (_error, observed) = source.into_parts();
                Ok(M1RearmedObservedCompletionOutputV1 {
                    observed,
                    carry,
                    queue_observation,
                    device,
                })
            }
            source => Err(Box::new(Self {
                source,
                carry,
                queue_observation,
                device,
            })),
        }
    }

    /// Retries only a lower pre-copy structural observation rejection.
    ///
    /// Success returns the split observed owner so the caller can inspect the
    /// copied token before supplying semantic expectations. Rejected or opaque
    /// post-copy lower states, semantic failures, and qualification gates return
    /// unchanged custody and never issue another completed read.
    ///
    /// # Errors
    ///
    /// Returns unchanged closed custody when retry is not admitted, or a fresh
    /// phase-accurate observation failure if the lower retry rejects again.
    pub fn retry_observation(
        self: Box<Self>,
    ) -> Result<M1RearmedObservedCompletionOutputV1, Box<Self>> {
        let Self {
            source,
            carry,
            queue_observation,
            device,
        } = *self;
        match source {
            M1RearmedReadbackFailureSourceV1::Observation(source) => {
                let (error, custody) = source.into_parts();
                match custody {
                    crate::M1CompletionObservationFailureCustodyV1::Recycled(queue) => {
                        match queue.observe_completion() {
                            Ok(observed) => Ok(M1RearmedObservedCompletionOutputV1 {
                                observed,
                                carry,
                                queue_observation,
                                device,
                            }),
                            Err(source) => Err(Box::new(Self {
                                source: M1RearmedReadbackFailureSourceV1::Observation(source),
                                carry,
                                queue_observation,
                                device,
                            })),
                        }
                    }
                    custody => Err(Box::new(Self {
                        source: M1RearmedReadbackFailureSourceV1::Observation(
                            crate::M1CompletionObservationFailureV1::from_parts(error, custody),
                        ),
                        carry,
                        queue_observation,
                        device,
                    })),
                }
            }
            source => Err(Box::new(Self {
                source,
                carry,
                queue_observation,
                device,
            })),
        }
    }

    /// Retries the exact admissible stage without reopening a successful copy.
    ///
    /// A lower `Recycled` observation retries the physical read and then joins
    /// the supplied semantics. A lower `Rejected` observation and the pre-read
    /// qualification gate return this owner unchanged. A semantic `Join`
    /// failure retries only the retained observation join.
    ///
    /// ```compile_fail
    /// use ferric_engine::M1RearmedReadbackFailureV1;
    /// fn retry_twice(failure: Box<M1RearmedReadbackFailureV1>) {
    ///     let expectations = [];
    ///     let _first = failure.retry(&expectations);
    ///     let _second = failure.retry(&expectations);
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns unchanged closed custody when retry is not admitted, or fresh
    /// phase-accurate failure custody if the admitted retry rejects again.
    pub fn retry(
        self: Box<Self>,
        expectations: &[crate::CompletionWireSemanticExpectation<'_>],
    ) -> Result<M1RearmedCompletedReadbackV1, Box<Self>> {
        if matches!(
            &self.source,
            M1RearmedReadbackFailureSourceV1::QualificationCaptureRequiresEvidence { .. }
                | M1RearmedReadbackFailureSourceV1::DirectDiagnosticCaptureRequiresEvidence { .. }
                | M1RearmedReadbackFailureSourceV1::SpeculativeDiagnosticCaptureRequiresEvidence { .. }
        ) {
            return Err(self);
        }
        if let M1RearmedReadbackFailureSourceV1::Observation(source) = &self.source {
            if matches!(
                source.custody(),
                crate::M1CompletionObservationFailureCustodyV1::Rejected(_)
                    | crate::M1CompletionObservationFailureCustodyV1::SnapshotReadFailed(_)
            ) {
                return Err(self);
            }
        }
        let capture_enabled = match &self.source {
            M1RearmedReadbackFailureSourceV1::QualificationCaptureRequiresEvidence { .. }
            | M1RearmedReadbackFailureSourceV1::ObservedQualificationCaptureRequiresEvidence {
                ..
            } => (true, false, false),
            M1RearmedReadbackFailureSourceV1::DirectDiagnosticCaptureRequiresEvidence { .. }
            | M1RearmedReadbackFailureSourceV1::ObservedDirectDiagnosticCaptureRequiresEvidence {
                ..
            } => (false, true, false),
            M1RearmedReadbackFailureSourceV1::SpeculativeDiagnosticCaptureRequiresEvidence {
                ..
            }
            | M1RearmedReadbackFailureSourceV1::ObservedSpeculativeDiagnosticCaptureRequiresEvidence {
                ..
            } => (false, false, true),
            M1RearmedReadbackFailureSourceV1::Observation(source) => match source.custody() {
                crate::M1CompletionObservationFailureCustodyV1::Recycled(queue) => {
                    let output = queue.custody().completion_output();
                    (
                        output.qualification_logits().is_some(),
                        output.direct_diagnostic_choices().is_some(),
                        output.speculative_diagnostic_choices().is_some(),
                    )
                }
                crate::M1CompletionObservationFailureCustodyV1::Rejected(_)
                | crate::M1CompletionObservationFailureCustodyV1::SnapshotReadFailed(_) => {
                    return Err(self);
                }
            },
            M1RearmedReadbackFailureSourceV1::Join(source) => (
                source.observed().qualification_logits_enabled(),
                source.observed().direct_diagnostic_choices_enabled(),
                source.observed().speculative_diagnostic_choices_enabled(),
            ),
        };
        if validate_rearmed_generic_semantics(
            capture_enabled.0,
            capture_enabled.1,
            capture_enabled.2,
            expectations,
        )
        .is_err()
        {
            return Err(self);
        }
        let Self {
            source,
            carry,
            queue_observation,
            device,
        } = *self;
        match source {
            M1RearmedReadbackFailureSourceV1::QualificationCaptureRequiresEvidence {
                lane,
                queue,
            } => Err(Box::new(Self {
                source: M1RearmedReadbackFailureSourceV1::QualificationCaptureRequiresEvidence {
                    lane,
                    queue,
                },
                carry,
                queue_observation,
                device,
            })),
            M1RearmedReadbackFailureSourceV1::DirectDiagnosticCaptureRequiresEvidence {
                queue,
            } => Err(Box::new(Self {
                source: M1RearmedReadbackFailureSourceV1::DirectDiagnosticCaptureRequiresEvidence {
                    queue,
                },
                carry,
                queue_observation,
                device,
            })),
            M1RearmedReadbackFailureSourceV1::SpeculativeDiagnosticCaptureRequiresEvidence {
                queue,
            } => Err(Box::new(Self {
                source:
                    M1RearmedReadbackFailureSourceV1::SpeculativeDiagnosticCaptureRequiresEvidence {
                        queue,
                    },
                carry,
                queue_observation,
                device,
            })),
            M1RearmedReadbackFailureSourceV1::ObservedQualificationCaptureRequiresEvidence {
                observed,
                ..
            } => rejoin_rearmed_readback(*observed, expectations, carry, queue_observation, device),
            M1RearmedReadbackFailureSourceV1::ObservedDirectDiagnosticCaptureRequiresEvidence {
                observed,
            } => Err(Box::new(Self {
                source:
                    M1RearmedReadbackFailureSourceV1::ObservedDirectDiagnosticCaptureRequiresEvidence {
                        observed,
                    },
                carry,
                queue_observation,
                device,
            })),
            M1RearmedReadbackFailureSourceV1::ObservedSpeculativeDiagnosticCaptureRequiresEvidence {
                observed,
            } => Err(Box::new(Self {
                source:
                    M1RearmedReadbackFailureSourceV1::ObservedSpeculativeDiagnosticCaptureRequiresEvidence {
                        observed,
                    },
                carry,
                queue_observation,
                device,
            })),
            M1RearmedReadbackFailureSourceV1::Observation(source) => {
                let (error, custody) = source.into_parts();
                match custody {
                    crate::M1CompletionObservationFailureCustodyV1::Recycled(queue) => {
                        match queue.observe_completion() {
                            Ok(observed) => rejoin_rearmed_readback(
                                observed,
                                expectations,
                                carry,
                                queue_observation,
                                device,
                            ),
                            Err(source) => Err(Box::new(Self {
                                source: M1RearmedReadbackFailureSourceV1::Observation(source),
                                carry,
                                queue_observation,
                                device,
                            })),
                        }
                    }
                    crate::M1CompletionObservationFailureCustodyV1::Rejected(rejected) => {
                        Err(Box::new(Self {
                            source: M1RearmedReadbackFailureSourceV1::Observation(
                                crate::M1CompletionObservationFailureV1::from_parts(
                                    error,
                                    crate::M1CompletionObservationFailureCustodyV1::Rejected(
                                        rejected,
                                    ),
                                ),
                            ),
                            carry,
                            queue_observation,
                            device,
                        }))
                    }
                    crate::M1CompletionObservationFailureCustodyV1::SnapshotReadFailed(failed) => {
                        Err(Box::new(Self {
                            source: M1RearmedReadbackFailureSourceV1::Observation(
                                crate::M1CompletionObservationFailureV1::from_parts(
                                    error,
                                    crate::M1CompletionObservationFailureCustodyV1::SnapshotReadFailed(
                                        failed,
                                    ),
                                ),
                            ),
                            carry,
                            queue_observation,
                            device,
                        }))
                    }
                }
            }
            M1RearmedReadbackFailureSourceV1::Join(source) => {
                let (_error, observed) = source.into_parts();
                rejoin_rearmed_readback(observed, expectations, carry, queue_observation, device)
            }
        }
    }

    /// Fail-stops `engine` before destroying and releasing the physical queue.
    ///
    /// Every source state is accepted. Copied rejected or observed bytes remain
    /// attached as diagnostic evidence, and every continuation owner remains
    /// quarantined beside the exact lower release outcome.
    ///
    /// ```compile_fail
    /// use ferric_engine::{Engine, M1RearmedReadbackFailureV1};
    /// fn teardown_twice(engine: &mut Engine<32>, failure: Box<M1RearmedReadbackFailureV1>) {
    ///     let _first = failure.destroy_queue_and_retain_custody(engine);
    ///     let _second = failure.destroy_queue_and_retain_custody(engine);
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns the exact terminal lower release owner together with the same
    /// diagnostic, copied evidence, and rearm continuation custody.
    pub fn destroy_queue_and_retain_custody<const C: usize>(
        self: Box<Self>,
        engine: &mut Engine<C>,
    ) -> Result<M1RearmedReadbackTeardownSuccessV1, Box<M1RearmedReadbackTeardownFailureV1>> {
        quarantine_readback_teardown(engine);
        let Self {
            source,
            carry,
            queue_observation,
            device,
        } = *self;
        match teardown_rearmed_readback_source(source) {
            Ok(source) => Ok(M1RearmedReadbackTeardownSuccessV1 {
                diagnostic: source.diagnostic,
                queue_release: source.queue_release,
                evidence: source.evidence,
                carry,
                queue_observation,
                device,
            }),
            Err(source) => {
                let source = *source;
                Err(Box::new(M1RearmedReadbackTeardownFailureV1 {
                    diagnostic: source.diagnostic,
                    source: source.source,
                    evidence: source.evidence,
                    carry,
                    queue_observation,
                    device,
                }))
            }
        }
    }

    /// Returns the queue observation retained from the completed generation.
    #[must_use]
    pub const fn queue_observation(&self) -> ComputeAqlQueueObservationV1 {
        self.queue_observation
    }

    /// Returns the checked physical-device receipt retained through failure.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.device
    }

    /// Returns the exact predecessor epoch retained by continuation custody.
    #[must_use]
    pub const fn previous_epoch(&self) -> CompletionEpoch {
        self.carry.previous_epoch
    }

    #[must_use]
    pub fn selected_requests(&self) -> impl ExactSizeIterator<Item = RequestId> + '_ {
        self.carry
            .selected
            .iter()
            .map(|cache| cache.projection().request)
    }

    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.carry.parked.len()
    }

    #[must_use]
    pub const fn terminal_count(&self) -> usize {
        self.carry.terminal.len()
    }

    pub const fn prior_checked(&self) -> &crate::M1CheckedCompletionOutputV1 {
        &self.carry.prior_checked
    }

    #[must_use]
    pub fn prior_logical_accepted_counts(&self) -> &[u32] {
        &self.carry.logical_accepted_counts
    }

    #[must_use]
    pub fn prior_externally_published_counts(&self) -> &[u32] {
        &self.carry.externally_published_counts
    }

    #[must_use]
    pub fn prior_release_counts(&self) -> &[M1CompletedKvPageReleaseCountsV1] {
        &self.carry.release_counts
    }

    #[must_use]
    pub const fn prior_completed_members(&self) -> usize {
        self.carry.completed_members
    }

    #[must_use]
    pub const fn prior_total_released(&self) -> usize {
        self.carry.total_released
    }
}

fn quarantine_readback_teardown<const C: usize>(engine: &mut Engine<C>) {
    engine.quarantine_m1_queue_rearm_failure();
}

#[derive(Debug)]
struct M1RearmedReadbackRetryJoinV1<T, Q> {
    source: T,
    carry: M1RearmContinuationCustodyV1,
    queue_observation: Q,
    device: Gfx942DeviceBinding,
}

fn rejoin_readback_retry<T, E, Q>(
    source: Result<T, E>,
    carry: M1RearmContinuationCustodyV1,
    queue_observation: Q,
    device: Gfx942DeviceBinding,
) -> Result<M1RearmedReadbackRetryJoinV1<T, Q>, Box<M1RearmedReadbackRetryJoinV1<E, Q>>> {
    match source {
        Ok(source) => Ok(M1RearmedReadbackRetryJoinV1 {
            source,
            carry,
            queue_observation,
            device,
        }),
        Err(source) => Err(Box::new(M1RearmedReadbackRetryJoinV1 {
            source,
            carry,
            queue_observation,
            device,
        })),
    }
}

fn rejoin_rearmed_readback(
    observed: crate::M1ObservedCompletionOutputV1,
    expectations: &[crate::CompletionWireSemanticExpectation<'_>],
    carry: M1RearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
) -> Result<M1RearmedCompletedReadbackV1, Box<M1RearmedReadbackFailureV1>> {
    match rejoin_readback_retry(
        observed.check_completion(expectations),
        carry,
        queue_observation,
        device,
    ) {
        Ok(joined) => Ok(M1RearmedCompletedReadbackV1 {
            readback: joined.source,
            carry: joined.carry,
            queue_observation: joined.queue_observation,
            device: joined.device,
        }),
        Err(joined) => {
            let joined = *joined;
            Err(Box::new(M1RearmedReadbackFailureV1 {
                source: M1RearmedReadbackFailureSourceV1::Join(joined.source),
                carry: joined.carry,
                queue_observation: joined.queue_observation,
                device: joined.device,
            }))
        }
    }
}

/// Exact diagnostic retained after generic rearm readback teardown.
#[derive(Debug)]
pub enum M1RearmedReadbackTeardownDiagnosticV1 {
    ObservedBeforeSemanticJoin,
    QualificationCaptureRequiresEvidence { lane: usize },
    DirectDiagnosticCaptureRequiresEvidence,
    SpeculativeDiagnosticCaptureRequiresEvidence,
    Observation(crate::M1CompletionObservationErrorV1),
    Join(crate::M1CompletedReadbackJoinErrorV1),
}

/// Copied completion evidence retained independently of allocation release.
#[derive(Debug)]
pub enum M1RearmedReadbackTeardownEvidenceV1 {
    None,
    RejectedCompact(Box<ServiceCompletedReadbackV1>),
    ObservedCompact(Box<crate::M1ObservedCompletionImageV1>),
}

impl M1RearmedReadbackTeardownEvidenceV1 {
    #[must_use]
    pub fn raw_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::None => None,
            Self::RejectedCompact(readback) => Some(readback.bytes()),
            Self::ObservedCompact(image) => Some(image.raw_bytes()),
        }
    }
}

/// Truthful allocation-release state after fail-stop readback teardown.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1RearmedReadbackCaptureReleaseStateV1 {
    /// The lower queue and its completion capture allocations were released.
    Released,
    /// Lower release failed; the returned physical owner remains quarantined.
    LowerReleaseFailure,
}

#[derive(Debug)]
struct RearmedReadbackTeardownSuccessV1 {
    diagnostic: M1RearmedReadbackTeardownDiagnosticV1,
    queue_release: ServiceQueueReleaseObservationV1,
    evidence: M1RearmedReadbackTeardownEvidenceV1,
}

#[derive(Debug)]
struct RearmedReadbackTeardownFailureV1 {
    diagnostic: M1RearmedReadbackTeardownDiagnosticV1,
    source: crate::M1PhysicalQueueReleaseFailureV1,
    evidence: M1RearmedReadbackTeardownEvidenceV1,
}

fn teardown_unchecked_rearmed_observation(
    observed: crate::M1ObservedCompletionOutputV1,
) -> Result<RearmedReadbackTeardownSuccessV1, Box<RearmedReadbackTeardownFailureV1>> {
    let diagnostic = M1RearmedReadbackTeardownDiagnosticV1::ObservedBeforeSemanticJoin;
    match observed.destroy_and_release_retaining_image() {
        Ok((queue_release, image)) => Ok(RearmedReadbackTeardownSuccessV1 {
            diagnostic,
            queue_release,
            evidence: M1RearmedReadbackTeardownEvidenceV1::ObservedCompact(Box::new(image)),
        }),
        Err(failure) => {
            let (source, image) = *failure;
            Err(Box::new(RearmedReadbackTeardownFailureV1 {
                diagnostic,
                source,
                evidence: M1RearmedReadbackTeardownEvidenceV1::ObservedCompact(Box::new(image)),
            }))
        }
    }
}

fn teardown_rearmed_readback_source(
    source: M1RearmedReadbackFailureSourceV1,
) -> Result<RearmedReadbackTeardownSuccessV1, Box<RearmedReadbackTeardownFailureV1>> {
    let finish = |diagnostic, evidence, released| match released {
        Ok(queue_release) => Ok(RearmedReadbackTeardownSuccessV1 {
            diagnostic,
            queue_release,
            evidence,
        }),
        Err(source) => Err(Box::new(RearmedReadbackTeardownFailureV1 {
            diagnostic,
            source,
            evidence,
        })),
    };
    match source {
        M1RearmedReadbackFailureSourceV1::QualificationCaptureRequiresEvidence { lane, queue } => {
            finish(
                M1RearmedReadbackTeardownDiagnosticV1::QualificationCaptureRequiresEvidence {
                    lane,
                },
                M1RearmedReadbackTeardownEvidenceV1::None,
                queue.destroy_and_release(),
            )
        }
        M1RearmedReadbackFailureSourceV1::ObservedQualificationCaptureRequiresEvidence {
            lane,
            observed,
        } => {
            let diagnostic =
                M1RearmedReadbackTeardownDiagnosticV1::QualificationCaptureRequiresEvidence {
                    lane,
                };
            match observed.destroy_and_release_retaining_image() {
                Ok((queue_release, image)) => Ok(RearmedReadbackTeardownSuccessV1 {
                    diagnostic,
                    queue_release,
                    evidence: M1RearmedReadbackTeardownEvidenceV1::ObservedCompact(Box::new(image)),
                }),
                Err(failure) => {
                    let (source, image) = *failure;
                    Err(Box::new(RearmedReadbackTeardownFailureV1 {
                        diagnostic,
                        source,
                        evidence: M1RearmedReadbackTeardownEvidenceV1::ObservedCompact(Box::new(
                            image,
                        )),
                    }))
                }
            }
        }
        M1RearmedReadbackFailureSourceV1::DirectDiagnosticCaptureRequiresEvidence { queue } => {
            finish(
                M1RearmedReadbackTeardownDiagnosticV1::DirectDiagnosticCaptureRequiresEvidence,
                M1RearmedReadbackTeardownEvidenceV1::None,
                queue.destroy_and_release(),
            )
        }
        M1RearmedReadbackFailureSourceV1::ObservedDirectDiagnosticCaptureRequiresEvidence {
            observed,
        } => {
            let diagnostic =
                M1RearmedReadbackTeardownDiagnosticV1::DirectDiagnosticCaptureRequiresEvidence;
            match observed.destroy_and_release_retaining_image() {
                Ok((queue_release, image)) => Ok(RearmedReadbackTeardownSuccessV1 {
                    diagnostic,
                    queue_release,
                    evidence: M1RearmedReadbackTeardownEvidenceV1::ObservedCompact(Box::new(image)),
                }),
                Err(failure) => {
                    let (source, image) = *failure;
                    Err(Box::new(RearmedReadbackTeardownFailureV1 {
                        diagnostic,
                        source,
                        evidence: M1RearmedReadbackTeardownEvidenceV1::ObservedCompact(Box::new(
                            image,
                        )),
                    }))
                }
            }
        }
        M1RearmedReadbackFailureSourceV1::SpeculativeDiagnosticCaptureRequiresEvidence {
            queue,
        } => finish(
            M1RearmedReadbackTeardownDiagnosticV1::SpeculativeDiagnosticCaptureRequiresEvidence,
            M1RearmedReadbackTeardownEvidenceV1::None,
            queue.destroy_and_release(),
        ),
        M1RearmedReadbackFailureSourceV1::ObservedSpeculativeDiagnosticCaptureRequiresEvidence {
            observed,
        } => {
            let diagnostic =
                M1RearmedReadbackTeardownDiagnosticV1::SpeculativeDiagnosticCaptureRequiresEvidence;
            match observed.destroy_and_release_retaining_image() {
                Ok((queue_release, image)) => Ok(RearmedReadbackTeardownSuccessV1 {
                    diagnostic,
                    queue_release,
                    evidence: M1RearmedReadbackTeardownEvidenceV1::ObservedCompact(Box::new(image)),
                }),
                Err(failure) => {
                    let (source, image) = *failure;
                    Err(Box::new(RearmedReadbackTeardownFailureV1 {
                        diagnostic,
                        source,
                        evidence: M1RearmedReadbackTeardownEvidenceV1::ObservedCompact(Box::new(
                            image,
                        )),
                    }))
                }
            }
        }
        M1RearmedReadbackFailureSourceV1::Observation(source) => {
            let (error, custody) = source.into_parts();
            match custody {
                crate::M1CompletionObservationFailureCustodyV1::Recycled(queue) => finish(
                    M1RearmedReadbackTeardownDiagnosticV1::Observation(error),
                    M1RearmedReadbackTeardownEvidenceV1::None,
                    queue.destroy_and_release(),
                ),
                crate::M1CompletionObservationFailureCustodyV1::Rejected(rejected) => {
                    let diagnostic = M1RearmedReadbackTeardownDiagnosticV1::Observation(error);
                    match rejected.destroy_and_release_retaining_readback() {
                        Ok((queue_release, readback)) => Ok(RearmedReadbackTeardownSuccessV1 {
                            diagnostic,
                            queue_release,
                            evidence: M1RearmedReadbackTeardownEvidenceV1::RejectedCompact(
                                Box::new(readback),
                            ),
                        }),
                        Err(failure) => {
                            let (source, readback) = *failure;
                            Err(Box::new(RearmedReadbackTeardownFailureV1 {
                                diagnostic,
                                source,
                                evidence: M1RearmedReadbackTeardownEvidenceV1::RejectedCompact(
                                    Box::new(readback),
                                ),
                            }))
                        }
                    }
                }
                crate::M1CompletionObservationFailureCustodyV1::SnapshotReadFailed(failed) => {
                    finish(
                        M1RearmedReadbackTeardownDiagnosticV1::Observation(error),
                        M1RearmedReadbackTeardownEvidenceV1::None,
                        failed.destroy_and_release(),
                    )
                }
            }
        }
        M1RearmedReadbackFailureSourceV1::Join(source) => {
            let (error, observed) = source.into_parts();
            let diagnostic = M1RearmedReadbackTeardownDiagnosticV1::Join(error);
            match observed.destroy_and_release_retaining_image() {
                Ok((queue_release, image)) => Ok(RearmedReadbackTeardownSuccessV1 {
                    diagnostic,
                    queue_release,
                    evidence: M1RearmedReadbackTeardownEvidenceV1::ObservedCompact(Box::new(image)),
                }),
                Err(failure) => {
                    let (source, image) = *failure;
                    Err(Box::new(RearmedReadbackTeardownFailureV1 {
                        diagnostic,
                        source,
                        evidence: M1RearmedReadbackTeardownEvidenceV1::ObservedCompact(Box::new(
                            image,
                        )),
                    }))
                }
            }
        }
    }
}

/// Clean fail-stop teardown retaining the exact readback diagnostic and lineage.
#[must_use = "readback diagnostic and terminal cache quarantine remain retained"]
#[derive(Debug)]
pub struct M1RearmedReadbackTeardownSuccessV1 {
    diagnostic: M1RearmedReadbackTeardownDiagnosticV1,
    queue_release: ServiceQueueReleaseObservationV1,
    evidence: M1RearmedReadbackTeardownEvidenceV1,
    carry: M1RearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

impl M1RearmedReadbackTeardownSuccessV1 {
    #[must_use]
    pub const fn diagnostic(&self) -> &M1RearmedReadbackTeardownDiagnosticV1 {
        &self.diagnostic
    }

    #[must_use]
    pub const fn queue_release(&self) -> ServiceQueueReleaseObservationV1 {
        self.queue_release
    }

    #[must_use = "copied evidence remains retained"]
    pub const fn evidence(&self) -> &M1RearmedReadbackTeardownEvidenceV1 {
        &self.evidence
    }

    /// Completion capture allocation release completed successfully.
    #[must_use]
    pub const fn capture_release_state(&self) -> M1RearmedReadbackCaptureReleaseStateV1 {
        M1RearmedReadbackCaptureReleaseStateV1::Released
    }

    #[must_use]
    pub fn selected_requests(&self) -> impl ExactSizeIterator<Item = RequestId> + '_ {
        self.carry
            .selected
            .iter()
            .map(|cache| cache.projection().request)
    }

    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.carry.parked.len()
    }

    #[must_use]
    pub const fn terminal_count(&self) -> usize {
        self.carry.terminal.len()
    }

    #[must_use]
    pub const fn previous_epoch(&self) -> CompletionEpoch {
        self.carry.previous_epoch
    }

    pub const fn prior_checked(&self) -> &crate::M1CheckedCompletionOutputV1 {
        &self.carry.prior_checked
    }

    #[must_use]
    pub fn prior_logical_accepted_counts(&self) -> &[u32] {
        &self.carry.logical_accepted_counts
    }

    #[must_use]
    pub fn prior_externally_published_counts(&self) -> &[u32] {
        &self.carry.externally_published_counts
    }

    #[must_use]
    pub fn prior_release_counts(&self) -> &[M1CompletedKvPageReleaseCountsV1] {
        &self.carry.release_counts
    }

    #[must_use]
    pub const fn prior_completed_members(&self) -> usize {
        self.carry.completed_members
    }

    #[must_use]
    pub const fn prior_total_released(&self) -> usize {
        self.carry.total_released
    }

    #[must_use]
    pub const fn queue_observation(&self) -> ComputeAqlQueueObservationV1 {
        self.queue_observation
    }

    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.device
    }
}

/// Terminal lower release failure retaining physical and Ferric custody.
#[must_use = "lower release owner and readback quarantine remain retained"]
#[derive(Debug)]
pub struct M1RearmedReadbackTeardownFailureV1 {
    diagnostic: M1RearmedReadbackTeardownDiagnosticV1,
    source: crate::M1PhysicalQueueReleaseFailureV1,
    evidence: M1RearmedReadbackTeardownEvidenceV1,
    carry: M1RearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

impl M1RearmedReadbackTeardownFailureV1 {
    #[must_use]
    pub const fn diagnostic(&self) -> &M1RearmedReadbackTeardownDiagnosticV1 {
        &self.diagnostic
    }

    pub const fn source(&self) -> &crate::M1PhysicalQueueReleaseFailureV1 {
        &self.source
    }

    #[must_use = "copied evidence remains retained"]
    pub const fn evidence(&self) -> &M1RearmedReadbackTeardownEvidenceV1 {
        &self.evidence
    }

    /// Lower release failed and the physical release owner remains retained.
    #[must_use]
    pub const fn capture_release_state(&self) -> M1RearmedReadbackCaptureReleaseStateV1 {
        M1RearmedReadbackCaptureReleaseStateV1::LowerReleaseFailure
    }

    #[must_use]
    pub fn selected_requests(&self) -> impl ExactSizeIterator<Item = RequestId> + '_ {
        self.carry
            .selected
            .iter()
            .map(|cache| cache.projection().request)
    }

    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.carry.parked.len()
    }

    #[must_use]
    pub const fn terminal_count(&self) -> usize {
        self.carry.terminal.len()
    }

    #[must_use]
    pub const fn previous_epoch(&self) -> CompletionEpoch {
        self.carry.previous_epoch
    }

    pub const fn prior_checked(&self) -> &crate::M1CheckedCompletionOutputV1 {
        &self.carry.prior_checked
    }

    #[must_use]
    pub fn prior_logical_accepted_counts(&self) -> &[u32] {
        &self.carry.logical_accepted_counts
    }

    #[must_use]
    pub fn prior_externally_published_counts(&self) -> &[u32] {
        &self.carry.externally_published_counts
    }

    #[must_use]
    pub fn prior_release_counts(&self) -> &[M1CompletedKvPageReleaseCountsV1] {
        &self.carry.release_counts
    }

    #[must_use]
    pub const fn prior_completed_members(&self) -> usize {
        self.carry.completed_members
    }

    #[must_use]
    pub const fn prior_total_released(&self) -> usize {
        self.carry.total_released
    }

    #[must_use]
    pub const fn queue_observation(&self) -> ComputeAqlQueueObservationV1 {
        self.queue_observation
    }

    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.device
    }
}

/// Joined completion readback paired with the selected and parked cache owners.
#[must_use = "joined rearm readback must complete KV custody or remain retained"]
#[derive(Debug)]
pub struct M1RearmedCompletedReadbackV1 {
    readback: crate::M1PhysicalCompletedReadbackV1,
    carry: M1RearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

/// Final qualification readback paired with selected and parked KV custody.
///
/// This owner exposes only a retiring completion transition. The caller must
/// first move every [`Self::selected_requests`] member into Engine retirement;
/// a successful physical completion then settles the final write, retires all
/// reachable pages, and preserves the copied evidence beside the outcome.
///
/// ```compile_fail
/// use ferric_engine::{Engine, M1RearmedQualifiedCompletedReadbackV1};
/// fn complete_twice(
///     engine: &mut Engine<32>,
///     readback: M1RearmedQualifiedCompletedReadbackV1,
/// ) {
///     let _first = readback.complete_retiring(engine);
///     let _second = readback.complete_retiring(engine);
/// }
/// ```
#[must_use = "qualified readback must retire its KV custody or remain retained"]
#[derive(Debug)]
pub struct M1RearmedQualifiedCompletedReadbackV1 {
    readback: M1RearmedCompletedReadbackV1,
    evidence: crate::M1QualificationCompletionEvidenceV1,
}

impl M1RearmedQualifiedCompletedReadbackV1 {
    /// Already-copied compact and final-logits qualification evidence.
    #[must_use = "qualification evidence remains retained by this readback"]
    pub const fn evidence(&self) -> &crate::M1QualificationCompletionEvidenceV1 {
        &self.evidence
    }

    /// Exact checked terminal compact records before completion fan-out.
    #[must_use = "terminal checked records remain tied to this readback"]
    pub const fn checked(&self) -> &crate::M1CheckedCompletionOutputV1 {
        self.readback.readback.checked()
    }

    /// Selected requests that must enter Engine retirement before completion.
    #[must_use]
    pub fn selected_requests(&self) -> impl ExactSizeIterator<Item = RequestId> + '_ {
        self.readback
            .carry
            .selected
            .iter()
            .map(|cache| cache.projection().request)
    }

    /// Exact predecessor completion epoch.
    #[must_use]
    pub const fn previous_epoch(&self) -> CompletionEpoch {
        self.readback.carry.previous_epoch
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.readback.round_history_len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.readback.round_history(index)
    }

    /// Quarantines the Engine and destroys the completed-readback queue while
    /// retaining exact terminal evidence, checked output, KV reservations, and
    /// round lineage.
    ///
    /// # Errors
    ///
    /// Returns terminal lower readback-queue release quarantine joined to all
    /// of the same terminal custody.
    pub fn destroy_queue_and_retain_custody<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1RearmedQualifiedReadbackTeardownSuccessV1,
        Box<M1RearmedQualifiedReadbackTeardownFailureV1>,
    > {
        quarantine_readback_teardown(engine);
        let Self { readback, evidence } = self;
        let M1RearmedCompletedReadbackV1 {
            readback,
            carry,
            queue_observation,
            device,
        } = readback;
        let (queue, checked, completion, kv) = readback.into_parts();
        let custody = M1RearmedQualifiedReadbackTeardownCustodyV1 {
            checked,
            completion,
            kv,
            evidence,
            carry,
            queue_observation,
            device,
        };
        match queue.destroy_and_release() {
            Ok(queue_release) => Ok(M1RearmedQualifiedReadbackTeardownSuccessV1 {
                queue_release,
                custody,
            }),
            Err(source) => Err(Box::new(M1RearmedQualifiedReadbackTeardownFailureV1 {
                source,
                custody,
            })),
        }
    }

    /// Completes every selected member with the terminal retiring disposition.
    ///
    /// This local wrapper never permits a qualification-bearing member to
    /// continue into another queue generation. Engine retirement remains an
    /// explicit prior transition so partial scheduler failure is visible to the
    /// caller before this owner is consumed.
    ///
    /// # Errors
    ///
    /// Returns retryable local custody if the terminal disposition roster or
    /// the existing completion preflight cannot reserve bounded host storage.
    pub fn complete_retiring<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1RearmedQualifiedCompletionOutcomeV1,
        Box<M1RearmedQualifiedCompletionPreflightFailureV1>,
    > {
        if let Err(error) = preflight_retiring_requests(
            engine,
            self.readback
                .carry
                .selected
                .iter()
                .map(|cache| cache.projection().request),
        ) {
            return Err(Box::new(M1RearmedQualifiedCompletionPreflightFailureV1 {
                error,
                custody: M1RearmedQualifiedCompletionPreflightCustodyV1::Readback(Box::new(self)),
            }));
        }
        let selected = self.readback.carry.selected.len();
        let dispositions = match retiring_dispositions(selected) {
            Ok(dispositions) => dispositions,
            Err(()) => {
                return Err(Box::new(M1RearmedQualifiedCompletionPreflightFailureV1 {
                    error: M1RearmedCompletionPreflightErrorV1::HostAllocation,
                    custody: M1RearmedQualifiedCompletionPreflightCustodyV1::Readback(Box::new(
                        self,
                    )),
                }))
            }
        };
        let Self { readback, evidence } = self;
        match readback.complete(engine, dispositions) {
            Ok(completion) => Ok(M1RearmedQualifiedCompletionOutcomeV1 {
                completion,
                evidence,
            }),
            Err(source) => Err(Box::new(M1RearmedQualifiedCompletionPreflightFailureV1 {
                error: source.error(),
                custody: M1RearmedQualifiedCompletionPreflightCustodyV1::Lower { source, evidence },
            })),
        }
    }
}

#[derive(Debug)]
struct M1RearmedQualifiedReadbackTeardownCustodyV1 {
    checked: crate::M1CheckedCompletionOutputV1,
    completion: crate::ExactCompletion,
    kv: crate::M1FullStepKvReservationCustodyV1,
    evidence: crate::M1QualificationCompletionEvidenceV1,
    carry: M1RearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

/// Clean queue teardown retaining a successful terminal qualification join.
#[must_use = "qualified readback evidence and lineage remain retained"]
#[derive(Debug)]
pub struct M1RearmedQualifiedReadbackTeardownSuccessV1 {
    queue_release: ServiceQueueReleaseObservationV1,
    custody: M1RearmedQualifiedReadbackTeardownCustodyV1,
}

/// Terminal lower release quarantine retaining a qualified readback.
#[must_use = "qualified readback release quarantine remains retained"]
#[derive(Debug)]
pub struct M1RearmedQualifiedReadbackTeardownFailureV1 {
    source: crate::M1PhysicalReadbackQueueReleaseFailureV1,
    custody: M1RearmedQualifiedReadbackTeardownCustodyV1,
}

impl M1RearmedQualifiedReadbackTeardownSuccessV1 {
    pub const fn checked(&self) -> &crate::M1CheckedCompletionOutputV1 {
        &self.custody.checked
    }

    #[must_use]
    pub const fn completion_epoch(&self) -> CompletionEpoch {
        self.custody.completion.epoch()
    }

    pub const fn kv_reservations(&self) -> &crate::M1FullStepKvReservationCustodyV1 {
        &self.custody.kv
    }

    #[must_use = "qualification evidence remains retained"]
    pub const fn evidence(&self) -> &crate::M1QualificationCompletionEvidenceV1 {
        &self.custody.evidence
    }

    #[must_use]
    pub fn selected_requests(&self) -> impl ExactSizeIterator<Item = RequestId> + '_ {
        self.custody
            .carry
            .selected
            .iter()
            .map(|cache| cache.projection().request)
    }

    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.custody.carry.parked.len()
    }

    #[must_use]
    pub const fn terminal_count(&self) -> usize {
        self.custody.carry.terminal.len()
    }

    #[must_use]
    pub const fn previous_epoch(&self) -> CompletionEpoch {
        self.custody.carry.previous_epoch
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.custody.carry.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.custody.carry.history.get(index)
    }

    #[must_use]
    pub const fn queue_observation(&self) -> ComputeAqlQueueObservationV1 {
        self.custody.queue_observation
    }

    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.custody.device
    }

    #[must_use]
    pub const fn queue_release(&self) -> ServiceQueueReleaseObservationV1 {
        self.queue_release
    }
}

impl M1RearmedQualifiedReadbackTeardownFailureV1 {
    pub const fn checked(&self) -> &crate::M1CheckedCompletionOutputV1 {
        &self.custody.checked
    }

    #[must_use]
    pub const fn completion_epoch(&self) -> CompletionEpoch {
        self.custody.completion.epoch()
    }

    pub const fn kv_reservations(&self) -> &crate::M1FullStepKvReservationCustodyV1 {
        &self.custody.kv
    }

    #[must_use = "qualification evidence remains retained"]
    pub const fn evidence(&self) -> &crate::M1QualificationCompletionEvidenceV1 {
        &self.custody.evidence
    }

    #[must_use]
    pub fn selected_requests(&self) -> impl ExactSizeIterator<Item = RequestId> + '_ {
        self.custody
            .carry
            .selected
            .iter()
            .map(|cache| cache.projection().request)
    }

    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.custody.carry.parked.len()
    }

    #[must_use]
    pub const fn terminal_count(&self) -> usize {
        self.custody.carry.terminal.len()
    }

    #[must_use]
    pub const fn previous_epoch(&self) -> CompletionEpoch {
        self.custody.carry.previous_epoch
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.custody.carry.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.custody.carry.history.get(index)
    }

    #[must_use]
    pub const fn queue_observation(&self) -> ComputeAqlQueueObservationV1 {
        self.custody.queue_observation
    }

    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.custody.device
    }

    pub const fn source(&self) -> &crate::M1PhysicalReadbackQueueReleaseFailureV1 {
        &self.source
    }
}

fn preflight_retiring_requests<const C: usize>(
    engine: &Engine<C>,
    requests: impl Iterator<Item = RequestId>,
) -> Result<(), M1RearmedCompletionPreflightErrorV1> {
    for (lane, request) in requests.enumerate() {
        if engine.state(request) != Some(RequestState::Retiring) {
            return Err(M1RearmedCompletionPreflightErrorV1::SelectedNotRetiring { lane });
        }
    }
    Ok(())
}

fn retiring_dispositions(
    count: usize,
) -> Result<Vec<crate::M1DeviceKvCompletionDispositionV1>, ()> {
    let mut dispositions = Vec::new();
    dispositions.try_reserve_exact(count).map_err(|_| ())?;
    dispositions.resize(count, crate::M1DeviceKvCompletionDispositionV1::Retire);
    Ok(dispositions)
}

#[derive(Debug)]
enum M1RearmedQualifiedCompletionPreflightCustodyV1 {
    Readback(Box<M1RearmedQualifiedCompletedReadbackV1>),
    Lower {
        source: M1RearmedCompletionPreflightFailureV1,
        evidence: crate::M1QualificationCompletionEvidenceV1,
    },
}

/// Retry-safe local failure before terminal qualification completion fan-out.
#[must_use = "qualification completion failure retains readback and evidence"]
#[derive(Debug)]
pub struct M1RearmedQualifiedCompletionPreflightFailureV1 {
    error: M1RearmedCompletionPreflightErrorV1,
    custody: M1RearmedQualifiedCompletionPreflightCustodyV1,
}

impl M1RearmedQualifiedCompletionPreflightFailureV1 {
    /// Exact bounded local preflight rejection.
    #[must_use]
    pub const fn error(&self) -> M1RearmedCompletionPreflightErrorV1 {
        self.error
    }

    /// Number of active selected and parked cache owners retained by failure.
    #[must_use]
    pub const fn retained_cache_count(&self) -> usize {
        match &self.custody {
            M1RearmedQualifiedCompletionPreflightCustodyV1::Readback(readback) => {
                readback.readback.carry.selected.len() + readback.readback.carry.parked.len()
            }
            M1RearmedQualifiedCompletionPreflightCustodyV1::Lower { source, .. } => {
                source.retained_cache_count()
            }
        }
    }

    /// Copied final qualification evidence retained through local failure.
    #[must_use = "qualification evidence remains retained for retry"]
    pub const fn evidence(&self) -> &crate::M1QualificationCompletionEvidenceV1 {
        match &self.custody {
            M1RearmedQualifiedCompletionPreflightCustodyV1::Readback(readback) => {
                &readback.evidence
            }
            M1RearmedQualifiedCompletionPreflightCustodyV1::Lower { evidence, .. } => evidence,
        }
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        match &self.custody {
            M1RearmedQualifiedCompletionPreflightCustodyV1::Readback(readback) => {
                readback.round_history_len()
            }
            M1RearmedQualifiedCompletionPreflightCustodyV1::Lower { source, .. } => {
                source.round_history_len()
            }
        }
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        match &self.custody {
            M1RearmedQualifiedCompletionPreflightCustodyV1::Readback(readback) => {
                readback.round_history(index)
            }
            M1RearmedQualifiedCompletionPreflightCustodyV1::Lower { source, .. } => {
                source.round_history(index)
            }
        }
    }

    /// Retries the unchanged terminal disposition preflight.
    ///
    /// # Errors
    ///
    /// Returns renewed retained custody if bounded host allocation still fails.
    pub fn retry<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<M1RearmedQualifiedCompletionOutcomeV1, Box<Self>> {
        match self.custody {
            M1RearmedQualifiedCompletionPreflightCustodyV1::Readback(readback) => {
                readback.complete_retiring(engine)
            }
            M1RearmedQualifiedCompletionPreflightCustodyV1::Lower { source, evidence } => {
                match source.retry(engine) {
                    Ok(completion) => Ok(M1RearmedQualifiedCompletionOutcomeV1 {
                        completion,
                        evidence,
                    }),
                    Err(source) => Err(Box::new(Self {
                        error: source.error(),
                        custody: M1RearmedQualifiedCompletionPreflightCustodyV1::Lower {
                            source,
                            evidence,
                        },
                    })),
                }
            }
        }
    }

    /// Quarantines the Engine and destroys the physical queue while retaining
    /// final qualification evidence and the exact completion-preflight owner.
    ///
    /// # Errors
    ///
    /// Returns terminal lower queue-release quarantine joined to the same
    /// final evidence and round lineage.
    pub fn destroy_queue_and_retain_custody<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1RearmedQualifiedCompletionPreflightTeardownSuccessV1,
        Box<M1RearmedQualifiedCompletionPreflightTeardownFailureV1>,
    > {
        let error = self.error;
        let (source, evidence) = match self.custody {
            M1RearmedQualifiedCompletionPreflightCustodyV1::Readback(readback) => {
                let M1RearmedQualifiedCompletedReadbackV1 { readback, evidence } = *readback;
                (
                    teardown_rearmed_completion_preflight(engine, error, readback, Vec::new()),
                    evidence,
                )
            }
            M1RearmedQualifiedCompletionPreflightCustodyV1::Lower { source, evidence } => {
                (source.destroy_queue_and_retain_custody(engine), evidence)
            }
        };
        match source {
            Ok(source) => {
                Ok(M1RearmedQualifiedCompletionPreflightTeardownSuccessV1 { source, evidence })
            }
            Err(source) => Err(Box::new(
                M1RearmedQualifiedCompletionPreflightTeardownFailureV1 { source, evidence },
            )),
        }
    }
}

/// Clean completion-preflight teardown retaining final qualification evidence.
#[must_use = "completion preflight teardown and final evidence remain retained"]
#[derive(Debug)]
pub struct M1RearmedQualifiedCompletionPreflightTeardownSuccessV1 {
    source: M1RearmedCompletionPreflightTeardownSuccessV1,
    evidence: crate::M1QualificationCompletionEvidenceV1,
}

impl M1RearmedQualifiedCompletionPreflightTeardownSuccessV1 {
    pub const fn source(&self) -> &M1RearmedCompletionPreflightTeardownSuccessV1 {
        &self.source
    }

    #[must_use = "final qualification evidence remains retained"]
    pub const fn evidence(&self) -> &crate::M1QualificationCompletionEvidenceV1 {
        &self.evidence
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.source.round_history_len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.source.round_history(index)
    }
}

/// Terminal completion-preflight release quarantine retaining final evidence.
#[must_use = "completion preflight quarantine and final evidence remain retained"]
#[derive(Debug)]
pub struct M1RearmedQualifiedCompletionPreflightTeardownFailureV1 {
    source: Box<M1RearmedCompletionPreflightTeardownFailureV1>,
    evidence: crate::M1QualificationCompletionEvidenceV1,
}

impl M1RearmedQualifiedCompletionPreflightTeardownFailureV1 {
    pub const fn source(&self) -> &M1RearmedCompletionPreflightTeardownFailureV1 {
        &self.source
    }

    #[must_use = "final qualification evidence remains retained"]
    pub const fn evidence(&self) -> &crate::M1QualificationCompletionEvidenceV1 {
        &self.evidence
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.source.round_history_len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.source.round_history(index)
    }
}

/// Terminal physical completion outcome retaining final qualification evidence.
#[must_use = "completion outcome and qualification evidence must remain retained"]
#[derive(Debug)]
pub struct M1RearmedQualifiedCompletionOutcomeV1 {
    completion: M1RearmedCompletionOutcomeV1,
    evidence: crate::M1QualificationCompletionEvidenceV1,
}

impl M1RearmedQualifiedCompletionOutcomeV1 {
    /// Existing completion, rejection, or poison outcome with all KV custody.
    #[must_use = "physical completion outcome remains retained"]
    pub const fn completion(&self) -> &M1RearmedCompletionOutcomeV1 {
        &self.completion
    }

    /// Exact copied compact and final-logits evidence.
    #[must_use = "qualification evidence remains retained"]
    pub const fn evidence(&self) -> &crate::M1QualificationCompletionEvidenceV1 {
        &self.evidence
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.completion.round_history_len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.completion.round_history(index)
    }

    /// Retries only an unchanged physical completion preflight rejection while
    /// retaining the final qualification evidence across every outcome.
    ///
    /// # Errors
    ///
    /// Returns the unchanged qualified owner when the physical outcome is
    /// already completed or terminally poisoned.
    pub fn retry_rejected<const C: usize>(self, engine: &mut Engine<C>) -> Result<Self, Box<Self>> {
        let Self {
            completion,
            evidence,
        } = self;
        match completion.retry_rejected(engine) {
            Ok(completion) => Ok(Self {
                completion,
                evidence,
            }),
            Err(completion) => Err(Box::new(Self {
                completion: *completion,
                evidence,
            })),
        }
    }

    /// Destroys a retry-exhausted rejected completion while retaining final
    /// qualification evidence and every round owner.
    ///
    /// # Errors
    ///
    /// Returns this qualified owner unchanged when its physical outcome is not
    /// retryably rejected.
    pub fn destroy_queue_and_retain_rejected<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        Result<
            M1RearmedQualifiedRejectedCompletionTeardownSuccessV1,
            Box<M1RearmedQualifiedRejectedCompletionTeardownFailureV1>,
        >,
        Box<Self>,
    > {
        let Self {
            completion,
            evidence,
        } = self;
        match completion.destroy_queue_and_retain_rejected(engine) {
            Ok(result) => Ok(match join_qualification_evidence(result, evidence) {
                Ok(joined) => Ok(M1RearmedQualifiedRejectedCompletionTeardownSuccessV1 {
                    source: joined.source,
                    evidence: joined.evidence,
                }),
                Err(joined) => Err(Box::new(
                    M1RearmedQualifiedRejectedCompletionTeardownFailureV1 {
                        source: joined.source,
                        evidence: joined.evidence,
                    },
                )),
            }),
            Err(completion) => Err(Box::new(Self {
                completion: *completion,
                evidence,
            })),
        }
    }

    /// Extracts terminal physical completion poison joined to final
    /// qualification evidence and round lineage.
    ///
    /// # Errors
    ///
    /// Returns this qualified owner unchanged unless the physical completion
    /// is poisoned.
    pub fn into_terminal_poison(self) -> Result<M1RearmedQualifiedPoisonedCompletionV1, Box<Self>> {
        let Self {
            completion,
            evidence,
        } = self;
        match completion.into_terminal_poison() {
            Ok(completion) => Ok(M1RearmedQualifiedPoisonedCompletionV1 {
                completion,
                evidence,
            }),
            Err(completion) => Err(Box::new(Self {
                completion: *completion,
                evidence,
            })),
        }
    }

    /// Releases retired pages from a successful terminal completion while
    /// retaining qualification evidence beside every exhaustive outcome.
    #[must_use = "release outcome retains completion and qualification custody"]
    pub fn release_completed(self) -> M1RearmedQualifiedRoundReleaseOutcomeV1 {
        let Self {
            completion,
            evidence,
        } = self;
        join_qualified_round_release(completion.release_completed(), evidence)
    }

    /// Separates the completion outcome and inert qualification evidence once.
    #[must_use = "both physical outcome and qualification evidence remain retained"]
    pub fn into_parts(
        self,
    ) -> (
        M1RearmedCompletionOutcomeV1,
        crate::M1QualificationCompletionEvidenceV1,
    ) {
        (self.completion, self.evidence)
    }
}

/// Terminal completion poison retaining final qualification evidence.
#[must_use = "terminal poison and qualification evidence remain retained"]
#[derive(Debug)]
pub struct M1RearmedQualifiedPoisonedCompletionV1 {
    completion: M1RearmedPoisonedCompletionV1,
    evidence: crate::M1QualificationCompletionEvidenceV1,
}

impl M1RearmedQualifiedPoisonedCompletionV1 {
    pub const fn completion(&self) -> &M1RearmedPoisonedCompletionV1 {
        &self.completion
    }

    #[must_use = "qualification evidence remains retained"]
    pub const fn evidence(&self) -> &crate::M1QualificationCompletionEvidenceV1 {
        &self.evidence
    }
}

/// Clean teardown retaining a rejected terminal completion and final evidence.
#[must_use = "terminal rejection and qualification evidence remain retained"]
#[derive(Debug)]
pub struct M1RearmedQualifiedRejectedCompletionTeardownSuccessV1 {
    source: M1RearmedRejectedCompletionTeardownSuccessV1,
    evidence: crate::M1QualificationCompletionEvidenceV1,
}

impl M1RearmedQualifiedRejectedCompletionTeardownSuccessV1 {
    pub const fn source(&self) -> &M1RearmedRejectedCompletionTeardownSuccessV1 {
        &self.source
    }

    #[must_use = "qualification evidence remains retained"]
    pub const fn evidence(&self) -> &crate::M1QualificationCompletionEvidenceV1 {
        &self.evidence
    }
}

/// Terminal release quarantine retaining a rejected completion and evidence.
#[must_use = "terminal rejection release quarantine remains retained"]
#[derive(Debug)]
pub struct M1RearmedQualifiedRejectedCompletionTeardownFailureV1 {
    source: Box<M1RearmedRejectedCompletionTeardownFailureV1>,
    evidence: crate::M1QualificationCompletionEvidenceV1,
}

impl M1RearmedQualifiedRejectedCompletionTeardownFailureV1 {
    pub const fn source(&self) -> &M1RearmedRejectedCompletionTeardownFailureV1 {
        &self.source
    }

    #[must_use = "qualification evidence remains retained"]
    pub const fn evidence(&self) -> &crate::M1QualificationCompletionEvidenceV1 {
        &self.evidence
    }
}

fn join_qualified_round_release(
    release: M1RearmedRoundReleaseOutcomeV1,
    evidence: crate::M1QualificationCompletionEvidenceV1,
) -> M1RearmedQualifiedRoundReleaseOutcomeV1 {
    match release {
        M1RearmedRoundReleaseOutcomeV1::Released(released) => {
            M1RearmedQualifiedRoundReleaseOutcomeV1::Released(M1RearmedQualifiedReleasedRoundV1 {
                released,
                evidence,
            })
        }
        M1RearmedRoundReleaseOutcomeV1::Rejected(source) => {
            M1RearmedQualifiedRoundReleaseOutcomeV1::Rejected(Box::new(
                M1RearmedQualifiedRoundPageReleaseFailureV1 { source, evidence },
            ))
        }
        M1RearmedRoundReleaseOutcomeV1::NotCompleted(completion) => {
            M1RearmedQualifiedRoundReleaseOutcomeV1::NotCompleted(
                M1RearmedQualifiedCompletionOutcomeV1 {
                    completion,
                    evidence,
                },
            )
        }
    }
}

/// Exhaustive terminal page-release transition retaining qualification evidence.
#[must_use = "qualification release outcome retains every linear owner"]
#[derive(Debug)]
pub enum M1RearmedQualifiedRoundReleaseOutcomeV1 {
    Released(M1RearmedQualifiedReleasedRoundV1),
    Rejected(Box<M1RearmedQualifiedRoundPageReleaseFailureV1>),
    NotCompleted(M1RearmedQualifiedCompletionOutcomeV1),
}

/// Retryable terminal page-release rejection retaining qualification evidence.
#[must_use = "page-release failure remains the sole qualified retry owner"]
#[derive(Debug)]
pub struct M1RearmedQualifiedRoundPageReleaseFailureV1 {
    source: Box<M1RearmedRoundPageReleaseFailureV1>,
    evidence: crate::M1QualificationCompletionEvidenceV1,
}

impl M1RearmedQualifiedRoundPageReleaseFailureV1 {
    /// Existing page-release diagnostic and current/parked KV custody.
    #[must_use = "page-release failure diagnostic remains retained"]
    pub const fn source(&self) -> &crate::M1CompletedStepKvReleaseErrorV1 {
        self.source.source()
    }

    /// Copied final qualification evidence retained through rejection.
    #[must_use = "qualification evidence remains retained for release retry"]
    pub const fn evidence(&self) -> &crate::M1QualificationCompletionEvidenceV1 {
        &self.evidence
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.source.round_history_len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.source.round_history(index)
    }

    /// Retries exact page release and rejoins qualification evidence.
    #[must_use = "release retry retains every exhaustive outcome"]
    pub fn retry(self) -> M1RearmedQualifiedRoundReleaseOutcomeV1 {
        join_qualified_round_release(self.source.retry(), self.evidence)
    }

    /// Destroys the terminal physical queue after page release cannot make
    /// progress, retaining final evidence and every round owner.
    ///
    /// # Errors
    ///
    /// Returns terminal lower queue-release quarantine joined to the same
    /// final evidence and history.
    pub fn destroy_queue_and_retain_round<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1RearmedQualifiedRoundPageReleaseTeardownSuccessV1,
        Box<M1RearmedQualifiedRoundPageReleaseTeardownFailureV1>,
    > {
        match join_qualification_evidence(
            self.source.destroy_queue_and_retain_round(engine),
            self.evidence,
        ) {
            Ok(joined) => Ok(M1RearmedQualifiedRoundPageReleaseTeardownSuccessV1 {
                source: joined.source,
                evidence: joined.evidence,
            }),
            Err(joined) => Err(Box::new(
                M1RearmedQualifiedRoundPageReleaseTeardownFailureV1 {
                    source: joined.source,
                    evidence: joined.evidence,
                },
            )),
        }
    }
}

/// Clean page-release-exhaustion teardown retaining final qualification
/// evidence.
#[must_use = "page-release teardown and final qualification evidence remain retained"]
#[derive(Debug)]
pub struct M1RearmedQualifiedRoundPageReleaseTeardownSuccessV1 {
    source: M1RearmedRoundPageReleaseTeardownSuccessV1,
    evidence: crate::M1QualificationCompletionEvidenceV1,
}

impl M1RearmedQualifiedRoundPageReleaseTeardownSuccessV1 {
    pub const fn source(&self) -> &M1RearmedRoundPageReleaseTeardownSuccessV1 {
        &self.source
    }

    #[must_use = "final qualification evidence remains retained"]
    pub const fn evidence(&self) -> &crate::M1QualificationCompletionEvidenceV1 {
        &self.evidence
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.source.round_history_len()
    }
}

/// Terminal page-release-exhaustion quarantine retaining final qualification
/// evidence.
#[must_use = "page-release quarantine and final qualification evidence remain retained"]
#[derive(Debug)]
pub struct M1RearmedQualifiedRoundPageReleaseTeardownFailureV1 {
    source: Box<M1RearmedRoundPageReleaseTeardownFailureV1>,
    evidence: crate::M1QualificationCompletionEvidenceV1,
}

impl M1RearmedQualifiedRoundPageReleaseTeardownFailureV1 {
    pub const fn source(&self) -> &M1RearmedRoundPageReleaseTeardownFailureV1 {
        &self.source
    }

    #[must_use = "final qualification evidence remains retained"]
    pub const fn evidence(&self) -> &crate::M1QualificationCompletionEvidenceV1 {
        &self.evidence
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.source.round_history_len()
    }
}

/// Released terminal round retaining final qualification evidence.
#[must_use = "released queue and qualification evidence must be torn down or retained"]
#[derive(Debug)]
pub struct M1RearmedQualifiedReleasedRoundV1 {
    released: M1LongLivedQueueReleasedRoundV1,
    evidence: crate::M1QualificationCompletionEvidenceV1,
}

impl M1RearmedQualifiedReleasedRoundV1 {
    /// Active cache count parked outside the terminal selected roster.
    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.released.parked_count()
    }

    /// Copied final qualification evidence retained after page release.
    #[must_use = "qualification evidence remains retained through teardown"]
    pub const fn evidence(&self) -> &crate::M1QualificationCompletionEvidenceV1 {
        &self.evidence
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.released.round_history_len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.released.round_history(index)
    }

    /// Destroys the completed native queue and retains the terminal round,
    /// parked cache lineage, prior history, and qualification evidence.
    ///
    /// ```compile_fail
    /// use ferric_engine::{Engine, M1RearmedQualifiedReleasedRoundV1};
    /// fn teardown_twice(released: M1RearmedQualifiedReleasedRoundV1, engine: &mut Engine<32>) {
    ///     let _first = released.destroy_queue_and_retain_round(engine);
    ///     let _second = released.destroy_queue_and_retain_round(engine);
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns terminal queue-release quarantine joined to the same final
    /// qualification evidence.
    pub fn destroy_queue_and_retain_round<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<M1RearmedQualifiedTeardownSuccessV1, Box<M1RearmedQualifiedTeardownFailureV1>> {
        let Self { released, evidence } = self;
        match released.destroy_queue_and_retain_round(engine) {
            Ok(teardown) => Ok(M1RearmedQualifiedTeardownSuccessV1 { teardown, evidence }),
            Err(teardown) => Err(Box::new(M1RearmedQualifiedTeardownFailureV1 {
                teardown,
                evidence,
            })),
        }
    }
}

/// Clean terminal queue teardown retaining qualification and round custody.
#[must_use = "terminal release and qualification evidence remain retained"]
#[derive(Debug)]
pub struct M1RearmedQualifiedTeardownSuccessV1 {
    teardown: M1LongLivedQueueRearmTeardownSuccessV1,
    evidence: crate::M1QualificationCompletionEvidenceV1,
}

impl M1RearmedQualifiedTeardownSuccessV1 {
    /// Exact queue release and current/parked terminal custody.
    pub const fn teardown(&self) -> &M1LongLivedQueueRearmTeardownSuccessV1 {
        &self.teardown
    }

    /// Final qualification evidence retained through clean teardown.
    #[must_use = "qualification evidence remains retained"]
    pub const fn evidence(&self) -> &crate::M1QualificationCompletionEvidenceV1 {
        &self.evidence
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.teardown.round_history_len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.teardown.round_history(index)
    }
}

/// Terminal queue-release failure retaining qualification and round custody.
#[must_use = "terminal quarantine and qualification evidence remain retained"]
#[derive(Debug)]
pub struct M1RearmedQualifiedTeardownFailureV1 {
    teardown: Box<M1LongLivedQueueRearmTeardownFailureV1>,
    evidence: crate::M1QualificationCompletionEvidenceV1,
}

impl M1RearmedQualifiedTeardownFailureV1 {
    /// Exact terminal queue-release quarantine and retained KV lineage.
    pub const fn teardown(&self) -> &M1LongLivedQueueRearmTeardownFailureV1 {
        &self.teardown
    }

    /// Final qualification evidence retained beside quarantine.
    #[must_use = "qualification evidence remains retained"]
    pub const fn evidence(&self) -> &crate::M1QualificationCompletionEvidenceV1 {
        &self.evidence
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.teardown.round_history_len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.teardown.round_history(index)
    }
}

/// Pure local rejection before selected caches enter the completion fan-out.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1RearmedCompletionPreflightErrorV1 {
    DispositionCount { expected: usize, actual: usize },
    SelectedNotRetiring { lane: usize },
    RoundHistoryCapacity { maximum: usize },
    HostAllocation,
}

/// Completion preflight rejection retaining readback, caches, and dispositions.
#[must_use = "completion rejection retains every linear input"]
#[derive(Debug)]
pub struct M1RearmedCompletionPreflightFailureV1 {
    error: M1RearmedCompletionPreflightErrorV1,
    readback: Box<M1RearmedCompletedReadbackV1>,
    dispositions: Vec<crate::M1DeviceKvCompletionDispositionV1>,
}

impl M1RearmedCompletionPreflightFailureV1 {
    #[must_use]
    pub const fn error(&self) -> M1RearmedCompletionPreflightErrorV1 {
        self.error
    }

    /// Semantically checked compact completion retained by this rejection.
    pub const fn checked(&self) -> &crate::M1CheckedCompletionOutputV1 {
        self.readback.checked()
    }

    #[must_use]
    pub const fn retained_cache_count(&self) -> usize {
        self.readback.carry.selected.len() + self.readback.carry.parked.len()
    }

    #[must_use]
    pub fn disposition_count(&self) -> usize {
        self.dispositions.len()
    }

    /// Borrows the exact dispositions retained for a local retry.
    #[must_use]
    pub fn dispositions(&self) -> &[crate::M1DeviceKvCompletionDispositionV1] {
        &self.dispositions
    }

    /// Recovers the unchanged readback and requested dispositions after a pure
    /// local preflight rejection.
    #[must_use = "readback and dispositions remain the sole retry inputs"]
    pub fn into_parts(
        self,
    ) -> (
        M1RearmedCompletionPreflightErrorV1,
        M1RearmedCompletedReadbackV1,
        Vec<crate::M1DeviceKvCompletionDispositionV1>,
    ) {
        (self.error, *self.readback, self.dispositions)
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.readback.round_history_len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.readback.round_history(index)
    }

    /// Retries the unchanged local completion preflight.
    ///
    /// This failure occurs before selected caches are moved into the existing
    /// completion fan-out, so retry does not fault or otherwise mutate Engine.
    ///
    /// # Errors
    ///
    /// Returns another unchanged local preflight failure if the supplied
    /// disposition count remains invalid or host reservation still fails.
    pub fn retry<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<M1RearmedCompletionOutcomeV1, Self> {
        (*self.readback).complete(engine, self.dispositions)
    }

    /// Quarantines the Engine and destroys the physical queue while retaining
    /// the rejected dispositions, completed readback, and complete round
    /// lineage.
    ///
    /// # Errors
    ///
    /// Returns terminal lower queue-release quarantine joined to the same
    /// completion-preflight custody.
    pub fn destroy_queue_and_retain_custody<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1RearmedCompletionPreflightTeardownSuccessV1,
        Box<M1RearmedCompletionPreflightTeardownFailureV1>,
    > {
        teardown_rearmed_completion_preflight(engine, self.error, *self.readback, self.dispositions)
    }
}

#[derive(Debug)]
struct M1RearmedCompletionPreflightTeardownCustodyV1 {
    error: M1RearmedCompletionPreflightErrorV1,
    checked: crate::M1CheckedCompletionOutputV1,
    completion: crate::ExactCompletion,
    kv: crate::M1FullStepKvReservationCustodyV1,
    dispositions: Vec<crate::M1DeviceKvCompletionDispositionV1>,
    carry: M1RearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

/// Clean queue teardown after completion preflight could not make progress.
#[must_use = "preflight diagnostic, readback, and round lineage remain retained"]
#[derive(Debug)]
pub struct M1RearmedCompletionPreflightTeardownSuccessV1 {
    queue_release: ServiceQueueReleaseObservationV1,
    custody: M1RearmedCompletionPreflightTeardownCustodyV1,
}

/// Terminal queue-release quarantine after completion preflight rejection.
#[must_use = "release quarantine, readback, and round lineage remain retained"]
#[derive(Debug)]
pub struct M1RearmedCompletionPreflightTeardownFailureV1 {
    source: crate::M1PhysicalReadbackQueueReleaseFailureV1,
    custody: M1RearmedCompletionPreflightTeardownCustodyV1,
}

impl M1RearmedCompletionPreflightTeardownSuccessV1 {
    #[must_use]
    pub const fn error(&self) -> M1RearmedCompletionPreflightErrorV1 {
        self.custody.error
    }
    pub const fn checked(&self) -> &crate::M1CheckedCompletionOutputV1 {
        &self.custody.checked
    }
    #[must_use]
    pub const fn completion_epoch(&self) -> CompletionEpoch {
        self.custody.completion.epoch()
    }
    pub const fn kv_reservations(&self) -> &crate::M1FullStepKvReservationCustodyV1 {
        &self.custody.kv
    }
    #[must_use]
    pub fn disposition_count(&self) -> usize {
        self.custody.dispositions.len()
    }
    #[must_use]
    pub fn retained_cache_count(&self) -> usize {
        self.custody.carry.selected.len() + self.custody.carry.parked.len()
    }
    #[must_use]
    pub fn selected_requests(&self) -> impl ExactSizeIterator<Item = RequestId> + '_ {
        self.custody
            .carry
            .selected
            .iter()
            .map(|cache| cache.projection().request)
    }
    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.custody.carry.parked.len()
    }
    #[must_use]
    pub const fn terminal_count(&self) -> usize {
        self.custody.carry.terminal.len()
    }
    #[must_use]
    pub const fn previous_epoch(&self) -> CompletionEpoch {
        self.custody.carry.previous_epoch
    }
    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.custody.carry.history.len()
    }
    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.custody.carry.history.get(index)
    }
    #[must_use]
    pub const fn queue_observation(&self) -> ComputeAqlQueueObservationV1 {
        self.custody.queue_observation
    }
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.custody.device
    }

    #[must_use]
    pub const fn queue_release(&self) -> ServiceQueueReleaseObservationV1 {
        self.queue_release
    }
}

impl M1RearmedCompletionPreflightTeardownFailureV1 {
    #[must_use]
    pub const fn error(&self) -> M1RearmedCompletionPreflightErrorV1 {
        self.custody.error
    }
    pub const fn checked(&self) -> &crate::M1CheckedCompletionOutputV1 {
        &self.custody.checked
    }
    #[must_use]
    pub const fn completion_epoch(&self) -> CompletionEpoch {
        self.custody.completion.epoch()
    }
    pub const fn kv_reservations(&self) -> &crate::M1FullStepKvReservationCustodyV1 {
        &self.custody.kv
    }
    #[must_use]
    pub fn disposition_count(&self) -> usize {
        self.custody.dispositions.len()
    }
    #[must_use]
    pub fn retained_cache_count(&self) -> usize {
        self.custody.carry.selected.len() + self.custody.carry.parked.len()
    }
    #[must_use]
    pub fn selected_requests(&self) -> impl ExactSizeIterator<Item = RequestId> + '_ {
        self.custody
            .carry
            .selected
            .iter()
            .map(|cache| cache.projection().request)
    }
    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.custody.carry.parked.len()
    }
    #[must_use]
    pub const fn terminal_count(&self) -> usize {
        self.custody.carry.terminal.len()
    }
    #[must_use]
    pub const fn previous_epoch(&self) -> CompletionEpoch {
        self.custody.carry.previous_epoch
    }
    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.custody.carry.history.len()
    }
    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.custody.carry.history.get(index)
    }
    #[must_use]
    pub const fn queue_observation(&self) -> ComputeAqlQueueObservationV1 {
        self.custody.queue_observation
    }
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.custody.device
    }

    pub const fn source(&self) -> &crate::M1PhysicalReadbackQueueReleaseFailureV1 {
        &self.source
    }
}

fn teardown_rearmed_completion_preflight<const C: usize>(
    engine: &mut Engine<C>,
    error: M1RearmedCompletionPreflightErrorV1,
    readback: M1RearmedCompletedReadbackV1,
    dispositions: Vec<crate::M1DeviceKvCompletionDispositionV1>,
) -> Result<
    M1RearmedCompletionPreflightTeardownSuccessV1,
    Box<M1RearmedCompletionPreflightTeardownFailureV1>,
> {
    quarantine_readback_teardown(engine);
    let M1RearmedCompletedReadbackV1 {
        readback,
        carry,
        queue_observation,
        device,
    } = readback;
    let (queue, checked, completion, kv) = readback.into_parts();
    let custody = M1RearmedCompletionPreflightTeardownCustodyV1 {
        error,
        checked,
        completion,
        kv,
        dispositions,
        carry,
        queue_observation,
        device,
    };
    match queue.destroy_and_release() {
        Ok(queue_release) => Ok(M1RearmedCompletionPreflightTeardownSuccessV1 {
            queue_release,
            custody,
        }),
        Err(source) => Err(Box::new(M1RearmedCompletionPreflightTeardownFailureV1 {
            source,
            custody,
        })),
    }
}

/// Existing physical completion outcome plus custody parked across the round.
#[must_use = "completion outcome and parked rearm custody must remain retained"]
#[derive(Debug)]
pub struct M1RearmedCompletionOutcomeV1 {
    outcome: crate::M1CompletedStepOutcomeV1,
    parked: Vec<ActiveDeviceKvCache>,
    terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
    history: M1NonEmptyRearmRoundHistoryV1,
}

impl M1RearmedCompletionOutcomeV1 {
    pub const fn outcome(&self) -> &crate::M1CompletedStepOutcomeV1 {
        &self.outcome
    }

    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.parked.len()
    }

    #[must_use]
    pub const fn terminal_count(&self) -> usize {
        self.terminal.len()
    }

    #[must_use]
    pub const fn queue_observation(&self) -> ComputeAqlQueueObservationV1 {
        self.history.latest().queue_observation()
    }

    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.history.latest().device()
    }

    #[must_use]
    pub const fn prior_completed_members(&self) -> usize {
        self.history.latest().completed_members()
    }

    #[must_use]
    pub const fn prior_total_released(&self) -> usize {
        self.history.latest().total_released()
    }

    pub const fn prior_checked(&self) -> &crate::M1CheckedCompletionOutputV1 {
        self.history.latest().checked()
    }

    #[must_use]
    pub fn prior_logical_accepted_counts(&self) -> &[u32] {
        self.history.latest().logical_accepted_counts()
    }

    #[must_use]
    pub fn prior_externally_published_counts(&self) -> &[u32] {
        self.history.latest().externally_published_counts()
    }

    #[must_use]
    pub fn prior_release_counts(&self) -> &[M1CompletedKvPageReleaseCountsV1] {
        self.history.latest().release_counts()
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.history.get(index)
    }

    /// Retries only the unchanged preflight-rejected physical completion.
    ///
    /// A completed or poisoned outcome is returned unchanged in `Err`.
    ///
    /// # Errors
    ///
    /// Returns the unchanged non-rejected outcome when retry is not admitted.
    pub fn retry_rejected<const C: usize>(self, engine: &mut Engine<C>) -> Result<Self, Box<Self>> {
        let Self {
            outcome,
            parked,
            terminal,
            history,
        } = self;
        let crate::M1CompletedStepOutcomeV1::Rejected(rejected) = outcome else {
            return Err(Box::new(Self {
                outcome,
                parked,
                terminal,
                history,
            }));
        };
        let (_error, readback, roster) = rejected.into_parts();
        Ok(Self {
            outcome: crate::complete_m1_physical_step_v1(engine, readback, roster),
            parked,
            terminal,
            history,
        })
    }

    /// Quarantines the Engine and destroys an unchanged rejected completion,
    /// retaining its diagnostic, roster, parked/terminal caches, and history.
    ///
    /// # Errors
    ///
    /// Returns this owner unchanged when the physical outcome is completed or
    /// poisoned rather than retryably rejected.
    pub fn destroy_queue_and_retain_rejected<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        Result<
            M1RearmedRejectedCompletionTeardownSuccessV1,
            Box<M1RearmedRejectedCompletionTeardownFailureV1>,
        >,
        Box<Self>,
    > {
        let Self {
            outcome,
            parked,
            terminal,
            history,
        } = self;
        let crate::M1CompletedStepOutcomeV1::Rejected(rejected) = outcome else {
            return Err(Box::new(Self {
                outcome,
                parked,
                terminal,
                history,
            }));
        };
        let teardown = rejected.destroy_queue_and_retain_rejection(engine);
        Ok(
            match join_terminal_lineage(teardown, parked, terminal, history) {
                Ok(joined) => Ok(M1RearmedRejectedCompletionTeardownSuccessV1 {
                    source: joined.source,
                    parked: joined.parked,
                    terminal: joined.terminal,
                    history: joined.history,
                }),
                Err(joined) => Err(Box::new(M1RearmedRejectedCompletionTeardownFailureV1 {
                    source: joined.source,
                    parked: joined.parked,
                    terminal: joined.terminal,
                    history: joined.history,
                })),
            },
        )
    }

    /// Extracts an already terminal physical completion poison while retaining
    /// parked/terminal caches and complete history.
    ///
    /// # Errors
    ///
    /// Returns this owner unchanged unless its physical outcome is poisoned.
    pub fn into_terminal_poison(self) -> Result<M1RearmedPoisonedCompletionV1, Box<Self>> {
        let Self {
            outcome,
            parked,
            terminal,
            history,
        } = self;
        let crate::M1CompletedStepOutcomeV1::Poisoned(poison) = outcome else {
            return Err(Box::new(Self {
                outcome,
                parked,
                terminal,
                history,
            }));
        };
        Ok(M1RearmedPoisonedCompletionV1 {
            poison: *poison,
            parked,
            terminal,
            history,
        })
    }

    /// Releases exact retired pages only from a successful completion and
    /// returns a closed owner that can schedule another same-shape round.
    #[must_use = "release outcome retains every queue and cache owner"]
    pub fn release_completed(self) -> M1RearmedRoundReleaseOutcomeV1 {
        let Self {
            outcome,
            parked,
            terminal,
            history,
        } = self;
        let crate::M1CompletedStepOutcomeV1::Completed(completed) = outcome else {
            return M1RearmedRoundReleaseOutcomeV1::NotCompleted(Self {
                outcome,
                parked,
                terminal,
                history,
            });
        };
        release_rearmed_round(completed, history, parked, terminal)
    }
}

/// Terminal completion poison retaining every rearm lineage owner.
#[must_use = "completion poison and round lineage remain retained"]
#[derive(Debug)]
pub struct M1RearmedPoisonedCompletionV1 {
    poison: crate::M1CompletedStepPoisonV1,
    parked: Vec<ActiveDeviceKvCache>,
    terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
    history: M1NonEmptyRearmRoundHistoryV1,
}

impl M1RearmedPoisonedCompletionV1 {
    pub const fn poison(&self) -> &crate::M1CompletedStepPoisonV1 {
        &self.poison
    }

    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.parked.len()
    }

    #[must_use]
    pub const fn terminal_count(&self) -> usize {
        self.terminal.len()
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.history.get(index)
    }
}

/// Clean queue teardown retaining a rejected rearmed completion and lineage.
#[must_use = "rejected completion and round lineage remain retained"]
#[derive(Debug)]
pub struct M1RearmedRejectedCompletionTeardownSuccessV1 {
    source: crate::M1CompletedStepRejectionTeardownSuccessV1,
    parked: Vec<ActiveDeviceKvCache>,
    terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
    history: M1NonEmptyRearmRoundHistoryV1,
}

/// Terminal lower release quarantine retaining rejected completion lineage.
#[must_use = "rejected completion release quarantine remains retained"]
#[derive(Debug)]
pub struct M1RearmedRejectedCompletionTeardownFailureV1 {
    source: Box<crate::M1CompletedStepRejectionTeardownFailureV1>,
    parked: Vec<ActiveDeviceKvCache>,
    terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
    history: M1NonEmptyRearmRoundHistoryV1,
}

impl M1RearmedRejectedCompletionTeardownSuccessV1 {
    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.parked.len()
    }

    #[must_use]
    pub const fn terminal_count(&self) -> usize {
        self.terminal.len()
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.history.get(index)
    }

    pub const fn source(&self) -> &crate::M1CompletedStepRejectionTeardownSuccessV1 {
        &self.source
    }
}

impl M1RearmedRejectedCompletionTeardownFailureV1 {
    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.parked.len()
    }

    #[must_use]
    pub const fn terminal_count(&self) -> usize {
        self.terminal.len()
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.history.get(index)
    }

    pub const fn source(&self) -> &crate::M1CompletedStepRejectionTeardownFailureV1 {
        &self.source
    }
}

/// Released current round plus active caches parked outside that round.
///
/// The parked caches are deliberately separate from `released`: the current
/// checked/logical-accept/external-publication/release arrays name only current
/// selected lanes.
///
/// ```compile_fail
/// use ferric_engine::M1LongLivedQueueReleasedRoundV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1LongLivedQueueReleasedRoundV1>();
/// ```
#[must_use = "released round custody must schedule again or remain retained"]
#[derive(Debug)]
pub struct M1LongLivedQueueReleasedRoundV1 {
    released: M1ReleasedCompletedStepV1,
    parked: Vec<ActiveDeviceKvCache>,
    terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
    history: M1NonEmptyRearmRoundHistoryV1,
}

/// Confirmed healthy shutdown of an all-terminal long-lived generic queue.
#[must_use = "all-terminal queue release and complete round lineage remain retained"]
#[derive(Debug)]
pub struct M1LongLivedQueueAllTerminalShutdownSuccessV1 {
    released: crate::M1ReleasedAllTerminalQueueShutdownSuccessV1,
    terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
    history: M1NonEmptyRearmRoundHistoryV1,
}

impl M1LongLivedQueueAllTerminalShutdownSuccessV1 {
    #[must_use = "confirmed queue destruction and current-step custody remain retained"]
    pub const fn released(&self) -> &crate::M1ReleasedAllTerminalQueueShutdownSuccessV1 {
        &self.released
    }

    #[must_use]
    pub const fn parked_count(&self) -> usize {
        0
    }

    #[must_use]
    pub const fn terminal_lineage_count(&self) -> usize {
        self.terminal.len()
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.history.get(index)
    }
}

/// Retry-safe preflight rejection retaining one complete released round.
#[must_use = "the unchanged long-lived released round remains retry-capable"]
#[derive(Debug)]
pub struct M1LongLivedQueueAllTerminalShutdownRejectionV1 {
    error: crate::M1AllTerminalQueueShutdownErrorV1,
    released: Box<M1LongLivedQueueReleasedRoundV1>,
}

impl M1LongLivedQueueAllTerminalShutdownRejectionV1 {
    #[must_use]
    pub const fn error(&self) -> crate::M1AllTerminalQueueShutdownErrorV1 {
        self.error
    }

    #[must_use = "the complete unchanged released round remains retained"]
    pub const fn released(&self) -> &M1LongLivedQueueReleasedRoundV1 {
        &self.released
    }

    /// Returns the rejection and unchanged long-lived owner exactly once.
    #[must_use = "the rejection and long-lived released round remain retained"]
    pub fn into_parts(
        self,
    ) -> (
        crate::M1AllTerminalQueueShutdownErrorV1,
        M1LongLivedQueueReleasedRoundV1,
    ) {
        (self.error, *self.released)
    }
}

/// Exhaustive long-lived all-terminal rejection or terminal quarantine.
#[must_use = "all-terminal long-lived shutdown failure retains complete custody"]
#[derive(Debug)]
pub enum M1LongLivedQueueAllTerminalShutdownFailureV1 {
    /// Pure preflight rejection retaining the unchanged round.
    Rejected(Box<M1LongLivedQueueAllTerminalShutdownRejectionV1>),
    /// Native release failed and the Engine was permanently quarantined.
    Quarantined(Box<M1LongLivedQueueRearmTeardownFailureV1>),
}

pub(crate) fn preflight_all_terminal_rearm_shutdown(
    parked_count: usize,
) -> Result<(), crate::M1AllTerminalQueueShutdownErrorV1> {
    if parked_count == 0 {
        Ok(())
    } else {
        Err(crate::M1AllTerminalQueueShutdownErrorV1::ParkedMembers {
            count: parked_count,
        })
    }
}

impl M1LongLivedQueueReleasedRoundV1 {
    pub const fn current_released(&self) -> &M1ReleasedCompletedStepV1 {
        &self.released
    }

    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.parked.len()
    }

    #[must_use]
    pub const fn terminal_lineage_count(&self) -> usize {
        self.terminal.len()
    }

    pub const fn prior_checked(&self) -> &crate::M1CheckedCompletionOutputV1 {
        self.history.latest().checked()
    }

    #[must_use]
    pub fn prior_logical_accepted_counts(&self) -> &[u32] {
        self.history.latest().logical_accepted_counts()
    }

    #[must_use]
    pub fn prior_externally_published_counts(&self) -> &[u32] {
        self.history.latest().externally_published_counts()
    }

    #[must_use]
    pub fn prior_release_counts(&self) -> &[M1CompletedKvPageReleaseCountsV1] {
        self.history.latest().release_counts()
    }

    #[must_use]
    pub const fn prior_completed_members(&self) -> usize {
        self.history.latest().completed_members()
    }

    #[must_use]
    pub const fn prior_total_released(&self) -> usize {
        self.history.latest().total_released()
    }

    #[must_use]
    pub const fn queue_observation(&self) -> ComputeAqlQueueObservationV1 {
        self.history.latest().queue_observation()
    }

    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.history.latest().device()
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.history.get(index)
    }

    /// Shuts down an all-terminal long-lived queue without faulting its Engine.
    ///
    /// Parked ownership is rejected before any queue or released-step owner is
    /// consumed. The lower transition then requires every current member and
    /// the Engine itself to be quiescent. Only confirmed native destruction
    /// returns success with a still-healthy Engine; ambiguous native failure
    /// returns the existing terminal quarantine joined to all round lineage.
    ///
    /// # Errors
    ///
    /// Returns the unchanged round on pure preflight rejection, or terminal
    /// exhaustive quarantine after the native release attempt fails.
    pub fn shutdown_all_terminal_queue<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1LongLivedQueueAllTerminalShutdownSuccessV1,
        M1LongLivedQueueAllTerminalShutdownFailureV1,
    > {
        if let Err(error) = preflight_all_terminal_rearm_shutdown(self.parked.len()) {
            return Err(M1LongLivedQueueAllTerminalShutdownFailureV1::Rejected(
                Box::new(M1LongLivedQueueAllTerminalShutdownRejectionV1 {
                    error,
                    released: Box::new(self),
                }),
            ));
        }
        let Self {
            released,
            parked,
            terminal,
            history,
        } = self;
        match released.shutdown_all_terminal_queue(engine) {
            Ok(released) => Ok(M1LongLivedQueueAllTerminalShutdownSuccessV1 {
                released,
                terminal,
                history,
            }),
            Err(crate::M1ReleasedAllTerminalQueueShutdownFailureV1::Rejected(rejection)) => {
                let (error, released) = rejection.into_parts();
                Err(M1LongLivedQueueAllTerminalShutdownFailureV1::Rejected(
                    Box::new(M1LongLivedQueueAllTerminalShutdownRejectionV1 {
                        error,
                        released: Box::new(Self {
                            released,
                            parked,
                            terminal,
                            history,
                        }),
                    }),
                ))
            }
            Err(crate::M1ReleasedAllTerminalQueueShutdownFailureV1::Quarantined(released)) => {
                Err(M1LongLivedQueueAllTerminalShutdownFailureV1::Quarantined(
                    Box::new(M1LongLivedQueueRearmTeardownFailureV1 {
                        released,
                        parked,
                        terminal,
                        history: M1RearmRoundHistoryV1::NonEmpty(history),
                    }),
                ))
            }
        }
    }

    /// Destroys the completed queue while retaining current and prior lineage.
    ///
    /// This is the terminal route after every current member was completed with
    /// `Retire`. Any parked active cache remains visible in the returned closed
    /// teardown owner; callers therefore must not claim whole-roster retirement
    /// unless [`Self::parked_count`] was zero before consuming this value.
    ///
    /// ```compile_fail
    /// use ferric_engine::{Engine, M1LongLivedQueueReleasedRoundV1};
    /// fn teardown_twice(released: M1LongLivedQueueReleasedRoundV1, engine: &mut Engine<32>) {
    ///     let _first = released.destroy_queue_and_retain_round(engine);
    ///     let _second = released.destroy_queue_and_retain_round(engine);
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns terminal lower-layer queue release quarantine together with all
    /// current, parked, terminal, and prior-round observation custody.
    pub fn destroy_queue_and_retain_round<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<M1LongLivedQueueRearmTeardownSuccessV1, Box<M1LongLivedQueueRearmTeardownFailureV1>>
    {
        let Self {
            released,
            parked,
            terminal,
            history,
        } = self;
        match released.destroy_queue_and_retain_step(engine) {
            Ok(released) => Ok(M1LongLivedQueueRearmTeardownSuccessV1 {
                released,
                parked,
                terminal,
                history: M1RearmRoundHistoryV1::NonEmpty(history),
            }),
            Err(released) => Err(Box::new(M1LongLivedQueueRearmTeardownFailureV1 {
                released,
                parked,
                terminal,
                history: M1RearmRoundHistoryV1::NonEmpty(history),
            })),
        }
    }

    /// Consumes current released and separately parked active custody into the
    /// same exact scheduling transition used for the first rearm round.
    ///
    /// # Errors
    ///
    /// Returns closed scheduling failure retaining current queue, parked
    /// caches, terminal lineage, and any scheduler-issued dispatch.
    pub fn schedule_next<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<M1ScheduledLongLivedQueueRearmV1, M1LongLivedQueueRearmScheduleFailureV1> {
        let Self {
            released,
            parked,
            terminal,
            history,
        } = self;
        schedule_m1_long_lived_queue_rearm_with_lineage_v1(
            engine,
            released,
            parked,
            terminal,
            M1RearmRoundHistoryV1::NonEmpty(history),
        )
    }

    /// Consumes current released custody and schedules exactly the named Ready
    /// subset in caller-provided lane order at `expected_epoch`.
    ///
    /// Unnamed active caches remain parked. Duplicate or unowned requests are
    /// rejected before detach; later exact scheduler rejection returns terminal
    /// exhaustive custody after detach.
    ///
    /// # Errors
    ///
    /// Returns phase-tagged exhaustive custody for any preflight, detach,
    /// exact-scheduler, or post-dispatch rejection.
    pub fn schedule_next_exact<const C: usize>(
        self,
        engine: &mut Engine<C>,
        expected_epoch: CompletionEpoch,
        requests: &[RequestId],
    ) -> Result<M1ScheduledLongLivedQueueRearmV1, M1LongLivedQueueRearmScheduleFailureV1> {
        let Self {
            released,
            parked,
            terminal,
            history,
        } = self;
        schedule_m1_long_lived_queue_rearm_exact_with_lineage_v1(
            engine,
            released,
            parked,
            terminal,
            M1RearmRoundHistoryV1::NonEmpty(history),
            expected_epoch,
            requests,
        )
    }
}

/// Retry-safe page-release rejection retaining the separately parked lineage.
#[must_use = "page-release rejection remains the sole retry owner"]
#[derive(Debug)]
pub struct M1RearmedRoundPageReleaseFailureV1 {
    source: Box<crate::M1CompletedStepKvReleaseFailureV1>,
    parked: Vec<ActiveDeviceKvCache>,
    terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
    history: M1NonEmptyRearmRoundHistoryV1,
}

impl M1RearmedRoundPageReleaseFailureV1 {
    #[must_use]
    pub const fn source(&self) -> &crate::M1CompletedStepKvReleaseErrorV1 {
        self.source.error()
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.history.get(index)
    }

    /// Retries exact page release with the unchanged completed owner.
    #[must_use = "retry outcome retains every current and parked owner"]
    pub fn retry(self) -> M1RearmedRoundReleaseOutcomeV1 {
        let (_error, completed) = (*self.source).into_parts();
        release_rearmed_round(completed, self.history, self.parked, self.terminal)
    }

    /// Destroys the physical queue after page release cannot make progress,
    /// retaining the completed current round and every parked/prior owner.
    ///
    /// # Errors
    ///
    /// Returns terminal lower queue-release quarantine joined to the same
    /// release diagnostic and round lineage.
    pub fn destroy_queue_and_retain_round<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1RearmedRoundPageReleaseTeardownSuccessV1,
        Box<M1RearmedRoundPageReleaseTeardownFailureV1>,
    > {
        let (error, completed) = (*self.source).into_parts();
        match join_terminal_lineage(
            completed.destroy_queue_and_retain_completion(engine),
            self.parked,
            self.terminal,
            self.history,
        ) {
            Ok(joined) => Ok(M1RearmedRoundPageReleaseTeardownSuccessV1 {
                error,
                completed: joined.source,
                parked: joined.parked,
                terminal: joined.terminal,
                history: joined.history,
            }),
            Err(joined) => Err(Box::new(M1RearmedRoundPageReleaseTeardownFailureV1 {
                error,
                completed: joined.source,
                parked: joined.parked,
                terminal: joined.terminal,
                history: joined.history,
            })),
        }
    }
}

/// Clean queue teardown retaining a completed round after page-release
/// exhaustion.
#[must_use = "page-release diagnostic and complete round lineage remain retained"]
#[derive(Debug)]
pub struct M1RearmedRoundPageReleaseTeardownSuccessV1 {
    error: crate::M1CompletedStepKvReleaseErrorV1,
    completed: crate::M1CompletedStepTeardownSuccessV1,
    parked: Vec<ActiveDeviceKvCache>,
    terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
    history: M1NonEmptyRearmRoundHistoryV1,
}

/// Terminal queue-release quarantine retaining a completed round after
/// page-release exhaustion.
#[must_use = "page-release and queue-release quarantine retain complete round lineage"]
#[derive(Debug)]
pub struct M1RearmedRoundPageReleaseTeardownFailureV1 {
    error: crate::M1CompletedStepKvReleaseErrorV1,
    completed: Box<crate::M1CompletedStepTeardownFailureV1>,
    parked: Vec<ActiveDeviceKvCache>,
    terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
    history: M1NonEmptyRearmRoundHistoryV1,
}

impl M1RearmedRoundPageReleaseTeardownSuccessV1 {
    #[must_use]
    pub const fn error(&self) -> &crate::M1CompletedStepKvReleaseErrorV1 {
        &self.error
    }

    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.parked.len()
    }

    #[must_use]
    pub const fn terminal_lineage_count(&self) -> usize {
        self.terminal.len()
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.history.get(index)
    }

    pub const fn completed(&self) -> &crate::M1CompletedStepTeardownSuccessV1 {
        &self.completed
    }
}

impl M1RearmedRoundPageReleaseTeardownFailureV1 {
    #[must_use]
    pub const fn error(&self) -> &crate::M1CompletedStepKvReleaseErrorV1 {
        &self.error
    }

    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.parked.len()
    }

    #[must_use]
    pub const fn terminal_lineage_count(&self) -> usize {
        self.terminal.len()
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.history.get(index)
    }

    pub const fn completed(&self) -> &crate::M1CompletedStepTeardownFailureV1 {
        &self.completed
    }
}

/// Exhaustive transition from completion into another schedulable round.
#[must_use = "every release outcome retains exact linear custody"]
#[derive(Debug)]
pub enum M1RearmedRoundReleaseOutcomeV1 {
    Released(M1LongLivedQueueReleasedRoundV1),
    Rejected(Box<M1RearmedRoundPageReleaseFailureV1>),
    NotCompleted(M1RearmedCompletionOutcomeV1),
}

fn release_rearmed_round(
    completed: crate::M1CompletedStepSuccessV1,
    history: M1NonEmptyRearmRoundHistoryV1,
    parked: Vec<ActiveDeviceKvCache>,
    terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
) -> M1RearmedRoundReleaseOutcomeV1 {
    match crate::release_m1_completed_step_kv_pages_v1(completed) {
        Ok(released) => M1RearmedRoundReleaseOutcomeV1::Released(M1LongLivedQueueReleasedRoundV1 {
            released,
            parked,
            terminal,
            history,
        }),
        Err(source) => {
            M1RearmedRoundReleaseOutcomeV1::Rejected(Box::new(M1RearmedRoundPageReleaseFailureV1 {
                source,
                parked,
                terminal,
                history,
            }))
        }
    }
}

impl M1RearmedCompletedReadbackV1 {
    /// Semantically checked compact completion retained before KV settlement.
    pub const fn checked(&self) -> &crate::M1CheckedCompletionOutputV1 {
        self.readback.checked()
    }

    /// Completes the fresh physical generation using selected caches in the
    /// exact scheduler order retained since scheduling.
    ///
    /// # Errors
    ///
    /// Returns unchanged readback/cache custody and all supplied dispositions
    /// for a count mismatch or host reservation failure.
    pub fn complete<const C: usize>(
        mut self,
        engine: &mut Engine<C>,
        dispositions: Vec<crate::M1DeviceKvCompletionDispositionV1>,
    ) -> Result<M1RearmedCompletionOutcomeV1, M1RearmedCompletionPreflightFailureV1> {
        if dispositions.len() != self.carry.selected.len() {
            return Err(M1RearmedCompletionPreflightFailureV1 {
                error: M1RearmedCompletionPreflightErrorV1::DispositionCount {
                    expected: self.carry.selected.len(),
                    actual: dispositions.len(),
                },
                readback: Box::new(self),
                dispositions,
            });
        }
        if let Err(error) = self.carry.history.try_reserve_append() {
            return Err(M1RearmedCompletionPreflightFailureV1 {
                error,
                readback: Box::new(self),
                dispositions,
            });
        }
        let mut members = Vec::new();
        if members
            .try_reserve_exact(self.carry.selected.len())
            .is_err()
        {
            return Err(M1RearmedCompletionPreflightFailureV1 {
                error: M1RearmedCompletionPreflightErrorV1::HostAllocation,
                readback: Box::new(self),
                dispositions,
            });
        }
        let Self {
            readback,
            carry,
            queue_observation,
            device,
        } = self;
        for (cache, disposition) in carry.selected.into_iter().zip(dispositions) {
            members.push(match disposition {
                crate::M1DeviceKvCompletionDispositionV1::Continue => {
                    crate::M1DeviceKvCompletionMemberV1::continuing(cache)
                }
                crate::M1DeviceKvCompletionDispositionV1::Retire => {
                    crate::M1DeviceKvCompletionMemberV1::retiring(cache)
                }
            });
        }
        let roster = crate::M1DeviceKvCompletionRosterV1::new(members);
        let outcome = crate::complete_m1_physical_step_v1(engine, readback, roster);
        let history = carry.history.append(M1RearmRoundHistoryEntryV1 {
            checked: carry.prior_checked,
            logical_accepted_counts: carry.logical_accepted_counts,
            externally_published_counts: carry.externally_published_counts,
            release_counts: carry.release_counts,
            completed_members: carry.completed_members,
            total_released: carry.total_released,
            queue_observation,
            device,
            rollover: carry.rollover,
        });
        Ok(M1RearmedCompletionOutcomeV1 {
            outcome,
            parked: carry.parked,
            terminal: carry.terminal,
            history,
        })
    }
}

fn replace_target<const N: usize>(
    queue: ServiceQueueUnboundSessionV1,
    old: &BoundM1StepWorkspaceSubleases<N>,
    plan: AddresslessM1StepWorkspacePlan,
    bytes: Box<[u8]>,
    descriptor: Gfx942DeviceContentDescriptorV1,
) -> Result<
    (
        ServiceQueueUnboundSessionV1,
        BoundM1StepWorkspaceSubleases<N>,
        [ServiceDeviceDispatchRangeV1; N],
    ),
    WorkspaceReplacementFailureV1<N>,
> {
    let allocation = plan.allocation();
    let update = queue
        .replace_initialized_partitioned_device_local::<DeviceWorkspaceRoleV1, N, N>(
            old.replacement_subleases(),
            bytes,
            allocation.alignment(),
            descriptor,
            member_layout(&plan),
        )
        .map_err(WorkspaceReplacementFailureV1::Update)?;
    bind_queue_replaced_m1_step_workspace(plan, update)
        .map_err(WorkspaceReplacementFailureV1::Binding)
}

fn replace_rollover_workspace<const OLD_N: usize, const NEW_N: usize>(
    queue: ServiceQueueUnboundSessionV1,
    old: &BoundM1StepWorkspaceSubleases<OLD_N>,
    plan: AddresslessM1StepWorkspacePlan,
    bytes: Box<[u8]>,
    descriptor: Gfx942DeviceContentDescriptorV1,
) -> Result<
    (
        ServiceQueueUnboundSessionV1,
        BoundM1StepWorkspaceSubleases<NEW_N>,
        [ServiceDeviceDispatchRangeV1; NEW_N],
    ),
    WorkspaceReplacementFailureV1<NEW_N>,
> {
    let allocation = plan.allocation();
    let update = queue
        .replace_initialized_partitioned_device_local::<DeviceWorkspaceRoleV1, OLD_N, NEW_N>(
            old.replacement_subleases(),
            bytes,
            allocation.alignment(),
            descriptor,
            member_layout(&plan),
        )
        .map_err(WorkspaceReplacementFailureV1::Update)?;
    bind_queue_replaced_m1_step_workspace(plan, update)
        .map_err(WorkspaceReplacementFailureV1::Binding)
}

#[inline(never)]
fn bind_submit_target_only(
    lower: ServiceQueueUnboundSessionV1,
    batch: Box<ServiceFixedBatchV1<'_, M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1>>,
    custody: M1PhysicalQueueBatchCustodyV1,
    step: M1PrepublicationStepCustodyV1,
    expected_observation: ComputeAqlQueueObservationV1,
) -> Result<M1PhysicalPublishedQueueSessionV1, M1LongLivedQueueRearmSubmissionFailureV1<'_>> {
    let lower = match lower.bind(*batch) {
        Ok(lower) => lower,
        Err(failure) => {
            return Err(submission_failure(
                M1LongLivedQueueRearmSubmissionPhaseV1::QueueBind,
                (failure, custody, step),
            ));
        }
    };
    if lower.observation() != expected_observation {
        return Err(submission_failure(
            M1LongLivedQueueRearmSubmissionPhaseV1::QueueObservation,
            (lower, custody, step),
        ));
    }
    M1PhysicalQueueSessionV1::TargetOnly(Box::new(M1PhysicalQueuePhaseCaseV1::from_queue_rearm(
        lower, custody, step,
    )))
    .submit()
    .map_err(|failure| {
        submission_failure(M1LongLivedQueueRearmSubmissionPhaseV1::QueueSubmit, failure)
    })
}

#[inline(never)]
fn bind_submit_speculative_k4(
    lower: ServiceQueueUnboundSessionV1,
    batch: Box<ServiceFixedBatchV1<'_, M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1>>,
    custody: M1PhysicalQueueBatchCustodyV1,
    step: M1PrepublicationStepCustodyV1,
    expected_observation: ComputeAqlQueueObservationV1,
) -> Result<M1PhysicalPublishedQueueSessionV1, M1LongLivedQueueRearmSubmissionFailureV1<'_>> {
    let lower = match lower.bind(*batch) {
        Ok(lower) => lower,
        Err(failure) => {
            return Err(submission_failure(
                M1LongLivedQueueRearmSubmissionPhaseV1::QueueBind,
                (failure, custody, step),
            ));
        }
    };
    if lower.observation() != expected_observation {
        return Err(submission_failure(
            M1LongLivedQueueRearmSubmissionPhaseV1::QueueObservation,
            (lower, custody, step),
        ));
    }
    M1PhysicalQueueSessionV1::SpeculativeK4(Box::new(M1PhysicalQueuePhaseCaseV1::from_queue_rearm(
        lower, custody, step,
    )))
    .submit()
    .map_err(|failure| {
        submission_failure(M1LongLivedQueueRearmSubmissionPhaseV1::QueueSubmit, failure)
    })
}

#[inline(never)]
fn bind_submit_speculative_k8(
    lower: ServiceQueueUnboundSessionV1,
    batch: Box<ServiceFixedBatchV1<'_, M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1>>,
    custody: M1PhysicalQueueBatchCustodyV1,
    step: M1PrepublicationStepCustodyV1,
    expected_observation: ComputeAqlQueueObservationV1,
) -> Result<M1PhysicalPublishedQueueSessionV1, M1LongLivedQueueRearmSubmissionFailureV1<'_>> {
    let lower = match lower.bind(*batch) {
        Ok(lower) => lower,
        Err(failure) => {
            return Err(submission_failure(
                M1LongLivedQueueRearmSubmissionPhaseV1::QueueBind,
                (failure, custody, step),
            ));
        }
    };
    if lower.observation() != expected_observation {
        return Err(submission_failure(
            M1LongLivedQueueRearmSubmissionPhaseV1::QueueObservation,
            (lower, custody, step),
        ));
    }
    M1PhysicalQueueSessionV1::SpeculativeK8(Box::new(M1PhysicalQueuePhaseCaseV1::from_queue_rearm(
        lower, custody, step,
    )))
    .submit()
    .map_err(|failure| {
        submission_failure(M1LongLivedQueueRearmSubmissionPhaseV1::QueueSubmit, failure)
    })
}

#[inline(never)]
fn bind_submit_speculative_k16(
    lower: ServiceQueueUnboundSessionV1,
    batch: Box<ServiceFixedBatchV1<'_, M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1>>,
    custody: M1PhysicalQueueBatchCustodyV1,
    step: M1PrepublicationStepCustodyV1,
    expected_observation: ComputeAqlQueueObservationV1,
) -> Result<M1PhysicalPublishedQueueSessionV1, M1LongLivedQueueRearmSubmissionFailureV1<'_>> {
    let lower = match lower.bind(*batch) {
        Ok(lower) => lower,
        Err(failure) => {
            return Err(submission_failure(
                M1LongLivedQueueRearmSubmissionPhaseV1::QueueBind,
                (failure, custody, step),
            ));
        }
    };
    if lower.observation() != expected_observation {
        return Err(submission_failure(
            M1LongLivedQueueRearmSubmissionPhaseV1::QueueObservation,
            (lower, custody, step),
        ));
    }
    M1PhysicalQueueSessionV1::SpeculativeK16(Box::new(
        M1PhysicalQueuePhaseCaseV1::from_queue_rearm(lower, custody, step),
    ))
    .submit()
    .map_err(|failure| {
        submission_failure(M1LongLivedQueueRearmSubmissionPhaseV1::QueueSubmit, failure)
    })
}

#[derive(Debug)]
struct DiagnosticCaptureResetFailureV1 {
    phase: M1LongLivedQueueRearmSubmissionPhaseV1,
    _retained: Box<dyn fmt::Debug>,
}

fn diagnostic_capture_reset_failure(
    phase: M1LongLivedQueueRearmSubmissionPhaseV1,
    retained: impl fmt::Debug + 'static,
) -> DiagnosticCaptureResetFailureV1 {
    DiagnosticCaptureResetFailureV1 {
        phase,
        _retained: Box::new(retained),
    }
}

fn reset_retained_diagnostic_capture(
    lower: ServiceQueueUnboundSessionV1,
    mut completion: crate::BoundM1CompletionOutputV1,
) -> Result<
    (
        ServiceQueueUnboundSessionV1,
        crate::BoundM1CompletionOutputV1,
    ),
    DiagnosticCaptureResetFailureV1,
> {
    if completion.direct_diagnostic_choices().is_some() {
        let (old, image) = {
            let choices = completion
                .direct_diagnostic_choices()
                .expect("presence checked above");
            let image = match choices.replacement_image() {
                Ok(image) => image,
                Err(error) => {
                    return Err(diagnostic_capture_reset_failure(
                        M1LongLivedQueueRearmSubmissionPhaseV1::DirectDiagnosticChoiceReplacement,
                        (lower, completion, error),
                    ));
                }
            };
            (choices.retained_range(), image)
        };
        let update = match lower.replace_initialized_host_visible::<HostDownloadRoleV1>(old, image)
        {
            Ok(update) => update,
            Err(failure) => {
                return Err(diagnostic_capture_reset_failure(
                    M1LongLivedQueueRearmSubmissionPhaseV1::DirectDiagnosticChoiceReplacement,
                    (failure, completion),
                ));
            }
        };
        let (lower, range, _snapshot) = update.into_parts();
        if let Err(error) = completion
            .direct_diagnostic_choices_mut()
            .expect("presence checked above")
            .replace_retained_range(range)
        {
            return Err(diagnostic_capture_reset_failure(
                M1LongLivedQueueRearmSubmissionPhaseV1::DirectDiagnosticChoiceReplacement,
                (lower, completion, error),
            ));
        }
        return Ok((lower, completion));
    }

    if completion.speculative_diagnostic_choices().is_some() {
        let (old_draft, draft_image, old_target, target_image) = {
            let choices = completion
                .speculative_diagnostic_choices()
                .expect("presence checked above");
            let draft_image = match choices.replacement_draft_image() {
                Ok(image) => image,
                Err(error) => {
                    return Err(diagnostic_capture_reset_failure(
                        M1LongLivedQueueRearmSubmissionPhaseV1::SpeculativeDraftChoiceReplacement,
                        (lower, completion, error),
                    ));
                }
            };
            let target_image = match choices.replacement_target_image() {
                Ok(image) => image,
                Err(error) => {
                    return Err(diagnostic_capture_reset_failure(
                        M1LongLivedQueueRearmSubmissionPhaseV1::SpeculativeTargetChoiceReplacement,
                        (lower, completion, draft_image, error),
                    ));
                }
            };
            (
                choices.retained_draft_range(),
                draft_image,
                choices.retained_target_range(),
                target_image,
            )
        };
        let draft_update = match lower
            .replace_initialized_host_visible::<HostDownloadRoleV1>(old_draft, draft_image)
        {
            Ok(update) => update,
            Err(failure) => {
                return Err(diagnostic_capture_reset_failure(
                    M1LongLivedQueueRearmSubmissionPhaseV1::SpeculativeDraftChoiceReplacement,
                    (failure, completion, target_image),
                ));
            }
        };
        let (lower, draft_range, _draft_snapshot) = draft_update.into_parts();
        if let Err(error) = completion
            .speculative_diagnostic_choices_mut()
            .expect("presence checked above")
            .replace_retained_draft_range(draft_range)
        {
            return Err(diagnostic_capture_reset_failure(
                M1LongLivedQueueRearmSubmissionPhaseV1::SpeculativeDraftChoiceReplacement,
                (lower, completion, target_image, error),
            ));
        }
        let target_update = match lower
            .replace_initialized_host_visible::<HostDownloadRoleV1>(old_target, target_image)
        {
            Ok(update) => update,
            Err(failure) => {
                return Err(diagnostic_capture_reset_failure(
                    M1LongLivedQueueRearmSubmissionPhaseV1::SpeculativeTargetChoiceReplacement,
                    (failure, completion),
                ));
            }
        };
        let (lower, target_range, _target_snapshot) = target_update.into_parts();
        if let Err(error) = completion
            .speculative_diagnostic_choices_mut()
            .expect("presence checked above")
            .replace_retained_target_range(target_range)
        {
            return Err(diagnostic_capture_reset_failure(
                M1LongLivedQueueRearmSubmissionPhaseV1::SpeculativeTargetChoiceReplacement,
                (lower, completion, error),
            ));
        }
        return Ok((lower, completion));
    }

    Ok((lower, completion))
}

pub(crate) fn retained_host_capture_ranges(
    completion: &crate::BoundM1CompletionOutputV1,
) -> Result<RetainedHostCaptureRangesV1, ()> {
    let qualification = completion.qualification_logits();
    let direct = completion.direct_diagnostic_choices();
    let speculative = completion.speculative_diagnostic_choices();
    let semantic = match (qualification, direct, speculative) {
        (None, None, None) => RetainedSemanticCaptureRangesV1::Ordinary,
        (Some(logits), None, None) => RetainedSemanticCaptureRangesV1::Qualification {
            logits: logits.retained_host_dispatch_range(),
        },
        (None, Some(choices), None) => RetainedSemanticCaptureRangesV1::DirectDiagnostic {
            choices: choices.retained_range(),
        },
        (None, None, Some(choices)) => RetainedSemanticCaptureRangesV1::SpeculativeDiagnostic {
            draft: choices.retained_draft_range(),
            draft_tokens: choices.shape().draft_tokens(),
            draft_rows: choices.retained_draft_read_ranges().map_err(|_| ())?,
            target: choices.retained_target_range(),
        },
        _ => return Err(()),
    };
    Ok(RetainedHostCaptureRangesV1 {
        ranges: RetainedCaptureRangesV1 {
            completion_output: completion.retained_host_dispatch_range(),
            semantic,
        },
        completion_snapshot: completion
            .completion_canary()
            .map(crate::BoundM1CompletionCanaryV1::snapshot_range),
    })
}

#[allow(clippy::too_many_arguments)]
fn finish_rearm_submission(
    shape: M1PhysicalFixedBatchShapeV1,
    lower: ServiceQueueUnboundSessionV1,
    mut custody: M1PhysicalQueueBatchRearmPartsV1,
    workspace_ranges: Vec<FreshWorkspaceRangeV1>,
    recipe: AddresslessM1PhysicalBufferRecipeV1,
    catalog: ContentBoundM1ProgramCatalogV1<'_>,
    step: M1PrepublicationStepCustodyV1,
    expected_observation: ComputeAqlQueueObservationV1,
) -> Result<M1PhysicalPublishedQueueSessionV1, M1LongLivedQueueRearmSubmissionFailureV1<'_>> {
    let previous_capture = match retained_host_capture_ranges(&custody.completion_output) {
        Ok(retained_capture) => retained_capture,
        Err(()) => {
            return Err(submission_failure(
                M1LongLivedQueueRearmSubmissionPhaseV1::WorkspaceRangeRebinding,
                (lower, custody, recipe, catalog, step),
            ));
        }
    };
    let (lower, completion_output) =
        match reset_retained_diagnostic_capture(lower, custody.completion_output) {
            Ok(reset) => reset,
            Err(failure) => {
                return Err(submission_failure(
                    failure.phase,
                    (
                        failure,
                        custody.catalog_id,
                        custody.selection,
                        custody.physical_recipe,
                        custody.workspace_composition,
                        custody.workspace_owners,
                        custody.source_rows,
                        custody.bound_rows,
                        custody.partitioned_memory,
                        recipe,
                        catalog,
                        step,
                    ),
                ));
            }
        };
    custody.completion_output = completion_output;
    let retained_capture = match retained_host_capture_ranges(&custody.completion_output) {
        Ok(retained_capture) => retained_capture,
        Err(()) => {
            return Err(submission_failure(
                M1LongLivedQueueRearmSubmissionPhaseV1::WorkspaceRangeRebinding,
                (lower, custody, recipe, catalog, step),
            ));
        }
    };
    let bound_rows = match rebuild_bound_rows(
        recipe.rows(),
        &custody.bound_rows,
        recipe.workspace_composition(),
        &workspace_ranges,
        &previous_capture,
        &retained_capture,
    ) {
        Ok(rows) => rows,
        Err(()) => {
            return Err(submission_failure(
                M1LongLivedQueueRearmSubmissionPhaseV1::WorkspaceRangeRebinding,
                (lower, custody, recipe, catalog, step),
            ));
        }
    };
    let (kernargs, workspace_composition, source_rows) = recipe.into_parts();
    let (physical_recipe, images) = kernargs.into_parts();
    custody.physical_recipe = physical_recipe;
    custody.workspace_composition = workspace_composition;
    custody.source_rows = source_rows;
    custody.bound_rows = bound_rows;
    let batch = match shape {
        M1PhysicalFixedBatchShapeV1::TargetOnly => {
            match lower_boxed_rearm_batch(
                catalog,
                &custody.physical_recipe,
                images,
                &custody.bound_rows,
            ) {
                Ok(batch) => RebuiltBatchV1::TargetOnly(batch),
                Err(failure) => {
                    return Err(submission_failure(
                        M1LongLivedQueueRearmSubmissionPhaseV1::FixedBatchRebuild,
                        (lower, custody, failure.catalog, failure.images, step),
                    ));
                }
            }
        }
        M1PhysicalFixedBatchShapeV1::SpeculativeK4 => {
            match lower_boxed_rearm_batch(
                catalog,
                &custody.physical_recipe,
                images,
                &custody.bound_rows,
            ) {
                Ok(batch) => RebuiltBatchV1::SpeculativeK4(batch),
                Err(failure) => {
                    return Err(submission_failure(
                        M1LongLivedQueueRearmSubmissionPhaseV1::FixedBatchRebuild,
                        (lower, custody, failure.catalog, failure.images, step),
                    ));
                }
            }
        }
        M1PhysicalFixedBatchShapeV1::SpeculativeK8 => {
            match lower_boxed_rearm_batch(
                catalog,
                &custody.physical_recipe,
                images,
                &custody.bound_rows,
            ) {
                Ok(batch) => RebuiltBatchV1::SpeculativeK8(batch),
                Err(failure) => {
                    return Err(submission_failure(
                        M1LongLivedQueueRearmSubmissionPhaseV1::FixedBatchRebuild,
                        (lower, custody, failure.catalog, failure.images, step),
                    ));
                }
            }
        }
        M1PhysicalFixedBatchShapeV1::SpeculativeK16 => {
            match lower_boxed_rearm_batch(
                catalog,
                &custody.physical_recipe,
                images,
                &custody.bound_rows,
            ) {
                Ok(batch) => RebuiltBatchV1::SpeculativeK16(batch),
                Err(failure) => {
                    return Err(submission_failure(
                        M1LongLivedQueueRearmSubmissionPhaseV1::FixedBatchRebuild,
                        (lower, custody, failure.catalog, failure.images, step),
                    ));
                }
            }
        }
        M1PhysicalFixedBatchShapeV1::PairedPrefill => {
            return Err(submission_failure(
                M1LongLivedQueueRearmSubmissionPhaseV1::FixedBatchRebuild,
                (lower, custody, catalog, images, step),
            ));
        }
    };
    let custody = M1PhysicalQueueBatchCustodyV1::from_rearm_parts(custody);
    match batch {
        RebuiltBatchV1::TargetOnly(batch) => {
            bind_submit_target_only(lower, batch, custody, step, expected_observation)
        }
        RebuiltBatchV1::SpeculativeK4(batch) => {
            bind_submit_speculative_k4(lower, batch, custody, step, expected_observation)
        }
        RebuiltBatchV1::SpeculativeK8(batch) => {
            bind_submit_speculative_k8(lower, batch, custody, step, expected_observation)
        }
        RebuiltBatchV1::SpeculativeK16(batch) => {
            bind_submit_speculative_k16(lower, batch, custody, step, expected_observation)
        }
    }
}

#[derive(Debug)]
struct PostQueueRemainderV1 {
    selected: Vec<ActiveDeviceKvCache>,
    parked: Vec<ActiveDeviceKvCache>,
    terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
    prior_checked: crate::M1CheckedCompletionOutputV1,
    logical_accepted_counts: Box<[u32]>,
    externally_published_counts: Box<[u32]>,
    release_counts: Box<[M1CompletedKvPageReleaseCountsV1]>,
    completed_members: usize,
    total_released: usize,
    history: M1RearmRoundHistoryV1,
}

struct StagedRearmSubmissionV1<'a> {
    shape: M1PhysicalFixedBatchShapeV1,
    lower: ServiceQueueUnboundSessionV1,
    custody: M1PhysicalQueueBatchRearmPartsV1,
    workspace_ranges: Vec<FreshWorkspaceRangeV1>,
    recipe: AddresslessM1PhysicalBufferRecipeV1,
    catalog: ContentBoundM1ProgramCatalogV1<'a>,
    step: M1PrepublicationStepCustodyV1,
    expected_observation: ComputeAqlQueueObservationV1,
    post: PostQueueRemainderV1,
    previous_epoch: CompletionEpoch,
    device: Gfx942DeviceBinding,
}

// Finish preparation before any const-cardinality fixed batch is lowered.
#[inline(never)]
fn prepare_rearm_submission(
    prepared: M1PreparedLongLivedQueueRearmV1,
    recipe: AddresslessM1PhysicalBufferRecipeV1,
    catalog: ContentBoundM1ProgramCatalogV1<'_>,
) -> Result<Box<StagedRearmSubmissionV1<'_>>, M1LongLivedQueueRearmSubmissionFailureV1<'_>> {
    if preflight_rearm(&prepared, &recipe, &catalog).is_err() {
        return Err(submission_failure(
            M1LongLivedQueueRearmSubmissionPhaseV1::Preflight,
            (prepared, recipe, catalog),
        ));
    }
    let M1PreparedLongLivedQueueRearmV1 {
        prepared,
        remainder,
    } = prepared;
    let ScheduledRemainderV1 {
        queue,
        selected,
        parked,
        terminal,
        prior_checked,
        logical_accepted_counts,
        externally_published_counts,
        release_counts,
        completed_members,
        total_released,
        history,
    } = remainder;
    let post = PostQueueRemainderV1 {
        selected,
        parked,
        terminal,
        prior_checked,
        logical_accepted_counts,
        externally_published_counts,
        release_counts,
        completed_members,
        total_released,
        history,
    };
    let (shape, lower, custody) = queue.into_rearm_parts();
    let expected_observation = lower.observation();
    let device = custody.device();
    let mut custody = custody.into_rearm_parts();
    let (plans, workspace_images, step) = prepared.into_rearm_parts();
    let previous_epoch = post.prior_checked.epoch();

    let (lower, workspace_owners, workspace_ranges) =
        match (shape, &custody.workspace_owners, plans, workspace_images) {
            (
                M1PhysicalFixedBatchShapeV1::TargetOnly,
                M1FullStepWorkspaceSubleaseOwners::TargetOnly { target: old_target },
                M1FullStepWorkspacePlans::TargetOnly { target: plan },
                M1FullStepWorkspaceImagesV1::TargetOnly { target: bytes },
            ) => {
                let descriptor = match crate::m1_step_workspace_content_descriptor_v1(
                    M1InitializedWorkspaceSlotV1::TargetOnlyTarget,
                    &bytes,
                ) {
                    Ok(descriptor) => descriptor,
                    Err(error) => {
                        return Err(submission_failure(
                            M1LongLivedQueueRearmSubmissionPhaseV1::Preflight,
                            (
                                lower, custody, plan, bytes, recipe, catalog, step, post, error,
                            ),
                        ));
                    }
                };
                let (lower, target, ranges) =
                    match replace_target(lower, old_target, *plan, bytes, descriptor) {
                        Ok(replaced) => replaced,
                        Err(failure) => {
                            let _ = failure.retained_owner_count();
                            return Err(submission_failure(
                                M1LongLivedQueueRearmSubmissionPhaseV1::TargetWorkspaceReplacement,
                                (failure, custody, recipe, catalog, step, post),
                            ));
                        }
                    };
                let mut catalog_ranges = Vec::new();
                if catalog_ranges.try_reserve_exact(ranges.len()).is_err() {
                    return Err(submission_failure(
                        M1LongLivedQueueRearmSubmissionPhaseV1::WorkspaceRangeRebinding,
                        (lower, custody, target, ranges, recipe, catalog, step, post),
                    ));
                }
                append_workspace_ranges(
                    &mut catalog_ranges,
                    M1FullStepWorkspaceRole::Target,
                    &target,
                    ranges,
                );
                (
                    lower,
                    M1FullStepWorkspaceSubleaseOwners::target_only(target),
                    catalog_ranges,
                )
            }
            (
                M1PhysicalFixedBatchShapeV1::SpeculativeK4
                | M1PhysicalFixedBatchShapeV1::SpeculativeK8
                | M1PhysicalFixedBatchShapeV1::SpeculativeK16,
                M1FullStepWorkspaceSubleaseOwners::SpeculativeRound {
                    draft_decode: old_draft,
                    target_speculative: old_target,
                },
                M1FullStepWorkspacePlans::SpeculativeRound {
                    draft_decode: draft_plan,
                    target_speculative: target_plan,
                },
                M1FullStepWorkspaceImagesV1::SpeculativeRound {
                    draft_decode: draft_bytes,
                    target_speculative: target_bytes,
                },
            ) => {
                let draft_descriptor = match crate::m1_step_workspace_content_descriptor_v1(
                    M1InitializedWorkspaceSlotV1::SpeculativeDraftDecode,
                    &draft_bytes,
                ) {
                    Ok(descriptor) => descriptor,
                    Err(error) => {
                        return Err(submission_failure(
                            M1LongLivedQueueRearmSubmissionPhaseV1::Preflight,
                            (
                                lower,
                                custody,
                                draft_plan,
                                target_plan,
                                draft_bytes,
                                target_bytes,
                                recipe,
                                catalog,
                                step,
                                post,
                                error,
                            ),
                        ));
                    }
                };
                let target_descriptor = match crate::m1_step_workspace_content_descriptor_v1(
                    M1InitializedWorkspaceSlotV1::SpeculativeTarget,
                    &target_bytes,
                ) {
                    Ok(descriptor) => descriptor,
                    Err(error) => {
                        return Err(submission_failure(
                            M1LongLivedQueueRearmSubmissionPhaseV1::Preflight,
                            (
                                lower,
                                custody,
                                draft_plan,
                                target_plan,
                                draft_bytes,
                                target_bytes,
                                recipe,
                                catalog,
                                step,
                                post,
                                error,
                            ),
                        ));
                    }
                };
                let (lower, draft, draft_ranges) = match replace_target(
                    lower,
                    old_draft,
                    *draft_plan,
                    draft_bytes,
                    draft_descriptor,
                ) {
                    Ok(replaced) => replaced,
                    Err(failure) => {
                        let _ = failure.retained_owner_count();
                        return Err(submission_failure(
                            M1LongLivedQueueRearmSubmissionPhaseV1::DraftWorkspaceReplacement,
                            (
                                failure,
                                custody,
                                target_plan,
                                target_bytes,
                                recipe,
                                catalog,
                                step,
                                post,
                            ),
                        ));
                    }
                };
                let (lower, target, target_ranges) = match replace_target(
                    lower,
                    old_target,
                    *target_plan,
                    target_bytes,
                    target_descriptor,
                ) {
                    Ok(replaced) => replaced,
                    Err(failure) => {
                        let _ = failure.retained_owner_count();
                        return Err(submission_failure(
                            M1LongLivedQueueRearmSubmissionPhaseV1::TargetWorkspaceReplacement,
                            (
                                failure,
                                custody,
                                draft,
                                draft_ranges,
                                recipe,
                                catalog,
                                step,
                                post,
                            ),
                        ));
                    }
                };
                let mut catalog_ranges = Vec::new();
                let range_count = draft_ranges.len() + target_ranges.len();
                if catalog_ranges.try_reserve_exact(range_count).is_err() {
                    return Err(submission_failure(
                        M1LongLivedQueueRearmSubmissionPhaseV1::WorkspaceRangeRebinding,
                        (
                            lower,
                            custody,
                            draft,
                            target,
                            draft_ranges,
                            target_ranges,
                            recipe,
                            catalog,
                            step,
                            post,
                        ),
                    ));
                }
                append_workspace_ranges(
                    &mut catalog_ranges,
                    M1FullStepWorkspaceRole::Draft,
                    &draft,
                    draft_ranges,
                );
                append_workspace_ranges(
                    &mut catalog_ranges,
                    M1FullStepWorkspaceRole::Target,
                    &target,
                    target_ranges,
                );
                (
                    lower,
                    M1FullStepWorkspaceSubleaseOwners::speculative_round(draft, target),
                    catalog_ranges,
                )
            }
            (_, _, plans, workspace_images) => {
                return Err(submission_failure(
                    M1LongLivedQueueRearmSubmissionPhaseV1::Preflight,
                    (
                        lower,
                        custody,
                        plans,
                        workspace_images,
                        recipe,
                        catalog,
                        step,
                        post,
                    ),
                ));
            }
        };
    // The stale former workspace witnesses are intentionally dropped only
    // after fresh generic replacement witnesses are retained above.
    custody.workspace_owners = workspace_owners;
    Ok(Box::new(StagedRearmSubmissionV1 {
        shape,
        lower,
        custody,
        workspace_ranges,
        recipe,
        catalog,
        step,
        expected_observation,
        post,
        previous_epoch,
        device,
    }))
}

// This boundary keeps fixed-batch lowering out of the larger preparation frame.
#[inline(never)]
fn finish_staged_rearm_submission(
    staged: Box<StagedRearmSubmissionV1<'_>>,
) -> Result<M1RearmedPublishedQueueV1, M1LongLivedQueueRearmSubmissionFailureV1<'_>> {
    let StagedRearmSubmissionV1 {
        shape,
        lower,
        custody,
        workspace_ranges,
        recipe,
        catalog,
        step,
        expected_observation,
        post,
        previous_epoch,
        device,
    } = *staged;
    let published = match finish_rearm_submission(
        shape,
        lower,
        custody,
        workspace_ranges,
        recipe,
        catalog,
        step,
        expected_observation,
    ) {
        Ok(published) => published,
        Err(failure) => {
            return Err(submission_failure(failure.phase(), (failure, post)));
        }
    };
    Ok(M1RearmedPublishedQueueV1 {
        queue: published,
        carry: M1RearmContinuationCustodyV1 {
            selected: post.selected,
            parked: post.parked,
            terminal: post.terminal,
            previous_epoch,
            prior_checked: post.prior_checked,
            logical_accepted_counts: post.logical_accepted_counts,
            externally_published_counts: post.externally_published_counts,
            release_counts: post.release_counts,
            completed_members: post.completed_members,
            total_released: post.total_released,
            history: post.history,
            rollover: None,
        },
        queue_observation: expected_observation,
        device,
    })
}

/// Replaces request workspaces, binds the fresh fixed batch to the same queue,
/// compares exact queue/device observations, and submits the next generation.
///
/// Model, KV-arena, page-ledger, and coherent completion-output custody are
/// transferred unchanged. Every workspace allocation is replaced through the
/// generic initialized partition replacement API, producing fresh allocation
/// generations, sublease witnesses, and dispatch ranges. Only non-workspace
/// buffers from an exactly equal retained semantic row are reused.
///
/// # Errors
///
/// Returns a closed, phase-tagged failure retaining every queue, allocation,
/// sublease, range, batch, cache, and scheduler owner available at rejection.
fn submit_m1_long_lived_queue_rearm_inner_v1(
    prepared: M1PreparedLongLivedQueueRearmV1,
    recipe: AddresslessM1PhysicalBufferRecipeV1,
    catalog: ContentBoundM1ProgramCatalogV1<'_>,
) -> Result<M1RearmedPublishedQueueV1, M1LongLivedQueueRearmSubmissionFailureV1<'_>> {
    let staged = prepare_rearm_submission(prepared, recipe, catalog)?;
    finish_staged_rearm_submission(staged)
}

/// Replaces workspaces and submits the fresh queue generation, or permanently
/// faults the in-flight Engine while retaining all available closed custody.
///
/// # Errors
///
/// Returns phase-tagged terminal custody after permanently faulting `engine`.
pub fn submit_m1_long_lived_queue_rearm_v1<'a, const C: usize>(
    engine: &mut Engine<C>,
    prepared: M1PreparedLongLivedQueueRearmV1,
    recipe: AddresslessM1PhysicalBufferRecipeV1,
    catalog: ContentBoundM1ProgramCatalogV1<'a>,
) -> Result<M1RearmedPublishedQueueV1, M1LongLivedQueueRearmSubmissionFailureV1<'a>> {
    match submit_m1_long_lived_queue_rearm_inner_v1(prepared, recipe, catalog) {
        Ok(published) => Ok(published),
        Err(failure) => {
            engine.quarantine_m1_queue_rearm_failure();
            Err(failure)
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn rollover_and_submit_finite_speculative<const N: usize, F>(
    lower: ServiceQueueUnboundSessionV1,
    ring_bytes: u32,
    batch: ServiceFixedBatchV1<'_, N>,
    custody: M1PhysicalQueueBatchCustodyV1,
    step: M1PrepublicationStepCustodyV1,
    selected: Vec<ActiveDeviceKvCache>,
    residue: crate::M1FiniteSpeculativeQueueRolloverResidueV1,
    predecessor_generation: u64,
    wrap: F,
) -> Result<
    (
        M1PhysicalPublishedQueueSessionV1,
        Vec<ActiveDeviceKvCache>,
        crate::M1FiniteSpeculativeQueueRolloverResidueV1,
        M1QueueRolloverObservationV1,
    ),
    M1LongLivedQueueRearmSubmissionFailureV1<'_>,
>
where
    F: FnOnce(M1PhysicalQueuePhaseCaseV1<ServiceQueueSessionV1<N>>) -> M1PhysicalQueueSessionV1,
{
    let rollover = match lower.rollover(ring_bytes, batch) {
        Ok(rollover) => rollover,
        Err(failure) => {
            return Err(submission_failure(
                M1LongLivedQueueRearmSubmissionPhaseV1::QueueRollover,
                (failure, custody, step, selected, residue),
            ));
        }
    };
    let rollover_observation = M1QueueRolloverObservationV1 {
        previous_queue_destroyed: rollover.previous_queue_destroyed(),
        previous_dispatch_generation: rollover.previous_dispatch_generation(),
        replacement_queue_observation: rollover.replacement_queue_observation(),
        replacement_dispatch_generation: rollover.replacement_dispatch_generation(),
    };
    let Some(expected_replacement_generation) = predecessor_generation.checked_add(1) else {
        return Err(submission_failure(
            M1LongLivedQueueRearmSubmissionPhaseV1::QueueObservation,
            (
                rollover,
                custody,
                step,
                selected,
                residue,
                rollover_observation,
            ),
        ));
    };
    if rollover_observation.previous_dispatch_generation != predecessor_generation
        || rollover_observation.replacement_dispatch_generation != expected_replacement_generation
    {
        return Err(submission_failure(
            M1LongLivedQueueRearmSubmissionPhaseV1::QueueObservation,
            (
                rollover,
                custody,
                step,
                selected,
                residue,
                rollover_observation,
            ),
        ));
    }
    let queue = wrap(M1PhysicalQueuePhaseCaseV1::from_queue_rearm(
        rollover.into_queue(),
        custody,
        step,
    ));
    let queue = match queue.submit() {
        Ok(published) => published,
        Err(failure) => {
            return Err(submission_failure(
                M1LongLivedQueueRearmSubmissionPhaseV1::QueueSubmit,
                (failure, selected, residue, rollover_observation),
            ));
        }
    };
    Ok((queue, selected, residue, rollover_observation))
}

fn submit_m1_finite_speculative_queue_rollover_inner_v1(
    prepared: M1PreparedFiniteSpeculativeQueueRolloverV1,
    recipe: AddresslessM1PhysicalBufferRecipeV1,
    catalog: ContentBoundM1ProgramCatalogV1<'_>,
    ring_bytes: u32,
) -> Result<M1RearmedPublishedQueueV1, M1LongLivedQueueRearmSubmissionFailureV1<'_>> {
    let (prepared, prior, next, reason, queue, selected, residue) =
        prepared.into_submission_parts();
    let old = queue.custody();
    if queue.shape() != M1PhysicalFixedBatchShapeV1::PairedPrefill
        || prior.shape() != M1PhysicalFixedBatchShapeV1::PairedPrefill
        || !matches!(
            next.shape(),
            M1PhysicalFixedBatchShapeV1::SpeculativeK4
                | M1PhysicalFixedBatchShapeV1::SpeculativeK8
                | M1PhysicalFixedBatchShapeV1::SpeculativeK16
        )
        || reason != crate::M1ServingRolloverReasonV1::Mode
        || old.selection() != prior.target()
        || old.catalog_id() != catalog.catalog_id()
        || old
            .partitioned_memory()
            .finite_speculative_rollover_output_state()
            != M1FiniteSpeculativeRolloverOutputPortfolioStateV1::Reserved
        || prepared.kind() != M1FullStepWorkspaceInputKind::SpeculativeRound
        || prepared.step().kv_reservations().target_selection() != next.target()
        || recipe.workspace_composition().workspace_plans() != prepared.plans()
        || recipe.requires_future_materialization()
        || recipe.rows().len() != next.shape().packet_count()
        || recipe.kernarg_recipe().images().len() != next.shape().packet_count()
        || selected.is_empty()
        || selected
            .iter()
            .any(|cache| cache.projection().device != old.device())
    {
        return Err(submission_failure(
            M1LongLivedQueueRearmSubmissionPhaseV1::Preflight,
            (
                prepared, prior, next, reason, queue, selected, residue, recipe, catalog,
            ),
        ));
    }

    let (old_shape, lower, custody) = queue.into_rearm_parts();
    let predecessor_observation = lower.observation();
    let predecessor_generation = lower.detached_dispatch_generation();
    let device = custody.device();
    let M1PhysicalQueueBatchRearmPartsV1 {
        catalog_id,
        selection: old_selection,
        physical_recipe: old_physical_recipe,
        workspace_composition: old_workspace_composition,
        workspace_owners,
        mut partitioned_memory,
        completion_output: prior_output,
        source_rows: old_source_rows,
        bound_rows: old_bound_rows,
        retired_rollover_custody,
    } = custody.into_rearm_parts();
    let (plans, workspace_images, step) = prepared.into_rearm_parts();
    let (old_draft, old_target, draft_plan, target_plan, draft_bytes, target_bytes) =
        match (workspace_owners, plans, workspace_images) {
            (
                M1FullStepWorkspaceSubleaseOwners::PairedPrefill { draft, target },
                M1FullStepWorkspacePlans::SpeculativeRound {
                    draft_decode,
                    target_speculative,
                },
                M1FullStepWorkspaceImagesV1::SpeculativeRound {
                    draft_decode: draft_bytes,
                    target_speculative: target_bytes,
                },
            ) => (
                draft,
                target,
                draft_decode,
                target_speculative,
                draft_bytes,
                target_bytes,
            ),
            (workspace_owners, plans, workspace_images) => {
                return Err(submission_failure(
                    M1LongLivedQueueRearmSubmissionPhaseV1::Preflight,
                    (
                        (
                            lower,
                            old_shape,
                            catalog_id,
                            old_selection,
                            old_physical_recipe,
                            old_workspace_composition,
                            workspace_owners,
                            partitioned_memory,
                            prior_output,
                        ),
                        (
                            old_source_rows,
                            old_bound_rows,
                            plans,
                            workspace_images,
                            step,
                            selected,
                            residue,
                            recipe,
                            catalog,
                        ),
                    ),
                ));
            }
        };
    let draft_descriptor = match crate::m1_step_workspace_content_descriptor_v1(
        M1InitializedWorkspaceSlotV1::SpeculativeDraftDecode,
        &draft_bytes,
    ) {
        Ok(descriptor) => descriptor,
        Err(error) => {
            return Err(submission_failure(
                M1LongLivedQueueRearmSubmissionPhaseV1::Preflight,
                (
                    (
                        lower,
                        old_shape,
                        catalog_id,
                        old_selection,
                        old_physical_recipe,
                        old_workspace_composition,
                        old_draft,
                        old_target,
                        draft_plan,
                        target_plan,
                        draft_bytes,
                    ),
                    (
                        target_bytes,
                        partitioned_memory,
                        prior_output,
                        old_source_rows,
                        old_bound_rows,
                        step,
                        selected,
                        residue,
                        recipe,
                        catalog,
                        error,
                    ),
                ),
            ));
        }
    };
    let target_descriptor = match crate::m1_step_workspace_content_descriptor_v1(
        M1InitializedWorkspaceSlotV1::SpeculativeTarget,
        &target_bytes,
    ) {
        Ok(descriptor) => descriptor,
        Err(error) => {
            return Err(submission_failure(
                M1LongLivedQueueRearmSubmissionPhaseV1::Preflight,
                (
                    (
                        lower,
                        old_shape,
                        catalog_id,
                        old_selection,
                        old_physical_recipe,
                        old_workspace_composition,
                        old_draft,
                        old_target,
                        draft_plan,
                        target_plan,
                        draft_bytes,
                    ),
                    (
                        target_bytes,
                        partitioned_memory,
                        prior_output,
                        old_source_rows,
                        old_bound_rows,
                        step,
                        selected,
                        residue,
                        recipe,
                        catalog,
                        error,
                    ),
                ),
            ));
        }
    };
    let (lower, draft, draft_ranges) = match replace_rollover_workspace(
        lower,
        &old_draft,
        *draft_plan,
        draft_bytes,
        draft_descriptor,
    ) {
        Ok(replaced) => replaced,
        Err(failure) => {
            return Err(submission_failure(
                M1LongLivedQueueRearmSubmissionPhaseV1::DraftWorkspaceReplacement,
                (
                    (
                        failure,
                        old_shape,
                        catalog_id,
                        old_selection,
                        old_physical_recipe,
                        old_workspace_composition,
                        old_target,
                        target_plan,
                        target_bytes,
                    ),
                    (
                        partitioned_memory,
                        prior_output,
                        old_source_rows,
                        old_bound_rows,
                        step,
                        selected,
                        residue,
                        recipe,
                        catalog,
                    ),
                ),
            ));
        }
    };
    let (lower, target, target_ranges) = match replace_rollover_workspace(
        lower,
        &old_target,
        *target_plan,
        target_bytes,
        target_descriptor,
    ) {
        Ok(replaced) => replaced,
        Err(failure) => {
            return Err(submission_failure(
                M1LongLivedQueueRearmSubmissionPhaseV1::TargetWorkspaceReplacement,
                (
                    (
                        failure,
                        old_shape,
                        catalog_id,
                        old_selection,
                        old_physical_recipe,
                        old_workspace_composition,
                        draft,
                        draft_ranges,
                    ),
                    (
                        partitioned_memory,
                        prior_output,
                        old_source_rows,
                        old_bound_rows,
                        step,
                        selected,
                        residue,
                        recipe,
                        catalog,
                    ),
                ),
            ));
        }
    };
    let mut workspace_ranges = Vec::new();
    if workspace_ranges
        .try_reserve_exact(draft_ranges.len() + target_ranges.len())
        .is_err()
    {
        return Err(submission_failure(
            M1LongLivedQueueRearmSubmissionPhaseV1::WorkspaceRangeRebinding,
            (
                (
                    lower,
                    old_shape,
                    catalog_id,
                    old_selection,
                    old_physical_recipe,
                    old_workspace_composition,
                    draft,
                    target,
                    draft_ranges,
                    target_ranges,
                ),
                (
                    partitioned_memory,
                    prior_output,
                    old_source_rows,
                    old_bound_rows,
                    step,
                    selected,
                    residue,
                    recipe,
                    catalog,
                ),
            ),
        ));
    }
    append_workspace_ranges(
        &mut workspace_ranges,
        M1FullStepWorkspaceRole::Draft,
        &draft,
        draft_ranges,
    );
    append_workspace_ranges(
        &mut workspace_ranges,
        M1FullStepWorkspaceRole::Target,
        &target,
        target_ranges,
    );
    let completion_output = match partitioned_memory
        .activate_finite_speculative_rollover_output(next.target(), prior_output)
    {
        Ok(output) => output,
        Err(failure) => {
            return Err(submission_failure(
                M1LongLivedQueueRearmSubmissionPhaseV1::RolloverOutputActivation,
                (
                    (
                        lower,
                        old_shape,
                        catalog_id,
                        old_selection,
                        old_physical_recipe,
                        old_workspace_composition,
                        draft,
                        target,
                        workspace_ranges,
                    ),
                    (
                        partitioned_memory,
                        failure,
                        old_source_rows,
                        old_bound_rows,
                        step,
                        selected,
                        residue,
                        recipe,
                        catalog,
                    ),
                ),
            ));
        }
    };
    let retained_capture = match retained_host_capture_ranges(&completion_output) {
        Ok(capture) => capture,
        Err(()) => {
            return Err(submission_failure(
                M1LongLivedQueueRearmSubmissionPhaseV1::WorkspaceRangeRebinding,
                (
                    (
                        lower,
                        old_shape,
                        catalog_id,
                        old_selection,
                        old_physical_recipe,
                        old_workspace_composition,
                        draft,
                        target,
                        workspace_ranges,
                    ),
                    (
                        partitioned_memory,
                        completion_output,
                        old_source_rows,
                        old_bound_rows,
                        step,
                        selected,
                        residue,
                        recipe,
                        catalog,
                    ),
                ),
            ));
        }
    };
    let bound_rows = match build_rollover_bound_rows(
        recipe.rows(),
        &old_source_rows,
        &old_bound_rows,
        recipe.workspace_composition(),
        &workspace_ranges,
        &retained_capture,
    ) {
        Ok(rows) => rows,
        Err(()) => {
            return Err(submission_failure(
                M1LongLivedQueueRearmSubmissionPhaseV1::WorkspaceRangeRebinding,
                (
                    (
                        lower,
                        old_shape,
                        catalog_id,
                        old_selection,
                        old_physical_recipe,
                        old_workspace_composition,
                        draft,
                        target,
                        workspace_ranges,
                    ),
                    (
                        partitioned_memory,
                        completion_output,
                        old_source_rows,
                        old_bound_rows,
                        step,
                        selected,
                        residue,
                        recipe,
                        catalog,
                    ),
                ),
            ));
        }
    };
    let (kernargs, workspace_composition, source_rows) = recipe.into_parts();
    let (physical_recipe, images) = kernargs.into_parts();
    let successor_shape = next.shape();
    let batch = match successor_shape {
        M1PhysicalFixedBatchShapeV1::SpeculativeK4 => {
            match lower_boxed_rearm_batch(catalog, &physical_recipe, images, &bound_rows) {
                Ok(batch) => RebuiltBatchV1::SpeculativeK4(batch),
                Err(failure) => {
                    return Err(submission_failure(
                        M1LongLivedQueueRearmSubmissionPhaseV1::FixedBatchRebuild,
                        (
                            (
                                lower,
                                old_shape,
                                catalog_id,
                                old_selection,
                                old_physical_recipe,
                                old_workspace_composition,
                                draft,
                                target,
                                partitioned_memory,
                                completion_output,
                            ),
                            (
                                old_source_rows,
                                old_bound_rows,
                                physical_recipe,
                                source_rows,
                                bound_rows,
                                failure.catalog,
                                failure.images,
                                step,
                                selected,
                                residue,
                            ),
                        ),
                    ));
                }
            }
        }
        M1PhysicalFixedBatchShapeV1::SpeculativeK8 => {
            match lower_boxed_rearm_batch(catalog, &physical_recipe, images, &bound_rows) {
                Ok(batch) => RebuiltBatchV1::SpeculativeK8(batch),
                Err(failure) => {
                    return Err(submission_failure(
                        M1LongLivedQueueRearmSubmissionPhaseV1::FixedBatchRebuild,
                        (
                            (
                                lower,
                                old_shape,
                                catalog_id,
                                old_selection,
                                old_physical_recipe,
                                old_workspace_composition,
                                draft,
                                target,
                                partitioned_memory,
                                completion_output,
                            ),
                            (
                                old_source_rows,
                                old_bound_rows,
                                physical_recipe,
                                source_rows,
                                bound_rows,
                                failure.catalog,
                                failure.images,
                                step,
                                selected,
                                residue,
                            ),
                        ),
                    ));
                }
            }
        }
        M1PhysicalFixedBatchShapeV1::SpeculativeK16 => {
            match lower_boxed_rearm_batch(catalog, &physical_recipe, images, &bound_rows) {
                Ok(batch) => RebuiltBatchV1::SpeculativeK16(batch),
                Err(failure) => {
                    return Err(submission_failure(
                        M1LongLivedQueueRearmSubmissionPhaseV1::FixedBatchRebuild,
                        (
                            (
                                lower,
                                old_shape,
                                catalog_id,
                                old_selection,
                                old_physical_recipe,
                                old_workspace_composition,
                                draft,
                                target,
                                partitioned_memory,
                                completion_output,
                            ),
                            (
                                old_source_rows,
                                old_bound_rows,
                                physical_recipe,
                                source_rows,
                                bound_rows,
                                failure.catalog,
                                failure.images,
                                step,
                                selected,
                                residue,
                            ),
                        ),
                    ));
                }
            }
        }
        M1PhysicalFixedBatchShapeV1::TargetOnly | M1PhysicalFixedBatchShapeV1::PairedPrefill => {
            unreachable!("finite-speculative successor shape was preflighted")
        }
    };
    let custody =
        M1PhysicalQueueBatchCustodyV1::from_rearm_parts(M1PhysicalQueueBatchRearmPartsV1 {
            catalog_id,
            selection: next.target(),
            physical_recipe,
            workspace_composition,
            workspace_owners: M1FullStepWorkspaceSubleaseOwners::speculative_round(draft, target),
            partitioned_memory,
            completion_output,
            source_rows,
            bound_rows,
            retired_rollover_custody,
        });
    let (queue, selected, residue, rollover_observation) = match batch {
        RebuiltBatchV1::SpeculativeK4(batch) => rollover_and_submit_finite_speculative(
            lower,
            ring_bytes,
            *batch,
            custody,
            step,
            selected,
            residue,
            predecessor_generation,
            |case| M1PhysicalQueueSessionV1::SpeculativeK4(Box::new(case)),
        )?,
        RebuiltBatchV1::SpeculativeK8(batch) => rollover_and_submit_finite_speculative(
            lower,
            ring_bytes,
            *batch,
            custody,
            step,
            selected,
            residue,
            predecessor_generation,
            |case| M1PhysicalQueueSessionV1::SpeculativeK8(Box::new(case)),
        )?,
        RebuiltBatchV1::SpeculativeK16(batch) => rollover_and_submit_finite_speculative(
            lower,
            ring_bytes,
            *batch,
            custody,
            step,
            selected,
            residue,
            predecessor_generation,
            |case| M1PhysicalQueueSessionV1::SpeculativeK16(Box::new(case)),
        )?,
        RebuiltBatchV1::TargetOnly(_) => {
            unreachable!("finite-speculative rollover cannot build target-only")
        }
    };
    let residue = residue.into_parts();
    let prior_checked = residue.checked;
    let previous_epoch = prior_checked.epoch();
    Ok(M1RearmedPublishedQueueV1 {
        queue,
        carry: M1RearmContinuationCustodyV1 {
            selected,
            parked: Vec::new(),
            terminal: Vec::new(),
            previous_epoch,
            prior_checked,
            logical_accepted_counts: residue.logical_accepted_counts,
            externally_published_counts: residue.externally_published_counts,
            release_counts: residue.release_counts,
            completed_members: residue.completed_members,
            total_released: residue.total_released,
            history: M1RearmRoundHistoryV1::Empty,
            rollover: Some(rollover_observation),
        },
        queue_observation: predecessor_observation,
        device,
    })
}

/// Replaces an S1 paired-prefill queue with a native S1/K4 generation.
///
/// # Errors
///
/// Returns phase-tagged terminal custody and permanently faults `engine`.
pub fn submit_m1_s1_k4_queue_rollover_v1<'a, const C: usize>(
    engine: &mut Engine<C>,
    prepared: M1PreparedS1K4QueueRolloverV1,
    recipe: AddresslessM1PhysicalBufferRecipeV1,
    catalog: ContentBoundM1ProgramCatalogV1<'a>,
    ring_bytes: u32,
) -> Result<M1RearmedPublishedQueueV1, M1LongLivedQueueRearmSubmissionFailureV1<'a>> {
    let exact = prepared.next_plan().target().bucket
        == ferric_spec::Qwen3PlanBucket::SpeculativeS1K4C8192
        && prepared.next_plan().shape() == M1PhysicalFixedBatchShapeV1::SpeculativeK4
        && prepared.next_plan().sequence_capacity() == 1;
    if !exact {
        engine.quarantine_m1_queue_rearm_failure();
        return Err(submission_failure(
            M1LongLivedQueueRearmSubmissionPhaseV1::Preflight,
            (prepared, recipe, catalog),
        ));
    }
    submit_m1_finite_speculative_queue_rollover_v1(engine, prepared, recipe, catalog, ring_bytes)
}

/// Replaces paired-prefill with one finite-speculative native queue.
///
/// # Errors
///
/// Returns phase-tagged terminal custody and permanently faults `engine` when
/// replacement construction, queue rollover, or submission rejects.
pub fn submit_m1_finite_speculative_queue_rollover_v1<'a, const C: usize>(
    engine: &mut Engine<C>,
    prepared: M1PreparedFiniteSpeculativeQueueRolloverV1,
    recipe: AddresslessM1PhysicalBufferRecipeV1,
    catalog: ContentBoundM1ProgramCatalogV1<'a>,
    ring_bytes: u32,
) -> Result<M1RearmedPublishedQueueV1, M1LongLivedQueueRearmSubmissionFailureV1<'a>> {
    match submit_m1_finite_speculative_queue_rollover_inner_v1(
        prepared, recipe, catalog, ring_bytes,
    ) {
        Ok(published) => Ok(published),
        Err(failure) => {
            engine.quarantine_m1_queue_rearm_failure();
            Err(failure)
        }
    }
}

impl M1RearmedPublishedQueueV1 {
    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.carry.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.carry.history.get(index)
    }
}

impl M1RearmedQueueProgressFailureV1 {
    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.carry.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.carry.history.get(index)
    }
}

impl M1RearmedCompletedQueueV1 {
    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.carry.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.carry.history.get(index)
    }
}

impl M1RearmedRecycledQueueV1 {
    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.carry.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.carry.history.get(index)
    }
}

impl M1RearmedObservedCompletionOutputV1 {
    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.carry.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.carry.history.get(index)
    }
}

impl M1RearmedQualificationObservationFailureV1 {
    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.carry.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.carry.history.get(index)
    }
}

impl M1RearmedQualificationObservationTeardownSuccessV1 {
    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.carry.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.carry.history.get(index)
    }
}

impl M1RearmedQualificationObservationTeardownFailureV1 {
    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.carry.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.carry.history.get(index)
    }
}

impl M1RearmedObservedQualificationOutputV1 {
    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.carry.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.carry.history.get(index)
    }
}

impl M1RearmedQualificationSemanticTeardownSuccessV1 {
    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.carry.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.carry.history.get(index)
    }
}

impl M1RearmedQualificationSemanticTeardownFailureV1 {
    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.carry.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.carry.history.get(index)
    }
}

impl M1RearmedReadbackFailureV1 {
    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.carry.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.carry.history.get(index)
    }
}

impl M1RearmedReadbackTeardownSuccessV1 {
    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.carry.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.carry.history.get(index)
    }
}

impl M1RearmedReadbackTeardownFailureV1 {
    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.carry.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.carry.history.get(index)
    }
}

impl M1RearmedCompletedReadbackV1 {
    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.carry.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&M1RearmRoundHistoryEntryV1> {
        self.carry.history.get(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device_cache::test_support::bind_gfx942_device;
    use ferric_spec::{Identity, Qwen3ModelRole, Qwen3PlanBucket};

    const fn selection(mode: Qwen3ExecutionMode, bucket: Qwen3PlanBucket) -> Qwen3PlanSelection {
        Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode,
            bucket,
        }
    }

    fn test_device() -> Gfx942DeviceBinding {
        bind_gfx942_device(
            Identity::new([91; 32]),
            7,
            crate::GFX942_PROCESSOR,
            crate::GFX942_TARGET_FEATURES,
        )
        .unwrap()
    }

    const fn qualification_capture_ranges(
        completion_output: u64,
        logits: u64,
    ) -> RetainedCaptureRangesV1<u64> {
        RetainedCaptureRangesV1 {
            completion_output,
            semantic: RetainedSemanticCaptureRangesV1::Qualification { logits },
        }
    }

    const fn direct_capture_ranges(
        completion_output: u64,
        choices: u64,
    ) -> RetainedCaptureRangesV1<u64> {
        RetainedCaptureRangesV1 {
            completion_output,
            semantic: RetainedSemanticCaptureRangesV1::DirectDiagnostic { choices },
        }
    }

    fn speculative_capture_ranges(
        completion_output: u64,
        draft: u64,
        draft_tokens: u8,
        draft_row_base: u64,
        target: u64,
    ) -> RetainedCaptureRangesV1<u64> {
        let mut draft_rows = [None; crate::M1_SPECULATIVE_DIAGNOSTIC_MAX_DRAFT_TOKENS_V1];
        for (index, row) in draft_rows
            .iter_mut()
            .take(usize::from(draft_tokens))
            .enumerate()
        {
            *row =
                Some(draft_row_base + u64::try_from(index).expect("bounded diagnostic test row"));
        }
        RetainedCaptureRangesV1 {
            completion_output,
            semantic: RetainedSemanticCaptureRangesV1::SpeculativeDiagnostic {
                draft,
                draft_tokens,
                draft_rows,
                target,
            },
        }
    }

    fn test_cache(request: RequestId, device: Gfx942DeviceBinding) -> ActiveDeviceKvCache {
        let bucket = Qwen3PlanBucket::DecodeS1C8192;
        ActiveDeviceKvCache::new(
            device,
            request,
            selection(Qwen3ExecutionMode::Decode, bucket),
            Qwen3PlanSelection {
                role: Qwen3ModelRole::Draft06B,
                mode: Qwen3ExecutionMode::Decode,
                bucket,
            },
        )
        .unwrap()
    }

    fn test_rearm_carry(selected: RequestId, parked: RequestId) -> M1RearmContinuationCustodyV1 {
        let device = test_device();
        let previous_epoch = CompletionEpoch::new(7);
        M1RearmContinuationCustodyV1 {
            selected: vec![test_cache(selected, device)],
            parked: vec![test_cache(parked, device)],
            terminal: Vec::new(),
            previous_epoch,
            prior_checked: crate::M1CheckedCompletionOutputV1::empty_for_rearm_test(
                selection(Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS1C8192),
                previous_epoch,
            ),
            logical_accepted_counts: Box::new([1]),
            externally_published_counts: Box::new([0]),
            release_counts: Box::new([]),
            completed_members: 0,
            total_released: 0,
            history: M1RearmRoundHistoryV1::Empty,
            rollover: None,
        }
    }

    #[test]
    fn qualification_capture_rearm_rejects_generic_direct_final_semantics() {
        let direct = [crate::CompletionWireSemanticExpectation::DirectFinalRow { choice: 41 }];
        assert_eq!(
            validate_rearmed_generic_semantics(true, false, false, &direct),
            Err(M1RearmedGenericSemanticGateV1::Qualification { lane: 0 })
        );
        assert_eq!(
            validate_rearmed_generic_semantics(false, true, false, &direct),
            Err(M1RearmedGenericSemanticGateV1::DirectDiagnostic)
        );
        assert_eq!(
            validate_rearmed_generic_semantics(false, false, true, &direct),
            Err(M1RearmedGenericSemanticGateV1::SpeculativeDiagnostic)
        );
        assert_eq!(
            validate_rearmed_generic_semantics(false, false, false, &direct),
            Ok(())
        );
    }

    #[test]
    fn qualification_context_reservation_rejects_a_queue_without_capture_custody() {
        let selection = selection(Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS1C8192);
        assert!(!qualification_logits_preflight(selection, None));
    }

    #[test]
    fn repeated_slice_excludes_prefill_and_shape_transitions() {
        assert_eq!(
            validate_rearm_eligibility(
                M1PhysicalFixedBatchShapeV1::PairedPrefill,
                selection(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128),
                false,
            ),
            Err(M1LongLivedQueueRearmScheduleErrorV1::UnsupportedPriorShape)
        );
        assert_eq!(
            validate_rearm_eligibility(
                M1PhysicalFixedBatchShapeV1::TargetOnly,
                selection(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128),
                false,
            ),
            Err(M1LongLivedQueueRearmScheduleErrorV1::UnsupportedPriorShape)
        );
        assert_eq!(
            validate_rearm_eligibility(
                M1PhysicalFixedBatchShapeV1::TargetOnly,
                selection(Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS1C8192),
                false,
            ),
            Ok(())
        );
        assert_eq!(
            validate_rearm_eligibility(
                M1PhysicalFixedBatchShapeV1::SpeculativeK4,
                selection(
                    Qwen3ExecutionMode::Speculative,
                    Qwen3PlanBucket::SpeculativeS1K4C8192,
                ),
                false,
            ),
            Ok(())
        );
    }

    #[test]
    fn qualification_bearing_target_decode_alone_is_admitted_for_terminal_rearm() {
        let target_decode = selection(Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS8C8192);
        assert_eq!(
            validate_rearm_eligibility(
                M1PhysicalFixedBatchShapeV1::TargetOnly,
                target_decode,
                false,
            ),
            Ok(())
        );
        assert_eq!(
            validate_rearm_eligibility(
                M1PhysicalFixedBatchShapeV1::TargetOnly,
                target_decode,
                true,
            ),
            Ok(())
        );
        assert_eq!(
            validate_rearm_eligibility(
                M1PhysicalFixedBatchShapeV1::TargetOnly,
                selection(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128),
                true,
            ),
            Err(M1LongLivedQueueRearmScheduleErrorV1::UnsupportedPriorShape)
        );
        assert_eq!(
            validate_rearm_eligibility(
                M1PhysicalFixedBatchShapeV1::SpeculativeK4,
                selection(
                    Qwen3ExecutionMode::Speculative,
                    Qwen3PlanBucket::SpeculativeS1K4C8192,
                ),
                true,
            ),
            Err(M1LongLivedQueueRearmScheduleErrorV1::UnsupportedPriorShape)
        );
    }

    #[test]
    fn final_qualification_completion_builds_only_retiring_dispositions() {
        let dispositions = retiring_dispositions(32).unwrap();
        assert_eq!(dispositions.len(), 32);
        assert!(dispositions
            .iter()
            .all(|disposition| *disposition == crate::M1DeviceKvCompletionDispositionV1::Retire));
        assert!(retiring_dispositions(0).unwrap().is_empty());
    }

    #[test]
    fn all_terminal_rearm_shutdown_rejects_every_parked_owner() {
        assert_eq!(preflight_all_terminal_rearm_shutdown(0), Ok(()));
        for count in [1, 2, usize::try_from(M1_MAX_ACTIVE_SEQUENCES).unwrap()] {
            assert_eq!(
                preflight_all_terminal_rearm_shutdown(count),
                Err(crate::M1AllTerminalQueueShutdownErrorV1::ParkedMembers { count })
            );
        }
    }

    #[test]
    fn all_terminal_shutdown_apis_are_linear_and_source_ordered() {
        fn assert_nameable<T>() {}

        type GenericShutdown = fn(
            crate::M1LongLivedQueueReleasedRoundV1,
            &mut Engine<32>,
        ) -> Result<
            crate::M1LongLivedQueueAllTerminalShutdownSuccessV1,
            crate::M1LongLivedQueueAllTerminalShutdownFailureV1,
        >;
        type AuthenticatedShutdown = fn(
            crate::M1AuthenticatedLongLivedQueueReleasedRoundV1,
            &mut Engine<32>,
        ) -> Result<
            crate::M1AuthenticatedLongLivedQueueAllTerminalShutdownSuccessV1,
            crate::M1AuthenticatedLongLivedQueueAllTerminalShutdownFailureV1,
        >;

        assert_nameable::<crate::M1AllTerminalQueueShutdownErrorV1>();
        assert_nameable::<crate::M1ReleasedAllTerminalQueueShutdownSuccessV1>();
        assert_nameable::<crate::M1ReleasedAllTerminalQueueShutdownRejectionV1>();
        assert_nameable::<crate::M1ReleasedAllTerminalQueueShutdownFailureV1>();
        assert_nameable::<crate::M1AuthenticatedReleasedAllTerminalQueueShutdownSuccessV1>();
        assert_nameable::<crate::M1AuthenticatedReleasedAllTerminalQueueShutdownRejectionV1>();
        assert_nameable::<crate::M1AuthenticatedReleasedAllTerminalQueueShutdownFailureV1>();
        assert_nameable::<crate::M1LongLivedQueueAllTerminalShutdownSuccessV1>();
        assert_nameable::<crate::M1LongLivedQueueAllTerminalShutdownRejectionV1>();
        assert_nameable::<crate::M1LongLivedQueueAllTerminalShutdownFailureV1>();
        assert_nameable::<crate::M1AuthenticatedLongLivedQueueAllTerminalShutdownSuccessV1>();
        assert_nameable::<crate::M1AuthenticatedLongLivedQueueAllTerminalShutdownRejectionV1>();
        assert_nameable::<crate::M1AuthenticatedLongLivedQueueAllTerminalShutdownFailureV1>();

        let _: GenericShutdown =
            crate::M1LongLivedQueueReleasedRoundV1::shutdown_all_terminal_queue::<32>;
        let _: AuthenticatedShutdown =
            crate::M1AuthenticatedLongLivedQueueReleasedRoundV1::shutdown_all_terminal_queue::<32>;

        for source in [
            include_str!("m1_queue_rearm.rs"),
            include_str!("authenticated_queue_rearm.rs"),
        ] {
            let preflight = source
                .find("preflight_all_terminal_rearm_shutdown(self.parked.len())")
                .unwrap();
            let destructure = source[preflight..]
                .find("let Self {")
                .map(|offset| preflight + offset)
                .unwrap();
            let lower = source[destructure..]
                .find("released.shutdown_all_terminal_queue(engine)")
                .map(|offset| destructure + offset)
                .unwrap();
            assert!(preflight < destructure);
            assert!(destructure < lower);
            assert!(source.contains("destroy_queue_and_retain_round"));
        }
    }

    #[test]
    fn qualification_observation_retry_rejoins_exact_carry_on_both_outcomes() {
        let selected = RequestId::new(0, 3);
        let parked = RequestId::new(1, 5);
        let success = rejoin_qualification_observation_retry(
            Ok::<u8, u16>(41),
            test_rearm_carry(selected, parked),
            73_u32,
            test_device(),
        )
        .unwrap();
        assert_eq!(success.source, 41);
        assert_eq!(success.queue_observation, 73);
        assert_eq!(success.carry.selected[0].projection().request, selected);
        assert_eq!(success.carry.parked[0].projection().request, parked);
        assert_eq!(success.carry.previous_epoch, CompletionEpoch::new(7));
        assert_eq!(&*success.carry.logical_accepted_counts, &[1]);
        assert_eq!(&*success.carry.externally_published_counts, &[0]);
        assert_eq!(success.device, test_device());

        let failure = rejoin_qualification_observation_retry(
            Err::<u8, u16>(97),
            test_rearm_carry(selected, parked),
            89_u32,
            test_device(),
        )
        .unwrap_err();
        assert_eq!(failure.source, 97);
        assert_eq!(failure.queue_observation, 89);
        assert_eq!(failure.carry.selected[0].projection().request, selected);
        assert_eq!(failure.carry.parked[0].projection().request, parked);
        assert_eq!(failure.carry.previous_epoch, CompletionEpoch::new(7));
        assert_eq!(&*failure.carry.logical_accepted_counts, &[1]);
        assert_eq!(&*failure.carry.externally_published_counts, &[0]);
        assert_eq!(failure.device, test_device());
    }

    #[test]
    fn terminal_lineage_and_qualification_evidence_rejoin_exact_owners() {
        let parked = vec![RequestId::new(1, 5)];
        let terminal = vec![Identity::new([95; 32])];
        let history = vec![Identity::new([96; 32])];

        let success_source = Box::new(Identity::new([97; 32]));
        let success_pointer = core::ptr::from_ref(success_source.as_ref());
        let success = join_terminal_lineage(
            Ok::<_, Box<Identity>>(success_source),
            parked.clone(),
            terminal.clone(),
            history.clone(),
        )
        .unwrap();
        assert_eq!(
            core::ptr::from_ref(success.source.as_ref()),
            success_pointer
        );
        assert_eq!(success.parked, parked);
        assert_eq!(success.terminal, terminal);
        assert_eq!(success.history, history);

        let evidence = Box::new(Identity::new([100; 32]));
        let evidence_pointer = core::ptr::from_ref(evidence.as_ref());
        let source_pointer = core::ptr::from_ref(success.source.as_ref());
        let qualified_success = join_qualification_evidence(
            Ok::<
                _,
                TerminalLineageJoinV1<Box<Identity>, Vec<RequestId>, Vec<Identity>, Vec<Identity>>,
            >(success),
            evidence,
        )
        .unwrap();
        assert_eq!(
            core::ptr::from_ref(qualified_success.source.source.as_ref()),
            source_pointer
        );
        assert_eq!(
            core::ptr::from_ref(qualified_success.evidence.as_ref()),
            evidence_pointer
        );
        assert_eq!(qualified_success.source.parked, parked);
        assert_eq!(qualified_success.source.terminal, terminal);
        assert_eq!(qualified_success.source.history, history);

        let failure_source = Box::new(Identity::new([98; 32]));
        let failure_pointer = core::ptr::from_ref(failure_source.as_ref());
        let failure = join_terminal_lineage(
            Err::<Box<Identity>, _>(failure_source),
            parked.clone(),
            terminal.clone(),
            history.clone(),
        )
        .unwrap_err();
        assert_eq!(
            core::ptr::from_ref(failure.source.as_ref()),
            failure_pointer
        );
        assert_eq!(failure.parked, parked);
        assert_eq!(failure.terminal, terminal);
        assert_eq!(failure.history, history);

        let evidence = Box::new(Identity::new([99; 32]));
        let evidence_pointer = core::ptr::from_ref(evidence.as_ref());
        let joined = join_qualification_evidence(
            Err::<
                TerminalLineageJoinV1<Box<Identity>, Vec<RequestId>, Vec<Identity>, Vec<Identity>>,
                _,
            >(failure),
            evidence,
        )
        .unwrap_err();
        assert_eq!(
            core::ptr::from_ref(joined.source.source.as_ref()),
            failure_pointer
        );
        assert_eq!(
            core::ptr::from_ref(joined.evidence.as_ref()),
            evidence_pointer
        );
        assert_eq!(joined.source.parked, parked);
        assert_eq!(joined.source.terminal, terminal);
        assert_eq!(joined.source.history, history);
    }

    #[test]
    fn qualification_prompt_count_history_keeps_logical_and_external_distinct() {
        let carry = test_rearm_carry(RequestId::new(0, 3), RequestId::new(1, 5));
        assert_eq!(&*carry.logical_accepted_counts, &[1]);
        assert_eq!(&*carry.externally_published_counts, &[0]);
    }

    #[test]
    fn scheduling_round_c_retains_round_a_and_b_history_before_appending_c() {
        let mut history = M1RearmRoundHistoryV1::<u64>::Empty;
        for round in [11, 22] {
            history.try_reserve_append().unwrap();
            history = M1RearmRoundHistoryV1::NonEmpty(history.append(round));
        }

        let mut scheduled_round_c = history;
        assert_eq!(scheduled_round_c.len(), 2);
        assert_eq!(scheduled_round_c.get(0), Some(&11));
        assert_eq!(scheduled_round_c.get(1), Some(&22));

        scheduled_round_c.try_reserve_append().unwrap();
        let completed_round_c = M1RearmRoundHistoryV1::NonEmpty(scheduled_round_c.append(33));
        assert_eq!(completed_round_c.len(), 3);
        assert_eq!(completed_round_c.get(2), Some(&33));
    }

    #[test]
    fn rearm_round_history_is_bounded_at_8192_without_rebuilding_prior_entries() {
        let mut history = M1RearmRoundHistoryV1::<usize>::Empty;
        for round in 0..M1_MAX_REARM_ROUND_HISTORY_V1 {
            history.try_reserve_append().unwrap();
            history = M1RearmRoundHistoryV1::NonEmpty(history.append(round));
        }

        assert_eq!(history.len(), M1_MAX_REARM_ROUND_HISTORY_V1);
        assert_eq!(history.get(0), Some(&0));
        assert_eq!(
            history.get(M1_MAX_REARM_ROUND_HISTORY_V1 - 1),
            Some(&(M1_MAX_REARM_ROUND_HISTORY_V1 - 1))
        );
        assert_eq!(
            history.try_reserve_append(),
            Err(M1RearmedCompletionPreflightErrorV1::RoundHistoryCapacity {
                maximum: M1_MAX_REARM_ROUND_HISTORY_V1,
            })
        );
        assert_eq!(history.len(), M1_MAX_REARM_ROUND_HISTORY_V1);
    }

    #[test]
    fn saturated_round_history_rejects_scheduling_before_consuming_teardown_custody() {
        type TeardownResult = Result<
            M1LongLivedQueueRearmTeardownSuccessV1,
            Box<M1LongLivedQueueRearmTeardownFailureV1>,
        >;
        fn recover_and_destroy(
            failure: M1LongLivedQueueRearmScheduleFailureV1,
            engine: &mut Engine<32>,
        ) -> Result<TeardownResult, M1LongLivedQueueRearmScheduleFailureV1> {
            failure
                .into_unscheduled()
                .map(|unscheduled| unscheduled.destroy_queue_and_retain_round(engine))
        }

        let mut history = M1RearmRoundHistoryV1::<usize>::Empty;
        for round in 0..M1_MAX_REARM_ROUND_HISTORY_V1 {
            history.try_reserve_append().unwrap();
            history = M1RearmRoundHistoryV1::NonEmpty(history.append(round));
        }

        assert_eq!(
            validate_rearm_round_history_schedule_capacity(&history),
            Err(M1LongLivedQueueRearmScheduleErrorV1::RoundHistoryCapacity {
                maximum: M1_MAX_REARM_ROUND_HISTORY_V1,
            })
        );
        assert_eq!(history.len(), M1_MAX_REARM_ROUND_HISTORY_V1);

        let _: fn(
            M1LongLivedQueueRearmScheduleFailureV1,
            &mut Engine<32>,
        ) -> Result<TeardownResult, M1LongLivedQueueRearmScheduleFailureV1> = recover_and_destroy;
    }

    #[test]
    fn round_history_api_is_nameable_from_the_crate_root_across_failure_owners() {
        type ReleasedHistory = for<'a> fn(
            &'a crate::M1LongLivedQueueReleasedRoundV1,
            usize,
        ) -> Option<&'a crate::M1RearmRoundHistoryEntryV1>;
        type PreflightHistory = for<'a> fn(
            &'a M1RearmedCompletionPreflightFailureV1,
            usize,
        )
            -> Option<&'a crate::M1RearmRoundHistoryEntryV1>;
        type QualifiedReadbackHistory = for<'a> fn(
            &'a M1RearmedQualifiedCompletedReadbackV1,
            usize,
        )
            -> Option<&'a crate::M1RearmRoundHistoryEntryV1>;
        type ObservedHistory = for<'a> fn(
            &'a crate::M1RearmedObservedCompletionOutputV1,
            usize,
        ) -> Option<&'a crate::M1RearmRoundHistoryEntryV1>;
        type QualifiedPreflightHistory =
            for<'a> fn(
                &'a M1RearmedQualifiedCompletionPreflightFailureV1,
                usize,
            ) -> Option<&'a crate::M1RearmRoundHistoryEntryV1>;
        type QualifiedTeardownFailureHistory =
            for<'a> fn(
                &'a M1RearmedQualifiedTeardownFailureV1,
                usize,
            ) -> Option<&'a crate::M1RearmRoundHistoryEntryV1>;

        let _: usize = crate::M1_MAX_REARM_ROUND_HISTORY_V1;
        let _: ReleasedHistory = crate::M1LongLivedQueueReleasedRoundV1::round_history;
        let _: PreflightHistory = M1RearmedCompletionPreflightFailureV1::round_history;
        let _: ObservedHistory = crate::M1RearmedObservedCompletionOutputV1::round_history;
        let _: QualifiedReadbackHistory = M1RearmedQualifiedCompletedReadbackV1::round_history;
        let _: QualifiedPreflightHistory =
            M1RearmedQualifiedCompletionPreflightFailureV1::round_history;
        let _: QualifiedTeardownFailureHistory = M1RearmedQualifiedTeardownFailureV1::round_history;
    }

    #[test]
    fn qualification_capture_ranges_survive_two_fresh_workspace_generations() {
        use ferric_build::M1StepWorkspaceRangeRole;

        let fresh_request = RearmRangeRequestV1::FreshWorkspace(
            M1FullStepWorkspaceRole::Target,
            M1StepWorkspaceRange::new(M1StepWorkspaceRangeRole::TokenIds, 0, 8, 8),
        );
        let requests = [
            RearmRangeRequestV1::RetainedCompletionOutput,
            RearmRangeRequestV1::RetainedQualificationLogits,
            RearmRangeRequestV1::RetainedQualificationLogits,
            fresh_request,
        ];
        let retained_compact = 101_u64;
        let retained_logits = 202_u64;
        let retained = qualification_capture_ranges(retained_compact, retained_logits);
        let generation_zero = [retained_compact, retained_logits, retained_logits, 303];
        let generation_one_fresh = [None, None, None, Some(404)];
        let mut generation_one_selection = RearmRangeSelectionV1::new(&retained.semantic);
        let generation_one = core::array::from_fn(|index| {
            generation_one_selection
                .select(
                    requests[index],
                    generation_zero[index],
                    generation_one_fresh[index],
                    retained,
                    retained,
                )
                .unwrap()
        });
        assert_eq!(generation_one_selection.validate(), Ok(()));
        assert_eq!(
            generation_one,
            [retained_compact, retained_logits, retained_logits, 404]
        );

        let generation_two_fresh = [None, None, None, Some(505)];
        let mut generation_two_selection = RearmRangeSelectionV1::new(&retained.semantic);
        let generation_two = core::array::from_fn(|index| {
            generation_two_selection
                .select(
                    requests[index],
                    generation_one[index],
                    generation_two_fresh[index],
                    retained,
                    retained,
                )
                .unwrap()
        });
        assert_eq!(generation_two_selection.validate(), Ok(()));
        assert_eq!(
            generation_two,
            [retained_compact, retained_logits, retained_logits, 505]
        );
        assert_ne!(generation_zero[3], generation_one[3]);
        assert_ne!(generation_one[3], generation_two[3]);
    }

    #[test]
    fn direct_capture_rebinds_both_k6_users_to_each_fresh_generation() {
        use crate::physical_buffer_recipe::tests::exact_inputs;

        let target = selection(Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS8C8192);
        let (kernargs, composition) =
            exact_inputs(crate::M1StepDispatchIntent::TargetOnly(target), 208);
        let recipe = crate::derive_m1_physical_buffer_recipe_v1(kernargs, composition).unwrap();
        let source_rows = recipe.rows();
        let composition = recipe.workspace_composition();
        let generation_zero_capture = direct_capture_ranges(101, 202);
        let generation_one_capture = direct_capture_ranges(101, 303);
        let generation_two_capture = direct_capture_ranges(101, 404);
        let mut old_token = 1_000_u64;
        let generation_zero = source_rows
            .iter()
            .map(|source| {
                let buffers = source
                    .buffers()
                    .iter()
                    .map(|semantic| {
                        let request = requested_workspace_range(
                            semantic.source(),
                            composition,
                            &generation_zero_capture.semantic,
                        )
                        .unwrap();
                        let range = match request {
                            RearmRangeRequestV1::RetainedCompletionOutput => 101,
                            RearmRangeRequestV1::RetainedDirectDiagnosticChoices => 202,
                            RearmRangeRequestV1::FreshWorkspace(_, _)
                            | RearmRangeRequestV1::Unchanged => {
                                old_token += 1;
                                old_token
                            }
                            RearmRangeRequestV1::RetainedQualificationLogits
                            | RearmRangeRequestV1::RetainedSpeculativeDraftChoices
                            | RearmRangeRequestV1::RetainedSpeculativeDraftChoice { .. }
                            | RearmRangeRequestV1::RetainedSpeculativeTargetChoices => {
                                panic!("direct recipe entered another capture route")
                            }
                        };
                        RearmBoundRangeV1 {
                            explicit_argument_index: semantic.explicit_argument_index(),
                            range,
                        }
                    })
                    .collect();
                RearmBoundRowV1 {
                    dispatch_index: source.dispatch_index(),
                    profile_id: source.profile_id(),
                    program: source.program(),
                    buffers,
                }
            })
            .collect::<Vec<_>>();

        let mut generation_one_fresh = 10_000_u64;
        let generation_one = rebuild_bound_row_ranges(
            source_rows,
            &generation_zero,
            composition,
            generation_zero_capture,
            generation_one_capture,
            |workspace, _| {
                generation_one_fresh += 1;
                Ok((workspace, generation_one_fresh))
            },
        )
        .unwrap();
        let mut generation_two_fresh = 20_000_u64;
        let generation_two = rebuild_bound_row_ranges(
            source_rows,
            &generation_one,
            composition,
            generation_one_capture,
            generation_two_capture,
            |workspace, _| {
                generation_two_fresh += 1;
                Ok((workspace, generation_two_fresh))
            },
        )
        .unwrap();

        let mut direct_sources = 0;
        for ((source, first), second) in
            source_rows.iter().zip(&generation_one).zip(&generation_two)
        {
            for ((semantic, first), second) in source
                .buffers()
                .iter()
                .zip(&first.buffers)
                .zip(&second.buffers)
            {
                if requested_workspace_range(
                    semantic.source(),
                    composition,
                    &generation_one_capture.semantic,
                )
                .unwrap()
                    == RearmRangeRequestV1::RetainedDirectDiagnosticChoices
                {
                    direct_sources += 1;
                    assert_eq!(first.range, 303);
                    assert_eq!(second.range, 404);
                }
            }
        }
        assert_eq!(direct_sources, 2);

        let mut hostile = generation_one.clone();
        let hostile_choice = source_rows
            .iter()
            .zip(&mut hostile)
            .flat_map(|(source, row)| source.buffers().iter().zip(&mut row.buffers))
            .find(|(semantic, _)| {
                retained_capture_range_request(semantic.source(), &generation_one_capture.semantic)
                    == Some(RearmRangeRequestV1::RetainedDirectDiagnosticChoices)
            })
            .map(|(_, buffer)| buffer)
            .expect("direct choice source exists");
        hostile_choice.range = 999_999;
        assert!(rebuild_bound_row_ranges(
            source_rows,
            &hostile,
            composition,
            generation_one_capture,
            generation_two_capture,
            |workspace, _| Ok((workspace, 30_000)),
        )
        .is_err());
    }

    #[test]
    fn speculative_capture_rebinds_every_k4_k8_k16_choice_use_to_fresh_ranges() {
        for draft_tokens in [4_u8, 8, 16] {
            let mut requests = vec![
                RearmRangeRequestV1::RetainedCompletionOutput,
                RearmRangeRequestV1::RetainedSpeculativeDraftChoices,
                RearmRangeRequestV1::RetainedSpeculativeDraftChoices,
            ];
            requests.extend((0..draft_tokens).map(|iteration| {
                RearmRangeRequestV1::RetainedSpeculativeDraftChoice { iteration }
            }));
            requests.extend((0..draft_tokens - 1).map(|iteration| {
                RearmRangeRequestV1::RetainedSpeculativeDraftChoice { iteration }
            }));
            requests.extend([
                RearmRangeRequestV1::RetainedSpeculativeTargetChoices,
                RearmRangeRequestV1::RetainedSpeculativeTargetChoices,
            ]);
            let generation_zero_capture =
                speculative_capture_ranges(101, 201, draft_tokens, 211, 301);
            let generation_one_capture =
                speculative_capture_ranges(101, 401, draft_tokens, 411, 501);
            let generation_two_capture =
                speculative_capture_ranges(101, 601, draft_tokens, 611, 701);
            let materialize = |capture: RetainedCaptureRangesV1<u64>| {
                let RetainedSemanticCaptureRangesV1::SpeculativeDiagnostic {
                    draft,
                    draft_rows,
                    target,
                    ..
                } = capture.semantic
                else {
                    unreachable!()
                };
                requests
                    .iter()
                    .map(|request| match request {
                        RearmRangeRequestV1::RetainedCompletionOutput => capture.completion_output,
                        RearmRangeRequestV1::RetainedSpeculativeDraftChoices => draft,
                        RearmRangeRequestV1::RetainedSpeculativeDraftChoice { iteration } => {
                            draft_rows[usize::from(*iteration)].unwrap()
                        }
                        RearmRangeRequestV1::RetainedSpeculativeTargetChoices => target,
                        _ => unreachable!(),
                    })
                    .collect::<Vec<_>>()
            };
            let generation_zero = materialize(generation_zero_capture);
            let expected_one = materialize(generation_one_capture);
            let expected_two = materialize(generation_two_capture);

            let mut first = RearmRangeSelectionV1::new(&generation_one_capture.semantic);
            let generation_one = requests
                .iter()
                .enumerate()
                .map(|(index, request)| {
                    first
                        .select(
                            *request,
                            generation_zero[index],
                            None,
                            generation_zero_capture,
                            generation_one_capture,
                        )
                        .unwrap()
                })
                .collect::<Vec<_>>();
            assert_eq!(first.validate(), Ok(()));
            assert_eq!(generation_one, expected_one);

            let mut second = RearmRangeSelectionV1::new(&generation_two_capture.semantic);
            let generation_two = requests
                .iter()
                .enumerate()
                .map(|(index, request)| {
                    second
                        .select(
                            *request,
                            generation_one[index],
                            None,
                            generation_one_capture,
                            generation_two_capture,
                        )
                        .unwrap()
                })
                .collect::<Vec<_>>();
            assert_eq!(second.validate(), Ok(()));
            assert_eq!(generation_two, expected_two);

            let hostile_index = 3 + usize::from(draft_tokens) - 1;
            let mut hostile = generation_one;
            hostile[hostile_index] = 999_999;
            let mut rejected = RearmRangeSelectionV1::new(&generation_two_capture.semantic);
            assert_eq!(
                rejected.select(
                    requests[hostile_index],
                    hostile[hostile_index],
                    None,
                    generation_one_capture,
                    generation_two_capture,
                ),
                Err(())
            );
        }
    }

    #[test]
    fn exact_target_recipe_rebuild_retains_capture_and_rebinds_other_workspaces_twice() {
        use crate::physical_buffer_recipe::tests::exact_inputs;

        let target = selection(Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS8C8192);
        let (kernargs, composition) =
            exact_inputs(crate::M1StepDispatchIntent::TargetOnly(target), 210);
        let recipe = crate::derive_m1_physical_buffer_recipe_v1(kernargs, composition).unwrap();
        let source_rows = recipe.rows();
        assert_eq!(source_rows.len(), 545);
        let composition = recipe.workspace_composition();
        let retained_compact = 101_u64;
        let retained_logits = 202_u64;
        let retained = qualification_capture_ranges(retained_compact, retained_logits);
        let mut old_token = 1_000_u64;
        let old_rows = source_rows
            .iter()
            .map(|source| {
                let buffers = source
                    .buffers()
                    .iter()
                    .map(|semantic| {
                        let request = requested_workspace_range(
                            semantic.source(),
                            composition,
                            &retained.semantic,
                        )
                        .unwrap();
                        let range = match request {
                            RearmRangeRequestV1::RetainedCompletionOutput => retained_compact,
                            RearmRangeRequestV1::RetainedQualificationLogits => retained_logits,
                            RearmRangeRequestV1::RetainedDirectDiagnosticChoices
                            | RearmRangeRequestV1::RetainedSpeculativeDraftChoices
                            | RearmRangeRequestV1::RetainedSpeculativeDraftChoice { .. }
                            | RearmRangeRequestV1::RetainedSpeculativeTargetChoices => {
                                panic!("qualification recipe entered a diagnostic route")
                            }
                            RearmRangeRequestV1::FreshWorkspace(_, _)
                            | RearmRangeRequestV1::Unchanged => {
                                old_token += 1;
                                old_token
                            }
                        };
                        RearmBoundRangeV1 {
                            explicit_argument_index: semantic.explicit_argument_index(),
                            range,
                        }
                    })
                    .collect();
                RearmBoundRowV1 {
                    dispatch_index: source.dispatch_index(),
                    profile_id: source.profile_id(),
                    program: source.program(),
                    buffers,
                }
            })
            .collect::<Vec<_>>();

        let mut generation_one_fresh = 10_000_u64;
        let generation_one = rebuild_bound_row_ranges(
            source_rows,
            &old_rows,
            composition,
            retained,
            retained,
            |workspace, _| {
                generation_one_fresh += 1;
                Ok((workspace, generation_one_fresh))
            },
        )
        .unwrap();
        let mut generation_two_fresh = 20_000_u64;
        let generation_two = rebuild_bound_row_ranges(
            source_rows,
            &generation_one,
            composition,
            retained,
            retained,
            |workspace, _| {
                generation_two_fresh += 1;
                Ok((workspace, generation_two_fresh))
            },
        )
        .unwrap();

        let mut compact_sources = 0;
        let mut logits_sources = 0;
        let mut changed_workspaces = 0;
        for ((source, first), second) in
            source_rows.iter().zip(&generation_one).zip(&generation_two)
        {
            for ((semantic, first), second) in source
                .buffers()
                .iter()
                .zip(&first.buffers)
                .zip(&second.buffers)
            {
                match requested_workspace_range(semantic.source(), composition, &retained.semantic)
                    .unwrap()
                {
                    RearmRangeRequestV1::RetainedCompletionOutput => {
                        compact_sources += 1;
                        assert_eq!(first.range, retained_compact);
                        assert_eq!(second.range, retained_compact);
                    }
                    RearmRangeRequestV1::RetainedQualificationLogits => {
                        logits_sources += 1;
                        assert_eq!(first.range, retained_logits);
                        assert_eq!(second.range, retained_logits);
                    }
                    RearmRangeRequestV1::RetainedDirectDiagnosticChoices
                    | RearmRangeRequestV1::RetainedSpeculativeDraftChoices
                    | RearmRangeRequestV1::RetainedSpeculativeDraftChoice { .. }
                    | RearmRangeRequestV1::RetainedSpeculativeTargetChoices => {
                        panic!("qualification recipe entered a diagnostic route")
                    }
                    RearmRangeRequestV1::FreshWorkspace(_, _) => {
                        changed_workspaces += 1;
                        assert_ne!(first.range, second.range);
                    }
                    RearmRangeRequestV1::Unchanged => {
                        assert_eq!(first.range, second.range);
                    }
                }
            }
        }
        assert_eq!(compact_sources, 1);
        assert_eq!(logits_sources, 2);
        assert!(changed_workspaces > 0);

        let mut hostile = generation_one.clone();
        let mut substituted = false;
        for (source, row) in source_rows.iter().zip(&mut hostile) {
            for (semantic, buffer) in source.buffers().iter().zip(&mut row.buffers) {
                if !substituted
                    && retained_capture_range_request(semantic.source(), &retained.semantic)
                        == Some(RearmRangeRequestV1::RetainedQualificationLogits)
                {
                    buffer.range = 999_999;
                    substituted = true;
                }
            }
        }
        assert!(substituted);
        assert!(rebuild_bound_row_ranges(
            source_rows,
            &hostile,
            composition,
            retained,
            retained,
            |workspace, _| Ok((workspace, 30_000)),
        )
        .is_err());

        let mut substituted_role = false;
        assert!(rebuild_bound_row_ranges(
            source_rows,
            &generation_one,
            composition,
            retained,
            retained,
            |workspace, _| {
                substituted_role = true;
                let hostile = match workspace {
                    M1FullStepWorkspaceRole::Draft => M1FullStepWorkspaceRole::Target,
                    M1FullStepWorkspaceRole::Target => M1FullStepWorkspaceRole::Draft,
                };
                Ok((hostile, 40_000))
            },
        )
        .is_err());
        assert!(substituted_role);

        let mut retained_logits_seen = 0;
        assert!(rebuild_bound_row_ranges_with_requests(
            source_rows,
            &old_rows,
            retained,
            retained,
            |source| {
                let request = requested_workspace_range(source, composition, &retained.semantic)?;
                if request == RearmRangeRequestV1::RetainedQualificationLogits {
                    retained_logits_seen += 1;
                    if retained_logits_seen == 2 {
                        return Ok(RearmRangeRequestV1::Unchanged);
                    }
                }
                Ok(request)
            },
            |workspace, _| Ok((workspace, 50_000)),
        )
        .is_err());
        assert_eq!(retained_logits_seen, 2);

        let mut too_many_old = old_rows.clone();
        let mut rewrote_fresh_range = false;
        'rows: for (source, old) in source_rows.iter().zip(&mut too_many_old) {
            for (semantic, old_buffer) in source.buffers().iter().zip(&mut old.buffers) {
                if matches!(
                    requested_workspace_range(semantic.source(), composition, &retained.semantic,)
                        .unwrap(),
                    RearmRangeRequestV1::FreshWorkspace(_, _)
                ) {
                    old_buffer.range = retained_logits;
                    rewrote_fresh_range = true;
                    break 'rows;
                }
            }
        }
        assert!(rewrote_fresh_range);
        let mut injected_extra_logits = false;
        assert!(rebuild_bound_row_ranges_with_requests(
            source_rows,
            &too_many_old,
            retained,
            retained,
            |source| {
                let request = requested_workspace_range(source, composition, &retained.semantic)?;
                if !injected_extra_logits
                    && matches!(request, RearmRangeRequestV1::FreshWorkspace(_, _))
                {
                    injected_extra_logits = true;
                    Ok(RearmRangeRequestV1::RetainedQualificationLogits)
                } else {
                    Ok(request)
                }
            },
            |workspace, _| Ok((workspace, 60_000)),
        )
        .is_err());
        assert!(injected_extra_logits);
    }

    #[test]
    fn qualification_capture_range_substitution_is_rejected() {
        let retained = qualification_capture_ranges(101, 202);
        let mut selection = RearmRangeSelectionV1::new(&retained.semantic);
        assert_eq!(
            selection.select(
                RearmRangeRequestV1::RetainedQualificationLogits,
                909_u64,
                None,
                retained,
                retained,
            ),
            Err(())
        );
        let mut selection = RearmRangeSelectionV1::new(&retained.semantic);
        assert_eq!(
            selection.select(
                RearmRangeRequestV1::RetainedCompletionOutput,
                808_u64,
                None,
                retained,
                retained,
            ),
            Err(())
        );
        assert_eq!(
            RearmRangeSelectionV1::new(&retained.semantic).validate(),
            Err(())
        );
    }

    #[test]
    fn rearm_snapshot_association_is_preserved_only_for_completion_output() {
        let completion = M1PhysicalBufferSourceV1::CompletionOutput { sequences: 1 };
        let ordinary = M1PhysicalBufferSourceV1::Workspace {
            workspace: M1FullStepWorkspaceRole::Target,
            range: ferric_build::M1StepWorkspaceRangeRole::ResidualHidden,
        };
        assert_eq!(
            select_rearm_completed_snapshot(completion, Some(41_u64), Some(41)),
            Ok(Some(41))
        );
        assert_eq!(
            select_rearm_completed_snapshot(completion, None::<u64>, None),
            Ok(None)
        );
        assert_eq!(
            select_rearm_completed_snapshot(completion, None, Some(41_u64)),
            Err(())
        );
        assert_eq!(
            select_rearm_completed_snapshot(completion, Some(40_u64), Some(41)),
            Err(())
        );
        assert_eq!(
            select_rearm_completed_snapshot(ordinary, None::<u64>, Some(41)),
            Ok(None)
        );
        assert_eq!(
            select_rearm_completed_snapshot(ordinary, Some(41_u64), Some(41)),
            Err(())
        );
    }

    #[test]
    fn qualification_capture_role_and_cardinality_substitution_fail_closed() {
        use ferric_build::M1StepWorkspaceRangeRole;

        let retained = qualification_capture_ranges(101, 202);

        assert_eq!(
            retained_capture_range_request(
                M1PhysicalBufferSourceV1::Workspace {
                    workspace: M1FullStepWorkspaceRole::Target,
                    range: M1StepWorkspaceRangeRole::Logits,
                },
                &retained.semantic,
            ),
            Some(RearmRangeRequestV1::RetainedQualificationLogits)
        );
        assert_eq!(
            retained_capture_range_request(
                M1PhysicalBufferSourceV1::Workspace {
                    workspace: M1FullStepWorkspaceRole::Draft,
                    range: M1StepWorkspaceRangeRole::Logits,
                },
                &retained.semantic,
            ),
            None
        );

        let retained_compact = 101_u64;
        let retained_logits = 202_u64;
        let mut too_few = RearmRangeSelectionV1::new(&retained.semantic);
        too_few
            .select(
                RearmRangeRequestV1::RetainedCompletionOutput,
                retained_compact,
                None,
                retained,
                retained,
            )
            .unwrap();
        too_few
            .select(
                RearmRangeRequestV1::RetainedQualificationLogits,
                retained_logits,
                None,
                retained,
                retained,
            )
            .unwrap();
        assert_eq!(too_few.validate(), Err(()));

        let mut too_many = RearmRangeSelectionV1::new(&retained.semantic);
        too_many
            .select(
                RearmRangeRequestV1::RetainedCompletionOutput,
                retained_compact,
                None,
                retained,
                retained,
            )
            .unwrap();
        for _ in 0..3 {
            too_many
                .select(
                    RearmRangeRequestV1::RetainedQualificationLogits,
                    retained_logits,
                    None,
                    retained,
                    retained,
                )
                .unwrap();
        }
        assert_eq!(too_many.validate(), Err(()));
    }

    #[test]
    fn generic_readback_retry_rejoins_exact_continuation_custody() {
        let selected = RequestId::new(0, 3);
        let parked = RequestId::new(1, 5);
        let success = rejoin_readback_retry(
            Ok::<u8, u16>(41),
            test_rearm_carry(selected, parked),
            73_u32,
            test_device(),
        )
        .unwrap();
        assert_eq!(success.source, 41);
        assert_eq!(success.queue_observation, 73);
        assert_eq!(success.carry.selected[0].projection().request, selected);
        assert_eq!(success.carry.parked[0].projection().request, parked);
        assert_eq!(&*success.carry.logical_accepted_counts, &[1]);
        assert_eq!(&*success.carry.externally_published_counts, &[0]);
        assert_eq!(success.device, test_device());

        let failure = rejoin_readback_retry(
            Err::<u8, u16>(97),
            test_rearm_carry(selected, parked),
            89_u32,
            test_device(),
        )
        .unwrap_err();
        assert_eq!(failure.source, 97);
        assert_eq!(failure.queue_observation, 89);
        assert_eq!(failure.carry.selected[0].projection().request, selected);
        assert_eq!(failure.carry.parked[0].projection().request, parked);
        assert_eq!(&*failure.carry.logical_accepted_counts, &[1]);
        assert_eq!(&*failure.carry.externally_published_counts, &[0]);
        assert_eq!(failure.device, test_device());
    }

    #[test]
    fn generic_readback_teardown_quarantines_in_flight_engine() {
        let mut engine = Engine::<1>::new(8, 4, 32).unwrap();
        let request = engine.admit().unwrap();
        engine.append_tentative(request, 1).unwrap();
        let scheduled = engine.dispatch_m1_ready().unwrap().unwrap();
        assert_eq!(scheduled.member(0), Some(request));
        assert!(!engine.is_faulted());

        quarantine_readback_teardown(&mut engine);
        assert!(engine.is_faulted());
        assert_eq!(engine.state(request), Some(RequestState::InFlight));
    }

    #[test]
    fn generic_readback_failure_has_consuming_retry_and_teardown_signatures() {
        type Observe =
            fn(
                M1RearmedRecycledQueueV1,
            )
                -> Result<M1RearmedObservedCompletionOutputV1, Box<M1RearmedReadbackFailureV1>>;
        type Image = for<'a> fn(
            &'a M1RearmedObservedCompletionOutputV1,
        ) -> &'a crate::M1ObservedCompletionImageV1;
        type Check =
            for<'a, 'b> fn(
                M1RearmedObservedCompletionOutputV1,
                &'a [crate::CompletionWireSemanticExpectation<'b>],
            )
                -> Result<M1RearmedCompletedReadbackV1, Box<M1RearmedReadbackFailureV1>>;
        type ObservationRetry =
            fn(
                Box<M1RearmedReadbackFailureV1>,
            )
                -> Result<M1RearmedObservedCompletionOutputV1, Box<M1RearmedReadbackFailureV1>>;
        type ObservedRecovery =
            fn(
                Box<M1RearmedReadbackFailureV1>,
            )
                -> Result<M1RearmedObservedCompletionOutputV1, Box<M1RearmedReadbackFailureV1>>;
        type ObservedTeardown = fn(
            M1RearmedObservedCompletionOutputV1,
            &mut Engine<32>,
        ) -> Result<
            M1RearmedReadbackTeardownSuccessV1,
            Box<M1RearmedReadbackTeardownFailureV1>,
        >;
        type Retry =
            for<'a, 'b> fn(
                Box<M1RearmedReadbackFailureV1>,
                &'a [crate::CompletionWireSemanticExpectation<'b>],
            )
                -> Result<M1RearmedCompletedReadbackV1, Box<M1RearmedReadbackFailureV1>>;
        type Teardown = fn(
            Box<M1RearmedReadbackFailureV1>,
            &mut Engine<32>,
        ) -> Result<
            M1RearmedReadbackTeardownSuccessV1,
            Box<M1RearmedReadbackTeardownFailureV1>,
        >;

        let _: Observe = M1RearmedRecycledQueueV1::observe_completion;
        let _: Image = M1RearmedObservedCompletionOutputV1::image;
        let _: Check = M1RearmedObservedCompletionOutputV1::check_completion;
        let _: ObservationRetry = M1RearmedReadbackFailureV1::retry_observation;
        let _: ObservedRecovery =
            M1RearmedReadbackFailureV1::recover_observed_after_semantic_rejection;
        let _: ObservedTeardown =
            M1RearmedObservedCompletionOutputV1::destroy_queue_and_retain_custody::<32>;
        let _: Retry = M1RearmedReadbackFailureV1::retry;
        let _: Teardown = M1RearmedReadbackFailureV1::destroy_queue_and_retain_custody::<32>;
    }

    #[test]
    fn terminal_completion_preflight_rejects_until_every_request_is_retiring() {
        let mut engine = Engine::<2>::new(8, 4, 32).unwrap();
        let request = engine.admit().unwrap();
        engine.append_tentative(request, 1).unwrap();
        let scheduled = engine.dispatch_m1_ready().unwrap().unwrap();
        assert_eq!(scheduled.member(0), Some(request));
        assert_eq!(
            preflight_retiring_requests(&engine, [request].into_iter()),
            Err(M1RearmedCompletionPreflightErrorV1::SelectedNotRetiring { lane: 0 })
        );
        assert_eq!(engine.state(request), Some(RequestState::InFlight));

        engine.retire(request).unwrap();
        assert_eq!(engine.state(request), Some(RequestState::Retiring));
        assert_eq!(
            preflight_retiring_requests(&engine, [request].into_iter()),
            Ok(())
        );
    }

    #[test]
    fn terminal_qualification_teardown_quarantines_in_flight_engine() {
        let mut engine = Engine::<1>::new(8, 4, 32).unwrap();
        let request = engine.admit().unwrap();
        engine.append_tentative(request, 1).unwrap();
        let scheduled = engine.dispatch_m1_ready().unwrap().unwrap();
        assert_eq!(scheduled.member(0), Some(request));
        assert!(!engine.is_faulted());

        quarantine_qualification_teardown(&mut engine);
        assert!(engine.is_faulted());
        assert_eq!(engine.state(request), Some(RequestState::InFlight));
    }

    #[test]
    fn qualification_terminal_custody_graph_has_retry_release_and_teardown() {
        type ObservationRetry = fn(
            M1RearmedQualificationObservationFailureV1,
        ) -> Result<
            M1RearmedObservedQualificationOutputV1,
            Box<M1RearmedQualificationObservationFailureV1>,
        >;
        type ObservationTeardown = fn(
            M1RearmedQualificationObservationFailureV1,
            &mut Engine<32>,
        ) -> Result<
            M1RearmedQualificationObservationTeardownSuccessV1,
            Box<M1RearmedQualificationObservationTeardownFailureV1>,
        >;
        type SemanticRecovery = fn(
            M1RearmedQualificationCompletedReadbackJoinFailureV1,
        ) -> (
            crate::M1CompletedReadbackJoinErrorV1,
            M1RearmedObservedQualificationOutputV1,
        );
        type SemanticTeardown = fn(
            M1RearmedQualificationCompletedReadbackJoinFailureV1,
            &mut Engine<32>,
        ) -> Result<
            M1RearmedQualificationSemanticTeardownSuccessV1,
            Box<M1RearmedQualificationSemanticTeardownFailureV1>,
        >;
        type FinalSemanticRetry = for<'a> fn(
            M1RearmedObservedQualificationOutputV1,
            &'a [crate::M1ValidatedQualificationContextStepV1],
        ) -> Result<
            M1RearmedQualifiedCompletedReadbackV1,
            M1RearmedQualificationCompletedReadbackJoinFailureV1,
        >;
        type TerminalCompletion = fn(
            M1RearmedQualifiedCompletedReadbackV1,
            &mut Engine<32>,
        ) -> Result<
            M1RearmedQualifiedCompletionOutcomeV1,
            Box<M1RearmedQualifiedCompletionPreflightFailureV1>,
        >;
        type QualifiedRecovery = fn(
            M1RearmedQualifiedCompletionOutcomeV1,
        ) -> (
            M1RearmedCompletionOutcomeV1,
            crate::M1QualificationCompletionEvidenceV1,
        );
        type QualifiedCompletionRetry = fn(
            M1RearmedQualifiedCompletionOutcomeV1,
            &mut Engine<32>,
        ) -> Result<
            M1RearmedQualifiedCompletionOutcomeV1,
            Box<M1RearmedQualifiedCompletionOutcomeV1>,
        >;
        type PageRelease =
            fn(M1RearmedQualifiedCompletionOutcomeV1) -> M1RearmedQualifiedRoundReleaseOutcomeV1;
        type PageReleaseRetry = fn(
            M1RearmedQualifiedRoundPageReleaseFailureV1,
        ) -> M1RearmedQualifiedRoundReleaseOutcomeV1;
        type TerminalTeardown = fn(
            M1RearmedQualifiedReleasedRoundV1,
            &mut Engine<32>,
        ) -> Result<
            M1RearmedQualifiedTeardownSuccessV1,
            Box<M1RearmedQualifiedTeardownFailureV1>,
        >;

        fn retry_final_semantic(
            observed: M1RearmedObservedQualificationOutputV1,
            contexts: &[crate::M1ValidatedQualificationContextStepV1],
        ) -> Result<
            M1RearmedQualifiedCompletedReadbackV1,
            M1RearmedQualificationCompletedReadbackJoinFailureV1,
        > {
            observed.check_final_completion(contexts)
        }

        let _: ObservationRetry = M1RearmedQualificationObservationFailureV1::retry;
        let _: ObservationTeardown =
            M1RearmedQualificationObservationFailureV1::destroy_queue_and_retain_custody::<32>;
        let _: SemanticRecovery = M1RearmedQualificationCompletedReadbackJoinFailureV1::into_parts;
        let _: SemanticTeardown =
            M1RearmedQualificationCompletedReadbackJoinFailureV1::destroy_queue_and_retain_custody::<
                32,
            >;
        let _: FinalSemanticRetry = retry_final_semantic;
        let _: TerminalCompletion = M1RearmedQualifiedCompletedReadbackV1::complete_retiring::<32>;
        let _: QualifiedRecovery = M1RearmedQualifiedCompletionOutcomeV1::into_parts;
        let _: QualifiedCompletionRetry =
            M1RearmedQualifiedCompletionOutcomeV1::retry_rejected::<32>;
        let _: PageRelease = M1RearmedQualifiedCompletionOutcomeV1::release_completed;
        let _: PageReleaseRetry = M1RearmedQualifiedRoundPageReleaseFailureV1::retry;
        let _: TerminalTeardown = M1RearmedQualifiedReleasedRoundV1::destroy_queue_and_retain_round;
    }

    #[test]
    fn qualification_observation_teardown_exposes_bytes_and_full_lineage() {
        fn assert_success(owner: &M1RearmedQualificationObservationTeardownSuccessV1) {
            let _: &M1RearmedReadbackTeardownEvidenceV1 = owner.compact_evidence();
            let _: &[ServiceCompletedReadbackV1] = owner.partial_logits();
            let _: Option<&[u8]> = owner.partial_logits_row_bytes(0);
            let _: &crate::M1CheckedCompletionOutputV1 = owner.prior_checked();
            let _: &[u32] = owner.prior_logical_accepted_counts();
            let _: &[u32] = owner.prior_externally_published_counts();
            let _: &[M1CompletedKvPageReleaseCountsV1] = owner.prior_release_counts();
            let _: usize = owner.terminal_lineage_count();
            let _: Option<&M1RearmRoundHistoryEntryV1> = owner.round_history(0);
            assert_eq!(
                owner.capture_release_state(),
                M1RearmedReadbackCaptureReleaseStateV1::Released
            );
        }

        fn assert_failure(owner: &M1RearmedQualificationObservationTeardownFailureV1) {
            let _: &crate::M1PhysicalQueueReleaseFailureV1 = owner.source();
            let _: &M1RearmedReadbackTeardownEvidenceV1 = owner.compact_evidence();
            let _: &[ServiceCompletedReadbackV1] = owner.partial_logits();
            let _: Option<&[u8]> = owner.partial_logits_row_bytes(0);
            let _: &crate::M1CheckedCompletionOutputV1 = owner.prior_checked();
            let _: &[u32] = owner.prior_logical_accepted_counts();
            let _: &[u32] = owner.prior_externally_published_counts();
            let _: &[M1CompletedKvPageReleaseCountsV1] = owner.prior_release_counts();
            let _: usize = owner.terminal_lineage_count();
            let _: Option<&M1RearmRoundHistoryEntryV1> = owner.round_history(0);
            assert_eq!(
                owner.capture_release_state(),
                M1RearmedReadbackCaptureReleaseStateV1::LowerReleaseFailure
            );
        }

        let _: fn(&M1RearmedQualificationObservationTeardownSuccessV1) = assert_success;
        let _: fn(&M1RearmedQualificationObservationTeardownFailureV1) = assert_failure;
    }

    #[test]
    fn every_runtime_failure_phase_is_closed() {
        let phases = [
            M1LongLivedQueueRearmSubmissionPhaseV1::Preflight,
            M1LongLivedQueueRearmSubmissionPhaseV1::DraftWorkspaceReplacement,
            M1LongLivedQueueRearmSubmissionPhaseV1::TargetWorkspaceReplacement,
            M1LongLivedQueueRearmSubmissionPhaseV1::WorkspaceRangeRebinding,
            M1LongLivedQueueRearmSubmissionPhaseV1::RolloverOutputActivation,
            M1LongLivedQueueRearmSubmissionPhaseV1::FixedBatchRebuild,
            M1LongLivedQueueRearmSubmissionPhaseV1::QueueBind,
            M1LongLivedQueueRearmSubmissionPhaseV1::QueueRollover,
            M1LongLivedQueueRearmSubmissionPhaseV1::QueueObservation,
            M1LongLivedQueueRearmSubmissionPhaseV1::QueueSubmit,
        ];
        assert_eq!(phases.len(), 10);
    }

    #[test]
    fn exact_successor_rejects_stale_skips_and_exhaustion() {
        assert_eq!(
            exact_next_epoch(CompletionEpoch::new(41)),
            Some(CompletionEpoch::new(42))
        );
        assert_ne!(
            exact_next_epoch(CompletionEpoch::new(41)),
            Some(CompletionEpoch::new(41))
        );
        assert_ne!(
            exact_next_epoch(CompletionEpoch::new(41)),
            Some(CompletionEpoch::new(43))
        );
        assert_eq!(exact_next_epoch(CompletionEpoch::new(u64::MAX)), None);
    }

    #[test]
    fn pure_third_epoch_partition_preserves_and_then_selects_parked_lineage() {
        let first = RequestId::new(0, 1);
        let parked = RequestId::new(1, 1);
        let after_release = [first, parked];

        assert_eq!(validate_request_partition(&after_release, &[first]), Ok(()));
        assert_eq!(
            exact_next_epoch(CompletionEpoch::new(1)),
            Some(CompletionEpoch::new(2))
        );

        // This pure boundary test cannot construct the KFD-backed released
        // queue fixture. It proves only the exact epoch and request-partition
        // rules used by the closed physical owner transition.
        assert_eq!(
            validate_request_partition(&after_release, &[parked, first]),
            Ok(())
        );
        assert_eq!(
            exact_next_epoch(CompletionEpoch::new(2)),
            Some(CompletionEpoch::new(3))
        );
    }

    #[test]
    fn hostile_partition_rejects_duplicate_and_new_requests() {
        let first = RequestId::new(0, 1);
        let stale_first = RequestId::new(0, 2);
        let second = RequestId::new(1, 1);
        assert_eq!(
            validate_request_partition(&[first, second], &[second, second]),
            Err(
                M1LongLivedQueueRearmScheduleErrorV1::DuplicateScheduledRequest {
                    first_lane: 0,
                    lane: 1,
                }
            )
        );
        assert_eq!(
            validate_request_partition(&[first], &[second]),
            Err(M1LongLivedQueueRearmScheduleErrorV1::UnownedScheduledRequest { lane: 0 })
        );
        assert_eq!(
            validate_request_partition(&[first, stale_first], &[first, stale_first]),
            Err(
                M1LongLivedQueueRearmScheduleErrorV1::DuplicateScheduledRequest {
                    first_lane: 0,
                    lane: 1,
                }
            )
        );
    }

    #[test]
    fn exact_rearm_dispatch_preserves_named_order_and_parks_unrelated_ready() {
        let mut engine = Engine::<4>::new(32, 4, 64).unwrap();
        let first = engine.admit().unwrap();
        let unrelated = engine.admit().unwrap();
        let third = engine.admit().unwrap();

        let scheduled = dispatch_m1_long_lived_queue_rearm_v1(
            &mut engine,
            M1LongLivedQueueRearmDispatchV1::Exact {
                expected_epoch: CompletionEpoch::new(1),
                requests: &[third, first],
            },
        )
        .unwrap();

        assert_eq!(scheduled.epoch(), CompletionEpoch::new(1));
        assert_eq!(scheduled.member_count(), 2);
        assert_eq!(scheduled.member(0), Some(third));
        assert_eq!(scheduled.member(1), Some(first));
        assert_eq!(engine.state(third), Some(RequestState::InFlight));
        assert_eq!(engine.state(first), Some(RequestState::InFlight));
        assert_eq!(engine.state(unrelated), Some(RequestState::Ready));
    }

    #[test]
    fn exact_rearm_dispatch_rejects_hostile_rosters_before_engine_mutation() {
        fn exact_error(
            result: Result<M1ScheduledDispatchV1, M1LongLivedQueueRearmDispatchFailureV1>,
        ) -> M1ExactDispatchErrorV1 {
            match result {
                Err(M1LongLivedQueueRearmDispatchFailureV1::Exact(error)) => error,
                Ok(_) | Err(_) => panic!("expected exact scheduler rejection"),
            }
        }

        let mut duplicate_engine = Engine::<4>::new(32, 4, 64).unwrap();
        let duplicate = duplicate_engine.admit().unwrap();
        assert_eq!(
            exact_error(dispatch_m1_long_lived_queue_rearm_v1(
                &mut duplicate_engine,
                M1LongLivedQueueRearmDispatchV1::Exact {
                    expected_epoch: CompletionEpoch::new(1),
                    requests: &[duplicate, duplicate],
                },
            )),
            M1ExactDispatchErrorV1::DuplicateRequest {
                first_lane: 0,
                lane: 1,
            }
        );
        assert_eq!(duplicate_engine.state(duplicate), Some(RequestState::Ready));

        let mut missing_engine = Engine::<4>::new(32, 4, 64).unwrap();
        let retained = missing_engine.admit().unwrap();
        let missing = RequestId::new(3, 99);
        assert_eq!(
            exact_error(dispatch_m1_long_lived_queue_rearm_v1(
                &mut missing_engine,
                M1LongLivedQueueRearmDispatchV1::Exact {
                    expected_epoch: CompletionEpoch::new(1),
                    requests: &[missing],
                },
            )),
            M1ExactDispatchErrorV1::MissingRequest {
                lane: 0,
                request: missing,
            }
        );
        assert_eq!(missing_engine.state(retained), Some(RequestState::Ready));

        let mut nonready_engine = Engine::<2>::new(16, 4, 32).unwrap();
        let nonready = nonready_engine.admit().unwrap();
        let _scheduled = dispatch_m1_long_lived_queue_rearm_v1(
            &mut nonready_engine,
            M1LongLivedQueueRearmDispatchV1::Exact {
                expected_epoch: CompletionEpoch::new(1),
                requests: &[nonready],
            },
        )
        .unwrap();
        let retained_ready = nonready_engine.admit().unwrap();
        assert_eq!(
            exact_error(dispatch_m1_long_lived_queue_rearm_v1(
                &mut nonready_engine,
                M1LongLivedQueueRearmDispatchV1::Exact {
                    expected_epoch: CompletionEpoch::new(2),
                    requests: &[nonready],
                },
            )),
            M1ExactDispatchErrorV1::RequestNotReady {
                lane: 0,
                request: nonready,
                state: RequestState::InFlight,
            }
        );
        assert_eq!(
            nonready_engine.state(nonready),
            Some(RequestState::InFlight)
        );
        assert_eq!(
            nonready_engine.state(retained_ready),
            Some(RequestState::Ready)
        );

        let mut epoch_engine = Engine::<2>::new(16, 4, 32).unwrap();
        let epoch_request = epoch_engine.admit().unwrap();
        assert_eq!(
            exact_error(dispatch_m1_long_lived_queue_rearm_v1(
                &mut epoch_engine,
                M1LongLivedQueueRearmDispatchV1::Exact {
                    expected_epoch: CompletionEpoch::new(2),
                    requests: &[epoch_request],
                },
            )),
            M1ExactDispatchErrorV1::EpochMismatch {
                expected: CompletionEpoch::new(1),
                actual: CompletionEpoch::new(2),
            }
        );
        assert_eq!(epoch_engine.state(epoch_request), Some(RequestState::Ready));
    }

    #[test]
    fn exact_rearm_api_keeps_terminal_and_post_dispatch_custody_nameable() {
        type ExactInitial =
            fn(
                &mut Engine<32>,
                M1ReleasedCompletedStepV1,
                CompletionEpoch,
                &[RequestId],
            )
                -> Result<M1ScheduledLongLivedQueueRearmV1, M1LongLivedQueueRearmScheduleFailureV1>;
        type ExactNext =
            fn(
                M1LongLivedQueueReleasedRoundV1,
                &mut Engine<32>,
                CompletionEpoch,
                &[RequestId],
            )
                -> Result<M1ScheduledLongLivedQueueRearmV1, M1LongLivedQueueRearmScheduleFailureV1>;
        type ExactRetry =
            fn(
                M1LongLivedQueueUnscheduledRoundV1,
                &mut Engine<32>,
                CompletionEpoch,
                &[RequestId],
            )
                -> Result<M1ScheduledLongLivedQueueRearmV1, M1LongLivedQueueRearmScheduleFailureV1>;
        type Close = fn(
            M1LongLivedQueueRearmScheduleFailureV1,
        ) -> M1LongLivedQueueRearmScheduleClosureOutcomeV1;
        type RetainedDispatch =
            for<'a> fn(&'a M1ScheduledLongLivedQueueRearmV1) -> &'a M1ScheduledDispatchV1;

        let _: ExactInitial = schedule_m1_long_lived_queue_rearm_exact_v1::<32>;
        let _: ExactNext = M1LongLivedQueueReleasedRoundV1::schedule_next_exact::<32>;
        let _: ExactRetry = M1LongLivedQueueUnscheduledRoundV1::retry_exact::<32>;
        let _: Close = M1LongLivedQueueRearmScheduleFailureV1::close_terminal;
        let _: RetainedDispatch = M1ScheduledLongLivedQueueRearmV1::scheduled_dispatch;

        assert_eq!(
            M1LongLivedQueueRearmScheduleErrorV1::ExactScheduler(
                M1ExactDispatchErrorV1::EmptyRoster,
            ),
            M1LongLivedQueueRearmScheduleErrorV1::ExactScheduler(
                M1ExactDispatchErrorV1::EmptyRoster,
            )
        );
    }

    #[test]
    fn speculative_diagnostic_failure_exposes_typed_terminal_closure() {
        type IntoParts = fn(
            Box<M1RearmedSpeculativeDiagnosticReadbackFailureV1>,
        ) -> (
            M1RearmedSpeculativeDiagnosticReadbackFailureSourceV1,
            M1RearmedSpeculativeDiagnosticRetainedCustodyV1,
        );
        type Teardown = fn(
            Box<M1RearmedSpeculativeDiagnosticReadbackFailureV1>,
            &mut Engine<32>,
        ) -> Result<
            M1RearmedSpeculativeDiagnosticReadbackTeardownSuccessV1,
            Box<M1RearmedSpeculativeDiagnosticReadbackTeardownFailureV1>,
        >;

        let _: IntoParts = M1RearmedSpeculativeDiagnosticReadbackFailureV1::into_parts;
        let _: Teardown =
            M1RearmedSpeculativeDiagnosticReadbackFailureV1::destroy_queue_and_retain_custody::<32>;
    }

    #[test]
    fn scheduling_wrapper_faults_every_consumed_queue_phase() {
        let mut released = Engine::<2>::new(16, 4, 32).unwrap();
        let released_result: Result<(), ()> = finish_schedule_transition(
            &mut released,
            Err((M1LongLivedQueueRearmSchedulePhaseV1::Released, ())),
        );
        assert_eq!(released_result, Err(()));
        assert!(!released.is_faulted());

        for phase in [
            M1LongLivedQueueRearmSchedulePhaseV1::QueueDetach,
            M1LongLivedQueueRearmSchedulePhaseV1::Detached,
            M1LongLivedQueueRearmSchedulePhaseV1::PostDispatch,
        ] {
            let mut terminal = Engine::<2>::new(16, 4, 32).unwrap();
            let terminal_result: Result<(), ()> =
                finish_schedule_transition(&mut terminal, Err((phase, ())));
            assert_eq!(terminal_result, Err(()));
            assert!(terminal.is_faulted());
        }
    }
}
