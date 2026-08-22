//! Linear engine custody for one request's target and draft paged KV.
//!
//! This module binds the verified source-level [`ferric_spec::PhysicalKvState`]
//! transitions to non-clone engine typestates. A page lease is retained in the
//! cache while its physical identity is reachable or retired. An initialized
//! prefix advances only after a crate-owned pending write is paired with an
//! [`ExactCompletion`] for the same epoch.
//!
//! The types below own no KFD allocation, GPU address, page contents, queue,
//! packet, signal, or hardware observation. There are deliberately no
//! production constructors for [`DeviceKvPageLease`] or
//! [`InitializedDeviceKvWrite`]: fe2o3 allocation authority and exact packet,
//! buffer, and KV-write-effect authority do not exist yet. Unit tests use
//! scoped stand-ins. Quiescent caches retain page custody and expose no release
//! or reuse operation. This foundation also does not implement the future
//! fan-out/composition that must derive scheduler, KV, and resource permits
//! from one ordered queue completion without duplicating linear authority.

use crate::ExactCompletion;
use ferric_spec::completion::CompletionEpoch;
use ferric_spec::{
    append_physical_page, cancel_physical_kv, commit_physical_kv, map_initialized_token,
    retire_cancelled_tail, rollback_physical_token, write_physical_token, Identity, LogicalKvState,
    PhysicalKvError, PhysicalKvLifecycle, PhysicalKvLocation, PhysicalKvState, PhysicalPageId,
    Qwen3ModelRole, Qwen3PlanSelection, RequestId, Target, M1_KV_PAGE_TABLE_ENTRIES,
    M1_KV_PAGE_TOKENS,
};
use vstd::prelude::*;

/// Exact processor declaration admitted by the M1 generated-runner template.
pub const GFX942_PROCESSOR: &str = "gfx942";
/// Exact target-feature declaration admitted by the M1 generated-runner template.
pub const GFX942_TARGET_FEATURES: &str = "+wavefrontsize64,-xnack";

verus! {

/// Identity-only binding for one declared M1 device.
///
/// This value is copyable because it owns no device resource or observation.
/// Successful construction validates declaration bytes, not real hardware.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gfx942DeviceBinding {
    device_id: Identity,
    node_id: u32,
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

closed spec fn exact_device_binding(
    binding: Gfx942DeviceBinding,
    device_id: Identity,
    node_id: u32,
) -> bool {
    binding.device_id == device_id
        && binding.node_id == node_id
        && binding.target == Target::Gfx942XnackMinus
}

fn exact_binding(device_id: Identity, node_id: u32) -> (binding: Gfx942DeviceBinding)
    ensures exact_device_binding(binding, device_id, node_id),
{
    proof { reveal(exact_device_binding); }
    Gfx942DeviceBinding {
        device_id,
        node_id,
        target: Target::Gfx942XnackMinus,
    }
}

} // verus!

/// Validates and retains the exact single-device gfx942 declaration.
///
/// This function does not inspect a machine or authenticate `device_id`.
///
/// # Errors
///
/// Rejects an absent identity or any processor/feature byte drift.
pub fn bind_gfx942_device(
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
    Ok(exact_binding(device_id, node_id))
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
    WriteGenerationExhausted,
    ZeroCompletionEpoch,
    CompletionEpochMismatch,
    NoRetiredPageAtEpoch,
    UnsettledPriorRetirement,
    OwnedPageTableDrift,
    ActivePagesRemain,
    Physical(PhysicalKvError),
}

impl From<PhysicalKvError> for DeviceKvCacheError {
    fn from(error: PhysicalKvError) -> Self {
        Self::Physical(error)
    }
}

/// Linear custody of one page subrange in a contracted role-scoped arena.
///
/// Fields and construction are crate-private. The future physical runner must
/// split one fe2o3 arena authority into disjoint page subleases before using
/// the integration constructor. Multiple page leases for one role retain the
/// same arena allocation identity; this source-only token is not allocation or
/// subrange evidence.
#[derive(Debug, PartialEq, Eq)]
pub struct DeviceKvPageLease {
    device: Gfx942DeviceBinding,
    allocation_id: Identity,
    page: PhysicalPageId,
}

impl DeviceKvPageLease {
    /// Returns the inert allocation identity without exposing an address.
    #[must_use]
    pub const fn allocation_id(&self) -> Identity {
        self.allocation_id
    }

    /// Returns the exact role-scoped physical page generation.
    #[must_use]
    pub const fn page(&self) -> PhysicalPageId {
        self.page
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

/// Non-clone request to initialize exactly the next token in one owned page.
///
/// Preparing this value does not initialize memory or advance the cache.
#[derive(Debug, PartialEq, Eq)]
pub struct PendingDeviceKvWrite {
    binding: PendingWriteBinding,
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
}

#[derive(Debug, PartialEq, Eq)]
struct RetiredPageLease {
    lease: DeviceKvPageLease,
    after_epoch: CompletionEpoch,
    quiescent: bool,
}

#[derive(Debug, PartialEq, Eq)]
struct RoleDeviceKvCache {
    physical: PhysicalKvState,
    arena_allocation_id: Option<Identity>,
    active_pages: Vec<DeviceKvPageLease>,
    retired_pages: Vec<RetiredPageLease>,
    pending: Option<PendingWriteBinding>,
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

    fn settle_retired_epoch(
        &mut self,
        completion: ExactCompletion,
    ) -> Result<usize, RetirementCompletionFailure> {
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
        Ok(matching)
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
            },
        })
    }

    #[must_use]
    pub fn projection(&self) -> DeviceKvCacheProjection {
        self.common.projection()
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
        let error = if self.common.request != request {
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
        cache.pending = Some(binding);
        Ok(PendingDeviceKvWrite { binding })
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
            } else if cache.pending != Some(initialized.binding) {
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
    ) -> Result<usize, RetirementCompletionFailure> {
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
    ) -> Result<usize, RetirementCompletionFailure> {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferric_spec::{Qwen3ExecutionMode, Qwen3PlanBucket, M1_KV_PHYSICAL_PAGE_SLOTS};

    impl DeviceKvPageLease {
        fn from_contracted_gfx942_allocation(
            device: Gfx942DeviceBinding,
            allocation_id: Identity,
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
                page,
            })
        }
    }

    impl PendingDeviceKvWrite {
        fn complete_for_test(
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

    const fn selection(role: Qwen3ModelRole) -> Qwen3PlanSelection {
        Qwen3PlanSelection {
            role,
            mode: Qwen3ExecutionMode::Speculative,
            bucket: Qwen3PlanBucket::SpeculativeS1K4C8192,
        }
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
            PhysicalPageId::new(role, index, 1),
        )
        .unwrap()
    }

    fn complete(
        pending: PendingDeviceKvWrite,
        epoch: u64,
    ) -> Result<InitializedDeviceKvWrite, Box<PendingWriteCompletionFailure>> {
        pending.complete_for_test(ExactCompletion::from_contracted_hsa_quiescence(
            CompletionEpoch::new(epoch),
        ))
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
        assert_eq!(
            cancelled
                .settle_retired_epoch(ExactCompletion::from_contracted_hsa_quiescence(
                    CompletionEpoch::new(11),
                ))
                .unwrap(),
            1
        );
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
}
