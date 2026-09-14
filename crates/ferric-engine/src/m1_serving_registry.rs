//! Deterministic M1 serving-roster and queue-rollover planning.
//!
//! The physical rearm path intentionally accepts only an unchanged execution
//! plan. This registry is the Ferric-owned boundary that keeps unlike work out
//! of one fixed batch and classifies every next roster as a fresh launch, an
//! unchanged-plan rearm, or a quiescent rollover. A rollover is only a planning
//! result: the caller must retain and rebuild the physical queue custody before
//! publishing the returned roster.

use core::{
    fmt,
    sync::atomic::{AtomicU64, Ordering},
};

use arrayvec::ArrayVec;
use ferric_spec::{
    completion::CompletionEpoch, scheduling::RequestState, PhysicalKvLifecycle, Qwen3ExecutionMode,
    Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection, RequestId, M1_KV_PAGE_TOKENS,
    M1_MAX_ACTIVE_SEQUENCES,
};

const M1_SERVING_INLINE_ROSTER_CAPACITY_V1: usize = M1_MAX_ACTIVE_SEQUENCES as usize;
type M1ServingInlineRosterV1 = ArrayVec<RequestId, M1_SERVING_INLINE_ROSTER_CAPACITY_V1>;

use crate::{
    authenticated_queue_rollover::join_m1_authenticated_prefill_registry_intent_v1,
    CheckedCompletionSemantics, M1AuthenticatedS1T128PrefillExecutionSuccessV1,
    M1PhysicalFixedBatchShapeV1, M1ReleasedDeviceKvMemberV1,
};

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

    /// Exact draft selection retained by device-KV state.
    ///
    /// Speculative execution reuses a one-token draft-decode workspace, but
    /// its device-KV reservation spans the complete speculative round. The KV
    /// cache therefore retains the role-adjusted speculative selection while
    /// [`Self::draft`] continues to name the reusable decode workspace.
    #[must_use]
    pub const fn draft_cache_selection(self) -> Qwen3PlanSelection {
        if matches!(self.target.mode, Qwen3ExecutionMode::Speculative) {
            Qwen3PlanSelection {
                role: Qwen3ModelRole::Draft06B,
                mode: self.target.mode,
                bucket: self.target.bucket,
            }
        } else {
            self.draft
        }
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
    /// A sealed full-acceptance witness requires one draft-only maintenance step.
    QuiescentDraftCatchup { parent: M1ServingPlanV1 },
    /// Checked maintenance must restore the original speculative physical shape.
    QuiescentDraftCatchupRestore { parent: M1ServingPlanV1 },
    /// The prior generation is quiescent, but its physical custody must be
    /// rebuilt before this roster can be published.
    QuiescentRollover {
        prior: M1ServingPlanV1,
        next: M1ServingPlanV1,
        reason: M1ServingRolloverReasonV1,
    },
    /// Every member of one bounded serving window retired, and the exact
    /// retained queue must be rebound to a fresh paired-prefill roster.
    ///
    /// This is not a continuous-batching or late-arrival transition. It is
    /// available only through the all-terminal window reservation below.
    QuiescentNewWindow {
        prior: M1ServingPlanV1,
        next: M1ServingPlanV1,
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
    /// No ready request needs this exact queue or one of its admitted native
    /// rollover paths, so the quiescent physical queue may retire.
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

/// Preallocated private entry storage for an all-terminal window replacement.
///
/// The entries remain inaccessible so this storage cannot manufacture registry
/// membership or publication authority.
#[derive(Debug)]
pub(crate) struct M1ServingNewWindowEntryStorageV1 {
    entries: Vec<M1ServingEntryV1>,
}

impl M1ServingNewWindowEntryStorageV1 {
    pub(crate) fn try_new(capacity: usize) -> Option<Self> {
        let mut entries = Vec::new();
        entries.try_reserve_exact(capacity).ok()?;
        Some(Self { entries })
    }
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
    QueueTransitionUnsupported,
    ReadyWorkRequiresQueue,
    PublicationReservationActive,
    PublicationReservationRequired,
    PublicationReservationMismatch,
    PublicationReservationExhausted,
    NewWindowPredecessorRequired,
    NewWindowPredecessorNotCompleted { index: usize },
    NewWindowPredecessorPlanMismatch { index: usize },
    NewWindowPredecessorEpochMismatch { index: usize },
    NewWindowRosterEmpty,
    NewWindowRosterExceedsPlan,
    NewWindowRosterCountMismatch,
    NewWindowRequestSlotMismatch { index: usize },
    NewWindowRequestGenerationMismatch { index: usize },
    HostAllocation,
    RegistryIdentityExhausted,
    RegistryIdentityMismatch,
    DraftCatchupWitnessRequired,
    DraftCatchupPhaseMismatch,
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
    requests: M1ServingInlineRosterV1,
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
    reservation: Box<M1ServingPublicationReservationV1>,
}

/// Copy-only registry evidence for one member removed by a new-window
/// reservation.
///
/// This value grants no Engine, queue, cache, page, or allocation authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1ServingCompletedWindowMemberV1 {
    request: RequestId,
    plan: M1ServingPlanV1,
    completion_epoch: CompletionEpoch,
}

impl M1ServingCompletedWindowMemberV1 {
    #[must_use]
    pub const fn request(self) -> RequestId {
        self.request
    }

    #[must_use]
    pub const fn plan(self) -> M1ServingPlanV1 {
        self.plan
    }

    #[must_use]
    pub const fn completion_epoch(self) -> CompletionEpoch {
        self.completion_epoch
    }
}

/// Move-only registry transaction for replacing one all-terminal bounded
/// window with one fresh paired-prefill roster.
///
/// The predecessor records remain here rather than being reconstructed from
/// public projections. Until this owner is committed or restored, the
/// registry's ordinary reservation exclusion prevents every conflicting
/// mutation.
#[must_use = "the all-terminal window reservation must be published, restored, or retained by terminal custody"]
#[derive(Debug, Eq, PartialEq)]
pub struct M1ServingNewWindowPublicationReservationV1 {
    prior: M1ServingPlanV1,
    predecessors: Vec<M1ServingEntryV1>,
    requests: Box<[RequestId]>,
    publication: M1ServingPublicationReservationV1,
}

impl M1ServingNewWindowPublicationReservationV1 {
    pub(crate) const fn registry_identity(&self) -> M1ServingRegistryIdentityV1 {
        self.publication.registry_identity()
    }

    #[must_use]
    pub const fn prior_plan(&self) -> M1ServingPlanV1 {
        self.prior
    }

    #[must_use]
    pub fn predecessor_count(&self) -> usize {
        self.predecessors.len()
    }

    #[must_use]
    pub fn predecessor(&self, index: usize) -> Option<M1ServingCompletedWindowMemberV1> {
        let entry = self.predecessors.get(index)?;
        let M1ServingRequestPhaseV1::Retired {
            quiescence: M1ServingQuiescenceV1::Completed(completion_epoch),
        } = entry.phase
        else {
            debug_assert!(false);
            return None;
        };
        Some(M1ServingCompletedWindowMemberV1 {
            request: entry.request,
            plan: entry.plan,
            completion_epoch,
        })
    }

    #[must_use]
    pub const fn plan(&self) -> M1ServingPlanV1 {
        self.publication.plan()
    }

    #[must_use]
    pub fn requests(&self) -> &[RequestId] {
        self.publication.requests()
    }

    #[must_use]
    pub const fn epoch(&self) -> CompletionEpoch {
        self.publication.epoch()
    }

    #[must_use]
    pub const fn action(&self) -> M1ServingQueueActionV1 {
        self.publication.action()
    }

    /// Produces the immutable descriptor used for pre-publication physical
    /// checks while this transaction retains both registry rosters.
    pub fn physical_batch(&self) -> M1ServingBatchPlanV1 {
        self.publication.physical_batch()
    }
}

/// Rejected new-window reservation with the caller's proposed roster intact.
#[must_use = "the rejected fresh roster remains owned by the caller"]
#[derive(Debug, Eq, PartialEq)]
pub struct M1ServingNewWindowReservationFailureV1 {
    error: M1ServingRegistryErrorV1,
    plan: M1ServingPlanV1,
    requests: Box<[RequestId]>,
}

impl M1ServingNewWindowReservationFailureV1 {
    #[must_use]
    pub const fn error(&self) -> M1ServingRegistryErrorV1 {
        self.error
    }

    #[must_use]
    pub const fn plan(&self) -> M1ServingPlanV1 {
        self.plan
    }

    #[must_use = "the rejected fresh roster remains linear caller input"]
    pub fn into_requests(self) -> Box<[RequestId]> {
        self.requests
    }
}

/// Failed commit or restoration with the complete two-roster transaction
/// retained.
#[must_use = "the exact new-window reservation remains live"]
#[derive(Debug, Eq, PartialEq)]
pub struct M1ServingNewWindowPublicationFailureV1 {
    error: M1ServingRegistryErrorV1,
    reservation: M1ServingNewWindowPublicationReservationV1,
}

impl M1ServingNewWindowPublicationFailureV1 {
    #[must_use]
    pub const fn error(&self) -> M1ServingRegistryErrorV1 {
        self.error
    }

    #[must_use = "the exact new-window reservation remains live"]
    pub fn into_reservation(self) -> M1ServingNewWindowPublicationReservationV1 {
        self.reservation
    }
}

/// Fresh roster recovered when an all-terminal window replacement is aborted
/// before any physical side effect.
#[must_use = "the restored fresh roster remains caller-owned"]
#[derive(Debug, Eq, PartialEq)]
pub struct M1ServingRestoredNewWindowV1 {
    plan: M1ServingPlanV1,
    requests: Box<[RequestId]>,
}

impl M1ServingRestoredNewWindowV1 {
    #[must_use]
    pub const fn plan(&self) -> M1ServingPlanV1 {
        self.plan
    }

    #[must_use = "the restored fresh roster remains caller-owned"]
    pub fn into_requests(self) -> Box<[RequestId]> {
        self.requests
    }
}

impl M1ServingPublicationFailureV1 {
    #[must_use]
    pub const fn error(&self) -> M1ServingRegistryErrorV1 {
        self.error
    }

    pub fn into_reservation(self) -> M1ServingPublicationReservationV1 {
        *self.reservation
    }
}

#[derive(Debug, Eq, PartialEq)]
struct M1ServingInFlightBatchV1 {
    plan: M1ServingPlanV1,
    requests: M1ServingInlineRosterV1,
    epoch: CompletionEpoch,
}

#[derive(Debug, Eq, PartialEq)]
struct M1ServingReservedBatchV1 {
    id: u64,
    plan: M1ServingPlanV1,
    epoch: CompletionEpoch,
    action: M1ServingQueueActionV1,
    draft_catchup: Option<M1ServingDraftCatchupKeyV1>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct M1ServingDraftCatchupKeyV1 {
    coordinator: crate::speculative_generation_loop::M1SpeculativeCoordinatorIdentityV1,
    parent: Qwen3PlanSelection,
    request: RequestId,
    completed_round: u64,
    prior_epoch: CompletionEpoch,
    epoch: CompletionEpoch,
    draft_committed: u32,
    target_committed: u32,
    token: ferric_spec::TokenId,
    prior_dispatch_generation: u64,
}

impl M1ServingDraftCatchupKeyV1 {
    fn from_pending(
        pending: &crate::authenticated_speculative_executor::M1AuthenticatedDraftCatchupPendingV1,
    ) -> Self {
        Self {
            coordinator: pending.coordinator_identity(),
            parent: pending.parent(),
            request: pending.request(),
            completed_round: pending.completed_round(),
            prior_epoch: pending.prior_epoch(),
            epoch: pending.epoch(),
            draft_committed: pending.draft_committed(),
            target_committed: pending.target_committed(),
            token: pending.token(),
            prior_dispatch_generation: pending.prior_dispatch_generation(),
        }
    }

    fn valid_for(self, plan: M1ServingPlanV1) -> bool {
        self.parent == plan.target()
            && plan.mode() == Qwen3ExecutionMode::Speculative
            && plan.sequence_capacity() == 1
            && self.request.generation() != 0
            && self.completed_round.checked_add(1).is_some()
            && self.prior_epoch.value() != 0
            && self.prior_epoch.value().checked_add(1) == Some(self.epoch.value())
            && self.prior_dispatch_generation != 0
            && self.prior_dispatch_generation.checked_add(1).is_some()
            && self.draft_committed.checked_add(1) == Some(self.target_committed)
            && self.target_committed <= ferric_spec::M1_MAX_CONTEXT_TOKENS
            && self.token < ferric_spec::QWEN3_VOCABULARY_SIZE
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum M1ServingDraftCatchupPhaseV1 {
    Pending(M1ServingDraftCatchupKeyV1),
    InFlight(M1ServingDraftCatchupKeyV1),
    Completed {
        key: M1ServingDraftCatchupKeyV1,
        dispatch_generation: u64,
    },
}

impl M1ServingDraftCatchupPhaseV1 {
    const fn key(self) -> M1ServingDraftCatchupKeyV1 {
        match self {
            Self::Pending(key) | Self::InFlight(key) | Self::Completed { key, .. } => key,
        }
    }
}

/// Fail-closed rejection while reconciling a real authenticated paired-prefill
/// completion into a fresh serving registry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedPrefillRegistryReconciliationErrorV1 {
    RegistryNotFresh,
    ProfileMismatch,
    IntentMismatch,
    QueueMismatch,
    CompletionMismatch,
    RequestMismatch,
    CacheMismatch,
    EngineMismatch,
}

/// Opaque terminal custody after authenticated prefill registry reconciliation
/// rejects. The registry and physical completion cannot be extracted.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedPrefillRegistryReconciliationFailureV1;
/// fn extract<const C: usize>(failure: M1AuthenticatedPrefillRegistryReconciliationFailureV1<C>) {
///     let _ = failure.into_parts();
/// }
/// ```
#[must_use = "failed authenticated prefill reconciliation retains all consumed custody"]
pub struct M1AuthenticatedPrefillRegistryReconciliationFailureV1<const C: usize> {
    error: M1AuthenticatedPrefillRegistryReconciliationErrorV1,
    _registry: M1ServingRegistryV1<C>,
    _completed: M1AuthenticatedS1T128PrefillExecutionSuccessV1<C>,
}

impl<const C: usize> fmt::Debug for M1AuthenticatedPrefillRegistryReconciliationFailureV1<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedPrefillRegistryReconciliationFailureV1")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl<const C: usize> M1AuthenticatedPrefillRegistryReconciliationFailureV1<C> {
    #[must_use]
    pub const fn error(&self) -> M1AuthenticatedPrefillRegistryReconciliationErrorV1 {
        self.error
    }

    #[allow(clippy::boxed_local)]
    pub(crate) fn into_resident_teardown(
        self: Box<Self>,
    ) -> crate::authenticated_resident_session::M1AuthenticatedResidentQueueTeardownV1 {
        let Self {
            error,
            _registry: registry,
            _completed: completed,
        } = *self;
        completed.close_for_resident().retain((registry, error))
    }
}

/// Move-only real paired-prefill completion joined to its exact registry
/// frontier. Queue, cache, page, and Engine owners remain opaque.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedPrefillRegistryReconciledV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1AuthenticatedPrefillRegistryReconciledV1<1>>();
/// ```
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedPrefillRegistryReconciledV1;
/// fn extract(reconciled: M1AuthenticatedPrefillRegistryReconciledV1<1>) {
///     let _ = reconciled.into_parts();
/// }
/// ```
#[must_use = "authenticated prefill and registry custody remain joined"]
pub struct M1AuthenticatedPrefillRegistryReconciledV1<const C: usize> {
    registry: M1ServingRegistryV1<C>,
    completed: M1AuthenticatedS1T128PrefillExecutionSuccessV1<C>,
    request: RequestId,
    epoch: CompletionEpoch,
    prefill: M1ServingPlanV1,
    successor: M1ServingPlanV1,
}

impl<const C: usize> fmt::Debug for M1AuthenticatedPrefillRegistryReconciledV1<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedPrefillRegistryReconciledV1")
            .field("request", &self.request)
            .field("epoch", &self.epoch)
            .field("prefill", &self.prefill)
            .field("successor", &self.successor)
            .finish_non_exhaustive()
    }
}

impl<const C: usize> M1AuthenticatedPrefillRegistryReconciledV1<C> {
    #[must_use]
    pub const fn request(&self) -> RequestId {
        self.request
    }

    #[must_use]
    pub const fn completion_epoch(&self) -> CompletionEpoch {
        self.epoch
    }

    #[must_use]
    pub const fn prefill_plan(&self) -> M1ServingPlanV1 {
        self.prefill
    }

    #[must_use]
    pub const fn successor_plan(&self) -> M1ServingPlanV1 {
        self.successor
    }

    #[must_use]
    pub const fn first_token(&self) -> ferric_spec::TokenId {
        self.completed.first_token()
    }

    #[must_use]
    pub const fn successor_epoch(&self) -> CompletionEpoch {
        CompletionEpoch::new(2)
    }

    pub(crate) const fn retained_logical_runner(&self) -> &crate::LogicalRunnerDeclaration {
        self.completed.released().queue().logical_runner()
    }

    pub(crate) fn permits_speculative_successor(&self) -> bool {
        self.completed.permits_speculative_successor()
    }

    pub(crate) fn close_for_resident(
        self,
    ) -> crate::authenticated_resident_session::M1AuthenticatedResidentQueueTeardownV1 {
        let Self {
            registry,
            completed,
            request,
            epoch,
            prefill,
            successor,
        } = self;
        completed
            .close_for_resident()
            .retain((registry, request, epoch, prefill, successor))
    }

    #[allow(dead_code)]
    pub(crate) fn into_parts(
        self,
    ) -> (
        M1ServingRegistryV1<C>,
        M1AuthenticatedS1T128PrefillExecutionSuccessV1<C>,
    ) {
        (self.registry, self.completed)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct M1AuthenticatedPrefillRegistryReconciliationV1 {
    request: RequestId,
    epoch: CompletionEpoch,
    prefill: M1ServingPlanV1,
    successor: M1ServingPlanV1,
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
    draft_catchup: Option<M1ServingDraftCatchupPhaseV1>,
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
            draft_catchup: None,
            next_reservation_id: 1,
            submitted_epoch: 0,
            completed_epoch: 0,
        })
    }

    fn is_fresh_for_authenticated_prefill_reconciliation(&self) -> bool {
        self.entries.is_empty()
            && self.bound_plan.is_none()
            && self.reservation.is_none()
            && self.in_flight.is_none()
            && self.draft_catchup.is_none()
            && self.next_reservation_id == 1
            && self.submitted_epoch == 0
            && self.completed_epoch == 0
    }

    fn install_authenticated_prefill_reconciliation(
        &mut self,
        reconciliation: M1AuthenticatedPrefillRegistryReconciliationV1,
    ) -> Result<(), M1AuthenticatedPrefillRegistryReconciliationErrorV1> {
        if !self.is_fresh_for_authenticated_prefill_reconciliation() {
            return Err(M1AuthenticatedPrefillRegistryReconciliationErrorV1::RegistryNotFresh);
        }
        self.entries.push(M1ServingEntryV1 {
            request: reconciliation.request,
            plan: reconciliation.successor,
            phase: M1ServingRequestPhaseV1::Ready,
            last_quiescence: Some(reconciliation.epoch),
        });
        self.bound_plan = Some(reconciliation.prefill);
        // The real prefill occupied the same first publication nonce as the
        // ordinary path; the next reservation must therefore start at two.
        self.next_reservation_id = 2;
        self.submitted_epoch = reconciliation.epoch.value();
        self.completed_epoch = reconciliation.epoch.value();
        Ok(())
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

    /// Physical queue shape; the logical bound plan remains the speculative parent.
    #[must_use]
    pub const fn bound_shape(&self) -> Option<M1PhysicalFixedBatchShapeV1> {
        if matches!(
            self.draft_catchup,
            Some(
                M1ServingDraftCatchupPhaseV1::InFlight(_)
                    | M1ServingDraftCatchupPhaseV1::Completed { .. }
            )
        ) {
            Some(M1PhysicalFixedBatchShapeV1::DraftCatchup)
        } else {
            match self.bound_plan {
                Some(plan) => Some(plan.shape()),
                None => None,
            }
        }
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
        if self.ready_work_needs_bound_queue(bound) {
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
        if self.ready_work_needs_bound_queue(bound) {
            return Err(M1ServingRegistryErrorV1::ReadyWorkRequiresQueue);
        }
        self.bound_plan = None;
        self.draft_catchup = None;
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
        if self.draft_catchup.is_some() {
            return Err(M1ServingRegistryErrorV1::DraftCatchupPhaseMismatch);
        }
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
        let Some(selected_plan) = self.next_ready_plan() else {
            return Ok(None);
        };
        let limit = C.min(selected_plan.sequence_capacity());
        let mut requests = M1ServingInlineRosterV1::new();
        if let Some(phase) = self.draft_catchup {
            let key = phase.key();
            if self.entry(key.request).is_none_or(|entry| {
                entry.phase != M1ServingRequestPhaseV1::Ready || entry.plan != selected_plan
            }) {
                return Err(M1ServingRegistryErrorV1::DraftCatchupPhaseMismatch);
            }
            requests
                .try_push(key.request)
                .map_err(|_| M1ServingRegistryErrorV1::CapacityExceedsM1)?;
        } else {
            for entry in self
                .entries
                .iter()
                .filter(|entry| {
                    entry.phase == M1ServingRequestPhaseV1::Ready && entry.plan == selected_plan
                })
                .take(limit)
            {
                requests
                    .try_push(entry.request)
                    .map_err(|_| M1ServingRegistryErrorV1::CapacityExceedsM1)?;
            }
        }
        let next_epoch = self
            .submitted_epoch
            .checked_add(1)
            .ok_or(M1ServingRegistryErrorV1::CompletionEpochMismatch)?;
        let action = self.classify_next_queue_action(selected_plan)?;
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
        if matches!(
            batch.action,
            M1ServingQueueActionV1::QuiescentDraftCatchup { .. }
                | M1ServingQueueActionV1::QuiescentDraftCatchupRestore { .. }
        ) {
            return Err(M1ServingRegistryErrorV1::DraftCatchupWitnessRequired);
        }
        self.validate_next_batch(&batch)?;
        self.reserve_validated_batch(batch, None)
    }

    /// Records the sealed full-acceptance obligation before next-step planning.
    pub(crate) fn register_draft_catchup_pending(
        &mut self,
        pending: &crate::authenticated_speculative_executor::M1AuthenticatedDraftCatchupPendingV1,
    ) -> Result<(), M1ServingRegistryErrorV1> {
        self.register_draft_catchup_key(M1ServingDraftCatchupKeyV1::from_pending(pending))
    }

    fn register_draft_catchup_key(
        &mut self,
        key: M1ServingDraftCatchupKeyV1,
    ) -> Result<(), M1ServingRegistryErrorV1> {
        self.reject_live_reservation()?;
        if self.in_flight.is_some() {
            return Err(M1ServingRegistryErrorV1::BatchAlreadyInFlight);
        }
        if self.draft_catchup.is_some() {
            return Err(M1ServingRegistryErrorV1::DraftCatchupPhaseMismatch);
        }
        let parent = self
            .bound_plan
            .ok_or(M1ServingRegistryErrorV1::QueuePlanMismatch)?;
        self.validate_draft_catchup_ready_key(parent, key)?;
        self.draft_catchup = Some(M1ServingDraftCatchupPhaseV1::Pending(key));
        Ok(())
    }

    pub(crate) fn reserve_draft_catchup_publication(
        &mut self,
        batch: M1ServingBatchPlanV1,
        pending: &crate::authenticated_speculative_executor::M1AuthenticatedDraftCatchupPendingV1,
    ) -> Result<M1ServingPublicationReservationV1, M1ServingRegistryErrorV1> {
        self.reserve_draft_catchup_key(batch, M1ServingDraftCatchupKeyV1::from_pending(pending))
    }

    fn reserve_draft_catchup_key(
        &mut self,
        batch: M1ServingBatchPlanV1,
        key: M1ServingDraftCatchupKeyV1,
    ) -> Result<M1ServingPublicationReservationV1, M1ServingRegistryErrorV1> {
        self.reject_live_reservation()?;
        if self.in_flight.is_some() {
            return Err(M1ServingRegistryErrorV1::BatchAlreadyInFlight);
        }
        self.validate_next_batch(&batch)?;
        self.validate_draft_catchup_start(&batch, key)?;
        self.reserve_validated_batch(batch, Some(key))
    }

    pub(crate) fn reserve_draft_catchup_restore_publication(
        &mut self,
        batch: M1ServingBatchPlanV1,
        completed: &crate::authenticated_speculative_executor::M1AuthenticatedDraftCatchupCompletedV1,
    ) -> Result<M1ServingPublicationReservationV1, M1ServingRegistryErrorV1> {
        self.reserve_draft_catchup_restore_key(
            batch,
            M1ServingDraftCatchupKeyV1::from_pending(completed.pending()),
            completed.dispatch_generation(),
        )
    }

    fn reserve_draft_catchup_restore_key(
        &mut self,
        batch: M1ServingBatchPlanV1,
        key: M1ServingDraftCatchupKeyV1,
        dispatch_generation: u64,
    ) -> Result<M1ServingPublicationReservationV1, M1ServingRegistryErrorV1> {
        self.reject_live_reservation()?;
        if self.in_flight.is_some() {
            return Err(M1ServingRegistryErrorV1::BatchAlreadyInFlight);
        }
        if self.draft_catchup
            != Some(M1ServingDraftCatchupPhaseV1::Completed {
                key,
                dispatch_generation,
            })
            || batch.requests.as_slice() != [key.request]
            || batch.plan.target() != key.parent
        {
            return Err(M1ServingRegistryErrorV1::DraftCatchupPhaseMismatch);
        }
        self.validate_next_batch(&batch)?;
        self.reserve_validated_batch(batch, None)
    }

    fn reserve_validated_batch(
        &mut self,
        batch: M1ServingBatchPlanV1,
        draft_catchup: Option<M1ServingDraftCatchupKeyV1>,
    ) -> Result<M1ServingPublicationReservationV1, M1ServingRegistryErrorV1> {
        let following_id = self
            .next_reservation_id
            .checked_add(1)
            .ok_or(M1ServingRegistryErrorV1::PublicationReservationExhausted)?;
        let id = self.next_reservation_id;
        self.reservation = Some(M1ServingReservedBatchV1 {
            id,
            plan: batch.plan,
            epoch: batch.epoch,
            action: batch.action,
            draft_catchup,
        });
        self.next_reservation_id = following_id;
        Ok(M1ServingPublicationReservationV1 {
            registry_identity: self.identity,
            id,
            batch,
        })
    }

    /// Atomically replaces every completed record from one bounded window with
    /// one exact next-generation paired-prefill roster and freezes its
    /// publication.
    ///
    /// This transition is deliberately narrower than admission while a queue
    /// is active: every existing registry entry must be `Retired` from a
    /// completed physical generation no newer than the registry completion
    /// frontier. The replacement must retain the same slots in the same lane
    /// order and advance each generation exactly once, and the complete roster
    /// must fit one paired-prefill batch. The removed entries move into the
    /// returned token before any physical detachment.
    /// Restoring that token reinstalls the exact entries, including order,
    /// plans, phases, and quiescence.
    ///
    /// # Errors
    ///
    /// Rejects a live reservation or batch, absent queue binding, any
    /// non-completed predecessor, invalid or duplicate fresh request, a
    /// non-prefill or mismatched roster, exhausted identities/epochs, or host
    /// allocation failure without changing the registry. The proposed roster
    /// is returned intact.
    pub fn reserve_completed_window_replacement(
        &mut self,
        plan: M1ServingPlanV1,
        requests: Box<[RequestId]>,
    ) -> Result<M1ServingNewWindowPublicationReservationV1, M1ServingNewWindowReservationFailureV1>
    {
        let Some(storage) = M1ServingNewWindowEntryStorageV1::try_new(requests.len()) else {
            return Err(M1ServingNewWindowReservationFailureV1 {
                error: M1ServingRegistryErrorV1::HostAllocation,
                plan,
                requests,
            });
        };
        self.reserve_completed_window_replacement_with_storage(plan, requests, storage)
    }

    pub(crate) fn reserve_completed_window_replacement_with_storage(
        &mut self,
        plan: M1ServingPlanV1,
        requests: Box<[RequestId]>,
        storage: M1ServingNewWindowEntryStorageV1,
    ) -> Result<M1ServingNewWindowPublicationReservationV1, M1ServingNewWindowReservationFailureV1>
    {
        let reject = |error, requests| {
            Err(M1ServingNewWindowReservationFailureV1 {
                error,
                plan,
                requests,
            })
        };

        if let Err(error) = self.reject_live_reservation() {
            return reject(error, requests);
        }
        if self.in_flight.is_some() {
            return reject(M1ServingRegistryErrorV1::BatchAlreadyInFlight, requests);
        }
        if self.draft_catchup.is_some() {
            return reject(
                M1ServingRegistryErrorV1::DraftCatchupPhaseMismatch,
                requests,
            );
        }
        let Some(prior) = self.bound_plan else {
            return reject(M1ServingRegistryErrorV1::QueuePlanMismatch, requests);
        };
        if self.entries.is_empty() {
            return reject(
                M1ServingRegistryErrorV1::NewWindowPredecessorRequired,
                requests,
            );
        }
        for (index, entry) in self.entries.iter().enumerate() {
            if let Err(error) =
                validate_completed_window_predecessor(entry, prior, self.completed_epoch, index)
            {
                return reject(error, requests);
            }
        }
        if plan.mode() != Qwen3ExecutionMode::Prefill {
            return reject(M1ServingRegistryErrorV1::AdmissionRequiresPrefill, requests);
        }
        if requests.is_empty() {
            return reject(M1ServingRegistryErrorV1::NewWindowRosterEmpty, requests);
        }
        if requests.len() > C || requests.len() > plan.sequence_capacity() {
            return reject(
                M1ServingRegistryErrorV1::NewWindowRosterExceedsPlan,
                requests,
            );
        }
        if requests.len() != self.entries.len() {
            return reject(
                M1ServingRegistryErrorV1::NewWindowRosterCountMismatch,
                requests,
            );
        }
        for (index, (request, predecessor)) in requests
            .iter()
            .copied()
            .zip(self.entries.iter())
            .enumerate()
        {
            if request.generation() == 0 {
                return reject(M1ServingRegistryErrorV1::InvalidRequest, requests);
            }
            if request.slot() != predecessor.request.slot() {
                return reject(
                    M1ServingRegistryErrorV1::NewWindowRequestSlotMismatch { index },
                    requests,
                );
            }
            let exact_next = predecessor.request.generation().checked_add(1);
            if exact_next != Some(request.generation()) {
                return reject(
                    M1ServingRegistryErrorV1::NewWindowRequestGenerationMismatch { index },
                    requests,
                );
            }
        }
        if self.submitted_epoch != self.completed_epoch {
            return reject(M1ServingRegistryErrorV1::CompletionEpochMismatch, requests);
        }
        let Some(next_epoch) = self.submitted_epoch.checked_add(1) else {
            return reject(M1ServingRegistryErrorV1::CompletionEpochMismatch, requests);
        };
        let Some(following_id) = self.next_reservation_id.checked_add(1) else {
            return reject(
                M1ServingRegistryErrorV1::PublicationReservationExhausted,
                requests,
            );
        };

        let mut next_entries = storage.entries;
        next_entries.clear();
        if next_entries.capacity() < requests.len() {
            return reject(M1ServingRegistryErrorV1::HostAllocation, requests);
        }
        next_entries.extend(requests.iter().copied().map(|request| M1ServingEntryV1 {
            request,
            plan,
            phase: M1ServingRequestPhaseV1::Ready,
            last_quiescence: None,
        }));
        let mut inline_requests = M1ServingInlineRosterV1::new();
        for request in requests.iter().copied() {
            if inline_requests.try_push(request).is_err() {
                return reject(
                    M1ServingRegistryErrorV1::NewWindowRosterExceedsPlan,
                    requests,
                );
            }
        }

        let id = self.next_reservation_id;
        let action = M1ServingQueueActionV1::QuiescentNewWindow { prior, next: plan };
        let publication = M1ServingPublicationReservationV1 {
            registry_identity: self.identity,
            id,
            batch: M1ServingBatchPlanV1 {
                plan,
                requests: inline_requests,
                epoch: CompletionEpoch::new(next_epoch),
                action,
            },
        };
        self.reservation = Some(M1ServingReservedBatchV1 {
            id,
            plan,
            epoch: CompletionEpoch::new(next_epoch),
            action,
            draft_catchup: None,
        });
        self.next_reservation_id = following_id;
        let predecessors = core::mem::replace(&mut self.entries, next_entries);
        Ok(M1ServingNewWindowPublicationReservationV1 {
            prior,
            predecessors,
            requests,
            publication,
        })
    }

    /// Restores an all-terminal predecessor roster before physical detachment.
    ///
    /// # Errors
    ///
    /// A cross-registry, stale, or substituted transaction is returned intact
    /// and the live registry remains unchanged. A successful restoration does
    /// not rewind the consumed reservation identity; a retry receives a fresh
    /// identity so the aborted capability cannot be replayed.
    pub fn restore_completed_window_replacement(
        &mut self,
        reservation: M1ServingNewWindowPublicationReservationV1,
    ) -> Result<M1ServingRestoredNewWindowV1, Box<M1ServingNewWindowPublicationFailureV1>> {
        if let Err(error) = self.validate_new_window_reservation(&reservation) {
            return Err(Box::new(M1ServingNewWindowPublicationFailureV1 {
                error,
                reservation,
            }));
        }
        let M1ServingNewWindowPublicationReservationV1 {
            prior: _,
            predecessors,
            requests,
            publication,
        } = reservation;
        let plan = publication.plan();
        self.reservation = None;
        self.entries = predecessors;
        Ok(M1ServingRestoredNewWindowV1 { plan, requests })
    }

    #[allow(dead_code)] // Consumed by the pending physical new-window bridge.
    pub(crate) fn preflight_new_window_publication(
        &self,
        reservation: &M1ServingNewWindowPublicationReservationV1,
    ) -> Result<(), M1ServingRegistryErrorV1> {
        self.validate_new_window_reservation(reservation)
    }

    #[allow(dead_code)] // Consumed by the pending physical new-window bridge.
    pub(crate) fn record_new_window_publication(
        &mut self,
        reservation: M1ServingNewWindowPublicationReservationV1,
    ) -> Result<(), Box<M1ServingNewWindowPublicationFailureV1>> {
        if let Err(error) = self.validate_new_window_reservation(&reservation) {
            return Err(Box::new(M1ServingNewWindowPublicationFailureV1 {
                error,
                reservation,
            }));
        }
        let M1ServingNewWindowPublicationReservationV1 {
            prior: _,
            predecessors: _,
            requests: _,
            publication,
        } = reservation;
        let batch = publication.batch;
        for entry in &mut self.entries {
            entry.phase = M1ServingRequestPhaseV1::InFlight { epoch: batch.epoch };
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
            return Err(M1ServingPublicationFailureV1 {
                error,
                reservation: Box::new(reservation),
            });
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
            return Err(M1ServingPublicationFailureV1 {
                error,
                reservation: Box::new(reservation),
            });
        }
        let maintenance_key = self
            .reservation
            .as_ref()
            .and_then(|live| live.draft_catchup);
        let batch = reservation.batch;
        match batch.action {
            M1ServingQueueActionV1::QuiescentDraftCatchup { .. } => {
                self.draft_catchup = Some(M1ServingDraftCatchupPhaseV1::InFlight(
                    maintenance_key.expect("validated maintenance reservation retains its key"),
                ));
            }
            M1ServingQueueActionV1::QuiescentDraftCatchupRestore { .. } => {
                self.draft_catchup = None;
            }
            _ => {}
        }
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

    pub(crate) fn complete_draft_catchup(
        &mut self,
        completed: &crate::authenticated_speculative_executor::M1AuthenticatedDraftCatchupCompletedV1,
    ) -> Result<(), M1ServingRegistryErrorV1> {
        self.complete_draft_catchup_key(
            M1ServingDraftCatchupKeyV1::from_pending(completed.pending()),
            completed.dispatch_generation(),
        )
    }

    fn complete_draft_catchup_key(
        &mut self,
        key: M1ServingDraftCatchupKeyV1,
        dispatch_generation: u64,
    ) -> Result<(), M1ServingRegistryErrorV1> {
        let disposition = self.validate_draft_catchup_completion(key, dispatch_generation)?;
        self.apply_validated_completion(key.epoch, &[disposition]);
        self.draft_catchup = Some(M1ServingDraftCatchupPhaseV1::Completed {
            key,
            dispatch_generation,
        });
        Ok(())
    }

    fn validate_draft_catchup_completion(
        &self,
        key: M1ServingDraftCatchupKeyV1,
        dispatch_generation: u64,
    ) -> Result<M1ServingCompletionDispositionV1, M1ServingRegistryErrorV1> {
        self.reject_live_reservation()?;
        let in_flight = self
            .in_flight
            .as_ref()
            .ok_or(M1ServingRegistryErrorV1::NoBatchInFlight)?;
        if self.draft_catchup != Some(M1ServingDraftCatchupPhaseV1::InFlight(key))
            || !key.valid_for(in_flight.plan)
            || key.prior_dispatch_generation.checked_add(1) != Some(dispatch_generation)
            || in_flight.requests.as_slice() != [key.request]
            || in_flight.epoch != key.epoch
            || self.bound_plan != Some(in_flight.plan)
            || self.submitted_epoch != key.epoch.value()
            || self.completed_epoch != key.prior_epoch.value()
        {
            return Err(M1ServingRegistryErrorV1::DraftCatchupPhaseMismatch);
        }
        let entry = self
            .entry(key.request)
            .ok_or(M1ServingRegistryErrorV1::UnknownRequest)?;
        if entry.plan != in_flight.plan || entry.last_quiescence != Some(key.prior_epoch) {
            return Err(M1ServingRegistryErrorV1::DraftCatchupPhaseMismatch);
        }
        match entry.phase {
            M1ServingRequestPhaseV1::InFlight { epoch } if epoch == key.epoch => {
                Ok(M1ServingCompletionDispositionV1::Continue(entry.plan))
            }
            M1ServingRequestPhaseV1::CancellationPending { epoch } if epoch == key.epoch => {
                Ok(M1ServingCompletionDispositionV1::Retire)
            }
            _ => Err(M1ServingRegistryErrorV1::DraftCatchupPhaseMismatch),
        }
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
        if self.draft_catchup.is_some() {
            return Err(M1ServingRegistryErrorV1::DraftCatchupWitnessRequired);
        }
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

    fn next_ready_plan(&self) -> Option<M1ServingPlanV1> {
        if let Some(phase) = self.draft_catchup {
            let key = phase.key();
            if let Some(entry) = self.entry(key.request).filter(|entry| {
                entry.phase == M1ServingRequestPhaseV1::Ready && entry.plan.target() == key.parent
            }) {
                return Some(entry.plan);
            }
        }
        self.entries
            .iter()
            .filter(|entry| entry.phase == M1ServingRequestPhaseV1::Ready)
            .min_by_key(|entry| plan_priority(entry.plan))
            .map(|entry| entry.plan)
    }

    fn ready_work_needs_bound_queue(&self, bound: M1ServingPlanV1) -> bool {
        if let Some(phase) = self.draft_catchup {
            let key = phase.key();
            return key.parent == bound.target()
                && self.entry(key.request).is_some_and(|entry| {
                    entry.phase == M1ServingRequestPhaseV1::Ready && entry.plan == bound
                });
        }
        self.next_ready_plan().is_some_and(|next| {
            next == bound || admit_m1_production_rollover_transition_v1(bound, next).is_some()
        })
    }

    fn classify_next_queue_action(
        &self,
        selected_plan: M1ServingPlanV1,
    ) -> Result<M1ServingQueueActionV1, M1ServingRegistryErrorV1> {
        match self.draft_catchup {
            None => classify_queue_action(self.bound_plan, selected_plan),
            Some(M1ServingDraftCatchupPhaseV1::Pending(key))
                if self.bound_plan == Some(selected_plan)
                    && key.parent == selected_plan.target()
                    && self.entry(key.request).is_some_and(|entry| {
                        entry.phase == M1ServingRequestPhaseV1::Ready && entry.plan == selected_plan
                    }) =>
            {
                Ok(M1ServingQueueActionV1::QuiescentDraftCatchup {
                    parent: selected_plan,
                })
            }
            Some(M1ServingDraftCatchupPhaseV1::Completed { key, .. })
                if self.bound_plan == Some(selected_plan)
                    && key.parent == selected_plan.target()
                    && self.entry(key.request).is_some_and(|entry| {
                        entry.phase == M1ServingRequestPhaseV1::Ready && entry.plan == selected_plan
                    }) =>
            {
                Ok(M1ServingQueueActionV1::QuiescentDraftCatchupRestore {
                    parent: selected_plan,
                })
            }
            Some(_) => Err(M1ServingRegistryErrorV1::DraftCatchupPhaseMismatch),
        }
    }

    fn validate_draft_catchup_start(
        &self,
        batch: &M1ServingBatchPlanV1,
        key: M1ServingDraftCatchupKeyV1,
    ) -> Result<(), M1ServingRegistryErrorV1> {
        if self.draft_catchup != Some(M1ServingDraftCatchupPhaseV1::Pending(key))
            || batch.action
                != (M1ServingQueueActionV1::QuiescentDraftCatchup { parent: batch.plan })
            || batch.requests.as_slice() != [key.request]
            || batch.epoch != key.epoch
        {
            return Err(M1ServingRegistryErrorV1::DraftCatchupPhaseMismatch);
        }
        self.validate_draft_catchup_ready_key(batch.plan, key)
    }

    fn validate_draft_catchup_ready_key(
        &self,
        parent: M1ServingPlanV1,
        key: M1ServingDraftCatchupKeyV1,
    ) -> Result<(), M1ServingRegistryErrorV1> {
        if !key.valid_for(parent)
            || self.bound_plan != Some(parent)
            || self.submitted_epoch != key.prior_epoch.value()
            || self.completed_epoch != key.prior_epoch.value()
            || self.entry(key.request).is_none_or(|entry| {
                entry.phase != M1ServingRequestPhaseV1::Ready
                    || entry.plan != parent
                    || entry.last_quiescence != Some(key.prior_epoch)
            })
        {
            return Err(M1ServingRegistryErrorV1::DraftCatchupPhaseMismatch);
        }
        Ok(())
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

    fn validate_ready_roster(
        &self,
        batch: &M1ServingBatchPlanV1,
    ) -> Result<(), M1ServingRegistryErrorV1> {
        if let Some(phase) = self.draft_catchup {
            let key = phase.key();
            if batch.requests.as_slice() != [key.request]
                || self.entry(key.request).is_none_or(|entry| {
                    entry.phase != M1ServingRequestPhaseV1::Ready || entry.plan != batch.plan
                })
            {
                return Err(M1ServingRegistryErrorV1::PublicationReservationMismatch);
            }
            return Ok(());
        }
        let limit = C.min(batch.plan.sequence_capacity());
        let mut expected_count = 0;
        for entry in self
            .entries
            .iter()
            .filter(|entry| {
                entry.phase == M1ServingRequestPhaseV1::Ready && entry.plan == batch.plan
            })
            .take(limit)
        {
            if batch.requests.get(expected_count) != Some(&entry.request) {
                return Err(M1ServingRegistryErrorV1::PublicationReservationMismatch);
            }
            expected_count += 1;
        }
        if expected_count == 0 {
            return Err(M1ServingRegistryErrorV1::RequestNotReady);
        }
        if batch.requests.len() != expected_count {
            return Err(M1ServingRegistryErrorV1::PublicationReservationMismatch);
        }
        Ok(())
    }

    fn validate_next_batch(
        &self,
        batch: &M1ServingBatchPlanV1,
    ) -> Result<(), M1ServingRegistryErrorV1> {
        let Some(selected_plan) = self.next_ready_plan() else {
            return Err(M1ServingRegistryErrorV1::RequestNotReady);
        };
        let next_epoch = self
            .submitted_epoch
            .checked_add(1)
            .ok_or(M1ServingRegistryErrorV1::CompletionEpochMismatch)?;
        let action = self.classify_next_queue_action(selected_plan)?;
        if batch.plan != selected_plan
            || batch.epoch != CompletionEpoch::new(next_epoch)
            || batch.action != action
        {
            return Err(M1ServingRegistryErrorV1::PublicationReservationMismatch);
        }
        self.validate_ready_roster(batch)
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
        if live.id != reservation.id
            || live.plan != reservation.batch.plan
            || live.epoch != reservation.batch.epoch
            || live.action != reservation.batch.action
        {
            return Err(M1ServingRegistryErrorV1::PublicationReservationMismatch);
        }
        if reservation.batch.epoch.value() != self.submitted_epoch.saturating_add(1) {
            return Err(M1ServingRegistryErrorV1::CompletionEpochMismatch);
        }
        match reservation.batch.action {
            M1ServingQueueActionV1::QuiescentDraftCatchup { parent } => {
                let key = live
                    .draft_catchup
                    .ok_or(M1ServingRegistryErrorV1::DraftCatchupWitnessRequired)?;
                if parent != reservation.batch.plan {
                    return Err(M1ServingRegistryErrorV1::DraftCatchupPhaseMismatch);
                }
                self.validate_draft_catchup_start(&reservation.batch, key)?;
            }
            M1ServingQueueActionV1::QuiescentDraftCatchupRestore { parent } => {
                let Some(M1ServingDraftCatchupPhaseV1::Completed { key, .. }) = self.draft_catchup
                else {
                    return Err(M1ServingRegistryErrorV1::DraftCatchupPhaseMismatch);
                };
                if live.draft_catchup.is_some()
                    || parent != reservation.batch.plan
                    || reservation.batch.requests.as_slice() != [key.request]
                {
                    return Err(M1ServingRegistryErrorV1::DraftCatchupPhaseMismatch);
                }
                self.validate_next_batch(&reservation.batch)?;
            }
            _ if live.draft_catchup.is_some() || self.draft_catchup.is_some() => {
                return Err(M1ServingRegistryErrorV1::DraftCatchupPhaseMismatch);
            }
            _ => {}
        }
        self.validate_ready_roster(&reservation.batch)
    }

    fn validate_new_window_reservation(
        &self,
        reservation: &M1ServingNewWindowPublicationReservationV1,
    ) -> Result<(), M1ServingRegistryErrorV1> {
        self.validate_reservation(&reservation.publication)?;
        let plan = reservation.publication.plan();
        if self.bound_plan != Some(reservation.prior)
            || reservation.publication.action()
                != (M1ServingQueueActionV1::QuiescentNewWindow {
                    prior: reservation.prior,
                    next: plan,
                })
            || reservation.predecessors.is_empty()
            || reservation.predecessors.len() != self.entries.len()
            || self.entries.len() != reservation.publication.requests().len()
            || reservation.requests.as_ref() != reservation.publication.requests()
            || self.entries.is_empty()
            || self.entries.len() > C
            || self.entries.len() > plan.sequence_capacity()
            || plan.mode() != Qwen3ExecutionMode::Prefill
        {
            return Err(M1ServingRegistryErrorV1::PublicationReservationMismatch);
        }
        for (index, entry) in reservation.predecessors.iter().enumerate() {
            validate_completed_window_predecessor(
                entry,
                reservation.prior,
                self.completed_epoch,
                index,
            )?;
        }
        for (lane, (entry, request)) in self
            .entries
            .iter()
            .zip(reservation.publication.requests())
            .enumerate()
        {
            if entry.request != *request
                || entry.plan != plan
                || entry.phase != M1ServingRequestPhaseV1::Ready
                || entry.last_quiescence.is_some()
            {
                return Err(M1ServingRegistryErrorV1::CompletionRosterMismatch { lane });
            }
            if request.generation() == 0 {
                return Err(M1ServingRegistryErrorV1::InvalidRequest);
            }
            let predecessor = &reservation.predecessors[lane];
            if request.slot() != predecessor.request.slot() {
                return Err(M1ServingRegistryErrorV1::NewWindowRequestSlotMismatch { index: lane });
            }
            let exact_next = predecessor.request.generation().checked_add(1);
            if exact_next != Some(request.generation()) {
                return Err(
                    M1ServingRegistryErrorV1::NewWindowRequestGenerationMismatch { index: lane },
                );
            }
        }
        Ok(())
    }
}

const fn authenticated_s1_t128_prefill_selection(role: Qwen3ModelRole) -> Qwen3PlanSelection {
    Qwen3PlanSelection {
        role,
        mode: Qwen3ExecutionMode::Prefill,
        bucket: Qwen3PlanBucket::PrefillS1T128,
    }
}

const fn authenticated_s1_k4_selection(role: Qwen3ModelRole) -> Qwen3PlanSelection {
    match role {
        Qwen3ModelRole::Target8B => Qwen3PlanSelection {
            role,
            mode: Qwen3ExecutionMode::Speculative,
            bucket: Qwen3PlanBucket::SpeculativeS1K4C8192,
        },
        Qwen3ModelRole::Draft06B => Qwen3PlanSelection {
            role,
            mode: Qwen3ExecutionMode::Decode,
            bucket: Qwen3PlanBucket::DecodeS1C8192,
        },
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct M1AuthenticatedPrefillRegistryMemberFactsV1 {
    member_request: RequestId,
    target: ferric_spec::LogicalKvState,
    draft: ferric_spec::LogicalKvState,
    target_active_pages: usize,
    draft_active_pages: usize,
    target_retired_pages: usize,
    draft_retired_pages: usize,
    target_quiescent_retired_pages: usize,
    draft_quiescent_retired_pages: usize,
    target_write_pending: bool,
    draft_write_pending: bool,
    target_qualification_future_pages: usize,
    device_join: M1AuthenticatedPrefillRegistryMemberAssociationV1,
    target_arena_join: M1AuthenticatedPrefillRegistryMemberAssociationV1,
    draft_arena_join: M1AuthenticatedPrefillRegistryMemberAssociationV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum M1AuthenticatedPrefillRegistryMemberAssociationV1 {
    Exact,
    Mismatch,
}

fn validate_authenticated_prefill_registry_member_v1(
    request: RequestId,
    facts: M1AuthenticatedPrefillRegistryMemberFactsV1,
) -> Result<(), M1AuthenticatedPrefillRegistryReconciliationErrorV1> {
    let expected_pages = 128_u32.div_ceil(M1_KV_PAGE_TOKENS) as usize;
    if facts.member_request != request
        || facts.target.request != request
        || facts.draft.request != request
    {
        return Err(M1AuthenticatedPrefillRegistryReconciliationErrorV1::RequestMismatch);
    }
    if facts.target.role != Qwen3ModelRole::Target8B
        || facts.draft.role != Qwen3ModelRole::Draft06B
        || facts.target.lifecycle != PhysicalKvLifecycle::Active
        || facts.draft.lifecycle != PhysicalKvLifecycle::Active
        || facts.target.resident_tokens != 128
        || facts.target.committed_tokens != 128
        || facts.draft.resident_tokens != 128
        || facts.draft.committed_tokens != 128
        || facts.target_active_pages != expected_pages
        || facts.draft_active_pages != expected_pages
        || facts.target_retired_pages != 0
        || facts.draft_retired_pages != 0
        || facts.target_quiescent_retired_pages != 0
        || facts.draft_quiescent_retired_pages != 0
        || facts.target_write_pending
        || facts.draft_write_pending
        || facts.target_qualification_future_pages != 0
        || facts.device_join != M1AuthenticatedPrefillRegistryMemberAssociationV1::Exact
        || facts.target_arena_join != M1AuthenticatedPrefillRegistryMemberAssociationV1::Exact
        || facts.draft_arena_join != M1AuthenticatedPrefillRegistryMemberAssociationV1::Exact
    {
        return Err(M1AuthenticatedPrefillRegistryReconciliationErrorV1::CacheMismatch);
    }
    Ok(())
}

fn validate_authenticated_prefill_registry_reconciliation_v1<const C: usize>(
    registry: &M1ServingRegistryV1<C>,
    completed: &M1AuthenticatedS1T128PrefillExecutionSuccessV1<C>,
) -> Result<
    M1AuthenticatedPrefillRegistryReconciliationV1,
    M1AuthenticatedPrefillRegistryReconciliationErrorV1,
> {
    if !registry.is_fresh_for_authenticated_prefill_reconciliation() {
        return Err(M1AuthenticatedPrefillRegistryReconciliationErrorV1::RegistryNotFresh);
    }
    let prefill = M1ServingPlanV1::new(
        authenticated_s1_t128_prefill_selection(Qwen3ModelRole::Target8B),
        authenticated_s1_t128_prefill_selection(Qwen3ModelRole::Draft06B),
    )
    .map_err(|_| M1AuthenticatedPrefillRegistryReconciliationErrorV1::ProfileMismatch)?;
    let successor = completed
        .singleton_speculative_successor()
        .ok_or(M1AuthenticatedPrefillRegistryReconciliationErrorV1::ProfileMismatch)?;
    let maximum_output_tokens = completed.generation_policy().max_output_tokens();
    let Some(total_context) = 128_u32.checked_add(maximum_output_tokens) else {
        return Err(M1AuthenticatedPrefillRegistryReconciliationErrorV1::ProfileMismatch);
    };
    let expected_target_tail_pages =
        crate::authenticated_prefill_bootstrap::s1_t128_successor_target_tail_pages_v1(
            successor,
            maximum_output_tokens,
        )
        .ok_or(M1AuthenticatedPrefillRegistryReconciliationErrorV1::ProfileMismatch)?;
    if completed.prompt_tokens().len() != 128
        || maximum_output_tokens == 0
        || total_context > 8_192
        || expected_target_tail_pages == 0
        || completed.target_rollover_pages().len() != expected_target_tail_pages
    {
        return Err(M1AuthenticatedPrefillRegistryReconciliationErrorV1::ProfileMismatch);
    }
    let released = completed.released();
    let checked = released.checked();
    let Some(physical_intent) = checked.speculative_rollover_intent() else {
        return Err(M1AuthenticatedPrefillRegistryReconciliationErrorV1::IntentMismatch);
    };
    let Some(intent) = join_m1_authenticated_prefill_registry_intent_v1(
        physical_intent,
        completed.rollover_intent(),
    ) else {
        return Err(M1AuthenticatedPrefillRegistryReconciliationErrorV1::IntentMismatch);
    };
    let request = completed.request();
    let epoch = checked.epoch();
    if intent.request != request {
        return Err(M1AuthenticatedPrefillRegistryReconciliationErrorV1::RequestMismatch);
    }
    if intent.prefill_selection != prefill.target()
        || intent.speculative_selection != successor.target()
    {
        return Err(M1AuthenticatedPrefillRegistryReconciliationErrorV1::ProfileMismatch);
    }
    if epoch.value() != 1
        || intent.prefill_epoch != epoch
        || completed.engine().completed_epoch() != epoch
    {
        return Err(M1AuthenticatedPrefillRegistryReconciliationErrorV1::CompletionMismatch);
    }
    if released.queue().shape() != M1PhysicalFixedBatchShapeV1::PairedPrefill
        || released.queue().custody().selection() != prefill.target()
        || checked.selection() != prefill.target()
    {
        return Err(M1AuthenticatedPrefillRegistryReconciliationErrorV1::QueueMismatch);
    }
    let [record] = checked.records() else {
        return Err(M1AuthenticatedPrefillRegistryReconciliationErrorV1::CompletionMismatch);
    };
    let [member] = released.members() else {
        return Err(M1AuthenticatedPrefillRegistryReconciliationErrorV1::CompletionMismatch);
    };
    let [release_count] = released.release_counts() else {
        return Err(M1AuthenticatedPrefillRegistryReconciliationErrorV1::CompletionMismatch);
    };
    let [first_token] = completed.direct_choices().choices() else {
        return Err(M1AuthenticatedPrefillRegistryReconciliationErrorV1::CompletionMismatch);
    };
    let wire = record.record();
    if released.logical_accepted_counts() != [1]
        || released.externally_published_counts() != [1]
        || released.completed_members() != 1
        || released.total_released() != 0
        || release_count.total() != 0
        || checked.dispatch_generation() == 0
        || record.selection() != prefill.target()
        || wire.request != request
        || wire.epoch != epoch
        || wire.accepted_draft_tokens != 0
        || wire.emitted_token_count != 1
        || wire.emitted_tokens[0] != *first_token
        || completed.first_token() != *first_token
        || record.semantics()
            != (CheckedCompletionSemantics::DirectFinalRow {
                token: *first_token,
            })
    {
        return Err(M1AuthenticatedPrefillRegistryReconciliationErrorV1::CompletionMismatch);
    }
    let M1ReleasedDeviceKvMemberV1::Active(cache) = member else {
        return Err(M1AuthenticatedPrefillRegistryReconciliationErrorV1::CacheMismatch);
    };
    let projection = cache.projection();
    let queue = released.queue().custody();
    validate_authenticated_prefill_registry_member_v1(
        request,
        M1AuthenticatedPrefillRegistryMemberFactsV1 {
            member_request: member.request(),
            target: projection.target,
            draft: projection.draft,
            target_active_pages: projection.target_active_pages,
            draft_active_pages: projection.draft_active_pages,
            target_retired_pages: projection.target_retired_pages,
            draft_retired_pages: projection.draft_retired_pages,
            target_quiescent_retired_pages: projection.target_quiescent_retired_pages,
            draft_quiescent_retired_pages: projection.draft_quiescent_retired_pages,
            target_write_pending: projection.target_write_pending,
            draft_write_pending: projection.draft_write_pending,
            target_qualification_future_pages: projection.target_qualification_future_pages,
            device_join: if projection.device == queue.device() {
                M1AuthenticatedPrefillRegistryMemberAssociationV1::Exact
            } else {
                M1AuthenticatedPrefillRegistryMemberAssociationV1::Mismatch
            },
            target_arena_join: if projection.target_arena_allocation_id
                == Some(
                    queue
                        .partitioned_memory()
                        .allocation_id(Qwen3ModelRole::Target8B),
                ) {
                M1AuthenticatedPrefillRegistryMemberAssociationV1::Exact
            } else {
                M1AuthenticatedPrefillRegistryMemberAssociationV1::Mismatch
            },
            draft_arena_join: if projection.draft_arena_allocation_id
                == Some(
                    queue
                        .partitioned_memory()
                        .allocation_id(Qwen3ModelRole::Draft06B),
                ) {
                M1AuthenticatedPrefillRegistryMemberAssociationV1::Exact
            } else {
                M1AuthenticatedPrefillRegistryMemberAssociationV1::Mismatch
            },
        },
    )?;
    if cache
        .preflight_quiescent_reselection(successor.target(), successor.draft_cache_selection())
        .is_err()
    {
        return Err(M1AuthenticatedPrefillRegistryReconciliationErrorV1::CacheMismatch);
    }
    if completed.engine().is_faulted()
        || completed.engine().live_count() != 1
        || completed.engine().state(request) != Some(RequestState::Ready)
        || completed.engine().resident_tokens(request) != Some(1)
        || completed.engine().committed_tokens(request) != Some(1)
    {
        return Err(M1AuthenticatedPrefillRegistryReconciliationErrorV1::EngineMismatch);
    }
    Ok(M1AuthenticatedPrefillRegistryReconciliationV1 {
        request,
        epoch,
        prefill,
        successor,
    })
}

/// Reconciles a fresh metadata registry from the exact authenticated physical
/// paired-prefill completion that bypassed the ordinary serving publication
/// bridge.
///
/// This function creates no synthetic publication or completion authority. It
/// consumes the real paired-prefill success owner, validates its Engine,
/// checked output, intent, queue, and active device-KV custody as one exact
/// S1/T128 to S1/K4 transition, then installs only the corresponding metadata
/// frontier. Success retains both owners opaquely for the physical rollover.
///
/// # Errors
///
/// Any mismatch retains the consumed registry and physical success owner in
/// opaque failure custody.
pub fn reconcile_m1_authenticated_s1_t128_prefill_registry_v1<const C: usize>(
    registry: M1ServingRegistryV1<C>,
    completed: M1AuthenticatedS1T128PrefillExecutionSuccessV1<C>,
) -> Result<
    M1AuthenticatedPrefillRegistryReconciledV1<C>,
    Box<M1AuthenticatedPrefillRegistryReconciliationFailureV1<C>>,
> {
    let legacy = M1ServingPlanV1::new(
        authenticated_s1_k4_selection(Qwen3ModelRole::Target8B),
        authenticated_s1_k4_selection(Qwen3ModelRole::Draft06B),
    )
    .ok();
    if completed.singleton_speculative_successor().is_none()
        || completed.singleton_speculative_successor() != legacy
    {
        return Err(Box::new(
            M1AuthenticatedPrefillRegistryReconciliationFailureV1 {
                error: M1AuthenticatedPrefillRegistryReconciliationErrorV1::ProfileMismatch,
                _registry: registry,
                _completed: completed,
            },
        ));
    }
    reconcile_m1_authenticated_s1_t128_finite_prefill_registry_v1(registry, completed)
}

pub(crate) fn reconcile_m1_authenticated_s1_t128_finite_prefill_registry_v1<const C: usize>(
    mut registry: M1ServingRegistryV1<C>,
    completed: M1AuthenticatedS1T128PrefillExecutionSuccessV1<C>,
) -> Result<
    M1AuthenticatedPrefillRegistryReconciledV1<C>,
    Box<M1AuthenticatedPrefillRegistryReconciliationFailureV1<C>>,
> {
    let reconciliation =
        match validate_authenticated_prefill_registry_reconciliation_v1(&registry, &completed) {
            Ok(reconciliation) => reconciliation,
            Err(error) => {
                return Err(Box::new(
                    M1AuthenticatedPrefillRegistryReconciliationFailureV1 {
                        error,
                        _registry: registry,
                        _completed: completed,
                    },
                ));
            }
        };
    if let Err(error) = registry.install_authenticated_prefill_reconciliation(reconciliation) {
        return Err(Box::new(
            M1AuthenticatedPrefillRegistryReconciliationFailureV1 {
                error,
                _registry: registry,
                _completed: completed,
            },
        ));
    }
    Ok(M1AuthenticatedPrefillRegistryReconciledV1 {
        registry,
        completed,
        request: reconciliation.request,
        epoch: reconciliation.epoch,
        prefill: reconciliation.prefill,
        successor: reconciliation.successor,
    })
}

fn validate_completed_window_predecessor(
    entry: &M1ServingEntryV1,
    prior: M1ServingPlanV1,
    completed_epoch: u64,
    index: usize,
) -> Result<(), M1ServingRegistryErrorV1> {
    if entry.plan != prior {
        return Err(M1ServingRegistryErrorV1::NewWindowPredecessorPlanMismatch { index });
    }
    let M1ServingRequestPhaseV1::Retired {
        quiescence: M1ServingQuiescenceV1::Completed(epoch),
    } = entry.phase
    else {
        return Err(M1ServingRegistryErrorV1::NewWindowPredecessorNotCompleted { index });
    };
    if epoch.value() == 0 || epoch.value() > completed_epoch || entry.last_quiescence != Some(epoch)
    {
        return Err(M1ServingRegistryErrorV1::NewWindowPredecessorEpochMismatch { index });
    }
    Ok(())
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

/// One exact cross-plan transition implemented by the production physical
/// serving path.
///
/// This crate-private authority is shared by registry planning and the
/// physical adapter. It deliberately excludes identity transitions, which use
/// same-shape rearm, and every transition without a production rollover
/// implementation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct M1ServingProductionRolloverTransitionV1 {
    prior: M1ServingPlanV1,
    next: M1ServingPlanV1,
    reason: M1ServingRolloverReasonV1,
}

impl M1ServingProductionRolloverTransitionV1 {
    pub(crate) const fn prior(self) -> M1ServingPlanV1 {
        self.prior
    }

    pub(crate) const fn next(self) -> M1ServingPlanV1 {
        self.next
    }

    pub(crate) const fn reason(self) -> M1ServingRolloverReasonV1 {
        self.reason
    }
}

/// Admits the exact finite production rollover catalog.
///
/// The admitted cross-plan successors are S1/T128 paired prefill into exact
/// S1/C8192 target decode or S1/K4, S1/K8, or S1/K16 speculation, and S8/T128
/// paired prefill into S8/K4 speculation. Callers must fail closed for `None`.
pub(crate) fn admit_m1_production_rollover_transition_v1(
    prior: M1ServingPlanV1,
    next: M1ServingPlanV1,
) -> Option<M1ServingProductionRolloverTransitionV1> {
    if let Some(transition) = admit_m1_target_decode_rollover_transition_v1(prior, next) {
        return Some(transition);
    }
    let admitted = matches!(
        (prior.target().bucket, next.target().bucket),
        (
            Qwen3PlanBucket::PrefillS1T128,
            Qwen3PlanBucket::SpeculativeS1K4C8192
                | Qwen3PlanBucket::SpeculativeS1K8C8192
                | Qwen3PlanBucket::SpeculativeS1K16C8192
        ) | (
            Qwen3PlanBucket::PrefillS8T128,
            Qwen3PlanBucket::SpeculativeS8K4C8192
        )
    ) && prior.mode() == Qwen3ExecutionMode::Prefill
        && prior.shape() == M1PhysicalFixedBatchShapeV1::PairedPrefill
        && next.mode() == Qwen3ExecutionMode::Speculative;
    admitted.then_some(M1ServingProductionRolloverTransitionV1 {
        prior,
        next,
        reason: M1ServingRolloverReasonV1::Mode,
    })
}

/// Recognizes the exact authenticated target-only production transition.
pub(crate) fn admit_m1_target_decode_rollover_transition_v1(
    prior: M1ServingPlanV1,
    next: M1ServingPlanV1,
) -> Option<M1ServingProductionRolloverTransitionV1> {
    let admitted = prior.mode() == Qwen3ExecutionMode::Prefill
        && prior.shape() == M1PhysicalFixedBatchShapeV1::PairedPrefill
        && prior.target().bucket == Qwen3PlanBucket::PrefillS1T128
        && prior.sequence_capacity() == 1
        && next.mode() == Qwen3ExecutionMode::Decode
        && next.shape() == M1PhysicalFixedBatchShapeV1::TargetOnly
        && next.target().bucket == Qwen3PlanBucket::DecodeS1C8192
        && next.draft().bucket == Qwen3PlanBucket::DecodeS1C8192
        && next.sequence_capacity() == 1;
    admitted.then_some(M1ServingProductionRolloverTransitionV1 {
        prior,
        next,
        reason: M1ServingRolloverReasonV1::Mode,
    })
}

fn classify_queue_action(
    prior: Option<M1ServingPlanV1>,
    next: M1ServingPlanV1,
) -> Result<M1ServingQueueActionV1, M1ServingRegistryErrorV1> {
    let Some(prior) = prior else {
        return Ok(M1ServingQueueActionV1::FreshLaunch);
    };
    if prior == next {
        return Ok(M1ServingQueueActionV1::SameShapeRearm);
    }
    let transition = admit_m1_production_rollover_transition_v1(prior, next)
        .ok_or(M1ServingRegistryErrorV1::QueueTransitionUnsupported)?;
    Ok(M1ServingQueueActionV1::QuiescentRollover {
        prior: transition.prior(),
        next: transition.next(),
        reason: transition.reason(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn complete_exact<const C: usize>(
        registry: &mut M1ServingRegistryV1<C>,
        epoch: CompletionEpoch,
        dispositions: &[M1ServingCompletionDispositionV1],
    ) -> Result<(), M1ServingRegistryErrorV1> {
        registry.validate_completion_exact(epoch, dispositions)?;
        registry.apply_validated_completion(epoch, dispositions);
        Ok(())
    }

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

    fn speculative_s1_k16() -> M1ServingPlanV1 {
        pair(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K16C8192,
        )
    }

    fn speculative_s8() -> M1ServingPlanV1 {
        pair(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS8K4C8192,
        )
    }

    fn all_plans() -> [M1ServingPlanV1; 11] {
        [
            prefill_s1(),
            prefill_s8(),
            prefill_s1_t512(),
            pair(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T2048),
            decode_s1(),
            decode_s8(),
            pair(Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS32C8192),
            speculative_s1(),
            speculative_s8(),
            speculative_s1_k8(),
            speculative_s1_k16(),
        ]
    }

    fn expected_production_rollover(prior: M1ServingPlanV1, next: M1ServingPlanV1) -> bool {
        expected_target_decode_rollover(prior, next)
            || matches!(
                (prior.target().bucket, next.target().bucket),
                (
                    Qwen3PlanBucket::PrefillS1T128,
                    Qwen3PlanBucket::SpeculativeS1K4C8192
                        | Qwen3PlanBucket::SpeculativeS1K8C8192
                        | Qwen3PlanBucket::SpeculativeS1K16C8192
                ) | (
                    Qwen3PlanBucket::PrefillS8T128,
                    Qwen3PlanBucket::SpeculativeS8K4C8192
                )
            )
    }

    fn expected_target_decode_rollover(prior: M1ServingPlanV1, next: M1ServingPlanV1) -> bool {
        prior == prefill_s1() && next == decode_s1()
    }

    fn publish_and_complete<const C: usize>(
        registry: &mut M1ServingRegistryV1<C>,
        dispositions: &[M1ServingCompletionDispositionV1],
    ) -> M1ServingQueueActionV1 {
        let batch = registry.plan_next().unwrap().unwrap();
        let epoch = batch.epoch();
        let action = batch.action();
        reserve_and_record(registry, batch);
        complete_exact(registry, epoch, dispositions).unwrap();
        action
    }

    fn reserve_and_record<const C: usize>(
        registry: &mut M1ServingRegistryV1<C>,
        batch: M1ServingBatchPlanV1,
    ) {
        let reservation = registry.reserve_publication(batch).unwrap();
        registry.record_publication(reservation).unwrap();
    }

    #[derive(Debug, Eq, PartialEq)]
    struct CatchupRegistrySnapshot {
        entries: Vec<M1ServingEntryV1>,
        bound: Option<M1ServingPlanV1>,
        reservation: Option<(
            u64,
            M1ServingPlanV1,
            CompletionEpoch,
            M1ServingQueueActionV1,
            Option<M1ServingDraftCatchupKeyV1>,
        )>,
        in_flight: Option<(M1ServingPlanV1, Vec<RequestId>, CompletionEpoch)>,
        phase: Option<M1ServingDraftCatchupPhaseV1>,
        next_reservation: u64,
        submitted: u64,
        completed: u64,
    }

    fn catchup_snapshot(registry: &M1ServingRegistryV1<2>) -> CatchupRegistrySnapshot {
        CatchupRegistrySnapshot {
            entries: registry.entries.clone(),
            bound: registry.bound_plan,
            reservation: registry.reservation.as_ref().map(|live| {
                (
                    live.id,
                    live.plan,
                    live.epoch,
                    live.action,
                    live.draft_catchup,
                )
            }),
            in_flight: registry
                .in_flight
                .as_ref()
                .map(|live| (live.plan, live.requests.to_vec(), live.epoch)),
            phase: registry.draft_catchup,
            next_reservation: registry.next_reservation_id,
            submitted: registry.submitted_epoch,
            completed: registry.completed_epoch,
        }
    }

    // These fixtures exercise registry metadata only; they do not mint physical witnesses.
    fn catchup_registry(
        parent: M1ServingPlanV1,
    ) -> (M1ServingRegistryV1<2>, M1ServingDraftCatchupKeyV1) {
        use crate::speculative_generation_loop::{
            M1SpeculativeGenerationLoopV1, M1SpeculativeGenerationPolicyV1,
            M1SpeculativeMemberSeedV1,
        };
        let request = RequestId::new(0, 1);
        let mut registry = M1ServingRegistryV1::<2>::new().unwrap();
        registry.admit(request, prefill_s1()).unwrap();
        let dispositions = [M1ServingCompletionDispositionV1::Continue(parent)];
        publish_and_complete(&mut registry, &dispositions);
        publish_and_complete(&mut registry, &dispositions);
        let coordinator = M1SpeculativeGenerationLoopV1::new(
            parent.target(),
            &[M1SpeculativeMemberSeedV1::new(
                request,
                70,
                10,
                10,
                M1SpeculativeGenerationPolicyV1::new(64, &[999]).unwrap(),
            )],
        )
        .unwrap();
        let k = match parent.target().bucket {
            Qwen3PlanBucket::SpeculativeS1K4C8192 => 4,
            Qwen3PlanBucket::SpeculativeS1K8C8192 => 8,
            Qwen3PlanBucket::SpeculativeS1K16C8192 => 16,
            _ => panic!("fixture requires a canonical singleton speculative parent"),
        };
        let key = M1ServingDraftCatchupKeyV1 {
            coordinator: coordinator.identity(),
            parent: parent.target(),
            request,
            completed_round: 0,
            prior_epoch: CompletionEpoch::new(2),
            epoch: CompletionEpoch::new(3),
            draft_committed: 10 + k,
            target_committed: 11 + k,
            token: 71,
            prior_dispatch_generation: 41,
        };
        (registry, key)
    }

    fn publish_catchup(registry: &mut M1ServingRegistryV1<2>, key: M1ServingDraftCatchupKeyV1) {
        registry.register_draft_catchup_key(key).unwrap();
        let batch = registry.plan_next().unwrap().unwrap();
        let reservation = registry.reserve_draft_catchup_key(batch, key).unwrap();
        registry.record_publication(reservation).unwrap();
    }

    #[test]
    fn draft_catchup_k4_k8_k16_preserves_obligation_across_both_publication_aborts() {
        for parent in [speculative_s1(), speculative_s1_k8(), speculative_s1_k16()] {
            let (mut registry, key) = catchup_registry(parent);
            registry.register_draft_catchup_key(key).unwrap();
            assert_eq!(registry.bound_shape(), Some(parent.shape()));
            let batch = registry.plan_next().unwrap().unwrap();
            assert_eq!(batch.requests(), &[key.request]);
            assert_eq!(batch.epoch(), key.epoch);
            assert_eq!(
                batch.action(),
                M1ServingQueueActionV1::QuiescentDraftCatchup { parent }
            );
            let before = catchup_snapshot(&registry);
            assert_eq!(
                registry.reserve_publication(batch.duplicate()),
                Err(M1ServingRegistryErrorV1::DraftCatchupWitnessRequired)
            );
            assert_eq!(catchup_snapshot(&registry), before);
            let reserved = registry.reserve_draft_catchup_key(batch, key).unwrap();
            let first_id = reserved.id;
            registry.abort_publication(reserved).unwrap();
            let mut after_abort = catchup_snapshot(&registry);
            assert_eq!(after_abort.next_reservation, before.next_reservation + 1);
            after_abort.next_reservation = before.next_reservation;
            assert_eq!(after_abort, before);
            let retry = registry.plan_next().unwrap().unwrap();
            let reserved = registry.reserve_draft_catchup_key(retry, key).unwrap();
            assert!(reserved.id > first_id);
            registry.record_publication(reserved).unwrap();
            assert_eq!(registry.bound_plan(), Some(parent));
            assert_eq!(
                registry.bound_shape(),
                Some(M1PhysicalFixedBatchShapeV1::DraftCatchup)
            );
            let before = catchup_snapshot(&registry);
            assert_eq!(
                complete_exact(
                    &mut registry,
                    key.epoch,
                    &[M1ServingCompletionDispositionV1::Continue(parent)]
                ),
                Err(M1ServingRegistryErrorV1::DraftCatchupWitnessRequired)
            );
            assert_eq!(catchup_snapshot(&registry), before);
            registry.complete_draft_catchup_key(key, 42).unwrap();
            assert_eq!(
                registry.phase(key.request),
                Some(M1ServingRequestPhaseV1::Ready)
            );
            assert_eq!((registry.submitted_epoch, registry.completed_epoch), (3, 3));
            assert_eq!(registry.bound_plan(), Some(parent));
            assert_eq!(
                registry.bound_shape(),
                Some(M1PhysicalFixedBatchShapeV1::DraftCatchup)
            );
            let before = catchup_snapshot(&registry);
            assert_eq!(
                registry.complete_draft_catchup_key(key, 42),
                Err(M1ServingRegistryErrorV1::NoBatchInFlight)
            );
            assert_eq!(catchup_snapshot(&registry), before);
            let restore = registry.plan_next().unwrap().unwrap();
            assert_eq!(restore.epoch(), CompletionEpoch::new(4));
            assert_eq!(
                restore.action(),
                M1ServingQueueActionV1::QuiescentDraftCatchupRestore { parent }
            );
            assert_eq!(
                registry.reserve_publication(restore.duplicate()),
                Err(M1ServingRegistryErrorV1::DraftCatchupWitnessRequired)
            );
            assert_eq!(catchup_snapshot(&registry), before);
            let reserved = registry
                .reserve_draft_catchup_restore_key(restore, key, 42)
                .unwrap();
            registry.abort_publication(reserved).unwrap();
            let mut after_abort = catchup_snapshot(&registry);
            assert_eq!(after_abort.next_reservation, before.next_reservation + 1);
            after_abort.next_reservation = before.next_reservation;
            assert_eq!(after_abort, before);
            let restore = registry.plan_next().unwrap().unwrap();
            let reserved = registry
                .reserve_draft_catchup_restore_key(restore, key, 42)
                .unwrap();
            registry.record_publication(reserved).unwrap();
            assert_eq!(registry.draft_catchup, None);
            assert_eq!(registry.bound_shape(), Some(parent.shape()));
            complete_exact(
                &mut registry,
                CompletionEpoch::new(4),
                &[M1ServingCompletionDispositionV1::Continue(parent)],
            )
            .unwrap();
            assert_eq!(
                registry.plan_next().unwrap().unwrap().action(),
                M1ServingQueueActionV1::SameShapeRearm
            );
        }
    }

    #[test]
    fn draft_catchup_rejects_invalid_registration_without_mutation() {
        let parent = speculative_s1_k16();
        let (mut registry, key) = catchup_registry(parent);
        let invalid = [
            M1ServingDraftCatchupKeyV1 {
                request: RequestId::new(0, 2),
                ..key
            },
            M1ServingDraftCatchupKeyV1 {
                request: RequestId::new(0, 0),
                ..key
            },
            M1ServingDraftCatchupKeyV1 {
                parent: speculative_s1().target(),
                ..key
            },
            M1ServingDraftCatchupKeyV1 {
                completed_round: u64::MAX,
                ..key
            },
            M1ServingDraftCatchupKeyV1 {
                prior_epoch: CompletionEpoch::new(0),
                ..key
            },
            M1ServingDraftCatchupKeyV1 {
                prior_epoch: CompletionEpoch::new(1),
                epoch: CompletionEpoch::new(2),
                ..key
            },
            M1ServingDraftCatchupKeyV1 {
                epoch: CompletionEpoch::new(4),
                ..key
            },
            M1ServingDraftCatchupKeyV1 {
                prior_dispatch_generation: 0,
                ..key
            },
            M1ServingDraftCatchupKeyV1 {
                prior_dispatch_generation: u64::MAX,
                ..key
            },
            M1ServingDraftCatchupKeyV1 {
                draft_committed: key.target_committed,
                ..key
            },
            M1ServingDraftCatchupKeyV1 {
                draft_committed: u32::MAX,
                ..key
            },
            M1ServingDraftCatchupKeyV1 {
                draft_committed: 8192,
                target_committed: 8193,
                ..key
            },
            M1ServingDraftCatchupKeyV1 {
                token: ferric_spec::QWEN3_VOCABULARY_SIZE,
                ..key
            },
        ];
        let before = catchup_snapshot(&registry);
        for candidate in invalid {
            assert_eq!(
                registry.register_draft_catchup_key(candidate),
                Err(M1ServingRegistryErrorV1::DraftCatchupPhaseMismatch),
                "{candidate:?}"
            );
            assert_eq!(catchup_snapshot(&registry), before);
        }
        registry.register_draft_catchup_key(key).unwrap();
        let before = catchup_snapshot(&registry);
        assert_eq!(
            registry.register_draft_catchup_key(key),
            Err(M1ServingRegistryErrorV1::DraftCatchupPhaseMismatch)
        );
        assert_eq!(catchup_snapshot(&registry), before);
        assert_eq!(
            registry.transition(key.request, decode_s1()),
            Err(M1ServingRegistryErrorV1::DraftCatchupPhaseMismatch)
        );
        assert_eq!(catchup_snapshot(&registry), before);
    }

    #[test]
    fn draft_catchup_rejects_substituted_keys_and_dispatch_generations() {
        let parent = speculative_s1_k8();
        let (mut registry, key) = catchup_registry(parent);
        let (_, other_key) = catchup_registry(parent);
        assert_ne!(key.coordinator, other_key.coordinator);
        let substitutions = [
            M1ServingDraftCatchupKeyV1 {
                coordinator: other_key.coordinator,
                ..key
            },
            M1ServingDraftCatchupKeyV1 {
                request: RequestId::new(0, 2),
                ..key
            },
            M1ServingDraftCatchupKeyV1 {
                parent: speculative_s1().target(),
                ..key
            },
            M1ServingDraftCatchupKeyV1 {
                completed_round: 1,
                ..key
            },
            M1ServingDraftCatchupKeyV1 {
                prior_epoch: CompletionEpoch::new(1),
                ..key
            },
            M1ServingDraftCatchupKeyV1 {
                epoch: CompletionEpoch::new(4),
                ..key
            },
            M1ServingDraftCatchupKeyV1 {
                prior_dispatch_generation: 42,
                ..key
            },
            M1ServingDraftCatchupKeyV1 {
                draft_committed: key.draft_committed + 1,
                target_committed: key.target_committed + 1,
                ..key
            },
            M1ServingDraftCatchupKeyV1 {
                token: key.token + 1,
                ..key
            },
        ];
        registry.register_draft_catchup_key(key).unwrap();
        let before = catchup_snapshot(&registry);
        for candidate in substitutions {
            let batch = registry.plan_next().unwrap().unwrap();
            assert_eq!(
                registry.reserve_draft_catchup_key(batch, candidate),
                Err(M1ServingRegistryErrorV1::DraftCatchupPhaseMismatch)
            );
            assert_eq!(catchup_snapshot(&registry), before);
        }
        let batch = registry.plan_next().unwrap().unwrap();
        let reserved = registry.reserve_draft_catchup_key(batch, key).unwrap();
        registry.record_publication(reserved).unwrap();
        let before = catchup_snapshot(&registry);
        for candidate in substitutions {
            assert_eq!(
                registry.complete_draft_catchup_key(candidate, 42),
                Err(M1ServingRegistryErrorV1::DraftCatchupPhaseMismatch)
            );
            assert_eq!(catchup_snapshot(&registry), before);
        }
        for generation in [0, 41, 43, u64::MAX] {
            assert_eq!(
                registry.complete_draft_catchup_key(key, generation),
                Err(M1ServingRegistryErrorV1::DraftCatchupPhaseMismatch)
            );
            assert_eq!(catchup_snapshot(&registry), before);
        }
        registry.complete_draft_catchup_key(key, 42).unwrap();
        let before = catchup_snapshot(&registry);
        for candidate in substitutions {
            let restore = registry.plan_next().unwrap().unwrap();
            assert_eq!(
                registry.reserve_draft_catchup_restore_key(restore, candidate, 42),
                Err(M1ServingRegistryErrorV1::DraftCatchupPhaseMismatch)
            );
            assert_eq!(catchup_snapshot(&registry), before);
        }
        for generation in [0, 41, 43, u64::MAX] {
            let restore = registry.plan_next().unwrap().unwrap();
            assert_eq!(
                registry.reserve_draft_catchup_restore_key(restore, key, generation),
                Err(M1ServingRegistryErrorV1::DraftCatchupPhaseMismatch)
            );
            assert_eq!(catchup_snapshot(&registry), before);
        }
    }

    #[test]
    fn draft_catchup_prioritizes_exact_request_over_older_ready_member() {
        let parent = speculative_s1();
        let (mut registry, key) = catchup_registry(parent);
        let other = M1ServingEntryV1 {
            request: RequestId::new(1, 1),
            plan: parent,
            phase: M1ServingRequestPhaseV1::Ready,
            last_quiescence: Some(key.prior_epoch),
        };
        registry.entries.insert(0, other);
        registry.register_draft_catchup_key(key).unwrap();
        let batch = registry.plan_next().unwrap().unwrap();
        assert_eq!(batch.requests(), &[key.request]);
        let mut substituted = batch.duplicate();
        substituted.requests = M1ServingInlineRosterV1::new();
        substituted.requests.try_push(other.request).unwrap();
        let before = catchup_snapshot(&registry);
        assert_eq!(
            registry.reserve_draft_catchup_key(substituted, key),
            Err(M1ServingRegistryErrorV1::PublicationReservationMismatch)
        );
        assert_eq!(catchup_snapshot(&registry), before);
        let reserved = registry.reserve_draft_catchup_key(batch, key).unwrap();
        registry.record_publication(reserved).unwrap();
        registry.complete_draft_catchup_key(key, 42).unwrap();
        assert_eq!(registry.entries[0], other);
        let restore = registry.plan_next().unwrap().unwrap();
        assert_eq!(restore.requests(), &[key.request]);
        assert_eq!(
            restore.action(),
            M1ServingQueueActionV1::QuiescentDraftCatchupRestore { parent }
        );
    }

    #[test]
    fn draft_catchup_cancelled_quiescent_member_requires_queue_retirement_before_other_work() {
        for completed in [false, true] {
            let parent = speculative_s1_k16();
            let (mut registry, key) = catchup_registry(parent);
            if completed {
                publish_catchup(&mut registry, key);
                registry.complete_draft_catchup_key(key, 42).unwrap();
            } else {
                registry.register_draft_catchup_key(key).unwrap();
            }
            let other = RequestId::new(1, 1);
            registry.admit(other, prefill_s1()).unwrap();
            let quiescence = if completed {
                key.epoch
            } else {
                key.prior_epoch
            };
            assert_eq!(
                registry.cancel(key.request).unwrap(),
                M1ServingRequestPhaseV1::Retired {
                    quiescence: M1ServingQuiescenceV1::Completed(quiescence),
                }
            );
            assert_eq!(
                registry.plan_next(),
                Err(M1ServingRegistryErrorV1::DraftCatchupPhaseMismatch)
            );
            assert_eq!(
                registry.quiescent_queue_action().unwrap(),
                M1ServingQuiescentQueueActionV1::Retire { bound: parent }
            );
            assert_eq!(
                registry.bound_shape(),
                Some(if completed {
                    M1PhysicalFixedBatchShapeV1::DraftCatchup
                } else {
                    parent.shape()
                })
            );
            registry.record_quiescent_queue_retirement(parent).unwrap();
            assert_eq!(registry.draft_catchup, None);
            assert_eq!(registry.bound_shape(), None);
            let next = registry.plan_next().unwrap().unwrap();
            assert_eq!(next.requests(), &[other]);
            assert_eq!(next.action(), M1ServingQueueActionV1::FreshLaunch);
            assert_eq!(next.epoch().value(), quiescence.value() + 1);
        }
    }

    #[test]
    fn draft_catchup_in_flight_cancellation_requires_checked_maintenance_completion() {
        let parent = speculative_s1_k8();
        let (mut registry, key) = catchup_registry(parent);
        publish_catchup(&mut registry, key);
        assert_eq!(
            registry.cancel(key.request).unwrap(),
            M1ServingRequestPhaseV1::CancellationPending { epoch: key.epoch }
        );
        let before = catchup_snapshot(&registry);
        assert_eq!(
            registry.record_quiescent_queue_retirement(parent),
            Err(M1ServingRegistryErrorV1::BatchAlreadyInFlight)
        );
        assert_eq!(
            complete_exact(
                &mut registry,
                key.epoch,
                &[M1ServingCompletionDispositionV1::Retire]
            ),
            Err(M1ServingRegistryErrorV1::DraftCatchupWitnessRequired)
        );
        assert_eq!(catchup_snapshot(&registry), before);
        registry.complete_draft_catchup_key(key, 42).unwrap();
        assert_eq!(
            registry.phase(key.request),
            Some(M1ServingRequestPhaseV1::Retired {
                quiescence: M1ServingQuiescenceV1::Completed(key.epoch),
            })
        );
        assert!(registry.plan_next().unwrap().is_none());
        assert_eq!(
            registry.bound_shape(),
            Some(M1PhysicalFixedBatchShapeV1::DraftCatchup)
        );
        registry.record_quiescent_queue_retirement(parent).unwrap();
        assert_eq!(registry.draft_catchup, None);
    }

    #[test]
    fn draft_catchup_rejected_publication_retains_the_exact_reservation() {
        let parent = speculative_s1();
        let (mut registry, key) = catchup_registry(parent);
        let (other, _) = catchup_registry(parent);
        registry.register_draft_catchup_key(key).unwrap();
        let batch = registry.plan_next().unwrap().unwrap();
        let mut reserved = registry.reserve_draft_catchup_key(batch, key).unwrap();
        let before = catchup_snapshot(&registry);
        reserved.registry_identity = other.identity;
        let failure = registry.record_publication(reserved).unwrap_err();
        assert_eq!(
            failure.error(),
            M1ServingRegistryErrorV1::RegistryIdentityMismatch
        );
        assert_eq!(catchup_snapshot(&registry), before);
        let mut reserved = failure.into_reservation();
        reserved.registry_identity = registry.identity;
        reserved.batch.action = M1ServingQueueActionV1::SameShapeRearm;
        let failure = registry.abort_publication(reserved).unwrap_err();
        assert_eq!(
            failure.error(),
            M1ServingRegistryErrorV1::PublicationReservationMismatch
        );
        assert_eq!(catchup_snapshot(&registry), before);
        let mut reserved = failure.into_reservation();
        reserved.batch.action = M1ServingQueueActionV1::QuiescentDraftCatchup { parent };
        registry.abort_publication(reserved).unwrap();
        assert_eq!(
            registry.draft_catchup,
            Some(M1ServingDraftCatchupPhaseV1::Pending(key))
        );
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
            assert_eq!(
                plan.draft_cache_selection(),
                selection(
                    Qwen3ModelRole::Draft06B,
                    Qwen3ExecutionMode::Speculative,
                    target_bucket,
                )
            );
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
    fn finite_production_rollover_catalog_and_planner_agree_for_every_plan_pair() {
        let plans = all_plans();
        let mut admitted = 0;
        for prior in plans {
            for next in plans {
                let expected = expected_production_rollover(prior, next);
                let target_expected = expected_target_decode_rollover(prior, next);
                let transition = admit_m1_production_rollover_transition_v1(prior, next);
                assert_eq!(transition.is_some(), expected, "{prior:?} -> {next:?}");
                if let Some(transition) = transition {
                    admitted += 1;
                    assert_eq!(transition.prior(), prior);
                    assert_eq!(transition.next(), next);
                    assert_eq!(transition.reason(), M1ServingRolloverReasonV1::Mode);
                }
                let target_transition = admit_m1_target_decode_rollover_transition_v1(prior, next);
                assert_eq!(
                    target_transition.is_some(),
                    target_expected,
                    "{prior:?} -> {next:?}"
                );
                if let Some(transition) = target_transition {
                    assert_eq!(transition.prior(), prior);
                    assert_eq!(transition.next(), next);
                    assert_eq!(transition.reason(), M1ServingRolloverReasonV1::Mode);
                }

                let classified = classify_queue_action(Some(prior), next);
                if prior == next {
                    assert_eq!(classified, Ok(M1ServingQueueActionV1::SameShapeRearm));
                } else if expected || target_expected {
                    assert_eq!(
                        classified,
                        Ok(M1ServingQueueActionV1::QuiescentRollover {
                            prior,
                            next,
                            reason: M1ServingRolloverReasonV1::Mode,
                        })
                    );
                } else {
                    assert_eq!(
                        classified,
                        Err(M1ServingRegistryErrorV1::QueueTransitionUnsupported)
                    );
                }
            }
            assert_eq!(
                classify_queue_action(None, prior),
                Ok(M1ServingQueueActionV1::FreshLaunch)
            );
        }
        assert_eq!(admitted, 5);
    }

    #[test]
    fn registry_plans_each_exact_prompt_compatible_rollover() {
        let cases = [
            (prefill_s1(), decode_s1()),
            (prefill_s1(), speculative_s1()),
            (prefill_s1(), speculative_s1_k8()),
            (prefill_s1(), speculative_s1_k16()),
            (prefill_s8(), speculative_s8()),
        ];
        for (lane, (prior, next)) in cases.into_iter().enumerate() {
            let request = RequestId::new(u32::try_from(lane).unwrap(), 1);
            let mut registry = M1ServingRegistryV1::<8>::new().unwrap();
            registry.admit(request, prior).unwrap();
            assert_eq!(
                publish_and_complete(
                    &mut registry,
                    &[M1ServingCompletionDispositionV1::Continue(next)],
                ),
                M1ServingQueueActionV1::FreshLaunch
            );
            assert_eq!(
                registry.quiescent_queue_action().unwrap(),
                M1ServingQuiescentQueueActionV1::RetainForReadyWork { bound: prior }
            );
            assert_eq!(
                registry.record_quiescent_queue_retirement(prior),
                Err(M1ServingRegistryErrorV1::ReadyWorkRequiresQueue)
            );
            let rollover = registry.plan_next().unwrap().unwrap();
            assert_eq!(rollover.plan(), next);
            assert_eq!(rollover.requests(), &[request]);
            assert_eq!(
                rollover.action(),
                M1ServingQueueActionV1::QuiescentRollover {
                    prior,
                    next,
                    reason: M1ServingRolloverReasonV1::Mode,
                }
            );
        }
    }

    #[test]
    fn unsupported_native_rollover_retires_before_fresh_successor_launch() {
        let request = RequestId::new(0, 1);
        let prior = prefill_s1();
        let successor = decode_s8();
        let mut registry = M1ServingRegistryV1::<1>::new().unwrap();
        registry.admit(request, prior).unwrap();
        let batch = registry.plan_next().unwrap().unwrap();
        let epoch = batch.epoch();
        reserve_and_record(&mut registry, batch);

        complete_exact(
            &mut registry,
            epoch,
            &[M1ServingCompletionDispositionV1::Continue(successor)],
        )
        .unwrap();
        assert_eq!(registry.plan(request), Some(successor));
        assert_eq!(registry.bound_plan(), Some(prior));
        assert_eq!(
            registry.phase(request),
            Some(M1ServingRequestPhaseV1::Ready)
        );
        assert_eq!(
            registry.plan_next(),
            Err(M1ServingRegistryErrorV1::QueueTransitionUnsupported)
        );
        assert_eq!(
            registry.quiescent_queue_action().unwrap(),
            M1ServingQuiescentQueueActionV1::Retire { bound: prior }
        );
        registry.record_quiescent_queue_retirement(prior).unwrap();
        let fresh = registry.plan_next().unwrap().unwrap();
        assert_eq!(fresh.plan(), successor);
        assert_eq!(fresh.action(), M1ServingQueueActionV1::FreshLaunch);
    }

    #[test]
    fn new_prefill_behind_a_speculative_queue_fails_closed_without_reordering() {
        let active = RequestId::new(0, 1);
        let arrival = RequestId::new(1, 1);
        let speculative = speculative_s8();
        let mut registry = M1ServingRegistryV1::<8>::new().unwrap();
        registry.admit(active, prefill_s8()).unwrap();
        publish_and_complete(
            &mut registry,
            &[M1ServingCompletionDispositionV1::Continue(speculative)],
        );
        publish_and_complete(
            &mut registry,
            &[M1ServingCompletionDispositionV1::Continue(speculative)],
        );
        registry.admit(arrival, prefill_s1()).unwrap();

        assert_eq!(
            registry.plan_next(),
            Err(M1ServingRegistryErrorV1::QueueTransitionUnsupported)
        );
        assert_eq!(registry.plan(active), Some(speculative));
        assert_eq!(registry.plan(arrival), Some(prefill_s1()));
        assert_eq!(registry.phase(active), Some(M1ServingRequestPhaseV1::Ready));
        assert_eq!(
            registry.phase(arrival),
            Some(M1ServingRequestPhaseV1::Ready)
        );
        assert_eq!(registry.bound_plan(), Some(speculative));
        assert!(!registry.has_in_flight_batch());
        assert!(!registry.has_publication_reservation());
        assert_eq!(
            registry.quiescent_queue_action().unwrap(),
            M1ServingQuiescentQueueActionV1::Retire { bound: speculative }
        );
        registry
            .record_quiescent_queue_retirement(speculative)
            .unwrap();
        let fresh = registry.plan_next().unwrap().unwrap();
        assert_eq!(fresh.plan(), prefill_s1());
        assert_eq!(fresh.requests(), &[arrival]);
        assert_eq!(fresh.action(), M1ServingQueueActionV1::FreshLaunch);
    }

    #[test]
    fn unchanged_speculative_plan_uses_same_shape_rearm() {
        let request = RequestId::new(0, 1);
        let mut registry = M1ServingRegistryV1::<1>::new().unwrap();
        registry.admit(request, prefill_s1()).unwrap();
        publish_and_complete(
            &mut registry,
            &[M1ServingCompletionDispositionV1::Continue(speculative_s1())],
        );
        publish_and_complete(
            &mut registry,
            &[M1ServingCompletionDispositionV1::Continue(speculative_s1())],
        );
        assert_eq!(
            registry.plan_next().unwrap().unwrap().action(),
            M1ServingQueueActionV1::SameShapeRearm
        );
    }

    #[test]
    fn unsupported_quiescent_sequence_bucket_change_retires_then_launches() {
        let request = RequestId::new(0, 1);
        let mut registry = M1ServingRegistryV1::<8>::new().unwrap();
        registry.admit(request, prefill_s1()).unwrap();
        publish_and_complete(
            &mut registry,
            &[M1ServingCompletionDispositionV1::Continue(speculative_s1())],
        );
        publish_and_complete(
            &mut registry,
            &[M1ServingCompletionDispositionV1::Continue(speculative_s1())],
        );

        registry.transition(request, speculative_s1_k8()).unwrap();
        assert_eq!(registry.plan(request), Some(speculative_s1_k8()));
        assert_eq!(registry.bound_plan(), Some(speculative_s1()));
        assert_eq!(
            registry.plan_next(),
            Err(M1ServingRegistryErrorV1::QueueTransitionUnsupported)
        );
        assert_eq!(
            registry.quiescent_queue_action().unwrap(),
            M1ServingQuiescentQueueActionV1::Retire {
                bound: speculative_s1()
            }
        );
        registry
            .record_quiescent_queue_retirement(speculative_s1())
            .unwrap();
        let fresh = registry.plan_next().unwrap().unwrap();
        assert_eq!(fresh.plan(), speculative_s1_k8());
        assert_eq!(fresh.action(), M1ServingQueueActionV1::FreshLaunch);
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
        complete_exact(
            &mut registry,
            epoch,
            &[M1ServingCompletionDispositionV1::Retire],
        )
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
            complete_exact(&mut registry, CompletionEpoch::new(epoch.value() + 1), &[]),
            Err(M1ServingRegistryErrorV1::CompletionEpochMismatch)
        );
        assert_eq!(
            registry.phase(request),
            Some(M1ServingRequestPhaseV1::InFlight { epoch })
        );
        complete_exact(
            &mut registry,
            epoch,
            &[M1ServingCompletionDispositionV1::Continue(speculative_s1())],
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
        assert_eq!(registry.plan(request), Some(speculative_s1()));
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
            complete_exact(&mut registry, CompletionEpoch::new(1), &[]),
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
                requests: reservation.requests().iter().copied().collect(),
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
                requests: reservation.requests().iter().copied().collect(),
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
                requests: [second, first].into_iter().collect(),
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

    #[test]
    fn completed_window_reservation_restores_exact_predecessor_records() {
        let first = RequestId::new(0, 1);
        let second = RequestId::new(1, 1);
        let mut registry = M1ServingRegistryV1::<8>::new().unwrap();
        registry.admit(first, prefill_s8()).unwrap();
        registry.admit(second, prefill_s8()).unwrap();
        let completed_epoch = registry.plan_next().unwrap().unwrap().epoch();
        publish_and_complete(
            &mut registry,
            &[
                M1ServingCompletionDispositionV1::Retire,
                M1ServingCompletionDispositionV1::Retire,
            ],
        );

        let next_first = RequestId::new(0, 2);
        let next_second = RequestId::new(1, 2);
        let reservation = registry
            .reserve_completed_window_replacement(
                prefill_s8(),
                vec![next_first, next_second].into_boxed_slice(),
            )
            .unwrap();
        let first_reservation_id = reservation.publication.id;
        assert_eq!(reservation.prior_plan(), prefill_s8());
        assert_eq!(reservation.predecessor_count(), 2);
        assert_eq!(
            reservation.predecessor(0),
            Some(M1ServingCompletedWindowMemberV1 {
                request: first,
                plan: prefill_s8(),
                completion_epoch: completed_epoch,
            })
        );
        assert_eq!(
            reservation.predecessor(1),
            Some(M1ServingCompletedWindowMemberV1 {
                request: second,
                plan: prefill_s8(),
                completion_epoch: completed_epoch,
            })
        );
        assert_eq!(reservation.requests(), [next_first, next_second]);
        assert_eq!(reservation.epoch(), CompletionEpoch::new(2));
        assert_eq!(
            reservation.action(),
            M1ServingQueueActionV1::QuiescentNewWindow {
                prior: prefill_s8(),
                next: prefill_s8(),
            }
        );
        assert_eq!(registry.phase(first), None);
        assert_eq!(registry.phase(second), None);
        assert_eq!(
            registry.phase(next_first),
            Some(M1ServingRequestPhaseV1::Ready)
        );
        assert_eq!(
            registry.phase(next_second),
            Some(M1ServingRequestPhaseV1::Ready)
        );
        assert_eq!(
            registry.admit(RequestId::new(2, 1), prefill_s8()),
            Err(M1ServingRegistryErrorV1::PublicationReservationActive)
        );

        let restored = registry
            .restore_completed_window_replacement(reservation)
            .unwrap();
        assert_eq!(restored.plan(), prefill_s8());
        assert_eq!(restored.into_requests().as_ref(), [next_first, next_second]);
        assert!(!registry.has_publication_reservation());
        assert_eq!(
            registry.phase(first),
            Some(M1ServingRequestPhaseV1::Retired {
                quiescence: M1ServingQuiescenceV1::Completed(completed_epoch),
            })
        );
        assert_eq!(
            registry.phase(second),
            Some(M1ServingRequestPhaseV1::Retired {
                quiescence: M1ServingQuiescenceV1::Completed(completed_epoch),
            })
        );
        assert_eq!(registry.phase(next_first), None);
        assert_eq!(registry.phase(next_second), None);
        assert_eq!(registry.bound_plan(), Some(prefill_s8()));
        assert_eq!(registry.plan_next().unwrap(), None);

        let retry = registry
            .reserve_completed_window_replacement(
                prefill_s8(),
                vec![next_first, next_second].into_boxed_slice(),
            )
            .unwrap();
        assert_eq!(retry.publication.id, first_reservation_id + 1);
        let _ = registry
            .restore_completed_window_replacement(retry)
            .unwrap();
    }

    #[test]
    fn completed_window_publication_drops_old_records_only_after_exact_commit() {
        let prior = RequestId::new(0, 1);
        let next = RequestId::new(0, 2);
        let mut registry = M1ServingRegistryV1::<1>::new().unwrap();
        registry.admit(prior, prefill_s1()).unwrap();
        publish_and_complete(&mut registry, &[M1ServingCompletionDispositionV1::Retire]);
        let reservation = registry
            .reserve_completed_window_replacement(prefill_s1(), vec![next].into_boxed_slice())
            .unwrap();
        let epoch = reservation.epoch();
        registry
            .preflight_new_window_publication(&reservation)
            .unwrap();
        registry.record_new_window_publication(reservation).unwrap();

        assert_eq!(registry.phase(prior), None);
        assert_eq!(
            registry.phase(next),
            Some(M1ServingRequestPhaseV1::InFlight { epoch })
        );
        assert_eq!(registry.bound_plan(), Some(prefill_s1()));
        assert!(registry.has_in_flight_batch());
        complete_exact(
            &mut registry,
            epoch,
            &[M1ServingCompletionDispositionV1::Retire],
        )
        .unwrap();
    }

    #[test]
    fn completed_window_reservation_rejects_nonterminal_and_hostile_new_rosters() {
        let prior = RequestId::new(0, 1);
        let mut registry = M1ServingRegistryV1::<2>::new().unwrap();
        registry.admit(prior, prefill_s1()).unwrap();
        let failure = registry
            .reserve_completed_window_replacement(
                prefill_s1(),
                vec![RequestId::new(0, 2)].into_boxed_slice(),
            )
            .unwrap_err();
        assert_eq!(failure.error(), M1ServingRegistryErrorV1::QueuePlanMismatch);
        let _ = failure.into_requests();

        publish_and_complete(&mut registry, &[M1ServingCompletionDispositionV1::Retire]);
        let never_submitted = RequestId::new(1, 1);
        registry.admit(never_submitted, prefill_s1()).unwrap();
        registry.cancel(never_submitted).unwrap();
        let failure = registry
            .reserve_completed_window_replacement(
                prefill_s1(),
                vec![RequestId::new(0, 2)].into_boxed_slice(),
            )
            .unwrap_err();
        assert_eq!(
            failure.error(),
            M1ServingRegistryErrorV1::NewWindowPredecessorNotCompleted { index: 1 }
        );
        let _ = failure.into_requests();
        registry.remove_retired(never_submitted).unwrap();

        for (plan, proposed, error) in [
            (
                prefill_s1(),
                Vec::new(),
                M1ServingRegistryErrorV1::NewWindowRosterEmpty,
            ),
            (
                prefill_s1(),
                vec![RequestId::new(0, 0)],
                M1ServingRegistryErrorV1::InvalidRequest,
            ),
            (
                prefill_s1(),
                vec![RequestId::new(0, 2), RequestId::new(1, 2)],
                M1ServingRegistryErrorV1::NewWindowRosterExceedsPlan,
            ),
            (
                decode_s1(),
                vec![RequestId::new(0, 2)],
                M1ServingRegistryErrorV1::AdmissionRequiresPrefill,
            ),
        ] {
            let failure = registry
                .reserve_completed_window_replacement(plan, proposed.into_boxed_slice())
                .unwrap_err();
            assert_eq!(failure.error(), error);
            let _ = failure.into_requests();
            assert_eq!(
                registry.phase(prior),
                Some(M1ServingRequestPhaseV1::Retired {
                    quiescence: M1ServingQuiescenceV1::Completed(CompletionEpoch::new(1)),
                })
            );
            assert!(!registry.has_publication_reservation());
        }

        let mut wide_registry = M1ServingRegistryV1::<8>::new().unwrap();
        wide_registry.admit(prior, prefill_s8()).unwrap();
        publish_and_complete(
            &mut wide_registry,
            &[M1ServingCompletionDispositionV1::Retire],
        );
        let failure = wide_registry
            .reserve_completed_window_replacement(
                prefill_s8(),
                vec![RequestId::new(1, 2), RequestId::new(1, 3)].into_boxed_slice(),
            )
            .unwrap_err();
        assert_eq!(
            failure.error(),
            M1ServingRegistryErrorV1::NewWindowRosterCountMismatch
        );
        let _ = failure.into_requests();
    }

    #[test]
    fn completed_window_transaction_rejects_cross_registry_and_action_substitution() {
        let prior = RequestId::new(0, 1);
        let next = RequestId::new(0, 2);
        let mut registry = M1ServingRegistryV1::<1>::new().unwrap();
        registry.admit(prior, prefill_s1()).unwrap();
        publish_and_complete(&mut registry, &[M1ServingCompletionDispositionV1::Retire]);
        let reservation = registry
            .reserve_completed_window_replacement(prefill_s1(), vec![next].into_boxed_slice())
            .unwrap();

        let mut other = M1ServingRegistryV1::<1>::new().unwrap();
        let failure = other
            .restore_completed_window_replacement(reservation)
            .unwrap_err();
        assert_eq!(
            failure.error(),
            M1ServingRegistryErrorV1::RegistryIdentityMismatch
        );
        let mut reservation = (*failure).into_reservation();
        reservation.publication.batch.action = M1ServingQueueActionV1::FreshLaunch;
        let failure = registry
            .restore_completed_window_replacement(reservation)
            .unwrap_err();
        assert_eq!(
            failure.error(),
            M1ServingRegistryErrorV1::PublicationReservationMismatch
        );
        let mut reservation = (*failure).into_reservation();
        reservation.publication.batch.action = M1ServingQueueActionV1::QuiescentNewWindow {
            prior: prefill_s1(),
            next: prefill_s1(),
        };
        let restored = registry
            .restore_completed_window_replacement(reservation)
            .unwrap();
        assert_eq!(restored.into_requests().as_ref(), [next]);
        assert_eq!(
            registry.phase(prior),
            Some(M1ServingRequestPhaseV1::Retired {
                quiescence: M1ServingQuiescenceV1::Completed(CompletionEpoch::new(1)),
            })
        );
    }

    #[test]
    fn completed_window_reservation_accepts_staggered_terminal_epochs() {
        let first = RequestId::new(0, 1);
        let second = RequestId::new(1, 1);
        let mut registry = M1ServingRegistryV1::<8>::new().unwrap();
        registry.admit(first, prefill_s8()).unwrap();
        registry.admit(second, prefill_s8()).unwrap();
        publish_and_complete(
            &mut registry,
            &[
                M1ServingCompletionDispositionV1::Continue(speculative_s8()),
                M1ServingCompletionDispositionV1::Continue(speculative_s8()),
            ],
        );
        publish_and_complete(
            &mut registry,
            &[
                M1ServingCompletionDispositionV1::Retire,
                M1ServingCompletionDispositionV1::Continue(speculative_s8()),
            ],
        );
        publish_and_complete(&mut registry, &[M1ServingCompletionDispositionV1::Retire]);

        let first_epoch = CompletionEpoch::new(2);
        let second_epoch = CompletionEpoch::new(3);
        assert_eq!(
            registry.phase(first),
            Some(M1ServingRequestPhaseV1::Retired {
                quiescence: M1ServingQuiescenceV1::Completed(first_epoch),
            })
        );
        assert_eq!(
            registry.phase(second),
            Some(M1ServingRequestPhaseV1::Retired {
                quiescence: M1ServingQuiescenceV1::Completed(second_epoch),
            })
        );

        let next_first = RequestId::new(0, 2);
        let next_second = RequestId::new(1, 2);
        let reservation = registry
            .reserve_completed_window_replacement(
                prefill_s8(),
                vec![next_first, next_second].into_boxed_slice(),
            )
            .unwrap();
        assert_eq!(reservation.prior_plan(), speculative_s8());
        assert_eq!(reservation.epoch(), CompletionEpoch::new(4));
        assert_eq!(
            reservation
                .predecessor(0)
                .map(M1ServingCompletedWindowMemberV1::completion_epoch),
            Some(first_epoch)
        );
        assert_eq!(
            reservation
                .predecessor(1)
                .map(M1ServingCompletedWindowMemberV1::completion_epoch),
            Some(second_epoch)
        );
        registry
            .preflight_new_window_publication(&reservation)
            .unwrap();

        let restored = registry
            .restore_completed_window_replacement(reservation)
            .unwrap();
        assert_eq!(restored.into_requests().as_ref(), [next_first, next_second]);
        assert_eq!(
            registry.phase(first).unwrap(),
            M1ServingRequestPhaseV1::Retired {
                quiescence: M1ServingQuiescenceV1::Completed(first_epoch),
            }
        );
        assert_eq!(
            registry.phase(second).unwrap(),
            M1ServingRequestPhaseV1::Retired {
                quiescence: M1ServingQuiescenceV1::Completed(second_epoch),
            }
        );
    }

    #[test]
    fn completed_window_reservation_rejects_invalid_predecessor_quiescence() {
        let first = RequestId::new(0, 1);
        let mut registry = M1ServingRegistryV1::<1>::new().unwrap();
        registry.admit(first, prefill_s1()).unwrap();
        publish_and_complete(&mut registry, &[M1ServingCompletionDispositionV1::Retire]);
        let next = RequestId::new(0, 2);
        let completed = CompletionEpoch::new(1);

        registry.entries[0].last_quiescence = None;
        let failure = registry
            .reserve_completed_window_replacement(prefill_s1(), vec![next].into_boxed_slice())
            .unwrap_err();
        assert_eq!(
            failure.error(),
            M1ServingRegistryErrorV1::NewWindowPredecessorEpochMismatch { index: 0 }
        );
        assert_eq!(failure.into_requests().as_ref(), [next]);
        assert!(!registry.has_publication_reservation());

        registry.entries[0].last_quiescence = Some(CompletionEpoch::new(2));
        let failure = registry
            .reserve_completed_window_replacement(prefill_s1(), vec![next].into_boxed_slice())
            .unwrap_err();
        assert_eq!(
            failure.error(),
            M1ServingRegistryErrorV1::NewWindowPredecessorEpochMismatch { index: 0 }
        );
        assert_eq!(failure.into_requests().as_ref(), [next]);
        assert!(!registry.has_publication_reservation());

        let future = CompletionEpoch::new(2);
        registry.entries[0].phase = M1ServingRequestPhaseV1::Retired {
            quiescence: M1ServingQuiescenceV1::Completed(future),
        };
        registry.entries[0].last_quiescence = Some(future);
        let failure = registry
            .reserve_completed_window_replacement(prefill_s1(), vec![next].into_boxed_slice())
            .unwrap_err();
        assert_eq!(
            failure.error(),
            M1ServingRegistryErrorV1::NewWindowPredecessorEpochMismatch { index: 0 }
        );
        assert_eq!(failure.into_requests().as_ref(), [next]);
        assert!(!registry.has_publication_reservation());

        registry.entries[0].phase = M1ServingRequestPhaseV1::Retired {
            quiescence: M1ServingQuiescenceV1::Completed(completed),
        };
        registry.entries[0].last_quiescence = Some(completed);
        let reservation = registry
            .reserve_completed_window_replacement(prefill_s1(), vec![next].into_boxed_slice())
            .unwrap();
        let _ = registry
            .restore_completed_window_replacement(reservation)
            .unwrap();
    }

    #[test]
    fn completed_window_predecessor_plan_epoch_and_request_generation_are_exact() {
        let prior = RequestId::new(0, 2);
        let mut registry = M1ServingRegistryV1::<1>::new().unwrap();
        registry.admit(prior, prefill_s1()).unwrap();
        publish_and_complete(&mut registry, &[M1ServingCompletionDispositionV1::Retire]);

        registry.entries[0].plan = decode_s1();
        let failure = registry
            .reserve_completed_window_replacement(
                prefill_s1(),
                vec![RequestId::new(0, 3)].into_boxed_slice(),
            )
            .unwrap_err();
        assert_eq!(
            failure.error(),
            M1ServingRegistryErrorV1::NewWindowPredecessorPlanMismatch { index: 0 }
        );
        let _ = failure.into_requests();
        registry.entries[0].plan = prefill_s1();

        registry.entries[0].phase = M1ServingRequestPhaseV1::Retired {
            quiescence: M1ServingQuiescenceV1::Completed(CompletionEpoch::new(2)),
        };
        registry.entries[0].last_quiescence = Some(CompletionEpoch::new(2));
        let failure = registry
            .reserve_completed_window_replacement(
                prefill_s1(),
                vec![RequestId::new(0, 3)].into_boxed_slice(),
            )
            .unwrap_err();
        assert_eq!(
            failure.error(),
            M1ServingRegistryErrorV1::NewWindowPredecessorEpochMismatch { index: 0 }
        );
        let _ = failure.into_requests();
        registry.entries[0].phase = M1ServingRequestPhaseV1::Retired {
            quiescence: M1ServingQuiescenceV1::Completed(CompletionEpoch::new(1)),
        };
        registry.entries[0].last_quiescence = Some(CompletionEpoch::new(1));

        for generation in [1, 2, 4] {
            let failure = registry
                .reserve_completed_window_replacement(
                    prefill_s1(),
                    vec![RequestId::new(0, generation)].into_boxed_slice(),
                )
                .unwrap_err();
            assert_eq!(
                failure.error(),
                M1ServingRegistryErrorV1::NewWindowRequestGenerationMismatch { index: 0 }
            );
            let _ = failure.into_requests();
            assert!(!registry.has_publication_reservation());
        }

        let reservation = registry
            .reserve_completed_window_replacement(
                prefill_s1(),
                vec![RequestId::new(0, 3)].into_boxed_slice(),
            )
            .unwrap();
        let restored = registry
            .restore_completed_window_replacement(reservation)
            .unwrap();
        assert_eq!(restored.into_requests().as_ref(), [RequestId::new(0, 3)]);
    }

    #[test]
    fn completed_window_roster_slots_order_and_generation_overflow_are_exact() {
        let first = RequestId::new(3, 7);
        let second = RequestId::new(9, 11);
        let mut registry = M1ServingRegistryV1::<8>::new().unwrap();
        registry.admit(first, prefill_s8()).unwrap();
        registry.admit(second, prefill_s8()).unwrap();
        publish_and_complete(
            &mut registry,
            &[
                M1ServingCompletionDispositionV1::Retire,
                M1ServingCompletionDispositionV1::Retire,
            ],
        );

        for (proposed, error) in [
            (
                vec![RequestId::new(3, 8)],
                M1ServingRegistryErrorV1::NewWindowRosterCountMismatch,
            ),
            (
                vec![RequestId::new(9, 12), RequestId::new(3, 8)],
                M1ServingRegistryErrorV1::NewWindowRequestSlotMismatch { index: 0 },
            ),
            (
                vec![RequestId::new(3, 8), RequestId::new(12, 1)],
                M1ServingRegistryErrorV1::NewWindowRequestSlotMismatch { index: 1 },
            ),
        ] {
            let failure = registry
                .reserve_completed_window_replacement(prefill_s8(), proposed.into_boxed_slice())
                .unwrap_err();
            assert_eq!(failure.error(), error);
            let _ = failure.into_requests();
            assert!(!registry.has_publication_reservation());
        }

        let exact = registry
            .reserve_completed_window_replacement(
                prefill_s8(),
                vec![RequestId::new(3, 8), RequestId::new(9, 12)].into_boxed_slice(),
            )
            .unwrap();
        let _ = registry
            .restore_completed_window_replacement(exact)
            .unwrap();

        let maximum = RequestId::new(0, u32::MAX);
        let mut overflow_registry = M1ServingRegistryV1::<1>::new().unwrap();
        overflow_registry.admit(maximum, prefill_s1()).unwrap();
        publish_and_complete(
            &mut overflow_registry,
            &[M1ServingCompletionDispositionV1::Retire],
        );
        let failure = overflow_registry
            .reserve_completed_window_replacement(
                prefill_s1(),
                vec![RequestId::new(0, 1)].into_boxed_slice(),
            )
            .unwrap_err();
        assert_eq!(
            failure.error(),
            M1ServingRegistryErrorV1::NewWindowRequestGenerationMismatch { index: 0 }
        );
        let _ = failure.into_requests();
        assert!(!overflow_registry.has_publication_reservation());
    }

    fn authenticated_prefill_logical_state(
        request: RequestId,
        role: Qwen3ModelRole,
    ) -> ferric_spec::LogicalKvState {
        ferric_spec::LogicalKvState {
            request,
            role,
            lifecycle: PhysicalKvLifecycle::Active,
            resident_tokens: 128,
            committed_tokens: 128,
        }
    }

    #[test]
    fn authenticated_prefill_member_reconciliation_rejects_generation_and_cache_drift() {
        enum Drift {
            MemberRequest,
            TargetRequest,
            DraftRequest,
            TargetRole,
            DraftRole,
            TargetLifecycle,
            DraftLifecycle,
            TargetResident,
            TargetCommitted,
            DraftResident,
            DraftCommitted,
            TargetActivePages,
            DraftActivePages,
            TargetRetiredPages,
            DraftRetiredPages,
            TargetQuiescentPages,
            DraftQuiescentPages,
            TargetWritePending,
            DraftWritePending,
            TargetFuturePages,
            Device,
            TargetArena,
            DraftArena,
        }

        let request = RequestId::new(0, 7);
        let target = authenticated_prefill_logical_state(request, Qwen3ModelRole::Target8B);
        let draft = authenticated_prefill_logical_state(request, Qwen3ModelRole::Draft06B);
        let exact = M1AuthenticatedPrefillRegistryMemberFactsV1 {
            member_request: request,
            target,
            draft,
            target_active_pages: 8,
            draft_active_pages: 8,
            target_retired_pages: 0,
            draft_retired_pages: 0,
            target_quiescent_retired_pages: 0,
            draft_quiescent_retired_pages: 0,
            target_write_pending: false,
            draft_write_pending: false,
            target_qualification_future_pages: 0,
            device_join: M1AuthenticatedPrefillRegistryMemberAssociationV1::Exact,
            target_arena_join: M1AuthenticatedPrefillRegistryMemberAssociationV1::Exact,
            draft_arena_join: M1AuthenticatedPrefillRegistryMemberAssociationV1::Exact,
        };
        assert_eq!(
            validate_authenticated_prefill_registry_member_v1(request, exact),
            Ok(())
        );

        let request_mismatch = M1AuthenticatedPrefillRegistryReconciliationErrorV1::RequestMismatch;
        let cache_mismatch = M1AuthenticatedPrefillRegistryReconciliationErrorV1::CacheMismatch;
        let cases = [
            ("member request", Drift::MemberRequest, request_mismatch),
            ("target request", Drift::TargetRequest, request_mismatch),
            ("draft request", Drift::DraftRequest, request_mismatch),
            ("target role", Drift::TargetRole, cache_mismatch),
            ("draft role", Drift::DraftRole, cache_mismatch),
            ("target lifecycle", Drift::TargetLifecycle, cache_mismatch),
            ("draft lifecycle", Drift::DraftLifecycle, cache_mismatch),
            ("target resident", Drift::TargetResident, cache_mismatch),
            ("target committed", Drift::TargetCommitted, cache_mismatch),
            ("draft resident", Drift::DraftResident, cache_mismatch),
            ("draft committed", Drift::DraftCommitted, cache_mismatch),
            (
                "target active pages",
                Drift::TargetActivePages,
                cache_mismatch,
            ),
            (
                "draft active pages",
                Drift::DraftActivePages,
                cache_mismatch,
            ),
            (
                "target retired pages",
                Drift::TargetRetiredPages,
                cache_mismatch,
            ),
            (
                "draft retired pages",
                Drift::DraftRetiredPages,
                cache_mismatch,
            ),
            (
                "target quiescent pages",
                Drift::TargetQuiescentPages,
                cache_mismatch,
            ),
            (
                "draft quiescent pages",
                Drift::DraftQuiescentPages,
                cache_mismatch,
            ),
            (
                "target write pending",
                Drift::TargetWritePending,
                cache_mismatch,
            ),
            (
                "draft write pending",
                Drift::DraftWritePending,
                cache_mismatch,
            ),
            (
                "target future pages",
                Drift::TargetFuturePages,
                cache_mismatch,
            ),
            ("device", Drift::Device, cache_mismatch),
            ("target arena", Drift::TargetArena, cache_mismatch),
            ("draft arena", Drift::DraftArena, cache_mismatch),
        ];
        for (name, drift, expected) in cases {
            let mut hostile = exact;
            match drift {
                Drift::MemberRequest => hostile.member_request = RequestId::new(0, 8),
                Drift::TargetRequest => hostile.target.request = RequestId::new(0, 6),
                Drift::DraftRequest => hostile.draft.request = RequestId::new(0, 6),
                Drift::TargetRole => hostile.target.role = Qwen3ModelRole::Draft06B,
                Drift::DraftRole => hostile.draft.role = Qwen3ModelRole::Target8B,
                Drift::TargetLifecycle => {
                    hostile.target.lifecycle = PhysicalKvLifecycle::Cancelled {
                        after_epoch: CompletionEpoch::new(1),
                    };
                }
                Drift::DraftLifecycle => {
                    hostile.draft.lifecycle = PhysicalKvLifecycle::RetiredAwaitingQuiescence {
                        after_epoch: CompletionEpoch::new(1),
                    };
                }
                Drift::TargetResident => hostile.target.resident_tokens = 127,
                Drift::TargetCommitted => hostile.target.committed_tokens = 127,
                Drift::DraftResident => hostile.draft.resident_tokens = 127,
                Drift::DraftCommitted => hostile.draft.committed_tokens = 127,
                Drift::TargetActivePages => hostile.target_active_pages = 7,
                Drift::DraftActivePages => hostile.draft_active_pages = 7,
                Drift::TargetRetiredPages => hostile.target_retired_pages = 1,
                Drift::DraftRetiredPages => hostile.draft_retired_pages = 1,
                Drift::TargetQuiescentPages => hostile.target_quiescent_retired_pages = 1,
                Drift::DraftQuiescentPages => hostile.draft_quiescent_retired_pages = 1,
                Drift::TargetWritePending => hostile.target_write_pending = true,
                Drift::DraftWritePending => hostile.draft_write_pending = true,
                Drift::TargetFuturePages => hostile.target_qualification_future_pages = 1,
                Drift::Device => {
                    hostile.device_join =
                        M1AuthenticatedPrefillRegistryMemberAssociationV1::Mismatch;
                }
                Drift::TargetArena => {
                    hostile.target_arena_join =
                        M1AuthenticatedPrefillRegistryMemberAssociationV1::Mismatch;
                }
                Drift::DraftArena => {
                    hostile.draft_arena_join =
                        M1AuthenticatedPrefillRegistryMemberAssociationV1::Mismatch;
                }
            }
            assert_eq!(
                validate_authenticated_prefill_registry_member_v1(request, hostile),
                Err(expected),
                "hostile cache drift accepted: {name}"
            );
        }
    }

    fn assert_authenticated_prefill_frontier_matches_ordinary<const C: usize>() {
        let request = RequestId::new(0, 1);
        let prefill = pair(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128);
        let successor = pair(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
        );
        let epoch = CompletionEpoch::new(1);

        let mut ordinary = M1ServingRegistryV1::<C>::new().unwrap();
        ordinary.admit(request, prefill).unwrap();
        let first = ordinary.plan_next().unwrap().unwrap();
        let first_reservation = ordinary.reserve_publication(first).unwrap();
        ordinary.record_publication(first_reservation).unwrap();
        ordinary
            .validate_completion_exact(
                epoch,
                &[M1ServingCompletionDispositionV1::Continue(successor)],
            )
            .unwrap();
        ordinary.apply_validated_completion(
            epoch,
            &[M1ServingCompletionDispositionV1::Continue(successor)],
        );

        let mut reconciled = M1ServingRegistryV1::<C>::new().unwrap();
        reconciled
            .install_authenticated_prefill_reconciliation(
                M1AuthenticatedPrefillRegistryReconciliationV1 {
                    request,
                    epoch,
                    prefill,
                    successor,
                },
            )
            .unwrap();
        assert_eq!(reconciled.phase(request), ordinary.phase(request));
        assert_eq!(reconciled.plan(request), ordinary.plan(request));
        assert_eq!(reconciled.bound_plan(), ordinary.bound_plan());
        let ordinary_next = ordinary.plan_next().unwrap().unwrap();
        let reconciled_next = reconciled.plan_next().unwrap().unwrap();
        assert_eq!(ordinary_next, reconciled_next);
        let ordinary_reservation = ordinary.reserve_publication(ordinary_next).unwrap();
        let reconciled_reservation = reconciled.reserve_publication(reconciled_next).unwrap();
        assert_eq!(ordinary_reservation.id, 2);
        assert_eq!(reconciled_reservation.id, ordinary_reservation.id);
    }

    #[test]
    fn authenticated_s1_prefill_frontier_is_independent_of_registry_capacity() {
        assert_authenticated_prefill_frontier_matches_ordinary::<1>();
        assert_authenticated_prefill_frontier_matches_ordinary::<8>();
        assert_authenticated_prefill_frontier_matches_ordinary::<32>();
    }

    #[test]
    fn authenticated_prefill_install_rejects_nonfresh_registry_without_mutation() {
        let request = RequestId::new(0, 1);
        let prefill = pair(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128);
        let successor = pair(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
        );
        let reconciliation = M1AuthenticatedPrefillRegistryReconciliationV1 {
            request,
            epoch: CompletionEpoch::new(1),
            prefill,
            successor,
        };
        let mut registry = M1ServingRegistryV1::<1>::new().unwrap();
        registry.admit(request, prefill).unwrap();
        assert_eq!(
            registry.install_authenticated_prefill_reconciliation(reconciliation),
            Err(M1AuthenticatedPrefillRegistryReconciliationErrorV1::RegistryNotFresh)
        );
        assert_eq!(
            registry.phase(request),
            Some(M1ServingRequestPhaseV1::Ready)
        );
        assert_eq!(registry.plan(request), Some(prefill));
        assert_eq!(registry.bound_plan(), None);
    }

    #[test]
    fn authenticated_prefill_reconciliation_source_uses_real_completion_only() {
        let source = include_str!("m1_serving_registry.rs");
        let custody = source
            .split(
                "/// Fail-closed rejection while reconciling a real authenticated paired-prefill",
            )
            .nth(1)
            .unwrap()
            .split("/// Deterministic Ferric registry")
            .next()
            .unwrap();
        let registry_glue_start = source
            .find("fn is_fresh_for_authenticated_prefill_reconciliation")
            .unwrap();
        let registry_glue_end = source[registry_glue_start..]
            .find("/// Registers a newly Engine-admitted request")
            .unwrap()
            + registry_glue_start;
        let registry_glue = &source[registry_glue_start..registry_glue_end];
        let reconciliation_start = source
            .find("const fn authenticated_s1_t128_prefill_selection")
            .unwrap();
        let reconciliation_end = source[reconciliation_start..]
            .find("fn validate_completed_window_predecessor")
            .unwrap()
            + reconciliation_start;
        let reconciliation = &source[reconciliation_start..reconciliation_end];
        let production = [custody, registry_glue, reconciliation].concat();
        for forbidden in ["unwrap(", "expect(", "panic!(", "unreachable!("] {
            assert!(
                !production.contains(forbidden),
                "forbidden reconciliation production path: {forbidden}"
            );
        }
        let validator = source
            .split("fn validate_authenticated_prefill_registry_reconciliation_v1")
            .nth(1)
            .unwrap()
            .split("/// Reconciles a fresh metadata registry")
            .next()
            .unwrap();
        for forbidden in ["unwrap(", "expect(", "panic!(", "unreachable!("] {
            assert!(
                !validator.contains(forbidden),
                "forbidden validator path: {forbidden}"
            );
        }
        for required in [
            "speculative_rollover_intent()",
            "join_m1_authenticated_prefill_registry_intent_v1",
            "released.queue().shape()",
            "checked.records()",
            "released.members()",
            "cache.projection()",
            "completed.engine().state(request)",
        ] {
            assert!(
                validator.contains(required),
                "missing physical join: {required}"
            );
        }
        let body = source
            .split("pub fn reconcile_m1_authenticated_s1_t128_prefill_registry_v1")
            .nth(1)
            .unwrap()
            .split("fn validate_completed_window_predecessor")
            .next()
            .unwrap();
        for forbidden in [
            ".admit(",
            "reserve_publication(",
            "record_publication(",
            "apply_preflighted_completion(",
        ] {
            assert!(
                !body.contains(forbidden),
                "forbidden synthetic path: {forbidden}"
            );
        }
        assert!(body.contains("validate_authenticated_prefill_registry_reconciliation_v1"));
        assert!(body.contains("install_authenticated_prefill_reconciliation"));
        assert!(!body.contains("C != 1"));

        let registry_bodies = [
            "fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result",
            "pub const fn error(&self) -> M1AuthenticatedPrefillRegistryReconciliationErrorV1",
            "pub const fn request(&self) -> RequestId",
            "pub const fn completion_epoch(&self) -> CompletionEpoch",
            "pub const fn prefill_plan(&self) -> M1ServingPlanV1",
            "pub const fn successor_plan(&self) -> M1ServingPlanV1",
            "pub const fn first_token(&self) -> ferric_spec::TokenId",
            "pub const fn successor_epoch(&self) -> CompletionEpoch",
            "pub(crate) fn into_parts(",
            "fn is_fresh_for_authenticated_prefill_reconciliation(&self) -> bool",
            "fn install_authenticated_prefill_reconciliation(",
            "const fn authenticated_s1_t128_prefill_selection(",
            "const fn authenticated_s1_k4_selection(",
            "fn validate_authenticated_prefill_registry_member_v1(",
            "fn validate_authenticated_prefill_registry_reconciliation_v1<",
            "pub fn reconcile_m1_authenticated_s1_t128_prefill_registry_v1<",
        ];
        assert_eq!(registry_bodies.len(), 16);
        for signature in registry_bodies {
            assert!(
                production.contains(signature),
                "new body omitted from source roster: {signature}"
            );
        }
        assert_eq!(production.matches(registry_bodies[0]).count(), 2);

        let rollover_source = include_str!("authenticated_queue_rollover.rs");
        assert_eq!(
            rollover_source
                .matches("fn join_m1_authenticated_prefill_registry_intent_v1(")
                .count(),
            1,
            "registry intent join helper omitted or duplicated"
        );
        let typed = reconcile_m1_authenticated_s1_t128_prefill_registry_v1::<8>;
        assert_eq!(core::mem::size_of_val(&typed), 0);
    }
}
