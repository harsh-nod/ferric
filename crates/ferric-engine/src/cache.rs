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
    pub fn index(self) -> u32 { self.index }

    #[must_use]
    pub fn generation(self) -> u32 { self.generation }
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

    pub(crate) closed spec fn same_state(&self, other: &Self) -> bool {
        &&& self.page_tokens == other.page_tokens
        &&& self.max_context_tokens == other.max_context_tokens
        &&& self.page_limit == other.page_limit
        &&& self.pages == other.pages
        &&& self.free_stack == other.free_stack
        &&& self.free_len == other.free_len
        &&& self.free_bitmap == other.free_bitmap
        &&& self.requests == other.requests
    }

    closed spec fn request_frame_except(&self, old: &Self, changed: int) -> bool {
        forall |request_index: int|
            0 <= request_index < MAX_REQUEST_SLOTS && request_index != changed ==>
                self.requests@[request_index] == old.requests@[request_index]
    }

    closed spec fn sealed_payload_frame(&self, old: &Self) -> bool {
        forall |page_index: int|
            0 <= page_index < old.page_limit
                && old.pages@[page_index].state == PageState::Sealed ==>
                    self.pages@[page_index].generation == old.pages@[page_index].generation
                        && self.pages@[page_index].state == PageState::Sealed
                        && self.pages@[page_index].initialized_tokens
                            == old.pages@[page_index].initialized_tokens
    }

    closed spec fn reachable_payload_frame_except(&self, old: &Self, excluded: int) -> bool {
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

    fn new_bounded(
        page_count: u32,
        page_tokens: u32,
        max_context_tokens: u32,
    ) -> (result: Result<Self, KvError>)
        ensures
            match result {
                Ok(pool) => pool.well_formed(),
                Err(_) => true,
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
            match result {
                Ok(()) => {
                    &&& request.slot < MAX_REQUEST_SLOTS
                    &&& final(self).requests@[request.slot as int].live
                    &&& final(self).requests@[request.slot as int].generation == request.generation
                    &&& final(self).request_frame_except(old(self), request.slot as int)
                }
                Err(_) => final(self).same_state(old(self)),
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
            match result {
                Ok(()) => {
                    &&& request.slot < MAX_REQUEST_SLOTS
                    &&& final(self).requests@[request.slot as int].resident_tokens
                        == old(self).requests@[request.slot as int].resident_tokens + token_count
                    &&& final(self).requests@[request.slot as int].committed_tokens
                        == old(self).requests@[request.slot as int].committed_tokens
                    &&& final(self).request_frame_except(old(self), request.slot as int)
                    &&& final(self).sealed_payload_frame(old(self))
                    &&& final(self).reachable_payload_frame_except(
                        old(self),
                        request.slot as int,
                    )
                }
                Err(_) => final(self).same_state(old(self)),
            },
    {
        let request_index = self.live_request_index(request)?;
        let old_resident = self.requests[request_index].resident_tokens;
        let new_resident = match old_resident.checked_add(token_count) {
            Some(value) => value,
            None => return Err(KvError::ContextExceeded),
        };
        if new_resident > self.max_context_tokens { return Err(KvError::ContextExceeded); }
        if token_count == 0 { return Ok(()); }

        let old_page_count = self.requests[request_index].page_count;
        let tail_capacity = if old_page_count == 0 {
            0
        } else {
            let tail = self.requests[request_index].pages[(old_page_count - 1) as usize];
            let slot = self.page_slot(tail)?;
            let state = slot.state;
            match state {
                PageState::Writable { owner_slot } => {
                    if owner_slot as u32 != request.slot {
                        return Err(KvError::InvariantViolation(Invariant::PageState));
                    }
                    self.page_tokens - slot.initialized_tokens
                }
                PageState::Sealed => 0,
                PageState::Free => return Err(KvError::InvariantViolation(Invariant::PageState)),
            }
        };
        let after_tail = token_count.saturating_sub(tail_capacity);
        let required_pages = if after_tail == 0 { 0 } else {
            (after_tail / self.page_tokens) + u32::from(after_tail % self.page_tokens != 0)
        };
        let final_page_count = match old_page_count.checked_add(required_pages) {
            Some(value) => value,
            None => return Err(KvError::RequestPageTableFull),
        };
        if final_page_count as usize > MAX_PAGES_PER_REQUEST {
            return Err(KvError::RequestPageTableFull);
        }
        if required_pages > self.free_len { return Err(KvError::OutOfPages); }

        let mut remaining = token_count;
        while remaining > 0
            invariant
                self.well_formed(),
                request.slot < MAX_REQUEST_SLOTS,
                self.requests@[request.slot as int].live,
                self.requests@[request.slot as int].generation == request.generation,
                self.requests@[request.slot as int].resident_tokens as int + remaining as int
                    == old(self).requests@[request.slot as int].resident_tokens as int
                        + token_count as int,
                self.requests@[request.slot as int].committed_tokens
                    == old(self).requests@[request.slot as int].committed_tokens,
                self.request_frame_except(old(self), request.slot as int),
                self.sealed_payload_frame(old(self)),
            decreases remaining,
        {
            let current_page_count = self.requests[request_index].page_count;
            let use_existing = if current_page_count > 0 {
                let tail = self.requests[request_index].pages[(current_page_count - 1) as usize];
                let state = self.pages[tail.index as usize].state;
                let writable_by_request = match state {
                    PageState::Writable { owner_slot } => owner_slot as u32 == request.slot,
                    PageState::Free | PageState::Sealed => false,
                };
                writable_by_request
                    && self.pages[tail.index as usize].initialized_tokens < self.page_tokens
            } else {
                false
            };
            let written = if use_existing {
                let tail = self.requests[request_index].pages[(current_page_count - 1) as usize];
                let available = self.page_tokens - self.pages[tail.index as usize].initialized_tokens;
                let written = remaining.min(available);
                self.append_existing_page(request, request_index, tail, written);
                written
            } else {
                let written = remaining.min(self.page_tokens);
                self.append_fresh_page(request, request_index, written);
                written
            };
            remaining -= written;
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
            match result {
                Ok(()) => {
                    &&& target.slot < MAX_REQUEST_SLOTS
                    &&& final(self).requests@[target.slot as int].resident_tokens == token_count
                    &&& final(self).requests@[target.slot as int].committed_tokens == token_count
                    &&& final(self).requests@[target.slot as int].page_count
                        == token_count / final(self).page_tokens
                    &&& final(self).requests@[source.slot as int]
                        == old(self).requests@[source.slot as int]
                    &&& forall |request_index: int|
                        0 <= request_index < MAX_REQUEST_SLOTS
                            && request_index != source.slot
                            && request_index != target.slot ==>
                                final(self).requests@[request_index]
                                    == old(self).requests@[request_index]
                    &&& final(self).sealed_payload_frame(old(self))
                    &&& forall |position: int|
                        0 <= position < final(self).requests@[target.slot as int].page_count ==>
                            final(self).requests@[target.slot as int].pages@[position]
                                == old(self).requests@[source.slot as int].pages@[position]
                }
                Err(_) => final(self).same_state(old(self)),
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

        let mut position = 0_u32;
        while position < shared_pages
            decreases shared_pages - position,
        {
            let page = self.requests[source_index].pages[position as usize];
            let slot = self.page_slot(page)?;
            if slot.initialized_tokens != self.page_tokens {
                return Err(KvError::PrefixPageIncomplete(page));
            }
            let state = slot.state;
            match state {
                PageState::Writable { owner_slot } => {
                    if owner_slot as u32 != source.slot {
                        return Err(KvError::InvariantViolation(Invariant::PageState));
                    }
                }
                PageState::Sealed => {
                    if reference_is_set(slot.reference_mask, target.slot) {
                        return Err(KvError::ReferenceCountExhausted(page));
                    }
                }
                _ => return Err(KvError::InvariantViolation(Invariant::PageState)),
            }
            position += 1;
        }

        position = 0;
        while position < shared_pages
            decreases shared_pages - position,
        {
            let page = self.requests[source_index].pages[position as usize];
            let page_index = page.index as usize;
            if matches!(self.pages[page_index].state, PageState::Writable { .. }) {
                self.pages[page_index].state = PageState::Sealed;
                self.pages[page_index].reference_mask =
                    set_reference(self.pages[page_index].reference_mask, source.slot);
            }
            self.pages[page_index].reference_mask =
                set_reference(self.pages[page_index].reference_mask, target.slot);
            self.requests[target_index].pages[position as usize] = page;
            position += 1;
        }
        self.requests[target_index].page_count = shared_pages;
        self.requests[target_index].resident_tokens = token_count;
        self.requests[target_index].committed_tokens = token_count;
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
                Ok(()) => logical_offset as int + span as int
                    <= self.requests@[request.slot as int].resident_tokens,
                Err(_) => true,
            },
    {
        let request_index = self.live_request_index(request)?;
        let end = match logical_offset.checked_add(span) {
            Some(value) => value,
            None => return Err(KvError::ReadOutOfBounds),
        };
        if end > self.requests[request_index].resident_tokens { return Err(KvError::ReadOutOfBounds); }
        if span == 0 { return Ok(()); }

        let first_page = logical_offset / self.page_tokens;
        let last_page = (end - 1) / self.page_tokens;
        let mut logical_page = first_page;
        while logical_page <= last_page
            decreases last_page - logical_page + 1,
        {
            let page = self.requests[request_index].pages[logical_page as usize];
            let slot = self.page_slot(page)?;
            let start_in_page = if logical_page == first_page { logical_offset % self.page_tokens } else { 0 };
            let end_in_page = if logical_page == last_page {
                ((end - 1) % self.page_tokens) + 1
            } else {
                self.page_tokens
            };
            if start_in_page >= end_in_page || end_in_page > slot.initialized_tokens {
                return Err(KvError::ReadUninitialized(page));
            }
            let state = slot.state;
            match state {
                PageState::Writable { owner_slot } => {
                    if owner_slot as u32 != request.slot {
                        return Err(KvError::InvariantViolation(Invariant::PageState));
                    }
                }
                PageState::Sealed => {}
                _ => return Err(KvError::InvariantViolation(Invariant::PageState)),
            }
            logical_page += 1;
        }
        Ok(())
    }

    fn live_request_index(&self, request: RequestKey) -> (result: Result<usize, KvError>)
        requires self.well_formed(),
        ensures
            match result {
                Ok(index) => {
                    &&& index < MAX_REQUEST_SLOTS
                    &&& index == request.slot
                    &&& self.requests@[index as int].live
                    &&& self.requests@[index as int].generation == request.generation
                }
                Err(_) => true,
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
                Err(_) => true,
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
            final(self).request_frame_except(old(self), request_index as int),
            final(self).pages@[page.index as int].initialized_tokens
                == old(self).pages@[page.index as int].initialized_tokens + written,
            forall |page_index: int|
                0 <= page_index < old(self).page_limit && page_index != page.index ==>
                    final(self).pages@[page_index] == old(self).pages@[page_index],
            final(self).free_stack == old(self).free_stack,
            final(self).free_len == old(self).free_len,
            final(self).free_bitmap == old(self).free_bitmap,
    {
        let page_index = page.index as usize;
        self.pages[page_index].initialized_tokens += written;
        self.requests[request_index].resident_tokens += written;
        assert forall |index: int| 0 <= index < MAX_REQUEST_SLOTS implies
            #[trigger] self.request_slot_well_formed(index) by {
            assert(old(self).request_slot_well_formed(index));
            if index == request_index {
                assert(self.requests@[index].page_count == old(self).requests@[index].page_count);
                assert(self.requests@[index].pages == old(self).requests@[index].pages);
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
        assert(self.well_formed());
    }

    fn append_fresh_page(&mut self, request: RequestKey, request_index: usize, written: u32) {
        let stack_index = (self.free_len - 1) as usize;
        let page_index = self.free_stack[stack_index] as usize;
        let chain_position = self.requests[request_index].page_count as usize;

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
    ) -> Result<Self, KvError> {
        Self::new_bounded(page_count, page_tokens, max_context_tokens)
    }

    /// Activates the exact generation expected by a scheduler request slot.
    pub fn create_request(&mut self, request: RequestId) -> Result<(), KvError> {
        self.create_request_key(request_key(request))
    }

    /// Materializes tentative logical KV positions.
    pub fn append_tentative(
        &mut self,
        request: RequestId,
        token_count: u32,
    ) -> Result<(), KvError> {
        self.append_tentative_key(request_key(request), token_count)
    }

    /// Shares only complete, committed, page-aligned prefix pages.
    pub fn share_committed_prefix(
        &mut self,
        source: RequestId,
        target: RequestId,
        token_count: u32,
    ) -> Result<(), KvError> {
        self.share_committed_prefix_key(
            request_key(source),
            request_key(target),
            token_count,
        )
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
    ) -> Result<(), KvError> {
        self.validate_read_key(request_key(request), logical_offset, span)
    }

    #[must_use]
    pub fn resident_tokens(&self, request: RequestId) -> Option<u32> {
        let slot = self.requests.get(request.slot() as usize)?;
        if slot.live && slot.generation == request.generation() {
            Some(slot.resident_tokens)
        } else {
            None
        }
    }

    #[must_use]
    pub fn committed_tokens(&self, request: RequestId) -> Option<u32> {
        let slot = self.requests.get(request.slot() as usize)?;
        if slot.live && slot.generation == request.generation() {
            Some(slot.committed_tokens)
        } else {
            None
        }
    }

    #[must_use]
    pub fn page_count(&self, request: RequestId) -> Option<u32> {
        let slot = self.requests.get(request.slot() as usize)?;
        if slot.live && slot.generation == request.generation() {
            Some(slot.page_count)
        } else {
            None
        }
    }

    #[must_use]
    pub fn page_at(&self, request: RequestId, logical_page: u32) -> Option<PageId> {
        let slot = self.requests.get(request.slot() as usize)?;
        if !slot.live || slot.generation != request.generation() || logical_page >= slot.page_count
        {
            return None;
        }
        Some(slot.pages[logical_page as usize])
    }

    #[must_use]
    pub fn free_pages(&self) -> u32 {
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
