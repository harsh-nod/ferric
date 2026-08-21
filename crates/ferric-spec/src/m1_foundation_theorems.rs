//! Query-bearing theorem wrappers for the finite M1 foundation registry.
//!
//! Each loop-free wrapper composes an existing executable contract into one
//! row-specific consequence. These are source-level proof foundations only;
//! they do not close an M1 property or future path and add no runtime, device,
//! hardware, numerical, or performance authority.

use crate::completion::CompletionEpoch;
use crate::continuous_batching::{
    apply_continuous_batch_step, apply_continuous_publish_step, ContinuousBatch,
    ContinuousBatchAction, ContinuousBatchError, ContinuousRequest,
};
use crate::graph::{
    expected_step, plan_step_count, Qwen3ExecutionMode, Qwen3PlanBucket, Qwen3PlanStep,
};
use crate::paged_kv_refinement::{
    release_retired_page, rollback_physical_token, write_physical_token, KvQuiescenceAuthority,
    PhysicalKvError, PhysicalKvState, PhysicalPageId,
};
use crate::speculative_step_composition::{
    settle_and_publish_speculative_step, AtomicSpeculativeStepError, AtomicSpeculativeStepOutcome,
};
use crate::step_plan_publication::{
    publish_reserved_delta, validate_step_plan, ReservedStateDelta, SpeculativeTokenInputs,
    StepPlan, StepPublication, StepPublicationError,
};
use crate::{
    Identity, IsolatedRequestKv, IsolatedSpeculativeKvExpectation, Qwen3ModelRole,
    Qwen3PlanSelection, RequestId, SpeculativeKvRoundIndex,
};
use vstd::prelude::*;

verus! {

/// An already-published active epoch rejects a second token publication.
pub fn batching_publish_once_theorem(
    current: ContinuousRequest,
    epoch: CompletionEpoch,
    emitted_tokens: u8,
) -> (result: Result<ContinuousRequest, ContinuousBatchError>)
    requires
        current.active_epoch_spec() == Some(epoch),
        crate::continuous_batching::publication_ready(current),
        current.published_for_active_epoch_spec(),
    ensures result == Err(ContinuousBatchError::AlreadyPublished),
{
    let result = apply_continuous_publish_step(current, epoch, emitted_tokens);
    assert(result == crate::continuous_batching::continuous_publish_step(current, epoch, emitted_tokens));
    proof {
        crate::continuous_batching::already_published_epoch_rejects(
            current,
            epoch,
            emitted_tokens,
        );
    }
    assert(result == Err(ContinuousBatchError::AlreadyPublished));
    result
}

/// An in-range stale generation is rejected and frames the entire batch.
pub fn batching_request_routing_theorem(
    batch: &mut ContinuousBatch,
    request: RequestId,
    action: ContinuousBatchAction,
) -> (result: Result<(), ContinuousBatchError>)
    requires
        old(batch).valid(),
        request.slot_spec() < crate::continuous_batching::M1_CONTINUOUS_BATCH_CAPACITY,
        old(batch).slots_spec()[request.slot_spec() as int].generation_spec()
            != request.generation_spec(),
    ensures
        result == Err(ContinuousBatchError::StaleGeneration),
        *final(batch) == *old(batch),
{
    let ghost entry = *batch;
    assert(entry == *old(batch));
    let result = apply_continuous_batch_step(batch, request, action);
    proof {
        crate::continuous_batching::stale_generation_is_expected_error(
            &entry,
            request,
            action,
        );
    }
    assert(result == Err(ContinuousBatchError::StaleGeneration));
    assert(*batch == entry);
    result
}

/// The executable graph lookup returns the exact admitted operator step.
pub fn graph_operator_order_theorem(
    role: Qwen3ModelRole,
    mode: Qwen3ExecutionMode,
    bucket: Qwen3PlanBucket,
    ordinal: u32,
    admitted: Qwen3PlanStep,
) -> (step: Option<Qwen3PlanStep>)
    requires crate::graph::expected_step_spec(role, mode, bucket, ordinal) == Some(admitted),
    ensures step == Some(admitted),
{
    let _ = admitted;
    let step = expected_step(role, mode, bucket, ordinal);
    assert(step == crate::graph::expected_step_spec(role, mode, bucket, ordinal));
    assert(step == Some(admitted));
    step
}

/// Target and draft graphs retain their distinct exact finite step counts.
pub fn graph_role_step_count_theorem(role: Qwen3ModelRole) -> (count: u32)
    ensures count == match role {
        Qwen3ModelRole::Target8B => crate::graph::QWEN3_TARGET_PLAN_STEPS,
        Qwen3ModelRole::Draft06B => crate::graph::QWEN3_DRAFT_PLAN_STEPS,
    },
{
    let count = plan_step_count(role);
    assert(count == crate::graph::plan_step_count_spec(role));
    proof {
        crate::graph::plan_step_count_is_role_exact(role);
    }
    assert(count == match role {
        Qwen3ModelRole::Target8B => crate::graph::QWEN3_TARGET_PLAN_STEPS,
        Qwen3ModelRole::Draft06B => crate::graph::QWEN3_DRAFT_PLAN_STEPS,
    });
    count
}

/// A selected request action preserves every distinct request slot.
pub fn isolation_other_request_frame_theorem(
    batch: &mut ContinuousBatch,
    request: RequestId,
    action: ContinuousBatchAction,
    other_slot: usize,
) -> (result: Result<(), ContinuousBatchError>)
    requires
        old(batch).valid(),
        other_slot < crate::continuous_batching::M1_CONTINUOUS_BATCH_CAPACITY,
        other_slot as int != request.slot_spec(),
    ensures
        final(batch).slots_spec()[other_slot as int]
            == old(batch).slots_spec()[other_slot as int],
{
    let _ = other_slot;
    let ghost entry = *batch;
    assert(entry == *old(batch));
    let result = apply_continuous_batch_step(batch, request, action);
    if result.is_ok() {
        proof {
            crate::continuous_batching::successful_batch_step_preserves_other_request(
                &entry,
                batch,
                request,
                action,
                other_slot as int,
            );
        }
    }
    assert(batch.slots_spec()[other_slot as int]
        == entry.slots_spec()[other_slot as int]);
    result
}

/// Successful retired-page release advances exactly one physical generation.
pub fn kv_release_generation_theorem(
    state: &mut PhysicalKvState,
    page: PhysicalPageId,
    authority: &KvQuiescenceAuthority,
) -> (result: Result<PhysicalPageId, PhysicalKvError>)
    ensures
        result.is_ok() == crate::paged_kv_refinement::release_retired_enabled(old(state), page, authority),
        result.is_ok() ==> {
            &&& crate::paged_kv_refinement::release_retired_transition(old(state), final(state), page)
            &&& result.unwrap().index_spec() == page.index_spec()
            &&& result.unwrap().generation_spec() as int
                == page.generation_spec() as int + 1
        },
        result.is_err() ==> *final(state) == *old(state),
{
    let result = release_retired_page(state, page, authority);
    if result.is_ok() {
        let released = result.unwrap();
        let _ = released;
        assert(crate::paged_kv_refinement::released_generation_matches(released, page));
        proof {
            crate::paged_kv_refinement::released_generation_has_exact_successor(
                released,
                page,
            );
        }
        assert(released.index_spec() == page.index_spec());
        assert(released.generation_spec() as int == page.generation_spec() as int + 1);
    }
    result
}

/// Successful rollback removes exactly the tentative tail and nothing else.
pub fn kv_rollback_retirement_theorem(
    state: &mut PhysicalKvState,
    request: RequestId,
    selection: Qwen3PlanSelection,
    after_epoch: CompletionEpoch,
) -> (result: Result<(), PhysicalKvError>)
    ensures
        result.is_ok() == crate::paged_kv_refinement::rollback_enabled(old(state), request, selection, after_epoch),
        result.is_ok() ==> crate::paged_kv_refinement::rollback_tail_transition(old(state), final(state), after_epoch),
        result.is_err() ==> *final(state) == *old(state),
{
    let ghost entry = *state;
    assert(entry == *old(state));
    let result = rollback_physical_token(state, request, selection, after_epoch);
    assert(result.is_ok() == crate::paged_kv_refinement::rollback_enabled(&entry, request, selection, after_epoch));
    assert(result.is_ok() ==> crate::paged_kv_refinement::rollback_tail_transition(&entry, state, after_epoch));
    result
}

/// Successful write extends exactly the initialized logical prefix.
pub fn kv_write_prefix_theorem(
    state: &mut PhysicalKvState,
    request: RequestId,
    selection: Qwen3PlanSelection,
    logical_position: u32,
) -> (result: Result<(), PhysicalKvError>)
    ensures
        result.is_ok() == crate::paged_kv_refinement::write_token_enabled(old(state), request, selection, logical_position),
        result.is_ok() ==> crate::paged_kv_refinement::write_at_transition(old(state), final(state), logical_position),
        result.is_err() ==> *final(state) == *old(state),
{
    let ghost entry = *state;
    assert(entry == *old(state));
    let result = write_physical_token(state, request, selection, logical_position);
    assert(result.is_ok() == crate::paged_kv_refinement::write_token_enabled(&entry, request, selection, logical_position));
    assert(result.is_ok() ==> crate::paged_kv_refinement::write_at_transition(&entry, state, logical_position));
    result
}

/// Successful one-shot publication moves exactly Validated to Published.
pub fn publication_phase_transition_theorem(
    publication: &mut StepPublication,
) -> (result: Result<ReservedStateDelta, StepPublicationError>)
    ensures
        result.is_ok() == crate::step_plan_publication::publication_phase_matches(
            old(publication).phase_spec(),
            crate::step_plan_publication::PublicationPhase::Validated,
        ),
        result.is_ok() ==> {
            &&& crate::step_plan_publication::publication_transition(old(publication), final(publication))
            &&& crate::step_plan_publication::publication_phase_matches(
                final(publication).phase_spec(),
                crate::step_plan_publication::PublicationPhase::Published,
            )
        },
        result.is_err() ==> *final(publication) == *old(publication),
{
    let ghost entry = *publication;
    assert(entry == *old(publication));
    let result = publish_reserved_delta(publication);
    if result.is_ok() {
        assert(crate::step_plan_publication::publication_transition(&entry, publication));
        proof {
            crate::step_plan_publication::publication_transition_reaches_published(
                &entry,
                publication,
            );
        }
        assert(crate::step_plan_publication::publication_phase_matches(
            publication.phase_spec(),
            crate::step_plan_publication::PublicationPhase::Published,
        ));
    }
    result
}

/// Plan validation accepts exactly the complete immutable plan authority.
pub fn publication_plan_identity_theorem(
    plan: StepPlan,
    expected_request: RequestId,
    expected_epoch: CompletionEpoch,
    expected_plan_id: &Identity,
    expected_selection: Qwen3PlanSelection,
) -> (result: Result<(), StepPublicationError>)
    ensures
        result.is_ok() == crate::step_plan_publication::step_plan_matches(
            plan,
            expected_request,
            expected_epoch,
            *expected_plan_id,
            expected_selection,
        ),
        result.is_ok() ==> {
            &&& crate::m1_completion::identity_present(*expected_plan_id)
            &&& crate::step_plan_publication::target_publication_role(expected_selection.role)
        },
{
    let result = validate_step_plan(
        plan,
        expected_request,
        expected_epoch,
        expected_plan_id,
        expected_selection,
    );
    if result.is_ok() {
        assert(crate::step_plan_publication::step_plan_matches(
            plan,
            expected_request,
            expected_epoch,
            *expected_plan_id,
            expected_selection,
        ));
        proof {
            crate::step_plan_publication::matching_step_plan_has_expected_authority(
                plan,
                expected_request,
                expected_epoch,
                *expected_plan_id,
                expected_selection,
            );
        }
        assert(crate::m1_completion::identity_present(*expected_plan_id));
        assert(crate::step_plan_publication::target_publication_role(expected_selection.role));
    }
    result
}

/// Success binds KV settlement to the exact publication-derived accepted count.
pub fn speculative_accepted_count_binding_theorem(
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
            Ok(outcome) => {
                &&& crate::speculative_step_composition::atomic_speculative_step_transition(
                    old(publication),
                    final(publication),
                    old(selected),
                    final(selected),
                    index,
                    expected,
                    token_inputs.draft_tokens@,
                    token_inputs.target_choices@,
                    outcome,
                )
                &&& outcome.settlement.accepted_draft_tokens
                    == outcome.published_delta.compact_completion_spec()
                        .accepted_draft_tokens
            },
            Err(_) => true,
        },
{
    let ghost entry_publication = *publication;
    let ghost entry_selected = *selected;
    assert(entry_publication == *old(publication));
    assert(entry_selected == *old(selected));
    let result = settle_and_publish_speculative_step(
        batch,
        publication,
        selected,
        other,
        index,
        expected,
        token_inputs,
    );
    if result.is_ok() {
        let outcome = result.as_ref().unwrap();
        let _ = outcome;
        proof {
            crate::speculative_step_composition::atomic_transition_binds_accepted_count(
                &entry_publication,
                publication,
                &entry_selected,
                selected,
                index,
                expected,
                token_inputs.draft_tokens@,
                token_inputs.target_choices@,
                *outcome,
            );
        }
    }
    result
}

/// Every rejection frames scheduler, publication, selected KV, and other KV.
pub fn speculative_atomic_failure_frame_theorem(
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
            Ok(_) => true,
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
    let result = settle_and_publish_speculative_step(
        batch,
        publication,
        selected,
        other,
        index,
        expected,
        token_inputs,
    );
    proof {
        assert(match result {
            Ok(_) => true,
            Err(_) => {
                &&& *publication == entry_publication
                &&& *selected == entry_selected
            },
        });
    }
    result
}

} // verus!
