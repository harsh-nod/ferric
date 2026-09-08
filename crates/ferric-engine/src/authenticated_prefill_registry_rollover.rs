//! Opaque registry integration for the first authenticated speculative round.

use core::{any::Any, fmt};

use ferric_spec::{completion::CompletionEpoch, RequestId, ValidatedM1StepInputs};

use crate::{
    prepare_m1_authenticated_speculative_rollover_v1,
    submit_m1_authenticated_speculative_rollover_v1, Engine, LogicalRunnerDeclaration,
    M1AuthenticatedPrefillRegistryReconciledV1, M1AuthenticatedPreparedSpeculativeRolloverV1,
    M1AuthenticatedScheduledSpeculativeRolloverV1,
    M1AuthenticatedSpeculativeExecutorTeardownFailureV1,
    M1AuthenticatedSpeculativeExecutorTeardownSuccessV1,
    M1AuthenticatedSpeculativePhysicalRoundSuccessV1,
    M1AuthenticatedSpeculativeRolloverPublishedV1,
    M1AuthenticatedSpeculativeRolloverScheduleFailureV1,
    M1FiniteSpeculativeQueueRolloverKvInputsV1, M1FullStepWorkspacePlans,
    M1ObservedDirectDiagnosticChoicesV1, M1ObservedSpeculativeDiagnosticChoicesV1,
    M1QueueWaitTimeoutV1, M1ServingCompletionDispositionV1, M1ServingPlanV1,
    M1ServingPublicationReservationV1, M1ServingRegistryV1, M1SpeculativeGenerationLoopV1,
    M1SpeculativeGenerationPolicyV1, M1SpeculativeMemberControlV1, M1SpeculativeMemberSeedV1,
    M1SpeculativeMemberStatusV1, M1SpeculativeRoundOutcomeV1,
};

/// Caller-supplied addressless inputs for the first S1/K4 successor.
///
/// Page leases, queue custody, the physical runner, and model allocations are
/// deliberately absent. They remain inside the reconciled prefill owner.
#[must_use = "first-round inputs must enter the reconciled rollover transition"]
#[derive(Debug)]
pub struct M1AuthenticatedPrefillRegistryFirstRoundInputsV1 {
    draft_decode: ValidatedM1StepInputs,
    target_speculative: ValidatedM1StepInputs,
    recipe_plans: crate::runner::M1PhysicalRunnerRecipeInputV1,
    preparation_plans: M1FullStepWorkspacePlans,
}

impl M1AuthenticatedPrefillRegistryFirstRoundInputsV1 {
    /// Joins validated logical inputs to two independently consumed workspace
    /// plan sets. Physical tail-page authority remains private.
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

    pub(crate) const fn new_with_recipe_input(
        draft_decode: ValidatedM1StepInputs,
        target_speculative: ValidatedM1StepInputs,
        recipe_plans: crate::runner::M1PhysicalRunnerRecipeInputV1,
        preparation_plans: M1FullStepWorkspacePlans,
    ) -> Self {
        Self {
            draft_decode,
            target_speculative,
            recipe_plans,
            preparation_plans,
        }
    }
}

/// Stable stage for the integrated prefill-to-first-round transition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedPrefillRegistryFirstRoundStageV1 {
    RegistryPlan,
    RegistryReservation,
    InputJoin,
    Coordinator,
    Schedule,
    Prepare,
    Submit,
    Deadline,
    RegistryPublication,
    PhysicalCompletion,
    RegistryCompletion,
}

/// Opaque retained or terminal custody for every integrated transition error.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedPrefillRegistryFirstRoundFailureV1;
/// fn extract(failure: M1AuthenticatedPrefillRegistryFirstRoundFailureV1) {
///     let _ = failure.into_parts();
/// }
/// ```
#[must_use = "failed first-round custody remains opaque and retained"]
pub struct M1AuthenticatedPrefillRegistryFirstRoundFailureV1 {
    stage: M1AuthenticatedPrefillRegistryFirstRoundStageV1,
    engine_quarantined: bool,
    teardown: crate::authenticated_resident_session::M1AuthenticatedResidentQueueTeardownV1,
}

impl fmt::Debug for M1AuthenticatedPrefillRegistryFirstRoundFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedPrefillRegistryFirstRoundFailureV1")
            .field("stage", &self.stage)
            .field("engine_quarantined", &self.engine_quarantined)
            .finish_non_exhaustive()
    }
}

impl M1AuthenticatedPrefillRegistryFirstRoundFailureV1 {
    #[must_use]
    pub const fn stage(&self) -> M1AuthenticatedPrefillRegistryFirstRoundStageV1 {
        self.stage
    }

    #[must_use]
    pub const fn engine_quarantined(&self) -> bool {
        self.engine_quarantined
    }

    pub(crate) fn into_resident_teardown(
        self,
    ) -> crate::authenticated_resident_session::M1AuthenticatedResidentQueueTeardownV1 {
        self.teardown.retain((self.stage, self.engine_quarantined))
    }
}

fn first_round_failure(
    stage: M1AuthenticatedPrefillRegistryFirstRoundStageV1,
    engine_quarantined: bool,
    teardown: crate::authenticated_resident_session::M1AuthenticatedResidentQueueTeardownV1,
) -> M1AuthenticatedPrefillRegistryFirstRoundFailureV1 {
    M1AuthenticatedPrefillRegistryFirstRoundFailureV1 {
        stage,
        engine_quarantined,
        teardown,
    }
}

fn teardown_from_disposition(
    disposition: crate::M1AuthenticatedSpeculativeFailureDispositionV1,
    retained: impl Any,
) -> crate::authenticated_resident_session::M1AuthenticatedResidentQueueTeardownV1 {
    crate::authenticated_resident_session::M1AuthenticatedResidentQueueTeardownV1::from_speculative_disposition(
        disposition,
    )
    .retain(retained)
}

fn teardown_from_result<T: Any, E: Any>(
    result: Result<T, E>,
    retained: impl Any,
) -> crate::authenticated_resident_session::M1AuthenticatedResidentQueueTeardownV1 {
    use crate::authenticated_resident_session::M1AuthenticatedResidentQueueTeardownV1 as Teardown;
    if result.is_ok() {
        Teardown::released((result, retained))
    } else {
        Teardown::quarantined((result, retained))
    }
}

#[derive(Debug)]
struct M1AuthenticatedPrefillRegistryFirstRoundResidueV1 {
    request: RequestId,
    first_token: ferric_spec::TokenId,
    successor: M1ServingPlanV1,
    _direct_choices: M1ObservedDirectDiagnosticChoicesV1,
    _remaining_target_pages: Vec<crate::DeviceKvPageLease>,
    _prompt_tokens: Box<[ferric_spec::TokenId]>,
    _policy: M1SpeculativeGenerationPolicyV1,
    diagnostic_ring_bytes: u32,
    queue_wait_timeout: M1QueueWaitTimeoutV1,
}

/// Reconciled registry reservation joined to a scheduled physical rollover.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedPrefillRegistryScheduledFirstRoundV1;
/// fn extract(value: M1AuthenticatedPrefillRegistryScheduledFirstRoundV1<8>) {
///     let _ = value.into_parts();
/// }
/// ```
#[must_use = "scheduled registry and physical custody must be prepared"]
pub struct M1AuthenticatedPrefillRegistryScheduledFirstRoundV1<const C: usize> {
    registry: M1ServingRegistryV1<C>,
    reservation: M1ServingPublicationReservationV1,
    engine: Engine<C>,
    scheduled: M1AuthenticatedScheduledSpeculativeRolloverV1,
    residue: M1AuthenticatedPrefillRegistryFirstRoundResidueV1,
}

impl<const C: usize> fmt::Debug for M1AuthenticatedPrefillRegistryScheduledFirstRoundV1<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedPrefillRegistryScheduledFirstRoundV1")
            .field("request", &self.residue.request)
            .field("epoch", &self.reservation.epoch())
            .finish_non_exhaustive()
    }
}

/// Prepared first-round physical rollover with its live registry reservation.
#[must_use = "prepared registry and physical custody must be published"]
pub struct M1AuthenticatedPrefillRegistryPreparedFirstRoundV1<const C: usize> {
    registry: M1ServingRegistryV1<C>,
    reservation: M1ServingPublicationReservationV1,
    engine: Engine<C>,
    prepared: M1AuthenticatedPreparedSpeculativeRolloverV1,
    residue: M1AuthenticatedPrefillRegistryFirstRoundResidueV1,
}

impl<const C: usize> fmt::Debug for M1AuthenticatedPrefillRegistryPreparedFirstRoundV1<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedPrefillRegistryPreparedFirstRoundV1")
            .field("request", &self.residue.request)
            .field("epoch", &self.reservation.epoch())
            .finish_non_exhaustive()
    }
}

/// Published first speculative generation with the matching registry in-flight.
#[must_use = "published custody must consume authenticated physical completion"]
pub struct M1AuthenticatedPrefillRegistryPublishedFirstRoundV1<const C: usize> {
    registry: M1ServingRegistryV1<C>,
    registry_identity: crate::M1ServingRegistryIdentityV1,
    epoch: CompletionEpoch,
    engine: Engine<C>,
    published: M1AuthenticatedSpeculativeRolloverPublishedV1,
    residue: M1AuthenticatedPrefillRegistryFirstRoundResidueV1,
}

impl<const C: usize> fmt::Debug for M1AuthenticatedPrefillRegistryPublishedFirstRoundV1<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedPrefillRegistryPublishedFirstRoundV1")
            .field("request", &self.residue.request)
            .field("epoch", &self.epoch)
            .finish_non_exhaustive()
    }
}

/// Authenticated physical and registry completion for first S1/K4 round.
#[must_use = "completed first-round queue custody must be closed or advanced"]
pub struct M1AuthenticatedPrefillRegistryCompletedFirstRoundV1<const C: usize> {
    registry: M1ServingRegistryV1<C>,
    engine: Engine<C>,
    physical: M1AuthenticatedSpeculativePhysicalRoundSuccessV1,
    residue: M1AuthenticatedPrefillRegistryFirstRoundResidueV1,
}

impl<const C: usize> fmt::Debug for M1AuthenticatedPrefillRegistryCompletedFirstRoundV1<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedPrefillRegistryCompletedFirstRoundV1")
            .field("request", &self.residue.request)
            .field("epoch", &self.physical.outcome().completed_epoch())
            .finish_non_exhaustive()
    }
}

/// Queue-release witness that intentionally retains active logical and KV state.
///
/// This is not a request-teardown or resource-release witness. The registry,
/// Engine, KV pages, program owners, authenticated outcome, and diagnostic
/// choices remain opaque and retained for a later resident-session transition.
#[must_use = "queue-released state-retained custody must remain retained"]
pub struct M1AuthenticatedPrefillRegistryFirstRoundQueueReleasedStateRetainedV1<const C: usize> {
    _registry: M1ServingRegistryV1<C>,
    _engine: Engine<C>,
    _teardown: M1AuthenticatedSpeculativeExecutorTeardownSuccessV1,
    outcome: M1SpeculativeRoundOutcomeV1,
    _choices: M1ObservedSpeculativeDiagnosticChoicesV1,
    residue: M1AuthenticatedPrefillRegistryFirstRoundResidueV1,
}

impl<const C: usize> fmt::Debug
    for M1AuthenticatedPrefillRegistryFirstRoundQueueReleasedStateRetainedV1<C>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedPrefillRegistryFirstRoundQueueReleasedStateRetainedV1")
            .field("request", &self.residue.request)
            .field("epoch", &self.outcome.completed_epoch())
            .finish_non_exhaustive()
    }
}

/// Opaque terminal quarantine witness after queue closure fails.
#[must_use = "quarantined terminal custody must remain retained"]
pub struct M1AuthenticatedPrefillRegistryFirstRoundQuarantineV1<const C: usize> {
    _registry: M1ServingRegistryV1<C>,
    _engine: Engine<C>,
    _teardown: Box<M1AuthenticatedSpeculativeExecutorTeardownFailureV1>,
    outcome: M1SpeculativeRoundOutcomeV1,
    _choices: M1ObservedSpeculativeDiagnosticChoicesV1,
    residue: M1AuthenticatedPrefillRegistryFirstRoundResidueV1,
}

impl<const C: usize> fmt::Debug for M1AuthenticatedPrefillRegistryFirstRoundQuarantineV1<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedPrefillRegistryFirstRoundQuarantineV1")
            .field("request", &self.residue.request)
            .field("epoch", &self.outcome.completed_epoch())
            .finish_non_exhaustive()
    }
}

/// Explicit queue disposition for a completed integrated round.
///
/// Neither variant proves request teardown or release of logical/KV resources.
#[must_use = "queue disposition custody must remain retained"]
#[derive(Debug)]
pub enum M1AuthenticatedPrefillRegistryFirstRoundClosureV1<const C: usize> {
    QueueReleasedStateRetained(
        Box<M1AuthenticatedPrefillRegistryFirstRoundQueueReleasedStateRetainedV1<C>>,
    ),
    Quarantined(Box<M1AuthenticatedPrefillRegistryFirstRoundQuarantineV1<C>>),
}

impl<const C: usize> M1AuthenticatedPrefillRegistryScheduledFirstRoundV1<C> {
    #[must_use]
    pub const fn request(&self) -> RequestId {
        self.residue.request
    }

    #[must_use]
    pub const fn epoch(&self) -> CompletionEpoch {
        self.reservation.epoch()
    }

    /// Prepares physical rollover workspaces while retaining the registry
    /// reservation inside the returned owner.
    ///
    /// # Errors
    ///
    /// Failure closes or quarantines detached physical custody and retains the
    /// registry, Engine, and all successor inputs opaquely.
    pub fn prepare(
        self,
        runner: &LogicalRunnerDeclaration,
    ) -> Result<
        M1AuthenticatedPrefillRegistryPreparedFirstRoundV1<C>,
        M1AuthenticatedPrefillRegistryFirstRoundFailureV1,
    > {
        let Self {
            mut registry,
            reservation,
            mut engine,
            scheduled,
            residue,
        } = self;
        let prepared = match prepare_m1_authenticated_speculative_rollover_v1(
            &mut engine,
            scheduled,
            runner,
        ) {
            Ok(prepared) => prepared,
            Err(source) => {
                let abort = registry.abort_publication(reservation);
                let disposition = source.into_disposition();
                return Err(first_round_failure(
                    M1AuthenticatedPrefillRegistryFirstRoundStageV1::Prepare,
                    engine.is_faulted(),
                    teardown_from_disposition(disposition, (engine, registry, abort, residue)),
                ));
            }
        };
        Ok(M1AuthenticatedPrefillRegistryPreparedFirstRoundV1 {
            registry,
            reservation,
            engine,
            prepared,
            residue,
        })
    }

    /// Prepares with the declaration already sealed into authenticated queue
    /// custody, avoiding any independently supplied logical runner.
    pub(crate) fn prepare_retained(
        self,
    ) -> Result<
        M1AuthenticatedPrefillRegistryPreparedFirstRoundV1<C>,
        M1AuthenticatedPrefillRegistryFirstRoundFailureV1,
    > {
        let Self {
            mut registry,
            reservation,
            mut engine,
            scheduled,
            residue,
        } = self;
        let prepared = match crate::authenticated_queue_rollover::prepare_m1_authenticated_speculative_rollover_retained_v1(
            &mut engine,
            scheduled,
        ) {
            Ok(prepared) => prepared,
            Err(source) => {
                let abort = registry.abort_publication(reservation);
                let disposition = source.into_disposition();
                return Err(first_round_failure(
                    M1AuthenticatedPrefillRegistryFirstRoundStageV1::Prepare,
                    engine.is_faulted(),
                    teardown_from_disposition(
                        disposition,
                        (engine, registry, abort, residue),
                    ),
                ));
            }
        };
        Ok(M1AuthenticatedPrefillRegistryPreparedFirstRoundV1 {
            registry,
            reservation,
            engine,
            prepared,
            residue,
        })
    }
}

impl<const C: usize> M1AuthenticatedPrefillRegistryPreparedFirstRoundV1<C> {
    #[must_use]
    pub const fn request(&self) -> RequestId {
        self.residue.request
    }

    #[must_use]
    pub const fn epoch(&self) -> CompletionEpoch {
        self.reservation.epoch()
    }

    /// Submits the physical rollover, then records the exact retained registry
    /// reservation. No registry publication is synthesized ahead of submit.
    ///
    /// # Errors
    ///
    /// Failure returns opaque terminal or retained custody. Any theoretically
    /// impossible post-submit registry drift faults the Engine and retains the
    /// published queue rather than releasing it through another path.
    pub fn publish(
        self,
    ) -> Result<
        M1AuthenticatedPrefillRegistryPublishedFirstRoundV1<C>,
        M1AuthenticatedPrefillRegistryFirstRoundFailureV1,
    > {
        self.publish_with_deadline(|_, timeout| Some(timeout))
    }

    pub(crate) fn publish_with_deadline(
        self,
        mut deadline_expired: impl FnMut(
            crate::authenticated_resident_session::M1AuthenticatedResidentDeadlineBoundaryV1,
            M1QueueWaitTimeoutV1,
        ) -> Option<M1QueueWaitTimeoutV1>,
    ) -> Result<
        M1AuthenticatedPrefillRegistryPublishedFirstRoundV1<C>,
        M1AuthenticatedPrefillRegistryFirstRoundFailureV1,
    > {
        use crate::authenticated_resident_session::M1AuthenticatedResidentDeadlineBoundaryV1 as Boundary;

        let Self {
            mut registry,
            reservation,
            mut engine,
            prepared,
            residue,
        } = self;
        if let Err(error) = registry.preflight_publication(&reservation) {
            engine.quarantine_m1_queue_rearm_failure();
            let disposition = prepared.cancel_and_close(&mut engine);
            return Err(first_round_failure(
                M1AuthenticatedPrefillRegistryFirstRoundStageV1::RegistryPublication,
                true,
                teardown_from_disposition(
                    disposition,
                    (engine, registry, reservation, residue, error),
                ),
            ));
        }
        let registry_identity = reservation.registry_identity();
        let epoch = reservation.epoch();
        if deadline_expired(Boundary::BeforeQueueSubmit, residue.queue_wait_timeout).is_none() {
            engine.quarantine_m1_queue_rearm_failure();
            let abort = registry.abort_publication(reservation);
            let disposition = prepared.cancel_and_close(&mut engine);
            return Err(first_round_failure(
                M1AuthenticatedPrefillRegistryFirstRoundStageV1::Deadline,
                true,
                teardown_from_disposition(disposition, (engine, registry, abort, residue)),
            ));
        }
        let published = match submit_m1_authenticated_speculative_rollover_v1(
            &mut engine,
            prepared,
            residue.diagnostic_ring_bytes,
            residue.queue_wait_timeout,
        ) {
            Ok(published) => published,
            Err(source) => {
                let abort = registry.abort_publication(reservation);
                let disposition = source.into_disposition();
                return Err(first_round_failure(
                    M1AuthenticatedPrefillRegistryFirstRoundStageV1::Submit,
                    engine.is_faulted(),
                    teardown_from_disposition(disposition, (engine, registry, abort, residue)),
                ));
            }
        };
        if deadline_expired(Boundary::AfterQueueSubmit, residue.queue_wait_timeout).is_none() {
            engine.quarantine_m1_queue_rearm_failure();
            let abort = registry.abort_publication(reservation);
            let disposition = published.cancel_and_close(&mut engine);
            return Err(first_round_failure(
                M1AuthenticatedPrefillRegistryFirstRoundStageV1::Deadline,
                true,
                teardown_from_disposition(disposition, (engine, registry, abort, residue)),
            ));
        }
        if let Err(error) = registry.record_publication(reservation) {
            engine.quarantine_m1_queue_rearm_failure();
            let disposition = published.cancel_and_close(&mut engine);
            return Err(first_round_failure(
                M1AuthenticatedPrefillRegistryFirstRoundStageV1::RegistryPublication,
                true,
                teardown_from_disposition(disposition, (engine, registry, error, residue)),
            ));
        }
        Ok(M1AuthenticatedPrefillRegistryPublishedFirstRoundV1 {
            registry,
            registry_identity,
            epoch,
            engine,
            published,
            residue,
        })
    }
}

impl<const C: usize> M1AuthenticatedPrefillRegistryPublishedFirstRoundV1<C> {
    #[must_use]
    pub const fn request(&self) -> RequestId {
        self.residue.request
    }

    #[must_use]
    pub const fn epoch(&self) -> CompletionEpoch {
        self.epoch
    }

    /// Completes the physical round from its authenticated checked and choice
    /// evidence, then applies the exact matching registry disposition.
    ///
    /// The caller controls only cancellation policy. It supplies no token,
    /// accepted-prefix, completion, or verification oracle.
    ///
    /// # Errors
    ///
    /// Physical failure is already closed or quarantined by the lower state
    /// machine. Any impossible registry or outcome drift explicitly faults the
    /// Engine and retains all post-completion custody opaquely.
    pub fn complete_round(
        self,
        controls: Vec<M1SpeculativeMemberControlV1>,
    ) -> Result<
        M1AuthenticatedPrefillRegistryCompletedFirstRoundV1<C>,
        M1AuthenticatedPrefillRegistryFirstRoundFailureV1,
    > {
        self.complete_round_with_deadline(controls, |_, timeout| Some(timeout))
    }

    pub(crate) fn complete_round_with_deadline(
        self,
        controls: Vec<M1SpeculativeMemberControlV1>,
        deadline_expired: impl FnMut(
            crate::authenticated_resident_session::M1AuthenticatedResidentDeadlineBoundaryV1,
            M1QueueWaitTimeoutV1,
        ) -> Option<M1QueueWaitTimeoutV1>,
    ) -> Result<
        M1AuthenticatedPrefillRegistryCompletedFirstRoundV1<C>,
        M1AuthenticatedPrefillRegistryFirstRoundFailureV1,
    > {
        let Self {
            mut registry,
            registry_identity,
            epoch,
            mut engine,
            published,
            residue,
        } = self;
        let physical =
            match published.complete_round_with_deadline(&mut engine, controls, deadline_expired) {
                Ok(physical) => physical,
                Err(source) => {
                    let stage = if source.stage()
                        == crate::M1AuthenticatedSpeculativePhysicalRoundStageV1::Deadline
                    {
                        M1AuthenticatedPrefillRegistryFirstRoundStageV1::Deadline
                    } else {
                        M1AuthenticatedPrefillRegistryFirstRoundStageV1::PhysicalCompletion
                    };
                    return Err(first_round_failure(
                        stage,
                        engine.is_faulted(),
                        teardown_from_disposition(
                            source.into_disposition(),
                            (engine, registry, residue),
                        ),
                    ));
                }
            };
        let outcome = physical.outcome();
        let [member] = outcome.members() else {
            engine.quarantine_m1_queue_rearm_failure();
            let disposition = physical.close_for_resident(&mut engine);
            return Err(first_round_failure(
                M1AuthenticatedPrefillRegistryFirstRoundStageV1::RegistryCompletion,
                true,
                teardown_from_disposition(disposition, (engine, registry, residue)),
            ));
        };
        let active = member.status() == M1SpeculativeMemberStatusV1::Active;
        let expected_physical = if active {
            crate::M1DeviceKvCompletionDispositionV1::Continue
        } else {
            crate::M1DeviceKvCompletionDispositionV1::Retire
        };
        let next_roster_matches = if active {
            outcome.next_active_roster() == [residue.request]
        } else {
            outcome.next_active_roster().is_empty()
        };
        if outcome.completed_round() != 0
            || outcome.completed_epoch() != epoch
            || outcome.selection() != residue.successor.target()
            || member.request() != residue.request
            || member.physical_disposition() != expected_physical
            || !next_roster_matches
        {
            engine.quarantine_m1_queue_rearm_failure();
            let disposition = physical.close_for_resident(&mut engine);
            return Err(first_round_failure(
                M1AuthenticatedPrefillRegistryFirstRoundStageV1::RegistryCompletion,
                true,
                teardown_from_disposition(disposition, (engine, registry, residue)),
            ));
        }
        let disposition = match member.status() {
            M1SpeculativeMemberStatusV1::Active => {
                M1ServingCompletionDispositionV1::Continue(residue.successor)
            }
            M1SpeculativeMemberStatusV1::Completed(_)
            | M1SpeculativeMemberStatusV1::Cancelled(_) => M1ServingCompletionDispositionV1::Retire,
        };
        if let Err(error) =
            registry.preflight_completion_exact_for(registry_identity, epoch, &[disposition])
        {
            engine.quarantine_m1_queue_rearm_failure();
            let disposition = physical.close_for_resident(&mut engine);
            return Err(first_round_failure(
                M1AuthenticatedPrefillRegistryFirstRoundStageV1::RegistryCompletion,
                true,
                teardown_from_disposition(disposition, (engine, registry, residue, error)),
            ));
        }
        registry.apply_preflighted_completion(epoch, &[disposition]);
        Ok(M1AuthenticatedPrefillRegistryCompletedFirstRoundV1 {
            registry,
            engine,
            physical,
            residue,
        })
    }
}

impl<const C: usize> M1AuthenticatedPrefillRegistryCompletedFirstRoundV1<C> {
    #[must_use]
    pub const fn request(&self) -> RequestId {
        self.residue.request
    }

    #[must_use]
    pub const fn first_token(&self) -> ferric_spec::TokenId {
        self.residue.first_token
    }

    #[must_use = "authenticated outcome remains borrowed without exposing executor custody"]
    pub const fn outcome(&self) -> &M1SpeculativeRoundOutcomeV1 {
        self.physical.outcome()
    }

    pub(crate) fn into_resident_parts(
        self,
    ) -> (
        M1ServingRegistryV1<C>,
        Engine<C>,
        M1AuthenticatedSpeculativePhysicalRoundSuccessV1,
        RequestId,
        ferric_spec::TokenId,
        M1ServingPlanV1,
    ) {
        (
            self.registry,
            self.engine,
            self.physical,
            self.residue.request,
            self.residue.first_token,
            self.residue.successor,
        )
    }

    /// Explicitly destroys the live speculative queue and retains all logical,
    /// KV, program, registry, and Engine state in either returned witness.
    ///
    /// This transition does not cancel the request or release its resources.
    #[must_use = "queue disposition custody must remain retained"]
    pub fn close(self) -> M1AuthenticatedPrefillRegistryFirstRoundClosureV1<C> {
        let Self {
            registry,
            mut engine,
            physical,
            residue,
        } = self;
        let (executor, outcome, choices) = physical.into_parts();
        match executor.destroy_queue_and_retain_state(&mut engine) {
            Ok(teardown) => {
                M1AuthenticatedPrefillRegistryFirstRoundClosureV1::QueueReleasedStateRetained(
                    Box::new(
                        M1AuthenticatedPrefillRegistryFirstRoundQueueReleasedStateRetainedV1 {
                            _registry: registry,
                            _engine: engine,
                            _teardown: teardown,
                            outcome,
                            _choices: choices,
                            residue,
                        },
                    ),
                )
            }
            Err(teardown) => M1AuthenticatedPrefillRegistryFirstRoundClosureV1::Quarantined(
                Box::new(M1AuthenticatedPrefillRegistryFirstRoundQuarantineV1 {
                    _registry: registry,
                    _engine: engine,
                    _teardown: teardown,
                    outcome,
                    _choices: choices,
                    residue,
                }),
            ),
        }
    }
}

impl<const C: usize> M1AuthenticatedPrefillRegistryFirstRoundQueueReleasedStateRetainedV1<C> {
    #[must_use]
    pub const fn request(&self) -> RequestId {
        self.residue.request
    }

    #[must_use = "authenticated outcome remains borrowed without exposing executor custody"]
    pub const fn outcome(&self) -> &M1SpeculativeRoundOutcomeV1 {
        &self.outcome
    }
}

impl<const C: usize> M1AuthenticatedPrefillRegistryFirstRoundQuarantineV1<C> {
    #[must_use]
    pub const fn request(&self) -> RequestId {
        self.residue.request
    }

    #[must_use = "authenticated outcome remains borrowed without exposing executor custody"]
    pub const fn outcome(&self) -> &M1SpeculativeRoundOutcomeV1 {
        &self.outcome
    }
}

impl<const C: usize> M1AuthenticatedPrefillRegistryReconciledV1<C> {
    /// Plans and reserves the exact epoch-two registry successor, then consumes
    /// the reconciled paired-prefill owner into authenticated rollover schedule.
    ///
    /// # Errors
    ///
    /// Pure registry or input rejection retains the complete reconciled owner.
    /// A lower scheduling rejection is explicitly aborted and closed, or
    /// retains the lower terminal disposition, together with registry custody.
    pub fn schedule_first_speculative_round(
        self,
        inputs: M1AuthenticatedPrefillRegistryFirstRoundInputsV1,
    ) -> Result<
        M1AuthenticatedPrefillRegistryScheduledFirstRoundV1<C>,
        M1AuthenticatedPrefillRegistryFirstRoundFailureV1,
    > {
        let expected_request = self.request();
        let expected_epoch = self.successor_epoch();
        let expected_successor = self.successor_plan();
        let (mut registry, completed) = self.into_parts();
        let batch = match registry.plan_next() {
            Ok(Some(batch)) => batch,
            Ok(None) => {
                let teardown = completed.close_for_resident().retain((registry, inputs));
                return Err(first_round_failure(
                    M1AuthenticatedPrefillRegistryFirstRoundStageV1::RegistryPlan,
                    false,
                    teardown,
                ));
            }
            Err(error) => {
                let teardown = completed
                    .close_for_resident()
                    .retain((registry, inputs, error));
                return Err(first_round_failure(
                    M1AuthenticatedPrefillRegistryFirstRoundStageV1::RegistryPlan,
                    false,
                    teardown,
                ));
            }
        };
        if batch.plan() != expected_successor
            || batch.epoch() != expected_epoch
            || batch.requests() != [expected_request]
        {
            let teardown = completed
                .close_for_resident()
                .retain((registry, inputs, batch));
            return Err(first_round_failure(
                M1AuthenticatedPrefillRegistryFirstRoundStageV1::RegistryPlan,
                false,
                teardown,
            ));
        }
        let reservation = match registry.reserve_publication(batch) {
            Ok(reservation) => reservation,
            Err(error) => {
                let teardown = completed
                    .close_for_resident()
                    .retain((registry, inputs, error));
                return Err(first_round_failure(
                    M1AuthenticatedPrefillRegistryFirstRoundStageV1::RegistryReservation,
                    false,
                    teardown,
                ));
            }
        };
        if let Err(error) = registry.preflight_publication(&reservation) {
            let abort = registry.abort_publication(reservation);
            let teardown = completed
                .close_for_resident()
                .retain((registry, inputs, abort, error));
            return Err(first_round_failure(
                M1AuthenticatedPrefillRegistryFirstRoundStageV1::RegistryReservation,
                false,
                teardown,
            ));
        }
        let batch = reservation.physical_batch();
        let (
            mut engine,
            released,
            first_token,
            direct_choices,
            draft_rollover_page,
            mut target_rollover_pages,
            intent,
            request,
            prompt_tokens,
            policy,
            diagnostic_ring_bytes,
            queue_wait_timeout,
        ) = completed.into_parts();
        let M1AuthenticatedPrefillRegistryFirstRoundInputsV1 {
            draft_decode,
            target_speculative,
            recipe_plans,
            preparation_plans,
        } = inputs;
        if target_rollover_pages.is_empty() {
            let abort = registry.abort_publication(reservation);
            let teardown = released.destroy_queue_and_retain_step(&mut engine);
            return Err(first_round_failure(
                M1AuthenticatedPrefillRegistryFirstRoundStageV1::InputJoin,
                engine.is_faulted(),
                teardown_from_result(
                    teardown,
                    (
                        engine,
                        registry,
                        abort,
                        draft_rollover_page,
                        target_rollover_pages,
                        direct_choices,
                        prompt_tokens,
                        policy,
                        intent,
                        draft_decode,
                        target_speculative,
                        recipe_plans,
                        preparation_plans,
                    ),
                ),
            ));
        }
        let remaining_target_pages = target_rollover_pages.split_off(1);
        let rollover_inputs = M1FiniteSpeculativeQueueRolloverKvInputsV1::new(
            draft_decode,
            target_speculative,
            vec![draft_rollover_page],
            target_rollover_pages,
        );
        let seed = M1SpeculativeMemberSeedV1::new(request, first_token, 128, 128, policy);
        let coordinator =
            match M1SpeculativeGenerationLoopV1::new(expected_successor.target(), &[seed]) {
                Ok(coordinator) => coordinator,
                Err(error) => {
                    let abort = registry.abort_publication(reservation);
                    let teardown = released.destroy_queue_and_retain_step(&mut engine);
                    return Err(first_round_failure(
                        M1AuthenticatedPrefillRegistryFirstRoundStageV1::Coordinator,
                        engine.is_faulted(),
                        teardown_from_result(
                            teardown,
                            (
                                engine,
                                registry,
                                abort,
                                intent,
                                rollover_inputs,
                                recipe_plans,
                                preparation_plans,
                                direct_choices,
                                remaining_target_pages,
                                prompt_tokens,
                                error,
                            ),
                        ),
                    ));
                }
            };
        let residue = M1AuthenticatedPrefillRegistryFirstRoundResidueV1 {
            request,
            first_token,
            successor: expected_successor,
            _direct_choices: direct_choices,
            _remaining_target_pages: remaining_target_pages,
            _prompt_tokens: prompt_tokens,
            _policy: policy,
            diagnostic_ring_bytes,
            queue_wait_timeout,
        };
        let scheduled = match crate::authenticated_queue_rollover::schedule_m1_authenticated_speculative_rollover_with_recipe_input_v1(
            &mut engine,
            released,
            &batch,
            intent,
            coordinator,
            rollover_inputs,
            recipe_plans,
            preparation_plans,
        ) {
            Ok(scheduled) => scheduled,
            Err(M1AuthenticatedSpeculativeRolloverScheduleFailureV1::PreDetach {
                error,
                retry,
            }) => {
                let abort = registry.abort_publication(reservation);
                let disposition = retry.cancel_and_close(&mut engine);
                return Err(first_round_failure(
                    M1AuthenticatedPrefillRegistryFirstRoundStageV1::Schedule,
                    engine.is_faulted(),
                    teardown_from_disposition(
                        disposition,
                        (engine, registry, abort, error, residue),
                    ),
                ));
            }
            Err(M1AuthenticatedSpeculativeRolloverScheduleFailureV1::Terminal {
                error,
                disposition,
            }) => {
                let abort = registry.abort_publication(reservation);
                return Err(first_round_failure(
                    M1AuthenticatedPrefillRegistryFirstRoundStageV1::Schedule,
                    engine.is_faulted(),
                    teardown_from_disposition(
                        disposition,
                        (engine, registry, abort, error, residue),
                    ),
                ));
            }
        };
        Ok(M1AuthenticatedPrefillRegistryScheduledFirstRoundV1 {
            registry,
            reservation,
            engine,
            scheduled,
            residue,
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn integrated_first_round_source_preserves_linear_authority_order() {
        let source = include_str!("authenticated_prefill_registry_rollover.rs");
        let production = source.split("#[cfg(test)]").next().unwrap();
        for forbidden in [
            ".unwrap(",
            ".expect(",
            "panic!(",
            "unreachable!(",
            ".remove(0)",
        ] {
            assert!(
                !production.contains(forbidden),
                "forbidden integrated production path: {forbidden}"
            );
        }
        assert!(!production.contains("pub fn into_parts"));

        let schedule = production
            .split("pub fn schedule_first_speculative_round")
            .nth(1)
            .unwrap();
        let plan = schedule.find("registry.plan_next()").unwrap();
        let reserve = schedule
            .find("registry.reserve_publication(batch)")
            .unwrap();
        let physical = schedule
            .find("schedule_m1_authenticated_speculative_rollover_with_recipe_input_v1")
            .unwrap();
        assert!(plan < reserve && reserve < physical);

        let publish = production
            .split("pub fn publish(")
            .nth(1)
            .unwrap()
            .split("impl<const C: usize> M1AuthenticatedPrefillRegistryPublishedFirstRoundV1")
            .next()
            .unwrap();
        let preflight = publish.find("preflight_publication").unwrap();
        let submit = publish
            .find("submit_m1_authenticated_speculative_rollover_v1")
            .unwrap();
        let record = publish.find("record_publication").unwrap();
        assert!(preflight < submit && submit < record);

        let complete = production
            .split("pub fn complete_round(")
            .nth(1)
            .unwrap()
            .split("impl<const C: usize> M1AuthenticatedPrefillRegistryCompletedFirstRoundV1")
            .next()
            .unwrap();
        let physical = complete.find("published.complete_round").unwrap();
        let registry_preflight = complete.find("preflight_completion_exact_for").unwrap();
        let registry_commit = complete.find("apply_preflighted_completion").unwrap();
        assert!(physical < registry_preflight && registry_preflight < registry_commit);
        assert!(!complete.contains("M1CheckedCompletionOutputV1"));
        assert!(!complete.contains("M1ObservedSpeculativeDiagnosticChoicesV1"));
    }

    #[test]
    fn first_round_input_surface_contains_no_page_or_queue_authority() {
        let source = include_str!("authenticated_prefill_registry_rollover.rs");
        let input = source
            .split("pub struct M1AuthenticatedPrefillRegistryFirstRoundInputsV1")
            .nth(1)
            .unwrap()
            .split("/// Stable stage")
            .next()
            .unwrap();
        assert!(input.contains("ValidatedM1StepInputs"));
        assert!(input.contains("M1FullStepWorkspacePlans"));
        assert!(!input.contains("DeviceKvPageLease"));
        assert!(!input.contains("Queue"));
        assert!(!input.contains("Engine"));
    }
}
