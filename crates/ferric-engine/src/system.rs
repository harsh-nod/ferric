//! Composed request lifecycle and KV ownership.

#[allow(unused_imports)]
use crate::cache::{KvError, KvPool, MAX_REQUEST_SLOTS};
use crate::epoch::ExactCompletion;
use crate::scheduler::{DispatchBatch, KvQuiescencePermit, Scheduler, SchedulerError};
use ferric_spec::completion::CompletionEpoch;
use ferric_spec::scheduling::RequestState;
use ferric_spec::RequestId;
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
    pub const fn error(&self) -> EngineError {
        self.error
    }

    #[must_use]
    pub fn into_completion(self) -> Option<ExactCompletion> {
        self.completion
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

    pub closed spec fn live_count_spec(&self) -> usize {
        self.scheduler.live_count_spec()
    }

    pub closed spec fn completed_epoch_spec(&self) -> CompletionEpoch {
        self.scheduler.completed_epoch_spec()
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

    /// Admits one request generation into the scheduler and KV pool.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Faulted`] after fail-stop, or the exact scheduler
    /// or KV admission error.
    pub fn admit(&mut self) -> (result: Result<RequestId, EngineError>)
        requires old(self).well_formed(),
        ensures final(self).well_formed(),
    {
        reveal(Engine::well_formed);
        self.require_healthy()?;
        let request = match self.scheduler.admit() {
            Ok(request) => request,
            Err(error) => return Err(EngineError::Scheduler(error)),
        };
        if let Err(error) = self.kv.create_request(request) {
            self.faulted = true;
            return Err(EngineError::Kv(error));
        }
        Ok(request)
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
        ensures final(self).well_formed(),
    {
        reveal(Engine::well_formed);
        self.require_healthy()?;
        if self.scheduler.state(request) != Some(RequestState::Ready) {
            return Err(EngineError::RequestNotReady);
        }
        match self.kv.append_tentative(request, token_count) {
            Ok(()) => Ok(()),
            Err(error) => Err(EngineError::Kv(error)),
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
        ensures final(self).well_formed(),
    {
        reveal(Engine::well_formed);
        self.require_healthy()?;
        if self.scheduler.state(source) != Some(RequestState::Ready)
            || self.scheduler.state(target) != Some(RequestState::Ready)
        {
            return Err(EngineError::RequestNotReady);
        }
        match self
            .kv
            .share_committed_prefix(source, target, token_count)
        {
            Ok(()) => Ok(()),
            Err(error) => Err(EngineError::Kv(error)),
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
        ensures self.well_formed(),
    {
        reveal(Engine::well_formed);
        self.require_healthy()?;
        match self.kv.validate_read(request, logical_offset, span) {
            Ok(()) => Ok(()),
            Err(error) => Err(EngineError::Kv(error)),
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
        ensures final(self).well_formed(),
    {
        reveal(Engine::well_formed);
        self.require_healthy()?;
        match self.scheduler.retire(request) {
            Ok(()) => Ok(()),
            Err(error) => Err(EngineError::Scheduler(error)),
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
        ensures final(self).well_formed(),
    {
        reveal(Engine::well_formed);
        self.require_healthy()?;
        let ghost entry_scheduler = self.scheduler;
        let ghost entry_kv = self.kv;
        let permit_result = match self.scheduler.take_retiring_permit() {
            Ok(result) => result,
            Err(error) => return Err(EngineError::Scheduler(error)),
        };
        let permit = match permit_result {
            Some(permit) => permit,
            None => {
                assert(self.well_formed());
                return Ok(None);
            }
        };
        let request = permit.request();
        let ghost before_release_kv = self.kv;
        let detached = match self.kv.release_request(request, permit) {
            Ok(detached) => detached,
            Err(failure) => {
                let (error, _permit) = failure.into_parts();
                self.faulted = true;
                assert(self.well_formed());
                return Err(EngineError::Kv(error));
            }
        };
        let ghost before_reclaim_scheduler = self.scheduler;
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
                }
                assert(self.well_formed());
                Ok(Some(request))
            }
            Err(error) => {
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
        ensures final(self).well_formed(),
    {
        reveal(Engine::well_formed);
        self.require_healthy()?;
        match self.scheduler.dispatch_ready(output) {
            Ok(batch) => Ok(batch),
            Err(error) => Err(EngineError::Scheduler(error)),
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
        ensures final(self).well_formed(),
    {
        reveal(Engine::well_formed);
        if self.faulted {
            return Err(CompletionFailure {
                error: EngineError::Faulted,
                completion: Some(completion),
            });
        }
        let member_count = self.scheduler.pending_batch_member_count();
        assert(member_count <= C);
        if accepted_tokens.len() != member_count {
            return Err(CompletionFailure {
                error: EngineError::CompletionResultCount {
                    expected: member_count,
                    actual: accepted_tokens.len(),
                },
                completion: Some(completion),
            });
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
                index <= member_count,
                member_count <= C,
                accepted_tokens@.len() == member_count,
            decreases member_count - index,
        {
            let request = match self.scheduler.pending_member(index) {
                Some(request) => request,
                None => {
                    self.faulted = true;
                    return Err(CompletionFailure {
                        error: EngineError::InvariantViolation,
                        completion: Some(completion),
                    });
                }
            };
            if self.scheduler.state(request) == Some(RequestState::InFlight) {
                let resident = match self.kv.resident_tokens(request) {
                    Some(tokens) => tokens,
                    None => {
                        self.faulted = true;
                        return Err(CompletionFailure {
                            error: EngineError::InvariantViolation,
                            completion: Some(completion),
                        });
                    }
                };
                let committed = match self.kv.committed_tokens(request) {
                    Some(tokens) => tokens,
                    None => {
                        self.faulted = true;
                        return Err(CompletionFailure {
                            error: EngineError::InvariantViolation,
                            completion: Some(completion),
                        });
                    }
                };
                let tentative = match resident.checked_sub(committed) {
                    Some(tokens) => tokens,
                    None => {
                        self.faulted = true;
                        return Err(CompletionFailure {
                            error: EngineError::InvariantViolation,
                            completion: Some(completion),
                        });
                    }
                };
                if accepted_tokens[index] > tentative {
                    return Err(CompletionFailure {
                        error: EngineError::Kv(KvError::CommitExceedsResident),
                        completion: Some(completion),
                    });
                }
            }
            index += 1;
        }

        let ghost before_completion_scheduler = self.scheduler;
        let completed = match self.scheduler.complete_exact(completion, &mut self.permits) {
            Ok(count) => count,
            Err(error) => {
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
                return Err(CompletionFailure {
                    error: EngineError::Scheduler(error),
                    completion: None,
                });
            }
        };
        assert(completed == member_count);
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

        index = 0;
        while index < completed
            invariant
                self.scheduler.basic_invariant(),
                self.kv.well_formed(),
                self.identity_agreement(),
                0 < C <= MAX_REQUEST_SLOTS,
                self.permits@.len() == C,
                index <= completed,
                completed <= self.permits@.len(),
                completed <= accepted_tokens@.len(),
                forall |position: int| 0 <= position < index ==>
                    self.permits@[position].is_none(),
                forall |position: int| index <= position < completed ==>
                    self.permits@[position].is_some(),
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
            decreases completed - index,
        {
            let ghost step_scheduler = self.scheduler;
            let ghost step_permits = self.permits@;
            assert(self.permits@[index as int].is_some());
            let permit = match self.permits[index].take() {
                Some(permit) => permit,
                None => {
                    self.faulted = true;
                    self.clear_permits();
                    return Err(CompletionFailure {
                        error: EngineError::InvariantViolation,
                        completion: None,
                    });
                }
            };
            let request = permit.request();
            let ghost permit_request = permit.request_spec();
            let ghost permit_origin = permit.origin_spec();
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
                            return Err(CompletionFailure {
                                error: EngineError::Kv(error),
                                completion: None,
                            });
                        }
                    };
                    let ghost finalized_request = finalized.request_spec();
                    assert(finalized_request == request);
                    let accept_result = self.scheduler.accept_finalized(finalized);
                    match accept_result {
                        Err(error) => {
                            self.faulted = true;
                            self.clear_permits();
                            return Err(CompletionFailure {
                                error: EngineError::Scheduler(error),
                                completion: None,
                            });
                        }
                        Ok(()) => {}
                    }
                    assert(before_accept_scheduler == step_scheduler);
                    assert(self.scheduler.detachment_ready_frame_except(
                        &step_scheduler,
                        finalized_request.slot_spec(),
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
                            request.slot_spec(),
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
                            return Err(CompletionFailure {
                                error: EngineError::Kv(error),
                                completion: None,
                            });
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
                            assert(self.scheduler.detachment_ready_frame_except(
                                &step_scheduler,
                                detached_request.slot_spec(),
                            ));
                            assert(detached_request.slot_spec() == request.slot_spec());
                            assert forall |slot: int| 0 <= slot < C implies {
                                &&& self.scheduler.slot_is_live_spec(slot)
                                    == self.kv.request_live_by_slot_spec(slot)
                                &&& self.scheduler.slot_generation_spec(slot)
                                    == self.kv.request_generation_by_slot_spec(slot)
                            } by {
                                if slot == _reclaimed.slot_spec() {
                                    assert(before_reclaim_scheduler.slot_is_live_spec(slot)
                                        == before_release_kv.request_live_by_slot_spec(slot));
                                    assert(before_reclaim_scheduler.slot_generation_spec(slot)
                                        == before_release_kv.request_generation_by_slot_spec(slot));
                                } else {
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
                                    request.slot_spec(),
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
                        }
                        Err(error) => {
                            assert(false);
                            self.faulted = true;
                            return Err(CompletionFailure {
                                error: EngineError::Scheduler(error),
                                completion: None,
                            });
                        }
                    }
                }
                Some(RequestState::Ready | RequestState::Vacant) | None => {
                    self.faulted = true;
                    self.clear_permits();
                    return Err(CompletionFailure {
                        error: EngineError::InvariantViolation,
                        completion: None,
                    });
                }
            }
            index += 1;
        }
        assert(self.well_formed());
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
}
