#![forbid(unsafe_code)]

//! M1 single-member speculative graph composition theorem.
//!
//! This compiler-rooted proof joins the exact target-authoritative greedy
//! publication, isolated logical target/draft KV settlement, one-member engine
//! accepted-count handoff, and exact completion epoch. It deliberately does
//! not claim that a physical graph ran, that device KV refines the isolated KV
//! model, or that a multi-member batch shares this transition. Kernel output,
//! queue readback, device identity, hardware, numerical, timing, and
//! performance qualification remain separate M1 obligations.

#[allow(unused_imports)]
use ferric_engine::{
    complete_single_member_speculative_graph, Engine, ExactCompletion,
    SingleMemberSpeculativeGraphFailure, SingleMemberSpeculativeGraphInputs,
    SingleMemberSpeculativeGraphOutcome,
};
#[allow(unused_imports)]
use ferric_spec::{
    ContinuousBatch, GreedyCommit, IsolatedRequestKv, IsolatedSpeculativeKvExpectation,
    SpeculativeKvRoundIndex, StepPublication, TokenId,
};
#[allow(unused_imports)]
use vstd::prelude::*;

verus! {

/// Exact success boundary proved for the current M1 single-member handoff.
pub open spec fn m1_single_member_speculative_success(
    before_publication: &StepPublication,
    after_publication: &StepPublication,
    before_selected: &IsolatedRequestKv,
    after_selected: &IsolatedRequestKv,
    index: &SpeculativeKvRoundIndex,
    expected: &IsolatedSpeculativeKvExpectation,
    draft_tokens: Seq<TokenId>,
    target_choices: Seq<TokenId>,
    completion_epoch: ferric_spec::completion::CompletionEpoch,
    final_engine_epoch: ferric_spec::completion::CompletionEpoch,
    outcome: SingleMemberSpeculativeGraphOutcome,
) -> bool {
    &&& ferric_spec::speculative_step_composition::atomic_speculative_step_transition(
        before_publication,
        after_publication,
        before_selected,
        after_selected,
        index,
        expected,
        draft_tokens,
        target_choices,
        outcome.logical_spec(),
    )
    &&& ferric_spec::request_isolation::isolated_speculative_settlement_transition(
        before_selected,
        after_selected,
        index,
        outcome.logical_spec().settlement,
    )
    &&& outcome.accepted_draft_tokens_spec()
        == outcome.logical_spec().settlement.accepted_draft_tokens
    &&& outcome.required_engine_accepted_tokens_spec() as int
        == outcome.accepted_draft_tokens_spec() as int + 1
    &&& outcome.required_engine_accepted_tokens_spec() as int
        == outcome.logical_spec().published_delta.compact_completion_spec()
            .emitted_token_count as int
    &&& exists|commit: GreedyCommit|
        ferric_spec::step_plan_publication::speculative_publication_matches(
            before_publication,
            index.request,
            index.completion_epoch,
            index.plan_id,
            index.target_selection,
            draft_tokens,
            target_choices,
            &commit,
        )
    &&& final_engine_epoch == completion_epoch
}

/// Exact fail-closed boundary proved for the current M1 handoff.
pub open spec fn m1_single_member_speculative_failure(
    before_publication: &StepPublication,
    after_publication: &StepPublication,
    before_selected: &IsolatedRequestKv,
    after_selected: &IsolatedRequestKv,
    completion_epoch: ferric_spec::completion::CompletionEpoch,
    final_engine_faulted: bool,
    failure: &SingleMemberSpeculativeGraphFailure,
) -> bool {
    &&& *after_publication == *before_publication
    &&& *after_selected == *before_selected
    &&& (failure.returns_completion_at_spec(completion_epoch)
        || (failure.consumed_completion_spec() && final_engine_faulted))
}

/// Executes and proves the strongest current M1 speculative system join.
///
/// Success establishes exact greedy target publication, isolated logical
/// target/draft KV settlement (including rejected-tail settlement), the
/// published `accepted + 1` count passed to the one-member engine, and the
/// exact completed epoch. Rejection frames publication and isolated KV; a
/// consumed completion is possible only on the engine's existing fail-stop
/// path.
///
/// This theorem accepts only an independently valid logical
/// [`ContinuousBatch`] witness. It is not scheduler-to-batch refinement and
/// grants no physical execution authority.
///
/// # Errors
///
/// Returns the exact ownership-preserving failure from
/// [`complete_single_member_speculative_graph`]. Pre-consumption rejection
/// retains the completion; a consumed completion is returned only as absent on
/// the engine's fail-stop path.
pub fn m1_single_member_speculative_graph_theorem<const C: usize>(
    engine: &mut Engine<C>,
    publication: &mut StepPublication,
    selected: &mut IsolatedRequestKv,
    completion: ExactCompletion,
    inputs: SingleMemberSpeculativeGraphInputs<'_>,
) -> (result: Result<SingleMemberSpeculativeGraphOutcome, SingleMemberSpeculativeGraphFailure>)
    requires
        old(engine).well_formed(),
        inputs.batch_valid_spec(),
    ensures
        final(engine).well_formed(),
        match result {
            Ok(outcome) => m1_single_member_speculative_success(
                old(publication),
                final(publication),
                old(selected),
                final(selected),
                inputs.index_spec(),
                inputs.expected_spec(),
                inputs.draft_tokens_spec(),
                inputs.target_choices_spec(),
                completion.epoch_spec(),
                final(engine).completed_epoch_spec(),
                outcome,
            ),
            Err(failure) => m1_single_member_speculative_failure(
                old(publication),
                final(publication),
                old(selected),
                final(selected),
                completion.epoch_spec(),
                final(engine).faulted_spec(),
                &failure,
            ),
        },
{
    let ghost entry_publication = *publication;
    let ghost entry_selected = *selected;
    let ghost completion_epoch = completion.epoch_spec();
    assert(entry_publication == *old(publication));
    assert(entry_selected == *old(selected));
    let result = complete_single_member_speculative_graph(
        engine,
        publication,
        selected,
        completion,
        inputs,
    );
    match &result {
        Ok(outcome) => {
            let ghost logical = outcome.logical_spec();
            let _ = outcome;
            proof {
                ferric_spec::speculative_step_composition::atomic_transition_binds_greedy_publication(
                    &entry_publication,
                    publication,
                    &entry_selected,
                    selected,
                    inputs.index_spec(),
                    inputs.expected_spec(),
                    inputs.draft_tokens_spec(),
                    inputs.target_choices_spec(),
                    logical,
                );
                assert(ferric_spec::speculative_step_composition::atomic_speculative_step_transition(
                    &entry_publication,
                    publication,
                    &entry_selected,
                    selected,
                    inputs.index_spec(),
                    inputs.expected_spec(),
                    inputs.draft_tokens_spec(),
                    inputs.target_choices_spec(),
                    logical,
                ));
                assert(ferric_spec::request_isolation::isolated_speculative_settlement_transition(
                    &entry_selected,
                    selected,
                    inputs.index_spec(),
                    logical.settlement,
                ));
                assert(outcome.required_engine_accepted_tokens_spec() as int
                    == outcome.accepted_draft_tokens_spec() as int + 1);
                assert(outcome.accepted_draft_tokens_spec()
                    == logical.settlement.accepted_draft_tokens);
                assert(logical.published_delta.compact_completion_spec()
                    .emitted_token_count as int
                    == logical.settlement.accepted_draft_tokens as int + 1);
                assert(outcome.required_engine_accepted_tokens_spec() as int
                    == logical.published_delta.compact_completion_spec()
                        .emitted_token_count as int);
                assert(exists|commit: GreedyCommit|
                    ferric_spec::step_plan_publication::speculative_publication_matches(
                        &entry_publication,
                        inputs.index_spec().request,
                        inputs.index_spec().completion_epoch,
                        inputs.index_spec().plan_id,
                        inputs.index_spec().target_selection,
                        inputs.draft_tokens_spec(),
                        inputs.target_choices_spec(),
                        &commit,
                    ));
                assert(final(engine).completed_epoch_spec() == completion_epoch);
                reveal(m1_single_member_speculative_success);
                assert(m1_single_member_speculative_success(
                    &entry_publication,
                    publication,
                    &entry_selected,
                    selected,
                    inputs.index_spec(),
                    inputs.expected_spec(),
                    inputs.draft_tokens_spec(),
                    inputs.target_choices_spec(),
                    completion_epoch,
                    final(engine).completed_epoch_spec(),
                    *outcome,
                ));
            }
        },
        Err(_) => {
            proof {
                reveal(m1_single_member_speculative_failure);
            }
        },
    }
    result
}

} // verus!
