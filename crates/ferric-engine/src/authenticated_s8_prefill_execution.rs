//! Authenticated execution and first S8/K4 rollover for fresh S8/T128 prefill.
//!
//! The bootstrap registry reservation is recorded only after physical queue
//! submission. First-token anchors come only from authenticated direct-choice
//! evidence. Physical KV settlement and registry completion remain ordered by
//! the same live roster before the existing finite-speculative scheduler is
//! entered.

use core::{any::Any, fmt};

use ferric_spec::{completion::CompletionEpoch, RequestId, TokenId, ValidatedM1StepInputs};

use crate::{
    complete_m1_authenticated_physical_step_v1,
    release_m1_authenticated_completed_step_kv_pages_v1, DeviceKvPageLease, Engine,
    M1AuthenticatedCompletedStepOutcomeV1, M1AuthenticatedPhysicalQueueSessionV1,
    M1AuthenticatedPreparedSpeculativeRolloverV1, M1AuthenticatedReleasedCompletedStepV1,
    M1AuthenticatedS8T128PrefillPrepublicationV1, M1AuthenticatedScheduledSpeculativeRolloverV1,
    M1AuthenticatedSpeculativePhysicalRoundSuccessV1, M1AuthenticatedSpeculativeRolloverIntentV1,
    M1AuthenticatedSpeculativeRolloverScheduleFailureV1, M1CaptureQuarantinedEngineV1,
    M1DeviceKvCompletionMemberV1, M1DeviceKvCompletionRosterV1,
    M1FiniteSpeculativeQueueRolloverKvInputsV1, M1FullStepWorkspacePlans,
    M1ObservedDirectDiagnosticChoicesV1, M1QueueWaitTimeoutV1, M1ServingCompletionDispositionV1,
    M1ServingPlanV1, M1ServingPublicationReservationV1, M1ServingQueueActionV1,
    M1ServingRegistryIdentityV1, M1ServingRegistryV1, M1SpeculativeGenerationLoopV1,
    M1SpeculativeGenerationPolicyV1, M1SpeculativeMemberControlV1, M1SpeculativeMemberSeedV1,
    M1SpeculativeMemberStatusV1,
};

/// Stable stage for fresh S8 prefill and its first speculative publication.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedS8PrefillExecutionStageV1 {
    EnginePreflight,
    RegistryPreflight,
    QueueCreate,
    QueueSubmit,
    RegistryPublication,
    QueueWait,
    QueueRecycle,
    CompletionObservation,
    DirectChoiceObservation,
    DirectCompletionCheck,
    DirectChoiceCardinality,
    PhysicalCompletion,
    PrefillPageRelease,
    RegistryCompletion,
    RolloverInputJoin,
    RolloverCoordinator,
    RolloverSchedule,
    RolloverPrepare,
    RolloverSubmit,
    RolloverPhysicalCompletion,
    RolloverRegistryCompletion,
}

/// Stable high-level reason for terminal failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedS8PrefillExecutionErrorV1 {
    EngineFaulted,
    RegistryDrift,
    DeadlineExpired,
    HostAllocation,
    DirectChoiceCardinality { expected: usize, actual: usize },
    RosterMismatch,
    PhysicalCompletionPoisoned,
    LowerRejected,
}

struct OpaqueExecutionCustody(Box<dyn Any>);

impl fmt::Debug for OpaqueExecutionCustody {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _ = &self.0;
        formatter
            .debug_struct("OpaqueExecutionCustody")
            .finish_non_exhaustive()
    }
}

/// Status-only terminal custody after explicitly closing an S8 queue.
///
/// Queue destruction and logical Engine quarantine are separate observations:
/// a clean queue release does not make the retained Engine reusable. No
/// registry, Engine, physical queue, or allocation owner can be recovered.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedS8CloseV1;
/// fn recover(closed: M1AuthenticatedS8CloseV1) {
///     let _ = closed.into_parts();
/// }
/// ```
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedS8CloseV1;
/// fn duplicate(closed: M1AuthenticatedS8CloseV1) {
///     let _ = closed.clone();
/// }
/// ```
#[must_use = "S8 close status and terminal custody must remain observed"]
pub struct M1AuthenticatedS8CloseV1 {
    queue_released: bool,
    engine_quarantined: bool,
    retained: OpaqueExecutionCustody,
}

impl fmt::Debug for M1AuthenticatedS8CloseV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedS8CloseV1")
            .field("queue_released", &self.queue_released)
            .field("engine_quarantined", &self.engine_quarantined)
            .field("retained", &self.retained)
            .finish_non_exhaustive()
    }
}

impl M1AuthenticatedS8CloseV1 {
    // The split payload preserves each caller's drop order around lower teardown custody.
    fn from_teardown<T: Any, E: Any>(
        teardown: Result<T, E>,
        engine_quarantined: bool,
        before_teardown: impl Any,
        after_teardown: impl Any,
    ) -> Self {
        Self {
            queue_released: teardown.is_ok(),
            engine_quarantined,
            retained: OpaqueExecutionCustody(Box::new((
                before_teardown,
                teardown,
                after_teardown,
            ))),
        }
    }

    fn from_disposition(
        disposition: crate::M1AuthenticatedSpeculativeFailureDispositionV1,
        engine_quarantined: bool,
        before_teardown: impl Any,
        after_teardown: impl Any,
    ) -> Self {
        Self {
            queue_released: disposition.queue_released(),
            engine_quarantined,
            retained: OpaqueExecutionCustody(Box::new((
                before_teardown,
                disposition,
                after_teardown,
            ))),
        }
    }

    /// Whether native queue destruction completed with retained release evidence.
    #[must_use]
    pub const fn queue_released(&self) -> bool {
        self.queue_released
    }

    /// Whether this close confirms that no live native queue remains.
    #[must_use]
    pub const fn permits_stop_success(&self) -> bool {
        self.queue_released
    }

    /// Logical Engine state after the lower teardown and terminalization.
    #[must_use]
    pub const fn engine_quarantined(&self) -> bool {
        self.engine_quarantined
    }
}

/// Terminal opaque custody after any execution or rollover failure.
#[must_use = "terminal S8 execution custody must be retained"]
pub struct M1AuthenticatedS8PrefillExecutionFailureV1<const C: usize> {
    stage: M1AuthenticatedS8PrefillExecutionStageV1,
    error: M1AuthenticatedS8PrefillExecutionErrorV1,
    engine: M1CaptureQuarantinedEngineV1<C>,
    retained: OpaqueExecutionCustody,
}

impl<const C: usize> fmt::Debug for M1AuthenticatedS8PrefillExecutionFailureV1<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedS8PrefillExecutionFailureV1")
            .field("stage", &self.stage)
            .field("error", &self.error)
            .field("engine_quarantined", &self.engine.is_faulted())
            .field("retains_opaque_custody", &true)
            .finish_non_exhaustive()
    }
}

impl<const C: usize> M1AuthenticatedS8PrefillExecutionFailureV1<C> {
    #[must_use]
    pub const fn stage(&self) -> M1AuthenticatedS8PrefillExecutionStageV1 {
        self.stage
    }

    #[must_use]
    pub const fn error(&self) -> M1AuthenticatedS8PrefillExecutionErrorV1 {
        self.error
    }

    #[must_use]
    pub const fn engine_quarantined(&self) -> bool {
        true
    }

    #[must_use = "Engine quarantine and retained custody both remain terminal"]
    pub fn into_parts(
        self,
    ) -> (
        M1CaptureQuarantinedEngineV1<C>,
        M1AuthenticatedS8PrefillExecutionStageV1,
        M1AuthenticatedS8PrefillExecutionErrorV1,
        Box<dyn Any>,
    ) {
        (self.engine, self.stage, self.error, self.retained.0)
    }
}

fn terminal_failure<const C: usize>(
    mut engine: Engine<C>,
    stage: M1AuthenticatedS8PrefillExecutionStageV1,
    error: M1AuthenticatedS8PrefillExecutionErrorV1,
    retained: impl Any,
) -> Box<M1AuthenticatedS8PrefillExecutionFailureV1<C>> {
    engine.quarantine_m1_queue_rearm_failure();
    Box::new(M1AuthenticatedS8PrefillExecutionFailureV1 {
        stage,
        error,
        engine: engine.into_m1_capture_quarantine(),
        retained: OpaqueExecutionCustody(Box::new(retained)),
    })
}

fn require_authenticated_prefill_choices(
    choices: &M1ObservedDirectDiagnosticChoicesV1,
    expected: usize,
) -> Result<&[TokenId], M1AuthenticatedS8PrefillExecutionErrorV1> {
    if choices.choices().len() != expected {
        return Err(
            M1AuthenticatedS8PrefillExecutionErrorV1::DirectChoiceCardinality {
                expected,
                actual: choices.choices().len(),
            },
        );
    }
    Ok(choices.choices())
}

#[derive(Debug)]
struct SuccessorCustody {
    draft_rollover_pages: Vec<DeviceKvPageLease>,
    target_rollover_pages: Vec<Vec<DeviceKvPageLease>>,
    rollover_intent: M1AuthenticatedSpeculativeRolloverIntentV1,
    requests: Box<[RequestId]>,
    prompts: Vec<Box<[TokenId]>>,
    policies: Vec<M1SpeculativeGenerationPolicyV1>,
    diagnostic_ring_bytes: u32,
    queue_wait_timeout: M1QueueWaitTimeoutV1,
}

/// Real authenticated S8/T128 completion joined to the ordinary registry frontier.
#[must_use = "completed S8 prefill must enter its first speculative round or close"]
pub struct M1AuthenticatedS8T128PrefillExecutionSuccessV1<const C: usize> {
    registry: M1ServingRegistryV1<C>,
    engine: Engine<C>,
    released: M1AuthenticatedReleasedCompletedStepV1,
    first_tokens: Box<[TokenId]>,
    direct_choices: M1ObservedDirectDiagnosticChoicesV1,
    successor: SuccessorCustody,
}

impl<const C: usize> fmt::Debug for M1AuthenticatedS8T128PrefillExecutionSuccessV1<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedS8T128PrefillExecutionSuccessV1")
            .field("requests", &self.successor.requests)
            .field("first_tokens", &self.first_tokens)
            .finish_non_exhaustive()
    }
}

impl<const C: usize> M1AuthenticatedS8T128PrefillExecutionSuccessV1<C> {
    #[must_use]
    pub fn requests(&self) -> &[RequestId] {
        &self.successor.requests
    }

    #[must_use]
    pub fn first_tokens(&self) -> &[TokenId] {
        &self.first_tokens
    }

    pub const fn direct_choices(&self) -> &M1ObservedDirectDiagnosticChoicesV1 {
        &self.direct_choices
    }

    /// Explicitly abandons rollover and reports the retained prefill queue's teardown.
    #[must_use = "closed prefill state remains opaque and retained"]
    pub fn close(mut self) -> M1AuthenticatedS8CloseV1 {
        let teardown = self
            .released
            .destroy_queue_and_retain_step(&mut self.engine);
        let engine = self.engine.into_m1_capture_quarantine();
        M1AuthenticatedS8CloseV1::from_teardown(
            teardown,
            engine.is_faulted(),
            (self.registry, engine),
            (self.first_tokens, self.direct_choices, self.successor),
        )
    }
}

/// Executes one real authenticated S8/T128 queue and advances its real registry.
///
/// # Errors
///
/// Any registry, queue, observation, settlement, or release rejection closes
/// available physical custody, quarantines the Engine, and returns opaque
/// terminal ownership.
#[allow(clippy::too_many_lines)]
pub fn execute_m1_authenticated_s8_t128_paired_prefill_v1<const C: usize>(
    prepared: M1AuthenticatedS8T128PrefillPrepublicationV1<C>,
    diagnostic_ring_bytes: u32,
    queue_wait_timeout: M1QueueWaitTimeoutV1,
) -> Result<
    M1AuthenticatedS8T128PrefillExecutionSuccessV1<C>,
    Box<M1AuthenticatedS8PrefillExecutionFailureV1<C>>,
> {
    use M1AuthenticatedS8PrefillExecutionErrorV1 as Error;
    use M1AuthenticatedS8PrefillExecutionStageV1 as Stage;

    let M1AuthenticatedS8T128PrefillPrepublicationV1 {
        mut engine,
        mut registry,
        reservation,
        prepublication,
        caches,
        draft_rollover_pages,
        target_rollover_pages,
        rollover_intent,
        requests,
        prompts,
        policies,
    } = prepared;
    let successor = SuccessorCustody {
        draft_rollover_pages,
        target_rollover_pages,
        rollover_intent,
        requests,
        prompts,
        policies,
        diagnostic_ring_bytes,
        queue_wait_timeout,
    };
    if engine.is_faulted() {
        return Err(terminal_failure(
            engine,
            Stage::EnginePreflight,
            Error::EngineFaulted,
            (registry, reservation, prepublication, caches, successor),
        ));
    }
    if let Err(error) = registry.preflight_publication(&reservation) {
        let abort = registry.abort_publication(reservation);
        return Err(terminal_failure(
            engine,
            Stage::RegistryPreflight,
            Error::RegistryDrift,
            (registry, abort, prepublication, caches, successor, error),
        ));
    }
    let registry_identity = reservation.registry_identity();
    let prefill_epoch = reservation.epoch();
    let queue = match M1AuthenticatedPhysicalQueueSessionV1::create(
        diagnostic_ring_bytes,
        prepublication,
    ) {
        Ok(queue) => queue,
        Err(failure) => {
            let abort = registry.abort_publication(reservation);
            let failure = failure.quarantine_engine(&mut engine);
            return Err(terminal_failure(
                engine,
                Stage::QueueCreate,
                Error::LowerRejected,
                (registry, abort, failure, caches, successor),
            ));
        }
    };
    let published = match queue.submit() {
        Ok(published) => published,
        Err(failure) => {
            let abort = registry.abort_publication(reservation);
            let closure = failure.close_without_authority(&mut engine);
            return Err(terminal_failure(
                engine,
                Stage::QueueSubmit,
                Error::LowerRejected,
                (registry, abort, closure, caches, successor),
            ));
        }
    };
    if let Err(failure) = registry.record_publication(reservation) {
        let closure = published.close_in_flight();
        return Err(terminal_failure(
            engine,
            Stage::RegistryPublication,
            Error::RegistryDrift,
            (registry, failure, closure, caches, successor),
        ));
    }
    let completed = match published.wait_for(queue_wait_timeout.milliseconds()) {
        Ok(completed) => completed,
        Err(failure) => {
            let failure = (*failure).quarantine_engine(&mut engine);
            return Err(terminal_failure(
                engine,
                Stage::QueueWait,
                Error::LowerRejected,
                (registry, failure, caches, successor),
            ));
        }
    };
    let recycled = match completed.recycle() {
        Ok(recycled) => recycled,
        Err(failure) => {
            let failure = (*failure).quarantine_engine(&mut engine);
            return Err(terminal_failure(
                engine,
                Stage::QueueRecycle,
                Error::LowerRejected,
                (registry, failure, caches, successor),
            ));
        }
    };
    let observed = match recycled.observe_completion() {
        Ok(observed) => observed,
        Err(failure) => match failure.retry() {
            Ok(observed) => observed,
            Err(failure) => {
                let teardown = (*failure).destroy_queue_and_retain_evidence(&mut engine);
                return Err(terminal_failure(
                    engine,
                    Stage::CompletionObservation,
                    Error::LowerRejected,
                    (registry, teardown, caches, successor),
                ));
            }
        },
    };
    let direct = match observed.observe_direct_diagnostic_choices() {
        Ok(direct) => direct,
        Err(failure) => {
            let teardown = (*failure).destroy_queue_and_retain_evidence(&mut engine);
            return Err(terminal_failure(
                engine,
                Stage::DirectChoiceObservation,
                Error::LowerRejected,
                (registry, teardown, caches, successor),
            ));
        }
    };
    let direct = match direct.check_completion() {
        Ok(direct) => direct,
        Err(failure) => {
            let teardown = (*failure).destroy_queue_and_retain_evidence(&mut engine);
            return Err(terminal_failure(
                engine,
                Stage::DirectCompletionCheck,
                Error::LowerRejected,
                (registry, teardown, caches, successor),
            ));
        }
    };
    let authenticated_choices =
        match require_authenticated_prefill_choices(direct.choices(), successor.requests.len()) {
            Ok(tokens) => tokens,
            Err(error) => {
                let teardown = direct.destroy_queue_and_retain_evidence(&mut engine);
                return Err(terminal_failure(
                    engine,
                    Stage::DirectChoiceCardinality,
                    error,
                    (registry, teardown, caches, successor),
                ));
            }
        };
    let mut first_tokens = Vec::new();
    if first_tokens
        .try_reserve_exact(authenticated_choices.len())
        .is_err()
    {
        let teardown = direct.destroy_queue_and_retain_evidence(&mut engine);
        return Err(terminal_failure(
            engine,
            Stage::DirectChoiceCardinality,
            Error::HostAllocation,
            (registry, teardown, caches, successor),
        ));
    }
    first_tokens.extend_from_slice(authenticated_choices);
    let first_tokens = first_tokens.into_boxed_slice();
    let mut members = Vec::new();
    if members.try_reserve_exact(caches.len()).is_err() {
        let teardown = direct.destroy_queue_and_retain_evidence(&mut engine);
        return Err(terminal_failure(
            engine,
            Stage::PhysicalCompletion,
            Error::HostAllocation,
            (registry, teardown, caches, first_tokens, successor),
        ));
    }
    members.extend(
        caches
            .into_iter()
            .map(M1DeviceKvCompletionMemberV1::continuing),
    );
    let (readback, direct_choices) = direct.into_parts();
    let physical = match complete_m1_authenticated_physical_step_v1(
        &mut engine,
        readback,
        M1DeviceKvCompletionRosterV1::new(members),
    ) {
        M1AuthenticatedCompletedStepOutcomeV1::Completed(physical) => physical,
        outcome @ (M1AuthenticatedCompletedStepOutcomeV1::Rejected(_)
        | M1AuthenticatedCompletedStepOutcomeV1::Poisoned(_)) => {
            let poisoned = matches!(&outcome, M1AuthenticatedCompletedStepOutcomeV1::Poisoned(_));
            let closure =
                crate::m1_completed_step::close_m1_authenticated_completed_step_outcome_v1(
                    &mut engine,
                    outcome,
                );
            return Err(terminal_failure(
                engine,
                Stage::PhysicalCompletion,
                if poisoned {
                    Error::PhysicalCompletionPoisoned
                } else {
                    Error::LowerRejected
                },
                (registry, closure, direct_choices, first_tokens, successor),
            ));
        }
    };
    let released = match release_m1_authenticated_completed_step_kv_pages_v1(physical) {
        Ok(released) => released,
        Err(failure) => {
            let (release_error, physical) = (*failure).into_parts();
            let teardown = physical.destroy_queue_and_retain_completion(&mut engine);
            return Err(terminal_failure(
                engine,
                Stage::PrefillPageRelease,
                Error::LowerRejected,
                (
                    registry,
                    release_error,
                    teardown,
                    direct_choices,
                    first_tokens,
                    successor,
                ),
            ));
        }
    };
    let mut dispositions = Vec::new();
    if dispositions
        .try_reserve_exact(successor.requests.len())
        .is_err()
    {
        let teardown = released.destroy_queue_and_retain_step(&mut engine);
        return Err(terminal_failure(
            engine,
            Stage::RegistryCompletion,
            Error::HostAllocation,
            (registry, teardown, direct_choices, first_tokens, successor),
        ));
    }
    dispositions.extend(
        (0..successor.requests.len())
            .map(|_| M1ServingCompletionDispositionV1::Continue(successor_plan())),
    );
    if let Err(error) =
        registry.preflight_completion_exact_for(registry_identity, prefill_epoch, &dispositions)
    {
        let teardown = released.destroy_queue_and_retain_step(&mut engine);
        return Err(terminal_failure(
            engine,
            Stage::RegistryCompletion,
            Error::RegistryDrift,
            (
                registry,
                teardown,
                direct_choices,
                first_tokens,
                successor,
                error,
            ),
        ));
    }
    registry.apply_preflighted_completion(prefill_epoch, &dispositions);
    Ok(M1AuthenticatedS8T128PrefillExecutionSuccessV1 {
        registry,
        engine,
        released,
        first_tokens,
        direct_choices,
        successor,
    })
}

fn successor_plan() -> M1ServingPlanV1 {
    use ferric_spec::{Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection};
    M1ServingPlanV1::new(
        Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Speculative,
            bucket: Qwen3PlanBucket::SpeculativeS8K4C8192,
        },
        Qwen3PlanSelection {
            role: Qwen3ModelRole::Draft06B,
            mode: Qwen3ExecutionMode::Decode,
            bucket: Qwen3PlanBucket::DecodeS8C8192,
        },
    )
    .expect("the frozen S8/K4 pair is admitted")
}

fn prefill_plan() -> M1ServingPlanV1 {
    use ferric_spec::{Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection};
    M1ServingPlanV1::new(
        Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Prefill,
            bucket: Qwen3PlanBucket::PrefillS8T128,
        },
        Qwen3PlanSelection {
            role: Qwen3ModelRole::Draft06B,
            mode: Qwen3ExecutionMode::Prefill,
            bucket: Qwen3PlanBucket::PrefillS8T128,
        },
    )
    .expect("the frozen S8/T128 pair is admitted")
}

/// Addressless inputs for the first real S8/K4 round.
#[must_use = "first-round inputs remain linear"]
#[derive(Debug)]
pub struct M1AuthenticatedS8FirstRoundInputsV1 {
    draft_decode: ValidatedM1StepInputs,
    target_speculative: ValidatedM1StepInputs,
    recipe_plans: crate::runner::M1PhysicalRunnerRecipeInputV1,
    preparation_plans: M1FullStepWorkspacePlans,
}

impl M1AuthenticatedS8FirstRoundInputsV1 {
    pub const fn new(
        draft_decode: ValidatedM1StepInputs,
        target_speculative: ValidatedM1StepInputs,
        recipe_plans: M1FullStepWorkspacePlans,
        preparation_plans: M1FullStepWorkspacePlans,
    ) -> Self {
        Self {
            draft_decode,
            target_speculative,
            recipe_plans: crate::runner::M1PhysicalRunnerRecipeInputV1::plans(recipe_plans),
            preparation_plans,
        }
    }
}

#[derive(Debug)]
struct FirstRoundResidue {
    requests: Box<[RequestId]>,
    _first_tokens: Box<[TokenId]>,
    _choices: M1ObservedDirectDiagnosticChoicesV1,
    _prompts: Vec<Box<[TokenId]>>,
    _policies: Vec<M1SpeculativeGenerationPolicyV1>,
    _remaining_target_pages: Vec<Vec<DeviceKvPageLease>>,
    diagnostic_ring_bytes: u32,
    queue_wait_timeout: M1QueueWaitTimeoutV1,
}

/// Registry reservation joined to a scheduled first S8/K4 physical round.
#[must_use = "scheduled S8/K4 custody must be prepared"]
pub struct M1AuthenticatedS8ScheduledFirstRoundV1<const C: usize> {
    registry: M1ServingRegistryV1<C>,
    reservation: M1ServingPublicationReservationV1,
    engine: Engine<C>,
    scheduled: M1AuthenticatedScheduledSpeculativeRolloverV1,
    residue: FirstRoundResidue,
}

/// Prepared first S8/K4 round with its exact registry reservation.
#[must_use = "prepared S8/K4 custody must be published"]
pub struct M1AuthenticatedS8PreparedFirstRoundV1<const C: usize> {
    registry: M1ServingRegistryV1<C>,
    reservation: M1ServingPublicationReservationV1,
    engine: Engine<C>,
    prepared: M1AuthenticatedPreparedSpeculativeRolloverV1,
    residue: FirstRoundResidue,
}

/// Published first S8/K4 round with matching in-flight registry state.
#[must_use = "published S8/K4 custody must be completed"]
pub struct M1AuthenticatedS8PublishedFirstRoundV1<const C: usize> {
    registry: M1ServingRegistryV1<C>,
    registry_identity: M1ServingRegistryIdentityV1,
    epoch: CompletionEpoch,
    engine: Engine<C>,
    published: crate::M1AuthenticatedSpeculativeRolloverPublishedV1,
    residue: FirstRoundResidue,
}

/// Completed first S8/K4 physical and registry round.
#[must_use = "completed S8/K4 custody must remain retained or close"]
pub struct M1AuthenticatedS8CompletedFirstRoundV1<const C: usize> {
    registry: M1ServingRegistryV1<C>,
    engine: Engine<C>,
    physical: M1AuthenticatedSpeculativePhysicalRoundSuccessV1,
    residue: FirstRoundResidue,
}

impl<const C: usize> fmt::Debug for M1AuthenticatedS8ScheduledFirstRoundV1<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("M1AuthenticatedS8ScheduledFirstRoundV1")
            .field("requests", &self.residue.requests)
            .field("epoch", &self.reservation.epoch())
            .finish_non_exhaustive()
    }
}

impl<const C: usize> M1AuthenticatedS8T128PrefillExecutionSuccessV1<C> {
    /// Joins authenticated prefill choices and preleased pages to the first S8/K4 schedule.
    ///
    /// # Errors
    ///
    /// Registry, roster, coordinator, or physical schedule rejection closes
    /// the released prefill queue and retains all owners terminally.
    #[allow(clippy::too_many_lines)]
    pub fn schedule_first_speculative_round(
        self,
        inputs: M1AuthenticatedS8FirstRoundInputsV1,
    ) -> Result<
        M1AuthenticatedS8ScheduledFirstRoundV1<C>,
        Box<M1AuthenticatedS8PrefillExecutionFailureV1<C>>,
    > {
        use M1AuthenticatedS8PrefillExecutionErrorV1 as Error;
        use M1AuthenticatedS8PrefillExecutionStageV1 as Stage;

        let Self {
            mut registry,
            mut engine,
            released,
            first_tokens,
            direct_choices,
            successor,
        } = self;
        let batch = match registry.plan_next() {
            Ok(Some(batch))
                if batch.plan() == successor_plan()
                    && batch.requests() == successor.requests.as_ref()
                    && batch.action()
                        == (M1ServingQueueActionV1::QuiescentRollover {
                            prior: prefill_plan(),
                            next: successor_plan(),
                            reason: crate::M1ServingRolloverReasonV1::Mode,
                        }) =>
            {
                batch
            }
            Ok(batch) => {
                let teardown = released.destroy_queue_and_retain_step(&mut engine);
                return Err(terminal_failure(
                    engine,
                    Stage::RolloverInputJoin,
                    Error::RegistryDrift,
                    (
                        registry,
                        teardown,
                        first_tokens,
                        direct_choices,
                        successor,
                        inputs,
                        batch,
                    ),
                ));
            }
            Err(error) => {
                let teardown = released.destroy_queue_and_retain_step(&mut engine);
                return Err(terminal_failure(
                    engine,
                    Stage::RolloverInputJoin,
                    Error::LowerRejected,
                    (
                        registry,
                        teardown,
                        first_tokens,
                        direct_choices,
                        successor,
                        inputs,
                        error,
                    ),
                ));
            }
        };
        let reservation = match registry.reserve_publication(batch) {
            Ok(reservation) => reservation,
            Err(error) => {
                let teardown = released.destroy_queue_and_retain_step(&mut engine);
                return Err(terminal_failure(
                    engine,
                    Stage::RolloverInputJoin,
                    Error::LowerRejected,
                    (
                        registry,
                        teardown,
                        first_tokens,
                        direct_choices,
                        successor,
                        inputs,
                        error,
                    ),
                ));
            }
        };
        let M1AuthenticatedS8FirstRoundInputsV1 {
            draft_decode,
            target_speculative,
            recipe_plans,
            preparation_plans,
        } = inputs;
        if successor.draft_rollover_pages.len() != successor.requests.len()
            || successor.target_rollover_pages.len() != successor.requests.len()
            || first_tokens.len() != successor.requests.len()
        {
            let abort = registry.abort_publication(reservation);
            let teardown = released.destroy_queue_and_retain_step(&mut engine);
            return Err(terminal_failure(
                engine,
                Stage::RolloverInputJoin,
                Error::RosterMismatch,
                (
                    registry,
                    abort,
                    teardown,
                    first_tokens,
                    direct_choices,
                    successor,
                    draft_decode,
                    target_speculative,
                    recipe_plans,
                    preparation_plans,
                ),
            ));
        }
        let mut draft_pages = Vec::new();
        let mut target_pages = Vec::new();
        let mut remaining_target_pages = Vec::new();
        if draft_pages
            .try_reserve_exact(successor.requests.len())
            .is_err()
            || target_pages
                .try_reserve_exact(successor.requests.len())
                .is_err()
            || remaining_target_pages
                .try_reserve_exact(successor.requests.len())
                .is_err()
        {
            let abort = registry.abort_publication(reservation);
            let teardown = released.destroy_queue_and_retain_step(&mut engine);
            return Err(terminal_failure(
                engine,
                Stage::RolloverInputJoin,
                Error::HostAllocation,
                (
                    registry,
                    abort,
                    teardown,
                    first_tokens,
                    direct_choices,
                    successor.requests,
                    successor.prompts,
                    successor.policies,
                    successor.draft_rollover_pages,
                    successor.target_rollover_pages,
                    draft_decode,
                    target_speculative,
                    recipe_plans,
                    preparation_plans,
                ),
            ));
        }
        draft_pages.extend(
            successor
                .draft_rollover_pages
                .into_iter()
                .map(|page| vec![page]),
        );
        for mut pages in successor.target_rollover_pages {
            if pages.is_empty() {
                let abort = registry.abort_publication(reservation);
                let teardown = released.destroy_queue_and_retain_step(&mut engine);
                return Err(terminal_failure(
                    engine,
                    Stage::RolloverInputJoin,
                    Error::RosterMismatch,
                    (
                        registry,
                        abort,
                        teardown,
                        first_tokens,
                        direct_choices,
                        successor.requests,
                        successor.prompts,
                        successor.policies,
                        draft_pages,
                        target_pages,
                        remaining_target_pages,
                        pages,
                        draft_decode,
                        target_speculative,
                        recipe_plans,
                        preparation_plans,
                    ),
                ));
            }
            let remaining = pages.split_off(1);
            target_pages.push(pages);
            remaining_target_pages.push(remaining);
        }
        let kv_inputs = M1FiniteSpeculativeQueueRolloverKvInputsV1::from_lane_leases(
            draft_decode,
            target_speculative,
            draft_pages,
            target_pages,
        );
        let mut seeds = Vec::new();
        if seeds.try_reserve_exact(successor.requests.len()).is_err() {
            let abort = registry.abort_publication(reservation);
            let teardown = released.destroy_queue_and_retain_step(&mut engine);
            return Err(terminal_failure(
                engine,
                Stage::RolloverCoordinator,
                Error::HostAllocation,
                (
                    registry,
                    abort,
                    teardown,
                    first_tokens,
                    direct_choices,
                    successor.requests,
                    successor.prompts,
                    successor.policies,
                    remaining_target_pages,
                    kv_inputs,
                    recipe_plans,
                    preparation_plans,
                ),
            ));
        }
        seeds.extend(
            successor
                .requests
                .iter()
                .copied()
                .zip(first_tokens.iter().copied())
                .zip(successor.policies.iter().copied())
                .map(|((request, token), policy)| {
                    M1SpeculativeMemberSeedV1::new(request, token, 128, 128, policy)
                }),
        );
        let coordinator =
            match M1SpeculativeGenerationLoopV1::new(successor_plan().target(), &seeds) {
                Ok(coordinator) => coordinator,
                Err(error) => {
                    let abort = registry.abort_publication(reservation);
                    let teardown = released.destroy_queue_and_retain_step(&mut engine);
                    return Err(terminal_failure(
                        engine,
                        Stage::RolloverCoordinator,
                        Error::LowerRejected,
                        (
                            registry,
                            abort,
                            teardown,
                            first_tokens,
                            direct_choices,
                            successor.requests,
                            successor.prompts,
                            successor.policies,
                            remaining_target_pages,
                            kv_inputs,
                            recipe_plans,
                            preparation_plans,
                            error,
                        ),
                    ));
                }
            };
        let batch = reservation.physical_batch();
        let scheduled = match crate::authenticated_queue_rollover::schedule_m1_authenticated_speculative_rollover_with_recipe_input_v1(
            &mut engine,
            released,
            &batch,
            successor.rollover_intent,
            coordinator,
            kv_inputs,
            recipe_plans,
            preparation_plans,
        ) {
            Ok(scheduled) => scheduled,
            Err(M1AuthenticatedSpeculativeRolloverScheduleFailureV1::PreDetach { error, retry }) => {
                let abort = registry.abort_publication(reservation);
                let disposition = retry.cancel_and_close(&mut engine);
                return Err(terminal_failure(
                    engine, Stage::RolloverSchedule, Error::LowerRejected,
                    (registry, abort, disposition, first_tokens, direct_choices, successor.requests, successor.prompts, successor.policies, remaining_target_pages, error),
                ));
            }
            Err(M1AuthenticatedSpeculativeRolloverScheduleFailureV1::Terminal { error, disposition }) => {
                let abort = registry.abort_publication(reservation);
                return Err(terminal_failure(
                    engine, Stage::RolloverSchedule, Error::LowerRejected,
                    (registry, abort, disposition, first_tokens, direct_choices, successor.requests, successor.prompts, successor.policies, remaining_target_pages, error),
                ));
            }
        };
        Ok(M1AuthenticatedS8ScheduledFirstRoundV1 {
            registry,
            reservation,
            engine,
            scheduled,
            residue: FirstRoundResidue {
                requests: successor.requests,
                _first_tokens: first_tokens,
                _choices: direct_choices,
                _prompts: successor.prompts,
                _policies: successor.policies,
                _remaining_target_pages: remaining_target_pages,
                diagnostic_ring_bytes: successor.diagnostic_ring_bytes,
                queue_wait_timeout: successor.queue_wait_timeout,
            },
        })
    }
}

impl<const C: usize> M1AuthenticatedS8ScheduledFirstRoundV1<C> {
    /// Prepares the scheduled S8/K4 round using the existing retained recipe boundary.
    ///
    /// # Errors
    ///
    /// Preparation rejection aborts the registry reservation and returns only
    /// terminal opaque custody.
    pub fn prepare(
        self,
    ) -> Result<
        M1AuthenticatedS8PreparedFirstRoundV1<C>,
        Box<M1AuthenticatedS8PrefillExecutionFailureV1<C>>,
    > {
        let Self {
            mut registry,
            reservation,
            mut engine,
            scheduled,
            residue,
        } = self;
        match crate::authenticated_queue_rollover::prepare_m1_authenticated_speculative_rollover_retained_v1(
            &mut engine,
            scheduled,
        ) {
            Ok(prepared) => Ok(M1AuthenticatedS8PreparedFirstRoundV1 {
                registry,
                reservation,
                engine,
                prepared,
                residue,
            }),
            Err(failure) => {
                let abort = registry.abort_publication(reservation);
                let disposition = failure.into_disposition();
                Err(terminal_failure(
                    engine,
                    M1AuthenticatedS8PrefillExecutionStageV1::RolloverPrepare,
                    M1AuthenticatedS8PrefillExecutionErrorV1::LowerRejected,
                    (registry, abort, disposition, residue),
                ))
            }
        }
    }

    /// Cancels before rollover publication and retains every owner opaquely.
    #[must_use = "cancelled scheduled S8/K4 custody remains retained"]
    pub fn cancel_and_close(mut self) -> M1AuthenticatedS8CloseV1 {
        let disposition = match crate::authenticated_queue_rollover::prepare_m1_authenticated_speculative_rollover_retained_v1(
            &mut self.engine,
            self.scheduled,
        ) {
            Ok(prepared) => prepared.cancel_and_close(&mut self.engine),
            Err(failure) => failure.into_disposition(),
        };
        let abort = self.registry.abort_publication(self.reservation);
        let engine = self.engine.into_m1_capture_quarantine();
        M1AuthenticatedS8CloseV1::from_disposition(
            disposition,
            engine.is_faulted(),
            (self.registry, engine, abort),
            self.residue,
        )
    }
}

impl<const C: usize> M1AuthenticatedS8PreparedFirstRoundV1<C> {
    /// Submits the existing S8/K4 rollover and records the exact reservation.
    ///
    /// # Errors
    ///
    /// Preflight, submission, or registry-record rejection closes physical
    /// custody and quarantines the Engine.
    pub fn publish(
        self,
    ) -> Result<
        M1AuthenticatedS8PublishedFirstRoundV1<C>,
        Box<M1AuthenticatedS8PrefillExecutionFailureV1<C>>,
    > {
        let Self {
            mut registry,
            reservation,
            mut engine,
            prepared,
            residue,
        } = self;
        if let Err(error) = registry.preflight_publication(&reservation) {
            let abort = registry.abort_publication(reservation);
            let disposition = prepared.cancel_and_close(&mut engine);
            return Err(terminal_failure(
                engine,
                M1AuthenticatedS8PrefillExecutionStageV1::RegistryPreflight,
                M1AuthenticatedS8PrefillExecutionErrorV1::RegistryDrift,
                (registry, abort, disposition, residue, error),
            ));
        }
        let identity = reservation.registry_identity();
        let epoch = reservation.epoch();
        let published = match crate::submit_m1_authenticated_speculative_rollover_v1(
            &mut engine,
            prepared,
            residue.diagnostic_ring_bytes,
            residue.queue_wait_timeout,
        ) {
            Ok(published) => published,
            Err(failure) => {
                let abort = registry.abort_publication(reservation);
                let disposition = failure.into_disposition();
                return Err(terminal_failure(
                    engine,
                    M1AuthenticatedS8PrefillExecutionStageV1::RolloverSubmit,
                    M1AuthenticatedS8PrefillExecutionErrorV1::LowerRejected,
                    (registry, abort, disposition, residue),
                ));
            }
        };
        if let Err(failure) = registry.record_publication(reservation) {
            let disposition = published.cancel_and_close(&mut engine);
            return Err(terminal_failure(
                engine,
                M1AuthenticatedS8PrefillExecutionStageV1::RegistryPublication,
                M1AuthenticatedS8PrefillExecutionErrorV1::RegistryDrift,
                (registry, failure, disposition, residue),
            ));
        }
        Ok(M1AuthenticatedS8PublishedFirstRoundV1 {
            registry,
            registry_identity: identity,
            epoch,
            engine,
            published,
            residue,
        })
    }

    /// Cancels prepared work before submission and retains every owner opaquely.
    #[must_use = "cancelled prepared S8/K4 custody remains retained"]
    pub fn cancel_and_close(mut self) -> M1AuthenticatedS8CloseV1 {
        let disposition = self.prepared.cancel_and_close(&mut self.engine);
        let abort = self.registry.abort_publication(self.reservation);
        let engine = self.engine.into_m1_capture_quarantine();
        M1AuthenticatedS8CloseV1::from_disposition(
            disposition,
            engine.is_faulted(),
            (self.registry, engine, abort),
            self.residue,
        )
    }
}

impl<const C: usize> M1AuthenticatedS8PublishedFirstRoundV1<C> {
    /// Completes one real S8/K4 round and applies its exact per-member registry disposition.
    ///
    /// # Errors
    ///
    /// Physical completion or registry reconciliation rejection closes the
    /// queue and returns terminal opaque custody.
    pub fn complete_round(
        self,
        controls: Vec<M1SpeculativeMemberControlV1>,
    ) -> Result<
        M1AuthenticatedS8CompletedFirstRoundV1<C>,
        Box<M1AuthenticatedS8PrefillExecutionFailureV1<C>>,
    > {
        let Self {
            mut registry,
            registry_identity,
            epoch,
            mut engine,
            published,
            residue,
        } = self;
        let physical = match published.complete_round(&mut engine, controls) {
            Ok(physical) => physical,
            Err(failure) => {
                let disposition = failure.into_disposition();
                return Err(terminal_failure(
                    engine,
                    M1AuthenticatedS8PrefillExecutionStageV1::RolloverPhysicalCompletion,
                    M1AuthenticatedS8PrefillExecutionErrorV1::LowerRejected,
                    (registry, disposition, residue),
                ));
            }
        };
        let outcome_header_matches = {
            let outcome = physical.outcome();
            outcome.members().len() == residue.requests.len()
                && outcome.completed_round() == 0
                && outcome.completed_epoch() == epoch
                && outcome.selection() == successor_plan().target()
        };
        if !outcome_header_matches {
            let disposition = physical.close_for_resident(&mut engine);
            return Err(terminal_failure(
                engine,
                M1AuthenticatedS8PrefillExecutionStageV1::RolloverRegistryCompletion,
                M1AuthenticatedS8PrefillExecutionErrorV1::RosterMismatch,
                (registry, disposition, residue),
            ));
        }
        let mut dispositions = Vec::new();
        if dispositions
            .try_reserve_exact(residue.requests.len())
            .is_err()
        {
            let disposition = physical.close_for_resident(&mut engine);
            return Err(terminal_failure(
                engine,
                M1AuthenticatedS8PrefillExecutionStageV1::RolloverRegistryCompletion,
                M1AuthenticatedS8PrefillExecutionErrorV1::HostAllocation,
                (registry, disposition, residue),
            ));
        }
        let mut active = Vec::new();
        if active.try_reserve_exact(residue.requests.len()).is_err() {
            let disposition = physical.close_for_resident(&mut engine);
            return Err(terminal_failure(
                engine,
                M1AuthenticatedS8PrefillExecutionStageV1::RolloverRegistryCompletion,
                M1AuthenticatedS8PrefillExecutionErrorV1::HostAllocation,
                (registry, disposition, residue),
            ));
        }
        let roster_matches = {
            let outcome = physical.outcome();
            let mut valid = true;
            for (lane, member) in outcome.members().iter().enumerate() {
                let status = member.status();
                let expected_physical = if status == M1SpeculativeMemberStatusV1::Active {
                    crate::M1DeviceKvCompletionDispositionV1::Continue
                } else {
                    crate::M1DeviceKvCompletionDispositionV1::Retire
                };
                if member.request() != residue.requests[lane]
                    || member.physical_disposition() != expected_physical
                {
                    valid = false;
                    break;
                }
                match status {
                    M1SpeculativeMemberStatusV1::Active => {
                        active.push(member.request());
                        dispositions
                            .push(M1ServingCompletionDispositionV1::Continue(successor_plan()));
                    }
                    M1SpeculativeMemberStatusV1::Completed(_)
                    | M1SpeculativeMemberStatusV1::Cancelled(_) => {
                        dispositions.push(M1ServingCompletionDispositionV1::Retire);
                    }
                }
            }
            valid && outcome.next_active_roster() == active.as_slice()
        };
        if !roster_matches {
            let disposition = physical.close_for_resident(&mut engine);
            return Err(terminal_failure(
                engine,
                M1AuthenticatedS8PrefillExecutionStageV1::RolloverRegistryCompletion,
                M1AuthenticatedS8PrefillExecutionErrorV1::RosterMismatch,
                (registry, disposition, residue),
            ));
        }
        if let Err(error) =
            registry.preflight_completion_exact_for(registry_identity, epoch, &dispositions)
        {
            let disposition = physical.close_for_resident(&mut engine);
            return Err(terminal_failure(
                engine,
                M1AuthenticatedS8PrefillExecutionStageV1::RolloverRegistryCompletion,
                M1AuthenticatedS8PrefillExecutionErrorV1::RegistryDrift,
                (registry, disposition, residue, error),
            ));
        }
        registry.apply_preflighted_completion(epoch, &dispositions);
        if let Some(pending) = physical.executor().draft_catchup_pending() {
            if let Err(error) = registry.register_draft_catchup_pending(pending) {
                let disposition = physical.close_for_resident(&mut engine);
                return Err(terminal_failure(
                    engine,
                    M1AuthenticatedS8PrefillExecutionStageV1::RolloverRegistryCompletion,
                    M1AuthenticatedS8PrefillExecutionErrorV1::RegistryDrift,
                    (registry, disposition, residue, error),
                ));
            }
        }
        Ok(M1AuthenticatedS8CompletedFirstRoundV1 {
            registry,
            engine,
            physical,
            residue,
        })
    }

    /// Closes in-flight rollover work and retains registry and physical custody.
    #[must_use = "cancelled published S8/K4 custody remains retained"]
    pub fn cancel_and_close(mut self) -> M1AuthenticatedS8CloseV1 {
        let disposition = self.published.cancel_and_close(&mut self.engine);
        let engine = self.engine.into_m1_capture_quarantine();
        M1AuthenticatedS8CloseV1::from_disposition(
            disposition,
            engine.is_faulted(),
            (self.registry, engine),
            self.residue,
        )
    }
}

impl<const C: usize> M1AuthenticatedS8CompletedFirstRoundV1<C> {
    #[must_use]
    pub fn requests(&self) -> &[RequestId] {
        &self.residue.requests
    }

    pub const fn outcome(&self) -> &crate::M1SpeculativeRoundOutcomeV1 {
        self.physical.outcome()
    }

    /// Reports physical queue teardown while retaining registry and Engine state opaquely.
    #[must_use = "closed S8 state remains opaque and retained"]
    pub fn close(mut self) -> M1AuthenticatedS8CloseV1 {
        let (executor, outcome, choices) = self.physical.into_parts();
        let teardown = executor.destroy_queue_and_retain_state(&mut self.engine);
        M1AuthenticatedS8CloseV1::from_teardown(
            teardown,
            self.engine.is_faulted(),
            (self.registry, self.engine),
            (outcome, choices, self.residue),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NonDebug(std::rc::Rc<std::cell::Cell<usize>>);

    impl Drop for NonDebug {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    #[test]
    fn opaque_debug_custody_retains_non_debug_owner_until_drop() {
        let drops = std::rc::Rc::new(std::cell::Cell::new(0));
        let owner = NonDebug(std::rc::Rc::clone(&drops));
        let retained: Box<dyn fmt::Debug> = Box::new(OpaqueExecutionCustody(Box::new(owner)));
        assert_eq!(drops.get(), 0);
        assert!(format!("{retained:?}").contains("OpaqueExecutionCustody"));
        assert_eq!(drops.get(), 0);
        drop(retained);
        assert_eq!(drops.get(), 1);
    }

    #[test]
    fn close_classifies_teardown_without_conflating_engine_quarantine_or_dropping_owners() {
        for queue_released in [false, true] {
            for engine_quarantined in [false, true] {
                let drops = std::rc::Rc::new(std::cell::Cell::new(0));
                let lower = NonDebug(std::rc::Rc::clone(&drops));
                let teardown: Result<NonDebug, NonDebug> = if queue_released {
                    Ok(lower)
                } else {
                    Err(lower)
                };
                let closed = M1AuthenticatedS8CloseV1::from_teardown(
                    teardown,
                    engine_quarantined,
                    NonDebug(std::rc::Rc::clone(&drops)),
                    NonDebug(std::rc::Rc::clone(&drops)),
                );
                assert_eq!(closed.queue_released(), queue_released);
                assert_eq!(closed.permits_stop_success(), queue_released);
                assert_eq!(closed.engine_quarantined(), engine_quarantined);
                assert!(format!("{closed:?}").contains("M1AuthenticatedS8CloseV1"));
                assert_eq!(drops.get(), 0);
                drop(closed);
                assert_eq!(drops.get(), 3);
            }
        }
    }

    #[test]
    fn close_preserves_released_and_quarantined_disposition_custody() {
        use crate::authenticated_speculative_executor::{
            quarantined_disposition, released_disposition,
        };

        for queue_released in [false, true] {
            let drops = std::rc::Rc::new(std::cell::Cell::new(0));
            let lower = OpaqueExecutionCustody(Box::new(NonDebug(std::rc::Rc::clone(&drops))));
            let disposition = if queue_released {
                released_disposition(lower)
            } else {
                quarantined_disposition(lower)
            };
            let closed = M1AuthenticatedS8CloseV1::from_disposition(
                disposition,
                true,
                NonDebug(std::rc::Rc::clone(&drops)),
                (),
            );
            assert_eq!(closed.queue_released(), queue_released);
            assert_eq!(closed.permits_stop_success(), queue_released);
            assert!(closed.engine_quarantined());
            assert!(format!("{closed:?}").contains("M1AuthenticatedS8CloseV1"));
            assert_eq!(drops.get(), 0);
            drop(closed);
            assert_eq!(drops.get(), 2);
        }
    }

    #[test]
    fn close_preserves_owner_drop_order_for_both_teardown_representations() {
        use crate::authenticated_speculative_executor::{
            quarantined_disposition, released_disposition,
        };

        struct OrderedDrop(u8, std::rc::Rc<std::cell::RefCell<Vec<u8>>>);
        impl Drop for OrderedDrop {
            fn drop(&mut self) {
                self.1.borrow_mut().push(self.0);
            }
        }

        for released in [false, true] {
            for use_disposition in [false, true] {
                let trace = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
                let before = OrderedDrop(0, std::rc::Rc::clone(&trace));
                let lower = OrderedDrop(1, std::rc::Rc::clone(&trace));
                let after = OrderedDrop(2, std::rc::Rc::clone(&trace));
                let closed = if use_disposition {
                    let lower = OpaqueExecutionCustody(Box::new(lower));
                    let disposition = if released {
                        released_disposition(lower)
                    } else {
                        quarantined_disposition(lower)
                    };
                    M1AuthenticatedS8CloseV1::from_disposition(disposition, true, before, after)
                } else {
                    let teardown: Result<OrderedDrop, OrderedDrop> = if released {
                        Ok(lower)
                    } else {
                        Err(lower)
                    };
                    M1AuthenticatedS8CloseV1::from_teardown(teardown, true, before, after)
                };
                assert!(trace.borrow().is_empty());
                drop(closed);
                assert_eq!(*trace.borrow(), [0, 1, 2]);
            }
        }
    }

    #[test]
    fn every_explicit_s8_close_returns_observable_status_without_exposing_owners() {
        let _: fn(M1AuthenticatedS8T128PrefillExecutionSuccessV1<8>) -> M1AuthenticatedS8CloseV1 =
            M1AuthenticatedS8T128PrefillExecutionSuccessV1::close;
        let _: fn(M1AuthenticatedS8ScheduledFirstRoundV1<8>) -> M1AuthenticatedS8CloseV1 =
            M1AuthenticatedS8ScheduledFirstRoundV1::cancel_and_close;
        let _: fn(M1AuthenticatedS8PreparedFirstRoundV1<8>) -> M1AuthenticatedS8CloseV1 =
            M1AuthenticatedS8PreparedFirstRoundV1::cancel_and_close;
        let _: fn(M1AuthenticatedS8PublishedFirstRoundV1<8>) -> M1AuthenticatedS8CloseV1 =
            M1AuthenticatedS8PublishedFirstRoundV1::cancel_and_close;
        let _: fn(M1AuthenticatedS8CompletedFirstRoundV1<8>) -> M1AuthenticatedS8CloseV1 =
            M1AuthenticatedS8CompletedFirstRoundV1::close;
    }

    fn choices(values: &[u32]) -> M1ObservedDirectDiagnosticChoicesV1 {
        M1ObservedDirectDiagnosticChoicesV1::for_serving_history_test(
            values.to_vec().into_boxed_slice(),
        )
    }

    #[test]
    fn direct_choice_roster_requires_exact_live_cardinality_and_order() {
        let exact = choices(&[11, 13, 17]);
        assert_eq!(
            require_authenticated_prefill_choices(&exact, 3),
            Ok(&[11, 13, 17][..])
        );
        for expected in [2, 4] {
            assert_eq!(
                require_authenticated_prefill_choices(&exact, expected),
                Err(
                    M1AuthenticatedS8PrefillExecutionErrorV1::DirectChoiceCardinality {
                        expected,
                        actual: 3,
                    }
                )
            );
        }
    }

    #[test]
    fn production_orders_physical_submit_before_registry_record_and_evidence_before_settlement() {
        let source = include_str!("authenticated_s8_prefill_execution.rs");
        let production = source.split("#[cfg(test)]").next().unwrap();
        let submit = production.find("queue.submit()").unwrap();
        let record = production
            .find("registry.record_publication(reservation)")
            .unwrap();
        let direct = production
            .find("observe_direct_diagnostic_choices()")
            .unwrap();
        let settle = production
            .find("complete_m1_authenticated_physical_step_v1(")
            .unwrap();
        assert!(submit < record);
        assert!(record < direct);
        assert!(direct < settle);
    }

    #[test]
    fn first_round_reuses_finite_rollover_without_singleton_reconciliation() {
        let source = include_str!("authenticated_s8_prefill_execution.rs");
        let production = source.split("#[cfg(test)]").next().unwrap();
        assert!(production.contains("M1FiniteSpeculativeQueueRolloverKvInputsV1::from_lane_leases"));
        assert!(production
            .contains("schedule_m1_authenticated_speculative_rollover_with_recipe_input_v1"));
        assert!(production.contains("registry.register_draft_catchup_pending(pending)"));
        assert!(!production.contains("reconcile_m1_authenticated_s1_t128"));
        assert!(!production.contains("M1AuthenticatedS1T128PrefillExecutionSuccessV1"));
    }
}
