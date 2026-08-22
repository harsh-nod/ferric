//! Fixed-capacity request slots and deterministic scheduling.

use crate::cache::{KvDetachedRequest, KvFinalizedRequest, MAX_REQUEST_SLOTS};
use crate::epoch::ExactCompletion;
use ferric_spec::completion::CompletionEpoch;
#[allow(unused_imports)]
use ferric_spec::scheduling::{LifecyclePhase, RequestState, RequestTransition, SequentialRequest};
use ferric_spec::{RequestId, M1_MAX_ACTIVE_SEQUENCES};
use vstd::prelude::*;

verus! {

const NO_EPOCH: u64 = 0;

fn slot_index_to_u32(slot_index: usize) -> (result: u32)
    requires slot_index < MAX_REQUEST_SLOTS,
    ensures result as int == slot_index as int,
{
    let Ok(result) = u32::try_from(slot_index) else {
        return u32::MAX;
    };
    result
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Slot {
    generation: u32,
    state: RequestState,
    active_epoch: u64,
    last_quiescent_epoch: u64,
    in_free_ring: bool,
    in_reclaim_ring: bool,
}

impl Slot {
    const INITIAL: Self = Self {
        generation: 1,
        state: RequestState::Vacant,
        active_epoch: NO_EPOCH,
        last_quiescent_epoch: NO_EPOCH,
        in_free_ring: true,
        in_reclaim_ring: false,
    };
}

/// Linear authority for exactly one request generation after GPU quiescence.
///
/// Only `complete_exact` and the already-quiescent retirement ring can create
/// this value. Cache finalization or detachment consumes it. Dropping it leaves
/// the request undispatchable and therefore fails by leaking capacity.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct KvQuiescencePermit {
    request: RequestId,
    origin: KvQuiescenceOrigin,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KvQuiescenceOrigin {
    NeverSubmitted,
    CompletedExact { epoch: u64 },
}

impl KvQuiescencePermit {
    pub(crate) const fn request(&self) -> (request: RequestId)
        ensures request == self.request_spec(),
    {
        self.request
    }

    pub(crate) const fn origin(&self) -> (origin: KvQuiescenceOrigin)
        ensures origin == self.origin_spec(),
    {
        self.origin
    }

    pub(crate) closed spec fn request_spec(&self) -> RequestId {
        self.request
    }

    pub(crate) closed spec fn origin_spec(&self) -> KvQuiescenceOrigin {
        self.origin
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BatchRecord {
    epoch: CompletionEpoch,
    member_count: usize,
}

impl BatchRecord {
    const EMPTY: Self = Self {
        epoch: CompletionEpoch { value: 0 },
        member_count: 0,
    };
}

/// Successful dispatch metadata. Members occupy the prefix written to the
/// caller-provided output slice.
///
/// ```compile_fail
/// use ferric_engine::DispatchBatch;
/// fn require_clone<T: Clone>() {}
/// require_clone::<DispatchBatch>();
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct DispatchBatch {
    epoch: CompletionEpoch,
    member_count: usize,
}

impl DispatchBatch {
    #[must_use]
    pub const fn epoch(&self) -> (epoch: CompletionEpoch)
        ensures epoch.value == self.epoch_spec().value,
    {
        self.epoch
    }

    #[must_use]
    pub const fn member_count(&self) -> (count: usize)
        ensures count == self.member_count_spec(),
    {
        self.member_count
    }

    pub closed spec fn member_count_spec(&self) -> usize {
        self.member_count
    }

    pub closed spec fn epoch_spec(&self) -> CompletionEpoch {
        self.epoch
    }
}

/// Linear M1 scheduler dispatch authority with an exact fixed-capacity member roster.
///
/// Only [`crate::Engine::dispatch_m1_ready`] constructs this owner. The scheduler
/// batch and immutable request prefix move together into physical queue custody.
/// Entries beyond [`Self::member_count`] are canonical `None` padding.
///
/// ```compile_fail
/// use ferric_engine::M1ScheduledDispatchV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1ScheduledDispatchV1>();
/// ```
#[must_use = "scheduled M1 dispatch authority must enter physical queue custody"]
#[derive(Debug, PartialEq, Eq)]
pub struct M1ScheduledDispatchV1 {
    batch: DispatchBatch,
    members: [Option<RequestId>; M1_MAX_ACTIVE_SEQUENCES as usize],
}

impl M1ScheduledDispatchV1 {
    pub(crate) fn from_dispatch_batch(
        batch: DispatchBatch,
        selected: &[RequestId; M1_MAX_ACTIVE_SEQUENCES as usize],
    ) -> Self
        requires batch.member_count_spec() <= M1_MAX_ACTIVE_SEQUENCES as usize,
    {
        let count = batch.member_count();
        let mut members = [None; M1_MAX_ACTIVE_SEQUENCES as usize];
        let mut index = 0;
        while index < count
            invariant
                index <= count,
                count <= M1_MAX_ACTIVE_SEQUENCES as usize,
                members@.len() == M1_MAX_ACTIVE_SEQUENCES as usize,
                selected@.len() == M1_MAX_ACTIVE_SEQUENCES as usize,
            decreases count - index,
        {
            members[index] = Some(selected[index]);
            index += 1;
        }
        Self { batch, members }
    }

    /// Returns the exact scheduler-issued completion epoch.
    #[must_use]
    pub const fn epoch(&self) -> CompletionEpoch {
        self.batch.epoch()
    }

    /// Returns the nonzero scheduler-selected prefix length.
    #[must_use]
    pub const fn member_count(&self) -> usize {
        self.batch.member_count()
    }

    /// Returns one exact scheduler-selected request or `None` outside the live prefix.
    #[must_use]
    pub fn member(&self, index: usize) -> Option<RequestId> {
        if index < M1_MAX_ACTIVE_SEQUENCES as usize {
            self.members[index]
        } else {
            None
        }
    }

    /// Returns the fixed M1 roster with canonical `None` padding.
    #[must_use]
    pub const fn members(
        &self,
    ) -> &[Option<RequestId>; M1_MAX_ACTIVE_SEQUENCES as usize] {
        &self.members
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SchedulerError {
    ZeroCapacity,
    CapacityExceedsKvSlots,
    OutOfSlots,
    EmptyBatchStorage,
    CompletionStorageTooSmall,
    CompletionStorageNotEmpty,
    SubmissionEpochExhausted,
    InvalidSlot,
    StaleRequest,
    RequestNotLive,
    AlreadyRetiring,
    NoPendingBatch,
    CompletionNotExactNext,
    CompletionEpochMismatch,
    FinalizationMismatch,
    DetachmentMismatch,
    GenerationExhausted,
    InvariantViolation,
}

/// A framed completion failure returns the unchanged linear quiescence
/// authority so the caller can correct transient storage/preflight errors and
/// retry. Success consumes the authority exactly once.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CompletionFailure {
    error: SchedulerError,
    completion: ExactCompletion,
}

impl CompletionFailure {
    pub(crate) fn into_parts(self) -> (parts: (SchedulerError, ExactCompletion))
        ensures
            parts.0 == self.error_spec(),
            parts.1.epoch_spec() == self.completion_epoch_spec(),
    {
        (self.error, self.completion)
    }

    pub(crate) closed spec fn error_spec(&self) -> SchedulerError {
        self.error
    }

    pub(crate) closed spec fn completion_epoch_spec(&self) -> CompletionEpoch {
        self.completion.epoch_spec()
    }
}

/// Fixed-capacity, allocation-free-after-construction request lifecycle.
///
/// `C` is part of the generated runner. Every ring is embedded in the value;
/// admission, cancellation, and reclamation are O(1), completion is O(batch),
/// and dispatch performs at most one O(C) rotating scan.
pub struct Scheduler<const C: usize> {
    slots: [Slot; C],
    free_ring: [usize; C],
    free_head: usize,
    free_len: usize,
    reclaim_ring: [usize; C],
    reclaim_head: usize,
    reclaim_len: usize,
    member_ring: [RequestId; C],
    member_head: usize,
    member_len: usize,
    batch_ring: [BatchRecord; C],
    batch_head: usize,
    batch_len: usize,
    cursor: usize,
    submitted: u64,
    completed: u64,
    live_count: usize,
}

impl<const C: usize> Scheduler<C> {
    /// Builds every slot and bounded ring. No later method allocates.
    ///
    /// # Errors
    ///
    /// Returns [`SchedulerError::ZeroCapacity`] for `C == 0` and
    /// [`SchedulerError::CapacityExceedsKvSlots`] when `C` exceeds the KV
    /// request table capacity.
    pub fn new() -> (result: Result<Self, SchedulerError>)
        ensures
            match result {
                Ok(scheduler) => {
                    &&& C > 0
                    &&& C <= MAX_REQUEST_SLOTS
                    &&& scheduler.basic_invariant()
                    &&& (forall|slot_index: int| 0 <= slot_index < C ==> {
                        &&& !scheduler.slot_is_live_spec(slot_index)
                        &&& scheduler.slot_generation_spec(slot_index) == 1
                    })
                }
                Err(error) => {
                    &&& error == if C == 0 {
                        SchedulerError::ZeroCapacity
                    } else {
                        SchedulerError::CapacityExceedsKvSlots
                    }
                    &&& (C == 0 || C > MAX_REQUEST_SLOTS)
                }
            },
    {
        reveal(Scheduler::basic_invariant);
        reveal(Scheduler::scalar_invariant);
        reveal(Scheduler::slot_invariant);
        reveal(Scheduler::free_ring_invariant);
        reveal(Scheduler::reclaim_ring_invariant);
        reveal(Scheduler::member_ring_invariant);
        reveal(Scheduler::batch_ring_invariant);
        if C == 0 {
            return Err(SchedulerError::ZeroCapacity);
        }
        if C > MAX_REQUEST_SLOTS {
            return Err(SchedulerError::CapacityExceedsKvSlots);
        }

        let mut free_ring = [0; C];
        let mut index = 0;
        while index < C
            invariant
                index <= C,
                forall|i: int| 0 <= i < index ==> free_ring[i] == i,
            decreases C - index,
        {
            free_ring[index] = index;
            index += 1;
        }

        let scheduler = Self {
            slots: [Slot::INITIAL; C],
            free_ring,
            free_head: 0,
            free_len: C,
            reclaim_ring: [0; C],
            reclaim_head: 0,
            reclaim_len: 0,
            member_ring: [RequestId::new(0, 0); C],
            member_head: 0,
            member_len: 0,
            batch_ring: [BatchRecord::EMPTY; C],
            batch_head: 0,
            batch_len: 0,
            cursor: 0,
            submitted: 0,
            completed: 0,
            live_count: 0,
        };
        assert forall|slot_index: int| 0 <= slot_index < C implies
            (#[trigger] scheduler.slots@[slot_index]).state == RequestState::Vacant
        by {}
        proof {
            live_count_all_vacant(scheduler.slots@, C as nat);
            nonreclaim_count_all_vacant(scheduler.slots@, C as nat);
        }
        assert(scheduler.scalar_invariant());
        assert(scheduler.slot_invariant());
        assert(scheduler.free_ring_invariant()) by {
            assert forall|slot_index: int| 0 <= slot_index < C implies
                usize_ring_contains::<C>(
                    scheduler.free_ring@,
                    scheduler.free_head,
                    scheduler.free_len,
                    slot_index,
                )
            by {
                assert(ring_position::<C>(0, slot_index as nat) == slot_index);
                assert(scheduler.free_ring@[slot_index] == slot_index);
                assert(exists|offset: int| 0 <= offset < C
                    && (#[trigger] scheduler.free_ring@[
                        ring_position::<C>(scheduler.free_head, offset as nat)
                    ]) == slot_index) by {
                    let offset = slot_index;
                    assert(scheduler.free_ring@[
                        ring_position::<C>(scheduler.free_head, offset as nat)
                    ] == slot_index);
                }
            }
        }
        assert(scheduler.reclaim_ring_invariant());
        assert(scheduler.member_ring_invariant());
        assert(scheduler.batch_ring_invariant());
        assert(scheduler.basic_invariant());
        Ok(scheduler)
    }

    pub closed spec fn scalar_invariant(&self) -> bool {
        &&& C > 0
        &&& C <= MAX_REQUEST_SLOTS
        &&& self.free_head < C
        &&& self.free_len <= C
        &&& self.reclaim_head < C
        &&& self.reclaim_len <= C
        &&& self.member_head < C
        &&& self.member_len <= C
        &&& self.batch_head < C
        &&& self.batch_len <= C
        &&& self.cursor < C
        &&& self.live_count <= C
        &&& self.live_count == live_slot_count(self.slots@, C as nat)
        &&& self.reclaim_len + nonreclaim_live_count(self.slots@, C as nat)
            == self.live_count
        &&& self.free_len + self.live_count == C
        &&& self.reclaim_len <= self.live_count
        &&& self.member_len <= self.live_count
        &&& self.batch_len <= self.member_len
        &&& self.completed <= self.submitted
        &&& self.submitted as int == self.completed as int + self.batch_len as int
    }

    pub closed spec fn slot_invariant(&self) -> bool {
        forall|slot_index: int| 0 <= slot_index < C ==> {
            let slot = #[trigger] self.slots@[slot_index];
            match slot.state {
                RequestState::Vacant => {
                    &&& slot.active_epoch == NO_EPOCH
                    &&& slot.last_quiescent_epoch == NO_EPOCH
                    &&& slot.in_free_ring
                    &&& !slot.in_reclaim_ring
                }
                RequestState::Ready => {
                    &&& slot.active_epoch == NO_EPOCH
                    &&& slot.last_quiescent_epoch <= self.completed
                    &&& !slot.in_free_ring
                    &&& !slot.in_reclaim_ring
                }
                RequestState::InFlight => {
                    &&& NO_EPOCH < slot.active_epoch <= self.submitted
                    &&& slot.last_quiescent_epoch <= self.completed
                    &&& !slot.in_free_ring
                    &&& !slot.in_reclaim_ring
                }
                RequestState::Retiring => {
                    &&& !slot.in_free_ring
                    &&& slot.active_epoch <= self.submitted
                    &&& slot.last_quiescent_epoch <= self.completed
                }
            }
        }
    }

    closed spec fn slot_invariant_at(&self, slot_index: int) -> bool {
        let slot = self.slots@[slot_index];
        match slot.state {
            RequestState::Vacant => {
                &&& slot.active_epoch == NO_EPOCH
                &&& slot.last_quiescent_epoch == NO_EPOCH
                &&& slot.in_free_ring
                &&& !slot.in_reclaim_ring
            }
            RequestState::Ready => {
                &&& slot.active_epoch == NO_EPOCH
                &&& slot.last_quiescent_epoch <= self.completed
                &&& !slot.in_free_ring
                &&& !slot.in_reclaim_ring
            }
            RequestState::InFlight => {
                &&& NO_EPOCH < slot.active_epoch <= self.submitted
                &&& slot.last_quiescent_epoch <= self.completed
                &&& !slot.in_free_ring
                &&& !slot.in_reclaim_ring
            }
            RequestState::Retiring => {
                &&& !slot.in_free_ring
                &&& slot.active_epoch <= self.submitted
                &&& slot.last_quiescent_epoch <= self.completed
            }
        }
    }

    pub closed spec fn free_ring_invariant(&self) -> bool {
        &&& (forall|offset: int| 0 <= offset < self.free_len ==> {
            let slot_index = #[trigger] self.free_ring@[
                ring_position::<C>(self.free_head, offset as nat)
            ];
            &&& slot_index < C
            &&& self.slots@[slot_index as int].state == RequestState::Vacant
            &&& self.slots@[slot_index as int].in_free_ring
        })
        &&& (forall|left: int, right: int|
            0 <= left < self.free_len && 0 <= right < self.free_len && left != right ==>
                #[trigger] usize_ring_entries_differ::<C>(
                    self.free_ring@,
                    self.free_head,
                    left,
                    right,
                ))
        &&& (forall|slot_index: int| 0 <= slot_index < C ==>
            #[trigger] self.slots@[slot_index].in_free_ring
                == usize_ring_contains::<C>(
                    self.free_ring@,
                    self.free_head,
                    self.free_len,
                    slot_index,
                ))
        &&& (forall|slot_index: int| 0 <= slot_index < C
            && !(#[trigger] self.slots@[slot_index].in_free_ring) ==> self.free_len < C)
    }

    pub closed spec fn reclaim_ring_invariant(&self) -> bool {
        &&& (forall|offset: int| 0 <= offset < self.reclaim_len ==> {
            let slot_index = #[trigger] self.reclaim_ring@[
                ring_position::<C>(self.reclaim_head, offset as nat)
            ];
            &&& slot_index < C
            &&& self.slots@[slot_index as int].state == RequestState::Retiring
            &&& self.slots@[slot_index as int].active_epoch == NO_EPOCH
            &&& self.slots@[slot_index as int].in_reclaim_ring
        })
        &&& (forall|left: int, right: int|
            0 <= left < self.reclaim_len && 0 <= right < self.reclaim_len && left != right ==>
                #[trigger] usize_ring_entries_differ::<C>(
                    self.reclaim_ring@,
                    self.reclaim_head,
                    left,
                    right,
                ))
        &&& (forall|slot_index: int| 0 <= slot_index < C ==>
            #[trigger] self.slots@[slot_index].in_reclaim_ring
                == usize_ring_contains::<C>(
                    self.reclaim_ring@,
                    self.reclaim_head,
                    self.reclaim_len,
                    slot_index,
                ))
        &&& (forall|slot_index: int| 0 <= slot_index < C
            && !(#[trigger] self.slots@[slot_index].in_reclaim_ring) ==> self.reclaim_len < C)
    }

    closed spec fn member_entry_valid(&self, offset: int) -> bool {
        let handle = self.member_ring@[
                ring_position::<C>(self.member_head, offset as nat)
            ];
        &&& handle.slot_spec() < C
        &&& self.slots@[handle.slot_spec() as int].generation
            == handle.generation_spec()
        &&& self.completed < self.slots@[handle.slot_spec() as int].active_epoch
        &&& self.slots@[handle.slot_spec() as int].active_epoch <= self.submitted
        &&& !self.slots@[handle.slot_spec() as int].in_reclaim_ring
        &&& (self.slots@[handle.slot_spec() as int].state == RequestState::InFlight
            || self.slots@[handle.slot_spec() as int].state == RequestState::Retiring)
    }

    closed spec fn member_entries_invariant(&self) -> bool {
        forall|offset: int| 0 <= offset < self.member_len ==>
            #[trigger] self.member_entry_valid(offset)
    }

    closed spec fn member_distinct_invariant(&self) -> bool {
        forall|left: int, right: int|
            0 <= left < self.member_len && 0 <= right < self.member_len && left != right ==>
                #[trigger] request_ring_slots_differ::<C>(
                    self.member_ring@,
                    self.member_head,
                    left,
                    right,
                )
    }

    closed spec fn member_membership_invariant(&self) -> bool {
        forall|slot_index: int| 0 <= slot_index < C ==>
            (((#[trigger] self.slots@[slot_index].state == RequestState::InFlight
                || self.slots@[slot_index].state == RequestState::Retiring)
                && self.completed < self.slots@[slot_index].active_epoch)
                == request_ring_contains_slot::<C>(
                    self.member_ring@,
                    self.member_head,
                    self.member_len,
                    slot_index,
                ))
    }

    pub closed spec fn member_ring_invariant(&self) -> bool {
        &&& self.member_entries_invariant()
        &&& self.member_distinct_invariant()
        &&& self.member_membership_invariant()
    }

    pub closed spec fn batch_ring_invariant(&self) -> bool {
        &&& batch_member_sum::<C>(self.batch_ring@, self.batch_head, self.batch_len as nat)
            == self.member_len
        &&& (forall|batch_offset: int| 0 <= batch_offset < self.batch_len ==> {
            let batch = #[trigger] self.batch_ring@[
                ring_position::<C>(self.batch_head, batch_offset as nat)
            ];
            &&& batch.member_count > 0
            &&& batch.epoch.value as int == self.completed as int + batch_offset + 1
            &&& batch.epoch.value <= self.submitted
            &&& (forall|member_offset: int|
                batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    batch_offset as nat,
                ) <= member_offset < batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    batch_offset as nat + 1,
                ) ==> {
                    let handle = #[trigger] self.member_ring@[
                        ring_position::<C>(self.member_head, member_offset as nat)
                    ];
                    self.slots@[handle.slot_spec() as int].active_epoch == batch.epoch.value
                })
        })
    }

    pub closed spec fn basic_invariant(&self) -> bool {
        &&& self.scalar_invariant()
        &&& self.slot_invariant()
        &&& self.free_ring_invariant()
        &&& self.reclaim_ring_invariant()
        &&& self.member_ring_invariant()
        &&& self.batch_ring_invariant()
    }

    pub closed spec fn same_scalars(&self, other: &Self) -> bool {
        &&& self.slots@ == other.slots@
        &&& self.free_ring@ == other.free_ring@
        &&& self.free_head == other.free_head
        &&& self.free_len == other.free_len
        &&& self.reclaim_ring@ == other.reclaim_ring@
        &&& self.reclaim_head == other.reclaim_head
        &&& self.reclaim_len == other.reclaim_len
        &&& self.member_ring@ == other.member_ring@
        &&& self.member_head == other.member_head
        &&& self.member_len == other.member_len
        &&& self.batch_ring@ == other.batch_ring@
        &&& self.batch_head == other.batch_head
        &&& self.batch_len == other.batch_len
        &&& self.cursor == other.cursor
        &&& self.submitted == other.submitted
        &&& self.completed == other.completed
        &&& self.live_count == other.live_count
    }

    pub(crate) proof fn same_scalars_reflexive(&self)
        ensures self.same_scalars(self),
    {
        reveal(Scheduler::same_scalars);
    }

    proof fn same_scalars_preserves_basic(&self, before: &Self)
        requires before.basic_invariant(), self.same_scalars(before),
        ensures self.basic_invariant(),
    {
        before.basic_implies_scalar();
        before.basic_implies_slots();
        before.basic_implies_free_ring();
        before.basic_implies_reclaim_ring();
        before.basic_implies_member_ring();
        before.basic_implies_batch_ring();
        assert(self.scalar_invariant()) by {
            reveal(Scheduler::same_scalars);
            reveal(Scheduler::scalar_invariant);
        }
        assert(self.slot_invariant()) by {
            reveal(Scheduler::same_scalars);
            reveal(Scheduler::slot_invariant);
        }
        assert(self.free_ring_invariant()) by {
            reveal(Scheduler::same_scalars);
            reveal(Scheduler::free_ring_invariant);
        }
        assert(self.reclaim_ring_invariant()) by {
            reveal(Scheduler::same_scalars);
            reveal(Scheduler::reclaim_ring_invariant);
        }
        assert(self.member_entries_invariant()) by {
            reveal(Scheduler::same_scalars);
            reveal(Scheduler::member_ring_invariant);
            reveal(Scheduler::member_entries_invariant);
            assert forall|offset: int| 0 <= offset < self.member_len implies
                #[trigger] self.member_entry_valid(offset) by {
                assert(before.member_entry_valid(offset));
                assert(self.member_entry_valid(offset)
                    == before.member_entry_valid(offset)) by {
                    reveal(Scheduler::member_entry_valid);
                }
            }
        }
        assert(self.member_distinct_invariant()) by {
            reveal(Scheduler::same_scalars);
            reveal(Scheduler::member_ring_invariant);
            reveal(Scheduler::member_distinct_invariant);
        }
        assert(self.member_membership_invariant()) by {
            reveal(Scheduler::same_scalars);
            reveal(Scheduler::member_ring_invariant);
            reveal(Scheduler::member_membership_invariant);
        }
        assert(self.member_ring_invariant()) by {
            reveal(Scheduler::member_ring_invariant);
        }
        assert(self.batch_ring_invariant()) by {
            reveal(Scheduler::same_scalars);
            reveal(Scheduler::batch_ring_invariant);
        }
        reveal(Scheduler::basic_invariant);
    }

    proof fn same_scalars_preserves_identity(&self, before: &Self)
        requires self.same_scalars(before),
        ensures self.identity_frame(before),
    {
        reveal(Scheduler::same_scalars);
        reveal(Scheduler::slot_generation_spec);
        reveal(Scheduler::slot_is_live_spec);
    }

    pub closed spec fn slot_model(&self, slot_index: int) -> SequentialRequest
        recommends 0 <= slot_index < C,
    {
        let slot = self.slots@[slot_index];
        let phase = match slot.state {
            RequestState::Vacant | RequestState::Ready => LifecyclePhase::Idle,
            RequestState::InFlight => if slot.active_epoch <= self.completed {
                LifecyclePhase::AwaitingKv
            } else {
                LifecyclePhase::Executing
            },
            RequestState::Retiring => if slot.active_epoch <= self.completed {
                LifecyclePhase::RetiringQuiescent
            } else {
                LifecyclePhase::RetiringExecuting
            },
        };
        SequentialRequest {
            state: slot.state,
            phase,
        }
    }

    pub closed spec fn slot_generation_spec(&self, slot_index: int) -> u32
        recommends 0 <= slot_index < C,
    {
        self.slots@[slot_index].generation
    }

    pub closed spec fn slot_is_live_spec(&self, slot_index: int) -> bool
        recommends 0 <= slot_index < C,
    {
        self.slots@[slot_index].state != RequestState::Vacant
    }

    pub open spec fn identity_frame(&self, before: &Self) -> bool {
        forall|slot_index: int| 0 <= slot_index < C ==> {
            &&& self.slot_generation_spec(slot_index)
                == before.slot_generation_spec(slot_index)
            &&& self.slot_is_live_spec(slot_index) == before.slot_is_live_spec(slot_index)
        }
    }

    pub(crate) open spec fn detachment_ready(
        &self,
        request: RequestId,
        origin: KvQuiescenceOrigin,
    ) -> bool {
        self.detachment_ready_inner(request, origin)
    }

    pub(crate) closed spec fn detachment_ready_inner(
        &self,
        request: RequestId,
        origin: KvQuiescenceOrigin,
    ) -> bool {
        let slot_index = request.slot_spec() as int;
        &&& slot_index < C
        &&& self.slot_generation_spec(slot_index) == request.generation_spec()
        &&& self.slot_model(slot_index) == SequentialRequest {
            state: RequestState::Retiring,
            phase: LifecyclePhase::RetiringQuiescent,
        }
        &&& !self.slots@[slot_index].in_reclaim_ring
        &&& match origin {
            KvQuiescenceOrigin::NeverSubmitted => {
                self.slots@[slot_index].active_epoch == NO_EPOCH
                    && self.slots@[slot_index].last_quiescent_epoch == NO_EPOCH
            }
            KvQuiescenceOrigin::CompletedExact { epoch } => {
                (self.slots@[slot_index].active_epoch != NO_EPOCH
                    && self.slots@[slot_index].active_epoch == epoch)
                    || (self.slots@[slot_index].active_epoch == NO_EPOCH
                        && self.slots@[slot_index].last_quiescent_epoch == epoch)
            }
        }
    }

    pub(crate) open spec fn detachment_ready_frame_except(
        &self,
        before: &Self,
        changed_slot: int,
    ) -> bool {
        forall|request: RequestId, origin: KvQuiescenceOrigin|
            request.slot_spec() < C && request.slot_spec() as int != changed_slot ==> {
                &&& self.state_spec(request) == before.state_spec(request)
                &&& #[trigger] self.detachment_ready(request, origin)
                    == before.detachment_ready(request, origin)
        }
    }

    pub(crate) proof fn apply_detachment_ready_frame_except(
        &self,
        before: &Self,
        changed_slot: int,
        request: RequestId,
        origin: KvQuiescenceOrigin,
    )
        requires
            self.detachment_ready_frame_except(before, changed_slot),
            request.slot_spec() < C,
            request.slot_spec() as int != changed_slot,
        ensures
            self.state_spec(request) == before.state_spec(request),
            self.detachment_ready(request, origin)
                == before.detachment_ready(request, origin),
    {
    }

    pub(crate) proof fn apply_detachment_ready_identity(
        &self,
        request: RequestId,
        origin: KvQuiescenceOrigin,
    )
        requires self.detachment_ready(request, origin),
        ensures
            request.slot_spec() < C,
            self.slot_generation_spec(request.slot_spec() as int)
                == request.generation_spec(),
    {
        reveal(Scheduler::detachment_ready_inner);
    }

    pub(crate) open spec fn detached_enabled(&self, detached: &KvDetachedRequest) -> bool {
        let request = detached.request_spec();
        &&& self.detachment_ready(request, detached.origin_spec())
        &&& self.slot_generation_spec(request.slot_spec() as int) < u32::MAX
    }

    pub closed spec fn slots_frame_except(&self, before: &Self, changed: int) -> bool {
        forall|slot_index: int| 0 <= slot_index < C && slot_index != changed ==>
            #[trigger] self.slots@[slot_index] == before.slots@[slot_index]
    }

    proof fn detachment_frame_from_slots_frame(&self, before: &Self, changed: int)
        requires
            self.slots_frame_except(before, changed),
            self.completed == before.completed,
        ensures self.detachment_ready_frame_except(before, changed),
    {
        assert forall|request: RequestId, origin: KvQuiescenceOrigin|
            request.slot_spec() < C && request.slot_spec() as int != changed implies {
                &&& self.state_spec(request) == before.state_spec(request)
                &&& self.detachment_ready(request, origin)
                    == before.detachment_ready(request, origin)
            } by {
                let slot_index = request.slot_spec() as int;
                assert(self.slots@[slot_index] == before.slots@[slot_index]);
                reveal(Scheduler::state_spec);
                reveal(Scheduler::slot_model);
                reveal(Scheduler::slot_generation_spec);
                reveal(Scheduler::detachment_ready_inner);
            }
    }

    proof fn identity_frame_from_slots_frame(&self, before: &Self, changed: int)
        requires
            0 <= changed < C,
            self.slots_frame_except(before, changed),
            self.slot_generation_spec(changed) == before.slot_generation_spec(changed),
            self.slot_is_live_spec(changed) == before.slot_is_live_spec(changed),
        ensures self.identity_frame(before),
    {
        assert forall|slot_index: int| 0 <= slot_index < C implies {
            &&& self.slot_generation_spec(slot_index)
                == before.slot_generation_spec(slot_index)
            &&& self.slot_is_live_spec(slot_index) == before.slot_is_live_spec(slot_index)
        } by {
            if slot_index != changed {
                assert(self.slots@[slot_index] == before.slots@[slot_index]);
                reveal(Scheduler::slot_generation_spec);
                reveal(Scheduler::slot_is_live_spec);
            }
        }
    }

    proof fn detached_refines_preserves_basic(
        &self,
        before: &Self,
        detached: &KvDetachedRequest,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.detached_refines(before, detached, &Ok(request)),
        ensures self.basic_invariant(),
    {
        self.detached_refines_preserves_scalar(before, detached, request);
        self.detached_refines_preserves_slots(before, detached, request);
        self.detached_refines_preserves_free_ring(before, detached, request);
        self.detached_refines_preserves_reclaim_ring(before, detached, request);
        self.detached_refines_preserves_member_ring(before, detached, request);
        self.detached_refines_preserves_batch_ring(before, detached, request);
        reveal(Scheduler::basic_invariant);
    }

    proof fn basic_implies_scalar(&self)
        requires self.basic_invariant(),
        ensures self.scalar_invariant(),
    {
        reveal(Scheduler::basic_invariant);
    }

    proof fn basic_implies_slots(&self)
        requires self.basic_invariant(),
        ensures self.slot_invariant(),
    {
        reveal(Scheduler::basic_invariant);
    }

    proof fn basic_implies_free_ring(&self)
        requires self.basic_invariant(),
        ensures self.free_ring_invariant(),
    {
        reveal(Scheduler::basic_invariant);
    }

    proof fn free_ring_entry_facts(&self, offset: int)
        requires
            self.free_ring_invariant(),
            0 <= offset < self.free_len,
        ensures {
            let slot_index = self.free_ring@[
                ring_position::<C>(self.free_head, offset as nat)
            ];
            &&& slot_index < C
            &&& self.slots@[slot_index as int].state == RequestState::Vacant
            &&& self.slots@[slot_index as int].in_free_ring
        },
    {
        reveal(Scheduler::free_ring_invariant);
    }

    proof fn basic_implies_reclaim_ring(&self)
        requires self.basic_invariant(),
        ensures self.reclaim_ring_invariant(),
    {
        reveal(Scheduler::basic_invariant);
    }

    proof fn reclaim_ring_entry_facts(&self, offset: int)
        requires
            self.reclaim_ring_invariant(),
            0 <= offset < self.reclaim_len,
        ensures {
            let slot_index = self.reclaim_ring@[
                ring_position::<C>(self.reclaim_head, offset as nat)
            ];
            &&& slot_index < C
            &&& self.slots@[slot_index as int].state == RequestState::Retiring
            &&& self.slots@[slot_index as int].active_epoch == NO_EPOCH
            &&& self.slots@[slot_index as int].in_reclaim_ring
        },
    {
        reveal(Scheduler::reclaim_ring_invariant);
    }

    proof fn reclaim_ring_membership_fact(&self, slot_index: int)
        requires
            self.reclaim_ring_invariant(),
            0 <= slot_index < C,
        ensures
            self.slots@[slot_index].in_reclaim_ring
                == usize_ring_contains::<C>(
                    self.reclaim_ring@,
                    self.reclaim_head,
                    self.reclaim_len,
                    slot_index,
                ),
    {
        reveal(Scheduler::reclaim_ring_invariant);
    }

    proof fn reclaim_ring_distinct_facts(&self)
        requires self.reclaim_ring_invariant(),
        ensures forall|left: int, right: int|
            0 <= left < self.reclaim_len
                && 0 <= right < self.reclaim_len
                && left != right ==>
                #[trigger] usize_ring_entries_differ::<C>(
                    self.reclaim_ring@,
                    self.reclaim_head,
                    left,
                    right,
                ),
    {
        reveal(Scheduler::reclaim_ring_invariant);
    }

    proof fn reclaim_ring_pop_facts(&self)
        requires self.basic_invariant(), self.reclaim_len > 0,
        ensures
            forall|slot_index: int| {
                #[trigger] usize_ring_contains::<C>(
                    self.reclaim_ring@,
                    next_position::<C>(self.reclaim_head),
                    ((self.reclaim_len as int) - 1) as usize,
                    slot_index,
                ) == (usize_ring_contains::<C>(
                    self.reclaim_ring@,
                    self.reclaim_head,
                    self.reclaim_len,
                    slot_index,
                ) && slot_index != self.reclaim_ring@[self.reclaim_head as int])
            },
            forall|left: int, right: int|
                0 <= left < self.reclaim_len - 1
                    && 0 <= right < self.reclaim_len - 1
                    && left != right ==>
                    #[trigger] usize_ring_entries_differ::<C>(
                        self.reclaim_ring@,
                        next_position::<C>(self.reclaim_head),
                        left,
                        right,
                    ),
            forall|offset: int| 0 <= offset < self.reclaim_len - 1 ==> {
                #[trigger] self.reclaim_ring@[
                    ring_position::<C>(next_position::<C>(self.reclaim_head), offset as nat)
                ] == self.reclaim_ring@[
                    ring_position::<C>(self.reclaim_head, (offset + 1) as nat)
                ]
            },
    {
        self.basic_implies_scalar();
        self.basic_implies_reclaim_ring();
        self.reclaim_ring_distinct_facts();
        reveal(Scheduler::scalar_invariant);
        usize_ring_pop_facts::<C>(self.reclaim_ring@, self.reclaim_head, self.reclaim_len);
    }

    proof fn reclaim_ring_pop_membership_fact(&self, slot_index: int)
        requires self.basic_invariant(), self.reclaim_len > 0,
        ensures
            usize_ring_contains::<C>(
                self.reclaim_ring@,
                next_position::<C>(self.reclaim_head),
                ((self.reclaim_len as int) - 1) as usize,
                slot_index,
            ) == (usize_ring_contains::<C>(
                self.reclaim_ring@,
                self.reclaim_head,
                self.reclaim_len,
                slot_index,
            ) && slot_index != self.reclaim_ring@[self.reclaim_head as int]),
    {
        self.reclaim_ring_pop_facts();
    }

    proof fn retiring_head_facts(&self)
        requires self.basic_invariant(), self.reclaim_len > 0,
        ensures {
            let slot_index = self.reclaim_ring@[self.reclaim_head as int];
            &&& slot_index < C
            &&& self.slots@[slot_index as int].state == RequestState::Retiring
            &&& self.slots@[slot_index as int].active_epoch == NO_EPOCH
            &&& self.slots@[slot_index as int].in_reclaim_ring
        },
    {
        self.basic_implies_scalar();
        self.basic_implies_reclaim_ring();
        self.reclaim_ring_entry_facts(0);
        assert(ring_position::<C>(self.reclaim_head, 0) == self.reclaim_head);
        reveal(Scheduler::scalar_invariant);
    }

    proof fn retiring_permit_updates_preserve_scalar(
        &self,
        before: &Self,
        slot_index: usize,
    )
        requires
            before.basic_invariant(),
            before.reclaim_len > 0,
            slot_index == before.reclaim_ring@[before.reclaim_head as int],
            self.retiring_permit_updates(before, slot_index),
        ensures self.scalar_invariant(),
    {
        before.basic_implies_scalar();
        before.retiring_head_facts();
        reveal(Scheduler::retiring_permit_updates);
        reveal(Scheduler::retiring_reclaim_updates);
        let replacement = self.slots@[slot_index as int];
        live_count_update_nonvacant(
            before.slots@,
            slot_index as int,
            replacement,
            C as nat,
        );
        nonreclaim_count_update_add(
            before.slots@,
            slot_index as int,
            replacement,
            C as nat,
        );
        reveal(Scheduler::scalar_invariant);
    }

    proof fn retiring_permit_updates_preserve_slots(
        &self,
        before: &Self,
        slot_index: usize,
    )
        requires
            before.basic_invariant(),
            before.reclaim_len > 0,
            slot_index == before.reclaim_ring@[before.reclaim_head as int],
            self.retiring_permit_updates(before, slot_index),
        ensures self.slot_invariant(),
    {
        before.basic_implies_slots();
        before.retiring_head_facts();
        reveal(Scheduler::retiring_permit_updates);
        reveal(Scheduler::retiring_reclaim_updates);
        reveal(Scheduler::slot_invariant);
    }

    proof fn retiring_permit_updates_preserve_free_ring(
        &self,
        before: &Self,
        slot_index: usize,
    )
        requires
            before.basic_invariant(),
            before.reclaim_len > 0,
            slot_index == before.reclaim_ring@[before.reclaim_head as int],
            self.retiring_permit_updates(before, slot_index),
        ensures self.free_ring_invariant(),
    {
        before.basic_implies_slots();
        before.basic_implies_free_ring();
        before.retiring_head_facts();
        reveal(Scheduler::retiring_permit_updates);
        reveal(Scheduler::retiring_reclaim_updates);
        reveal(Scheduler::slot_invariant);
        reveal(Scheduler::free_ring_invariant);
    }

    proof fn retiring_reclaim_entries(
        &self,
        before: &Self,
        slot_index: usize,
    )
        requires
            before.basic_invariant(),
            before.reclaim_len > 0,
            slot_index == before.reclaim_ring@[before.reclaim_head as int],
            self.retiring_reclaim_updates(before, slot_index),
        ensures forall|offset: int| 0 <= offset < self.reclaim_len ==> {
            let observed = #[trigger] self.reclaim_ring@[
                ring_position::<C>(self.reclaim_head, offset as nat)
            ];
            &&& observed < C
            &&& self.slots@[observed as int].state == RequestState::Retiring
            &&& self.slots@[observed as int].active_epoch == NO_EPOCH
            &&& self.slots@[observed as int].in_reclaim_ring
        },
    {
        before.basic_implies_scalar();
        before.basic_implies_reclaim_ring();
        before.retiring_head_facts();
        reveal(Scheduler::retiring_reclaim_updates);
        before.reclaim_ring_pop_facts();
        assert forall|offset: int| 0 <= offset < self.reclaim_len implies {
            let observed = #[trigger] self.reclaim_ring@[
                ring_position::<C>(self.reclaim_head, offset as nat)
            ];
            &&& observed < C
            &&& self.slots@[observed as int].state == RequestState::Retiring
            &&& self.slots@[observed as int].active_epoch == NO_EPOCH
            &&& self.slots@[observed as int].in_reclaim_ring
        } by {
            let old_offset = offset + 1;
            let observed = before.reclaim_ring@[
                ring_position::<C>(before.reclaim_head, old_offset as nat)
            ];
            assert(observed != slot_index) by {
                assert(usize_ring_entries_differ::<C>(
                    before.reclaim_ring@,
                    before.reclaim_head,
                    0,
                    old_offset,
                ));
                reveal(usize_ring_entries_differ);
                assert(ring_position::<C>(before.reclaim_head, 0) == before.reclaim_head);
            }
            assert(self.slots@[observed as int] == before.slots@[observed as int]);
            before.reclaim_ring_entry_facts(old_offset);
        }
    }

    proof fn retiring_reclaim_membership_at(
        &self,
        before: &Self,
        slot_index: usize,
        observed: int,
    )
        requires
            before.basic_invariant(),
            before.reclaim_len > 0,
            slot_index == before.reclaim_ring@[before.reclaim_head as int],
            self.retiring_reclaim_updates(before, slot_index),
            0 <= observed < C,
        ensures
            #[trigger] self.slots@[observed].in_reclaim_ring
                == usize_ring_contains::<C>(
                    self.reclaim_ring@,
                    self.reclaim_head,
                    self.reclaim_len,
                    observed,
                ),
    {
        before.basic_implies_scalar();
        before.basic_implies_reclaim_ring();
        before.retiring_head_facts();
        reveal(Scheduler::retiring_reclaim_updates);
        before.reclaim_ring_pop_membership_fact(observed);
        before.reclaim_ring_membership_fact(observed);
        if observed == slot_index {
            assert(!self.slots@[observed].in_reclaim_ring);
        } else {
            assert(self.slots@[observed] == before.slots@[observed]);
        }
    }

    proof fn retiring_reclaim_exact_membership(
        &self,
        before: &Self,
        slot_index: usize,
    )
        requires
            before.basic_invariant(),
            before.reclaim_len > 0,
            slot_index == before.reclaim_ring@[before.reclaim_head as int],
            self.retiring_reclaim_updates(before, slot_index),
        ensures forall|observed: int| 0 <= observed < C ==>
            #[trigger] self.slots@[observed].in_reclaim_ring
                == usize_ring_contains::<C>(
                    self.reclaim_ring@,
                    self.reclaim_head,
                    self.reclaim_len,
                    observed,
                ),
    {
        assert forall|observed: int| 0 <= observed < C implies
            #[trigger] self.slots@[observed].in_reclaim_ring
                == usize_ring_contains::<C>(
                    self.reclaim_ring@,
                    self.reclaim_head,
                    self.reclaim_len,
                    observed,
                ) by {
            self.retiring_reclaim_membership_at(before, slot_index, observed);
        }
    }

    proof fn retiring_reclaim_capacity(
        &self,
        before: &Self,
        slot_index: usize,
    )
        requires
            before.basic_invariant(),
            before.reclaim_len > 0,
            slot_index == before.reclaim_ring@[before.reclaim_head as int],
            self.retiring_reclaim_updates(before, slot_index),
        ensures forall|observed: int| 0 <= observed < C
            && !(#[trigger] self.slots@[observed].in_reclaim_ring) ==> self.reclaim_len < C,
    {
        before.basic_implies_scalar();
        reveal(Scheduler::retiring_reclaim_updates);
        reveal(Scheduler::scalar_invariant);
        assert forall|observed: int| 0 <= observed < C
            && !(#[trigger] self.slots@[observed].in_reclaim_ring) implies self.reclaim_len < C
        by {
            assert(self.reclaim_len < before.reclaim_len);
            assert(before.reclaim_len <= C);
        }
    }

    proof fn retiring_permit_updates_preserve_reclaim_ring(
        &self,
        before: &Self,
        slot_index: usize,
    )
        requires
            before.basic_invariant(),
            before.reclaim_len > 0,
            slot_index == before.reclaim_ring@[before.reclaim_head as int],
            self.retiring_permit_updates(before, slot_index),
        ensures self.reclaim_ring_invariant(),
    {
        before.basic_implies_scalar();
        before.basic_implies_reclaim_ring();
        reveal(Scheduler::retiring_permit_updates);
        assert(self.retiring_reclaim_updates(before, slot_index));
        reveal(Scheduler::retiring_reclaim_updates);
        before.reclaim_ring_pop_facts();
        self.retiring_reclaim_entries(before, slot_index);
        self.retiring_reclaim_exact_membership(before, slot_index);
        self.retiring_reclaim_capacity(before, slot_index);
        reveal(Scheduler::reclaim_ring_invariant);
    }

    proof fn retiring_permit_member_entry_preserved(
        &self,
        before: &Self,
        slot_index: usize,
        offset: int,
    )
        requires
            before.basic_invariant(),
            before.reclaim_len > 0,
            slot_index == before.reclaim_ring@[before.reclaim_head as int],
            self.retiring_permit_updates(before, slot_index),
            0 <= offset < self.member_len,
        ensures self.member_entry_valid(offset),
    {
        before.basic_implies_member_entries();
        before.retiring_head_facts();
        reveal(Scheduler::retiring_permit_updates);
        reveal(Scheduler::retiring_reclaim_updates);
        assert(before.member_entry_valid(offset)) by {
            reveal(Scheduler::member_entries_invariant);
        }
        let handle = before.member_ring@[
            ring_position::<C>(before.member_head, offset as nat)
        ];
        assert(handle.slot_spec() as int != slot_index as int) by {
            reveal(Scheduler::member_entry_valid);
        }
        assert(self.slots@[handle.slot_spec() as int]
            == before.slots@[handle.slot_spec() as int]);
        reveal(Scheduler::member_entry_valid);
    }

    proof fn retiring_permit_member_membership_at(
        &self,
        before: &Self,
        slot_index: usize,
        observed: int,
    )
        requires
            before.basic_invariant(),
            before.reclaim_len > 0,
            slot_index == before.reclaim_ring@[before.reclaim_head as int],
            self.retiring_permit_updates(before, slot_index),
            0 <= observed < C,
        ensures
            (((self.slots@[observed].state == RequestState::InFlight
                || self.slots@[observed].state == RequestState::Retiring)
                && self.completed < self.slots@[observed].active_epoch)
                == request_ring_contains_slot::<C>(
                    self.member_ring@,
                    self.member_head,
                    self.member_len,
                    observed,
                )),
    {
        before.basic_implies_member_ring();
        before.retiring_head_facts();
        reveal(Scheduler::member_ring_invariant);
        reveal(Scheduler::member_membership_invariant);
        reveal(Scheduler::retiring_permit_updates);
        reveal(Scheduler::retiring_reclaim_updates);
        if observed != slot_index as int {
            assert(self.slots@[observed] == before.slots@[observed]);
        }
    }

    proof fn retiring_permit_updates_preserve_member_ring(
        &self,
        before: &Self,
        slot_index: usize,
    )
        requires
            before.basic_invariant(),
            before.reclaim_len > 0,
            slot_index == before.reclaim_ring@[before.reclaim_head as int],
            self.retiring_permit_updates(before, slot_index),
        ensures self.member_ring_invariant(),
    {
        before.basic_implies_member_ring();
        reveal(Scheduler::retiring_permit_updates);
        assert forall|offset: int| 0 <= offset < self.member_len implies
            #[trigger] self.member_entry_valid(offset) by {
            self.retiring_permit_member_entry_preserved(
                before,
                slot_index,
                offset,
            );
        }
        assert(self.member_entries_invariant()) by {
            reveal(Scheduler::member_entries_invariant);
        }
        assert(self.member_distinct_invariant()) by {
            reveal(Scheduler::member_ring_invariant);
            reveal(Scheduler::member_distinct_invariant);
        }
        assert forall|observed: int| 0 <= observed < C implies
            (((#[trigger] self.slots@[observed].state == RequestState::InFlight
                || self.slots@[observed].state == RequestState::Retiring)
                && self.completed < self.slots@[observed].active_epoch)
                == request_ring_contains_slot::<C>(
                    self.member_ring@,
                    self.member_head,
                    self.member_len,
                    observed,
                )) by {
            self.retiring_permit_member_membership_at(
                before,
                slot_index,
                observed,
            );
        }
        assert(self.member_membership_invariant()) by {
            reveal(Scheduler::member_membership_invariant);
        }
        reveal(Scheduler::member_ring_invariant);
    }

    proof fn retiring_permit_updates_preserve_batch_ring(
        &self,
        before: &Self,
        slot_index: usize,
    )
        requires
            before.basic_invariant(),
            before.reclaim_len > 0,
            slot_index == before.reclaim_ring@[before.reclaim_head as int],
            self.retiring_permit_updates(before, slot_index),
        ensures self.batch_ring_invariant(),
    {
        before.basic_implies_batch_ring();
        before.basic_implies_member_ring();
        before.retiring_head_facts();
        reveal(Scheduler::retiring_permit_updates);
        reveal(Scheduler::member_ring_invariant);
        reveal(Scheduler::batch_ring_invariant);
    }

    proof fn retiring_permit_updates_preserve_basic(
        &self,
        before: &Self,
        slot_index: usize,
    )
        requires
            before.basic_invariant(),
            before.reclaim_len > 0,
            slot_index == before.reclaim_ring@[before.reclaim_head as int],
            self.retiring_permit_updates(before, slot_index),
        ensures self.basic_invariant(),
    {
        self.retiring_permit_updates_preserve_scalar(before, slot_index);
        self.retiring_permit_updates_preserve_slots(before, slot_index);
        self.retiring_permit_updates_preserve_free_ring(before, slot_index);
        self.retiring_permit_updates_preserve_reclaim_ring(before, slot_index);
        self.retiring_permit_updates_preserve_member_ring(before, slot_index);
        self.retiring_permit_updates_preserve_batch_ring(before, slot_index);
        reveal(Scheduler::basic_invariant);
    }

    proof fn retiring_permit_updates_establish_postconditions(
        &self,
        before: &Self,
        slot_index: usize,
        request: RequestId,
        origin: KvQuiescenceOrigin,
    )
        requires
            before.basic_invariant(),
            before.reclaim_len > 0,
            slot_index == before.reclaim_ring@[before.reclaim_head as int],
            self.retiring_permit_updates(before, slot_index),
            request.slot_spec() == slot_index,
            request.generation_spec() == before.slots@[slot_index as int].generation,
            origin == if before.slots@[slot_index as int].last_quiescent_epoch == NO_EPOCH {
                KvQuiescenceOrigin::NeverSubmitted
            } else {
                KvQuiescenceOrigin::CompletedExact {
                    epoch: before.slots@[slot_index as int].last_quiescent_epoch,
                }
            },
        ensures
            self.basic_invariant(),
            self.identity_frame(before),
            self.detachment_ready(request, origin),
            self.retiring_permit_refines(
                before,
                &Ok(Some(KvQuiescencePermit { request, origin })),
            ),
    {
        before.retiring_head_facts();
        self.retiring_permit_updates_preserve_basic(before, slot_index);
        reveal(Scheduler::retiring_permit_updates);
        reveal(Scheduler::retiring_reclaim_updates);
        assert(self.slots_frame_except(before, slot_index as int)) by {
            reveal(Scheduler::slots_frame_except);
        }
        self.identity_frame_from_slots_frame(before, slot_index as int);
        assert(self.detachment_ready(request, origin)) by {
            reveal(Scheduler::detachment_ready_inner);
            reveal(Scheduler::slot_model);
            reveal(Scheduler::slot_generation_spec);
        }
        reveal(Scheduler::retiring_permit_refines);
    }

    proof fn basic_implies_member_ring(&self)
        requires self.basic_invariant(),
        ensures self.member_ring_invariant(),
    {
        reveal(Scheduler::basic_invariant);
    }

    proof fn basic_implies_member_entries(&self)
        requires self.basic_invariant(),
        ensures self.member_entries_invariant(),
    {
        self.basic_implies_member_ring();
        reveal(Scheduler::member_ring_invariant);
    }

    proof fn member_slot_indices_are_distinct(&self)
        requires self.basic_invariant(),
        ensures {
            let members = member_slot_indices::<C>(
                self.member_ring@,
                self.member_head,
                self.member_len,
            );
            &&& members.len() == self.member_len
            &&& members.no_duplicates()
        },
    {
        self.basic_implies_scalar();
        self.basic_implies_member_ring();
        let members = member_slot_indices::<C>(
            self.member_ring@,
            self.member_head,
            self.member_len,
        );
        assert(members.len() == self.member_len) by {
            reveal(member_slot_indices);
        }
        assert(members.no_duplicates()) by {
            reveal(Seq::no_duplicates);
            assert forall|left: int, right: int|
                0 <= left < members.len()
                    && 0 <= right < members.len()
                    && left != right implies members[left] != members[right]
            by {
                reveal(member_slot_indices);
                assert(request_ring_slots_differ::<C>(
                    self.member_ring@,
                    self.member_head,
                    left,
                    right,
                )) by {
                    reveal(Scheduler::member_ring_invariant);
                }
                reveal(request_ring_slots_differ);
            }
        }
    }

    proof fn member_slot_index_is_nonvacant(&self, slot_index: int)
        requires
            self.basic_invariant(),
            member_slot_indices::<C>(
                self.member_ring@,
                self.member_head,
                self.member_len,
            ).contains(slot_index),
        ensures
            0 <= slot_index < C,
            self.slots@[slot_index].state != RequestState::Vacant,
    {
        self.basic_implies_scalar();
        self.basic_implies_member_ring();
        let members = member_slot_indices::<C>(
            self.member_ring@,
            self.member_head,
            self.member_len,
        );
        let offset = choose|offset: int| 0 <= offset < members.len()
            && members[offset] == slot_index;
        let handle = self.member_ring@[
            ring_position::<C>(self.member_head, offset as nat)
        ];
        assert(handle.slot_spec() as int == slot_index) by {
            reveal(member_slot_indices);
        }
        assert(self.member_entry_valid(offset)) by {
            reveal(Scheduler::member_ring_invariant);
            reveal(Scheduler::member_entries_invariant);
        }
        reveal(Scheduler::member_entry_valid);
    }

    proof fn member_slot_index_is_live(&self, slot_index: int)
        requires
            self.basic_invariant(),
            member_slot_indices::<C>(
                self.member_ring@,
                self.member_head,
                self.member_len,
            ).contains(slot_index),
        ensures
            live_slot_indices(self.slots@, C as nat).contains(slot_index),
    {
        self.member_slot_index_is_nonvacant(slot_index);
        live_slot_indices_facts(self.slots@, C as nat);
    }

    proof fn member_slot_indices_are_live(&self)
        requires self.basic_invariant(),
        ensures {
            let members = member_slot_indices::<C>(
                self.member_ring@,
                self.member_head,
                self.member_len,
            );
            let live = live_slot_indices(self.slots@, C as nat);
            forall|slot_index: int| members.contains(slot_index) ==>
                #[trigger] live.contains(slot_index)
        },
    {
        let members = member_slot_indices::<C>(
            self.member_ring@,
            self.member_head,
            self.member_len,
        );
        let live = live_slot_indices(self.slots@, C as nat);
        assert forall|slot_index: int| members.contains(slot_index) implies
            #[trigger] live.contains(slot_index) by {
            self.member_slot_index_is_live(slot_index);
        }
    }

    proof fn nonexecuting_live_slot_gives_member_slack(&self, changed: int)
        requires
            self.basic_invariant(),
            0 <= changed < C,
            self.slots@[changed].state != RequestState::Vacant,
            !((self.slots@[changed].state == RequestState::InFlight
                || self.slots@[changed].state == RequestState::Retiring)
                && self.completed < self.slots@[changed].active_epoch),
        ensures self.member_len < self.live_count,
    {
        self.basic_implies_scalar();
        self.member_slot_indices_are_distinct();
        self.member_slot_indices_are_live();
        let members = member_slot_indices::<C>(
            self.member_ring@,
            self.member_head,
            self.member_len,
        );
        let live = live_slot_indices(self.slots@, C as nat);
        live_slot_indices_facts(self.slots@, C as nat);
        member_slot_indices_contains_iff::<C>(
            self.member_ring@,
            self.member_head,
            self.member_len,
            changed,
        );
        assert(!members.contains(changed)) by {
            self.basic_implies_member_ring();
            reveal(Scheduler::member_ring_invariant);
        }
        assert(live.contains(changed));
        members.to_set_ensures();
        live.to_set_ensures();
        members.unique_seq_to_set();
        live.unique_seq_to_set();
        assert(members.to_set().subset_of(live.to_set())) by {
            assert forall|slot_index: int| members.to_set().contains(slot_index) implies
                #[trigger] live.to_set().contains(slot_index) by {
                assert(members.contains(slot_index));
                assert(live.contains(slot_index));
            }
        }
        assert(!members.to_set().contains(changed));
        assert(live.to_set().contains(changed));
        members.to_set().lemma_subset_not_in_lt(live.to_set(), changed);
        reveal(Scheduler::scalar_invariant);
    }

    proof fn basic_implies_batch_ring(&self)
        requires self.basic_invariant(),
        ensures self.batch_ring_invariant(),
    {
        reveal(Scheduler::basic_invariant);
    }

    proof fn basic_implies_batch_entry(&self, batch_offset: int)
        requires
            self.basic_invariant(),
            0 <= batch_offset < self.batch_len,
        ensures {
            let batch = self.batch_ring@[
                ring_position::<C>(self.batch_head, batch_offset as nat)
            ];
            &&& batch.member_count > 0
            &&& batch.epoch.value as int == self.completed as int + batch_offset + 1
            &&& batch.epoch.value <= self.submitted
            &&& (forall|member_offset: int|
                batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    batch_offset as nat,
                ) <= member_offset < batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    batch_offset as nat + 1,
                ) ==> {
                    let handle = #[trigger] self.member_ring@[
                        ring_position::<C>(self.member_head, member_offset as nat)
                    ];
                    self.slots@[handle.slot_spec() as int].active_epoch == batch.epoch.value
                })
        },
    {
        self.basic_implies_batch_ring();
        reveal(Scheduler::batch_ring_invariant);
    }

    proof fn basic_batch_entry_header_facts(&self, batch_offset: int)
        requires
            self.basic_invariant(),
            0 <= batch_offset < self.batch_len,
        ensures {
            let batch = self.batch_ring@[
                ring_position::<C>(self.batch_head, batch_offset as nat)
            ];
            &&& batch.member_count > 0
            &&& batch.epoch.value as int == self.completed as int + batch_offset + 1
            &&& batch.epoch.value <= self.submitted
        },
    {
        self.basic_implies_batch_entry(batch_offset);
    }

    proof fn basic_batch_member_epoch_fact(
        &self,
        batch_offset: int,
        member_offset: int,
    )
        requires
            self.basic_invariant(),
            0 <= batch_offset < self.batch_len,
            batch_member_sum::<C>(
                self.batch_ring@,
                self.batch_head,
                batch_offset as nat,
            ) <= member_offset < batch_member_sum::<C>(
                self.batch_ring@,
                self.batch_head,
                batch_offset as nat + 1,
            ),
        ensures {
            let batch = self.batch_ring@[
                ring_position::<C>(self.batch_head, batch_offset as nat)
            ];
            let handle = self.member_ring@[
                ring_position::<C>(self.member_head, member_offset as nat)
            ];
            self.slots@[handle.slot_spec() as int].active_epoch == batch.epoch.value
        },
    {
        self.basic_implies_batch_entry(batch_offset);
    }

    proof fn admit_refines_preserves_scalar(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.admitted_slot_refines(before, request),
            self.admitted_fields_refine(before),
        ensures self.scalar_invariant(),
    {
        before.basic_implies_scalar();
        reveal(Scheduler::scalar_invariant);
        reveal(Scheduler::admitted_slot_refines);
        reveal(Scheduler::admitted_fields_refine);
        let slot_index = request.slot_spec() as int;
        assert(self.slots@ == before.slots@.update(slot_index, self.slots@[slot_index])) by {
            assert forall|index: int| 0 <= index < C implies
                self.slots@[index]
                    == before.slots@.update(slot_index, self.slots@[slot_index])[index]
            by {
                if index != slot_index {
                    assert(self.slots@[index] == before.slots@[index]);
                }
            }
        }
        live_count_update_admit(
            before.slots@,
            slot_index,
            self.slots@[slot_index],
            C as nat,
        );
        nonreclaim_count_update_add(
            before.slots@,
            slot_index,
            self.slots@[slot_index],
            C as nat,
        );
    }

    proof fn admit_refines_preserves_slots(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.admitted_slot_refines(before, request),
            self.admitted_fields_refine(before),
        ensures self.slot_invariant(),
    {
        before.basic_implies_slots();
        reveal(Scheduler::slot_invariant);
        reveal(Scheduler::admitted_slot_refines);
        reveal(Scheduler::admitted_fields_refine);
        reveal(Scheduler::slot_model);
        reveal(ferric_spec::scheduling::request_transition);
        let slot_index = request.slot_spec() as int;
        assert forall|index: int| 0 <= index < C implies {
            let slot = #[trigger] self.slots@[index];
            match slot.state {
                RequestState::Vacant => {
                    &&& slot.active_epoch == NO_EPOCH
                    &&& slot.last_quiescent_epoch == NO_EPOCH
                    &&& slot.in_free_ring
                    &&& !slot.in_reclaim_ring
                }
                RequestState::Ready => {
                    &&& slot.active_epoch == NO_EPOCH
                    &&& slot.last_quiescent_epoch <= self.completed
                    &&& !slot.in_free_ring
                    &&& !slot.in_reclaim_ring
                }
                RequestState::InFlight => {
                    &&& NO_EPOCH < slot.active_epoch <= self.submitted
                    &&& slot.last_quiescent_epoch <= self.completed
                    &&& !slot.in_free_ring
                    &&& !slot.in_reclaim_ring
                }
                RequestState::Retiring => {
                    &&& !slot.in_free_ring
                    &&& slot.active_epoch <= self.submitted
                    &&& slot.last_quiescent_epoch <= self.completed
                }
            }
        } by {
            if index != slot_index {
                assert(self.slots@[index] == before.slots@[index]);
            }
        }
    }

    proof fn admitted_free_pop_facts(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.admitted_slot_refines(before, request),
            self.admitted_fields_refine(before),
        ensures
            forall|index: int| {
                #[trigger] usize_ring_contains::<C>(
                    self.free_ring@,
                    self.free_head,
                    self.free_len,
                    index,
                ) == (usize_ring_contains::<C>(
                    before.free_ring@,
                    before.free_head,
                    before.free_len,
                    index,
                ) && index != request.slot_spec())
            },
            forall|left: int, right: int|
                0 <= left < self.free_len && 0 <= right < self.free_len && left != right ==>
                    #[trigger] usize_ring_entries_differ::<C>(
                        self.free_ring@,
                        self.free_head,
                        left,
                        right,
                    ),
            forall|offset: int| 0 <= offset < self.free_len ==>
                #[trigger] self.free_ring@[
                    ring_position::<C>(self.free_head, offset as nat)
                ] == before.free_ring@[
                    ring_position::<C>(before.free_head, (offset + 1) as nat)
                ],
    {
        before.basic_implies_free_ring();
        reveal(Scheduler::free_ring_invariant);
        reveal(Scheduler::admitted_slot_refines);
        reveal(Scheduler::admitted_fields_refine);
        usize_ring_pop_facts::<C>(
            before.free_ring@,
            before.free_head,
            before.free_len,
        );
        assert(self.free_len == ((before.free_len as int) - 1) as usize);
        assert(request.slot_spec() as int
            == before.free_ring@[before.free_head as int] as int);
        assert forall|index: int| usize_ring_contains::<C>(
            self.free_ring@,
            self.free_head,
            self.free_len,
            index,
        ) == (usize_ring_contains::<C>(
            before.free_ring@,
            before.free_head,
            before.free_len,
            index,
        ) && index != request.slot_spec()) by {}
    }

    proof fn admitted_free_entries_are_vacant(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.admitted_slot_refines(before, request),
            self.admitted_fields_refine(before),
        ensures
            forall|offset: int| 0 <= offset < self.free_len ==> {
                let observed = #[trigger] self.free_ring@[
                    ring_position::<C>(self.free_head, offset as nat)
                ];
                &&& observed < C
                &&& self.slots@[observed as int].state == RequestState::Vacant
                &&& self.slots@[observed as int].in_free_ring
            },
    {
        before.basic_implies_free_ring();
        self.admitted_free_pop_facts(before, request);
        reveal(Scheduler::free_ring_invariant);
        reveal(Scheduler::admitted_slot_refines);
        let removed = request.slot_spec() as int;
        assert forall|offset: int| 0 <= offset < self.free_len implies {
            let observed = #[trigger] self.free_ring@[
                ring_position::<C>(self.free_head, offset as nat)
            ];
            &&& (#[trigger] self.free_ring@[
                ring_position::<C>(self.free_head, offset as nat)
            ]) < C
            &&& self.slots@[observed as int].state == RequestState::Vacant
            &&& self.slots@[observed as int].in_free_ring
        } by {
            let observed = self.free_ring@[
                ring_position::<C>(self.free_head, offset as nat)
            ];
            assert(usize_ring_contains::<C>(
                self.free_ring@,
                self.free_head,
                self.free_len,
                observed as int,
            )) by {
                reveal(usize_ring_contains);
                assert(exists|witness: int| 0 <= witness < self.free_len
                    && #[trigger] self.free_ring@[
                        ring_position::<C>(self.free_head, witness as nat)
                    ] == observed) by {
                    let witness = offset;
                }
            }
            assert(observed as int != removed);
            assert(self.slots@[observed as int] == before.slots@[observed as int]);
        }
    }

    proof fn admitted_free_exact_membership(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.admitted_slot_refines(before, request),
            self.admitted_fields_refine(before),
        ensures
            forall|index: int| 0 <= index < C ==>
                #[trigger] self.slots@[index].in_free_ring
                    == usize_ring_contains::<C>(
                        self.free_ring@,
                        self.free_head,
                        self.free_len,
                        index,
                    ),
    {
        before.basic_implies_free_ring();
        self.admitted_free_pop_facts(before, request);
        reveal(Scheduler::free_ring_invariant);
        reveal(Scheduler::admitted_slot_refines);
        let removed = request.slot_spec() as int;
        assert forall|index: int| 0 <= index < C implies
            #[trigger] self.slots@[index].in_free_ring
                == usize_ring_contains::<C>(
                    self.free_ring@,
                    self.free_head,
                    self.free_len,
                    index,
                )
        by {
            if index != removed {
                assert(self.slots@[index] == before.slots@[index]);
            }
        }
    }

    proof fn admitted_free_has_capacity(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.admitted_slot_refines(before, request),
            self.admitted_fields_refine(before),
        ensures
            forall|index: int| 0 <= index < C
                && !(#[trigger] self.slots@[index].in_free_ring) ==> self.free_len < C,
    {
        before.basic_implies_scalar();
        reveal(Scheduler::scalar_invariant);
        reveal(Scheduler::admitted_fields_refine);
    }

    proof fn admit_refines_preserves_free_ring(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.admitted_slot_refines(before, request),
            self.admitted_fields_refine(before),
        ensures self.free_ring_invariant(),
    {
        self.admitted_free_pop_facts(before, request);
        self.admitted_free_entries_are_vacant(before, request);
        self.admitted_free_exact_membership(before, request);
        self.admitted_free_has_capacity(before, request);
        reveal(Scheduler::free_ring_invariant);
    }

    proof fn admit_refines_preserves_reclaim_ring(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.admitted_slot_refines(before, request),
            self.admitted_fields_refine(before),
        ensures self.reclaim_ring_invariant(),
    {
        before.basic_implies_reclaim_ring();
        reveal(Scheduler::reclaim_ring_invariant);
        reveal(Scheduler::admitted_slot_refines);
        reveal(Scheduler::admitted_fields_refine);
        let slot_index = request.slot_spec() as int;
        assert forall|offset: int| 0 <= offset < self.reclaim_len implies {
            let observed = #[trigger] self.reclaim_ring@[
                ring_position::<C>(self.reclaim_head, offset as nat)
            ];
            &&& observed < C
            &&& self.slots@[observed as int].state == RequestState::Retiring
            &&& self.slots@[observed as int].active_epoch == NO_EPOCH
            &&& self.slots@[observed as int].in_reclaim_ring
        } by {
            let observed = self.reclaim_ring@[
                ring_position::<C>(self.reclaim_head, offset as nat)
            ];
            assert(observed as int != slot_index);
            assert(self.slots@[observed as int] == before.slots@[observed as int]);
        }
        assert forall|index: int| 0 <= index < C implies
            #[trigger] self.slots@[index].in_reclaim_ring
                == usize_ring_contains::<C>(
                    self.reclaim_ring@,
                    self.reclaim_head,
                    self.reclaim_len,
                    index,
                )
        by {
            if index != slot_index {
                assert(self.slots@[index] == before.slots@[index]);
            }
        }
    }

    proof fn admitted_member_entry_preserved(
        &self,
        before: &Self,
        request: RequestId,
        offset: int,
    )
        requires
            before.basic_invariant(),
            self.admitted_slot_refines(before, request),
            self.admitted_fields_refine(before),
            0 <= offset < self.member_len,
        ensures self.member_entry_valid(offset),
    {
        reveal(Scheduler::admitted_fields_refine);
        before.basic_implies_member_entries();
        assert(before.member_entry_valid(offset)) by {
            reveal(Scheduler::member_entries_invariant);
        }
        self.admitted_member_slot_unchanged(before, request, offset);
        reveal(Scheduler::member_entry_valid);
    }

    proof fn admitted_member_membership_at(
        &self,
        before: &Self,
        request: RequestId,
        index: int,
    )
        requires
            before.basic_invariant(),
            self.admitted_slot_refines(before, request),
            self.admitted_fields_refine(before),
            0 <= index < C,
        ensures
            (((self.slots@[index].state == RequestState::InFlight
                || self.slots@[index].state == RequestState::Retiring)
                && self.completed < self.slots@[index].active_epoch)
                == request_ring_contains_slot::<C>(
                    self.member_ring@,
                    self.member_head,
                    self.member_len,
                    index,
                )),
    {
        before.basic_implies_member_ring();
        reveal(Scheduler::member_ring_invariant);
        reveal(Scheduler::member_membership_invariant);
        reveal(Scheduler::admitted_slot_refines);
        reveal(Scheduler::admitted_fields_refine);
        let changed = request.slot_spec() as int;
        if index != changed {
            assert(self.slots@[index] == before.slots@[index]);
        }
    }

    proof fn admit_refines_preserves_member_ring(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.admitted_slot_refines(before, request),
            self.admitted_fields_refine(before),
        ensures self.member_ring_invariant(),
    {
        before.basic_implies_member_ring();
        reveal(Scheduler::admitted_fields_refine);
        assert forall|offset: int| 0 <= offset < self.member_len implies
            #[trigger] self.member_entry_valid(offset) by {
            self.admitted_member_entry_preserved(before, request, offset);
        }
        assert(self.member_entries_invariant()) by {
            reveal(Scheduler::member_entries_invariant);
        }
        assert(self.member_distinct_invariant()) by {
            reveal(Scheduler::member_ring_invariant);
            reveal(Scheduler::member_distinct_invariant);
        }
        assert forall|index: int| 0 <= index < C implies
            (((#[trigger] self.slots@[index].state == RequestState::InFlight
                || self.slots@[index].state == RequestState::Retiring)
                && self.completed < self.slots@[index].active_epoch)
                == request_ring_contains_slot::<C>(
                    self.member_ring@,
                    self.member_head,
                    self.member_len,
                    index,
                ))
        by {
            self.admitted_member_membership_at(before, request, index);
        }
        assert(self.member_membership_invariant()) by {
            reveal(Scheduler::member_membership_invariant);
        }
        reveal(Scheduler::member_ring_invariant);
    }

    proof fn admitted_member_slot_unchanged(
        &self,
        before: &Self,
        request: RequestId,
        offset: int,
    )
        requires
            before.basic_invariant(),
            self.admitted_slot_refines(before, request),
            self.admitted_fields_refine(before),
            0 <= offset < before.member_len,
        ensures {
            let handle = before.member_ring@[
                ring_position::<C>(before.member_head, offset as nat)
            ];
            &&& handle.slot_spec() as int != request.slot_spec() as int
            &&& self.slots@[handle.slot_spec() as int]
                == before.slots@[handle.slot_spec() as int]
        },
    {
        before.member_entry_facts(offset as usize);
        reveal(Scheduler::admitted_slot_refines);
        let handle = before.member_ring@[
            ring_position::<C>(before.member_head, offset as nat)
        ];
        assert(handle.slot_spec() as int != request.slot_spec() as int);
        reveal(Scheduler::slots_frame_except);
    }

    proof fn admitted_batch_sum_preserved(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.admitted_slot_refines(before, request),
            self.admitted_fields_refine(before),
        ensures
            batch_member_sum::<C>(self.batch_ring@, self.batch_head, self.batch_len as nat)
                == self.member_len,
    {
        before.basic_implies_batch_ring();
        reveal(Scheduler::batch_ring_invariant);
        reveal(Scheduler::admitted_fields_refine);
    }

    proof fn admitted_batch_member_epoch_preserved(
        &self,
        before: &Self,
        batch_offset: int,
        member_offset: int,
    )
        requires
            before.basic_invariant(),
            self.member_ring@ == before.member_ring@,
            self.member_head == before.member_head,
            self.member_len == before.member_len,
            self.batch_ring@ == before.batch_ring@,
            self.batch_head == before.batch_head,
            self.batch_len == before.batch_len,
            0 <= batch_offset < self.batch_len,
            batch_member_sum::<C>(
                self.batch_ring@,
                self.batch_head,
                batch_offset as nat,
            ) <= member_offset < batch_member_sum::<C>(
                self.batch_ring@,
                self.batch_head,
                batch_offset as nat + 1,
            ),
            self.slots@[
                self.member_ring@[
                    ring_position::<C>(self.member_head, member_offset as nat)
                ].slot_spec() as int
            ] == before.slots@[
                before.member_ring@[
                    ring_position::<C>(before.member_head, member_offset as nat)
                ].slot_spec() as int
            ],
        ensures {
            let batch = self.batch_ring@[
                ring_position::<C>(self.batch_head, batch_offset as nat)
            ];
            let handle = self.member_ring@[
                ring_position::<C>(self.member_head, member_offset as nat)
            ];
            self.slots@[handle.slot_spec() as int].active_epoch == batch.epoch.value
        },
    {
        before.basic_batch_member_epoch_fact(batch_offset, member_offset);
    }

    proof fn admitted_batch_header_preserved(
        &self,
        before: &Self,
        request: RequestId,
        batch_offset: int,
    )
        requires
            before.basic_invariant(),
            self.admitted_slot_refines(before, request),
            self.admitted_fields_refine(before),
            0 <= batch_offset < self.batch_len,
        ensures {
            let batch = self.batch_ring@[
                ring_position::<C>(self.batch_head, batch_offset as nat)
            ];
            &&& batch.member_count > 0
            &&& batch.epoch.value as int == self.completed as int + batch_offset + 1
            &&& batch.epoch.value <= self.submitted
        },
    {
        before.basic_batch_entry_header_facts(batch_offset);
        reveal(Scheduler::admitted_fields_refine);
    }

    proof fn admitted_batch_member_preserved(
        &self,
        before: &Self,
        request: RequestId,
        batch_offset: int,
        member_offset: int,
    )
        requires
            before.basic_invariant(),
            self.admitted_slot_refines(before, request),
            self.admitted_fields_refine(before),
            0 <= batch_offset < self.batch_len,
            batch_member_sum::<C>(
                self.batch_ring@,
                self.batch_head,
                batch_offset as nat,
            ) <= member_offset < batch_member_sum::<C>(
                self.batch_ring@,
                self.batch_head,
                batch_offset as nat + 1,
            ),
        ensures {
            let batch = self.batch_ring@[
                ring_position::<C>(self.batch_head, batch_offset as nat)
            ];
            let handle = self.member_ring@[
                ring_position::<C>(self.member_head, member_offset as nat)
            ];
            self.slots@[handle.slot_spec() as int].active_epoch == batch.epoch.value
        },
    {
        reveal(Scheduler::admitted_fields_refine);
        before.basic_implies_batch_ring();
        reveal(Scheduler::batch_ring_invariant);
        batch_member_sum_monotonic::<C>(
            before.batch_ring@,
            before.batch_head,
            batch_offset as nat + 1,
            before.batch_len as nat,
        );
        self.admitted_member_slot_unchanged(before, request, member_offset);
        self.admitted_batch_member_epoch_preserved(before, batch_offset, member_offset);
    }

    proof fn admitted_batch_epoch_members_preserved(
        &self,
        before: &Self,
        request: RequestId,
        batch_offset: int,
    )
        requires
            before.basic_invariant(),
            self.admitted_slot_refines(before, request),
            self.admitted_fields_refine(before),
            0 <= batch_offset < self.batch_len,
        ensures {
            let batch = self.batch_ring@[
                ring_position::<C>(self.batch_head, batch_offset as nat)
            ];
            forall|member_offset: int|
                batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    batch_offset as nat,
                ) <= member_offset < batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    batch_offset as nat + 1,
                ) ==> {
                    let handle = #[trigger] self.member_ring@[
                        ring_position::<C>(self.member_head, member_offset as nat)
                    ];
                    self.slots@[handle.slot_spec() as int].active_epoch == batch.epoch.value
                }
        },
    {
        reveal(Scheduler::admitted_fields_refine);
        assert forall|member_offset: int|
            batch_member_sum::<C>(
                self.batch_ring@,
                self.batch_head,
                batch_offset as nat,
            ) <= member_offset < batch_member_sum::<C>(
                self.batch_ring@,
                self.batch_head,
                batch_offset as nat + 1,
            ) implies {
                let handle = #[trigger] self.member_ring@[
                    ring_position::<C>(self.member_head, member_offset as nat)
                ];
                self.slots@[handle.slot_spec() as int].active_epoch
                    == self.batch_ring@[
                        ring_position::<C>(self.batch_head, batch_offset as nat)
                    ].epoch.value
        } by {
            self.admitted_batch_member_preserved(
                before,
                request,
                batch_offset,
                member_offset,
            );
        }
    }

    proof fn admitted_batch_entry_preserved(
        &self,
        before: &Self,
        request: RequestId,
        batch_offset: int,
    )
        requires
            before.basic_invariant(),
            self.admitted_slot_refines(before, request),
            self.admitted_fields_refine(before),
            0 <= batch_offset < self.batch_len,
        ensures {
            let batch = self.batch_ring@[
                ring_position::<C>(self.batch_head, batch_offset as nat)
            ];
            &&& batch.member_count > 0
            &&& batch.epoch.value as int == self.completed as int + batch_offset + 1
            &&& batch.epoch.value <= self.submitted
            &&& (forall|member_offset: int|
                batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    batch_offset as nat,
                ) <= member_offset < batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    batch_offset as nat + 1,
                ) ==> {
                    let handle = #[trigger] self.member_ring@[
                        ring_position::<C>(self.member_head, member_offset as nat)
                    ];
                    self.slots@[handle.slot_spec() as int].active_epoch == batch.epoch.value
                })
        },
    {
        self.admitted_batch_header_preserved(before, request, batch_offset);
        self.admitted_batch_epoch_members_preserved(before, request, batch_offset);
    }

    proof fn admitted_batch_entries_preserved(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.admitted_slot_refines(before, request),
            self.admitted_fields_refine(before),
        ensures
            forall|batch_offset: int| 0 <= batch_offset < self.batch_len ==> {
                let batch = #[trigger] self.batch_ring@[
                    ring_position::<C>(self.batch_head, batch_offset as nat)
                ];
                &&& batch.member_count > 0
                &&& batch.epoch.value as int == self.completed as int + batch_offset + 1
                &&& batch.epoch.value <= self.submitted
                &&& (forall|member_offset: int|
                    batch_member_sum::<C>(
                        self.batch_ring@,
                        self.batch_head,
                        batch_offset as nat,
                    ) <= member_offset < batch_member_sum::<C>(
                        self.batch_ring@,
                        self.batch_head,
                        batch_offset as nat + 1,
                    ) ==> {
                        let handle = #[trigger] self.member_ring@[
                            ring_position::<C>(self.member_head, member_offset as nat)
                        ];
                        self.slots@[handle.slot_spec() as int].active_epoch == batch.epoch.value
                    })
            },
    {
        assert forall|batch_offset: int| 0 <= batch_offset < self.batch_len implies {
            let batch = #[trigger] self.batch_ring@[
                ring_position::<C>(self.batch_head, batch_offset as nat)
            ];
            &&& batch.member_count > 0
            &&& batch.epoch.value as int == self.completed as int + batch_offset + 1
            &&& batch.epoch.value <= self.submitted
            &&& (forall|member_offset: int|
                batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    batch_offset as nat,
                ) <= member_offset < batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    batch_offset as nat + 1,
                ) ==> {
                    let handle = #[trigger] self.member_ring@[
                        ring_position::<C>(self.member_head, member_offset as nat)
                    ];
                    self.slots@[handle.slot_spec() as int].active_epoch == batch.epoch.value
                })
        } by {
            self.admitted_batch_entry_preserved(before, request, batch_offset);
        }
    }

    proof fn admit_refines_preserves_batch_ring(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.admitted_slot_refines(before, request),
            self.admitted_fields_refine(before),
        ensures self.batch_ring_invariant(),
    {
        self.admitted_batch_sum_preserved(before, request);
        self.admitted_batch_entries_preserved(before, request);
        reveal(Scheduler::batch_ring_invariant);
    }

    proof fn admit_refines_preserves_other_rings(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.admitted_slot_refines(before, request),
            self.admitted_fields_refine(before),
        ensures
            self.reclaim_ring_invariant(),
            self.member_ring_invariant(),
            self.batch_ring_invariant(),
    {
        self.admit_refines_preserves_reclaim_ring(before, request);
        self.admit_refines_preserves_member_ring(before, request);
        self.admit_refines_preserves_batch_ring(before, request);
    }

    proof fn admit_refines_preserves_basic(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.admitted_slot_refines(before, request),
            self.admitted_fields_refine(before),
        ensures self.basic_invariant(),
    {
        self.admit_refines_preserves_scalar(before, request);
        self.admit_refines_preserves_slots(before, request);
        self.admit_refines_preserves_free_ring(before, request);
        self.admit_refines_preserves_other_rings(before, request);
        reveal(Scheduler::basic_invariant);
    }

    proof fn admitted_step_establishes_postconditions(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.admitted_slot_refines(before, request),
            self.admitted_fields_refine(before),
        ensures
            self.basic_invariant(),
            self.admit_refines(before, &Ok(request)),
            self.slot_is_live_spec(request.slot_spec() as int),
            self.slot_generation_spec(request.slot_spec() as int)
                == before.slot_generation_spec(request.slot_spec() as int),
            forall|other: int|
                0 <= other < C && other != request.slot_spec() as int ==> {
                    &&& self.slot_is_live_spec(other) == before.slot_is_live_spec(other)
                    &&& self.slot_generation_spec(other)
                        == before.slot_generation_spec(other)
                },
    {
        self.admitted_step_implies_refines(before, request);
        self.admit_refines_preserves_basic(before, request);
        reveal(Scheduler::admit_refines);
        reveal(Scheduler::slot_is_live_spec);
        reveal(Scheduler::slot_generation_spec);
        reveal(Scheduler::slots_frame_except);
        let slot_index = request.slot_spec() as int;
        assert forall|other: int| 0 <= other < C && other != slot_index implies {
            &&& self.slot_is_live_spec(other) == before.slot_is_live_spec(other)
            &&& self.slot_generation_spec(other) == before.slot_generation_spec(other)
        } by {
            assert(self.slots@[other] == before.slots@[other]);
        }
    }

    proof fn admit_error_establishes_postconditions(&self)
        requires
            self.basic_invariant(),
            self.free_len == 0,
        ensures
            self.basic_invariant(),
            self.admit_refines(self, &Err(SchedulerError::OutOfSlots)),
            self.identity_frame(self),
    {
        self.same_scalars_reflexive();
        reveal(Scheduler::admit_refines);
        reveal(Scheduler::identity_frame);
    }

    proof fn admit_scalar_preflight(&self)
        requires
            self.basic_invariant(),
            self.free_len > 0,
        ensures
            C > 0,
            C <= MAX_REQUEST_SLOTS,
            self.free_head < C,
            self.free_len <= C,
            self.live_count < C,
    {
        self.basic_implies_scalar();
        reveal(Scheduler::scalar_invariant);
    }

    proof fn admit_head_preflight(&self)
        requires
            self.basic_invariant(),
            self.free_len > 0,
        ensures
            self.free_ring@[self.free_head as int] < C,
            self.slots@[self.free_ring@[self.free_head as int] as int].state
                == RequestState::Vacant,
            self.slots@[self.free_ring@[self.free_head as int] as int].in_free_ring,
    {
        self.basic_implies_free_ring();
        self.free_ring_entry_facts(0);
        assert(ring_position::<C>(self.free_head, 0) == self.free_head);
    }

    proof fn pending_batch_facts(&self)
        requires
            self.basic_invariant(),
            self.batch_len > 0,
        ensures
            self.batch_ring@[self.batch_head as int].member_count > 0,
            self.batch_ring@[self.batch_head as int].member_count <= self.member_len,
            self.batch_ring@[self.batch_head as int].epoch.value == self.completed + 1,
            forall|offset: int|
                0 <= offset < self.batch_ring@[self.batch_head as int].member_count ==> {
                    let handle = #[trigger] self.member_ring@[
                        ring_position::<C>(self.member_head, offset as nat)
                    ];
                    &&& handle.slot_spec() < C
                    &&& self.slots@[handle.slot_spec() as int].generation
                        == handle.generation_spec()
                    &&& self.slots@[handle.slot_spec() as int].active_epoch
                        == self.batch_ring@[self.batch_head as int].epoch.value
                    &&& !self.slots@[handle.slot_spec() as int].in_reclaim_ring
                    &&& (self.slots@[handle.slot_spec() as int].state
                        == RequestState::InFlight
                        || self.slots@[handle.slot_spec() as int].state
                            == RequestState::Retiring)
                },
            forall|left: int, right: int|
                0 <= left < self.batch_ring@[self.batch_head as int].member_count
                    && 0 <= right < self.batch_ring@[self.batch_head as int].member_count
                    && left != right ==>
                        #[trigger] request_ring_slots_differ::<C>(
                            self.member_ring@,
                            self.member_head,
                            left,
                            right,
                        ),
    {
        self.pending_batch_head_facts();
        let batch = self.batch_ring@[self.batch_head as int];
        assert(batch.epoch.value == self.completed + 1);
        assert forall|offset: int| 0 <= offset < batch.member_count implies {
            let handle = #[trigger] self.member_ring@[
                ring_position::<C>(self.member_head, offset as nat)
            ];
            &&& handle.slot_spec() < C
            &&& self.slots@[handle.slot_spec() as int].generation
                == handle.generation_spec()
            &&& self.slots@[handle.slot_spec() as int].active_epoch == batch.epoch.value
            &&& !self.slots@[handle.slot_spec() as int].in_reclaim_ring
            &&& (self.slots@[handle.slot_spec() as int].state == RequestState::InFlight
                || self.slots@[handle.slot_spec() as int].state == RequestState::Retiring)
        } by {
            self.pending_member_facts(offset as usize);
        }
        assert forall|left: int, right: int|
            0 <= left < batch.member_count
                && 0 <= right < batch.member_count
                && left != right implies
                    #[trigger] request_ring_slots_differ::<C>(
                        self.member_ring@,
                        self.member_head,
                        left,
                        right,
                    )
        by {
            self.pending_batch_head_facts();
            assert(left < self.member_len);
            assert(right < self.member_len);
            self.basic_implies_member_ring();
            reveal(Scheduler::member_ring_invariant);
            reveal(Scheduler::member_distinct_invariant);
        }
    }

    proof fn pending_batch_head_facts(&self)
        requires
            self.basic_invariant(),
            self.batch_len > 0,
        ensures
            self.batch_ring@[self.batch_head as int].member_count > 0,
            self.batch_ring@[self.batch_head as int].member_count <= self.member_len,
            self.batch_ring@[self.batch_head as int].epoch.value == self.completed + 1,
    {
        self.basic_implies_scalar();
        self.basic_implies_batch_ring();
        let batch = self.batch_ring@[self.batch_head as int];
        assert(ring_position::<C>(self.batch_head, 0) == self.batch_head);
        assert(batch.member_count > 0) by {
            reveal(Scheduler::batch_ring_invariant);
        }
        batch_member_sum_monotonic::<C>(
            self.batch_ring@,
            self.batch_head,
            1,
            self.batch_len as nat,
        );
        assert(batch_member_sum::<C>(self.batch_ring@, self.batch_head, 1)
            == batch.member_count) by {
            reveal(batch_member_sum);
        }
        assert(batch.member_count <= self.member_len) by {
            reveal(Scheduler::batch_ring_invariant);
        }
        assert(batch.epoch.value as int == self.completed as int + 1) by {
            reveal(Scheduler::batch_ring_invariant);
        }
    }

    proof fn member_entry_bounds(&self, offset: usize)
        requires
            self.basic_invariant(),
            offset < self.member_len,
        ensures
            offset < C,
            0 <= ring_position::<C>(self.member_head, offset as nat) < C,
    {
        self.basic_implies_scalar();
        reveal(Scheduler::scalar_invariant);
        ring_position_bounds::<C>(self.member_head, offset as nat);
    }

    proof fn member_entry_slot_bound(&self, offset: usize)
        requires
            self.basic_invariant(),
            offset < self.member_len,
        ensures self.member_ring@[
            ring_position::<C>(self.member_head, offset as nat)
        ].slot_spec() < C,
    {
        self.basic_implies_member_entries();
        assert(self.member_entry_valid(offset as int)) by {
            reveal(Scheduler::member_entries_invariant);
        }
        reveal(Scheduler::member_entry_valid);
    }

    proof fn member_entry_generation(&self, offset: usize)
        requires
            self.basic_invariant(),
            offset < self.member_len,
        ensures {
            let handle = self.member_ring@[
                ring_position::<C>(self.member_head, offset as nat)
            ];
            self.slots@[handle.slot_spec() as int].generation == handle.generation_spec()
        },
    {
        self.basic_implies_member_entries();
        assert(self.member_entry_valid(offset as int)) by {
            reveal(Scheduler::member_entries_invariant);
        }
        assert({
            let handle = self.member_ring@[
                ring_position::<C>(self.member_head, offset as nat)
            ];
            self.slots@[handle.slot_spec() as int].generation == handle.generation_spec()
        }) by {
            reveal(Scheduler::member_entry_valid);
        }
    }

    proof fn member_entry_state(&self, offset: usize)
        requires
            self.basic_invariant(),
            offset < self.member_len,
        ensures {
            let handle = self.member_ring@[
                ring_position::<C>(self.member_head, offset as nat)
            ];
            self.slots@[handle.slot_spec() as int].state == RequestState::InFlight
                || self.slots@[handle.slot_spec() as int].state == RequestState::Retiring
        },
    {
        self.basic_implies_member_entries();
        assert(self.member_entry_valid(offset as int)) by {
            reveal(Scheduler::member_entries_invariant);
        }
        assert({
            let handle = self.member_ring@[
                ring_position::<C>(self.member_head, offset as nat)
            ];
            self.slots@[handle.slot_spec() as int].state == RequestState::InFlight
                || self.slots@[handle.slot_spec() as int].state == RequestState::Retiring
        }) by {
            reveal(Scheduler::member_entry_valid);
        }
    }

    proof fn member_entry_facts(&self, offset: usize)
        requires
            self.basic_invariant(),
            offset < self.member_len,
        ensures
            offset < C,
            0 <= ring_position::<C>(self.member_head, offset as nat) < C,
            {
                let handle = self.member_ring@[
                    ring_position::<C>(self.member_head, offset as nat)
                ];
                &&& handle.slot_spec() < C
                &&& self.slots@[handle.slot_spec() as int].generation
                    == handle.generation_spec()
                &&& self.completed < self.slots@[handle.slot_spec() as int].active_epoch
                &&& self.slots@[handle.slot_spec() as int].active_epoch <= self.submitted
                &&& !self.slots@[handle.slot_spec() as int].in_reclaim_ring
                &&& (self.slots@[handle.slot_spec() as int].state
                    == RequestState::InFlight
                    || self.slots@[handle.slot_spec() as int].state
                        == RequestState::Retiring)
            },
    {
        self.member_entry_bounds(offset);
        self.member_entry_slot_bound(offset);
        self.member_entry_generation(offset);
        self.member_entry_state(offset);
        self.basic_implies_member_entries();
        assert(self.member_entry_valid(offset as int)) by {
            reveal(Scheduler::member_entries_invariant);
        }
        reveal(Scheduler::member_entry_valid);
    }

    proof fn pending_member_epoch_fact(&self, offset: usize)
        requires
            self.basic_invariant(),
            self.batch_len > 0,
            offset < self.batch_ring@[self.batch_head as int].member_count,
        ensures {
            let handle = self.member_ring@[
                ring_position::<C>(self.member_head, offset as nat)
            ];
            self.slots@[handle.slot_spec() as int].active_epoch
                == self.batch_ring@[self.batch_head as int].epoch.value
        },
    {
        assert(batch_member_sum::<C>(self.batch_ring@, self.batch_head, 0) == 0) by {
            reveal(batch_member_sum);
        }
        assert(batch_member_sum::<C>(self.batch_ring@, self.batch_head, 1)
            == self.batch_ring@[self.batch_head as int].member_count) by {
            reveal(batch_member_sum);
        }
        self.basic_batch_member_epoch_fact(0, offset as int);
    }

    proof fn pending_member_facts(&self, offset: usize)
        requires
            self.basic_invariant(),
            self.batch_len > 0,
            offset < self.batch_ring@[self.batch_head as int].member_count,
        ensures
            offset < C,
            0 <= ring_position::<C>(self.member_head, offset as nat) < C,
            {
                let handle = self.member_ring@[
                    ring_position::<C>(self.member_head, offset as nat)
                ];
                &&& handle.slot_spec() < C
                &&& self.slots@[handle.slot_spec() as int].generation
                    == handle.generation_spec()
                &&& self.slots@[handle.slot_spec() as int].active_epoch
                    == self.batch_ring@[self.batch_head as int].epoch.value
                &&& !self.slots@[handle.slot_spec() as int].in_reclaim_ring
                &&& (self.slots@[handle.slot_spec() as int].state
                    == RequestState::InFlight
                    || self.slots@[handle.slot_spec() as int].state
                        == RequestState::Retiring)
            },
    {
        self.pending_batch_head_facts();
        assert(offset < self.member_len);
        self.member_entry_facts(offset);
        self.pending_member_epoch_fact(offset);
    }

    closed spec fn completion_member_valid(&self, offset: int, observed: u64) -> bool {
        let handle = self.member_ring@[
            ring_position::<C>(self.member_head, offset as nat)
        ];
        &&& handle.slot_spec() < C
        &&& self.slots@[handle.slot_spec() as int].generation == handle.generation_spec()
        &&& self.slots@[handle.slot_spec() as int].active_epoch == observed
        &&& (self.slots@[handle.slot_spec() as int].state == RequestState::InFlight
            || self.slots@[handle.slot_spec() as int].state == RequestState::Retiring)
    }

    proof fn pending_completion_members_valid(&self, observed: u64)
        requires
            self.basic_invariant(),
            self.batch_len > 0,
            self.batch_ring@[self.batch_head as int].epoch.value == observed,
        ensures
            forall|offset: int|
                0 <= offset < self.batch_ring@[self.batch_head as int].member_count ==>
                    #[trigger] self.completion_member_valid(offset, observed),
    {
        assert forall|offset: int|
            0 <= offset < self.batch_ring@[self.batch_head as int].member_count implies
                #[trigger] self.completion_member_valid(offset, observed)
        by {
            self.pending_batch_head_facts();
            self.member_entry_facts(offset as usize);
            self.pending_member_epoch_fact(offset as usize);
            reveal(Scheduler::completion_member_valid);
        }
    }

    proof fn detached_refines_preserves_scalar(
        &self,
        before: &Self,
        detached: &KvDetachedRequest,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.detached_refines(before, detached, &Ok(request)),
        ensures self.scalar_invariant(),
    {
        reveal(Scheduler::detached_refines);
        reveal(Scheduler::detached_count_refines);
    }

    proof fn detached_refines_preserves_slots(
        &self,
        before: &Self,
        detached: &KvDetachedRequest,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.detached_refines(before, detached, &Ok(request)),
        ensures self.slot_invariant(),
    {
        before.basic_implies_slots();
        reveal(Scheduler::slot_invariant);
        reveal(Scheduler::detached_refines);
        reveal(Scheduler::detached_slot_refines);
        reveal(Scheduler::slot_model);
        reveal(ferric_spec::scheduling::request_transition);
    }

    proof fn detached_free_append_facts(
        &self,
        before: &Self,
        detached: &KvDetachedRequest,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.detached_refines(before, detached, &Ok(request)),
        ensures
            self.free_head == before.free_head,
            self.free_len == before.free_len + 1,
            forall|slot_index: int| {
                #[trigger] usize_ring_contains::<C>(
                    self.free_ring@,
                    self.free_head,
                    self.free_len,
                    slot_index,
                ) == (usize_ring_contains::<C>(
                    before.free_ring@,
                    before.free_head,
                    before.free_len,
                    slot_index,
                ) || slot_index == detached.request_spec().slot_spec())
            },
            forall|left: int, right: int|
                0 <= left < self.free_len && 0 <= right < self.free_len && left != right ==>
                    #[trigger] usize_ring_entries_differ::<C>(
                        self.free_ring@,
                        self.free_head,
                        left,
                        right,
                    ),
            forall|offset: int| 0 <= offset < before.free_len ==>
                #[trigger] self.free_ring@[
                    ring_position::<C>(self.free_head, offset as nat)
                ] == before.free_ring@[
                    ring_position::<C>(before.free_head, offset as nat)
                ],
            self.free_ring@[
                ring_position::<C>(self.free_head, before.free_len as nat)
            ] == detached.request_spec().slot_spec(),
    {
        before.basic_implies_free_ring();
        before.basic_implies_slots();
        reveal(Scheduler::detached_refines);
        reveal(Scheduler::detached_slot_refines);
        reveal(Scheduler::detached_free_ring_refines);
        let slot_index = detached.request_spec().slot_spec() as int;
        let tail = ring_position::<C>(before.free_head, before.free_len as nat);
        ring_position_bounds::<C>(before.free_head, before.free_len as nat);
        assert(0 <= tail < before.free_ring@.len());
        assert(!before.slots@[slot_index].in_free_ring) by {
            reveal(Scheduler::slot_invariant);
            reveal(Scheduler::slot_model);
            reveal(ferric_spec::scheduling::request_transition);
        }
        assert(!usize_ring_contains::<C>(
            before.free_ring@,
            before.free_head,
            before.free_len,
            slot_index,
        )) by {
            reveal(Scheduler::free_ring_invariant);
        }
        assert(self.free_ring@ == before.free_ring@.update(tail, slot_index as usize)) by {
            assert forall|ring_index: int| 0 <= ring_index < C implies
                self.free_ring@[ring_index]
                    == before.free_ring@.update(tail, slot_index as usize)[ring_index]
            by {
                if ring_index == tail {
                } else {
                    assert(self.free_ring@[ring_index] == before.free_ring@[ring_index]);
                }
            }
        }
        usize_ring_append_facts::<C>(
            before.free_ring@,
            self.free_ring@,
            before.free_head,
            before.free_len,
            slot_index as usize,
        );
        assert(slot_index as usize as int == slot_index);
        assert(self.free_len == ((before.free_len as int) + 1) as usize);
        assert forall|observed: int| usize_ring_contains::<C>(
            self.free_ring@,
            self.free_head,
            self.free_len,
            observed,
        ) == (usize_ring_contains::<C>(
            before.free_ring@,
            before.free_head,
            before.free_len,
            observed,
        ) || observed == detached.request_spec().slot_spec()) by {
            assert(self.free_head == before.free_head);
            assert(observed == slot_index <==>
                observed == detached.request_spec().slot_spec());
        }
    }

    proof fn detached_free_entries_are_vacant(
        &self,
        before: &Self,
        detached: &KvDetachedRequest,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.detached_refines(before, detached, &Ok(request)),
        ensures
            forall|offset: int| 0 <= offset < self.free_len ==> {
                let observed = #[trigger] self.free_ring@[
                    ring_position::<C>(self.free_head, offset as nat)
                ];
                &&& observed < C
                &&& self.slots@[observed as int].state == RequestState::Vacant
                &&& self.slots@[observed as int].in_free_ring
            },
    {
        before.basic_implies_free_ring();
        before.basic_implies_slots();
        self.detached_free_append_facts(before, detached, request);
        self.detached_refines_preserves_scalar(before, detached, request);
        self.detached_refines_preserves_slots(before, detached, request);
        let slot_index = detached.request_spec().slot_spec() as int;
        assert forall|offset: int| 0 <= offset < self.free_len implies {
            let observed = #[trigger] self.free_ring@[
                ring_position::<C>(self.free_head, offset as nat)
            ];
            &&& observed < C
            &&& self.slots@[observed as int].state == RequestState::Vacant
            &&& self.slots@[observed as int].in_free_ring
        } by {
            if offset < before.free_len {
                let observed = before.free_ring@[
                    ring_position::<C>(before.free_head, offset as nat)
                ];
                assert(observed != slot_index as usize);
                assert(self.slots@[observed as int] == before.slots@[observed as int]);
                reveal(Scheduler::free_ring_invariant);
            } else {
                assert(offset == before.free_len);
                reveal(Scheduler::detached_slot_refines);
                reveal(Scheduler::slot_model);
                reveal(ferric_spec::scheduling::request_transition);
            }
        }
    }

    proof fn detached_free_exact_membership(
        &self,
        before: &Self,
        detached: &KvDetachedRequest,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.detached_refines(before, detached, &Ok(request)),
        ensures
            forall|observed: int| 0 <= observed < C ==>
                #[trigger] self.slots@[observed].in_free_ring
                    == usize_ring_contains::<C>(
                        self.free_ring@,
                        self.free_head,
                        self.free_len,
                        observed,
                    ),
    {
        before.basic_implies_free_ring();
        before.basic_implies_slots();
        self.detached_free_append_facts(before, detached, request);
        self.detached_refines_preserves_slots(before, detached, request);
        reveal(Scheduler::detached_refines);
        reveal(Scheduler::detached_slot_refines);
        let slot_index = detached.request_spec().slot_spec() as int;
        assert forall|observed: int| 0 <= observed < C implies
            self.slots@[observed].in_free_ring
                == usize_ring_contains::<C>(
                    self.free_ring@,
                    self.free_head,
                    self.free_len,
                    observed,
                )
        by {
            if observed == slot_index {
                reveal(Scheduler::detached_slot_refines);
            } else {
                assert(self.slots@[observed] == before.slots@[observed]);
                reveal(Scheduler::free_ring_invariant);
            }
        }
    }

    proof fn detached_free_has_capacity_for_nonmember(
        &self,
        before: &Self,
        detached: &KvDetachedRequest,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.detached_refines(before, detached, &Ok(request)),
        ensures
            forall|observed: int| 0 <= observed < C
                && !(#[trigger] self.slots@[observed].in_free_ring) ==> self.free_len < C,
    {
        self.detached_refines_preserves_scalar(before, detached, request);
        self.detached_refines_preserves_slots(before, detached, request);
        assert forall|observed: int| 0 <= observed < C
            && !self.slots@[observed].in_free_ring implies self.free_len < C
        by {
            assert(self.slots@[observed].state != RequestState::Vacant) by {
                reveal(Scheduler::slot_invariant);
            }
            live_count_positive_at(self.slots@, C as nat, observed);
            reveal(Scheduler::scalar_invariant);
        }
    }

    proof fn detached_refines_preserves_free_ring(
        &self,
        before: &Self,
        detached: &KvDetachedRequest,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.detached_refines(before, detached, &Ok(request)),
        ensures self.free_ring_invariant(),
    {
        self.detached_free_append_facts(before, detached, request);
        self.detached_free_entries_are_vacant(before, detached, request);
        self.detached_free_exact_membership(before, detached, request);
        self.detached_free_has_capacity_for_nonmember(before, detached, request);
        reveal(Scheduler::free_ring_invariant);
    }

    proof fn detached_refines_preserves_reclaim_ring(
        &self,
        before: &Self,
        detached: &KvDetachedRequest,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.detached_refines(before, detached, &Ok(request)),
        ensures self.reclaim_ring_invariant(),
    {
        before.basic_implies_reclaim_ring();
        reveal(Scheduler::reclaim_ring_invariant);
        reveal(Scheduler::detached_refines);
        reveal(Scheduler::detached_slot_refines);
        reveal(Scheduler::detached_other_rings_refine);
    }

    proof fn detached_member_entry_preserved(
        &self,
        before: &Self,
        detached: &KvDetachedRequest,
        request: RequestId,
        offset: int,
    )
        requires
            before.basic_invariant(),
            self.detached_refines(before, detached, &Ok(request)),
            0 <= offset < self.member_len,
        ensures self.member_entry_valid(offset),
    {
        reveal(Scheduler::detached_refines);
        reveal(Scheduler::detached_slot_refines);
        reveal(Scheduler::detached_other_rings_refine);
        before.member_entry_facts(offset as usize);
        before.basic_implies_member_entries();
        let changed = detached.request_spec().slot_spec() as int;
        assert(before.slot_model(changed).phase == LifecyclePhase::RetiringQuiescent) by {
            reveal(Scheduler::slot_model);
            reveal(ferric_spec::scheduling::request_transition);
        }
        let handle = before.member_ring@[
            ring_position::<C>(before.member_head, offset as nat)
        ];
        assert(handle.slot_spec() as int != changed) by {
            reveal(Scheduler::slot_model);
        }
        assert(self.slots@[handle.slot_spec() as int]
            == before.slots@[handle.slot_spec() as int]);
        assert(before.member_entry_valid(offset)) by {
            reveal(Scheduler::member_entries_invariant);
        }
        reveal(Scheduler::member_entry_valid);
    }

    proof fn detached_member_membership_at(
        &self,
        before: &Self,
        detached: &KvDetachedRequest,
        request: RequestId,
        observed: int,
    )
        requires
            before.basic_invariant(),
            self.detached_refines(before, detached, &Ok(request)),
            0 <= observed < C,
        ensures
            (((self.slots@[observed].state == RequestState::InFlight
                || self.slots@[observed].state == RequestState::Retiring)
                && self.completed < self.slots@[observed].active_epoch)
                == request_ring_contains_slot::<C>(
                    self.member_ring@,
                    self.member_head,
                    self.member_len,
                    observed,
                )),
    {
        before.basic_implies_member_ring();
        reveal(Scheduler::member_ring_invariant);
        reveal(Scheduler::member_membership_invariant);
        reveal(Scheduler::detached_refines);
        reveal(Scheduler::detached_slot_refines);
        reveal(Scheduler::detached_other_rings_refine);
        let changed = detached.request_spec().slot_spec() as int;
        if observed == changed {
            assert(before.slot_model(changed).phase == LifecyclePhase::RetiringQuiescent) by {
                reveal(Scheduler::slot_model);
                reveal(ferric_spec::scheduling::request_transition);
            }
            reveal(Scheduler::slot_model);
        } else {
            assert(self.slots@[observed] == before.slots@[observed]);
        }
    }

    proof fn detached_refines_preserves_member_ring(
        &self,
        before: &Self,
        detached: &KvDetachedRequest,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.detached_refines(before, detached, &Ok(request)),
        ensures self.member_ring_invariant(),
    {
        before.basic_implies_member_ring();
        reveal(Scheduler::detached_refines);
        reveal(Scheduler::detached_other_rings_refine);
        assert forall|offset: int| 0 <= offset < self.member_len implies
            #[trigger] self.member_entry_valid(offset) by {
            self.detached_member_entry_preserved(
                before,
                detached,
                request,
                offset,
            );
        }
        assert(self.member_entries_invariant()) by {
            reveal(Scheduler::member_entries_invariant);
        }
        assert(self.member_distinct_invariant()) by {
            reveal(Scheduler::member_ring_invariant);
            reveal(Scheduler::member_distinct_invariant);
        }
        assert forall|observed: int| 0 <= observed < C implies
            (((#[trigger] self.slots@[observed].state == RequestState::InFlight
                || self.slots@[observed].state == RequestState::Retiring)
                && self.completed < self.slots@[observed].active_epoch)
                == request_ring_contains_slot::<C>(
                    self.member_ring@,
                    self.member_head,
                    self.member_len,
                    observed,
                )) by {
            self.detached_member_membership_at(
                before,
                detached,
                request,
                observed,
            );
        }
        assert(self.member_membership_invariant()) by {
            reveal(Scheduler::member_membership_invariant);
        }
        reveal(Scheduler::member_ring_invariant);
    }

    proof fn detached_batch_sum_preserved(
        &self,
        before: &Self,
        detached: &KvDetachedRequest,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.detached_refines(before, detached, &Ok(request)),
        ensures
            batch_member_sum::<C>(self.batch_ring@, self.batch_head, self.batch_len as nat)
                == self.member_len,
    {
        before.basic_implies_batch_ring();
        reveal(Scheduler::batch_ring_invariant);
        reveal(Scheduler::detached_refines);
        reveal(Scheduler::detached_other_rings_refine);
    }

    proof fn detached_batch_header_preserved(
        &self,
        before: &Self,
        detached: &KvDetachedRequest,
        request: RequestId,
        batch_offset: int,
    )
        requires
            before.basic_invariant(),
            self.detached_refines(before, detached, &Ok(request)),
            0 <= batch_offset < self.batch_len,
        ensures {
            let batch = self.batch_ring@[
                ring_position::<C>(self.batch_head, batch_offset as nat)
            ];
            &&& batch.member_count > 0
            &&& batch.epoch.value as int == self.completed as int + batch_offset + 1
            &&& batch.epoch.value <= self.submitted
        },
    {
        reveal(Scheduler::detached_refines);
        reveal(Scheduler::detached_other_rings_refine);
        before.basic_batch_entry_header_facts(batch_offset);
    }

    proof fn detached_batch_member_epoch_preserved(
        &self,
        before: &Self,
        detached: &KvDetachedRequest,
        request: RequestId,
        batch_offset: int,
        member_offset: int,
    )
        requires
            before.basic_invariant(),
            self.detached_refines(before, detached, &Ok(request)),
            0 <= batch_offset < self.batch_len,
            batch_member_sum::<C>(
                self.batch_ring@,
                self.batch_head,
                batch_offset as nat,
            ) <= member_offset < batch_member_sum::<C>(
                self.batch_ring@,
                self.batch_head,
                batch_offset as nat + 1,
            ),
        ensures {
            let batch = self.batch_ring@[
                ring_position::<C>(self.batch_head, batch_offset as nat)
            ];
            let handle = self.member_ring@[
                ring_position::<C>(self.member_head, member_offset as nat)
            ];
            self.slots@[handle.slot_spec() as int].active_epoch == batch.epoch.value
        },
    {
        reveal(Scheduler::detached_refines);
        reveal(Scheduler::detached_slot_refines);
        reveal(Scheduler::detached_other_rings_refine);
        before.basic_implies_batch_ring();
        batch_member_sum_monotonic::<C>(
            before.batch_ring@,
            before.batch_head,
            batch_offset as nat + 1,
            before.batch_len as nat,
        );
        assert(0 <= member_offset < before.member_len) by {
            reveal(Scheduler::batch_ring_invariant);
        }
        before.basic_batch_member_epoch_fact(batch_offset, member_offset);
        before.member_entry_facts(member_offset as usize);
        let changed = detached.request_spec().slot_spec() as int;
        assert(before.slot_model(changed).phase == LifecyclePhase::RetiringQuiescent) by {
            reveal(Scheduler::slot_model);
            reveal(ferric_spec::scheduling::request_transition);
        }
        let handle = before.member_ring@[
            ring_position::<C>(before.member_head, member_offset as nat)
        ];
        assert(handle.slot_spec() as int != changed) by {
            reveal(Scheduler::slot_model);
        }
        assert(self.slots@[handle.slot_spec() as int]
            == before.slots@[handle.slot_spec() as int]);
    }

    proof fn detached_batch_entry_preserved(
        &self,
        before: &Self,
        detached: &KvDetachedRequest,
        request: RequestId,
        batch_offset: int,
    )
        requires
            before.basic_invariant(),
            self.detached_refines(before, detached, &Ok(request)),
            0 <= batch_offset < self.batch_len,
        ensures {
            let batch = self.batch_ring@[
                ring_position::<C>(self.batch_head, batch_offset as nat)
            ];
            &&& batch.member_count > 0
            &&& batch.epoch.value as int == self.completed as int + batch_offset + 1
            &&& batch.epoch.value <= self.submitted
            &&& (forall|member_offset: int|
                batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    batch_offset as nat,
                ) <= member_offset < batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    batch_offset as nat + 1,
                ) ==> {
                    let handle = #[trigger] self.member_ring@[
                        ring_position::<C>(self.member_head, member_offset as nat)
                    ];
                    self.slots@[handle.slot_spec() as int].active_epoch == batch.epoch.value
                })
        },
    {
        self.detached_batch_header_preserved(before, detached, request, batch_offset);
        assert forall|member_offset: int|
            batch_member_sum::<C>(
                self.batch_ring@,
                self.batch_head,
                batch_offset as nat,
            ) <= member_offset < batch_member_sum::<C>(
                self.batch_ring@,
                self.batch_head,
                batch_offset as nat + 1,
            ) implies {
                let handle = #[trigger] self.member_ring@[
                    ring_position::<C>(self.member_head, member_offset as nat)
                ];
                self.slots@[handle.slot_spec() as int].active_epoch
                    == self.batch_ring@[
                        ring_position::<C>(self.batch_head, batch_offset as nat)
                    ].epoch.value
        } by {
            self.detached_batch_member_epoch_preserved(
                before,
                detached,
                request,
                batch_offset,
                member_offset,
            );
        }
    }

    proof fn detached_batch_entries_preserved(
        &self,
        before: &Self,
        detached: &KvDetachedRequest,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.detached_refines(before, detached, &Ok(request)),
        ensures
            forall|batch_offset: int| 0 <= batch_offset < self.batch_len ==> {
                let batch = #[trigger] self.batch_ring@[
                    ring_position::<C>(self.batch_head, batch_offset as nat)
                ];
                &&& batch.member_count > 0
                &&& batch.epoch.value as int == self.completed as int + batch_offset + 1
                &&& batch.epoch.value <= self.submitted
                &&& (forall|member_offset: int|
                    batch_member_sum::<C>(
                        self.batch_ring@,
                        self.batch_head,
                        batch_offset as nat,
                    ) <= member_offset < batch_member_sum::<C>(
                        self.batch_ring@,
                        self.batch_head,
                        batch_offset as nat + 1,
                    ) ==> {
                        let handle = #[trigger] self.member_ring@[
                            ring_position::<C>(self.member_head, member_offset as nat)
                        ];
                        self.slots@[handle.slot_spec() as int].active_epoch
                            == batch.epoch.value
                    })
            },
    {
        assert forall|batch_offset: int| 0 <= batch_offset < self.batch_len implies {
            let batch = #[trigger] self.batch_ring@[
                ring_position::<C>(self.batch_head, batch_offset as nat)
            ];
            &&& batch.member_count > 0
            &&& batch.epoch.value as int == self.completed as int + batch_offset + 1
            &&& batch.epoch.value <= self.submitted
            &&& (forall|member_offset: int|
                batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    batch_offset as nat,
                ) <= member_offset < batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    batch_offset as nat + 1,
                ) ==> {
                    let handle = #[trigger] self.member_ring@[
                        ring_position::<C>(self.member_head, member_offset as nat)
                    ];
                    self.slots@[handle.slot_spec() as int].active_epoch == batch.epoch.value
                })
        } by {
            self.detached_batch_entry_preserved(
                before,
                detached,
                request,
                batch_offset,
            );
        }
    }

    proof fn detached_refines_preserves_batch_ring(
        &self,
        before: &Self,
        detached: &KvDetachedRequest,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.detached_refines(before, detached, &Ok(request)),
        ensures self.batch_ring_invariant(),
    {
        self.detached_batch_sum_preserved(before, detached, request);
        self.detached_batch_entries_preserved(before, detached, request);
        reveal(Scheduler::batch_ring_invariant);
    }

    closed spec fn admitted_slot_refines(
        &self,
        before: &Self,
        request: RequestId,
    ) -> bool {
        let slot_index = request.slot_spec() as int;
        &&& 0 <= slot_index < C
        &&& request.slot_spec() == before.free_ring@[before.free_head as int]
        &&& before.slots@[slot_index].state == RequestState::Vacant
        &&& request.generation_spec() == before.slots@[slot_index].generation
        &&& self.slots_frame_except(before, slot_index)
        &&& self.slots@[slot_index].generation == before.slots@[slot_index].generation
        &&& self.slots@[slot_index].state == RequestState::Ready
        &&& self.slots@[slot_index].active_epoch == before.slots@[slot_index].active_epoch
        &&& self.slots@[slot_index].last_quiescent_epoch
            == before.slots@[slot_index].last_quiescent_epoch
        &&& !self.slots@[slot_index].in_free_ring
        &&& self.slots@[slot_index].in_reclaim_ring
            == before.slots@[slot_index].in_reclaim_ring
    }

    closed spec fn admitted_fields_refine(&self, before: &Self) -> bool {
        &&& before.free_len > 0
        &&& self.free_ring@ == before.free_ring@
        &&& self.free_head == next_position::<C>(before.free_head)
        &&& self.free_len + 1 == before.free_len
        &&& self.live_count == before.live_count + 1
        &&& self.member_ring@ == before.member_ring@
        &&& self.member_head == before.member_head
        &&& self.member_len == before.member_len
        &&& self.batch_ring@ == before.batch_ring@
        &&& self.batch_head == before.batch_head
        &&& self.batch_len == before.batch_len
        &&& self.reclaim_ring@ == before.reclaim_ring@
        &&& self.reclaim_head == before.reclaim_head
        &&& self.reclaim_len == before.reclaim_len
        &&& self.cursor == before.cursor
        &&& self.submitted == before.submitted
        &&& self.completed == before.completed
    }

    proof fn admitted_step_implies_refines(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.admitted_slot_refines(before, request),
            self.admitted_fields_refine(before),
        ensures self.admit_refines(before, &Ok(request)),
    {
        reveal(Scheduler::admit_refines);
        reveal(Scheduler::admitted_slot_refines);
        reveal(Scheduler::admitted_fields_refine);
        reveal(Scheduler::slot_model);
        reveal(ferric_spec::scheduling::request_transition);
    }

    pub closed spec fn admit_refines(
        &self,
        before: &Self,
        result: &Result<RequestId, SchedulerError>,
    ) -> bool {
        match result {
            Err(error) => {
                &&& *error == SchedulerError::OutOfSlots
                &&& before.free_len == 0
                &&& self.same_scalars(before)
            }
            Ok(request) => {
                let slot_index = before.free_ring@[before.free_head as int];
                &&& before.free_len > 0
                &&& request.slot_spec() == slot_index
                &&& request.generation_spec()
                    == before.slots@[slot_index as int].generation
                &&& ferric_spec::scheduling::request_transition(
                    before.slot_model(slot_index as int),
                    RequestTransition::Admit,
                ) == Ok(self.slot_model(slot_index as int))
                &&& self.slots_frame_except(before, slot_index as int)
                &&& self.slots@[slot_index as int].generation
                    == before.slots@[slot_index as int].generation
                &&& self.slots@[slot_index as int].active_epoch
                    == before.slots@[slot_index as int].active_epoch
                &&& self.slots@[slot_index as int].last_quiescent_epoch
                    == before.slots@[slot_index as int].last_quiescent_epoch
                &&& !self.slots@[slot_index as int].in_free_ring
                &&& self.slots@[slot_index as int].in_reclaim_ring
                    == before.slots@[slot_index as int].in_reclaim_ring
                &&& self.free_ring@ == before.free_ring@
                &&& self.free_head == next_position::<C>(before.free_head)
                &&& self.free_len + 1 == before.free_len
                &&& self.live_count == before.live_count + 1
                &&& self.member_ring@ == before.member_ring@
                &&& self.member_head == before.member_head
                &&& self.member_len == before.member_len
                &&& self.batch_ring@ == before.batch_ring@
                &&& self.batch_head == before.batch_head
                &&& self.batch_len == before.batch_len
                &&& self.reclaim_ring@ == before.reclaim_ring@
                &&& self.reclaim_head == before.reclaim_head
                &&& self.reclaim_len == before.reclaim_len
                &&& self.cursor == before.cursor
                &&& self.submitted == before.submitted
                &&& self.completed == before.completed
            }
        }
    }

    pub(crate) proof fn apply_admitted_request_identity(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.admit_refines(before, &Ok(request)),
        ensures
            request.slot_spec() < C,
            !before.slot_is_live_spec(request.slot_spec() as int),
            before.slot_generation_spec(request.slot_spec() as int)
                == request.generation_spec(),
    {
        before.basic_implies_scalar();
        before.basic_implies_free_ring();
        before.free_ring_entry_facts(0);
        reveal(Scheduler::admit_refines);
        reveal(Scheduler::slot_is_live_spec);
        reveal(Scheduler::slot_generation_spec);
        let slot_index = before.free_ring@[before.free_head as int];
        assert(0 < before.free_len);
        assert(ring_position::<C>(before.free_head, 0) == before.free_head);
        assert(slot_index < C);
        assert(before.slots@[slot_index as int].state == RequestState::Vacant);
        assert(request.slot_spec() == slot_index);
        assert(request.generation_spec() == before.slots@[slot_index as int].generation);
    }

    closed spec fn retired_slot_refines(
        &self,
        before: &Self,
        request: RequestId,
    ) -> bool {
        let slot_index = request.slot_spec() as int;
        &&& 0 <= slot_index < C
        &&& request.generation_spec() == before.slots@[slot_index].generation
        &&& (before.slots@[slot_index].state == RequestState::Ready
            || before.slots@[slot_index].state == RequestState::InFlight)
        &&& self.slots_frame_except(before, slot_index)
        &&& self.slots@[slot_index].generation == before.slots@[slot_index].generation
        &&& self.slots@[slot_index].state == RequestState::Retiring
        &&& self.slots@[slot_index].active_epoch == before.slots@[slot_index].active_epoch
        &&& self.slots@[slot_index].last_quiescent_epoch
            == before.slots@[slot_index].last_quiescent_epoch
        &&& self.slots@[slot_index].in_free_ring
            == before.slots@[slot_index].in_free_ring
        &&& self.slots@[slot_index].in_reclaim_ring
            == (before.slots@[slot_index].state == RequestState::Ready)
    }

    closed spec fn retired_fields_refine(
        &self,
        before: &Self,
        request: RequestId,
    ) -> bool {
        let slot_index = request.slot_spec() as int;
        &&& self.free_ring@ == before.free_ring@
        &&& self.free_head == before.free_head
        &&& self.free_len == before.free_len
        &&& self.member_ring@ == before.member_ring@
        &&& self.member_head == before.member_head
        &&& self.member_len == before.member_len
        &&& self.batch_ring@ == before.batch_ring@
        &&& self.batch_head == before.batch_head
        &&& self.batch_len == before.batch_len
        &&& self.cursor == before.cursor
        &&& self.submitted == before.submitted
        &&& self.completed == before.completed
        &&& self.live_count == before.live_count
        &&& if before.slots@[slot_index].state == RequestState::Ready {
            let tail = ring_position::<C>(before.reclaim_head, before.reclaim_len as nat);
            &&& before.reclaim_len < C
            &&& self.reclaim_head == before.reclaim_head
            &&& self.reclaim_len == before.reclaim_len + 1
            &&& self.reclaim_ring@
                == before.reclaim_ring@.update(tail, slot_index as usize)
        } else {
            &&& self.reclaim_ring@ == before.reclaim_ring@
            &&& self.reclaim_head == before.reclaim_head
            &&& self.reclaim_len == before.reclaim_len
        }
    }

    proof fn retired_step_implies_refines(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.retired_slot_refines(before, request),
            self.retired_fields_refine(before, request),
        ensures self.retire_refines(before, request, &Ok(())),
    {
        before.basic_implies_slots();
        reveal(Scheduler::slot_invariant);
        reveal(Scheduler::retire_refines);
        reveal(Scheduler::retired_slot_refines);
        reveal(Scheduler::retired_fields_refine);
        reveal(Scheduler::slot_model);
        reveal(Scheduler::slots_frame_except);
        reveal(retire_expected_error);
        reveal(ferric_spec::scheduling::request_transition);
        let slot_index = request.slot_spec() as int;
        if before.slots@[slot_index].state == RequestState::Ready {
            let tail = ring_position::<C>(before.reclaim_head, before.reclaim_len as nat);
            assert forall|ring_index: int| 0 <= ring_index < C && ring_index != tail implies
                #[trigger] self.reclaim_ring@[ring_index]
                    == before.reclaim_ring@[ring_index] by {}
        }
    }

    proof fn retire_error_reflexive(
        &self,
        request: RequestId,
        error: SchedulerError,
    )
        requires
            self.basic_invariant(),
            Some(error) == retire_expected_error::<C>(self, request),
        ensures
            self.retire_refines(self, request, &Err(error)),
            self.identity_frame(self),
    {
        self.same_scalars_reflexive();
        reveal(Scheduler::retire_refines);
        assert forall|slot_index: int| 0 <= slot_index < C implies {
            &&& self.slot_generation_spec(slot_index) == self.slot_generation_spec(slot_index)
            &&& self.slot_is_live_spec(slot_index) == self.slot_is_live_spec(slot_index)
        } by {}
    }

    proof fn retired_step_preserves_scalar(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.retired_slot_refines(before, request),
            self.retired_fields_refine(before, request),
        ensures self.scalar_invariant(),
    {
        before.basic_implies_scalar();
        reveal(Scheduler::scalar_invariant);
        reveal(Scheduler::retired_slot_refines);
        reveal(Scheduler::retired_fields_refine);
        let slot_index = request.slot_spec() as int;
        assert(self.slots@ == before.slots@.update(slot_index, self.slots@[slot_index])) by {
            assert forall|index: int| 0 <= index < C implies
                self.slots@[index]
                    == before.slots@.update(slot_index, self.slots@[slot_index])[index]
            by {
                if index != slot_index {
                    assert(self.slots@[index] == before.slots@[index]);
                }
            }
        }
        live_count_update_nonvacant(
            before.slots@,
            slot_index,
            self.slots@[slot_index],
            C as nat,
        );
        if before.slots@[slot_index].state == RequestState::Ready {
            nonreclaim_count_update_remove(
                before.slots@,
                slot_index,
                self.slots@[slot_index],
                C as nat,
            );
        } else {
            nonreclaim_count_update_preserved(
                before.slots@,
                slot_index,
                self.slots@[slot_index],
                C as nat,
            );
        }
    }

    proof fn retired_ready_slot_preserves_invariant(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.retired_slot_refines(before, request),
            self.retired_fields_refine(before, request),
            before.slots@[request.slot_spec() as int].state == RequestState::Ready,
        ensures self.slot_invariant_at(request.slot_spec() as int),
    {
        before.basic_implies_slots();
        reveal(Scheduler::slot_invariant);
        reveal(Scheduler::slot_invariant_at);
        reveal(Scheduler::retired_slot_refines);
        reveal(Scheduler::retired_fields_refine);
    }

    proof fn retired_inflight_slot_preserves_invariant(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.retired_slot_refines(before, request),
            self.retired_fields_refine(before, request),
            before.slots@[request.slot_spec() as int].state == RequestState::InFlight,
        ensures self.slot_invariant_at(request.slot_spec() as int),
    {
        before.basic_implies_slots();
        reveal(Scheduler::slot_invariant);
        reveal(Scheduler::slot_invariant_at);
        reveal(Scheduler::retired_slot_refines);
        reveal(Scheduler::retired_fields_refine);
    }

    proof fn retired_changed_slot_preserves_invariant(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.retired_slot_refines(before, request),
            self.retired_fields_refine(before, request),
        ensures self.slot_invariant_at(request.slot_spec() as int),
    {
        reveal(Scheduler::retired_slot_refines);
        if before.slots@[request.slot_spec() as int].state == RequestState::Ready {
            self.retired_ready_slot_preserves_invariant(before, request);
        } else {
            self.retired_inflight_slot_preserves_invariant(before, request);
        }
    }

    proof fn retired_step_preserves_slots(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.retired_slot_refines(before, request),
            self.retired_fields_refine(before, request),
        ensures self.slot_invariant(),
    {
        before.basic_implies_slots();
        self.retired_changed_slot_preserves_invariant(before, request);
        reveal(Scheduler::slot_invariant);
        reveal(Scheduler::slot_invariant_at);
        reveal(Scheduler::retired_slot_refines);
        reveal(Scheduler::retired_fields_refine);
        let slot_index = request.slot_spec() as int;
        assert forall|index: int| 0 <= index < C implies
            #[trigger] self.slot_invariant_at(index) by {
            if index != slot_index {
                assert(self.slots@[index] == before.slots@[index]);
            } else {
                assert(index == slot_index);
            }
        }
    }

    proof fn retired_step_preserves_free_ring(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.retired_slot_refines(before, request),
            self.retired_fields_refine(before, request),
        ensures self.free_ring_invariant(),
    {
        before.basic_implies_free_ring();
        reveal(Scheduler::free_ring_invariant);
        reveal(Scheduler::retired_slot_refines);
        reveal(Scheduler::retired_fields_refine);
        let slot_index = request.slot_spec() as int;
        assert forall|offset: int| 0 <= offset < self.free_len implies {
            let observed = #[trigger] self.free_ring@[
                ring_position::<C>(self.free_head, offset as nat)
            ];
            &&& observed < C
            &&& self.slots@[observed as int].state == RequestState::Vacant
            &&& self.slots@[observed as int].in_free_ring
        } by {
            let observed = self.free_ring@[
                ring_position::<C>(self.free_head, offset as nat)
            ];
            assert(observed as int != slot_index);
            assert(self.slots@[observed as int] == before.slots@[observed as int]);
        }
        assert forall|index: int| 0 <= index < C implies
            #[trigger] self.slots@[index].in_free_ring
                == usize_ring_contains::<C>(
                    self.free_ring@,
                    self.free_head,
                    self.free_len,
                    index,
                )
        by {
            if index != slot_index {
                assert(self.slots@[index] == before.slots@[index]);
            }
        }
    }

    proof fn retired_ready_reclaim_append_facts(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.retired_slot_refines(before, request),
            self.retired_fields_refine(before, request),
            before.slots@[request.slot_spec() as int].state == RequestState::Ready,
        ensures
            forall|index: int| {
                #[trigger] usize_ring_contains::<C>(
                    self.reclaim_ring@,
                    self.reclaim_head,
                    self.reclaim_len,
                    index,
                ) == (usize_ring_contains::<C>(
                    before.reclaim_ring@,
                    before.reclaim_head,
                    before.reclaim_len,
                    index,
                ) || index == request.slot_spec())
            },
            forall|left: int, right: int|
                0 <= left < self.reclaim_len
                    && 0 <= right < self.reclaim_len
                    && left != right ==>
                #[trigger] usize_ring_entries_differ::<C>(
                    self.reclaim_ring@,
                    self.reclaim_head,
                    left,
                    right,
                ),
            forall|offset: int| 0 <= offset < before.reclaim_len ==>
                #[trigger] self.reclaim_ring@[
                    ring_position::<C>(self.reclaim_head, offset as nat)
                ] == before.reclaim_ring@[
                    ring_position::<C>(before.reclaim_head, offset as nat)
                ],
            self.reclaim_ring@[
                ring_position::<C>(self.reclaim_head, before.reclaim_len as nat)
            ] == request.slot_spec(),
    {
        before.basic_implies_reclaim_ring();
        reveal(Scheduler::reclaim_ring_invariant);
        reveal(Scheduler::retired_slot_refines);
        reveal(Scheduler::retired_fields_refine);
        let slot_index = request.slot_spec() as int;
        assert(!usize_ring_contains::<C>(
            before.reclaim_ring@,
            before.reclaim_head,
            before.reclaim_len,
            slot_index,
        ));
        usize_ring_append_facts::<C>(
            before.reclaim_ring@,
            self.reclaim_ring@,
            before.reclaim_head,
            before.reclaim_len,
            slot_index as usize,
        );
        assert(slot_index as usize as int == slot_index);
        assert(self.reclaim_len == ((before.reclaim_len as int) + 1) as usize);
        assert forall|index: int| usize_ring_contains::<C>(
            self.reclaim_ring@,
            self.reclaim_head,
            self.reclaim_len,
            index,
        ) == (usize_ring_contains::<C>(
            before.reclaim_ring@,
            before.reclaim_head,
            before.reclaim_len,
            index,
        ) || index == request.slot_spec()) by {}
    }

    proof fn retired_ready_reclaim_entries(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.retired_slot_refines(before, request),
            self.retired_fields_refine(before, request),
            before.slots@[request.slot_spec() as int].state == RequestState::Ready,
        ensures
            forall|offset: int| 0 <= offset < self.reclaim_len ==> {
                let observed = #[trigger] self.reclaim_ring@[
                    ring_position::<C>(self.reclaim_head, offset as nat)
                ];
                &&& observed < C
                &&& self.slots@[observed as int].state == RequestState::Retiring
                &&& self.slots@[observed as int].active_epoch == NO_EPOCH
                &&& self.slots@[observed as int].in_reclaim_ring
            },
    {
        before.basic_implies_reclaim_ring();
        self.retired_ready_reclaim_append_facts(before, request);
        reveal(Scheduler::reclaim_ring_invariant);
        reveal(Scheduler::retired_slot_refines);
        let slot_index = request.slot_spec() as int;
        assert forall|offset: int| 0 <= offset < self.reclaim_len implies {
            let observed = #[trigger] self.reclaim_ring@[
                ring_position::<C>(self.reclaim_head, offset as nat)
            ];
            &&& observed < C
            &&& self.slots@[observed as int].state == RequestState::Retiring
            &&& self.slots@[observed as int].active_epoch == NO_EPOCH
            &&& self.slots@[observed as int].in_reclaim_ring
        } by {
            let observed = self.reclaim_ring@[
                ring_position::<C>(self.reclaim_head, offset as nat)
            ];
            assert(usize_ring_contains::<C>(
                self.reclaim_ring@,
                self.reclaim_head,
                self.reclaim_len,
                observed as int,
            )) by {
                reveal(usize_ring_contains);
                assert(exists|witness: int| 0 <= witness < self.reclaim_len
                    && #[trigger] self.reclaim_ring@[
                        ring_position::<C>(self.reclaim_head, witness as nat)
                    ] == observed) by {
                    let witness = offset;
                }
            }
            if observed as int != slot_index {
                assert(self.slots@[observed as int] == before.slots@[observed as int]);
            }
        }
    }

    proof fn retired_ready_reclaim_exact_membership(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.retired_slot_refines(before, request),
            self.retired_fields_refine(before, request),
            before.slots@[request.slot_spec() as int].state == RequestState::Ready,
        ensures
            forall|index: int| 0 <= index < C ==>
                #[trigger] self.slots@[index].in_reclaim_ring
                    == usize_ring_contains::<C>(
                        self.reclaim_ring@,
                        self.reclaim_head,
                        self.reclaim_len,
                        index,
                    ),
    {
        before.basic_implies_reclaim_ring();
        self.retired_ready_reclaim_append_facts(before, request);
        reveal(Scheduler::reclaim_ring_invariant);
        reveal(Scheduler::retired_slot_refines);
        let slot_index = request.slot_spec() as int;
        assert forall|index: int| 0 <= index < C implies
            #[trigger] self.slots@[index].in_reclaim_ring
                == usize_ring_contains::<C>(
                    self.reclaim_ring@,
                    self.reclaim_head,
                    self.reclaim_len,
                    index,
                )
        by {
            if index != slot_index {
                assert(self.slots@[index] == before.slots@[index]);
            }
        }
    }

    proof fn retired_ready_reclaim_has_capacity(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.retired_slot_refines(before, request),
            self.retired_fields_refine(before, request),
            before.slots@[request.slot_spec() as int].state == RequestState::Ready,
        ensures
            forall|index: int| 0 <= index < C
                && !(#[trigger] self.slots@[index].in_reclaim_ring) ==>
                    self.reclaim_len < C,
    {
        self.retired_step_preserves_scalar(before, request);
        reveal(Scheduler::scalar_invariant);
        assert forall|index: int| 0 <= index < C
            && !self.slots@[index].in_reclaim_ring implies self.reclaim_len < C
        by {
            if self.slots@[index].state == RequestState::Vacant {
                live_count_below_if_vacant(self.slots@, C as nat, index);
            } else {
                nonreclaim_count_positive_at(self.slots@, C as nat, index);
            }
        }
    }

    proof fn retired_ready_preserves_reclaim_ring(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.retired_slot_refines(before, request),
            self.retired_fields_refine(before, request),
            before.slots@[request.slot_spec() as int].state == RequestState::Ready,
        ensures self.reclaim_ring_invariant(),
    {
        self.retired_ready_reclaim_append_facts(before, request);
        self.retired_ready_reclaim_entries(before, request);
        self.retired_ready_reclaim_exact_membership(before, request);
        self.retired_ready_reclaim_has_capacity(before, request);
        reveal(Scheduler::reclaim_ring_invariant);
    }

    proof fn retired_inflight_preserves_reclaim_ring(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.retired_slot_refines(before, request),
            self.retired_fields_refine(before, request),
            before.slots@[request.slot_spec() as int].state == RequestState::InFlight,
        ensures self.reclaim_ring_invariant(),
    {
        before.basic_implies_reclaim_ring();
        reveal(Scheduler::reclaim_ring_invariant);
        reveal(Scheduler::retired_slot_refines);
        reveal(Scheduler::retired_fields_refine);
        let slot_index = request.slot_spec() as int;
        assert forall|offset: int| 0 <= offset < self.reclaim_len implies {
            let observed = #[trigger] self.reclaim_ring@[
                ring_position::<C>(self.reclaim_head, offset as nat)
            ];
            &&& observed < C
            &&& self.slots@[observed as int].state == RequestState::Retiring
            &&& self.slots@[observed as int].active_epoch == NO_EPOCH
            &&& self.slots@[observed as int].in_reclaim_ring
        } by {
            let observed = self.reclaim_ring@[
                ring_position::<C>(self.reclaim_head, offset as nat)
            ];
            assert(observed as int != slot_index);
            assert(self.slots@[observed as int] == before.slots@[observed as int]);
        }
        assert forall|index: int| 0 <= index < C implies
            #[trigger] self.slots@[index].in_reclaim_ring
                == usize_ring_contains::<C>(
                    self.reclaim_ring@,
                    self.reclaim_head,
                    self.reclaim_len,
                    index,
                )
        by {
            if index != slot_index {
                assert(self.slots@[index] == before.slots@[index]);
            }
        }
    }

    proof fn retired_step_preserves_reclaim_ring(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.retired_slot_refines(before, request),
            self.retired_fields_refine(before, request),
        ensures self.reclaim_ring_invariant(),
    {
        reveal(Scheduler::retired_slot_refines);
        if before.slots@[request.slot_spec() as int].state == RequestState::Ready {
            self.retired_ready_preserves_reclaim_ring(before, request);
        } else {
            self.retired_inflight_preserves_reclaim_ring(before, request);
        }
    }

    proof fn retired_member_entry_preserved(
        &self,
        before: &Self,
        request: RequestId,
        offset: int,
    )
        requires
            before.basic_invariant(),
            self.retired_slot_refines(before, request),
            self.retired_fields_refine(before, request),
            0 <= offset < self.member_len,
        ensures self.member_entry_valid(offset),
    {
        reveal(Scheduler::retired_fields_refine);
        before.basic_implies_member_entries();
        assert(before.member_entry_valid(offset)) by {
            reveal(Scheduler::member_entries_invariant);
        }
        let handle = before.member_ring@[
            ring_position::<C>(before.member_head, offset as nat)
        ];
        reveal(Scheduler::retired_slot_refines);
        if handle.slot_spec() as int != request.slot_spec() as int {
            assert(self.slots@[handle.slot_spec() as int]
                == before.slots@[handle.slot_spec() as int]);
        }
        reveal(Scheduler::member_entry_valid);
    }

    proof fn retired_member_membership_at(
        &self,
        before: &Self,
        request: RequestId,
        index: int,
    )
        requires
            before.basic_invariant(),
            self.retired_slot_refines(before, request),
            self.retired_fields_refine(before, request),
            0 <= index < C,
        ensures
            (((self.slots@[index].state == RequestState::InFlight
                || self.slots@[index].state == RequestState::Retiring)
                && self.completed < self.slots@[index].active_epoch)
                == request_ring_contains_slot::<C>(
                    self.member_ring@,
                    self.member_head,
                    self.member_len,
                    index,
                )),
    {
        before.basic_implies_member_ring();
        reveal(Scheduler::member_ring_invariant);
        reveal(Scheduler::member_membership_invariant);
        reveal(Scheduler::retired_slot_refines);
        reveal(Scheduler::retired_fields_refine);
        let changed = request.slot_spec() as int;
        if index != changed {
            assert(self.slots@[index] == before.slots@[index]);
        }
    }

    proof fn retired_step_preserves_member_ring(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.retired_slot_refines(before, request),
            self.retired_fields_refine(before, request),
        ensures self.member_ring_invariant(),
    {
        before.basic_implies_member_ring();
        reveal(Scheduler::retired_fields_refine);
        assert forall|offset: int| 0 <= offset < self.member_len implies
            #[trigger] self.member_entry_valid(offset) by {
            self.retired_member_entry_preserved(before, request, offset);
        }
        assert(self.member_entries_invariant()) by {
            reveal(Scheduler::member_entries_invariant);
        }
        assert(self.member_distinct_invariant()) by {
            reveal(Scheduler::member_ring_invariant);
            reveal(Scheduler::member_distinct_invariant);
        }
        assert forall|index: int| 0 <= index < C implies
            (((#[trigger] self.slots@[index].state == RequestState::InFlight
                || self.slots@[index].state == RequestState::Retiring)
                && self.completed < self.slots@[index].active_epoch)
                == request_ring_contains_slot::<C>(
                    self.member_ring@,
                    self.member_head,
                    self.member_len,
                    index,
                ))
        by {
            self.retired_member_membership_at(before, request, index);
        }
        assert(self.member_membership_invariant()) by {
            reveal(Scheduler::member_membership_invariant);
        }
        reveal(Scheduler::member_ring_invariant);
    }

    proof fn retired_batch_sum_preserved(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.retired_slot_refines(before, request),
            self.retired_fields_refine(before, request),
        ensures
            batch_member_sum::<C>(self.batch_ring@, self.batch_head, self.batch_len as nat)
                == self.member_len,
    {
        before.basic_implies_batch_ring();
        reveal(Scheduler::batch_ring_invariant);
        reveal(Scheduler::retired_fields_refine);
    }

    proof fn retired_batch_header_preserved(
        &self,
        before: &Self,
        request: RequestId,
        batch_offset: int,
    )
        requires
            before.basic_invariant(),
            self.retired_slot_refines(before, request),
            self.retired_fields_refine(before, request),
            0 <= batch_offset < self.batch_len,
        ensures {
            let batch = self.batch_ring@[
                ring_position::<C>(self.batch_head, batch_offset as nat)
            ];
            &&& batch.member_count > 0
            &&& batch.epoch.value as int == self.completed as int + batch_offset + 1
            &&& batch.epoch.value <= self.submitted
        },
    {
        reveal(Scheduler::retired_fields_refine);
        before.basic_batch_entry_header_facts(batch_offset);
    }

    proof fn retired_member_active_epoch_preserved(
        &self,
        before: &Self,
        request: RequestId,
        member_offset: int,
    )
        requires
            before.basic_invariant(),
            self.retired_slot_refines(before, request),
            self.retired_fields_refine(before, request),
            0 <= member_offset < self.member_len,
        ensures {
            let handle = self.member_ring@[
                ring_position::<C>(self.member_head, member_offset as nat)
            ];
            self.slots@[handle.slot_spec() as int].active_epoch
                == before.slots@[handle.slot_spec() as int].active_epoch
        },
    {
        reveal(Scheduler::retired_fields_refine);
        before.member_entry_facts(member_offset as usize);
        let handle = before.member_ring@[
            ring_position::<C>(before.member_head, member_offset as nat)
        ];
        reveal(Scheduler::retired_slot_refines);
        if handle.slot_spec() as int != request.slot_spec() as int {
            assert(self.slots@[handle.slot_spec() as int]
                == before.slots@[handle.slot_spec() as int]);
        }
    }

    proof fn retired_batch_member_epoch_preserved(
        &self,
        before: &Self,
        request: RequestId,
        batch_offset: int,
        member_offset: int,
    )
        requires
            before.basic_invariant(),
            self.retired_slot_refines(before, request),
            self.retired_fields_refine(before, request),
            0 <= batch_offset < self.batch_len,
            batch_member_sum::<C>(
                self.batch_ring@,
                self.batch_head,
                batch_offset as nat,
            ) <= member_offset < batch_member_sum::<C>(
                self.batch_ring@,
                self.batch_head,
                batch_offset as nat + 1,
            ),
        ensures {
            let batch = self.batch_ring@[
                ring_position::<C>(self.batch_head, batch_offset as nat)
            ];
            let handle = self.member_ring@[
                ring_position::<C>(self.member_head, member_offset as nat)
            ];
            self.slots@[handle.slot_spec() as int].active_epoch == batch.epoch.value
        },
    {
        reveal(Scheduler::retired_fields_refine);
        before.basic_implies_batch_ring();
        reveal(Scheduler::batch_ring_invariant);
        batch_member_sum_monotonic::<C>(
            before.batch_ring@,
            before.batch_head,
            batch_offset as nat + 1,
            before.batch_len as nat,
        );
        self.retired_member_active_epoch_preserved(before, request, member_offset);
        before.basic_batch_member_epoch_fact(batch_offset, member_offset);
    }

    proof fn retired_batch_entry_preserved(
        &self,
        before: &Self,
        request: RequestId,
        batch_offset: int,
    )
        requires
            before.basic_invariant(),
            self.retired_slot_refines(before, request),
            self.retired_fields_refine(before, request),
            0 <= batch_offset < self.batch_len,
        ensures {
            let batch = self.batch_ring@[
                ring_position::<C>(self.batch_head, batch_offset as nat)
            ];
            &&& batch.member_count > 0
            &&& batch.epoch.value as int == self.completed as int + batch_offset + 1
            &&& batch.epoch.value <= self.submitted
            &&& (forall|member_offset: int|
                batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    batch_offset as nat,
                ) <= member_offset < batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    batch_offset as nat + 1,
                ) ==> {
                    let handle = #[trigger] self.member_ring@[
                        ring_position::<C>(self.member_head, member_offset as nat)
                    ];
                    self.slots@[handle.slot_spec() as int].active_epoch == batch.epoch.value
                })
        },
    {
        self.retired_batch_header_preserved(before, request, batch_offset);
        assert forall|member_offset: int|
            batch_member_sum::<C>(
                self.batch_ring@,
                self.batch_head,
                batch_offset as nat,
            ) <= member_offset < batch_member_sum::<C>(
                self.batch_ring@,
                self.batch_head,
                batch_offset as nat + 1,
            ) implies {
                let handle = #[trigger] self.member_ring@[
                    ring_position::<C>(self.member_head, member_offset as nat)
                ];
                self.slots@[handle.slot_spec() as int].active_epoch
                    == self.batch_ring@[
                        ring_position::<C>(self.batch_head, batch_offset as nat)
                    ].epoch.value
        } by {
            self.retired_batch_member_epoch_preserved(
                before,
                request,
                batch_offset,
                member_offset,
            );
        }
    }

    proof fn retired_batch_entries_preserved(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.retired_slot_refines(before, request),
            self.retired_fields_refine(before, request),
        ensures
            forall|batch_offset: int| 0 <= batch_offset < self.batch_len ==> {
                let batch = #[trigger] self.batch_ring@[
                    ring_position::<C>(self.batch_head, batch_offset as nat)
                ];
                &&& batch.member_count > 0
                &&& batch.epoch.value as int == self.completed as int + batch_offset + 1
                &&& batch.epoch.value <= self.submitted
                &&& (forall|member_offset: int|
                    batch_member_sum::<C>(
                        self.batch_ring@,
                        self.batch_head,
                        batch_offset as nat,
                    ) <= member_offset < batch_member_sum::<C>(
                        self.batch_ring@,
                        self.batch_head,
                        batch_offset as nat + 1,
                    ) ==> {
                        let handle = #[trigger] self.member_ring@[
                            ring_position::<C>(self.member_head, member_offset as nat)
                        ];
                        self.slots@[handle.slot_spec() as int].active_epoch == batch.epoch.value
                    })
            },
    {
        assert forall|batch_offset: int| 0 <= batch_offset < self.batch_len implies {
            let batch = #[trigger] self.batch_ring@[
                ring_position::<C>(self.batch_head, batch_offset as nat)
            ];
            &&& batch.member_count > 0
            &&& batch.epoch.value as int == self.completed as int + batch_offset + 1
            &&& batch.epoch.value <= self.submitted
            &&& (forall|member_offset: int|
                batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    batch_offset as nat,
                ) <= member_offset < batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    batch_offset as nat + 1,
                ) ==> {
                    let handle = #[trigger] self.member_ring@[
                        ring_position::<C>(self.member_head, member_offset as nat)
                    ];
                    self.slots@[handle.slot_spec() as int].active_epoch == batch.epoch.value
                })
        } by {
            self.retired_batch_entry_preserved(before, request, batch_offset);
        }
    }

    proof fn retired_step_preserves_batch_ring(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.retired_slot_refines(before, request),
            self.retired_fields_refine(before, request),
        ensures self.batch_ring_invariant(),
    {
        self.retired_batch_sum_preserved(before, request);
        self.retired_batch_entries_preserved(before, request);
        reveal(Scheduler::batch_ring_invariant);
    }

    proof fn retired_step_preserves_basic(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.retired_slot_refines(before, request),
            self.retired_fields_refine(before, request),
        ensures self.basic_invariant(),
    {
        self.retired_step_preserves_scalar(before, request);
        self.retired_step_preserves_slots(before, request);
        self.retired_step_preserves_free_ring(before, request);
        self.retired_step_preserves_reclaim_ring(before, request);
        self.retired_step_preserves_member_ring(before, request);
        self.retired_step_preserves_batch_ring(before, request);
        reveal(Scheduler::basic_invariant);
    }

    proof fn retired_step_preserves_identity(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.retired_slot_refines(before, request),
            self.retired_fields_refine(before, request),
        ensures self.identity_frame(before),
    {
        reveal(Scheduler::retired_slot_refines);
        reveal(Scheduler::slot_generation_spec);
        reveal(Scheduler::slot_is_live_spec);
        self.identity_frame_from_slots_frame(
            before,
            request.slot_spec() as int,
        );
    }

    proof fn retired_step_establishes_postconditions(
        &self,
        before: &Self,
        request: RequestId,
    )
        requires
            before.basic_invariant(),
            self.retired_slot_refines(before, request),
            self.retired_fields_refine(before, request),
        ensures
            self.basic_invariant(),
            self.retire_refines(before, request, &Ok(())),
            self.identity_frame(before),
    {
        self.retired_step_implies_refines(before, request);
        self.retired_step_preserves_basic(before, request);
        self.retired_step_preserves_identity(before, request);
    }

    proof fn retire_error_establishes_postconditions(
        &self,
        request: RequestId,
        error: SchedulerError,
    )
        requires
            self.basic_invariant(),
            Some(error) == retire_expected_error::<C>(self, request),
        ensures
            self.basic_invariant(),
            self.retire_refines(self, request, &Err(error)),
            self.identity_frame(self),
    {
        self.retire_error_reflexive(request, error);
    }

    pub closed spec fn retire_refines(
        &self,
        before: &Self,
        request: RequestId,
        result: &Result<(), SchedulerError>,
    ) -> bool {
        match result {
            Err(error) => {
                &&& Some(*error) == retire_expected_error::<C>(before, request)
                &&& self.same_scalars(before)
            }
            Ok(()) => {
                let slot_index = request.slot_spec() as int;
                &&& retire_expected_error::<C>(before, request).is_none()
                &&& slot_index < C
                &&& request.generation_spec() == before.slots@[slot_index].generation
                &&& ferric_spec::scheduling::request_transition(
                    before.slot_model(slot_index),
                    RequestTransition::Retire,
                ) == Ok(self.slot_model(slot_index))
                &&& self.slots_frame_except(before, slot_index)
                &&& self.slots@[slot_index].generation == before.slots@[slot_index].generation
                &&& self.slots@[slot_index].active_epoch
                    == before.slots@[slot_index].active_epoch
                &&& self.slots@[slot_index].last_quiescent_epoch
                    == before.slots@[slot_index].last_quiescent_epoch
                &&& self.slots@[slot_index].in_free_ring
                    == before.slots@[slot_index].in_free_ring
                &&& (before.slot_model(slot_index) == SequentialRequest {
                    state: RequestState::Ready,
                    phase: LifecyclePhase::Idle,
                } ==> {
                    let tail = ring_position::<C>(before.reclaim_head, before.reclaim_len as nat);
                    &&& self.reclaim_head == before.reclaim_head
                    &&& self.reclaim_len == before.reclaim_len + 1
                    &&& self.reclaim_ring@[tail] == slot_index
                    &&& self.slots@[slot_index].in_reclaim_ring
                    &&& (forall|ring_index: int| 0 <= ring_index < C && ring_index != tail ==>
                        #[trigger] self.reclaim_ring@[ring_index]
                            == before.reclaim_ring@[ring_index])
                })
                &&& (before.slot_model(slot_index) != SequentialRequest {
                    state: RequestState::Ready,
                    phase: LifecyclePhase::Idle,
                } ==> {
                    &&& self.reclaim_ring@ == before.reclaim_ring@
                    &&& self.reclaim_head == before.reclaim_head
                    &&& self.reclaim_len == before.reclaim_len
                    &&& self.slots@[slot_index].in_reclaim_ring
                        == before.slots@[slot_index].in_reclaim_ring
                })
                &&& self.free_ring@ == before.free_ring@
                &&& self.free_head == before.free_head
                &&& self.free_len == before.free_len
                &&& self.member_ring@ == before.member_ring@
                &&& self.member_head == before.member_head
                &&& self.member_len == before.member_len
                &&& self.batch_ring@ == before.batch_ring@
                &&& self.batch_head == before.batch_head
                &&& self.batch_len == before.batch_len
                &&& self.cursor == before.cursor
                &&& self.submitted == before.submitted
                &&& self.completed == before.completed
                &&& self.live_count == before.live_count
            }
        }
    }

    pub closed spec fn dispatch_refines(
        &self,
        before: &Self,
        before_output: Seq<RequestId>,
        output: Seq<RequestId>,
        result: &Result<Option<DispatchBatch>, SchedulerError>,
    ) -> bool {
        let available = C - before.member_len;
        let limit = if before_output.len() < available {
            before_output.len()
        } else {
            available as nat
        };
        let selected = ready_selection::<C>(before.slots@, before.cursor, C as nat, limit);
        match result {
            Err(error) => {
                &&& *error == if before_output.len() == 0 {
                    SchedulerError::EmptyBatchStorage
                } else {
                    SchedulerError::SubmissionEpochExhausted
                }
                &&& (before_output.len() == 0
                    || (before_output.len() > 0 && before.submitted == u64::MAX))
                &&& self.same_scalars(before)
                &&& output == before_output
            }
            Ok(None) => {
                &&& self.same_scalars(before)
                &&& output == before_output
                &&& before_output.len() > 0
                &&& before.submitted < u64::MAX
                &&& (before.batch_len == C || before.member_len == C || selected.len() == 0)
            }
            Ok(Some(batch)) => {
                &&& before_output.len() > 0
                &&& before.submitted < u64::MAX
                &&& before.batch_len < C
                &&& before.member_len < C
                &&& selected.len() > 0
                &&& batch.member_count_spec() == selected.len()
                &&& batch.epoch.value as int == before.submitted as int + 1
                &&& self.submitted == before.submitted + 1
                &&& self.completed == before.completed
                &&& self.live_count == before.live_count
                &&& self.free_ring@ == before.free_ring@
                &&& self.free_head == before.free_head
                &&& self.free_len == before.free_len
                &&& self.reclaim_ring@ == before.reclaim_ring@
                &&& self.reclaim_head == before.reclaim_head
                &&& self.reclaim_len == before.reclaim_len
                &&& self.member_head == before.member_head
                &&& self.member_len == before.member_len + selected.len()
                &&& self.batch_head == before.batch_head
                &&& self.batch_len == before.batch_len + 1
                &&& self.cursor == ready_scan_cursor::<C>(
                    before.slots@,
                    before.cursor,
                    C as nat,
                    limit,
                )
                &&& output.len() == before_output.len()
                &&& (forall|output_index: int|
                    selected.len() <= output_index < output.len() ==>
                        #[trigger] output[output_index] == before_output[output_index])
                &&& (forall|selected_offset: int| 0 <= selected_offset < selected.len() ==> {
                    let slot_index = #[trigger] selected[selected_offset];
                    &&& output[selected_offset].slot_spec() == slot_index
                    &&& output[selected_offset].generation_spec()
                        == before.slots@[slot_index].generation
                    &&& ferric_spec::scheduling::request_transition(
                        before.slot_model(slot_index),
                        RequestTransition::Dispatch,
                    ) == Ok(self.slot_model(slot_index))
                    &&& self.slots@[slot_index].active_epoch == batch.epoch.value
                    &&& self.slots@[slot_index].generation
                        == before.slots@[slot_index].generation
                    &&& self.slots@[slot_index].last_quiescent_epoch
                        == before.slots@[slot_index].last_quiescent_epoch
                    &&& self.slots@[slot_index].in_free_ring
                        == before.slots@[slot_index].in_free_ring
                    &&& self.slots@[slot_index].in_reclaim_ring
                        == before.slots@[slot_index].in_reclaim_ring
                    &&& self.member_ring@[ring_position::<C>(
                        before.member_head,
                        (before.member_len + selected_offset) as nat,
                    )] == output[selected_offset]
                })
                &&& (forall|ring_index: int| 0 <= ring_index < C
                    && !(exists|selected_offset: int| 0 <= selected_offset < selected.len()
                        && (#[trigger] ring_position::<C>(
                            before.member_head,
                            (before.member_len + selected_offset) as nat,
                        )) == ring_index) ==>
                            #[trigger] self.member_ring@[ring_index]
                                == before.member_ring@[ring_index])
                &&& self.batch_ring@[ring_position::<C>(
                    before.batch_head,
                    before.batch_len as nat,
                )].epoch.value == batch.epoch.value
                &&& self.batch_ring@[ring_position::<C>(
                    before.batch_head,
                    before.batch_len as nat,
                )].member_count == selected.len()
                &&& (forall|ring_index: int| 0 <= ring_index < C
                    && ring_index != ring_position::<C>(
                        before.batch_head,
                        before.batch_len as nat,
                    ) ==> #[trigger] self.batch_ring@[ring_index]
                        == before.batch_ring@[ring_index])
                &&& (forall|slot_index: int| 0 <= slot_index < C
                    && !selected.contains(slot_index) ==>
                        #[trigger] self.slots@[slot_index] == before.slots@[slot_index])
            }
        }
    }

    pub closed spec fn dispatch_execution_refines(
        &self,
        before: &Self,
        before_output: Seq<RequestId>,
        output: Seq<RequestId>,
        result: &Result<Option<DispatchBatch>, SchedulerError>,
    ) -> bool {
        let available = C - before.member_len;
        let limit = if before_output.len() < available {
            before_output.len()
        } else {
            available as nat
        };
        let expected = ready_selection::<C>(
            before.slots@,
            before.cursor,
            C as nat,
            limit,
        );
        match result {
            Err(error) => {
                &&& *error == if before_output.len() == 0 {
                    SchedulerError::EmptyBatchStorage
                } else {
                    SchedulerError::SubmissionEpochExhausted
                }
                &&& (before_output.len() == 0
                    || (before_output.len() > 0 && before.submitted == u64::MAX))
                &&& self.same_scalars(before)
                &&& output == before_output
            }
            Ok(None) => {
                &&& before_output.len() > 0
                &&& before.submitted < u64::MAX
                &&& (before.batch_len == C || before.member_len == C
                    || expected.len() == 0)
                &&& self.same_scalars(before)
                &&& output == before_output
            }
            Ok(Some(batch)) => {
                let chosen = output.subrange(0, batch.member_count_spec() as int);
                &&& before_output.len() > 0
                &&& before.submitted < u64::MAX
                &&& before.batch_len < C
                &&& before.member_len < C
                &&& batch.member_count_spec() > 0
                &&& batch.member_count_spec() == chosen.len()
                &&& batch.member_count_spec() == expected.len()
                &&& batch.epoch_spec().value as int == before.submitted as int + 1
                &&& selected_request_slots(chosen) == expected
                &&& before.dispatch_chosen_ready(chosen)
                &&& before.member_len + chosen.len() <= C
                &&& self.dispatch_commit_refines(
                    before,
                    chosen,
                    self.cursor,
                    batch.epoch_spec().value,
                )
                &&& self.cursor == ready_scan_cursor::<C>(
                    before.slots@,
                    before.cursor,
                    C as nat,
                    limit,
                )
                &&& self.cursor < C
                &&& output == dispatch_selected_output(before_output, chosen)
            }
        }
    }

    closed spec fn dispatch_scan_oracle(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        scanned: usize,
        limit: usize,
        slot_index: usize,
        member_tail: usize,
    ) -> bool {
        let chosen_slots = selected_request_slots(chosen);
        let residual = ready_selection::<C>(
            before.slots@,
            slot_index,
            (C - scanned) as nat,
            (limit - chosen.len()) as nat,
        );
        &&& chosen.len() <= scanned
        &&& chosen.len() <= limit
        &&& before.member_len + chosen.len() <= C
        &&& slot_index < C
        &&& member_tail < C
        &&& (scanned < C ==> slot_index as int
            == ring_position::<C>(before.cursor, scanned as nat))
        &&& member_tail as int == ring_position_or_head::<C>(
            before.member_head,
            (before.member_len + chosen.len()) as nat,
        )
        &&& ready_selection::<C>(before.slots@, before.cursor, C as nat, limit as nat)
            == chosen_slots.add(residual)
        &&& ready_scan_cursor::<C>(before.slots@, before.cursor, C as nat, limit as nat)
            == ready_scan_cursor::<C>(
                before.slots@,
                slot_index,
                (C - scanned) as nat,
                (limit - chosen.len()) as nat,
            )
        &&& chosen_slots.no_duplicates()
        &&& (forall|chosen_offset: int| 0 <= chosen_offset < chosen.len() ==> {
            let chosen_request = #[trigger] chosen[chosen_offset];
            let chosen_slot = chosen_request.slot_spec() as int;
            &&& 0 <= chosen_slot < C
            &&& chosen_request.generation_spec() == before.slots@[chosen_slot].generation
            &&& (exists|scan_offset: int| 0 <= scan_offset < scanned
                && chosen_slot
                    == #[trigger] ring_position::<C>(before.cursor, scan_offset as nat))
        })
    }

    closed spec fn dispatch_scan_projection(
        &self,
        before: &Self,
        before_output: Seq<RequestId>,
        output: Seq<RequestId>,
        chosen: Seq<RequestId>,
        next_epoch: u64,
    ) -> bool {
        &&& self.slots@ == dispatch_selected_slots(before.slots@, chosen, next_epoch)
        &&& output == dispatch_selected_output(before_output, chosen)
        &&& self.member_ring@ == dispatch_selected_members::<C>(
            before.member_ring@,
            before.member_head,
            before.member_len,
            chosen,
        )
    }

    closed spec fn dispatch_scan_frames(&self, before: &Self) -> bool {
        &&& self.free_ring@ == before.free_ring@
        &&& self.free_head == before.free_head
        &&& self.free_len == before.free_len
        &&& self.reclaim_ring@ == before.reclaim_ring@
        &&& self.reclaim_head == before.reclaim_head
        &&& self.reclaim_len == before.reclaim_len
        &&& self.member_head == before.member_head
        &&& self.member_len == before.member_len
        &&& self.batch_ring@ == before.batch_ring@
        &&& self.batch_head == before.batch_head
        &&& self.batch_len == before.batch_len
        &&& self.cursor == before.cursor
        &&& self.submitted == before.submitted
        &&& self.completed == before.completed
        &&& self.live_count == before.live_count
    }

    closed spec fn dispatch_commit_refines(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
    ) -> bool {
        let batch_tail = ring_position::<C>(before.batch_head, before.batch_len as nat);
        let batch = BatchRecord {
            epoch: CompletionEpoch { value: next_epoch },
            member_count: chosen.len() as usize,
        };
        &&& chosen.len() > 0
        &&& self.slots@ == dispatch_selected_slots(before.slots@, chosen, next_epoch)
        &&& self.free_ring@ == before.free_ring@
        &&& self.free_head == before.free_head
        &&& self.free_len == before.free_len
        &&& self.reclaim_ring@ == before.reclaim_ring@
        &&& self.reclaim_head == before.reclaim_head
        &&& self.reclaim_len == before.reclaim_len
        &&& self.member_ring@ == dispatch_selected_members::<C>(
            before.member_ring@,
            before.member_head,
            before.member_len,
            chosen,
        )
        &&& self.member_head == before.member_head
        &&& self.member_len == before.member_len + chosen.len()
        &&& self.batch_ring@ == before.batch_ring@.update(batch_tail, batch)
        &&& self.batch_head == before.batch_head
        &&& self.batch_len == before.batch_len + 1
        &&& self.cursor == next_cursor
        &&& self.submitted == next_epoch
        &&& self.completed == before.completed
        &&& self.live_count == before.live_count
    }

    closed spec fn dispatch_chosen_ready(&self, chosen: Seq<RequestId>) -> bool {
        &&& selected_request_slots(chosen).no_duplicates()
        &&& (forall|offset: int| 0 <= offset < chosen.len() ==> {
            let request = #[trigger] chosen[offset];
            let slot_index = request.slot_spec() as int;
            &&& 0 <= slot_index < C
            &&& request.generation_spec() == self.slots@[slot_index].generation
            &&& self.slots@[slot_index].state == RequestState::Ready
        })
    }

    proof fn dispatch_scan_chosen_ready(
        &self,
        before: &Self,
        before_output: Seq<RequestId>,
        output: Seq<RequestId>,
        chosen: Seq<RequestId>,
        scanned: usize,
        limit: usize,
        next_epoch: u64,
        slot_index: usize,
        member_tail: usize,
    )
        requires
            before.basic_invariant(),
            self.dispatch_scan_refines(
                before,
                before_output,
                output,
                chosen,
                scanned,
                limit,
                next_epoch,
                slot_index,
                member_tail,
            ),
            selected_request_slots(chosen) == ready_selection::<C>(
                before.slots@,
                before.cursor,
                C as nat,
                limit as nat,
            ),
        ensures before.dispatch_chosen_ready(chosen),
    {
        before.basic_implies_scalar();
        reveal(Scheduler::scalar_invariant);
        reveal(Scheduler::dispatch_scan_refines);
        reveal(Scheduler::dispatch_scan_oracle);
        reveal(Scheduler::dispatch_chosen_ready);
        assert forall|offset: int| 0 <= offset < chosen.len() implies {
            let request = #[trigger] chosen[offset];
            let selected_slot = selected_request_slots(chosen)[offset];
            let slot_index = request.slot_spec() as int;
            &&& 0 <= slot_index < C
            &&& request.generation_spec() == before.slots@[slot_index].generation
            &&& before.slots@[slot_index].state == RequestState::Ready
        } by {
            reveal(selected_request_slots);
            ready_selection_entry_ready::<C>(
                before.slots@,
                before.cursor,
                C as nat,
                limit as nat,
                offset,
            );
        }
    }

    proof fn dispatch_commit_scalar_counts(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
        ensures
            self.live_count == live_slot_count(self.slots@, C as nat),
            self.reclaim_len + nonreclaim_live_count(self.slots@, C as nat)
                == self.live_count,
            self.free_len + self.live_count == C,
    {
        before.basic_implies_scalar();
        reveal(Scheduler::scalar_invariant);
        reveal(Scheduler::dispatch_chosen_ready);
        reveal(Scheduler::dispatch_commit_refines);
        dispatch_selected_counts_preserved(
            before.slots@,
            chosen,
            next_epoch,
            C as nat,
        );
    }

    proof fn dispatch_commit_scalar_static_bounds(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            chosen.len() <= C - before.member_len,
            next_cursor < C,
        ensures
            C > 0,
            C <= MAX_REQUEST_SLOTS,
            self.free_head < C,
            self.free_len <= C,
            self.reclaim_head < C,
            self.reclaim_len <= C,
            self.member_head < C,
            self.batch_head < C,
            self.cursor < C,
            self.live_count <= C,
            self.reclaim_len <= self.live_count,
    {
        before.basic_implies_scalar();
        reveal(Scheduler::scalar_invariant);
        reveal(Scheduler::dispatch_commit_refines);
    }

    proof fn dispatch_chosen_member_capacity(
        &self,
        chosen: Seq<RequestId>,
    )
        requires
            self.basic_invariant(),
            self.dispatch_chosen_ready(chosen),
        ensures self.member_len + chosen.len() <= self.live_count,
    {
        self.basic_implies_scalar();
        self.basic_implies_member_ring();
        self.member_slot_indices_are_distinct();
        self.member_slot_indices_are_live();
        reveal(Scheduler::scalar_invariant);
        reveal(Scheduler::member_ring_invariant);
        reveal(Scheduler::dispatch_chosen_ready);
        let members = member_slot_indices::<C>(
            self.member_ring@,
            self.member_head,
            self.member_len,
        );
        let selected = selected_request_slots(chosen);
        let combined = members.add(selected);
        let live = live_slot_indices(self.slots@, C as nat);
        live_slot_indices_facts(self.slots@, C as nat);
        assert(selected.len() == chosen.len()) by {
            reveal(selected_request_slots);
        }
        assert forall|offset: int| 0 <= offset < selected.len() implies
            #[trigger] live.contains(selected[offset]) by {
            reveal(selected_request_slots);
        }
        assert forall|offset: int| 0 <= offset < selected.len() implies
            !members.contains(#[trigger] selected[offset]) by {
            let slot_index = selected[offset];
            reveal(selected_request_slots);
            member_slot_indices_contains_iff::<C>(
                self.member_ring@,
                self.member_head,
                self.member_len,
                slot_index,
            );
        }
        assert(combined.no_duplicates()) by {
            reveal(Seq::no_duplicates);
            assert forall|left: int, right: int|
                0 <= left < combined.len()
                    && 0 <= right < combined.len()
                    && left != right implies combined[left] != combined[right] by {
                if left < members.len() && right < members.len() {
                    assert(members[left] != members[right]);
                } else if members.len() <= left && members.len() <= right {
                    assert(selected[left - members.len()] != selected[right - members.len()]);
                } else if left < members.len() {
                    assert(!members.contains(selected[right - members.len()]));
                } else {
                    assert(!members.contains(selected[left - members.len()]));
                }
            }
        }
        assert forall|slot_index: int| combined.contains(slot_index) implies
            #[trigger] live.contains(slot_index) by {
            if !members.contains(slot_index) {
                assert(selected.contains(slot_index));
                let offset = choose|offset: int| 0 <= offset < selected.len()
                    && selected[offset] == slot_index;
            }
        }
        combined.to_set_ensures();
        live.to_set_ensures();
        combined.unique_seq_to_set();
        live.unique_seq_to_set();
        assert(combined.to_set().subset_of(live.to_set()));
        vstd::set_lib::lemma_len_subset(combined.to_set(), live.to_set());
    }

    proof fn dispatch_commit_scalar_member_bounds(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            chosen.len() <= C - before.member_len,
        ensures
            self.member_len <= C,
            self.member_len <= self.live_count,
    {
        before.basic_implies_scalar();
        before.dispatch_chosen_member_capacity(chosen);
        reveal(Scheduler::scalar_invariant);
        reveal(Scheduler::dispatch_commit_refines);
    }

    proof fn dispatch_commit_scalar_batch_bounds(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
    )
        requires
            before.basic_invariant(),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            chosen.len() <= C - before.member_len,
        ensures
            self.batch_len <= C,
            self.batch_len <= self.member_len,
    {
        before.basic_implies_scalar();
        reveal(Scheduler::scalar_invariant);
        reveal(Scheduler::dispatch_commit_refines);
    }

    proof fn dispatch_commit_scalar_epochs(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
    )
        requires
            before.basic_invariant(),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            next_epoch as int == before.submitted as int + 1,
        ensures
            self.completed <= self.submitted,
            self.submitted as int == self.completed as int + self.batch_len as int,
    {
        before.basic_implies_scalar();
        reveal(Scheduler::scalar_invariant);
        reveal(Scheduler::dispatch_commit_refines);
    }

    proof fn dispatch_commit_preserves_scalar(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            chosen.len() <= C - before.member_len,
            next_cursor < C,
            next_epoch as int == before.submitted as int + 1,
        ensures self.scalar_invariant(),
    {
        self.dispatch_commit_scalar_counts(before, chosen, next_cursor, next_epoch);
        self.dispatch_commit_scalar_static_bounds(before, chosen, next_cursor, next_epoch);
        self.dispatch_commit_scalar_member_bounds(before, chosen, next_cursor, next_epoch);
        self.dispatch_commit_scalar_batch_bounds(before, chosen, next_cursor, next_epoch);
        self.dispatch_commit_scalar_epochs(before, chosen, next_cursor, next_epoch);
        reveal(Scheduler::scalar_invariant);
    }

    proof fn dispatch_commit_slot_at(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
        slot_index: int,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            next_epoch as int == before.submitted as int + 1,
            0 <= slot_index < C,
        ensures self.slot_invariant_at(slot_index),
    {
        before.basic_implies_scalar();
        before.basic_implies_slots();
        reveal(Scheduler::scalar_invariant);
        reveal(Scheduler::slot_invariant);
        reveal(Scheduler::slot_invariant_at);
        reveal(Scheduler::dispatch_chosen_ready);
        reveal(Scheduler::dispatch_commit_refines);
        assert forall|offset: int| 0 <= offset < chosen.len() implies
            chosen[offset].slot_spec() < before.slots@.len() by {
            assert(chosen[offset].slot_spec() < C);
        }
        if selected_request_slots(chosen).contains(slot_index) {
            let offset = choose|offset: int|
                0 <= offset < selected_request_slots(chosen).len()
                    && selected_request_slots(chosen)[offset] == slot_index;
            reveal(selected_request_slots);
            dispatch_selected_slots_selected_fact(before.slots@, chosen, next_epoch, offset);
        } else {
            dispatch_selected_slots_frame_fact(
                before.slots@,
                chosen,
                next_epoch,
                slot_index,
            );
        }
    }

    proof fn dispatch_commit_slot_points(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            next_epoch as int == before.submitted as int + 1,
        ensures
            forall|slot_index: int| 0 <= slot_index < C ==>
                #[trigger] self.slot_invariant_at(slot_index),
    {
        assert forall|slot_index: int| 0 <= slot_index < C implies
            #[trigger] self.slot_invariant_at(slot_index) by {
            self.dispatch_commit_slot_at(
                before,
                chosen,
                next_cursor,
                next_epoch,
                slot_index,
            );
        }
    }

    proof fn slot_points_imply_invariant(&self)
        requires
            forall|slot_index: int| 0 <= slot_index < C ==>
                #[trigger] self.slot_invariant_at(slot_index),
        ensures self.slot_invariant(),
    {
        assert forall|slot_index: int| 0 <= slot_index < C implies {
            let slot = #[trigger] self.slots@[slot_index];
            match slot.state {
                RequestState::Vacant => {
                    &&& slot.active_epoch == NO_EPOCH
                    &&& slot.last_quiescent_epoch == NO_EPOCH
                    &&& slot.in_free_ring
                    &&& !slot.in_reclaim_ring
                }
                RequestState::Ready => {
                    &&& slot.active_epoch == NO_EPOCH
                    &&& slot.last_quiescent_epoch <= self.completed
                    &&& !slot.in_free_ring
                    &&& !slot.in_reclaim_ring
                }
                RequestState::InFlight => {
                    &&& NO_EPOCH < slot.active_epoch <= self.submitted
                    &&& slot.last_quiescent_epoch <= self.completed
                    &&& !slot.in_free_ring
                    &&& !slot.in_reclaim_ring
                }
                RequestState::Retiring => {
                    &&& !slot.in_free_ring
                    &&& slot.active_epoch <= self.submitted
                    &&& slot.last_quiescent_epoch <= self.completed
                }
            }
        } by {
            assert(self.slot_invariant_at(slot_index));
            reveal(Scheduler::slot_invariant_at);
        }
        reveal(Scheduler::slot_invariant);
    }

    proof fn dispatch_commit_preserves_slots(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            next_epoch as int == before.submitted as int + 1,
        ensures self.slot_invariant(),
    {
        self.dispatch_commit_slot_points(before, chosen, next_cursor, next_epoch);
        self.slot_points_imply_invariant();
    }

    proof fn dispatch_commit_nonready_frame_at(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
        slot_index: int,
    )
        requires
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            0 <= slot_index < C,
            before.slots@[slot_index].state != RequestState::Ready,
        ensures self.slots@[slot_index] == before.slots@[slot_index],
    {
        reveal(Scheduler::dispatch_chosen_ready);
        reveal(Scheduler::dispatch_commit_refines);
        assert forall|offset: int| 0 <= offset < chosen.len() implies
            chosen[offset].slot_spec() < before.slots@.len() by {
            assert(chosen[offset].slot_spec() < C);
        }
        assert(!selected_request_slots(chosen).contains(slot_index)) by {
            if selected_request_slots(chosen).contains(slot_index) {
                let offset = choose|offset: int|
                    0 <= offset < selected_request_slots(chosen).len()
                        && selected_request_slots(chosen)[offset] == slot_index;
                reveal(selected_request_slots);
            }
        }
        dispatch_selected_slots_frame_fact(
            before.slots@,
            chosen,
            next_epoch,
            slot_index,
        );
    }

    proof fn dispatch_commit_slot_flags_frame_at(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
        slot_index: int,
    )
        requires
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            0 <= slot_index < C,
        ensures
            self.slots@[slot_index].in_free_ring
                == before.slots@[slot_index].in_free_ring,
            self.slots@[slot_index].in_reclaim_ring
                == before.slots@[slot_index].in_reclaim_ring,
    {
        reveal(Scheduler::dispatch_chosen_ready);
        reveal(Scheduler::dispatch_commit_refines);
        assert forall|offset: int| 0 <= offset < chosen.len() implies
            chosen[offset].slot_spec() < before.slots@.len() by {
            assert(chosen[offset].slot_spec() < C);
        }
        if selected_request_slots(chosen).contains(slot_index) {
            let offset = choose|offset: int|
                0 <= offset < selected_request_slots(chosen).len()
                    && selected_request_slots(chosen)[offset] == slot_index;
            reveal(selected_request_slots);
            dispatch_selected_slots_selected_fact(
                before.slots@,
                chosen,
                next_epoch,
                offset,
            );
        } else {
            dispatch_selected_slots_frame_fact(
                before.slots@,
                chosen,
                next_epoch,
                slot_index,
            );
        }
    }

    closed spec fn dispatch_commit_slot_flags_frame_relation(&self, before: &Self) -> bool {
        &&& (forall|slot_index: int| 0 <= slot_index < C ==>
            (#[trigger] self.slots@[slot_index].in_free_ring)
                == before.slots@[slot_index].in_free_ring)
        &&& (forall|slot_index: int| 0 <= slot_index < C ==>
            (#[trigger] self.slots@[slot_index].in_reclaim_ring)
                == before.slots@[slot_index].in_reclaim_ring)
    }

    proof fn dispatch_commit_slot_flags_frame(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
    )
        requires
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
        ensures self.dispatch_commit_slot_flags_frame_relation(before),
    {
        reveal(Scheduler::dispatch_commit_slot_flags_frame_relation);
        assert forall|slot_index: int| 0 <= slot_index < C implies
            (#[trigger] self.slots@[slot_index].in_free_ring)
                == before.slots@[slot_index].in_free_ring by {
            self.dispatch_commit_slot_flags_frame_at(
                before,
                chosen,
                next_cursor,
                next_epoch,
                slot_index,
            );
        }
        assert forall|slot_index: int| 0 <= slot_index < C implies
            (#[trigger] self.slots@[slot_index].in_reclaim_ring)
                == before.slots@[slot_index].in_reclaim_ring by {
            self.dispatch_commit_slot_flags_frame_at(
                before,
                chosen,
                next_cursor,
                next_epoch,
                slot_index,
            );
        }
    }

    proof fn dispatch_commit_free_entries(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
        ensures
            forall|offset: int| 0 <= offset < self.free_len ==> {
                let slot_index = #[trigger] self.free_ring@[
                    ring_position::<C>(self.free_head, offset as nat)
                ];
                &&& slot_index < C
                &&& self.slots@[slot_index as int].state == RequestState::Vacant
                &&& self.slots@[slot_index as int].in_free_ring
            },
    {
        before.basic_implies_free_ring();
        reveal(Scheduler::free_ring_invariant);
        reveal(Scheduler::dispatch_commit_refines);
        assert forall|offset: int| 0 <= offset < self.free_len implies {
            let slot_index = #[trigger] self.free_ring@[
                ring_position::<C>(self.free_head, offset as nat)
            ];
            &&& slot_index < C
            &&& self.slots@[slot_index as int].state == RequestState::Vacant
            &&& self.slots@[slot_index as int].in_free_ring
        } by {
            let slot_index = self.free_ring@[
                ring_position::<C>(self.free_head, offset as nat)
            ];
            self.dispatch_commit_nonready_frame_at(
                before,
                chosen,
                next_cursor,
                next_epoch,
                slot_index as int,
            );
        }
    }

    proof fn dispatch_commit_reclaim_entries(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
        ensures
            forall|offset: int| 0 <= offset < self.reclaim_len ==> {
                let slot_index = #[trigger] self.reclaim_ring@[
                    ring_position::<C>(self.reclaim_head, offset as nat)
                ];
                &&& slot_index < C
                &&& self.slots@[slot_index as int].state == RequestState::Retiring
                &&& self.slots@[slot_index as int].active_epoch == NO_EPOCH
                &&& self.slots@[slot_index as int].in_reclaim_ring
            },
    {
        before.basic_implies_reclaim_ring();
        reveal(Scheduler::reclaim_ring_invariant);
        reveal(Scheduler::dispatch_commit_refines);
        assert forall|offset: int| 0 <= offset < self.reclaim_len implies {
            let slot_index = #[trigger] self.reclaim_ring@[
                ring_position::<C>(self.reclaim_head, offset as nat)
            ];
            &&& slot_index < C
            &&& self.slots@[slot_index as int].state == RequestState::Retiring
            &&& self.slots@[slot_index as int].active_epoch == NO_EPOCH
            &&& self.slots@[slot_index as int].in_reclaim_ring
        } by {
            let slot_index = self.reclaim_ring@[
                ring_position::<C>(self.reclaim_head, offset as nat)
            ];
            self.dispatch_commit_nonready_frame_at(
                before,
                chosen,
                next_cursor,
                next_epoch,
                slot_index as int,
            );
        }
    }

    proof fn dispatch_commit_preserves_free_ring(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
        ensures self.free_ring_invariant(),
    {
        before.basic_implies_free_ring();
        self.dispatch_commit_free_entries(before, chosen, next_cursor, next_epoch);
        self.dispatch_commit_slot_flags_frame(before, chosen, next_cursor, next_epoch);
        reveal(Scheduler::dispatch_commit_slot_flags_frame_relation);
        reveal(Scheduler::dispatch_commit_refines);
        reveal(Scheduler::free_ring_invariant);
    }

    proof fn dispatch_commit_preserves_reclaim_ring(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
        ensures self.reclaim_ring_invariant(),
    {
        self.dispatch_commit_reclaim_entries(before, chosen, next_cursor, next_epoch);
        self.dispatch_commit_reclaim_structure(before, chosen, next_cursor, next_epoch);
        reveal(Scheduler::reclaim_ring_invariant);
    }

    proof fn dispatch_commit_reclaim_structure(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
        ensures
            forall|left: int, right: int|
                0 <= left < self.reclaim_len
                    && 0 <= right < self.reclaim_len
                    && left != right ==>
                        #[trigger] usize_ring_entries_differ::<C>(
                            self.reclaim_ring@,
                            self.reclaim_head,
                            left,
                            right,
                        ),
            forall|slot_index: int| 0 <= slot_index < C ==>
                #[trigger] self.slots@[slot_index].in_reclaim_ring
                    == usize_ring_contains::<C>(
                        self.reclaim_ring@,
                        self.reclaim_head,
                        self.reclaim_len,
                        slot_index,
                    ),
            forall|slot_index: int| 0 <= slot_index < C
                && !(#[trigger] self.slots@[slot_index].in_reclaim_ring) ==>
                    self.reclaim_len < C,
    {
        self.dispatch_commit_reclaim_distinct(before, chosen, next_cursor, next_epoch);
        self.dispatch_commit_reclaim_exact_membership(
            before,
            chosen,
            next_cursor,
            next_epoch,
        );
        self.dispatch_commit_reclaim_capacity(before, chosen, next_cursor, next_epoch);
    }

    proof fn dispatch_commit_reclaim_distinct(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
    )
        requires
            before.basic_invariant(),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
        ensures
            forall|left: int, right: int|
                0 <= left < self.reclaim_len
                    && 0 <= right < self.reclaim_len
                    && left != right ==>
                        #[trigger] usize_ring_entries_differ::<C>(
                            self.reclaim_ring@,
                            self.reclaim_head,
                            left,
                            right,
                        ),
    {
        before.basic_implies_reclaim_ring();
        reveal(Scheduler::dispatch_commit_refines);
        reveal(Scheduler::reclaim_ring_invariant);
    }

    proof fn dispatch_commit_reclaim_exact_membership(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
        ensures
            forall|slot_index: int| 0 <= slot_index < C ==>
                #[trigger] self.slots@[slot_index].in_reclaim_ring
                    == usize_ring_contains::<C>(
                        self.reclaim_ring@,
                        self.reclaim_head,
                        self.reclaim_len,
                        slot_index,
                    ),
    {
        assert forall|slot_index: int| 0 <= slot_index < C implies
            #[trigger] self.slots@[slot_index].in_reclaim_ring
                == usize_ring_contains::<C>(
                    self.reclaim_ring@,
                    self.reclaim_head,
                    self.reclaim_len,
                    slot_index,
                ) by {
            self.dispatch_commit_reclaim_membership_at(
                before,
                chosen,
                next_cursor,
                next_epoch,
                slot_index,
            );
        }
    }

    proof fn dispatch_commit_reclaim_membership_at(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
        slot_index: int,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            0 <= slot_index < C,
        ensures
            self.slots@[slot_index].in_reclaim_ring
                == usize_ring_contains::<C>(
                    self.reclaim_ring@,
                    self.reclaim_head,
                    self.reclaim_len,
                    slot_index,
                ),
    {
        before.basic_implies_reclaim_ring();
        self.dispatch_commit_slot_flags_frame_at(
            before,
            chosen,
            next_cursor,
            next_epoch,
            slot_index,
        );
        reveal(Scheduler::dispatch_commit_refines);
        reveal(Scheduler::reclaim_ring_invariant);
    }

    proof fn dispatch_commit_reclaim_capacity(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
        ensures self.reclaim_len < C,
    {
        before.basic_implies_reclaim_ring();
        reveal(Scheduler::dispatch_chosen_ready);
        reveal(Scheduler::dispatch_commit_refines);
        reveal(Scheduler::reclaim_ring_invariant);
        let selected = chosen[0].slot_spec() as int;
        assert(0 <= selected < C);
        assert(!before.slots@[selected].in_reclaim_ring);
    }

    proof fn dispatch_commit_member_shape(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
    )
        requires self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
        ensures
            self.member_head == before.member_head,
            self.member_len == before.member_len + chosen.len(),
            self.submitted == next_epoch,
            self.completed == before.completed,
    {
        reveal(Scheduler::dispatch_commit_refines);
    }

    proof fn dispatch_commit_member_handle_at(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
        offset: int,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            before.member_len + chosen.len() <= C,
            0 <= offset < self.member_len,
        ensures {
            let handle = self.member_ring@[
                ring_position::<C>(self.member_head, offset as nat)
            ];
            if offset < before.member_len {
                handle == before.member_ring@[
                    ring_position::<C>(before.member_head, offset as nat)
                ]
            } else {
                handle == chosen[offset - before.member_len]
            }
        },
    {
        self.dispatch_commit_member_shape(before, chosen, next_cursor, next_epoch);
        if offset < before.member_len {
            self.dispatch_commit_old_member_handle_at(
                before,
                chosen,
                next_cursor,
                next_epoch,
                offset,
            );
        } else {
            self.dispatch_commit_selected_member_handle_at(
                before,
                chosen,
                next_cursor,
                next_epoch,
                offset - before.member_len,
            );
        }
    }

    proof fn dispatch_commit_old_member_handle_at(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
        offset: int,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            before.member_len + chosen.len() <= C,
            0 <= offset < before.member_len,
        ensures
            self.member_ring@[ring_position::<C>(self.member_head, offset as nat)]
                == before.member_ring@[
                    ring_position::<C>(before.member_head, offset as nat)
                ],
    {
        before.basic_implies_scalar();
        reveal(Scheduler::scalar_invariant);
        reveal(Scheduler::dispatch_chosen_ready);
        reveal(Scheduler::dispatch_commit_refines);
        assert forall|chosen_offset: int| 0 <= chosen_offset < chosen.len() implies
            chosen[chosen_offset].slot_spec() < C by {
        }
        let ring_index = ring_position::<C>(before.member_head, offset as nat);
        assert(!(exists|chosen_offset: int| 0 <= chosen_offset < chosen.len()
            && #[trigger] ring_position::<C>(
                before.member_head,
                (before.member_len + chosen_offset) as nat,
            ) == ring_index)) by {
            if exists|chosen_offset: int| 0 <= chosen_offset < chosen.len()
                && #[trigger] ring_position::<C>(
                    before.member_head,
                    (before.member_len + chosen_offset) as nat,
                ) == ring_index {
                let chosen_offset = choose|chosen_offset: int|
                    0 <= chosen_offset < chosen.len()
                        && #[trigger] ring_position::<C>(
                            before.member_head,
                            (before.member_len + chosen_offset) as nat,
                        ) == ring_index;
                ring_position_injective::<C>(
                    before.member_head,
                    offset as nat,
                    (before.member_len + chosen_offset) as nat,
                );
            }
        }
        dispatch_selected_members_frame_fact::<C>(
            before.member_ring@,
            before.member_head,
            before.member_len,
            chosen,
            ring_index,
        );
    }

    proof fn dispatch_commit_selected_member_handle_at(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
        chosen_offset: int,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            before.member_len + chosen.len() <= C,
            0 <= chosen_offset < chosen.len(),
        ensures
            self.member_ring@[
                ring_position::<C>(
                    self.member_head,
                    (before.member_len + chosen_offset) as nat,
                )
            ] == chosen[chosen_offset],
    {
        before.basic_implies_scalar();
        reveal(Scheduler::scalar_invariant);
        reveal(Scheduler::dispatch_chosen_ready);
        reveal(Scheduler::dispatch_commit_refines);
        assert forall|offset: int| 0 <= offset < chosen.len() implies
            chosen[offset].slot_spec() < C by {
        }
        dispatch_selected_members_selected_fact::<C>(
            before.member_ring@,
            before.member_head,
            before.member_len,
            chosen,
            chosen_offset,
        );
    }

    proof fn dispatch_commit_old_member_entry_at(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
        offset: int,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            before.member_len + chosen.len() <= C,
            next_epoch as int == before.submitted as int + 1,
            0 <= offset < before.member_len,
        ensures {
            let handle = self.member_ring@[
                ring_position::<C>(self.member_head, offset as nat)
            ];
            &&& handle.slot_spec() < C
            &&& self.slots@[handle.slot_spec() as int].generation
                == handle.generation_spec()
            &&& self.completed < self.slots@[handle.slot_spec() as int].active_epoch
            &&& self.slots@[handle.slot_spec() as int].active_epoch <= self.submitted
            &&& !self.slots@[handle.slot_spec() as int].in_reclaim_ring
            &&& (self.slots@[handle.slot_spec() as int].state == RequestState::InFlight
                || self.slots@[handle.slot_spec() as int].state == RequestState::Retiring)
        },
    {
        before.member_entry_facts(offset as usize);
        self.dispatch_commit_old_member_handle_at(
            before,
            chosen,
            next_cursor,
            next_epoch,
            offset,
        );
        let old_handle = before.member_ring@[
            ring_position::<C>(before.member_head, offset as nat)
        ];
        self.dispatch_commit_nonready_frame_at(
            before,
            chosen,
            next_cursor,
            next_epoch,
            old_handle.slot_spec() as int,
        );
        self.dispatch_commit_member_shape(before, chosen, next_cursor, next_epoch);
    }

    proof fn dispatch_commit_selected_member_entry_at(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
        chosen_offset: int,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            before.member_len + chosen.len() <= C,
            next_epoch as int == before.submitted as int + 1,
            0 <= chosen_offset < chosen.len(),
        ensures {
            let offset = before.member_len + chosen_offset;
            let handle = self.member_ring@[
                ring_position::<C>(self.member_head, offset as nat)
            ];
            &&& handle.slot_spec() < C
            &&& self.slots@[handle.slot_spec() as int].generation
                == handle.generation_spec()
            &&& self.completed < self.slots@[handle.slot_spec() as int].active_epoch
            &&& self.slots@[handle.slot_spec() as int].active_epoch <= self.submitted
            &&& !self.slots@[handle.slot_spec() as int].in_reclaim_ring
            &&& (self.slots@[handle.slot_spec() as int].state == RequestState::InFlight
                || self.slots@[handle.slot_spec() as int].state == RequestState::Retiring)
        },
    {
        before.basic_implies_scalar();
        self.dispatch_commit_selected_member_handle_at(
            before,
            chosen,
            next_cursor,
            next_epoch,
            chosen_offset,
        );
        reveal(Scheduler::scalar_invariant);
        reveal(Scheduler::dispatch_chosen_ready);
        reveal(Scheduler::dispatch_commit_refines);
        assert forall|offset: int| 0 <= offset < chosen.len() implies
            chosen[offset].slot_spec() < before.slots@.len() by {
            assert(chosen[offset].slot_spec() < C);
        }
        reveal(selected_request_slots);
        dispatch_selected_slots_selected_fact(
            before.slots@,
            chosen,
            next_epoch,
            chosen_offset,
        );
    }

    proof fn dispatch_commit_member_entry_at(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
        offset: int,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            before.member_len + chosen.len() <= C,
            next_epoch as int == before.submitted as int + 1,
            0 <= offset < self.member_len,
        ensures {
            let handle = self.member_ring@[
                ring_position::<C>(self.member_head, offset as nat)
            ];
            &&& handle.slot_spec() < C
            &&& self.slots@[handle.slot_spec() as int].generation
                == handle.generation_spec()
            &&& self.completed < self.slots@[handle.slot_spec() as int].active_epoch
            &&& self.slots@[handle.slot_spec() as int].active_epoch <= self.submitted
            &&& !self.slots@[handle.slot_spec() as int].in_reclaim_ring
            &&& (self.slots@[handle.slot_spec() as int].state == RequestState::InFlight
                || self.slots@[handle.slot_spec() as int].state == RequestState::Retiring)
        },
    {
        self.dispatch_commit_member_shape(before, chosen, next_cursor, next_epoch);
        if offset < before.member_len {
            self.dispatch_commit_old_member_entry_at(
                before,
                chosen,
                next_cursor,
                next_epoch,
                offset,
            );
        } else {
            let chosen_offset = offset - before.member_len;
            self.dispatch_commit_selected_member_entry_at(
                before,
                chosen,
                next_cursor,
                next_epoch,
                chosen_offset,
            );
        }
    }

    proof fn dispatch_commit_member_entries(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            before.member_len + chosen.len() <= C,
            next_epoch as int == before.submitted as int + 1,
        ensures
            forall|offset: int| 0 <= offset < self.member_len ==> {
                let handle = #[trigger] self.member_ring@[
                    ring_position::<C>(self.member_head, offset as nat)
                ];
                &&& handle.slot_spec() < C
                &&& self.slots@[handle.slot_spec() as int].generation
                    == handle.generation_spec()
                &&& self.completed < self.slots@[handle.slot_spec() as int].active_epoch
                &&& self.slots@[handle.slot_spec() as int].active_epoch <= self.submitted
                &&& !self.slots@[handle.slot_spec() as int].in_reclaim_ring
                &&& (self.slots@[handle.slot_spec() as int].state == RequestState::InFlight
                    || self.slots@[handle.slot_spec() as int].state == RequestState::Retiring)
            },
    {
        assert forall|offset: int| 0 <= offset < self.member_len implies {
            let handle = #[trigger] self.member_ring@[
                ring_position::<C>(self.member_head, offset as nat)
            ];
            &&& handle.slot_spec() < C
            &&& self.slots@[handle.slot_spec() as int].generation
                == handle.generation_spec()
            &&& self.completed < self.slots@[handle.slot_spec() as int].active_epoch
            &&& self.slots@[handle.slot_spec() as int].active_epoch <= self.submitted
            &&& !self.slots@[handle.slot_spec() as int].in_reclaim_ring
            &&& (self.slots@[handle.slot_spec() as int].state == RequestState::InFlight
                || self.slots@[handle.slot_spec() as int].state == RequestState::Retiring)
        } by {
            self.dispatch_commit_member_entry_at(
                before,
                chosen,
                next_cursor,
                next_epoch,
                offset,
            );
        }
    }

    proof fn dispatch_commit_old_members_differ(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
        left: int,
        right: int,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            before.member_len + chosen.len() <= C,
            0 <= left < before.member_len,
            0 <= right < before.member_len,
            left != right,
        ensures
            self.member_ring@[ring_position::<C>(self.member_head, left as nat)].slot_spec()
                != self.member_ring@[
                    ring_position::<C>(self.member_head, right as nat)
                ].slot_spec(),
    {
        before.basic_implies_member_ring();
        self.dispatch_commit_old_member_handle_at(
            before,
            chosen,
            next_cursor,
            next_epoch,
            left,
        );
        self.dispatch_commit_old_member_handle_at(
            before,
            chosen,
            next_cursor,
            next_epoch,
            right,
        );
        reveal(Scheduler::member_ring_invariant);
        assert(request_ring_slots_differ::<C>(
            before.member_ring@,
            before.member_head,
            left,
            right,
        ));
        reveal(request_ring_slots_differ);
        reveal(Scheduler::dispatch_commit_refines);
    }

    proof fn dispatch_commit_selected_members_differ(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
        left: int,
        right: int,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            before.member_len + chosen.len() <= C,
            0 <= left < chosen.len(),
            0 <= right < chosen.len(),
            left != right,
        ensures
            self.member_ring@[
                ring_position::<C>(self.member_head, (before.member_len + left) as nat)
            ].slot_spec()
                != self.member_ring@[
                    ring_position::<C>(self.member_head, (before.member_len + right) as nat)
                ].slot_spec(),
    {
        before.basic_implies_scalar();
        reveal(Scheduler::scalar_invariant);
        reveal(Scheduler::dispatch_chosen_ready);
        reveal(Scheduler::dispatch_commit_refines);
        assert forall|offset: int| 0 <= offset < chosen.len() implies
            chosen[offset].slot_spec() < C by {
        }
        dispatch_selected_members_selected_slots_differ::<C>(
            before.member_ring@,
            before.member_head,
            before.member_len,
            chosen,
            left,
            right,
        );
    }

    proof fn dispatch_commit_old_selected_members_differ(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
        old_offset: int,
        chosen_offset: int,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            before.member_len + chosen.len() <= C,
            0 <= old_offset < before.member_len,
            0 <= chosen_offset < chosen.len(),
        ensures
            self.member_ring@[
                ring_position::<C>(self.member_head, old_offset as nat)
            ].slot_spec()
                != self.member_ring@[
                    ring_position::<C>(
                        self.member_head,
                        (before.member_len + chosen_offset) as nat,
                    )
                ].slot_spec(),
    {
        before.basic_implies_member_ring();
        self.dispatch_commit_old_member_handle_at(
            before,
            chosen,
            next_cursor,
            next_epoch,
            old_offset,
        );
        self.dispatch_commit_selected_member_handle_at(
            before,
            chosen,
            next_cursor,
            next_epoch,
            chosen_offset,
        );
        reveal(Scheduler::member_ring_invariant);
        reveal(Scheduler::dispatch_chosen_ready);
        let old_handle = before.member_ring@[
            ring_position::<C>(before.member_head, old_offset as nat)
        ];
        let selected = chosen[chosen_offset];
        assert(old_handle.slot_spec() != selected.slot_spec());
    }

    proof fn dispatch_commit_old_member_distinct_summary(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            before.member_len + chosen.len() <= C,
        ensures
            forall|left: int, right: int|
                0 <= left < before.member_len
                    && 0 <= right < before.member_len
                    && left != right ==>
                        #[trigger] request_ring_slots_differ::<C>(
                            self.member_ring@,
                            self.member_head,
                            left,
                            right,
                        ),
    {
        assert forall|left: int, right: int|
            0 <= left < before.member_len
                && 0 <= right < before.member_len
                && left != right implies
                    #[trigger] request_ring_slots_differ::<C>(
                        self.member_ring@,
                        self.member_head,
                        left,
                        right,
                    ) by {
            self.dispatch_commit_old_members_differ(
                before,
                chosen,
                next_cursor,
                next_epoch,
                left,
                right,
            );
            reveal(request_ring_slots_differ);
        }
    }

    proof fn dispatch_commit_selected_member_distinct_summary(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            before.member_len + chosen.len() <= C,
        ensures
            forall|left: int, right: int|
                0 <= left < chosen.len()
                    && 0 <= right < chosen.len()
                    && left != right ==>
                        #[trigger] request_ring_slots_differ::<C>(
                            self.member_ring@,
                            self.member_head,
                            before.member_len + left,
                            before.member_len + right,
                        ),
    {
        assert forall|left: int, right: int|
            0 <= left < chosen.len()
                && 0 <= right < chosen.len()
                && left != right implies
                    #[trigger] request_ring_slots_differ::<C>(
                        self.member_ring@,
                        self.member_head,
                        before.member_len + left,
                        before.member_len + right,
                    ) by {
            self.dispatch_commit_selected_members_differ(
                before,
                chosen,
                next_cursor,
                next_epoch,
                left,
                right,
            );
            reveal(request_ring_slots_differ);
        }
    }

    proof fn dispatch_commit_cross_member_distinct_summary(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            before.member_len + chosen.len() <= C,
        ensures
            forall|old_offset: int, chosen_offset: int|
                0 <= old_offset < before.member_len
                    && 0 <= chosen_offset < chosen.len() ==>
                        #[trigger] request_ring_slots_differ::<C>(
                            self.member_ring@,
                            self.member_head,
                            old_offset,
                            before.member_len + chosen_offset,
                        ),
    {
        assert forall|old_offset: int, chosen_offset: int|
            0 <= old_offset < before.member_len
                && 0 <= chosen_offset < chosen.len() implies
                    #[trigger] request_ring_slots_differ::<C>(
                        self.member_ring@,
                        self.member_head,
                        old_offset,
                        before.member_len + chosen_offset,
                    ) by {
            self.dispatch_commit_old_selected_members_differ(
                before,
                chosen,
                next_cursor,
                next_epoch,
                old_offset,
                chosen_offset,
            );
            reveal(request_ring_slots_differ);
        }
    }

    proof fn dispatch_commit_member_distinct(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            before.member_len + chosen.len() <= C,
        ensures
            forall|left: int, right: int|
                0 <= left < self.member_len
                    && 0 <= right < self.member_len
                    && left != right ==>
                        #[trigger] request_ring_slots_differ::<C>(
                            self.member_ring@,
                            self.member_head,
                            left,
                            right,
                        ),
    {
        reveal(Scheduler::dispatch_commit_refines);
        assert forall|left: int, right: int|
            0 <= left < self.member_len
                && 0 <= right < self.member_len
                && left != right implies
                    #[trigger] request_ring_slots_differ::<C>(
                        self.member_ring@,
                        self.member_head,
                        left,
                        right,
                    ) by {
            if left < before.member_len && right < before.member_len {
                self.dispatch_commit_old_members_differ(
                    before,
                    chosen,
                    next_cursor,
                    next_epoch,
                    left,
                    right,
                );
                reveal(request_ring_slots_differ);
            } else if before.member_len <= left && before.member_len <= right {
                self.dispatch_commit_selected_members_differ(
                    before,
                    chosen,
                    next_cursor,
                    next_epoch,
                    left - before.member_len,
                    right - before.member_len,
                );
                reveal(request_ring_slots_differ);
            } else if left < before.member_len {
                self.dispatch_commit_old_selected_members_differ(
                    before,
                    chosen,
                    next_cursor,
                    next_epoch,
                    left,
                    right - before.member_len,
                );
                reveal(request_ring_slots_differ);
            } else {
                self.dispatch_commit_old_selected_members_differ(
                    before,
                    chosen,
                    next_cursor,
                    next_epoch,
                    right,
                    left - before.member_len,
                );
                reveal(request_ring_slots_differ);
            }
        }
    }

    proof fn dispatch_commit_member_contains_at(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
        slot_index: int,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            before.member_len + chosen.len() <= C,
        ensures
            request_ring_contains_slot::<C>(
                self.member_ring@,
                self.member_head,
                self.member_len,
                slot_index,
            ) == (request_ring_contains_slot::<C>(
                before.member_ring@,
                before.member_head,
                before.member_len,
                slot_index,
            ) || selected_request_slots(chosen).contains(slot_index)),
    {
        reveal(Scheduler::dispatch_commit_refines);
        reveal(request_ring_contains_slot);
        reveal(selected_request_slots);
        if request_ring_contains_slot::<C>(
            self.member_ring@,
            self.member_head,
            self.member_len,
            slot_index,
        ) {
            let offset = choose|offset: int| 0 <= offset < self.member_len
                && (#[trigger] self.member_ring@[
                    ring_position::<C>(self.member_head, offset as nat)
                ].slot_spec()) == slot_index;
            self.dispatch_commit_member_handle_at(
                before,
                chosen,
                next_cursor,
                next_epoch,
                offset,
            );
            if offset < before.member_len {
                assert(request_ring_contains_slot::<C>(
                    before.member_ring@,
                    before.member_head,
                    before.member_len,
                    slot_index,
                )) by {
                    let old_offset = offset;
                }
            } else {
                assert(selected_request_slots(chosen).contains(slot_index)) by {
                    let selected_offset = offset - before.member_len;
                    assert(0 <= selected_offset < chosen.len());
                    assert(selected_request_slots(chosen)[selected_offset]
                        == chosen[selected_offset].slot_spec() as int);
                    assert(selected_request_slots(chosen)[selected_offset] == slot_index);
                }
            }
        }
        if request_ring_contains_slot::<C>(
            before.member_ring@,
            before.member_head,
            before.member_len,
            slot_index,
        ) {
            let offset = choose|offset: int| 0 <= offset < before.member_len
                && (#[trigger] before.member_ring@[
                    ring_position::<C>(before.member_head, offset as nat)
                ].slot_spec()) == slot_index;
            self.dispatch_commit_member_handle_at(
                before,
                chosen,
                next_cursor,
                next_epoch,
                offset,
            );
            assert(request_ring_contains_slot::<C>(
                self.member_ring@,
                self.member_head,
                self.member_len,
                slot_index,
            )) by {
                let new_offset = offset;
            }
        }
        if selected_request_slots(chosen).contains(slot_index) {
            let chosen_offset = choose|chosen_offset: int|
                0 <= chosen_offset < selected_request_slots(chosen).len()
                    && selected_request_slots(chosen)[chosen_offset] == slot_index;
            self.dispatch_commit_member_handle_at(
                before,
                chosen,
                next_cursor,
                next_epoch,
                before.member_len + chosen_offset,
            );
            assert(request_ring_contains_slot::<C>(
                self.member_ring@,
                self.member_head,
                self.member_len,
                slot_index,
            )) by {
                let new_offset = before.member_len + chosen_offset;
            }
        }
    }

    proof fn dispatch_commit_member_membership_at(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
        slot_index: int,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            before.member_len + chosen.len() <= C,
            next_epoch as int == before.submitted as int + 1,
            0 <= slot_index < C,
        ensures
            ((self.slots@[slot_index].state == RequestState::InFlight
                || self.slots@[slot_index].state == RequestState::Retiring)
                && self.completed < self.slots@[slot_index].active_epoch)
                == request_ring_contains_slot::<C>(
                    self.member_ring@,
                    self.member_head,
                    self.member_len,
                    slot_index,
                ),
    {
        before.basic_implies_scalar();
        before.basic_implies_member_ring();
        self.dispatch_commit_member_contains_at(
            before,
            chosen,
            next_cursor,
            next_epoch,
            slot_index,
        );
        reveal(Scheduler::scalar_invariant);
        reveal(Scheduler::member_ring_invariant);
        reveal(Scheduler::dispatch_chosen_ready);
        reveal(Scheduler::dispatch_commit_refines);
        assert forall|offset: int| 0 <= offset < chosen.len() implies
            chosen[offset].slot_spec() < before.slots@.len() by {
            assert(chosen[offset].slot_spec() < C);
        }
        if selected_request_slots(chosen).contains(slot_index) {
            let chosen_offset = choose|chosen_offset: int|
                0 <= chosen_offset < selected_request_slots(chosen).len()
                    && selected_request_slots(chosen)[chosen_offset] == slot_index;
            reveal(selected_request_slots);
            dispatch_selected_slots_selected_fact(
                before.slots@,
                chosen,
                next_epoch,
                chosen_offset,
            );
        } else {
            dispatch_selected_slots_frame_fact(
                before.slots@,
                chosen,
                next_epoch,
                slot_index,
            );
        }
    }

    proof fn dispatch_commit_member_exact_membership(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            before.member_len + chosen.len() <= C,
            next_epoch as int == before.submitted as int + 1,
        ensures
            forall|slot_index: int| 0 <= slot_index < C ==>
                (((#[trigger] self.slots@[slot_index].state == RequestState::InFlight
                    || self.slots@[slot_index].state == RequestState::Retiring)
                    && self.completed < self.slots@[slot_index].active_epoch)
                    == request_ring_contains_slot::<C>(
                        self.member_ring@,
                        self.member_head,
                        self.member_len,
                        slot_index,
                    )),
    {
        assert forall|slot_index: int| 0 <= slot_index < C implies
            (((#[trigger] self.slots@[slot_index].state == RequestState::InFlight
                || self.slots@[slot_index].state == RequestState::Retiring)
                && self.completed < self.slots@[slot_index].active_epoch)
                == request_ring_contains_slot::<C>(
                    self.member_ring@,
                    self.member_head,
                    self.member_len,
                    slot_index,
                )) by {
            self.dispatch_commit_member_membership_at(
                before,
                chosen,
                next_cursor,
                next_epoch,
                slot_index,
            );
        }
    }

    proof fn dispatch_commit_preserves_member_ring(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            before.member_len + chosen.len() <= C,
            next_epoch as int == before.submitted as int + 1,
        ensures self.member_ring_invariant(),
    {
        self.dispatch_commit_member_entries(before, chosen, next_cursor, next_epoch);
        self.dispatch_commit_member_distinct(before, chosen, next_cursor, next_epoch);
        self.dispatch_commit_member_exact_membership(
            before,
            chosen,
            next_cursor,
            next_epoch,
        );
        reveal(Scheduler::member_ring_invariant);
    }

    proof fn dispatch_commit_batch_sum(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
    )
        requires
            before.basic_invariant(),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            before.member_len + chosen.len() <= C,
        ensures
            batch_member_sum::<C>(
                self.batch_ring@,
                self.batch_head,
                self.batch_len as nat,
            ) == self.member_len,
    {
        before.basic_implies_scalar();
        before.basic_implies_batch_ring();
        reveal(Scheduler::scalar_invariant);
        reveal(Scheduler::batch_ring_invariant);
        reveal(Scheduler::dispatch_commit_refines);
        let batch = BatchRecord {
            epoch: CompletionEpoch { value: next_epoch },
            member_count: chosen.len() as usize,
        };
        batch_member_sum_append::<C>(
            before.batch_ring@,
            before.batch_head,
            before.batch_len,
            batch,
        );
    }

    proof fn dispatch_commit_old_batch_record_at(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
        batch_offset: int,
    )
        requires
            before.basic_invariant(),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            before.member_len + chosen.len() <= C,
            0 <= batch_offset < before.batch_len,
        ensures
            self.batch_ring@[
                ring_position::<C>(self.batch_head, batch_offset as nat)
            ] == before.batch_ring@[
                ring_position::<C>(before.batch_head, batch_offset as nat)
            ],
    {
        before.basic_implies_scalar();
        reveal(Scheduler::scalar_invariant);
        reveal(Scheduler::dispatch_commit_refines);
        let tail = ring_position::<C>(before.batch_head, before.batch_len as nat);
        ring_position_injective::<C>(
            before.batch_head,
            batch_offset as nat,
            before.batch_len as nat,
        );
        assert(ring_position::<C>(before.batch_head, batch_offset as nat) != tail);
    }

    proof fn dispatch_commit_old_batch_sum_prefix(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
        count: nat,
    )
        requires
            before.basic_invariant(),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            before.member_len + chosen.len() <= C,
            count <= before.batch_len,
        ensures
            batch_member_sum::<C>(self.batch_ring@, self.batch_head, count)
                == batch_member_sum::<C>(before.batch_ring@, before.batch_head, count),
    {
        before.basic_implies_scalar();
        reveal(Scheduler::scalar_invariant);
        reveal(Scheduler::dispatch_commit_refines);
        let tail = ring_position::<C>(before.batch_head, before.batch_len as nat);
        ring_position_bounds::<C>(before.batch_head, before.batch_len as nat);
        assert forall|offset: int| 0 <= offset < count implies
            #[trigger] ring_position::<C>(before.batch_head, offset as nat) != tail by {
            ring_position_injective::<C>(
                before.batch_head,
                offset as nat,
                before.batch_len as nat,
            );
        }
        batch_member_sum_update_frame::<C>(
            before.batch_ring@,
            before.batch_head,
            count,
            tail,
            BatchRecord {
                epoch: CompletionEpoch { value: next_epoch },
                member_count: chosen.len() as usize,
            },
        );
    }

    proof fn dispatch_commit_old_batch_header_at(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
        batch_offset: int,
    )
        requires
            before.basic_invariant(),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            before.member_len + chosen.len() <= C,
            next_epoch as int == before.submitted as int + 1,
            0 <= batch_offset < before.batch_len,
        ensures {
            let batch = self.batch_ring@[
                ring_position::<C>(self.batch_head, batch_offset as nat)
            ];
            &&& batch.member_count > 0
            &&& batch.epoch.value as int == self.completed as int + batch_offset + 1
            &&& batch.epoch.value <= self.submitted
        },
    {
        before.basic_implies_scalar();
        before.basic_batch_entry_header_facts(batch_offset);
        self.dispatch_commit_old_batch_record_at(
            before,
            chosen,
            next_cursor,
            next_epoch,
            batch_offset,
        );
        reveal(Scheduler::scalar_invariant);
        reveal(Scheduler::dispatch_commit_refines);
    }

    proof fn dispatch_commit_old_batch_member_epoch_at(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
        batch_offset: int,
        member_offset: int,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            before.member_len + chosen.len() <= C,
            0 <= batch_offset < before.batch_len,
            batch_member_sum::<C>(
                self.batch_ring@,
                self.batch_head,
                batch_offset as nat,
            ) <= member_offset < batch_member_sum::<C>(
                self.batch_ring@,
                self.batch_head,
                batch_offset as nat + 1,
            ),
        ensures {
            let batch = self.batch_ring@[
                ring_position::<C>(self.batch_head, batch_offset as nat)
            ];
            let handle = self.member_ring@[
                ring_position::<C>(self.member_head, member_offset as nat)
            ];
            self.slots@[handle.slot_spec() as int].active_epoch == batch.epoch.value
        },
    {
        self.dispatch_commit_old_batch_member_range_at(
            before,
            chosen,
            next_cursor,
            next_epoch,
            batch_offset,
            member_offset,
        );
        before.basic_batch_member_epoch_fact(batch_offset, member_offset);
        before.member_entry_facts(member_offset as usize);
        self.dispatch_commit_old_batch_record_at(
            before,
            chosen,
            next_cursor,
            next_epoch,
            batch_offset,
        );
        self.dispatch_commit_old_member_handle_at(
            before,
            chosen,
            next_cursor,
            next_epoch,
            member_offset,
        );
        let old_handle = before.member_ring@[
            ring_position::<C>(before.member_head, member_offset as nat)
        ];
        self.dispatch_commit_nonready_frame_at(
            before,
            chosen,
            next_cursor,
            next_epoch,
            old_handle.slot_spec() as int,
        );
    }

    proof fn dispatch_commit_old_batch_member_range_at(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
        batch_offset: int,
        member_offset: int,
    )
        requires
            before.basic_invariant(),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            before.member_len + chosen.len() <= C,
            0 <= batch_offset < before.batch_len,
            batch_member_sum::<C>(
                self.batch_ring@,
                self.batch_head,
                batch_offset as nat,
            ) <= member_offset < batch_member_sum::<C>(
                self.batch_ring@,
                self.batch_head,
                batch_offset as nat + 1,
            ),
        ensures
            batch_member_sum::<C>(
                before.batch_ring@,
                before.batch_head,
                batch_offset as nat,
            ) <= member_offset < batch_member_sum::<C>(
                before.batch_ring@,
                before.batch_head,
                batch_offset as nat + 1,
            ),
            0 <= member_offset < before.member_len,
    {
        before.basic_implies_scalar();
        before.basic_implies_batch_ring();
        self.dispatch_commit_old_batch_sum_prefix(
            before,
            chosen,
            next_cursor,
            next_epoch,
            batch_offset as nat,
        );
        self.dispatch_commit_old_batch_sum_prefix(
            before,
            chosen,
            next_cursor,
            next_epoch,
            batch_offset as nat + 1,
        );
        batch_member_sum_monotonic::<C>(
            before.batch_ring@,
            before.batch_head,
            batch_offset as nat + 1,
            before.batch_len as nat,
        );
        reveal(Scheduler::scalar_invariant);
        reveal(Scheduler::batch_ring_invariant);
    }

    proof fn dispatch_commit_old_batch_epoch_members_at(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
        batch_offset: int,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            before.member_len + chosen.len() <= C,
            0 <= batch_offset < before.batch_len,
        ensures {
            let batch = self.batch_ring@[
                ring_position::<C>(self.batch_head, batch_offset as nat)
            ];
            forall|member_offset: int|
                batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    batch_offset as nat,
                ) <= member_offset < batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    batch_offset as nat + 1,
                ) ==> {
                    let handle = #[trigger] self.member_ring@[
                        ring_position::<C>(self.member_head, member_offset as nat)
                    ];
                    self.slots@[handle.slot_spec() as int].active_epoch == batch.epoch.value
                }
        },
    {
        assert forall|member_offset: int|
            batch_member_sum::<C>(
                self.batch_ring@,
                self.batch_head,
                batch_offset as nat,
            ) <= member_offset < batch_member_sum::<C>(
                self.batch_ring@,
                self.batch_head,
                batch_offset as nat + 1,
            ) implies {
                let handle = #[trigger] self.member_ring@[
                    ring_position::<C>(self.member_head, member_offset as nat)
                ];
                self.slots@[handle.slot_spec() as int].active_epoch
                    == self.batch_ring@[
                        ring_position::<C>(self.batch_head, batch_offset as nat)
                    ].epoch.value
        } by {
            self.dispatch_commit_old_batch_member_epoch_at(
                before,
                chosen,
                next_cursor,
                next_epoch,
                batch_offset,
                member_offset,
            );
        }
    }

    proof fn dispatch_commit_old_batch_entry_at(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
        batch_offset: int,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            before.member_len + chosen.len() <= C,
            next_epoch as int == before.submitted as int + 1,
            0 <= batch_offset < before.batch_len,
        ensures {
            let batch = self.batch_ring@[
                ring_position::<C>(self.batch_head, batch_offset as nat)
            ];
            &&& batch.member_count > 0
            &&& batch.epoch.value as int == self.completed as int + batch_offset + 1
            &&& batch.epoch.value <= self.submitted
            &&& (forall|member_offset: int|
                batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    batch_offset as nat,
                ) <= member_offset < batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    batch_offset as nat + 1,
                ) ==> {
                    let handle = #[trigger] self.member_ring@[
                        ring_position::<C>(self.member_head, member_offset as nat)
                    ];
                    self.slots@[handle.slot_spec() as int].active_epoch == batch.epoch.value
                })
        },
    {
        self.dispatch_commit_old_batch_header_at(
            before,
            chosen,
            next_cursor,
            next_epoch,
            batch_offset,
        );
        self.dispatch_commit_old_batch_epoch_members_at(
            before,
            chosen,
            next_cursor,
            next_epoch,
            batch_offset,
        );
    }

    proof fn dispatch_commit_old_batch_entries(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            before.member_len + chosen.len() <= C,
            next_epoch as int == before.submitted as int + 1,
        ensures
            forall|batch_offset: int| 0 <= batch_offset < before.batch_len ==> {
                let batch = #[trigger] self.batch_ring@[
                    ring_position::<C>(self.batch_head, batch_offset as nat)
                ];
                &&& batch.member_count > 0
                &&& batch.epoch.value as int == self.completed as int + batch_offset + 1
                &&& batch.epoch.value <= self.submitted
                &&& (forall|member_offset: int|
                    batch_member_sum::<C>(
                        self.batch_ring@,
                        self.batch_head,
                        batch_offset as nat,
                    ) <= member_offset < batch_member_sum::<C>(
                        self.batch_ring@,
                        self.batch_head,
                        batch_offset as nat + 1,
                    ) ==> {
                        let handle = #[trigger] self.member_ring@[
                            ring_position::<C>(self.member_head, member_offset as nat)
                        ];
                        self.slots@[handle.slot_spec() as int].active_epoch == batch.epoch.value
                    })
            },
    {
        assert forall|batch_offset: int| 0 <= batch_offset < before.batch_len implies {
            let batch = #[trigger] self.batch_ring@[
                ring_position::<C>(self.batch_head, batch_offset as nat)
            ];
            &&& batch.member_count > 0
            &&& batch.epoch.value as int == self.completed as int + batch_offset + 1
            &&& batch.epoch.value <= self.submitted
            &&& (forall|member_offset: int|
                batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    batch_offset as nat,
                ) <= member_offset < batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    batch_offset as nat + 1,
                ) ==> {
                    let handle = #[trigger] self.member_ring@[
                        ring_position::<C>(self.member_head, member_offset as nat)
                    ];
                    self.slots@[handle.slot_spec() as int].active_epoch == batch.epoch.value
                })
        } by {
            self.dispatch_commit_old_batch_entry_at(
                before,
                chosen,
                next_cursor,
                next_epoch,
                batch_offset,
            );
        }
    }

    proof fn dispatch_commit_new_batch_header(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
    )
        requires
            before.basic_invariant(),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            before.member_len + chosen.len() <= C,
            next_epoch as int == before.submitted as int + 1,
        ensures {
            let batch = self.batch_ring@[
                ring_position::<C>(self.batch_head, before.batch_len as nat)
            ];
            &&& batch.member_count > 0
            &&& batch.epoch.value as int
                == self.completed as int + before.batch_len as int + 1
            &&& batch.epoch.value <= self.submitted
        },
    {
        before.basic_implies_scalar();
        reveal(Scheduler::scalar_invariant);
        reveal(Scheduler::dispatch_commit_refines);
    }

    proof fn dispatch_commit_new_batch_member_offset(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
        member_offset: int,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            before.member_len + chosen.len() <= C,
            batch_member_sum::<C>(
                self.batch_ring@,
                self.batch_head,
                before.batch_len as nat,
            ) <= member_offset < batch_member_sum::<C>(
                self.batch_ring@,
                self.batch_head,
                before.batch_len as nat + 1,
            ),
        ensures
            before.member_len <= member_offset < before.member_len + chosen.len(),
            0 <= member_offset - before.member_len < chosen.len(),
    {
        before.basic_implies_batch_ring();
        self.dispatch_commit_old_batch_sum_prefix(
            before,
            chosen,
            next_cursor,
            next_epoch,
            before.batch_len as nat,
        );
        self.dispatch_commit_batch_sum(before, chosen, next_cursor, next_epoch);
        reveal(Scheduler::batch_ring_invariant);
        reveal(Scheduler::dispatch_commit_refines);
    }

    proof fn dispatch_commit_new_batch_member_epoch_at(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
        member_offset: int,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            before.member_len + chosen.len() <= C,
            next_epoch as int == before.submitted as int + 1,
            batch_member_sum::<C>(
                self.batch_ring@,
                self.batch_head,
                before.batch_len as nat,
            ) <= member_offset < batch_member_sum::<C>(
                self.batch_ring@,
                self.batch_head,
                before.batch_len as nat + 1,
            ),
        ensures {
            let batch = self.batch_ring@[
                ring_position::<C>(self.batch_head, before.batch_len as nat)
            ];
            let handle = self.member_ring@[
                ring_position::<C>(self.member_head, member_offset as nat)
            ];
            self.slots@[handle.slot_spec() as int].active_epoch == batch.epoch.value
        },
    {
        before.basic_implies_scalar();
        self.dispatch_commit_new_batch_member_offset(
            before,
            chosen,
            next_cursor,
            next_epoch,
            member_offset,
        );
        reveal(Scheduler::scalar_invariant);
        reveal(Scheduler::dispatch_chosen_ready);
        reveal(Scheduler::dispatch_commit_refines);
        let chosen_offset = member_offset - before.member_len;
        self.dispatch_commit_selected_member_handle_at(
            before,
            chosen,
            next_cursor,
            next_epoch,
            chosen_offset,
        );
        assert forall|offset: int| 0 <= offset < chosen.len() implies
            chosen[offset].slot_spec() < before.slots@.len() by {
            assert(chosen[offset].slot_spec() < C);
        }
        reveal(selected_request_slots);
        dispatch_selected_slots_selected_fact(
            before.slots@,
            chosen,
            next_epoch,
            chosen_offset,
        );
    }

    proof fn dispatch_commit_new_batch_epoch_members(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            before.member_len + chosen.len() <= C,
            next_epoch as int == before.submitted as int + 1,
        ensures {
            let batch = self.batch_ring@[
                ring_position::<C>(self.batch_head, before.batch_len as nat)
            ];
            forall|member_offset: int|
                batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    before.batch_len as nat,
                ) <= member_offset < batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    before.batch_len as nat + 1,
                ) ==> {
                    let handle = #[trigger] self.member_ring@[
                        ring_position::<C>(self.member_head, member_offset as nat)
                    ];
                    self.slots@[handle.slot_spec() as int].active_epoch == batch.epoch.value
                }
        },
    {
        assert forall|member_offset: int|
            batch_member_sum::<C>(
                self.batch_ring@,
                self.batch_head,
                before.batch_len as nat,
            ) <= member_offset < batch_member_sum::<C>(
                self.batch_ring@,
                self.batch_head,
                before.batch_len as nat + 1,
            ) implies {
                let handle = #[trigger] self.member_ring@[
                    ring_position::<C>(self.member_head, member_offset as nat)
                ];
                self.slots@[handle.slot_spec() as int].active_epoch
                    == self.batch_ring@[
                        ring_position::<C>(self.batch_head, before.batch_len as nat)
                    ].epoch.value
        } by {
            self.dispatch_commit_new_batch_member_epoch_at(
                before,
                chosen,
                next_cursor,
                next_epoch,
                member_offset,
            );
        }
    }

    proof fn dispatch_commit_new_batch_entry(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            before.member_len + chosen.len() <= C,
            next_epoch as int == before.submitted as int + 1,
        ensures {
            let batch = self.batch_ring@[
                ring_position::<C>(self.batch_head, before.batch_len as nat)
            ];
            &&& batch.member_count > 0
            &&& batch.epoch.value as int
                == self.completed as int + before.batch_len as int + 1
            &&& batch.epoch.value <= self.submitted
            &&& (forall|member_offset: int|
                batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    before.batch_len as nat,
                ) <= member_offset < batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    before.batch_len as nat + 1,
                ) ==> {
                    let handle = #[trigger] self.member_ring@[
                        ring_position::<C>(self.member_head, member_offset as nat)
                    ];
                    self.slots@[handle.slot_spec() as int].active_epoch == batch.epoch.value
                })
        },
    {
        self.dispatch_commit_new_batch_header(before, chosen, next_cursor, next_epoch);
        self.dispatch_commit_new_batch_epoch_members(
            before,
            chosen,
            next_cursor,
            next_epoch,
        );
    }

    proof fn dispatch_commit_batch_entries(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            before.member_len + chosen.len() <= C,
            next_epoch as int == before.submitted as int + 1,
        ensures
            forall|batch_offset: int| 0 <= batch_offset < self.batch_len ==> {
                let batch = #[trigger] self.batch_ring@[
                    ring_position::<C>(self.batch_head, batch_offset as nat)
                ];
                &&& batch.member_count > 0
                &&& batch.epoch.value as int == self.completed as int + batch_offset + 1
                &&& batch.epoch.value <= self.submitted
                &&& (forall|member_offset: int|
                    batch_member_sum::<C>(
                        self.batch_ring@,
                        self.batch_head,
                        batch_offset as nat,
                    ) <= member_offset < batch_member_sum::<C>(
                        self.batch_ring@,
                        self.batch_head,
                        batch_offset as nat + 1,
                    ) ==> {
                        let handle = #[trigger] self.member_ring@[
                            ring_position::<C>(self.member_head, member_offset as nat)
                        ];
                        self.slots@[handle.slot_spec() as int].active_epoch == batch.epoch.value
                    })
            },
    {
        self.dispatch_commit_old_batch_entries(
            before,
            chosen,
            next_cursor,
            next_epoch,
        );
        self.dispatch_commit_new_batch_entry(before, chosen, next_cursor, next_epoch);
        reveal(Scheduler::dispatch_commit_refines);
        assert forall|batch_offset: int| 0 <= batch_offset < self.batch_len implies {
            let batch = #[trigger] self.batch_ring@[
                ring_position::<C>(self.batch_head, batch_offset as nat)
            ];
            &&& batch.member_count > 0
            &&& batch.epoch.value as int == self.completed as int + batch_offset + 1
            &&& batch.epoch.value <= self.submitted
            &&& (forall|member_offset: int|
                batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    batch_offset as nat,
                ) <= member_offset < batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    batch_offset as nat + 1,
                ) ==> {
                    let handle = #[trigger] self.member_ring@[
                        ring_position::<C>(self.member_head, member_offset as nat)
                    ];
                    self.slots@[handle.slot_spec() as int].active_epoch == batch.epoch.value
                })
        } by {
            if before.batch_len <= batch_offset {
                assert(batch_offset == before.batch_len);
            }
        }
    }

    proof fn dispatch_commit_preserves_batch_ring(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            before.member_len + chosen.len() <= C,
            next_epoch as int == before.submitted as int + 1,
        ensures self.batch_ring_invariant(),
    {
        self.dispatch_commit_batch_sum(before, chosen, next_cursor, next_epoch);
        self.dispatch_commit_batch_entries(before, chosen, next_cursor, next_epoch);
        reveal(Scheduler::batch_ring_invariant);
    }

    proof fn dispatch_commit_preserves_basic(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        next_cursor: usize,
        next_epoch: u64,
    )
        requires
            before.basic_invariant(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(before, chosen, next_cursor, next_epoch),
            before.member_len + chosen.len() <= C,
            next_cursor < C,
            next_epoch as int == before.submitted as int + 1,
        ensures self.basic_invariant(),
    {
        self.dispatch_commit_preserves_scalar(
            before,
            chosen,
            next_cursor,
            next_epoch,
        );
        self.dispatch_commit_preserves_slots(before, chosen, next_cursor, next_epoch);
        self.dispatch_commit_preserves_free_ring(before, chosen, next_cursor, next_epoch);
        self.dispatch_commit_preserves_reclaim_ring(
            before,
            chosen,
            next_cursor,
            next_epoch,
        );
        self.dispatch_commit_preserves_member_ring(
            before,
            chosen,
            next_cursor,
            next_epoch,
        );
        self.dispatch_commit_preserves_batch_ring(
            before,
            chosen,
            next_cursor,
            next_epoch,
        );
        reveal(Scheduler::basic_invariant);
    }

    closed spec fn dispatch_scan_refines(
        &self,
        before: &Self,
        before_output: Seq<RequestId>,
        output: Seq<RequestId>,
        chosen: Seq<RequestId>,
        scanned: usize,
        limit: usize,
        next_epoch: u64,
        slot_index: usize,
        member_tail: usize,
    ) -> bool {
        &&& self.dispatch_scan_oracle(
            before,
            chosen,
            scanned,
            limit,
            slot_index,
            member_tail,
        )
        &&& self.dispatch_scan_projection(
            before,
            before_output,
            output,
            chosen,
            next_epoch,
        )
        &&& self.dispatch_scan_frames(before)
    }

    proof fn dispatch_scan_oracle_init(
        &self,
        limit: usize,
        member_tail: usize,
    )
        requires
            self.basic_invariant(),
            self.member_len < C,
            limit <= C - self.member_len,
            member_tail as int
                == ring_position::<C>(self.member_head, self.member_len as nat),
        ensures
            self.dispatch_scan_oracle(
                self,
                Seq::empty(),
                0,
                limit,
                self.cursor,
                member_tail,
            ),
    {
        self.basic_implies_scalar();
        assert(self.cursor < C) by {
            reveal(Scheduler::scalar_invariant);
        }
        assert(member_tail < C) by {
            reveal(Scheduler::scalar_invariant);
            ring_position_bounds::<C>(self.member_head, self.member_len as nat);
        }
        assert(self.cursor as int == ring_position::<C>(self.cursor, 0)) by {
            reveal(Scheduler::scalar_invariant);
            ring_position_bounds::<C>(self.cursor, 0);
        }
        assert(member_tail as int == ring_position_or_head::<C>(
            self.member_head,
            (self.member_len + Seq::<RequestId>::empty().len()) as nat,
        )) by {
            reveal(Scheduler::scalar_invariant);
            reveal(ring_position_or_head);
        }
        selected_request_slots_empty();
        assert(ready_selection::<C>(self.slots@, self.cursor, C as nat, limit as nat)
            == selected_request_slots(Seq::<RequestId>::empty()).add(
                ready_selection::<C>(self.slots@, self.cursor, C as nat, limit as nat),
            ));
        assert(selected_request_slots(Seq::<RequestId>::empty()).no_duplicates()) by {
            reveal(Seq::no_duplicates);
        }
        reveal(Scheduler::dispatch_scan_oracle);
    }

    proof fn dispatch_scan_projection_init(
        &self,
        output: Seq<RequestId>,
        next_epoch: u64,
    )
        requires self.basic_invariant(),
        ensures
            self.dispatch_scan_projection(
                self,
                output,
                output,
                Seq::empty(),
                next_epoch,
            ),
    {
        self.basic_implies_scalar();
        reveal(Scheduler::scalar_invariant);
        dispatch_selected_slots_empty(self.slots@, next_epoch);
        dispatch_selected_output_empty(output);
        dispatch_selected_members_empty::<C>(
            self.member_ring@,
            self.member_head,
            self.member_len,
        );
        reveal(Scheduler::dispatch_scan_projection);
    }

    proof fn dispatch_scan_frames_init(&self)
        ensures self.dispatch_scan_frames(self),
    {
        reveal(Scheduler::dispatch_scan_frames);
    }

    proof fn dispatch_scan_init(
        &self,
        output: Seq<RequestId>,
        limit: usize,
        next_epoch: u64,
        member_tail: usize,
    )
        requires
            self.basic_invariant(),
            self.member_len < C,
            limit <= output.len(),
            limit <= C - self.member_len,
            member_tail as int
                == ring_position::<C>(self.member_head, self.member_len as nat),
        ensures
            self.dispatch_scan_refines(
                self,
                output,
                output,
                Seq::empty(),
                0,
                limit,
                next_epoch,
                self.cursor,
                member_tail,
            ),
    {
        self.dispatch_scan_oracle_init(limit, member_tail);
        self.dispatch_scan_projection_init(output, next_epoch);
        self.dispatch_scan_frames_init();
        reveal(Scheduler::dispatch_scan_refines);
    }

    proof fn dispatch_scan_current_not_chosen(
        &self,
        before: &Self,
        before_output: Seq<RequestId>,
        output: Seq<RequestId>,
        chosen: Seq<RequestId>,
        scanned: usize,
        limit: usize,
        next_epoch: u64,
        slot_index: usize,
        member_tail: usize,
    )
        requires
            C > 0,
            before.cursor < C,
            self.dispatch_scan_refines(
                before,
                before_output,
                output,
                chosen,
                scanned,
                limit,
                next_epoch,
                slot_index,
                member_tail,
            ),
            scanned < C,
        ensures
            !selected_request_slots(chosen).contains(slot_index as int),
            self.slots@[slot_index as int] == before.slots@[slot_index as int],
    {
        reveal(Scheduler::dispatch_scan_refines);
        reveal(Scheduler::dispatch_scan_oracle);
        reveal(Scheduler::dispatch_scan_projection);
        let chosen_slots = selected_request_slots(chosen);
        if chosen_slots.contains(slot_index as int) {
            let chosen_offset = choose|chosen_offset: int|
                0 <= chosen_offset < chosen.len()
                    && chosen[chosen_offset].slot_spec() as int == slot_index as int;
            let scan_offset = choose|scan_offset: int|
                0 <= scan_offset < scanned
                    && chosen[chosen_offset].slot_spec() as int
                        == #[trigger] ring_position::<C>(before.cursor, scan_offset as nat);
            ring_position_injective::<C>(
                before.cursor,
                scan_offset as nat,
                scanned as nat,
            );
            assert(false);
        }
        assert forall|chosen_offset: int| 0 <= chosen_offset < chosen.len() implies
            chosen[chosen_offset].slot_spec() < before.slots@.len() by {
            let chosen_slot = chosen[chosen_offset].slot_spec() as int;
            assert(0 <= chosen_slot < C);
        }
        dispatch_selected_slots_frame_fact(
            before.slots@,
            chosen,
            next_epoch,
            slot_index as int,
        );
    }

    proof fn dispatch_ready_selection_step(
        before: &Self,
        chosen: Seq<RequestId>,
        scanned: usize,
        limit: usize,
        slot_index: usize,
        handle: RequestId,
    )
        requires
            scanned < C,
            chosen.len() < limit,
            before.slots@[slot_index as int].state == RequestState::Ready,
            handle.slot_spec() == slot_index,
            ready_selection::<C>(before.slots@, before.cursor, C as nat, limit as nat)
                == selected_request_slots(chosen).add(ready_selection::<C>(
                    before.slots@,
                    slot_index,
                    (C - scanned) as nat,
                    (limit - chosen.len()) as nat,
                )),
        ensures
            ready_selection::<C>(before.slots@, before.cursor, C as nat, limit as nat)
                == selected_request_slots(chosen.push(handle)).add(ready_selection::<C>(
                    before.slots@,
                    next_position::<C>(slot_index),
                    (C - (scanned + 1)) as nat,
                    (limit - chosen.push(handle).len()) as nat,
                )),
    {
        reveal(ready_selection);
        selected_request_slots_push(chosen, handle);
        assert(selected_request_slots(chosen.push(handle))
            == selected_request_slots(chosen).add(seq![slot_index as int])) by {
            assert(selected_request_slots(chosen).push(slot_index as int)
                == selected_request_slots(chosen).add(seq![slot_index as int]));
        }
    }

    proof fn dispatch_ready_cursor_step(
        before: &Self,
        chosen: Seq<RequestId>,
        scanned: usize,
        limit: usize,
        slot_index: usize,
        handle: RequestId,
    )
        requires
            scanned < C,
            chosen.len() < limit,
            before.slots@[slot_index as int].state == RequestState::Ready,
            ready_scan_cursor::<C>(before.slots@, before.cursor, C as nat, limit as nat)
                == ready_scan_cursor::<C>(
                    before.slots@,
                    slot_index,
                    (C - scanned) as nat,
                    (limit - chosen.len()) as nat,
                ),
        ensures
            ready_scan_cursor::<C>(before.slots@, before.cursor, C as nat, limit as nat)
                == ready_scan_cursor::<C>(
                    before.slots@,
                    next_position::<C>(slot_index),
                    (C - (scanned + 1)) as nat,
                    (limit - chosen.push(handle).len()) as nat,
                ),
    {
        reveal(ready_scan_cursor);
    }

    proof fn selected_slots_push_preserves_unique(
        chosen: Seq<RequestId>,
        handle: RequestId,
    )
        requires
            selected_request_slots(chosen).no_duplicates(),
            !selected_request_slots(chosen).contains(handle.slot_spec() as int),
        ensures selected_request_slots(chosen.push(handle)).no_duplicates(),
    {
        selected_request_slots_push(chosen, handle);
        let chosen_slots = selected_request_slots(chosen);
        reveal(Seq::no_duplicates);
        assert forall|left: int, right: int|
            0 <= left < selected_request_slots(chosen.push(handle)).len()
                && 0 <= right < selected_request_slots(chosen.push(handle)).len()
                && left != right implies
                    selected_request_slots(chosen.push(handle))[left]
                        != selected_request_slots(chosen.push(handle))[right] by {
            if left < chosen_slots.len() && right < chosen_slots.len() {
                assert(chosen_slots[left] != chosen_slots[right]);
            } else if left == chosen_slots.len() {
                assert(selected_request_slots(chosen.push(handle))[left]
                    == handle.slot_spec() as int);
                assert(selected_request_slots(chosen.push(handle))[right] == chosen_slots[right]);
            } else {
                assert(right == chosen_slots.len());
                assert(selected_request_slots(chosen.push(handle))[left] == chosen_slots[left]);
                assert(selected_request_slots(chosen.push(handle))[right]
                    == handle.slot_spec() as int);
            }
        }
    }

    proof fn dispatch_chosen_facts_push(
        before: &Self,
        chosen: Seq<RequestId>,
        scanned: usize,
        slot_index: usize,
        handle: RequestId,
    )
        requires
            C > 0,
            before.cursor < C,
            scanned < C,
            slot_index < C,
            slot_index as int == ring_position::<C>(before.cursor, scanned as nat),
            handle.slot_spec() == slot_index,
            handle.generation_spec() == before.slots@[slot_index as int].generation,
            forall|chosen_offset: int| 0 <= chosen_offset < chosen.len() ==> {
                let chosen_request = #[trigger] chosen[chosen_offset];
                let chosen_slot = chosen_request.slot_spec() as int;
                &&& 0 <= chosen_slot < C
                &&& chosen_request.generation_spec()
                    == before.slots@[chosen_slot].generation
                &&& (exists|scan_offset: int| 0 <= scan_offset < scanned
                    && chosen_slot
                        == #[trigger] ring_position::<C>(
                            before.cursor,
                            scan_offset as nat,
                        ))
            },
        ensures
            forall|chosen_offset: int| 0 <= chosen_offset < chosen.push(handle).len() ==> {
                let chosen_request = #[trigger] chosen.push(handle)[chosen_offset];
                let chosen_slot = chosen_request.slot_spec() as int;
                &&& 0 <= chosen_slot < C
                &&& chosen_request.generation_spec()
                    == before.slots@[chosen_slot].generation
                &&& (exists|scan_offset: int| 0 <= scan_offset < scanned + 1
                    && chosen_slot
                        == #[trigger] ring_position::<C>(
                            before.cursor,
                            scan_offset as nat,
                        ))
            },
    {
        assert forall|chosen_offset: int|
            0 <= chosen_offset < chosen.push(handle).len() implies {
                let chosen_request = #[trigger] chosen.push(handle)[chosen_offset];
                let chosen_slot = chosen_request.slot_spec() as int;
                &&& 0 <= chosen_slot < C
                &&& chosen_request.generation_spec()
                    == before.slots@[chosen_slot].generation
                &&& (exists|scan_offset: int| 0 <= scan_offset < scanned + 1
                    && chosen_slot
                        == #[trigger] ring_position::<C>(
                            before.cursor,
                            scan_offset as nat,
                        ))
        } by {
            if chosen_offset < chosen.len() {
                assert(chosen.push(handle)[chosen_offset] == chosen[chosen_offset]);
                assert({
                    let chosen_request = chosen[chosen_offset];
                    let chosen_slot = chosen_request.slot_spec() as int;
                    &&& 0 <= chosen_slot < C
                    &&& chosen_request.generation_spec()
                        == before.slots@[chosen_slot].generation
                    &&& (exists|scan_offset: int| 0 <= scan_offset < scanned
                        && chosen_slot
                            == #[trigger] ring_position::<C>(
                                before.cursor,
                                scan_offset as nat,
                            ))
                });
            } else {
                assert(chosen_offset == chosen.len());
                assert(chosen.push(handle)[chosen_offset] == handle);
                assert(exists|scan_offset: int| 0 <= scan_offset < scanned + 1
                    && handle.slot_spec() as int
                        == #[trigger] ring_position::<C>(
                            before.cursor,
                            scan_offset as nat,
                        )) by {
                    let scan_offset = scanned as int;
                    assert(0 <= scan_offset < scanned + 1);
                    assert(scan_offset as nat == scanned as nat);
                    assert(handle.slot_spec() as int == slot_index as int);
                    assert(slot_index as int
                        == ring_position::<C>(before.cursor, scanned as nat));
                    assert(handle.slot_spec() as int
                        == ring_position::<C>(before.cursor, scanned as nat));
                }
            }
        }
    }

    proof fn dispatch_scan_oracle_ready_step(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        scanned: usize,
        limit: usize,
        slot_index: usize,
        member_tail: usize,
        handle: RequestId,
    )
        requires
            C > 0,
            before.cursor < C,
            before.member_head < C,
            self.dispatch_scan_oracle(
                before,
                chosen,
                scanned,
                limit,
                slot_index,
                member_tail,
            ),
            scanned < C,
            chosen.len() < limit,
            limit <= C - before.member_len,
            before.slots@[slot_index as int].state == RequestState::Ready,
            !selected_request_slots(chosen).contains(slot_index as int),
            handle.slot_spec() == slot_index,
            handle.generation_spec() == before.slots@[slot_index as int].generation,
        ensures
            self.dispatch_scan_oracle(
                before,
                chosen.push(handle),
                (scanned + 1) as usize,
                limit,
                next_position::<C>(slot_index),
                next_position::<C>(member_tail),
            ),
    {
        reveal(Scheduler::dispatch_scan_oracle);
        Self::dispatch_ready_selection_step(
            before,
            chosen,
            scanned,
            limit,
            slot_index,
            handle,
        );
        Self::dispatch_ready_cursor_step(before, chosen, scanned, limit, slot_index, handle);
        Self::selected_slots_push_preserves_unique(chosen, handle);
        ring_position_or_head_next::<C>(
            before.member_head,
            (before.member_len + chosen.len()) as nat,
        );
        if scanned + 1 < C {
            ring_position_next::<C>(before.cursor, scanned as nat);
        }
        Self::dispatch_chosen_facts_push(before, chosen, scanned, slot_index, handle);
    }

    proof fn dispatch_scan_oracle_skip_step(
        &self,
        before: &Self,
        chosen: Seq<RequestId>,
        scanned: usize,
        limit: usize,
        slot_index: usize,
        member_tail: usize,
    )
        requires
            C > 0,
            before.cursor < C,
            self.dispatch_scan_oracle(
                before,
                chosen,
                scanned,
                limit,
                slot_index,
                member_tail,
            ),
            scanned < C,
            chosen.len() < limit,
            before.slots@[slot_index as int].state != RequestState::Ready,
        ensures
            self.dispatch_scan_oracle(
                before,
                chosen,
                (scanned + 1) as usize,
                limit,
                next_position::<C>(slot_index),
                member_tail,
            ),
    {
        reveal(Scheduler::dispatch_scan_oracle);
        reveal(ready_selection);
        reveal(ready_scan_cursor);
        if scanned + 1 < C {
            ring_position_next::<C>(before.cursor, scanned as nat);
        }
    }

    proof fn dispatch_scan_projection_ready_step(
        &self,
        before: &Self,
        before_output: Seq<RequestId>,
        prior_output: Seq<RequestId>,
        prior_slots: Seq<Slot>,
        prior_members: Seq<RequestId>,
        chosen: Seq<RequestId>,
        next_epoch: u64,
        slot_index: usize,
        member_tail: usize,
        handle: RequestId,
    )
        requires
            C > 0,
            before.member_head < C,
            prior_slots == dispatch_selected_slots(before.slots@, chosen, next_epoch),
            prior_output == dispatch_selected_output(before_output, chosen),
            prior_members == dispatch_selected_members::<C>(
                before.member_ring@,
                before.member_head,
                before.member_len,
                chosen,
            ),
            prior_slots[slot_index as int] == before.slots@[slot_index as int],
            self.slots@ == prior_slots.update(
                slot_index as int,
                Slot {
                    state: RequestState::InFlight,
                    active_epoch: next_epoch,
                    ..prior_slots[slot_index as int]
                },
            ),
            self.member_ring@ == prior_members.update(member_tail as int, handle),
            prior_output.update(chosen.len() as int, handle)
                == dispatch_selected_output(before_output, chosen.push(handle)),
            member_tail as int == ring_position::<C>(
                before.member_head,
                (before.member_len + chosen.len()) as nat,
            ),
            before.member_len + chosen.len() < C,
            handle.slot_spec() == slot_index,
            handle.slot_spec() < C,
            forall|offset: int| 0 <= offset < chosen.len() ==>
                chosen[offset].slot_spec() < C,
        ensures
            self.slots@
                == dispatch_selected_slots(before.slots@, chosen.push(handle), next_epoch),
            self.member_ring@ == dispatch_selected_members::<C>(
                before.member_ring@,
                before.member_head,
                before.member_len,
                chosen.push(handle),
            ),
    {
        dispatch_selected_slots_push(before.slots@, chosen, next_epoch, handle);
        dispatch_selected_members_push::<C>(
            before.member_ring@,
            before.member_head,
            before.member_len,
            chosen,
            handle,
        );
    }

    proof fn dispatch_scan_finished(
        &self,
        before: &Self,
        before_output: Seq<RequestId>,
        output: Seq<RequestId>,
        chosen: Seq<RequestId>,
        scanned: usize,
        limit: usize,
        next_epoch: u64,
        slot_index: usize,
        member_tail: usize,
    )
        requires
            self.dispatch_scan_refines(
                before,
                before_output,
                output,
                chosen,
                scanned,
                limit,
                next_epoch,
                slot_index,
                member_tail,
            ),
            scanned == C || chosen.len() == limit,
        ensures
            selected_request_slots(chosen)
                == ready_selection::<C>(
                    before.slots@,
                    before.cursor,
                    C as nat,
                    limit as nat,
                ),
            chosen.len() == ready_selection::<C>(
                before.slots@,
                before.cursor,
                C as nat,
                limit as nat,
            ).len(),
            slot_index == ready_scan_cursor::<C>(
                before.slots@,
                before.cursor,
                C as nat,
                limit as nat,
            ),
    {
        reveal(Scheduler::dispatch_scan_refines);
        reveal(Scheduler::dispatch_scan_oracle);
        reveal(ready_selection);
        reveal(ready_scan_cursor);
        assert(ready_selection::<C>(
            before.slots@,
            slot_index,
            (C - scanned) as nat,
            (limit - chosen.len()) as nat,
        ) == Seq::<int>::empty());
        assert(ready_scan_cursor::<C>(
            before.slots@,
            slot_index,
            (C - scanned) as nat,
            (limit - chosen.len()) as nat,
        ) == slot_index);
        assert(selected_request_slots(chosen).add(Seq::<int>::empty())
            == selected_request_slots(chosen));
        reveal(selected_request_slots);
    }

    pub(crate) closed spec fn completion_refines(
        &self,
        before: &Self,
        completion_epoch: u64,
        before_permits: Seq<Option<KvQuiescencePermit>>,
        permits: Seq<Option<KvQuiescencePermit>>,
        result: &Result<usize, CompletionFailure>,
    ) -> bool {
        match result {
            Err(failure) => {
                &&& Some(failure.error_spec()) == completion_expected_error::<C>(
                    before,
                    completion_epoch,
                    before_permits,
                )
                &&& failure.completion_epoch_spec().value == completion_epoch
                &&& self.same_scalars(before)
                &&& permits == before_permits
            }
            Ok(count) => {
                let batch = before.batch_ring@[before.batch_head as int];
                &&& completion_expected_error::<C>(
                    before,
                    completion_epoch,
                    before_permits,
                ).is_none()
                &&& before.batch_len > 0
                &&& completion_epoch == before.completed + 1
                &&& batch.epoch.value == completion_epoch
                &&& *count == batch.member_count
                &&& *count <= permits.len()
                &&& self.completed == completion_epoch
                &&& self.submitted == before.submitted
                &&& self.slots@ == before.slots@
                &&& self.member_len + *count == before.member_len
                &&& self.member_head
                    == ring_advance::<C>(before.member_head, *count as nat)
                &&& self.member_ring@ == before.member_ring@
                &&& self.batch_len + 1 == before.batch_len
                &&& self.batch_head == next_position::<C>(before.batch_head)
                &&& self.batch_ring@ == before.batch_ring@
                &&& self.free_ring@ == before.free_ring@
                &&& self.free_head == before.free_head
                &&& self.free_len == before.free_len
                &&& self.reclaim_ring@ == before.reclaim_ring@
                &&& self.reclaim_head == before.reclaim_head
                &&& self.reclaim_len == before.reclaim_len
                &&& self.cursor == before.cursor
                &&& self.live_count == before.live_count
                &&& permits.len() == before_permits.len()
                &&& (forall|permit_index: int| *count <= permit_index < permits.len() ==>
                    #[trigger] permits[permit_index] == before_permits[permit_index])
                &&& (forall|member_offset: int| 0 <= member_offset < *count ==> {
                    let request = #[trigger] before.member_ring@[
                        ring_position::<C>(before.member_head, member_offset as nat)
                    ];
                    match permits[member_offset] {
                        Some(permit) => {
                            &&& permit.request_spec() == request
                            &&& permit.origin_spec()
                                == KvQuiescenceOrigin::CompletedExact {
                                    epoch: completion_epoch,
                                }
                            &&& ferric_spec::scheduling::request_transition(
                                before.slot_model(request.slot_spec() as int),
                                RequestTransition::CompleteExact,
                            ) == Ok(self.slot_model(request.slot_spec() as int))
                            &&& self.slots@[request.slot_spec() as int].active_epoch
                                == before.slots@[request.slot_spec() as int].active_epoch
                            &&& self.slots@[request.slot_spec() as int].generation
                                == before.slots@[request.slot_spec() as int].generation
                            &&& self.slots@[request.slot_spec() as int].last_quiescent_epoch
                                == before.slots@[request.slot_spec() as int].last_quiescent_epoch
                            &&& self.slots@[request.slot_spec() as int].in_free_ring
                                == before.slots@[request.slot_spec() as int].in_free_ring
                            &&& self.slots@[request.slot_spec() as int].in_reclaim_ring
                                == before.slots@[request.slot_spec() as int].in_reclaim_ring
                        }
                        None => false,
                    }
                })
                &&& (forall|slot_index: int| 0 <= slot_index < C
                    && !(exists|member_offset: int| 0 <= member_offset < *count
                        && (#[trigger] before.member_ring@[
                            ring_position::<C>(before.member_head, member_offset as nat)
                        ].slot_spec()) == slot_index) ==>
                            #[trigger] self.slots@[slot_index] == before.slots@[slot_index])
            }
        }
    }

    pub(crate) closed spec fn completed_batch_refines(
        &self,
        before: &Self,
        permits: Seq<Option<KvQuiescencePermit>>,
        count: usize,
    ) -> bool {
        &&& count == before.pending_batch_member_count_spec()
        &&& count <= permits.len()
        &&& (forall |offset: int| 0 <= offset < count ==> match
            #[trigger] permits[offset]
        {
            Some(permit) => {
                let request = permit.request_spec();
                &&& request.slot_spec() < C
                &&& before.pending_member_spec(offset as usize) == Some(request)
                &&& self.state_spec(request) == before.state_spec(request)
                &&& (self.state_spec(request) == Some(RequestState::InFlight)
                    || self.state_spec(request) == Some(RequestState::Retiring))
                &&& (self.state_spec(request) == Some(RequestState::Retiring) ==>
                    self.detachment_ready(request, permit.origin_spec()))
            }
            None => false,
        })
        &&& (forall |left: int, right: int| 0 <= left < right < count ==>
            permits[left].unwrap().request_spec().slot_spec()
                != permits[right].unwrap().request_spec().slot_spec())
    }

    pub(crate) proof fn apply_completed_batch_member(
        &self,
        before: &Self,
        permits: Seq<Option<KvQuiescencePermit>>,
        count: usize,
        offset: int,
    )
        requires
            self.completed_batch_refines(before, permits, count),
            0 <= offset < count,
        ensures
            permits[offset].is_some(),
            permits[offset].unwrap().request_spec().slot_spec() < C,
            before.pending_member_spec(offset as usize)
                == Some(permits[offset].unwrap().request_spec()),
            self.state_spec(permits[offset].unwrap().request_spec())
                == before.state_spec(permits[offset].unwrap().request_spec()),
            self.state_spec(permits[offset].unwrap().request_spec())
                == Some(RequestState::InFlight)
                || self.state_spec(permits[offset].unwrap().request_spec())
                    == Some(RequestState::Retiring),
            self.state_spec(permits[offset].unwrap().request_spec())
                == Some(RequestState::Retiring) ==> self.detachment_ready(
                    permits[offset].unwrap().request_spec(),
                    permits[offset].unwrap().origin_spec(),
                ),
    {
        reveal(Scheduler::completed_batch_refines);
    }

    pub(crate) proof fn apply_completed_batch_distinct(
        &self,
        before: &Self,
        permits: Seq<Option<KvQuiescencePermit>>,
        count: usize,
        left: int,
        right: int,
    )
        requires
            self.completed_batch_refines(before, permits, count),
            0 <= left < right < count,
        ensures
            permits[left].unwrap().request_spec().slot_spec()
                != permits[right].unwrap().request_spec().slot_spec(),
    {
        reveal(Scheduler::completed_batch_refines);
    }

    pub(crate) proof fn apply_completed_storage_length(
        &self,
        before: &Self,
        completion_epoch: u64,
        before_permits: Seq<Option<KvQuiescencePermit>>,
        permits: Seq<Option<KvQuiescencePermit>>,
        count: usize,
    )
        requires self.completion_refines(
            before,
            completion_epoch,
            before_permits,
            permits,
            &Ok(count),
        ),
        ensures permits.len() == before_permits.len(),
    {
        reveal(Scheduler::completion_refines);
    }

    pub(crate) closed spec fn finalized_slot_refines(
        &self,
        before: &Self,
        request: RequestId,
        epoch: u64,
    ) -> bool {
        let slot_index = request.slot_spec() as int;
        &&& slot_index < C
        &&& request.generation_spec() == before.slots@[slot_index].generation
        &&& before.slots@[slot_index].active_epoch == epoch
        &&& self.slots@[slot_index].last_quiescent_epoch == epoch
        &&& ferric_spec::scheduling::request_transition(
            before.slot_model(slot_index),
            RequestTransition::FinalizeKv,
        ) == Ok(self.slot_model(slot_index))
        &&& self.slots_frame_except(before, slot_index)
        &&& self.slots@[slot_index].generation == before.slots@[slot_index].generation
        &&& self.slots@[slot_index].active_epoch == NO_EPOCH
        &&& self.slots@[slot_index].in_free_ring == before.slots@[slot_index].in_free_ring
        &&& self.slots@[slot_index].in_reclaim_ring
            == before.slots@[slot_index].in_reclaim_ring
        &&& self.free_ring@ == before.free_ring@
        &&& self.free_head == before.free_head
        &&& self.free_len == before.free_len
        &&& self.reclaim_ring@ == before.reclaim_ring@
        &&& self.reclaim_head == before.reclaim_head
        &&& self.reclaim_len == before.reclaim_len
        &&& self.member_ring@ == before.member_ring@
        &&& self.member_head == before.member_head
        &&& self.member_len == before.member_len
        &&& self.batch_ring@ == before.batch_ring@
        &&& self.batch_head == before.batch_head
        &&& self.batch_len == before.batch_len
        &&& self.cursor == before.cursor
        &&& self.submitted == before.submitted
        &&& self.completed == before.completed
        &&& self.live_count == before.live_count
    }

    closed spec fn finalized_slot_updates(
        &self,
        before: &Self,
        request: RequestId,
        epoch: u64,
    ) -> bool {
        let slot_index = request.slot_spec() as int;
        &&& slot_index < C
        &&& before.slots@[slot_index].generation == request.generation_spec()
        &&& before.slots@[slot_index].state == RequestState::InFlight
        &&& before.slots@[slot_index].active_epoch == epoch
        &&& NO_EPOCH < epoch <= before.completed
        &&& self.slots@
            == before.slots@.update(slot_index, self.slots@[slot_index])
        &&& self.slots@[slot_index].generation == before.slots@[slot_index].generation
        &&& self.slots@[slot_index].state == RequestState::Ready
        &&& self.slots@[slot_index].active_epoch == NO_EPOCH
        &&& self.slots@[slot_index].last_quiescent_epoch == epoch
        &&& self.slots@[slot_index].in_free_ring
            == before.slots@[slot_index].in_free_ring
        &&& self.slots@[slot_index].in_reclaim_ring
            == before.slots@[slot_index].in_reclaim_ring
        &&& self.free_ring@ == before.free_ring@
        &&& self.free_head == before.free_head
        &&& self.free_len == before.free_len
        &&& self.reclaim_ring@ == before.reclaim_ring@
        &&& self.reclaim_head == before.reclaim_head
        &&& self.reclaim_len == before.reclaim_len
        &&& self.member_ring@ == before.member_ring@
        &&& self.member_head == before.member_head
        &&& self.member_len == before.member_len
        &&& self.batch_ring@ == before.batch_ring@
        &&& self.batch_head == before.batch_head
        &&& self.batch_len == before.batch_len
        &&& self.cursor == before.cursor
        &&& self.submitted == before.submitted
        &&& self.completed == before.completed
        &&& self.live_count == before.live_count
    }

    pub(crate) closed spec fn finalized_refines(
        &self,
        before: &Self,
        finalized: &KvFinalizedRequest,
        result: &Result<(), SchedulerError>,
    ) -> bool {
        match result {
            Err(error) => {
                &&& Some(*error) == finalized_expected_error::<C>(before, finalized)
                &&& self.same_scalars(before)
            }
            Ok(()) => {
                let request = finalized.request_spec();
                &&& finalized_expected_error::<C>(before, finalized).is_none()
                &&& match finalized.origin_spec() {
                    KvQuiescenceOrigin::CompletedExact { epoch } => {
                        self.finalized_slot_refines(before, request, epoch)
                    }
                    KvQuiescenceOrigin::NeverSubmitted => false,
                }
            }
        }
    }

    closed spec fn detached_slot_refines(
        &self,
        before: &Self,
        detached: &KvDetachedRequest,
        request: RequestId,
    ) -> bool {
        let detached_request = detached.request_spec();
        let slot_index = detached_request.slot_spec() as int;
        &&& request == detached_request
        &&& slot_index < C
        &&& detached_request.generation_spec() == before.slots@[slot_index].generation
        &&& (match detached.origin_spec() {
            KvQuiescenceOrigin::NeverSubmitted => {
                before.slots@[slot_index].active_epoch == NO_EPOCH
                    && before.slots@[slot_index].last_quiescent_epoch == NO_EPOCH
            }
            KvQuiescenceOrigin::CompletedExact { epoch } => {
                (before.slots@[slot_index].active_epoch != NO_EPOCH
                    && before.slots@[slot_index].active_epoch == epoch)
                    || (before.slots@[slot_index].active_epoch == NO_EPOCH
                        && before.slots@[slot_index].last_quiescent_epoch == epoch)
            }
        })
        &&& ferric_spec::scheduling::request_transition(
            before.slot_model(slot_index),
            RequestTransition::DetachKv,
        ) == Ok(self.slot_model(slot_index))
        &&& self.slots_frame_except(before, slot_index)
        &&& self.slots@[slot_index].generation == before.slots@[slot_index].generation + 1
        &&& self.slots@[slot_index].active_epoch == NO_EPOCH
        &&& self.slots@[slot_index].last_quiescent_epoch == NO_EPOCH
        &&& self.slots@[slot_index].in_free_ring
        &&& !self.slots@[slot_index].in_reclaim_ring
    }

    closed spec fn detached_free_ring_refines(
        &self,
        before: &Self,
        detached: &KvDetachedRequest,
    ) -> bool {
        let slot_index = detached.request_spec().slot_spec() as int;
        let free_tail = ring_position::<C>(before.free_head, before.free_len as nat);
        &&& self.free_head == before.free_head
        &&& self.free_len == before.free_len + 1
        &&& self.free_ring@[free_tail] == slot_index
        &&& (forall|ring_index: int| 0 <= ring_index < C && ring_index != free_tail ==>
            #[trigger] self.free_ring@[ring_index] == before.free_ring@[ring_index])
    }

    closed spec fn detached_other_rings_refine(&self, before: &Self) -> bool {
        &&& self.reclaim_ring@ == before.reclaim_ring@
        &&& self.reclaim_head == before.reclaim_head
        &&& self.reclaim_len == before.reclaim_len
        &&& self.member_ring@ == before.member_ring@
        &&& self.member_head == before.member_head
        &&& self.member_len == before.member_len
        &&& self.batch_ring@ == before.batch_ring@
        &&& self.batch_head == before.batch_head
        &&& self.batch_len == before.batch_len
        &&& self.cursor == before.cursor
        &&& self.submitted == before.submitted
        &&& self.completed == before.completed
    }

    closed spec fn detached_count_refines(&self, before: &Self) -> bool {
        &&& self.scalar_invariant()
        &&& self.live_count + 1 == before.live_count
        &&& live_slot_count(self.slots@, C as nat) + 1
            == live_slot_count(before.slots@, C as nat)
        &&& nonreclaim_live_count(self.slots@, C as nat) + 1
            == nonreclaim_live_count(before.slots@, C as nat)
    }

    pub(crate) closed spec fn detached_refines(
        &self,
        before: &Self,
        detached: &KvDetachedRequest,
        result: &Result<RequestId, SchedulerError>,
    ) -> bool {
        match result {
            Err(error) => {
                &&& Some(*error) == detached_expected_error::<C>(before, detached)
                &&& self.same_scalars(before)
            }
            Ok(request) => {
                &&& detached_expected_error::<C>(before, detached).is_none()
                &&& self.detached_slot_refines(before, detached, *request)
                &&& self.detached_free_ring_refines(before, detached)
                &&& self.detached_other_rings_refine(before)
                &&& self.detached_count_refines(before)
            }
        }
    }

    closed spec fn reclaim_slot_updates(
        &self,
        before: &Self,
        request: RequestId,
        next_generation: u32,
    ) -> bool {
        let slot_index = request.slot_spec() as int;
        let free_tail = ring_position::<C>(before.free_head, before.free_len as nat);
        let replacement = Slot {
            generation: next_generation,
            state: RequestState::Vacant,
            active_epoch: NO_EPOCH,
            last_quiescent_epoch: NO_EPOCH,
            in_free_ring: true,
            in_reclaim_ring: false,
        };
        &&& self.slots@ == before.slots@.update(slot_index, replacement)
        &&& self.free_ring@
            == before.free_ring@.update(free_tail, slot_index as usize)
        &&& self.free_head == before.free_head
        &&& self.free_len == before.free_len + 1
        &&& self.reclaim_ring@ == before.reclaim_ring@
        &&& self.reclaim_head == before.reclaim_head
        &&& self.reclaim_len == before.reclaim_len
        &&& self.member_ring@ == before.member_ring@
        &&& self.member_head == before.member_head
        &&& self.member_len == before.member_len
        &&& self.batch_ring@ == before.batch_ring@
        &&& self.batch_head == before.batch_head
        &&& self.batch_len == before.batch_len
        &&& self.cursor == before.cursor
        &&& self.submitted == before.submitted
        &&& self.completed == before.completed
        &&& self.live_count + 1 == before.live_count
    }

    closed spec fn retiring_reclaim_updates(&self, before: &Self, slot_index: usize) -> bool {
        let replacement = Slot {
            in_reclaim_ring: false,
            ..before.slots@[slot_index as int]
        };
        &&& self.slots@ == before.slots@.update(slot_index as int, replacement)
        &&& self.reclaim_ring@ == before.reclaim_ring@
        &&& self.reclaim_head == next_position::<C>(before.reclaim_head)
        &&& self.reclaim_len + 1 == before.reclaim_len
    }

    closed spec fn retiring_permit_updates(&self, before: &Self, slot_index: usize) -> bool {
        &&& self.retiring_reclaim_updates(before, slot_index)
        &&& self.free_ring@ == before.free_ring@
        &&& self.free_head == before.free_head
        &&& self.free_len == before.free_len
        &&& self.member_ring@ == before.member_ring@
        &&& self.member_head == before.member_head
        &&& self.member_len == before.member_len
        &&& self.batch_ring@ == before.batch_ring@
        &&& self.batch_head == before.batch_head
        &&& self.batch_len == before.batch_len
        &&& self.cursor == before.cursor
        &&& self.submitted == before.submitted
        &&& self.completed == before.completed
        &&& self.live_count == before.live_count
    }

    pub(crate) closed spec fn retiring_permit_refines(
        &self,
        before: &Self,
        result: &Result<Option<KvQuiescencePermit>, SchedulerError>,
    ) -> bool {
        match result {
            Err(_) => false,
            Ok(None) => self.same_scalars(before) && before.reclaim_len == 0,
            Ok(Some(permit)) => {
                let slot_index = before.reclaim_ring@[before.reclaim_head as int];
                let slot = before.slots@[slot_index as int];
                &&& permit.request_spec().slot_spec() == slot_index
                &&& permit.request_spec().generation_spec() == slot.generation
                &&& permit.origin_spec() == if slot.last_quiescent_epoch == NO_EPOCH {
                    KvQuiescenceOrigin::NeverSubmitted
                } else {
                    KvQuiescenceOrigin::CompletedExact {
                        epoch: slot.last_quiescent_epoch,
                    }
                }
                &&& self.slots_frame_except(before, slot_index as int)
                &&& self.slots@[slot_index as int].state == slot.state
                &&& self.slots@[slot_index as int].generation == slot.generation
                &&& self.slots@[slot_index as int].active_epoch == slot.active_epoch
                &&& self.slots@[slot_index as int].last_quiescent_epoch
                    == slot.last_quiescent_epoch
                &&& self.slots@[slot_index as int].in_free_ring == slot.in_free_ring
                &&& !self.slots@[slot_index as int].in_reclaim_ring
                &&& self.reclaim_ring@ == before.reclaim_ring@
                &&& self.reclaim_head == next_position::<C>(before.reclaim_head)
                &&& self.reclaim_len + 1 == before.reclaim_len
                &&& self.free_ring@ == before.free_ring@
                &&& self.free_head == before.free_head
                &&& self.free_len == before.free_len
                &&& self.member_ring@ == before.member_ring@
                &&& self.member_head == before.member_head
                &&& self.member_len == before.member_len
                &&& self.batch_ring@ == before.batch_ring@
                &&& self.batch_head == before.batch_head
                &&& self.batch_len == before.batch_len
                &&& self.cursor == before.cursor
                &&& self.submitted == before.submitted
                &&& self.completed == before.completed
                &&& self.live_count == before.live_count
            }
        }
    }

    #[must_use]
    pub const fn capacity(&self) -> (capacity: usize)
        ensures capacity == C,
    {
        C
    }

    #[must_use]
    pub const fn live_count(&self) -> (count: usize)
        ensures count == self.live_count_spec(),
    {
        self.live_count
    }

    pub closed spec fn live_count_spec(&self) -> usize {
        self.live_count
    }

    #[must_use]
    pub const fn completed_epoch(&self) -> (epoch: CompletionEpoch)
        ensures epoch.value == self.completed_epoch_spec().value,
    {
        CompletionEpoch { value: self.completed }
    }

    pub closed spec fn completed_epoch_spec(&self) -> CompletionEpoch {
        CompletionEpoch { value: self.completed }
    }

    pub(crate) closed spec fn pending_batch_member_count_spec(&self) -> usize {
        if self.batch_len == 0 {
            0
        } else {
            self.batch_ring@[self.batch_head as int].member_count
        }
    }

    pub(crate) closed spec fn pending_member_spec(&self, offset: usize) -> Option<RequestId> {
        if self.batch_len == 0
            || offset >= self.batch_ring@[self.batch_head as int].member_count
        {
            None
        } else {
            Some(self.member_ring@[
                ring_position::<C>(self.member_head, offset as nat)
            ])
        }
    }

    pub(crate) closed spec fn dispatch_enabled_spec(&self, output_capacity: usize) -> bool {
        &&& output_capacity > 0
        &&& self.submitted < u64::MAX
        &&& self.batch_len < C
        &&& self.member_len < C
        &&& ready_selection::<C>(
            self.slots@,
            self.cursor,
            C as nat,
            if output_capacity < C - self.member_len {
                output_capacity as nat
            } else {
                (C - self.member_len) as nat
            },
        ).len() > 0
    }

    pub(crate) closed spec fn completion_enabled_spec(
        &self,
        epoch: u64,
        permits: Seq<Option<KvQuiescencePermit>>,
    ) -> bool {
        completion_expected_error::<C>(self, epoch, permits).is_none()
    }

    pub(crate) fn pending_batch_member_count(&self) -> (count: usize)
        requires self.basic_invariant(),
        ensures
            count == self.pending_batch_member_count_spec(),
            count <= C,
    {
        proof {
            self.basic_implies_scalar();
        }
        reveal(Scheduler::scalar_invariant);
        reveal(Scheduler::pending_batch_member_count_spec);
        if self.batch_len == 0 {
            0
        } else {
            proof {
                self.pending_batch_head_facts();
            }
            self.batch_ring[self.batch_head].member_count
        }
    }

    pub(crate) fn pending_member(&self, offset: usize) -> (member: Option<RequestId>)
        requires self.basic_invariant(),
        ensures member == self.pending_member_spec(offset),
    {
        proof {
            self.basic_implies_scalar();
        }
        reveal(Scheduler::scalar_invariant);
        reveal(Scheduler::pending_member_spec);
        if self.batch_len == 0 || offset >= self.batch_ring[self.batch_head].member_count {
            None
        } else {
            proof {
                self.pending_member_facts(offset);
            }
            let position = ring_tail::<C>(self.member_head, offset);
            Some(self.member_ring[position])
        }
    }

    #[inline]
    fn admit_available(
        &mut self,
        slot_index: usize,
    ) -> (request: RequestId)
        requires
            C > 0,
            C <= MAX_REQUEST_SLOTS,
            old(self).free_head < C,
            old(self).free_len > 0,
            old(self).free_len <= C,
            old(self).live_count < C,
            slot_index < C,
            old(self).free_ring@[old(self).free_head as int] == slot_index,
            old(self).slots@[slot_index as int].state == RequestState::Vacant,
            old(self).slots@[slot_index as int].in_free_ring,
        ensures
            final(self).admitted_slot_refines(old(self), request),
            final(self).admitted_fields_refine(old(self)),
    {
        let slot = self.slots[slot_index];

        self.free_head = advance::<C>(self.free_head);
        self.free_len -= 1;
        self.slots[slot_index].state = RequestState::Ready;
        self.slots[slot_index].in_free_ring = false;
        self.live_count += 1;
        let request = RequestId::new(slot_index_to_u32(slot_index), slot.generation);
        assert(self.admitted_slot_refines(old(self), request)) by {
            reveal(Scheduler::admitted_slot_refines);
            reveal(Scheduler::slots_frame_except);
        }
        assert(self.admitted_fields_refine(old(self))) by {
            reveal(Scheduler::admitted_fields_refine);
        }
        request
    }

    #[inline]
    fn admit_established(&mut self) -> (request: RequestId)
        requires
            old(self).basic_invariant(),
            old(self).free_len > 0,
        ensures
            final(self).admitted_slot_refines(old(self), request),
            final(self).admitted_fields_refine(old(self)),
    {
        proof {
            self.admit_scalar_preflight();
            self.admit_head_preflight();
        }
        let slot_index = self.free_ring[self.free_head];
        assert(slot_index < C);
        assert(self.slots@[slot_index as int].state == RequestState::Vacant);
        assert(self.slots@[slot_index as int].in_free_ring);
        self.admit_available(slot_index)
    }

    #[inline]
    fn admit_refined(&mut self) -> (request: RequestId)
        requires
            old(self).basic_invariant(),
            old(self).free_len > 0,
        ensures
            final(self).basic_invariant(),
            final(self).admit_refines(old(self), &Ok(request)),
            final(self).slot_is_live_spec(request.slot_spec() as int),
            final(self).slot_generation_spec(request.slot_spec() as int)
                == old(self).slot_generation_spec(request.slot_spec() as int),
            forall|other: int|
                0 <= other < C && other != request.slot_spec() as int ==> {
                    &&& final(self).slot_is_live_spec(other)
                        == old(self).slot_is_live_spec(other)
                    &&& final(self).slot_generation_spec(other)
                        == old(self).slot_generation_spec(other)
                },
    {
        let request = self.admit_established();
        proof {
            self.admitted_step_establishes_postconditions(old(self), request);
        }
        request
    }

    #[inline]
    fn retire_inflight(
        &mut self,
        _request: RequestId,
        slot_index: usize,
    )
        requires
            old(self).basic_invariant(),
            slot_index < C,
            _request.slot_spec() as int == slot_index as int,
            old(self).slots@[slot_index as int].generation == _request.generation_spec(),
            old(self).slots@[slot_index as int].state == RequestState::InFlight,
        ensures
            final(self).retired_slot_refines(old(self), _request),
            final(self).retired_fields_refine(old(self), _request),
    {
        self.slots[slot_index].state = RequestState::Retiring;
        assert(self.retired_slot_refines(old(self), _request)) by {
            reveal(Scheduler::retired_slot_refines);
            reveal(Scheduler::slots_frame_except);
        }
        assert(self.retired_fields_refine(old(self), _request)) by {
            reveal(Scheduler::retired_fields_refine);
        }
    }

    #[inline]
    fn retire_ready(
        &mut self,
        _request: RequestId,
        slot_index: usize,
    )
        requires
            old(self).basic_invariant(),
            slot_index < C,
            _request.slot_spec() as int == slot_index as int,
            old(self).slots@[slot_index as int].generation == _request.generation_spec(),
            old(self).slots@[slot_index as int].state == RequestState::Ready,
        ensures
            final(self).retired_slot_refines(old(self), _request),
            final(self).retired_fields_refine(old(self), _request),
    {
        proof {
            self.basic_implies_slots();
            self.basic_implies_reclaim_ring();
        }
        assert(self.reclaim_len < C);
        assert(!self.slots@[slot_index as int].in_reclaim_ring);
        let tail = ring_tail::<C>(self.reclaim_head, self.reclaim_len);
        self.reclaim_ring[tail] = slot_index;
        self.reclaim_len += 1;
        self.slots[slot_index].state = RequestState::Retiring;
        self.slots[slot_index].active_epoch = NO_EPOCH;
        self.slots[slot_index].in_reclaim_ring = true;
        assert(self.retired_slot_refines(old(self), _request)) by {
            reveal(Scheduler::retired_slot_refines);
            reveal(Scheduler::slots_frame_except);
        }
        assert(self.retired_fields_refine(old(self), _request)) by {
            reveal(Scheduler::retired_fields_refine);
            assert(self.reclaim_ring@
                == old(self).reclaim_ring@.update(tail as int, slot_index));
        }
    }

    /// Admits one request from the O(1) free ring.
    ///
    /// # Errors
    ///
    /// Returns [`SchedulerError::OutOfSlots`] when every slot is live.
    pub fn admit(&mut self) -> (result: Result<RequestId, SchedulerError>)
        requires old(self).basic_invariant(),
        ensures
            final(self).basic_invariant(),
            final(self).admit_refines(old(self), &result),
            match result {
                Err(_) => final(self).identity_frame(old(self)),
                Ok(request) => {
                    let slot_index = request.slot_spec() as int;
                    &&& final(self).slot_is_live_spec(slot_index)
                    &&& final(self).slot_generation_spec(slot_index)
                        == old(self).slot_generation_spec(slot_index)
                    &&& (forall|other: int| 0 <= other < C && other != slot_index ==> {
                        &&& final(self).slot_is_live_spec(other)
                            == old(self).slot_is_live_spec(other)
                        &&& final(self).slot_generation_spec(other)
                            == old(self).slot_generation_spec(other)
                    })
                }
            },
    {
        if self.free_len == 0 {
            proof {
                self.admit_error_establishes_postconditions();
            }
            return Err(SchedulerError::OutOfSlots);
        }
        Ok(self.admit_refined())
    }

    /// Retires a request. An in-flight request stays attached to its batch;
    /// a ready request enters the O(1) reclaim ring immediately.
    ///
    /// # Errors
    ///
    /// Returns an exact identity or state error when `request` is invalid,
    /// stale, vacant, or already retiring.
    pub fn retire(&mut self, request: RequestId) -> (result: Result<(), SchedulerError>)
        requires old(self).basic_invariant(),
        ensures
            final(self).basic_invariant(),
            final(self).retire_refines(old(self), request, &result),
            final(self).identity_frame(old(self)),
    {
        proof {
            self.basic_implies_slots();
            self.basic_implies_reclaim_ring();
        }
        let slot_index = request.slot() as usize;
        if slot_index >= C {
            proof {
                reveal(retire_expected_error);
                self.retire_error_establishes_postconditions(
                    request,
                    SchedulerError::InvalidSlot,
                );
            }
            return Err(SchedulerError::InvalidSlot);
        }
        let slot = self.slots[slot_index];
        if slot.generation != request.generation() {
            proof {
                reveal(retire_expected_error);
                self.retire_error_establishes_postconditions(
                    request,
                    SchedulerError::StaleRequest,
                );
            }
            return Err(SchedulerError::StaleRequest);
        }
        match slot.state {
            RequestState::Vacant => {
                proof {
                    reveal(retire_expected_error);
                    self.retire_error_establishes_postconditions(
                        request,
                        SchedulerError::RequestNotLive,
                    );
                }
                Err(SchedulerError::RequestNotLive)
            }
            RequestState::Retiring => {
                proof {
                    reveal(retire_expected_error);
                    self.retire_error_establishes_postconditions(
                        request,
                        SchedulerError::AlreadyRetiring,
                    );
                }
                Err(SchedulerError::AlreadyRetiring)
            }
            RequestState::InFlight => {
                self.retire_inflight(request, slot_index);
                proof {
                    self.retired_step_establishes_postconditions(old(self), request);
                }
                Ok(())
            }
            RequestState::Ready => {
                self.retire_ready(request, slot_index);
                proof {
                    self.retired_step_establishes_postconditions(old(self), request);
                }
                Ok(())
            }
        }
    }

    #[inline(always)]
    fn dispatch_one_selected(
        &mut self,
        output: &mut [RequestId],
        selected: usize,
        slot_index: usize,
        member_tail: usize,
        next_epoch: u64,
    ) -> (handle: RequestId)
        requires
            selected < old(output)@.len(),
            C <= MAX_REQUEST_SLOTS,
            slot_index < C,
            member_tail < C,
            old(self).slots@[slot_index as int].state == RequestState::Ready,
        ensures
            handle.slot_spec() == slot_index,
            handle.generation_spec()
                == old(self).slots@[slot_index as int].generation,
            final(output)@
                == old(output)@.update(selected as int, handle),
            final(self).slots@
                == old(self).slots@.update(
                    slot_index as int,
                    Slot {
                        state: RequestState::InFlight,
                        active_epoch: next_epoch,
                        ..old(self).slots@[slot_index as int]
                    },
                ),
            final(self).member_ring@
                == old(self).member_ring@.update(member_tail as int, handle),
            final(self).dispatch_scan_frames(old(self)),
    {
        reveal(Scheduler::dispatch_scan_frames);
        let slot = self.slots[slot_index];
        let handle = RequestId::new(slot_index_to_u32(slot_index), slot.generation);
        output[selected] = handle;
        self.member_ring[member_tail] = handle;
        self.slots[slot_index].state = RequestState::InFlight;
        self.slots[slot_index].active_epoch = next_epoch;
        handle
    }

    fn dispatch_preflight(
        &self,
        output: &[RequestId],
    ) -> (result: Result<Option<(u64, usize)>, SchedulerError>)
        requires self.basic_invariant(),
        ensures
            match result {
                Err(error) => {
                    &&& error == if output@.len() == 0 {
                        SchedulerError::EmptyBatchStorage
                    } else {
                        SchedulerError::SubmissionEpochExhausted
                    }
                    &&& (output@.len() == 0
                        || (output@.len() > 0 && self.submitted == u64::MAX))
                }
                Ok(None) => {
                    &&& output@.len() > 0
                    &&& self.submitted < u64::MAX
                    &&& (self.batch_len == C || self.member_len == C)
                }
                Ok(Some((next_epoch, limit))) => {
                    &&& output@.len() > 0
                    &&& self.submitted < u64::MAX
                    &&& self.batch_len < C
                    &&& self.member_len < C
                    &&& next_epoch as int == self.submitted as int + 1
                    &&& limit as nat == if output@.len() < C - self.member_len {
                        output@.len()
                    } else {
                        (C - self.member_len) as nat
                    }
                }
            },
    {
        if output.is_empty() {
            return Err(SchedulerError::EmptyBatchStorage);
        }
        let next_epoch = match self.submitted.checked_add(1) {
            Some(epoch) => epoch,
            None => return Err(SchedulerError::SubmissionEpochExhausted),
        };
        if self.batch_len == C || self.member_len == C {
            return Ok(None);
        }

        let available_members = C - self.member_len;
        let limit = if output.len() < available_members {
            output.len()
        } else {
            available_members
        };
        Ok(Some((next_epoch, limit)))
    }

    #[inline(always)]
    fn dispatch_scan_commit(
        &mut self,
        output: &mut [RequestId],
        limit: usize,
        next_epoch: u64,
    ) -> (result: (usize, usize, usize))
        requires
            old(self).basic_invariant(),
            old(self).member_len < C,
            limit <= C - old(self).member_len,
            limit <= old(output)@.len(),
        ensures {
            let selected = result.0;
            let slot_index = result.1;
            let member_tail = result.2;
            let chosen = final(output)@.subrange(0, selected as int);
            let expected = ready_selection::<C>(
                old(self).slots@,
                old(self).cursor,
                C as nat,
                limit as nat,
            );
            &&& selected == chosen.len()
            &&& selected == expected.len()
            &&& selected <= limit
            &&& slot_index == ready_scan_cursor::<C>(
                old(self).slots@,
                old(self).cursor,
                C as nat,
                limit as nat,
            )
            &&& slot_index < C
            &&& member_tail < C
            &&& member_tail as int == ring_position_or_head::<C>(
                old(self).member_head,
                (old(self).member_len + selected) as nat,
            )
            &&& selected_request_slots(chosen) == expected
            &&& old(self).member_len + chosen.len() <= C
            &&& old(self).dispatch_chosen_ready(chosen)
            &&& final(self).dispatch_scan_projection(
                old(self),
                old(output)@,
                final(output)@,
                chosen,
                next_epoch,
            )
            &&& final(self).dispatch_scan_frames(old(self))
            &&& final(output)@.len() == old(output)@.len()
        },
    {
        proof {
            self.basic_implies_scalar();
        }
        reveal(Scheduler::scalar_invariant);
        let member_start = ring_tail::<C>(self.member_head, self.member_len);
        let mut member_tail = member_start;
        let mut slot_index = self.cursor;
        let mut scanned = 0;
        let mut selected = 0;
        let ghost mut chosen = Seq::<RequestId>::empty();

        proof {
            self.dispatch_scan_init(output@, limit, next_epoch, member_tail);
            assert(self.dispatch_scan_refines(
                old(self),
                old(output)@,
                output@,
                chosen,
                scanned,
                limit,
                next_epoch,
                slot_index,
                member_tail,
            ));
        }

        while scanned < C && selected < limit
            invariant
                C > 0,
                C <= MAX_REQUEST_SLOTS,
                old(self).cursor < C,
                old(self).member_head < C,
                limit <= C - old(self).member_len,
                self.dispatch_scan_refines(
                    old(self),
                    old(output)@,
                    output@,
                    chosen,
                    scanned,
                    limit,
                    next_epoch,
                    slot_index,
                    member_tail,
                ),
                selected == chosen.len(),
                scanned <= C,
                selected <= limit,
                limit <= output.len(),
                output.len() == old(output)@.len(),
            decreases C - scanned,
        {
            proof {
                reveal(Scheduler::dispatch_scan_refines);
                reveal(Scheduler::dispatch_scan_oracle);
                self.dispatch_scan_current_not_chosen(
                    old(self),
                    old(output)@,
                    output@,
                    chosen,
                    scanned,
                    limit,
                    next_epoch,
                    slot_index,
                    member_tail,
                );
            }
            let slot = self.slots[slot_index];
            assert(slot == self.slots@[slot_index as int]);
            match slot.state {
            RequestState::Ready => {
                let ghost previous_chosen = chosen;
                let ghost prior_output = output@;
                let ghost prior_slots = self.slots@;
                let ghost prior_members = self.member_ring@;
                let ghost prior_member_tail = member_tail;
                proof {
                    reveal(Scheduler::dispatch_scan_refines);
                    reveal(Scheduler::dispatch_scan_projection);
                }
                assert(C <= MAX_REQUEST_SLOTS);
                assert(self.slots@[slot_index as int].state == RequestState::Ready);
                assert(selected < output.len());
                let _handle = self.dispatch_one_selected(
                    output,
                    selected,
                    slot_index,
                    member_tail,
                    next_epoch,
                );
                member_tail = advance::<C>(member_tail);
                selected += 1;
                proof {
                    dispatch_selected_output_push(old(output)@, previous_chosen, _handle);
                    self.dispatch_scan_projection_ready_step(
                        old(self),
                        old(output)@,
                        prior_output,
                        prior_slots,
                        prior_members,
                        previous_chosen,
                        next_epoch,
                        slot_index,
                        prior_member_tail,
                        _handle,
                    );
                    assert(output@
                        == dispatch_selected_output(
                            old(output)@,
                            previous_chosen.push(_handle),
                        ));
                    self.dispatch_scan_oracle_ready_step(
                        old(self),
                        previous_chosen,
                        scanned,
                        limit,
                        slot_index,
                        prior_member_tail,
                        _handle,
                    );
                    chosen = previous_chosen.push(_handle);
                    assert(self.dispatch_scan_projection(
                        old(self),
                        old(output)@,
                        output@,
                        chosen,
                        next_epoch,
                    )) by {
                        reveal(Scheduler::dispatch_scan_projection);
                    }
                    assert(self.dispatch_scan_frames(old(self))) by {
                        reveal(Scheduler::dispatch_scan_frames);
                    }
                    assert(self.dispatch_scan_refines(
                        old(self),
                        old(output)@,
                        output@,
                        chosen,
                        (scanned + 1) as usize,
                        limit,
                        next_epoch,
                        next_position::<C>(slot_index),
                        member_tail,
                    )) by {
                        reveal(Scheduler::dispatch_scan_refines);
                    }
                }
            }
            _ => {
                proof {
                    assert(self.slots@[slot_index as int].state
                        != RequestState::Ready);
                    assert(old(self).slots@[slot_index as int].state
                        != RequestState::Ready);
                    self.dispatch_scan_oracle_skip_step(
                        old(self),
                        chosen,
                        scanned,
                        limit,
                        slot_index,
                        member_tail,
                    );
                    assert(self.dispatch_scan_refines(
                        old(self),
                        old(output)@,
                        output@,
                        chosen,
                        (scanned + 1) as usize,
                        limit,
                        next_epoch,
                        next_position::<C>(slot_index),
                        member_tail,
                    )) by {
                        reveal(Scheduler::dispatch_scan_refines);
                    }
                }
            }
            }
            slot_index = advance::<C>(slot_index);
            scanned += 1;
        }

        proof {
            self.dispatch_scan_finished(
                old(self),
                old(output)@,
                output@,
                chosen,
                scanned,
                limit,
                next_epoch,
                slot_index,
                member_tail,
            );
            self.dispatch_scan_chosen_ready(
                old(self),
                old(output)@,
                output@,
                chosen,
                scanned,
                limit,
                next_epoch,
                slot_index,
                member_tail,
            );
            dispatch_selected_output_facts(old(output)@, chosen);
            assert(output@.subrange(0, selected as int) == chosen) by {
                assert forall|offset: int| 0 <= offset < selected implies
                    output@.subrange(0, selected as int)[offset] == chosen[offset] by {
                    assert(output@[offset] == chosen[offset]);
                }
            }
            assert(self.dispatch_scan_projection(
                old(self),
                old(output)@,
                output@,
                output@.subrange(0, selected as int),
                next_epoch,
            ));
        }

        (selected, slot_index, member_tail)
    }

    fn dispatch_enabled(
        &mut self,
        output: &mut [RequestId],
        next_epoch: u64,
        limit: usize,
    ) -> (result: Option<DispatchBatch>)
        requires
            old(self).basic_invariant(),
            old(output)@.len() > 0,
            old(self).submitted < u64::MAX,
            old(self).batch_len < C,
            old(self).member_len < C,
            next_epoch as int == old(self).submitted as int + 1,
            limit as nat == if old(output)@.len() < C - old(self).member_len {
                old(output)@.len()
            } else {
                (C - old(self).member_len) as nat
            },
        ensures
            match result {
                None => {
                    let expected = ready_selection::<C>(
                        old(self).slots@,
                        old(self).cursor,
                        C as nat,
                        limit as nat,
                    );
                    &&& expected.len() == 0
                    &&& final(self).same_scalars(old(self))
                    &&& final(output)@ == old(output)@
                }
                Some(batch) => {
                    let chosen = final(output)@.subrange(
                        0,
                        batch.member_count_spec() as int,
                    );
                    let expected = ready_selection::<C>(
                        old(self).slots@,
                        old(self).cursor,
                        C as nat,
                        limit as nat,
                    );
                    &&& batch.member_count_spec() > 0
                    &&& batch.member_count_spec() <= old(output)@.len()
                    &&& batch.member_count_spec() == chosen.len()
                    &&& batch.member_count_spec() == expected.len()
                    &&& batch.epoch_spec().value == next_epoch
                    &&& selected_request_slots(chosen) == expected
                    &&& old(self).dispatch_chosen_ready(chosen)
                    &&& old(self).member_len + chosen.len() <= C
                    &&& final(self).dispatch_commit_refines(
                        old(self),
                        chosen,
                        final(self).cursor,
                        next_epoch,
                    )
                    &&& final(self).cursor == ready_scan_cursor::<C>(
                        old(self).slots@,
                        old(self).cursor,
                        C as nat,
                        limit as nat,
                    )
                    &&& final(self).cursor < C
                    &&& final(output)@
                        == dispatch_selected_output(old(output)@, chosen)
                }
            },
    {
        let (selected, slot_index, _member_tail) =
            self.dispatch_scan_commit(output, limit, next_epoch);
        let ghost chosen = output@.subrange(0, selected as int);

        if selected == 0 {
            proof {
                reveal(Scheduler::same_scalars);
                reveal(Scheduler::dispatch_scan_projection);
                reveal(Scheduler::dispatch_scan_frames);
                reveal(dispatch_selected_slots);
                reveal(dispatch_selected_output);
                reveal(dispatch_selected_members);
                assert(self.same_scalars(old(self)));
            }
            return None;
        }

        let batch_tail = ring_tail::<C>(self.batch_head, self.batch_len);
        self.batch_ring[batch_tail] = BatchRecord {
            epoch: CompletionEpoch { value: next_epoch },
            member_count: selected,
        };
        self.batch_len += 1;
        self.member_len += selected;
        self.cursor = slot_index;
        self.submitted = next_epoch;

        assert(self.dispatch_commit_refines(
            old(self),
            chosen,
            slot_index,
            next_epoch,
        )) by {
            reveal(Scheduler::dispatch_commit_refines);
            reveal(Scheduler::dispatch_scan_projection);
            reveal(Scheduler::dispatch_scan_frames);
            assert(self.batch_ring@ == old(self).batch_ring@.update(
                batch_tail as int,
                BatchRecord {
                    epoch: CompletionEpoch { value: next_epoch },
                    member_count: selected,
                },
            ));
        }
        Some(DispatchBatch {
            epoch: CompletionEpoch { value: next_epoch },
            member_count: selected,
        })
    }

    proof fn dispatch_enabled_compose_refines_some(
        &self,
        before: &Self,
        before_output: Seq<RequestId>,
        output: Seq<RequestId>,
        next_epoch: u64,
        limit: usize,
        batch: DispatchBatch,
        chosen: Seq<RequestId>,
    )
        requires
            C > 0,
            before.cursor < C,
            before.member_head < C,
            before.batch_head < C,
            before.completed <= before.submitted,
            before_output.len() > 0,
            before.submitted < u64::MAX,
            before.batch_len < C,
            before.member_len < C,
            next_epoch as int == before.submitted as int + 1,
            limit as nat == if before_output.len() < C - before.member_len {
                before_output.len()
            } else {
                (C - before.member_len) as nat
            },
            batch.member_count_spec() <= output.len(),
            chosen == output.subrange(0, batch.member_count_spec() as int),
            chosen.len() <= before_output.len(),
            batch.member_count_spec() > 0,
            batch.member_count_spec() == chosen.len(),
            batch.member_count_spec() == ready_selection::<C>(
                before.slots@,
                before.cursor,
                C as nat,
                limit as nat,
            ).len(),
            batch.epoch_spec().value == next_epoch,
            selected_request_slots(chosen) == ready_selection::<C>(
                before.slots@,
                before.cursor,
                C as nat,
                limit as nat,
            ),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(
                before,
                chosen,
                self.cursor,
                next_epoch,
            ),
            self.cursor == ready_scan_cursor::<C>(
                before.slots@,
                before.cursor,
                C as nat,
                limit as nat,
            ),
            output == dispatch_selected_output(before_output, chosen),
        ensures self.dispatch_refines(before, before_output, output, &Ok(Some(batch))),
    {
        let selected = ready_selection::<C>(
            before.slots@,
            before.cursor,
            C as nat,
            limit as nat,
        );
        let batch_tail = ring_position::<C>(
            before.batch_head,
            before.batch_len as nat,
        );

        ready_scan_facts::<C>(
            before.slots@,
            before.cursor,
            C as nat,
            limit as nat,
        );
        dispatch_selected_output_facts(before_output, chosen);

        assert(chosen.len() == selected.len()) by {
            assert(selected_request_slots(chosen) == selected);
            reveal(selected_request_slots);
        }
        assert(limit <= C - before.member_len);
        assert(chosen.len() <= limit);
        assert(before.member_len + chosen.len() <= C);
        assert forall|chosen_offset: int| 0 <= chosen_offset < chosen.len() implies
            chosen[chosen_offset].slot_spec() < C by {
            reveal(Scheduler::dispatch_chosen_ready);
        }

        assert(output.len() == before_output.len());
        assert forall|output_index: int|
            selected.len() <= output_index < output.len() implies
                #[trigger] output[output_index] == before_output[output_index] by {
            assert(output == dispatch_selected_output(before_output, chosen));
        }

        assert forall|selected_offset: int| 0 <= selected_offset < selected.len() implies {
            let slot_index = #[trigger] selected[selected_offset];
            &&& output[selected_offset].slot_spec() == slot_index
            &&& output[selected_offset].generation_spec()
                == before.slots@[slot_index].generation
            &&& ferric_spec::scheduling::request_transition(
                before.slot_model(slot_index),
                RequestTransition::Dispatch,
            ) == Ok(self.slot_model(slot_index))
            &&& self.slots@[slot_index].active_epoch == batch.epoch.value
            &&& self.slots@[slot_index].generation
                == before.slots@[slot_index].generation
            &&& self.slots@[slot_index].last_quiescent_epoch
                == before.slots@[slot_index].last_quiescent_epoch
            &&& self.slots@[slot_index].in_free_ring
                == before.slots@[slot_index].in_free_ring
            &&& self.slots@[slot_index].in_reclaim_ring
                == before.slots@[slot_index].in_reclaim_ring
            &&& self.member_ring@[ring_position::<C>(
                before.member_head,
                (before.member_len + selected_offset) as nat,
            )] == output[selected_offset]
        } by {
            assert(selected_request_slots(chosen).len() == chosen.len()) by {
                reveal(selected_request_slots);
            }
            assert(selected_offset < chosen.len());
            assert(selected[selected_offset]
                == chosen[selected_offset].slot_spec() as int) by {
                assert(selected_request_slots(chosen) == selected);
                reveal(selected_request_slots);
            }
            let request = chosen[selected_offset];
            let slot_index = request.slot_spec() as int;
            assert(output[selected_offset] == request);
            assert(request.generation_spec() == before.slots@[slot_index].generation) by {
                reveal(Scheduler::dispatch_chosen_ready);
            }
            assert(before.slots@[slot_index].state == RequestState::Ready) by {
                reveal(Scheduler::dispatch_chosen_ready);
            }
            assert(selected_request_slots(chosen).no_duplicates()) by {
                reveal(Scheduler::dispatch_chosen_ready);
            }
            dispatch_selected_slots_selected_fact(
                before.slots@,
                chosen,
                next_epoch,
                selected_offset,
            );
            dispatch_selected_members_selected_fact::<C>(
                before.member_ring@,
                before.member_head,
                before.member_len,
                chosen,
                selected_offset,
            );
            assert(self.slots@ == dispatch_selected_slots(
                before.slots@,
                chosen,
                next_epoch,
            )) by {
                reveal(Scheduler::dispatch_commit_refines);
            }
            assert(self.member_ring@ == dispatch_selected_members::<C>(
                before.member_ring@,
                before.member_head,
                before.member_len,
                chosen,
            )) by {
                reveal(Scheduler::dispatch_commit_refines);
            }
            assert(self.completed == before.completed) by {
                reveal(Scheduler::dispatch_commit_refines);
            }
            assert(next_epoch > self.completed);
            assert(batch.epoch.value == next_epoch);
            assert(ferric_spec::scheduling::request_transition(
                before.slot_model(slot_index),
                RequestTransition::Dispatch,
            ) == Ok(self.slot_model(slot_index))) by {
                reveal(Scheduler::slot_model);
                reveal(ferric_spec::scheduling::request_transition);
            }
        }

        assert forall|ring_index: int| 0 <= ring_index < C
            && !(exists|selected_offset: int| 0 <= selected_offset < selected.len()
                && (#[trigger] ring_position::<C>(
                    before.member_head,
                    (before.member_len + selected_offset) as nat,
                )) == ring_index) implies
                    #[trigger] self.member_ring@[ring_index]
                        == before.member_ring@[ring_index] by {
            assert forall|chosen_offset: int| 0 <= chosen_offset < chosen.len() implies
                chosen[chosen_offset].slot_spec() < C by {
                reveal(Scheduler::dispatch_chosen_ready);
            }
            assert(!(exists|chosen_offset: int| 0 <= chosen_offset < chosen.len()
                && #[trigger] ring_position::<C>(
                    before.member_head,
                    (before.member_len + chosen_offset) as nat,
                ) == ring_index));
            dispatch_selected_members_frame_fact::<C>(
                before.member_ring@,
                before.member_head,
                before.member_len,
                chosen,
                ring_index,
            );
            assert(self.member_ring@ == dispatch_selected_members::<C>(
                before.member_ring@,
                before.member_head,
                before.member_len,
                chosen,
            )) by {
                reveal(Scheduler::dispatch_commit_refines);
            }
        }

        ring_position_bounds::<C>(before.batch_head, before.batch_len as nat);
        assert(self.batch_ring@ == before.batch_ring@.update(
            batch_tail,
            BatchRecord {
                epoch: CompletionEpoch { value: next_epoch },
                member_count: chosen.len() as usize,
            },
        )) by {
            reveal(Scheduler::dispatch_commit_refines);
        }
        assert(self.batch_ring@[batch_tail].epoch.value == batch.epoch.value);
        assert(self.batch_ring@[batch_tail].member_count == selected.len());
        assert forall|ring_index: int| 0 <= ring_index < C
            && ring_index != batch_tail implies
                #[trigger] self.batch_ring@[ring_index]
                    == before.batch_ring@[ring_index] by {
        }

        assert forall|slot_index: int| 0 <= slot_index < C
            && !selected.contains(slot_index) implies
                #[trigger] self.slots@[slot_index] == before.slots@[slot_index] by {
            assert(!selected_request_slots(chosen).contains(slot_index));
            assert forall|chosen_offset: int| 0 <= chosen_offset < chosen.len() implies
                chosen[chosen_offset].slot_spec() < before.slots@.len() by {
                reveal(Scheduler::dispatch_chosen_ready);
            }
            dispatch_selected_slots_frame_fact(
                before.slots@,
                chosen,
                next_epoch,
                slot_index,
            );
            assert(self.slots@ == dispatch_selected_slots(
                before.slots@,
                chosen,
                next_epoch,
            )) by {
                reveal(Scheduler::dispatch_commit_refines);
            }
        }

        reveal(Scheduler::dispatch_refines);
        reveal(Scheduler::dispatch_commit_refines);
    }

    proof fn dispatch_enabled_compose_identity_none(
        &self,
        before: &Self,
    )
        requires self.same_scalars(before),
        ensures self.identity_frame(before),
    {
        self.same_scalars_preserves_identity(before);
    }

    proof fn dispatch_enabled_compose_identity_some(
        &self,
        before: &Self,
        batch: DispatchBatch,
        chosen: Seq<RequestId>,
        next_epoch: u64,
    )
        requires
            batch.member_count_spec() == chosen.len(),
            before.dispatch_chosen_ready(chosen),
            self.dispatch_commit_refines(
                before,
                chosen,
                self.cursor,
                next_epoch,
            ),
        ensures self.identity_frame(before),
    {
        reveal(Scheduler::dispatch_chosen_ready);
        reveal(Scheduler::dispatch_commit_refines);
        assert forall|chosen_offset: int| 0 <= chosen_offset < chosen.len() implies
            chosen[chosen_offset].slot_spec() < before.slots@.len() by {
            assert(chosen[chosen_offset].slot_spec() < C);
        }
        assert forall|slot_index: int| 0 <= slot_index < C implies {
            &&& self.slot_generation_spec(slot_index)
                == before.slot_generation_spec(slot_index)
            &&& self.slot_is_live_spec(slot_index)
                == before.slot_is_live_spec(slot_index)
        } by {
            if selected_request_slots(chosen).contains(slot_index) {
                let chosen_offset = choose|chosen_offset: int|
                    0 <= chosen_offset < selected_request_slots(chosen).len()
                        && selected_request_slots(chosen)[chosen_offset] == slot_index;
                reveal(selected_request_slots);
                dispatch_selected_slots_selected_fact(
                    before.slots@,
                    chosen,
                    next_epoch,
                    chosen_offset,
                );
                assert(before.slots@[slot_index].state == RequestState::Ready);
            } else {
                dispatch_selected_slots_frame_fact(
                    before.slots@,
                    chosen,
                    next_epoch,
                    slot_index,
                );
            }
            reveal(Scheduler::slot_generation_spec);
            reveal(Scheduler::slot_is_live_spec);
        }
    }

    proof fn dispatch_enabled_compose_none(
        &self,
        before: &Self,
        before_output: Seq<RequestId>,
        output: Seq<RequestId>,
        next_epoch: u64,
        limit: usize,
    )
        requires
            before.basic_invariant(),
            before_output.len() > 0,
            before.submitted < u64::MAX,
            before.batch_len < C,
            before.member_len < C,
            next_epoch as int == before.submitted as int + 1,
            limit as nat == if before_output.len() < C - before.member_len {
                before_output.len()
            } else {
                (C - before.member_len) as nat
            },
            ready_selection::<C>(
                before.slots@,
                before.cursor,
                C as nat,
                limit as nat,
            ).len() == 0,
            self.same_scalars(before),
            output == before_output,
        ensures
            self.dispatch_execution_refines(before, before_output, output, &Ok(None)),
    {
        reveal(Scheduler::dispatch_execution_refines);
    }

    proof fn dispatch_enabled_compose_some(
        &self,
        before: &Self,
        before_output: Seq<RequestId>,
        output: Seq<RequestId>,
        next_epoch: u64,
        limit: usize,
        batch: DispatchBatch,
    )
        requires
            before.basic_invariant(),
            before_output.len() > 0,
            before.submitted < u64::MAX,
            before.batch_len < C,
            before.member_len < C,
            next_epoch as int == before.submitted as int + 1,
            limit as nat == if before_output.len() < C - before.member_len {
                before_output.len()
            } else {
                (C - before.member_len) as nat
            },
            {
                let chosen = output.subrange(
                    0,
                    batch.member_count_spec() as int,
                );
                let expected = ready_selection::<C>(
                    before.slots@,
                    before.cursor,
                    C as nat,
                    limit as nat,
                );
                &&& batch.member_count_spec() > 0
                &&& batch.member_count_spec() == chosen.len()
                &&& batch.member_count_spec() == expected.len()
                &&& batch.epoch_spec().value == next_epoch
                &&& selected_request_slots(chosen) == expected
                &&& before.dispatch_chosen_ready(chosen)
                &&& before.member_len + chosen.len() <= C
                &&& self.dispatch_commit_refines(
                    before,
                    chosen,
                    self.cursor,
                    next_epoch,
                )
                &&& self.cursor == ready_scan_cursor::<C>(
                    before.slots@,
                    before.cursor,
                    C as nat,
                    limit as nat,
                )
                &&& self.cursor < C
                &&& output == dispatch_selected_output(before_output, chosen)
            },
        ensures
            self.dispatch_execution_refines(
                before,
                before_output,
                output,
                &Ok(Some(batch)),
            ),
    {
        reveal(Scheduler::dispatch_execution_refines);
    }

    /// Performs one deterministic rotating scan and submits one compact batch.
    /// Selected handles are written to the prefix of `output`.
    ///
    /// # Errors
    ///
    /// Returns [`SchedulerError::EmptyBatchStorage`] for empty output storage
    /// and [`SchedulerError::SubmissionEpochExhausted`] after the final epoch.
    pub fn dispatch_ready(
        &mut self,
        output: &mut [RequestId],
    ) -> (result: Result<Option<DispatchBatch>, SchedulerError>)
        requires old(self).basic_invariant(),
        ensures
            final(self).dispatch_execution_refines(
                old(self),
                old(output)@,
                final(output)@,
                &result,
            ),
            match result {
                Ok(Some(batch)) => batch.member_count_spec() <= old(output)@.len(),
                _ => true,
            },
    {
        match self.dispatch_preflight(output) {
            Err(error) => {
                proof {
                    self.same_scalars_reflexive();
                    reveal(Scheduler::dispatch_execution_refines);
                }
                Err(error)
            }
            Ok(None) => {
                proof {
                    self.same_scalars_reflexive();
                    reveal(Scheduler::dispatch_execution_refines);
                }
                Ok(None)
            }
            Ok(Some((next_epoch, limit))) => {
                let result = self.dispatch_enabled(output, next_epoch, limit);
                proof {
                    match &result {
                        None => {
                            self.dispatch_enabled_compose_none(
                                old(self),
                                old(output)@,
                                output@,
                                next_epoch,
                                limit,
                            );
                        }
                        Some(batch) => {
                            let ghost batch_snapshot = DispatchBatch {
                                epoch: batch.epoch,
                                member_count: batch.member_count,
                            };
                            self.dispatch_enabled_compose_some(
                                old(self),
                                old(output)@,
                                output@,
                                next_epoch,
                                limit,
                                batch_snapshot,
                            );
                        }
                    }
                }
                Ok(result)
            }
        }
    }

    pub(crate) proof fn apply_dispatch_refines(
        &self,
        before: &Self,
        before_output: Seq<RequestId>,
        output: Seq<RequestId>,
        result: &Result<Option<DispatchBatch>, SchedulerError>,
    )
        requires
            before.basic_invariant(),
            self.dispatch_execution_refines(before, before_output, output, result),
        ensures self.dispatch_refines(before, before_output, output, result),
    {
        match result {
            Err(_) | Ok(None) => {
                reveal(Scheduler::dispatch_execution_refines);
                reveal(Scheduler::dispatch_refines);
            }
            Ok(Some(batch)) => {
                before.basic_implies_scalar();
                reveal(Scheduler::scalar_invariant);
                reveal(Scheduler::dispatch_execution_refines);
                let available = (C - before.member_len) as usize;
                let limit = if before_output.len() < available {
                    before_output.len() as usize
                } else {
                    available
                };
                let chosen = output.subrange(0, batch.member_count_spec() as int);
                ready_scan_facts::<C>(
                    before.slots@,
                    before.cursor,
                    C as nat,
                    limit as nat,
                );
                assert(chosen.len() <= before_output.len());
                dispatch_selected_output_facts(before_output, chosen);
                self.dispatch_enabled_compose_refines_some(
                    before,
                    before_output,
                    output,
                    batch.epoch_spec().value,
                    limit,
                    *batch,
                    chosen,
                );
            }
        }
    }

    pub(crate) proof fn apply_dispatch_basic(
        &self,
        before: &Self,
        before_output: Seq<RequestId>,
        output: Seq<RequestId>,
        result: &Result<Option<DispatchBatch>, SchedulerError>,
    )
        requires
            before.basic_invariant(),
            self.dispatch_execution_refines(before, before_output, output, result),
        ensures self.basic_invariant(),
    {
        match result {
            Err(_) | Ok(None) => {
                reveal(Scheduler::dispatch_execution_refines);
                self.same_scalars_preserves_basic(before);
            }
            Ok(Some(batch)) => {
                reveal(Scheduler::dispatch_execution_refines);
                let chosen = output.subrange(0, batch.member_count_spec() as int);
                self.dispatch_commit_preserves_basic(
                    before,
                    chosen,
                    self.cursor,
                    batch.epoch_spec().value,
                );
            }
        }
    }

    pub(crate) proof fn apply_dispatch_identity(
        &self,
        before: &Self,
        before_output: Seq<RequestId>,
        output: Seq<RequestId>,
        result: &Result<Option<DispatchBatch>, SchedulerError>,
    )
        requires
            before.basic_invariant(),
            self.dispatch_execution_refines(before, before_output, output, result),
        ensures self.identity_frame(before),
    {
        match result {
            Err(_) | Ok(None) => {
                reveal(Scheduler::dispatch_execution_refines);
                self.dispatch_enabled_compose_identity_none(before);
            }
            Ok(Some(batch)) => {
                reveal(Scheduler::dispatch_execution_refines);
                let chosen = output.subrange(0, batch.member_count_spec() as int);
                self.dispatch_enabled_compose_identity_some(
                    before,
                    *batch,
                    chosen,
                    batch.epoch_spec().value,
                );
            }
        }
    }

    fn completion_preflight(
        &self,
        batch: BatchRecord,
        observed: u64,
        permits: &[Option<KvQuiescencePermit>],
    ) -> (result: Result<(), SchedulerError>)
        requires
            self.basic_invariant(),
            self.batch_len > 0,
            batch == self.batch_ring@[self.batch_head as int],
            batch.member_count > 0,
            batch.member_count <= self.member_len,
            batch.member_count <= permits.len(),
            batch.epoch.value == observed,
        ensures
            result == if option_prefix_empty(permits@, batch.member_count as nat) {
                Ok(())
            } else {
                Err(SchedulerError::CompletionStorageNotEmpty)
            },
    {
        proof {
            self.basic_implies_scalar();
            self.pending_completion_members_valid(observed);
        }
        assert(C > 0) by {
            reveal(Scheduler::scalar_invariant);
        }
        assert(self.member_head < C) by {
            reveal(Scheduler::scalar_invariant);
        }
        assert(self.member_len <= C) by {
            reveal(Scheduler::scalar_invariant);
        }
        let mut checked = 0;
        let mut check_head = self.member_head;
        while checked < batch.member_count
            invariant
                C > 0,
                self.member_head < C,
                checked <= batch.member_count,
                batch.member_count <= self.member_len,
                self.member_len <= C,
                batch.member_count <= permits.len(),
                check_head < C,
                forall|offset: int|
                    0 <= offset < batch.member_count ==>
                        #[trigger] self.completion_member_valid(offset, observed),
                check_head as int == ring_position_or_head::<C>(
                    self.member_head,
                    checked as nat,
                ),
                option_prefix_empty(permits@, checked as nat),
            decreases batch.member_count - checked,
        {
            if permits[checked].is_some() {
                assert(!option_prefix_empty(permits@, batch.member_count as nat)) by {
                    reveal(option_prefix_empty);
                }
                return Err(SchedulerError::CompletionStorageNotEmpty);
            }
            assert(checked < C);
            assert(check_head as int
                == ring_position::<C>(self.member_head, checked as nat)) by {
                reveal(ring_position_or_head);
            }
            assert(self.completion_member_valid(checked as int, observed));
            proof {
                reveal(Scheduler::completion_member_valid);
            }
            let handle = self.member_ring[check_head];
            if handle.slot() as usize >= C {
                assert(false);
                return Err(SchedulerError::InvariantViolation);
            }
            let slot = self.slots[handle.slot() as usize];
            if slot.generation != handle.generation() || slot.active_epoch != observed {
                assert(false);
                return Err(SchedulerError::InvariantViolation);
            }
            match slot.state {
                RequestState::InFlight | RequestState::Retiring => {}
                _ => {
                    assert(false);
                    return Err(SchedulerError::InvariantViolation);
                }
            }
            proof {
                ring_position_or_head_next::<C>(self.member_head, checked as nat);
            }
            check_head = advance::<C>(check_head);
            checked += 1;
        }
        Ok(())
    }

    proof fn completion_success_refinement(
        &self,
        before: &Self,
        completion_epoch: u64,
        before_permits: Seq<Option<KvQuiescencePermit>>,
        permits: Seq<Option<KvQuiescencePermit>>,
        count: usize,
    )
        requires
            before.basic_invariant(),
            before.batch_len > 0,
            completion_expected_error::<C>(before, completion_epoch, before_permits).is_none(),
            completion_epoch == before.completed + 1,
            count == before.batch_ring@[before.batch_head as int].member_count,
            count <= C,
            count <= before_permits.len(),
            self.slots@ == before.slots@,
            self.free_ring@ == before.free_ring@,
            self.free_head == before.free_head,
            self.free_len == before.free_len,
            self.reclaim_ring@ == before.reclaim_ring@,
            self.reclaim_head == before.reclaim_head,
            self.reclaim_len == before.reclaim_len,
            self.member_ring@ == before.member_ring@,
            self.member_head == ring_advance::<C>(before.member_head, count as nat),
            self.member_len + count == before.member_len,
            self.batch_ring@ == before.batch_ring@,
            self.batch_head == next_position::<C>(before.batch_head),
            self.batch_len + 1 == before.batch_len,
            self.cursor == before.cursor,
            self.submitted == before.submitted,
            self.completed == completion_epoch,
            self.live_count == before.live_count,
            permits == completed_permits::<C>(
                before_permits,
                before.member_ring@,
                before.member_head,
                count as nat,
                completion_epoch,
            ),
        ensures self.completion_refines(
                before,
                completion_epoch,
                before_permits,
                permits,
                &Ok(count),
            ),
    {
        assert(self.completion_refines(
            before,
            completion_epoch,
            before_permits,
            permits,
            &Ok(count),
        )) by {
            before.pending_batch_facts();
            completed_permits_facts::<C>(
                before_permits,
                before.member_ring@,
                before.member_head,
                count as nat,
                completion_epoch,
            );
            reveal(Scheduler::completion_refines);
            reveal(Scheduler::slot_model);
        }
    }

    proof fn completion_completed_batch_from_refinement(
        &self,
        before: &Self,
        completion_epoch: u64,
        before_permits: Seq<Option<KvQuiescencePermit>>,
        permits: Seq<Option<KvQuiescencePermit>>,
        count: usize,
    )
        requires
            before.basic_invariant(),
            self.completion_refines(
                before,
                completion_epoch,
                before_permits,
                permits,
                &Ok(count),
            ),
        ensures self.completed_batch_refines(before, permits, count),
    {
        before.pending_batch_facts();
        reveal(Scheduler::completion_refines);
        assert forall|offset: int| 0 <= offset < count implies match
            #[trigger] permits[offset]
        {
            Some(permit) => {
                let request = permit.request_spec();
                &&& request.slot_spec() < C
                &&& before.pending_member_spec(offset as usize) == Some(request)
                &&& self.state_spec(request) == before.state_spec(request)
                &&& (self.state_spec(request) == Some(RequestState::InFlight)
                    || self.state_spec(request) == Some(RequestState::Retiring))
                &&& (self.state_spec(request) == Some(RequestState::Retiring) ==>
                    self.detachment_ready(request, permit.origin_spec()))
            }
            None => false,
        } by {
            let request = before.member_ring@[
                ring_position::<C>(before.member_head, offset as nat)
            ];
            assert(permits[offset].is_some());
            assert(permits[offset].unwrap().request_spec() == request);
            before.pending_member_facts(offset as usize);
            assert(request.slot_spec() < C);
            assert(self.slots@[request.slot_spec() as int]
                == before.slots@[request.slot_spec() as int]);
            assert(before.slots@[request.slot_spec() as int].generation
                == request.generation_spec());
            assert(before.slots@[request.slot_spec() as int].state
                == RequestState::InFlight
                || before.slots@[request.slot_spec() as int].state
                    == RequestState::Retiring);
            reveal(Scheduler::pending_member_spec);
            reveal(Scheduler::state_spec);
            reveal(Scheduler::slot_model);
            reveal(Scheduler::detachment_ready_inner);
            reveal(Scheduler::slot_generation_spec);
        }
        assert forall|left: int, right: int| 0 <= left < right < count implies
            permits[left].unwrap().request_spec().slot_spec()
                != permits[right].unwrap().request_spec().slot_spec()
        by {
            assert(request_ring_slots_differ::<C>(
                before.member_ring@,
                before.member_head,
                left,
                right,
            ));
            reveal(request_ring_slots_differ);
        }
        reveal(Scheduler::completed_batch_refines);
        reveal(Scheduler::pending_batch_member_count_spec);
    }

    proof fn completion_public_from_refinement(
        &self,
        before: &Self,
        completion_epoch: u64,
        before_permits: Seq<Option<KvQuiescencePermit>>,
        permits: Seq<Option<KvQuiescencePermit>>,
        count: usize,
    )
        requires
            before.basic_invariant(),
            self.completion_refines(
                before,
                completion_epoch,
                before_permits,
                permits,
                &Ok(count),
            ),
            self.completed_batch_refines(before, permits, count),
        ensures
            self.identity_frame(before),
            count == before.pending_batch_member_count_spec(),
            self.completed_epoch_spec().value == completion_epoch,
            count <= permits.len(),
            forall|offset: int| count <= offset < permits.len() ==>
                #[trigger] permits[offset] == before_permits[offset],
            forall|offset: int| 0 <= offset < count ==> match
                #[trigger] permits[offset]
            {
                Some(permit) => {
                    let request = permit.request_spec();
                    &&& request.slot_spec() < C
                    &&& before.pending_member_spec(offset as usize) == Some(request)
                    &&& self.state_spec(request) == before.state_spec(request)
                    &&& (self.state_spec(request) == Some(RequestState::InFlight)
                        || self.state_spec(request) == Some(RequestState::Retiring))
                    &&& (self.slot_model(request.slot_spec() as int).state
                        == RequestState::Retiring ==>
                            self.detachment_ready(request, permit.origin_spec()))
                }
                None => false,
            },
    {
        reveal(Scheduler::completion_refines);
        reveal(Scheduler::identity_frame);
        reveal(Scheduler::slot_generation_spec);
        reveal(Scheduler::slot_is_live_spec);
        reveal(Scheduler::pending_batch_member_count_spec);
        reveal(Scheduler::pending_member_spec);
        reveal(Scheduler::completed_epoch_spec);
        reveal(Scheduler::state_spec);
        reveal(Scheduler::slot_model);
        reveal(Scheduler::detachment_ready_inner);
    }

    proof fn completion_success_postconditions(
        &self,
        before: &Self,
        completion_epoch: u64,
        before_permits: Seq<Option<KvQuiescencePermit>>,
        permits: Seq<Option<KvQuiescencePermit>>,
        count: usize,
    )
        requires
            before.basic_invariant(),
            before.batch_len > 0,
            completion_expected_error::<C>(before, completion_epoch, before_permits).is_none(),
            completion_epoch == before.completed + 1,
            count == before.batch_ring@[before.batch_head as int].member_count,
            count <= C,
            count <= before_permits.len(),
            self.slots@ == before.slots@,
            self.free_ring@ == before.free_ring@,
            self.free_head == before.free_head,
            self.free_len == before.free_len,
            self.reclaim_ring@ == before.reclaim_ring@,
            self.reclaim_head == before.reclaim_head,
            self.reclaim_len == before.reclaim_len,
            self.member_ring@ == before.member_ring@,
            self.member_head == ring_advance::<C>(before.member_head, count as nat),
            self.member_len + count == before.member_len,
            self.batch_ring@ == before.batch_ring@,
            self.batch_head == next_position::<C>(before.batch_head),
            self.batch_len + 1 == before.batch_len,
            self.cursor == before.cursor,
            self.submitted == before.submitted,
            self.completed == completion_epoch,
            self.live_count == before.live_count,
            permits == completed_permits::<C>(
                before_permits,
                before.member_ring@,
                before.member_head,
                count as nat,
                completion_epoch,
            ),
        ensures
            self.completion_refines(
                before,
                completion_epoch,
                before_permits,
                permits,
                &Ok(count),
            ),
            self.identity_frame(before),
            self.completed_batch_refines(before, permits, count),
            count == before.pending_batch_member_count_spec(),
            self.completed_epoch_spec().value == completion_epoch,
            count <= permits.len(),
            forall|offset: int| count <= offset < permits.len() ==>
                #[trigger] permits[offset] == before_permits[offset],
            forall|offset: int| 0 <= offset < count ==> match
                #[trigger] permits[offset]
            {
                Some(permit) => {
                    let request = permit.request_spec();
                    &&& request.slot_spec() < C
                    &&& before.pending_member_spec(offset as usize) == Some(request)
                    &&& self.state_spec(request) == before.state_spec(request)
                    &&& (self.state_spec(request) == Some(RequestState::InFlight)
                        || self.state_spec(request) == Some(RequestState::Retiring))
                    &&& (self.slot_model(request.slot_spec() as int).state
                        == RequestState::Retiring ==>
                            self.detachment_ready(request, permit.origin_spec()))
                }
                None => false,
            },
    {
        self.completion_success_refinement(
            before,
            completion_epoch,
            before_permits,
            permits,
            count,
        );
        self.completion_completed_batch_from_refinement(
            before,
            completion_epoch,
            before_permits,
            permits,
            count,
        );
        self.completion_public_from_refinement(
            before,
            completion_epoch,
            before_permits,
            permits,
            count,
        );
    }

    proof fn completion_preserves_scalar(
        &self,
        before: &Self,
        completion_epoch: u64,
        before_permits: Seq<Option<KvQuiescencePermit>>,
        permits: Seq<Option<KvQuiescencePermit>>,
        count: usize,
    )
        requires
            before.basic_invariant(),
            self.completion_refines(
                before,
                completion_epoch,
                before_permits,
                permits,
                &Ok(count),
            ),
        ensures self.scalar_invariant(),
    {
        before.basic_implies_scalar();
        before.basic_implies_batch_ring();
        reveal(Scheduler::completion_refines);
        before.pending_batch_facts();
        reveal(Scheduler::scalar_invariant);
        reveal(Scheduler::batch_ring_invariant);
        ring_advance_bounds::<C>(before.member_head, count as nat);
        assert(next_position::<C>(before.batch_head) < C) by {
            reveal(next_position);
        }
        batch_member_sum_pop::<C>(
            before.batch_ring@,
            before.batch_head,
            before.batch_len as nat,
        );
        assert forall|offset: int| 0 <= offset < self.batch_len implies
            (#[trigger] self.batch_ring@[
                ring_position::<C>(self.batch_head, offset as nat)
            ].member_count) > 0 by {
            ring_position_after_pop::<C>(before.batch_head, offset as nat);
            assert(offset + 1 < before.batch_len);
        }
        positive_batch_count_le_sum::<C>(
            self.batch_ring@,
            self.batch_head,
            self.batch_len as nat,
        );
        assert(batch_member_sum::<C>(
            self.batch_ring@,
            self.batch_head,
            self.batch_len as nat,
        ) == self.member_len);
    }

    proof fn completion_preserves_slots(
        &self,
        before: &Self,
        completion_epoch: u64,
        before_permits: Seq<Option<KvQuiescencePermit>>,
        permits: Seq<Option<KvQuiescencePermit>>,
        count: usize,
    )
        requires
            before.basic_invariant(),
            self.completion_refines(
                before,
                completion_epoch,
                before_permits,
                permits,
                &Ok(count),
            ),
        ensures self.slot_invariant(),
    {
        before.basic_implies_slots();
        reveal(Scheduler::slot_invariant);
        reveal(Scheduler::completion_refines);
    }

    proof fn completion_preserves_free_ring(
        &self,
        before: &Self,
        completion_epoch: u64,
        before_permits: Seq<Option<KvQuiescencePermit>>,
        permits: Seq<Option<KvQuiescencePermit>>,
        count: usize,
    )
        requires
            before.basic_invariant(),
            self.completion_refines(
                before,
                completion_epoch,
                before_permits,
                permits,
                &Ok(count),
            ),
        ensures self.free_ring_invariant(),
    {
        before.basic_implies_free_ring();
        reveal(Scheduler::free_ring_invariant);
        reveal(Scheduler::completion_refines);
    }

    proof fn completion_preserves_reclaim_ring(
        &self,
        before: &Self,
        completion_epoch: u64,
        before_permits: Seq<Option<KvQuiescencePermit>>,
        permits: Seq<Option<KvQuiescencePermit>>,
        count: usize,
    )
        requires
            before.basic_invariant(),
            self.completion_refines(
                before,
                completion_epoch,
                before_permits,
                permits,
                &Ok(count),
            ),
        ensures self.reclaim_ring_invariant(),
    {
        before.basic_implies_reclaim_ring();
        reveal(Scheduler::reclaim_ring_invariant);
        reveal(Scheduler::completion_refines);
    }

    proof fn completion_member_entry_preserved(
        &self,
        before: &Self,
        completion_epoch: u64,
        before_permits: Seq<Option<KvQuiescencePermit>>,
        permits: Seq<Option<KvQuiescencePermit>>,
        count: usize,
        offset: int,
    )
        requires
            before.basic_invariant(),
            self.completion_refines(
                before,
                completion_epoch,
                before_permits,
                permits,
                &Ok(count),
            ),
            self.batch_ring_invariant(),
            0 <= offset < self.member_len,
        ensures self.member_entry_valid(offset),
    {
        reveal(Scheduler::completion_refines);
        before.basic_implies_scalar();
        before.basic_implies_member_entries();
        self.completion_batch_sum_preserved(
            before,
            completion_epoch,
            before_permits,
            permits,
            count,
        );
        batch_member_owner::<C>(
            self.batch_ring@,
            self.batch_head,
            self.batch_len as nat,
            offset as nat,
        );
        let batch_offset = choose|batch_offset: int|
            0 <= batch_offset < self.batch_len
                && (#[trigger] batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    batch_offset as nat,
                )) <= offset < batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    batch_offset as nat + 1,
                );
        let old_offset = count + offset;
        assert(0 <= old_offset < before.member_len);
        assert(before.member_entry_valid(old_offset)) by {
            reveal(Scheduler::member_entries_invariant);
        }
        ring_position_after_advance::<C>(
            before.member_head,
            count as nat,
            offset as nat,
        );
        let batch = self.batch_ring@[
            ring_position::<C>(self.batch_head, batch_offset as nat)
        ];
        assert(batch.member_count > 0
            && batch.epoch.value as int
                == self.completed as int + batch_offset + 1
            && batch.epoch.value <= self.submitted) by {
            reveal(Scheduler::batch_ring_invariant);
        }
        assert({
            let handle = self.member_ring@[
                ring_position::<C>(self.member_head, offset as nat)
            ];
            self.slots@[handle.slot_spec() as int].active_epoch == batch.epoch.value
        }) by {
            reveal(Scheduler::batch_ring_invariant);
        }
        reveal(Scheduler::member_entry_valid);
    }

    proof fn completion_member_entries_preserved(
        &self,
        before: &Self,
        completion_epoch: u64,
        before_permits: Seq<Option<KvQuiescencePermit>>,
        permits: Seq<Option<KvQuiescencePermit>>,
        count: usize,
    )
        requires
            before.basic_invariant(),
            self.completion_refines(
                before,
                completion_epoch,
                before_permits,
                permits,
                &Ok(count),
            ),
            self.batch_ring_invariant(),
        ensures self.member_entries_invariant(),
    {
        assert forall|offset: int| 0 <= offset < self.member_len implies
            #[trigger] self.member_entry_valid(offset) by {
            self.completion_member_entry_preserved(
                before,
                completion_epoch,
                before_permits,
                permits,
                count,
                offset,
            );
        }
        reveal(Scheduler::member_entries_invariant);
    }

    proof fn completion_member_distinct_at(
        &self,
        before: &Self,
        completion_epoch: u64,
        before_permits: Seq<Option<KvQuiescencePermit>>,
        permits: Seq<Option<KvQuiescencePermit>>,
        count: usize,
        left: int,
        right: int,
    )
        requires
            before.basic_invariant(),
            self.completion_refines(
                before,
                completion_epoch,
                before_permits,
                permits,
                &Ok(count),
            ),
            0 <= left < self.member_len,
            0 <= right < self.member_len,
            left != right,
        ensures
            request_ring_slots_differ::<C>(
                self.member_ring@,
                self.member_head,
                left,
                right,
            ),
    {
        reveal(Scheduler::completion_refines);
        before.basic_implies_scalar();
        before.basic_implies_member_ring();
        assert(request_ring_slots_differ::<C>(
            before.member_ring@,
            before.member_head,
            count + left,
            count + right,
        )) by {
            reveal(Scheduler::member_ring_invariant);
            reveal(Scheduler::member_distinct_invariant);
        }
        ring_position_after_advance::<C>(
            before.member_head,
            count as nat,
            left as nat,
        );
        ring_position_after_advance::<C>(
            before.member_head,
            count as nat,
            right as nat,
        );
        reveal(request_ring_slots_differ);
    }

    proof fn completion_member_distinct_preserved(
        &self,
        before: &Self,
        completion_epoch: u64,
        before_permits: Seq<Option<KvQuiescencePermit>>,
        permits: Seq<Option<KvQuiescencePermit>>,
        count: usize,
    )
        requires
            before.basic_invariant(),
            self.completion_refines(
                before,
                completion_epoch,
                before_permits,
                permits,
                &Ok(count),
            ),
        ensures self.member_distinct_invariant(),
    {
        assert forall|left: int, right: int|
            0 <= left < self.member_len
                && 0 <= right < self.member_len
                && left != right implies
                    #[trigger] request_ring_slots_differ::<C>(
                        self.member_ring@,
                        self.member_head,
                        left,
                        right,
                    ) by {
            self.completion_member_distinct_at(
                before,
                completion_epoch,
                before_permits,
                permits,
                count,
                left,
                right,
            );
        }
        reveal(Scheduler::member_distinct_invariant);
    }

    proof fn completion_member_contains_implies_live(
        &self,
        before: &Self,
        completion_epoch: u64,
        before_permits: Seq<Option<KvQuiescencePermit>>,
        permits: Seq<Option<KvQuiescencePermit>>,
        count: usize,
        slot_index: int,
    )
        requires
            before.basic_invariant(),
            self.completion_refines(
                before,
                completion_epoch,
                before_permits,
                permits,
                &Ok(count),
            ),
            self.member_entries_invariant(),
            0 <= slot_index < C,
            request_ring_contains_slot::<C>(
                self.member_ring@,
                self.member_head,
                self.member_len,
                slot_index,
            ),
        ensures
            (self.slots@[slot_index].state == RequestState::InFlight
                || self.slots@[slot_index].state == RequestState::Retiring)
                && self.completed < self.slots@[slot_index].active_epoch,
    {
        let offset = choose|offset: int| 0 <= offset < self.member_len
            && (#[trigger] self.member_ring@[
                ring_position::<C>(self.member_head, offset as nat)
            ].slot_spec()) == slot_index;
        assert(self.member_entry_valid(offset)) by {
            reveal(Scheduler::member_entries_invariant);
        }
        reveal(Scheduler::member_entry_valid);
    }

    proof fn completion_member_live_implies_contains(
        &self,
        before: &Self,
        completion_epoch: u64,
        before_permits: Seq<Option<KvQuiescencePermit>>,
        permits: Seq<Option<KvQuiescencePermit>>,
        count: usize,
        slot_index: int,
    )
        requires
            before.basic_invariant(),
            self.completion_refines(
                before,
                completion_epoch,
                before_permits,
                permits,
                &Ok(count),
            ),
            0 <= slot_index < C,
            (self.slots@[slot_index].state == RequestState::InFlight
                || self.slots@[slot_index].state == RequestState::Retiring)
                && self.completed < self.slots@[slot_index].active_epoch,
        ensures
            request_ring_contains_slot::<C>(
                self.member_ring@,
                self.member_head,
                self.member_len,
                slot_index,
            ),
    {
        reveal(Scheduler::completion_refines);
        before.basic_implies_member_ring();
        assert((before.slots@[slot_index].state == RequestState::InFlight
            || before.slots@[slot_index].state == RequestState::Retiring)
            && before.completed < before.slots@[slot_index].active_epoch);
        assert(request_ring_contains_slot::<C>(
            before.member_ring@,
            before.member_head,
            before.member_len,
            slot_index,
        )) by {
            reveal(Scheduler::member_ring_invariant);
            reveal(Scheduler::member_membership_invariant);
        }
        let old_offset = choose|old_offset: int| 0 <= old_offset < before.member_len
            && (#[trigger] before.member_ring@[
                ring_position::<C>(before.member_head, old_offset as nat)
            ].slot_spec()) == slot_index;
        self.completion_member_old_offset_is_suffix(
            before,
            completion_epoch,
            before_permits,
            permits,
            count,
            slot_index,
            old_offset,
        );
        self.completion_member_suffix_is_contained(
            before,
            completion_epoch,
            before_permits,
            permits,
            count,
            slot_index,
            old_offset,
        );
    }

    proof fn completion_member_old_offset_is_suffix(
        &self,
        before: &Self,
        completion_epoch: u64,
        before_permits: Seq<Option<KvQuiescencePermit>>,
        permits: Seq<Option<KvQuiescencePermit>>,
        count: usize,
        slot_index: int,
        old_offset: int,
    )
        requires
            before.basic_invariant(),
            self.completion_refines(
                before,
                completion_epoch,
                before_permits,
                permits,
                &Ok(count),
            ),
            0 <= slot_index < C,
            (self.slots@[slot_index].state == RequestState::InFlight
                || self.slots@[slot_index].state == RequestState::Retiring)
                && self.completed < self.slots@[slot_index].active_epoch,
            0 <= old_offset < before.member_len,
            before.member_ring@[
                ring_position::<C>(before.member_head, old_offset as nat)
            ].slot_spec() == slot_index,
        ensures count <= old_offset,
    {
        reveal(Scheduler::completion_refines);
        if old_offset < count {
            before.pending_batch_facts();
            assert(before.slots@[slot_index].active_epoch == completion_epoch);
            assert(false);
        }
    }

    proof fn completion_member_suffix_fields(
        &self,
        before: &Self,
        completion_epoch: u64,
        before_permits: Seq<Option<KvQuiescencePermit>>,
        permits: Seq<Option<KvQuiescencePermit>>,
        count: usize,
    )
        requires
            self.completion_refines(
                before,
                completion_epoch,
                before_permits,
                permits,
                &Ok(count),
            ),
        ensures
            self.member_len + count == before.member_len,
            self.member_head == ring_advance::<C>(before.member_head, count as nat),
            self.member_ring@ == before.member_ring@,
    {
        reveal(Scheduler::completion_refines);
    }

    proof fn completion_member_suffix_is_contained(
        &self,
        before: &Self,
        completion_epoch: u64,
        before_permits: Seq<Option<KvQuiescencePermit>>,
        permits: Seq<Option<KvQuiescencePermit>>,
        count: usize,
        slot_index: int,
        old_offset: int,
    )
        requires
            before.basic_invariant(),
            self.completion_refines(
                before,
                completion_epoch,
                before_permits,
                permits,
                &Ok(count),
            ),
            0 <= slot_index < C,
            count <= old_offset < before.member_len,
            before.member_ring@[
                ring_position::<C>(before.member_head, old_offset as nat)
            ].slot_spec() == slot_index,
        ensures
            request_ring_contains_slot::<C>(
                self.member_ring@,
                self.member_head,
                self.member_len,
                slot_index,
            ),
    {
        self.completion_member_suffix_fields(
            before,
            completion_epoch,
            before_permits,
            permits,
            count,
        );
        before.basic_implies_scalar();
        ring_advance_bounds::<C>(before.member_head, count as nat);
        request_ring_suffix_contains::<C>(
            self.member_ring@,
            before.member_head,
            self.member_head,
            before.member_len,
            self.member_len,
            count,
            old_offset,
            slot_index,
        );
    }

    proof fn completion_member_membership_at(
        &self,
        before: &Self,
        completion_epoch: u64,
        before_permits: Seq<Option<KvQuiescencePermit>>,
        permits: Seq<Option<KvQuiescencePermit>>,
        count: usize,
        slot_index: int,
    )
        requires
            before.basic_invariant(),
            self.completion_refines(
                before,
                completion_epoch,
                before_permits,
                permits,
                &Ok(count),
            ),
            self.member_entries_invariant(),
            0 <= slot_index < C,
        ensures
            ((self.slots@[slot_index].state == RequestState::InFlight
                || self.slots@[slot_index].state == RequestState::Retiring)
                && self.completed < self.slots@[slot_index].active_epoch)
                == request_ring_contains_slot::<C>(
                    self.member_ring@,
                    self.member_head,
                    self.member_len,
                    slot_index,
                ),
    {
        if request_ring_contains_slot::<C>(
            self.member_ring@,
            self.member_head,
            self.member_len,
            slot_index,
        ) {
            self.completion_member_contains_implies_live(
                before,
                completion_epoch,
                before_permits,
                permits,
                count,
                slot_index,
            );
        }
        if (self.slots@[slot_index].state == RequestState::InFlight
            || self.slots@[slot_index].state == RequestState::Retiring)
            && self.completed < self.slots@[slot_index].active_epoch {
            self.completion_member_live_implies_contains(
                before,
                completion_epoch,
                before_permits,
                permits,
                count,
                slot_index,
            );
        }
    }

    proof fn completion_member_membership_preserved(
        &self,
        before: &Self,
        completion_epoch: u64,
        before_permits: Seq<Option<KvQuiescencePermit>>,
        permits: Seq<Option<KvQuiescencePermit>>,
        count: usize,
    )
        requires
            before.basic_invariant(),
            self.completion_refines(
                before,
                completion_epoch,
                before_permits,
                permits,
                &Ok(count),
            ),
            self.member_entries_invariant(),
        ensures self.member_membership_invariant(),
    {
        assert forall|slot_index: int| 0 <= slot_index < C implies
            (((#[trigger] self.slots@[slot_index].state == RequestState::InFlight
                || self.slots@[slot_index].state == RequestState::Retiring)
                && self.completed < self.slots@[slot_index].active_epoch)
                == request_ring_contains_slot::<C>(
                    self.member_ring@,
                    self.member_head,
                    self.member_len,
                    slot_index,
                )) by {
            self.completion_member_membership_at(
                before,
                completion_epoch,
                before_permits,
                permits,
                count,
                slot_index,
            );
        }
        reveal(Scheduler::member_membership_invariant);
    }

    proof fn completion_preserves_member_ring(
        &self,
        before: &Self,
        completion_epoch: u64,
        before_permits: Seq<Option<KvQuiescencePermit>>,
        permits: Seq<Option<KvQuiescencePermit>>,
        count: usize,
    )
        requires
            before.basic_invariant(),
            self.completion_refines(
                before,
                completion_epoch,
                before_permits,
                permits,
                &Ok(count),
            ),
        ensures self.member_ring_invariant(),
    {
        self.completion_preserves_batch_ring(
            before,
            completion_epoch,
            before_permits,
            permits,
            count,
        );
        self.completion_member_entries_preserved(
            before,
            completion_epoch,
            before_permits,
            permits,
            count,
        );
        self.completion_member_distinct_preserved(
            before,
            completion_epoch,
            before_permits,
            permits,
            count,
        );
        self.completion_member_membership_preserved(
            before,
            completion_epoch,
            before_permits,
            permits,
            count,
        );
        reveal(Scheduler::member_ring_invariant);
    }

    proof fn completion_batch_sum_preserved(
        &self,
        before: &Self,
        completion_epoch: u64,
        before_permits: Seq<Option<KvQuiescencePermit>>,
        permits: Seq<Option<KvQuiescencePermit>>,
        count: usize,
    )
        requires
            before.basic_invariant(),
            self.completion_refines(
                before,
                completion_epoch,
                before_permits,
                permits,
                &Ok(count),
            ),
        ensures
            batch_member_sum::<C>(
                self.batch_ring@,
                self.batch_head,
                self.batch_len as nat,
            ) == self.member_len,
    {
        before.basic_implies_batch_ring();
        reveal(Scheduler::completion_refines);
        reveal(Scheduler::batch_ring_invariant);
        batch_member_sum_pop::<C>(
            before.batch_ring@,
            before.batch_head,
            before.batch_len as nat,
        );
    }

    proof fn completion_batch_header_preserved(
        &self,
        before: &Self,
        completion_epoch: u64,
        before_permits: Seq<Option<KvQuiescencePermit>>,
        permits: Seq<Option<KvQuiescencePermit>>,
        count: usize,
        batch_offset: int,
    )
        requires
            before.basic_invariant(),
            self.completion_refines(
                before,
                completion_epoch,
                before_permits,
                permits,
                &Ok(count),
            ),
            0 <= batch_offset < self.batch_len,
        ensures {
            let batch = self.batch_ring@[
                ring_position::<C>(self.batch_head, batch_offset as nat)
            ];
            &&& batch.member_count > 0
            &&& batch.epoch.value as int == self.completed as int + batch_offset + 1
            &&& batch.epoch.value <= self.submitted
        },
    {
        reveal(Scheduler::completion_refines);
        before.basic_implies_batch_entry(batch_offset + 1);
        ring_position_after_pop::<C>(before.batch_head, batch_offset as nat);
    }

    proof fn completion_batch_members_preserved(
        &self,
        before: &Self,
        completion_epoch: u64,
        before_permits: Seq<Option<KvQuiescencePermit>>,
        permits: Seq<Option<KvQuiescencePermit>>,
        count: usize,
        batch_offset: int,
    )
        requires
            before.basic_invariant(),
            self.completion_refines(
                before,
                completion_epoch,
                before_permits,
                permits,
                &Ok(count),
            ),
            0 <= batch_offset < self.batch_len,
        ensures {
            let batch = self.batch_ring@[
                ring_position::<C>(self.batch_head, batch_offset as nat)
            ];
            forall|member_offset: int|
                batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    batch_offset as nat,
                ) <= member_offset < batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    batch_offset as nat + 1,
                ) ==> {
                    let handle = #[trigger] self.member_ring@[
                        ring_position::<C>(self.member_head, member_offset as nat)
                    ];
                    self.slots@[handle.slot_spec() as int].active_epoch == batch.epoch.value
                }
        },
    {
        reveal(Scheduler::completion_refines);
        before.basic_implies_scalar();
        before.basic_implies_batch_entry(batch_offset + 1);
        self.completion_batch_sum_preserved(
            before,
            completion_epoch,
            before_permits,
            permits,
            count,
        );
        batch_member_sum_pop::<C>(
            before.batch_ring@,
            before.batch_head,
            (batch_offset + 1) as nat,
        );
        batch_member_sum_pop::<C>(
            before.batch_ring@,
            before.batch_head,
            (batch_offset + 2) as nat,
        );
        batch_member_sum_monotonic::<C>(
            self.batch_ring@,
            self.batch_head,
            (batch_offset + 1) as nat,
            self.batch_len as nat,
        );
        ring_position_after_pop::<C>(before.batch_head, batch_offset as nat);
        assert forall|member_offset: int|
            batch_member_sum::<C>(
                self.batch_ring@,
                self.batch_head,
                batch_offset as nat,
            ) <= member_offset < batch_member_sum::<C>(
                self.batch_ring@,
                self.batch_head,
                batch_offset as nat + 1,
            ) implies {
                let handle = #[trigger] self.member_ring@[
                    ring_position::<C>(self.member_head, member_offset as nat)
                ];
                self.slots@[handle.slot_spec() as int].active_epoch
                    == self.batch_ring@[
                        ring_position::<C>(self.batch_head, batch_offset as nat)
                    ].epoch.value
            } by {
            assert(member_offset < self.member_len);
            assert(count + member_offset < before.member_len);
            ring_position_after_advance::<C>(
                before.member_head,
                count as nat,
                member_offset as nat,
            );
            assert(batch_member_sum::<C>(
                before.batch_ring@,
                before.batch_head,
                (batch_offset + 1) as nat,
            ) <= count + member_offset);
            assert(count + member_offset < batch_member_sum::<C>(
                before.batch_ring@,
                before.batch_head,
                (batch_offset + 2) as nat,
            ));
        }
    }

    proof fn completion_preserves_batch_ring(
        &self,
        before: &Self,
        completion_epoch: u64,
        before_permits: Seq<Option<KvQuiescencePermit>>,
        permits: Seq<Option<KvQuiescencePermit>>,
        count: usize,
    )
        requires
            before.basic_invariant(),
            self.completion_refines(
                before,
                completion_epoch,
                before_permits,
                permits,
                &Ok(count),
            ),
        ensures self.batch_ring_invariant(),
    {
        self.completion_batch_sum_preserved(
            before,
            completion_epoch,
            before_permits,
            permits,
            count,
        );
        assert forall|batch_offset: int| 0 <= batch_offset < self.batch_len implies {
            let batch = #[trigger] self.batch_ring@[
                ring_position::<C>(self.batch_head, batch_offset as nat)
            ];
            &&& batch.member_count > 0
            &&& batch.epoch.value as int == self.completed as int + batch_offset + 1
            &&& batch.epoch.value <= self.submitted
            &&& (forall|member_offset: int|
                batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    batch_offset as nat,
                ) <= member_offset < batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    batch_offset as nat + 1,
                ) ==> {
                    let handle = #[trigger] self.member_ring@[
                        ring_position::<C>(self.member_head, member_offset as nat)
                    ];
                    self.slots@[handle.slot_spec() as int].active_epoch == batch.epoch.value
                })
        } by {
            self.completion_batch_header_preserved(
                before,
                completion_epoch,
                before_permits,
                permits,
                count,
                batch_offset,
            );
            self.completion_batch_members_preserved(
                before,
                completion_epoch,
                before_permits,
                permits,
                count,
                batch_offset,
            );
        }
        reveal(Scheduler::batch_ring_invariant);
    }

    proof fn completion_success_preserves_basic(
        &self,
        before: &Self,
        completion_epoch: u64,
        before_permits: Seq<Option<KvQuiescencePermit>>,
        permits: Seq<Option<KvQuiescencePermit>>,
        count: usize,
    )
        requires
            before.basic_invariant(),
            self.completion_refines(
                before,
                completion_epoch,
                before_permits,
                permits,
                &Ok(count),
            ),
        ensures self.basic_invariant(),
    {
        self.completion_preserves_scalar(
            before, completion_epoch, before_permits, permits, count,
        );
        self.completion_preserves_slots(
            before, completion_epoch, before_permits, permits, count,
        );
        self.completion_preserves_free_ring(
            before, completion_epoch, before_permits, permits, count,
        );
        self.completion_preserves_reclaim_ring(
            before, completion_epoch, before_permits, permits, count,
        );
        self.completion_preserves_member_ring(
            before, completion_epoch, before_permits, permits, count,
        );
        self.completion_preserves_batch_ring(
            before, completion_epoch, before_permits, permits, count,
        );
        reveal(Scheduler::basic_invariant);
    }

    /// Consumes exact quiescence evidence and emits one linear KV permit per
    /// member into caller-owned fixed storage.
    ///
    /// Member slots are not touched. Advancing `completed` derives abstract
    /// `AwaitingKv`/`RetiringQuiescent` phases from each active epoch. Neither
    /// state is dispatchable until the cache consumes its exact permit and
    /// returns finalized or detached evidence.
    pub(crate) fn complete_exact(
        &mut self,
        completion: ExactCompletion,
        permits: &mut [Option<KvQuiescencePermit>],
    ) -> (result: Result<usize, CompletionFailure>)
        requires old(self).basic_invariant(),
        ensures
            final(self).basic_invariant(),
            final(self).completion_refines(
                old(self),
                completion.epoch_spec().value,
                old(permits)@,
                final(permits)@,
                &result,
            ),
            final(self).identity_frame(old(self)),
            match result {
                Ok(count) => {
                    &&& count == old(self).pending_batch_member_count_spec()
                    &&& final(self).completed_epoch_spec() == completion.epoch_spec()
                    &&& final(self).completed_batch_refines(
                        old(self),
                        final(permits)@,
                        count,
                    )
                    &&& count <= final(permits).len()
                    &&& (forall|offset: int| 0 <= offset < count ==> {
                        match #[trigger] final(permits)@[offset] {
                            Some(permit) => {
                                let request = permit.request_spec();
                                &&& request.slot_spec() < C
                                &&& match old(self).pending_member_spec(offset as usize) {
                                    Some(member) => member == request,
                                    None => false,
                                }
                                &&& final(self).state_spec(request)
                                    == old(self).state_spec(request)
                                &&& (final(self).state_spec(request)
                                    == Some(RequestState::InFlight)
                                    || final(self).state_spec(request)
                                        == Some(RequestState::Retiring))
                                &&& (final(self).slot_model(request.slot_spec() as int).state
                                    == RequestState::Retiring ==>
                                        final(self).detachment_ready(
                                            request,
                                            permit.origin_spec(),
                                        ))
                            }
                            None => false,
                        }
                    })
                    &&& (forall|offset: int| count <= offset < final(permits).len() ==>
                        #[trigger] final(permits)@[offset] == old(permits)@[offset])
                }
                Err(failure) => {
                    &&& failure.completion_epoch_spec() == completion.epoch_spec()
                    &&& final(permits)@ == old(permits)@
                    &&& final(self).same_scalars(old(self))
                }
            },
    {
        if self.batch_len == 0 {
            return Err(CompletionFailure {
                error: SchedulerError::NoPendingBatch,
                completion,
            });
        }
        let expected = match self.completed.checked_add(1) {
            Some(epoch) => epoch,
            None => {
                return Err(CompletionFailure {
                    error: SchedulerError::CompletionNotExactNext,
                    completion,
                });
            }
        };
        let observed = completion.epoch().value;
        if observed != expected {
            return Err(CompletionFailure {
                error: SchedulerError::CompletionNotExactNext,
                completion,
            });
        }
        let batch = self.batch_ring[self.batch_head];
        if batch.epoch.value != observed || batch.member_count == 0 {
            return Err(CompletionFailure {
                error: SchedulerError::CompletionEpochMismatch,
                completion,
            });
        }
        if permits.len() < batch.member_count {
            return Err(CompletionFailure {
                error: SchedulerError::CompletionStorageTooSmall,
                completion,
            });
        }
        proof {
            self.pending_batch_facts();
        }
        assert(batch.member_count <= self.member_len);
        assert(self.same_scalars(old(self))) by {
            reveal(Scheduler::same_scalars);
        }
        assert(permits@ == old(permits)@);

        if let Err(error) = self.completion_preflight(batch, observed, permits) {
            return Err(CompletionFailure { error, completion });
        }

        proof {
            self.basic_implies_scalar();
        }
        assert(C > 0) by {
            reveal(Scheduler::scalar_invariant);
        }
        assert(self.member_head < C) by {
            reveal(Scheduler::scalar_invariant);
        }
        assert(batch.member_count <= C) by {
            reveal(Scheduler::scalar_invariant);
        }
        let mut processed = 0;
        let mut member_head = self.member_head;
        proof {
            completed_permits_empty::<C>(
                old(permits)@,
                old(self).member_ring@,
                old(self).member_head,
                observed,
            );
        }
        while processed < batch.member_count
            invariant
                C > 0,
                old(self).member_head < C,
                processed <= batch.member_count,
                batch.member_count <= C,
                processed <= C,
                batch.member_count <= permits.len(),
                permits.len() == old(permits)@.len(),
                batch.member_count <= old(permits)@.len(),
                member_head < C,
                member_head as int == ring_position_or_head::<C>(
                    old(self).member_head,
                    processed as nat,
                ),
                permits@ == completed_permits::<C>(
                    old(permits)@,
                    old(self).member_ring@,
                    old(self).member_head,
                    processed as nat,
                    observed,
                ),
            decreases batch.member_count - processed,
        {
            assert(processed < C);
            assert(processed < permits.len());
            assert(member_head as int
                == ring_position::<C>(old(self).member_head, processed as nat)) by {
                reveal(ring_position_or_head);
            }
            let handle = self.member_ring[member_head];
            assert(handle == old(self).member_ring@[ring_position::<C>(
                old(self).member_head,
                processed as nat,
            )]);
            proof {
                completed_permits_push::<C>(
                    old(permits)@,
                    old(self).member_ring@,
                    old(self).member_head,
                    processed as nat,
                    observed,
                );
            }
            permits[processed] = Some(KvQuiescencePermit {
                request: handle,
                origin: KvQuiescenceOrigin::CompletedExact { epoch: observed },
            });
            proof {
                ring_position_or_head_next::<C>(
                    old(self).member_head,
                    processed as nat,
                );
            }
            member_head = advance::<C>(member_head);
            processed += 1;
        }

        proof {
            completed_permits_facts::<C>(
                old(permits)@,
                old(self).member_ring@,
                old(self).member_head,
                processed as nat,
                observed,
            );
        }

        self.member_head = member_head;
        self.member_len -= batch.member_count;
        self.batch_head = advance::<C>(self.batch_head);
        self.batch_len -= 1;
        self.completed = observed;
        proof {
            ring_advance_matches_position::<C>(old(self).member_head, processed as nat);
            assert(completion_expected_error::<C>(
                old(self),
                observed,
                old(permits)@,
            ).is_none()) by {
                reveal(completion_expected_error);
            }
            self.completion_success_postconditions(
                old(self),
                observed,
                old(permits)@,
                permits@,
                batch.member_count,
            );
            self.completion_success_preserves_basic(
                old(self),
                observed,
                old(permits)@,
                permits@,
                batch.member_count,
            );
        }
        Ok(batch.member_count)
    }

    /// Removes one already-quiescent terminal request from the O(1) reclaim
    /// ring and returns the only authority that can detach its KV state.
    pub(crate) fn take_retiring_permit(
        &mut self,
    ) -> (result: Result<Option<KvQuiescencePermit>, SchedulerError>)
        requires old(self).basic_invariant(),
        ensures
            final(self).basic_invariant(),
            final(self).retiring_permit_refines(old(self), &result),
            final(self).identity_frame(old(self)),
            match &result {
                Ok(Some(permit)) => final(self).detachment_ready(
                    permit.request_spec(),
                    permit.origin_spec(),
                ),
                Ok(None) | Err(_) => true,
            },
    {
        if self.reclaim_len == 0 {
            assert(self.same_scalars(old(self))) by {
                reveal(Scheduler::same_scalars);
            }
            assert(self.retiring_permit_refines(old(self), &Ok(None))) by {
                reveal(Scheduler::retiring_permit_refines);
            }
            return Ok(None);
        }
        proof {
            self.retiring_head_facts();
        }
        let slot_index = self.reclaim_ring[self.reclaim_head];
        assert(slot_index < C);
        if slot_index >= C {
            assert(false);
            return Err(SchedulerError::InvariantViolation);
        }
        let slot = self.slots[slot_index];
        assert(slot.state == RequestState::Retiring);
        assert(slot.active_epoch == NO_EPOCH);
        assert(slot.in_reclaim_ring);
        match slot.state {
            RequestState::Retiring => {}
            _ => return Err(SchedulerError::InvariantViolation),
        }
        if slot.active_epoch != NO_EPOCH {
            assert(false);
            return Err(SchedulerError::InvariantViolation);
        }
        if !slot.in_reclaim_ring {
            assert(false);
            return Err(SchedulerError::InvariantViolation);
        }
        let ghost old_slots = self.slots@;
        self.reclaim_head = advance::<C>(self.reclaim_head);
        self.reclaim_len -= 1;
        self.slots[slot_index].in_reclaim_ring = false;
        assert(self.slots@ == old_slots.update(slot_index as int, self.slots@[slot_index as int]));
        assert(self.retiring_permit_updates(old(self), slot_index)) by {
            reveal(Scheduler::retiring_permit_updates);
            reveal(Scheduler::retiring_reclaim_updates);
        }
        let request = RequestId::new(slot_index_to_u32(slot_index), slot.generation);
        let origin = if slot.last_quiescent_epoch == NO_EPOCH {
            KvQuiescenceOrigin::NeverSubmitted
        } else {
            KvQuiescenceOrigin::CompletedExact {
                epoch: slot.last_quiescent_epoch,
            }
        };
        proof {
            self.retiring_permit_updates_establish_postconditions(
                old(self),
                slot_index,
                request,
                origin,
            );
        }
        Ok(Some(KvQuiescencePermit { request, origin }))
    }

    fn finalized_origin(
        finalized: &KvFinalizedRequest,
    ) -> (result: Result<u64, SchedulerError>)
        ensures result == finalized_origin_spec(finalized),
    {
        reveal(finalized_origin_spec);
        match finalized.origin() {
            KvQuiescenceOrigin::CompletedExact { epoch } => Ok(epoch),
            KvQuiescenceOrigin::NeverSubmitted => Err(SchedulerError::FinalizationMismatch),
        }
    }

    fn finalized_request_preflight(
        &self,
        request: RequestId,
        epoch: u64,
    ) -> (result: Result<(), SchedulerError>)
        ensures result == finalized_request_preflight_spec::<C>(self, request, epoch),
    {
        let slot_index = request.slot() as usize;
        if slot_index >= C {
            let result = Err(SchedulerError::InvalidSlot);
            assert(result == finalized_request_preflight_spec::<C>(self, request, epoch)) by {
                reveal(finalized_request_preflight_spec);
            }
            return result;
        }
        let slot = self.slots[slot_index];
        assert(slot_index as int == request.slot_spec() as int);
        assert(self.slots@[slot_index as int] == slot);
        if slot.generation != request.generation() {
            let result = Err(SchedulerError::FinalizationMismatch);
            assert(result == finalized_request_preflight_spec::<C>(self, request, epoch)) by {
                reveal(finalized_request_preflight_spec);
            }
            return result;
        }
        match slot.state {
            RequestState::InFlight => {}
            RequestState::Vacant | RequestState::Ready | RequestState::Retiring => {
                let result = Err(SchedulerError::FinalizationMismatch);
                assert(result == finalized_request_preflight_spec::<C>(self, request, epoch)) by {
                    reveal(finalized_request_preflight_spec);
                }
                return result;
            }
        }
        if slot.active_epoch == NO_EPOCH {
            let result = Err(SchedulerError::FinalizationMismatch);
            assert(result == finalized_request_preflight_spec::<C>(self, request, epoch)) by {
                reveal(finalized_request_preflight_spec);
            }
            return result;
        }
        if slot.active_epoch > self.completed {
            let result = Err(SchedulerError::FinalizationMismatch);
            assert(result == finalized_request_preflight_spec::<C>(self, request, epoch)) by {
                reveal(finalized_request_preflight_spec);
            }
            return result;
        }
        if slot.active_epoch != epoch {
            let result = Err(SchedulerError::FinalizationMismatch);
            assert(result == finalized_request_preflight_spec::<C>(self, request, epoch)) by {
                reveal(finalized_request_preflight_spec);
            }
            return result;
        }
        let result = Ok(());
        assert(slot.generation == request.generation_spec());
        assert(slot.state == RequestState::InFlight);
        assert(slot.active_epoch == epoch);
        assert(result == finalized_request_preflight_spec::<C>(self, request, epoch)) by {
            reveal(finalized_request_preflight_spec);
        }
        result
    }

    fn finalized_preflight(
        &self,
        finalized: &KvFinalizedRequest,
    ) -> (result: Result<(RequestId, u64), SchedulerError>)
        requires self.basic_invariant(),
        ensures result == finalized_preflight_spec::<C>(self, finalized),
    {
        reveal(finalized_preflight_spec);
        let request = finalized.request();
        let epoch = Self::finalized_origin(finalized)?;
        self.finalized_request_preflight(request, epoch)?;
        Ok((request, epoch))
    }

    proof fn finalized_slot_updates_preserve_scalar(
        &self,
        before: &Self,
        request: RequestId,
        epoch: u64,
    )
        requires
            before.basic_invariant(),
            self.finalized_slot_updates(before, request, epoch),
        ensures self.scalar_invariant(),
    {
        before.basic_implies_scalar();
        reveal(Scheduler::finalized_slot_updates);
        reveal(Scheduler::scalar_invariant);
        let slot_index = request.slot_spec() as int;
        let old_slots = before.slots@;
        live_count_update_nonvacant(
            old_slots,
            slot_index,
            self.slots@[slot_index],
            C as nat,
        );
        nonreclaim_count_update_preserved(
            old_slots,
            slot_index,
            self.slots@[slot_index],
            C as nat,
        );
        assert(live_slot_count(self.slots@, C as nat)
            == live_slot_count(old_slots, C as nat));
    }

    proof fn finalized_slot_updates_preserve_slots(
        &self,
        before: &Self,
        request: RequestId,
        epoch: u64,
    )
        requires
            before.basic_invariant(),
            self.finalized_slot_updates(before, request, epoch),
        ensures self.slot_invariant(),
    {
        before.basic_implies_slots();
        reveal(Scheduler::finalized_slot_updates);
        reveal(Scheduler::slot_invariant);
        reveal(Scheduler::slot_invariant_at);
        let slot_index = request.slot_spec() as int;
        assert(self.slot_invariant_at(slot_index));
        assert forall|observed: int| 0 <= observed < C implies
            #[trigger] self.slot_invariant_at(observed) by {
            if observed != slot_index {
                assert(self.slots@[observed] == before.slots@[observed]);
            }
        }
    }

    proof fn finalized_slot_updates_preserve_free_ring(
        &self,
        before: &Self,
        request: RequestId,
        epoch: u64,
    )
        requires
            before.basic_invariant(),
            self.finalized_slot_updates(before, request, epoch),
        ensures self.free_ring_invariant(),
    {
        before.basic_implies_free_ring();
        reveal(Scheduler::finalized_slot_updates);
        reveal(Scheduler::free_ring_invariant);
        let slot_index = request.slot_spec() as int;
        assert forall|offset: int| 0 <= offset < self.free_len implies {
            let observed = #[trigger] self.free_ring@[
                ring_position::<C>(self.free_head, offset as nat)
            ];
            &&& observed < C
            &&& self.slots@[observed as int].state == RequestState::Vacant
            &&& self.slots@[observed as int].in_free_ring
        } by {
            let observed = self.free_ring@[
                ring_position::<C>(self.free_head, offset as nat)
            ];
            assert(observed as int != slot_index);
            assert(self.slots@[observed as int] == before.slots@[observed as int]);
        }
        assert forall|observed: int| 0 <= observed < C implies
            #[trigger] self.slots@[observed].in_free_ring
                == usize_ring_contains::<C>(
                    self.free_ring@,
                    self.free_head,
                    self.free_len,
                    observed,
                ) by {
            if observed != slot_index {
                assert(self.slots@[observed] == before.slots@[observed]);
            }
        }
    }

    proof fn finalized_slot_updates_preserve_reclaim_ring(
        &self,
        before: &Self,
        request: RequestId,
        epoch: u64,
    )
        requires
            before.basic_invariant(),
            self.finalized_slot_updates(before, request, epoch),
        ensures self.reclaim_ring_invariant(),
    {
        before.basic_implies_reclaim_ring();
        reveal(Scheduler::finalized_slot_updates);
        reveal(Scheduler::reclaim_ring_invariant);
        let slot_index = request.slot_spec() as int;
        assert forall|offset: int| 0 <= offset < self.reclaim_len implies {
            let observed = #[trigger] self.reclaim_ring@[
                ring_position::<C>(self.reclaim_head, offset as nat)
            ];
            &&& observed < C
            &&& self.slots@[observed as int].state == RequestState::Retiring
            &&& self.slots@[observed as int].active_epoch == NO_EPOCH
            &&& self.slots@[observed as int].in_reclaim_ring
        } by {
            let observed = self.reclaim_ring@[
                ring_position::<C>(self.reclaim_head, offset as nat)
            ];
            assert(observed as int != slot_index);
            assert(self.slots@[observed as int] == before.slots@[observed as int]);
        }
        assert forall|observed: int| 0 <= observed < C implies
            #[trigger] self.slots@[observed].in_reclaim_ring
                == usize_ring_contains::<C>(
                    self.reclaim_ring@,
                    self.reclaim_head,
                    self.reclaim_len,
                    observed,
                ) by {
            if observed != slot_index {
                assert(self.slots@[observed] == before.slots@[observed]);
            }
        }
    }

    proof fn finalized_slot_updates_preserve_member_ring(
        &self,
        before: &Self,
        request: RequestId,
        epoch: u64,
    )
        requires
            before.basic_invariant(),
            self.finalized_slot_updates(before, request, epoch),
        ensures self.member_ring_invariant(),
    {
        before.basic_implies_member_ring();
        reveal(Scheduler::finalized_slot_updates);
        reveal(Scheduler::member_ring_invariant);
        reveal(Scheduler::member_entries_invariant);
        reveal(Scheduler::member_membership_invariant);
        let slot_index = request.slot_spec() as int;
        assert forall|offset: int| 0 <= offset < self.member_len implies
            #[trigger] self.member_entry_valid(offset) by {
            let handle = self.member_ring@[
                ring_position::<C>(self.member_head, offset as nat)
            ];
            assert(before.member_entry_valid(offset));
            assert(handle.slot_spec() as int != slot_index) by {
                reveal(Scheduler::member_entry_valid);
            }
            assert(self.slots@[handle.slot_spec() as int]
                == before.slots@[handle.slot_spec() as int]);
            reveal(Scheduler::member_entry_valid);
        }
        assert forall|observed: int| 0 <= observed < C implies
            (((#[trigger] self.slots@[observed].state == RequestState::InFlight
                || self.slots@[observed].state == RequestState::Retiring)
                && self.completed < self.slots@[observed].active_epoch)
                == request_ring_contains_slot::<C>(
                    self.member_ring@,
                    self.member_head,
                    self.member_len,
                    observed,
                )) by {
            if observed != slot_index {
                assert(self.slots@[observed] == before.slots@[observed]);
            }
        }
    }

    proof fn finalized_slot_updates_preserve_batch_sum(
        &self,
        before: &Self,
        request: RequestId,
        epoch: u64,
    )
        requires
            before.basic_invariant(),
            self.finalized_slot_updates(before, request, epoch),
        ensures
            batch_member_sum::<C>(self.batch_ring@, self.batch_head, self.batch_len as nat)
                == self.member_len,
    {
        before.basic_implies_batch_ring();
        reveal(Scheduler::finalized_slot_updates);
        reveal(Scheduler::batch_ring_invariant);
    }

    proof fn finalized_slot_updates_preserve_batch_header(
        &self,
        before: &Self,
        request: RequestId,
        epoch: u64,
        batch_offset: int,
    )
        requires
            before.basic_invariant(),
            self.finalized_slot_updates(before, request, epoch),
            0 <= batch_offset < self.batch_len,
        ensures {
            let batch = self.batch_ring@[
                ring_position::<C>(self.batch_head, batch_offset as nat)
            ];
            &&& batch.member_count > 0
            &&& batch.epoch.value as int == self.completed as int + batch_offset + 1
            &&& batch.epoch.value <= self.submitted
        },
    {
        reveal(Scheduler::finalized_slot_updates);
        before.basic_batch_entry_header_facts(batch_offset);
    }

    proof fn finalized_slot_updates_preserve_member_epoch(
        &self,
        before: &Self,
        request: RequestId,
        epoch: u64,
        member_offset: int,
    )
        requires
            before.basic_invariant(),
            self.finalized_slot_updates(before, request, epoch),
            0 <= member_offset < self.member_len,
        ensures {
            let handle = self.member_ring@[
                ring_position::<C>(self.member_head, member_offset as nat)
            ];
            self.slots@[handle.slot_spec() as int].active_epoch
                == before.slots@[handle.slot_spec() as int].active_epoch
        },
    {
        reveal(Scheduler::finalized_slot_updates);
        let slot_index = request.slot_spec() as int;
        before.member_entry_facts(member_offset as usize);
        let handle = before.member_ring@[
            ring_position::<C>(before.member_head, member_offset as nat)
        ];
        assert(handle.slot_spec() as int != slot_index);
        assert(self.slots@[handle.slot_spec() as int]
            == before.slots@[handle.slot_spec() as int]);
    }

    proof fn finalized_slot_updates_preserve_batch_member(
        &self,
        before: &Self,
        request: RequestId,
        epoch: u64,
        batch_offset: int,
        member_offset: int,
    )
        requires
            before.basic_invariant(),
            self.finalized_slot_updates(before, request, epoch),
            0 <= batch_offset < self.batch_len,
            batch_member_sum::<C>(
                self.batch_ring@,
                self.batch_head,
                batch_offset as nat,
            ) <= member_offset < batch_member_sum::<C>(
                self.batch_ring@,
                self.batch_head,
                batch_offset as nat + 1,
            ),
        ensures {
            let batch = self.batch_ring@[
                ring_position::<C>(self.batch_head, batch_offset as nat)
            ];
            let handle = self.member_ring@[
                ring_position::<C>(self.member_head, member_offset as nat)
            ];
            self.slots@[handle.slot_spec() as int].active_epoch == batch.epoch.value
        },
    {
        reveal(Scheduler::finalized_slot_updates);
        before.basic_implies_batch_ring();
        reveal(Scheduler::batch_ring_invariant);
        batch_member_sum_monotonic::<C>(
            before.batch_ring@,
            before.batch_head,
            batch_offset as nat + 1,
            before.batch_len as nat,
        );
        self.finalized_slot_updates_preserve_member_epoch(
            before,
            request,
            epoch,
            member_offset,
        );
        before.basic_batch_member_epoch_fact(batch_offset, member_offset);
    }

    proof fn finalized_slot_updates_preserve_batch_entry(
        &self,
        before: &Self,
        request: RequestId,
        epoch: u64,
        batch_offset: int,
    )
        requires
            before.basic_invariant(),
            self.finalized_slot_updates(before, request, epoch),
            0 <= batch_offset < self.batch_len,
        ensures {
            let batch = self.batch_ring@[
                ring_position::<C>(self.batch_head, batch_offset as nat)
            ];
            &&& batch.member_count > 0
            &&& batch.epoch.value as int == self.completed as int + batch_offset + 1
            &&& batch.epoch.value <= self.submitted
            &&& (forall|member_offset: int|
                batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    batch_offset as nat,
                ) <= member_offset < batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    batch_offset as nat + 1,
                ) ==> {
                    let handle = #[trigger] self.member_ring@[
                        ring_position::<C>(self.member_head, member_offset as nat)
                    ];
                    self.slots@[handle.slot_spec() as int].active_epoch == batch.epoch.value
                })
        },
    {
        self.finalized_slot_updates_preserve_batch_header(
            before,
            request,
            epoch,
            batch_offset,
        );
        assert forall|member_offset: int|
            batch_member_sum::<C>(
                self.batch_ring@,
                self.batch_head,
                batch_offset as nat,
            ) <= member_offset < batch_member_sum::<C>(
                self.batch_ring@,
                self.batch_head,
                batch_offset as nat + 1,
            ) implies {
                let handle = #[trigger] self.member_ring@[
                    ring_position::<C>(self.member_head, member_offset as nat)
                ];
                self.slots@[handle.slot_spec() as int].active_epoch
                    == self.batch_ring@[
                        ring_position::<C>(self.batch_head, batch_offset as nat)
                    ].epoch.value
        } by {
            self.finalized_slot_updates_preserve_batch_member(
                before,
                request,
                epoch,
                batch_offset,
                member_offset,
            );
        }
    }

    proof fn finalized_slot_updates_preserve_batch_entries(
        &self,
        before: &Self,
        request: RequestId,
        epoch: u64,
    )
        requires
            before.basic_invariant(),
            self.finalized_slot_updates(before, request, epoch),
        ensures
            forall|batch_offset: int| 0 <= batch_offset < self.batch_len ==> {
                let batch = #[trigger] self.batch_ring@[
                    ring_position::<C>(self.batch_head, batch_offset as nat)
                ];
                &&& batch.member_count > 0
                &&& batch.epoch.value as int == self.completed as int + batch_offset + 1
                &&& batch.epoch.value <= self.submitted
                &&& (forall|member_offset: int|
                    batch_member_sum::<C>(
                        self.batch_ring@,
                        self.batch_head,
                        batch_offset as nat,
                    ) <= member_offset < batch_member_sum::<C>(
                        self.batch_ring@,
                        self.batch_head,
                        batch_offset as nat + 1,
                    ) ==> {
                        let handle = #[trigger] self.member_ring@[
                            ring_position::<C>(self.member_head, member_offset as nat)
                        ];
                        self.slots@[handle.slot_spec() as int].active_epoch == batch.epoch.value
                    })
            },
    {
        assert forall|batch_offset: int| 0 <= batch_offset < self.batch_len implies {
            let batch = #[trigger] self.batch_ring@[
                ring_position::<C>(self.batch_head, batch_offset as nat)
            ];
            &&& batch.member_count > 0
            &&& batch.epoch.value as int == self.completed as int + batch_offset + 1
            &&& batch.epoch.value <= self.submitted
            &&& (forall|member_offset: int|
                batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    batch_offset as nat,
                ) <= member_offset < batch_member_sum::<C>(
                    self.batch_ring@,
                    self.batch_head,
                    batch_offset as nat + 1,
                ) ==> {
                    let handle = #[trigger] self.member_ring@[
                        ring_position::<C>(self.member_head, member_offset as nat)
                    ];
                    self.slots@[handle.slot_spec() as int].active_epoch == batch.epoch.value
                })
        } by {
            self.finalized_slot_updates_preserve_batch_entry(
                before,
                request,
                epoch,
                batch_offset,
            );
        }
    }

    proof fn finalized_slot_updates_preserve_batch_ring(
        &self,
        before: &Self,
        request: RequestId,
        epoch: u64,
    )
        requires
            before.basic_invariant(),
            self.finalized_slot_updates(before, request, epoch),
        ensures self.batch_ring_invariant(),
    {
        self.finalized_slot_updates_preserve_batch_sum(before, request, epoch);
        self.finalized_slot_updates_preserve_batch_entries(before, request, epoch);
        reveal(Scheduler::batch_ring_invariant);
    }

    proof fn finalized_slot_updates_imply_refines(
        &self,
        before: &Self,
        request: RequestId,
        epoch: u64,
    )
        requires
            before.basic_invariant(),
            self.finalized_slot_updates(before, request, epoch),
        ensures self.finalized_slot_refines(before, request, epoch),
    {
        self.finalized_slot_updates_preserve_transition(before, request, epoch);
        self.finalized_slot_updates_preserve_frame(before, request, epoch);
        reveal(Scheduler::finalized_slot_updates);
        reveal(Scheduler::finalized_slot_refines);
    }

    proof fn finalized_slot_updates_preserve_transition(
        &self,
        before: &Self,
        request: RequestId,
        epoch: u64,
    )
        requires
            before.basic_invariant(),
            self.finalized_slot_updates(before, request, epoch),
        ensures {
            let slot_index = request.slot_spec() as int;
            ferric_spec::scheduling::request_transition(
                before.slot_model(slot_index),
                RequestTransition::FinalizeKv,
            ) == Ok(self.slot_model(slot_index))
        },
    {
        let slot_index = request.slot_spec() as int;
        before.basic_implies_slots();
        assert(before.slot_model(slot_index) == SequentialRequest {
            state: RequestState::InFlight,
            phase: LifecyclePhase::AwaitingKv,
        }) by {
            reveal(Scheduler::slot_invariant);
            reveal(Scheduler::finalized_slot_updates);
            reveal(Scheduler::slot_model);
        }
        assert(self.slot_model(slot_index) == SequentialRequest {
            state: RequestState::Ready,
            phase: LifecyclePhase::Idle,
        }) by {
            reveal(Scheduler::finalized_slot_updates);
            reveal(Scheduler::slot_model);
        }
        reveal(ferric_spec::scheduling::request_transition);
    }

    proof fn finalized_slot_updates_preserve_frame(
        &self,
        before: &Self,
        request: RequestId,
        epoch: u64,
    )
        requires
            before.basic_invariant(),
            self.finalized_slot_updates(before, request, epoch),
        ensures self.slots_frame_except(before, request.slot_spec() as int),
    {
        reveal(Scheduler::finalized_slot_updates);
        reveal(Scheduler::slots_frame_except);
        let slot_index = request.slot_spec() as int;
        assert forall|observed: int| 0 <= observed < C && observed != slot_index implies
            #[trigger] self.slots@[observed] == before.slots@[observed] by {}
    }

    proof fn finalized_slot_updates_establish_postconditions(
        &self,
        before: &Self,
        request: RequestId,
        epoch: u64,
    )
        requires
            before.basic_invariant(),
            self.finalized_slot_updates(before, request, epoch),
        ensures
            self.basic_invariant(),
            self.finalized_slot_refines(before, request, epoch),
    {
        self.finalized_slot_updates_preserve_scalar(before, request, epoch);
        self.finalized_slot_updates_preserve_slots(before, request, epoch);
        self.finalized_slot_updates_preserve_free_ring(before, request, epoch);
        self.finalized_slot_updates_preserve_reclaim_ring(before, request, epoch);
        self.finalized_slot_updates_preserve_member_ring(before, request, epoch);
        self.finalized_slot_updates_preserve_batch_ring(before, request, epoch);
        self.finalized_slot_updates_imply_refines(before, request, epoch);
        reveal(Scheduler::basic_invariant);
    }

    fn finalize_slot(&mut self, request: RequestId, epoch: u64)
        requires
            old(self).basic_invariant(),
            request.slot_spec() < C,
            old(self).slots@[request.slot_spec() as int].generation
                == request.generation_spec(),
            old(self).slots@[request.slot_spec() as int].state == RequestState::InFlight,
            old(self).slots@[request.slot_spec() as int].active_epoch == epoch,
            NO_EPOCH < epoch <= old(self).completed,
        ensures
            final(self).basic_invariant(),
            final(self).finalized_slot_refines(old(self), request, epoch),
    {
        let slot_index = request.slot() as usize;
        let ghost old_slots = self.slots@;
        self.slots[slot_index].state = RequestState::Ready;
        self.slots[slot_index].active_epoch = NO_EPOCH;
        self.slots[slot_index].last_quiescent_epoch = epoch;
        assert(self.slots@ == old_slots.update(slot_index as int, self.slots@[slot_index as int]));
        proof {
            assert(self.finalized_slot_updates(old(self), request, epoch)) by {
                reveal(Scheduler::finalized_slot_updates);
            }
            self.finalized_slot_updates_establish_postconditions(old(self), request, epoch);
        }
    }

    proof fn finalized_error_establishes_postconditions(
        &self,
        before: &Self,
        finalized: &KvFinalizedRequest,
        error: SchedulerError,
    )
        requires
            before.basic_invariant(),
            self.basic_invariant(),
            self.same_scalars(before),
            finalized_preflight_spec::<C>(before, finalized) == Err(error),
        ensures
            self.basic_invariant(),
            self.finalized_refines(before, finalized, &Err(error)),
            self.identity_frame(before),
            self.completed_epoch_spec() == before.completed_epoch_spec(),
            self.detachment_ready_frame_except(
                before,
                finalized.request_spec().slot_spec() as int,
            ),
    {
        let changed = finalized.request_spec().slot_spec() as int;
        assert(self.slots_frame_except(before, changed)) by {
            reveal(Scheduler::same_scalars);
            reveal(Scheduler::slots_frame_except);
        }
        self.detachment_frame_from_slots_frame(before, changed);
        if 0 <= changed && changed < C {
            assert(self.slot_generation_spec(changed)
                == before.slot_generation_spec(changed)) by {
                reveal(Scheduler::same_scalars);
                reveal(Scheduler::slot_generation_spec);
            }
            assert(self.slot_is_live_spec(changed)
                == before.slot_is_live_spec(changed)) by {
                reveal(Scheduler::same_scalars);
                reveal(Scheduler::slot_is_live_spec);
            }
            self.identity_frame_from_slots_frame(before, changed);
        } else {
            assert(self.identity_frame(before)) by {
                reveal(Scheduler::same_scalars);
                reveal(Scheduler::slot_generation_spec);
                reveal(Scheduler::slot_is_live_spec);
            }
        }
        assert(self.finalized_refines(before, finalized, &Err(error))) by {
            reveal(Scheduler::finalized_refines);
        }
        reveal(Scheduler::completed_epoch_spec);
    }

    proof fn finalized_success_establishes_frames(
        &self,
        before: &Self,
        finalized: &KvFinalizedRequest,
        request: RequestId,
        epoch: u64,
    )
        requires
            before.basic_invariant(),
            finalized_preflight_spec::<C>(before, finalized) == Ok((request, epoch)),
            self.finalized_slot_refines(before, request, epoch),
        ensures
            self.identity_frame(before),
            self.completed_epoch_spec() == before.completed_epoch_spec(),
            self.detachment_ready_frame_except(
                before,
                finalized.request_spec().slot_spec() as int,
            ),
    {
        let changed = request.slot_spec() as int;
        assert(self.slots_frame_except(before, changed)) by {
            reveal(Scheduler::finalized_slot_refines);
        }
        self.detachment_frame_from_slots_frame(before, changed);
        assert(self.slot_generation_spec(changed)
            == before.slot_generation_spec(changed)) by {
            reveal(Scheduler::finalized_slot_refines);
            reveal(Scheduler::slot_generation_spec);
        }
        assert(self.slot_is_live_spec(changed) == before.slot_is_live_spec(changed)) by {
            reveal(finalized_preflight_spec);
            reveal(finalized_request_preflight_spec);
            reveal(Scheduler::finalized_slot_refines);
            reveal(Scheduler::slot_model);
            reveal(Scheduler::slot_is_live_spec);
            reveal(ferric_spec::scheduling::request_transition);
        }
        self.identity_frame_from_slots_frame(before, changed);
        reveal(finalized_preflight_spec);
        reveal(Scheduler::finalized_slot_refines);
        reveal(Scheduler::completed_epoch_spec);
    }

    proof fn finalized_success_establishes_result(
        &self,
        before: &Self,
        finalized: &KvFinalizedRequest,
        request: RequestId,
        epoch: u64,
    )
        requires
            before.basic_invariant(),
            finalized_preflight_spec::<C>(before, finalized) == Ok((request, epoch)),
            self.finalized_slot_refines(before, request, epoch),
        ensures
            self.finalized_refines(before, finalized, &Ok(())),
            self.state_spec(finalized.request_spec()) == Some(RequestState::Ready),
            self.slot_is_live_spec(finalized.request_spec().slot_spec() as int),
    {
        assert(self.finalized_refines(before, finalized, &Ok(()))) by {
            reveal(Scheduler::finalized_refines);
            reveal(Scheduler::slot_model);
            reveal(Scheduler::slots_frame_except);
        }
        reveal(finalized_preflight_spec);
        reveal(Scheduler::finalized_slot_refines);
        reveal(Scheduler::state_spec);
        reveal(Scheduler::slot_model);
        reveal(Scheduler::slot_is_live_spec);
        reveal(ferric_spec::scheduling::request_transition);
    }

    proof fn finalized_success_establishes_postconditions(
        &self,
        before: &Self,
        finalized: &KvFinalizedRequest,
        request: RequestId,
        epoch: u64,
    )
        requires
            before.basic_invariant(),
            self.basic_invariant(),
            finalized_preflight_spec::<C>(before, finalized) == Ok((request, epoch)),
            self.finalized_slot_refines(before, request, epoch),
        ensures
            self.basic_invariant(),
            self.finalized_refines(before, finalized, &Ok(())),
            self.identity_frame(before),
            self.completed_epoch_spec() == before.completed_epoch_spec(),
            self.detachment_ready_frame_except(
                before,
                finalized.request_spec().slot_spec() as int,
            ),
            self.state_spec(finalized.request_spec()) == Some(RequestState::Ready),
            self.slot_is_live_spec(finalized.request_spec().slot_spec() as int),
    {
        self.finalized_success_establishes_frames(before, finalized, request, epoch);
        self.finalized_success_establishes_result(before, finalized, request, epoch);
    }

    /// Consumes cache-owned evidence for the exact completed speculative step
    /// before making the request dispatchable again.
    pub(crate) fn accept_finalized(
        &mut self,
        finalized: KvFinalizedRequest,
    ) -> (result: Result<(), SchedulerError>)
        requires old(self).basic_invariant(),
        ensures
            final(self).basic_invariant(),
            final(self).finalized_refines(old(self), &finalized, &result),
            final(self).identity_frame(old(self)),
            final(self).completed_epoch_spec() == old(self).completed_epoch_spec(),
            final(self).detachment_ready_frame_except(
                old(self),
                finalized.request_spec().slot_spec() as int,
            ),
            result.is_ok() ==> final(self).state_spec(finalized.request_spec())
                == Some(RequestState::Ready),
            result.is_ok() ==> final(self).slot_is_live_spec(
                finalized.request_spec().slot_spec() as int,
            ),
    {
        let (request, epoch) = match self.finalized_preflight(&finalized) {
            Ok(validated) => validated,
            Err(error) => {
                let result = Err(error);
                assert(self.same_scalars(old(self))) by {
                    reveal(Scheduler::same_scalars);
                }
                proof {
                    self.finalized_error_establishes_postconditions(
                        old(self),
                        &finalized,
                        error,
                    );
                }
                return result;
            }
        };
        self.finalize_slot(request, epoch);
        let result = Ok(());
        proof {
            self.finalized_success_establishes_postconditions(
                old(self),
                &finalized,
                request,
                epoch,
            );
        }
        result
    }

    fn reclaim_next_generation(
        &self,
        slot_index: usize,
    ) -> (result: Result<u32, SchedulerError>)
        requires
            self.basic_invariant(),
            slot_index < C,
        ensures
            match result {
                Err(error) => {
                    &&& error == SchedulerError::GenerationExhausted
                    &&& self.slots@[slot_index as int].generation == u32::MAX
                }
                Ok(next_generation) => {
                    &&& self.slots@[slot_index as int].generation != u32::MAX
                    &&& next_generation as int
                        == self.slots@[slot_index as int].generation as int + 1
                }
            },
    {
        let generation = self.slots[slot_index].generation;
        if generation == u32::MAX {
            return Err(SchedulerError::GenerationExhausted);
        }
        proof {
            u32_increment_is_exact(generation);
        }
        Ok(generation + 1)
    }

    fn detached_preflight(
        &self,
        detached: &KvDetachedRequest,
    ) -> (result: Result<(RequestId, u32), SchedulerError>)
        requires self.basic_invariant(),
        ensures detached_preflight_refines::<C>(self, detached, &result),
    {
        let request = detached.request();
        let slot_index = request.slot() as usize;
        if slot_index >= C {
            let result = Err(SchedulerError::InvalidSlot);
            assert(detached_preflight_refines::<C>(self, detached, &result)) by {
                reveal(detached_preflight_refines);
                reveal(detached_expected_error);
            }
            return result;
        }
        let slot = self.slots[slot_index];
        let origin = detached.origin();
        assert(request == detached.request_spec());
        assert(origin == detached.origin_spec());
        assert(slot_index as int == request.slot_spec() as int);
        assert(self.slots@[slot_index as int] == slot);
        let origin_matches = match origin {
            KvQuiescenceOrigin::NeverSubmitted => {
                slot.active_epoch == NO_EPOCH && slot.last_quiescent_epoch == NO_EPOCH
            }
            KvQuiescenceOrigin::CompletedExact { epoch } => {
                (slot.active_epoch != NO_EPOCH
                    && slot.active_epoch == epoch
                    && epoch <= self.completed)
                    || (slot.active_epoch == NO_EPOCH && slot.last_quiescent_epoch == epoch)
            }
        };
        assert(origin_matches == match detached.origin_spec() {
            KvQuiescenceOrigin::NeverSubmitted => {
                self.slots@[slot_index as int].active_epoch == NO_EPOCH
                    && self.slots@[slot_index as int].last_quiescent_epoch == NO_EPOCH
            }
            KvQuiescenceOrigin::CompletedExact { epoch } => {
                (self.slots@[slot_index as int].active_epoch != NO_EPOCH
                    && self.slots@[slot_index as int].active_epoch == epoch
                    && epoch <= self.completed)
                    || (self.slots@[slot_index as int].active_epoch == NO_EPOCH
                        && self.slots@[slot_index as int].last_quiescent_epoch == epoch)
            }
        });
        let request_generation = request.generation();
        if slot.generation != request_generation {
            let result = Err(SchedulerError::DetachmentMismatch);
            assert(detached_preflight_refines::<C>(self, detached, &result)) by {
                reveal(detached_preflight_refines);
                reveal(detached_expected_error);
            }
            return result;
        }
        match slot.state {
            RequestState::Retiring => {}
            RequestState::Vacant | RequestState::Ready | RequestState::InFlight => {
                let result = Err(SchedulerError::DetachmentMismatch);
                assert(detached_preflight_refines::<C>(self, detached, &result)) by {
                    reveal(detached_preflight_refines);
                    reveal(detached_expected_error);
                }
                return result;
            }
        }
        if slot.in_reclaim_ring || !origin_matches {
            let result = Err(SchedulerError::DetachmentMismatch);
            assert(detached_preflight_refines::<C>(self, detached, &result)) by {
                reveal(detached_preflight_refines);
                reveal(detached_expected_error);
            }
            return result;
        }
        let next_generation = match self.reclaim_next_generation(slot_index) {
            Ok(next_generation) => next_generation,
            Err(error) => {
                let result = Err(error);
                assert(detached_preflight_refines::<C>(self, detached, &result)) by {
                    reveal(detached_preflight_refines);
                    reveal(detached_expected_error);
                    reveal(Scheduler::slot_generation_spec);
                }
                return result;
            }
        };
        if self.free_len == C {
            let result = Err(SchedulerError::InvariantViolation);
            assert(detached_preflight_refines::<C>(self, detached, &result)) by {
                reveal(detached_preflight_refines);
                reveal(detached_expected_error);
            }
            return result;
        }
        let result = Ok((request, next_generation));
        assert(detached_preflight_refines::<C>(self, detached, &result)) by {
            reveal(detached_preflight_refines);
            reveal(detached_expected_error);
            reveal(Scheduler::slot_generation_spec);
        }
        result
    }

    proof fn reclaim_slot_updates_establish_postconditions(
        &self,
        before: &Self,
        detached: &KvDetachedRequest,
        request: RequestId,
        next_generation: u32,
    )
        requires
            before.basic_invariant(),
            detached_expected_error::<C>(before, detached).is_none(),
            request == detached.request_spec(),
            next_generation as int
                == before.slot_generation_spec(request.slot_spec() as int) as int + 1,
            self.reclaim_slot_updates(before, request, next_generation),
        ensures
            self.basic_invariant(),
            self.detached_refines(before, detached, &Ok(request)),
            self.state_spec(request).is_none(),
            self.detachment_ready_frame_except(before, request.slot_spec() as int),
    {
        reveal(Scheduler::reclaim_slot_updates);
        let slot_index = request.slot_spec() as int;
        live_count_update_reclaim(
            before.slots@,
            slot_index,
            self.slots@[slot_index],
            C as nat,
        );
        nonreclaim_count_update_remove(
            before.slots@,
            slot_index,
            self.slots@[slot_index],
            C as nat,
        );
        before.basic_implies_scalar();
        assert(before.free_len < C) by {
            reveal(detached_expected_error);
        }
        assert(before.slots@[slot_index].state != RequestState::Vacant) by {
            reveal(detached_expected_error);
        }
        assert(!((before.slots@[slot_index].state == RequestState::InFlight
            || before.slots@[slot_index].state == RequestState::Retiring)
            && before.completed < before.slots@[slot_index].active_epoch)) by {
            reveal(detached_expected_error);
        }
        before.nonexecuting_live_slot_gives_member_slack(slot_index);
        assert(self.member_len <= self.live_count);
        assert(self.scalar_invariant()) by {
            reveal(Scheduler::scalar_invariant);
        }
        assert(self.detached_refines(before, detached, &Ok(request))) by {
            reveal(Scheduler::basic_invariant);
            reveal(Scheduler::scalar_invariant);
            reveal(Scheduler::detached_refines);
            reveal(Scheduler::slot_model);
            reveal(Scheduler::slots_frame_except);
            reveal(detached_expected_error);
        }
        self.detached_refines_preserves_basic(before, detached, request);
        self.detachment_frame_from_slots_frame(before, slot_index);
        assert(self.basic_invariant());
        assert(self.detached_refines(before, detached, &Ok(request)));
        assert(self.state_spec(request).is_none()) by {
            reveal(Scheduler::state_spec);
        }
        assert(self.detachment_ready_frame_except(before, slot_index));
    }

    fn reclaim_slot(
        &mut self,
        _detached: &KvDetachedRequest,
        request: RequestId,
        next_generation: u32,
    ) -> (reclaimed: RequestId)
        requires
            old(self).basic_invariant(),
            detached_expected_error::<C>(old(self), _detached).is_none(),
            request == _detached.request_spec(),
            next_generation as int
                == old(self).slot_generation_spec(request.slot_spec() as int) as int + 1,
        ensures
            final(self).basic_invariant(),
            final(self).detached_refines(old(self), _detached, &Ok(reclaimed)),
            reclaimed == request,
            final(self).state_spec(reclaimed).is_none(),
            final(self).detachment_ready_frame_except(
                old(self),
                request.slot_spec() as int,
            ),
    {
        let slot_index = request.slot() as usize;
        let ghost old_slots = self.slots@;
        let free_tail = ring_tail::<C>(self.free_head, self.free_len);
        self.free_ring[free_tail] = slot_index;
        self.free_len += 1;
        self.slots[slot_index] = Slot {
            generation: next_generation,
            state: RequestState::Vacant,
            active_epoch: NO_EPOCH,
            last_quiescent_epoch: NO_EPOCH,
            in_free_ring: true,
            in_reclaim_ring: false,
        };
        self.live_count -= 1;
        assert(self.slots@ == old_slots.update(slot_index as int, self.slots@[slot_index as int]));
        assert(self.reclaim_slot_updates(old(self), request, next_generation)) by {
            reveal(Scheduler::reclaim_slot_updates);
        }
        proof {
            self.reclaim_slot_updates_establish_postconditions(
                old(self),
                _detached,
                request,
                next_generation,
            );
        }
        request
    }

    /// Returns a terminal slot to the free ring only after exact cache-owned
    /// detachment evidence is consumed.
    pub(crate) fn reclaim_detached(
        &mut self,
        detached: KvDetachedRequest,
    ) -> (result: Result<RequestId, SchedulerError>)
        requires old(self).basic_invariant(),
        ensures
            final(self).basic_invariant(),
            final(self).detached_refines(old(self), &detached, &result),
            old(self).detached_enabled(&detached) ==> result.is_ok(),
            final(self).completed_epoch_spec() == old(self).completed_epoch_spec(),
            final(self).detachment_ready_frame_except(
                old(self),
                detached.request_spec().slot_spec() as int,
            ),
            match result {
                Err(_) => final(self).identity_frame(old(self)),
                Ok(request) => {
                    let slot_index = request.slot_spec() as int;
                    &&& request == detached.request_spec()
                    &&& final(self).state_spec(request).is_none()
                    &&& !final(self).slot_is_live_spec(slot_index)
                    &&& final(self).slot_generation_spec(slot_index)
                        == old(self).slot_generation_spec(slot_index) + 1
                    &&& (forall|other: int| 0 <= other < C && other != slot_index ==> {
                        &&& final(self).slot_is_live_spec(other)
                            == old(self).slot_is_live_spec(other)
                        &&& final(self).slot_generation_spec(other)
                            == old(self).slot_generation_spec(other)
                    })
                }
            },
    {
        reveal(Scheduler::same_scalars);
        let (request, next_generation) = match self.detached_preflight(&detached) {
            Ok(validated) => validated,
            Err(error) => {
                let result = Err(error);
                assert(self.same_scalars(old(self))) by {
                    reveal(Scheduler::same_scalars);
                }
                proof {
                    let changed = detached.request_spec().slot_spec() as int;
                    assert(self.slots_frame_except(old(self), changed)) by {
                        reveal(Scheduler::same_scalars);
                        reveal(Scheduler::slots_frame_except);
                    }
                    self.detachment_frame_from_slots_frame(old(self), changed);
                }
                return result;
            }
        };
        Ok(self.reclaim_slot(&detached, request, next_generation))
    }

    #[must_use]
    pub fn state(&self, request: RequestId) -> (state: Option<RequestState>)
        ensures state == self.state_spec(request),
    {
        reveal(Scheduler::state_spec);
        let slot_number = request.slot();
        let slot_index = slot_number as usize;
        if slot_index >= C {
            return None;
        }
        let slot = self.slots[slot_index];
        let generation = request.generation();
        if slot.generation == generation {
            match slot.state {
                RequestState::Vacant => None,
                RequestState::Ready | RequestState::InFlight | RequestState::Retiring => {
                    Some(slot.state)
                }
            }
        } else {
            None
        }
    }

    pub closed spec fn state_spec(&self, request: RequestId) -> Option<RequestState> {
        if request.slot_spec() >= C {
            None
        } else {
            let slot = self.slots@[request.slot_spec() as int];
            if slot.generation == request.generation_spec() && slot.state != RequestState::Vacant {
                Some(slot.state)
            } else {
                None
            }
        }
    }
}

fn advance<const C: usize>(index: usize) -> (next: usize)
    requires C > 0, index < C,
    ensures next < C, next == next_position::<C>(index),
{
    if index + 1 == C {
        0
    } else {
        index + 1
    }
}

fn ring_tail<const C: usize>(head: usize, len: usize) -> (tail: usize)
    requires C > 0, head < C, len < C,
    ensures tail < C, tail as int == ring_position::<C>(head, len as nat),
{
    let distance = C - head;
    if len < distance {
        head + len
    } else {
        len - distance
    }
}

spec fn retire_expected_error<const C: usize>(
    scheduler: &Scheduler<C>,
    request: RequestId,
) -> Option<SchedulerError> {
    let slot_index = request.slot_spec() as int;
    if slot_index >= C {
        Some(SchedulerError::InvalidSlot)
    } else if scheduler.slots@[slot_index].generation != request.generation_spec() {
        Some(SchedulerError::StaleRequest)
    } else {
        match scheduler.slots@[slot_index].state {
            RequestState::Vacant => Some(SchedulerError::RequestNotLive),
            RequestState::Retiring => Some(SchedulerError::AlreadyRetiring),
            RequestState::Ready | RequestState::InFlight => None,
        }
    }
}

spec fn completion_expected_error<const C: usize>(
    scheduler: &Scheduler<C>,
    completion_epoch: u64,
    permits: Seq<Option<KvQuiescencePermit>>,
) -> Option<SchedulerError>
    recommends scheduler.basic_invariant(),
{
    if scheduler.batch_len == 0 {
        Some(SchedulerError::NoPendingBatch)
    } else if scheduler.completed == u64::MAX || completion_epoch != scheduler.completed + 1 {
        Some(SchedulerError::CompletionNotExactNext)
    } else {
        let batch = scheduler.batch_ring@[scheduler.batch_head as int];
        if batch.epoch.value != completion_epoch || batch.member_count == 0 {
            Some(SchedulerError::CompletionEpochMismatch)
        } else if permits.len() < batch.member_count {
            Some(SchedulerError::CompletionStorageTooSmall)
        } else if !option_prefix_empty(permits, batch.member_count as nat) {
            Some(SchedulerError::CompletionStorageNotEmpty)
        } else {
            None
        }
    }
}

spec fn finalized_expected_error<const C: usize>(
    scheduler: &Scheduler<C>,
    finalized: &KvFinalizedRequest,
) -> Option<SchedulerError> {
    match finalized_preflight_spec::<C>(scheduler, finalized) {
        Ok(_) => None,
        Err(error) => Some(error),
    }
}

spec fn finalized_preflight_spec<const C: usize>(
    scheduler: &Scheduler<C>,
    finalized: &KvFinalizedRequest,
) -> Result<(RequestId, u64), SchedulerError> {
    let request = finalized.request_spec();
    match finalized_origin_spec(finalized) {
        Err(error) => Err(error),
        Ok(epoch) => match finalized_request_preflight_spec::<C>(scheduler, request, epoch) {
            Err(error) => Err(error),
            Ok(()) => Ok((request, epoch)),
        }
    }
}

spec fn finalized_origin_spec(
    finalized: &KvFinalizedRequest,
) -> Result<u64, SchedulerError> {
    match finalized.origin_spec() {
        KvQuiescenceOrigin::CompletedExact { epoch } => Ok(epoch),
        KvQuiescenceOrigin::NeverSubmitted => Err(SchedulerError::FinalizationMismatch),
    }
}

spec fn finalized_request_preflight_spec<const C: usize>(
    scheduler: &Scheduler<C>,
    request: RequestId,
    epoch: u64,
) -> Result<(), SchedulerError> {
    let slot_index = request.slot_spec() as int;
    if slot_index >= C {
        Err(SchedulerError::InvalidSlot)
    } else {
        let slot = scheduler.slots@[slot_index];
        if slot.generation == request.generation_spec()
            && slot.state == RequestState::InFlight
            && slot.active_epoch == epoch
            && NO_EPOCH < epoch <= scheduler.completed
        {
            Ok(())
        } else {
            Err(SchedulerError::FinalizationMismatch)
        }
    }
}

spec fn detached_expected_error<const C: usize>(
    scheduler: &Scheduler<C>,
    detached: &KvDetachedRequest,
) -> Option<SchedulerError> {
    let request = detached.request_spec();
    let slot_index = request.slot_spec() as int;
    if slot_index >= C {
        Some(SchedulerError::InvalidSlot)
    } else {
        let slot = scheduler.slots@[slot_index];
        let origin_matches = match detached.origin_spec() {
            KvQuiescenceOrigin::NeverSubmitted => {
                slot.active_epoch == NO_EPOCH && slot.last_quiescent_epoch == NO_EPOCH
            }
            KvQuiescenceOrigin::CompletedExact { epoch } => {
                (slot.active_epoch != NO_EPOCH
                    && slot.active_epoch == epoch
                    && epoch <= scheduler.completed)
                    || (slot.active_epoch == NO_EPOCH && slot.last_quiescent_epoch == epoch)
            }
        };
        if slot.generation != request.generation_spec()
            || slot.state != RequestState::Retiring
            || slot.in_reclaim_ring
            || !origin_matches
        {
            Some(SchedulerError::DetachmentMismatch)
        } else if slot.generation == u32::MAX {
            Some(SchedulerError::GenerationExhausted)
        } else if scheduler.free_len == C {
            Some(SchedulerError::InvariantViolation)
        } else {
            None
        }
    }
}

closed spec fn detached_preflight_refines<const C: usize>(
    scheduler: &Scheduler<C>,
    detached: &KvDetachedRequest,
    result: &Result<(RequestId, u32), SchedulerError>,
) -> bool {
    match result {
        Err(error) => Some(*error) == detached_expected_error::<C>(scheduler, detached),
        Ok((request, next_generation)) => {
            let detached_request = detached.request_spec();
            let slot_index = detached_request.slot_spec() as int;
            &&& detached_expected_error::<C>(scheduler, detached).is_none()
            &&& *request == detached_request
            &&& *next_generation as int
                == scheduler.slot_generation_spec(slot_index) as int + 1
        }
    }
}

spec fn option_prefix_empty(values: Seq<Option<KvQuiescencePermit>>, prefix: nat) -> bool {
    forall|index: int| 0 <= index < prefix ==> (#[trigger] values[index]).is_none()
}

spec fn next_position<const C: usize>(head: usize) -> usize
    recommends C > 0, head < C,
{
    if head + 1 == C {
        0
    } else {
        (head + 1) as usize
    }
}

spec fn ring_position<const C: usize>(head: usize, offset: nat) -> int
    recommends C > 0, head < C, offset < C,
{
    if offset < C - head {
        (head + offset) as int
    } else {
        (offset - (C - head)) as int
    }
}

#[verifier::bit_vector]
proof fn u32_increment_is_exact(value: u32)
    requires value != u32::MAX,
    ensures (value + 1) as int == value as int + 1,
{
}

proof fn ring_position_bounds<const C: usize>(head: usize, offset: nat)
    requires C > 0, head < C, offset < C,
    ensures 0 <= ring_position::<C>(head, offset) < C,
{
}

proof fn ring_position_injective<const C: usize>(head: usize, left: nat, right: nat)
    requires
        C > 0,
        head < C,
        left < C,
        right < C,
    ensures
        ring_position::<C>(head, left) == ring_position::<C>(head, right) ==> left == right,
{
}

proof fn ring_position_next<const C: usize>(head: usize, offset: nat)
    requires
        C > 0,
        head < C,
        offset + 1 < C,
    ensures
        next_position::<C>(ring_position::<C>(head, offset) as usize)
            == ring_position::<C>(head, offset + 1),
{
    ring_position_bounds::<C>(head, offset);
}

spec fn ring_position_or_head<const C: usize>(head: usize, offset: nat) -> int
    recommends C > 0, head < C, offset <= C,
{
    if offset == C {
        head as int
    } else {
        ring_position::<C>(head, offset)
    }
}

proof fn ring_position_or_head_next<const C: usize>(head: usize, offset: nat)
    requires
        C > 0,
        head < C,
        offset < C,
    ensures
        next_position::<C>(ring_position_or_head::<C>(head, offset) as usize) as int
            == ring_position_or_head::<C>(head, offset + 1),
{
    reveal(ring_position_or_head);
    if offset + 1 < C {
        ring_position_next::<C>(head, offset);
    }
}

spec fn ready_selection<const C: usize>(
    slots: Seq<Slot>,
    cursor: usize,
    scans_left: nat,
    take_left: nat,
) -> Seq<int>
    recommends
        C > 0,
        slots.len() == C,
        cursor < C,
        scans_left <= C,
    decreases scans_left,
{
    if scans_left == 0 || take_left == 0 {
        Seq::empty()
    } else {
        let next = next_position::<C>(cursor);
        if slots[cursor as int].state == RequestState::Ready {
            seq![cursor as int].add(ready_selection::<C>(
                slots,
                next,
                (scans_left - 1) as nat,
                (take_left - 1) as nat,
            ))
        } else {
            ready_selection::<C>(
                slots,
                next,
                (scans_left - 1) as nat,
                take_left,
            )
        }
    }
}

spec fn ready_scan_cursor<const C: usize>(
    slots: Seq<Slot>,
    cursor: usize,
    scans_left: nat,
    take_left: nat,
) -> usize
    recommends
        C > 0,
        slots.len() == C,
        cursor < C,
        scans_left <= C,
    decreases scans_left,
{
    if scans_left == 0 || take_left == 0 {
        cursor
    } else {
        let next = next_position::<C>(cursor);
        if slots[cursor as int].state == RequestState::Ready {
            ready_scan_cursor::<C>(
                slots,
                next,
                (scans_left - 1) as nat,
                (take_left - 1) as nat,
            )
        } else {
            ready_scan_cursor::<C>(slots, next, (scans_left - 1) as nat, take_left)
        }
    }
}

proof fn ready_scan_facts<const C: usize>(
    slots: Seq<Slot>,
    cursor: usize,
    scans_left: nat,
    take_left: nat,
)
    requires
        C > 0,
        slots.len() == C,
        cursor < C,
        scans_left <= C,
    ensures
        ready_selection::<C>(slots, cursor, scans_left, take_left).len() <= scans_left,
        ready_selection::<C>(slots, cursor, scans_left, take_left).len() <= take_left,
        ready_scan_cursor::<C>(slots, cursor, scans_left, take_left) < C,
        forall|offset: int|
            0 <= offset
                < ready_selection::<C>(slots, cursor, scans_left, take_left).len() ==>
                    0 <= #[trigger] ready_selection::<C>(
                        slots,
                        cursor,
                        scans_left,
                        take_left,
                    )[offset] < C,
    decreases scans_left,
{
    reveal(ready_selection);
    reveal(ready_scan_cursor);
    if scans_left > 0 && take_left > 0 {
        let next = next_position::<C>(cursor);
        if slots[cursor as int].state == RequestState::Ready {
            ready_scan_facts::<C>(
                slots,
                next,
                (scans_left - 1) as nat,
                (take_left - 1) as nat,
            );
            assert forall|offset: int|
                0 <= offset
                    < ready_selection::<C>(slots, cursor, scans_left, take_left).len()
                implies 0 <= #[trigger] ready_selection::<C>(
                    slots,
                    cursor,
                    scans_left,
                    take_left,
                )[offset] < C
            by {
                if offset > 0 {
                    assert(ready_selection::<C>(slots, cursor, scans_left, take_left)[offset]
                        == ready_selection::<C>(
                            slots,
                            next,
                            (scans_left - 1) as nat,
                            (take_left - 1) as nat,
                        )[offset - 1]);
                }
            }
        } else {
            ready_scan_facts::<C>(
                slots,
                next,
                (scans_left - 1) as nat,
                take_left,
            );
        }
    }
}

proof fn ready_selection_entry_ready<const C: usize>(
    slots: Seq<Slot>,
    cursor: usize,
    scans_left: nat,
    take_left: nat,
    offset: int,
)
    requires
        C > 0,
        slots.len() == C,
        cursor < C,
        scans_left <= C,
        0 <= offset < ready_selection::<C>(slots, cursor, scans_left, take_left).len(),
    ensures
        slots[ready_selection::<C>(slots, cursor, scans_left, take_left)[offset]].state
            == RequestState::Ready,
    decreases scans_left,
{
    reveal(ready_selection);
    if scans_left > 0 && take_left > 0 {
        let next = next_position::<C>(cursor);
        if slots[cursor as int].state == RequestState::Ready {
            if offset > 0 {
                ready_selection_entry_ready::<C>(
                    slots,
                    next,
                    (scans_left - 1) as nat,
                    (take_left - 1) as nat,
                    offset - 1,
                );
            }
        } else {
            ready_selection_entry_ready::<C>(
                slots,
                next,
                (scans_left - 1) as nat,
                take_left,
                offset,
            );
        }
    }
}

spec fn selected_request_slots(chosen: Seq<RequestId>) -> Seq<int> {
    Seq::new(chosen.len(), |offset: int| chosen[offset].slot_spec() as int)
}

proof fn selected_request_slots_empty()
    ensures selected_request_slots(Seq::empty()).len() == 0,
{
    reveal(selected_request_slots);
}

proof fn selected_request_slots_push(chosen: Seq<RequestId>, added: RequestId)
    ensures
        selected_request_slots(chosen.push(added))
            == selected_request_slots(chosen).push(added.slot_spec() as int),
{
    reveal(selected_request_slots);
    assert forall|offset: int| 0 <= offset < chosen.len() + 1 implies
        #[trigger] selected_request_slots(chosen.push(added))[offset]
            == selected_request_slots(chosen).push(added.slot_spec() as int)[offset]
    by {
        if offset < chosen.len() {
            assert(chosen.push(added)[offset] == chosen[offset]);
        } else {
            assert(offset == chosen.len());
            assert(chosen.push(added)[offset] == added);
        }
    }
}

proof fn selected_request_slots_subrange(chosen: Seq<RequestId>, end: int)
    requires 0 <= end <= chosen.len(),
    ensures
        selected_request_slots(chosen.subrange(0, end))
            == selected_request_slots(chosen).subrange(0, end),
{
    reveal(selected_request_slots);
    assert forall|offset: int| 0 <= offset < end implies
        #[trigger] selected_request_slots(chosen.subrange(0, end))[offset]
            == selected_request_slots(chosen).subrange(0, end)[offset] by {
        assert(chosen.subrange(0, end)[offset] == chosen[offset]);
    }
}

spec fn dispatch_selected_slots(
    slots: Seq<Slot>,
    chosen: Seq<RequestId>,
    epoch: u64,
) -> Seq<Slot>
    recommends
        forall|offset: int| 0 <= offset < chosen.len() ==>
            chosen[offset].slot_spec() < slots.len(),
    decreases chosen.len(),
{
    if chosen.len() == 0 {
        slots
    } else {
        let prefix = chosen.subrange(0, chosen.len() - 1);
        let slot_index = chosen[chosen.len() - 1].slot_spec() as int;
        let prior = dispatch_selected_slots(slots, prefix, epoch);
        prior.update(slot_index, Slot {
            state: RequestState::InFlight,
            active_epoch: epoch,
            ..slots[slot_index]
        })
    }
}

proof fn dispatch_selected_slots_empty(slots: Seq<Slot>, epoch: u64)
    ensures dispatch_selected_slots(slots, Seq::empty(), epoch) == slots,
{
    reveal(dispatch_selected_slots);
}

proof fn dispatch_selected_slots_push(
    slots: Seq<Slot>,
    chosen: Seq<RequestId>,
    epoch: u64,
    added: RequestId,
)
    requires
        forall|offset: int| 0 <= offset < chosen.len() ==>
            chosen[offset].slot_spec() < slots.len(),
        added.slot_spec() < slots.len(),
    ensures
        dispatch_selected_slots(slots, chosen.push(added), epoch)
            == dispatch_selected_slots(slots, chosen, epoch).update(
                added.slot_spec() as int,
                Slot {
                    state: RequestState::InFlight,
                    active_epoch: epoch,
                    ..slots[added.slot_spec() as int]
                },
            ),
{
    reveal(dispatch_selected_slots);
    assert(chosen.push(added).subrange(0, chosen.len() as int) == chosen);
}

proof fn dispatch_selected_slots_len(
    slots: Seq<Slot>,
    chosen: Seq<RequestId>,
    epoch: u64,
)
    requires
        forall|offset: int| 0 <= offset < chosen.len() ==>
            chosen[offset].slot_spec() < slots.len(),
    ensures dispatch_selected_slots(slots, chosen, epoch).len() == slots.len(),
    decreases chosen.len(),
{
    reveal(dispatch_selected_slots);
    if chosen.len() > 0 {
        let prefix = chosen.subrange(0, chosen.len() - 1);
        assert forall|offset: int| 0 <= offset < prefix.len() implies
            prefix[offset].slot_spec() < slots.len() by {
            assert(prefix[offset] == chosen[offset]);
        }
        dispatch_selected_slots_len(slots, prefix, epoch);
    }
}

spec fn dispatch_selected_output(
    output: Seq<RequestId>,
    chosen: Seq<RequestId>,
) -> Seq<RequestId>
    recommends
        chosen.len() <= output.len(),
    decreases chosen.len(),
{
    if chosen.len() == 0 {
        output
    } else {
        let prefix = chosen.subrange(0, chosen.len() - 1);
        dispatch_selected_output(output, prefix).update(
            chosen.len() - 1,
            chosen[chosen.len() - 1],
        )
    }
}

proof fn dispatch_selected_output_empty(output: Seq<RequestId>)
    ensures dispatch_selected_output(output, Seq::empty()) == output,
{
    reveal(dispatch_selected_output);
}

proof fn dispatch_selected_output_push(
    output: Seq<RequestId>,
    chosen: Seq<RequestId>,
    added: RequestId,
)
    requires chosen.len() < output.len(),
    ensures
        dispatch_selected_output(output, chosen.push(added))
            == dispatch_selected_output(output, chosen).update(chosen.len() as int, added),
{
    reveal(dispatch_selected_output);
    assert(chosen.push(added).subrange(0, chosen.len() as int) == chosen);
}

proof fn dispatch_selected_output_facts(
    output: Seq<RequestId>,
    chosen: Seq<RequestId>,
)
    requires chosen.len() <= output.len(),
    ensures
        dispatch_selected_output(output, chosen).len() == output.len(),
        forall|offset: int| 0 <= offset < chosen.len() ==>
            #[trigger] dispatch_selected_output(output, chosen)[offset] == chosen[offset],
        forall|offset: int| chosen.len() <= offset < output.len() ==>
            #[trigger] dispatch_selected_output(output, chosen)[offset] == output[offset],
    decreases chosen.len(),
{
    reveal(dispatch_selected_output);
    if chosen.len() > 0 {
        let prefix = chosen.subrange(0, chosen.len() - 1);
        dispatch_selected_output_facts(output, prefix);
        assert forall|offset: int| 0 <= offset < chosen.len() implies
            #[trigger] dispatch_selected_output(output, chosen)[offset] == chosen[offset] by {
            if offset < prefix.len() {
                assert(prefix[offset] == chosen[offset]);
            } else {
                assert(offset == chosen.len() - 1);
            }
        }
        assert forall|offset: int| chosen.len() <= offset < output.len() implies
            #[trigger] dispatch_selected_output(output, chosen)[offset] == output[offset] by {
            assert(offset != chosen.len() - 1);
        }
    }
}

spec fn dispatch_selected_members<const C: usize>(
    members: Seq<RequestId>,
    head: usize,
    base_len: usize,
    chosen: Seq<RequestId>,
) -> Seq<RequestId>
    recommends
        C > 0,
        members.len() == C,
        head < C,
        base_len + chosen.len() <= C,
        forall|offset: int| 0 <= offset < chosen.len() ==>
            chosen[offset].slot_spec() < C,
    decreases chosen.len(),
{
    if chosen.len() == 0 {
        members
    } else {
        let prefix = chosen.subrange(0, chosen.len() - 1);
        let selected_offset = chosen.len() - 1;
        dispatch_selected_members::<C>(members, head, base_len, prefix).update(
            ring_position::<C>(head, (base_len + selected_offset) as nat),
            chosen[selected_offset],
        )
    }
}

proof fn dispatch_selected_members_empty<const C: usize>(
    members: Seq<RequestId>,
    head: usize,
    base_len: usize,
)
    requires
        C > 0,
        members.len() == C,
        head < C,
        base_len <= C,
    ensures
        dispatch_selected_members::<C>(members, head, base_len, Seq::empty()) == members,
{
    reveal(dispatch_selected_members);
}

proof fn dispatch_selected_members_push<const C: usize>(
    members: Seq<RequestId>,
    head: usize,
    base_len: usize,
    chosen: Seq<RequestId>,
    added: RequestId,
)
    requires
        C > 0,
        members.len() == C,
        head < C,
        base_len + chosen.len() < C,
        forall|offset: int| 0 <= offset < chosen.len() ==>
            chosen[offset].slot_spec() < C,
        added.slot_spec() < C,
    ensures
        dispatch_selected_members::<C>(members, head, base_len, chosen.push(added))
            == dispatch_selected_members::<C>(members, head, base_len, chosen).update(
                ring_position::<C>(head, (base_len + chosen.len()) as nat),
                added,
            ),
{
    reveal(dispatch_selected_members);
    assert(chosen.push(added).subrange(0, chosen.len() as int) == chosen);
}

spec fn completed_permits<const C: usize>(
    before: Seq<Option<KvQuiescencePermit>>,
    members: Seq<RequestId>,
    head: usize,
    processed: nat,
    epoch: u64,
) -> Seq<Option<KvQuiescencePermit>>
    recommends
        C > 0,
        members.len() == C,
        head < C,
        processed <= C,
        processed <= before.len(),
    decreases processed,
{
    if processed == 0 {
        before
    } else {
        let prior = completed_permits::<C>(
            before,
            members,
            head,
            (processed - 1) as nat,
            epoch,
        );
        let request = members[ring_position::<C>(head, (processed - 1) as nat)];
        prior.update(
            processed - 1,
            Some(KvQuiescencePermit {
                request,
                origin: KvQuiescenceOrigin::CompletedExact { epoch },
            }),
        )
    }
}

proof fn completed_permits_empty<const C: usize>(
    before: Seq<Option<KvQuiescencePermit>>,
    members: Seq<RequestId>,
    head: usize,
    epoch: u64,
)
    requires
        C > 0,
        members.len() == C,
        head < C,
    ensures completed_permits::<C>(before, members, head, 0, epoch) == before,
{
    reveal(completed_permits);
}

proof fn completed_permits_push<const C: usize>(
    before: Seq<Option<KvQuiescencePermit>>,
    members: Seq<RequestId>,
    head: usize,
    processed: nat,
    epoch: u64,
)
    requires
        C > 0,
        members.len() == C,
        head < C,
        processed < C,
        processed < before.len(),
    ensures
        completed_permits::<C>(before, members, head, processed + 1, epoch)
            == completed_permits::<C>(before, members, head, processed, epoch).update(
                processed as int,
                Some(KvQuiescencePermit {
                    request: members[ring_position::<C>(head, processed)],
                    origin: KvQuiescenceOrigin::CompletedExact { epoch },
                }),
            ),
{
    reveal(completed_permits);
}

proof fn completed_permits_facts<const C: usize>(
    before: Seq<Option<KvQuiescencePermit>>,
    members: Seq<RequestId>,
    head: usize,
    processed: nat,
    epoch: u64,
)
    requires
        C > 0,
        members.len() == C,
        head < C,
        processed <= C,
        processed <= before.len(),
    ensures
        completed_permits::<C>(before, members, head, processed, epoch).len()
            == before.len(),
        forall|offset: int| 0 <= offset < processed ==> {
            let request = members[ring_position::<C>(head, offset as nat)];
            match #[trigger] completed_permits::<C>(
                before,
                members,
                head,
                processed,
                epoch,
            )[offset] {
                Some(permit) => {
                    &&& permit.request_spec() == request
                    &&& permit.origin_spec()
                        == KvQuiescenceOrigin::CompletedExact { epoch }
                }
                None => false,
            }
        },
        forall|offset: int| processed <= offset < before.len() ==>
            #[trigger] completed_permits::<C>(
                before,
                members,
                head,
                processed,
                epoch,
            )[offset] == before[offset],
    decreases processed,
{
    if processed == 0 {
        completed_permits_empty::<C>(before, members, head, epoch);
    } else {
        let previous = (processed - 1) as nat;
        completed_permits_facts::<C>(before, members, head, previous, epoch);
        completed_permits_push::<C>(before, members, head, previous, epoch);
        assert forall|offset: int| 0 <= offset < processed implies {
            let request = members[ring_position::<C>(head, offset as nat)];
            match #[trigger] completed_permits::<C>(
                before,
                members,
                head,
                processed,
                epoch,
            )[offset] {
                Some(permit) => {
                    &&& permit.request_spec() == request
                    &&& permit.origin_spec()
                        == KvQuiescenceOrigin::CompletedExact { epoch }
                }
                None => false,
            }
        } by {
            if offset < previous {
                assert(completed_permits::<C>(before, members, head, processed, epoch)[offset]
                    == completed_permits::<C>(before, members, head, previous, epoch)[offset]);
            } else {
                assert(offset == previous);
            }
        }
        assert forall|offset: int| processed <= offset < before.len() implies
            #[trigger] completed_permits::<C>(
                before,
                members,
                head,
                processed,
                epoch,
            )[offset] == before[offset]
        by {
            assert(offset != previous);
            assert(completed_permits::<C>(before, members, head, processed, epoch)[offset]
                == completed_permits::<C>(before, members, head, previous, epoch)[offset]);
        }
    }
}

proof fn dispatch_selected_members_len<const C: usize>(
    members: Seq<RequestId>,
    head: usize,
    base_len: usize,
    chosen: Seq<RequestId>,
)
    requires
        C > 0,
        members.len() == C,
        head < C,
        base_len + chosen.len() <= C,
        forall|offset: int| 0 <= offset < chosen.len() ==>
            chosen[offset].slot_spec() < C,
    ensures
        dispatch_selected_members::<C>(members, head, base_len, chosen).len()
            == members.len(),
    decreases chosen.len(),
{
    reveal(dispatch_selected_members);
    if chosen.len() > 0 {
        let prefix = chosen.subrange(0, chosen.len() - 1);
        assert forall|offset: int| 0 <= offset < prefix.len() implies
            prefix[offset].slot_spec() < C by {
            assert(prefix[offset] == chosen[offset]);
        }
        dispatch_selected_members_len::<C>(members, head, base_len, prefix);
    }
}

proof fn dispatch_selected_slots_selected_fact(
    slots: Seq<Slot>,
    chosen: Seq<RequestId>,
    epoch: u64,
    offset: int,
)
    requires
        forall|chosen_offset: int| 0 <= chosen_offset < chosen.len() ==>
            chosen[chosen_offset].slot_spec() < slots.len(),
        selected_request_slots(chosen).no_duplicates(),
        0 <= offset < chosen.len(),
        chosen[offset].slot_spec() < slots.len(),
    ensures {
        let slot_index = chosen[offset].slot_spec() as int;
        dispatch_selected_slots(slots, chosen, epoch)[slot_index] == Slot {
            state: RequestState::InFlight,
            active_epoch: epoch,
            ..slots[slot_index]
        }
    },
    decreases chosen.len(),
{
    assert(chosen.len() > 0);
    let prefix = chosen.subrange(0, chosen.len() - 1);
    let last = chosen[chosen.len() - 1];
    let last_slot = last.slot_spec() as int;
    assert(chosen.subrange(0, chosen.len() - 1) == prefix);
    assert(dispatch_selected_slots(slots, chosen, epoch)
        == dispatch_selected_slots(slots, prefix, epoch).update(
            last_slot,
            Slot {
                state: RequestState::InFlight,
                active_epoch: epoch,
                ..slots[last_slot]
            },
        ));
    let chosen_slots = selected_request_slots(chosen);
    selected_request_slots_subrange(chosen, chosen.len() - 1);
    dispatch_selected_slots_len(slots, chosen, epoch);
    assert forall|prefix_offset: int| 0 <= prefix_offset < prefix.len() implies
        prefix[prefix_offset].slot_spec() < slots.len() by {
        assert(prefix[prefix_offset] == chosen[prefix_offset]);
    }
    dispatch_selected_slots_len(slots, prefix, epoch);
    reveal(dispatch_selected_slots);
    if offset < chosen.len() - 1 {
        assert(prefix[offset] == chosen[offset]);
        assert forall|prefix_offset: int| 0 <= prefix_offset < prefix.len() implies
            prefix[prefix_offset].slot_spec() < slots.len() by {
            assert(prefix[prefix_offset] == chosen[prefix_offset]);
        }
        assert(selected_request_slots(prefix).no_duplicates()) by {
            reveal(Seq::no_duplicates);
            assert forall|left: int, right: int|
                0 <= left < selected_request_slots(prefix).len()
                    && 0 <= right < selected_request_slots(prefix).len()
                    && left != right implies
                        selected_request_slots(prefix)[left]
                            != selected_request_slots(prefix)[right] by {
                assert(chosen_slots[left] != chosen_slots[right]);
            }
        }
        dispatch_selected_slots_selected_fact(slots, prefix, epoch, offset);
        assert(chosen[offset].slot_spec() != last.slot_spec()) by {
            reveal(Seq::no_duplicates);
            reveal(selected_request_slots);
            assert(chosen_slots[offset]
                != chosen_slots[chosen.len() - 1]);
        }
        let slot_index = chosen[offset].slot_spec() as int;
        assert(0 <= slot_index < slots.len());
        assert(dispatch_selected_slots(slots, chosen, epoch)[slot_index]
            == dispatch_selected_slots(slots, prefix, epoch)[slot_index]);
        assert(dispatch_selected_slots(slots, chosen, epoch)[slot_index] == Slot {
            state: RequestState::InFlight,
            active_epoch: epoch,
            ..slots[slot_index]
        });
    } else {
        assert(offset == chosen.len() - 1);
        let slot_index = chosen[offset].slot_spec() as int;
        assert(slot_index == last_slot);
        assert(0 <= slot_index < slots.len());
        assert(dispatch_selected_slots(slots, chosen, epoch)[slot_index] == Slot {
            state: RequestState::InFlight,
            active_epoch: epoch,
            ..slots[slot_index]
        });
    }
}

proof fn dispatch_selected_slots_frame_fact(
    slots: Seq<Slot>,
    chosen: Seq<RequestId>,
    epoch: u64,
    slot_index: int,
)
    requires
        forall|chosen_offset: int| 0 <= chosen_offset < chosen.len() ==>
            chosen[chosen_offset].slot_spec() < slots.len(),
        0 <= slot_index < slots.len(),
        !selected_request_slots(chosen).contains(slot_index),
    ensures dispatch_selected_slots(slots, chosen, epoch)[slot_index] == slots[slot_index],
    decreases chosen.len(),
{
    dispatch_selected_slots_len(slots, chosen, epoch);
    reveal(dispatch_selected_slots);
    if chosen.len() > 0 {
        let prefix = chosen.subrange(0, chosen.len() - 1);
        let last = chosen[chosen.len() - 1];
        selected_request_slots_subrange(chosen, chosen.len() - 1);
        assert(last.slot_spec() as int != slot_index) by {
            if last.slot_spec() as int == slot_index {
                reveal(selected_request_slots);
                assert(selected_request_slots(chosen)[chosen.len() - 1] == slot_index);
                assert(selected_request_slots(chosen).contains(slot_index));
            }
        }
        assert forall|prefix_offset: int| 0 <= prefix_offset < prefix.len() implies
            prefix[prefix_offset].slot_spec() < slots.len() by {
            assert(prefix[prefix_offset] == chosen[prefix_offset]);
        }
        dispatch_selected_slots_len(slots, prefix, epoch);
        assert(!selected_request_slots(prefix).contains(slot_index)) by {
            if selected_request_slots(prefix).contains(slot_index) {
                let prefix_offset = choose|prefix_offset: int|
                    0 <= prefix_offset < selected_request_slots(prefix).len()
                        && selected_request_slots(prefix)[prefix_offset] == slot_index;
                assert(selected_request_slots(chosen)[prefix_offset] == slot_index);
                assert(selected_request_slots(chosen).contains(slot_index));
            }
        }
        dispatch_selected_slots_frame_fact(slots, prefix, epoch, slot_index);
    }
}

proof fn dispatch_selected_counts_preserved(
    slots: Seq<Slot>,
    chosen: Seq<RequestId>,
    epoch: u64,
    prefix_len: nat,
)
    requires
        prefix_len <= slots.len(),
        forall|offset: int| 0 <= offset < chosen.len() ==> {
            let request = #[trigger] chosen[offset];
            let slot_index = request.slot_spec() as int;
            &&& 0 <= slot_index < prefix_len
            &&& slots[slot_index].state == RequestState::Ready
        },
        selected_request_slots(chosen).no_duplicates(),
    ensures
        live_slot_count(dispatch_selected_slots(slots, chosen, epoch), prefix_len)
            == live_slot_count(slots, prefix_len),
        nonreclaim_live_count(dispatch_selected_slots(slots, chosen, epoch), prefix_len)
            == nonreclaim_live_count(slots, prefix_len),
    decreases chosen.len(),
{
    assert forall|offset: int| 0 <= offset < chosen.len() implies
        chosen[offset].slot_spec() < slots.len() by {
        assert((chosen[offset].slot_spec() as int) < prefix_len);
    }
    dispatch_selected_slots_len(slots, chosen, epoch);
    reveal(dispatch_selected_slots);
    if chosen.len() > 0 {
        let prefix = chosen.subrange(0, chosen.len() - 1);
        let last = chosen[chosen.len() - 1];
        let last_slot = last.slot_spec() as int;
        selected_request_slots_subrange(chosen, chosen.len() - 1);
        assert forall|offset: int| 0 <= offset < prefix.len() implies {
            let request = #[trigger] prefix[offset];
            let slot_index = request.slot_spec() as int;
            &&& 0 <= slot_index < prefix_len
            &&& slots[slot_index].state == RequestState::Ready
        } by {
            assert(prefix[offset] == chosen[offset]);
        }
        assert(selected_request_slots(prefix).no_duplicates()) by {
            reveal(Seq::no_duplicates);
            assert forall|left: int, right: int|
                0 <= left < selected_request_slots(prefix).len()
                    && 0 <= right < selected_request_slots(prefix).len()
                    && left != right implies
                        selected_request_slots(prefix)[left]
                            != selected_request_slots(prefix)[right] by {
                assert(selected_request_slots(chosen)[left]
                    != selected_request_slots(chosen)[right]);
            }
        }
        dispatch_selected_counts_preserved(slots, prefix, epoch, prefix_len);
        assert forall|offset: int| 0 <= offset < prefix.len() implies
            prefix[offset].slot_spec() < slots.len() by {
            assert((prefix[offset].slot_spec() as int) < prefix_len);
        }
        dispatch_selected_slots_len(slots, prefix, epoch);
        assert(!selected_request_slots(prefix).contains(last_slot)) by {
            if selected_request_slots(prefix).contains(last_slot) {
                let prior = choose|prior: int| 0 <= prior < prefix.len()
                    && selected_request_slots(prefix)[prior] == last_slot;
                reveal(Seq::no_duplicates);
                assert(selected_request_slots(chosen)[prior]
                    == selected_request_slots(chosen)[chosen.len() - 1]);
            }
        }
        dispatch_selected_slots_frame_fact(slots, prefix, epoch, last_slot);
        let prior_slots = dispatch_selected_slots(slots, prefix, epoch);
        let replacement = Slot {
            state: RequestState::InFlight,
            active_epoch: epoch,
            ..slots[last_slot]
        };
        assert(prior_slots[last_slot] == slots[last_slot]);
        live_count_update_nonvacant(prior_slots, last_slot, replacement, prefix_len);
        nonreclaim_count_update_preserved(prior_slots, last_slot, replacement, prefix_len);
    }
}

proof fn dispatch_selected_members_selected_fact<const C: usize>(
    members: Seq<RequestId>,
    head: usize,
    base_len: usize,
    chosen: Seq<RequestId>,
    offset: int,
)
    requires
        C > 0,
        members.len() == C,
        head < C,
        base_len + chosen.len() <= C,
        forall|chosen_offset: int| 0 <= chosen_offset < chosen.len() ==>
            chosen[chosen_offset].slot_spec() < C,
        0 <= offset < chosen.len(),
    ensures
        dispatch_selected_members::<C>(members, head, base_len, chosen)[
            ring_position::<C>(head, (base_len + offset) as nat)
        ] == chosen[offset],
    decreases chosen.len(),
{
    assert(chosen.len() > 0);
    let prefix = chosen.subrange(0, chosen.len() - 1);
    let last_position = ring_position::<C>(
        head,
        (base_len + chosen.len() - 1) as nat,
    );
    dispatch_selected_members_len::<C>(members, head, base_len, chosen);
    assert forall|prefix_offset: int| 0 <= prefix_offset < prefix.len() implies
        prefix[prefix_offset].slot_spec() < C by {
        assert(prefix[prefix_offset] == chosen[prefix_offset]);
    }
    dispatch_selected_members_len::<C>(members, head, base_len, prefix);
    reveal(dispatch_selected_members);
    assert(dispatch_selected_members::<C>(members, head, base_len, chosen)
        == dispatch_selected_members::<C>(members, head, base_len, prefix).update(
            last_position,
            chosen[chosen.len() - 1],
        ));
    if offset < chosen.len() - 1 {
        assert(prefix[offset] == chosen[offset]);
        assert forall|prefix_offset: int| 0 <= prefix_offset < prefix.len() implies
            prefix[prefix_offset].slot_spec() < C by {
            assert(prefix[prefix_offset] == chosen[prefix_offset]);
        }
        dispatch_selected_members_selected_fact::<C>(
            members,
            head,
            base_len,
            prefix,
            offset,
        );
        ring_position_injective::<C>(
            head,
            (base_len + offset) as nat,
            (base_len + chosen.len() - 1) as nat,
        );
    } else {
        assert(offset == chosen.len() - 1);
        assert(ring_position::<C>(head, (base_len + offset) as nat) == last_position);
    }
}

proof fn dispatch_selected_members_selected_slots_differ<const C: usize>(
    members: Seq<RequestId>,
    head: usize,
    base_len: usize,
    chosen: Seq<RequestId>,
    left: int,
    right: int,
)
    requires
        C > 0,
        members.len() == C,
        head < C,
        base_len + chosen.len() <= C,
        forall|chosen_offset: int| 0 <= chosen_offset < chosen.len() ==>
            chosen[chosen_offset].slot_spec() < C,
        selected_request_slots(chosen).no_duplicates(),
        0 <= left < chosen.len(),
        0 <= right < chosen.len(),
        left != right,
    ensures
        dispatch_selected_members::<C>(members, head, base_len, chosen)[
            ring_position::<C>(head, (base_len + left) as nat)
        ].slot_spec()
            != dispatch_selected_members::<C>(members, head, base_len, chosen)[
                ring_position::<C>(head, (base_len + right) as nat)
            ].slot_spec(),
{
    dispatch_selected_members_len::<C>(members, head, base_len, chosen);
    dispatch_selected_members_selected_fact::<C>(members, head, base_len, chosen, left);
    dispatch_selected_members_selected_fact::<C>(members, head, base_len, chosen, right);
    reveal(Seq::no_duplicates);
    reveal(selected_request_slots);
    assert(selected_request_slots(chosen).len() == chosen.len());
    assert(selected_request_slots(chosen)[left] != selected_request_slots(chosen)[right]);
    assert(chosen[left].slot_spec() != chosen[right].slot_spec());
}

proof fn dispatch_selected_members_frame_fact<const C: usize>(
    members: Seq<RequestId>,
    head: usize,
    base_len: usize,
    chosen: Seq<RequestId>,
    ring_index: int,
)
    requires
        C > 0,
        members.len() == C,
        head < C,
        base_len + chosen.len() <= C,
        forall|chosen_offset: int| 0 <= chosen_offset < chosen.len() ==>
            chosen[chosen_offset].slot_spec() < C,
        0 <= ring_index < C,
        !(exists|offset: int| 0 <= offset < chosen.len()
            && #[trigger] ring_position::<C>(head, (base_len + offset) as nat) == ring_index),
    ensures
        dispatch_selected_members::<C>(members, head, base_len, chosen)[ring_index]
            == members[ring_index],
    decreases chosen.len(),
{
    reveal(dispatch_selected_members);
    if chosen.len() > 0 {
        let prefix = chosen.subrange(0, chosen.len() - 1);
        let last_position = ring_position::<C>(
            head,
            (base_len + chosen.len() - 1) as nat,
        );
        assert(last_position != ring_index);
        assert forall|prefix_offset: int| 0 <= prefix_offset < prefix.len() implies
            prefix[prefix_offset].slot_spec() < C by {
            assert(prefix[prefix_offset] == chosen[prefix_offset]);
        }
        assert(!(exists|offset: int| 0 <= offset < prefix.len()
            && #[trigger] ring_position::<C>(head, (base_len + offset) as nat) == ring_index));
        dispatch_selected_members_frame_fact::<C>(
            members,
            head,
            base_len,
            prefix,
            ring_index,
        );
    }
}

spec fn ring_advance<const C: usize>(head: usize, steps: nat) -> usize
    recommends C > 0, head < C,
    decreases steps,
{
    if steps == 0 {
        head
    } else {
        next_position::<C>(ring_advance::<C>(head, (steps - 1) as nat))
    }
}

proof fn ring_advance_matches_position<const C: usize>(head: usize, steps: nat)
    requires
        C > 0,
        head < C,
        steps <= C,
    ensures
        ring_advance::<C>(head, steps) as int
            == ring_position_or_head::<C>(head, steps),
    decreases steps,
{
    if steps == 0 {
        reveal(ring_advance);
        reveal(ring_position_or_head);
    } else {
        ring_advance_matches_position::<C>(head, (steps - 1) as nat);
        ring_position_or_head_next::<C>(head, (steps - 1) as nat);
        reveal(ring_advance);
    }
}

proof fn ring_advance_bounds<const C: usize>(head: usize, steps: nat)
    requires
        C > 0,
        head < C,
        steps <= C,
    ensures
        ring_advance::<C>(head, steps) < C,
{
    ring_advance_matches_position::<C>(head, steps);
    reveal(ring_position_or_head);
    if steps < C {
        ring_position_bounds::<C>(head, steps);
    }
}

proof fn ring_position_after_advance<const C: usize>(
    head: usize,
    consumed: nat,
    offset: nat,
)
    requires
        C > 0,
        head < C,
        consumed + offset < C,
    ensures
        ring_position::<C>(ring_advance::<C>(head, consumed), offset)
            == ring_position::<C>(head, consumed + offset),
    decreases consumed,
{
    if consumed == 0 {
        reveal(ring_advance);
    } else {
        let prior = (consumed - 1) as nat;
        ring_advance_bounds::<C>(head, prior);
        ring_position_after_advance::<C>(head, prior, offset + 1);
        ring_position_after_pop::<C>(ring_advance::<C>(head, prior), offset);
        reveal(ring_advance);
    }
}

proof fn ring_position_after_pop<const C: usize>(head: usize, offset: nat)
    requires
        C > 0,
        head < C,
        offset + 1 < C,
    ensures
        ring_position::<C>(next_position::<C>(head), offset)
            == ring_position::<C>(head, offset + 1),
{
}

spec fn batch_member_sum<const C: usize>(
    batches: Seq<BatchRecord>,
    head: usize,
    count: nat,
) -> nat
    recommends C > 0, head < C, count <= C, batches.len() == C,
    decreases count,
{
    if count == 0 {
        0
    } else {
        batch_member_sum::<C>(batches, head, (count - 1) as nat)
            + batches[ring_position::<C>(head, (count - 1) as nat)].member_count as nat
    }
}

proof fn batch_member_sum_update_frame<const C: usize>(
    batches: Seq<BatchRecord>,
    head: usize,
    count: nat,
    ring_index: int,
    replacement: BatchRecord,
)
    requires
        C > 0,
        batches.len() == C,
        head < C,
        count <= C,
        0 <= ring_index < C,
        forall|offset: int| 0 <= offset < count ==>
            #[trigger] ring_position::<C>(head, offset as nat) != ring_index,
    ensures
        batch_member_sum::<C>(
            batches.update(ring_index, replacement),
            head,
            count,
        ) == batch_member_sum::<C>(batches, head, count),
    decreases count,
{
    reveal(batch_member_sum);
    if count > 0 {
        assert forall|offset: int| 0 <= offset < count - 1 implies
            #[trigger] ring_position::<C>(head, offset as nat) != ring_index by {
        }
        batch_member_sum_update_frame::<C>(
            batches,
            head,
            (count - 1) as nat,
            ring_index,
            replacement,
        );
        ring_position_bounds::<C>(head, (count - 1) as nat);
    }
}

proof fn batch_member_sum_append<const C: usize>(
    batches: Seq<BatchRecord>,
    head: usize,
    len: usize,
    replacement: BatchRecord,
)
    requires
        C > 0,
        batches.len() == C,
        head < C,
        len < C,
    ensures
        batch_member_sum::<C>(
            batches.update(ring_position::<C>(head, len as nat), replacement),
            head,
            (len + 1) as nat,
        ) == batch_member_sum::<C>(batches, head, len as nat)
            + replacement.member_count,
{
    let tail = ring_position::<C>(head, len as nat);
    ring_position_bounds::<C>(head, len as nat);
    assert forall|offset: int| 0 <= offset < len implies
        #[trigger] ring_position::<C>(head, offset as nat) != tail by {
        ring_position_injective::<C>(head, offset as nat, len as nat);
    }
    batch_member_sum_update_frame::<C>(batches, head, len as nat, tail, replacement);
    reveal(batch_member_sum);
}

proof fn batch_member_sum_monotonic<const C: usize>(
    batches: Seq<BatchRecord>,
    head: usize,
    smaller: nat,
    larger: nat,
)
    requires
        C > 0,
        head < C,
        batches.len() == C,
        smaller <= larger <= C,
    ensures
        batch_member_sum::<C>(batches, head, smaller)
            <= batch_member_sum::<C>(batches, head, larger),
    decreases larger - smaller,
{
    if smaller < larger {
        batch_member_sum_monotonic::<C>(
            batches,
            head,
            smaller,
            (larger - 1) as nat,
        );
        reveal(batch_member_sum);
    }
}

proof fn batch_member_sum_pop<const C: usize>(
    batches: Seq<BatchRecord>,
    head: usize,
    count: nat,
)
    requires
        C > 0,
        head < C,
        batches.len() == C,
        0 < count <= C,
    ensures
        batch_member_sum::<C>(batches, head, count)
            == batches[head as int].member_count
                + batch_member_sum::<C>(
                    batches,
                    next_position::<C>(head),
                    (count - 1) as nat,
                ),
    decreases count,
{
    assert(next_position::<C>(head) < C) by {
        reveal(next_position);
    }
    if count == 1 {
        assert(ring_position::<C>(head, 0) == head) by {
            reveal(ring_position);
        }
        assert(batch_member_sum::<C>(batches, head, 0) == 0) by {
            reveal(batch_member_sum);
        }
        assert(batch_member_sum::<C>(batches, head, 1)
            == batches[head as int].member_count) by {
            reveal(batch_member_sum);
        }
        assert(batch_member_sum::<C>(batches, next_position::<C>(head), 0) == 0) by {
            reveal(batch_member_sum);
        }
    } else {
        let prior = (count - 1) as nat;
        batch_member_sum_pop::<C>(batches, head, prior);
        ring_position_after_pop::<C>(head, (prior - 1) as nat);
        assert(ring_position::<C>(next_position::<C>(head), (prior - 1) as nat)
            == ring_position::<C>(head, prior));
        assert(batch_member_sum::<C>(batches, head, count)
            == batch_member_sum::<C>(batches, head, prior)
                + batches[ring_position::<C>(head, prior)].member_count) by {
            reveal(batch_member_sum);
        }
        assert(batch_member_sum::<C>(batches, next_position::<C>(head), prior)
            == batch_member_sum::<C>(
                batches,
                next_position::<C>(head),
                (prior - 1) as nat,
            ) + batches[ring_position::<C>(head, prior)].member_count) by {
            reveal(batch_member_sum);
        }
    }
}

proof fn positive_batch_count_le_sum<const C: usize>(
    batches: Seq<BatchRecord>,
    head: usize,
    count: nat,
)
    requires
        C > 0,
        head < C,
        batches.len() == C,
        count <= C,
        forall|offset: int| 0 <= offset < count ==>
            (#[trigger] batches[ring_position::<C>(head, offset as nat)].member_count) > 0,
    ensures
        count <= batch_member_sum::<C>(batches, head, count),
    decreases count,
{
    if count > 0 {
        let prior = (count - 1) as nat;
        positive_batch_count_le_sum::<C>(batches, head, prior);
        reveal(batch_member_sum);
    }
}

proof fn batch_member_owner<const C: usize>(
    batches: Seq<BatchRecord>,
    head: usize,
    count: nat,
    member_offset: nat,
)
    requires
        C > 0,
        head < C,
        batches.len() == C,
        count <= C,
        member_offset < batch_member_sum::<C>(batches, head, count),
    ensures
        exists|batch_offset: int| 0 <= batch_offset < count
            && (#[trigger] batch_member_sum::<C>(batches, head, batch_offset as nat))
                <= member_offset < batch_member_sum::<C>(
                    batches,
                    head,
                    batch_offset as nat + 1,
                ),
    decreases count,
{
    let prior = (count - 1) as nat;
    assert(count > 0) by {
        if count == 0 {
            reveal(batch_member_sum);
        }
    }
    if member_offset < batch_member_sum::<C>(batches, head, prior) {
        batch_member_owner::<C>(batches, head, prior, member_offset);
    } else {
        assert(member_offset < batch_member_sum::<C>(batches, head, prior + 1));
        assert(exists|batch_offset: int| 0 <= batch_offset < count
            && (#[trigger] batch_member_sum::<C>(batches, head, batch_offset as nat))
                <= member_offset < batch_member_sum::<C>(
                    batches,
                    head,
                    batch_offset as nat + 1,
                )) by {
            let batch_offset = prior as int;
            assert(0 <= batch_offset < count);
        }
    }
}

spec fn live_slot_count(slots: Seq<Slot>, prefix: nat) -> nat
    recommends prefix <= slots.len(),
    decreases prefix,
{
    if prefix == 0 {
        0
    } else {
        let prior = live_slot_count(slots, (prefix - 1) as nat);
        if slots[(prefix - 1) as int].state == RequestState::Vacant {
            prior
        } else {
            prior + 1
        }
    }
}

spec fn live_slot_indices(slots: Seq<Slot>, prefix: nat) -> Seq<int>
    recommends prefix <= slots.len(),
    decreases prefix,
{
    if prefix == 0 {
        Seq::empty()
    } else {
        let prior = live_slot_indices(slots, (prefix - 1) as nat);
        if slots[(prefix - 1) as int].state == RequestState::Vacant {
            prior
        } else {
            prior.push((prefix - 1) as int)
        }
    }
}

spec fn member_slot_indices<const C: usize>(
    ring: Seq<RequestId>,
    head: usize,
    len: usize,
) -> Seq<int>
    recommends C > 0, head < C, len <= C, ring.len() == C,
{
    Seq::new(len as nat, |offset: int| {
        ring[ring_position::<C>(head, offset as nat)].slot_spec() as int
    })
}

proof fn member_slot_indices_contains_iff<const C: usize>(
    ring: Seq<RequestId>,
    head: usize,
    len: usize,
    slot_index: int,
)
    requires
        C > 0,
        head < C,
        len <= C,
        ring.len() == C,
    ensures
        member_slot_indices::<C>(ring, head, len).contains(slot_index)
            == request_ring_contains_slot::<C>(ring, head, len, slot_index),
{
    let members = member_slot_indices::<C>(ring, head, len);
    assert(members.len() == len) by {
        reveal(member_slot_indices);
    }
    if members.contains(slot_index) {
        let offset = choose|offset: int| 0 <= offset < members.len()
            && members[offset] == slot_index;
        assert(ring[ring_position::<C>(head, offset as nat)].slot_spec() as int
            == slot_index) by {
            reveal(member_slot_indices);
        }
        assert(request_ring_contains_slot::<C>(ring, head, len, slot_index)) by {
            reveal(request_ring_contains_slot);
            assert(exists|ring_offset: int| 0 <= ring_offset < len
                && (#[trigger] ring[ring_position::<C>(head, ring_offset as nat)].slot_spec())
                    == slot_index) by {
                let ring_offset = offset;
                assert(0 <= ring_offset < len);
            }
        }
    }
    if request_ring_contains_slot::<C>(ring, head, len, slot_index) {
        let offset = choose|offset: int| 0 <= offset < len
            && (#[trigger] ring[ring_position::<C>(head, offset as nat)].slot_spec())
                == slot_index;
        assert(members[offset] == slot_index) by {
            reveal(member_slot_indices);
        }
        assert(members.contains(slot_index)) by {
            reveal(Seq::contains);
        }
    }
}

proof fn seq_push_preserves_no_duplicates(values: Seq<int>, added: int)
    requires
        values.no_duplicates(),
        !values.contains(added),
    ensures
        values.push(added).no_duplicates(),
{
    reveal(Seq::no_duplicates);
    assert forall|left: int, right: int|
        0 <= left < values.push(added).len()
            && 0 <= right < values.push(added).len()
            && left != right implies values.push(added)[left] != values.push(added)[right]
    by {
        if left < values.len() && right < values.len() {
            assert(values[left] != values[right]);
        } else if left == values.len() {
            assert(right < values.len());
            assert(values.push(added)[left] == added);
            assert(values.push(added)[right] == values[right]);
            assert(values[right] != added) by {
                if values[right] == added {
                    assert(values.contains(added));
                }
            }
        } else {
            assert(right == values.len());
            assert(left < values.len());
            assert(values.push(added)[right] == added);
            assert(values.push(added)[left] == values[left]);
            assert(values[left] != added) by {
                if values[left] == added {
                    assert(values.contains(added));
                }
            }
        }
    }
}

proof fn live_slot_indices_facts(slots: Seq<Slot>, prefix: nat)
    requires prefix <= slots.len(),
    ensures
        live_slot_indices(slots, prefix).len() == live_slot_count(slots, prefix),
        live_slot_indices(slots, prefix).no_duplicates(),
        forall|slot_index: int| {
            #[trigger] live_slot_indices(slots, prefix).contains(slot_index)
                == (0 <= slot_index < prefix
                    && slots[slot_index].state != RequestState::Vacant)
        },
    decreases prefix,
{
    if prefix > 0 {
        let prior_prefix = (prefix - 1) as nat;
        let last = (prefix - 1) as int;
        live_slot_indices_facts(slots, prior_prefix);
        let prior = live_slot_indices(slots, prior_prefix);
        if slots[last].state == RequestState::Vacant {
            assert(live_slot_indices(slots, prefix) == prior) by {
                reveal(live_slot_indices);
            }
            assert forall|slot_index: int| {
                #[trigger] live_slot_indices(slots, prefix).contains(slot_index)
                    == (0 <= slot_index < prefix
                        && slots[slot_index].state != RequestState::Vacant)
            } by {
                if slot_index == last {
                    assert(!prior.contains(slot_index));
                }
            }
        } else {
            assert(!prior.contains(last));
            seq_push_preserves_no_duplicates(prior, last);
            assert(live_slot_indices(slots, prefix) == prior.push(last)) by {
                reveal(live_slot_indices);
            }
            assert forall|slot_index: int| {
                #[trigger] live_slot_indices(slots, prefix).contains(slot_index)
                    == (0 <= slot_index < prefix
                        && slots[slot_index].state != RequestState::Vacant)
            } by {
                vstd::seq_lib::lemma_seq_contains_after_push(prior, last, slot_index);
                if slot_index != last {
                    assert(slot_index < prior_prefix <==> slot_index < prefix);
                }
            }
        }
    }
    assert(live_slot_indices(slots, prefix).len() == live_slot_count(slots, prefix)) by {
        reveal(live_slot_indices);
        reveal(live_slot_count);
    }
}

spec fn nonreclaim_live_count(slots: Seq<Slot>, prefix: nat) -> nat
    recommends prefix <= slots.len(),
    decreases prefix,
{
    if prefix == 0 {
        0
    } else {
        let prior = nonreclaim_live_count(slots, (prefix - 1) as nat);
        let slot = slots[(prefix - 1) as int];
        if slot.state != RequestState::Vacant && !slot.in_reclaim_ring {
            prior + 1
        } else {
            prior
        }
    }
}

proof fn live_count_all_vacant(slots: Seq<Slot>, prefix: nat)
    requires
        prefix <= slots.len(),
        forall|index: int| 0 <= index < prefix ==>
            (#[trigger] slots[index]).state == RequestState::Vacant,
    ensures live_slot_count(slots, prefix) == 0,
    decreases prefix,
{
    reveal(live_slot_count);
    if prefix > 0 {
        live_count_all_vacant(slots, (prefix - 1) as nat);
    }
}

proof fn live_count_positive_at(slots: Seq<Slot>, prefix: nat, slot_index: int)
    requires
        prefix <= slots.len(),
        0 <= slot_index < prefix,
        slots[slot_index].state != RequestState::Vacant,
    ensures live_slot_count(slots, prefix) > 0,
    decreases prefix,
{
    if slot_index < prefix - 1 {
        live_count_positive_at(slots, (prefix - 1) as nat, slot_index);
    }
    reveal(live_slot_count);
}

proof fn live_count_bounded(slots: Seq<Slot>, prefix: nat)
    requires prefix <= slots.len(),
    ensures live_slot_count(slots, prefix) <= prefix,
    decreases prefix,
{
    if prefix > 0 {
        live_count_bounded(slots, (prefix - 1) as nat);
    }
    reveal(live_slot_count);
}

proof fn live_count_below_if_vacant(
    slots: Seq<Slot>,
    prefix: nat,
    slot_index: int,
)
    requires
        prefix <= slots.len(),
        0 <= slot_index < prefix,
        slots[slot_index].state == RequestState::Vacant,
    ensures live_slot_count(slots, prefix) < prefix,
    decreases prefix,
{
    if slot_index < prefix - 1 {
        live_count_below_if_vacant(slots, (prefix - 1) as nat, slot_index);
    } else {
        assert(slot_index == prefix - 1);
        live_count_bounded(slots, (prefix - 1) as nat);
    }
    reveal(live_slot_count);
}

proof fn nonreclaim_count_positive_at(
    slots: Seq<Slot>,
    prefix: nat,
    slot_index: int,
)
    requires
        prefix <= slots.len(),
        0 <= slot_index < prefix,
        slots[slot_index].state != RequestState::Vacant,
        !slots[slot_index].in_reclaim_ring,
    ensures nonreclaim_live_count(slots, prefix) > 0,
    decreases prefix,
{
    if slot_index < prefix - 1 {
        nonreclaim_count_positive_at(slots, (prefix - 1) as nat, slot_index);
    }
    reveal(nonreclaim_live_count);
}

proof fn nonreclaim_count_all_vacant(slots: Seq<Slot>, prefix: nat)
    requires
        prefix <= slots.len(),
        forall|index: int| 0 <= index < prefix ==>
            (#[trigger] slots[index]).state == RequestState::Vacant,
    ensures nonreclaim_live_count(slots, prefix) == 0,
    decreases prefix,
{
    reveal(nonreclaim_live_count);
    if prefix > 0 {
        nonreclaim_count_all_vacant(slots, (prefix - 1) as nat);
    }
}

proof fn live_count_update_after_prefix(
    slots: Seq<Slot>,
    slot_index: int,
    replacement: Slot,
    prefix: nat,
)
    requires
        prefix <= slots.len(),
        prefix <= slot_index < slots.len(),
    ensures
        live_slot_count(slots.update(slot_index, replacement), prefix)
            == live_slot_count(slots, prefix),
    decreases prefix,
{
    if prefix > 0 {
        live_count_update_after_prefix(slots, slot_index, replacement, (prefix - 1) as nat);
        assert(slot_index != prefix - 1);
        assert(slots.update(slot_index, replacement)[(prefix - 1) as int]
            == slots[(prefix - 1) as int]);
    }
    assert(live_slot_count(slots.update(slot_index, replacement), prefix)
        == live_slot_count(slots, prefix)) by {
        reveal(live_slot_count);
    }
}

proof fn nonreclaim_count_update_after_prefix(
    slots: Seq<Slot>,
    slot_index: int,
    replacement: Slot,
    prefix: nat,
)
    requires
        prefix <= slots.len(),
        prefix <= slot_index < slots.len(),
    ensures
        nonreclaim_live_count(slots.update(slot_index, replacement), prefix)
            == nonreclaim_live_count(slots, prefix),
    decreases prefix,
{
    if prefix > 0 {
        nonreclaim_count_update_after_prefix(
            slots,
            slot_index,
            replacement,
            (prefix - 1) as nat,
        );
        assert(slot_index != prefix - 1);
        assert(slots.update(slot_index, replacement)[(prefix - 1) as int]
            == slots[(prefix - 1) as int]);
    }
    assert(nonreclaim_live_count(slots.update(slot_index, replacement), prefix)
        == nonreclaim_live_count(slots, prefix)) by {
        reveal(nonreclaim_live_count);
    }
}

proof fn live_count_update_nonvacant(
    slots: Seq<Slot>,
    slot_index: int,
    replacement: Slot,
    prefix: nat,
)
    requires
        prefix <= slots.len(),
        0 <= slot_index < prefix,
        slots[slot_index].state != RequestState::Vacant,
        replacement.state != RequestState::Vacant,
    ensures
        live_slot_count(slots.update(slot_index, replacement), prefix)
            == live_slot_count(slots, prefix),
    decreases prefix,
{
    if prefix > 0 {
        if slot_index < prefix - 1 {
            live_count_update_nonvacant(slots, slot_index, replacement, (prefix - 1) as nat);
            assert(slots.update(slot_index, replacement)[(prefix - 1) as int]
                == slots[(prefix - 1) as int]);
        } else {
            assert(slot_index == prefix - 1);
            assert(slots.update(slot_index, replacement)[(prefix - 1) as int] == replacement);
            live_count_update_after_prefix(
                slots,
                slot_index,
                replacement,
                (prefix - 1) as nat,
            );
        }
    }
    assert(live_slot_count(slots.update(slot_index, replacement), prefix)
        == live_slot_count(slots, prefix)) by {
        reveal(live_slot_count);
    }
}

proof fn live_count_update_admit(
    slots: Seq<Slot>,
    slot_index: int,
    replacement: Slot,
    prefix: nat,
)
    requires
        prefix <= slots.len(),
        0 <= slot_index < prefix,
        slots[slot_index].state == RequestState::Vacant,
        replacement.state != RequestState::Vacant,
    ensures
        live_slot_count(slots.update(slot_index, replacement), prefix)
            == live_slot_count(slots, prefix) + 1,
    decreases prefix,
{
    if prefix > 0 {
        if slot_index < prefix - 1 {
            live_count_update_admit(slots, slot_index, replacement, (prefix - 1) as nat);
            assert(slots.update(slot_index, replacement)[(prefix - 1) as int]
                == slots[(prefix - 1) as int]);
        } else {
            assert(slot_index == prefix - 1);
            assert(slots.update(slot_index, replacement)[(prefix - 1) as int] == replacement);
            live_count_update_after_prefix(
                slots,
                slot_index,
                replacement,
                (prefix - 1) as nat,
            );
        }
    }
    assert(live_slot_count(slots.update(slot_index, replacement), prefix)
        == live_slot_count(slots, prefix) + 1) by {
        reveal(live_slot_count);
    }
}

proof fn live_count_update_reclaim(
    slots: Seq<Slot>,
    slot_index: int,
    replacement: Slot,
    prefix: nat,
)
    requires
        prefix <= slots.len(),
        0 <= slot_index < prefix,
        slots[slot_index].state != RequestState::Vacant,
        replacement.state == RequestState::Vacant,
    ensures
        live_slot_count(slots.update(slot_index, replacement), prefix) + 1
            == live_slot_count(slots, prefix),
    decreases prefix,
{
    if prefix > 0 {
        if slot_index < prefix - 1 {
            live_count_update_reclaim(slots, slot_index, replacement, (prefix - 1) as nat);
            assert(slots.update(slot_index, replacement)[(prefix - 1) as int]
                == slots[(prefix - 1) as int]);
        } else {
            assert(slot_index == prefix - 1);
            assert(slots.update(slot_index, replacement)[(prefix - 1) as int] == replacement);
            live_count_update_after_prefix(
                slots,
                slot_index,
                replacement,
                (prefix - 1) as nat,
            );
        }
    }
    assert(live_slot_count(slots.update(slot_index, replacement), prefix) + 1
        == live_slot_count(slots, prefix)) by {
        reveal(live_slot_count);
    }
}

proof fn nonreclaim_count_update_preserved(
    slots: Seq<Slot>,
    slot_index: int,
    replacement: Slot,
    prefix: nat,
)
    requires
        prefix <= slots.len(),
        0 <= slot_index < prefix,
        (slots[slot_index].state != RequestState::Vacant
            && !slots[slot_index].in_reclaim_ring)
            == (replacement.state != RequestState::Vacant && !replacement.in_reclaim_ring),
    ensures
        nonreclaim_live_count(slots.update(slot_index, replacement), prefix)
            == nonreclaim_live_count(slots, prefix),
    decreases prefix,
{
    if prefix > 0 {
        if slot_index < prefix - 1 {
            nonreclaim_count_update_preserved(
                slots,
                slot_index,
                replacement,
                (prefix - 1) as nat,
            );
            assert(slots.update(slot_index, replacement)[(prefix - 1) as int]
                == slots[(prefix - 1) as int]);
        } else {
            assert(slot_index == prefix - 1);
            assert(slots.update(slot_index, replacement)[(prefix - 1) as int] == replacement);
            nonreclaim_count_update_after_prefix(
                slots,
                slot_index,
                replacement,
                (prefix - 1) as nat,
            );
        }
    }
    assert(nonreclaim_live_count(slots.update(slot_index, replacement), prefix)
        == nonreclaim_live_count(slots, prefix)) by {
        reveal(nonreclaim_live_count);
    }
}

proof fn nonreclaim_count_update_add(
    slots: Seq<Slot>,
    slot_index: int,
    replacement: Slot,
    prefix: nat,
)
    requires
        prefix <= slots.len(),
        0 <= slot_index < prefix,
        slots[slot_index].state == RequestState::Vacant
            || slots[slot_index].in_reclaim_ring,
        replacement.state != RequestState::Vacant,
        !replacement.in_reclaim_ring,
    ensures
        nonreclaim_live_count(slots.update(slot_index, replacement), prefix)
            == nonreclaim_live_count(slots, prefix) + 1,
    decreases prefix,
{
    if prefix > 0 {
        if slot_index < prefix - 1 {
            nonreclaim_count_update_add(slots, slot_index, replacement, (prefix - 1) as nat);
            assert(slots.update(slot_index, replacement)[(prefix - 1) as int]
                == slots[(prefix - 1) as int]);
        } else {
            assert(slot_index == prefix - 1);
            assert(slots.update(slot_index, replacement)[(prefix - 1) as int] == replacement);
            nonreclaim_count_update_after_prefix(
                slots,
                slot_index,
                replacement,
                (prefix - 1) as nat,
            );
        }
    }
    assert(nonreclaim_live_count(slots.update(slot_index, replacement), prefix)
        == nonreclaim_live_count(slots, prefix) + 1) by {
        reveal(nonreclaim_live_count);
    }
}

proof fn nonreclaim_count_update_remove(
    slots: Seq<Slot>,
    slot_index: int,
    replacement: Slot,
    prefix: nat,
)
    requires
        prefix <= slots.len(),
        0 <= slot_index < prefix,
        slots[slot_index].state != RequestState::Vacant,
        !slots[slot_index].in_reclaim_ring,
        replacement.state == RequestState::Vacant || replacement.in_reclaim_ring,
    ensures
        nonreclaim_live_count(slots.update(slot_index, replacement), prefix) + 1
            == nonreclaim_live_count(slots, prefix),
    decreases prefix,
{
    if prefix > 0 {
        if slot_index < prefix - 1 {
            nonreclaim_count_update_remove(slots, slot_index, replacement, (prefix - 1) as nat);
            assert(slots.update(slot_index, replacement)[(prefix - 1) as int]
                == slots[(prefix - 1) as int]);
        } else {
            assert(slot_index == prefix - 1);
            assert(slots.update(slot_index, replacement)[(prefix - 1) as int] == replacement);
            nonreclaim_count_update_after_prefix(
                slots,
                slot_index,
                replacement,
                (prefix - 1) as nat,
            );
        }
    }
    assert(nonreclaim_live_count(slots.update(slot_index, replacement), prefix) + 1
        == nonreclaim_live_count(slots, prefix)) by {
        reveal(nonreclaim_live_count);
    }
}

spec fn usize_ring_contains<const C: usize>(
    ring: Seq<usize>,
    head: usize,
    len: usize,
    slot_index: int,
) -> bool
    recommends C > 0, head < C, len <= C, ring.len() == C,
{
    exists|offset: int| 0 <= offset < len
        && (#[trigger] ring[ring_position::<C>(head, offset as nat)]) == slot_index
}

proof fn usize_ring_append_facts<const C: usize>(
    before: Seq<usize>,
    after: Seq<usize>,
    head: usize,
    len: usize,
    added: usize,
)
    requires
        C > 0,
        head < C,
        len < C,
        before.len() == C,
        after == before.update(ring_position::<C>(head, len as nat), added),
        !usize_ring_contains::<C>(before, head, len, added as int),
        forall|left: int, right: int|
            0 <= left < len && 0 <= right < len && left != right ==>
                #[trigger] usize_ring_entries_differ::<C>(before, head, left, right),
    ensures
        forall|slot_index: int| {
            #[trigger] usize_ring_contains::<C>(
                after,
                head,
                ((len as int) + 1) as usize,
                slot_index,
            )
                == (usize_ring_contains::<C>(before, head, len, slot_index)
                    || slot_index == added)
        },
        forall|left: int, right: int|
            0 <= left < len + 1 && 0 <= right < len + 1 && left != right ==>
                #[trigger] usize_ring_entries_differ::<C>(after, head, left, right),
        forall|offset: int| 0 <= offset < len ==>
            #[trigger] after[ring_position::<C>(head, offset as nat)]
                == before[ring_position::<C>(head, offset as nat)],
        after[ring_position::<C>(head, len as nat)] == added,
{
    let new_len: usize = ((len as int) + 1) as usize;
    let tail = ring_position::<C>(head, len as nat);
    ring_position_bounds::<C>(head, len as nat);
    assert(new_len as int == len as int + 1);
    assert(new_len <= C);
    assert(0 <= tail < after.len());
    assert(after[tail] == added);
    assert forall|offset: int| 0 <= offset < len implies
        #[trigger] after[ring_position::<C>(head, offset as nat)]
            == before[ring_position::<C>(head, offset as nat)]
    by {
        ring_position_bounds::<C>(head, offset as nat);
        ring_position_injective::<C>(head, offset as nat, len as nat);
        assert(ring_position::<C>(head, offset as nat) != tail);
    }
    assert forall|slot_index: int| usize_ring_contains::<C>(after, head, new_len, slot_index)
        implies usize_ring_contains::<C>(before, head, len, slot_index)
            || slot_index == added by {
        if usize_ring_contains::<C>(after, head, new_len, slot_index) {
            reveal(usize_ring_contains);
            let offset = choose|offset: int| 0 <= offset < new_len
                && #[trigger] after[ring_position::<C>(head, offset as nat)] == slot_index;
            if offset < len {
                assert(before[ring_position::<C>(head, offset as nat)] == slot_index);
                assert(exists|old_offset: int| 0 <= old_offset < len
                    && #[trigger] before[ring_position::<C>(head, old_offset as nat)]
                        == slot_index) by {
                    assert(0 <= offset < len);
                }
                assert(usize_ring_contains::<C>(before, head, len, slot_index));
            } else {
                assert(offset == len);
                assert(slot_index == added);
            }
        }
    }
    assert forall|slot_index: int| usize_ring_contains::<C>(before, head, len, slot_index)
            || slot_index == added
        implies usize_ring_contains::<C>(after, head, new_len, slot_index) by {
        if usize_ring_contains::<C>(before, head, len, slot_index) {
            reveal(usize_ring_contains);
            let offset = choose|offset: int| 0 <= offset < len
                && #[trigger] before[ring_position::<C>(head, offset as nat)] == slot_index;
            assert(after[ring_position::<C>(head, offset as nat)] == slot_index);
            assert(exists|new_offset: int| 0 <= new_offset < new_len
                && #[trigger] after[ring_position::<C>(head, new_offset as nat)]
                    == slot_index) by {
                assert(0 <= offset < new_len);
            }
        } else if slot_index == added {
            reveal(usize_ring_contains);
            assert(after[tail] == slot_index);
            assert(after[ring_position::<C>(head, len as nat)] == slot_index);
            let len_int: int = len as int;
            let new_len_int: int = new_len as int;
            assert(0 <= len_int < new_len_int);
            assert(exists|offset: int| 0 <= offset < new_len
                && #[trigger] after[ring_position::<C>(head, offset as nat)] == slot_index) by {
                let offset: int = len_int;
                assert(0 <= offset < new_len);
                assert(after[ring_position::<C>(head, offset as nat)] == slot_index);
            }
        }
    }
    assert forall|slot_index: int| usize_ring_contains::<C>(
        after,
        head,
        new_len,
        slot_index,
    ) == (usize_ring_contains::<C>(before, head, len, slot_index)
        || slot_index == added) by {}
    assert forall|left: int, right: int|
        0 <= left < len + 1 && 0 <= right < len + 1 && left != right implies
            usize_ring_entries_differ::<C>(after, head, left, right)
    by {
        if left < len && right < len {
            assert(usize_ring_entries_differ::<C>(before, head, left, right));
        } else if left == len {
            assert(right < len);
            assert(before[ring_position::<C>(head, right as nat)] != added);
        } else {
            assert(right == len);
            assert(left < len);
            assert(before[ring_position::<C>(head, left as nat)] != added);
        }
        reveal(usize_ring_entries_differ);
    }
}

proof fn usize_ring_pop_facts<const C: usize>(ring: Seq<usize>, head: usize, len: usize)
    requires
        C > 0,
        head < C,
        0 < len <= C,
        ring.len() == C,
        forall|left: int, right: int|
            0 <= left < len && 0 <= right < len && left != right ==>
                #[trigger] usize_ring_entries_differ::<C>(ring, head, left, right),
    ensures
        forall|slot_index: int| {
            #[trigger] usize_ring_contains::<C>(
                ring,
                next_position::<C>(head),
                ((len as int) - 1) as usize,
                slot_index,
            ) == (usize_ring_contains::<C>(ring, head, len, slot_index)
                && slot_index != ring[head as int])
        },
        forall|left: int, right: int|
            0 <= left < len - 1 && 0 <= right < len - 1 && left != right ==>
                #[trigger] usize_ring_entries_differ::<C>(
                    ring,
                    next_position::<C>(head),
                    left,
                    right,
                ),
        forall|offset: int| 0 <= offset < len - 1 ==> {
            #[trigger] ring[ring_position::<C>(next_position::<C>(head), offset as nat)]
                == ring[ring_position::<C>(head, (offset + 1) as nat)]
        },
{
    let new_head = next_position::<C>(head);
    let new_len: usize = ((len as int) - 1) as usize;
    assert(new_head < C);
    assert(new_len < C);
    assert forall|offset: int| 0 <= offset < new_len implies {
        #[trigger] ring[ring_position::<C>(new_head, offset as nat)]
            == ring[ring_position::<C>(head, (offset + 1) as nat)]
    } by {
        ring_position_after_pop::<C>(head, offset as nat);
    }
    assert forall|slot_index: int|
        usize_ring_contains::<C>(ring, new_head, new_len, slot_index)
            implies usize_ring_contains::<C>(ring, head, len, slot_index)
                && slot_index != ring[head as int] by {
        if usize_ring_contains::<C>(ring, new_head, new_len, slot_index) {
            reveal(usize_ring_contains);
            let offset = choose|offset: int| 0 <= offset < new_len
                && #[trigger] ring[ring_position::<C>(new_head, offset as nat)] == slot_index;
            let old_offset = offset + 1;
            assert(0 < old_offset < len);
            assert(ring[ring_position::<C>(head, old_offset as nat)] == slot_index);
            assert(exists|witness: int| 0 <= witness < len
                && #[trigger] ring[ring_position::<C>(head, witness as nat)] == slot_index) by {
                assert(0 <= old_offset < len);
            }
            assert(usize_ring_contains::<C>(ring, head, len, slot_index));
            assert(ring_position::<C>(head, 0) == head);
            assert(usize_ring_entries_differ::<C>(ring, head, 0, old_offset));
        }
    }
    assert forall|slot_index: int|
        usize_ring_contains::<C>(ring, head, len, slot_index)
            && slot_index != ring[head as int]
            implies usize_ring_contains::<C>(ring, new_head, new_len, slot_index) by {
        if usize_ring_contains::<C>(ring, head, len, slot_index)
            && slot_index != ring[head as int]
        {
            reveal(usize_ring_contains);
            let old_offset = choose|offset: int| 0 <= offset < len
                && #[trigger] ring[ring_position::<C>(head, offset as nat)] == slot_index;
            assert(old_offset != 0) by {
                if old_offset == 0 {
                    assert(ring_position::<C>(head, 0) == head);
                }
            }
            let offset = old_offset - 1;
            assert(0 <= offset < new_len);
            assert(ring[ring_position::<C>(new_head, offset as nat)] == slot_index);
            assert(exists|witness: int| 0 <= witness < new_len
                && #[trigger] ring[ring_position::<C>(new_head, witness as nat)] == slot_index) by {
                assert(0 <= offset < new_len);
            }
        }
    }
    assert forall|slot_index: int| usize_ring_contains::<C>(ring, new_head, new_len, slot_index)
        == (usize_ring_contains::<C>(ring, head, len, slot_index)
            && slot_index != ring[head as int]) by {}
    assert forall|left: int, right: int|
        0 <= left < new_len && 0 <= right < new_len && left != right implies
            usize_ring_entries_differ::<C>(ring, new_head, left, right)
    by {
        ring_position_after_pop::<C>(head, left as nat);
        ring_position_after_pop::<C>(head, right as nat);
        assert(usize_ring_entries_differ::<C>(ring, head, left + 1, right + 1));
        reveal(usize_ring_entries_differ);
    }
}

spec fn request_ring_contains_slot<const C: usize>(
    ring: Seq<RequestId>,
    head: usize,
    len: usize,
    slot_index: int,
) -> bool
    recommends C > 0, head < C, len <= C, ring.len() == C,
{
    exists|offset: int| 0 <= offset < len
        && (#[trigger] ring[ring_position::<C>(head, offset as nat)].slot_spec()) == slot_index
}

proof fn request_ring_contains_slot_at<const C: usize>(
    ring: Seq<RequestId>,
    head: usize,
    len: usize,
    offset: int,
    slot_index: int,
)
    requires
        C > 0,
        head < C,
        len <= C,
        ring.len() == C,
        0 <= offset < len,
        ring[ring_position::<C>(head, offset as nat)].slot_spec() == slot_index,
    ensures request_ring_contains_slot::<C>(ring, head, len, slot_index),
{
    reveal(request_ring_contains_slot);
    assert(exists|witness: int| 0 <= witness < len
        && (#[trigger] ring[ring_position::<C>(head, witness as nat)].slot_spec()) == slot_index) by {
        let witness = offset;
        assert(0 <= witness < len);
    }
}

proof fn request_ring_suffix_contains<const C: usize>(
    ring: Seq<RequestId>,
    old_head: usize,
    new_head: usize,
    old_len: usize,
    new_len: usize,
    count: usize,
    old_offset: int,
    slot_index: int,
)
    requires
        C > 0,
        old_head < C,
        old_len <= C,
        ring.len() == C,
        new_head == ring_advance::<C>(old_head, count as nat),
        new_head < C,
        new_len + count == old_len,
        count <= old_offset < old_len,
        ring[ring_position::<C>(old_head, old_offset as nat)].slot_spec() == slot_index,
    ensures request_ring_contains_slot::<C>(ring, new_head, new_len, slot_index),
{
    let offset = old_offset - count;
    assert(0 <= offset < new_len);
    ring_position_after_advance::<C>(old_head, count as nat, offset as nat);
    assert(ring[ring_position::<C>(new_head, offset as nat)].slot_spec() == slot_index);
    request_ring_contains_slot_at::<C>(ring, new_head, new_len, offset, slot_index);
}

spec fn usize_ring_entries_differ<const C: usize>(
    ring: Seq<usize>,
    head: usize,
    left: int,
    right: int,
) -> bool
    recommends
        C > 0,
        head < C,
        0 <= left < C,
        0 <= right < C,
        ring.len() == C,
{
    ring[ring_position::<C>(head, left as nat)]
        != ring[ring_position::<C>(head, right as nat)]
}

spec fn request_ring_slots_differ<const C: usize>(
    ring: Seq<RequestId>,
    head: usize,
    left: int,
    right: int,
) -> bool
    recommends
        C > 0,
        head < C,
        0 <= left < C,
        0 <= right < C,
        ring.len() == C,
{
    ring[ring_position::<C>(head, left as nat)].slot_spec()
        != ring[ring_position::<C>(head, right as nat)].slot_spec()
}

}

#[cfg(test)]
impl M1ScheduledDispatchV1 {
    pub(crate) fn for_test(epoch: CompletionEpoch, selected: &[RequestId]) -> Self {
        assert!(!selected.is_empty());
        assert!(selected.len() <= M1_MAX_ACTIVE_SEQUENCES as usize);
        let batch = DispatchBatch {
            epoch,
            member_count: selected.len(),
        };
        let mut members = [None; M1_MAX_ACTIVE_SEQUENCES as usize];
        for (destination, source) in members.iter_mut().zip(selected) {
            *destination = Some(*source);
        }
        Self { batch, members }
    }
}

#[cfg(test)]
mod tests {
    use super::{KvQuiescenceOrigin, KvQuiescencePermit, Scheduler, SchedulerError};
    use crate::cache::{KvError, KvPool};
    use crate::epoch::ExactCompletion;
    use ferric_spec::scheduling::{LifecyclePhase, RequestState};
    use ferric_spec::RequestId;

    fn output<const N: usize>() -> [RequestId; N] {
        [RequestId::new(u32::MAX, 0); N]
    }

    fn permits<const N: usize>() -> [Option<KvQuiescencePermit>; N] {
        std::array::from_fn(|_| None)
    }

    fn derived_phase<const C: usize>(
        scheduler: &Scheduler<C>,
        request: RequestId,
    ) -> LifecyclePhase {
        let slot = scheduler.slots[request.slot() as usize];
        match slot.state {
            RequestState::Vacant | RequestState::Ready => LifecyclePhase::Idle,
            RequestState::InFlight if slot.active_epoch <= scheduler.completed => {
                LifecyclePhase::AwaitingKv
            }
            RequestState::InFlight => LifecyclePhase::Executing,
            RequestState::Retiring if slot.active_epoch <= scheduler.completed => {
                LifecyclePhase::RetiringQuiescent
            }
            RequestState::Retiring => LifecyclePhase::RetiringExecuting,
        }
    }

    #[test]
    fn dispatch_rotates_and_completion_emits_exact_member_permits() {
        let mut scheduler = Scheduler::<3>::new().unwrap();
        let first = scheduler.admit().unwrap();
        let second = scheduler.admit().unwrap();
        let third = scheduler.admit().unwrap();
        let mut cache = KvPool::new(12, 4, 32).unwrap();
        cache.create_request(first).unwrap();
        cache.create_request(second).unwrap();
        cache.create_request(third).unwrap();
        cache.append_tentative(second, 2).unwrap();

        let mut members = output::<2>();
        let batch = scheduler.dispatch_ready(&mut members).unwrap().unwrap();
        assert_eq!(members, [first, second]);
        scheduler.retire(first).unwrap();
        let mut authorities = permits::<2>();
        scheduler
            .complete_exact(
                ExactCompletion::from_contracted_hsa_quiescence(batch.epoch()),
                &mut authorities,
            )
            .unwrap();
        assert_eq!(scheduler.state(first), Some(RequestState::Retiring));
        assert_eq!(scheduler.state(second), Some(RequestState::InFlight));
        assert_eq!(authorities[0].as_ref().unwrap().request(), first);
        assert_eq!(authorities[1].as_ref().unwrap().request(), second);

        let detached = cache
            .release_request(first, authorities[0].take().unwrap())
            .unwrap();
        assert_eq!(scheduler.reclaim_detached(detached).unwrap(), first);
        let finalized = cache
            .finalize_tentative(second, 1, authorities[1].take().unwrap())
            .unwrap();
        scheduler.accept_finalized(finalized).unwrap();
        assert_eq!(scheduler.state(second), Some(RequestState::Ready));

        scheduler.retire(second).unwrap();
        let terminal = scheduler.take_retiring_permit().unwrap().unwrap();
        let detached = cache.release_request(second, terminal).unwrap();
        scheduler.reclaim_detached(detached).unwrap();
        let replacement = scheduler.admit().unwrap();
        cache.create_request(replacement).unwrap();
        assert_eq!(replacement.slot(), first.slot());
        assert_ne!(replacement.generation(), first.generation());

        let next = scheduler.dispatch_ready(&mut members).unwrap().unwrap();
        assert_eq!(members[0], third);
        assert_eq!(members[1], replacement);
        assert_eq!(next.member_count(), 2);
    }

    #[test]
    fn exact_completion_rejects_skip_and_replay_without_mutation() {
        let mut scheduler = Scheduler::<2>::new().unwrap();
        let request = scheduler.admit().unwrap();
        let mut members = output::<1>();
        let batch = scheduler.dispatch_ready(&mut members).unwrap().unwrap();
        let mut authorities = permits::<1>();

        let skipped = ferric_spec::completion::CompletionEpoch::new(batch.epoch().value() + 1);
        let failure = scheduler
            .complete_exact(
                ExactCompletion::from_contracted_hsa_quiescence(skipped),
                &mut authorities,
            )
            .unwrap_err();
        let (error, returned_completion) = failure.into_parts();
        assert_eq!(error, SchedulerError::CompletionNotExactNext);
        assert_eq!(returned_completion.epoch(), skipped);
        assert_eq!(scheduler.state(request), Some(RequestState::InFlight));
        scheduler
            .complete_exact(
                ExactCompletion::from_contracted_hsa_quiescence(batch.epoch()),
                &mut authorities,
            )
            .unwrap();
        assert_eq!(scheduler.state(request), Some(RequestState::InFlight));
        let mut replay_storage = permits::<1>();
        let failure = scheduler
            .complete_exact(
                ExactCompletion::from_contracted_hsa_quiescence(batch.epoch()),
                &mut replay_storage,
            )
            .unwrap_err();
        let (error, completion) = failure.into_parts();
        assert_eq!(error, SchedulerError::NoPendingBatch);
        assert_eq!(completion.epoch(), batch.epoch());
    }

    #[test]
    fn epoch_derived_completion_frames_slots_and_only_quiesces_the_head_batch() {
        let mut scheduler = Scheduler::<4>::new().unwrap();
        let first = scheduler.admit().unwrap();
        let second = scheduler.admit().unwrap();
        let third = scheduler.admit().unwrap();
        let fourth = scheduler.admit().unwrap();
        let mut cache = KvPool::new(8, 4, 16).unwrap();
        for request in [first, second, third, fourth] {
            cache.create_request(request).unwrap();
        }

        let mut first_members = output::<2>();
        let first_batch = scheduler
            .dispatch_ready(&mut first_members)
            .unwrap()
            .unwrap();
        assert_eq!(first_members, [first, second]);
        let mut second_members = output::<2>();
        let second_batch = scheduler
            .dispatch_ready(&mut second_members)
            .unwrap()
            .unwrap();
        assert_eq!(second_members, [third, fourth]);

        scheduler.retire(first).unwrap();
        assert_eq!(
            derived_phase(&scheduler, first),
            LifecyclePhase::RetiringExecuting
        );
        assert_eq!(derived_phase(&scheduler, second), LifecyclePhase::Executing);
        assert_eq!(derived_phase(&scheduler, third), LifecyclePhase::Executing);
        assert_eq!(derived_phase(&scheduler, fourth), LifecyclePhase::Executing);

        let before_slots = scheduler.slots;
        let mut authorities = permits::<2>();
        assert_eq!(
            scheduler
                .complete_exact(
                    ExactCompletion::from_contracted_hsa_quiescence(first_batch.epoch()),
                    &mut authorities,
                )
                .unwrap(),
            2
        );

        assert_eq!(scheduler.slots, before_slots);
        assert_eq!(scheduler.completed, first_batch.epoch().value());
        assert_eq!(
            derived_phase(&scheduler, first),
            LifecyclePhase::RetiringQuiescent
        );
        assert_eq!(
            derived_phase(&scheduler, second),
            LifecyclePhase::AwaitingKv
        );
        assert_eq!(derived_phase(&scheduler, third), LifecyclePhase::Executing);
        assert_eq!(derived_phase(&scheduler, fourth), LifecyclePhase::Executing);
        assert!(scheduler.slots[third.slot() as usize].active_epoch > scheduler.completed);
        assert!(scheduler.slots[fourth.slot() as usize].active_epoch > scheduler.completed);
        assert_eq!(scheduler.member_len, 2);
        assert_eq!(scheduler.batch_len, 1);
        assert_eq!(scheduler.member_ring[scheduler.member_head], third);
        assert_eq!(
            scheduler.member_ring[super::advance::<4>(scheduler.member_head)],
            fourth
        );
        assert_eq!(
            scheduler.batch_ring[scheduler.batch_head].epoch,
            second_batch.epoch()
        );

        for (permit, request) in authorities.iter().zip([first, second]) {
            let permit = permit.as_ref().unwrap();
            assert_eq!(permit.request(), request);
            let epoch = match permit.origin() {
                KvQuiescenceOrigin::CompletedExact { epoch } => epoch,
                KvQuiescenceOrigin::NeverSubmitted => panic!("submitted request got ready permit"),
            };
            assert_eq!(epoch, scheduler.slots[request.slot() as usize].active_epoch);
            assert!(epoch <= scheduler.completed);
        }

        let stale_request = RequestId::new(second.slot(), second.generation() + 1);
        let stale_permit = KvQuiescencePermit {
            request: stale_request,
            origin: KvQuiescenceOrigin::CompletedExact {
                epoch: first_batch.epoch().value(),
            },
        };
        assert_eq!(
            cache
                .finalize_tentative(second, 0, stale_permit)
                .unwrap_err()
                .into_parts()
                .0,
            KvError::InvalidQuiescencePermit
        );

        let wrong_epoch_permit = KvQuiescencePermit {
            request: second,
            origin: KvQuiescenceOrigin::CompletedExact {
                epoch: second_batch.epoch().value(),
            },
        };
        let wrong_epoch_finalized = cache
            .finalize_tentative(second, 0, wrong_epoch_permit)
            .unwrap();
        assert_eq!(
            scheduler.accept_finalized(wrong_epoch_finalized),
            Err(SchedulerError::FinalizationMismatch)
        );
        assert_eq!(scheduler.state(second), Some(RequestState::InFlight));
    }

    #[test]
    fn later_cancel_waits_for_its_exact_completion_before_detach() {
        let mut scheduler = Scheduler::<2>::new().unwrap();
        let first = scheduler.admit().unwrap();
        let second = scheduler.admit().unwrap();
        let mut cache = KvPool::new(4, 4, 16).unwrap();
        cache.create_request(first).unwrap();
        cache.create_request(second).unwrap();

        let mut members = output::<1>();
        let first_batch = scheduler.dispatch_ready(&mut members).unwrap().unwrap();
        assert_eq!(members[0], first);
        let second_batch = scheduler.dispatch_ready(&mut members).unwrap().unwrap();
        assert_eq!(members[0], second);
        scheduler.retire(second).unwrap();

        let mut first_permit = permits::<1>();
        scheduler
            .complete_exact(
                ExactCompletion::from_contracted_hsa_quiescence(first_batch.epoch()),
                &mut first_permit,
            )
            .unwrap();
        assert_eq!(
            derived_phase(&scheduler, second),
            LifecyclePhase::RetiringExecuting
        );
        assert!(scheduler.take_retiring_permit().unwrap().is_none());

        let finalized = cache
            .finalize_tentative(first, 0, first_permit[0].take().unwrap())
            .unwrap();
        scheduler.accept_finalized(finalized).unwrap();

        let mut second_permit = permits::<1>();
        scheduler
            .complete_exact(
                ExactCompletion::from_contracted_hsa_quiescence(second_batch.epoch()),
                &mut second_permit,
            )
            .unwrap();
        assert_eq!(
            derived_phase(&scheduler, second),
            LifecyclePhase::RetiringQuiescent
        );
        let detached = cache
            .release_request(second, second_permit[0].take().unwrap())
            .unwrap();
        scheduler.reclaim_detached(detached).unwrap();
        assert_eq!(scheduler.state(second), None);
    }

    #[test]
    fn retirement_after_completion_preserves_issued_detach_authority() {
        let mut scheduler = Scheduler::<1>::new().unwrap();
        let request = scheduler.admit().unwrap();
        let mut cache = KvPool::new(2, 4, 16).unwrap();
        cache.create_request(request).unwrap();

        let mut members = output::<1>();
        let batch = scheduler.dispatch_ready(&mut members).unwrap().unwrap();
        let mut completed = permits::<1>();
        scheduler
            .complete_exact(
                ExactCompletion::from_contracted_hsa_quiescence(batch.epoch()),
                &mut completed,
            )
            .unwrap();
        assert_eq!(
            derived_phase(&scheduler, request),
            LifecyclePhase::AwaitingKv
        );

        scheduler.retire(request).unwrap();
        assert_eq!(
            derived_phase(&scheduler, request),
            LifecyclePhase::RetiringQuiescent
        );
        let detached = cache
            .release_request(request, completed[0].take().unwrap())
            .unwrap();
        scheduler.reclaim_detached(detached).unwrap();
        assert_eq!(scheduler.state(request), None);
    }

    #[test]
    fn wrong_epoch_detachment_is_rejected_without_scheduler_reuse() {
        let mut scheduler = Scheduler::<1>::new().unwrap();
        let request = scheduler.admit().unwrap();
        let mut cache = KvPool::new(2, 4, 16).unwrap();
        let mut hostile_cache = KvPool::new(2, 4, 16).unwrap();
        cache.create_request(request).unwrap();
        hostile_cache.create_request(request).unwrap();

        let mut members = output::<1>();
        let batch = scheduler.dispatch_ready(&mut members).unwrap().unwrap();
        scheduler.retire(request).unwrap();
        let mut completed = permits::<1>();
        scheduler
            .complete_exact(
                ExactCompletion::from_contracted_hsa_quiescence(batch.epoch()),
                &mut completed,
            )
            .unwrap();

        let wrong_permit = KvQuiescencePermit {
            request,
            origin: KvQuiescenceOrigin::CompletedExact {
                epoch: batch.epoch().value() + 1,
            },
        };
        let wrong_detached = hostile_cache
            .release_request(request, wrong_permit)
            .unwrap();
        let generation = scheduler.slots[request.slot() as usize].generation;
        assert_eq!(
            scheduler.reclaim_detached(wrong_detached),
            Err(SchedulerError::DetachmentMismatch)
        );
        assert_eq!(scheduler.state(request), Some(RequestState::Retiring));
        assert_eq!(
            scheduler.slots[request.slot() as usize].generation,
            generation
        );

        let detached = cache
            .release_request(request, completed[0].take().unwrap())
            .unwrap();
        scheduler.reclaim_detached(detached).unwrap();
        assert_eq!(scheduler.state(request), None);
    }

    #[test]
    fn completion_storage_failure_returns_exact_authority_for_retry() {
        let mut scheduler = Scheduler::<1>::new().unwrap();
        let request = scheduler.admit().unwrap();
        let mut members = output::<1>();
        let batch = scheduler.dispatch_ready(&mut members).unwrap().unwrap();
        let completion = ExactCompletion::from_contracted_hsa_quiescence(batch.epoch());
        let mut occupied = [Some(KvQuiescencePermit {
            request,
            origin: KvQuiescenceOrigin::NeverSubmitted,
        })];

        let failure = scheduler
            .complete_exact(completion, &mut occupied)
            .unwrap_err();
        let (error, completion) = failure.into_parts();
        assert_eq!(error, SchedulerError::CompletionStorageNotEmpty);
        assert_eq!(completion.epoch(), batch.epoch());
        assert_eq!(scheduler.state(request), Some(RequestState::InFlight));

        occupied[0] = None;
        assert_eq!(
            scheduler.complete_exact(completion, &mut occupied).unwrap(),
            1
        );
        assert_eq!(occupied[0].as_ref().unwrap().request(), request);
    }

    #[test]
    fn ready_retirement_is_immediately_reclaimable() {
        let mut scheduler = Scheduler::<1>::new().unwrap();
        let request = scheduler.admit().unwrap();
        scheduler.retire(request).unwrap();
        assert_eq!(scheduler.state(request), Some(RequestState::Retiring));
        assert_eq!(
            scheduler.slots[request.slot() as usize].active_epoch,
            super::NO_EPOCH
        );
        assert_eq!(
            derived_phase(&scheduler, request),
            LifecyclePhase::RetiringQuiescent
        );
        let permit = scheduler.take_retiring_permit().unwrap().unwrap();
        assert_eq!(permit.request(), request);
        assert_eq!(permit.origin(), KvQuiescenceOrigin::NeverSubmitted);
        assert_eq!(scheduler.live_count(), 1);
    }

    #[test]
    fn max_generation_reclaim_preflight_fails_closed_and_frames_state() {
        let mut scheduler = Scheduler::<1>::new().unwrap();
        scheduler.slots[0].generation = u32::MAX;
        let request = scheduler.admit().unwrap();
        assert_eq!(request.generation(), u32::MAX);

        let before_slots = scheduler.slots;
        let before_free_ring = scheduler.free_ring;
        let before_free_head = scheduler.free_head;
        let before_free_len = scheduler.free_len;
        let before_reclaim_ring = scheduler.reclaim_ring;
        let before_reclaim_head = scheduler.reclaim_head;
        let before_reclaim_len = scheduler.reclaim_len;
        let before_member_ring = scheduler.member_ring;
        let before_member_head = scheduler.member_head;
        let before_member_len = scheduler.member_len;
        let before_batch_ring = scheduler.batch_ring;
        let before_batch_head = scheduler.batch_head;
        let before_batch_len = scheduler.batch_len;
        let before_cursor = scheduler.cursor;
        let before_submitted = scheduler.submitted;
        let before_completed = scheduler.completed;
        let before_live_count = scheduler.live_count;

        assert_eq!(
            scheduler.reclaim_next_generation(0),
            Err(SchedulerError::GenerationExhausted)
        );
        assert_eq!(scheduler.slots, before_slots);
        assert_eq!(scheduler.free_ring, before_free_ring);
        assert_eq!(scheduler.free_head, before_free_head);
        assert_eq!(scheduler.free_len, before_free_len);
        assert_eq!(scheduler.reclaim_ring, before_reclaim_ring);
        assert_eq!(scheduler.reclaim_head, before_reclaim_head);
        assert_eq!(scheduler.reclaim_len, before_reclaim_len);
        assert_eq!(scheduler.member_ring, before_member_ring);
        assert_eq!(scheduler.member_head, before_member_head);
        assert_eq!(scheduler.member_len, before_member_len);
        assert_eq!(scheduler.batch_ring, before_batch_ring);
        assert_eq!(scheduler.batch_head, before_batch_head);
        assert_eq!(scheduler.batch_len, before_batch_len);
        assert_eq!(scheduler.cursor, before_cursor);
        assert_eq!(scheduler.submitted, before_submitted);
        assert_eq!(scheduler.completed, before_completed);
        assert_eq!(scheduler.live_count, before_live_count);
        assert_eq!(scheduler.state(request), Some(RequestState::Ready));
    }
}
