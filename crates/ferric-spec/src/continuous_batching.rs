//! Logical continuous-batching and request-isolation contract for M1.
//!
//! This module contains no queue, GPU, allocation, KV-page, or timing model.
//! It lifts the sequential request lifecycle into a fixed 32-slot state and
//! proves that one interleaved global action changes exactly one generational
//! request. Physical runners must separately refine this relation.

use crate::completion::CompletionEpoch;
use crate::scheduling::{
    apply_request_transition, LifecyclePhase, RequestState, RequestTransition, SequentialRequest,
    TransitionError,
};
use crate::{RequestId, M1_MAX_ACTIVE_SEQUENCES, M1_MAX_COMPLETION_TOKENS};
use vstd::prelude::*;

verus! {

/// Fixed M1 scheduler capacity. It intentionally matches the admitted limit.
pub const M1_CONTINUOUS_BATCH_CAPACITY: usize = M1_MAX_ACTIVE_SEQUENCES as usize;

/// Per-request logical state enriched with exact generation and publication data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContinuousRequest {
    generation: u32,
    lifecycle: SequentialRequest,
    active_epoch: CompletionEpoch,
    has_active_epoch: bool,
    published_for_active_epoch: bool,
    published_token_count: u64,
}

impl ContinuousRequest {
    const INITIAL: Self = Self {
        generation: 1,
        lifecycle: SequentialRequest {
            state: RequestState::Vacant,
            phase: LifecyclePhase::Idle,
        },
        active_epoch: CompletionEpoch { value: 0 },
        has_active_epoch: false,
        published_for_active_epoch: false,
        published_token_count: 0,
    };

    pub closed spec fn valid(self) -> bool {
        self.generation > 0
            && match (self.lifecycle.state, self.lifecycle.phase) {
                (RequestState::Vacant, LifecyclePhase::Idle)
                | (RequestState::Ready, LifecyclePhase::Idle)
                | (RequestState::Retiring, LifecyclePhase::RetiringQuiescent) => {
                    !self.has_active_epoch && !self.published_for_active_epoch
                }
                (RequestState::InFlight, LifecyclePhase::Executing)
                | (RequestState::Retiring, LifecyclePhase::RetiringExecuting) => {
                    self.has_active_epoch
                        && self.active_epoch.value > 0
                        && !self.published_for_active_epoch
                }
                (RequestState::InFlight, LifecyclePhase::AwaitingKv) => {
                    self.has_active_epoch && self.active_epoch.value > 0
                }
                _ => false,
            }
    }

    pub closed spec fn generation_spec(self) -> u32 {
        self.generation
    }

    pub closed spec fn lifecycle_spec(self) -> SequentialRequest {
        self.lifecycle
    }

    pub closed spec fn active_epoch_spec(self) -> Option<CompletionEpoch> {
        if self.has_active_epoch {
            Some(self.active_epoch)
        } else {
            None
        }
    }

    pub closed spec fn published_for_active_epoch_spec(self) -> bool {
        self.published_for_active_epoch
    }

    pub closed spec fn published_token_count_spec(self) -> u64 {
        self.published_token_count
    }

    #[must_use]
    pub const fn generation(self) -> (generation: u32)
        ensures generation == self.generation_spec(),
    {
        self.generation
    }

    #[must_use]
    pub const fn lifecycle(self) -> (lifecycle: SequentialRequest)
        ensures lifecycle == self.lifecycle_spec(),
    {
        self.lifecycle
    }

    #[must_use]
    pub const fn active_epoch(self) -> (epoch: Option<CompletionEpoch>)
        ensures epoch == self.active_epoch_spec(),
    {
        if self.has_active_epoch {
            Some(self.active_epoch)
        } else {
            None
        }
    }

    #[must_use]
    pub const fn published_for_active_epoch(self) -> (published: bool)
        ensures published == self.published_for_active_epoch_spec(),
    {
        self.published_for_active_epoch
    }

    #[must_use]
    pub const fn published_token_count(self) -> (count: u64)
        ensures count == self.published_token_count_spec(),
    {
        self.published_token_count
    }
}

/// One request-local action admitted into an interleaved batch trace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContinuousBatchAction {
    Admit,
    Dispatch { epoch: CompletionEpoch },
    Retire,
    CompleteExact { epoch: CompletionEpoch },
    Publish { epoch: CompletionEpoch, emitted_tokens: u8 },
    FinalizeKv,
    DetachKv,
}

/// Fail-closed rejection from the logical continuous-batch oracle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContinuousBatchError {
    SlotOutOfRange,
    StaleGeneration,
    Lifecycle(TransitionError),
    EpochMissing,
    EpochAlreadyActive,
    EpochMismatch,
    PublicationNotReady,
    AlreadyPublished,
    InvalidTokenCount,
    TokenCountExhausted,
    GenerationExhausted,
}

pub closed spec fn publication_ready(current: ContinuousRequest) -> bool {
    match (current.lifecycle.state, current.lifecycle.phase) {
        (RequestState::InFlight, LifecyclePhase::AwaitingKv) => true,
        _ => false,
    }
}

pub closed spec fn valid_publication_token_count(emitted_tokens: u8) -> bool {
    0 < emitted_tokens as int <= M1_MAX_COMPLETION_TOKENS
}

pub closed spec fn retiring_quiescent(lifecycle: SequentialRequest) -> bool {
    match lifecycle.phase {
        LifecyclePhase::RetiringQuiescent => true,
        _ => false,
    }
}

pub closed spec fn continuous_publish_step(
    current: ContinuousRequest,
    epoch: CompletionEpoch,
    emitted_tokens: u8,
) -> Result<ContinuousRequest, ContinuousBatchError> {
    if !current.has_active_epoch {
        Err(ContinuousBatchError::EpochMissing)
    } else if current.active_epoch.value != epoch.value {
        Err(ContinuousBatchError::EpochMismatch)
    } else if !publication_ready(current) {
        Err(ContinuousBatchError::PublicationNotReady)
    } else if current.published_for_active_epoch {
        Err(ContinuousBatchError::AlreadyPublished)
    } else if !valid_publication_token_count(emitted_tokens) {
        Err(ContinuousBatchError::InvalidTokenCount)
    } else if current.published_token_count > u64::MAX - emitted_tokens as u64 {
        Err(ContinuousBatchError::TokenCountExhausted)
    } else {
        Ok(ContinuousRequest {
            published_for_active_epoch: true,
            published_token_count: (current.published_token_count + emitted_tokens as u64) as u64,
            ..current
        })
    }
}

/// Fixed-capacity interleaved state. Slot position is the request slot identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContinuousBatch {
    slots: [ContinuousRequest; M1_CONTINUOUS_BATCH_CAPACITY],
}

impl ContinuousBatch {
    /// Constructs the only initial state: 32 vacant generation-one requests.
    #[must_use]
    pub const fn initial() -> (batch: Self)
        ensures batch.valid(),
    {
        Self { slots: [ContinuousRequest::INITIAL; M1_CONTINUOUS_BATCH_CAPACITY] }
    }

    pub closed spec fn slots_spec(&self) -> Seq<ContinuousRequest> {
        self.slots@
    }

    pub closed spec fn valid(&self) -> bool {
        self.slots_spec().len() == M1_CONTINUOUS_BATCH_CAPACITY
            && forall|slot: int| 0 <= slot < self.slots_spec().len()
                ==> self.slots_spec()[slot].valid()
    }

    /// Returns a slot only for its exact live generation.
    #[must_use]
    pub fn request(&self, request: RequestId) -> (result: Option<ContinuousRequest>)
        ensures
            result == if request.slot_spec() < M1_CONTINUOUS_BATCH_CAPACITY
                && self.slots_spec()[request.slot_spec() as int].generation_spec()
                    == request.generation_spec()
            {
                Some(self.slots_spec()[request.slot_spec() as int])
            } else {
                None
            },
    {
        let slot = request.slot() as usize;
        if slot >= M1_CONTINUOUS_BATCH_CAPACITY {
            return None;
        }
        let entry = self.slots[slot];
        if entry.generation != request.generation() {
            return None;
        }
        Some(entry)
    }

    fn replace(&mut self, slot: usize, request: ContinuousRequest)
        requires slot < M1_CONTINUOUS_BATCH_CAPACITY,
        ensures
            final(self).slots_spec() == old(self).slots_spec().update(slot as int, request),
    {
        self.slots[slot] = request;
    }
}

/// Crate-local bridge exposing the validity carried by a fixed batch slot.
pub(crate) proof fn valid_continuous_batch_slot(
    batch: &ContinuousBatch,
    slot: int,
)
    requires
        batch.valid(),
        0 <= slot < M1_CONTINUOUS_BATCH_CAPACITY,
    ensures batch.slots_spec()[slot].valid(),
{
}

/// Crate-local bridge from a valid active scheduler request to its positive epoch.
pub(crate) proof fn valid_continuous_active_epoch(
    current: ContinuousRequest,
    epoch: CompletionEpoch,
)
    requires
        current.valid(),
        current.active_epoch_spec() == Some(epoch),
    ensures epoch.value > 0,
{
}

fn lifecycle_result(
    current: ContinuousRequest,
    transition: RequestTransition,
) -> (result: Result<SequentialRequest, ContinuousBatchError>)
    ensures
        result == match crate::scheduling::request_transition(current.lifecycle, transition) {
            Ok(next) => Ok(next),
            Err(error) => Err(ContinuousBatchError::Lifecycle(error)),
        },
{
    match apply_request_transition(current.lifecycle, transition) {
        Ok(next) => Ok(next),
        Err(error) => Err(ContinuousBatchError::Lifecycle(error)),
    }
}

/// Exact request-local action relation.
pub closed spec fn continuous_request_step(
    current: ContinuousRequest,
    action: ContinuousBatchAction,
) -> Result<ContinuousRequest, ContinuousBatchError> {
    match action {
        ContinuousBatchAction::Admit => {
            match crate::scheduling::request_transition(current.lifecycle, RequestTransition::Admit) {
                Ok(lifecycle) => Ok(ContinuousRequest {
                    lifecycle,
                    ..current
                }),
                Err(error) => Err(ContinuousBatchError::Lifecycle(error)),
            }
        }
        ContinuousBatchAction::Dispatch { epoch } => {
            if epoch.value == 0 {
                Err(ContinuousBatchError::EpochMissing)
            } else if current.has_active_epoch {
                Err(ContinuousBatchError::EpochAlreadyActive)
            } else {
                match crate::scheduling::request_transition(current.lifecycle, RequestTransition::Dispatch) {
                    Ok(lifecycle) => Ok(ContinuousRequest {
                        lifecycle,
                        active_epoch: epoch,
                        has_active_epoch: true,
                        published_for_active_epoch: false,
                        ..current
                    }),
                    Err(error) => Err(ContinuousBatchError::Lifecycle(error)),
                }
            }
        }
        ContinuousBatchAction::Retire => {
            match crate::scheduling::request_transition(current.lifecycle, RequestTransition::Retire) {
                Ok(lifecycle) => {
                    if retiring_quiescent(lifecycle) {
                        Ok(ContinuousRequest {
                            lifecycle,
                            active_epoch: CompletionEpoch { value: 0 },
                            has_active_epoch: false,
                            published_for_active_epoch: false,
                            ..current
                        })
                    } else {
                        Ok(ContinuousRequest { lifecycle, ..current })
                    }
                }
                Err(error) => Err(ContinuousBatchError::Lifecycle(error)),
            }
        }
        ContinuousBatchAction::CompleteExact { epoch } => {
            if !current.has_active_epoch {
                Err(ContinuousBatchError::EpochMissing)
            } else if current.active_epoch.value != epoch.value {
                Err(ContinuousBatchError::EpochMismatch)
            } else {
                match crate::scheduling::request_transition(current.lifecycle, RequestTransition::CompleteExact) {
                    Ok(lifecycle) => {
                        if retiring_quiescent(lifecycle) {
                            Ok(ContinuousRequest {
                                lifecycle,
                                active_epoch: CompletionEpoch { value: 0 },
                                has_active_epoch: false,
                                published_for_active_epoch: false,
                                ..current
                            })
                        } else {
                            Ok(ContinuousRequest { lifecycle, ..current })
                        }
                    }
                    Err(error) => Err(ContinuousBatchError::Lifecycle(error)),
                }
            }
        }
        ContinuousBatchAction::Publish { epoch, emitted_tokens } => {
            continuous_publish_step(current, epoch, emitted_tokens)
        }
        ContinuousBatchAction::FinalizeKv => {
            if !current.published_for_active_epoch {
                Err(ContinuousBatchError::PublicationNotReady)
            } else {
                match crate::scheduling::request_transition(current.lifecycle, RequestTransition::FinalizeKv) {
                    Ok(lifecycle) => Ok(ContinuousRequest {
                        lifecycle,
                        active_epoch: CompletionEpoch { value: 0 },
                        has_active_epoch: false,
                        published_for_active_epoch: false,
                        ..current
                    }),
                    Err(error) => Err(ContinuousBatchError::Lifecycle(error)),
                }
            }
        }
        ContinuousBatchAction::DetachKv => {
            if current.has_active_epoch {
                Err(ContinuousBatchError::EpochAlreadyActive)
            } else if current.generation == u32::MAX {
                Err(ContinuousBatchError::GenerationExhausted)
            } else {
                match crate::scheduling::request_transition(current.lifecycle, RequestTransition::DetachKv) {
                    Ok(lifecycle) => Ok(ContinuousRequest {
                        generation: (current.generation + 1) as u32,
                        lifecycle,
                        active_epoch: CompletionEpoch { value: 0 },
                        has_active_epoch: false,
                        published_for_active_epoch: false,
                        published_token_count: 0,
                    }),
                    Err(error) => Err(ContinuousBatchError::Lifecycle(error)),
                }
            }
        }
    }
}

fn is_publication_ready(current: ContinuousRequest) -> (ready: bool)
    ensures ready == publication_ready(current),
{
    proof {
        reveal(publication_ready);
    }
    matches!(
        (current.lifecycle.state, current.lifecycle.phase),
        (RequestState::InFlight, LifecyclePhase::AwaitingKv)
    )
}

fn is_valid_publication_token_count(emitted_tokens: u8) -> (valid: bool)
    ensures valid == valid_publication_token_count(emitted_tokens),
{
    proof {
        reveal(valid_publication_token_count);
    }
    emitted_tokens > 0 && emitted_tokens as usize <= M1_MAX_COMPLETION_TOKENS
}

fn is_retiring_quiescent(lifecycle: SequentialRequest) -> (quiescent: bool)
    ensures quiescent == retiring_quiescent(lifecycle),
{
    proof {
        reveal(retiring_quiescent);
    }
    matches!(lifecycle.phase, LifecyclePhase::RetiringQuiescent)
}

pub(crate) fn apply_continuous_publish_step(
    current: ContinuousRequest,
    epoch: CompletionEpoch,
    emitted_tokens: u8,
) -> (result: Result<ContinuousRequest, ContinuousBatchError>)
    ensures result == continuous_publish_step(current, epoch, emitted_tokens),
{
    proof {
        reveal(continuous_publish_step);
    }
    if !current.has_active_epoch {
        return Err(ContinuousBatchError::EpochMissing);
    }
    if current.active_epoch.value != epoch.value {
        return Err(ContinuousBatchError::EpochMismatch);
    }
    if !is_publication_ready(current) {
        return Err(ContinuousBatchError::PublicationNotReady);
    }
    if current.published_for_active_epoch {
        return Err(ContinuousBatchError::AlreadyPublished);
    }
    if !is_valid_publication_token_count(emitted_tokens) {
        return Err(ContinuousBatchError::InvalidTokenCount);
    }
    if current.published_token_count > u64::MAX - u64::from(emitted_tokens) {
        return Err(ContinuousBatchError::TokenCountExhausted);
    }
    let published_token_count = current.published_token_count + u64::from(emitted_tokens);
    Ok(ContinuousRequest {
        published_for_active_epoch: true,
        published_token_count,
        ..current
    })
}

fn apply_continuous_request_step(
    current: ContinuousRequest,
    action: ContinuousBatchAction,
) -> (result: Result<ContinuousRequest, ContinuousBatchError>)
    ensures result == continuous_request_step(current, action),
{
    proof {
        reveal(continuous_request_step);
    }
    match action {
        ContinuousBatchAction::Admit => {
            let lifecycle = lifecycle_result(current, RequestTransition::Admit)?;
            let result = Ok(ContinuousRequest { lifecycle, ..current });
            assert(result == continuous_request_step(current, action));
            result
        }
        ContinuousBatchAction::Dispatch { epoch } => {
            if epoch.value == 0 {
                return Err(ContinuousBatchError::EpochMissing);
            }
            if current.has_active_epoch {
                return Err(ContinuousBatchError::EpochAlreadyActive);
            }
            let lifecycle = lifecycle_result(current, RequestTransition::Dispatch)?;
            let result = Ok(ContinuousRequest {
                lifecycle,
                active_epoch: epoch,
                has_active_epoch: true,
                published_for_active_epoch: false,
                ..current
            });
            assert(result == continuous_request_step(current, action));
            result
        }
        ContinuousBatchAction::Retire => {
            let lifecycle = lifecycle_result(current, RequestTransition::Retire)?;
            let result = if is_retiring_quiescent(lifecycle) {
                Ok(ContinuousRequest {
                    lifecycle,
                    active_epoch: CompletionEpoch { value: 0 },
                    has_active_epoch: false,
                    published_for_active_epoch: false,
                    ..current
                })
            } else {
                Ok(ContinuousRequest { lifecycle, ..current })
            };
            assert(result == continuous_request_step(current, action));
            result
        }
        ContinuousBatchAction::CompleteExact { epoch } => {
            if !current.has_active_epoch {
                return Err(ContinuousBatchError::EpochMissing);
            }
            if current.active_epoch.value != epoch.value {
                return Err(ContinuousBatchError::EpochMismatch);
            }
            let lifecycle = lifecycle_result(current, RequestTransition::CompleteExact)?;
            let result = if is_retiring_quiescent(lifecycle) {
                Ok(ContinuousRequest {
                    lifecycle,
                    active_epoch: CompletionEpoch { value: 0 },
                    has_active_epoch: false,
                    published_for_active_epoch: false,
                    ..current
                })
            } else {
                Ok(ContinuousRequest { lifecycle, ..current })
            };
            assert(result == continuous_request_step(current, action));
            result
        }
        ContinuousBatchAction::Publish { epoch, emitted_tokens } => {
            let result = apply_continuous_publish_step(current, epoch, emitted_tokens);
            assert(result == continuous_request_step(current, action));
            result
        }
        ContinuousBatchAction::FinalizeKv => {
            if !current.published_for_active_epoch {
                return Err(ContinuousBatchError::PublicationNotReady);
            }
            let lifecycle = lifecycle_result(current, RequestTransition::FinalizeKv)?;
            let result = Ok(ContinuousRequest {
                lifecycle,
                active_epoch: CompletionEpoch { value: 0 },
                has_active_epoch: false,
                published_for_active_epoch: false,
                ..current
            });
            assert(result == continuous_request_step(current, action));
            result
        }
        ContinuousBatchAction::DetachKv => {
            if current.has_active_epoch {
                return Err(ContinuousBatchError::EpochAlreadyActive);
            }
            let Some(generation) = current.generation.checked_add(1) else {
                return Err(ContinuousBatchError::GenerationExhausted);
            };
            let lifecycle = lifecycle_result(current, RequestTransition::DetachKv)?;
            let result = Ok(ContinuousRequest {
                generation,
                lifecycle,
                active_epoch: CompletionEpoch { value: 0 },
                has_active_epoch: false,
                published_for_active_epoch: false,
                published_token_count: 0,
            });
            assert(result == continuous_request_step(current, action));
            result
        }
    }
}

/// Exact successful interleaved transition: one local step and a full frame.
pub closed spec fn continuous_batch_step_refines(
    before: &ContinuousBatch,
    after: &ContinuousBatch,
    request: RequestId,
    action: ContinuousBatchAction,
) -> bool {
    &&& request.slot_spec() < M1_CONTINUOUS_BATCH_CAPACITY
    &&& before.slots_spec()[request.slot_spec() as int].generation
        == request.generation_spec()
    &&& continuous_request_step(
        before.slots_spec()[request.slot_spec() as int],
        action,
    ).is_ok()
    &&& after.slots_spec()[request.slot_spec() as int]
        == continuous_request_step(
            before.slots_spec()[request.slot_spec() as int],
            action,
        ).unwrap()
    &&& forall|slot: int| 0 <= slot < before.slots_spec().len()
        && slot != request.slot_spec() ==> after.slots_spec()[slot] == before.slots_spec()[slot]
}

pub closed spec fn continuous_batch_expected_error(
    before: &ContinuousBatch,
    request: RequestId,
    action: ContinuousBatchAction,
) -> Option<ContinuousBatchError> {
    if request.slot_spec() >= M1_CONTINUOUS_BATCH_CAPACITY {
        Some(ContinuousBatchError::SlotOutOfRange)
    } else if before.slots_spec()[request.slot_spec() as int].generation
        != request.generation_spec()
    {
        Some(ContinuousBatchError::StaleGeneration)
    } else {
        match continuous_request_step(
            before.slots_spec()[request.slot_spec() as int],
            action,
        ) {
            Ok(_) => None,
            Err(error) => Some(error),
        }
    }
}

/// Applies one request-local action to the fixed interleaved state.
///
/// # Errors
///
/// Rejects an out-of-range or stale request, or the exact disabled local
/// transition. Every rejection leaves the entire batch unchanged.
pub fn apply_continuous_batch_step(
    batch: &mut ContinuousBatch,
    request: RequestId,
    action: ContinuousBatchAction,
) -> (result: Result<(), ContinuousBatchError>)
    requires old(batch).valid(),
    ensures
        final(batch).valid(),
        match result {
            Ok(()) => {
                continuous_batch_expected_error(old(batch), request, action).is_none()
                    && continuous_batch_step_refines(old(batch), final(batch), request, action)
            }
            Err(error) => {
                continuous_batch_expected_error(old(batch), request, action) == Some(error)
                    && *final(batch) == *old(batch)
            }
        },
{
    let slot = request.slot() as usize;
    if slot >= M1_CONTINUOUS_BATCH_CAPACITY {
        return Err(ContinuousBatchError::SlotOutOfRange);
    }
    let current = batch.slots[slot];
    if current.generation != request.generation() {
        return Err(ContinuousBatchError::StaleGeneration);
    }
    let updated = apply_continuous_request_step(current, action)?;
    batch.replace(slot, updated);
    proof {
        reveal(continuous_batch_step_refines);
        reveal(continuous_batch_expected_error);
    }
    Ok(())
}

/// A successful step has exact selected-request projection.
pub proof fn successful_batch_step_projects_to_sequential_request(
    before: &ContinuousBatch,
    after: &ContinuousBatch,
    request: RequestId,
    action: ContinuousBatchAction,
)
    requires continuous_batch_step_refines(before, after, request, action),
    ensures
        after.slots_spec()[request.slot_spec() as int]
            == continuous_request_step(
                before.slots_spec()[request.slot_spec() as int],
                action,
            ).unwrap(),
{
    reveal(continuous_batch_step_refines);
}

/// A successful step cannot change any different request slot.
pub proof fn successful_batch_step_preserves_other_request(
    before: &ContinuousBatch,
    after: &ContinuousBatch,
    request: RequestId,
    action: ContinuousBatchAction,
    other_slot: int,
)
    requires
        continuous_batch_step_refines(before, after, request, action),
        0 <= other_slot < M1_CONTINUOUS_BATCH_CAPACITY,
        other_slot != request.slot_spec(),
    ensures after.slots_spec()[other_slot] == before.slots_spec()[other_slot],
{
    reveal(continuous_batch_step_refines);
}

/// Exact finite trace relation for an interleaving of independently routed requests.
pub open spec fn interleaved_batch_trace_refines(
    states: Seq<ContinuousBatch>,
    requests: Seq<RequestId>,
    actions: Seq<ContinuousBatchAction>,
) -> bool {
    states.len() == requests.len() + 1
        && actions.len() == requests.len()
        && forall|step: int| 0 <= step < requests.len() ==>
            continuous_batch_step_refines(
                &states[step],
                &states[step + 1],
                requests[step],
                actions[step],
            )
}

/// An interleaved trace containing no action for `other_slot` preserves it exactly.
pub proof fn interleaved_trace_preserves_unselected_request(
    states: Seq<ContinuousBatch>,
    requests: Seq<RequestId>,
    actions: Seq<ContinuousBatchAction>,
    other_slot: int,
)
    requires
        interleaved_batch_trace_refines(states, requests, actions),
        0 <= other_slot < M1_CONTINUOUS_BATCH_CAPACITY,
        forall|step: int| 0 <= step < requests.len()
            ==> requests[step].slot_spec() != other_slot,
    ensures
        states.last().slots_spec()[other_slot]
            == states.first().slots_spec()[other_slot],
    decreases requests.len(),
{
    reveal(interleaved_batch_trace_refines);
    if requests.len() > 0 {
        let tail_states = states.subrange(1, states.len() as int);
        let tail_requests = requests.subrange(1, requests.len() as int);
        let tail_actions = actions.subrange(1, actions.len() as int);
        assert(interleaved_batch_trace_refines(tail_states, tail_requests, tail_actions)) by {
            reveal(interleaved_batch_trace_refines);
        }
        successful_batch_step_preserves_other_request(
            &states[0],
            &states[1],
            requests[0],
            actions[0],
            other_slot,
        );
        interleaved_trace_preserves_unselected_request(
            tail_states,
            tail_requests,
            tail_actions,
            other_slot,
        );
    }
}

}

#[cfg(test)]
mod tests {
    use super::{
        apply_continuous_batch_step, ContinuousBatch, ContinuousBatchAction, ContinuousBatchError,
    };
    use crate::completion::CompletionEpoch;
    use crate::scheduling::{LifecyclePhase, RequestState, TransitionError};
    use crate::RequestId;

    #[test]
    fn two_requests_interleave_without_cross_request_changes() {
        let first = RequestId::new(0, 1);
        let second = RequestId::new(31, 1);
        let mut batch = ContinuousBatch::initial();

        apply_continuous_batch_step(&mut batch, first, ContinuousBatchAction::Admit).unwrap();
        apply_continuous_batch_step(&mut batch, second, ContinuousBatchAction::Admit).unwrap();
        apply_continuous_batch_step(
            &mut batch,
            first,
            ContinuousBatchAction::Dispatch {
                epoch: CompletionEpoch::new(7),
            },
        )
        .unwrap();
        apply_continuous_batch_step(
            &mut batch,
            second,
            ContinuousBatchAction::Dispatch {
                epoch: CompletionEpoch::new(8),
            },
        )
        .unwrap();

        assert_eq!(
            batch.request(first).unwrap().lifecycle().state,
            RequestState::InFlight
        );
        assert_eq!(
            batch.request(second).unwrap().active_epoch(),
            Some(CompletionEpoch::new(8))
        );

        apply_continuous_batch_step(
            &mut batch,
            first,
            ContinuousBatchAction::CompleteExact {
                epoch: CompletionEpoch::new(7),
            },
        )
        .unwrap();
        apply_continuous_batch_step(
            &mut batch,
            first,
            ContinuousBatchAction::Publish {
                epoch: CompletionEpoch::new(7),
                emitted_tokens: 1,
            },
        )
        .unwrap();
        apply_continuous_batch_step(&mut batch, first, ContinuousBatchAction::FinalizeKv).unwrap();

        assert_eq!(
            batch.request(first).unwrap().lifecycle().state,
            RequestState::Ready
        );
        assert_eq!(batch.request(first).unwrap().published_token_count(), 1);
        assert_eq!(
            batch.request(second).unwrap().lifecycle().phase,
            LifecyclePhase::Executing
        );
    }

    #[test]
    fn cancellation_waits_for_exact_completion_and_never_resurrects() {
        let request = RequestId::new(4, 1);
        let mut batch = ContinuousBatch::initial();
        apply_continuous_batch_step(&mut batch, request, ContinuousBatchAction::Admit).unwrap();
        apply_continuous_batch_step(
            &mut batch,
            request,
            ContinuousBatchAction::Dispatch {
                epoch: CompletionEpoch::new(3),
            },
        )
        .unwrap();
        apply_continuous_batch_step(&mut batch, request, ContinuousBatchAction::Retire).unwrap();
        assert_eq!(
            batch.request(request).unwrap().lifecycle().phase,
            LifecyclePhase::RetiringExecuting
        );
        assert_eq!(
            apply_continuous_batch_step(
                &mut batch,
                request,
                ContinuousBatchAction::CompleteExact {
                    epoch: CompletionEpoch::new(4)
                },
            ),
            Err(ContinuousBatchError::EpochMismatch)
        );
        apply_continuous_batch_step(
            &mut batch,
            request,
            ContinuousBatchAction::CompleteExact {
                epoch: CompletionEpoch::new(3),
            },
        )
        .unwrap();
        assert_eq!(
            batch.request(request).unwrap().lifecycle().phase,
            LifecyclePhase::RetiringQuiescent
        );
        assert_eq!(
            apply_continuous_batch_step(
                &mut batch,
                request,
                ContinuousBatchAction::Publish {
                    epoch: CompletionEpoch::new(3),
                    emitted_tokens: 1,
                },
            ),
            Err(ContinuousBatchError::EpochMissing)
        );
    }

    #[test]
    fn detach_advances_generation_and_rejects_stale_identity() {
        let stale = RequestId::new(9, 1);
        let next = RequestId::new(9, 2);
        let mut batch = ContinuousBatch::initial();
        apply_continuous_batch_step(&mut batch, stale, ContinuousBatchAction::Admit).unwrap();
        apply_continuous_batch_step(&mut batch, stale, ContinuousBatchAction::Retire).unwrap();
        apply_continuous_batch_step(&mut batch, stale, ContinuousBatchAction::DetachKv).unwrap();

        assert_eq!(batch.request(stale), None);
        assert_eq!(
            batch.request(next).unwrap().lifecycle().state,
            RequestState::Vacant
        );
        assert_eq!(
            apply_continuous_batch_step(&mut batch, stale, ContinuousBatchAction::Admit),
            Err(ContinuousBatchError::StaleGeneration)
        );
        apply_continuous_batch_step(&mut batch, next, ContinuousBatchAction::Admit).unwrap();
    }

    #[test]
    fn publication_is_exactly_once_and_bounded() {
        let request = RequestId::new(2, 1);
        let mut batch = ContinuousBatch::initial();
        apply_continuous_batch_step(&mut batch, request, ContinuousBatchAction::Admit).unwrap();
        apply_continuous_batch_step(
            &mut batch,
            request,
            ContinuousBatchAction::Dispatch {
                epoch: CompletionEpoch::new(10),
            },
        )
        .unwrap();
        apply_continuous_batch_step(
            &mut batch,
            request,
            ContinuousBatchAction::CompleteExact {
                epoch: CompletionEpoch::new(10),
            },
        )
        .unwrap();
        assert_eq!(
            apply_continuous_batch_step(
                &mut batch,
                request,
                ContinuousBatchAction::Publish {
                    epoch: CompletionEpoch::new(10),
                    emitted_tokens: 0,
                },
            ),
            Err(ContinuousBatchError::InvalidTokenCount)
        );
        apply_continuous_batch_step(
            &mut batch,
            request,
            ContinuousBatchAction::Publish {
                epoch: CompletionEpoch::new(10),
                emitted_tokens: 17,
            },
        )
        .unwrap();
        assert_eq!(
            apply_continuous_batch_step(
                &mut batch,
                request,
                ContinuousBatchAction::Publish {
                    epoch: CompletionEpoch::new(10),
                    emitted_tokens: 1,
                },
            ),
            Err(ContinuousBatchError::AlreadyPublished)
        );
    }

    #[test]
    fn disabled_lifecycle_transition_is_exact_and_non_mutating() {
        let request = RequestId::new(1, 1);
        let mut batch = ContinuousBatch::initial();
        let before = batch;
        assert_eq!(
            apply_continuous_batch_step(
                &mut batch,
                request,
                ContinuousBatchAction::Dispatch {
                    epoch: CompletionEpoch::new(1)
                },
            ),
            Err(ContinuousBatchError::Lifecycle(TransitionError::NotReady))
        );
        assert_eq!(batch, before);
    }
}
