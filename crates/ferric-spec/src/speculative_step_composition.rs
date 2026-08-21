//! Atomic logical composition of speculative completion, KV settlement, and publication.
//!
//! All compact-completion, greedy, indexing, routing, physical-KV, and accounting
//! checks complete before either owned state is changed. The correction or bonus
//! remains deferred to the next step exactly as required by the round index.
//! This is source-level logical semantics only; it provides no engine, queue,
//! device, address, runtime, machine, timing, or performance refinement.

use crate::request_isolation::{
    apply_preflighted_isolated_speculative_kv, preflight_isolated_speculative_kv,
};
use crate::step_plan_publication::{
    apply_preflighted_speculative_publication, preflight_speculative_publication,
};
use crate::{
    ContinuousBatch, IsolatedRequestKv, IsolatedSpeculativeKvExpectation,
    IsolatedSpeculativeKvSettlement, RequestIsolationError, ReservedStateDelta,
    SpeculativeKvRoundIndex, SpeculativeTokenInputs, StepPublication, StepPublicationError,
    TokenId,
};
use vstd::prelude::*;

verus! {

/// Fail-closed rejection from the atomic logical composition.
#[derive(Debug, PartialEq, Eq)]
pub enum AtomicSpeculativeStepError {
    /// The supplied live draft sequence differs from the exact round index.
    DraftTokensMismatch,
    /// Compact completion or one-shot publication validation failed.
    Publication(StepPublicationError),
    /// Routing, indexing, physical KV, or retired accounting failed.
    Kv(RequestIsolationError),
}

/// Exact logical effects returned after one atomic successful composition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtomicSpeculativeStepOutcome {
    pub settlement: IsolatedSpeculativeKvSettlement,
    pub published_delta: ReservedStateDelta,
}

/// The caller-supplied draft slice is exactly the live index prefix, not a substitute.
pub closed spec fn draft_tokens_match_index(
    index: &SpeculativeKvRoundIndex,
    draft_tokens: Seq<TokenId>,
) -> bool {
    index.draft_token_count as int <= index.draft_tokens@.len()
        && draft_tokens.len() == index.draft_token_count as int
        && forall|position: int|
            0 <= position < draft_tokens.len()
                ==> draft_tokens[position] == index.draft_tokens[position]
}

/// Complete success relation for the logical transaction.
pub closed spec fn atomic_speculative_step_transition(
    before_publication: &StepPublication,
    after_publication: &StepPublication,
    before_selected: &IsolatedRequestKv,
    after_selected: &IsolatedRequestKv,
    index: &SpeculativeKvRoundIndex,
    expected: &IsolatedSpeculativeKvExpectation,
    draft_tokens: Seq<TokenId>,
    target_choices: Seq<TokenId>,
    outcome: AtomicSpeculativeStepOutcome,
) -> bool {
    &&& draft_tokens_match_index(index, draft_tokens)
    &&& index.valid_for(
        expected.request_spec(),
        expected.completion_epoch_spec(),
        expected.plan_id_spec(),
        expected.target_selection_spec(),
        expected.draft_selection_spec(),
    )
    &&& crate::speculative_kv_indexing::correction_is_deferred(index.correction_bonus)
    &&& crate::step_plan_publication::speculative_validation_and_publication_transition(
        before_publication,
        after_publication,
        index.request,
        index.completion_epoch,
        index.plan_id,
        index.target_selection,
        draft_tokens,
        target_choices,
    )
    &&& crate::request_isolation::isolated_speculative_settlement_transition(
        before_selected,
        after_selected,
        index,
        outcome.settlement,
    )
    &&& outcome.settlement.accepted_draft_tokens
        == outcome.published_delta.compact_completion_spec().accepted_draft_tokens
    &&& outcome.published_delta == after_publication.delta_spec()
}

fn exact_draft_tokens(
    index: &SpeculativeKvRoundIndex,
    draft_tokens: &[TokenId],
) -> (matches: bool)
    ensures matches == draft_tokens_match_index(index, draft_tokens@),
{
    proof { reveal(draft_tokens_match_index); }
    if index.draft_token_count as usize > index.draft_tokens.len()
        || draft_tokens.len() != index.draft_token_count as usize
    {
        return false;
    }
    let mut position = 0usize;
    while position < draft_tokens.len()
        invariant
            draft_tokens@.len() == index.draft_token_count as int,
            index.draft_token_count as int <= index.draft_tokens@.len(),
            position <= draft_tokens@.len(),
            forall|prior: int|
                0 <= prior < position
                    ==> draft_tokens@[prior] == index.draft_tokens@[prior],
        decreases draft_tokens.len() - position,
    {
        if draft_tokens[position] != index.draft_tokens[position] {
            return false;
        }
        position += 1;
    }
    true
}

/// Validates and atomically applies one exact target-authoritative speculative step.
///
/// The scheduler batch is observed but never changed. The other request is
/// framed exactly. No fallible operation runs after the two immutable
/// preflights succeed.
///
/// # Errors
///
/// Returns [`AtomicSpeculativeStepError`] for any draft/index drift, malformed
/// compact completion, stale publication identity or phase, invalid target
/// verification, stale KV authority, malformed role state, or retired counter
/// exhaustion. Every error preserves all four mutable inputs exactly.
pub fn settle_and_publish_speculative_step(
    batch: &mut ContinuousBatch,
    publication: &mut StepPublication,
    selected: &mut IsolatedRequestKv,
    other: &mut IsolatedRequestKv,
    index: &SpeculativeKvRoundIndex,
    expected: &IsolatedSpeculativeKvExpectation,
    token_inputs: SpeculativeTokenInputs<'_>,
) -> (result: Result<AtomicSpeculativeStepOutcome, AtomicSpeculativeStepError>)
    requires old(batch).valid(),
    ensures
        *final(batch) == *old(batch),
        *final(other) == *old(other),
        match result {
            Ok(outcome) => atomic_speculative_step_transition(
                old(publication),
                final(publication),
                old(selected),
                final(selected),
                index,
                expected,
                token_inputs.draft_tokens@,
                token_inputs.target_choices@,
                outcome,
            ),
            Err(_) => {
                &&& *final(publication) == *old(publication)
                &&& *final(selected) == *old(selected)
            },
        },
{
    let ghost entry_publication = *publication;
    let ghost entry_selected = *selected;
    assert(entry_publication == *old(publication));
    assert(entry_selected == *old(selected));
    proof {
        reveal(atomic_speculative_step_transition);
    }
    if !exact_draft_tokens(index, token_inputs.draft_tokens) {
        return Err(AtomicSpeculativeStepError::DraftTokensMismatch);
    }
    let publication_permit = match preflight_speculative_publication(
        publication,
        index.request,
        index.completion_epoch,
        &index.plan_id,
        index.target_selection,
        token_inputs,
    ) {
        Ok(permit) => permit,
        Err(error) => return Err(AtomicSpeculativeStepError::Publication(error)),
    };
    let accepted_draft_tokens = publication_permit.accepted_draft_tokens();
    let ghost publication_accepted = publication_permit.accepted_draft_tokens_spec();
    assert(accepted_draft_tokens == publication_accepted);
    let kv_permit = match preflight_isolated_speculative_kv(
        batch,
        selected,
        other,
        index,
        accepted_draft_tokens,
        expected,
    ) {
        Ok(permit) => permit,
        Err(error) => return Err(AtomicSpeculativeStepError::Kv(error)),
    };
    let ghost kv_accepted = kv_permit.accepted_draft_tokens_spec();
    assert(kv_accepted == accepted_draft_tokens);

    let settlement = apply_preflighted_isolated_speculative_kv(selected, index, kv_permit);
    let published_delta = apply_preflighted_speculative_publication(
        publication,
        publication_permit,
        index.request,
        index.completion_epoch,
        &index.plan_id,
        index.target_selection,
        token_inputs,
    );
    let outcome = AtomicSpeculativeStepOutcome {
        settlement,
        published_delta,
    };
    assert(draft_tokens_match_index(index, token_inputs.draft_tokens@));
    assert(index.valid_for(
        expected.request_spec(),
        expected.completion_epoch_spec(),
        expected.plan_id_spec(),
        expected.target_selection_spec(),
        expected.draft_selection_spec(),
    ));
    proof {
        index.valid_for_implies_valid(
            expected.request_spec(),
            expected.completion_epoch_spec(),
            expected.plan_id_spec(),
            expected.target_selection_spec(),
            expected.draft_selection_spec(),
        );
        index.valid_implies_correction_is_deferred();
        assert(crate::speculative_kv_indexing::correction_is_deferred(
            index.correction_bonus,
        ));
    }
    assert(crate::step_plan_publication::speculative_validation_and_publication_transition(
        &entry_publication,
        publication,
        index.request,
        index.completion_epoch,
        index.plan_id,
        index.target_selection,
        token_inputs.draft_tokens@,
        token_inputs.target_choices@,
    ));
    assert(crate::request_isolation::isolated_speculative_settlement_transition(
        &entry_selected,
        selected,
        index,
        settlement,
    ));
    assert(settlement.accepted_draft_tokens == kv_accepted);
    assert(published_delta.compact_completion_spec().accepted_draft_tokens
        == publication_accepted);
    assert(settlement.accepted_draft_tokens
        == published_delta.compact_completion_spec().accepted_draft_tokens);
    assert(published_delta == publication.delta_spec());
    assert(atomic_speculative_step_transition(
        &entry_publication,
        publication,
        &entry_selected,
        selected,
        index,
        expected,
        token_inputs.draft_tokens@,
        token_inputs.target_choices@,
        outcome,
    ));
    Ok(outcome)
}

} // verus!
