//! Exact cross-shape queue rollover from one S1 paired prefill into S1/K4.
//!
//! The scheduler owns preflight, detachment, exact dispatch, and quiescent KV
//! reselection. The preparation phase then reserves exact draft and target KV
//! writes and binds the successor workspace images for native rollover.

use core::fmt;

use fe2o3_service_host::{ServiceQueueReleaseFailureV1, ServiceQueueReleaseObservationV1};
use ferric_spec::{
    completion::CompletionEpoch, scheduling::RequestState, Qwen3ExecutionMode, Qwen3PlanBucket,
    ValidatedM1StepInputs,
};

use crate::{
    ActiveDeviceKvCache, DeviceKvCacheError, DeviceKvCacheProjection, DeviceKvPageLease, Engine,
    LogicalRunnerDeclaration, M1CheckedCompletionOutputV1, M1CompletedKvPageReleaseCountsV1,
    M1ExactDispatchErrorV1, M1FullStepKvWorkspaceTablesV1, M1FullStepWorkspacePlans,
    M1PhysicalFixedBatchShapeV1, M1PhysicalQueueBatchCustodyV1,
    M1PhysicalReadbackDetachedQueueSessionV1, M1PhysicalReadbackQueueOperationFailureV1,
    M1PrepareFailureV1, M1PreparedScheduledWorkspaceImagesV1, M1ReleasedCompletedStepV1,
    M1ReleasedDeviceKvMemberV1, M1S1K4RolloverOutputPortfolioStateV1, M1ScheduledDispatchV1,
    M1ServingBatchPlanV1, M1ServingPlanV1, M1ServingQueueActionV1, M1ServingRolloverReasonV1,
};

/// Stable rejection or fail-stop diagnostic for exact S1/K4 scheduling.
#[derive(Debug)]
pub enum M1S1K4QueueRolloverScheduleErrorV1 {
    Action,
    UnsupportedTransition,
    EpochExhausted,
    EpochNotExactNext {
        expected: CompletionEpoch,
        actual: CompletionEpoch,
    },
    Roster,
    QueueShape,
    QueueSelection,
    OutputReserve(M1S1K4RolloverOutputPortfolioStateV1),
    MemberCustody,
    EngineFaulted,
    RequestNotReady,
    Detach,
    ExactDispatch(M1ExactDispatchErrorV1),
    CacheReselection(DeviceKvCacheError),
}

/// Inert observations and member custody separated from a detached queue.
#[must_use = "rollover residue must remain paired with its queue phase"]
#[derive(Debug)]
pub struct M1S1K4QueueRolloverResidueV1 {
    checked: M1CheckedCompletionOutputV1,
    members: Vec<M1ReleasedDeviceKvMemberV1>,
    logical_accepted_counts: Box<[u32]>,
    externally_published_counts: Box<[u32]>,
    release_counts: Box<[M1CompletedKvPageReleaseCountsV1]>,
    completed_members: usize,
    total_released: usize,
}

pub(crate) struct M1S1K4QueueRolloverResiduePartsV1 {
    pub(crate) checked: M1CheckedCompletionOutputV1,
    pub(crate) logical_accepted_counts: Box<[u32]>,
    pub(crate) externally_published_counts: Box<[u32]>,
    pub(crate) release_counts: Box<[M1CompletedKvPageReleaseCountsV1]>,
    pub(crate) completed_members: usize,
    pub(crate) total_released: usize,
}

impl M1S1K4QueueRolloverResidueV1 {
    pub const fn checked(&self) -> &M1CheckedCompletionOutputV1 {
        &self.checked
    }

    pub fn members(&self) -> &[M1ReleasedDeviceKvMemberV1] {
        &self.members
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

    pub(crate) fn into_parts(self) -> M1S1K4QueueRolloverResiduePartsV1 {
        debug_assert!(self.members.is_empty());
        M1S1K4QueueRolloverResiduePartsV1 {
            checked: self.checked,
            logical_accepted_counts: self.logical_accepted_counts,
            externally_published_counts: self.externally_published_counts,
            release_counts: self.release_counts,
            completed_members: self.completed_members,
            total_released: self.total_released,
        }
    }
}

/// Exhaustive custody retained by one failed schedule transition.
#[must_use = "failed rollover scheduling retains every available owner"]
#[derive(Debug)]
pub enum M1S1K4QueueRolloverScheduleFailureCustodyV1 {
    /// Pure preflight rejection retains the original released owner.
    Released(Box<M1ReleasedCompletedStepV1>),
    /// Queue detachment failed terminally and retains generic quarantine.
    Detach {
        source: M1PhysicalReadbackQueueOperationFailureV1,
        residue: Box<M1S1K4QueueRolloverResidueV1>,
    },
    /// Detachment succeeded; optional scheduler authority records later progress.
    Detached {
        queue: Box<M1PhysicalReadbackDetachedQueueSessionV1>,
        scheduled: Option<Box<M1ScheduledDispatchV1>>,
        residue: Box<M1S1K4QueueRolloverResidueV1>,
    },
}

/// Retryable-before-detach or terminal-after-detach schedule failure.
#[must_use = "schedule failure custody must be retried or quarantined"]
#[derive(Debug)]
pub struct M1S1K4QueueRolloverScheduleFailureV1 {
    error: M1S1K4QueueRolloverScheduleErrorV1,
    terminal: bool,
    custody: M1S1K4QueueRolloverScheduleFailureCustodyV1,
}

impl M1S1K4QueueRolloverScheduleFailureV1 {
    #[must_use]
    pub const fn error(&self) -> &M1S1K4QueueRolloverScheduleErrorV1 {
        &self.error
    }

    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        self.terminal
    }

    #[must_use = "all retained rollover custody remains linear"]
    pub fn into_custody(self) -> M1S1K4QueueRolloverScheduleFailureCustodyV1 {
        self.custody
    }

    /// Closes a failed post-detach transition while preserving an intact
    /// released owner for retry.
    pub fn close_terminal(self) -> M1S1K4QueueRolloverScheduleClosureOutcomeV1 {
        match self {
            failure @ Self {
                custody: M1S1K4QueueRolloverScheduleFailureCustodyV1::Released(_),
                ..
            } => M1S1K4QueueRolloverScheduleClosureOutcomeV1::Released(failure),
            Self {
                error,
                custody: M1S1K4QueueRolloverScheduleFailureCustodyV1::Detach { source, residue },
                ..
            } => M1S1K4QueueRolloverScheduleClosureOutcomeV1::Detach(
                M1S1K4QueueRolloverScheduleDetachQuarantineV1 {
                    error,
                    source,
                    residue,
                },
            ),
            Self {
                error,
                custody:
                    M1S1K4QueueRolloverScheduleFailureCustodyV1::Detached {
                        queue,
                        scheduled,
                        residue,
                    },
                ..
            } => {
                let (shape, lower, batch_custody) = (*queue).into_rearm_parts();
                let custody = M1S1K4QueueRolloverScheduleDetachedCustodyV1 {
                    error,
                    shape,
                    batch_custody,
                    scheduled,
                    residue,
                };
                M1S1K4QueueRolloverScheduleClosureOutcomeV1::Detached(Box::new(
                    match lower.destroy_and_release() {
                        Ok(queue_release) => {
                            Ok(M1S1K4QueueRolloverScheduleDetachedTeardownSuccessV1 {
                                queue_release,
                                custody,
                            })
                        }
                        Err(source) => Err(Box::new(
                            M1S1K4QueueRolloverScheduleDetachedTeardownFailureV1 {
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

/// Exhaustive closure of a failed S1/K4 scheduling transition.
#[must_use = "rollover schedule closure retains every phase-local owner"]
#[derive(Debug)]
pub enum M1S1K4QueueRolloverScheduleClosureOutcomeV1 {
    Released(M1S1K4QueueRolloverScheduleFailureV1),
    Detach(M1S1K4QueueRolloverScheduleDetachQuarantineV1),
    Detached(
        Box<
            Result<
                M1S1K4QueueRolloverScheduleDetachedTeardownSuccessV1,
                Box<M1S1K4QueueRolloverScheduleDetachedTeardownFailureV1>,
            >,
        >,
    ),
}

/// Lower detach quarantine retaining exact released-step residue.
#[must_use = "detach quarantine and rollover residue remain retained"]
#[derive(Debug)]
pub struct M1S1K4QueueRolloverScheduleDetachQuarantineV1 {
    error: M1S1K4QueueRolloverScheduleErrorV1,
    source: M1PhysicalReadbackQueueOperationFailureV1,
    residue: Box<M1S1K4QueueRolloverResidueV1>,
}

impl M1S1K4QueueRolloverScheduleDetachQuarantineV1 {
    #[must_use]
    pub const fn error(&self) -> &M1S1K4QueueRolloverScheduleErrorV1 {
        &self.error
    }

    pub const fn source(&self) -> &M1PhysicalReadbackQueueOperationFailureV1 {
        &self.source
    }

    pub const fn residue(&self) -> &M1S1K4QueueRolloverResidueV1 {
        &self.residue
    }
}

#[derive(Debug)]
struct M1S1K4QueueRolloverScheduleDetachedCustodyV1 {
    error: M1S1K4QueueRolloverScheduleErrorV1,
    shape: M1PhysicalFixedBatchShapeV1,
    batch_custody: M1PhysicalQueueBatchCustodyV1,
    scheduled: Option<Box<M1ScheduledDispatchV1>>,
    residue: Box<M1S1K4QueueRolloverResidueV1>,
}

/// Clean release after a failed post-detach rollover schedule.
#[must_use = "detached rollover scheduling residue remains retained"]
#[derive(Debug)]
pub struct M1S1K4QueueRolloverScheduleDetachedTeardownSuccessV1 {
    queue_release: ServiceQueueReleaseObservationV1,
    custody: M1S1K4QueueRolloverScheduleDetachedCustodyV1,
}

/// Terminal lower release failure retaining failed rollover schedule custody.
#[must_use = "detached rollover release quarantine remains retained"]
#[derive(Debug)]
pub struct M1S1K4QueueRolloverScheduleDetachedTeardownFailureV1 {
    source: ServiceQueueReleaseFailureV1,
    custody: M1S1K4QueueRolloverScheduleDetachedCustodyV1,
}

impl M1S1K4QueueRolloverScheduleDetachedTeardownSuccessV1 {
    #[must_use]
    pub const fn queue_release(&self) -> ServiceQueueReleaseObservationV1 {
        self.queue_release
    }

    #[must_use]
    pub const fn shape(&self) -> M1PhysicalFixedBatchShapeV1 {
        self.custody.shape
    }

    #[must_use]
    pub const fn error(&self) -> &M1S1K4QueueRolloverScheduleErrorV1 {
        &self.custody.error
    }

    #[must_use]
    pub const fn has_scheduled_dispatch(&self) -> bool {
        self.custody.scheduled.is_some()
    }

    pub const fn residue(&self) -> &M1S1K4QueueRolloverResidueV1 {
        &self.custody.residue
    }

    pub const fn batch_custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        &self.custody.batch_custody
    }
}

impl M1S1K4QueueRolloverScheduleDetachedTeardownFailureV1 {
    pub const fn source(&self) -> &ServiceQueueReleaseFailureV1 {
        &self.source
    }

    #[must_use]
    pub const fn shape(&self) -> M1PhysicalFixedBatchShapeV1 {
        self.custody.shape
    }

    #[must_use]
    pub const fn error(&self) -> &M1S1K4QueueRolloverScheduleErrorV1 {
        &self.custody.error
    }

    #[must_use]
    pub const fn has_scheduled_dispatch(&self) -> bool {
        self.custody.scheduled.is_some()
    }

    pub const fn residue(&self) -> &M1S1K4QueueRolloverResidueV1 {
        &self.custody.residue
    }

    pub const fn batch_custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        &self.custody.batch_custody
    }
}

/// Detached predecessor, exact successor dispatch, and reselected live cache.
#[must_use = "scheduled rollover custody must reserve and prepare the successor batch"]
#[derive(Debug)]
pub struct M1ScheduledS1K4QueueRolloverV1 {
    prior: M1ServingPlanV1,
    next: M1ServingPlanV1,
    reason: M1ServingRolloverReasonV1,
    queue: M1PhysicalReadbackDetachedQueueSessionV1,
    scheduled: M1ScheduledDispatchV1,
    selected: ActiveDeviceKvCache,
    residue: M1S1K4QueueRolloverResidueV1,
}

impl M1ScheduledS1K4QueueRolloverV1 {
    #[must_use]
    pub const fn prior_plan(&self) -> M1ServingPlanV1 {
        self.prior
    }

    #[must_use]
    pub const fn next_plan(&self) -> M1ServingPlanV1 {
        self.next
    }

    #[must_use]
    pub const fn reason(&self) -> M1ServingRolloverReasonV1 {
        self.reason
    }

    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        &self.scheduled
    }

    #[must_use]
    pub fn selected_cache(&self) -> DeviceKvCacheProjection {
        self.selected.projection()
    }

    #[must_use]
    pub const fn predecessor_dispatch_generation(&self) -> u64 {
        self.queue.detached_dispatch_generation()
    }

    #[must_use = "completed predecessor observations remain retained"]
    pub const fn residue(&self) -> &M1S1K4QueueRolloverResidueV1 {
        &self.residue
    }

    /// Destroys the detached predecessor while retaining the successor
    /// scheduler dispatch, reselected cache, and all predecessor observations.
    ///
    /// # Errors
    ///
    /// Returns lower release quarantine joined to every retained owner.
    pub fn destroy_queue_and_retain_round<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1ScheduledS1K4QueueRolloverTeardownSuccessV1,
        Box<M1ScheduledS1K4QueueRolloverTeardownFailureV1>,
    > {
        engine.quarantine_m1_queue_rearm_failure();
        let Self {
            prior,
            next,
            reason,
            queue,
            scheduled,
            selected,
            residue,
        } = self;
        let (shape, lower, batch_custody) = queue.into_rearm_parts();
        let custody = M1ScheduledS1K4QueueRolloverTeardownCustodyV1 {
            prior,
            next,
            reason,
            shape,
            batch_custody,
            scheduled,
            selected,
            residue,
        };
        match lower.destroy_and_release() {
            Ok(queue_release) => Ok(M1ScheduledS1K4QueueRolloverTeardownSuccessV1 {
                queue_release,
                custody,
            }),
            Err(source) => Err(Box::new(M1ScheduledS1K4QueueRolloverTeardownFailureV1 {
                source,
                custody,
            })),
        }
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        M1ServingPlanV1,
        M1ServingPlanV1,
        M1ServingRolloverReasonV1,
        M1PhysicalReadbackDetachedQueueSessionV1,
        M1ScheduledDispatchV1,
        ActiveDeviceKvCache,
        M1S1K4QueueRolloverResidueV1,
    ) {
        (
            self.prior,
            self.next,
            self.reason,
            self.queue,
            self.scheduled,
            self.selected,
            self.residue,
        )
    }
}

#[derive(Debug)]
struct M1ScheduledS1K4QueueRolloverTeardownCustodyV1 {
    prior: M1ServingPlanV1,
    next: M1ServingPlanV1,
    reason: M1ServingRolloverReasonV1,
    shape: M1PhysicalFixedBatchShapeV1,
    batch_custody: M1PhysicalQueueBatchCustodyV1,
    scheduled: M1ScheduledDispatchV1,
    selected: ActiveDeviceKvCache,
    residue: M1S1K4QueueRolloverResidueV1,
}

/// Clean teardown of an already scheduled S1/K4 rollover round.
#[must_use = "scheduled rollover round custody remains retained"]
#[derive(Debug)]
pub struct M1ScheduledS1K4QueueRolloverTeardownSuccessV1 {
    queue_release: ServiceQueueReleaseObservationV1,
    custody: M1ScheduledS1K4QueueRolloverTeardownCustodyV1,
}

/// Terminal lower release failure retaining an already scheduled rollover.
#[must_use = "scheduled rollover release quarantine remains retained"]
#[derive(Debug)]
pub struct M1ScheduledS1K4QueueRolloverTeardownFailureV1 {
    source: ServiceQueueReleaseFailureV1,
    custody: M1ScheduledS1K4QueueRolloverTeardownCustodyV1,
}

impl M1ScheduledS1K4QueueRolloverTeardownCustodyV1 {
    const fn plan_transition(
        &self,
    ) -> (M1ServingPlanV1, M1ServingPlanV1, M1ServingRolloverReasonV1) {
        (self.prior, self.next, self.reason)
    }
}

impl M1ScheduledS1K4QueueRolloverTeardownSuccessV1 {
    #[must_use]
    pub const fn queue_release(&self) -> ServiceQueueReleaseObservationV1 {
        self.queue_release
    }

    #[must_use]
    pub const fn shape(&self) -> M1PhysicalFixedBatchShapeV1 {
        self.custody.shape
    }

    #[must_use]
    pub const fn plan_transition(
        &self,
    ) -> (M1ServingPlanV1, M1ServingPlanV1, M1ServingRolloverReasonV1) {
        self.custody.plan_transition()
    }

    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        &self.custody.scheduled
    }

    #[must_use]
    pub fn selected_cache(&self) -> DeviceKvCacheProjection {
        self.custody.selected.projection()
    }

    pub const fn residue(&self) -> &M1S1K4QueueRolloverResidueV1 {
        &self.custody.residue
    }

    pub const fn batch_custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        &self.custody.batch_custody
    }
}

impl M1ScheduledS1K4QueueRolloverTeardownFailureV1 {
    pub const fn source(&self) -> &ServiceQueueReleaseFailureV1 {
        &self.source
    }

    #[must_use]
    pub const fn shape(&self) -> M1PhysicalFixedBatchShapeV1 {
        self.custody.shape
    }

    #[must_use]
    pub const fn plan_transition(
        &self,
    ) -> (M1ServingPlanV1, M1ServingPlanV1, M1ServingRolloverReasonV1) {
        self.custody.plan_transition()
    }

    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        &self.custody.scheduled
    }

    #[must_use]
    pub fn selected_cache(&self) -> DeviceKvCacheProjection {
        self.custody.selected.projection()
    }

    pub const fn residue(&self) -> &M1S1K4QueueRolloverResidueV1 {
        &self.custody.residue
    }

    pub const fn batch_custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        &self.custody.batch_custody
    }
}

/// Exact S1/K4 next-round inputs retained independently of queue custody.
#[must_use = "rollover KV inputs contain linear page leases"]
#[derive(Debug, Eq, PartialEq)]
pub struct M1S1K4QueueRolloverKvInputsV1 {
    draft_decode: ValidatedM1StepInputs,
    target_speculative: ValidatedM1StepInputs,
    draft_page_leases: Vec<DeviceKvPageLease>,
    target_page_leases: Vec<DeviceKvPageLease>,
}

impl M1S1K4QueueRolloverKvInputsV1 {
    pub const fn new(
        draft_decode: ValidatedM1StepInputs,
        target_speculative: ValidatedM1StepInputs,
        draft_page_leases: Vec<DeviceKvPageLease>,
        target_page_leases: Vec<DeviceKvPageLease>,
    ) -> Self {
        Self {
            draft_decode,
            target_speculative,
            draft_page_leases,
            target_page_leases,
        }
    }
}

/// Reservation or table-binding stage of an S1/K4 rollover failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1S1K4QueueRolloverKvReservationPhaseV1 {
    Preflight,
    DraftReservation,
    TargetReservation,
    TargetTableBinding,
    DraftTableBinding,
}

#[derive(Debug)]
struct OpaqueS1K4RolloverCustodyV1(Box<dyn fmt::Debug>);

/// Fail-stop custody after successor dispatch and KV reservation begin.
#[must_use = "rollover reservation failure retains all available linear custody"]
#[derive(Debug)]
pub struct M1S1K4QueueRolloverKvReservationFailureV1 {
    phase: M1S1K4QueueRolloverKvReservationPhaseV1,
    retained: OpaqueS1K4RolloverCustodyV1,
}

impl M1S1K4QueueRolloverKvReservationFailureV1 {
    #[must_use]
    pub const fn phase(&self) -> M1S1K4QueueRolloverKvReservationPhaseV1 {
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
    phase: M1S1K4QueueRolloverKvReservationPhaseV1,
    retained: impl fmt::Debug + 'static,
) -> M1S1K4QueueRolloverKvReservationFailureV1 {
    M1S1K4QueueRolloverKvReservationFailureV1 {
        phase,
        retained: OpaqueS1K4RolloverCustodyV1(Box::new(retained)),
    }
}

fn exact_s1_input_matches(
    input: &ValidatedM1StepInputs,
    selection: ferric_spec::Qwen3PlanSelection,
    scheduled: &M1ScheduledDispatchV1,
    request: ferric_spec::RequestId,
) -> bool {
    input.selection() == selection
        && input.live_lane_count() == 1
        && input.lanes()[0].is_some_and(|lane| {
            lane.request() == request && lane.completion_epoch() == scheduled.epoch()
        })
}

/// Successor KV tables joined to one detached predecessor and exact dispatch.
#[must_use = "reserved rollover custody must prepare fresh workspace images"]
#[derive(Debug)]
pub struct M1ReservedS1K4QueueRolloverV1 {
    prior: M1ServingPlanV1,
    next: M1ServingPlanV1,
    reason: M1ServingRolloverReasonV1,
    queue: M1PhysicalReadbackDetachedQueueSessionV1,
    scheduled: M1ScheduledDispatchV1,
    selected: ActiveDeviceKvCache,
    tables: M1FullStepKvWorkspaceTablesV1,
    residue: M1S1K4QueueRolloverResidueV1,
}

impl M1ReservedS1K4QueueRolloverV1 {
    #[must_use]
    pub const fn next_plan(&self) -> M1ServingPlanV1 {
        self.next
    }

    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        &self.scheduled
    }

    #[must_use]
    pub fn selected_cache(&self) -> DeviceKvCacheProjection {
        self.selected.projection()
    }
}

fn reserve_m1_s1_k4_queue_rollover_kv_inner_v1(
    scheduled: M1ScheduledS1K4QueueRolloverV1,
    inputs: M1S1K4QueueRolloverKvInputsV1,
) -> Result<M1ReservedS1K4QueueRolloverV1, M1S1K4QueueRolloverKvReservationFailureV1> {
    let (prior, next, reason, queue, scheduled, mut selected, residue) = scheduled.into_parts();
    let M1S1K4QueueRolloverKvInputsV1 {
        draft_decode,
        target_speculative,
        draft_page_leases,
        target_page_leases,
    } = inputs;
    let request = selected.projection().request;
    if queue.shape() != M1PhysicalFixedBatchShapeV1::PairedPrefill
        || scheduled.member_count() != 1
        || scheduled.member(0) != Some(request)
        || !exact_s1_input_matches(&draft_decode, next.draft(), &scheduled, request)
        || !exact_s1_input_matches(&target_speculative, next.target(), &scheduled, request)
    {
        return Err(kv_reservation_failure(
            M1S1K4QueueRolloverKvReservationPhaseV1::Preflight,
            (
                prior,
                next,
                reason,
                queue,
                scheduled,
                selected,
                residue,
                draft_decode,
                target_speculative,
                draft_page_leases,
                target_page_leases,
            ),
        ));
    }

    let draft_reservation = match selected.reserve_speculative_draft_round_write(
        request,
        next.target(),
        next.draft(),
        draft_decode.context_lengths()[0],
        scheduled.epoch(),
        draft_page_leases,
    ) {
        Ok(reservation) => reservation,
        Err(failure) => {
            let (source, draft_page_leases) = (*failure).into_parts();
            return Err(kv_reservation_failure(
                M1S1K4QueueRolloverKvReservationPhaseV1::DraftReservation,
                (
                    prior,
                    next,
                    reason,
                    queue,
                    scheduled,
                    selected,
                    residue,
                    draft_decode,
                    target_speculative,
                    draft_page_leases,
                    target_page_leases,
                    source,
                ),
            ));
        }
    };
    let target_reservation = match selected.reserve_step_write(
        request,
        ferric_spec::Qwen3ModelRole::Target8B,
        target_speculative.context_lengths()[0],
        target_speculative.active_lengths()[0],
        scheduled.epoch(),
        target_page_leases,
    ) {
        Ok(reservation) => reservation,
        Err(failure) => {
            let (source, target_page_leases) = (*failure).into_parts();
            return Err(kv_reservation_failure(
                M1S1K4QueueRolloverKvReservationPhaseV1::TargetReservation,
                (
                    prior,
                    next,
                    reason,
                    queue,
                    scheduled,
                    selected,
                    residue,
                    draft_decode,
                    target_speculative,
                    draft_reservation,
                    target_page_leases,
                    source,
                ),
            ));
        }
    };
    let target =
        match crate::bind_m1_kv_workspace_table_v1(target_speculative, vec![target_reservation]) {
            Ok(table) => table,
            Err(failure) => {
                return Err(kv_reservation_failure(
                    M1S1K4QueueRolloverKvReservationPhaseV1::TargetTableBinding,
                    (
                        prior,
                        next,
                        reason,
                        queue,
                        scheduled,
                        selected,
                        residue,
                        draft_decode,
                        draft_reservation,
                        failure,
                    ),
                ));
            }
        };
    let draft = match crate::bind_m1_speculative_draft_kv_round_workspace_table_v1(
        next.target(),
        draft_decode,
        vec![draft_reservation],
    ) {
        Ok(table) => table,
        Err(failure) => {
            return Err(kv_reservation_failure(
                M1S1K4QueueRolloverKvReservationPhaseV1::DraftTableBinding,
                (
                    prior, next, reason, queue, scheduled, selected, residue, target, failure,
                ),
            ));
        }
    };
    Ok(M1ReservedS1K4QueueRolloverV1 {
        prior,
        next,
        reason,
        queue,
        scheduled,
        selected,
        tables: M1FullStepKvWorkspaceTablesV1::SpeculativeRound {
            draft_decode: draft,
            target_speculative: target,
        },
        residue,
    })
}

/// Installs exact successor KV reservations and faults the Engine on failure.
///
/// # Errors
///
/// Returns terminal retained custody after successor scheduling has advanced.
pub fn reserve_m1_s1_k4_queue_rollover_kv_v1<const C: usize>(
    engine: &mut Engine<C>,
    scheduled: M1ScheduledS1K4QueueRolloverV1,
    inputs: M1S1K4QueueRolloverKvInputsV1,
) -> Result<M1ReservedS1K4QueueRolloverV1, M1S1K4QueueRolloverKvReservationFailureV1> {
    match reserve_m1_s1_k4_queue_rollover_kv_inner_v1(scheduled, inputs) {
        Ok(reserved) => Ok(reserved),
        Err(failure) => {
            engine.quarantine_m1_queue_rearm_failure();
            Err(failure)
        }
    }
}

#[derive(Debug)]
struct M1S1K4QueueRolloverPreparedRemainderV1 {
    prior: M1ServingPlanV1,
    next: M1ServingPlanV1,
    reason: M1ServingRolloverReasonV1,
    queue: M1PhysicalReadbackDetachedQueueSessionV1,
    selected: ActiveDeviceKvCache,
    residue: M1S1K4QueueRolloverResidueV1,
}

/// Fresh S1/K4 workspace images retained with detached rollover custody.
#[must_use = "prepared rollover custody must replace and publish the queue"]
#[derive(Debug)]
pub struct M1PreparedS1K4QueueRolloverV1 {
    prepared: M1PreparedScheduledWorkspaceImagesV1,
    remainder: M1S1K4QueueRolloverPreparedRemainderV1,
}

impl M1PreparedS1K4QueueRolloverV1 {
    #[must_use]
    pub const fn next_epoch(&self) -> CompletionEpoch {
        self.prepared.step().scheduled_dispatch().epoch()
    }

    #[must_use]
    pub const fn prior_plan(&self) -> M1ServingPlanV1 {
        self.remainder.prior
    }

    #[must_use]
    pub const fn next_plan(&self) -> M1ServingPlanV1 {
        self.remainder.next
    }

    #[must_use]
    pub const fn reason(&self) -> M1ServingRolloverReasonV1 {
        self.remainder.reason
    }

    #[must_use]
    pub const fn predecessor_dispatch_generation(&self) -> u64 {
        self.remainder.queue.detached_dispatch_generation()
    }

    #[must_use]
    pub fn selected_cache(&self) -> DeviceKvCacheProjection {
        self.remainder.selected.projection()
    }

    pub const fn residue(&self) -> &M1S1K4QueueRolloverResidueV1 {
        &self.remainder.residue
    }

    pub(crate) fn into_submission_parts(
        self,
    ) -> (
        M1PreparedScheduledWorkspaceImagesV1,
        M1ServingPlanV1,
        M1ServingPlanV1,
        M1ServingRolloverReasonV1,
        M1PhysicalReadbackDetachedQueueSessionV1,
        ActiveDeviceKvCache,
        M1S1K4QueueRolloverResidueV1,
    ) {
        (
            self.prepared,
            self.remainder.prior,
            self.remainder.next,
            self.remainder.reason,
            self.remainder.queue,
            self.remainder.selected,
            self.remainder.residue,
        )
    }
}

/// Workspace-image preparation rejection with complete detached custody.
#[must_use = "rollover preparation failure retains all available linear custody"]
#[derive(Debug)]
pub struct M1S1K4QueueRolloverPrepareFailureV1 {
    source: M1PrepareFailureV1,
    remainder: Box<M1S1K4QueueRolloverPreparedRemainderV1>,
}

impl M1S1K4QueueRolloverPrepareFailureV1 {
    pub const fn source(&self) -> &M1PrepareFailureV1 {
        &self.source
    }

    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        true
    }

    #[must_use]
    pub const fn next_plan(&self) -> M1ServingPlanV1 {
        self.remainder.next
    }
}

fn prepare_m1_s1_k4_queue_rollover_inner_v1(
    reserved: M1ReservedS1K4QueueRolloverV1,
    runner: &LogicalRunnerDeclaration,
    plans: M1FullStepWorkspacePlans,
) -> Result<M1PreparedS1K4QueueRolloverV1, Box<M1S1K4QueueRolloverPrepareFailureV1>> {
    let M1ReservedS1K4QueueRolloverV1 {
        prior,
        next,
        reason,
        queue,
        scheduled,
        selected,
        tables,
        residue,
    } = reserved;
    let remainder = M1S1K4QueueRolloverPreparedRemainderV1 {
        prior,
        next,
        reason,
        queue,
        selected,
        residue,
    };
    match crate::prepare_m1_scheduled_workspace_images_v1(scheduled, runner, plans, tables) {
        Ok(prepared) => Ok(M1PreparedS1K4QueueRolloverV1 {
            prepared,
            remainder,
        }),
        Err(source) => Err(Box::new(M1S1K4QueueRolloverPrepareFailureV1 {
            source,
            remainder: Box::new(remainder),
        })),
    }
}

/// Prepares exact S1/K4 workspace images after cross-shape KV reservation.
///
/// # Errors
///
/// Returns terminal detached custody and faults the Engine on rejection.
pub fn prepare_m1_s1_k4_queue_rollover_v1<const C: usize>(
    engine: &mut Engine<C>,
    reserved: M1ReservedS1K4QueueRolloverV1,
    runner: &LogicalRunnerDeclaration,
    plans: M1FullStepWorkspacePlans,
) -> Result<M1PreparedS1K4QueueRolloverV1, Box<M1S1K4QueueRolloverPrepareFailureV1>> {
    match prepare_m1_s1_k4_queue_rollover_inner_v1(reserved, runner, plans) {
        Ok(prepared) => Ok(prepared),
        Err(failure) => {
            engine.quarantine_m1_queue_rearm_failure();
            Err(failure)
        }
    }
}

fn exact_next_epoch(epoch: CompletionEpoch) -> Option<CompletionEpoch> {
    epoch.value().checked_add(1).map(CompletionEpoch::new)
}

fn exact_s1_k4_transition(
    batch: &M1ServingBatchPlanV1,
) -> Result<
    (M1ServingPlanV1, M1ServingPlanV1, M1ServingRolloverReasonV1),
    M1S1K4QueueRolloverScheduleErrorV1,
> {
    let M1ServingQueueActionV1::QuiescentRollover {
        prior,
        next,
        reason,
    } = batch.action()
    else {
        return Err(M1S1K4QueueRolloverScheduleErrorV1::Action);
    };

    validate_s1_k4_transition(prior, next, reason)?;
    Ok((prior, next, reason))
}

fn validate_s1_k4_transition(
    prior: M1ServingPlanV1,
    next: M1ServingPlanV1,
    reason: M1ServingRolloverReasonV1,
) -> Result<(), M1S1K4QueueRolloverScheduleErrorV1> {
    let prior_is_s1_prefill = prior.shape() == M1PhysicalFixedBatchShapeV1::PairedPrefill
        && prior.mode() == Qwen3ExecutionMode::Prefill
        && prior.sequence_capacity() == 1;
    let next_is_exact_s1_k4 = next.shape() == M1PhysicalFixedBatchShapeV1::SpeculativeK4
        && next.target().mode == Qwen3ExecutionMode::Speculative
        && next.target().bucket == Qwen3PlanBucket::SpeculativeS1K4C8192
        && next.draft().mode == Qwen3ExecutionMode::Decode
        && next.draft().bucket == Qwen3PlanBucket::DecodeS1C8192
        && next.sequence_capacity() == 1;
    if !prior_is_s1_prefill || !next_is_exact_s1_k4 || reason != M1ServingRolloverReasonV1::Mode {
        return Err(M1S1K4QueueRolloverScheduleErrorV1::UnsupportedTransition);
    }
    Ok(())
}

fn preflight_released<const C: usize>(
    engine: &Engine<C>,
    released: &M1ReleasedCompletedStepV1,
    batch: &M1ServingBatchPlanV1,
) -> Result<
    (M1ServingPlanV1, M1ServingPlanV1, M1ServingRolloverReasonV1),
    M1S1K4QueueRolloverScheduleErrorV1,
> {
    let plans = exact_s1_k4_transition(batch)?;
    let (prior, next, _) = plans;
    let Some(expected_epoch) = exact_next_epoch(released.checked().epoch()) else {
        return Err(M1S1K4QueueRolloverScheduleErrorV1::EpochExhausted);
    };
    if batch.epoch() != expected_epoch {
        return Err(M1S1K4QueueRolloverScheduleErrorV1::EpochNotExactNext {
            expected: expected_epoch,
            actual: batch.epoch(),
        });
    }
    let [request] = batch.requests() else {
        return Err(M1S1K4QueueRolloverScheduleErrorV1::Roster);
    };
    if released.queue().shape() != M1PhysicalFixedBatchShapeV1::PairedPrefill {
        return Err(M1S1K4QueueRolloverScheduleErrorV1::QueueShape);
    }
    let queue_custody = released.queue().custody();
    if queue_custody.selection() != prior.target()
        || released.checked().selection() != prior.target()
    {
        return Err(M1S1K4QueueRolloverScheduleErrorV1::QueueSelection);
    }
    let reserve_state = queue_custody
        .partitioned_memory()
        .s1_k4_rollover_output_state();
    if reserve_state != M1S1K4RolloverOutputPortfolioStateV1::Reserved {
        return Err(M1S1K4QueueRolloverScheduleErrorV1::OutputReserve(
            reserve_state,
        ));
    }
    let [M1ReleasedDeviceKvMemberV1::Active(cache)] = released.members() else {
        return Err(M1S1K4QueueRolloverScheduleErrorV1::MemberCustody);
    };
    if cache.projection().request != *request {
        return Err(M1S1K4QueueRolloverScheduleErrorV1::MemberCustody);
    }
    cache
        .preflight_quiescent_reselection(next.target(), next.draft_cache_selection())
        .map_err(M1S1K4QueueRolloverScheduleErrorV1::CacheReselection)?;
    if engine.is_faulted() {
        return Err(M1S1K4QueueRolloverScheduleErrorV1::EngineFaulted);
    }
    if engine.state(*request) != Some(RequestState::Ready) {
        return Err(M1S1K4QueueRolloverScheduleErrorV1::RequestNotReady);
    }
    Ok(plans)
}

fn released_failure(
    error: M1S1K4QueueRolloverScheduleErrorV1,
    released: M1ReleasedCompletedStepV1,
) -> M1S1K4QueueRolloverScheduleFailureV1 {
    M1S1K4QueueRolloverScheduleFailureV1 {
        error,
        terminal: false,
        custody: M1S1K4QueueRolloverScheduleFailureCustodyV1::Released(Box::new(released)),
    }
}

fn terminal_failure<const C: usize>(
    engine: &mut Engine<C>,
    error: M1S1K4QueueRolloverScheduleErrorV1,
    custody: M1S1K4QueueRolloverScheduleFailureCustodyV1,
) -> M1S1K4QueueRolloverScheduleFailureV1 {
    engine.quarantine_m1_queue_rearm_failure();
    M1S1K4QueueRolloverScheduleFailureV1 {
        error,
        terminal: true,
        custody,
    }
}

/// Detaches one completed S1 paired-prefill queue, dispatches the exact next
/// roster once, and reselects its sole live cache for S1/K4.
///
/// # Errors
///
/// Every rejection before detachment returns the original released owner.
/// Detach, exact-dispatch, or post-dispatch reselection failure quarantines the
/// Engine and returns all available detached/scheduler/member custody.
pub fn schedule_m1_s1_k4_queue_rollover_exact_v1<const C: usize>(
    engine: &mut Engine<C>,
    released: M1ReleasedCompletedStepV1,
    batch: &M1ServingBatchPlanV1,
) -> Result<M1ScheduledS1K4QueueRolloverV1, Box<M1S1K4QueueRolloverScheduleFailureV1>> {
    let (prior, next, reason) = match preflight_released(engine, &released, batch) {
        Ok(plans) => plans,
        Err(error) => return Err(Box::new(released_failure(error, released))),
    };
    let (
        queue,
        checked,
        members,
        logical_accepted_counts,
        externally_published_counts,
        release_counts,
        completed_members,
        total_released,
    ) = released.into_rearm_parts();
    let mut residue = M1S1K4QueueRolloverResidueV1 {
        checked,
        members,
        logical_accepted_counts,
        externally_published_counts,
        release_counts,
        completed_members,
        total_released,
    };
    let queue = match queue.detach() {
        Ok(queue) => queue,
        Err(source) => {
            return Err(Box::new(terminal_failure(
                engine,
                M1S1K4QueueRolloverScheduleErrorV1::Detach,
                M1S1K4QueueRolloverScheduleFailureCustodyV1::Detach {
                    source,
                    residue: Box::new(residue),
                },
            )));
        }
    };
    let scheduled = match engine.dispatch_m1_exact_ready(batch.epoch(), batch.requests()) {
        Ok(scheduled) => scheduled,
        Err(source) => {
            return Err(Box::new(terminal_failure(
                engine,
                M1S1K4QueueRolloverScheduleErrorV1::ExactDispatch(source),
                M1S1K4QueueRolloverScheduleFailureCustodyV1::Detached {
                    queue: Box::new(queue),
                    scheduled: None,
                    residue: Box::new(residue),
                },
            )));
        }
    };
    let Some(M1ReleasedDeviceKvMemberV1::Active(mut selected)) = residue.members.pop() else {
        unreachable!("exact one-member active custody was checked before detachment")
    };
    if let Err(source) = selected.reselect_quiescent(next.target(), next.draft_cache_selection()) {
        residue
            .members
            .push(M1ReleasedDeviceKvMemberV1::Active(selected));
        return Err(Box::new(terminal_failure(
            engine,
            M1S1K4QueueRolloverScheduleErrorV1::CacheReselection(source),
            M1S1K4QueueRolloverScheduleFailureCustodyV1::Detached {
                queue: Box::new(queue),
                scheduled: Some(Box::new(scheduled)),
                residue: Box::new(residue),
            },
        )));
    }
    Ok(M1ScheduledS1K4QueueRolloverV1 {
        prior,
        next,
        reason,
        queue,
        scheduled,
        selected,
        residue,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferric_spec::{Qwen3ModelRole, Qwen3PlanSelection};

    fn selection(
        role: Qwen3ModelRole,
        mode: Qwen3ExecutionMode,
        bucket: Qwen3PlanBucket,
    ) -> Qwen3PlanSelection {
        Qwen3PlanSelection { role, mode, bucket }
    }

    fn plan(mode: Qwen3ExecutionMode, bucket: Qwen3PlanBucket) -> M1ServingPlanV1 {
        let draft = match (mode, bucket) {
            (Qwen3ExecutionMode::Speculative, Qwen3PlanBucket::SpeculativeS1K4C8192) => selection(
                Qwen3ModelRole::Draft06B,
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS1C8192,
            ),
            _ => selection(Qwen3ModelRole::Draft06B, mode, bucket),
        };
        M1ServingPlanV1::new(selection(Qwen3ModelRole::Target8B, mode, bucket), draft).unwrap()
    }

    #[test]
    fn exact_transition_predicate_is_closed_to_s1_prefill_into_s1_k4() {
        let prior = plan(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128);
        let next = plan(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
        );
        assert_eq!(prior.sequence_capacity(), 1);
        assert_eq!(next.sequence_capacity(), 1);
        assert_eq!(next.draft().mode, Qwen3ExecutionMode::Decode);
        assert_eq!(
            next.draft_cache_selection().mode,
            Qwen3ExecutionMode::Speculative
        );
        assert!(validate_s1_k4_transition(prior, next, M1ServingRolloverReasonV1::Mode).is_ok());

        let wider_prior = plan(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS8T128);
        assert!(matches!(
            validate_s1_k4_transition(wider_prior, next, M1ServingRolloverReasonV1::Mode),
            Err(M1S1K4QueueRolloverScheduleErrorV1::UnsupportedTransition)
        ));
        assert!(matches!(
            validate_s1_k4_transition(prior, next, M1ServingRolloverReasonV1::Shape),
            Err(M1S1K4QueueRolloverScheduleErrorV1::UnsupportedTransition)
        ));

        let decode_prior = plan(Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS1C8192);
        assert!(matches!(
            validate_s1_k4_transition(decode_prior, next, M1ServingRolloverReasonV1::Mode),
            Err(M1S1K4QueueRolloverScheduleErrorV1::UnsupportedTransition)
        ));
    }

    #[test]
    fn exact_next_epoch_rejects_exhaustion() {
        assert_eq!(
            exact_next_epoch(CompletionEpoch::new(8)),
            Some(CompletionEpoch::new(9))
        );
        assert_eq!(exact_next_epoch(CompletionEpoch::new(u64::MAX)), None);
    }
}
