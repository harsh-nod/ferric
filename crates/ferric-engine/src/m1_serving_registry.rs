//! Deterministic M1 serving-roster and queue-rollover planning.
//!
//! The physical rearm path intentionally accepts only an unchanged execution
//! plan. This registry is the Ferric-owned boundary that keeps unlike work out
//! of one fixed batch and classifies every next roster as a fresh launch, an
//! unchanged-plan rearm, or a quiescent rollover. A rollover is only a planning
//! result: the caller must retain and rebuild the physical queue custody before
//! publishing the returned roster.

use core::sync::atomic::{AtomicU64, Ordering};

use ferric_spec::{
    completion::CompletionEpoch, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket,
    Qwen3PlanSelection, RequestId, M1_MAX_ACTIVE_SEQUENCES,
};

use crate::M1PhysicalFixedBatchShapeV1;

static NEXT_M1_SERVING_REGISTRY_IDENTITY_V1: AtomicU64 = AtomicU64::new(1);

/// Opaque identity binding every move-only authority to one registry instance.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1ServingRegistryIdentityV1(u64);

impl M1ServingRegistryIdentityV1 {
    fn fresh() -> Result<Self, M1ServingRegistryErrorV1> {
        NEXT_M1_SERVING_REGISTRY_IDENTITY_V1
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                current.checked_add(1)
            })
            .map(Self)
            .map_err(|_| M1ServingRegistryErrorV1::RegistryIdentityExhausted)
    }
}

/// Exact paired target/draft plan used by one homogeneous serving roster.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1ServingPlanV1 {
    target: Qwen3PlanSelection,
    draft: Qwen3PlanSelection,
    shape: M1PhysicalFixedBatchShapeV1,
    sequence_capacity: usize,
}

impl M1ServingPlanV1 {
    /// Validates an exact target/draft pair and derives its physical shape.
    ///
    /// # Errors
    ///
    /// Rejects invalid roles, modes, buckets, cross-role plan drift, or an
    /// unsupported sequence capacity.
    pub fn new(
        target: Qwen3PlanSelection,
        draft: Qwen3PlanSelection,
    ) -> Result<Self, M1ServingRegistryErrorV1> {
        if target.role != Qwen3ModelRole::Target8B
            || draft.role != Qwen3ModelRole::Draft06B
            || target.validate().is_err()
            || draft.validate().is_err()
        {
            return Err(M1ServingRegistryErrorV1::InvalidPlanPair);
        }
        let (shape, draft_mode, draft_bucket) = match (target.mode, target.bucket) {
            (
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T128
                | Qwen3PlanBucket::PrefillS8T128
                | Qwen3PlanBucket::PrefillS1T512
                | Qwen3PlanBucket::PrefillS1T2048,
            ) => (
                M1PhysicalFixedBatchShapeV1::PairedPrefill,
                Qwen3ExecutionMode::Prefill,
                target.bucket,
            ),
            (
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS1C8192
                | Qwen3PlanBucket::DecodeS8C8192
                | Qwen3PlanBucket::DecodeS32C8192,
            ) => (
                M1PhysicalFixedBatchShapeV1::TargetOnly,
                Qwen3ExecutionMode::Decode,
                target.bucket,
            ),
            (Qwen3ExecutionMode::Speculative, Qwen3PlanBucket::SpeculativeS1K4C8192) => (
                M1PhysicalFixedBatchShapeV1::SpeculativeK4,
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS1C8192,
            ),
            (Qwen3ExecutionMode::Speculative, Qwen3PlanBucket::SpeculativeS8K4C8192) => (
                M1PhysicalFixedBatchShapeV1::SpeculativeK4,
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS8C8192,
            ),
            (Qwen3ExecutionMode::Speculative, Qwen3PlanBucket::SpeculativeS1K8C8192) => (
                M1PhysicalFixedBatchShapeV1::SpeculativeK8,
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS1C8192,
            ),
            (Qwen3ExecutionMode::Speculative, Qwen3PlanBucket::SpeculativeS1K16C8192) => (
                M1PhysicalFixedBatchShapeV1::SpeculativeK16,
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS1C8192,
            ),
            _ => return Err(M1ServingRegistryErrorV1::InvalidPlanPair),
        };
        if draft.mode != draft_mode || draft.bucket != draft_bucket {
            return Err(M1ServingRegistryErrorV1::InvalidPlanPair);
        }
        let dimensions = target
            .bucket
            .dimensions(target.role, target.mode)
            .ok_or(M1ServingRegistryErrorV1::InvalidPlanPair)?;
        let sequence_capacity = usize::try_from(dimensions.sequences)
            .map_err(|_| M1ServingRegistryErrorV1::InvalidPlanPair)?;
        if sequence_capacity == 0 || sequence_capacity > M1_MAX_ACTIVE_SEQUENCES as usize {
            return Err(M1ServingRegistryErrorV1::InvalidPlanPair);
        }
        Ok(Self {
            target,
            draft,
            shape,
            sequence_capacity,
        })
    }

    #[must_use]
    pub const fn target(self) -> Qwen3PlanSelection {
        self.target
    }

    #[must_use]
    pub const fn draft(self) -> Qwen3PlanSelection {
        self.draft
    }

    #[must_use]
    pub const fn mode(self) -> Qwen3ExecutionMode {
        self.target.mode
    }

    #[must_use]
    pub const fn shape(self) -> M1PhysicalFixedBatchShapeV1 {
        self.shape
    }

    #[must_use]
    pub const fn sequence_capacity(self) -> usize {
        self.sequence_capacity
    }
}

/// Exact reason a physical queue cannot use the unchanged-plan rearm path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1ServingRolloverReasonV1 {
    Mode,
    Shape,
    Bucket,
}

/// Physical queue action required before one planned homogeneous roster runs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1ServingQueueActionV1 {
    /// No physical queue is currently retained.
    FreshLaunch,
    /// The existing rearm path may reuse the same exact physical plan.
    SameShapeRearm,
    /// The prior generation is quiescent, but its physical custody must be
    /// rebuilt before this roster can be published.
    QuiescentRollover {
        prior: M1ServingPlanV1,
        next: M1ServingPlanV1,
        reason: M1ServingRolloverReasonV1,
    },
}

/// Queue disposition when no physical generation is currently in flight.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1ServingQuiescentQueueActionV1 {
    /// The registry has not launched a physical queue.
    NoQueue,
    /// At least one request is ready; [`M1ServingRegistryV1::plan_next`] decides
    /// whether the queue can rearm or must roll over.
    RetainForReadyWork { bound: M1ServingPlanV1 },
    /// No request remains ready, so the quiescent physical queue may retire.
    Retire { bound: M1ServingPlanV1 },
}

/// Registry-only request state. Physical cache/page custody remains in the
/// completed-step and rearm owners.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1ServingRequestPhaseV1 {
    Ready,
    InFlight { epoch: CompletionEpoch },
    CancellationPending { epoch: CompletionEpoch },
    Retired { quiescence: M1ServingQuiescenceV1 },
}

/// Exact quiescence source for a retired registry member.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1ServingQuiescenceV1 {
    NeverSubmitted,
    Completed(CompletionEpoch),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct M1ServingEntryV1 {
    request: RequestId,
    plan: M1ServingPlanV1,
    phase: M1ServingRequestPhaseV1,
    last_quiescence: Option<CompletionEpoch>,
}

/// Stable fail-closed serving-registry rejection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1ServingRegistryErrorV1 {
    ZeroCapacity,
    CapacityExceedsM1,
    OutOfSlots,
    DuplicateRequest,
    UnknownRequest,
    InvalidRequest,
    InvalidPlanPair,
    AdmissionRequiresPrefill,
    RequestNotReady,
    RequestNotInFlight,
    CancellationAlreadyRequested,
    RequestAlreadyRetired,
    TransitionRequiresQuiescence,
    PrefillMustAdvance,
    ReversePrefillTransition,
    BatchAlreadyInFlight,
    NoBatchInFlight,
    CompletionEpochMismatch,
    CompletionRosterMismatch { lane: usize },
    CompletionDispositionCount,
    QueuePlanMismatch,
    ReadyWorkRequiresQueue,
    PublicationReservationActive,
    PublicationReservationRequired,
    PublicationReservationMismatch,
    PublicationReservationExhausted,
    RegistryIdentityExhausted,
    RegistryIdentityMismatch,
}

/// One exact completion disposition in scheduler roster order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1ServingCompletionDispositionV1 {
    Continue(M1ServingPlanV1),
    Retire,
}

/// Move-only homogeneous roster selected for one physical generation.
#[must_use = "a serving plan must be published, rolled over, or retained"]
#[derive(Debug, Eq, PartialEq)]
pub struct M1ServingBatchPlanV1 {
    plan: M1ServingPlanV1,
    requests: Box<[RequestId]>,
    epoch: CompletionEpoch,
    action: M1ServingQueueActionV1,
}

impl M1ServingBatchPlanV1 {
    #[must_use]
    pub const fn plan(&self) -> M1ServingPlanV1 {
        self.plan
    }

    #[must_use]
    pub fn requests(&self) -> &[RequestId] {
        &self.requests
    }

    #[must_use]
    pub const fn epoch(&self) -> CompletionEpoch {
        self.epoch
    }

    #[must_use]
    pub const fn action(&self) -> M1ServingQueueActionV1 {
        self.action
    }

    fn duplicate(&self) -> Self {
        Self {
            plan: self.plan,
            requests: self.requests.clone(),
            epoch: self.epoch,
            action: self.action,
        }
    }
}

/// Move-only authority for publishing one exact registry roster.
///
/// Reserving does not move requests out of `Ready`. The token instead freezes
/// registry mutations which could invalidate the physical submission. A
/// coordinator may duplicate the immutable batch descriptor for the physical
/// bridge while retaining this capability for the publication join.
#[must_use = "a publication reservation must be recorded or aborted"]
#[derive(Debug, Eq, PartialEq)]
pub struct M1ServingPublicationReservationV1 {
    registry_identity: M1ServingRegistryIdentityV1,
    id: u64,
    batch: M1ServingBatchPlanV1,
}

impl M1ServingPublicationReservationV1 {
    pub(crate) const fn registry_identity(&self) -> M1ServingRegistryIdentityV1 {
        self.registry_identity
    }

    #[must_use]
    pub const fn plan(&self) -> M1ServingPlanV1 {
        self.batch.plan()
    }

    #[must_use]
    pub fn requests(&self) -> &[RequestId] {
        self.batch.requests()
    }

    #[must_use]
    pub const fn epoch(&self) -> CompletionEpoch {
        self.batch.epoch()
    }

    #[must_use]
    pub const fn action(&self) -> M1ServingQueueActionV1 {
        self.batch.action()
    }

    /// Produces the immutable descriptor consumed by the physical bridge while
    /// this move-only reservation remains with the publication coordinator.
    pub fn physical_batch(&self) -> M1ServingBatchPlanV1 {
        self.batch.duplicate()
    }
}

/// Failed publication or abort with the reservation capability retained.
#[must_use]
#[derive(Debug, Eq, PartialEq)]
pub struct M1ServingPublicationFailureV1 {
    error: M1ServingRegistryErrorV1,
    reservation: M1ServingPublicationReservationV1,
}

impl M1ServingPublicationFailureV1 {
    #[must_use]
    pub const fn error(&self) -> M1ServingRegistryErrorV1 {
        self.error
    }

    pub fn into_reservation(self) -> M1ServingPublicationReservationV1 {
        self.reservation
    }
}

#[derive(Debug, Eq, PartialEq)]
struct M1ServingInFlightBatchV1 {
    plan: M1ServingPlanV1,
    requests: Box<[RequestId]>,
    epoch: CompletionEpoch,
}

#[derive(Debug, Eq, PartialEq)]
struct M1ServingReservedBatchV1 {
    id: u64,
    batch: M1ServingBatchPlanV1,
}

/// Deterministic Ferric registry for homogeneous M1 serving batches.
///
/// Prefill has priority over decode, and decode has priority over speculative
/// work. Within one exact plan, admission order is stable. This lets a serving
/// loop hold unlike ready requests while the physical queue executes one valid
/// fixed-batch shape.
pub struct M1ServingRegistryV1<const C: usize> {
    identity: M1ServingRegistryIdentityV1,
    entries: Vec<M1ServingEntryV1>,
    bound_plan: Option<M1ServingPlanV1>,
    reservation: Option<M1ServingReservedBatchV1>,
    in_flight: Option<M1ServingInFlightBatchV1>,
    next_reservation_id: u64,
    submitted_epoch: u64,
    completed_epoch: u64,
}

impl<const C: usize> M1ServingRegistryV1<C> {
    /// Constructs a bounded metadata registry.
    ///
    /// # Errors
    ///
    /// Rejects zero capacity or a capacity above the reviewed M1 sequence cap.
    pub fn new() -> Result<Self, M1ServingRegistryErrorV1> {
        if C == 0 {
            return Err(M1ServingRegistryErrorV1::ZeroCapacity);
        }
        if C > M1_MAX_ACTIVE_SEQUENCES as usize {
            return Err(M1ServingRegistryErrorV1::CapacityExceedsM1);
        }
        let identity = M1ServingRegistryIdentityV1::fresh()?;
        Ok(Self {
            identity,
            entries: Vec::with_capacity(C),
            bound_plan: None,
            reservation: None,
            in_flight: None,
            next_reservation_id: 1,
            submitted_epoch: 0,
            completed_epoch: 0,
        })
    }

    /// Registers a newly Engine-admitted request without mixing it into an
    /// unlike physical batch.
    ///
    /// # Errors
    ///
    /// Rejects invalid or duplicate request generations, full capacity, or a
    /// non-prefill initial plan.
    pub fn admit(
        &mut self,
        request: RequestId,
        prefill: M1ServingPlanV1,
    ) -> Result<(), M1ServingRegistryErrorV1> {
        self.reject_live_reservation()?;
        if request.generation() == 0 {
            return Err(M1ServingRegistryErrorV1::InvalidRequest);
        }
        if prefill.mode() != Qwen3ExecutionMode::Prefill {
            return Err(M1ServingRegistryErrorV1::AdmissionRequiresPrefill);
        }
        if self.entries.iter().any(|entry| entry.request == request) {
            return Err(M1ServingRegistryErrorV1::DuplicateRequest);
        }
        if self.entries.len() == C {
            return Err(M1ServingRegistryErrorV1::OutOfSlots);
        }
        self.entries.push(M1ServingEntryV1 {
            request,
            plan: prefill,
            phase: M1ServingRequestPhaseV1::Ready,
            last_quiescence: None,
        });
        Ok(())
    }

    #[must_use]
    pub fn phase(&self, request: RequestId) -> Option<M1ServingRequestPhaseV1> {
        self.entry(request).map(|entry| entry.phase)
    }

    #[must_use]
    pub fn plan(&self, request: RequestId) -> Option<M1ServingPlanV1> {
        self.entry(request).map(|entry| entry.plan)
    }

    #[must_use]
    pub const fn bound_plan(&self) -> Option<M1ServingPlanV1> {
        self.bound_plan
    }

    #[must_use]
    pub const fn has_in_flight_batch(&self) -> bool {
        self.in_flight.is_some()
    }

    #[must_use]
    pub const fn has_publication_reservation(&self) -> bool {
        self.reservation.is_some()
    }

    /// Classifies the retained physical queue while no generation is in flight.
    ///
    /// # Errors
    ///
    /// Rejects observation while a published batch remains in flight.
    pub fn quiescent_queue_action(
        &self,
    ) -> Result<M1ServingQuiescentQueueActionV1, M1ServingRegistryErrorV1> {
        self.reject_live_reservation()?;
        if self.in_flight.is_some() {
            return Err(M1ServingRegistryErrorV1::BatchAlreadyInFlight);
        }
        let Some(bound) = self.bound_plan else {
            return Ok(M1ServingQuiescentQueueActionV1::NoQueue);
        };
        if self
            .entries
            .iter()
            .any(|entry| entry.phase == M1ServingRequestPhaseV1::Ready)
        {
            Ok(M1ServingQuiescentQueueActionV1::RetainForReadyWork { bound })
        } else {
            Ok(M1ServingQuiescentQueueActionV1::Retire { bound })
        }
    }

    /// Records that the caller destroyed the exact quiescent physical queue
    /// identified by [`Self::quiescent_queue_action`]. A later admission will
    /// consequently require a fresh launch.
    ///
    /// # Errors
    ///
    /// Rejects retirement while a batch is in flight, while ready work still
    /// needs the retained queue, or when the caller names a stale bound plan.
    pub fn record_quiescent_queue_retirement(
        &mut self,
        bound: M1ServingPlanV1,
    ) -> Result<(), M1ServingRegistryErrorV1> {
        self.reject_live_reservation()?;
        if self.in_flight.is_some() {
            return Err(M1ServingRegistryErrorV1::BatchAlreadyInFlight);
        }
        if self.bound_plan != Some(bound) {
            return Err(M1ServingRegistryErrorV1::QueuePlanMismatch);
        }
        if self
            .entries
            .iter()
            .any(|entry| entry.phase == M1ServingRequestPhaseV1::Ready)
        {
            return Err(M1ServingRegistryErrorV1::ReadyWorkRequiresQueue);
        }
        self.bound_plan = None;
        Ok(())
    }

    /// Changes one quiescent member's next execution mode without mutating its
    /// physical cache selection. The later queue action exposes whether a
    /// physical rollover is required.
    ///
    /// # Errors
    ///
    /// Rejects in-flight, cancelled, or never-completed requests, reverse
    /// decode/speculative-to-prefill transitions.
    pub fn transition(
        &mut self,
        request: RequestId,
        next: M1ServingPlanV1,
    ) -> Result<(), M1ServingRegistryErrorV1> {
        self.reject_live_reservation()?;
        let entry = self.entry_mut(request)?;
        if entry.phase != M1ServingRequestPhaseV1::Ready || entry.last_quiescence.is_none() {
            return Err(M1ServingRegistryErrorV1::TransitionRequiresQuiescence);
        }
        validate_plan_transition(entry.plan, next)?;
        entry.plan = next;
        Ok(())
    }

    /// Requests cancellation. A ready request retires at its already-recorded
    /// quiescence point; an in-flight request remains cancellation-pending until
    /// its exact completion is joined.
    ///
    /// # Errors
    ///
    /// Rejects an unknown request or one already cancellation-pending or retired.
    pub fn cancel(
        &mut self,
        request: RequestId,
    ) -> Result<M1ServingRequestPhaseV1, M1ServingRegistryErrorV1> {
        self.reject_live_reservation()?;
        let entry = self.entry_mut(request)?;
        entry.phase = match entry.phase {
            M1ServingRequestPhaseV1::Ready => M1ServingRequestPhaseV1::Retired {
                quiescence: entry.last_quiescence.map_or(
                    M1ServingQuiescenceV1::NeverSubmitted,
                    M1ServingQuiescenceV1::Completed,
                ),
            },
            M1ServingRequestPhaseV1::InFlight { epoch } => {
                M1ServingRequestPhaseV1::CancellationPending { epoch }
            }
            M1ServingRequestPhaseV1::CancellationPending { .. } => {
                return Err(M1ServingRegistryErrorV1::CancellationAlreadyRequested);
            }
            M1ServingRequestPhaseV1::Retired { .. } => {
                return Err(M1ServingRegistryErrorV1::RequestAlreadyRetired);
            }
        };
        Ok(entry.phase)
    }

    /// Selects the next deterministic homogeneous roster without mutating the
    /// registry. `None` means no request is ready.
    ///
    /// # Errors
    ///
    /// Rejects planning while another roster remains in flight or epoch
    /// exhaustion.
    pub fn plan_next(&self) -> Result<Option<M1ServingBatchPlanV1>, M1ServingRegistryErrorV1> {
        self.reject_live_reservation()?;
        if self.in_flight.is_some() {
            return Err(M1ServingRegistryErrorV1::BatchAlreadyInFlight);
        }
        let Some(selected_plan) = self
            .entries
            .iter()
            .filter(|entry| entry.phase == M1ServingRequestPhaseV1::Ready)
            .min_by_key(|entry| plan_priority(entry.plan))
            .map(|entry| entry.plan)
        else {
            return Ok(None);
        };
        let limit = C.min(selected_plan.sequence_capacity());
        let requests = self
            .entries
            .iter()
            .filter(|entry| {
                entry.phase == M1ServingRequestPhaseV1::Ready && entry.plan == selected_plan
            })
            .take(limit)
            .map(|entry| entry.request)
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let next_epoch = self
            .submitted_epoch
            .checked_add(1)
            .ok_or(M1ServingRegistryErrorV1::CompletionEpochMismatch)?;
        let action = classify_queue_action(self.bound_plan, selected_plan);
        Ok(Some(M1ServingBatchPlanV1 {
            plan: selected_plan,
            requests,
            epoch: CompletionEpoch::new(next_epoch),
            action,
        }))
    }

    /// Freezes the exact next deterministic plan, epoch, and ordered roster.
    /// Requests remain `Ready` until physical publication succeeds.
    ///
    /// # Errors
    ///
    /// Rejects an overlapping reservation or in-flight batch, an exhausted
    /// reservation identity, or any stale/reordered/non-deterministic plan.
    pub fn reserve_publication(
        &mut self,
        batch: M1ServingBatchPlanV1,
    ) -> Result<M1ServingPublicationReservationV1, M1ServingRegistryErrorV1> {
        self.reject_live_reservation()?;
        if self.in_flight.is_some() {
            return Err(M1ServingRegistryErrorV1::BatchAlreadyInFlight);
        }
        let Some(expected) = self.plan_next()? else {
            return Err(M1ServingRegistryErrorV1::RequestNotReady);
        };
        if batch != expected {
            return Err(M1ServingRegistryErrorV1::PublicationReservationMismatch);
        }
        let following_id = self
            .next_reservation_id
            .checked_add(1)
            .ok_or(M1ServingRegistryErrorV1::PublicationReservationExhausted)?;
        let id = self.next_reservation_id;
        self.reservation = Some(M1ServingReservedBatchV1 {
            id,
            batch: batch.duplicate(),
        });
        self.next_reservation_id = following_id;
        Ok(M1ServingPublicationReservationV1 {
            registry_identity: self.identity,
            id,
            batch,
        })
    }

    /// Consumes the matching pre-dispatch reservation without advancing the
    /// completion epoch or changing any request, plan, or queue state.
    ///
    /// # Errors
    ///
    /// A stale, forged, or mismatched token is returned to the caller and does
    /// not clear the live reservation.
    pub fn abort_publication(
        &mut self,
        reservation: M1ServingPublicationReservationV1,
    ) -> Result<(), M1ServingPublicationFailureV1> {
        if let Err(error) = self.validate_reservation(&reservation) {
            return Err(M1ServingPublicationFailureV1 { error, reservation });
        }
        self.reservation = None;
        Ok(())
    }

    /// Records successful physical publication of the exact planned roster.
    /// A caller must not invoke this until any required quiescent rollover has
    /// completed and retained physical custody has been rebound.
    ///
    /// # Errors
    ///
    /// Rejects an overlapping batch, stale epoch, missing request, or a roster
    /// whose ready phase or exact plan drifted. Every rejection returns the
    /// reservation for retry or abort.
    ///
    /// ```compile_fail
    /// use ferric_engine::{M1ServingBatchPlanV1, M1ServingRegistryV1};
    /// fn publish_without_reservation(
    ///     registry: &mut M1ServingRegistryV1<32>,
    ///     batch: M1ServingBatchPlanV1,
    /// ) {
    ///     let _ = registry.record_publication(batch);
    /// }
    /// ```
    pub(crate) fn preflight_publication(
        &self,
        reservation: &M1ServingPublicationReservationV1,
    ) -> Result<(), M1ServingRegistryErrorV1> {
        self.validate_reservation(reservation)
    }

    pub(crate) fn record_publication(
        &mut self,
        reservation: M1ServingPublicationReservationV1,
    ) -> Result<(), M1ServingPublicationFailureV1> {
        if let Err(error) = self.validate_reservation(&reservation) {
            return Err(M1ServingPublicationFailureV1 { error, reservation });
        }
        let batch = reservation.batch;
        for entry in &mut self.entries {
            if batch.requests.contains(&entry.request) {
                entry.phase = M1ServingRequestPhaseV1::InFlight { epoch: batch.epoch };
            }
        }
        self.reservation = None;
        self.submitted_epoch = batch.epoch.value();
        self.bound_plan = Some(batch.plan);
        self.in_flight = Some(M1ServingInFlightBatchV1 {
            plan: batch.plan,
            requests: batch.requests,
            epoch: batch.epoch,
        });
        Ok(())
    }

    pub(crate) fn preflight_completion_exact_for(
        &self,
        identity: M1ServingRegistryIdentityV1,
        epoch: CompletionEpoch,
        dispositions: &[M1ServingCompletionDispositionV1],
    ) -> Result<(), M1ServingRegistryErrorV1> {
        if identity != self.identity {
            return Err(M1ServingRegistryErrorV1::RegistryIdentityMismatch);
        }
        self.validate_completion_exact(epoch, dispositions)
    }

    pub(crate) fn apply_preflighted_completion(
        &mut self,
        epoch: CompletionEpoch,
        dispositions: &[M1ServingCompletionDispositionV1],
    ) {
        self.apply_validated_completion(epoch, dispositions);
    }

    #[cfg(test)]
    pub(crate) fn complete_exact(
        &mut self,
        epoch: CompletionEpoch,
        dispositions: &[M1ServingCompletionDispositionV1],
    ) -> Result<(), M1ServingRegistryErrorV1> {
        self.validate_completion_exact(epoch, dispositions)?;
        self.apply_validated_completion(epoch, dispositions);
        Ok(())
    }

    fn apply_validated_completion(
        &mut self,
        epoch: CompletionEpoch,
        dispositions: &[M1ServingCompletionDispositionV1],
    ) {
        let in_flight = self
            .in_flight
            .take()
            .expect("validated completion retains its in-flight batch");
        for (request, disposition) in in_flight.requests.iter().copied().zip(dispositions) {
            let entry = self
                .entries
                .iter_mut()
                .find(|entry| entry.request == request)
                .expect("validated completion retains every roster entry");
            entry.last_quiescence = Some(epoch);
            match disposition {
                M1ServingCompletionDispositionV1::Continue(next) => {
                    entry.plan = *next;
                    entry.phase = M1ServingRequestPhaseV1::Ready;
                }
                M1ServingCompletionDispositionV1::Retire => {
                    entry.phase = M1ServingRequestPhaseV1::Retired {
                        quiescence: M1ServingQuiescenceV1::Completed(epoch),
                    };
                }
            }
        }
        self.completed_epoch = epoch.value();
    }

    fn validate_completion_exact(
        &self,
        epoch: CompletionEpoch,
        dispositions: &[M1ServingCompletionDispositionV1],
    ) -> Result<(), M1ServingRegistryErrorV1> {
        self.reject_live_reservation()?;
        let Some(in_flight) = self.in_flight.as_ref() else {
            return Err(M1ServingRegistryErrorV1::NoBatchInFlight);
        };
        if in_flight.epoch != epoch
            || epoch.value() != self.completed_epoch.saturating_add(1)
            || epoch.value() != self.submitted_epoch
        {
            return Err(M1ServingRegistryErrorV1::CompletionEpochMismatch);
        }
        if dispositions.len() != in_flight.requests.len() {
            return Err(M1ServingRegistryErrorV1::CompletionDispositionCount);
        }
        for (lane, (request, disposition)) in in_flight
            .requests
            .iter()
            .copied()
            .zip(dispositions.iter().copied())
            .enumerate()
        {
            let Some(entry) = self.entries.iter().find(|entry| entry.request == request) else {
                return Err(M1ServingRegistryErrorV1::CompletionRosterMismatch { lane });
            };
            let phase_matches = matches!(
                entry.phase,
                M1ServingRequestPhaseV1::InFlight { epoch: active }
                    | M1ServingRequestPhaseV1::CancellationPending { epoch: active }
                    if active == epoch
            );
            if !phase_matches || entry.plan != in_flight.plan {
                return Err(M1ServingRegistryErrorV1::CompletionRosterMismatch { lane });
            }
            match (entry.phase, disposition) {
                (
                    M1ServingRequestPhaseV1::CancellationPending { .. },
                    M1ServingCompletionDispositionV1::Continue(_),
                ) => return Err(M1ServingRegistryErrorV1::CancellationAlreadyRequested),
                (_, M1ServingCompletionDispositionV1::Continue(next)) => {
                    validate_plan_transition(entry.plan, next)?;
                }
                (_, M1ServingCompletionDispositionV1::Retire) => {}
            }
        }
        Ok(())
    }

    /// Removes one already-quiescent registry record after the caller has
    /// retained or released its physical terminal custody.
    ///
    /// # Errors
    ///
    /// Rejects an unknown request or one that has not reached registry retirement.
    pub fn remove_retired(
        &mut self,
        request: RequestId,
    ) -> Result<M1ServingQuiescenceV1, M1ServingRegistryErrorV1> {
        self.reject_live_reservation()?;
        let Some(index) = self
            .entries
            .iter()
            .position(|entry| entry.request == request)
        else {
            return Err(M1ServingRegistryErrorV1::UnknownRequest);
        };
        let M1ServingRequestPhaseV1::Retired { quiescence } = self.entries[index].phase else {
            return Err(M1ServingRegistryErrorV1::RequestNotReady);
        };
        self.entries.remove(index);
        Ok(quiescence)
    }

    fn entry(&self, request: RequestId) -> Option<&M1ServingEntryV1> {
        self.entries.iter().find(|entry| entry.request == request)
    }

    fn entry_mut(
        &mut self,
        request: RequestId,
    ) -> Result<&mut M1ServingEntryV1, M1ServingRegistryErrorV1> {
        self.entries
            .iter_mut()
            .find(|entry| entry.request == request)
            .ok_or(M1ServingRegistryErrorV1::UnknownRequest)
    }

    fn reject_live_reservation(&self) -> Result<(), M1ServingRegistryErrorV1> {
        if self.reservation.is_some() {
            Err(M1ServingRegistryErrorV1::PublicationReservationActive)
        } else {
            Ok(())
        }
    }

    fn validate_reservation(
        &self,
        reservation: &M1ServingPublicationReservationV1,
    ) -> Result<(), M1ServingRegistryErrorV1> {
        if reservation.registry_identity != self.identity {
            return Err(M1ServingRegistryErrorV1::RegistryIdentityMismatch);
        }
        if self.in_flight.is_some() {
            return Err(M1ServingRegistryErrorV1::BatchAlreadyInFlight);
        }
        let Some(live) = self.reservation.as_ref() else {
            return Err(M1ServingRegistryErrorV1::PublicationReservationRequired);
        };
        if live.id != reservation.id || live.batch != reservation.batch {
            return Err(M1ServingRegistryErrorV1::PublicationReservationMismatch);
        }
        if reservation.batch.epoch.value() != self.submitted_epoch.saturating_add(1) {
            return Err(M1ServingRegistryErrorV1::CompletionEpochMismatch);
        }
        for (lane, request) in reservation.batch.requests.iter().copied().enumerate() {
            let Some(entry) = self.entries.iter().find(|entry| entry.request == request) else {
                return Err(M1ServingRegistryErrorV1::CompletionRosterMismatch { lane });
            };
            if entry.phase != M1ServingRequestPhaseV1::Ready || entry.plan != reservation.batch.plan
            {
                return Err(M1ServingRegistryErrorV1::CompletionRosterMismatch { lane });
            }
        }
        Ok(())
    }
}

fn validate_plan_transition(
    current: M1ServingPlanV1,
    next: M1ServingPlanV1,
) -> Result<(), M1ServingRegistryErrorV1> {
    if current.mode() != Qwen3ExecutionMode::Prefill && next.mode() == Qwen3ExecutionMode::Prefill {
        return Err(M1ServingRegistryErrorV1::ReversePrefillTransition);
    }
    if current.mode() == Qwen3ExecutionMode::Prefill && next.mode() == Qwen3ExecutionMode::Prefill {
        return Err(M1ServingRegistryErrorV1::PrefillMustAdvance);
    }
    Ok(())
}

fn plan_priority(plan: M1ServingPlanV1) -> u8 {
    match plan.mode() {
        Qwen3ExecutionMode::Prefill => 0,
        Qwen3ExecutionMode::Decode => 1,
        Qwen3ExecutionMode::Speculative => 2,
    }
}

fn classify_queue_action(
    prior: Option<M1ServingPlanV1>,
    next: M1ServingPlanV1,
) -> M1ServingQueueActionV1 {
    let Some(prior) = prior else {
        return M1ServingQueueActionV1::FreshLaunch;
    };
    if prior == next {
        return M1ServingQueueActionV1::SameShapeRearm;
    }
    let reason = if prior.mode() != next.mode() {
        M1ServingRolloverReasonV1::Mode
    } else if prior.shape() != next.shape() {
        M1ServingRolloverReasonV1::Shape
    } else {
        M1ServingRolloverReasonV1::Bucket
    };
    M1ServingQueueActionV1::QuiescentRollover {
        prior,
        next,
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selection(
        role: Qwen3ModelRole,
        mode: Qwen3ExecutionMode,
        bucket: Qwen3PlanBucket,
    ) -> Qwen3PlanSelection {
        Qwen3PlanSelection { role, mode, bucket }
    }

    fn pair(mode: Qwen3ExecutionMode, bucket: Qwen3PlanBucket) -> M1ServingPlanV1 {
        let (draft_mode, draft_bucket) = match (mode, bucket) {
            (
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K4C8192
                | Qwen3PlanBucket::SpeculativeS1K8C8192
                | Qwen3PlanBucket::SpeculativeS1K16C8192,
            ) => (Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS1C8192),
            (Qwen3ExecutionMode::Speculative, Qwen3PlanBucket::SpeculativeS8K4C8192) => {
                (Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS8C8192)
            }
            _ => (mode, bucket),
        };
        M1ServingPlanV1::new(
            selection(Qwen3ModelRole::Target8B, mode, bucket),
            selection(Qwen3ModelRole::Draft06B, draft_mode, draft_bucket),
        )
        .unwrap()
    }

    fn prefill_s1() -> M1ServingPlanV1 {
        pair(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128)
    }

    fn prefill_s1_t512() -> M1ServingPlanV1 {
        pair(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T512)
    }

    fn prefill_s8() -> M1ServingPlanV1 {
        pair(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS8T128)
    }

    fn decode_s1() -> M1ServingPlanV1 {
        pair(Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS1C8192)
    }

    fn decode_s8() -> M1ServingPlanV1 {
        pair(Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS8C8192)
    }

    fn speculative_s1() -> M1ServingPlanV1 {
        pair(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
        )
    }

    fn speculative_s1_k8() -> M1ServingPlanV1 {
        pair(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K8C8192,
        )
    }

    fn publish_and_complete<const C: usize>(
        registry: &mut M1ServingRegistryV1<C>,
        dispositions: &[M1ServingCompletionDispositionV1],
    ) -> M1ServingQueueActionV1 {
        let batch = registry.plan_next().unwrap().unwrap();
        let epoch = batch.epoch();
        let action = batch.action();
        reserve_and_record(registry, batch);
        registry.complete_exact(epoch, dispositions).unwrap();
        action
    }

    fn reserve_and_record<const C: usize>(
        registry: &mut M1ServingRegistryV1<C>,
        batch: M1ServingBatchPlanV1,
    ) {
        let reservation = registry.reserve_publication(batch).unwrap();
        registry.record_publication(reservation).unwrap();
    }

    #[test]
    fn speculative_plans_accept_canonical_draft_decode_mappings() {
        let cases = [
            (
                Qwen3PlanBucket::SpeculativeS1K4C8192,
                Qwen3PlanBucket::DecodeS1C8192,
                M1PhysicalFixedBatchShapeV1::SpeculativeK4,
            ),
            (
                Qwen3PlanBucket::SpeculativeS8K4C8192,
                Qwen3PlanBucket::DecodeS8C8192,
                M1PhysicalFixedBatchShapeV1::SpeculativeK4,
            ),
            (
                Qwen3PlanBucket::SpeculativeS1K8C8192,
                Qwen3PlanBucket::DecodeS1C8192,
                M1PhysicalFixedBatchShapeV1::SpeculativeK8,
            ),
            (
                Qwen3PlanBucket::SpeculativeS1K16C8192,
                Qwen3PlanBucket::DecodeS1C8192,
                M1PhysicalFixedBatchShapeV1::SpeculativeK16,
            ),
        ];

        for (target_bucket, draft_bucket, expected_shape) in cases {
            let target = selection(
                Qwen3ModelRole::Target8B,
                Qwen3ExecutionMode::Speculative,
                target_bucket,
            );
            let draft = selection(
                Qwen3ModelRole::Draft06B,
                Qwen3ExecutionMode::Decode,
                draft_bucket,
            );
            let plan = M1ServingPlanV1::new(target, draft).unwrap();

            assert_eq!(plan.target(), target);
            assert_eq!(plan.draft(), draft);
            assert_eq!(plan.shape(), expected_shape);
        }
    }

    #[test]
    fn speculative_plans_reject_noncanonical_draft_mappings() {
        let cases = [
            (
                Qwen3PlanBucket::SpeculativeS1K4C8192,
                Qwen3PlanBucket::DecodeS8C8192,
            ),
            (
                Qwen3PlanBucket::SpeculativeS8K4C8192,
                Qwen3PlanBucket::DecodeS1C8192,
            ),
            (
                Qwen3PlanBucket::SpeculativeS1K8C8192,
                Qwen3PlanBucket::DecodeS8C8192,
            ),
            (
                Qwen3PlanBucket::SpeculativeS1K16C8192,
                Qwen3PlanBucket::DecodeS8C8192,
            ),
        ];

        for (target_bucket, wrong_draft_bucket) in cases {
            let target = selection(
                Qwen3ModelRole::Target8B,
                Qwen3ExecutionMode::Speculative,
                target_bucket,
            );
            let same_speculative_selection = selection(
                Qwen3ModelRole::Draft06B,
                Qwen3ExecutionMode::Speculative,
                target_bucket,
            );
            let wrong_decode_selection = selection(
                Qwen3ModelRole::Draft06B,
                Qwen3ExecutionMode::Decode,
                wrong_draft_bucket,
            );

            assert_eq!(
                M1ServingPlanV1::new(target, same_speculative_selection),
                Err(M1ServingRegistryErrorV1::InvalidPlanPair)
            );
            assert_eq!(
                M1ServingPlanV1::new(target, wrong_decode_selection),
                Err(M1ServingRegistryErrorV1::InvalidPlanPair)
            );
        }
    }

    #[test]
    fn paired_prefill_decode_and_speculative_changes_require_quiescent_rollover() {
        let request = RequestId::new(3, 1);
        let mut registry = M1ServingRegistryV1::<4>::new().unwrap();
        registry.admit(request, prefill_s1()).unwrap();

        assert_eq!(
            publish_and_complete(
                &mut registry,
                &[M1ServingCompletionDispositionV1::Continue(decode_s1())],
            ),
            M1ServingQueueActionV1::FreshLaunch
        );
        let decode = registry.plan_next().unwrap().unwrap();
        assert!(matches!(
            decode.action(),
            M1ServingQueueActionV1::QuiescentRollover {
                reason: M1ServingRolloverReasonV1::Mode,
                ..
            }
        ));
        let epoch = decode.epoch();
        reserve_and_record(&mut registry, decode);
        registry
            .complete_exact(
                epoch,
                &[M1ServingCompletionDispositionV1::Continue(speculative_s1())],
            )
            .unwrap();
        let speculative = registry.plan_next().unwrap().unwrap();
        assert!(matches!(
            speculative.action(),
            M1ServingQueueActionV1::QuiescentRollover {
                reason: M1ServingRolloverReasonV1::Mode,
                ..
            }
        ));
        let epoch = speculative.epoch();
        reserve_and_record(&mut registry, speculative);
        registry
            .complete_exact(
                epoch,
                &[M1ServingCompletionDispositionV1::Continue(
                    speculative_s1_k8(),
                )],
            )
            .unwrap();
        assert!(matches!(
            registry.plan_next().unwrap().unwrap().action(),
            M1ServingQueueActionV1::QuiescentRollover {
                reason: M1ServingRolloverReasonV1::Shape,
                ..
            }
        ));
    }

    #[test]
    fn new_prefill_admission_isolated_then_joins_changed_decode_roster() {
        let first = RequestId::new(0, 1);
        let second = RequestId::new(1, 1);
        let mut registry = M1ServingRegistryV1::<4>::new().unwrap();
        registry.admit(first, prefill_s1()).unwrap();
        publish_and_complete(
            &mut registry,
            &[M1ServingCompletionDispositionV1::Continue(decode_s8())],
        );
        publish_and_complete(
            &mut registry,
            &[M1ServingCompletionDispositionV1::Continue(decode_s8())],
        );

        registry.admit(second, prefill_s1()).unwrap();
        let prefill = registry.plan_next().unwrap().unwrap();
        assert_eq!(prefill.requests(), &[second]);
        let epoch = prefill.epoch();
        reserve_and_record(&mut registry, prefill);
        registry
            .complete_exact(
                epoch,
                &[M1ServingCompletionDispositionV1::Continue(decode_s8())],
            )
            .unwrap();

        let joined = registry.plan_next().unwrap().unwrap();
        assert_eq!(joined.requests(), &[first, second]);
        assert_eq!(joined.plan(), decode_s8());
        assert!(matches!(
            joined.action(),
            M1ServingQueueActionV1::QuiescentRollover { .. }
        ));
    }

    #[test]
    fn unchanged_decode_plan_uses_same_shape_rearm() {
        let request = RequestId::new(0, 1);
        let mut registry = M1ServingRegistryV1::<1>::new().unwrap();
        registry.admit(request, prefill_s1()).unwrap();
        publish_and_complete(
            &mut registry,
            &[M1ServingCompletionDispositionV1::Continue(decode_s1())],
        );
        publish_and_complete(
            &mut registry,
            &[M1ServingCompletionDispositionV1::Continue(decode_s1())],
        );
        assert_eq!(
            registry.plan_next().unwrap().unwrap().action(),
            M1ServingQueueActionV1::SameShapeRearm
        );
    }

    #[test]
    fn quiescent_sequence_bucket_change_requires_rollover() {
        let request = RequestId::new(0, 1);
        let mut registry = M1ServingRegistryV1::<8>::new().unwrap();
        registry.admit(request, prefill_s1()).unwrap();
        publish_and_complete(
            &mut registry,
            &[M1ServingCompletionDispositionV1::Continue(decode_s1())],
        );
        publish_and_complete(
            &mut registry,
            &[M1ServingCompletionDispositionV1::Continue(decode_s1())],
        );

        registry.transition(request, decode_s8()).unwrap();
        assert!(matches!(
            registry.plan_next().unwrap().unwrap().action(),
            M1ServingQueueActionV1::QuiescentRollover {
                reason: M1ServingRolloverReasonV1::Bucket,
                ..
            }
        ));
    }

    #[test]
    fn ready_and_inflight_cancellation_retain_exact_quiescence() {
        let never_submitted = RequestId::new(0, 1);
        let in_flight = RequestId::new(1, 1);
        let mut registry = M1ServingRegistryV1::<2>::new().unwrap();
        registry.admit(never_submitted, prefill_s1()).unwrap();
        assert_eq!(
            registry.cancel(never_submitted).unwrap(),
            M1ServingRequestPhaseV1::Retired {
                quiescence: M1ServingQuiescenceV1::NeverSubmitted
            }
        );
        assert_eq!(
            registry.remove_retired(never_submitted).unwrap(),
            M1ServingQuiescenceV1::NeverSubmitted
        );

        registry.admit(in_flight, prefill_s1()).unwrap();
        let batch = registry.plan_next().unwrap().unwrap();
        let epoch = batch.epoch();
        reserve_and_record(&mut registry, batch);
        assert_eq!(
            registry.cancel(in_flight).unwrap(),
            M1ServingRequestPhaseV1::CancellationPending { epoch }
        );
        registry
            .complete_exact(epoch, &[M1ServingCompletionDispositionV1::Retire])
            .unwrap();
        assert_eq!(
            registry.quiescent_queue_action().unwrap(),
            M1ServingQuiescentQueueActionV1::Retire {
                bound: prefill_s1()
            }
        );
        assert_eq!(
            registry.record_quiescent_queue_retirement(decode_s1()),
            Err(M1ServingRegistryErrorV1::QueuePlanMismatch)
        );
        registry
            .record_quiescent_queue_retirement(prefill_s1())
            .unwrap();
        assert_eq!(
            registry.quiescent_queue_action().unwrap(),
            M1ServingQuiescentQueueActionV1::NoQueue
        );
        let replacement = RequestId::new(0, 2);
        registry.admit(replacement, prefill_s1()).unwrap();
        assert_eq!(
            registry.plan_next().unwrap().unwrap().action(),
            M1ServingQueueActionV1::FreshLaunch
        );
        assert_eq!(
            registry.remove_retired(in_flight).unwrap(),
            M1ServingQuiescenceV1::Completed(epoch)
        );
    }

    #[test]
    fn exact_completion_and_transition_preflights_do_not_mutate_on_rejection() {
        let request = RequestId::new(0, 1);
        let mut registry = M1ServingRegistryV1::<1>::new().unwrap();
        registry.admit(request, prefill_s1()).unwrap();
        assert_eq!(
            registry.transition(request, decode_s1()),
            Err(M1ServingRegistryErrorV1::TransitionRequiresQuiescence)
        );
        let batch = registry.plan_next().unwrap().unwrap();
        let epoch = batch.epoch();
        reserve_and_record(&mut registry, batch);
        assert_eq!(
            registry.complete_exact(CompletionEpoch::new(epoch.value() + 1), &[]),
            Err(M1ServingRegistryErrorV1::CompletionEpochMismatch)
        );
        assert_eq!(
            registry.phase(request),
            Some(M1ServingRequestPhaseV1::InFlight { epoch })
        );
        registry
            .complete_exact(
                epoch,
                &[M1ServingCompletionDispositionV1::Continue(decode_s1())],
            )
            .unwrap();
        assert_eq!(
            registry.transition(request, prefill_s1()),
            Err(M1ServingRegistryErrorV1::ReversePrefillTransition)
        );
        assert_eq!(
            validate_plan_transition(prefill_s1(), prefill_s1()),
            Err(M1ServingRegistryErrorV1::PrefillMustAdvance)
        );
        assert_eq!(registry.plan(request), Some(decode_s1()));
    }

    #[test]
    fn plan_pair_capacity_and_duplicate_admission_fail_closed() {
        assert!(matches!(
            M1ServingRegistryV1::<0>::new(),
            Err(M1ServingRegistryErrorV1::ZeroCapacity)
        ));
        let invalid = M1ServingPlanV1::new(
            Qwen3PlanSelection {
                role: Qwen3ModelRole::Target8B,
                mode: Qwen3ExecutionMode::Decode,
                bucket: Qwen3PlanBucket::DecodeS1C8192,
            },
            Qwen3PlanSelection {
                role: Qwen3ModelRole::Draft06B,
                mode: Qwen3ExecutionMode::Prefill,
                bucket: Qwen3PlanBucket::PrefillS1T128,
            },
        );
        assert_eq!(invalid, Err(M1ServingRegistryErrorV1::InvalidPlanPair));
        let request = RequestId::new(0, 1);
        let mut registry = M1ServingRegistryV1::<1>::new().unwrap();
        registry.admit(request, prefill_s1()).unwrap();
        assert_eq!(
            registry.admit(request, prefill_s1()),
            Err(M1ServingRegistryErrorV1::DuplicateRequest)
        );
    }

    #[test]
    fn live_reservation_excludes_planning_and_invalidating_mutations() {
        let first = RequestId::new(0, 1);
        let second = RequestId::new(1, 1);
        let mut registry = M1ServingRegistryV1::<4>::new().unwrap();
        registry.admit(first, prefill_s1()).unwrap();
        registry.admit(second, prefill_s1_t512()).unwrap();
        let batch = registry.plan_next().unwrap().unwrap();
        let reservation = registry.reserve_publication(batch).unwrap();
        let duplicate_plan = reservation.physical_batch();

        assert!(registry.has_publication_reservation());
        assert_eq!(
            registry.plan_next(),
            Err(M1ServingRegistryErrorV1::PublicationReservationActive)
        );
        assert_eq!(
            registry.reserve_publication(duplicate_plan),
            Err(M1ServingRegistryErrorV1::PublicationReservationActive)
        );
        assert_eq!(
            registry.admit(RequestId::new(2, 1), prefill_s1()),
            Err(M1ServingRegistryErrorV1::PublicationReservationActive)
        );
        assert_eq!(
            registry.transition(first, decode_s1()),
            Err(M1ServingRegistryErrorV1::PublicationReservationActive)
        );
        assert_eq!(
            registry.cancel(second),
            Err(M1ServingRegistryErrorV1::PublicationReservationActive)
        );
        assert_eq!(
            registry.quiescent_queue_action(),
            Err(M1ServingRegistryErrorV1::PublicationReservationActive)
        );
        assert_eq!(
            registry.record_quiescent_queue_retirement(prefill_s1()),
            Err(M1ServingRegistryErrorV1::PublicationReservationActive)
        );
        assert_eq!(
            registry.complete_exact(CompletionEpoch::new(1), &[]),
            Err(M1ServingRegistryErrorV1::PublicationReservationActive)
        );
        assert_eq!(
            registry.remove_retired(second),
            Err(M1ServingRegistryErrorV1::PublicationReservationActive)
        );
        assert_eq!(registry.phase(first), Some(M1ServingRequestPhaseV1::Ready));
        assert_eq!(registry.phase(second), Some(M1ServingRequestPhaseV1::Ready));

        registry.abort_publication(reservation).unwrap();
    }

    #[test]
    fn abort_is_exact_nonmutating_and_does_not_advance_epoch() {
        let request = RequestId::new(0, 1);
        let mut registry = M1ServingRegistryV1::<2>::new().unwrap();
        registry.admit(request, prefill_s1()).unwrap();
        let planned = registry.plan_next().unwrap().unwrap();
        let expected = planned.duplicate();
        let reservation = registry.reserve_publication(planned).unwrap();
        let registry_identity = reservation.registry_identity;
        let stale_id = reservation.id;
        let stale_batch = reservation.physical_batch();

        let forged = M1ServingPublicationReservationV1 {
            registry_identity,
            id: stale_id + 1,
            batch: stale_batch.duplicate(),
        };
        let failure = registry.abort_publication(forged).unwrap_err();
        assert_eq!(
            failure.error(),
            M1ServingRegistryErrorV1::PublicationReservationMismatch
        );
        let _forged = failure.into_reservation();
        assert!(registry.has_publication_reservation());

        registry.abort_publication(reservation).unwrap();
        assert!(!registry.has_publication_reservation());
        assert_eq!(
            registry.phase(request),
            Some(M1ServingRequestPhaseV1::Ready)
        );
        assert_eq!(registry.bound_plan(), None);
        assert_eq!(registry.plan_next().unwrap(), Some(expected));

        let replacement = registry.plan_next().unwrap().unwrap();
        let replacement = registry.reserve_publication(replacement).unwrap();
        let stale = M1ServingPublicationReservationV1 {
            registry_identity,
            id: stale_id,
            batch: stale_batch,
        };
        let failure = registry.abort_publication(stale).unwrap_err();
        assert_eq!(
            failure.error(),
            M1ServingRegistryErrorV1::PublicationReservationMismatch
        );
        assert!(registry.has_publication_reservation());
        registry.abort_publication(replacement).unwrap();
    }

    #[test]
    fn reserve_rejects_a_stale_roster_without_mutation() {
        let stale = RequestId::new(0, 1);
        let next = RequestId::new(1, 1);
        let mut registry = M1ServingRegistryV1::<2>::new().unwrap();
        registry.admit(stale, prefill_s1()).unwrap();
        registry.admit(next, prefill_s1()).unwrap();
        let planned = registry.plan_next().unwrap().unwrap();
        assert_eq!(planned.requests(), &[stale]);
        registry.cancel(stale).unwrap();

        assert_eq!(
            registry.reserve_publication(planned),
            Err(M1ServingRegistryErrorV1::PublicationReservationMismatch)
        );
        assert!(!registry.has_publication_reservation());
        assert_eq!(
            registry.phase(stale),
            Some(M1ServingRequestPhaseV1::Retired {
                quiescence: M1ServingQuiescenceV1::NeverSubmitted,
            })
        );
        assert_eq!(registry.phase(next), Some(M1ServingRequestPhaseV1::Ready));
        assert_eq!(registry.plan_next().unwrap().unwrap().requests(), &[next]);
    }

    #[test]
    fn record_requires_exact_token_epoch_plan_and_ordered_roster() {
        let first = RequestId::new(0, 1);
        let second = RequestId::new(1, 1);
        let unrelated = RequestId::new(2, 1);
        let mut registry = M1ServingRegistryV1::<4>::new().unwrap();
        registry.admit(first, prefill_s8()).unwrap();
        registry.admit(second, prefill_s8()).unwrap();
        registry.admit(unrelated, prefill_s1()).unwrap();
        let batch = registry.plan_next().unwrap().unwrap();
        assert_eq!(batch.requests(), &[first, second]);
        let reservation = registry.reserve_publication(batch).unwrap();
        let registry_identity = reservation.registry_identity;
        let id = reservation.id;

        let wrong_id = M1ServingPublicationReservationV1 {
            registry_identity,
            id: id + 1,
            batch: reservation.physical_batch(),
        };
        let failure = registry.record_publication(wrong_id).unwrap_err();
        assert_eq!(
            failure.error(),
            M1ServingRegistryErrorV1::PublicationReservationMismatch
        );
        let _wrong_id = failure.into_reservation();

        let wrong_epoch = M1ServingPublicationReservationV1 {
            registry_identity,
            id,
            batch: M1ServingBatchPlanV1 {
                plan: reservation.plan(),
                requests: reservation.requests().into(),
                epoch: CompletionEpoch::new(reservation.epoch().value() + 1),
                action: reservation.action(),
            },
        };
        let failure = registry.record_publication(wrong_epoch).unwrap_err();
        assert_eq!(
            failure.error(),
            M1ServingRegistryErrorV1::PublicationReservationMismatch
        );
        let _wrong_epoch = failure.into_reservation();

        let wrong_plan = M1ServingPublicationReservationV1 {
            registry_identity,
            id,
            batch: M1ServingBatchPlanV1 {
                plan: prefill_s1(),
                requests: reservation.requests().into(),
                epoch: reservation.epoch(),
                action: reservation.action(),
            },
        };
        assert_eq!(
            registry.record_publication(wrong_plan).unwrap_err().error(),
            M1ServingRegistryErrorV1::PublicationReservationMismatch
        );

        let reordered = M1ServingPublicationReservationV1 {
            registry_identity,
            id,
            batch: M1ServingBatchPlanV1 {
                plan: reservation.plan(),
                requests: vec![second, first].into_boxed_slice(),
                epoch: reservation.epoch(),
                action: reservation.action(),
            },
        };
        assert_eq!(
            registry.record_publication(reordered).unwrap_err().error(),
            M1ServingRegistryErrorV1::PublicationReservationMismatch
        );
        assert!(registry.has_publication_reservation());

        let epoch = reservation.epoch();
        let replay = M1ServingPublicationReservationV1 {
            registry_identity,
            id,
            batch: reservation.physical_batch(),
        };
        registry.record_publication(reservation).unwrap();
        assert!(!registry.has_publication_reservation());
        assert!(registry.has_in_flight_batch());
        assert_eq!(
            registry.phase(first),
            Some(M1ServingRequestPhaseV1::InFlight { epoch })
        );
        assert_eq!(
            registry.phase(second),
            Some(M1ServingRequestPhaseV1::InFlight { epoch })
        );
        assert_eq!(
            registry.phase(unrelated),
            Some(M1ServingRequestPhaseV1::Ready)
        );
        let failure = registry.record_publication(replay).unwrap_err();
        assert_eq!(
            failure.error(),
            M1ServingRegistryErrorV1::BatchAlreadyInFlight
        );
        let _replay = failure.into_reservation();
    }
}
