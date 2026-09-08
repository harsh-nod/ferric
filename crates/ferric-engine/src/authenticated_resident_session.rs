//! Bounded authenticated S1/K4 resident execution across R33 windows.
//!
//! This owner composes existing authenticated queue, registry, speculative,
//! and new-window typestates. It accepts only addressless workspace plans and
//! pretokenized inputs; it cannot acquire programs, devices, or publication
//! authority.

use core::{any::Any, fmt};
use std::collections::VecDeque;

use ferric_spec::{
    completion::CompletionEpoch, validate_m1_step_inputs, M1StepInputCandidate,
    M1StepInputValidationOutcome, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket,
    Qwen3PlanSelection, RequestId, TokenId, ValidatedM1StepInputs,
};

use crate::{
    prepare_m1_authenticated_s1_t128_prefill_prepublication_v1,
    prepare_m1_authenticated_speculative_new_window_v1,
    reconcile_m1_authenticated_s1_t128_prefill_registry_v1,
    schedule_m1_authenticated_speculative_new_window_v1,
    submit_m1_authenticated_speculative_new_window_v1,
    submit_m1_authenticated_speculative_rollover_v1, Engine, M1AuthenticatedPhysicalRunnerV1,
    M1AuthenticatedPrefillRegistryFirstRoundInputsV1, M1AuthenticatedS1T128PrefillBootstrapInputV1,
    M1AuthenticatedSpeculativeFailureDispositionV1,
    M1AuthenticatedSpeculativeNewWindowMemberDispositionV1,
    M1AuthenticatedSpeculativeNewWindowReleasedMemberStatusV1,
    M1AuthenticatedSpeculativeNewWindowSuccessorInputsV1,
    M1AuthenticatedSpeculativePhysicalExecutorV1, M1AuthenticatedSpeculativePhysicalRoundInputsV1,
    M1AuthenticatedSpeculativeRolloverMemberIntentV1, M1AuthenticatedTargetWindowClockStartV1,
    M1AuthenticatedTargetWindowTimingV1, M1FullStepWorkspaceInputKind, M1FullStepWorkspacePlans,
    M1ObservedSpeculativeDiagnosticChoicesV1, M1PartitionedModelMemoryKvPoolV1,
    M1QueueWaitTimeoutV1, M1ServingCompletionDispositionV1, M1ServingPlanV1,
    M1ServingQueueActionV1, M1ServingQueuedGenerationBindingV1,
    M1ServingQueuedPairedPrefillNewWindowV1, M1ServingRegistryV1, M1SpeculativeGenerationLoopV1,
    M1SpeculativeMemberControlV1, M1SpeculativeMemberSeedV1, M1SpeculativeMemberStatusV1,
    M1SpeculativeRoundOutcomeV1,
};

const TARGET_PREFILL: Qwen3PlanSelection = Qwen3PlanSelection {
    role: Qwen3ModelRole::Target8B,
    mode: Qwen3ExecutionMode::Prefill,
    bucket: Qwen3PlanBucket::PrefillS1T128,
};
const DRAFT_PREFILL: Qwen3PlanSelection = Qwen3PlanSelection {
    role: Qwen3ModelRole::Draft06B,
    mode: Qwen3ExecutionMode::Prefill,
    bucket: Qwen3PlanBucket::PrefillS1T128,
};

fn push_preallocated<T>(values: &mut Vec<T>, value: T) -> Result<(), T> {
    if values.len() == values.capacity() {
        return Err(value);
    }
    values.push(value);
    Ok(())
}

fn extend_preallocated_copy<T: Copy>(values: &mut Vec<T>, extension: &[T]) -> bool {
    if values.capacity().saturating_sub(values.len()) < extension.len() {
        return false;
    }
    values.extend_from_slice(extension);
    true
}

/// Exact maximum number of workload windows retained by one resident owner.
pub const M1_AUTHENTICATED_RESIDENT_WINDOWS_V1: usize =
    crate::M1_MAX_AUTHENTICATED_SPECULATIVE_WINDOWS_V1;

/// Internal checkpoints at every effect boundary governed by the immutable
/// R33 start deadline.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedResidentDeadlineBoundaryV1 {
    BeforeQueueCreate,
    BeforeNewWindowSchedule,
    BeforeFirstRolloverSchedule,
    BeforeSuccessorRolloverSchedule,
    BeforeQueueSubmit,
    AfterQueueSubmit,
    BeforeCompletionWait,
    AfterCompletionWait,
    BeforeReadback,
    AfterReadback,
    BeforeSettlement,
    AfterSettlement,
}

fn attempt_resident_schedule_if_live<R, T, D, F>(
    deadline: &mut D,
    boundary: M1AuthenticatedResidentDeadlineBoundaryV1,
    timeout: M1QueueWaitTimeoutV1,
    retained: &mut Option<R>,
    schedule: F,
) -> Option<T>
where
    D: FnMut(
        M1AuthenticatedResidentDeadlineBoundaryV1,
        M1QueueWaitTimeoutV1,
    ) -> Option<M1QueueWaitTimeoutV1>,
    F: FnOnce(R) -> T,
{
    deadline(boundary, timeout)?;
    Some(schedule(retained.take().expect(
        "live resident schedule retains its exact input owner",
    )))
}

/// Terminal native-queue state retained by a resident transition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedResidentQueueTeardownStatusV1 {
    /// No native queue existed at the rejected boundary.
    NoQueue,
    /// Native destruction completed and release evidence is retained.
    Released,
    /// Native destruction could not be confirmed; quarantine custody is retained.
    Quarantined,
}

/// Typed terminal custody after explicit native queue teardown or quarantine.
#[must_use = "resident teardown custody must remain retained"]
pub(crate) struct M1AuthenticatedResidentQueueTeardownV1 {
    status: M1AuthenticatedResidentQueueTeardownStatusV1,
    retained: Box<dyn Any>,
}

impl M1AuthenticatedResidentQueueTeardownV1 {
    pub(crate) fn no_queue(retained: impl Any) -> Self {
        Self {
            status: M1AuthenticatedResidentQueueTeardownStatusV1::NoQueue,
            retained: Box::new(retained),
        }
    }

    pub(crate) fn released(retained: impl Any) -> Self {
        Self {
            status: M1AuthenticatedResidentQueueTeardownStatusV1::Released,
            retained: Box::new(retained),
        }
    }

    pub(crate) fn quarantined(retained: impl Any) -> Self {
        Self {
            status: M1AuthenticatedResidentQueueTeardownStatusV1::Quarantined,
            retained: Box::new(retained),
        }
    }

    pub(crate) fn from_speculative_disposition(
        disposition: crate::M1AuthenticatedSpeculativeFailureDispositionV1,
    ) -> Self {
        if disposition.queue_released() {
            Self::released(disposition)
        } else {
            Self::quarantined(disposition)
        }
    }

    pub(crate) fn retain(self, extra: impl Any) -> Self {
        Self {
            status: self.status,
            retained: Box::new((self.retained, extra)),
        }
    }
}

/// Two independently consumed addressless workspace-plan sets for one round.
#[must_use = "resident round plans remain linear until physical preparation"]
#[derive(Debug)]
pub struct M1AuthenticatedResidentRoundPlansV1 {
    recipe: M1FullStepWorkspacePlans,
    preparation: M1FullStepWorkspacePlans,
}

impl M1AuthenticatedResidentRoundPlansV1 {
    /// Joins exact recipe and preparation copies without granting allocation.
    ///
    /// # Errors
    ///
    /// Returns both owners when they are not identical speculative-round plans.
    pub fn new(
        recipe: M1FullStepWorkspacePlans,
        preparation: M1FullStepWorkspacePlans,
    ) -> Result<Self, (M1FullStepWorkspacePlans, M1FullStepWorkspacePlans)> {
        if recipe.kind() != M1FullStepWorkspaceInputKind::SpeculativeRound || recipe != preparation
        {
            return Err((recipe, preparation));
        }
        Ok(Self {
            recipe,
            preparation,
        })
    }
}

/// Exact pretokenized input and worst-case speculative plan budget for one window.
#[must_use = "resident window input must be executed or retained"]
#[derive(Debug)]
pub struct M1AuthenticatedResidentWindowInputV1 {
    bootstrap: M1AuthenticatedS1T128PrefillBootstrapInputV1,
    rounds: VecDeque<M1AuthenticatedResidentRoundPlansV1>,
    diagnostic_ring_bytes: u32,
    queue_wait_timeout: M1QueueWaitTimeoutV1,
}

impl M1AuthenticatedResidentWindowInputV1 {
    /// Requires one plan pair per possible successor generation. A physical
    /// round always publishes at least one token, so this is a sufficient and
    /// exact worst-case bound.
    ///
    /// # Errors
    ///
    /// Returns bootstrap and round-plan custody when the plan count is not the
    /// exact successor bound or the diagnostic ring has zero bytes.
    pub fn new(
        bootstrap: M1AuthenticatedS1T128PrefillBootstrapInputV1,
        rounds: Vec<M1AuthenticatedResidentRoundPlansV1>,
        diagnostic_ring_bytes: u32,
        queue_wait_timeout: M1QueueWaitTimeoutV1,
    ) -> Result<
        Self,
        (
            M1AuthenticatedS1T128PrefillBootstrapInputV1,
            Vec<M1AuthenticatedResidentRoundPlansV1>,
        ),
    > {
        let expected = bootstrap.maximum_successor_output_tokens() as usize;
        if rounds.len() != expected || diagnostic_ring_bytes == 0 {
            return Err((bootstrap, rounds));
        }
        Ok(Self {
            bootstrap,
            rounds: VecDeque::from(rounds),
            diagnostic_ring_bytes,
            queue_wait_timeout,
        })
    }

    #[must_use]
    pub fn prompt_tokens(&self) -> &[TokenId] {
        self.bootstrap.prompt_tokens()
    }

    #[must_use]
    pub const fn expected_output_tokens(&self) -> u32 {
        self.bootstrap.maximum_successor_output_tokens() + 1
    }
}

/// Stable resident execution stage retained by an opaque failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedResidentStageV1 {
    Input,
    Clock,
    PrefillPrepare,
    PrefillExecute,
    RegistryReconcile,
    EarlyStop,
    RegistryPlan,
    RegistryReservation,
    LogicalInputs,
    FirstRoundSchedule,
    FirstRoundPrepare,
    FirstRoundPublish,
    FirstRoundComplete,
    SameShapeRound,
    RegistryCompletion,
    NewWindowReservation,
    NewWindowSchedule,
    NewWindowPrepare,
    NewWindowPublish,
    NewWindowObserve,
    NewWindowSettle,
    SuccessorSchedule,
    SuccessorPrepare,
    SuccessorPublish,
    SuccessorComplete,
    Cancellation,
    WindowLimit,
    Timing,
}

/// Opaque exhaustive custody for any resident transition failure.
#[must_use = "failed resident custody remains terminal and opaque"]
pub struct M1AuthenticatedResidentFailureV1 {
    stage: M1AuthenticatedResidentStageV1,
    engine_quarantined: bool,
    teardown: M1AuthenticatedResidentQueueTeardownV1,
}

impl fmt::Debug for M1AuthenticatedResidentFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedResidentFailureV1")
            .field("stage", &self.stage)
            .field("engine_quarantined", &self.engine_quarantined)
            .finish_non_exhaustive()
    }
}

impl M1AuthenticatedResidentFailureV1 {
    #[must_use]
    pub const fn stage(&self) -> M1AuthenticatedResidentStageV1 {
        self.stage
    }

    #[must_use]
    pub const fn engine_quarantined(&self) -> bool {
        self.engine_quarantined
    }

    #[must_use]
    pub fn retains_all_custody(&self) -> bool {
        let _ = &self.teardown.retained;
        true
    }

    /// Consumes failure custody into its explicit native teardown outcome.
    #[must_use = "resident failure teardown outcome must remain retained"]
    #[allow(clippy::boxed_local)]
    pub fn close(self: Box<Self>) -> M1AuthenticatedResidentCloseV1 {
        let Self {
            engine_quarantined,
            teardown,
            ..
        } = *self;
        M1AuthenticatedResidentCloseV1 {
            status: teardown.status,
            engine_quarantined,
            retained: teardown.retained,
        }
    }
}

#[allow(clippy::unnecessary_box_returns)]
fn resident_failure(
    stage: M1AuthenticatedResidentStageV1,
    engine_quarantined: bool,
    teardown: M1AuthenticatedResidentQueueTeardownV1,
) -> Box<M1AuthenticatedResidentFailureV1> {
    Box::new(M1AuthenticatedResidentFailureV1 {
        stage,
        engine_quarantined,
        teardown,
    })
}

fn close_resident_executor<const C: usize>(
    mut engine: Engine<C>,
    executor: M1AuthenticatedSpeculativePhysicalExecutorV1,
    retained: impl Any,
) -> M1AuthenticatedResidentQueueTeardownV1 {
    let teardown = executor.destroy_queue_and_retain_state(&mut engine);
    if teardown.is_ok() {
        M1AuthenticatedResidentQueueTeardownV1::released((engine, teardown, retained))
    } else {
        M1AuthenticatedResidentQueueTeardownV1::quarantined((engine, teardown, retained))
    }
}

fn close_resident_physical<const C: usize>(
    mut engine: Engine<C>,
    physical: crate::M1AuthenticatedSpeculativePhysicalRoundSuccessV1,
    retained: impl Any,
) -> M1AuthenticatedResidentQueueTeardownV1 {
    M1AuthenticatedResidentQueueTeardownV1::from_speculative_disposition(
        physical.close_for_resident(&mut engine),
    )
    .retain((engine, retained))
}

fn close_resident_result<T: Any, E: Any>(
    result: Result<T, E>,
    retained: impl Any,
) -> M1AuthenticatedResidentQueueTeardownV1 {
    if result.is_ok() {
        M1AuthenticatedResidentQueueTeardownV1::released((result, retained))
    } else {
        M1AuthenticatedResidentQueueTeardownV1::quarantined((result, retained))
    }
}

fn close_new_window_schedule_failure<const C: usize>(
    engine: &mut Engine<C>,
    failure: crate::M1AuthenticatedSpeculativeNewWindowScheduleFailureV1,
) -> M1AuthenticatedSpeculativeFailureDispositionV1 {
    match failure {
        crate::M1AuthenticatedSpeculativeNewWindowScheduleFailureV1::PreDetach { error, retry } => {
            retry.cancel_and_close(engine).retain(error)
        }
        crate::M1AuthenticatedSpeculativeNewWindowScheduleFailureV1::Terminal {
            error,
            disposition,
        } => disposition.retain(error),
    }
}

fn close_rollover_schedule_failure<const C: usize>(
    engine: &mut Engine<C>,
    failure: crate::M1AuthenticatedSpeculativeRolloverScheduleFailureV1,
) -> M1AuthenticatedSpeculativeFailureDispositionV1 {
    match failure {
        crate::M1AuthenticatedSpeculativeRolloverScheduleFailureV1::PreDetach { error, retry } => {
            retry.cancel_and_close(engine).retain(error)
        }
        crate::M1AuthenticatedSpeculativeRolloverScheduleFailureV1::Terminal {
            error,
            disposition,
        } => disposition.retain(error),
    }
}

#[derive(Debug)]
struct M1AuthenticatedResidentRoundEvidenceV1 {
    _outcome: M1SpeculativeRoundOutcomeV1,
    _choices: M1ObservedSpeculativeDiagnosticChoicesV1,
}

/// All-terminal authenticated owner ready for another exact new window.
#[must_use = "resident session must advance, close, or remain retained"]
pub struct M1AuthenticatedResidentSessionV1<const C: usize> {
    registry: M1ServingRegistryV1<C>,
    engine: Engine<C>,
    executor: M1AuthenticatedSpeculativePhysicalExecutorV1,
    request: RequestId,
    plan: M1ServingPlanV1,
    completed_windows: usize,
    evidence: Vec<M1AuthenticatedResidentRoundEvidenceV1>,
    unused_plans: Vec<M1AuthenticatedResidentRoundPlansV1>,
}

impl<const C: usize> fmt::Debug for M1AuthenticatedResidentSessionV1<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedResidentSessionV1")
            .field("request", &self.request)
            .field("plan", &self.plan)
            .field("completed_windows", &self.completed_windows)
            .field("engine_faulted", &self.engine.is_faulted())
            .finish_non_exhaustive()
    }
}

impl<const C: usize> M1AuthenticatedResidentSessionV1<C> {
    #[must_use]
    pub const fn completed_windows(&self) -> usize {
        self.completed_windows
    }

    #[must_use]
    pub fn is_ready_for_new_window(&self) -> bool {
        !self.engine.is_faulted()
            && self.executor.is_complete()
            && self.completed_windows < M1_AUTHENTICATED_RESIDENT_WINDOWS_V1
    }

    /// Explicitly tears down the retained physical queue and consumes every
    /// resident owner. The returned value exposes status only, never custody.
    #[must_use = "resident close evidence must remain observed"]
    pub fn close(mut self) -> M1AuthenticatedResidentCloseV1 {
        let teardown = self
            .executor
            .destroy_queue_and_retain_state(&mut self.engine);
        let status = if teardown.is_ok() {
            M1AuthenticatedResidentQueueTeardownStatusV1::Released
        } else {
            M1AuthenticatedResidentQueueTeardownStatusV1::Quarantined
        };
        M1AuthenticatedResidentCloseV1 {
            status,
            engine_quarantined: self.engine.is_faulted(),
            retained: Box::new((
                self.registry,
                self.engine,
                teardown,
                self.evidence,
                self.unused_plans,
            )),
        }
    }
}

/// Opaque terminal result of explicitly closing a resident session.
#[must_use = "resident close evidence retains all terminal custody"]
pub struct M1AuthenticatedResidentCloseV1 {
    status: M1AuthenticatedResidentQueueTeardownStatusV1,
    engine_quarantined: bool,
    retained: Box<dyn Any>,
}

impl fmt::Debug for M1AuthenticatedResidentCloseV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedResidentCloseV1")
            .field("queue_status", &self.status)
            .field("engine_quarantined", &self.engine_quarantined)
            .finish_non_exhaustive()
    }
}

impl M1AuthenticatedResidentCloseV1 {
    #[must_use]
    pub const fn queue_released(&self) -> bool {
        matches!(
            self.status,
            M1AuthenticatedResidentQueueTeardownStatusV1::Released
        )
    }

    /// Exact native queue state retained by this terminal close.
    #[must_use]
    pub const fn queue_status(&self) -> M1AuthenticatedResidentQueueTeardownStatusV1 {
        self.status
    }

    /// Whether stop can prove that no live native queue remains.
    #[must_use]
    pub const fn permits_stop_success(&self) -> bool {
        !matches!(
            self.status,
            M1AuthenticatedResidentQueueTeardownStatusV1::Quarantined
        )
    }

    #[must_use]
    pub const fn engine_quarantined(&self) -> bool {
        self.engine_quarantined
    }

    #[must_use]
    pub fn retains_all_custody(&self) -> bool {
        let _ = &self.retained;
        true
    }
}

/// Completed event-backed window report together with reusable resident custody.
#[must_use = "completed resident session custody remains linear"]
pub struct M1AuthenticatedResidentWindowSuccessV1<const C: usize> {
    session: M1AuthenticatedResidentSessionV1<C>,
    tokens: Box<[TokenId]>,
    timing: M1AuthenticatedTargetWindowTimingV1,
}

impl<const C: usize> fmt::Debug for M1AuthenticatedResidentWindowSuccessV1<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedResidentWindowSuccessV1")
            .field("completed_windows", &self.session.completed_windows)
            .field("output_tokens", &self.tokens.len())
            .field("timing", &self.timing)
            .finish_non_exhaustive()
    }
}

impl<const C: usize> M1AuthenticatedResidentWindowSuccessV1<C> {
    #[must_use]
    pub fn tokens(&self) -> &[TokenId] {
        &self.tokens
    }

    #[must_use]
    pub const fn timing(&self) -> M1AuthenticatedTargetWindowTimingV1 {
        self.timing
    }

    #[must_use = "resident session and report tokens remain owned"]
    pub fn into_parts(
        self,
    ) -> (
        M1AuthenticatedResidentSessionV1<C>,
        Box<[TokenId]>,
        M1AuthenticatedTargetWindowTimingV1,
    ) {
        (self.session, self.tokens, self.timing)
    }
}

fn one_lane_prefill_inputs(
    runner: &crate::LogicalRunnerDeclaration,
    selection: Qwen3PlanSelection,
    request: RequestId,
    epoch: CompletionEpoch,
    prompt: &[TokenId],
) -> Option<ValidatedM1StepInputs> {
    if prompt.len() != 128 {
        return None;
    }
    let plan = runner.bind_step_plan(request, epoch, selection).ok()?;
    let candidate = M1StepInputCandidate::new(
        selection,
        vec![Some(plan)],
        prompt.to_vec(),
        (0..128_u32).collect(),
        vec![128],
        vec![0],
    );
    match validate_m1_step_inputs(candidate) {
        M1StepInputValidationOutcome::Validated(inputs) => Some(inputs),
        M1StepInputValidationOutcome::Rejected(_) => None,
    }
}

fn one_lane_role_inputs(
    runner: &crate::LogicalRunnerDeclaration,
    selection: ferric_spec::Qwen3PlanSelection,
    request: RequestId,
    epoch: CompletionEpoch,
    anchor: TokenId,
    committed: u32,
) -> Option<ValidatedM1StepInputs> {
    let dimensions = selection
        .bucket
        .dimensions(selection.role, selection.mode)?;
    if dimensions.sequences != 1
        || !matches!(
            selection.mode,
            Qwen3ExecutionMode::Decode | Qwen3ExecutionMode::Speculative
        )
    {
        return None;
    }
    let width = dimensions.active_tokens as usize;
    let mut tokens = vec![0; width];
    let mut positions = Vec::new();
    positions.try_reserve_exact(width).ok()?;
    tokens[0] = anchor;
    for column in 0..dimensions.active_tokens {
        positions.push(committed.checked_add(column)?);
    }
    let plan = runner.bind_step_plan(request, epoch, selection).ok()?;
    match validate_m1_step_inputs(M1StepInputCandidate::new(
        selection,
        vec![Some(plan)],
        tokens,
        positions,
        vec![dimensions.active_tokens],
        vec![committed],
    )) {
        M1StepInputValidationOutcome::Validated(inputs) => Some(inputs),
        M1StepInputValidationOutcome::Rejected(_) => None,
    }
}

#[derive(Debug)]
struct M1AuthenticatedResidentRoleInputStorageV1 {
    lanes: Vec<Option<ferric_spec::StepPlan>>,
    tokens: Vec<TokenId>,
    positions: Vec<u32>,
    active_lengths: Vec<u32>,
    context_lengths: Vec<u32>,
}

impl M1AuthenticatedResidentRoleInputStorageV1 {
    fn try_new(width: usize) -> Option<Self> {
        let mut lanes = Vec::new();
        let mut tokens = Vec::new();
        let mut positions = Vec::new();
        let mut active_lengths = Vec::new();
        let mut context_lengths = Vec::new();
        lanes.try_reserve_exact(1).ok()?;
        tokens.try_reserve_exact(width).ok()?;
        positions.try_reserve_exact(width).ok()?;
        active_lengths.try_reserve_exact(1).ok()?;
        context_lengths.try_reserve_exact(1).ok()?;
        tokens.resize(width, 0);
        Some(Self {
            lanes,
            tokens,
            positions,
            active_lengths,
            context_lengths,
        })
    }

    fn fill(
        mut self,
        runner: &crate::LogicalRunnerDeclaration,
        selection: ferric_spec::Qwen3PlanSelection,
        request: RequestId,
        epoch: CompletionEpoch,
        anchor: TokenId,
        committed: u32,
    ) -> Option<ValidatedM1StepInputs> {
        let dimensions = selection
            .bucket
            .dimensions(selection.role, selection.mode)?;
        let width = dimensions.active_tokens as usize;
        if dimensions.sequences != 1
            || self.tokens.len() != width
            || !matches!(
                selection.mode,
                Qwen3ExecutionMode::Decode | Qwen3ExecutionMode::Speculative
            )
        {
            return None;
        }
        self.tokens.fill(0);
        self.tokens[0] = anchor;
        self.positions.clear();
        for column in 0..dimensions.active_tokens {
            push_preallocated(&mut self.positions, committed.checked_add(column)?).ok()?;
        }
        self.lanes.clear();
        push_preallocated(
            &mut self.lanes,
            Some(runner.bind_step_plan(request, epoch, selection).ok()?),
        )
        .ok()?;
        self.active_lengths.clear();
        push_preallocated(&mut self.active_lengths, dimensions.active_tokens).ok()?;
        self.context_lengths.clear();
        push_preallocated(&mut self.context_lengths, committed).ok()?;
        match validate_m1_step_inputs(M1StepInputCandidate::new(
            selection,
            self.lanes,
            self.tokens,
            self.positions,
            self.active_lengths,
            self.context_lengths,
        )) {
            M1StepInputValidationOutcome::Validated(inputs) => Some(inputs),
            M1StepInputValidationOutcome::Rejected(_) => None,
        }
    }
}

#[derive(Debug)]
struct M1AuthenticatedResidentRoundInputStorageV1 {
    draft: M1AuthenticatedResidentRoleInputStorageV1,
    target: M1AuthenticatedResidentRoleInputStorageV1,
    controls: Vec<M1SpeculativeMemberControlV1>,
}

impl M1AuthenticatedResidentRoundInputStorageV1 {
    fn try_new(plan: M1ServingPlanV1) -> Option<Self> {
        let draft = plan
            .draft()
            .bucket
            .dimensions(plan.draft().role, plan.draft().mode)?;
        let target = plan
            .target()
            .bucket
            .dimensions(plan.target().role, plan.target().mode)?;
        let mut controls = Vec::new();
        controls.try_reserve_exact(1).ok()?;
        Some(Self {
            draft: M1AuthenticatedResidentRoleInputStorageV1::try_new(
                draft.active_tokens as usize,
            )?,
            target: M1AuthenticatedResidentRoleInputStorageV1::try_new(
                target.active_tokens as usize,
            )?,
            controls,
        })
    }
}

fn preallocate_resident_round_inputs(
    count: usize,
    plan: M1ServingPlanV1,
) -> Option<VecDeque<M1AuthenticatedResidentRoundInputStorageV1>> {
    let mut storage = VecDeque::new();
    storage.try_reserve_exact(count).ok()?;
    for _ in 0..count {
        storage.push_back(M1AuthenticatedResidentRoundInputStorageV1::try_new(plan)?);
    }
    Some(storage)
}

fn next_round_inputs(
    executor: &M1AuthenticatedSpeculativePhysicalExecutorV1,
    request: RequestId,
    plan: M1ServingPlanV1,
    epoch: CompletionEpoch,
    plans: M1AuthenticatedResidentRoundPlansV1,
    mut storage: M1AuthenticatedResidentRoundInputStorageV1,
) -> Option<M1AuthenticatedSpeculativePhysicalRoundInputsV1> {
    if plan.target().bucket != Qwen3PlanBucket::SpeculativeS1K4C8192
        || executor.active_roster().as_slice() != [request]
    {
        return None;
    }
    let snapshot = executor.member_snapshot(request)?;
    if snapshot.status() != M1SpeculativeMemberStatusV1::Active {
        return None;
    }
    let runner = executor.retained_logical_runner();
    let draft = storage.draft.fill(
        runner,
        plan.draft(),
        request,
        epoch,
        snapshot.next_anchor(),
        snapshot.draft_committed_tokens(),
    )?;
    let target = storage.target.fill(
        runner,
        plan.target(),
        request,
        epoch,
        snapshot.next_anchor(),
        snapshot.target_committed_tokens(),
    )?;
    storage.controls.clear();
    push_preallocated(
        &mut storage.controls,
        M1SpeculativeMemberControlV1::continuing(request),
    )
    .ok()?;
    Some(
        M1AuthenticatedSpeculativePhysicalRoundInputsV1::with_authenticated_tail_pages(
            draft,
            target,
            plans.recipe,
            plans.preparation,
            storage.controls,
        ),
    )
}

/// Executes the first S1/T128 window through repeated authenticated S1/K4
/// rounds until checked policy reaches an all-terminal state.
///
/// # Panics
///
/// Panics only if the private physical executor violates its `FnOnce`
/// post-submit contract by consuming no publication reservation.
///
/// # Errors
///
/// Returns exhaustive opaque custody for any invalid input, deadline, logical,
/// registry, physical, settlement, teardown, or timing transition.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_lines)]
pub fn execute_m1_authenticated_resident_first_window_v1<const C: usize>(
    clock_start: M1AuthenticatedTargetWindowClockStartV1,
    mut engine: Engine<C>,
    runner: M1AuthenticatedPhysicalRunnerV1,
    model_memory: M1PartitionedModelMemoryKvPoolV1,
    input: M1AuthenticatedResidentWindowInputV1,
    mut deadline: impl FnMut(
        M1AuthenticatedResidentDeadlineBoundaryV1,
        M1QueueWaitTimeoutV1,
    ) -> Option<M1QueueWaitTimeoutV1>,
) -> Result<M1AuthenticatedResidentWindowSuccessV1<C>, Box<M1AuthenticatedResidentFailureV1>> {
    let M1AuthenticatedResidentWindowInputV1 {
        bootstrap,
        mut rounds,
        diagnostic_ring_bytes,
        queue_wait_timeout,
    } = input;
    if deadline(
        M1AuthenticatedResidentDeadlineBoundaryV1::BeforeQueueCreate,
        queue_wait_timeout,
    )
    .is_none()
    {
        engine.quarantine_m1_queue_rearm_failure();
        return Err(resident_failure(
            M1AuthenticatedResidentStageV1::Cancellation,
            true,
            M1AuthenticatedResidentQueueTeardownV1::no_queue((
                engine,
                runner,
                model_memory,
                bootstrap,
                rounds,
                diagnostic_ring_bytes,
                queue_wait_timeout,
            )),
        ));
    }
    let prepared = match prepare_m1_authenticated_s1_t128_prefill_prepublication_v1(
        engine,
        runner,
        model_memory,
        bootstrap,
    ) {
        Ok(prepared) => prepared,
        Err(failure) => {
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::PrefillPrepare,
                failure.engine_quarantined(),
                M1AuthenticatedResidentQueueTeardownV1::no_queue(failure),
            ));
        }
    };
    let executed = match crate::authenticated_prefill_executor::execute_m1_authenticated_s1_t128_paired_prefill_with_deadline_v1(
        prepared,
        diagnostic_ring_bytes,
        queue_wait_timeout,
        &mut deadline,
    ) {
        Ok(executed) => executed,
        Err(failure) => {
            let stage = if failure.error()
                == crate::M1AuthenticatedS1T128PrefillExecutionErrorV1::DeadlineExpired
            {
                M1AuthenticatedResidentStageV1::Cancellation
            } else {
                M1AuthenticatedResidentStageV1::PrefillExecute
            };
            return Err(resident_failure(
                stage,
                failure.engine_quarantined(),
                failure.into_resident_teardown(),
            ));
        }
    };
    let first_token_offset_ns = match clock_start.elapsed_ns() {
        Ok(offset) => offset,
        Err(error) => {
            let teardown = executed.close_for_resident().retain((rounds, error));
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::Clock,
                false,
                teardown,
            ));
        }
    };
    let first_token = executed.first_token();
    let registry = match M1ServingRegistryV1::<C>::new() {
        Ok(registry) => registry,
        Err(error) => {
            let teardown = executed.close_for_resident().retain((rounds, error));
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::RegistryReconcile,
                false,
                teardown,
            ));
        }
    };
    let reconciled =
        match reconcile_m1_authenticated_s1_t128_prefill_registry_v1(registry, executed) {
            Ok(reconciled) => reconciled,
            Err(failure) => {
                return Err(resident_failure(
                    M1AuthenticatedResidentStageV1::RegistryReconcile,
                    false,
                    failure.into_resident_teardown().retain(rounds),
                ));
            }
        };
    let request = reconciled.request();
    let plan = reconciled.successor_plan();
    let epoch = reconciled.successor_epoch();
    if !reconciled.permits_speculative_successor() {
        let teardown = reconciled.close_for_resident().retain(rounds);
        return Err(resident_failure(
            M1AuthenticatedResidentStageV1::EarlyStop,
            false,
            teardown,
        ));
    }
    let Some(first_plans) = rounds.pop_front() else {
        let teardown = reconciled.close_for_resident();
        return Err(resident_failure(
            M1AuthenticatedResidentStageV1::Input,
            false,
            teardown,
        ));
    };
    let draft = match one_lane_role_inputs(
        reconciled.retained_logical_runner(),
        plan.draft(),
        request,
        epoch,
        first_token,
        128,
    ) {
        Some(draft) => draft,
        None => {
            let teardown = reconciled
                .close_for_resident()
                .retain((first_plans, rounds));
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::LogicalInputs,
                false,
                teardown,
            ));
        }
    };
    let target = match one_lane_role_inputs(
        reconciled.retained_logical_runner(),
        plan.target(),
        request,
        epoch,
        first_token,
        128,
    ) {
        Some(target) => target,
        None => {
            let teardown = reconciled
                .close_for_resident()
                .retain((draft, first_plans, rounds));
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::LogicalInputs,
                false,
                teardown,
            ));
        }
    };
    let mut pending_schedule = Some((
        reconciled,
        M1AuthenticatedPrefillRegistryFirstRoundInputsV1::new(
            draft,
            target,
            first_plans.recipe,
            first_plans.preparation,
        ),
    ));
    let Some(scheduled) = attempt_resident_schedule_if_live(
        &mut deadline,
        M1AuthenticatedResidentDeadlineBoundaryV1::BeforeFirstRolloverSchedule,
        queue_wait_timeout,
        &mut pending_schedule,
        |(reconciled, inputs)| reconciled.schedule_first_speculative_round(inputs),
    ) else {
        let (reconciled, inputs) = pending_schedule
            .take()
            .expect("expired first rollover schedule retains exact inputs");
        let teardown = reconciled.close_for_resident().retain((inputs, rounds));
        return Err(resident_failure(
            M1AuthenticatedResidentStageV1::Cancellation,
            true,
            teardown,
        ));
    };
    let scheduled = match scheduled {
        Ok(scheduled) => scheduled,
        Err(failure) => {
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::FirstRoundSchedule,
                failure.engine_quarantined(),
                failure.into_resident_teardown().retain(rounds),
            ));
        }
    };
    let prepared = match scheduled.prepare_retained() {
        Ok(prepared) => prepared,
        Err(failure) => {
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::FirstRoundPrepare,
                failure.engine_quarantined(),
                failure.into_resident_teardown().retain(rounds),
            ));
        }
    };
    let published = match prepared.publish_with_deadline(&mut deadline) {
        Ok(published) => published,
        Err(failure) => {
            let stage = if failure.stage()
                == crate::M1AuthenticatedPrefillRegistryFirstRoundStageV1::Deadline
            {
                M1AuthenticatedResidentStageV1::Cancellation
            } else {
                M1AuthenticatedResidentStageV1::FirstRoundPublish
            };
            return Err(resident_failure(
                stage,
                failure.engine_quarantined(),
                failure.into_resident_teardown().retain(rounds),
            ));
        }
    };
    let completed = match published.complete_round_with_deadline(
        vec![M1SpeculativeMemberControlV1::continuing(request)],
        &mut deadline,
    ) {
        Ok(completed) => completed,
        Err(failure) => {
            let stage = if failure.stage()
                == crate::M1AuthenticatedPrefillRegistryFirstRoundStageV1::Deadline
            {
                M1AuthenticatedResidentStageV1::Cancellation
            } else {
                M1AuthenticatedResidentStageV1::FirstRoundComplete
            };
            return Err(resident_failure(
                stage,
                failure.engine_quarantined(),
                failure.into_resident_teardown().retain(rounds),
            ));
        }
    };
    let (mut registry, mut engine, physical, request, retained_first_token, plan) =
        completed.into_resident_parts();
    if retained_first_token != first_token {
        engine.quarantine_m1_queue_rearm_failure();
        let teardown = close_resident_physical(engine, physical, (registry, rounds));
        return Err(resident_failure(
            M1AuthenticatedResidentStageV1::FirstRoundComplete,
            true,
            teardown,
        ));
    }
    let (mut executor, outcome, choices) = physical.into_parts();
    let Some(mut round_input_storage) = preallocate_resident_round_inputs(rounds.len(), plan)
    else {
        engine.quarantine_m1_queue_rearm_failure();
        let teardown =
            close_resident_executor(engine, executor, (registry, outcome, choices, rounds));
        return Err(resident_failure(
            M1AuthenticatedResidentStageV1::Input,
            true,
            teardown,
        ));
    };
    let mut tokens = Vec::new();
    if tokens.try_reserve_exact(128).is_err() {
        engine.quarantine_m1_queue_rearm_failure();
        let teardown = close_resident_executor(
            engine,
            executor,
            (registry, outcome, choices, rounds, round_input_storage),
        );
        return Err(resident_failure(
            M1AuthenticatedResidentStageV1::Input,
            true,
            teardown,
        ));
    }
    tokens.push(first_token);
    let Some(member) = outcome
        .members()
        .first()
        .filter(|_| outcome.members().len() == 1)
    else {
        engine.quarantine_m1_queue_rearm_failure();
        let teardown = close_resident_executor(
            engine,
            executor,
            (registry, outcome, choices, rounds, tokens),
        );
        return Err(resident_failure(
            M1AuthenticatedResidentStageV1::FirstRoundComplete,
            true,
            teardown,
        ));
    };
    tokens.extend_from_slice(member.published().tokens());
    let mut evidence = Vec::new();
    if evidence.try_reserve_exact(1 + rounds.len()).is_err() {
        engine.quarantine_m1_queue_rearm_failure();
        let teardown = close_resident_executor(
            engine,
            executor,
            (
                registry,
                outcome,
                choices,
                rounds,
                round_input_storage,
                tokens,
            ),
        );
        return Err(resident_failure(
            M1AuthenticatedResidentStageV1::Input,
            true,
            teardown,
        ));
    }
    evidence.push(M1AuthenticatedResidentRoundEvidenceV1 {
        _outcome: outcome,
        _choices: choices,
    });

    while !executor.is_complete() {
        if deadline(
            M1AuthenticatedResidentDeadlineBoundaryV1::BeforeQueueSubmit,
            queue_wait_timeout,
        )
        .is_none()
        {
            let cancellation = registry.cancel(request);
            engine.quarantine_m1_queue_rearm_failure();
            let teardown = executor.destroy_queue_and_retain_state(&mut engine);
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::Cancellation,
                true,
                close_resident_result(
                    teardown,
                    (registry, engine, rounds, evidence, tokens, cancellation),
                ),
            ));
        }
        let Some(plans) = rounds.pop_front() else {
            engine.quarantine_m1_queue_rearm_failure();
            let teardown = executor.destroy_queue_and_retain_state(&mut engine);
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::Input,
                true,
                close_resident_result(teardown, (registry, engine, evidence, tokens)),
            ));
        };
        let Some(storage) = round_input_storage.pop_front() else {
            engine.quarantine_m1_queue_rearm_failure();
            let teardown = close_resident_executor(
                engine,
                executor,
                (registry, plans, rounds, evidence, tokens),
            );
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::Input,
                true,
                teardown,
            ));
        };
        let batch = match registry.plan_next() {
            Ok(Some(batch)) => batch,
            result => {
                engine.quarantine_m1_queue_rearm_failure();
                let teardown = close_resident_executor(
                    engine,
                    executor,
                    (registry, plans, rounds, evidence, tokens, result),
                );
                return Err(resident_failure(
                    M1AuthenticatedResidentStageV1::RegistryPlan,
                    true,
                    teardown,
                ));
            }
        };
        if batch.plan() != plan
            || batch.requests() != [request]
            || batch.action() != M1ServingQueueActionV1::SameShapeRearm
        {
            engine.quarantine_m1_queue_rearm_failure();
            let teardown = close_resident_executor(
                engine,
                executor,
                (registry, batch, plans, rounds, evidence, tokens),
            );
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::RegistryPlan,
                true,
                teardown,
            ));
        }
        let epoch = batch.epoch();
        let inputs = match next_round_inputs(&executor, request, plan, epoch, plans, storage) {
            Some(inputs) => inputs,
            None => {
                engine.quarantine_m1_queue_rearm_failure();
                let teardown =
                    close_resident_executor(engine, executor, (registry, rounds, evidence, tokens));
                return Err(resident_failure(
                    M1AuthenticatedResidentStageV1::LogicalInputs,
                    true,
                    teardown,
                ));
            }
        };
        let reservation = match registry.reserve_publication(batch) {
            Ok(reservation) => reservation,
            Err(error) => {
                engine.quarantine_m1_queue_rearm_failure();
                let teardown = close_resident_executor(
                    engine,
                    executor,
                    (registry, inputs, rounds, evidence, tokens, error),
                );
                return Err(resident_failure(
                    M1AuthenticatedResidentStageV1::RegistryReservation,
                    true,
                    teardown,
                ));
            }
        };
        let registry_identity = reservation.registry_identity();
        let mut reservation = Some(reservation);
        let physical = match executor.execute_round_with_post_submit_and_deadline(
            &mut engine,
            inputs,
            || {
                registry.record_publication(
                    reservation
                        .take()
                        .expect("post-submit registry reservation remains present"),
                )
            },
            &mut deadline,
        ) {
            Ok(physical) => physical,
            Err(failure) => {
                let abort = reservation.map(|reservation| registry.abort_publication(reservation));
                let stage = if failure.stage()
                    == crate::M1AuthenticatedSpeculativePhysicalRoundStageV1::Deadline
                {
                    M1AuthenticatedResidentStageV1::Cancellation
                } else {
                    M1AuthenticatedResidentStageV1::SameShapeRound
                };
                let disposition = failure.close_for_resident(&mut engine);
                return Err(resident_failure(
                    stage,
                    engine.is_faulted(),
                    M1AuthenticatedResidentQueueTeardownV1::from_speculative_disposition(
                        disposition,
                    )
                    .retain((registry, engine, abort, rounds, evidence, tokens)),
                ));
            }
        };
        let (next_executor, outcome, choices) = physical.into_parts();
        let [member] = outcome.members() else {
            engine.quarantine_m1_queue_rearm_failure();
            let teardown = close_resident_executor(
                engine,
                next_executor,
                (registry, outcome, choices, rounds, evidence, tokens),
            );
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::RegistryCompletion,
                true,
                teardown,
            ));
        };
        let disposition = if member.status() == M1SpeculativeMemberStatusV1::Active {
            M1ServingCompletionDispositionV1::Continue(plan)
        } else {
            M1ServingCompletionDispositionV1::Retire
        };
        if registry
            .preflight_completion_exact_for(registry_identity, epoch, &[disposition])
            .is_err()
        {
            engine.quarantine_m1_queue_rearm_failure();
            let teardown = close_resident_executor(
                engine,
                next_executor,
                (registry, outcome, choices, rounds, evidence, tokens),
            );
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::RegistryCompletion,
                true,
                teardown,
            ));
        }
        if !extend_preallocated_copy(&mut tokens, member.published().tokens()) {
            engine.quarantine_m1_queue_rearm_failure();
            let teardown = close_resident_executor(
                engine,
                next_executor,
                (registry, outcome, choices, rounds, evidence, tokens),
            );
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::Input,
                true,
                teardown,
            ));
        }
        let next_evidence = M1AuthenticatedResidentRoundEvidenceV1 {
            _outcome: outcome,
            _choices: choices,
        };
        if let Err(next_evidence) = push_preallocated(&mut evidence, next_evidence) {
            engine.quarantine_m1_queue_rearm_failure();
            let teardown = close_resident_executor(
                engine,
                next_executor,
                (registry, next_evidence, rounds, evidence, tokens),
            );
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::Input,
                true,
                teardown,
            ));
        }
        registry.apply_preflighted_completion(epoch, &[disposition]);
        executor = next_executor;
    }
    if deadline(
        M1AuthenticatedResidentDeadlineBoundaryV1::AfterSettlement,
        queue_wait_timeout,
    )
    .is_none()
    {
        let cancellation = registry.cancel(request);
        engine.quarantine_m1_queue_rearm_failure();
        let teardown = executor.destroy_queue_and_retain_state(&mut engine);
        return Err(resident_failure(
            M1AuthenticatedResidentStageV1::Cancellation,
            true,
            close_resident_result(
                teardown,
                (registry, engine, rounds, evidence, tokens, cancellation),
            ),
        ));
    }

    let terminal_token_offset_ns = match clock_start.elapsed_ns() {
        Ok(offset) => offset,
        Err(error) => {
            engine.quarantine_m1_queue_rearm_failure();
            let teardown = close_resident_executor(
                engine,
                executor,
                (registry, rounds, evidence, tokens, error),
            );
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::Clock,
                true,
                teardown,
            ));
        }
    };
    let duration_ns = match clock_start.elapsed_ns() {
        Ok(duration) => duration,
        Err(error) => {
            engine.quarantine_m1_queue_rearm_failure();
            let teardown = close_resident_executor(
                engine,
                executor,
                (registry, rounds, evidence, tokens, error),
            );
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::Clock,
                true,
                teardown,
            ));
        }
    };
    let timing = match M1AuthenticatedTargetWindowTimingV1::from_resident_boundaries(
        duration_ns,
        first_token_offset_ns,
        terminal_token_offset_ns,
        tokens.len(),
    ) {
        Some(timing) => timing,
        None => {
            engine.quarantine_m1_queue_rearm_failure();
            let teardown =
                close_resident_executor(engine, executor, (registry, rounds, evidence, tokens));
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::Timing,
                true,
                teardown,
            ));
        }
    };
    Ok(M1AuthenticatedResidentWindowSuccessV1 {
        session: M1AuthenticatedResidentSessionV1 {
            registry,
            engine,
            executor,
            request,
            plan,
            completed_windows: 1,
            evidence,
            unused_plans: rounds.into_iter().collect(),
        },
        tokens: tokens.into_boxed_slice(),
        timing,
    })
}

/// Replaces one all-terminal S1/K4 generation with the next authenticated
/// S1/T128 window, then executes its finite S1/K4 successor to completion.
///
/// # Panics
///
/// Panics only if the private physical executor violates its `FnOnce`
/// post-submit contract by consuming no publication reservation.
///
/// # Errors
///
/// Returns exhaustive opaque custody for any invalid input, deadline, logical,
/// registry, physical, settlement, teardown, window-limit, or timing transition.
#[allow(clippy::too_many_lines)]
pub fn execute_m1_authenticated_resident_next_window_v1<const C: usize>(
    clock_start: M1AuthenticatedTargetWindowClockStartV1,
    session: M1AuthenticatedResidentSessionV1<C>,
    input: M1AuthenticatedResidentWindowInputV1,
    mut deadline: impl FnMut(
        M1AuthenticatedResidentDeadlineBoundaryV1,
        M1QueueWaitTimeoutV1,
    ) -> Option<M1QueueWaitTimeoutV1>,
) -> Result<M1AuthenticatedResidentWindowSuccessV1<C>, Box<M1AuthenticatedResidentFailureV1>> {
    let M1AuthenticatedResidentSessionV1 {
        mut registry,
        mut engine,
        executor,
        request,
        plan,
        completed_windows,
        mut evidence,
        mut unused_plans,
    } = session;
    if completed_windows >= M1_AUTHENTICATED_RESIDENT_WINDOWS_V1
        || engine.is_faulted()
        || !executor.is_complete()
    {
        let engine_quarantined = engine.is_faulted();
        let teardown =
            close_resident_executor(engine, executor, (registry, input, evidence, unused_plans));
        return Err(resident_failure(
            M1AuthenticatedResidentStageV1::WindowLimit,
            engine_quarantined,
            teardown,
        ));
    }
    let M1AuthenticatedResidentWindowInputV1 {
        bootstrap,
        mut rounds,
        diagnostic_ring_bytes,
        queue_wait_timeout,
    } = input;
    if deadline(
        M1AuthenticatedResidentDeadlineBoundaryV1::BeforeNewWindowSchedule,
        queue_wait_timeout,
    )
    .is_none()
    {
        engine.quarantine_m1_queue_rearm_failure();
        let teardown = executor.destroy_queue_and_retain_state(&mut engine);
        return Err(resident_failure(
            M1AuthenticatedResidentStageV1::Cancellation,
            true,
            close_resident_result(
                teardown,
                (
                    registry,
                    engine,
                    bootstrap,
                    rounds,
                    diagnostic_ring_bytes,
                    queue_wait_timeout,
                    evidence,
                    unused_plans,
                ),
            ),
        ));
    }
    let (prompt, policy, prefill_preparation, prefill_recipe) = bootstrap.into_resident_parts();
    let Some(next_generation) = request.generation().checked_add(1) else {
        engine.quarantine_m1_queue_rearm_failure();
        let teardown = executor.destroy_queue_and_retain_state(&mut engine);
        return Err(resident_failure(
            M1AuthenticatedResidentStageV1::NewWindowReservation,
            true,
            close_resident_result(
                teardown,
                (
                    registry,
                    engine,
                    prompt,
                    policy,
                    prefill_preparation,
                    prefill_recipe,
                    rounds,
                    evidence,
                    unused_plans,
                ),
            ),
        ));
    };
    let next_request = RequestId::new(request.slot(), next_generation);
    let prefill_plan = match M1ServingPlanV1::new(TARGET_PREFILL, DRAFT_PREFILL) {
        Ok(plan) => plan,
        Err(error) => {
            engine.quarantine_m1_queue_rearm_failure();
            let teardown = executor.destroy_queue_and_retain_state(&mut engine);
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::NewWindowReservation,
                true,
                close_resident_result(
                    teardown,
                    (
                        registry,
                        engine,
                        prompt,
                        policy,
                        prefill_preparation,
                        prefill_recipe,
                        rounds,
                        evidence,
                        unused_plans,
                        error,
                    ),
                ),
            ));
        }
    };
    let reservation = match registry
        .reserve_completed_window_replacement(prefill_plan, vec![next_request].into_boxed_slice())
    {
        Ok(reservation) => reservation,
        Err(failure) => {
            engine.quarantine_m1_queue_rearm_failure();
            let teardown = executor.destroy_queue_and_retain_state(&mut engine);
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::NewWindowReservation,
                true,
                close_resident_result(
                    teardown,
                    (
                        registry,
                        engine,
                        prompt,
                        policy,
                        prefill_preparation,
                        prefill_recipe,
                        rounds,
                        evidence,
                        unused_plans,
                        failure,
                    ),
                ),
            ));
        }
    };
    let batch = reservation.physical_batch();
    let draft_prefill = match one_lane_prefill_inputs(
        executor.retained_logical_runner(),
        DRAFT_PREFILL,
        next_request,
        batch.epoch(),
        &prompt,
    ) {
        Some(inputs) => inputs,
        None => {
            engine.quarantine_m1_queue_rearm_failure();
            let teardown = executor.destroy_queue_and_retain_state(&mut engine);
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::LogicalInputs,
                true,
                close_resident_result(
                    teardown,
                    (
                        registry,
                        reservation,
                        engine,
                        prompt,
                        policy,
                        prefill_preparation,
                        prefill_recipe,
                        rounds,
                        evidence,
                        unused_plans,
                    ),
                ),
            ));
        }
    };
    let target_prefill = match one_lane_prefill_inputs(
        executor.retained_logical_runner(),
        TARGET_PREFILL,
        next_request,
        batch.epoch(),
        &prompt,
    ) {
        Some(inputs) => inputs,
        None => {
            engine.quarantine_m1_queue_rearm_failure();
            let teardown = executor.destroy_queue_and_retain_state(&mut engine);
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::LogicalInputs,
                true,
                close_resident_result(
                    teardown,
                    (
                        registry,
                        reservation,
                        engine,
                        draft_prefill,
                        prompt,
                        policy,
                        prefill_preparation,
                        prefill_recipe,
                        rounds,
                        evidence,
                        unused_plans,
                    ),
                ),
            ));
        }
    };
    let queued = M1ServingQueuedPairedPrefillNewWindowV1::new(
        M1ServingQueuedGenerationBindingV1::new(
            prefill_plan,
            vec![next_request].into_boxed_slice(),
            batch.epoch(),
        ),
        draft_prefill,
        target_prefill,
        prefill_preparation,
        prefill_recipe,
    );
    let registry_identity = reservation.registry_identity();
    if deadline(
        M1AuthenticatedResidentDeadlineBoundaryV1::BeforeNewWindowSchedule,
        queue_wait_timeout,
    )
    .is_none()
    {
        engine.quarantine_m1_queue_rearm_failure();
        let restored = registry.restore_completed_window_replacement(reservation);
        let teardown = close_resident_executor(
            engine,
            executor,
            (registry, restored, rounds, evidence, unused_plans),
        );
        return Err(resident_failure(
            M1AuthenticatedResidentStageV1::Cancellation,
            true,
            teardown,
        ));
    }
    let scheduled = match schedule_m1_authenticated_speculative_new_window_v1(
        &mut engine,
        executor,
        &batch,
        queued,
        plan,
        vec![M1AuthenticatedSpeculativeRolloverMemberIntentV1::new(
            next_request,
            policy,
        )],
        diagnostic_ring_bytes,
        queue_wait_timeout,
    ) {
        Ok(scheduled) => scheduled,
        Err(failure) => {
            let error = failure.error();
            let restored = failure
                .is_pre_detach_retry()
                .then(|| registry.restore_completed_window_replacement(reservation));
            let disposition = close_new_window_schedule_failure(&mut engine, failure);
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::NewWindowSchedule,
                engine.is_faulted(),
                M1AuthenticatedResidentQueueTeardownV1::from_speculative_disposition(disposition)
                    .retain((
                        registry,
                        engine,
                        error,
                        restored,
                        rounds,
                        evidence,
                        unused_plans,
                    )),
            ));
        }
    };
    let prepared = match prepare_m1_authenticated_speculative_new_window_v1(&mut engine, scheduled)
    {
        Ok(prepared) => prepared,
        Err(failure) => {
            let disposition = close_new_window_schedule_failure(&mut engine, failure);
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::NewWindowPrepare,
                engine.is_faulted(),
                M1AuthenticatedResidentQueueTeardownV1::from_speculative_disposition(disposition)
                    .retain((
                        registry,
                        reservation,
                        engine,
                        rounds,
                        evidence,
                        unused_plans,
                    )),
            ));
        }
    };
    if deadline(
        M1AuthenticatedResidentDeadlineBoundaryV1::BeforeQueueSubmit,
        queue_wait_timeout,
    )
    .is_none()
    {
        engine.quarantine_m1_queue_rearm_failure();
        let disposition = prepared.cancel_and_close(&mut engine);
        return Err(resident_failure(
            M1AuthenticatedResidentStageV1::Cancellation,
            true,
            M1AuthenticatedResidentQueueTeardownV1::from_speculative_disposition(disposition)
                .retain((
                    registry,
                    reservation,
                    engine,
                    rounds,
                    evidence,
                    unused_plans,
                )),
        ));
    }
    let published = match submit_m1_authenticated_speculative_new_window_v1(&mut engine, prepared) {
        Ok(published) => published,
        Err(failure) => {
            let disposition = failure.into_disposition();
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::NewWindowPublish,
                engine.is_faulted(),
                M1AuthenticatedResidentQueueTeardownV1::from_speculative_disposition(disposition)
                    .retain((
                        registry,
                        reservation,
                        engine,
                        rounds,
                        evidence,
                        unused_plans,
                    )),
            ));
        }
    };
    if deadline(
        M1AuthenticatedResidentDeadlineBoundaryV1::AfterQueueSubmit,
        queue_wait_timeout,
    )
    .is_none()
    {
        engine.quarantine_m1_queue_rearm_failure();
        let disposition = published.cancel_and_close(&mut engine);
        return Err(resident_failure(
            M1AuthenticatedResidentStageV1::Cancellation,
            true,
            M1AuthenticatedResidentQueueTeardownV1::from_speculative_disposition(disposition)
                .retain((
                    registry,
                    reservation,
                    engine,
                    rounds,
                    evidence,
                    unused_plans,
                )),
        ));
    }
    if let Err(failure) = registry.record_new_window_publication(reservation) {
        engine.quarantine_m1_queue_rearm_failure();
        let disposition = published.cancel_and_close(&mut engine);
        return Err(resident_failure(
            M1AuthenticatedResidentStageV1::NewWindowPublish,
            true,
            M1AuthenticatedResidentQueueTeardownV1::from_speculative_disposition(disposition)
                .retain((registry, engine, failure, rounds, evidence, unused_plans)),
        ));
    }
    let observed = match published.observe_with_deadline(&mut engine, &mut deadline) {
        Ok(observed) => observed,
        Err(failure) => {
            let stage = if failure.error()
                == crate::M1AuthenticatedSpeculativeNewWindowObservationErrorV1::Deadline
            {
                M1AuthenticatedResidentStageV1::Cancellation
            } else {
                M1AuthenticatedResidentStageV1::NewWindowObserve
            };
            let closure = failure.cancel_and_close(&mut engine);
            return Err(resident_failure(
                stage,
                true,
                M1AuthenticatedResidentQueueTeardownV1::from_speculative_disposition(closure)
                    .retain((registry, engine, rounds, evidence, unused_plans)),
            ));
        }
    };
    let first_token_offset_ns = match clock_start.elapsed_ns() {
        Ok(offset) => offset,
        Err(error) => {
            let closure = observed.cancel_and_close(&mut engine);
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::Clock,
                true,
                M1AuthenticatedResidentQueueTeardownV1::from_speculative_disposition(closure)
                    .retain((registry, engine, rounds, evidence, unused_plans, error)),
            ));
        }
    };
    let Some(observed_member) = observed.member(0).filter(|_| observed.member_count() == 1) else {
        let closure = observed.cancel_and_close(&mut engine);
        return Err(resident_failure(
            M1AuthenticatedResidentStageV1::NewWindowObserve,
            true,
            M1AuthenticatedResidentQueueTeardownV1::from_speculative_disposition(closure).retain((
                registry,
                engine,
                rounds,
                evidence,
                unused_plans,
            )),
        ));
    };
    let continues = policy.permits_fresh_anchor(observed_member.emitted_token());
    let completion = if continues {
        M1ServingCompletionDispositionV1::Continue(plan)
    } else {
        M1ServingCompletionDispositionV1::Retire
    };
    if registry
        .preflight_completion_exact_for(registry_identity, observed.epoch(), &[completion])
        .is_err()
    {
        let closure = observed.cancel_and_close(&mut engine);
        return Err(resident_failure(
            M1AuthenticatedResidentStageV1::RegistryCompletion,
            true,
            M1AuthenticatedResidentQueueTeardownV1::from_speculative_disposition(closure).retain((
                registry,
                engine,
                rounds,
                evidence,
                unused_plans,
            )),
        ));
    }
    let physical_disposition = if continues {
        M1AuthenticatedSpeculativeNewWindowMemberDispositionV1::Continue
    } else {
        M1AuthenticatedSpeculativeNewWindowMemberDispositionV1::Retire
    };
    let released =
        match observed.settle_with_deadline(&mut engine, vec![physical_disposition], |boundary| {
            deadline(boundary, queue_wait_timeout).is_none()
        }) {
            Ok(released) => released,
            Err(failure) => {
                let stage = if failure.error()
                    == crate::M1AuthenticatedSpeculativeNewWindowSettlementErrorV1::Deadline
                {
                    M1AuthenticatedResidentStageV1::Cancellation
                } else {
                    M1AuthenticatedResidentStageV1::NewWindowSettle
                };
                let closure = failure.cancel_and_close(&mut engine);
                return Err(resident_failure(
                    stage,
                    true,
                    M1AuthenticatedResidentQueueTeardownV1::from_speculative_disposition(closure)
                        .retain((registry, engine, rounds, evidence, unused_plans)),
                ));
            }
        };
    registry.apply_preflighted_completion(released.epoch(), &[completion]);
    let Some(released_member) = released.member(0).filter(|_| released.member_count() == 1) else {
        let closure = released.cancel_and_close(&mut engine);
        return Err(resident_failure(
            M1AuthenticatedResidentStageV1::NewWindowSettle,
            true,
            M1AuthenticatedResidentQueueTeardownV1::from_speculative_disposition(closure).retain((
                registry,
                engine,
                rounds,
                evidence,
                unused_plans,
            )),
        ));
    };
    if released_member.request() != next_request
        || released_member.status()
            != M1AuthenticatedSpeculativeNewWindowReleasedMemberStatusV1::Continuing
    {
        let closure = released.cancel_and_close(&mut engine);
        return Err(resident_failure(
            M1AuthenticatedResidentStageV1::SuccessorSchedule,
            true,
            M1AuthenticatedResidentQueueTeardownV1::from_speculative_disposition(closure).retain((
                registry,
                engine,
                rounds,
                evidence,
                unused_plans,
            )),
        ));
    }
    let Some(first_plans) = rounds.pop_front() else {
        let closure = released.cancel_and_close(&mut engine);
        return Err(resident_failure(
            M1AuthenticatedResidentStageV1::Input,
            true,
            M1AuthenticatedResidentQueueTeardownV1::from_speculative_disposition(closure).retain((
                registry,
                engine,
                evidence,
                unused_plans,
            )),
        ));
    };
    let batch = match registry.plan_next() {
        Ok(Some(batch)) => batch,
        result => {
            let closure = released.cancel_and_close(&mut engine);
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::RegistryPlan,
                true,
                M1AuthenticatedResidentQueueTeardownV1::from_speculative_disposition(closure)
                    .retain((
                        registry,
                        engine,
                        result,
                        first_plans,
                        rounds,
                        evidence,
                        unused_plans,
                    )),
            ));
        }
    };
    if batch.plan() != plan || batch.requests() != [next_request] {
        let closure = released.cancel_and_close(&mut engine);
        return Err(resident_failure(
            M1AuthenticatedResidentStageV1::RegistryPlan,
            true,
            M1AuthenticatedResidentQueueTeardownV1::from_speculative_disposition(closure).retain((
                registry,
                engine,
                batch,
                first_plans,
                rounds,
                evidence,
                unused_plans,
            )),
        ));
    }
    let draft = one_lane_role_inputs(
        released.retained_logical_runner(),
        plan.draft(),
        next_request,
        batch.epoch(),
        released_member.emitted_token(),
        released_member.draft_committed_tokens(),
    );
    let target = one_lane_role_inputs(
        released.retained_logical_runner(),
        plan.target(),
        next_request,
        batch.epoch(),
        released_member.emitted_token(),
        released_member.target_committed_tokens(),
    );
    let coordinator = M1SpeculativeGenerationLoopV1::new(
        plan.target(),
        &[M1SpeculativeMemberSeedV1::new(
            next_request,
            released_member.emitted_token(),
            released_member.target_committed_tokens(),
            released_member.draft_committed_tokens(),
            policy,
        )],
    );
    let (Some(draft), Some(target), Ok(coordinator)) = (draft, target, coordinator) else {
        let closure = released.cancel_and_close(&mut engine);
        return Err(resident_failure(
            M1AuthenticatedResidentStageV1::LogicalInputs,
            true,
            M1AuthenticatedResidentQueueTeardownV1::from_speculative_disposition(closure).retain((
                registry,
                engine,
                first_plans,
                rounds,
                evidence,
                unused_plans,
            )),
        ));
    };
    let epoch = batch.epoch();
    let successor_reservation = match registry.reserve_publication(batch) {
        Ok(reservation) => reservation,
        Err(error) => {
            let closure = released.cancel_and_close(&mut engine);
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::RegistryReservation,
                true,
                M1AuthenticatedResidentQueueTeardownV1::from_speculative_disposition(closure)
                    .retain((
                        registry,
                        engine,
                        error,
                        first_plans,
                        rounds,
                        evidence,
                        unused_plans,
                    )),
            ));
        }
    };
    let successor_batch = successor_reservation.physical_batch();
    let successor_identity = successor_reservation.registry_identity();
    let mut pending_schedule = Some((
        released,
        coordinator,
        M1AuthenticatedSpeculativeNewWindowSuccessorInputsV1::new(draft, target),
        first_plans.recipe,
        first_plans.preparation,
    ));
    let Some(scheduled) = attempt_resident_schedule_if_live(
        &mut deadline,
        M1AuthenticatedResidentDeadlineBoundaryV1::BeforeSuccessorRolloverSchedule,
        queue_wait_timeout,
        &mut pending_schedule,
        |(released, coordinator, inputs, recipe, preparation)| {
            released.schedule_successor(
                &mut engine,
                &successor_batch,
                coordinator,
                inputs,
                recipe,
                preparation,
            )
        },
    ) else {
        let (released, coordinator, inputs, recipe, preparation) = pending_schedule
            .take()
            .expect("expired successor rollover schedule retains exact inputs");
        engine.quarantine_m1_queue_rearm_failure();
        let abort = registry.abort_publication(successor_reservation);
        let closure = released.cancel_and_close(&mut engine);
        return Err(resident_failure(
            M1AuthenticatedResidentStageV1::Cancellation,
            true,
            M1AuthenticatedResidentQueueTeardownV1::from_speculative_disposition(closure).retain((
                registry,
                engine,
                abort,
                rounds,
                evidence,
                unused_plans,
                coordinator,
                inputs,
                recipe,
                preparation,
            )),
        ));
    };
    let scheduled = match scheduled {
        Ok(scheduled) => scheduled,
        Err(crate::M1AuthenticatedSpeculativeNewWindowSuccessorJoinFailureV1::PreJoin {
            error,
            retry,
        }) => {
            let closure = retry.cancel_and_close(&mut engine);
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::SuccessorSchedule,
                true,
                M1AuthenticatedResidentQueueTeardownV1::from_speculative_disposition(closure)
                    .retain((
                        registry,
                        successor_reservation,
                        engine,
                        error,
                        rounds,
                        evidence,
                        unused_plans,
                    )),
            ));
        }
        Err(crate::M1AuthenticatedSpeculativeNewWindowSuccessorJoinFailureV1::Rollover(
            failure,
        )) => {
            let disposition = close_rollover_schedule_failure(&mut engine, failure);
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::SuccessorSchedule,
                engine.is_faulted(),
                M1AuthenticatedResidentQueueTeardownV1::from_speculative_disposition(disposition)
                    .retain((
                        registry,
                        successor_reservation,
                        engine,
                        rounds,
                        evidence,
                        unused_plans,
                    )),
            ));
        }
    };
    let prepared = match crate::authenticated_queue_rollover::prepare_m1_authenticated_speculative_rollover_retained_v1(
        &mut engine,
        scheduled,
    ) {
        Ok(prepared) => prepared,
        Err(failure) => {
            let disposition = failure.into_disposition();
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::SuccessorPrepare,
                engine.is_faulted(),
                M1AuthenticatedResidentQueueTeardownV1::from_speculative_disposition(disposition)
                    .retain((
                        registry,
                        successor_reservation,
                        engine,
                        rounds,
                        evidence,
                        unused_plans,
                    )),
            ));
        }
    };
    if deadline(
        M1AuthenticatedResidentDeadlineBoundaryV1::BeforeQueueSubmit,
        queue_wait_timeout,
    )
    .is_none()
    {
        engine.quarantine_m1_queue_rearm_failure();
        let disposition = prepared.cancel_and_close(&mut engine);
        return Err(resident_failure(
            M1AuthenticatedResidentStageV1::Cancellation,
            true,
            M1AuthenticatedResidentQueueTeardownV1::from_speculative_disposition(disposition)
                .retain((
                    registry,
                    successor_reservation,
                    engine,
                    rounds,
                    evidence,
                    unused_plans,
                )),
        ));
    }
    let published = match submit_m1_authenticated_speculative_rollover_v1(
        &mut engine,
        prepared,
        diagnostic_ring_bytes,
        queue_wait_timeout,
    ) {
        Ok(published) => published,
        Err(failure) => {
            let disposition = failure.into_disposition();
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::SuccessorPublish,
                engine.is_faulted(),
                M1AuthenticatedResidentQueueTeardownV1::from_speculative_disposition(disposition)
                    .retain((
                        registry,
                        successor_reservation,
                        engine,
                        rounds,
                        evidence,
                        unused_plans,
                    )),
            ));
        }
    };
    if deadline(
        M1AuthenticatedResidentDeadlineBoundaryV1::AfterQueueSubmit,
        queue_wait_timeout,
    )
    .is_none()
    {
        engine.quarantine_m1_queue_rearm_failure();
        let disposition = published.cancel_and_close(&mut engine);
        return Err(resident_failure(
            M1AuthenticatedResidentStageV1::Cancellation,
            true,
            M1AuthenticatedResidentQueueTeardownV1::from_speculative_disposition(disposition)
                .retain((
                    registry,
                    successor_reservation,
                    engine,
                    rounds,
                    evidence,
                    unused_plans,
                )),
        ));
    }
    if let Err(failure) = registry.record_publication(successor_reservation) {
        engine.quarantine_m1_queue_rearm_failure();
        let disposition = published.cancel_and_close(&mut engine);
        return Err(resident_failure(
            M1AuthenticatedResidentStageV1::SuccessorPublish,
            true,
            M1AuthenticatedResidentQueueTeardownV1::from_speculative_disposition(disposition)
                .retain((registry, engine, failure, rounds, evidence, unused_plans)),
        ));
    }
    let physical = match published.complete_round_with_deadline(
        &mut engine,
        vec![M1SpeculativeMemberControlV1::continuing(next_request)],
        &mut deadline,
    ) {
        Ok(physical) => physical,
        Err(failure) => {
            let stage = if failure.stage()
                == crate::M1AuthenticatedSpeculativePhysicalRoundStageV1::Deadline
            {
                M1AuthenticatedResidentStageV1::Cancellation
            } else {
                M1AuthenticatedResidentStageV1::SuccessorComplete
            };
            let disposition = failure.into_disposition();
            return Err(resident_failure(
                stage,
                engine.is_faulted(),
                M1AuthenticatedResidentQueueTeardownV1::from_speculative_disposition(disposition)
                    .retain((registry, engine, rounds, evidence, unused_plans)),
            ));
        }
    };
    let (mut executor, outcome, choices) = physical.into_parts();
    let [member] = outcome.members() else {
        engine.quarantine_m1_queue_rearm_failure();
        let teardown = close_resident_executor(
            engine,
            executor,
            (registry, outcome, choices, rounds, evidence, unused_plans),
        );
        return Err(resident_failure(
            M1AuthenticatedResidentStageV1::SuccessorComplete,
            true,
            teardown,
        ));
    };
    let disposition = if member.status() == M1SpeculativeMemberStatusV1::Active {
        M1ServingCompletionDispositionV1::Continue(plan)
    } else {
        M1ServingCompletionDispositionV1::Retire
    };
    if registry
        .preflight_completion_exact_for(successor_identity, epoch, &[disposition])
        .is_err()
    {
        engine.quarantine_m1_queue_rearm_failure();
        let teardown = close_resident_executor(
            engine,
            executor,
            (registry, outcome, choices, rounds, evidence, unused_plans),
        );
        return Err(resident_failure(
            M1AuthenticatedResidentStageV1::RegistryCompletion,
            true,
            teardown,
        ));
    }
    registry.apply_preflighted_completion(epoch, &[disposition]);
    let Some(mut round_input_storage) = preallocate_resident_round_inputs(rounds.len(), plan)
    else {
        engine.quarantine_m1_queue_rearm_failure();
        let teardown = close_resident_executor(
            engine,
            executor,
            (registry, outcome, choices, rounds, evidence, unused_plans),
        );
        return Err(resident_failure(
            M1AuthenticatedResidentStageV1::Input,
            true,
            teardown,
        ));
    };
    let mut tokens = Vec::new();
    if tokens.try_reserve_exact(128).is_err()
        || evidence.try_reserve_exact(1 + rounds.len()).is_err()
    {
        engine.quarantine_m1_queue_rearm_failure();
        let teardown = close_resident_executor(
            engine,
            executor,
            (
                registry,
                outcome,
                choices,
                rounds,
                evidence,
                unused_plans,
                round_input_storage,
            ),
        );
        return Err(resident_failure(
            M1AuthenticatedResidentStageV1::Input,
            true,
            teardown,
        ));
    }
    tokens.push(released_member.emitted_token());
    tokens.extend_from_slice(member.published().tokens());
    evidence.push(M1AuthenticatedResidentRoundEvidenceV1 {
        _outcome: outcome,
        _choices: choices,
    });

    while !executor.is_complete() {
        if deadline(
            M1AuthenticatedResidentDeadlineBoundaryV1::BeforeQueueSubmit,
            queue_wait_timeout,
        )
        .is_none()
        {
            let cancellation = registry.cancel(next_request);
            engine.quarantine_m1_queue_rearm_failure();
            let teardown = executor.destroy_queue_and_retain_state(&mut engine);
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::Cancellation,
                true,
                close_resident_result(
                    teardown,
                    (
                        registry,
                        engine,
                        rounds,
                        evidence,
                        tokens,
                        unused_plans,
                        cancellation,
                    ),
                ),
            ));
        }
        let Some(plans) = rounds.pop_front() else {
            engine.quarantine_m1_queue_rearm_failure();
            let teardown = executor.destroy_queue_and_retain_state(&mut engine);
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::Input,
                true,
                close_resident_result(teardown, (registry, engine, evidence, tokens, unused_plans)),
            ));
        };
        let Some(storage) = round_input_storage.pop_front() else {
            engine.quarantine_m1_queue_rearm_failure();
            let teardown = close_resident_executor(
                engine,
                executor,
                (registry, plans, rounds, evidence, tokens, unused_plans),
            );
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::Input,
                true,
                teardown,
            ));
        };
        let batch = match registry.plan_next() {
            Ok(Some(batch)) => batch,
            result => {
                engine.quarantine_m1_queue_rearm_failure();
                let teardown = close_resident_executor(
                    engine,
                    executor,
                    (
                        registry,
                        plans,
                        rounds,
                        evidence,
                        tokens,
                        unused_plans,
                        result,
                    ),
                );
                return Err(resident_failure(
                    M1AuthenticatedResidentStageV1::RegistryPlan,
                    true,
                    teardown,
                ));
            }
        };
        if batch.plan() != plan
            || batch.requests() != [next_request]
            || batch.action() != M1ServingQueueActionV1::SameShapeRearm
        {
            engine.quarantine_m1_queue_rearm_failure();
            let teardown = close_resident_executor(
                engine,
                executor,
                (
                    registry,
                    batch,
                    plans,
                    rounds,
                    evidence,
                    tokens,
                    unused_plans,
                ),
            );
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::RegistryPlan,
                true,
                teardown,
            ));
        }
        let epoch = batch.epoch();
        let inputs = match next_round_inputs(&executor, next_request, plan, epoch, plans, storage) {
            Some(inputs) => inputs,
            None => {
                engine.quarantine_m1_queue_rearm_failure();
                let teardown = close_resident_executor(
                    engine,
                    executor,
                    (registry, rounds, evidence, tokens, unused_plans),
                );
                return Err(resident_failure(
                    M1AuthenticatedResidentStageV1::LogicalInputs,
                    true,
                    teardown,
                ));
            }
        };
        let reservation = match registry.reserve_publication(batch) {
            Ok(reservation) => reservation,
            Err(error) => {
                engine.quarantine_m1_queue_rearm_failure();
                let teardown = close_resident_executor(
                    engine,
                    executor,
                    (
                        registry,
                        inputs,
                        rounds,
                        evidence,
                        tokens,
                        unused_plans,
                        error,
                    ),
                );
                return Err(resident_failure(
                    M1AuthenticatedResidentStageV1::RegistryReservation,
                    true,
                    teardown,
                ));
            }
        };
        let registry_identity = reservation.registry_identity();
        let mut reservation = Some(reservation);
        let physical = match executor.execute_round_with_post_submit_and_deadline(
            &mut engine,
            inputs,
            || {
                registry.record_publication(
                    reservation
                        .take()
                        .expect("post-submit registry reservation remains present"),
                )
            },
            &mut deadline,
        ) {
            Ok(physical) => physical,
            Err(failure) => {
                let abort = reservation.map(|reservation| registry.abort_publication(reservation));
                let stage = if failure.stage()
                    == crate::M1AuthenticatedSpeculativePhysicalRoundStageV1::Deadline
                {
                    M1AuthenticatedResidentStageV1::Cancellation
                } else {
                    M1AuthenticatedResidentStageV1::SameShapeRound
                };
                let disposition = failure.close_for_resident(&mut engine);
                return Err(resident_failure(
                    stage,
                    engine.is_faulted(),
                    M1AuthenticatedResidentQueueTeardownV1::from_speculative_disposition(
                        disposition,
                    )
                    .retain((
                        registry,
                        engine,
                        abort,
                        rounds,
                        evidence,
                        tokens,
                        unused_plans,
                    )),
                ));
            }
        };
        let (next_executor, outcome, choices) = physical.into_parts();
        let [member] = outcome.members() else {
            engine.quarantine_m1_queue_rearm_failure();
            let teardown = close_resident_executor(
                engine,
                next_executor,
                (registry, outcome, choices, rounds, evidence, tokens),
            );
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::RegistryCompletion,
                true,
                teardown,
            ));
        };
        let disposition = if member.status() == M1SpeculativeMemberStatusV1::Active {
            M1ServingCompletionDispositionV1::Continue(plan)
        } else {
            M1ServingCompletionDispositionV1::Retire
        };
        if registry
            .preflight_completion_exact_for(registry_identity, epoch, &[disposition])
            .is_err()
        {
            engine.quarantine_m1_queue_rearm_failure();
            let teardown = close_resident_executor(
                engine,
                next_executor,
                (registry, outcome, choices, rounds, evidence, tokens),
            );
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::RegistryCompletion,
                true,
                teardown,
            ));
        }
        if !extend_preallocated_copy(&mut tokens, member.published().tokens()) {
            engine.quarantine_m1_queue_rearm_failure();
            let teardown = close_resident_executor(
                engine,
                next_executor,
                (
                    registry,
                    outcome,
                    choices,
                    rounds,
                    evidence,
                    tokens,
                    unused_plans,
                ),
            );
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::Input,
                true,
                teardown,
            ));
        }
        let next_evidence = M1AuthenticatedResidentRoundEvidenceV1 {
            _outcome: outcome,
            _choices: choices,
        };
        if let Err(next_evidence) = push_preallocated(&mut evidence, next_evidence) {
            engine.quarantine_m1_queue_rearm_failure();
            let teardown = close_resident_executor(
                engine,
                next_executor,
                (
                    registry,
                    next_evidence,
                    rounds,
                    evidence,
                    tokens,
                    unused_plans,
                ),
            );
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::Input,
                true,
                teardown,
            ));
        }
        registry.apply_preflighted_completion(epoch, &[disposition]);
        executor = next_executor;
    }

    if deadline(
        M1AuthenticatedResidentDeadlineBoundaryV1::AfterSettlement,
        queue_wait_timeout,
    )
    .is_none()
    {
        let cancellation = registry.cancel(next_request);
        engine.quarantine_m1_queue_rearm_failure();
        let teardown = executor.destroy_queue_and_retain_state(&mut engine);
        return Err(resident_failure(
            M1AuthenticatedResidentStageV1::Cancellation,
            true,
            close_resident_result(
                teardown,
                (
                    registry,
                    engine,
                    rounds,
                    evidence,
                    tokens,
                    unused_plans,
                    cancellation,
                ),
            ),
        ));
    }

    let terminal_token_offset_ns = match clock_start.elapsed_ns() {
        Ok(offset) => offset,
        Err(error) => {
            engine.quarantine_m1_queue_rearm_failure();
            let teardown = close_resident_executor(
                engine,
                executor,
                (registry, rounds, evidence, tokens, unused_plans, error),
            );
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::Clock,
                true,
                teardown,
            ));
        }
    };
    let duration_ns = match clock_start.elapsed_ns() {
        Ok(duration) => duration,
        Err(error) => {
            engine.quarantine_m1_queue_rearm_failure();
            let teardown = close_resident_executor(
                engine,
                executor,
                (registry, rounds, evidence, tokens, unused_plans, error),
            );
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::Clock,
                true,
                teardown,
            ));
        }
    };
    let timing = match M1AuthenticatedTargetWindowTimingV1::from_resident_boundaries(
        duration_ns,
        first_token_offset_ns,
        terminal_token_offset_ns,
        tokens.len(),
    ) {
        Some(timing) => timing,
        None => {
            engine.quarantine_m1_queue_rearm_failure();
            let teardown = close_resident_executor(
                engine,
                executor,
                (registry, rounds, evidence, tokens, unused_plans),
            );
            return Err(resident_failure(
                M1AuthenticatedResidentStageV1::Timing,
                true,
                teardown,
            ));
        }
    };
    unused_plans.extend(rounds);
    Ok(M1AuthenticatedResidentWindowSuccessV1 {
        session: M1AuthenticatedResidentSessionV1 {
            registry,
            engine,
            executor,
            request: next_request,
            plan,
            completed_windows: completed_windows + 1,
            evidence,
            unused_plans,
        },
        tokens: tokens.into_boxed_slice(),
        timing,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use stats_alloc::{Region, INSTRUMENTED_SYSTEM};
    use std::alloc::System;

    #[global_allocator]
    static TEST_ALLOCATOR: &stats_alloc::StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

    fn advance_registry_round(
        registry: &mut M1ServingRegistryV1<1>,
        disposition: M1ServingCompletionDispositionV1,
    ) {
        let batch = registry.plan_next().unwrap().unwrap();
        let epoch = batch.epoch();
        let reservation = registry.reserve_publication(batch).unwrap();
        let registry_identity = reservation.registry_identity();
        registry.record_publication(reservation).unwrap();
        registry
            .preflight_completion_exact_for(registry_identity, epoch, &[disposition])
            .unwrap();
        registry.apply_preflighted_completion(epoch, &[disposition]);
    }

    fn execute_production_planning_round(
        registry: &mut M1ServingRegistryV1<1>,
        coordinator: &mut M1SpeculativeGenerationLoopV1,
        plan: M1ServingPlanV1,
        token: TokenId,
    ) {
        let batch = registry.plan_next().unwrap().unwrap();
        assert_eq!(batch.plan(), plan);
        assert_eq!(batch.action(), M1ServingQueueActionV1::SameShapeRearm);
        let epoch = batch.epoch();
        let roster = coordinator.active_roster();
        assert_eq!(coordinator.active_count(), 1);
        assert_eq!(roster.as_slice(), batch.requests());
        let binding = coordinator
            .bind_round(coordinator.next_round(), epoch, &roster)
            .unwrap();
        let reservation = registry.reserve_publication(batch).unwrap();
        let registry_identity = reservation.registry_identity();
        registry.record_publication(reservation).unwrap();
        let observations = [
            crate::speculative_generation_loop::CheckedMemberObservationV1 {
                request: roster[0],
                semantics: crate::CheckedCompletionSemantics::Speculative {
                    accepted_draft_tokens: 0,
                    correction_or_bonus: token,
                },
                emitted: crate::M1SpeculativeTokenBlockV1::from_slice(&[token]).unwrap(),
            },
        ];
        let controls = [M1SpeculativeMemberControlV1::continuing(roster[0])];
        let preflighted = coordinator
            .preflight_observed_round(
                binding,
                coordinator.shape().selection(),
                epoch,
                &observations,
                &controls,
            )
            .unwrap();
        coordinator
            .preflight_prepared_round_commit(&preflighted)
            .unwrap();
        let disposition = M1ServingCompletionDispositionV1::Continue(plan);
        registry
            .preflight_completion_exact_for(registry_identity, epoch, &[disposition])
            .unwrap();
        let outcome = coordinator.commit_preflighted_round(preflighted).unwrap();
        assert_eq!(outcome.next_active_roster(), roster.as_slice());
        registry.apply_preflighted_completion(epoch, &[disposition]);
        core::hint::black_box(outcome);
    }

    #[test]
    fn resident_protocol_is_exactly_twenty_windows() {
        assert_eq!(M1_AUTHENTICATED_RESIDENT_WINDOWS_V1, 20);
        assert_eq!(
            M1_AUTHENTICATED_RESIDENT_WINDOWS_V1,
            crate::M1_MAX_AUTHENTICATED_SPECULATIVE_WINDOWS_V1
        );
    }

    #[test]
    fn resident_close_only_permits_stop_after_confirmed_queue_absence() {
        for status in [
            M1AuthenticatedResidentQueueTeardownStatusV1::NoQueue,
            M1AuthenticatedResidentQueueTeardownStatusV1::Released,
        ] {
            let closed = M1AuthenticatedResidentCloseV1 {
                status,
                engine_quarantined: false,
                retained: Box::new(()),
            };
            assert!(closed.permits_stop_success());
            assert!(closed.permits_stop_success());
            assert!(closed.retains_all_custody());
        }

        let quarantined = M1AuthenticatedResidentCloseV1 {
            status: M1AuthenticatedResidentQueueTeardownStatusV1::Quarantined,
            engine_quarantined: true,
            retained: Box::new("ambiguous native destruction"),
        };
        assert!(!quarantined.permits_stop_success());
        assert!(!quarantined.permits_stop_success());
        assert!(quarantined.engine_quarantined());
        assert!(quarantined.retains_all_custody());
    }

    #[test]
    fn hostile_pre_schedule_expiry_preserves_owner_and_runs_no_effect() {
        #[derive(Debug)]
        struct ModelScheduleOwner {
            rolled_back: bool,
        }

        let timeout = M1QueueWaitTimeoutV1::new(10).unwrap();
        for boundary in [
            M1AuthenticatedResidentDeadlineBoundaryV1::BeforeFirstRolloverSchedule,
            M1AuthenticatedResidentDeadlineBoundaryV1::BeforeSuccessorRolloverSchedule,
        ] {
            let mut pending = Some(ModelScheduleOwner { rolled_back: false });
            let mut detaches = 0;
            let mut dispatches = 0;
            let mut reselections = 0;
            let attempt = attempt_resident_schedule_if_live(
                &mut |observed, _| {
                    assert_eq!(observed, boundary);
                    None
                },
                boundary,
                timeout,
                &mut pending,
                |owner| {
                    detaches += 1;
                    dispatches += 1;
                    reselections += 1;
                    owner
                },
            );
            assert!(attempt.is_none());
            assert_eq!((detaches, dispatches, reselections), (0, 0, 0));
            let mut owner = pending.expect("expired schedule retains rollback owner");
            owner.rolled_back = true;
            assert!(owner.rolled_back);
        }
    }

    #[test]
    #[ignore = "must run alone because the allocator counter is process-global"]
    fn warmed_resident_production_planning_round_allocates_zero_times() {
        let plan = M1ServingPlanV1::new(
            Qwen3PlanSelection {
                role: Qwen3ModelRole::Target8B,
                mode: Qwen3ExecutionMode::Speculative,
                bucket: Qwen3PlanBucket::SpeculativeS1K4C8192,
            },
            Qwen3PlanSelection {
                role: Qwen3ModelRole::Draft06B,
                mode: Qwen3ExecutionMode::Decode,
                bucket: Qwen3PlanBucket::DecodeS1C8192,
            },
        )
        .unwrap();
        let prefill = M1ServingPlanV1::new(TARGET_PREFILL, DRAFT_PREFILL).unwrap();
        let request = RequestId::new(0, 1);
        let mut registry = M1ServingRegistryV1::<1>::new().unwrap();
        registry.admit(request, prefill).unwrap();
        advance_registry_round(
            &mut registry,
            M1ServingCompletionDispositionV1::Continue(plan),
        );
        advance_registry_round(
            &mut registry,
            M1ServingCompletionDispositionV1::Continue(plan),
        );
        let policy = crate::M1SpeculativeGenerationPolicyV1::new(128, &[999]).unwrap();
        let mut coordinator = M1SpeculativeGenerationLoopV1::new(
            plan.target(),
            &[M1SpeculativeMemberSeedV1::new(request, 70, 10, 10, policy)],
        )
        .unwrap();

        execute_production_planning_round(&mut registry, &mut coordinator, plan, 700);

        let one = Region::new(&INSTRUMENTED_SYSTEM);
        execute_production_planning_round(&mut registry, &mut coordinator, plan, 701);
        let one = one.change();
        assert_eq!(one.allocations, 0, "one steady-state round allocated");
        assert_eq!(one.reallocations, 0, "one steady-state round reallocated");

        let repeated = Region::new(&INSTRUMENTED_SYSTEM);
        for token in 702..718 {
            execute_production_planning_round(&mut registry, &mut coordinator, plan, token);
        }
        let repeated = repeated.change();
        assert_eq!(repeated.allocations, 0, "repeated steady-state rounds allocated");
        assert_eq!(
            repeated.reallocations, 0,
            "repeated steady-state rounds reallocated"
        );
    }

    fn allocation_source_gate_accepts(registry: &str, coordinator: &str) -> bool {
        let plan_next = registry
            .split("pub fn plan_next")
            .nth(1)
            .and_then(|tail| tail.split("pub fn reserve_publication").next())
            .unwrap_or_default();
        let reserve = registry
            .split("pub fn reserve_publication")
            .nth(1)
            .and_then(|tail| tail.split("pub fn reserve_completed_window_replacement").next())
            .unwrap_or_default();
        let active = coordinator
            .split("pub fn active_roster")
            .nth(1)
            .and_then(|tail| tail.split("pub fn active_count").next())
            .unwrap_or_default();
        !plan_next.contains("collect::<Vec")
            && !reserve.contains("self.plan_next()")
            && !reserve.contains("batch.duplicate()")
            && !active.contains(".collect()")
            && plan_next.contains("M1ServingInlineRosterV1::new()")
            && active.contains("M1SpeculativeActiveRosterV1::new()")
    }

    #[test]
    fn hostile_heap_roster_restorations_fail_allocation_source_gate() {
        let registry = include_str!("m1_serving_registry.rs");
        let coordinator = include_str!("speculative_generation_loop.rs");
        assert!(allocation_source_gate_accepts(registry, coordinator));

        let collected_plan = registry.replacen(
            "let mut requests = M1ServingInlineRosterV1::new();",
            "let requests = self.entries.iter().collect::<Vec<_>>();",
            1,
        );
        assert!(!allocation_source_gate_accepts(&collected_plan, coordinator));

        let replanned_reservation = registry.replacen(
            "self.validate_next_batch(&batch)?;",
            "let _expected = self.plan_next()?;",
            1,
        );
        assert!(!allocation_source_gate_accepts(
            &replanned_reservation,
            coordinator
        ));

        let cloned_reservation = registry.replacen(
            "plan: batch.plan,",
            "batch: batch.duplicate(),",
            1,
        );
        assert!(!allocation_source_gate_accepts(
            &cloned_reservation,
            coordinator
        ));

        let collected_active = coordinator.replacen(
            "let mut roster = M1SpeculativeActiveRosterV1::new();",
            "return self.members.iter().map(|member| member.request).collect();",
            1,
        );
        assert!(!allocation_source_gate_accepts(registry, &collected_active));
    }

    #[test]
    fn first_window_rejects_terminal_prefill_choice_before_successor_scheduling() {
        let source = include_str!("authenticated_resident_session.rs");
        let first = source
            .split("pub fn execute_m1_authenticated_resident_first_window_v1")
            .nth(1)
            .and_then(|tail| {
                tail.split("pub fn execute_m1_authenticated_resident_next_window_v1")
                    .next()
            })
            .expect("resident first-window source is present");
        let policy = first.find("permits_speculative_successor").unwrap();
        let deadline = first.find("BeforeFirstRolloverSchedule").unwrap();
        let schedule = first.find("schedule_first_speculative_round").unwrap();
        assert!(policy < deadline && deadline < schedule);
    }

    #[test]
    fn hostile_new_window_deadline_cannot_cross_submit_observe_or_settle() {
        let source = include_str!("authenticated_resident_session.rs");
        let next = source
            .split("pub fn execute_m1_authenticated_resident_next_window_v1")
            .nth(1)
            .and_then(|tail| tail.split("#[cfg(test)]").next())
            .expect("resident next-window source is present");
        let ordered = [
            "M1AuthenticatedResidentDeadlineBoundaryV1::BeforeNewWindowSchedule",
            "schedule_m1_authenticated_speculative_new_window_v1",
            "prepare_m1_authenticated_speculative_new_window_v1",
            "M1AuthenticatedResidentDeadlineBoundaryV1::BeforeQueueSubmit",
            "submit_m1_authenticated_speculative_new_window_v1",
            "M1AuthenticatedResidentDeadlineBoundaryV1::AfterQueueSubmit",
            "registry.record_new_window_publication",
            "published.observe_with_deadline",
            ".settle_with_deadline",
            "BeforeSuccessorRolloverSchedule",
            "released.schedule_successor",
            "registry.abort_publication(successor_reservation)",
            "prepare_m1_authenticated_speculative_rollover_retained_v1",
            "M1AuthenticatedResidentDeadlineBoundaryV1::BeforeQueueSubmit",
            "submit_m1_authenticated_speculative_rollover_v1",
            "M1AuthenticatedResidentDeadlineBoundaryV1::AfterQueueSubmit",
            "registry.record_publication",
            "published.complete_round_with_deadline",
            "execute_round_with_post_submit_and_deadline",
        ];
        let mut tail = next;
        for needle in ordered {
            let position = tail
                .find(needle)
                .unwrap_or_else(|| panic!("missing hostile new-window deadline guard {needle}"));
            tail = &tail[position + needle.len()..];
        }

        let queue = include_str!("authenticated_queue_rollover.rs");
        let settle = queue
            .split("fn settle_authenticated_speculative_new_window_with_deadline")
            .nth(1)
            .and_then(|tail| {
                tail.split("fn finish_authenticated_speculative_new_window_completion(")
                    .next()
            })
            .expect("deadline-bound new-window settlement is present");
        let mut tail = settle;
        for needle in [
            "Boundary::BeforeReadback",
            "observed.check_completion",
            "Boundary::AfterReadback",
            "Boundary::BeforeSettlement",
            "readback.complete",
            "Boundary::AfterSettlement",
            "finish_authenticated_speculative_new_window_completion_with_deadline",
        ] {
            let position = tail
                .find(needle)
                .unwrap_or_else(|| panic!("missing hostile new-window settlement guard {needle}"));
            tail = &tail[position + needle.len()..];
        }
        let release = queue
            .split("fn finish_authenticated_speculative_new_window_completion_with_deadline")
            .nth(1)
            .and_then(|tail| {
                tail.split("fn finish_authenticated_speculative_new_window_release")
                    .next()
            })
            .expect("deadline-bound new-window page release is present");
        let before = release.find("Boundary::BeforeSettlement").unwrap();
        let effect = release.find("outcome.release_completed()").unwrap();
        let after = release.find("Boundary::AfterSettlement").unwrap();
        assert!(before < effect && effect < after);
    }

    #[test]
    fn resident_source_has_no_authority_or_endpoint_acquisition() {
        let production = include_str!("authenticated_resident_session.rs")
            .split_once("#[cfg(test)]")
            .map_or(
                include_str!("authenticated_resident_session.rs"),
                |(source, _)| source,
            );
        for forbidden in [
            "TcpListener",
            "UnixListener",
            "std::fs::read",
            "Command::new",
            "load_artifact",
            "raw_kfd",
        ] {
            assert!(
                !production.contains(forbidden),
                "forbidden resident authority {forbidden}"
            );
        }
    }
}
