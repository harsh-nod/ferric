//! Bounded authenticated S1/T128 target-only window execution.
//!
//! This module owns the causal join between an authenticated paired-prefill
//! input, the serving registry, checked device tokens, and terminal queue
//! settlement for the first one-request R33 service slice.

use core::fmt;
use std::any::Any;
use std::collections::VecDeque;

use ferric_spec::{
    validate_m1_step_inputs, M1StepInputCandidate, M1StepInputValidationOutcome,
    Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection, RequestId, TokenId,
    M1_KV_PAGE_TOKENS, M1_MAX_CONTEXT_TOKENS,
};
use rustix::time::{clock_gettime, ClockId};

use crate::{
    execute_m1_authenticated_s1_t128_paired_prefill_v1,
    prepare_m1_authenticated_long_lived_queue_rearm_v1,
    reserve_m1_authenticated_long_lived_queue_rearm_kv_v1,
    run_m1_authenticated_target_decode_serving_v1,
    submit_m1_authenticated_long_lived_queue_rearm_v1, CompletionWireSemanticExpectation,
    DeviceKvPageLease, Engine, M1AuthenticatedLongLivedQueueRearmScheduleFailureV1,
    M1AuthenticatedLongLivedQueueReleasedRoundV1, M1AuthenticatedRearmedRoundReleaseOutcomeV1,
    M1AuthenticatedReleasedCompletedStepV1, M1AuthenticatedS1T128PrefillPrepublicationV1,
    M1AuthenticatedTargetDecodeScheduleFailureV1, M1AuthenticatedTargetDecodeServingFailureV1,
    M1AuthenticatedTargetDecodeServingInputsV1, M1DeviceKvCompletionDispositionV1,
    M1FullStepWorkspaceInputKind, M1FullStepWorkspacePlans, M1LongLivedQueueRearmKvInputsV1,
    M1PhysicalRunnerRecipeOutcomeV1, M1ServingCompletionDispositionV1, M1ServingPlanV1,
    M1ServingRegistryV1,
};

const PROMPT_TOKENS: u32 = 128;
const PREFILL_TARGET_PAGES: u32 = PROMPT_TOKENS.div_ceil(M1_KV_PAGE_TOKENS);

/// Stable rejection before any authenticated execution owner is consumed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedTargetWindowInputErrorV1 {
    OutputTokenCount { actual: u32 },
    ContextExceeded { output_tokens: u32 },
    PlanCount { expected: usize, actual: usize },
    WorkspaceShape { round: usize },
    TargetPageCount { expected: usize, actual: usize },
}

/// Immutable bounds for one exact authenticated target-only service window.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1AuthenticatedTargetWindowBoundsV1 {
    output_tokens: u32,
    target_page_count: u32,
}

impl M1AuthenticatedTargetWindowBoundsV1 {
    /// The paired-prefill choice counts as the first output token.
    pub fn new(output_tokens: u32) -> Result<Self, M1AuthenticatedTargetWindowInputErrorV1> {
        if output_tokens < 2 {
            return Err(M1AuthenticatedTargetWindowInputErrorV1::OutputTokenCount {
                actual: output_tokens,
            });
        }
        // The prefill choice is the first decode anchor, not a KV write.
        let total_resident_tokens = PROMPT_TOKENS
            .checked_add(output_tokens - 1)
            .ok_or(M1AuthenticatedTargetWindowInputErrorV1::ContextExceeded { output_tokens })?;
        if total_resident_tokens > M1_MAX_CONTEXT_TOKENS {
            return Err(M1AuthenticatedTargetWindowInputErrorV1::ContextExceeded { output_tokens });
        }
        Ok(Self {
            output_tokens,
            target_page_count: total_resident_tokens.div_ceil(M1_KV_PAGE_TOKENS),
        })
    }

    #[must_use]
    pub const fn output_tokens(self) -> u32 {
        self.output_tokens
    }

    #[must_use]
    pub const fn successor_generations(self) -> u32 {
        self.output_tokens - 1
    }

    /// Total resident target pages, including the eight S1/T128 prefill pages.
    #[must_use]
    pub const fn target_page_count(self) -> u32 {
        self.target_page_count
    }

    /// Tail pages newly leased before model memory enters queue custody.
    #[must_use]
    pub const fn newly_leased_target_page_count(self) -> u32 {
        self.target_page_count - PREFILL_TARGET_PAGES
    }
}

/// Linear preparation and recipe workspace plans for one target generation.
#[must_use = "target-window round plans remain linear"]
#[derive(Debug, Eq, PartialEq)]
pub struct M1AuthenticatedTargetWindowRoundPlansV1 {
    preparation: M1FullStepWorkspacePlans,
    recipe: M1FullStepWorkspacePlans,
}

impl M1AuthenticatedTargetWindowRoundPlansV1 {
    /// Rejects non-target workspace shapes without consuming either owner.
    pub fn new(
        preparation: M1FullStepWorkspacePlans,
        recipe: M1FullStepWorkspacePlans,
    ) -> Result<Self, (M1FullStepWorkspacePlans, M1FullStepWorkspacePlans)> {
        if preparation.kind() != M1FullStepWorkspaceInputKind::TargetOnly
            || recipe.kind() != M1FullStepWorkspaceInputKind::TargetOnly
        {
            return Err((preparation, recipe));
        }
        Ok(Self {
            preparation,
            recipe,
        })
    }

    fn into_parts(self) -> (M1FullStepWorkspacePlans, M1FullStepWorkspacePlans) {
        (self.preparation, self.recipe)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedTargetWindowExecutionStageV1 {
    Input,
    ClockStart,
    RegistryAdmission,
    PrefillExecution,
    PrefillPublication,
    PrefillCompletion,
    DecodeEnqueue,
    DecodePublication,
    DecodeWait,
    DecodeRecycle,
    DecodeObservation,
    DecodeSemanticCheck,
    DecodeCompletion,
    DecodePageRelease,
    RegistryCompletion,
    EarlyStop,
    QueueTeardown,
    ClockFinish,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedTargetWindowExecutionErrorV1 {
    InvalidInput(M1AuthenticatedTargetWindowInputErrorV1),
    HostAllocation,
    Clock,
    Registry,
    Engine,
    Physical,
    Observation,
    Completion,
    EarlyStop,
    Teardown,
}

/// Opaque `CLOCK_MONOTONIC_RAW` boundary captured when a service request is accepted.
///
/// Capture this before authenticated bootstrap so TTFT includes every backend
/// effect performed after accepting the request.
#[must_use = "an accepted target-window clock boundary must be consumed"]
#[derive(Debug)]
pub struct M1AuthenticatedTargetWindowClockStartV1 {
    started_ns: u128,
}

impl M1AuthenticatedTargetWindowClockStartV1 {
    /// Captures the request-arrival boundary without acquiring execution authority.
    pub fn capture() -> Result<Self, M1AuthenticatedTargetWindowExecutionErrorV1> {
        monotonic_raw_ns()
            .map(|started_ns| Self { started_ns })
            .map_err(|()| M1AuthenticatedTargetWindowExecutionErrorV1::Clock)
    }
}

#[derive(Debug)]
struct OpaqueCustody(Box<dyn Any>);

#[must_use = "failed authenticated target-window custody must remain retained"]
pub struct M1AuthenticatedTargetWindowExecutionFailureV1 {
    stage: M1AuthenticatedTargetWindowExecutionStageV1,
    error: M1AuthenticatedTargetWindowExecutionErrorV1,
    retained: OpaqueCustody,
}

impl fmt::Debug for M1AuthenticatedTargetWindowExecutionFailureV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("M1AuthenticatedTargetWindowExecutionFailureV1")
            .field("stage", &self.stage)
            .field("error", &self.error)
            .field("retains_all_custody", &true)
            .finish()
    }
}

impl M1AuthenticatedTargetWindowExecutionFailureV1 {
    #[must_use]
    pub const fn stage(&self) -> M1AuthenticatedTargetWindowExecutionStageV1 {
        self.stage
    }

    #[must_use]
    pub const fn error(&self) -> M1AuthenticatedTargetWindowExecutionErrorV1 {
        self.error
    }

    #[must_use]
    pub fn retains_all_custody(&self) -> bool {
        let _ = &self.retained.0;
        true
    }
}

fn fail(
    stage: M1AuthenticatedTargetWindowExecutionStageV1,
    error: M1AuthenticatedTargetWindowExecutionErrorV1,
    retained: impl Any,
) -> Box<M1AuthenticatedTargetWindowExecutionFailureV1> {
    Box::new(M1AuthenticatedTargetWindowExecutionFailureV1 {
        stage,
        error,
        retained: OpaqueCustody(Box::new(retained)),
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1AuthenticatedTargetWindowTimingV1 {
    duration_ns: u64,
    first_token_offset_ns: u64,
    terminal_token_offset_ns: u64,
}

impl M1AuthenticatedTargetWindowTimingV1 {
    #[must_use]
    pub const fn duration_ns(self) -> u64 {
        self.duration_ns
    }

    #[must_use]
    pub const fn first_token_offset_ns(self) -> u64 {
        self.first_token_offset_ns
    }

    #[must_use]
    pub const fn terminal_token_offset_ns(self) -> u64 {
        self.terminal_token_offset_ns
    }
}

#[must_use = "successful authenticated target-window custody remains retained"]
pub struct M1AuthenticatedTargetWindowExecutionSuccessV1 {
    tokens: Box<[TokenId]>,
    timing: M1AuthenticatedTargetWindowTimingV1,
    retained: OpaqueCustody,
}

impl fmt::Debug for M1AuthenticatedTargetWindowExecutionSuccessV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("M1AuthenticatedTargetWindowExecutionSuccessV1")
            .field("tokens", &self.tokens)
            .field("timing", &self.timing)
            .field("retains_terminal_custody", &true)
            .finish()
    }
}

impl M1AuthenticatedTargetWindowExecutionSuccessV1 {
    #[must_use]
    pub fn tokens(&self) -> &[TokenId] {
        &self.tokens
    }

    #[must_use]
    pub const fn timing(&self) -> M1AuthenticatedTargetWindowTimingV1 {
        self.timing
    }

    #[must_use]
    pub fn retains_terminal_custody(&self) -> bool {
        let _ = &self.retained.0;
        true
    }
}

#[derive(Debug)]
enum ReleasedRound {
    Prefill(M1AuthenticatedReleasedCompletedStepV1),
    Rearmed(M1AuthenticatedLongLivedQueueReleasedRoundV1),
}

fn close_released<const C: usize>(
    engine: &mut Engine<C>,
    released: ReleasedRound,
) -> Box<dyn fmt::Debug> {
    match released {
        ReleasedRound::Prefill(released) => {
            Box::new(released.destroy_queue_and_retain_step(engine))
        }
        ReleasedRound::Rearmed(released) => {
            Box::new(released.destroy_queue_and_retain_round(engine))
        }
    }
}

const fn prefill_selection(role: Qwen3ModelRole) -> Qwen3PlanSelection {
    Qwen3PlanSelection {
        role,
        mode: Qwen3ExecutionMode::Prefill,
        bucket: Qwen3PlanBucket::PrefillS1T128,
    }
}

const fn target_selection(role: Qwen3ModelRole) -> Qwen3PlanSelection {
    Qwen3PlanSelection {
        role,
        mode: Qwen3ExecutionMode::Decode,
        bucket: Qwen3PlanBucket::DecodeS1C8192,
    }
}

fn prefill_serving_plan() -> Result<M1ServingPlanV1, crate::M1ServingRegistryErrorV1> {
    M1ServingPlanV1::new(
        prefill_selection(Qwen3ModelRole::Target8B),
        prefill_selection(Qwen3ModelRole::Draft06B),
    )
}

fn target_serving_plan() -> Result<M1ServingPlanV1, crate::M1ServingRegistryErrorV1> {
    M1ServingPlanV1::new(
        target_selection(Qwen3ModelRole::Target8B),
        target_selection(Qwen3ModelRole::Draft06B),
    )
}

fn monotonic_raw_ns() -> Result<u128, ()> {
    let timestamp = clock_gettime(ClockId::MonotonicRaw);
    let seconds = u128::try_from(timestamp.tv_sec).map_err(|_| ())?;
    let nanos = u128::try_from(timestamp.tv_nsec).map_err(|_| ())?;
    seconds
        .checked_mul(1_000_000_000)
        .and_then(|value| value.checked_add(nanos))
        .ok_or(())
}

fn elapsed_ns(started_ns: u128) -> Result<u64, ()> {
    u64::try_from(monotonic_raw_ns()?.checked_sub(started_ns).ok_or(())?).map_err(|_| ())
}

fn validated_target_input(
    plan: ferric_spec::StepPlan,
    input_token: TokenId,
    context: u32,
) -> Result<ferric_spec::ValidatedM1StepInputs, ()> {
    match validate_m1_step_inputs(M1StepInputCandidate::new(
        target_selection(Qwen3ModelRole::Target8B),
        vec![Some(plan)],
        vec![input_token],
        vec![context],
        vec![1],
        vec![context],
    )) {
        M1StepInputValidationOutcome::Validated(inputs) => Ok(inputs),
        M1StepInputValidationOutcome::Rejected(_) => Err(()),
    }
}

fn observed_token(image: &crate::M1ObservedCompletionImageV1) -> Result<TokenId, ()> {
    let [record] = image.records() else {
        return Err(());
    };
    let [token] = record.emitted_tokens() else {
        return Err(());
    };
    Ok(*token)
}

fn reserve_next<const C: usize>(
    registry: &mut M1ServingRegistryV1<C>,
) -> Result<crate::M1ServingPublicationReservationV1, crate::M1ServingRegistryErrorV1> {
    let batch = registry
        .plan_next()?
        .ok_or(crate::M1ServingRegistryErrorV1::RequestNotReady)?;
    registry.reserve_publication(batch)
}

#[allow(clippy::too_many_arguments)]
fn publish_target_round<const C: usize>(
    engine: &mut Engine<C>,
    released: ReleasedRound,
    batch: &crate::M1ServingBatchPlanV1,
    request: RequestId,
    anchor: TokenId,
    context: u32,
    page_leases: Vec<DeviceKvPageLease>,
    plans: M1AuthenticatedTargetWindowRoundPlansV1,
    diagnostic_ring_bytes: u32,
) -> Result<crate::M1AuthenticatedRearmedPublishedQueueV1, Box<dyn fmt::Debug>> {
    let (preparation_plans, recipe_plans) = plans.into_parts();
    match released {
        ReleasedRound::Prefill(released) => {
            let plan = match released.queue().operations().runner().bind_step_plan(
                request,
                batch.epoch(),
                target_selection(Qwen3ModelRole::Target8B),
            ) {
                Ok(plan) => plan,
                Err(error) => {
                    return Err(Box::new((
                        released,
                        page_leases,
                        preparation_plans,
                        recipe_plans,
                        error,
                    )));
                }
            };
            let target = match validated_target_input(plan, anchor, context) {
                Ok(target) => target,
                Err(()) => {
                    return Err(Box::new((
                        released,
                        page_leases,
                        preparation_plans,
                        recipe_plans,
                        "target input rejected",
                    )));
                }
            };
            let inputs = M1AuthenticatedTargetDecodeServingInputsV1::new(
                target,
                page_leases,
                preparation_plans,
                recipe_plans,
            );
            match run_m1_authenticated_target_decode_serving_v1(
                engine,
                released,
                batch,
                inputs,
                diagnostic_ring_bytes,
            ) {
                Ok(published) => Ok(published),
                Err(M1AuthenticatedTargetDecodeServingFailureV1::Schedule(
                    M1AuthenticatedTargetDecodeScheduleFailureV1::PreDetach { error, retry },
                )) => Err(Box::new((error, retry.cancel_and_close(engine)))),
                Err(error) => Err(Box::new(error)),
            }
        }
        ReleasedRound::Rearmed(released) => {
            let scheduled =
                match released.schedule_next_exact(engine, batch.epoch(), batch.requests()) {
                    Ok(scheduled) => scheduled,
                    Err(M1AuthenticatedLongLivedQueueRearmScheduleFailureV1::Rejected(
                        rejected,
                    )) => {
                        let (error, released) = rejected.into_parts();
                        return Err(Box::new((
                            error,
                            released.destroy_queue_and_retain_round(engine),
                        )));
                    }
                    Err(error) => return Err(Box::new(error)),
                };
            let plan = match scheduled.bind_selected_step_plan(request) {
                Ok(plan) => plan,
                Err(error) => {
                    return Err(Box::new((
                        scheduled,
                        page_leases,
                        preparation_plans,
                        recipe_plans,
                        error,
                    )));
                }
            };
            let target = match validated_target_input(plan, anchor, context) {
                Ok(target) => target,
                Err(()) => {
                    return Err(Box::new((
                        scheduled,
                        page_leases,
                        preparation_plans,
                        recipe_plans,
                        "target input rejected",
                    )));
                }
            };
            let recipe = match scheduled.derive_retained_step_recipe(recipe_plans) {
                M1PhysicalRunnerRecipeOutcomeV1::Prepared(recipe) => recipe,
                M1PhysicalRunnerRecipeOutcomeV1::Rejected(error) => {
                    return Err(Box::new((scheduled, page_leases, preparation_plans, error)));
                }
            };
            let reserved = match reserve_m1_authenticated_long_lived_queue_rearm_kv_v1(
                engine,
                scheduled,
                M1LongLivedQueueRearmKvInputsV1::target_only(target, vec![page_leases]),
            ) {
                Ok(reserved) => reserved,
                Err(error) => return Err(Box::new((error, preparation_plans, recipe))),
            };
            let prepared = match prepare_m1_authenticated_long_lived_queue_rearm_v1(
                engine,
                reserved,
                preparation_plans,
            ) {
                Ok(prepared) => prepared,
                Err(error) => return Err(Box::new((error, recipe))),
            };
            submit_m1_authenticated_long_lived_queue_rearm_v1(engine, prepared, recipe)
                .map_err(|error| Box::new(error) as Box<dyn fmt::Debug>)
        }
    }
}

struct SuccessorCustody {
    prefill_choices: crate::M1ObservedDirectDiagnosticChoicesV1,
    draft_rollover_page: DeviceKvPageLease,
    rollover_intent: crate::M1AuthenticatedSpeculativeRolloverIntentV1,
    prompt_tokens: Box<[TokenId]>,
    policy: crate::M1SpeculativeGenerationPolicyV1,
}

impl fmt::Debug for SuccessorCustody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SuccessorCustody")
            .field("prefill_choices", &self.prefill_choices)
            .field("draft_rollover_page", &self.draft_rollover_page)
            .field("rollover_intent", &self.rollover_intent)
            .field("prompt_tokens", &self.prompt_tokens)
            .field("policy", &self.policy)
            .finish()
    }
}

/// Executes an exact authenticated S1/T128 request through target-only decode.
///
/// Each logical batch is reserved before its physical publication. A successor
/// receives only the token copied from the immediately preceding authenticated
/// completion after its direct S1 semantic check. Success requires exact output
/// cardinality, Engine retirement, registry retirement, and terminal queue
/// teardown.
#[allow(clippy::too_many_lines)]
pub fn execute_m1_authenticated_s1_t128_target_window_v1<const C: usize>(
    clock_start: M1AuthenticatedTargetWindowClockStartV1,
    prepared: M1AuthenticatedS1T128PrefillPrepublicationV1<C>,
    round_plans: Vec<M1AuthenticatedTargetWindowRoundPlansV1>,
    diagnostic_ring_bytes: u32,
    queue_wait_timeout: crate::M1QueueWaitTimeoutV1,
) -> Result<
    M1AuthenticatedTargetWindowExecutionSuccessV1,
    Box<M1AuthenticatedTargetWindowExecutionFailureV1>,
> {
    let output_tokens = match prepared.maximum_successor_output_tokens().checked_add(1) {
        Some(output_tokens) => output_tokens,
        None => {
            return Err(fail(
                M1AuthenticatedTargetWindowExecutionStageV1::Input,
                M1AuthenticatedTargetWindowExecutionErrorV1::InvalidInput(
                    M1AuthenticatedTargetWindowInputErrorV1::ContextExceeded {
                        output_tokens: u32::MAX,
                    },
                ),
                (prepared, round_plans),
            ));
        }
    };
    let bounds = match M1AuthenticatedTargetWindowBoundsV1::new(output_tokens) {
        Ok(bounds) => bounds,
        Err(error) => {
            return Err(fail(
                M1AuthenticatedTargetWindowExecutionStageV1::Input,
                M1AuthenticatedTargetWindowExecutionErrorV1::InvalidInput(error),
                (prepared, round_plans),
            ));
        }
    };
    let expected_rounds = bounds.successor_generations() as usize;
    if round_plans.len() != expected_rounds {
        return Err(fail(
            M1AuthenticatedTargetWindowExecutionStageV1::Input,
            M1AuthenticatedTargetWindowExecutionErrorV1::InvalidInput(
                M1AuthenticatedTargetWindowInputErrorV1::PlanCount {
                    expected: expected_rounds,
                    actual: round_plans.len(),
                },
            ),
            (prepared, round_plans),
        ));
    }
    if let Some(round) = round_plans.iter().position(|plans| {
        plans.preparation.kind() != M1FullStepWorkspaceInputKind::TargetOnly
            || plans.recipe.kind() != M1FullStepWorkspaceInputKind::TargetOnly
    }) {
        return Err(fail(
            M1AuthenticatedTargetWindowExecutionStageV1::Input,
            M1AuthenticatedTargetWindowExecutionErrorV1::InvalidInput(
                M1AuthenticatedTargetWindowInputErrorV1::WorkspaceShape { round },
            ),
            (prepared, round_plans),
        ));
    }
    let expected_pages = bounds.newly_leased_target_page_count() as usize;
    if prepared.target_rollover_pages().len() != expected_pages {
        return Err(fail(
            M1AuthenticatedTargetWindowExecutionStageV1::Input,
            M1AuthenticatedTargetWindowExecutionErrorV1::InvalidInput(
                M1AuthenticatedTargetWindowInputErrorV1::TargetPageCount {
                    expected: expected_pages,
                    actual: prepared.target_rollover_pages().len(),
                },
            ),
            (prepared, round_plans),
        ));
    }
    let request = prepared.request();
    let prefill = match prefill_serving_plan() {
        Ok(plan) => plan,
        Err(error) => {
            return Err(fail(
                M1AuthenticatedTargetWindowExecutionStageV1::RegistryAdmission,
                M1AuthenticatedTargetWindowExecutionErrorV1::Registry,
                (prepared, round_plans, error),
            ));
        }
    };
    let target = match target_serving_plan() {
        Ok(plan) => plan,
        Err(error) => {
            return Err(fail(
                M1AuthenticatedTargetWindowExecutionStageV1::RegistryAdmission,
                M1AuthenticatedTargetWindowExecutionErrorV1::Registry,
                (prepared, round_plans, error),
            ));
        }
    };
    let mut registry = match M1ServingRegistryV1::<C>::new() {
        Ok(registry) => registry,
        Err(error) => {
            return Err(fail(
                M1AuthenticatedTargetWindowExecutionStageV1::RegistryAdmission,
                M1AuthenticatedTargetWindowExecutionErrorV1::Registry,
                (prepared, round_plans, error),
            ));
        }
    };
    if let Err(error) = registry.admit(request, prefill) {
        return Err(fail(
            M1AuthenticatedTargetWindowExecutionStageV1::RegistryAdmission,
            M1AuthenticatedTargetWindowExecutionErrorV1::Registry,
            (prepared, round_plans, registry, error),
        ));
    }
    let prefill_reservation = match reserve_next(&mut registry) {
        Ok(reservation) => reservation,
        Err(error) => {
            return Err(fail(
                M1AuthenticatedTargetWindowExecutionStageV1::RegistryAdmission,
                M1AuthenticatedTargetWindowExecutionErrorV1::Registry,
                (prepared, round_plans, registry, error),
            ));
        }
    };
    if let Err(error) = registry.preflight_publication(&prefill_reservation) {
        return Err(fail(
            M1AuthenticatedTargetWindowExecutionStageV1::RegistryAdmission,
            M1AuthenticatedTargetWindowExecutionErrorV1::Registry,
            (prepared, round_plans, registry, prefill_reservation, error),
        ));
    }
    let prefill_registry_identity = prefill_reservation.registry_identity();
    let prefill_epoch = prefill_reservation.epoch();
    let started_ns = clock_start.started_ns;
    let executed = match execute_m1_authenticated_s1_t128_paired_prefill_v1(
        prepared,
        diagnostic_ring_bytes,
        queue_wait_timeout,
    ) {
        Ok(executed) => executed,
        Err(source) => {
            let abort = registry.abort_publication(prefill_reservation);
            return Err(fail(
                M1AuthenticatedTargetWindowExecutionStageV1::PrefillExecution,
                M1AuthenticatedTargetWindowExecutionErrorV1::Physical,
                (source, round_plans, registry, abort),
            ));
        }
    };
    let first_token_offset_ns = match elapsed_ns(started_ns) {
        Ok(offset) => offset,
        Err(()) => {
            let (
                mut engine,
                released,
                token,
                choices,
                draft,
                pages,
                intent,
                request,
                prompt,
                policy,
                ring,
                timeout,
            ) = executed.into_parts();
            let teardown = released.destroy_queue_and_retain_step(&mut engine);
            return Err(fail(
                M1AuthenticatedTargetWindowExecutionStageV1::ClockStart,
                M1AuthenticatedTargetWindowExecutionErrorV1::Clock,
                (
                    (engine, teardown, token, choices, draft, pages, intent),
                    (
                        request,
                        prompt,
                        policy,
                        ring,
                        timeout,
                        round_plans,
                        registry,
                        prefill_reservation,
                    ),
                ),
            ));
        }
    };
    if let Err(error) = registry.record_publication(prefill_reservation) {
        let (
            mut engine,
            released,
            token,
            choices,
            draft,
            pages,
            intent,
            request,
            prompt,
            policy,
            ring,
            timeout,
        ) = executed.into_parts();
        let teardown = released.destroy_queue_and_retain_step(&mut engine);
        return Err(fail(
            M1AuthenticatedTargetWindowExecutionStageV1::PrefillPublication,
            M1AuthenticatedTargetWindowExecutionErrorV1::Registry,
            (
                (engine, teardown, token, choices, draft, pages, intent),
                (
                    request,
                    prompt,
                    policy,
                    ring,
                    timeout,
                    round_plans,
                    registry,
                    error,
                ),
            ),
        ));
    }
    let first_token = executed.first_token();
    let first_stops = executed
        .generation_policy()
        .stop_tokens()
        .contains(&first_token);
    let prefill_dispositions = if first_stops {
        [M1ServingCompletionDispositionV1::Retire]
    } else {
        [M1ServingCompletionDispositionV1::Continue(target)]
    };
    if let Err(error) = registry.preflight_completion_exact_for(
        prefill_registry_identity,
        prefill_epoch,
        &prefill_dispositions,
    ) {
        let (
            mut engine,
            released,
            token,
            choices,
            draft,
            pages,
            intent,
            request,
            prompt,
            policy,
            ring,
            timeout,
        ) = executed.into_parts();
        let teardown = released.destroy_queue_and_retain_step(&mut engine);
        return Err(fail(
            M1AuthenticatedTargetWindowExecutionStageV1::PrefillCompletion,
            M1AuthenticatedTargetWindowExecutionErrorV1::Registry,
            (
                (engine, teardown, token, choices, draft, pages, intent),
                (
                    request,
                    prompt,
                    policy,
                    ring,
                    timeout,
                    round_plans,
                    registry,
                    error,
                ),
            ),
        ));
    }
    registry.apply_preflighted_completion(prefill_epoch, &prefill_dispositions);
    let (
        mut engine,
        released,
        first_token,
        prefill_choices,
        draft_rollover_page,
        pages,
        rollover_intent,
        request,
        prompt_tokens,
        policy,
        _,
        _,
    ) = executed.into_parts();
    let successor_custody = SuccessorCustody {
        prefill_choices,
        draft_rollover_page,
        rollover_intent,
        prompt_tokens,
        policy,
    };
    let mut tokens = Vec::new();
    if tokens
        .try_reserve_exact(bounds.output_tokens() as usize)
        .is_err()
    {
        let teardown = released.destroy_queue_and_retain_step(&mut engine);
        return Err(fail(
            M1AuthenticatedTargetWindowExecutionStageV1::PrefillCompletion,
            M1AuthenticatedTargetWindowExecutionErrorV1::HostAllocation,
            (
                engine,
                teardown,
                pages,
                successor_custody,
                round_plans,
                registry,
            ),
        ));
    }
    tokens.push(first_token);
    if first_stops {
        let retirement = engine.retire(request);
        let teardown = released.destroy_queue_and_retain_step(&mut engine);
        return Err(fail(
            M1AuthenticatedTargetWindowExecutionStageV1::EarlyStop,
            M1AuthenticatedTargetWindowExecutionErrorV1::EarlyStop,
            (
                engine,
                teardown,
                retirement,
                tokens,
                pages,
                successor_custody,
                round_plans,
                registry,
            ),
        ));
    }
    let mut released = ReleasedRound::Prefill(released);
    let mut plans = round_plans.into_iter();
    let mut target_pages = VecDeque::from(pages);
    let mut terminal_token_offset_ns = first_token_offset_ns;

    for successor in 0..bounds.successor_generations() {
        if let Err(error) = engine.append_tentative(request, 1) {
            let closed = close_released(&mut engine, released);
            return Err(fail(
                M1AuthenticatedTargetWindowExecutionStageV1::DecodeEnqueue,
                M1AuthenticatedTargetWindowExecutionErrorV1::Engine,
                (
                    engine,
                    closed,
                    error,
                    tokens,
                    plans,
                    target_pages,
                    successor_custody,
                    registry,
                ),
            ));
        }
        let reservation = match reserve_next(&mut registry) {
            Ok(reservation) => reservation,
            Err(error) => {
                let closed = close_released(&mut engine, released);
                return Err(fail(
                    M1AuthenticatedTargetWindowExecutionStageV1::DecodePublication,
                    M1AuthenticatedTargetWindowExecutionErrorV1::Registry,
                    (
                        engine,
                        closed,
                        error,
                        tokens,
                        plans,
                        target_pages,
                        successor_custody,
                        registry,
                    ),
                ));
            }
        };
        if let Err(error) = registry.preflight_publication(&reservation) {
            let closed = close_released(&mut engine, released);
            return Err(fail(
                M1AuthenticatedTargetWindowExecutionStageV1::DecodePublication,
                M1AuthenticatedTargetWindowExecutionErrorV1::Registry,
                (
                    engine,
                    closed,
                    error,
                    reservation,
                    tokens,
                    plans,
                    target_pages,
                    successor_custody,
                    registry,
                ),
            ));
        }
        let batch = reservation.physical_batch();
        let registry_identity = reservation.registry_identity();
        let epoch = reservation.epoch();
        let Some(round_plans) = plans.next() else {
            let abort = registry.abort_publication(reservation);
            let closed = close_released(&mut engine, released);
            return Err(fail(
                M1AuthenticatedTargetWindowExecutionStageV1::Input,
                M1AuthenticatedTargetWindowExecutionErrorV1::InvalidInput(
                    M1AuthenticatedTargetWindowInputErrorV1::PlanCount {
                        expected: expected_rounds,
                        actual: successor as usize,
                    },
                ),
                (
                    engine,
                    closed,
                    abort,
                    tokens,
                    plans,
                    target_pages,
                    successor_custody,
                    registry,
                ),
            ));
        };
        let context = PROMPT_TOKENS + successor;
        let page_leases = if context.is_multiple_of(M1_KV_PAGE_TOKENS) {
            let Some(page) = target_pages.pop_front() else {
                let abort = registry.abort_publication(reservation);
                let closed = close_released(&mut engine, released);
                return Err(fail(
                    M1AuthenticatedTargetWindowExecutionStageV1::Input,
                    M1AuthenticatedTargetWindowExecutionErrorV1::InvalidInput(
                        M1AuthenticatedTargetWindowInputErrorV1::TargetPageCount {
                            expected: expected_pages,
                            actual: expected_pages.saturating_sub(target_pages.len()),
                        },
                    ),
                    (
                        engine,
                        closed,
                        abort,
                        round_plans,
                        tokens,
                        plans,
                        target_pages,
                        successor_custody,
                        registry,
                    ),
                ));
            };
            vec![page]
        } else {
            Vec::new()
        };
        let Some(anchor) = tokens.last().copied() else {
            let abort = registry.abort_publication(reservation);
            let closed = close_released(&mut engine, released);
            return Err(fail(
                M1AuthenticatedTargetWindowExecutionStageV1::DecodePublication,
                M1AuthenticatedTargetWindowExecutionErrorV1::Observation,
                (
                    engine,
                    closed,
                    abort,
                    round_plans,
                    tokens,
                    plans,
                    target_pages,
                    successor_custody,
                    registry,
                ),
            ));
        };
        let published = match publish_target_round(
            &mut engine,
            released,
            &batch,
            request,
            anchor,
            context,
            page_leases,
            round_plans,
            diagnostic_ring_bytes,
        ) {
            Ok(published) => published,
            Err(error) => {
                let abort = registry.abort_publication(reservation);
                return Err(fail(
                    M1AuthenticatedTargetWindowExecutionStageV1::DecodePublication,
                    M1AuthenticatedTargetWindowExecutionErrorV1::Physical,
                    (
                        engine,
                        error,
                        abort,
                        tokens,
                        plans,
                        target_pages,
                        successor_custody,
                        registry,
                    ),
                ));
            }
        };
        if let Err(error) = registry.record_publication(reservation) {
            engine.quarantine_m1_queue_rearm_failure();
            return Err(fail(
                M1AuthenticatedTargetWindowExecutionStageV1::DecodePublication,
                M1AuthenticatedTargetWindowExecutionErrorV1::Registry,
                (
                    engine,
                    published,
                    error,
                    tokens,
                    plans,
                    target_pages,
                    successor_custody,
                    registry,
                ),
            ));
        }
        let completed = match published.wait_for(queue_wait_timeout.milliseconds(), &mut engine) {
            Ok(completed) => completed,
            Err(error) => {
                return Err(fail(
                    M1AuthenticatedTargetWindowExecutionStageV1::DecodeWait,
                    M1AuthenticatedTargetWindowExecutionErrorV1::Physical,
                    (
                        engine,
                        error,
                        tokens,
                        plans,
                        target_pages,
                        successor_custody,
                        registry,
                    ),
                ));
            }
        };
        let recycled = match completed.recycle(&mut engine) {
            Ok(recycled) => recycled,
            Err(error) => {
                return Err(fail(
                    M1AuthenticatedTargetWindowExecutionStageV1::DecodeRecycle,
                    M1AuthenticatedTargetWindowExecutionErrorV1::Physical,
                    (
                        engine,
                        error,
                        tokens,
                        plans,
                        target_pages,
                        successor_custody,
                        registry,
                    ),
                ));
            }
        };
        let observed = match recycled.observe_completion() {
            Ok(observed) => observed,
            Err(error) => match error.retry_observation() {
                Ok(observed) => observed,
                Err(error) => {
                    let teardown = error.destroy_queue_and_retain_custody(&mut engine);
                    return Err(fail(
                        M1AuthenticatedTargetWindowExecutionStageV1::DecodeObservation,
                        M1AuthenticatedTargetWindowExecutionErrorV1::Observation,
                        (
                            engine,
                            teardown,
                            tokens,
                            plans,
                            target_pages,
                            successor_custody,
                            registry,
                        ),
                    ));
                }
            },
        };
        let emitted = match observed_token(observed.image()) {
            Ok(emitted) => emitted,
            Err(()) => {
                engine.quarantine_m1_queue_rearm_failure();
                return Err(fail(
                    M1AuthenticatedTargetWindowExecutionStageV1::DecodeObservation,
                    M1AuthenticatedTargetWindowExecutionErrorV1::Observation,
                    (
                        engine,
                        observed,
                        tokens,
                        plans,
                        target_pages,
                        successor_custody,
                        registry,
                    ),
                ));
            }
        };
        let semantic = [CompletionWireSemanticExpectation::DirectFinalRow { choice: emitted }];
        let readback = match observed.check_completion(&semantic) {
            Ok(readback) => readback,
            Err(error) => {
                let teardown = error.destroy_queue_and_retain_custody(&mut engine);
                return Err(fail(
                    M1AuthenticatedTargetWindowExecutionStageV1::DecodeSemanticCheck,
                    M1AuthenticatedTargetWindowExecutionErrorV1::Observation,
                    (
                        engine,
                        teardown,
                        tokens,
                        plans,
                        target_pages,
                        successor_custody,
                        registry,
                    ),
                ));
            }
        };
        tokens.push(emitted);
        terminal_token_offset_ns = match elapsed_ns(started_ns) {
            Ok(offset) => offset,
            Err(()) => {
                let teardown = readback.destroy_queue_and_retain_custody(&mut engine);
                return Err(fail(
                    M1AuthenticatedTargetWindowExecutionStageV1::ClockFinish,
                    M1AuthenticatedTargetWindowExecutionErrorV1::Clock,
                    (
                        engine,
                        teardown,
                        tokens,
                        plans,
                        target_pages,
                        successor_custody,
                        registry,
                    ),
                ));
            }
        };
        let final_round = successor + 1 == bounds.successor_generations();
        let early_stop = !final_round && successor_custody.policy.stop_tokens().contains(&emitted);
        let retiring = final_round || early_stop;
        let registry_dispositions = if retiring {
            [M1ServingCompletionDispositionV1::Retire]
        } else {
            [M1ServingCompletionDispositionV1::Continue(target)]
        };
        if let Err(error) = registry.preflight_completion_exact_for(
            registry_identity,
            epoch,
            &registry_dispositions,
        ) {
            let teardown = readback.destroy_queue_and_retain_custody(&mut engine);
            return Err(fail(
                M1AuthenticatedTargetWindowExecutionStageV1::RegistryCompletion,
                M1AuthenticatedTargetWindowExecutionErrorV1::Registry,
                (
                    engine,
                    teardown,
                    error,
                    tokens,
                    plans,
                    target_pages,
                    successor_custody,
                    registry,
                ),
            ));
        }
        if retiring {
            if let Err(error) = engine.retire(request) {
                let teardown = readback.destroy_queue_and_retain_custody(&mut engine);
                return Err(fail(
                    M1AuthenticatedTargetWindowExecutionStageV1::DecodeCompletion,
                    M1AuthenticatedTargetWindowExecutionErrorV1::Engine,
                    (
                        engine,
                        teardown,
                        error,
                        tokens,
                        plans,
                        target_pages,
                        successor_custody,
                        registry,
                    ),
                ));
            }
        }
        let physical_disposition = if retiring {
            M1DeviceKvCompletionDispositionV1::Retire
        } else {
            M1DeviceKvCompletionDispositionV1::Continue
        };
        let completion = match readback.complete(&mut engine, vec![physical_disposition]) {
            Ok(completion) => completion,
            Err(error) => {
                return Err(fail(
                    M1AuthenticatedTargetWindowExecutionStageV1::DecodeCompletion,
                    M1AuthenticatedTargetWindowExecutionErrorV1::Completion,
                    (
                        engine,
                        error,
                        tokens,
                        plans,
                        target_pages,
                        successor_custody,
                        registry,
                    ),
                ));
            }
        };
        let next_released = match completion.release_completed() {
            M1AuthenticatedRearmedRoundReleaseOutcomeV1::Released(released) => released,
            error => {
                return Err(fail(
                    M1AuthenticatedTargetWindowExecutionStageV1::DecodePageRelease,
                    M1AuthenticatedTargetWindowExecutionErrorV1::Completion,
                    (
                        engine,
                        error,
                        tokens,
                        plans,
                        target_pages,
                        successor_custody,
                        registry,
                    ),
                ));
            }
        };
        registry.apply_preflighted_completion(epoch, &registry_dispositions);
        if early_stop {
            let teardown = next_released.shutdown_all_terminal_queue(&mut engine);
            return Err(fail(
                M1AuthenticatedTargetWindowExecutionStageV1::EarlyStop,
                M1AuthenticatedTargetWindowExecutionErrorV1::EarlyStop,
                (
                    engine,
                    teardown,
                    tokens,
                    plans,
                    target_pages,
                    successor_custody,
                    registry,
                ),
            ));
        }
        released = ReleasedRound::Rearmed(next_released);
    }

    if !target_pages.is_empty() {
        let closed = close_released(&mut engine, released);
        return Err(fail(
            M1AuthenticatedTargetWindowExecutionStageV1::Input,
            M1AuthenticatedTargetWindowExecutionErrorV1::InvalidInput(
                M1AuthenticatedTargetWindowInputErrorV1::TargetPageCount {
                    expected: expected_pages,
                    actual: expected_pages.saturating_sub(target_pages.len()),
                },
            ),
            (
                engine,
                closed,
                tokens,
                plans,
                target_pages,
                successor_custody,
                registry,
            ),
        ));
    }
    let released = match released {
        ReleasedRound::Rearmed(released) => released,
        ReleasedRound::Prefill(released) => {
            let closed = released.destroy_queue_and_retain_step(&mut engine);
            return Err(fail(
                M1AuthenticatedTargetWindowExecutionStageV1::QueueTeardown,
                M1AuthenticatedTargetWindowExecutionErrorV1::Teardown,
                (engine, closed, tokens, plans, successor_custody, registry),
            ));
        }
    };
    let teardown = match released.shutdown_all_terminal_queue(&mut engine) {
        Ok(teardown) => teardown,
        Err(error) => {
            return Err(fail(
                M1AuthenticatedTargetWindowExecutionStageV1::QueueTeardown,
                M1AuthenticatedTargetWindowExecutionErrorV1::Teardown,
                (engine, error, tokens, plans, successor_custody, registry),
            ));
        }
    };
    if let Err(error) = registry.remove_retired(request) {
        return Err(fail(
            M1AuthenticatedTargetWindowExecutionStageV1::RegistryCompletion,
            M1AuthenticatedTargetWindowExecutionErrorV1::Registry,
            (
                engine,
                teardown,
                error,
                tokens,
                plans,
                successor_custody,
                registry,
            ),
        ));
    }
    let duration_ns = match elapsed_ns(started_ns) {
        Ok(duration_ns)
            if first_token_offset_ns > 0
                && terminal_token_offset_ns > first_token_offset_ns
                && duration_ns >= terminal_token_offset_ns
                && (terminal_token_offset_ns - first_token_offset_ns)
                    / u64::from(bounds.output_tokens() - 1)
                    > 0 =>
        {
            duration_ns
        }
        _ => {
            return Err(fail(
                M1AuthenticatedTargetWindowExecutionStageV1::ClockFinish,
                M1AuthenticatedTargetWindowExecutionErrorV1::Clock,
                (engine, teardown, tokens, plans, successor_custody, registry),
            ));
        }
    };
    Ok(M1AuthenticatedTargetWindowExecutionSuccessV1 {
        tokens: tokens.into_boxed_slice(),
        timing: M1AuthenticatedTargetWindowTimingV1 {
            duration_ns,
            first_token_offset_ns,
            terminal_token_offset_ns,
        },
        retained: OpaqueCustody(Box::new((engine, teardown, successor_custody, registry))),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_r33_window_counts_prefill_choice_once() {
        let bounds = M1AuthenticatedTargetWindowBoundsV1::new(128).unwrap();
        assert_eq!(bounds.output_tokens(), 128);
        assert_eq!(bounds.successor_generations(), 127);
        assert_eq!(bounds.target_page_count(), 16);
        assert_eq!(bounds.newly_leased_target_page_count(), 8);

        let boundary = M1AuthenticatedTargetWindowBoundsV1::new(17).unwrap();
        assert_eq!(boundary.target_page_count(), 9);
        assert_eq!(boundary.newly_leased_target_page_count(), 1);
    }

    #[test]
    fn invalid_or_over_context_window_fails_before_custody_moves() {
        assert_eq!(
            M1AuthenticatedTargetWindowBoundsV1::new(1),
            Err(M1AuthenticatedTargetWindowInputErrorV1::OutputTokenCount { actual: 1 })
        );
        assert_eq!(
            M1AuthenticatedTargetWindowBoundsV1::new(M1_MAX_CONTEXT_TOKENS - 126),
            Err(M1AuthenticatedTargetWindowInputErrorV1::ContextExceeded {
                output_tokens: M1_MAX_CONTEXT_TOKENS - 126,
            })
        );
    }
}
