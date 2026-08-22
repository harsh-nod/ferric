#![forbid(unsafe_code)]

//! M1 continuous-batching source-level composition theorems.
//!
//! These executable binders compose the fixed-capacity logical batch oracle
//! with its request-local lifecycle relation. They prove exact routing by
//! generational request identity, legal selected-request transitions,
//! publish-once, rejection framing, and preservation of every nonselected
//! request slot.
//!
//! They do not refine a physical scheduler, queue, device, or KV bytes; cover
//! multiple queues; establish hardware or numerical behavior; qualify
//! performance; or close M1.

#[allow(unused_imports)]
use ferric_spec::{
    apply_continuous_batch_step, ContinuousBatch, ContinuousBatchAction, ContinuousBatchError,
    RequestId,
};
#[allow(unused_imports)]
use vstd::prelude::*;

verus! {

/// Exact source-level consequence of one routed continuous-batch action.
pub open spec fn m1_continuous_batch_action_refines(
    before: &ContinuousBatch,
    after: &ContinuousBatch,
    request: RequestId,
    action: ContinuousBatchAction,
    result: &Result<(), ContinuousBatchError>,
) -> bool {
    &&& after.valid()
    &&& match result {
        Ok(()) => ferric_spec::continuous_batching::continuous_batch_step_refines(
            before,
            after,
            request,
            action,
        ),
        Err(error) => {
            &&& ferric_spec::continuous_batching::continuous_batch_expected_error(
                before,
                request,
                action,
            ) == Some(*error)
            &&& *after == *before
        },
    }
    &&& forall|slot: int|
        0 <= slot < ferric_spec::continuous_batching::M1_CONTINUOUS_BATCH_CAPACITY
        && slot != request.slot_spec()
        ==> after.slots_spec()[slot] == before.slots_spec()[slot]
}

/// Executes one logical batch action and proves its selected transition and
/// the exact frame for every nonselected slot.
///
/// # Errors
///
/// Returns the exact fail-closed batch-oracle rejection. Every rejection
/// leaves the complete batch unchanged.
pub fn m1_continuous_batch_action_theorem(
    batch: &mut ContinuousBatch,
    request: RequestId,
    action: ContinuousBatchAction,
) -> (result: Result<(), ContinuousBatchError>)
    requires old(batch).valid(),
    ensures m1_continuous_batch_action_refines(
        old(batch),
        final(batch),
        request,
        action,
        &result,
    ),
{
    let ghost entry = *batch;
    assert(entry == *old(batch));
    let result = apply_continuous_batch_step(batch, request, action);
    match result {
        Ok(()) => {
            assert(ferric_spec::continuous_batching::continuous_batch_step_refines(
                &entry,
                batch,
                request,
                action,
            ));
            assert forall|slot: int|
                0 <= slot
                    < ferric_spec::continuous_batching::M1_CONTINUOUS_BATCH_CAPACITY
                && slot != request.slot_spec()
                implies batch.slots_spec()[slot] == entry.slots_spec()[slot] by {
                ferric_spec::continuous_batching::successful_batch_step_preserves_other_request(
                    &entry,
                    batch,
                    request,
                    action,
                    slot,
                );
            }
        },
        Err(_) => {
            assert(*batch == entry);
        },
    }
    proof {
        reveal(m1_continuous_batch_action_refines);
    }
    result
}

/// Rejects an in-range stale generation and frames the complete logical batch.
///
/// # Errors
///
/// Always returns [`ContinuousBatchError::StaleGeneration`] under its proof
/// preconditions.
pub fn m1_stale_generation_rejection_theorem(
    batch: &mut ContinuousBatch,
    request: RequestId,
    action: ContinuousBatchAction,
) -> (result: Result<(), ContinuousBatchError>)
    requires
        old(batch).valid(),
        request.slot_spec()
            < ferric_spec::continuous_batching::M1_CONTINUOUS_BATCH_CAPACITY,
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
        ferric_spec::continuous_batching::stale_generation_is_expected_error(
            &entry,
            request,
            action,
        );
    }
    assert(result == Err(ContinuousBatchError::StaleGeneration));
    result
}

/// Rejects a second publication for one exact active epoch and frames the
/// complete logical batch.
///
/// # Errors
///
/// Always returns [`ContinuousBatchError::AlreadyPublished`] under its proof
/// preconditions.
pub fn m1_publish_once_rejection_theorem(
    batch: &mut ContinuousBatch,
    request: RequestId,
    epoch: ferric_spec::completion::CompletionEpoch,
    emitted_tokens: u8,
) -> (result: Result<(), ContinuousBatchError>)
    requires
        old(batch).valid(),
        request.slot_spec()
            < ferric_spec::continuous_batching::M1_CONTINUOUS_BATCH_CAPACITY,
        old(batch).slots_spec()[request.slot_spec() as int].generation_spec()
            == request.generation_spec(),
        old(batch).slots_spec()[request.slot_spec() as int].active_epoch_spec()
            == Some(epoch),
        ferric_spec::continuous_batching::publication_ready(
            old(batch).slots_spec()[request.slot_spec() as int],
        ),
        old(batch).slots_spec()[request.slot_spec() as int]
            .published_for_active_epoch_spec(),
    ensures
        result == Err(ContinuousBatchError::AlreadyPublished),
        *final(batch) == *old(batch),
{
    let ghost entry = *batch;
    assert(entry == *old(batch));
    let action = ContinuousBatchAction::Publish { epoch, emitted_tokens };
    let result = apply_continuous_batch_step(batch, request, action);
    proof {
        ferric_spec::continuous_batching::already_published_batch_is_expected_error(
            &entry,
            request,
            epoch,
            emitted_tokens,
        );
    }
    assert(result == Err(ContinuousBatchError::AlreadyPublished));
    result
}

} // verus!
