//! Authenticated fresh-engine S8/T128 paired-prefill bootstrap.
//!
//! This is the wide counterpart to the frozen singleton bootstrap. It admits
//! one nonempty ordered live prefix, preserves the physical eight-lane shape,
//! reserves every request's independent KV state, and carries the ordinary
//! serving-registry publication reservation beside authenticated physical
//! prepublication. It creates no queue and observes no model output.

use core::{any::Any, fmt};

use fe2o3_kfd::GFX942_MAX_FIXED_DISPATCH_DATA_V1;
use ferric_spec::{
    completion::CompletionEpoch, validate_m1_step_inputs, M1StepInputCandidate,
    M1StepInputValidationOutcome, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket,
    Qwen3PlanSelection, RequestId, TokenId, ValidatedM1StepInputs, M1_KV_PAGE_TOKENS,
    M1_MAX_CONTEXT_TOKENS, QWEN3_VOCABULARY_SIZE,
};

use crate::{
    bind_m1_authenticated_speculative_rollover_intent_v1, bind_m1_kv_workspace_table_v1,
    ActiveDeviceKvCache, DeviceKvPageLease, Engine, LogicalRunnerDeclaration,
    M1AuthenticatedPhysicalRunnerV1, M1AuthenticatedPrepublicationBatchV1,
    M1AuthenticatedSpeculativeRolloverIntentV1, M1AuthenticatedSpeculativeRolloverMemberIntentV1,
    M1CaptureQuarantinedEngineV1, M1FullStepKvWorkspaceTablesV1, M1FullStepWorkspaceInputKind,
    M1FullStepWorkspacePlans, M1PartitionedModelMemoryKvPoolV1, M1PhysicalRunnerRecipeOutcomeV1,
    M1ServingPlanV1, M1ServingPublicationReservationV1, M1ServingQueueActionV1,
    M1ServingRegistryV1, M1SpeculativeGenerationPolicyV1, M1StepDispatchIntent,
};

const PHYSICAL_LANES: usize = 8;
const PREFILL_WIDTH: usize = 128;
const EXPECTED_PREFILL_QUEUE_DATA_ALLOCATIONS: usize = 11;

const TARGET_PREFILL: Qwen3PlanSelection = Qwen3PlanSelection {
    role: Qwen3ModelRole::Target8B,
    mode: Qwen3ExecutionMode::Prefill,
    bucket: Qwen3PlanBucket::PrefillS8T128,
};
const DRAFT_PREFILL: Qwen3PlanSelection = Qwen3PlanSelection {
    role: Qwen3ModelRole::Draft06B,
    mode: Qwen3ExecutionMode::Prefill,
    bucket: Qwen3PlanBucket::PrefillS8T128,
};
const TARGET_SUCCESSOR: Qwen3PlanSelection = Qwen3PlanSelection {
    role: Qwen3ModelRole::Target8B,
    mode: Qwen3ExecutionMode::Speculative,
    bucket: Qwen3PlanBucket::SpeculativeS8K4C8192,
};
const DRAFT_SUCCESSOR: Qwen3PlanSelection = Qwen3PlanSelection {
    role: Qwen3ModelRole::Draft06B,
    mode: Qwen3ExecutionMode::Decode,
    bucket: Qwen3PlanBucket::DecodeS8C8192,
};

fn prefill_plan() -> M1ServingPlanV1 {
    M1ServingPlanV1::new(TARGET_PREFILL, DRAFT_PREFILL)
        .expect("the frozen S8/T128 pair is admitted")
}

fn successor_plan() -> M1ServingPlanV1 {
    M1ServingPlanV1::new(TARGET_SUCCESSOR, DRAFT_SUCCESSOR)
        .expect("the frozen S8/K4 pair is admitted")
}

fn exact_workspace_plans(
    preparation: &M1FullStepWorkspacePlans,
    recipe: &M1FullStepWorkspacePlans,
) -> bool {
    preparation.kind() == M1FullStepWorkspaceInputKind::PairedPrefill
        && recipe.kind() == M1FullStepWorkspaceInputKind::PairedPrefill
        && preparation.target().selection() == TARGET_PREFILL
        && recipe.target().selection() == TARGET_PREFILL
        && preparation
            .draft()
            .is_some_and(|draft| draft.selection() == DRAFT_PREFILL)
        && recipe
            .draft()
            .is_some_and(|draft| draft.selection() == DRAFT_PREFILL)
        && preparation == recipe
}

/// Stable pure-input rejection before any Engine or registry mutation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedS8T128PrefillBootstrapInputErrorV1 {
    MemberCount {
        actual: usize,
    },
    PolicyCount {
        prompts: usize,
        policies: usize,
    },
    PromptLength {
        lane: usize,
        actual: usize,
    },
    TokenOutOfRange {
        lane: usize,
        column: usize,
        token: TokenId,
    },
    ContextExceeded {
        lane: usize,
        output: u32,
    },
    WorkspaceShape,
}

/// Pure rejection retaining every caller-owned input unchanged.
#[must_use = "rejected S8 bootstrap inputs remain linearly owned"]
#[derive(Debug)]
pub struct M1AuthenticatedS8T128PrefillBootstrapInputFailureV1 {
    error: M1AuthenticatedS8T128PrefillBootstrapInputErrorV1,
    prompts: Vec<Vec<TokenId>>,
    policies: Vec<M1SpeculativeGenerationPolicyV1>,
    preparation_plans: M1FullStepWorkspacePlans,
    recipe_plans: M1FullStepWorkspacePlans,
}

impl M1AuthenticatedS8T128PrefillBootstrapInputFailureV1 {
    #[must_use]
    pub const fn error(&self) -> M1AuthenticatedS8T128PrefillBootstrapInputErrorV1 {
        self.error
    }

    #[must_use = "all rejected input owners remain linear"]
    pub fn into_parts(
        self,
    ) -> (
        Vec<Vec<TokenId>>,
        Vec<M1SpeculativeGenerationPolicyV1>,
        M1FullStepWorkspacePlans,
        M1FullStepWorkspacePlans,
    ) {
        (
            self.prompts,
            self.policies,
            self.preparation_plans,
            self.recipe_plans,
        )
    }
}

fn input_failure(
    error: M1AuthenticatedS8T128PrefillBootstrapInputErrorV1,
    prompts: Vec<Vec<TokenId>>,
    policies: Vec<M1SpeculativeGenerationPolicyV1>,
    preparation_plans: M1FullStepWorkspacePlans,
    recipe_plans: M1FullStepWorkspacePlans,
) -> M1AuthenticatedS8T128PrefillBootstrapInputFailureV1 {
    M1AuthenticatedS8T128PrefillBootstrapInputFailureV1 {
        error,
        prompts,
        policies,
        preparation_plans,
        recipe_plans,
    }
}

/// Validated addressless inputs for one fresh S8/T128 publication.
#[must_use = "validated S8 bootstrap inputs remain linear"]
#[derive(Debug)]
pub struct M1AuthenticatedS8T128PrefillBootstrapInputV1 {
    prompts: Vec<Box<[TokenId]>>,
    policies: Vec<M1SpeculativeGenerationPolicyV1>,
    preparation_plans: M1FullStepWorkspacePlans,
    recipe_plans: crate::runner::M1PhysicalRunnerRecipeInputV1,
}

impl M1AuthenticatedS8T128PrefillBootstrapInputV1 {
    /// Admits the complete physical S8 live-member range, one through eight.
    ///
    /// Each prompt is exactly T128. The policy remains per member; the direct
    /// prefill choice is outside its successor output limit.
    ///
    /// # Errors
    ///
    /// Returns every input unchanged when roster cardinality, prompt bytes,
    /// policy context, or either workspace-plan copy is not exact.
    pub fn new(
        prompts: Vec<Vec<TokenId>>,
        policies: Vec<M1SpeculativeGenerationPolicyV1>,
        preparation_plans: M1FullStepWorkspacePlans,
        recipe_plans: M1FullStepWorkspacePlans,
    ) -> Result<Self, M1AuthenticatedS8T128PrefillBootstrapInputFailureV1> {
        if prompts.is_empty() || prompts.len() > PHYSICAL_LANES {
            let actual = prompts.len();
            return Err(input_failure(
                M1AuthenticatedS8T128PrefillBootstrapInputErrorV1::MemberCount { actual },
                prompts,
                policies,
                preparation_plans,
                recipe_plans,
            ));
        }
        if policies.len() != prompts.len() {
            let prompt_count = prompts.len();
            let policy_count = policies.len();
            return Err(input_failure(
                M1AuthenticatedS8T128PrefillBootstrapInputErrorV1::PolicyCount {
                    prompts: prompt_count,
                    policies: policy_count,
                },
                prompts,
                policies,
                preparation_plans,
                recipe_plans,
            ));
        }
        for (lane, prompt) in prompts.iter().enumerate() {
            if prompt.len() != PREFILL_WIDTH {
                return Err(input_failure(
                    M1AuthenticatedS8T128PrefillBootstrapInputErrorV1::PromptLength {
                        lane,
                        actual: prompt.len(),
                    },
                    prompts,
                    policies,
                    preparation_plans,
                    recipe_plans,
                ));
            }
            if let Some((column, token)) = prompt
                .iter()
                .copied()
                .enumerate()
                .find(|(_, token)| *token >= QWEN3_VOCABULARY_SIZE)
            {
                return Err(input_failure(
                    M1AuthenticatedS8T128PrefillBootstrapInputErrorV1::TokenOutOfRange {
                        lane,
                        column,
                        token,
                    },
                    prompts,
                    policies,
                    preparation_plans,
                    recipe_plans,
                ));
            }
            let output = policies[lane].max_output_tokens();
            if 129_u32
                .checked_add(output)
                .is_none_or(|total| total > M1_MAX_CONTEXT_TOKENS)
            {
                return Err(input_failure(
                    M1AuthenticatedS8T128PrefillBootstrapInputErrorV1::ContextExceeded {
                        lane,
                        output,
                    },
                    prompts,
                    policies,
                    preparation_plans,
                    recipe_plans,
                ));
            }
        }
        if !exact_workspace_plans(&preparation_plans, &recipe_plans) {
            return Err(input_failure(
                M1AuthenticatedS8T128PrefillBootstrapInputErrorV1::WorkspaceShape,
                prompts,
                policies,
                preparation_plans,
                recipe_plans,
            ));
        }
        Ok(Self {
            prompts: prompts.into_iter().map(Vec::into_boxed_slice).collect(),
            policies,
            preparation_plans,
            recipe_plans: crate::runner::M1PhysicalRunnerRecipeInputV1::plans(recipe_plans),
        })
    }

    #[must_use]
    pub fn live_member_count(&self) -> usize {
        self.prompts.len()
    }

    #[must_use]
    pub fn prompts(&self) -> &[Box<[TokenId]>] {
        &self.prompts
    }

    #[must_use]
    pub fn policies(&self) -> &[M1SpeculativeGenerationPolicyV1] {
        &self.policies
    }
}

/// Phase at which wide authenticated bootstrap stopped.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedS8T128PrefillBootstrapPhaseV1 {
    FreshEnginePreflight,
    RegistryCreation,
    EngineAdmission,
    RegistryAdmission,
    EngineReadyTransition,
    EngineDispatch,
    RegistryPlan,
    RegistryReservation,
    LogicalPlanBinding,
    StepInputValidation,
    DeviceCache,
    PrefillPageLease,
    StepReservation,
    WorkspaceTableBinding,
    RolloverPageLease,
    RecipeDerivation,
    WorkspacePreparation,
    RolloverIntentBinding,
    WorkspaceAllocation,
    RolloverOutputReservation,
    CompletionOutputAllocation,
    DiagnosticCapture,
    PrepublicationAllocationRoster,
    AuthenticatedPrepublication,
}

/// Stable terminal bootstrap reason.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedS8T128PrefillBootstrapErrorV1 {
    FreshEngineRequired,
    ScheduledRosterMismatch,
    RegistryRosterMismatch,
    HostAllocation,
    FixedDispatchDataRosterMismatch {
        expected: usize,
        actual: usize,
        maximum: usize,
    },
    LowerRejected,
}

struct OpaqueBootstrapCustody(Box<dyn Any>);

impl fmt::Debug for OpaqueBootstrapCustody {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _ = &self.0;
        formatter
            .debug_struct("OpaqueBootstrapCustody")
            .finish_non_exhaustive()
    }
}

/// Terminal failure retaining a quarantined Engine and every other owner.
#[must_use = "terminal S8 bootstrap custody must be retained"]
pub struct M1AuthenticatedS8T128PrefillBootstrapFailureV1<const C: usize> {
    phase: M1AuthenticatedS8T128PrefillBootstrapPhaseV1,
    error: M1AuthenticatedS8T128PrefillBootstrapErrorV1,
    engine: M1CaptureQuarantinedEngineV1<C>,
    retained: OpaqueBootstrapCustody,
}

impl<const C: usize> fmt::Debug for M1AuthenticatedS8T128PrefillBootstrapFailureV1<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedS8T128PrefillBootstrapFailureV1")
            .field("phase", &self.phase)
            .field("error", &self.error)
            .field("engine_quarantined", &self.engine.is_faulted())
            .field("retains_opaque_custody", &true)
            .finish_non_exhaustive()
    }
}

impl<const C: usize> M1AuthenticatedS8T128PrefillBootstrapFailureV1<C> {
    #[must_use]
    pub const fn phase(&self) -> M1AuthenticatedS8T128PrefillBootstrapPhaseV1 {
        self.phase
    }

    #[must_use]
    pub const fn error(&self) -> M1AuthenticatedS8T128PrefillBootstrapErrorV1 {
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
        M1AuthenticatedS8T128PrefillBootstrapPhaseV1,
        M1AuthenticatedS8T128PrefillBootstrapErrorV1,
        Box<dyn Any>,
    ) {
        (self.engine, self.phase, self.error, self.retained.0)
    }
}

fn terminal_failure<const C: usize>(
    engine: Engine<C>,
    phase: M1AuthenticatedS8T128PrefillBootstrapPhaseV1,
    error: M1AuthenticatedS8T128PrefillBootstrapErrorV1,
    retained: impl Any,
) -> Box<M1AuthenticatedS8T128PrefillBootstrapFailureV1<C>> {
    Box::new(M1AuthenticatedS8T128PrefillBootstrapFailureV1 {
        phase,
        error,
        engine: engine.into_m1_capture_quarantine(),
        retained: OpaqueBootstrapCustody(Box::new(retained)),
    })
}

fn padded_prefill_inputs(
    runner: &LogicalRunnerDeclaration,
    requests: &[RequestId],
    epoch: CompletionEpoch,
    selection: Qwen3PlanSelection,
    prompts: &[Box<[TokenId]>],
) -> Result<ValidatedM1StepInputs, Box<dyn fmt::Debug>> {
    let mut lanes = Vec::with_capacity(PHYSICAL_LANES);
    for request in requests.iter().copied() {
        let plan = runner
            .bind_step_plan(request, epoch, selection)
            .map_err(|error| Box::new(error) as Box<dyn fmt::Debug>)?;
        lanes.push(Some(plan));
    }
    lanes.resize_with(PHYSICAL_LANES, || None);
    let mut tokens = vec![0; PHYSICAL_LANES * PREFILL_WIDTH];
    let mut positions = vec![0; PHYSICAL_LANES * PREFILL_WIDTH];
    let mut active_lengths = vec![0; PHYSICAL_LANES];
    let context_lengths = vec![0; PHYSICAL_LANES];
    for (lane, prompt) in prompts.iter().enumerate() {
        let start = lane * PREFILL_WIDTH;
        tokens[start..start + PREFILL_WIDTH].copy_from_slice(prompt);
        for (column, position) in positions[start..start + PREFILL_WIDTH]
            .iter_mut()
            .enumerate()
        {
            *position = u32::try_from(column).expect("T128 column fits u32");
        }
        active_lengths[lane] = 128;
    }
    match validate_m1_step_inputs(M1StepInputCandidate::new(
        selection,
        lanes,
        tokens,
        positions,
        active_lengths,
        context_lengths,
    )) {
        M1StepInputValidationOutcome::Validated(inputs) => Ok(inputs),
        M1StepInputValidationOutcome::Rejected(failure) => Err(Box::new(failure)),
    }
}

fn lease_pages(
    memory: &mut M1PartitionedModelMemoryKvPoolV1,
    request: RequestId,
    role: Qwen3ModelRole,
    first: u32,
    end: u32,
) -> Result<Vec<DeviceKvPageLease>, (Vec<DeviceKvPageLease>, crate::M1DeviceKvArenaLeaseErrorV1)> {
    let mut pages = Vec::new();
    for physical_index in first..end {
        match memory.lease_page(request, role, physical_index) {
            Ok(page) => pages.push(page),
            Err(error) => return Err((pages, error)),
        }
    }
    Ok(pages)
}

fn successor_target_page_end(policy: M1SpeculativeGenerationPolicyV1) -> Option<u32> {
    let logical_rows = u32::from(
        crate::M1SpeculativePhysicalShapeV1::from_selection(TARGET_SUCCESSOR)
            .ok()?
            .draft_tokens(),
    ) + 1;
    let tokens = 128_u32.checked_add(policy.max_output_tokens().max(logical_rows))?;
    (tokens <= M1_MAX_CONTEXT_TOKENS).then(|| tokens.div_ceil(M1_KV_PAGE_TOKENS))
}

/// Authenticated physical prepublication joined to its real fresh registry reservation.
#[must_use = "S8 prepublication and registry custody remain linear"]
pub struct M1AuthenticatedS8T128PrefillPrepublicationV1<const C: usize> {
    pub(crate) engine: Engine<C>,
    pub(crate) registry: M1ServingRegistryV1<C>,
    pub(crate) reservation: M1ServingPublicationReservationV1,
    pub(crate) prepublication: M1AuthenticatedPrepublicationBatchV1,
    pub(crate) caches: Vec<ActiveDeviceKvCache>,
    pub(crate) draft_rollover_pages: Vec<DeviceKvPageLease>,
    pub(crate) target_rollover_pages: Vec<Vec<DeviceKvPageLease>>,
    pub(crate) rollover_intent: M1AuthenticatedSpeculativeRolloverIntentV1,
    pub(crate) requests: Box<[RequestId]>,
    pub(crate) prompts: Vec<Box<[TokenId]>>,
    pub(crate) policies: Vec<M1SpeculativeGenerationPolicyV1>,
}

impl<const C: usize> fmt::Debug for M1AuthenticatedS8T128PrefillPrepublicationV1<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedS8T128PrefillPrepublicationV1")
            .field("requests", &self.requests)
            .field("prepublication", &self.prepublication)
            .finish_non_exhaustive()
    }
}

impl<const C: usize> M1AuthenticatedS8T128PrefillPrepublicationV1<C> {
    #[must_use]
    pub fn requests(&self) -> &[RequestId] {
        &self.requests
    }

    pub const fn prepublication(&self) -> &M1AuthenticatedPrepublicationBatchV1 {
        &self.prepublication
    }
}

/// Builds a real fresh S8/T128 prepublication and freezes its first registry reservation.
///
/// # Errors
///
/// Any rejection after ownership enters this function quarantines the Engine
/// and retains registry, physical, KV, prompt, and policy custody opaquely.
#[allow(clippy::too_many_lines)]
pub fn prepare_m1_authenticated_s8_t128_prefill_prepublication_v1<const C: usize>(
    mut engine: Engine<C>,
    runner: M1AuthenticatedPhysicalRunnerV1,
    mut memory: M1PartitionedModelMemoryKvPoolV1,
    input: M1AuthenticatedS8T128PrefillBootstrapInputV1,
) -> Result<
    M1AuthenticatedS8T128PrefillPrepublicationV1<C>,
    Box<M1AuthenticatedS8T128PrefillBootstrapFailureV1<C>>,
> {
    use M1AuthenticatedS8T128PrefillBootstrapErrorV1 as Error;
    use M1AuthenticatedS8T128PrefillBootstrapPhaseV1 as Phase;

    if engine.is_faulted() || engine.live_count() != 0 || engine.completed_epoch().value() != 0 {
        return Err(terminal_failure(
            engine,
            Phase::FreshEnginePreflight,
            Error::FreshEngineRequired,
            (runner, memory, input),
        ));
    }
    let M1AuthenticatedS8T128PrefillBootstrapInputV1 {
        prompts,
        policies,
        preparation_plans,
        recipe_plans,
    } = input;
    let mut registry = match M1ServingRegistryV1::<C>::new() {
        Ok(registry) => registry,
        Err(error) => {
            return Err(terminal_failure(
                engine,
                Phase::RegistryCreation,
                Error::LowerRejected,
                (
                    runner,
                    memory,
                    prompts,
                    policies,
                    preparation_plans,
                    recipe_plans,
                    error,
                ),
            ));
        }
    };
    let prefill = prefill_plan();
    let successor = successor_plan();
    let mut requests = Vec::new();
    if requests.try_reserve_exact(prompts.len()).is_err() {
        return Err(terminal_failure(
            engine,
            Phase::EngineAdmission,
            Error::HostAllocation,
            (
                runner,
                memory,
                registry,
                prompts,
                policies,
                preparation_plans,
                recipe_plans,
            ),
        ));
    }
    for lane in 0..prompts.len() {
        let request = match engine.admit() {
            Ok(request) => request,
            Err(error) => {
                return Err(terminal_failure(
                    engine,
                    Phase::EngineAdmission,
                    Error::LowerRejected,
                    (
                        runner,
                        memory,
                        registry,
                        prompts,
                        policies,
                        preparation_plans,
                        recipe_plans,
                        requests,
                        lane,
                        error,
                    ),
                ));
            }
        };
        if let Err(error) = registry.admit(request, prefill) {
            return Err(terminal_failure(
                engine,
                Phase::RegistryAdmission,
                Error::LowerRejected,
                (
                    runner,
                    memory,
                    registry,
                    prompts,
                    policies,
                    preparation_plans,
                    recipe_plans,
                    requests,
                    request,
                    error,
                ),
            ));
        }
        if let Err(error) = engine.append_tentative(request, 1) {
            return Err(terminal_failure(
                engine,
                Phase::EngineReadyTransition,
                Error::LowerRejected,
                (
                    runner,
                    memory,
                    registry,
                    prompts,
                    policies,
                    preparation_plans,
                    recipe_plans,
                    requests,
                    request,
                    error,
                ),
            ));
        }
        requests.push(request);
    }
    let scheduled = match engine.dispatch_m1_ready() {
        Ok(Some(scheduled))
            if scheduled.member_count() == requests.len()
                && requests
                    .iter()
                    .copied()
                    .enumerate()
                    .all(|(lane, request)| scheduled.member(lane) == Some(request)) =>
        {
            scheduled
        }
        Ok(scheduled) => {
            return Err(terminal_failure(
                engine,
                Phase::EngineDispatch,
                Error::ScheduledRosterMismatch,
                (
                    runner,
                    memory,
                    registry,
                    prompts,
                    policies,
                    preparation_plans,
                    recipe_plans,
                    requests,
                    scheduled,
                ),
            ));
        }
        Err(error) => {
            return Err(terminal_failure(
                engine,
                Phase::EngineDispatch,
                Error::LowerRejected,
                (
                    runner,
                    memory,
                    registry,
                    prompts,
                    policies,
                    preparation_plans,
                    recipe_plans,
                    requests,
                    error,
                ),
            ));
        }
    };
    let epoch = scheduled.epoch();
    let batch = match registry.plan_next() {
        Ok(Some(batch))
            if batch.plan() == prefill
                && batch.epoch() == epoch
                && batch.requests() == requests.as_slice()
                && batch.action() == M1ServingQueueActionV1::FreshLaunch =>
        {
            batch
        }
        Ok(batch) => {
            return Err(terminal_failure(
                engine,
                Phase::RegistryPlan,
                Error::RegistryRosterMismatch,
                (
                    runner,
                    memory,
                    registry,
                    prompts,
                    policies,
                    preparation_plans,
                    recipe_plans,
                    requests,
                    scheduled,
                    batch,
                ),
            ));
        }
        Err(error) => {
            return Err(terminal_failure(
                engine,
                Phase::RegistryPlan,
                Error::LowerRejected,
                (
                    runner,
                    memory,
                    registry,
                    prompts,
                    policies,
                    preparation_plans,
                    recipe_plans,
                    requests,
                    scheduled,
                    error,
                ),
            ));
        }
    };
    let reservation = match registry.reserve_publication(batch) {
        Ok(reservation) => reservation,
        Err(error) => {
            return Err(terminal_failure(
                engine,
                Phase::RegistryReservation,
                Error::LowerRejected,
                (
                    runner,
                    memory,
                    registry,
                    prompts,
                    policies,
                    preparation_plans,
                    recipe_plans,
                    requests,
                    scheduled,
                    error,
                ),
            ));
        }
    };
    let target_inputs = match padded_prefill_inputs(
        runner.logical_runner(),
        &requests,
        epoch,
        TARGET_PREFILL,
        &prompts,
    ) {
        Ok(inputs) => inputs,
        Err(error) => {
            return Err(terminal_failure(
                engine,
                Phase::StepInputValidation,
                Error::LowerRejected,
                (
                    runner,
                    memory,
                    registry,
                    reservation,
                    prompts,
                    policies,
                    preparation_plans,
                    recipe_plans,
                    requests,
                    scheduled,
                    error,
                ),
            ));
        }
    };
    let draft_inputs = match padded_prefill_inputs(
        runner.logical_runner(),
        &requests,
        epoch,
        DRAFT_PREFILL,
        &prompts,
    ) {
        Ok(inputs) => inputs,
        Err(error) => {
            return Err(terminal_failure(
                engine,
                Phase::StepInputValidation,
                Error::LowerRejected,
                (
                    runner,
                    memory,
                    registry,
                    reservation,
                    prompts,
                    policies,
                    preparation_plans,
                    recipe_plans,
                    requests,
                    scheduled,
                    target_inputs,
                    error,
                ),
            ));
        }
    };
    let page_count = 128_u32.div_ceil(M1_KV_PAGE_TOKENS);
    let mut caches = Vec::new();
    let mut target_pending = Vec::new();
    let mut draft_pending = Vec::new();
    if caches.try_reserve_exact(requests.len()).is_err()
        || target_pending.try_reserve_exact(requests.len()).is_err()
        || draft_pending.try_reserve_exact(requests.len()).is_err()
    {
        return Err(terminal_failure(
            engine,
            Phase::DeviceCache,
            Error::HostAllocation,
            (
                runner,
                memory,
                registry,
                reservation,
                prompts,
                policies,
                preparation_plans,
                recipe_plans,
                requests,
                scheduled,
                target_inputs,
                draft_inputs,
            ),
        ));
    }
    for request in requests.iter().copied() {
        let mut cache =
            match ActiveDeviceKvCache::new(memory.device(), request, TARGET_PREFILL, DRAFT_PREFILL)
            {
                Ok(cache) => cache,
                Err(error) => {
                    return Err(terminal_failure(
                        engine,
                        Phase::DeviceCache,
                        Error::LowerRejected,
                        (
                            runner,
                            memory,
                            registry,
                            reservation,
                            prompts,
                            policies,
                            preparation_plans,
                            recipe_plans,
                            requests,
                            scheduled,
                            target_inputs,
                            draft_inputs,
                            caches,
                            target_pending,
                            draft_pending,
                            error,
                        ),
                    ));
                }
            };
        let target_pages = match lease_pages(
            &mut memory,
            request,
            Qwen3ModelRole::Target8B,
            0,
            page_count,
        ) {
            Ok(pages) => pages,
            Err(error) => {
                return Err(terminal_failure(
                    engine,
                    Phase::PrefillPageLease,
                    Error::LowerRejected,
                    (
                        runner,
                        memory,
                        registry,
                        reservation,
                        prompts,
                        policies,
                        preparation_plans,
                        recipe_plans,
                        requests,
                        scheduled,
                        target_inputs,
                        draft_inputs,
                        caches,
                        target_pending,
                        draft_pending,
                        cache,
                        error,
                    ),
                ));
            }
        };
        let draft_pages = match lease_pages(
            &mut memory,
            request,
            Qwen3ModelRole::Draft06B,
            0,
            page_count,
        ) {
            Ok(pages) => pages,
            Err(error) => {
                return Err(terminal_failure(
                    engine,
                    Phase::PrefillPageLease,
                    Error::LowerRejected,
                    (
                        runner,
                        memory,
                        registry,
                        reservation,
                        prompts,
                        policies,
                        preparation_plans,
                        recipe_plans,
                        requests,
                        scheduled,
                        target_inputs,
                        draft_inputs,
                        caches,
                        target_pending,
                        draft_pending,
                        cache,
                        target_pages,
                        error,
                    ),
                ));
            }
        };
        let target = match cache.reserve_step_write(
            request,
            Qwen3ModelRole::Target8B,
            0,
            128,
            epoch,
            target_pages,
        ) {
            Ok(pending) => pending,
            Err(error) => {
                return Err(terminal_failure(
                    engine,
                    Phase::StepReservation,
                    Error::LowerRejected,
                    (
                        runner,
                        memory,
                        registry,
                        reservation,
                        prompts,
                        policies,
                        preparation_plans,
                        recipe_plans,
                        requests,
                        scheduled,
                        target_inputs,
                        draft_inputs,
                        caches,
                        target_pending,
                        draft_pending,
                        cache,
                        draft_pages,
                        error,
                    ),
                ));
            }
        };
        let draft = match cache.reserve_step_write(
            request,
            Qwen3ModelRole::Draft06B,
            0,
            128,
            epoch,
            draft_pages,
        ) {
            Ok(pending) => pending,
            Err(error) => {
                return Err(terminal_failure(
                    engine,
                    Phase::StepReservation,
                    Error::LowerRejected,
                    (
                        runner,
                        memory,
                        registry,
                        reservation,
                        prompts,
                        policies,
                        preparation_plans,
                        recipe_plans,
                        requests,
                        scheduled,
                        target_inputs,
                        draft_inputs,
                        caches,
                        target_pending,
                        draft_pending,
                        cache,
                        target,
                        error,
                    ),
                ));
            }
        };
        caches.push(cache);
        target_pending.push(target);
        draft_pending.push(draft);
    }
    let target_table = match bind_m1_kv_workspace_table_v1(target_inputs, target_pending) {
        Ok(table) => table,
        Err(error) => {
            return Err(terminal_failure(
                engine,
                Phase::WorkspaceTableBinding,
                Error::LowerRejected,
                (
                    runner,
                    memory,
                    registry,
                    reservation,
                    prompts,
                    policies,
                    preparation_plans,
                    recipe_plans,
                    requests,
                    scheduled,
                    draft_inputs,
                    caches,
                    draft_pending,
                    error,
                ),
            ));
        }
    };
    let draft_table = match bind_m1_kv_workspace_table_v1(draft_inputs, draft_pending) {
        Ok(table) => table,
        Err(error) => {
            return Err(terminal_failure(
                engine,
                Phase::WorkspaceTableBinding,
                Error::LowerRejected,
                (
                    runner,
                    memory,
                    registry,
                    reservation,
                    prompts,
                    policies,
                    preparation_plans,
                    recipe_plans,
                    requests,
                    scheduled,
                    target_table,
                    caches,
                    error,
                ),
            ));
        }
    };
    let mut draft_rollover_pages = Vec::new();
    let mut target_rollover_pages = Vec::new();
    if draft_rollover_pages
        .try_reserve_exact(requests.len())
        .is_err()
        || target_rollover_pages
            .try_reserve_exact(requests.len())
            .is_err()
    {
        return Err(terminal_failure(
            engine,
            Phase::RolloverPageLease,
            Error::HostAllocation,
            (
                runner,
                memory,
                registry,
                reservation,
                prompts,
                policies,
                preparation_plans,
                recipe_plans,
                requests,
                scheduled,
                target_table,
                draft_table,
                caches,
            ),
        ));
    }
    for (lane, request) in requests.iter().copied().enumerate() {
        let Some(target_end) = successor_target_page_end(policies[lane]) else {
            return Err(terminal_failure(
                engine,
                Phase::RolloverPageLease,
                Error::LowerRejected,
                (
                    runner,
                    memory,
                    registry,
                    reservation,
                    prompts,
                    policies,
                    preparation_plans,
                    recipe_plans,
                    requests,
                    scheduled,
                    target_table,
                    draft_table,
                    caches,
                    draft_rollover_pages,
                    target_rollover_pages,
                    lane,
                ),
            ));
        };
        let target = match lease_pages(
            &mut memory,
            request,
            Qwen3ModelRole::Target8B,
            page_count,
            target_end,
        ) {
            Ok(pages) => pages,
            Err(error) => {
                return Err(terminal_failure(
                    engine,
                    Phase::RolloverPageLease,
                    Error::LowerRejected,
                    (
                        runner,
                        memory,
                        registry,
                        reservation,
                        prompts,
                        policies,
                        preparation_plans,
                        recipe_plans,
                        requests,
                        scheduled,
                        target_table,
                        draft_table,
                        caches,
                        draft_rollover_pages,
                        target_rollover_pages,
                        lane,
                        error,
                    ),
                ));
            }
        };
        let mut draft = match lease_pages(
            &mut memory,
            request,
            Qwen3ModelRole::Draft06B,
            page_count,
            page_count + 1,
        ) {
            Ok(pages) => pages,
            Err(error) => {
                return Err(terminal_failure(
                    engine,
                    Phase::RolloverPageLease,
                    Error::LowerRejected,
                    (
                        runner,
                        memory,
                        registry,
                        reservation,
                        prompts,
                        policies,
                        preparation_plans,
                        recipe_plans,
                        requests,
                        scheduled,
                        target_table,
                        draft_table,
                        caches,
                        draft_rollover_pages,
                        target_rollover_pages,
                        target,
                        lane,
                        error,
                    ),
                ));
            }
        };
        let Some(draft) = draft.pop() else {
            return Err(terminal_failure(
                engine,
                Phase::RolloverPageLease,
                Error::LowerRejected,
                (
                    runner,
                    memory,
                    registry,
                    reservation,
                    prompts,
                    policies,
                    preparation_plans,
                    recipe_plans,
                    requests,
                    scheduled,
                    target_table,
                    draft_table,
                    caches,
                    draft_rollover_pages,
                    target_rollover_pages,
                    target,
                    lane,
                ),
            ));
        };
        draft_rollover_pages.push(draft);
        target_rollover_pages.push(target);
    }
    let recipe = match recipe_plans.derive(
        runner.operations(),
        M1StepDispatchIntent::PairedPrefill(TARGET_PREFILL),
    ) {
        M1PhysicalRunnerRecipeOutcomeV1::Prepared(recipe) => recipe,
        M1PhysicalRunnerRecipeOutcomeV1::Rejected(error) => {
            return Err(terminal_failure(
                engine,
                Phase::RecipeDerivation,
                Error::LowerRejected,
                (
                    runner,
                    memory,
                    registry,
                    reservation,
                    prompts,
                    policies,
                    preparation_plans,
                    requests,
                    scheduled,
                    target_table,
                    draft_table,
                    caches,
                    draft_rollover_pages,
                    target_rollover_pages,
                    error,
                ),
            ));
        }
    };
    let tables = M1FullStepKvWorkspaceTablesV1::PairedPrefill {
        draft: draft_table,
        target: target_table,
    };
    let prepared = match runner.prepare_scheduled_workspaces(scheduled, preparation_plans, tables) {
        Ok(prepared) => prepared,
        Err(error) => {
            return Err(terminal_failure(
                engine,
                Phase::WorkspacePreparation,
                Error::LowerRejected,
                (
                    runner,
                    memory,
                    registry,
                    reservation,
                    prompts,
                    policies,
                    requests,
                    caches,
                    draft_rollover_pages,
                    target_rollover_pages,
                    recipe,
                    error,
                ),
            ));
        }
    };
    let member_intents = requests
        .iter()
        .copied()
        .zip(policies.iter().copied())
        .map(|(request, policy)| {
            M1AuthenticatedSpeculativeRolloverMemberIntentV1::new(request, policy)
        })
        .collect();
    let intent_prepared = match bind_m1_authenticated_speculative_rollover_intent_v1(
        prepared,
        successor,
        member_intents,
    ) {
        Ok(prepared) => prepared,
        Err(error) => {
            return Err(terminal_failure(
                engine,
                Phase::RolloverIntentBinding,
                Error::LowerRejected,
                (
                    runner,
                    memory,
                    registry,
                    reservation,
                    prompts,
                    policies,
                    requests,
                    caches,
                    draft_rollover_pages,
                    target_rollover_pages,
                    recipe,
                    error,
                ),
            ));
        }
    };
    let (prepared, rollover_intent) = intent_prepared.into_parts();
    let mut allocated = match runner.allocate_scheduled_workspaces(memory, prepared) {
        Ok(allocated) => allocated,
        Err(error) => {
            return Err(terminal_failure(
                engine,
                Phase::WorkspaceAllocation,
                Error::LowerRejected,
                (
                    runner,
                    registry,
                    reservation,
                    prompts,
                    policies,
                    requests,
                    caches,
                    draft_rollover_pages,
                    target_rollover_pages,
                    recipe,
                    rollover_intent,
                    error,
                ),
            ));
        }
    };
    if let Err(error) = allocated.reserve_finite_speculative_rollover_output(TARGET_SUCCESSOR) {
        return Err(terminal_failure(
            engine,
            Phase::RolloverOutputReservation,
            Error::LowerRejected,
            (
                runner,
                registry,
                reservation,
                prompts,
                policies,
                requests,
                caches,
                draft_rollover_pages,
                target_rollover_pages,
                allocated,
                recipe,
                rollover_intent,
                error,
            ),
        ));
    }
    let completion = match allocated.allocate_completion_output(TARGET_PREFILL) {
        Ok(completion) => completion,
        Err(error) => {
            return Err(terminal_failure(
                engine,
                Phase::CompletionOutputAllocation,
                Error::LowerRejected,
                (
                    runner,
                    registry,
                    reservation,
                    prompts,
                    policies,
                    requests,
                    caches,
                    draft_rollover_pages,
                    target_rollover_pages,
                    allocated,
                    recipe,
                    rollover_intent,
                    error,
                ),
            ));
        }
    };
    let completion = match allocated.enable_direct_diagnostic_choices_capture(completion) {
        Ok(completion) => completion,
        Err(error) => {
            return Err(terminal_failure(
                engine,
                Phase::DiagnosticCapture,
                Error::LowerRejected,
                (
                    runner,
                    registry,
                    reservation,
                    prompts,
                    policies,
                    requests,
                    caches,
                    draft_rollover_pages,
                    target_rollover_pages,
                    allocated,
                    recipe,
                    rollover_intent,
                    error,
                ),
            ));
        }
    };
    let actual = allocated.partitioned_memory().retained_allocation_count();
    if actual != EXPECTED_PREFILL_QUEUE_DATA_ALLOCATIONS
        || actual > GFX942_MAX_FIXED_DISPATCH_DATA_V1
    {
        return Err(terminal_failure(
            engine,
            Phase::PrepublicationAllocationRoster,
            Error::FixedDispatchDataRosterMismatch {
                expected: EXPECTED_PREFILL_QUEUE_DATA_ALLOCATIONS,
                actual,
                maximum: GFX942_MAX_FIXED_DISPATCH_DATA_V1,
            },
            (
                runner,
                registry,
                reservation,
                prompts,
                policies,
                requests,
                caches,
                draft_rollover_pages,
                target_rollover_pages,
                allocated,
                recipe,
                rollover_intent,
                completion,
            ),
        ));
    }
    let prepublication = match runner.prepare_first_step(allocated, recipe, completion) {
        Ok(prepublication) => prepublication,
        Err(error) => {
            return Err(terminal_failure(
                engine,
                Phase::AuthenticatedPrepublication,
                Error::LowerRejected,
                (
                    registry,
                    reservation,
                    prompts,
                    policies,
                    requests,
                    caches,
                    draft_rollover_pages,
                    target_rollover_pages,
                    rollover_intent,
                    error,
                ),
            ));
        }
    };
    Ok(M1AuthenticatedS8T128PrefillPrepublicationV1 {
        engine,
        registry,
        reservation,
        prepublication,
        caches,
        draft_rollover_pages,
        target_rollover_pages,
        rollover_intent,
        requests: requests.into_boxed_slice(),
        prompts,
        policies,
    })
}

#[cfg(test)]
mod tests {
    use ferric_build::{
        generate_qwen3_gfx942_runner_declaration, m1_step_workspace_requirements,
        plan_addressless_m1_step_workspace, publish_qwen3_gfx942_runner_declaration,
        qwen3_runner_closure_test_fixture, AddresslessM1StepWorkspacePlan,
        AvailableM1StepWorkspace, DeclaredM1StepWorkspaceAllocation, M1StepWorkspaceDeclaration,
        M1StepWorkspacePlanOutcome,
    };
    use ferric_spec::Identity;

    use super::*;

    fn logical_runner() -> LogicalRunnerDeclaration {
        let generated =
            generate_qwen3_gfx942_runner_declaration(qwen3_runner_closure_test_fixture())
                .expect("generate fixture runner");
        let published =
            publish_qwen3_gfx942_runner_declaration(generated).expect("publish fixture runner");
        LogicalRunnerDeclaration::from_published(published)
    }

    fn plan(selection: Qwen3PlanSelection, byte: u8) -> AddresslessM1StepWorkspacePlan {
        let requirements = m1_step_workspace_requirements(selection).expect("requirements");
        let available = AvailableM1StepWorkspace::new(M1StepWorkspaceDeclaration::new(
            selection,
            DeclaredM1StepWorkspaceAllocation::new(
                Identity::new([byte; 32]),
                requirements.allocation_byte_len(),
                requirements.allocation_alignment(),
            ),
            requirements.ranges().to_vec().into_boxed_slice(),
        ));
        match plan_addressless_m1_step_workspace(selection, available) {
            M1StepWorkspacePlanOutcome::Planned(plan) => plan,
            M1StepWorkspacePlanOutcome::Rejected(_) => panic!("workspace fixture rejected"),
        }
    }

    fn plans() -> (M1FullStepWorkspacePlans, M1FullStepWorkspacePlans) {
        let build = || {
            M1FullStepWorkspacePlans::paired_prefill(
                plan(DRAFT_PREFILL, 1),
                plan(TARGET_PREFILL, 2),
            )
        };
        (build(), build())
    }

    fn input(count: usize) -> M1AuthenticatedS8T128PrefillBootstrapInputV1 {
        let (preparation, recipe) = plans();
        M1AuthenticatedS8T128PrefillBootstrapInputV1::new(
            (0..count)
                .map(|lane| vec![u32::try_from(lane).unwrap() + 1; 128])
                .collect(),
            (0..count)
                .map(|lane| {
                    M1SpeculativeGenerationPolicyV1::new(7 + u32::try_from(lane).unwrap(), &[])
                        .unwrap()
                })
                .collect(),
            preparation,
            recipe,
        )
        .unwrap()
    }

    fn assert_rejected_without_loss(
        prompts: Vec<Vec<TokenId>>,
        policies: Vec<M1SpeculativeGenerationPolicyV1>,
        preparation: M1FullStepWorkspacePlans,
        recipe: M1FullStepWorkspacePlans,
        expected: M1AuthenticatedS8T128PrefillBootstrapInputErrorV1,
    ) {
        let prompt_values = prompts.clone();
        let policy_values = policies.clone();
        let prompt_owner = prompts.as_ptr();
        let prompt_rows: Vec<_> = prompts.iter().map(Vec::as_ptr).collect();
        let policy_owner = policies.as_ptr();
        let preparation_kind = preparation.kind();
        let preparation_target = core::ptr::from_ref(preparation.target());
        let preparation_draft = preparation.draft().map(core::ptr::from_ref);
        let recipe_kind = recipe.kind();
        let recipe_target = core::ptr::from_ref(recipe.target());
        let recipe_draft = recipe.draft().map(core::ptr::from_ref);
        let failure = M1AuthenticatedS8T128PrefillBootstrapInputV1::new(
            prompts,
            policies,
            preparation,
            recipe,
        )
        .unwrap_err();
        assert_eq!(failure.error(), expected);
        let (prompts, policies, preparation, recipe) = failure.into_parts();
        assert_eq!(prompts, prompt_values);
        assert_eq!(policies, policy_values);
        assert_eq!(prompts.as_ptr(), prompt_owner);
        assert_eq!(policies.as_ptr(), policy_owner);
        for (prompt, owner) in prompts.iter().zip(prompt_rows) {
            assert_eq!(prompt.as_ptr(), owner);
        }
        assert_eq!(preparation.kind(), preparation_kind);
        assert_eq!(core::ptr::from_ref(preparation.target()), preparation_target);
        assert_eq!(preparation.draft().map(core::ptr::from_ref), preparation_draft);
        assert_eq!(recipe.kind(), recipe_kind);
        assert_eq!(core::ptr::from_ref(recipe.target()), recipe_target);
        assert_eq!(recipe.draft().map(core::ptr::from_ref), recipe_draft);
    }

    #[test]
    fn admitted_live_range_is_one_through_eight() {
        for count in 1..=PHYSICAL_LANES {
            let input = input(count);
            assert_eq!(input.live_member_count(), count);
            assert_eq!(input.prompts().len(), count);
            assert_eq!(input.policies().len(), count);
            for (lane, policy) in input.policies().iter().copied().enumerate() {
                assert_eq!(policy.max_output_tokens(), 7 + u32::try_from(lane).unwrap());
            }
        }
    }

    #[test]
    fn constructor_rejects_empty_and_oversized_rosters_without_loss() {
        for count in [0, PHYSICAL_LANES + 1] {
            let (preparation, recipe) = plans();
            assert_rejected_without_loss(
                vec![vec![1; 128]; count],
                vec![M1SpeculativeGenerationPolicyV1::new(7, &[]).unwrap(); count],
                preparation,
                recipe,
                M1AuthenticatedS8T128PrefillBootstrapInputErrorV1::MemberCount { actual: count },
            );
        }
    }

    #[test]
    fn two_live_rows_are_ordered_and_six_lanes_are_zero_padded() {
        let input = input(2);
        let requests = [RequestId::new(3, 2), RequestId::new(7, 2)];
        for selection in [DRAFT_PREFILL, TARGET_PREFILL] {
            let rows = padded_prefill_inputs(
                &logical_runner(),
                &requests,
                CompletionEpoch::new(1),
                selection,
                input.prompts(),
            )
            .unwrap();
            assert_eq!(rows.live_lane_count(), 2);
            assert_eq!(rows.active_lengths(), &[128, 128, 0, 0, 0, 0, 0, 0]);
            assert_eq!(rows.context_lengths(), &[0; 8]);
            assert!(rows.lanes()[2..].iter().all(Option::is_none));
            assert!(rows.token_ids()[256..].iter().all(|token| *token == 0));
            assert!(rows.position_ids()[256..]
                .iter()
                .all(|position| *position == 0));
            assert_eq!(rows.lanes()[0].as_ref().unwrap().request(), requests[0]);
            assert_eq!(rows.lanes()[1].as_ref().unwrap().request(), requests[1]);
        }
    }

    #[test]
    fn every_admitted_live_count_keeps_exact_physical_s8_geometry() {
        let runner = logical_runner();
        for count in 1..=PHYSICAL_LANES {
            let input = input(count);
            let requests = (0..count)
                .map(|lane| RequestId::new(u32::try_from(lane).unwrap(), 2))
                .collect::<Vec<_>>();
            let rows = padded_prefill_inputs(
                &runner,
                &requests,
                CompletionEpoch::new(1),
                TARGET_PREFILL,
                input.prompts(),
            )
            .unwrap();
            assert_eq!(rows.live_lane_count(), u32::try_from(count).unwrap());
            assert_eq!(rows.lanes().len(), PHYSICAL_LANES);
            assert!(rows.lanes()[..count].iter().all(Option::is_some));
            assert!(rows.lanes()[count..].iter().all(Option::is_none));
            assert!(rows.active_lengths()[..count]
                .iter()
                .all(|active| *active == 128));
            assert!(rows.active_lengths()[count..]
                .iter()
                .all(|active| *active == 0));
        }
    }

    #[test]
    fn per_member_policy_sets_independent_target_tail_span() {
        let short = M1SpeculativeGenerationPolicyV1::new(1, &[]).unwrap();
        let long = M1SpeculativeGenerationPolicyV1::new(127, &[]).unwrap();
        assert_eq!(successor_target_page_end(short), Some(9));
        assert_eq!(successor_target_page_end(long), Some(16));
    }

    #[test]
    fn constructor_rejects_missing_and_extra_policies_without_loss() {
        for count in [0, 1, 3] {
            let (preparation, recipe) = plans();
            assert_rejected_without_loss(
                vec![vec![1; 128], vec![2; 128]],
                vec![M1SpeculativeGenerationPolicyV1::new(7, &[]).unwrap(); count],
                preparation,
                recipe,
                M1AuthenticatedS8T128PrefillBootstrapInputErrorV1::PolicyCount {
                    prompts: 2,
                    policies: count,
                },
            );
        }
    }

    #[test]
    fn constructor_rejects_non_t128_prompts_on_every_live_lane_without_loss() {
        for lane in 0..PHYSICAL_LANES {
            for actual in [0, PREFILL_WIDTH - 1, PREFILL_WIDTH + 1] {
                let (preparation, recipe) = plans();
                let mut prompts = vec![vec![1; PREFILL_WIDTH]; PHYSICAL_LANES];
                prompts[lane] = vec![2; actual];
                assert_rejected_without_loss(
                    prompts,
                    vec![M1SpeculativeGenerationPolicyV1::new(7, &[]).unwrap(); PHYSICAL_LANES],
                    preparation,
                    recipe,
                    M1AuthenticatedS8T128PrefillBootstrapInputErrorV1::PromptLength {
                        lane,
                        actual,
                    },
                );
            }
        }
    }

    #[test]
    fn constructor_rejects_out_of_vocabulary_tokens_without_loss() {
        for lane in 0..PHYSICAL_LANES {
            for column in [0, PREFILL_WIDTH / 2, PREFILL_WIDTH - 1] {
                for token in [QWEN3_VOCABULARY_SIZE, u32::MAX] {
                    let (preparation, recipe) = plans();
                    let mut prompts = vec![vec![1; PREFILL_WIDTH]; PHYSICAL_LANES];
                    prompts[lane][column] = token;
                    assert_rejected_without_loss(
                        prompts,
                        vec![M1SpeculativeGenerationPolicyV1::new(7, &[]).unwrap(); PHYSICAL_LANES],
                        preparation,
                        recipe,
                        M1AuthenticatedS8T128PrefillBootstrapInputErrorV1::TokenOutOfRange {
                            lane,
                            column,
                            token,
                        },
                    );
                }
            }
        }
    }

    #[test]
    fn constructor_admits_vocabulary_and_context_upper_bounds() {
        let (preparation, recipe) = plans();
        let token = QWEN3_VOCABULARY_SIZE - 1;
        let output = M1_MAX_CONTEXT_TOKENS - 129;
        let policy = M1SpeculativeGenerationPolicyV1::new(output, &[token]).unwrap();
        let admitted = M1AuthenticatedS8T128PrefillBootstrapInputV1::new(
            vec![vec![token; PREFILL_WIDTH]; PHYSICAL_LANES],
            vec![policy; PHYSICAL_LANES],
            preparation,
            recipe,
        )
        .unwrap();
        assert_eq!(admitted.live_member_count(), PHYSICAL_LANES);
        assert!(admitted
            .prompts()
            .iter()
            .all(|prompt| prompt.as_ref() == [token; PREFILL_WIDTH]));
        assert_eq!(admitted.policies(), &[policy; PHYSICAL_LANES]);
    }

    #[test]
    fn constructor_rejects_context_overflow_on_every_live_lane_without_loss() {
        for lane in 0..PHYSICAL_LANES {
            let (preparation, recipe) = plans();
            let output = M1_MAX_CONTEXT_TOKENS - 128;
            let mut policies =
                vec![M1SpeculativeGenerationPolicyV1::new(7, &[]).unwrap(); PHYSICAL_LANES];
            policies[lane] = M1SpeculativeGenerationPolicyV1::new(output, &[]).unwrap();
            assert_rejected_without_loss(
                vec![vec![1; PREFILL_WIDTH]; PHYSICAL_LANES],
                policies,
                preparation,
                recipe,
                M1AuthenticatedS8T128PrefillBootstrapInputErrorV1::ContextExceeded {
                    lane,
                    output,
                },
            );
        }
    }

    fn wrong_workspace_plans(case: usize) -> M1FullStepWorkspacePlans {
        match case {
            0 => M1FullStepWorkspacePlans::target_only(plan(TARGET_PREFILL, 2)),
            1 => M1FullStepWorkspacePlans::paired_prefill(
                plan(
                    Qwen3PlanSelection {
                        bucket: Qwen3PlanBucket::PrefillS1T128,
                        ..DRAFT_PREFILL
                    },
                    1,
                ),
                plan(
                    Qwen3PlanSelection {
                        bucket: Qwen3PlanBucket::PrefillS1T128,
                        ..TARGET_PREFILL
                    },
                    2,
                ),
            ),
            2 => M1FullStepWorkspacePlans::speculative_round(
                plan(DRAFT_SUCCESSOR, 1),
                plan(TARGET_SUCCESSOR, 2),
            ),
            3 => M1FullStepWorkspacePlans::paired_prefill(
                plan(TARGET_PREFILL, 1),
                plan(DRAFT_PREFILL, 2),
            ),
            _ => panic!("unknown workspace rejection case"),
        }
    }

    #[test]
    fn constructor_rejects_equal_but_wrong_workspace_shapes_without_loss() {
        for case in 0..4 {
            let preparation = wrong_workspace_plans(case);
            let recipe = wrong_workspace_plans(case);
            assert_eq!(preparation, recipe);
            assert_rejected_without_loss(
                vec![vec![1; PREFILL_WIDTH]; 2],
                vec![M1SpeculativeGenerationPolicyV1::new(7, &[]).unwrap(); 2],
                preparation,
                recipe,
                M1AuthenticatedS8T128PrefillBootstrapInputErrorV1::WorkspaceShape,
            );
        }
    }

    #[test]
    fn constructor_rejects_workspace_identity_drift_in_either_copy_without_loss() {
        for (draft_identity, target_identity) in [(3, 2), (1, 4)] {
            for swap in [false, true] {
                let (mut preparation, _) = plans();
                let mut recipe = M1FullStepWorkspacePlans::paired_prefill(
                    plan(DRAFT_PREFILL, draft_identity),
                    plan(TARGET_PREFILL, target_identity),
                );
                if swap {
                    core::mem::swap(&mut preparation, &mut recipe);
                }
                assert_ne!(preparation, recipe);
                assert_rejected_without_loss(
                    vec![vec![1; PREFILL_WIDTH]; 2],
                    vec![M1SpeculativeGenerationPolicyV1::new(7, &[]).unwrap(); 2],
                    preparation,
                    recipe,
                    M1AuthenticatedS8T128PrefillBootstrapInputErrorV1::WorkspaceShape,
                );
            }
        }
    }

    #[test]
    fn production_source_orders_registry_reservation_before_prepublication() {
        let source = include_str!("authenticated_s8_prefill_bootstrap.rs");
        let production = source.split("#[cfg(test)]").next().unwrap();
        let reserve = production
            .find("registry.reserve_publication(batch)")
            .unwrap();
        let prepublication = production.find("runner.prepare_first_step").unwrap();
        assert!(reserve < prepublication);
        assert!(production.contains("engine.into_m1_capture_quarantine()"));
        let prepublication_owner = production
            .split("impl<const C: usize> M1AuthenticatedS8T128PrefillPrepublicationV1<C> {")
            .nth(1)
            .unwrap()
            .split("/// Builds a real fresh S8/T128 prepublication")
            .next()
            .unwrap();
        assert!(!prepublication_owner.contains("pub fn into_parts"));
    }
}
