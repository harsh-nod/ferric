//! Closed repeated-step rearm for one long-lived M1 physical queue.
//!
//! This Ferric-only bridge consumes a released completed step, detaches the
//! recycled generic queue, captures exactly one next scheduler dispatch, and
//! replaces every request-specific workspace allocation before binding and
//! submitting the same native queue. The first version deliberately admits
//! only an unchanged decode or speculative selection and fixed-batch shape.
//! It does not admit prefill transitions, shape changes, new requests, or a
//! complete serving registry, and makes no hardware, numerical, or performance
//! claim.

use core::fmt;

use fe2o3_kfd::{ComputeAqlQueueObservationV1, Gfx942DeviceContentDescriptorV1};
use fe2o3_service_host::{
    DeviceWorkspaceRoleV1, ServiceDeviceDispatchRangeV1, ServiceFixedBatchV1,
    ServiceFixedDispatchBufferV1, ServiceFixedDispatchPacketV1, ServiceQueueDataUpdateFailureV1,
    ServiceQueueUnboundSessionV1,
};
use ferric_build::{AddresslessM1StepWorkspacePlan, M1StepWorkspaceRange};
use ferric_spec::{completion::CompletionEpoch, Qwen3ExecutionMode, Qwen3PlanSelection, RequestId};

use crate::physical_fixed_batch::M1PhysicalQueueBatchRearmPartsV1;
use crate::step_workspace_subleases::{
    bind_queue_replaced_m1_step_workspace, M1QueueReplacedWorkspaceBindingFailureV1,
};
use crate::{
    prepare_m1_scheduled_workspace_images_v1, ActiveDeviceKvCache,
    AddresslessM1FullStepWorkspaceComposition, AddresslessM1PhysicalBufferRecipeV1,
    BoundM1StepWorkspaceSubleases, ContentBoundM1ProgramCatalogV1, Engine, EngineError,
    Gfx942DeviceBinding, LogicalRunnerDeclaration, M1BoundPhysicalBufferRowV1,
    M1CompletedKvPageReleaseCountsV1, M1FullStepKvWorkspaceTablesV1, M1FullStepWorkspaceImagesV1,
    M1FullStepWorkspaceInputKind, M1FullStepWorkspacePlans, M1FullStepWorkspaceRole,
    M1FullStepWorkspaceSubleaseOwners, M1InitializedWorkspaceSlotV1, M1PhysicalBufferRecipeRowV1,
    M1PhysicalBufferSourceV1, M1PhysicalFixedBatchShapeV1, M1PhysicalPublishedQueueSessionV1,
    M1PhysicalQueueBatchCustodyV1, M1PhysicalQueuePhaseCaseV1, M1PhysicalQueueSessionV1,
    M1PhysicalReadbackDetachedQueueSessionV1, M1PhysicalReadbackQueueOperationFailureV1,
    M1PrepareFailureV1, M1PreparedScheduledWorkspaceImagesV1, M1PrepublicationStepCustodyV1,
    M1ReleasedCompletedStepV1, M1ReleasedDeviceKvMemberV1, M1ReleasedTerminalDeviceKvMemberV1,
    M1ScheduledDispatchV1, M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1,
    M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1, M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1,
    M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1,
};

/// Stable rejection before a fresh workspace replacement begins.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1LongLivedQueueRearmScheduleErrorV1 {
    UnsupportedPriorShape,
    NoContinuingRequests,
    Detach,
    Scheduler,
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
    emitted_counts: Box<[u32]>,
    release_counts: Box<[M1CompletedKvPageReleaseCountsV1]>,
    completed_members: usize,
    total_released: usize,
}

#[derive(Debug)]
enum ScheduleFailureCustodyV1 {
    ReleasedWithLineage {
        released: Box<M1ReleasedCompletedStepV1>,
        parked: Vec<ActiveDeviceKvCache>,
        terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
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
                    },
                ..
            } => Ok(M1LongLivedQueueUnscheduledRoundV1 {
                released: *released,
                parked,
                terminal,
            }),
            failure => Err(failure),
        }
    }
}

/// Intact released round that can retry scheduling or cleanly release its queue.
#[must_use = "an unscheduled round must retry or retain/release every owner"]
#[derive(Debug)]
pub struct M1LongLivedQueueUnscheduledRoundV1 {
    released: M1ReleasedCompletedStepV1,
    parked: Vec<ActiveDeviceKvCache>,
    terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
}

impl M1LongLivedQueueUnscheduledRoundV1 {
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
        )
    }

    /// Destroys and releases the intact queue while retaining every lineage owner.
    ///
    /// # Errors
    ///
    /// Returns terminal lower-layer queue release quarantine together with all
    /// parked and terminal lineage custody.
    pub fn destroy_queue_and_retain_round(
        self,
    ) -> Result<M1LongLivedQueueRearmTeardownSuccessV1, Box<M1LongLivedQueueRearmTeardownFailureV1>>
    {
        let Self {
            released,
            parked,
            terminal,
        } = self;
        match released.destroy_queue_and_retain_step() {
            Ok(released) => Ok(M1LongLivedQueueRearmTeardownSuccessV1 {
                released,
                parked,
                terminal,
            }),
            Err(released) => Err(Box::new(M1LongLivedQueueRearmTeardownFailureV1 {
                released,
                parked,
                terminal,
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
}

/// Terminal queue-release failure retaining current and historical custody.
#[must_use = "terminal queue release failure retains every available owner"]
#[derive(Debug)]
pub struct M1LongLivedQueueRearmTeardownFailureV1 {
    released: Box<crate::M1ReleasedQueueTeardownFailureV1>,
    parked: Vec<ActiveDeviceKvCache>,
    terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
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
    emitted_counts: Box<[u32]>,
    release_counts: Box<[M1CompletedKvPageReleaseCountsV1]>,
    completed_members: usize,
    total_released: usize,
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

    /// Returns immutable selected-cache projections in scheduler order.
    pub fn selected_cache_projections(
        &self,
    ) -> impl ExactSizeIterator<Item = crate::DeviceKvCacheProjection> + '_ {
        self.selected.iter().map(ActiveDeviceKvCache::projection)
    }
}

fn repeated_shape_is_supported(
    shape: M1PhysicalFixedBatchShapeV1,
    selection: Qwen3PlanSelection,
) -> bool {
    match shape {
        M1PhysicalFixedBatchShapeV1::TargetOnly => selection.mode == Qwen3ExecutionMode::Decode,
        M1PhysicalFixedBatchShapeV1::SpeculativeK4
        | M1PhysicalFixedBatchShapeV1::SpeculativeK8
        | M1PhysicalFixedBatchShapeV1::SpeculativeK16 => {
            selection.mode == Qwen3ExecutionMode::Speculative
        }
        M1PhysicalFixedBatchShapeV1::PairedPrefill => false,
    }
}

fn exact_next_epoch(previous: CompletionEpoch) -> Option<CompletionEpoch> {
    previous.value().checked_add(1).map(CompletionEpoch::new)
}

fn validate_request_partition(
    available: &[RequestId],
    scheduled: &[RequestId],
) -> Result<(), M1LongLivedQueueRearmScheduleErrorV1> {
    for (lane, request) in scheduled.iter().copied().enumerate() {
        if let Some(first_lane) = scheduled[..lane].iter().position(|prior| *prior == request) {
            return Err(
                M1LongLivedQueueRearmScheduleErrorV1::DuplicateScheduledRequest {
                    first_lane,
                    lane,
                },
            );
        }
        if !available.contains(&request) {
            return Err(M1LongLivedQueueRearmScheduleErrorV1::UnownedScheduledRequest { lane });
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

/// Detaches one released queue and captures exactly one next scheduler batch.
///
/// Scheduler order is authoritative. Every selected request must be one of the
/// released continuing caches; other continuing caches remain parked. New
/// requests, prefill transitions, and shape changes are rejected by this slice.
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
    schedule_m1_long_lived_queue_rearm_with_lineage_v1(engine, released, Vec::new(), Vec::new())
}

fn schedule_m1_long_lived_queue_rearm_with_lineage_v1<const C: usize>(
    engine: &mut Engine<C>,
    released: M1ReleasedCompletedStepV1,
    parked_lineage: Vec<ActiveDeviceKvCache>,
    terminal_lineage: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
) -> Result<M1ScheduledLongLivedQueueRearmV1, M1LongLivedQueueRearmScheduleFailureV1> {
    let result = schedule_m1_long_lived_queue_rearm_inner_v1(
        engine,
        released,
        parked_lineage,
        terminal_lineage,
    );
    finish_schedule_transition(engine, result)
}

fn schedule_m1_long_lived_queue_rearm_inner_v1<const C: usize>(
    engine: &mut Engine<C>,
    released: M1ReleasedCompletedStepV1,
    parked_lineage: Vec<ActiveDeviceKvCache>,
    terminal_lineage: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
) -> Result<
    M1ScheduledLongLivedQueueRearmV1,
    (
        M1LongLivedQueueRearmSchedulePhaseV1,
        M1LongLivedQueueRearmScheduleFailureV1,
    ),
> {
    let shape = released.queue().shape();
    let selection = released.queue().custody().selection();
    if !repeated_shape_is_supported(shape, selection) {
        return Err(schedule_phase_failure(
            M1LongLivedQueueRearmSchedulePhaseV1::Released,
            M1LongLivedQueueRearmScheduleErrorV1::UnsupportedPriorShape,
            ScheduleFailureCustodyV1::ReleasedWithLineage {
                released: Box::new(released),
                parked: parked_lineage,
                terminal: terminal_lineage,
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
            },
        ));
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
            },
        ));
    }

    let (
        queue,
        checked,
        mut members,
        emitted_counts,
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
        emitted_counts,
        release_counts,
        completed_members,
        total_released,
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
    let scheduled = match engine.dispatch_m1_ready() {
        Ok(Some(scheduled)) => scheduled,
        Ok(None) => {
            return Err(schedule_phase_failure(
                M1LongLivedQueueRearmSchedulePhaseV1::Detached,
                M1LongLivedQueueRearmScheduleErrorV1::EmptySchedulerBatch,
                ScheduleFailureCustodyV1::Detached {
                    queue: Box::new(queue),
                    residue: Box::new(residue),
                    scheduler_error: None,
                    scheduled: None,
                },
            ));
        }
        Err(error) => {
            return Err(schedule_phase_failure(
                M1LongLivedQueueRearmSchedulePhaseV1::Detached,
                M1LongLivedQueueRearmScheduleErrorV1::Scheduler,
                ScheduleFailureCustodyV1::Detached {
                    queue: Box::new(queue),
                    residue: Box::new(residue),
                    scheduler_error: Some(error),
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
        emitted_counts: residue.emitted_counts,
        release_counts: residue.release_counts,
        completed_members: residue.completed_members,
        total_released: residue.total_released,
    })
}

#[derive(Debug)]
struct ScheduledRemainderV1 {
    queue: M1PhysicalReadbackDetachedQueueSessionV1,
    selected: Vec<ActiveDeviceKvCache>,
    parked: Vec<ActiveDeviceKvCache>,
    terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
    prior_checked: crate::M1CheckedCompletionOutputV1,
    emitted_counts: Box<[u32]>,
    release_counts: Box<[M1CompletedKvPageReleaseCountsV1]>,
    completed_members: usize,
    total_released: usize,
}

/// Fresh page leases and validated workspace inputs for the next same-shape round.
#[must_use = "KV reservation inputs contain linear page leases"]
#[derive(Debug, Eq, PartialEq)]
pub enum M1LongLivedQueueRearmKvInputsV1 {
    TargetOnly {
        target: ferric_spec::ValidatedM1StepInputs,
        target_page_leases: Vec<Vec<crate::DeviceKvPageLease>>,
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
        emitted_counts,
        release_counts,
        completed_members,
        total_released,
    } = scheduled;
    let remainder = ScheduledRemainderV1 {
        queue,
        selected,
        parked,
        terminal,
        prior_checked,
        emitted_counts,
        release_counts,
        completed_members,
        total_released,
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
    WorkspaceRangeRebinding,
    FixedBatchRebuild,
    QueueBind,
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
struct FreshWorkspaceRangeV1 {
    workspace: M1FullStepWorkspaceRole,
    semantic: M1StepWorkspaceRange,
    dispatch: ServiceDeviceDispatchRangeV1,
}

fn member_layout<const N: usize>(plan: &AddresslessM1StepWorkspacePlan) -> [(u64, u64, u64); N] {
    core::array::from_fn(|index| {
        let range = plan.ranges()[index];
        (range.offset(), range.byte_len(), range.alignment())
    })
}

fn append_workspace_ranges<const N: usize>(
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

fn requested_workspace_range(
    source: M1PhysicalBufferSourceV1,
    composition: &AddresslessM1FullStepWorkspaceComposition,
) -> Option<(M1FullStepWorkspaceRole, M1StepWorkspaceRange)> {
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
            .map(|range| (workspace, range)),
        M1PhysicalBufferSourceV1::SpeculativeDraftChoices(row) => {
            Some((M1FullStepWorkspaceRole::Target, row.range()))
        }
        M1PhysicalBufferSourceV1::SpeculativeDraftIterationMetadata {
            workspace,
            range,
            draft_segment,
            ..
        } => {
            let binding = composition.segment_binding(draft_segment)?;
            let row = match range {
                ferric_build::M1StepWorkspaceRangeRole::DraftPositionIds => {
                    binding.draft_position_ids_subrange()?.range()
                }
                ferric_build::M1StepWorkspaceRangeRole::DraftContextLengths => {
                    binding.draft_context_lengths_subrange()?.range()
                }
                _ => return None,
            };
            Some((workspace, row))
        }
        M1PhysicalBufferSourceV1::CompletionOutput { .. }
        | M1PhysicalBufferSourceV1::ModelWeight { .. }
        | M1PhysicalBufferSourceV1::KvCachePlane { .. } => None,
    }
}

fn resolve_fresh_workspace_range(
    source: M1PhysicalBufferSourceV1,
    composition: &AddresslessM1FullStepWorkspaceComposition,
    ranges: &[FreshWorkspaceRangeV1],
) -> Result<Option<ServiceDeviceDispatchRangeV1>, ()> {
    let Some((workspace, requested)) = requested_workspace_range(source, composition) else {
        return Ok(None);
    };
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
        .map(Some)
        .map_err(|_| ())
}

fn rebuild_bound_rows(
    source_rows: &[M1PhysicalBufferRecipeRowV1],
    old_bound_rows: &[M1BoundPhysicalBufferRowV1],
    composition: &AddresslessM1FullStepWorkspaceComposition,
    workspace_ranges: &[FreshWorkspaceRangeV1],
) -> Result<Box<[M1BoundPhysicalBufferRowV1]>, ()> {
    if source_rows.len() != old_bound_rows.len() {
        return Err(());
    }
    let mut rows = Vec::new();
    rows.try_reserve_exact(source_rows.len()).map_err(|_| ())?;
    for (source, old) in source_rows.iter().zip(old_bound_rows) {
        if source.dispatch_index() != old.dispatch_index()
            || source.profile_id() != old.profile_id()
            || source.program() != old.program()
            || source.buffers().len() != old.buffers().len()
        {
            return Err(());
        }
        let mut buffers = Vec::new();
        buffers
            .try_reserve_exact(source.buffers().len())
            .map_err(|_| ())?;
        for (semantic, old_buffer) in source.buffers().iter().zip(old.buffers()) {
            if semantic.explicit_argument_index() != old_buffer.explicit_argument_index() {
                return Err(());
            }
            let buffer = match resolve_fresh_workspace_range(
                semantic.source(),
                composition,
                workspace_ranges,
            )? {
                Some(range) => {
                    ServiceFixedDispatchBufferV1::new(semantic.explicit_argument_index(), range)
                }
                None => *old_buffer,
            };
            buffers.push(buffer);
        }
        rows.push(M1BoundPhysicalBufferRowV1::from_queue_rearm(
            source,
            buffers.into_boxed_slice(),
        ));
    }
    Ok(rows.into_boxed_slice())
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

fn lower_batch<'a, const N: usize>(
    catalog: ContentBoundM1ProgramCatalogV1<'a>,
    physical: &crate::AddresslessM1PhysicalDispatchRecipeV1,
    images: Box<[crate::M1PhysicalKernargImageV1]>,
    bound: &[M1BoundPhysicalBufferRowV1],
) -> ServiceFixedBatchV1<'a, N> {
    let mut images = images.into_vec().into_iter();
    let packets = core::array::from_fn(|index| {
        let physical = physical.rows()[index];
        let image = images.next().expect("cardinality was checked");
        ServiceFixedDispatchPacketV1::new(
            physical.program_index(),
            physical.geometry(),
            physical.dynamic_group_segment_bytes(),
            image.into_bytes(),
            bound[index].buffers().to_vec().into_boxed_slice(),
        )
    });
    ServiceFixedBatchV1::new(catalog.into_programs(), packets)
}

#[derive(Debug)]
struct M1RearmContinuationCustodyV1 {
    selected: Vec<ActiveDeviceKvCache>,
    parked: Vec<ActiveDeviceKvCache>,
    terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
    previous_epoch: CompletionEpoch,
    prior_checked: crate::M1CheckedCompletionOutputV1,
    emitted_counts: Box<[u32]>,
    release_counts: Box<[M1CompletedKvPageReleaseCountsV1]>,
    completed_members: usize,
    total_released: usize,
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
///     let _first = published.wait(engine, 1);
///     let _second = published.wait(engine, 1);
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
    #[must_use]
    pub const fn queue_observation(&self) -> ComputeAqlQueueObservationV1 {
        self.queue_observation
    }

    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.device
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
    pub fn prior_emitted_counts(&self) -> &[u32] {
        &self.carry.emitted_counts
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
        polls: u32,
    ) -> Result<M1RearmedCompletedQueueV1, Box<M1RearmedQueueProgressFailureV1>> {
        let Self {
            queue,
            carry,
            queue_observation,
            device,
        } = self;
        match queue.wait(polls) {
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

/// Recycled rearmed queue ready for the existing exact readback join.
#[must_use = "recycled rearm custody must read exact completion or remain retained"]
#[derive(Debug)]
pub struct M1RearmedRecycledQueueV1 {
    queue: crate::M1PhysicalRecycledQueueSessionV1,
    carry: M1RearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

impl M1RearmedRecycledQueueV1 {
    /// Reads and checks the exact completion bytes for the fresh generation.
    ///
    /// # Errors
    ///
    /// Returns retry-safe unchanged recycled custody for queue, coordinate,
    /// wire, scheduler-roster, padding, or semantic rejection.
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
        match queue.read_and_check_completion(expectations) {
            Ok(readback) => Ok(M1RearmedCompletedReadbackV1 {
                readback,
                carry,
                queue_observation,
                device,
            }),
            Err(source) => Err(Box::new(M1RearmedReadbackFailureV1 {
                source,
                carry,
                queue_observation,
                device,
            })),
        }
    }
}

/// Retry-safe exact-readback rejection retaining the same recycled owner.
#[must_use = "readback rejection must be retried, torn down, or retained"]
#[derive(Debug)]
pub struct M1RearmedReadbackFailureV1 {
    source: crate::M1CompletedReadbackJoinFailureV1,
    carry: M1RearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

impl M1RearmedReadbackFailureV1 {
    #[must_use]
    pub const fn source(&self) -> &crate::M1CompletedReadbackJoinErrorV1 {
        self.source.error()
    }

    /// Recovers the unchanged recycled queue without exposing a raw session.
    #[must_use = "retry-capable recycled queue and cache custody must remain retained"]
    pub fn into_recycled(self) -> M1RearmedRecycledQueueV1 {
        let (_error, queue) = self.source.into_parts();
        M1RearmedRecycledQueueV1 {
            queue,
            carry: self.carry,
            queue_observation: self.queue_observation,
            device: self.device,
        }
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

/// Pure local rejection before selected caches enter the completion fan-out.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1RearmedCompletionPreflightErrorV1 {
    DispositionCount { expected: usize, actual: usize },
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

    #[must_use]
    pub const fn retained_cache_count(&self) -> usize {
        self.readback.carry.selected.len() + self.readback.carry.parked.len()
    }

    #[must_use]
    pub fn disposition_count(&self) -> usize {
        self.dispositions.len()
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
}

/// Existing physical completion outcome plus custody parked across the round.
#[must_use = "completion outcome and parked rearm custody must remain retained"]
#[derive(Debug)]
pub struct M1RearmedCompletionOutcomeV1 {
    outcome: crate::M1CompletedStepOutcomeV1,
    parked: Vec<ActiveDeviceKvCache>,
    terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
    prior_checked: crate::M1CheckedCompletionOutputV1,
    prior_emitted_counts: Box<[u32]>,
    prior_release_counts: Box<[M1CompletedKvPageReleaseCountsV1]>,
    prior_completed_members: usize,
    prior_total_released: usize,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
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
        self.queue_observation
    }

    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.device
    }

    #[must_use]
    pub const fn prior_completed_members(&self) -> usize {
        self.prior_completed_members
    }

    #[must_use]
    pub const fn prior_total_released(&self) -> usize {
        self.prior_total_released
    }

    pub const fn prior_checked(&self) -> &crate::M1CheckedCompletionOutputV1 {
        &self.prior_checked
    }

    #[must_use]
    pub fn prior_emitted_counts(&self) -> &[u32] {
        &self.prior_emitted_counts
    }

    #[must_use]
    pub fn prior_release_counts(&self) -> &[M1CompletedKvPageReleaseCountsV1] {
        &self.prior_release_counts
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
            prior_checked,
            prior_emitted_counts,
            prior_release_counts,
            prior_completed_members,
            prior_total_released,
            queue_observation,
            device,
        } = self;
        let crate::M1CompletedStepOutcomeV1::Rejected(rejected) = outcome else {
            return Err(Box::new(Self {
                outcome,
                parked,
                terminal,
                prior_checked,
                prior_emitted_counts,
                prior_release_counts,
                prior_completed_members,
                prior_total_released,
                queue_observation,
                device,
            }));
        };
        let (_error, readback, roster) = rejected.into_parts();
        Ok(Self {
            outcome: crate::complete_m1_physical_step_v1(engine, readback, roster),
            parked,
            terminal,
            prior_checked,
            prior_emitted_counts,
            prior_release_counts,
            prior_completed_members,
            prior_total_released,
            queue_observation,
            device,
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
            prior_checked,
            prior_emitted_counts,
            prior_release_counts,
            prior_completed_members,
            prior_total_released,
            queue_observation,
            device,
        } = self;
        let crate::M1CompletedStepOutcomeV1::Completed(completed) = outcome else {
            return M1RearmedRoundReleaseOutcomeV1::NotCompleted(Self {
                outcome,
                parked,
                terminal,
                prior_checked,
                prior_emitted_counts,
                prior_release_counts,
                prior_completed_members,
                prior_total_released,
                queue_observation,
                device,
            });
        };
        release_rearmed_round(
            completed,
            M1PriorRearmRoundHistoryV1 {
                prior_checked,
                prior_emitted_counts,
                prior_release_counts,
                prior_completed_members,
                prior_total_released,
                queue_observation,
                device,
            },
            parked,
            terminal,
        )
    }
}

#[derive(Debug)]
struct M1PriorRearmRoundHistoryV1 {
    prior_checked: crate::M1CheckedCompletionOutputV1,
    prior_emitted_counts: Box<[u32]>,
    prior_release_counts: Box<[M1CompletedKvPageReleaseCountsV1]>,
    prior_completed_members: usize,
    prior_total_released: usize,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

/// Released current round plus active caches parked outside that round.
///
/// The parked caches are deliberately separate from `released`: the current
/// checked/emitted/release arrays name only current selected lanes.
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
    history: M1PriorRearmRoundHistoryV1,
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
        &self.history.prior_checked
    }

    #[must_use]
    pub fn prior_emitted_counts(&self) -> &[u32] {
        &self.history.prior_emitted_counts
    }

    #[must_use]
    pub fn prior_release_counts(&self) -> &[M1CompletedKvPageReleaseCountsV1] {
        &self.history.prior_release_counts
    }

    #[must_use]
    pub const fn prior_completed_members(&self) -> usize {
        self.history.prior_completed_members
    }

    #[must_use]
    pub const fn prior_total_released(&self) -> usize {
        self.history.prior_total_released
    }

    #[must_use]
    pub const fn queue_observation(&self) -> ComputeAqlQueueObservationV1 {
        self.history.queue_observation
    }

    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.history.device
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
            history: _,
        } = self;
        schedule_m1_long_lived_queue_rearm_with_lineage_v1(engine, released, parked, terminal)
    }
}

/// Retry-safe page-release rejection retaining the separately parked lineage.
#[must_use = "page-release rejection remains the sole retry owner"]
#[derive(Debug)]
pub struct M1RearmedRoundPageReleaseFailureV1 {
    source: Box<crate::M1CompletedStepKvReleaseFailureV1>,
    parked: Vec<ActiveDeviceKvCache>,
    terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
    history: M1PriorRearmRoundHistoryV1,
}

impl M1RearmedRoundPageReleaseFailureV1 {
    #[must_use]
    pub const fn source(&self) -> &crate::M1CompletedStepKvReleaseErrorV1 {
        self.source.error()
    }

    /// Retries exact page release with the unchanged completed owner.
    #[must_use = "retry outcome retains every current and parked owner"]
    pub fn retry(self) -> M1RearmedRoundReleaseOutcomeV1 {
        let (_error, completed) = (*self.source).into_parts();
        release_rearmed_round(completed, self.history, self.parked, self.terminal)
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
    history: M1PriorRearmRoundHistoryV1,
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
    /// Completes the fresh physical generation using selected caches in the
    /// exact scheduler order retained since scheduling.
    ///
    /// # Errors
    ///
    /// Returns unchanged readback/cache custody and all supplied dispositions
    /// for a count mismatch or host reservation failure.
    pub fn complete<const C: usize>(
        self,
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
        Ok(M1RearmedCompletionOutcomeV1 {
            outcome,
            parked: carry.parked,
            terminal: carry.terminal,
            prior_checked: carry.prior_checked,
            prior_emitted_counts: carry.emitted_counts,
            prior_release_counts: carry.release_counts,
            prior_completed_members: carry.completed_members,
            prior_total_released: carry.total_released,
            queue_observation,
            device,
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

fn bind_submit_target_only(
    lower: ServiceQueueUnboundSessionV1,
    batch: ServiceFixedBatchV1<'_, M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1>,
    custody: M1PhysicalQueueBatchCustodyV1,
    step: M1PrepublicationStepCustodyV1,
    expected_observation: ComputeAqlQueueObservationV1,
) -> Result<M1PhysicalPublishedQueueSessionV1, M1LongLivedQueueRearmSubmissionFailureV1<'_>> {
    let lower = match lower.bind(batch) {
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

fn bind_submit_speculative_k4(
    lower: ServiceQueueUnboundSessionV1,
    batch: ServiceFixedBatchV1<'_, M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1>,
    custody: M1PhysicalQueueBatchCustodyV1,
    step: M1PrepublicationStepCustodyV1,
    expected_observation: ComputeAqlQueueObservationV1,
) -> Result<M1PhysicalPublishedQueueSessionV1, M1LongLivedQueueRearmSubmissionFailureV1<'_>> {
    let lower = match lower.bind(batch) {
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

fn bind_submit_speculative_k8(
    lower: ServiceQueueUnboundSessionV1,
    batch: ServiceFixedBatchV1<'_, M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1>,
    custody: M1PhysicalQueueBatchCustodyV1,
    step: M1PrepublicationStepCustodyV1,
    expected_observation: ComputeAqlQueueObservationV1,
) -> Result<M1PhysicalPublishedQueueSessionV1, M1LongLivedQueueRearmSubmissionFailureV1<'_>> {
    let lower = match lower.bind(batch) {
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

fn bind_submit_speculative_k16(
    lower: ServiceQueueUnboundSessionV1,
    batch: ServiceFixedBatchV1<'_, M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1>,
    custody: M1PhysicalQueueBatchCustodyV1,
    step: M1PrepublicationStepCustodyV1,
    expected_observation: ComputeAqlQueueObservationV1,
) -> Result<M1PhysicalPublishedQueueSessionV1, M1LongLivedQueueRearmSubmissionFailureV1<'_>> {
    let lower = match lower.bind(batch) {
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
    let bound_rows = match rebuild_bound_rows(
        recipe.rows(),
        &custody.bound_rows,
        recipe.workspace_composition(),
        &workspace_ranges,
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
    let batch = match shape {
        M1PhysicalFixedBatchShapeV1::TargetOnly => RebuiltBatchV1::TargetOnly(Box::new(
            lower_batch(catalog, &physical_recipe, images, &bound_rows),
        )),
        M1PhysicalFixedBatchShapeV1::SpeculativeK4 => RebuiltBatchV1::SpeculativeK4(Box::new(
            lower_batch(catalog, &physical_recipe, images, &bound_rows),
        )),
        M1PhysicalFixedBatchShapeV1::SpeculativeK8 => RebuiltBatchV1::SpeculativeK8(Box::new(
            lower_batch(catalog, &physical_recipe, images, &bound_rows),
        )),
        M1PhysicalFixedBatchShapeV1::SpeculativeK16 => RebuiltBatchV1::SpeculativeK16(Box::new(
            lower_batch(catalog, &physical_recipe, images, &bound_rows),
        )),
        M1PhysicalFixedBatchShapeV1::PairedPrefill => {
            return Err(submission_failure(
                M1LongLivedQueueRearmSubmissionPhaseV1::FixedBatchRebuild,
                (lower, custody, physical_recipe, images, bound_rows, step),
            ));
        }
    };
    custody.physical_recipe = physical_recipe;
    custody.workspace_composition = workspace_composition;
    custody.source_rows = source_rows;
    custody.bound_rows = bound_rows;
    let custody = M1PhysicalQueueBatchCustodyV1::from_rearm_parts(custody);
    match batch {
        RebuiltBatchV1::TargetOnly(batch) => {
            bind_submit_target_only(lower, *batch, custody, step, expected_observation)
        }
        RebuiltBatchV1::SpeculativeK4(batch) => {
            bind_submit_speculative_k4(lower, *batch, custody, step, expected_observation)
        }
        RebuiltBatchV1::SpeculativeK8(batch) => {
            bind_submit_speculative_k8(lower, *batch, custody, step, expected_observation)
        }
        RebuiltBatchV1::SpeculativeK16(batch) => {
            bind_submit_speculative_k16(lower, *batch, custody, step, expected_observation)
        }
    }
}

#[derive(Debug)]
struct PostQueueRemainderV1 {
    selected: Vec<ActiveDeviceKvCache>,
    parked: Vec<ActiveDeviceKvCache>,
    terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
    prior_checked: crate::M1CheckedCompletionOutputV1,
    emitted_counts: Box<[u32]>,
    release_counts: Box<[M1CompletedKvPageReleaseCountsV1]>,
    completed_members: usize,
    total_released: usize,
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
        emitted_counts,
        release_counts,
        completed_members,
        total_released,
    } = remainder;
    let post = PostQueueRemainderV1 {
        selected,
        parked,
        terminal,
        prior_checked,
        emitted_counts,
        release_counts,
        completed_members,
        total_released,
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
            emitted_counts: post.emitted_counts,
            release_counts: post.release_counts,
            completed_members: post.completed_members,
            total_released: post.total_released,
        },
        queue_observation: expected_observation,
        device,
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use ferric_spec::{Qwen3ModelRole, Qwen3PlanBucket};

    const fn selection(mode: Qwen3ExecutionMode, bucket: Qwen3PlanBucket) -> Qwen3PlanSelection {
        Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode,
            bucket,
        }
    }

    #[test]
    fn repeated_slice_excludes_prefill_and_shape_transitions() {
        assert!(!repeated_shape_is_supported(
            M1PhysicalFixedBatchShapeV1::PairedPrefill,
            selection(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128),
        ));
        assert!(!repeated_shape_is_supported(
            M1PhysicalFixedBatchShapeV1::TargetOnly,
            selection(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128),
        ));
        assert!(repeated_shape_is_supported(
            M1PhysicalFixedBatchShapeV1::TargetOnly,
            selection(Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS1C8192),
        ));
        assert!(repeated_shape_is_supported(
            M1PhysicalFixedBatchShapeV1::SpeculativeK4,
            selection(
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K4C8192,
            ),
        ));
    }

    #[test]
    fn every_runtime_failure_phase_is_closed() {
        let phases = [
            M1LongLivedQueueRearmSubmissionPhaseV1::Preflight,
            M1LongLivedQueueRearmSubmissionPhaseV1::DraftWorkspaceReplacement,
            M1LongLivedQueueRearmSubmissionPhaseV1::TargetWorkspaceReplacement,
            M1LongLivedQueueRearmSubmissionPhaseV1::WorkspaceRangeRebinding,
            M1LongLivedQueueRearmSubmissionPhaseV1::FixedBatchRebuild,
            M1LongLivedQueueRearmSubmissionPhaseV1::QueueBind,
            M1LongLivedQueueRearmSubmissionPhaseV1::QueueObservation,
            M1LongLivedQueueRearmSubmissionPhaseV1::QueueSubmit,
        ];
        assert_eq!(phases.len(), 8);
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
