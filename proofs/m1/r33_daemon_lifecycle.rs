#![forbid(unsafe_code)]

//! Authority-free R33 daemon lifecycle refinement.
//!
//! This executable model is intentionally independent of the engineering
//! adapter and its `r33_service` implementation. It models only the protocol
//! state transition boundary: a successful backend action first creates a
//! pending response, and response delivery or abandonment then selects the
//! stable successor. It proves the exact 20-window order, fail-stop recovery,
//! and exact stop replay. It grants no adapter conformance, transport,
//! process, GPU, timing, serving, or M1 qualification authority.

use vstd::prelude::*;

verus! {

/// Exact number of measurement windows in one admitted R33 server start.
pub const M1_R33_DAEMON_WINDOW_COUNT_V1: usize = 20;

/// Authority-free daemon action modeled by this proof.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1R33DaemonActionV1 {
    Start,
    Ready,
    Measure,
    Stop,
}

impl M1R33DaemonActionV1 {
    fn matches(self, other: Self) -> (matches: bool)
        ensures matches == (self == other),
    {
        matches!(
            (self, other),
            (Self::Start, Self::Start)
                | (Self::Ready, Self::Ready)
                | (Self::Measure, Self::Measure)
                | (Self::Stop, Self::Stop)
        )
    }
}

/// Exact request identity and action coordinates used by the model.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1R33DaemonCommandV1 {
    pub action: M1R33DaemonActionV1,
    pub instance: u64,
    pub server_start: u64,
    pub window: usize,
    pub request: u64,
}

impl M1R33DaemonCommandV1 {
    fn matches(self, other: Self) -> (matches: bool)
        ensures matches == (self == other),
    {
        self.action.matches(other.action)
            && self.instance == other.instance
            && self.server_start == other.server_start
            && self.window == other.window
            && self.request == other.request
    }
}

/// Stable lifecycle states between complete response dispositions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1R33DaemonStableStateV1 {
    Idle,
    AwaitReady {
        instance: u64,
        server_start: u64,
    },
    BetweenWindows {
        instance: u64,
        server_start: u64,
        next: usize,
    },
    AwaitStop {
        instance: u64,
        server_start: u64,
    },
    Faulted {
        instance: u64,
        server_start: u64,
    },
    StopReplay {
        command: M1R33DaemonCommandV1,
    },
}

/// Pending response whose disposition selects one exact stable state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1R33DaemonPendingV1 {
    pub delivered: M1R33DaemonStableStateV1,
    pub abandoned: M1R33DaemonStableStateV1,
    pub request: u64,
}

/// Complete modeled lifecycle, including the response-acknowledgement gap.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1R33DaemonLifecycleV1 {
    Stable(M1R33DaemonStableStateV1),
    Pending(M1R33DaemonPendingV1),
}

/// Stable rejection from the authority-free dispatch model.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1R33DaemonDispatchErrorV1 {
    ResponsePending,
    StartRequired,
    OnlyExactReadyAdmitted,
    OnlyNextWindowAdmitted,
    OnlyExactStopAdmitted,
}

/// Transport disposition observed after the response is produced.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1R33DaemonResponseV1 {
    Delivered { request: u64 },
    Abandoned,
}

pub open spec fn m1_r33_stable_state_well_formed(
    state: M1R33DaemonStableStateV1,
) -> bool {
    match state {
        M1R33DaemonStableStateV1::BetweenWindows { next, .. } => {
            next < M1_R33_DAEMON_WINDOW_COUNT_V1
        },
        M1R33DaemonStableStateV1::StopReplay { command } => {
            command.action == M1R33DaemonActionV1::Stop
        },
        _ => true,
    }
}

pub open spec fn m1_r33_lifecycle_well_formed(state: M1R33DaemonLifecycleV1) -> bool {
    match state {
        M1R33DaemonLifecycleV1::Stable(stable) => m1_r33_stable_state_well_formed(stable),
        M1R33DaemonLifecycleV1::Pending(pending) => {
            m1_r33_stable_state_well_formed(pending.delivered)
                && m1_r33_stable_state_well_formed(pending.abandoned)
        },
    }
}

pub open spec fn m1_r33_exact_instance_action(
    command: M1R33DaemonCommandV1,
    action: M1R33DaemonActionV1,
    instance: u64,
    server_start: u64,
) -> bool {
    command.action == action
        && command.instance == instance
        && command.server_start == server_start
}

pub open spec fn m1_r33_pending(
    command: M1R33DaemonCommandV1,
    delivered: M1R33DaemonStableStateV1,
    abandoned: M1R33DaemonStableStateV1,
) -> M1R33DaemonLifecycleV1 {
    M1R33DaemonLifecycleV1::Pending(M1R33DaemonPendingV1 {
        delivered,
        abandoned,
        request: command.request,
    })
}

/// Mathematical dispatch relation implemented by [`dispatch_m1_r33_daemon_action_v1`].
pub open spec fn m1_r33_daemon_dispatch_spec(
    state: M1R33DaemonLifecycleV1,
    command: M1R33DaemonCommandV1,
    backend_succeeded: bool,
) -> Result<M1R33DaemonLifecycleV1, M1R33DaemonDispatchErrorV1> {
    match state {
        M1R33DaemonLifecycleV1::Pending(_) => {
            Err(M1R33DaemonDispatchErrorV1::ResponsePending)
        },
        M1R33DaemonLifecycleV1::Stable(stable) => match stable {
            M1R33DaemonStableStateV1::Idle => {
                if command.action != M1R33DaemonActionV1::Start {
                    Err(M1R33DaemonDispatchErrorV1::StartRequired)
                } else if !backend_succeeded {
                    Ok(M1R33DaemonLifecycleV1::Stable(M1R33DaemonStableStateV1::Faulted {
                        instance: command.instance,
                        server_start: command.server_start,
                    }))
                } else {
                    Ok(m1_r33_pending(
                        command,
                        M1R33DaemonStableStateV1::AwaitReady {
                            instance: command.instance,
                            server_start: command.server_start,
                        },
                        M1R33DaemonStableStateV1::Faulted {
                            instance: command.instance,
                            server_start: command.server_start,
                        },
                    ))
                }
            },
            M1R33DaemonStableStateV1::AwaitReady { instance, server_start } => {
                if m1_r33_exact_instance_action(
                    command,
                    M1R33DaemonActionV1::Stop,
                    instance,
                    server_start,
                ) {
                    m1_r33_stop_dispatch_spec(command, instance, server_start, backend_succeeded)
                } else if !m1_r33_exact_instance_action(
                    command,
                    M1R33DaemonActionV1::Ready,
                    instance,
                    server_start,
                ) {
                    Err(M1R33DaemonDispatchErrorV1::OnlyExactReadyAdmitted)
                } else if !backend_succeeded {
                    Ok(M1R33DaemonLifecycleV1::Stable(M1R33DaemonStableStateV1::Faulted {
                        instance,
                        server_start,
                    }))
                } else {
                    Ok(m1_r33_pending(
                        command,
                        M1R33DaemonStableStateV1::BetweenWindows {
                            instance,
                            server_start,
                            next: 0,
                        },
                        M1R33DaemonStableStateV1::Faulted { instance, server_start },
                    ))
                }
            },
            M1R33DaemonStableStateV1::BetweenWindows {
                instance,
                server_start,
                next,
            } => {
                if m1_r33_exact_instance_action(
                    command,
                    M1R33DaemonActionV1::Stop,
                    instance,
                    server_start,
                ) {
                    m1_r33_stop_dispatch_spec(command, instance, server_start, backend_succeeded)
                } else if !m1_r33_exact_instance_action(
                    command,
                    M1R33DaemonActionV1::Measure,
                    instance,
                    server_start,
                ) || command.window != next {
                    Err(M1R33DaemonDispatchErrorV1::OnlyNextWindowAdmitted)
                } else if !backend_succeeded {
                    Ok(M1R33DaemonLifecycleV1::Stable(M1R33DaemonStableStateV1::Faulted {
                        instance,
                        server_start,
                    }))
                } else {
                    let delivered = if next + 1 == M1_R33_DAEMON_WINDOW_COUNT_V1 {
                        M1R33DaemonStableStateV1::AwaitStop { instance, server_start }
                    } else {
                        M1R33DaemonStableStateV1::BetweenWindows {
                            instance,
                            server_start,
                            next: (next + 1) as usize,
                        }
                    };
                    Ok(m1_r33_pending(
                        command,
                        delivered,
                        M1R33DaemonStableStateV1::Faulted { instance, server_start },
                    ))
                }
            },
            M1R33DaemonStableStateV1::AwaitStop { instance, server_start }
            | M1R33DaemonStableStateV1::Faulted { instance, server_start } => {
                m1_r33_stop_dispatch_spec(command, instance, server_start, backend_succeeded)
            },
            M1R33DaemonStableStateV1::StopReplay { command: expected } => {
                if command != expected {
                    Err(M1R33DaemonDispatchErrorV1::OnlyExactStopAdmitted)
                } else {
                    Ok(m1_r33_pending(
                        command,
                        M1R33DaemonStableStateV1::Idle,
                        M1R33DaemonStableStateV1::StopReplay { command: expected },
                    ))
                }
            },
        },
    }
}

pub open spec fn m1_r33_stop_dispatch_spec(
    command: M1R33DaemonCommandV1,
    instance: u64,
    server_start: u64,
    backend_succeeded: bool,
) -> Result<M1R33DaemonLifecycleV1, M1R33DaemonDispatchErrorV1> {
    if !m1_r33_exact_instance_action(
        command,
        M1R33DaemonActionV1::Stop,
        instance,
        server_start,
    ) {
        Err(M1R33DaemonDispatchErrorV1::OnlyExactStopAdmitted)
    } else if !backend_succeeded {
        Ok(M1R33DaemonLifecycleV1::Stable(M1R33DaemonStableStateV1::Faulted {
            instance,
            server_start,
        }))
    } else {
        Ok(m1_r33_pending(
            command,
            M1R33DaemonStableStateV1::Idle,
            M1R33DaemonStableStateV1::StopReplay { command },
        ))
    }
}

/// Executes one daemon dispatch while preserving the exact mathematical relation.
///
/// # Errors
///
/// Returns the exact fail-closed rejection selected by the lifecycle relation.
pub fn dispatch_m1_r33_daemon_action_v1(
    state: M1R33DaemonLifecycleV1,
    command: M1R33DaemonCommandV1,
    backend_succeeded: bool,
) -> (result: Result<M1R33DaemonLifecycleV1, M1R33DaemonDispatchErrorV1>)
    requires m1_r33_lifecycle_well_formed(state),
    ensures
        result == m1_r33_daemon_dispatch_spec(state, command, backend_succeeded),
        result.is_ok() ==> m1_r33_lifecycle_well_formed(result.unwrap()),
{
    match state {
        M1R33DaemonLifecycleV1::Pending(_) => {
            Err(M1R33DaemonDispatchErrorV1::ResponsePending)
        },
        M1R33DaemonLifecycleV1::Stable(stable) => match stable {
            M1R33DaemonStableStateV1::Idle => {
                if !command.action.matches(M1R33DaemonActionV1::Start) {
                    Err(M1R33DaemonDispatchErrorV1::StartRequired)
                } else if !backend_succeeded {
                    Ok(M1R33DaemonLifecycleV1::Stable(M1R33DaemonStableStateV1::Faulted {
                        instance: command.instance,
                        server_start: command.server_start,
                    }))
                } else {
                    Ok(m1_r33_pending_exec(
                        command,
                        M1R33DaemonStableStateV1::AwaitReady {
                            instance: command.instance,
                            server_start: command.server_start,
                        },
                        M1R33DaemonStableStateV1::Faulted {
                            instance: command.instance,
                            server_start: command.server_start,
                        },
                    ))
                }
            },
            M1R33DaemonStableStateV1::AwaitReady { instance, server_start } => {
                if exact_instance_action_exec(
                    command,
                    M1R33DaemonActionV1::Stop,
                    instance,
                    server_start,
                ) {
                    stop_dispatch_exec(command, instance, server_start, backend_succeeded)
                } else if !exact_instance_action_exec(
                    command,
                    M1R33DaemonActionV1::Ready,
                    instance,
                    server_start,
                ) {
                    Err(M1R33DaemonDispatchErrorV1::OnlyExactReadyAdmitted)
                } else if !backend_succeeded {
                    Ok(M1R33DaemonLifecycleV1::Stable(M1R33DaemonStableStateV1::Faulted {
                        instance,
                        server_start,
                    }))
                } else {
                    Ok(m1_r33_pending_exec(
                        command,
                        M1R33DaemonStableStateV1::BetweenWindows {
                            instance,
                            server_start,
                            next: 0,
                        },
                        M1R33DaemonStableStateV1::Faulted { instance, server_start },
                    ))
                }
            },
            M1R33DaemonStableStateV1::BetweenWindows {
                instance,
                server_start,
                next,
            } => {
                if exact_instance_action_exec(
                    command,
                    M1R33DaemonActionV1::Stop,
                    instance,
                    server_start,
                ) {
                    stop_dispatch_exec(command, instance, server_start, backend_succeeded)
                } else if !exact_instance_action_exec(
                    command,
                    M1R33DaemonActionV1::Measure,
                    instance,
                    server_start,
                ) || command.window != next {
                    Err(M1R33DaemonDispatchErrorV1::OnlyNextWindowAdmitted)
                } else if !backend_succeeded {
                    Ok(M1R33DaemonLifecycleV1::Stable(M1R33DaemonStableStateV1::Faulted {
                        instance,
                        server_start,
                    }))
                } else {
                    let delivered = if next + 1 == M1_R33_DAEMON_WINDOW_COUNT_V1 {
                        M1R33DaemonStableStateV1::AwaitStop { instance, server_start }
                    } else {
                        M1R33DaemonStableStateV1::BetweenWindows {
                            instance,
                            server_start,
                            next: next + 1,
                        }
                    };
                    Ok(m1_r33_pending_exec(
                        command,
                        delivered,
                        M1R33DaemonStableStateV1::Faulted { instance, server_start },
                    ))
                }
            },
            M1R33DaemonStableStateV1::AwaitStop { instance, server_start }
            | M1R33DaemonStableStateV1::Faulted { instance, server_start } => {
                stop_dispatch_exec(command, instance, server_start, backend_succeeded)
            },
            M1R33DaemonStableStateV1::StopReplay { command: expected } => {
                if command.matches(expected) {
                    Ok(m1_r33_pending_exec(
                        command,
                        M1R33DaemonStableStateV1::Idle,
                        M1R33DaemonStableStateV1::StopReplay { command: expected },
                    ))
                } else {
                    Err(M1R33DaemonDispatchErrorV1::OnlyExactStopAdmitted)
                }
            },
        },
    }
}

fn exact_instance_action_exec(
    command: M1R33DaemonCommandV1,
    action: M1R33DaemonActionV1,
    instance: u64,
    server_start: u64,
) -> (exact: bool)
    ensures exact == m1_r33_exact_instance_action(command, action, instance, server_start),
{
    command.action.matches(action)
        && command.instance == instance
        && command.server_start == server_start
}

fn m1_r33_pending_exec(
    command: M1R33DaemonCommandV1,
    delivered: M1R33DaemonStableStateV1,
    abandoned: M1R33DaemonStableStateV1,
) -> (pending: M1R33DaemonLifecycleV1)
    ensures pending == m1_r33_pending(command, delivered, abandoned),
{
    M1R33DaemonLifecycleV1::Pending(M1R33DaemonPendingV1 {
        delivered,
        abandoned,
        request: command.request,
    })
}

fn stop_dispatch_exec(
    command: M1R33DaemonCommandV1,
    instance: u64,
    server_start: u64,
    backend_succeeded: bool,
) -> (result: Result<M1R33DaemonLifecycleV1, M1R33DaemonDispatchErrorV1>)
    ensures result == m1_r33_stop_dispatch_spec(
        command,
        instance,
        server_start,
        backend_succeeded,
    ),
{
    if !exact_instance_action_exec(
        command,
        M1R33DaemonActionV1::Stop,
        instance,
        server_start,
    ) {
        Err(M1R33DaemonDispatchErrorV1::OnlyExactStopAdmitted)
    } else if !backend_succeeded {
        Ok(M1R33DaemonLifecycleV1::Stable(M1R33DaemonStableStateV1::Faulted {
            instance,
            server_start,
        }))
    } else {
        Ok(m1_r33_pending_exec(
            command,
            M1R33DaemonStableStateV1::Idle,
            M1R33DaemonStableStateV1::StopReplay { command },
        ))
    }
}

/// Mathematical response-disposition relation.
pub open spec fn m1_r33_daemon_response_spec(
    state: M1R33DaemonLifecycleV1,
    response: M1R33DaemonResponseV1,
) -> Option<M1R33DaemonLifecycleV1> {
    match state {
        M1R33DaemonLifecycleV1::Stable(_) => None,
        M1R33DaemonLifecycleV1::Pending(pending) => match response {
            M1R33DaemonResponseV1::Delivered { request } => {
                if request == pending.request {
                    Some(M1R33DaemonLifecycleV1::Stable(pending.delivered))
                } else {
                    Some(M1R33DaemonLifecycleV1::Stable(pending.abandoned))
                }
            },
            M1R33DaemonResponseV1::Abandoned => {
                Some(M1R33DaemonLifecycleV1::Stable(pending.abandoned))
            },
        },
    }
}

/// Resolves one pending response without allowing abandonment to advance.
#[must_use]
pub fn resolve_m1_r33_daemon_response_v1(
    state: M1R33DaemonLifecycleV1,
    response: M1R33DaemonResponseV1,
) -> (result: Option<M1R33DaemonLifecycleV1>)
    requires m1_r33_lifecycle_well_formed(state),
    ensures
        result == m1_r33_daemon_response_spec(state, response),
        result.is_some() ==> m1_r33_lifecycle_well_formed(result.unwrap()),
{
    match state {
        M1R33DaemonLifecycleV1::Stable(_) => None,
        M1R33DaemonLifecycleV1::Pending(pending) => match response {
            M1R33DaemonResponseV1::Delivered { request } => {
                if request == pending.request {
                    Some(M1R33DaemonLifecycleV1::Stable(pending.delivered))
                } else {
                    Some(M1R33DaemonLifecycleV1::Stable(pending.abandoned))
                }
            },
            M1R33DaemonResponseV1::Abandoned => {
                Some(M1R33DaemonLifecycleV1::Stable(pending.abandoned))
            },
        },
    }
}

fn dispatch_and_deliver(
    state: M1R33DaemonLifecycleV1,
    command: M1R33DaemonCommandV1,
) -> (next: M1R33DaemonLifecycleV1)
    requires
        m1_r33_lifecycle_well_formed(state),
        m1_r33_daemon_dispatch_spec(state, command, true).is_ok(),
    ensures
        next == m1_r33_daemon_response_spec(
            m1_r33_daemon_dispatch_spec(state, command, true).unwrap(),
            M1R33DaemonResponseV1::Delivered { request: command.request },
        ).unwrap(),
        m1_r33_lifecycle_well_formed(next),
{
    let pending = match dispatch_m1_r33_daemon_action_v1(state, command, true) {
        Ok(pending) => pending,
        Err(_) => {
            assert(false);
            return state;
        },
    };
    match resolve_m1_r33_daemon_response_v1(
        pending,
        M1R33DaemonResponseV1::Delivered { request: command.request },
    ) {
        Some(next) => next,
        None => {
            assert(false);
            state
        },
    }
}

fn run_twenty_ordered_measures(
    mut state: M1R33DaemonLifecycleV1,
    instance: u64,
    server_start: u64,
) -> (result: (M1R33DaemonLifecycleV1, usize))
    requires state == M1R33DaemonLifecycleV1::Stable(
        M1R33DaemonStableStateV1::BetweenWindows {
            instance,
            server_start,
            next: 0,
        },
    ),
    ensures
        result.0 == M1R33DaemonLifecycleV1::Stable(
            M1R33DaemonStableStateV1::AwaitStop { instance, server_start },
        ),
        result.1 == M1_R33_DAEMON_WINDOW_COUNT_V1,
{
    let mut measured: usize = 0;
    while measured < M1_R33_DAEMON_WINDOW_COUNT_V1
        invariant
            measured <= M1_R33_DAEMON_WINDOW_COUNT_V1,
            state == if measured < M1_R33_DAEMON_WINDOW_COUNT_V1 {
                M1R33DaemonLifecycleV1::Stable(
                    M1R33DaemonStableStateV1::BetweenWindows {
                        instance,
                        server_start,
                        next: measured,
                    },
                )
            } else {
                M1R33DaemonLifecycleV1::Stable(
                    M1R33DaemonStableStateV1::AwaitStop { instance, server_start },
                )
            },
        decreases M1_R33_DAEMON_WINDOW_COUNT_V1 - measured,
    {
        let measure = M1R33DaemonCommandV1 {
            action: M1R33DaemonActionV1::Measure,
            instance,
            server_start,
            window: measured,
            request: measured as u64,
        };
        state = dispatch_and_deliver(state, measure);
        measured += 1;
    }
    (state, measured)
}

/// Executes and proves `start -> ready -> exactly 20 ordered measures -> stop`.
#[must_use]
pub fn m1_r33_daemon_successful_lifecycle_theorem(
    instance: u64,
    server_start: u64,
) -> (result: (M1R33DaemonLifecycleV1, usize))
    ensures
        result.0 == M1R33DaemonLifecycleV1::Stable(M1R33DaemonStableStateV1::Idle),
        result.1 == M1_R33_DAEMON_WINDOW_COUNT_V1,
{
    let start = M1R33DaemonCommandV1 {
        action: M1R33DaemonActionV1::Start,
        instance,
        server_start,
        window: 0,
        request: 20,
    };
    let mut state = dispatch_and_deliver(
        M1R33DaemonLifecycleV1::Stable(M1R33DaemonStableStateV1::Idle),
        start,
    );
    assert(state == M1R33DaemonLifecycleV1::Stable(
        M1R33DaemonStableStateV1::AwaitReady { instance, server_start },
    ));

    let ready = M1R33DaemonCommandV1 {
        action: M1R33DaemonActionV1::Ready,
        instance,
        server_start,
        window: 0,
        request: 21,
    };
    state = dispatch_and_deliver(state, ready);
    assert(state == M1R33DaemonLifecycleV1::Stable(
        M1R33DaemonStableStateV1::BetweenWindows {
            instance,
            server_start,
            next: 0,
        },
    ));

    let (measured_state, measured) = run_twenty_ordered_measures(state, instance, server_start);
    state = measured_state;
    assert(state == M1R33DaemonLifecycleV1::Stable(
        M1R33DaemonStableStateV1::AwaitStop { instance, server_start },
    ));

    let stop = M1R33DaemonCommandV1 {
        action: M1R33DaemonActionV1::Stop,
        instance,
        server_start,
        window: 0,
        request: 22,
    };
    state = dispatch_and_deliver(state, stop);
    (state, measured)
}

/// Proves faulted exact-stop-only recovery and exact stop replay.
#[must_use]
pub fn m1_r33_daemon_fault_stop_replay_theorem(
    instance: u64,
    server_start: u64,
) -> (state: M1R33DaemonLifecycleV1)
    ensures state == M1R33DaemonLifecycleV1::Stable(M1R33DaemonStableStateV1::Idle),
{
    let start = M1R33DaemonCommandV1 {
        action: M1R33DaemonActionV1::Start,
        instance,
        server_start,
        window: 0,
        request: 30,
    };
    let mut state = match dispatch_m1_r33_daemon_action_v1(
        M1R33DaemonLifecycleV1::Stable(M1R33DaemonStableStateV1::Idle),
        start,
        false,
    ) {
        Ok(state) => state,
        Err(_) => {
            assert(false);
            return M1R33DaemonLifecycleV1::Stable(M1R33DaemonStableStateV1::Idle);
        },
    };
    assert(state == M1R33DaemonLifecycleV1::Stable(
        M1R33DaemonStableStateV1::Faulted { instance, server_start },
    ));

    let _invalid = M1R33DaemonCommandV1 {
        action: M1R33DaemonActionV1::Measure,
        instance,
        server_start,
        window: 0,
        request: 31,
    };
    let _invalid_result = dispatch_m1_r33_daemon_action_v1(state, _invalid, true);
    assert(_invalid_result == Err(M1R33DaemonDispatchErrorV1::OnlyExactStopAdmitted));

    let stop = M1R33DaemonCommandV1 {
        action: M1R33DaemonActionV1::Stop,
        instance,
        server_start,
        window: 0,
        request: 32,
    };
    let pending_stop = match dispatch_m1_r33_daemon_action_v1(state, stop, true) {
        Ok(state) => state,
        Err(_) => {
            assert(false);
            return state;
        },
    };
    state = match resolve_m1_r33_daemon_response_v1(
        pending_stop,
        M1R33DaemonResponseV1::Abandoned,
    ) {
        Some(state) => state,
        None => {
            assert(false);
            return state;
        },
    };
    assert(state == M1R33DaemonLifecycleV1::Stable(
        M1R33DaemonStableStateV1::StopReplay { command: stop },
    ));

    let _wrong_replay = M1R33DaemonCommandV1 { request: 33, ..stop };
    let _wrong_replay_result =
        dispatch_m1_r33_daemon_action_v1(state, _wrong_replay, true);
    assert(_wrong_replay_result == Err(M1R33DaemonDispatchErrorV1::OnlyExactStopAdmitted));

    let pending_replay = match dispatch_m1_r33_daemon_action_v1(state, stop, false) {
        Ok(state) => state,
        Err(_) => {
            assert(false);
            return state;
        },
    };
    state = match resolve_m1_r33_daemon_response_v1(
        pending_replay,
        M1R33DaemonResponseV1::Delivered { request: stop.request },
    ) {
        Some(state) => state,
        None => {
            assert(false);
            return state;
        },
    };
    state
}

} // verus!

#[cfg(test)]
mod tests {
    use super::*;

    fn command(action: M1R33DaemonActionV1, window: usize, request: u64) -> M1R33DaemonCommandV1 {
        M1R33DaemonCommandV1 {
            action,
            instance: 7,
            server_start: 11,
            window,
            request,
        }
    }

    fn delivered(
        state: M1R33DaemonLifecycleV1,
        command: M1R33DaemonCommandV1,
    ) -> M1R33DaemonLifecycleV1 {
        let pending = dispatch_m1_r33_daemon_action_v1(state, command, true).unwrap();
        resolve_m1_r33_daemon_response_v1(
            pending,
            M1R33DaemonResponseV1::Delivered {
                request: command.request,
            },
        )
        .unwrap()
    }

    #[test]
    fn successful_path_requires_twenty_ordered_windows() {
        let (state, measured) = m1_r33_daemon_successful_lifecycle_theorem(7, 11);
        assert_eq!(measured, M1_R33_DAEMON_WINDOW_COUNT_V1);
        assert_eq!(
            state,
            M1R33DaemonLifecycleV1::Stable(M1R33DaemonStableStateV1::Idle)
        );
    }

    #[test]
    fn out_of_order_measure_is_rejected() {
        let state = delivered(
            M1R33DaemonLifecycleV1::Stable(M1R33DaemonStableStateV1::Idle),
            command(M1R33DaemonActionV1::Start, 0, 20),
        );
        let state = delivered(state, command(M1R33DaemonActionV1::Ready, 0, 21));
        assert_eq!(
            dispatch_m1_r33_daemon_action_v1(
                state,
                command(M1R33DaemonActionV1::Measure, 1, 0),
                true,
            ),
            Err(M1R33DaemonDispatchErrorV1::OnlyNextWindowAdmitted)
        );
    }

    #[test]
    fn abandoned_measure_faults_instead_of_advancing() {
        let state = delivered(
            M1R33DaemonLifecycleV1::Stable(M1R33DaemonStableStateV1::Idle),
            command(M1R33DaemonActionV1::Start, 0, 20),
        );
        let state = delivered(state, command(M1R33DaemonActionV1::Ready, 0, 21));
        let pending = dispatch_m1_r33_daemon_action_v1(
            state,
            command(M1R33DaemonActionV1::Measure, 0, 0),
            true,
        )
        .unwrap();
        assert_eq!(
            resolve_m1_r33_daemon_response_v1(pending, M1R33DaemonResponseV1::Abandoned,),
            Some(M1R33DaemonLifecycleV1::Stable(
                M1R33DaemonStableStateV1::Faulted {
                    instance: 7,
                    server_start: 11,
                }
            ))
        );
    }

    #[test]
    fn fault_recovery_replays_only_the_exact_stop() {
        assert_eq!(
            m1_r33_daemon_fault_stop_replay_theorem(7, 11),
            M1R33DaemonLifecycleV1::Stable(M1R33DaemonStableStateV1::Idle)
        );
    }

    #[test]
    fn exact_stop_replay_does_not_repeat_the_backend_stop() {
        let stop = command(M1R33DaemonActionV1::Stop, 0, 32);
        let replay =
            M1R33DaemonLifecycleV1::Stable(M1R33DaemonStableStateV1::StopReplay { command: stop });
        assert_eq!(
            dispatch_m1_r33_daemon_action_v1(replay, stop, false),
            Ok(M1R33DaemonLifecycleV1::Pending(M1R33DaemonPendingV1 {
                delivered: M1R33DaemonStableStateV1::Idle,
                abandoned: M1R33DaemonStableStateV1::StopReplay { command: stop },
                request: stop.request,
            }))
        );
    }
}
