//! Logical publication contract for one admitted M1 generated-plan step.
//!
//! This module owns no physical runner, queue, GPU, or KV-cache behavior. It
//! only validates a reserved logical token delta and controls its one-shot
//! publication or discard transition.

use crate::completion::CompletionEpoch;
use crate::{
    validate_compact_completion, verify_speculative_completion, CompactCompletionError,
    CompactCompletionRecord, GreedyCommit, Identity, Qwen3ExecutionMode, Qwen3ModelRole,
    Qwen3PlanError, Qwen3PlanSelection, RequestId, SpeculativeCompletionError, TokenId,
    M1_MAX_COMPLETION_TOKENS,
};
use vstd::prelude::*;

verus! {

/// Immutable logical authority for one generated-plan execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepPlan {
    request: RequestId,
    completion_epoch: CompletionEpoch,
    plan_id: Identity,
    selection: Qwen3PlanSelection,
}

impl StepPlan {
    /// Constructs a logical plan authority. Validation remains fail-closed.
    #[must_use]
    pub const fn new(
        request: RequestId,
        completion_epoch: CompletionEpoch,
        plan_id: Identity,
        selection: Qwen3PlanSelection,
    ) -> Self {
        Self { request, completion_epoch, plan_id, selection }
    }

    /// Exact generational request identity.
    #[must_use]
    pub const fn request(&self) -> RequestId {
        self.request
    }

    /// Exact completion epoch.
    #[must_use]
    pub const fn completion_epoch(&self) -> CompletionEpoch {
        self.completion_epoch
    }

    /// Exact generated-plan identity.
    #[must_use]
    pub const fn plan_id(&self) -> &Identity {
        &self.plan_id
    }

    /// Exact target role, mode, and finite graph bucket.
    #[must_use]
    pub const fn selection(&self) -> Qwen3PlanSelection {
        self.selection
    }
}

/// Fixed, reserved logical effects awaiting validation and publication.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReservedStateDelta {
    request: RequestId,
    completion_epoch: CompletionEpoch,
    plan_id: Identity,
    selection: Qwen3PlanSelection,
    accepted_token_count: u8,
    emitted_token_count: u8,
    emitted_tokens: [TokenId; M1_MAX_COMPLETION_TOKENS],
}

/// Borrowed exact sequences used by speculative target verification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpeculativeTokenInputs<'a> {
    pub draft_tokens: &'a [TokenId],
    pub target_choices: &'a [TokenId],
}

impl ReservedStateDelta {
    pub closed spec fn compact_completion_spec(&self) -> CompactCompletionRecord {
        CompactCompletionRecord {
            request: self.request,
            epoch: self.completion_epoch,
            plan_id: self.plan_id,
            accepted_draft_tokens: self.accepted_token_count,
            emitted_token_count: self.emitted_token_count,
            emitted_tokens: self.emitted_tokens,
        }
    }

    /// Reserves the untrusted compact result together with its observed graph selection.
    #[must_use]
    pub const fn from_compact_completion(
        record: CompactCompletionRecord,
        selection: Qwen3PlanSelection,
    ) -> Self {
        Self {
            request: record.request,
            completion_epoch: record.epoch,
            plan_id: record.plan_id,
            selection,
            accepted_token_count: record.accepted_draft_tokens,
            emitted_token_count: record.emitted_token_count,
            emitted_tokens: record.emitted_tokens,
        }
    }

    /// Reconstructs the compact record validated by this logical contract.
    #[must_use]
    pub const fn compact_completion(&self) -> (record: CompactCompletionRecord)
        ensures record == self.compact_completion_spec(),
    {
        CompactCompletionRecord {
            request: self.request,
            epoch: self.completion_epoch,
            plan_id: self.plan_id,
            accepted_draft_tokens: self.accepted_token_count,
            emitted_token_count: self.emitted_token_count,
            emitted_tokens: self.emitted_tokens,
        }
    }

    /// Exact generational request identity affected by the delta.
    #[must_use]
    pub const fn request(&self) -> RequestId {
        self.request
    }

    /// Exact completion epoch affected by the delta.
    #[must_use]
    pub const fn completion_epoch(&self) -> CompletionEpoch {
        self.completion_epoch
    }

    /// Exact generated-plan identity which produced the delta.
    #[must_use]
    pub const fn plan_id(&self) -> &Identity {
        &self.plan_id
    }

    /// Exact target role, mode, and finite graph bucket which produced the delta.
    #[must_use]
    pub const fn selection(&self) -> Qwen3PlanSelection {
        self.selection
    }

    /// Number of accepted draft tokens. Direct publication requires zero.
    #[must_use]
    pub const fn accepted_token_count(&self) -> u8 {
        self.accepted_token_count
    }

    /// Number of live emitted tokens, bounded by 17.
    #[must_use]
    pub const fn emitted_token_count(&self) -> u8 {
        self.emitted_token_count
    }

    /// Fixed canonical token effect; unused positions are zero.
    #[must_use]
    pub const fn emitted_tokens(&self) -> &[TokenId; M1_MAX_COMPLETION_TOKENS] {
        &self.emitted_tokens
    }
}

/// One-shot logical publication state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublicationPhase {
    /// Reserved effects have not been semantically validated.
    Unpublished,
    /// Reserved effects are validated and may be published once.
    Validated,
    /// Effects were published; this is terminal.
    Published,
    /// Effects were discarded; this is terminal.
    Discarded,
}

/// A reserved publication with private phase authority.
#[derive(Debug, PartialEq, Eq)]
pub struct StepPublication {
    plan: StepPlan,
    delta: ReservedStateDelta,
    phase: PublicationPhase,
}

impl StepPublication {
    pub closed spec fn phase_spec(&self) -> PublicationPhase {
        self.phase
    }

    pub closed spec fn delta_spec(&self) -> ReservedStateDelta {
        self.delta
    }

    /// Reserves untrusted logical effects in the only constructible initial phase.
    #[must_use]
    pub const fn reserve(plan: StepPlan, delta: ReservedStateDelta) -> Self {
        Self { plan, delta, phase: PublicationPhase::Unpublished }
    }

    /// Immutable plan authority.
    #[must_use]
    pub const fn plan(&self) -> StepPlan {
        self.plan
    }

    /// Immutable reserved effects.
    #[must_use]
    pub const fn delta(&self) -> (delta: ReservedStateDelta)
        ensures delta == self.delta_spec(),
    {
        self.delta
    }

    /// Current one-shot publication phase.
    #[must_use]
    pub const fn phase(&self) -> (phase: PublicationPhase)
        ensures phase == self.phase_spec(),
    {
        self.phase
    }

    fn set_phase(&mut self, phase: PublicationPhase)
        ensures
            final(self).phase_spec() == phase,
            final(self).plan == old(self).plan,
            final(self).delta_spec() == old(self).delta_spec(),
    {
        self.phase = phase;
    }
}

/// Private, non-clone proof of complete speculative publication validation.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct SpeculativePublicationPermit {
    commit: GreedyCommit,
    accepted_draft_tokens: u8,
}

/// Fail-closed logical publication rejection.
#[derive(Debug, PartialEq, Eq)]
pub enum StepPublicationError {
    /// Validation or a terminal transition was requested from the wrong phase.
    WrongPhase,
    /// Publication is reserved to target-model results.
    NonTargetPublication,
    /// The requested validator does not match the plan mode.
    WrongValidationMode,
    /// The expected generated-plan identity is absent.
    ExpectedPlanIdentityAbsent,
    /// The plan belongs to a different request slot or generation.
    RequestMismatch,
    /// The plan belongs to a different completion epoch.
    CompletionEpochMismatch,
    /// The plan identity is stale or substituted.
    PlanIdentityMismatch,
    /// The role, mode, or finite bucket is invalid or substituted.
    Selection(Qwen3PlanError),
    /// Reserved effects do not repeat the exact immutable plan authority.
    DeltaAuthorityMismatch,
    /// A direct prefill/decode compact result is malformed.
    Completion(CompactCompletionError),
    /// A speculative compact result does not refine target verification.
    Speculative(SpeculativeCompletionError),
}

/// Exact expected authority relation for an immutable logical step plan.
pub closed spec fn step_plan_matches(
    plan: StepPlan,
    expected_request: RequestId,
    expected_epoch: CompletionEpoch,
    expected_plan_id: Identity,
    expected_selection: Qwen3PlanSelection,
) -> bool {
    crate::m1_completion::identity_present(expected_plan_id)
        && plan.request.slot_spec() == expected_request.slot_spec()
        && plan.request.generation_spec() == expected_request.generation_spec()
        && plan.completion_epoch.value == expected_epoch.value
        && plan.plan_id.bytes_spec() == expected_plan_id.bytes_spec()
        && plan.selection.valid()
        && expected_selection.valid()
        && plan.selection == expected_selection
        && target_publication_role(plan.selection.role)
}

pub closed spec fn target_publication_role(role: Qwen3ModelRole) -> bool {
    match role {
        Qwen3ModelRole::Target8B => true,
        Qwen3ModelRole::Draft06B => false,
    }
}

pub closed spec fn direct_publication_mode(mode: Qwen3ExecutionMode) -> bool {
    match mode {
        Qwen3ExecutionMode::Prefill | Qwen3ExecutionMode::Decode => true,
        Qwen3ExecutionMode::Speculative => false,
    }
}

pub closed spec fn speculative_publication_mode(mode: Qwen3ExecutionMode) -> bool {
    match mode {
        Qwen3ExecutionMode::Speculative => true,
        Qwen3ExecutionMode::Prefill | Qwen3ExecutionMode::Decode => false,
    }
}

pub closed spec fn publication_phase_matches(
    actual: PublicationPhase,
    expected: PublicationPhase,
) -> bool {
    match (actual, expected) {
        (PublicationPhase::Unpublished, PublicationPhase::Unpublished)
        | (PublicationPhase::Validated, PublicationPhase::Validated)
        | (PublicationPhase::Published, PublicationPhase::Published)
        | (PublicationPhase::Discarded, PublicationPhase::Discarded) => true,
        _ => false,
    }
}

/// Exact duplication of immutable plan authority in the reserved effects.
pub closed spec fn delta_authority_matches(delta: ReservedStateDelta, plan: StepPlan) -> bool {
    delta.request.slot_spec() == plan.request.slot_spec()
        && delta.request.generation_spec() == plan.request.generation_spec()
        && delta.completion_epoch.value == plan.completion_epoch.value
        && delta.plan_id.bytes_spec() == plan.plan_id.bytes_spec()
        && delta.selection == plan.selection
}

/// Fields other than the publication phase are immutable across transitions.
pub closed spec fn publication_payload_preserved(
    before: &StepPublication,
    after: &StepPublication,
) -> bool {
    before.plan == after.plan && before.delta == after.delta
}

/// Exact validation-state transition.
pub closed spec fn validation_transition(
    before: &StepPublication,
    after: &StepPublication,
) -> bool {
    publication_phase_matches(before.phase, PublicationPhase::Unpublished)
        && publication_phase_matches(after.phase, PublicationPhase::Validated)
        && publication_payload_preserved(before, after)
}

/// Exact one-shot publication transition.
pub closed spec fn publication_transition(
    before: &StepPublication,
    after: &StepPublication,
) -> bool {
    publication_phase_matches(before.phase, PublicationPhase::Validated)
        && publication_phase_matches(after.phase, PublicationPhase::Published)
        && publication_payload_preserved(before, after)
}

/// Exact terminal discard transition from either nonterminal phase.
pub closed spec fn discard_transition(
    before: &StepPublication,
    after: &StepPublication,
) -> bool {
    (publication_phase_matches(before.phase, PublicationPhase::Unpublished)
        || publication_phase_matches(before.phase, PublicationPhase::Validated))
        && publication_phase_matches(after.phase, PublicationPhase::Discarded)
        && publication_payload_preserved(before, after)
}

/// Exact direct prefill/decode validation relation.
pub closed spec fn direct_publication_is_valid(
    publication: &StepPublication,
    expected_request: RequestId,
    expected_epoch: CompletionEpoch,
    expected_plan_id: Identity,
    expected_selection: Qwen3PlanSelection,
) -> bool {
    publication_phase_matches(publication.phase, PublicationPhase::Unpublished)
        && step_plan_matches(
            publication.plan,
            expected_request,
            expected_epoch,
            expected_plan_id,
            expected_selection,
        )
        && direct_publication_mode(publication.plan.selection.mode)
        && delta_authority_matches(publication.delta, publication.plan)
        && crate::m1_completion::compact_completion_matches(
            publication.delta.compact_completion_spec(),
            publication.plan.request,
            publication.plan.completion_epoch,
            publication.plan.plan_id,
            0,
        )
}

/// Exact speculative validation relation composed with target verification.
pub closed spec fn speculative_publication_matches(
    publication: &StepPublication,
    expected_request: RequestId,
    expected_epoch: CompletionEpoch,
    expected_plan_id: Identity,
    expected_selection: Qwen3PlanSelection,
    draft_tokens: Seq<TokenId>,
    target_choices: Seq<TokenId>,
    commit: &GreedyCommit,
) -> bool {
    publication_phase_matches(publication.phase, PublicationPhase::Unpublished)
        && step_plan_matches(
            publication.plan,
            expected_request,
            expected_epoch,
            expected_plan_id,
            expected_selection,
        )
        && speculative_publication_mode(publication.plan.selection.mode)
        && delta_authority_matches(publication.delta, publication.plan)
        && crate::speculative_completion::speculative_completion_matches(
            publication.delta.compact_completion_spec(),
            publication.plan.request,
            publication.plan.completion_epoch,
            publication.plan.plan_id,
            draft_tokens,
            target_choices,
            commit,
        )
}

/// Exact unpublished-to-published transition after speculative validation.
pub closed spec fn speculative_validation_and_publication_transition(
    before: &StepPublication,
    after: &StepPublication,
    expected_request: RequestId,
    expected_epoch: CompletionEpoch,
    expected_plan_id: Identity,
    expected_selection: Qwen3PlanSelection,
    draft_tokens: Seq<TokenId>,
    target_choices: Seq<TokenId>,
) -> bool {
    &&& publication_phase_matches(before.phase, PublicationPhase::Unpublished)
    &&& publication_phase_matches(after.phase, PublicationPhase::Published)
    &&& publication_payload_preserved(before, after)
    &&& exists|commit: GreedyCommit| speculative_publication_matches(
        before,
        expected_request,
        expected_epoch,
        expected_plan_id,
        expected_selection,
        draft_tokens,
        target_choices,
        &commit,
    )
}

impl SpeculativePublicationPermit {
    pub(crate) closed spec fn accepted_draft_tokens_spec(&self) -> u8 {
        self.accepted_draft_tokens
    }

    pub(crate) closed spec fn valid_for(
        &self,
        publication: &StepPublication,
        expected_request: RequestId,
        expected_epoch: CompletionEpoch,
        expected_plan_id: Identity,
        expected_selection: Qwen3PlanSelection,
        draft_tokens: Seq<TokenId>,
        target_choices: Seq<TokenId>,
    ) -> bool {
        &&& speculative_publication_matches(
            publication,
            expected_request,
            expected_epoch,
            expected_plan_id,
            expected_selection,
            draft_tokens,
            target_choices,
            &self.commit,
        )
        &&& self.accepted_draft_tokens_spec() as nat == self.commit.accepted_spec()
    }

    pub(crate) fn accepted_draft_tokens(&self) -> (accepted: u8)
        ensures accepted == self.accepted_draft_tokens_spec(),
    {
        self.accepted_draft_tokens
    }
}

fn is_target_publication_role(role: Qwen3ModelRole) -> (target: bool)
    ensures target == target_publication_role(role),
{
    match role {
        Qwen3ModelRole::Target8B => true,
        Qwen3ModelRole::Draft06B => false,
    }
}

fn is_direct_publication_mode(mode: Qwen3ExecutionMode) -> (direct: bool)
    ensures direct == direct_publication_mode(mode),
{
    match mode {
        Qwen3ExecutionMode::Prefill | Qwen3ExecutionMode::Decode => true,
        Qwen3ExecutionMode::Speculative => false,
    }
}

fn is_speculative_publication_mode(mode: Qwen3ExecutionMode) -> (speculative: bool)
    ensures speculative == speculative_publication_mode(mode),
{
    match mode {
        Qwen3ExecutionMode::Speculative => true,
        Qwen3ExecutionMode::Prefill | Qwen3ExecutionMode::Decode => false,
    }
}

fn phase_matches(
    actual: PublicationPhase,
    expected: PublicationPhase,
) -> (matches: bool)
    ensures matches == publication_phase_matches(actual, expected),
{
    matches!((actual, expected),
        (PublicationPhase::Unpublished, PublicationPhase::Unpublished)
            | (PublicationPhase::Validated, PublicationPhase::Validated)
            | (PublicationPhase::Published, PublicationPhase::Published)
            | (PublicationPhase::Discarded, PublicationPhase::Discarded)
    )
}

pub(crate) fn validate_step_plan(
    plan: StepPlan,
    expected_request: RequestId,
    expected_epoch: CompletionEpoch,
    expected_plan_id: &Identity,
    expected_selection: Qwen3PlanSelection,
) -> (result: Result<(), StepPublicationError>)
    ensures result.is_ok() == step_plan_matches(
        plan,
        expected_request,
        expected_epoch,
        *expected_plan_id,
        expected_selection,
    ),
{
    proof {
        reveal(step_plan_matches);
        reveal(target_publication_role);
    }
    if !expected_plan_id.is_present() {
        return Err(StepPublicationError::ExpectedPlanIdentityAbsent);
    }
    if plan.request.slot() != expected_request.slot()
        || plan.request.generation() != expected_request.generation()
    {
        return Err(StepPublicationError::RequestMismatch);
    }
    if plan.completion_epoch.value != expected_epoch.value {
        return Err(StepPublicationError::CompletionEpochMismatch);
    }
    if !plan.plan_id.equals(expected_plan_id) {
        return Err(StepPublicationError::PlanIdentityMismatch);
    }
    if let Err(error) = plan.selection.validate() {
        return Err(StepPublicationError::Selection(error));
    }
    if let Err(error) = expected_selection.validate() {
        return Err(StepPublicationError::Selection(error));
    }
    if !plan.selection.matches(expected_selection) {
        return Err(StepPublicationError::Selection(Qwen3PlanError::SelectionMismatch));
    }
    if !is_target_publication_role(plan.selection.role) {
        return Err(StepPublicationError::NonTargetPublication);
    }
    Ok(())
}

fn validate_delta_authority(
    delta: ReservedStateDelta,
    plan: StepPlan,
) -> (result: Result<(), StepPublicationError>)
    ensures result.is_ok() == delta_authority_matches(delta, plan),
{
    if delta.request.slot() != plan.request.slot()
        || delta.request.generation() != plan.request.generation()
        || delta.completion_epoch.value != plan.completion_epoch.value
        || !delta.plan_id.equals(&plan.plan_id)
        || !delta.selection.matches(plan.selection)
    {
        return Err(StepPublicationError::DeltaAuthorityMismatch);
    }
    Ok(())
}

/// Validates one target prefill/decode delta before publication.
///
/// # Errors
///
/// Returns [`StepPublicationError`] unless the unpublished plan and delta bind
/// every expected authority field and the compact effect is exactly zero
/// accepted draft tokens plus one in-vocabulary target token.
pub fn validate_direct_publication(
    publication: &mut StepPublication,
    expected_request: RequestId,
    expected_epoch: CompletionEpoch,
    expected_plan_id: &Identity,
    expected_selection: Qwen3PlanSelection,
) -> (result: Result<(), StepPublicationError>)
    ensures
        result.is_ok() == direct_publication_is_valid(
            old(publication),
            expected_request,
            expected_epoch,
            *expected_plan_id,
            expected_selection,
        ),
        result.is_ok() ==> validation_transition(old(publication), final(publication)),
        result.is_err() ==> *final(publication) == *old(publication),
{
    let ghost entry = *publication;
    assert(entry == *old(publication));
    proof {
        reveal(direct_publication_is_valid);
        reveal(direct_publication_mode);
        reveal(publication_phase_matches);
        reveal(validation_transition);
        reveal(publication_payload_preserved);
    }
    if !phase_matches(publication.phase(), PublicationPhase::Unpublished) {
        return Err(StepPublicationError::WrongPhase);
    }
    validate_step_plan(
        publication.plan,
        expected_request,
        expected_epoch,
        expected_plan_id,
        expected_selection,
    )?;
    if !is_direct_publication_mode(publication.plan.selection.mode) {
        return Err(StepPublicationError::WrongValidationMode);
    }
    validate_delta_authority(publication.delta, publication.plan)?;
    let record = publication.delta.compact_completion();
    let completion_result = validate_compact_completion(
        &record,
        publication.plan.request,
        publication.plan.completion_epoch,
        &publication.plan.plan_id,
        0,
    );
    if let Err(error) = completion_result {
        return Err(StepPublicationError::Completion(error));
    }
    assert(record == entry.delta.compact_completion_spec());
    assert(direct_publication_is_valid(
        &entry,
        expected_request,
        expected_epoch,
        *expected_plan_id,
        expected_selection,
    ));
    publication.set_phase(PublicationPhase::Validated);
    assert(validation_transition(&entry, publication));
    Ok(())
}

/// Checks every speculative publication obligation without changing phase.
pub(crate) fn preflight_speculative_publication(
    publication: &StepPublication,
    expected_request: RequestId,
    expected_epoch: CompletionEpoch,
    expected_plan_id: &Identity,
    expected_selection: Qwen3PlanSelection,
    token_inputs: SpeculativeTokenInputs<'_>,
) -> (result: Result<SpeculativePublicationPermit, StepPublicationError>)
    ensures match result {
        Ok(permit) => permit.valid_for(
            publication,
            expected_request,
            expected_epoch,
            *expected_plan_id,
            expected_selection,
            token_inputs.draft_tokens@,
            token_inputs.target_choices@,
        ),
        Err(_) => true,
    },
{
    proof {
        reveal(SpeculativePublicationPermit::valid_for);
        reveal(speculative_publication_matches);
        reveal(speculative_publication_mode);
        reveal(publication_phase_matches);
    }
    if !phase_matches(publication.phase(), PublicationPhase::Unpublished) {
        return Err(StepPublicationError::WrongPhase);
    }
    validate_step_plan(
        publication.plan,
        expected_request,
        expected_epoch,
        expected_plan_id,
        expected_selection,
    )?;
    if !is_speculative_publication_mode(publication.plan.selection.mode) {
        return Err(StepPublicationError::WrongValidationMode);
    }
    validate_delta_authority(publication.delta, publication.plan)?;
    let record = publication.delta.compact_completion();
    let commit = match verify_speculative_completion(
        &record,
        publication.plan.request,
        publication.plan.completion_epoch,
        &publication.plan.plan_id,
        token_inputs.draft_tokens,
        token_inputs.target_choices,
    ) {
        Ok(commit) => commit,
        Err(error) => return Err(StepPublicationError::Speculative(error)),
    };
    let accepted_draft_tokens = record.accepted_draft_tokens;
    let permit = SpeculativePublicationPermit {
        commit,
        accepted_draft_tokens,
    };
    assert(permit.valid_for(
        publication,
        expected_request,
        expected_epoch,
        *expected_plan_id,
        expected_selection,
        token_inputs.draft_tokens@,
        token_inputs.target_choices@,
    ));
    Ok(permit)
}

/// Applies a preflighted unpublished-to-validated transition infallibly.
pub(crate) fn apply_preflighted_speculative_validation(
    publication: &mut StepPublication,
    permit: SpeculativePublicationPermit,
    _expected_request: RequestId,
    _expected_epoch: CompletionEpoch,
    _expected_plan_id: &Identity,
    _expected_selection: Qwen3PlanSelection,
    _token_inputs: SpeculativeTokenInputs<'_>,
) -> (commit: GreedyCommit)
    requires permit.valid_for(
        old(publication),
        _expected_request,
        _expected_epoch,
        *_expected_plan_id,
        _expected_selection,
        _token_inputs.draft_tokens@,
        _token_inputs.target_choices@,
    ),
    ensures
        speculative_publication_matches(
            old(publication),
            _expected_request,
            _expected_epoch,
            *_expected_plan_id,
            _expected_selection,
            _token_inputs.draft_tokens@,
            _token_inputs.target_choices@,
            &commit,
        ),
        validation_transition(old(publication), final(publication)),
{
    proof {
        reveal(SpeculativePublicationPermit::valid_for);
        reveal(validation_transition);
        reveal(publication_payload_preserved);
        reveal(publication_phase_matches);
    }
    publication.set_phase(PublicationPhase::Validated);
    permit.commit
}

/// Applies validated publication and one-shot publication with no error path.
pub(crate) fn apply_preflighted_speculative_publication(
    publication: &mut StepPublication,
    _permit: SpeculativePublicationPermit,
    _expected_request: RequestId,
    _expected_epoch: CompletionEpoch,
    _expected_plan_id: &Identity,
    _expected_selection: Qwen3PlanSelection,
    _token_inputs: SpeculativeTokenInputs<'_>,
) -> (delta: ReservedStateDelta)
    requires _permit.valid_for(
        old(publication),
        _expected_request,
        _expected_epoch,
        *_expected_plan_id,
        _expected_selection,
        _token_inputs.draft_tokens@,
        _token_inputs.target_choices@,
    ),
    ensures
        speculative_validation_and_publication_transition(
            old(publication),
            final(publication),
            _expected_request,
            _expected_epoch,
            *_expected_plan_id,
            _expected_selection,
            _token_inputs.draft_tokens@,
            _token_inputs.target_choices@,
        ),
        delta == final(publication).delta_spec(),
        delta.compact_completion_spec().accepted_draft_tokens
            == _permit.accepted_draft_tokens_spec(),
{
    proof {
        reveal(SpeculativePublicationPermit::valid_for);
        reveal(speculative_validation_and_publication_transition);
        reveal(publication_payload_preserved);
        reveal(publication_phase_matches);
        assert(exists|commit: GreedyCommit| speculative_publication_matches(
            old(publication),
            _expected_request,
            _expected_epoch,
            *_expected_plan_id,
            _expected_selection,
            _token_inputs.draft_tokens@,
            _token_inputs.target_choices@,
            &commit,
        )) by {
            let witness = _permit.commit;
            assert(speculative_publication_matches(
                old(publication),
                _expected_request,
                _expected_epoch,
                *_expected_plan_id,
                _expected_selection,
                _token_inputs.draft_tokens@,
                _token_inputs.target_choices@,
                &witness,
            ));
        }
    }
    publication.set_phase(PublicationPhase::Validated);
    publication.set_phase(PublicationPhase::Published);
    publication.delta
}

/// Validates one target speculative delta against exact greedy completion.
///
/// # Errors
///
/// Returns [`StepPublicationError`] unless all immutable authority and phase
/// checks pass and [`verify_speculative_completion`] accepts the exact draft
/// and target-choice sequences.
pub fn validate_speculative_publication(
    publication: &mut StepPublication,
    expected_request: RequestId,
    expected_epoch: CompletionEpoch,
    expected_plan_id: &Identity,
    expected_selection: Qwen3PlanSelection,
    draft_tokens: &[TokenId],
    target_choices: &[TokenId],
) -> (result: Result<GreedyCommit, StepPublicationError>)
    ensures
        match result {
            Ok(commit) => {
                speculative_publication_matches(
                    old(publication),
                    expected_request,
                    expected_epoch,
                    *expected_plan_id,
                    expected_selection,
                    draft_tokens@,
                    target_choices@,
                    &commit,
                ) && validation_transition(old(publication), final(publication))
            },
            Err(_) => *final(publication) == *old(publication),
        },
{
    proof {
        reveal(speculative_publication_matches);
        reveal(validation_transition);
    }
    let token_inputs = SpeculativeTokenInputs {
        draft_tokens,
        target_choices,
    };
    let permit = preflight_speculative_publication(
        publication,
        expected_request,
        expected_epoch,
        expected_plan_id,
        expected_selection,
        token_inputs,
    )?;
    let commit = apply_preflighted_speculative_validation(
        publication,
        permit,
        expected_request,
        expected_epoch,
        expected_plan_id,
        expected_selection,
        token_inputs,
    );
    Ok(commit)
}

/// Publishes one previously validated reserved delta.
///
/// # Errors
///
/// Returns [`StepPublicationError::WrongPhase`] for publication before
/// validation, double publication, or publication after discard.
pub fn publish_reserved_delta(
    publication: &mut StepPublication,
) -> (result: Result<ReservedStateDelta, StepPublicationError>)
    ensures
        result.is_ok() == publication_phase_matches(
            old(publication).phase_spec(),
            PublicationPhase::Validated,
        ),
        match result {
            Ok(delta) => {
                publication_transition(old(publication), final(publication))
                    && delta == final(publication).delta_spec()
            },
            Err(_) => *final(publication) == *old(publication),
        },
{
    let ghost entry = *publication;
    assert(entry == *old(publication));
    proof {
        reveal(StepPublication::phase_spec);
        reveal(StepPublication::delta_spec);
        reveal(publication_phase_matches);
        reveal(publication_transition);
        reveal(publication_payload_preserved);
    }
    if !phase_matches(publication.phase(), PublicationPhase::Validated) {
        assert(*publication == entry);
        return Err(StepPublicationError::WrongPhase);
    }
    publication.set_phase(PublicationPhase::Published);
    assert(publication_transition(&entry, publication));
    Ok(publication.delta)
}

/// Irrevocably discards an unpublished or validated reserved delta.
///
/// # Errors
///
/// Returns [`StepPublicationError::WrongPhase`] after publication or a prior
/// discard. Payload fields remain immutable in every case.
pub fn discard_reserved_delta(
    publication: &mut StepPublication,
) -> (result: Result<ReservedStateDelta, StepPublicationError>)
    ensures
        result.is_ok() == (
            publication_phase_matches(
                old(publication).phase_spec(),
                PublicationPhase::Unpublished,
            ) || publication_phase_matches(
                old(publication).phase_spec(),
                PublicationPhase::Validated,
            )
        ),
        match result {
            Ok(delta) => {
                discard_transition(old(publication), final(publication))
                    && delta == final(publication).delta_spec()
            },
            Err(_) => *final(publication) == *old(publication),
        },
{
    let ghost entry = *publication;
    assert(entry == *old(publication));
    proof {
        reveal(StepPublication::phase_spec);
        reveal(StepPublication::delta_spec);
        reveal(publication_phase_matches);
        reveal(discard_transition);
        reveal(publication_payload_preserved);
    }
    let unpublished = phase_matches(publication.phase(), PublicationPhase::Unpublished);
    let validated = phase_matches(publication.phase(), PublicationPhase::Validated);
    if !unpublished && !validated
    {
        assert(*publication == entry);
        return Err(StepPublicationError::WrongPhase);
    }
    publication.set_phase(PublicationPhase::Discarded);
    assert(discard_transition(&entry, publication));
    Ok(publication.delta)
}

} // verus!

#[cfg(test)]
mod tests {
    use super::{
        discard_reserved_delta, publish_reserved_delta, validate_direct_publication,
        validate_speculative_publication, PublicationPhase, ReservedStateDelta, StepPlan,
        StepPublication, StepPublicationError,
    };
    use crate::completion::CompletionEpoch;
    use crate::{
        CompactCompletionRecord, Identity, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket,
        Qwen3PlanSelection, RequestId, SpeculativeCompletionError, M1_MAX_COMPLETION_TOKENS,
    };

    const REQUEST: RequestId = RequestId::new(3, 7);
    const EPOCH: CompletionEpoch = CompletionEpoch::new(11);

    const fn identity(byte: u8) -> Identity {
        Identity::new([byte; 32])
    }

    const fn selection(mode: Qwen3ExecutionMode, bucket: Qwen3PlanBucket) -> Qwen3PlanSelection {
        Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode,
            bucket,
        }
    }

    fn record(accepted: u8, emitted: &[u32]) -> CompactCompletionRecord {
        let mut emitted_tokens = [0; M1_MAX_COMPLETION_TOKENS];
        emitted_tokens[..emitted.len()].copy_from_slice(emitted);
        CompactCompletionRecord {
            request: REQUEST,
            epoch: EPOCH,
            plan_id: identity(5),
            accepted_draft_tokens: accepted,
            emitted_token_count: u8::try_from(emitted.len()).unwrap(),
            emitted_tokens,
        }
    }

    fn reserved_publication(
        selected: Qwen3PlanSelection,
        observed: Qwen3PlanSelection,
        completion: CompactCompletionRecord,
    ) -> StepPublication {
        StepPublication::reserve(
            StepPlan::new(REQUEST, EPOCH, identity(5), selected),
            ReservedStateDelta::from_compact_completion(completion, observed),
        )
    }

    fn direct_selection() -> Qwen3PlanSelection {
        selection(Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS1C8192)
    }

    fn speculative_selection() -> Qwen3PlanSelection {
        selection(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
        )
    }

    #[test]
    fn direct_delta_validates_and_publishes_exactly_once() {
        let selected = direct_selection();
        let mut publication = reserved_publication(selected, selected, record(0, &[19]));
        validate_direct_publication(&mut publication, REQUEST, EPOCH, &identity(5), selected)
            .unwrap();
        assert_eq!(publication.phase(), PublicationPhase::Validated);
        let delta = publish_reserved_delta(&mut publication).unwrap();
        assert_eq!(delta.accepted_token_count(), 0);
        assert_eq!(delta.emitted_token_count(), 1);
        assert_eq!(delta.emitted_tokens()[0], 19);
        assert_eq!(publication.phase(), PublicationPhase::Published);
        assert_eq!(
            publish_reserved_delta(&mut publication),
            Err(StepPublicationError::WrongPhase)
        );
    }

    #[test]
    fn speculative_delta_composes_exact_greedy_completion() {
        let selected = speculative_selection();
        let mut publication = reserved_publication(selected, selected, record(2, &[3, 4, 9]));
        let commit = validate_speculative_publication(
            &mut publication,
            REQUEST,
            EPOCH,
            &identity(5),
            selected,
            &[3, 4, 5],
            &[3, 4, 9, 6],
        )
        .unwrap();
        assert_eq!(commit.accepted_draft_tokens(), 2);
        assert_eq!(commit.emitted_tokens(), &[3, 4, 9]);
        assert_eq!(publication.phase(), PublicationPhase::Validated);
        assert_eq!(
            publish_reserved_delta(&mut publication)
                .unwrap()
                .emitted_token_count(),
            3
        );
    }

    #[test]
    fn stale_request_generation_epoch_and_plan_identity_fail_closed() {
        let selected = direct_selection();
        for (completion, expected_error) in [
            (
                CompactCompletionRecord {
                    request: RequestId::new(3, 8),
                    ..record(0, &[19])
                },
                StepPublicationError::DeltaAuthorityMismatch,
            ),
            (
                CompactCompletionRecord {
                    epoch: CompletionEpoch::new(12),
                    ..record(0, &[19])
                },
                StepPublicationError::DeltaAuthorityMismatch,
            ),
            (
                CompactCompletionRecord {
                    plan_id: identity(6),
                    ..record(0, &[19])
                },
                StepPublicationError::DeltaAuthorityMismatch,
            ),
        ] {
            let mut publication = reserved_publication(selected, selected, completion);
            assert_eq!(
                validate_direct_publication(
                    &mut publication,
                    REQUEST,
                    EPOCH,
                    &identity(5),
                    selected,
                ),
                Err(expected_error)
            );
            assert_eq!(publication.phase(), PublicationPhase::Unpublished);
        }

        let mut stale_plan = reserved_publication(selected, selected, record(0, &[19]));
        assert_eq!(
            validate_direct_publication(
                &mut stale_plan,
                RequestId::new(3, 8),
                EPOCH,
                &identity(5),
                selected,
            ),
            Err(StepPublicationError::RequestMismatch)
        );
    }

    #[test]
    fn role_mode_and_bucket_substitution_fail_closed() {
        let selected = direct_selection();
        let wrong_bucket = selection(Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS8C8192);
        let mut publication = reserved_publication(selected, wrong_bucket, record(0, &[19]));
        assert_eq!(
            validate_direct_publication(&mut publication, REQUEST, EPOCH, &identity(5), selected),
            Err(StepPublicationError::DeltaAuthorityMismatch)
        );

        let mut wrong_mode = reserved_publication(selected, selected, record(0, &[19]));
        assert_eq!(
            validate_speculative_publication(
                &mut wrong_mode,
                REQUEST,
                EPOCH,
                &identity(5),
                selected,
                &[],
                &[19],
            ),
            Err(StepPublicationError::WrongValidationMode)
        );

        let draft_selection = Qwen3PlanSelection {
            role: Qwen3ModelRole::Draft06B,
            mode: Qwen3ExecutionMode::Decode,
            bucket: Qwen3PlanBucket::DecodeS1C8192,
        };
        let mut draft = reserved_publication(draft_selection, draft_selection, record(0, &[19]));
        assert_eq!(
            validate_direct_publication(&mut draft, REQUEST, EPOCH, &identity(5), draft_selection,),
            Err(StepPublicationError::NonTargetPublication)
        );
    }

    #[test]
    fn publish_before_validation_and_rejected_suffix_fail_closed() {
        let direct = direct_selection();
        let mut unpublished = reserved_publication(direct, direct, record(0, &[19]));
        assert_eq!(
            publish_reserved_delta(&mut unpublished),
            Err(StepPublicationError::WrongPhase)
        );

        let selected = speculative_selection();
        let mut rejected_suffix = reserved_publication(selected, selected, record(2, &[3, 4, 5]));
        assert_eq!(
            validate_speculative_publication(
                &mut rejected_suffix,
                REQUEST,
                EPOCH,
                &identity(5),
                selected,
                &[3, 4, 5],
                &[3, 4, 9, 6],
            ),
            Err(StepPublicationError::Speculative(
                SpeculativeCompletionError::EmittedTokenMismatch
            ))
        );
        assert_eq!(rejected_suffix.phase(), PublicationPhase::Unpublished);
    }

    #[test]
    fn discard_is_terminal_before_or_after_validation() {
        let selected = direct_selection();
        let mut unpublished = reserved_publication(selected, selected, record(0, &[19]));
        discard_reserved_delta(&mut unpublished).unwrap();
        assert_eq!(unpublished.phase(), PublicationPhase::Discarded);
        assert_eq!(
            validate_direct_publication(&mut unpublished, REQUEST, EPOCH, &identity(5), selected),
            Err(StepPublicationError::WrongPhase)
        );
        assert_eq!(
            discard_reserved_delta(&mut unpublished),
            Err(StepPublicationError::WrongPhase)
        );

        let mut validated = reserved_publication(selected, selected, record(0, &[19]));
        validate_direct_publication(&mut validated, REQUEST, EPOCH, &identity(5), selected)
            .unwrap();
        discard_reserved_delta(&mut validated).unwrap();
        assert_eq!(validated.phase(), PublicationPhase::Discarded);
        assert_eq!(
            publish_reserved_delta(&mut validated),
            Err(StepPublicationError::WrongPhase)
        );
    }
}
