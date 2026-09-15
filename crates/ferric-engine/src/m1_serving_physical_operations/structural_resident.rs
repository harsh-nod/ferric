//! Real structural resident transitions. These owners never grant authenticated admission.

use super::{
    fmt, joined_structural_draft_catchup_pending, schedule_m1_long_lived_queue_rearm_exact_v1,
    validate_custody_guard, M1QueuedServingPhysicalInputProviderV1, M1ServingBatchPlanV1,
    M1ServingCommittedSpeculativeRoundV1, M1ServingPhysicalReadbackV1,
    M1ServingPhysicalRunnerAdapterPhaseV1, M1ServingPhysicalRunnerDiagnosticHistoryV1,
    M1ServingPhysicalRunnerOperationErrorV1, M1ServingPhysicalRunnerOperationsV1,
    M1ServingPhysicalRunnerPublishedStateV1, M1ServingPhysicalRunnerPublishedV1,
    M1ServingPhysicalRunnerQuiescentStateV1, M1ServingPhysicalRunnerQuiescentV1,
    M1ServingPhysicalRunnerReadbackV1, M1ServingPreparedSemanticEvidenceV1,
    M1StructuralDraftCatchupPendingV1,
};
use crate::m1_queue_rearm::structural_draft_catchup::{
    prepare_structural_draft_catchup_v1, prepare_structural_speculative_rearm_v1,
    submit_structural_draft_catchup_v1, submit_structural_restore_v1,
    StructuralDraftCatchupScratchV1, StructuralRestoreScratchV1, StructuralTransitionStorageV1,
};
use crate::{M1FullStepWorkspacePlans, M1ServingQueueActionV1};

fn structural_first_logical_span_is_unreserved(
    committed: Option<u32>,
    resident: Option<u32>,
) -> bool {
    committed == Some(1) && resident == Some(1)
}

/// Independent, canonical workspace-plan copies for one bounded structural continuation.
#[derive(Debug)]
pub struct M1StructuralResidentRoundInputV1 {
    speculative_preparation: M1FullStepWorkspacePlans,
    speculative_recipe: M1FullStepWorkspacePlans,
    catchup_preparation: M1FullStepWorkspacePlans,
    catchup_recipe: M1FullStepWorkspacePlans,
}

impl M1StructuralResidentRoundInputV1 {
    /// Retains addressless plans; admission occurs against the joined physical owner.
    #[must_use]
    pub fn new(
        speculative_preparation: M1FullStepWorkspacePlans,
        speculative_recipe: M1FullStepWorkspacePlans,
        catchup_preparation: M1FullStepWorkspacePlans,
        catchup_recipe: M1FullStepWorkspacePlans,
    ) -> Self {
        Self {
            speculative_preparation,
            speculative_recipe,
            catchup_preparation,
            catchup_recipe,
        }
    }
}

/// The original serving commit and its optional draft-maintenance obligation.
#[must_use = "the joined physical owner must continue or terminate"]
#[derive(Debug)]
pub struct M1StructuralResidentCommittedRoundV1 {
    committed: M1ServingCommittedSpeculativeRoundV1<M1ServingPhysicalRunnerQuiescentV1>,
    pending: Option<M1StructuralDraftCatchupPendingV1>,
}

impl M1StructuralResidentCommittedRoundV1 {
    pub const fn outcome(&self) -> &crate::M1SpeculativeRoundOutcomeV1 {
        self.committed.outcome()
    }

    #[must_use]
    pub const fn requires_draft_catchup(&self) -> bool {
        self.pending.is_some()
    }

    /// Independent model-choice captures joined by the actual serving readbacks.
    pub fn diagnostic_history(&self) -> &M1ServingPhysicalRunnerDiagnosticHistoryV1 {
        self.committed.quiescent().diagnostic_history()
    }

    /// Counts genuinely completed maintenance transitions retained in physical history.
    #[must_use]
    pub fn completed_draft_catchup_count(&self) -> usize {
        match &self.committed.quiescent().state {
            M1ServingPhysicalRunnerQuiescentStateV1::Rearmed { released, .. } => (0..released
                .round_history_len())
                .filter(|&index| {
                    released.round_history(index).is_some_and(
                        crate::M1RearmRoundHistoryEntryV1::has_structural_draft_catchup,
                    )
                })
                .count(),
            M1ServingPhysicalRunnerQuiescentStateV1::First { .. }
            | M1ServingPhysicalRunnerQuiescentStateV1::Unscheduled { .. } => 0,
        }
    }
}

/// Fail-closed structural custody, including the provider and any live reservation.
#[must_use = "failed physical custody must remain retained"]
pub struct M1StructuralResidentFailureV1<'a> {
    stage: &'static str,
    diagnostic: Option<crate::m1_queue_rearm::M1QueueRearmKvReservationDiagnosticV1>,
    retained: Box<dyn fmt::Debug + 'a>,
}

impl fmt::Debug for M1StructuralResidentFailureV1<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _ = &self.retained;
        formatter
            .debug_struct("M1StructuralResidentFailureV1")
            .field("stage", &self.stage)
            .field("diagnostic", &self.diagnostic)
            .field("custody_retained", &true)
            .finish_non_exhaustive()
    }
}

impl fmt::Display for M1StructuralResidentFailureV1<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, formatter)
    }
}

impl std::error::Error for M1StructuralResidentFailureV1<'_> {}

impl<'a, const C: usize>
    M1ServingPhysicalRunnerOperationsV1<'a, C, M1QueuedServingPhysicalInputProviderV1>
{
    fn structural_failure(
        &mut self,
        stage: &'static str,
        retained: impl fmt::Debug + 'a,
    ) -> M1StructuralResidentFailureV1<'a> {
        self.engine.quarantine_m1_queue_rearm_failure();
        self.phase = M1ServingPhysicalRunnerAdapterPhaseV1::Sealed;
        M1StructuralResidentFailureV1 {
            stage,
            diagnostic: None,
            retained: Box::new((retained, self.provider.take())),
        }
    }

    /// Reserves the first speculative logical span from this adapter's real paired-prefill custody.
    ///
    /// Paired prefill commits one logical output anchor while both physical KV
    /// roles commit all 128 input tokens. Physical context bounds are checked
    /// against those KV cursors, not the separate logical completion count.
    ///
    /// # Errors
    /// Rejects any unrelated owner, registry batch, cursor or Engine phase without mutation.
    pub fn reserve_structural_first_speculative_span(
        &mut self,
        physical: &crate::M1ServingPhysicalQueueCustodyV1<M1ServingPhysicalRunnerQuiescentV1>,
        batch: &M1ServingBatchPlanV1,
    ) -> Result<(), M1ServingPhysicalRunnerOperationErrorV1> {
        let crate::M1ServingPhysicalQueueCustodyV1::Quiescent { plan, custody } = physical else {
            return Err(M1ServingPhysicalRunnerOperationErrorV1::PlanMismatch);
        };
        validate_custody_guard(
            self.identity,
            custody.adapter_identity(),
            self.phase
                == M1ServingPhysicalRunnerAdapterPhaseV1::Quiescent {
                    epoch: custody.epoch(),
                },
        )?;
        let M1ServingPhysicalRunnerQuiescentStateV1::First { released, .. } = &custody.state else {
            return Err(M1ServingPhysicalRunnerOperationErrorV1::PlanMismatch);
        };
        let [crate::M1ReleasedDeviceKvMemberV1::Active(cache)] = released.members() else {
            return Err(M1ServingPhysicalRunnerOperationErrorV1::PlanMismatch);
        };
        let projection = cache.projection();
        let width = batch
            .plan()
            .target()
            .bucket
            .dimensions(batch.plan().target().role, batch.plan().target().mode)
            .map_or(0, |d| d.active_tokens);
        if self.active_plan != Some(*plan)
            || batch.requests() != [projection.request]
            || custody.epoch().value().checked_add(1) != Some(batch.epoch().value())
            || !matches!(batch.action(), M1ServingQueueActionV1::QuiescentRollover { prior, next, .. } if prior == *plan && next == batch.plan())
            || !matches!(width, 5 | 9 | 17)
            || projection.target.committed_tokens != projection.draft.committed_tokens
            || projection.target.committed_tokens != projection.target.resident_tokens
            || projection.draft.committed_tokens != projection.draft.resident_tokens
            || projection
                .target
                .committed_tokens
                .checked_add(width)
                .is_none_or(|end| end > ferric_spec::M1_MAX_CONTEXT_TOKENS)
            || self.engine.state(projection.request)
                != Some(ferric_spec::scheduling::RequestState::Ready)
            || !structural_first_logical_span_is_unreserved(
                self.engine.committed_tokens(projection.request),
                self.engine.resident_tokens(projection.request),
            )
        {
            return Err(M1ServingPhysicalRunnerOperationErrorV1::PlanMismatch);
        }
        self.engine
            .append_tentative(projection.request, width)
            .map_err(|_| M1ServingPhysicalRunnerOperationErrorV1::SameShapeSchedule)
    }

    /// Commits real checked speculative output and registers a full-acceptance obligation atomically.
    ///
    /// # Errors
    /// Any failed join seals this operations owner while retaining the physical failure or commit.
    pub fn commit_structural_resident_round(
        &mut self,
        readback: M1ServingPhysicalReadbackV1<M1ServingPhysicalRunnerReadbackV1>,
        registry: &mut crate::M1ServingRegistryV1<C>,
        coordinator: &mut crate::M1SpeculativeGenerationLoopV1,
        permit: crate::M1SpeculativePreflightedRoundV1,
    ) -> Result<M1StructuralResidentCommittedRoundV1, M1StructuralResidentFailureV1<'a>> {
        if let Err(error) = self.checked_completion_for_readback(&readback) {
            return Err(self.structural_failure("readback owner join", (error, readback, permit)));
        }
        let committed = match readback.commit_speculative(registry, coordinator, permit, self) {
            Ok(committed) => committed,
            Err(error) => return Err(self.structural_failure("speculative commit", error)),
        };
        let pending = match joined_structural_draft_catchup_pending(coordinator, &committed) {
            Ok(pending) => pending,
            Err(()) => {
                return Err(self.structural_failure("maintenance obligation join", committed))
            }
        };
        if let Some(witness) = &pending {
            if let Err(error) = registry.register_structural_draft_catchup_pending(witness) {
                return Err(self
                    .structural_failure("maintenance registration", (error, committed, pending)));
            }
        }
        Ok(M1StructuralResidentCommittedRoundV1 { committed, pending })
    }

    /// Advances one real active singleton, executing draft maintenance first after full acceptance.
    ///
    /// The supplied absolute deadline bounds every phase. All failures seal the
    /// adapter and retain both model caches, the exact queue and any outstanding reservation.
    ///
    /// # Errors
    /// Rejects terminal outcomes, cross-owner inputs, unsupported plans or any physical failure.
    pub fn continue_structural_resident_round(
        &mut self,
        current: M1StructuralResidentCommittedRoundV1,
        registry: &mut crate::M1ServingRegistryV1<C>,
        coordinator: &mut crate::M1SpeculativeGenerationLoopV1,
        input: M1StructuralResidentRoundInputV1,
        deadline: std::time::Instant,
    ) -> Result<
        (
            crate::M1ServingPhysicalPublishedV1<M1ServingPhysicalRunnerPublishedV1>,
            crate::M1SpeculativeRoundOutcomeV1,
        ),
        M1StructuralResidentFailureV1<'a>,
    > {
        let mut deadline_expired = || std::time::Instant::now() >= deadline;
        let runner = self.runner;
        let quiescent = current.committed.quiescent();
        let parent = current.committed.plan().target();
        let outcome = current.outcome();
        if deadline_expired()
            || self.engine.is_faulted()
            || self
                .provider
                .as_ref()
                .is_none_or(|provider| provider.pending_generation_count() != 0)
            || self.identity != quiescent.adapter_identity()
            || current
                .pending
                .as_ref()
                .is_some_and(|pending| pending.adapter_identity != self.identity)
            || self.phase
                != (M1ServingPhysicalRunnerAdapterPhaseV1::Quiescent {
                    epoch: quiescent.epoch(),
                })
            || self.active_plan != Some(current.committed.plan())
            || outcome.coordinator_identity() != coordinator.identity()
            || outcome.members().len() != 1
            || outcome.next_active_roster().len() != 1
            || outcome.members()[0].status() != crate::M1SpeculativeMemberStatusV1::Active
            || input.speculative_preparation != input.speculative_recipe
            || input.catchup_preparation != input.catchup_recipe
            || input.speculative_preparation.target().selection() != parent
        {
            return Err(self.structural_failure("continuation preflight", (current, input)));
        }
        let member = outcome.members()[0];
        let Some(anchor) = member.next_draft_anchor() else {
            return Err(
                self.structural_failure("missing real continuation token", (current, input))
            );
        };
        let committed_cursor = member.target_settlement().commit_end();
        if current.pending.is_none() && member.draft_settlement().commit_end() != committed_cursor {
            return Err(self.structural_failure("unequal ordinary cursors", (current, input)));
        }
        let Some(mut speculative_scratch) =
            StructuralRestoreScratchV1::try_new(parent, &input.speculative_preparation)
        else {
            return Err(self.structural_failure("speculative scratch", (current, input)));
        };
        let M1StructuralResidentRoundInputV1 {
            speculative_preparation,
            speculative_recipe,
            catchup_preparation,
            catchup_recipe,
        } = input;
        let speculative_recipe = match self.runner.derive_step_recipe(
            crate::M1StepDispatchIntent::SpeculativeRound(parent),
            speculative_recipe,
        ) {
            crate::M1PhysicalRunnerRecipeOutcomeV1::Prepared(recipe) => recipe,
            crate::M1PhysicalRunnerRecipeOutcomeV1::Rejected(error) => {
                return Err(self.structural_failure(
                    "speculative recipe",
                    (
                        error,
                        current,
                        speculative_preparation,
                        catchup_preparation,
                        catchup_recipe,
                        speculative_scratch,
                    ),
                ))
            }
        };
        let batch = match registry.plan_next() {
            Ok(Some(batch)) => batch,
            result => {
                return Err(self.structural_failure(
                    "next registry plan",
                    (
                        result,
                        current,
                        speculative_preparation,
                        speculative_recipe,
                        catchup_preparation,
                        catchup_recipe,
                        speculative_scratch,
                    ),
                ))
            }
        };
        let request = member.request();
        if batch.plan().target() != parent
            || batch.requests() != [request]
            || quiescent.epoch().value().checked_add(1) != Some(batch.epoch().value())
        {
            return Err(self.structural_failure(
                "next registry owner",
                (
                    batch,
                    current,
                    speculative_preparation,
                    speculative_recipe,
                    catchup_preparation,
                    catchup_recipe,
                    speculative_scratch,
                ),
            ));
        }
        let (maintenance_prepared, enter_storage, restore_storage) = if current.pending.is_some() {
            let Some(scratch) =
                StructuralDraftCatchupScratchV1::try_new(parent, &catchup_preparation)
            else {
                return Err(self.structural_failure(
                    "maintenance scratch",
                    (
                        batch,
                        current,
                        speculative_preparation,
                        speculative_recipe,
                        catchup_preparation,
                        catchup_recipe,
                        speculative_scratch,
                    ),
                ));
            };
            let recipe = match self.runner.derive_step_recipe(
                crate::M1StepDispatchIntent::DraftCatchup(parent),
                catchup_recipe,
            ) {
                crate::M1PhysicalRunnerRecipeOutcomeV1::Prepared(recipe) => recipe,
                crate::M1PhysicalRunnerRecipeOutcomeV1::Rejected(error) => {
                    return Err(self.structural_failure(
                        "maintenance recipe",
                        (
                            error,
                            batch,
                            current,
                            speculative_preparation,
                            speculative_recipe,
                            catchup_preparation,
                            scratch,
                            speculative_scratch,
                        ),
                    ))
                }
            };
            let enter = StructuralTransitionStorageV1::try_new(parent, true, &recipe);
            let restore =
                StructuralTransitionStorageV1::try_new(parent, false, &speculative_recipe);
            let (Some(enter), Some(restore)) = (enter, restore) else {
                return Err(self.structural_failure(
                    "transition scratch",
                    (
                        batch,
                        current,
                        speculative_preparation,
                        speculative_recipe,
                        catchup_preparation,
                        recipe,
                        scratch,
                        speculative_scratch,
                    ),
                ));
            };
            (
                Some((catchup_preparation, recipe, scratch)),
                Some(enter),
                Some(restore),
            )
        } else {
            (None, None, None)
        };
        if deadline_expired() {
            return Err(self.structural_failure(
                "deadline before scheduling",
                (
                    batch,
                    current,
                    speculative_preparation,
                    speculative_recipe,
                    speculative_scratch,
                    maintenance_prepared,
                    enter_storage,
                    restore_storage,
                ),
            ));
        }
        let reservation = match &current.pending {
            Some(pending) => registry.reserve_structural_draft_catchup_publication(batch, pending),
            None => registry.reserve_publication(batch),
        };
        let reservation = match reservation {
            Ok(reservation) => reservation,
            Err(error) => {
                return Err(self.structural_failure(
                    "publication reservation",
                    (
                        error,
                        current,
                        speculative_preparation,
                        speculative_recipe,
                        speculative_scratch,
                        maintenance_prepared,
                        enter_storage,
                        restore_storage,
                    ),
                ))
            }
        };
        let batch = reservation.physical_batch();
        if current.pending.is_none() {
            let width = parent
                .bucket
                .dimensions(parent.role, parent.mode)
                .map_or(0, |d| d.active_tokens);
            if let Err(error) = self.engine.append_tentative(request, width) {
                return Err(self.structural_failure(
                    "ordinary target span",
                    (
                        error,
                        reservation,
                        current,
                        speculative_preparation,
                        speculative_recipe,
                        speculative_scratch,
                    ),
                ));
            }
        }
        let M1StructuralResidentCommittedRoundV1 { committed, pending } = current;
        let (_, physical, outcome) = committed.into_parts();
        let crate::M1ServingPhysicalQueueCustodyV1::Quiescent { plan, custody } = physical else {
            return Err(self.structural_failure(
                "continuation physical owner",
                (
                    (physical, outcome, reservation, pending),
                    (
                        speculative_preparation,
                        speculative_recipe,
                        speculative_scratch,
                        maintenance_prepared,
                        enter_storage,
                        restore_storage,
                    ),
                ),
            ));
        };
        let M1ServingPhysicalRunnerQuiescentV1 { state, .. } = custody;
        let (scheduled, diagnostic_history) = match state {
            M1ServingPhysicalRunnerQuiescentStateV1::First {
                released,
                diagnostic_history,
            } => (
                schedule_m1_long_lived_queue_rearm_exact_v1(
                    self.engine,
                    released,
                    batch.epoch(),
                    batch.requests(),
                ),
                diagnostic_history,
            ),
            M1ServingPhysicalRunnerQuiescentStateV1::Rearmed {
                released,
                diagnostic_history,
            } => (
                released.schedule_next_exact(self.engine, batch.epoch(), batch.requests()),
                diagnostic_history,
            ),
            M1ServingPhysicalRunnerQuiescentStateV1::Unscheduled {
                unscheduled,
                diagnostic_history,
            } => (
                unscheduled.retry_exact(self.engine, batch.epoch(), batch.requests()),
                diagnostic_history,
            ),
        };
        let scheduled = match scheduled {
            Ok(scheduled) => scheduled,
            Err(error) => {
                return Err(self.structural_failure(
                    "exact resident scheduling",
                    (
                        (error, reservation, pending, outcome, diagnostic_history),
                        (
                            speculative_preparation,
                            speculative_recipe,
                            speculative_scratch,
                            maintenance_prepared,
                            enter_storage,
                            restore_storage,
                        ),
                    ),
                ))
            }
        };
        let (published, reservation) = if let Some(pending) = pending {
            let (
                (catchup_preparation, catchup_recipe, mut catchup_scratch),
                mut enter_storage,
                mut restore_storage,
            ) = match (maintenance_prepared, enter_storage, restore_storage) {
                (Some(prepared), Some(enter), Some(restore)) => (prepared, enter, restore),
                retained => {
                    return Err(self.structural_failure(
                        "missing maintenance storage",
                        (
                            (scheduled, reservation, pending, outcome, diagnostic_history),
                            (
                                speculative_preparation,
                                speculative_recipe,
                                speculative_scratch,
                                retained,
                            ),
                        ),
                    ))
                }
            };
            if deadline_expired() {
                return Err(self.structural_failure(
                    "deadline before maintenance preparation",
                    (
                        (scheduled, reservation, pending, outcome, diagnostic_history),
                        (
                            catchup_preparation,
                            catchup_recipe,
                            catchup_scratch,
                            speculative_preparation,
                            speculative_recipe,
                            speculative_scratch,
                            enter_storage,
                            restore_storage,
                        ),
                    ),
                ));
            }
            let prepared = match prepare_structural_draft_catchup_v1(
                scheduled,
                &pending,
                self.runner.logical_runner(),
                catchup_preparation,
                &mut catchup_scratch,
            ) {
                Ok(prepared) => prepared,
                Err(error) => {
                    return Err(self.structural_failure(
                        "maintenance preparation",
                        (
                            (error, reservation, pending, outcome, diagnostic_history),
                            (
                                catchup_recipe,
                                catchup_scratch,
                                speculative_preparation,
                                speculative_recipe,
                                speculative_scratch,
                                enter_storage,
                                restore_storage,
                            ),
                        ),
                    ))
                }
            };
            let catalog = match runner.structural_resident_program_catalog() {
                Ok(catalog) => catalog,
                Err(error) => {
                    return Err(self.structural_failure(
                        "maintenance catalog",
                        (
                            (
                                error,
                                prepared,
                                reservation,
                                pending,
                                outcome,
                                diagnostic_history,
                            ),
                            (
                                catchup_recipe,
                                catchup_scratch,
                                speculative_preparation,
                                speculative_recipe,
                                speculative_scratch,
                                enter_storage,
                                restore_storage,
                            ),
                        ),
                    ))
                }
            };
            if deadline_expired() {
                return Err(self.structural_failure(
                    "deadline before maintenance publication",
                    (
                        (
                            prepared,
                            reservation,
                            pending,
                            outcome,
                            diagnostic_history,
                            catalog,
                        ),
                        (
                            catchup_recipe,
                            catchup_scratch,
                            speculative_preparation,
                            speculative_recipe,
                            speculative_scratch,
                            enter_storage,
                            restore_storage,
                        ),
                    ),
                ));
            }
            let published = match submit_structural_draft_catchup_v1(
                prepared,
                catchup_recipe,
                catalog,
                &pending,
                self.ring_bytes,
                &mut enter_storage,
            ) {
                Ok(published) => published,
                Err(error) => {
                    return Err(self.structural_failure(
                        "maintenance publication",
                        (
                            (error, reservation, pending, outcome, diagnostic_history),
                            (
                                catchup_scratch,
                                speculative_preparation,
                                speculative_recipe,
                                speculative_scratch,
                                enter_storage,
                                restore_storage,
                            ),
                        ),
                    ))
                }
            };
            if published.scheduled_dispatch().epoch() != batch.epoch()
                || published.scheduled_dispatch().member_count() != 1
                || published.scheduled_dispatch().member(0) != Some(request)
            {
                return Err(self.structural_failure(
                    "maintenance scheduler join",
                    (
                        (published, reservation, pending, outcome, diagnostic_history),
                        (
                            catchup_scratch,
                            speculative_preparation,
                            speculative_recipe,
                            speculative_scratch,
                            enter_storage,
                            restore_storage,
                        ),
                    ),
                ));
            }
            if let Err(error) = registry.record_publication(reservation) {
                return Err(self.structural_failure(
                    "maintenance registry publication",
                    (
                        (error, published, pending, outcome, diagnostic_history),
                        (
                            catchup_scratch,
                            speculative_preparation,
                            speculative_recipe,
                            speculative_scratch,
                            enter_storage,
                            restore_storage,
                        ),
                    ),
                ));
            }
            let remaining_ms = u32::try_from(
                deadline
                    .saturating_duration_since(std::time::Instant::now())
                    .as_millis(),
            )
            .unwrap_or(u32::MAX)
            .min(self.queue_wait_timeout.milliseconds());
            if remaining_ms == 0 {
                return Err(self.structural_failure(
                    "deadline before maintenance readback",
                    (
                        (published, pending, outcome, diagnostic_history),
                        (
                            catchup_scratch,
                            speculative_preparation,
                            speculative_recipe,
                            speculative_scratch,
                            enter_storage,
                            restore_storage,
                        ),
                    ),
                ));
            }
            let Some(readback_storage) = catchup_scratch.take_readback() else {
                return Err(self.structural_failure(
                    "missing maintenance readback storage",
                    (
                        (published, pending, outcome, diagnostic_history),
                        (
                            catchup_scratch,
                            speculative_preparation,
                            speculative_recipe,
                            speculative_scratch,
                            enter_storage,
                            restore_storage,
                        ),
                    ),
                ));
            };
            let released = match published.read_and_settle(
                self.engine,
                self.runner.logical_runner(),
                pending,
                remaining_ms,
                readback_storage,
            ) {
                Ok(released) => released,
                Err(error) => {
                    return Err(self.structural_failure(
                        "maintenance completion",
                        (
                            error,
                            outcome,
                            diagnostic_history,
                            catchup_scratch,
                            speculative_preparation,
                            speculative_recipe,
                            speculative_scratch,
                            enter_storage,
                            restore_storage,
                        ),
                    ))
                }
            };
            if let Err(error) = coordinator.commit_structural_draft_catchup(released.completed()) {
                return Err(self.structural_failure(
                    "maintenance coordinator join",
                    (
                        error,
                        released,
                        outcome,
                        diagnostic_history,
                        speculative_preparation,
                        speculative_recipe,
                        speculative_scratch,
                        restore_storage,
                    ),
                ));
            }
            if let Err(error) = registry.complete_structural_draft_catchup(released.completed()) {
                return Err(self.structural_failure(
                    "maintenance registry completion",
                    (
                        error,
                        released,
                        outcome,
                        diagnostic_history,
                        speculative_preparation,
                        speculative_recipe,
                        speculative_scratch,
                        restore_storage,
                    ),
                ));
            }
            if deadline_expired() {
                let closed = released.close(self.engine);
                return Err(self.structural_failure(
                    "deadline after maintenance completion",
                    (
                        closed,
                        outcome,
                        diagnostic_history,
                        speculative_preparation,
                        speculative_recipe,
                        speculative_scratch,
                        restore_storage,
                    ),
                ));
            }
            let restore_batch = match registry.plan_next() {
                Ok(Some(batch)) => batch,
                result => {
                    return Err(self.structural_failure(
                        "restore registry plan",
                        (
                            result,
                            released,
                            outcome,
                            diagnostic_history,
                            speculative_preparation,
                            speculative_recipe,
                            speculative_scratch,
                            restore_storage,
                        ),
                    ))
                }
            };
            let restore_reservation = match registry
                .reserve_structural_draft_catchup_restore_publication(
                    restore_batch,
                    released.completed(),
                ) {
                Ok(reservation) => reservation,
                Err(error) => {
                    return Err(self.structural_failure(
                        "restore reservation",
                        (
                            error,
                            released,
                            outcome,
                            diagnostic_history,
                            speculative_preparation,
                            speculative_recipe,
                            speculative_scratch,
                            restore_storage,
                        ),
                    ))
                }
            };
            let prepared = match released.prepare_restore(
                self.engine,
                self.runner.logical_runner(),
                speculative_preparation,
                &mut speculative_scratch,
                &mut deadline_expired,
            ) {
                Ok(prepared) => prepared,
                Err(error) => {
                    return Err(self.structural_failure(
                        "restore preparation",
                        (
                            error,
                            restore_reservation,
                            outcome,
                            diagnostic_history,
                            speculative_recipe,
                            speculative_scratch,
                            restore_storage,
                        ),
                    ))
                }
            };
            let catalog = match runner.structural_resident_program_catalog() {
                Ok(catalog) => catalog,
                Err(error) => {
                    return Err(self.structural_failure(
                        "restore catalog",
                        (
                            error,
                            prepared,
                            restore_reservation,
                            outcome,
                            diagnostic_history,
                            speculative_recipe,
                            speculative_scratch,
                            restore_storage,
                        ),
                    ))
                }
            };
            if deadline_expired() {
                return Err(self.structural_failure(
                    "deadline before restore publication",
                    (
                        prepared,
                        restore_reservation,
                        outcome,
                        diagnostic_history,
                        speculative_recipe,
                        speculative_scratch,
                        restore_storage,
                        catalog,
                    ),
                ));
            }
            let published = match submit_structural_restore_v1(
                prepared,
                speculative_recipe,
                catalog,
                self.ring_bytes,
                &mut restore_storage,
            ) {
                Ok(published) => published,
                Err(error) => {
                    return Err(self.structural_failure(
                        "restore publication",
                        (
                            error,
                            restore_reservation,
                            outcome,
                            diagnostic_history,
                            speculative_scratch,
                            restore_storage,
                        ),
                    ))
                }
            };
            (published, restore_reservation)
        } else {
            let prepared = match prepare_structural_speculative_rearm_v1(
                self.engine,
                scheduled,
                self.runner.logical_runner(),
                speculative_preparation,
                anchor,
                committed_cursor,
                &mut speculative_scratch,
            ) {
                Ok(prepared) => prepared,
                Err(error) => {
                    let diagnostic = error.diagnostic();
                    let mut failure = self.structural_failure(
                        "ordinary speculative preparation",
                        (
                            error,
                            reservation,
                            outcome,
                            diagnostic_history,
                            speculative_recipe,
                            speculative_scratch,
                        ),
                    );
                    failure.diagnostic = diagnostic;
                    return Err(failure);
                }
            };
            if deadline_expired() {
                return Err(self.structural_failure(
                    "deadline before ordinary publication",
                    (
                        prepared,
                        reservation,
                        outcome,
                        diagnostic_history,
                        speculative_recipe,
                        speculative_scratch,
                    ),
                ));
            }
            match runner.submit_rearm(self.engine, prepared, speculative_recipe) {
                Ok(published) => (published, reservation),
                Err(error) => {
                    return Err(self.structural_failure(
                        "ordinary speculative publication",
                        (
                            error,
                            reservation,
                            outcome,
                            diagnostic_history,
                            speculative_scratch,
                        ),
                    ))
                }
            }
        };
        let epoch = published.scheduled_dispatch().epoch();
        self.phase = M1ServingPhysicalRunnerAdapterPhaseV1::Published { epoch };
        let published = M1ServingPhysicalRunnerPublishedV1 {
            adapter_identity: self.identity,
            epoch,
            plan,
            state: M1ServingPhysicalRunnerPublishedStateV1::Rearmed {
                published,
                semantic_evidence: M1ServingPreparedSemanticEvidenceV1::SpeculativeK4,
                diagnostic_history,
            },
        };
        match crate::m1_serving_physical_bridge::record_structural_resident_publication(
            published,
            reservation,
            registry,
            self,
        ) {
            Ok(published) => Ok((published, outcome)),
            Err(error) => {
                Err(self.structural_failure("speculative registry publication", (error, outcome)))
            }
        }
    }

    /// Reads a real published round using the remaining duration of the same absolute deadline.
    ///
    /// # Errors
    /// A deadline or readback failure retains the published or terminal queue custody.
    pub fn read_structural_resident_round(
        &mut self,
        published: crate::M1ServingPhysicalPublishedV1<M1ServingPhysicalRunnerPublishedV1>,
        deadline: std::time::Instant,
    ) -> Result<
        M1ServingPhysicalReadbackV1<M1ServingPhysicalRunnerReadbackV1>,
        M1StructuralResidentFailureV1<'a>,
    > {
        let remaining_ms = u32::try_from(
            deadline
                .saturating_duration_since(std::time::Instant::now())
                .as_millis(),
        )
        .unwrap_or(u32::MAX)
        .min(self.queue_wait_timeout.milliseconds());
        let Some(timeout) = crate::M1QueueWaitTimeoutV1::new(remaining_ms) else {
            return Err(self.structural_failure("deadline before speculative readback", published));
        };
        self.queue_wait_timeout = timeout;
        let epoch = published.epoch();
        match published.read_physical(epoch, self) {
            Ok(readback) => Ok(readback),
            Err(error) => Err(self.structural_failure("speculative readback", error)),
        }
    }

    /// Destroys the real all-terminal speculative queue without executing unnecessary catch-up.
    ///
    /// # Errors
    /// Retains every owner if the outcome, adapter or physical terminal roster disagrees.
    pub fn finish_structural_resident_round(
        &mut self,
        current: M1StructuralResidentCommittedRoundV1,
    ) -> Result<
        (
            crate::M1SpeculativeRoundOutcomeV1,
            crate::M1LongLivedQueueAllTerminalShutdownSuccessV1,
        ),
        M1StructuralResidentFailureV1<'a>,
    > {
        if current.pending.is_some()
            || !current.outcome().next_active_roster().is_empty()
            || current.committed.quiescent().adapter_identity() != self.identity
            || self.phase
                != (M1ServingPhysicalRunnerAdapterPhaseV1::Quiescent {
                    epoch: current.committed.quiescent().epoch(),
                })
        {
            return Err(self.structural_failure("terminal preflight", current));
        }
        let (_, physical, outcome) = current.committed.into_parts();
        let crate::M1ServingPhysicalQueueCustodyV1::Quiescent { custody, .. } = physical else {
            return Err(self.structural_failure("terminal physical owner", (physical, outcome)));
        };
        let M1ServingPhysicalRunnerQuiescentV1 {
            state:
                M1ServingPhysicalRunnerQuiescentStateV1::Rearmed {
                    released,
                    diagnostic_history,
                },
            ..
        } = custody
        else {
            return Err(
                self.structural_failure("terminal speculative queue shape", (custody, outcome))
            );
        };
        match released.shutdown_all_terminal_queue(self.engine) {
            Ok(shutdown) => {
                self.phase = M1ServingPhysicalRunnerAdapterPhaseV1::Sealed;
                drop(diagnostic_history);
                Ok((outcome, shutdown))
            }
            Err(error) => Err(self.structural_failure(
                "terminal queue teardown",
                (error, diagnostic_history, outcome),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::m1_serving_physical_operations::structural_draft_catchup_member_input;
    use crate::{
        CompletionWireExpectation, CompletionWireSemanticExpectation,
        M1ObservedSpeculativeDiagnosticChoicesV1, M1ScheduledDispatchV1,
    };
    use ferric_qwen_kernels::logits::Qwen3LogitsCompactRecordLayoutV1 as CompletionLayout;
    use ferric_spec::{
        completion::CompletionEpoch, Identity, Qwen3PlanBucket, Qwen3PlanSelection, RequestId,
        StepPlan, TokenId,
    };

    #[test]
    fn structural_resident_diagnostic_never_formats_or_drops_retained_custody() {
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        };
        struct Retained(Arc<AtomicUsize>);
        impl fmt::Debug for Retained {
            fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
                panic!("retained custody must not be formatted");
            }
        }
        impl Drop for Retained {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }
        let drops = Arc::new(AtomicUsize::new(0));
        let failure = M1StructuralResidentFailureV1 {
            stage: "ordinary speculative preparation",
            diagnostic: None,
            retained: Box::new(Retained(Arc::clone(&drops))),
        };
        for text in [format!("{failure:?}"), format!("{failure}")] {
            assert!(text.len() < 1024);
            assert!(text.contains("ordinary speculative preparation"));
            assert!(text.contains("custody_retained"));
        }
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        drop(failure);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn structural_first_span_rejects_duplicate_or_prompt_length_logical_reservation() {
        assert!(structural_first_logical_span_is_unreserved(
            Some(1),
            Some(1)
        ));
        for width in [5, 9, 17] {
            let committed = Some(1);
            let resident = Some(1 + width);
            let before = (committed, resident);
            assert!(!structural_first_logical_span_is_unreserved(
                committed, resident
            ));
            assert_eq!((committed, resident), before);
        }
        for (committed, resident) in [
            (None, None),
            (Some(0), Some(1)),
            (Some(128), Some(128)),
            (Some(1), Some(128)),
            (Some(1), None),
        ] {
            assert!(!structural_first_logical_span_is_unreserved(
                committed, resident
            ));
        }
    }

    fn parent(k: u8) -> Qwen3PlanSelection {
        Qwen3PlanSelection {
            role: ferric_spec::Qwen3ModelRole::Target8B,
            mode: ferric_spec::Qwen3ExecutionMode::Speculative,
            bucket: match k {
                4 => Qwen3PlanBucket::SpeculativeS1K4C8192,
                8 => Qwen3PlanBucket::SpeculativeS1K8C8192,
                16 => Qwen3PlanBucket::SpeculativeS1K16C8192,
                _ => panic!("singleton K"),
            },
        }
    }

    // Inert host completion data only; this creates no native queue or read lease.
    fn member_fixture(
        k: u8,
        accepted: u8,
        committed: u32,
        cap: u32,
        cancel: bool,
        stop: bool,
    ) -> (
        crate::M1SpeculativeRoundOutcomeV1,
        M1ObservedSpeculativeDiagnosticChoicesV1,
    ) {
        let selection = parent(k);
        let request = RequestId::new(0, 1);
        let epoch = CompletionEpoch::new(1);
        let plan_id = Identity::new([0x69; 32]);
        let candidates: Vec<TokenId> = (0..u32::from(k)).map(|index| 100 + index).collect();
        let mut target = candidates.clone();
        target.push(900);
        if accepted < k {
            target[usize::from(accepted)] = 800;
        }
        let mut emitted = candidates[..usize::from(accepted)].to_vec();
        emitted.push(target[usize::from(accepted)]);
        let mut bytes = vec![0_u8; CompletionLayout::RECORD_BYTES_USIZE];
        bytes[CompletionLayout::REQUEST_SLOT_OFFSET..CompletionLayout::REQUEST_SLOT_OFFSET + 4]
            .copy_from_slice(&request.slot().to_le_bytes());
        bytes[CompletionLayout::REQUEST_GENERATION_OFFSET
            ..CompletionLayout::REQUEST_GENERATION_OFFSET + 4]
            .copy_from_slice(&request.generation().to_le_bytes());
        bytes[CompletionLayout::COMPLETION_EPOCH_OFFSET
            ..CompletionLayout::COMPLETION_EPOCH_OFFSET + 8]
            .copy_from_slice(&epoch.value().to_le_bytes());
        bytes[CompletionLayout::PLAN_IDENTITY_OFFSET
            ..CompletionLayout::PLAN_IDENTITY_OFFSET + CompletionLayout::PLAN_IDENTITY_BYTES]
            .copy_from_slice(plan_id.as_bytes());
        bytes[CompletionLayout::ACCEPTED_DRAFT_TOKENS_OFFSET] = accepted;
        bytes[CompletionLayout::EMITTED_TOKEN_COUNT_OFFSET] = emitted.len().try_into().unwrap();
        for (index, token) in emitted.into_iter().enumerate() {
            let offset = CompletionLayout::token_offset(index).unwrap();
            bytes[offset..offset + 4].copy_from_slice(&token.to_le_bytes());
        }
        let scheduled = M1ScheduledDispatchV1::for_test(epoch, &[request]);
        let plan = StepPlan::new(request, epoch, plan_id, selection);
        let observed = crate::M1ObservedCompletionImageV1::from_bytes_for_test(
            crate::m1_completion_output_shape_v1(selection).unwrap(),
            selection,
            &scheduled,
            7,
            5,
            384,
            bytes.into_boxed_slice(),
        )
        .unwrap();
        let expectations = [CompletionWireExpectation::new(
            &plan,
            CompletionWireSemanticExpectation::Speculative {
                draft_tokens: &candidates,
                target_choices: &target,
            },
        )];
        let checked = crate::completed_readback_join::check_m1_completed_output_v1(
            &observed,
            selection,
            &scheduled,
            &expectations,
        )
        .unwrap();
        let stop_tokens = if stop {
            vec![candidates[usize::from(k - 1)]]
        } else {
            Vec::new()
        };
        let policy = crate::M1SpeculativeGenerationPolicyV1::new(cap, &stop_tokens).unwrap();
        let seed = crate::M1SpeculativeMemberSeedV1::new(request, 50, committed, committed, policy);
        let mut coordinator =
            crate::M1SpeculativeGenerationLoopV1::new(selection, &[seed]).unwrap();
        let binding = coordinator.bind_round(0, epoch, &[request]).unwrap();
        let control = if cancel {
            crate::M1SpeculativeMemberControlV1::cancelling(
                request,
                crate::M1SpeculativeCancellationReasonV1::ServerShutdown,
            )
        } else {
            crate::M1SpeculativeMemberControlV1::continuing(request)
        };
        let outcome = coordinator
            .complete_checked_round(binding, &checked, &[control])
            .unwrap();
        let choices = crate::speculative_diagnostic_choices::synthetic_observed_choices_for_test(
            selection,
            7,
            &candidates,
            &target,
        );
        (outcome, choices)
    }

    #[test]
    fn structural_catchup_all_acceptances_use_last_checked_candidate_only_for_active_full_round() {
        for k in [4, 8, 16] {
            for accepted in 0..=k {
                let (outcome, choices) = member_fixture(k, accepted, 128, 64, false, false);
                let input = structural_draft_catchup_member_input(
                    parent(k),
                    &outcome.members()[0],
                    &choices,
                )
                .unwrap();
                if accepted == k {
                    let input = input.expect("active full acceptance requires one draft write");
                    assert_eq!(input.draft_committed, 128 + u32::from(k));
                    assert_eq!(input.target_committed, input.draft_committed + 1);
                    assert_eq!(input.token, 100 + u32::from(k) - 1);
                    assert_eq!(input.next_anchor, 900);
                    assert_ne!(input.token, input.next_anchor);
                    assert_ne!(input.token, 50);
                } else {
                    assert_eq!(input, None);
                    assert_eq!(
                        outcome.members()[0].target_settlement().commit_end(),
                        outcome.members()[0].draft_settlement().commit_end()
                    );
                }
            }
        }
    }

    #[test]
    fn structural_catchup_terminal_full_acceptance_skips_output_limit_cancel_and_stop() {
        for k in [4, 8, 16] {
            for cap in [1, u32::from(k), u32::from(k) + 1] {
                let (outcome, choices) = member_fixture(k, k, 128, cap, false, false);
                assert!(outcome.next_active_roster().is_empty());
                assert_eq!(
                    structural_draft_catchup_member_input(
                        parent(k),
                        &outcome.members()[0],
                        &choices
                    ),
                    Ok(None)
                );
            }
            for (cancel, stop) in [(true, false), (false, true)] {
                let (outcome, choices) = member_fixture(k, k, 128, 64, cancel, stop);
                assert!(outcome.next_active_roster().is_empty());
                assert_eq!(
                    structural_draft_catchup_member_input(
                        parent(k),
                        &outcome.members()[0],
                        &choices
                    ),
                    Ok(None)
                );
            }
        }
    }

    #[test]
    fn structural_catchup_boundaries_preserve_end_exclusive_missing_position() {
        for k in [4, 8, 16] {
            for committed in [127, 128, 143, 144, 8192 - u32::from(k) - 1] {
                let (outcome, choices) = member_fixture(k, k, committed, 64, false, false);
                let input = structural_draft_catchup_member_input(
                    parent(k),
                    &outcome.members()[0],
                    &choices,
                )
                .unwrap()
                .unwrap();
                assert_eq!(input.draft_committed, committed + u32::from(k));
                assert_eq!(input.target_committed, committed + u32::from(k) + 1);
                assert!(input.target_committed <= 8192);
            }
        }
    }

    #[test]
    fn structural_catchup_rejects_last_choice_generation_and_parent_substitution() {
        for k in [4, 8, 16] {
            let (outcome, choices) = member_fixture(k, k, 128, 64, false, false);
            let member = &outcome.members()[0];
            let mut candidates = choices.draft_choices_for_lane(0).unwrap().to_vec();
            let target = choices.target_choices_for_lane(0).unwrap();
            candidates[usize::from(k - 1)] = 900;
            let wrong_last =
                crate::speculative_diagnostic_choices::synthetic_observed_choices_for_test(
                    parent(k),
                    7,
                    &candidates,
                    target,
                );
            assert!(structural_draft_catchup_member_input(parent(k), member, &wrong_last).is_err());
            let zero_generation =
                crate::speculative_diagnostic_choices::synthetic_observed_choices_for_test(
                    parent(k),
                    0,
                    choices.draft_choices_for_lane(0).unwrap(),
                    target,
                );
            assert!(
                structural_draft_catchup_member_input(parent(k), member, &zero_generation).is_err()
            );
            let mut wrong_parent = parent(k);
            wrong_parent.role = ferric_spec::Qwen3ModelRole::Draft06B;
            assert!(structural_draft_catchup_member_input(wrong_parent, member, &choices).is_err());
            assert!(structural_draft_catchup_member_input(
                parent(if k == 4 { 8 } else { 4 }),
                member,
                &choices
            )
            .is_err());
        }
    }
}
