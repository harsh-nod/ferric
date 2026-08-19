//! Fixed-capacity request slots and deterministic scheduling.

use crate::epoch::ExactCompletion;
use ferric_spec::completion::CompletionEpoch;
use ferric_spec::scheduling::{LifecyclePhase, RequestState};
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
    pub const fn epoch(self) -> CompletionEpoch {
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SchedulerError {
    ZeroCapacity,
    CapacityExceedsRequestId,
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
                Ok(scheduler) => scheduler.basic_invariant(),
                Err(_) => true,
            },
    {
        if C == 0 {
            return Err(SchedulerError::ZeroCapacity);
        }
        if C > u32::MAX as usize {
            return Err(SchedulerError::CapacityExceedsRequestId);
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

        Ok(Self {
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
        })
    }

    pub closed spec fn basic_invariant(&self) -> bool {
        &&& C > 0
        &&& C <= u32::MAX as usize
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
        &&& self.free_len + self.live_count == C
        &&& self.reclaim_len <= self.live_count
        &&& self.member_len <= self.live_count
        &&& self.batch_len <= self.member_len
        &&& self.completed <= self.submitted
        &&& (forall|slot_index: int| 0 <= slot_index < C ==> {
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
        })
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

    #[must_use]
    pub const fn capacity(&self) -> usize {
        C
    }

    #[must_use]
    pub const fn live_count(&self) -> usize {
        self.live_count
    }

    #[must_use]
    pub const fn completed_epoch(&self) -> CompletionEpoch {
        CompletionEpoch { value: self.completed }
    }

    /// Admits one request from the O(1) free ring.
    pub fn admit(&mut self) -> (result: Result<RequestId, SchedulerError>)
        requires old(self).basic_invariant(),
        ensures
            final(self).basic_invariant(),
            result.is_err() ==> final(self).same_scalars(old(self)),
            match result {
                Ok(handle) => handle.slot_spec() < C,
                Err(_) => true,
            },
    {
        reveal(Scheduler::basic_invariant);
        reveal(Scheduler::same_scalars);
        if self.free_len == 0 {
            return Err(SchedulerError::OutOfSlots);
        }
        let ring_index = self.free_head;
        let slot_index = self.free_ring[ring_index];
        if slot_index >= C {
            return Err(SchedulerError::InvariantViolation);
        }
        let slot = self.slots[slot_index];
        if slot.state != RequestState::Vacant || !slot.in_free_ring {
            return Err(SchedulerError::InvariantViolation);
        }

        self.free_head = advance::<C>(self.free_head);
        self.free_len -= 1;
        self.slots[slot_index].state = RequestState::Ready;
        self.slots[slot_index].in_free_ring = false;
        self.live_count += 1;
        Ok(RequestId::new(slot_index as u32, slot.generation))
    }

    /// Retires a request. An in-flight request stays attached to its batch;
    /// a ready request enters the O(1) reclaim ring immediately.
    pub fn retire(&mut self, request: RequestId) -> (result: Result<(), SchedulerError>)
        requires old(self).basic_invariant(),
        ensures
            final(self).basic_invariant(),
            result.is_err() ==> final(self).same_scalars(old(self)),
    {
        reveal(Scheduler::basic_invariant);
        reveal(Scheduler::same_scalars);
        let slot_index = request.slot() as usize;
        if slot_index >= C {
            return Err(SchedulerError::InvalidSlot);
        }
        let slot = self.slots[slot_index];
        if slot.generation != request.generation() {
            return Err(SchedulerError::StaleRequest);
        }
        match slot.state {
            RequestState::Vacant => Err(SchedulerError::RequestNotLive),
            RequestState::Retiring => Err(SchedulerError::AlreadyRetiring),
            RequestState::InFlight => {
                self.slots[slot_index].state = RequestState::Retiring;
                if slot.phase == LifecyclePhase::Executing {
                    self.slots[slot_index].phase = LifecyclePhase::RetiringExecuting;
                } else {
                    self.slots[slot_index].phase = LifecyclePhase::RetiringQuiescent;
                }
                Ok(())
            }
            RequestState::Ready => {
                if self.reclaim_len == C || slot.in_reclaim_ring {
                    return Err(SchedulerError::InvariantViolation);
                }
                let tail = ring_tail::<C>(self.reclaim_head, self.reclaim_len);
                self.reclaim_ring[tail] = slot_index;
                self.reclaim_len += 1;
                self.slots[slot_index].state = RequestState::Retiring;
                self.slots[slot_index].phase = LifecyclePhase::RetiringQuiescent;
                self.slots[slot_index].active_epoch = slot.last_quiescent_epoch;
                self.slots[slot_index].in_reclaim_ring = true;
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
            result.is_err() ==> final(self).same_scalars(old(self)),
            match result {
                Ok(Some(batch)) => batch.member_count_spec() <= final(output).len(),
                _ => true,
            },
    {
        reveal(Scheduler::basic_invariant);
        reveal(Scheduler::same_scalars);
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
            result.is_err() ==> final(self).same_scalars(old(self)),
            match result {
                Ok(count) => count <= final(permits).len(),
                Err(_) => true,
            },
    {
        reveal(Scheduler::basic_invariant);
        reveal(Scheduler::same_scalars);
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
            result.is_err() ==> final(self).same_scalars(old(self)),
    {
        reveal(Scheduler::basic_invariant);
        reveal(Scheduler::same_scalars);
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

    #[must_use]
    pub fn state(&self, request: RequestId) -> Option<RequestState> {
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
}

fn advance<const C: usize>(index: usize) -> (next: usize)
    requires C > 0, index < C,
    ensures next < C,
{
    if index + 1 == C {
        0
    } else {
        index + 1
    }
}

fn ring_tail<const C: usize>(head: usize, len: usize) -> (tail: usize)
    requires C > 0, head < C, len < C,
    ensures tail < C,
{
    let distance = C - head;
    if len < distance {
        head + len
    } else {
        len - distance
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

        let next = scheduler.dispatch_ready(&mut members).unwrap().unwrap();
        assert_eq!(members[0], third);
        assert_eq!(next.member_count(), 1);
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
