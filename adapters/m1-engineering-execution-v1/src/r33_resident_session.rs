//! Reusable fail-closed custody state for an authenticated R33 session.
//!
//! This module deliberately does not manufacture physical authority. The
//! generic custody is intended to be Ferric's move-only authenticated
//! runner/Engine/model/KV/queue owner. A successful window must return that
//! owner before another window can be admitted; a failed window is retained
//! opaquely until exact stop.

use core::fmt;

use ferric_engine::M1AuthenticatedSpeculativePhysicalRoundInputsV1;
use ferric_spec::{completion::CompletionEpoch, RequestId, TokenId};

use crate::r33_service::M1R33WorkloadWindowV1;

/// Exact number of sequential windows in one R33 service instance.
pub const M1_R33_RESIDENT_WINDOWS_PER_INSTANCE_V1: usize = 20;

/// Observable state of one resident custody session.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1R33ResidentCustodySessionPhaseV1 {
    Ready,
    Exhausted,
    Faulted,
    Stopped,
}

/// Stable pre-execution rejection that leaves resident custody unchanged.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1R33ResidentWindowAdmissionErrorV1 {
    InvalidWindow,
    UnsupportedRoster,
    WindowLimit,
    WindowOrder,
    WindowBinding,
    InstanceBinding,
    SessionNotReady,
}

/// One typed dynamic input joined to an exact R33 workload window.
#[must_use = "the exact window binding and dynamic input remain joined"]
#[derive(Debug)]
pub struct M1R33ResidentWindowBindingV1<I> {
    sequence: usize,
    row_id: Box<str>,
    row_ordinal: u64,
    server_start: u64,
    window: u64,
    prompt_tokens: Box<[TokenId]>,
    expected_output_tokens: u64,
    input: I,
}

/// Binding rejection retaining the unchanged dynamic input.
#[must_use = "the rejected dynamic input remains caller-owned"]
#[derive(Debug)]
pub struct M1R33ResidentWindowBindingFailureV1<I> {
    error: M1R33ResidentWindowAdmissionErrorV1,
    input: I,
}

impl<I> M1R33ResidentWindowBindingFailureV1<I> {
    #[must_use]
    pub const fn error(&self) -> M1R33ResidentWindowAdmissionErrorV1 {
        self.error
    }

    #[must_use = "the rejected dynamic input remains caller-owned"]
    pub fn into_input(self) -> I {
        self.input
    }
}

impl<I> M1R33ResidentWindowBindingV1<I> {
    /// Joins a move-only dynamic execution input to one exact one-request row.
    ///
    /// # Errors
    ///
    /// Rejects malformed rows, multi-request windows, and sequences outside
    /// the exact 20-window R33 instance. The input is returned unchanged.
    pub fn bind(
        sequence: usize,
        window: &M1R33WorkloadWindowV1,
        input: I,
    ) -> Result<Self, M1R33ResidentWindowBindingFailureV1<I>> {
        let reject = |error, input| M1R33ResidentWindowBindingFailureV1 { error, input };
        if sequence >= M1_R33_RESIDENT_WINDOWS_PER_INSTANCE_V1 {
            return Err(reject(
                M1R33ResidentWindowAdmissionErrorV1::WindowLimit,
                input,
            ));
        }
        if window.validate().is_err() {
            return Err(reject(
                M1R33ResidentWindowAdmissionErrorV1::InvalidWindow,
                input,
            ));
        }
        let Some(request) = window
            .requests
            .first()
            .filter(|_| window.requests.len() == 1)
        else {
            return Err(reject(
                M1R33ResidentWindowAdmissionErrorV1::UnsupportedRoster,
                input,
            ));
        };
        Ok(Self {
            sequence,
            row_id: window.row.id.clone().into_boxed_str(),
            row_ordinal: window.row.ordinal,
            server_start: window.row.server_start,
            window: window.row.window,
            prompt_tokens: request.prompt_tokens.clone().into_boxed_slice(),
            expected_output_tokens: request.expected_output_tokens,
            input,
        })
    }

    #[must_use]
    pub const fn sequence(&self) -> usize {
        self.sequence
    }

    #[must_use]
    pub fn prompt_tokens(&self) -> &[TokenId] {
        &self.prompt_tokens
    }

    #[must_use]
    pub const fn expected_output_tokens(&self) -> u64 {
        self.expected_output_tokens
    }

    fn matches(&self, window: &M1R33WorkloadWindowV1) -> bool {
        let Some(request) = window
            .requests
            .first()
            .filter(|_| window.requests.len() == 1)
        else {
            return false;
        };
        window.validate().is_ok()
            && self.row_id.as_ref() == window.row.id.as_str()
            && self.row_ordinal == window.row.ordinal
            && self.server_start == window.row.server_start
            && self.window == window.row.window
            && self.prompt_tokens.as_ref() == request.prompt_tokens.as_slice()
            && self.expected_output_tokens == request.expected_output_tokens
    }

    fn into_input(self) -> I {
        self.input
    }
}

/// Authenticated facts required to build token-bearing inputs for the next round.
///
/// This is descriptive state, not completion or launch authority. The lower
/// authenticated executor independently rejoins every field to its retained
/// queue generation before detachment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1R33AuthenticatedDynamicRoundFactsV1 {
    request: RequestId,
    epoch: CompletionEpoch,
    round: u64,
    anchor: TokenId,
    target_committed_tokens: u32,
    draft_committed_tokens: u32,
}

impl M1R33AuthenticatedDynamicRoundFactsV1 {
    pub const fn new(
        request: RequestId,
        epoch: CompletionEpoch,
        round: u64,
        anchor: TokenId,
        target_committed_tokens: u32,
        draft_committed_tokens: u32,
    ) -> Self {
        Self {
            request,
            epoch,
            round,
            anchor,
            target_committed_tokens,
            draft_committed_tokens,
        }
    }

    #[must_use]
    pub const fn request(self) -> RequestId {
        self.request
    }

    #[must_use]
    pub const fn epoch(self) -> CompletionEpoch {
        self.epoch
    }

    #[must_use]
    pub const fn round(self) -> u64 {
        self.round
    }

    #[must_use]
    pub const fn anchor(self) -> TokenId {
        self.anchor
    }

    #[must_use]
    pub const fn target_committed_tokens(self) -> u32 {
        self.target_committed_tokens
    }

    #[must_use]
    pub const fn draft_committed_tokens(self) -> u32 {
        self.draft_committed_tokens
    }
}

/// Supplies one exact move-only round input after authenticated output is known.
///
/// Implementations may own logical-runner declarations and linear workspace
/// plan pairs. They cannot grant execution authority: the returned input is
/// still checked by `M1AuthenticatedSpeculativePhysicalExecutorV1` against its
/// retained request, epoch, anchor, role cursors, queue shape, and lineage.
pub trait M1R33AuthenticatedDynamicRoundInputSourceV1 {
    type Failure: fmt::Debug;

    fn next_round(
        &mut self,
        facts: M1R33AuthenticatedDynamicRoundFactsV1,
    ) -> Result<M1AuthenticatedSpeculativePhysicalRoundInputsV1, Self::Failure>;
}

/// Terminal execution failure retained by a resident session.
#[must_use = "terminal physical failure custody remains retained"]
pub struct M1R33ResidentTerminalFailureV1<F> {
    code: &'static str,
    retained: F,
}

impl<F> M1R33ResidentTerminalFailureV1<F> {
    pub(crate) const fn new(code: &'static str, retained: F) -> Self {
        Self { code, retained }
    }

    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.code
    }
}

impl<F: fmt::Debug> fmt::Debug for M1R33ResidentTerminalFailureV1<F> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1R33ResidentTerminalFailureV1")
            .field("code", &self.code)
            .field("retained", &self.retained)
            .finish()
    }
}

/// Window attempt rejection. Admission returns the unchanged input; execution
/// failure has already moved all physical custody into the session fault.
#[must_use]
#[derive(Debug)]
pub enum M1R33ResidentWindowAttemptFailureV1<I> {
    Admission {
        error: M1R33ResidentWindowAdmissionErrorV1,
        binding: M1R33ResidentWindowBindingV1<I>,
    },
    Execution {
        code: &'static str,
    },
}

impl<I> M1R33ResidentWindowAttemptFailureV1<I> {
    #[must_use]
    pub const fn admission_error(&self) -> Option<M1R33ResidentWindowAdmissionErrorV1> {
        match self {
            Self::Admission { error, .. } => Some(*error),
            Self::Execution { .. } => None,
        }
    }

    #[must_use]
    pub const fn code(&self) -> Option<&'static str> {
        match self {
            Self::Admission { .. } => None,
            Self::Execution { code } => Some(code),
        }
    }

    #[must_use = "a pre-execution rejection retains the exact input"]
    pub fn into_binding(self) -> Option<M1R33ResidentWindowBindingV1<I>> {
        match self {
            Self::Admission { binding, .. } => Some(binding),
            Self::Execution { .. } => None,
        }
    }
}

enum M1R33ResidentCustodyStateV1<C> {
    Transitioning,
    Ready(C),
    Exhausted(C),
    Faulted(Box<dyn fmt::Debug>),
    Stopped,
}

/// Linear owner enforcing reusable success and terminal failure custody.
#[must_use = "resident authenticated custody must be executed or stopped"]
pub struct M1R33ResidentCustodySessionV1<C> {
    instance_sha256: Box<str>,
    server_start: u64,
    completed_windows: usize,
    state: M1R33ResidentCustodyStateV1<C>,
}

impl<C> fmt::Debug for M1R33ResidentCustodySessionV1<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1R33ResidentCustodySessionV1")
            .field("instance_sha256", &self.instance_sha256)
            .field("server_start", &self.server_start)
            .field("completed_windows", &self.completed_windows)
            .field("phase", &self.phase())
            .finish_non_exhaustive()
    }
}

impl<C> M1R33ResidentCustodySessionV1<C> {
    /// Creates one instance-bound owner without publishing its custody.
    ///
    /// # Errors
    ///
    /// Returns `custody` unchanged when the instance is not lowercase SHA-256.
    pub fn new(instance_sha256: &str, server_start: u64, custody: C) -> Result<Self, C> {
        if !super::r33_production_backend::valid_sha256(instance_sha256) {
            return Err(custody);
        }
        Ok(Self {
            instance_sha256: instance_sha256.into(),
            server_start,
            completed_windows: 0,
            state: M1R33ResidentCustodyStateV1::Ready(custody),
        })
    }

    #[must_use]
    pub const fn completed_windows(&self) -> usize {
        self.completed_windows
    }

    #[must_use]
    pub const fn phase(&self) -> M1R33ResidentCustodySessionPhaseV1 {
        match &self.state {
            M1R33ResidentCustodyStateV1::Ready(_) => M1R33ResidentCustodySessionPhaseV1::Ready,
            M1R33ResidentCustodyStateV1::Exhausted(_) => {
                M1R33ResidentCustodySessionPhaseV1::Exhausted
            }
            M1R33ResidentCustodyStateV1::Transitioning => {
                M1R33ResidentCustodySessionPhaseV1::Faulted
            }
            M1R33ResidentCustodyStateV1::Faulted(retained) => {
                let _ = retained;
                M1R33ResidentCustodySessionPhaseV1::Faulted
            }
            M1R33ResidentCustodyStateV1::Stopped => {
                M1R33ResidentCustodySessionPhaseV1::Stopped
            }
        }
    }

    /// Executes one exact bound window and retains returned custody on success.
    ///
    /// Admission rejection occurs before custody moves. Once `execute` is
    /// called, failure is terminal and its opaque lower owner remains in this
    /// session until `stop`.
    pub fn execute_window<I, R, F>(
        &mut self,
        instance_sha256: &str,
        window: &M1R33WorkloadWindowV1,
        binding: M1R33ResidentWindowBindingV1<I>,
        execute: impl FnOnce(C, I) -> Result<(C, R), M1R33ResidentTerminalFailureV1<F>>,
    ) -> Result<R, M1R33ResidentWindowAttemptFailureV1<I>>
    where
        F: fmt::Debug + 'static,
    {
        let admission_error = if self.instance_sha256.as_ref() != instance_sha256
            || self.server_start != window.row.server_start
        {
            Some(M1R33ResidentWindowAdmissionErrorV1::InstanceBinding)
        } else if binding.sequence != self.completed_windows {
            Some(M1R33ResidentWindowAdmissionErrorV1::WindowOrder)
        } else if !binding.matches(window) {
            Some(M1R33ResidentWindowAdmissionErrorV1::WindowBinding)
        } else {
            match &self.state {
                M1R33ResidentCustodyStateV1::Ready(_) => None,
                M1R33ResidentCustodyStateV1::Exhausted(_) => {
                    Some(M1R33ResidentWindowAdmissionErrorV1::WindowLimit)
                }
                M1R33ResidentCustodyStateV1::Transitioning
                | M1R33ResidentCustodyStateV1::Faulted(_)
                | M1R33ResidentCustodyStateV1::Stopped => {
                    Some(M1R33ResidentWindowAdmissionErrorV1::SessionNotReady)
                }
            }
        };
        if let Some(error) = admission_error {
            return Err(M1R33ResidentWindowAttemptFailureV1::Admission { error, binding });
        }

        let state = core::mem::replace(
            &mut self.state,
            M1R33ResidentCustodyStateV1::Transitioning,
        );
        let M1R33ResidentCustodyStateV1::Ready(custody) = state else {
            self.state = state;
            return Err(M1R33ResidentWindowAttemptFailureV1::Admission {
                error: M1R33ResidentWindowAdmissionErrorV1::SessionNotReady,
                binding,
            });
        };
        match execute(custody, binding.into_input()) {
            Ok((custody, report)) => {
                self.completed_windows += 1;
                self.state = if self.completed_windows == M1_R33_RESIDENT_WINDOWS_PER_INSTANCE_V1 {
                    M1R33ResidentCustodyStateV1::Exhausted(custody)
                } else {
                    M1R33ResidentCustodyStateV1::Ready(custody)
                };
                Ok(report)
            }
            Err(failure) => {
                let code = failure.code();
                self.state = M1R33ResidentCustodyStateV1::Faulted(Box::new(failure));
                Err(M1R33ResidentWindowAttemptFailureV1::Execution { code })
            }
        }
    }

    /// Drops all retained custody exactly once for the bound instance.
    pub fn stop(
        &mut self,
        instance_sha256: &str,
    ) -> Result<(), M1R33ResidentWindowAdmissionErrorV1> {
        if self.instance_sha256.as_ref() != instance_sha256 {
            return Err(M1R33ResidentWindowAdmissionErrorV1::InstanceBinding);
        }
        if matches!(&self.state, M1R33ResidentCustodyStateV1::Stopped) {
            return Ok(());
        }
        let state = core::mem::replace(&mut self.state, M1R33ResidentCustodyStateV1::Stopped);
        drop(state);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::r33_wire::{M1R33CollectorRowV1, M1R33WorkV1, M1R33WorkloadRequestV1};
    use std::cell::Cell;
    use std::rc::Rc;

    const INSTANCE: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const WRONG_INSTANCE: &str =
        "1123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[derive(Debug)]
    struct DropWitness(Rc<Cell<usize>>);

    impl Drop for DropWitness {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    fn window(sequence: usize) -> M1R33WorkloadWindowV1 {
        M1R33WorkloadWindowV1 {
            requests: vec![M1R33WorkloadRequestV1 {
                expected_output_tokens: 8,
                prompt_tokens: vec![u32::try_from(sequence).unwrap(); 128],
                request_ordinal: 0,
            }],
            row: M1R33CollectorRowV1 {
                expected_work: M1R33WorkV1 {
                    input_tokens: 128,
                    output_tokens: 8,
                    successful_requests: 1,
                    total_tokens: 136,
                },
                id: format!("start-0.window-{sequence:02}"),
                ordinal: u64::try_from(sequence).unwrap(),
                phase: if sequence < 10 { "warmup" } else { "recorded" }.to_owned(),
                server_start: 0,
                window: u64::try_from(sequence % 10).unwrap(),
            },
        }
    }

    #[test]
    fn twenty_successes_retain_one_custody_owner_until_stop() {
        let drops = Rc::new(Cell::new(0));
        let mut session =
            M1R33ResidentCustodySessionV1::new(INSTANCE, 0, DropWitness(Rc::clone(&drops)))
                .unwrap();
        for sequence in 0..M1_R33_RESIDENT_WINDOWS_PER_INSTANCE_V1 {
            let window = window(sequence);
            let binding = M1R33ResidentWindowBindingV1::bind(sequence, &window, sequence).unwrap();
            let report = session
                .execute_window(INSTANCE, &window, binding, |custody, input| {
                    Ok::<_, M1R33ResidentTerminalFailureV1<()>>((custody, input))
                })
                .unwrap();
            assert_eq!(report, sequence);
            assert_eq!(session.completed_windows(), sequence + 1);
            assert_eq!(
                session.phase(),
                if sequence + 1 == M1_R33_RESIDENT_WINDOWS_PER_INSTANCE_V1 {
                    M1R33ResidentCustodySessionPhaseV1::Exhausted
                } else {
                    M1R33ResidentCustodySessionPhaseV1::Ready
                }
            );
            assert_eq!(drops.get(), 0);
        }
        assert_eq!(session.phase(), M1R33ResidentCustodySessionPhaseV1::Exhausted);
        session.stop(INSTANCE).unwrap();
        assert_eq!(drops.get(), 1);
        session.stop(INSTANCE).unwrap();
        assert_eq!(drops.get(), 1);
    }

    #[test]
    fn hostile_identity_order_and_substitution_preserve_ready_custody() {
        let drops = Rc::new(Cell::new(0));
        let mut session =
            M1R33ResidentCustodySessionV1::new(INSTANCE, 0, DropWitness(Rc::clone(&drops)))
                .unwrap();
        let first = window(0);
        let binding = M1R33ResidentWindowBindingV1::bind(0, &first, 7).unwrap();
        let failure = session
            .execute_window(
                WRONG_INSTANCE,
                &first,
                binding,
                |_, _| -> Result<
                    (DropWitness, ()),
                    M1R33ResidentTerminalFailureV1<()>,
                > { unreachable!("identity rejection is pre-execution") },
            )
            .unwrap_err();
        assert_eq!(
            failure.admission_error(),
            Some(M1R33ResidentWindowAdmissionErrorV1::InstanceBinding)
        );
        let binding = failure.into_binding().unwrap();
        let second = window(1);
        let failure = session
            .execute_window(
                INSTANCE,
                &second,
                binding,
                |_, _| -> Result<
                    (DropWitness, ()),
                    M1R33ResidentTerminalFailureV1<()>,
                > { unreachable!("window substitution is pre-execution") },
            )
            .unwrap_err();
        assert_eq!(
            failure.admission_error(),
            Some(M1R33ResidentWindowAdmissionErrorV1::WindowBinding)
        );
        let out_of_order =
            M1R33ResidentWindowBindingV1::bind(1, &second, 11).unwrap();
        let failure = session
            .execute_window(
                INSTANCE,
                &second,
                out_of_order,
                |_, _| -> Result<
                    (DropWitness, ()),
                    M1R33ResidentTerminalFailureV1<()>,
                > { unreachable!("out-of-order rejection is pre-execution") },
            )
            .unwrap_err();
        assert_eq!(
            failure.admission_error(),
            Some(M1R33ResidentWindowAdmissionErrorV1::WindowOrder)
        );
        assert_eq!(session.phase(), M1R33ResidentCustodySessionPhaseV1::Ready);
        assert_eq!(session.completed_windows(), 0);
        assert_eq!(drops.get(), 0);
    }

    #[test]
    fn terminal_execution_failure_retains_custody_until_exact_stop() {
        let drops = Rc::new(Cell::new(0));
        let mut session =
            M1R33ResidentCustodySessionV1::new(INSTANCE, 0, DropWitness(Rc::clone(&drops)))
                .unwrap();
        let first = window(0);
        let binding = M1R33ResidentWindowBindingV1::bind(0, &first, 9).unwrap();
        let failure = session
            .execute_window(INSTANCE, &first, binding, |custody, input| {
                Err::<(DropWitness, ()), _>(M1R33ResidentTerminalFailureV1::new(
                    "physical-quarantine",
                    (custody, input),
                ))
            })
            .unwrap_err();
        assert_eq!(failure.code(), Some("physical-quarantine"));
        assert_eq!(session.phase(), M1R33ResidentCustodySessionPhaseV1::Faulted);
        assert_eq!(session.completed_windows(), 0);
        assert_eq!(drops.get(), 0);
        assert_eq!(
            session.stop(WRONG_INSTANCE).unwrap_err(),
            M1R33ResidentWindowAdmissionErrorV1::InstanceBinding
        );
        assert_eq!(drops.get(), 0);
        session.stop(INSTANCE).unwrap();
        assert_eq!(drops.get(), 1);
    }

    #[test]
    fn out_of_range_binding_returns_move_only_input() {
        let drops = Rc::new(Cell::new(0));
        let input = DropWitness(Rc::clone(&drops));
        let failure = M1R33ResidentWindowBindingV1::bind(
            M1_R33_RESIDENT_WINDOWS_PER_INSTANCE_V1,
            &window(0),
            input,
        )
        .unwrap_err();
        assert_eq!(
            failure.error(),
            M1R33ResidentWindowAdmissionErrorV1::WindowLimit
        );
        assert_eq!(drops.get(), 0);
        drop(failure.into_input());
        assert_eq!(drops.get(), 1);
    }

    #[test]
    fn dynamic_round_facts_preserve_authenticated_coordinates() {
        let request = RequestId::new(37, 2);
        let epoch = CompletionEpoch::new(41);
        let facts = M1R33AuthenticatedDynamicRoundFactsV1::new(
            request,
            epoch,
            3,
            101,
            132,
            130,
        );
        assert_eq!(facts.request(), request);
        assert_eq!(facts.epoch(), epoch);
        assert_eq!(facts.round(), 3);
        assert_eq!(facts.anchor(), 101);
        assert_eq!(facts.target_committed_tokens(), 132);
        assert_eq!(facts.draft_committed_tokens(), 130);
    }
}
