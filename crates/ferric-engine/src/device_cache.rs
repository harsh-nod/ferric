//! Linear engine custody for one request's target and draft paged KV.
//!
//! This module binds the verified source-level [`ferric_spec::PhysicalKvState`]
//! transitions to non-clone engine typestates. A page lease is retained in the
//! cache while its physical identity is reachable or retired. An initialized
//! prefix advances only after a crate-owned pending write is paired with an
//! [`ExactCompletion`] for the same epoch. A bulk step reservation snapshots
//! the complete addressless page table and exact write spans without claiming
//! initialization, dispatch, or completion. Once the ordered queue has
//! completed, the crate-private step-completion bridge revalidates that entire
//! snapshot, appends its retained leases, and initializes the reserved physical
//! interval before returning the same single completion capability.
//!
//! The types below own no KFD allocation, GPU address, page contents, queue,
//! packet, signal, or hardware observation. Production page leases are minted
//! only by [`M1PartitionedModelMemoryKvPoolV1`], which consumes the exact model
//! memory owner and retains the generic owner's sole target/draft KV plane
//! partitions. There is deliberately no production constructor for
//! [`InitializedDeviceKvWrite`]: the generated runner must source it from exact
//! queue custody. Quiescent caches release retired leases only through the
//! closed completed-step join, which returns them to that exact queue-retained
//! ledger. Step completion establishes initialized physical state
//! only; it deliberately does not decide acceptance, rollback, scheduler
//! completion, or resource release policy.

use crate::bound_step_workspaces::bind_addressless_m1_full_step_workspace_subleases;
use crate::initialized_step_workspaces::allocate_initialized_m1_full_step_workspaces_v1;
use crate::{
    allocate_m1_completion_output_v1, qualification_logits::attach_m1_qualification_logits_v1,
    AddresslessM1FullStepWorkspaceComposition, BoundM1CompletionOutputV1,
    BoundM1FullStepWorkspaceSubleases, BoundModelMemoryAllocationsV1, ExactCompletion,
    InitializedM1FullStepWorkspaceAllocationFailureV1, M1CompletionOutputErrorV1,
    M1DeviceBoundModelMemoryV1, M1FullStepWorkspaceDispatchRangeError, M1FullStepWorkspaceImagesV1,
    M1FullStepWorkspacePlans, M1FullStepWorkspaceRole, M1FullStepWorkspaceSubleaseBindingFailure,
    M1FullStepWorkspaceSubleaseOwners, ModelMemoryAllocationBindingErrorV1,
    ModelMemoryDispatchRangeErrorV1,
};
use core::fmt;
use fe2o3_service_host::{
    DeviceLocalAllocationV1, DeviceStateRoleV1, ServiceAllocationErrorV1,
    ServiceAllocationSessionV1, ServiceAllocationSubleaseSetV1, ServiceDeviceDispatchRangeV1,
};
use ferric_build::{
    KvCacheComponent, M1StepWorkspaceRangeRole, ModelMemoryAllocationKind, ModelMemoryPlanError,
    QWEN3_KV_ARENA_ALIGNMENT_V1, QWEN3_KV_PAGE_BYTES_V1,
};
use ferric_spec::completion::CompletionEpoch;
use ferric_spec::{
    append_physical_page, cancel_physical_kv, commit_physical_kv, map_initialized_token,
    retire_cancelled_tail, rollback_physical_token, write_physical_token, Identity, LogicalKvState,
    M1QualificationLaneExecutionBinding, M1QualificationLaneGrouping, PhysicalKvError,
    PhysicalKvLifecycle, PhysicalKvLocation, PhysicalKvState, PhysicalPageId, Qwen3ExecutionMode,
    Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection, RequestId, Target,
    M1_KV_PAGE_TABLE_ENTRIES, M1_KV_PAGE_TOKENS, M1_KV_PHYSICAL_PAGE_SLOTS,
    M1_MAX_ACTIVE_SEQUENCES,
};
use vstd::prelude::*;

/// Exact processor declaration admitted by the M1 generated-runner template.
pub const GFX942_PROCESSOR: &str = "gfx942";
/// Exact target-feature declaration admitted by the M1 generated-runner template.
pub const GFX942_TARGET_FEATURES: &str = "+wavefrontsize64,-xnack";
/// Exact P16 target-page cardinality required by one C8192 qualification lane.
pub const M1_QUALIFICATION_TARGET_PAGE_COUNT_V1: usize = M1_KV_PAGE_TABLE_ENTRIES;

verus! {

/// Detached receipt for one checked physical M1 device admission.
///
/// This value is copyable because it owns no device resource. Production
/// construction is crate-private and derives every field from the same
/// [`fe2o3_kfd::CheckedGfx942XnackMinusDevice`] later consumed into the service
/// allocation session. Copying the receipt does not copy KFD, allocation, load,
/// queue, publication, or execution authority.
///
/// ```compile_fail
/// use ferric_engine::{bind_gfx942_device, GFX942_PROCESSOR, GFX942_TARGET_FEATURES};
/// use ferric_spec::Identity;
/// let _ = bind_gfx942_device(
///     Identity::new([1; 32]),
///     7,
///     GFX942_PROCESSOR,
///     GFX942_TARGET_FEATURES,
/// );
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gfx942DeviceBinding {
    device_id: Identity,
    node_id: u32,
    kfd_gpu_id: u32,
    gpu_unique_id: u64,
    admission_generation: u64,
    target: Target,
}

impl Gfx942DeviceBinding {
    pub closed spec fn device_id_spec(&self) -> Identity { self.device_id }

    pub closed spec fn node_id_spec(&self) -> u32 { self.node_id }

    pub closed spec fn target_spec(&self) -> Target { self.target }

    #[must_use]
    pub const fn device_id(self) -> (device_id: Identity)
        ensures device_id == self.device_id_spec(),
    {
        self.device_id
    }

    #[must_use]
    pub const fn node_id(self) -> (node_id: u32)
        ensures node_id == self.node_id_spec(),
    {
        self.node_id
    }

    #[must_use]
    pub const fn target(self) -> (target: Target)
        ensures target == self.target_spec(),
    {
        self.target
    }
}

} // verus!

impl Gfx942DeviceBinding {
    /// KFD GPU identifier observed by the checked fe2 device admission.
    #[must_use]
    pub const fn kfd_gpu_id(self) -> u32 {
        self.kfd_gpu_id
    }

    /// Stable GPU unique identifier observed in the checked topology snapshot.
    #[must_use]
    pub const fn gpu_unique_id(self) -> u64 {
        self.gpu_unique_id
    }

    /// Process-local fe2 device-admission generation committed before VM acquisition.
    #[must_use]
    pub const fn admission_generation(self) -> u64 {
        self.admission_generation
    }

    pub(crate) const fn from_physical_receipt(
        device_id: Identity,
        node_id: u32,
        kfd_gpu_id: u32,
        gpu_unique_id: u64,
        admission_generation: u64,
    ) -> Self {
        Self {
            device_id,
            node_id,
            kfd_gpu_id,
            gpu_unique_id,
            admission_generation,
            target: Target::Gfx942XnackMinus,
        }
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;

    /// Builds an inert device receipt for host-only state-machine tests.
    pub(crate) fn bind_gfx942_device(
        device_id: Identity,
        node_id: u32,
        processor: &str,
        target_features: &str,
    ) -> Result<Gfx942DeviceBinding, DeviceKvCacheError> {
        if !device_id.is_present() {
            return Err(DeviceKvCacheError::MissingDeviceIdentity);
        }
        if processor != GFX942_PROCESSOR {
            return Err(DeviceKvCacheError::ProcessorMismatch);
        }
        if target_features != GFX942_TARGET_FEATURES {
            return Err(DeviceKvCacheError::TargetFeaturesMismatch);
        }
        Ok(Gfx942DeviceBinding {
            device_id,
            node_id,
            kfd_gpu_id: node_id,
            gpu_unique_id: u64::from(node_id),
            admission_generation: 1,
            target: Target::Gfx942XnackMinus,
        })
    }
}

/// Fail-closed device-cache rejection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceKvCacheError {
    MissingDeviceIdentity,
    MissingAllocationIdentity,
    ProcessorMismatch,
    TargetFeaturesMismatch,
    WrongDevice,
    WrongRequest,
    WrongRole,
    PlanPairMismatch,
    ArenaAllocationMismatch,
    AllocationAlias,
    PendingWriteExists,
    NoPendingWrite,
    PendingWriteMismatch,
    ZeroStepActiveTokens,
    StepActiveLengthMismatch,
    StepCommittedPositionMismatch,
    StepTentativeTokensRemain,
    StepRangeOverflow,
    StepPageLeaseCountMismatch,
    StepPhysicalAlias,
    StepSelectionMismatch,
    StepPageTableDrift,
    StepWriteSpanDrift,
    WriteGenerationExhausted,
    ZeroCompletionEpoch,
    CompletionEpochMismatch,
    NoRetiredPageAtEpoch,
    UnsettledPriorRetirement,
    OwnedPageTableDrift,
    ActivePagesRemain,
    QualificationLaneCountMismatch,
    QualificationInitialWitnessMismatch,
    QualificationCacheNotFresh,
    QualificationReserveAlreadyInstalled,
    QualificationReserveMissing,
    QualificationWitnessMismatch,
    QualificationPageOrderMismatch,
    QualificationFuturePagesRemain,
    QualificationHostCustodyAllocation,
    Physical(PhysicalKvError),
}

impl From<PhysicalKvError> for DeviceKvCacheError {
    fn from(error: PhysicalKvError) -> Self {
        Self::Physical(error)
    }
}

/// Linear custody of one page subrange in a contracted role-scoped arena.
///
/// Fields and construction are crate-private. The production pool retains the
/// sole generic plane partitions and checks this page's disjoint fragments
/// across every layer before construction. Multiple page leases for one role
/// retain the same arena allocation identity, while request, local index, and
/// generation select one unique global ledger slot.
///
/// ```compile_fail
/// use ferric_engine::DeviceKvPageLease;
/// use ferric_spec::{Identity, PhysicalPageId, Qwen3ModelRole, RequestId};
/// fn forge() -> DeviceKvPageLease {
///     DeviceKvPageLease {
///         device: todo!(),
///         allocation_id: Identity::new([1; 32]),
///         request: RequestId::new(0, 1),
///         page: PhysicalPageId::new(Qwen3ModelRole::Target8B, 0, 1),
///     }
/// }
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct DeviceKvPageLease {
    device: Gfx942DeviceBinding,
    allocation_id: Identity,
    request: RequestId,
    page: PhysicalPageId,
}

impl DeviceKvPageLease {
    /// Returns the inert allocation identity without exposing an address.
    #[must_use]
    pub const fn allocation_id(&self) -> Identity {
        self.allocation_id
    }

    /// Returns the exact generational request whose global arena slot is held.
    #[must_use]
    pub const fn request(&self) -> RequestId {
        self.request
    }

    /// Returns the exact role-scoped physical page generation.
    #[must_use]
    pub const fn page(&self) -> PhysicalPageId {
        self.page
    }
}

/// Linear custody of every not-yet-reachable target P16 page for one lane.
///
/// The vector is stored in reverse physical order so `pop()` yields exact
/// request-local page indices `0..511`. Construction is private to the typed
/// all-lane prelease boundary; callers cannot add, remove, or reorder leases.
///
/// ```compile_fail
/// use ferric_engine::M1QualificationTargetPageReserveV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1QualificationTargetPageReserveV1>();
/// ```
#[must_use = "qualification future-page custody must remain attached to its exact cache"]
#[derive(Debug, PartialEq, Eq)]
pub struct M1QualificationTargetPageReserveV1 {
    device: Gfx942DeviceBinding,
    allocation_id: Identity,
    request: RequestId,
    policy_identity: Identity,
    grouping: M1QualificationLaneGrouping,
    declared_workload_digest: Identity,
    lane: M1QualificationLaneExecutionBinding,
    unused_pages: Vec<DeviceKvPageLease>,
}

impl M1QualificationTargetPageReserveV1 {
    /// Exact request generation owning this reserve.
    #[must_use]
    pub const fn request(&self) -> RequestId {
        self.request
    }

    /// Exact target-arena allocation identity shared by every retained page.
    #[must_use]
    pub const fn allocation_id(&self) -> Identity {
        self.allocation_id
    }

    /// Exact ordered qualification lane bound before queue admission.
    #[must_use]
    pub const fn lane(&self) -> M1QualificationLaneExecutionBinding {
        self.lane
    }

    /// Number of future pages that remain outside the active page table.
    #[must_use]
    pub const fn unused_page_count(&self) -> usize {
        self.unused_pages.len()
    }

    fn matches_context(&self, context: crate::M1ValidatedQualificationContextStepV1) -> bool {
        self.policy_identity.equals(&context.policy_identity())
            && self.grouping == context.grouping()
            && self
                .declared_workload_digest
                .equals(&context.declared_workload_digest())
            && self.lane == context.lane()
    }

    fn ordered_state_is_valid(&self) -> bool {
        self.unused_pages
            .iter()
            .rev()
            .enumerate()
            .all(|(offset, lease)| {
                let consumed =
                    M1_QUALIFICATION_TARGET_PAGE_COUNT_V1.saturating_sub(self.unused_pages.len());
                usize::try_from(lease.page.index()) == Ok(consumed + offset)
                    && lease.device == self.device
                    && lease.request == self.request
                    && lease.page.role() == Qwen3ModelRole::Target8B
                    && lease.allocation_id.equals(&self.allocation_id)
                    && lease.page.generation() != 0
            })
    }
}

/// Progress retained by a failed all-lane qualification page prelease.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct M1QualificationTargetPagePreleaseProgressV1 {
    /// First lane whose reserve is not yet complete.
    pub lane: u32,
    /// Next request-local P16 page index required for that lane.
    pub page: u32,
}

/// Fail-closed all-lane qualification prelease diagnostic.
#[derive(Debug)]
pub enum M1QualificationTargetPagePreleaseErrorV1 {
    /// Host custody for one bounded lane vector could not be reserved.
    HostCustodyAllocation,
    /// A cache, grouping, request roster, or initial witness was invalid.
    Cache {
        lane: u32,
        source: DeviceKvCacheError,
    },
    /// The model-memory pool rejected the exact next target page.
    Page {
        lane: u32,
        page: u32,
        source: M1DeviceKvArenaLeaseErrorV1,
    },
}

impl fmt::Display for M1QualificationTargetPagePreleaseErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "M1 qualification target-page prelease rejected: {self:?}"
        )
    }
}

impl std::error::Error for M1QualificationTargetPagePreleaseErrorV1 {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Page { source, .. } => Some(source),
            Self::HostCustodyAllocation | Self::Cache { .. } => None,
        }
    }
}

const M1_TARGET_KV_PLANE_SUBLEASES_V1: usize = 72;
const M1_DRAFT_KV_PLANE_SUBLEASES_V1: usize = 56;
const M1_GLOBAL_KV_PAGE_SLOTS_V1: usize =
    M1_MAX_ACTIVE_SEQUENCES as usize * M1_KV_PHYSICAL_PAGE_SLOTS;
pub(crate) const M1_KV_PAGE_RETURN_ROLE_ORDER_V1: [Qwen3ModelRole; 2] =
    [Qwen3ModelRole::Draft06B, Qwen3ModelRole::Target8B];

type TargetKvPlaneSubleasesV1 = ServiceAllocationSubleaseSetV1<
    DeviceStateRoleV1,
    DeviceLocalAllocationV1,
    M1_TARGET_KV_PLANE_SUBLEASES_V1,
>;
type DraftKvPlaneSubleasesV1 = ServiceAllocationSubleaseSetV1<
    DeviceStateRoleV1,
    DeviceLocalAllocationV1,
    M1_DRAFT_KV_PLANE_SUBLEASES_V1,
>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum M1KvPoolPageStateV1 {
    Free { generation: u32 },
    Leased { request: RequestId, generation: u32 },
}

impl M1KvPoolPageStateV1 {
    const INITIAL: Self = Self::Free { generation: 1 };
}

/// Fail-closed model-memory KV partition or page-lease error.
#[derive(Debug)]
pub enum M1DeviceKvArenaLeaseErrorV1 {
    /// The consumed model-memory owner no longer matches its exact plan.
    ModelMemory(ModelMemoryAllocationBindingErrorV1),
    /// One canonical KV plane or request page could not be resolved.
    ModelPlan(ModelMemoryPlanError),
    /// A role's complete plane partition drifted from canonical geometry.
    PlaneGeometry { role: Qwen3ModelRole },
    /// Host reservation for the bounded generation ledger failed.
    HostLedgerAllocation { role: Qwen3ModelRole },
    /// The generic allocation owner rejected a key, partition, or subrange.
    Allocation {
        role: Qwen3ModelRole,
        source: ServiceAllocationErrorV1,
    },
    /// A zero or out-of-range generational request was supplied.
    RequestOutOfRange,
    /// The request-local physical page index exceeds the exact table bound.
    PageOutOfRange,
    /// The exact role/request/page slot is already leased.
    PageAlreadyLeased,
    /// The retained generation ledger disagreed with the resolved page.
    GenerationLedgerDrift,
    /// A retained reservation names a different role-scoped allocation.
    AllocationIdentityMismatch,
    /// A retained reservation names a page not leased to its exact request.
    PageLeaseMismatch,
    /// Retry was requested after a generic partition had already succeeded.
    RetryDenied,
}

impl fmt::Display for M1DeviceKvArenaLeaseErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "M1 device KV arena lease rejected: {self:?}")
    }
}

impl std::error::Error for M1DeviceKvArenaLeaseErrorV1 {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ModelMemory(source) => Some(source),
            Self::ModelPlan(source) => Some(source),
            Self::Allocation { source, .. } => Some(source),
            Self::PlaneGeometry { .. }
            | Self::HostLedgerAllocation { .. }
            | Self::RequestOutOfRange
            | Self::PageOutOfRange
            | Self::PageAlreadyLeased
            | Self::GenerationLedgerDrift
            | Self::AllocationIdentityMismatch
            | Self::PageLeaseMismatch
            | Self::RetryDenied => None,
        }
    }
}

/// Opaque recovery after a model-memory KV partition attempt.
///
/// `TargetPartitioned` deliberately exposes no model-memory or sublease
/// extraction API. A failure after the first generic partition cannot honestly
/// roll that allocation back; retaining this value keeps the sole public
/// partition witness paired with the consumed model owner for quarantine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum M1DeviceKvArenaLeaseRecoveryPhaseV1 {
    /// Neither role arena was partitioned.
    Unpartitioned,
    /// Target was partitioned before draft reservation failed.
    TargetPartitioned,
}

/// Opaque quarantine custody after only the target arena was partitioned.
///
/// ```compile_fail
/// use ferric_engine::M1TargetPartitionedKvQuarantineV1;
/// fn bypass(quarantine: M1TargetPartitionedKvQuarantineV1) {
///     let _model = quarantine.model_memory;
///     let _allocations = quarantine.allocations;
/// }
/// ```
#[must_use = "partial generic partition custody must remain quarantined"]
#[derive(Debug)]
pub struct M1TargetPartitionedKvQuarantineV1 {
    device: Gfx942DeviceBinding,
    model_memory: BoundModelMemoryAllocationsV1,
    allocations: ServiceAllocationSessionV1,
    target_planes: TargetKvPlaneSubleasesV1,
}

impl M1TargetPartitionedKvQuarantineV1 {
    /// Returns the checked physical-device receipt retained in quarantine.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.device
    }

    /// Returns the exact inert target-arena identity retained in quarantine.
    #[must_use]
    pub const fn target_allocation_id(&self) -> Identity {
        self.model_memory.selected_allocation_identity(
            Qwen3ModelRole::Target8B,
            ModelMemoryAllocationKind::KvArena,
        )
    }

    /// Returns the fixed target-plane witness cardinality.
    #[must_use]
    pub const fn target_plane_count(&self) -> usize {
        self.target_planes.len()
    }

    /// Returns the redacted number of allocations retained by the session.
    #[must_use]
    pub fn retained_allocation_count(&self) -> usize {
        self.allocations.allocation_count()
    }
}

#[must_use = "failed model-memory and any completed partition remain retained"]
#[derive(Debug)]
pub enum M1DeviceKvArenaLeaseRecoveryV1 {
    /// Neither role arena was partitioned; the opaque owner can retry directly.
    Unpartitioned(Box<M1UnpartitionedModelMemoryKvRecoveryV1>),
    /// Target was partitioned; all custody is opaque and fail-closed.
    TargetPartitioned(Box<M1TargetPartitionedKvQuarantineV1>),
}

impl M1DeviceKvArenaLeaseRecoveryV1 {
    /// Returns the checked physical-device receipt retained in either recovery phase.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        match self {
            Self::Unpartitioned(recovery) => recovery.device(),
            Self::TargetPartitioned(quarantine) => quarantine.device(),
        }
    }

    /// Returns the redacted retained partition phase without exposing owners.
    #[must_use]
    pub const fn phase(&self) -> M1DeviceKvArenaLeaseRecoveryPhaseV1 {
        match self {
            Self::Unpartitioned(_) => M1DeviceKvArenaLeaseRecoveryPhaseV1::Unpartitioned,
            Self::TargetPartitioned(_) => M1DeviceKvArenaLeaseRecoveryPhaseV1::TargetPartitioned,
        }
    }

    /// Retries only an unchanged pre-partition owner without exposing either raw input.
    ///
    /// A target-partitioned recovery remains quarantined inside the returned
    /// failure and never regains legacy session or model-memory access.
    ///
    /// # Errors
    ///
    /// Returns the exact ordinary binding failure on an unchanged retry, or
    /// [`M1DeviceKvArenaLeaseErrorV1::RetryDenied`] with unchanged quarantine
    /// after any generic partition has already succeeded.
    pub fn retry(
        self,
    ) -> Result<M1PartitionedModelMemoryKvPoolV1, M1DeviceKvArenaLeaseBindingFailureV1> {
        match self {
            Self::Unpartitioned(recovery) => {
                let M1UnpartitionedModelMemoryKvRecoveryV1 { initialized } = *recovery;
                bind_m1_partitioned_model_memory_kv_pool_v1(initialized)
            }
            recovery @ Self::TargetPartitioned(_) => Err(M1DeviceKvArenaLeaseBindingFailureV1 {
                error: M1DeviceKvArenaLeaseErrorV1::RetryDenied,
                recovery: Box::new(recovery),
            }),
        }
    }
}

/// Opaque unchanged inputs retained before the first generic partition.
///
/// The only consuming operation is [`M1DeviceKvArenaLeaseRecoveryV1::retry`].
/// Raw model-memory and service-session owners cannot be extracted.
///
/// ```compile_fail
/// use ferric_engine::M1UnpartitionedModelMemoryKvRecoveryV1;
/// fn escape(recovery: M1UnpartitionedModelMemoryKvRecoveryV1) {
///     let _ = recovery.initialized;
/// }
/// ```
#[must_use = "unchanged partition inputs must be retried or retained"]
#[derive(Debug)]
pub struct M1UnpartitionedModelMemoryKvRecoveryV1 {
    initialized: M1DeviceBoundModelMemoryV1,
}

impl M1UnpartitionedModelMemoryKvRecoveryV1 {
    /// Returns the checked physical-device receipt retained before partitioning.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.initialized.device()
    }
}

/// Transactional partition rejection retaining every still-live owner.
#[must_use = "the exact recovery custody must be consumed or quarantined"]
#[derive(Debug)]
pub struct M1DeviceKvArenaLeaseBindingFailureV1 {
    error: M1DeviceKvArenaLeaseErrorV1,
    recovery: Box<M1DeviceKvArenaLeaseRecoveryV1>,
}

impl M1DeviceKvArenaLeaseBindingFailureV1 {
    /// Returns the checked physical-device receipt retained by the failure.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.recovery.device()
    }

    /// Returns the fail-closed partition diagnostic.
    #[must_use]
    pub const fn error(&self) -> &M1DeviceKvArenaLeaseErrorV1 {
        &self.error
    }

    /// Recovers the exact diagnostic and retained ownership state.
    #[must_use = "the exact recovery custody remains retained"]
    pub fn into_parts(self) -> (M1DeviceKvArenaLeaseErrorV1, M1DeviceKvArenaLeaseRecoveryV1) {
        (self.error, *self.recovery)
    }
}

/// Closed production owner for partitioned model memory and KV page leases.
///
/// This value consumes the non-clone model-memory owner and retains the sole
/// generic target/draft KV plane partitions. It intentionally provides no
/// model-memory borrow or extraction method: legacy unpartitioned KV ranges
/// are invalid after this transition. The physical fixed-batch builder consumes
/// this exact owner and carries its receipt through queue custody.
///
/// Page leases are minted only after every layer's exact key/value page
/// subrange is revalidated against the live generic allocation session. The
/// internal ledger makes each global role/request/page slot one-shot until a
/// future exact quiescent-release bridge advances its generation.
///
/// Ferric cannot instantiate a host-only fake [`ServiceAllocationSessionV1`]:
/// its public constructors require a real checked KFD session and the pinned
/// dependency exposes no test-support backend. Duplicate-partition rejection
/// and denial of legacy ranges after partition are therefore exercised by the
/// pinned `fe2o3-service-host` tests
/// `sublease_registration_is_atomic_and_duplicate_consumption_is_rejected`
/// and
/// `partitioned_queue_admission_accepts_member_subranges_and_rejects_escape`.
/// Ferric's compile-fail tests below additionally prove that safe callers
/// cannot retain the consumed session/model owner or construct a second pool.
///
/// ```compile_fail
/// use ferric_engine::M1PartitionedModelMemoryKvPoolV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1PartitionedModelMemoryKvPoolV1>();
/// ```
///
/// ```compile_fail
/// use ferric_engine::M1PartitionedModelMemoryKvPoolV1;
/// fn bypass_legacy_ranges(pool: &M1PartitionedModelMemoryKvPoolV1) {
///     let _ = &pool.model_memory;
///     let _ = &pool.allocations;
/// }
/// ```
#[must_use = "partition and page-generation custody must remain retained"]
#[derive(Debug)]
pub struct M1PartitionedModelMemoryKvPoolV1 {
    device: Gfx942DeviceBinding,
    model_memory: BoundModelMemoryAllocationsV1,
    allocations: ServiceAllocationSessionV1,
    target_planes: TargetKvPlaneSubleasesV1,
    draft_planes: DraftKvPlaneSubleasesV1,
    target_pages: Box<[M1KvPoolPageStateV1]>,
    draft_pages: Box<[M1KvPoolPageStateV1]>,
}

/// Opaque Ferric custody retained after generic queue ownership is created.
///
/// The service allocation session moves into the generic queue ledger while
/// this owner retains the only model-memory, partition, and page-generation
/// witnesses. It deliberately exposes no allocation, range, lease, or raw
/// model-memory API.
///
/// ```compile_fail
/// use ferric_engine::M1PartitionedModelMemoryKvQueueCustodyV1;
/// fn escape(custody: M1PartitionedModelMemoryKvQueueCustodyV1) {
///     let _ = custody.model_memory;
///     let _ = custody.target_planes;
/// }
/// ```
#[must_use = "partition and page-generation custody must remain paired with the queue"]
#[derive(Debug)]
pub struct M1PartitionedModelMemoryKvQueueCustodyV1 {
    device: Gfx942DeviceBinding,
    model_memory: BoundModelMemoryAllocationsV1,
    target_planes: TargetKvPlaneSubleasesV1,
    draft_planes: DraftKvPlaneSubleasesV1,
    target_pages: Box<[M1KvPoolPageStateV1]>,
    draft_pages: Box<[M1KvPoolPageStateV1]>,
}

impl M1PartitionedModelMemoryKvQueueCustodyV1 {
    /// Returns the exact inert device declaration retained through queue phases.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.device
    }

    /// Returns one exact role-scoped KV allocation identity without exposing its owner.
    #[must_use]
    pub const fn allocation_id(&self, role: Qwen3ModelRole) -> Identity {
        self.model_memory
            .selected_allocation_identity(role, ModelMemoryAllocationKind::KvArena)
    }

    /// Returns the fixed number of retained partition members for one role.
    #[must_use]
    pub const fn plane_count(&self, role: Qwen3ModelRole) -> usize {
        match role {
            Qwen3ModelRole::Target8B => self.target_planes.len(),
            Qwen3ModelRole::Draft06B => self.draft_planes.len(),
        }
    }

    /// Returns the fixed ledger cardinality without exposing ledger contents.
    #[must_use]
    pub fn page_slot_count(&self, role: Qwen3ModelRole) -> usize {
        match role {
            Qwen3ModelRole::Target8B => self.target_pages.len(),
            Qwen3ModelRole::Draft06B => self.draft_pages.len(),
        }
    }

    pub(crate) fn revalidate_page_return_authority(
        &self,
    ) -> Result<(), M1DeviceKvArenaLeaseErrorV1> {
        self.model_memory
            .revalidate_for_kv_partition()
            .map_err(M1DeviceKvArenaLeaseErrorV1::ModelMemory)?;
        if self.target_planes.len() != M1_TARGET_KV_PLANE_SUBLEASES_V1
            || self.draft_planes.len() != M1_DRAFT_KV_PLANE_SUBLEASES_V1
            || self.target_pages.len() != M1_GLOBAL_KV_PAGE_SLOTS_V1
            || self.draft_pages.len() != M1_GLOBAL_KV_PAGE_SLOTS_V1
        {
            return Err(M1DeviceKvArenaLeaseErrorV1::GenerationLedgerDrift);
        }
        Ok(())
    }

    pub(crate) fn preflight_page_return(
        &self,
        expected_role: Qwen3ModelRole,
        lease: &DeviceKvPageLease,
    ) -> Result<M1PreflightedKvPageReturnV1, M1KvPageReturnErrorV1> {
        let global_index = global_page_index(lease.request, lease.page.index())
            .map_err(|_| M1KvPageReturnErrorV1::Index)?;
        preflight_page_return_identity(
            self.device,
            self.allocation_id(expected_role),
            expected_role,
            self.page_ledger(expected_role).get(global_index).copied(),
            global_index,
            lease.request,
            lease,
        )
    }

    pub(crate) fn commit_page_return(
        &mut self,
        preflighted: M1PreflightedKvPageReturnV1,
        lease: DeviceKvPageLease,
    ) {
        let state = &mut self.page_ledger_mut(preflighted.role)[preflighted.global_index];
        commit_page_return_state(state, preflighted, lease);
    }

    fn page_ledger(&self, role: Qwen3ModelRole) -> &[M1KvPoolPageStateV1] {
        match role {
            Qwen3ModelRole::Target8B => &self.target_pages,
            Qwen3ModelRole::Draft06B => &self.draft_pages,
        }
    }

    fn page_ledger_mut(&mut self, role: Qwen3ModelRole) -> &mut [M1KvPoolPageStateV1] {
        match role {
            Qwen3ModelRole::Target8B => &mut self.target_pages,
            Qwen3ModelRole::Draft06B => &mut self.draft_pages,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum M1KvPageReturnErrorV1 {
    Device,
    Allocation,
    Request,
    Role,
    Index,
    Ledger,
    GenerationExhausted,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct M1PreflightedKvPageReturnV1 {
    pub(crate) role: Qwen3ModelRole,
    pub(crate) request: RequestId,
    pub(crate) page: PhysicalPageId,
    pub(crate) allocation_id: Identity,
    pub(crate) global_index: usize,
    next_generation: u32,
}

fn preflight_page_return_identity(
    expected_device: Gfx942DeviceBinding,
    expected_allocation_id: Identity,
    expected_role: Qwen3ModelRole,
    state: Option<M1KvPoolPageStateV1>,
    global_index: usize,
    expected_request: RequestId,
    lease: &DeviceKvPageLease,
) -> Result<M1PreflightedKvPageReturnV1, M1KvPageReturnErrorV1> {
    if lease.device != expected_device {
        return Err(M1KvPageReturnErrorV1::Device);
    }
    if lease.request != expected_request {
        return Err(M1KvPageReturnErrorV1::Request);
    }
    if lease.page.role() != expected_role {
        return Err(M1KvPageReturnErrorV1::Role);
    }
    if !lease.allocation_id.equals(&expected_allocation_id) {
        return Err(M1KvPageReturnErrorV1::Allocation);
    }
    validate_leased_page_state(state, lease.request, lease.page.generation())
        .map_err(|_| M1KvPageReturnErrorV1::Ledger)?;
    let next_generation = lease
        .page
        .generation()
        .checked_add(1)
        .ok_or(M1KvPageReturnErrorV1::GenerationExhausted)?;
    Ok(M1PreflightedKvPageReturnV1 {
        role: expected_role,
        request: lease.request,
        page: lease.page,
        allocation_id: lease.allocation_id,
        global_index,
        next_generation,
    })
}

const fn returned_page_state(preflighted: &M1PreflightedKvPageReturnV1) -> M1KvPoolPageStateV1 {
    M1KvPoolPageStateV1::Free {
        generation: preflighted.next_generation,
    }
}

fn commit_page_return_state(
    state: &mut M1KvPoolPageStateV1,
    preflighted: M1PreflightedKvPageReturnV1,
    _lease: DeviceKvPageLease,
) {
    *state = returned_page_state(&preflighted);
}

impl M1PartitionedModelMemoryKvPoolV1 {
    /// Returns the exact device declaration retained by the pool.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.device
    }

    /// Returns the exact inert arena identity retained for one model role.
    #[must_use]
    pub const fn allocation_id(&self, role: Qwen3ModelRole) -> Identity {
        self.model_memory
            .selected_allocation_identity(role, ModelMemoryAllocationKind::KvArena)
    }

    /// Revalidates one retained reservation page against the private lease ledger.
    ///
    /// This checks exact request slot/generation, role-scoped allocation
    /// identity, request-local index, nonzero physical generation, ledger
    /// occupancy, and every K/V page fragment before reporting success.
    ///
    /// # Errors
    ///
    /// Rejects any request, allocation, page, generation, ledger, model-plan,
    /// partition, mapping, or fragment-geometry drift.
    pub fn validate_page_identity(
        &self,
        request: RequestId,
        allocation_id: Identity,
        page: PhysicalPageId,
    ) -> Result<(), M1DeviceKvArenaLeaseErrorV1> {
        let global_index = global_page_index(request, page.index())?;
        if allocation_id != self.allocation_id(page.role()) {
            return Err(M1DeviceKvArenaLeaseErrorV1::AllocationIdentityMismatch);
        }
        validate_leased_page_state(
            self.page_ledger(page.role()).get(global_index).copied(),
            request,
            page.generation(),
        )?;
        self.model_memory
            .revalidate_for_kv_partition()
            .map_err(M1DeviceKvArenaLeaseErrorV1::ModelMemory)?;
        self.validate_page_fragments(request, page, global_index)
    }

    /// Resolves one exact model-weight range through retained allocation custody.
    ///
    /// # Errors
    ///
    /// Rejects model-plan, key, owner-generation, mapping, or range drift.
    pub fn weight_dispatch_range(
        &self,
        role: Qwen3ModelRole,
        kind: ferric_spec::Qwen3TensorKind,
        layer: u32,
    ) -> Result<ServiceDeviceDispatchRangeV1, ModelMemoryDispatchRangeErrorV1> {
        self.model_memory
            .weight_dispatch_range(&self.allocations, role, kind, layer)
    }

    /// Allocates one exact coherent completion output inside this closed owner.
    ///
    /// # Errors
    ///
    /// Rejects selection, extent, allocation, mapping, or range drift while
    /// retaining the allocation session inside the pool.
    pub fn allocate_completion_output(
        &mut self,
        selection: Qwen3PlanSelection,
    ) -> Result<BoundM1CompletionOutputV1, M1CompletionOutputErrorV1> {
        allocate_m1_completion_output_v1(&mut self.allocations, selection)
    }

    /// Adds qualification-only host-visible logits capture to a compact output.
    ///
    /// Production callers retain the ordinary compact-only allocation path.
    /// Qualification must opt in before physical buffer binding.
    ///
    /// # Errors
    ///
    /// Rejects shape, allocation, mapping, or range drift while retaining the
    /// already-bound compact output inside the boxed failure.
    pub fn enable_qualification_logits_capture(
        &mut self,
        completion: BoundM1CompletionOutputV1,
    ) -> Result<BoundM1CompletionOutputV1, Box<crate::M1QualificationLogitsAllocationFailureV1>>
    {
        attach_m1_qualification_logits_v1(&mut self.allocations, completion)
    }

    pub(crate) fn completion_output_dispatch_range(
        &self,
        completion: &BoundM1CompletionOutputV1,
        selection: Qwen3PlanSelection,
    ) -> Result<fe2o3_service_host::ServiceHostDispatchRangeV1, M1CompletionOutputErrorV1> {
        completion.host_dispatch_range(&self.allocations, selection)
    }

    pub(crate) fn qualification_logits_dispatch_range(
        &self,
        logits: &crate::BoundM1QualificationLogitsV1,
        selection: Qwen3PlanSelection,
    ) -> Result<fe2o3_service_host::ServiceHostDispatchRangeV1, crate::M1QualificationLogitsErrorV1>
    {
        logits.host_dispatch_range(&self.allocations, selection)
    }

    pub(crate) fn allocate_full_step_workspaces(
        &mut self,
        plans: M1FullStepWorkspacePlans,
        images: M1FullStepWorkspaceImagesV1,
    ) -> Result<M1FullStepWorkspaceSubleaseOwners, InitializedM1FullStepWorkspaceAllocationFailureV1>
    {
        allocate_initialized_m1_full_step_workspaces_v1(&mut self.allocations, plans, images)
    }

    pub(crate) fn bind_full_step_workspaces(
        &self,
        composition: AddresslessM1FullStepWorkspaceComposition,
        owners: M1FullStepWorkspaceSubleaseOwners,
    ) -> Result<BoundM1FullStepWorkspaceSubleases, M1FullStepWorkspaceSubleaseBindingFailure> {
        bind_addressless_m1_full_step_workspace_subleases(composition, owners, &self.allocations)
    }

    pub(crate) fn workspace_segment_dispatch_range(
        &self,
        workspaces: &BoundM1FullStepWorkspaceSubleases,
        segment_index: u8,
        workspace: M1FullStepWorkspaceRole,
        role: M1StepWorkspaceRangeRole,
    ) -> Result<ServiceDeviceDispatchRangeV1, M1FullStepWorkspaceDispatchRangeError> {
        workspaces.segment_dispatch_range(&self.allocations, segment_index, workspace, role)
    }

    pub(crate) fn speculative_token_assembly_anchor_dispatch_range(
        &self,
        workspaces: &BoundM1FullStepWorkspaceSubleases,
        verification_segment: u8,
    ) -> Result<ServiceDeviceDispatchRangeV1, M1FullStepWorkspaceDispatchRangeError> {
        workspaces.speculative_token_assembly_anchor_dispatch_range(
            &self.allocations,
            verification_segment,
        )
    }

    pub(crate) fn speculative_draft_choice_dispatch_range(
        &self,
        workspaces: &BoundM1FullStepWorkspaceSubleases,
        producer_segment: u8,
    ) -> Result<ServiceDeviceDispatchRangeV1, M1FullStepWorkspaceDispatchRangeError> {
        workspaces.speculative_draft_choice_dispatch_range(&self.allocations, producer_segment)
    }

    pub(crate) fn speculative_draft_position_dispatch_range(
        &self,
        workspaces: &BoundM1FullStepWorkspaceSubleases,
        draft_segment: u8,
    ) -> Result<ServiceDeviceDispatchRangeV1, M1FullStepWorkspaceDispatchRangeError> {
        workspaces.speculative_draft_position_dispatch_range(&self.allocations, draft_segment)
    }

    pub(crate) fn speculative_draft_context_dispatch_range(
        &self,
        workspaces: &BoundM1FullStepWorkspaceSubleases,
        draft_segment: u8,
    ) -> Result<ServiceDeviceDispatchRangeV1, M1FullStepWorkspaceDispatchRangeError> {
        workspaces.speculative_draft_context_dispatch_range(&self.allocations, draft_segment)
    }

    pub(crate) fn into_queue_creation_parts(
        self,
    ) -> (
        ServiceAllocationSessionV1,
        M1PartitionedModelMemoryKvQueueCustodyV1,
    ) {
        (
            self.allocations,
            M1PartitionedModelMemoryKvQueueCustodyV1 {
                device: self.device,
                model_memory: self.model_memory,
                target_planes: self.target_planes,
                draft_planes: self.draft_planes,
                target_pages: self.target_pages,
                draft_pages: self.draft_pages,
            },
        )
    }

    pub(crate) fn from_rejected_queue_creation(
        allocations: ServiceAllocationSessionV1,
        custody: M1PartitionedModelMemoryKvQueueCustodyV1,
    ) -> Self {
        Self {
            device: custody.device,
            model_memory: custody.model_memory,
            allocations,
            target_planes: custody.target_planes,
            draft_planes: custody.draft_planes,
            target_pages: custody.target_pages,
            draft_pages: custody.draft_pages,
        }
    }

    /// Resolves one complete KV plane through its unique generic sublease.
    ///
    /// This is the only partition-compatible replacement for the legacy
    /// model-memory KV range resolver. It exposes no model-memory owner or
    /// native address.
    ///
    /// # Errors
    ///
    /// Rejects an out-of-range layer, retained model drift, or generic owner,
    /// allocation-generation, partition-member, mapping, or range drift.
    pub fn kv_dispatch_range(
        &self,
        role: Qwen3ModelRole,
        component: KvCacheComponent,
        layer: u32,
    ) -> Result<ServiceDeviceDispatchRangeV1, M1DeviceKvArenaLeaseErrorV1> {
        self.model_memory
            .revalidate_for_kv_partition()
            .map_err(M1DeviceKvArenaLeaseErrorV1::ModelMemory)?;
        let declared = self
            .model_memory
            .plan()
            .kv_layer(role, component, layer)
            .map_err(M1DeviceKvArenaLeaseErrorV1::ModelPlan)?;
        if declared.allocation_id() != self.allocation_id(role) {
            return Err(M1DeviceKvArenaLeaseErrorV1::PlaneGeometry { role });
        }
        let member_index = plane_member_index(role, component, layer)?;
        let range = match role {
            Qwen3ModelRole::Target8B => self.allocations.sublease_range(
                &self.target_planes,
                member_index,
                0,
                declared.byte_len(),
                QWEN3_KV_ARENA_ALIGNMENT_V1,
            ),
            Qwen3ModelRole::Draft06B => self.allocations.sublease_range(
                &self.draft_planes,
                member_index,
                0,
                declared.byte_len(),
                QWEN3_KV_ARENA_ALIGNMENT_V1,
            ),
        }
        .map_err(|source| M1DeviceKvArenaLeaseErrorV1::Allocation { role, source })?;
        let dispatch = self
            .allocations
            .device_dispatch_range(range)
            .map_err(|source| M1DeviceKvArenaLeaseErrorV1::Allocation { role, source })?;
        if dispatch.offset_bytes() != declared.offset()
            || dispatch.extent_bytes() != declared.byte_len()
        {
            return Err(M1DeviceKvArenaLeaseErrorV1::PlaneGeometry { role });
        }
        Ok(dispatch)
    }

    /// Mints one unique request-local page lease from exact partition custody.
    ///
    /// The generation is sourced only from the pool's private ledger. All
    /// key/value fragments across every role layer are checked before the slot
    /// changes from free to leased, so every rejection is transactional.
    ///
    /// # Errors
    ///
    /// Rejects stale requests, out-of-range or duplicate pages, model/geometry
    /// drift, and every generic owner or sublease mismatch without changing the
    /// page-generation ledger.
    pub fn lease_page(
        &mut self,
        request: RequestId,
        role: Qwen3ModelRole,
        physical_index: u32,
    ) -> Result<DeviceKvPageLease, M1DeviceKvArenaLeaseErrorV1> {
        let global_index = global_page_index(request, physical_index)?;
        let state = *self
            .page_ledger(role)
            .get(global_index)
            .ok_or(M1DeviceKvArenaLeaseErrorV1::PageOutOfRange)?;
        let generation = free_page_generation(state)?;

        self.model_memory
            .revalidate_for_kv_partition()
            .map_err(M1DeviceKvArenaLeaseErrorV1::ModelMemory)?;
        let page = PhysicalPageId::new(role, physical_index, generation);
        self.validate_page_fragments(request, page, global_index)?;

        self.page_ledger_mut(role)[global_index] = M1KvPoolPageStateV1::Leased {
            request,
            generation,
        };
        Ok(DeviceKvPageLease {
            device: self.device,
            allocation_id: self.allocation_id(role),
            request,
            page,
        })
    }

    fn page_ledger(&self, role: Qwen3ModelRole) -> &[M1KvPoolPageStateV1] {
        match role {
            Qwen3ModelRole::Target8B => &self.target_pages,
            Qwen3ModelRole::Draft06B => &self.draft_pages,
        }
    }

    fn page_ledger_mut(&mut self, role: Qwen3ModelRole) -> &mut [M1KvPoolPageStateV1] {
        match role {
            Qwen3ModelRole::Target8B => &mut self.target_pages,
            Qwen3ModelRole::Draft06B => &mut self.draft_pages,
        }
    }

    fn validate_page_fragments(
        &self,
        request: RequestId,
        page: PhysicalPageId,
        expected_global_index: usize,
    ) -> Result<(), M1DeviceKvArenaLeaseErrorV1> {
        let role = page.role();
        for layer in 0..role.layers() {
            for component in [KvCacheComponent::Key, KvCacheComponent::Value] {
                let binding = self
                    .model_memory
                    .plan()
                    .kv_request_page(request, page, component, layer)
                    .map_err(M1DeviceKvArenaLeaseErrorV1::ModelPlan)?;
                if usize::try_from(binding.global_page()) != Ok(expected_global_index)
                    || binding.range().allocation_id() != self.allocation_id(role)
                    || binding.range().byte_len() != QWEN3_KV_PAGE_BYTES_V1
                {
                    return Err(M1DeviceKvArenaLeaseErrorV1::PlaneGeometry { role });
                }
                let member_index = plane_member_index(role, component, layer)?;
                let relative_offset = u64::from(binding.global_page())
                    .checked_mul(QWEN3_KV_PAGE_BYTES_V1)
                    .ok_or(M1DeviceKvArenaLeaseErrorV1::PlaneGeometry { role })?;
                let range = match role {
                    Qwen3ModelRole::Target8B => self.allocations.sublease_range(
                        &self.target_planes,
                        member_index,
                        relative_offset,
                        QWEN3_KV_PAGE_BYTES_V1,
                        QWEN3_KV_ARENA_ALIGNMENT_V1,
                    ),
                    Qwen3ModelRole::Draft06B => self.allocations.sublease_range(
                        &self.draft_planes,
                        member_index,
                        relative_offset,
                        QWEN3_KV_PAGE_BYTES_V1,
                        QWEN3_KV_ARENA_ALIGNMENT_V1,
                    ),
                }
                .map_err(|source| M1DeviceKvArenaLeaseErrorV1::Allocation { role, source })?;
                let dispatch = self
                    .allocations
                    .device_dispatch_range(range)
                    .map_err(|source| M1DeviceKvArenaLeaseErrorV1::Allocation { role, source })?;
                if dispatch.offset_bytes() != binding.range().offset()
                    || dispatch.extent_bytes() != QWEN3_KV_PAGE_BYTES_V1
                {
                    return Err(M1DeviceKvArenaLeaseErrorV1::PlaneGeometry { role });
                }
            }
        }
        Ok(())
    }
}

/// Successful all-lane qualification page prelease.
///
/// The model-memory pool remains paired with every cache. Each cache now owns
/// one complete target future-page reserve and can proceed to queue admission.
#[must_use = "preleased model memory and cache custody must proceed together"]
#[derive(Debug)]
pub struct M1QualificationTargetPagePreleaseSuccessV1 {
    pool: M1PartitionedModelMemoryKvPoolV1,
    caches: Vec<ActiveDeviceKvCache>,
}

impl M1QualificationTargetPagePreleaseSuccessV1 {
    /// Recovers the exact pool and ordered lane caches after successful prelease.
    #[must_use = "the exact pool and all ordered caches remain linear"]
    pub fn into_parts(self) -> (M1PartitionedModelMemoryKvPoolV1, Vec<ActiveDeviceKvCache>) {
        (self.pool, self.caches)
    }
}

/// Retry owner for an incomplete all-lane qualification target-page prelease.
///
/// It retains model memory, all ordered caches, validated initial witnesses,
/// and every target page minted before a failure. No extraction path can split
/// a partial prefix from its pool ledger; the only consuming operation retries
/// from the exact next lane/page coordinate.
///
/// ```compile_fail
/// use ferric_engine::M1QualificationTargetPagePreleaseRecoveryV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1QualificationTargetPagePreleaseRecoveryV1>();
/// ```
#[must_use = "partial target-page prelease custody must be retried or retained"]
#[derive(Debug)]
pub struct M1QualificationTargetPagePreleaseRecoveryV1 {
    pool: M1PartitionedModelMemoryKvPoolV1,
    caches: Vec<ActiveDeviceKvCache>,
    initial_contexts: Vec<crate::M1ValidatedQualificationContextStepV1>,
    grouping: M1QualificationLaneGrouping,
    pages_by_lane: Vec<Vec<DeviceKvPageLease>>,
}

impl M1QualificationTargetPagePreleaseRecoveryV1 {
    /// Exact first incomplete lane/page coordinate retained for retry.
    #[must_use]
    pub fn progress(&self) -> M1QualificationTargetPagePreleaseProgressV1 {
        qualification_target_page_prelease_progress(&self.pages_by_lane)
    }

    /// Retries from the exact retained lane/page coordinate.
    ///
    /// # Errors
    ///
    /// Returns all owners unchanged except for any additional successfully
    /// minted prefix pages, which remain inside the returned retry owner.
    pub fn retry(
        mut self,
    ) -> Result<
        M1QualificationTargetPagePreleaseSuccessV1,
        Box<M1QualificationTargetPagePreleaseFailureV1>,
    > {
        if let Err((lane, source)) = validate_qualification_prelease_inputs(
            &self.pool,
            &self.caches,
            &self.initial_contexts,
            self.grouping,
        ) {
            return Err(Box::new(M1QualificationTargetPagePreleaseFailureV1 {
                error: M1QualificationTargetPagePreleaseErrorV1::Cache { lane, source },
                recovery: self,
            }));
        }

        if self.pages_by_lane.len() < self.caches.len()
            && self
                .pages_by_lane
                .try_reserve_exact(self.caches.len() - self.pages_by_lane.len())
                .is_err()
        {
            return Err(Box::new(M1QualificationTargetPagePreleaseFailureV1 {
                error: M1QualificationTargetPagePreleaseErrorV1::HostCustodyAllocation,
                recovery: self,
            }));
        }
        while self.pages_by_lane.len() < self.caches.len() {
            let mut pages = Vec::new();
            if pages
                .try_reserve_exact(M1_QUALIFICATION_TARGET_PAGE_COUNT_V1)
                .is_err()
            {
                return Err(Box::new(M1QualificationTargetPagePreleaseFailureV1 {
                    error: M1QualificationTargetPagePreleaseErrorV1::HostCustodyAllocation,
                    recovery: self,
                }));
            }
            self.pages_by_lane.push(pages);
        }

        let acquisition = acquire_qualification_target_page_prefix(
            &self.caches,
            &mut self.pages_by_lane,
            |request, page| {
                self.pool
                    .lease_page(request, Qwen3ModelRole::Target8B, page)
            },
        );
        if let Err((lane, page, source)) = acquisition {
            return Err(Box::new(M1QualificationTargetPagePreleaseFailureV1 {
                error: M1QualificationTargetPagePreleaseErrorV1::Page { lane, page, source },
                recovery: self,
            }));
        }

        let target_allocation_id = self.pool.allocation_id(Qwen3ModelRole::Target8B);
        for (lane, cache) in self.caches.iter_mut().enumerate() {
            let mut pages = core::mem::take(&mut self.pages_by_lane[lane]);
            pages.reverse();
            let context = self.initial_contexts[lane];
            cache.common.target_qualification_reserve = Some(M1QualificationTargetPageReserveV1 {
                device: self.pool.device(),
                allocation_id: target_allocation_id,
                request: cache.common.request,
                policy_identity: context.policy_identity(),
                grouping: context.grouping(),
                declared_workload_digest: context.declared_workload_digest(),
                lane: context.lane(),
                unused_pages: pages,
            });
        }
        Ok(M1QualificationTargetPagePreleaseSuccessV1 {
            pool: self.pool,
            caches: self.caches,
        })
    }
}

fn qualification_target_page_prelease_progress(
    pages_by_lane: &[Vec<DeviceKvPageLease>],
) -> M1QualificationTargetPagePreleaseProgressV1 {
    for (lane, pages) in pages_by_lane.iter().enumerate() {
        if pages.len() < M1_QUALIFICATION_TARGET_PAGE_COUNT_V1 {
            return M1QualificationTargetPagePreleaseProgressV1 {
                lane: u32::try_from(lane).unwrap_or(u32::MAX),
                page: u32::try_from(pages.len()).unwrap_or(u32::MAX),
            };
        }
    }
    M1QualificationTargetPagePreleaseProgressV1 {
        lane: u32::try_from(pages_by_lane.len()).unwrap_or(u32::MAX),
        page: 0,
    }
}

fn acquire_qualification_target_page_prefix<E>(
    caches: &[ActiveDeviceKvCache],
    pages_by_lane: &mut [Vec<DeviceKvPageLease>],
    mut lease_page: impl FnMut(RequestId, u32) -> Result<DeviceKvPageLease, E>,
) -> Result<(), (u32, u32, E)> {
    for lane in 0..caches.len() {
        while pages_by_lane[lane].len() < M1_QUALIFICATION_TARGET_PAGE_COUNT_V1 {
            let lane_u32 = u32::try_from(lane).unwrap_or(u32::MAX);
            let page = u32::try_from(pages_by_lane[lane].len()).unwrap_or(u32::MAX);
            let lease = lease_page(caches[lane].common.request, page)
                .map_err(|source| (lane_u32, page, source))?;
            pages_by_lane[lane].push(lease);
        }
    }
    Ok(())
}

/// Failed all-lane prelease retaining every linear owner for consuming retry.
#[must_use = "the exact partial-prelease owner must be retried or retained"]
#[derive(Debug)]
pub struct M1QualificationTargetPagePreleaseFailureV1 {
    error: M1QualificationTargetPagePreleaseErrorV1,
    recovery: M1QualificationTargetPagePreleaseRecoveryV1,
}

impl M1QualificationTargetPagePreleaseFailureV1 {
    /// Exact fail-closed diagnostic.
    #[must_use]
    pub const fn error(&self) -> &M1QualificationTargetPagePreleaseErrorV1 {
        &self.error
    }

    /// Exact retained lane/page progress.
    #[must_use]
    pub fn progress(&self) -> M1QualificationTargetPagePreleaseProgressV1 {
        self.recovery.progress()
    }

    /// Recovers the diagnostic and the only owner capable of retrying it.
    #[must_use = "the exact diagnostic and partial-prelease recovery remain paired"]
    pub fn into_parts(
        self,
    ) -> (
        M1QualificationTargetPagePreleaseErrorV1,
        M1QualificationTargetPagePreleaseRecoveryV1,
    ) {
        (self.error, self.recovery)
    }
}

/// Preleases exactly 512 target P16 pages for every ordered qualification lane.
///
/// All cache and ordinal-zero witness validation precedes the first page mint.
/// A later pool or host-allocation failure retains the pool, every cache, every
/// acquired page, and the exact retry coordinate inside the returned failure.
///
/// # Errors
///
/// Rejects grouping/cardinality, non-fresh or substituted caches, initial
/// witness drift, host custody allocation, or exact page-pool failures.
pub fn prelease_m1_qualification_target_pages_v1(
    pool: M1PartitionedModelMemoryKvPoolV1,
    caches: Vec<ActiveDeviceKvCache>,
    initial_contexts: Vec<crate::M1ValidatedQualificationContextStepV1>,
    grouping: M1QualificationLaneGrouping,
) -> Result<
    M1QualificationTargetPagePreleaseSuccessV1,
    Box<M1QualificationTargetPagePreleaseFailureV1>,
> {
    M1QualificationTargetPagePreleaseRecoveryV1 {
        pool,
        caches,
        initial_contexts,
        grouping,
        pages_by_lane: Vec::new(),
    }
    .retry()
}

const fn qualification_decode_bucket(grouping: M1QualificationLaneGrouping) -> Qwen3PlanBucket {
    match grouping {
        M1QualificationLaneGrouping::S1 => Qwen3PlanBucket::DecodeS1C8192,
        M1QualificationLaneGrouping::S8 => Qwen3PlanBucket::DecodeS8C8192,
        M1QualificationLaneGrouping::S32 => Qwen3PlanBucket::DecodeS32C8192,
    }
}

fn validate_qualification_prelease_inputs(
    pool: &M1PartitionedModelMemoryKvPoolV1,
    caches: &[ActiveDeviceKvCache],
    contexts: &[crate::M1ValidatedQualificationContextStepV1],
    grouping: M1QualificationLaneGrouping,
) -> Result<(), (u32, DeviceKvCacheError)> {
    let lane_count = usize::try_from(grouping.sequences()).unwrap_or(usize::MAX);
    if caches.len() != lane_count || contexts.len() != lane_count {
        return Err((0, DeviceKvCacheError::QualificationLaneCountMismatch));
    }
    let bucket = qualification_decode_bucket(grouping);
    for lane in 0..lane_count {
        let lane_u32 = u32::try_from(lane).unwrap_or(u32::MAX);
        let cache = &caches[lane];
        let context = contexts[lane];
        if cache.common.device != pool.device()
            || cache.common.target.selection()
                != (Qwen3PlanSelection {
                    role: Qwen3ModelRole::Target8B,
                    mode: Qwen3ExecutionMode::Decode,
                    bucket,
                })
            || cache.common.draft.selection()
                != (Qwen3PlanSelection {
                    role: Qwen3ModelRole::Draft06B,
                    mode: Qwen3ExecutionMode::Decode,
                    bucket,
                })
            || cache.common.target.logical().committed_tokens != 0
            || cache.common.target.logical().resident_tokens != 0
            || cache.common.draft.logical().committed_tokens != 0
            || cache.common.draft.logical().resident_tokens != 0
            || !cache.common.target.active_pages.is_empty()
            || !cache.common.target.retired_pages.is_empty()
            || !cache.common.draft.active_pages.is_empty()
            || !cache.common.draft.retired_pages.is_empty()
            || cache.common.target.pending.is_some()
            || cache.common.draft.pending.is_some()
            || cache.common.target.arena_allocation_id.is_some()
            || cache.common.draft.arena_allocation_id.is_some()
        {
            return Err((lane_u32, DeviceKvCacheError::QualificationCacheNotFresh));
        }
        if cache.common.target_qualification_reserve.is_some() {
            return Err((
                lane_u32,
                DeviceKvCacheError::QualificationReserveAlreadyInstalled,
            ));
        }
        if context.ordinal() != 0
            || context.grouping() != grouping
            || context.lane().lane_ordinal != lane_u32
            || (lane > 0
                && (!context
                    .policy_identity()
                    .equals(&contexts[0].policy_identity())
                    || !context
                        .declared_workload_digest()
                        .equals(&contexts[0].declared_workload_digest())))
        {
            return Err((
                lane_u32,
                DeviceKvCacheError::QualificationInitialWitnessMismatch,
            ));
        }
        if caches[..lane]
            .iter()
            .any(|prior| prior.common.request == cache.common.request)
        {
            return Err((lane_u32, DeviceKvCacheError::WrongRequest));
        }
    }
    Ok(())
}

/// Consumes exact model memory into sole generic KV plane partitions.
///
/// Both bounded page-generation ledgers and all host-only geometry are
/// prepared before the first generic owner mutation. The target partition is
/// then reserved before draft. If the second atomic reservation fails, the
/// failure retains the completed target witness and consumed model-memory
/// owner in an opaque quarantine state; no rollback is claimed.
///
/// # Errors
///
/// Returns [`M1DeviceKvArenaLeaseBindingFailureV1`] for model, geometry, host
/// allocation, generic owner, mapped-state, existing-partition, or sublease
/// reservation failure.
///
/// The service session and model-memory owner are both linear inputs. A safe
/// caller cannot create a second pool or retain a legacy range path:
///
/// ```compile_fail
/// use ferric_engine::{
///     bind_m1_partitioned_model_memory_kv_pool_v1, M1DeviceBoundModelMemoryV1,
/// };
/// fn partition_twice(initialized: M1DeviceBoundModelMemoryV1) {
///     let _first = bind_m1_partitioned_model_memory_kv_pool_v1(initialized);
///     let _second = bind_m1_partitioned_model_memory_kv_pool_v1(initialized);
/// }
/// ```
pub fn bind_m1_partitioned_model_memory_kv_pool_v1(
    initialized: M1DeviceBoundModelMemoryV1,
) -> Result<M1PartitionedModelMemoryKvPoolV1, M1DeviceKvArenaLeaseBindingFailureV1> {
    let (device, mut allocations, model_memory) = initialized.into_parts();
    if let Err(source) = model_memory.revalidate_for_kv_partition() {
        return Err(unpartitioned_kv_pool_failure(
            M1DeviceKvArenaLeaseErrorV1::ModelMemory(source),
            device,
            model_memory,
            allocations,
        ));
    }
    let target_layout = match kv_plane_partition_layout::<M1_TARGET_KV_PLANE_SUBLEASES_V1>(
        &model_memory,
        Qwen3ModelRole::Target8B,
    ) {
        Ok(layout) => layout,
        Err(error) => {
            return Err(unpartitioned_kv_pool_failure(
                error,
                device,
                model_memory,
                allocations,
            ));
        }
    };
    let draft_layout = match kv_plane_partition_layout::<M1_DRAFT_KV_PLANE_SUBLEASES_V1>(
        &model_memory,
        Qwen3ModelRole::Draft06B,
    ) {
        Ok(layout) => layout,
        Err(error) => {
            return Err(unpartitioned_kv_pool_failure(
                error,
                device,
                model_memory,
                allocations,
            ));
        }
    };
    let target_pages = match new_page_ledger(Qwen3ModelRole::Target8B) {
        Ok(ledger) => ledger,
        Err(error) => {
            return Err(unpartitioned_kv_pool_failure(
                error,
                device,
                model_memory,
                allocations,
            ));
        }
    };
    let draft_pages = match new_page_ledger(Qwen3ModelRole::Draft06B) {
        Ok(ledger) => ledger,
        Err(error) => {
            return Err(unpartitioned_kv_pool_failure(
                error,
                device,
                model_memory,
                allocations,
            ));
        }
    };

    for role in [Qwen3ModelRole::Target8B, Qwen3ModelRole::Draft06B] {
        let key = model_memory.kv_allocation_key(role);
        if let Err(source) =
            allocations.range(key, 0, key.extent_bytes(), QWEN3_KV_ARENA_ALIGNMENT_V1)
        {
            return Err(unpartitioned_kv_pool_failure(
                M1DeviceKvArenaLeaseErrorV1::Allocation { role, source },
                device,
                model_memory,
                allocations,
            ));
        }
    }

    let target_planes = match allocations.reserve_disjoint_subleases(
        model_memory.kv_allocation_key(Qwen3ModelRole::Target8B),
        target_layout,
    ) {
        Ok(subleases) => subleases,
        Err(source) => {
            return Err(unpartitioned_kv_pool_failure(
                M1DeviceKvArenaLeaseErrorV1::Allocation {
                    role: Qwen3ModelRole::Target8B,
                    source,
                },
                device,
                model_memory,
                allocations,
            ));
        }
    };
    let draft_planes = match allocations.reserve_disjoint_subleases(
        model_memory.kv_allocation_key(Qwen3ModelRole::Draft06B),
        draft_layout,
    ) {
        Ok(subleases) => subleases,
        Err(source) => {
            return Err(M1DeviceKvArenaLeaseBindingFailureV1 {
                error: M1DeviceKvArenaLeaseErrorV1::Allocation {
                    role: Qwen3ModelRole::Draft06B,
                    source,
                },
                recovery: Box::new(M1DeviceKvArenaLeaseRecoveryV1::TargetPartitioned(Box::new(
                    M1TargetPartitionedKvQuarantineV1 {
                        device,
                        model_memory,
                        allocations,
                        target_planes,
                    },
                ))),
            });
        }
    };

    Ok(M1PartitionedModelMemoryKvPoolV1 {
        device,
        model_memory,
        allocations,
        target_planes,
        draft_planes,
        target_pages,
        draft_pages,
    })
}

fn unpartitioned_kv_pool_failure(
    error: M1DeviceKvArenaLeaseErrorV1,
    device: Gfx942DeviceBinding,
    model_memory: BoundModelMemoryAllocationsV1,
    allocations: ServiceAllocationSessionV1,
) -> M1DeviceKvArenaLeaseBindingFailureV1 {
    M1DeviceKvArenaLeaseBindingFailureV1 {
        error,
        recovery: Box::new(M1DeviceKvArenaLeaseRecoveryV1::Unpartitioned(Box::new(
            M1UnpartitionedModelMemoryKvRecoveryV1 {
                initialized: M1DeviceBoundModelMemoryV1::from_parts(
                    device,
                    allocations,
                    model_memory,
                ),
            },
        ))),
    }
}

fn new_page_ledger(
    role: Qwen3ModelRole,
) -> Result<Box<[M1KvPoolPageStateV1]>, M1DeviceKvArenaLeaseErrorV1> {
    let mut slots = Vec::new();
    slots
        .try_reserve_exact(M1_GLOBAL_KV_PAGE_SLOTS_V1)
        .map_err(|_| M1DeviceKvArenaLeaseErrorV1::HostLedgerAllocation { role })?;
    slots.resize(M1_GLOBAL_KV_PAGE_SLOTS_V1, M1KvPoolPageStateV1::INITIAL);
    Ok(slots.into_boxed_slice())
}

fn free_page_generation(state: M1KvPoolPageStateV1) -> Result<u32, M1DeviceKvArenaLeaseErrorV1> {
    let M1KvPoolPageStateV1::Free { generation } = state else {
        return Err(M1DeviceKvArenaLeaseErrorV1::PageAlreadyLeased);
    };
    if generation == 0 {
        return Err(M1DeviceKvArenaLeaseErrorV1::GenerationLedgerDrift);
    }
    Ok(generation)
}

fn validate_leased_page_state(
    state: Option<M1KvPoolPageStateV1>,
    request: RequestId,
    generation: u32,
) -> Result<(), M1DeviceKvArenaLeaseErrorV1> {
    if generation == 0 {
        return Err(M1DeviceKvArenaLeaseErrorV1::GenerationLedgerDrift);
    }
    if state
        != Some(M1KvPoolPageStateV1::Leased {
            request,
            generation,
        })
    {
        return Err(M1DeviceKvArenaLeaseErrorV1::PageLeaseMismatch);
    }
    Ok(())
}

fn kv_plane_partition_layout<const N: usize>(
    model_memory: &BoundModelMemoryAllocationsV1,
    role: Qwen3ModelRole,
) -> Result<[(u64, u64, u64); N], M1DeviceKvArenaLeaseErrorV1> {
    if N != role.layers() as usize * 2 {
        return Err(M1DeviceKvArenaLeaseErrorV1::PlaneGeometry { role });
    }
    let expected_plane_bytes = u64::try_from(M1_GLOBAL_KV_PAGE_SLOTS_V1)
        .ok()
        .and_then(|slots| slots.checked_mul(QWEN3_KV_PAGE_BYTES_V1))
        .ok_or(M1DeviceKvArenaLeaseErrorV1::PlaneGeometry { role })?;
    let allocation_id =
        model_memory.selected_allocation_identity(role, ModelMemoryAllocationKind::KvArena);
    let mut members = [(0, 0, 0); N];
    let mut member_index = 0usize;
    for layer in 0..role.layers() {
        for component in [KvCacheComponent::Key, KvCacheComponent::Value] {
            let range = model_memory
                .plan()
                .kv_layer(role, component, layer)
                .map_err(M1DeviceKvArenaLeaseErrorV1::ModelPlan)?;
            let expected_offset = u64::try_from(member_index)
                .ok()
                .and_then(|index| index.checked_mul(expected_plane_bytes))
                .ok_or(M1DeviceKvArenaLeaseErrorV1::PlaneGeometry { role })?;
            if range.allocation_id() != allocation_id
                || range.offset() != expected_offset
                || range.byte_len() != expected_plane_bytes
            {
                return Err(M1DeviceKvArenaLeaseErrorV1::PlaneGeometry { role });
            }
            members[member_index] = (
                range.offset(),
                range.byte_len(),
                QWEN3_KV_ARENA_ALIGNMENT_V1,
            );
            member_index += 1;
        }
    }
    if member_index != N {
        return Err(M1DeviceKvArenaLeaseErrorV1::PlaneGeometry { role });
    }
    Ok(members)
}

fn plane_member_index(
    role: Qwen3ModelRole,
    component: KvCacheComponent,
    layer: u32,
) -> Result<usize, M1DeviceKvArenaLeaseErrorV1> {
    if layer >= role.layers() {
        return Err(M1DeviceKvArenaLeaseErrorV1::ModelPlan(
            ModelMemoryPlanError::LayerOutOfRange { role, layer },
        ));
    }
    let component_index = match component {
        KvCacheComponent::Key => 0usize,
        KvCacheComponent::Value => 1usize,
    };
    usize::try_from(layer)
        .ok()
        .and_then(|layer| layer.checked_mul(2))
        .and_then(|base| base.checked_add(component_index))
        .ok_or(M1DeviceKvArenaLeaseErrorV1::PlaneGeometry { role })
}

fn global_page_index(
    request: RequestId,
    physical_index: u32,
) -> Result<usize, M1DeviceKvArenaLeaseErrorV1> {
    if request.generation() == 0 || request.slot() >= M1_MAX_ACTIVE_SEQUENCES {
        return Err(M1DeviceKvArenaLeaseErrorV1::RequestOutOfRange);
    }
    let local_index =
        usize::try_from(physical_index).map_err(|_| M1DeviceKvArenaLeaseErrorV1::PageOutOfRange)?;
    if local_index >= M1_KV_PHYSICAL_PAGE_SLOTS {
        return Err(M1DeviceKvArenaLeaseErrorV1::PageOutOfRange);
    }
    usize::try_from(request.slot())
        .ok()
        .and_then(|slot| slot.checked_mul(M1_KV_PHYSICAL_PAGE_SLOTS))
        .and_then(|base| base.checked_add(local_index))
        .filter(|index| *index < M1_GLOBAL_KV_PAGE_SLOTS_V1)
        .ok_or(M1DeviceKvArenaLeaseErrorV1::PageOutOfRange)
}

#[cfg(test)]
impl DeviceKvPageLease {
    pub(crate) fn from_contracted_workspace_bridge_test_allocation(
        device: Gfx942DeviceBinding,
        allocation_id: Identity,
        request: RequestId,
        page: PhysicalPageId,
    ) -> Self {
        Self {
            device,
            allocation_id,
            request,
            page,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PendingWriteBinding {
    device: Gfx942DeviceBinding,
    allocation_id: Identity,
    request: RequestId,
    selection: Qwen3PlanSelection,
    page: PhysicalPageId,
    logical_position: u32,
    epoch: CompletionEpoch,
    write_generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PendingStepWriteBinding {
    device: Gfx942DeviceBinding,
    request: RequestId,
    selection: Qwen3PlanSelection,
    committed_tokens: u32,
    active_tokens: u32,
    end_tokens: u32,
    epoch: CompletionEpoch,
    write_generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PendingWriteState {
    Token(PendingWriteBinding),
    Step(PendingStepWriteBinding),
}

/// Non-clone request to initialize exactly the next token in one owned page.
///
/// Preparing this value does not initialize memory or advance the cache.
#[derive(Debug, PartialEq, Eq)]
pub struct PendingDeviceKvWrite {
    binding: PendingWriteBinding,
}

/// Addressless identity for one required logical page-table entry.
///
/// This is an inert observation. It contains no device address, allocation
/// authority, initialized-memory claim, or completion evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeviceKvStepPageIdentity {
    logical_page: u32,
    allocation_id: Identity,
    page: PhysicalPageId,
}

impl DeviceKvStepPageIdentity {
    #[must_use]
    pub const fn logical_page(&self) -> u32 {
        self.logical_page
    }

    #[must_use]
    pub const fn allocation_id(&self) -> Identity {
        self.allocation_id
    }

    #[must_use]
    pub const fn page(&self) -> PhysicalPageId {
        self.page
    }
}

/// Exact token subrange written within one required step page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeviceKvStepPageBinding {
    identity: DeviceKvStepPageIdentity,
    first_offset: u32,
    token_count: u32,
}

impl DeviceKvStepPageBinding {
    #[must_use]
    pub const fn logical_page(&self) -> u32 {
        self.identity.logical_page
    }

    #[must_use]
    pub const fn allocation_id(&self) -> Identity {
        self.identity.allocation_id
    }

    #[must_use]
    pub const fn page(&self) -> PhysicalPageId {
        self.identity.page
    }

    #[must_use]
    pub const fn first_offset(&self) -> u32 {
        self.first_offset
    }

    #[must_use]
    pub const fn token_count(&self) -> u32 {
        self.token_count
    }
}

/// Linear reservation of one role's exact next step-write interval.
///
/// The reservation covers `[committed_tokens, committed_tokens +
/// active_tokens)` and retains every not-yet-bound page lease needed by that
/// interval. Preparing it does not append pages, initialize memory, or prove
/// that a dispatch completed. It can be inspected, aborted, or consumed by the
/// crate-private exact-completion bridge that initializes the whole interval.
///
/// This type is intentionally not `Clone`.
///
/// ```compile_fail
/// use ferric_engine::PendingDeviceKvStepWrite;
/// fn require_clone<T: Clone>() {}
/// require_clone::<PendingDeviceKvStepWrite>();
/// ```
///
/// ```compile_fail
/// use ferric_engine::PendingDeviceKvStepWrite;
/// fn consume_once(_: PendingDeviceKvStepWrite) {}
/// fn consume_twice(pending: PendingDeviceKvStepWrite) {
///     consume_once(pending);
///     consume_once(pending);
/// }
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct PendingDeviceKvStepWrite {
    binding: PendingStepWriteBinding,
    page_table: Box<[DeviceKvStepPageIdentity]>,
    write_pages: Box<[DeviceKvStepPageBinding]>,
    new_page_leases: Vec<DeviceKvPageLease>,
}

/// Typed one-token target-KV reservation for an authenticated C8192 context step.
///
/// This wrapper binds the ordinary physical write reservation to the exact
/// validated policy/workload/lane/ordinal witness that authorized consuming a
/// future page. It is intentionally linear and can be unwrapped only by value.
#[must_use = "qualification context and pending target write must proceed together"]
#[derive(Debug, PartialEq, Eq)]
pub struct M1PendingQualificationContextStepWriteV1 {
    context: crate::M1ValidatedQualificationContextStepV1,
    pending: PendingDeviceKvStepWrite,
    active_pages: usize,
    unused_future_pages: usize,
}

/// Transactional qualification-step rejection.
///
/// The borrowed cache has already recovered any temporarily popped future page
/// before this value is returned, so the diagnostic owns no linear resource.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct M1QualificationContextStepReservationFailureV1 {
    error: DeviceKvCacheError,
}

impl M1QualificationContextStepReservationFailureV1 {
    /// Exact fail-closed cache or witness diagnostic.
    #[must_use]
    pub const fn error(self) -> DeviceKvCacheError {
        self.error
    }
}

impl M1PendingQualificationContextStepWriteV1 {
    /// Exact validated context witness authorizing this physical write.
    #[must_use]
    pub const fn context(&self) -> crate::M1ValidatedQualificationContextStepV1 {
        self.context
    }

    /// Borrows the ordinary one-token pending write for workspace binding.
    #[must_use]
    pub const fn pending_step_write(&self) -> &PendingDeviceKvStepWrite {
        &self.pending
    }

    /// Exact number of already-active target pages at reservation time.
    #[must_use]
    pub const fn active_page_count(&self) -> usize {
        self.active_pages
    }

    /// Exact number of pages transferred from the future reserve into this write.
    #[must_use]
    pub const fn pending_new_page_count(&self) -> usize {
        self.pending.new_page_count()
    }

    /// Exact future-page suffix remaining inside the cache.
    #[must_use]
    pub const fn unused_future_page_count(&self) -> usize {
        self.unused_future_pages
    }

    /// Checks the fixed 512-page conservation equation for this reservation.
    #[must_use]
    pub const fn conserves_target_pages(&self) -> bool {
        self.active_pages
            .saturating_add(self.pending.new_page_count())
            .saturating_add(self.unused_future_pages)
            == M1_QUALIFICATION_TARGET_PAGE_COUNT_V1
    }

    /// Recovers the ordinary pending write for the exact-completion bridge.
    #[must_use = "the pending target write remains linear"]
    pub fn into_pending_step_write(self) -> PendingDeviceKvStepWrite {
        self.pending
    }
}

/// One full speculative round's aggregate draft-KV reservation.
///
/// The cache reservation remains bound to the paired draft-speculative
/// selection, while `draft_decode_selection` names the one-token workspace
/// reused by each of the K sequential draft segments. `draft_tokens` is derived
/// only from the exact target speculative bucket. This wrapper is intentionally
/// not `Clone`.
///
/// ```compile_fail
/// use ferric_engine::PendingSpeculativeDraftKvRoundWrite;
/// fn require_clone<T: Clone>() {}
/// require_clone::<PendingSpeculativeDraftKvRoundWrite>();
/// ```
#[must_use = "the aggregate draft-round reservation must remain retained until settled or aborted"]
#[derive(Debug, PartialEq, Eq)]
pub struct PendingSpeculativeDraftKvRoundWrite {
    target_speculative_selection: Qwen3PlanSelection,
    draft_decode_selection: Qwen3PlanSelection,
    draft_tokens: u32,
    pending: PendingDeviceKvStepWrite,
}

impl PendingSpeculativeDraftKvRoundWrite {
    /// Returns the exact target speculative selection that defines K.
    #[must_use]
    pub const fn target_speculative_selection(&self) -> Qwen3PlanSelection {
        self.target_speculative_selection
    }

    /// Returns the exact reusable one-token draft-decode selection.
    #[must_use]
    pub const fn draft_decode_selection(&self) -> Qwen3PlanSelection {
        self.draft_decode_selection
    }

    /// Returns the exact K4, K8, or K16 aggregate write width.
    #[must_use]
    pub const fn draft_tokens(&self) -> u32 {
        self.draft_tokens
    }

    /// Borrows the one underlying pending marker and physical-page snapshot.
    #[must_use]
    pub const fn pending_step_write(&self) -> &PendingDeviceKvStepWrite {
        &self.pending
    }

    /// Recovers the one pending step write for abort or exact completion.
    #[must_use = "the single pending write remains linear"]
    pub fn into_pending_step_write(self) -> PendingDeviceKvStepWrite {
        self.pending
    }
}

pub(crate) fn m1_speculative_draft_round_shape_v1(
    target_speculative_selection: Qwen3PlanSelection,
) -> Option<(Qwen3PlanSelection, Qwen3PlanSelection, u32)> {
    if target_speculative_selection.role != Qwen3ModelRole::Target8B
        || target_speculative_selection.mode != Qwen3ExecutionMode::Speculative
    {
        return None;
    }
    let (draft_decode_bucket, draft_tokens) = match target_speculative_selection.bucket {
        Qwen3PlanBucket::SpeculativeS1K4C8192 => (Qwen3PlanBucket::DecodeS1C8192, 4),
        Qwen3PlanBucket::SpeculativeS8K4C8192 => (Qwen3PlanBucket::DecodeS8C8192, 4),
        Qwen3PlanBucket::SpeculativeS1K8C8192 => (Qwen3PlanBucket::DecodeS1C8192, 8),
        Qwen3PlanBucket::SpeculativeS1K16C8192 => (Qwen3PlanBucket::DecodeS1C8192, 16),
        Qwen3PlanBucket::PrefillS1T128
        | Qwen3PlanBucket::PrefillS8T128
        | Qwen3PlanBucket::PrefillS1T512
        | Qwen3PlanBucket::PrefillS1T2048
        | Qwen3PlanBucket::DecodeS1C8192
        | Qwen3PlanBucket::DecodeS8C8192
        | Qwen3PlanBucket::DecodeS32C8192 => return None,
    };
    Some((
        Qwen3PlanSelection {
            role: Qwen3ModelRole::Draft06B,
            mode: Qwen3ExecutionMode::Speculative,
            bucket: target_speculative_selection.bucket,
        },
        Qwen3PlanSelection {
            role: Qwen3ModelRole::Draft06B,
            mode: Qwen3ExecutionMode::Decode,
            bucket: draft_decode_bucket,
        },
        draft_tokens,
    ))
}

impl PendingDeviceKvStepWrite {
    #[must_use]
    pub const fn request(&self) -> RequestId {
        self.binding.request
    }

    #[must_use]
    pub const fn selection(&self) -> Qwen3PlanSelection {
        self.binding.selection
    }

    #[must_use]
    pub const fn committed_tokens(&self) -> u32 {
        self.binding.committed_tokens
    }

    #[must_use]
    pub const fn active_tokens(&self) -> u32 {
        self.binding.active_tokens
    }

    #[must_use]
    pub const fn end_tokens(&self) -> u32 {
        self.binding.end_tokens
    }

    #[must_use]
    pub const fn epoch(&self) -> CompletionEpoch {
        self.binding.epoch
    }

    #[must_use]
    pub const fn page_table(&self) -> &[DeviceKvStepPageIdentity] {
        &self.page_table
    }

    /// Exact nonempty per-page spans partitioning the step-write interval.
    #[must_use]
    pub const fn write_pages(&self) -> &[DeviceKvStepPageBinding] {
        &self.write_pages
    }

    /// Number of page leases retained for page-table positions not yet bound.
    #[must_use]
    pub const fn new_page_count(&self) -> usize {
        self.new_page_leases.len()
    }
}

#[cfg(test)]
impl PendingSpeculativeDraftKvRoundWrite {
    pub(crate) fn corrupt_target_selection_for_test(&mut self, selection: Qwen3PlanSelection) {
        self.target_speculative_selection = selection;
    }

    pub(crate) fn corrupt_draft_decode_selection_for_test(
        &mut self,
        selection: Qwen3PlanSelection,
    ) {
        self.draft_decode_selection = selection;
    }

    pub(crate) fn corrupt_draft_tokens_for_test(&mut self, draft_tokens: u32) {
        self.draft_tokens = draft_tokens;
    }

    pub(crate) fn corrupt_page_for_test(
        &mut self,
        entry: usize,
        logical_page: u32,
        allocation_id: Identity,
        page: PhysicalPageId,
    ) {
        self.pending.corrupt_workspace_bridge_page_for_test(
            entry,
            logical_page,
            allocation_id,
            page,
        );
    }
}

/// Inert record of the exact step interval installed in physical KV state.
///
/// This value owns no page lease, device allocation, queue, signal, or
/// [`ExactCompletion`]. It is kept non-clone so the eventual runner can move one
/// exact interval record through its sequential completion fan-out.
#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct InertInitializedDeviceKvStepWrite {
    binding: PendingStepWriteBinding,
    page_table: Box<[DeviceKvStepPageIdentity]>,
    write_pages: Box<[DeviceKvStepPageBinding]>,
}

#[allow(dead_code)]
impl InertInitializedDeviceKvStepWrite {
    pub(crate) const fn request(&self) -> RequestId {
        self.binding.request
    }

    pub(crate) const fn selection(&self) -> Qwen3PlanSelection {
        self.binding.selection
    }

    pub(crate) const fn committed_tokens(&self) -> u32 {
        self.binding.committed_tokens
    }

    pub(crate) const fn active_tokens(&self) -> u32 {
        self.binding.active_tokens
    }

    pub(crate) const fn end_tokens(&self) -> u32 {
        self.binding.end_tokens
    }

    pub(crate) const fn epoch(&self) -> CompletionEpoch {
        self.binding.epoch
    }

    pub(crate) const fn page_table(&self) -> &[DeviceKvStepPageIdentity] {
        &self.page_table
    }

    pub(crate) const fn write_pages(&self) -> &[DeviceKvStepPageBinding] {
        &self.write_pages
    }
}

/// Successful initialized-step transition retaining active cache custody.
#[allow(dead_code)]
#[must_use = "active cache custody, interval evidence, and exact completion remain linear"]
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CompletedDeviceKvStepWrite {
    cache: ActiveDeviceKvCache,
    initialized: InertInitializedDeviceKvStepWrite,
    completion: ExactCompletion,
}

#[allow(dead_code)]
impl CompletedDeviceKvStepWrite {
    pub(crate) fn into_parts(
        self,
    ) -> (
        ActiveDeviceKvCache,
        InertInitializedDeviceKvStepWrite,
        ExactCompletion,
    ) {
        (self.cache, self.initialized, self.completion)
    }
}

/// Retry-safe step-completion rejection retaining all unchanged inputs.
#[allow(dead_code)]
#[must_use = "rejection retains cache, reservation, and exact completion custody"]
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct DeviceKvStepCompletionFailure {
    error: DeviceKvCacheError,
    cache: ActiveDeviceKvCache,
    pending: PendingDeviceKvStepWrite,
    completion: ExactCompletion,
}

#[allow(dead_code)]
impl DeviceKvStepCompletionFailure {
    pub(crate) const fn error(&self) -> DeviceKvCacheError {
        self.error
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        DeviceKvCacheError,
        ActiveDeviceKvCache,
        PendingDeviceKvStepWrite,
        ExactCompletion,
    ) {
        (self.error, self.cache, self.pending, self.completion)
    }
}

/// Terminal custody after an impossible post-preflight model transition.
///
/// Some new leases may already have moved into `common`; any not-yet-appended
/// leases remain in `unappended_page_leases`. No active-cache recovery,
/// mutation, read, release, or reuse operation is exposed.
#[allow(dead_code)]
#[must_use = "poisoned custody must remain quarantined"]
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct PoisonedDeviceKvStepCompletion {
    error: DeviceKvCacheError,
    common: DeviceKvCacheCommon,
    binding: PendingStepWriteBinding,
    page_table: Box<[DeviceKvStepPageIdentity]>,
    write_pages: Box<[DeviceKvStepPageBinding]>,
    unappended_page_leases: Vec<DeviceKvPageLease>,
    completion: ExactCompletion,
}

#[allow(dead_code)]
impl PoisonedDeviceKvStepCompletion {
    pub(crate) const fn error(&self) -> DeviceKvCacheError {
        self.error
    }

    pub(crate) fn projection(&self) -> DeviceKvCacheProjection {
        self.common.projection()
    }

    pub(crate) const fn completion_epoch(&self) -> CompletionEpoch {
        self.completion.epoch()
    }
}

/// Exhaustive result of joining one pending step write to exact completion.
#[allow(dead_code)]
#[must_use = "every outcome retains linear cache and completion custody"]
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum DeviceKvStepCompletionOutcome {
    Completed(CompletedDeviceKvStepWrite),
    Rejected(DeviceKvStepCompletionFailure),
    Poisoned(PoisonedDeviceKvStepCompletion),
}

#[cfg(test)]
impl PendingDeviceKvStepWrite {
    pub(crate) fn corrupt_workspace_bridge_page_for_test(
        &mut self,
        entry: usize,
        logical_page: u32,
        allocation_id: Identity,
        page: PhysicalPageId,
    ) {
        self.page_table[entry] = DeviceKvStepPageIdentity {
            logical_page,
            allocation_id,
            page,
        };
    }

    pub(crate) fn corrupt_completion_bridge_request_for_test(&mut self, request: RequestId) {
        self.binding.request = request;
    }

    pub(crate) fn corrupt_completion_bridge_selection_for_test(
        &mut self,
        selection: Qwen3PlanSelection,
    ) {
        self.binding.selection = selection;
    }

    pub(crate) fn corrupt_completion_bridge_write_span_for_test(
        &mut self,
        entry: usize,
        token_count: u32,
    ) {
        self.write_pages[entry].token_count = token_count;
    }
}

/// Retry-safe step reservation rejection retaining every supplied page lease.
#[derive(Debug, PartialEq, Eq)]
pub struct DeviceKvStepReservationFailure {
    error: DeviceKvCacheError,
    page_leases: Vec<DeviceKvPageLease>,
}

impl DeviceKvStepReservationFailure {
    #[must_use]
    pub const fn error(&self) -> DeviceKvCacheError {
        self.error
    }

    #[must_use]
    pub fn into_parts(self) -> (DeviceKvCacheError, Vec<DeviceKvPageLease>) {
        (self.error, self.page_leases)
    }
}

/// Leases recovered by aborting a pending step reservation.
#[derive(Debug, PartialEq, Eq)]
pub struct AbortedDeviceKvStepWrite {
    page_leases: Vec<DeviceKvPageLease>,
}

impl AbortedDeviceKvStepWrite {
    #[must_use]
    pub const fn page_count(&self) -> usize {
        self.page_leases.len()
    }

    #[must_use]
    pub fn into_page_leases(self) -> Vec<DeviceKvPageLease> {
        self.page_leases
    }
}

/// Retry-safe abort rejection retaining the exact linear reservation.
#[derive(Debug, PartialEq, Eq)]
pub struct DeviceKvStepAbortFailure {
    error: DeviceKvCacheError,
    pending: PendingDeviceKvStepWrite,
}

impl DeviceKvStepAbortFailure {
    #[must_use]
    pub const fn error(&self) -> DeviceKvCacheError {
        self.error
    }

    #[must_use]
    pub fn into_parts(self) -> (DeviceKvCacheError, PendingDeviceKvStepWrite) {
        (self.error, self.pending)
    }
}

impl PendingDeviceKvWrite {
    #[must_use]
    pub const fn request(&self) -> RequestId {
        self.binding.request
    }

    #[must_use]
    pub const fn selection(&self) -> Qwen3PlanSelection {
        self.binding.selection
    }

    #[must_use]
    pub const fn page(&self) -> PhysicalPageId {
        self.binding.page
    }

    #[must_use]
    pub const fn logical_position(&self) -> u32 {
        self.binding.logical_position
    }

    #[must_use]
    pub const fn epoch(&self) -> CompletionEpoch {
        self.binding.epoch
    }
}

/// Retry-safe failure retaining both linear authority inputs.
#[derive(Debug, PartialEq, Eq)]
pub struct PendingWriteCompletionFailure {
    error: DeviceKvCacheError,
    pending: PendingDeviceKvWrite,
    completion: ExactCompletion,
}

impl PendingWriteCompletionFailure {
    #[must_use]
    pub const fn error(&self) -> DeviceKvCacheError {
        self.error
    }

    #[must_use]
    pub fn into_parts(self) -> (PendingDeviceKvWrite, ExactCompletion) {
        (self.pending, self.completion)
    }
}

/// Non-clone authority that one exact pending physical write completed.
#[derive(Debug, PartialEq, Eq)]
pub struct InitializedDeviceKvWrite {
    binding: PendingWriteBinding,
    completion: ExactCompletion,
}

/// Copyable initialized-location observation. It owns no allocation or address.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeviceKvReadBinding {
    pub device: Gfx942DeviceBinding,
    pub allocation_id: Identity,
    pub request: RequestId,
    pub selection: Qwen3PlanSelection,
    pub logical_position: u32,
    pub location: PhysicalKvLocation,
}

/// Copyable observation of the exact target/draft source-level projection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeviceKvCacheProjection {
    pub device: Gfx942DeviceBinding,
    pub request: RequestId,
    pub target: LogicalKvState,
    pub draft: LogicalKvState,
    pub target_arena_allocation_id: Option<Identity>,
    pub draft_arena_allocation_id: Option<Identity>,
    pub target_active_pages: usize,
    pub draft_active_pages: usize,
    pub target_retired_pages: usize,
    pub draft_retired_pages: usize,
    pub target_quiescent_retired_pages: usize,
    pub draft_quiescent_retired_pages: usize,
    pub target_write_pending: bool,
    pub draft_write_pending: bool,
    pub target_qualification_future_pages: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct RetiredPageLease {
    lease: DeviceKvPageLease,
    after_epoch: CompletionEpoch,
    quiescent: bool,
}

impl RetiredPageLease {
    pub(crate) const fn lease(&self) -> &DeviceKvPageLease {
        &self.lease
    }

    pub(crate) const fn after_epoch(&self) -> CompletionEpoch {
        self.after_epoch
    }

    pub(crate) const fn is_quiescent(&self) -> bool {
        self.quiescent
    }

    pub(crate) fn into_lease(self) -> DeviceKvPageLease {
        self.lease
    }
}

#[derive(Debug, PartialEq, Eq)]
struct RoleDeviceKvCache {
    physical: PhysicalKvState,
    arena_allocation_id: Option<Identity>,
    active_pages: Vec<DeviceKvPageLease>,
    retired_pages: Vec<RetiredPageLease>,
    pending: Option<PendingWriteState>,
    next_write_generation: u64,
}

impl RoleDeviceKvCache {
    fn new(request: RequestId, selection: Qwen3PlanSelection) -> Result<Self, DeviceKvCacheError> {
        let physical = PhysicalKvState::new(request, selection)?;
        Ok(Self {
            physical,
            arena_allocation_id: None,
            active_pages: Vec::with_capacity(M1_KV_PAGE_TABLE_ENTRIES),
            retired_pages: Vec::with_capacity(M1_KV_PAGE_TABLE_ENTRIES),
            pending: None,
            next_write_generation: 1,
        })
    }

    const fn selection(&self) -> Qwen3PlanSelection {
        self.physical.selection()
    }

    const fn logical(&self) -> LogicalKvState {
        self.physical.logical_state()
    }
}

#[derive(Debug, PartialEq, Eq)]
struct DeviceKvCacheCommon {
    device: Gfx942DeviceBinding,
    request: RequestId,
    target: RoleDeviceKvCache,
    draft: RoleDeviceKvCache,
    target_qualification_reserve: Option<M1QualificationTargetPageReserveV1>,
}

impl DeviceKvCacheCommon {
    fn projection(&self) -> DeviceKvCacheProjection {
        DeviceKvCacheProjection {
            device: self.device,
            request: self.request,
            target: self.target.logical(),
            draft: self.draft.logical(),
            target_arena_allocation_id: self.target.arena_allocation_id,
            draft_arena_allocation_id: self.draft.arena_allocation_id,
            target_active_pages: self.target.active_pages.len(),
            draft_active_pages: self.draft.active_pages.len(),
            target_retired_pages: self.target.retired_pages.len(),
            draft_retired_pages: self.draft.retired_pages.len(),
            target_quiescent_retired_pages: self
                .target
                .retired_pages
                .iter()
                .filter(|retired| retired.quiescent)
                .count(),
            draft_quiescent_retired_pages: self
                .draft
                .retired_pages
                .iter()
                .filter(|retired| retired.quiescent)
                .count(),
            target_write_pending: self.target.pending.is_some(),
            draft_write_pending: self.draft.pending.is_some(),
            target_qualification_future_pages: self
                .target_qualification_reserve
                .as_ref()
                .map_or(0, M1QualificationTargetPageReserveV1::unused_page_count),
        }
    }

    fn role(&self, role: Qwen3ModelRole) -> &RoleDeviceKvCache {
        match role {
            Qwen3ModelRole::Target8B => &self.target,
            Qwen3ModelRole::Draft06B => &self.draft,
        }
    }

    fn role_mut(&mut self, role: Qwen3ModelRole) -> &mut RoleDeviceKvCache {
        match role {
            Qwen3ModelRole::Target8B => &mut self.target,
            Qwen3ModelRole::Draft06B => &mut self.draft,
        }
    }

    fn other_role_arena(&self, role: Qwen3ModelRole) -> Option<Identity> {
        match role {
            Qwen3ModelRole::Target8B => self.draft.arena_allocation_id,
            Qwen3ModelRole::Draft06B => self.target.arena_allocation_id,
        }
    }

    fn owned_table_matches(cache: &RoleDeviceKvCache) -> bool {
        usize::try_from(cache.physical.page_count()) == Ok(cache.active_pages.len())
            && cache.arena_allocation_id.is_some()
                == (!cache.active_pages.is_empty() || !cache.retired_pages.is_empty())
            && cache
                .active_pages
                .iter()
                .enumerate()
                .all(|(position, lease)| {
                    cache
                        .arena_allocation_id
                        .is_some_and(|arena| arena.equals(&lease.allocation_id))
                        && u32::try_from(position)
                            .ok()
                            .and_then(|position| cache.physical.page_at(position))
                            == Some(lease.page)
                })
            && cache.retired_pages.iter().all(|retired| {
                cache
                    .arena_allocation_id
                    .is_some_and(|arena| arena.equals(&retired.lease.allocation_id))
            })
    }

    fn release_state_is_valid(&self) -> bool {
        self.target.pending.is_none()
            && self.draft.pending.is_none()
            && self
                .target_qualification_reserve
                .as_ref()
                .is_none_or(|reserve| reserve.unused_pages.is_empty())
            && Self::owned_table_matches(&self.target)
            && Self::owned_table_matches(&self.draft)
    }

    fn retired_pages(&self, role: Qwen3ModelRole) -> &[RetiredPageLease] {
        &self.role(role).retired_pages
    }

    fn take_retired_pages(&mut self, role: Qwen3ModelRole) -> Vec<RetiredPageLease> {
        let cache = self.role_mut(role);
        let retired = core::mem::take(&mut cache.retired_pages);
        if cache.active_pages.is_empty() {
            cache.arena_allocation_id = None;
        }
        retired
    }

    fn settle_retired_epoch(
        &mut self,
        completion: ExactCompletion,
    ) -> Result<(usize, ExactCompletion), RetirementCompletionFailure> {
        let exact_epoch = completion.epoch();
        let matching = self
            .target
            .retired_pages
            .iter()
            .chain(self.draft.retired_pages.iter())
            .filter(|retired| !retired.quiescent && retired.after_epoch == exact_epoch)
            .count();
        if matching == 0 {
            return Err(RetirementCompletionFailure {
                error: DeviceKvCacheError::NoRetiredPageAtEpoch,
                completion,
            });
        }
        for retired in self
            .target
            .retired_pages
            .iter_mut()
            .chain(self.draft.retired_pages.iter_mut())
            .filter(|retired| !retired.quiescent && retired.after_epoch == exact_epoch)
        {
            retired.quiescent = true;
        }
        Ok((matching, completion))
    }

    fn validate_request(&self, request: RequestId) -> Result<(), DeviceKvCacheError> {
        if self.request != request {
            return Err(DeviceKvCacheError::WrongRequest);
        }
        Ok(())
    }

    fn read_binding(
        &self,
        request: RequestId,
        role: Qwen3ModelRole,
        logical_position: u32,
    ) -> Result<DeviceKvReadBinding, DeviceKvCacheError> {
        self.validate_request(request)?;
        let cache = self.role(role);
        let location = map_initialized_token(
            &cache.physical,
            request,
            cache.selection(),
            logical_position,
        )?;
        let logical_page = logical_position / M1_KV_PAGE_TOKENS;
        let Some(lease) = cache.active_pages.get(logical_page as usize) else {
            return Err(DeviceKvCacheError::OwnedPageTableDrift);
        };
        if lease.page != location.page {
            return Err(DeviceKvCacheError::OwnedPageTableDrift);
        }
        Ok(DeviceKvReadBinding {
            device: self.device,
            allocation_id: lease.allocation_id,
            request,
            selection: cache.selection(),
            logical_position,
            location,
        })
    }
}

/// Active target/draft KV custody for one exact request generation.
#[derive(Debug, PartialEq, Eq)]
pub struct ActiveDeviceKvCache {
    common: DeviceKvCacheCommon,
}

impl ActiveDeviceKvCache {
    /// Creates empty isolated target and draft page tables.
    ///
    /// # Errors
    ///
    /// Rejects zero/stale requests, invalid selections, swapped roles, or
    /// mismatched target/draft mode and bucket pairs.
    pub fn new(
        device: Gfx942DeviceBinding,
        request: RequestId,
        target_selection: Qwen3PlanSelection,
        draft_selection: Qwen3PlanSelection,
    ) -> Result<Self, DeviceKvCacheError> {
        if target_selection.role != Qwen3ModelRole::Target8B
            || draft_selection.role != Qwen3ModelRole::Draft06B
            || target_selection.mode != draft_selection.mode
            || target_selection.bucket != draft_selection.bucket
        {
            return Err(DeviceKvCacheError::PlanPairMismatch);
        }
        let target = RoleDeviceKvCache::new(request, target_selection)?;
        let draft = RoleDeviceKvCache::new(request, draft_selection)?;
        Ok(Self {
            common: DeviceKvCacheCommon {
                device,
                request,
                target,
                draft,
                target_qualification_reserve: None,
            },
        })
    }

    #[must_use]
    pub fn projection(&self) -> DeviceKvCacheProjection {
        self.common.projection()
    }

    pub(crate) fn release_state_is_valid(&self) -> bool {
        self.common.release_state_is_valid()
    }

    pub(crate) fn retired_pages(&self, role: Qwen3ModelRole) -> &[RetiredPageLease] {
        self.common.retired_pages(role)
    }

    pub(crate) fn take_retired_pages(&mut self, role: Qwen3ModelRole) -> Vec<RetiredPageLease> {
        self.common.take_retired_pages(role)
    }

    /// Borrows qualification-only future target-page custody, when installed.
    #[must_use]
    pub const fn qualification_target_page_reserve(
        &self,
    ) -> Option<&M1QualificationTargetPageReserveV1> {
        self.common.target_qualification_reserve.as_ref()
    }

    /// Reserves the exact one-token target write for one C8192 qualification step.
    ///
    /// The validated witness must match the reserve's exact grouping, policy,
    /// workload declaration, and ordered lane identity. Its ordinal must equal
    /// the cache's committed/resident token count. Exactly at ordinals divisible
    /// by 16, this method pops physical page `ordinal / 16`; every other ordinal
    /// supplies no new lease. A downstream reservation rejection reinserts the
    /// popped lease before returning.
    ///
    /// # Errors
    ///
    /// Rejects request, witness, decode shape, ordinal, reserve order,
    /// conservation, host custody, or ordinary step-reservation drift without
    /// losing any page lease or installing a pending marker.
    pub fn reserve_m1_qualification_context_step_write_v1(
        &mut self,
        request: RequestId,
        lane_ordinal: u32,
        context: crate::M1ValidatedQualificationContextStepV1,
        epoch: CompletionEpoch,
    ) -> Result<
        M1PendingQualificationContextStepWriteV1,
        M1QualificationContextStepReservationFailureV1,
    > {
        let reject = |error| Err(M1QualificationContextStepReservationFailureV1 { error });
        if self.common.request != request {
            return reject(DeviceKvCacheError::WrongRequest);
        }
        let Some(reserve) = self.common.target_qualification_reserve.as_ref() else {
            return reject(DeviceKvCacheError::QualificationReserveMissing);
        };
        if reserve.request != request
            || reserve.device != self.common.device
            || reserve.lane.lane_ordinal != lane_ordinal
            || context.lane().lane_ordinal != lane_ordinal
            || !reserve.matches_context(context)
        {
            return reject(DeviceKvCacheError::QualificationWitnessMismatch);
        }
        let expected_selection = Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Decode,
            bucket: qualification_decode_bucket(reserve.grouping),
        };
        if self.common.target.selection() != expected_selection {
            return reject(DeviceKvCacheError::StepSelectionMismatch);
        }
        let ordinal = context.ordinal();
        let consumed_before =
            usize::try_from(ordinal.div_ceil(M1_KV_PAGE_TOKENS)).unwrap_or(usize::MAX);
        let Some(expected_unused) =
            M1_QUALIFICATION_TARGET_PAGE_COUNT_V1.checked_sub(consumed_before)
        else {
            return reject(DeviceKvCacheError::QualificationPageOrderMismatch);
        };
        if !reserve.ordered_state_is_valid()
            || reserve.unused_pages.len() != expected_unused
            || self.common.target.active_pages.len() != consumed_before
        {
            return reject(DeviceKvCacheError::QualificationPageOrderMismatch);
        }

        let needs_page = ordinal.is_multiple_of(M1_KV_PAGE_TOKENS);
        let mut page_leases = Vec::new();
        if needs_page && page_leases.try_reserve_exact(1).is_err() {
            return reject(DeviceKvCacheError::QualificationHostCustodyAllocation);
        }
        if needs_page {
            let expected_page = ordinal / M1_KV_PAGE_TOKENS;
            let Some(reserve) = self.common.target_qualification_reserve.as_mut() else {
                return reject(DeviceKvCacheError::QualificationReserveMissing);
            };
            let Some(lease) = reserve.unused_pages.pop() else {
                return reject(DeviceKvCacheError::QualificationPageOrderMismatch);
            };
            if lease.page.index() != expected_page {
                reserve.unused_pages.push(lease);
                return reject(DeviceKvCacheError::QualificationPageOrderMismatch);
            }
            page_leases.push(lease);
        }

        let pending = match self.reserve_step_write(
            request,
            Qwen3ModelRole::Target8B,
            ordinal,
            1,
            epoch,
            page_leases,
        ) {
            Ok(pending) => pending,
            Err(failure) => {
                let (error, returned) = failure.into_parts();
                if !returned.is_empty() {
                    let Some(reserve) = self.common.target_qualification_reserve.as_mut() else {
                        return reject(DeviceKvCacheError::QualificationReserveMissing);
                    };
                    reserve.unused_pages.extend(returned);
                }
                return reject(error);
            }
        };
        let active_pages = self.common.target.active_pages.len();
        let unused_future_pages = self
            .common
            .target_qualification_reserve
            .as_ref()
            .map_or(0, M1QualificationTargetPageReserveV1::unused_page_count);
        Ok(M1PendingQualificationContextStepWriteV1 {
            context,
            pending,
            active_pages,
            unused_future_pages,
        })
    }

    /// Consumes one allocation lease into the exact role page table.
    ///
    /// Failure retains the unchanged linear lease for retry or teardown.
    ///
    /// # Errors
    ///
    /// Rejects device, request, role, allocation, generation, table, and
    /// pending-write drift without consuming the returned lease.
    pub fn append_page(
        &mut self,
        request: RequestId,
        lease: DeviceKvPageLease,
    ) -> Result<(), DeviceKvAppendFailure> {
        let error = if self.common.request != request || lease.request != request {
            Some(DeviceKvCacheError::WrongRequest)
        } else if self.common.device != lease.device {
            Some(DeviceKvCacheError::WrongDevice)
        } else if self
            .common
            .other_role_arena(lease.page.role())
            .is_some_and(|arena| arena.equals(&lease.allocation_id))
        {
            Some(DeviceKvCacheError::AllocationAlias)
        } else {
            let cache = self.common.role(lease.page.role());
            if cache.pending.is_some() {
                Some(DeviceKvCacheError::PendingWriteExists)
            } else if !DeviceKvCacheCommon::owned_table_matches(cache) {
                Some(DeviceKvCacheError::OwnedPageTableDrift)
            } else if cache
                .arena_allocation_id
                .is_some_and(|arena| !arena.equals(&lease.allocation_id))
            {
                Some(DeviceKvCacheError::ArenaAllocationMismatch)
            } else if lease.page.role() != cache.selection().role {
                Some(DeviceKvCacheError::WrongRole)
            } else {
                None
            }
        };
        if let Some(error) = error {
            return Err(DeviceKvAppendFailure { error, lease });
        }

        let role = lease.page.role();
        let page = lease.page;
        let cache = self.common.role_mut(role);
        let selection = cache.selection();
        if let Err(error) = append_physical_page(&mut cache.physical, request, selection, page) {
            return Err(DeviceKvAppendFailure {
                error: error.into(),
                lease,
            });
        }
        if cache.arena_allocation_id.is_none() {
            cache.arena_allocation_id = Some(lease.allocation_id);
        }
        cache.active_pages.push(lease);
        Ok(())
    }

    /// Reserves the exact next logical write without marking it initialized.
    ///
    /// # Errors
    ///
    /// Rejects stale requests, zero epochs, duplicate pending writes, missing
    /// owned pages, logical gaps, and write-generation exhaustion.
    pub fn prepare_write(
        &mut self,
        request: RequestId,
        role: Qwen3ModelRole,
        logical_position: u32,
        epoch: CompletionEpoch,
    ) -> Result<PendingDeviceKvWrite, DeviceKvCacheError> {
        self.common.validate_request(request)?;
        if epoch.value() == 0 {
            return Err(DeviceKvCacheError::ZeroCompletionEpoch);
        }
        let device = self.common.device;
        let cache = self.common.role_mut(role);
        if cache.pending.is_some() {
            return Err(DeviceKvCacheError::PendingWriteExists);
        }
        if !DeviceKvCacheCommon::owned_table_matches(cache) {
            return Err(DeviceKvCacheError::OwnedPageTableDrift);
        }
        if logical_position != cache.logical().resident_tokens {
            return Err(DeviceKvCacheError::Physical(
                PhysicalKvError::LogicalPositionMismatch,
            ));
        }
        let logical_page = logical_position / M1_KV_PAGE_TOKENS;
        let Some(lease) = cache.active_pages.get(logical_page as usize) else {
            return Err(DeviceKvCacheError::Physical(PhysicalKvError::MissingPage));
        };
        if lease.page.role() != role {
            return Err(DeviceKvCacheError::OwnedPageTableDrift);
        }
        let write_generation = cache.next_write_generation;
        let Some(next) = write_generation.checked_add(1) else {
            return Err(DeviceKvCacheError::WriteGenerationExhausted);
        };
        let binding = PendingWriteBinding {
            device,
            allocation_id: lease.allocation_id,
            request,
            selection: cache.selection(),
            page: lease.page,
            logical_position,
            epoch,
            write_generation,
        };
        cache.next_write_generation = next;
        cache.pending = Some(PendingWriteState::Token(binding));
        Ok(PendingDeviceKvWrite { binding })
    }

    /// Reserves all physical page identities for one exact next step interval.
    ///
    /// `new_page_leases` must contain exactly the missing logical tail pages in
    /// order. Existing tail-page custody remains in the cache; missing leases
    /// move into the returned reservation. Every rejection returns the input
    /// leases unchanged, and no rejection installs a pending marker.
    ///
    /// This is a structural prerequisite only. Success neither appends the new
    /// pages to [`PhysicalKvState`] nor initializes any device bytes.
    ///
    /// # Errors
    ///
    /// Rejects stale request authority, zero or selection-incompatible active
    /// lengths, context overflow, committed/resident drift, another pending
    /// write, missing or excess leases, device/role/allocation drift, aliases,
    /// stale physical generations, and reservation-generation exhaustion.
    pub fn reserve_step_write(
        &mut self,
        request: RequestId,
        role: Qwen3ModelRole,
        committed_tokens: u32,
        active_tokens: u32,
        epoch: CompletionEpoch,
        new_page_leases: Vec<DeviceKvPageLease>,
    ) -> Result<PendingDeviceKvStepWrite, Box<DeviceKvStepReservationFailure>> {
        let reject = |error, page_leases| {
            Err(Box::new(DeviceKvStepReservationFailure {
                error,
                page_leases,
            }))
        };

        if let Err(error) = self.common.validate_request(request) {
            return reject(error, new_page_leases);
        }
        if epoch.value() == 0 {
            return reject(DeviceKvCacheError::ZeroCompletionEpoch, new_page_leases);
        }

        let device = self.common.device;
        let other_role_arena = self.common.other_role_arena(role);
        let cache = self.common.role(role);
        if cache.pending.is_some() {
            return reject(DeviceKvCacheError::PendingWriteExists, new_page_leases);
        }
        if !DeviceKvCacheCommon::owned_table_matches(cache) {
            return reject(DeviceKvCacheError::OwnedPageTableDrift, new_page_leases);
        }

        let selection = cache.selection();
        let Some(dimensions) = selection.bucket.dimensions(role, selection.mode) else {
            return reject(
                DeviceKvCacheError::Physical(PhysicalKvError::InvalidSelection),
                new_page_leases,
            );
        };
        if active_tokens == 0 {
            return reject(DeviceKvCacheError::ZeroStepActiveTokens, new_page_leases);
        }
        let active_matches = match selection.mode {
            Qwen3ExecutionMode::Prefill => active_tokens <= dimensions.active_tokens,
            Qwen3ExecutionMode::Decode | Qwen3ExecutionMode::Speculative => {
                active_tokens == dimensions.active_tokens
            }
        };
        if !active_matches {
            return reject(
                DeviceKvCacheError::StepActiveLengthMismatch,
                new_page_leases,
            );
        }

        let logical = cache.logical();
        if selection.mode == Qwen3ExecutionMode::Prefill && committed_tokens != 0 {
            return reject(
                DeviceKvCacheError::StepCommittedPositionMismatch,
                new_page_leases,
            );
        }
        if logical.committed_tokens != committed_tokens {
            return reject(
                DeviceKvCacheError::StepCommittedPositionMismatch,
                new_page_leases,
            );
        }
        if logical.resident_tokens != committed_tokens {
            return reject(
                DeviceKvCacheError::StepTentativeTokensRemain,
                new_page_leases,
            );
        }
        let Some(end_tokens) = committed_tokens.checked_add(active_tokens) else {
            return reject(DeviceKvCacheError::StepRangeOverflow, new_page_leases);
        };
        if end_tokens > dimensions.context_tokens {
            return reject(
                DeviceKvCacheError::Physical(PhysicalKvError::ContextExceeded),
                new_page_leases,
            );
        }

        let required_page_count = ((end_tokens - 1) / M1_KV_PAGE_TOKENS + 1) as usize;
        let existing_page_count = cache.active_pages.len();
        let Some(missing_page_count) = required_page_count.checked_sub(existing_page_count) else {
            return reject(DeviceKvCacheError::OwnedPageTableDrift, new_page_leases);
        };
        if new_page_leases.len() != missing_page_count {
            return reject(
                DeviceKvCacheError::StepPageLeaseCountMismatch,
                new_page_leases,
            );
        }

        let expected_arena = cache.arena_allocation_id.or_else(|| {
            new_page_leases
                .first()
                .map(DeviceKvPageLease::allocation_id)
        });
        for (position, lease) in new_page_leases.iter().enumerate() {
            if lease.request != request {
                return reject(DeviceKvCacheError::WrongRequest, new_page_leases);
            }
            if lease.device != device {
                return reject(DeviceKvCacheError::WrongDevice, new_page_leases);
            }
            if lease.page.role() != role {
                return reject(DeviceKvCacheError::WrongRole, new_page_leases);
            }
            if expected_arena.is_none_or(|arena| !arena.equals(&lease.allocation_id)) {
                return reject(DeviceKvCacheError::ArenaAllocationMismatch, new_page_leases);
            }
            if other_role_arena.is_some_and(|arena| arena.equals(&lease.allocation_id)) {
                return reject(DeviceKvCacheError::AllocationAlias, new_page_leases);
            }
            if lease.page.generation() == 0
                || cache.physical.page_generation(lease.page.index())
                    != Some(lease.page.generation())
            {
                return reject(
                    DeviceKvCacheError::Physical(PhysicalKvError::PageGenerationMismatch),
                    new_page_leases,
                );
            }
            if cache
                .active_pages
                .iter()
                .any(|owned| owned.page.index() == lease.page.index())
                || cache
                    .retired_pages
                    .iter()
                    .any(|retired| retired.lease.page.index() == lease.page.index())
                || new_page_leases[..position]
                    .iter()
                    .any(|prior| prior.page.index() == lease.page.index())
            {
                return reject(DeviceKvCacheError::StepPhysicalAlias, new_page_leases);
            }
        }

        let write_generation = cache.next_write_generation;
        let Some(next_write_generation) = write_generation.checked_add(1) else {
            return reject(
                DeviceKvCacheError::WriteGenerationExhausted,
                new_page_leases,
            );
        };
        let mut page_table = Vec::with_capacity(required_page_count);
        for logical_page in 0..required_page_count {
            let Ok(logical_page_u32) = u32::try_from(logical_page) else {
                return reject(DeviceKvCacheError::StepRangeOverflow, new_page_leases);
            };
            let lease = if logical_page < existing_page_count {
                &cache.active_pages[logical_page]
            } else {
                &new_page_leases[logical_page - existing_page_count]
            };
            page_table.push(DeviceKvStepPageIdentity {
                logical_page: logical_page_u32,
                allocation_id: lease.allocation_id,
                page: lease.page,
            });
        }

        let first_logical_page = committed_tokens / M1_KV_PAGE_TOKENS;
        let last_logical_page = (end_tokens - 1) / M1_KV_PAGE_TOKENS;
        let mut write_pages = Vec::new();
        for logical_page in first_logical_page..=last_logical_page {
            let logical_page_start = logical_page * M1_KV_PAGE_TOKENS;
            let span_start = committed_tokens.max(logical_page_start);
            let span_end = end_tokens.min(logical_page_start + M1_KV_PAGE_TOKENS);
            let lease = if (logical_page as usize) < existing_page_count {
                &cache.active_pages[logical_page as usize]
            } else {
                &new_page_leases[logical_page as usize - existing_page_count]
            };
            write_pages.push(DeviceKvStepPageBinding {
                identity: DeviceKvStepPageIdentity {
                    logical_page,
                    allocation_id: lease.allocation_id,
                    page: lease.page,
                },
                first_offset: span_start - logical_page_start,
                token_count: span_end - span_start,
            });
        }
        let binding = PendingStepWriteBinding {
            device,
            request,
            selection,
            committed_tokens,
            active_tokens,
            end_tokens,
            epoch,
            write_generation,
        };
        let cache = self.common.role_mut(role);
        cache.next_write_generation = next_write_generation;
        cache.pending = Some(PendingWriteState::Step(binding));
        Ok(PendingDeviceKvStepWrite {
            binding,
            page_table: page_table.into_boxed_slice(),
            write_pages: write_pages.into_boxed_slice(),
            new_page_leases,
        })
    }

    /// Reserves one full speculative round's K sequential draft-token writes.
    ///
    /// `target_speculative_selection` must be this cache's exact target K4, K8,
    /// or K16 selection. `draft_decode_selection` must be the paired `Draft06B`
    /// decode workspace shape: S1 for target S1 and S8 for target S8. The
    /// aggregate width is derived exclusively from the target bucket; callers
    /// cannot supply or override K. Success installs one draft pending marker
    /// for `[committed_tokens, committed_tokens + K)` and retains the one-token
    /// workspace selection beside that linear reservation.
    ///
    /// # Errors
    ///
    /// Rejects target/draft selection drift before mutation and returns every
    /// supplied page lease unchanged. Exact cache, interval, lease, and epoch
    /// validation then uses [`Self::reserve_step_write`].
    pub fn reserve_speculative_draft_round_write(
        &mut self,
        request: RequestId,
        target_speculative_selection: Qwen3PlanSelection,
        draft_decode_selection: Qwen3PlanSelection,
        committed_tokens: u32,
        epoch: CompletionEpoch,
        new_page_leases: Vec<DeviceKvPageLease>,
    ) -> Result<PendingSpeculativeDraftKvRoundWrite, Box<DeviceKvStepReservationFailure>> {
        let reject = |page_leases| {
            Err(Box::new(DeviceKvStepReservationFailure {
                error: DeviceKvCacheError::StepSelectionMismatch,
                page_leases,
            }))
        };
        let Some((draft_speculative_selection, expected_draft_decode, draft_tokens)) =
            m1_speculative_draft_round_shape_v1(target_speculative_selection)
        else {
            return reject(new_page_leases);
        };
        if self.common.target.selection() != target_speculative_selection
            || self.common.draft.selection() != draft_speculative_selection
            || draft_decode_selection != expected_draft_decode
        {
            return reject(new_page_leases);
        }

        let pending = self.reserve_step_write(
            request,
            Qwen3ModelRole::Draft06B,
            committed_tokens,
            draft_tokens,
            epoch,
            new_page_leases,
        )?;
        Ok(PendingSpeculativeDraftKvRoundWrite {
            target_speculative_selection,
            draft_decode_selection,
            draft_tokens,
            pending,
        })
    }

    /// Aborts an exact pending step reservation and recovers all new leases.
    ///
    /// Failure returns the unchanged reservation. Success clears only the
    /// matching pending marker and does not alter logical KV state or existing
    /// page custody.
    ///
    /// # Errors
    ///
    /// Rejects a reservation issued by another cache or any pending-marker or
    /// existing owned-page-table drift, retaining the reservation for retry.
    pub fn abort_step_write(
        &mut self,
        pending: PendingDeviceKvStepWrite,
    ) -> Result<AbortedDeviceKvStepWrite, Box<DeviceKvStepAbortFailure>> {
        let role = pending.binding.selection.role;
        let error = if pending.binding.device != self.common.device {
            Some(DeviceKvCacheError::WrongDevice)
        } else if pending.binding.request != self.common.request {
            Some(DeviceKvCacheError::WrongRequest)
        } else {
            let cache = self.common.role(role);
            if cache.pending != Some(PendingWriteState::Step(pending.binding)) {
                Some(DeviceKvCacheError::PendingWriteMismatch)
            } else if !DeviceKvCacheCommon::owned_table_matches(cache) {
                Some(DeviceKvCacheError::OwnedPageTableDrift)
            } else {
                None
            }
        };
        if let Some(error) = error {
            return Err(Box::new(DeviceKvStepAbortFailure { error, pending }));
        }

        self.common.role_mut(role).pending = None;
        Ok(AbortedDeviceKvStepWrite {
            page_leases: pending.new_page_leases,
        })
    }

    pub(crate) fn preflight_step_completion(
        &self,
        pending: &PendingDeviceKvStepWrite,
        completion: &ExactCompletion,
    ) -> Result<(), DeviceKvCacheError> {
        let binding = pending.binding;
        if binding.device != self.common.device {
            return Err(DeviceKvCacheError::WrongDevice);
        }
        if binding.request != self.common.request {
            return Err(DeviceKvCacheError::WrongRequest);
        }
        if completion.epoch() != binding.epoch {
            return Err(DeviceKvCacheError::CompletionEpochMismatch);
        }

        let role = binding.selection.role;
        let cache = self.common.role(role);
        if cache.selection() != binding.selection {
            return Err(DeviceKvCacheError::StepSelectionMismatch);
        }
        match cache.pending {
            None => return Err(DeviceKvCacheError::NoPendingWrite),
            Some(PendingWriteState::Step(marker)) if marker == binding => {}
            Some(_) => return Err(DeviceKvCacheError::PendingWriteMismatch),
        }
        if binding
            .write_generation
            .checked_add(1)
            .is_none_or(|next| next != cache.next_write_generation)
        {
            return Err(DeviceKvCacheError::PendingWriteMismatch);
        }
        if !DeviceKvCacheCommon::owned_table_matches(cache) {
            return Err(DeviceKvCacheError::OwnedPageTableDrift);
        }

        let Some(dimensions) = binding
            .selection
            .bucket
            .dimensions(role, binding.selection.mode)
        else {
            return Err(DeviceKvCacheError::StepSelectionMismatch);
        };
        if binding.active_tokens == 0 {
            return Err(DeviceKvCacheError::ZeroStepActiveTokens);
        }
        let active_matches = match binding.selection.mode {
            Qwen3ExecutionMode::Prefill => binding.active_tokens <= dimensions.active_tokens,
            Qwen3ExecutionMode::Decode | Qwen3ExecutionMode::Speculative => {
                binding.active_tokens == dimensions.active_tokens
            }
        };
        if !active_matches {
            return Err(DeviceKvCacheError::StepActiveLengthMismatch);
        }
        if binding.selection.mode == Qwen3ExecutionMode::Prefill && binding.committed_tokens != 0 {
            return Err(DeviceKvCacheError::StepCommittedPositionMismatch);
        }

        let logical = cache.logical();
        if logical.lifecycle != PhysicalKvLifecycle::Active {
            return Err(DeviceKvCacheError::Physical(
                PhysicalKvError::WrongLifecycle,
            ));
        }
        if logical.committed_tokens != binding.committed_tokens {
            return Err(DeviceKvCacheError::StepCommittedPositionMismatch);
        }
        if logical.resident_tokens != binding.committed_tokens {
            return Err(DeviceKvCacheError::StepTentativeTokensRemain);
        }
        let Some(end_tokens) = binding.committed_tokens.checked_add(binding.active_tokens) else {
            return Err(DeviceKvCacheError::StepRangeOverflow);
        };
        if end_tokens != binding.end_tokens {
            return Err(DeviceKvCacheError::StepWriteSpanDrift);
        }
        if end_tokens > dimensions.context_tokens {
            return Err(DeviceKvCacheError::Physical(
                PhysicalKvError::ContextExceeded,
            ));
        }

        let required_page_count = usize::try_from((end_tokens - 1) / M1_KV_PAGE_TOKENS + 1)
            .map_err(|_| DeviceKvCacheError::StepRangeOverflow)?;
        if pending.page_table.len() != required_page_count {
            return Err(DeviceKvCacheError::StepPageTableDrift);
        }
        let existing_page_count = cache.active_pages.len();
        let Some(missing_page_count) = required_page_count.checked_sub(existing_page_count) else {
            return Err(DeviceKvCacheError::OwnedPageTableDrift);
        };
        if pending.new_page_leases.len() != missing_page_count {
            return Err(DeviceKvCacheError::StepPageTableDrift);
        }

        let expected_arena = cache.arena_allocation_id.or_else(|| {
            pending
                .new_page_leases
                .first()
                .map(DeviceKvPageLease::allocation_id)
        });
        let Some(expected_arena) = expected_arena else {
            return Err(DeviceKvCacheError::StepPageTableDrift);
        };
        if self
            .common
            .other_role_arena(role)
            .is_some_and(|arena| arena.equals(&expected_arena))
        {
            return Err(DeviceKvCacheError::AllocationAlias);
        }

        for (position, identity) in pending.page_table.iter().enumerate() {
            let logical_page =
                u32::try_from(position).map_err(|_| DeviceKvCacheError::StepRangeOverflow)?;
            let expected_lease = if position < existing_page_count {
                &cache.active_pages[position]
            } else {
                &pending.new_page_leases[position - existing_page_count]
            };
            if identity.logical_page != logical_page
                || identity.page != expected_lease.page
                || !identity.allocation_id.equals(&expected_lease.allocation_id)
            {
                return Err(DeviceKvCacheError::StepPageTableDrift);
            }
            if expected_lease.request != binding.request {
                return Err(DeviceKvCacheError::WrongRequest);
            }
            if expected_lease.device != binding.device {
                return Err(DeviceKvCacheError::WrongDevice);
            }
            if identity.page.role() != role {
                return Err(DeviceKvCacheError::WrongRole);
            }
            if !identity.allocation_id.equals(&expected_arena) {
                return Err(DeviceKvCacheError::ArenaAllocationMismatch);
            }
            if identity.page.generation() == 0
                || cache.physical.page_generation(identity.page.index())
                    != Some(identity.page.generation())
            {
                return Err(DeviceKvCacheError::Physical(
                    PhysicalKvError::PageGenerationMismatch,
                ));
            }
            if pending.page_table[..position]
                .iter()
                .any(|prior| prior.page.index() == identity.page.index())
            {
                return Err(DeviceKvCacheError::StepPhysicalAlias);
            }
        }

        let first_logical_page = binding.committed_tokens / M1_KV_PAGE_TOKENS;
        let last_logical_page = (binding.end_tokens - 1) / M1_KV_PAGE_TOKENS;
        let expected_write_page_count = usize::try_from(last_logical_page - first_logical_page + 1)
            .map_err(|_| DeviceKvCacheError::StepRangeOverflow)?;
        if pending.write_pages.len() != expected_write_page_count {
            return Err(DeviceKvCacheError::StepWriteSpanDrift);
        }
        for (position, write) in pending.write_pages.iter().enumerate() {
            let position =
                u32::try_from(position).map_err(|_| DeviceKvCacheError::StepRangeOverflow)?;
            let logical_page = first_logical_page
                .checked_add(position)
                .ok_or(DeviceKvCacheError::StepRangeOverflow)?;
            let logical_page_start = logical_page
                .checked_mul(M1_KV_PAGE_TOKENS)
                .ok_or(DeviceKvCacheError::StepRangeOverflow)?;
            let span_start = binding.committed_tokens.max(logical_page_start);
            let span_end = binding
                .end_tokens
                .min(logical_page_start + M1_KV_PAGE_TOKENS);
            let table_identity = pending
                .page_table
                .get(logical_page as usize)
                .ok_or(DeviceKvCacheError::StepPageTableDrift)?;
            if write.identity != *table_identity
                || write.identity.logical_page != logical_page
                || write.first_offset != span_start - logical_page_start
                || write.token_count != span_end - span_start
                || write.token_count == 0
            {
                return Err(DeviceKvCacheError::StepWriteSpanDrift);
            }
        }
        Ok(())
    }

    pub(crate) fn preflight_step_settlement(
        &self,
        pending: &PendingDeviceKvStepWrite,
        accepted_tokens: u32,
        after_epoch: CompletionEpoch,
    ) -> Result<(), DeviceKvCacheError> {
        if after_epoch.value() == 0 || pending.epoch() != after_epoch {
            return Err(DeviceKvCacheError::CompletionEpochMismatch);
        }
        if accepted_tokens > pending.active_tokens() {
            return Err(DeviceKvCacheError::Physical(
                PhysicalKvError::CommitExceedsResident,
            ));
        }
        let rejected_tokens = pending.active_tokens() - accepted_tokens;
        if rejected_tokens > M1_KV_PAGE_TOKENS {
            return Err(DeviceKvCacheError::Physical(
                PhysicalKvError::SettlementTailTooWide,
            ));
        }
        Ok(())
    }

    pub(crate) fn settle_completed_step(
        &mut self,
        initialized: &InertInitializedDeviceKvStepWrite,
        accepted_tokens: u32,
        after_epoch: CompletionEpoch,
    ) -> Result<u32, DeviceKvCacheError> {
        if initialized.epoch() != after_epoch {
            return Err(DeviceKvCacheError::CompletionEpochMismatch);
        }
        let request = initialized.request();
        let role = initialized.selection().role;
        self.accept_initialized(request, role, accepted_tokens)?;
        let rejected_tokens = initialized
            .active_tokens()
            .checked_sub(accepted_tokens)
            .ok_or(DeviceKvCacheError::Physical(
                PhysicalKvError::CommitExceedsResident,
            ))?;
        let mut retired_pages = 0u32;
        for _ in 0..rejected_tokens {
            if matches!(
                self.rollback_one(request, role, after_epoch)?,
                DeviceKvRetirementOutcome::PageRetired(_)
            ) {
                retired_pages = retired_pages
                    .checked_add(1)
                    .ok_or(DeviceKvCacheError::OwnedPageTableDrift)?;
            }
        }
        Ok(retired_pages)
    }

    pub(crate) fn preflight_retirement_after_step(
        &self,
        request: RequestId,
        after_epoch: CompletionEpoch,
    ) -> Result<(), DeviceKvCacheError> {
        self.common.validate_request(request)?;
        if after_epoch.value() == 0 {
            return Err(DeviceKvCacheError::ZeroCompletionEpoch);
        }
        for cache in [&self.common.target, &self.common.draft] {
            if !matches!(cache.logical().lifecycle, PhysicalKvLifecycle::Active) {
                return Err(DeviceKvCacheError::Physical(
                    PhysicalKvError::WrongLifecycle,
                ));
            }
            if !DeviceKvCacheCommon::owned_table_matches(cache) {
                return Err(DeviceKvCacheError::OwnedPageTableDrift);
            }
            if cache
                .retired_pages
                .iter()
                .any(|retired| !retired.quiescent && retired.after_epoch != after_epoch)
            {
                return Err(DeviceKvCacheError::UnsettledPriorRetirement);
            }
        }
        Ok(())
    }

    /// Joins one exact ordered-queue completion to a pending bulk KV write.
    ///
    /// Every cache, marker, epoch, selection, page-table, and write-span check
    /// completes before mutation. A rejection therefore returns the unchanged
    /// cache, reservation, and completion. Success appends all retained leases,
    /// initializes the complete reserved interval, clears its pending marker,
    /// and returns the same single [`ExactCompletion`] beside inert interval
    /// evidence. No acceptance or rollback decision is made here.
    ///
    /// A failure from an already-preflighted source-model transition is treated
    /// as an internal invariant violation. Its partially transitioned custody
    /// is terminally quarantined instead of pretending that rollback occurred.
    #[allow(dead_code)]
    pub(crate) fn complete_step_write(
        self,
        pending: PendingDeviceKvStepWrite,
        completion: ExactCompletion,
    ) -> DeviceKvStepCompletionOutcome {
        if let Err(error) = self.preflight_step_completion(&pending, &completion) {
            return DeviceKvStepCompletionOutcome::Rejected(DeviceKvStepCompletionFailure {
                error,
                cache: self,
                pending,
                completion,
            });
        }

        let PendingDeviceKvStepWrite {
            binding,
            page_table,
            write_pages,
            new_page_leases,
        } = pending;
        let role = binding.selection.role;
        let mut common = self.common;
        let mut new_page_leases = new_page_leases.into_iter();
        for logical_position in binding.committed_tokens..binding.end_tokens {
            let logical_page = logical_position / M1_KV_PAGE_TOKENS;
            if common.role(role).physical.page_count() <= logical_page {
                let Some(lease) = new_page_leases.next() else {
                    return DeviceKvStepCompletionOutcome::Poisoned(
                        PoisonedDeviceKvStepCompletion {
                            error: DeviceKvCacheError::StepPageTableDrift,
                            common,
                            binding,
                            page_table,
                            write_pages,
                            unappended_page_leases: Vec::new(),
                            completion,
                        },
                    );
                };
                let append_result = append_physical_page(
                    &mut common.role_mut(role).physical,
                    binding.request,
                    binding.selection,
                    lease.page,
                );
                if let Err(error) = append_result {
                    let mut unappended_page_leases = Vec::new();
                    unappended_page_leases.push(lease);
                    unappended_page_leases.extend(new_page_leases);
                    return DeviceKvStepCompletionOutcome::Poisoned(
                        PoisonedDeviceKvStepCompletion {
                            error: error.into(),
                            common,
                            binding,
                            page_table,
                            write_pages,
                            unappended_page_leases,
                            completion,
                        },
                    );
                }
                let cache = common.role_mut(role);
                if cache.arena_allocation_id.is_none() {
                    cache.arena_allocation_id = Some(lease.allocation_id);
                }
                cache.active_pages.push(lease);
            }

            let write_result = write_physical_token(
                &mut common.role_mut(role).physical,
                binding.request,
                binding.selection,
                logical_position,
            );
            if let Err(error) = write_result {
                let unappended_page_leases = new_page_leases.collect();
                return DeviceKvStepCompletionOutcome::Poisoned(PoisonedDeviceKvStepCompletion {
                    error: error.into(),
                    common,
                    binding,
                    page_table,
                    write_pages,
                    unappended_page_leases,
                    completion,
                });
            }
        }
        if let Some(lease) = new_page_leases.next() {
            let mut unappended_page_leases = Vec::new();
            unappended_page_leases.push(lease);
            unappended_page_leases.extend(new_page_leases);
            return DeviceKvStepCompletionOutcome::Poisoned(PoisonedDeviceKvStepCompletion {
                error: DeviceKvCacheError::StepPageTableDrift,
                common,
                binding,
                page_table,
                write_pages,
                unappended_page_leases,
                completion,
            });
        }
        common.role_mut(role).pending = None;

        DeviceKvStepCompletionOutcome::Completed(CompletedDeviceKvStepWrite {
            cache: ActiveDeviceKvCache { common },
            initialized: InertInitializedDeviceKvStepWrite {
                binding,
                page_table,
                write_pages,
            },
            completion,
        })
    }

    /// Applies one completed physical write to the verified initialized prefix.
    ///
    /// Failure retains the completion authority. No public source constructor
    /// can produce that authority.
    ///
    /// # Errors
    ///
    /// Rejects missing, stale, mismatched, wrong-epoch, or table-drifted write
    /// authority without advancing the initialized prefix.
    pub fn apply_initialized_write(
        &mut self,
        initialized: InitializedDeviceKvWrite,
    ) -> Result<DeviceKvReadBinding, Box<WriteApplicationFailure>> {
        let role = initialized.binding.selection.role;
        let error = {
            let cache = self.common.role(role);
            if cache.pending.is_none() {
                Some(DeviceKvCacheError::NoPendingWrite)
            } else if cache.pending != Some(PendingWriteState::Token(initialized.binding)) {
                Some(DeviceKvCacheError::PendingWriteMismatch)
            } else if initialized.completion.epoch().value() != initialized.binding.epoch.value() {
                Some(DeviceKvCacheError::CompletionEpochMismatch)
            } else {
                None
            }
        };
        if let Some(error) = error {
            return Err(Box::new(WriteApplicationFailure { error, initialized }));
        }

        let binding = initialized.binding;
        let device = self.common.device;
        let cache = self.common.role_mut(role);
        if !DeviceKvCacheCommon::owned_table_matches(cache) {
            return Err(Box::new(WriteApplicationFailure {
                error: DeviceKvCacheError::OwnedPageTableDrift,
                initialized,
            }));
        }
        let logical_page = binding.logical_position / M1_KV_PAGE_TOKENS;
        let offset = binding.logical_position % M1_KV_PAGE_TOKENS;
        let Some(lease) = cache.active_pages.get(logical_page as usize) else {
            return Err(Box::new(WriteApplicationFailure {
                error: DeviceKvCacheError::OwnedPageTableDrift,
                initialized,
            }));
        };
        if lease.page != binding.page
            || !lease.allocation_id.equals(&binding.allocation_id)
            || lease.device != binding.device
        {
            return Err(Box::new(WriteApplicationFailure {
                error: DeviceKvCacheError::PendingWriteMismatch,
                initialized,
            }));
        }
        let allocation_id = lease.allocation_id;
        if let Err(error) = write_physical_token(
            &mut cache.physical,
            binding.request,
            binding.selection,
            binding.logical_position,
        ) {
            return Err(Box::new(WriteApplicationFailure {
                error: error.into(),
                initialized,
            }));
        }
        cache.pending = None;
        Ok(DeviceKvReadBinding {
            device,
            allocation_id,
            request: binding.request,
            selection: binding.selection,
            logical_position: binding.logical_position,
            location: PhysicalKvLocation {
                page: binding.page,
                offset,
            },
        })
    }

    /// Resolves only an already initialized and still reachable logical token.
    ///
    /// # Errors
    ///
    /// Rejects stale request generations, uninitialized positions, cross-role
    /// mappings, and owned/model page-table drift.
    pub fn map_initialized(
        &self,
        request: RequestId,
        role: Qwen3ModelRole,
        logical_position: u32,
    ) -> Result<DeviceKvReadBinding, DeviceKvCacheError> {
        self.common.read_binding(request, role, logical_position)
    }

    /// Publishes exactly an initialized tentative prefix for one role.
    ///
    /// # Errors
    ///
    /// Rejects stale requests, a pending physical write, or a count beyond the
    /// initialized resident suffix.
    pub fn accept_initialized(
        &mut self,
        request: RequestId,
        role: Qwen3ModelRole,
        accepted_tokens: u32,
    ) -> Result<(), DeviceKvCacheError> {
        self.common.validate_request(request)?;
        let cache = self.common.role_mut(role);
        if cache.pending.is_some() {
            return Err(DeviceKvCacheError::PendingWriteExists);
        }
        if !DeviceKvCacheCommon::owned_table_matches(cache) {
            return Err(DeviceKvCacheError::OwnedPageTableDrift);
        }
        let selection = cache.selection();
        commit_physical_kv(&mut cache.physical, request, selection, accepted_tokens)?;
        Ok(())
    }

    /// Rolls back exactly one initialized but uncommitted suffix token.
    ///
    /// A page that becomes unreachable moves from the active owned table to
    /// retirement custody at `after_epoch`; it is never returned for reuse.
    ///
    /// # Errors
    ///
    /// Rejects stale requests, zero epochs, pending writes, committed rollback,
    /// stale physical generations, or owned/model table drift.
    pub fn rollback_one(
        &mut self,
        request: RequestId,
        role: Qwen3ModelRole,
        after_epoch: CompletionEpoch,
    ) -> Result<DeviceKvRetirementOutcome, DeviceKvCacheError> {
        self.common.validate_request(request)?;
        if after_epoch.value() == 0 {
            return Err(DeviceKvCacheError::ZeroCompletionEpoch);
        }
        if self
            .common
            .target_qualification_reserve
            .as_ref()
            .is_some_and(|reserve| !reserve.unused_pages.is_empty())
        {
            return Err(DeviceKvCacheError::QualificationFuturePagesRemain);
        }
        let cache = self.common.role_mut(role);
        if cache.pending.is_some() {
            return Err(DeviceKvCacheError::PendingWriteExists);
        }
        if !DeviceKvCacheCommon::owned_table_matches(cache) {
            return Err(DeviceKvCacheError::OwnedPageTableDrift);
        }
        let before_pages = cache.physical.page_count();
        let Some(lease) = cache.active_pages.pop() else {
            return Err(DeviceKvCacheError::OwnedPageTableDrift);
        };
        let selection = cache.selection();
        if let Err(error) =
            rollback_physical_token(&mut cache.physical, request, selection, after_epoch)
        {
            cache.active_pages.push(lease);
            return Err(error.into());
        }
        if cache.physical.page_count() == before_pages {
            cache.active_pages.push(lease);
            return Ok(DeviceKvRetirementOutcome::TokenRemoved);
        }
        let page = lease.page;
        cache.retired_pages.push(RetiredPageLease {
            lease,
            after_epoch,
            quiescent: false,
        });
        Ok(DeviceKvRetirementOutcome::PageRetired(page))
    }

    /// Settles all rollback-retired leases for one exact completed epoch.
    ///
    /// This changes only engine retirement custody; it does not release or
    /// reuse a page and does not construct physical allocation authority.
    ///
    /// # Errors
    ///
    /// Returns the unchanged completion when no unsettled retired page names
    /// its exact epoch.
    pub fn settle_retired_epoch(
        &mut self,
        completion: ExactCompletion,
    ) -> Result<(usize, ExactCompletion), RetirementCompletionFailure> {
        self.common.settle_retired_epoch(completion)
    }

    /// Cancels both role projections and transfers the cache into retirement.
    pub fn cancel(
        mut self,
        request: RequestId,
        after_epoch: CompletionEpoch,
    ) -> DeviceKvCancellationOutcome {
        let error = if self.common.request != request {
            Some(DeviceKvCacheError::WrongRequest)
        } else if after_epoch.value() == 0 {
            Some(DeviceKvCacheError::ZeroCompletionEpoch)
        } else if self
            .common
            .target_qualification_reserve
            .as_ref()
            .is_some_and(|reserve| !reserve.unused_pages.is_empty())
        {
            Some(DeviceKvCacheError::QualificationFuturePagesRemain)
        } else if self.common.target.pending.is_some() || self.common.draft.pending.is_some() {
            Some(DeviceKvCacheError::PendingWriteExists)
        } else if !matches!(
            self.common.target.logical().lifecycle,
            PhysicalKvLifecycle::Active
        ) || !matches!(
            self.common.draft.logical().lifecycle,
            PhysicalKvLifecycle::Active
        ) {
            Some(DeviceKvCacheError::Physical(
                PhysicalKvError::WrongLifecycle,
            ))
        } else {
            None
        };
        if let Some(error) = error {
            return DeviceKvCancellationOutcome::Rejected(DeviceKvCancellationFailure {
                error,
                cache: self,
            });
        }

        let target_selection = self.common.target.selection();
        if let Err(error) = cancel_physical_kv(
            &mut self.common.target.physical,
            request,
            target_selection,
            after_epoch,
        ) {
            return DeviceKvCancellationOutcome::Rejected(DeviceKvCancellationFailure {
                error: error.into(),
                cache: self,
            });
        }
        let draft_selection = self.common.draft.selection();
        if let Err(error) = cancel_physical_kv(
            &mut self.common.draft.physical,
            request,
            draft_selection,
            after_epoch,
        ) {
            return DeviceKvCancellationOutcome::Poisoned(PoisonedDeviceKvCache {
                error: error.into(),
                common: self.common,
            });
        }
        DeviceKvCancellationOutcome::Cancelled(CancelledDeviceKvCache {
            common: self.common,
            after_epoch,
        })
    }
}

/// Retry-safe append rejection retaining the exact page lease.
#[derive(Debug, PartialEq, Eq)]
pub struct DeviceKvAppendFailure {
    error: DeviceKvCacheError,
    lease: DeviceKvPageLease,
}

impl DeviceKvAppendFailure {
    #[must_use]
    pub const fn error(&self) -> DeviceKvCacheError {
        self.error
    }

    #[must_use]
    pub fn into_parts(self) -> (DeviceKvCacheError, DeviceKvPageLease) {
        (self.error, self.lease)
    }
}

/// Retry-safe initialized-write rejection retaining its linear authority.
#[derive(Debug, PartialEq, Eq)]
pub struct WriteApplicationFailure {
    error: DeviceKvCacheError,
    initialized: InitializedDeviceKvWrite,
}

impl WriteApplicationFailure {
    #[must_use]
    pub const fn error(&self) -> DeviceKvCacheError {
        self.error
    }

    #[must_use]
    pub fn into_parts(self) -> (DeviceKvCacheError, InitializedDeviceKvWrite) {
        (self.error, self.initialized)
    }
}

/// Retry-safe exact-completion rejection for retired page custody.
#[derive(Debug, PartialEq, Eq)]
pub struct RetirementCompletionFailure {
    error: DeviceKvCacheError,
    completion: ExactCompletion,
}

impl RetirementCompletionFailure {
    #[must_use]
    pub const fn error(&self) -> DeviceKvCacheError {
        self.error
    }

    #[must_use]
    pub fn into_completion(self) -> ExactCompletion {
        self.completion
    }
}

/// Observable result of one rollback or cancelled-tail transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceKvRetirementOutcome {
    TokenRemoved,
    PageRetired(PhysicalPageId),
}

/// Exact outcome of an active-to-cancelled typestate transition.
#[must_use]
#[derive(Debug, PartialEq, Eq)]
pub enum DeviceKvCancellationOutcome {
    Cancelled(CancelledDeviceKvCache),
    Rejected(DeviceKvCancellationFailure),
    /// Retains all custody if an impossible second-role model transition fails.
    Poisoned(PoisonedDeviceKvCache),
}

/// Retry-safe cancellation rejection retaining the active cache.
#[derive(Debug, PartialEq, Eq)]
pub struct DeviceKvCancellationFailure {
    error: DeviceKvCacheError,
    cache: ActiveDeviceKvCache,
}

impl DeviceKvCancellationFailure {
    #[must_use]
    pub const fn error(&self) -> DeviceKvCacheError {
        self.error
    }

    #[must_use]
    pub fn into_parts(self) -> (DeviceKvCacheError, ActiveDeviceKvCache) {
        (self.error, self.cache)
    }
}

/// Terminal custody for an internal two-role composition failure.
///
/// No mutation, read, publication, release, or reuse methods are exposed.
#[derive(Debug, PartialEq, Eq)]
pub struct PoisonedDeviceKvCache {
    error: DeviceKvCacheError,
    common: DeviceKvCacheCommon,
}

impl PoisonedDeviceKvCache {
    #[must_use]
    pub const fn error(&self) -> DeviceKvCacheError {
        self.error
    }

    #[must_use]
    pub fn projection(&self) -> DeviceKvCacheProjection {
        self.common.projection()
    }
}

/// Cancelled cache whose active page tables must be retired tail-first.
#[derive(Debug, PartialEq, Eq)]
pub struct CancelledDeviceKvCache {
    common: DeviceKvCacheCommon,
    after_epoch: CompletionEpoch,
}

impl CancelledDeviceKvCache {
    #[must_use]
    pub fn projection(&self) -> DeviceKvCacheProjection {
        self.common.projection()
    }

    #[must_use]
    pub const fn after_epoch(&self) -> CompletionEpoch {
        self.after_epoch
    }

    /// Retires one role's current tail page while retaining its lease.
    ///
    /// # Errors
    ///
    /// Rejects stale requests and any model/owned-table drift.
    pub fn retire_next(
        &mut self,
        request: RequestId,
        role: Qwen3ModelRole,
    ) -> Result<Option<PhysicalPageId>, DeviceKvCacheError> {
        self.common.validate_request(request)?;
        let cache = self.common.role_mut(role);
        if cache.active_pages.is_empty() {
            return Ok(None);
        }
        if !DeviceKvCacheCommon::owned_table_matches(cache) {
            return Err(DeviceKvCacheError::OwnedPageTableDrift);
        }
        let Some(lease) = cache.active_pages.pop() else {
            return Err(DeviceKvCacheError::OwnedPageTableDrift);
        };
        let selection = cache.selection();
        let page = match retire_cancelled_tail(
            &mut cache.physical,
            request,
            selection,
            self.after_epoch,
        ) {
            Ok(page) => page,
            Err(error) => {
                cache.active_pages.push(lease);
                return Err(error.into());
            }
        };
        cache.retired_pages.push(RetiredPageLease {
            lease,
            after_epoch: self.after_epoch,
            quiescent: false,
        });
        Ok(Some(page))
    }

    /// Drains both bounded page tables into retained retirement custody.
    ///
    /// # Errors
    ///
    /// Rejects stale requests or exact model/owned-table drift.
    pub fn retire_all(&mut self, request: RequestId) -> Result<u32, DeviceKvCacheError> {
        self.common.validate_request(request)?;
        let mut retired = 0u32;
        for role in [Qwen3ModelRole::Target8B, Qwen3ModelRole::Draft06B] {
            while !self.common.role(role).active_pages.is_empty() {
                if self.retire_next(request, role)?.is_none() {
                    return Err(DeviceKvCacheError::OwnedPageTableDrift);
                }
                retired = retired
                    .checked_add(1)
                    .ok_or(DeviceKvCacheError::OwnedPageTableDrift)?;
            }
        }
        Ok(retired)
    }

    /// Settles rollback-retired leases whose exact epoch precedes cancellation.
    ///
    /// # Errors
    ///
    /// Returns the unchanged completion when no unsettled retired page names
    /// its exact epoch.
    pub fn settle_retired_epoch(
        &mut self,
        completion: ExactCompletion,
    ) -> Result<(usize, ExactCompletion), RetirementCompletionFailure> {
        self.common.settle_retired_epoch(completion)
    }

    /// Consumes exact completion and enters quiescent retirement custody.
    ///
    /// Success exposes no page release/reuse operation until the fe2o3 lease
    /// bridge exists. Failure retains both the cache and completion.
    ///
    /// # Errors
    ///
    /// Rejects a wrong completion epoch, reachable active pages, retirement
    /// epoch drift, or a model lifecycle that is not awaiting quiescence.
    pub fn quiesce(
        mut self,
        completion: ExactCompletion,
    ) -> Result<QuiescentDeviceKvCache, Box<QuiescenceFailure>> {
        let error = if completion.epoch().value() != self.after_epoch.value() {
            Some(DeviceKvCacheError::CompletionEpochMismatch)
        } else if !self.common.target.active_pages.is_empty()
            || !self.common.draft.active_pages.is_empty()
        {
            Some(DeviceKvCacheError::ActivePagesRemain)
        } else if self
            .common
            .target
            .retired_pages
            .iter()
            .chain(self.common.draft.retired_pages.iter())
            .any(|retired| !retired.quiescent && retired.after_epoch != self.after_epoch)
        {
            Some(DeviceKvCacheError::UnsettledPriorRetirement)
        } else if !matches!(
            self.common.target.logical().lifecycle,
            PhysicalKvLifecycle::RetiredAwaitingQuiescence { after_epoch }
                if after_epoch == self.after_epoch
        ) || !matches!(
            self.common.draft.logical().lifecycle,
            PhysicalKvLifecycle::RetiredAwaitingQuiescence { after_epoch }
                if after_epoch == self.after_epoch
        ) {
            Some(DeviceKvCacheError::OwnedPageTableDrift)
        } else {
            None
        };
        if let Some(error) = error {
            return Err(Box::new(QuiescenceFailure {
                error,
                cache: self,
                completion,
            }));
        }
        for retired in self
            .common
            .target
            .retired_pages
            .iter_mut()
            .chain(self.common.draft.retired_pages.iter_mut())
            .filter(|retired| retired.after_epoch == self.after_epoch)
        {
            retired.quiescent = true;
        }
        Ok(QuiescentDeviceKvCache {
            common: self.common,
            completion,
        })
    }
}

/// Retry-safe quiescence rejection retaining both authority inputs.
#[derive(Debug, PartialEq, Eq)]
pub struct QuiescenceFailure {
    error: DeviceKvCacheError,
    cache: CancelledDeviceKvCache,
    completion: ExactCompletion,
}

impl QuiescenceFailure {
    #[must_use]
    pub const fn error(&self) -> DeviceKvCacheError {
        self.error
    }

    #[must_use]
    pub fn into_parts(self) -> (CancelledDeviceKvCache, ExactCompletion) {
        (self.cache, self.completion)
    }
}

/// Quiescent terminal custody retaining every retired page lease.
///
/// There is intentionally no allocation-release, generation-advance, or reuse
/// operation. Adding one requires the missing fe2o3 physical lease authority.
#[derive(Debug, PartialEq, Eq)]
pub struct QuiescentDeviceKvCache {
    common: DeviceKvCacheCommon,
    completion: ExactCompletion,
}

impl QuiescentDeviceKvCache {
    #[must_use]
    pub fn projection(&self) -> DeviceKvCacheProjection {
        self.common.projection()
    }

    #[must_use]
    pub const fn completion_epoch(&self) -> CompletionEpoch {
        self.completion.epoch()
    }

    pub(crate) fn into_threaded_parts(self) -> (SettledQuiescentDeviceKvCache, ExactCompletion) {
        let completion_epoch = self.completion.epoch();
        (
            SettledQuiescentDeviceKvCache {
                common: self.common,
                completion_epoch,
            },
            self.completion,
        )
    }
}

/// Terminal quiescent device-KV custody after completion authority moves on.
#[must_use = "terminal device-KV custody must remain retained"]
#[derive(Debug, PartialEq, Eq)]
pub struct SettledQuiescentDeviceKvCache {
    common: DeviceKvCacheCommon,
    completion_epoch: CompletionEpoch,
}

impl SettledQuiescentDeviceKvCache {
    #[must_use]
    pub fn projection(&self) -> DeviceKvCacheProjection {
        self.common.projection()
    }

    #[must_use]
    pub const fn completion_epoch(&self) -> CompletionEpoch {
        self.completion_epoch
    }

    pub(crate) fn release_state_is_valid(&self) -> bool {
        self.common.release_state_is_valid()
            && self.common.target.active_pages.is_empty()
            && self.common.draft.active_pages.is_empty()
    }

    pub(crate) fn retired_pages(&self, role: Qwen3ModelRole) -> &[RetiredPageLease] {
        self.common.retired_pages(role)
    }

    pub(crate) fn take_retired_pages(&mut self, role: Qwen3ModelRole) -> Vec<RetiredPageLease> {
        self.common.take_retired_pages(role)
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::bind_gfx942_device;
    use super::*;
    use ferric_build::qwen3_kv_arena_bytes;
    use ferric_spec::{
        m1_qualification_context_plan, M1QualificationContextPlan,
        M1QualificationExecutionBindingDeclaration, M1QualificationLaneExecutionBinding,
        M1QualificationLaneGrouping, Qwen3ExecutionMode, Qwen3PlanBucket,
        M1_KV_PHYSICAL_PAGE_SLOTS,
    };

    impl DeviceKvPageLease {
        fn from_contracted_gfx942_allocation(
            device: Gfx942DeviceBinding,
            allocation_id: Identity,
            request: RequestId,
            page: PhysicalPageId,
        ) -> Result<Self, DeviceKvCacheError> {
            if !allocation_id.is_present() {
                return Err(DeviceKvCacheError::MissingAllocationIdentity);
            }
            if page.index() as usize >= M1_KV_PHYSICAL_PAGE_SLOTS {
                return Err(DeviceKvCacheError::Physical(
                    PhysicalKvError::PageOutOfRange,
                ));
            }
            if page.generation() == 0 {
                return Err(DeviceKvCacheError::Physical(
                    PhysicalKvError::PageGenerationMismatch,
                ));
            }
            Ok(Self {
                device,
                allocation_id,
                request,
                page,
            })
        }
    }

    impl PendingDeviceKvWrite {
        pub(crate) fn complete_for_test(
            self,
            completion: ExactCompletion,
        ) -> Result<InitializedDeviceKvWrite, Box<PendingWriteCompletionFailure>> {
            if completion.epoch().value() != self.binding.epoch.value() {
                return Err(Box::new(PendingWriteCompletionFailure {
                    error: DeviceKvCacheError::CompletionEpochMismatch,
                    pending: self,
                    completion,
                }));
            }
            Ok(InitializedDeviceKvWrite {
                binding: self.binding,
                completion,
            })
        }
    }

    fn identity(tag: u8) -> Identity {
        Identity::new([tag; 32])
    }

    fn device() -> Gfx942DeviceBinding {
        bind_gfx942_device(identity(1), 7, GFX942_PROCESSOR, GFX942_TARGET_FEATURES).unwrap()
    }

    const fn request() -> RequestId {
        RequestId::new(3, 7)
    }

    #[test]
    fn partition_geometry_covers_both_exact_role_arenas_without_gaps() {
        assert_eq!(
            M1_TARGET_KV_PLANE_SUBLEASES_V1,
            Qwen3ModelRole::Target8B.layers() as usize * 2
        );
        assert_eq!(
            M1_DRAFT_KV_PLANE_SUBLEASES_V1,
            Qwen3ModelRole::Draft06B.layers() as usize * 2
        );
        let plane_bytes =
            u64::try_from(M1_GLOBAL_KV_PAGE_SLOTS_V1).unwrap() * QWEN3_KV_PAGE_BYTES_V1;
        for role in [Qwen3ModelRole::Target8B, Qwen3ModelRole::Draft06B] {
            assert_eq!(
                plane_bytes * u64::from(role.layers()) * 2,
                qwen3_kv_arena_bytes(role)
            );
            assert!(plane_bytes.is_multiple_of(QWEN3_KV_ARENA_ALIGNMENT_V1));
        }
    }

    #[test]
    fn global_page_slots_are_request_partitioned_and_fail_closed() {
        assert_eq!(global_page_index(RequestId::new(0, 1), 0).unwrap(), 0);
        assert_eq!(
            global_page_index(RequestId::new(M1_MAX_ACTIVE_SEQUENCES - 1, 9), 511).unwrap(),
            M1_GLOBAL_KV_PAGE_SLOTS_V1 - 1
        );
        assert!(matches!(
            global_page_index(RequestId::new(0, 0), 0),
            Err(M1DeviceKvArenaLeaseErrorV1::RequestOutOfRange)
        ));
        assert!(matches!(
            global_page_index(RequestId::new(M1_MAX_ACTIVE_SEQUENCES, 1), 0),
            Err(M1DeviceKvArenaLeaseErrorV1::RequestOutOfRange)
        ));
        assert!(matches!(
            global_page_index(RequestId::new(0, 1), 512),
            Err(M1DeviceKvArenaLeaseErrorV1::PageOutOfRange)
        ));
    }

    #[test]
    fn page_generation_ledger_rejects_zero_and_duplicate_custody() {
        assert_eq!(
            free_page_generation(M1KvPoolPageStateV1::INITIAL).unwrap(),
            1
        );
        assert!(matches!(
            free_page_generation(M1KvPoolPageStateV1::Free { generation: 0 }),
            Err(M1DeviceKvArenaLeaseErrorV1::GenerationLedgerDrift)
        ));
        assert!(matches!(
            free_page_generation(M1KvPoolPageStateV1::Leased {
                request: RequestId::new(0, 1),
                generation: 1,
            }),
            Err(M1DeviceKvArenaLeaseErrorV1::PageAlreadyLeased)
        ));
    }

    #[test]
    fn retained_page_validation_rejects_free_stale_and_cross_request_ledger_entries() {
        let request = RequestId::new(3, 7);
        let leased = M1KvPoolPageStateV1::Leased {
            request,
            generation: 11,
        };
        assert!(validate_leased_page_state(Some(leased), request, 11).is_ok());
        assert!(matches!(
            validate_leased_page_state(Some(M1KvPoolPageStateV1::INITIAL), request, 1),
            Err(M1DeviceKvArenaLeaseErrorV1::PageLeaseMismatch)
        ));
        assert!(matches!(
            validate_leased_page_state(Some(leased), request, 10),
            Err(M1DeviceKvArenaLeaseErrorV1::PageLeaseMismatch)
        ));
        assert!(matches!(
            validate_leased_page_state(Some(leased), RequestId::new(4, 1), 11),
            Err(M1DeviceKvArenaLeaseErrorV1::PageLeaseMismatch)
        ));
        assert!(matches!(
            validate_leased_page_state(Some(leased), request, 0),
            Err(M1DeviceKvArenaLeaseErrorV1::GenerationLedgerDrift)
        ));
        assert!(matches!(
            validate_leased_page_state(None, request, 11),
            Err(M1DeviceKvArenaLeaseErrorV1::PageLeaseMismatch)
        ));
    }

    #[test]
    fn returned_page_preflight_rejects_hostile_identity_substitution() {
        let expected_device = device();
        let other_device =
            bind_gfx942_device(identity(9), 8, GFX942_PROCESSOR, GFX942_TARGET_FEATURES).unwrap();
        let allocation = identity(2);
        let expected_request = RequestId::new(3, 7);
        let page = PhysicalPageId::new(Qwen3ModelRole::Draft06B, 5, 11);
        let state = Some(M1KvPoolPageStateV1::Leased {
            request: expected_request,
            generation: 11,
        });
        let lease = |device, allocation_id, request, page| DeviceKvPageLease {
            device,
            allocation_id,
            request,
            page,
        };

        let exact = lease(expected_device, allocation, expected_request, page);
        let ticket = preflight_page_return_identity(
            expected_device,
            allocation,
            Qwen3ModelRole::Draft06B,
            state,
            1541,
            expected_request,
            &exact,
        )
        .unwrap();
        assert_eq!(ticket.global_index, 1541);
        assert_eq!(
            returned_page_state(&ticket),
            M1KvPoolPageStateV1::Free { generation: 12 }
        );

        assert_eq!(
            preflight_page_return_identity(
                expected_device,
                allocation,
                Qwen3ModelRole::Draft06B,
                state,
                1541,
                expected_request,
                &lease(other_device, allocation, expected_request, page),
            ),
            Err(M1KvPageReturnErrorV1::Device)
        );
        assert_eq!(
            preflight_page_return_identity(
                expected_device,
                allocation,
                Qwen3ModelRole::Draft06B,
                state,
                1541,
                expected_request,
                &lease(expected_device, identity(3), expected_request, page),
            ),
            Err(M1KvPageReturnErrorV1::Allocation)
        );
        assert_eq!(
            preflight_page_return_identity(
                expected_device,
                allocation,
                Qwen3ModelRole::Draft06B,
                state,
                1541,
                RequestId::new(4, 7),
                &exact,
            ),
            Err(M1KvPageReturnErrorV1::Request)
        );
        assert_eq!(
            preflight_page_return_identity(
                expected_device,
                allocation,
                Qwen3ModelRole::Target8B,
                state,
                1541,
                expected_request,
                &exact,
            ),
            Err(M1KvPageReturnErrorV1::Role)
        );
        assert_eq!(
            preflight_page_return_identity(
                expected_device,
                allocation,
                Qwen3ModelRole::Draft06B,
                Some(M1KvPoolPageStateV1::Leased {
                    request: expected_request,
                    generation: 10,
                }),
                1541,
                expected_request,
                &exact,
            ),
            Err(M1KvPageReturnErrorV1::Ledger)
        );
    }

    #[test]
    fn returned_page_generation_exhaustion_is_preflight_only() {
        let request = RequestId::new(1, 4);
        let lease = DeviceKvPageLease {
            device: device(),
            allocation_id: identity(2),
            request,
            page: PhysicalPageId::new(Qwen3ModelRole::Target8B, 0, u32::MAX),
        };
        assert_eq!(
            preflight_page_return_identity(
                device(),
                identity(2),
                Qwen3ModelRole::Target8B,
                Some(M1KvPoolPageStateV1::Leased {
                    request,
                    generation: u32::MAX,
                }),
                512,
                request,
                &lease,
            ),
            Err(M1KvPageReturnErrorV1::GenerationExhausted)
        );
    }

    #[test]
    fn whole_roster_return_is_transactional_and_draft_then_target() {
        let requests = [RequestId::new(0, 4), RequestId::new(1, 9)];
        let generations = [10u32, 20u32];
        let mut draft = [
            M1KvPoolPageStateV1::Leased {
                request: requests[0],
                generation: generations[0],
            },
            M1KvPoolPageStateV1::Leased {
                request: requests[1],
                generation: generations[1],
            },
        ];
        let mut target = draft;
        let before_draft = draft;
        let before_target = target;
        let allocation = |role| match role {
            Qwen3ModelRole::Draft06B => identity(71),
            Qwen3ModelRole::Target8B => identity(72),
        };
        let lease = |role, lane: usize| DeviceKvPageLease {
            device: device(),
            allocation_id: allocation(role),
            request: requests[lane],
            page: PhysicalPageId::new(role, 4, generations[lane]),
        };

        let hostile = lease(Qwen3ModelRole::Target8B, 1);
        assert_eq!(
            preflight_page_return_identity(
                device(),
                allocation(Qwen3ModelRole::Target8B),
                Qwen3ModelRole::Target8B,
                Some(M1KvPoolPageStateV1::Leased {
                    request: requests[1],
                    generation: generations[1] - 1,
                }),
                global_page_index(requests[1], 4).unwrap(),
                requests[1],
                &hostile,
            ),
            Err(M1KvPageReturnErrorV1::Ledger)
        );
        assert_eq!(draft, before_draft);
        assert_eq!(target, before_target);

        let mut staged = Vec::new();
        for role in M1_KV_PAGE_RETURN_ROLE_ORDER_V1 {
            for lane in 0..requests.len() {
                let lease = lease(role, lane);
                let state = match role {
                    Qwen3ModelRole::Draft06B => draft[lane],
                    Qwen3ModelRole::Target8B => target[lane],
                };
                let ticket = preflight_page_return_identity(
                    device(),
                    allocation(role),
                    role,
                    Some(state),
                    global_page_index(requests[lane], 4).unwrap(),
                    requests[lane],
                    &lease,
                )
                .unwrap();
                staged.push((role, lane, ticket, lease));
            }
        }
        assert_eq!(draft, before_draft);
        assert_eq!(target, before_target);

        let mut order = Vec::new();
        for (role, lane, ticket, lease) in staged {
            order.push((role, lane));
            let state = match role {
                Qwen3ModelRole::Draft06B => &mut draft[lane],
                Qwen3ModelRole::Target8B => &mut target[lane],
            };
            commit_page_return_state(state, ticket, lease);
        }
        assert_eq!(
            order,
            vec![
                (Qwen3ModelRole::Draft06B, 0),
                (Qwen3ModelRole::Draft06B, 1),
                (Qwen3ModelRole::Target8B, 0),
                (Qwen3ModelRole::Target8B, 1),
            ]
        );
        assert_eq!(
            draft,
            [
                M1KvPoolPageStateV1::Free { generation: 11 },
                M1KvPoolPageStateV1::Free { generation: 21 },
            ]
        );
        assert_eq!(target, draft);
    }

    const fn selection(role: Qwen3ModelRole) -> Qwen3PlanSelection {
        Qwen3PlanSelection {
            role,
            mode: Qwen3ExecutionMode::Speculative,
            bucket: Qwen3PlanBucket::SpeculativeS1K4C8192,
        }
    }

    const fn selected(
        role: Qwen3ModelRole,
        mode: Qwen3ExecutionMode,
        bucket: Qwen3PlanBucket,
    ) -> Qwen3PlanSelection {
        Qwen3PlanSelection { role, mode, bucket }
    }

    fn cache_for(
        request: RequestId,
        mode: Qwen3ExecutionMode,
        bucket: Qwen3PlanBucket,
    ) -> ActiveDeviceKvCache {
        ActiveDeviceKvCache::new(
            device(),
            request,
            selected(Qwen3ModelRole::Target8B, mode, bucket),
            selected(Qwen3ModelRole::Draft06B, mode, bucket),
        )
        .unwrap()
    }

    fn cache() -> ActiveDeviceKvCache {
        ActiveDeviceKvCache::new(
            device(),
            request(),
            selection(Qwen3ModelRole::Target8B),
            selection(Qwen3ModelRole::Draft06B),
        )
        .unwrap()
    }

    fn lease(role: Qwen3ModelRole, index: u32, allocation_tag: u8) -> DeviceKvPageLease {
        DeviceKvPageLease::from_contracted_gfx942_allocation(
            device(),
            identity(allocation_tag),
            request(),
            PhysicalPageId::new(role, index, 1),
        )
        .unwrap()
    }

    fn leases(
        role: Qwen3ModelRole,
        start_index: u32,
        count: usize,
        allocation_tag: u8,
    ) -> Vec<DeviceKvPageLease> {
        (0..count)
            .map(|offset| {
                lease(
                    role,
                    start_index + u32::try_from(offset).unwrap(),
                    allocation_tag,
                )
            })
            .collect()
    }

    fn qualification_execution_binding(
        grouping: M1QualificationLaneGrouping,
    ) -> M1QualificationExecutionBindingDeclaration {
        let ordered_lanes = (0..grouping.sequences())
            .map(|lane_ordinal| {
                let lane_tag = u8::try_from(lane_ordinal).unwrap();
                M1QualificationLaneExecutionBinding {
                    lane_ordinal,
                    lane_identity: Identity::new([0x20 + lane_tag; 32]),
                    token_sequence_identity: Identity::new([0x60 + lane_tag; 32]),
                }
            })
            .collect();
        M1QualificationExecutionBindingDeclaration {
            declared_workload_digest: Identity::new(
                [0xa0 + u8::try_from(grouping.sequences()).unwrap(); 32],
            ),
            ordered_lanes,
        }
    }

    fn qualification_plan(
        grouping: M1QualificationLaneGrouping,
    ) -> (
        M1QualificationContextPlan,
        M1QualificationExecutionBindingDeclaration,
    ) {
        let expected = qualification_execution_binding(grouping);
        let plan = m1_qualification_context_plan(grouping, expected.clone());
        (plan, expected)
    }

    fn qualification_cache(
        request: RequestId,
        grouping: M1QualificationLaneGrouping,
    ) -> ActiveDeviceKvCache {
        cache_for(
            request,
            Qwen3ExecutionMode::Decode,
            qualification_decode_bucket(grouping),
        )
    }

    fn qualification_page_lease(
        request: RequestId,
        index: u32,
        allocation_tag: u8,
    ) -> DeviceKvPageLease {
        DeviceKvPageLease::from_contracted_gfx942_allocation(
            device(),
            identity(allocation_tag),
            request,
            PhysicalPageId::new(Qwen3ModelRole::Target8B, index, 1),
        )
        .unwrap()
    }

    fn install_qualification_reserve_for_test(
        cache: &mut ActiveDeviceKvCache,
        context: crate::M1ValidatedQualificationContextStepV1,
        allocation_tag: u8,
    ) {
        let mut unused_pages: Vec<_> = (0..M1_QUALIFICATION_TARGET_PAGE_COUNT_V1)
            .map(|index| {
                qualification_page_lease(
                    cache.common.request,
                    u32::try_from(index).unwrap(),
                    allocation_tag,
                )
            })
            .collect();
        unused_pages.reverse();
        cache.common.target_qualification_reserve = Some(M1QualificationTargetPageReserveV1 {
            device: cache.common.device,
            allocation_id: identity(allocation_tag),
            request: cache.common.request,
            policy_identity: context.policy_identity(),
            grouping: context.grouping(),
            declared_workload_digest: context.declared_workload_digest(),
            lane: context.lane(),
            unused_pages,
        });
    }

    fn complete(
        pending: PendingDeviceKvWrite,
        epoch: u64,
    ) -> Result<InitializedDeviceKvWrite, Box<PendingWriteCompletionFailure>> {
        pending.complete_for_test(ExactCompletion::from_contracted_hsa_quiescence(
            CompletionEpoch::new(epoch),
        ))
    }

    fn complete_step(
        cache: ActiveDeviceKvCache,
        pending: PendingDeviceKvStepWrite,
        epoch: u64,
    ) -> DeviceKvStepCompletionOutcome {
        cache.complete_step_write(
            pending,
            ExactCompletion::from_contracted_hsa_quiescence(CompletionEpoch::new(epoch)),
        )
    }

    fn prefill_step_reservation(epoch: u64) -> (ActiveDeviceKvCache, PendingDeviceKvStepWrite) {
        let mut cache = cache_for(
            request(),
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T128,
        );
        let pending = cache
            .reserve_step_write(
                request(),
                Qwen3ModelRole::Target8B,
                0,
                128,
                CompletionEpoch::new(epoch),
                leases(Qwen3ModelRole::Target8B, 0, 8, 90),
            )
            .unwrap();
        (cache, pending)
    }

    fn cross_page_step_reservation(epoch: u64) -> (ActiveDeviceKvCache, PendingDeviceKvStepWrite) {
        let role = Qwen3ModelRole::Target8B;
        let mut cache = cache_for(
            request(),
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K16C8192,
        );
        append_and_initialize(&mut cache, role, 0, 91, 15, epoch - 1);
        cache.accept_initialized(request(), role, 15).unwrap();
        let pending = cache
            .reserve_step_write(
                request(),
                role,
                15,
                17,
                CompletionEpoch::new(epoch),
                leases(role, 1, 1, 91),
            )
            .unwrap();
        (cache, pending)
    }

    fn assert_completion_rejection_is_identical(
        outcome: DeviceKvStepCompletionOutcome,
        expected_error: DeviceKvCacheError,
        expected_cache: ActiveDeviceKvCache,
        expected_pending: PendingDeviceKvStepWrite,
        expected_completion: ExactCompletion,
    ) {
        let DeviceKvStepCompletionOutcome::Rejected(failure) = outcome else {
            panic!("step completion was not rejected");
        };
        assert_eq!(failure.error(), expected_error);
        let (error, cache, pending, completion) = failure.into_parts();
        assert_eq!(error, expected_error);
        assert_eq!(cache, expected_cache);
        assert_eq!(pending, expected_pending);
        assert_eq!(completion, expected_completion);
    }

    fn append_and_initialize(
        cache: &mut ActiveDeviceKvCache,
        role: Qwen3ModelRole,
        page_index: u32,
        allocation_tag: u8,
        count: u32,
        epoch: u64,
    ) {
        cache
            .append_page(request(), lease(role, page_index, allocation_tag))
            .unwrap();
        let start = cache.projection().target.resident_tokens;
        let start = if role == Qwen3ModelRole::Target8B {
            start
        } else {
            cache.projection().draft.resident_tokens
        };
        for position in start..start + count {
            let pending = cache
                .prepare_write(request(), role, position, CompletionEpoch::new(epoch))
                .unwrap();
            let initialized = complete(pending, epoch).unwrap();
            let read = cache.apply_initialized_write(initialized).unwrap();
            assert_eq!(read.logical_position, position);
        }
    }

    #[test]
    fn prefill_step_reservations_cover_all_exact_page_identities_and_abort_losslessly() {
        for (bucket, active_tokens) in [
            (Qwen3PlanBucket::PrefillS1T128, 128),
            (Qwen3PlanBucket::PrefillS8T128, 128),
            (Qwen3PlanBucket::PrefillS1T512, 512),
            (Qwen3PlanBucket::PrefillS1T2048, 2_048),
        ] {
            let mut cache = cache_for(request(), Qwen3ExecutionMode::Prefill, bucket);
            let page_count = usize::try_from(active_tokens / M1_KV_PAGE_TOKENS).unwrap();
            let pending = cache
                .reserve_step_write(
                    request(),
                    Qwen3ModelRole::Target8B,
                    0,
                    active_tokens,
                    CompletionEpoch::new(31),
                    leases(Qwen3ModelRole::Target8B, 0, page_count, 71),
                )
                .unwrap();

            assert_eq!(pending.committed_tokens(), 0);
            assert_eq!(pending.active_tokens(), active_tokens);
            assert_eq!(pending.end_tokens(), active_tokens);
            assert_eq!(pending.epoch(), CompletionEpoch::new(31));
            assert_eq!(pending.page_table().len(), page_count);
            assert_eq!(pending.write_pages().len(), page_count);
            assert_eq!(pending.new_page_count(), page_count);
            assert_eq!(
                pending
                    .write_pages()
                    .iter()
                    .map(DeviceKvStepPageBinding::token_count)
                    .sum::<u32>(),
                active_tokens
            );
            for (logical_page, page) in pending.write_pages().iter().enumerate() {
                assert_eq!(page.logical_page(), u32::try_from(logical_page).unwrap());
                assert_eq!(page.page().index(), u32::try_from(logical_page).unwrap());
                assert_eq!(page.page().generation(), 1);
                assert_eq!(page.first_offset(), 0);
                assert_eq!(page.token_count(), M1_KV_PAGE_TOKENS);
                assert_eq!(page.allocation_id(), identity(71));
            }
            assert!(cache.projection().target_write_pending);
            assert_eq!(cache.projection().target.resident_tokens, 0);

            let aborted = cache.abort_step_write(pending).unwrap();
            assert_eq!(aborted.page_count(), page_count);
            let recovered = aborted.into_page_leases();
            assert_eq!(recovered.len(), page_count);
            assert!(recovered
                .iter()
                .enumerate()
                .all(|(index, lease)| lease.page().index() == u32::try_from(index).unwrap()));
            let projection = cache.projection();
            assert!(!projection.target_write_pending);
            assert_eq!(projection.target.resident_tokens, 0);
            assert_eq!(projection.target_active_pages, 0);
        }
    }

    #[test]
    fn speculative_step_widths_reserve_exact_cross_page_spans() {
        for (bucket, target_active, draft_active) in [
            (Qwen3PlanBucket::SpeculativeS1K4C8192, 5, 4),
            (Qwen3PlanBucket::SpeculativeS8K4C8192, 5, 4),
            (Qwen3PlanBucket::SpeculativeS1K8C8192, 9, 8),
            (Qwen3PlanBucket::SpeculativeS1K16C8192, 17, 16),
        ] {
            for (role, active_tokens) in [
                (Qwen3ModelRole::Target8B, target_active),
                (Qwen3ModelRole::Draft06B, draft_active),
            ] {
                let mut cache = cache_for(request(), Qwen3ExecutionMode::Speculative, bucket);
                append_and_initialize(&mut cache, role, 0, 72, 15, 32);
                cache.accept_initialized(request(), role, 15).unwrap();

                let pending = cache
                    .reserve_step_write(
                        request(),
                        role,
                        15,
                        active_tokens,
                        CompletionEpoch::new(33),
                        leases(role, 1, 1, 72),
                    )
                    .unwrap();
                assert_eq!(pending.page_table().len(), 2);
                assert_eq!(pending.write_pages().len(), 2);
                assert_eq!(pending.write_pages()[0].logical_page(), 0);
                assert_eq!(pending.write_pages()[0].first_offset(), 15);
                assert_eq!(pending.write_pages()[0].token_count(), 1);
                assert_eq!(
                    pending.write_pages()[0].page(),
                    PhysicalPageId::new(role, 0, 1)
                );
                assert_eq!(pending.write_pages()[1].logical_page(), 1);
                assert_eq!(pending.write_pages()[1].first_offset(), 0);
                assert_eq!(pending.write_pages()[1].token_count(), active_tokens - 1);
                assert_eq!(
                    pending.write_pages()[1].page(),
                    PhysicalPageId::new(role, 1, 1)
                );
                assert_eq!(
                    pending
                        .write_pages()
                        .iter()
                        .map(DeviceKvStepPageBinding::token_count)
                        .sum::<u32>(),
                    active_tokens
                );
                assert_eq!(cache.abort_step_write(pending).unwrap().page_count(), 1);
            }
        }
    }

    #[test]
    fn reservation_snapshots_the_committed_prefix_beyond_the_write_pages() {
        let mut cache = cache_for(
            request(),
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
        );
        append_and_initialize(&mut cache, Qwen3ModelRole::Target8B, 0, 76, 16, 38);
        cache
            .accept_initialized(request(), Qwen3ModelRole::Target8B, 16)
            .unwrap();

        let pending = cache
            .reserve_step_write(
                request(),
                Qwen3ModelRole::Target8B,
                16,
                5,
                CompletionEpoch::new(39),
                leases(Qwen3ModelRole::Target8B, 1, 1, 76),
            )
            .unwrap();
        assert_eq!(pending.page_table().len(), 2);
        assert_eq!(pending.page_table()[0].logical_page(), 0);
        assert_eq!(pending.page_table()[0].page().index(), 0);
        assert_eq!(pending.page_table()[0].page().generation(), 1);
        assert_eq!(pending.page_table()[1].logical_page(), 1);
        assert_eq!(pending.page_table()[1].page().index(), 1);
        assert_eq!(pending.page_table()[1].page().generation(), 1);
        assert_eq!(pending.write_pages().len(), 1);
        assert_eq!(pending.write_pages()[0].logical_page(), 1);
        assert_eq!(pending.write_pages()[0].first_offset(), 0);
        assert_eq!(pending.write_pages()[0].token_count(), 5);
        assert_eq!(cache.abort_step_write(pending).unwrap().page_count(), 1);
    }

    #[test]
    fn reservation_rejections_are_transactional_and_retryable() {
        let mut cache = cache_for(
            request(),
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T128,
        );
        let failure = cache
            .reserve_step_write(
                request(),
                Qwen3ModelRole::Target8B,
                0,
                128,
                CompletionEpoch::new(34),
                leases(Qwen3ModelRole::Target8B, 0, 7, 73),
            )
            .unwrap_err();
        assert_eq!(
            failure.error(),
            DeviceKvCacheError::StepPageLeaseCountMismatch
        );
        let (_, mut recovered) = failure.into_parts();
        assert_eq!(recovered.len(), 7);
        assert!(!cache.projection().target_write_pending);
        assert_eq!(cache.projection().target.resident_tokens, 0);

        recovered.push(lease(Qwen3ModelRole::Target8B, 7, 73));
        let pending = cache
            .reserve_step_write(
                request(),
                Qwen3ModelRole::Target8B,
                0,
                128,
                CompletionEpoch::new(34),
                recovered,
            )
            .unwrap();
        assert_eq!(cache.abort_step_write(pending).unwrap().page_count(), 8);

        let mut duplicated = leases(Qwen3ModelRole::Target8B, 0, 8, 73);
        duplicated[7].page = PhysicalPageId::new(Qwen3ModelRole::Target8B, 0, 1);
        let failure = cache
            .reserve_step_write(
                request(),
                Qwen3ModelRole::Target8B,
                0,
                128,
                CompletionEpoch::new(35),
                duplicated,
            )
            .unwrap_err();
        assert_eq!(failure.error(), DeviceKvCacheError::StepPhysicalAlias);
        assert_eq!(failure.into_parts().1.len(), 8);
        assert!(!cache.projection().target_write_pending);
    }

    #[test]
    fn abort_mismatch_retains_reservation_for_its_exact_cache() {
        let mut owner = cache_for(
            request(),
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T128,
        );
        let pending = owner
            .reserve_step_write(
                request(),
                Qwen3ModelRole::Target8B,
                0,
                128,
                CompletionEpoch::new(36),
                leases(Qwen3ModelRole::Target8B, 0, 8, 74),
            )
            .unwrap();
        let other_request = RequestId::new(request().slot() + 1, request().generation());
        let mut other = cache_for(
            other_request,
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T128,
        );
        let failure = other.abort_step_write(pending).unwrap_err();
        assert_eq!(failure.error(), DeviceKvCacheError::WrongRequest);
        let (_, pending) = failure.into_parts();
        assert!(!other.projection().target_write_pending);
        assert!(owner.projection().target_write_pending);
        assert_eq!(owner.abort_step_write(pending).unwrap().page_count(), 8);
        assert!(!owner.projection().target_write_pending);
    }

    #[test]
    fn pending_step_blocks_conflicting_role_transitions() {
        let mut cache = cache_for(
            request(),
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
        );
        let pending = cache
            .reserve_step_write(
                request(),
                Qwen3ModelRole::Target8B,
                0,
                5,
                CompletionEpoch::new(37),
                leases(Qwen3ModelRole::Target8B, 0, 1, 75),
            )
            .unwrap();
        assert_eq!(
            cache.prepare_write(
                request(),
                Qwen3ModelRole::Target8B,
                0,
                CompletionEpoch::new(37),
            ),
            Err(DeviceKvCacheError::PendingWriteExists)
        );
        assert_eq!(
            cache.accept_initialized(request(), Qwen3ModelRole::Target8B, 0),
            Err(DeviceKvCacheError::PendingWriteExists)
        );
        let failure = cache
            .append_page(request(), lease(Qwen3ModelRole::Target8B, 0, 75))
            .unwrap_err();
        assert_eq!(failure.error(), DeviceKvCacheError::PendingWriteExists);
        assert_eq!(failure.into_parts().1.page().index(), 0);
        assert_eq!(cache.abort_step_write(pending).unwrap().page_count(), 1);
    }

    #[test]
    fn completed_steps_initialize_every_prefill_decode_and_speculative_width() {
        let cases = [
            (
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T128,
                Qwen3ModelRole::Target8B,
                128,
                0,
            ),
            (
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T512,
                Qwen3ModelRole::Target8B,
                512,
                0,
            ),
            (
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T2048,
                Qwen3ModelRole::Target8B,
                2_048,
                0,
            ),
            (
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS1C8192,
                Qwen3ModelRole::Target8B,
                1,
                15,
            ),
            (
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K4C8192,
                Qwen3ModelRole::Target8B,
                5,
                15,
            ),
            (
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K4C8192,
                Qwen3ModelRole::Draft06B,
                4,
                15,
            ),
            (
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K8C8192,
                Qwen3ModelRole::Target8B,
                9,
                15,
            ),
            (
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K8C8192,
                Qwen3ModelRole::Draft06B,
                8,
                15,
            ),
            (
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K16C8192,
                Qwen3ModelRole::Target8B,
                17,
                15,
            ),
            (
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K16C8192,
                Qwen3ModelRole::Draft06B,
                16,
                15,
            ),
        ];

        for (case, (mode, bucket, role, active_tokens, committed_tokens)) in
            cases.into_iter().enumerate()
        {
            let epoch = 100 + u64::try_from(case).unwrap();
            let allocation_tag = 100 + u8::try_from(case).unwrap();
            let mut cache = cache_for(request(), mode, bucket);
            if committed_tokens != 0 {
                append_and_initialize(
                    &mut cache,
                    role,
                    0,
                    allocation_tag,
                    committed_tokens,
                    epoch - 1,
                );
                cache
                    .accept_initialized(request(), role, committed_tokens)
                    .unwrap();
            }
            let end_tokens = committed_tokens + active_tokens;
            let required_pages = usize::try_from(end_tokens.div_ceil(M1_KV_PAGE_TOKENS)).unwrap();
            let existing_pages = usize::from(committed_tokens != 0);
            let pending = cache
                .reserve_step_write(
                    request(),
                    role,
                    committed_tokens,
                    active_tokens,
                    CompletionEpoch::new(epoch),
                    leases(
                        role,
                        u32::try_from(existing_pages).unwrap(),
                        required_pages - existing_pages,
                        allocation_tag,
                    ),
                )
                .unwrap();

            let completed = match complete_step(cache, pending, epoch) {
                DeviceKvStepCompletionOutcome::Completed(completed) => completed,
                other => panic!("exact step completion case {case} did not complete: {other:?}"),
            };
            let (cache, initialized, completion) = completed.into_parts();
            assert_eq!(completion.epoch(), CompletionEpoch::new(epoch));
            assert_eq!(initialized.request(), request());
            assert_eq!(initialized.selection(), selected(role, mode, bucket));
            assert_eq!(initialized.committed_tokens(), committed_tokens);
            assert_eq!(initialized.active_tokens(), active_tokens);
            assert_eq!(initialized.end_tokens(), end_tokens);
            assert_eq!(initialized.epoch(), CompletionEpoch::new(epoch));
            assert_eq!(initialized.page_table().len(), required_pages);
            assert_eq!(
                initialized
                    .write_pages()
                    .iter()
                    .map(DeviceKvStepPageBinding::token_count)
                    .sum::<u32>(),
                active_tokens
            );

            let projection = cache.projection();
            let logical = if role == Qwen3ModelRole::Target8B {
                projection.target
            } else {
                projection.draft
            };
            assert_eq!(logical.committed_tokens, committed_tokens);
            assert_eq!(logical.resident_tokens, end_tokens);
            assert!(!projection.target_write_pending);
            assert!(!projection.draft_write_pending);
            for logical_position in 0..end_tokens {
                assert_eq!(
                    cache
                        .map_initialized(request(), role, logical_position)
                        .unwrap()
                        .logical_position,
                    logical_position
                );
            }
        }
    }

    #[test]
    fn cross_page_completion_retains_existing_tail_and_appends_new_page_custody() {
        let (cache, pending) = cross_page_step_reservation(120);
        assert_eq!(pending.write_pages().len(), 2);
        assert_eq!(pending.write_pages()[0].first_offset(), 15);
        assert_eq!(pending.write_pages()[0].token_count(), 1);
        assert_eq!(pending.write_pages()[1].first_offset(), 0);
        assert_eq!(pending.write_pages()[1].token_count(), 16);

        let DeviceKvStepCompletionOutcome::Completed(completed) =
            complete_step(cache, pending, 120)
        else {
            panic!("cross-page step completion did not complete");
        };
        let (cache, initialized, completion) = completed.into_parts();
        assert_eq!(completion.epoch(), CompletionEpoch::new(120));
        assert_eq!(initialized.write_pages().len(), 2);
        let projection = cache.projection();
        assert_eq!(projection.target_active_pages, 2);
        assert_eq!(projection.target.committed_tokens, 15);
        assert_eq!(projection.target.resident_tokens, 32);
        assert_eq!(
            cache
                .map_initialized(request(), Qwen3ModelRole::Target8B, 31)
                .unwrap()
                .location,
            PhysicalKvLocation {
                page: PhysicalPageId::new(Qwen3ModelRole::Target8B, 1, 1),
                offset: 15,
            }
        );
    }

    #[test]
    fn speculative_draft_round_reserves_and_completes_one_exact_k_interval() {
        for (case, bucket) in [
            Qwen3PlanBucket::SpeculativeS1K4C8192,
            Qwen3PlanBucket::SpeculativeS8K4C8192,
            Qwen3PlanBucket::SpeculativeS1K8C8192,
            Qwen3PlanBucket::SpeculativeS1K16C8192,
        ]
        .into_iter()
        .enumerate()
        {
            let epoch = 140 + u64::try_from(case).unwrap();
            let allocation_tag = 140 + u8::try_from(case).unwrap();
            let target = selected(
                Qwen3ModelRole::Target8B,
                Qwen3ExecutionMode::Speculative,
                bucket,
            );
            let (_, draft_decode, draft_tokens) =
                m1_speculative_draft_round_shape_v1(target).unwrap();
            let mut cache = cache_for(request(), Qwen3ExecutionMode::Speculative, bucket);
            append_and_initialize(
                &mut cache,
                Qwen3ModelRole::Draft06B,
                0,
                allocation_tag,
                15,
                epoch - 1,
            );
            cache
                .accept_initialized(request(), Qwen3ModelRole::Draft06B, 15)
                .unwrap();

            let aggregate = cache
                .reserve_speculative_draft_round_write(
                    request(),
                    target,
                    draft_decode,
                    15,
                    CompletionEpoch::new(epoch),
                    leases(Qwen3ModelRole::Draft06B, 1, 1, allocation_tag),
                )
                .unwrap();
            assert_eq!(aggregate.target_speculative_selection(), target);
            assert_eq!(aggregate.draft_decode_selection(), draft_decode);
            assert_eq!(aggregate.draft_tokens(), draft_tokens);
            assert_eq!(aggregate.pending_step_write().committed_tokens(), 15);
            assert_eq!(aggregate.pending_step_write().active_tokens(), draft_tokens);
            assert_eq!(
                aggregate.pending_step_write().end_tokens(),
                15 + draft_tokens
            );
            assert_eq!(aggregate.pending_step_write().write_pages().len(), 2);
            assert!(cache.projection().draft_write_pending);

            let completed = match complete_step(cache, aggregate.into_pending_step_write(), epoch) {
                DeviceKvStepCompletionOutcome::Completed(completed) => completed,
                other => panic!("draft round case {case} did not complete: {other:?}"),
            };
            let (cache, initialized, completion) = completed.into_parts();
            assert_eq!(completion.epoch(), CompletionEpoch::new(epoch));
            assert_eq!(initialized.active_tokens(), draft_tokens);
            assert_eq!(initialized.end_tokens(), 15 + draft_tokens);
            let projection = cache.projection();
            assert_eq!(projection.draft.committed_tokens, 15);
            assert_eq!(projection.draft.resident_tokens, 15 + draft_tokens);
            assert_eq!(projection.draft_active_pages, 2);
            assert!(!projection.draft_write_pending);
        }
    }

    #[test]
    fn speculative_draft_round_selection_rejection_is_transactional() {
        let bucket = Qwen3PlanBucket::SpeculativeS1K4C8192;
        let target = selected(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Speculative,
            bucket,
        );
        let (_, draft_decode, _) = m1_speculative_draft_round_shape_v1(target).unwrap();
        let wrong_target = selected(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K8C8192,
        );
        let wrong_draft = selected(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS8C8192,
        );
        for (supplied_target, supplied_draft) in
            [(wrong_target, draft_decode), (target, wrong_draft)]
        {
            let mut cache = cache_for(request(), Qwen3ExecutionMode::Speculative, bucket);
            let before = cache.projection();
            let failure = cache
                .reserve_speculative_draft_round_write(
                    request(),
                    supplied_target,
                    supplied_draft,
                    0,
                    CompletionEpoch::new(150),
                    leases(Qwen3ModelRole::Draft06B, 0, 1, 150),
                )
                .unwrap_err();
            assert_eq!(failure.error(), DeviceKvCacheError::StepSelectionMismatch);
            let (_, recovered_leases) = failure.into_parts();
            assert_eq!(recovered_leases.len(), 1);
            assert_eq!(cache.projection(), before);
            let recovered = cache
                .reserve_speculative_draft_round_write(
                    request(),
                    target,
                    draft_decode,
                    0,
                    CompletionEpoch::new(150),
                    recovered_leases,
                )
                .unwrap();
            assert_eq!(recovered.draft_tokens(), 4);
        }
    }

    #[test]
    fn completion_epoch_rejection_returns_identical_linear_inputs() {
        let (cache, pending) = prefill_step_reservation(121);
        let (expected_cache, expected_pending) = prefill_step_reservation(121);
        let supplied = ExactCompletion::from_contracted_hsa_quiescence(CompletionEpoch::new(122));
        let expected = ExactCompletion::from_contracted_hsa_quiescence(CompletionEpoch::new(122));
        let outcome = cache.complete_step_write(pending, supplied);
        assert_completion_rejection_is_identical(
            outcome,
            DeviceKvCacheError::CompletionEpochMismatch,
            expected_cache,
            expected_pending,
            expected,
        );
    }

    #[test]
    fn request_selection_and_pending_marker_drift_reject_before_mutation() {
        let (cache, mut pending) = prefill_step_reservation(123);
        let (expected_cache, mut expected_pending) = prefill_step_reservation(123);
        let stale_request = RequestId::new(request().slot(), request().generation() + 1);
        pending.corrupt_completion_bridge_request_for_test(stale_request);
        expected_pending.corrupt_completion_bridge_request_for_test(stale_request);
        assert_completion_rejection_is_identical(
            complete_step(cache, pending, 123),
            DeviceKvCacheError::WrongRequest,
            expected_cache,
            expected_pending,
            ExactCompletion::from_contracted_hsa_quiescence(CompletionEpoch::new(123)),
        );

        let (cache, mut pending) = prefill_step_reservation(124);
        let (expected_cache, mut expected_pending) = prefill_step_reservation(124);
        let drifted_selection = selected(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T512,
        );
        pending.corrupt_completion_bridge_selection_for_test(drifted_selection);
        expected_pending.corrupt_completion_bridge_selection_for_test(drifted_selection);
        assert_completion_rejection_is_identical(
            complete_step(cache, pending, 124),
            DeviceKvCacheError::StepSelectionMismatch,
            expected_cache,
            expected_pending,
            ExactCompletion::from_contracted_hsa_quiescence(CompletionEpoch::new(124)),
        );

        let (mut cache, pending) = prefill_step_reservation(125);
        let (mut expected_cache, expected_pending) = prefill_step_reservation(125);
        cache.common.target.pending = None;
        expected_cache.common.target.pending = None;
        assert_completion_rejection_is_identical(
            complete_step(cache, pending, 125),
            DeviceKvCacheError::NoPendingWrite,
            expected_cache,
            expected_pending,
            ExactCompletion::from_contracted_hsa_quiescence(CompletionEpoch::new(125)),
        );

        let (mut cache, pending) = prefill_step_reservation(126);
        let (mut expected_cache, expected_pending) = prefill_step_reservation(126);
        for marker in [
            &mut cache.common.target.pending,
            &mut expected_cache.common.target.pending,
        ] {
            let Some(PendingWriteState::Step(mut binding)) = *marker else {
                panic!("fixture did not retain its pending step marker");
            };
            binding.write_generation += 1;
            *marker = Some(PendingWriteState::Step(binding));
        }
        assert_completion_rejection_is_identical(
            complete_step(cache, pending, 126),
            DeviceKvCacheError::PendingWriteMismatch,
            expected_cache,
            expected_pending,
            ExactCompletion::from_contracted_hsa_quiescence(CompletionEpoch::new(126)),
        );
    }

    #[test]
    fn page_table_write_span_and_owned_cache_drift_reject_before_mutation() {
        let (cache, mut pending) = prefill_step_reservation(127);
        let (expected_cache, mut expected_pending) = prefill_step_reservation(127);
        let original = pending.page_table()[0];
        pending.corrupt_workspace_bridge_page_for_test(
            0,
            7,
            original.allocation_id(),
            original.page(),
        );
        expected_pending.corrupt_workspace_bridge_page_for_test(
            0,
            7,
            original.allocation_id(),
            original.page(),
        );
        assert_completion_rejection_is_identical(
            complete_step(cache, pending, 127),
            DeviceKvCacheError::StepPageTableDrift,
            expected_cache,
            expected_pending,
            ExactCompletion::from_contracted_hsa_quiescence(CompletionEpoch::new(127)),
        );

        let (cache, mut pending) = prefill_step_reservation(128);
        let (expected_cache, mut expected_pending) = prefill_step_reservation(128);
        pending.corrupt_completion_bridge_write_span_for_test(0, 15);
        expected_pending.corrupt_completion_bridge_write_span_for_test(0, 15);
        assert_completion_rejection_is_identical(
            complete_step(cache, pending, 128),
            DeviceKvCacheError::StepWriteSpanDrift,
            expected_cache,
            expected_pending,
            ExactCompletion::from_contracted_hsa_quiescence(CompletionEpoch::new(128)),
        );

        let (mut cache, pending) = cross_page_step_reservation(129);
        let (mut expected_cache, expected_pending) = cross_page_step_reservation(129);
        cache.common.target.active_pages[0].page =
            PhysicalPageId::new(Qwen3ModelRole::Target8B, 2, 1);
        expected_cache.common.target.active_pages[0].page =
            PhysicalPageId::new(Qwen3ModelRole::Target8B, 2, 1);
        assert_completion_rejection_is_identical(
            complete_step(cache, pending, 129),
            DeviceKvCacheError::OwnedPageTableDrift,
            expected_cache,
            expected_pending,
            ExactCompletion::from_contracted_hsa_quiescence(CompletionEpoch::new(129)),
        );
    }

    #[test]
    fn exact_device_declaration_rejects_identity_and_target_drift() {
        assert_eq!(
            bind_gfx942_device(
                Identity::new([0; 32]),
                0,
                GFX942_PROCESSOR,
                GFX942_TARGET_FEATURES,
            ),
            Err(DeviceKvCacheError::MissingDeviceIdentity)
        );
        assert_eq!(
            bind_gfx942_device(identity(1), 0, "gfx941", GFX942_TARGET_FEATURES),
            Err(DeviceKvCacheError::ProcessorMismatch)
        );
        assert_eq!(
            bind_gfx942_device(identity(1), 0, GFX942_PROCESSOR, "+xnack"),
            Err(DeviceKvCacheError::TargetFeaturesMismatch)
        );
        assert_eq!(device().target(), Target::Gfx942XnackMinus);
    }

    #[test]
    fn target_and_draft_plan_roles_and_buckets_are_isolated() {
        assert_eq!(
            ActiveDeviceKvCache::new(
                device(),
                request(),
                selection(Qwen3ModelRole::Draft06B),
                selection(Qwen3ModelRole::Target8B),
            ),
            Err(DeviceKvCacheError::PlanPairMismatch)
        );
        let mismatched_draft = Qwen3PlanSelection {
            role: Qwen3ModelRole::Draft06B,
            mode: Qwen3ExecutionMode::Decode,
            bucket: Qwen3PlanBucket::DecodeS1C8192,
        };
        assert_eq!(
            ActiveDeviceKvCache::new(
                device(),
                request(),
                selection(Qwen3ModelRole::Target8B),
                mismatched_draft,
            ),
            Err(DeviceKvCacheError::PlanPairMismatch)
        );
    }

    #[test]
    fn cross_role_allocation_alias_is_rejected_without_losing_the_lease() {
        let mut cache = cache();
        cache
            .append_page(request(), lease(Qwen3ModelRole::Target8B, 0, 21))
            .unwrap();
        let failure = cache
            .append_page(request(), lease(Qwen3ModelRole::Draft06B, 0, 21))
            .unwrap_err();
        assert_eq!(failure.error(), DeviceKvCacheError::AllocationAlias);
        let (_, returned) = failure.into_parts();
        assert_eq!(returned.page().role(), Qwen3ModelRole::Draft06B);
        assert_eq!(cache.projection().target_active_pages, 1);
        assert_eq!(cache.projection().draft_active_pages, 0);
    }

    #[test]
    fn same_role_pages_share_one_arena_and_reject_arena_substitution() {
        let mut cache = cache();
        append_and_initialize(&mut cache, Qwen3ModelRole::Target8B, 0, 41, 16, 20);
        cache
            .append_page(request(), lease(Qwen3ModelRole::Target8B, 1, 41))
            .unwrap();
        let projection = cache.projection();
        assert_eq!(projection.target_arena_allocation_id, Some(identity(41)));
        assert_eq!(projection.draft_arena_allocation_id, None);
        assert_eq!(projection.target_active_pages, 2);

        let failure = cache
            .append_page(request(), lease(Qwen3ModelRole::Target8B, 2, 42))
            .unwrap_err();
        assert_eq!(failure.error(), DeviceKvCacheError::ArenaAllocationMismatch);
        let (_, returned) = failure.into_parts();
        assert_eq!(returned.allocation_id(), identity(42));
        assert_eq!(cache.projection().target_active_pages, 2);
    }

    #[test]
    fn wrong_device_and_stale_page_generation_are_rejected_transactionally() {
        let mut cache = cache();
        let other_device =
            bind_gfx942_device(identity(1), 8, GFX942_PROCESSOR, GFX942_TARGET_FEATURES).unwrap();
        let wrong_device = DeviceKvPageLease::from_contracted_gfx942_allocation(
            other_device,
            identity(30),
            request(),
            PhysicalPageId::new(Qwen3ModelRole::Target8B, 0, 1),
        )
        .unwrap();
        let failure = cache.append_page(request(), wrong_device).unwrap_err();
        assert_eq!(failure.error(), DeviceKvCacheError::WrongDevice);
        let (_, wrong_device) = failure.into_parts();
        assert_eq!(wrong_device.allocation_id(), identity(30));

        let stale = DeviceKvPageLease::from_contracted_gfx942_allocation(
            device(),
            identity(31),
            request(),
            PhysicalPageId::new(Qwen3ModelRole::Target8B, 0, 2),
        )
        .unwrap();
        let failure = cache.append_page(request(), stale).unwrap_err();
        assert_eq!(
            failure.error(),
            DeviceKvCacheError::Physical(PhysicalKvError::PageGenerationMismatch)
        );
        assert_eq!(cache.projection().target_active_pages, 0);
        assert_eq!(cache.projection().target.resident_tokens, 0);
    }

    #[test]
    fn request_bound_page_lease_cannot_cross_request_generations() {
        let mut cache = cache();
        let other_request = RequestId::new(request().slot(), request().generation() + 1);
        let lease = DeviceKvPageLease::from_contracted_gfx942_allocation(
            device(),
            identity(32),
            other_request,
            PhysicalPageId::new(Qwen3ModelRole::Target8B, 0, 1),
        )
        .unwrap();
        let failure = cache.append_page(request(), lease).unwrap_err();
        assert_eq!(failure.error(), DeviceKvCacheError::WrongRequest);
        let (_, recovered) = failure.into_parts();
        assert_eq!(recovered.request(), other_request);
        assert_eq!(cache.projection().target_active_pages, 0);
    }

    #[test]
    fn hostile_owned_tail_drift_is_detected_before_rollback_or_retire_mutation() {
        let mut active = cache();
        append_and_initialize(&mut active, Qwen3ModelRole::Target8B, 0, 32, 1, 16);
        active.common.target.active_pages[0].page =
            PhysicalPageId::new(Qwen3ModelRole::Target8B, 1, 1);
        assert_eq!(
            active.rollback_one(
                request(),
                Qwen3ModelRole::Target8B,
                CompletionEpoch::new(16),
            ),
            Err(DeviceKvCacheError::OwnedPageTableDrift)
        );
        let projection = active.projection();
        assert_eq!(projection.target.resident_tokens, 1);
        assert_eq!(projection.target_active_pages, 1);
        assert_eq!(projection.target_retired_pages, 0);

        let mut cancelled = match active.cancel(request(), CompletionEpoch::new(16)) {
            DeviceKvCancellationOutcome::Cancelled(cancelled) => cancelled,
            other => panic!("unexpected cancellation outcome: {other:?}"),
        };
        assert_eq!(
            cancelled.retire_next(request(), Qwen3ModelRole::Target8B),
            Err(DeviceKvCacheError::OwnedPageTableDrift)
        );
        let projection = cancelled.projection();
        assert_eq!(projection.target.resident_tokens, 1);
        assert_eq!(projection.target_active_pages, 1);
        assert_eq!(projection.target_retired_pages, 0);
    }

    #[test]
    fn uninitialized_read_and_wrong_completion_epoch_fail_closed() {
        let mut cache = cache();
        cache
            .append_page(request(), lease(Qwen3ModelRole::Target8B, 0, 22))
            .unwrap();
        assert_eq!(
            cache.map_initialized(request(), Qwen3ModelRole::Target8B, 0),
            Err(DeviceKvCacheError::Physical(
                PhysicalKvError::LogicalPositionOutOfRange,
            ))
        );
        let pending = cache
            .prepare_write(
                request(),
                Qwen3ModelRole::Target8B,
                0,
                CompletionEpoch::new(9),
            )
            .unwrap();
        let failure = complete(pending, 10).unwrap_err();
        assert_eq!(failure.error(), DeviceKvCacheError::CompletionEpochMismatch);
        let (pending, _) = failure.into_parts();
        let initialized = complete(pending, 9).unwrap();
        cache.apply_initialized_write(initialized).unwrap();
        assert_eq!(
            cache
                .map_initialized(request(), Qwen3ModelRole::Target8B, 0)
                .unwrap()
                .location
                .offset,
            0
        );
    }

    #[test]
    fn stale_request_and_stale_pending_generation_never_mutate_the_prefix() {
        let mut cache = cache();
        cache
            .append_page(request(), lease(Qwen3ModelRole::Target8B, 0, 23))
            .unwrap();
        let stale = RequestId::new(request().slot(), request().generation() + 1);
        assert_eq!(
            cache.prepare_write(stale, Qwen3ModelRole::Target8B, 0, CompletionEpoch::new(8),),
            Err(DeviceKvCacheError::WrongRequest)
        );
        let pending = cache
            .prepare_write(
                request(),
                Qwen3ModelRole::Target8B,
                0,
                CompletionEpoch::new(8),
            )
            .unwrap();
        let mut initialized = complete(pending, 8).unwrap();
        initialized.binding.write_generation += 1;
        let failure = cache.apply_initialized_write(initialized).unwrap_err();
        assert_eq!(failure.error(), DeviceKvCacheError::PendingWriteMismatch);
        assert_eq!(cache.projection().target.resident_tokens, 0);
        assert!(cache.projection().target_write_pending);
    }

    #[test]
    fn accept_and_rollback_retain_only_the_initialized_committed_prefix() {
        let mut cache = cache();
        append_and_initialize(&mut cache, Qwen3ModelRole::Target8B, 0, 24, 16, 11);
        cache
            .accept_initialized(request(), Qwen3ModelRole::Target8B, 8)
            .unwrap();
        for _ in 0..8 {
            assert_eq!(
                cache
                    .rollback_one(
                        request(),
                        Qwen3ModelRole::Target8B,
                        CompletionEpoch::new(11),
                    )
                    .unwrap(),
                DeviceKvRetirementOutcome::TokenRemoved
            );
        }
        let projection = cache.projection();
        assert_eq!(projection.target.resident_tokens, 8);
        assert_eq!(projection.target.committed_tokens, 8);
        assert_eq!(projection.target_active_pages, 1);
        assert_eq!(projection.target_retired_pages, 0);
        assert_eq!(
            cache.rollback_one(
                request(),
                Qwen3ModelRole::Target8B,
                CompletionEpoch::new(11),
            ),
            Err(DeviceKvCacheError::Physical(
                PhysicalKvError::NoTentativeToken,
            ))
        );
    }

    #[test]
    fn empty_tentative_tail_page_enters_retirement_custody() {
        let mut cache = cache();
        append_and_initialize(&mut cache, Qwen3ModelRole::Draft06B, 0, 25, 16, 12);
        cache
            .accept_initialized(request(), Qwen3ModelRole::Draft06B, 16)
            .unwrap();
        append_and_initialize(&mut cache, Qwen3ModelRole::Draft06B, 1, 25, 1, 12);
        assert!(matches!(
            cache
                .rollback_one(
                    request(),
                    Qwen3ModelRole::Draft06B,
                    CompletionEpoch::new(12),
                )
                .unwrap(),
            DeviceKvRetirementOutcome::PageRetired(page)
                if page == PhysicalPageId::new(Qwen3ModelRole::Draft06B, 1, 1)
        ));
        let projection = cache.projection();
        assert_eq!(projection.draft.resident_tokens, 16);
        assert_eq!(projection.draft_active_pages, 1);
        assert_eq!(projection.draft_retired_pages, 1);
    }

    #[test]
    fn cancellation_rejects_pending_writes_and_quiescence_is_exact_epoch_only() {
        let mut cache = cache();
        cache
            .append_page(request(), lease(Qwen3ModelRole::Target8B, 0, 27))
            .unwrap();
        let _pending = cache
            .prepare_write(
                request(),
                Qwen3ModelRole::Target8B,
                0,
                CompletionEpoch::new(13),
            )
            .unwrap();
        let failure = match cache.cancel(request(), CompletionEpoch::new(13)) {
            DeviceKvCancellationOutcome::Rejected(failure) => failure,
            other => panic!("unexpected cancellation outcome: {other:?}"),
        };
        assert_eq!(failure.error(), DeviceKvCacheError::PendingWriteExists);
    }

    #[test]
    fn cancelled_pages_retire_before_exact_quiescent_terminal_custody() {
        let mut cache = cache();
        append_and_initialize(&mut cache, Qwen3ModelRole::Target8B, 0, 28, 4, 14);
        append_and_initialize(&mut cache, Qwen3ModelRole::Draft06B, 0, 29, 3, 14);
        let mut cancelled = match cache.cancel(request(), CompletionEpoch::new(14)) {
            DeviceKvCancellationOutcome::Cancelled(cancelled) => cancelled,
            other => panic!("unexpected cancellation outcome: {other:?}"),
        };
        assert_eq!(cancelled.retire_all(request()).unwrap(), 2);
        let wrong = ExactCompletion::from_contracted_hsa_quiescence(CompletionEpoch::new(15));
        let failure = cancelled.quiesce(wrong).unwrap_err();
        assert_eq!(failure.error(), DeviceKvCacheError::CompletionEpochMismatch);
        let (cancelled, _) = failure.into_parts();
        let exact = ExactCompletion::from_contracted_hsa_quiescence(CompletionEpoch::new(14));
        let quiescent = cancelled.quiesce(exact).unwrap();
        let projection = quiescent.projection();
        assert_eq!(projection.target_active_pages, 0);
        assert_eq!(projection.draft_active_pages, 0);
        assert_eq!(projection.target_retired_pages, 1);
        assert_eq!(projection.draft_retired_pages, 1);
        assert_eq!(projection.target_quiescent_retired_pages, 1);
        assert_eq!(projection.draft_quiescent_retired_pages, 1);
        assert_eq!(quiescent.completion_epoch(), CompletionEpoch::new(14));
    }

    #[test]
    fn rollback_retirement_requires_its_exact_completion_before_later_quiescence() {
        let mut active = cache();
        append_and_initialize(&mut active, Qwen3ModelRole::Target8B, 0, 33, 16, 11);
        active
            .accept_initialized(request(), Qwen3ModelRole::Target8B, 16)
            .unwrap();
        append_and_initialize(&mut active, Qwen3ModelRole::Target8B, 1, 33, 1, 11);
        assert!(matches!(
            active
                .rollback_one(
                    request(),
                    Qwen3ModelRole::Target8B,
                    CompletionEpoch::new(11),
                )
                .unwrap(),
            DeviceKvRetirementOutcome::PageRetired(_)
        ));

        let mut cancelled = match active.cancel(request(), CompletionEpoch::new(14)) {
            DeviceKvCancellationOutcome::Cancelled(cancelled) => cancelled,
            other => panic!("unexpected cancellation outcome: {other:?}"),
        };
        assert_eq!(cancelled.retire_all(request()).unwrap(), 1);
        let failure = cancelled
            .quiesce(ExactCompletion::from_contracted_hsa_quiescence(
                CompletionEpoch::new(14),
            ))
            .unwrap_err();
        assert_eq!(
            failure.error(),
            DeviceKvCacheError::UnsettledPriorRetirement
        );
        let (mut cancelled, _) = failure.into_parts();
        let (settled, _completion) = cancelled
            .settle_retired_epoch(ExactCompletion::from_contracted_hsa_quiescence(
                CompletionEpoch::new(11),
            ))
            .unwrap();
        assert_eq!(settled, 1);
        let quiescent = cancelled
            .quiesce(ExactCompletion::from_contracted_hsa_quiescence(
                CompletionEpoch::new(14),
            ))
            .unwrap();
        let projection = quiescent.projection();
        assert_eq!(projection.target_retired_pages, 2);
        assert_eq!(projection.target_quiescent_retired_pages, 2);
        assert_eq!(projection.draft_retired_pages, 0);
        assert_eq!(projection.draft_quiescent_retired_pages, 0);
    }

    #[test]
    fn continuing_release_moves_only_quiescent_rollback_pages() {
        let mut active = cache();
        append_and_initialize(&mut active, Qwen3ModelRole::Target8B, 0, 51, 16, 21);
        active
            .accept_initialized(request(), Qwen3ModelRole::Target8B, 16)
            .unwrap();
        append_and_initialize(&mut active, Qwen3ModelRole::Target8B, 1, 51, 1, 21);
        assert!(matches!(
            active
                .rollback_one(
                    request(),
                    Qwen3ModelRole::Target8B,
                    CompletionEpoch::new(21),
                )
                .unwrap(),
            DeviceKvRetirementOutcome::PageRetired(_)
        ));
        let (_settled, _completion) = active
            .settle_retired_epoch(ExactCompletion::from_contracted_hsa_quiescence(
                CompletionEpoch::new(21),
            ))
            .unwrap();
        assert!(active.release_state_is_valid());

        let returned = active.take_retired_pages(Qwen3ModelRole::Target8B);
        assert_eq!(returned.len(), 1);
        assert_eq!(returned[0].lease().page().index(), 1);
        assert!(returned[0].is_quiescent());
        let projection = active.projection();
        assert_eq!(projection.target_active_pages, 1);
        assert_eq!(projection.target_retired_pages, 0);
        assert_eq!(projection.target_arena_allocation_id, Some(identity(51)));
    }

    #[test]
    fn terminal_release_moves_every_role_page_and_clears_arena_markers() {
        let mut cache = cache();
        append_and_initialize(&mut cache, Qwen3ModelRole::Target8B, 0, 61, 2, 22);
        append_and_initialize(&mut cache, Qwen3ModelRole::Draft06B, 0, 62, 2, 22);
        let mut cancelled = match cache.cancel(request(), CompletionEpoch::new(22)) {
            DeviceKvCancellationOutcome::Cancelled(cancelled) => cancelled,
            other => panic!("unexpected cancellation outcome: {other:?}"),
        };
        assert_eq!(cancelled.retire_all(request()).unwrap(), 2);
        let quiescent = cancelled
            .quiesce(ExactCompletion::from_contracted_hsa_quiescence(
                CompletionEpoch::new(22),
            ))
            .unwrap();
        let (mut settled, _completion) = quiescent.into_threaded_parts();
        assert!(settled.release_state_is_valid());

        let draft = settled.take_retired_pages(Qwen3ModelRole::Draft06B);
        let target = settled.take_retired_pages(Qwen3ModelRole::Target8B);
        assert_eq!(draft.len(), 1);
        assert_eq!(target.len(), 1);
        assert_eq!(draft[0].lease().allocation_id(), identity(62));
        assert_eq!(target[0].lease().allocation_id(), identity(61));
        let projection = settled.projection();
        assert_eq!(projection.draft_retired_pages, 0);
        assert_eq!(projection.target_retired_pages, 0);
        assert_eq!(projection.draft_arena_allocation_id, None);
        assert_eq!(projection.target_arena_allocation_id, None);
    }

    #[test]
    fn qualification_reserve_holds_exact_s1_s8_s32_page_rosters_in_pop_order() {
        for grouping in [
            M1QualificationLaneGrouping::S1,
            M1QualificationLaneGrouping::S8,
            M1QualificationLaneGrouping::S32,
        ] {
            let (plan, expected) = qualification_plan(grouping);
            let validated =
                crate::validate_m1_qualification_context_plan_v1(&plan, grouping, &expected)
                    .unwrap();
            for lane in 0..grouping.sequences() {
                let lane_request = RequestId::new(lane, 11);
                let context = validated.step(0, lane).unwrap();
                let mut cache = qualification_cache(lane_request, grouping);
                install_qualification_reserve_for_test(&mut cache, context, 71);
                let reserve = cache.qualification_target_page_reserve().unwrap();
                assert_eq!(reserve.request(), lane_request);
                assert_eq!(reserve.lane().lane_ordinal, lane);
                assert_eq!(
                    reserve.unused_page_count(),
                    M1_QUALIFICATION_TARGET_PAGE_COUNT_V1
                );
                assert!(reserve.ordered_state_is_valid());
                assert_eq!(reserve.unused_pages.last().unwrap().page().index(), 0);
                assert_eq!(reserve.unused_pages.first().unwrap().page().index(), 511);
            }
        }
    }

    #[test]
    fn qualification_partial_acquisition_failures_retain_exact_retry_prefix() {
        let caches: Vec<_> = (0..8)
            .map(|lane| {
                qualification_cache(RequestId::new(lane, 12), M1QualificationLaneGrouping::S8)
            })
            .collect();
        let mut pages_by_lane: Vec<Vec<DeviceKvPageLease>> = (0..8)
            .map(|_| Vec::with_capacity(M1_QUALIFICATION_TARGET_PAGE_COUNT_V1))
            .collect();

        let first = acquire_qualification_target_page_prefix(
            &caches,
            &mut pages_by_lane,
            |request, page| {
                if request.slot() == 0 && page == 37 {
                    Err(())
                } else {
                    Ok(qualification_page_lease(request, page, 76))
                }
            },
        );
        assert_eq!(first, Err((0, 37, ())));
        assert_eq!(pages_by_lane[0].len(), 37);
        assert!(pages_by_lane[1..].iter().all(Vec::is_empty));
        assert_eq!(
            qualification_target_page_prelease_progress(&pages_by_lane),
            M1QualificationTargetPagePreleaseProgressV1 { lane: 0, page: 37 }
        );

        let second = acquire_qualification_target_page_prefix(
            &caches,
            &mut pages_by_lane,
            |request, page| {
                if request.slot() == 3 && page == 41 {
                    Err(())
                } else {
                    Ok(qualification_page_lease(request, page, 76))
                }
            },
        );
        assert_eq!(second, Err((3, 41, ())));
        assert!(pages_by_lane[..3]
            .iter()
            .all(|pages| pages.len() == M1_QUALIFICATION_TARGET_PAGE_COUNT_V1));
        assert_eq!(pages_by_lane[3].len(), 41);
        assert!(pages_by_lane[4..].iter().all(Vec::is_empty));

        acquire_qualification_target_page_prefix(&caches, &mut pages_by_lane, |request, page| {
            Ok::<_, ()>(qualification_page_lease(request, page, 76))
        })
        .unwrap();
        assert!(pages_by_lane
            .iter()
            .all(|pages| pages.len() == M1_QUALIFICATION_TARGET_PAGE_COUNT_V1));
        for (lane, pages) in pages_by_lane.iter().enumerate() {
            assert!(pages.iter().enumerate().all(|(page, lease)| {
                lease.request() == RequestId::new(u32::try_from(lane).unwrap(), 12)
                    && lease.page().index() == u32::try_from(page).unwrap()
            }));
        }
    }

    #[test]
    fn qualification_step_rejects_request_lane_witness_and_ordinal_substitution() {
        let grouping = M1QualificationLaneGrouping::S8;
        let (plan, expected) = qualification_plan(grouping);
        let validated =
            crate::validate_m1_qualification_context_plan_v1(&plan, grouping, &expected).unwrap();
        let context0 = validated.step(0, 0).unwrap();
        let mut cache = qualification_cache(RequestId::new(0, 19), grouping);
        install_qualification_reserve_for_test(&mut cache, context0, 72);

        assert_eq!(
            cache
                .reserve_m1_qualification_context_step_write_v1(
                    RequestId::new(1, 19),
                    0,
                    context0,
                    CompletionEpoch::new(1),
                )
                .unwrap_err()
                .error(),
            DeviceKvCacheError::WrongRequest
        );
        assert_eq!(
            cache
                .reserve_m1_qualification_context_step_write_v1(
                    RequestId::new(0, 19),
                    0,
                    validated.step(0, 1).unwrap(),
                    CompletionEpoch::new(1),
                )
                .unwrap_err()
                .error(),
            DeviceKvCacheError::QualificationWitnessMismatch
        );
        assert_eq!(
            cache
                .reserve_m1_qualification_context_step_write_v1(
                    RequestId::new(0, 19),
                    0,
                    validated.step(16, 0).unwrap(),
                    CompletionEpoch::new(1),
                )
                .unwrap_err()
                .error(),
            DeviceKvCacheError::QualificationPageOrderMismatch
        );
        assert_eq!(
            cache.projection().target_qualification_future_pages,
            M1_QUALIFICATION_TARGET_PAGE_COUNT_V1
        );
        assert!(!cache.projection().target_write_pending);
    }

    #[test]
    fn qualification_boundary_failure_reinserts_page_and_preserves_conservation() {
        let grouping = M1QualificationLaneGrouping::S1;
        let (plan, expected) = qualification_plan(grouping);
        let validated =
            crate::validate_m1_qualification_context_plan_v1(&plan, grouping, &expected).unwrap();
        let context = validated.step(0, 0).unwrap();
        let mut cache = qualification_cache(request(), grouping);
        install_qualification_reserve_for_test(&mut cache, context, 73);

        let error = cache
            .reserve_m1_qualification_context_step_write_v1(
                request(),
                0,
                context,
                CompletionEpoch::new(0),
            )
            .unwrap_err();
        assert_eq!(error.error(), DeviceKvCacheError::ZeroCompletionEpoch);
        let projection = cache.projection();
        assert_eq!(projection.target_active_pages, 0);
        assert_eq!(
            projection.target_qualification_future_pages,
            M1_QUALIFICATION_TARGET_PAGE_COUNT_V1
        );
        assert!(!projection.target_write_pending);
        assert_eq!(
            cache
                .qualification_target_page_reserve()
                .unwrap()
                .unused_pages
                .last()
                .unwrap()
                .page()
                .index(),
            0
        );

        let pending = cache
            .reserve_m1_qualification_context_step_write_v1(
                request(),
                0,
                context,
                CompletionEpoch::new(1),
            )
            .unwrap();
        assert_eq!(pending.active_page_count(), 0);
        assert_eq!(pending.pending_new_page_count(), 1);
        assert_eq!(pending.unused_future_page_count(), 511);
        assert!(pending.conserves_target_pages());
        let aborted = cache
            .abort_step_write(pending.into_pending_step_write())
            .unwrap();
        cache
            .common
            .target_qualification_reserve
            .as_mut()
            .unwrap()
            .unused_pages
            .extend(aborted.into_page_leases());
        assert_eq!(
            cache.projection().target_qualification_future_pages,
            M1_QUALIFICATION_TARGET_PAGE_COUNT_V1
        );
    }

    #[test]
    fn qualification_future_pages_block_rollback_cancel_and_release() {
        let grouping = M1QualificationLaneGrouping::S1;
        let (plan, expected) = qualification_plan(grouping);
        let validated =
            crate::validate_m1_qualification_context_plan_v1(&plan, grouping, &expected).unwrap();
        let context = validated.step(0, 0).unwrap();
        let mut cache = qualification_cache(request(), grouping);
        install_qualification_reserve_for_test(&mut cache, context, 74);
        assert!(!cache.release_state_is_valid());
        assert_eq!(
            cache.rollback_one(request(), Qwen3ModelRole::Target8B, CompletionEpoch::new(1),),
            Err(DeviceKvCacheError::QualificationFuturePagesRemain)
        );
        let failure = match cache.cancel(request(), CompletionEpoch::new(1)) {
            DeviceKvCancellationOutcome::Rejected(failure) => failure,
            other => panic!("unexpected cancellation outcome: {other:?}"),
        };
        assert_eq!(
            failure.error(),
            DeviceKvCacheError::QualificationFuturePagesRemain
        );
        let (_, cache) = failure.into_parts();
        assert_eq!(
            cache.projection().target_qualification_future_pages,
            M1_QUALIFICATION_TARGET_PAGE_COUNT_V1
        );
    }

    #[test]
    fn qualification_s1_full_context_consumes_exact_boundary_pages() {
        let grouping = M1QualificationLaneGrouping::S1;
        let (plan, expected) = qualification_plan(grouping);
        let validated =
            crate::validate_m1_qualification_context_plan_v1(&plan, grouping, &expected).unwrap();
        let mut cache = qualification_cache(request(), grouping);
        install_qualification_reserve_for_test(&mut cache, validated.step(0, 0).unwrap(), 75);

        for ordinal in 0..8_192u32 {
            let epoch = CompletionEpoch::new(u64::from(ordinal) + 1);
            let qualified = cache
                .reserve_m1_qualification_context_step_write_v1(
                    request(),
                    0,
                    validated.step(ordinal, 0).unwrap(),
                    epoch,
                )
                .unwrap();
            let expected_active = usize::try_from(ordinal.div_ceil(16)).unwrap();
            let expected_pending = usize::from(ordinal.is_multiple_of(16));
            assert_eq!(qualified.active_page_count(), expected_active);
            assert_eq!(qualified.pending_new_page_count(), expected_pending);
            assert!(qualified.conserves_target_pages());
            if matches!(ordinal, 0 | 15 | 16 | 8_176 | 8_191) {
                assert_eq!(
                    qualified.unused_future_page_count(),
                    512 - expected_active - expected_pending
                );
            }

            let completed =
                match complete_step(cache, qualified.into_pending_step_write(), epoch.value()) {
                    DeviceKvStepCompletionOutcome::Completed(completed) => completed,
                    other => panic!("unexpected qualification completion outcome: {other:?}"),
                };
            let (mut completed_cache, initialized, _completion) = completed.into_parts();
            assert_eq!(
                completed_cache
                    .settle_completed_step(&initialized, 1, epoch)
                    .unwrap(),
                0
            );
            cache = completed_cache;
        }

        let projection = cache.projection();
        assert_eq!(projection.target.committed_tokens, 8_192);
        assert_eq!(projection.target.resident_tokens, 8_192);
        assert_eq!(projection.target_active_pages, 512);
        assert_eq!(projection.target_qualification_future_pages, 0);
        assert!(cache.release_state_is_valid());
    }
}
