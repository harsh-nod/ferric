//! Composed request lifecycle and KV ownership.

#[allow(unused_imports)]
use crate::cache::{KvDetachedRequest, KvError, KvPool, MAX_REQUEST_SLOTS};
use crate::epoch::ExactCompletion;
use crate::scheduler::{
    DispatchBatch, KvQuiescencePermit, M1ScheduledDispatchV1, Scheduler, SchedulerError,
};
use ferric_spec::completion::CompletionEpoch;
use ferric_spec::scheduling::RequestState;
use ferric_spec::{RequestId, M1_MAX_ACTIVE_SEQUENCES};
use vstd::prelude::*;

verus! {

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineError {
    Faulted,
    CompletionResultCount { expected: usize, actual: usize },
    RequestNotReady,
    Scheduler(SchedulerError),
    Kv(KvError),
    InvariantViolation,
}

/// A rejected completion returns the authority only when it was not consumed.
/// Once exact completion has advanced, an internal failure is fail-stop and
/// the authority is absent.
#[derive(Debug, PartialEq, Eq)]
pub struct CompletionFailure {
    error: EngineError,
    completion: Option<ExactCompletion>,
}

impl CompletionFailure {
    #[must_use]
    pub const fn error(&self) -> (error: EngineError)
        ensures error == self.error_spec(),
    {
        self.error
    }

    #[must_use]
    pub fn into_completion(self) -> (completion: Option<ExactCompletion>)
        ensures completion == self.completion_spec(),
    {
        self.completion
    }

    pub closed spec fn completion_spec(&self) -> Option<ExactCompletion> {
        self.completion
    }

    pub closed spec fn error_spec(&self) -> EngineError {
        self.error
    }

    pub closed spec fn returns_completion_at_spec(&self, epoch: CompletionEpoch) -> bool {
        match self.completion {
            Some(completion) => completion.epoch_spec() == epoch,
            None => false,
        }
    }

    pub closed spec fn consumed_completion_spec(&self) -> bool {
        self.completion.is_none()
    }

    fn returned(error: EngineError, completion: ExactCompletion) -> (failure: Self)
        ensures
            failure.error_spec() == error,
            failure.returns_completion_at_spec(completion.epoch_spec()),
    {
        Self {
            error,
            completion: Some(completion),
        }
    }

    fn consumed(error: EngineError) -> (failure: Self)
        ensures
            failure.error_spec() == error,
            failure.consumed_completion_spec(),
    {
        Self {
            error,
            completion: None,
        }
    }
}

/// Sole owner of request scheduling and KV metadata.
///
/// All storage is bounded at construction. The completion scratch vector has
/// exactly `C` elements and is never resized after construction.
pub struct Engine<const C: usize> {
    scheduler: Scheduler<C>,
    kv: KvPool,
    permits: Vec<Option<KvQuiescencePermit>>,
    faulted: bool,
}

/// Permanently faulted Engine retained after a capture phase abandons all
/// admitted request authority.
#[must_use = "the quarantined Engine remains the terminal scheduler/KV owner"]
pub struct M1CaptureQuarantinedEngineV1<const C: usize> {
    engine: Engine<C>,
}

impl<const C: usize> M1CaptureQuarantinedEngineV1<C> {
    #[must_use]
    pub fn is_faulted(&self) -> bool {
        self.engine.is_faulted()
    }
}

impl<const C: usize> Engine<C> {
    closed spec fn identity_agreement(&self) -> bool {
        &&& forall |slot: int| 0 <= slot < C ==> {
            &&& self.scheduler.slot_is_live_spec(slot)
                == self.kv.request_live_by_slot_spec(slot)
            &&& self.scheduler.slot_generation_spec(slot)
                == self.kv.request_generation_by_slot_spec(slot)
        }
        &&& forall |slot: int| C <= slot < MAX_REQUEST_SLOTS ==> {
            &&& !self.kv.request_live_by_slot_spec(slot)
            &&& self.kv.request_generation_by_slot_spec(slot) == 1
        }
    }

    pub closed spec fn well_formed(&self) -> bool {
        &&& self.scheduler.basic_invariant()
        &&& self.kv.well_formed()
        &&& 0 < C <= MAX_REQUEST_SLOTS
        &&& self.permits@.len() == C
        &&& (forall |index: int| 0 <= index < C ==>
            self.permits@[index].is_none())
        &&& self.identity_agreement()
    }

    pub closed spec fn faulted_spec(&self) -> bool {
        self.faulted
    }

    pub closed spec fn queue_rearm_quarantine_refines(&self, before: &Self) -> bool {
        &&& self.scheduler == before.scheduler
        &&& self.kv == before.kv
        &&& self.permits@ == before.permits@
    }

    pub closed spec fn live_count_spec(&self) -> usize {
        self.scheduler.live_count_spec()
    }

    pub closed spec fn completed_epoch_spec(&self) -> CompletionEpoch {
        self.scheduler.completed_epoch_spec()
    }

    pub closed spec fn submitted_epoch_spec(&self) -> CompletionEpoch {
        self.scheduler.submitted_epoch_spec()
    }

    pub closed spec fn state_spec(&self, request: RequestId) -> Option<RequestState> {
        self.scheduler.state_spec(request)
    }

    pub closed spec fn resident_tokens_spec(&self, request: RequestId) -> Option<u32> {
        self.kv.resident_tokens_spec(request)
    }

    pub closed spec fn committed_tokens_spec(&self, request: RequestId) -> Option<u32> {
        self.kv.committed_tokens_spec(request)
    }

    pub closed spec fn free_pages_spec(&self) -> u32 {
        self.kv.free_pages_spec()
    }

    pub closed spec fn pending_batch_member_count_spec(&self) -> usize {
        self.scheduler.pending_batch_member_count_spec()
    }

    pub closed spec fn pending_member_spec(&self, offset: usize) -> Option<RequestId> {
        self.scheduler.pending_member_spec(offset)
    }

    pub closed spec fn slot_is_live_spec(&self, slot: int) -> bool {
        self.scheduler.slot_is_live_spec(slot)
    }

    pub closed spec fn slot_generation_spec(&self, slot: int) -> u32 {
        self.scheduler.slot_generation_spec(slot)
    }

    /// A completion epoch is externally reordered relative to the exact next
    /// pending scheduler batch. The count guards exclude earlier preflight
    /// errors that have a different diagnostic.
    pub open spec fn completion_epoch_reordered(
        &self,
        completion_epoch: CompletionEpoch,
        accepted_count: nat,
    ) -> bool {
        &&& !self.faulted_spec()
        &&& self.pending_batch_member_count_spec() > 0
        &&& accepted_count == self.pending_batch_member_count_spec()
        &&& ferric_spec::completion::exact_next(
            self.completed_epoch_spec(),
            completion_epoch,
        ).is_err()
    }

    closed spec fn same_state(&self, before: &Self) -> bool {
        &&& self.scheduler.same_scalars(&before.scheduler)
        &&& self.kv.same_state(&before.kv)
        &&& self.permits@ == before.permits@
        &&& self.faulted == before.faulted
    }

    pub closed spec fn new_refines(
        page_count: u32,
        page_tokens: u32,
        max_context_tokens: u32,
        result: &Result<Self, EngineError>,
    ) -> bool {
        match result {
            Ok(engine) => {
                &&& 0 < C <= MAX_REQUEST_SLOTS
                &&& engine.scheduler.basic_invariant()
                &&& (forall|slot: int| 0 <= slot < C ==> {
                    &&& !engine.slot_is_live_spec(slot)
                    &&& engine.slot_generation_spec(slot) == 1
                })
                &&& KvPool::new_refines(
                    page_count,
                    page_tokens,
                    max_context_tokens,
                    &Ok(engine.kv),
                )
                &&& engine.well_formed()
                &&& !engine.faulted_spec()
            }
            Err(EngineError::Scheduler(error)) => {
                &&& *error == if C == 0 {
                    SchedulerError::ZeroCapacity
                } else {
                    SchedulerError::CapacityExceedsKvSlots
                }
                &&& (C == 0 || C > MAX_REQUEST_SLOTS)
            }
            Err(EngineError::Kv(error)) => {
                &&& 0 < C <= MAX_REQUEST_SLOTS
                &&& KvPool::new_refines(
                    page_count,
                    page_tokens,
                    max_context_tokens,
                    &Err(*error),
                )
            }
            Err(_) => false,
        }
    }

    pub closed spec fn admit_refines(
        &self,
        before: &Self,
        result: &Result<RequestId, EngineError>,
    ) -> bool {
        &&& self.permits@ == before.permits@
        &&& self.faulted == before.faulted
        &&& match result {
            Err(EngineError::Faulted) => {
                &&& before.faulted_spec()
                &&& self.scheduler.same_scalars(&before.scheduler)
                &&& self.kv.same_state(&before.kv)
            }
            Err(EngineError::Scheduler(error)) => {
                &&& !before.faulted_spec()
                &&& self.kv.same_state(&before.kv)
                &&& exists |scheduler_result: Result<RequestId, SchedulerError>|
                    #[trigger] self.scheduler.admit_refines(
                        &before.scheduler,
                        &scheduler_result,
                    ) && match (&scheduler_result, result) {
                        (Err(scheduler_error), Err(EngineError::Scheduler(engine_error))) => {
                            scheduler_error == engine_error
                        }
                        _ => false,
                    }
            }
            Ok(request) => {
                &&& !before.faulted_spec()
                &&& exists |scheduler_result: Result<RequestId, SchedulerError>|
                    #[trigger] self.scheduler.admit_refines(
                        &before.scheduler,
                        &scheduler_result,
                    ) && match (&scheduler_result, result) {
                        (Ok(scheduler_request), Ok(engine_request)) => {
                            scheduler_request == engine_request
                        }
                        _ => false,
                    }
                &&& exists |kv_result: Result<(), KvError>|
                    #[trigger] self.kv.create_refines(
                        &before.kv,
                        *request,
                        &kv_result,
                    ) && match (&kv_result, result) {
                        (Ok(()), Ok(_)) => true,
                        _ => false,
                    }
            }
            Err(_) => false,
        }
    }

    pub closed spec fn append_tentative_refines(
        &self,
        before: &Self,
        request: RequestId,
        token_count: u32,
        result: &Result<(), EngineError>,
    ) -> bool {
        &&& self.scheduler.same_scalars(&before.scheduler)
        &&& self.permits@ == before.permits@
        &&& self.faulted == before.faulted
        &&& match result {
            Err(EngineError::Faulted) => {
                before.faulted_spec() && self.kv.same_state(&before.kv)
            }
            Err(EngineError::RequestNotReady) => {
                &&& !before.faulted_spec()
                &&& before.state_spec(request) != Some(RequestState::Ready)
                &&& self.kv.same_state(&before.kv)
            }
            Ok(()) => {
                &&& !before.faulted_spec()
                &&& before.state_spec(request) == Some(RequestState::Ready)
                &&& exists |kv_result: Result<(), KvError>|
                    #[trigger] self.kv.append_refines(
                        &before.kv,
                        request,
                        token_count,
                        &kv_result,
                    ) && match (&kv_result, result) {
                        (Ok(()), Ok(())) => true,
                        _ => false,
                    }
            }
            Err(EngineError::Kv(error)) => {
                &&& !before.faulted_spec()
                &&& before.state_spec(request) == Some(RequestState::Ready)
                &&& exists |kv_result: Result<(), KvError>|
                    #[trigger] self.kv.append_refines(
                        &before.kv,
                        request,
                        token_count,
                        &kv_result,
                    ) && match (&kv_result, result) {
                        (Err(kv_error), Err(EngineError::Kv(engine_error))) => {
                            kv_error == engine_error
                        }
                        _ => false,
                    }
            }
            Err(_) => false,
        }
    }

    pub closed spec fn share_committed_prefix_refines(
        &self,
        before: &Self,
        source: RequestId,
        target: RequestId,
        token_count: u32,
        result: &Result<(), EngineError>,
    ) -> bool {
        &&& self.scheduler.same_scalars(&before.scheduler)
        &&& self.permits@ == before.permits@
        &&& self.faulted == before.faulted
        &&& match result {
            Err(EngineError::Faulted) => {
                before.faulted_spec() && self.kv.same_state(&before.kv)
            }
            Err(EngineError::RequestNotReady) => {
                &&& !before.faulted_spec()
                &&& (before.state_spec(source) != Some(RequestState::Ready)
                    || before.state_spec(target) != Some(RequestState::Ready))
                &&& self.kv.same_state(&before.kv)
            }
            Ok(()) => {
                &&& !before.faulted_spec()
                &&& before.state_spec(source) == Some(RequestState::Ready)
                &&& before.state_spec(target) == Some(RequestState::Ready)
                &&& exists |kv_result: Result<(), KvError>|
                    #[trigger] self.kv.share_refines(
                        &before.kv,
                        source,
                        target,
                        token_count,
                        &kv_result,
                    ) && match (&kv_result, result) {
                        (Ok(()), Ok(())) => true,
                        _ => false,
                    }
            }
            Err(EngineError::Kv(error)) => {
                &&& !before.faulted_spec()
                &&& before.state_spec(source) == Some(RequestState::Ready)
                &&& before.state_spec(target) == Some(RequestState::Ready)
                &&& exists |kv_result: Result<(), KvError>|
                    #[trigger] self.kv.share_refines(
                        &before.kv,
                        source,
                        target,
                        token_count,
                        &kv_result,
                    ) && match (&kv_result, result) {
                        (Err(kv_error), Err(EngineError::Kv(engine_error))) => {
                            kv_error == engine_error
                        }
                        _ => false,
                    }
            }
            Err(_) => false,
        }
    }

    pub closed spec fn validate_read_refines(
        &self,
        request: RequestId,
        logical_offset: u32,
        span: u32,
        result: &Result<(), EngineError>,
    ) -> bool {
        match result {
            Err(EngineError::Faulted) => self.faulted_spec(),
            Ok(()) => {
                &&& !self.faulted_spec()
                &&& exists |kv_result: Result<(), KvError>|
                    #[trigger] self.kv.read_refines(
                        request,
                        logical_offset,
                        span,
                        &kv_result,
                    ) && match (&kv_result, result) {
                        (Ok(()), Ok(())) => true,
                        _ => false,
                    }
            }
            Err(EngineError::Kv(error)) => {
                &&& !self.faulted_spec()
                &&& exists |kv_result: Result<(), KvError>|
                    #[trigger] self.kv.read_refines(
                        request,
                        logical_offset,
                        span,
                        &kv_result,
                    ) && match (&kv_result, result) {
                        (Err(kv_error), Err(EngineError::Kv(engine_error))) => {
                            kv_error == engine_error
                        }
                        _ => false,
                    }
            }
            Err(_) => false,
        }
    }

    pub closed spec fn retire_refines(
        &self,
        before: &Self,
        request: RequestId,
        result: &Result<(), EngineError>,
    ) -> bool {
        &&& self.kv.same_state(&before.kv)
        &&& self.permits@ == before.permits@
        &&& self.faulted == before.faulted
        &&& match result {
            Err(EngineError::Faulted) => {
                &&& before.faulted_spec()
                &&& self.scheduler.same_scalars(&before.scheduler)
            }
            _ => {
                &&& !before.faulted_spec()
                &&& exists |scheduler_result: Result<(), SchedulerError>|
                    self.scheduler.retire_refines(
                        &before.scheduler,
                        request,
                        &scheduler_result,
                    )
                    && match (&scheduler_result, result) {
                        (Ok(()), Ok(())) => true,
                        (Err(scheduler_error), Err(EngineError::Scheduler(engine_error))) => {
                            scheduler_error == engine_error
                        }
                        _ => false,
                    }
            }
        }
    }

    closed spec fn reclaim_success_step(
        &self,
        before: &Self,
        permit: KvQuiescencePermit,
        detached: KvDetachedRequest,
        after_take: Scheduler<C>,
        after_release: KvPool,
        result_request: RequestId,
    ) -> bool {
        let release_request = permit.request_spec();
        &&& after_take.retiring_permit_refines(
            &before.scheduler,
            &Ok(Some(permit)),
        )
        &&& before.kv.release_authority_enabled(release_request, &permit)
        &&& before.kv.release_authority_decision(release_request, &permit) == Ok(())
        &&& detached.request_spec() == release_request
        &&& detached.origin_spec() == permit.origin_spec()
        &&& KvPool::same_request_id(permit.request_spec(), release_request)
        &&& release_request.generation_spec() < u32::MAX
        &&& !after_release.request_live_by_slot_spec(release_request.slot_spec() as int)
        &&& after_release.resident_tokens_spec(release_request).is_none()
        &&& after_release.committed_tokens_spec(release_request).is_none()
        &&& after_release.request_generation_by_slot_spec(
            release_request.slot_spec() as int,
        ) == before.kv.request_generation_by_slot_spec(
            release_request.slot_spec() as int,
        ) + 1
        &&& after_release.identity_frame_except(
            &before.kv,
            release_request.slot_spec() as int,
        )
        &&& after_release.request_frame_except(
            &before.kv,
            release_request.slot_spec() as int,
        )
        &&& after_release.release_page_frame(&before.kv, release_request.slot_spec())
        &&& self.scheduler.detached_refines(
            &after_take,
            &detached,
            &Ok(result_request),
        )
        &&& self.kv.same_state(&after_release)
    }

    closed spec fn reclaim_release_error_step(
        &self,
        before: &Self,
        permit: KvQuiescencePermit,
        after_take: Scheduler<C>,
        error: KvError,
    ) -> bool {
        &&& after_take.retiring_permit_refines(
            &before.scheduler,
            &Ok(Some(permit)),
        )
        &&& self.scheduler.same_scalars(&after_take)
        &&& !before.kv.release_authority_enabled(permit.request_spec(), &permit)
        &&& before.kv.release_authority_decision(permit.request_spec(), &permit)
            == Err(error)
    }

    closed spec fn reclaim_success_refines(
        &self,
        before: &Self,
        result_request: RequestId,
    ) -> bool {
        &&& !before.faulted_spec()
        &&& self.permits@ == before.permits@
        &&& !self.faulted_spec()
        &&& exists |permit: KvQuiescencePermit,
            detached: KvDetachedRequest,
            after_take: Scheduler<C>,
            after_release: KvPool|
            #[trigger] self.reclaim_success_step(
                before,
                permit,
                detached,
                after_take,
                after_release,
                result_request,
            )
    }

    pub closed spec fn reclaim_one_refines(
        &self,
        before: &Self,
        result: &Result<Option<RequestId>, EngineError>,
    ) -> bool {
        match result {
            Err(EngineError::Faulted) => {
                before.faulted_spec() && self.same_state(before)
            }
            Ok(None) => {
                &&& !before.faulted_spec()
                &&& self.scheduler.retiring_permit_refines(
                    &before.scheduler,
                    &Ok(None),
                )
                &&& self.kv.same_state(&before.kv)
                &&& self.permits@ == before.permits@
                &&& self.faulted == before.faulted
            }
            Ok(Some(request)) => self.reclaim_success_refines(before, *request),
            Err(EngineError::Scheduler(error)) => {
                &&& !before.faulted_spec()
                &&& self.kv.same_state(&before.kv)
                &&& self.permits@ == before.permits@
                &&& self.faulted == before.faulted
                &&& exists |scheduler_result: Result<
                    Option<KvQuiescencePermit>,
                    SchedulerError,
                >| #[trigger] self.scheduler.retiring_permit_refines(
                    &before.scheduler,
                    &scheduler_result,
                ) && match (&scheduler_result, result) {
                    (Err(scheduler_error), Err(EngineError::Scheduler(engine_error))) => {
                        scheduler_error == engine_error
                    }
                    _ => false,
                }
            }
            Err(EngineError::Kv(error)) => {
                &&& !before.faulted_spec()
                &&& self.faulted_spec()
                &&& self.permits@ == before.permits@
                &&& self.kv.same_state(&before.kv)
                &&& exists |permit: KvQuiescencePermit, after_take: Scheduler<C>|
                    #[trigger] self.reclaim_release_error_step(
                        before,
                        permit,
                        after_take,
                        *error,
                    )
            }
            Err(_) => false,
        }
    }

    pub closed spec fn dispatch_ready_refines(
        &self,
        before: &Self,
        before_output: Seq<RequestId>,
        output: Seq<RequestId>,
        result: &Result<Option<DispatchBatch>, EngineError>,
    ) -> bool {
        &&& self.kv.same_state(&before.kv)
        &&& self.permits@ == before.permits@
        &&& self.faulted == before.faulted
        &&& match result {
            Err(EngineError::Faulted) => {
                &&& before.faulted_spec()
                &&& self.scheduler.same_scalars(&before.scheduler)
                &&& output == before_output
            }
            Ok(batch) => {
                &&& !before.faulted_spec()
                &&& self.scheduler.dispatch_refines(
                    &before.scheduler,
                    before_output,
                    output,
                    &Ok(*batch),
                )
            }
            Err(EngineError::Scheduler(error)) => {
                &&& !before.faulted_spec()
                &&& self.scheduler.dispatch_refines(
                    &before.scheduler,
                    before_output,
                    output,
                    &Err(*error),
                )
            }
            Err(_) => false,
        }
    }

    closed spec fn pending_prefix_contains_slot(
        before: &Self,
        prefix: int,
        slot: int,
    ) -> bool {
        exists |offset: int| 0 <= offset < prefix && match
            #[trigger] before.pending_member_spec(offset as usize)
        {
            Some(request) => request.slot_spec() as int == slot,
            None => false,
        }
    }

    closed spec fn pending_members_distinct(&self, count: int) -> bool {
        forall |left: int, right: int| 0 <= left < right < count ==> match (
            #[trigger] self.pending_member_spec(left as usize),
            #[trigger] self.pending_member_spec(right as usize),
        ) {
            (Some(left_request), Some(right_request)) => {
                left_request.slot_spec() != right_request.slot_spec()
            }
            _ => false,
        }
    }

    closed spec fn pending_members_bounded(&self, count: int) -> bool {
        forall |offset: int| 0 <= offset < count ==> match
            #[trigger] self.pending_member_spec(offset as usize)
        {
            Some(request) => request.slot_spec() < C,
            None => false,
        }
    }

    closed spec fn observations_frame_except(&self, before: &Self, changed: int) -> bool {
        &&& (forall |request: RequestId| request.slot_spec() < C
            && request.slot_spec() as int != changed ==> {
                &&& self.state_spec(request) == before.state_spec(request)
                &&& self.resident_tokens_spec(request)
                    == before.resident_tokens_spec(request)
                &&& self.committed_tokens_spec(request)
                    == before.committed_tokens_spec(request)
            })
        &&& (forall |slot: int| 0 <= slot < C && slot != changed ==> {
            &&& self.slot_is_live_spec(slot) == before.slot_is_live_spec(slot)
            &&& self.slot_generation_spec(slot) == before.slot_generation_spec(slot)
        })
    }

    closed spec fn member_completion_refines(
        &self,
        before: &Self,
        accepted_tokens: Seq<u32>,
        offset: int,
    ) -> bool {
        match before.pending_member_spec(offset as usize) {
            None => false,
            Some(request) => match before.state_spec(request) {
                Some(RequestState::InFlight) => {
                    &&& self.state_spec(request) == Some(RequestState::Ready)
                    &&& self.resident_tokens_spec(request)
                        == self.committed_tokens_spec(request)
                    &&& match (
                        before.committed_tokens_spec(request),
                        self.committed_tokens_spec(request),
                    ) {
                        (Some(before_committed), Some(committed)) => {
                            committed as int == before_committed as int
                                + accepted_tokens[offset] as int
                        }
                        _ => false,
                    }
                    &&& self.slot_is_live_spec(request.slot_spec() as int)
                    &&& self.slot_generation_spec(request.slot_spec() as int)
                        == before.slot_generation_spec(request.slot_spec() as int)
                }
                Some(RequestState::Retiring) => {
                    &&& self.state_spec(request).is_none()
                    &&& self.resident_tokens_spec(request).is_none()
                    &&& self.committed_tokens_spec(request).is_none()
                    &&& !self.slot_is_live_spec(request.slot_spec() as int)
                    &&& self.slot_generation_spec(request.slot_spec() as int) as int
                        == before.slot_generation_spec(request.slot_spec() as int) as int + 1
                }
                _ => false,
            },
        }
    }

    closed spec fn completion_prefix_refines(
        &self,
        before: &Self,
        accepted_tokens: Seq<u32>,
        prefix: int,
    ) -> bool {
        &&& 0 <= prefix <= before.pending_batch_member_count_spec()
        &&& accepted_tokens.len() == before.pending_batch_member_count_spec()
        &&& (forall |offset: int| 0 <= offset < prefix ==>
            self.member_completion_refines(before, accepted_tokens, offset))
        &&& (forall |offset: int|
            prefix <= offset < before.pending_batch_member_count_spec() ==> match
                #[trigger] before.pending_member_spec(offset as usize)
            {
                Some(request) => {
                    &&& self.state_spec(request) == before.state_spec(request)
                    &&& self.resident_tokens_spec(request)
                        == before.resident_tokens_spec(request)
                    &&& self.committed_tokens_spec(request)
                        == before.committed_tokens_spec(request)
                    &&& self.slot_is_live_spec(request.slot_spec() as int)
                        == before.slot_is_live_spec(request.slot_spec() as int)
                    &&& self.slot_generation_spec(request.slot_spec() as int)
                        == before.slot_generation_spec(request.slot_spec() as int)
                }
                None => false,
            })
    }

    proof fn advance_completion_prefix(
        &self,
        before_step: &Self,
        entry: &Self,
        accepted_tokens: Seq<u32>,
        index: int,
        request: RequestId,
    )
        requires
            before_step.completion_prefix_refines(entry, accepted_tokens, index),
            self.observations_frame_except(
                before_step,
                request.slot_spec() as int,
            ),
            entry.pending_member_spec(index as usize) == Some(request),
            entry.pending_members_distinct(entry.pending_batch_member_count_spec() as int),
            entry.pending_members_bounded(entry.pending_batch_member_count_spec() as int),
            self.member_completion_refines(entry, accepted_tokens, index),
            0 <= index < entry.pending_batch_member_count_spec(),
        ensures
            self.completion_prefix_refines(entry, accepted_tokens, index + 1),
    {
        reveal(Engine::completion_prefix_refines);
        reveal(Engine::pending_members_distinct);
        reveal(Engine::pending_members_bounded);
        reveal(Engine::observations_frame_except);
        reveal(Engine::member_completion_refines);
        assert forall |offset: int| 0 <= offset < index + 1 implies
            self.member_completion_refines(entry, accepted_tokens, offset) by {
            if offset < index {
                assert(before_step.member_completion_refines(
                    entry,
                    accepted_tokens,
                    offset,
                ));
                assert(entry.pending_member_spec(offset as usize).is_some());
                let other = entry.pending_member_spec(offset as usize).unwrap();
                assert(other.slot_spec() < C);
                assert(other.slot_spec() != request.slot_spec());
                assert(self.state_spec(other) == before_step.state_spec(other));
                assert(self.resident_tokens_spec(other)
                    == before_step.resident_tokens_spec(other));
                assert(self.committed_tokens_spec(other)
                    == before_step.committed_tokens_spec(other));
                assert(self.slot_is_live_spec(other.slot_spec() as int)
                    == before_step.slot_is_live_spec(other.slot_spec() as int));
                assert(self.slot_generation_spec(other.slot_spec() as int)
                    == before_step.slot_generation_spec(other.slot_spec() as int));
            } else {
                assert(offset == index);
            }
        }
        assert forall |offset: int|
            index + 1 <= offset < entry.pending_batch_member_count_spec() implies match
                #[trigger] entry.pending_member_spec(offset as usize)
            {
                Some(other) => {
                    &&& self.state_spec(other) == entry.state_spec(other)
                    &&& self.resident_tokens_spec(other)
                        == entry.resident_tokens_spec(other)
                    &&& self.committed_tokens_spec(other)
                        == entry.committed_tokens_spec(other)
                    &&& self.slot_is_live_spec(other.slot_spec() as int)
                        == entry.slot_is_live_spec(other.slot_spec() as int)
                    &&& self.slot_generation_spec(other.slot_spec() as int)
                        == entry.slot_generation_spec(other.slot_spec() as int)
                }
                None => false,
            } by {
            assert(entry.pending_member_spec(offset as usize).is_some());
            let other = entry.pending_member_spec(offset as usize).unwrap();
            assert(other.slot_spec() < C);
            assert(other.slot_spec() != request.slot_spec());
            assert(self.state_spec(other) == before_step.state_spec(other));
            assert(self.resident_tokens_spec(other)
                == before_step.resident_tokens_spec(other));
            assert(self.committed_tokens_spec(other)
                == before_step.committed_tokens_spec(other));
            assert(self.slot_is_live_spec(other.slot_spec() as int)
                == before_step.slot_is_live_spec(other.slot_spec() as int));
            assert(self.slot_generation_spec(other.slot_spec() as int)
                == before_step.slot_generation_spec(other.slot_spec() as int));
        }
    }

    pub closed spec fn completion_refines(
        &self,
        before: &Self,
        completion_epoch: CompletionEpoch,
        accepted_tokens: Seq<u32>,
        result: &Result<usize, CompletionFailure>,
    ) -> bool {
        match result {
            Ok(completed) => {
                &&& !before.faulted_spec()
                &&& *completed == before.pending_batch_member_count_spec()
                &&& accepted_tokens.len() == *completed
                &&& self.completed_epoch_spec() == completion_epoch
                &&& self.completion_prefix_refines(
                    before,
                    accepted_tokens,
                    *completed as int,
                )
                &&& !self.faulted_spec()
            }
            Err(failure) => {
                match failure.completion {
                    Some(returned) => {
                        &&& returned.epoch_spec() == completion_epoch
                        &&& self.same_state(before)
                    }
                    None => {
                        &&& !before.faulted_spec()
                        &&& self.faulted_spec()
                    }
                }
            }
        }
    }

    pub(crate) proof fn apply_completion_observations(
        &self,
        before: &Self,
        completion_epoch: CompletionEpoch,
        accepted_tokens: Seq<u32>,
        result: &Result<usize, CompletionFailure>,
    )
        requires self.completion_refines(
            before,
            completion_epoch,
            accepted_tokens,
            result,
        ),
        ensures
            match result {
                Ok(completed) => {
                    &&& *completed == before.pending_batch_member_count_spec()
                    &&& self.completed_epoch_spec() == completion_epoch
                }
                Err(failure) => match failure.completion_spec() {
                    Some(returned) => returned.epoch_spec() == completion_epoch,
                    None => self.faulted_spec(),
                },
            },
    {
        reveal(Engine::completion_refines);
        reveal(CompletionFailure::completion_spec);
    }

    /// Constructs all request, ring, page, and completion-scratch storage.
    ///
    /// # Errors
    ///
    /// Returns the scheduler or KV constructor error when the requested bounds
    /// cannot be represented by the fixed-capacity engine.
    pub fn new(
        page_count: u32,
        page_tokens: u32,
        max_context_tokens: u32,
    ) -> (result: Result<Self, EngineError>)
        ensures
            match result {
                Ok(engine) => engine.well_formed() && !engine.faulted_spec(),
                Err(_) => true,
            },
            Self::new_refines(page_count, page_tokens, max_context_tokens, &result),
    {
        let scheduler = match Scheduler::<C>::new() {
            Ok(scheduler) => scheduler,
            Err(error) => return Err(EngineError::Scheduler(error)),
        };
        let kv = match KvPool::new(page_count, page_tokens, max_context_tokens) {
            Ok(kv) => kv,
            Err(error) => return Err(EngineError::Kv(error)),
        };
        let mut permits: Vec<Option<KvQuiescencePermit>> = Vec::with_capacity(C);
        let mut index = 0;
        while index < C
            invariant
                index <= C,
                permits@.len() == index,
                forall |position: int| 0 <= position < index ==>
                    permits@[position].is_none(),
            decreases C - index,
        {
            permits.push(None);
            index += 1;
        }
        let engine = Self {
            scheduler,
            kv,
            permits,
            faulted: false,
        };
        assert forall |slot: int| 0 <= slot < C implies {
            &&& engine.scheduler.slot_is_live_spec(slot)
                == engine.kv.request_live_by_slot_spec(slot)
            &&& engine.scheduler.slot_generation_spec(slot)
                == engine.kv.request_generation_by_slot_spec(slot)
        } by {
        }
        assert forall |slot: int| C <= slot < MAX_REQUEST_SLOTS implies {
            &&& !engine.kv.request_live_by_slot_spec(slot)
            &&& engine.kv.request_generation_by_slot_spec(slot) == 1
        } by {
        }
        assert(engine.scheduler.basic_invariant());
        assert(engine.kv.well_formed());
        assert(0 < C <= MAX_REQUEST_SLOTS);
        assert(engine.permits@.len() == C);
        assert forall |position: int| 0 <= position < C implies
            engine.permits@[position].is_none() by {
        }
        reveal(Engine::well_formed);
        reveal(Engine::faulted_spec);
        assert(engine.well_formed());
        Ok(engine)
    }

    #[must_use]
    pub const fn is_faulted(&self) -> (faulted: bool)
        ensures faulted == self.faulted_spec(),
    {
        self.faulted
    }

    /// Permanently quarantines an in-flight M1 queue rearm after Ferric retains
    /// every available queue, cache, allocation, and scheduler owner.
    pub(crate) fn quarantine_m1_queue_rearm_failure(&mut self)
        requires old(self).well_formed(),
        ensures
            final(self).well_formed(),
            final(self).faulted_spec(),
            final(self).queue_rearm_quarantine_refines(old(self)),
    {
        reveal(Engine::well_formed);
        reveal(Engine::faulted_spec);
        reveal(Engine::queue_rearm_quarantine_refines);
        self.faulted = true;
    }

    /// Consumes the Engine into terminal capture-quarantine custody.
    #[must_use = "the quarantined Engine remains the terminal scheduler/KV owner"]
    pub fn into_m1_capture_quarantine(self) -> M1CaptureQuarantinedEngineV1<C>
        requires self.well_formed(),
    {
        let mut engine = self;
        engine.quarantine_m1_queue_rearm_failure();
        M1CaptureQuarantinedEngineV1 { engine }
    }

    #[must_use]
    pub const fn capacity(&self) -> (capacity: usize)
        ensures capacity == C,
    {
        self.scheduler.capacity()
    }

    #[must_use]
    pub const fn live_count(&self) -> (count: usize)
        ensures count == self.live_count_spec(),
    {
        self.scheduler.live_count()
    }

    #[must_use]
    pub const fn completed_epoch(&self) -> (epoch: CompletionEpoch)
        ensures epoch == self.completed_epoch_spec(),
    {
        self.scheduler.completed_epoch()
    }

    pub(crate) fn pending_batch_member_count(&self) -> (count: usize)
        requires self.well_formed(),
        ensures count == self.pending_batch_member_count_spec(),
    {
        self.scheduler.pending_batch_member_count()
    }

    pub(crate) fn pending_member(&self, offset: usize) -> (request: Option<RequestId>)
        requires self.well_formed(),
        ensures request == self.pending_member_spec(offset),
    {
        self.scheduler.pending_member(offset)
    }

    /// Performs every externally rejectable exact-completion check without
    /// consuming completion authority or mutating scheduler/KV state.
    pub(crate) fn preflight_complete_exact(
        &self,
        completion: &ExactCompletion,
        accepted_tokens: &[u32],
    ) -> (result: Result<(), EngineError>)
        requires self.well_formed(),
        ensures
            self.completion_epoch_reordered(
                completion.epoch_spec(),
                accepted_tokens@.len(),
            ) ==> result == Err(EngineError::Scheduler(
                SchedulerError::CompletionNotExactNext,
            )),
    {
        proof {
            reveal(Engine::completion_epoch_reordered);
        }
        if self.faulted {
            return Err(EngineError::Faulted);
        }
        let member_count = self.scheduler.pending_batch_member_count();
        if accepted_tokens.len() != member_count {
            return Err(EngineError::CompletionResultCount {
                expected: member_count,
                actual: accepted_tokens.len(),
            });
        }
        if member_count == 0 {
            return Err(EngineError::Scheduler(SchedulerError::NoPendingBatch));
        }
        let completed_epoch = self.scheduler.completed_epoch();
        assert(completed_epoch == self.completed_epoch_spec()) by {
            reveal(Engine::completed_epoch_spec);
        }
        let observed_epoch = completion.epoch();
        match ferric_spec::completion::check_exact_next(completed_epoch, observed_epoch) {
            Err(_) => {
                return Err(EngineError::Scheduler(
                    SchedulerError::CompletionNotExactNext,
                ));
            }
            Ok(_) => {}
        }
        assert(observed_epoch == completion.epoch_spec());
        assert(!self.completion_epoch_reordered(
            completion.epoch_spec(),
            accepted_tokens@.len(),
        ));

        let mut index = 0;
        while index < member_count
            invariant
                self.well_formed(),
                !self.completion_epoch_reordered(
                    completion.epoch_spec(),
                    accepted_tokens@.len(),
                ),
                index <= member_count,
                member_count <= C,
                accepted_tokens@.len() == member_count,
            decreases member_count - index,
        {
            let request = match self.scheduler.pending_member(index) {
                Some(request) => request,
                None => return Err(EngineError::InvariantViolation),
            };
            match self.scheduler.state(request) {
                Some(RequestState::InFlight) => {
                    let resident = match self.kv.resident_tokens(request) {
                        Some(tokens) => tokens,
                        None => return Err(EngineError::InvariantViolation),
                    };
                    let committed = match self.kv.committed_tokens(request) {
                        Some(tokens) => tokens,
                        None => return Err(EngineError::InvariantViolation),
                    };
                    let tentative = match resident.checked_sub(committed) {
                        Some(tokens) => tokens,
                        None => return Err(EngineError::InvariantViolation),
                    };
                    if accepted_tokens[index] > tentative {
                        return Err(EngineError::Kv(KvError::CommitExceedsResident));
                    }
                }
                Some(RequestState::Retiring) => {
                    if !self.scheduler.pending_retirement_ready(
                        index,
                        request,
                        completion.epoch(),
                    ) {
                        if request.generation() == u32::MAX {
                            return Err(EngineError::Scheduler(
                                SchedulerError::GenerationExhausted,
                            ));
                        }
                        return Err(EngineError::InvariantViolation);
                    }
                }
                _ => return Err(EngineError::InvariantViolation),
            }
            index += 1;
        }
        Ok(())
    }

    /// Rejects a non-next exact completion during the immutable preflight and
    /// returns its linear completion authority unchanged.
    ///
    /// This is a proof-bearing view of the same order check used by
    /// [`Self::complete_exact`]. The accepted-count and pending-batch guards in
    /// `completion_epoch_reordered` exclude earlier diagnostics.
    #[must_use]
    pub fn reject_reordered_completion(
        &self,
        completion: ExactCompletion,
        accepted_tokens: &[u32],
    ) -> (failure: CompletionFailure)
        requires
            self.well_formed(),
            self.completion_epoch_reordered(
                completion.epoch_spec(),
                accepted_tokens@.len(),
            ),
        ensures
            failure.error_spec() == EngineError::Scheduler(
                SchedulerError::CompletionNotExactNext,
            ),
            failure.returns_completion_at_spec(completion.epoch_spec()),
    {
        let result = self.preflight_complete_exact(&completion, accepted_tokens);
        match result {
            Err(error) => {
                assert(error == EngineError::Scheduler(
                    SchedulerError::CompletionNotExactNext,
                ));
                CompletionFailure::returned(error, completion)
            }
            Ok(()) => {
                assert(false);
                CompletionFailure::returned(
                    EngineError::Scheduler(SchedulerError::CompletionNotExactNext),
                    completion,
                )
            }
        }
    }

    /// Admits one request generation into the scheduler and KV pool.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Faulted`] after fail-stop, or the exact scheduler
    /// or KV admission error.
    pub fn admit(&mut self) -> (result: Result<RequestId, EngineError>)
        requires old(self).well_formed(),
        ensures
            final(self).well_formed(),
            final(self).admit_refines(old(self), &result),
    {
        let ghost entry = *self;
        reveal(Engine::well_formed);
        reveal(Engine::admit_refines);
        reveal(Engine::faulted_spec);
        proof {
            self.scheduler.same_scalars_reflexive();
            self.kv.same_state_reflexive();
        }
        self.require_healthy()?;
        let ghost entry_scheduler = self.scheduler;
        let ghost entry_kv = self.kv;
        let admit_result = self.scheduler.admit();
        assert(self.scheduler.admit_refines(&entry_scheduler, &admit_result));
        let request = match &admit_result {
            Ok(request) => {
                assert(exists |scheduler_result: Result<RequestId, SchedulerError>|
                    #[trigger] self.scheduler.admit_refines(
                        &entry.scheduler,
                        &scheduler_result,
                    ) && match (
                        &scheduler_result,
                        &Ok::<RequestId, EngineError>(*request),
                    ) {
                        (Ok(scheduler_request), Ok(engine_request)) => {
                            scheduler_request == engine_request
                        }
                        _ => false,
                    }) by {
                }
                *request
            }
            Err(error) => {
                assert(self.kv.same_state(&entry.kv));
                assert(exists |scheduler_result: Result<RequestId, SchedulerError>|
                    #[trigger] self.scheduler.admit_refines(
                        &entry.scheduler,
                        &scheduler_result,
                    ) && match (
                        &scheduler_result,
                        &Err::<RequestId, EngineError>(EngineError::Scheduler(*error)),
                    ) {
                        (Err(scheduler_error), Err(EngineError::Scheduler(engine_error))) => {
                            scheduler_error == engine_error
                        }
                        _ => false,
                    }) by {
                }
                assert(self.admit_refines(
                    &entry,
                    &Err(EngineError::Scheduler(*error)),
                ));
                return Err(EngineError::Scheduler(*error));
            }
        };
        assert(self.kv.create_enabled(request)) by {
            reveal(KvPool::create_enabled);
            self.scheduler.apply_admitted_request_identity(
                &entry_scheduler,
                request,
            );
            assert(request.slot_spec() < C);
            assert(!entry_scheduler.slot_is_live_spec(request.slot_spec() as int));
            assert(entry_scheduler.slot_generation_spec(request.slot_spec() as int)
                == request.generation_spec());
            assert(entry_scheduler.slot_is_live_spec(request.slot_spec() as int)
                == entry_kv.request_live_by_slot_spec(request.slot_spec() as int));
            assert(entry_scheduler.slot_generation_spec(request.slot_spec() as int)
                == entry_kv.request_generation_by_slot_spec(request.slot_spec() as int));
        }
        let create_result = self.kv.create_request(request);
        assert(self.kv.create_refines(&entry_kv, request, &create_result));
        match &create_result {
            Ok(()) => {
                assert forall |slot: int| 0 <= slot < C implies {
                    &&& self.scheduler.slot_is_live_spec(slot)
                        == self.kv.request_live_by_slot_spec(slot)
                    &&& self.scheduler.slot_generation_spec(slot)
                        == self.kv.request_generation_by_slot_spec(slot)
                } by {
                    if slot == request.slot_spec() {
                        assert(self.scheduler.slot_is_live_spec(slot));
                        assert(self.scheduler.slot_generation_spec(slot)
                            == request.generation_spec());
                        assert(self.kv.request_live_by_slot_spec(slot));
                        assert(self.kv.request_generation_by_slot_spec(slot)
                            == request.generation_spec());
                        assert(entry_scheduler.slot_is_live_spec(slot)
                            == entry_kv.request_live_by_slot_spec(slot));
                        assert(entry_scheduler.slot_generation_spec(slot)
                            == entry_kv.request_generation_by_slot_spec(slot));
                    } else {
                        assert(self.scheduler.slot_is_live_spec(slot)
                            == entry_scheduler.slot_is_live_spec(slot));
                        assert(self.scheduler.slot_generation_spec(slot)
                            == entry_scheduler.slot_generation_spec(slot));
                        assert(self.kv.request_live_by_slot_spec(slot)
                            == entry_kv.request_live_by_slot_spec(slot));
                        assert(self.kv.request_generation_by_slot_spec(slot)
                            == entry_kv.request_generation_by_slot_spec(slot));
                    }
                }
                assert forall |slot: int| C <= slot < MAX_REQUEST_SLOTS implies {
                    &&& !self.kv.request_live_by_slot_spec(slot)
                    &&& self.kv.request_generation_by_slot_spec(slot) == 1
                } by {
                    self.scheduler.apply_admitted_request_identity(
                        &entry_scheduler,
                        request,
                    );
                    assert((request.slot_spec() as int) < C);
                    assert(slot != request.slot_spec() as int);
                    self.kv.apply_identity_frame_except(
                        &entry_kv,
                        request.slot_spec() as int,
                        slot,
                    );
                }
                assert(exists |kv_result: Result<(), KvError>|
                    #[trigger] self.kv.create_refines(
                        &entry.kv,
                        request,
                        &kv_result,
                    ) && match (&kv_result, &Ok::<RequestId, EngineError>(request)) {
                        (Ok(()), Ok(_)) => true,
                        _ => false,
                    }) by {
                }
                assert(self.admit_refines(&entry, &Ok(request)));
                Ok(request)
            }
            Err(error) => {
                assert(false);
                self.faulted = true;
                Err(EngineError::Kv(*error))
            }
        }
    }

    /// Extends a live request with tentative logical KV tokens.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Faulted`] after fail-stop, or the exact KV
    /// capacity, identity, or context-bound error.
    pub fn append_tentative(
        &mut self,
        request: RequestId,
        token_count: u32,
    ) -> (result: Result<(), EngineError>)
        requires old(self).well_formed(),
        ensures
            final(self).well_formed(),
            final(self).append_tentative_refines(
                old(self),
                request,
                token_count,
                &result,
            ),
    {
        let ghost entry = *self;
        reveal(Engine::well_formed);
        reveal(Engine::append_tentative_refines);
        reveal(Engine::faulted_spec);
        reveal(Engine::state_spec);
        proof {
            self.scheduler.same_scalars_reflexive();
            self.kv.same_state_reflexive();
        }
        self.require_healthy()?;
        let request_state = self.scheduler.state(request);
        assert(request_state == self.scheduler.state_spec(request));
        assert(self.scheduler == entry.scheduler);
        assert(entry.state_spec(request) == request_state) by {
            reveal(Engine::state_spec);
        }
        match request_state {
            Some(RequestState::Ready) => {}
            Some(RequestState::Vacant | RequestState::InFlight | RequestState::Retiring)
            | None => {
                assert(self.scheduler.same_scalars(&entry.scheduler)) by {
                    assert(self.scheduler == entry.scheduler);
                    entry.scheduler.same_scalars_reflexive();
                }
                assert(self.kv.same_state(&entry.kv)) by {
                    assert(self.kv == entry.kv);
                    entry.kv.same_state_reflexive();
                }
                assert(entry.state_spec(request) != Some(RequestState::Ready));
                assert(self.permits@ == entry.permits@);
                assert(self.faulted == entry.faulted);
                assert(self.append_tentative_refines(
                    &entry,
                    request,
                    token_count,
                    &Err(EngineError::RequestNotReady),
                ));
                return Err(EngineError::RequestNotReady);
            }
        }
        let append_result = self.kv.append_tentative(request, token_count);
        assert(self.kv.append_refines(
            &entry.kv,
            request,
            token_count,
            &append_result,
        ));
        match &append_result {
            Ok(()) => {
                assert(self.scheduler.same_scalars(&entry.scheduler)) by {
                    assert(self.scheduler == entry.scheduler);
                    entry.scheduler.same_scalars_reflexive();
                }
                assert(!entry.faulted_spec());
                assert(entry.state_spec(request) == Some(RequestState::Ready));
                assert(exists |kv_result: Result<(), KvError>|
                    #[trigger] self.kv.append_refines(
                        &entry.kv,
                        request,
                        token_count,
                        &kv_result,
                    ) && match (&kv_result, &Ok::<(), EngineError>(())) {
                        (Ok(()), Ok(())) => true,
                        _ => false,
                    }) by {
                }
                assert(self.permits@ == entry.permits@);
                assert(self.faulted == entry.faulted);
                assert(self.append_tentative_refines(
                    &entry,
                    request,
                    token_count,
                    &Ok(()),
                ));
                Ok(())
            }
            Err(error) => {
                assert(self.scheduler.same_scalars(&entry.scheduler)) by {
                    assert(self.scheduler == entry.scheduler);
                    entry.scheduler.same_scalars_reflexive();
                }
                assert(!entry.faulted_spec());
                assert(entry.state_spec(request) == Some(RequestState::Ready));
                assert(exists |kv_result: Result<(), KvError>|
                    #[trigger] self.kv.append_refines(
                        &entry.kv,
                        request,
                        token_count,
                        &kv_result,
                    ) && match (
                        &kv_result,
                        &Err::<(), EngineError>(EngineError::Kv(*error)),
                    ) {
                        (Err(kv_error), Err(EngineError::Kv(engine_error))) => {
                            kv_error == engine_error
                        }
                        _ => false,
                    }) by {
                }
                assert(self.permits@ == entry.permits@);
                assert(self.faulted == entry.faulted);
                assert(self.append_tentative_refines(
                    &entry,
                    request,
                    token_count,
                    &Err(EngineError::Kv(*error)),
                ));
                Err(EngineError::Kv(*error))
            }
        }
    }

    /// Shares a page-aligned committed prefix between live requests.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Faulted`] after fail-stop, or the exact KV
    /// identity, alignment, target-state, or capacity error.
    pub fn share_committed_prefix(
        &mut self,
        source: RequestId,
        target: RequestId,
        token_count: u32,
    ) -> (result: Result<(), EngineError>)
        requires old(self).well_formed(),
        ensures
            final(self).well_formed(),
            final(self).share_committed_prefix_refines(
                old(self),
                source,
                target,
                token_count,
                &result,
            ),
    {
        let ghost entry = *self;
        reveal(Engine::well_formed);
        reveal(Engine::share_committed_prefix_refines);
        reveal(Engine::faulted_spec);
        reveal(Engine::state_spec);
        proof {
            self.scheduler.same_scalars_reflexive();
            self.kv.same_state_reflexive();
        }
        self.require_healthy()?;
        let source_state = self.scheduler.state(source);
        assert(source_state == self.scheduler.state_spec(source));
        assert(self.scheduler == entry.scheduler);
        assert(entry.state_spec(source) == source_state) by {
            reveal(Engine::state_spec);
        }
        match source_state {
            Some(RequestState::Ready) => {}
            _ => {
                assert(self.scheduler.same_scalars(&entry.scheduler)) by {
                    assert(self.scheduler == entry.scheduler);
                    entry.scheduler.same_scalars_reflexive();
                }
                assert(self.kv.same_state(&entry.kv)) by {
                    assert(self.kv == entry.kv);
                    entry.kv.same_state_reflexive();
                }
                assert(entry.state_spec(source) != Some(RequestState::Ready));
                assert(self.permits@ == entry.permits@);
                assert(self.faulted == entry.faulted);
                assert(self.share_committed_prefix_refines(
                    &entry,
                    source,
                    target,
                    token_count,
                    &Err(EngineError::RequestNotReady),
                ));
                return Err(EngineError::RequestNotReady);
            }
        }
        let target_state = self.scheduler.state(target);
        assert(target_state == self.scheduler.state_spec(target));
        assert(entry.state_spec(target) == target_state) by {
            reveal(Engine::state_spec);
        }
        match target_state {
            Some(RequestState::Ready) => {}
            _ => {
                assert(self.scheduler.same_scalars(&entry.scheduler)) by {
                    assert(self.scheduler == entry.scheduler);
                    entry.scheduler.same_scalars_reflexive();
                }
                assert(self.kv.same_state(&entry.kv)) by {
                    assert(self.kv == entry.kv);
                    entry.kv.same_state_reflexive();
                }
                assert(entry.state_spec(source) == Some(RequestState::Ready));
                assert(entry.state_spec(target) != Some(RequestState::Ready));
                assert(self.permits@ == entry.permits@);
                assert(self.faulted == entry.faulted);
                assert(self.share_committed_prefix_refines(
                    &entry,
                    source,
                    target,
                    token_count,
                    &Err(EngineError::RequestNotReady),
                ));
                return Err(EngineError::RequestNotReady);
            }
        }
        let share_result = self
            .kv
            .share_committed_prefix(source, target, token_count);
        assert(self.kv.share_refines(
            &entry.kv,
            source,
            target,
            token_count,
            &share_result,
        ));
        match &share_result {
            Ok(()) => {
                assert(self.scheduler.same_scalars(&entry.scheduler)) by {
                    assert(self.scheduler == entry.scheduler);
                    entry.scheduler.same_scalars_reflexive();
                }
                assert(!entry.faulted_spec());
                assert(entry.state_spec(source) == Some(RequestState::Ready));
                assert(entry.state_spec(target) == Some(RequestState::Ready));
                assert(exists |kv_result: Result<(), KvError>|
                    #[trigger] self.kv.share_refines(
                        &entry.kv,
                        source,
                        target,
                        token_count,
                        &kv_result,
                    ) && match (&kv_result, &Ok::<(), EngineError>(())) {
                        (Ok(()), Ok(())) => true,
                        _ => false,
                    }) by {
                }
                assert(self.permits@ == entry.permits@);
                assert(self.faulted == entry.faulted);
                assert(self.share_committed_prefix_refines(
                    &entry,
                    source,
                    target,
                    token_count,
                    &Ok(()),
                ));
                Ok(())
            }
            Err(error) => {
                assert(self.scheduler.same_scalars(&entry.scheduler)) by {
                    assert(self.scheduler == entry.scheduler);
                    entry.scheduler.same_scalars_reflexive();
                }
                assert(!entry.faulted_spec());
                assert(entry.state_spec(source) == Some(RequestState::Ready));
                assert(entry.state_spec(target) == Some(RequestState::Ready));
                assert(exists |kv_result: Result<(), KvError>|
                    #[trigger] self.kv.share_refines(
                        &entry.kv,
                        source,
                        target,
                        token_count,
                        &kv_result,
                    ) && match (
                        &kv_result,
                        &Err::<(), EngineError>(EngineError::Kv(*error)),
                    ) {
                        (Err(kv_error), Err(EngineError::Kv(engine_error))) => {
                            kv_error == engine_error
                        }
                        _ => false,
                    }) by {
                }
                assert(self.permits@ == entry.permits@);
                assert(self.faulted == entry.faulted);
                assert(self.share_committed_prefix_refines(
                    &entry,
                    source,
                    target,
                    token_count,
                    &Err(EngineError::Kv(*error)),
                ));
                Err(EngineError::Kv(*error))
            }
        }
    }

    /// Validates that a logical KV range is initialized for a live request.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Faulted`] after fail-stop, or the exact KV
    /// identity or initialized-range error.
    pub fn validate_read(
        &self,
        request: RequestId,
        logical_offset: u32,
        span: u32,
    ) -> (result: Result<(), EngineError>)
        requires self.well_formed(),
        ensures
            self.well_formed(),
            self.validate_read_refines(request, logical_offset, span, &result),
    {
        reveal(Engine::well_formed);
        reveal(Engine::validate_read_refines);
        reveal(Engine::faulted_spec);
        self.require_healthy()?;
        let read_result = self.kv.validate_read(request, logical_offset, span);
        assert(self.kv.read_refines(request, logical_offset, span, &read_result));
        match &read_result {
            Ok(()) => {
                assert(exists |kv_result: Result<(), KvError>|
                    #[trigger] self.kv.read_refines(
                        request,
                        logical_offset,
                        span,
                        &kv_result,
                    ) && match (&kv_result, &Ok::<(), EngineError>(())) {
                        (Ok(()), Ok(())) => true,
                        _ => false,
                    }) by {
                }
                assert(self.validate_read_refines(request, logical_offset, span, &Ok(())));
                Ok(())
            }
            Err(error) => {
                assert(exists |kv_result: Result<(), KvError>|
                    #[trigger] self.kv.read_refines(
                        request,
                        logical_offset,
                        span,
                        &kv_result,
                    ) && match (
                        &kv_result,
                        &Err::<(), EngineError>(EngineError::Kv(*error)),
                    ) {
                        (Err(kv_error), Err(EngineError::Kv(engine_error))) => {
                            kv_error == engine_error
                        }
                        _ => false,
                    }) by {
                }
                assert(self.validate_read_refines(
                    request,
                    logical_offset,
                    span,
                    &Err(EngineError::Kv(*error)),
                ));
                Err(EngineError::Kv(*error))
            }
        }
    }

    /// Moves a live request into terminal retirement.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Faulted`] after fail-stop, or the exact scheduler
    /// identity or lifecycle error.
    pub fn retire(&mut self, request: RequestId) -> (result: Result<(), EngineError>)
        requires old(self).well_formed(),
        ensures
            final(self).well_formed(),
            final(self).retire_refines(old(self), request, &result),
    {
        let ghost entry = *self;
        let ghost entry_scheduler = self.scheduler;
        reveal(Engine::well_formed);
        reveal(Engine::retire_refines);
        reveal(Engine::faulted_spec);
        proof {
            self.scheduler.same_scalars_reflexive();
            self.kv.same_state_reflexive();
        }
        self.require_healthy()?;
        assert(self.scheduler == entry_scheduler);
        let ghost before_retire_scheduler = self.scheduler;
        assert(before_retire_scheduler == entry.scheduler);
        let scheduler_result = self.scheduler.retire(request);
        assert(self.scheduler.retire_refines(
            &before_retire_scheduler,
            request,
            &scheduler_result,
        ));
        match &scheduler_result {
            Ok(()) => {
                assert(self.kv.same_state(&entry.kv)) by {
                    assert(self.kv == entry.kv);
                    entry.kv.same_state_reflexive();
                }
                assert(!entry.faulted_spec());
                assert(self.permits@ == entry.permits@);
                assert(self.faulted == entry.faulted);
                assert(exists |witness: Result<(), SchedulerError>|
                    self.scheduler.retire_refines(
                        &entry.scheduler,
                        request,
                        &witness,
                    )
                    && match (&witness, &Ok(())) {
                        (Ok(()), Ok(())) => true,
                        (Err(scheduler_error), Err(EngineError::Scheduler(engine_error))) => {
                            scheduler_error == engine_error
                        }
                        _ => false,
                    }) by {
                    assert(self.scheduler.retire_refines(
                        &entry.scheduler,
                        request,
                        &scheduler_result,
                    ));
                }
                assert(self.retire_refines(&entry, request, &Ok(())));
                Ok(())
            }
            Err(error) => {
                assert(self.kv.same_state(&entry.kv)) by {
                    assert(self.kv == entry.kv);
                    entry.kv.same_state_reflexive();
                }
                assert(!entry.faulted_spec());
                assert(self.permits@ == entry.permits@);
                assert(self.faulted == entry.faulted);
                assert(exists |witness: Result<(), SchedulerError>|
                    self.scheduler.retire_refines(
                        &entry.scheduler,
                        request,
                        &witness,
                    )
                    && match (
                        &witness,
                        &Err(EngineError::Scheduler(*error)),
                    ) {
                        (Ok(()), Ok(())) => true,
                        (Err(scheduler_error), Err(EngineError::Scheduler(engine_error))) => {
                            scheduler_error == engine_error
                        }
                        _ => false,
                    }) by {
                    assert(self.scheduler.retire_refines(
                        &entry.scheduler,
                        request,
                        &scheduler_result,
                    ));
                }
                assert(self.retire_refines(
                    &entry,
                    request,
                    &Err(EngineError::Scheduler(*error)),
                ));
                Err(EngineError::Scheduler(*error))
            }
        }
    }

    /// Reclaims one already-quiescent retired request, when available.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Faulted`] after fail-stop. An internal authority,
    /// KV, or scheduler mismatch also faults the engine and returns its error.
    pub fn reclaim_one(&mut self) -> (result: Result<Option<RequestId>, EngineError>)
        requires old(self).well_formed(),
        ensures
            final(self).well_formed(),
            final(self).reclaim_one_refines(old(self), &result),
    {
        let ghost entry = *self;
        reveal(Engine::well_formed);
        reveal(Engine::reclaim_one_refines);
        reveal(Engine::same_state);
        reveal(Engine::faulted_spec);
        proof {
            self.scheduler.same_scalars_reflexive();
            self.kv.same_state_reflexive();
        }
        self.require_healthy()?;
        assert(self.scheduler == entry.scheduler);
        assert(self.kv == entry.kv);
        let ghost entry_scheduler = entry.scheduler;
        let ghost entry_kv = entry.kv;
        let take_result = self.scheduler.take_retiring_permit();
        let permit = match take_result {
            Err(error) => {
                assert(self.kv.same_state(&entry.kv)) by {
                    assert(self.kv == entry.kv);
                    entry.kv.same_state_reflexive();
                }
                assert(self.permits@ == entry.permits@);
                assert(self.faulted == entry.faulted);
                assert(exists |scheduler_result: Result<
                    Option<KvQuiescencePermit>,
                    SchedulerError,
                >| #[trigger] self.scheduler.retiring_permit_refines(
                    &entry.scheduler,
                    &scheduler_result,
                ) && match (
                    &scheduler_result,
                    &Err::<Option<RequestId>, EngineError>(EngineError::Scheduler(error)),
                ) {
                    (Err(scheduler_error), Err(EngineError::Scheduler(engine_error))) => {
                        scheduler_error == engine_error
                    }
                    _ => false,
                }) by {
                }
                assert(self.well_formed());
                assert(self.reclaim_one_refines(
                    &entry,
                    &Err(EngineError::Scheduler(error)),
                ));
                return Err(EngineError::Scheduler(error));
            }
            Ok(None) => {
                assert(self.scheduler.retiring_permit_refines(
                    &entry.scheduler,
                    &Ok(None),
                ));
                assert(self.kv.same_state(&entry.kv)) by {
                    assert(self.kv == entry.kv);
                    entry.kv.same_state_reflexive();
                }
                assert(self.permits@ == entry.permits@);
                assert(self.faulted == entry.faulted);
                assert(!entry.faulted_spec());
                assert(self.well_formed());
                assert(self.reclaim_one_refines(&entry, &Ok(None)));
                return Ok(None);
            }
            Ok(Some(permit)) => permit,
        };
        let ghost taken_permit = permit;
        let request = permit.request();
        let ghost permit_request = permit.request_spec();
        let ghost permit_origin = permit.origin_spec();
        let ghost before_release_scheduler = self.scheduler;
        let ghost before_release_kv = self.kv;
        assert(before_release_scheduler.identity_frame(&entry_scheduler));
        assert(before_release_scheduler.retiring_permit_refines(
            &entry_scheduler,
            &Ok(Some(taken_permit)),
        ));
        let detached = match self.kv.release_request(request, permit) {
            Ok(detached) => detached,
            Err(failure) => {
                let ghost release_failure = failure;
                let (error, _permit) = failure.into_parts();
                self.faulted = true;
                assert(self.kv.same_state(&entry.kv));
                assert(self.permits@ == entry.permits@);
                assert(self.scheduler.same_scalars(&before_release_scheduler)) by {
                    assert(self.scheduler == before_release_scheduler);
                    before_release_scheduler.same_scalars_reflexive();
                }
                assert(self.reclaim_release_error_step(
                    &entry,
                    taken_permit,
                    before_release_scheduler,
                    error,
                )) by {
                    reveal(Engine::reclaim_release_error_step);
                    assert(before_release_kv == entry.kv);
                    assert(release_failure.error_spec() == error);
                }
                assert(exists |witness_permit: KvQuiescencePermit,
                    witness_after_take: Scheduler<C>|
                    #[trigger] self.reclaim_release_error_step(
                        &entry,
                        witness_permit,
                        witness_after_take,
                        error,
                    )) by {
                }
                assert(self.well_formed());
                assert(self.reclaim_one_refines(&entry, &Err(EngineError::Kv(error))));
                return Err(EngineError::Kv(error));
            }
        };
        let ghost released_detached = detached;
        let ghost after_release_kv = self.kv;
        assert(detached.request_spec() == permit_request);
        assert(detached.origin_spec() == permit_origin);
        assert(before_release_scheduler.detachment_ready(permit_request, permit_origin));
        let ghost before_reclaim_scheduler = self.scheduler;
        assert(before_reclaim_scheduler == before_release_scheduler);
        assert(before_reclaim_scheduler.detached_enabled(&detached)) by {
            reveal(Scheduler::detached_enabled);
            reveal(Scheduler::detachment_ready);
            assert(before_reclaim_scheduler.detachment_ready(
                detached.request_spec(),
                detached.origin_spec(),
            ));
            before_reclaim_scheduler.apply_detachment_ready_identity(
                detached.request_spec(),
                detached.origin_spec(),
            );
            assert(detached.request_spec() == request);
            assert(request.generation_spec() < u32::MAX);
        }
        match self.scheduler.reclaim_detached(detached) {
            Ok(request) => {
                assert forall |slot: int| 0 <= slot < C implies {
                    &&& self.scheduler.slot_is_live_spec(slot)
                        == self.kv.request_live_by_slot_spec(slot)
                    &&& self.scheduler.slot_generation_spec(slot)
                        == self.kv.request_generation_by_slot_spec(slot)
                } by {
                    if slot == request.slot_spec() {
                        assert(entry_scheduler.slot_is_live_spec(slot)
                            == entry_kv.request_live_by_slot_spec(slot));
                        assert(entry_scheduler.slot_generation_spec(slot)
                            == entry_kv.request_generation_by_slot_spec(slot));
                    } else {
                        assert(before_reclaim_scheduler.slot_is_live_spec(slot)
                            == entry_scheduler.slot_is_live_spec(slot));
                        assert(before_reclaim_scheduler.slot_generation_spec(slot)
                            == entry_scheduler.slot_generation_spec(slot));
                        assert(before_release_kv.request_live_by_slot_spec(slot)
                            == entry_kv.request_live_by_slot_spec(slot));
                        assert(before_release_kv.request_generation_by_slot_spec(slot)
                            == entry_kv.request_generation_by_slot_spec(slot));
                    }
                }
                assert forall |slot: int| C <= slot < MAX_REQUEST_SLOTS implies {
                    &&& !self.kv.request_live_by_slot_spec(slot)
                    &&& self.kv.request_generation_by_slot_spec(slot) == 1
                } by {
                    before_reclaim_scheduler.apply_detachment_ready_identity(
                        request,
                        permit_origin,
                    );
                    assert((request.slot_spec() as int) < C);
                    assert(slot != request.slot_spec() as int);
                    self.kv.apply_identity_frame_except(
                        &before_release_kv,
                        request.slot_spec() as int,
                        slot,
                    );
                }
                assert(request == permit_request);
                proof {
                    before_reclaim_scheduler.apply_detachment_ready_identity(
                        request,
                        permit_origin,
                    );
                }
                assert(self.permits@ == entry.permits@);
                assert(!entry.faulted_spec());
                assert(!self.faulted_spec());
                assert(self.kv.same_state(&after_release_kv)) by {
                    assert(self.kv == after_release_kv);
                    after_release_kv.same_state_reflexive();
                }
                assert(self.reclaim_success_step(
                    &entry,
                    taken_permit,
                    released_detached,
                    before_release_scheduler,
                    after_release_kv,
                    request,
                )) by {
                    reveal(Engine::reclaim_success_step);
                    assert(before_release_kv == entry.kv);
                    assert(before_reclaim_scheduler == before_release_scheduler);
                }
                assert(exists |witness_permit: KvQuiescencePermit,
                    witness_detached: KvDetachedRequest,
                    witness_after_take: Scheduler<C>,
                    witness_after_release: KvPool|
                    #[trigger] self.reclaim_success_step(
                        &entry,
                        witness_permit,
                        witness_detached,
                        witness_after_take,
                        witness_after_release,
                        request,
                    )) by {
                }
                assert(self.well_formed());
                assert(self.reclaim_success_refines(&entry, request)) by {
                    reveal(Engine::reclaim_success_refines);
                }
                assert(self.reclaim_one_refines(&entry, &Ok(Some(request)))) by {
                    reveal(Engine::reclaim_one_refines);
                }
                Ok(Some(request))
            }
            Err(error) => {
                assert(false);
                self.faulted = true;
                assert(self.well_formed());
                Err(EngineError::Scheduler(error))
            }
        }
    }

    /// Dispatches a deterministic ready batch into caller-owned storage.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Faulted`] after fail-stop, or the exact scheduler
    /// storage or epoch error.
    pub fn dispatch_ready(
        &mut self,
        output: &mut [RequestId],
    ) -> (result: Result<Option<DispatchBatch>, EngineError>)
        requires old(self).well_formed(),
        ensures
            final(self).well_formed(),
            final(self).dispatch_ready_refines(
                old(self),
                old(output)@,
                final(output)@,
                &result,
            ),
            match result {
                Ok(Some(batch)) => {
                    &&& batch.member_count_spec() > 0
                    &&& batch.member_count_spec() <= old(output)@.len()
                    &&& final(output)@.len() == old(output)@.len()
                    &&& batch.epoch_spec().value as int
                        == old(self).submitted_epoch_spec().value as int + 1
                    &&& final(self).submitted_epoch_spec() == batch.epoch_spec()
                    &&& batch.epoch_spec().value
                        > old(self).completed_epoch_spec().value
                    &&& forall|offset: int|
                        0 <= offset < batch.member_count_spec() ==> {
                            let request = #[trigger] final(output)@[offset];
                            &&& old(self).state_spec(request)
                                == Some(RequestState::Ready)
                            &&& final(self).state_spec(request)
                                == Some(RequestState::InFlight)
                        }
                    &&& forall|request: RequestId|
                        request.slot_spec() < C
                        && (forall|offset: int|
                            0 <= offset < batch.member_count_spec() ==>
                                #[trigger] final(output)@[offset].slot_spec()
                                    != request.slot_spec())
                        ==> final(self).state_spec(request)
                            == old(self).state_spec(request)
                },
                _ => true,
            },
    {
        reveal(Engine::well_formed);
        reveal(Engine::dispatch_ready_refines);
        reveal(Engine::faulted_spec);
        reveal(Engine::completed_epoch_spec);
        reveal(Engine::submitted_epoch_spec);
        reveal(Engine::state_spec);
        proof {
            self.scheduler.same_scalars_reflexive();
            self.kv.same_state_reflexive();
        }
        self.require_healthy()?;
        let ghost before_scheduler = self.scheduler;
        let ghost before_output = output@;
        let scheduler_result = self.scheduler.dispatch_ready(output);
        proof {
            match &scheduler_result {
                Ok(Some(batch)) => {
                    self.scheduler.expose_successful_dispatch_observations(
                        &before_scheduler,
                        before_output,
                        output@,
                        batch,
                    );
                }
                _ => {},
            }
        }
        proof {
            self.scheduler.apply_dispatch_refines(
                &before_scheduler,
                before_output,
                output@,
                &scheduler_result,
            );
        }
        proof {
            self.scheduler.apply_dispatch_basic(
                &before_scheduler,
                before_output,
                output@,
                &scheduler_result,
            );
        }
        proof {
            self.scheduler.apply_dispatch_identity(
                &before_scheduler,
                before_output,
                output@,
                &scheduler_result,
            );
        }
        match scheduler_result {
            Ok(batch) => Ok(batch),
            Err(error) => Err(EngineError::Scheduler(error)),
        }
    }

    /// Dispatches at most the M1 lane capacity and captures the exact selected roster.
    ///
    /// The scheduler's linear batch metadata and the caller-inaccessible stack
    /// scratch prefix move immediately into one [`M1ScheduledDispatchV1`].
    ///
    /// # Errors
    ///
    /// Returns the same fail-stop or scheduler error as [`Self::dispatch_ready`].
    pub fn dispatch_m1_ready(
        &mut self,
    ) -> (result: Result<Option<M1ScheduledDispatchV1>, EngineError>)
        requires old(self).well_formed(),
        ensures final(self).well_formed(),
    {
        let mut selected = [RequestId::new(0, 0); M1_MAX_ACTIVE_SEQUENCES as usize];
        match self.dispatch_ready(&mut selected)? {
            Some(batch) => Ok(Some(M1ScheduledDispatchV1::from_dispatch_batch(
                batch,
                &selected,
            ))),
            None => Ok(None),
        }
    }

    /// Applies one exact completion and its per-member accepted token counts.
    ///
    /// Caller-controlled length and acceptance bounds are checked before the
    /// completion authority is consumed. An internal generation-exhaustion or
    /// invariant failure after quiescence permanently faults the engine; no
    /// affected request becomes dispatchable or reusable.
    ///
    /// # Errors
    ///
    /// Returns a [`CompletionFailure`] for a rejected external result or an
    /// internal fail-stop error. The exact completion authority is returned
    /// only when it was not consumed.
    pub fn complete_exact(
        &mut self,
        completion: ExactCompletion,
        accepted_tokens: &[u32],
    ) -> (result: Result<usize, CompletionFailure>)
        requires old(self).well_formed(),
        ensures
            final(self).well_formed(),
            final(self).completion_refines(
                old(self),
                completion.epoch_spec(),
                accepted_tokens@,
                &result,
            ),
    {
        reveal(Engine::well_formed);
        reveal(Engine::faulted_spec);
        reveal(Engine::completion_refines);
        reveal(Engine::same_state);
        reveal(CompletionFailure::returns_completion_at_spec);
        reveal(CompletionFailure::consumed_completion_spec);
        let ghost entry = *self;
        let ghost input_completion_epoch = completion.epoch_spec();
        proof {
            self.scheduler.same_scalars_reflexive();
            self.kv.same_state_reflexive();
        }
        assert(self.same_state(&entry));
        assert(entry == *old(self));
        if let Err(error) = self.preflight_complete_exact(&completion, accepted_tokens) {
            let ghost returned_epoch = completion.epoch_spec();
            let result = Err(CompletionFailure::returned(error, completion));
            assert(self.completion_refines(
                &entry,
                returned_epoch,
                accepted_tokens@,
                &result,
            ));
            return result;
        }
        if self.faulted {
            return Err(CompletionFailure::returned(EngineError::Faulted, completion));
        }
        let member_count = self.scheduler.pending_batch_member_count();
        assert(member_count <= C);
        if accepted_tokens.len() != member_count {
            return Err(CompletionFailure::returned(
                EngineError::CompletionResultCount {
                    expected: member_count,
                    actual: accepted_tokens.len(),
                },
                completion,
            ));
        }

        let mut index = 0;
        while index < member_count
            invariant
                self.scheduler.basic_invariant(),
                self.kv.well_formed(),
                self.identity_agreement(),
                0 < C <= MAX_REQUEST_SLOTS,
                self.permits@.len() == C,
                forall |position: int| 0 <= position < C ==>
                    self.permits@[position].is_none(),
                self.same_state(&entry),
                entry == *old(self),
                completion.epoch_spec() == input_completion_epoch,
                !entry.faulted,
                !self.faulted,
                index <= member_count,
                member_count <= C,
                accepted_tokens@.len() == member_count,
            decreases member_count - index,
        {
            let request = match self.scheduler.pending_member(index) {
                Some(request) => request,
                None => {
                    self.faulted = true;
                    return Err(CompletionFailure::consumed(
                        EngineError::InvariantViolation,
                    ));
                }
            };
            if self.scheduler.state(request) == Some(RequestState::InFlight) {
                let resident = match self.kv.resident_tokens(request) {
                    Some(tokens) => tokens,
                    None => {
                        self.faulted = true;
                        return Err(CompletionFailure::consumed(
                            EngineError::InvariantViolation,
                        ));
                    }
                };
                let committed = match self.kv.committed_tokens(request) {
                    Some(tokens) => tokens,
                    None => {
                        self.faulted = true;
                        return Err(CompletionFailure::consumed(
                            EngineError::InvariantViolation,
                        ));
                    }
                };
                let tentative = match resident.checked_sub(committed) {
                    Some(tokens) => tokens,
                    None => {
                        self.faulted = true;
                        return Err(CompletionFailure::consumed(
                            EngineError::InvariantViolation,
                        ));
                    }
                };
                if accepted_tokens[index] > tentative {
                    let ghost returned_epoch = completion.epoch_spec();
                    let result = Err(CompletionFailure::returned(
                        EngineError::Kv(KvError::CommitExceedsResident),
                        completion,
                    ));
                    assert(self.completion_refines(
                        &entry,
                        returned_epoch,
                        accepted_tokens@,
                        &result,
                    ));
                    return result;
                }
            }
            index += 1;
        }

        let ghost before_completion_scheduler = self.scheduler;
        let ghost before_completion_permits = self.permits@;
        assert(before_completion_scheduler == entry.scheduler);
        assert(self.kv == entry.kv);
        assert(before_completion_permits.len() == C);
        assert(completion.epoch_spec() == input_completion_epoch);
        let completed = match self.scheduler.complete_exact(completion, &mut self.permits) {
            Ok(count) => count,
            Err(failure) => {
                let (error, completion) = failure.into_parts();
                assert forall |slot: int| 0 <= slot < C implies {
                    &&& self.scheduler.slot_is_live_spec(slot)
                        == self.kv.request_live_by_slot_spec(slot)
                    &&& self.scheduler.slot_generation_spec(slot)
                        == self.kv.request_generation_by_slot_spec(slot)
                } by {
                    assert(self.scheduler.slot_is_live_spec(slot)
                        == before_completion_scheduler.slot_is_live_spec(slot));
                    assert(self.scheduler.slot_generation_spec(slot)
                        == before_completion_scheduler.slot_generation_spec(slot));
                }
                assert(self.well_formed());
                return Err(CompletionFailure::returned(
                    EngineError::Scheduler(error),
                    completion,
                ));
            }
        };
        assert(completed == member_count);
        proof {
            self.scheduler.apply_completed_storage_length(
                &before_completion_scheduler,
                input_completion_epoch.value,
                before_completion_permits,
                self.permits@,
                completed,
            );
        }
        assert(completed <= self.permits@.len());
        assert(self.permits@.len() == C);
        assert forall |position: int| completed <= position < C implies
            self.permits@[position].is_none() by {
        }
        assert forall |slot: int| 0 <= slot < C implies {
            &&& self.scheduler.slot_is_live_spec(slot)
                == self.kv.request_live_by_slot_spec(slot)
            &&& self.scheduler.slot_generation_spec(slot)
                == self.kv.request_generation_by_slot_spec(slot)
        } by {
            assert(self.scheduler.slot_is_live_spec(slot)
                == before_completion_scheduler.slot_is_live_spec(slot));
            assert(self.scheduler.slot_generation_spec(slot)
                == before_completion_scheduler.slot_generation_spec(slot));
        }
        assert(self.identity_agreement());
        assert forall |position: int| 0 <= position < completed
            && self.scheduler.state_spec(
                self.permits@[position].unwrap().request_spec(),
            ) == Some(RequestState::Retiring) implies self.scheduler.detachment_ready(
                self.permits@[position].unwrap().request_spec(),
                self.permits@[position].unwrap().origin_spec(),
            ) by {
            self.scheduler.apply_completed_batch_member(
                &before_completion_scheduler,
                self.permits@,
                completed,
                position,
            );
        }

        assert(self.completion_prefix_refines(
            &entry,
            accepted_tokens@,
            0,
        )) by {
            reveal(Engine::completion_prefix_refines);
            reveal(Engine::state_spec);
            reveal(Engine::resident_tokens_spec);
            reveal(Engine::committed_tokens_spec);
            reveal(Engine::slot_is_live_spec);
            reveal(Engine::slot_generation_spec);
            assert(accepted_tokens@.len() == entry.pending_batch_member_count_spec());
            assert forall |offset: int|
                0 <= offset < entry.pending_batch_member_count_spec() implies match
                    #[trigger] entry.pending_member_spec(offset as usize)
                {
                    Some(request) => {
                        &&& self.state_spec(request) == entry.state_spec(request)
                        &&& self.resident_tokens_spec(request)
                            == entry.resident_tokens_spec(request)
                        &&& self.committed_tokens_spec(request)
                            == entry.committed_tokens_spec(request)
                        &&& self.slot_is_live_spec(request.slot_spec() as int)
                            == entry.slot_is_live_spec(request.slot_spec() as int)
                        &&& self.slot_generation_spec(request.slot_spec() as int)
                            == entry.slot_generation_spec(request.slot_spec() as int)
                    }
                    None => false,
                } by {
                assert(offset < completed);
                self.scheduler.apply_completed_batch_member(
                    &before_completion_scheduler,
                    self.permits@,
                    completed,
                    offset,
                );
                let permit = self.permits@[offset].unwrap();
                let request = permit.request_spec();
                assert(entry.pending_member_spec(offset as usize) == Some(request));
                assert(self.scheduler.state_spec(request)
                    == entry.scheduler.state_spec(request));
            }
        }
        assert(entry.pending_members_distinct(completed as int)) by {
            reveal(Engine::pending_members_distinct);
            assert forall |left: int, right: int| 0 <= left < right < completed implies match (
                #[trigger] entry.pending_member_spec(left as usize),
                #[trigger] entry.pending_member_spec(right as usize),
            ) {
                (Some(left_request), Some(right_request)) => {
                    left_request.slot_spec() != right_request.slot_spec()
                }
                _ => false,
            } by {
                self.scheduler.apply_completed_batch_member(
                    &before_completion_scheduler,
                    self.permits@,
                    completed,
                    left,
                );
                self.scheduler.apply_completed_batch_member(
                    &before_completion_scheduler,
                    self.permits@,
                    completed,
                    right,
                );
                self.scheduler.apply_completed_batch_distinct(
                    &before_completion_scheduler,
                    self.permits@,
                    completed,
                    left,
                    right,
                );
                assert(entry.pending_member_spec(left as usize)
                    == Some(self.permits@[left].unwrap().request_spec()));
                assert(entry.pending_member_spec(right as usize)
                    == Some(self.permits@[right].unwrap().request_spec()));
            }
        }
        assert(entry.pending_members_bounded(completed as int)) by {
            reveal(Engine::pending_members_bounded);
            assert forall |offset: int| 0 <= offset < completed implies match
                #[trigger] entry.pending_member_spec(offset as usize)
            {
                Some(request) => request.slot_spec() < C,
                None => false,
            } by {
                self.scheduler.apply_completed_batch_member(
                    &before_completion_scheduler,
                    self.permits@,
                    completed,
                    offset,
                );
                assert(entry.pending_member_spec(offset as usize)
                    == Some(self.permits@[offset].unwrap().request_spec()));
            }
        }
        assert forall |left: int, right: int| 0 <= left < right < completed implies
            self.permits@[left].unwrap().request_spec().slot_spec()
                != self.permits@[right].unwrap().request_spec().slot_spec() by {
            self.scheduler.apply_completed_batch_distinct(
                &before_completion_scheduler,
                self.permits@,
                completed,
                left,
                right,
            );
        }

        index = 0;
        while index < completed
            invariant
                self.scheduler.basic_invariant(),
                self.kv.well_formed(),
                self.identity_agreement(),
                0 < C <= MAX_REQUEST_SLOTS,
                self.permits@.len() == C,
                index <= completed,
                completed == entry.pending_batch_member_count_spec(),
                completed <= self.permits@.len(),
                completed <= accepted_tokens@.len(),
                forall |position: int| 0 <= position < index ==>
                    self.permits@[position].is_none(),
                forall |position: int| index <= position < completed ==>
                    self.permits@[position].is_some(),
                forall |position: int| index <= position < completed ==> match
                    #[trigger] entry.pending_member_spec(position as usize)
                {
                    Some(request) => {
                        self.permits@[position].unwrap().request_spec() == request
                    }
                    None => false,
                },
                forall |position: int| index <= position < completed ==>
                    self.permits@[position].unwrap().request_spec().slot_spec() < C,
                forall |left: int, right: int| index <= left < right < completed ==>
                    self.permits@[left].unwrap().request_spec().slot_spec()
                        != self.permits@[right].unwrap().request_spec().slot_spec(),
                forall |position: int| index <= position < completed
                    && self.scheduler.state_spec(
                        self.permits@[position].unwrap().request_spec(),
                    ) == Some(RequestState::Retiring) ==> self.scheduler.detachment_ready(
                        self.permits@[position].unwrap().request_spec(),
                        self.permits@[position].unwrap().origin_spec(),
                    ),
                forall |position: int| completed <= position < C ==>
                    self.permits@[position].is_none(),
                self.completion_prefix_refines(
                    &entry,
                    accepted_tokens@,
                    index as int,
                ),
                entry.pending_members_distinct(completed as int),
                entry.pending_members_bounded(completed as int),
                entry == *old(self),
                self.completed_epoch_spec() == input_completion_epoch,
                !entry.faulted,
                !self.faulted,
            decreases completed - index,
        {
            let ghost step_engine = *self;
            let ghost step_scheduler = self.scheduler;
            let ghost step_kv = self.kv;
            let ghost step_permits = self.permits@;
            assert(self.permits@[index as int].is_some());
            assert((index as int) as usize == index);
            assert(entry.pending_member_spec((index as int) as usize)
                == Some(self.permits@[index as int].unwrap().request_spec()));
            assert(entry.pending_member_spec(index)
                == Some(self.permits@[index as int].unwrap().request_spec()));
            let permit = match self.permits[index].take() {
                Some(permit) => permit,
                None => {
                    self.faulted = true;
                    self.clear_permits();
                    return Err(CompletionFailure::consumed(
                        EngineError::InvariantViolation,
                    ));
                }
            };
            let request = permit.request();
            let ghost permit_request = permit.request_spec();
            let ghost permit_origin = permit.origin_spec();
            assert(permit_request == step_permits[index as int].unwrap().request_spec());
            assert(entry.pending_member_spec(index) == Some(request));
            assert({
                &&& step_engine.state_spec(request) == entry.state_spec(request)
                &&& step_engine.resident_tokens_spec(request)
                    == entry.resident_tokens_spec(request)
                &&& step_engine.committed_tokens_spec(request)
                    == entry.committed_tokens_spec(request)
                &&& step_engine.slot_is_live_spec(request.slot_spec() as int)
                    == entry.slot_is_live_spec(request.slot_spec() as int)
                &&& step_engine.slot_generation_spec(request.slot_spec() as int)
                    == entry.slot_generation_spec(request.slot_spec() as int)
            }) by {
                reveal(Engine::completion_prefix_refines);
            }
            match self.scheduler.state(request) {
                Some(RequestState::InFlight) => {
                    let ghost before_accept_scheduler = self.scheduler;
                    let finalized = match self.kv.finalize_tentative(
                        request,
                        accepted_tokens[index],
                        permit,
                    ) {
                        Ok(finalized) => finalized,
                        Err(failure) => {
                            let (error, _permit) = failure.into_parts();
                            self.faulted = true;
                            self.clear_permits();
                            return Err(CompletionFailure::consumed(EngineError::Kv(error)));
                        }
                    };
                    let ghost finalized_request = finalized.request_spec();
                    assert(finalized_request == request);
                    let accept_result = self.scheduler.accept_finalized(finalized);
                    match accept_result {
                        Err(error) => {
                            self.faulted = true;
                            self.clear_permits();
                            return Err(CompletionFailure::consumed(EngineError::Scheduler(
                                error,
                            )));
                        }
                        Ok(()) => {}
                    }
                    assert(before_accept_scheduler == step_scheduler);
                    assert(self.scheduler.detachment_ready_frame_except(
                        &step_scheduler,
                        finalized_request.slot_spec() as int,
                    ));
                    assert(finalized_request.slot_spec() == request.slot_spec());
                    assert forall |position: int| index < position < completed
                        && self.scheduler.state_spec(
                            self.permits@[position].unwrap().request_spec(),
                        ) == Some(RequestState::Retiring) implies
                            self.scheduler.detachment_ready(
                                self.permits@[position].unwrap().request_spec(),
                                self.permits@[position].unwrap().origin_spec(),
                            ) by {
                        assert(self.permits@[position] == step_permits[position]);
                        assert(self.permits@[position].unwrap().request_spec().slot_spec()
                            != request.slot_spec());
                        self.scheduler.apply_detachment_ready_frame_except(
                            &step_scheduler,
                            request.slot_spec() as int,
                            self.permits@[position].unwrap().request_spec(),
                            self.permits@[position].unwrap().origin_spec(),
                        );
                        assert(self.scheduler.state_spec(
                            self.permits@[position].unwrap().request_spec(),
                        ) == before_accept_scheduler.state_spec(
                            self.permits@[position].unwrap().request_spec(),
                        ));
                        assert(step_scheduler.state_spec(
                            step_permits[position].unwrap().request_spec(),
                        ) == Some(RequestState::Retiring));
                        assert(step_scheduler.detachment_ready(
                            step_permits[position].unwrap().request_spec(),
                            step_permits[position].unwrap().origin_spec(),
                        ));
                        assert(before_accept_scheduler.detachment_ready(
                            self.permits@[position].unwrap().request_spec(),
                            self.permits@[position].unwrap().origin_spec(),
                        ));
                    }
                    assert(self.observations_frame_except(
                        &step_engine,
                        request.slot_spec() as int,
                    )) by {
                        reveal(Engine::observations_frame_except);
                        reveal(Engine::state_spec);
                        reveal(Engine::resident_tokens_spec);
                        reveal(Engine::committed_tokens_spec);
                        reveal(Engine::slot_is_live_spec);
                        reveal(Engine::slot_generation_spec);
                        assert(entry.state_spec(request) == Some(RequestState::InFlight));
                        assert(self.state_spec(request) == Some(RequestState::Ready));
                        assert(self.resident_tokens_spec(request)
                            == self.committed_tokens_spec(request));
                        assert(self.committed_tokens_spec(request).is_some());
                        assert(entry.committed_tokens_spec(request).is_some());
                        assert(self.committed_tokens_spec(request).unwrap() as int
                            == entry.committed_tokens_spec(request).unwrap() as int
                                + accepted_tokens@[index as int] as int);
                        assert(self.slot_is_live_spec(request.slot_spec() as int));
                        assert(self.slot_generation_spec(request.slot_spec() as int)
                            == entry.slot_generation_spec(request.slot_spec() as int));
                        assert forall |other: RequestId| other.slot_spec() < C
                            && other.slot_spec() != request.slot_spec() implies {
                                &&& self.state_spec(other) == step_engine.state_spec(other)
                                &&& self.resident_tokens_spec(other)
                                    == step_engine.resident_tokens_spec(other)
                                &&& self.committed_tokens_spec(other)
                                    == step_engine.committed_tokens_spec(other)
                            } by {
                            self.scheduler.apply_detachment_ready_frame_except(
                                &step_scheduler,
                                request.slot_spec() as int,
                                other,
                                permit_origin,
                            );
                            self.kv.request_frame_preserves_other(
                                &step_kv,
                                request.slot_spec() as int,
                                other,
                            );
                        }
                        assert forall |slot: int| 0 <= slot < C
                            && slot != request.slot_spec() as int implies {
                                &&& self.slot_is_live_spec(slot)
                                    == step_engine.slot_is_live_spec(slot)
                                &&& self.slot_generation_spec(slot)
                                    == step_engine.slot_generation_spec(slot)
                            } by {
                        }
                    }
                    assert(entry.pending_member_spec(index) == Some(request));
                    assert(self.member_completion_refines(
                        &entry,
                        accepted_tokens@,
                        index as int,
                    )) by {
                        reveal(Engine::member_completion_refines);
                        reveal(Engine::state_spec);
                        reveal(Engine::resident_tokens_spec);
                        reveal(Engine::committed_tokens_spec);
                        reveal(Engine::slot_is_live_spec);
                        reveal(Engine::slot_generation_spec);
                    }
                }
                Some(RequestState::Retiring) => {
                    assert(request == permit_request);
                    assert(self.scheduler.state_spec(request)
                        == Some(RequestState::Retiring));
                    assert(self.scheduler.detachment_ready(request, permit_origin));
                    let ghost before_release_kv = self.kv;
                    let ghost before_reclaim_scheduler = self.scheduler;
                    assert(before_reclaim_scheduler == step_scheduler);
                    let detached = match self.kv.release_request(request, permit) {
                        Ok(detached) => detached,
                        Err(failure) => {
                            let (error, _permit) = failure.into_parts();
                            self.faulted = true;
                            self.clear_permits();
                            return Err(CompletionFailure::consumed(EngineError::Kv(error)));
                        }
                    };
                    assert(detached.request_spec() == request);
                    assert(detached.origin_spec() == permit_origin);
                    assert(request.generation_spec() < u32::MAX);
                    assert(before_reclaim_scheduler.detachment_ready(
                        detached.request_spec(),
                        detached.origin_spec(),
                    ));
                    assert(before_reclaim_scheduler.detached_enabled(&detached));
                    let ghost detached_request = detached.request_spec();
                    let reclaim_result = self.scheduler.reclaim_detached(detached);
                    match reclaim_result {
                        Ok(_reclaimed) => {
                            assert(_reclaimed == request);
                            assert(self.kv.identity_frame_except(
                                &before_release_kv,
                                request.slot_spec() as int,
                            ));
                            assert(self.scheduler.detachment_ready_frame_except(
                                &step_scheduler,
                                detached_request.slot_spec() as int,
                            ));
                            assert(detached_request.slot_spec() == request.slot_spec());
                            assert forall |slot: int| 0 <= slot < C implies {
                                &&& self.scheduler.slot_is_live_spec(slot)
                                    == self.kv.request_live_by_slot_spec(slot)
                                &&& self.scheduler.slot_generation_spec(slot)
                                    == self.kv.request_generation_by_slot_spec(slot)
                            } by {
                                if slot == _reclaimed.slot_spec() {
                                    assert(slot == request.slot_spec());
                                    assert(before_reclaim_scheduler.slot_is_live_spec(slot)
                                        == before_release_kv.request_live_by_slot_spec(slot));
                                    assert(before_reclaim_scheduler.slot_generation_spec(slot)
                                        == before_release_kv.request_generation_by_slot_spec(slot));
                                } else {
                                    assert(slot != request.slot_spec());
                                    assert(self.scheduler.slot_is_live_spec(slot)
                                        == before_reclaim_scheduler.slot_is_live_spec(slot));
                                    assert(self.scheduler.slot_generation_spec(slot)
                                        == before_reclaim_scheduler.slot_generation_spec(slot));
                                    assert(self.kv.request_live_by_slot_spec(slot)
                                        == before_release_kv.request_live_by_slot_spec(slot));
                                    assert(self.kv.request_generation_by_slot_spec(slot)
                                        == before_release_kv.request_generation_by_slot_spec(slot));
                                }
                            }
                            assert forall |position: int| index < position < completed
                                && self.scheduler.state_spec(
                                    self.permits@[position].unwrap().request_spec(),
                                ) == Some(RequestState::Retiring) implies
                                    self.scheduler.detachment_ready(
                                        self.permits@[position].unwrap().request_spec(),
                                        self.permits@[position].unwrap().origin_spec(),
                                    ) by {
                                assert(self.permits@[position] == step_permits[position]);
                                assert(self.permits@[position].unwrap()
                                    .request_spec().slot_spec() != request.slot_spec());
                                self.scheduler.apply_detachment_ready_frame_except(
                                    &step_scheduler,
                                    request.slot_spec() as int,
                                    self.permits@[position].unwrap().request_spec(),
                                    self.permits@[position].unwrap().origin_spec(),
                                );
                                assert(self.scheduler.state_spec(
                                    self.permits@[position].unwrap().request_spec(),
                                ) == before_reclaim_scheduler.state_spec(
                                    self.permits@[position].unwrap().request_spec(),
                                ));
                                assert(step_scheduler.state_spec(
                                    step_permits[position].unwrap().request_spec(),
                                ) == Some(RequestState::Retiring));
                                assert(step_scheduler.detachment_ready(
                                    step_permits[position].unwrap().request_spec(),
                                    step_permits[position].unwrap().origin_spec(),
                                ));
                                assert(before_reclaim_scheduler.detachment_ready(
                                    self.permits@[position].unwrap().request_spec(),
                                    self.permits@[position].unwrap().origin_spec(),
                                ));
                            }
                            assert(self.observations_frame_except(
                                &step_engine,
                                request.slot_spec() as int,
                            )) by {
                                reveal(Engine::observations_frame_except);
                                reveal(Engine::state_spec);
                                reveal(Engine::resident_tokens_spec);
                                reveal(Engine::committed_tokens_spec);
                                reveal(Engine::slot_is_live_spec);
                                reveal(Engine::slot_generation_spec);
                                assert forall |other: RequestId| other.slot_spec() < C
                                    && other.slot_spec() != request.slot_spec() implies {
                                        &&& self.state_spec(other)
                                            == step_engine.state_spec(other)
                                        &&& self.resident_tokens_spec(other)
                                            == step_engine.resident_tokens_spec(other)
                                        &&& self.committed_tokens_spec(other)
                                            == step_engine.committed_tokens_spec(other)
                                    } by {
                                    self.scheduler.apply_detachment_ready_frame_except(
                                        &step_scheduler,
                                        request.slot_spec() as int,
                                        other,
                                        permit_origin,
                                    );
                                    self.kv.request_frame_preserves_other(
                                        &step_kv,
                                        request.slot_spec() as int,
                                        other,
                                    );
                                }
                                assert forall |slot: int| 0 <= slot < C
                                    && slot != request.slot_spec() as int implies {
                                        &&& self.slot_is_live_spec(slot)
                                            == step_engine.slot_is_live_spec(slot)
                                        &&& self.slot_generation_spec(slot)
                                            == step_engine.slot_generation_spec(slot)
                                    } by {
                                }
                            }
                            assert(entry.pending_member_spec(index) == Some(request));
                            assert(self.member_completion_refines(
                                &entry,
                                accepted_tokens@,
                                index as int,
                            )) by {
                                reveal(Engine::member_completion_refines);
                                reveal(Engine::state_spec);
                                reveal(Engine::resident_tokens_spec);
                                reveal(Engine::committed_tokens_spec);
                                reveal(Engine::slot_is_live_spec);
                                reveal(Engine::slot_generation_spec);
                                assert(entry.state_spec(request)
                                    == Some(RequestState::Retiring));
                                assert(self.state_spec(request).is_none());
                                assert(self.resident_tokens_spec(request).is_none());
                                assert(self.committed_tokens_spec(request).is_none());
                                assert(!self.slot_is_live_spec(request.slot_spec() as int));
                                assert(self.slot_generation_spec(request.slot_spec() as int) as int
                                    == entry.slot_generation_spec(request.slot_spec() as int) as int
                                        + 1);
                            }
                        }
                        Err(error) => {
                            assert(false);
                            self.faulted = true;
                            return Err(CompletionFailure::consumed(EngineError::Scheduler(
                                error,
                            )));
                        }
                    }
                }
                Some(RequestState::Ready | RequestState::Vacant) | None => {
                    self.faulted = true;
                    self.clear_permits();
                    return Err(CompletionFailure::consumed(
                        EngineError::InvariantViolation,
                    ));
                }
            }
            proof {
                self.advance_completion_prefix(
                    &step_engine,
                    &entry,
                    accepted_tokens@,
                    index as int,
                    request,
                );
            }
            index += 1;
        }
        assert(self.well_formed());
        assert(self.completion_prefix_refines(
            &entry,
            accepted_tokens@,
            completed as int,
        ));
        assert(self.completed_epoch_spec() == input_completion_epoch);
        assert(self.completion_refines(
            &entry,
            input_completion_epoch,
            accepted_tokens@,
            &Ok(completed),
        ));
        Ok(completed)
    }

    fn clear_permits(&mut self)
        requires
            old(self).scheduler.basic_invariant(),
            old(self).kv.well_formed(),
            old(self).identity_agreement(),
            old(self).permits@.len() == C,
            0 < C <= MAX_REQUEST_SLOTS,
        ensures
            final(self).scheduler.basic_invariant(),
            final(self).kv.well_formed(),
            final(self).identity_agreement(),
            final(self).permits@.len() == C,
            forall |index: int| 0 <= index < C ==>
                final(self).permits@[index].is_none(),
            final(self).faulted == old(self).faulted,
    {
        let mut index = 0;
        while index < C
            invariant
                self.scheduler.basic_invariant(),
                self.kv.well_formed(),
                self.identity_agreement(),
                self.permits@.len() == C,
                index <= C,
                forall |position: int| 0 <= position < index ==>
                    self.permits@[position].is_none(),
                self.faulted == old(self).faulted,
            decreases C - index,
        {
            self.permits.set(index, None);
            index += 1;
        }
    }

    #[must_use]
    pub fn state(&self, request: RequestId) -> (state: Option<RequestState>)
        ensures state == self.state_spec(request),
    {
        self.scheduler.state(request)
    }

    #[must_use]
    pub fn resident_tokens(&self, request: RequestId) -> (tokens: Option<u32>)
        requires self.well_formed(),
        ensures tokens == self.resident_tokens_spec(request),
    {
        reveal(Engine::well_formed);
        self.kv.resident_tokens(request)
    }

    #[must_use]
    pub fn committed_tokens(&self, request: RequestId) -> (tokens: Option<u32>)
        requires self.well_formed(),
        ensures tokens == self.committed_tokens_spec(request),
    {
        reveal(Engine::well_formed);
        self.kv.committed_tokens(request)
    }

    #[must_use]
    pub fn free_pages(&self) -> (pages: u32)
        ensures pages == self.free_pages_spec(),
    {
        self.kv.free_pages()
    }

    fn require_healthy(&self) -> (result: Result<(), EngineError>)
        ensures result == if self.faulted { Err(EngineError::Faulted) } else { Ok(()) },
    {
        if self.faulted {
            Err(EngineError::Faulted)
        } else {
            Ok(())
        }
    }
}

}

#[cfg(test)]
mod tests {
    use super::{Engine, EngineError};
    use crate::epoch::ExactCompletion;
    use ferric_spec::completion::CompletionEpoch;
    use ferric_spec::scheduling::RequestState;
    use ferric_spec::RequestId;

    fn output<const N: usize>() -> [RequestId; N] {
        [RequestId::new(0, 0); N]
    }

    #[test]
    fn completion_publishes_before_redispatch() {
        let mut engine = Engine::<4>::new(32, 4, 64).unwrap();
        let request = engine.admit().unwrap();
        engine.append_tentative(request, 4).unwrap();
        let mut members = output::<4>();
        let batch = engine.dispatch_ready(&mut members).unwrap().unwrap();
        assert_eq!(members[0], request);

        let completion = ExactCompletion::from_contracted_hsa_quiescence(batch.epoch());
        assert_eq!(engine.complete_exact(completion, &[2]).unwrap(), 1);
        assert_eq!(engine.committed_tokens(request), Some(2));
        assert_eq!(engine.resident_tokens(request), Some(2));
        assert_eq!(engine.state(request), Some(RequestState::Ready));
    }

    #[test]
    fn m1_dispatch_owns_exact_selected_prefix_and_canonical_padding() {
        let mut engine = Engine::<4>::new(32, 4, 64).unwrap();
        let first = engine.admit().unwrap();
        let second = engine.admit().unwrap();
        engine.append_tentative(first, 1).unwrap();
        engine.append_tentative(second, 1).unwrap();

        let scheduled = engine.dispatch_m1_ready().unwrap().unwrap();
        assert_eq!(scheduled.epoch(), CompletionEpoch::new(1));
        assert_eq!(scheduled.member_count(), 2);
        assert_eq!(scheduled.member(0), Some(first));
        assert_eq!(scheduled.member(1), Some(second));
        assert!(scheduled.members()[2..].iter().all(Option::is_none));
    }

    #[test]
    fn invalid_acceptance_is_rejected_before_completion_advances() {
        let mut engine = Engine::<2>::new(8, 4, 32).unwrap();
        let request = engine.admit().unwrap();
        engine.append_tentative(request, 1).unwrap();
        let mut members = output::<2>();
        let batch = engine.dispatch_ready(&mut members).unwrap().unwrap();
        let completion = ExactCompletion::from_contracted_hsa_quiescence(batch.epoch());

        let failure = engine.complete_exact(completion, &[2]).unwrap_err();
        assert_eq!(
            failure.error(),
            EngineError::Kv(crate::KvError::CommitExceedsResident)
        );
        assert!(failure.into_completion().is_some());
        assert_eq!(engine.completed_epoch(), CompletionEpoch::new(0));
        assert_eq!(engine.state(request), Some(RequestState::InFlight));
    }

    #[test]
    fn scheduler_rejection_returns_authority_and_preserves_engine_for_retry() {
        let mut engine = Engine::<2>::new(8, 4, 32).unwrap();
        let request = engine.admit().unwrap();
        engine.append_tentative(request, 1).unwrap();
        let mut members = output::<2>();
        let batch = engine.dispatch_ready(&mut members).unwrap().unwrap();
        let later_epoch = CompletionEpoch::new(batch.epoch().value() + 1);
        let later = ExactCompletion::from_contracted_hsa_quiescence(later_epoch);

        let failure = engine.complete_exact(later, &[1]).unwrap_err();
        assert_eq!(
            failure.error(),
            EngineError::Scheduler(crate::SchedulerError::CompletionNotExactNext)
        );
        assert_eq!(failure.into_completion().unwrap().epoch(), later_epoch);
        assert_eq!(engine.completed_epoch(), CompletionEpoch::new(0));
        assert_eq!(engine.state(request), Some(RequestState::InFlight));
        assert_eq!(engine.committed_tokens(request), Some(0));

        let exact = ExactCompletion::from_contracted_hsa_quiescence(batch.epoch());
        assert_eq!(engine.complete_exact(exact, &[1]).unwrap(), 1);
        assert_eq!(engine.state(request), Some(RequestState::Ready));
        assert_eq!(engine.committed_tokens(request), Some(1));
    }

    #[test]
    fn in_flight_kv_mutation_is_rejected_transactionally() {
        let mut engine = Engine::<2>::new(8, 4, 32).unwrap();
        let request = engine.admit().unwrap();
        engine.append_tentative(request, 1).unwrap();
        let mut members = output::<2>();
        engine.dispatch_ready(&mut members).unwrap().unwrap();

        assert_eq!(
            engine.append_tentative(request, 1),
            Err(EngineError::RequestNotReady)
        );
        assert_eq!(engine.resident_tokens(request), Some(1));
        assert_eq!(engine.committed_tokens(request), Some(0));
        assert_eq!(engine.state(request), Some(RequestState::InFlight));
    }

    #[test]
    fn prefix_share_rejects_in_flight_source_and_target() {
        let mut engine = Engine::<2>::new(8, 4, 32).unwrap();
        let source = engine.admit().unwrap();
        let target = engine.admit().unwrap();
        let mut member = output::<1>();

        let source_batch = engine.dispatch_ready(&mut member).unwrap().unwrap();
        assert_eq!(member[0], source);
        assert_eq!(
            engine.share_committed_prefix(source, target, 4),
            Err(EngineError::RequestNotReady)
        );
        engine
            .complete_exact(
                ExactCompletion::from_contracted_hsa_quiescence(source_batch.epoch()),
                &[0],
            )
            .unwrap();

        let target_batch = engine.dispatch_ready(&mut member).unwrap().unwrap();
        assert_eq!(member[0], target);
        assert_eq!(
            engine.share_committed_prefix(source, target, 4),
            Err(EngineError::RequestNotReady)
        );
        assert_eq!(engine.state(source), Some(RequestState::Ready));
        assert_eq!(engine.state(target), Some(RequestState::InFlight));
        assert_eq!(engine.free_pages(), 8);

        engine
            .complete_exact(
                ExactCompletion::from_contracted_hsa_quiescence(target_batch.epoch()),
                &[0],
            )
            .unwrap();
    }

    #[test]
    fn ready_retirement_detaches_before_slot_reuse() {
        let mut engine = Engine::<1>::new(8, 4, 32).unwrap();
        let first = engine.admit().unwrap();
        engine.retire(first).unwrap();
        assert_eq!(engine.reclaim_one().unwrap(), Some(first));
        assert_eq!(engine.state(first), None);
        let second = engine.admit().unwrap();
        assert_eq!(second.slot(), first.slot());
        assert_eq!(second.generation(), first.generation() + 1);
    }

    #[test]
    fn in_flight_retirement_reclaims_only_after_exact_completion() {
        let mut engine = Engine::<1>::new(8, 4, 32).unwrap();
        let request = engine.admit().unwrap();
        engine.append_tentative(request, 1).unwrap();
        let mut members = output::<1>();
        let batch = engine.dispatch_ready(&mut members).unwrap().unwrap();
        engine.retire(request).unwrap();
        assert_eq!(engine.reclaim_one().unwrap(), None);

        let completion = ExactCompletion::from_contracted_hsa_quiescence(batch.epoch());
        assert_eq!(engine.complete_exact(completion, &[0]).unwrap(), 1);
        assert_eq!(engine.state(request), None);
        let reused = engine.admit().unwrap();
        assert_eq!(reused.generation(), request.generation() + 1);
    }

    #[test]
    fn mixed_completion_publishes_active_and_detaches_retired_members() {
        let mut engine = Engine::<2>::new(16, 4, 32).unwrap();
        let retired = engine.admit().unwrap();
        let active = engine.admit().unwrap();
        engine.append_tentative(retired, 2).unwrap();
        engine.append_tentative(active, 3).unwrap();
        let mut members = output::<2>();
        let batch = engine.dispatch_ready(&mut members).unwrap().unwrap();
        assert_eq!(members, [retired, active]);
        engine.retire(retired).unwrap();

        let completion = ExactCompletion::from_contracted_hsa_quiescence(batch.epoch());
        assert_eq!(engine.complete_exact(completion, &[0, 2]).unwrap(), 2);
        assert_eq!(engine.state(retired), None);
        assert_eq!(engine.resident_tokens(retired), None);
        assert_eq!(engine.state(active), Some(RequestState::Ready));
        assert_eq!(engine.committed_tokens(active), Some(2));
        assert_eq!(engine.resident_tokens(active), Some(2));
        assert_eq!(engine.live_count(), 1);
    }

    #[test]
    fn exact_completion_preflight_rejects_epoch_count_and_acceptance_without_mutation() {
        let mut engine = Engine::<2>::new(16, 4, 32).unwrap();
        let first = engine.admit().unwrap();
        let second = engine.admit().unwrap();
        engine.append_tentative(first, 2).unwrap();
        engine.append_tentative(second, 3).unwrap();
        let mut members = output::<2>();
        let batch = engine.dispatch_ready(&mut members).unwrap().unwrap();
        assert_eq!(members, [first, second]);

        let exact = ExactCompletion::from_contracted_hsa_quiescence(batch.epoch());
        assert_eq!(engine.preflight_complete_exact(&exact, &[1, 2]), Ok(()));
        assert!(matches!(
            engine.preflight_complete_exact(&exact, &[1]),
            Err(EngineError::CompletionResultCount {
                expected: 2,
                actual: 1
            })
        ));
        assert_eq!(
            engine.preflight_complete_exact(&exact, &[3, 2]),
            Err(EngineError::Kv(super::KvError::CommitExceedsResident))
        );
        let late = ExactCompletion::from_contracted_hsa_quiescence(CompletionEpoch::new(
            batch.epoch().value() + 1,
        ));
        assert!(matches!(
            engine.preflight_complete_exact(&late, &[1, 2]),
            Err(EngineError::Scheduler(
                super::SchedulerError::CompletionNotExactNext
            ))
        ));
        assert_eq!(engine.completed_epoch(), CompletionEpoch::new(0));
        assert_eq!(engine.state(first), Some(RequestState::InFlight));
        assert_eq!(engine.state(second), Some(RequestState::InFlight));
        assert_eq!(engine.resident_tokens(first), Some(2));
        assert_eq!(engine.resident_tokens(second), Some(3));
    }

    #[test]
    fn retiring_preflight_checks_future_exact_detachment_without_settling() {
        let mut engine = Engine::<1>::new(8, 4, 32).unwrap();
        let request = engine.admit().unwrap();
        engine.append_tentative(request, 1).unwrap();
        let mut members = output::<1>();
        let batch = engine.dispatch_ready(&mut members).unwrap().unwrap();
        engine.retire(request).unwrap();
        let exact = ExactCompletion::from_contracted_hsa_quiescence(batch.epoch());

        assert_eq!(engine.preflight_complete_exact(&exact, &[0]), Ok(()));
        assert_eq!(engine.state(request), Some(RequestState::Retiring));
        assert_eq!(engine.completed_epoch(), CompletionEpoch::new(0));
        assert_eq!(engine.resident_tokens(request), Some(1));
    }

    #[test]
    fn engine_transitions_preserve_completion_scratch_capacity() {
        let mut engine = Engine::<2>::new(16, 4, 32).unwrap();
        let scratch_len = engine.permits.len();
        let scratch_capacity = engine.permits.capacity();
        let request = engine.admit().unwrap();
        engine.append_tentative(request, 2).unwrap();
        let mut members = output::<2>();
        let batch = engine.dispatch_ready(&mut members).unwrap().unwrap();
        let completion = ExactCompletion::from_contracted_hsa_quiescence(batch.epoch());
        engine.complete_exact(completion, &[1]).unwrap();
        engine.retire(request).unwrap();
        engine.reclaim_one().unwrap();

        assert_eq!(engine.permits.len(), scratch_len);
        assert_eq!(engine.permits.capacity(), scratch_capacity);
    }
}
