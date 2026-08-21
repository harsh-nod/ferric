//! Logical physical-paged-KV refinement for one admitted Qwen3 request.
//!
//! The fixed metadata below models exact page-table translation, initialized
//! prefixes, generations, rollback, cancellation, retirement, and reuse. It is
//! a source-level sequential contract only. It owns no device allocation,
//! address, copy, kernel, queue, completion, HSA, or machine refinement.

use crate::completion::CompletionEpoch;
use crate::{Qwen3ModelRole, Qwen3PlanSelection, RequestId, M1_MAX_CONTEXT_TOKENS};
use vstd::prelude::*;

verus! {

/// Ferric M0 and the admitted M1 graph use 16-token KV pages.
pub const M1_KV_PAGE_TOKENS: u32 = 16;
/// One 8K request needs exactly 512 logical page-table entries.
pub const M1_KV_PAGE_TABLE_ENTRIES: usize = 512;
/// This one-request refinement state has one physical slot per table entry.
pub const M1_KV_PHYSICAL_PAGE_SLOTS: usize = 512;

const M1_KV_PAGE_TABLE_ENTRIES_U32: u32 = 512;
const M1_KV_PHYSICAL_PAGE_SLOTS_U32: u32 = 512;

/// A physical page identity is scoped to an exact target or draft pool.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhysicalPageId {
    role: Qwen3ModelRole,
    index: u32,
    generation: u32,
}

impl PhysicalPageId {
    pub closed spec fn role_spec(&self) -> Qwen3ModelRole { self.role }

    pub closed spec fn index_spec(&self) -> u32 { self.index }

    pub closed spec fn generation_spec(&self) -> u32 { self.generation }

    #[must_use]
    pub const fn new(role: Qwen3ModelRole, index: u32, generation: u32) -> Self {
        Self { role, index, generation }
    }

    #[must_use]
    pub const fn role(self) -> (role: Qwen3ModelRole)
        ensures role == self.role_spec(),
    { self.role }

    #[must_use]
    pub const fn index(self) -> (index: u32)
        ensures index == self.index_spec(),
    { self.index }

    #[must_use]
    pub const fn generation(self) -> (generation: u32)
        ensures generation == self.generation_spec(),
    { self.generation }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PhysicalPageOwnership {
    Free,
    Exclusive { request: RequestId, role: Qwen3ModelRole },
    Retired {
        request: RequestId,
        role: Qwen3ModelRole,
        after_epoch: CompletionEpoch,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PhysicalPageSlot {
    generation: u32,
    ownership: PhysicalPageOwnership,
    initialized_prefix: u32,
}

impl PhysicalPageSlot {
    const FREE: Self = Self {
        generation: 1,
        ownership: PhysicalPageOwnership::Free,
        initialized_prefix: 0,
    };
}

/// Logical request visibility around cancellation and retirement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicalKvLifecycle {
    Active,
    Cancelled { after_epoch: CompletionEpoch },
    RetiredAwaitingQuiescence { after_epoch: CompletionEpoch },
}

/// The abstract M0-compatible token state refined by physical metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LogicalKvState {
    pub request: RequestId,
    pub role: Qwen3ModelRole,
    pub lifecycle: PhysicalKvLifecycle,
    pub resident_tokens: u32,
    pub committed_tokens: u32,
}

/// Exact translation of one initialized logical token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhysicalKvLocation {
    pub page: PhysicalPageId,
    pub offset: u32,
}

/// Fail-closed rejection for the physical metadata contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicalKvError {
    InvalidSelection,
    ZeroRequestGeneration,
    WrongLifecycle,
    RequestMismatch,
    SelectionMismatch,
    RoleMismatch,
    PageOutOfRange,
    PageGenerationMismatch,
    PageNotFree,
    PhysicalAlias,
    PageNotRequired,
    PageTableExhausted,
    ContextExceeded,
    LogicalPositionMismatch,
    LogicalPositionOutOfRange,
    MissingPage,
    PageOwnershipMismatch,
    UninitializedRead,
    CommitExceedsResident,
    NoTentativeToken,
    SettlementCursorMismatch,
    SettlementIntervalMismatch,
    SettlementTailTooWide,
    SettlementPageCountMismatch,
    SettlementPhysicalAlias,
    ZeroRetirementEpoch,
    RetirementEpochMismatch,
    NoPageToRetire,
    GenerationExhausted,
    InvalidQuiescenceAuthority,
}

/// Private, non-clone authority. The crate-local logical composition produces
/// this value only after observing an exact scheduler completion; no public
/// source-level constructor exists.
#[derive(Debug, PartialEq, Eq)]
pub struct KvQuiescenceAuthority {
    request: RequestId,
    role: Qwen3ModelRole,
    exact_epoch: CompletionEpoch,
}

/// Private checked authority for one infallible speculative tail settlement.
/// The token is non-clone and its fields have no public constructor.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct PhysicalKvSettlementPermit {
    request: RequestId,
    selection: Qwen3PlanSelection,
    after_epoch: CompletionEpoch,
    pre_committed: u32,
    tentative_end: u32,
    commit_end: u32,
    old_page_count: u32,
    next_page_count: u32,
    retired_pages: u32,
    retired_page: Option<PhysicalPageId>,
    retained_page: Option<PhysicalPageId>,
}

impl KvQuiescenceAuthority {
    pub(crate) closed spec fn request_spec(&self) -> RequestId { self.request }

    pub(crate) closed spec fn role_spec(&self) -> Qwen3ModelRole { self.role }

    pub(crate) closed spec fn exact_epoch_spec(&self) -> CompletionEpoch { self.exact_epoch }

    pub(crate) fn from_exact_completion(
        request: RequestId,
        role: Qwen3ModelRole,
        exact_epoch: CompletionEpoch,
    ) -> (authority: Self)
        requires exact_epoch.value > 0,
        ensures
            authority.request_spec() == request,
            authority.role_spec() == role,
            authority.exact_epoch_spec() == exact_epoch,
    {
        Self { request, role, exact_epoch }
    }
}

/// Fixed-capacity physical metadata for exactly one admitted request and role.
/// Fields are private and the authority is deliberately not `Clone`.
#[derive(Debug, PartialEq, Eq)]
pub struct PhysicalKvState {
    request: RequestId,
    selection: Qwen3PlanSelection,
    lifecycle: PhysicalKvLifecycle,
    max_context_tokens: u32,
    resident_tokens: u32,
    committed_tokens: u32,
    page_count: u32,
    page_table: [Option<PhysicalPageId>; M1_KV_PAGE_TABLE_ENTRIES],
    page_slots: [PhysicalPageSlot; M1_KV_PHYSICAL_PAGE_SLOTS],
}

pub closed spec fn same_request(left: RequestId, right: RequestId) -> bool {
    left.slot_spec() == right.slot_spec()
        && left.generation_spec() == right.generation_spec()
}

pub closed spec fn role_matches(left: Qwen3ModelRole, right: Qwen3ModelRole) -> bool {
    match (left, right) {
        (Qwen3ModelRole::Target8B, Qwen3ModelRole::Target8B)
        | (Qwen3ModelRole::Draft06B, Qwen3ModelRole::Draft06B) => true,
        _ => false,
    }
}

pub closed spec fn lifecycle_matches(
    left: PhysicalKvLifecycle,
    right: PhysicalKvLifecycle,
) -> bool {
    match (left, right) {
        (PhysicalKvLifecycle::Active, PhysicalKvLifecycle::Active) => true,
        (
            PhysicalKvLifecycle::Cancelled { after_epoch: left_epoch },
            PhysicalKvLifecycle::Cancelled { after_epoch: right_epoch },
        ) => left_epoch.value == right_epoch.value,
        (
            PhysicalKvLifecycle::RetiredAwaitingQuiescence { after_epoch: left_epoch },
            PhysicalKvLifecycle::RetiredAwaitingQuiescence { after_epoch: right_epoch },
        ) => left_epoch.value == right_epoch.value,
        _ => false,
    }
}

pub closed spec fn kv_selection_valid(selection: Qwen3PlanSelection) -> bool {
    selection
        .bucket
        .dimensions_spec(selection.role, selection.mode)
        .is_some()
}

impl PhysicalKvState {
    pub closed spec fn abstraction_spec(&self) -> LogicalKvState {
        LogicalKvState {
            request: self.request,
            role: self.selection.role,
            lifecycle: self.lifecycle,
            resident_tokens: self.resident_tokens,
            committed_tokens: self.committed_tokens,
        }
    }

    pub closed spec fn selection_spec(&self) -> Qwen3PlanSelection { self.selection }

    pub closed spec fn table_contains_index_spec(&self, index: u32) -> bool {
        exists|position: int| 0 <= position < M1_KV_PAGE_TABLE_ENTRIES
            && self.page_table@[position].is_some()
            && self.page_table@[position].unwrap().index == index
    }

    pub closed spec fn immutable_frame(&self, before: &Self) -> bool {
        self.request == before.request
            && self.selection == before.selection
            && self.max_context_tokens == before.max_context_tokens
    }

    pub closed spec fn initial_refinement(
        &self,
        request: RequestId,
        selection: Qwen3PlanSelection,
        max_context_tokens: u32,
    ) -> bool {
        &&& self.request == request
        &&& self.selection == selection
        &&& lifecycle_matches(self.lifecycle, PhysicalKvLifecycle::Active)
        &&& self.max_context_tokens == max_context_tokens
        &&& self.resident_tokens == 0
        &&& self.committed_tokens == 0
        &&& self.page_count == 0
        &&& forall|position: int| 0 <= position < M1_KV_PAGE_TABLE_ENTRIES ==>
            self.page_table@[position].is_none()
        &&& forall|position: int| 0 <= position < M1_KV_PHYSICAL_PAGE_SLOTS ==>
            self.page_slots@[position] == PhysicalPageSlot::FREE
    }

    /// Constructs the only public initial state for an exact admitted graph bucket.
    ///
    /// # Errors
    ///
    /// Rejects invalid role/mode/bucket selections and zero request generations.
    pub fn new(
        request: RequestId,
        selection: Qwen3PlanSelection,
    ) -> (result: Result<Self, PhysicalKvError>)
        ensures match result {
            Ok(state) => {
                &&& request.generation_spec() > 0
                &&& kv_selection_valid(selection)
                &&& state.selection_spec() == selection
                &&& state.abstraction_spec().request.slot_spec() == request.slot_spec()
                &&& state.abstraction_spec().request.generation_spec()
                    == request.generation_spec()
                &&& state.abstraction_spec().role == selection.role
                &&& state.abstraction_spec().lifecycle == PhysicalKvLifecycle::Active
                &&& state.abstraction_spec().resident_tokens == 0
                &&& state.abstraction_spec().committed_tokens == 0
                &&& state.initial_refinement(
                    request,
                    selection,
                    selection.bucket.dimensions_spec(selection.role, selection.mode).unwrap().context_tokens,
                )
            }
            Err(PhysicalKvError::ZeroRequestGeneration) => request.generation_spec() == 0,
            Err(PhysicalKvError::InvalidSelection) => {
                request.generation_spec() > 0 && !kv_selection_valid(selection)
            }
            Err(_) => false,
        },
    {
        if request.generation() == 0 {
            return Err(PhysicalKvError::ZeroRequestGeneration);
        }
        let Some(dimensions) = selection.bucket.dimensions(selection.role, selection.mode) else {
            return Err(PhysicalKvError::InvalidSelection);
        };
        let page_table = vstd::array::array_fill_for_copy_types(None);
        let page_slots = vstd::array::array_fill_for_copy_types(PhysicalPageSlot::FREE);
        let state = Self {
            request,
            selection,
            lifecycle: PhysicalKvLifecycle::Active,
            max_context_tokens: dimensions.context_tokens,
            resident_tokens: 0,
            committed_tokens: 0,
            page_count: 0,
            page_table,
            page_slots,
        };
        proof {
            reveal(kv_selection_valid);
            reveal(PhysicalKvState::initial_refinement);
            reveal(lifecycle_matches);
        }
        Ok(state)
    }

    #[must_use]
    pub const fn logical_state(&self) -> (state: LogicalKvState)
        ensures state == self.abstraction_spec(),
    {
        LogicalKvState {
            request: self.request,
            role: self.selection.role,
            lifecycle: self.lifecycle,
            resident_tokens: self.resident_tokens,
            committed_tokens: self.committed_tokens,
        }
    }

    #[must_use]
    pub const fn selection(&self) -> (selection: Qwen3PlanSelection)
        ensures selection == self.selection_spec(),
    { self.selection }

    #[must_use]
    pub const fn max_context_tokens(&self) -> u32 { self.max_context_tokens }

    #[must_use]
    pub const fn page_count(&self) -> u32 { self.page_count }

    pub closed spec fn page_at_spec(&self, position: u32) -> Option<PhysicalPageId> {
        if position < M1_KV_PAGE_TABLE_ENTRIES {
            self.page_table@[position as int]
        } else {
            None
        }
    }

    /// Returns the exact physical generation retained at one logical page-table position.
    #[must_use]
    pub fn page_at(&self, position: u32) -> (page: Option<PhysicalPageId>)
        ensures page == self.page_at_spec(position),
    {
        if position >= M1_KV_PAGE_TABLE_ENTRIES_U32 {
            None
        } else {
            self.page_table[position as usize]
        }
    }

    #[must_use]
    pub fn page_generation(&self, index: u32) -> Option<u32> {
        if index >= M1_KV_PHYSICAL_PAGE_SLOTS_U32 {
            None
        } else {
            Some(self.page_slots[index as usize].generation)
        }
    }

    fn table_contains_index(&self, index: u32) -> (contains: bool)
        ensures contains == self.table_contains_index_spec(index),
    {
        let mut position = 0usize;
        while position < M1_KV_PAGE_TABLE_ENTRIES
            invariant
                position <= M1_KV_PAGE_TABLE_ENTRIES,
                forall|prior: int| 0 <= prior < position ==>
                    !(self.page_table@[prior].is_some()
                        && self.page_table@[prior].unwrap().index == index),
            decreases M1_KV_PAGE_TABLE_ENTRIES - position,
        {
            if let Some(page) = self.page_table[position] {
                if page.index == index {
                    return true;
                }
            }
            position += 1;
        }
        false
    }
}

fn same_role(left: Qwen3ModelRole, right: Qwen3ModelRole) -> (same: bool)
    ensures same == role_matches(left, right),
{
    matches!((left, right),
        (Qwen3ModelRole::Target8B, Qwen3ModelRole::Target8B)
            | (Qwen3ModelRole::Draft06B, Qwen3ModelRole::Draft06B)
    )
}

fn is_active(lifecycle: PhysicalKvLifecycle) -> (active: bool)
    ensures active == lifecycle_matches(lifecycle, PhysicalKvLifecycle::Active),
{
    matches!(lifecycle, PhysicalKvLifecycle::Active)
}

closed spec fn free_slot_matches(slot: PhysicalPageSlot, page: PhysicalPageId) -> bool {
    page.generation > 0
        && slot.generation == page.generation
        && match slot.ownership {
            PhysicalPageOwnership::Free => slot.initialized_prefix == 0,
            _ => false,
        }
}

fn is_free_slot(slot: PhysicalPageSlot, page: PhysicalPageId) -> (available: bool)
    ensures available == free_slot_matches(slot, page),
{
    page.generation > 0
        && slot.generation == page.generation
        && matches!(slot.ownership, PhysicalPageOwnership::Free)
        && slot.initialized_prefix == 0
}

closed spec fn exclusive_owner_matches(
    ownership: PhysicalPageOwnership,
    request: RequestId,
    role: Qwen3ModelRole,
) -> bool {
    match ownership {
        PhysicalPageOwnership::Exclusive {
            request: owner,
            role: owner_role,
        } => same_request(owner, request) && role_matches(owner_role, role),
        _ => false,
    }
}

fn is_exclusive_owner(
    ownership: PhysicalPageOwnership,
    request: RequestId,
    role: Qwen3ModelRole,
) -> (matches: bool)
    ensures matches == exclusive_owner_matches(ownership, request, role),
{
    match ownership {
        PhysicalPageOwnership::Exclusive {
            request: owner,
            role: owner_role,
        } => {
            owner.slot() == request.slot()
                && owner.generation() == request.generation()
                && same_role(owner_role, role)
        }
        _ => false,
    }
}

closed spec fn retired_owner_matches(
    ownership: PhysicalPageOwnership,
    request: RequestId,
    role: Qwen3ModelRole,
    exact_epoch: CompletionEpoch,
) -> bool {
    match ownership {
        PhysicalPageOwnership::Retired {
            request: owner,
            role: owner_role,
            after_epoch,
        } => {
            same_request(owner, request)
                && role_matches(owner_role, role)
                && after_epoch.value == exact_epoch.value
        }
        _ => false,
    }
}

fn is_retired_owner(
    ownership: PhysicalPageOwnership,
    request: RequestId,
    role: Qwen3ModelRole,
    exact_epoch: CompletionEpoch,
) -> (matches: bool)
    ensures matches == retired_owner_matches(ownership, request, role, exact_epoch),
{
    match ownership {
        PhysicalPageOwnership::Retired {
            request: owner,
            role: owner_role,
            after_epoch,
        } => {
            owner.slot() == request.slot()
                && owner.generation() == request.generation()
                && same_role(owner_role, role)
                && after_epoch.value == exact_epoch.value
        }
        _ => false,
    }
}

fn validate_active_authority(
    state: &PhysicalKvState,
    request: RequestId,
    selection: Qwen3PlanSelection,
) -> (result: Result<(), PhysicalKvError>)
    ensures result.is_ok() == (
        lifecycle_matches(state.lifecycle, PhysicalKvLifecycle::Active)
            && same_request(state.request, request)
            && state.selection == selection
    ),
{
    proof {
        reveal(lifecycle_matches);
        reveal(same_request);
    }
    if !is_active(state.lifecycle) {
        return Err(PhysicalKvError::WrongLifecycle);
    }
    if state.request.slot() != request.slot()
        || state.request.generation() != request.generation()
    {
        return Err(PhysicalKvError::RequestMismatch);
    }
    if !state.selection.matches(selection) {
        return Err(PhysicalKvError::SelectionMismatch);
    }
    Ok(())
}

pub closed spec fn append_page_enabled(
    state: &PhysicalKvState,
    request: RequestId,
    selection: Qwen3PlanSelection,
    page: PhysicalPageId,
) -> bool {
    &&& lifecycle_matches(state.lifecycle, PhysicalKvLifecycle::Active)
    &&& same_request(state.request, request)
    &&& state.selection == selection
    &&& role_matches(page.role, state.selection.role)
    &&& page.index < M1_KV_PHYSICAL_PAGE_SLOTS
    &&& state.resident_tokens < state.max_context_tokens
    &&& state.resident_tokens % M1_KV_PAGE_TOKENS == 0
    &&& state.page_count == state.resident_tokens / M1_KV_PAGE_TOKENS
    &&& state.page_count < M1_KV_PAGE_TABLE_ENTRIES
    &&& !state.table_contains_index_spec(page.index)
    &&& free_slot_matches(state.page_slots@[page.index as int], page)
}

pub closed spec fn append_page_transition(
    before: &PhysicalKvState,
    after: &PhysicalKvState,
    page: PhysicalPageId,
) -> bool {
    &&& after.immutable_frame(before)
    &&& after.lifecycle == before.lifecycle
    &&& after.resident_tokens == before.resident_tokens
    &&& after.committed_tokens == before.committed_tokens
    &&& after.page_count == before.page_count + 1
    &&& after.page_table@ == before.page_table@.update(before.page_count as int, Some(page))
    &&& after.page_slots@ == before.page_slots@.update(
        page.index as int,
        PhysicalPageSlot {
            generation: page.generation,
            ownership: PhysicalPageOwnership::Exclusive {
                request: before.request,
                role: before.selection.role,
            },
            initialized_prefix: 0,
        },
    )
}

/// Binds the next logical page to an exact free physical generation.
///
/// # Errors
///
/// Rejects stale identities, cross-role pages, aliases, non-boundary appends,
/// exhausted tables, and non-free slots without mutation.
pub fn append_physical_page(
    state: &mut PhysicalKvState,
    request: RequestId,
    selection: Qwen3PlanSelection,
    page: PhysicalPageId,
) -> (result: Result<(), PhysicalKvError>)
    ensures
        result.is_ok() == append_page_enabled(old(state), request, selection, page),
        result.is_ok() ==> append_page_transition(old(state), final(state), page),
        result.is_err() ==> *final(state) == *old(state),
{
    let ghost entry = *state;
    assert(entry == *old(state));
    proof {
        reveal(append_page_enabled);
        reveal(append_page_transition);
        reveal(PhysicalKvState::immutable_frame);
    }
    validate_active_authority(state, request, selection)?;
    if !same_role(page.role, state.selection.role) {
        return Err(PhysicalKvError::RoleMismatch);
    }
    if page.index >= M1_KV_PHYSICAL_PAGE_SLOTS_U32 {
        return Err(PhysicalKvError::PageOutOfRange);
    }
    if state.resident_tokens >= state.max_context_tokens {
        return Err(PhysicalKvError::ContextExceeded);
    }
    if !state.resident_tokens.is_multiple_of(M1_KV_PAGE_TOKENS)
        || state.page_count != state.resident_tokens / M1_KV_PAGE_TOKENS
    {
        return Err(PhysicalKvError::PageNotRequired);
    }
    if state.page_count >= M1_KV_PAGE_TABLE_ENTRIES_U32 {
        return Err(PhysicalKvError::PageTableExhausted);
    }
    if state.table_contains_index(page.index) {
        return Err(PhysicalKvError::PhysicalAlias);
    }
    let slot = state.page_slots[page.index as usize];
    if !is_free_slot(slot, page) {
        if page.generation == 0 || slot.generation != page.generation {
            return Err(PhysicalKvError::PageGenerationMismatch);
        }
        return Err(PhysicalKvError::PageNotFree);
    }
    assert(append_page_enabled(&entry, request, selection, page));
    let table_position = state.page_count as usize;
    state.page_table[table_position] = Some(page);
    state.page_slots[page.index as usize] = PhysicalPageSlot {
        generation: page.generation,
        ownership: PhysicalPageOwnership::Exclusive {
            request: state.request,
            role: state.selection.role,
        },
        initialized_prefix: 0,
    };
    state.page_count += 1;
    assert(append_page_transition(&entry, state, page));
    Ok(())
}

pub closed spec fn map_initialized_decision(
    state: &PhysicalKvState,
    request: RequestId,
    selection: Qwen3PlanSelection,
    logical_position: u32,
) -> Result<PhysicalKvLocation, PhysicalKvError> {
    if !lifecycle_matches(state.lifecycle, PhysicalKvLifecycle::Active) {
        Err(PhysicalKvError::WrongLifecycle)
    } else if !same_request(state.request, request) {
        Err(PhysicalKvError::RequestMismatch)
    } else if state.selection != selection {
        Err(PhysicalKvError::SelectionMismatch)
    } else if logical_position >= state.resident_tokens {
        Err(PhysicalKvError::LogicalPositionOutOfRange)
    } else {
        let logical_page = logical_position / M1_KV_PAGE_TOKENS;
        let offset = logical_position % M1_KV_PAGE_TOKENS;
        if logical_page >= state.page_count || logical_page >= M1_KV_PAGE_TABLE_ENTRIES {
            Err(PhysicalKvError::MissingPage)
        } else if state.page_table@[logical_page as int].is_none() {
            Err(PhysicalKvError::MissingPage)
        } else {
            let page = state.page_table@[logical_page as int].unwrap();
            if !role_matches(page.role, state.selection.role) {
                Err(PhysicalKvError::RoleMismatch)
            } else if page.index >= M1_KV_PHYSICAL_PAGE_SLOTS {
                Err(PhysicalKvError::PageOutOfRange)
            } else {
                let slot = state.page_slots@[page.index as int];
                if page.generation == 0 || slot.generation != page.generation {
                    Err(PhysicalKvError::PageGenerationMismatch)
                } else if !exclusive_owner_matches(
                    slot.ownership,
                    state.request,
                    state.selection.role,
                ) {
                    Err(PhysicalKvError::PageOwnershipMismatch)
                } else if offset >= slot.initialized_prefix {
                    Err(PhysicalKvError::UninitializedRead)
                } else {
                    Ok(PhysicalKvLocation { page, offset })
                }
            }
        }
    }
}

/// Resolves an initialized logical token to its exact physical page and offset.
///
/// # Errors
///
/// Rejects stale authority, missing or stale mappings, cross-role ownership,
/// out-of-range positions, and reads beyond the initialized prefix.
pub fn map_initialized_token(
    state: &PhysicalKvState,
    request: RequestId,
    selection: Qwen3PlanSelection,
    logical_position: u32,
) -> (result: Result<PhysicalKvLocation, PhysicalKvError>)
    ensures
        result.is_ok()
            == map_initialized_decision(state, request, selection, logical_position).is_ok(),
        result.is_ok() ==>
            result == map_initialized_decision(state, request, selection, logical_position),
{
    proof {
        reveal(map_initialized_decision);
        reveal(lifecycle_matches);
        reveal(same_request);
    }
    validate_active_authority(state, request, selection)?;
    if logical_position >= state.resident_tokens {
        return Err(PhysicalKvError::LogicalPositionOutOfRange);
    }
    let logical_page = logical_position / M1_KV_PAGE_TOKENS;
    let offset = logical_position % M1_KV_PAGE_TOKENS;
    if logical_page >= state.page_count || logical_page >= M1_KV_PAGE_TABLE_ENTRIES_U32 {
        return Err(PhysicalKvError::MissingPage);
    }
    let Some(page) = state.page_table[logical_page as usize] else {
        return Err(PhysicalKvError::MissingPage);
    };
    if !same_role(page.role, state.selection.role) {
        return Err(PhysicalKvError::RoleMismatch);
    }
    if page.index >= M1_KV_PHYSICAL_PAGE_SLOTS_U32 {
        return Err(PhysicalKvError::PageOutOfRange);
    }
    let slot = state.page_slots[page.index as usize];
    if page.generation == 0 || slot.generation != page.generation {
        return Err(PhysicalKvError::PageGenerationMismatch);
    }
    if !is_exclusive_owner(slot.ownership, state.request, state.selection.role) {
        return Err(PhysicalKvError::PageOwnershipMismatch);
    }
    if offset >= slot.initialized_prefix {
        return Err(PhysicalKvError::UninitializedRead);
    }
    Ok(PhysicalKvLocation { page, offset })
}

pub closed spec fn write_token_enabled(
    state: &PhysicalKvState,
    request: RequestId,
    selection: Qwen3PlanSelection,
    logical_position: u32,
) -> bool {
    &&& lifecycle_matches(state.lifecycle, PhysicalKvLifecycle::Active)
    &&& same_request(state.request, request)
    &&& state.selection == selection
    &&& logical_position == state.resident_tokens
    &&& logical_position < state.max_context_tokens
    &&& logical_position < M1_MAX_CONTEXT_TOKENS
    &&& logical_position / M1_KV_PAGE_TOKENS < state.page_count
    &&& state.page_table@[logical_position as int / M1_KV_PAGE_TOKENS as int].is_some()
    &&& {
        let page = state.page_table@[logical_position as int / M1_KV_PAGE_TOKENS as int].unwrap();
        let slot = state.page_slots@[page.index as int];
        &&& role_matches(page.role, state.selection.role)
        &&& page.index < M1_KV_PHYSICAL_PAGE_SLOTS
        &&& page.generation > 0
        &&& slot.generation == page.generation
        &&& exclusive_owner_matches(slot.ownership, state.request, state.selection.role)
        &&& slot.initialized_prefix == logical_position % M1_KV_PAGE_TOKENS
        &&& slot.initialized_prefix < M1_KV_PAGE_TOKENS
    }
}

pub closed spec fn write_token_transition(
    before: &PhysicalKvState,
    after: &PhysicalKvState,
    page: PhysicalPageId,
) -> bool {
    let old_slot = before.page_slots@[page.index as int];
    &&& after.immutable_frame(before)
    &&& after.lifecycle == before.lifecycle
    &&& after.resident_tokens == before.resident_tokens + 1
    &&& after.committed_tokens == before.committed_tokens
    &&& after.page_count == before.page_count
    &&& after.page_table == before.page_table
    &&& after.page_slots@ == before.page_slots@.update(
        page.index as int,
        PhysicalPageSlot {
            generation: old_slot.generation,
            ownership: old_slot.ownership,
            initialized_prefix: (old_slot.initialized_prefix as int + 1) as u32,
        },
    )
}

pub closed spec fn write_at_transition(
    before: &PhysicalKvState,
    after: &PhysicalKvState,
    logical_position: u32,
) -> bool {
    let page = before.page_table@[logical_position as int / M1_KV_PAGE_TOKENS as int].unwrap();
    write_token_transition(before, after, page)
}

/// Initializes exactly the next logical token in its already-bound page.
///
/// # Errors
///
/// Rejects gaps, overwrites, missing pages, stale generations, wrong owners,
/// wrong roles, and context overflow without mutation.
pub fn write_physical_token(
    state: &mut PhysicalKvState,
    request: RequestId,
    selection: Qwen3PlanSelection,
    logical_position: u32,
) -> (result: Result<(), PhysicalKvError>)
    ensures
        result.is_ok() == write_token_enabled(old(state), request, selection, logical_position),
        result.is_ok() ==> write_at_transition(old(state), final(state), logical_position),
        result.is_err() ==> *final(state) == *old(state),
{
    proof {
        reveal(write_token_enabled);
        reveal(write_at_transition);
        reveal(write_token_transition);
        reveal(PhysicalKvState::immutable_frame);
    }
    validate_active_authority(state, request, selection)?;
    if logical_position != state.resident_tokens {
        return Err(PhysicalKvError::LogicalPositionMismatch);
    }
    if logical_position >= state.max_context_tokens
        || logical_position >= M1_MAX_CONTEXT_TOKENS
    {
        return Err(PhysicalKvError::ContextExceeded);
    }
    let logical_page = logical_position / M1_KV_PAGE_TOKENS;
    let offset = logical_position % M1_KV_PAGE_TOKENS;
    if logical_page >= state.page_count || logical_page >= M1_KV_PAGE_TABLE_ENTRIES_U32 {
        return Err(PhysicalKvError::MissingPage);
    }
    let Some(page) = state.page_table[logical_page as usize] else {
        return Err(PhysicalKvError::MissingPage);
    };
    if !same_role(page.role, state.selection.role) {
        return Err(PhysicalKvError::RoleMismatch);
    }
    if page.index >= M1_KV_PHYSICAL_PAGE_SLOTS_U32 {
        return Err(PhysicalKvError::PageOutOfRange);
    }
    let slot = state.page_slots[page.index as usize];
    if page.generation == 0 || slot.generation != page.generation {
        return Err(PhysicalKvError::PageGenerationMismatch);
    }
    if !is_exclusive_owner(slot.ownership, state.request, state.selection.role) {
        return Err(PhysicalKvError::PageOwnershipMismatch);
    }
    if slot.initialized_prefix != offset || slot.initialized_prefix >= M1_KV_PAGE_TOKENS {
        return Err(PhysicalKvError::LogicalPositionMismatch);
    }
    state.page_slots[page.index as usize].initialized_prefix += 1;
    state.resident_tokens += 1;
    Ok(())
}

pub closed spec fn commit_enabled(
    state: &PhysicalKvState,
    request: RequestId,
    selection: Qwen3PlanSelection,
    accepted_tokens: u32,
) -> bool {
    lifecycle_matches(state.lifecycle, PhysicalKvLifecycle::Active)
        && same_request(state.request, request)
        && state.selection == selection
        && accepted_tokens <= state.resident_tokens - state.committed_tokens
}

pub closed spec fn commit_transition(
    before: &PhysicalKvState,
    after: &PhysicalKvState,
    accepted_tokens: u32,
) -> bool {
    &&& after.immutable_frame(before)
    &&& after.lifecycle == before.lifecycle
    &&& after.resident_tokens == before.resident_tokens
    &&& after.committed_tokens == before.committed_tokens + accepted_tokens
    &&& after.page_count == before.page_count
    &&& after.page_table == before.page_table
    &&& after.page_slots == before.page_slots
}

/// Publishes exactly an accepted prefix of currently resident tokens.
///
/// # Errors
///
/// Rejects stale authority and accepted counts beyond the tentative suffix.
pub fn commit_physical_kv(
    state: &mut PhysicalKvState,
    request: RequestId,
    selection: Qwen3PlanSelection,
    accepted_tokens: u32,
) -> (result: Result<(), PhysicalKvError>)
    ensures
        result.is_ok() == commit_enabled(old(state), request, selection, accepted_tokens),
        result.is_ok() ==> commit_transition(old(state), final(state), accepted_tokens),
        result.is_err() ==> *final(state) == *old(state),
{
    proof {
        reveal(commit_enabled);
        reveal(commit_transition);
        reveal(PhysicalKvState::immutable_frame);
    }
    validate_active_authority(state, request, selection)?;
    if state.committed_tokens > state.resident_tokens
        || accepted_tokens > state.resident_tokens - state.committed_tokens
    {
        return Err(PhysicalKvError::CommitExceedsResident);
    }
    state.committed_tokens += accepted_tokens;
    Ok(())
}

pub closed spec fn rollback_enabled(
    state: &PhysicalKvState,
    request: RequestId,
    selection: Qwen3PlanSelection,
    after_epoch: CompletionEpoch,
) -> bool {
    &&& lifecycle_matches(state.lifecycle, PhysicalKvLifecycle::Active)
    &&& same_request(state.request, request)
    &&& state.selection == selection
    &&& after_epoch.value > 0
    &&& state.resident_tokens > state.committed_tokens
    &&& 0 < state.page_count <= M1_KV_PAGE_TABLE_ENTRIES
    &&& state.page_table@[state.page_count as int - 1].is_some()
    &&& {
        let page = state.page_table@[state.page_count as int - 1].unwrap();
        &&& role_matches(page.role, state.selection.role)
        &&& page.index < M1_KV_PHYSICAL_PAGE_SLOTS
        &&& page.generation > 0
        &&& state.page_slots@[page.index as int].generation == page.generation
        &&& exclusive_owner_matches(
            state.page_slots@[page.index as int].ownership,
            state.request,
            state.selection.role,
        )
        &&& state.page_slots@[page.index as int].initialized_prefix as int
            == (state.resident_tokens as int - 1) % M1_KV_PAGE_TOKENS as int + 1
    }
}

pub closed spec fn rollback_transition(
    before: &PhysicalKvState,
    after: &PhysicalKvState,
    after_epoch: CompletionEpoch,
    page: PhysicalPageId,
) -> bool {
    let old_slot = before.page_slots@[page.index as int];
    &&& after.immutable_frame(before)
    &&& after.lifecycle == before.lifecycle
    &&& after.resident_tokens == before.resident_tokens - 1
    &&& after.committed_tokens == before.committed_tokens
    &&& if old_slot.initialized_prefix == 1 {
        &&& after.page_count == before.page_count - 1
        &&& after.page_table@ == before.page_table@.update(before.page_count as int - 1, None)
        &&& after.page_slots@ == before.page_slots@.update(
            page.index as int,
            PhysicalPageSlot {
                generation: page.generation,
                ownership: PhysicalPageOwnership::Retired {
                    request: before.request,
                    role: before.selection.role,
                    after_epoch,
                },
                initialized_prefix: 1,
            },
        )
    } else {
        &&& after.page_count == before.page_count
        &&& after.page_table == before.page_table
        &&& after.page_slots@ == before.page_slots@.update(
            page.index as int,
            PhysicalPageSlot {
                generation: old_slot.generation,
                ownership: old_slot.ownership,
                initialized_prefix: (old_slot.initialized_prefix as int - 1) as u32,
            },
        )
    }
}

pub closed spec fn rollback_tail_transition(
    before: &PhysicalKvState,
    after: &PhysicalKvState,
    after_epoch: CompletionEpoch,
) -> bool {
    let page = before.page_table@[before.page_count as int - 1].unwrap();
    rollback_transition(before, after, after_epoch, page)
}

/// Removes exactly one tentative suffix token from logical reachability.
/// A now-empty physical page becomes retired at the exact supplied epoch.
///
/// # Errors
///
/// Rejects committed-token rollback, zero retirement epochs, stale metadata,
/// and non-tail or uninitialized state without mutation.
pub fn rollback_physical_token(
    state: &mut PhysicalKvState,
    request: RequestId,
    selection: Qwen3PlanSelection,
    after_epoch: CompletionEpoch,
) -> (result: Result<(), PhysicalKvError>)
    ensures
        result.is_ok() == rollback_enabled(old(state), request, selection, after_epoch),
        result.is_ok() ==> rollback_tail_transition(old(state), final(state), after_epoch),
        result.is_err() ==> *final(state) == *old(state),
{
    proof {
        reveal(rollback_enabled);
        reveal(rollback_tail_transition);
        reveal(rollback_transition);
        reveal(PhysicalKvState::immutable_frame);
    }
    validate_active_authority(state, request, selection)?;
    if after_epoch.value == 0 {
        return Err(PhysicalKvError::ZeroRetirementEpoch);
    }
    if state.resident_tokens <= state.committed_tokens {
        return Err(PhysicalKvError::NoTentativeToken);
    }
    if state.page_count == 0 || state.page_count > M1_KV_PAGE_TABLE_ENTRIES_U32 {
        return Err(PhysicalKvError::MissingPage);
    }
    let table_position = state.page_count - 1;
    let Some(page) = state.page_table[table_position as usize] else {
        return Err(PhysicalKvError::MissingPage);
    };
    if !same_role(page.role, state.selection.role) {
        return Err(PhysicalKvError::RoleMismatch);
    }
    if page.index >= M1_KV_PHYSICAL_PAGE_SLOTS_U32 {
        return Err(PhysicalKvError::PageOutOfRange);
    }
    let slot = state.page_slots[page.index as usize];
    if page.generation == 0 || slot.generation != page.generation {
        return Err(PhysicalKvError::PageGenerationMismatch);
    }
    if !is_exclusive_owner(slot.ownership, state.request, state.selection.role) {
        return Err(PhysicalKvError::PageOwnershipMismatch);
    }
    let expected_prefix = (state.resident_tokens - 1) % M1_KV_PAGE_TOKENS + 1;
    if slot.initialized_prefix != expected_prefix {
        return Err(PhysicalKvError::UninitializedRead);
    }
    state.resident_tokens -= 1;
    if slot.initialized_prefix == 1 {
        state.page_table[table_position as usize] = None;
        state.page_count -= 1;
        state.page_slots[page.index as usize] = PhysicalPageSlot {
            generation: page.generation,
            ownership: PhysicalPageOwnership::Retired {
                request: state.request,
                role: state.selection.role,
                after_epoch,
            },
            initialized_prefix: 1,
        };
    } else {
        state.page_slots[page.index as usize].initialized_prefix -= 1;
    }
    Ok(())
}

pub(crate) closed spec fn settlement_page_count(tokens: u32) -> u32 {
    if tokens == 0 {
        0
    } else {
        ((tokens as int - 1) / M1_KV_PAGE_TOKENS as int + 1) as u32
    }
}

fn physical_page_count(tokens: u32) -> (count: u32)
    ensures count == settlement_page_count(tokens),
{
    if tokens == 0 {
        0
    } else {
        (tokens - 1) / M1_KV_PAGE_TOKENS + 1
    }
}

closed spec fn affected_settlement_page_valid(
    state: &PhysicalKvState,
    position: int,
) -> bool {
    &&& 0 <= position < state.page_count
    &&& state.page_table@[position].is_some()
    &&& {
        let page = state.page_table@[position].unwrap();
        let slot = state.page_slots@[page.index as int];
        &&& role_matches(page.role, state.selection.role)
        &&& page.index < M1_KV_PHYSICAL_PAGE_SLOTS
        &&& page.generation > 0
        &&& slot.generation == page.generation
        &&& exclusive_owner_matches(slot.ownership, state.request, state.selection.role)
        &&& slot.initialized_prefix as int == if position + 1 < state.page_count as int {
            M1_KV_PAGE_TOKENS as int
        } else {
            state.resident_tokens as int - position * M1_KV_PAGE_TOKENS as int
        }
        &&& forall|other: int|
            0 <= other < M1_KV_PAGE_TABLE_ENTRIES && other != position
                && state.page_table@[other].is_some()
                ==> state.page_table@[other].unwrap().index != page.index
    }
}

pub(crate) closed spec fn physical_settlement_enabled(
    state: &PhysicalKvState,
    request: RequestId,
    selection: Qwen3PlanSelection,
    after_epoch: CompletionEpoch,
    pre_committed: u32,
    tentative_end: u32,
    commit_end: u32,
) -> bool {
    &&& lifecycle_matches(state.lifecycle, PhysicalKvLifecycle::Active)
    &&& same_request(state.request, request)
    &&& state.selection == selection
    &&& after_epoch.value > 0
    &&& state.committed_tokens == pre_committed
    &&& state.resident_tokens == tentative_end
    &&& pre_committed <= commit_end <= tentative_end
    &&& tentative_end as int - commit_end as int <= M1_KV_PAGE_TOKENS
    &&& state.page_count == settlement_page_count(tentative_end)
    &&& state.page_count <= M1_KV_PAGE_TABLE_ENTRIES
    &&& settlement_page_count(commit_end) <= state.page_count
    &&& state.page_count as int - settlement_page_count(commit_end) as int <= 1
    &&& if pre_committed < tentative_end {
        forall|position: int|
            pre_committed as int / M1_KV_PAGE_TOKENS as int <= position
                && position < state.page_count as int
                ==> #[trigger] affected_settlement_page_valid(state, position)
    } else {
        true
    }
}

impl PhysicalKvSettlementPermit {
    pub(crate) closed spec fn valid_for(&self, state: &PhysicalKvState) -> bool {
        let shrink_retained_tail = self.commit_end < self.tentative_end
            && self.commit_end % M1_KV_PAGE_TOKENS != 0;
        &&& physical_settlement_enabled(
            state,
            self.request,
            self.selection,
            self.after_epoch,
            self.pre_committed,
            self.tentative_end,
            self.commit_end,
        )
        &&& self.old_page_count == state.page_count
        &&& self.next_page_count == settlement_page_count(self.commit_end)
        &&& self.retired_pages as int
            == state.page_count as int - self.next_page_count as int
        &&& if self.retired_pages == 1 {
            &&& self.old_page_count > 0
            &&& self.retired_page.is_some()
            &&& state.page_table@[self.old_page_count as int - 1]
                == self.retired_page
            &&& self.retired_page.unwrap().index < M1_KV_PHYSICAL_PAGE_SLOTS
        } else {
            self.retired_page.is_none()
        }
        &&& if shrink_retained_tail {
            &&& self.next_page_count > 0
            &&& self.retained_page.is_some()
            &&& state.page_table@[self.next_page_count as int - 1]
                == self.retained_page
            &&& self.retained_page.unwrap().index < M1_KV_PHYSICAL_PAGE_SLOTS
        } else {
            self.retained_page.is_none()
        }
        &&& (self.retired_page.is_some() && self.retained_page.is_some()
            ==> self.retired_page.unwrap().index != self.retained_page.unwrap().index)
    }

    pub(crate) closed spec fn retired_pages_spec(&self) -> u32 { self.retired_pages }

    pub(crate) closed spec fn after_epoch_spec(&self) -> CompletionEpoch { self.after_epoch }

    pub(crate) closed spec fn pre_committed_spec(&self) -> u32 { self.pre_committed }

    pub(crate) closed spec fn tentative_end_spec(&self) -> u32 { self.tentative_end }

    pub(crate) closed spec fn commit_end_spec(&self) -> u32 { self.commit_end }

    pub(crate) fn retired_pages(&self) -> (count: u32)
        ensures count == self.retired_pages_spec(),
    {
        self.retired_pages
    }
}

pub closed spec fn physical_speculative_settlement_matches(
    before: &PhysicalKvState,
    after: &PhysicalKvState,
    after_epoch: CompletionEpoch,
    tentative_end: u32,
    commit_end: u32,
    retired_pages: u32,
) -> bool {
    let next_page_count = settlement_page_count(commit_end);
    let old_page = if retired_pages == 1 {
        before.page_table@[before.page_count as int - 1].unwrap()
    } else {
        PhysicalPageId {
            role: before.selection.role,
            index: 0,
            generation: 0,
        }
    };
    let table_after_retirement = if retired_pages == 1 {
        before.page_table@.update(before.page_count as int - 1, None)
    } else {
        before.page_table@
    };
    let slots_after_retirement = if retired_pages == 1 {
        before.page_slots@.update(
            old_page.index as int,
            PhysicalPageSlot {
                generation: old_page.generation,
                ownership: PhysicalPageOwnership::Retired {
                    request: before.request,
                    role: before.selection.role,
                    after_epoch,
                },
                initialized_prefix: before.page_slots@[old_page.index as int].initialized_prefix,
            },
        )
    } else {
        before.page_slots@
    };
    let shrink_retained_tail = commit_end < tentative_end
        && commit_end % M1_KV_PAGE_TOKENS != 0;
    let retained_page = if shrink_retained_tail {
        before.page_table@[next_page_count as int - 1].unwrap()
    } else {
        old_page
    };
    let expected_slots = if shrink_retained_tail {
        let retained_slot = slots_after_retirement[retained_page.index as int];
        slots_after_retirement.update(
            retained_page.index as int,
            PhysicalPageSlot {
                generation: retained_slot.generation,
                ownership: retained_slot.ownership,
                initialized_prefix: commit_end % M1_KV_PAGE_TOKENS,
            },
        )
    } else {
        slots_after_retirement
    };
    &&& after.immutable_frame(before)
    &&& after.lifecycle == before.lifecycle
    &&& after.resident_tokens == commit_end
    &&& after.committed_tokens == commit_end
    &&& after.page_count == next_page_count
    &&& after.page_table@ == table_after_retirement
    &&& after.page_slots@ == expected_slots
}

pub(crate) closed spec fn physical_settlement_transition(
    before: &PhysicalKvState,
    after: &PhysicalKvState,
    permit: &PhysicalKvSettlementPermit,
) -> bool {
    physical_speculative_settlement_matches(
        before,
        after,
        permit.after_epoch,
        permit.tentative_end,
        permit.commit_end,
        permit.retired_pages,
    )
}

/// Checks the complete affected tail without mutating physical authority.
pub(crate) fn preflight_physical_speculative_settlement(
    state: &PhysicalKvState,
    request: RequestId,
    selection: Qwen3PlanSelection,
    after_epoch: CompletionEpoch,
    pre_committed: u32,
    tentative_end: u32,
    commit_end: u32,
) -> (result: Result<PhysicalKvSettlementPermit, PhysicalKvError>)
    ensures match result {
        Ok(permit) => {
            &&& permit.valid_for(state)
            &&& permit.after_epoch_spec() == after_epoch
            &&& permit.pre_committed_spec() == pre_committed
            &&& permit.tentative_end_spec() == tentative_end
            &&& permit.commit_end_spec() == commit_end
        },
        Err(_) => true,
    },
{
    proof {
        reveal(physical_settlement_enabled);
        reveal(affected_settlement_page_valid);
        reveal(PhysicalKvSettlementPermit::valid_for);
    }
    validate_active_authority(state, request, selection)?;
    if after_epoch.value == 0 {
        return Err(PhysicalKvError::ZeroRetirementEpoch);
    }
    if state.committed_tokens != pre_committed {
        return Err(PhysicalKvError::SettlementCursorMismatch);
    }
    if state.resident_tokens != tentative_end {
        return Err(PhysicalKvError::SettlementIntervalMismatch);
    }
    if pre_committed > commit_end || commit_end > tentative_end {
        return Err(PhysicalKvError::SettlementIntervalMismatch);
    }
    if tentative_end - commit_end > M1_KV_PAGE_TOKENS {
        return Err(PhysicalKvError::SettlementTailTooWide);
    }
    let expected_page_count = physical_page_count(tentative_end);
    let next_page_count = physical_page_count(commit_end);
    if state.page_count != expected_page_count
        || state.page_count > M1_KV_PAGE_TABLE_ENTRIES_U32
        || next_page_count > state.page_count
        || state.page_count - next_page_count > 1
    {
        return Err(PhysicalKvError::SettlementPageCountMismatch);
    }
    if pre_committed < tentative_end {
        let mut position = pre_committed / M1_KV_PAGE_TOKENS;
        while position < state.page_count
            invariant
                pre_committed < tentative_end,
                state.resident_tokens == tentative_end,
                position <= state.page_count,
                state.page_count <= M1_KV_PAGE_TABLE_ENTRIES,
                state.page_table@.len() == M1_KV_PAGE_TABLE_ENTRIES,
                state.page_slots@.len() == M1_KV_PHYSICAL_PAGE_SLOTS,
                forall|prior: int|
                    pre_committed as int / M1_KV_PAGE_TOKENS as int <= prior
                        && prior < position as int
                        ==> #[trigger] affected_settlement_page_valid(state, prior),
            decreases state.page_count - position,
        {
            let Some(page) = state.page_table[position as usize] else {
                return Err(PhysicalKvError::MissingPage);
            };
            if !same_role(page.role, state.selection.role) {
                return Err(PhysicalKvError::RoleMismatch);
            }
            if page.index >= M1_KV_PHYSICAL_PAGE_SLOTS_U32 {
                return Err(PhysicalKvError::PageOutOfRange);
            }
            let slot = state.page_slots[page.index as usize];
            if page.generation == 0 || slot.generation != page.generation {
                return Err(PhysicalKvError::PageGenerationMismatch);
            }
            if !is_exclusive_owner(slot.ownership, state.request, state.selection.role) {
                return Err(PhysicalKvError::PageOwnershipMismatch);
            }
            let Some(page_start) = position.checked_mul(M1_KV_PAGE_TOKENS) else {
                return Err(PhysicalKvError::SettlementPageCountMismatch);
            };
            if page_start >= tentative_end {
                return Err(PhysicalKvError::SettlementPageCountMismatch);
            }
            let expected_prefix = if position + 1 < state.page_count {
                M1_KV_PAGE_TOKENS
            } else {
                tentative_end - page_start
            };
            if slot.initialized_prefix != expected_prefix {
                return Err(PhysicalKvError::UninitializedRead);
            }
            let mut other = 0u32;
            while other < M1_KV_PAGE_TABLE_ENTRIES_U32
                invariant
                    other <= M1_KV_PAGE_TABLE_ENTRIES,
                    position < state.page_count,
                    state.resident_tokens == tentative_end,
                    state.page_count <= M1_KV_PAGE_TABLE_ENTRIES,
                    state.page_table@.len() == M1_KV_PAGE_TABLE_ENTRIES,
                    state.page_table@[position as int].is_some(),
                    state.page_table@[position as int].unwrap() == page,
                    forall|prior: int| 0 <= prior < other as int && prior != position as int
                        && state.page_table@[prior].is_some()
                        ==> state.page_table@[prior].unwrap().index != page.index,
                decreases M1_KV_PAGE_TABLE_ENTRIES - other,
            {
                if other != position {
                    if let Some(alias) = state.page_table[other as usize] {
                        if alias.index == page.index {
                            return Err(PhysicalKvError::SettlementPhysicalAlias);
                        }
                    }
                }
                other += 1;
            }
            assert forall|other: int|
                0 <= other < M1_KV_PAGE_TABLE_ENTRIES && other != position as int
                    && state.page_table@[other].is_some()
                    implies state.page_table@[other].unwrap().index != page.index by {
                assert(state.page_table@[other].unwrap().index != page.index);
            }
            assert(affected_settlement_page_valid(state, position as int)) by {
                reveal(affected_settlement_page_valid);
                assert(0 <= (position as int)
                    && (position as int) < (state.page_count as int));
                assert(state.page_table@[position as int].is_some());
                assert(state.page_table@[position as int].unwrap() == page);
                assert(role_matches(page.role, state.selection.role));
                assert(page.index < M1_KV_PHYSICAL_PAGE_SLOTS);
                assert(slot.generation == page.generation);
                assert(exclusive_owner_matches(
                    slot.ownership,
                    state.request,
                    state.selection.role,
                ));
                assert(page_start as int
                    == position as int * M1_KV_PAGE_TOKENS as int);
                assert((position + 1 < state.page_count)
                    == (position as int + 1 < state.page_count as int));
                assert(slot.initialized_prefix == expected_prefix);
                if position + 1 < state.page_count {
                    assert(expected_prefix == M1_KV_PAGE_TOKENS);
                } else {
                    assert(expected_prefix == tentative_end - page_start);
                    assert(state.resident_tokens == tentative_end);
                }
                assert(slot.initialized_prefix as int == if position as int + 1
                    < state.page_count as int
                {
                    M1_KV_PAGE_TOKENS as int
                } else {
                    state.resident_tokens as int
                        - position as int * M1_KV_PAGE_TOKENS as int
                });
            }
            position += 1;
        }
        assert forall|position: int|
            pre_committed as int / M1_KV_PAGE_TOKENS as int <= position
                && position < state.page_count as int
                implies #[trigger] affected_settlement_page_valid(state, position) by {
            assert(affected_settlement_page_valid(state, position));
        }
    }
    let retired_pages = state.page_count - next_page_count;
    let retired_page = if retired_pages == 1 {
        state.page_table[(state.page_count - 1) as usize]
    } else {
        None
    };
    if retired_pages == 1 && retired_page.is_none() {
        return Err(PhysicalKvError::MissingPage);
    }
    let shrink_retained_tail = commit_end < tentative_end
        && !commit_end.is_multiple_of(M1_KV_PAGE_TOKENS);
    let retained_page = if shrink_retained_tail {
        state.page_table[(next_page_count - 1) as usize]
    } else {
        None
    };
    if shrink_retained_tail && retained_page.is_none() {
        return Err(PhysicalKvError::MissingPage);
    }
    if let Some(page) = retired_page {
        if page.index >= M1_KV_PHYSICAL_PAGE_SLOTS_U32 {
            return Err(PhysicalKvError::PageOutOfRange);
        }
    }
    if let Some(page) = retained_page {
        if page.index >= M1_KV_PHYSICAL_PAGE_SLOTS_U32 {
            return Err(PhysicalKvError::PageOutOfRange);
        }
    }
    if let (Some(retired), Some(retained)) = (retired_page, retained_page) {
        if retired.index == retained.index {
            return Err(PhysicalKvError::SettlementPhysicalAlias);
        }
    }
    proof {
        assert(physical_settlement_enabled(
            state,
            request,
            selection,
            after_epoch,
            pre_committed,
            tentative_end,
            commit_end,
        ));
        assert(retired_pages as int
            == state.page_count as int - next_page_count as int);
        if retired_pages == 1 {
            assert(state.page_count > 0);
            assert(retired_page.is_some());
            assert(state.page_table@[state.page_count as int - 1] == retired_page);
            assert(retired_page.unwrap().index < M1_KV_PHYSICAL_PAGE_SLOTS);
        } else {
            assert(retired_pages == 0);
            assert(retired_page.is_none());
        }
        if shrink_retained_tail {
            assert(next_page_count > 0);
            assert(retained_page.is_some());
            assert(state.page_table@[next_page_count as int - 1] == retained_page);
            assert(retained_page.unwrap().index < M1_KV_PHYSICAL_PAGE_SLOTS);
        } else {
            assert(retained_page.is_none());
        }
        assert(retired_page.is_some() && retained_page.is_some()
            ==> retired_page.unwrap().index != retained_page.unwrap().index);
    }
    let permit = PhysicalKvSettlementPermit {
        request,
        selection,
        after_epoch,
        pre_committed,
        tentative_end,
        commit_end,
        old_page_count: state.page_count,
        next_page_count,
        retired_pages,
        retired_page,
        retained_page,
    };
    assert(permit.valid_for(state));
    Ok(permit)
}

/// Applies a previously checked settlement with no fallible second stage.
pub(crate) fn apply_preflighted_physical_speculative_settlement(
    state: &mut PhysicalKvState,
    permit: PhysicalKvSettlementPermit,
)
    requires permit.valid_for(old(state)),
    ensures
        physical_settlement_transition(old(state), final(state), &permit),
        physical_speculative_settlement_matches(
            old(state),
            final(state),
            permit.after_epoch_spec(),
            permit.tentative_end_spec(),
            permit.commit_end_spec(),
            permit.retired_pages_spec(),
        ),
{
    proof {
        reveal(PhysicalKvSettlementPermit::valid_for);
        reveal(physical_settlement_enabled);
        reveal(physical_settlement_transition);
        reveal(physical_speculative_settlement_matches);
        reveal(affected_settlement_page_valid);
        reveal(PhysicalKvState::immutable_frame);
    }
    let next_page_count = permit.next_page_count;
    if let Some(page) = permit.retired_page {
        let table_position = permit.old_page_count - 1;
        let initialized_prefix = state.page_slots[page.index as usize].initialized_prefix;
        state.page_table[table_position as usize] = None;
        state.page_slots[page.index as usize] = PhysicalPageSlot {
            generation: page.generation,
            ownership: PhysicalPageOwnership::Retired {
                request: state.request,
                role: state.selection.role,
                after_epoch: permit.after_epoch,
            },
            initialized_prefix,
        };
    }
    if let Some(retained_page) = permit.retained_page {
        state.page_slots[retained_page.index as usize].initialized_prefix =
            permit.commit_end % M1_KV_PAGE_TOKENS;
    }
    state.page_count = next_page_count;
    state.resident_tokens = permit.commit_end;
    state.committed_tokens = permit.commit_end;
}

pub closed spec fn cancel_enabled(
    state: &PhysicalKvState,
    request: RequestId,
    selection: Qwen3PlanSelection,
    after_epoch: CompletionEpoch,
) -> bool {
    lifecycle_matches(state.lifecycle, PhysicalKvLifecycle::Active)
        && same_request(state.request, request)
        && state.selection == selection
        && after_epoch.value > 0
}

pub(crate) proof fn active_projection_enables_cancel(
    state: &PhysicalKvState,
    request: RequestId,
    selection: Qwen3PlanSelection,
    after_epoch: CompletionEpoch,
)
    requires
        state.abstraction_spec().lifecycle == PhysicalKvLifecycle::Active,
        state.abstraction_spec().request.slot_spec() == request.slot_spec(),
        state.abstraction_spec().request.generation_spec() == request.generation_spec(),
        state.selection_spec() == selection,
        after_epoch.value > 0,
    ensures cancel_enabled(state, request, selection, after_epoch),
{
    reveal(cancel_enabled);
    reveal(PhysicalKvState::abstraction_spec);
}

pub closed spec fn cancel_transition(
    before: &PhysicalKvState,
    after: &PhysicalKvState,
    after_epoch: CompletionEpoch,
) -> bool {
    &&& after.immutable_frame(before)
    &&& after.lifecycle == if before.page_count == 0 {
        PhysicalKvLifecycle::RetiredAwaitingQuiescence { after_epoch }
    } else {
        PhysicalKvLifecycle::Cancelled { after_epoch }
    }
    &&& after.resident_tokens == before.resident_tokens
    &&& after.committed_tokens == before.committed_tokens
    &&& after.page_count == before.page_count
    &&& after.page_table == before.page_table
    &&& after.page_slots == before.page_slots
}

/// Makes all request mappings logically unreachable while retaining pages.
///
/// # Errors
///
/// Rejects stale request/selection authority, repeated cancellation, and a
/// zero retirement epoch without mutation.
pub fn cancel_physical_kv(
    state: &mut PhysicalKvState,
    request: RequestId,
    selection: Qwen3PlanSelection,
    after_epoch: CompletionEpoch,
) -> (result: Result<(), PhysicalKvError>)
    ensures
        result.is_ok() == cancel_enabled(old(state), request, selection, after_epoch),
        result.is_ok() ==> cancel_transition(old(state), final(state), after_epoch),
        result.is_err() ==> *final(state) == *old(state),
{
    proof {
        reveal(cancel_enabled);
        reveal(cancel_transition);
        reveal(PhysicalKvState::immutable_frame);
    }
    validate_active_authority(state, request, selection)?;
    if after_epoch.value == 0 {
        return Err(PhysicalKvError::ZeroRetirementEpoch);
    }
    state.lifecycle = if state.page_count == 0 {
        PhysicalKvLifecycle::RetiredAwaitingQuiescence { after_epoch }
    } else {
        PhysicalKvLifecycle::Cancelled { after_epoch }
    };
    Ok(())
}

pub closed spec fn retire_tail_enabled(
    state: &PhysicalKvState,
    request: RequestId,
    selection: Qwen3PlanSelection,
    after_epoch: CompletionEpoch,
) -> bool {
    &&& lifecycle_matches(
        state.lifecycle,
        PhysicalKvLifecycle::Cancelled { after_epoch },
    )
    &&& same_request(state.request, request)
    &&& state.selection == selection
    &&& 0 < state.page_count <= M1_KV_PAGE_TABLE_ENTRIES
    &&& state.page_table@[state.page_count as int - 1].is_some()
    &&& {
        let page = state.page_table@[state.page_count as int - 1].unwrap();
        &&& role_matches(page.role, state.selection.role)
        &&& page.index < M1_KV_PHYSICAL_PAGE_SLOTS
        &&& page.generation > 0
        &&& state.page_slots@[page.index as int].generation == page.generation
        &&& exclusive_owner_matches(
            state.page_slots@[page.index as int].ownership,
            state.request,
            state.selection.role,
        )
        &&& 0 < state.page_slots@[page.index as int].initialized_prefix
        &&& state.page_slots@[page.index as int].initialized_prefix <= state.resident_tokens
    }
}

pub closed spec fn retire_tail_transition(
    before: &PhysicalKvState,
    after: &PhysicalKvState,
    after_epoch: CompletionEpoch,
    page: PhysicalPageId,
) -> bool {
    let removed = before.page_slots@[page.index as int].initialized_prefix;
    let remaining = (before.resident_tokens as int - removed as int) as u32;
    &&& after.immutable_frame(before)
    &&& after.lifecycle == if before.page_count == 1 {
        PhysicalKvLifecycle::RetiredAwaitingQuiescence { after_epoch }
    } else {
        before.lifecycle
    }
    &&& after.resident_tokens == remaining
    &&& after.committed_tokens == if before.committed_tokens > remaining {
        remaining
    } else {
        before.committed_tokens
    }
    &&& after.page_count == before.page_count - 1
    &&& after.page_table@ == before.page_table@.update(before.page_count as int - 1, None)
    &&& after.page_slots@ == before.page_slots@.update(
        page.index as int,
        PhysicalPageSlot {
            generation: page.generation,
            ownership: PhysicalPageOwnership::Retired {
                request: before.request,
                role: before.selection.role,
                after_epoch,
            },
            initialized_prefix: removed,
        },
    )
}

/// Retires exactly one cancelled tail page and removes it from the page table.
///
/// # Errors
///
/// Rejects a mismatched epoch, request, role, selection, page, or owner.
pub fn retire_cancelled_tail(
    state: &mut PhysicalKvState,
    request: RequestId,
    selection: Qwen3PlanSelection,
    after_epoch: CompletionEpoch,
) -> (result: Result<PhysicalPageId, PhysicalKvError>)
    ensures
        result.is_ok() == retire_tail_enabled(old(state), request, selection, after_epoch),
        result.is_ok() ==> retire_tail_transition(
            old(state),
            final(state),
            after_epoch,
            result.unwrap(),
        ),
        result.is_err() ==> *final(state) == *old(state),
{
    proof {
        reveal(retire_tail_enabled);
        reveal(retire_tail_transition);
        reveal(PhysicalKvState::immutable_frame);
        reveal(lifecycle_matches);
        reveal(same_request);
    }
    let expected_epoch = match state.lifecycle {
        PhysicalKvLifecycle::Cancelled { after_epoch: expected } => expected,
        _ => return Err(PhysicalKvError::WrongLifecycle),
    };
    if expected_epoch.value != after_epoch.value {
        return Err(PhysicalKvError::RetirementEpochMismatch);
    }
    if state.request.slot() != request.slot()
        || state.request.generation() != request.generation()
    {
        return Err(PhysicalKvError::RequestMismatch);
    }
    if !state.selection.matches(selection) {
        return Err(PhysicalKvError::SelectionMismatch);
    }
    if state.page_count == 0 || state.page_count > M1_KV_PAGE_TABLE_ENTRIES_U32 {
        return Err(PhysicalKvError::NoPageToRetire);
    }
    let table_position = state.page_count - 1;
    let Some(page) = state.page_table[table_position as usize] else {
        return Err(PhysicalKvError::MissingPage);
    };
    if !same_role(page.role, state.selection.role) {
        return Err(PhysicalKvError::RoleMismatch);
    }
    if page.index >= M1_KV_PHYSICAL_PAGE_SLOTS_U32 {
        return Err(PhysicalKvError::PageOutOfRange);
    }
    let slot = state.page_slots[page.index as usize];
    if page.generation == 0 || slot.generation != page.generation {
        return Err(PhysicalKvError::PageGenerationMismatch);
    }
    if !is_exclusive_owner(slot.ownership, state.request, state.selection.role) {
        return Err(PhysicalKvError::PageOwnershipMismatch);
    }
    if slot.initialized_prefix == 0 || slot.initialized_prefix > state.resident_tokens {
        return Err(PhysicalKvError::UninitializedRead);
    }
    let remaining = state.resident_tokens - slot.initialized_prefix;
    state.page_table[table_position as usize] = None;
    state.page_count -= 1;
    state.resident_tokens = remaining;
    if state.committed_tokens > remaining {
        state.committed_tokens = remaining;
    }
    state.page_slots[page.index as usize] = PhysicalPageSlot {
        generation: page.generation,
        ownership: PhysicalPageOwnership::Retired {
            request: state.request,
            role: state.selection.role,
            after_epoch,
        },
        initialized_prefix: slot.initialized_prefix,
    };
    if state.page_count == 0 {
        state.lifecycle = PhysicalKvLifecycle::RetiredAwaitingQuiescence { after_epoch };
    }
    Ok(page)
}

pub closed spec fn release_retired_enabled(
    state: &PhysicalKvState,
    page: PhysicalPageId,
    authority: &KvQuiescenceAuthority,
) -> bool {
    &&& same_request(state.request, authority.request)
    &&& role_matches(state.selection.role, authority.role)
    &&& page.index < M1_KV_PHYSICAL_PAGE_SLOTS
    &&& page.generation > 0
    &&& state.page_slots@[page.index as int].generation == page.generation
    &&& state.page_slots@[page.index as int].generation < u32::MAX
    &&& retired_owner_matches(
        state.page_slots@[page.index as int].ownership,
        authority.request,
        authority.role,
        authority.exact_epoch,
    )
    &&& !state.table_contains_index_spec(page.index)
}

pub closed spec fn release_retired_transition(
    before: &PhysicalKvState,
    after: &PhysicalKvState,
    page: PhysicalPageId,
) -> bool {
    &&& after.immutable_frame(before)
    &&& after.lifecycle == before.lifecycle
    &&& after.resident_tokens == before.resident_tokens
    &&& after.committed_tokens == before.committed_tokens
    &&& after.page_count == before.page_count
    &&& after.page_table == before.page_table
    &&& after.page_slots@ == before.page_slots@.update(
        page.index as int,
        PhysicalPageSlot {
            generation: (page.generation as int + 1) as u32,
            ownership: PhysicalPageOwnership::Free,
            initialized_prefix: 0,
        },
    )
}

pub closed spec fn released_generation_matches(
    released: PhysicalPageId,
    retired: PhysicalPageId,
) -> bool {
    role_matches(released.role, retired.role)
        && released.index == retired.index
        && released.generation as int == retired.generation as int + 1
}

pub(crate) proof fn released_generation_has_exact_successor(
    released: PhysicalPageId,
    prior: PhysicalPageId,
)
    requires released_generation_matches(released, prior),
    ensures
        released.index_spec() == prior.index_spec(),
        released.generation_spec() as int == prior.generation_spec() as int + 1,
{
    reveal(released_generation_matches);
}

/// Releases a retired physical generation only under exact quiescence authority.
///
/// The authority has no public constructor. The crate-local logical composition
/// creates it only after exact scheduler epoch observation.
///
/// # Errors
///
/// Rejects stale page generations, aliases, wrong request/role/epoch authority,
/// non-retired pages, and exhausted generations without mutation.
pub fn release_retired_page(
    state: &mut PhysicalKvState,
    page: PhysicalPageId,
    authority: &KvQuiescenceAuthority,
) -> (result: Result<PhysicalPageId, PhysicalKvError>)
    ensures
        result.is_ok() == release_retired_enabled(old(state), page, authority),
        result.is_ok() ==> {
            &&& released_generation_matches(result.unwrap(), page)
            &&& release_retired_transition(old(state), final(state), page)
        },
        result.is_err() ==> *final(state) == *old(state),
{
    proof {
        reveal(release_retired_enabled);
        reveal(release_retired_transition);
        reveal(released_generation_matches);
        reveal(PhysicalKvState::immutable_frame);
        reveal(same_request);
    }
    if state.request.slot() != authority.request.slot()
        || state.request.generation() != authority.request.generation()
        || !same_role(state.selection.role, authority.role)
    {
        return Err(PhysicalKvError::InvalidQuiescenceAuthority);
    }
    if page.index >= M1_KV_PHYSICAL_PAGE_SLOTS_U32 {
        return Err(PhysicalKvError::PageOutOfRange);
    }
    let slot = state.page_slots[page.index as usize];
    if page.generation == 0 || slot.generation != page.generation {
        return Err(PhysicalKvError::PageGenerationMismatch);
    }
    if slot.generation == u32::MAX {
        return Err(PhysicalKvError::GenerationExhausted);
    }
    if !is_retired_owner(
        slot.ownership,
        authority.request,
        authority.role,
        authority.exact_epoch,
    ) {
        return Err(PhysicalKvError::InvalidQuiescenceAuthority);
    }
    if state.table_contains_index(page.index) {
        return Err(PhysicalKvError::PhysicalAlias);
    }
    let next = PhysicalPageId {
        role: page.role,
        index: page.index,
        generation: page.generation + 1,
    };
    state.page_slots[page.index as usize] = PhysicalPageSlot {
        generation: next.generation,
        ownership: PhysicalPageOwnership::Free,
        initialized_prefix: 0,
    };
    Ok(next)
}

} // verus!

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Qwen3ExecutionMode, Qwen3PlanBucket};

    fn request() -> RequestId {
        RequestId::new(3, 7)
    }

    fn target_decode() -> Qwen3PlanSelection {
        Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Decode,
            bucket: Qwen3PlanBucket::DecodeS1C8192,
        }
    }

    fn draft_decode() -> Qwen3PlanSelection {
        Qwen3PlanSelection {
            role: Qwen3ModelRole::Draft06B,
            mode: Qwen3ExecutionMode::Decode,
            bucket: Qwen3PlanBucket::DecodeS1C8192,
        }
    }

    fn append_and_write(
        state: &mut PhysicalKvState,
        selection: Qwen3PlanSelection,
        page_index: u32,
        token_count: u32,
    ) -> PhysicalPageId {
        let generation = state.page_generation(page_index).unwrap();
        let page = PhysicalPageId::new(selection.role, page_index, generation);
        append_physical_page(state, request(), selection, page).unwrap();
        let start = state.logical_state().resident_tokens;
        for position in start..start + token_count {
            write_physical_token(state, request(), selection, position).unwrap();
        }
        page
    }

    #[test]
    fn exact_mapping_crosses_the_fixed_page_boundary() {
        let selection = target_decode();
        let mut state = PhysicalKvState::new(request(), selection).unwrap();
        let first = append_and_write(&mut state, selection, 9, M1_KV_PAGE_TOKENS);
        let second = append_and_write(&mut state, selection, 2, 1);

        assert_eq!(
            map_initialized_token(&state, request(), selection, 15),
            Ok(PhysicalKvLocation {
                page: first,
                offset: 15
            })
        );
        assert_eq!(
            map_initialized_token(&state, request(), selection, 16),
            Ok(PhysicalKvLocation {
                page: second,
                offset: 0
            })
        );
    }

    fn target_with_seventeen_tokens() -> (PhysicalKvState, PhysicalPageId, PhysicalPageId) {
        let selection = target_decode();
        let mut state = PhysicalKvState::new(request(), selection).unwrap();
        let first = append_and_write(&mut state, selection, 9, M1_KV_PAGE_TOKENS);
        let second = append_and_write(&mut state, selection, 2, 1);
        (state, first, second)
    }

    #[test]
    fn whole_tail_preflight_rejects_stale_alias_owner_and_prefix_metadata() {
        let selection = target_decode();
        let epoch = CompletionEpoch::new(41);

        let (mut stale, _, stale_page) = target_with_seventeen_tokens();
        stale.page_slots[stale_page.index as usize].generation += 1;
        let before = stale.logical_state();
        assert_eq!(
            preflight_physical_speculative_settlement(
                &stale,
                request(),
                selection,
                epoch,
                0,
                17,
                1,
            ),
            Err(PhysicalKvError::PageGenerationMismatch)
        );
        assert_eq!(stale.logical_state(), before);

        let (mut wrong_owner, _, owner_page) = target_with_seventeen_tokens();
        wrong_owner.page_slots[owner_page.index as usize].ownership = PhysicalPageOwnership::Free;
        assert_eq!(
            preflight_physical_speculative_settlement(
                &wrong_owner,
                request(),
                selection,
                epoch,
                0,
                17,
                1,
            ),
            Err(PhysicalKvError::PageOwnershipMismatch)
        );

        let (mut wrong_prefix, _, prefix_page) = target_with_seventeen_tokens();
        wrong_prefix.page_slots[prefix_page.index as usize].initialized_prefix = 2;
        assert_eq!(
            preflight_physical_speculative_settlement(
                &wrong_prefix,
                request(),
                selection,
                epoch,
                0,
                17,
                1,
            ),
            Err(PhysicalKvError::UninitializedRead)
        );

        let (mut alias, first_page, _) = target_with_seventeen_tokens();
        alias.page_table[1] = Some(first_page);
        assert_eq!(
            preflight_physical_speculative_settlement(
                &alias,
                request(),
                selection,
                epoch,
                0,
                17,
                1,
            ),
            Err(PhysicalKvError::SettlementPhysicalAlias)
        );

        let (mut full_acceptance_stale, first_page, _) = target_with_seventeen_tokens();
        full_acceptance_stale.page_slots[first_page.index as usize].generation += 1;
        assert_eq!(
            preflight_physical_speculative_settlement(
                &full_acceptance_stale,
                request(),
                selection,
                epoch,
                0,
                17,
                17,
            ),
            Err(PhysicalKvError::PageGenerationMismatch)
        );
    }

    #[test]
    fn stale_request_selection_role_and_generation_are_rejected() {
        let selection = target_decode();
        let mut state = PhysicalKvState::new(request(), selection).unwrap();
        let target_page = PhysicalPageId::new(Qwen3ModelRole::Target8B, 0, 1);
        assert_eq!(
            append_physical_page(&mut state, RequestId::new(3, 8), selection, target_page),
            Err(PhysicalKvError::RequestMismatch)
        );
        assert_eq!(
            append_physical_page(&mut state, request(), draft_decode(), target_page),
            Err(PhysicalKvError::SelectionMismatch)
        );
        assert_eq!(
            append_physical_page(
                &mut state,
                request(),
                selection,
                PhysicalPageId::new(Qwen3ModelRole::Draft06B, 0, 1),
            ),
            Err(PhysicalKvError::RoleMismatch)
        );
        assert_eq!(
            append_physical_page(
                &mut state,
                request(),
                selection,
                PhysicalPageId::new(Qwen3ModelRole::Target8B, 0, 2),
            ),
            Err(PhysicalKvError::PageGenerationMismatch)
        );
        assert_eq!(
            append_physical_page(
                &mut state,
                request(),
                selection,
                PhysicalPageId::new(Qwen3ModelRole::Target8B, M1_KV_PHYSICAL_PAGE_SLOTS_U32, 1,),
            ),
            Err(PhysicalKvError::PageOutOfRange)
        );
    }

    #[test]
    fn uninitialized_gap_overwrite_and_out_of_range_are_rejected() {
        let selection = target_decode();
        let mut state = PhysicalKvState::new(request(), selection).unwrap();
        let page = PhysicalPageId::new(selection.role, 0, 1);
        append_physical_page(&mut state, request(), selection, page).unwrap();
        assert_eq!(
            map_initialized_token(&state, request(), selection, 0),
            Err(PhysicalKvError::LogicalPositionOutOfRange)
        );
        assert_eq!(
            write_physical_token(&mut state, request(), selection, 1),
            Err(PhysicalKvError::LogicalPositionMismatch)
        );
        write_physical_token(&mut state, request(), selection, 0).unwrap();
        assert_eq!(
            write_physical_token(&mut state, request(), selection, 0),
            Err(PhysicalKvError::LogicalPositionMismatch)
        );
        assert_eq!(
            map_initialized_token(&state, request(), selection, 1),
            Err(PhysicalKvError::LogicalPositionOutOfRange)
        );
        assert_eq!(state.page_generation(M1_KV_PHYSICAL_PAGE_SLOTS_U32), None);
    }

    #[test]
    fn page_alias_and_non_boundary_append_are_rejected() {
        let selection = target_decode();
        let mut state = PhysicalKvState::new(request(), selection).unwrap();
        let page = append_and_write(&mut state, selection, 4, 1);
        assert_eq!(
            append_physical_page(&mut state, request(), selection, page),
            Err(PhysicalKvError::PageNotRequired)
        );
        for position in 1..M1_KV_PAGE_TOKENS {
            write_physical_token(&mut state, request(), selection, position).unwrap();
        }
        assert_eq!(
            append_physical_page(&mut state, request(), selection, page),
            Err(PhysicalKvError::PhysicalAlias)
        );
    }

    #[test]
    fn commit_and_rollback_preserve_only_the_accepted_prefix() {
        let selection = target_decode();
        let mut state = PhysicalKvState::new(request(), selection).unwrap();
        let page = append_and_write(&mut state, selection, 1, 3);
        commit_physical_kv(&mut state, request(), selection, 1).unwrap();
        assert_eq!(
            commit_physical_kv(&mut state, request(), selection, 3),
            Err(PhysicalKvError::CommitExceedsResident)
        );
        rollback_physical_token(&mut state, request(), selection, CompletionEpoch::new(12))
            .unwrap();
        assert_eq!(state.logical_state().resident_tokens, 2);
        assert_eq!(state.logical_state().committed_tokens, 1);
        assert_eq!(
            map_initialized_token(&state, request(), selection, 2),
            Err(PhysicalKvError::LogicalPositionOutOfRange)
        );
        assert_eq!(
            map_initialized_token(&state, request(), selection, 1),
            Ok(PhysicalKvLocation { page, offset: 1 })
        );
    }

    #[test]
    fn rolled_back_page_cannot_reuse_before_exact_quiescence() {
        let selection = target_decode();
        let mut state = PhysicalKvState::new(request(), selection).unwrap();
        let page = append_and_write(&mut state, selection, 0, 1);
        let epoch = CompletionEpoch::new(44);
        rollback_physical_token(&mut state, request(), selection, epoch).unwrap();

        assert_eq!(
            append_physical_page(&mut state, request(), selection, page),
            Err(PhysicalKvError::PageNotFree)
        );
        let wrong = KvQuiescenceAuthority {
            request: request(),
            role: selection.role,
            exact_epoch: CompletionEpoch::new(45),
        };
        assert_eq!(
            release_retired_page(&mut state, page, &wrong),
            Err(PhysicalKvError::InvalidQuiescenceAuthority)
        );
        let exact = KvQuiescenceAuthority {
            request: request(),
            role: selection.role,
            exact_epoch: epoch,
        };
        let next = release_retired_page(&mut state, page, &exact).unwrap();
        assert_eq!(next.generation(), page.generation() + 1);
        assert_eq!(
            append_physical_page(&mut state, request(), selection, page),
            Err(PhysicalKvError::PageGenerationMismatch)
        );
        append_physical_page(&mut state, request(), selection, next).unwrap();
    }

    #[test]
    fn cancellation_is_unreachable_then_retires_tail_at_exact_epoch() {
        let selection = draft_decode();
        let mut state = PhysicalKvState::new(request(), selection).unwrap();
        let first = append_and_write(&mut state, selection, 0, M1_KV_PAGE_TOKENS);
        let second = append_and_write(&mut state, selection, 1, 2);
        commit_physical_kv(&mut state, request(), selection, 5).unwrap();
        let epoch = CompletionEpoch::new(101);
        cancel_physical_kv(&mut state, request(), selection, epoch).unwrap();
        assert_eq!(
            map_initialized_token(&state, request(), selection, 0),
            Err(PhysicalKvError::WrongLifecycle)
        );
        assert_eq!(
            cancel_physical_kv(&mut state, request(), selection, epoch),
            Err(PhysicalKvError::WrongLifecycle)
        );
        assert_eq!(
            retire_cancelled_tail(&mut state, request(), selection, CompletionEpoch::new(102),),
            Err(PhysicalKvError::RetirementEpochMismatch)
        );
        assert_eq!(
            retire_cancelled_tail(&mut state, request(), selection, epoch),
            Ok(second)
        );
        assert_eq!(
            retire_cancelled_tail(&mut state, request(), selection, epoch),
            Ok(first)
        );
        assert_eq!(state.logical_state().resident_tokens, 0);
        assert_eq!(state.logical_state().committed_tokens, 0);
        assert_eq!(
            state.logical_state().lifecycle,
            PhysicalKvLifecycle::RetiredAwaitingQuiescence { after_epoch: epoch }
        );
    }

    #[test]
    fn invalid_bucket_and_zero_generation_fail_closed() {
        let invalid = Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Prefill,
            bucket: Qwen3PlanBucket::DecodeS1C8192,
        };
        assert_eq!(
            PhysicalKvState::new(request(), invalid),
            Err(PhysicalKvError::InvalidSelection)
        );
        assert_eq!(
            PhysicalKvState::new(RequestId::new(3, 0), target_decode()),
            Err(PhysicalKvError::ZeroRequestGeneration)
        );
    }

    #[test]
    fn finite_bucket_context_and_zero_epoch_fail_closed() {
        let selection = Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Prefill,
            bucket: Qwen3PlanBucket::PrefillS1T128,
        };
        let mut state = PhysicalKvState::new(request(), selection).unwrap();
        assert_eq!(state.max_context_tokens(), 128);
        for page_index in 0..8 {
            append_and_write(&mut state, selection, page_index, M1_KV_PAGE_TOKENS);
        }
        assert_eq!(state.logical_state().resident_tokens, 128);
        assert_eq!(
            append_physical_page(
                &mut state,
                request(),
                selection,
                PhysicalPageId::new(selection.role, 8, 1),
            ),
            Err(PhysicalKvError::ContextExceeded)
        );
        assert_eq!(
            cancel_physical_kv(&mut state, request(), selection, CompletionEpoch::new(0),),
            Err(PhysicalKvError::ZeroRetirementEpoch)
        );
    }

    #[test]
    fn empty_cancellation_reaches_retired_state_without_a_page() {
        let selection = target_decode();
        let mut state = PhysicalKvState::new(request(), selection).unwrap();
        let epoch = CompletionEpoch::new(5);
        cancel_physical_kv(&mut state, request(), selection, epoch).unwrap();
        assert_eq!(
            state.logical_state().lifecycle,
            PhysicalKvLifecycle::RetiredAwaitingQuiescence { after_epoch: epoch }
        );
        assert_eq!(
            retire_cancelled_tail(&mut state, request(), selection, epoch),
            Err(PhysicalKvError::WrongLifecycle)
        );
    }
}
