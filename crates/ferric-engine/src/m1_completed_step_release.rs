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

use fe2o3_host::AuthenticatedServiceQueueReleaseV1;
use fe2o3_service_host::ServiceQueueReleaseObservationV1;
use ferric_spec::completion::CompletionEpoch;
use ferric_spec::{PhysicalKvLifecycle, PhysicalPageId, Qwen3ModelRole, RequestId};

use crate::device_cache::{
    M1KvPageReturnErrorV1, M1PreflightedKvPageReturnV1, RetiredPageLease,
    M1_KV_PAGE_RETURN_ROLE_ORDER_V1,
};
use crate::{
    ActiveDeviceKvCache, DeviceKvCacheProjection, Engine, M1AuthenticatedCompletedStepSuccessV1,
    M1AuthenticatedPhysicalPostReadbackQueueReleaseFailureV1,
    M1AuthenticatedPhysicalReadbackQueueSessionV1, M1CheckedCompletionOutputV1,
    M1CompletedDeviceKvMemberV1, M1CompletedStepSuccessV1, M1DeviceKvArenaLeaseErrorV1,
    M1PhysicalFixedBatchShapeV1, M1PhysicalQueueBatchCustodyV1,
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

/// Retry-safe rejection retaining the exact unchanged authenticated
/// completed-step owner.
#[must_use = "a rejected authenticated completed-step owner remains the sole retry input"]
#[derive(Debug)]
pub struct M1AuthenticatedCompletedStepKvReleaseFailureV1 {
    error: M1CompletedStepKvReleaseErrorV1,
    completed: M1AuthenticatedCompletedStepSuccessV1,
}

/// ```compile_fail
/// use ferric_engine::{
///     release_m1_authenticated_completed_step_kv_pages_v1,
///     M1AuthenticatedCompletedStepSuccessV1,
/// };
/// fn return_twice(completed: M1AuthenticatedCompletedStepSuccessV1) {
///     let _first = release_m1_authenticated_completed_step_kv_pages_v1(completed);
///     let _second = release_m1_authenticated_completed_step_kv_pages_v1(completed);
/// }
/// ```
impl M1AuthenticatedCompletedStepKvReleaseFailureV1 {
    #[must_use]
    pub const fn error(&self) -> &M1CompletedStepKvReleaseErrorV1 {
        &self.error
    }

    /// Recovers the unchanged authenticated completed-step owner for retry.
    #[must_use = "the unchanged authenticated completed-step owner remains linear"]
    pub fn into_parts(
        self,
    ) -> (
        M1CompletedStepKvReleaseErrorV1,
        M1AuthenticatedCompletedStepSuccessV1,
    ) {
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

/// Closed authenticated post-release owner retaining queue, output, and
/// active-cache custody.
///
/// There is deliberately no raw queue conversion or general custody
/// extraction. Authenticated retained rearm must consume this exact owner.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedReleasedCompletedStepV1;
/// fn extract_raw(released: M1AuthenticatedReleasedCompletedStepV1) {
///     let _ = released.into_raw();
/// }
/// ```
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedReleasedCompletedStepV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1AuthenticatedReleasedCompletedStepV1>();
/// ```
#[must_use = "authenticated released queue and active KV custody must remain paired"]
#[derive(Debug)]
pub struct M1AuthenticatedReleasedCompletedStepV1 {
    queue: M1AuthenticatedPhysicalReadbackQueueSessionV1,
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

/// Successful authenticated final queue release retaining completed-step
/// observations and a consuming program-owner handoff.
///
/// ```
/// use ferric_engine::M1AuthenticatedReleasedQueueTeardownSuccessV1;
/// fn recover_programs(owner: M1AuthenticatedReleasedQueueTeardownSuccessV1) {
///     let (release, ..) = owner.into_parts();
///     let _programs = release.into_program_sets();
/// }
/// ```
#[must_use = "authenticated release and completed-step observations remain owned"]
#[derive(Debug)]
pub struct M1AuthenticatedReleasedQueueTeardownSuccessV1 {
    queue_release: AuthenticatedServiceQueueReleaseV1,
    checked: M1CheckedCompletionOutputV1,
    members: Vec<M1ReleasedDeviceKvMemberV1>,
    logical_accepted_counts: Box<[u32]>,
    externally_published_counts: Box<[u32]>,
    release_counts: Box<[M1CompletedKvPageReleaseCountsV1]>,
    completed_members: usize,
    total_released: usize,
}

/// Terminal authenticated queue-release failure retaining every released-step
/// owner and opaque lower quarantine.
///
/// Lower program custody remains recoverable without a raw queue:
///
/// ```
/// use ferric_engine::M1AuthenticatedReleasedQueueTeardownFailureV1;
/// fn recover_programs(owner: M1AuthenticatedReleasedQueueTeardownFailureV1) {
///     let (source, ..) = owner.into_parts();
///     let (lower, _ferric_residue) = source.into_parts();
///     let (_error, _programs) = lower.into_parts();
/// }
/// ```
#[must_use = "authenticated release quarantine and completed-step custody remain retained"]
#[derive(Debug)]
pub struct M1AuthenticatedReleasedQueueTeardownFailureV1 {
    source: M1AuthenticatedPhysicalPostReadbackQueueReleaseFailureV1,
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

impl M1AuthenticatedReleasedQueueTeardownSuccessV1 {
    /// Authenticated program release and native queue-destruction evidence.
    #[must_use = "released authenticated program sets remain explicitly owned"]
    pub const fn queue_release(&self) -> &AuthenticatedServiceQueueReleaseV1 {
        &self.queue_release
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

    /// Separates the authenticated release and every completed-step
    /// observation exactly once.
    #[must_use = "all authenticated teardown owners remain retained"]
    #[allow(clippy::type_complexity)]
    pub fn into_parts(
        self,
    ) -> (
        AuthenticatedServiceQueueReleaseV1,
        M1CheckedCompletionOutputV1,
        Vec<M1ReleasedDeviceKvMemberV1>,
        Box<[u32]>,
        Box<[u32]>,
        Box<[M1CompletedKvPageReleaseCountsV1]>,
        usize,
        usize,
    ) {
        (
            self.queue_release,
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

impl M1AuthenticatedReleasedQueueTeardownFailureV1 {
    #[must_use = "authenticated release quarantine remains retained"]
    pub const fn source(&self) -> &M1AuthenticatedPhysicalPostReadbackQueueReleaseFailureV1 {
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

    /// Separates terminal authenticated quarantine and every released-step
    /// owner exactly once.
    #[must_use = "all authenticated teardown failure owners remain retained"]
    #[allow(clippy::type_complexity)]
    pub fn into_parts(
        self,
    ) -> (
        M1AuthenticatedPhysicalPostReadbackQueueReleaseFailureV1,
        M1CheckedCompletionOutputV1,
        Vec<M1ReleasedDeviceKvMemberV1>,
        Box<[u32]>,
        Box<[u32]>,
        Box<[M1CompletedKvPageReleaseCountsV1]>,
        usize,
        usize,
    ) {
        (
            self.source,
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
    pub fn destroy_queue_and_retain_step<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<M1ReleasedQueueTeardownSuccessV1, Box<M1ReleasedQueueTeardownFailureV1>> {
        engine.quarantine_m1_queue_rearm_failure();
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

impl M1AuthenticatedReleasedCompletedStepV1 {
    /// Authenticated post-readback queue with no raw conversion.
    #[must_use = "authenticated queue custody remains retained"]
    pub const fn queue(&self) -> &M1AuthenticatedPhysicalReadbackQueueSessionV1 {
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

    /// Faults the logical Engine, destroys the authenticated queue, and
    /// retains every released member and observation.
    ///
    /// # Errors
    ///
    /// Returns terminal authenticated release quarantine paired with every
    /// released-step owner and observation.
    pub fn destroy_queue_and_retain_step<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1AuthenticatedReleasedQueueTeardownSuccessV1,
        Box<M1AuthenticatedReleasedQueueTeardownFailureV1>,
    > {
        engine.quarantine_m1_queue_rearm_failure();
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
            Ok(queue_release) => Ok(M1AuthenticatedReleasedQueueTeardownSuccessV1 {
                queue_release,
                checked,
                members,
                logical_accepted_counts,
                externally_published_counts,
                release_counts,
                completed_members,
                total_released,
            }),
            Err(source) => Err(Box::new(M1AuthenticatedReleasedQueueTeardownFailureV1 {
                source: *source,
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
        M1AuthenticatedPhysicalReadbackQueueSessionV1,
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

trait M1CompletedStepKvReleaseQueueV1 {
    fn shape(&self) -> M1PhysicalFixedBatchShapeV1;
    fn custody(&self) -> &M1PhysicalQueueBatchCustodyV1;
    fn custody_mut(&mut self) -> &mut M1PhysicalQueueBatchCustodyV1;
}

impl M1CompletedStepKvReleaseQueueV1 for M1PhysicalReadbackQueueSessionV1 {
    fn shape(&self) -> M1PhysicalFixedBatchShapeV1 {
        M1PhysicalReadbackQueueSessionV1::shape(self)
    }

    fn custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        M1PhysicalReadbackQueueSessionV1::custody(self)
    }

    fn custody_mut(&mut self) -> &mut M1PhysicalQueueBatchCustodyV1 {
        M1PhysicalReadbackQueueSessionV1::custody_mut(self)
    }
}

impl M1CompletedStepKvReleaseQueueV1 for M1AuthenticatedPhysicalReadbackQueueSessionV1 {
    fn shape(&self) -> M1PhysicalFixedBatchShapeV1 {
        M1AuthenticatedPhysicalReadbackQueueSessionV1::shape(self)
    }

    fn custody(&self) -> &M1PhysicalQueueBatchCustodyV1 {
        M1AuthenticatedPhysicalReadbackQueueSessionV1::custody(self)
    }

    fn custody_mut(&mut self) -> &mut M1PhysicalQueueBatchCustodyV1 {
        M1AuthenticatedPhysicalReadbackQueueSessionV1::custody_mut(self)
    }
}

trait M1CompletedStepKvReleaseCarrierV1: Sized {
    type Queue: M1CompletedStepKvReleaseQueueV1;

    fn queue(&self) -> &Self::Queue;
    fn checked(&self) -> &M1CheckedCompletionOutputV1;
    fn members(&self) -> &[M1CompletedDeviceKvMemberV1];
    fn logical_accepted_counts(&self) -> &[u32];
    fn externally_published_counts(&self) -> &[u32];
    fn completed_members(&self) -> usize;

    #[allow(clippy::type_complexity)]
    fn into_release_parts(
        self,
    ) -> (
        Self::Queue,
        M1CheckedCompletionOutputV1,
        Vec<M1CompletedDeviceKvMemberV1>,
        Box<[u32]>,
        Box<[u32]>,
        usize,
    );
}

impl M1CompletedStepKvReleaseCarrierV1 for M1CompletedStepSuccessV1 {
    type Queue = M1PhysicalReadbackQueueSessionV1;

    fn queue(&self) -> &Self::Queue {
        M1CompletedStepSuccessV1::queue(self)
    }

    fn checked(&self) -> &M1CheckedCompletionOutputV1 {
        M1CompletedStepSuccessV1::checked(self)
    }

    fn members(&self) -> &[M1CompletedDeviceKvMemberV1] {
        M1CompletedStepSuccessV1::members(self)
    }

    fn logical_accepted_counts(&self) -> &[u32] {
        M1CompletedStepSuccessV1::logical_accepted_counts(self)
    }

    fn externally_published_counts(&self) -> &[u32] {
        M1CompletedStepSuccessV1::externally_published_counts(self)
    }

    fn completed_members(&self) -> usize {
        M1CompletedStepSuccessV1::completed_members(self)
    }

    #[allow(clippy::type_complexity)]
    fn into_release_parts(
        self,
    ) -> (
        Self::Queue,
        M1CheckedCompletionOutputV1,
        Vec<M1CompletedDeviceKvMemberV1>,
        Box<[u32]>,
        Box<[u32]>,
        usize,
    ) {
        M1CompletedStepSuccessV1::into_release_parts(self)
    }
}

impl M1CompletedStepKvReleaseCarrierV1 for M1AuthenticatedCompletedStepSuccessV1 {
    type Queue = M1AuthenticatedPhysicalReadbackQueueSessionV1;

    fn queue(&self) -> &Self::Queue {
        M1AuthenticatedCompletedStepSuccessV1::queue(self)
    }

    fn checked(&self) -> &M1CheckedCompletionOutputV1 {
        M1AuthenticatedCompletedStepSuccessV1::checked(self)
    }

    fn members(&self) -> &[M1CompletedDeviceKvMemberV1] {
        M1AuthenticatedCompletedStepSuccessV1::members(self)
    }

    fn logical_accepted_counts(&self) -> &[u32] {
        M1AuthenticatedCompletedStepSuccessV1::logical_accepted_counts(self)
    }

    fn externally_published_counts(&self) -> &[u32] {
        M1AuthenticatedCompletedStepSuccessV1::externally_published_counts(self)
    }

    fn completed_members(&self) -> usize {
        M1AuthenticatedCompletedStepSuccessV1::completed_members(self)
    }

    #[allow(clippy::type_complexity)]
    fn into_release_parts(
        self,
    ) -> (
        Self::Queue,
        M1CheckedCompletionOutputV1,
        Vec<M1CompletedDeviceKvMemberV1>,
        Box<[u32]>,
        Box<[u32]>,
        usize,
    ) {
        M1AuthenticatedCompletedStepSuccessV1::into_release_parts(self)
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

fn validate_roster<C>(completed: &C) -> Result<usize, M1CompletedStepKvReleaseErrorV1>
where
    C: M1CompletedStepKvReleaseCarrierV1,
{
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

fn preflight_all<C>(
    completed: &C,
) -> Result<Vec<MemberReleasePlanV1>, M1CompletedStepKvReleaseErrorV1>
where
    C: M1CompletedStepKvReleaseCarrierV1,
{
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

fn commit_role<Q>(
    queue: &mut Q,
    members: &mut [M1CompletedDeviceKvMemberV1],
    plans: &mut [MemberReleasePlanV1],
    role: Qwen3ModelRole,
) where
    Q: M1CompletedStepKvReleaseQueueV1,
{
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

struct M1ReleasedCompletedStepCoreV1<Q> {
    queue: Q,
    checked: M1CheckedCompletionOutputV1,
    members: Vec<M1ReleasedDeviceKvMemberV1>,
    logical_accepted_counts: Box<[u32]>,
    externally_published_counts: Box<[u32]>,
    release_counts: Box<[M1CompletedKvPageReleaseCountsV1]>,
    completed_members: usize,
    total_released: usize,
}

struct M1CompletedStepKvReleaseCoreFailureV1<C> {
    error: M1CompletedStepKvReleaseErrorV1,
    completed: C,
}

fn release_m1_completed_step_kv_pages_core_v1<C>(
    completed: C,
) -> Result<M1ReleasedCompletedStepCoreV1<C::Queue>, Box<M1CompletedStepKvReleaseCoreFailureV1<C>>>
where
    C: M1CompletedStepKvReleaseCarrierV1,
{
    let mut plans = match preflight_all(&completed) {
        Ok(plans) => plans,
        Err(error) => {
            return Err(Box::new(M1CompletedStepKvReleaseCoreFailureV1 {
                error,
                completed,
            }));
        }
    };

    let member_count = completed.members().len();
    let mut released_members = Vec::new();
    if released_members.try_reserve_exact(member_count).is_err() {
        return Err(Box::new(M1CompletedStepKvReleaseCoreFailureV1 {
            error: M1CompletedStepKvReleaseErrorV1::HostAllocation,
            completed,
        }));
    }
    let mut release_counts = Vec::new();
    if release_counts.try_reserve_exact(member_count).is_err() {
        return Err(Box::new(M1CompletedStepKvReleaseCoreFailureV1 {
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

    Ok(M1ReleasedCompletedStepCoreV1 {
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
    match release_m1_completed_step_kv_pages_core_v1(completed) {
        Ok(core) => Ok(M1ReleasedCompletedStepV1 {
            queue: core.queue,
            checked: core.checked,
            members: core.members,
            logical_accepted_counts: core.logical_accepted_counts,
            externally_published_counts: core.externally_published_counts,
            release_counts: core.release_counts,
            completed_members: core.completed_members,
            total_released: core.total_released,
        }),
        Err(failure) => {
            let M1CompletedStepKvReleaseCoreFailureV1 { error, completed } = *failure;
            Err(Box::new(M1CompletedStepKvReleaseFailureV1 {
                error,
                completed,
            }))
        }
    }
}

/// Returns every quiescent retired page in an authenticated completed roster
/// to its exact pool without converting or exposing the lower queue.
///
/// Failure retains the byte-for-byte authenticated ownership graph for retry.
/// Success advances each returned page generation exactly once in the same
/// deterministic draft-then-target order as the raw path.
///
/// # Errors
///
/// Applies the same complete roster, semantic, device, allocation, page,
/// quiescence, and generation checks before the first mutation.
pub fn release_m1_authenticated_completed_step_kv_pages_v1(
    completed: M1AuthenticatedCompletedStepSuccessV1,
) -> Result<
    M1AuthenticatedReleasedCompletedStepV1,
    Box<M1AuthenticatedCompletedStepKvReleaseFailureV1>,
> {
    match release_m1_completed_step_kv_pages_core_v1(completed) {
        Ok(core) => Ok(M1AuthenticatedReleasedCompletedStepV1 {
            queue: core.queue,
            checked: core.checked,
            members: core.members,
            logical_accepted_counts: core.logical_accepted_counts,
            externally_published_counts: core.externally_published_counts,
            release_counts: core.release_counts,
            completed_members: core.completed_members,
            total_released: core.total_released,
        }),
        Err(failure) => {
            let M1CompletedStepKvReleaseCoreFailureV1 { error, completed } = *failure;
            Err(Box::new(M1AuthenticatedCompletedStepKvReleaseFailureV1 {
                error,
                completed,
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_and_authenticated_release_use_the_same_private_contracts() {
        fn assert_queue<Q: M1CompletedStepKvReleaseQueueV1>() {}
        fn assert_carrier<C: M1CompletedStepKvReleaseCarrierV1>() {}

        assert_queue::<M1PhysicalReadbackQueueSessionV1>();
        assert_queue::<M1AuthenticatedPhysicalReadbackQueueSessionV1>();
        assert_carrier::<M1CompletedStepSuccessV1>();
        assert_carrier::<M1AuthenticatedCompletedStepSuccessV1>();
    }

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
