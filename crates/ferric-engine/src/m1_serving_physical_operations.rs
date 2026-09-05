//! Production physical-operation adapter for dynamic M1 serving.
//!
//! The registry intentionally owns no tokens, workspace images, page leases,
//! or active device-cache owners. Those inputs stay behind
//! [`M1ServingPhysicalInputProviderV1`], while this adapter alone performs the
//! exact scheduler transition and consumes the resulting physical typestates.
//! Direct paired-prefill and target-only generations use compact final-row
//! semantics. Finite speculative generations additionally require the exact
//! independent diagnostic-choice attachment for their selected S/K shape.

use core::{
    fmt,
    sync::atomic::{AtomicU64, Ordering},
};

use ferric_spec::{completion::CompletionEpoch, Qwen3PlanBucket, RequestId};

use crate::m1_serving_physical_input_provider::M1ServingCommittedSpeculativeMemberBindingV1;
use crate::m1_serving_registry::admit_m1_production_rollover_transition_v1;

use crate::{
    complete_m1_physical_step_v1, release_m1_completed_step_kv_pages_v1,
    schedule_m1_finite_speculative_queue_rollover_v1, schedule_m1_long_lived_queue_rearm_exact_v1,
    ActiveDeviceKvCache, AddresslessM1PhysicalBufferRecipeV1, BoundM1CompletionOutputV1, Engine,
    M1AllocatedScheduledStepV1, M1CheckedCompletionOutputV1, M1CompletedStepOutcomeV1,
    M1CompletedStepRejectionV1, M1DeviceKvCompletionDispositionV1, M1DeviceKvCompletionMemberV1,
    M1DeviceKvCompletionRosterV1, M1ExactDispatchErrorV1,
    M1FiniteSpeculativeQueueRolloverScheduleFailureCustodyV1, M1LongLivedQueueReleasedRoundV1,
    M1LongLivedQueueUnscheduledRoundV1, M1ObservedSpeculativeDiagnosticChoicesV1,
    M1PhysicalCompletedReadbackV1, M1PhysicalFixedBatchShapeV1, M1PhysicalPublishedQueueSessionV1,
    M1PhysicalRunnerV1, M1PreparedLongLivedQueueRearmV1, M1QueuedServingPhysicalInputProviderV1,
    M1RearmedCompletedReadbackV1, M1RearmedCompletionOutcomeV1,
    M1RearmedCompletionPreflightFailureV1, M1RearmedPublishedQueueV1,
    M1RearmedRoundReleaseOutcomeV1, M1ReleasedCompletedStepV1, M1ScheduledDispatchV1,
    M1ScheduledFiniteSpeculativeQueueRolloverV1, M1ScheduledLongLivedQueueRearmV1,
    M1ScheduledS1K4QueueRolloverV1, M1ServingBatchPlanV1, M1ServingCommittedSpeculativeRoundV1,
    M1ServingPhysicalOperationFailureV1, M1ServingPhysicalOperationResultV1,
    M1ServingPhysicalOperationsV1, M1ServingPhysicalReadbackV1, M1ServingPlanV1,
    M1ServingQueuedFiniteSpeculativeRolloverV1, M1ServingQueuedGenerationBindingV1,
    M1ServingQueuedS1K4RolloverV1, M1ServingQueuedSameShapeRearmV1, M1ServingRolloverReasonV1,
    M1SpeculativeMemberStatusV1, M1_MAX_REARM_ROUND_HISTORY_V1,
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
    /// Finite speculation derives expectations from attached independent choices.
    ///
    /// The variant name is retained for source compatibility with the original
    /// exact S1/K4 serving API.
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

/// Request-owned inputs prepared after one finite paired-prefill rollover.
#[must_use = "prepared rollover custody must publish or remain retained"]
#[derive(Debug)]
pub struct M1ServingPreparedFiniteSpeculativeRolloverV1 {
    prepared: crate::M1PreparedFiniteSpeculativeQueueRolloverV1,
    recipe: AddresslessM1PhysicalBufferRecipeV1,
    semantic_evidence: M1ServingPreparedSemanticEvidenceV1,
}

impl M1ServingPreparedFiniteSpeculativeRolloverV1 {
    /// Joins a prepared native rollover to its exact physical recipe and
    /// independent finite-speculative semantic evidence.
    pub const fn new(
        prepared: crate::M1PreparedFiniteSpeculativeQueueRolloverV1,
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

/// Source-compatible name for the original exact S1/K4 prepared rollover.
pub type M1ServingPreparedS1K4RolloverV1 = M1ServingPreparedFiniteSpeculativeRolloverV1;

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

    /// Declares support for rollover shapes beyond the original exact S1/K4
    /// provider contract. Existing providers remain exact and fail closed.
    #[must_use]
    fn supports_wider_finite_speculative_rollover(&self) -> bool {
        false
    }

    /// Prepares one transition admitted by the finite production rollover catalog.
    ///
    /// The adapter calls this default only for the original exact S1/K4 shape.
    /// Wider shapes require an explicit capability opt-in before queue detach.
    ///
    /// # Errors
    ///
    /// Returns exhaustive provider custody when rollover inputs cannot be
    /// prepared.
    fn prepare_finite_speculative_rollover(
        &mut self,
        runner: &M1PhysicalRunnerV1,
        engine: &mut Engine<C>,
        batch: &M1ServingBatchPlanV1,
        scheduled: M1ScheduledFiniteSpeculativeQueueRolloverV1,
    ) -> Result<M1ServingPreparedFiniteSpeculativeRolloverV1, Self::Failure> {
        self.prepare_s1_k4_rollover(runner, engine, batch, scheduled)
    }
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
    bindings: Vec<M1ServingPhysicalRunnerDiagnosticBindingV1>,
}

/// Exact serving identity for one retained diagnostic evidence generation.
#[derive(Debug, Eq, PartialEq)]
pub struct M1ServingPhysicalRunnerDiagnosticBindingV1 {
    plan: M1ServingPlanV1,
    epoch: CompletionEpoch,
    requests: Vec<RequestId>,
}

impl M1ServingPhysicalRunnerDiagnosticBindingV1 {
    #[must_use]
    pub const fn plan(&self) -> M1ServingPlanV1 {
        self.plan
    }

    #[must_use]
    pub const fn epoch(&self) -> CompletionEpoch {
        self.epoch
    }

    /// Ordered live request roster corresponding to evidence lane order.
    #[must_use]
    pub fn requests(&self) -> &[RequestId] {
        &self.requests
    }
}

impl M1ServingPhysicalRunnerDiagnosticHistoryV1 {
    fn new() -> Self {
        Self {
            evidence: Vec::new(),
            bindings: Vec::new(),
        }
    }

    fn len(&self) -> usize {
        self.evidence.len()
    }

    fn try_reserve_exact(
        &mut self,
        additional: usize,
    ) -> Result<(), std::collections::TryReserveError> {
        self.evidence.try_reserve_exact(additional)?;
        self.bindings.try_reserve_exact(additional)
    }

    fn push(
        &mut self,
        evidence: M1ServingPhysicalRunnerReadbackEvidenceV1,
        binding: M1ServingPhysicalRunnerDiagnosticBindingV1,
    ) {
        self.evidence.push(evidence);
        self.bindings.push(binding);
    }

    /// Borrows settled direct or speculative evidence in generation order.
    pub fn evidence(&self) -> &[M1ServingPhysicalRunnerReadbackEvidenceV1] {
        &self.evidence
    }

    /// Borrows exact plan, epoch, and live-roster bindings in evidence order.
    #[must_use]
    pub fn bindings(&self) -> &[M1ServingPhysicalRunnerDiagnosticBindingV1] {
        debug_assert_eq!(self.evidence.len(), self.bindings.len());
        &self.bindings
    }
}

fn prepare_diagnostic_binding(
    plan: M1ServingPlanV1,
    epoch: CompletionEpoch,
    checked: &M1CheckedCompletionOutputV1,
) -> Result<M1ServingPhysicalRunnerDiagnosticBindingV1, std::collections::TryReserveError> {
    let mut requests = Vec::new();
    requests.try_reserve_exact(checked.records().len())?;
    requests.extend(
        checked
            .records()
            .iter()
            .map(|record| record.record().request),
    );
    Ok(M1ServingPhysicalRunnerDiagnosticBindingV1 {
        plan,
        epoch,
        requests,
    })
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

/// One physically published serving generation.
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
    /// Independent finite-speculative draft and target choices.
    ///
    /// The variant name is retained for source compatibility with the original
    /// exact S1/K4 serving API.
    SpeculativeK4(Box<M1ObservedSpeculativeDiagnosticChoicesV1>),
}

impl M1ServingPhysicalRunnerReadbackEvidenceV1 {
    fn append_diagnostic_history(
        self,
        history: &mut M1ServingPhysicalRunnerDiagnosticHistoryV1,
        binding: M1ServingPhysicalRunnerDiagnosticBindingV1,
    ) {
        history.push(self, binding);
    }
}

/// Semantically joined readback retaining independent choice evidence.
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

/// Exhaustive failure to append one checked finite-speculative successor.
///
/// Both variants retain the exact rejected input. `Unavailable` means no
/// provider queue mutation was attempted; `Provider` preserves the lower
/// allocation diagnostic after a failed transactional enqueue.
#[must_use = "failed dynamic enqueue retains the exact generation input"]
#[derive(Debug)]
pub enum M1ServingPhysicalRunnerFiniteSpeculativeRolloverEnqueueFailureV1 {
    /// Adapter/readback validation rejected before provider queue mutation.
    Unavailable {
        /// Stable reason enqueue was unavailable.
        source: M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1,
        /// Exact unchanged generation input.
        input: Box<M1ServingQueuedFiniteSpeculativeRolloverV1>,
    },
    /// Transactional provider queue growth failed with lower custody intact.
    Provider {
        /// Lower host queue-growth diagnostic.
        source: std::collections::TryReserveError,
        /// Exact unchanged generation input.
        input: Box<M1ServingQueuedFiniteSpeculativeRolloverV1>,
    },
}

impl M1ServingPhysicalRunnerFiniteSpeculativeRolloverEnqueueFailureV1 {
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
    pub const fn input(&self) -> &M1ServingQueuedFiniteSpeculativeRolloverV1 {
        match self {
            Self::Unavailable { input, .. } | Self::Provider { input, .. } => input,
        }
    }

    /// Recovers the unchanged pre-boxed input from either failure class.
    #[must_use = "the rejected generation input remains linear"]
    pub fn into_input(self) -> Box<M1ServingQueuedFiniteSpeculativeRolloverV1> {
        match self {
            Self::Unavailable { input, .. } | Self::Provider { input, .. } => input,
        }
    }
}

/// Source-compatible name for the original exact S1/K4 enqueue failure.
pub type M1ServingPhysicalRunnerGenerationEnqueueFailureV1 =
    M1ServingPhysicalRunnerFiniteSpeculativeRolloverEnqueueFailureV1;

/// Exhaustive failure to append one committed finite-speculative rearm.
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

/// Shape-generic name for finite-speculative rearm enqueue failures.
pub type M1ServingPhysicalRunnerSpeculativeRearmEnqueueFailureV1 =
    M1ServingPhysicalRunnerS1K4RearmEnqueueFailureV1;

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
        failure: crate::M1PhysicalRunnerFiniteSpeculativeRolloverSubmissionFailureV1<'a>,
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

    /// Borrows checked completion only from this adapter's exact readback custody.
    ///
    /// # Errors
    ///
    /// Rejects cross-adapter custody, any non-readback phase or epoch, and plan
    /// drift between the bridge wrapper, operation custody, and active adapter.
    pub fn checked_completion_for_readback<'b>(
        &self,
        readback: &'b M1ServingPhysicalReadbackV1<M1ServingPhysicalRunnerReadbackV1>,
    ) -> Result<&'b M1CheckedCompletionOutputV1, M1ServingPhysicalRunnerOperationErrorV1>
    where
        P: M1ServingPhysicalInputProviderV1<C>,
        P::Failure: 'a,
    {
        let custody = readback.operation_custody();
        validate_custody_guard(
            self.identity,
            custody.adapter_identity(),
            matches!(
                self.phase,
                M1ServingPhysicalRunnerAdapterPhaseV1::Readback { epoch }
                    if epoch == custody.epoch()
            ),
        )?;
        if self.active_plan != Some(custody.plan())
            || readback.plan() != custody.plan()
            || readback.epoch() != custody.epoch()
            || readback.batch().plan() != custody.plan()
            || readback.batch().epoch() != custody.epoch()
        {
            return Err(M1ServingPhysicalRunnerOperationErrorV1::PlanMismatch);
        }
        Ok(self.checked_completion(custody))
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
    /// Appends one finite-speculative successor after paired-prefill readback.
    ///
    /// This capability is deliberately available only for the concrete queued
    /// provider. It validates the readback custody, exact next epoch and
    /// request roster, finite transition catalog, and every direct lane anchor
    /// before the provider queue can change.
    ///
    /// # Errors
    ///
    /// Returns the pre-boxed input unchanged when the adapter/readback cannot
    /// accept a successor or when transactional provider queue growth fails.
    pub fn try_enqueue_finite_speculative_rollover_after_readback(
        &mut self,
        readback: &M1ServingPhysicalReadbackV1<M1ServingPhysicalRunnerReadbackV1>,
        input: Box<M1ServingQueuedFiniteSpeculativeRolloverV1>,
    ) -> Result<(), M1ServingPhysicalRunnerFiniteSpeculativeRolloverEnqueueFailureV1> {
        let readback = readback.operation_custody();
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
                M1ServingPhysicalRunnerFiniteSpeculativeRolloverEnqueueFailureV1::Unavailable {
                    source,
                    input,
                },
            );
        }
        for (lane, record) in checked.records().iter().enumerate() {
            let crate::CheckedCompletionSemantics::DirectFinalRow { token } = record.semantics()
            else {
                return Err(
                    M1ServingPhysicalRunnerFiniteSpeculativeRolloverEnqueueFailureV1::Unavailable {
                        source: M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1::UnsupportedTransition,
                        input,
                    },
                );
            };
            if !input.matches_anchor_at(lane, token) {
                return Err(
                    M1ServingPhysicalRunnerFiniteSpeculativeRolloverEnqueueFailureV1::Unavailable {
                        source:
                            M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1::AnchorMismatch,
                        input,
                    },
                );
            }
        }
        let Some(provider) = self.provider.as_mut() else {
            return Err(
                M1ServingPhysicalRunnerFiniteSpeculativeRolloverEnqueueFailureV1::Unavailable {
                    source:
                        M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1::ProviderUnavailable,
                    input,
                },
            );
        };
        provider
            .try_enqueue_finite_speculative_rollover(input)
            .map_err(|(source, input)| {
                M1ServingPhysicalRunnerFiniteSpeculativeRolloverEnqueueFailureV1::Provider {
                    source,
                    input,
                }
            })
    }

    /// Appends the original exact S1/K4 successor after paired-prefill readback.
    ///
    /// # Errors
    ///
    /// Returns the input unchanged when it is not the legacy exact S1/K4 case,
    /// or forwards the generic finite-rollover rejection.
    pub fn try_enqueue_s1_k4_rollover_after_readback(
        &mut self,
        readback: &M1ServingPhysicalReadbackV1<M1ServingPhysicalRunnerReadbackV1>,
        input: Box<M1ServingQueuedS1K4RolloverV1>,
    ) -> Result<(), M1ServingPhysicalRunnerGenerationEnqueueFailureV1> {
        if input.binding().plan().target().bucket != Qwen3PlanBucket::SpeculativeS1K4C8192
            || input.binding().requests().len() != 1
        {
            return Err(
                M1ServingPhysicalRunnerFiniteSpeculativeRolloverEnqueueFailureV1::Unavailable {
                    source:
                        M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1::UnsupportedTransition,
                    input,
                },
            );
        }
        self.try_enqueue_finite_speculative_rollover_after_readback(readback, input)
    }

    /// Appends one finite-speculative successor after atomic speculative commit.
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
    pub fn try_enqueue_speculative_rearm_after_commit(
        &mut self,
        committed: &M1ServingCommittedSpeculativeRoundV1<M1ServingPhysicalRunnerQuiescentV1>,
        input: Box<M1ServingQueuedSameShapeRearmV1>,
    ) -> Result<(), M1ServingPhysicalRunnerSpeculativeRearmEnqueueFailureV1> {
        let quiescent = committed.quiescent();
        let outcome = committed.outcome();
        try_enqueue_committed_speculative_rearm(
            &mut self.provider,
            self.identity,
            self.phase,
            self.active_plan,
            quiescent.adapter_identity(),
            quiescent.epoch(),
            committed.plan(),
            outcome.selection(),
            outcome.completed_epoch(),
            outcome.next_active_roster(),
            outcome.members(),
            input,
        )
    }

    /// Appends an exact same-shape S1/K4 successor after atomic commit.
    ///
    /// This compatibility entry point remains closed to every other finite
    /// speculative shape. Use [`Self::try_enqueue_speculative_rearm_after_commit`]
    /// for S8/K4, S1/K8, or S1/K16.
    ///
    /// # Errors
    ///
    /// Returns the pre-boxed input unchanged for a wider plan or when generic
    /// committed-rearm validation or transactional provider growth fails.
    pub fn try_enqueue_s1_k4_rearm_after_commit(
        &mut self,
        committed: &M1ServingCommittedSpeculativeRoundV1<M1ServingPhysicalRunnerQuiescentV1>,
        input: Box<M1ServingQueuedSameShapeRearmV1>,
    ) -> Result<(), M1ServingPhysicalRunnerS1K4RearmEnqueueFailureV1> {
        if let Err(source) = validate_s1_k4_rearm_compatibility_plan(committed.plan()) {
            return Err(
                M1ServingPhysicalRunnerSpeculativeRearmEnqueueFailureV1::Unavailable {
                    source,
                    input,
                },
            );
        }
        self.try_enqueue_speculative_rearm_after_commit(committed, input)
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

fn validate_s1_k4_rearm_compatibility_plan(
    plan: M1ServingPlanV1,
) -> Result<(), M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1> {
    if supports_evidence_bound_s1_k4(plan) {
        Ok(())
    } else {
        Err(M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1::UnsupportedTransition)
    }
}

fn supports_evidence_bound_speculation(plan: M1ServingPlanV1) -> bool {
    matches!(
        (plan.shape(), plan.target().bucket),
        (
            M1PhysicalFixedBatchShapeV1::SpeculativeK4,
            Qwen3PlanBucket::SpeculativeS1K4C8192 | Qwen3PlanBucket::SpeculativeS8K4C8192
        ) | (
            M1PhysicalFixedBatchShapeV1::SpeculativeK8,
            Qwen3PlanBucket::SpeculativeS1K8C8192
        ) | (
            M1PhysicalFixedBatchShapeV1::SpeculativeK16,
            Qwen3PlanBucket::SpeculativeS1K16C8192
        )
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct M1CommittedSpeculativeRearmMemberAuthorityV1 {
    request: RequestId,
    anchor: Option<ferric_spec::TokenId>,
    target_committed: u32,
    draft_committed: u32,
}

#[allow(clippy::too_many_arguments)]
fn try_enqueue_committed_speculative_rearm(
    provider: &mut Option<M1QueuedServingPhysicalInputProviderV1>,
    expected_identity: M1ServingPhysicalRunnerAdapterIdentityV1,
    phase: M1ServingPhysicalRunnerAdapterPhaseV1,
    active_plan: Option<M1ServingPlanV1>,
    custody_identity: M1ServingPhysicalRunnerAdapterIdentityV1,
    custody_epoch: CompletionEpoch,
    custody_plan: M1ServingPlanV1,
    outcome_selection: ferric_spec::Qwen3PlanSelection,
    outcome_epoch: CompletionEpoch,
    outcome_next_roster: &[RequestId],
    outcome_members: &[crate::M1SpeculativeMemberRoundOutcomeV1],
    input: Box<M1ServingQueuedSameShapeRearmV1>,
) -> Result<(), M1ServingPhysicalRunnerSpeculativeRearmEnqueueFailureV1> {
    if let Err(source) = validate_speculative_rearm_enqueue(
        provider.is_some(),
        provider.as_ref().map_or(
            0,
            M1QueuedServingPhysicalInputProviderV1::pending_generation_count,
        ),
        expected_identity,
        phase,
        active_plan,
        custody_identity,
        custody_epoch,
        custody_plan,
        outcome_selection,
        outcome_epoch,
        outcome_next_roster,
        input.binding(),
    ) {
        return Err(
            M1ServingPhysicalRunnerSpeculativeRearmEnqueueFailureV1::Unavailable { source, input },
        );
    }
    let authorities = outcome_members
        .iter()
        .copied()
        .filter(|member| member.status() == M1SpeculativeMemberStatusV1::Active)
        .map(|member| M1CommittedSpeculativeRearmMemberAuthorityV1 {
            request: member.request(),
            anchor: member.next_draft_anchor(),
            target_committed: member.target_settlement().commit_end(),
            draft_committed: member.draft_settlement().commit_end(),
        });
    if let Err(source) =
        validate_committed_speculative_rearm_input(&input, outcome_next_roster, authorities)
    {
        return Err(
            M1ServingPhysicalRunnerSpeculativeRearmEnqueueFailureV1::Unavailable { source, input },
        );
    }
    let Some(provider) = provider.as_mut() else {
        return Err(
            M1ServingPhysicalRunnerSpeculativeRearmEnqueueFailureV1::Unavailable {
                source: M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1::ProviderUnavailable,
                input,
            },
        );
    };
    provider
        .try_enqueue_speculative_rearm(input)
        .map_err(|(source, input)| {
            M1ServingPhysicalRunnerSpeculativeRearmEnqueueFailureV1::Provider { source, input }
        })
}

#[allow(clippy::too_many_arguments)]
fn validate_speculative_rearm_enqueue(
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
    if active_plan != Some(custody_plan) || !supports_evidence_bound_speculation(custody_plan) {
        return Err(Unavailable::UnsupportedTransition);
    }
    if outcome_selection != custody_plan.target() || outcome_epoch != custody_epoch {
        return Err(Unavailable::CommitOutcomeMismatch);
    }
    let Some(next_epoch) = custody_epoch.value().checked_add(1) else {
        return Err(Unavailable::BindingMismatch);
    };
    if outcome_next_roster.is_empty()
        || outcome_next_roster.len() > custody_plan.sequence_capacity()
        || binding.plan() != custody_plan
        || binding.epoch() != CompletionEpoch::new(next_epoch)
        || binding.requests() != outcome_next_roster
    {
        return Err(Unavailable::BindingMismatch);
    }
    Ok(())
}

fn validate_committed_speculative_rearm_input(
    input: &M1ServingQueuedSameShapeRearmV1,
    expected_roster: &[RequestId],
    mut authorities: impl Iterator<Item = M1CommittedSpeculativeRearmMemberAuthorityV1>,
) -> Result<(), M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1> {
    use M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1 as Unavailable;

    if expected_roster.is_empty() {
        return Err(Unavailable::BindingMismatch);
    }
    for (lane, request) in expected_roster.iter().copied().enumerate() {
        let Some(authority) = authorities.next() else {
            return Err(Unavailable::CommitOutcomeMismatch);
        };
        if authority.request != request {
            return Err(Unavailable::CommitOutcomeMismatch);
        }
        let Some(anchor) = authority.anchor else {
            return Err(Unavailable::CommitOutcomeMismatch);
        };
        let member = M1ServingCommittedSpeculativeMemberBindingV1 {
            request,
            epoch: input.binding().epoch(),
            anchor,
            target_committed: authority.target_committed,
            draft_committed: authority.draft_committed,
        };
        if !input.matches_committed_speculative_member(lane, expected_roster.len(), &member) {
            return Err(Unavailable::CommittedInputMismatch);
        }
    }
    if authorities.next().is_some() {
        return Err(Unavailable::CommitOutcomeMismatch);
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
    if admit_m1_production_rollover_transition_v1(prior, binding.plan()).is_none() {
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
    supports_direct_serving(plan) || supports_evidence_bound_speculation(plan)
}

fn supports_same_shape_rearm(plan: M1ServingPlanV1) -> bool {
    plan.shape() == M1PhysicalFixedBatchShapeV1::TargetOnly
        || supports_evidence_bound_speculation(plan)
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
            supports_evidence_bound_speculation(plan)
                && expected_lanes > 0
                && expected_lanes <= plan.sequence_capacity()
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
        let transition = admit_m1_production_rollover_transition_v1(prior, next);
        let exact_s1_k4 = supports_evidence_bound_s1_k4(next);
        let supports_wider = self.provider.as_ref().is_some_and(
            M1ServingPhysicalInputProviderV1::supports_wider_finite_speculative_rollover,
        );
        if transition.is_none_or(|transition| transition.reason() != reason)
            || (!exact_s1_k4 && !supports_wider)
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
            match schedule_m1_finite_speculative_queue_rollover_v1(self.engine, released, batch) {
                Ok(scheduled) => scheduled,
                Err(failure) if !failure.is_terminal() => {
                    let M1FiniteSpeculativeQueueRolloverScheduleFailureCustodyV1::Released(
                        released,
                    ) = failure.into_custody()
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
        let provider = self
            .provider
            .as_mut()
            .expect("provider presence checked before rollover scheduling");
        let prepared = match if exact_s1_k4 {
            provider.prepare_s1_k4_rollover(self.runner, self.engine, batch, scheduled)
        } else {
            provider.prepare_finite_speculative_rollover(self.runner, self.engine, batch, scheduled)
        } {
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
        let M1ServingPreparedFiniteSpeculativeRolloverV1 {
            prepared,
            recipe,
            semantic_evidence,
        } = prepared;
        let published = match self.runner.submit_finite_speculative_rollover(
            self.engine,
            self.ring_bytes,
            prepared,
            recipe,
        ) {
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
        if published.shape() != next.shape() || published.rollover_observation().is_none() {
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
                    ) if supports_evidence_bound_s1_k4(batch.plan()) => {
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
                    (
                        M1PhysicalFixedBatchShapeV1::SpeculativeK4
                        | M1PhysicalFixedBatchShapeV1::SpeculativeK8
                        | M1PhysicalFixedBatchShapeV1::SpeculativeK16,
                        semantic_evidence @ M1ServingPreparedSemanticEvidenceV1::SpeculativeK4,
                    ) => {
                        let diagnostic = match observed.observe_speculative_diagnostic_choices() {
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
                    ) if supports_evidence_bound_s1_k4(batch.plan()) => {
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
                    (
                        M1PhysicalFixedBatchShapeV1::SpeculativeK4
                        | M1PhysicalFixedBatchShapeV1::SpeculativeK8
                        | M1PhysicalFixedBatchShapeV1::SpeculativeK16,
                        semantic_evidence @ M1ServingPreparedSemanticEvidenceV1::SpeculativeK4,
                    ) => {
                        let joined = match recycled
                            .read_and_check_speculative_diagnostic_completion()
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
        let diagnostic_binding = match prepare_diagnostic_binding(
            custody.plan(),
            readback_epoch,
            self.checked_completion(&custody),
        ) {
            Ok(binding) => binding,
            Err(_) => {
                return Err(M1ServingPhysicalOperationFailureV1::Retryable {
                    source: M1ServingPhysicalRunnerOperationErrorV1::DiagnosticHistoryCapacity,
                    custody,
                });
            }
        };
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
                                evidence.append_diagnostic_history(
                                    &mut diagnostic_history,
                                    diagnostic_binding,
                                );
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
                        evidence
                            .append_diagnostic_history(&mut diagnostic_history, diagnostic_binding);
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

    fn validate_s1_k4_rollover_anchor(
        input: &M1ServingQueuedS1K4RolloverV1,
        semantics: crate::CheckedCompletionSemantics,
    ) -> Result<(), M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1> {
        let crate::CheckedCompletionSemantics::DirectFinalRow { token } = semantics else {
            return Err(
                M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1::UnsupportedTransition,
            );
        };
        if !input.matches_anchor(token) {
            return Err(M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1::AnchorMismatch);
        }
        Ok(())
    }

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

    fn queued_speculative_rearm_test_input(
        plan: M1ServingPlanV1,
        requests: &[RequestId],
        epoch: CompletionEpoch,
        anchors: &[ferric_spec::TokenId],
        target_committed: &[u32],
        draft_committed: &[u32],
        target_future_override: Option<(usize, usize, ferric_spec::TokenId)>,
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
            selection: Qwen3PlanSelection,
            requests: &[RequestId],
            epoch: CompletionEpoch,
            identity_byte: u8,
            tokens: Vec<u32>,
            positions: Vec<u32>,
            committed: &[u32],
        ) -> ValidatedM1StepInputs {
            let dimensions = selection
                .bucket
                .dimensions(selection.role, selection.mode)
                .expect("canonical test selection has dimensions");
            let capacity = dimensions.sequences as usize;
            let mut lanes = Vec::with_capacity(capacity);
            for lane in 0..capacity {
                lanes.push(requests.get(lane).copied().map(|request| {
                    StepPlan::new(
                        request,
                        epoch,
                        Identity::new([identity_byte; 32]),
                        selection,
                    )
                }));
            }
            let mut active_lengths = vec![0; capacity];
            active_lengths[..requests.len()].fill(dimensions.active_tokens);
            let mut context_lengths = vec![0; capacity];
            context_lengths[..committed.len()].copy_from_slice(committed);
            let candidate = M1StepInputCandidate::new(
                selection,
                lanes,
                tokens,
                positions,
                active_lengths,
                context_lengths,
            );
            match validate_m1_step_inputs(candidate) {
                M1StepInputValidationOutcome::Validated(inputs) => inputs,
                M1StepInputValidationOutcome::Rejected(failure) => {
                    panic!(
                        "host-only speculative rearm input rejected: {:?}",
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

        assert_eq!(requests.len(), anchors.len());
        assert_eq!(requests.len(), target_committed.len());
        assert_eq!(requests.len(), draft_committed.len());
        let target_dimensions = plan
            .target()
            .bucket
            .dimensions(plan.target().role, plan.target().mode)
            .expect("canonical target selection has dimensions");
        let draft_dimensions = plan
            .draft()
            .bucket
            .dimensions(plan.draft().role, plan.draft().mode)
            .expect("canonical draft selection has dimensions");
        let target_width = target_dimensions.active_tokens as usize;
        let draft_width = draft_dimensions.active_tokens as usize;
        let mut draft_tokens = vec![0; draft_dimensions.sequences as usize * draft_width];
        let mut draft_positions = vec![0; draft_tokens.len()];
        let mut target_tokens = vec![0; target_dimensions.sequences as usize * target_width];
        let mut target_positions = vec![0; target_tokens.len()];
        for lane in 0..requests.len() {
            draft_tokens[lane * draft_width] = anchors[lane];
            for column in 0..draft_width {
                draft_positions[lane * draft_width + column] = draft_committed[lane]
                    + u32::try_from(column).expect("finite speculative width fits u32");
            }
            target_tokens[lane * target_width] = anchors[lane];
            for column in 0..target_width {
                target_positions[lane * target_width + column] = target_committed[lane]
                    + u32::try_from(column).expect("finite speculative width fits u32");
            }
        }
        if let Some((lane, column, token)) = target_future_override {
            target_tokens[lane * target_width + column] = token;
        }
        let draft = input(
            plan.draft(),
            requests,
            epoch,
            51,
            draft_tokens,
            draft_positions,
            draft_committed,
        );
        let target = input(
            plan.target(),
            requests,
            epoch,
            52,
            target_tokens,
            target_positions,
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
            M1ServingQueuedGenerationBindingV1::new(
                plan,
                requests.to_vec().into_boxed_slice(),
                epoch,
            ),
            crate::M1LongLivedQueueRearmKvInputsV1::speculative_round(
                draft,
                target,
                (0..requests.len()).map(|_| Vec::new()).collect(),
                (0..requests.len()).map(|_| Vec::new()).collect(),
            ),
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
        let plan = serving_plan(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        );
        queued_speculative_rearm_test_input(
            plan,
            &[request],
            epoch,
            &[anchor],
            &[target_committed],
            &[draft_committed],
            (target_future_token != 0).then_some((0, 1, target_future_token)),
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
            validate_speculative_rearm_enqueue(
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
            validate_speculative_rearm_enqueue(
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
            validate_speculative_rearm_enqueue(
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
            validate_speculative_rearm_enqueue(
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
            validate_speculative_rearm_enqueue(
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
            validate_speculative_rearm_enqueue(
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
    fn speculative_rearm_admits_nonempty_rosters_up_to_shape_capacity() {
        use M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1 as Unavailable;

        let identity = M1ServingPhysicalRunnerAdapterIdentityV1::fresh()
            .expect("test process has adapter identities");
        let plan = serving_plan(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS8K4C8192,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS8C8192,
        );
        let epoch = CompletionEpoch::new(2);
        let requests: Vec<_> = (0..8).map(|lane| RequestId::new(100 + lane, 1)).collect();
        for live in [1, 3, 8] {
            let binding = M1ServingQueuedGenerationBindingV1::new(
                plan,
                requests[..live].to_vec().into_boxed_slice(),
                CompletionEpoch::new(3),
            );
            assert_eq!(
                validate_speculative_rearm_enqueue(
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
                    &requests[..live],
                    &binding,
                ),
                Ok(())
            );
        }

        for roster in [&requests[..0], &requests[..8]] {
            let bound_requests = if roster.is_empty() {
                Vec::new()
            } else {
                let mut over_capacity = roster.to_vec();
                over_capacity.push(RequestId::new(108, 1));
                over_capacity
            };
            let binding = M1ServingQueuedGenerationBindingV1::new(
                plan,
                bound_requests.clone().into_boxed_slice(),
                CompletionEpoch::new(3),
            );
            assert_eq!(
                validate_speculative_rearm_enqueue(
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
                    &bound_requests,
                    &binding,
                ),
                Err(Unavailable::BindingMismatch)
            );
        }
    }

    #[test]
    fn generic_rearm_enqueue_filters_inactive_members_and_inserts_s8_successor() {
        let identity = M1ServingPhysicalRunnerAdapterIdentityV1::fresh()
            .expect("test process has adapter identities");
        let plan = serving_plan(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS8K4C8192,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS8C8192,
        );
        let completed_epoch = CompletionEpoch::new(2);
        let next_epoch = CompletionEpoch::new(3);
        let first = RequestId::new(0, 1);
        let retired = RequestId::new(1, 1);
        let third = RequestId::new(2, 1);
        let active_roster = [first, third];
        let members = [
            crate::M1SpeculativeMemberRoundOutcomeV1::for_serving_rearm_test(
                first,
                M1SpeculativeMemberStatusV1::Active,
                Some(501),
                130,
                129,
            ),
            crate::M1SpeculativeMemberRoundOutcomeV1::for_serving_rearm_test(
                retired,
                M1SpeculativeMemberStatusV1::Cancelled(
                    crate::M1SpeculativeCancellationReasonV1::ServerShutdown,
                ),
                None,
                230,
                229,
            ),
            crate::M1SpeculativeMemberRoundOutcomeV1::for_serving_rearm_test(
                third,
                M1SpeculativeMemberStatusV1::Active,
                Some(503),
                330,
                329,
            ),
        ];
        let input = Box::new(queued_speculative_rearm_test_input(
            plan,
            &active_roster,
            next_epoch,
            &[501, 503],
            &[130, 330],
            &[129, 329],
            None,
        ));
        let mut provider = Some(M1QueuedServingPhysicalInputProviderV1::new());

        try_enqueue_committed_speculative_rearm(
            &mut provider,
            identity,
            M1ServingPhysicalRunnerAdapterPhaseV1::Quiescent {
                epoch: completed_epoch,
            },
            Some(plan),
            identity,
            completed_epoch,
            plan,
            plan.target(),
            completed_epoch,
            &active_roster,
            &members,
            input,
        )
        .expect("active S8 members authorize one exact queued successor");

        let provider = provider.expect("successful enqueue retains provider");
        assert_eq!(provider.pending_generation_count(), 1);
        assert_eq!(
            provider.next_generation_phase(),
            Some(crate::M1ServingQueuedGenerationPhaseV1::SameShapeRearm)
        );
        let mut pending = provider.into_pending_inputs();
        let Some(crate::M1ServingQueuedGenerationInputV1::SameShapeRearm(queued)) =
            pending.pop_front()
        else {
            panic!("generic enqueue must insert the typed same-shape successor")
        };
        assert_eq!(queued.binding().plan(), plan);
        assert_eq!(queued.binding().epoch(), next_epoch);
        assert_eq!(queued.binding().requests(), &active_roster);
        assert!(pending.is_empty());
    }

    #[test]
    fn dynamic_s1_k4_rearm_requires_committed_anchor_and_role_cursors() {
        use M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1 as Unavailable;

        let request = RequestId::new(7, 1);
        let epoch = CompletionEpoch::new(3);
        let exact = queued_s1_k4_rearm_test_input(request, epoch, 900, 133, 132, 0);
        let authority = M1CommittedSpeculativeRearmMemberAuthorityV1 {
            request,
            anchor: Some(900),
            target_committed: 133,
            draft_committed: 132,
        };
        assert_eq!(
            validate_committed_speculative_rearm_input(&exact, &[request], [authority].into_iter(),),
            Ok(())
        );

        let substituted_anchor = queued_s1_k4_rearm_test_input(request, epoch, 901, 133, 132, 0);
        assert_eq!(
            validate_committed_speculative_rearm_input(
                &substituted_anchor,
                &[request],
                [authority].into_iter(),
            ),
            Err(Unavailable::CommittedInputMismatch)
        );
        let substituted_cursor = queued_s1_k4_rearm_test_input(request, epoch, 900, 134, 132, 0);
        assert_eq!(
            validate_committed_speculative_rearm_input(
                &substituted_cursor,
                &[request],
                [authority].into_iter(),
            ),
            Err(Unavailable::CommittedInputMismatch)
        );
        let substituted_draft_cursor =
            queued_s1_k4_rearm_test_input(request, epoch, 900, 133, 131, 0);
        assert_eq!(
            validate_committed_speculative_rearm_input(
                &substituted_draft_cursor,
                &[request],
                [authority].into_iter(),
            ),
            Err(Unavailable::CommittedInputMismatch)
        );
        let nonzero_future_placeholder =
            queued_s1_k4_rearm_test_input(request, epoch, 900, 133, 132, 1);
        assert_eq!(
            validate_committed_speculative_rearm_input(
                &nonzero_future_placeholder,
                &[request],
                [authority].into_iter(),
            ),
            Err(Unavailable::CommittedInputMismatch)
        );
        assert_eq!(
            validate_committed_speculative_rearm_input(
                &exact,
                &[request],
                [M1CommittedSpeculativeRearmMemberAuthorityV1 {
                    request: RequestId::new(8, 1),
                    ..authority
                }]
                .into_iter(),
            ),
            Err(Unavailable::CommitOutcomeMismatch)
        );
        assert_eq!(
            validate_committed_speculative_rearm_input(
                &exact,
                &[request],
                [M1CommittedSpeculativeRearmMemberAuthorityV1 {
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
    fn multi_lane_speculative_rearm_binds_each_roster_member_anchor_and_cursor() {
        use M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1 as Unavailable;

        let plan = serving_plan(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS8K4C8192,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS8C8192,
        );
        let requests = [
            RequestId::new(7, 1),
            RequestId::new(8, 1),
            RequestId::new(9, 1),
        ];
        let epoch = CompletionEpoch::new(3);
        let anchors = [900, 901, 902];
        let target_committed = [133, 211, 377];
        let draft_committed = [132, 210, 376];
        let authorities = [
            M1CommittedSpeculativeRearmMemberAuthorityV1 {
                request: requests[0],
                anchor: Some(anchors[0]),
                target_committed: target_committed[0],
                draft_committed: draft_committed[0],
            },
            M1CommittedSpeculativeRearmMemberAuthorityV1 {
                request: requests[1],
                anchor: Some(anchors[1]),
                target_committed: target_committed[1],
                draft_committed: draft_committed[1],
            },
            M1CommittedSpeculativeRearmMemberAuthorityV1 {
                request: requests[2],
                anchor: Some(anchors[2]),
                target_committed: target_committed[2],
                draft_committed: draft_committed[2],
            },
        ];
        let exact = queued_speculative_rearm_test_input(
            plan,
            &requests,
            epoch,
            &anchors,
            &target_committed,
            &draft_committed,
            None,
        );
        assert_eq!(
            validate_committed_speculative_rearm_input(&exact, &requests, authorities.into_iter(),),
            Ok(())
        );

        let full_requests: Vec<_> = (0..8).map(|lane| RequestId::new(10 + lane, 1)).collect();
        let full_anchors: Vec<_> = (0_u32..8).map(|lane| 1_000 + lane).collect();
        let full_target_committed: Vec<_> = (0_u32..8).map(|lane| 200 + lane).collect();
        let full_draft_committed: Vec<_> = (0_u32..8).map(|lane| 190 + lane).collect();
        let full_authorities: Vec<_> = (0..8)
            .map(|lane| M1CommittedSpeculativeRearmMemberAuthorityV1 {
                request: full_requests[lane],
                anchor: Some(full_anchors[lane]),
                target_committed: full_target_committed[lane],
                draft_committed: full_draft_committed[lane],
            })
            .collect();
        let full = queued_speculative_rearm_test_input(
            plan,
            &full_requests,
            epoch,
            &full_anchors,
            &full_target_committed,
            &full_draft_committed,
            None,
        );
        assert_eq!(
            validate_committed_speculative_rearm_input(
                &full,
                &full_requests,
                full_authorities.into_iter(),
            ),
            Ok(())
        );

        let mut swapped = authorities;
        swapped.swap(0, 1);
        assert_eq!(
            validate_committed_speculative_rearm_input(&exact, &requests, swapped.into_iter(),),
            Err(Unavailable::CommitOutcomeMismatch)
        );
        let mut missing_anchor = authorities;
        missing_anchor[1].anchor = None;
        assert_eq!(
            validate_committed_speculative_rearm_input(
                &exact,
                &requests,
                missing_anchor.into_iter(),
            ),
            Err(Unavailable::CommitOutcomeMismatch)
        );
        assert_eq!(
            validate_committed_speculative_rearm_input(
                &exact,
                &requests,
                authorities.into_iter().chain([authorities[0]].into_iter()),
            ),
            Err(Unavailable::CommitOutcomeMismatch)
        );
        let regenerated = [RequestId::new(7, 2), requests[1], requests[2]];
        assert_eq!(
            validate_committed_speculative_rearm_input(
                &exact,
                &regenerated,
                authorities.into_iter(),
            ),
            Err(Unavailable::CommitOutcomeMismatch)
        );

        let substituted_anchor = queued_speculative_rearm_test_input(
            plan,
            &requests,
            epoch,
            &[anchors[0], 999, anchors[2]],
            &target_committed,
            &draft_committed,
            None,
        );
        assert_eq!(
            validate_committed_speculative_rearm_input(
                &substituted_anchor,
                &requests,
                authorities.into_iter(),
            ),
            Err(Unavailable::CommittedInputMismatch)
        );
        let substituted_cursor = queued_speculative_rearm_test_input(
            plan,
            &requests,
            epoch,
            &anchors,
            &[target_committed[0], target_committed[1], 378],
            &draft_committed,
            None,
        );
        assert_eq!(
            validate_committed_speculative_rearm_input(
                &substituted_cursor,
                &requests,
                authorities.into_iter(),
            ),
            Err(Unavailable::CommittedInputMismatch)
        );
        let nonzero_future_placeholder = queued_speculative_rearm_test_input(
            plan,
            &requests,
            epoch,
            &anchors,
            &target_committed,
            &draft_committed,
            Some((1, 4, 77)),
        );
        assert_eq!(
            validate_committed_speculative_rearm_input(
                &nonzero_future_placeholder,
                &requests,
                authorities.into_iter(),
            ),
            Err(Unavailable::CommittedInputMismatch)
        );
    }

    #[test]
    fn k8_and_k16_rearm_validate_their_full_target_placeholder_widths() {
        use M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1 as Unavailable;

        let request = RequestId::new(7, 1);
        let epoch = CompletionEpoch::new(3);
        let authority = M1CommittedSpeculativeRearmMemberAuthorityV1 {
            request,
            anchor: Some(900),
            target_committed: 133,
            draft_committed: 132,
        };
        for (target_bucket, final_column) in [
            (Qwen3PlanBucket::SpeculativeS1K8C8192, 8),
            (Qwen3PlanBucket::SpeculativeS1K16C8192, 16),
        ] {
            let plan = serving_plan(
                Qwen3ExecutionMode::Speculative,
                target_bucket,
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS1C8192,
            );
            let exact = queued_speculative_rearm_test_input(
                plan,
                &[request],
                epoch,
                &[900],
                &[133],
                &[132],
                None,
            );
            assert_eq!(
                validate_committed_speculative_rearm_input(
                    &exact,
                    &[request],
                    [authority].into_iter(),
                ),
                Ok(())
            );
            let nonzero_final_placeholder = queued_speculative_rearm_test_input(
                plan,
                &[request],
                epoch,
                &[900],
                &[133],
                &[132],
                Some((0, final_column, 77)),
            );
            assert_eq!(
                validate_committed_speculative_rearm_input(
                    &nonzero_final_placeholder,
                    &[request],
                    [authority].into_iter(),
                ),
                Err(Unavailable::CommittedInputMismatch)
            );
        }
    }

    #[test]
    fn legacy_s1_k4_rearm_plan_gate_rejects_every_wider_shape() {
        use M1ServingPhysicalRunnerGenerationEnqueueUnavailableV1 as Unavailable;

        let s1_k4 = serving_plan(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        );
        assert_eq!(validate_s1_k4_rearm_compatibility_plan(s1_k4), Ok(()));
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
            assert_eq!(
                validate_s1_k4_rearm_compatibility_plan(plan),
                Err(Unavailable::UnsupportedTransition)
            );
        }
    }

    #[test]
    fn diagnostic_bindings_preserve_lane_attribution_across_roster_shrink() {
        use ferric_qwen_kernels::logits::Qwen3LogitsCompactRecordLayoutV1 as Layout;
        use ferric_spec::{Identity, StepPlan};

        fn checked_output(
            selection: Qwen3PlanSelection,
            epoch: CompletionEpoch,
            requests: &[RequestId],
        ) -> M1CheckedCompletionOutputV1 {
            let plan_id = Identity::new([91; 32]);
            let draft_tokens: [ferric_spec::TokenId; 4] = [31, 32, 33, 34];
            let target_choices: [ferric_spec::TokenId; 5] = [41, 42, 43, 44, 45];
            let records = requests
                .iter()
                .map(|request| {
                    let mut bytes = [0; Layout::RECORD_BYTES_USIZE];
                    bytes[Layout::REQUEST_SLOT_OFFSET..Layout::REQUEST_SLOT_OFFSET + 4]
                        .copy_from_slice(&request.slot().to_le_bytes());
                    bytes[Layout::REQUEST_GENERATION_OFFSET..Layout::REQUEST_GENERATION_OFFSET + 4]
                        .copy_from_slice(&request.generation().to_le_bytes());
                    bytes[Layout::COMPLETION_EPOCH_OFFSET..Layout::COMPLETION_EPOCH_OFFSET + 8]
                        .copy_from_slice(&epoch.value().to_le_bytes());
                    bytes[Layout::PLAN_IDENTITY_OFFSET
                        ..Layout::PLAN_IDENTITY_OFFSET + Layout::PLAN_IDENTITY_BYTES]
                        .copy_from_slice(plan_id.as_bytes());
                    bytes[Layout::EMITTED_TOKEN_COUNT_OFFSET] = 1;
                    let token_offset = Layout::token_offset(0).expect("first token slot exists");
                    bytes[token_offset..token_offset + 4]
                        .copy_from_slice(&target_choices[0].to_le_bytes());
                    let step = StepPlan::new(*request, epoch, plan_id, selection);
                    crate::check_inert_completion_record(
                        &bytes,
                        crate::CompletionWireExpectation::new(
                            &step,
                            crate::CompletionWireSemanticExpectation::Speculative {
                                draft_tokens: &draft_tokens,
                                target_choices: &target_choices,
                            },
                        ),
                    )
                    .expect("synthetic K7 bytes satisfy the independent S8/K4 expectation")
                })
                .collect::<Vec<_>>()
                .into_boxed_slice();
            M1CheckedCompletionOutputV1::for_serving_history_test(selection, epoch, records)
        }

        let plan = serving_plan(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS8K4C8192,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS8C8192,
        );
        let first = RequestId::new(7, 1);
        let second = RequestId::new(8, 1);
        let third = RequestId::new(9, 1);
        let first_epoch = CompletionEpoch::new(2);
        let second_epoch = CompletionEpoch::new(3);
        let first_checked = checked_output(plan.target(), first_epoch, &[first, second, third]);
        let second_checked = checked_output(plan.target(), second_epoch, &[first, third]);
        let mut history = M1ServingPhysicalRunnerDiagnosticHistoryV1::new();
        for (epoch, checked) in [
            (first_epoch, &first_checked),
            (second_epoch, &second_checked),
        ] {
            let binding = prepare_diagnostic_binding(plan, epoch, checked)
                .expect("bounded checked roster fits diagnostic history");
            let choices = vec![41; checked.records().len()].into_boxed_slice();
            history
                .try_reserve_exact(1)
                .expect("two diagnostic entries fit host memory");
            M1ServingPhysicalRunnerReadbackEvidenceV1::Direct(
                crate::M1ObservedDirectDiagnosticChoicesV1::for_serving_history_test(choices),
            )
            .append_diagnostic_history(&mut history, binding);
        }
        let bindings = history.bindings();
        assert_eq!(history.evidence().len(), 2);
        assert_eq!(bindings[0].plan(), plan);
        assert_eq!(bindings[0].epoch(), first_epoch);
        assert_eq!(bindings[0].requests(), &[first, second, third]);
        assert_eq!(bindings[1].epoch(), second_epoch);
        assert_eq!(bindings[1].requests(), &[first, third]);
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
    fn legacy_s1_k4_failure_variants_remain_importable_from_the_enum() {
        use M1ServingPhysicalRunnerS1K4RearmEnqueueFailureV1::{Provider, Unavailable};

        fn exhaustively_match(value: M1ServingPhysicalRunnerS1K4RearmEnqueueFailureV1) {
            match value {
                Unavailable { .. } | Provider { .. } => {}
            }
        }

        let _ = exhaustively_match as fn(M1ServingPhysicalRunnerS1K4RearmEnqueueFailureV1);
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
    fn all_finite_speculative_shapes_are_admitted_with_exact_evidence_contracts() {
        let s1_k4 = serving_plan(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        );
        assert!(supports_evidence_bound_s1_k4(s1_k4));
        assert!(supports_evidence_bound_speculation(s1_k4));
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
            assert!(!supports_evidence_bound_s1_k4(plan));
            assert!(supports_evidence_bound_speculation(plan));
            assert!(supports_serving_plan(plan));
            assert!(supports_same_shape_rearm(plan));
            assert!(prepared_semantic_evidence_matches(
                plan,
                1,
                &M1ServingPreparedSemanticEvidenceV1::SpeculativeK4,
            ));
            assert!(prepared_semantic_evidence_matches(
                plan,
                plan.sequence_capacity(),
                &M1ServingPreparedSemanticEvidenceV1::SpeculativeK4,
            ));
            assert!(!prepared_semantic_evidence_matches(
                plan,
                0,
                &M1ServingPreparedSemanticEvidenceV1::SpeculativeK4,
            ));
            assert!(!prepared_semantic_evidence_matches(
                plan,
                plan.sequence_capacity() + 1,
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
    fn configured_mi300x_bridge_runs_output_fed_s1_k8_rollover_and_rearm() {
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
            bind_m1_kv_workspace_table_v1, bind_structural_m1_physical_runner_v1,
            initialize_m1_physical_runner_memory_v1, reopen_persisted_m1_kernel_artifacts_v1,
            ActiveDeviceKvCache, Engine, M1FiniteSpeculativeQueueRolloverKvInputsV1,
            M1FullStepKvWorkspaceTablesV1, M1FullStepWorkspacePlans,
            M1QueuedServingPhysicalInputProviderV1, M1RearmRoundHistoryEntryV1,
            M1ServingCompletionDispositionV1, M1ServingPhysicalQueueCustodyV1,
            M1ServingQueueActionV1, M1ServingQueuedFiniteSpeculativeRolloverV1,
            M1ServingQueuedFirstPublicationV1, M1ServingQueuedGenerationBindingV1,
            M1ServingQueuedGenerationInputV1, M1ServingQueuedSameShapeRearmV1, M1ServingRegistryV1,
            M1SpeculativeCancellationReasonV1, M1SpeculativeGenerationLoopV1,
            M1SpeculativeGenerationPolicyV1, M1SpeculativeMemberControlV1,
            M1SpeculativeMemberSeedV1,
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

        // This output-fed ignored smoke observes only typed bridge lifecycle,
        // native queue rollover, and same-shape rearm structure. Fixture artifacts do not
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
        let runner = bind_structural_m1_physical_runner_v1(artifacts, publication)
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
            Qwen3PlanBucket::SpeculativeS1K8C8192,
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
        let physical = M1ServingPhysicalQueueCustodyV1::Vacant;

        let prefill_batch = registry
            .plan_next()
            .expect("plan paired prefill")
            .expect("paired prefill is ready");
        let prefill_reservation = registry
            .reserve_publication(prefill_batch)
            .expect("reserve paired-prefill publication");
        let prefill_batch = prefill_reservation.physical_batch();
        assert_eq!(prefill_batch.action(), M1ServingQueueActionV1::FreshLaunch);
        let published = physical
            .publish(prefill_reservation, &mut registry, &mut operations)
            .expect("publish paired prefill through the generic serving bridge");
        let readback = published
            .read_physical(prefill_epoch, &mut operations)
            .expect("read paired-prefill completion through the generic serving bridge");
        let anchor = {
            let checked = readback.checked(&operations);
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
            vec![anchor, 0, 0, 0, 0, 0, 0, 0, 0],
            (128..137).collect(),
            9,
            128,
        );
        let draft_decode_inputs = input(draft_decode_plan, vec![anchor], vec![128], 1, 128);
        let rollover_inputs = M1FiniteSpeculativeQueueRolloverKvInputsV1::new(
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
        let rollover = M1ServingQueuedFiniteSpeculativeRolloverV1::new(
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
            .try_enqueue_finite_speculative_rollover_after_readback(&readback, Box::new(rollover))
            .expect("enqueue output-fed S1/K8 successor after paired-prefill readback");
        assert_eq!(
            operations
                .provider()
                .expect("adapter retains queued provider")
                .pending_generation_count(),
            1
        );
        let (_, physical) = readback
            .complete_exact(
                &mut registry,
                &[M1ServingCompletionDispositionV1::Continue(speculative)],
                &mut operations,
            )
            .expect("settle paired prefill and advance the registry through the bridge");
        operations
            .engine
            .append_tentative(request, 9)
            .expect("append one exact S1/K8 target span");

        let rollover_batch = registry
            .plan_next()
            .expect("plan exact S1/K8 successor")
            .expect("S1/K8 successor is ready");
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
            .expect("reserve exact S1/K8 rollover publication");
        let published = physical
            .publish(rollover_reservation, &mut registry, &mut operations)
            .expect("roll over into native S1/K8 through the generic serving bridge");
        let readback = published
            .read_physical(rollover_epoch, &mut operations)
            .expect("read the first native S1/K8 round through the generic bridge");
        let policy = M1SpeculativeGenerationPolicyV1::new(512, &[])
            .expect("construct nonterminal structural policy");
        let seed = M1SpeculativeMemberSeedV1::new(request, anchor, 128, 128, policy);
        let mut coordinator = M1SpeculativeGenerationLoopV1::new(speculative.target(), &[seed])
            .expect("construct S1/K8 speculative coordinator");
        let binding = coordinator
            .bind_round(0, rollover_epoch, &[request])
            .expect("bind first native S1/K8 round");
        let permit = coordinator
            .preflight_checked_round(
                binding,
                readback.checked(&operations),
                &[M1SpeculativeMemberControlV1::continuing(request)],
            )
            .expect("preflight actual first-round checked output");
        let committed = readback
            .commit_speculative(&mut registry, &mut coordinator, permit, &mut operations)
            .expect("atomically settle and commit the first speculative round");
        let [member] = committed.outcome().members() else {
            panic!("S1/K8 commit must retain one exact member outcome")
        };
        assert_eq!(member.status(), M1SpeculativeMemberStatusV1::Active);
        let next_anchor = member
            .next_draft_anchor()
            .expect("continuing first round must retain its actual next anchor");
        let target_committed = member.target_settlement().commit_end();
        let draft_committed = member.draft_settlement().commit_end();

        let rearm_epoch = CompletionEpoch::new(3);
        let target_rearm_plan = runner
            .logical_runner()
            .bind_step_plan(request, rearm_epoch, speculative.target())
            .expect("bind second-round target plan");
        let draft_rearm_plan = runner
            .logical_runner()
            .bind_step_plan(request, rearm_epoch, speculative.draft())
            .expect("bind second-round draft plan");
        let target_rearm_inputs = input(
            target_rearm_plan,
            vec![next_anchor, 0, 0, 0, 0, 0, 0, 0, 0],
            (target_committed..target_committed + 9).collect(),
            9,
            target_committed,
        );
        let draft_rearm_inputs = input(
            draft_rearm_plan,
            vec![next_anchor],
            vec![draft_committed],
            1,
            draft_committed,
        );
        let rearm_preparation_plans = M1FullStepWorkspacePlans::speculative_round(
            workspace_plan(speculative.draft(), 94),
            workspace_plan(speculative.target(), 95),
        );
        let rearm_recipe_plans = M1FullStepWorkspacePlans::speculative_round(
            workspace_plan(speculative.draft(), 94),
            workspace_plan(speculative.target(), 95),
        );
        let rearm = M1ServingQueuedSameShapeRearmV1::new(
            M1ServingQueuedGenerationBindingV1::new(
                speculative,
                vec![request].into_boxed_slice(),
                rearm_epoch,
            ),
            crate::M1LongLivedQueueRearmKvInputsV1::speculative_round(
                draft_rearm_inputs,
                target_rearm_inputs,
                vec![Vec::new()],
                vec![Vec::new()],
            ),
            rearm_preparation_plans,
            rearm_recipe_plans,
        );
        operations
            .try_enqueue_speculative_rearm_after_commit(&committed, Box::new(rearm))
            .expect("enqueue the exact committed output-fed same-shape rearm");
        assert_eq!(
            operations
                .provider()
                .expect("adapter retains queued provider")
                .pending_generation_count(),
            1
        );
        let (_, physical, first_outcome) = committed.into_parts();
        assert_eq!(first_outcome.completed_epoch(), rollover_epoch);
        assert_eq!(first_outcome.next_active_roster(), &[request]);
        operations
            .engine
            .append_tentative(request, 9)
            .expect("append the second exact S1/K8 target span");

        let rearm_batch = registry
            .plan_next()
            .expect("plan second exact S1/K8 generation")
            .expect("same-shape S1/K8 successor is ready");
        assert_eq!(rearm_batch.epoch(), rearm_epoch);
        assert_eq!(rearm_batch.action(), M1ServingQueueActionV1::SameShapeRearm);
        let rearm_reservation = registry
            .reserve_publication(rearm_batch)
            .expect("reserve second S1/K8 publication");
        let published = physical
            .publish(rearm_reservation, &mut registry, &mut operations)
            .expect("publish same-shape S1/K8 rearm through the generic bridge");
        let readback = published
            .read_physical(rearm_epoch, &mut operations)
            .expect("read the second native S1/K8 round through the generic bridge");
        let binding = coordinator
            .bind_round(1, rearm_epoch, &[request])
            .expect("bind second native S1/K8 round");
        let permit = coordinator
            .preflight_checked_round(
                binding,
                readback.checked(&operations),
                &[M1SpeculativeMemberControlV1::cancelling(
                    request,
                    M1SpeculativeCancellationReasonV1::ServerShutdown,
                )],
            )
            .expect("preflight actual second-round checked output");
        operations
            .engine
            .retire(request)
            .expect("mark the final Engine member retiring before atomic settlement");
        let committed = readback
            .commit_speculative(&mut registry, &mut coordinator, permit, &mut operations)
            .expect("atomically settle and commit the second speculative round");
        assert!(committed.outcome().next_active_roster().is_empty());
        assert!(matches!(
            committed.outcome().members(),
            [member]
                if member.status()
                    == M1SpeculativeMemberStatusV1::Cancelled(
                        M1SpeculativeCancellationReasonV1::ServerShutdown
                    )
        ));
        let (_, physical, second_outcome) = committed.into_parts();
        assert_eq!(second_outcome.completed_epoch(), rearm_epoch);
        assert_eq!(
            operations
                .provider()
                .expect("adapter retains queued provider")
                .pending_generation_count(),
            0
        );

        let M1ServingPhysicalQueueCustodyV1::Quiescent {
            plan: released_plan,
            custody: quiescent,
        } = physical
        else {
            panic!("second commit must retain quiescent native queue custody")
        };
        assert_eq!(released_plan, speculative);
        let M1ServingPhysicalRunnerQuiescentV1 { state, .. } = quiescent;
        let M1ServingPhysicalRunnerQuiescentStateV1::Rearmed {
            released,
            diagnostic_history,
        } = state
        else {
            panic!("second S1/K8 round did not return rearmed queue custody")
        };
        assert!(matches!(
            diagnostic_history.evidence(),
            [
                M1ServingPhysicalRunnerReadbackEvidenceV1::Direct(_),
                M1ServingPhysicalRunnerReadbackEvidenceV1::SpeculativeK4(_),
                M1ServingPhysicalRunnerReadbackEvidenceV1::SpeculativeK4(_)
            ]
        ));
        assert!(matches!(
            diagnostic_history.bindings(),
            [first, rollover, rearm]
                if first.plan() == prefill
                    && first.epoch() == prefill_epoch
                    && first.requests() == [request]
                    && rollover.plan() == speculative
                    && rollover.epoch() == rollover_epoch
                    && rollover.requests() == [request]
                    && rearm.plan() == speculative
                    && rearm.epoch() == rearm_epoch
                    && rearm.requests() == [request]
        ));
        assert_eq!(released.round_history_len(), 2);
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
        assert!(released
            .round_history(1)
            .expect("second round retains same-shape queue history")
            .rollover_observation()
            .is_none());

        let teardown = released
            .destroy_queue_and_retain_round(operations.engine)
            .expect("destroy replacement queue and retain exact round lineage");
        assert_eq!(teardown.round_history_len(), 2);
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
