//! Transactional return of completed-step retired KV pages.
//!
//! The join consumes one successful completed-step owner because that owner is
//! the first phase that pairs the scheduler-ordered cache roster with the exact
//! post-readback queue retaining the private page-generation ledger. Every
//! roster and page check, including all host allocation, completes before any
//! lease or ledger mutation. Commit then returns draft pages before target
//! pages, preserving member order within each role.
//!
//! This layer does not construct KFD authority, rearm a queue, or expose a
//! general leasing API. Hardware-backed end-to-end construction remains gated
//! by the existing physical queue creation path; the pure preflight and ledger
//! transitions are covered independently here and in `device_cache`.

use core::fmt;

use fe2o3_service_host::ServiceQueueReleaseObservationV1;
use ferric_spec::completion::CompletionEpoch;
use ferric_spec::{PhysicalKvLifecycle, PhysicalPageId, Qwen3ModelRole, RequestId};

use crate::device_cache::{
    M1KvPageReturnErrorV1, M1PreflightedKvPageReturnV1, RetiredPageLease,
    M1_KV_PAGE_RETURN_ROLE_ORDER_V1,
};
use crate::{
    ActiveDeviceKvCache, DeviceKvCacheProjection, M1CheckedCompletionOutputV1,
    M1CompletedDeviceKvMemberV1, M1CompletedStepSuccessV1, M1DeviceKvArenaLeaseErrorV1,
    M1PhysicalReadbackQueueReleaseFailureV1, M1PhysicalReadbackQueueSessionV1,
};

/// Stable whole-roster page-return rejection.
#[derive(Debug)]
pub enum M1CompletedStepKvReleaseErrorV1 {
    /// A bounded host preflight allocation failed before mutation.
    HostAllocation,
    /// The closed queue's retained model/partition/page ledger drifted.
    Authority(M1DeviceKvArenaLeaseErrorV1),
    /// One successful completion cardinality no longer matches its roster.
    MemberCount { expected: usize, actual: usize },
    /// The Engine completion count no longer names the whole roster.
    CompletedMemberCount { expected: usize, actual: usize },
    /// The checked record and completed cache disagree in scheduler order.
    RequestOrder { lane: usize },
    /// Two lanes repeat one generational request.
    DuplicateRequest { first_lane: usize, lane: usize },
    /// The checked output and retained queue select different physical graphs.
    Selection,
    /// The post-readback queue enum disagrees with its retained selection.
    Shape,
    /// One terminal member did not retain the checked completion epoch.
    TerminalEpoch { lane: usize },
    /// One raw K7 compact count drifted from its checked semantic case.
    RawCompactCount { lane: usize },
    /// One Engine logical-accept count drifted from checked semantics.
    LogicalAcceptedCount { lane: usize },
    /// One external-publication count drifted from checked semantics.
    ExternallyPublishedCount { lane: usize },
    /// One cache is pending, structurally inconsistent, or in the wrong lifecycle.
    CacheState { lane: usize, role: Qwen3ModelRole },
    /// One cache belongs to a different physical device.
    CacheDevice { lane: usize },
    /// One cache arena identity differs from the queue-retained role owner.
    CacheAllocation { lane: usize, role: Qwen3ModelRole },
    /// A retired page was not quiescent before return.
    NonquiescentPage {
        lane: usize,
        role: Qwen3ModelRole,
        page: PhysicalPageId,
    },
    /// A retirement epoch was zero or later than the completed step.
    RetirementEpoch {
        lane: usize,
        role: Qwen3ModelRole,
        page: PhysicalPageId,
    },
    /// One lease failed exact device/allocation/request/role/index/ledger checks.
    PageIdentity {
        lane: usize,
        role: Qwen3ModelRole,
        page: PhysicalPageId,
        source: M1CompletedKvPageIdentityErrorV1,
    },
    /// Two retained lease owners name the same role-scoped global ledger slot.
    DuplicatePage {
        first_lane: usize,
        lane: usize,
        role: Qwen3ModelRole,
        global_index: usize,
    },
}

impl fmt::Display for M1CompletedStepKvReleaseErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "M1 completed-step KV release rejected: {self:?}")
    }
}

impl std::error::Error for M1CompletedStepKvReleaseErrorV1 {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Authority(source) => Some(source),
            _ => None,
        }
    }
}

/// Stable exact-page diagnostic without exposing the private ledger.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1CompletedKvPageIdentityErrorV1 {
    WrongDevice,
    WrongAllocation,
    WrongRequest,
    WrongRole,
    PageOutOfRange,
    LedgerMismatch,
    GenerationExhausted,
}

/// Retry-safe rejection retaining the exact unchanged completed-step owner.
#[must_use = "a rejected completed-step owner remains the sole retry input"]
#[derive(Debug)]
pub struct M1CompletedStepKvReleaseFailureV1 {
    error: M1CompletedStepKvReleaseErrorV1,
    completed: M1CompletedStepSuccessV1,
}

/// ```compile_fail
/// use ferric_engine::{release_m1_completed_step_kv_pages_v1, M1CompletedStepSuccessV1};
/// fn return_twice(completed: M1CompletedStepSuccessV1) {
///     let _first = release_m1_completed_step_kv_pages_v1(completed);
///     let _second = release_m1_completed_step_kv_pages_v1(completed);
/// }
/// ```
impl M1CompletedStepKvReleaseFailureV1 {
    #[must_use]
    pub const fn error(&self) -> &M1CompletedStepKvReleaseErrorV1 {
        &self.error
    }

    #[must_use = "the unchanged completed-step owner remains linear"]
    pub fn into_parts(self) -> (M1CompletedStepKvReleaseErrorV1, M1CompletedStepSuccessV1) {
        (self.error, self.completed)
    }
}

/// Per-member counts committed in deterministic draft-then-target order.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct M1CompletedKvPageReleaseCountsV1 {
    draft: usize,
    target: usize,
}

impl M1CompletedKvPageReleaseCountsV1 {
    #[must_use]
    pub const fn draft(self) -> usize {
        self.draft
    }

    #[must_use]
    pub const fn target(self) -> usize {
        self.target
    }

    #[must_use]
    pub const fn total(self) -> usize {
        self.draft + self.target
    }
}

/// Copy-only terminal observation after every retired lease returned.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1ReleasedTerminalDeviceKvMemberV1 {
    request: RequestId,
    completion_epoch: CompletionEpoch,
    target: ferric_spec::LogicalKvState,
    draft: ferric_spec::LogicalKvState,
    released: M1CompletedKvPageReleaseCountsV1,
}

impl M1ReleasedTerminalDeviceKvMemberV1 {
    #[must_use]
    pub const fn request(self) -> RequestId {
        self.request
    }

    #[must_use]
    pub const fn completion_epoch(self) -> CompletionEpoch {
        self.completion_epoch
    }

    #[must_use]
    pub const fn target(self) -> ferric_spec::LogicalKvState {
        self.target
    }

    #[must_use]
    pub const fn draft(self) -> ferric_spec::LogicalKvState {
        self.draft
    }

    #[must_use]
    pub const fn released(self) -> M1CompletedKvPageReleaseCountsV1 {
        self.released
    }
}

/// Post-release member custody in original scheduler order.
#[must_use = "active cache custody must remain paired with the released step"]
#[derive(Debug, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum M1ReleasedDeviceKvMemberV1 {
    Active(ActiveDeviceKvCache),
    Terminal(M1ReleasedTerminalDeviceKvMemberV1),
}

impl M1ReleasedDeviceKvMemberV1 {
    #[must_use]
    pub fn request(&self) -> RequestId {
        match self {
            Self::Active(cache) => cache.projection().request,
            Self::Terminal(observation) => observation.request(),
        }
    }
}

/// Closed post-release owner retaining queue, output, and active-cache custody.
///
/// There is deliberately no general `into_parts`, queue-custody extraction, or
/// page-leasing method. A later queue rearm bridge must consume this exact owner.
///
/// ```compile_fail
/// use ferric_engine::M1ReleasedCompletedStepV1;
/// fn split(released: M1ReleasedCompletedStepV1) {
///     let _ = released.into_parts();
/// }
/// ```
///
/// ```compile_fail
/// use ferric_engine::M1ReleasedCompletedStepV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1ReleasedCompletedStepV1>();
/// ```
#[must_use = "released queue and active KV custody must remain paired"]
#[derive(Debug)]
pub struct M1ReleasedCompletedStepV1 {
    queue: M1PhysicalReadbackQueueSessionV1,
    checked: M1CheckedCompletionOutputV1,
    members: Vec<M1ReleasedDeviceKvMemberV1>,
    logical_accepted_counts: Box<[u32]>,
    externally_published_counts: Box<[u32]>,
    release_counts: Box<[M1CompletedKvPageReleaseCountsV1]>,
    completed_members: usize,
    total_released: usize,
}

/// Successful final queue release retaining the completed-step observations.
#[must_use = "released member and completion observations remain owned"]
#[derive(Debug)]
pub struct M1ReleasedQueueTeardownSuccessV1 {
    queue_release: ServiceQueueReleaseObservationV1,
    checked: M1CheckedCompletionOutputV1,
    members: Vec<M1ReleasedDeviceKvMemberV1>,
    logical_accepted_counts: Box<[u32]>,
    externally_published_counts: Box<[u32]>,
    release_counts: Box<[M1CompletedKvPageReleaseCountsV1]>,
    completed_members: usize,
    total_released: usize,
}

impl M1ReleasedQueueTeardownSuccessV1 {
    #[must_use]
    pub const fn queue_release(&self) -> ServiceQueueReleaseObservationV1 {
        self.queue_release
    }

    pub const fn checked(&self) -> &M1CheckedCompletionOutputV1 {
        &self.checked
    }

    pub fn members(&self) -> &[M1ReleasedDeviceKvMemberV1] {
        &self.members
    }

    #[must_use]
    pub fn logical_accepted_counts(&self) -> &[u32] {
        &self.logical_accepted_counts
    }

    #[must_use]
    pub fn externally_published_counts(&self) -> &[u32] {
        &self.externally_published_counts
    }

    #[must_use]
    pub fn release_counts(&self) -> &[M1CompletedKvPageReleaseCountsV1] {
        &self.release_counts
    }

    #[must_use]
    pub const fn completed_members(&self) -> usize {
        self.completed_members
    }

    #[must_use]
    pub const fn total_released(&self) -> usize {
        self.total_released
    }
}

/// Terminal queue-release failure retaining all available completed-step custody.
#[must_use = "terminal queue release failure retains physical and member custody"]
#[derive(Debug)]
pub struct M1ReleasedQueueTeardownFailureV1 {
    source: M1PhysicalReadbackQueueReleaseFailureV1,
    checked: M1CheckedCompletionOutputV1,
    members: Vec<M1ReleasedDeviceKvMemberV1>,
    logical_accepted_counts: Box<[u32]>,
    externally_published_counts: Box<[u32]>,
    release_counts: Box<[M1CompletedKvPageReleaseCountsV1]>,
    completed_members: usize,
    total_released: usize,
}

impl M1ReleasedQueueTeardownFailureV1 {
    pub const fn source(&self) -> &M1PhysicalReadbackQueueReleaseFailureV1 {
        &self.source
    }

    pub const fn checked(&self) -> &M1CheckedCompletionOutputV1 {
        &self.checked
    }

    pub fn members(&self) -> &[M1ReleasedDeviceKvMemberV1] {
        &self.members
    }

    #[must_use]
    pub fn logical_accepted_counts(&self) -> &[u32] {
        &self.logical_accepted_counts
    }

    #[must_use]
    pub fn externally_published_counts(&self) -> &[u32] {
        &self.externally_published_counts
    }

    #[must_use]
    pub fn release_counts(&self) -> &[M1CompletedKvPageReleaseCountsV1] {
        &self.release_counts
    }

    #[must_use]
    pub const fn completed_members(&self) -> usize {
        self.completed_members
    }

    #[must_use]
    pub const fn total_released(&self) -> usize {
        self.total_released
    }
}

impl M1ReleasedCompletedStepV1 {
    pub const fn queue(&self) -> &M1PhysicalReadbackQueueSessionV1 {
        &self.queue
    }

    pub const fn checked(&self) -> &M1CheckedCompletionOutputV1 {
        &self.checked
    }

    pub fn members(&self) -> &[M1ReleasedDeviceKvMemberV1] {
        &self.members
    }

    #[must_use]
    pub fn logical_accepted_counts(&self) -> &[u32] {
        &self.logical_accepted_counts
    }

    #[must_use]
    pub fn externally_published_counts(&self) -> &[u32] {
        &self.externally_published_counts
    }

    #[must_use]
    pub fn release_counts(&self) -> &[M1CompletedKvPageReleaseCountsV1] {
        &self.release_counts
    }

    #[must_use]
    pub const fn completed_members(&self) -> usize {
        self.completed_members
    }

    #[must_use]
    pub const fn total_released(&self) -> usize {
        self.total_released
    }

    /// Destroys the completed queue and releases its allocation session while
    /// retaining the completed-step member and observation custody.
    ///
    /// This is the clean terminal route when no request continues. It does not
    /// claim that an active member may be discarded; any such owner remains in
    /// the returned closed success or failure value.
    ///
    /// # Errors
    ///
    /// Returns terminal lower-layer release quarantine paired with every
    /// remaining completed-step owner and observation.
    pub fn destroy_queue_and_retain_step(
        self,
    ) -> Result<M1ReleasedQueueTeardownSuccessV1, Box<M1ReleasedQueueTeardownFailureV1>> {
        let Self {
            queue,
            checked,
            members,
            logical_accepted_counts,
            externally_published_counts,
            release_counts,
            completed_members,
            total_released,
        } = self;
        match queue.destroy_and_release() {
            Ok(queue_release) => Ok(M1ReleasedQueueTeardownSuccessV1 {
                queue_release,
                checked,
                members,
                logical_accepted_counts,
                externally_published_counts,
                release_counts,
                completed_members,
                total_released,
            }),
            Err(source) => Err(Box::new(M1ReleasedQueueTeardownFailureV1 {
                source,
                checked,
                members,
                logical_accepted_counts,
                externally_published_counts,
                release_counts,
                completed_members,
                total_released,
            })),
        }
    }

    pub(crate) fn try_reserve_rearm_members(
        &mut self,
        additional: usize,
    ) -> Result<(), std::collections::TryReserveError> {
        self.members.try_reserve_exact(additional)
    }

    #[allow(clippy::type_complexity)]
    pub(crate) fn into_rearm_parts(
        self,
    ) -> (
        M1PhysicalReadbackQueueSessionV1,
        M1CheckedCompletionOutputV1,
        Vec<M1ReleasedDeviceKvMemberV1>,
        Box<[u32]>,
        Box<[u32]>,
        Box<[M1CompletedKvPageReleaseCountsV1]>,
        usize,
        usize,
    ) {
        (
            self.queue,
            self.checked,
            self.members,
            self.logical_accepted_counts,
            self.externally_published_counts,
            self.release_counts,
            self.completed_members,
            self.total_released,
        )
    }
}

#[derive(Debug)]
struct MemberReleasePlanV1 {
    draft: Vec<M1PreflightedKvPageReturnV1>,
    target: Vec<M1PreflightedKvPageReturnV1>,
    counts: M1CompletedKvPageReleaseCountsV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SeenPageV1 {
    role: Qwen3ModelRole,
    global_index: usize,
    lane: usize,
}

fn member_projection(member: &M1CompletedDeviceKvMemberV1) -> DeviceKvCacheProjection {
    match member {
        M1CompletedDeviceKvMemberV1::Active(cache) => cache.projection(),
        M1CompletedDeviceKvMemberV1::Quiescent(cache) => cache.projection(),
    }
}

fn member_release_state_is_valid(member: &M1CompletedDeviceKvMemberV1) -> bool {
    match member {
        M1CompletedDeviceKvMemberV1::Active(cache) => cache.release_state_is_valid(),
        M1CompletedDeviceKvMemberV1::Quiescent(cache) => cache.release_state_is_valid(),
    }
}

fn member_retired_pages(
    member: &M1CompletedDeviceKvMemberV1,
    role: Qwen3ModelRole,
) -> &[RetiredPageLease] {
    match member {
        M1CompletedDeviceKvMemberV1::Active(cache) => cache.retired_pages(role),
        M1CompletedDeviceKvMemberV1::Quiescent(cache) => cache.retired_pages(role),
    }
}

fn member_take_retired_pages(
    member: &mut M1CompletedDeviceKvMemberV1,
    role: Qwen3ModelRole,
) -> Vec<RetiredPageLease> {
    match member {
        M1CompletedDeviceKvMemberV1::Active(cache) => cache.take_retired_pages(role),
        M1CompletedDeviceKvMemberV1::Quiescent(cache) => cache.take_retired_pages(role),
    }
}

fn role_lifecycle_is_valid(
    member: &M1CompletedDeviceKvMemberV1,
    lifecycle: PhysicalKvLifecycle,
    checked_epoch: CompletionEpoch,
) -> bool {
    match member {
        M1CompletedDeviceKvMemberV1::Active(_) => lifecycle == PhysicalKvLifecycle::Active,
        M1CompletedDeviceKvMemberV1::Quiescent(_) => matches!(
            lifecycle,
            PhysicalKvLifecycle::RetiredAwaitingQuiescence { after_epoch }
                if after_epoch == checked_epoch
        ),
    }
}

fn map_page_error(error: M1KvPageReturnErrorV1) -> M1CompletedKvPageIdentityErrorV1 {
    match error {
        M1KvPageReturnErrorV1::Device => M1CompletedKvPageIdentityErrorV1::WrongDevice,
        M1KvPageReturnErrorV1::Allocation => M1CompletedKvPageIdentityErrorV1::WrongAllocation,
        M1KvPageReturnErrorV1::Request => M1CompletedKvPageIdentityErrorV1::WrongRequest,
        M1KvPageReturnErrorV1::Role => M1CompletedKvPageIdentityErrorV1::WrongRole,
        M1KvPageReturnErrorV1::Index => M1CompletedKvPageIdentityErrorV1::PageOutOfRange,
        M1KvPageReturnErrorV1::Ledger => M1CompletedKvPageIdentityErrorV1::LedgerMismatch,
        M1KvPageReturnErrorV1::GenerationExhausted => {
            M1CompletedKvPageIdentityErrorV1::GenerationExhausted
        }
    }
}

fn validate_request_lane(
    lane: usize,
    member_request: RequestId,
    record_request: RequestId,
) -> Result<(), M1CompletedStepKvReleaseErrorV1> {
    if member_request != record_request {
        return Err(M1CompletedStepKvReleaseErrorV1::RequestOrder { lane });
    }
    Ok(())
}

fn validate_retirement(
    lane: usize,
    role: Qwen3ModelRole,
    page: PhysicalPageId,
    quiescent: bool,
    after_epoch: CompletionEpoch,
    checked_epoch: CompletionEpoch,
) -> Result<(), M1CompletedStepKvReleaseErrorV1> {
    if !quiescent {
        return Err(M1CompletedStepKvReleaseErrorV1::NonquiescentPage { lane, role, page });
    }
    if after_epoch.value() == 0 || after_epoch.value() > checked_epoch.value() {
        return Err(M1CompletedStepKvReleaseErrorV1::RetirementEpoch { lane, role, page });
    }
    Ok(())
}

fn validate_unique_seen_pages(
    seen: &mut [SeenPageV1],
) -> Result<(), M1CompletedStepKvReleaseErrorV1> {
    seen.sort_unstable_by_key(|entry| {
        (
            match entry.role {
                Qwen3ModelRole::Draft06B => 0u8,
                Qwen3ModelRole::Target8B => 1u8,
            },
            entry.global_index,
        )
    });
    for pair in seen.windows(2) {
        if pair[0].role == pair[1].role && pair[0].global_index == pair[1].global_index {
            return Err(M1CompletedStepKvReleaseErrorV1::DuplicatePage {
                first_lane: pair[0].lane,
                lane: pair[1].lane,
                role: pair[1].role,
                global_index: pair[1].global_index,
            });
        }
    }
    Ok(())
}

fn validate_roster(
    completed: &M1CompletedStepSuccessV1,
) -> Result<usize, M1CompletedStepKvReleaseErrorV1> {
    let member_count = completed.members().len();
    for actual in [
        completed.checked().records().len(),
        completed.logical_accepted_counts().len(),
        completed.externally_published_counts().len(),
    ] {
        if actual != member_count {
            return Err(M1CompletedStepKvReleaseErrorV1::MemberCount {
                expected: member_count,
                actual,
            });
        }
    }
    if completed.completed_members() != member_count {
        return Err(M1CompletedStepKvReleaseErrorV1::CompletedMemberCount {
            expected: member_count,
            actual: completed.completed_members(),
        });
    }
    if completed.queue().custody().selection() != completed.checked().selection() {
        return Err(M1CompletedStepKvReleaseErrorV1::Selection);
    }
    if completed.queue().custody().retained_intent_shape() != Some(completed.queue().shape()) {
        return Err(M1CompletedStepKvReleaseErrorV1::Shape);
    }
    let checked_epoch = completed.checked().epoch();
    let mut total_pages = 0usize;
    for (lane, member) in completed.members().iter().enumerate() {
        let projection = member_projection(member);
        let record = completed.checked().records()[lane].record();
        validate_request_lane(lane, projection.request, record.request)?;
        validate_request_lane(lane, member.request(), record.request)?;
        let semantics = completed.checked().records()[lane].semantics();
        if u32::from(record.emitted_token_count) != semantics.raw_compact_count() {
            return Err(M1CompletedStepKvReleaseErrorV1::RawCompactCount { lane });
        }
        if completed.logical_accepted_counts()[lane] != semantics.logical_accepted_count() {
            return Err(M1CompletedStepKvReleaseErrorV1::LogicalAcceptedCount { lane });
        }
        if completed.externally_published_counts()[lane] != semantics.externally_published_count() {
            return Err(M1CompletedStepKvReleaseErrorV1::ExternallyPublishedCount { lane });
        }
        for first_lane in 0..lane {
            if completed.members()[first_lane].request() == member.request() {
                return Err(M1CompletedStepKvReleaseErrorV1::DuplicateRequest { first_lane, lane });
            }
        }
        if !member_release_state_is_valid(member) {
            return Err(M1CompletedStepKvReleaseErrorV1::CacheState {
                lane,
                role: Qwen3ModelRole::Draft06B,
            });
        }
        for (role, logical) in [
            (Qwen3ModelRole::Draft06B, projection.draft),
            (Qwen3ModelRole::Target8B, projection.target),
        ] {
            if logical.request != projection.request
                || logical.role != role
                || !role_lifecycle_is_valid(member, logical.lifecycle, checked_epoch)
            {
                return Err(M1CompletedStepKvReleaseErrorV1::CacheState { lane, role });
            }
            total_pages = total_pages
                .checked_add(member_retired_pages(member, role).len())
                .ok_or(M1CompletedStepKvReleaseErrorV1::HostAllocation)?;
        }
        if let M1CompletedDeviceKvMemberV1::Quiescent(cache) = member {
            if cache.completion_epoch() != checked_epoch {
                return Err(M1CompletedStepKvReleaseErrorV1::TerminalEpoch { lane });
            }
        }
    }
    Ok(total_pages)
}

fn preflight_all(
    completed: &M1CompletedStepSuccessV1,
) -> Result<Vec<MemberReleasePlanV1>, M1CompletedStepKvReleaseErrorV1> {
    let total_pages = validate_roster(completed)?;
    let partitioned = completed.queue().custody().partitioned_memory();
    partitioned
        .revalidate_page_return_authority()
        .map_err(M1CompletedStepKvReleaseErrorV1::Authority)?;

    let mut plans = Vec::new();
    plans
        .try_reserve_exact(completed.members().len())
        .map_err(|_| M1CompletedStepKvReleaseErrorV1::HostAllocation)?;
    let mut seen = Vec::new();
    seen.try_reserve_exact(total_pages)
        .map_err(|_| M1CompletedStepKvReleaseErrorV1::HostAllocation)?;
    let checked_epoch = completed.checked().epoch();

    for (lane, member) in completed.members().iter().enumerate() {
        let projection = member_projection(member);
        if projection.device != partitioned.device() {
            return Err(M1CompletedStepKvReleaseErrorV1::CacheDevice { lane });
        }
        for (role, arena) in [
            (
                Qwen3ModelRole::Draft06B,
                projection.draft_arena_allocation_id,
            ),
            (
                Qwen3ModelRole::Target8B,
                projection.target_arena_allocation_id,
            ),
        ] {
            if arena.is_some_and(|identity| !identity.equals(&partitioned.allocation_id(role))) {
                return Err(M1CompletedStepKvReleaseErrorV1::CacheAllocation { lane, role });
            }
        }
        let mut role_plans: [Vec<M1PreflightedKvPageReturnV1>; 2] = [Vec::new(), Vec::new()];
        for (role_index, role) in [Qwen3ModelRole::Draft06B, Qwen3ModelRole::Target8B]
            .into_iter()
            .enumerate()
        {
            let retired_pages = member_retired_pages(member, role);
            role_plans[role_index]
                .try_reserve_exact(retired_pages.len())
                .map_err(|_| M1CompletedStepKvReleaseErrorV1::HostAllocation)?;
            for retired in retired_pages {
                let page = retired.lease().page();
                validate_retirement(
                    lane,
                    role,
                    page,
                    retired.is_quiescent(),
                    retired.after_epoch(),
                    checked_epoch,
                )?;
                if retired.lease().request() != member.request() {
                    return Err(M1CompletedStepKvReleaseErrorV1::PageIdentity {
                        lane,
                        role,
                        page,
                        source: M1CompletedKvPageIdentityErrorV1::WrongRequest,
                    });
                }
                let ticket = partitioned
                    .preflight_page_return(role, retired.lease())
                    .map_err(|source| M1CompletedStepKvReleaseErrorV1::PageIdentity {
                        lane,
                        role,
                        page,
                        source: map_page_error(source),
                    })?;
                seen.push(SeenPageV1 {
                    role,
                    global_index: ticket.global_index,
                    lane,
                });
                role_plans[role_index].push(ticket);
            }
        }
        let [draft, target] = role_plans;
        plans.push(MemberReleasePlanV1 {
            counts: M1CompletedKvPageReleaseCountsV1 {
                draft: draft.len(),
                target: target.len(),
            },
            draft,
            target,
        });
    }

    validate_unique_seen_pages(&mut seen)?;
    Ok(plans)
}

fn commit_role(
    queue: &mut M1PhysicalReadbackQueueSessionV1,
    members: &mut [M1CompletedDeviceKvMemberV1],
    plans: &mut [MemberReleasePlanV1],
    role: Qwen3ModelRole,
) {
    for (member, plan) in members.iter_mut().zip(plans.iter_mut()) {
        let retired = member_take_retired_pages(member, role);
        let tickets = match role {
            Qwen3ModelRole::Draft06B => core::mem::take(&mut plan.draft),
            Qwen3ModelRole::Target8B => core::mem::take(&mut plan.target),
        };
        for (retired, ticket) in retired.into_iter().zip(tickets) {
            queue
                .custody_mut()
                .partitioned_memory_mut()
                .commit_page_return(ticket, retired.into_lease());
        }
    }
}

/// Returns every quiescent retired page in a completed roster to its exact pool.
///
/// Failure retains the byte-for-byte ownership graph of the input for retry.
/// Success advances each returned page generation exactly once. Commit order is
/// all draft lanes followed by all target lanes.
///
/// # Errors
///
/// Rejects incomplete or reordered rosters, duplicate requests or pages,
/// nonquiescent retirement, stale/substituted page identity, allocation or
/// device drift, and generation exhaustion before any mutation.
pub fn release_m1_completed_step_kv_pages_v1(
    completed: M1CompletedStepSuccessV1,
) -> Result<M1ReleasedCompletedStepV1, Box<M1CompletedStepKvReleaseFailureV1>> {
    let mut plans = match preflight_all(&completed) {
        Ok(plans) => plans,
        Err(error) => {
            return Err(Box::new(M1CompletedStepKvReleaseFailureV1 {
                error,
                completed,
            }));
        }
    };

    let member_count = completed.members().len();
    let mut released_members = Vec::new();
    if released_members.try_reserve_exact(member_count).is_err() {
        return Err(Box::new(M1CompletedStepKvReleaseFailureV1 {
            error: M1CompletedStepKvReleaseErrorV1::HostAllocation,
            completed,
        }));
    }
    let mut release_counts = Vec::new();
    if release_counts.try_reserve_exact(member_count).is_err() {
        return Err(Box::new(M1CompletedStepKvReleaseFailureV1 {
            error: M1CompletedStepKvReleaseErrorV1::HostAllocation,
            completed,
        }));
    }
    release_counts.extend(plans.iter().map(|plan| plan.counts));
    let total_released = release_counts.iter().map(|counts| counts.total()).sum();

    let (
        mut queue,
        checked,
        mut members,
        logical_accepted_counts,
        externally_published_counts,
        completed_members,
    ) = completed.into_release_parts();
    for role in M1_KV_PAGE_RETURN_ROLE_ORDER_V1 {
        commit_role(&mut queue, &mut members, &mut plans, role);
    }

    for (member, released) in members.into_iter().zip(release_counts.iter().copied()) {
        match member {
            M1CompletedDeviceKvMemberV1::Active(cache) => {
                released_members.push(M1ReleasedDeviceKvMemberV1::Active(cache));
            }
            M1CompletedDeviceKvMemberV1::Quiescent(cache) => {
                let projection = cache.projection();
                released_members.push(M1ReleasedDeviceKvMemberV1::Terminal(
                    M1ReleasedTerminalDeviceKvMemberV1 {
                        request: projection.request,
                        completion_epoch: cache.completion_epoch(),
                        target: projection.target,
                        draft: projection.draft,
                        released,
                    },
                ));
            }
        }
    }

    Ok(M1ReleasedCompletedStepV1 {
        queue,
        checked,
        members: released_members,
        logical_accepted_counts,
        externally_published_counts,
        release_counts: release_counts.into_boxed_slice(),
        completed_members,
        total_released,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_identity_errors_remain_exact_and_distinct() {
        assert_eq!(
            map_page_error(M1KvPageReturnErrorV1::Device),
            M1CompletedKvPageIdentityErrorV1::WrongDevice
        );
        assert_eq!(
            map_page_error(M1KvPageReturnErrorV1::Allocation),
            M1CompletedKvPageIdentityErrorV1::WrongAllocation
        );
        assert_eq!(
            map_page_error(M1KvPageReturnErrorV1::Request),
            M1CompletedKvPageIdentityErrorV1::WrongRequest
        );
        assert_eq!(
            map_page_error(M1KvPageReturnErrorV1::Role),
            M1CompletedKvPageIdentityErrorV1::WrongRole
        );
        assert_eq!(
            map_page_error(M1KvPageReturnErrorV1::Index),
            M1CompletedKvPageIdentityErrorV1::PageOutOfRange
        );
        assert_eq!(
            map_page_error(M1KvPageReturnErrorV1::Ledger),
            M1CompletedKvPageIdentityErrorV1::LedgerMismatch
        );
        assert_eq!(
            map_page_error(M1KvPageReturnErrorV1::GenerationExhausted),
            M1CompletedKvPageIdentityErrorV1::GenerationExhausted
        );
    }

    #[test]
    fn release_counts_preserve_draft_then_target_accounting() {
        let counts = M1CompletedKvPageReleaseCountsV1 {
            draft: 3,
            target: 5,
        };
        assert_eq!(counts.draft(), 3);
        assert_eq!(counts.target(), 5);
        assert_eq!(counts.total(), 8);
    }

    #[test]
    fn scheduler_order_and_cross_generation_substitution_fail_closed() {
        let request = RequestId::new(7, 11);
        assert!(validate_request_lane(0, request, request).is_ok());
        assert!(matches!(
            validate_request_lane(3, request, RequestId::new(7, 12)),
            Err(M1CompletedStepKvReleaseErrorV1::RequestOrder { lane: 3 })
        ));
        assert!(matches!(
            validate_request_lane(4, request, RequestId::new(8, 11)),
            Err(M1CompletedStepKvReleaseErrorV1::RequestOrder { lane: 4 })
        ));
    }

    #[test]
    fn duplicate_global_page_is_role_scoped_and_rejected() {
        let mut duplicate = [
            SeenPageV1 {
                role: Qwen3ModelRole::Target8B,
                global_index: 513,
                lane: 0,
            },
            SeenPageV1 {
                role: Qwen3ModelRole::Target8B,
                global_index: 513,
                lane: 1,
            },
        ];
        assert!(matches!(
            validate_unique_seen_pages(&mut duplicate),
            Err(M1CompletedStepKvReleaseErrorV1::DuplicatePage {
                first_lane: 0,
                lane: 1,
                role: Qwen3ModelRole::Target8B,
                global_index: 513,
            })
        ));

        let mut distinct_roles = [
            SeenPageV1 {
                role: Qwen3ModelRole::Draft06B,
                global_index: 513,
                lane: 0,
            },
            SeenPageV1 {
                role: Qwen3ModelRole::Target8B,
                global_index: 513,
                lane: 0,
            },
        ];
        assert!(validate_unique_seen_pages(&mut distinct_roles).is_ok());
    }

    #[test]
    fn nonquiescent_future_and_zero_retirement_epochs_fail_closed() {
        let page = PhysicalPageId::new(Qwen3ModelRole::Draft06B, 9, 4);
        let checked = CompletionEpoch::new(10);
        assert!(matches!(
            validate_retirement(
                2,
                Qwen3ModelRole::Draft06B,
                page,
                false,
                CompletionEpoch::new(10),
                checked,
            ),
            Err(M1CompletedStepKvReleaseErrorV1::NonquiescentPage { lane: 2, .. })
        ));
        assert!(matches!(
            validate_retirement(
                2,
                Qwen3ModelRole::Draft06B,
                page,
                true,
                CompletionEpoch::new(11),
                checked,
            ),
            Err(M1CompletedStepKvReleaseErrorV1::RetirementEpoch { lane: 2, .. })
        ));
        assert!(matches!(
            validate_retirement(
                2,
                Qwen3ModelRole::Draft06B,
                page,
                true,
                CompletionEpoch::new(0),
                checked,
            ),
            Err(M1CompletedStepKvReleaseErrorV1::RetirementEpoch { lane: 2, .. })
        ));
    }
}
