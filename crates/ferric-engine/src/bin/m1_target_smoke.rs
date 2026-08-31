//! Target-only prompt-to-text smoke execution.
//!
//! This module deliberately reuses the qualification command's authenticated
//! deployment bootstrap and long-lived target-decode queue. It emits a useful
//! text observation, but grants no evidence, qualification, or correctness
//! authority.

use super::{
    bind_m1_kv_workspace_table_v1, build_authenticated_sequential_plan_catalog,
    build_preliminary_identity_closure, complete_closure, domain_identity,
    generate_qwen3_gfx942_runner_declaration, hex_bytes, load_closure, load_model_inputs,
    model_memory_plan, prepare_m1_long_lived_queue_rearm_v1,
    publish_qwen3_gfx942_runner_declaration, release_m1_completed_step_kv_pages_v1,
    reopen_persisted_m1_kernel_artifacts_v1, reserve_m1_long_lived_queue_rearm_kv_v1,
    schedule_m1_long_lived_queue_rearm_v1, validate_m1_step_inputs, ActiveDeviceKvCache,
    CaptureResult, CompletionWireSemanticExpectation, DeviceSelector, Engine,
    M1CompletedStepOutcomeV1, M1DeviceKvCompletionDispositionV1, M1DeviceKvCompletionMemberV1,
    M1DeviceKvCompletionRosterV1, M1FullStepKvWorkspaceTablesV1, M1FullStepWorkspacePlans,
    M1LongLivedQueueRearmKvInputsV1, M1LongLivedQueueReleasedRoundV1,
    M1PhysicalRunnerRecipeOutcomeV1, M1StepDispatchIntent, M1StepInputCandidate,
    M1StepInputValidationOutcome, OpenedKfd, OsString, Path, Qwen3ExecutionMode, Qwen3ModelRole,
    Qwen3PlanBucket, Qwen3PlanSelection, RequestId, StepPlan,
};
use ferric_build::{
    authenticate_qwen3_tokenizer, SpecialTokenDecodePolicy, SpecialTokenEncodePolicy,
    TokenizerExecutionLimits,
};
use ferric_engine::{
    bind_structural_m1_physical_runner_v1, complete_m1_physical_step_v1,
    initialize_m1_physical_runner_memory_v1, require_m1_authenticated_roster_acquisition_v1,
    DeviceKvPageLease, M1LongLivedQueueRearmScheduleFailureV1, M1PhysicalRunnerV1,
    M1RearmedRoundReleaseOutcomeV1, M1ScheduledLongLivedQueueRearmV1,
};
use ferric_spec::{
    ValidatedM1StepInputs, M1_KV_PAGE_TOKENS, M1_QUALIFICATION_TOKENS_PER_LANE, QWEN3_IM_END_TOKEN,
};
use serde_json::json;
use std::collections::VecDeque;
use std::fmt;
use std::io::{Cursor, Write};
use std::process;

pub(super) const COMMAND: &str = "run-target-smoke";

const STATUS: &str = "smoke-non-evidence-non-qualification";
const AUTHORITY: &str = "ferric-target-only-smoke-only";
const NONCLAIM: &str = "Raw-prompt target-only text smoke only. Every prompt-priming and generation choice is reported as a non-evidence diagnostic and settled from the same inert physical K7 observation. This output is not evidence, is not a qualification result, does not establish numerical or hardware correctness, and closes no M1 requirement.";
const TARGET: &str = "gfx942:xnack-";

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

impl SmokeTokenLoop {
    fn new(prompt: Vec<u32>, max_new_tokens: usize) -> CaptureResult<Self> {
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

    fn context_ordinal(&self) -> CaptureResult<u32> {
        u32::try_from(self.dispatches)
            .map_err(|_| "dispatch count does not fit the physical context ordinal".to_owned())
    }

    fn next_input(&self) -> CaptureResult<u32> {
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
struct SmokeExecution {
    generated_tokens: Vec<u32>,
    prompt_observations: Vec<u32>,
    stop_reason: StopReason,
}

#[derive(Debug)]
enum ReleasedRound {
    First(Box<ferric_engine::M1ReleasedCompletedStepV1>),
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

pub(super) fn run(arguments: &[OsString]) -> CaptureResult<()> {
    let [prepacked_root, artifact_root, closure_path, gpu_unique_id, max_new_tokens, prompt] =
        arguments
    else {
        return Err("usage: ferric-m1-qualification-capture run-target-smoke PREPACKED-SNAPSHOT KERNEL-ARTIFACTS CLOSURE GPU-UNIQUE-ID MAX-NEW-TOKENS RAW-PROMPT".to_owned());
    };
    let gpu_unique_id = gpu_unique_id
        .to_str()
        .ok_or_else(|| "GPU unique ID must be UTF-8 decimal".to_owned())?
        .parse::<u64>()
        .map_err(|_| "GPU unique ID must be a decimal u64".to_owned())?;
    let max_new_tokens = max_new_tokens
        .to_str()
        .ok_or_else(|| "MAX-NEW-TOKENS must be UTF-8 decimal".to_owned())?
        .parse::<usize>()
        .map_err(|_| "MAX-NEW-TOKENS must be a decimal usize".to_owned())?;
    let prompt = prompt
        .to_str()
        .ok_or_else(|| "RAW-PROMPT must be UTF-8".to_owned())?;

    require_m1_authenticated_roster_acquisition_v1(Path::new(artifact_root))
        .map_err(|error| error.to_string())?;

    let closure = load_closure(Path::new(closure_path))?;
    let artifacts = reopen_persisted_m1_kernel_artifacts_v1(Path::new(artifact_root))
        .map_err(|error| format!("cannot authenticate persisted kernel artifacts: {error}"))?;
    let executable_catalog_id = artifacts.program_catalog_id();
    let snapshot =
        super::SecureDirectory::open(Path::new(prepacked_root), "prepacked snapshot root")?;
    let model = load_model_inputs(&snapshot)?;
    let tokenizer =
        authenticate_qwen3_tokenizer(Qwen3ModelRole::Target8B, Cursor::new(&model.tokenizer))
            .map_err(|error| format!("cannot authenticate target tokenizer: {error}"))?;
    let prompt_tokens = tokenizer
        .encode(
            prompt,
            TokenizerExecutionLimits::m1(),
            SpecialTokenEncodePolicy::Reject,
        )
        .map_err(|error| format!("cannot encode raw prompt: {error}"))?;
    let token_loop = SmokeTokenLoop::new(prompt_tokens, max_new_tokens)?;

    let runner_admission = model.authenticate()?;
    let plan_catalog = build_authenticated_sequential_plan_catalog(runner_admission)
        .map_err(|error| format!("cannot build authenticated plan catalog: {error:?}"))?;
    let external = complete_closure(&closure, &plan_catalog, executable_catalog_id)?;
    let identity_closure = build_preliminary_identity_closure(plan_catalog, external)
        .map_err(|error| format!("cannot build runner identity closure: {error:?}"))?;
    let declaration = generate_qwen3_gfx942_runner_declaration(identity_closure)
        .map_err(|error| format!("cannot generate authenticated runner declaration: {error:?}"))?;
    let publication = publish_qwen3_gfx942_runner_declaration(declaration)
        .map_err(|error| format!("cannot publish runner declaration: {error:?}"))?;
    let runner = bind_structural_m1_physical_runner_v1(artifacts, publication)
        .map_err(|error| format!("cannot bind physical runner: {error:?}"))?;

    let memory_admission = model.authenticate()?;
    let memory_plan = model_memory_plan(memory_admission)?;
    let checked = OpenedKfd::open_default()
        .map_err(|error| format!("cannot open KFD: {error}"))?
        .admit_uapi()
        .map_err(|error| format!("cannot admit pinned KFD UAPI: {error}"))?
        .bind_gfx942_xnack_minus(DeviceSelector::UniqueId(gpu_unique_id))
        .map_err(|error| format!("cannot bind selected gfx942:xnack- device: {error}"))?;
    let memory = initialize_m1_physical_runner_memory_v1(
        checked,
        memory_plan,
        model.target_weights,
        model.draft_weights,
    )
    .map_err(|error| format!("cannot initialize physical model memory: {error:?}"))?;

    let execution = execute(&runner, memory, token_loop);
    let text_bytes = tokenizer
        .decode_to_bytes(
            &execution.generated_tokens,
            TokenizerExecutionLimits::m1(),
            SpecialTokenDecodePolicy::Skip,
        )
        .map_err(|error| format!("cannot decode generated token bytes: {error}"))?;
    let text = lossy_text(&text_bytes);
    let direct_published_token_count = execution
        .prompt_observations
        .len()
        .saturating_add(execution.generated_tokens.len());
    let report = json!({
        "authority": AUTHORITY,
        "direct_published_token_count": direct_published_token_count,
        "generated_token_count": execution.generated_tokens.len(),
        "generated_token_ids": execution.generated_tokens,
        "nonclaim": NONCLAIM,
        "prompt_priming_published_choice_token_ids": execution.prompt_observations,
        "status": STATUS,
        "target": TARGET,
        "termination": execution.stop_reason.as_str(),
        "text": text,
        "text_bytes_hex": hex_bytes(&text_bytes),
        "text_utf8_policy": "lossy-replacement",
    });
    let mut stdout = std::io::stdout().lock();
    serde_json::to_writer(&mut stdout, &report)
        .map_err(|error| format!("cannot serialize smoke report: {error}"))?;
    stdout
        .write_all(b"\n")
        .map_err(|error| format!("cannot write smoke report: {error}"))?;
    Ok(())
}

fn execute(
    runner: &M1PhysicalRunnerV1,
    mut memory: ferric_engine::M1PartitionedModelMemoryKvPoolV1,
    mut tokens: SmokeTokenLoop,
) -> SmokeExecution {
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
    let prepared = match super::prepare_scheduled_workspaces_with_retries(
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
    let mut allocated =
        match super::allocate_scheduled_workspaces_with_retries(runner, memory, prepared) {
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
    let published = match super::publish_first_step_with_retries(
        runner,
        &mut engine,
        allocated,
        recipe,
        completion,
    ) {
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
        return finish_execution(tokens, reason);
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
            return finish_execution(tokens, reason);
        }
    }
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

fn finish_execution(tokens: SmokeTokenLoop, stop_reason: StopReason) -> SmokeExecution {
    SmokeExecution {
        generated_tokens: tokens.generated,
        prompt_observations: tokens.prompt_observations,
        stop_reason,
    }
}

fn lossy_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn bind_step_plan(
    runner: &M1PhysicalRunnerV1,
    scheduled: &ferric_engine::M1ScheduledDispatchV1,
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
    super::workload_workspace_plan(selection, workspace_identity)
        .unwrap_or_else(|error| fail_stop("smoke workspace planning", error))
}

fn smoke_recipe(
    runner: &M1PhysicalRunnerV1,
    selection: Qwen3PlanSelection,
    workspace_identity: [u8; 32],
) -> ferric_engine::AddresslessM1PhysicalBufferRecipeV1 {
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

fn observed_token(image: &ferric_engine::M1ObservedCompletionImageV1) -> CaptureResult<u32> {
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
    readback: ferric_engine::M1PhysicalCompletedReadbackV1,
    member: M1DeviceKvCompletionMemberV1,
    unused_page_leases: VecDeque<DeviceKvPageLease>,
) -> (
    ferric_engine::M1CompletedStepSuccessV1,
    VecDeque<DeviceKvPageLease>,
) {
    let roster = M1DeviceKvCompletionRosterV1::new(vec![member]);
    match complete_m1_physical_step_v1(engine, readback, roster) {
        M1CompletedStepOutcomeV1::Completed(completed) => (completed, unused_page_leases),
        other => fail_stop("smoke initial completion", (other, unused_page_leases)),
    }
}

fn release_first(
    completed: ferric_engine::M1CompletedStepSuccessV1,
    unused_page_leases: VecDeque<DeviceKvPageLease>,
) -> (
    ferric_engine::M1ReleasedCompletedStepV1,
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
    fn truncated_utf8_is_preserved_in_hex_and_displayed_lossily() {
        let bytes = [0xf0, 0x9f];
        assert_eq!(hex_bytes(&bytes), "f09f");
        assert_eq!(lossy_text(&bytes), "\u{fffd}");
    }

    #[test]
    fn report_authority_is_explicitly_non_evidentiary() {
        assert_eq!(STATUS, "smoke-non-evidence-non-qualification");
        assert!(NONCLAIM.contains("not evidence"));
        assert!(NONCLAIM.contains("not a qualification result"));
        assert!(NONCLAIM.contains("closes no M1 requirement"));
        assert!(NONCLAIM.contains("Every prompt-priming and generation choice is reported"));
    }

    #[test]
    fn command_reports_the_exact_positional_usage() {
        let error = run(&[]).unwrap_err();
        assert!(error.contains("run-target-smoke PREPACKED-SNAPSHOT"));
        assert!(!error.contains("MODEL-SOURCE"));
        assert!(error.ends_with("MAX-NEW-TOKENS RAW-PROMPT"));
    }
}
