use crate::scheduler::{KvQuiescenceOrigin, KvQuiescencePermit};
use ferric_spec::RequestId;
use std::fmt;
use vstd::prelude::*;

verus! {

/// Build-generated upper bound for physical KV page slots.
pub const MAX_PAGE_SLOTS: usize = 16_384;
/// M0 admission bound for concurrently live request slots.
pub const MAX_REQUEST_SLOTS: usize = 32;
/// Build-generated per-request page-table bound (8K tokens at 16 tokens/page).
pub const MAX_PAGES_PER_REQUEST: usize = 512;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PageId {
    index: u32,
    generation: u32,
}

impl PageId {
    const EMPTY: Self = Self { index: 0, generation: 0 };

    #[must_use]
    pub fn index(self) -> (index: u32)
        ensures index == self.index_spec(),
    { self.index }

    #[must_use]
    pub fn generation(self) -> (generation: u32)
        ensures generation == self.generation_spec(),
    { self.generation }

    pub closed spec fn index_spec(&self) -> u32 { self.index }
    pub closed spec fn generation_spec(&self) -> u32 { self.generation }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RequestKey { slot: u32, generation: u32 }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PageState {
    Free,
    Writable { owner_slot: u8 },
    Sealed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PageSlot {
    generation: u32,
    state: PageState,
    initialized_tokens: u32,
    reference_mask: u32,
}

impl PageSlot {
    fn free() -> (slot: Self)
        ensures
            slot.generation == 1,
            slot.state == PageState::Free,
            slot.initialized_tokens == 0,
            slot.reference_mask == 0,
    {
        Self {
            generation: 1,
            state: PageState::Free,
            initialized_tokens: 0,
            reference_mask: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RequestSlot {
    generation: u32,
    live: bool,
    committed_tokens: u32,
    resident_tokens: u32,
    page_count: u32,
    pages: [PageId; MAX_PAGES_PER_REQUEST],
}

impl RequestSlot {
    fn empty() -> (slot: Self)
        ensures
            slot.generation == 1,
            !slot.live,
            slot.committed_tokens == 0,
            slot.resident_tokens == 0,
            slot.page_count == 0,
            forall |position: int|
                0 <= position < MAX_PAGES_PER_REQUEST ==>
                    slot.pages@[position] == PageId::EMPTY,
    {
        let pages = vstd::array::array_fill_for_copy_types(PageId::EMPTY);
        Self {
            generation: 1,
            live: false,
            committed_tokens: 0,
            resident_tokens: 0,
            page_count: 0,
            pages,
        }
    }
}

/// Cache evidence consumed by the lifecycle before a request becomes ready.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct KvFinalizedRequest {
    request: RequestId,
    origin: KvQuiescenceOrigin,
}

impl KvFinalizedRequest {
    pub(crate) const fn request(&self) -> (request: RequestId)
        ensures request == self.request_spec(),
    { self.request }

    pub(crate) const fn origin(&self) -> (origin: KvQuiescenceOrigin)
        ensures origin == self.origin_spec(),
    { self.origin }

    pub(crate) closed spec fn request_spec(&self) -> RequestId { self.request }
    pub(crate) closed spec fn origin_spec(&self) -> KvQuiescenceOrigin { self.origin }
}

/// Cache evidence consumed by the lifecycle before a request slot is reused.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct KvDetachedRequest {
    request: RequestId,
    origin: KvQuiescenceOrigin,
}

impl KvDetachedRequest {
    pub(crate) const fn request(&self) -> (request: RequestId)
        ensures request == self.request_spec(),
    { self.request }

    pub(crate) const fn origin(&self) -> (origin: KvQuiescenceOrigin)
        ensures origin == self.origin_spec(),
    { self.origin }

    pub(crate) closed spec fn request_spec(&self) -> RequestId { self.request }
    pub(crate) closed spec fn origin_spec(&self) -> KvQuiescenceOrigin { self.origin }
}

/// Retry-safe authority error. The unique permit is always returned unchanged.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct KvAuthorityError {
    error: KvError,
    permit: KvQuiescencePermit,
}

impl KvAuthorityError {
    pub(crate) fn into_parts(self) -> (parts: (KvError, KvQuiescencePermit))
        ensures
            parts.1.request_spec() == self.permit_request_spec(),
            parts.1.origin_spec() == self.permit_origin_spec(),
    {
        (self.error, self.permit)
    }

    pub(crate) closed spec fn permit_request_spec(&self) -> RequestId {
        self.permit.request_spec()
    }

    pub(crate) closed spec fn permit_origin_spec(&self) -> KvQuiescenceOrigin {
        self.permit.origin_spec()
    }
}

/// Fixed-capacity, allocation-free-after-construction paged KV metadata.
#[derive(Debug)]
pub struct KvPool {
    page_tokens: u32,
    max_context_tokens: u32,
    page_limit: u32,
    pages: Vec<PageSlot>,
    free_stack: Vec<u32>,
    free_len: u32,
    free_bitmap: Vec<bool>,
    requests: Vec<RequestSlot>,
}

closed spec fn has_reference(mask: u32, request_slot: u32) -> bool {
    (mask & (1_u32 << request_slot)) != 0
}

closed spec fn has_other_reference(mask: u32, excluded_slot: u32) -> bool {
    (mask & !(1_u32 << excluded_slot)) != 0
}

#[verifier::bit_vector]
proof fn set_reference_lemma(mask: u32, set_slot: u32, observed_slot: u32)
    requires set_slot < 32, observed_slot < 32,
    ensures
        has_reference(mask | (1_u32 << set_slot), observed_slot)
            == (has_reference(mask, observed_slot) || set_slot == observed_slot),
{}

#[verifier::bit_vector]
proof fn clear_reference_lemma(mask: u32, cleared_slot: u32, observed_slot: u32)
    requires cleared_slot < 32, observed_slot < 32,
    ensures
        has_reference(mask & !(1_u32 << cleared_slot), observed_slot)
            == (has_reference(mask, observed_slot) && cleared_slot != observed_slot),
{}

#[verifier::bit_vector]
proof fn zero_reference_lemma(request_slot: u32)
    requires request_slot < 32,
    ensures !has_reference(0, request_slot),
{}

fn set_reference(mask: u32, request_slot: u32) -> (updated: u32)
    requires request_slot < MAX_REQUEST_SLOTS,
    ensures
        updated == mask | (1_u32 << request_slot),
        forall |observed_slot: u32| observed_slot < MAX_REQUEST_SLOTS ==>
            has_reference(updated, observed_slot)
                == (has_reference(mask, observed_slot) || request_slot == observed_slot),
{
    let updated = mask | (1_u32 << request_slot);
    assert forall |observed_slot: u32| observed_slot < MAX_REQUEST_SLOTS implies
        has_reference(updated, observed_slot)
            == (has_reference(mask, observed_slot) || request_slot == observed_slot) by {
        set_reference_lemma(mask, request_slot, observed_slot);
    }
    updated
}

fn clear_reference(mask: u32, request_slot: u32) -> (updated: u32)
    requires request_slot < MAX_REQUEST_SLOTS,
    ensures
        updated == mask & !(1_u32 << request_slot),
        forall |observed_slot: u32| observed_slot < MAX_REQUEST_SLOTS ==>
            has_reference(updated, observed_slot)
                == (has_reference(mask, observed_slot) && request_slot != observed_slot),
{
    let updated = mask & !(1_u32 << request_slot);
    assert forall |observed_slot: u32| observed_slot < MAX_REQUEST_SLOTS implies
        has_reference(updated, observed_slot)
            == (has_reference(mask, observed_slot) && request_slot != observed_slot) by {
        clear_reference_lemma(mask, request_slot, observed_slot);
    }
    updated
}

fn reference_is_set(mask: u32, request_slot: u32) -> (is_set: bool)
    requires request_slot < MAX_REQUEST_SLOTS,
    ensures is_set == has_reference(mask, request_slot),
{
    (mask & (1_u32 << request_slot)) != 0
}

closed spec fn logical_page(request: RequestSlot, position: int) -> PageId {
    request.pages@[position]
}

closed spec fn logical_pages_distinct(request: RequestSlot, left: int, right: int) -> bool {
    logical_page(request, left).index != logical_page(request, right).index
}

closed spec fn free_positions_distinct(pool: &KvPool, left: int, right: int) -> bool {
    pool.free_stack@[left] != pool.free_stack@[right]
}

impl KvPool {
    closed spec fn ceil_pages(tokens: int, page_tokens: int) -> int
        recommends tokens >= 0, page_tokens > 0,
    {
        (tokens + page_tokens - 1) / page_tokens
    }

    pub closed spec fn new_enabled(
        page_count: u32,
        page_tokens: u32,
        max_context_tokens: u32,
    ) -> bool {
        &&& page_count > 0
        &&& page_tokens > 0
        &&& max_context_tokens > 0
        &&& page_count <= MAX_PAGE_SLOTS
        &&& page_tokens <= max_context_tokens
        &&& (max_context_tokens as int + page_tokens as int - 1)
            / page_tokens as int <= MAX_PAGES_PER_REQUEST
    }

    pub closed spec fn request_live_by_slot_spec(&self, slot: int) -> bool
        recommends 0 <= slot < MAX_REQUEST_SLOTS,
    {
        self.requests@[slot].live
    }

    pub closed spec fn request_generation_by_slot_spec(&self, slot: int) -> u32
        recommends 0 <= slot < MAX_REQUEST_SLOTS,
    {
        self.requests@[slot].generation
    }

    pub open spec fn identity_frame(&self, before: &Self) -> bool {
        forall |slot: int| 0 <= slot < MAX_REQUEST_SLOTS ==> {
            &&& self.request_live_by_slot_spec(slot)
                == before.request_live_by_slot_spec(slot)
            &&& self.request_generation_by_slot_spec(slot)
                == before.request_generation_by_slot_spec(slot)
        }
    }

    pub open spec fn create_identity_refines(
        &self,
        before: &Self,
        request: RequestId,
        result: &Result<(), KvError>,
    ) -> bool {
        match result {
            Err(_) => self.identity_frame(before),
            Ok(()) => {
                let changed = request.slot_spec() as int;
                &&& 0 <= changed < MAX_REQUEST_SLOTS
                &&& !before.request_live_by_slot_spec(changed)
                &&& self.request_live_by_slot_spec(changed)
                &&& self.request_generation_by_slot_spec(changed)
                    == before.request_generation_by_slot_spec(changed)
                &&& request.generation_spec()
                    == self.request_generation_by_slot_spec(changed)
                &&& forall |slot: int| 0 <= slot < MAX_REQUEST_SLOTS && slot != changed ==> {
                    &&& self.request_live_by_slot_spec(slot)
                        == before.request_live_by_slot_spec(slot)
                    &&& self.request_generation_by_slot_spec(slot)
                        == before.request_generation_by_slot_spec(slot)
                }
            }
        }
    }

    pub(crate) open spec fn release_identity_refines(
        &self,
        before: &Self,
        request: RequestId,
        result: &Result<KvDetachedRequest, KvAuthorityError>,
    ) -> bool {
        match result {
            Err(_) => self.identity_frame(before),
            Ok(_) => {
                let changed = request.slot_spec() as int;
                &&& 0 <= changed < MAX_REQUEST_SLOTS
                &&& before.request_live_by_slot_spec(changed)
                &&& !self.request_live_by_slot_spec(changed)
                &&& self.request_generation_by_slot_spec(changed)
                    == before.request_generation_by_slot_spec(changed) + 1
                &&& request.generation_spec()
                    == before.request_generation_by_slot_spec(changed)
                &&& forall |slot: int| 0 <= slot < MAX_REQUEST_SLOTS && slot != changed ==> {
                    &&& self.request_live_by_slot_spec(slot)
                        == before.request_live_by_slot_spec(slot)
                    &&& self.request_generation_by_slot_spec(slot)
                        == before.request_generation_by_slot_spec(slot)
                }
            }
        }
    }

    pub closed spec fn request_matches_spec(&self, request: RequestId) -> bool {
        self.key_matches(RequestKey {
            slot: request.slot_spec(),
            generation: request.generation_spec(),
        })
    }

    closed spec fn key_matches(&self, request: RequestKey) -> bool {
        &&& request.slot < MAX_REQUEST_SLOTS
        &&& self.requests@[request.slot as int].live
        &&& self.requests@[request.slot as int].generation == request.generation
    }

    pub closed spec fn resident_tokens_spec(&self, request: RequestId) -> Option<u32> {
        if self.request_matches_spec(request) {
            Some(self.requests@[request.slot_spec() as int].resident_tokens)
        } else {
            None
        }
    }

    pub closed spec fn committed_tokens_spec(&self, request: RequestId) -> Option<u32> {
        if self.request_matches_spec(request) {
            Some(self.requests@[request.slot_spec() as int].committed_tokens)
        } else {
            None
        }
    }

    pub closed spec fn page_count_spec(&self, request: RequestId) -> Option<u32> {
        if self.request_matches_spec(request) {
            Some(self.requests@[request.slot_spec() as int].page_count)
        } else {
            None
        }
    }

    pub closed spec fn page_at_spec(
        &self,
        request: RequestId,
        logical_page: u32,
    ) -> Option<PageId> {
        if self.request_matches_spec(request)
            && logical_page < self.requests@[request.slot_spec() as int].page_count
        {
            Some(self.requests@[request.slot_spec() as int].pages@[logical_page as int])
        } else {
            None
        }
    }

    pub closed spec fn free_pages_spec(&self) -> u32 { self.free_len }

    pub closed spec fn page_tokens_spec(&self) -> u32 { self.page_tokens }

    pub closed spec fn max_context_tokens_spec(&self) -> u32 { self.max_context_tokens }

    pub closed spec fn page_limit_spec(&self) -> u32 { self.page_limit }

    pub open spec fn create_enabled(&self, request: RequestId) -> bool {
        &&& request.slot_spec() < MAX_REQUEST_SLOTS
        &&& !self.request_live_by_slot_spec(request.slot_spec() as int)
        &&& self.request_generation_by_slot_spec(request.slot_spec() as int)
            == request.generation_spec()
    }

    closed spec fn create_key_enabled(&self, request: RequestKey) -> bool {
        &&& request.slot < MAX_REQUEST_SLOTS
        &&& !self.requests@[request.slot as int].live
        &&& self.requests@[request.slot as int].generation == request.generation
    }

    pub closed spec fn append_enabled(&self, request: RequestId, token_count: u32) -> bool {
        self.append_key_enabled(
            RequestKey {
                slot: request.slot_spec(),
                generation: request.generation_spec(),
            },
            token_count,
        )
    }

    closed spec fn append_key_enabled(&self, request: RequestKey, token_count: u32) -> bool {
        if self.key_matches(request) {
            let slot = self.requests@[request.slot as int];
            let new_resident = slot.resident_tokens as int + token_count as int;
            let tail_capacity = if slot.page_count == 0 {
                0
            } else {
                let tail = slot.pages@[slot.page_count - 1];
                match self.pages@[tail.index as int].state {
                    PageState::Writable { .. } => {
                        self.page_tokens - self.pages@[tail.index as int].initialized_tokens
                    }
                    PageState::Sealed | PageState::Free => 0,
                }
            };
            let after_tail = if token_count > tail_capacity {
                token_count as int - tail_capacity as int
            } else {
                0
            };
            let full_pages = after_tail / self.page_tokens as int;
            let extra_page = if after_tail % self.page_tokens as int == 0 { 0_int } else { 1_int };
            let required_pages = full_pages + extra_page;
            &&& new_resident <= self.max_context_tokens
            &&& slot.page_count as int + required_pages <= MAX_PAGES_PER_REQUEST
            &&& required_pages <= self.free_len
        } else {
            false
        }
    }

    pub closed spec fn share_enabled(
        &self,
        source: RequestId,
        target: RequestId,
        token_count: u32,
    ) -> bool {
        self.share_key_enabled(
            RequestKey {
                slot: source.slot_spec(),
                generation: source.generation_spec(),
            },
            RequestKey {
                slot: target.slot_spec(),
                generation: target.generation_spec(),
            },
            token_count,
        )
    }

    closed spec fn share_key_enabled(
        &self,
        source: RequestKey,
        target: RequestKey,
        token_count: u32,
    ) -> bool {
        &&& self.key_matches(source)
        &&& self.key_matches(target)
        &&& source.slot != target.slot
        &&& token_count > 0
        &&& token_count % self.page_tokens == 0
        &&& token_count <= self.requests@[source.slot as int].committed_tokens
        &&& self.requests@[target.slot as int].resident_tokens == 0
        &&& token_count / self.page_tokens <= MAX_PAGES_PER_REQUEST
    }

    pub closed spec fn read_enabled(
        &self,
        request: RequestId,
        logical_offset: u32,
        span: u32,
    ) -> bool {
        self.read_key_enabled(
            RequestKey {
                slot: request.slot_spec(),
                generation: request.generation_spec(),
            },
            logical_offset,
            span,
        )
    }

    closed spec fn read_key_enabled(
        &self,
        request: RequestKey,
        logical_offset: u32,
        span: u32,
    ) -> bool {
        &&& self.key_matches(request)
        &&& logical_offset as int + span as int
            <= self.requests@[request.slot as int].resident_tokens
    }

    pub closed spec fn finalize_enabled(
        &self,
        request: RequestId,
        accepted_tokens: u32,
    ) -> bool {
        self.finalize_key_enabled(
            RequestKey {
                slot: request.slot_spec(),
                generation: request.generation_spec(),
            },
            accepted_tokens,
        )
    }

    closed spec fn finalize_key_enabled(
        &self,
        request: RequestKey,
        accepted_tokens: u32,
    ) -> bool {
        if self.key_matches(request) {
            let slot = self.requests@[request.slot as int];
            let committed = slot.committed_tokens as int + accepted_tokens as int;
            let retained = if committed == 0 {
                0
            } else {
                (committed + self.page_tokens as int - 1) / self.page_tokens as int
            };
            &&& committed <= slot.resident_tokens
            &&& retained <= slot.page_count
            &&& self.free_len as int + slot.page_count as int - retained <= self.page_limit
            &&& forall |position: int|
                retained <= position < slot.page_count ==>
                    self.pages@[slot.pages@[position].index as int].generation < u32::MAX
        } else {
            false
        }
    }

    pub closed spec fn release_enabled(&self, request: RequestId) -> bool {
        self.release_key_enabled(RequestKey {
            slot: request.slot_spec(),
            generation: request.generation_spec(),
        })
    }

    closed spec fn reclaim_prefix_count(&self, request_slot: int, end: int) -> int
        recommends
            0 <= request_slot < MAX_REQUEST_SLOTS,
            0 <= end <= self.requests@[request_slot].page_count,
        decreases end,
    {
        if end == 0 {
            0
        } else {
            let page = self.requests@[request_slot].pages@[end - 1];
            self.reclaim_prefix_count(request_slot, end - 1)
                + if has_other_reference(
                    self.pages@[page.index as int].reference_mask,
                    request_slot as u32,
                ) { 0_int } else { 1_int }
        }
    }

    closed spec fn release_key_enabled(&self, request: RequestKey) -> bool {
        if self.key_matches(request) {
            let slot = self.requests@[request.slot as int];
            let reclaim_count = self.reclaim_prefix_count(
                request.slot as int,
                slot.page_count as int,
            );
            &&& slot.generation < u32::MAX
            &&& self.free_len as int + reclaim_count <= self.page_limit
            &&& forall |position: int|
                0 <= position < slot.page_count
                    && !has_other_reference(
                        self.pages@[slot.pages@[position].index as int].reference_mask,
                        request.slot,
                    ) ==>
                        self.pages@[slot.pages@[position].index as int].generation < u32::MAX
        } else {
            false
        }
    }

    closed spec fn chain_has_page(&self, request_slot: int, page_index: int) -> bool {
        0 <= request_slot < MAX_REQUEST_SLOTS
            && self.requests@[request_slot].live
            && exists |position: int|
                0 <= position < self.requests@[request_slot].page_count
                    && self.requests@[request_slot].pages@[position].index == page_index
    }

    closed spec fn free_stack_has_page(&self, page_index: int) -> bool {
        exists |position: int|
            0 <= position < self.free_len
                && self.free_stack@[position] == page_index
    }

    closed spec fn request_slot_well_formed(&self, request_index: int) -> bool {
        let request = self.requests@[request_index];
        &&& request.generation > 0
        &&& request.committed_tokens <= request.resident_tokens
        &&& request.resident_tokens <= self.max_context_tokens
        &&& request.page_count <= MAX_PAGES_PER_REQUEST
        &&& (!request.live ==> request.committed_tokens == 0
            && request.resident_tokens == 0
            && request.page_count == 0)
        &&& (request.live ==> (
            (request.resident_tokens == 0 <==> request.page_count == 0)
            && (request.page_count > 0 ==> {
                &&& ((request.page_count as int - 1) * self.page_tokens as int)
                    < (request.resident_tokens as int)
                &&& (request.resident_tokens as int)
                    <= (request.page_count as int * self.page_tokens as int)
            })
            && forall |position: int|
                0 <= position < request.page_count ==> {
                    let page = request.pages@[position];
                    &&& #[trigger] request.pages@[position].index < self.page_limit
                    &&& page.generation == self.pages@[page.index as int].generation
                    &&& self.pages@[page.index as int].initialized_tokens
                        == if position + 1 < request.page_count {
                            self.page_tokens
                        } else {
                            (request.resident_tokens as int
                                - position * self.page_tokens as int) as u32
                        }
                    &&& (match self.pages@[page.index as int].state {
                        PageState::Writable { owner_slot } => owner_slot as int == request_index,
                        PageState::Sealed => {
                            (position + 1) * self.page_tokens as int
                                <= request.committed_tokens
                        }
                        PageState::Free => false,
                    })
                }
            && forall |left: int, right: int|
                0 <= left < right < request.page_count ==>
                    #[trigger] logical_pages_distinct(request, left, right)
        ))
    }

    closed spec fn page_slot_well_formed(&self, page_index: int) -> bool {
        let page = self.pages@[page_index];
        &&& page.generation > 0
        &&& (forall |request_index: int|
            0 <= request_index < MAX_REQUEST_SLOTS ==>
                (has_reference(page.reference_mask, request_index as u32)
                    <==> self.chain_has_page(request_index, page_index)))
        &&& (match page.state {
            PageState::Free => {
                page.initialized_tokens == 0
                    && page.reference_mask == 0
                    && self.free_bitmap@[page_index]
            }
            PageState::Writable { owner_slot } => {
                owner_slot < MAX_REQUEST_SLOTS
                    && self.requests@[owner_slot as int].live
                    && has_reference(page.reference_mask, owner_slot as u32)
                    && 0 < page.initialized_tokens <= self.page_tokens
                    && !self.free_bitmap@[page_index]
            }
            PageState::Sealed => {
                (exists |request_index: int|
                    0 <= request_index < MAX_REQUEST_SLOTS
                        && #[trigger] has_reference(
                            page.reference_mask,
                            request_index as u32,
                        ))
                    && page.initialized_tokens == self.page_tokens
                    && !self.free_bitmap@[page_index]
            }
        })
    }

    pub closed spec fn well_formed(&self) -> bool {
        &&& self.pages@.len() == MAX_PAGE_SLOTS
        &&& self.free_stack@.len() == MAX_PAGE_SLOTS
        &&& self.free_bitmap@.len() == MAX_PAGE_SLOTS
        &&& self.requests@.len() == MAX_REQUEST_SLOTS
        &&& 0 < self.page_tokens <= self.max_context_tokens
        &&& 0 < self.page_limit <= MAX_PAGE_SLOTS
        &&& self.free_len <= self.page_limit
        &&& (self.max_context_tokens as int + self.page_tokens as int - 1)
            / self.page_tokens as int <= MAX_PAGES_PER_REQUEST
        &&& (forall |request_index: int|
            0 <= request_index < MAX_REQUEST_SLOTS ==>
                self.request_slot_well_formed(request_index))
        &&& (forall |page_index: int|
            0 <= page_index < self.page_limit ==>
                self.page_slot_well_formed(page_index)
                    && (self.free_bitmap@[page_index]
                        <==> self.free_stack_has_page(page_index)))
        &&& (forall |position: int|
            0 <= position < self.free_len ==>
                self.free_stack@[position] < self.page_limit)
        &&& (forall |left: int, right: int|
            0 <= left < right < self.free_len ==>
                self.free_stack@[left] != self.free_stack@[right])
    }

    pub closed spec fn same_state(&self, other: &Self) -> bool {
        &&& self.page_tokens == other.page_tokens
        &&& self.max_context_tokens == other.max_context_tokens
        &&& self.page_limit == other.page_limit
        &&& self.pages == other.pages
        &&& self.free_stack == other.free_stack
        &&& self.free_len == other.free_len
        &&& self.free_bitmap == other.free_bitmap
        &&& self.requests == other.requests
    }

    pub closed spec fn request_frame_except(&self, old: &Self, changed: int) -> bool {
        forall |request_index: int|
            0 <= request_index < MAX_REQUEST_SLOTS && request_index != changed ==>
                self.requests@[request_index] == old.requests@[request_index]
    }

    pub closed spec fn request_frame_except_two(
        &self,
        old: &Self,
        first: int,
        second: int,
    ) -> bool {
        forall |request_index: int|
            0 <= request_index < MAX_REQUEST_SLOTS
                && request_index != first
                && request_index != second ==>
                    self.requests@[request_index] == old.requests@[request_index]
    }

    pub closed spec fn sealed_payload_frame(&self, old: &Self) -> bool {
        forall |page_index: int|
            0 <= page_index < old.page_limit
                && old.pages@[page_index].state == PageState::Sealed ==>
                    self.pages@[page_index].generation == old.pages@[page_index].generation
                        && self.pages@[page_index].state == PageState::Sealed
                        && self.pages@[page_index].initialized_tokens
                            == old.pages@[page_index].initialized_tokens
    }

    pub closed spec fn exact_sealed_frame(&self, old: &Self) -> bool {
        forall |page_index: int|
            0 <= page_index < old.page_limit
                && old.pages@[page_index].state == PageState::Sealed ==>
                    self.pages@[page_index] == old.pages@[page_index]
    }

    pub closed spec fn reachable_payload_frame_except(&self, old: &Self, excluded: int) -> bool {
        forall |page_index: int|
            0 <= page_index < old.page_limit
                && (exists |request_index: int|
                    0 <= request_index < MAX_REQUEST_SLOTS
                        && request_index != excluded
                        && old.chain_has_page(request_index, page_index)) ==>
                    self.pages@[page_index].generation == old.pages@[page_index].generation
                        && self.pages@[page_index].state == old.pages@[page_index].state
                        && self.pages@[page_index].initialized_tokens
                            == old.pages@[page_index].initialized_tokens
    }

    closed spec fn release_page_matches(&self, old: &Self, page_index: int, released: u32) -> bool {
        let old_page = old.pages@[page_index];
        let new_page = self.pages@[page_index];
        if has_reference(old_page.reference_mask, released) {
            if has_other_reference(old_page.reference_mask, released) {
                &&& new_page.generation == old_page.generation
                &&& new_page.state == old_page.state
                &&& new_page.initialized_tokens == old_page.initialized_tokens
                &&& new_page.reference_mask
                    == (old_page.reference_mask & !(1_u32 << released))
            } else {
                &&& new_page.generation == old_page.generation + 1
                &&& new_page.state == PageState::Free
                &&& new_page.initialized_tokens == 0
                &&& new_page.reference_mask == 0
            }
        } else {
            new_page == old_page
        }
    }

    closed spec fn release_page_frame(&self, old: &Self, released: u32) -> bool {
        forall |page_index: int| 0 <= page_index < old.page_limit ==>
            #[trigger] self.release_page_matches(old, page_index, released)
    }

    closed spec fn source_prefix_has_page(
        &self,
        source: int,
        shared_pages: int,
        page_index: int,
    ) -> bool {
        exists |position: int|
            0 <= position < shared_pages
                && self.requests@[source].pages@[position].index == page_index
    }

    closed spec fn share_page_matches(
        &self,
        old: &Self,
        source: int,
        target: u32,
        shared_pages: int,
        page_index: int,
    ) -> bool {
        if old.source_prefix_has_page(source, shared_pages, page_index) {
            &&& self.pages@[page_index].generation == old.pages@[page_index].generation
            &&& self.pages@[page_index].state == PageState::Sealed
            &&& self.pages@[page_index].initialized_tokens == old.pages@[page_index].initialized_tokens
            &&& self.pages@[page_index].reference_mask
                == (old.pages@[page_index].reference_mask | (1_u32 << target))
        } else {
            self.pages@[page_index] == old.pages@[page_index]
        }
    }

    pub closed spec fn share_page_frame(
        &self,
        old: &Self,
        source: int,
        target: u32,
        shared_pages: int,
    ) -> bool {
        forall |page_index: int| 0 <= page_index < old.page_limit ==>
            #[trigger] self.share_page_matches(old, source, target, shared_pages, page_index)
    }

    proof fn append_frames_transitive(
        first: &Self,
        middle: &Self,
        last: &Self,
        changed: int,
    )
        requires
            first.well_formed(),
            middle.well_formed(),
            last.well_formed(),
            middle.request_frame_except(first, changed),
            last.request_frame_except(middle, changed),
            middle.sealed_payload_frame(first),
            last.sealed_payload_frame(middle),
            middle.exact_sealed_frame(first),
            last.exact_sealed_frame(middle),
            middle.reachable_payload_frame_except(first, changed),
            last.reachable_payload_frame_except(middle, changed),
        ensures
            last.request_frame_except(first, changed),
            last.sealed_payload_frame(first),
            last.exact_sealed_frame(first),
            last.reachable_payload_frame_except(first, changed),
    {
        assert forall |request_index: int|
            0 <= request_index < MAX_REQUEST_SLOTS && request_index != changed implies
                last.requests@[request_index] == first.requests@[request_index] by {
        }
        assert forall |page_index: int|
            0 <= page_index < first.page_limit
                && first.pages@[page_index].state == PageState::Sealed implies
                    last.pages@[page_index].generation == first.pages@[page_index].generation
                        && last.pages@[page_index].state == PageState::Sealed
                        && last.pages@[page_index].initialized_tokens
                            == first.pages@[page_index].initialized_tokens by {
            assert(middle.pages@[page_index].state == PageState::Sealed);
        }
        assert forall |page_index: int|
            0 <= page_index < first.page_limit
                && first.pages@[page_index].state == PageState::Sealed implies
                    last.pages@[page_index] == first.pages@[page_index] by {
            assert(middle.pages@[page_index] == first.pages@[page_index]);
            assert(middle.pages@[page_index].state == PageState::Sealed);
        }
        assert forall |page_index: int|
            0 <= page_index < first.page_limit
                && (exists |request_index: int|
                    0 <= request_index < MAX_REQUEST_SLOTS
                        && request_index != changed
                        && first.chain_has_page(request_index, page_index)) implies
                    last.pages@[page_index].generation == first.pages@[page_index].generation
                        && last.pages@[page_index].state == first.pages@[page_index].state
                        && last.pages@[page_index].initialized_tokens
                            == first.pages@[page_index].initialized_tokens by {
            let request_index = choose |request_index: int|
                0 <= request_index < MAX_REQUEST_SLOTS
                    && request_index != changed
                    && first.chain_has_page(request_index, page_index);
            assert(middle.requests@[request_index] == first.requests@[request_index]);
            assert(middle.chain_has_page(request_index, page_index));
        }
    }

    proof fn positive_factor_product(factor: int, value: int)
        requires factor >= 1, value > 0,
        ensures factor * value >= value,
    {
        vstd::arithmetic::mul::lemma_mul_inequality(1, factor, value);
        vstd::arithmetic::mul::lemma_mul_basics(value);
    }

    proof fn zero_factor_product(factor: int, value: int)
        requires factor == 0,
        ensures factor * value == 0,
    {
        vstd::arithmetic::mul::lemma_mul_basics(value);
    }

    proof fn increment_product(factor: int, value: int)
        ensures (factor + 1) * value == factor * value + value,
    {
        vstd::arithmetic::mul::lemma_mul_is_distributive_add_other_way(value, factor, 1);
        vstd::arithmetic::mul::lemma_mul_basics(value);
    }

    proof fn zero_append_enabled(&self, request: RequestKey)
        requires self.well_formed(), self.key_matches(request),
        ensures self.append_key_enabled(request, 0),
    {
        reveal(KvPool::append_key_enabled);
        reveal(KvPool::request_slot_well_formed);
        let slot = self.requests@[request.slot as int];
        assert(self.request_slot_well_formed(request.slot as int));
        assert(slot.page_count <= MAX_PAGES_PER_REQUEST);
        assert(0 <= self.free_len);
        if slot.page_count > 0 {
            let tail = slot.pages@[slot.page_count - 1];
            assert(tail.index < self.page_limit);
            assert(self.pages@[tail.index as int].initialized_tokens <= self.page_tokens);
        }
        vstd::arithmetic::div_mod::lemma_div_of0(self.page_tokens as int);
        vstd::arithmetic::div_mod::lemma_fundamental_div_mod(0, self.page_tokens as int);
    }

    fn new_bounded(
        page_count: u32,
        page_tokens: u32,
        max_context_tokens: u32,
    ) -> (result: Result<Self, KvError>)
        ensures
            match result {
                Ok(pool) => Self::new_enabled(page_count, page_tokens, max_context_tokens)
                    && pool.well_formed()
                    && forall |slot: int| 0 <= slot < MAX_REQUEST_SLOTS ==> {
                        &&& !pool.request_live_by_slot_spec(slot)
                        &&& pool.request_generation_by_slot_spec(slot) == 1
                    },
                Err(_) => !Self::new_enabled(page_count, page_tokens, max_context_tokens),
            }
    {
        if page_count == 0 { return Err(KvError::ZeroCapacity(Capacity::Pages)); }
        if page_tokens == 0 { return Err(KvError::ZeroCapacity(Capacity::PageTokens)); }
        if max_context_tokens == 0 { return Err(KvError::ZeroCapacity(Capacity::ContextTokens)); }
        if page_count > MAX_PAGE_SLOTS as u32 {
            return Err(KvError::CapacityExceedsBuildBound(Capacity::Pages));
        }
        if page_tokens > max_context_tokens { return Err(KvError::PageExceedsContext); }
        let rounded_context = max_context_tokens as u64 + page_tokens as u64 - 1;
        let required_chain_pages = rounded_context / page_tokens as u64;
        if required_chain_pages > MAX_PAGES_PER_REQUEST as u64 {
            return Err(KvError::CapacityExceedsBuildBound(Capacity::RequestPages));
        }

        let free_page = PageSlot::free();
        let pages = vec![free_page; MAX_PAGE_SLOTS];
        let free_stack = vec![0_u32; MAX_PAGE_SLOTS];
        let free_bitmap = vec![true; MAX_PAGE_SLOTS];
        let empty_request = RequestSlot::empty();
        let requests = vec![empty_request; MAX_REQUEST_SLOTS];
        let mut pool = Self {
            page_tokens,
            max_context_tokens,
            page_limit: page_count,
            pages,
            free_stack,
            free_len: page_count,
            free_bitmap,
            requests,
        };
        let mut index = 0_u32;
        while index < page_count
            invariant
                index <= page_count,
                page_count <= MAX_PAGE_SLOTS,
                pool.page_tokens == page_tokens,
                pool.max_context_tokens == max_context_tokens,
                pool.page_limit == page_count,
                pool.free_len == page_count,
                pool.pages@.len() == MAX_PAGE_SLOTS,
                pool.free_stack@.len() == MAX_PAGE_SLOTS,
                pool.free_bitmap@.len() == MAX_PAGE_SLOTS,
                pool.requests@.len() == MAX_REQUEST_SLOTS,
                forall |position: int|
                    0 <= position < index ==>
                        pool.free_stack@[position] == page_count as int - position - 1,
                forall |request_index: int|
                    0 <= request_index < MAX_REQUEST_SLOTS ==>
                        pool.requests@[request_index].generation == 1
                            && !pool.requests@[request_index].live
                            && pool.requests@[request_index].committed_tokens == 0
                            && pool.requests@[request_index].resident_tokens == 0
                            && pool.requests@[request_index].page_count == 0,
                forall |page_index: int|
                    0 <= page_index < MAX_PAGE_SLOTS ==>
                        pool.pages@[page_index] == free_page
                            && pool.free_bitmap@[page_index],
            decreases page_count - index,
        {
            pool.free_stack[index as usize] = page_count - index - 1;
            index += 1;
        }
        assert forall |page_index: int| 0 <= page_index < page_count implies
            #[trigger] pool.free_stack_has_page(page_index) by {
            let position = page_count as int - page_index - 1;
            assert(0 <= position < pool.free_len);
            assert(pool.free_stack@[position] == page_index);
        }
        assert forall |left: int, right: int|
            0 <= left < right < pool.free_len implies
                #[trigger] free_positions_distinct(&pool, left, right) by {
            assert(pool.free_stack@[left] == page_count as int - left - 1);
            assert(pool.free_stack@[right] == page_count as int - right - 1);
        }
        assert(0 < pool.page_tokens <= pool.max_context_tokens);
        assert(0 < pool.page_limit <= MAX_PAGE_SLOTS);
        assert(pool.free_len <= pool.page_limit);
        assert((pool.max_context_tokens as int + pool.page_tokens as int - 1)
            / pool.page_tokens as int <= MAX_PAGES_PER_REQUEST);
        assert forall |request_index: int|
            0 <= request_index < MAX_REQUEST_SLOTS implies
                #[trigger] pool.request_slot_well_formed(request_index) by {
        }
        assert forall |page_index: int| 0 <= page_index < pool.page_limit implies
            #[trigger] pool.page_slot_well_formed(page_index) by {
            assert forall |request_index: int|
                0 <= request_index < MAX_REQUEST_SLOTS implies
                    (has_reference(pool.pages@[page_index].reference_mask, request_index as u32)
                        <==> pool.chain_has_page(request_index, page_index)) by {
                assert(!pool.requests@[request_index].live);
                assert(!pool.chain_has_page(request_index, page_index));
                zero_reference_lemma(request_index as u32);
                assert(!has_reference(pool.pages@[page_index].reference_mask, request_index as u32));
            }
        }
        assert forall |position: int| 0 <= position < pool.free_len implies
            pool.free_stack@[position] < pool.page_limit by {
            assert(pool.free_stack@[position] == page_count as int - position - 1);
        }
        assert(pool.well_formed());
        Ok(pool)
    }

    fn create_request_key(&mut self, request: RequestKey) -> (result: Result<(), KvError>)
        requires old(self).well_formed(),
        ensures
            final(self).well_formed(),
            final(self).page_tokens_spec() == old(self).page_tokens_spec(),
            final(self).max_context_tokens_spec() == old(self).max_context_tokens_spec(),
            final(self).page_limit_spec() == old(self).page_limit_spec(),
            match result {
                Ok(()) => {
                    &&& old(self).create_key_enabled(request)
                    &&& request.slot < MAX_REQUEST_SLOTS
                    &&& final(self).requests@[request.slot as int].live
                    &&& final(self).requests@[request.slot as int].generation == request.generation
                    &&& final(self).request_frame_except(old(self), request.slot as int)
                }
                Err(_) => {
                    &&& !old(self).create_key_enabled(request)
                    &&& final(self).same_state(old(self))
                }
            },
    {
        if request.slot >= MAX_REQUEST_SLOTS as u32 {
            return Err(KvError::InvalidRequestSlot(request.slot));
        }
        let index = request.slot as usize;
        assert(index < MAX_REQUEST_SLOTS);
        if self.requests[index].live {
            return Err(KvError::RequestSlotOccupied(request.slot));
        }
        if self.requests[index].generation != request.generation {
            return Err(KvError::StaleRequestGeneration {
                slot: request.slot,
                expected: self.requests[index].generation,
                actual: request.generation,
            });
        }
        assert(old(self).request_slot_well_formed(request.slot as int));
        assert(!old(self).requests@[request.slot as int].live);
        assert(old(self).requests@[request.slot as int].page_count == 0);
        self.requests[index].live = true;
        assert(self.requests@[request.slot as int].page_count == 0);
        assert forall |request_index: int|
            0 <= request_index < MAX_REQUEST_SLOTS implies
                #[trigger] self.request_slot_well_formed(request_index) by {
            if request_index == index as int {
                assert(old(self).request_slot_well_formed(request_index));
                assert(old(self).requests@[request_index].committed_tokens == 0);
                assert(old(self).requests@[request_index].resident_tokens == 0);
                assert(old(self).requests@[request_index].page_count == 0);
                assert(self.requests@[request_index].generation
                    == old(self).requests@[request_index].generation);
                assert(self.requests@[request_index].committed_tokens == 0);
                assert(self.requests@[request_index].resident_tokens == 0);
                assert(self.requests@[request_index].page_count == 0);
            } else {
                assert(old(self).request_slot_well_formed(request_index));
                assert(self.requests@[request_index] == old(self).requests@[request_index]);
            }
        }
        assert forall |page_index: int| 0 <= page_index < self.page_limit implies
            #[trigger] self.page_slot_well_formed(page_index) by {
            assert(old(self).page_slot_well_formed(page_index));
            assert(self.pages@[page_index] == old(self).pages@[page_index]);
            assert(self.free_bitmap@[page_index] == old(self).free_bitmap@[page_index]);
            assert forall |request_index: int|
                0 <= request_index < MAX_REQUEST_SLOTS implies
                    (has_reference(self.pages@[page_index].reference_mask, request_index as u32)
                        <==> self.chain_has_page(request_index, page_index)) by {
                if request_index == index as int {
                    assert(self.requests@[request_index].page_count == 0);
                    assert(!self.chain_has_page(request_index, page_index));
                    assert(!old(self).chain_has_page(request_index, page_index));
                } else {
                    assert(self.requests@[request_index] == old(self).requests@[request_index]);
                    assert(self.chain_has_page(request_index, page_index)
                        == old(self).chain_has_page(request_index, page_index));
                }
            }
        }
        assert(self.page_tokens == old(self).page_tokens);
        assert(self.max_context_tokens == old(self).max_context_tokens);
        assert(self.page_limit == old(self).page_limit);
        assert(self.pages == old(self).pages);
        assert(self.free_stack == old(self).free_stack);
        assert(self.free_len == old(self).free_len);
        assert(self.free_bitmap == old(self).free_bitmap);
        assert forall |page_index: int| 0 <= page_index < self.page_limit implies
            (self.free_bitmap@[page_index] <==> self.free_stack_has_page(page_index)) by {
            assert(old(self).free_bitmap@[page_index]
                <==> old(self).free_stack_has_page(page_index));
        }
        assert(self.well_formed());
        Ok(())
    }

    fn append_tentative_key(
        &mut self,
        request: RequestKey,
        token_count: u32,
    ) -> (result: Result<(), KvError>)
        requires old(self).well_formed(),
        ensures
            final(self).well_formed(),
            final(self).page_tokens_spec() == old(self).page_tokens_spec(),
            final(self).max_context_tokens_spec() == old(self).max_context_tokens_spec(),
            final(self).page_limit_spec() == old(self).page_limit_spec(),
            match result {
                Ok(()) => {
                    &&& old(self).append_key_enabled(request, token_count)
                    &&& request.slot < MAX_REQUEST_SLOTS
                    &&& final(self).requests@[request.slot as int].resident_tokens
                        == old(self).requests@[request.slot as int].resident_tokens + token_count
                    &&& final(self).requests@[request.slot as int].committed_tokens
                        == old(self).requests@[request.slot as int].committed_tokens
                    &&& final(self).requests@[request.slot as int].live
                        == old(self).requests@[request.slot as int].live
                    &&& final(self).requests@[request.slot as int].generation
                        == old(self).requests@[request.slot as int].generation
                    &&& final(self).request_frame_except(old(self), request.slot as int)
                    &&& final(self).sealed_payload_frame(old(self))
                    &&& final(self).exact_sealed_frame(old(self))
                    &&& final(self).reachable_payload_frame_except(
                        old(self),
                        request.slot as int,
                    )
                }
                Err(_) => {
                    &&& !old(self).append_key_enabled(request, token_count)
                    &&& final(self).same_state(old(self))
                }
            },
    {
        let request_index = self.live_request_index(request)?;
        reveal(KvPool::append_key_enabled);
        reveal(KvPool::key_matches);
        assert(request_index == request.slot);
        assert(request_index < MAX_REQUEST_SLOTS);
        assert(self.request_slot_well_formed(request_index as int));
        let old_resident = self.requests[request_index].resident_tokens;
        let new_resident = match old_resident.checked_add(token_count) {
            Some(value) => value,
            None => return Err(KvError::ContextExceeded),
        };
        if new_resident > self.max_context_tokens { return Err(KvError::ContextExceeded); }
        if token_count == 0 {
            proof { Self::zero_append_enabled(old(self), request); }
            return Ok(());
        }

        let old_page_count = self.requests[request_index].page_count;
        assert(old_page_count <= MAX_PAGES_PER_REQUEST as u32);
        let tail_page = if old_page_count == 0 {
            PageId::EMPTY
        } else {
            self.requests[request_index].pages[(old_page_count - 1) as usize]
        };
        let (tail_capacity, tail_initialized) = if old_page_count == 0 {
            (0, 0)
        } else {
            let slot = self.page_slot(tail_page)?;
            assert(self.page_slot_well_formed(tail_page.index as int));
            let state = slot.state;
            match state {
                PageState::Writable { owner_slot } => {
                    if owner_slot as u32 != request.slot {
                        return Err(KvError::InvariantViolation(Invariant::PageState));
                    }
                    assert(slot.initialized_tokens <= self.page_tokens);
                    (self.page_tokens - slot.initialized_tokens, slot.initialized_tokens)
                }
                PageState::Sealed => {
                    assert(slot.initialized_tokens == self.page_tokens);
                    (0, slot.initialized_tokens)
                }
                PageState::Free => return Err(KvError::InvariantViolation(Invariant::PageState)),
            }
        };
        let after_tail = token_count.saturating_sub(tail_capacity);
        let full_pages = after_tail / self.page_tokens;
        let partial_tokens = after_tail % self.page_tokens;
        let extra_page = if partial_tokens == 0 { 0_u32 } else { 1_u32 };
        let required_pages = match full_pages.checked_add(extra_page) {
            Some(value) => value,
            None => return Err(KvError::RequestPageTableFull),
        };
        proof {
            vstd::arithmetic::div_mod::lemma_fundamental_div_mod(
                after_tail as int,
                self.page_tokens as int,
            );
            vstd::arithmetic::div_mod::lemma_mod_pos_bound(
                after_tail as int,
                self.page_tokens as int,
            );
        }
        assert(after_tail as int
            == self.page_tokens as int * full_pages as int + partial_tokens as int);
        assert(after_tail as int
            == old(self).page_tokens as int * full_pages as int + partial_tokens as int);
        assert(partial_tokens < self.page_tokens);
        assert(required_pages == full_pages + extra_page);
        let final_page_count = match old_page_count.checked_add(required_pages) {
            Some(value) => value,
            None => return Err(KvError::RequestPageTableFull),
        };
        if final_page_count > MAX_PAGES_PER_REQUEST as u32 {
            return Err(KvError::RequestPageTableFull);
        }
        if required_pages > self.free_len { return Err(KvError::OutOfPages); }

        let tail_written = token_count.min(tail_capacity);
        if old_page_count > 0 {
            let ghost tail_position = old_page_count as int - 1;
            assert(tail_page == old(self).requests@[request.slot as int].pages@[tail_position]);
            assert(old(self).pages@[tail_page.index as int].initialized_tokens == tail_initialized);
            assert(tail_initialized + tail_capacity == self.page_tokens);
            assert(tail_initialized as int
                == old_resident as int - tail_position * self.page_tokens as int);
        }
        let mut remaining = token_count;
        if tail_written > 0 {
            self.append_existing_page(request, request_index, tail_page, tail_written);
            remaining -= tail_written;
        }
        assert(remaining == after_tail);
        assert(self.requests@[request.slot as int].live);
        assert(self.requests@[request.slot as int].generation == request.generation);
        assert(self.requests@[request.slot as int].page_count
            == old(self).requests@[request.slot as int].page_count);
        assert(remaining == after_tail);
        if remaining > 0 {
            if old_page_count == 0 {
                assert(old_resident == 0);
                assert(self.requests@[request.slot as int].resident_tokens == 0);
                assert(self.requests@[request.slot as int].page_count == 0);
            } else {
                assert(token_count > tail_capacity);
                assert(tail_written == tail_capacity);
                assert(self.requests@[request.slot as int].resident_tokens
                    == old_resident + tail_capacity);
                assert(self.requests@[request.slot as int].page_count == old_page_count);
                let ghost tail_position = old_page_count as int - 1;
                assert(tail_initialized as int
                    == old_resident as int - tail_position * self.page_tokens as int);
                assert(old_resident as int
                    == tail_position * self.page_tokens as int + tail_initialized as int);
                assert(tail_position + 1 == old_page_count as int);
                proof { Self::increment_product(tail_position, self.page_tokens as int); }
                assert(tail_position * self.page_tokens as int + self.page_tokens as int
                    == old_page_count as int * self.page_tokens as int);
                assert(self.requests@[request.slot as int].resident_tokens as int
                    == old_page_count as int * self.page_tokens as int);
            }
        }
        assert(remaining > 0 ==> self.requests@[request.slot as int].resident_tokens as int
            == self.requests@[request.slot as int].page_count as int * self.page_tokens as int);
        assert(self.page_tokens == old(self).page_tokens);
        assert(self.max_context_tokens == old(self).max_context_tokens);
        assert(self.page_limit == old(self).page_limit);
        assert(old_resident as int + token_count as int == new_resident as int);
        assert(new_resident <= self.max_context_tokens);
        assert(final_page_count == old_page_count + required_pages);
        assert(final_page_count <= MAX_PAGES_PER_REQUEST as u32);
        assert(required_pages <= old(self).free_len);
        proof {
            vstd::arithmetic::mul::lemma_mul_is_commutative(
                self.page_tokens as int,
                full_pages as int,
            );
        }
        assert(remaining as int
            == (self.page_tokens as int * full_pages as int + partial_tokens as int));
        assert(remaining as int
            == (full_pages as int * self.page_tokens as int + partial_tokens as int));

        let mut allocated = 0_u32;
        while allocated < required_pages
            invariant
                self.well_formed(),
                old(self).well_formed(),
                self.page_tokens == old(self).page_tokens,
                self.max_context_tokens == old(self).max_context_tokens,
                self.page_limit == old(self).page_limit,
                request_index == request.slot,
                request_index < MAX_REQUEST_SLOTS,
                request.slot < MAX_REQUEST_SLOTS,
                self.requests@[request.slot as int].live,
                self.requests@[request.slot as int].generation == request.generation,
                self.requests@[request.slot as int].resident_tokens as int + remaining as int
                    == old(self).requests@[request.slot as int].resident_tokens as int
                        + token_count as int,
                old(self).requests@[request.slot as int].resident_tokens as int
                        + token_count as int
                    <= self.max_context_tokens,
                self.requests@[request.slot as int].committed_tokens
                    == old(self).requests@[request.slot as int].committed_tokens,
                self.request_frame_except(old(self), request.slot as int),
                self.sealed_payload_frame(old(self)),
                self.exact_sealed_frame(old(self)),
                self.reachable_payload_frame_except(old(self), request.slot as int),
                allocated <= required_pages,
                required_pages == full_pages + extra_page,
                extra_page == if partial_tokens == 0 { 0_u32 } else { 1_u32 },
                partial_tokens < self.page_tokens,
                final_page_count == old(self).requests@[request.slot as int].page_count
                    + required_pages,
                final_page_count <= MAX_PAGES_PER_REQUEST as u32,
                required_pages <= old(self).free_len,
                self.free_len as int + allocated as int == old(self).free_len,
                self.requests@[request.slot as int].page_count
                    == old(self).requests@[request.slot as int].page_count + allocated,
                allocated <= full_pages ==> remaining as int
                    == (full_pages as int - allocated as int) * self.page_tokens as int
                        + partial_tokens as int,
                allocated > full_pages ==> {
                    &&& partial_tokens > 0
                    &&& allocated == full_pages + 1
                    &&& remaining == 0
                },
                remaining > 0 ==> self.requests@[request.slot as int].resident_tokens as int
                    == self.requests@[request.slot as int].page_count as int
                        * self.page_tokens as int,
            decreases required_pages - allocated,
        {
            let ghost previous = *self;
            let plan_index = allocated;
            assert(plan_index < required_pages);
            assert(plan_index <= full_pages);
            assert(self.page_tokens > 0);
            let ghost factor = full_pages as int - plan_index as int;
            let ghost page_tokens_int = self.page_tokens as int;
            assert(remaining as int
                == factor * page_tokens_int + partial_tokens as int);
            if plan_index < full_pages {
                assert((plan_index as int) < (full_pages as int));
                assert(factor >= 1);
                assert(0 < page_tokens_int);
                assert(0 <= partial_tokens as int);
                proof { Self::positive_factor_product(factor, page_tokens_int); }
                assert(remaining as int >= self.page_tokens as int);
            } else {
                assert(plan_index == full_pages);
                assert(plan_index as int == full_pages as int);
                assert(factor == 0);
                proof { Self::zero_factor_product(factor, page_tokens_int); }
                assert(extra_page == 1);
                assert(partial_tokens > 0);
                assert(remaining == partial_tokens);
                assert(remaining < self.page_tokens);
            }
            assert(remaining > 0);
            let written = remaining.min(self.page_tokens);
            assert(0 < written <= self.page_tokens);
            assert(written <= remaining);
            if plan_index < full_pages {
                assert(written == self.page_tokens);
            } else {
                assert(written == partial_tokens);
            }
            let previous_remaining = remaining;
            assert((self.requests@[request.slot as int].page_count as int)
                < (final_page_count as int));
            assert((final_page_count as int) <= (MAX_PAGES_PER_REQUEST as int));
            assert(self.requests@[request.slot as int].page_count < MAX_PAGES_PER_REQUEST);
            assert(self.free_len > 0);
            self.append_fresh_page(request, request_index, written);
            remaining -= written;
            assert(remaining as int == previous_remaining as int - written as int);
            allocated += 1;
            assert(allocated == plan_index + 1);
            proof {
                Self::append_frames_transitive(
                    old(self),
                    &previous,
                    self,
                    request.slot as int,
                );
            }
            if plan_index < full_pages {
                assert(written == self.page_tokens);
                assert(allocated <= full_pages);
                assert(previous.requests@[request.slot as int].resident_tokens as int
                    + written as int
                    == self.requests@[request.slot as int].resident_tokens as int);
                assert(previous.requests@[request.slot as int].page_count + 1
                    == self.requests@[request.slot as int].page_count);
                let ghost old_factor = full_pages as int - plan_index as int;
                let ghost new_factor = full_pages as int - allocated as int;
                assert(new_factor == old_factor - 1);
                assert(previous_remaining as int
                    == old_factor * self.page_tokens as int + partial_tokens as int);
                assert(remaining as int
                    == previous_remaining as int - self.page_tokens as int);
                assert(old_factor * self.page_tokens as int - self.page_tokens as int
                    == (old_factor - 1) * self.page_tokens as int) by (nonlinear_arith);
                assert(remaining as int
                    == new_factor * self.page_tokens as int + partial_tokens as int);
            } else {
                assert(plan_index == full_pages);
                assert(partial_tokens > 0);
                assert(written == partial_tokens);
                assert(allocated == full_pages + 1);
                assert(remaining == 0);
            }
            if remaining > 0 {
                assert(plan_index < full_pages);
                assert(written == self.page_tokens);
                assert(previous.requests@[request.slot as int].resident_tokens as int
                    == previous.requests@[request.slot as int].page_count as int
                        * self.page_tokens as int);
                assert(self.requests@[request.slot as int].resident_tokens as int
                    == previous.requests@[request.slot as int].resident_tokens as int
                        + self.page_tokens as int);
                assert(self.requests@[request.slot as int].page_count as int
                    == previous.requests@[request.slot as int].page_count as int + 1);
                let ghost previous_count = previous.requests@[request.slot as int].page_count as int;
                proof { Self::increment_product(previous_count, self.page_tokens as int); }
                assert(self.requests@[request.slot as int].resident_tokens as int
                    == previous_count * self.page_tokens as int + self.page_tokens as int);
                assert(self.requests@[request.slot as int].page_count as int
                    == previous_count + 1);
                assert(self.requests@[request.slot as int].resident_tokens as int
                    == self.requests@[request.slot as int].page_count as int
                        * self.page_tokens as int);
            }
        }
        assert(allocated == required_pages);
        if partial_tokens == 0 {
            assert(extra_page == 0);
            assert(allocated == full_pages);
            assert(allocated <= full_pages);
            assert(remaining as int
                == (full_pages as int - allocated as int) * self.page_tokens as int
                    + partial_tokens as int);
            assert(remaining == 0);
        } else {
            assert(extra_page == 1);
            assert(allocated == full_pages + 1);
            assert(allocated > full_pages);
            assert(remaining == 0);
        }
        Ok(())
    }

    fn share_committed_prefix_key(
        &mut self,
        source: RequestKey,
        target: RequestKey,
        token_count: u32,
    ) -> (result: Result<(), KvError>)
        requires old(self).well_formed(),
        ensures
            final(self).well_formed(),
            final(self).page_tokens_spec() == old(self).page_tokens_spec(),
            final(self).max_context_tokens_spec() == old(self).max_context_tokens_spec(),
            final(self).page_limit_spec() == old(self).page_limit_spec(),
            match result {
                Ok(()) => {
                    &&& old(self).share_key_enabled(source, target, token_count)
                    &&& target.slot < MAX_REQUEST_SLOTS
                    &&& final(self).requests@[target.slot as int].resident_tokens == token_count
                    &&& final(self).requests@[target.slot as int].committed_tokens == token_count
                    &&& final(self).requests@[target.slot as int].page_count
                        == token_count / final(self).page_tokens
                    &&& final(self).requests@[target.slot as int].live
                        == old(self).requests@[target.slot as int].live
                    &&& final(self).requests@[target.slot as int].generation
                        == old(self).requests@[target.slot as int].generation
                    &&& final(self).requests@[source.slot as int]
                        == old(self).requests@[source.slot as int]
                    &&& forall |request_index: int|
                        0 <= request_index < MAX_REQUEST_SLOTS
                            && request_index != source.slot
                            && request_index != target.slot ==>
                                final(self).requests@[request_index]
                                    == old(self).requests@[request_index]
                    &&& final(self).sealed_payload_frame(old(self))
                    &&& final(self).share_page_frame(
                        old(self),
                        source.slot as int,
                        target.slot,
                        token_count as int / old(self).page_tokens as int,
                    )
                    &&& forall |position: int|
                        0 <= position < final(self).requests@[target.slot as int].page_count ==>
                            final(self).requests@[target.slot as int].pages@[position]
                                == old(self).requests@[source.slot as int].pages@[position]
                }
                Err(_) => {
                    &&& !old(self).share_key_enabled(source, target, token_count)
                    &&& final(self).same_state(old(self))
                }
            },
    {
        let source_index = self.live_request_index(source)?;
        let target_index = self.live_request_index(target)?;
        if source_index == target_index { return Err(KvError::SameRequestShare); }
        if token_count == 0 || token_count % self.page_tokens != 0 {
            return Err(KvError::PrefixNotPageAligned);
        }
        if token_count > self.requests[source_index].committed_tokens {
            return Err(KvError::PrefixExceedsCommitted);
        }
        if self.requests[target_index].resident_tokens != 0 { return Err(KvError::TargetNotEmpty); }
        let shared_pages = token_count / self.page_tokens;
        if shared_pages as usize > MAX_PAGES_PER_REQUEST { return Err(KvError::RequestPageTableFull); }
        proof {
            vstd::arithmetic::div_mod::lemma_fundamental_div_mod(
                token_count as int,
                self.page_tokens as int,
            );
        }
        assert(token_count as int == self.page_tokens as int * shared_pages as int);
        assert(token_count as int == shared_pages as int * self.page_tokens as int) by {
            vstd::arithmetic::mul::lemma_mul_is_commutative(
                self.page_tokens as int,
                shared_pages as int,
            );
        }
        reveal(KvPool::request_slot_well_formed);
        reveal(KvPool::share_page_frame);
        reveal(KvPool::share_page_matches);
        reveal(KvPool::source_prefix_has_page);
        assert(self.request_slot_well_formed(source_index as int));
        assert(self.request_slot_well_formed(target_index as int));
        assert(shared_pages as int * self.page_tokens as int
            <= self.requests@[source_index as int].page_count as int * self.page_tokens as int);
        proof {
            vstd::arithmetic::mul::lemma_mul_inequality_converse(
                shared_pages as int,
                self.requests@[source_index as int].page_count as int,
                self.page_tokens as int,
            );
        }
        assert(shared_pages <= self.requests@[source_index as int].page_count);
        assert(self.requests@[target_index as int].page_count == 0);
        assert(self.requests@[target_index as int].resident_tokens == 0);
        assert(self.requests@[target_index as int].committed_tokens == 0);
        assert(self.share_page_frame(self, source.slot as int, target.slot, 0));
        proof { vstd::arithmetic::mul::lemma_mul_basics(self.page_tokens as int); }
        assert(0_int * self.page_tokens as int == 0);

        let mut position = 0_u32;
        while position < shared_pages
            invariant
                self.well_formed(),
                old(self).well_formed(),
                self.page_tokens == old(self).page_tokens,
                self.max_context_tokens == old(self).max_context_tokens,
                self.page_limit == old(self).page_limit,
                self.free_stack == old(self).free_stack,
                self.free_len == old(self).free_len,
                self.free_bitmap == old(self).free_bitmap,
                source_index < MAX_REQUEST_SLOTS,
                target_index < MAX_REQUEST_SLOTS,
                source_index == source.slot,
                target_index == target.slot,
                source_index != target_index,
                position <= shared_pages,
                shared_pages <= MAX_PAGES_PER_REQUEST,
                token_count as int == self.page_tokens as int * shared_pages as int,
                token_count as int == shared_pages as int * self.page_tokens as int,
                token_count <= old(self).requests@[source_index as int].committed_tokens,
                shared_pages <= old(self).requests@[source_index as int].page_count,
                old(self).request_slot_well_formed(source_index as int),
                old(self).request_slot_well_formed(target_index as int),
                old(self).requests@[source_index as int].live,
                old(self).requests@[target_index as int].live,
                self.requests@[source_index as int]
                    == old(self).requests@[source_index as int],
                self.requests@[target_index as int].generation
                    == old(self).requests@[target_index as int].generation,
                self.requests@[target_index as int].live,
                self.requests@[target_index as int].page_count == position,
                self.requests@[target_index as int].resident_tokens as int
                    == position as int * self.page_tokens as int,
                self.requests@[target_index as int].committed_tokens
                    == self.requests@[target_index as int].resident_tokens,
                forall |prior: int| 0 <= prior < position ==>
                    self.requests@[target_index as int].pages@[prior]
                        == old(self).requests@[source_index as int].pages@[prior],
                forall |request_index: int|
                    0 <= request_index < MAX_REQUEST_SLOTS
                        && request_index != source_index
                        && request_index != target_index ==>
                            self.requests@[request_index] == old(self).requests@[request_index],
                self.share_page_frame(
                    old(self),
                    source_index as int,
                    target.slot,
                    position as int,
                ),
            decreases shared_pages - position,
        {
            let ghost previous = *self;
            assert(position < MAX_PAGES_PER_REQUEST);
            assert((position as int + 1) <= shared_pages as int);
            proof {
                vstd::arithmetic::mul::lemma_mul_inequality(
                    position as int + 1,
                    shared_pages as int,
                    self.page_tokens as int,
                );
            }
            assert(token_count as int == shared_pages as int * self.page_tokens as int);
            assert((position as int + 1) * self.page_tokens as int
                <= token_count as int);
            assert((position as int + 1) * self.page_tokens as int
                <= self.requests@[source_index as int].committed_tokens);
            assert(self.requests@[target_index as int].resident_tokens
                    + self.page_tokens
                <= self.max_context_tokens) by {
                Self::increment_product(position as int, self.page_tokens as int);
                assert(self.requests@[target_index as int].resident_tokens as int
                        + self.page_tokens as int
                    == (position as int + 1) * self.page_tokens as int);
                assert(token_count <= old(self).requests@[source_index as int].committed_tokens);
                assert(old(self).requests@[source_index as int].committed_tokens
                    <= old(self).requests@[source_index as int].resident_tokens);
                assert(old(self).requests@[source_index as int].resident_tokens
                    <= self.max_context_tokens);
            }
            self.share_next_page(source_index, target_index, position);
            position += 1;
            assert(self.requests@[target_index as int].resident_tokens as int
                == position as int * self.page_tokens as int) by {
                Self::increment_product(position as int - 1, self.page_tokens as int);
                assert(previous.requests@[target_index as int].resident_tokens as int
                    == (position as int - 1) * self.page_tokens as int);
            }
            assert forall |page_index: int| 0 <= page_index < old(self).page_limit implies
                #[trigger] self.share_page_matches(
                    old(self),
                    source_index as int,
                    target.slot,
                    position as int,
                    page_index,
                ) by {
                let current = old(self).requests@[source_index as int].pages@[position as int - 1];
                if old(self).source_prefix_has_page(
                    source_index as int,
                    position as int,
                    page_index,
                ) {
                    let shared_position = choose |shared_position: int|
                        0 <= shared_position < position
                            && old(self).requests@[source_index as int]
                                .pages@[shared_position].index == page_index;
                    if shared_position + 1 < position {
                        assert(old(self).source_prefix_has_page(
                            source_index as int,
                            position as int - 1,
                            page_index,
                        ));
                        assert(current.index != page_index) by {
                            assert(old(self).request_slot_well_formed(source_index as int));
                            assert(logical_pages_distinct(
                                old(self).requests@[source_index as int],
                                shared_position,
                                position as int - 1,
                            ));
                        }
                        assert(previous.share_page_matches(
                            old(self),
                            source_index as int,
                            target.slot,
                            position as int - 1,
                            page_index,
                        ));
                        assert(self.pages@[page_index] == previous.pages@[page_index]);
                    } else {
                        assert(shared_position == position as int - 1);
                        assert(page_index == current.index);
                        assert(!old(self).source_prefix_has_page(
                            source_index as int,
                            position as int - 1,
                            page_index,
                        )) by {
                            if old(self).source_prefix_has_page(
                                source_index as int,
                                position as int - 1,
                                page_index,
                            ) {
                                let prior = choose |prior: int|
                                    0 <= prior < position as int - 1
                                        && old(self).requests@[source_index as int]
                                            .pages@[prior].index == page_index;
                                assert(old(self).request_slot_well_formed(source_index as int));
                                assert(logical_pages_distinct(
                                    old(self).requests@[source_index as int],
                                    prior,
                                    position as int - 1,
                                ));
                            }
                        }
                        assert(previous.share_page_matches(
                            old(self),
                            source_index as int,
                            target.slot,
                            position as int - 1,
                            page_index,
                        ));
                        assert(previous.pages@[page_index] == old(self).pages@[page_index]);
                        assert(self.pages@[page_index].generation
                            == previous.pages@[page_index].generation);
                        assert(self.pages@[page_index].state == PageState::Sealed);
                        assert(self.pages@[page_index].initialized_tokens
                            == previous.pages@[page_index].initialized_tokens);
                        assert(self.pages@[page_index].reference_mask
                            == previous.pages@[page_index].reference_mask
                                | (1_u32 << target.slot));
                    }
                } else {
                    assert(!old(self).source_prefix_has_page(
                        source_index as int,
                        position as int - 1,
                        page_index,
                    ));
                    assert(page_index != current.index);
                    assert(previous.share_page_matches(
                        old(self),
                        source_index as int,
                        target.slot,
                        position as int - 1,
                        page_index,
                    ));
                    assert(previous.pages@[page_index] == old(self).pages@[page_index]);
                    assert(self.pages@[page_index] == previous.pages@[page_index]);
                }
            }
        }
        assert(position == shared_pages);
        assert(self.requests@[target_index as int].resident_tokens == token_count);
        assert(self.requests@[target_index as int].committed_tokens == token_count);
        assert(self.sealed_payload_frame(old(self))) by {
            assert forall |page_index: int|
                0 <= page_index < old(self).page_limit
                    && old(self).pages@[page_index].state == PageState::Sealed implies
                        self.pages@[page_index].generation
                                == old(self).pages@[page_index].generation
                            && self.pages@[page_index].state == PageState::Sealed
                            && self.pages@[page_index].initialized_tokens
                                == old(self).pages@[page_index].initialized_tokens by {
                if old(self).source_prefix_has_page(
                    source_index as int,
                    shared_pages as int,
                    page_index,
                ) {
                    assert(self.share_page_matches(
                        old(self),
                        source_index as int,
                        target.slot,
                        shared_pages as int,
                        page_index,
                    ));
                } else {
                    assert(self.share_page_matches(
                        old(self),
                        source_index as int,
                        target.slot,
                        shared_pages as int,
                        page_index,
                    ));
                    assert(self.pages@[page_index] == old(self).pages@[page_index]);
                }
            }
        }
        Ok(())
    }

    fn finalize_tentative_key(
        &mut self,
        request: RequestKey,
        accepted_tokens: u32,
    ) -> (result: Result<(), KvError>)
        requires old(self).well_formed(),
        ensures
            final(self).well_formed(),
            match result {
                Ok(()) => {
                    &&& request.slot < MAX_REQUEST_SLOTS
                    &&& final(self).requests@[request.slot as int].resident_tokens
                        == final(self).requests@[request.slot as int].committed_tokens
                    &&& final(self).requests@[request.slot as int].committed_tokens
                        == old(self).requests@[request.slot as int].committed_tokens
                            + accepted_tokens
                    &&& final(self).request_frame_except(old(self), request.slot as int)
                    &&& final(self).reachable_payload_frame_except(
                        old(self),
                        request.slot as int,
                    )
                }
                Err(_) => final(self).same_state(old(self)),
            },
    {
        let request_index = self.live_request_index(request)?;
        let committed = match self.requests[request_index]
            .committed_tokens
            .checked_add(accepted_tokens)
        {
            Some(value) => value,
            None => return Err(KvError::CommitExceedsResident),
        };
        if committed > self.requests[request_index].resident_tokens {
            return Err(KvError::CommitExceedsResident);
        }
        let retained_pages = if committed == 0 { 0 } else {
            (committed / self.page_tokens) + u32::from(committed % self.page_tokens != 0)
        };
        let old_page_count = self.requests[request_index].page_count;
        let reclaim_count = old_page_count - retained_pages;
        if self.free_len.checked_add(reclaim_count).is_none()
            || self.free_len + reclaim_count > self.page_limit
        {
            return Err(KvError::InvariantViolation(Invariant::FreeStack));
        }

        if retained_pages > 0 && committed % self.page_tokens != 0 {
            let tail = self.requests[request_index].pages[(retained_pages - 1) as usize];
            let slot = self.page_slot(tail)?;
            let state = slot.state;
            match state {
                PageState::Writable { owner_slot } => {
                    if owner_slot as u32 != request.slot {
                        return Err(KvError::InvariantViolation(Invariant::TentativePage));
                    }
                }
                _ => return Err(KvError::InvariantViolation(Invariant::TentativePage)),
            }
        }

        let mut position = retained_pages;
        while position < old_page_count
            decreases old_page_count - position,
        {
            let page = self.requests[request_index].pages[position as usize];
            let slot = self.page_slot(page)?;
            if slot.generation == u32::MAX { return Err(KvError::GenerationExhausted(page)); }
            let state = slot.state;
            match state {
                PageState::Writable { owner_slot } => {
                    if owner_slot as u32 != request.slot
                        || !reference_is_set(slot.reference_mask, request.slot)
                    {
                        return Err(KvError::InvariantViolation(Invariant::TentativePage));
                    }
                }
                PageState::Sealed | PageState::Free => {
                    return Err(KvError::InvariantViolation(Invariant::TentativePage));
                }
            }
            position += 1;
        }

        position = retained_pages;
        while position < old_page_count
            decreases old_page_count - position,
        {
            let page = self.requests[request_index].pages[position as usize];
            self.reclaim_page_unchecked(page.index as usize);
            self.requests[request_index].pages[position as usize] = PageId::EMPTY;
            position += 1;
        }
        self.requests[request_index].page_count = retained_pages;
        self.requests[request_index].resident_tokens = committed;
        self.requests[request_index].committed_tokens = committed;
        if retained_pages > 0 && committed % self.page_tokens != 0 {
            let tail = self.requests[request_index].pages[(retained_pages - 1) as usize];
            let tail_index = tail.index as usize;
            self.pages[tail_index].initialized_tokens = committed % self.page_tokens;
        }
        Ok(())
    }

    fn release_request_key(
        &mut self,
        request: RequestKey,
    ) -> (result: Result<(), KvError>)
        requires old(self).well_formed(),
        ensures
            final(self).well_formed(),
            match result {
                Ok(()) => {
                    &&& request.slot < MAX_REQUEST_SLOTS
                    &&& !final(self).requests@[request.slot as int].live
                    &&& final(self).requests@[request.slot as int].generation
                        == old(self).requests@[request.slot as int].generation + 1
                    &&& final(self).request_frame_except(old(self), request.slot as int)
                    &&& final(self).release_page_frame(old(self), request.slot)
                }
                Err(_) => final(self).same_state(old(self)),
            },
    {
        let request_index = self.live_request_index(request)?;
        if self.requests[request_index].generation == u32::MAX {
            return Err(KvError::RequestGenerationExhausted(request.slot));
        }
        let page_count = self.requests[request_index].page_count;
        let mut reclaim = [false; MAX_PAGES_PER_REQUEST];
        let mut reclaim_count = 0_u32;

        let mut position = 0_u32;
        while position < page_count
            decreases page_count - position,
        {
            let page = self.requests[request_index].pages[position as usize];
            let slot = self.page_slot(page)?;
            let state = slot.state;
            match state {
                PageState::Writable { owner_slot } => {
                    if owner_slot as u32 != request.slot
                        || !reference_is_set(slot.reference_mask, request.slot)
                    {
                        return Err(KvError::InvariantViolation(Invariant::ReferenceCount));
                    }
                    if slot.generation == u32::MAX { return Err(KvError::GenerationExhausted(page)); }
                    reclaim[position as usize] = true;
                    reclaim_count += 1;
                }
                PageState::Sealed => {
                    if !reference_is_set(slot.reference_mask, request.slot) {
                        return Err(KvError::InvariantViolation(Invariant::ReferenceCount));
                    }
                    let shared = self.page_has_other_reference(page.index as usize, request.slot);
                    if !shared && slot.generation == u32::MAX {
                        return Err(KvError::GenerationExhausted(page));
                    }
                    if !shared {
                        reclaim[position as usize] = true;
                        reclaim_count += 1;
                    }
                }
                PageState::Free => return Err(KvError::InvariantViolation(Invariant::ReferenceCount)),
            }
            position += 1;
        }
        if self.free_len.checked_add(reclaim_count).is_none()
            || self.free_len + reclaim_count > self.page_limit
        {
            return Err(KvError::InvariantViolation(Invariant::FreeStack));
        }

        position = 0;
        while position < page_count
            decreases page_count - position,
        {
            let page = self.requests[request_index].pages[position as usize];
            let page_index = page.index as usize;
            if reclaim[position as usize] {
                self.reclaim_page_unchecked(page_index);
            } else {
                self.pages[page_index].reference_mask = clear_reference(
                    self.pages[page_index].reference_mask,
                    request.slot,
                );
            }
            self.requests[request_index].pages[position as usize] = PageId::EMPTY;
            position += 1;
        }

        let slot = &mut self.requests[request_index];
        slot.live = false;
        slot.committed_tokens = 0;
        slot.resident_tokens = 0;
        slot.page_count = 0;
        slot.generation += 1;
        Ok(())
    }

    fn validate_read_key(
        &self,
        request: RequestKey,
        logical_offset: u32,
        span: u32,
    ) -> (result: Result<(), KvError>)
        requires self.well_formed(),
        ensures
            self.well_formed(),
            match result {
                Ok(()) => {
                    &&& self.read_key_enabled(request, logical_offset, span)
                    &&& logical_offset as int + span as int
                        <= self.requests@[request.slot as int].resident_tokens
                }
                Err(_) => !self.read_key_enabled(request, logical_offset, span),
            },
    {
        let request_index = self.live_request_index(request)?;
        let end = match logical_offset.checked_add(span) {
            Some(value) => value,
            None => return Err(KvError::ReadOutOfBounds),
        };
        if end > self.requests[request_index].resident_tokens { return Err(KvError::ReadOutOfBounds); }
        Ok(())
    }

    fn live_request_index(&self, request: RequestKey) -> (result: Result<usize, KvError>)
        requires self.well_formed(),
        ensures
            match result {
                Ok(index) => {
                    &&& self.key_matches(request)
                    &&& index < MAX_REQUEST_SLOTS
                    &&& index == request.slot
                    &&& self.requests@[index as int].live
                    &&& self.requests@[index as int].generation == request.generation
                }
                Err(_) => !self.key_matches(request),
            },
    {
        let index = request.slot as usize;
        if index >= MAX_REQUEST_SLOTS { return Err(KvError::InvalidRequestSlot(request.slot)); }
        let slot = &self.requests[index];
        if !slot.live { return Err(KvError::UnknownRequest(request.slot)); }
        if slot.generation != request.generation {
            return Err(KvError::StaleRequestGeneration {
                slot: request.slot,
                expected: slot.generation,
                actual: request.generation,
            });
        }
        Ok(index)
    }

    fn page_slot(&self, page: PageId) -> (result: Result<&PageSlot, KvError>)
        requires self.well_formed(),
        ensures
            match result {
                Ok(slot) => {
                    &&& page.index < self.page_limit
                    &&& *slot == self.pages@[page.index as int]
                    &&& slot.generation == page.generation
                }
                Err(_) => {
                    ||| page.index >= self.page_limit
                    ||| (page.index < self.page_limit
                        && self.pages@[page.index as int].generation != page.generation)
                }
            },
    {
        let index = page.index as usize;
        if index >= self.page_limit as usize { return Err(KvError::InvalidPage(page)); }
        let slot = &self.pages[index];
        if slot.generation != page.generation { return Err(KvError::StalePage(page)); }
        Ok(slot)
    }

    fn page_has_other_reference(&self, page_index: usize, excluded_slot: u32) -> (found: bool)
        requires
            page_index < self.page_limit,
            self.page_limit <= MAX_PAGE_SLOTS,
            excluded_slot < MAX_REQUEST_SLOTS,
        ensures
            found == has_other_reference(
                self.pages@[page_index as int].reference_mask,
                excluded_slot,
            ),
    {
        (self.pages[page_index].reference_mask & !(1_u32 << excluded_slot)) != 0
    }

    fn share_next_page(
        &mut self,
        source_index: usize,
        target_index: usize,
        position: u32,
    )
        requires
            old(self).well_formed(),
            source_index < MAX_REQUEST_SLOTS,
            target_index < MAX_REQUEST_SLOTS,
            source_index != target_index,
            old(self).requests@[source_index as int].live,
            old(self).requests@[target_index as int].live,
            position < MAX_PAGES_PER_REQUEST,
            position < old(self).requests@[source_index as int].page_count,
            position == old(self).requests@[target_index as int].page_count,
            old(self).requests@[target_index as int].resident_tokens as int
                == position as int * old(self).page_tokens as int,
            old(self).requests@[target_index as int].committed_tokens
                == old(self).requests@[target_index as int].resident_tokens,
            (position as int + 1) * old(self).page_tokens as int
                <= old(self).requests@[source_index as int].committed_tokens,
            old(self).requests@[target_index as int].resident_tokens
                    + old(self).page_tokens
                <= old(self).max_context_tokens,
            forall |prior: int| 0 <= prior < position ==>
                old(self).requests@[target_index as int].pages@[prior]
                    == old(self).requests@[source_index as int].pages@[prior],
        ensures
            final(self).well_formed(),
            final(self).page_tokens == old(self).page_tokens,
            final(self).max_context_tokens == old(self).max_context_tokens,
            final(self).page_limit == old(self).page_limit,
            final(self).free_stack == old(self).free_stack,
            final(self).free_len == old(self).free_len,
            final(self).free_bitmap == old(self).free_bitmap,
            final(self).requests@[source_index as int]
                == old(self).requests@[source_index as int],
            final(self).requests@[target_index as int].generation
                == old(self).requests@[target_index as int].generation,
            final(self).requests@[target_index as int].live,
            final(self).requests@[target_index as int].page_count == position + 1,
            final(self).requests@[target_index as int].resident_tokens
                == old(self).requests@[target_index as int].resident_tokens
                    + old(self).page_tokens,
            final(self).requests@[target_index as int].committed_tokens
                == final(self).requests@[target_index as int].resident_tokens,
            forall |prior: int| 0 <= prior <= position ==>
                final(self).requests@[target_index as int].pages@[prior]
                    == old(self).requests@[source_index as int].pages@[prior],
            forall |request_index: int|
                0 <= request_index < MAX_REQUEST_SLOTS
                    && request_index != source_index
                    && request_index != target_index ==>
                        final(self).requests@[request_index] == old(self).requests@[request_index],
            {
                let page = old(self).requests@[source_index as int].pages@[position as int];
                &&& final(self).pages@[page.index as int].generation
                    == old(self).pages@[page.index as int].generation
                &&& final(self).pages@[page.index as int].state == PageState::Sealed
                &&& final(self).pages@[page.index as int].initialized_tokens
                    == old(self).pages@[page.index as int].initialized_tokens
                &&& final(self).pages@[page.index as int].reference_mask
                    == old(self).pages@[page.index as int].reference_mask
                        | (1_u32 << target_index as u32)
            },
            forall |page_index: int|
                0 <= page_index < old(self).page_limit
                    && page_index
                        != old(self).requests@[source_index as int].pages@[position as int].index ==>
                            final(self).pages@[page_index] == old(self).pages@[page_index],
    {
        reveal(KvPool::request_slot_well_formed);
        reveal(KvPool::page_slot_well_formed);
        reveal(KvPool::chain_has_page);
        reveal(KvPool::free_stack_has_page);
        assert(old(self).requests@.len() == MAX_REQUEST_SLOTS);
        assert(old(self).pages@.len() == MAX_PAGE_SLOTS);
        assert(old(self).request_slot_well_formed(source_index as int));
        assert(old(self).request_slot_well_formed(target_index as int));
        let page = self.requests[source_index].pages[position as usize];
        assert((position as usize) < MAX_PAGES_PER_REQUEST);
        assert(self.requests@[source_index as int].pages@.len() == MAX_PAGES_PER_REQUEST);
        let page_index = page.index as usize;
        assert(page.index < self.page_limit);
        assert(self.page_slot_well_formed(page.index as int));
        assert(self.pages@[page.index as int].generation == page.generation);
        assert(self.pages@[page.index as int].initialized_tokens == self.page_tokens) by {
            if position + 1 < self.requests@[source_index as int].page_count {
            } else {
                assert(position + 1 == self.requests@[source_index as int].page_count);
                assert(self.requests@[source_index as int].resident_tokens as int
                    == self.requests@[source_index as int].page_count as int
                        * self.page_tokens as int);
                assert(self.pages@[page.index as int].initialized_tokens as int
                    == self.requests@[source_index as int].resident_tokens as int
                        - position as int * self.page_tokens as int);
                Self::increment_product(position as int, self.page_tokens as int);
            }
        }
        assert(!self.chain_has_page(target_index as int, page.index as int)) by {
            assert forall |prior: int|
                0 <= prior < self.requests@[target_index as int].page_count implies
                    self.requests@[target_index as int].pages@[prior].index != page.index by {
                assert(self.requests@[target_index as int].pages@[prior]
                    == self.requests@[source_index as int].pages@[prior]);
                assert(prior < position);
                assert(logical_pages_distinct(
                    self.requests@[source_index as int],
                    prior,
                    position as int,
                ));
            }
        }
        assert(!has_reference(
            self.pages@[page.index as int].reference_mask,
            target_index as u32,
        ));
        let state = self.pages[page_index].state;
        match state {
            PageState::Writable { owner_slot } => {
                assert(owner_slot as usize == source_index);
                self.pages[page_index].state = PageState::Sealed;
            }
            PageState::Sealed => {}
            PageState::Free => { assert(false); }
        }
        self.pages[page_index].reference_mask =
            set_reference(self.pages[page_index].reference_mask, target_index as u32);
        self.requests[target_index].pages[position as usize] = page;
        self.requests[target_index].page_count += 1;
        self.requests[target_index].resident_tokens += self.page_tokens;
        self.requests[target_index].committed_tokens += self.page_tokens;

        assert forall |index: int| 0 <= index < MAX_REQUEST_SLOTS implies
            #[trigger] self.request_slot_well_formed(index) by {
            assert(old(self).request_slot_well_formed(index));
            if index == target_index {
                let count = self.requests@[index].page_count as int;
                let resident = self.requests@[index].resident_tokens as int;
                let page_tokens = self.page_tokens as int;
                assert(count == position as int + 1);
                Self::increment_product(position as int, page_tokens);
                assert(resident == count * page_tokens);
                assert((count - 1) * page_tokens < resident);
                assert forall |logical: int| 0 <= logical < count implies {
                    let logical_page = self.requests@[index].pages@[logical];
                    &&& #[trigger] self.requests@[index].pages@[logical].index < self.page_limit
                    &&& logical_page.generation
                        == self.pages@[logical_page.index as int].generation
                    &&& self.pages@[logical_page.index as int].initialized_tokens
                        == if logical + 1 < count {
                            self.page_tokens
                        } else {
                            (resident - logical * page_tokens) as u32
                        }
                    &&& (match self.pages@[logical_page.index as int].state {
                        PageState::Writable { owner_slot } => owner_slot as int == index,
                        PageState::Sealed => (logical + 1) * page_tokens
                            <= self.requests@[index].committed_tokens,
                        PageState::Free => false,
                    })
                } by {
                    if logical < position {
                        assert(self.requests@[index].pages@[logical]
                            == old(self).requests@[source_index as int].pages@[logical]);
                        assert(old(self).requests@[target_index as int].pages@[logical]
                            == old(self).requests@[source_index as int].pages@[logical]);
                        assert(old(self).request_slot_well_formed(target_index as int));
                    } else {
                        assert(logical == position);
                        assert(self.requests@[index].pages@[logical] == page);
                        assert(self.pages@[page.index as int].state == PageState::Sealed);
                        assert(self.pages@[page.index as int].initialized_tokens == self.page_tokens);
                        assert((logical + 1) * page_tokens == resident);
                    }
                }
                assert forall |left: int, right: int| 0 <= left < right < count implies
                    #[trigger] logical_pages_distinct(self.requests@[index], left, right) by {
                    if right < position {
                        assert(self.requests@[index].pages@[left]
                            == old(self).requests@[source_index as int].pages@[left]);
                        assert(self.requests@[index].pages@[right]
                            == old(self).requests@[source_index as int].pages@[right]);
                    } else {
                        assert(right == position);
                        assert(self.requests@[index].pages@[left]
                            == old(self).requests@[source_index as int].pages@[left]);
                        assert(self.requests@[index].pages@[right]
                            == old(self).requests@[source_index as int].pages@[position as int]);
                    }
                    assert(logical_pages_distinct(
                        old(self).requests@[source_index as int],
                        left,
                        right,
                    ));
                }
            } else if index == source_index {
                assert(self.requests@[index] == old(self).requests@[index]);
                assert forall |logical: int|
                    0 <= logical < self.requests@[index].page_count implies {
                        let logical_page = self.requests@[index].pages@[logical];
                        &&& #[trigger] self.requests@[index].pages@[logical].index < self.page_limit
                        &&& logical_page.generation
                            == self.pages@[logical_page.index as int].generation
                        &&& self.pages@[logical_page.index as int].initialized_tokens
                            == if logical + 1 < self.requests@[index].page_count {
                                self.page_tokens
                            } else {
                                (self.requests@[index].resident_tokens as int
                                    - logical * self.page_tokens as int) as u32
                            }
                        &&& (match self.pages@[logical_page.index as int].state {
                            PageState::Writable { owner_slot } => owner_slot as int == index,
                            PageState::Sealed => (logical + 1) * self.page_tokens as int
                                <= self.requests@[index].committed_tokens,
                            PageState::Free => false,
                        })
                } by {
                    let logical_page = self.requests@[index].pages@[logical];
                    if logical_page.index == page.index {
                        assert(logical == position) by {
                            if logical < position {
                                assert(logical_pages_distinct(
                                    self.requests@[index],
                                    logical,
                                    position as int,
                                ));
                            } else if logical > position {
                                assert(logical_pages_distinct(
                                    self.requests@[index],
                                    position as int,
                                    logical,
                                ));
                            }
                        }
                        assert(self.pages@[page.index as int].state == PageState::Sealed);
                    }
                }
            } else {
                assert(self.requests@[index] == old(self).requests@[index]);
                assert forall |logical: int|
                    0 <= logical < self.requests@[index].page_count implies {
                        let logical_page = self.requests@[index].pages@[logical];
                        &&& #[trigger] self.requests@[index].pages@[logical].index < self.page_limit
                        &&& logical_page.generation
                            == self.pages@[logical_page.index as int].generation
                        &&& self.pages@[logical_page.index as int].initialized_tokens
                            == if logical + 1 < self.requests@[index].page_count {
                                self.page_tokens
                            } else {
                                (self.requests@[index].resident_tokens as int
                                    - logical * self.page_tokens as int) as u32
                            }
                        &&& (match self.pages@[logical_page.index as int].state {
                            PageState::Writable { owner_slot } => owner_slot as int == index,
                            PageState::Sealed => (logical + 1) * self.page_tokens as int
                                <= self.requests@[index].committed_tokens,
                            PageState::Free => false,
                        })
                } by {
                    let logical_page = self.requests@[index].pages@[logical];
                    if logical_page.index == page.index {
                        assert(old(self).pages@[page.index as int].state == PageState::Sealed);
                    }
                }
            }
        }
        assert forall |index: int| 0 <= index < self.page_limit implies
            #[trigger] self.page_slot_well_formed(index) by {
            assert(old(self).page_slot_well_formed(index));
            if index == page.index {
                assert(self.pages@[index].state == PageState::Sealed);
                assert(self.pages@[index].initialized_tokens == self.page_tokens);
                assert(has_reference(self.pages@[index].reference_mask, target_index as u32));
                assert(self.chain_has_page(target_index as int, index)) by {
                    assert(self.requests@[target_index as int].pages@[position as int] == page);
                }
                assert forall |slot: int| 0 <= slot < MAX_REQUEST_SLOTS implies
                    (has_reference(self.pages@[index].reference_mask, slot as u32)
                        <==> self.chain_has_page(slot, index)) by {
                    if slot == target_index {
                    } else {
                        assert(self.requests@[slot].pages == old(self).requests@[slot].pages);
                        assert(self.requests@[slot].page_count == old(self).requests@[slot].page_count);
                        assert(self.requests@[slot].live == old(self).requests@[slot].live);
                        assert(self.chain_has_page(slot, index)
                            == old(self).chain_has_page(slot, index));
                    }
                }
            } else {
                assert(self.pages@[index] == old(self).pages@[index]);
                assert forall |slot: int| 0 <= slot < MAX_REQUEST_SLOTS implies
                    (has_reference(self.pages@[index].reference_mask, slot as u32)
                        <==> self.chain_has_page(slot, index)) by {
                    if slot == target_index {
                        assert(self.requests@[slot].live == old(self).requests@[slot].live);
                        assert(self.requests@[slot].page_count
                            == old(self).requests@[slot].page_count + 1);
                        assert(self.chain_has_page(slot, index)
                            == old(self).chain_has_page(slot, index)) by {
                            if self.chain_has_page(slot, index) {
                                let logical = choose |logical: int|
                                    0 <= logical < self.requests@[slot].page_count
                                        && self.requests@[slot].pages@[logical].index == index;
                                if logical < old(self).requests@[slot].page_count {
                                    assert(self.requests@[slot].pages@[logical]
                                        == old(self).requests@[slot].pages@[logical]);
                                } else {
                                    assert(logical == position);
                                    assert(self.requests@[slot].pages@[logical].index == page.index);
                                    assert(index != page.index);
                                    assert(false);
                                }
                            }
                            if old(self).chain_has_page(slot, index) {
                                let logical = choose |logical: int|
                                    0 <= logical < old(self).requests@[slot].page_count
                                        && old(self).requests@[slot].pages@[logical].index == index;
                                assert(self.requests@[slot].pages@[logical]
                                    == old(self).requests@[slot].pages@[logical]);
                            }
                        }
                    } else {
                        assert(self.requests@[slot] == old(self).requests@[slot]);
                        assert(self.chain_has_page(slot, index)
                            == old(self).chain_has_page(slot, index));
                    }
                }
            }
        }
        assert(self.page_tokens == old(self).page_tokens);
        assert(self.max_context_tokens == old(self).max_context_tokens);
        assert(self.page_limit == old(self).page_limit);
        assert(self.pages@.len() == old(self).pages@.len());
        assert(self.free_stack == old(self).free_stack);
        assert(self.free_len == old(self).free_len);
        assert(self.free_bitmap == old(self).free_bitmap);
        assert(self.requests@.len() == old(self).requests@.len());
        assert forall |index: int| 0 <= index < self.page_limit implies
            (self.free_bitmap@[index] <==> self.free_stack_has_page(index)) by {
            assert(old(self).free_bitmap@[index]
                <==> old(self).free_stack_has_page(index));
        }
        assert(self.well_formed());
    }

    fn append_existing_page(
        &mut self,
        _request: RequestKey,
        request_index: usize,
        page: PageId,
        written: u32,
    )
        requires
            old(self).well_formed(),
            request_index < MAX_REQUEST_SLOTS,
            old(self).requests@[request_index as int].live,
            old(self).requests@[request_index as int].page_count > 0,
            page == old(self).requests@[request_index as int].pages@[
                old(self).requests@[request_index as int].page_count - 1
            ],
            page.index < old(self).page_limit,
            old(self).pages@[page.index as int].state
                == (PageState::Writable { owner_slot: request_index as u8 }),
            0 < written,
            old(self).pages@[page.index as int].initialized_tokens + written
                <= old(self).page_tokens,
            old(self).requests@[request_index as int].resident_tokens + written
                <= old(self).max_context_tokens,
        ensures
            final(self).well_formed(),
            final(self).requests@[request_index as int].resident_tokens
                == old(self).requests@[request_index as int].resident_tokens + written,
            final(self).requests@[request_index as int].committed_tokens
                == old(self).requests@[request_index as int].committed_tokens,
            final(self).requests@[request_index as int].generation
                == old(self).requests@[request_index as int].generation,
            final(self).requests@[request_index as int].live
                == old(self).requests@[request_index as int].live,
            final(self).requests@[request_index as int].page_count
                == old(self).requests@[request_index as int].page_count,
            final(self).requests@[request_index as int].pages
                == old(self).requests@[request_index as int].pages,
            final(self).request_frame_except(old(self), request_index as int),
            final(self).sealed_payload_frame(old(self)),
            final(self).exact_sealed_frame(old(self)),
            final(self).reachable_payload_frame_except(old(self), request_index as int),
            final(self).pages@[page.index as int].initialized_tokens
                == old(self).pages@[page.index as int].initialized_tokens + written,
            forall |page_index: int|
                0 <= page_index < old(self).page_limit && page_index != page.index ==>
                    final(self).pages@[page_index] == old(self).pages@[page_index],
            final(self).free_stack == old(self).free_stack,
            final(self).free_len == old(self).free_len,
            final(self).free_bitmap == old(self).free_bitmap,
            final(self).page_tokens == old(self).page_tokens,
            final(self).max_context_tokens == old(self).max_context_tokens,
            final(self).page_limit == old(self).page_limit,
    {
        reveal(KvPool::request_slot_well_formed);
        reveal(KvPool::page_slot_well_formed);
        reveal(KvPool::chain_has_page);
        reveal(KvPool::free_stack_has_page);
        let page_index = page.index as usize;
        self.pages[page_index].initialized_tokens += written;
        self.requests[request_index].resident_tokens += written;
        assert forall |index: int| 0 <= index < MAX_REQUEST_SLOTS implies
            #[trigger] self.request_slot_well_formed(index) by {
            assert(old(self).request_slot_well_formed(index));
            if index == request_index {
                assert(self.requests@[index].page_count == old(self).requests@[index].page_count);
                assert(self.requests@[index].pages == old(self).requests@[index].pages);
                assert(self.requests@[index].generation > 0);
                assert(self.requests@[index].committed_tokens
                    <= self.requests@[index].resident_tokens);
                assert(self.requests@[index].resident_tokens <= self.max_context_tokens);
                assert(self.requests@[index].page_count <= MAX_PAGES_PER_REQUEST);
                assert(self.requests@[index].live);
                assert(self.requests@[index].resident_tokens > 0);
                assert(self.requests@[index].page_count > 0);
                let count = self.requests@[index].page_count as int;
                let resident = self.requests@[index].resident_tokens as int;
                let page_tokens = self.page_tokens as int;
                let old_resident = old(self).requests@[index].resident_tokens as int;
                let old_initialized = old(self).pages@[page.index as int].initialized_tokens as int;
                assert((count - 1) * page_tokens < resident);
                assert(old_initialized == old_resident - (count - 1) * page_tokens);
                assert(resident == old_resident + written);
                assert(old_initialized + written <= page_tokens);
                assert(count * page_tokens == (count - 1) * page_tokens + page_tokens)
                    by (nonlinear_arith);
                assert(resident <= count * page_tokens);
                assert forall |position: int|
                    0 <= position < self.requests@[index].page_count implies {
                        let logical = self.requests@[index].pages@[position];
                        &&& #[trigger] self.requests@[index].pages@[position].index < self.page_limit
                        &&& logical.generation == self.pages@[logical.index as int].generation
                        &&& self.pages@[logical.index as int].initialized_tokens
                            == if position + 1 < self.requests@[index].page_count {
                                self.page_tokens
                            } else {
                                (self.requests@[index].resident_tokens as int
                                    - position * self.page_tokens as int) as u32
                            }
                        &&& (match self.pages@[logical.index as int].state {
                            PageState::Writable { owner_slot } => owner_slot as int == index,
                            PageState::Sealed => (position + 1) * self.page_tokens as int
                                <= self.requests@[index].committed_tokens,
                            PageState::Free => false,
                        })
                } by {
                    if position + 1 < self.requests@[index].page_count {
                        assert(logical_pages_distinct(
                            self.requests@[index],
                            position,
                            self.requests@[index].page_count as int - 1,
                        ));
                    }
                }
                assert forall |left: int, right: int|
                    0 <= left < right < self.requests@[index].page_count implies
                        #[trigger] logical_pages_distinct(
                            self.requests@[index],
                            left,
                            right,
                        ) by {
                    assert(old(self).request_slot_well_formed(index));
                    assert(logical_pages_distinct(old(self).requests@[index], left, right));
                    assert(logical_page(self.requests@[index], left)
                        == logical_page(old(self).requests@[index], left));
                    assert(logical_page(self.requests@[index], right)
                        == logical_page(old(self).requests@[index], right));
                }
            } else {
                assert(self.requests@[index] == old(self).requests@[index]);
            }
        }
        assert forall |index: int| 0 <= index < self.page_limit implies
            #[trigger] self.page_slot_well_formed(index) by {
            assert(old(self).page_slot_well_formed(index));
            assert forall |slot: int| 0 <= slot < MAX_REQUEST_SLOTS implies
                (has_reference(self.pages@[index].reference_mask, slot as u32)
                    <==> self.chain_has_page(slot, index)) by {
                assert(self.requests@[slot].pages == old(self).requests@[slot].pages);
                assert(self.requests@[slot].page_count == old(self).requests@[slot].page_count);
                assert(self.requests@[slot].live == old(self).requests@[slot].live);
                assert(self.chain_has_page(slot, index)
                    == old(self).chain_has_page(slot, index));
            }
        }
        assert(self.page_tokens == old(self).page_tokens);
        assert(self.max_context_tokens == old(self).max_context_tokens);
        assert(self.page_limit == old(self).page_limit);
        assert(self.pages@.len() == old(self).pages@.len());
        assert(self.free_stack == old(self).free_stack);
        assert(self.free_len == old(self).free_len);
        assert(self.free_bitmap == old(self).free_bitmap);
        assert(self.requests@.len() == old(self).requests@.len());
        assert forall |index: int| 0 <= index < self.page_limit implies
            (self.free_bitmap@[index] <==> self.free_stack_has_page(index)) by {
            assert(old(self).free_bitmap@[index]
                <==> old(self).free_stack_has_page(index));
        }
        assert(self.well_formed());
    }

    fn append_fresh_page(&mut self, request: RequestKey, request_index: usize, written: u32)
        requires
            old(self).well_formed(),
            request.slot < MAX_REQUEST_SLOTS,
            request_index == request.slot,
            old(self).requests@[request_index as int].live,
            old(self).requests@[request_index as int].generation == request.generation,
            old(self).requests@[request_index as int].page_count < MAX_PAGES_PER_REQUEST,
            old(self).free_len > 0,
            0 < written <= old(self).page_tokens,
            old(self).requests@[request_index as int].resident_tokens + written
                <= old(self).max_context_tokens,
            old(self).requests@[request_index as int].resident_tokens as int
                == old(self).requests@[request_index as int].page_count as int
                    * old(self).page_tokens as int,
        ensures
            final(self).well_formed(),
            final(self).requests@[request_index as int].resident_tokens
                == old(self).requests@[request_index as int].resident_tokens + written,
            final(self).requests@[request_index as int].committed_tokens
                == old(self).requests@[request_index as int].committed_tokens,
            final(self).requests@[request_index as int].generation
                == old(self).requests@[request_index as int].generation,
            final(self).requests@[request_index as int].live
                == old(self).requests@[request_index as int].live,
            final(self).requests@[request_index as int].page_count
                == old(self).requests@[request_index as int].page_count + 1,
            final(self).request_frame_except(old(self), request_index as int),
            final(self).sealed_payload_frame(old(self)),
            final(self).exact_sealed_frame(old(self)),
            final(self).reachable_payload_frame_except(old(self), request_index as int),
            final(self).free_len + 1 == old(self).free_len,
            final(self).free_stack == old(self).free_stack,
            final(self).page_tokens == old(self).page_tokens,
            final(self).max_context_tokens == old(self).max_context_tokens,
            final(self).page_limit == old(self).page_limit,
    {
        reveal(KvPool::request_slot_well_formed);
        reveal(KvPool::page_slot_well_formed);
        reveal(KvPool::chain_has_page);
        reveal(KvPool::free_stack_has_page);
        let stack_index = (self.free_len - 1) as usize;
        let page_index = self.free_stack[stack_index] as usize;
        let chain_position = self.requests[request_index].page_count as usize;

        assert(stack_index as int == old(self).free_len - 1);
        assert(old(self).free_stack@[old(self).free_len - 1] == page_index);
        assert(old(self).free_stack_has_page(page_index as int));
        assert(old(self).free_bitmap@[page_index as int]);
        assert(old(self).page_slot_well_formed(page_index as int));
        assert(old(self).pages@[page_index as int].state == PageState::Free);
        assert(old(self).pages@[page_index as int].reference_mask == 0);
        assert forall |slot: int| 0 <= slot < MAX_REQUEST_SLOTS implies
            !old(self).chain_has_page(slot, page_index as int) by {
            zero_reference_lemma(slot as u32);
            assert(!has_reference(0, slot as u32));
        }

        self.free_len -= 1;
        self.free_bitmap[page_index] = false;
        self.pages[page_index].state = PageState::Writable {
            owner_slot: request.slot as u8,
        };
        self.pages[page_index].initialized_tokens = written;
        self.pages[page_index].reference_mask =
            set_reference(self.pages[page_index].reference_mask, request.slot);
        let page = PageId { index: page_index as u32, generation: self.pages[page_index].generation };
        self.requests[request_index].pages[chain_position] = page;
        self.requests[request_index].page_count += 1;
        self.requests[request_index].resident_tokens += written;

        assert forall |index: int| 0 <= index < MAX_REQUEST_SLOTS implies
            #[trigger] self.request_slot_well_formed(index) by {
            assert(old(self).request_slot_well_formed(index));
            if index == request_index {
                let old_count = old(self).requests@[index].page_count as int;
                let new_count = self.requests@[index].page_count as int;
                let new_resident = self.requests@[index].resident_tokens as int;
                let page_tokens = self.page_tokens as int;
                let old_resident = old(self).requests@[index].resident_tokens as int;
                let written_int = written as int;
                assert(new_count == old_count + 1);
                assert(new_resident == old_resident + written_int);
                assert(old_resident == old_count * page_tokens);
                assert(0 < written_int <= page_tokens);
                assert(new_count - 1 == old_count);
                assert((new_count - 1) * page_tokens == old_count * page_tokens);
                assert(new_count * page_tokens == (old_count + 1) * page_tokens);
                assert((old_count + 1) * page_tokens
                    == old_count * page_tokens + page_tokens) by (nonlinear_arith);
                assert((new_count - 1) * page_tokens < new_resident);
                assert(new_resident <= new_count * page_tokens);
                assert forall |position: int| 0 <= position < new_count implies {
                    let logical = self.requests@[index].pages@[position];
                    &&& #[trigger] self.requests@[index].pages@[position].index < self.page_limit
                    &&& logical.generation == self.pages@[logical.index as int].generation
                    &&& self.pages@[logical.index as int].initialized_tokens
                        == if position + 1 < new_count {
                            self.page_tokens
                        } else {
                            (new_resident - position * page_tokens) as u32
                        }
                    &&& (match self.pages@[logical.index as int].state {
                        PageState::Writable { owner_slot } => owner_slot as int == index,
                        PageState::Sealed => (position + 1) * page_tokens
                            <= self.requests@[index].committed_tokens,
                        PageState::Free => false,
                    })
                } by {
                    if position < old_count {
                        assert(self.requests@[index].pages@[position]
                            == old(self).requests@[index].pages@[position]);
                        assert(old(self).requests@[index].pages@[position].index != page_index);
                        let old_logical = old(self).requests@[index].pages@[position];
                        assert(self.pages@[old_logical.index as int]
                            == old(self).pages@[old_logical.index as int]);
                        if position + 1 == old_count {
                            assert(position == old_count - 1);
                            assert(old(self).pages@[old_logical.index as int].initialized_tokens
                                == (old_resident - position * page_tokens) as u32);
                            assert(position * page_tokens == (old_count - 1) * page_tokens);
                            assert(old_count * page_tokens
                                - (old_count - 1) * page_tokens == page_tokens)
                                by (nonlinear_arith);
                            assert(old_resident - position * page_tokens == page_tokens)
                                by {
                                    assert(old_resident == old_count * page_tokens);
                                };
                        }
                    } else {
                        assert(position == old_count);
                        assert(self.requests@[index].pages@[position].index == page_index);
                        assert(self.pages@[page_index as int].initialized_tokens == written);
                        assert(position == old_count);
                        assert(position * page_tokens == old_count * page_tokens);
                        assert(new_resident - position * page_tokens == written_int)
                            by {
                                assert(new_resident == old_resident + written_int);
                                assert(old_resident == old_count * page_tokens);
                            };
                    }
                }
                assert forall |left: int, right: int|
                    0 <= left < right < new_count implies
                        #[trigger] logical_pages_distinct(
                            self.requests@[index],
                            left,
                            right,
                        ) by {
                    if right < old_count {
                        assert(logical_pages_distinct(old(self).requests@[index], left, right));
                    } else {
                        assert(right == old_count);
                        assert(!old(self).chain_has_page(index, page_index as int));
                    }
                }
            } else {
                assert(self.requests@[index] == old(self).requests@[index]);
            }
        }
        assert forall |slot: int| 0 <= slot < MAX_REQUEST_SLOTS implies
            (#[trigger] has_reference(self.pages@[page_index as int].reference_mask, slot as u32)
                <==> slot == request_index) by {
            zero_reference_lemma(slot as u32);
            set_reference_lemma(0, request.slot, slot as u32);
        }
        assert forall |slot: int| 0 <= slot < MAX_REQUEST_SLOTS implies
            (#[trigger] self.chain_has_page(slot, page_index as int)
                <==> slot == request_index) by {
            if slot == request_index {
                let witness = old(self).requests@[slot].page_count as int;
                assert(0 <= witness < self.requests@[slot].page_count);
                assert(self.requests@[slot].pages@[witness].index == page_index);
            } else {
                assert(self.requests@[slot] == old(self).requests@[slot]);
                assert(!old(self).chain_has_page(slot, page_index as int));
            }
        }
        assert forall |index: int| 0 <= index < self.page_limit implies
            #[trigger] self.page_slot_well_formed(index) by {
            assert(old(self).page_slot_well_formed(index));
            assert forall |slot: int| 0 <= slot < MAX_REQUEST_SLOTS implies
                (has_reference(self.pages@[index].reference_mask, slot as u32)
                    <==> self.chain_has_page(slot, index)) by {
                if index == page_index {
                    assert(has_reference(self.pages@[index].reference_mask, slot as u32)
                        <==> slot == request_index);
                    assert(self.chain_has_page(slot, index) <==> slot == request_index);
                } else {
                    assert(self.pages@[index] == old(self).pages@[index]);
                    if slot == request_index {
                        assert(self.requests@[slot].page_count
                            == old(self).requests@[slot].page_count + 1);
                        assert(self.requests@[slot].pages@[
                            old(self).requests@[slot].page_count as int
                        ]
                            .index == page_index);
                        assert forall |position: int|
                            0 <= position < old(self).requests@[slot].page_count implies
                                self.requests@[slot].pages@[position]
                                    == old(self).requests@[slot].pages@[position] by {
                        }
                        if self.chain_has_page(slot, index) {
                            let position = choose |position: int|
                                0 <= position < self.requests@[slot].page_count
                                    && self.requests@[slot].pages@[position].index == index;
                            if position == old(self).requests@[slot].page_count {
                                assert(index == page_index);
                            } else {
                                assert(position < old(self).requests@[slot].page_count);
                                assert(old(self).chain_has_page(slot, index));
                            }
                        }
                        if old(self).chain_has_page(slot, index) {
                            let position = choose |position: int|
                                0 <= position < old(self).requests@[slot].page_count
                                    && old(self).requests@[slot].pages@[position].index == index;
                            assert(self.requests@[slot].pages@[position].index == index);
                            assert(self.chain_has_page(slot, index));
                        }
                    } else {
                        assert(self.requests@[slot] == old(self).requests@[slot]);
                        assert(self.chain_has_page(slot, index)
                            == old(self).chain_has_page(slot, index));
                    }
                }
            }
        }
        assert forall |index: int| 0 <= index < self.page_limit implies
            (self.free_bitmap@[index] <==> self.free_stack_has_page(index)) by {
            assert(old(self).free_bitmap@[index]
                <==> old(self).free_stack_has_page(index));
            if index == page_index {
                assert forall |position: int| 0 <= position < self.free_len implies
                    self.free_stack@[position] != index by {
                    assert(free_positions_distinct(
                        old(self),
                        position,
                        old(self).free_len as int - 1,
                    ));
                }
            } else if old(self).free_stack_has_page(index) {
                let position = choose |position: int|
                    0 <= position < old(self).free_len
                        && old(self).free_stack@[position] == index;
                assert(position != old(self).free_len - 1);
                assert(0 <= position < self.free_len);
                assert(self.free_stack@[position] == index);
                assert(self.free_stack_has_page(index));
            }
        }
        assert(self.well_formed());
    }

    fn raise_committed(&mut self, request_index: usize, committed: u32)
        requires
            old(self).well_formed(),
            request_index < MAX_REQUEST_SLOTS,
            old(self).requests@[request_index as int].live,
            old(self).requests@[request_index as int].committed_tokens <= committed,
            committed <= old(self).requests@[request_index as int].resident_tokens,
        ensures
            final(self).well_formed(),
            final(self).requests@[request_index as int].committed_tokens == committed,
            final(self).requests@[request_index as int].resident_tokens
                == old(self).requests@[request_index as int].resident_tokens,
            final(self).requests@[request_index as int].generation
                == old(self).requests@[request_index as int].generation,
            final(self).requests@[request_index as int].live
                == old(self).requests@[request_index as int].live,
            final(self).requests@[request_index as int].page_count
                == old(self).requests@[request_index as int].page_count,
            final(self).requests@[request_index as int].pages
                == old(self).requests@[request_index as int].pages,
            final(self).request_frame_except(old(self), request_index as int),
            final(self).pages == old(self).pages,
            final(self).free_stack == old(self).free_stack,
            final(self).free_len == old(self).free_len,
            final(self).free_bitmap == old(self).free_bitmap,
            final(self).page_tokens == old(self).page_tokens,
            final(self).max_context_tokens == old(self).max_context_tokens,
            final(self).page_limit == old(self).page_limit,
    {
        reveal(KvPool::request_slot_well_formed);
        reveal(KvPool::page_slot_well_formed);
        self.requests[request_index].committed_tokens = committed;
        assert forall |index: int| 0 <= index < MAX_REQUEST_SLOTS implies
            #[trigger] self.request_slot_well_formed(index) by {
            assert(old(self).request_slot_well_formed(index));
            if index == request_index {
                assert(self.requests@[index].generation == old(self).requests@[index].generation);
                assert(self.requests@[index].live == old(self).requests@[index].live);
                assert(self.requests@[index].resident_tokens
                    == old(self).requests@[index].resident_tokens);
                assert(self.requests@[index].page_count
                    == old(self).requests@[index].page_count);
                assert(self.requests@[index].pages == old(self).requests@[index].pages);
                assert(self.requests@[index].generation > 0);
                assert(self.requests@[index].committed_tokens
                    <= self.requests@[index].resident_tokens);
                assert(self.requests@[index].resident_tokens <= self.max_context_tokens);
                assert(self.requests@[index].page_count <= MAX_PAGES_PER_REQUEST);
                assert(self.requests@[index].live);
                assert forall |position: int|
                    0 <= position < self.requests@[index].page_count implies {
                        let page = self.requests@[index].pages@[position];
                        &&& #[trigger] self.requests@[index].pages@[position].index < self.page_limit
                        &&& page.generation == self.pages@[page.index as int].generation
                        &&& self.pages@[page.index as int].initialized_tokens
                            == if position + 1 < self.requests@[index].page_count {
                                self.page_tokens
                            } else {
                                (self.requests@[index].resident_tokens as int
                                    - position * self.page_tokens as int) as u32
                            }
                        &&& (match self.pages@[page.index as int].state {
                            PageState::Writable { owner_slot } => owner_slot as int == index,
                            PageState::Sealed => (position + 1) * self.page_tokens as int
                                <= self.requests@[index].committed_tokens,
                            PageState::Free => false,
                        })
                } by {
                    let page = self.requests@[index].pages@[position];
                    assert(self.pages@[page.index as int] == old(self).pages@[page.index as int]);
                    if self.pages@[page.index as int].state == PageState::Sealed {
                        assert((position + 1) * self.page_tokens as int
                            <= old(self).requests@[index].committed_tokens);
                        assert(old(self).requests@[index].committed_tokens
                            <= self.requests@[index].committed_tokens);
                    }
                }
                assert forall |left: int, right: int|
                    0 <= left < right < self.requests@[index].page_count implies
                        #[trigger] logical_pages_distinct(
                            self.requests@[index],
                            left,
                            right,
                        ) by {
                    assert(logical_pages_distinct(old(self).requests@[index], left, right));
                }
            } else {
                assert(self.requests@[index] == old(self).requests@[index]);
            }
        }
        assert forall |index: int| 0 <= index < self.page_limit implies
            #[trigger] self.page_slot_well_formed(index) by {
            assert(old(self).page_slot_well_formed(index));
            assert(self.pages@[index] == old(self).pages@[index]);
            assert forall |slot: int| 0 <= slot < MAX_REQUEST_SLOTS implies
                (has_reference(self.pages@[index].reference_mask, slot as u32)
                    <==> self.chain_has_page(slot, index)) by {
                assert(self.requests@[slot].pages == old(self).requests@[slot].pages);
                assert(self.requests@[slot].page_count == old(self).requests@[slot].page_count);
                assert(self.requests@[slot].live == old(self).requests@[slot].live);
                assert(self.chain_has_page(slot, index)
                    == old(self).chain_has_page(slot, index));
            }
            match self.pages@[index].state {
                PageState::Writable { owner_slot } => {
                    assert(self.requests@[owner_slot as int].live
                        == old(self).requests@[owner_slot as int].live);
                }
                PageState::Sealed | PageState::Free => {}
            }
        }
        assert(self.free_stack == old(self).free_stack);
        assert(self.free_bitmap == old(self).free_bitmap);
        assert(self.free_len == old(self).free_len);
        assert(self.pages == old(self).pages);
        assert(self.page_tokens == old(self).page_tokens);
        assert(self.max_context_tokens == old(self).max_context_tokens);
        assert(self.page_limit == old(self).page_limit);
        assert(self.requests@.len() == old(self).requests@.len());
        assert forall |index: int| 0 <= index < self.page_limit implies
            (self.free_bitmap@[index] <==> self.free_stack_has_page(index)) by {
            assert(old(self).free_bitmap@[index]
                <==> old(self).free_stack_has_page(index));
        }
        assert(self.well_formed());
    }

    fn drop_exclusive_tail(&mut self, request_index: usize, page: PageId)
        requires
            old(self).well_formed(),
            request_index < MAX_REQUEST_SLOTS,
            old(self).requests@[request_index as int].live,
            old(self).requests@[request_index as int].page_count > 0,
            old(self).requests@[request_index as int].page_count <= MAX_PAGES_PER_REQUEST,
            page == old(self).requests@[request_index as int].pages@[
                old(self).requests@[request_index as int].page_count - 1
            ],
            page.index < old(self).page_limit,
            old(self).pages@[page.index as int].state
                == (PageState::Writable { owner_slot: request_index as u8 }),
            old(self).pages@[page.index as int].generation < u32::MAX,
            old(self).requests@[request_index as int].committed_tokens as int
                <= old(self).requests@[request_index as int].resident_tokens as int
                    - old(self).pages@[page.index as int].initialized_tokens as int,
            old(self).free_len < old(self).page_limit,
        ensures
            final(self).well_formed(),
            final(self).requests@[request_index as int].page_count + 1
                == old(self).requests@[request_index as int].page_count,
            final(self).requests@[request_index as int].committed_tokens
                == old(self).requests@[request_index as int].committed_tokens,
            final(self).requests@[request_index as int].generation
                == old(self).requests@[request_index as int].generation,
            final(self).requests@[request_index as int].live
                == old(self).requests@[request_index as int].live,
            final(self).request_frame_except(old(self), request_index as int),
            final(self).free_len == old(self).free_len + 1,
            final(self).page_tokens == old(self).page_tokens,
            final(self).max_context_tokens == old(self).max_context_tokens,
            final(self).page_limit == old(self).page_limit,
    {
        reveal(KvPool::request_slot_well_formed);
        reveal(KvPool::page_slot_well_formed);
        reveal(KvPool::chain_has_page);
        reveal(KvPool::free_stack_has_page);
        let old_count = self.requests[request_index].page_count;
        let position = old_count - 1;
        assert(position < MAX_PAGES_PER_REQUEST);
        assert(self.requests@[request_index as int].pages@[position as int] == page);
        let page_index = page.index as usize;
        let initialized = self.pages[page_index].initialized_tokens;
        let generation = self.pages[page_index].generation + 1;
        let new_resident = self.requests[request_index].resident_tokens - initialized;
        let stack_position = self.free_len as usize;

        self.requests[request_index].pages[position as usize] = PageId::EMPTY;
        self.requests[request_index].page_count = position;
        self.requests[request_index].resident_tokens = new_resident;
        self.pages[page_index] = PageSlot {
            generation,
            state: PageState::Free,
            initialized_tokens: 0,
            reference_mask: 0,
        };
        self.free_bitmap[page_index] = true;
        self.free_stack[stack_position] = page.index;
        self.free_len += 1;

        assert forall |index: int| 0 <= index < MAX_REQUEST_SLOTS implies
            #[trigger] self.request_slot_well_formed(index) by {
            assert(old(self).request_slot_well_formed(index));
            if index == request_index {
                assert(self.requests@[index].resident_tokens as int
                    == self.requests@[index].page_count as int * self.page_tokens as int);
                assert forall |logical: int|
                    0 <= logical < self.requests@[index].page_count implies {
                        let logical_page = self.requests@[index].pages@[logical];
                        &&& #[trigger] self.requests@[index].pages@[logical].index < self.page_limit
                        &&& logical_page.generation
                            == self.pages@[logical_page.index as int].generation
                        &&& self.pages@[logical_page.index as int].initialized_tokens
                            == if logical + 1 < self.requests@[index].page_count {
                                self.page_tokens
                            } else {
                                (self.requests@[index].resident_tokens as int
                                    - logical * self.page_tokens as int) as u32
                            }
                        &&& (match self.pages@[logical_page.index as int].state {
                            PageState::Writable { owner_slot } => owner_slot as int == index,
                            PageState::Sealed => (logical + 1) * self.page_tokens as int
                                <= self.requests@[index].committed_tokens,
                            PageState::Free => false,
                        })
                } by {
                    assert(logical < position);
                    assert(self.requests@[index].pages@[logical]
                        == old(self).requests@[index].pages@[logical]);
                    assert(logical_pages_distinct(
                        old(self).requests@[index],
                        logical,
                        position as int,
                    ));
                }
                assert forall |left: int, right: int|
                    0 <= left < right < self.requests@[index].page_count implies
                        #[trigger] logical_pages_distinct(
                            self.requests@[index],
                            left,
                            right,
                        ) by {
                    assert(logical_pages_distinct(old(self).requests@[index], left, right));
                }
            } else {
                assert(self.requests@[index] == old(self).requests@[index]);
            }
        }
        assert forall |index: int| 0 <= index < self.page_limit implies
            #[trigger] self.page_slot_well_formed(index) by {
            assert(old(self).page_slot_well_formed(index));
            if index == page_index {
                assert forall |slot: int| 0 <= slot < MAX_REQUEST_SLOTS implies
                    !self.chain_has_page(slot, index) by {
                    if slot == request_index {
                        if self.chain_has_page(slot, index) {
                            let logical = choose |logical: int|
                                0 <= logical < self.requests@[slot].page_count
                                    && self.requests@[slot].pages@[logical].index == index;
                            assert(logical < position);
                            assert(self.requests@[slot].pages@[logical]
                                == old(self).requests@[slot].pages@[logical]);
                            assert(logical_pages_distinct(
                                old(self).requests@[slot],
                                logical,
                                position as int,
                            ));
                        }
                    } else {
                        assert(self.requests@[slot] == old(self).requests@[slot]);
                        if self.chain_has_page(slot, index) {
                            assert(old(self).chain_has_page(slot, index));
                            assert(has_reference(
                                old(self).pages@[index].reference_mask,
                                slot as u32,
                            ));
                            assert(old(self).pages@[index].state
                                == (PageState::Writable { owner_slot: request_index as u8 }));
                            assert(old(self).request_slot_well_formed(slot));
                            let logical = choose |logical: int|
                                0 <= logical < old(self).requests@[slot].page_count
                                    && old(self).requests@[slot].pages@[logical].index == index;
                            assert(old(self).pages@[index].state
                                == (PageState::Writable { owner_slot: slot as u8 }));
                        }
                    }
                }
                assert forall |slot: int| 0 <= slot < MAX_REQUEST_SLOTS implies
                    (has_reference(self.pages@[index].reference_mask, slot as u32)
                        <==> self.chain_has_page(slot, index)) by {
                    zero_reference_lemma(slot as u32);
                }
            } else {
                assert(self.pages@[index] == old(self).pages@[index]);
                assert forall |slot: int| 0 <= slot < MAX_REQUEST_SLOTS implies
                    (has_reference(self.pages@[index].reference_mask, slot as u32)
                        <==> self.chain_has_page(slot, index)) by {
                    if slot == request_index {
                        assert(self.chain_has_page(slot, index)
                            == old(self).chain_has_page(slot, index)) by {
                            if old(self).chain_has_page(slot, index) {
                                let logical = choose |logical: int|
                                    0 <= logical < old(self).requests@[slot].page_count
                                        && old(self).requests@[slot].pages@[logical].index == index;
                                assert(logical != position);
                                assert(logical < position);
                            }
                            if self.chain_has_page(slot, index) {
                                let logical = choose |logical: int|
                                    0 <= logical < self.requests@[slot].page_count
                                        && self.requests@[slot].pages@[logical].index == index;
                                assert(logical < position);
                            }
                        }
                    } else {
                        assert(self.requests@[slot] == old(self).requests@[slot]);
                        assert(self.chain_has_page(slot, index)
                            == old(self).chain_has_page(slot, index));
                    }
                }
            }
        }
        assert forall |index: int| 0 <= index < self.page_limit implies
            (self.free_bitmap@[index] <==> self.free_stack_has_page(index)) by {
            if index == page_index {
                assert(self.free_stack@[old(self).free_len as int] == index);
            } else {
                assert(self.free_bitmap@[index] == old(self).free_bitmap@[index]);
                assert(self.free_stack_has_page(index)
                    == old(self).free_stack_has_page(index)) by {
                    if self.free_stack_has_page(index) {
                        let free_position = choose |free_position: int|
                            0 <= free_position < self.free_len
                                && self.free_stack@[free_position] == index;
                        if free_position == old(self).free_len {
                            assert(index == page_index);
                        }
                    }
                }
            }
        }
        assert forall |left: int, right: int|
            0 <= left < right < self.free_len implies
                #[trigger] free_positions_distinct(self, left, right) by {
            if right == old(self).free_len {
                assert(self.free_stack@[right] == page_index);
                assert(!old(self).free_stack_has_page(page_index as int));
            } else {
                assert(right < old(self).free_len);
                assert(free_positions_distinct(old(self), left, right));
            }
        }
        assert(self.well_formed());
    }

    fn reclaim_page_unchecked(&mut self, page_index: usize) {
        let generation = self.pages[page_index].generation + 1;
        self.pages[page_index] = PageSlot {
            generation,
            state: PageState::Free,
            initialized_tokens: 0,
            reference_mask: 0,
        };
        self.free_bitmap[page_index] = true;
        self.free_stack[self.free_len as usize] = page_index as u32;
        self.free_len += 1;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Capacity { Pages, PageTokens, ContextTokens, RequestPages }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Invariant { FreeStack, PageState, ReferenceCount, TentativePage }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KvError {
    ZeroCapacity(Capacity),
    CapacityExceedsBuildBound(Capacity),
    PageExceedsContext,
    InvalidRequestSlot(u32),
    RequestSlotOccupied(u32),
    UnknownRequest(u32),
    StaleRequestGeneration { slot: u32, expected: u32, actual: u32 },
    RequestGenerationExhausted(u32),
    ContextExceeded,
    CommitExceedsResident,
    RequestPageTableFull,
    OutOfPages,
    InvalidPage(PageId),
    StalePage(PageId),
    GenerationExhausted(PageId),
    ReferenceCountExhausted(PageId),
    SameRequestShare,
    PrefixNotPageAligned,
    PrefixExceedsCommitted,
    PrefixPageIncomplete(PageId),
    TargetNotEmpty,
    ReadOutOfBounds,
    ReadUninitialized(PageId),
    InvalidQuiescencePermit,
    InvariantViolation(Invariant),
}

fn request_key(request: RequestId) -> (key: RequestKey)
    ensures
        key.slot == request.slot_spec(),
        key.generation == request.generation_spec(),
{
    RequestKey {
        slot: request.slot(),
        generation: request.generation(),
    }
}

impl KvPool {
    /// Creates an empty pool within build-generated bounds.
    pub fn new(
        page_count: u32,
        page_tokens: u32,
        max_context_tokens: u32,
    ) -> (result: Result<Self, KvError>)
        ensures
            match result {
                Ok(pool) => Self::new_enabled(page_count, page_tokens, max_context_tokens)
                    && pool.well_formed()
                    && forall |slot: int| 0 <= slot < MAX_REQUEST_SLOTS ==> {
                        &&& !pool.request_live_by_slot_spec(slot)
                        &&& pool.request_generation_by_slot_spec(slot) == 1
                    },
                Err(_) => !Self::new_enabled(page_count, page_tokens, max_context_tokens),
            },
    {
        Self::new_bounded(page_count, page_tokens, max_context_tokens)
    }

    /// Activates the exact generation expected by a scheduler request slot.
    pub fn create_request(&mut self, request: RequestId) -> (result: Result<(), KvError>)
        requires old(self).well_formed(),
        ensures
            final(self).well_formed(),
            final(self).page_tokens_spec() == old(self).page_tokens_spec(),
            final(self).max_context_tokens_spec() == old(self).max_context_tokens_spec(),
            final(self).page_limit_spec() == old(self).page_limit_spec(),
            final(self).create_identity_refines(old(self), request, &result),
            match result {
                Ok(()) => {
                    &&& old(self).create_enabled(request)
                    &&& final(self).request_matches_spec(request)
                    &&& final(self).request_frame_except(
                        old(self),
                        request.slot_spec() as int,
                    )
                }
                Err(_) => {
                    &&& !old(self).create_enabled(request)
                    &&& final(self).same_state(old(self))
                }
            },
    {
        reveal(KvPool::create_enabled);
        reveal(KvPool::request_matches_spec);
        self.create_request_key(request_key(request))
    }

    /// Materializes tentative logical KV positions.
    pub fn append_tentative(
        &mut self,
        request: RequestId,
        token_count: u32,
    ) -> (result: Result<(), KvError>)
        requires old(self).well_formed(),
        ensures
            final(self).well_formed(),
            final(self).page_tokens_spec() == old(self).page_tokens_spec(),
            final(self).max_context_tokens_spec() == old(self).max_context_tokens_spec(),
            final(self).page_limit_spec() == old(self).page_limit_spec(),
            final(self).identity_frame(old(self)),
            match result {
                Ok(()) => {
                    &&& old(self).append_enabled(request, token_count)
                    &&& final(self).resident_tokens_spec(request).is_some()
                    &&& final(self).resident_tokens_spec(request).unwrap() as int
                        == old(self).resident_tokens_spec(request).unwrap() as int
                            + token_count as int
                    &&& final(self).committed_tokens_spec(request)
                        == old(self).committed_tokens_spec(request)
                    &&& final(self).request_frame_except(
                        old(self),
                        request.slot_spec() as int,
                    )
                    &&& final(self).exact_sealed_frame(old(self))
                    &&& final(self).reachable_payload_frame_except(
                        old(self),
                        request.slot_spec() as int,
                    )
                }
                Err(_) => {
                    &&& !old(self).append_enabled(request, token_count)
                    &&& final(self).same_state(old(self))
                }
            },
    {
        reveal(KvPool::append_enabled);
        reveal(KvPool::request_matches_spec);
        reveal(KvPool::resident_tokens_spec);
        reveal(KvPool::committed_tokens_spec);
        reveal(KvPool::key_matches);
        let ghost previous = *self;
        let key = request_key(request);
        match self.append_tentative_key(key, token_count) {
            Ok(()) => {
                assert(previous.append_key_enabled(key, token_count));
                reveal(KvPool::append_key_enabled);
                assert(previous.key_matches(key));
                assert(self.key_matches(key));
                assert(previous.request_matches_spec(request));
                assert(self.request_matches_spec(request));
                Ok(())
            }
            Err(error) => {
                assert(!previous.append_key_enabled(key, token_count));
                Err(error)
            }
        }
    }

    /// Shares only complete, committed, page-aligned prefix pages.
    pub fn share_committed_prefix(
        &mut self,
        source: RequestId,
        target: RequestId,
        token_count: u32,
    ) -> (result: Result<(), KvError>)
        requires old(self).well_formed(),
        ensures
            final(self).well_formed(),
            final(self).page_tokens_spec() == old(self).page_tokens_spec(),
            final(self).max_context_tokens_spec() == old(self).max_context_tokens_spec(),
            final(self).page_limit_spec() == old(self).page_limit_spec(),
            final(self).identity_frame(old(self)),
            match result {
                Ok(()) => {
                    &&& old(self).share_enabled(source, target, token_count)
                    &&& final(self).resident_tokens_spec(target) == Some(token_count)
                    &&& final(self).committed_tokens_spec(target) == Some(token_count)
                    &&& final(self).resident_tokens_spec(source)
                        == old(self).resident_tokens_spec(source)
                    &&& final(self).committed_tokens_spec(source)
                        == old(self).committed_tokens_spec(source)
                    &&& final(self).request_frame_except_two(
                        old(self),
                        source.slot_spec() as int,
                        target.slot_spec() as int,
                    )
                    &&& final(self).sealed_payload_frame(old(self))
                    &&& final(self).share_page_frame(
                        old(self),
                        source.slot_spec() as int,
                        target.slot_spec(),
                        token_count as int / old(self).page_tokens_spec() as int,
                    )
                }
                Err(_) => {
                    &&& !old(self).share_enabled(source, target, token_count)
                    &&& final(self).same_state(old(self))
                }
            },
    {
        reveal(KvPool::share_enabled);
        reveal(KvPool::request_matches_spec);
        reveal(KvPool::resident_tokens_spec);
        reveal(KvPool::committed_tokens_spec);
        reveal(KvPool::key_matches);
        let ghost previous = *self;
        let source_key = request_key(source);
        let target_key = request_key(target);
        match self.share_committed_prefix_key(source_key, target_key, token_count) {
            Ok(()) => {
                assert(previous.share_key_enabled(source_key, target_key, token_count));
                reveal(KvPool::share_key_enabled);
                assert(previous.key_matches(source_key));
                assert(previous.key_matches(target_key));
                assert(self.key_matches(source_key));
                assert(self.key_matches(target_key));
                assert(previous.request_matches_spec(source));
                assert(previous.request_matches_spec(target));
                assert(self.request_matches_spec(source));
                assert(self.request_matches_spec(target));
                Ok(())
            }
            Err(error) => {
                assert(!previous.share_key_enabled(source_key, target_key, token_count));
                Err(error)
            }
        }
    }

    /// Atomically commits the accepted tentative prefix and drops its suffix.
    pub(crate) fn finalize_tentative(
        &mut self,
        request: RequestId,
        accepted_tokens: u32,
        permit: KvQuiescencePermit,
    ) -> (result: Result<KvFinalizedRequest, KvAuthorityError>)
        requires old(self).well_formed(),
        ensures
            final(self).well_formed(),
            final(self).identity_frame(old(self)),
            match result {
                Ok(evidence) => {
                    &&& evidence.request_spec() == request
                    &&& evidence.origin_spec() == permit.origin_spec()
                    &&& permit.request_spec() == request
                }
                Err(failure) => {
                    &&& final(self).same_state(old(self))
                    &&& failure.permit_request_spec() == permit.request_spec()
                    &&& failure.permit_origin_spec() == permit.origin_spec()
                }
            },
    {
        let origin = permit.origin();
        if permit.request() != request || origin == KvQuiescenceOrigin::NeverSubmitted {
            return Err(KvAuthorityError {
                error: KvError::InvalidQuiescencePermit,
                permit,
            });
        }
        match self.finalize_tentative_key(request_key(request), accepted_tokens) {
            Ok(()) => Ok(KvFinalizedRequest { request, origin }),
            Err(error) => Err(KvAuthorityError { error, permit }),
        }
    }

    /// Releases a request reference set only after device quiescence.
    pub(crate) fn release_request(
        &mut self,
        request: RequestId,
        permit: KvQuiescencePermit,
    ) -> (result: Result<KvDetachedRequest, KvAuthorityError>)
        requires old(self).well_formed(),
        ensures
            final(self).well_formed(),
            final(self).release_identity_refines(old(self), request, &result),
            match result {
                Ok(evidence) => {
                    &&& evidence.request_spec() == request
                    &&& evidence.origin_spec() == permit.origin_spec()
                    &&& permit.request_spec() == request
                    &&& request.generation_spec() < u32::MAX
                }
                Err(failure) => {
                    &&& final(self).same_state(old(self))
                    &&& failure.permit_request_spec() == permit.request_spec()
                    &&& failure.permit_origin_spec() == permit.origin_spec()
                }
            },
    {
        let origin = permit.origin();
        if permit.request() != request {
            return Err(KvAuthorityError {
                error: KvError::InvalidQuiescencePermit,
                permit,
            });
        }
        match self.release_request_key(request_key(request)) {
            Ok(()) => Ok(KvDetachedRequest { request, origin }),
            Err(error) => Err(KvAuthorityError { error, permit }),
        }
    }

    /// Validates an initialized logical range.
    pub fn validate_read(
        &self,
        request: RequestId,
        logical_offset: u32,
        span: u32,
    ) -> (result: Result<(), KvError>)
        requires self.well_formed(),
        ensures
            match result {
                Ok(()) => self.read_enabled(request, logical_offset, span),
                Err(_) => !self.read_enabled(request, logical_offset, span),
            },
    {
        reveal(KvPool::read_enabled);
        self.validate_read_key(request_key(request), logical_offset, span)
    }

    #[must_use]
    pub fn resident_tokens(&self, request: RequestId) -> (tokens: Option<u32>)
        requires self.well_formed(),
        ensures tokens == self.resident_tokens_spec(request),
    {
        reveal(KvPool::resident_tokens_spec);
        reveal(KvPool::request_matches_spec);
        reveal(KvPool::key_matches);
        let slot = self.requests.get(request.slot() as usize)?;
        if slot.live && slot.generation == request.generation() {
            Some(slot.resident_tokens)
        } else {
            None
        }
    }

    #[must_use]
    pub fn committed_tokens(&self, request: RequestId) -> (tokens: Option<u32>)
        requires self.well_formed(),
        ensures tokens == self.committed_tokens_spec(request),
    {
        reveal(KvPool::committed_tokens_spec);
        reveal(KvPool::request_matches_spec);
        reveal(KvPool::key_matches);
        let slot = self.requests.get(request.slot() as usize)?;
        if slot.live && slot.generation == request.generation() {
            Some(slot.committed_tokens)
        } else {
            None
        }
    }

    #[must_use]
    pub fn page_count(&self, request: RequestId) -> (count: Option<u32>)
        requires self.well_formed(),
        ensures count == self.page_count_spec(request),
    {
        reveal(KvPool::page_count_spec);
        reveal(KvPool::request_matches_spec);
        reveal(KvPool::key_matches);
        let slot = self.requests.get(request.slot() as usize)?;
        if slot.live && slot.generation == request.generation() {
            Some(slot.page_count)
        } else {
            None
        }
    }

    #[must_use]
    pub fn page_at(
        &self,
        request: RequestId,
        logical_page: u32,
    ) -> (page: Option<PageId>)
        requires self.well_formed(),
        ensures page == self.page_at_spec(request, logical_page),
    {
        reveal(KvPool::page_at_spec);
        reveal(KvPool::request_matches_spec);
        reveal(KvPool::key_matches);
        let slot = self.requests.get(request.slot() as usize)?;
        if !slot.live || slot.generation != request.generation() || logical_page >= slot.page_count
        {
            return None;
        }
        reveal(KvPool::request_slot_well_formed);
        assert(self.request_slot_well_formed(request.slot_spec() as int));
        assert(logical_page < MAX_PAGES_PER_REQUEST);
        Some(slot.pages[logical_page as usize])
    }

    #[must_use]
    pub fn free_pages(&self) -> (pages: u32)
        ensures pages == self.free_pages_spec(),
    {
        self.free_len
    }
}

} // verus!

impl fmt::Display for KvError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for KvError {}

#[cfg(test)]
mod tests {
    use super::{KvError, KvPool};
    use crate::epoch::ExactCompletion;
    use crate::scheduler::{KvQuiescencePermit, Scheduler};
    use ferric_spec::RequestId;

    fn request(slot: u32, generation: u32) -> RequestId {
        RequestId::new(slot, generation)
    }
    fn completed(expected: RequestId) -> KvQuiescencePermit {
        assert_eq!(expected.generation(), 1);
        let mut scheduler = Scheduler::<32>::new().unwrap();
        let mut admitted = request(0, 0);
        while admitted.slot() < expected.slot() {
            admitted = scheduler.admit().unwrap();
        }
        assert_eq!(admitted, expected);
        let mut members = [request(0, 0); 32];
        let batch = scheduler.dispatch_ready(&mut members).unwrap().unwrap();
        let mut permits: [Option<KvQuiescencePermit>; 32] = std::array::from_fn(|_| None);
        scheduler
            .complete_exact(
                ExactCompletion::from_contracted_hsa_quiescence(batch.epoch()),
                &mut permits,
            )
            .unwrap();
        permits
            .into_iter()
            .flatten()
            .find(|permit| permit.request() == expected)
            .unwrap()
    }

    #[test]
    fn append_commit_and_quiescent_rollback_preserve_committed_prefix() {
        let id = request(1, 1);
        let mut pool = KvPool::new(8, 4, 16).unwrap();
        pool.create_request(id).unwrap();
        pool.append_tentative(id, 8).unwrap();
        let retained_tail = pool.page_at(id, 1).unwrap();
        pool.finalize_tentative(id, 5, completed(id)).unwrap();
        assert_eq!(pool.committed_tokens(id), Some(5));
        assert_eq!(pool.resident_tokens(id), Some(5));
        assert_eq!(pool.page_count(id), Some(2));
        assert_eq!(pool.page_at(id, 1), Some(retained_tail));
        assert_eq!(pool.validate_read(id, 0, 5), Ok(()));
        assert_eq!(pool.validate_read(id, 4, 2), Err(KvError::ReadOutOfBounds));
    }

    #[test]
    fn rollback_invalidates_wholly_tentative_pages() {
        let first = request(1, 1);
        let second = request(2, 1);
        let mut pool = KvPool::new(8, 4, 16).unwrap();
        pool.create_request(first).unwrap();
        pool.append_tentative(first, 4).unwrap();
        pool.append_tentative(first, 4).unwrap();
        let stale = pool.page_at(first, 1).unwrap();
        pool.finalize_tentative(first, 4, completed(first)).unwrap();
        pool.create_request(second).unwrap();
        pool.append_tentative(second, 1).unwrap();
        assert_ne!(pool.page_at(second, 0), Some(stale));
        assert_eq!(pool.free_pages(), 6);
    }

    #[test]
    fn page_aligned_sharing_is_immutable_and_extends_by_cow() {
        let source = request(1, 1);
        let target = request(2, 1);
        let mut pool = KvPool::new(8, 4, 16).unwrap();
        pool.create_request(source).unwrap();
        pool.create_request(target).unwrap();
        pool.append_tentative(source, 8).unwrap();
        pool.finalize_tentative(source, 8, completed(source))
            .unwrap();
        pool.share_committed_prefix(source, target, 8).unwrap();
        assert_eq!(pool.page_at(source, 0), pool.page_at(target, 0));
        assert_eq!(pool.page_at(source, 1), pool.page_at(target, 1));
        pool.append_tentative(target, 1).unwrap();
        assert_eq!(pool.page_at(source, 2), None);
        assert!(pool.page_at(target, 2).is_some());
        assert_eq!(pool.resident_tokens(source), Some(8));
        assert_eq!(pool.resident_tokens(target), Some(9));
        assert_eq!(pool.validate_read(target, 3, 6), Ok(()));
    }

    #[test]
    fn partial_prefix_and_nonempty_target_are_rejected_transactionally() {
        let source = request(1, 1);
        let target = request(2, 1);
        let mut pool = KvPool::new(8, 4, 16).unwrap();
        pool.create_request(source).unwrap();
        pool.create_request(target).unwrap();
        pool.append_tentative(source, 8).unwrap();
        pool.finalize_tentative(source, 8, completed(source))
            .unwrap();
        assert_eq!(
            pool.share_committed_prefix(source, target, 6),
            Err(KvError::PrefixNotPageAligned)
        );
        assert_eq!(pool.page_count(target), Some(0));
        pool.append_tentative(target, 1).unwrap();
        assert_eq!(
            pool.share_committed_prefix(source, target, 4),
            Err(KvError::TargetNotEmpty)
        );
        assert_eq!(pool.resident_tokens(target), Some(1));
    }

    #[test]
    fn shared_reference_release_does_not_recycle_live_prefix() {
        let source = request(1, 1);
        let target = request(2, 1);
        let mut pool = KvPool::new(4, 4, 16).unwrap();
        pool.create_request(source).unwrap();
        pool.create_request(target).unwrap();
        pool.append_tentative(source, 4).unwrap();
        pool.finalize_tentative(source, 4, completed(source))
            .unwrap();
        pool.share_committed_prefix(source, target, 4).unwrap();
        let shared = pool.page_at(target, 0).unwrap();
        pool.release_request(source, completed(source)).unwrap();
        assert_eq!(pool.page_at(target, 0), Some(shared));
        assert_eq!(pool.validate_read(target, 0, 4), Ok(()));
        assert_eq!(pool.free_pages(), 3);
    }

    #[test]
    fn request_slot_requires_the_next_generation() {
        let first = request(1, 1);
        let next = request(1, 2);
        let mut pool = KvPool::new(4, 4, 16).unwrap();
        pool.create_request(first).unwrap();
        pool.release_request(first, completed(first)).unwrap();
        assert!(matches!(
            pool.create_request(first),
            Err(KvError::StaleRequestGeneration { .. })
        ));
        pool.create_request(next).unwrap();
    }

    #[test]
    fn out_of_pages_is_transactional() {
        let id = request(1, 1);
        let mut pool = KvPool::new(1, 4, 16).unwrap();
        pool.create_request(id).unwrap();
        pool.append_tentative(id, 3).unwrap();
        assert_eq!(pool.append_tentative(id, 2), Err(KvError::OutOfPages));
        assert_eq!(pool.resident_tokens(id), Some(3));
        assert_eq!(pool.page_count(id), Some(1));
        assert_eq!(pool.free_pages(), 0);
    }

    #[test]
    fn wrong_permit_cannot_reclaim() {
        let id = request(1, 1);
        let other = request(2, 1);
        let mut pool = KvPool::new(2, 4, 8).unwrap();
        pool.create_request(id).unwrap();
        pool.append_tentative(id, 4).unwrap();
        let free_before = pool.free_pages();
        assert_eq!(
            pool.finalize_tentative(id, 0, completed(other))
                .unwrap_err()
                .into_parts()
                .0,
            KvError::InvalidQuiescencePermit
        );
        assert_eq!(pool.free_pages(), free_before);
        assert_eq!(pool.resident_tokens(id), Some(4));
    }

    #[test]
    fn transition_paths_preserve_fixed_storage_capacities() {
        let source = request(1, 1);
        let target = request(2, 1);
        let mut pool = KvPool::new(8, 4, 16).unwrap();
        let capacities = (
            pool.pages.capacity(),
            pool.free_stack.capacity(),
            pool.free_bitmap.capacity(),
            pool.requests.capacity(),
        );

        pool.create_request(source).unwrap();
        pool.create_request(target).unwrap();
        pool.append_tentative(source, 8).unwrap();
        pool.finalize_tentative(source, 8, completed(source))
            .unwrap();
        pool.share_committed_prefix(source, target, 4).unwrap();
        pool.append_tentative(target, 1).unwrap();
        pool.release_request(source, completed(source)).unwrap();

        assert_eq!(
            capacities,
            (
                pool.pages.capacity(),
                pool.free_stack.capacity(),
                pool.free_bitmap.capacity(),
                pool.requests.capacity(),
            )
        );
        assert_eq!(pool.pages.len(), super::MAX_PAGE_SLOTS);
        assert_eq!(pool.free_stack.len(), super::MAX_PAGE_SLOTS);
        assert_eq!(pool.free_bitmap.len(), super::MAX_PAGE_SLOTS);
        assert_eq!(pool.requests.len(), super::MAX_REQUEST_SLOTS);
    }
}
