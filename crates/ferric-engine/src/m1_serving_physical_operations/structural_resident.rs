//! Real structural resident transitions. These owners never grant authenticated admission.

use super::*;
use crate::m1_queue_rearm::structural_draft_catchup::{
    prepare_structural_draft_catchup_v1, prepare_structural_speculative_rearm_v1,
    submit_structural_draft_catchup_v1, submit_structural_restore_v1,
    StructuralDraftCatchupScratchV1, StructuralRestoreScratchV1, StructuralTransitionStorageV1,
};

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
    #[must_use]
    pub const fn outcome(&self) -> &crate::M1SpeculativeRoundOutcomeV1 {
        self.committed.outcome()
    }

    #[must_use]
    pub const fn requires_draft_catchup(&self) -> bool {
        self.pending.is_some()
    }
}

/// Fail-closed structural custody, including the provider and any live reservation.
#[must_use = "failed physical custody must remain retained"]
#[derive(Debug)]
pub struct M1StructuralResidentFailureV1<'a> {
    stage: &'static str,
    retained: Box<dyn fmt::Debug + 'a>,
}

impl fmt::Display for M1StructuralResidentFailureV1<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "structural resident {}: {:?}",
            self.stage, self.retained
        )
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
            retained: Box::new((retained, self.provider.take())),
        }
    }

    /// Reserves the first speculative logical span from this adapter's real paired-prefill custody.
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
            || !projection
                .target
                .committed_tokens
                .checked_add(width)
                .is_some_and(|end| end <= ferric_spec::M1_MAX_CONTEXT_TOKENS)
            || self.engine.state(projection.request) != Some(crate::RequestState::Ready)
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
        let (maintenance_prepared, mut enter_storage, mut restore_storage) =
            if current.pending.is_some() {
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
            unreachable!("joined commit retains quiescent physical custody")
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
            let (catchup_preparation, catchup_recipe, mut catchup_scratch) =
                maintenance_prepared.expect("full acceptance prepared maintenance");
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
                enter_storage
                    .as_mut()
                    .expect("preclock maintenance storage"),
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
            let remaining_ms = deadline
                .saturating_duration_since(std::time::Instant::now())
                .as_millis()
                .min(u128::from(self.queue_wait_timeout.milliseconds()))
                as u32;
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
            let released = match published.read_and_settle(
                self.engine,
                self.runner.logical_runner(),
                pending,
                remaining_ms,
                catchup_scratch
                    .take_readback()
                    .expect("preclock maintenance readback"),
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
                restore_storage.as_mut().expect("preclock restore storage"),
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
                    return Err(self.structural_failure(
                        "ordinary speculative preparation",
                        (
                            error,
                            reservation,
                            outcome,
                            diagnostic_history,
                            speculative_recipe,
                            speculative_scratch,
                        ),
                    ))
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
        let remaining_ms = deadline
            .saturating_duration_since(std::time::Instant::now())
            .as_millis()
            .min(u128::from(self.queue_wait_timeout.milliseconds()))
            as u32;
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
            unreachable!("committed physical owner is quiescent")
        };
        let M1ServingPhysicalRunnerQuiescentStateV1::Rearmed {
            released,
            diagnostic_history,
        } = custody.state
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
