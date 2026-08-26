//! Production physical-operation adapter for dynamic M1 serving.
//!
//! The registry intentionally owns no tokens, workspace images, page leases,
//! or active device-cache owners. Those inputs stay behind
//! [`M1ServingPhysicalInputProviderV1`], while this adapter alone performs the
//! exact scheduler transition and consumes the resulting physical typestates.
//! The initial implementation admits only S1/K4 speculative generations with
//! the independent diagnostic-choice attachment. Other serving shapes fail
//! before scheduler or queue progress.

use core::{
    fmt,
    sync::atomic::{AtomicU64, Ordering},
};

use ferric_spec::{completion::CompletionEpoch, Qwen3PlanBucket, RequestId};

use crate::{
    complete_m1_physical_step_v1, release_m1_completed_step_kv_pages_v1,
    schedule_m1_long_lived_queue_rearm_exact_v1, ActiveDeviceKvCache,
    AddresslessM1PhysicalBufferRecipeV1, BoundM1CompletionOutputV1, Engine,
    M1AllocatedScheduledStepV1, M1CheckedCompletionOutputV1, M1CompletedStepOutcomeV1,
    M1CompletedStepRejectionV1, M1DeviceKvCompletionDispositionV1, M1DeviceKvCompletionMemberV1,
    M1DeviceKvCompletionRosterV1, M1ExactDispatchErrorV1, M1LongLivedQueueReleasedRoundV1,
    M1LongLivedQueueUnscheduledRoundV1, M1ObservedSpeculativeDiagnosticChoicesV1,
    M1PhysicalCompletedReadbackV1, M1PhysicalFixedBatchShapeV1, M1PhysicalPublishedQueueSessionV1,
    M1PhysicalRunnerV1, M1PreparedLongLivedQueueRearmV1, M1RearmedCompletedReadbackV1,
    M1RearmedCompletionOutcomeV1, M1RearmedCompletionPreflightFailureV1, M1RearmedPublishedQueueV1,
    M1RearmedRoundReleaseOutcomeV1, M1ReleasedCompletedStepV1, M1ScheduledDispatchV1,
    M1ScheduledLongLivedQueueRearmV1, M1ServingBatchPlanV1, M1ServingPhysicalOperationFailureV1,
    M1ServingPhysicalOperationResultV1, M1ServingPhysicalOperationsV1, M1ServingPlanV1,
    M1ServingRolloverReasonV1, M1_MAX_REARM_ROUND_HISTORY_V1,
};

/// Request-owned inputs prepared after the adapter issues the exact first dispatch.
#[must_use = "prepared first-publication custody must publish or remain retained"]
#[derive(Debug)]
pub struct M1ServingPreparedFirstPublicationV1 {
    allocated: M1AllocatedScheduledStepV1,
    recipe: AddresslessM1PhysicalBufferRecipeV1,
    completion_output: BoundM1CompletionOutputV1,
    selected: Vec<ActiveDeviceKvCache>,
}

impl M1ServingPreparedFirstPublicationV1 {
    /// Joins provider-owned physical inputs without granting publication authority.
    pub fn new(
        allocated: M1AllocatedScheduledStepV1,
        recipe: AddresslessM1PhysicalBufferRecipeV1,
        completion_output: BoundM1CompletionOutputV1,
        selected: Vec<ActiveDeviceKvCache>,
    ) -> Self {
        Self {
            allocated,
            recipe,
            completion_output,
            selected,
        }
    }
}

/// Request-owned inputs prepared after exact same-shape rearm scheduling.
#[must_use = "prepared rearm custody must publish or remain retained"]
#[derive(Debug)]
pub struct M1ServingPreparedSameShapeRearmV1 {
    prepared: M1PreparedLongLivedQueueRearmV1,
    recipe: AddresslessM1PhysicalBufferRecipeV1,
}

impl M1ServingPreparedSameShapeRearmV1 {
    /// Joins the existing prepared-rearm typestate to its exact physical recipe.
    pub const fn new(
        prepared: M1PreparedLongLivedQueueRearmV1,
        recipe: AddresslessM1PhysicalBufferRecipeV1,
    ) -> Self {
        Self { prepared, recipe }
    }
}

/// Supplies request/model inputs that the serving registry deliberately does not own.
///
/// Implementations must retain every consumed scheduler, cache, lease, table,
/// workspace, and allocation owner inside `Failure` on rejection. The adapter
/// treats either failure as terminal because exact scheduling has already
/// advanced or detached the predecessor queue.
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
    choices: Vec<M1ObservedSpeculativeDiagnosticChoicesV1>,
}

impl M1ServingPhysicalRunnerDiagnosticHistoryV1 {
    fn new() -> Self {
        Self {
            choices: Vec::new(),
        }
    }

    fn len(&self) -> usize {
        self.choices.len()
    }

    fn try_reserve_exact(
        &mut self,
        additional: usize,
    ) -> Result<(), std::collections::TryReserveError> {
        self.choices.try_reserve_exact(additional)
    }

    fn push(&mut self, choices: M1ObservedSpeculativeDiagnosticChoicesV1) {
        self.choices.push(choices);
    }

    /// Borrows the settled choice evidence in generation order.
    pub fn choices(&self) -> &[M1ObservedSpeculativeDiagnosticChoicesV1] {
        &self.choices
    }
}

/// Complete quiescent physical custody returned after one serving settlement.
#[must_use = "quiescent queue, caches, and diagnostic evidence must remain retained"]
#[derive(Debug)]
pub struct M1ServingPhysicalRunnerQuiescentV1 {
    adapter_identity: M1ServingPhysicalRunnerAdapterIdentityV1,
    epoch: CompletionEpoch,
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

    fn first(
        adapter_identity: M1ServingPhysicalRunnerAdapterIdentityV1,
        epoch: CompletionEpoch,
        released: M1ReleasedCompletedStepV1,
        diagnostic_history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    ) -> Self {
        Self {
            adapter_identity,
            epoch,
            state: M1ServingPhysicalRunnerQuiescentStateV1::First {
                released,
                diagnostic_history,
            },
        }
    }

    fn rearmed(
        adapter_identity: M1ServingPhysicalRunnerAdapterIdentityV1,
        epoch: CompletionEpoch,
        released: M1LongLivedQueueReleasedRoundV1,
        diagnostic_history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    ) -> Self {
        Self {
            adapter_identity,
            epoch,
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
    state: M1ServingPhysicalRunnerPublishedStateV1,
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
enum M1ServingPhysicalRunnerPublishedStateV1 {
    First {
        published: M1PhysicalPublishedQueueSessionV1,
        selected: Vec<ActiveDeviceKvCache>,
        diagnostic_history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    },
    Rearmed {
        published: M1RearmedPublishedQueueV1,
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

/// Semantically joined readback retaining independent S1/K4 choice evidence.
#[must_use = "readback, caches, and diagnostic choices must settle or remain retained"]
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub struct M1ServingPhysicalRunnerReadbackV1 {
    adapter_identity: M1ServingPhysicalRunnerAdapterIdentityV1,
    epoch: CompletionEpoch,
    state: M1ServingPhysicalRunnerReadbackStateV1,
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
enum M1ServingPhysicalRunnerReadbackStateV1 {
    First {
        state: M1ServingFirstReadbackStateV1,
        choices: M1ObservedSpeculativeDiagnosticChoicesV1,
        diagnostic_history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    },
    Rearmed {
        state: M1ServingRearmedReadbackStateV1,
        choices: M1ObservedSpeculativeDiagnosticChoicesV1,
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

    fn first(
        adapter_identity: M1ServingPhysicalRunnerAdapterIdentityV1,
        epoch: CompletionEpoch,
        state: M1ServingFirstReadbackStateV1,
        choices: M1ObservedSpeculativeDiagnosticChoicesV1,
        diagnostic_history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    ) -> Self {
        Self {
            adapter_identity,
            epoch,
            state: M1ServingPhysicalRunnerReadbackStateV1::First {
                state,
                choices,
                diagnostic_history,
            },
        }
    }

    fn rearmed(
        adapter_identity: M1ServingPhysicalRunnerAdapterIdentityV1,
        epoch: CompletionEpoch,
        state: M1ServingRearmedReadbackStateV1,
        choices: M1ObservedSpeculativeDiagnosticChoicesV1,
        diagnostic_history: M1ServingPhysicalRunnerDiagnosticHistoryV1,
    ) -> Self {
        Self {
            adapter_identity,
            epoch,
            state: M1ServingPhysicalRunnerReadbackStateV1::Rearmed {
                state,
                choices,
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

/// Stable stage reported through the generic serving bridge.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1ServingPhysicalRunnerOperationErrorV1 {
    UnsupportedEvidenceShape,
    ExactFirstDispatch(M1ExactDispatchErrorV1),
    ProviderPreparation,
    SelectedRosterCount,
    FirstPublication,
    SameShapeSchedule,
    SameShapePublication,
    RolloverUnavailable,
    EpochMismatch,
    QueueWait,
    QueueRecycle,
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

/// Opaque exhaustive terminal custody retained at the exact failed stage.
#[must_use = "terminal physical custody must remain retained for teardown/diagnosis"]
pub struct M1ServingPhysicalRunnerTerminalCustodyV1<'a, P> {
    stage: M1ServingPhysicalRunnerOperationErrorV1,
    provider: Option<P>,
    custody: Box<dyn fmt::Debug + 'a>,
}

impl<P> fmt::Debug for M1ServingPhysicalRunnerTerminalCustodyV1<'_, P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1ServingPhysicalRunnerTerminalCustodyV1")
            .field("stage", &self.stage)
            .field("provider_retained", &self.provider.is_some())
            .field("custody", &self.custody)
            .finish()
    }
}

impl<'a, P> M1ServingPhysicalRunnerTerminalCustodyV1<'a, P> {
    #[must_use]
    pub const fn stage(&self) -> M1ServingPhysicalRunnerOperationErrorV1 {
        self.stage
    }

    /// Separates the stage, provider, and erased lower custody without dropping either owner.
    #[must_use = "terminal provider and lower custody must both remain retained"]
    pub fn into_parts(
        self,
    ) -> (
        M1ServingPhysicalRunnerOperationErrorV1,
        Option<P>,
        Box<dyn fmt::Debug + 'a>,
    ) {
        (self.stage, self.provider, self.custody)
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
        })
    }

    #[must_use]
    pub const fn provider(&self) -> Option<&P> {
        self.provider.as_ref()
    }

    fn terminal<Q, T: fmt::Debug + 'a>(
        &mut self,
        stage: M1ServingPhysicalRunnerOperationErrorV1,
        custody: T,
    ) -> M1ServingPhysicalOperationFailureV1<
        Q,
        M1ServingPhysicalRunnerTerminalCustodyV1<'a, P>,
        M1ServingPhysicalRunnerOperationErrorV1,
    > {
        self.engine.quarantine_m1_queue_rearm_failure();
        self.phase = M1ServingPhysicalRunnerAdapterPhaseV1::Sealed;
        M1ServingPhysicalOperationFailureV1::Terminal {
            source: stage,
            custody: M1ServingPhysicalRunnerTerminalCustodyV1 {
                stage,
                provider: self.provider.take(),
                custody: Box::new(custody),
            },
        }
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
    type TerminalCustody = M1ServingPhysicalRunnerTerminalCustodyV1<'a, P>;

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
            return Err(self.terminal(M1ServingPhysicalRunnerOperationErrorV1::AdapterSealed, ()));
        }
        if !phase_allows_fresh_launch(self.phase) {
            return Err(M1ServingPhysicalOperationFailureV1::Retryable {
                source: M1ServingPhysicalRunnerOperationErrorV1::PhaseMismatch,
                custody: (),
            });
        }
        if !supports_evidence_bound_s1_k4(batch.plan()) {
            return Err(M1ServingPhysicalOperationFailureV1::Retryable {
                source: M1ServingPhysicalRunnerOperationErrorV1::UnsupportedEvidenceShape,
                custody: (),
            });
        }
        let scheduled = match self
            .engine
            .dispatch_m1_exact_ready(batch.epoch(), batch.requests())
        {
            Ok(scheduled) => scheduled,
            Err(error) if exact_dispatch_failure_is_fail_stop(error) => {
                return Err(self.terminal(
                    M1ServingPhysicalRunnerOperationErrorV1::ExactFirstDispatch(error),
                    error,
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
                    M1ServingPhysicalRunnerOperationErrorV1::ProviderPreparation,
                    failure,
                ));
            }
        };
        if !exact_request_roster_matches(
            batch.requests(),
            prepared
                .selected
                .iter()
                .map(|cache| cache.projection().request),
        ) || prepared
            .completion_output
            .speculative_diagnostic_choices()
            .is_none()
        {
            return Err(self.terminal(
                M1ServingPhysicalRunnerOperationErrorV1::SelectedRosterCount,
                prepared,
            ));
        }
        let M1ServingPreparedFirstPublicationV1 {
            allocated,
            recipe,
            completion_output,
            selected,
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
                    M1ServingPhysicalRunnerOperationErrorV1::FirstPublication,
                    (failure, selected),
                ));
            }
        };
        if published.shape() != M1PhysicalFixedBatchShapeV1::SpeculativeK4 {
            return Err(self.terminal(
                M1ServingPhysicalRunnerOperationErrorV1::UnsupportedEvidenceShape,
                (published, selected),
            ));
        }
        self.phase = M1ServingPhysicalRunnerAdapterPhaseV1::Published {
            epoch: batch.epoch(),
        };
        Ok(M1ServingPhysicalRunnerPublishedV1 {
            adapter_identity: self.identity,
            epoch: batch.epoch(),
            state: M1ServingPhysicalRunnerPublishedStateV1::First {
                published,
                selected,
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
                M1ServingPhysicalRunnerOperationErrorV1::AdapterSealed,
                custody,
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
        if !supports_evidence_bound_s1_k4(batch.plan()) {
            return Err(M1ServingPhysicalOperationFailureV1::Retryable {
                source: M1ServingPhysicalRunnerOperationErrorV1::UnsupportedEvidenceShape,
                custody,
            });
        }
        let M1ServingPhysicalRunnerQuiescentV1 {
            adapter_identity,
            epoch: custody_epoch,
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
                                    state: M1ServingPhysicalRunnerQuiescentStateV1::Unscheduled {
                                        unscheduled,
                                        diagnostic_history,
                                    },
                                },
                            });
                        }
                        return Err(self.terminal(
                            M1ServingPhysicalRunnerOperationErrorV1::SameShapeSchedule,
                            (failure, diagnostic_history),
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
                                    state: M1ServingPhysicalRunnerQuiescentStateV1::Unscheduled {
                                        unscheduled,
                                        diagnostic_history,
                                    },
                                },
                            });
                        }
                        return Err(self.terminal(
                            M1ServingPhysicalRunnerOperationErrorV1::SameShapeSchedule,
                            (failure, diagnostic_history),
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
                                        state:
                                            M1ServingPhysicalRunnerQuiescentStateV1::Unscheduled {
                                                unscheduled,
                                                diagnostic_history,
                                            },
                                    },
                                });
                            }
                            return Err(self.terminal(
                                M1ServingPhysicalRunnerOperationErrorV1::SameShapeSchedule,
                                (failure, diagnostic_history),
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
                    M1ServingPhysicalRunnerOperationErrorV1::ProviderPreparation,
                    (failure, diagnostic_history),
                ));
            }
        };
        let published =
            match self
                .runner
                .submit_rearm(self.engine, prepared.prepared, prepared.recipe)
            {
                Ok(published) => published,
                Err(failure) => {
                    return Err(self.terminal(
                        M1ServingPhysicalRunnerOperationErrorV1::SameShapePublication,
                        (failure, diagnostic_history),
                    ));
                }
            };
        if published.shape() != M1PhysicalFixedBatchShapeV1::SpeculativeK4 {
            return Err(self.terminal(
                M1ServingPhysicalRunnerOperationErrorV1::UnsupportedEvidenceShape,
                (published, diagnostic_history),
            ));
        }
        self.phase = M1ServingPhysicalRunnerAdapterPhaseV1::Published {
            epoch: batch.epoch(),
        };
        Ok(M1ServingPhysicalRunnerPublishedV1 {
            adapter_identity,
            epoch: batch.epoch(),
            state: M1ServingPhysicalRunnerPublishedStateV1::Rearmed {
                published,
                diagnostic_history,
            },
        })
    }

    fn quiescent_rollover(
        &mut self,
        custody: Self::Quiescent,
        _prior: M1ServingPlanV1,
        _next: M1ServingPlanV1,
        _reason: M1ServingRolloverReasonV1,
        _batch: &M1ServingBatchPlanV1,
    ) -> M1ServingPhysicalOperationResultV1<
        Self::Published,
        Self::Quiescent,
        Self::TerminalCustody,
        Self::Error,
    > {
        if self.provider.is_none() {
            return Err(self.terminal(
                M1ServingPhysicalRunnerOperationErrorV1::AdapterSealed,
                custody,
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
        Err(M1ServingPhysicalOperationFailureV1::Retryable {
            source: M1ServingPhysicalRunnerOperationErrorV1::RolloverUnavailable,
            custody,
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
                M1ServingPhysicalRunnerOperationErrorV1::AdapterSealed,
                custody,
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
            || !supports_evidence_bound_s1_k4(batch.plan())
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
        let M1ServingPhysicalRunnerPublishedV1 {
            adapter_identity,
            epoch: custody_epoch,
            state,
        } = custody;
        match state {
            M1ServingPhysicalRunnerPublishedStateV1::First {
                published,
                selected,
                diagnostic_history,
            } => {
                if published.shape() != M1PhysicalFixedBatchShapeV1::SpeculativeK4 {
                    return Err(M1ServingPhysicalOperationFailureV1::Retryable {
                        source: M1ServingPhysicalRunnerOperationErrorV1::UnsupportedEvidenceShape,
                        custody: M1ServingPhysicalRunnerPublishedV1 {
                            adapter_identity,
                            epoch: custody_epoch,
                            state: M1ServingPhysicalRunnerPublishedStateV1::First {
                                published,
                                selected,
                                diagnostic_history,
                            },
                        },
                    });
                }
                let completed = match published.wait() {
                    Ok(completed) => completed,
                    Err(failure) => {
                        return Err(self.terminal(
                            M1ServingPhysicalRunnerOperationErrorV1::QueueWait,
                            (failure, selected, diagnostic_history),
                        ));
                    }
                };
                let recycled = match completed.recycle() {
                    Ok(recycled) => recycled,
                    Err(failure) => {
                        return Err(self.terminal(
                            M1ServingPhysicalRunnerOperationErrorV1::QueueRecycle,
                            (failure, selected, diagnostic_history),
                        ));
                    }
                };
                let observed = match recycled.observe_completion() {
                    Ok(observed) => observed,
                    Err(failure) => {
                        return Err(self.terminal(
                            M1ServingPhysicalRunnerOperationErrorV1::DiagnosticReadback,
                            (failure, selected, diagnostic_history),
                        ));
                    }
                };
                let diagnostic = match observed.observe_speculative_k4_diagnostic_choices() {
                    Ok(diagnostic) => diagnostic,
                    Err(failure) => {
                        return Err(self.terminal(
                            M1ServingPhysicalRunnerOperationErrorV1::DiagnosticReadback,
                            (failure, selected, diagnostic_history),
                        ));
                    }
                };
                let joined = match diagnostic.check_completion() {
                    Ok(joined) => joined,
                    Err(failure) => {
                        return Err(self.terminal(
                            M1ServingPhysicalRunnerOperationErrorV1::DiagnosticReadback,
                            (failure, selected, diagnostic_history),
                        ));
                    }
                };
                let (readback, choices) = joined.into_parts();
                self.phase = M1ServingPhysicalRunnerAdapterPhaseV1::Readback { epoch };
                Ok(M1ServingPhysicalRunnerReadbackV1 {
                    adapter_identity,
                    epoch: custody_epoch,
                    state: M1ServingPhysicalRunnerReadbackStateV1::First {
                        state: M1ServingFirstReadbackStateV1::Ready { readback, selected },
                        choices,
                        diagnostic_history,
                    },
                })
            }
            M1ServingPhysicalRunnerPublishedStateV1::Rearmed {
                published,
                diagnostic_history,
            } => {
                if published.shape() != M1PhysicalFixedBatchShapeV1::SpeculativeK4 {
                    return Err(M1ServingPhysicalOperationFailureV1::Retryable {
                        source: M1ServingPhysicalRunnerOperationErrorV1::UnsupportedEvidenceShape,
                        custody: M1ServingPhysicalRunnerPublishedV1 {
                            adapter_identity,
                            epoch: custody_epoch,
                            state: M1ServingPhysicalRunnerPublishedStateV1::Rearmed {
                                published,
                                diagnostic_history,
                            },
                        },
                    });
                }
                let completed = match published.wait(self.engine) {
                    Ok(completed) => completed,
                    Err(failure) => {
                        return Err(self.terminal(
                            M1ServingPhysicalRunnerOperationErrorV1::QueueWait,
                            (failure, diagnostic_history),
                        ));
                    }
                };
                let recycled = match completed.recycle(self.engine) {
                    Ok(recycled) => recycled,
                    Err(failure) => {
                        return Err(self.terminal(
                            M1ServingPhysicalRunnerOperationErrorV1::QueueRecycle,
                            (failure, diagnostic_history),
                        ));
                    }
                };
                let joined = match recycled.read_and_check_speculative_k4_diagnostic_completion() {
                    Ok(joined) => joined,
                    Err(failure) => {
                        return Err(self.terminal(
                            M1ServingPhysicalRunnerOperationErrorV1::DiagnosticReadback,
                            (failure, diagnostic_history),
                        ));
                    }
                };
                let (readback, choices) = joined.into_parts();
                self.phase = M1ServingPhysicalRunnerAdapterPhaseV1::Readback { epoch };
                Ok(M1ServingPhysicalRunnerReadbackV1 {
                    adapter_identity,
                    epoch: custody_epoch,
                    state: M1ServingPhysicalRunnerReadbackStateV1::Rearmed {
                        state: M1ServingRearmedReadbackStateV1::Ready(readback),
                        choices,
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
                M1ServingPhysicalRunnerOperationErrorV1::AdapterSealed,
                custody,
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
            state,
        } = custody;
        match state {
            M1ServingPhysicalRunnerReadbackStateV1::First {
                state,
                choices,
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
                            state,
                            choices,
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
                                    M1ServingFirstReadbackStateV1::Ready { readback, selected },
                                    choices,
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
                                    M1ServingFirstReadbackStateV1::Ready {
                                        readback,
                                        selected,
                                    },
                                    choices,
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
                                    M1ServingFirstReadbackStateV1::Rejected(rejected),
                                    choices,
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
                                diagnostic_history.push(choices);
                                self.phase = M1ServingPhysicalRunnerAdapterPhaseV1::Quiescent {
                                    epoch: readback_epoch,
                                };
                                Ok(M1ServingPhysicalRunnerQuiescentV1::first(
                                    adapter_identity,
                                    readback_epoch,
                                    released,
                                    diagnostic_history,
                                ))
                            }
                            Err(failure) => Err(self.terminal(
                                M1ServingPhysicalRunnerOperationErrorV1::PageReleaseRejected,
                                (failure, choices, diagnostic_history),
                            )),
                        }
                    }
                    M1CompletedStepOutcomeV1::Rejected(rejected) => {
                        Err(M1ServingPhysicalOperationFailureV1::Retryable {
                            source: M1ServingPhysicalRunnerOperationErrorV1::CompletionRejected,
                            custody: M1ServingPhysicalRunnerReadbackV1::first(
                                adapter_identity,
                                readback_epoch,
                                M1ServingFirstReadbackStateV1::Rejected(rejected),
                                choices,
                                diagnostic_history,
                            ),
                        })
                    }
                    M1CompletedStepOutcomeV1::Poisoned(poison) => Err(self.terminal(
                        M1ServingPhysicalRunnerOperationErrorV1::CompletionPoisoned,
                        (poison, choices, diagnostic_history),
                    )),
                }
            }
            M1ServingPhysicalRunnerReadbackStateV1::Rearmed {
                state,
                choices,
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
                            state,
                            choices,
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
                                        M1ServingRearmedReadbackStateV1::PreflightRejected(failure),
                                        choices,
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
                                    M1ServingRearmedReadbackStateV1::PreflightRejected(failure),
                                    choices,
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
                                        M1ServingRearmedReadbackStateV1::PreflightRejected(failure),
                                        choices,
                                        diagnostic_history,
                                    ),
                                });
                            }
                        }
                    }
                    M1ServingRearmedReadbackStateV1::CompletionRejected(outcome) => {
                        let M1CompletedStepOutcomeV1::Rejected(rejected) = outcome.outcome() else {
                            return Err(self.terminal(
                                M1ServingPhysicalRunnerOperationErrorV1::CompletionPoisoned,
                                (outcome, choices, diagnostic_history),
                            ));
                        };
                        if !dispositions_match_roster(&dispositions, rejected.roster()) {
                            return Err(M1ServingPhysicalOperationFailureV1::Retryable {
                                source: M1ServingPhysicalRunnerOperationErrorV1::DispositionDrift,
                                custody: M1ServingPhysicalRunnerReadbackV1::rearmed(
                                    adapter_identity,
                                    readback_epoch,
                                    M1ServingRearmedReadbackStateV1::CompletionRejected(outcome),
                                    choices,
                                    diagnostic_history,
                                ),
                            });
                        }
                        match outcome.retry_rejected(self.engine) {
                            Ok(outcome) => outcome,
                            Err(outcome) => {
                                return Err(self.terminal(
                                    M1ServingPhysicalRunnerOperationErrorV1::CompletionPoisoned,
                                    (outcome, choices, diagnostic_history),
                                ));
                            }
                        }
                    }
                };
                match outcome.release_completed() {
                    M1RearmedRoundReleaseOutcomeV1::Released(released) => {
                        diagnostic_history.push(choices);
                        self.phase = M1ServingPhysicalRunnerAdapterPhaseV1::Quiescent {
                            epoch: readback_epoch,
                        };
                        Ok(M1ServingPhysicalRunnerQuiescentV1::rearmed(
                            adapter_identity,
                            readback_epoch,
                            released,
                            diagnostic_history,
                        ))
                    }
                    M1RearmedRoundReleaseOutcomeV1::Rejected(failure) => Err(self.terminal(
                        M1ServingPhysicalRunnerOperationErrorV1::PageReleaseRejected,
                        (failure, choices, diagnostic_history),
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
                                        M1ServingRearmedReadbackStateV1::CompletionRejected(
                                            outcome,
                                        ),
                                        choices,
                                        diagnostic_history,
                                    ),
                                })
                            }
                            M1CompletedStepOutcomeV1::Completed(_)
                            | M1CompletedStepOutcomeV1::Poisoned(_) => Err(self.terminal(
                                M1ServingPhysicalRunnerOperationErrorV1::CompletionPoisoned,
                                (outcome, choices, diagnostic_history),
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
}
