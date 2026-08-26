//! Deterministic M1 serving-roster and queue-rollover planning.
//!
//! The physical rearm path intentionally accepts only an unchanged execution
//! plan. This registry is the Ferric-owned boundary that keeps unlike work out
//! of one fixed batch and classifies every next roster as a fresh launch, an
//! unchanged-plan rearm, or a quiescent rollover. A rollover is only a planning
//! result: the caller must retain and rebuild the physical queue custody before
//! publishing the returned roster.

use ferric_spec::{
    completion::CompletionEpoch, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket,
    Qwen3PlanSelection, RequestId, M1_MAX_ACTIVE_SEQUENCES,
};

use crate::M1PhysicalFixedBatchShapeV1;

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
            || target.mode != draft.mode
            || target.bucket != draft.bucket
            || target.validate().is_err()
            || draft.validate().is_err()
        {
            return Err(M1ServingRegistryErrorV1::InvalidPlanPair);
        }
        let shape = match (target.mode, target.bucket) {
            (
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T128
                | Qwen3PlanBucket::PrefillS8T128
                | Qwen3PlanBucket::PrefillS1T512
                | Qwen3PlanBucket::PrefillS1T2048,
            ) => M1PhysicalFixedBatchShapeV1::PairedPrefill,
            (
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS1C8192
                | Qwen3PlanBucket::DecodeS8C8192
                | Qwen3PlanBucket::DecodeS32C8192,
            ) => M1PhysicalFixedBatchShapeV1::TargetOnly,
            (
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K4C8192 | Qwen3PlanBucket::SpeculativeS8K4C8192,
            ) => M1PhysicalFixedBatchShapeV1::SpeculativeK4,
            (Qwen3ExecutionMode::Speculative, Qwen3PlanBucket::SpeculativeS1K8C8192) => {
                M1PhysicalFixedBatchShapeV1::SpeculativeK8
            }
            (Qwen3ExecutionMode::Speculative, Qwen3PlanBucket::SpeculativeS1K16C8192) => {
                M1PhysicalFixedBatchShapeV1::SpeculativeK16
            }
            _ => return Err(M1ServingRegistryErrorV1::InvalidPlanPair),
        };
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
}

#[derive(Debug, Eq, PartialEq)]
struct M1ServingInFlightBatchV1 {
    plan: M1ServingPlanV1,
    requests: Box<[RequestId]>,
    epoch: CompletionEpoch,
}

/// Deterministic Ferric registry for homogeneous M1 serving batches.
///
/// Prefill has priority over decode, and decode has priority over speculative
/// work. Within one exact plan, admission order is stable. This lets a serving
/// loop hold unlike ready requests while the physical queue executes one valid
/// fixed-batch shape.
pub struct M1ServingRegistryV1<const C: usize> {
    entries: Vec<M1ServingEntryV1>,
    bound_plan: Option<M1ServingPlanV1>,
    in_flight: Option<M1ServingInFlightBatchV1>,
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
        Ok(Self {
            entries: Vec::with_capacity(C),
            bound_plan: None,
            in_flight: None,
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

    /// Classifies the retained physical queue while no generation is in flight.
    ///
    /// # Errors
    ///
    /// Rejects observation while a published batch remains in flight.
    pub fn quiescent_queue_action(
        &self,
    ) -> Result<M1ServingQuiescentQueueActionV1, M1ServingRegistryErrorV1> {
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

    /// Records successful physical publication of the exact planned roster.
    /// A caller must not invoke this until any required quiescent rollover has
    /// completed and retained physical custody has been rebound.
    ///
    /// # Errors
    ///
    /// Rejects an overlapping batch, stale epoch, missing request, or a roster
    /// whose ready phase or exact plan changed after planning.
    pub fn record_publication(
        &mut self,
        batch: M1ServingBatchPlanV1,
    ) -> Result<(), M1ServingRegistryErrorV1> {
        if self.in_flight.is_some() {
            return Err(M1ServingRegistryErrorV1::BatchAlreadyInFlight);
        }
        if batch.epoch.value() != self.submitted_epoch.saturating_add(1) {
            return Err(M1ServingRegistryErrorV1::CompletionEpochMismatch);
        }
        for (lane, request) in batch.requests.iter().copied().enumerate() {
            let Some(entry) = self.entries.iter().find(|entry| entry.request == request) else {
                return Err(M1ServingRegistryErrorV1::CompletionRosterMismatch { lane });
            };
            if entry.phase != M1ServingRequestPhaseV1::Ready || entry.plan != batch.plan {
                return Err(M1ServingRegistryErrorV1::CompletionRosterMismatch { lane });
            }
        }
        for request in batch.requests.iter().copied() {
            self.entry_mut(request)?.phase =
                M1ServingRequestPhaseV1::InFlight { epoch: batch.epoch };
        }
        self.submitted_epoch = batch.epoch.value();
        self.bound_plan = Some(batch.plan);
        self.in_flight = Some(M1ServingInFlightBatchV1 {
            plan: batch.plan,
            requests: batch.requests,
            epoch: batch.epoch,
        });
        Ok(())
    }

    /// Joins one exact completed generation and applies the complete ordered
    /// disposition roster atomically after preflight.
    ///
    /// # Errors
    ///
    /// Rejects an absent batch, stale/reordered epoch, incomplete disposition
    /// roster, request/plan drift, continuation after cancellation, or an
    /// invalid next-plan transition.
    pub fn complete_exact(
        &mut self,
        epoch: CompletionEpoch,
        dispositions: &[M1ServingCompletionDispositionV1],
    ) -> Result<(), M1ServingRegistryErrorV1> {
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
        let in_flight = self
            .in_flight
            .take()
            .ok_or(M1ServingRegistryErrorV1::NoBatchInFlight)?;
        for (request, disposition) in in_flight.requests.iter().copied().zip(dispositions) {
            let entry = self.entry_mut(request)?;
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

    fn pair(mode: Qwen3ExecutionMode, bucket: Qwen3PlanBucket) -> M1ServingPlanV1 {
        M1ServingPlanV1::new(
            Qwen3PlanSelection {
                role: Qwen3ModelRole::Target8B,
                mode,
                bucket,
            },
            Qwen3PlanSelection {
                role: Qwen3ModelRole::Draft06B,
                mode,
                bucket,
            },
        )
        .unwrap()
    }

    fn prefill_s1() -> M1ServingPlanV1 {
        pair(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128)
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
        registry.record_publication(batch).unwrap();
        registry.complete_exact(epoch, dispositions).unwrap();
        action
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
        registry.record_publication(decode).unwrap();
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
        registry.record_publication(speculative).unwrap();
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
        registry.record_publication(prefill).unwrap();
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
        registry.record_publication(batch).unwrap();
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
        registry.record_publication(batch).unwrap();
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
}
