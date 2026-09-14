//! Authenticated fresh-engine bootstrap through paired-prefill prepublication.
//!
//! This boundary admits one Qwen3 request in `PrefillS1T128`. The legacy input
//! and preparation entrypoints retain their exact K4 successor contract. The
//! explicit finite constructor binds an already-admitted singleton K4, K8, or
//! K16 successor for resident execution. It schedules the request and binds the
//! exact pretokenized row into both model roles, reserves model KV and rollover
//! custody, and produces an authenticated prepublication batch. It does not
//! create or submit a queue, observe a clock, read a token, or claim serving.

use core::fmt;

use fe2o3_kfd::GFX942_MAX_FIXED_DISPATCH_DATA_V1;
use ferric_spec::{
    completion::CompletionEpoch, validate_m1_step_inputs, M1StepInputCandidate,
    M1StepInputValidationOutcome, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket,
    Qwen3PlanSelection, RequestId, TokenId, ValidatedM1StepInputs, M1_KV_PAGE_TOKENS,
    M1_MAX_CONTEXT_TOKENS, QWEN3_VOCABULARY_SIZE,
};

use crate::{
    bind_m1_authenticated_speculative_rollover_intent_v1, bind_m1_kv_workspace_table_v1,
    ActiveDeviceKvCache, DeviceKvPageLease, Engine, M1AuthenticatedPhysicalRunnerV1,
    M1AuthenticatedPrepublicationBatchV1, M1AuthenticatedSpeculativeRolloverIntentV1,
    M1AuthenticatedSpeculativeRolloverMemberIntentV1, M1CaptureQuarantinedEngineV1,
    M1FullStepKvWorkspaceTablesV1, M1FullStepWorkspaceInputKind, M1FullStepWorkspacePlans,
    M1PartitionedModelMemoryKvPoolV1, M1PhysicalRunnerRecipeOutcomeV1, M1ServingPlanV1,
    M1SpeculativeGenerationPolicyV1, M1StepDispatchIntent,
};

const PREFILL_WIDTH: usize = 128;
// Four model allocations, two paired workspaces, three S1/K4 successor
// outputs, one active compact output, and one direct-choice capture.
const EXPECTED_PREFILL_QUEUE_DATA_ALLOCATIONS: usize = 11;

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
const TARGET_SUCCESSOR: Qwen3PlanSelection = Qwen3PlanSelection {
    role: Qwen3ModelRole::Target8B,
    mode: Qwen3ExecutionMode::Speculative,
    bucket: Qwen3PlanBucket::SpeculativeS1K4C8192,
};
const DRAFT_SUCCESSOR: Qwen3PlanSelection = Qwen3PlanSelection {
    role: Qwen3ModelRole::Draft06B,
    mode: Qwen3ExecutionMode::Decode,
    bucket: Qwen3PlanBucket::DecodeS1C8192,
};

/// Stable rejection of an exact S1/T128 pretokenized bootstrap input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedS1T128PrefillBootstrapInputErrorV1 {
    UnsupportedSuccessor { actual: Qwen3PlanSelection },
    PromptLength { required: usize, actual: usize },
    TokenOutOfRange { column: usize, token: TokenId },
    InvalidOutputLimit { actual: u32 },
    ContextExceeded { prompt: usize, output: u32 },
    WorkspaceShape,
}

/// Input rejection retaining the exact prompt and both workspace-plan copies.
#[must_use = "rejected authenticated bootstrap inputs remain linearly owned"]
#[derive(Debug)]
pub struct M1AuthenticatedS1T128PrefillBootstrapInputFailureV1 {
    error: M1AuthenticatedS1T128PrefillBootstrapInputErrorV1,
    prompt_tokens: Vec<TokenId>,
    maximum_successor_output_tokens: u32,
    preparation_plans: M1FullStepWorkspacePlans,
    recipe_plans: M1FullStepWorkspacePlans,
}

impl M1AuthenticatedS1T128PrefillBootstrapInputFailureV1 {
    #[must_use]
    pub const fn error(&self) -> M1AuthenticatedS1T128PrefillBootstrapInputErrorV1 {
        self.error
    }

    /// Recovers every unchanged constructor input.
    #[must_use = "the rejected prompt and workspace plans remain linear"]
    pub fn into_parts(
        self,
    ) -> (
        Vec<TokenId>,
        u32,
        M1FullStepWorkspacePlans,
        M1FullStepWorkspacePlans,
    ) {
        (
            self.prompt_tokens,
            self.maximum_successor_output_tokens,
            self.preparation_plans,
            self.recipe_plans,
        )
    }
}

fn input_failure(
    error: M1AuthenticatedS1T128PrefillBootstrapInputErrorV1,
    prompt_tokens: Vec<TokenId>,
    maximum_successor_output_tokens: u32,
    preparation_plans: M1FullStepWorkspacePlans,
    recipe_plans: M1FullStepWorkspacePlans,
) -> M1AuthenticatedS1T128PrefillBootstrapInputFailureV1 {
    M1AuthenticatedS1T128PrefillBootstrapInputFailureV1 {
        error,
        prompt_tokens,
        maximum_successor_output_tokens,
        preparation_plans,
        recipe_plans,
    }
}

/// Move-only exact input for authenticated S1/T128 paired-prefill bootstrap.
#[must_use = "pretokenized bootstrap input and workspace plans remain linear"]
#[derive(Debug)]
pub struct M1AuthenticatedS1T128PrefillBootstrapInputV1 {
    prompt_tokens: Box<[TokenId]>,
    policy: M1SpeculativeGenerationPolicyV1,
    preparation_plans: M1FullStepWorkspacePlans,
    recipe_plans: crate::runner::M1PhysicalRunnerRecipeInputV1,
    successor: M1ServingPlanV1,
}

pub(crate) fn admitted_s1_t128_speculative_successor_v1(
    target: Qwen3PlanSelection,
) -> Option<M1ServingPlanV1> {
    if !matches!(
        target.bucket,
        Qwen3PlanBucket::SpeculativeS1K4C8192
            | Qwen3PlanBucket::SpeculativeS1K8C8192
            | Qwen3PlanBucket::SpeculativeS1K16C8192
    ) {
        return None;
    }
    let successor = M1ServingPlanV1::new(target, DRAFT_SUCCESSOR).ok()?;
    let prefill = M1ServingPlanV1::new(TARGET_PREFILL, DRAFT_PREFILL).ok()?;
    crate::m1_serving_registry::admit_m1_production_rollover_transition_v1(prefill, successor)?;
    Some(successor)
}

pub(crate) fn s1_t128_successor_target_tail_pages_v1(
    successor: M1ServingPlanV1,
    maximum_output_tokens: u32,
) -> Option<usize> {
    if admitted_s1_t128_speculative_successor_v1(successor.target()) != Some(successor)
        || maximum_output_tokens == 0
    {
        return None;
    }
    let logical_rows = crate::M1SpeculativePhysicalShapeV1::from_selection(successor.target())
        .ok()?
        .draft_tokens() as u32
        + 1;
    let tokens = 128_u32.checked_add(maximum_output_tokens.max(logical_rows))?;
    if tokens > M1_MAX_CONTEXT_TOKENS {
        return None;
    }
    usize::try_from(tokens.div_ceil(M1_KV_PAGE_TOKENS) - 128_u32.div_ceil(M1_KV_PAGE_TOKENS)).ok()
}

impl M1AuthenticatedS1T128PrefillBootstrapInputV1 {
    /// Binds one exact pretokenized request to paired-prefill workspace plans.
    ///
    /// The output limit applies only to successor speculative publications;
    /// the direct paired-prefill token is outside this count. Standard Qwen3
    /// terminal tokens are installed in the successor policy.
    ///
    /// # Errors
    ///
    /// Rejects a non-128 prompt, an invalid token or successor limit, a total
    /// context span above the M1 limit, or non-paired-prefill workspace plans.
    /// Every unchanged input is retained by the returned failure.
    pub fn new(
        prompt_tokens: Vec<TokenId>,
        maximum_successor_output_tokens: u32,
        preparation_plans: M1FullStepWorkspacePlans,
        recipe_plans: M1FullStepWorkspacePlans,
    ) -> Result<Self, M1AuthenticatedS1T128PrefillBootstrapInputFailureV1> {
        Self::new_with_speculative_successor(
            TARGET_SUCCESSOR,
            prompt_tokens,
            maximum_successor_output_tokens,
            preparation_plans,
            recipe_plans,
        )
    }

    /// Selects one already-admitted singleton K4, K8, or K16 successor.
    ///
    /// # Errors
    ///
    /// Retains every input on an unsupported successor or the same prompt,
    /// output-policy, and paired-prefill rejection enforced by `new`.
    pub fn new_with_speculative_successor(
        target: Qwen3PlanSelection,
        prompt_tokens: Vec<TokenId>,
        maximum_successor_output_tokens: u32,
        preparation_plans: M1FullStepWorkspacePlans,
        recipe_plans: M1FullStepWorkspacePlans,
    ) -> Result<Self, M1AuthenticatedS1T128PrefillBootstrapInputFailureV1> {
        let Some(successor) = admitted_s1_t128_speculative_successor_v1(target) else {
            return Err(input_failure(
                M1AuthenticatedS1T128PrefillBootstrapInputErrorV1::UnsupportedSuccessor {
                    actual: target,
                },
                prompt_tokens,
                maximum_successor_output_tokens,
                preparation_plans,
                recipe_plans,
            ));
        };
        if prompt_tokens.len() != PREFILL_WIDTH {
            return Err(input_failure(
                M1AuthenticatedS1T128PrefillBootstrapInputErrorV1::PromptLength {
                    required: PREFILL_WIDTH,
                    actual: prompt_tokens.len(),
                },
                prompt_tokens,
                maximum_successor_output_tokens,
                preparation_plans,
                recipe_plans,
            ));
        }
        if let Some((column, token)) = prompt_tokens
            .iter()
            .copied()
            .enumerate()
            .find(|(_, token)| *token >= QWEN3_VOCABULARY_SIZE)
        {
            return Err(input_failure(
                M1AuthenticatedS1T128PrefillBootstrapInputErrorV1::TokenOutOfRange {
                    column,
                    token,
                },
                prompt_tokens,
                maximum_successor_output_tokens,
                preparation_plans,
                recipe_plans,
            ));
        }
        if maximum_successor_output_tokens == 0
            || maximum_successor_output_tokens > M1_MAX_CONTEXT_TOKENS
        {
            return Err(input_failure(
                M1AuthenticatedS1T128PrefillBootstrapInputErrorV1::InvalidOutputLimit {
                    actual: maximum_successor_output_tokens,
                },
                prompt_tokens,
                maximum_successor_output_tokens,
                preparation_plans,
                recipe_plans,
            ));
        }
        if u32::try_from(prompt_tokens.len())
            .ok()
            .and_then(|prompt| prompt.checked_add(1))
            .and_then(|with_prefill_choice| {
                with_prefill_choice.checked_add(maximum_successor_output_tokens)
            })
            .is_none_or(|total| total > M1_MAX_CONTEXT_TOKENS)
        {
            return Err(input_failure(
                M1AuthenticatedS1T128PrefillBootstrapInputErrorV1::ContextExceeded {
                    prompt: prompt_tokens.len(),
                    output: maximum_successor_output_tokens,
                },
                prompt_tokens,
                maximum_successor_output_tokens,
                preparation_plans,
                recipe_plans,
            ));
        }
        if preparation_plans.kind() != M1FullStepWorkspaceInputKind::PairedPrefill
            || recipe_plans.kind() != M1FullStepWorkspaceInputKind::PairedPrefill
        {
            return Err(input_failure(
                M1AuthenticatedS1T128PrefillBootstrapInputErrorV1::WorkspaceShape,
                prompt_tokens,
                maximum_successor_output_tokens,
                preparation_plans,
                recipe_plans,
            ));
        }
        let policy = match M1SpeculativeGenerationPolicyV1::qwen3(maximum_successor_output_tokens) {
            Ok(policy) => policy,
            Err(_) => {
                return Err(input_failure(
                    M1AuthenticatedS1T128PrefillBootstrapInputErrorV1::InvalidOutputLimit {
                        actual: maximum_successor_output_tokens,
                    },
                    prompt_tokens,
                    maximum_successor_output_tokens,
                    preparation_plans,
                    recipe_plans,
                ));
            }
        };
        Ok(Self {
            prompt_tokens: prompt_tokens.into_boxed_slice(),
            policy,
            preparation_plans,
            recipe_plans: crate::runner::M1PhysicalRunnerRecipeInputV1::plans(recipe_plans),
            successor,
        })
    }

    #[must_use]
    pub fn prompt_tokens(&self) -> &[TokenId] {
        &self.prompt_tokens
    }

    #[must_use]
    pub const fn maximum_successor_output_tokens(&self) -> u32 {
        self.policy.max_output_tokens()
    }

    /// Exact immutable plan selected before authenticated prefill publication.
    #[must_use]
    pub const fn successor_plan(&self) -> M1ServingPlanV1 {
        self.successor
    }

    pub(crate) const fn preparation_plans(&self) -> &M1FullStepWorkspacePlans {
        &self.preparation_plans
    }

    pub(crate) fn prepare_recipe(&mut self, runner: &M1AuthenticatedPhysicalRunnerV1) -> bool {
        let pending = core::mem::replace(
            &mut self.recipe_plans,
            crate::runner::M1PhysicalRunnerRecipeInputV1::Empty,
        );
        let (prepared, accepted) =
            pending.prepare(runner, M1StepDispatchIntent::PairedPrefill(TARGET_PREFILL));
        self.recipe_plans = prepared;
        accepted
    }

    pub(crate) const fn prepared_recipe(
        &self,
    ) -> Option<&crate::AddresslessM1PhysicalBufferRecipeV1> {
        self.recipe_plans.prepared_recipe()
    }

    /// Separates a validated bootstrap only for Ferric-owned resident rollover.
    ///
    /// The values remain addressless and carry no queue or publication authority.
    pub(crate) fn into_resident_parts(
        self,
    ) -> (
        Box<[TokenId]>,
        M1SpeculativeGenerationPolicyV1,
        M1FullStepWorkspacePlans,
        crate::runner::M1PhysicalRunnerRecipeInputV1,
    ) {
        (
            self.prompt_tokens,
            self.policy,
            self.preparation_plans,
            self.recipe_plans,
        )
    }
}

/// Exact phase at which authenticated fresh-prefill preparation stopped.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedS1T128PrefillBootstrapPhaseV1 {
    FreshEnginePreflight,
    EngineAdmission,
    EngineReadyTransition,
    EngineDispatch,
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

/// Stable high-level reason for a terminal bootstrap rejection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedS1T128PrefillBootstrapErrorV1 {
    FreshEngineRequired,
    ScheduledRosterMismatch,
    FixedDispatchDataRosterMismatch {
        expected: usize,
        actual: usize,
        maximum: usize,
    },
    LowerRejected,
}

struct OpaqueM1AuthenticatedS1T128PrefillBootstrapCustodyV1(Box<dyn fmt::Debug>);

/// Terminal failure retaining all authenticated, model, KV, and prompt owners.
///
/// Failures permanently quarantine the consumed
/// Engine. The opaque retained value is deliberately not recoverable into an
/// alternate runner or structural execution path.
#[must_use = "terminal authenticated bootstrap failure custody must be retained for teardown"]
pub struct M1AuthenticatedS1T128PrefillBootstrapFailureV1<const C: usize> {
    phase: M1AuthenticatedS1T128PrefillBootstrapPhaseV1,
    error: M1AuthenticatedS1T128PrefillBootstrapErrorV1,
    engine: M1CaptureQuarantinedEngineV1<C>,
    retained: OpaqueM1AuthenticatedS1T128PrefillBootstrapCustodyV1,
}

impl<const C: usize> fmt::Debug for M1AuthenticatedS1T128PrefillBootstrapFailureV1<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedS1T128PrefillBootstrapFailureV1")
            .field("phase", &self.phase)
            .field("error", &self.error)
            .field("engine_quarantined", &self.engine.is_faulted())
            .field("retained", &self.retained.0)
            .finish_non_exhaustive()
    }
}

impl<const C: usize> M1AuthenticatedS1T128PrefillBootstrapFailureV1<C> {
    #[must_use]
    pub const fn phase(&self) -> M1AuthenticatedS1T128PrefillBootstrapPhaseV1 {
        self.phase
    }

    #[must_use]
    pub const fn error(&self) -> M1AuthenticatedS1T128PrefillBootstrapErrorV1 {
        self.error
    }

    /// All rejections retain a consumed terminal Engine owner.
    #[must_use]
    pub const fn engine_quarantined(&self) -> bool {
        true
    }

    /// Borrows terminal custody for diagnostics without exposing its owners.
    #[must_use]
    pub fn retained_debug(&self) -> &dyn fmt::Debug {
        self.retained.0.as_ref()
    }

    /// Separates terminal scheduler/KV quarantine from every other retained owner.
    #[must_use = "Engine quarantine and lower failure custody both remain terminal"]
    pub fn into_parts(
        self,
    ) -> (
        M1CaptureQuarantinedEngineV1<C>,
        M1AuthenticatedS1T128PrefillBootstrapPhaseV1,
        M1AuthenticatedS1T128PrefillBootstrapErrorV1,
        Box<dyn fmt::Debug>,
    ) {
        (self.engine, self.phase, self.error, self.retained.0)
    }
}

/// Authenticated paired-prefill batch plus every owner needed by its successor.
///
/// This is prepublication custody only. It has not created a queue, submitted
/// packets, observed completion, produced a token, or measured latency.
#[must_use = "authenticated prepublication and successor custody remain linear"]
pub struct M1AuthenticatedS1T128PrefillPrepublicationV1<const C: usize> {
    engine: Engine<C>,
    prepublication: M1AuthenticatedPrepublicationBatchV1,
    cache: ActiveDeviceKvCache,
    draft_rollover_page: DeviceKvPageLease,
    target_rollover_pages: Vec<DeviceKvPageLease>,
    rollover_intent: M1AuthenticatedSpeculativeRolloverIntentV1,
    request: RequestId,
    prompt_tokens: Box<[TokenId]>,
    policy: M1SpeculativeGenerationPolicyV1,
}

impl<const C: usize> fmt::Debug for M1AuthenticatedS1T128PrefillPrepublicationV1<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedS1T128PrefillPrepublicationV1")
            .field("engine_faulted", &self.engine.is_faulted())
            .field("request", &self.request)
            .field("prompt_tokens", &self.prompt_tokens)
            .field("policy", &self.policy)
            .field("prepublication", &self.prepublication)
            .finish_non_exhaustive()
    }
}

impl<const C: usize> M1AuthenticatedS1T128PrefillPrepublicationV1<C> {
    #[must_use]
    pub const fn request(&self) -> RequestId {
        self.request
    }

    #[must_use]
    pub fn prompt_tokens(&self) -> &[TokenId] {
        &self.prompt_tokens
    }

    #[must_use]
    pub const fn maximum_successor_output_tokens(&self) -> u32 {
        self.policy.max_output_tokens()
    }

    /// Exact target page tail preleased before physical queue custody begins.
    #[must_use = "target rollover page custody remains retained"]
    pub fn target_rollover_pages(&self) -> &[DeviceKvPageLease] {
        &self.target_rollover_pages
    }

    pub const fn prepublication(&self) -> &M1AuthenticatedPrepublicationBatchV1 {
        &self.prepublication
    }

    /// The exact fresh Engine whose dispatch authority is inside prepublication.
    #[must_use]
    pub const fn engine(&self) -> &Engine<C> {
        &self.engine
    }

    /// Separates exact queue input from successor cache/page/intent custody.
    #[must_use = "all authenticated prepublication and successor owners remain linear"]
    #[allow(clippy::type_complexity)]
    pub fn into_parts(
        self,
    ) -> (
        Engine<C>,
        M1AuthenticatedPrepublicationBatchV1,
        ActiveDeviceKvCache,
        DeviceKvPageLease,
        Vec<DeviceKvPageLease>,
        M1AuthenticatedSpeculativeRolloverIntentV1,
        RequestId,
        Box<[TokenId]>,
        M1SpeculativeGenerationPolicyV1,
    ) {
        (
            self.engine,
            self.prepublication,
            self.cache,
            self.draft_rollover_page,
            self.target_rollover_pages,
            self.rollover_intent,
            self.request,
            self.prompt_tokens,
            self.policy,
        )
    }
}

fn terminal_failure<const C: usize>(
    engine: Engine<C>,
    phase: M1AuthenticatedS1T128PrefillBootstrapPhaseV1,
    error: M1AuthenticatedS1T128PrefillBootstrapErrorV1,
    retained: impl fmt::Debug + 'static,
) -> Box<M1AuthenticatedS1T128PrefillBootstrapFailureV1<C>> {
    Box::new(M1AuthenticatedS1T128PrefillBootstrapFailureV1 {
        phase,
        error,
        engine: engine.into_m1_capture_quarantine(),
        retained: OpaqueM1AuthenticatedS1T128PrefillBootstrapCustodyV1(Box::new(retained)),
    })
}

fn padded_prefill_inputs(
    runner: &M1AuthenticatedPhysicalRunnerV1,
    request: RequestId,
    epoch: CompletionEpoch,
    selection: Qwen3PlanSelection,
    prompt: &[TokenId],
) -> Result<
    ValidatedM1StepInputs,
    (
        M1AuthenticatedS1T128PrefillBootstrapPhaseV1,
        Box<dyn fmt::Debug>,
    ),
> {
    let plan = runner
        .logical_runner()
        .bind_step_plan(request, epoch, selection)
        .map_err(|error| {
            (
                M1AuthenticatedS1T128PrefillBootstrapPhaseV1::LogicalPlanBinding,
                Box::new(error) as Box<dyn fmt::Debug>,
            )
        })?;
    let tokens = prompt.to_vec();
    let positions = (0..128_u32).collect();
    let candidate = M1StepInputCandidate::new(
        selection,
        vec![Some(plan)],
        tokens,
        positions,
        vec![128],
        vec![0],
    );
    match validate_m1_step_inputs(candidate) {
        M1StepInputValidationOutcome::Validated(inputs) => Ok(inputs),
        M1StepInputValidationOutcome::Rejected(rejection) => Err((
            M1AuthenticatedS1T128PrefillBootstrapPhaseV1::StepInputValidation,
            Box::new(rejection),
        )),
    }
}

fn lease_pages(
    memory: &mut M1PartitionedModelMemoryKvPoolV1,
    request: RequestId,
    role: Qwen3ModelRole,
    page_count: u32,
) -> Result<Vec<DeviceKvPageLease>, (Vec<DeviceKvPageLease>, crate::M1DeviceKvArenaLeaseErrorV1)> {
    let mut pages = Vec::with_capacity(page_count as usize);
    for physical_index in 0..page_count {
        match memory.lease_page(request, role, physical_index) {
            Ok(page) => pages.push(page),
            Err(error) => return Err((pages, error)),
        }
    }
    Ok(pages)
}

/// Builds authenticated S1/T128 paired-prefill prepublication from a fresh Engine.
///
/// Success forms the authenticated queue input for the first real model/KV
/// write, but does not execute that write: the returned batch has not entered a
/// physical queue. Any rejection after request admission quarantines `engine`
/// and returns every remaining owner in opaque terminal custody.
///
/// # Errors
///
/// Returns terminal custody when the Engine is not fresh or any scheduler,
/// logical-input, KV, workspace, recipe, output, or authenticated packet
/// preparation step rejects. The consumed Engine is quarantined on every
/// failure.
#[allow(clippy::too_many_lines)]
pub fn prepare_m1_authenticated_s1_t128_prefill_prepublication_v1<const C: usize>(
    engine: Engine<C>,
    runner: M1AuthenticatedPhysicalRunnerV1,
    memory: M1PartitionedModelMemoryKvPoolV1,
    input: M1AuthenticatedS1T128PrefillBootstrapInputV1,
) -> Result<
    M1AuthenticatedS1T128PrefillPrepublicationV1<C>,
    Box<M1AuthenticatedS1T128PrefillBootstrapFailureV1<C>>,
> {
    if input.successor.target() != TARGET_SUCCESSOR {
        return Err(terminal_failure(
            engine,
            M1AuthenticatedS1T128PrefillBootstrapPhaseV1::RolloverIntentBinding,
            M1AuthenticatedS1T128PrefillBootstrapErrorV1::LowerRejected,
            (runner, memory, input),
        ));
    }
    prepare_m1_authenticated_s1_t128_finite_prefill_prepublication_v1(engine, runner, memory, input)
}

#[allow(clippy::too_many_lines)]
pub(crate) fn prepare_m1_authenticated_s1_t128_finite_prefill_prepublication_v1<const C: usize>(
    mut engine: Engine<C>,
    runner: M1AuthenticatedPhysicalRunnerV1,
    mut memory: M1PartitionedModelMemoryKvPoolV1,
    input: M1AuthenticatedS1T128PrefillBootstrapInputV1,
) -> Result<
    M1AuthenticatedS1T128PrefillPrepublicationV1<C>,
    Box<M1AuthenticatedS1T128PrefillBootstrapFailureV1<C>>,
> {
    if engine.is_faulted() || engine.live_count() != 0 || engine.completed_epoch().value() != 0 {
        return Err(terminal_failure(
            engine,
            M1AuthenticatedS1T128PrefillBootstrapPhaseV1::FreshEnginePreflight,
            M1AuthenticatedS1T128PrefillBootstrapErrorV1::FreshEngineRequired,
            (runner, memory, input),
        ));
    }
    let M1AuthenticatedS1T128PrefillBootstrapInputV1 {
        prompt_tokens,
        policy,
        preparation_plans,
        recipe_plans,
        successor,
    } = input;
    let request = match engine.admit() {
        Ok(request) => request,
        Err(error) => {
            return Err(terminal_failure(
                engine,
                M1AuthenticatedS1T128PrefillBootstrapPhaseV1::EngineAdmission,
                M1AuthenticatedS1T128PrefillBootstrapErrorV1::LowerRejected,
                (
                    runner,
                    memory,
                    prompt_tokens,
                    policy,
                    preparation_plans,
                    recipe_plans,
                    error,
                ),
            ));
        }
    };
    if let Err(error) = engine.append_tentative(request, 1) {
        return Err(terminal_failure(
            engine,
            M1AuthenticatedS1T128PrefillBootstrapPhaseV1::EngineReadyTransition,
            M1AuthenticatedS1T128PrefillBootstrapErrorV1::LowerRejected,
            (
                runner,
                memory,
                prompt_tokens,
                policy,
                preparation_plans,
                recipe_plans,
                request,
                error,
            ),
        ));
    }
    let scheduled = match engine.dispatch_m1_ready() {
        Ok(Some(scheduled))
            if scheduled.member_count() == 1 && scheduled.member(0) == Some(request) =>
        {
            scheduled
        }
        Ok(scheduled) => {
            return Err(terminal_failure(
                engine,
                M1AuthenticatedS1T128PrefillBootstrapPhaseV1::EngineDispatch,
                M1AuthenticatedS1T128PrefillBootstrapErrorV1::ScheduledRosterMismatch,
                (
                    runner,
                    memory,
                    prompt_tokens,
                    policy,
                    preparation_plans,
                    recipe_plans,
                    request,
                    scheduled,
                ),
            ));
        }
        Err(error) => {
            return Err(terminal_failure(
                engine,
                M1AuthenticatedS1T128PrefillBootstrapPhaseV1::EngineDispatch,
                M1AuthenticatedS1T128PrefillBootstrapErrorV1::LowerRejected,
                (
                    runner,
                    memory,
                    prompt_tokens,
                    policy,
                    preparation_plans,
                    recipe_plans,
                    request,
                    error,
                ),
            ));
        }
    };
    let epoch = scheduled.epoch();
    let target_inputs =
        match padded_prefill_inputs(&runner, request, epoch, TARGET_PREFILL, &prompt_tokens) {
            Ok(inputs) => inputs,
            Err((phase, error)) => {
                return Err(terminal_failure(
                    engine,
                    phase,
                    M1AuthenticatedS1T128PrefillBootstrapErrorV1::LowerRejected,
                    (
                        runner,
                        memory,
                        prompt_tokens,
                        policy,
                        preparation_plans,
                        recipe_plans,
                        scheduled,
                        error,
                    ),
                ));
            }
        };
    let draft_inputs =
        match padded_prefill_inputs(&runner, request, epoch, DRAFT_PREFILL, &prompt_tokens) {
            Ok(inputs) => inputs,
            Err((phase, error)) => {
                return Err(terminal_failure(
                    engine,
                    phase,
                    M1AuthenticatedS1T128PrefillBootstrapErrorV1::LowerRejected,
                    (
                        runner,
                        memory,
                        prompt_tokens,
                        policy,
                        preparation_plans,
                        recipe_plans,
                        scheduled,
                        target_inputs,
                        error,
                    ),
                ));
            }
        };
    let mut cache =
        match ActiveDeviceKvCache::new(memory.device(), request, TARGET_PREFILL, DRAFT_PREFILL) {
            Ok(cache) => cache,
            Err(error) => {
                return Err(terminal_failure(
                    engine,
                    M1AuthenticatedS1T128PrefillBootstrapPhaseV1::DeviceCache,
                    M1AuthenticatedS1T128PrefillBootstrapErrorV1::LowerRejected,
                    (
                        runner,
                        memory,
                        prompt_tokens,
                        policy,
                        preparation_plans,
                        recipe_plans,
                        scheduled,
                        target_inputs,
                        draft_inputs,
                        error,
                    ),
                ));
            }
        };
    let active_tokens = 128_u32;
    let page_count = active_tokens.div_ceil(M1_KV_PAGE_TOKENS);
    let target_pages = match lease_pages(&mut memory, request, Qwen3ModelRole::Target8B, page_count)
    {
        Ok(pages) => pages,
        Err((pages, error)) => {
            return Err(terminal_failure(
                engine,
                M1AuthenticatedS1T128PrefillBootstrapPhaseV1::PrefillPageLease,
                M1AuthenticatedS1T128PrefillBootstrapErrorV1::LowerRejected,
                (
                    runner,
                    memory,
                    cache,
                    prompt_tokens,
                    policy,
                    preparation_plans,
                    recipe_plans,
                    scheduled,
                    target_inputs,
                    draft_inputs,
                    pages,
                    error,
                ),
            ));
        }
    };
    let draft_pages = match lease_pages(&mut memory, request, Qwen3ModelRole::Draft06B, page_count)
    {
        Ok(pages) => pages,
        Err((pages, error)) => {
            return Err(terminal_failure(
                engine,
                M1AuthenticatedS1T128PrefillBootstrapPhaseV1::PrefillPageLease,
                M1AuthenticatedS1T128PrefillBootstrapErrorV1::LowerRejected,
                (
                    runner,
                    memory,
                    cache,
                    prompt_tokens,
                    policy,
                    preparation_plans,
                    recipe_plans,
                    scheduled,
                    target_inputs,
                    draft_inputs,
                    (target_pages, pages, error),
                ),
            ));
        }
    };
    let target_pending = match cache.reserve_step_write(
        request,
        Qwen3ModelRole::Target8B,
        0,
        active_tokens,
        epoch,
        target_pages,
    ) {
        Ok(pending) => pending,
        Err(error) => {
            return Err(terminal_failure(
                engine,
                M1AuthenticatedS1T128PrefillBootstrapPhaseV1::StepReservation,
                M1AuthenticatedS1T128PrefillBootstrapErrorV1::LowerRejected,
                (
                    runner,
                    memory,
                    cache,
                    prompt_tokens,
                    policy,
                    preparation_plans,
                    recipe_plans,
                    scheduled,
                    target_inputs,
                    draft_inputs,
                    draft_pages,
                    error,
                ),
            ));
        }
    };
    let draft_pending = match cache.reserve_step_write(
        request,
        Qwen3ModelRole::Draft06B,
        0,
        active_tokens,
        epoch,
        draft_pages,
    ) {
        Ok(pending) => pending,
        Err(error) => {
            return Err(terminal_failure(
                engine,
                M1AuthenticatedS1T128PrefillBootstrapPhaseV1::StepReservation,
                M1AuthenticatedS1T128PrefillBootstrapErrorV1::LowerRejected,
                (
                    runner,
                    memory,
                    cache,
                    prompt_tokens,
                    policy,
                    preparation_plans,
                    recipe_plans,
                    scheduled,
                    target_inputs,
                    draft_inputs,
                    target_pending,
                    error,
                ),
            ));
        }
    };
    let target_table = match bind_m1_kv_workspace_table_v1(target_inputs, vec![target_pending]) {
        Ok(table) => table,
        Err(error) => {
            return Err(terminal_failure(
                engine,
                M1AuthenticatedS1T128PrefillBootstrapPhaseV1::WorkspaceTableBinding,
                M1AuthenticatedS1T128PrefillBootstrapErrorV1::LowerRejected,
                (
                    runner,
                    memory,
                    cache,
                    prompt_tokens,
                    policy,
                    preparation_plans,
                    recipe_plans,
                    scheduled,
                    draft_inputs,
                    draft_pending,
                    error,
                ),
            ));
        }
    };
    let draft_table = match bind_m1_kv_workspace_table_v1(draft_inputs, vec![draft_pending]) {
        Ok(table) => table,
        Err(error) => {
            return Err(terminal_failure(
                engine,
                M1AuthenticatedS1T128PrefillBootstrapPhaseV1::WorkspaceTableBinding,
                M1AuthenticatedS1T128PrefillBootstrapErrorV1::LowerRejected,
                (
                    runner,
                    memory,
                    cache,
                    prompt_tokens,
                    policy,
                    preparation_plans,
                    recipe_plans,
                    scheduled,
                    target_table,
                    error,
                ),
            ));
        }
    };
    let successor_target_page_count =
        s1_t128_successor_target_tail_pages_v1(successor, policy.max_output_tokens())
            .and_then(|pages| u32::try_from(pages).ok())
            .and_then(|pages| page_count.checked_add(pages))
            .unwrap_or(u32::MAX);
    let mut target_rollover_pages = Vec::new();
    if target_rollover_pages
        .try_reserve_exact(successor_target_page_count.saturating_sub(page_count) as usize)
        .is_err()
    {
        return Err(terminal_failure(
            engine,
            M1AuthenticatedS1T128PrefillBootstrapPhaseV1::RolloverPageLease,
            M1AuthenticatedS1T128PrefillBootstrapErrorV1::LowerRejected,
            (
                runner,
                memory,
                cache,
                prompt_tokens,
                policy,
                preparation_plans,
                recipe_plans,
                scheduled,
                target_table,
                draft_table,
            ),
        ));
    }
    for page_index in page_count..successor_target_page_count {
        match memory.lease_page(request, Qwen3ModelRole::Target8B, page_index) {
            Ok(page) => target_rollover_pages.push(page),
            Err(error) => {
                return Err(terminal_failure(
                    engine,
                    M1AuthenticatedS1T128PrefillBootstrapPhaseV1::RolloverPageLease,
                    M1AuthenticatedS1T128PrefillBootstrapErrorV1::LowerRejected,
                    (
                        runner,
                        memory,
                        cache,
                        prompt_tokens,
                        policy,
                        preparation_plans,
                        recipe_plans,
                        scheduled,
                        target_table,
                        draft_table,
                        target_rollover_pages,
                        error,
                    ),
                ));
            }
        }
    }
    let draft_rollover_page = match memory.lease_page(request, Qwen3ModelRole::Draft06B, page_count)
    {
        Ok(page) => page,
        Err(error) => {
            return Err(terminal_failure(
                engine,
                M1AuthenticatedS1T128PrefillBootstrapPhaseV1::RolloverPageLease,
                M1AuthenticatedS1T128PrefillBootstrapErrorV1::LowerRejected,
                (
                    runner,
                    memory,
                    cache,
                    prompt_tokens,
                    policy,
                    preparation_plans,
                    recipe_plans,
                    scheduled,
                    target_table,
                    draft_table,
                    target_rollover_pages,
                    error,
                ),
            ));
        }
    };
    let recipe = match recipe_plans.derive(
        runner.operations(),
        M1StepDispatchIntent::PairedPrefill(TARGET_PREFILL),
    ) {
        M1PhysicalRunnerRecipeOutcomeV1::Prepared(recipe) => recipe,
        M1PhysicalRunnerRecipeOutcomeV1::Rejected(error) => {
            return Err(terminal_failure(
                engine,
                M1AuthenticatedS1T128PrefillBootstrapPhaseV1::RecipeDerivation,
                M1AuthenticatedS1T128PrefillBootstrapErrorV1::LowerRejected,
                (
                    runner,
                    memory,
                    cache,
                    prompt_tokens,
                    policy,
                    preparation_plans,
                    scheduled,
                    target_table,
                    draft_table,
                    target_rollover_pages,
                    draft_rollover_page,
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
                M1AuthenticatedS1T128PrefillBootstrapPhaseV1::WorkspacePreparation,
                M1AuthenticatedS1T128PrefillBootstrapErrorV1::LowerRejected,
                (
                    runner,
                    memory,
                    cache,
                    prompt_tokens,
                    policy,
                    recipe,
                    target_rollover_pages,
                    draft_rollover_page,
                    error,
                ),
            ));
        }
    };
    let successor = match admitted_s1_t128_speculative_successor_v1(successor.target()) {
        Some(successor) => successor,
        None => {
            return Err(terminal_failure(
                engine,
                M1AuthenticatedS1T128PrefillBootstrapPhaseV1::RolloverIntentBinding,
                M1AuthenticatedS1T128PrefillBootstrapErrorV1::LowerRejected,
                (
                    runner,
                    memory,
                    cache,
                    prompt_tokens,
                    policy,
                    recipe,
                    target_rollover_pages,
                    draft_rollover_page,
                    prepared,
                    successor,
                ),
            ));
        }
    };
    let intent_prepared = match bind_m1_authenticated_speculative_rollover_intent_v1(
        prepared,
        successor,
        vec![M1AuthenticatedSpeculativeRolloverMemberIntentV1::new(
            request, policy,
        )],
    ) {
        Ok(prepared) => prepared,
        Err(error) => {
            return Err(terminal_failure(
                engine,
                M1AuthenticatedS1T128PrefillBootstrapPhaseV1::RolloverIntentBinding,
                M1AuthenticatedS1T128PrefillBootstrapErrorV1::LowerRejected,
                (
                    runner,
                    memory,
                    cache,
                    prompt_tokens,
                    policy,
                    recipe,
                    target_rollover_pages,
                    draft_rollover_page,
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
                M1AuthenticatedS1T128PrefillBootstrapPhaseV1::WorkspaceAllocation,
                M1AuthenticatedS1T128PrefillBootstrapErrorV1::LowerRejected,
                (
                    runner,
                    cache,
                    prompt_tokens,
                    policy,
                    recipe,
                    target_rollover_pages,
                    draft_rollover_page,
                    rollover_intent,
                    error,
                ),
            ));
        }
    };
    let reserve_output = if successor.target() == TARGET_SUCCESSOR {
        allocated
            .reserve_s1_k4_rollover_output()
            .map_err(|error| Box::new(error) as Box<dyn fmt::Debug>)
    } else {
        allocated
            .reserve_finite_speculative_rollover_output(successor.target())
            .map_err(|error| Box::new(error) as Box<dyn fmt::Debug>)
    };
    if let Err(error) = reserve_output {
        return Err(terminal_failure(
            engine,
            M1AuthenticatedS1T128PrefillBootstrapPhaseV1::RolloverOutputReservation,
            M1AuthenticatedS1T128PrefillBootstrapErrorV1::LowerRejected,
            (
                runner,
                allocated,
                cache,
                prompt_tokens,
                policy,
                recipe,
                target_rollover_pages,
                draft_rollover_page,
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
                M1AuthenticatedS1T128PrefillBootstrapPhaseV1::CompletionOutputAllocation,
                M1AuthenticatedS1T128PrefillBootstrapErrorV1::LowerRejected,
                (
                    runner,
                    allocated,
                    cache,
                    prompt_tokens,
                    policy,
                    recipe,
                    target_rollover_pages,
                    draft_rollover_page,
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
                M1AuthenticatedS1T128PrefillBootstrapPhaseV1::DiagnosticCapture,
                M1AuthenticatedS1T128PrefillBootstrapErrorV1::LowerRejected,
                (
                    runner,
                    allocated,
                    cache,
                    prompt_tokens,
                    policy,
                    recipe,
                    target_rollover_pages,
                    draft_rollover_page,
                    rollover_intent,
                    error,
                ),
            ));
        }
    };
    let prepublication_allocation_count =
        allocated.partitioned_memory().retained_allocation_count();
    if prepublication_allocation_count != EXPECTED_PREFILL_QUEUE_DATA_ALLOCATIONS
        || prepublication_allocation_count > GFX942_MAX_FIXED_DISPATCH_DATA_V1
    {
        return Err(terminal_failure(
            engine,
            M1AuthenticatedS1T128PrefillBootstrapPhaseV1::PrepublicationAllocationRoster,
            M1AuthenticatedS1T128PrefillBootstrapErrorV1::FixedDispatchDataRosterMismatch {
                expected: EXPECTED_PREFILL_QUEUE_DATA_ALLOCATIONS,
                actual: prepublication_allocation_count,
                maximum: GFX942_MAX_FIXED_DISPATCH_DATA_V1,
            },
            (
                runner,
                allocated,
                cache,
                prompt_tokens,
                policy,
                recipe,
                target_rollover_pages,
                draft_rollover_page,
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
                M1AuthenticatedS1T128PrefillBootstrapPhaseV1::AuthenticatedPrepublication,
                M1AuthenticatedS1T128PrefillBootstrapErrorV1::LowerRejected,
                (
                    cache,
                    prompt_tokens,
                    policy,
                    target_rollover_pages,
                    draft_rollover_page,
                    rollover_intent,
                    error,
                ),
            ));
        }
    };
    Ok(M1AuthenticatedS1T128PrefillPrepublicationV1 {
        engine,
        prepublication,
        cache,
        draft_rollover_page,
        target_rollover_pages,
        rollover_intent,
        request,
        prompt_tokens,
        policy,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferric_build::{
        m1_step_workspace_requirements, plan_addressless_m1_step_workspace,
        AddresslessM1StepWorkspacePlan, AvailableM1StepWorkspace,
        DeclaredM1StepWorkspaceAllocation, M1StepWorkspaceDeclaration, M1StepWorkspacePlanOutcome,
    };
    use ferric_spec::Identity;
    use std::cell::Cell;
    use std::rc::Rc;

    #[derive(Debug)]
    struct DropWitness(Rc<Cell<bool>>);

    impl Drop for DropWitness {
        fn drop(&mut self) {
            self.0.set(true);
        }
    }

    fn plan(selection: Qwen3PlanSelection, byte: u8) -> AddresslessM1StepWorkspacePlan {
        let requirements = m1_step_workspace_requirements(selection).unwrap();
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
            M1StepWorkspacePlanOutcome::Rejected(_) => panic!("test workspace rejected"),
        }
    }

    fn plans() -> (M1FullStepWorkspacePlans, M1FullStepWorkspacePlans) {
        (
            M1FullStepWorkspacePlans::paired_prefill(
                plan(DRAFT_PREFILL, 1),
                plan(TARGET_PREFILL, 2),
            ),
            M1FullStepWorkspacePlans::paired_prefill(
                plan(DRAFT_PREFILL, 1),
                plan(TARGET_PREFILL, 2),
            ),
        )
    }

    #[test]
    fn exact_profile_accepts_full_width_prompt_and_successor_output() {
        let (preparation, recipe) = plans();
        let input = M1AuthenticatedS1T128PrefillBootstrapInputV1::new(
            vec![1; 128],
            32,
            preparation,
            recipe,
        )
        .unwrap();
        assert_eq!(input.prompt_tokens(), [1; 128]);
        assert_eq!(input.maximum_successor_output_tokens(), 32);
        assert_eq!(input.successor_plan().target(), TARGET_SUCCESSOR);
    }

    #[test]
    fn singleton_successor_constructor_binds_each_finite_plan_and_tail() {
        for (bucket, tail_pages) in [
            (Qwen3PlanBucket::SpeculativeS1K4C8192, 1),
            (Qwen3PlanBucket::SpeculativeS1K8C8192, 1),
            (Qwen3PlanBucket::SpeculativeS1K16C8192, 2),
        ] {
            let target = Qwen3PlanSelection {
                bucket,
                ..TARGET_SUCCESSOR
            };
            for limit in [1, 3, 127] {
                let (preparation, recipe) = plans();
                let input =
                    M1AuthenticatedS1T128PrefillBootstrapInputV1::new_with_speculative_successor(
                        target,
                        vec![7; 128],
                        limit,
                        preparation,
                        recipe,
                    )
                    .unwrap();
                assert_eq!(input.successor_plan().target(), target);
                assert_eq!(input.successor_plan().draft(), DRAFT_SUCCESSOR);
                assert_eq!(input.maximum_successor_output_tokens(), limit);
                let expected = if limit == 127 { 8 } else { tail_pages };
                assert_eq!(
                    s1_t128_successor_target_tail_pages_v1(input.successor_plan(), limit),
                    Some(expected)
                );
            }
        }
    }

    #[test]
    fn singleton_successor_rejects_role_mode_and_unsupported_bucket_without_losing_inputs() {
        for target in [
            Qwen3PlanSelection {
                role: Qwen3ModelRole::Draft06B,
                ..TARGET_SUCCESSOR
            },
            Qwen3PlanSelection {
                mode: Qwen3ExecutionMode::Decode,
                ..TARGET_SUCCESSOR
            },
            Qwen3PlanSelection {
                bucket: Qwen3PlanBucket::SpeculativeS8K4C8192,
                ..TARGET_SUCCESSOR
            },
            TARGET_PREFILL,
        ] {
            let (preparation, recipe) = plans();
            let failure =
                M1AuthenticatedS1T128PrefillBootstrapInputV1::new_with_speculative_successor(
                    target,
                    vec![7; 128],
                    3,
                    preparation,
                    recipe,
                )
                .unwrap_err();
            assert_eq!(
                failure.error(),
                M1AuthenticatedS1T128PrefillBootstrapInputErrorV1::UnsupportedSuccessor {
                    actual: target
                }
            );
            let (prompt, limit, preparation, recipe) = failure.into_parts();
            assert_eq!(prompt, vec![7; 128]);
            assert_eq!(limit, 3);
            assert_eq!(preparation, recipe);
        }
    }

    #[test]
    fn singleton_successor_context_bound_includes_direct_prefill_choice() {
        let target = Qwen3PlanSelection {
            bucket: Qwen3PlanBucket::SpeculativeS1K16C8192,
            ..TARGET_SUCCESSOR
        };
        for (limit, accepted) in [(8_063, true), (8_064, false), (u32::MAX, false)] {
            let (preparation, recipe) = plans();
            let result =
                M1AuthenticatedS1T128PrefillBootstrapInputV1::new_with_speculative_successor(
                    target,
                    vec![7; 128],
                    limit,
                    preparation,
                    recipe,
                );
            assert_eq!(result.is_ok(), accepted);
        }
    }

    #[test]
    fn fixed_s1_k4_prepublication_allocation_roster_fits_kfd_limit() {
        let model_memory = 4;
        let paired_workspaces = 2;
        let s1_k4_successor_output = 3;
        let active_compact_output = 1;
        let direct_choice_capture = 1;
        assert_eq!(
            EXPECTED_PREFILL_QUEUE_DATA_ALLOCATIONS,
            model_memory
                + paired_workspaces
                + s1_k4_successor_output
                + active_compact_output
                + direct_choice_capture
        );
        assert_eq!(EXPECTED_PREFILL_QUEUE_DATA_ALLOCATIONS, 11);
        const {
            assert!(EXPECTED_PREFILL_QUEUE_DATA_ALLOCATIONS <= GFX942_MAX_FIXED_DISPATCH_DATA_V1);
        }
    }

    #[test]
    fn input_rejects_empty_wide_invalid_and_over_context_prompts() {
        for (prompt, output, expected) in [
            (
                Vec::new(),
                2,
                M1AuthenticatedS1T128PrefillBootstrapInputErrorV1::PromptLength {
                    required: 128,
                    actual: 0,
                },
            ),
            (
                vec![1; 129],
                2,
                M1AuthenticatedS1T128PrefillBootstrapInputErrorV1::PromptLength {
                    required: 128,
                    actual: 129,
                },
            ),
            (
                {
                    let mut prompt = vec![1; 128];
                    prompt[0] = QWEN3_VOCABULARY_SIZE;
                    prompt
                },
                2,
                M1AuthenticatedS1T128PrefillBootstrapInputErrorV1::TokenOutOfRange {
                    column: 0,
                    token: QWEN3_VOCABULARY_SIZE,
                },
            ),
            (
                vec![1; 128],
                0,
                M1AuthenticatedS1T128PrefillBootstrapInputErrorV1::InvalidOutputLimit { actual: 0 },
            ),
            (
                vec![1; 128],
                M1_MAX_CONTEXT_TOKENS - 128,
                M1AuthenticatedS1T128PrefillBootstrapInputErrorV1::ContextExceeded {
                    prompt: 128,
                    output: M1_MAX_CONTEXT_TOKENS - 128,
                },
            ),
        ] {
            let (preparation, recipe) = plans();
            let failure = M1AuthenticatedS1T128PrefillBootstrapInputV1::new(
                prompt,
                output,
                preparation,
                recipe,
            )
            .unwrap_err();
            assert_eq!(failure.error(), expected);
            let (_prompt, retained_output, _preparation, _recipe) = failure.into_parts();
            assert_eq!(retained_output, output);
        }
    }

    #[test]
    fn input_rejects_non_prefill_workspace_shapes_without_losing_them() {
        let preparation = M1FullStepWorkspacePlans::target_only(plan(TARGET_PREFILL, 1));
        let (_, recipe) = plans();
        let failure =
            M1AuthenticatedS1T128PrefillBootstrapInputV1::new(vec![1; 128], 2, preparation, recipe)
                .unwrap_err();
        assert_eq!(
            failure.error(),
            M1AuthenticatedS1T128PrefillBootstrapInputErrorV1::WorkspaceShape
        );
        let (_, _, preparation, recipe) = failure.into_parts();
        assert_eq!(preparation.kind(), M1FullStepWorkspaceInputKind::TargetOnly);
        assert_eq!(recipe.kind(), M1FullStepWorkspaceInputKind::PairedPrefill);
    }

    #[test]
    fn phase_local_failure_quarantines_engine_and_retains_lower_custody() {
        let dropped = Rc::new(Cell::new(false));
        let failure = terminal_failure(
            Engine::<1>::new(512, 256, M1_MAX_CONTEXT_TOKENS).unwrap(),
            M1AuthenticatedS1T128PrefillBootstrapPhaseV1::WorkspaceAllocation,
            M1AuthenticatedS1T128PrefillBootstrapErrorV1::LowerRejected,
            DropWitness(Rc::clone(&dropped)),
        );
        assert_eq!(
            failure.phase(),
            M1AuthenticatedS1T128PrefillBootstrapPhaseV1::WorkspaceAllocation
        );
        assert_eq!(
            failure.error(),
            M1AuthenticatedS1T128PrefillBootstrapErrorV1::LowerRejected
        );
        assert!(failure.engine_quarantined());
        assert!(!dropped.get());

        let (engine, phase, error, retained) = (*failure).into_parts();
        assert!(engine.is_faulted());
        assert_eq!(
            phase,
            M1AuthenticatedS1T128PrefillBootstrapPhaseV1::WorkspaceAllocation
        );
        assert_eq!(
            error,
            M1AuthenticatedS1T128PrefillBootstrapErrorV1::LowerRejected
        );
        assert!(!dropped.get());
        drop(retained);
        assert!(dropped.get());
    }
}
