//! Exact two-member S8/T128 paired-prefill input for an authenticated S8/K4 window.
//!
//! This module deliberately starts at the existing all-terminal new-window
//! boundary. It does not manufacture a fresh physical executor. The caller
//! must hold the real completed speculative executor and the registry's exact
//! all-terminal replacement reservation. Lower authenticated queue code owns
//! page allocation, direct-choice observation, KV settlement, and the first
//! speculative-successor join.

use core::fmt;

use ferric_spec::{
    completion::CompletionEpoch, validate_m1_step_inputs, M1StepInputCandidate,
    M1StepInputValidationOutcome, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket,
    Qwen3PlanSelection, RequestId, TokenId, ValidatedM1StepInputs, QWEN3_VOCABULARY_SIZE,
};

use crate::{
    prepare_m1_authenticated_speculative_new_window_v1,
    schedule_m1_authenticated_speculative_new_window_v1,
    submit_m1_authenticated_speculative_new_window_v1, Engine, LogicalRunnerDeclaration,
    M1AuthenticatedPreparedSpeculativeNewWindowV1, M1AuthenticatedScheduledSpeculativeNewWindowV1,
    M1AuthenticatedSpeculativeFailureDispositionV1, M1AuthenticatedSpeculativeNewWindowPublishedV1,
    M1AuthenticatedSpeculativeNewWindowScheduleErrorV1,
    M1AuthenticatedSpeculativeNewWindowScheduleFailureV1,
    M1AuthenticatedSpeculativeNewWindowSchedulePreDetachRetryV1,
    M1AuthenticatedSpeculativePhysicalExecutorV1, M1AuthenticatedSpeculativeRolloverMemberIntentV1,
    M1AuthenticatedSpeculativeRolloverSubmissionStageV1, M1FullStepWorkspaceInputKind,
    M1FullStepWorkspacePlans, M1QueueWaitTimeoutV1, M1ServingNewWindowPublicationReservationV1,
    M1ServingPlanV1, M1ServingQueueActionV1, M1ServingQueuedGenerationBindingV1,
    M1ServingQueuedPairedPrefillNewWindowV1, M1ServingRegistryErrorV1, M1ServingRegistryV1,
    M1SpeculativeGenerationPolicyV1,
};

const LIVE_MEMBERS: usize = 2;
const PHYSICAL_LANES: usize = 8;
const PREFILL_WIDTH: usize = 128;

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

fn paired_prefill_plan() -> M1ServingPlanV1 {
    M1ServingPlanV1::new(TARGET_PREFILL, DRAFT_PREFILL)
        .expect("the frozen S8/T128 pair is admitted")
}

fn speculative_successor_plan() -> M1ServingPlanV1 {
    M1ServingPlanV1::new(TARGET_SUCCESSOR, DRAFT_SUCCESSOR)
        .expect("the frozen S8/K4 pair is admitted")
}

fn exact_paired_prefill_workspace_plans(
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

/// Stable pure-input rejection before any executor or registry mutation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedS8K4WindowInputErrorV1 {
    Plan,
    QueueAction,
    MemberCount {
        actual: usize,
    },
    DuplicateSlot,
    PromptLength {
        lane: usize,
        actual: usize,
    },
    TokenOutOfRange {
        lane: usize,
        column: usize,
        token: TokenId,
    },
    WorkspaceShape,
    LogicalPlan {
        lane: usize,
        role: Qwen3ModelRole,
    },
    StepInput {
        role: Qwen3ModelRole,
    },
}

/// Rejected constructor inputs, including the live registry reservation.
#[must_use = "constructor rejection retains the exact reservation, prompts, policies, and plans"]
#[derive(Debug)]
pub struct M1AuthenticatedS8K4WindowInputFailureV1 {
    error: M1AuthenticatedS8K4WindowInputErrorV1,
    reservation: M1ServingNewWindowPublicationReservationV1,
    prompts: [Vec<TokenId>; LIVE_MEMBERS],
    policies: [M1SpeculativeGenerationPolicyV1; LIVE_MEMBERS],
    preparation_plans: M1FullStepWorkspacePlans,
    recipe_plans: M1FullStepWorkspacePlans,
}

impl M1AuthenticatedS8K4WindowInputFailureV1 {
    #[must_use]
    pub const fn error(&self) -> M1AuthenticatedS8K4WindowInputErrorV1 {
        self.error
    }

    /// Recovers every unchanged caller-owned constructor input.
    #[must_use = "the rejected registry reservation remains linear"]
    pub fn into_parts(
        self,
    ) -> (
        M1ServingNewWindowPublicationReservationV1,
        [Vec<TokenId>; LIVE_MEMBERS],
        [M1SpeculativeGenerationPolicyV1; LIVE_MEMBERS],
        M1FullStepWorkspacePlans,
        M1FullStepWorkspacePlans,
    ) {
        (
            self.reservation,
            self.prompts,
            self.policies,
            self.preparation_plans,
            self.recipe_plans,
        )
    }
}

fn input_failure(
    error: M1AuthenticatedS8K4WindowInputErrorV1,
    reservation: M1ServingNewWindowPublicationReservationV1,
    prompts: [Vec<TokenId>; LIVE_MEMBERS],
    policies: [M1SpeculativeGenerationPolicyV1; LIVE_MEMBERS],
    preparation_plans: M1FullStepWorkspacePlans,
    recipe_plans: M1FullStepWorkspacePlans,
) -> Box<M1AuthenticatedS8K4WindowInputFailureV1> {
    Box::new(M1AuthenticatedS8K4WindowInputFailureV1 {
        error,
        reservation,
        prompts,
        policies,
        preparation_plans,
        recipe_plans,
    })
}

fn validate_prompt_rows(
    requests: &[RequestId],
    prompts: &[Vec<TokenId>; LIVE_MEMBERS],
) -> Result<(), M1AuthenticatedS8K4WindowInputErrorV1> {
    if requests.len() != LIVE_MEMBERS {
        return Err(M1AuthenticatedS8K4WindowInputErrorV1::MemberCount {
            actual: requests.len(),
        });
    }
    if requests[0].slot() == requests[1].slot() {
        return Err(M1AuthenticatedS8K4WindowInputErrorV1::DuplicateSlot);
    }
    for (lane, prompt) in prompts.iter().enumerate() {
        if prompt.len() != PREFILL_WIDTH {
            return Err(M1AuthenticatedS8K4WindowInputErrorV1::PromptLength {
                lane,
                actual: prompt.len(),
            });
        }
        if let Some((column, token)) = prompt
            .iter()
            .copied()
            .enumerate()
            .find(|(_, token)| *token >= QWEN3_VOCABULARY_SIZE)
        {
            return Err(M1AuthenticatedS8K4WindowInputErrorV1::TokenOutOfRange {
                lane,
                column,
                token,
            });
        }
    }
    Ok(())
}

fn padded_prefill_inputs(
    runner: &LogicalRunnerDeclaration,
    requests: &[RequestId],
    epoch: CompletionEpoch,
    selection: Qwen3PlanSelection,
    prompts: &[Vec<TokenId>; LIVE_MEMBERS],
) -> Result<ValidatedM1StepInputs, M1AuthenticatedS8K4WindowInputErrorV1> {
    let mut lanes = Vec::with_capacity(PHYSICAL_LANES);
    for (lane, request) in requests.iter().copied().enumerate() {
        let plan = runner
            .bind_step_plan(request, epoch, selection)
            .map_err(|_| M1AuthenticatedS8K4WindowInputErrorV1::LogicalPlan {
                lane,
                role: selection.role,
            })?;
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
        active_lengths[lane] = u32::try_from(PREFILL_WIDTH).expect("T128 width fits u32");
    }
    let candidate = M1StepInputCandidate::new(
        selection,
        lanes,
        tokens,
        positions,
        active_lengths,
        context_lengths,
    );
    match validate_m1_step_inputs(candidate) {
        M1StepInputValidationOutcome::Validated(inputs) => Ok(inputs),
        M1StepInputValidationOutcome::Rejected(_) => {
            Err(M1AuthenticatedS8K4WindowInputErrorV1::StepInput {
                role: selection.role,
            })
        }
    }
}

fn ordered_member_intents(
    requests: &[RequestId],
    policies: [M1SpeculativeGenerationPolicyV1; LIVE_MEMBERS],
) -> Vec<M1AuthenticatedSpeculativeRolloverMemberIntentV1> {
    requests
        .iter()
        .copied()
        .zip(policies)
        .map(|(request, policy)| {
            M1AuthenticatedSpeculativeRolloverMemberIntentV1::new(request, policy)
        })
        .collect()
}

/// Exact two-member paired-prefill owner for one all-terminal S8/K4 window.
///
/// The physical predecessor executor is intentionally absent. It enters only
/// through [`Self::schedule`], where the existing authenticated scheduler
/// checks terminal lineage before detachment and allocates disjoint draft and
/// target pages after detachment.
#[must_use = "the exact window input must be scheduled or retained"]
#[derive(Debug)]
pub struct M1AuthenticatedS8K4WindowInputV1 {
    reservation: M1ServingNewWindowPublicationReservationV1,
    queued: M1ServingQueuedPairedPrefillNewWindowV1,
    successor: M1ServingPlanV1,
    member_intents: Vec<M1AuthenticatedSpeculativeRolloverMemberIntentV1>,
}

impl M1AuthenticatedS8K4WindowInputV1 {
    /// Binds an exact ordered two-member S8/T128 prompt/policy roster.
    ///
    /// # Errors
    ///
    /// Purely rejects a substituted plan/action/roster, malformed prompt, a
    /// non-paired workspace owner, or a logical-runner plan mismatch. Every
    /// caller-owned value is returned unchanged in the failure.
    pub fn new(
        reservation: M1ServingNewWindowPublicationReservationV1,
        runner: &LogicalRunnerDeclaration,
        prompts: [Vec<TokenId>; LIVE_MEMBERS],
        policies: [M1SpeculativeGenerationPolicyV1; LIVE_MEMBERS],
        preparation_plans: M1FullStepWorkspacePlans,
        recipe_plans: M1FullStepWorkspacePlans,
    ) -> Result<Self, Box<M1AuthenticatedS8K4WindowInputFailureV1>> {
        let prefill = paired_prefill_plan();
        let successor = speculative_successor_plan();
        let batch = reservation.physical_batch();
        if reservation.plan() != prefill || batch.plan() != prefill {
            return Err(input_failure(
                M1AuthenticatedS8K4WindowInputErrorV1::Plan,
                reservation,
                prompts,
                policies,
                preparation_plans,
                recipe_plans,
            ));
        }
        if reservation.action()
            != (M1ServingQueueActionV1::QuiescentNewWindow {
                prior: successor,
                next: prefill,
            })
        {
            return Err(input_failure(
                M1AuthenticatedS8K4WindowInputErrorV1::QueueAction,
                reservation,
                prompts,
                policies,
                preparation_plans,
                recipe_plans,
            ));
        }
        if let Err(error) = validate_prompt_rows(batch.requests(), &prompts) {
            return Err(input_failure(
                error,
                reservation,
                prompts,
                policies,
                preparation_plans,
                recipe_plans,
            ));
        }
        if !exact_paired_prefill_workspace_plans(&preparation_plans, &recipe_plans) {
            return Err(input_failure(
                M1AuthenticatedS8K4WindowInputErrorV1::WorkspaceShape,
                reservation,
                prompts,
                policies,
                preparation_plans,
                recipe_plans,
            ));
        }
        let draft_prefill = match padded_prefill_inputs(
            runner,
            batch.requests(),
            batch.epoch(),
            DRAFT_PREFILL,
            &prompts,
        ) {
            Ok(inputs) => inputs,
            Err(error) => {
                return Err(input_failure(
                    error,
                    reservation,
                    prompts,
                    policies,
                    preparation_plans,
                    recipe_plans,
                ));
            }
        };
        let target_prefill = match padded_prefill_inputs(
            runner,
            batch.requests(),
            batch.epoch(),
            TARGET_PREFILL,
            &prompts,
        ) {
            Ok(inputs) => inputs,
            Err(error) => {
                return Err(input_failure(
                    error,
                    reservation,
                    prompts,
                    policies,
                    preparation_plans,
                    recipe_plans,
                ));
            }
        };
        let requests = batch.requests().to_vec().into_boxed_slice();
        let queued = M1ServingQueuedPairedPrefillNewWindowV1::new(
            M1ServingQueuedGenerationBindingV1::new(prefill, requests.clone(), batch.epoch()),
            draft_prefill,
            target_prefill,
            preparation_plans,
            recipe_plans,
        );
        let member_intents = ordered_member_intents(&requests, policies);
        Ok(Self {
            reservation,
            queued,
            successor,
            member_intents,
        })
    }

    #[must_use]
    pub fn requests(&self) -> &[RequestId] {
        self.reservation.requests()
    }

    #[must_use]
    pub const fn epoch(&self) -> CompletionEpoch {
        self.reservation.epoch()
    }

    /// Enters the existing authenticated scheduler at its pure pre-detach gate.
    ///
    /// # Errors
    ///
    /// Pure rejection returns a retry owner that still contains the completed
    /// executor, queue inputs, and registry reservation. Detach-or-later
    /// failure seals both registry and physical custody behind an opaque owner.
    pub fn schedule<const C: usize>(
        self,
        engine: &mut Engine<C>,
        executor: M1AuthenticatedSpeculativePhysicalExecutorV1,
        ring_bytes: u32,
        next_queue_wait_timeout: M1QueueWaitTimeoutV1,
    ) -> Result<M1AuthenticatedS8K4WindowScheduledV1, M1AuthenticatedS8K4WindowScheduleFailureV1>
    {
        let batch = self.reservation.physical_batch();
        match schedule_m1_authenticated_speculative_new_window_v1(
            engine,
            executor,
            &batch,
            self.queued,
            self.successor,
            self.member_intents,
            ring_bytes,
            next_queue_wait_timeout,
        ) {
            Ok(scheduled) => Ok(M1AuthenticatedS8K4WindowScheduledV1 {
                reservation: self.reservation,
                scheduled,
            }),
            Err(failure) => Err(schedule_failure(self.reservation, failure)),
        }
    }
}

/// Opaque detach-or-later custody. No registry reservation or queue owner can
/// be recovered independently.
#[must_use = "terminal registry and physical custody must remain retained"]
pub struct M1AuthenticatedS8K4WindowTerminalCustodyV1(Box<dyn fmt::Debug>);

impl fmt::Debug for M1AuthenticatedS8K4WindowTerminalCustodyV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _ = &self.0;
        formatter
            .debug_struct("M1AuthenticatedS8K4WindowTerminalCustodyV1")
            .finish_non_exhaustive()
    }
}

/// Exact pre-detach retry owner. It deliberately exposes no component parts.
#[must_use = "retry custody contains the completed executor and registry reservation"]
#[derive(Debug)]
pub struct M1AuthenticatedS8K4WindowPreDetachRetryV1 {
    reservation: M1ServingNewWindowPublicationReservationV1,
    retry: M1AuthenticatedSpeculativeNewWindowSchedulePreDetachRetryV1,
}

impl M1AuthenticatedS8K4WindowPreDetachRetryV1 {
    /// Retries the unchanged pure schedule admission against its frozen batch.
    ///
    /// # Errors
    ///
    /// Pure rejection preserves the retry owner. Detach-or-later failure
    /// retains the registry and physical custody in an opaque terminal owner.
    pub fn retry<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<M1AuthenticatedS8K4WindowScheduledV1, M1AuthenticatedS8K4WindowScheduleFailureV1>
    {
        let batch = self.reservation.physical_batch();
        match self.retry.retry(engine, &batch) {
            Ok(scheduled) => Ok(M1AuthenticatedS8K4WindowScheduledV1 {
                reservation: self.reservation,
                scheduled,
            }),
            Err(failure) => Err(schedule_failure(self.reservation, failure)),
        }
    }

    /// Abandons the retry without exposing either registry or queue authority.
    #[must_use = "terminal registry and physical custody remain retained"]
    pub fn cancel_and_close<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> M1AuthenticatedS8K4WindowTerminalCustodyV1 {
        let disposition = self.retry.cancel_and_close(engine);
        M1AuthenticatedS8K4WindowTerminalCustodyV1(Box::new((self.reservation, disposition)))
    }
}

/// Pure retry rejection or opaque detach-or-later custody.
#[must_use = "new-window failure custody must be retried or retained"]
#[derive(Debug)]
pub enum M1AuthenticatedS8K4WindowScheduleFailureV1 {
    PreDetach {
        error: M1AuthenticatedSpeculativeNewWindowScheduleErrorV1,
        retry: Box<M1AuthenticatedS8K4WindowPreDetachRetryV1>,
    },
    Terminal {
        error: M1AuthenticatedSpeculativeNewWindowScheduleErrorV1,
        custody: M1AuthenticatedS8K4WindowTerminalCustodyV1,
    },
}

impl M1AuthenticatedS8K4WindowScheduleFailureV1 {
    #[must_use]
    pub const fn error(&self) -> M1AuthenticatedSpeculativeNewWindowScheduleErrorV1 {
        match self {
            Self::PreDetach { error, .. } | Self::Terminal { error, .. } => *error,
        }
    }

    #[must_use]
    pub const fn is_pre_detach_retry(&self) -> bool {
        matches!(self, Self::PreDetach { .. })
    }
}

fn schedule_failure(
    reservation: M1ServingNewWindowPublicationReservationV1,
    failure: M1AuthenticatedSpeculativeNewWindowScheduleFailureV1,
) -> M1AuthenticatedS8K4WindowScheduleFailureV1 {
    match failure {
        M1AuthenticatedSpeculativeNewWindowScheduleFailureV1::PreDetach { error, retry } => {
            M1AuthenticatedS8K4WindowScheduleFailureV1::PreDetach {
                error,
                retry: Box::new(M1AuthenticatedS8K4WindowPreDetachRetryV1 {
                    reservation,
                    retry: *retry,
                }),
            }
        }
        M1AuthenticatedSpeculativeNewWindowScheduleFailureV1::Terminal { error, disposition } => {
            M1AuthenticatedS8K4WindowScheduleFailureV1::Terminal {
                error,
                custody: M1AuthenticatedS8K4WindowTerminalCustodyV1(Box::new((
                    reservation,
                    disposition,
                ))),
            }
        }
    }
}

/// Detached exact schedule retaining the uncommitted registry reservation.
#[must_use = "scheduled custody must be prepared or retained"]
#[derive(Debug)]
pub struct M1AuthenticatedS8K4WindowScheduledV1 {
    reservation: M1ServingNewWindowPublicationReservationV1,
    scheduled: M1AuthenticatedScheduledSpeculativeNewWindowV1,
}

impl M1AuthenticatedS8K4WindowScheduledV1 {
    /// Derives the authenticated paired-prefill recipe without exposing the
    /// detached queue.
    ///
    /// # Errors
    ///
    /// Recipe preparation failure retains the registry reservation together
    /// with the scheduler's retry or terminal physical custody.
    pub fn prepare<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<M1AuthenticatedS8K4WindowPreparedV1, M1AuthenticatedS8K4WindowScheduleFailureV1>
    {
        match prepare_m1_authenticated_speculative_new_window_v1(engine, self.scheduled) {
            Ok(prepared) => Ok(M1AuthenticatedS8K4WindowPreparedV1 {
                reservation: self.reservation,
                prepared,
            }),
            Err(failure) => Err(schedule_failure(self.reservation, failure)),
        }
    }

    /// Closes a detached schedule through the existing authenticated recipe
    /// boundary and retains the registry reservation opaquely.
    #[must_use = "terminal registry and physical custody remain retained"]
    pub fn cancel_and_close<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> M1AuthenticatedS8K4WindowTerminalCustodyV1 {
        let disposition =
            match prepare_m1_authenticated_speculative_new_window_v1(engine, self.scheduled) {
                Ok(prepared) => prepared.cancel_and_close(engine),
                Err(M1AuthenticatedSpeculativeNewWindowScheduleFailureV1::PreDetach {
                    retry,
                    ..
                }) => retry.cancel_and_close(engine),
                Err(M1AuthenticatedSpeculativeNewWindowScheduleFailureV1::Terminal {
                    disposition,
                    ..
                }) => disposition,
            };
        M1AuthenticatedS8K4WindowTerminalCustodyV1(Box::new((self.reservation, disposition)))
    }
}

/// Recipe-checked paired-prefill owner and its exact registry transaction.
#[must_use = "prepared custody must be published and recorded or retained"]
#[derive(Debug)]
pub struct M1AuthenticatedS8K4WindowPreparedV1 {
    reservation: M1ServingNewWindowPublicationReservationV1,
    prepared: M1AuthenticatedPreparedSpeculativeNewWindowV1,
}

/// Terminal submit or registry-record rejection with no separable authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedS8K4WindowPublishErrorV1 {
    Submission(M1AuthenticatedSpeculativeRolloverSubmissionStageV1),
    Registry(M1ServingRegistryErrorV1),
}

#[must_use = "terminal publication custody must remain retained"]
#[derive(Debug)]
pub struct M1AuthenticatedS8K4WindowPublishFailureV1 {
    error: M1AuthenticatedS8K4WindowPublishErrorV1,
    custody: M1AuthenticatedS8K4WindowTerminalCustodyV1,
}

impl M1AuthenticatedS8K4WindowPublishFailureV1 {
    #[must_use]
    pub const fn error(&self) -> M1AuthenticatedS8K4WindowPublishErrorV1 {
        self.error
    }

    #[must_use = "terminal registry and physical custody remain retained"]
    pub const fn custody(&self) -> &M1AuthenticatedS8K4WindowTerminalCustodyV1 {
        &self.custody
    }
}

impl M1AuthenticatedS8K4WindowPreparedV1 {
    /// Cancels before submission and retains the registry reservation with the
    /// authenticated physical closure.
    #[must_use = "terminal registry and physical custody remain retained"]
    pub fn cancel_and_close<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> M1AuthenticatedS8K4WindowTerminalCustodyV1 {
        let disposition = self.prepared.cancel_and_close(engine);
        M1AuthenticatedS8K4WindowTerminalCustodyV1(Box::new((self.reservation, disposition)))
    }

    /// Preflights the supplied registry, publishes once, then records the same
    /// reservation in that registry.
    ///
    /// A physical submission failure and a defensive registry-record failure
    /// both close or quarantine physical custody and retain every remaining
    /// owner opaquely. Success returns the existing published owner; its
    /// observation exposes ordered first choices and its release owns the only
    /// admitted S8/K4 successor join.
    ///
    /// # Errors
    ///
    /// Registry preflight, physical submission, or registry recording can
    /// reject the window. Every failure retains opaque terminal custody.
    pub fn submit_and_record<const C: usize>(
        self,
        engine: &mut Engine<C>,
        registry: &mut M1ServingRegistryV1<C>,
    ) -> Result<
        M1AuthenticatedSpeculativeNewWindowPublishedV1,
        M1AuthenticatedS8K4WindowPublishFailureV1,
    > {
        if let Err(error) = registry.preflight_new_window_publication(&self.reservation) {
            let disposition = self.prepared.cancel_and_close(engine);
            return Err(M1AuthenticatedS8K4WindowPublishFailureV1 {
                error: M1AuthenticatedS8K4WindowPublishErrorV1::Registry(error),
                custody: M1AuthenticatedS8K4WindowTerminalCustodyV1(Box::new((
                    self.reservation,
                    disposition,
                ))),
            });
        }
        let published =
            match submit_m1_authenticated_speculative_new_window_v1(engine, self.prepared) {
                Ok(published) => published,
                Err(failure) => {
                    let stage = failure.stage();
                    let disposition = failure.into_disposition();
                    return Err(M1AuthenticatedS8K4WindowPublishFailureV1 {
                        error: M1AuthenticatedS8K4WindowPublishErrorV1::Submission(stage),
                        custody: M1AuthenticatedS8K4WindowTerminalCustodyV1(Box::new((
                            self.reservation,
                            disposition,
                        ))),
                    });
                }
            };
        if let Err(failure) = registry.record_new_window_publication(self.reservation) {
            let error = failure.error();
            let disposition: M1AuthenticatedSpeculativeFailureDispositionV1 =
                published.cancel_and_close(engine).retain(failure);
            return Err(M1AuthenticatedS8K4WindowPublishFailureV1 {
                error: M1AuthenticatedS8K4WindowPublishErrorV1::Registry(error),
                custody: M1AuthenticatedS8K4WindowTerminalCustodyV1(Box::new(disposition)),
            });
        }
        Ok(published)
    }
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
                .expect("generate fixture runner declaration");
        let published = publish_qwen3_gfx942_runner_declaration(generated)
            .expect("publish fixture runner declaration");
        LogicalRunnerDeclaration::from_published(published)
    }

    fn prompts() -> [Vec<TokenId>; LIVE_MEMBERS] {
        [vec![17; PREFILL_WIDTH], vec![29; PREFILL_WIDTH]]
    }

    fn workspace_plan(selection: Qwen3PlanSelection, byte: u8) -> AddresslessM1StepWorkspacePlan {
        let requirements =
            m1_step_workspace_requirements(selection).expect("workspace requirements");
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

    fn workspace_plans(
        draft: Qwen3PlanSelection,
        target: Qwen3PlanSelection,
        identity: u8,
    ) -> M1FullStepWorkspacePlans {
        M1FullStepWorkspacePlans::paired_prefill(
            workspace_plan(draft, identity),
            workspace_plan(target, identity.wrapping_add(1)),
        )
    }

    #[test]
    fn two_live_s8_rows_bind_ordered_requests_and_pad_six_lanes() {
        let runner = logical_runner();
        let requests = [RequestId::new(3, 2), RequestId::new(7, 2)];
        let prompts = prompts();
        let epoch = CompletionEpoch::new(9);
        for selection in [DRAFT_PREFILL, TARGET_PREFILL] {
            let inputs = padded_prefill_inputs(&runner, &requests, epoch, selection, &prompts)
                .expect("two-member S8 rows validate");
            assert_eq!(inputs.live_lane_count(), LIVE_MEMBERS as u32);
            assert_eq!(inputs.lanes().len(), PHYSICAL_LANES);
            assert_eq!(inputs.active_lengths(), &[128, 128, 0, 0, 0, 0, 0, 0]);
            assert_eq!(inputs.context_lengths(), &[0; PHYSICAL_LANES]);
            for (lane, request) in requests.iter().copied().enumerate() {
                let plan = inputs.lanes()[lane].as_ref().expect("live lane has a plan");
                assert_eq!(plan.request(), request);
                assert_eq!(plan.completion_epoch(), epoch);
                assert_eq!(plan.selection(), selection);
                let start = lane * PREFILL_WIDTH;
                assert_eq!(
                    &inputs.token_ids()[start..start + PREFILL_WIDTH],
                    &prompts[lane]
                );
                assert_eq!(
                    &inputs.position_ids()[start..start + PREFILL_WIDTH],
                    &(0..128_u32).collect::<Vec<_>>()
                );
            }
            assert!(inputs.lanes()[LIVE_MEMBERS..].iter().all(Option::is_none));
            assert!(inputs.token_ids()[LIVE_MEMBERS * PREFILL_WIDTH..]
                .iter()
                .all(|token| *token == 0));
            assert!(inputs.position_ids()[LIVE_MEMBERS * PREFILL_WIDTH..]
                .iter()
                .all(|position| *position == 0));
        }
    }

    #[test]
    fn prompt_gate_rejects_bad_count_duplicate_slot_length_and_token() {
        let exact = prompts();
        assert_eq!(
            validate_prompt_rows(&[RequestId::new(0, 2)], &exact),
            Err(M1AuthenticatedS8K4WindowInputErrorV1::MemberCount { actual: 1 })
        );
        assert_eq!(
            validate_prompt_rows(&[RequestId::new(0, 2), RequestId::new(0, 3)], &exact,),
            Err(M1AuthenticatedS8K4WindowInputErrorV1::DuplicateSlot)
        );
        let mut short = prompts();
        short[1].pop();
        assert_eq!(
            validate_prompt_rows(&[RequestId::new(0, 2), RequestId::new(1, 2)], &short,),
            Err(M1AuthenticatedS8K4WindowInputErrorV1::PromptLength {
                lane: 1,
                actual: 127,
            })
        );
        let mut invalid = prompts();
        invalid[0][23] = QWEN3_VOCABULARY_SIZE;
        assert_eq!(
            validate_prompt_rows(&[RequestId::new(0, 2), RequestId::new(1, 2)], &invalid,),
            Err(M1AuthenticatedS8K4WindowInputErrorV1::TokenOutOfRange {
                lane: 0,
                column: 23,
                token: QWEN3_VOCABULARY_SIZE,
            })
        );
    }

    #[test]
    fn exact_plan_pair_is_s8_prefill_to_s8_k4() {
        let prefill = paired_prefill_plan();
        let successor = speculative_successor_plan();
        assert_eq!(prefill.target(), TARGET_PREFILL);
        assert_eq!(prefill.draft(), DRAFT_PREFILL);
        assert_eq!(prefill.sequence_capacity(), PHYSICAL_LANES);
        assert_eq!(successor.target(), TARGET_SUCCESSOR);
        assert_eq!(successor.draft(), DRAFT_SUCCESSOR);
        assert_eq!(successor.sequence_capacity(), PHYSICAL_LANES);
        assert!(
            crate::m1_serving_registry::admit_m1_production_rollover_transition_v1(
                prefill, successor,
            )
            .is_some()
        );
    }

    #[test]
    fn member_intents_preserve_request_and_policy_order() {
        let requests = [RequestId::new(3, 2), RequestId::new(7, 2)];
        let policies = [
            M1SpeculativeGenerationPolicyV1::new(7, &[17]).expect("first policy"),
            M1SpeculativeGenerationPolicyV1::new(11, &[29]).expect("second policy"),
        ];
        let intents = ordered_member_intents(&requests, policies);
        assert_eq!(intents.len(), LIVE_MEMBERS);
        for lane in 0..LIVE_MEMBERS {
            assert_eq!(intents[lane].request(), requests[lane]);
            assert_eq!(intents[lane].policy(), policies[lane]);
        }
    }

    #[test]
    fn exact_workspace_gate_rejects_wrong_s1_selectors_and_unequal_plan_sets() {
        let preparation = workspace_plans(DRAFT_PREFILL, TARGET_PREFILL, 1);
        let recipe = workspace_plans(DRAFT_PREFILL, TARGET_PREFILL, 1);
        assert!(exact_paired_prefill_workspace_plans(&preparation, &recipe));

        let s1_target = Qwen3PlanSelection {
            bucket: Qwen3PlanBucket::PrefillS1T128,
            ..TARGET_PREFILL
        };
        let s1_draft = Qwen3PlanSelection {
            bucket: Qwen3PlanBucket::PrefillS1T128,
            ..DRAFT_PREFILL
        };
        let wrong = workspace_plans(s1_draft, s1_target, 1);
        assert!(!exact_paired_prefill_workspace_plans(&preparation, &wrong));

        let unequal = workspace_plans(DRAFT_PREFILL, TARGET_PREFILL, 9);
        assert!(!exact_paired_prefill_workspace_plans(
            &preparation,
            &unequal
        ));
    }

    #[test]
    fn source_delegates_all_physical_authority_to_existing_authenticated_path() {
        let source = include_str!("authenticated_s8_k4_window.rs");
        assert!(source.contains("schedule_m1_authenticated_speculative_new_window_v1("));
        assert!(source.contains("prepare_m1_authenticated_speculative_new_window_v1("));
        assert!(source.contains("submit_m1_authenticated_speculative_new_window_v1("));
        let preflight = source
            .find("registry.preflight_new_window_publication")
            .expect("registry preflight is present");
        let submit = source
            .find("submit_m1_authenticated_speculative_new_window_v1(")
            .expect("physical submit is present");
        assert!(preflight < submit);
        assert!(source.contains("registry.record_new_window_publication"));
    }
}
