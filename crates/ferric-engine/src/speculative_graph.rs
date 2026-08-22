//! Narrow single-member speculative completion composition.
//!
//! This module joins one already-existing ordered completion with the checked
//! logical publication/KV transaction and the existing engine completion path.
//! It does not construct completion authority, execute a graph, refine the
//! engine KV pool to the two-role isolated KV model, or support multi-member
//! batches. The `ContinuousBatch` is an independently checked logical witness;
//! no runtime scheduler refinement is claimed.

use crate::{Engine, EngineError, ExactCompletion};
use ferric_spec::{
    apply_preflighted_speculative_step, preflight_speculative_step, AtomicSpeculativeStepError,
    AtomicSpeculativeStepOutcome, ContinuousBatch, IsolatedRequestKv,
    IsolatedSpeculativeKvExpectation, RequestId, SpeculativeKvRoundIndex, SpeculativeTokenInputs,
    StepPublication,
};
use vstd::prelude::*;

verus! {

/// Fail-closed rejection from the narrow single-member composition.
#[derive(Debug, PartialEq, Eq)]
pub enum SingleMemberSpeculativeGraphError {
    /// The completion does not name the round's exact epoch.
    CompletionEpochMismatch,
    /// The engine's head batch is not exactly one member.
    PendingMemberCount { actual: usize },
    /// The sole pending engine request differs from the speculative request.
    PendingRequestMismatch { actual: Option<RequestId> },
    /// Logical publication or isolated KV preflight failed.
    Logical(AtomicSpeculativeStepError),
    /// The existing engine completion path rejected or failed-stop.
    Engine(EngineError),
}

/// Ownership-preserving failure for one speculative graph attempt.
///
/// Before engine consumption, the exact input completion is returned unchanged.
/// A failure without a completion can only follow the engine's existing
/// post-consumption fail-stop path and grants no recovery authority.
///
/// ```compile_fail
/// use ferric_engine::SingleMemberSpeculativeGraphFailure;
///
/// fn recover_twice(failure: SingleMemberSpeculativeGraphFailure) {
///     let _first = failure.into_completion();
///     let _second = failure.into_completion();
/// }
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct SingleMemberSpeculativeGraphFailure {
    error: SingleMemberSpeculativeGraphError,
    completion: Option<ExactCompletion>,
}

impl SingleMemberSpeculativeGraphFailure {
    /// Borrows the diagnostic without consuming retained authority.
    #[must_use]
    pub const fn error(&self) -> &SingleMemberSpeculativeGraphError {
        &self.error
    }

    /// Recovers the exact completion only when the engine did not consume it.
    #[must_use]
    pub fn into_completion(self) -> (completion: Option<ExactCompletion>)
        ensures completion == self.completion,
    {
        self.completion
    }

    pub closed spec fn returns_completion_at_spec(
        &self,
        epoch: ferric_spec::completion::CompletionEpoch,
    ) -> bool {
        match self.completion {
            Some(completion) => completion.epoch_spec() == epoch,
            None => false,
        }
    }

    pub closed spec fn consumed_completion_spec(&self) -> bool {
        self.completion.is_none()
    }

    fn returned(
        error: SingleMemberSpeculativeGraphError,
        completion: ExactCompletion,
    ) -> (failure: Self)
        ensures failure.returns_completion_at_spec(completion.epoch_spec()),
    {
        Self {
            error,
            completion: Some(completion),
        }
    }
}

/// Inert observations from one successful single-member handoff.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SingleMemberSpeculativeGraphOutcome {
    logical: AtomicSpeculativeStepOutcome,
    accepted_draft_tokens: u8,
    required_engine_accepted_tokens: u32,
}

impl SingleMemberSpeculativeGraphOutcome {
    #[must_use]
    pub const fn logical(&self) -> AtomicSpeculativeStepOutcome {
        self.logical
    }

    #[must_use]
    pub const fn accepted_draft_tokens(&self) -> u8 {
        self.accepted_draft_tokens
    }

    /// Arithmetic count passed to the existing single-KV engine completion.
    ///
    /// This observation is not a cross-model KV refinement claim.
    #[must_use]
    pub const fn required_engine_accepted_tokens(&self) -> (accepted: u32)
        ensures accepted as int == self.accepted_draft_tokens as int + 1,
    {
        self.required_engine_accepted_tokens
    }
}

/// Immutable logical inputs for one single-member speculative handoff.
///
/// The bundle is inert and carries no completion, engine, publication, KV,
/// allocation, queue, or device authority.
pub struct SingleMemberSpeculativeGraphInputs<'a> {
    batch: &'a ContinuousBatch,
    other: &'a IsolatedRequestKv,
    index: &'a SpeculativeKvRoundIndex,
    expected: &'a IsolatedSpeculativeKvExpectation,
    token_inputs: SpeculativeTokenInputs<'a>,
}

impl<'a> SingleMemberSpeculativeGraphInputs<'a> {
    /// Bundles the exact logical witnesses checked before engine completion.
    #[must_use]
    pub const fn new(
        batch: &'a ContinuousBatch,
        other: &'a IsolatedRequestKv,
        index: &'a SpeculativeKvRoundIndex,
        expected: &'a IsolatedSpeculativeKvExpectation,
        token_inputs: SpeculativeTokenInputs<'a>,
    ) -> Self {
        Self {
            batch,
            other,
            index,
            expected,
            token_inputs,
        }
    }
}

/// Completes one exact single-member speculative step.
///
/// All logical checks and the exact count mapping complete before the sole
/// `ExactCompletion` is moved exactly once into [`Engine::complete_exact`]. If
/// that engine call succeeds, the opaque logical permit applies infallibly.
/// No derived permit contains or can recover the completion authority.
///
/// # Errors
///
/// External epoch, membership, logical, or retryable engine rejection returns
/// the unchanged completion. A post-consumption engine failure returns no
/// completion, leaves publication/isolated KV unapplied, and is fail-stop.
pub fn complete_single_member_speculative_graph<const C: usize>(
    engine: &mut Engine<C>,
    publication: &mut StepPublication,
    selected: &mut IsolatedRequestKv,
    completion: ExactCompletion,
    inputs: SingleMemberSpeculativeGraphInputs<'_>,
) -> (result: Result<SingleMemberSpeculativeGraphOutcome, SingleMemberSpeculativeGraphFailure>)
    requires
        old(engine).well_formed(),
        inputs.batch.valid(),
    ensures
        final(engine).well_formed(),
        match result {
            Ok(outcome) => {
                &&& ferric_spec::speculative_step_composition::atomic_speculative_step_transition(
                    old(publication),
                    final(publication),
                    old(selected),
                    final(selected),
                    inputs.index,
                    inputs.expected,
                    inputs.token_inputs.draft_tokens@,
                    inputs.token_inputs.target_choices@,
                    outcome.logical,
                )
                &&& outcome.accepted_draft_tokens
                    == outcome.logical.settlement.accepted_draft_tokens
                &&& outcome.required_engine_accepted_tokens as int
                    == outcome.accepted_draft_tokens as int + 1
                &&& final(engine).completed_epoch_spec() == completion.epoch_spec()
            },
            Err(failure) => {
                &&& *final(publication) == *old(publication)
                &&& *final(selected) == *old(selected)
                &&& (failure.returns_completion_at_spec(completion.epoch_spec())
                    || (failure.consumed_completion_spec() && final(engine).faulted_spec()))
            },
        },
{
    let ghost completion_epoch = completion.epoch_spec();
    if completion.epoch().value != inputs.index.completion_epoch.value {
        return Err(SingleMemberSpeculativeGraphFailure::returned(
            SingleMemberSpeculativeGraphError::CompletionEpochMismatch,
            completion,
        ));
    }
    let member_count = engine.pending_batch_member_count();
    if member_count != 1 {
        return Err(SingleMemberSpeculativeGraphFailure::returned(
            SingleMemberSpeculativeGraphError::PendingMemberCount {
                actual: member_count,
            },
            completion,
        ));
    }
    let pending = engine.pending_member(0);
    if pending != Some(inputs.index.request) {
        return Err(SingleMemberSpeculativeGraphFailure::returned(
            SingleMemberSpeculativeGraphError::PendingRequestMismatch { actual: pending },
            completion,
        ));
    }
    let permit = match preflight_speculative_step(
        inputs.batch,
        publication,
        selected,
        inputs.other,
        inputs.index,
        inputs.expected,
        inputs.token_inputs,
    ) {
        Ok(permit) => permit,
        Err(error) => {
            return Err(SingleMemberSpeculativeGraphFailure::returned(
                SingleMemberSpeculativeGraphError::Logical(error),
                completion,
            ));
        },
    };
    let accepted_draft_tokens = permit.accepted_draft_tokens();
    let required_engine_accepted_tokens = permit.required_single_member_accepted_tokens();
    let accepted = [required_engine_accepted_tokens];
    let completion_result = engine.complete_exact(completion, &accepted);
    let _completed = match completion_result {
        Ok(completed) => completed,
        Err(failure) => {
            let error = failure.error();
            let returned = failure.into_completion();
            return Err(SingleMemberSpeculativeGraphFailure {
                error: SingleMemberSpeculativeGraphError::Engine(error),
                completion: returned,
            });
        },
    };
    proof {
        reveal(Engine::completion_refines);
    }
    assert(_completed == 1);
    assert(engine.completed_epoch_spec() == completion_epoch);
    let logical = apply_preflighted_speculative_step(
        publication,
        selected,
        inputs.index,
        inputs.expected,
        inputs.token_inputs,
        permit,
    );
    assert(logical.settlement.accepted_draft_tokens == accepted_draft_tokens);
    let outcome = SingleMemberSpeculativeGraphOutcome {
        logical,
        accepted_draft_tokens,
        required_engine_accepted_tokens,
    };
    assert(outcome.required_engine_accepted_tokens as int
        == outcome.accepted_draft_tokens as int + 1);
    Ok(outcome)
}

} // verus!

#[cfg(test)]
mod tests {
    use super::{
        complete_single_member_speculative_graph, SingleMemberSpeculativeGraphError,
        SingleMemberSpeculativeGraphInputs,
    };
    use crate::{Engine, ExactCompletion};
    use ferric_spec::completion::CompletionEpoch;
    use ferric_spec::scheduling::RequestState;
    use ferric_spec::{
        apply_isolated_kv_action, apply_isolated_scheduler_step, CompactCompletionRecord,
        ContinuousBatch, CorrectionBonusKvDisposition, Identity, IsolatedKvAction,
        IsolatedRequestKv, IsolatedSchedulerAction, IsolatedSpeculativeKvExpectation,
        PhysicalPageId, PublicationPhase, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket,
        Qwen3PlanSelection, RequestId, ReservedStateDelta, SpeculativeKvInterval,
        SpeculativeKvRoundIndex, SpeculativeTokenInputs, StepPlan, StepPublication,
        M1_MAX_COMPLETION_TOKENS, M1_MAX_SPECULATIVE_KV_DRAFT_TOKENS,
        M1_MAX_SPECULATIVE_KV_TARGET_INPUTS,
    };

    const K: u8 = 4;
    const ACCEPTED_DRAFT: u8 = 2;

    struct Fixture {
        engine: Engine<1>,
        batch: ContinuousBatch,
        publication: StepPublication,
        selected: IsolatedRequestKv,
        other: IsolatedRequestKv,
        index: SpeculativeKvRoundIndex,
        expected: IsolatedSpeculativeKvExpectation,
        draft_tokens: [u32; K as usize],
        target_choices: [u32; K as usize + 1],
        epoch: CompletionEpoch,
        request: RequestId,
    }

    const fn selection(role: Qwen3ModelRole) -> Qwen3PlanSelection {
        Qwen3PlanSelection {
            role,
            mode: Qwen3ExecutionMode::Speculative,
            bucket: Qwen3PlanBucket::SpeculativeS1K4C8192,
        }
    }

    fn write_interval(
        batch: &ContinuousBatch,
        selected: &mut IsolatedRequestKv,
        other: &mut IsolatedRequestKv,
        request: RequestId,
        role: Qwen3ModelRole,
        end: u32,
    ) {
        for logical_position in 0..end {
            if logical_position == 0 {
                apply_isolated_kv_action(
                    batch,
                    selected,
                    other,
                    request,
                    role,
                    IsolatedKvAction::AppendPage {
                        page: PhysicalPageId::new(role, 0, 1),
                    },
                )
                .unwrap();
            }
            apply_isolated_kv_action(
                batch,
                selected,
                other,
                request,
                role,
                IsolatedKvAction::WriteToken { logical_position },
            )
            .unwrap();
        }
    }

    fn build_fixture(engine_tentative_tokens: u32) -> Fixture {
        let mut engine = Engine::<1>::new(16, 4, 64).unwrap();
        let request = engine.admit().unwrap();
        engine
            .append_tentative(request, engine_tentative_tokens)
            .unwrap();
        let mut members = [RequestId::new(0, 0); 1];
        let dispatched = engine.dispatch_ready(&mut members).unwrap().unwrap();
        assert_eq!(members[0], request);
        let epoch = dispatched.epoch();

        let mut batch = ContinuousBatch::initial();
        let mut selected = IsolatedRequestKv::new(
            request,
            selection(Qwen3ModelRole::Target8B),
            selection(Qwen3ModelRole::Draft06B),
        )
        .unwrap();
        let mut other = IsolatedRequestKv::new(
            RequestId::new(1, 1),
            selection(Qwen3ModelRole::Target8B),
            selection(Qwen3ModelRole::Draft06B),
        )
        .unwrap();
        apply_isolated_scheduler_step(
            &mut batch,
            &mut selected,
            &mut other,
            request,
            IsolatedSchedulerAction::Admit,
        )
        .unwrap();
        apply_isolated_scheduler_step(
            &mut batch,
            &mut selected,
            &mut other,
            request,
            IsolatedSchedulerAction::Dispatch { epoch },
        )
        .unwrap();
        write_interval(
            &batch,
            &mut selected,
            &mut other,
            request,
            Qwen3ModelRole::Target8B,
            u32::from(K) + 1,
        );
        write_interval(
            &batch,
            &mut selected,
            &mut other,
            request,
            Qwen3ModelRole::Draft06B,
            u32::from(K),
        );
        apply_isolated_scheduler_step(
            &mut batch,
            &mut selected,
            &mut other,
            request,
            IsolatedSchedulerAction::CompleteExact { epoch },
        )
        .unwrap();

        let draft_tokens = [100, 101, 102, 103];
        let target_choices = [100, 101, 900, 903, 904];
        let mut all_draft_tokens = [0; M1_MAX_SPECULATIVE_KV_DRAFT_TOKENS];
        all_draft_tokens[..draft_tokens.len()].copy_from_slice(&draft_tokens);
        let mut target_commit_ends = [0; M1_MAX_SPECULATIVE_KV_TARGET_INPUTS];
        let mut draft_commit_ends = [0; M1_MAX_SPECULATIVE_KV_TARGET_INPUTS];
        for accepted in 0..=usize::from(K) {
            let accepted = u32::try_from(accepted).unwrap();
            target_commit_ends[accepted as usize] = accepted + 1;
            draft_commit_ends[accepted as usize] = if accepted < u32::from(K) {
                accepted + 1
            } else {
                u32::from(K)
            };
        }
        let index = SpeculativeKvRoundIndex {
            request,
            completion_epoch: epoch,
            plan_id: Identity::new([23; 32]),
            target_selection: selection(Qwen3ModelRole::Target8B),
            draft_selection: selection(Qwen3ModelRole::Draft06B),
            draft_token_count: K,
            round_anchor: 77,
            draft_tokens: all_draft_tokens,
            target_pre_committed: 0,
            draft_pre_committed: 0,
            target_tentative: SpeculativeKvInterval {
                start: 0,
                end: u32::from(K) + 1,
            },
            draft_tentative: SpeculativeKvInterval {
                start: 0,
                end: u32::from(K),
            },
            target_commit_ends,
            draft_commit_ends,
            correction_bonus: CorrectionBonusKvDisposition::DeferredUntilNextStep,
        };
        let expected = IsolatedSpeculativeKvExpectation::new(
            request,
            epoch,
            index.plan_id,
            index.target_selection,
            index.draft_selection,
        );
        let mut emitted_tokens = [0; M1_MAX_COMPLETION_TOKENS];
        emitted_tokens[..3].copy_from_slice(&[100, 101, 900]);
        let record = CompactCompletionRecord {
            request,
            epoch,
            plan_id: index.plan_id,
            accepted_draft_tokens: ACCEPTED_DRAFT,
            emitted_token_count: 3,
            emitted_tokens,
        };
        let publication = StepPublication::reserve(
            StepPlan::new(request, epoch, index.plan_id, index.target_selection),
            ReservedStateDelta::from_compact_completion(record, index.target_selection),
        );

        Fixture {
            engine,
            batch,
            publication,
            selected,
            other,
            index,
            expected,
            draft_tokens,
            target_choices,
            epoch,
            request,
        }
    }

    fn token_inputs<'a>(
        draft_tokens: &'a [u32],
        target_choices: &'a [u32],
    ) -> SpeculativeTokenInputs<'a> {
        SpeculativeTokenInputs {
            draft_tokens,
            target_choices,
        }
    }

    #[test]
    fn exact_single_member_completion_moves_once_and_applies_after_engine_success() {
        let mut fixture = build_fixture(u32::from(K) + 1);
        let completion = ExactCompletion::from_contracted_hsa_quiescence(fixture.epoch);
        let inputs = SingleMemberSpeculativeGraphInputs::new(
            &fixture.batch,
            &fixture.other,
            &fixture.index,
            &fixture.expected,
            token_inputs(&fixture.draft_tokens, &fixture.target_choices),
        );
        let result = complete_single_member_speculative_graph(
            &mut fixture.engine,
            &mut fixture.publication,
            &mut fixture.selected,
            completion,
            inputs,
        )
        .unwrap();

        assert_eq!(result.accepted_draft_tokens(), ACCEPTED_DRAFT);
        assert_eq!(result.required_engine_accepted_tokens(), 3);
        assert_eq!(fixture.engine.committed_tokens(fixture.request), Some(3));
        assert_eq!(fixture.engine.resident_tokens(fixture.request), Some(3));
        assert_eq!(
            fixture.engine.state(fixture.request),
            Some(RequestState::Ready)
        );
        assert_eq!(fixture.publication.phase(), PublicationPhase::Published);
        let projection = fixture.selected.projection();
        assert_eq!(projection.target.committed_tokens, 3);
        assert_eq!(projection.draft.committed_tokens, 3);
    }

    #[test]
    fn epoch_and_pending_member_drift_return_the_same_completion() {
        let mut fixture = build_fixture(u32::from(K) + 1);
        let wrong_epoch = CompletionEpoch::new(fixture.epoch.value() + 1);
        let completion = ExactCompletion::from_contracted_hsa_quiescence(wrong_epoch);
        let before_publication = fixture.publication.phase();
        let before_selected = fixture.selected.projection();
        let inputs = SingleMemberSpeculativeGraphInputs::new(
            &fixture.batch,
            &fixture.other,
            &fixture.index,
            &fixture.expected,
            token_inputs(&fixture.draft_tokens, &fixture.target_choices),
        );
        let failure = complete_single_member_speculative_graph(
            &mut fixture.engine,
            &mut fixture.publication,
            &mut fixture.selected,
            completion,
            inputs,
        )
        .unwrap_err();
        assert_eq!(
            failure.error(),
            &SingleMemberSpeculativeGraphError::CompletionEpochMismatch
        );
        assert_eq!(failure.into_completion().unwrap().epoch(), wrong_epoch);
        assert_eq!(fixture.publication.phase(), before_publication);
        assert_eq!(fixture.selected.projection(), before_selected);

        let completion = ExactCompletion::from_contracted_hsa_quiescence(fixture.epoch);
        fixture.index.request = RequestId::new(1, 1);
        let inputs = SingleMemberSpeculativeGraphInputs::new(
            &fixture.batch,
            &fixture.other,
            &fixture.index,
            &fixture.expected,
            token_inputs(&fixture.draft_tokens, &fixture.target_choices),
        );
        let failure = complete_single_member_speculative_graph(
            &mut fixture.engine,
            &mut fixture.publication,
            &mut fixture.selected,
            completion,
            inputs,
        )
        .unwrap_err();
        assert!(matches!(
            failure.error(),
            SingleMemberSpeculativeGraphError::PendingRequestMismatch { .. }
        ));
        assert_eq!(failure.into_completion().unwrap().epoch(), fixture.epoch);
    }

    #[test]
    fn empty_engine_returns_completion_and_preserves_logical_state() {
        let mut fixture = build_fixture(u32::from(K) + 1);
        fixture.engine = Engine::<1>::new(16, 4, 64).unwrap();
        let before_publication = fixture.publication.phase();
        let before_selected = fixture.selected.projection();
        let completion = ExactCompletion::from_contracted_hsa_quiescence(fixture.epoch);
        let inputs = SingleMemberSpeculativeGraphInputs::new(
            &fixture.batch,
            &fixture.other,
            &fixture.index,
            &fixture.expected,
            token_inputs(&fixture.draft_tokens, &fixture.target_choices),
        );
        let failure = complete_single_member_speculative_graph(
            &mut fixture.engine,
            &mut fixture.publication,
            &mut fixture.selected,
            completion,
            inputs,
        )
        .unwrap_err();

        assert_eq!(
            failure.error(),
            &SingleMemberSpeculativeGraphError::PendingMemberCount { actual: 0 }
        );
        assert_eq!(failure.into_completion().unwrap().epoch(), fixture.epoch);
        assert_eq!(fixture.publication.phase(), before_publication);
        assert_eq!(fixture.selected.projection(), before_selected);
    }

    #[test]
    fn logical_drift_and_retryable_engine_rejection_preserve_logical_state() {
        let mut fixture = build_fixture(u32::from(K) + 1);
        fixture.index.plan_id = Identity::new([31; 32]);
        let before_publication = fixture.publication.phase();
        let before_selected = fixture.selected.projection();
        let completion = ExactCompletion::from_contracted_hsa_quiescence(fixture.epoch);
        let inputs = SingleMemberSpeculativeGraphInputs::new(
            &fixture.batch,
            &fixture.other,
            &fixture.index,
            &fixture.expected,
            token_inputs(&fixture.draft_tokens, &fixture.target_choices),
        );
        let failure = complete_single_member_speculative_graph(
            &mut fixture.engine,
            &mut fixture.publication,
            &mut fixture.selected,
            completion,
            inputs,
        )
        .unwrap_err();
        assert!(matches!(
            failure.error(),
            SingleMemberSpeculativeGraphError::Logical(_)
        ));
        assert_eq!(failure.into_completion().unwrap().epoch(), fixture.epoch);
        assert_eq!(fixture.publication.phase(), before_publication);
        assert_eq!(fixture.selected.projection(), before_selected);

        let mut fixture = build_fixture(2);
        let before_publication = fixture.publication.phase();
        let before_selected = fixture.selected.projection();
        let completion = ExactCompletion::from_contracted_hsa_quiescence(fixture.epoch);
        let inputs = SingleMemberSpeculativeGraphInputs::new(
            &fixture.batch,
            &fixture.other,
            &fixture.index,
            &fixture.expected,
            token_inputs(&fixture.draft_tokens, &fixture.target_choices),
        );
        let failure = complete_single_member_speculative_graph(
            &mut fixture.engine,
            &mut fixture.publication,
            &mut fixture.selected,
            completion,
            inputs,
        )
        .unwrap_err();
        assert!(matches!(
            failure.error(),
            SingleMemberSpeculativeGraphError::Engine(_)
        ));
        assert_eq!(failure.into_completion().unwrap().epoch(), fixture.epoch);
        assert_eq!(fixture.engine.completed_epoch(), CompletionEpoch::new(0));
        assert_eq!(fixture.publication.phase(), before_publication);
        assert_eq!(fixture.selected.projection(), before_selected);
    }
}
