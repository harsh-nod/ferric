//! Logical sequence bounds for the isolated engineering tensor-parallel driver.
//!
//! These contracts concern host state only. They do not authenticate completion,
//! GPU ownership, numerical results, or initialization of device memory.

use vstd::prelude::*;

verus! {

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TensorParallelSequenceErrorV1 {
    InvalidCapacity,
    InvalidToken,
    Busy,
    Exhausted,
    Poisoned,
    NotRunning,
    EpochOverflow,
}

#[derive(Debug)]
pub struct TensorParallelSequenceV1 {
    capacity: u32,
    vocabulary: u32,
    position: u32,
    epoch: u64,
    running: bool,
    poisoned: bool,
}

impl TensorParallelSequenceV1 {
    pub closed spec fn valid(&self) -> bool {
        0 < self.capacity <= 8192
            && self.vocabulary > 0
            && self.position <= self.capacity
            && (self.running ==> self.position < self.capacity)
    }

    pub closed spec fn view(&self) -> (u32, u32, u32, u64, bool, bool) {
        (self.capacity, self.vocabulary, self.position, self.epoch,
            self.running, self.poisoned)
    }

    pub fn new(capacity: u32, vocabulary: u32)
        -> (result: Result<Self, TensorParallelSequenceErrorV1>)
        ensures match result {
            Ok(state) => state.valid()
                && state.view() == (capacity, vocabulary, 0u32, 0u64, false, false),
            Err(_) => capacity == 0 || capacity > 8192 || vocabulary == 0,
        },
    {
        if capacity == 0 || capacity > 8192 || vocabulary == 0 {
            return Err(TensorParallelSequenceErrorV1::InvalidCapacity);
        }
        Ok(Self { capacity, vocabulary, position: 0, epoch: 0,
            running: false, poisoned: false })
    }

    pub fn position(&self) -> (position: u32)
        ensures position == self.view().2,
    { self.position }

    pub fn epoch(&self) -> (epoch: u64)
        ensures epoch == self.view().3,
    { self.epoch }

    pub fn begin(&mut self, token: u32)
        -> (result: Result<u32, TensorParallelSequenceErrorV1>)
        requires old(self).valid(),
        ensures final(self).valid(), match result {
            Ok(position) => position == old(self).view().2
                && !old(self).view().4 && !old(self).view().5
                && position < final(self).view().0 && token < final(self).view().1
                && final(self).view() == (old(self).view().0, old(self).view().1,
                    old(self).view().2, old(self).view().3, true, false),
            Err(_) => final(self).view() == old(self).view(),
        },
    {
        if self.poisoned { return Err(TensorParallelSequenceErrorV1::Poisoned); }
        if self.running { return Err(TensorParallelSequenceErrorV1::Busy); }
        if token >= self.vocabulary { return Err(TensorParallelSequenceErrorV1::InvalidToken); }
        if self.position >= self.capacity { return Err(TensorParallelSequenceErrorV1::Exhausted); }
        self.running = true;
        Ok(self.position)
    }

    pub fn complete(&mut self) -> (result: Result<(), TensorParallelSequenceErrorV1>)
        requires old(self).valid(),
        ensures final(self).valid(), match result {
            Ok(()) => old(self).view().4 && !old(self).view().5
                && final(self).view().2 == old(self).view().2 + 1
                && final(self).view() == (old(self).view().0, old(self).view().1,
                final(self).view().2, old(self).view().3, false, false),
            Err(_) => final(self).view() == old(self).view(),
        },
    {
        if self.poisoned { return Err(TensorParallelSequenceErrorV1::Poisoned); }
        if !self.running { return Err(TensorParallelSequenceErrorV1::NotRunning); }
        self.position += 1;
        self.running = false;
        Ok(())
    }

    pub fn poison(&mut self)
        requires old(self).valid(),
        ensures final(self).valid(), final(self).view() == (old(self).view().0, old(self).view().1,
            old(self).view().2, old(self).view().3, old(self).view().4, true),
    { self.poisoned = true; }

    pub fn reset(&mut self) -> (result: Result<(), TensorParallelSequenceErrorV1>)
        requires old(self).valid(),
        ensures final(self).valid(), match result {
            Ok(()) => !old(self).view().4 && !old(self).view().5
                && final(self).view().3 == old(self).view().3 + 1
                && final(self).view() == (old(self).view().0, old(self).view().1,
                0u32, final(self).view().3, false, false),
            Err(_) => final(self).view() == old(self).view(),
        },
    {
        if self.poisoned { return Err(TensorParallelSequenceErrorV1::Poisoned); }
        if self.running { return Err(TensorParallelSequenceErrorV1::Busy); }
        if self.epoch == u64::MAX { return Err(TensorParallelSequenceErrorV1::EpochOverflow); }
        self.epoch += 1;
        self.position = 0;
        Ok(())
    }
}
}

#[cfg(test)]
mod tests;
