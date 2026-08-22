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

/// Opaque, non-clone authority for one fully preflighted logical transaction.
///
/// This permit contains no device-completion authority. It can only apply the
/// publication and logical target/draft KV effects checked by
/// [`preflight_speculative_step`].
///
/// ```compile_fail
/// use ferric_spec::SpeculativeStepPreflight;
///
/// fn duplicate(permit: SpeculativeStepPreflight) {
///     let _second = permit.clone();
/// }
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct SpeculativeStepPreflight {
    publication: crate::step_plan_publication::SpeculativePublicationPermit,
    kv: crate::request_isolation::IsolatedSpeculativeKvSettlementPermit,
    accepted_draft_tokens: u8,
}

impl SpeculativeStepPreflight {
    pub closed spec fn accepted_draft_tokens_spec(&self) -> u8 {
        self.accepted_draft_tokens
    }

    pub closed spec fn required_single_member_accepted_tokens_spec(&self) -> u32 {
        (self.accepted_draft_tokens as int + 1) as u32
    }

    pub closed spec fn valid_for(
        &self,
        publication: &StepPublication,
        selected: &IsolatedRequestKv,
        index: &SpeculativeKvRoundIndex,
        expected: &IsolatedSpeculativeKvExpectation,
        draft_tokens: Seq<TokenId>,
        target_choices: Seq<TokenId>,
    ) -> bool {
        &&& index.valid_for(
            expected.request_spec(),
            expected.completion_epoch_spec(),
            expected.plan_id_spec(),
            expected.target_selection_spec(),
            expected.draft_selection_spec(),
        )
        &&& draft_tokens_match_index(index, draft_tokens)
        &&& self.publication.valid_for(
            publication,
            index.request,
            index.completion_epoch,
            index.plan_id,
            index.target_selection,
            draft_tokens,
            target_choices,
        )
        &&& self.kv.valid_for(selected, index)
        &&& self.accepted_draft_tokens == self.publication.accepted_draft_tokens_spec()
        &&& self.accepted_draft_tokens == self.kv.accepted_draft_tokens_spec()
    }

    /// Exact number of draft candidates accepted by target verification.
    #[must_use]
    pub const fn accepted_draft_tokens(&self) -> (accepted: u8)
        ensures accepted == self.accepted_draft_tokens_spec(),
    {
        self.accepted_draft_tokens
    }

    /// Required single-member engine count: accepted draft tokens plus one.
    ///
    /// This arithmetic does not establish a refinement between the engine's
    /// single KV pool and either isolated target/draft KV state.
    #[must_use]
    pub const fn required_single_member_accepted_tokens(&self) -> (accepted: u32)
        ensures
            accepted == self.required_single_member_accepted_tokens_spec(),
            accepted as int == self.accepted_draft_tokens_spec() as int + 1,
    {
        self.accepted_draft_tokens as u32 + 1
    }
}

/// Computes the required accepted count for the narrow single-member handoff.
///
/// This is only a bounded arithmetic mapping. It does not establish a
/// cross-model KV refinement.
#[must_use]
pub const fn required_single_member_accepted_count(
    accepted_draft_tokens: u8,
) -> (accepted: u32)
    ensures accepted as int == accepted_draft_tokens as int + 1,
{
    accepted_draft_tokens as u32 + 1
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

pub(crate) proof fn atomic_transition_binds_accepted_count(
    before_publication: &StepPublication,
    after_publication: &StepPublication,
    before_selected: &IsolatedRequestKv,
    after_selected: &IsolatedRequestKv,
    index: &SpeculativeKvRoundIndex,
    expected: &IsolatedSpeculativeKvExpectation,
    draft_tokens: Seq<TokenId>,
    target_choices: Seq<TokenId>,
    outcome: AtomicSpeculativeStepOutcome,
)
    requires atomic_speculative_step_transition(
        before_publication,
        after_publication,
        before_selected,
        after_selected,
        index,
        expected,
        draft_tokens,
        target_choices,
        outcome,
    ),
    ensures
        outcome.settlement.accepted_draft_tokens
            == outcome.published_delta.compact_completion_spec().accepted_draft_tokens,
{
    reveal(atomic_speculative_step_transition);
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

/// Checks every publication and logical KV obligation without changing state.
///
/// The returned permit is opaque, non-clone, and contains no completion,
/// queue, device, allocation, or runtime authority.
///
/// # Errors
///
/// Returns [`AtomicSpeculativeStepError`] when the draft tokens, publication,
/// or isolated target/draft KV state differs from the exact indexed round.
pub fn preflight_speculative_step(
    batch: &ContinuousBatch,
    publication: &StepPublication,
    selected: &IsolatedRequestKv,
    other: &IsolatedRequestKv,
    index: &SpeculativeKvRoundIndex,
    expected: &IsolatedSpeculativeKvExpectation,
    token_inputs: SpeculativeTokenInputs<'_>,
) -> (result: Result<SpeculativeStepPreflight, AtomicSpeculativeStepError>)
    requires batch.valid(),
    ensures match result {
        Ok(permit) => permit.valid_for(
            publication,
            selected,
            index,
            expected,
            token_inputs.draft_tokens@,
            token_inputs.target_choices@,
        ),
        Err(_) => true,
    },
{
    proof {
        reveal(SpeculativeStepPreflight::valid_for);
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
    let permit = SpeculativeStepPreflight {
        publication: publication_permit,
        kv: kv_permit,
        accepted_draft_tokens,
    };
    assert(permit.valid_for(
        publication,
        selected,
        index,
        expected,
        token_inputs.draft_tokens@,
        token_inputs.target_choices@,
    ));
    Ok(permit)
}

/// Applies one fully checked publication and logical KV transaction.
///
/// No fallible operation runs here. The permit is consumed exactly once.
pub fn apply_preflighted_speculative_step(
    publication: &mut StepPublication,
    selected: &mut IsolatedRequestKv,
    index: &SpeculativeKvRoundIndex,
    _expected: &IsolatedSpeculativeKvExpectation,
    token_inputs: SpeculativeTokenInputs<'_>,
    permit: SpeculativeStepPreflight,
) -> (outcome: AtomicSpeculativeStepOutcome)
    requires permit.valid_for(
        old(publication),
        old(selected),
        index,
        _expected,
        token_inputs.draft_tokens@,
        token_inputs.target_choices@,
    ),
    ensures
        atomic_speculative_step_transition(
            old(publication),
            final(publication),
            old(selected),
            final(selected),
            index,
            _expected,
            token_inputs.draft_tokens@,
            token_inputs.target_choices@,
            outcome,
        ),
        outcome.settlement.accepted_draft_tokens as int + 1
            == permit.required_single_member_accepted_tokens_spec() as int,
{
    let ghost entry_publication = *publication;
    let ghost entry_selected = *selected;
    let ghost required_single_member_accepted_tokens =
        permit.required_single_member_accepted_tokens_spec();
    proof {
        reveal(SpeculativeStepPreflight::valid_for);
        reveal(atomic_speculative_step_transition);
        index.valid_for_implies_valid(
            _expected.request_spec(),
            _expected.completion_epoch_spec(),
            _expected.plan_id_spec(),
            _expected.target_selection_spec(),
            _expected.draft_selection_spec(),
        );
        index.valid_implies_correction_is_deferred();
    }
    let SpeculativeStepPreflight {
        publication: publication_permit,
        kv: kv_permit,
        accepted_draft_tokens: _accepted_draft_tokens,
    } = permit;
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
    assert(settlement.accepted_draft_tokens == _accepted_draft_tokens);
    assert(published_delta.compact_completion_spec().accepted_draft_tokens
        == _accepted_draft_tokens);
    assert(settlement.accepted_draft_tokens
        == published_delta.compact_completion_spec().accepted_draft_tokens);
    assert(published_delta == publication.delta_spec());
    assert(atomic_speculative_step_transition(
        &entry_publication,
        publication,
        &entry_selected,
        selected,
        index,
        _expected,
        token_inputs.draft_tokens@,
        token_inputs.target_choices@,
        outcome,
    ));
    assert(outcome.settlement.accepted_draft_tokens as int + 1
        == required_single_member_accepted_tokens as int);
    outcome
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
    let permit = preflight_speculative_step(
        batch,
        publication,
        selected,
        other,
        index,
        expected,
        token_inputs,
    )?;
    let outcome = apply_preflighted_speculative_step(
        publication,
        selected,
        index,
        expected,
        token_inputs,
        permit,
    );
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
