#![forbid(unsafe_code)]

//! M1 scheduler source-level composition theorems.
//!
//! The executable dispatch binder identifies the exact scheduler-issued
//! roster as the live prefix of caller-owned output, binds the returned epoch
//! to the scheduler's exact next submitted epoch, and proves every selected
//! request takes the legal `Ready -> InFlight` transition. The executable
//! completion binder proves fail-closed rejection of a reordered epoch while
//! returning its linear completion authority and preserving the whole engine
//! through an immutable borrow.
//!
//! These theorems do not refine physical queues, device execution, or KV
//! bytes; cover multiple queues; establish hardware or numerical behavior;
//! qualify performance; or close M1.

#[allow(unused_imports)]
use ferric_engine::{
    CompletionFailure, DispatchBatch, Engine, EngineError, ExactCompletion, SchedulerError,
};
#[allow(unused_imports)]
use ferric_spec::{
    completion::CompletionEpoch, scheduling::RequestState, RequestId, M1_MAX_ACTIVE_SEQUENCES,
};
#[allow(unused_imports)]
use vstd::prelude::*;

verus! {

/// The exact external roster issued by a successful scheduler dispatch.
///
/// The dispatch refinement binds every entry in this prefix to the internal
/// deterministic ready selection and frames the remainder of `output`.
pub open spec fn m1_scheduler_issued_roster(
    output: Seq<RequestId>,
    batch: &DispatchBatch,
) -> Seq<RequestId> {
    output.subrange(0, batch.member_count_spec() as int)
}

/// Public M1 consequences of one source-level engine dispatch.
pub open spec fn m1_scheduler_dispatch_refines<const C: usize>(
    before: &Engine<C>,
    after: &Engine<C>,
    before_output: Seq<RequestId>,
    output: Seq<RequestId>,
    result: &Result<Option<DispatchBatch>, EngineError>,
) -> bool {
    &&& after.well_formed()
    &&& after.dispatch_ready_refines(before, before_output, output, result)
    &&& match result {
        Ok(Some(batch)) => {
            let roster = m1_scheduler_issued_roster(output, batch);
            &&& batch.member_count_spec() > 0
            &&& batch.member_count_spec() <= before_output.len()
            &&& output.len() == before_output.len()
            &&& roster.len() == batch.member_count_spec()
            &&& batch.epoch_spec().value as int
                == before.submitted_epoch_spec().value as int + 1
            &&& after.submitted_epoch_spec() == batch.epoch_spec()
            &&& batch.epoch_spec().value > before.completed_epoch_spec().value
            &&& forall|offset: int| 0 <= offset < roster.len() ==> {
                let request = #[trigger] roster[offset];
                &&& request == output[offset]
                &&& before.state_spec(request) == Some(RequestState::Ready)
                &&& after.state_spec(request) == Some(RequestState::InFlight)
            }
            &&& forall|request: RequestId|
                request.slot_spec() < C
                && (forall|offset: int| 0 <= offset < roster.len() ==>
                    #[trigger] roster[offset].slot_spec() != request.slot_spec())
                ==> after.state_spec(request) == before.state_spec(request)
        }
        _ => true,
    }
}

/// Executes one M1-width deterministic scheduler dispatch and binds its exact
/// caller-visible roster, epoch, and selected lifecycle transitions.
///
/// # Errors
///
/// Returns the exact fail-closed engine or scheduler error carried by
/// [`Engine::dispatch_ready`].
pub fn m1_scheduler_dispatch_theorem<const C: usize>(
    engine: &mut Engine<C>,
    output: &mut [RequestId; M1_MAX_ACTIVE_SEQUENCES as usize],
) -> (result: Result<Option<DispatchBatch>, EngineError>)
    requires
        old(engine).well_formed(),
        C <= M1_MAX_ACTIVE_SEQUENCES,
    ensures m1_scheduler_dispatch_refines(
        old(engine),
        final(engine),
        old(output)@,
        final(output)@,
        &result,
    ),
{
    let ghost entry = *engine;
    let result = engine.dispatch_ready(output);
    proof {
        match &result {
            Ok(Some(batch)) => {
                assert(m1_scheduler_issued_roster(output@, batch).len()
                    == batch.member_count_spec());
                assert forall|offset: int|
                    0 <= offset < m1_scheduler_issued_roster(output@, batch).len()
                    implies {
                        let request = #[trigger]
                            m1_scheduler_issued_roster(output@, batch)[offset];
                        &&& request == output@[offset]
                        &&& entry.state_spec(request) == Some(RequestState::Ready)
                        &&& engine.state_spec(request) == Some(RequestState::InFlight)
                    } by {
                        reveal(m1_scheduler_issued_roster);
                    }
                assert forall|request: RequestId|
                    request.slot_spec() < C
                    && (forall|offset: int|
                        0 <= offset < m1_scheduler_issued_roster(output@, batch).len()
                        ==> #[trigger]
                            m1_scheduler_issued_roster(output@, batch)[offset].slot_spec()
                                != request.slot_spec())
                    implies engine.state_spec(request) == entry.state_spec(request) by {
                    assert forall|offset: int| 0 <= offset < batch.member_count_spec()
                        implies output@[offset].slot_spec() != request.slot_spec() by {
                        assert(0 <= offset
                            < m1_scheduler_issued_roster(output@, batch).len());
                        assert(m1_scheduler_issued_roster(output@, batch)[offset].slot_spec()
                            != request.slot_spec());
                        reveal(m1_scheduler_issued_roster);
                    }
                }
            }
            _ => {},
        }
        reveal(m1_scheduler_dispatch_refines);
    }
    result
}

/// Exact fail-closed result of a source-level reordered completion preflight.
pub open spec fn m1_reordered_completion_is_rejected<const C: usize>(
    engine: &Engine<C>,
    completion_epoch: CompletionEpoch,
    accepted_count: nat,
    failure: &CompletionFailure,
) -> bool {
    &&& engine.well_formed()
    &&& engine.completion_epoch_reordered(completion_epoch, accepted_count)
    &&& failure.error_spec() == EngineError::Scheduler(
        SchedulerError::CompletionNotExactNext,
    )
    &&& failure.returns_completion_at_spec(completion_epoch)
}

/// Rejects a skipped, replayed, or stale completion epoch before mutation,
/// returns its linear authority, and frames the complete engine state.
pub fn m1_reordered_completion_rejection_theorem<const C: usize>(
    engine: &mut Engine<C>,
    completion: ExactCompletion,
    accepted_tokens: &[u32],
) -> (failure: CompletionFailure)
    requires
        old(engine).well_formed(),
        old(engine).completion_epoch_reordered(
            completion.epoch_spec(),
            accepted_tokens@.len(),
        ),
    ensures
        m1_reordered_completion_is_rejected(
            final(engine),
            completion.epoch_spec(),
            accepted_tokens@.len(),
            &failure,
        ),
        *final(engine) == *old(engine),
{
    let ghost completion_epoch = completion.epoch_spec();
    let failure = engine.reject_reordered_completion(completion, accepted_tokens);
    proof {
        reveal(m1_reordered_completion_is_rejected);
    }
    failure
}

} // verus!
