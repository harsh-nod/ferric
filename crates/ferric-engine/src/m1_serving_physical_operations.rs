//! Production physical-operation adapter for dynamic M1 serving.
//!
//! The registry intentionally owns no tokens, workspace images, page leases,
//! or active device-cache owners. Those inputs stay behind
//! [`M1ServingPhysicalInputProviderV1`], while this adapter alone performs the
//! exact scheduler transition and consumes the resulting physical typestates.
//! Direct paired-prefill and target-only generations use compact final-row
//! semantics. S1/K4 speculative generations additionally require the
//! independent diagnostic-choice attachment. Wider speculative shapes fail
//! before scheduler or queue progress.

use core::{
    fmt,
    sync::atomic::{AtomicU64, Ordering},
};

use ferric_spec::{completion::CompletionEpoch, Qwen3PlanBucket, RequestId};

use crate::{
    complete_m1_physical_step_v1, release_m1_completed_step_kv_pages_v1,
    schedule_m1_long_lived_queue_rearm_exact_v1, schedule_m1_s1_k4_queue_rollover_exact_v1,
    ActiveDeviceKvCache, AddresslessM1PhysicalBufferRecipeV1, BoundM1CompletionOutputV1, Engine,
    M1AllocatedScheduledStepV1, M1CheckedCompletionOutputV1, M1CompletedStepOutcomeV1,
    M1CompletedStepRejectionV1, M1DeviceKvCompletionDispositionV1, M1DeviceKvCompletionMemberV1,
    M1DeviceKvCompletionRosterV1, M1ExactDispatchErrorV1, M1LongLivedQueueReleasedRoundV1,
    M1LongLivedQueueUnscheduledRoundV1, M1ObservedSpeculativeDiagnosticChoicesV1,
    M1PhysicalCompletedReadbackV1, M1PhysicalFixedBatchShapeV1, M1PhysicalPublishedQueueSessionV1,
    M1PhysicalRunnerV1, M1PreparedLongLivedQueueRearmV1, M1QueuedServingPhysicalInputProviderV1,
    M1RearmedCompletedReadbackV1, M1RearmedCompletionOutcomeV1,
    M1RearmedCompletionPreflightFailureV1, M1RearmedPublishedQueueV1,
    M1RearmedRoundReleaseOutcomeV1, M1ReleasedCompletedStepV1,
    M1S1K4QueueRolloverScheduleFailureCustodyV1, M1ScheduledDispatchV1,
    M1ScheduledLongLivedQueueRearmV1, M1ScheduledS1K4QueueRolloverV1, M1ServingBatchPlanV1,
    M1ServingCommittedSpeculativeRoundV1, M1ServingPhysicalOperationFailureV1,
    M1ServingPhysicalOperationResultV1, M1ServingPhysicalOperationsV1, M1ServingPlanV1,
    M1ServingQueuedGenerationBindingV1, M1ServingQueuedS1K4RolloverV1,
    M1ServingQueuedSameShapeRearmV1, M1ServingRolloverReasonV1, M1SpeculativeMemberStatusV1,
    M1_MAX_REARM_ROUND_HISTORY_V1,
};

/// Request-owned inputs prepared after the adapter issues the exact first dispatch.
#[must_use = "prepared first-publication custody must publish or remain retained"]
#[derive(Debug)]
pub struct M1ServingPreparedFirstPublicationV1 {
    allocated: M1AllocatedScheduledStepV1,
    recipe: AddresslessM1PhysicalBufferRecipeV1,
    completion_output: BoundM1CompletionOutputV1,
    selected: Vec<ActiveDeviceKvCache>,
    semantic_evidence: M1ServingPreparedSemanticEvidenceV1,
}

/// Provider-owned semantic evidence prepared independently of compact readback.
#[must_use = "prepared semantic evidence must remain paired with publication custody"]
#[derive(Debug)]
pub enum M1ServingPreparedSemanticEvidenceV1 {
    /// Direct choices will come from the attached independent K6 capture.
    Direct,
    /// S1/K4 will derive its expectations from the attached independent choices.
    SpeculativeK4,
}

impl M1ServingPreparedFirstPublicationV1 {
    /// Joins provider-owned physical inputs without granting publication authority.
    pub fn new(
        allocated: M1AllocatedScheduledStepV1,
        recipe: AddresslessM1PhysicalBufferRecipeV1,
        completion_output: BoundM1CompletionOutputV1,
        selected: Vec<ActiveDeviceKvCache>,
        semantic_evidence: M1ServingPreparedSemanticEvidenceV1,
    ) -> Self {
        Self {
            allocated,
            recipe,
            completion_output,
            selected,
            semantic_evidence,
        }
    }

    /// Recovers every unpublished physical owner after a fail-stop preparation rejection.
    #[must_use = "all prepared publication owners remain linear"]
    pub fn into_parts(
        self,
    ) -> (
        M1AllocatedScheduledStepV1,
        AddresslessM1PhysicalBufferRecipeV1,
        BoundM1CompletionOutputV1,
        Vec<ActiveDeviceKvCache>,
        M1ServingPreparedSemanticEvidenceV1,
    ) {
        (
            self.allocated,
            self.recipe,
            self.completion_output,
            self.selected,
            self.semantic_evidence,
        )
    }
}

/// Request-owned inputs prepared after exact same-shape rearm scheduling.
#[must_use = "prepared rearm custody must publish or remain retained"]
#[derive(Debug)]
pub struct M1ServingPreparedSameShapeRearmV1 {
    prepared: M1PreparedLongLivedQueueRearmV1,
    recipe: AddresslessM1PhysicalBufferRecipeV1,
    semantic_evidence: M1ServingPreparedSemanticEvidenceV1,
}

/// Request-owned inputs prepared after exact paired-prefill to S1/K4 scheduling.
#[must_use = "prepared rollover custody must publish or remain retained"]
#[derive(Debug)]
pub struct M1ServingPreparedS1K4RolloverV1 {
    prepared: crate::M1PreparedS1K4QueueRolloverV1,
    recipe: AddresslessM1PhysicalBufferRecipeV1,
    semantic_evidence: M1ServingPreparedSemanticEvidenceV1,
}

impl M1ServingPreparedS1K4RolloverV1 {
    /// Joins a prepared native rollover to its exact physical recipe and
    /// independent S1/K4 semantic evidence.
    pub const fn new(
        prepared: crate::M1PreparedS1K4QueueRolloverV1,
        recipe: AddresslessM1PhysicalBufferRecipeV1,
        semantic_evidence: M1ServingPreparedSemanticEvidenceV1,
    ) -> Self {
        Self {
            prepared,
            recipe,
            semantic_evidence,
        }
    }
}

impl M1ServingPreparedSameShapeRearmV1 {
    /// Joins the existing prepared-rearm typestate to its exact physical recipe.
    pub const fn new(
        prepared: M1PreparedLongLivedQueueRearmV1,
        recipe: AddresslessM1PhysicalBufferRecipeV1,
        semantic_evidence: M1ServingPreparedSemanticEvidenceV1,
    ) -> Self {
        Self {
            prepared,
            recipe,
            semantic_evidence,
        }
    }
}

/// Supplies request/model inputs that the serving registry deliberately does not own.
///
/// Implementations must retain every consumed scheduler, cache, lease, table,
/// workspace, and allocation owner inside `Failure` on rejection. The adapter
/// treats either failure as terminal because exact scheduling has already
/// advanced or detached the predecessor queue. Direct final-row expectations
/// must come from provider-owned evidence independent of the compact completion
/// image; deriving them from that image would be self-authenticating and is not
/// admitted by this production boundary.
pub trait M1ServingPhysicalInputProviderV1<const C: usize> {
    type Failure: fmt::Debug;

    /// # Errors
    ///
    /// Returns exhaustive provider custody when the exact first publication
    /// inputs cannot be prepared.
    fn prepare_first_publication(
        &mut self,
        runner: &M1PhysicalRunnerV1,
        engine: &mut Engine<C>,
        batch: &M1ServingBatchPlanV1,
        scheduled: M1ScheduledDispatchV1,
    ) -> Result<M1ServingPreparedFirstPublicationV1, Self::Failure>;

    /// # Errors
    ///
    /// Returns exhaustive provider custody when the exact same-shape inputs
    /// cannot be prepared.
    fn prepare_same_shape_rearm(
        &mut self,
        runner: &M1PhysicalRunnerV1,
        engine: &mut Engine<C>,
        batch: &M1ServingBatchPlanV1,
        scheduled: M1ScheduledLongLivedQueueRearmV1,
    ) -> Result<M1ServingPreparedSameShapeRearmV1, Self::Failure>;

    /// # Errors
    ///
    /// Returns exhaustive detached queue, cache, reservation, workspace, and
    /// semantic-evidence custody when S1/K4 rollover inputs cannot be prepared.
    fn prepare_s1_k4_rollover(
        &mut self,
        runner: &M1PhysicalRunnerV1,
        engine: &mut Engine<C>,
        batch: &M1ServingBatchPlanV1,
        scheduled: M1ScheduledS1K4QueueRolloverV1,
    ) -> Result<M1ServingPreparedS1K4RolloverV1, Self::Failure>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct M1ServingPhysicalRunnerAdapterIdentityV1(u64);

impl M1ServingPhysicalRunnerAdapterIdentityV1 {
    fn fresh() -> Option<Self> {
        static NEXT: AtomicU64 = AtomicU64::new(1);

        NEXT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
            next.checked_add(1)
        })
        .ok()
        .map(Self)
    }
}

/// Opaque diagnostic evidence history retained inside lifecycle custody.
#[must_use = "diagnostic evidence must remain retained"]
#[derive(Debug)]
pub struct M1ServingPhysicalRunnerDiagnosticHistoryV1 {
    evidence: Vec<M1ServingPhysicalRunnerReadbackEvidenceV1>,
}

impl M1ServingPhysicalRunnerDiagnosticHistoryV1 {
    fn new() -> Self {
        Self {
            evidence: Vec::new(),
        }
    }

    fn len(&self) -> usize {
        self.evidence.len()
    }

    fn try_reserve_exact(
        &mut self,
        additional: usize,
    ) -> Result<(), std::collections::TryReserveError> {
        self.evidence.try_reserve_exact(additional)
    }

    fn push(&mut self, evidence: M1ServingPhysicalRunnerReadbackEvidenceV1) {
        self.evidence.push(evidence);
    }

    /// Borrows settled direct or speculative evidence in generation order.
    pub fn evidence(&self) -> &[M1ServingPhysicalRunnerReadbackEvidenceV1] {
        &self.evidence
    }
}

/// Complete quiescent physical custody returned after one serving settlement.
#[must_use = "quiescent queue, caches, and diagnostic evidence must remain retained"]
#[derive(Debug)]
pub struct M1ServingPhysicalRunnerQuiescentV1 {
    adapter_identity: M1ServingPhysicalRunnerAdapterIdentityV1,
    epoch: CompletionEpoch,
    plan: M1ServingPlanV1,
    state: M1ServingPhysicalRunnerQuiescentStateV1,
}

#[derive(Debug)]
enum M1ServingPhysicalRunnerQuiescentStateV1 {
    First {
        released: M1ReleasedCompletedStepV1,
        diagnostic_history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    },
    Rearmed {
        released: M1LongLivedQueueReleasedRoundV1,
        diagnostic_history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    },
    Unscheduled {
        unscheduled: M1LongLivedQueueUnscheduledRoundV1,
        diagnostic_history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    },
}

impl M1ServingPhysicalRunnerQuiescentV1 {
    fn adapter_identity(&self) -> M1ServingPhysicalRunnerAdapterIdentityV1 {
        self.adapter_identity
    }

    fn epoch(&self) -> CompletionEpoch {
        self.epoch
    }

    /// Returns the exact serving plan retained by this physical queue.
    #[must_use]
    pub const fn plan(&self) -> M1ServingPlanV1 {
        self.plan
    }

    /// Borrows settled direct or speculative evidence in generation order.
    pub fn diagnostic_history(&self) -> &M1ServingPhysicalRunnerDiagnosticHistoryV1 {
        match &self.state {
            M1ServingPhysicalRunnerQuiescentStateV1::First {
                diagnostic_history, ..
            }
            | M1ServingPhysicalRunnerQuiescentStateV1::Rearmed {
                diagnostic_history, ..
            }
            | M1ServingPhysicalRunnerQuiescentStateV1::Unscheduled {
                diagnostic_history, ..
            } => diagnostic_history,
        }
    }

    fn first(
        adapter_identity: M1ServingPhysicalRunnerAdapterIdentityV1,
        epoch: CompletionEpoch,
        plan: M1ServingPlanV1,
        released: M1ReleasedCompletedStepV1,
        diagnostic_history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    ) -> Self {
        Self {
            adapter_identity,
            epoch,
            plan,
            state: M1ServingPhysicalRunnerQuiescentStateV1::First {
                released,
                diagnostic_history,
            },
        }
    }

    fn rearmed(
        adapter_identity: M1ServingPhysicalRunnerAdapterIdentityV1,
        epoch: CompletionEpoch,
        plan: M1ServingPlanV1,
        released: M1LongLivedQueueReleasedRoundV1,
        diagnostic_history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    ) -> Self {
        Self {
            adapter_identity,
            epoch,
            plan,
            state: M1ServingPhysicalRunnerQuiescentStateV1::Rearmed {
                released,
                diagnostic_history,
            },
        }
    }
}

/// One physically published S1/K4 serving generation.
#[must_use = "published physical generation must complete or remain retained"]
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub struct M1ServingPhysicalRunnerPublishedV1 {
    adapter_identity: M1ServingPhysicalRunnerAdapterIdentityV1,
    epoch: CompletionEpoch,
    plan: M1ServingPlanV1,
    state: M1ServingPhysicalRunnerPublishedStateV1,
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
enum M1ServingPhysicalRunnerPublishedStateV1 {
    First {
        published: M1PhysicalPublishedQueueSessionV1,
        selected: Vec<ActiveDeviceKvCache>,
        semantic_evidence: M1ServingPreparedSemanticEvidenceV1,
        diagnostic_history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    },
    Rearmed {
        published: M1RearmedPublishedQueueV1,
        semantic_evidence: M1ServingPreparedSemanticEvidenceV1,
        diagnostic_history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    },
}

impl M1ServingPhysicalRunnerPublishedV1 {
    fn adapter_identity(&self) -> M1ServingPhysicalRunnerAdapterIdentityV1 {
        self.adapter_identity
    }

    fn epoch(&self) -> CompletionEpoch {
        self.epoch
    }

    /// Returns the exact serving plan bound to this physical publication.
    #[must_use]
    pub const fn plan(&self) -> M1ServingPlanV1 {
        self.plan
    }
}

#[derive(Debug)]
pub enum M1ServingFirstReadbackStateV1 {
    Ready {
        readback: M1PhysicalCompletedReadbackV1,
        selected: Vec<ActiveDeviceKvCache>,
    },
    Rejected(M1CompletedStepRejectionV1),
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum M1ServingRearmedReadbackStateV1 {
    Ready(M1RearmedCompletedReadbackV1),
    PreflightRejected(M1RearmedCompletionPreflightFailureV1),
    CompletionRejected(M1RearmedCompletionOutcomeV1),
}

/// Exact semantic evidence retained through one serving readback.
#[must_use = "checked readback evidence must settle or remain retained"]
#[derive(Debug)]
pub enum M1ServingPhysicalRunnerReadbackEvidenceV1 {
    /// Independent target choices for paired-prefill or target-only semantics.
    Direct(crate::M1ObservedDirectDiagnosticChoicesV1),
    /// Independent S1/K4 draft and target choices.
    SpeculativeK4(Box<M1ObservedSpeculativeDiagnosticChoicesV1>),
}

impl M1ServingPhysicalRunnerReadbackEvidenceV1 {
    fn append_diagnostic_history(self, history: &mut M1ServingPhysicalRunnerDiagnosticHistoryV1) {
        history.push(self);
    }
}

/// Semantically joined readback retaining independent S1/K4 choice evidence.
#[must_use = "readback, caches, and diagnostic choices must settle or remain retained"]
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub struct M1ServingPhysicalRunnerReadbackV1 {
    adapter_identity: M1ServingPhysicalRunnerAdapterIdentityV1,
    epoch: CompletionEpoch,
    plan: M1ServingPlanV1,
    state: M1ServingPhysicalRunnerReadbackStateV1,
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
enum M1ServingPhysicalRunnerReadbackStateV1 {
    First {
        state: M1ServingFirstReadbackStateV1,
        evidence: M1ServingPhysicalRunnerReadbackEvidenceV1,
        diagnostic_history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    },
    Rearmed {
        state: M1ServingRearmedReadbackStateV1,
        evidence: M1ServingPhysicalRunnerReadbackEvidenceV1,
        diagnostic_history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    },
}

impl M1ServingPhysicalRunnerReadbackV1 {
    fn adapter_identity(&self) -> M1ServingPhysicalRunnerAdapterIdentityV1 {
        self.adapter_identity
    }

    fn epoch(&self) -> CompletionEpoch {
        self.epoch
    }

    /// Returns the exact serving plan bound to this checked readback.
    #[must_use]
    pub const fn plan(&self) -> M1ServingPlanV1 {
        self.plan
    }

    fn first(
        adapter_identity: M1ServingPhysicalRunnerAdapterIdentityV1,
        epoch: CompletionEpoch,
        plan: M1ServingPlanV1,
        state: M1ServingFirstReadbackStateV1,
        evidence: M1ServingPhysicalRunnerReadbackEvidenceV1,
        diagnostic_history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    ) -> Self {
        Self {
            adapter_identity,
            epoch,
            plan,
            state: M1ServingPhysicalRunnerReadbackStateV1::First {
                state,
                evidence,
                diagnostic_history,
            },
        }
    }

    fn rearmed(
        adapter_identity: M1ServingPhysicalRunnerAdapterIdentityV1,
        epoch: CompletionEpoch,
        plan: M1ServingPlanV1,
        state: M1ServingRearmedReadbackStateV1,
        evidence: M1ServingPhysicalRunnerReadbackEvidenceV1,
        diagnostic_history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    ) -> Self {
        Self {
            adapter_identity,
            epoch,
            plan,
            state: M1ServingPhysicalRunnerReadbackStateV1::Rearmed {
                state,
                evidence,
                diagnostic_history,
            },
        }
    }
}

/// Failure to allocate a unique, process-local physical-adapter identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1ServingPhysicalRunnerOperationsCreateErrorV1 {
    AdapterIdentityExhausted,
}

/// Stable reason a dynamic queued generation is unavailable at one readback.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1 {
    /// The adapter already transferred or sealed its concrete provider.
    ProviderUnavailable,
    /// Another queued generation would run before the output-dependent input.
    ProviderQueueNotEmpty,
    /// Readback belongs to a different process-local adapter instance.
    CustodyIdentityMismatch,
    /// The adapter does not currently own this exact unsettled readback epoch.
    ReadbackPhaseMismatch,
    /// The adapter does not currently own this exact committed quiescent epoch.
    QuiescentPhaseMismatch,
    /// Source shape or successor phase/shape is outside the closed transition.
    UnsupportedTransition,
    /// Successor epoch or ordered request roster drifted from readback.
    BindingMismatch,
    /// Draft or target successor input did not start from the checked token.
    AnchorMismatch,
    /// Committed coordinator outcome drifted from settled physical custody.
    CommitOutcomeMismatch,
    /// Successor role inputs drifted from committed anchors or KV cursors.
    CommittedInputMismatch,
}

/// Exhaustive failure to append one checked S1/K4 successor generation.
///
/// Both variants retain the exact rejected input. `Unavailable` means no
/// provider queue mutation was attempted; `Provider` preserves the lower
/// allocation diagnostic after a failed transactional enqueue.
#[must_use = "failed dynamic enqueue retains the exact generation input"]
#[derive(Debug)]
pub enum M1ServingPhysicalRunnerGenerationEnqueueFailureV1 {
    /// Adapter/readback validation rejected before provider queue mutation.
    Unavailable {
        /// Stable reason enqueue was unavailable.
        source: M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1,
        /// Exact unchanged generation input.
        input: Box<M1ServingQueuedS1K4RolloverV1>,
    },
    /// Transactional provider queue growth failed with lower custody intact.
    Provider {
        /// Lower host queue-growth diagnostic.
        source: std::collections::TryReserveError,
        /// Exact unchanged generation input.
        input: Box<M1ServingQueuedS1K4RolloverV1>,
    },
}

impl M1ServingPhysicalRunnerGenerationEnqueueFailureV1 {
    /// Stable unavailable reason, or `None` for a lower provider failure.
    #[must_use]
    pub const fn unavailable(
        &self,
    ) -> Option<M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1> {
        match self {
            Self::Unavailable { source, .. } => Some(*source),
            Self::Provider { .. } => None,
        }
    }

    /// Lower queue-growth failure, or `None` when enqueue was unavailable.
    #[must_use]
    pub const fn provider_failure(&self) -> Option<&std::collections::TryReserveError> {
        match self {
            Self::Unavailable { .. } => None,
            Self::Provider { source, .. } => Some(source),
        }
    }

    /// Borrows the unchanged generation input retained on every failure path.
    #[must_use = "the rejected generation input remains linear"]
    pub const fn input(&self) -> &M1ServingQueuedS1K4RolloverV1 {
        match self {
            Self::Unavailable { input, .. } | Self::Provider { input, .. } => input,
        }
    }

    /// Recovers the unchanged pre-boxed input from either failure class.
    #[must_use = "the rejected generation input remains linear"]
    pub fn into_input(self) -> Box<M1ServingQueuedS1K4RolloverV1> {
        match self {
            Self::Unavailable { input, .. } | Self::Provider { input, .. } => input,
        }
    }
}

/// Exhaustive failure to append one committed same-shape S1/K4 rearm.
///
/// Both variants retain the exact pre-boxed input. `Unavailable` means no
/// provider queue mutation was attempted; `Provider` preserves the lower
/// allocation diagnostic after a failed transactional enqueue.
#[must_use = "failed dynamic enqueue retains the exact generation input"]
#[derive(Debug)]
pub enum M1ServingPhysicalRunnerS1K4RearmEnqueueFailureV1 {
    /// Adapter, custody, coordinator, or input validation rejected before mutation.
    Unavailable {
        /// Stable reason enqueue was unavailable.
        source: M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1,
        /// Exact unchanged same-shape generation input.
        input: Box<M1ServingQueuedSameShapeRearmV1>,
    },
    /// Transactional provider queue growth failed with lower custody intact.
    Provider {
        /// Lower host queue-growth diagnostic.
        source: std::collections::TryReserveError,
        /// Exact unchanged same-shape generation input.
        input: Box<M1ServingQueuedSameShapeRearmV1>,
    },
}

impl M1ServingPhysicalRunnerS1K4RearmEnqueueFailureV1 {
    /// Stable unavailable reason, or `None` for a lower provider failure.
    #[must_use]
    pub const fn unavailable(
        &self,
    ) -> Option<M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1> {
        match self {
            Self::Unavailable { source, .. } => Some(*source),
            Self::Provider { .. } => None,
        }
    }

    /// Lower queue-growth failure, or `None` when enqueue was unavailable.
    #[must_use]
    pub const fn provider_failure(&self) -> Option<&std::collections::TryReserveError> {
        match self {
            Self::Unavailable { .. } => None,
            Self::Provider { source, .. } => Some(source),
        }
    }

    /// Borrows the unchanged same-shape input retained on every failure path.
    #[must_use = "the rejected generation input remains linear"]
    pub const fn input(&self) -> &M1ServingQueuedSameShapeRearmV1 {
        match self {
            Self::Unavailable { input, .. } | Self::Provider { input, .. } => input,
        }
    }

    /// Recovers the unchanged same-shape input from either failure class.
    #[must_use = "the rejected generation input remains linear"]
    pub fn into_input(self) -> Box<M1ServingQueuedSameShapeRearmV1> {
        match self {
            Self::Unavailable { input, .. } | Self::Provider { input, .. } => input,
        }
    }
}

/// Stable stage reported through the generic serving bridge.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1ServingPhysicalRunnerOperationErrorV1 {
    UnsupportedEvidenceShape,
    PlanMismatch,
    ExactFirstDispatch(M1ExactDispatchErrorV1),
    ProviderPreparation,
    SelectedRosterCount,
    FirstPublication,
    SameShapeSchedule,
    SameShapePublication,
    RolloverSchedule,
    RolloverPublication,
    RolloverUnavailable,
    EpochMismatch,
    QueueWait,
    QueueRecycle,
    CompletionReadback,
    DiagnosticReadback,
    DiagnosticHistoryCapacity,
    DispositionCount,
    DispositionDrift,
    CompletionPreflightCapacity,
    CompletionRejected,
    CompletionPoisoned,
    PageReleaseRejected,
    AdapterSealed,
    PhaseMismatch,
    CustodyIdentityMismatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum M1ServingPhysicalRunnerAdapterPhaseV1 {
    InitialReady,
    Published { epoch: CompletionEpoch },
    Readback { epoch: CompletionEpoch },
    Quiescent { epoch: CompletionEpoch },
    Sealed,
}

/// Exhaustive phase-accurate lower custody after the serving adapter seals.
#[must_use = "terminal lower custody must remain retained for teardown or diagnosis"]
#[derive(Debug)]
pub enum M1ServingPhysicalRunnerTerminalLowerCustodyV1<'a, F> {
    AdapterSealedVacant,
    AdapterSealedQuiescent(Box<M1ServingPhysicalRunnerQuiescentV1>),
    AdapterSealedPublished(Box<M1ServingPhysicalRunnerPublishedV1>),
    AdapterSealedReadback(Box<M1ServingPhysicalRunnerReadbackV1>),
    ExactFirstDispatch(M1ExactDispatchErrorV1),
    FirstProviderPreparation(Box<F>),
    FirstPreparedRosterRejected(Box<M1ServingPreparedFirstPublicationV1>),
    FirstPreparedEvidenceRejected(Box<M1ServingPreparedFirstPublicationV1>),
    FirstPublication {
        failure: crate::M1PhysicalRunnerFirstPublicationFailureV1<'a>,
        selected: Vec<ActiveDeviceKvCache>,
        semantic_evidence: M1ServingPreparedSemanticEvidenceV1,
    },
    FirstUnexpectedShape {
        published: M1PhysicalPublishedQueueSessionV1,
        selected: Vec<ActiveDeviceKvCache>,
        semantic_evidence: M1ServingPreparedSemanticEvidenceV1,
    },
    RearmSchedule {
        failure: crate::M1LongLivedQueueRearmScheduleFailureV1,
        history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    },
    RearmProviderPreparation {
        failure: Box<F>,
        history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    },
    RearmPreparedInputRejected {
        prepared: Box<M1ServingPreparedSameShapeRearmV1>,
        history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    },
    RearmPublication {
        failure: crate::M1PhysicalRunnerRearmSubmissionFailureV1<'a>,
        semantic_evidence: M1ServingPreparedSemanticEvidenceV1,
        history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    },
    RearmUnexpectedShape {
        published: Box<M1RearmedPublishedQueueV1>,
        semantic_evidence: M1ServingPreparedSemanticEvidenceV1,
        history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    },
    RolloverSchedule {
        failure: Box<crate::M1S1K4QueueRolloverScheduleFailureV1>,
        history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    },
    RolloverProviderPreparation {
        failure: Box<F>,
        history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    },
    RolloverPreparedInputRejected {
        prepared: Box<M1ServingPreparedS1K4RolloverV1>,
        history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    },
    RolloverPublication {
        failure: crate::M1PhysicalRunnerS1K4RolloverSubmissionFailureV1<'a>,
        semantic_evidence: M1ServingPreparedSemanticEvidenceV1,
        history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    },
    RolloverUnexpectedShape {
        published: Box<M1RearmedPublishedQueueV1>,
        semantic_evidence: M1ServingPreparedSemanticEvidenceV1,
        history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    },
    FirstQueueWait {
        failure: crate::M1PhysicalQueueOperationFailureV1,
        selected: Vec<ActiveDeviceKvCache>,
        semantic_evidence: M1ServingPreparedSemanticEvidenceV1,
        history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    },
    FirstQueueRecycle {
        failure: crate::M1PhysicalQueueOperationFailureV1,
        selected: Vec<ActiveDeviceKvCache>,
        semantic_evidence: M1ServingPreparedSemanticEvidenceV1,
        history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    },
    FirstCompactObservation {
        failure: crate::M1CompletionObservationFailureV1,
        selected: Vec<ActiveDeviceKvCache>,
        semantic_evidence: M1ServingPreparedSemanticEvidenceV1,
        history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    },
    FirstChoiceObservation {
        failure: Box<crate::M1SpeculativeDiagnosticObservationFailureV1>,
        selected: Vec<ActiveDeviceKvCache>,
        semantic_evidence: M1ServingPreparedSemanticEvidenceV1,
        history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    },
    FirstDiagnosticJoin {
        failure: Box<crate::M1SpeculativeDiagnosticCompletedReadbackJoinFailureV1>,
        selected: Vec<ActiveDeviceKvCache>,
        semantic_evidence: M1ServingPreparedSemanticEvidenceV1,
        history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    },
    FirstDirectObservation {
        failure: Box<crate::M1DirectDiagnosticObservationFailureV1>,
        selected: Vec<ActiveDeviceKvCache>,
        semantic_evidence: M1ServingPreparedSemanticEvidenceV1,
        history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    },
    FirstDirectJoin {
        failure: Box<crate::M1DirectDiagnosticCompletedReadbackJoinFailureV1>,
        selected: Vec<ActiveDeviceKvCache>,
        semantic_evidence: M1ServingPreparedSemanticEvidenceV1,
        history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    },
    RearmedQueueProgress {
        failure: Box<crate::M1RearmedQueueProgressFailureV1>,
        semantic_evidence: M1ServingPreparedSemanticEvidenceV1,
        history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    },
    RearmedDiagnosticReadback {
        failure: Box<crate::M1RearmedSpeculativeDiagnosticReadbackFailureV1>,
        semantic_evidence: M1ServingPreparedSemanticEvidenceV1,
        history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    },
    RearmedDirectReadback {
        failure: Box<crate::M1RearmedDirectDiagnosticReadbackFailureV1>,
        semantic_evidence: M1ServingPreparedSemanticEvidenceV1,
        history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    },
    FirstPageRelease {
        failure: Box<crate::M1CompletedStepKvReleaseFailureV1>,
        evidence: M1ServingPhysicalRunnerReadbackEvidenceV1,
        history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    },
    FirstCompletionPoison {
        poison: Box<crate::M1CompletedStepPoisonV1>,
        evidence: M1ServingPhysicalRunnerReadbackEvidenceV1,
        history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    },
    RearmedCompletionTerminal {
        outcome: Box<M1RearmedCompletionOutcomeV1>,
        evidence: M1ServingPhysicalRunnerReadbackEvidenceV1,
        history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    },
    RearmedPageRelease {
        failure: Box<crate::M1RearmedRoundPageReleaseFailureV1>,
        evidence: M1ServingPhysicalRunnerReadbackEvidenceV1,
        history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    },
}

impl<F> M1ServingPhysicalRunnerTerminalLowerCustodyV1<'_, F> {
    #[must_use]
    pub const fn stage(&self) -> M1ServingPhysicalRunnerOperationErrorV1 {
        match self {
            Self::AdapterSealedVacant
            | Self::AdapterSealedQuiescent(_)
            | Self::AdapterSealedPublished(_)
            | Self::AdapterSealedReadback(_) => {
                M1ServingPhysicalRunnerOperationErrorV1::AdapterSealed
            }
            Self::ExactFirstDispatch(error) => {
                M1ServingPhysicalRunnerOperationErrorV1::ExactFirstDispatch(*error)
            }
            Self::FirstProviderPreparation(_)
            | Self::RearmProviderPreparation { .. }
            | Self::RolloverProviderPreparation { .. } => {
                M1ServingPhysicalRunnerOperationErrorV1::ProviderPreparation
            }
            Self::FirstPreparedRosterRejected(_) => {
                M1ServingPhysicalRunnerOperationErrorV1::SelectedRosterCount
            }
            Self::FirstPreparedEvidenceRejected(_)
            | Self::RearmPreparedInputRejected { .. }
            | Self::RolloverPreparedInputRejected { .. }
            | Self::FirstUnexpectedShape { .. }
            | Self::RearmUnexpectedShape { .. }
            | Self::RolloverUnexpectedShape { .. } => {
                M1ServingPhysicalRunnerOperationErrorV1::UnsupportedEvidenceShape
            }
            Self::FirstPublication { .. } => {
                M1ServingPhysicalRunnerOperationErrorV1::FirstPublication
            }
            Self::RearmSchedule { .. } => {
                M1ServingPhysicalRunnerOperationErrorV1::SameShapeSchedule
            }
            Self::RearmPublication { .. } => {
                M1ServingPhysicalRunnerOperationErrorV1::SameShapePublication
            }
            Self::RolloverSchedule { .. } => {
                M1ServingPhysicalRunnerOperationErrorV1::RolloverSchedule
            }
            Self::RolloverPublication { .. } => {
                M1ServingPhysicalRunnerOperationErrorV1::RolloverPublication
            }
            Self::FirstQueueWait { .. } => M1ServingPhysicalRunnerOperationErrorV1::QueueWait,
            Self::FirstQueueRecycle { .. } => M1ServingPhysicalRunnerOperationErrorV1::QueueRecycle,
            Self::FirstCompactObservation { .. }
            | Self::FirstDirectObservation { .. }
            | Self::FirstDirectJoin { .. }
            | Self::RearmedDirectReadback { .. } => {
                M1ServingPhysicalRunnerOperationErrorV1::CompletionReadback
            }
            Self::FirstChoiceObservation { .. }
            | Self::FirstDiagnosticJoin { .. }
            | Self::RearmedDiagnosticReadback { .. } => {
                M1ServingPhysicalRunnerOperationErrorV1::DiagnosticReadback
            }
            Self::RearmedQueueProgress { failure, .. } => match failure.phase() {
                crate::M1LongLivedQueueRearmProgressPhaseV1::QueueWait => {
                    M1ServingPhysicalRunnerOperationErrorV1::QueueWait
                }
                crate::M1LongLivedQueueRearmProgressPhaseV1::SignalRecycle => {
                    M1ServingPhysicalRunnerOperationErrorV1::QueueRecycle
                }
            },
            Self::FirstPageRelease { .. } | Self::RearmedPageRelease { .. } => {
                M1ServingPhysicalRunnerOperationErrorV1::PageReleaseRejected
            }
            Self::FirstCompletionPoison { .. } | Self::RearmedCompletionTerminal { .. } => {
                M1ServingPhysicalRunnerOperationErrorV1::CompletionPoisoned
            }
        }
    }
}

/// Opaque exhaustive terminal custody retaining the provider and typed lower owner.
///
/// ```compile_fail
/// use ferric_engine::M1ServingPhysicalRunnerTerminalCustodyV1;
/// fn split(custody: M1ServingPhysicalRunnerTerminalCustodyV1<'static, (), ()>) {
///     let M1ServingPhysicalRunnerTerminalCustodyV1 { provider, lower } = custody;
///     let _ = (provider, lower);
/// }
/// ```
#[must_use = "terminal physical custody must remain retained for teardown or diagnosis"]
pub struct M1ServingPhysicalRunnerTerminalCustodyV1<'a, P, F> {
    provider: Option<P>,
    plan: Option<M1ServingPlanV1>,
    lower: Box<M1ServingPhysicalRunnerTerminalLowerCustodyV1<'a, F>>,
}

impl<P, F: fmt::Debug> fmt::Debug for M1ServingPhysicalRunnerTerminalCustodyV1<'_, P, F> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1ServingPhysicalRunnerTerminalCustodyV1")
            .field("stage", &self.stage())
            .field("provider_retained", &self.provider.is_some())
            .field("plan", &self.plan)
            .field("lower", &self.lower)
            .finish()
    }
}

impl<'a, P, F> M1ServingPhysicalRunnerTerminalCustodyV1<'a, P, F> {
    #[must_use]
    pub fn stage(&self) -> M1ServingPhysicalRunnerOperationErrorV1 {
        self.lower.stage()
    }

    #[must_use]
    pub const fn provider(&self) -> Option<&P> {
        self.provider.as_ref()
    }

    /// Returns the exact serving plan retained when a batch had been selected.
    #[must_use]
    pub const fn plan(&self) -> Option<M1ServingPlanV1> {
        self.plan
    }

    pub fn lower(&self) -> &M1ServingPhysicalRunnerTerminalLowerCustodyV1<'a, F> {
        &self.lower
    }

    /// Separates the provider from exact typed lower custody without dropping either owner.
    #[must_use = "terminal provider and lower custody must both remain retained"]
    pub fn into_parts(
        self,
    ) -> (
        Option<P>,
        Option<M1ServingPlanV1>,
        Box<M1ServingPhysicalRunnerTerminalLowerCustodyV1<'a, F>>,
    ) {
        (self.provider, self.plan, self.lower)
    }
}

/// Production adapter over the admitted physical runner and one live Engine.
pub struct M1ServingPhysicalRunnerOperationsV1<'a, const C: usize, P> {
    runner: &'a M1PhysicalRunnerV1,
    engine: &'a mut Engine<C>,
    provider: Option<P>,
    ring_bytes: u32,
    identity: M1ServingPhysicalRunnerAdapterIdentityV1,
    phase: M1ServingPhysicalRunnerAdapterPhaseV1,
    active_plan: Option<M1ServingPlanV1>,
}

impl<'a, const C: usize, P> M1ServingPhysicalRunnerOperationsV1<'a, C, P> {
    /// Creates a physical adapter with a unique process-local custody identity.
    ///
    /// # Errors
    ///
    /// Returns an error after exhausting the non-repeating adapter identity
    /// space; no runner, engine, or provider transition occurs in that case.
    pub fn new(
        runner: &'a M1PhysicalRunnerV1,
        engine: &'a mut Engine<C>,
        provider: P,
        ring_bytes: u32,
    ) -> Result<Self, M1ServingPhysicalRunnerOperationsCreateErrorV1> {
        let identity = M1ServingPhysicalRunnerAdapterIdentityV1::fresh()
            .ok_or(M1ServingPhysicalRunnerOperationsCreateErrorV1::AdapterIdentityExhausted)?;
        Ok(Self {
            runner,
            engine,
            provider: Some(provider),
            ring_bytes,
            identity,
            phase: M1ServingPhysicalRunnerAdapterPhaseV1::InitialReady,
            active_plan: None,
        })
    }

    #[must_use]
    pub const fn provider(&self) -> Option<&P> {
        self.provider.as_ref()
    }

    fn terminal<Q, F>(
        &mut self,
        lower: M1ServingPhysicalRunnerTerminalLowerCustodyV1<'a, F>,
    ) -> M1ServingPhysicalOperationFailureV1<
        Q,
        M1ServingPhysicalRunnerTerminalCustodyV1<'a, P, F>,
        M1ServingPhysicalRunnerOperationErrorV1,
    >
    where
        F: fmt::Debug + 'a,
    {
        self.engine.quarantine_m1_queue_rearm_failure();
        self.phase = M1ServingPhysicalRunnerAdapterPhaseV1::Sealed;
        let source = lower.stage();
        M1ServingPhysicalOperationFailureV1::Terminal {
            source,
            custody: M1ServingPhysicalRunnerTerminalCustodyV1 {
                provider: self.provider.take(),
                plan: self.active_plan,
                lower: Box::new(lower),
            },
        }
    }
}

impl<const C: usize>
    M1ServingPhysicalRunnerOperationsV1<'_, C, M1QueuedServingPhysicalInputProviderV1>
{
    /// Appends an exact S1/K4 successor after paired-prefill readback.
    ///
    /// This capability is deliberately available only for the concrete queued
    /// provider. It validates the readback custody, exact next epoch and
    /// request roster, and closed S1-prefill to S1/K4 transition before the
    /// provider queue can change.
    ///
    /// # Errors
    ///
    /// Returns the pre-boxed input unchanged when the adapter/readback cannot
    /// accept a successor or when transactional provider queue growth fails.
    pub fn try_enqueue_s1_k4_rollover_after_readback(
        &mut self,
        readback: &M1ServingPhysicalRunnerReadbackV1,
        input: Box<M1ServingQueuedS1K4RolloverV1>,
    ) -> Result<(), M1ServingPhysicalRunnerGenerationEnqueueFailureV1> {
        let checked = self.checked_completion(readback);
        if let Err(source) = validate_generation_enqueue(
            self.provider.is_some(),
            self.provider.as_ref().map_or(
                0,
                M1QueuedServingPhysicalInputProviderV1::pending_generation_count,
            ),
            self.identity,
            self.phase,
            readback.adapter_identity(),
            readback.epoch(),
            readback.plan(),
            input.binding(),
            checked
                .records()
                .iter()
                .map(|record| record.record().request),
        ) {
            return Err(
                M1ServingPhysicalRunnerGenerationEnqueueFailureV1::Unavailable { source, input },
            );
        }
        let [record] = checked.records() else {
            return Err(
                M1ServingPhysicalRunnerGenerationEnqueueFailureV1::Unavailable {
                    source:
                        M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1::UnsupportedTransition,
                    input,
                },
            );
        };
        if let Err(source) = validate_s1_k4_rollover_anchor(&input, record.semantics()) {
            return Err(
                M1ServingPhysicalRunnerGenerationEnqueueFailureV1::Unavailable { source, input },
            );
        }
        let Some(provider) = self.provider.as_mut() else {
            return Err(
                M1ServingPhysicalRunnerGenerationEnqueueFailureV1::Unavailable {
                    source:
                        M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1::ProviderUnavailable,
                    input,
                },
            );
        };
        provider
            .try_enqueue_s1_k4_rollover(input)
            .map_err(|(source, input)| {
                M1ServingPhysicalRunnerGenerationEnqueueFailureV1::Provider { source, input }
            })
    }

    /// Appends one same-shape S1/K4 successor after atomic speculative commit.
    ///
    /// The borrowed outcome is coordinator authority produced only after
    /// physical settlement and registry commit. The provider queue changes
    /// only after exact custody, epoch, roster, anchor, and committed-role
    /// cursor validation.
    ///
    /// # Errors
    ///
    /// Returns the pre-boxed input unchanged when the adapter, committed
    /// outcome, or role inputs drift, or when provider queue growth fails.
    pub fn try_enqueue_s1_k4_rearm_after_commit(
        &mut self,
        committed: &M1ServingCommittedSpeculativeRoundV1<M1ServingPhysicalRunnerQuiescentV1>,
        input: Box<M1ServingQueuedSameShapeRearmV1>,
    ) -> Result<(), M1ServingPhysicalRunnerS1K4RearmEnqueueFailureV1> {
        let quiescent = committed.quiescent();
        let outcome = committed.outcome();
        if let Err(source) = validate_s1_k4_rearm_enqueue(
            self.provider.is_some(),
            self.provider.as_ref().map_or(
                0,
                M1QueuedServingPhysicalInputProviderV1::pending_generation_count,
            ),
            self.identity,
            self.phase,
            self.active_plan,
            quiescent.adapter_identity(),
            quiescent.epoch(),
            committed.plan(),
            outcome.selection(),
            outcome.completed_epoch(),
            outcome.next_active_roster(),
            input.binding(),
        ) {
            return Err(
                M1ServingPhysicalRunnerS1K4RearmEnqueueFailureV1::Unavailable { source, input },
            );
        }
        let authorities = outcome
            .members()
            .iter()
            .copied()
            .filter(|member| member.status() == M1SpeculativeMemberStatusV1::Active)
            .map(|member| M1CommittedS1K4RearmMemberAuthorityV1 {
                request: member.request(),
                anchor: member.next_draft_anchor(),
                target_committed: member.target_settlement().commit_end(),
                draft_committed: member.draft_settlement().commit_end(),
            });
        if let Err(source) =
            validate_committed_s1_k4_rearm_input(&input, outcome.next_active_roster(), authorities)
        {
            return Err(
                M1ServingPhysicalRunnerS1K4RearmEnqueueFailureV1::Unavailable { source, input },
            );
        }
        let Some(provider) = self.provider.as_mut() else {
            return Err(
                M1ServingPhysicalRunnerS1K4RearmEnqueueFailureV1::Unavailable {
                    source:
                        M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1::ProviderUnavailable,
                    input,
                },
            );
        };
        provider
            .try_enqueue_s1_k4_rearm(input)
            .map_err(
                |(source, input)| M1ServingPhysicalRunnerS1K4RearmEnqueueFailureV1::Provider {
                    source,
                    input,
                },
            )
    }
}

impl<const C: usize, P> Drop for M1ServingPhysicalRunnerOperationsV1<'_, C, P> {
    fn drop(&mut self) {
        if !matches!(
            self.phase,
            M1ServingPhysicalRunnerAdapterPhaseV1::InitialReady
                | M1ServingPhysicalRunnerAdapterPhaseV1::Sealed
        ) {
            self.engine.quarantine_m1_queue_rearm_failure();
            self.phase = M1ServingPhysicalRunnerAdapterPhaseV1::Sealed;
        }
    }
}

fn supports_evidence_bound_s1_k4(plan: M1ServingPlanV1) -> bool {
    plan.shape() == M1PhysicalFixedBatchShapeV1::SpeculativeK4
        && plan.sequence_capacity() == 1
        && plan.target().bucket == Qwen3PlanBucket::SpeculativeS1K4C8192
}

fn supports_s1_paired_prefill_rollover_source(plan: M1ServingPlanV1) -> bool {
    plan.shape() == M1PhysicalFixedBatchShapeV1::PairedPrefill
        && plan.sequence_capacity() == 1
        && plan.target().mode == ferric_spec::Qwen3ExecutionMode::Prefill
        && plan.target().bucket == Qwen3PlanBucket::PrefillS1T128
}

fn validate_s1_k4_rollover_anchor(
    input: &M1ServingQueuedS1K4RolloverV1,
    semantics: crate::CheckedCompletionSemantics,
) -> Result<(), M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1> {
    let crate::CheckedCompletionSemantics::DirectFinalRow { token } = semantics else {
        return Err(M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1::UnsupportedTransition);
    };
    if !input.matches_anchor(token) {
        return Err(M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1::AnchorMismatch);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct M1CommittedS1K4RearmMemberAuthorityV1 {
    request: RequestId,
    anchor: Option<ferric_spec::TokenId>,
    target_committed: u32,
    draft_committed: u32,
}

#[allow(clippy::too_many_arguments)]
fn validate_s1_k4_rearm_enqueue(
    provider_available: bool,
    pending_generation_count: usize,
    expected_identity: M1ServingPhysicalRunnerAdapterIdentityV1,
    phase: M1ServingPhysicalRunnerAdapterPhaseV1,
    active_plan: Option<M1ServingPlanV1>,
    custody_identity: M1ServingPhysicalRunnerAdapterIdentityV1,
    custody_epoch: CompletionEpoch,
    custody_plan: M1ServingPlanV1,
    outcome_selection: ferric_spec::Qwen3PlanSelection,
    outcome_epoch: CompletionEpoch,
    outcome_next_roster: &[RequestId],
    binding: &M1ServingQueuedGenerationBindingV1,
) -> Result<(), M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1> {
    use M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1 as Unavailable;

    if !provider_available {
        return Err(Unavailable::ProviderUnavailable);
    }
    if pending_generation_count != 0 {
        return Err(Unavailable::ProviderQueueNotEmpty);
    }
    if expected_identity != custody_identity {
        return Err(Unavailable::CustodyIdentityMismatch);
    }
    if phase
        != (M1ServingPhysicalRunnerAdapterPhaseV1::Quiescent {
            epoch: custody_epoch,
        })
    {
        return Err(Unavailable::QuiescentPhaseMismatch);
    }
    if active_plan != Some(custody_plan) || !supports_evidence_bound_s1_k4(custody_plan) {
        return Err(Unavailable::UnsupportedTransition);
    }
    if outcome_selection != custody_plan.target() || outcome_epoch != custody_epoch {
        return Err(Unavailable::CommitOutcomeMismatch);
    }
    let Some(next_epoch) = custody_epoch.value().checked_add(1) else {
        return Err(Unavailable::BindingMismatch);
    };
    if outcome_next_roster.len() != 1
        || binding.plan() != custody_plan
        || binding.epoch() != CompletionEpoch::new(next_epoch)
        || binding.requests() != outcome_next_roster
    {
        return Err(Unavailable::BindingMismatch);
    }
    Ok(())
}

fn validate_committed_s1_k4_rearm_input(
    input: &M1ServingQueuedSameShapeRearmV1,
    expected_roster: &[RequestId],
    mut authorities: impl Iterator<Item = M1CommittedS1K4RearmMemberAuthorityV1>,
) -> Result<(), M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1> {
    use M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1 as Unavailable;

    let [request] = expected_roster else {
        return Err(Unavailable::BindingMismatch);
    };
    let Some(authority) = authorities.next() else {
        return Err(Unavailable::CommitOutcomeMismatch);
    };
    if authority.request != *request || authorities.next().is_some() {
        return Err(Unavailable::CommitOutcomeMismatch);
    }
    let Some(anchor) = authority.anchor else {
        return Err(Unavailable::CommitOutcomeMismatch);
    };
    if !input.matches_committed_s1_k4_member(
        *request,
        input.binding().epoch(),
        anchor,
        authority.target_committed,
        authority.draft_committed,
    ) {
        return Err(Unavailable::CommittedInputMismatch);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_generation_enqueue(
    provider_available: bool,
    pending_generation_count: usize,
    expected_identity: M1ServingPhysicalRunnerAdapterIdentityV1,
    phase: M1ServingPhysicalRunnerAdapterPhaseV1,
    readback_identity: M1ServingPhysicalRunnerAdapterIdentityV1,
    readback_epoch: CompletionEpoch,
    prior: M1ServingPlanV1,
    binding: &M1ServingQueuedGenerationBindingV1,
    readback_requests: impl ExactSizeIterator<Item = RequestId>,
) -> Result<(), M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1> {
    use M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1 as Unavailable;

    if !provider_available {
        return Err(Unavailable::ProviderUnavailable);
    }
    if pending_generation_count != 0 {
        return Err(Unavailable::ProviderQueueNotEmpty);
    }
    if expected_identity != readback_identity {
        return Err(Unavailable::CustodyIdentityMismatch);
    }
    if phase
        != (M1ServingPhysicalRunnerAdapterPhaseV1::Readback {
            epoch: readback_epoch,
        })
    {
        return Err(Unavailable::ReadbackPhaseMismatch);
    }
    if !supports_s1_paired_prefill_rollover_source(prior)
        || !supports_evidence_bound_s1_k4(binding.plan())
    {
        return Err(Unavailable::UnsupportedTransition);
    }
    let Some(next_epoch) = readback_epoch.value().checked_add(1) else {
        return Err(Unavailable::BindingMismatch);
    };
    if binding.epoch() != CompletionEpoch::new(next_epoch)
        || !exact_request_roster_matches(binding.requests(), readback_requests)
    {
        return Err(Unavailable::BindingMismatch);
    }
    Ok(())
}

fn supports_direct_serving(plan: M1ServingPlanV1) -> bool {
    matches!(
        plan.shape(),
        M1PhysicalFixedBatchShapeV1::PairedPrefill | M1PhysicalFixedBatchShapeV1::TargetOnly
    )
}

fn supports_serving_plan(plan: M1ServingPlanV1) -> bool {
    supports_direct_serving(plan) || supports_evidence_bound_s1_k4(plan)
}

fn supports_same_shape_rearm(plan: M1ServingPlanV1) -> bool {
    plan.shape() == M1PhysicalFixedBatchShapeV1::TargetOnly || supports_evidence_bound_s1_k4(plan)
}

fn validate_exact_serving_plan(
    expected: M1ServingPlanV1,
    actual: M1ServingPlanV1,
) -> Result<(), M1ServingPhysicalRunnerOperationErrorV1> {
    if expected == actual {
        Ok(())
    } else {
        Err(M1ServingPhysicalRunnerOperationErrorV1::PlanMismatch)
    }
}

fn completion_selection_matches(
    plan: M1ServingPlanV1,
    actual: ferric_spec::Qwen3PlanSelection,
) -> bool {
    actual == plan.target()
}

fn prepared_evidence_matches(
    plan: M1ServingPlanV1,
    expected_lanes: usize,
    completion_output: &BoundM1CompletionOutputV1,
    semantic_evidence: &M1ServingPreparedSemanticEvidenceV1,
) -> bool {
    if !completion_selection_matches(plan, completion_output.shape().selection()) {
        return false;
    }
    let diagnostic_attached = completion_output.speculative_diagnostic_choices().is_some();
    let direct_attached = completion_output.direct_diagnostic_choices().is_some();
    if !prepared_semantic_evidence_matches(plan, expected_lanes, semantic_evidence) {
        return false;
    }
    match semantic_evidence {
        M1ServingPreparedSemanticEvidenceV1::Direct => direct_attached && !diagnostic_attached,
        M1ServingPreparedSemanticEvidenceV1::SpeculativeK4 => {
            diagnostic_attached && !direct_attached
        }
    }
}

fn prepared_semantic_evidence_matches(
    plan: M1ServingPlanV1,
    expected_lanes: usize,
    semantic_evidence: &M1ServingPreparedSemanticEvidenceV1,
) -> bool {
    match semantic_evidence {
        M1ServingPreparedSemanticEvidenceV1::Direct => {
            supports_direct_serving(plan)
                && expected_lanes > 0
                && expected_lanes <= plan.sequence_capacity()
        }
        M1ServingPreparedSemanticEvidenceV1::SpeculativeK4 => {
            supports_evidence_bound_s1_k4(plan) && expected_lanes == plan.sequence_capacity()
        }
    }
}

fn phase_allows_fresh_launch(phase: M1ServingPhysicalRunnerAdapterPhaseV1) -> bool {
    phase == M1ServingPhysicalRunnerAdapterPhaseV1::InitialReady
}

fn exact_request_roster_matches(
    expected: &[RequestId],
    actual: impl ExactSizeIterator<Item = RequestId>,
) -> bool {
    actual.len() == expected.len()
        && actual
            .zip(expected.iter().copied())
            .all(|(actual, expected)| actual == expected)
}

fn diagnostic_history_can_append(len: usize) -> bool {
    len < M1_MAX_REARM_ROUND_HISTORY_V1.saturating_add(1)
}

fn dispositions_match_roster(
    dispositions: &[M1DeviceKvCompletionDispositionV1],
    roster: &M1DeviceKvCompletionRosterV1,
) -> bool {
    dispositions.len() == roster.member_count()
        && dispositions
            .iter()
            .zip(roster.members())
            .all(|(expected, actual)| *expected == actual.disposition())
}

fn schedule_failure_is_fail_stop(failure: &crate::M1LongLivedQueueRearmScheduleFailureV1) -> bool {
    rearm_schedule_error_is_fail_stop(failure.error())
}

fn rearm_schedule_error_is_fail_stop(error: crate::M1LongLivedQueueRearmScheduleErrorV1) -> bool {
    matches!(
        error,
        crate::M1LongLivedQueueRearmScheduleErrorV1::EpochExhausted
            | crate::M1LongLivedQueueRearmScheduleErrorV1::ExactScheduler(
                M1ExactDispatchErrorV1::Faulted | M1ExactDispatchErrorV1::SubmissionEpochExhausted
            )
    )
}

fn exact_dispatch_failure_is_fail_stop(error: M1ExactDispatchErrorV1) -> bool {
    matches!(
        error,
        M1ExactDispatchErrorV1::Faulted | M1ExactDispatchErrorV1::SubmissionEpochExhausted
    )
}

fn validate_adapter_identity(
    expected: M1ServingPhysicalRunnerAdapterIdentityV1,
    actual: M1ServingPhysicalRunnerAdapterIdentityV1,
) -> Result<(), M1ServingPhysicalRunnerOperationErrorV1> {
    if expected == actual {
        Ok(())
    } else {
        Err(M1ServingPhysicalRunnerOperationErrorV1::CustodyIdentityMismatch)
    }
}

fn validate_custody_guard(
    expected_identity: M1ServingPhysicalRunnerAdapterIdentityV1,
    actual_identity: M1ServingPhysicalRunnerAdapterIdentityV1,
    phase_matches: bool,
) -> Result<(), M1ServingPhysicalRunnerOperationErrorV1> {
    validate_adapter_identity(expected_identity, actual_identity)?;
    if phase_matches {
        Ok(())
    } else {
        Err(M1ServingPhysicalRunnerOperationErrorV1::PhaseMismatch)
    }
}

impl<'a, const C: usize, P> M1ServingPhysicalOperationsV1
    for M1ServingPhysicalRunnerOperationsV1<'a, C, P>
where
    P: M1ServingPhysicalInputProviderV1<C>,
    P::Failure: 'a,
{
    type Quiescent = M1ServingPhysicalRunnerQuiescentV1;
    type Published = M1ServingPhysicalRunnerPublishedV1;
    type Readback = M1ServingPhysicalRunnerReadbackV1;
    type Error = M1ServingPhysicalRunnerOperationErrorV1;
    type TerminalCustody = M1ServingPhysicalRunnerTerminalCustodyV1<'a, P, P::Failure>;

    fn scheduled_dispatch<'b>(&self, custody: &'b Self::Published) -> &'b M1ScheduledDispatchV1 {
        match &custody.state {
            M1ServingPhysicalRunnerPublishedStateV1::First { published, .. } => {
                published.scheduled_dispatch()
            }
            M1ServingPhysicalRunnerPublishedStateV1::Rearmed { published, .. } => {
                published.scheduled_dispatch()
            }
        }
    }

    fn fresh_launch(
        &mut self,
        batch: &M1ServingBatchPlanV1,
    ) -> M1ServingPhysicalOperationResultV1<Self::Published, (), Self::TerminalCustody, Self::Error>
    {
        if self.provider.is_none() {
            return Err(
                self.terminal(M1ServingPhysicalRunnerTerminalLowerCustodyV1::AdapterSealedVacant)
            );
        }
        if !phase_allows_fresh_launch(self.phase) {
            return Err(M1ServingPhysicalOperationFailureV1::Retryable {
                source: M1ServingPhysicalRunnerOperationErrorV1::PhaseMismatch,
                custody: (),
            });
        }
        if !supports_serving_plan(batch.plan()) {
            return Err(M1ServingPhysicalOperationFailureV1::Retryable {
                source: M1ServingPhysicalRunnerOperationErrorV1::UnsupportedEvidenceShape,
                custody: (),
            });
        }
        self.active_plan = Some(batch.plan());
        let scheduled = match self
            .engine
            .dispatch_m1_exact_ready(batch.epoch(), batch.requests())
        {
            Ok(scheduled) => scheduled,
            Err(error) if exact_dispatch_failure_is_fail_stop(error) => {
                return Err(self.terminal(
                    M1ServingPhysicalRunnerTerminalLowerCustodyV1::ExactFirstDispatch(error),
                ));
            }
            Err(error) => {
                return Err(M1ServingPhysicalOperationFailureV1::Retryable {
                    source: M1ServingPhysicalRunnerOperationErrorV1::ExactFirstDispatch(error),
                    custody: (),
                });
            }
        };
        let prepared = match self
            .provider
            .as_mut()
            .expect("provider presence checked before exact dispatch")
            .prepare_first_publication(self.runner, self.engine, batch, scheduled)
        {
            Ok(prepared) => prepared,
            Err(failure) => {
                return Err(self.terminal(
                    M1ServingPhysicalRunnerTerminalLowerCustodyV1::FirstProviderPreparation(
                        Box::new(failure),
                    ),
                ));
            }
        };
        if !exact_request_roster_matches(
            batch.requests(),
            prepared
                .selected
                .iter()
                .map(|cache| cache.projection().request),
        ) {
            return Err(self.terminal(
                M1ServingPhysicalRunnerTerminalLowerCustodyV1::FirstPreparedRosterRejected(
                    Box::new(prepared),
                ),
            ));
        }
        if !prepared_evidence_matches(
            batch.plan(),
            batch.requests().len(),
            &prepared.completion_output,
            &prepared.semantic_evidence,
        ) {
            return Err(self.terminal(
                M1ServingPhysicalRunnerTerminalLowerCustodyV1::FirstPreparedEvidenceRejected(
                    Box::new(prepared),
                ),
            ));
        }
        let M1ServingPreparedFirstPublicationV1 {
            allocated,
            recipe,
            completion_output,
            selected,
            semantic_evidence,
        } = prepared;
        let published = match self.runner.publish_first_step(
            self.engine,
            self.ring_bytes,
            allocated,
            recipe,
            completion_output,
        ) {
            Ok(published) => published,
            Err(failure) => {
                return Err(self.terminal(
                    M1ServingPhysicalRunnerTerminalLowerCustodyV1::FirstPublication {
                        failure,
                        selected,
                        semantic_evidence,
                    },
                ));
            }
        };
        if published.shape() != batch.plan().shape() {
            return Err(self.terminal(
                M1ServingPhysicalRunnerTerminalLowerCustodyV1::FirstUnexpectedShape {
                    published,
                    selected,
                    semantic_evidence,
                },
            ));
        }
        self.phase = M1ServingPhysicalRunnerAdapterPhaseV1::Published {
            epoch: batch.epoch(),
        };
        Ok(M1ServingPhysicalRunnerPublishedV1 {
            adapter_identity: self.identity,
            epoch: batch.epoch(),
            plan: batch.plan(),
            state: M1ServingPhysicalRunnerPublishedStateV1::First {
                published,
                selected,
                semantic_evidence,
                diagnostic_history: M1ServingPhysicalRunnerDiagnosticHistoryV1::new(),
            },
        })
    }

    fn same_shape_rearm(
        &mut self,
        custody: Self::Quiescent,
        batch: &M1ServingBatchPlanV1,
    ) -> M1ServingPhysicalOperationResultV1<
        Self::Published,
        Self::Quiescent,
        Self::TerminalCustody,
        Self::Error,
    > {
        if self.provider.is_none() {
            return Err(self.terminal(
                M1ServingPhysicalRunnerTerminalLowerCustodyV1::AdapterSealedQuiescent(Box::new(
                    custody,
                )),
            ));
        }
        if let Err(source) = validate_custody_guard(
            self.identity,
            custody.adapter_identity(),
            self.phase
                == (M1ServingPhysicalRunnerAdapterPhaseV1::Quiescent {
                    epoch: custody.epoch(),
                }),
        ) {
            return Err(M1ServingPhysicalOperationFailureV1::Retryable { source, custody });
        }
        if !supports_same_shape_rearm(batch.plan()) {
            return Err(M1ServingPhysicalOperationFailureV1::Retryable {
                source: M1ServingPhysicalRunnerOperationErrorV1::UnsupportedEvidenceShape,
                custody,
            });
        }
        if let Err(source) = validate_exact_serving_plan(custody.plan(), batch.plan()) {
            return Err(M1ServingPhysicalOperationFailureV1::Retryable { source, custody });
        }
        let M1ServingPhysicalRunnerQuiescentV1 {
            adapter_identity,
            epoch: custody_epoch,
            plan,
            state,
        } = custody;
        let (scheduled, diagnostic_history) = match state {
            M1ServingPhysicalRunnerQuiescentStateV1::First {
                released,
                diagnostic_history,
            } => {
                let scheduled = match schedule_m1_long_lived_queue_rearm_exact_v1(
                    self.engine,
                    released,
                    batch.epoch(),
                    batch.requests(),
                ) {
                    Ok(scheduled) => scheduled,
                    Err(failure) => {
                        if !failure.is_terminal() && !schedule_failure_is_fail_stop(&failure) {
                            let unscheduled =
                                failure.into_unscheduled().unwrap_or_else(|failure| {
                                    unreachable!(
                                        "released-phase schedule failure must recover: {failure:?}"
                                    )
                                });
                            return Err(M1ServingPhysicalOperationFailureV1::Retryable {
                                source: M1ServingPhysicalRunnerOperationErrorV1::SameShapeSchedule,
                                custody: M1ServingPhysicalRunnerQuiescentV1 {
                                    adapter_identity,
                                    epoch: custody_epoch,
                                    plan,
                                    state: M1ServingPhysicalRunnerQuiescentStateV1::Unscheduled {
                                        unscheduled,
                                        diagnostic_history,
                                    },
                                },
                            });
                        }
                        return Err(self.terminal(
                            M1ServingPhysicalRunnerTerminalLowerCustodyV1::RearmSchedule {
                                failure,
                                history: diagnostic_history,
                            },
                        ));
                    }
                };
                (scheduled, diagnostic_history)
            }
            M1ServingPhysicalRunnerQuiescentStateV1::Rearmed {
                released,
                diagnostic_history,
            } => {
                let scheduled = match released.schedule_next_exact(
                    self.engine,
                    batch.epoch(),
                    batch.requests(),
                ) {
                    Ok(scheduled) => scheduled,
                    Err(failure) => {
                        if !failure.is_terminal() && !schedule_failure_is_fail_stop(&failure) {
                            let unscheduled =
                                failure.into_unscheduled().unwrap_or_else(|failure| {
                                    unreachable!(
                                        "released-phase schedule failure must recover: {failure:?}"
                                    )
                                });
                            return Err(M1ServingPhysicalOperationFailureV1::Retryable {
                                source: M1ServingPhysicalRunnerOperationErrorV1::SameShapeSchedule,
                                custody: M1ServingPhysicalRunnerQuiescentV1 {
                                    adapter_identity,
                                    epoch: custody_epoch,
                                    plan,
                                    state: M1ServingPhysicalRunnerQuiescentStateV1::Unscheduled {
                                        unscheduled,
                                        diagnostic_history,
                                    },
                                },
                            });
                        }
                        return Err(self.terminal(
                            M1ServingPhysicalRunnerTerminalLowerCustodyV1::RearmSchedule {
                                failure,
                                history: diagnostic_history,
                            },
                        ));
                    }
                };
                (scheduled, diagnostic_history)
            }
            M1ServingPhysicalRunnerQuiescentStateV1::Unscheduled {
                unscheduled,
                diagnostic_history,
            } => {
                let scheduled =
                    match unscheduled.retry_exact(self.engine, batch.epoch(), batch.requests()) {
                        Ok(scheduled) => scheduled,
                        Err(failure) => {
                            if !failure.is_terminal() && !schedule_failure_is_fail_stop(&failure) {
                                let unscheduled =
                                    failure.into_unscheduled().unwrap_or_else(|failure| {
                                        unreachable!(
                                    "released-phase schedule failure must recover: {failure:?}"
                                )
                                    });
                                return Err(M1ServingPhysicalOperationFailureV1::Retryable {
                                    source:
                                        M1ServingPhysicalRunnerOperationErrorV1::SameShapeSchedule,
                                    custody: M1ServingPhysicalRunnerQuiescentV1 {
                                        adapter_identity,
                                        epoch: custody_epoch,
                                        plan,
                                        state:
                                            M1ServingPhysicalRunnerQuiescentStateV1::Unscheduled {
                                                unscheduled,
                                                diagnostic_history,
                                            },
                                    },
                                });
                            }
                            return Err(self.terminal(
                                M1ServingPhysicalRunnerTerminalLowerCustodyV1::RearmSchedule {
                                    failure,
                                    history: diagnostic_history,
                                },
                            ));
                        }
                    };
                (scheduled, diagnostic_history)
            }
        };
        let prepared = match self
            .provider
            .as_mut()
            .expect("provider presence checked before rearm scheduling")
            .prepare_same_shape_rearm(self.runner, self.engine, batch, scheduled)
        {
            Ok(prepared) => prepared,
            Err(failure) => {
                return Err(self.terminal(
                    M1ServingPhysicalRunnerTerminalLowerCustodyV1::RearmProviderPreparation {
                        failure: Box::new(failure),
                        history: diagnostic_history,
                    },
                ));
            }
        };
        if !prepared_semantic_evidence_matches(
            batch.plan(),
            batch.requests().len(),
            &prepared.semantic_evidence,
        ) {
            return Err(self.terminal(
                M1ServingPhysicalRunnerTerminalLowerCustodyV1::RearmPreparedInputRejected {
                    prepared: Box::new(prepared),
                    history: diagnostic_history,
                },
            ));
        }
        let M1ServingPreparedSameShapeRearmV1 {
            prepared,
            recipe,
            semantic_evidence,
        } = prepared;
        let published = match self.runner.submit_rearm(self.engine, prepared, recipe) {
            Ok(published) => published,
            Err(failure) => {
                return Err(self.terminal(
                    M1ServingPhysicalRunnerTerminalLowerCustodyV1::RearmPublication {
                        failure,
                        semantic_evidence,
                        history: diagnostic_history,
                    },
                ));
            }
        };
        if published.shape() != batch.plan().shape() {
            return Err(self.terminal(
                M1ServingPhysicalRunnerTerminalLowerCustodyV1::RearmUnexpectedShape {
                    published: Box::new(published),
                    semantic_evidence,
                    history: diagnostic_history,
                },
            ));
        }
        self.phase = M1ServingPhysicalRunnerAdapterPhaseV1::Published {
            epoch: batch.epoch(),
        };
        Ok(M1ServingPhysicalRunnerPublishedV1 {
            adapter_identity,
            epoch: batch.epoch(),
            plan,
            state: M1ServingPhysicalRunnerPublishedStateV1::Rearmed {
                published,
                semantic_evidence,
                diagnostic_history,
            },
        })
    }

    fn quiescent_rollover(
        &mut self,
        custody: Self::Quiescent,
        prior: M1ServingPlanV1,
        next: M1ServingPlanV1,
        reason: M1ServingRolloverReasonV1,
        batch: &M1ServingBatchPlanV1,
    ) -> M1ServingPhysicalOperationResultV1<
        Self::Published,
        Self::Quiescent,
        Self::TerminalCustody,
        Self::Error,
    > {
        if self.provider.is_none() {
            return Err(self.terminal(
                M1ServingPhysicalRunnerTerminalLowerCustodyV1::AdapterSealedQuiescent(Box::new(
                    custody,
                )),
            ));
        }
        if let Err(source) = validate_custody_guard(
            self.identity,
            custody.adapter_identity(),
            self.phase
                == (M1ServingPhysicalRunnerAdapterPhaseV1::Quiescent {
                    epoch: custody.epoch(),
                }),
        ) {
            return Err(M1ServingPhysicalOperationFailureV1::Retryable { source, custody });
        }
        if custody.plan() != prior || batch.plan() != next {
            return Err(M1ServingPhysicalOperationFailureV1::Retryable {
                source: M1ServingPhysicalRunnerOperationErrorV1::PlanMismatch,
                custody,
            });
        }
        if prior.shape() != M1PhysicalFixedBatchShapeV1::PairedPrefill
            || !supports_evidence_bound_s1_k4(next)
            || reason != M1ServingRolloverReasonV1::Mode
            || !matches!(
                &custody.state,
                M1ServingPhysicalRunnerQuiescentStateV1::First { .. }
            )
        {
            return Err(M1ServingPhysicalOperationFailureV1::Retryable {
                source: M1ServingPhysicalRunnerOperationErrorV1::RolloverUnavailable,
                custody,
            });
        }
        let M1ServingPhysicalRunnerQuiescentV1 {
            adapter_identity,
            epoch: custody_epoch,
            plan,
            state,
        } = custody;
        let M1ServingPhysicalRunnerQuiescentStateV1::First {
            released,
            diagnostic_history,
        } = state
        else {
            unreachable!("rollover state checked before consuming custody")
        };
        let scheduled =
            match schedule_m1_s1_k4_queue_rollover_exact_v1(self.engine, released, batch) {
                Ok(scheduled) => scheduled,
                Err(failure) if !failure.is_terminal() => {
                    let M1S1K4QueueRolloverScheduleFailureCustodyV1::Released(released) =
                        failure.into_custody()
                    else {
                        unreachable!("nonterminal rollover schedule retains released custody")
                    };
                    return Err(M1ServingPhysicalOperationFailureV1::Retryable {
                        source: M1ServingPhysicalRunnerOperationErrorV1::RolloverSchedule,
                        custody: M1ServingPhysicalRunnerQuiescentV1::first(
                            adapter_identity,
                            custody_epoch,
                            plan,
                            *released,
                            diagnostic_history,
                        ),
                    });
                }
                Err(failure) => {
                    return Err(self.terminal(
                        M1ServingPhysicalRunnerTerminalLowerCustodyV1::RolloverSchedule {
                            failure,
                            history: diagnostic_history,
                        },
                    ));
                }
            };
        self.active_plan = Some(next);
        let prepared = match self
            .provider
            .as_mut()
            .expect("provider presence checked before rollover scheduling")
            .prepare_s1_k4_rollover(self.runner, self.engine, batch, scheduled)
        {
            Ok(prepared) => prepared,
            Err(failure) => {
                return Err(self.terminal(
                    M1ServingPhysicalRunnerTerminalLowerCustodyV1::RolloverProviderPreparation {
                        failure: Box::new(failure),
                        history: diagnostic_history,
                    },
                ));
            }
        };
        if !matches!(
            prepared.semantic_evidence,
            M1ServingPreparedSemanticEvidenceV1::SpeculativeK4
        ) || !prepared_semantic_evidence_matches(
            batch.plan(),
            batch.requests().len(),
            &prepared.semantic_evidence,
        ) {
            return Err(self.terminal(
                M1ServingPhysicalRunnerTerminalLowerCustodyV1::RolloverPreparedInputRejected {
                    prepared: Box::new(prepared),
                    history: diagnostic_history,
                },
            ));
        }
        let M1ServingPreparedS1K4RolloverV1 {
            prepared,
            recipe,
            semantic_evidence,
        } = prepared;
        let published =
            match self
                .runner
                .submit_s1_k4_rollover(self.engine, self.ring_bytes, prepared, recipe)
            {
                Ok(published) => published,
                Err(failure) => {
                    return Err(self.terminal(
                        M1ServingPhysicalRunnerTerminalLowerCustodyV1::RolloverPublication {
                            failure,
                            semantic_evidence,
                            history: diagnostic_history,
                        },
                    ));
                }
            };
        if published.shape() != M1PhysicalFixedBatchShapeV1::SpeculativeK4
            || published.rollover_observation().is_none()
        {
            return Err(self.terminal(
                M1ServingPhysicalRunnerTerminalLowerCustodyV1::RolloverUnexpectedShape {
                    published: Box::new(published),
                    semantic_evidence,
                    history: diagnostic_history,
                },
            ));
        }
        self.phase = M1ServingPhysicalRunnerAdapterPhaseV1::Published {
            epoch: batch.epoch(),
        };
        Ok(M1ServingPhysicalRunnerPublishedV1 {
            adapter_identity,
            epoch: batch.epoch(),
            plan: next,
            state: M1ServingPhysicalRunnerPublishedStateV1::Rearmed {
                published,
                semantic_evidence,
                diagnostic_history,
            },
        })
    }

    fn read_published(
        &mut self,
        custody: Self::Published,
        epoch: CompletionEpoch,
        batch: &M1ServingBatchPlanV1,
    ) -> M1ServingPhysicalOperationResultV1<
        Self::Readback,
        Self::Published,
        Self::TerminalCustody,
        Self::Error,
    > {
        if self.provider.is_none() {
            return Err(self.terminal(
                M1ServingPhysicalRunnerTerminalLowerCustodyV1::AdapterSealedPublished(Box::new(
                    custody,
                )),
            ));
        }
        if let Err(source) = validate_custody_guard(
            self.identity,
            custody.adapter_identity(),
            self.phase
                == (M1ServingPhysicalRunnerAdapterPhaseV1::Published {
                    epoch: custody.epoch(),
                }),
        ) {
            return Err(M1ServingPhysicalOperationFailureV1::Retryable { source, custody });
        }
        if epoch != custody.epoch()
            || epoch != batch.epoch()
            || !supports_serving_plan(batch.plan())
        {
            return Err(M1ServingPhysicalOperationFailureV1::Retryable {
                source: if epoch == custody.epoch() && epoch == batch.epoch() {
                    M1ServingPhysicalRunnerOperationErrorV1::UnsupportedEvidenceShape
                } else {
                    M1ServingPhysicalRunnerOperationErrorV1::EpochMismatch
                },
                custody,
            });
        }
        if let Err(source) = validate_exact_serving_plan(custody.plan(), batch.plan()) {
            return Err(M1ServingPhysicalOperationFailureV1::Retryable { source, custody });
        }
        let M1ServingPhysicalRunnerPublishedV1 {
            adapter_identity,
            epoch: custody_epoch,
            plan,
            state,
        } = custody;
        match state {
            M1ServingPhysicalRunnerPublishedStateV1::First {
                published,
                selected,
                semantic_evidence,
                diagnostic_history,
            } => {
                if published.shape() != batch.plan().shape()
                    || !prepared_semantic_evidence_matches(
                        batch.plan(),
                        batch.requests().len(),
                        &semantic_evidence,
                    )
                {
                    return Err(M1ServingPhysicalOperationFailureV1::Retryable {
                        source: M1ServingPhysicalRunnerOperationErrorV1::UnsupportedEvidenceShape,
                        custody: M1ServingPhysicalRunnerPublishedV1 {
                            adapter_identity,
                            epoch: custody_epoch,
                            plan,
                            state: M1ServingPhysicalRunnerPublishedStateV1::First {
                                published,
                                selected,
                                semantic_evidence,
                                diagnostic_history,
                            },
                        },
                    });
                }
                let completed = match published.wait() {
                    Ok(completed) => completed,
                    Err(failure) => {
                        return Err(self.terminal(
                            M1ServingPhysicalRunnerTerminalLowerCustodyV1::FirstQueueWait {
                                failure,
                                selected,
                                semantic_evidence,
                                history: diagnostic_history,
                            },
                        ));
                    }
                };
                let recycled = match completed.recycle() {
                    Ok(recycled) => recycled,
                    Err(failure) => {
                        return Err(self.terminal(
                            M1ServingPhysicalRunnerTerminalLowerCustodyV1::FirstQueueRecycle {
                                failure,
                                selected,
                                semantic_evidence,
                                history: diagnostic_history,
                            },
                        ));
                    }
                };
                let observed = match recycled.observe_completion() {
                    Ok(observed) => observed,
                    Err(failure) => {
                        return Err(self.terminal(
                            M1ServingPhysicalRunnerTerminalLowerCustodyV1::FirstCompactObservation {
                                failure,
                                selected,
                                semantic_evidence,
                                history: diagnostic_history,
                            },
                        ));
                    }
                };
                let (readback, evidence) = match (batch.plan().shape(), semantic_evidence) {
                    (
                        M1PhysicalFixedBatchShapeV1::PairedPrefill
                        | M1PhysicalFixedBatchShapeV1::TargetOnly,
                        semantic_evidence @ M1ServingPreparedSemanticEvidenceV1::Direct,
                    ) => {
                        let diagnostic = match observed.observe_direct_diagnostic_choices() {
                            Ok(diagnostic) => diagnostic,
                            Err(failure) => {
                                return Err(self.terminal(
                                    M1ServingPhysicalRunnerTerminalLowerCustodyV1::FirstDirectObservation {
                                        failure,
                                        selected,
                                        semantic_evidence,
                                        history: diagnostic_history,
                                    },
                                ));
                            }
                        };
                        let joined = match diagnostic.check_completion() {
                            Ok(joined) => joined,
                            Err(failure) => {
                                return Err(self.terminal(
                                    M1ServingPhysicalRunnerTerminalLowerCustodyV1::FirstDirectJoin {
                                        failure,
                                        selected,
                                        semantic_evidence,
                                        history: diagnostic_history,
                                    },
                                ));
                            }
                        };
                        let (readback, choices) = joined.into_parts();
                        (
                            readback,
                            M1ServingPhysicalRunnerReadbackEvidenceV1::Direct(choices),
                        )
                    }
                    (
                        M1PhysicalFixedBatchShapeV1::SpeculativeK4,
                        semantic_evidence @ M1ServingPreparedSemanticEvidenceV1::SpeculativeK4,
                    ) => {
                        let diagnostic = match observed.observe_speculative_k4_diagnostic_choices()
                        {
                            Ok(diagnostic) => diagnostic,
                            Err(failure) => {
                                return Err(self.terminal(
                                        M1ServingPhysicalRunnerTerminalLowerCustodyV1::FirstChoiceObservation {
                                            failure,
                                            selected,
                                            semantic_evidence,
                                            history: diagnostic_history,
                                        },
                                    ));
                            }
                        };
                        let joined = match diagnostic.check_completion() {
                            Ok(joined) => joined,
                            Err(failure) => {
                                return Err(self.terminal(
                                    M1ServingPhysicalRunnerTerminalLowerCustodyV1::FirstDiagnosticJoin {
                                        failure,
                                        selected,
                                        semantic_evidence,
                                        history: diagnostic_history,
                                    },
                                ));
                            }
                        };
                        let (readback, choices) = joined.into_parts();
                        (
                            readback,
                            M1ServingPhysicalRunnerReadbackEvidenceV1::SpeculativeK4(Box::new(
                                choices,
                            )),
                        )
                    }
                    _ => unreachable!("unsupported evidence shape passed readback preflight"),
                };
                self.phase = M1ServingPhysicalRunnerAdapterPhaseV1::Readback { epoch };
                Ok(M1ServingPhysicalRunnerReadbackV1 {
                    adapter_identity,
                    epoch: custody_epoch,
                    plan,
                    state: M1ServingPhysicalRunnerReadbackStateV1::First {
                        state: M1ServingFirstReadbackStateV1::Ready { readback, selected },
                        evidence,
                        diagnostic_history,
                    },
                })
            }
            M1ServingPhysicalRunnerPublishedStateV1::Rearmed {
                published,
                semantic_evidence,
                diagnostic_history,
            } => {
                if published.shape() != batch.plan().shape()
                    || !prepared_semantic_evidence_matches(
                        batch.plan(),
                        batch.requests().len(),
                        &semantic_evidence,
                    )
                {
                    return Err(M1ServingPhysicalOperationFailureV1::Retryable {
                        source: M1ServingPhysicalRunnerOperationErrorV1::UnsupportedEvidenceShape,
                        custody: M1ServingPhysicalRunnerPublishedV1 {
                            adapter_identity,
                            epoch: custody_epoch,
                            plan,
                            state: M1ServingPhysicalRunnerPublishedStateV1::Rearmed {
                                published,
                                semantic_evidence,
                                diagnostic_history,
                            },
                        },
                    });
                }
                let completed = match published.wait(self.engine) {
                    Ok(completed) => completed,
                    Err(failure) => {
                        return Err(self.terminal(
                            M1ServingPhysicalRunnerTerminalLowerCustodyV1::RearmedQueueProgress {
                                failure,
                                semantic_evidence,
                                history: diagnostic_history,
                            },
                        ));
                    }
                };
                let recycled = match completed.recycle(self.engine) {
                    Ok(recycled) => recycled,
                    Err(failure) => {
                        return Err(self.terminal(
                            M1ServingPhysicalRunnerTerminalLowerCustodyV1::RearmedQueueProgress {
                                failure,
                                semantic_evidence,
                                history: diagnostic_history,
                            },
                        ));
                    }
                };
                let (readback, evidence) = match (batch.plan().shape(), semantic_evidence) {
                    (
                        M1PhysicalFixedBatchShapeV1::TargetOnly,
                        semantic_evidence @ M1ServingPreparedSemanticEvidenceV1::Direct,
                    ) => {
                        let joined = match recycled.read_and_check_direct_diagnostic_completion() {
                            Ok(joined) => joined,
                            Err(failure) => {
                                return Err(self.terminal(
                                        M1ServingPhysicalRunnerTerminalLowerCustodyV1::RearmedDirectReadback {
                                            failure,
                                            semantic_evidence,
                                            history: diagnostic_history,
                                        },
                                    ));
                            }
                        };
                        let (readback, choices) = joined.into_parts();
                        (
                            readback,
                            M1ServingPhysicalRunnerReadbackEvidenceV1::Direct(choices),
                        )
                    }
                    (
                        M1PhysicalFixedBatchShapeV1::SpeculativeK4,
                        semantic_evidence @ M1ServingPreparedSemanticEvidenceV1::SpeculativeK4,
                    ) => {
                        let joined = match recycled
                            .read_and_check_speculative_k4_diagnostic_completion()
                        {
                            Ok(joined) => joined,
                            Err(failure) => {
                                return Err(self.terminal(
                                        M1ServingPhysicalRunnerTerminalLowerCustodyV1::RearmedDiagnosticReadback {
                                            failure,
                                            semantic_evidence,
                                            history: diagnostic_history,
                                        },
                                    ));
                            }
                        };
                        let (readback, choices) = joined.into_parts();
                        (
                            readback,
                            M1ServingPhysicalRunnerReadbackEvidenceV1::SpeculativeK4(Box::new(
                                choices,
                            )),
                        )
                    }
                    _ => unreachable!("unsupported rearm evidence passed publication preflight"),
                };
                self.phase = M1ServingPhysicalRunnerAdapterPhaseV1::Readback { epoch };
                Ok(M1ServingPhysicalRunnerReadbackV1 {
                    adapter_identity,
                    epoch: custody_epoch,
                    plan,
                    state: M1ServingPhysicalRunnerReadbackStateV1::Rearmed {
                        state: M1ServingRearmedReadbackStateV1::Ready(readback),
                        evidence,
                        diagnostic_history,
                    },
                })
            }
        }
    }

    fn checked_completion<'b>(
        &self,
        custody: &'b Self::Readback,
    ) -> &'b M1CheckedCompletionOutputV1 {
        match &custody.state {
            M1ServingPhysicalRunnerReadbackStateV1::First { state, .. } => match state {
                M1ServingFirstReadbackStateV1::Ready { readback, .. } => readback.checked(),
                M1ServingFirstReadbackStateV1::Rejected(rejected) => rejected.checked(),
            },
            M1ServingPhysicalRunnerReadbackStateV1::Rearmed { state, .. } => match state {
                M1ServingRearmedReadbackStateV1::Ready(readback) => readback.checked(),
                M1ServingRearmedReadbackStateV1::PreflightRejected(rejected) => rejected.checked(),
                M1ServingRearmedReadbackStateV1::CompletionRejected(rejected) => {
                    let M1CompletedStepOutcomeV1::Rejected(rejected) = rejected.outcome() else {
                        unreachable!("adapter retains only rejected completion outcomes here")
                    };
                    rejected.checked()
                }
            },
        }
    }

    fn settle_readback(
        &mut self,
        custody: Self::Readback,
        dispositions: Vec<M1DeviceKvCompletionDispositionV1>,
    ) -> M1ServingPhysicalOperationResultV1<
        Self::Quiescent,
        Self::Readback,
        Self::TerminalCustody,
        Self::Error,
    > {
        if self.provider.is_none() {
            return Err(self.terminal(
                M1ServingPhysicalRunnerTerminalLowerCustodyV1::AdapterSealedReadback(Box::new(
                    custody,
                )),
            ));
        }
        let readback_epoch = custody.epoch();
        if self.checked_completion(&custody).epoch() != readback_epoch {
            return Err(M1ServingPhysicalOperationFailureV1::Retryable {
                source: M1ServingPhysicalRunnerOperationErrorV1::EpochMismatch,
                custody,
            });
        }
        if let Err(source) = validate_custody_guard(
            self.identity,
            custody.adapter_identity(),
            self.phase
                == M1ServingPhysicalRunnerAdapterPhaseV1::Readback {
                    epoch: readback_epoch,
                },
        ) {
            return Err(M1ServingPhysicalOperationFailureV1::Retryable { source, custody });
        }
        let M1ServingPhysicalRunnerReadbackV1 {
            adapter_identity,
            epoch: _,
            plan,
            state,
        } = custody;
        match state {
            M1ServingPhysicalRunnerReadbackStateV1::First {
                state,
                evidence,
                mut diagnostic_history,
            } => {
                if !diagnostic_history_can_append(diagnostic_history.len())
                    || diagnostic_history.try_reserve_exact(1).is_err()
                {
                    return Err(M1ServingPhysicalOperationFailureV1::Retryable {
                        source: M1ServingPhysicalRunnerOperationErrorV1::DiagnosticHistoryCapacity,
                        custody: M1ServingPhysicalRunnerReadbackV1::first(
                            adapter_identity,
                            readback_epoch,
                            plan,
                            state,
                            evidence,
                            diagnostic_history,
                        ),
                    });
                }
                let outcome = match state {
                    M1ServingFirstReadbackStateV1::Ready { readback, selected } => {
                        if dispositions.len() != selected.len() {
                            return Err(M1ServingPhysicalOperationFailureV1::Retryable {
                                source: M1ServingPhysicalRunnerOperationErrorV1::DispositionCount,
                                custody: M1ServingPhysicalRunnerReadbackV1::first(
                                    adapter_identity,
                                    readback_epoch,
                                    plan,
                                    M1ServingFirstReadbackStateV1::Ready { readback, selected },
                                    evidence,
                                    diagnostic_history,
                                ),
                            });
                        }
                        let mut members = Vec::new();
                        if members.try_reserve_exact(selected.len()).is_err() {
                            return Err(M1ServingPhysicalOperationFailureV1::Retryable {
                                source:
                                    M1ServingPhysicalRunnerOperationErrorV1::CompletionPreflightCapacity,
                                custody: M1ServingPhysicalRunnerReadbackV1::first(
                                    adapter_identity,
                                    readback_epoch,
                                    plan,
                                    M1ServingFirstReadbackStateV1::Ready {
                                        readback,
                                        selected,
                                    },
                                    evidence,
                                    diagnostic_history,
                                ),
                            });
                        }
                        for (cache, disposition) in selected.into_iter().zip(dispositions) {
                            members.push(match disposition {
                                M1DeviceKvCompletionDispositionV1::Continue => {
                                    M1DeviceKvCompletionMemberV1::continuing(cache)
                                }
                                M1DeviceKvCompletionDispositionV1::Retire => {
                                    M1DeviceKvCompletionMemberV1::retiring(cache)
                                }
                            });
                        }
                        complete_m1_physical_step_v1(
                            self.engine,
                            readback,
                            M1DeviceKvCompletionRosterV1::new(members),
                        )
                    }
                    M1ServingFirstReadbackStateV1::Rejected(rejected) => {
                        if !dispositions_match_roster(&dispositions, rejected.roster()) {
                            return Err(M1ServingPhysicalOperationFailureV1::Retryable {
                                source: M1ServingPhysicalRunnerOperationErrorV1::DispositionDrift,
                                custody: M1ServingPhysicalRunnerReadbackV1::first(
                                    adapter_identity,
                                    readback_epoch,
                                    plan,
                                    M1ServingFirstReadbackStateV1::Rejected(rejected),
                                    evidence,
                                    diagnostic_history,
                                ),
                            });
                        }
                        let (_error, readback, roster) = rejected.into_parts();
                        complete_m1_physical_step_v1(self.engine, readback, roster)
                    }
                };
                match outcome {
                    M1CompletedStepOutcomeV1::Completed(completed) => {
                        match release_m1_completed_step_kv_pages_v1(completed) {
                            Ok(released) => {
                                evidence.append_diagnostic_history(&mut diagnostic_history);
                                self.phase = M1ServingPhysicalRunnerAdapterPhaseV1::Quiescent {
                                    epoch: readback_epoch,
                                };
                                Ok(M1ServingPhysicalRunnerQuiescentV1::first(
                                    adapter_identity,
                                    readback_epoch,
                                    plan,
                                    released,
                                    diagnostic_history,
                                ))
                            }
                            Err(failure) => Err(self.terminal(
                                M1ServingPhysicalRunnerTerminalLowerCustodyV1::FirstPageRelease {
                                    failure,
                                    evidence,
                                    history: diagnostic_history,
                                },
                            )),
                        }
                    }
                    M1CompletedStepOutcomeV1::Rejected(rejected) => {
                        Err(M1ServingPhysicalOperationFailureV1::Retryable {
                            source: M1ServingPhysicalRunnerOperationErrorV1::CompletionRejected,
                            custody: M1ServingPhysicalRunnerReadbackV1::first(
                                adapter_identity,
                                readback_epoch,
                                plan,
                                M1ServingFirstReadbackStateV1::Rejected(rejected),
                                evidence,
                                diagnostic_history,
                            ),
                        })
                    }
                    M1CompletedStepOutcomeV1::Poisoned(poison) => Err(self.terminal(
                        M1ServingPhysicalRunnerTerminalLowerCustodyV1::FirstCompletionPoison {
                            poison,
                            evidence,
                            history: diagnostic_history,
                        },
                    )),
                }
            }
            M1ServingPhysicalRunnerReadbackStateV1::Rearmed {
                state,
                evidence,
                mut diagnostic_history,
            } => {
                if !diagnostic_history_can_append(diagnostic_history.len())
                    || diagnostic_history.try_reserve_exact(1).is_err()
                {
                    return Err(M1ServingPhysicalOperationFailureV1::Retryable {
                        source: M1ServingPhysicalRunnerOperationErrorV1::DiagnosticHistoryCapacity,
                        custody: M1ServingPhysicalRunnerReadbackV1::rearmed(
                            adapter_identity,
                            readback_epoch,
                            plan,
                            state,
                            evidence,
                            diagnostic_history,
                        ),
                    });
                }
                let outcome = match state {
                    M1ServingRearmedReadbackStateV1::Ready(readback) => {
                        match readback.complete(self.engine, dispositions) {
                            Ok(outcome) => outcome,
                            Err(failure) => {
                                return Err(M1ServingPhysicalOperationFailureV1::Retryable {
                                    source:
                                        M1ServingPhysicalRunnerOperationErrorV1::CompletionRejected,
                                    custody: M1ServingPhysicalRunnerReadbackV1::rearmed(
                                        adapter_identity,
                                        readback_epoch,
                                        plan,
                                        M1ServingRearmedReadbackStateV1::PreflightRejected(failure),
                                        evidence,
                                        diagnostic_history,
                                    ),
                                });
                            }
                        }
                    }
                    M1ServingRearmedReadbackStateV1::PreflightRejected(failure) => {
                        if failure.dispositions() != dispositions.as_slice() {
                            return Err(M1ServingPhysicalOperationFailureV1::Retryable {
                                source: M1ServingPhysicalRunnerOperationErrorV1::DispositionDrift,
                                custody: M1ServingPhysicalRunnerReadbackV1::rearmed(
                                    adapter_identity,
                                    readback_epoch,
                                    plan,
                                    M1ServingRearmedReadbackStateV1::PreflightRejected(failure),
                                    evidence,
                                    diagnostic_history,
                                ),
                            });
                        }
                        let (_error, readback, retained) = failure.into_parts();
                        match readback.complete(self.engine, retained) {
                            Ok(outcome) => outcome,
                            Err(failure) => {
                                return Err(M1ServingPhysicalOperationFailureV1::Retryable {
                                    source:
                                        M1ServingPhysicalRunnerOperationErrorV1::CompletionRejected,
                                    custody: M1ServingPhysicalRunnerReadbackV1::rearmed(
                                        adapter_identity,
                                        readback_epoch,
                                        plan,
                                        M1ServingRearmedReadbackStateV1::PreflightRejected(failure),
                                        evidence,
                                        diagnostic_history,
                                    ),
                                });
                            }
                        }
                    }
                    M1ServingRearmedReadbackStateV1::CompletionRejected(outcome) => {
                        let M1CompletedStepOutcomeV1::Rejected(rejected) = outcome.outcome() else {
                            return Err(self.terminal(
                                M1ServingPhysicalRunnerTerminalLowerCustodyV1::RearmedCompletionTerminal {
                                    outcome: Box::new(outcome),
                                    evidence,
                                    history: diagnostic_history,
                                },
                            ));
                        };
                        if !dispositions_match_roster(&dispositions, rejected.roster()) {
                            return Err(M1ServingPhysicalOperationFailureV1::Retryable {
                                source: M1ServingPhysicalRunnerOperationErrorV1::DispositionDrift,
                                custody: M1ServingPhysicalRunnerReadbackV1::rearmed(
                                    adapter_identity,
                                    readback_epoch,
                                    plan,
                                    M1ServingRearmedReadbackStateV1::CompletionRejected(outcome),
                                    evidence,
                                    diagnostic_history,
                                ),
                            });
                        }
                        match outcome.retry_rejected(self.engine) {
                            Ok(outcome) => outcome,
                            Err(outcome) => {
                                return Err(self.terminal(
                                    M1ServingPhysicalRunnerTerminalLowerCustodyV1::RearmedCompletionTerminal {
                                        outcome,
                                        evidence,
                                        history: diagnostic_history,
                                    },
                                ));
                            }
                        }
                    }
                };
                match outcome.release_completed() {
                    M1RearmedRoundReleaseOutcomeV1::Released(released) => {
                        evidence.append_diagnostic_history(&mut diagnostic_history);
                        self.phase = M1ServingPhysicalRunnerAdapterPhaseV1::Quiescent {
                            epoch: readback_epoch,
                        };
                        Ok(M1ServingPhysicalRunnerQuiescentV1::rearmed(
                            adapter_identity,
                            readback_epoch,
                            plan,
                            released,
                            diagnostic_history,
                        ))
                    }
                    M1RearmedRoundReleaseOutcomeV1::Rejected(failure) => Err(self.terminal(
                        M1ServingPhysicalRunnerTerminalLowerCustodyV1::RearmedPageRelease {
                            failure,
                            evidence,
                            history: diagnostic_history,
                        },
                    )),
                    M1RearmedRoundReleaseOutcomeV1::NotCompleted(outcome) => {
                        match outcome.outcome() {
                            M1CompletedStepOutcomeV1::Rejected(_) => {
                                Err(M1ServingPhysicalOperationFailureV1::Retryable {
                                    source:
                                        M1ServingPhysicalRunnerOperationErrorV1::CompletionRejected,
                                    custody: M1ServingPhysicalRunnerReadbackV1::rearmed(
                                        adapter_identity,
                                        readback_epoch,
                                        plan,
                                        M1ServingRearmedReadbackStateV1::CompletionRejected(
                                            outcome,
                                        ),
                                        evidence,
                                        diagnostic_history,
                                    ),
                                })
                            }
                            M1CompletedStepOutcomeV1::Completed(_)
                            | M1CompletedStepOutcomeV1::Poisoned(_) => Err(self.terminal(
                                M1ServingPhysicalRunnerTerminalLowerCustodyV1::RearmedCompletionTerminal {
                                    outcome: Box::new(outcome),
                                    evidence,
                                    history: diagnostic_history,
                                },
                            )),
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferric_spec::{Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanSelection};

    fn serving_plan(
        target_mode: Qwen3ExecutionMode,
        target_bucket: Qwen3PlanBucket,
        draft_mode: Qwen3ExecutionMode,
        draft_bucket: Qwen3PlanBucket,
    ) -> M1ServingPlanV1 {
        M1ServingPlanV1::new(
            Qwen3PlanSelection {
                role: Qwen3ModelRole::Target8B,
                mode: target_mode,
                bucket: target_bucket,
            },
            Qwen3PlanSelection {
                role: Qwen3ModelRole::Draft06B,
                mode: draft_mode,
                bucket: draft_bucket,
            },
        )
        .expect("test plan must be canonical")
    }

    fn queued_s1_k4_test_input(
        request: RequestId,
        epoch: CompletionEpoch,
        anchor: ferric_spec::TokenId,
    ) -> M1ServingQueuedS1K4RolloverV1 {
        use ferric_build::{
            m1_step_workspace_requirements, plan_addressless_m1_step_workspace,
            AvailableM1StepWorkspace, DeclaredM1StepWorkspaceAllocation,
            M1StepWorkspaceDeclaration, M1StepWorkspacePlanOutcome,
        };
        use ferric_spec::{
            validate_m1_step_inputs, Identity, M1StepInputCandidate, M1StepInputValidationOutcome,
            StepPlan, ValidatedM1StepInputs,
        };

        fn input(plan: StepPlan, tokens: Vec<u32>, positions: Vec<u32>) -> ValidatedM1StepInputs {
            let active_length = u32::try_from(tokens.len()).expect("test token count fits u32");
            let candidate = M1StepInputCandidate::new(
                plan.selection(),
                vec![Some(plan)],
                tokens,
                positions,
                vec![active_length],
                vec![128],
            );
            match validate_m1_step_inputs(candidate) {
                M1StepInputValidationOutcome::Validated(inputs) => inputs,
                M1StepInputValidationOutcome::Rejected(failure) => {
                    panic!("host-only rollover input rejected: {:?}", failure.error())
                }
            }
        }

        fn workspace(
            selection: Qwen3PlanSelection,
            identity_byte: u8,
        ) -> ferric_build::AddresslessM1StepWorkspacePlan {
            let requirements = m1_step_workspace_requirements(selection)
                .expect("canonical selection has workspace requirements");
            let available = AvailableM1StepWorkspace::new(M1StepWorkspaceDeclaration::new(
                selection,
                DeclaredM1StepWorkspaceAllocation::new(
                    Identity::new([identity_byte; 32]),
                    requirements.allocation_byte_len(),
                    requirements.allocation_alignment(),
                ),
                requirements.ranges().to_vec().into_boxed_slice(),
            ));
            match plan_addressless_m1_step_workspace(selection, available) {
                M1StepWorkspacePlanOutcome::Planned(plan) => plan,
                M1StepWorkspacePlanOutcome::Rejected(_) => panic!("test workspace rejected"),
            }
        }

        let plan = serving_plan(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        );
        let draft = input(
            StepPlan::new(request, epoch, Identity::new([41; 32]), plan.draft()),
            vec![anchor],
            vec![128],
        );
        let target = input(
            StepPlan::new(request, epoch, Identity::new([42; 32]), plan.target()),
            vec![anchor, 0, 0, 0, 0],
            (128..133).collect(),
        );
        let preparation = crate::M1FullStepWorkspacePlans::speculative_round(
            workspace(plan.draft(), 43),
            workspace(plan.target(), 44),
        );
        let recipe = crate::M1FullStepWorkspacePlans::speculative_round(
            workspace(plan.draft(), 43),
            workspace(plan.target(), 44),
        );
        M1ServingQueuedS1K4RolloverV1::new(
            M1ServingQueuedGenerationBindingV1::new(plan, vec![request].into_boxed_slice(), epoch),
            crate::M1S1K4QueueRolloverKvInputsV1::new(draft, target, Vec::new(), Vec::new()),
            preparation,
            recipe,
        )
    }

    fn queued_s1_k4_rearm_test_input(
        request: RequestId,
        epoch: CompletionEpoch,
        anchor: ferric_spec::TokenId,
        target_committed: u32,
        draft_committed: u32,
        target_future_token: ferric_spec::TokenId,
    ) -> M1ServingQueuedSameShapeRearmV1 {
        use ferric_build::{
            m1_step_workspace_requirements, plan_addressless_m1_step_workspace,
            AvailableM1StepWorkspace, DeclaredM1StepWorkspaceAllocation,
            M1StepWorkspaceDeclaration, M1StepWorkspacePlanOutcome,
        };
        use ferric_spec::{
            validate_m1_step_inputs, Identity, M1StepInputCandidate, M1StepInputValidationOutcome,
            StepPlan, ValidatedM1StepInputs,
        };

        fn input(
            plan: StepPlan,
            tokens: Vec<u32>,
            positions: Vec<u32>,
            committed: u32,
        ) -> ValidatedM1StepInputs {
            let active_length = u32::try_from(tokens.len()).expect("test token count fits u32");
            let candidate = M1StepInputCandidate::new(
                plan.selection(),
                vec![Some(plan)],
                tokens,
                positions,
                vec![active_length],
                vec![committed],
            );
            match validate_m1_step_inputs(candidate) {
                M1StepInputValidationOutcome::Validated(inputs) => inputs,
                M1StepInputValidationOutcome::Rejected(failure) => {
                    panic!(
                        "host-only S1/K4 rearm input rejected: {:?}",
                        failure.error()
                    )
                }
            }
        }

        fn workspace(
            selection: Qwen3PlanSelection,
            identity_byte: u8,
        ) -> ferric_build::AddresslessM1StepWorkspacePlan {
            let requirements = m1_step_workspace_requirements(selection)
                .expect("canonical selection has workspace requirements");
            let available = AvailableM1StepWorkspace::new(M1StepWorkspaceDeclaration::new(
                selection,
                DeclaredM1StepWorkspaceAllocation::new(
                    Identity::new([identity_byte; 32]),
                    requirements.allocation_byte_len(),
                    requirements.allocation_alignment(),
                ),
                requirements.ranges().to_vec().into_boxed_slice(),
            ));
            match plan_addressless_m1_step_workspace(selection, available) {
                M1StepWorkspacePlanOutcome::Planned(plan) => plan,
                M1StepWorkspacePlanOutcome::Rejected(_) => panic!("test workspace rejected"),
            }
        }

        let plan = serving_plan(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        );
        let draft = input(
            StepPlan::new(request, epoch, Identity::new([51; 32]), plan.draft()),
            vec![anchor],
            vec![draft_committed],
            draft_committed,
        );
        let target = input(
            StepPlan::new(request, epoch, Identity::new([52; 32]), plan.target()),
            vec![anchor, target_future_token, 0, 0, 0],
            (target_committed..target_committed + 5).collect(),
            target_committed,
        );
        let preparation = crate::M1FullStepWorkspacePlans::speculative_round(
            workspace(plan.draft(), 53),
            workspace(plan.target(), 54),
        );
        let recipe = crate::M1FullStepWorkspacePlans::speculative_round(
            workspace(plan.draft(), 53),
            workspace(plan.target(), 54),
        );
        M1ServingQueuedSameShapeRearmV1::new(
            M1ServingQueuedGenerationBindingV1::new(plan, vec![request].into_boxed_slice(), epoch),
            crate::M1LongLivedQueueRearmKvInputsV1::speculative_round(
                draft,
                target,
                vec![Vec::new()],
                vec![Vec::new()],
            ),
            preparation,
            recipe,
        )
    }

    #[test]
    fn terminal_stage_is_derived_from_typed_lower_custody() {
        let retained_plan = serving_plan(
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        );
        let terminal = M1ServingPhysicalRunnerTerminalCustodyV1 {
            provider: Some(17_u8),
            plan: Some(retained_plan),
            lower: Box::new(
                M1ServingPhysicalRunnerTerminalLowerCustodyV1::<()>::ExactFirstDispatch(
                    M1ExactDispatchErrorV1::SubmissionEpochExhausted,
                ),
            ),
        };
        assert_eq!(
            terminal.stage(),
            M1ServingPhysicalRunnerOperationErrorV1::ExactFirstDispatch(
                M1ExactDispatchErrorV1::SubmissionEpochExhausted
            )
        );
        assert_eq!(terminal.provider(), Some(&17));
        assert_eq!(terminal.plan(), Some(retained_plan));

        let (provider, plan, lower) = terminal.into_parts();
        assert_eq!(provider, Some(17));
        assert_eq!(plan, Some(retained_plan));
        assert!(matches!(
            *lower,
            M1ServingPhysicalRunnerTerminalLowerCustodyV1::ExactFirstDispatch(
                M1ExactDispatchErrorV1::SubmissionEpochExhausted
            )
        ));
    }

    #[test]
    fn terminal_vacant_stage_cannot_drift() {
        let lower =
            M1ServingPhysicalRunnerTerminalLowerCustodyV1::<'static, ()>::AdapterSealedVacant;
        assert_eq!(
            lower.stage(),
            M1ServingPhysicalRunnerOperationErrorV1::AdapterSealed
        );
    }

    #[test]
    fn adapter_identity_rejects_cross_adapter_custody() {
        let first = M1ServingPhysicalRunnerAdapterIdentityV1::fresh()
            .expect("test process must have adapter identities available");
        let second = M1ServingPhysicalRunnerAdapterIdentityV1::fresh()
            .expect("test process must have adapter identities available");
        assert_eq!(validate_custody_guard(first, first, true), Ok(()));
        assert_eq!(
            validate_custody_guard(first, second, true),
            Err(M1ServingPhysicalRunnerOperationErrorV1::CustodyIdentityMismatch)
        );
        assert_eq!(
            validate_custody_guard(second, first, true),
            Err(M1ServingPhysicalRunnerOperationErrorV1::CustodyIdentityMismatch)
        );
        assert_eq!(
            validate_custody_guard(first, first, false),
            Err(M1ServingPhysicalRunnerOperationErrorV1::PhaseMismatch)
        );
    }

    #[test]
    fn fresh_launch_phase_preflight_is_single_use() {
        assert!(phase_allows_fresh_launch(
            M1ServingPhysicalRunnerAdapterPhaseV1::InitialReady
        ));
        assert!(!phase_allows_fresh_launch(
            M1ServingPhysicalRunnerAdapterPhaseV1::Published {
                epoch: CompletionEpoch::new(1),
            }
        ));
        assert!(!phase_allows_fresh_launch(
            M1ServingPhysicalRunnerAdapterPhaseV1::Quiescent {
                epoch: CompletionEpoch::new(1),
            }
        ));
        assert!(!phase_allows_fresh_launch(
            M1ServingPhysicalRunnerAdapterPhaseV1::Sealed
        ));
    }

    #[test]
    fn rollover_admission_is_closed_to_paired_prefill_into_exact_s1_k4() {
        let prefill = serving_plan(
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T128,
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T128,
        );
        let speculative = serving_plan(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        );
        let decode = serving_plan(
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        );

        assert_eq!(prefill.shape(), M1PhysicalFixedBatchShapeV1::PairedPrefill);
        assert!(supports_evidence_bound_s1_k4(speculative));
        assert_ne!(decode.shape(), M1PhysicalFixedBatchShapeV1::PairedPrefill);
        assert!(!supports_evidence_bound_s1_k4(decode));
    }

    #[test]
    fn dynamic_enqueue_reports_provider_unavailable_before_custody_checks() {
        let identity = M1ServingPhysicalRunnerAdapterIdentityV1::fresh()
            .expect("test process has adapter identities");
        let other = M1ServingPhysicalRunnerAdapterIdentityV1::fresh()
            .expect("test process has adapter identities");
        let request = RequestId::new(7, 1);
        let speculative = serving_plan(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        );
        let binding = M1ServingQueuedGenerationBindingV1::new(
            speculative,
            vec![request].into_boxed_slice(),
            CompletionEpoch::new(2),
        );

        assert_eq!(
            validate_generation_enqueue(
                false,
                0,
                identity,
                M1ServingPhysicalRunnerAdapterPhaseV1::InitialReady,
                other,
                CompletionEpoch::new(1),
                speculative,
                &binding,
                core::iter::empty(),
            ),
            Err(M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1::ProviderUnavailable)
        );
        assert_eq!(
            validate_generation_enqueue(
                true,
                1,
                identity,
                M1ServingPhysicalRunnerAdapterPhaseV1::InitialReady,
                other,
                CompletionEpoch::new(1),
                speculative,
                &binding,
                core::iter::empty(),
            ),
            Err(M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1::ProviderQueueNotEmpty)
        );
    }

    #[test]
    fn dynamic_enqueue_requires_the_exact_unsettled_readback_phase() {
        let identity = M1ServingPhysicalRunnerAdapterIdentityV1::fresh()
            .expect("test process has adapter identities");
        let request = RequestId::new(7, 1);
        let prefill = serving_plan(
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T128,
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T128,
        );
        let speculative = serving_plan(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        );
        let epoch = CompletionEpoch::new(1);
        let binding = M1ServingQueuedGenerationBindingV1::new(
            speculative,
            vec![request].into_boxed_slice(),
            CompletionEpoch::new(2),
        );

        for phase in [
            M1ServingPhysicalRunnerAdapterPhaseV1::InitialReady,
            M1ServingPhysicalRunnerAdapterPhaseV1::Published { epoch },
            M1ServingPhysicalRunnerAdapterPhaseV1::Quiescent { epoch },
            M1ServingPhysicalRunnerAdapterPhaseV1::Sealed,
        ] {
            assert_eq!(
                validate_generation_enqueue(
                    true,
                    0,
                    identity,
                    phase,
                    identity,
                    epoch,
                    prefill,
                    &binding,
                    [request].into_iter(),
                ),
                Err(M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1::ReadbackPhaseMismatch)
            );
        }
        assert_eq!(
            validate_generation_enqueue(
                true,
                0,
                identity,
                M1ServingPhysicalRunnerAdapterPhaseV1::Readback { epoch },
                identity,
                epoch,
                prefill,
                &binding,
                [request].into_iter(),
            ),
            Ok(())
        );
    }

    #[test]
    fn dynamic_enqueue_rejects_identity_transition_epoch_and_roster_drift() {
        use M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1 as Unavailable;

        let identity = M1ServingPhysicalRunnerAdapterIdentityV1::fresh()
            .expect("test process has adapter identities");
        let other = M1ServingPhysicalRunnerAdapterIdentityV1::fresh()
            .expect("test process has adapter identities");
        let request = RequestId::new(7, 1);
        let prefill = serving_plan(
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T128,
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T128,
        );
        let speculative = serving_plan(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        );
        let epoch = CompletionEpoch::new(1);
        let exact = M1ServingQueuedGenerationBindingV1::new(
            speculative,
            vec![request].into_boxed_slice(),
            CompletionEpoch::new(2),
        );
        let stale = M1ServingQueuedGenerationBindingV1::new(
            speculative,
            vec![request].into_boxed_slice(),
            CompletionEpoch::new(3),
        );

        assert_eq!(
            validate_generation_enqueue(
                true,
                0,
                identity,
                M1ServingPhysicalRunnerAdapterPhaseV1::Readback { epoch },
                other,
                epoch,
                prefill,
                &exact,
                [request].into_iter(),
            ),
            Err(Unavailable::CustodyIdentityMismatch)
        );
        assert_eq!(
            validate_generation_enqueue(
                true,
                0,
                identity,
                M1ServingPhysicalRunnerAdapterPhaseV1::Readback { epoch },
                identity,
                epoch,
                speculative,
                &exact,
                [request].into_iter(),
            ),
            Err(Unavailable::UnsupportedTransition)
        );
        assert_eq!(
            validate_generation_enqueue(
                true,
                0,
                identity,
                M1ServingPhysicalRunnerAdapterPhaseV1::Readback { epoch },
                identity,
                epoch,
                prefill,
                &stale,
                [request].into_iter(),
            ),
            Err(Unavailable::BindingMismatch)
        );
        assert_eq!(
            validate_generation_enqueue(
                true,
                0,
                identity,
                M1ServingPhysicalRunnerAdapterPhaseV1::Readback { epoch },
                identity,
                epoch,
                prefill,
                &exact,
                [RequestId::new(8, 1)].into_iter(),
            ),
            Err(Unavailable::BindingMismatch)
        );
    }

    #[test]
    fn dynamic_enqueue_requires_the_checked_prefill_anchor() {
        use M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1 as Unavailable;

        let request = RequestId::new(7, 1);
        let epoch = CompletionEpoch::new(2);
        let exact = queued_s1_k4_test_input(request, epoch, 7);
        assert_eq!(
            validate_s1_k4_rollover_anchor(
                &exact,
                crate::CheckedCompletionSemantics::DirectFinalRow { token: 7 },
            ),
            Ok(())
        );

        let substituted = queued_s1_k4_test_input(request, epoch, 8);
        assert_eq!(
            validate_s1_k4_rollover_anchor(
                &substituted,
                crate::CheckedCompletionSemantics::DirectFinalRow { token: 7 },
            ),
            Err(Unavailable::AnchorMismatch)
        );
    }

    #[test]
    fn dynamic_enqueue_failures_retain_the_exact_generation_input() {
        let request = RequestId::new(7, 1);
        let epoch = CompletionEpoch::new(2);
        let input = queued_s1_k4_test_input(request, epoch, 7);
        let failure = M1ServingPhysicalRunnerGenerationEnqueueFailureV1::Unavailable {
            source: M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1::ReadbackPhaseMismatch,
            input: Box::new(input),
        };
        assert_eq!(
            failure.unavailable(),
            Some(M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1::ReadbackPhaseMismatch)
        );
        assert!(failure.provider_failure().is_none());
        assert_eq!(failure.input().binding().requests(), &[request],);
        let recovered = failure.into_input();
        assert_eq!(recovered.binding().epoch(), epoch);
        assert!(recovered.matches_anchor(7));

        let input = queued_s1_k4_test_input(request, epoch, 7);
        let allocation = Vec::<u8>::new()
            .try_reserve(usize::MAX)
            .expect_err("usize::MAX queue growth must fail");
        let failure = M1ServingPhysicalRunnerGenerationEnqueueFailureV1::Provider {
            source: allocation,
            input: Box::new(input),
        };
        assert_eq!(failure.unavailable(), None);
        assert!(failure.provider_failure().is_some());
        assert_eq!(failure.input().binding().requests(), &[request],);
        let recovered = failure.into_input();
        assert_eq!(recovered.binding().epoch(), epoch);
        assert!(recovered.matches_anchor(7));
    }

    #[test]
    fn dynamic_s1_k4_rearm_requires_exact_quiescent_commit_binding() {
        use M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1 as Unavailable;

        let identity = M1ServingPhysicalRunnerAdapterIdentityV1::fresh()
            .expect("test process has adapter identities");
        let other = M1ServingPhysicalRunnerAdapterIdentityV1::fresh()
            .expect("test process has adapter identities");
        let request = RequestId::new(7, 1);
        let plan = serving_plan(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        );
        let epoch = CompletionEpoch::new(2);
        let binding = M1ServingQueuedGenerationBindingV1::new(
            plan,
            vec![request].into_boxed_slice(),
            CompletionEpoch::new(3),
        );
        let validate = |provider_available, pending_generation_count, phase, custody_identity| {
            validate_s1_k4_rearm_enqueue(
                provider_available,
                pending_generation_count,
                identity,
                phase,
                Some(plan),
                custody_identity,
                epoch,
                plan,
                plan.target(),
                epoch,
                &[request],
                &binding,
            )
        };

        assert_eq!(
            validate(
                false,
                0,
                M1ServingPhysicalRunnerAdapterPhaseV1::InitialReady,
                other,
            ),
            Err(Unavailable::ProviderUnavailable)
        );
        assert_eq!(
            validate(
                true,
                1,
                M1ServingPhysicalRunnerAdapterPhaseV1::InitialReady,
                other,
            ),
            Err(Unavailable::ProviderQueueNotEmpty)
        );
        assert_eq!(
            validate(
                true,
                0,
                M1ServingPhysicalRunnerAdapterPhaseV1::Quiescent { epoch },
                other,
            ),
            Err(Unavailable::CustodyIdentityMismatch)
        );
        assert_eq!(
            validate(
                true,
                0,
                M1ServingPhysicalRunnerAdapterPhaseV1::Readback { epoch },
                identity,
            ),
            Err(Unavailable::QuiescentPhaseMismatch)
        );
        assert_eq!(
            validate(
                true,
                0,
                M1ServingPhysicalRunnerAdapterPhaseV1::Quiescent { epoch },
                identity,
            ),
            Ok(())
        );

        assert_eq!(
            validate_s1_k4_rearm_enqueue(
                true,
                0,
                identity,
                M1ServingPhysicalRunnerAdapterPhaseV1::Quiescent { epoch },
                Some(plan),
                identity,
                epoch,
                plan,
                plan.target(),
                CompletionEpoch::new(1),
                &[request],
                &binding,
            ),
            Err(Unavailable::CommitOutcomeMismatch)
        );
        assert_eq!(
            validate_s1_k4_rearm_enqueue(
                true,
                0,
                identity,
                M1ServingPhysicalRunnerAdapterPhaseV1::Quiescent { epoch },
                None,
                identity,
                epoch,
                plan,
                plan.target(),
                epoch,
                &[request],
                &binding,
            ),
            Err(Unavailable::UnsupportedTransition)
        );
        assert_eq!(
            validate_s1_k4_rearm_enqueue(
                true,
                0,
                identity,
                M1ServingPhysicalRunnerAdapterPhaseV1::Quiescent { epoch },
                Some(plan),
                identity,
                epoch,
                plan,
                plan.draft(),
                epoch,
                &[request],
                &binding,
            ),
            Err(Unavailable::CommitOutcomeMismatch)
        );
        assert_eq!(
            validate_s1_k4_rearm_enqueue(
                true,
                0,
                identity,
                M1ServingPhysicalRunnerAdapterPhaseV1::Quiescent { epoch },
                Some(plan),
                identity,
                epoch,
                plan,
                plan.target(),
                epoch,
                &[],
                &binding,
            ),
            Err(Unavailable::BindingMismatch)
        );
        let stale = M1ServingQueuedGenerationBindingV1::new(
            plan,
            vec![request].into_boxed_slice(),
            CompletionEpoch::new(4),
        );
        assert_eq!(
            validate_s1_k4_rearm_enqueue(
                true,
                0,
                identity,
                M1ServingPhysicalRunnerAdapterPhaseV1::Quiescent { epoch },
                Some(plan),
                identity,
                epoch,
                plan,
                plan.target(),
                epoch,
                &[request],
                &stale,
            ),
            Err(Unavailable::BindingMismatch)
        );
    }

    #[test]
    fn dynamic_s1_k4_rearm_requires_committed_anchor_and_role_cursors() {
        use M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1 as Unavailable;

        let request = RequestId::new(7, 1);
        let epoch = CompletionEpoch::new(3);
        let exact = queued_s1_k4_rearm_test_input(request, epoch, 900, 133, 132, 0);
        let authority = M1CommittedS1K4RearmMemberAuthorityV1 {
            request,
            anchor: Some(900),
            target_committed: 133,
            draft_committed: 132,
        };
        assert_eq!(
            validate_committed_s1_k4_rearm_input(&exact, &[request], [authority].into_iter()),
            Ok(())
        );

        let substituted_anchor = queued_s1_k4_rearm_test_input(request, epoch, 901, 133, 132, 0);
        assert_eq!(
            validate_committed_s1_k4_rearm_input(
                &substituted_anchor,
                &[request],
                [authority].into_iter(),
            ),
            Err(Unavailable::CommittedInputMismatch)
        );
        let substituted_cursor = queued_s1_k4_rearm_test_input(request, epoch, 900, 134, 132, 0);
        assert_eq!(
            validate_committed_s1_k4_rearm_input(
                &substituted_cursor,
                &[request],
                [authority].into_iter(),
            ),
            Err(Unavailable::CommittedInputMismatch)
        );
        let substituted_draft_cursor =
            queued_s1_k4_rearm_test_input(request, epoch, 900, 133, 131, 0);
        assert_eq!(
            validate_committed_s1_k4_rearm_input(
                &substituted_draft_cursor,
                &[request],
                [authority].into_iter(),
            ),
            Err(Unavailable::CommittedInputMismatch)
        );
        let nonzero_future_placeholder =
            queued_s1_k4_rearm_test_input(request, epoch, 900, 133, 132, 1);
        assert_eq!(
            validate_committed_s1_k4_rearm_input(
                &nonzero_future_placeholder,
                &[request],
                [authority].into_iter(),
            ),
            Err(Unavailable::CommittedInputMismatch)
        );
        assert_eq!(
            validate_committed_s1_k4_rearm_input(
                &exact,
                &[request],
                [M1CommittedS1K4RearmMemberAuthorityV1 {
                    request: RequestId::new(8, 1),
                    ..authority
                }]
                .into_iter(),
            ),
            Err(Unavailable::CommitOutcomeMismatch)
        );
        assert_eq!(
            validate_committed_s1_k4_rearm_input(
                &exact,
                &[request],
                [M1CommittedS1K4RearmMemberAuthorityV1 {
                    request,
                    anchor: None,
                    target_committed: 133,
                    draft_committed: 132,
                }]
                .into_iter(),
            ),
            Err(Unavailable::CommitOutcomeMismatch)
        );
    }

    #[test]
    fn dynamic_s1_k4_rearm_failures_retain_preboxed_input() {
        let request = RequestId::new(7, 1);
        let epoch = CompletionEpoch::new(3);
        let input = Box::new(queued_s1_k4_rearm_test_input(
            request, epoch, 900, 133, 132, 0,
        ));
        let input_address = std::ptr::from_ref(input.as_ref());
        let failure = M1ServingPhysicalRunnerS1K4RearmEnqueueFailureV1::Unavailable {
            source: M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1::CommitOutcomeMismatch,
            input,
        };
        assert_eq!(
            failure.unavailable(),
            Some(M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1::CommitOutcomeMismatch)
        );
        assert!(failure.provider_failure().is_none());
        assert_eq!(std::ptr::from_ref(failure.input()), input_address);
        assert_eq!(failure.input().binding().epoch(), epoch);
        let recovered = failure.into_input();
        assert_eq!(recovered.binding().requests(), &[request]);

        let input = Box::new(queued_s1_k4_rearm_test_input(
            request, epoch, 900, 133, 132, 0,
        ));
        let input_address = std::ptr::from_ref(input.as_ref());
        let source = Vec::<u8>::new()
            .try_reserve(usize::MAX)
            .expect_err("usize::MAX queue growth must fail");
        let failure = M1ServingPhysicalRunnerS1K4RearmEnqueueFailureV1::Provider { source, input };
        assert_eq!(failure.unavailable(), None);
        assert!(failure.provider_failure().is_some());
        assert_eq!(std::ptr::from_ref(failure.input()), input_address);
        assert_eq!(failure.input().binding().epoch(), epoch);
        let recovered = failure.into_input();
        assert_eq!(recovered.binding().requests(), &[request]);
    }

    #[test]
    fn selected_roster_requires_exact_count_identity_and_lane_order() {
        let first = RequestId::new(7, 1);
        let second = RequestId::new(8, 1);
        let regenerated_first = RequestId::new(7, 2);
        let expected = [first, second];

        assert!(exact_request_roster_matches(
            &expected,
            [first, second].into_iter()
        ));
        assert!(!exact_request_roster_matches(
            &expected,
            [second, first].into_iter()
        ));
        assert!(!exact_request_roster_matches(
            &expected,
            [first].into_iter()
        ));
        assert!(!exact_request_roster_matches(
            &expected,
            [regenerated_first, second].into_iter()
        ));
    }

    #[test]
    fn diagnostic_history_bound_allows_exactly_first_plus_rearms() {
        assert!(diagnostic_history_can_append(0));
        assert!(diagnostic_history_can_append(M1_MAX_REARM_ROUND_HISTORY_V1));
        assert!(!diagnostic_history_can_append(
            M1_MAX_REARM_ROUND_HISTORY_V1.saturating_add(1)
        ));
    }

    #[test]
    fn direct_prefill_and_decode_plans_are_admitted_without_widening_rearm() {
        for bucket in [
            Qwen3PlanBucket::PrefillS1T128,
            Qwen3PlanBucket::PrefillS8T128,
            Qwen3PlanBucket::PrefillS1T512,
            Qwen3PlanBucket::PrefillS1T2048,
        ] {
            let plan = serving_plan(
                Qwen3ExecutionMode::Prefill,
                bucket,
                Qwen3ExecutionMode::Prefill,
                bucket,
            );
            assert!(supports_direct_serving(plan));
            assert!(supports_serving_plan(plan));
            assert!(!supports_same_shape_rearm(plan));
            assert!(prepared_semantic_evidence_matches(
                plan,
                1,
                &M1ServingPreparedSemanticEvidenceV1::Direct,
            ));
        }

        for bucket in [
            Qwen3PlanBucket::DecodeS1C8192,
            Qwen3PlanBucket::DecodeS8C8192,
            Qwen3PlanBucket::DecodeS32C8192,
        ] {
            let plan = serving_plan(
                Qwen3ExecutionMode::Decode,
                bucket,
                Qwen3ExecutionMode::Decode,
                bucket,
            );
            assert!(supports_direct_serving(plan));
            assert!(supports_serving_plan(plan));
            assert!(supports_same_shape_rearm(plan));
            assert!(prepared_semantic_evidence_matches(
                plan,
                plan.sequence_capacity(),
                &M1ServingPreparedSemanticEvidenceV1::Direct,
            ));
            assert!(!prepared_semantic_evidence_matches(
                plan,
                0,
                &M1ServingPreparedSemanticEvidenceV1::Direct,
            ));
            assert!(!prepared_semantic_evidence_matches(
                plan,
                plan.sequence_capacity() + 1,
                &M1ServingPreparedSemanticEvidenceV1::Direct,
            ));
        }
    }

    #[test]
    fn exact_plan_binding_rejects_same_shape_bucket_substitution() {
        let prefill_128 = serving_plan(
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T128,
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T128,
        );
        let prefill_512 = serving_plan(
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T512,
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T512,
        );
        assert_eq!(prefill_128.shape(), prefill_512.shape());
        assert_eq!(
            validate_exact_serving_plan(prefill_128, prefill_512),
            Err(M1ServingPhysicalRunnerOperationErrorV1::PlanMismatch)
        );
        assert!(!completion_selection_matches(
            prefill_128,
            prefill_512.target()
        ));

        let decode_s1 = serving_plan(
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        );
        let decode_s8 = serving_plan(
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS8C8192,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS8C8192,
        );
        assert_eq!(decode_s1.shape(), decode_s8.shape());
        assert_eq!(
            validate_exact_serving_plan(decode_s1, decode_s8),
            Err(M1ServingPhysicalRunnerOperationErrorV1::PlanMismatch)
        );
        assert!(!completion_selection_matches(decode_s1, decode_s8.target()));

        assert_eq!(validate_exact_serving_plan(decode_s1, decode_s1), Ok(()));
        assert!(completion_selection_matches(decode_s1, decode_s1.target()));
    }

    #[test]
    fn only_s1_k4_speculation_is_admitted() {
        let s1_k4 = serving_plan(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        );
        assert!(supports_evidence_bound_s1_k4(s1_k4));
        assert!(supports_serving_plan(s1_k4));
        assert!(supports_same_shape_rearm(s1_k4));
        assert!(prepared_semantic_evidence_matches(
            s1_k4,
            1,
            &M1ServingPreparedSemanticEvidenceV1::SpeculativeK4,
        ));
        assert!(!prepared_semantic_evidence_matches(
            s1_k4,
            0,
            &M1ServingPreparedSemanticEvidenceV1::SpeculativeK4,
        ));

        for (target_bucket, draft_bucket) in [
            (
                Qwen3PlanBucket::SpeculativeS8K4C8192,
                Qwen3PlanBucket::DecodeS8C8192,
            ),
            (
                Qwen3PlanBucket::SpeculativeS1K8C8192,
                Qwen3PlanBucket::DecodeS1C8192,
            ),
            (
                Qwen3PlanBucket::SpeculativeS1K16C8192,
                Qwen3PlanBucket::DecodeS1C8192,
            ),
        ] {
            let plan = serving_plan(
                Qwen3ExecutionMode::Speculative,
                target_bucket,
                Qwen3ExecutionMode::Decode,
                draft_bucket,
            );
            assert!(!supports_serving_plan(plan));
            assert!(!supports_same_shape_rearm(plan));
            assert!(!prepared_semantic_evidence_matches(
                plan,
                plan.sequence_capacity(),
                &M1ServingPreparedSemanticEvidenceV1::SpeculativeK4,
            ));
        }
    }

    #[test]
    fn exact_dispatch_fail_stop_taxonomy_is_closed() {
        assert!(exact_dispatch_failure_is_fail_stop(
            M1ExactDispatchErrorV1::Faulted
        ));
        assert!(exact_dispatch_failure_is_fail_stop(
            M1ExactDispatchErrorV1::SubmissionEpochExhausted
        ));
        assert!(!exact_dispatch_failure_is_fail_stop(
            M1ExactDispatchErrorV1::EmptyRoster
        ));
        assert!(!exact_dispatch_failure_is_fail_stop(
            M1ExactDispatchErrorV1::PendingBatchCapacityExhausted
        ));
    }

    #[test]
    fn rearm_schedule_fail_stop_taxonomy_includes_epoch_exhaustion() {
        use crate::M1LongLivedQueueRearmScheduleErrorV1 as RearmError;

        assert!(rearm_schedule_error_is_fail_stop(
            RearmError::EpochExhausted
        ));
        assert!(rearm_schedule_error_is_fail_stop(
            RearmError::ExactScheduler(M1ExactDispatchErrorV1::Faulted)
        ));
        assert!(rearm_schedule_error_is_fail_stop(
            RearmError::ExactScheduler(M1ExactDispatchErrorV1::SubmissionEpochExhausted)
        ));
        assert!(!rearm_schedule_error_is_fail_stop(
            RearmError::HostAllocation
        ));
        assert!(!rearm_schedule_error_is_fail_stop(
            RearmError::ExactScheduler(M1ExactDispatchErrorV1::PendingBatchCapacityExhausted)
        ));
    }

    #[test]
    #[ignore = "requires admitted K1-K7 artifacts, prepacked Qwen bytes, and an exclusive MI300X"]
    fn configured_mi300x_queued_provider_rolls_s1_prefill_into_native_s1_k4() {
        use std::fs;

        use fe2o3_kfd::{DeviceSelector, OpenedKfd};
        use ferric_build::{
            generate_qwen3_gfx942_runner_declaration, m1_step_workspace_requirements,
            plan_addressless_m1_step_workspace, publish_qwen3_gfx942_runner_declaration,
            qwen3_model_memory_plan_test_fixture, qwen3_runner_closure_test_fixture,
            AddresslessM1StepWorkspacePlan, AvailableM1StepWorkspace,
            DeclaredM1StepWorkspaceAllocation, M1StepWorkspaceDeclaration,
            M1StepWorkspacePlanOutcome,
        };
        use ferric_spec::{
            validate_m1_step_inputs, Identity, M1StepInputCandidate, M1StepInputValidationOutcome,
            ValidatedM1StepInputs,
        };

        use crate::{
            bind_m1_kv_workspace_table_v1, bind_m1_physical_runner_v1,
            initialize_m1_physical_runner_memory_v1, reopen_persisted_m1_kernel_artifacts_v1,
            ActiveDeviceKvCache, Engine, M1DeviceKvCompletionDispositionV1,
            M1FullStepKvWorkspaceTablesV1, M1FullStepWorkspacePlans,
            M1QueuedServingPhysicalInputProviderV1, M1RearmRoundHistoryEntryV1,
            M1S1K4QueueRolloverKvInputsV1, M1ServingCompletionDispositionV1,
            M1ServingQueueActionV1, M1ServingQueuedFirstPublicationV1,
            M1ServingQueuedGenerationBindingV1, M1ServingQueuedGenerationInputV1,
            M1ServingQueuedS1K4RolloverV1, M1ServingRegistryV1,
        };

        fn required_path(name: &str) -> std::path::PathBuf {
            std::env::var_os(name).map_or_else(|| panic!("set {name}"), std::path::PathBuf::from)
        }

        fn input(
            plan: ferric_spec::StepPlan,
            tokens: Vec<u32>,
            positions: Vec<u32>,
            active_length: u32,
            context_length: u32,
        ) -> ValidatedM1StepInputs {
            let candidate = M1StepInputCandidate::new(
                plan.selection(),
                vec![Some(plan)],
                tokens,
                positions,
                vec![active_length],
                vec![context_length],
            );
            match validate_m1_step_inputs(candidate) {
                M1StepInputValidationOutcome::Validated(inputs) => inputs,
                M1StepInputValidationOutcome::Rejected(failure) => {
                    panic!("rollover smoke input rejected: {:?}", failure.error())
                }
            }
        }

        fn workspace_plan(
            selection: Qwen3PlanSelection,
            identity_byte: u8,
        ) -> AddresslessM1StepWorkspacePlan {
            let requirements = m1_step_workspace_requirements(selection)
                .expect("canonical selection has workspace requirements");
            let available = AvailableM1StepWorkspace::new(M1StepWorkspaceDeclaration::new(
                selection,
                DeclaredM1StepWorkspaceAllocation::new(
                    Identity::new([identity_byte; 32]),
                    requirements.allocation_byte_len(),
                    requirements.allocation_alignment(),
                ),
                requirements.ranges().to_vec().into_boxed_slice(),
            ));
            match plan_addressless_m1_step_workspace(selection, available) {
                M1StepWorkspacePlanOutcome::Planned(plan) => plan,
                M1StepWorkspacePlanOutcome::Rejected(_) => {
                    panic!("exact workspace fixture rejected")
                }
            }
        }

        // This output-fed ignored smoke observes only typed lifecycle and
        // native queue rollover structure. Fixture artifacts do not
        // authenticate deployment inputs, so it makes no numerical,
        // performance, evidence, qualification, or Qwen-correctness claim.
        let artifact_directory = required_path("FERRIC_M1_KERNEL_ARTIFACT_DIRECTORY");
        let target_weights = required_path("FERRIC_M1_TARGET_PREPACKED_WEIGHTS");
        let draft_weights = required_path("FERRIC_M1_DRAFT_PREPACKED_WEIGHTS");
        let unique_id = std::env::var("FERRIC_M1_GPU_UNIQUE_ID")
            .expect("set FERRIC_M1_GPU_UNIQUE_ID to the selected MI300X unique ID")
            .parse::<u64>()
            .expect("FERRIC_M1_GPU_UNIQUE_ID must be a decimal u64");

        let artifacts = reopen_persisted_m1_kernel_artifacts_v1(artifact_directory)
            .expect("admit persisted K1-K7 artifacts");
        let declaration =
            generate_qwen3_gfx942_runner_declaration(qwen3_runner_closure_test_fixture())
                .expect("generate fixture structural publication");
        let publication = publish_qwen3_gfx942_runner_declaration(declaration)
            .expect("publish fixture structural declaration");
        let runner = bind_m1_physical_runner_v1(artifacts, publication)
            .expect("bind persisted kernels to canonical operations");
        let checked = OpenedKfd::open_default()
            .expect("open KFD")
            .admit_uapi()
            .expect("admit pinned KFD UAPI")
            .bind_gfx942_xnack_minus(DeviceSelector::UniqueId(unique_id))
            .expect("bind checked gfx942:xnack- MI300X");
        let mut memory = initialize_m1_physical_runner_memory_v1(
            checked,
            qwen3_model_memory_plan_test_fixture(),
            fs::read(target_weights)
                .expect("read target prepacked bytes")
                .into_boxed_slice(),
            fs::read(draft_weights)
                .expect("read draft prepacked bytes")
                .into_boxed_slice(),
        )
        .expect("initialize target/draft memory and partition KV arenas");

        let prefill = serving_plan(
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T128,
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T128,
        );
        let speculative = serving_plan(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        );
        let mut engine = Engine::<1>::new(512, 256, 8_192).expect("construct one-lane engine");
        let request = engine.admit().expect("admit one request");
        engine
            .append_tentative(request, 1)
            .expect("append the one logical prefill completion token");
        let mut registry = M1ServingRegistryV1::<1>::new().expect("construct one-lane registry");
        registry
            .admit(request, prefill)
            .expect("admit paired prefill into serving registry");

        let prefill_epoch = CompletionEpoch::new(1);
        let target_prefill_plan = runner
            .logical_runner()
            .bind_step_plan(request, prefill_epoch, prefill.target())
            .expect("bind target prefill plan");
        let draft_prefill_plan = runner
            .logical_runner()
            .bind_step_plan(request, prefill_epoch, prefill.draft())
            .expect("bind draft prefill plan");
        let target_prefill_inputs = input(
            target_prefill_plan,
            vec![1; 128],
            (0..128).collect(),
            128,
            0,
        );
        let draft_prefill_inputs =
            input(draft_prefill_plan, vec![1; 128], (0..128).collect(), 128, 0);

        let mut cache =
            ActiveDeviceKvCache::new(memory.device(), request, prefill.target(), prefill.draft())
                .expect("construct paired-prefill cache");
        let target_prefill_leases = (0..8_u32)
            .map(|page| {
                memory
                    .lease_page(request, Qwen3ModelRole::Target8B, page)
                    .expect("lease target prefill page")
            })
            .collect();
        let draft_prefill_leases = (0..8_u32)
            .map(|page| {
                memory
                    .lease_page(request, Qwen3ModelRole::Draft06B, page)
                    .expect("lease draft prefill page")
            })
            .collect();
        let target_prefill_pending = cache
            .reserve_step_write(
                request,
                Qwen3ModelRole::Target8B,
                0,
                128,
                prefill_epoch,
                target_prefill_leases,
            )
            .expect("reserve target prefill write");
        let draft_prefill_pending = cache
            .reserve_step_write(
                request,
                Qwen3ModelRole::Draft06B,
                0,
                128,
                prefill_epoch,
                draft_prefill_leases,
            )
            .expect("reserve draft prefill write");
        let target_prefill_table =
            bind_m1_kv_workspace_table_v1(target_prefill_inputs, vec![target_prefill_pending])
                .expect("bind target prefill KV table");
        let draft_prefill_table =
            bind_m1_kv_workspace_table_v1(draft_prefill_inputs, vec![draft_prefill_pending])
                .expect("bind draft prefill KV table");
        let draft_rollover_lease = memory
            .lease_page(request, Qwen3ModelRole::Draft06B, 8)
            .expect("lease first post-prefill draft page");
        let target_rollover_lease = memory
            .lease_page(request, Qwen3ModelRole::Target8B, 8)
            .expect("lease first post-prefill target page");
        let prefill_tables = M1FullStepKvWorkspaceTablesV1::PairedPrefill {
            draft: draft_prefill_table,
            target: target_prefill_table,
        };
        let prefill_preparation_plans = M1FullStepWorkspacePlans::paired_prefill(
            workspace_plan(prefill.draft(), 90),
            workspace_plan(prefill.target(), 91),
        );
        let prefill_recipe_plans = M1FullStepWorkspacePlans::paired_prefill(
            workspace_plan(prefill.draft(), 90),
            workspace_plan(prefill.target(), 91),
        );
        let first = M1ServingQueuedGenerationInputV1::first_publication(
            M1ServingQueuedFirstPublicationV1::new(
                M1ServingQueuedGenerationBindingV1::new(
                    prefill,
                    vec![request].into_boxed_slice(),
                    prefill_epoch,
                ),
                memory,
                prefill_tables,
                prefill_preparation_plans,
                prefill_recipe_plans,
                vec![cache],
            ),
        );

        let rollover_epoch = CompletionEpoch::new(2);
        let provider = M1QueuedServingPhysicalInputProviderV1::from_ordered_inputs(vec![first]);
        let mut operations =
            M1ServingPhysicalRunnerOperationsV1::new(&runner, &mut engine, provider, 1 << 20)
                .expect("construct queued physical serving adapter");

        let prefill_batch = registry
            .plan_next()
            .expect("plan paired prefill")
            .expect("paired prefill is ready");
        let prefill_reservation = registry
            .reserve_publication(prefill_batch)
            .expect("reserve paired-prefill publication");
        let prefill_batch = prefill_reservation.physical_batch();
        assert_eq!(prefill_batch.action(), M1ServingQueueActionV1::FreshLaunch);
        let published = operations
            .fresh_launch(&prefill_batch)
            .expect("publish paired prefill through queued provider");
        registry
            .record_publication(prefill_reservation)
            .expect("record paired-prefill publication");
        let readback = operations
            .read_published(published, prefill_epoch, &prefill_batch)
            .expect("read paired-prefill completion");
        let anchor = {
            let checked = operations.checked_completion(&readback);
            let [record] = checked.records() else {
                panic!("S1 paired prefill must produce one checked record")
            };
            let crate::CheckedCompletionSemantics::DirectFinalRow { token } = record.semantics()
            else {
                panic!("paired-prefill checked record must retain direct semantics")
            };
            token
        };
        let target_speculative_plan = runner
            .logical_runner()
            .bind_step_plan(request, rollover_epoch, speculative.target())
            .expect("bind target speculative plan");
        let draft_decode_plan = runner
            .logical_runner()
            .bind_step_plan(request, rollover_epoch, speculative.draft())
            .expect("bind draft decode plan");
        let target_speculative_inputs = input(
            target_speculative_plan,
            vec![anchor, 0, 0, 0, 0],
            (128..133).collect(),
            5,
            128,
        );
        let draft_decode_inputs = input(draft_decode_plan, vec![anchor], vec![128], 1, 128);
        let rollover_inputs = M1S1K4QueueRolloverKvInputsV1::new(
            draft_decode_inputs,
            target_speculative_inputs,
            vec![draft_rollover_lease],
            vec![target_rollover_lease],
        );
        let rollover_preparation_plans = M1FullStepWorkspacePlans::speculative_round(
            workspace_plan(speculative.draft(), 92),
            workspace_plan(speculative.target(), 93),
        );
        let rollover_recipe_plans = M1FullStepWorkspacePlans::speculative_round(
            workspace_plan(speculative.draft(), 92),
            workspace_plan(speculative.target(), 93),
        );
        let rollover = M1ServingQueuedS1K4RolloverV1::new(
            M1ServingQueuedGenerationBindingV1::new(
                speculative,
                vec![request].into_boxed_slice(),
                rollover_epoch,
            ),
            rollover_inputs,
            rollover_preparation_plans,
            rollover_recipe_plans,
        );
        operations
            .try_enqueue_s1_k4_rollover_after_readback(&readback, Box::new(rollover))
            .expect("enqueue output-fed S1/K4 successor after paired-prefill readback");
        assert_eq!(
            operations
                .provider()
                .expect("adapter retains queued provider")
                .pending_generation_count(),
            1
        );
        let quiescent = operations
            .settle_readback(readback, vec![M1DeviceKvCompletionDispositionV1::Continue])
            .expect("settle paired-prefill completion");
        registry
            .complete_exact(
                prefill_epoch,
                &[M1ServingCompletionDispositionV1::Continue(speculative)],
            )
            .expect("advance registry from prefill to speculative");
        operations
            .engine
            .append_tentative(request, 5)
            .expect("append one exact S1/K4 target span");

        let rollover_batch = registry
            .plan_next()
            .expect("plan exact S1/K4 successor")
            .expect("S1/K4 successor is ready");
        assert_eq!(rollover_batch.epoch(), rollover_epoch);
        assert_eq!(
            rollover_batch.action(),
            M1ServingQueueActionV1::QuiescentRollover {
                prior: prefill,
                next: speculative,
                reason: M1ServingRolloverReasonV1::Mode,
            }
        );
        let rollover_reservation = registry
            .reserve_publication(rollover_batch)
            .expect("reserve exact S1/K4 rollover publication");
        let rollover_batch = rollover_reservation.physical_batch();
        let published = operations
            .quiescent_rollover(
                quiescent,
                prefill,
                speculative,
                M1ServingRolloverReasonV1::Mode,
                &rollover_batch,
            )
            .expect("replace paired-prefill queue with native S1/K4 queue");
        registry
            .record_publication(rollover_reservation)
            .expect("record exact S1/K4 rollover publication");
        let readback = operations
            .read_published(published, rollover_epoch, &rollover_batch)
            .expect("read replacement S1/K4 completion");
        operations
            .engine
            .retire(request)
            .expect("mark final Engine member retiring before physical settlement");
        let quiescent = operations
            .settle_readback(readback, vec![M1DeviceKvCompletionDispositionV1::Retire])
            .expect("settle replacement S1/K4 completion");
        registry
            .complete_exact(rollover_epoch, &[M1ServingCompletionDispositionV1::Retire])
            .expect("retire final serving-registry member");
        assert_eq!(
            operations
                .provider()
                .expect("adapter retains queued provider")
                .pending_generation_count(),
            0
        );

        let M1ServingPhysicalRunnerQuiescentV1 { state, .. } = quiescent;
        let M1ServingPhysicalRunnerQuiescentStateV1::Rearmed {
            released,
            diagnostic_history,
        } = state
        else {
            panic!("S1/K4 rollover did not return rearmed queue custody")
        };
        assert!(matches!(
            diagnostic_history.evidence(),
            [
                M1ServingPhysicalRunnerReadbackEvidenceV1::Direct(_),
                M1ServingPhysicalRunnerReadbackEvidenceV1::SpeculativeK4(_)
            ]
        ));
        assert_eq!(released.round_history_len(), 1);
        let history = released
            .round_history(0)
            .expect("replacement round retains rollover history");
        let rollover = history
            .rollover_observation()
            .expect("replacement round retains native rollover observation");
        assert_eq!(
            rollover.previous_queue_destroyed().queue_id(),
            history.queue_observation().queue_id()
        );
        assert!(rollover.previous_queue_destroyed().released_resources() > 0);
        assert_eq!(rollover.previous_dispatch_generation(), 1);
        assert_eq!(rollover.replacement_dispatch_generation(), 2);
        assert_eq!(
            rollover.replacement_queue_observation().ring_bytes(),
            1 << 20
        );

        let teardown = released
            .destroy_queue_and_retain_round(operations.engine)
            .expect("destroy replacement queue and retain exact round lineage");
        assert_eq!(teardown.round_history_len(), 1);
        assert!(teardown
            .round_history(0)
            .and_then(M1RearmRoundHistoryEntryV1::rollover_observation)
            .is_some());
        registry
            .record_quiescent_queue_retirement(speculative)
            .expect("record destroyed replacement queue");
        registry
            .remove_retired(request)
            .expect("remove retired serving-registry member");
        operations.phase = M1ServingPhysicalRunnerAdapterPhaseV1::Sealed;
        assert!(!operations.engine.is_faulted());
        drop(teardown);
    }
}
