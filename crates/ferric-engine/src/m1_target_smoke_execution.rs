//! Artifact-neutral target-only prompt-to-text smoke execution.
//!
//! The controller begins only after a caller has bound a structural runner and
//! initialized model memory. Artifact admission, publication selection, and
//! reporting remain the responsibility of the calling binary.

use ferric_build::{
    m1_step_workspace_requirements, plan_addressless_m1_step_workspace, AvailableM1StepWorkspace,
    DeclaredM1StepWorkspaceAllocation, M1StepWorkspaceDeclaration, M1StepWorkspacePlanOutcome,
};
use ferric_spec::{
    validate_m1_step_inputs, Identity, M1StepInputCandidate, M1StepInputValidationOutcome,
    Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection, RequestId, StepPlan,
    ValidatedM1StepInputs, M1_KV_PAGE_TOKENS, M1_QUALIFICATION_TOKENS_PER_LANE, QWEN3_IM_END_TOKEN,
};
use rustix::time::{clock_gettime, ClockId};
use sha2::{Digest, Sha256};

use crate::{
    bind_m1_kv_workspace_table_v1, complete_m1_physical_step_v1,
    prepare_m1_long_lived_queue_rearm_v1, release_m1_completed_step_kv_pages_v1,
    reserve_m1_long_lived_queue_rearm_kv_v1, schedule_m1_long_lived_queue_rearm_v1,
    ActiveDeviceKvCache, CompletionWireSemanticExpectation, DeviceKvPageLease, Engine,
    M1CompletedStepOutcomeV1, M1DeviceKvCompletionDispositionV1, M1DeviceKvCompletionMemberV1,
    M1DeviceKvCompletionRosterV1, M1FullStepKvWorkspaceTablesV1, M1FullStepWorkspacePlans,
    M1LongLivedQueueRearmKvInputsV1, M1LongLivedQueueRearmScheduleFailureV1,
    M1LongLivedQueueReleasedRoundV1, M1PhysicalRunnerRecipeOutcomeV1, M1PhysicalRunnerV1,
    M1RearmedRoundReleaseOutcomeV1, M1ScheduledLongLivedQueueRearmV1, M1StepDispatchIntent,
};
use std::collections::VecDeque;
use std::fmt;
use std::io::Write;
use std::process;

const SMOKE_RECOVERY_RETRIES: usize = 2;
const SMOKE_QUEUE_RING_BYTES: u32 = 1 << 20;

type SmokeResult<T> = Result<T, String>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StopReason {
    ImEnd,
    MaxNewTokens,
    ContextBound,
}

impl StopReason {
    const fn as_str(self) -> &'static str {
        match self {
            Self::ImEnd => "qwen3-im-end",
            Self::MaxNewTokens => "max-new-tokens",
            Self::ContextBound => "context-bound",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ObservationAction {
    Continue,
    Stop(StopReason),
}

#[derive(Debug)]
struct SmokeTokenLoop {
    prompt: Vec<u32>,
    dispatches: usize,
    prompt_observations: Vec<u32>,
    generated: Vec<u32>,
    max_new_tokens: usize,
}

#[derive(Debug)]
struct SmokeTimer {
    first_generated_token_offset_ns: Option<u64>,
    last_generated_token_offset_ns: Option<u64>,
    started_ns: u128,
}

impl SmokeTimer {
    fn start() -> SmokeResult<Self> {
        Ok(Self {
            first_generated_token_offset_ns: None,
            last_generated_token_offset_ns: None,
            started_ns: monotonic_raw_ns()?,
        })
    }

    fn observe_generated(&mut self, tokens: &SmokeTokenLoop) {
        if tokens.generated.is_empty() {
            return;
        }
        let offset = elapsed_ns(self.started_ns)
            .unwrap_or_else(|error| fail_stop("smoke generated-token timing", error));
        if tokens.generated.len() == 1 && self.first_generated_token_offset_ns.is_none() {
            self.first_generated_token_offset_ns = Some(offset);
        }
        self.last_generated_token_offset_ns = Some(offset);
    }

    fn finish(self) -> SmokeResult<M1TargetSmokeTimingV1> {
        let first_generated_token_offset_ns =
            self.first_generated_token_offset_ns.ok_or_else(|| {
                "smoke execution completed without a first generated token".to_owned()
            })?;
        let duration_ns = elapsed_ns(self.started_ns)?;
        let last_generated_token_offset_ns = self
            .last_generated_token_offset_ns
            .ok_or_else(|| "smoke execution completed without a last generated token".to_owned())?;
        if last_generated_token_offset_ns < first_generated_token_offset_ns
            || duration_ns < last_generated_token_offset_ns
        {
            return Err("smoke generated-token timing order is invalid".to_owned());
        }
        Ok(M1TargetSmokeTimingV1 {
            duration_ns,
            first_generated_token_offset_ns,
            last_generated_token_offset_ns,
        })
    }
}

impl SmokeTokenLoop {
    fn new(prompt: Vec<u32>, max_new_tokens: usize) -> SmokeResult<Self> {
        if prompt.is_empty() {
            return Err("raw prompt encoded to zero tokens".to_owned());
        }
        if prompt.len() > M1_QUALIFICATION_TOKENS_PER_LANE as usize {
            return Err("raw prompt exceeds the C8192 context bound".to_owned());
        }
        if max_new_tokens == 0 || max_new_tokens > M1_QUALIFICATION_TOKENS_PER_LANE as usize {
            return Err("MAX-NEW-TOKENS must be in 1..=8192".to_owned());
        }
        Ok(Self {
            prompt,
            dispatches: 0,
            prompt_observations: Vec::new(),
            generated: Vec::new(),
            max_new_tokens,
        })
    }

    fn context_ordinal(&self) -> SmokeResult<u32> {
        u32::try_from(self.dispatches)
            .map_err(|_| "dispatch count does not fit the physical context ordinal".to_owned())
    }

    fn next_input(&self) -> SmokeResult<u32> {
        if self.dispatches < self.prompt.len() {
            return Ok(self.prompt[self.dispatches]);
        }
        self.generated
            .last()
            .copied()
            .ok_or_else(|| "generation feedback token is absent".to_owned())
    }

    fn maximum_dispatches(&self) -> usize {
        self.prompt
            .len()
            .saturating_add(self.max_new_tokens.saturating_sub(1))
            .min(M1_QUALIFICATION_TOKENS_PER_LANE as usize)
    }

    fn required_page_count(&self) -> usize {
        self.maximum_dispatches()
            .div_ceil(M1_KV_PAGE_TOKENS as usize)
    }

    fn observe(&mut self, token: u32) -> ObservationAction {
        let completing_prompt = self.dispatches + 1 == self.prompt.len();
        let generating = self.dispatches >= self.prompt.len();
        self.dispatches += 1;
        if completing_prompt || generating {
            self.generated.push(token);
            if token == QWEN3_IM_END_TOKEN {
                return ObservationAction::Stop(StopReason::ImEnd);
            }
            if self.generated.len() == self.max_new_tokens {
                return ObservationAction::Stop(StopReason::MaxNewTokens);
            }
        } else {
            self.prompt_observations.push(token);
        }
        if self.dispatches == M1_QUALIFICATION_TOKENS_PER_LANE as usize {
            return ObservationAction::Stop(StopReason::ContextBound);
        }
        ObservationAction::Continue
    }
}

#[derive(Debug)]
pub struct M1TargetSmokeExecutionV1 {
    prompt_tokens: Vec<u32>,
    generated_tokens: Vec<u32>,
    prompt_observations: Vec<u32>,
    stop_reason: StopReason,
    timing: M1TargetSmokeTimingV1,
}

/// Monotonic-raw timing for the single request executed by target smoke.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1TargetSmokeTimingV1 {
    duration_ns: u64,
    first_generated_token_offset_ns: u64,
    last_generated_token_offset_ns: u64,
}

impl M1TargetSmokeTimingV1 {
    /// Controller-entry to completed device teardown in nanoseconds.
    #[must_use]
    pub const fn duration_ns(self) -> u64 {
        self.duration_ns
    }

    /// Controller-entry to the first structurally observed generated token.
    #[must_use]
    pub const fn first_generated_token_offset_ns(self) -> u64 {
        self.first_generated_token_offset_ns
    }

    /// Controller-entry to the last structurally observed generated token.
    #[must_use]
    pub const fn last_generated_token_offset_ns(self) -> u64 {
        self.last_generated_token_offset_ns
    }
}

impl M1TargetSmokeExecutionV1 {
    /// Exact raw-prompt tokens dispatched by the controller.
    #[must_use]
    pub fn prompt_tokens(&self) -> &[u32] {
        &self.prompt_tokens
    }

    /// Exact target choices observed while teacher-forcing the prompt prefix.
    #[must_use]
    pub fn prompt_observations(&self) -> &[u32] {
        &self.prompt_observations
    }

    /// Exact generated target token sequence.
    #[must_use]
    pub fn generated_tokens(&self) -> &[u32] {
        &self.generated_tokens
    }

    /// Stable termination label for the controller's exact stopping condition.
    #[must_use]
    pub const fn termination(&self) -> &'static str {
        self.stop_reason.as_str()
    }

    /// Timing for this single target-smoke request.
    #[must_use]
    pub const fn timing(&self) -> M1TargetSmokeTimingV1 {
        self.timing
    }
}

#[derive(Debug)]
enum ReleasedRound {
    First(Box<crate::M1ReleasedCompletedStepV1>),
    Rearmed(Box<M1LongLivedQueueReleasedRoundV1>),
}

impl ReleasedRound {
    fn schedule(
        self,
        engine: &mut Engine<1>,
    ) -> Result<M1ScheduledLongLivedQueueRearmV1, M1LongLivedQueueRearmScheduleFailureV1> {
        match self {
            Self::First(released) => schedule_m1_long_lived_queue_rearm_v1(engine, *released),
            Self::Rearmed(released) => (*released).schedule_next(engine),
        }
    }

    fn teardown(self, engine: &mut Engine<1>, unused_page_leases: VecDeque<DeviceKvPageLease>) {
        match self {
            Self::First(released) => match (*released).destroy_queue_and_retain_step(engine) {
                Ok(closed) => drop((closed, unused_page_leases)),
                Err(failure) => fail_stop(
                    "final first-generation queue teardown",
                    (failure, unused_page_leases),
                ),
            },
            Self::Rearmed(released) => match (*released).destroy_queue_and_retain_round(engine) {
                Ok(closed) => drop((closed, unused_page_leases)),
                Err(failure) => fail_stop(
                    "final rearmed queue teardown",
                    (failure, unused_page_leases),
                ),
            },
        }
    }
}

/// Runs one target-only token loop from an already-bound structural runner.
///
/// Artifact admission and model-memory initialization must be completed by the
/// calling boundary. Runtime custody failures fail-stop without fabricating a
/// recoverable owner.
///
/// # Errors
///
/// Rejects an empty or over-bound prompt and an invalid generation bound before
/// any device operation begins.
pub fn execute_m1_target_smoke_v1(
    runner: &M1PhysicalRunnerV1,
    memory: crate::M1PartitionedModelMemoryKvPoolV1,
    prompt_tokens: Vec<u32>,
    max_new_tokens: usize,
) -> SmokeResult<M1TargetSmokeExecutionV1> {
    let tokens = SmokeTokenLoop::new(prompt_tokens, max_new_tokens)?;
    let timing = SmokeTimer::start()?;
    Ok(execute_smoke(runner, memory, tokens, timing))
}

fn execute_smoke(
    runner: &M1PhysicalRunnerV1,
    mut memory: crate::M1PartitionedModelMemoryKvPoolV1,
    mut tokens: SmokeTokenLoop,
    mut timing: SmokeTimer,
) -> M1TargetSmokeExecutionV1 {
    let selection = Qwen3PlanSelection {
        role: Qwen3ModelRole::Target8B,
        mode: Qwen3ExecutionMode::Decode,
        bucket: Qwen3PlanBucket::DecodeS1C8192,
    };
    let draft_selection = Qwen3PlanSelection {
        role: Qwen3ModelRole::Draft06B,
        mode: Qwen3ExecutionMode::Decode,
        bucket: Qwen3PlanBucket::DecodeS1C8192,
    };
    let workspace_identity = smoke_workspace_identity(&tokens);
    let mut engine = match Engine::<1>::new(
        M1_QUALIFICATION_TOKENS_PER_LANE / M1_KV_PAGE_TOKENS,
        M1_KV_PAGE_TOKENS,
        M1_QUALIFICATION_TOKENS_PER_LANE,
    ) {
        Ok(engine) => engine,
        Err(error) => fail_stop("smoke Engine construction", error),
    };
    let request = match engine.admit() {
        Ok(request) => request,
        Err(error) => fail_stop("smoke request admission", error),
    };
    let mut cache =
        match ActiveDeviceKvCache::new(memory.device(), request, selection, draft_selection) {
            Ok(cache) => cache,
            Err(error) => fail_stop("smoke device-KV cache construction", (memory, error)),
        };
    let page_count = tokens.required_page_count();
    let mut unused_page_leases = VecDeque::new();
    if unused_page_leases.try_reserve_exact(page_count).is_err() {
        fail_stop(
            "smoke target-page lease custody allocation",
            (memory, cache),
        );
    }
    for physical_index in 0..page_count {
        let physical_index = match u32::try_from(physical_index) {
            Ok(index) => index,
            Err(error) => fail_stop(
                "smoke target-page index conversion",
                (memory, cache, unused_page_leases, error),
            ),
        };
        match memory.lease_page(request, Qwen3ModelRole::Target8B, physical_index) {
            Ok(lease) => unused_page_leases.push_back(lease),
            Err(error) => fail_stop(
                "smoke target-page prelease",
                (memory, cache, unused_page_leases, physical_index, error),
            ),
        }
    }
    let Some(first_page) = unused_page_leases.pop_front() else {
        fail_stop("smoke first target page absent", (memory, cache));
    };
    if let Err(error) = engine.append_tentative(request, 1) {
        fail_stop(
            "smoke initial Engine enqueue",
            (memory, cache, first_page, unused_page_leases, error),
        );
    }
    let scheduled = match engine.dispatch_m1_ready() {
        Ok(Some(scheduled)) => scheduled,
        other => fail_stop(
            "smoke initial scheduling",
            (memory, cache, first_page, unused_page_leases, other),
        ),
    };
    let plan = bind_step_plan(runner, &scheduled, request, selection, 0);
    let input_token = match tokens.next_input() {
        Ok(token) => token,
        Err(error) => fail_stop(
            "smoke initial input selection",
            (
                memory,
                cache,
                first_page,
                unused_page_leases,
                scheduled,
                error,
            ),
        ),
    };
    let input = validated_step_input(plan, input_token, 0);
    let reservation = match cache.reserve_step_write(
        request,
        Qwen3ModelRole::Target8B,
        0,
        1,
        scheduled.epoch(),
        vec![first_page],
    ) {
        Ok(reservation) => reservation,
        Err(failure) => fail_stop(
            "smoke initial KV reservation",
            (memory, cache, unused_page_leases, scheduled, input, failure),
        ),
    };
    let table = match bind_m1_kv_workspace_table_v1(input, vec![reservation]) {
        Ok(table) => table,
        Err(failure) => fail_stop(
            "smoke initial KV workspace binding",
            (memory, cache, unused_page_leases, scheduled, failure),
        ),
    };
    let workspace_plan = smoke_workspace_plan(selection, workspace_identity);
    let recipe = smoke_recipe(runner, selection, workspace_identity);
    let prepared = match prepare_scheduled_workspaces_with_retries(
        runner,
        scheduled,
        M1FullStepWorkspacePlans::target_only(workspace_plan),
        M1FullStepKvWorkspaceTablesV1::TargetOnly { target: table },
    ) {
        Ok(prepared) => prepared,
        Err(failure) => fail_stop(
            "smoke initial workspace preparation",
            (memory, cache, unused_page_leases, OpaqueCustody(failure)),
        ),
    };
    let mut allocated = match allocate_scheduled_workspaces_with_retries(runner, memory, prepared) {
        Ok(allocated) => allocated,
        Err(failure) => fail_stop_opaque(
            "smoke initial workspace allocation",
            (cache, unused_page_leases, failure),
        ),
    };
    let completion = match allocated.allocate_completion_output(selection) {
        Ok(completion) => completion,
        Err(error) => fail_stop(
            "smoke initial completion allocation",
            (allocated, cache, unused_page_leases, error),
        ),
    };
    let published =
        match publish_first_step_with_retries(runner, &mut engine, allocated, recipe, completion) {
            Ok(published) => published,
            Err(failure) => fail_stop(
                "smoke initial publication",
                (cache, unused_page_leases, failure),
            ),
        };
    let completed = match published.wait() {
        Ok(completed) => completed,
        Err(failure) => fail_stop(
            "smoke initial queue wait",
            (cache, unused_page_leases, failure),
        ),
    };
    let recycled = match completed.recycle() {
        Ok(recycled) => recycled,
        Err(failure) => fail_stop(
            "smoke initial queue recycle",
            (cache, unused_page_leases, failure),
        ),
    };
    let observed = match recycled.observe_completion() {
        Ok(observed) => observed,
        Err(failure) => match failure.retry() {
            Ok(observed) => observed,
            Err(failure) => fail_stop(
                "smoke initial K7 observation",
                (cache, unused_page_leases, failure),
            ),
        },
    };
    let emitted = match observed_token(observed.image()) {
        Ok(token) => token,
        Err(error) => fail_stop(
            "smoke initial K7 S1 observation shape",
            (cache, unused_page_leases, observed, error),
        ),
    };
    let action = tokens.observe(emitted);
    timing.observe_generated(&tokens);
    let semantic = [CompletionWireSemanticExpectation::DirectFinalRow { choice: emitted }];
    let readback = match observed.check_completion(&semantic) {
        Ok(readback) => readback,
        Err(failure) => fail_stop(
            "smoke initial semantic settlement",
            (cache, unused_page_leases, failure),
        ),
    };
    let stopping = matches!(action, ObservationAction::Stop(_));
    if stopping {
        if let Err(error) = engine.retire(request) {
            fail_stop(
                "smoke initial retirement",
                (cache, unused_page_leases, readback, error),
            );
        }
    }
    let member = if stopping {
        M1DeviceKvCompletionMemberV1::retiring(cache)
    } else {
        M1DeviceKvCompletionMemberV1::continuing(cache)
    };
    let (completed, leases) = complete_first(&mut engine, readback, member, unused_page_leases);
    let (released, mut unused_page_leases) = release_first(completed, leases);
    if let ObservationAction::Stop(reason) = action {
        ReleasedRound::First(Box::new(released)).teardown(&mut engine, unused_page_leases);
        return finish_execution(tokens, reason, timing);
    }
    let mut released = ReleasedRound::First(Box::new(released));

    loop {
        let ordinal = match tokens.context_ordinal() {
            Ok(ordinal) => ordinal,
            Err(error) => fail_stop(
                "smoke rearm context ordinal",
                (released, unused_page_leases, error),
            ),
        };
        if let Err(error) = engine.append_tentative(request, 1) {
            fail_stop(
                "smoke rearm Engine enqueue",
                (released, unused_page_leases, error),
            );
        }
        let scheduled = match released.schedule(&mut engine) {
            Ok(scheduled) => scheduled,
            Err(failure) => fail_stop("smoke rearm scheduling", (failure, unused_page_leases)),
        };
        let lane_page_leases = if ordinal.is_multiple_of(M1_KV_PAGE_TOKENS) {
            let Some(lease) = unused_page_leases.pop_front() else {
                fail_stop(
                    "smoke preleased target page absent",
                    (scheduled, unused_page_leases, ordinal),
                );
            };
            vec![lease]
        } else {
            Vec::new()
        };
        let plan = bind_step_plan(
            runner,
            scheduled.scheduled_dispatch(),
            request,
            selection,
            ordinal,
        );
        let input_token = match tokens.next_input() {
            Ok(token) => token,
            Err(error) => fail_stop(
                "smoke rearm input selection",
                (scheduled, lane_page_leases, unused_page_leases, error),
            ),
        };
        let input = validated_step_input(plan, input_token, ordinal);
        let reserved = match reserve_m1_long_lived_queue_rearm_kv_v1(
            &mut engine,
            scheduled,
            M1LongLivedQueueRearmKvInputsV1::target_only(input, vec![lane_page_leases]),
        ) {
            Ok(reserved) => reserved,
            Err(failure) => fail_stop("smoke rearm KV reservation", (failure, unused_page_leases)),
        };
        let workspace_plan = smoke_workspace_plan(selection, workspace_identity);
        let prepared = match prepare_m1_long_lived_queue_rearm_v1(
            &mut engine,
            reserved,
            runner.logical_runner(),
            M1FullStepWorkspacePlans::target_only(workspace_plan),
        ) {
            Ok(prepared) => prepared,
            Err(failure) => fail_stop(
                "smoke rearm workspace preparation",
                (failure, unused_page_leases),
            ),
        };
        let recipe = smoke_recipe(runner, selection, workspace_identity);
        let published = match runner.submit_rearm(&mut engine, prepared, recipe) {
            Ok(published) => published,
            Err(failure) => match failure.retry(runner, &mut engine) {
                Ok(published) => published,
                Err(failure) => fail_stop("smoke rearm submission", (failure, unused_page_leases)),
            },
        };
        let completed = match published.wait(&mut engine) {
            Ok(completed) => completed,
            Err(failure) => fail_stop("smoke rearm queue wait", (failure, unused_page_leases)),
        };
        let recycled = match completed.recycle(&mut engine) {
            Ok(recycled) => recycled,
            Err(failure) => fail_stop("smoke rearm queue recycle", (failure, unused_page_leases)),
        };
        let observed = match recycled.observe_completion() {
            Ok(observed) => observed,
            Err(failure) => match failure.retry_observation() {
                Ok(observed) => observed,
                Err(failure) => {
                    fail_stop("smoke rearm K7 observation", (failure, unused_page_leases))
                }
            },
        };
        let emitted = match observed_token(observed.image()) {
            Ok(token) => token,
            Err(error) => fail_stop(
                "smoke rearm K7 S1 observation shape",
                (observed, unused_page_leases, error),
            ),
        };
        let action = tokens.observe(emitted);
        timing.observe_generated(&tokens);
        let semantic = [CompletionWireSemanticExpectation::DirectFinalRow { choice: emitted }];
        let readback = match observed.check_completion(&semantic) {
            Ok(readback) => readback,
            Err(failure) => fail_stop(
                "smoke rearm semantic settlement",
                (failure, unused_page_leases),
            ),
        };
        let stopping = matches!(action, ObservationAction::Stop(_));
        if stopping {
            if let Err(error) = engine.retire(request) {
                fail_stop(
                    "smoke rearm retirement",
                    (readback, unused_page_leases, error),
                );
            }
        }
        let disposition = if stopping {
            M1DeviceKvCompletionDispositionV1::Retire
        } else {
            M1DeviceKvCompletionDispositionV1::Continue
        };
        let completion = match readback.complete(&mut engine, vec![disposition]) {
            Ok(completion) => completion,
            Err(failure) => fail_stop(
                "smoke rearm completion preflight",
                (failure, unused_page_leases),
            ),
        };
        released = match completion.release_completed() {
            M1RearmedRoundReleaseOutcomeV1::Released(released) => {
                ReleasedRound::Rearmed(Box::new(released))
            }
            other => fail_stop("smoke rearm page release", (other, unused_page_leases)),
        };
        if let ObservationAction::Stop(reason) = action {
            released.teardown(&mut engine, unused_page_leases);
            return finish_execution(tokens, reason, timing);
        }
    }
}

enum SmokePreparationFailureV1 {
    Join {
        _diagnostic: crate::M1PrepublicationJoinErrorV1,
        _scheduled: Box<crate::M1ScheduledDispatchV1>,
        _plans: M1FullStepWorkspacePlans,
        _tables: Box<M1FullStepKvWorkspaceTablesV1>,
    },
    Composition {
        _failure: crate::M1PrepublicationCompositionFailureV1,
    },
}

enum SmokeAllocationFailureV1 {
    Preflight {
        _diagnostic: crate::InitializedM1FullStepWorkspacePreflightErrorV1,
        _memory: Box<crate::M1PartitionedModelMemoryKvPoolV1>,
        _prepared: Box<crate::M1PreparedScheduledWorkspaceImagesV1>,
    },
    Terminal {
        _failure: crate::M1PrepublicationAllocationFailureV1,
    },
}

fn prepare_scheduled_workspaces_with_retries(
    runner: &M1PhysicalRunnerV1,
    scheduled: crate::M1ScheduledDispatchV1,
    plans: M1FullStepWorkspacePlans,
    tables: M1FullStepKvWorkspaceTablesV1,
) -> Result<crate::M1PreparedScheduledWorkspaceImagesV1, Box<SmokePreparationFailureV1>> {
    let mut failure = match runner.prepare_scheduled_workspaces(scheduled, plans, tables) {
        Ok(prepared) => return Ok(prepared),
        Err(failure) => failure,
    };
    let mut attempts = 0;
    loop {
        let (diagnostic, scheduled, plans, tables) = match failure {
            crate::M1PrepareFailureV1::Join(failure) => failure.into_parts(),
            crate::M1PrepareFailureV1::Composition(failure) => {
                return Err(Box::new(SmokePreparationFailureV1::Composition {
                    _failure: failure,
                }));
            }
        };
        if attempts == SMOKE_RECOVERY_RETRIES {
            return Err(Box::new(SmokePreparationFailureV1::Join {
                _diagnostic: diagnostic,
                _scheduled: Box::new(scheduled),
                _plans: plans,
                _tables: Box::new(tables),
            }));
        }
        attempts += 1;
        failure = match runner.prepare_scheduled_workspaces(scheduled, plans, tables) {
            Ok(prepared) => return Ok(prepared),
            Err(failure) => failure,
        };
    }
}

fn allocate_scheduled_workspaces_with_retries(
    runner: &M1PhysicalRunnerV1,
    memory: crate::M1PartitionedModelMemoryKvPoolV1,
    prepared: crate::M1PreparedScheduledWorkspaceImagesV1,
) -> Result<crate::M1AllocatedScheduledStepV1, Box<SmokeAllocationFailureV1>> {
    let mut failure = match runner.allocate_scheduled_workspaces(memory, prepared) {
        Ok(allocated) => return Ok(allocated),
        Err(failure) => failure,
    };
    let mut attempts = 0;
    loop {
        let (diagnostic, memory, prepared) = match failure.into_preflight_prepared() {
            Ok(parts) => parts,
            Err(failure) => {
                return Err(Box::new(SmokeAllocationFailureV1::Terminal {
                    _failure: failure,
                }));
            }
        };
        if attempts == SMOKE_RECOVERY_RETRIES {
            return Err(Box::new(SmokeAllocationFailureV1::Preflight {
                _diagnostic: diagnostic,
                _memory: Box::new(memory),
                _prepared: Box::new(prepared),
            }));
        }
        attempts += 1;
        failure = match runner.allocate_scheduled_workspaces(memory, prepared) {
            Ok(allocated) => return Ok(allocated),
            Err(failure) => failure,
        };
    }
}

fn publish_first_step_with_retries<'runner>(
    runner: &'runner M1PhysicalRunnerV1,
    engine: &mut Engine<1>,
    allocated: crate::M1AllocatedScheduledStepV1,
    recipe: crate::AddresslessM1PhysicalBufferRecipeV1,
    completion: crate::BoundM1CompletionOutputV1,
) -> Result<
    crate::M1PhysicalPublishedQueueSessionV1,
    crate::M1PhysicalRunnerFirstPublicationFailureV1<'runner>,
> {
    let failure = match runner.publish_first_step(
        engine,
        SMOKE_QUEUE_RING_BYTES,
        allocated,
        recipe,
        completion,
    ) {
        Ok(published) => return Ok(published),
        Err(failure) => failure,
    };
    retry_with_bounded_policy(failure, SMOKE_RECOVERY_RETRIES, |failure| {
        failure.retry(runner, engine, SMOKE_QUEUE_RING_BYTES)
    })
}

fn retry_with_bounded_policy<Owner, Success>(
    mut owner: Owner,
    attempts: usize,
    mut retry: impl FnMut(Owner) -> Result<Success, Owner>,
) -> Result<Success, Owner> {
    for _ in 0..attempts {
        owner = match retry(owner) {
            Ok(success) => return Ok(success),
            Err(owner) => owner,
        };
    }
    Err(owner)
}

fn workload_workspace_plan(
    selection: Qwen3PlanSelection,
    workload_identity: [u8; 32],
) -> SmokeResult<ferric_build::AddresslessM1StepWorkspacePlan> {
    let requirements = m1_step_workspace_requirements(selection)
        .map_err(|error| format!("cannot derive workspace requirements: {error:?}"))?;
    let identity = domain_identity(
        b"ferric.m1.qualification-workspace.v1",
        &[&workload_identity, selection_bytes(selection).as_slice()],
    );
    let available = AvailableM1StepWorkspace::new(M1StepWorkspaceDeclaration::new(
        selection,
        DeclaredM1StepWorkspaceAllocation::new(
            identity,
            requirements.allocation_byte_len(),
            requirements.allocation_alignment(),
        ),
        requirements.ranges().to_vec().into_boxed_slice(),
    ));
    match plan_addressless_m1_step_workspace(selection, available) {
        M1StepWorkspacePlanOutcome::Planned(plan) => Ok(plan),
        M1StepWorkspacePlanOutcome::Rejected(error) => {
            Err(format!("workload workspace plan rejected: {error:?}"))
        }
    }
}

fn selection_bytes(selection: Qwen3PlanSelection) -> Vec<u8> {
    format!(
        "target-8b\0{}\0{}",
        match selection.mode {
            Qwen3ExecutionMode::Prefill => "prefill",
            Qwen3ExecutionMode::Decode => "decode",
            Qwen3ExecutionMode::Speculative => "speculative",
        },
        bucket_name(selection.bucket)
    )
    .into_bytes()
}

const fn bucket_name(bucket: Qwen3PlanBucket) -> &'static str {
    match bucket {
        Qwen3PlanBucket::PrefillS1T128 => "prefill-s1-t128",
        Qwen3PlanBucket::PrefillS8T128 => "prefill-s8-t128",
        Qwen3PlanBucket::PrefillS1T512 => "prefill-s1-t512",
        Qwen3PlanBucket::PrefillS1T2048 => "prefill-s1-t2048",
        Qwen3PlanBucket::DecodeS1C8192 => "decode-s1-c8192",
        Qwen3PlanBucket::DecodeS8C8192 => "decode-s8-c8192",
        Qwen3PlanBucket::DecodeS32C8192 => "decode-s32-c8192",
        Qwen3PlanBucket::SpeculativeS1K4C8192 => "speculative-s1-k4-c8192",
        Qwen3PlanBucket::SpeculativeS8K4C8192 => "speculative-s8-k4-c8192",
        Qwen3PlanBucket::SpeculativeS1K8C8192 => "speculative-s1-k8-c8192",
        Qwen3PlanBucket::SpeculativeS1K16C8192 => "speculative-s1-k16-c8192",
    }
}

fn domain_identity(domain: &[u8], fields: &[&[u8]]) -> Identity {
    let mut hasher = Sha256::new();
    hash_field(&mut hasher, domain);
    for field in fields {
        hash_field(&mut hasher, field);
    }
    Identity::new(hasher.finalize().into())
}

fn hash_field(hasher: &mut Sha256, field: &[u8]) {
    hasher.update(u64::try_from(field.len()).unwrap_or(u64::MAX).to_le_bytes());
    hasher.update(field);
}

fn smoke_workspace_identity(tokens: &SmokeTokenLoop) -> [u8; 32] {
    let mut prompt_bytes = Vec::with_capacity(tokens.prompt.len().saturating_mul(4));
    for token in &tokens.prompt {
        prompt_bytes.extend_from_slice(&token.to_le_bytes());
    }
    let max_new_tokens = u64::try_from(tokens.max_new_tokens)
        .unwrap_or(u64::MAX)
        .to_le_bytes();
    *domain_identity(
        b"ferric.m1.target-smoke-workload.v1",
        &[&prompt_bytes, &max_new_tokens],
    )
    .as_bytes()
}

fn finish_execution(
    tokens: SmokeTokenLoop,
    stop_reason: StopReason,
    timing: SmokeTimer,
) -> M1TargetSmokeExecutionV1 {
    let timing = timing
        .finish()
        .unwrap_or_else(|error| fail_stop("smoke terminal timing", error));
    M1TargetSmokeExecutionV1 {
        prompt_tokens: tokens.prompt,
        generated_tokens: tokens.generated,
        prompt_observations: tokens.prompt_observations,
        stop_reason,
        timing,
    }
}

fn monotonic_raw_ns() -> SmokeResult<u128> {
    let timestamp = clock_gettime(ClockId::MonotonicRaw);
    let seconds = u128::try_from(timestamp.tv_sec)
        .map_err(|_| "monotonic-raw clock returned negative seconds".to_owned())?;
    let nanoseconds = u128::try_from(timestamp.tv_nsec)
        .map_err(|_| "monotonic-raw clock returned negative nanoseconds".to_owned())?;
    seconds
        .checked_mul(1_000_000_000)
        .and_then(|value| value.checked_add(nanoseconds))
        .ok_or_else(|| "monotonic-raw nanosecond conversion overflowed".to_owned())
}

fn elapsed_ns(started_ns: u128) -> SmokeResult<u64> {
    let elapsed = monotonic_raw_ns()?
        .checked_sub(started_ns)
        .ok_or_else(|| "monotonic-raw clock moved backwards".to_owned())?;
    u64::try_from(elapsed).map_err(|_| "smoke timing duration does not fit u64".to_owned())
}

fn bind_step_plan(
    runner: &M1PhysicalRunnerV1,
    scheduled: &crate::M1ScheduledDispatchV1,
    request: RequestId,
    selection: Qwen3PlanSelection,
    ordinal: u32,
) -> StepPlan {
    if scheduled.member_count() != 1 || scheduled.member(0) != Some(request) {
        fail_stop(
            "smoke scheduled roster drift",
            (scheduled, request, ordinal),
        );
    }
    match runner
        .logical_runner()
        .bind_step_plan(request, scheduled.epoch(), selection)
    {
        Ok(plan) => plan,
        Err(error) => fail_stop("smoke step-plan binding", (scheduled, ordinal, error)),
    }
}

fn validated_step_input(plan: StepPlan, token: u32, context: u32) -> ValidatedM1StepInputs {
    let candidate = M1StepInputCandidate::new(
        plan.selection(),
        vec![Some(plan)],
        vec![token],
        vec![context],
        vec![1],
        vec![context],
    );
    match validate_m1_step_inputs(candidate) {
        M1StepInputValidationOutcome::Validated(inputs) => inputs,
        M1StepInputValidationOutcome::Rejected(failure) => {
            fail_stop("smoke step-input validation", failure)
        }
    }
}

fn smoke_workspace_plan(
    selection: Qwen3PlanSelection,
    workspace_identity: [u8; 32],
) -> ferric_build::AddresslessM1StepWorkspacePlan {
    workload_workspace_plan(selection, workspace_identity)
        .unwrap_or_else(|error| fail_stop("smoke workspace planning", error))
}

fn smoke_recipe(
    runner: &M1PhysicalRunnerV1,
    selection: Qwen3PlanSelection,
    workspace_identity: [u8; 32],
) -> crate::AddresslessM1PhysicalBufferRecipeV1 {
    match runner.derive_step_recipe(
        M1StepDispatchIntent::TargetOnly(selection),
        M1FullStepWorkspacePlans::target_only(smoke_workspace_plan(selection, workspace_identity)),
    ) {
        M1PhysicalRunnerRecipeOutcomeV1::Prepared(recipe) => recipe,
        M1PhysicalRunnerRecipeOutcomeV1::Rejected(error) => {
            fail_stop("smoke physical recipe derivation", error)
        }
    }
}

fn observed_token(image: &crate::M1ObservedCompletionImageV1) -> SmokeResult<u32> {
    let [record] = image.records() else {
        return Err("K7 image does not contain exactly one S1 record".to_owned());
    };
    let [token] = record.emitted_tokens() else {
        return Err("K7 S1 record does not contain exactly one direct token".to_owned());
    };
    Ok(*token)
}

fn complete_first(
    engine: &mut Engine<1>,
    readback: crate::M1PhysicalCompletedReadbackV1,
    member: M1DeviceKvCompletionMemberV1,
    unused_page_leases: VecDeque<DeviceKvPageLease>,
) -> (crate::M1CompletedStepSuccessV1, VecDeque<DeviceKvPageLease>) {
    let roster = M1DeviceKvCompletionRosterV1::new(vec![member]);
    match complete_m1_physical_step_v1(engine, readback, roster) {
        M1CompletedStepOutcomeV1::Completed(completed) => (completed, unused_page_leases),
        other => fail_stop("smoke initial completion", (other, unused_page_leases)),
    }
}

fn release_first(
    completed: crate::M1CompletedStepSuccessV1,
    unused_page_leases: VecDeque<DeviceKvPageLease>,
) -> (
    crate::M1ReleasedCompletedStepV1,
    VecDeque<DeviceKvPageLease>,
) {
    match release_m1_completed_step_kv_pages_v1(completed) {
        Ok(released) => (released, unused_page_leases),
        Err(failure) => fail_stop("smoke initial page release", (failure, unused_page_leases)),
    }
}

struct OpaqueCustody<T>(T);

impl<T> fmt::Debug for OpaqueCustody<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _ = &self.0;
        formatter.write_str("retained opaque custody")
    }
}

fn fail_stop<T: fmt::Debug>(phase: &'static str, custody: T) -> ! {
    let _ = writeln!(std::io::stderr().lock(), "FAIL-STOP: {phase}: {custody:?}");
    std::mem::forget(custody);
    process::abort()
}

fn fail_stop_opaque<T>(phase: &'static str, custody: T) -> ! {
    fail_stop(phase, OpaqueCustody(custody))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_prompt_is_teacher_forced_before_first_generated_token() {
        let mut state = SmokeTokenLoop::new(vec![10, 11, 12], 3).unwrap();
        assert_eq!(state.next_input().unwrap(), 10);
        assert_eq!(state.observe(90), ObservationAction::Continue);
        assert_eq!(state.next_input().unwrap(), 11);
        assert_eq!(state.observe(91), ObservationAction::Continue);
        assert_eq!(state.next_input().unwrap(), 12);
        assert_eq!(state.observe(20), ObservationAction::Continue);
        assert_eq!(state.next_input().unwrap(), 20);
        assert_eq!(state.prompt_observations, vec![90, 91]);
        assert_eq!(state.generated, vec![20]);
    }

    #[test]
    fn priming_choice_is_published_diagnostic_but_never_feedback_or_stop() {
        let mut state = SmokeTokenLoop::new(vec![10, 11], 1).unwrap();
        assert_eq!(
            state.observe(QWEN3_IM_END_TOKEN),
            ObservationAction::Continue
        );
        assert_eq!(state.next_input().unwrap(), 11);
        assert_eq!(state.prompt_observations, vec![QWEN3_IM_END_TOKEN]);
        assert!(state.generated.is_empty());
        assert_eq!(
            state.observe(20),
            ObservationAction::Stop(StopReason::MaxNewTokens)
        );
        assert_eq!(state.generated, vec![20]);
    }

    #[test]
    fn physical_feedback_stops_at_im_end() {
        let mut state = SmokeTokenLoop::new(vec![10], 4).unwrap();
        assert_eq!(state.observe(20), ObservationAction::Continue);
        assert_eq!(state.next_input().unwrap(), 20);
        assert_eq!(
            state.observe(QWEN3_IM_END_TOKEN),
            ObservationAction::Stop(StopReason::ImEnd)
        );
        assert_eq!(state.generated, vec![20, QWEN3_IM_END_TOKEN]);
    }

    #[test]
    fn max_new_tokens_is_an_exact_stop() {
        let mut state = SmokeTokenLoop::new(vec![10], 2).unwrap();
        assert_eq!(state.observe(20), ObservationAction::Continue);
        assert_eq!(
            state.observe(21),
            ObservationAction::Stop(StopReason::MaxNewTokens)
        );
    }

    #[test]
    fn page_budget_is_exact_across_boundaries_and_context_cap() {
        for (prompt, maximum_new, expected_dispatches, expected_pages) in [
            (1, 1, 1, 1),
            (16, 1, 16, 1),
            (16, 2, 17, 2),
            (17, 1, 17, 2),
            (1, 8_192, 8_192, 512),
            (8_192, 8_192, 8_192, 512),
        ] {
            let state = SmokeTokenLoop::new(vec![10; prompt], maximum_new).unwrap();
            assert_eq!(state.maximum_dispatches(), expected_dispatches);
            assert_eq!(state.required_page_count(), expected_pages);
        }
    }

    #[test]
    fn missing_feedback_state_fails_closed_instead_of_forging_a_token() {
        let mut state = SmokeTokenLoop::new(vec![10], 2).unwrap();
        state.dispatches = 1;
        assert_eq!(
            state.next_input().unwrap_err(),
            "generation feedback token is absent"
        );
    }

    #[test]
    fn context_bound_stops_before_an_out_of_range_feedback_dispatch() {
        let mut state =
            SmokeTokenLoop::new(vec![10; M1_QUALIFICATION_TOKENS_PER_LANE as usize], 2).unwrap();
        for _ in 0..M1_QUALIFICATION_TOKENS_PER_LANE - 1 {
            assert_eq!(state.observe(20), ObservationAction::Continue);
        }
        assert_eq!(
            state.observe(21),
            ObservationAction::Stop(StopReason::ContextBound)
        );
        assert_eq!(state.generated, vec![21]);
    }

    #[test]
    fn empty_prompt_and_zero_generation_bound_are_rejected() {
        assert!(SmokeTokenLoop::new(Vec::new(), 1).is_err());
        assert!(SmokeTokenLoop::new(vec![1], 0).is_err());
        assert!(SmokeTokenLoop::new(vec![1; 8_193], 1).is_err());
        assert!(SmokeTokenLoop::new(vec![1], 8_193).is_err());
    }

    #[test]
    fn completed_observation_retains_prompt_choices_and_termination() {
        let mut state = SmokeTokenLoop::new(vec![10], 1).unwrap();
        let mut timing = SmokeTimer::start().unwrap();
        assert_eq!(
            state.observe(20),
            ObservationAction::Stop(StopReason::MaxNewTokens)
        );
        timing.observe_generated(&state);
        let observation = finish_execution(state, StopReason::MaxNewTokens, timing);
        assert_eq!(observation.prompt_tokens(), &[10]);
        assert!(observation.prompt_observations().is_empty());
        assert_eq!(observation.generated_tokens(), &[20]);
        assert_eq!(observation.termination(), "max-new-tokens");
        assert!(observation.timing().duration_ns() > 0);
        assert_eq!(
            observation.timing().first_generated_token_offset_ns(),
            observation.timing().last_generated_token_offset_ns()
        );
        assert!(
            observation.timing().last_generated_token_offset_ns()
                <= observation.timing().duration_ns()
        );
    }
}
