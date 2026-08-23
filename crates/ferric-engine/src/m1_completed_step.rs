//! Transactional completion fan-out for one physical M1 queue generation.
//!
//! The completed readback owns the only exact completion capability. This
//! module first checks the complete scheduler-ordered cache and reservation
//! roster without mutation, then threads that capability through every device
//! KV transition and finally into `Engine::complete_exact` exactly once.

use core::fmt;

use ferric_spec::completion::CompletionEpoch;
use ferric_spec::scheduling::RequestState;
use ferric_spec::{M1QualificationContextStepKind, Qwen3ModelRole, RequestId};

use crate::device_cache::{
    DeviceKvStepCompletionOutcome, InertInitializedDeviceKvStepWrite,
    PoisonedDeviceKvStepCompletion,
};
use crate::{
    ActiveDeviceKvCache, CheckedCompletionSemantics, DeviceKvCacheError,
    DeviceKvCancellationOutcome, Engine, EngineError, ExactCompletion, M1CheckedCompletionOutputV1,
    M1FullStepKvReservationCustodyV1, M1PhysicalCompletedReadbackV1, M1PhysicalFixedBatchShapeV1,
    M1PhysicalReadbackQueueSessionV1, PendingDeviceKvStepWrite, SettledQuiescentDeviceKvCache,
};

/// Requested post-completion lifecycle for one exact scheduler member.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1DeviceKvCompletionDispositionV1 {
    /// Settle the completed write and keep the request active.
    Continue,
    /// Make the completed write unreachable and enter terminal quiescence.
    Retire,
}

/// Move-only device-KV custody for one scheduler member.
///
/// ```compile_fail
/// use ferric_engine::M1DeviceKvCompletionMemberV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1DeviceKvCompletionMemberV1>();
/// ```
#[must_use = "device-KV member custody must enter completion or remain retained"]
#[derive(Debug, PartialEq, Eq)]
pub struct M1DeviceKvCompletionMemberV1 {
    cache: ActiveDeviceKvCache,
    disposition: M1DeviceKvCompletionDispositionV1,
}

impl M1DeviceKvCompletionMemberV1 {
    /// Keeps one exact request active after completion.
    pub const fn continuing(cache: ActiveDeviceKvCache) -> Self {
        Self {
            cache,
            disposition: M1DeviceKvCompletionDispositionV1::Continue,
        }
    }

    /// Retires one exact request after its in-flight generation completes.
    pub const fn retiring(cache: ActiveDeviceKvCache) -> Self {
        Self {
            cache,
            disposition: M1DeviceKvCompletionDispositionV1::Retire,
        }
    }

    /// Exact request generation retained by the cache.
    #[must_use]
    pub fn request(&self) -> RequestId {
        self.cache.projection().request
    }

    /// Requested post-completion disposition.
    #[must_use]
    pub const fn disposition(&self) -> M1DeviceKvCompletionDispositionV1 {
        self.disposition
    }
}

/// Scheduler-ordered, move-only device-KV completion roster.
///
/// ```compile_fail
/// use ferric_engine::M1DeviceKvCompletionRosterV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1DeviceKvCompletionRosterV1>();
/// ```
#[must_use = "the exact cache roster must enter completion or remain retained"]
#[derive(Debug, PartialEq, Eq)]
pub struct M1DeviceKvCompletionRosterV1 {
    members: Vec<M1DeviceKvCompletionMemberV1>,
}

impl M1DeviceKvCompletionRosterV1 {
    /// Retains callers' exact scheduler order for transactional validation.
    pub fn new(members: Vec<M1DeviceKvCompletionMemberV1>) -> Self {
        Self { members }
    }

    /// Number of retained scheduler members.
    #[must_use]
    pub const fn member_count(&self) -> usize {
        self.members.len()
    }

    /// Borrows the exact retained order.
    pub fn members(&self) -> &[M1DeviceKvCompletionMemberV1] {
        &self.members
    }
}

/// Stable preflight or terminal composition diagnostic.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1CompletedStepErrorV1 {
    HostAllocation,
    Epoch,
    Shape,
    MemberCount {
        expected: usize,
        actual: usize,
    },
    RequestOrder {
        lane: usize,
    },
    Selection {
        lane: usize,
    },
    Reservation {
        lane: usize,
    },
    CompletionSemantics {
        lane: usize,
    },
    Cache {
        lane: usize,
        source: DeviceKvCacheError,
    },
    Engine(EngineError),
    Internal {
        lane: usize,
    },
}

impl fmt::Display for M1CompletedStepErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "M1 completed-step fan-out failed: {self:?}")
    }
}

impl std::error::Error for M1CompletedStepErrorV1 {}

/// Pure rejection retaining the exact physical readback and cache roster.
#[must_use = "rejected completion inputs remain recoverable"]
#[derive(Debug)]
pub struct M1CompletedStepRejectionV1 {
    error: M1CompletedStepErrorV1,
    readback: M1PhysicalCompletedReadbackV1,
    roster: M1DeviceKvCompletionRosterV1,
}

impl M1CompletedStepRejectionV1 {
    #[must_use]
    pub const fn error(&self) -> M1CompletedStepErrorV1 {
        self.error
    }

    #[must_use = "the exact rejected owners remain linear"]
    pub fn into_parts(
        self,
    ) -> (
        M1CompletedStepErrorV1,
        M1PhysicalCompletedReadbackV1,
        M1DeviceKvCompletionRosterV1,
    ) {
        (self.error, self.readback, self.roster)
    }
}

/// Device-KV custody after one member completed successfully.
#[must_use = "completed member custody must remain retained"]
#[derive(Debug, PartialEq, Eq)]
pub enum M1CompletedDeviceKvMemberV1 {
    Active(ActiveDeviceKvCache),
    Quiescent(SettledQuiescentDeviceKvCache),
}

impl M1CompletedDeviceKvMemberV1 {
    #[must_use]
    pub fn request(&self) -> RequestId {
        match self {
            Self::Active(cache) => cache.projection().request,
            Self::Quiescent(cache) => cache.projection().request,
        }
    }
}

/// Successful physical and Engine completion of one exact queue generation.
#[must_use = "post-readback queue and completed KV custody must remain retained"]
#[derive(Debug)]
pub struct M1CompletedStepSuccessV1 {
    queue: M1PhysicalReadbackQueueSessionV1,
    checked: M1CheckedCompletionOutputV1,
    members: Vec<M1CompletedDeviceKvMemberV1>,
    logical_accepted_counts: Box<[u32]>,
    externally_published_counts: Box<[u32]>,
    completed_members: usize,
}

impl M1CompletedStepSuccessV1 {
    pub const fn queue(&self) -> &M1PhysicalReadbackQueueSessionV1 {
        &self.queue
    }

    pub const fn checked(&self) -> &M1CheckedCompletionOutputV1 {
        &self.checked
    }

    pub fn members(&self) -> &[M1CompletedDeviceKvMemberV1] {
        &self.members
    }

    #[must_use]
    pub fn logical_accepted_counts(&self) -> &[u32] {
        &self.logical_accepted_counts
    }

    /// Per-member tokens made externally visible by this completion.
    #[must_use]
    pub fn externally_published_counts(&self) -> &[u32] {
        &self.externally_published_counts
    }

    #[must_use]
    pub const fn completed_members(&self) -> usize {
        self.completed_members
    }

    #[must_use = "all successful custody remains linear"]
    #[allow(clippy::type_complexity)]
    pub fn into_parts(
        self,
    ) -> (
        M1PhysicalReadbackQueueSessionV1,
        M1CheckedCompletionOutputV1,
        Vec<M1CompletedDeviceKvMemberV1>,
        Box<[u32]>,
        Box<[u32]>,
    ) {
        (
            self.queue,
            self.checked,
            self.members,
            self.logical_accepted_counts,
            self.externally_published_counts,
        )
    }

    #[allow(clippy::type_complexity)]
    pub(crate) fn into_release_parts(
        self,
    ) -> (
        M1PhysicalReadbackQueueSessionV1,
        M1CheckedCompletionOutputV1,
        Vec<M1CompletedDeviceKvMemberV1>,
        Box<[u32]>,
        Box<[u32]>,
        usize,
    ) {
        (
            self.queue,
            self.checked,
            self.members,
            self.logical_accepted_counts,
            self.externally_published_counts,
            self.completed_members,
        )
    }
}

#[derive(Debug)]
enum MemberReservationsV1 {
    TargetOnly {
        target: PendingDeviceKvStepWrite,
    },
    Paired {
        draft: PendingDeviceKvStepWrite,
        target: PendingDeviceKvStepWrite,
    },
    Speculative {
        draft: PendingDeviceKvStepWrite,
        target: PendingDeviceKvStepWrite,
    },
}

#[derive(Clone, Copy, Debug)]
struct MemberArithmeticV1 {
    target_accepted: u32,
    draft_accepted: u32,
    logical_accepted: u32,
    externally_published: u32,
}

#[derive(Debug)]
struct M1CompletedStepPreflightV1 {
    arithmetic: Vec<MemberArithmeticV1>,
    logical_accepted: Vec<u32>,
    externally_published: Vec<u32>,
}

#[derive(Debug)]
struct BoundMemberWorkV1 {
    member: M1DeviceKvCompletionMemberV1,
    reservations: MemberReservationsV1,
    arithmetic: MemberArithmeticV1,
}

#[allow(dead_code)]
#[derive(Debug)]
enum PoisonedCurrentMemberV1 {
    Active(ActiveDeviceKvCache),
    Cancelled(crate::CancelledDeviceKvCache),
    DeviceTransition(PoisonedDeviceKvStepCompletion),
    Cancellation(crate::PoisonedDeviceKvCache),
}

#[allow(dead_code)]
#[derive(Debug)]
struct M1CompletedStepPoisonCustodyV1 {
    completion: Option<ExactCompletion>,
    completed: Vec<M1CompletedDeviceKvMemberV1>,
    logical_accepted_counts: Box<[u32]>,
    externally_published_counts: Box<[u32]>,
    current: Option<PoisonedCurrentMemberV1>,
    initialized: [Option<InertInitializedDeviceKvStepWrite>; 2],
    pending: [Option<PendingDeviceKvStepWrite>; 2],
    remaining: Vec<Option<BoundMemberWorkV1>>,
}

/// Terminal custody after a supposedly infallible post-preflight transition failed.
#[must_use = "poisoned completion custody requires process-level quarantine"]
#[derive(Debug)]
pub struct M1CompletedStepPoisonV1 {
    error: M1CompletedStepErrorV1,
    queue: M1PhysicalReadbackQueueSessionV1,
    checked: M1CheckedCompletionOutputV1,
    custody: M1CompletedStepPoisonCustodyV1,
}

impl M1CompletedStepPoisonV1 {
    #[must_use]
    pub const fn error(&self) -> M1CompletedStepErrorV1 {
        self.error
    }

    pub const fn queue(&self) -> &M1PhysicalReadbackQueueSessionV1 {
        &self.queue
    }

    pub const fn checked(&self) -> &M1CheckedCompletionOutputV1 {
        &self.checked
    }

    #[must_use]
    pub fn completed_member_count(&self) -> usize {
        self.custody.completed.len()
    }

    /// Exact logical accepted counts derived before the first mutation.
    #[must_use]
    pub fn logical_accepted_counts(&self) -> &[u32] {
        &self.custody.logical_accepted_counts
    }

    /// Exact externally published counts derived before the first mutation.
    #[must_use]
    pub fn externally_published_counts(&self) -> &[u32] {
        &self.custody.externally_published_counts
    }

    #[must_use]
    pub const fn retains_completion(&self) -> bool {
        self.custody.completion.is_some()
    }
}

/// Exhaustive completion result separating retryable preflight rejection from poison.
///
/// ```compile_fail
/// use ferric_engine::M1CompletedStepOutcomeV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1CompletedStepOutcomeV1>();
/// ```
#[must_use = "every outcome retains exact linear custody"]
#[derive(Debug)]
pub enum M1CompletedStepOutcomeV1 {
    Completed(M1CompletedStepSuccessV1),
    Rejected(M1CompletedStepRejectionV1),
    Poisoned(Box<M1CompletedStepPoisonV1>),
}

fn reject(
    error: M1CompletedStepErrorV1,
    readback: M1PhysicalCompletedReadbackV1,
    roster: M1DeviceKvCompletionRosterV1,
) -> M1CompletedStepOutcomeV1 {
    M1CompletedStepOutcomeV1::Rejected(M1CompletedStepRejectionV1 {
        error,
        readback,
        roster,
    })
}

fn expected_shape_matches(
    shape: M1PhysicalFixedBatchShapeV1,
    reservations: &M1FullStepKvReservationCustodyV1,
) -> bool {
    matches!(
        (shape, reservations),
        (
            M1PhysicalFixedBatchShapeV1::TargetOnly,
            M1FullStepKvReservationCustodyV1::TargetOnly { .. }
        ) | (
            M1PhysicalFixedBatchShapeV1::PairedPrefill,
            M1FullStepKvReservationCustodyV1::PairedPrefill { .. }
        ) | (
            M1PhysicalFixedBatchShapeV1::SpeculativeK4
                | M1PhysicalFixedBatchShapeV1::SpeculativeK8
                | M1PhysicalFixedBatchShapeV1::SpeculativeK16,
            M1FullStepKvReservationCustodyV1::SpeculativeRound { .. }
        )
    )
}

fn preflight_engine<const C: usize>(
    engine: &Engine<C>,
    completion: &ExactCompletion,
    roster: &M1DeviceKvCompletionRosterV1,
    emitted: &[u32],
) -> Result<(), M1CompletedStepErrorV1> {
    let member_count = engine.pending_batch_member_count();
    if member_count != roster.member_count() {
        return Err(M1CompletedStepErrorV1::MemberCount {
            expected: member_count,
            actual: roster.member_count(),
        });
    }
    if emitted.len() != member_count {
        return Err(M1CompletedStepErrorV1::MemberCount {
            expected: member_count,
            actual: emitted.len(),
        });
    }
    for (lane, member) in roster.members().iter().enumerate() {
        let request = member.request();
        if engine.pending_member(lane) != Some(request) {
            return Err(M1CompletedStepErrorV1::RequestOrder { lane });
        }
        let expected_state = match member.disposition() {
            M1DeviceKvCompletionDispositionV1::Continue => RequestState::InFlight,
            M1DeviceKvCompletionDispositionV1::Retire => RequestState::Retiring,
        };
        if engine.state(request) != Some(expected_state) {
            return Err(M1CompletedStepErrorV1::RequestOrder { lane });
        }
    }
    engine
        .preflight_complete_exact(completion, emitted)
        .map_err(M1CompletedStepErrorV1::Engine)
}

fn preflight_member(
    lane: usize,
    member: &M1DeviceKvCompletionMemberV1,
    reservations: [Option<&PendingDeviceKvStepWrite>; 2],
    checked: &M1CheckedCompletionOutputV1,
    completion: &ExactCompletion,
    arithmetic: MemberArithmeticV1,
) -> Result<(), M1CompletedStepErrorV1> {
    let record = &checked.records()[lane];
    let request = member.request();
    if record.record().request != request || record.record().epoch != checked.epoch() {
        return Err(M1CompletedStepErrorV1::RequestOrder { lane });
    }
    for pending in reservations.iter().flatten() {
        if pending.request() != request || pending.epoch() != checked.epoch() {
            return Err(M1CompletedStepErrorV1::Reservation { lane });
        }
        member
            .cache
            .preflight_step_completion(pending, completion)
            .map_err(|source| M1CompletedStepErrorV1::Cache { lane, source })?;
    }
    for pending in reservations.iter().flatten() {
        let accepted = if pending.selection().role == Qwen3ModelRole::Target8B {
            arithmetic.target_accepted
        } else {
            arithmetic.draft_accepted
        };
        member
            .cache
            .preflight_step_settlement(pending, accepted, checked.epoch())
            .map_err(|source| M1CompletedStepErrorV1::Cache { lane, source })?;
    }
    match member.disposition() {
        M1DeviceKvCompletionDispositionV1::Continue => {}
        M1DeviceKvCompletionDispositionV1::Retire => member
            .cache
            .preflight_retirement_after_step(request, checked.epoch())
            .map_err(|source| M1CompletedStepErrorV1::Cache { lane, source })?,
    }
    Ok(())
}

fn preflight_target_completion_semantics(
    lane: usize,
    pending: &PendingDeviceKvStepWrite,
    semantics: CheckedCompletionSemantics,
    disposition: M1DeviceKvCompletionDispositionV1,
) -> Result<(), M1CompletedStepErrorV1> {
    let matches = match semantics {
        CheckedCompletionSemantics::DirectFinalRow { .. } => {
            pending.qualification_context().is_none()
        }
        CheckedCompletionSemantics::QualificationPromptCommit { context, .. } => {
            context.step().kind == M1QualificationContextStepKind::TeacherForcedPromptContext
                && pending.qualification_context_exactly_matches(context)
                && disposition == M1DeviceKvCompletionDispositionV1::Continue
        }
        CheckedCompletionSemantics::QualificationFinalRow { context, .. } => {
            context.step().kind == M1QualificationContextStepKind::FinalObserved
                && pending.qualification_context_exactly_matches(context)
                && disposition == M1DeviceKvCompletionDispositionV1::Retire
        }
        CheckedCompletionSemantics::Speculative { .. } => false,
    };
    if !matches {
        return Err(M1CompletedStepErrorV1::CompletionSemantics { lane });
    }
    Ok(())
}

fn preflight_all<const C: usize>(
    engine: &Engine<C>,
    readback: &M1PhysicalCompletedReadbackV1,
    roster: &M1DeviceKvCompletionRosterV1,
) -> Result<M1CompletedStepPreflightV1, M1CompletedStepErrorV1> {
    let checked = readback.checked();
    let reservations = readback.kv_reservations();
    let member_count = checked.records().len();
    if checked.epoch() != readback.completion_epoch() {
        return Err(M1CompletedStepErrorV1::Epoch);
    }
    if !expected_shape_matches(readback.queue().shape(), reservations) {
        return Err(M1CompletedStepErrorV1::Shape);
    }
    if roster.member_count() != member_count {
        return Err(M1CompletedStepErrorV1::MemberCount {
            expected: member_count,
            actual: roster.member_count(),
        });
    }
    let reservation_count = match reservations {
        M1FullStepKvReservationCustodyV1::TargetOnly { target } => target.reservations().len(),
        M1FullStepKvReservationCustodyV1::PairedPrefill { draft, target } => {
            if draft.reservations().len() != target.reservations().len() {
                return Err(M1CompletedStepErrorV1::MemberCount {
                    expected: target.reservations().len(),
                    actual: draft.reservations().len(),
                });
            }
            target.reservations().len()
        }
        M1FullStepKvReservationCustodyV1::SpeculativeRound {
            draft_decode,
            target_speculative,
        } => {
            if draft_decode.reservations().len() != target_speculative.reservations().len() {
                return Err(M1CompletedStepErrorV1::MemberCount {
                    expected: target_speculative.reservations().len(),
                    actual: draft_decode.reservations().len(),
                });
            }
            target_speculative.reservations().len()
        }
    };
    if reservation_count != member_count {
        return Err(M1CompletedStepErrorV1::MemberCount {
            expected: member_count,
            actual: reservation_count,
        });
    }

    let mut arithmetic = Vec::new();
    arithmetic
        .try_reserve_exact(member_count)
        .map_err(|_| M1CompletedStepErrorV1::HostAllocation)?;
    let mut logical_accepted = Vec::new();
    logical_accepted
        .try_reserve_exact(member_count)
        .map_err(|_| M1CompletedStepErrorV1::HostAllocation)?;
    let mut externally_published = Vec::new();
    externally_published
        .try_reserve_exact(member_count)
        .map_err(|_| M1CompletedStepErrorV1::HostAllocation)?;
    for lane in 0..member_count {
        let record = &checked.records()[lane];
        if record.selection() != checked.selection() {
            return Err(M1CompletedStepErrorV1::Selection { lane });
        }
        let (rows, member_arithmetic) = match reservations {
            M1FullStepKvReservationCustodyV1::TargetOnly { target } => {
                let target = &target.reservations()[lane];
                let semantics = record.semantics();
                if target.selection() != checked.selection()
                    || !matches!(
                        semantics,
                        CheckedCompletionSemantics::DirectFinalRow { .. }
                            | CheckedCompletionSemantics::QualificationPromptCommit { .. }
                            | CheckedCompletionSemantics::QualificationFinalRow { .. }
                    )
                    || u32::from(record.record().emitted_token_count)
                        != semantics.raw_compact_count()
                    || (matches!(
                        semantics,
                        CheckedCompletionSemantics::QualificationPromptCommit { .. }
                            | CheckedCompletionSemantics::QualificationFinalRow { .. }
                    ) && target.active_tokens() != 1)
                {
                    return Err(M1CompletedStepErrorV1::CompletionSemantics { lane });
                }
                preflight_target_completion_semantics(
                    lane,
                    target,
                    semantics,
                    roster.members()[lane].disposition(),
                )?;
                (
                    [Some(target), None],
                    MemberArithmeticV1 {
                        target_accepted: target.active_tokens(),
                        draft_accepted: 0,
                        logical_accepted: semantics.logical_accepted_count(),
                        externally_published: semantics.externally_published_count(),
                    },
                )
            }
            M1FullStepKvReservationCustodyV1::PairedPrefill { draft, target } => {
                let draft = &draft.reservations()[lane];
                let target = &target.reservations()[lane];
                if target.selection() != checked.selection()
                    || draft.selection().role != Qwen3ModelRole::Draft06B
                    || draft.selection().mode != target.selection().mode
                    || draft.selection().bucket != target.selection().bucket
                    || !matches!(
                        record.semantics(),
                        CheckedCompletionSemantics::DirectFinalRow { .. }
                    )
                    || record.record().emitted_token_count != 1
                {
                    return Err(M1CompletedStepErrorV1::CompletionSemantics { lane });
                }
                (
                    [Some(target), Some(draft)],
                    MemberArithmeticV1 {
                        target_accepted: target.active_tokens(),
                        draft_accepted: draft.active_tokens(),
                        logical_accepted: 1,
                        externally_published: 1,
                    },
                )
            }
            M1FullStepKvReservationCustodyV1::SpeculativeRound {
                draft_decode,
                target_speculative,
            } => {
                let draft = &draft_decode.reservations()[lane];
                let target = &target_speculative.reservations()[lane];
                let accepted = match record.semantics() {
                    CheckedCompletionSemantics::Speculative {
                        accepted_draft_tokens,
                        ..
                    } => u32::from(accepted_draft_tokens),
                    CheckedCompletionSemantics::DirectFinalRow { .. }
                    | CheckedCompletionSemantics::QualificationPromptCommit { .. }
                    | CheckedCompletionSemantics::QualificationFinalRow { .. } => {
                        return Err(M1CompletedStepErrorV1::CompletionSemantics { lane });
                    }
                };
                let emitted = accepted
                    .checked_add(1)
                    .ok_or(M1CompletedStepErrorV1::CompletionSemantics { lane })?;
                let draft_accepted = emitted.min(draft.draft_tokens());
                if target.selection() != checked.selection()
                    || draft.target_speculative_selection() != checked.selection()
                    || target.active_tokens() < emitted
                    || draft.pending_step_write().active_tokens() < draft_accepted
                    || u32::from(record.record().emitted_token_count) != emitted
                {
                    return Err(M1CompletedStepErrorV1::CompletionSemantics { lane });
                }
                (
                    [Some(target), Some(draft.pending_step_write())],
                    MemberArithmeticV1 {
                        target_accepted: emitted,
                        draft_accepted,
                        logical_accepted: emitted,
                        externally_published: emitted,
                    },
                )
            }
        };
        preflight_member(
            lane,
            &roster.members()[lane],
            rows,
            checked,
            readback.completion_authority(),
            member_arithmetic,
        )?;
        arithmetic.push(member_arithmetic);
        logical_accepted.push(member_arithmetic.logical_accepted);
        externally_published.push(member_arithmetic.externally_published);
    }
    preflight_engine(
        engine,
        readback.completion_authority(),
        roster,
        &logical_accepted,
    )?;
    Ok(M1CompletedStepPreflightV1 {
        arithmetic,
        logical_accepted,
        externally_published,
    })
}

fn bind_work(
    roster: M1DeviceKvCompletionRosterV1,
    reservations: M1FullStepKvReservationCustodyV1,
    arithmetic: Vec<MemberArithmeticV1>,
    mut bound: Vec<Option<BoundMemberWorkV1>>,
) -> Vec<Option<BoundMemberWorkV1>> {
    match reservations {
        M1FullStepKvReservationCustodyV1::TargetOnly { target } => {
            for ((member, arithmetic), target) in roster
                .members
                .into_iter()
                .zip(arithmetic)
                .zip(target.into_reservations())
            {
                bound.push(Some(BoundMemberWorkV1 {
                    member,
                    reservations: MemberReservationsV1::TargetOnly { target },
                    arithmetic,
                }));
            }
        }
        M1FullStepKvReservationCustodyV1::PairedPrefill { draft, target } => {
            for (((member, arithmetic), target), draft) in roster
                .members
                .into_iter()
                .zip(arithmetic)
                .zip(target.into_reservations())
                .zip(draft.into_reservations())
            {
                bound.push(Some(BoundMemberWorkV1 {
                    member,
                    reservations: MemberReservationsV1::Paired { draft, target },
                    arithmetic,
                }));
            }
        }
        M1FullStepKvReservationCustodyV1::SpeculativeRound {
            draft_decode,
            target_speculative,
        } => {
            for (((member, arithmetic), target), draft) in roster
                .members
                .into_iter()
                .zip(arithmetic)
                .zip(target_speculative.into_reservations())
                .zip(draft_decode.into_reservations())
            {
                bound.push(Some(BoundMemberWorkV1 {
                    member,
                    reservations: MemberReservationsV1::Speculative {
                        draft: draft.into_pending_step_write(),
                        target,
                    },
                    arithmetic,
                }));
            }
        }
    }
    bound
}

#[derive(Debug)]
struct ApplyFailureV1 {
    error: DeviceKvCacheError,
    current: PoisonedCurrentMemberV1,
    completion: Option<ExactCompletion>,
    initialized: [Option<InertInitializedDeviceKvStepWrite>; 2],
    pending: [Option<PendingDeviceKvStepWrite>; 2],
}

fn initialize(
    cache: ActiveDeviceKvCache,
    pending: PendingDeviceKvStepWrite,
    completion: ExactCompletion,
) -> Result<
    (
        ActiveDeviceKvCache,
        InertInitializedDeviceKvStepWrite,
        ExactCompletion,
    ),
    Box<ApplyFailureV1>,
> {
    match cache.complete_step_write(pending, completion) {
        DeviceKvStepCompletionOutcome::Completed(completed) => Ok(completed.into_parts()),
        DeviceKvStepCompletionOutcome::Rejected(failure) => {
            let (error, cache, pending, completion) = failure.into_parts();
            Err(Box::new(ApplyFailureV1 {
                error,
                current: PoisonedCurrentMemberV1::Active(cache),
                completion: Some(completion),
                initialized: [None, None],
                pending: [Some(pending), None],
            }))
        }
        DeviceKvStepCompletionOutcome::Poisoned(poisoned) => Err(Box::new(ApplyFailureV1 {
            error: poisoned.error(),
            current: PoisonedCurrentMemberV1::DeviceTransition(poisoned),
            completion: None,
            initialized: [None, None],
            pending: [None, None],
        })),
    }
}

fn finish_active(
    mut cache: ActiveDeviceKvCache,
    initialized: &[Option<InertInitializedDeviceKvStepWrite>; 2],
    arithmetic: MemberArithmeticV1,
    epoch: CompletionEpoch,
    mut completion: ExactCompletion,
) -> Result<(M1CompletedDeviceKvMemberV1, ExactCompletion), Box<ApplyFailureV1>> {
    let mut retired = 0u32;
    for write in initialized.iter().flatten() {
        let accepted = if write.selection().role == Qwen3ModelRole::Target8B {
            arithmetic.target_accepted
        } else {
            arithmetic.draft_accepted
        };
        match cache.settle_completed_step(write, accepted, epoch) {
            Ok(count) => {
                retired = retired.saturating_add(count);
            }
            Err(error) => {
                return Err(Box::new(ApplyFailureV1 {
                    error,
                    current: PoisonedCurrentMemberV1::Active(cache),
                    completion: Some(completion),
                    initialized: [None, None],
                    pending: [None, None],
                }));
            }
        }
    }
    if retired != 0 {
        match cache.settle_retired_epoch(completion) {
            Ok((_count, returned)) => completion = returned,
            Err(failure) => {
                let completion = failure.into_completion();
                return Err(Box::new(ApplyFailureV1 {
                    error: DeviceKvCacheError::NoRetiredPageAtEpoch,
                    current: PoisonedCurrentMemberV1::Active(cache),
                    completion: Some(completion),
                    initialized: [None, None],
                    pending: [None, None],
                }));
            }
        }
    }
    Ok((M1CompletedDeviceKvMemberV1::Active(cache), completion))
}

fn finish_retiring(
    mut cache: ActiveDeviceKvCache,
    request: RequestId,
    epoch: CompletionEpoch,
    completion: ExactCompletion,
    initialized: &[Option<InertInitializedDeviceKvStepWrite>; 2],
    arithmetic: MemberArithmeticV1,
) -> Result<(M1CompletedDeviceKvMemberV1, ExactCompletion), Box<ApplyFailureV1>> {
    for write in initialized.iter().flatten() {
        let accepted = if write.selection().role == Qwen3ModelRole::Target8B {
            arithmetic.target_accepted
        } else {
            arithmetic.draft_accepted
        };
        if let Err(error) = cache.settle_completed_step(write, accepted, epoch) {
            return Err(Box::new(ApplyFailureV1 {
                error,
                current: PoisonedCurrentMemberV1::Active(cache),
                completion: Some(completion),
                initialized: [None, None],
                pending: [None, None],
            }));
        }
    }
    let mut cancelled = match cache.cancel(request, epoch) {
        DeviceKvCancellationOutcome::Cancelled(cancelled) => cancelled,
        DeviceKvCancellationOutcome::Rejected(failure) => {
            let (error, cache) = failure.into_parts();
            return Err(Box::new(ApplyFailureV1 {
                error,
                current: PoisonedCurrentMemberV1::Active(cache),
                completion: Some(completion),
                initialized: [None, None],
                pending: [None, None],
            }));
        }
        DeviceKvCancellationOutcome::Poisoned(poisoned) => {
            return Err(Box::new(ApplyFailureV1 {
                error: poisoned.error(),
                current: PoisonedCurrentMemberV1::Cancellation(poisoned),
                completion: Some(completion),
                initialized: [None, None],
                pending: [None, None],
            }));
        }
    };
    if let Err(error) = cancelled.retire_all(request) {
        return Err(Box::new(ApplyFailureV1 {
            error,
            current: PoisonedCurrentMemberV1::Cancelled(cancelled),
            completion: Some(completion),
            initialized: [None, None],
            pending: [None, None],
        }));
    }
    match cancelled.quiesce(completion) {
        Ok(quiescent) => {
            let (settled, completion) = quiescent.into_threaded_parts();
            Ok((M1CompletedDeviceKvMemberV1::Quiescent(settled), completion))
        }
        Err(failure) => {
            let error = failure.error();
            let (cancelled, completion) = failure.into_parts();
            Err(Box::new(ApplyFailureV1 {
                error,
                current: PoisonedCurrentMemberV1::Cancelled(cancelled),
                completion: Some(completion),
                initialized: [None, None],
                pending: [None, None],
            }))
        }
    }
}

fn apply_member(
    work: BoundMemberWorkV1,
    completion: ExactCompletion,
    epoch: CompletionEpoch,
) -> Result<(M1CompletedDeviceKvMemberV1, ExactCompletion), Box<ApplyFailureV1>> {
    let BoundMemberWorkV1 {
        member,
        reservations,
        arithmetic,
    } = work;
    let request = member.request();
    let disposition = member.disposition;
    let mut initialized = [None, None];
    let (cache, completion) = match reservations {
        MemberReservationsV1::TargetOnly { target } => {
            match initialize(member.cache, target, completion) {
                Ok((cache, target, completion)) => {
                    initialized[0] = Some(target);
                    (cache, completion)
                }
                Err(failure) => return Err(failure),
            }
        }
        MemberReservationsV1::Paired { draft, target }
        | MemberReservationsV1::Speculative { draft, target } => {
            let (cache, target, completion) = match initialize(member.cache, target, completion) {
                Ok(parts) => parts,
                Err(mut failure) => {
                    failure.pending[1] = Some(draft);
                    return Err(failure);
                }
            };
            initialized[0] = Some(target);
            let (cache, draft, completion) = match initialize(cache, draft, completion) {
                Ok(parts) => parts,
                Err(mut failure) => {
                    failure.initialized[0] = initialized[0].take();
                    return Err(failure);
                }
            };
            initialized[1] = Some(draft);
            (cache, completion)
        }
    };
    let result = match disposition {
        M1DeviceKvCompletionDispositionV1::Continue => {
            finish_active(cache, &initialized, arithmetic, epoch, completion)
        }
        M1DeviceKvCompletionDispositionV1::Retire => {
            finish_retiring(cache, request, epoch, completion, &initialized, arithmetic)
        }
    };
    match result {
        Ok(completed) => Ok(completed),
        Err(mut failure) => {
            failure.initialized = initialized;
            Err(failure)
        }
    }
}

fn poison(
    error: M1CompletedStepErrorV1,
    queue: M1PhysicalReadbackQueueSessionV1,
    checked: M1CheckedCompletionOutputV1,
    custody: M1CompletedStepPoisonCustodyV1,
) -> M1CompletedStepOutcomeV1 {
    M1CompletedStepOutcomeV1::Poisoned(Box::new(M1CompletedStepPoisonV1 {
        error,
        queue,
        checked,
        custody,
    }))
}

/// Completes one physical M1 readback across every scheduler member exactly once.
///
/// All output, reservation, cache-roster, arithmetic, and Engine checks finish
/// before the first device-cache mutation. A preflight rejection returns both
/// linear inputs unchanged. Any unexpected failure after mutation is terminal
/// and retains opaque queue, cache, reservation, and completion custody.
pub fn complete_m1_physical_step_v1<const C: usize>(
    engine: &mut Engine<C>,
    readback: M1PhysicalCompletedReadbackV1,
    roster: M1DeviceKvCompletionRosterV1,
) -> M1CompletedStepOutcomeV1 {
    let M1CompletedStepPreflightV1 {
        arithmetic,
        logical_accepted,
        externally_published,
    } = match preflight_all(engine, &readback, &roster) {
        Ok(preflight) => preflight,
        Err(error) => return reject(error, readback, roster),
    };
    let member_count = arithmetic.len();
    let mut completed = Vec::new();
    if completed.try_reserve_exact(member_count).is_err() {
        return reject(M1CompletedStepErrorV1::HostAllocation, readback, roster);
    }
    let mut remaining = Vec::new();
    if remaining.try_reserve_exact(member_count).is_err() {
        return reject(M1CompletedStepErrorV1::HostAllocation, readback, roster);
    }

    let (queue, checked, completion, reservations) = readback.into_parts();
    let epoch = checked.epoch();
    let mut remaining = bind_work(roster, reservations, arithmetic, remaining);
    let mut completion = Some(completion);
    for lane in 0..member_count {
        let Some(work) = remaining.get_mut(lane).and_then(Option::take) else {
            return poison(
                M1CompletedStepErrorV1::Internal { lane },
                queue,
                checked,
                M1CompletedStepPoisonCustodyV1 {
                    completion,
                    completed,
                    logical_accepted_counts: logical_accepted.into_boxed_slice(),
                    externally_published_counts: externally_published.into_boxed_slice(),
                    current: None,
                    initialized: [None, None],
                    pending: [None, None],
                    remaining,
                },
            );
        };
        let Some(authority) = completion.take() else {
            if let Some(slot) = remaining.get_mut(lane) {
                *slot = Some(work);
            }
            return poison(
                M1CompletedStepErrorV1::Internal { lane },
                queue,
                checked,
                M1CompletedStepPoisonCustodyV1 {
                    completion: None,
                    completed,
                    logical_accepted_counts: logical_accepted.into_boxed_slice(),
                    externally_published_counts: externally_published.into_boxed_slice(),
                    current: None,
                    initialized: [None, None],
                    pending: [None, None],
                    remaining,
                },
            );
        };
        match apply_member(work, authority, epoch) {
            Ok((member, returned)) => {
                completed.push(member);
                completion = Some(returned);
            }
            Err(failure) => {
                return poison(
                    M1CompletedStepErrorV1::Cache {
                        lane,
                        source: failure.error,
                    },
                    queue,
                    checked,
                    M1CompletedStepPoisonCustodyV1 {
                        completion: failure.completion,
                        completed,
                        logical_accepted_counts: logical_accepted.into_boxed_slice(),
                        externally_published_counts: externally_published.into_boxed_slice(),
                        current: Some(failure.current),
                        initialized: failure.initialized,
                        pending: failure.pending,
                        remaining,
                    },
                );
            }
        }
    }

    let Some(authority) = completion.take() else {
        return poison(
            M1CompletedStepErrorV1::Internal { lane: member_count },
            queue,
            checked,
            M1CompletedStepPoisonCustodyV1 {
                completion: None,
                completed,
                logical_accepted_counts: logical_accepted.into_boxed_slice(),
                externally_published_counts: externally_published.into_boxed_slice(),
                current: None,
                initialized: [None, None],
                pending: [None, None],
                remaining,
            },
        );
    };
    match engine.complete_exact(authority, &logical_accepted) {
        Ok(completed_members) => M1CompletedStepOutcomeV1::Completed(M1CompletedStepSuccessV1 {
            queue,
            checked,
            members: completed,
            logical_accepted_counts: logical_accepted.into_boxed_slice(),
            externally_published_counts: externally_published.into_boxed_slice(),
            completed_members,
        }),
        Err(failure) => {
            let error = failure.error();
            poison(
                M1CompletedStepErrorV1::Engine(error),
                queue,
                checked,
                M1CompletedStepPoisonCustodyV1 {
                    completion: failure.into_completion(),
                    completed,
                    logical_accepted_counts: logical_accepted.into_boxed_slice(),
                    externally_published_counts: externally_published.into_boxed_slice(),
                    current: None,
                    initialized: [None, None],
                    pending: [None, None],
                    remaining,
                },
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use ferric_spec::{
        m1_qualification_context_plan, Identity, M1QualificationExecutionBindingDeclaration,
        M1QualificationLaneExecutionBinding, M1QualificationLaneGrouping, PhysicalKvLifecycle,
        PhysicalPageId, Qwen3ExecutionMode, Qwen3PlanBucket, Qwen3PlanSelection,
    };

    use super::*;
    use crate::device_cache::test_support::bind_gfx942_device;
    use crate::{DeviceKvPageLease, Gfx942DeviceBinding, GFX942_PROCESSOR, GFX942_TARGET_FEATURES};

    const EPOCH: CompletionEpoch = CompletionEpoch::new(1);

    fn identity(tag: u8) -> Identity {
        Identity::new([tag; 32])
    }

    fn device() -> Gfx942DeviceBinding {
        bind_gfx942_device(identity(1), 7, GFX942_PROCESSOR, GFX942_TARGET_FEATURES).unwrap()
    }

    const fn selection(
        role: Qwen3ModelRole,
        mode: Qwen3ExecutionMode,
        bucket: Qwen3PlanBucket,
    ) -> Qwen3PlanSelection {
        Qwen3PlanSelection { role, mode, bucket }
    }

    fn cache(
        request: RequestId,
        mode: Qwen3ExecutionMode,
        bucket: Qwen3PlanBucket,
    ) -> ActiveDeviceKvCache {
        ActiveDeviceKvCache::new(
            device(),
            request,
            selection(Qwen3ModelRole::Target8B, mode, bucket),
            selection(Qwen3ModelRole::Draft06B, mode, bucket),
        )
        .unwrap()
    }

    fn lease(request: RequestId, role: Qwen3ModelRole, tag: u8) -> DeviceKvPageLease {
        DeviceKvPageLease::from_contracted_workspace_bridge_test_allocation(
            device(),
            identity(tag),
            request,
            PhysicalPageId::new(role, 0, 1),
        )
    }

    fn exact() -> ExactCompletion {
        ExactCompletion::from_contracted_hsa_quiescence(EPOCH)
    }

    fn with_large_stack(test: fn()) {
        std::thread::Builder::new()
            .stack_size(8 * 1024 * 1024)
            .spawn(test)
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn post_preflight_poison_custody_retains_both_semantic_count_vectors() {
        let custody = M1CompletedStepPoisonCustodyV1 {
            completion: None,
            completed: Vec::new(),
            logical_accepted_counts: vec![1].into_boxed_slice(),
            externally_published_counts: vec![0].into_boxed_slice(),
            current: None,
            initialized: [None, None],
            pending: [None, None],
            remaining: Vec::new(),
        };
        assert_eq!(&*custody.logical_accepted_counts, &[1]);
        assert_eq!(&*custody.externally_published_counts, &[0]);
    }

    fn paired_work(
        request: RequestId,
        disposition: M1DeviceKvCompletionDispositionV1,
    ) -> BoundMemberWorkV1 {
        let mut cache = cache(
            request,
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T128,
        );
        let target = cache
            .reserve_step_write(
                request,
                Qwen3ModelRole::Target8B,
                0,
                4,
                EPOCH,
                vec![lease(request, Qwen3ModelRole::Target8B, 10)],
            )
            .unwrap();
        let draft = cache
            .reserve_step_write(
                request,
                Qwen3ModelRole::Draft06B,
                0,
                4,
                EPOCH,
                vec![lease(request, Qwen3ModelRole::Draft06B, 11)],
            )
            .unwrap();
        BoundMemberWorkV1 {
            member: M1DeviceKvCompletionMemberV1 { cache, disposition },
            reservations: MemberReservationsV1::Paired { draft, target },
            arithmetic: MemberArithmeticV1 {
                target_accepted: 4,
                draft_accepted: 4,
                logical_accepted: 1,
                externally_published: 1,
            },
        }
    }

    fn speculative_work(
        request: RequestId,
        accepted_draft: u32,
        disposition: M1DeviceKvCompletionDispositionV1,
    ) -> BoundMemberWorkV1 {
        let bucket = Qwen3PlanBucket::SpeculativeS1K4C8192;
        let target_selection = selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Speculative,
            bucket,
        );
        let mut cache = cache(request, Qwen3ExecutionMode::Speculative, bucket);
        let target = cache
            .reserve_step_write(
                request,
                Qwen3ModelRole::Target8B,
                0,
                5,
                EPOCH,
                vec![lease(request, Qwen3ModelRole::Target8B, 20)],
            )
            .unwrap();
        let draft = cache
            .reserve_speculative_draft_round_write(
                request,
                target_selection,
                selection(
                    Qwen3ModelRole::Draft06B,
                    Qwen3ExecutionMode::Decode,
                    Qwen3PlanBucket::DecodeS1C8192,
                ),
                0,
                EPOCH,
                vec![lease(request, Qwen3ModelRole::Draft06B, 21)],
            )
            .unwrap()
            .into_pending_step_write();
        let emitted = accepted_draft + 1;
        BoundMemberWorkV1 {
            member: M1DeviceKvCompletionMemberV1 { cache, disposition },
            reservations: MemberReservationsV1::Speculative { draft, target },
            arithmetic: MemberArithmeticV1 {
                target_accepted: emitted,
                draft_accepted: emitted.min(4),
                logical_accepted: emitted,
                externally_published: emitted,
            },
        }
    }

    fn target_only_work(
        request: RequestId,
        disposition: M1DeviceKvCompletionDispositionV1,
    ) -> BoundMemberWorkV1 {
        let mut cache = cache(
            request,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        );
        let target = cache
            .reserve_step_write(
                request,
                Qwen3ModelRole::Target8B,
                0,
                1,
                EPOCH,
                vec![lease(request, Qwen3ModelRole::Target8B, 30)],
            )
            .unwrap();
        BoundMemberWorkV1 {
            member: M1DeviceKvCompletionMemberV1 { cache, disposition },
            reservations: MemberReservationsV1::TargetOnly { target },
            arithmetic: MemberArithmeticV1 {
                target_accepted: 1,
                draft_accepted: 0,
                logical_accepted: 1,
                externally_published: 1,
            },
        }
    }

    fn qualification_context(
        workload_tag: u8,
        lane_tag: u8,
        token_sequence_tag: u8,
        ordinal: u32,
    ) -> crate::M1ValidatedQualificationContextStepV1 {
        let grouping = M1QualificationLaneGrouping::S1;
        let expected = M1QualificationExecutionBindingDeclaration {
            declared_workload_digest: identity(workload_tag),
            ordered_lanes: vec![M1QualificationLaneExecutionBinding {
                lane_ordinal: 0,
                lane_identity: identity(lane_tag),
                token_sequence_identity: identity(token_sequence_tag),
            }],
        };
        let plan = m1_qualification_context_plan(grouping, expected.clone());
        crate::validate_m1_qualification_context_plan_v1(&plan, grouping, &expected)
            .unwrap()
            .step(ordinal, 0)
            .unwrap()
    }

    #[test]
    fn qualification_completion_requires_the_reservation_carried_exact_witness() {
        with_large_stack(
            qualification_completion_requires_the_reservation_carried_exact_witness_inner,
        );
    }

    fn qualification_completion_requires_the_reservation_carried_exact_witness_inner() {
        let context_a = qualification_context(81, 82, 83, 0);
        let context_b = qualification_context(91, 92, 93, 0);
        let later_context_a = qualification_context(81, 82, 83, 1);
        let mut work = target_only_work(
            RequestId::new(0, 21),
            M1DeviceKvCompletionDispositionV1::Continue,
        );
        let MemberReservationsV1::TargetOnly { target } = &mut work.reservations else {
            unreachable!();
        };
        target.bind_qualification_context_for_test(context_a);

        assert_eq!(
            preflight_target_completion_semantics(
                0,
                target,
                CheckedCompletionSemantics::QualificationPromptCommit {
                    choice: 7,
                    context: context_a,
                },
                M1DeviceKvCompletionDispositionV1::Continue,
            ),
            Ok(())
        );
        for substituted in [context_b, later_context_a] {
            assert_eq!(
                preflight_target_completion_semantics(
                    0,
                    target,
                    CheckedCompletionSemantics::QualificationPromptCommit {
                        choice: 7,
                        context: substituted,
                    },
                    M1DeviceKvCompletionDispositionV1::Continue,
                ),
                Err(M1CompletedStepErrorV1::CompletionSemantics { lane: 0 })
            );
        }
        assert_eq!(
            preflight_target_completion_semantics(
                0,
                target,
                CheckedCompletionSemantics::DirectFinalRow { token: 7 },
                M1DeviceKvCompletionDispositionV1::Continue,
            ),
            Err(M1CompletedStepErrorV1::CompletionSemantics { lane: 0 })
        );
    }

    #[test]
    fn qualification_disposition_follows_exact_step_after_future_reserve_is_empty() {
        with_large_stack(
            qualification_disposition_follows_exact_step_after_future_reserve_is_empty_inner,
        );
    }

    fn qualification_disposition_follows_exact_step_after_future_reserve_is_empty_inner() {
        for ordinal in [8_176, 8_190] {
            let context = qualification_context(81, 82, 83, ordinal);
            let mut work = target_only_work(
                RequestId::new(0, 30 + ordinal),
                M1DeviceKvCompletionDispositionV1::Retire,
            );
            let before = work.member.cache.projection();
            let MemberReservationsV1::TargetOnly { target } = &mut work.reservations else {
                unreachable!();
            };
            target.bind_qualification_context_for_test(context);
            assert_eq!(
                preflight_target_completion_semantics(
                    0,
                    target,
                    CheckedCompletionSemantics::QualificationPromptCommit { choice: 7, context },
                    M1DeviceKvCompletionDispositionV1::Retire,
                ),
                Err(M1CompletedStepErrorV1::CompletionSemantics { lane: 0 })
            );
            assert_eq!(target.qualification_context(), Some(context));
            assert_eq!(work.member.cache.projection(), before);
        }

        let context = qualification_context(81, 82, 83, 8_191);
        let mut work = target_only_work(
            RequestId::new(0, 8_221),
            M1DeviceKvCompletionDispositionV1::Retire,
        );
        let before = work.member.cache.projection();
        let MemberReservationsV1::TargetOnly { target } = &mut work.reservations else {
            unreachable!();
        };
        target.bind_qualification_context_for_test(context);
        let final_semantics =
            CheckedCompletionSemantics::QualificationFinalRow { token: 7, context };
        assert_eq!(
            preflight_target_completion_semantics(
                0,
                target,
                final_semantics,
                M1DeviceKvCompletionDispositionV1::Continue,
            ),
            Err(M1CompletedStepErrorV1::CompletionSemantics { lane: 0 })
        );
        assert_eq!(target.qualification_context(), Some(context));
        assert_eq!(work.member.cache.projection(), before);
        assert_eq!(
            preflight_target_completion_semantics(
                0,
                target,
                final_semantics,
                M1DeviceKvCompletionDispositionV1::Retire,
            ),
            Ok(())
        );
    }

    #[test]
    fn paired_direct_completion_publishes_both_roles() {
        with_large_stack(paired_direct_completion_publishes_both_roles_inner);
    }

    fn paired_direct_completion_publishes_both_roles_inner() {
        let request = RequestId::new(2, 3);
        let (completed, completion) = apply_member(
            paired_work(request, M1DeviceKvCompletionDispositionV1::Continue),
            exact(),
            EPOCH,
        )
        .unwrap();
        assert_eq!(completion.epoch(), EPOCH);
        let M1CompletedDeviceKvMemberV1::Active(cache) = completed else {
            panic!("continuing member must remain active");
        };
        let projection = cache.projection();
        assert_eq!(projection.target.committed_tokens, 4);
        assert_eq!(projection.draft.committed_tokens, 4);
        assert!(!projection.target_write_pending);
        assert!(!projection.draft_write_pending);
    }

    #[test]
    fn speculative_partial_acceptance_rolls_back_both_suffixes() {
        with_large_stack(speculative_partial_acceptance_rolls_back_both_suffixes_inner);
    }

    fn speculative_partial_acceptance_rolls_back_both_suffixes_inner() {
        let request = RequestId::new(3, 4);
        let (completed, completion) = apply_member(
            speculative_work(request, 2, M1DeviceKvCompletionDispositionV1::Continue),
            exact(),
            EPOCH,
        )
        .unwrap();
        assert_eq!(completion.epoch(), EPOCH);
        let M1CompletedDeviceKvMemberV1::Active(cache) = completed else {
            panic!("continuing member must remain active");
        };
        let projection = cache.projection();
        assert_eq!(projection.target.committed_tokens, 3);
        assert_eq!(projection.target.resident_tokens, 3);
        assert_eq!(projection.draft.committed_tokens, 3);
        assert_eq!(projection.draft.resident_tokens, 3);
        assert_eq!(projection.target_active_pages, 1);
        assert_eq!(projection.draft_active_pages, 1);
        assert_eq!(projection.target_quiescent_retired_pages, 0);
        assert_eq!(projection.draft_quiescent_retired_pages, 0);
    }

    #[test]
    fn retiring_completion_settles_then_enters_quiescent_custody() {
        with_large_stack(retiring_completion_settles_then_enters_quiescent_custody_inner);
    }

    fn retiring_completion_settles_then_enters_quiescent_custody_inner() {
        let request = RequestId::new(4, 5);
        let (completed, completion) = apply_member(
            target_only_work(request, M1DeviceKvCompletionDispositionV1::Retire),
            exact(),
            EPOCH,
        )
        .unwrap();
        assert_eq!(completion.epoch(), EPOCH);
        let M1CompletedDeviceKvMemberV1::Quiescent(cache) = completed else {
            panic!("retiring member must become quiescent");
        };
        let projection = cache.projection();
        assert_eq!(
            projection.target.lifecycle,
            PhysicalKvLifecycle::RetiredAwaitingQuiescence { after_epoch: EPOCH }
        );
        assert_eq!(projection.target_quiescent_retired_pages, 1);
        assert_eq!(cache.completion_epoch(), EPOCH);
    }

    #[test]
    fn paired_late_target_drift_proves_target_is_attempted_before_draft() {
        with_large_stack(paired_late_target_drift_proves_target_is_attempted_before_draft_inner);
    }

    fn paired_late_target_drift_proves_target_is_attempted_before_draft_inner() {
        let request = RequestId::new(5, 6);
        let mut work = paired_work(request, M1DeviceKvCompletionDispositionV1::Continue);
        let MemberReservationsV1::Paired { target, .. } = &mut work.reservations else {
            unreachable!();
        };
        target.corrupt_completion_bridge_request_for_test(RequestId::new(5, 7));
        let failure = apply_member(work, exact(), EPOCH).unwrap_err();
        assert!(failure.initialized.iter().all(Option::is_none));
        assert_eq!(
            failure.pending[0].as_ref().unwrap().selection().role,
            Qwen3ModelRole::Target8B
        );
        assert_eq!(
            failure.pending[1].as_ref().unwrap().selection().role,
            Qwen3ModelRole::Draft06B
        );
    }

    #[test]
    fn speculative_late_draft_drift_retains_initialized_target_first() {
        with_large_stack(speculative_late_draft_drift_retains_initialized_target_first_inner);
    }

    fn speculative_late_draft_drift_retains_initialized_target_first_inner() {
        let request = RequestId::new(6, 7);
        let mut work = speculative_work(request, 1, M1DeviceKvCompletionDispositionV1::Continue);
        let MemberReservationsV1::Speculative { draft, .. } = &mut work.reservations else {
            unreachable!();
        };
        draft.corrupt_completion_bridge_request_for_test(RequestId::new(6, 8));
        let failure = apply_member(work, exact(), EPOCH).unwrap_err();
        assert_eq!(
            failure.initialized[0].as_ref().unwrap().selection().role,
            Qwen3ModelRole::Target8B
        );
        assert!(failure.initialized[1].is_none());
        assert_eq!(
            failure.pending[0].as_ref().unwrap().selection().role,
            Qwen3ModelRole::Draft06B
        );
    }

    #[test]
    fn one_completion_threads_lane_major_then_completes_engine_once() {
        with_large_stack(one_completion_threads_lane_major_then_completes_engine_once_inner);
    }

    fn one_completion_threads_lane_major_then_completes_engine_once_inner() {
        let mut engine = Engine::<2>::new(16, 4, 32).unwrap();
        let first = engine.admit().unwrap();
        let second = engine.admit().unwrap();
        engine.append_tentative(first, 1).unwrap();
        engine.append_tentative(second, 1).unwrap();
        let mut scheduled = [RequestId::new(0, 0); 2];
        let batch = engine.dispatch_ready(&mut scheduled).unwrap().unwrap();
        assert_eq!(scheduled, [first, second]);
        assert_eq!(batch.epoch(), EPOCH);

        let mut completion = ExactCompletion::from_contracted_hsa_quiescence(batch.epoch());
        for request in scheduled {
            let (completed, returned) = apply_member(
                target_only_work(request, M1DeviceKvCompletionDispositionV1::Continue),
                completion,
                batch.epoch(),
            )
            .unwrap();
            assert_eq!(completed.request(), request);
            completion = returned;
        }
        assert_eq!(engine.complete_exact(completion, &[1, 1]).unwrap(), 2);
        assert_eq!(engine.completed_epoch(), EPOCH);
    }

    #[test]
    fn qualification_prompt_commit_advances_physical_and_engine_kv_without_publication() {
        with_large_stack(
            qualification_prompt_commit_advances_physical_and_engine_kv_without_publication_inner,
        );
    }

    fn qualification_prompt_commit_advances_physical_and_engine_kv_without_publication_inner() {
        let mut engine = Engine::<1>::new(16, 4, 32).unwrap();
        let request = engine.admit().unwrap();
        assert_eq!(engine.committed_tokens(request), Some(0));
        engine.append_tentative(request, 1).unwrap();
        let mut scheduled = [RequestId::new(0, 0); 1];
        let batch = engine.dispatch_ready(&mut scheduled).unwrap().unwrap();
        assert_eq!(scheduled, [request]);

        let mut work = target_only_work(request, M1DeviceKvCompletionDispositionV1::Continue);
        work.arithmetic = MemberArithmeticV1 {
            target_accepted: 1,
            draft_accepted: 0,
            logical_accepted: 1,
            externally_published: 0,
        };
        assert_eq!(work.arithmetic.target_accepted, 1);
        assert_eq!(work.arithmetic.logical_accepted, 1);
        assert_eq!(work.arithmetic.externally_published, 0);
        let (completed, completion) = apply_member(
            work,
            ExactCompletion::from_contracted_hsa_quiescence(batch.epoch()),
            batch.epoch(),
        )
        .unwrap();
        let M1CompletedDeviceKvMemberV1::Active(mut physical) = completed else {
            panic!("qualification priming member must continue");
        };
        assert_eq!(physical.projection().target.committed_tokens, 1);

        assert_eq!(engine.complete_exact(completion, &[1]).unwrap(), 1);
        assert_eq!(engine.committed_tokens(request), Some(1));
        engine.append_tentative(request, 1).unwrap();
        let next = engine.dispatch_ready(&mut scheduled).unwrap().unwrap();
        assert_eq!(scheduled, [request]);
        assert_eq!(next.epoch(), CompletionEpoch::new(2));
        let next_physical = physical
            .reserve_step_write(
                request,
                Qwen3ModelRole::Target8B,
                1,
                1,
                next.epoch(),
                Vec::new(),
            )
            .expect("the next same-page context reservation must succeed");
        assert_eq!(next_physical.committed_tokens(), 1);
        assert_eq!(next_physical.active_tokens(), 1);
    }

    #[test]
    fn scheduler_member_order_is_rejected_before_cache_mutation() {
        with_large_stack(scheduler_member_order_is_rejected_before_cache_mutation_inner);
    }

    fn scheduler_member_order_is_rejected_before_cache_mutation_inner() {
        let mut engine = Engine::<2>::new(16, 4, 32).unwrap();
        let first = engine.admit().unwrap();
        let second = engine.admit().unwrap();
        engine.append_tentative(first, 1).unwrap();
        engine.append_tentative(second, 1).unwrap();
        let mut scheduled = [RequestId::new(0, 0); 2];
        let batch = engine.dispatch_ready(&mut scheduled).unwrap().unwrap();
        assert_eq!(scheduled, [first, second]);
        let completion = ExactCompletion::from_contracted_hsa_quiescence(batch.epoch());
        let roster = M1DeviceKvCompletionRosterV1::new(vec![
            M1DeviceKvCompletionMemberV1::continuing(cache(
                second,
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS1C8192,
            )),
            M1DeviceKvCompletionMemberV1::continuing(cache(
                first,
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS1C8192,
            )),
        ]);
        assert_eq!(
            preflight_engine(&engine, &completion, &roster, &[1, 1]),
            Err(M1CompletedStepErrorV1::RequestOrder { lane: 0 })
        );
        assert_eq!(engine.completed_epoch(), CompletionEpoch::new(0));
        assert_eq!(engine.state(first), Some(RequestState::InFlight));
        assert_eq!(engine.state(second), Some(RequestState::InFlight));
    }
}
