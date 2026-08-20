//! Fixed-capacity request slots and deterministic scheduling.

use crate::cache::{KvDetachedRequest, KvFinalizedRequest, MAX_REQUEST_SLOTS};
use crate::epoch::ExactCompletion;
use ferric_spec::completion::CompletionEpoch;
#[allow(unused_imports)]
use ferric_spec::scheduling::{LifecyclePhase, RequestState, RequestTransition, SequentialRequest};
use ferric_spec::RequestId;
use vstd::prelude::*;

verus! {

const NO_EPOCH: u64 = 0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Slot {
    generation: u32,
    state: RequestState,
    phase: LifecyclePhase,
    active_epoch: u64,
    last_quiescent_epoch: u64,
    in_free_ring: bool,
    in_reclaim_ring: bool,
}

impl Slot {
    const INITIAL: Self = Self {
        generation: 1,
        state: RequestState::Vacant,
        phase: LifecyclePhase::Idle,
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DispatchBatch {
    epoch: CompletionEpoch,
    member_count: usize,
}

impl DispatchBatch {
    #[must_use]
    pub const fn epoch(self) -> (epoch: CompletionEpoch)
        ensures epoch.value == self.epoch_spec().value,
    {
        self.epoch
    }

    #[must_use]
    pub const fn member_count(self) -> (count: usize)
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
    pub fn new() -> (result: Result<Self, SchedulerError>)
        ensures
            match result {
                Ok(scheduler) => {
                    &&& C > 0
                    &&& C <= MAX_REQUEST_SLOTS
                    &&& scheduler.basic_invariant()
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
        &&& self.free_len + self.live_count == C
        &&& self.reclaim_len <= self.live_count
        &&& self.member_len <= self.live_count
        &&& self.batch_len <= self.member_len
        &&& self.completed <= self.submitted
    }

    pub closed spec fn slot_invariant(&self) -> bool {
        forall|slot_index: int| 0 <= slot_index < C ==> {
            let slot = #[trigger] self.slots@[slot_index];
            match slot.state {
                RequestState::Vacant => {
                    &&& slot.active_epoch == NO_EPOCH
                    &&& slot.last_quiescent_epoch == NO_EPOCH
                    &&& slot.phase == LifecyclePhase::Idle
                    &&& slot.in_free_ring
                    &&& !slot.in_reclaim_ring
                }
                RequestState::Ready => {
                    &&& slot.active_epoch == NO_EPOCH
                    &&& slot.last_quiescent_epoch <= self.completed
                    &&& slot.phase == LifecyclePhase::Idle
                    &&& !slot.in_free_ring
                    &&& !slot.in_reclaim_ring
                }
                RequestState::InFlight => {
                    &&& (slot.phase == LifecyclePhase::Executing
                        || slot.phase == LifecyclePhase::AwaitingKv)
                    &&& (slot.phase == LifecyclePhase::Executing ==>
                        self.completed < slot.active_epoch <= self.submitted)
                    &&& (slot.phase == LifecyclePhase::AwaitingKv ==>
                        NO_EPOCH < slot.active_epoch <= self.completed)
                    &&& !slot.in_free_ring
                    &&& !slot.in_reclaim_ring
                }
                RequestState::Retiring => {
                    &&& !slot.in_free_ring
                    &&& (slot.phase == LifecyclePhase::RetiringExecuting
                        || slot.phase == LifecyclePhase::RetiringQuiescent)
                    &&& (slot.phase == LifecyclePhase::RetiringExecuting ==>
                        self.completed < slot.active_epoch <= self.submitted)
                    &&& (slot.phase == LifecyclePhase::RetiringQuiescent ==>
                        slot.active_epoch <= self.completed)
                    &&& slot.last_quiescent_epoch <= self.completed
                }
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
            &&& self.slots@[slot_index as int].phase == LifecyclePhase::RetiringQuiescent
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

    pub closed spec fn member_ring_invariant(&self) -> bool {
        &&& (forall|offset: int| 0 <= offset < self.member_len ==> {
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
            &&& (self.slots@[handle.slot_spec() as int].phase == LifecyclePhase::Executing
                || self.slots@[handle.slot_spec() as int].phase
                    == LifecyclePhase::RetiringExecuting)
        })
        &&& (forall|left: int, right: int|
            0 <= left < self.member_len && 0 <= right < self.member_len && left != right ==>
                #[trigger] request_ring_slots_differ::<C>(
                    self.member_ring@,
                    self.member_head,
                    left,
                    right,
                ))
        &&& (forall|slot_index: int| 0 <= slot_index < C ==>
            ((#[trigger] self.slots@[slot_index].phase == LifecyclePhase::Executing
                || self.slots@[slot_index].phase == LifecyclePhase::RetiringExecuting)
                == request_ring_contains_slot::<C>(
                    self.member_ring@,
                    self.member_head,
                    self.member_len,
                    slot_index,
                )))
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

    pub(crate) closed spec fn slot_model(&self, slot_index: int) -> SequentialRequest
        recommends 0 <= slot_index < C,
    {
        SequentialRequest {
            state: self.slots@[slot_index].state,
            phase: self.slots@[slot_index].phase,
        }
    }

    pub(crate) closed spec fn slot_generation_spec(&self, slot_index: int) -> u32
        recommends 0 <= slot_index < C,
    {
        self.slots@[slot_index].generation
    }

    pub(crate) closed spec fn slot_is_live_spec(&self, slot_index: int) -> bool
        recommends 0 <= slot_index < C,
    {
        self.slots@[slot_index].state != RequestState::Vacant
    }

    pub closed spec fn slots_frame_except(&self, before: &Self, changed: int) -> bool {
        forall|slot_index: int| 0 <= slot_index < C && slot_index != changed ==>
            #[trigger] self.slots@[slot_index] == before.slots@[slot_index]
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

    pub(crate) closed spec fn completion_refines(
        &self,
        before: &Self,
        completion_epoch: u64,
        before_permits: Seq<Option<KvQuiescencePermit>>,
        permits: Seq<Option<KvQuiescencePermit>>,
        result: &Result<usize, SchedulerError>,
    ) -> bool {
        match result {
            Err(error) => {
                &&& Some(*error) == completion_expected_error::<C>(
                    before,
                    completion_epoch,
                    before_permits,
                )
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
                let slot_index = request.slot_spec() as int;
                &&& finalized_expected_error::<C>(before, finalized).is_none()
                &&& slot_index < C
                &&& request.generation_spec() == before.slots@[slot_index].generation
                &&& (exists|epoch: u64|
                    finalized.origin_spec() == KvQuiescenceOrigin::CompletedExact { epoch }
                    && before.slots@[slot_index].active_epoch == epoch
                    && self.slots@[slot_index].last_quiescent_epoch == epoch)
                &&& ferric_spec::scheduling::request_transition(
                    before.slot_model(slot_index),
                    RequestTransition::FinalizeKv,
                ) == Ok(self.slot_model(slot_index))
                &&& self.slots_frame_except(before, slot_index)
                &&& self.slots@[slot_index].generation == before.slots@[slot_index].generation
                &&& self.slots@[slot_index].active_epoch == NO_EPOCH
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
        }
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
                let detached_request = detached.request_spec();
                let slot_index = detached_request.slot_spec() as int;
                let free_tail = ring_position::<C>(before.free_head, before.free_len as nat);
                &&& detached_expected_error::<C>(before, detached).is_none()
                &&& *request == detached_request
                &&& slot_index < C
                &&& detached_request.generation_spec()
                    == before.slots@[slot_index].generation
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
                &&& self.slots@[slot_index].generation
                    == before.slots@[slot_index].generation + 1
                &&& self.slots@[slot_index].active_epoch == NO_EPOCH
                &&& self.slots@[slot_index].last_quiescent_epoch == NO_EPOCH
                &&& self.slots@[slot_index].in_free_ring
                &&& !self.slots@[slot_index].in_reclaim_ring
                &&& self.free_head == before.free_head
                &&& self.free_len == before.free_len + 1
                &&& self.free_ring@[free_tail] == slot_index
                &&& (forall|ring_index: int| 0 <= ring_index < C && ring_index != free_tail ==>
                    #[trigger] self.free_ring@[ring_index] == before.free_ring@[ring_index])
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
        }
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
                &&& self.slots@[slot_index as int].phase == slot.phase
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
        ensures count == self.pending_batch_member_count_spec(),
    {
        reveal(Scheduler::basic_invariant);
        reveal(Scheduler::scalar_invariant);
        reveal(Scheduler::pending_batch_member_count_spec);
        if self.batch_len == 0 {
            0
        } else {
            self.batch_ring[self.batch_head].member_count
        }
    }

    pub(crate) fn pending_member(&self, offset: usize) -> (member: Option<RequestId>)
        requires self.basic_invariant(),
        ensures member == self.pending_member_spec(offset),
    {
        reveal(Scheduler::basic_invariant);
        reveal(Scheduler::scalar_invariant);
        reveal(Scheduler::batch_ring_invariant);
        reveal(Scheduler::pending_member_spec);
        if self.batch_len == 0 || offset >= self.batch_ring[self.batch_head].member_count {
            None
        } else {
            let position = ring_tail::<C>(self.member_head, offset);
            Some(self.member_ring[position])
        }
    }

    /// Admits one request from the O(1) free ring.
    pub fn admit(&mut self) -> (result: Result<RequestId, SchedulerError>)
        requires old(self).basic_invariant(),
        ensures
            final(self).basic_invariant(),
            final(self).admit_refines(old(self), &result),
    {
        reveal(Scheduler::basic_invariant);
        reveal(Scheduler::scalar_invariant);
        reveal(Scheduler::slot_invariant);
        reveal(Scheduler::free_ring_invariant);
        reveal(Scheduler::reclaim_ring_invariant);
        reveal(Scheduler::member_ring_invariant);
        reveal(Scheduler::batch_ring_invariant);
        reveal(Scheduler::same_scalars);
        reveal(Scheduler::admit_refines);
        reveal(Scheduler::slot_model);
        reveal(Scheduler::slots_frame_except);
        if self.free_len == 0 {
            return Err(SchedulerError::OutOfSlots);
        }
        let ring_index = self.free_head;
        let slot_index = self.free_ring[ring_index];
        assert(slot_index < C);
        let slot = self.slots[slot_index];
        assert(slot.state == RequestState::Vacant);
        assert(slot.in_free_ring);

        let _old_free_head = self.free_head;
        let _old_free_len = self.free_len;
        let ghost old_slots = self.slots@;
        let ghost old_free_ring = self.free_ring@;
        assert forall|offset: int| 0 <= offset < _old_free_len - 1 implies {
            let shifted = offset + 1;
            let old_position = ring_position::<C>(_old_free_head, shifted as nat);
            let entry = old_free_ring[old_position];
            &&& 0 <= old_position < C
            &&& (#[trigger] old_free_ring[
                ring_position::<C>(_old_free_head, (offset + 1) as nat)
            ]) < C
            &&& entry != slot_index
            &&& old_slots[entry as int].state == RequestState::Vacant
            &&& old_slots[entry as int].in_free_ring
        } by {
            let shifted = offset + 1;
            let old_position = ring_position::<C>(_old_free_head, shifted as nat);
            ring_position_bounds::<C>(_old_free_head, shifted as nat);
            assert(self.free_ring@[old_position] == old_free_ring[old_position]);
            assert(self.free_ring@[
                ring_position::<C>(_old_free_head, shifted as nat)
            ] == old_free_ring[old_position]);
            assert(usize_ring_entries_differ::<C>(
                old_free_ring,
                _old_free_head,
                0,
                shifted,
            ));
            assert(ring_position::<C>(_old_free_head, 0) == _old_free_head);
            assert(old_free_ring[_old_free_head as int] == slot_index);
        }
        self.free_head = advance::<C>(self.free_head);
        self.free_len -= 1;
        self.slots[slot_index].state = RequestState::Ready;
        self.slots[slot_index].in_free_ring = false;
        self.live_count += 1;
        assert forall|offset: int| 0 <= offset < self.free_len implies {
            let shifted = offset + 1;
            let new_position = ring_position::<C>(self.free_head, offset as nat);
            let old_position = ring_position::<C>(_old_free_head, shifted as nat);
            &&& new_position == old_position
            &&& (#[trigger] self.free_ring@[
                ring_position::<C>(self.free_head, offset as nat)
            ]) < C
            &&& self.slots@[self.free_ring@[new_position] as int].state
                == RequestState::Vacant
            &&& self.slots@[self.free_ring@[new_position] as int].in_free_ring
        } by {
            ring_position_after_pop::<C>(_old_free_head, offset as nat);
            ring_position_bounds::<C>(self.free_head, offset as nat);
            assert(offset + 1 < _old_free_len);
            assert(old_slots[slot_index as int].state == RequestState::Vacant);
            assert(old_free_ring[_old_free_head as int] == slot_index);
        }
        assert forall|left: int, right: int|
            0 <= left < self.free_len && 0 <= right < self.free_len && left != right implies
                usize_ring_entries_differ::<C>(self.free_ring@, self.free_head, left, right)
        by {
            ring_position_after_pop::<C>(_old_free_head, left as nat);
            ring_position_after_pop::<C>(_old_free_head, right as nat);
            assert(usize_ring_entries_differ::<C>(
                old_free_ring,
                _old_free_head,
                left + 1,
                right + 1,
            ));
        }
        proof {
            live_count_update_admit(
                old_slots,
                slot_index as int,
                self.slots@[slot_index as int],
                C as nat,
            );
        }
        assert(self.scalar_invariant());
        assert(self.slot_invariant());
        assert(self.free_ring_invariant());
        assert(self.reclaim_ring_invariant());
        assert(self.member_ring_invariant());
        assert(self.batch_ring_invariant());
        assert(self.basic_invariant());
        Ok(RequestId::new(slot_index as u32, slot.generation))
    }

    /// Retires a request. An in-flight request stays attached to its batch;
    /// a ready request enters the O(1) reclaim ring immediately.
    pub fn retire(&mut self, request: RequestId) -> (result: Result<(), SchedulerError>)
        requires old(self).basic_invariant(),
        ensures
            final(self).basic_invariant(),
            final(self).retire_refines(old(self), request, &result),
    {
        reveal(Scheduler::basic_invariant);
        reveal(Scheduler::scalar_invariant);
        reveal(Scheduler::slot_invariant);
        reveal(Scheduler::free_ring_invariant);
        reveal(Scheduler::reclaim_ring_invariant);
        reveal(Scheduler::member_ring_invariant);
        reveal(Scheduler::batch_ring_invariant);
        reveal(Scheduler::same_scalars);
        reveal(Scheduler::retire_refines);
        reveal(Scheduler::slot_model);
        reveal(Scheduler::slots_frame_except);
        let slot_index = request.slot() as usize;
        if slot_index >= C {
            return Err(SchedulerError::InvalidSlot);
        }
        let slot = self.slots[slot_index];
        if slot.generation != request.generation() {
            return Err(SchedulerError::StaleRequest);
        }
        let ghost lifecycle_old_slots = self.slots@;
        match slot.state {
            RequestState::Vacant => {
                assert(self.same_scalars(old(self)));
                assert(self.basic_invariant());
                Err(SchedulerError::RequestNotLive)
            }
            RequestState::Retiring => {
                assert(self.same_scalars(old(self)));
                assert(self.basic_invariant());
                Err(SchedulerError::AlreadyRetiring)
            }
            RequestState::InFlight => {
                self.slots[slot_index].state = RequestState::Retiring;
                if slot.phase == LifecyclePhase::Executing {
                    self.slots[slot_index].phase = LifecyclePhase::RetiringExecuting;
                } else {
                    self.slots[slot_index].phase = LifecyclePhase::RetiringQuiescent;
                }
                proof {
                    live_count_update_nonvacant(
                        lifecycle_old_slots,
                        slot_index as int,
                        self.slots@[slot_index as int],
                        C as nat,
                    );
                }
                assert(self.scalar_invariant());
                assert(self.slot_invariant());
                assert(self.free_ring_invariant());
                assert(self.reclaim_ring_invariant());
                assert(self.member_ring_invariant());
                assert(self.batch_ring_invariant());
                assert(self.basic_invariant());
                Ok(())
            }
            RequestState::Ready => {
                assert(self.reclaim_len < C);
                assert(!slot.in_reclaim_ring);
                let tail = ring_tail::<C>(self.reclaim_head, self.reclaim_len);
                let ghost old_reclaim_ring = self.reclaim_ring@;
                let _old_reclaim_len = self.reclaim_len;
                let _old_reclaim_head = self.reclaim_head;
                assert forall|offset: int| 0 <= offset < _old_reclaim_len implies {
                    let old_slot = #[trigger] old_reclaim_ring[
                        ring_position::<C>(_old_reclaim_head, offset as nat)
                    ];
                    old_slot != slot_index
                } by {
                    let old_slot = old_reclaim_ring[
                        ring_position::<C>(_old_reclaim_head, offset as nat)
                    ];
                    assert(self.slots@[old_slot as int].state == RequestState::Retiring);
                }
                self.reclaim_ring[tail] = slot_index;
                self.reclaim_len += 1;
                self.slots[slot_index].state = RequestState::Retiring;
                self.slots[slot_index].phase = LifecyclePhase::RetiringQuiescent;
                self.slots[slot_index].active_epoch = NO_EPOCH;
                self.slots[slot_index].in_reclaim_ring = true;
                proof {
                    live_count_update_nonvacant(
                        lifecycle_old_slots,
                        slot_index as int,
                        self.slots@[slot_index as int],
                        C as nat,
                    );
                }
                assert forall|left: int, right: int|
                    0 <= left < self.reclaim_len
                        && 0 <= right < self.reclaim_len
                        && left != right implies
                    usize_ring_entries_differ::<C>(
                        self.reclaim_ring@,
                        self.reclaim_head,
                        left,
                        right,
                    )
                by {
                    if left < _old_reclaim_len && right < _old_reclaim_len {
                        assert(usize_ring_entries_differ::<C>(
                            old_reclaim_ring,
                            _old_reclaim_head,
                            left,
                            right,
                        ));
                    }
                }
                assert(self.scalar_invariant());
                assert(self.slot_invariant());
                assert(self.free_ring_invariant());
                assert(self.reclaim_ring_invariant());
                assert(self.member_ring_invariant());
                assert(self.batch_ring_invariant());
                assert(self.basic_invariant());
                Ok(())
            }
        }
    }

    /// Performs one deterministic rotating scan and submits one compact batch.
    /// Selected handles are written to the prefix of `output`.
    pub fn dispatch_ready(
        &mut self,
        output: &mut [RequestId],
    ) -> (result: Result<Option<DispatchBatch>, SchedulerError>)
        requires old(self).basic_invariant(),
        ensures
            final(self).basic_invariant(),
            final(self).dispatch_refines(old(self), old(output)@, final(output)@, &result),
    {
        reveal(Scheduler::basic_invariant);
        reveal(Scheduler::scalar_invariant);
        reveal(Scheduler::slot_invariant);
        reveal(Scheduler::free_ring_invariant);
        reveal(Scheduler::reclaim_ring_invariant);
        reveal(Scheduler::member_ring_invariant);
        reveal(Scheduler::batch_ring_invariant);
        reveal(Scheduler::same_scalars);
        reveal(Scheduler::dispatch_refines);
        reveal(Scheduler::slot_model);
        reveal(ready_selection);
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
        let member_start = ring_tail::<C>(self.member_head, self.member_len);
        let mut member_tail = member_start;
        let mut slot_index = self.cursor;
        let mut scanned = 0;
        let mut selected = 0;

        while scanned < C && selected < limit
            invariant
                self.basic_invariant(),
                scanned <= C,
                selected <= scanned,
                selected <= limit,
                selected <= output.len(),
                slot_index < C,
                member_tail < C,
                self.member_len + selected <= C,
            decreases C - scanned,
        {
            let slot = self.slots[slot_index];
            if slot.state == RequestState::Ready {
                let handle = RequestId::new(slot_index as u32, slot.generation);
                output[selected] = handle;
                self.member_ring[member_tail] = handle;
                member_tail = advance::<C>(member_tail);
                self.slots[slot_index].state = RequestState::InFlight;
                self.slots[slot_index].phase = LifecyclePhase::Executing;
                self.slots[slot_index].active_epoch = next_epoch;
                selected += 1;
            }
            slot_index = advance::<C>(slot_index);
            scanned += 1;
        }

        if selected == 0 {
            return Ok(None);
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

        Ok(Some(DispatchBatch {
            epoch: CompletionEpoch { value: next_epoch },
            member_count: selected,
        }))
    }

    /// Consumes exact quiescence evidence and emits one linear KV permit per
    /// member into caller-owned fixed storage.
    ///
    /// Normal members remain `InFlight`/`AwaitingKv`; cancelled members remain
    /// `Retiring`/`RetiringQuiescent`. Neither is dispatchable until the cache
    /// consumes its permit and returns exact finalized or detached evidence.
    pub(crate) fn complete_exact(
        &mut self,
        completion: ExactCompletion,
        permits: &mut [Option<KvQuiescencePermit>],
    ) -> (result: Result<usize, SchedulerError>)
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
    {
        reveal(Scheduler::basic_invariant);
        reveal(Scheduler::scalar_invariant);
        reveal(Scheduler::slot_invariant);
        reveal(Scheduler::free_ring_invariant);
        reveal(Scheduler::reclaim_ring_invariant);
        reveal(Scheduler::member_ring_invariant);
        reveal(Scheduler::batch_ring_invariant);
        reveal(Scheduler::same_scalars);
        reveal(Scheduler::completion_refines);
        reveal(Scheduler::slot_model);
        if self.batch_len == 0 {
            return Err(SchedulerError::NoPendingBatch);
        }
        let expected = match self.completed.checked_add(1) {
            Some(epoch) => epoch,
            None => return Err(SchedulerError::CompletionNotExactNext),
        };
        let observed = completion.epoch().value;
        if observed != expected {
            return Err(SchedulerError::CompletionNotExactNext);
        }
        let batch = self.batch_ring[self.batch_head];
        if batch.epoch.value != observed || batch.member_count == 0 {
            return Err(SchedulerError::CompletionEpochMismatch);
        }
        if permits.len() < batch.member_count {
            return Err(SchedulerError::CompletionStorageTooSmall);
        }

        // Preflight the complete compact member range before any mutation so
        // every error return frames the scheduler exactly.
        let mut checked = 0;
        let mut check_head = self.member_head;
        while checked < batch.member_count
            invariant
                checked <= batch.member_count,
                checked <= self.member_len,
                check_head < C,
                batch.member_count <= permits.len(),
            decreases batch.member_count - checked,
        {
            if permits[checked].is_some() {
                return Err(SchedulerError::CompletionStorageNotEmpty);
            }
            let handle = self.member_ring[check_head];
            if handle.slot() as usize >= C {
                return Err(SchedulerError::InvariantViolation);
            }
            let slot = self.slots[handle.slot() as usize];
            if slot.generation != handle.generation() || slot.active_epoch != observed {
                return Err(SchedulerError::InvariantViolation);
            }
            let valid_phase = (slot.state == RequestState::InFlight
                && slot.phase == LifecyclePhase::Executing)
                || (slot.state == RequestState::Retiring
                    && slot.phase == LifecyclePhase::RetiringExecuting);
            if !valid_phase {
                return Err(SchedulerError::InvariantViolation);
            }
            check_head = advance::<C>(check_head);
            checked += 1;
        }

        let mut processed = 0;
        let mut member_head = self.member_head;
        while processed < batch.member_count
            invariant
                processed <= batch.member_count,
                processed <= self.member_len,
                batch.member_count <= permits.len(),
                member_head < C,
            decreases batch.member_count - processed,
        {
            let handle = self.member_ring[member_head];
            let handle_slot = handle.slot() as usize;
            let slot = self.slots[handle_slot];
            if slot.state == RequestState::InFlight {
                self.slots[handle_slot].phase = LifecyclePhase::AwaitingKv;
            } else {
                self.slots[handle_slot].phase = LifecyclePhase::RetiringQuiescent;
            }
            permits[processed] = Some(KvQuiescencePermit {
                request: handle,
                origin: KvQuiescenceOrigin::CompletedExact { epoch: observed },
            });
            member_head = advance::<C>(member_head);
            processed += 1;
        }

        self.member_head = member_head;
        self.member_len -= batch.member_count;
        self.batch_head = advance::<C>(self.batch_head);
        self.batch_len -= 1;
        self.completed = observed;
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
    {
        reveal(Scheduler::basic_invariant);
        reveal(Scheduler::scalar_invariant);
        reveal(Scheduler::slot_invariant);
        reveal(Scheduler::free_ring_invariant);
        reveal(Scheduler::reclaim_ring_invariant);
        reveal(Scheduler::member_ring_invariant);
        reveal(Scheduler::batch_ring_invariant);
        reveal(Scheduler::same_scalars);
        reveal(Scheduler::retiring_permit_refines);
        reveal(Scheduler::slots_frame_except);
        if self.reclaim_len == 0 {
            return Ok(None);
        }
        let reclaim_index = self.reclaim_head;
        let slot_index = self.reclaim_ring[reclaim_index];
        if slot_index >= C {
            return Err(SchedulerError::InvariantViolation);
        }
        let slot = self.slots[slot_index];
        if slot.state != RequestState::Retiring
            || slot.active_epoch != NO_EPOCH
            || !slot.in_reclaim_ring
        {
            return Err(SchedulerError::InvariantViolation);
        }
        self.reclaim_head = advance::<C>(self.reclaim_head);
        self.reclaim_len -= 1;
        self.slots[slot_index].in_reclaim_ring = false;

        Ok(Some(KvQuiescencePermit {
            request: RequestId::new(slot_index as u32, slot.generation),
            origin: if slot.last_quiescent_epoch == NO_EPOCH {
                KvQuiescenceOrigin::NeverSubmitted
            } else {
                KvQuiescenceOrigin::CompletedExact {
                    epoch: slot.last_quiescent_epoch,
                }
            },
        }))
    }

    /// Consumes cache-owned evidence for the exact completed speculative step
    /// before making the request dispatchable again.
    #[verifier::spinoff_prover]
    pub(crate) fn accept_finalized(
        &mut self,
        finalized: KvFinalizedRequest,
    ) -> (result: Result<(), SchedulerError>)
        requires old(self).basic_invariant(),
        ensures
            final(self).basic_invariant(),
            final(self).finalized_refines(old(self), &finalized, &result),
    {
        reveal(Scheduler::basic_invariant);
        reveal(Scheduler::scalar_invariant);
        reveal(Scheduler::slot_invariant);
        reveal(Scheduler::free_ring_invariant);
        reveal(Scheduler::reclaim_ring_invariant);
        reveal(Scheduler::member_ring_invariant);
        reveal(Scheduler::batch_ring_invariant);
        reveal(Scheduler::same_scalars);
        reveal(Scheduler::finalized_refines);
        reveal(Scheduler::slot_model);
        reveal(Scheduler::slots_frame_except);
        reveal(finalized_expected_error);
        let request = finalized.request();
        let epoch = match finalized.origin() {
            KvQuiescenceOrigin::CompletedExact { epoch } => epoch,
            KvQuiescenceOrigin::NeverSubmitted => {
                let result = Err(SchedulerError::FinalizationMismatch);
                assert(self.finalized_refines(old(self), &finalized, &result));
                return result;
            }
        };
        let slot_index = request.slot() as usize;
        if slot_index >= C {
            let result = Err(SchedulerError::InvalidSlot);
            assert(self.finalized_refines(old(self), &finalized, &result));
            return result;
        }
        let slot = self.slots[slot_index];
        if slot.generation != request.generation()
            || slot.state != RequestState::InFlight
            || slot.phase != LifecyclePhase::AwaitingKv
            || slot.active_epoch != epoch
        {
            let result = Err(SchedulerError::FinalizationMismatch);
            assert(self.finalized_refines(old(self), &finalized, &result));
            return result;
        }

        let ghost old_slots = self.slots@;
        self.slots[slot_index].state = RequestState::Ready;
        self.slots[slot_index].phase = LifecyclePhase::Idle;
        self.slots[slot_index].active_epoch = NO_EPOCH;
        self.slots[slot_index].last_quiescent_epoch = epoch;
        proof {
            live_count_update_nonvacant(
                old_slots,
                slot_index as int,
                self.slots@[slot_index as int],
                C as nat,
            );
        }
        assert forall|observed: int| 0 <= observed < C implies {
            let observed_slot = #[trigger] self.slots@[observed];
            match observed_slot.state {
                RequestState::Vacant => {
                    &&& observed_slot.active_epoch == NO_EPOCH
                    &&& observed_slot.last_quiescent_epoch == NO_EPOCH
                    &&& observed_slot.phase == LifecyclePhase::Idle
                    &&& observed_slot.in_free_ring
                    &&& !observed_slot.in_reclaim_ring
                }
                RequestState::Ready => {
                    &&& observed_slot.active_epoch == NO_EPOCH
                    &&& observed_slot.last_quiescent_epoch <= self.completed
                    &&& observed_slot.phase == LifecyclePhase::Idle
                    &&& !observed_slot.in_free_ring
                    &&& !observed_slot.in_reclaim_ring
                }
                RequestState::InFlight => {
                    &&& (observed_slot.phase == LifecyclePhase::Executing
                        || observed_slot.phase == LifecyclePhase::AwaitingKv)
                    &&& (observed_slot.phase == LifecyclePhase::Executing ==>
                        self.completed < observed_slot.active_epoch <= self.submitted)
                    &&& (observed_slot.phase == LifecyclePhase::AwaitingKv ==>
                        NO_EPOCH < observed_slot.active_epoch <= self.completed)
                    &&& !observed_slot.in_free_ring
                    &&& !observed_slot.in_reclaim_ring
                }
                RequestState::Retiring => {
                    &&& !observed_slot.in_free_ring
                    &&& (observed_slot.phase == LifecyclePhase::RetiringExecuting
                        || observed_slot.phase == LifecyclePhase::RetiringQuiescent)
                    &&& (observed_slot.phase == LifecyclePhase::RetiringExecuting ==>
                        self.completed < observed_slot.active_epoch <= self.submitted)
                    &&& (observed_slot.phase == LifecyclePhase::RetiringQuiescent ==>
                        observed_slot.active_epoch <= self.completed)
                    &&& observed_slot.last_quiescent_epoch <= self.completed
                }
            }
        } by {
            if observed != slot_index {
                assert(self.slots@[observed] == old(self).slots@[observed]);
            }
        }
        assert(self.scalar_invariant());
        assert(self.slot_invariant());
        assert(self.free_ring_invariant());
        assert(self.reclaim_ring_invariant());
        assert(self.member_ring_invariant());
        assert(self.batch_ring_invariant());
        assert(self.basic_invariant());
        let result = Ok(());
        assert(self.finalized_refines(old(self), &finalized, &result));
        result
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
    {
        reveal(Scheduler::basic_invariant);
        reveal(Scheduler::scalar_invariant);
        reveal(Scheduler::slot_invariant);
        reveal(Scheduler::free_ring_invariant);
        reveal(Scheduler::reclaim_ring_invariant);
        reveal(Scheduler::member_ring_invariant);
        reveal(Scheduler::batch_ring_invariant);
        reveal(Scheduler::same_scalars);
        reveal(Scheduler::detached_refines);
        reveal(Scheduler::slot_model);
        reveal(Scheduler::slots_frame_except);
        reveal(detached_expected_error);
        let request = detached.request();
        let slot_index = request.slot() as usize;
        if slot_index >= C {
            return Err(SchedulerError::InvalidSlot);
        }
        let slot = self.slots[slot_index];
        let origin_matches = match detached.origin() {
            KvQuiescenceOrigin::NeverSubmitted => {
                slot.active_epoch == NO_EPOCH && slot.last_quiescent_epoch == NO_EPOCH
            }
            KvQuiescenceOrigin::CompletedExact { epoch } => {
                (slot.active_epoch != NO_EPOCH && slot.active_epoch == epoch)
                    || (slot.active_epoch == NO_EPOCH && slot.last_quiescent_epoch == epoch)
            }
        };
        if slot.generation != request.generation()
            || slot.state != RequestState::Retiring
            || slot.phase != LifecyclePhase::RetiringQuiescent
            || slot.in_reclaim_ring
            || !origin_matches
        {
            return Err(SchedulerError::DetachmentMismatch);
        }
        let next_generation = match slot.generation.checked_add(1) {
            Some(generation) => generation,
            None => return Err(SchedulerError::GenerationExhausted),
        };
        if self.free_len == C {
            return Err(SchedulerError::InvariantViolation);
        }
        let free_tail = ring_tail::<C>(self.free_head, self.free_len);
        self.free_ring[free_tail] = slot_index;
        self.free_len += 1;
        self.slots[slot_index] = Slot {
            generation: next_generation,
            state: RequestState::Vacant,
            phase: LifecyclePhase::Idle,
            active_epoch: NO_EPOCH,
            last_quiescent_epoch: NO_EPOCH,
            in_free_ring: true,
            in_reclaim_ring: false,
        };
        self.live_count -= 1;
        Ok(request)
    }

    #[must_use]
    pub fn state(&self, request: RequestId) -> (state: Option<RequestState>)
        ensures state == self.state_spec(request),
    {
        reveal(Scheduler::state_spec);
        if request.slot() as usize >= C {
            return None;
        }
        let slot = self.slots[request.slot() as usize];
        if slot.generation == request.generation() && slot.state != RequestState::Vacant {
            Some(slot.state)
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
    let request = finalized.request_spec();
    let slot_index = request.slot_spec() as int;
    if slot_index >= C {
        Some(SchedulerError::InvalidSlot)
    } else {
        let slot = scheduler.slots@[slot_index];
        match finalized.origin_spec() {
            KvQuiescenceOrigin::CompletedExact { epoch } => {
                if slot.generation == request.generation_spec()
                    && slot.state == RequestState::InFlight
                    && slot.phase == LifecyclePhase::AwaitingKv
                    && slot.active_epoch == epoch
                {
                    None
                } else {
                    Some(SchedulerError::FinalizationMismatch)
                }
            }
            KvQuiescenceOrigin::NeverSubmitted => Some(SchedulerError::FinalizationMismatch),
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
                (slot.active_epoch != NO_EPOCH && slot.active_epoch == epoch)
                    || (slot.active_epoch == NO_EPOCH && slot.last_quiescent_epoch == epoch)
            }
        };
        if slot.generation != request.generation_spec()
            || slot.state != RequestState::Retiring
            || slot.phase != LifecyclePhase::RetiringQuiescent
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

proof fn ring_position_bounds<const C: usize>(head: usize, offset: nat)
    requires C > 0, head < C, offset < C,
    ensures 0 <= ring_position::<C>(head, offset) < C,
{
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
        if slots[cursor as int].state == RequestState::Ready
            && slots[cursor as int].phase == LifecyclePhase::Idle
        {
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
        if slots[cursor as int].state == RequestState::Ready
            && slots[cursor as int].phase == LifecyclePhase::Idle
        {
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

spec fn ring_advance<const C: usize>(head: usize, steps: nat) -> usize
    recommends C > 0, head < C,
    decreases steps,
{
    if steps == 0 {
        head
    } else {
        ring_advance::<C>(next_position::<C>(head), (steps - 1) as nat)
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

proof fn live_count_all_vacant(slots: Seq<Slot>, prefix: nat)
    requires
        prefix <= slots.len(),
        forall|index: int| 0 <= index < prefix ==>
            (#[trigger] slots[index]).state == RequestState::Vacant,
    ensures live_slot_count(slots, prefix) == 0,
    decreases prefix,
{
    if prefix > 0 {
        live_count_all_vacant(slots, (prefix - 1) as nat);
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
    if prefix > 0 && slot_index < prefix - 1 {
        live_count_update_nonvacant(slots, slot_index, replacement, (prefix - 1) as nat);
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
    if prefix > 0 && slot_index < prefix - 1 {
        live_count_update_admit(slots, slot_index, replacement, (prefix - 1) as nat);
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
    if prefix > 0 && slot_index < prefix - 1 {
        live_count_update_reclaim(slots, slot_index, replacement, (prefix - 1) as nat);
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
mod tests {
    use super::{KvQuiescencePermit, Scheduler, SchedulerError};
    use crate::cache::KvPool;
    use crate::epoch::ExactCompletion;
    use ferric_spec::scheduling::RequestState;
    use ferric_spec::RequestId;

    fn output<const N: usize>() -> [RequestId; N] {
        [RequestId::new(u32::MAX, 0); N]
    }

    fn permits<const N: usize>() -> [Option<KvQuiescencePermit>; N] {
        std::array::from_fn(|_| None)
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
        assert_eq!(
            scheduler.complete_exact(
                ExactCompletion::from_contracted_hsa_quiescence(skipped),
                &mut authorities,
            ),
            Err(SchedulerError::CompletionNotExactNext)
        );
        assert_eq!(scheduler.state(request), Some(RequestState::InFlight));
        scheduler
            .complete_exact(
                ExactCompletion::from_contracted_hsa_quiescence(batch.epoch()),
                &mut authorities,
            )
            .unwrap();
        assert_eq!(scheduler.state(request), Some(RequestState::InFlight));
        let mut replay_storage = permits::<1>();
        assert_eq!(
            scheduler.complete_exact(
                ExactCompletion::from_contracted_hsa_quiescence(batch.epoch()),
                &mut replay_storage,
            ),
            Err(SchedulerError::NoPendingBatch)
        );
    }

    #[test]
    fn ready_retirement_is_immediately_reclaimable() {
        let mut scheduler = Scheduler::<1>::new().unwrap();
        let request = scheduler.admit().unwrap();
        scheduler.retire(request).unwrap();
        assert_eq!(scheduler.state(request), Some(RequestState::Retiring));
        let permit = scheduler.take_retiring_permit().unwrap().unwrap();
        assert_eq!(permit.request(), request);
        assert_eq!(scheduler.live_count(), 1);
    }
}
