#![forbid(unsafe_code)]

//! M1 single-member speculative graph composition theorem.
//!
//! This compiler-rooted proof joins the exact target-authoritative greedy
//! publication, isolated logical target/draft KV settlement, one-member engine
//! accepted-count handoff, and exact completion epoch. It deliberately does
//! not claim that a physical graph ran, that device KV refines the isolated KV
//! model, or that a multi-member batch shares this transition. Kernel output,
//! queue readback, device identity, hardware, numerical, timing, and
//! performance qualification remain separate M1 obligations. Scheduler and
//! multi-member refinement, machine semantics, and M1 closure are not proved.

#[allow(unused_imports)]
use ferric_engine::{
    complete_single_member_speculative_graph, Engine, ExactCompletion,
    SingleMemberSpeculativeGraphFailure, SingleMemberSpeculativeGraphInputs,
    SingleMemberSpeculativeGraphOutcome,
};
#[allow(unused_imports)]
use ferric_spec::{
    select_lowest_argmax, CompactCompletionError, ContinuousBatch, GreedyCommit, IsolatedRequestKv,
    IsolatedSpeculativeKvExpectation, SpeculativeKvRoundIndex, SpeculativeTokenInputs,
    StepPublication, TokenId, QWEN3_VOCABULARY_SIZE,
};
#[allow(unused_imports)]
use vstd::prelude::*;

verus! {

/// Five exact target-logit rows for the fixed M1 `K=4` speculative round.
///
/// Each row is interpreted through [`ferric_spec::select_lowest_argmax`]. The
/// rows carry no device, numerical-representation, or completion authority.
pub struct M1DeterministicSamplerScoreRowsV1<'a> {
    /// Target choice for draft position zero.
    pub choice_0: &'a [i64],
    /// Target choice for draft position one.
    pub choice_1: &'a [i64],
    /// Target choice for draft position two.
    pub choice_2: &'a [i64],
    /// Target choice for draft position three.
    pub choice_3: &'a [i64],
    /// Target bonus choice after accepting all four draft tokens.
    pub choice_4: &'a [i64],
}

/// Exact logical witnesses consumed by the deterministic sampler theorem.
pub struct M1DeterministicSamplerInputsV1<'a> {
    /// Independently valid logical scheduler witness.
    pub batch: &'a ContinuousBatch,
    /// Distinct request owner framed by the single-member composition.
    pub other: &'a IsolatedRequestKv,
    /// Exact target/draft speculative KV index.
    pub index: &'a SpeculativeKvRoundIndex,
    /// Exact isolated-KV settlement expectation.
    pub expected: &'a IsolatedSpeculativeKvExpectation,
    /// Four live draft candidates for the fixed M1 round.
    pub draft_tokens: &'a [TokenId],
    /// Five target-logit rows, one per target choice.
    pub scores: M1DeterministicSamplerScoreRowsV1<'a>,
}

/// Selects one exact lowest-ID argmax through the executable Ferric body.
fn select_exact_lowest_argmax(scores: &[i64]) -> (token: TokenId)
    requires scores@.len() == QWEN3_VOCABULARY_SIZE,
    ensures ferric_spec::is_lowest_argmax(scores@, token),
{
    match select_lowest_argmax(scores) {
        Ok(token) => token,
        Err(CompactCompletionError::ScoreCountMismatch) => {
            assert(scores@.len() != QWEN3_VOCABULARY_SIZE);
            assert(false);
            0
        },
        Err(_) => {
            assert(false);
            0
        },
    }
}

/// Complete source-level deterministic sampler composition for one M1 round.
pub open spec fn m1_deterministic_sampler_refinement_success(
    before_publication: &StepPublication,
    after_publication: &StepPublication,
    before_selected: &IsolatedRequestKv,
    after_selected: &IsolatedRequestKv,
    index: &SpeculativeKvRoundIndex,
    expected: &IsolatedSpeculativeKvExpectation,
    draft_tokens: Seq<TokenId>,
    choice_0_scores: Seq<i64>,
    choice_1_scores: Seq<i64>,
    choice_2_scores: Seq<i64>,
    choice_3_scores: Seq<i64>,
    choice_4_scores: Seq<i64>,
    completion_epoch: ferric_spec::completion::CompletionEpoch,
    final_engine_epoch: ferric_spec::completion::CompletionEpoch,
    outcome: SingleMemberSpeculativeGraphOutcome,
) -> bool {
    draft_tokens.len() == 4
        && exists|target_choices: Seq<TokenId>|
            target_choices.len() == 5
            && ferric_spec::is_lowest_argmax(
                choice_0_scores,
                target_choices[0],
            )
            && ferric_spec::is_lowest_argmax(
                choice_1_scores,
                target_choices[1],
            )
            && ferric_spec::is_lowest_argmax(
                choice_2_scores,
                target_choices[2],
            )
            && ferric_spec::is_lowest_argmax(
                choice_3_scores,
                target_choices[3],
            )
            && ferric_spec::is_lowest_argmax(
                choice_4_scores,
                target_choices[4],
            )
            && m1_single_member_speculative_success(
                before_publication,
                after_publication,
                before_selected,
                after_selected,
                index,
                expected,
                draft_tokens,
                target_choices,
                completion_epoch,
                final_engine_epoch,
                outcome,
            )
}

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

/// Computes exact lowest-ID target choices and publishes one M1 `K=4` round.
///
/// The five target choices are produced by the executable Ferric argmax body,
/// then consumed by the existing compact-completion, maximal-prefix greedy,
/// isolated-KV, publication, and exact-engine-completion composition. Mapping
/// device FP values to the integer score rows is a named external numerical
/// premise; physical graph execution, machine refinement, and hardware remain
/// separate obligations.
///
/// # Errors
///
/// Returns the same ownership-preserving failure as
/// [`m1_single_member_speculative_graph_theorem`].
pub fn m1_deterministic_sampler_refinement_theorem<const C: usize>(
    engine: &mut Engine<C>,
    publication: &mut StepPublication,
    selected: &mut IsolatedRequestKv,
    completion: ExactCompletion,
    inputs: M1DeterministicSamplerInputsV1<'_>,
) -> (result: Result<SingleMemberSpeculativeGraphOutcome, SingleMemberSpeculativeGraphFailure>)
    requires
        old(engine).well_formed(),
        inputs.batch.valid(),
        inputs.draft_tokens@.len() == 4,
        inputs.scores.choice_0@.len() == QWEN3_VOCABULARY_SIZE,
        inputs.scores.choice_1@.len() == QWEN3_VOCABULARY_SIZE,
        inputs.scores.choice_2@.len() == QWEN3_VOCABULARY_SIZE,
        inputs.scores.choice_3@.len() == QWEN3_VOCABULARY_SIZE,
        inputs.scores.choice_4@.len() == QWEN3_VOCABULARY_SIZE,
    ensures
        final(engine).well_formed(),
        match result {
            Ok(outcome) => m1_deterministic_sampler_refinement_success(
                old(publication),
                final(publication),
                old(selected),
                final(selected),
                inputs.index,
                inputs.expected,
                inputs.draft_tokens@,
                inputs.scores.choice_0@,
                inputs.scores.choice_1@,
                inputs.scores.choice_2@,
                inputs.scores.choice_3@,
                inputs.scores.choice_4@,
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
    let choice_0 = select_exact_lowest_argmax(inputs.scores.choice_0);
    let choice_1 = select_exact_lowest_argmax(inputs.scores.choice_1);
    let choice_2 = select_exact_lowest_argmax(inputs.scores.choice_2);
    let choice_3 = select_exact_lowest_argmax(inputs.scores.choice_3);
    let choice_4 = select_exact_lowest_argmax(inputs.scores.choice_4);
    let target_choices = [choice_0, choice_1, choice_2, choice_3, choice_4];
    let graph_inputs = SingleMemberSpeculativeGraphInputs::new(
        inputs.batch,
        inputs.other,
        inputs.index,
        inputs.expected,
        SpeculativeTokenInputs {
            draft_tokens: inputs.draft_tokens,
            target_choices: &target_choices,
        },
    );
    let result = m1_single_member_speculative_graph_theorem(
        engine,
        publication,
        selected,
        completion,
        graph_inputs,
    );
    match &result {
        Ok(_outcome) => {
            proof {
                reveal(m1_deterministic_sampler_refinement_success);
                assert(target_choices@.len() == 5);
                assert(target_choices@[0] == choice_0);
                assert(target_choices@[1] == choice_1);
                assert(target_choices@[2] == choice_2);
                assert(target_choices@[3] == choice_3);
                assert(target_choices@[4] == choice_4);
                assert(exists|choices: Seq<TokenId>|
                    choices.len() == 5
                    && ferric_spec::is_lowest_argmax(
                        inputs.scores.choice_0@,
                        choices[0],
                    )
                    && ferric_spec::is_lowest_argmax(
                        inputs.scores.choice_1@,
                        choices[1],
                    )
                    && ferric_spec::is_lowest_argmax(
                        inputs.scores.choice_2@,
                        choices[2],
                    )
                    && ferric_spec::is_lowest_argmax(
                        inputs.scores.choice_3@,
                        choices[3],
                    )
                    && ferric_spec::is_lowest_argmax(
                        inputs.scores.choice_4@,
                        choices[4],
                    )
                    && m1_single_member_speculative_success(
                        old(publication),
                        publication,
                        old(selected),
                        selected,
                        inputs.index,
                        inputs.expected,
                        inputs.draft_tokens@,
                        choices,
                        completion.epoch_spec(),
                        engine.completed_epoch_spec(),
                        *_outcome,
                    )) by {
                    let choices = target_choices@;
                    assert(m1_single_member_speculative_success(
                        old(publication),
                        publication,
                        old(selected),
                        selected,
                        inputs.index,
                        inputs.expected,
                        inputs.draft_tokens@,
                        choices,
                        completion.epoch_spec(),
                        engine.completed_epoch_spec(),
                        *_outcome,
                    ));
                }
            }
        },
        Err(_) => {},
    }
    result
}

} // verus!
