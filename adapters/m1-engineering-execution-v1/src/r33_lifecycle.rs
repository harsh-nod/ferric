//! Bounded-window lifecycle state for the Ferric R33 adapter.
//!
//! R33 invokes an external adapter once per lifecycle action, but one Ferric
//! server instance must remain alive across those invocations. This module is
//! the in-process half of that boundary. It admits a complete, pretokenized
//! window before physical publication, binds that window to Ferric's production
//! physical-runner operations, and records only clock observations made by the
//! live process. Process supervision and the collector wire protocol remain at
//! the executable boundary.

use ferric_engine::{
    Engine, EngineError, M1CheckedCompletionOutputV1, M1PhysicalRunnerV1,
    M1QueuedServingPhysicalInputProviderV1, M1ServingBatchPlanV1,
    M1ServingFirstPublicationWorkMatchErrorV1, M1ServingPhysicalReadbackV1,
    M1ServingPhysicalRunnerOperationErrorV1, M1ServingPhysicalRunnerOperationsCreateErrorV1,
    M1ServingPhysicalRunnerOperationsV1, M1ServingPhysicalRunnerReadbackV1, M1ServingPlanV1,
    M1ServingQuiescenceV1, M1ServingRegistryErrorV1, M1ServingRegistryV1, M1ServingRequestPhaseV1,
};
use ferric_spec::{
    M1_MAX_ACTIVE_SEQUENCES, M1_MAX_CONTEXT_TOKENS, QWEN3_VOCABULARY_SIZE, Qwen3ExecutionMode,
    RequestId,
};
use rustix::time::{ClockId, clock_gettime};
use serde::Serialize;
use std::fmt;

/// R33's exact timing-clock label.
pub const M1_R33_MONOTONIC_RAW_CLOCK_V1: &str = "monotonic-raw-nanoseconds";

/// One externally frozen request after tokenizer admission.
#[derive(Debug)]
pub struct M1R33PretokenizedRequestV1 {
    request_ordinal: usize,
    prompt_tokens: Box<[u32]>,
    expected_output_tokens: usize,
}

impl M1R33PretokenizedRequestV1 {
    /// Validates one request before any Engine state changes.
    ///
    /// # Errors
    ///
    /// Rejects non-contiguous ordinals, empty or out-of-vocabulary prompts,
    /// fewer than two output tokens, and work outside the M1 context bound.
    pub fn new(
        request_ordinal: usize,
        prompt_tokens: Box<[u32]>,
        expected_output_tokens: usize,
    ) -> Result<Self, M1R33PretokenizedRequestErrorV1> {
        if prompt_tokens.is_empty() {
            return Err(M1R33PretokenizedRequestErrorV1::EmptyPrompt);
        }
        if prompt_tokens
            .iter()
            .any(|token| *token >= QWEN3_VOCABULARY_SIZE)
        {
            return Err(M1R33PretokenizedRequestErrorV1::TokenOutOfRange);
        }
        if expected_output_tokens < 2 {
            return Err(M1R33PretokenizedRequestErrorV1::OutputTooShort);
        }
        if prompt_tokens
            .len()
            .checked_add(expected_output_tokens)
            .is_none_or(|tokens| tokens > M1_MAX_CONTEXT_TOKENS as usize)
        {
            return Err(M1R33PretokenizedRequestErrorV1::ContextExceeded);
        }
        Ok(Self {
            request_ordinal,
            prompt_tokens,
            expected_output_tokens,
        })
    }

    /// Position in the collector's ordered successful-request population.
    #[must_use]
    pub const fn request_ordinal(&self) -> usize {
        self.request_ordinal
    }

    /// Exact tokenizer-admitted prompt tokens.
    #[must_use]
    pub fn prompt_tokens(&self) -> &[u32] {
        &self.prompt_tokens
    }

    /// Exact output work frozen by the R33 row.
    #[must_use]
    pub const fn expected_output_tokens(&self) -> usize {
        self.expected_output_tokens
    }
}

/// Pretokenized request rejection before Engine admission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1R33PretokenizedRequestErrorV1 {
    EmptyPrompt,
    TokenOutOfRange,
    OutputTooShort,
    ContextExceeded,
}

/// Stable binding between one R33 request ordinal and Ferric request authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1R33WindowRequestBindingV1 {
    request_ordinal: usize,
    request: RequestId,
    input_tokens: usize,
    expected_output_tokens: usize,
}

impl M1R33WindowRequestBindingV1 {
    #[must_use]
    pub const fn request_ordinal(self) -> usize {
        self.request_ordinal
    }

    #[must_use]
    pub const fn request(self) -> RequestId {
        self.request
    }

    #[must_use]
    pub const fn input_tokens(self) -> usize {
        self.input_tokens
    }

    #[must_use]
    pub const fn expected_output_tokens(self) -> usize {
        self.expected_output_tokens
    }
}

#[derive(Debug)]
struct WindowRequestStateV1 {
    binding: M1R33WindowRequestBindingV1,
    prompt_tokens: Box<[u32]>,
    arrival_offset_ns: u64,
    first_token_offset_ns: Option<u64>,
    terminal_offset_ns: Option<u64>,
    observed_output_tokens: usize,
}

/// One exact R33 collector request event.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct M1R33RequestEventV1 {
    pub arrival_offset_ns: u64,
    pub first_token_offset_ns: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub request_ordinal: u64,
    pub terminal_offset_ns: u64,
}

/// Completed monotonic-raw observation for one bounded window.
#[derive(Debug)]
pub struct M1R33WindowObservationV1 {
    duration_ns: u64,
    request_events: Box<[M1R33RequestEventV1]>,
}

impl M1R33WindowObservationV1 {
    #[must_use]
    pub const fn duration_ns(&self) -> u64 {
        self.duration_ns
    }

    #[must_use]
    pub fn request_events(&self) -> &[M1R33RequestEventV1] {
        &self.request_events
    }
}

/// Admission or observation failure for one bounded R33 window.
#[derive(Debug)]
pub enum M1R33WindowErrorV1 {
    Capacity,
    EmptyWindow,
    NonContiguousOrdinal {
        expected: usize,
        actual: usize,
    },
    PrefillPlan,
    Clock,
    ClockOrder,
    Engine {
        admitted: Box<[RequestId]>,
        source: EngineError,
    },
    Registry {
        admitted: Box<[RequestId]>,
        source: M1ServingRegistryErrorV1,
    },
    UnknownRequest,
    DuplicateTerminal,
    OutputAfterTerminal,
    EmptyOutputObservation,
    OutputTokenOutOfRange,
    OutputWorkExceeded,
    OutputWorkIncomplete,
    PhysicalSelectionMismatch,
    PhysicalEpochMismatch,
    PhysicalRosterMismatch,
    PhysicalRegistryMismatch,
    PhysicalOutputSemanticMismatch,
    IncompleteWindow,
    CounterOverflow,
}

/// One bounded R33 window admitted into Ferric's actual Engine and registry.
///
/// All arrivals are recorded before this value can be bound to physical
/// operations. Consequently, no API can add an arrival after publication.
#[must_use = "an admitted R33 window must be executed or explicitly abandoned"]
pub struct M1R33AdmittedWindowV1<const C: usize> {
    prefill: M1ServingPlanV1,
    registry: M1ServingRegistryV1<C>,
    requests: Vec<WindowRequestStateV1>,
    started_ns: u128,
}

impl<const C: usize> fmt::Debug for M1R33AdmittedWindowV1<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1R33AdmittedWindowV1")
            .field("request_count", &self.requests.len())
            .field("started_ns", &self.started_ns)
            .finish_non_exhaustive()
    }
}

impl<const C: usize> M1R33AdmittedWindowV1<C> {
    /// Validates all host work before mutating the exact ordered Engine roster.
    ///
    /// The supplied Engine must be a fresh instance dedicated to this window.
    /// If Engine or registry admission unexpectedly fails, the error reports all
    /// request IDs already admitted and the caller must abandon that Engine.
    ///
    /// # Errors
    ///
    /// Rejects work outside the fixed M1 envelope or any lower admission error.
    pub fn admit(
        engine: &mut Engine<C>,
        prefill: M1ServingPlanV1,
        requests: Vec<M1R33PretokenizedRequestV1>,
    ) -> Result<Self, M1R33WindowErrorV1> {
        let started_ns = monotonic_raw_ns()?;
        Self::admit_at(engine, prefill, requests, started_ns, monotonic_raw_ns)
    }

    fn admit_at(
        engine: &mut Engine<C>,
        prefill: M1ServingPlanV1,
        requests: Vec<M1R33PretokenizedRequestV1>,
        started_ns: u128,
        mut now: impl FnMut() -> Result<u128, M1R33WindowErrorV1>,
    ) -> Result<Self, M1R33WindowErrorV1> {
        if C == 0 || C > M1_MAX_ACTIVE_SEQUENCES as usize || requests.len() > C {
            return Err(M1R33WindowErrorV1::Capacity);
        }
        if requests.is_empty() {
            return Err(M1R33WindowErrorV1::EmptyWindow);
        }
        if prefill.mode() != Qwen3ExecutionMode::Prefill
            || prefill.sequence_capacity() < requests.len()
        {
            return Err(M1R33WindowErrorV1::PrefillPlan);
        }
        for (expected, request) in requests.iter().enumerate() {
            if request.request_ordinal != expected {
                return Err(M1R33WindowErrorV1::NonContiguousOrdinal {
                    expected,
                    actual: request.request_ordinal,
                });
            }
        }

        let mut registry =
            M1ServingRegistryV1::<C>::new().map_err(|source| M1R33WindowErrorV1::Registry {
                admitted: Box::new([]),
                source,
            })?;
        let mut admitted = Vec::new();
        admitted
            .try_reserve_exact(requests.len())
            .map_err(|_| M1R33WindowErrorV1::Capacity)?;
        let mut states = Vec::new();
        states
            .try_reserve_exact(requests.len())
            .map_err(|_| M1R33WindowErrorV1::Capacity)?;

        for work in requests {
            let request = engine
                .admit()
                .map_err(|source| M1R33WindowErrorV1::Engine {
                    admitted: admitted.clone().into_boxed_slice(),
                    source,
                })?;
            admitted.push(request);
            engine
                .append_tentative(request, 1)
                .map_err(|source| M1R33WindowErrorV1::Engine {
                    admitted: admitted.clone().into_boxed_slice(),
                    source,
                })?;
            registry
                .admit(request, prefill)
                .map_err(|source| M1R33WindowErrorV1::Registry {
                    admitted: admitted.clone().into_boxed_slice(),
                    source,
                })?;
            let arrival_offset_ns = elapsed_ns(started_ns, now()?)?;
            states.push(WindowRequestStateV1 {
                binding: M1R33WindowRequestBindingV1 {
                    request_ordinal: work.request_ordinal,
                    request,
                    input_tokens: work.prompt_tokens.len(),
                    expected_output_tokens: work.expected_output_tokens,
                },
                prompt_tokens: work.prompt_tokens,
                arrival_offset_ns,
                first_token_offset_ns: None,
                terminal_offset_ns: None,
                observed_output_tokens: 0,
            });
        }
        Ok(Self {
            prefill,
            registry,
            requests: states,
            started_ns,
        })
    }

    /// Exact request bindings in collector ordinal order.
    #[must_use]
    pub fn request_bindings(
        &self,
    ) -> impl ExactSizeIterator<Item = M1R33WindowRequestBindingV1> + '_ {
        self.requests.iter().map(|state| state.binding)
    }

    /// Exact tokenizer-admitted prompt retained for physical input construction.
    #[must_use]
    pub fn prompt_tokens(&self, request: RequestId) -> Option<&[u32]> {
        self.requests
            .iter()
            .find(|state| state.binding.request == request)
            .map(|state| state.prompt_tokens.as_ref())
    }

    /// Ferric's live registry for this exact bounded window.
    #[must_use]
    pub const fn registry(&self) -> &M1ServingRegistryV1<C> {
        &self.registry
    }

    #[cfg(test)]
    fn observe_output_at(
        &mut self,
        request: RequestId,
        tokens: &[u32],
        observed_ns: u128,
    ) -> Result<(), M1R33WindowErrorV1> {
        let offset = elapsed_ns(self.started_ns, observed_ns)?;
        let state = self.request_mut(request)?;
        let next_count = validate_output_observation(state, tokens, offset)?;
        if state.first_token_offset_ns.is_none() {
            state.first_token_offset_ns = Some(offset);
        }
        state.observed_output_tokens = next_count;
        Ok(())
    }

    fn observe_checked_completion_at(
        &mut self,
        batch: &M1ServingBatchPlanV1,
        checked: &M1CheckedCompletionOutputV1,
        observed_ns: u128,
    ) -> Result<(), M1R33WindowErrorV1> {
        if checked.selection() != batch.plan().target() {
            return Err(M1R33WindowErrorV1::PhysicalSelectionMismatch);
        }
        if checked.epoch() != batch.epoch() {
            return Err(M1R33WindowErrorV1::PhysicalEpochMismatch);
        }
        if checked.records().len() != batch.requests().len() {
            return Err(M1R33WindowErrorV1::PhysicalRosterMismatch);
        }
        for (index, request) in batch.requests().iter().copied().enumerate() {
            if batch.requests()[..index].contains(&request)
                || !self
                    .requests
                    .iter()
                    .any(|state| state.binding.request == request)
                || self.registry.plan(request) != Some(batch.plan())
                || self.registry.phase(request)
                    != Some(M1ServingRequestPhaseV1::InFlight {
                        epoch: batch.epoch(),
                    })
            {
                return Err(M1R33WindowErrorV1::PhysicalRegistryMismatch);
            }
        }
        let offset = elapsed_ns(self.started_ns, observed_ns)?;
        for (index, (expected_request, record)) in batch
            .requests()
            .iter()
            .copied()
            .zip(checked.records())
            .enumerate()
        {
            let observed = record.record();
            if observed.request != expected_request
                || observed.epoch != batch.epoch()
                || batch.requests()[..index].contains(&expected_request)
            {
                return Err(M1R33WindowErrorV1::PhysicalRosterMismatch);
            }
            let count = usize::from(observed.emitted_token_count);
            if count != record.semantics().externally_published_count() as usize {
                return Err(M1R33WindowErrorV1::PhysicalOutputSemanticMismatch);
            }
            let tokens = observed
                .emitted_tokens
                .get(..count)
                .ok_or(M1R33WindowErrorV1::PhysicalOutputSemanticMismatch)?;
            let state = self
                .requests
                .iter()
                .find(|state| state.binding.request == expected_request)
                .ok_or(M1R33WindowErrorV1::UnknownRequest)?;
            validate_output_observation(state, tokens, offset)?;
        }
        for (expected_request, record) in batch.requests().iter().copied().zip(checked.records()) {
            let observed = record.record();
            let count = usize::from(observed.emitted_token_count);
            let state = self.request_mut(expected_request)?;
            if state.first_token_offset_ns.is_none() {
                state.first_token_offset_ns = Some(offset);
            }
            state.observed_output_tokens += count;
        }
        Ok(())
    }

    fn observe_terminal_at(
        &mut self,
        request: RequestId,
        observed_ns: u128,
    ) -> Result<(), M1R33WindowErrorV1> {
        let offset = elapsed_ns(self.started_ns, observed_ns)?;
        let state = self.request_mut(request)?;
        if state.terminal_offset_ns.is_some() {
            return Err(M1R33WindowErrorV1::DuplicateTerminal);
        }
        if state.observed_output_tokens != state.binding.expected_output_tokens {
            return Err(M1R33WindowErrorV1::OutputWorkIncomplete);
        }
        let first = state
            .first_token_offset_ns
            .ok_or(M1R33WindowErrorV1::OutputWorkIncomplete)?;
        if offset <= first {
            return Err(M1R33WindowErrorV1::ClockOrder);
        }
        state.terminal_offset_ns = Some(offset);
        Ok(())
    }

    /// Seals a complete event population for the R33 collector.
    ///
    /// # Errors
    ///
    /// Rejects any request without exact output work and a terminal event.
    pub fn finish(&self) -> Result<M1R33WindowObservationV1, M1R33WindowErrorV1> {
        let finished_ns = monotonic_raw_ns()?;
        self.finish_at(finished_ns)
    }

    fn finish_at(&self, finished_ns: u128) -> Result<M1R33WindowObservationV1, M1R33WindowErrorV1> {
        let duration_ns = elapsed_ns(self.started_ns, finished_ns)?;
        let mut events = Vec::new();
        events
            .try_reserve_exact(self.requests.len())
            .map_err(|_| M1R33WindowErrorV1::Capacity)?;
        for state in &self.requests {
            let first = state
                .first_token_offset_ns
                .ok_or(M1R33WindowErrorV1::IncompleteWindow)?;
            let terminal = state
                .terminal_offset_ns
                .ok_or(M1R33WindowErrorV1::IncompleteWindow)?;
            if state.observed_output_tokens != state.binding.expected_output_tokens
                || terminal > duration_ns
            {
                return Err(M1R33WindowErrorV1::IncompleteWindow);
            }
            events.push(M1R33RequestEventV1 {
                arrival_offset_ns: state.arrival_offset_ns,
                first_token_offset_ns: first,
                input_tokens: u64::try_from(state.binding.input_tokens)
                    .map_err(|_| M1R33WindowErrorV1::CounterOverflow)?,
                output_tokens: u64::try_from(state.observed_output_tokens)
                    .map_err(|_| M1R33WindowErrorV1::CounterOverflow)?,
                request_ordinal: u64::try_from(state.binding.request_ordinal)
                    .map_err(|_| M1R33WindowErrorV1::CounterOverflow)?,
                terminal_offset_ns: terminal,
            });
        }
        Ok(M1R33WindowObservationV1 {
            duration_ns,
            request_events: events.into_boxed_slice(),
        })
    }

    /// Binds the admitted window to Ferric's concrete production physical path.
    ///
    /// The returned owner borrows one live Engine and physical runner and owns
    /// the exact queued physical-input provider. It is intentionally not a mock
    /// execution trait. The caller must keep this owner alive for the whole
    /// window and drive the existing registry/physical typestate APIs.
    ///
    /// # Errors
    ///
    /// Rejects any first-generation plan, roster, or prompt substitution before
    /// consuming the provider. The provider is retained on these preflight
    /// failures. The lower constructor consumes it only if its process-local
    /// identity space is exhausted after preflight.
    pub fn bind_physical<'a>(
        self,
        runner: &'a M1PhysicalRunnerV1,
        engine: &'a mut Engine<C>,
        provider: M1QueuedServingPhysicalInputProviderV1,
        ring_bytes: u32,
    ) -> Result<M1R33PhysicalWindowV1<'a, C>, Box<M1R33PhysicalWindowBindFailureV1<C>>> {
        let mut requests = Vec::new();
        let mut prompt_tokens = Vec::new();
        if requests.try_reserve_exact(self.requests.len()).is_err()
            || prompt_tokens
                .try_reserve_exact(self.requests.len())
                .is_err()
        {
            return Err(Box::new(M1R33PhysicalWindowBindFailureV1 {
                window: self,
                provider: Some(provider),
                source: M1R33PhysicalWindowBindErrorV1::Capacity,
            }));
        }
        requests.extend(self.requests.iter().map(|state| state.binding.request));
        prompt_tokens.extend(
            self.requests
                .iter()
                .map(|state| state.prompt_tokens.as_ref()),
        );
        if let Err(source) =
            provider.preflight_first_publication_work(self.prefill, &requests, &prompt_tokens)
        {
            return Err(Box::new(M1R33PhysicalWindowBindFailureV1 {
                window: self,
                provider: Some(provider),
                source: M1R33PhysicalWindowBindErrorV1::FirstPublicationWork(source),
            }));
        }
        match M1ServingPhysicalRunnerOperationsV1::new(runner, engine, provider, ring_bytes) {
            Ok(operations) => Ok(M1R33PhysicalWindowV1 {
                window: self,
                operations,
            }),
            Err(source) => Err(Box::new(M1R33PhysicalWindowBindFailureV1 {
                window: self,
                provider: None,
                source: M1R33PhysicalWindowBindErrorV1::Operations(source),
            })),
        }
    }

    fn request_mut(
        &mut self,
        request: RequestId,
    ) -> Result<&mut WindowRequestStateV1, M1R33WindowErrorV1> {
        self.requests
            .iter_mut()
            .find(|state| state.binding.request == request)
            .ok_or(M1R33WindowErrorV1::UnknownRequest)
    }
}

/// Concrete physical binding for one already-admitted R33 window.
///
/// This is the long-lived in-process owner that an independently supervised
/// adapter service retains. It does not expose an API for adding requests.
pub struct M1R33PhysicalWindowV1<'a, const C: usize> {
    window: M1R33AdmittedWindowV1<C>,
    operations: M1ServingPhysicalRunnerOperationsV1<'a, C, M1QueuedServingPhysicalInputProviderV1>,
}

impl<const C: usize> fmt::Debug for M1R33PhysicalWindowV1<'_, C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1R33PhysicalWindowV1")
            .field("window", &self.window)
            .field("provider_present", &self.operations.provider().is_some())
            .finish_non_exhaustive()
    }
}

impl<'a, const C: usize> M1R33PhysicalWindowV1<'a, C> {
    pub const fn window(&self) -> &M1R33AdmittedWindowV1<C> {
        &self.window
    }

    /// Mutable event/controller access for checked physical output callbacks.
    pub const fn window_mut(&mut self) -> &mut M1R33AdmittedWindowV1<C> {
        &mut self.window
    }

    /// Mutable registry access used by the existing publication bridge.
    #[must_use]
    pub const fn registry_mut(&mut self) -> &mut M1ServingRegistryV1<C> {
        &mut self.window.registry
    }

    /// Concrete physical operations over the same live Engine and runner.
    #[must_use]
    pub const fn operations_mut(
        &mut self,
    ) -> &mut M1ServingPhysicalRunnerOperationsV1<'a, C, M1QueuedServingPhysicalInputProviderV1>
    {
        &mut self.operations
    }

    /// Records one generation from readback custody owned by these operations.
    ///
    /// Adapter identity and physical phase are checked before the window checks
    /// its current registry plan, in-flight epoch, ordered roster, semantic
    /// output count, tokens, timing, and remaining declared work.
    ///
    /// # Errors
    ///
    /// Rejects unrelated physical custody or any logical/event mismatch before
    /// changing the request event population.
    pub fn observe_physical_readback(
        &mut self,
        readback: &M1ServingPhysicalReadbackV1<M1ServingPhysicalRunnerReadbackV1>,
    ) -> Result<(), M1R33PhysicalObservationErrorV1> {
        let observed_ns = monotonic_raw_ns().map_err(M1R33PhysicalObservationErrorV1::Window)?;
        let checked = self
            .operations
            .checked_completion_for_readback(readback)
            .map_err(M1R33PhysicalObservationErrorV1::Physical)?;
        self.window
            .observe_checked_completion_at(readback.batch(), checked, observed_ns)
            .map_err(M1R33PhysicalObservationErrorV1::Window)
    }

    /// Records terminal timing only after physical settlement retired the request.
    ///
    /// # Errors
    ///
    /// Rejects a request not retired through an exact completed epoch, or any
    /// incomplete/duplicate/non-monotonic terminal event.
    pub fn observe_terminal_after_settlement(
        &mut self,
        request: RequestId,
    ) -> Result<(), M1R33PhysicalObservationErrorV1> {
        if !matches!(
            self.window.registry.phase(request),
            Some(M1ServingRequestPhaseV1::Retired {
                quiescence: M1ServingQuiescenceV1::Completed(_),
            })
        ) {
            return Err(M1R33PhysicalObservationErrorV1::Window(
                M1R33WindowErrorV1::PhysicalRegistryMismatch,
            ));
        }
        let observed_ns = monotonic_raw_ns().map_err(M1R33PhysicalObservationErrorV1::Window)?;
        self.window
            .observe_terminal_at(request, observed_ns)
            .map_err(M1R33PhysicalObservationErrorV1::Window)
    }

    /// Produces the complete collector event population without consuming operations.
    ///
    /// # Errors
    ///
    /// Rejects a window with incomplete output, terminal, or duration state.
    pub fn finish_observation(&self) -> Result<M1R33WindowObservationV1, M1R33WindowErrorV1> {
        self.window.finish()
    }
}

/// Exact layer that rejected a physical readback observation.
#[derive(Debug)]
pub enum M1R33PhysicalObservationErrorV1 {
    Physical(M1ServingPhysicalRunnerOperationErrorV1),
    Window(M1R33WindowErrorV1),
}

/// Physical-operation binding failure retaining logical window custody.
#[derive(Debug)]
pub struct M1R33PhysicalWindowBindFailureV1<const C: usize> {
    window: M1R33AdmittedWindowV1<C>,
    provider: Option<M1QueuedServingPhysicalInputProviderV1>,
    source: M1R33PhysicalWindowBindErrorV1,
}

/// Stable reason that exact physical binding failed before publication.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1R33PhysicalWindowBindErrorV1 {
    Capacity,
    FirstPublicationWork(M1ServingFirstPublicationWorkMatchErrorV1),
    Operations(M1ServingPhysicalRunnerOperationsCreateErrorV1),
}

impl<const C: usize> M1R33PhysicalWindowBindFailureV1<C> {
    #[must_use]
    pub const fn source(&self) -> M1R33PhysicalWindowBindErrorV1 {
        self.source
    }

    /// Recovers the admitted window and any provider not consumed by the lower constructor.
    #[must_use = "the admitted window and queued provider remain owned"]
    pub fn into_parts(
        self,
    ) -> (
        M1R33AdmittedWindowV1<C>,
        Option<M1QueuedServingPhysicalInputProviderV1>,
    ) {
        (self.window, self.provider)
    }
}

fn monotonic_raw_ns() -> Result<u128, M1R33WindowErrorV1> {
    let timestamp = clock_gettime(ClockId::MonotonicRaw);
    let seconds = u128::try_from(timestamp.tv_sec).map_err(|_| M1R33WindowErrorV1::Clock)?;
    let nanoseconds = u128::try_from(timestamp.tv_nsec).map_err(|_| M1R33WindowErrorV1::Clock)?;
    seconds
        .checked_mul(1_000_000_000)
        .and_then(|value| value.checked_add(nanoseconds))
        .ok_or(M1R33WindowErrorV1::Clock)
}

fn elapsed_ns(started_ns: u128, observed_ns: u128) -> Result<u64, M1R33WindowErrorV1> {
    let elapsed = observed_ns
        .checked_sub(started_ns)
        .ok_or(M1R33WindowErrorV1::ClockOrder)?;
    u64::try_from(elapsed).map_err(|_| M1R33WindowErrorV1::Clock)
}

fn validate_output_observation(
    state: &WindowRequestStateV1,
    tokens: &[u32],
    offset: u64,
) -> Result<usize, M1R33WindowErrorV1> {
    if tokens.is_empty() {
        return Err(M1R33WindowErrorV1::EmptyOutputObservation);
    }
    if tokens.iter().any(|token| *token >= QWEN3_VOCABULARY_SIZE) {
        return Err(M1R33WindowErrorV1::OutputTokenOutOfRange);
    }
    if state.terminal_offset_ns.is_some() {
        return Err(M1R33WindowErrorV1::OutputAfterTerminal);
    }
    let next_count = state
        .observed_output_tokens
        .checked_add(tokens.len())
        .ok_or(M1R33WindowErrorV1::CounterOverflow)?;
    if next_count > state.binding.expected_output_tokens {
        return Err(M1R33WindowErrorV1::OutputWorkExceeded);
    }
    if state.first_token_offset_ns.is_none() && offset <= state.arrival_offset_ns {
        return Err(M1R33WindowErrorV1::ClockOrder);
    }
    Ok(next_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferric_spec::{Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection};

    fn selection(
        role: Qwen3ModelRole,
        mode: Qwen3ExecutionMode,
        bucket: Qwen3PlanBucket,
    ) -> Qwen3PlanSelection {
        Qwen3PlanSelection { role, mode, bucket }
    }

    fn prefill() -> M1ServingPlanV1 {
        M1ServingPlanV1::new(
            selection(
                Qwen3ModelRole::Target8B,
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS8T128,
            ),
            selection(
                Qwen3ModelRole::Draft06B,
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS8T128,
            ),
        )
        .unwrap()
    }

    fn work(ordinal: usize, input: usize, output: usize) -> M1R33PretokenizedRequestV1 {
        M1R33PretokenizedRequestV1::new(ordinal, vec![17; input].into_boxed_slice(), output)
            .unwrap()
    }

    fn engine() -> Engine<8> {
        Engine::new(32, 256, 8192).unwrap()
    }

    #[test]
    fn pretokenized_work_fails_closed_at_every_bound() {
        assert_eq!(
            M1R33PretokenizedRequestV1::new(0, Box::new([]), 2).unwrap_err(),
            M1R33PretokenizedRequestErrorV1::EmptyPrompt
        );
        assert_eq!(
            M1R33PretokenizedRequestV1::new(0, vec![QWEN3_VOCABULARY_SIZE].into_boxed_slice(), 2,)
                .unwrap_err(),
            M1R33PretokenizedRequestErrorV1::TokenOutOfRange
        );
        assert_eq!(
            M1R33PretokenizedRequestV1::new(0, vec![1].into_boxed_slice(), 1).unwrap_err(),
            M1R33PretokenizedRequestErrorV1::OutputTooShort
        );
        assert_eq!(
            M1R33PretokenizedRequestV1::new(
                0,
                vec![1; M1_MAX_CONTEXT_TOKENS as usize].into_boxed_slice(),
                2,
            )
            .unwrap_err(),
            M1R33PretokenizedRequestErrorV1::ContextExceeded
        );
    }

    #[test]
    fn bounded_window_binds_ordered_engine_and_registry_requests() {
        let mut engine = engine();
        let mut timestamps = [1_000_000_010_u128, 1_000_000_020].into_iter();
        let window = M1R33AdmittedWindowV1::admit_at(
            &mut engine,
            prefill(),
            vec![work(0, 3, 2), work(1, 4, 3)],
            1_000_000_000,
            || Ok(timestamps.next().unwrap()),
        )
        .unwrap();
        let bindings = window.request_bindings().collect::<Vec<_>>();
        assert_eq!(bindings.len(), 2);
        assert_eq!(bindings[0].request_ordinal(), 0);
        assert_eq!(bindings[0].input_tokens(), 3);
        assert_eq!(bindings[0].expected_output_tokens(), 2);
        assert_eq!(bindings[1].request_ordinal(), 1);
        assert_eq!(
            window.registry().phase(bindings[0].request()).unwrap(),
            ferric_engine::M1ServingRequestPhaseV1::Ready
        );
    }

    #[test]
    fn event_population_is_exact_ordered_and_collector_compatible() {
        let mut engine = engine();
        let mut arrivals = [10_010_u128, 10_020].into_iter();
        let mut window = M1R33AdmittedWindowV1::admit_at(
            &mut engine,
            prefill(),
            vec![work(0, 3, 2), work(1, 4, 3)],
            10_000,
            || Ok(arrivals.next().unwrap()),
        )
        .unwrap();
        let bindings = window.request_bindings().collect::<Vec<_>>();
        window
            .observe_output_at(bindings[0].request(), &[5], 10_100)
            .unwrap();
        window
            .observe_output_at(bindings[1].request(), &[6, 7], 10_110)
            .unwrap();
        window
            .observe_output_at(bindings[0].request(), &[8], 10_200)
            .unwrap();
        window
            .observe_output_at(bindings[1].request(), &[9], 10_220)
            .unwrap();
        window
            .observe_terminal_at(bindings[0].request(), 10_300)
            .unwrap();
        window
            .observe_terminal_at(bindings[1].request(), 10_320)
            .unwrap();
        let observation = window.finish_at(10_400).unwrap();
        assert_eq!(observation.duration_ns(), 400);
        assert_eq!(
            observation.request_events(),
            &[
                M1R33RequestEventV1 {
                    arrival_offset_ns: 10,
                    first_token_offset_ns: 100,
                    input_tokens: 3,
                    output_tokens: 2,
                    request_ordinal: 0,
                    terminal_offset_ns: 300,
                },
                M1R33RequestEventV1 {
                    arrival_offset_ns: 20,
                    first_token_offset_ns: 110,
                    input_tokens: 4,
                    output_tokens: 3,
                    request_ordinal: 1,
                    terminal_offset_ns: 320,
                },
            ]
        );
    }

    #[test]
    fn hostile_event_mutations_are_rejected_without_state_advance() {
        let mut engine = engine();
        let mut window = M1R33AdmittedWindowV1::admit_at(
            &mut engine,
            prefill(),
            vec![work(0, 2, 2)],
            50,
            || Ok(60),
        )
        .unwrap();
        let request = window.request_bindings().next().unwrap().request();
        assert!(matches!(
            window.observe_output_at(request, &[], 70),
            Err(M1R33WindowErrorV1::EmptyOutputObservation)
        ));
        assert!(matches!(
            window.observe_output_at(request, &[1], 60),
            Err(M1R33WindowErrorV1::ClockOrder)
        ));
        window.observe_output_at(request, &[1], 70).unwrap();
        assert!(matches!(
            window.observe_terminal_at(request, 80),
            Err(M1R33WindowErrorV1::OutputWorkIncomplete)
        ));
        assert!(matches!(
            window.observe_output_at(request, &[2, 3], 90),
            Err(M1R33WindowErrorV1::OutputWorkExceeded)
        ));
        window.observe_output_at(request, &[2], 90).unwrap();
        window.observe_terminal_at(request, 100).unwrap();
        assert!(matches!(
            window.observe_output_at(request, &[3], 110),
            Err(M1R33WindowErrorV1::OutputAfterTerminal)
        ));
        assert!(matches!(
            window.observe_terminal_at(request, 110),
            Err(M1R33WindowErrorV1::DuplicateTerminal)
        ));
    }

    #[test]
    fn admission_rejects_late_shape_and_ordinal_drift_before_engine_mutation() {
        let mut engine = engine();
        assert!(matches!(
            M1R33AdmittedWindowV1::admit_at(&mut engine, prefill(), vec![work(1, 2, 2)], 0, || Ok(
                1
            ),),
            Err(M1R33WindowErrorV1::NonContiguousOrdinal {
                expected: 0,
                actual: 1
            })
        ));
        assert_eq!(engine.admit().unwrap().slot(), 0);
    }
}
