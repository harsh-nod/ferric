//! Exact cross-shape queue rollover from one S1 paired prefill into S1/K4.
//!
//! The scheduler owns preflight, detachment, exact dispatch, and quiescent KV
//! reselection. The preparation phase then reserves exact draft and target KV
//! writes and binds the successor workspace images for native rollover.

use core::fmt;

use fe2o3_service_host::{ServiceQueueReleaseFailureV1, ServiceQueueReleaseObservationV1};
use ferric_spec::{
    completion::CompletionEpoch, scheduling::RequestState, Qwen3PlanBucket, ValidatedM1StepInputs,
};

use crate::m1_serving_registry::admit_m1_production_rollover_transition_v1;
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
    selected: Vec<ActiveDeviceKvCache>,
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
        self.selected[0].projection()
    }

    /// Returns one roster-indexed selected cache projection.
    #[must_use]
    pub fn selected_cache_at(&self, lane: usize) -> Option<DeviceKvCacheProjection> {
        self.selected.get(lane).map(ActiveDeviceKvCache::projection)
    }

    /// Exact number of successor caches in scheduler order.
    #[must_use]
    pub fn selected_cache_count(&self) -> usize {
        self.selected.len()
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
        Vec<ActiveDeviceKvCache>,
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
    selected: Vec<ActiveDeviceKvCache>,
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
        self.custody.selected[0].projection()
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
        self.custody.selected[0].projection()
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
    page_leases: M1FiniteSpeculativeQueueRolloverPageLeasesV1,
}

#[derive(Debug, Eq, PartialEq)]
enum M1FiniteSpeculativeQueueRolloverPageLeasesV1 {
    ExactS1 {
        draft: Vec<DeviceKvPageLease>,
        target: Vec<DeviceKvPageLease>,
    },
    Roster {
        draft: Vec<Vec<DeviceKvPageLease>>,
        target: Vec<Vec<DeviceKvPageLease>>,
    },
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
            page_leases: M1FiniteSpeculativeQueueRolloverPageLeasesV1::ExactS1 {
                draft: draft_page_leases,
                target: target_page_leases,
            },
        }
    }

    /// Constructs exact roster-indexed finite-speculative rollover inputs.
    pub const fn from_lane_leases(
        draft_decode: ValidatedM1StepInputs,
        target_speculative: ValidatedM1StepInputs,
        draft_page_leases: Vec<Vec<DeviceKvPageLease>>,
        target_page_leases: Vec<Vec<DeviceKvPageLease>>,
    ) -> Self {
        Self {
            draft_decode,
            target_speculative,
            page_leases: M1FiniteSpeculativeQueueRolloverPageLeasesV1::Roster {
                draft: draft_page_leases,
                target: target_page_leases,
            },
        }
    }

    /// Whether both successor token streams start from the checked prefill token.
    #[must_use]
    pub fn matches_anchor(&self, anchor: ferric_spec::TokenId) -> bool {
        self.matches_anchor_at(0, anchor)
    }

    /// Whether every live successor lane starts from the ordered anchor.
    #[must_use]
    pub fn matches_anchors(&self, anchors: &[ferric_spec::TokenId]) -> bool {
        anchors.len() == self.draft_decode.live_lane_count() as usize
            && anchors
                .iter()
                .copied()
                .enumerate()
                .all(|(lane, anchor)| self.matches_anchor_at(lane, anchor))
    }

    /// Whether one exact live lane starts both role rows from `anchor`.
    #[must_use]
    pub fn matches_anchor_at(&self, lane: usize, anchor: ferric_spec::TokenId) -> bool {
        if lane >= self.draft_decode.live_lane_count() as usize
            || lane >= self.target_speculative.live_lane_count() as usize
        {
            return false;
        }
        let draft_width = self.draft_decode.dimensions().active_tokens as usize;
        let target_width = self.target_speculative.dimensions().active_tokens as usize;
        let draft_index = lane.checked_mul(draft_width);
        let target_index = lane.checked_mul(target_width);
        draft_index.and_then(|index| self.draft_decode.token_ids().get(index)) == Some(&anchor)
            && target_index.and_then(|index| self.target_speculative.token_ids().get(index))
                == Some(&anchor)
    }

    pub(crate) const fn validated_inputs(
        &self,
    ) -> (&ValidatedM1StepInputs, &ValidatedM1StepInputs) {
        (&self.draft_decode, &self.target_speculative)
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        ValidatedM1StepInputs,
        ValidatedM1StepInputs,
        Vec<Vec<DeviceKvPageLease>>,
        Vec<Vec<DeviceKvPageLease>>,
    ) {
        let (draft, target) = match self.page_leases {
            M1FiniteSpeculativeQueueRolloverPageLeasesV1::ExactS1 { draft, target } => {
                (vec![draft], vec![target])
            }
            M1FiniteSpeculativeQueueRolloverPageLeasesV1::Roster { draft, target } => {
                (draft, target)
            }
        };
        (self.draft_decode, self.target_speculative, draft, target)
    }
}

/// Generic finite-speculative rollover input custody.
pub type M1FiniteSpeculativeQueueRolloverKvInputsV1 = M1S1K4QueueRolloverKvInputsV1;

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

fn exact_roster_input_matches(
    input: &ValidatedM1StepInputs,
    selection: ferric_spec::Qwen3PlanSelection,
    scheduled: &M1ScheduledDispatchV1,
) -> bool {
    input.selection() == selection
        && input.live_lane_count() as usize == scheduled.member_count()
        && input.lanes().iter().enumerate().all(|(lane, plan)| {
            if lane < scheduled.member_count() {
                plan.is_some_and(|plan| {
                    Some(plan.request()) == scheduled.member(lane)
                        && plan.completion_epoch() == scheduled.epoch()
                })
            } else {
                plan.is_none()
            }
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
    selected: Vec<ActiveDeviceKvCache>,
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
        self.selected[0].projection()
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
        page_leases,
    } = inputs;
    let (draft_page_leases, target_page_leases) = match page_leases {
        M1FiniteSpeculativeQueueRolloverPageLeasesV1::ExactS1 { draft, target } => {
            (vec![draft], vec![target])
        }
        M1FiniteSpeculativeQueueRolloverPageLeasesV1::Roster { draft, target } => (draft, target),
    };
    if queue.shape() != M1PhysicalFixedBatchShapeV1::PairedPrefill
        || scheduled.member_count() == 0
        || selected.len() != scheduled.member_count()
        || selected
            .iter()
            .enumerate()
            .any(|(lane, cache)| Some(cache.projection().request) != scheduled.member(lane))
        || draft_page_leases.len() != scheduled.member_count()
        || target_page_leases.len() != scheduled.member_count()
        || !exact_roster_input_matches(&draft_decode, next.draft(), &scheduled)
        || !exact_roster_input_matches(&target_speculative, next.target(), &scheduled)
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

    let mut draft_reservations = Vec::new();
    let mut target_reservations = Vec::new();
    if draft_reservations
        .try_reserve_exact(selected.len())
        .is_err()
        || target_reservations
            .try_reserve_exact(selected.len())
            .is_err()
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
    let mut draft_leases = draft_page_leases.into_iter();
    for (lane, cache) in selected.iter_mut().enumerate() {
        let request = cache.projection().request;
        let leases = draft_leases
            .next()
            .expect("preflight matched draft lease roster");
        match cache.reserve_speculative_draft_round_write(
            request,
            next.target(),
            next.draft(),
            draft_decode.context_lengths()[lane],
            scheduled.epoch(),
            leases,
        ) {
            Ok(reservation) => draft_reservations.push(reservation),
            Err(failure) => {
                let (source, leases) = (*failure).into_parts();
                return Err(kv_reservation_failure(
                    M1S1K4QueueRolloverKvReservationPhaseV1::DraftReservation,
                    (
                        (prior, next, reason, queue, scheduled, selected, residue),
                        (
                            draft_decode,
                            target_speculative,
                            draft_reservations,
                            leases,
                            draft_leases,
                            target_page_leases,
                            source,
                        ),
                    ),
                ));
            }
        }
    }
    let mut target_leases = target_page_leases.into_iter();
    for (lane, cache) in selected.iter_mut().enumerate() {
        let request = cache.projection().request;
        let leases = target_leases
            .next()
            .expect("preflight matched target lease roster");
        match cache.reserve_step_write(
            request,
            ferric_spec::Qwen3ModelRole::Target8B,
            target_speculative.context_lengths()[lane],
            target_speculative.active_lengths()[lane],
            scheduled.epoch(),
            leases,
        ) {
            Ok(reservation) => target_reservations.push(reservation),
            Err(failure) => {
                let (source, leases) = (*failure).into_parts();
                return Err(kv_reservation_failure(
                    M1S1K4QueueRolloverKvReservationPhaseV1::TargetReservation,
                    (
                        (prior, next, reason, queue, scheduled, selected, residue),
                        (
                            draft_decode,
                            target_speculative,
                            draft_reservations,
                            target_reservations,
                            leases,
                            target_leases,
                            source,
                        ),
                    ),
                ));
            }
        }
    }
    let target = match crate::bind_m1_kv_workspace_table_v1(target_speculative, target_reservations)
    {
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
                    draft_reservations,
                    failure,
                ),
            ));
        }
    };
    let draft = match crate::bind_m1_speculative_draft_kv_round_workspace_table_v1(
        next.target(),
        draft_decode,
        draft_reservations,
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
    if validate_s1_k4_transition(scheduled.prior, scheduled.next, scheduled.reason).is_err() {
        engine.quarantine_m1_queue_rearm_failure();
        return Err(kv_reservation_failure(
            M1S1K4QueueRolloverKvReservationPhaseV1::Preflight,
            (scheduled, inputs),
        ));
    }
    match reserve_m1_s1_k4_queue_rollover_kv_inner_v1(scheduled, inputs) {
        Ok(reserved) => Ok(reserved),
        Err(failure) => {
            engine.quarantine_m1_queue_rearm_failure();
            Err(failure)
        }
    }
}

/// Installs exact roster-indexed finite-speculative successor KV reservations.
///
/// # Errors
///
/// Returns terminal retained custody after successor scheduling has advanced.
pub fn reserve_m1_finite_speculative_queue_rollover_kv_v1<const C: usize>(
    engine: &mut Engine<C>,
    scheduled: M1ScheduledFiniteSpeculativeQueueRolloverV1,
    inputs: M1FiniteSpeculativeQueueRolloverKvInputsV1,
) -> Result<
    M1ReservedFiniteSpeculativeQueueRolloverV1,
    M1FiniteSpeculativeQueueRolloverKvReservationFailureV1,
> {
    match reserve_m1_s1_k4_queue_rollover_kv_inner_v1(scheduled, inputs) {
        Ok(reserved) => Ok(M1ReservedFiniteSpeculativeQueueRolloverV1(reserved)),
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
    selected: Vec<ActiveDeviceKvCache>,
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
        self.remainder.selected[0].projection()
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
        Vec<ActiveDeviceKvCache>,
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

/// Prepares finite-speculative rollover workspace images after KV reservation.
///
/// # Errors
///
/// Returns terminal detached custody and faults the Engine on rejection.
pub fn prepare_m1_finite_speculative_queue_rollover_v1<const C: usize>(
    engine: &mut Engine<C>,
    reserved: M1ReservedFiniteSpeculativeQueueRolloverV1,
    runner: &LogicalRunnerDeclaration,
    plans: M1FullStepWorkspacePlans,
) -> Result<
    M1PreparedFiniteSpeculativeQueueRolloverV1,
    Box<M1FiniteSpeculativeQueueRolloverPrepareFailureV1>,
> {
    match prepare_m1_s1_k4_queue_rollover_inner_v1(reserved.0, runner, plans) {
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

fn finite_speculative_transition(
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
    validate_finite_speculative_transition(prior, next, reason)?;
    Ok((prior, next, reason))
}

fn validate_s1_k4_transition(
    prior: M1ServingPlanV1,
    next: M1ServingPlanV1,
    reason: M1ServingRolloverReasonV1,
) -> Result<(), M1S1K4QueueRolloverScheduleErrorV1> {
    let next_is_exact_s1_k4 = next.shape() == M1PhysicalFixedBatchShapeV1::SpeculativeK4
        && next.target().bucket == Qwen3PlanBucket::SpeculativeS1K4C8192
        && next.draft().bucket == Qwen3PlanBucket::DecodeS1C8192
        && next.sequence_capacity() == 1;
    let admitted = admit_m1_production_rollover_transition_v1(prior, next);
    if !next_is_exact_s1_k4 || admitted.is_none_or(|transition| transition.reason() != reason) {
        return Err(M1S1K4QueueRolloverScheduleErrorV1::UnsupportedTransition);
    }
    Ok(())
}

fn validate_finite_speculative_transition(
    prior: M1ServingPlanV1,
    next: M1ServingPlanV1,
    reason: M1ServingRolloverReasonV1,
) -> Result<(), M1S1K4QueueRolloverScheduleErrorV1> {
    if admit_m1_production_rollover_transition_v1(prior, next)
        .is_none_or(|transition| transition.reason() != reason)
    {
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
    let plans = finite_speculative_transition(batch)?;
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
    if batch.requests().is_empty() || batch.requests().len() > next.sequence_capacity() {
        return Err(M1S1K4QueueRolloverScheduleErrorV1::Roster);
    }
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
        .finite_speculative_rollover_output_state();
    if reserve_state != M1S1K4RolloverOutputPortfolioStateV1::Reserved {
        return Err(M1S1K4QueueRolloverScheduleErrorV1::OutputReserve(
            reserve_state,
        ));
    }
    if released.members().len() != batch.requests().len() {
        return Err(M1S1K4QueueRolloverScheduleErrorV1::MemberCustody);
    }
    for (lane, (member, request)) in released
        .members()
        .iter()
        .zip(batch.requests().iter().copied())
        .enumerate()
    {
        let M1ReleasedDeviceKvMemberV1::Active(cache) = member else {
            return Err(M1S1K4QueueRolloverScheduleErrorV1::MemberCustody);
        };
        if cache.projection().request != request || batch.requests().get(lane) != Some(&request) {
            return Err(M1S1K4QueueRolloverScheduleErrorV1::MemberCustody);
        }
        cache
            .preflight_quiescent_reselection(next.target(), next.draft_cache_selection())
            .map_err(M1S1K4QueueRolloverScheduleErrorV1::CacheReselection)?;
        if engine.state(request) != Some(RequestState::Ready) {
            return Err(M1S1K4QueueRolloverScheduleErrorV1::RequestNotReady);
        }
    }
    if engine.is_faulted() {
        return Err(M1S1K4QueueRolloverScheduleErrorV1::EngineFaulted);
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
pub fn schedule_m1_finite_speculative_queue_rollover_v1<const C: usize>(
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
    let mut selected = Vec::new();
    if selected.try_reserve_exact(residue.members.len()).is_err() {
        return Err(Box::new(terminal_failure(
            engine,
            M1S1K4QueueRolloverScheduleErrorV1::MemberCustody,
            M1S1K4QueueRolloverScheduleFailureCustodyV1::Detached {
                queue: Box::new(queue),
                scheduled: Some(Box::new(scheduled)),
                residue: Box::new(residue),
            },
        )));
    }
    while let Some(member) = residue.members.pop() {
        let M1ReleasedDeviceKvMemberV1::Active(mut cache) = member else {
            unreachable!("all-active custody was checked before detachment")
        };
        if let Err(source) = cache.reselect_quiescent(next.target(), next.draft_cache_selection()) {
            residue
                .members
                .push(M1ReleasedDeviceKvMemberV1::Active(cache));
            residue.members.extend(
                selected
                    .drain(..)
                    .rev()
                    .map(M1ReleasedDeviceKvMemberV1::Active),
            );
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
        selected.push(cache);
    }
    selected.reverse();
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

/// Exact-gated source-compatible S1/K4 rollover scheduler.
///
/// # Errors
///
/// Returns the original released owner before detachment or complete terminal
/// detached custody after scheduling has advanced.
pub fn schedule_m1_s1_k4_queue_rollover_exact_v1<const C: usize>(
    engine: &mut Engine<C>,
    released: M1ReleasedCompletedStepV1,
    batch: &M1ServingBatchPlanV1,
) -> Result<M1ScheduledS1K4QueueRolloverV1, Box<M1S1K4QueueRolloverScheduleFailureV1>> {
    if let Err(error) = exact_s1_k4_transition(batch) {
        return Err(Box::new(released_failure(error, released)));
    }
    schedule_m1_finite_speculative_queue_rollover_v1(engine, released, batch)
}

/// Generic finite-speculative scheduled rollover custody.
pub type M1ScheduledFiniteSpeculativeQueueRolloverV1 = M1ScheduledS1K4QueueRolloverV1;
/// Generic finite-speculative reserved rollover custody.
///
/// This nominal owner keeps wider reservations out of the legacy exact S1/K4
/// preparation entry point while preserving the original exact type.
#[must_use = "reserved rollover custody must prepare fresh workspace images"]
#[derive(Debug)]
pub struct M1ReservedFiniteSpeculativeQueueRolloverV1(M1ReservedS1K4QueueRolloverV1);

impl M1ReservedFiniteSpeculativeQueueRolloverV1 {
    #[must_use]
    pub const fn next_plan(&self) -> M1ServingPlanV1 {
        self.0.next_plan()
    }

    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        self.0.scheduled_dispatch()
    }

    #[must_use]
    pub fn selected_cache(&self) -> DeviceKvCacheProjection {
        self.0.selected_cache()
    }
}
/// Generic finite-speculative prepared rollover custody.
pub type M1PreparedFiniteSpeculativeQueueRolloverV1 = M1PreparedS1K4QueueRolloverV1;
/// Generic finite-speculative scheduling error.
pub type M1FiniteSpeculativeQueueRolloverScheduleErrorV1 = M1S1K4QueueRolloverScheduleErrorV1;
/// Generic finite-speculative scheduling failure custody.
pub type M1FiniteSpeculativeQueueRolloverScheduleFailureV1 = M1S1K4QueueRolloverScheduleFailureV1;
/// Generic finite-speculative scheduling failure phase custody.
pub type M1FiniteSpeculativeQueueRolloverScheduleFailureCustodyV1 =
    M1S1K4QueueRolloverScheduleFailureCustodyV1;
/// Generic finite-speculative predecessor residue.
pub type M1FiniteSpeculativeQueueRolloverResidueV1 = M1S1K4QueueRolloverResidueV1;
/// Generic finite-speculative KV reservation failure.
pub type M1FiniteSpeculativeQueueRolloverKvReservationFailureV1 =
    M1S1K4QueueRolloverKvReservationFailureV1;
/// Generic finite-speculative KV reservation phase.
pub type M1FiniteSpeculativeQueueRolloverKvReservationPhaseV1 =
    M1S1K4QueueRolloverKvReservationPhaseV1;
/// Generic finite-speculative workspace preparation failure.
pub type M1FiniteSpeculativeQueueRolloverPrepareFailureV1 = M1S1K4QueueRolloverPrepareFailureV1;

#[cfg(test)]
mod tests {
    use super::*;
    use ferric_spec::{
        validate_m1_step_inputs, Identity, M1StepInputCandidate, M1StepInputValidationOutcome,
        Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanSelection, RequestId, StepPlan,
        ValidatedM1StepInputs,
    };

    fn selection(
        role: Qwen3ModelRole,
        mode: Qwen3ExecutionMode,
        bucket: Qwen3PlanBucket,
    ) -> Qwen3PlanSelection {
        Qwen3PlanSelection { role, mode, bucket }
    }

    fn plan(mode: Qwen3ExecutionMode, bucket: Qwen3PlanBucket) -> M1ServingPlanV1 {
        let draft = match (mode, bucket) {
            (
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K4C8192
                | Qwen3PlanBucket::SpeculativeS1K8C8192
                | Qwen3PlanBucket::SpeculativeS1K16C8192,
            ) => selection(
                Qwen3ModelRole::Draft06B,
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS1C8192,
            ),
            (Qwen3ExecutionMode::Speculative, Qwen3PlanBucket::SpeculativeS8K4C8192) => selection(
                Qwen3ModelRole::Draft06B,
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS8C8192,
            ),
            _ => selection(Qwen3ModelRole::Draft06B, mode, bucket),
        };
        M1ServingPlanV1::new(selection(Qwen3ModelRole::Target8B, mode, bucket), draft).unwrap()
    }

    fn anchored_inputs(
        selected: Qwen3PlanSelection,
        anchors: &[ferric_spec::TokenId],
    ) -> ValidatedM1StepInputs {
        let dimensions = selected
            .bucket
            .dimensions(selected.role, selected.mode)
            .unwrap();
        let sequences = dimensions.sequences as usize;
        let width = dimensions.active_tokens as usize;
        let mut lanes = vec![None; sequences];
        let mut tokens = vec![0; sequences * width];
        let mut positions = vec![0; sequences * width];
        let mut active_lengths = vec![0; sequences];
        let mut context_lengths = vec![0; sequences];
        for (lane, anchor) in anchors.iter().copied().enumerate() {
            lanes[lane] = Some(StepPlan::new(
                RequestId::new(u32::try_from(lane).unwrap(), 1),
                CompletionEpoch::new(1),
                Identity::new([1; 32]),
                selected,
            ));
            active_lengths[lane] = dimensions.active_tokens;
            context_lengths[lane] = 128;
            for column in 0..width {
                let index = lane * width + column;
                tokens[index] = if column == 0 { anchor } else { 1 };
                positions[index] = 128 + u32::try_from(column).unwrap();
            }
        }
        match validate_m1_step_inputs(M1StepInputCandidate::new(
            selected,
            lanes,
            tokens,
            positions,
            active_lengths,
            context_lengths,
        )) {
            M1StepInputValidationOutcome::Validated(inputs) => inputs,
            M1StepInputValidationOutcome::Rejected(failure) => {
                panic!("anchor fixture rejected: {:?}", failure.error())
            }
        }
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
    fn finite_speculative_transition_predicate_accepts_only_the_four_canonical_pairs() {
        for (prior_bucket, next_bucket) in [
            (
                Qwen3PlanBucket::PrefillS1T128,
                Qwen3PlanBucket::SpeculativeS1K4C8192,
            ),
            (
                Qwen3PlanBucket::PrefillS8T128,
                Qwen3PlanBucket::SpeculativeS8K4C8192,
            ),
            (
                Qwen3PlanBucket::PrefillS1T128,
                Qwen3PlanBucket::SpeculativeS1K8C8192,
            ),
            (
                Qwen3PlanBucket::PrefillS1T128,
                Qwen3PlanBucket::SpeculativeS1K16C8192,
            ),
        ] {
            let prior = plan(Qwen3ExecutionMode::Prefill, prior_bucket);
            let next = plan(Qwen3ExecutionMode::Speculative, next_bucket);
            assert!(validate_finite_speculative_transition(
                prior,
                next,
                M1ServingRolloverReasonV1::Mode
            )
            .is_ok());
        }

        let s8_prior = plan(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS8T128);
        let s1_next = plan(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
        );
        assert!(matches!(
            validate_finite_speculative_transition(
                s8_prior,
                s1_next,
                M1ServingRolloverReasonV1::Mode
            ),
            Err(M1S1K4QueueRolloverScheduleErrorV1::UnsupportedTransition)
        ));
        assert!(matches!(
            validate_finite_speculative_transition(
                plan(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128),
                s1_next,
                M1ServingRolloverReasonV1::Shape
            ),
            Err(M1S1K4QueueRolloverScheduleErrorV1::UnsupportedTransition)
        ));

        for unsupported_prefill in [
            Qwen3PlanBucket::PrefillS1T512,
            Qwen3PlanBucket::PrefillS1T2048,
        ] {
            let prior = plan(Qwen3ExecutionMode::Prefill, unsupported_prefill);
            for next_bucket in [
                Qwen3PlanBucket::SpeculativeS1K4C8192,
                Qwen3PlanBucket::SpeculativeS1K8C8192,
                Qwen3PlanBucket::SpeculativeS1K16C8192,
            ] {
                let next = plan(Qwen3ExecutionMode::Speculative, next_bucket);
                assert!(matches!(
                    validate_finite_speculative_transition(
                        prior,
                        next,
                        M1ServingRolloverReasonV1::Mode,
                    ),
                    Err(M1S1K4QueueRolloverScheduleErrorV1::UnsupportedTransition)
                ));
            }
        }
    }

    #[test]
    fn roster_anchor_checks_are_lane_indexed_and_reject_inactive_padding() {
        let target = selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS8K4C8192,
        );
        let draft = selection(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS8C8192,
        );
        let inputs = M1S1K4QueueRolloverKvInputsV1::from_lane_leases(
            anchored_inputs(draft, &[11, 22]),
            anchored_inputs(target, &[11, 22]),
            Vec::new(),
            Vec::new(),
        );

        assert!(inputs.matches_anchor_at(0, 11));
        assert!(inputs.matches_anchor_at(1, 22));
        assert!(!inputs.matches_anchor_at(0, 22));
        assert!(!inputs.matches_anchor_at(2, 0));
        assert!(inputs.matches_anchors(&[11, 22]));
        assert!(!inputs.matches_anchors(&[11]));
        assert!(!inputs.matches_anchors(&[11, 23]));
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
