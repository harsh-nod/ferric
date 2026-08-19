//! Sequential request and scheduler semantics.

use vstd::prelude::*;

verus! {

/// Abstract request-slot state shared by the oracle and production machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestState {
    Vacant,
    Ready,
    InFlight,
    Retiring,
}

/// Internal phase needed to distinguish GPU execution from mandatory KV work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifecyclePhase {
    Idle,
    Executing,
    AwaitingKv,
    RetiringExecuting,
    RetiringQuiescent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SequentialRequest {
    pub state: RequestState,
    pub phase: LifecyclePhase,
}

/// One sequential request transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestTransition {
    Admit,
    Dispatch,
    Retire,
    CompleteExact,
    FinalizeKv,
    DetachKv,
}

/// Rejection from the executable sequential transition oracle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionError {
    Occupied,
    NotReady,
    NotInFlight,
    AlreadyRetiring,
    NotRetiring,
}

/// Applies the sequential lifecycle rule for one slot.
///
/// Production code does not call this function in its hot path. Its Verus
/// postcondition is the abstract relation refined by the scheduler methods.
pub fn apply_request_transition(
    request: SequentialRequest,
    transition: RequestTransition,
) -> (result: Result<SequentialRequest, TransitionError>)
    ensures
        result == request_transition(request, transition),
{
    match transition {
        RequestTransition::Admit => match (request.state, request.phase) {
            (RequestState::Vacant, LifecyclePhase::Idle) => Ok(SequentialRequest {
                state: RequestState::Ready,
                phase: LifecyclePhase::Idle,
            }),
            _ => Err(TransitionError::Occupied),
        },
        RequestTransition::Dispatch => match (request.state, request.phase) {
            (RequestState::Ready, LifecyclePhase::Idle) => Ok(SequentialRequest {
                state: RequestState::InFlight,
                phase: LifecyclePhase::Executing,
            }),
            _ => Err(TransitionError::NotReady),
        },
        RequestTransition::Retire => match (request.state, request.phase) {
            (RequestState::Ready, LifecyclePhase::Idle)
            | (RequestState::InFlight, LifecyclePhase::AwaitingKv) => Ok(SequentialRequest {
                state: RequestState::Retiring,
                phase: LifecyclePhase::RetiringQuiescent,
            }),
            (RequestState::InFlight, LifecyclePhase::Executing) => Ok(SequentialRequest {
                state: RequestState::Retiring,
                phase: LifecyclePhase::RetiringExecuting,
            }),
            (RequestState::Retiring, _) => Err(TransitionError::AlreadyRetiring),
            _ => Err(TransitionError::NotReady),
        },
        RequestTransition::CompleteExact => match (request.state, request.phase) {
            (RequestState::InFlight, LifecyclePhase::Executing) => Ok(SequentialRequest {
                state: RequestState::InFlight,
                phase: LifecyclePhase::AwaitingKv,
            }),
            (RequestState::Retiring, LifecyclePhase::RetiringExecuting) => {
                Ok(SequentialRequest {
                    state: RequestState::Retiring,
                    phase: LifecyclePhase::RetiringQuiescent,
                })
            }
            _ => Err(TransitionError::NotInFlight),
        },
        RequestTransition::FinalizeKv => match (request.state, request.phase) {
            (RequestState::InFlight, LifecyclePhase::AwaitingKv) => Ok(SequentialRequest {
                state: RequestState::Ready,
                phase: LifecyclePhase::Idle,
            }),
            _ => Err(TransitionError::NotInFlight),
        },
        RequestTransition::DetachKv => match (request.state, request.phase) {
            (RequestState::Retiring, LifecyclePhase::RetiringQuiescent) => {
                Ok(SequentialRequest {
                    state: RequestState::Vacant,
                    phase: LifecyclePhase::Idle,
                })
            }
            _ => Err(TransitionError::NotRetiring),
        },
    }
}

/// Mathematical form of [`apply_request_transition`].
pub open spec fn request_transition(
    request: SequentialRequest,
    transition: RequestTransition,
) -> Result<SequentialRequest, TransitionError> {
    match transition {
        RequestTransition::Admit => match (request.state, request.phase) {
            (RequestState::Vacant, LifecyclePhase::Idle) => Ok(SequentialRequest {
                state: RequestState::Ready,
                phase: LifecyclePhase::Idle,
            }),
            _ => Err(TransitionError::Occupied),
        },
        RequestTransition::Dispatch => match (request.state, request.phase) {
            (RequestState::Ready, LifecyclePhase::Idle) => Ok(SequentialRequest {
                state: RequestState::InFlight,
                phase: LifecyclePhase::Executing,
            }),
            _ => Err(TransitionError::NotReady),
        },
        RequestTransition::Retire => match (request.state, request.phase) {
            (RequestState::Ready, LifecyclePhase::Idle)
            | (RequestState::InFlight, LifecyclePhase::AwaitingKv) => Ok(SequentialRequest {
                state: RequestState::Retiring,
                phase: LifecyclePhase::RetiringQuiescent,
            }),
            (RequestState::InFlight, LifecyclePhase::Executing) => Ok(SequentialRequest {
                state: RequestState::Retiring,
                phase: LifecyclePhase::RetiringExecuting,
            }),
            (RequestState::Retiring, _) => Err(TransitionError::AlreadyRetiring),
            _ => Err(TransitionError::NotReady),
        },
        RequestTransition::CompleteExact => match (request.state, request.phase) {
            (RequestState::InFlight, LifecyclePhase::Executing) => Ok(SequentialRequest {
                state: RequestState::InFlight,
                phase: LifecyclePhase::AwaitingKv,
            }),
            (RequestState::Retiring, LifecyclePhase::RetiringExecuting) => {
                Ok(SequentialRequest {
                    state: RequestState::Retiring,
                    phase: LifecyclePhase::RetiringQuiescent,
                })
            }
            _ => Err(TransitionError::NotInFlight),
        },
        RequestTransition::FinalizeKv => match (request.state, request.phase) {
            (RequestState::InFlight, LifecyclePhase::AwaitingKv) => Ok(SequentialRequest {
                state: RequestState::Ready,
                phase: LifecyclePhase::Idle,
            }),
            _ => Err(TransitionError::NotInFlight),
        },
        RequestTransition::DetachKv => match (request.state, request.phase) {
            (RequestState::Retiring, LifecyclePhase::RetiringQuiescent) => {
                Ok(SequentialRequest {
                    state: RequestState::Vacant,
                    phase: LifecyclePhase::Idle,
                })
            }
            _ => Err(TransitionError::NotRetiring),
        },
    }
}

/// Cancellation remains terminal when an in-flight member completes.
pub proof fn completion_does_not_resurrect_retiring()
    ensures
        request_transition(
            SequentialRequest {
                state: RequestState::Retiring,
                phase: LifecyclePhase::RetiringExecuting,
            },
            RequestTransition::CompleteExact,
        ) == Ok(SequentialRequest {
            state: RequestState::Retiring,
            phase: LifecyclePhase::RetiringQuiescent,
        }),
{
}

}

#[cfg(test)]
mod tests {
    use super::{
        apply_request_transition, LifecyclePhase, RequestState, RequestTransition,
        SequentialRequest, TransitionError,
    };

    #[test]
    fn retiring_completion_is_terminal() {
        assert_eq!(
            apply_request_transition(
                SequentialRequest {
                    state: RequestState::Retiring,
                    phase: LifecyclePhase::RetiringExecuting,
                },
                RequestTransition::CompleteExact,
            ),
            Ok(SequentialRequest {
                state: RequestState::Retiring,
                phase: LifecyclePhase::RetiringQuiescent,
            })
        );
        assert_eq!(
            apply_request_transition(
                SequentialRequest {
                    state: RequestState::Vacant,
                    phase: LifecyclePhase::Idle,
                },
                RequestTransition::Dispatch,
            ),
            Err(TransitionError::NotReady)
        );
    }
}
