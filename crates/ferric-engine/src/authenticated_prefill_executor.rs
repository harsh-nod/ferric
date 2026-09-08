//! Production authenticated execution of the first paired-prefill generation.
//!
//! This boundary consumes the authenticated S1/T128 prepublication owner,
//! executes exactly one paired-prefill queue generation, joins the compact
//! completion to independently read direct-choice evidence, settles Engine and
//! device-KV state, and releases completed prefill pages. Success is custody
//! suitable for the later speculative-rollover slice. This module does not
//! execute rollover, observe clocks, publish R33 evidence, or claim serving.

use core::fmt;

use ferric_spec::{RequestId, TokenId};

use crate::{
    complete_m1_authenticated_physical_step_v1,
    release_m1_authenticated_completed_step_kv_pages_v1, DeviceKvPageLease, Engine,
    M1AuthenticatedCompletedStepOutcomeV1, M1AuthenticatedPhysicalQueueSessionV1,
    M1AuthenticatedReleasedCompletedStepV1, M1AuthenticatedS1T128PrefillPrepublicationV1,
    M1AuthenticatedSpeculativeRolloverIntentV1, M1CaptureQuarantinedEngineV1,
    M1DeviceKvCompletionMemberV1, M1DeviceKvCompletionRosterV1,
    M1ObservedDirectDiagnosticChoicesV1, M1QueueWaitTimeoutV1, M1SpeculativeGenerationPolicyV1,
};

/// Exact execution phase at which the first authenticated prefill stopped.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedS1T128PrefillExecutionStageV1 {
    EnginePreflight,
    QueueCreate,
    QueueSubmit,
    QueueWait,
    QueueRecycle,
    CompletionObservation,
    DirectChoiceObservation,
    DirectCompletionCheck,
    DirectChoiceCardinality,
    PhysicalCompletion,
    PrefillPageRelease,
}

/// Stable high-level reason for terminal authenticated prefill failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedS1T128PrefillExecutionErrorV1 {
    EngineFaulted,
    DeadlineExpired,
    LowerRejected,
    HostAllocation,
    DirectChoiceCardinality { expected: usize, actual: usize },
    PhysicalCompletionPoisoned,
}

#[derive(Debug)]
struct M1AuthenticatedS1T128PrefillSuccessorCustodyV1 {
    draft_rollover_page: DeviceKvPageLease,
    target_rollover_pages: Vec<DeviceKvPageLease>,
    rollover_intent: M1AuthenticatedSpeculativeRolloverIntentV1,
    request: RequestId,
    prompt_tokens: Box<[TokenId]>,
    policy: M1SpeculativeGenerationPolicyV1,
    diagnostic_ring_bytes: u32,
    queue_wait_timeout: M1QueueWaitTimeoutV1,
}

struct OpaqueM1AuthenticatedS1T128PrefillExecutionCustodyV1(Box<dyn fmt::Debug>);

/// Terminal failure retaining the quarantined Engine and every remaining owner.
///
/// No live queue, prepublication, completed-step, or retry owner can be
/// recovered from this boundary:
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedS1T128PrefillExecutionFailureV1;
/// fn recover_live(failure: M1AuthenticatedS1T128PrefillExecutionFailureV1<1>) {
///     let _ = failure.into_prepublication();
/// }
/// ```
#[must_use = "terminal authenticated prefill custody must be retained"]
pub struct M1AuthenticatedS1T128PrefillExecutionFailureV1<const C: usize> {
    stage: M1AuthenticatedS1T128PrefillExecutionStageV1,
    error: M1AuthenticatedS1T128PrefillExecutionErrorV1,
    engine: M1CaptureQuarantinedEngineV1<C>,
    queue_status:
        crate::authenticated_resident_session::M1AuthenticatedResidentQueueTeardownStatusV1,
    retained: OpaqueM1AuthenticatedS1T128PrefillExecutionCustodyV1,
}

impl<const C: usize> fmt::Debug for M1AuthenticatedS1T128PrefillExecutionFailureV1<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedS1T128PrefillExecutionFailureV1")
            .field("stage", &self.stage)
            .field("error", &self.error)
            .field("engine_quarantined", &self.engine.is_faulted())
            .field("queue_status", &self.queue_status)
            .field("retained", &self.retained.0)
            .finish_non_exhaustive()
    }
}

impl<const C: usize> M1AuthenticatedS1T128PrefillExecutionFailureV1<C> {
    #[must_use]
    pub const fn stage(&self) -> M1AuthenticatedS1T128PrefillExecutionStageV1 {
        self.stage
    }

    #[must_use]
    pub const fn error(&self) -> M1AuthenticatedS1T128PrefillExecutionErrorV1 {
        self.error
    }

    /// All failures consume the scheduler into terminal quarantine.
    #[must_use]
    pub const fn engine_quarantined(&self) -> bool {
        true
    }

    /// Borrows retained custody for diagnostics without recovering its owners.
    #[must_use]
    pub fn retained_debug(&self) -> &dyn fmt::Debug {
        self.retained.0.as_ref()
    }

    pub(crate) fn into_resident_teardown(
        self: Box<Self>,
    ) -> crate::authenticated_resident_session::M1AuthenticatedResidentQueueTeardownV1 {
        use crate::authenticated_resident_session::{
            M1AuthenticatedResidentQueueTeardownStatusV1 as Status,
            M1AuthenticatedResidentQueueTeardownV1 as Teardown,
        };
        match self.queue_status {
            Status::NoQueue => Teardown::no_queue(self),
            Status::Released => Teardown::released(self),
            Status::Quarantined => Teardown::quarantined(self),
        }
    }

    /// Separates terminal Engine quarantine from opaque retained custody.
    #[must_use = "Engine quarantine and retained custody both remain terminal"]
    pub fn into_parts(
        self,
    ) -> (
        M1CaptureQuarantinedEngineV1<C>,
        M1AuthenticatedS1T128PrefillExecutionStageV1,
        M1AuthenticatedS1T128PrefillExecutionErrorV1,
        Box<dyn fmt::Debug>,
    ) {
        (self.engine, self.stage, self.error, self.retained.0)
    }
}

fn terminal_failure<const C: usize>(
    engine: Engine<C>,
    stage: M1AuthenticatedS1T128PrefillExecutionStageV1,
    error: M1AuthenticatedS1T128PrefillExecutionErrorV1,
    queue_status: crate::authenticated_resident_session::M1AuthenticatedResidentQueueTeardownStatusV1,
    retained: impl fmt::Debug + 'static,
) -> Box<M1AuthenticatedS1T128PrefillExecutionFailureV1<C>> {
    Box::new(M1AuthenticatedS1T128PrefillExecutionFailureV1 {
        stage,
        error,
        engine: engine.into_m1_capture_quarantine(),
        queue_status,
        retained: OpaqueM1AuthenticatedS1T128PrefillExecutionCustodyV1(Box::new(retained)),
    })
}

fn physical_closure_status(
    closure: &crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1,
) -> crate::authenticated_resident_session::M1AuthenticatedResidentQueueTeardownStatusV1 {
    use crate::{
        authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1,
        authenticated_resident_session::M1AuthenticatedResidentQueueTeardownStatusV1,
    };
    match closure {
        M1AuthenticatedPhysicalQueueClosureV1::Released(_) => {
            M1AuthenticatedResidentQueueTeardownStatusV1::Released
        }
        M1AuthenticatedPhysicalQueueClosureV1::Quarantined(_) => {
            M1AuthenticatedResidentQueueTeardownStatusV1::Quarantined
        }
    }
}

fn teardown_result_status<T, E>(
    result: &Result<T, E>,
) -> crate::authenticated_resident_session::M1AuthenticatedResidentQueueTeardownStatusV1 {
    use crate::authenticated_resident_session::M1AuthenticatedResidentQueueTeardownStatusV1;
    if result.is_ok() {
        M1AuthenticatedResidentQueueTeardownStatusV1::Released
    } else {
        M1AuthenticatedResidentQueueTeardownStatusV1::Quarantined
    }
}

fn completed_step_closure_status(
    closure: &crate::m1_completed_step::M1AuthenticatedCompletedStepClosureV1,
) -> crate::authenticated_resident_session::M1AuthenticatedResidentQueueTeardownStatusV1 {
    use crate::{
        authenticated_resident_session::M1AuthenticatedResidentQueueTeardownStatusV1,
        m1_completed_step::M1AuthenticatedCompletedStepClosureV1,
    };
    match closure {
        M1AuthenticatedCompletedStepClosureV1::Released(_) => {
            M1AuthenticatedResidentQueueTeardownStatusV1::Released
        }
        M1AuthenticatedCompletedStepClosureV1::Quarantined(_) => {
            M1AuthenticatedResidentQueueTeardownStatusV1::Quarantined
        }
    }
}

fn close_prefill_observed<const C: usize>(
    engine: &mut Engine<C>,
    observed: crate::M1AuthenticatedObservedCompletionOutputV1,
) -> crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1 {
    match observed.check_completion(&[]) {
        Ok(readback) => close_prefill_readback(engine, readback),
        Err(failure) => {
            match failure.destroy_queue_and_retain_evidence(engine) {
                Ok(released) => crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1::Released(Box::new(released)),
                Err(quarantined) => crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1::Quarantined(Box::new(quarantined)),
            }
        }
    }
}

fn close_prefill_readback<const C: usize>(
    engine: &mut Engine<C>,
    readback: crate::M1AuthenticatedPhysicalCompletedReadbackV1,
) -> crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1 {
    engine.quarantine_m1_queue_rearm_failure();
    let (queue, checked, completion, kv) = readback.into_parts();
    crate::authenticated_physical_queue::retain_in_queue_closure(
        match queue.destroy_and_release() {
            Ok(released) => crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1::Released(Box::new(released)),
            Err(quarantined) => crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1::Quarantined(quarantined),
        },
        (checked, completion, kv),
    )
}

fn close_prefill_direct_observed<const C: usize>(
    engine: &mut Engine<C>,
    direct: crate::M1AuthenticatedObservedDirectDiagnosticOutputV1,
) -> Result<Box<dyn fmt::Debug>, Box<dyn fmt::Debug>> {
    match direct.check_completion() {
        Ok(completed) => match completed.destroy_queue_and_retain_evidence(engine) {
            Ok(released) => Ok(Box::new(released)),
            Err(quarantined) => Err(quarantined),
        },
        Err(failure) => match (*failure).destroy_queue_and_retain_evidence(engine) {
            Ok(released) => Ok(Box::new(released)),
            Err(quarantined) => Err(quarantined),
        },
    }
}

fn require_one_authenticated_prefill_choice(
    choices: &M1ObservedDirectDiagnosticChoicesV1,
) -> Result<TokenId, M1AuthenticatedS1T128PrefillExecutionErrorV1> {
    let [choice] = choices.choices() else {
        return Err(
            M1AuthenticatedS1T128PrefillExecutionErrorV1::DirectChoiceCardinality {
                expected: 1,
                actual: choices.choices().len(),
            },
        );
    };
    Ok(*choice)
}

/// Rollover-ready custody after one exact authenticated paired-prefill step.
///
/// The first token is copied only from authenticated direct-choice evidence.
/// The returned Engine and released step are the exact pair consumed by the
/// authenticated speculative-rollover API.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedS1T128PrefillExecutionSuccessV1;
/// fn split_twice(success: M1AuthenticatedS1T128PrefillExecutionSuccessV1<1>) {
///     let _first = success.into_parts();
///     let _second = success.into_parts();
/// }
/// ```
#[must_use = "rollover-ready authenticated prefill custody remains linear"]
pub struct M1AuthenticatedS1T128PrefillExecutionSuccessV1<const C: usize> {
    engine: Engine<C>,
    released: M1AuthenticatedReleasedCompletedStepV1,
    first_token: TokenId,
    direct_choices: M1ObservedDirectDiagnosticChoicesV1,
    successor: M1AuthenticatedS1T128PrefillSuccessorCustodyV1,
}

impl<const C: usize> fmt::Debug for M1AuthenticatedS1T128PrefillExecutionSuccessV1<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedS1T128PrefillExecutionSuccessV1")
            .field("engine_faulted", &self.engine.is_faulted())
            .field("first_token", &self.first_token)
            .field("direct_choices", &self.direct_choices)
            .field("successor", &self.successor)
            .finish_non_exhaustive()
    }
}

impl<const C: usize> M1AuthenticatedS1T128PrefillExecutionSuccessV1<C> {
    #[must_use]
    pub const fn engine(&self) -> &Engine<C> {
        &self.engine
    }

    #[must_use = "released authenticated queue and device-KV custody remain retained"]
    pub const fn released(&self) -> &M1AuthenticatedReleasedCompletedStepV1 {
        &self.released
    }

    #[must_use]
    pub const fn first_token(&self) -> TokenId {
        self.first_token
    }

    #[must_use = "authenticated direct-choice evidence remains retained"]
    pub const fn direct_choices(&self) -> &M1ObservedDirectDiagnosticChoicesV1 {
        &self.direct_choices
    }

    #[must_use]
    pub const fn request(&self) -> RequestId {
        self.successor.request
    }

    #[must_use]
    pub fn prompt_tokens(&self) -> &[TokenId] {
        &self.successor.prompt_tokens
    }

    #[must_use]
    pub const fn generation_policy(&self) -> M1SpeculativeGenerationPolicyV1 {
        self.successor.policy
    }

    #[must_use = "draft rollover page custody remains retained"]
    pub const fn draft_rollover_page(&self) -> &DeviceKvPageLease {
        &self.successor.draft_rollover_page
    }

    #[must_use = "target rollover page custody remains retained"]
    pub fn target_rollover_page(&self) -> Option<&DeviceKvPageLease> {
        self.successor.target_rollover_pages.first()
    }

    /// Exact target page tail preleased before model memory entered queue custody.
    #[must_use = "target rollover page custody remains retained"]
    pub fn target_rollover_pages(&self) -> &[DeviceKvPageLease] {
        &self.successor.target_rollover_pages
    }

    #[must_use = "authenticated rollover intent remains retained"]
    pub const fn rollover_intent(&self) -> &M1AuthenticatedSpeculativeRolloverIntentV1 {
        &self.successor.rollover_intent
    }

    /// Inert queue configuration retained for the later rollover slice.
    #[must_use]
    pub const fn diagnostic_ring_bytes(&self) -> u32 {
        self.successor.diagnostic_ring_bytes
    }

    /// Inert bounded-wait configuration retained for the later rollover slice.
    #[must_use]
    pub const fn queue_wait_timeout(&self) -> M1QueueWaitTimeoutV1 {
        self.successor.queue_wait_timeout
    }

    pub(crate) fn permits_speculative_successor(&self) -> bool {
        self.successor.policy.permits_fresh_anchor(self.first_token)
    }

    pub(crate) fn close_for_resident(
        self,
    ) -> crate::authenticated_resident_session::M1AuthenticatedResidentQueueTeardownV1 {
        use crate::authenticated_resident_session::M1AuthenticatedResidentQueueTeardownV1 as Teardown;
        let Self {
            mut engine,
            released,
            first_token,
            direct_choices,
            successor,
        } = self;
        match released.destroy_queue_and_retain_step(&mut engine) {
            Ok(closed) => Teardown::released((
                engine.into_m1_capture_quarantine(),
                closed,
                first_token,
                direct_choices,
                successor,
            )),
            Err(quarantined) => Teardown::quarantined((
                engine.into_m1_capture_quarantine(),
                quarantined,
                first_token,
                direct_choices,
                successor,
            )),
        }
    }

    /// Separates the exact Engine/released pair and all successor custody once.
    #[must_use = "all rollover-ready authenticated owners remain linear"]
    #[allow(clippy::type_complexity)]
    pub fn into_parts(
        self,
    ) -> (
        Engine<C>,
        M1AuthenticatedReleasedCompletedStepV1,
        TokenId,
        M1ObservedDirectDiagnosticChoicesV1,
        DeviceKvPageLease,
        Vec<DeviceKvPageLease>,
        M1AuthenticatedSpeculativeRolloverIntentV1,
        RequestId,
        Box<[TokenId]>,
        M1SpeculativeGenerationPolicyV1,
        u32,
        M1QueueWaitTimeoutV1,
    ) {
        let Self {
            engine,
            released,
            first_token,
            direct_choices,
            successor:
                M1AuthenticatedS1T128PrefillSuccessorCustodyV1 {
                    draft_rollover_page,
                    target_rollover_pages,
                    rollover_intent,
                    request,
                    prompt_tokens,
                    policy,
                    diagnostic_ring_bytes,
                    queue_wait_timeout,
                },
        } = self;
        (
            engine,
            released,
            first_token,
            direct_choices,
            draft_rollover_page,
            target_rollover_pages,
            rollover_intent,
            request,
            prompt_tokens,
            policy,
            diagnostic_ring_bytes,
            queue_wait_timeout,
        )
    }
}

/// Executes one exact authenticated S1/T128 paired-prefill generation.
///
/// The diagnostic ring byte count and wait timeout are configuration only;
/// neither grants token, completion, queue, or timing authority. Success is not
/// a speculative-rollover execution, latency observation, R33 report, or HTTP
/// serving claim.
///
/// # Errors
///
/// Every rejection consumes the Engine into terminal quarantine, attempts to
/// destroy any recoverable live queue, and retains all remaining owners behind
/// opaque diagnostic custody. No failure can resume through a structural path.
#[allow(clippy::too_many_lines)]
pub fn execute_m1_authenticated_s1_t128_paired_prefill_v1<const C: usize>(
    prepared: M1AuthenticatedS1T128PrefillPrepublicationV1<C>,
    diagnostic_ring_bytes: u32,
    queue_wait_timeout: M1QueueWaitTimeoutV1,
) -> Result<
    M1AuthenticatedS1T128PrefillExecutionSuccessV1<C>,
    Box<M1AuthenticatedS1T128PrefillExecutionFailureV1<C>>,
> {
    execute_m1_authenticated_s1_t128_paired_prefill_with_deadline_v1(
        prepared,
        diagnostic_ring_bytes,
        queue_wait_timeout,
        |_, configured| Some(configured),
    )
}

#[allow(clippy::too_many_lines)]
pub(crate) fn execute_m1_authenticated_s1_t128_paired_prefill_with_deadline_v1<const C: usize>(
    prepared: M1AuthenticatedS1T128PrefillPrepublicationV1<C>,
    diagnostic_ring_bytes: u32,
    queue_wait_timeout: M1QueueWaitTimeoutV1,
    mut deadline_expired: impl FnMut(
        crate::authenticated_resident_session::M1AuthenticatedResidentDeadlineBoundaryV1,
        M1QueueWaitTimeoutV1,
    ) -> Option<M1QueueWaitTimeoutV1>,
) -> Result<
    M1AuthenticatedS1T128PrefillExecutionSuccessV1<C>,
    Box<M1AuthenticatedS1T128PrefillExecutionFailureV1<C>>,
> {
    use crate::authenticated_resident_session::{
        M1AuthenticatedResidentDeadlineBoundaryV1 as Boundary,
        M1AuthenticatedResidentQueueTeardownStatusV1 as Status,
    };

    let (
        mut engine,
        prepublication,
        cache,
        draft_rollover_page,
        target_rollover_pages,
        rollover_intent,
        request,
        prompt_tokens,
        policy,
    ) = prepared.into_parts();
    let successor = M1AuthenticatedS1T128PrefillSuccessorCustodyV1 {
        draft_rollover_page,
        target_rollover_pages,
        rollover_intent,
        request,
        prompt_tokens,
        policy,
        diagnostic_ring_bytes,
        queue_wait_timeout,
    };
    if engine.is_faulted() {
        return Err(terminal_failure(
            engine,
            M1AuthenticatedS1T128PrefillExecutionStageV1::EnginePreflight,
            M1AuthenticatedS1T128PrefillExecutionErrorV1::EngineFaulted,
            Status::NoQueue,
            (prepublication, cache, successor),
        ));
    }

    if deadline_expired(Boundary::BeforeQueueCreate, queue_wait_timeout).is_none() {
        engine.quarantine_m1_queue_rearm_failure();
        return Err(terminal_failure(
            engine,
            M1AuthenticatedS1T128PrefillExecutionStageV1::QueueCreate,
            M1AuthenticatedS1T128PrefillExecutionErrorV1::DeadlineExpired,
            Status::NoQueue,
            (prepublication, cache, successor),
        ));
    }

    let queue = match M1AuthenticatedPhysicalQueueSessionV1::create(
        diagnostic_ring_bytes,
        prepublication,
    ) {
        Ok(queue) => queue,
        Err(failure) => {
            let queue_status = match &failure {
                crate::M1AuthenticatedPhysicalQueueCreateFailureV1::Rejected { .. } => {
                    Status::NoQueue
                }
                crate::M1AuthenticatedPhysicalQueueCreateFailureV1::Terminal(_) => {
                    Status::Quarantined
                }
            };
            let failure = failure.quarantine_engine(&mut engine);
            return Err(terminal_failure(
                engine,
                M1AuthenticatedS1T128PrefillExecutionStageV1::QueueCreate,
                M1AuthenticatedS1T128PrefillExecutionErrorV1::LowerRejected,
                queue_status,
                (failure, cache, successor),
            ));
        }
    };
    if deadline_expired(Boundary::BeforeQueueSubmit, queue_wait_timeout).is_none() {
        let closure = queue.close_unpublished();
        let queue_status = physical_closure_status(&closure);
        return Err(terminal_failure(
            engine,
            M1AuthenticatedS1T128PrefillExecutionStageV1::QueueSubmit,
            M1AuthenticatedS1T128PrefillExecutionErrorV1::DeadlineExpired,
            queue_status,
            (closure, cache, successor),
        ));
    }
    let published = match queue.submit() {
        Ok(published) => published,
        Err(failure) => {
            let closure = failure.close_without_authority(&mut engine);
            let queue_status = physical_closure_status(&closure);
            return Err(terminal_failure(
                engine,
                M1AuthenticatedS1T128PrefillExecutionStageV1::QueueSubmit,
                M1AuthenticatedS1T128PrefillExecutionErrorV1::LowerRejected,
                queue_status,
                (closure, cache, successor),
            ));
        }
    };
    if deadline_expired(Boundary::AfterQueueSubmit, queue_wait_timeout).is_none() {
        let closure = published.close_in_flight();
        let queue_status = physical_closure_status(&closure);
        return Err(terminal_failure(
            engine,
            M1AuthenticatedS1T128PrefillExecutionStageV1::QueueSubmit,
            M1AuthenticatedS1T128PrefillExecutionErrorV1::DeadlineExpired,
            queue_status,
            (closure, cache, successor),
        ));
    }
    let Some(wait_timeout) = deadline_expired(Boundary::BeforeCompletionWait, queue_wait_timeout)
    else {
        let closure = published.close_in_flight();
        let queue_status = physical_closure_status(&closure);
        return Err(terminal_failure(
            engine,
            M1AuthenticatedS1T128PrefillExecutionStageV1::QueueWait,
            M1AuthenticatedS1T128PrefillExecutionErrorV1::DeadlineExpired,
            queue_status,
            (closure, cache, successor),
        ));
    };
    let completed = match published.wait_for(wait_timeout.milliseconds()) {
        Ok(completed) => completed,
        Err(failure) => {
            let failure = (*failure).quarantine_engine(&mut engine);
            return Err(terminal_failure(
                engine,
                M1AuthenticatedS1T128PrefillExecutionStageV1::QueueWait,
                M1AuthenticatedS1T128PrefillExecutionErrorV1::LowerRejected,
                Status::Quarantined,
                (failure, cache, successor),
            ));
        }
    };
    if deadline_expired(Boundary::AfterCompletionWait, queue_wait_timeout).is_none() {
        let closure = completed.close_completed();
        let queue_status = physical_closure_status(&closure);
        return Err(terminal_failure(
            engine,
            M1AuthenticatedS1T128PrefillExecutionStageV1::QueueWait,
            M1AuthenticatedS1T128PrefillExecutionErrorV1::DeadlineExpired,
            queue_status,
            (closure, cache, successor),
        ));
    }
    let recycled = match completed.recycle() {
        Ok(recycled) => recycled,
        Err(failure) => {
            let failure = (*failure).quarantine_engine(&mut engine);
            return Err(terminal_failure(
                engine,
                M1AuthenticatedS1T128PrefillExecutionStageV1::QueueRecycle,
                M1AuthenticatedS1T128PrefillExecutionErrorV1::LowerRejected,
                Status::Quarantined,
                (failure, cache, successor),
            ));
        }
    };
    if deadline_expired(Boundary::BeforeReadback, queue_wait_timeout).is_none() {
        let closure = recycled.close_recycled();
        let queue_status = physical_closure_status(&closure);
        return Err(terminal_failure(
            engine,
            M1AuthenticatedS1T128PrefillExecutionStageV1::CompletionObservation,
            M1AuthenticatedS1T128PrefillExecutionErrorV1::DeadlineExpired,
            queue_status,
            (closure, cache, successor),
        ));
    }
    let observed = match recycled.observe_completion() {
        Ok(observed) => observed,
        Err(failure) => match failure.retry() {
            Ok(observed) => observed,
            Err(failure) => {
                let teardown = (*failure).destroy_queue_and_retain_evidence(&mut engine);
                let queue_status = teardown_result_status(&teardown);
                return Err(terminal_failure(
                    engine,
                    M1AuthenticatedS1T128PrefillExecutionStageV1::CompletionObservation,
                    M1AuthenticatedS1T128PrefillExecutionErrorV1::LowerRejected,
                    queue_status,
                    (teardown, cache, successor),
                ));
            }
        },
    };
    if deadline_expired(Boundary::AfterReadback, queue_wait_timeout).is_none()
        || deadline_expired(Boundary::BeforeReadback, queue_wait_timeout).is_none()
    {
        let closure = close_prefill_observed(&mut engine, observed);
        let queue_status = physical_closure_status(&closure);
        return Err(terminal_failure(
            engine,
            M1AuthenticatedS1T128PrefillExecutionStageV1::DirectChoiceObservation,
            M1AuthenticatedS1T128PrefillExecutionErrorV1::DeadlineExpired,
            queue_status,
            (closure, cache, successor),
        ));
    }
    let direct = match observed.observe_direct_diagnostic_choices() {
        Ok(direct) => direct,
        Err(failure) => {
            let teardown = (*failure).destroy_queue_and_retain_evidence(&mut engine);
            let queue_status = teardown_result_status(&teardown);
            return Err(terminal_failure(
                engine,
                M1AuthenticatedS1T128PrefillExecutionStageV1::DirectChoiceObservation,
                M1AuthenticatedS1T128PrefillExecutionErrorV1::LowerRejected,
                queue_status,
                (teardown, cache, successor),
            ));
        }
    };
    if deadline_expired(Boundary::AfterReadback, queue_wait_timeout).is_none()
        || deadline_expired(Boundary::BeforeReadback, queue_wait_timeout).is_none()
    {
        let teardown = close_prefill_direct_observed(&mut engine, direct);
        let queue_status = teardown_result_status(&teardown);
        return Err(terminal_failure(
            engine,
            M1AuthenticatedS1T128PrefillExecutionStageV1::DirectCompletionCheck,
            M1AuthenticatedS1T128PrefillExecutionErrorV1::DeadlineExpired,
            queue_status,
            (teardown, cache, successor),
        ));
    }
    let direct = match direct.check_completion() {
        Ok(direct) => direct,
        Err(failure) => {
            let teardown = (*failure).destroy_queue_and_retain_evidence(&mut engine);
            let queue_status = teardown_result_status(&teardown);
            return Err(terminal_failure(
                engine,
                M1AuthenticatedS1T128PrefillExecutionStageV1::DirectCompletionCheck,
                M1AuthenticatedS1T128PrefillExecutionErrorV1::LowerRejected,
                queue_status,
                (teardown, cache, successor),
            ));
        }
    };
    if deadline_expired(Boundary::AfterReadback, queue_wait_timeout).is_none() {
        let teardown = direct.destroy_queue_and_retain_evidence(&mut engine);
        let queue_status = teardown_result_status(&teardown);
        return Err(terminal_failure(
            engine,
            M1AuthenticatedS1T128PrefillExecutionStageV1::DirectCompletionCheck,
            M1AuthenticatedS1T128PrefillExecutionErrorV1::DeadlineExpired,
            queue_status,
            (teardown, cache, successor),
        ));
    }
    let first_token = match require_one_authenticated_prefill_choice(direct.choices()) {
        Ok(first_token) => first_token,
        Err(error) => {
            let teardown = direct.destroy_queue_and_retain_evidence(&mut engine);
            let queue_status = teardown_result_status(&teardown);
            return Err(terminal_failure(
                engine,
                M1AuthenticatedS1T128PrefillExecutionStageV1::DirectChoiceCardinality,
                error,
                queue_status,
                (teardown, cache, successor),
            ));
        }
    };

    let mut members = Vec::new();
    if members.try_reserve_exact(1).is_err() {
        let teardown = direct.destroy_queue_and_retain_evidence(&mut engine);
        let queue_status = teardown_result_status(&teardown);
        return Err(terminal_failure(
            engine,
            M1AuthenticatedS1T128PrefillExecutionStageV1::PhysicalCompletion,
            M1AuthenticatedS1T128PrefillExecutionErrorV1::HostAllocation,
            queue_status,
            (teardown, cache, successor),
        ));
    }
    members.push(M1DeviceKvCompletionMemberV1::continuing(cache));
    let (readback, direct_choices) = direct.into_parts();
    if deadline_expired(Boundary::BeforeSettlement, queue_wait_timeout).is_none() {
        let closure = close_prefill_readback(&mut engine, readback);
        let queue_status = physical_closure_status(&closure);
        return Err(terminal_failure(
            engine,
            M1AuthenticatedS1T128PrefillExecutionStageV1::PhysicalCompletion,
            M1AuthenticatedS1T128PrefillExecutionErrorV1::DeadlineExpired,
            queue_status,
            (closure, direct_choices, first_token, successor, members),
        ));
    }
    let physical = complete_m1_authenticated_physical_step_v1(
        &mut engine,
        readback,
        M1DeviceKvCompletionRosterV1::new(members),
    );
    let physical = match physical {
        M1AuthenticatedCompletedStepOutcomeV1::Completed(physical) => physical,
        M1AuthenticatedCompletedStepOutcomeV1::Rejected(rejected) => {
            let closure =
                crate::m1_completed_step::close_m1_authenticated_completed_step_outcome_v1(
                    &mut engine,
                    M1AuthenticatedCompletedStepOutcomeV1::Rejected(rejected),
                );
            return Err(terminal_failure(
                engine,
                M1AuthenticatedS1T128PrefillExecutionStageV1::PhysicalCompletion,
                M1AuthenticatedS1T128PrefillExecutionErrorV1::LowerRejected,
                completed_step_closure_status(&closure),
                (closure, direct_choices, first_token, successor),
            ));
        }
        M1AuthenticatedCompletedStepOutcomeV1::Poisoned(poisoned) => {
            let closure =
                crate::m1_completed_step::close_m1_authenticated_completed_step_outcome_v1(
                    &mut engine,
                    M1AuthenticatedCompletedStepOutcomeV1::Poisoned(poisoned),
                );
            return Err(terminal_failure(
                engine,
                M1AuthenticatedS1T128PrefillExecutionStageV1::PhysicalCompletion,
                M1AuthenticatedS1T128PrefillExecutionErrorV1::PhysicalCompletionPoisoned,
                completed_step_closure_status(&closure),
                (closure, direct_choices, first_token, successor),
            ));
        }
    };
    if deadline_expired(Boundary::AfterSettlement, queue_wait_timeout).is_none()
        || deadline_expired(Boundary::BeforeSettlement, queue_wait_timeout).is_none()
    {
        let teardown = physical.destroy_queue_and_retain_completion(&mut engine);
        let queue_status = teardown_result_status(&teardown);
        return Err(terminal_failure(
            engine,
            M1AuthenticatedS1T128PrefillExecutionStageV1::PrefillPageRelease,
            M1AuthenticatedS1T128PrefillExecutionErrorV1::DeadlineExpired,
            queue_status,
            (teardown, direct_choices, first_token, successor),
        ));
    }
    let released = match release_m1_authenticated_completed_step_kv_pages_v1(physical) {
        Ok(released) => released,
        Err(failure) => {
            let (release_error, physical) = (*failure).into_parts();
            let teardown = physical.destroy_queue_and_retain_completion(&mut engine);
            let queue_status = teardown_result_status(&teardown);
            return Err(terminal_failure(
                engine,
                M1AuthenticatedS1T128PrefillExecutionStageV1::PrefillPageRelease,
                M1AuthenticatedS1T128PrefillExecutionErrorV1::LowerRejected,
                queue_status,
                (
                    release_error,
                    teardown,
                    direct_choices,
                    first_token,
                    successor,
                ),
            ));
        }
    };
    if deadline_expired(Boundary::AfterSettlement, queue_wait_timeout).is_none() {
        let teardown = released.destroy_queue_and_retain_step(&mut engine);
        let queue_status = teardown_result_status(&teardown);
        return Err(terminal_failure(
            engine,
            M1AuthenticatedS1T128PrefillExecutionStageV1::PrefillPageRelease,
            M1AuthenticatedS1T128PrefillExecutionErrorV1::DeadlineExpired,
            queue_status,
            (teardown, direct_choices, first_token, successor),
        ));
    }

    Ok(M1AuthenticatedS1T128PrefillExecutionSuccessV1 {
        engine,
        released,
        first_token,
        direct_choices,
        successor,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        require_one_authenticated_prefill_choice, M1AuthenticatedS1T128PrefillExecutionErrorV1,
    };
    use crate::M1ObservedDirectDiagnosticChoicesV1;

    fn choices(values: &[u32]) -> M1ObservedDirectDiagnosticChoicesV1 {
        M1ObservedDirectDiagnosticChoicesV1::for_serving_history_test(
            values.to_vec().into_boxed_slice(),
        )
    }

    #[test]
    fn exact_authenticated_direct_choice_becomes_first_token() {
        assert_eq!(
            require_one_authenticated_prefill_choice(&choices(&[17])),
            Ok(17)
        );
    }

    #[test]
    fn hostile_direct_choice_cardinalities_fail_closed() {
        for values in [&[][..], &[3, 5][..]] {
            assert_eq!(
                require_one_authenticated_prefill_choice(&choices(values)),
                Err(
                    M1AuthenticatedS1T128PrefillExecutionErrorV1::DirectChoiceCardinality {
                        expected: 1,
                        actual: values.len(),
                    }
                )
            );
        }
    }

    #[test]
    fn production_source_pins_post_join_completion_and_release_order() {
        let source = include_str!("authenticated_prefill_executor.rs");
        let production = source
            .split("pub(crate) fn execute_m1_authenticated_s1_t128_paired_prefill_with_deadline_v1")
            .nth(1)
            .and_then(|tail| tail.split("#[cfg(test)]").next())
            .expect("production executor source is present");
        let ordered = [
            "direct.check_completion()",
            "require_one_authenticated_prefill_choice(direct.choices())",
            "complete_m1_authenticated_physical_step_v1(",
            "release_m1_authenticated_completed_step_kv_pages_v1(physical)",
            "Ok(M1AuthenticatedS1T128PrefillExecutionSuccessV1",
        ];
        let mut prior = 0;
        for needle in ordered {
            let position = production
                .find(needle)
                .expect("ordered transition is present");
            assert!(position >= prior, "transition order drifted at {needle}");
            prior = position;
        }
    }

    #[test]
    fn hostile_first_prefill_deadline_cannot_cross_any_effect_boundary() {
        let source = include_str!("authenticated_prefill_executor.rs");
        let production = source
            .split("pub(crate) fn execute_m1_authenticated_s1_t128_paired_prefill_with_deadline_v1")
            .nth(1)
            .and_then(|tail| tail.split("#[cfg(test)]").next())
            .expect("deadline-bound prefill executor source is present");
        let ordered = [
            "Boundary::BeforeQueueSubmit",
            "queue.submit()",
            "Boundary::AfterQueueSubmit",
            "Boundary::BeforeCompletionWait",
            "published.wait_for(",
            "Boundary::AfterCompletionWait",
            "Boundary::BeforeReadback",
            "recycled.observe_completion()",
            "Boundary::AfterReadback",
            "Boundary::BeforeReadback",
            "observed.observe_direct_diagnostic_choices()",
            "Boundary::AfterReadback",
            "Boundary::BeforeReadback",
            "direct.check_completion()",
            "Boundary::AfterReadback",
            "Boundary::BeforeSettlement",
            "complete_m1_authenticated_physical_step_v1(",
            "Boundary::AfterSettlement",
            "Boundary::BeforeSettlement",
            "release_m1_authenticated_completed_step_kv_pages_v1(physical)",
            "Boundary::AfterSettlement",
        ];
        let mut tail = production;
        for needle in ordered {
            let position = tail
                .find(needle)
                .unwrap_or_else(|| panic!("missing hostile prefill deadline guard {needle}"));
            tail = &tail[position + needle.len()..];
        }
        assert!(production.contains("let Some(wait_timeout)"));
        assert!(production.contains("published.wait_for(wait_timeout.milliseconds())"));
    }
}
