#![forbid(unsafe_code)]

//! M1 source-level request-noninterference theorems.
//!
//! These executable proof roots compose the exact scheduler slot frame with
//! generation-owned target/draft physical-KV custody, structurally validated
//! fixed-shape input rows, compact completion routing, and atomic speculative
//! publication. Every modeled operation frames a distinct nonselected request;
//! every atomic rejection additionally frames the selected publication and KV.
//!
//! The workspace result is a logical lane/row separation theorem, not an
//! allocation or address-separation theorem. Device bytes, physical workspace
//! subleases, queue execution/readback, driver and hardware behavior, numerical
//! refinement, timing, traffic analysis, and resource-contention side channels
//! remain outside this source-level boundary.

#[allow(unused_imports)]
use ferric_spec::completion::CompletionEpoch;
#[allow(unused_imports)]
use ferric_spec::{
    AtomicSpeculativeStepError, AtomicSpeculativeStepOutcome, CompactCompletionError,
    CompactCompletionRecord, ContinuousBatch, IsolatedKvAction, IsolatedRequestKv,
    IsolatedSchedulerAction, IsolatedSpeculativeKvExpectation, PhysicalPageId, Qwen3ModelRole,
    RequestId, RequestIsolationError, SpeculativeKvRoundIndex, SpeculativeTokenInputs,
    StepPublication, ValidatedM1StepInputs, M1_CONTINUOUS_BATCH_CAPACITY,
};
#[allow(unused_imports)]
use vstd::prelude::*;

verus! {

/// Exact scheduler-slot and physical-KV frame for a distinct nonselected request.
pub open spec fn m1_other_request_preserved(
    before_batch: &ContinuousBatch,
    after_batch: &ContinuousBatch,
    before_other: &IsolatedRequestKv,
    after_other: &IsolatedRequestKv,
    selected_request: RequestId,
) -> bool {
    &&& before_other.request_spec().slot_spec() < M1_CONTINUOUS_BATCH_CAPACITY
    &&& before_other.request_spec().slot_spec() != selected_request.slot_spec()
    &&& after_other.exact_physical_frame(before_other)
    &&& after_batch.slots_spec()[before_other.request_spec().slot_spec() as int]
        == before_batch.slots_spec()[before_other.request_spec().slot_spec() as int]
}

/// Exact request, plan, epoch, and disjoint logical row ownership of two live lanes.
pub open spec fn m1_workspace_lanes_isolated(
    inputs: &ValidatedM1StepInputs,
    selected_lane: int,
    other_lane: int,
) -> bool {
    let selected = inputs.lanes_spec()[selected_lane].unwrap();
    let other = inputs.lanes_spec()[other_lane].unwrap();
    let width = inputs.dimensions_spec().active_tokens as int;
    &&& inputs.valid()
    &&& 0 <= selected_lane < inputs.live_lanes_spec()
    &&& 0 <= other_lane < inputs.live_lanes_spec()
    &&& selected_lane != other_lane
    &&& width > 0
    &&& selected.selection_spec() == inputs.selection_spec()
    &&& other.selection_spec() == inputs.selection_spec()
    &&& selected.plan_id_spec() == other.plan_id_spec()
    &&& selected.completion_epoch_spec() == other.completion_epoch_spec()
    &&& selected.request_spec().slot_spec() != other.request_spec().slot_spec()
    &&& if selected_lane < other_lane {
        (selected_lane + 1) * width <= other_lane * width
    } else {
        (other_lane + 1) * width <= selected_lane * width
    }
}

/// Exact source-level route accepted for one compact completion.
pub open spec fn m1_completion_route_isolated(
    record: CompactCompletionRecord,
    expected_request: RequestId,
    other_request: RequestId,
    expected_epoch: CompletionEpoch,
    expected_plan_id: ferric_spec::Identity,
    draft_token_count: u8,
) -> bool {
    &&& ferric_spec::compact_completion_matches(
        record,
        expected_request,
        expected_epoch,
        expected_plan_id,
        draft_token_count,
    )
    &&& record.request.slot_spec() == expected_request.slot_spec()
    &&& record.request.generation_spec() == expected_request.generation_spec()
    &&& record.epoch == expected_epoch
    &&& record.plan_id.bytes_spec() == expected_plan_id.bytes_spec()
    &&& record.request.slot_spec() != other_request.slot_spec()
}

/// Applies one scheduler transition while framing the distinct request's exact
/// scheduler slot and complete target/draft physical owner.
///
/// # Errors
///
/// Returns the exact source-level routing or scheduler error. Rejection also
/// preserves the selected physical owner.
pub fn m1_isolated_scheduler_step_theorem(
    batch: &mut ContinuousBatch,
    selected: &mut IsolatedRequestKv,
    other: &mut IsolatedRequestKv,
    request: RequestId,
    action: IsolatedSchedulerAction,
) -> (result: Result<(), RequestIsolationError>)
    requires
        old(batch).valid(),
        old(other).request_spec().slot_spec() < M1_CONTINUOUS_BATCH_CAPACITY,
        old(other).request_spec().slot_spec() != request.slot_spec(),
    ensures
        final(batch).valid(),
        m1_other_request_preserved(
            old(batch),
            final(batch),
            old(other),
            final(other),
            request,
        ),
        result.is_err() ==> *final(selected) == *old(selected),
{
    let ghost entry_batch = *batch;
    let ghost entry_other = *other;
    let result = ferric_spec::apply_isolated_scheduler_step(
        batch,
        selected,
        other,
        request,
        action,
    );
    proof {
        ferric_spec::request_isolation::isolated_action_preserves_other_request(
            &entry_batch,
            batch,
            &entry_other,
            other,
            request,
        );
        reveal(m1_other_request_preserved);
    }
    result
}

/// Cancels one request while framing the distinct request's scheduler and
/// target/draft physical state.
///
/// # Errors
///
/// Returns the exact fail-closed cancellation error.
pub fn m1_isolated_cancel_theorem(
    batch: &mut ContinuousBatch,
    selected: &mut IsolatedRequestKv,
    other: &mut IsolatedRequestKv,
    request: RequestId,
) -> (result: Result<(), RequestIsolationError>)
    requires
        old(batch).valid(),
        old(other).request_spec().slot_spec() < M1_CONTINUOUS_BATCH_CAPACITY,
        old(other).request_spec().slot_spec() != request.slot_spec(),
    ensures
        final(batch).valid(),
        m1_other_request_preserved(
            old(batch),
            final(batch),
            old(other),
            final(other),
            request,
        ),
{
    let ghost entry_batch = *batch;
    let ghost entry_other = *other;
    let result = ferric_spec::cancel_isolated_request(batch, selected, other, request);
    proof {
        ferric_spec::request_isolation::isolated_action_preserves_other_request(
            &entry_batch,
            batch,
            &entry_other,
            other,
            request,
        );
        reveal(m1_other_request_preserved);
    }
    result
}

/// Applies one request-owned KV transition while framing the scheduler batch
/// and the distinct request's complete physical owner.
///
/// # Errors
///
/// Returns the exact lifecycle, routing, role, epoch, or physical-KV error.
/// Rejection also preserves the selected physical owner.
pub fn m1_isolated_kv_step_theorem(
    batch: &ContinuousBatch,
    selected: &mut IsolatedRequestKv,
    other: &mut IsolatedRequestKv,
    request: RequestId,
    role: Qwen3ModelRole,
    action: IsolatedKvAction,
) -> (result: Result<Option<PhysicalPageId>, RequestIsolationError>)
    requires
        batch.valid(),
        old(other).request_spec().slot_spec() < M1_CONTINUOUS_BATCH_CAPACITY,
        old(other).request_spec().slot_spec() != request.slot_spec(),
    ensures
        m1_other_request_preserved(
            batch,
            batch,
            old(other),
            final(other),
            request,
        ),
        result.is_err() ==> *final(selected) == *old(selected),
{
    let ghost entry_other = *other;
    let result = ferric_spec::apply_isolated_kv_action(
        batch,
        selected,
        other,
        request,
        role,
        action,
    );
    proof {
        ferric_spec::request_isolation::equal_isolated_request_kv_is_exact_frame(
            &entry_other,
            other,
        );
        reveal(m1_other_request_preserved);
    }
    result
}

/// Releases one exact retired page generation while framing every state field
/// owned by the distinct request.
///
/// # Errors
///
/// Returns the exact routing, quiescence, generation, or physical release error.
pub fn m1_isolated_page_release_theorem(
    batch: &ContinuousBatch,
    selected: &mut IsolatedRequestKv,
    other: &mut IsolatedRequestKv,
    request: RequestId,
    role: Qwen3ModelRole,
    page: PhysicalPageId,
    exact_epoch: CompletionEpoch,
) -> (result: Result<PhysicalPageId, RequestIsolationError>)
    requires
        batch.valid(),
        old(other).request_spec().slot_spec() < M1_CONTINUOUS_BATCH_CAPACITY,
        old(other).request_spec().slot_spec() != request.slot_spec(),
    ensures
        m1_other_request_preserved(
            batch,
            batch,
            old(other),
            final(other),
            request,
        ),
        result.is_err() ==> *final(selected) == *old(selected),
        result.is_ok() ==> ferric_spec::request_isolation::isolated_exact_page_release_transition(
            old(selected),
            final(selected),
            request,
            role,
            page,
            result.unwrap(),
            exact_epoch,
        ),
{
    let ghost entry_other = *other;
    let result = ferric_spec::release_isolated_page(
        batch,
        selected,
        other,
        request,
        role,
        page,
        exact_epoch,
    );
    proof {
        ferric_spec::request_isolation::equal_isolated_request_kv_is_exact_frame(
            &entry_other,
            other,
        );
        reveal(m1_other_request_preserved);
    }
    result
}

/// Detaches one fully quiescent request generation while framing the distinct
/// request's exact scheduler slot and physical owner.
///
/// # Errors
///
/// Returns the exact fail-closed detachment error.
pub fn m1_isolated_detach_theorem(
    batch: &mut ContinuousBatch,
    selected: &mut IsolatedRequestKv,
    other: &mut IsolatedRequestKv,
    request: RequestId,
) -> (result: Result<RequestId, RequestIsolationError>)
    requires
        old(batch).valid(),
        old(other).request_spec().slot_spec() < M1_CONTINUOUS_BATCH_CAPACITY,
        old(other).request_spec().slot_spec() != request.slot_spec(),
    ensures
        final(batch).valid(),
        m1_other_request_preserved(
            old(batch),
            final(batch),
            old(other),
            final(other),
            request,
        ),
{
    let ghost entry_batch = *batch;
    let ghost entry_other = *other;
    let result = ferric_spec::detach_isolated_request(batch, selected, other, request);
    proof {
        ferric_spec::request_isolation::isolated_action_preserves_other_request(
            &entry_batch,
            batch,
            &entry_other,
            other,
            request,
        );
        reveal(m1_other_request_preserved);
    }
    result
}

/// Proves that two validated live input lanes own distinct request slots and
/// nonoverlapping fixed-shape token/position row intervals.
pub fn m1_workspace_lane_noninterference_theorem(
    _inputs: &ValidatedM1StepInputs,
    _selected_lane: u32,
    _other_lane: u32,
)
    requires
        _inputs.valid(),
        _selected_lane < _inputs.live_lanes_spec(),
        _other_lane < _inputs.live_lanes_spec(),
        _selected_lane != _other_lane,
    ensures m1_workspace_lanes_isolated(
        _inputs,
        _selected_lane as int,
        _other_lane as int,
    ),
{
    proof {
        ferric_spec::validated_m1_step_input_lane_isolation(
            _inputs,
            _selected_lane as int,
            _other_lane as int,
        );
        reveal(m1_workspace_lanes_isolated);
    }
}

/// Validates one compact completion against its exact generational request,
/// epoch, and plan without admitting it for a distinct request slot.
///
/// # Errors
///
/// Returns the exact compact-completion validation error.
pub fn m1_completion_routing_theorem(
    record: &CompactCompletionRecord,
    expected_request: RequestId,
    _other_request: RequestId,
    expected_epoch: CompletionEpoch,
    expected_plan_id: &ferric_spec::Identity,
    draft_token_count: u8,
) -> (result: Result<(), CompactCompletionError>)
    requires expected_request.slot_spec() != _other_request.slot_spec(),
    ensures match result {
        Ok(()) => m1_completion_route_isolated(
            *record,
            expected_request,
            _other_request,
            expected_epoch,
            *expected_plan_id,
            draft_token_count,
        ),
        Err(_) => !ferric_spec::compact_completion_matches(
            *record,
            expected_request,
            expected_epoch,
            *expected_plan_id,
            draft_token_count,
        ),
    },
{
    let result = ferric_spec::validate_compact_completion(
        record,
        expected_request,
        expected_epoch,
        expected_plan_id,
        draft_token_count,
    );
    proof {
        if result.is_ok() {
            ferric_spec::compact_completion_matches_exposes_route(
                *record,
                expected_request,
                expected_epoch,
                *expected_plan_id,
                draft_token_count,
            );
            reveal(m1_completion_route_isolated);
        }
    }
    result
}

/// Atomically settles selected target/draft KV and publishes its completion
/// while framing the scheduler, the distinct request's KV, and its publication.
///
/// # Errors
///
/// Every rejection preserves both publications, both request KV owners, and
/// the complete scheduler batch.
#[allow(clippy::too_many_arguments)]
pub fn m1_atomic_request_noninterference_theorem(
    batch: &mut ContinuousBatch,
    publication: &mut StepPublication,
    selected: &mut IsolatedRequestKv,
    other: &mut IsolatedRequestKv,
    _other_publication: &mut StepPublication,
    index: &SpeculativeKvRoundIndex,
    expected: &IsolatedSpeculativeKvExpectation,
    token_inputs: SpeculativeTokenInputs<'_>,
) -> (result: Result<AtomicSpeculativeStepOutcome, AtomicSpeculativeStepError>)
    requires
        old(batch).valid(),
        old(other).request_spec().slot_spec() < M1_CONTINUOUS_BATCH_CAPACITY,
        old(other).request_spec().slot_spec() != index.request.slot_spec(),
    ensures
        *final(batch) == *old(batch),
        m1_other_request_preserved(
            old(batch),
            final(batch),
            old(other),
            final(other),
            index.request,
        ),
        *final(_other_publication) == *old(_other_publication),
        match result {
            Ok(outcome) => {
                ferric_spec::speculative_step_composition::atomic_speculative_step_transition(
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
            },
            Err(_) => {
                &&& *final(publication) == *old(publication)
                &&& *final(selected) == *old(selected)
            },
        },
{
    let ghost entry_other = *other;
    let result = ferric_spec::settle_and_publish_speculative_step(
        batch,
        publication,
        selected,
        other,
        index,
        expected,
        token_inputs,
    );
    proof {
        ferric_spec::request_isolation::equal_isolated_request_kv_is_exact_frame(
            &entry_other,
            other,
        );
        reveal(m1_other_request_preserved);
    }
    result
}

} // verus!
