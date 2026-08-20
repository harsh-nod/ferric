//! Composed request lifecycle and KV ownership.

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
    pub closed spec fn well_formed(&self) -> bool {
        &&& self.scheduler.basic_invariant()
        &&& self.kv.well_formed()
        &&& 0 < C <= MAX_REQUEST_SLOTS
        &&& self.permits@.len() == C
        &&& (forall |index: int| 0 <= index < C ==>
            self.permits@[index].is_none())
        &&& (!self.faulted ==> {
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
        })
    }

    /// Constructs all request, ring, page, and completion-scratch storage.
    pub fn new(
        page_count: u32,
        page_tokens: u32,
        max_context_tokens: u32,
    ) -> Result<Self, EngineError> {
        let scheduler = match Scheduler::<C>::new() {
            Ok(scheduler) => scheduler,
            Err(error) => return Err(EngineError::Scheduler(error)),
        };
        let kv = match KvPool::new(page_count, page_tokens, max_context_tokens) {
            Ok(kv) => kv,
            Err(error) => return Err(EngineError::Kv(error)),
        };
        let mut permits = Vec::with_capacity(C);
        let mut index = 0;
        while index < C
            decreases C - index,
        {
            permits.push(None);
            index += 1;
        }
        Ok(Self {
            scheduler,
            kv,
            permits,
            faulted: false,
        })
    }

    #[must_use]
    pub const fn is_faulted(&self) -> bool {
        self.faulted
    }

    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.scheduler.capacity()
    }

    #[must_use]
    pub const fn live_count(&self) -> usize {
        self.scheduler.live_count()
    }

    #[must_use]
    pub const fn completed_epoch(&self) -> CompletionEpoch {
        self.scheduler.completed_epoch()
    }

    pub fn admit(&mut self) -> Result<RequestId, EngineError>
        requires old(self).well_formed(),
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

    pub fn append_tentative(
        &mut self,
        request: RequestId,
        token_count: u32,
    ) -> Result<(), EngineError>
        requires old(self).well_formed(),
    {
        reveal(Engine::well_formed);
        self.require_healthy()?;
        match self.kv.append_tentative(request, token_count) {
            Ok(()) => Ok(()),
            Err(error) => Err(EngineError::Kv(error)),
        }
    }

    pub fn share_committed_prefix(
        &mut self,
        source: RequestId,
        target: RequestId,
        token_count: u32,
    ) -> Result<(), EngineError>
        requires old(self).well_formed(),
    {
        reveal(Engine::well_formed);
        self.require_healthy()?;
        match self
            .kv
            .share_committed_prefix(source, target, token_count)
        {
            Ok(()) => Ok(()),
            Err(error) => Err(EngineError::Kv(error)),
        }
    }

    pub fn validate_read(
        &self,
        request: RequestId,
        logical_offset: u32,
        span: u32,
    ) -> Result<(), EngineError>
        requires self.well_formed(),
    {
        reveal(Engine::well_formed);
        self.require_healthy()?;
        match self.kv.validate_read(request, logical_offset, span) {
            Ok(()) => Ok(()),
            Err(error) => Err(EngineError::Kv(error)),
        }
    }

    pub fn retire(&mut self, request: RequestId) -> Result<(), EngineError>
        requires old(self).well_formed(),
    {
        reveal(Engine::well_formed);
        self.require_healthy()?;
        match self.scheduler.retire(request) {
            Ok(()) => Ok(()),
            Err(error) => Err(EngineError::Scheduler(error)),
        }
    }

    pub fn reclaim_one(&mut self) -> Result<Option<RequestId>, EngineError>
        requires old(self).well_formed(),
    {
        reveal(Engine::well_formed);
        self.require_healthy()?;
        let permit_result = match self.scheduler.take_retiring_permit() {
            Ok(result) => result,
            Err(error) => return Err(EngineError::Scheduler(error)),
        };
        let permit = match permit_result {
            Some(permit) => permit,
            None => return Ok(None),
        };
        let request = permit.request();
        let detached = match self.kv.release_request(request, permit) {
            Ok(detached) => detached,
            Err(failure) => {
                let (error, _permit) = failure.into_parts();
                self.faulted = true;
                return Err(EngineError::Kv(error));
            }
        };
        match self.scheduler.reclaim_detached(detached) {
            Ok(request) => Ok(Some(request)),
            Err(error) => {
                self.faulted = true;
                Err(EngineError::Scheduler(error))
            }
        }
    }

    pub fn dispatch_ready(
        &mut self,
        output: &mut [RequestId],
    ) -> Result<Option<DispatchBatch>, EngineError>
        requires old(self).well_formed(),
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
    pub fn complete_exact(
        &mut self,
        completion: ExactCompletion,
        accepted_tokens: &[u32],
    ) -> Result<usize, CompletionFailure>
        requires old(self).well_formed(),
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

        let completed = match self.scheduler.complete_exact(completion, &mut self.permits) {
            Ok(count) => count,
            Err(error) => {
                return Err(CompletionFailure {
                    error: EngineError::Scheduler(error),
                    completion: None,
                });
            }
        };
        assert(completed == member_count);
        assert(completed <= self.permits@.len());

        index = 0;
        while index < completed
            invariant
                self.scheduler.basic_invariant(),
                self.kv.well_formed(),
                index <= completed,
                completed <= self.permits@.len(),
                completed <= accepted_tokens@.len(),
            decreases completed - index,
        {
            let permit = match self.permits[index].take() {
                Some(permit) => permit,
                None => {
                    self.faulted = true;
                    return Err(CompletionFailure {
                        error: EngineError::InvariantViolation,
                        completion: None,
                    });
                }
            };
            let request = permit.request();
            match self.scheduler.state(request) {
                Some(RequestState::InFlight) => {
                    let finalized = match self.kv.finalize_tentative(
                        request,
                        accepted_tokens[index],
                        permit,
                    ) {
                        Ok(finalized) => finalized,
                        Err(failure) => {
                            let (error, _permit) = failure.into_parts();
                            self.faulted = true;
                            return Err(CompletionFailure {
                                error: EngineError::Kv(error),
                                completion: None,
                            });
                        }
                    };
                    if let Err(error) = self.scheduler.accept_finalized(finalized) {
                        self.faulted = true;
                        return Err(CompletionFailure {
                            error: EngineError::Scheduler(error),
                            completion: None,
                        });
                    }
                }
                Some(RequestState::Retiring) => {
                    let detached = match self.kv.release_request(request, permit) {
                        Ok(detached) => detached,
                        Err(failure) => {
                            let (error, _permit) = failure.into_parts();
                            self.faulted = true;
                            return Err(CompletionFailure {
                                error: EngineError::Kv(error),
                                completion: None,
                            });
                        }
                    };
                    if let Err(error) = self.scheduler.reclaim_detached(detached) {
                        self.faulted = true;
                        return Err(CompletionFailure {
                            error: EngineError::Scheduler(error),
                            completion: None,
                        });
                    }
                }
                Some(RequestState::Ready) | Some(RequestState::Vacant) | None => {
                    self.faulted = true;
                    return Err(CompletionFailure {
                        error: EngineError::InvariantViolation,
                        completion: None,
                    });
                }
            }
            index += 1;
        }
        Ok(completed)
    }

    #[must_use]
    pub fn state(&self, request: RequestId) -> Option<RequestState> {
        self.scheduler.state(request)
    }

    #[must_use]
    pub fn resident_tokens(&self, request: RequestId) -> Option<u32>
        requires self.well_formed(),
    {
        reveal(Engine::well_formed);
        self.kv.resident_tokens(request)
    }

    #[must_use]
    pub fn committed_tokens(&self, request: RequestId) -> Option<u32>
        requires self.well_formed(),
    {
        reveal(Engine::well_formed);
        self.kv.committed_tokens(request)
    }

    #[must_use]
    pub fn free_pages(&self) -> u32 {
        self.kv.free_pages()
    }

    fn require_healthy(&self) -> Result<(), EngineError> {
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
