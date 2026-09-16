//! Dependency-free engineering model for the seven-task atomic micrograph.
//!
//! Each step models one scheduler linearization point, not a GPU clock tick.
//! LDS phases are collapsed into execution because all lanes follow the same
//! fixed phases. This explores host-side protocol interleavings; it is not a
//! Rust memory-model proof, Verus qualification, or GPU launch authority.

pub const TASKS: usize = 7;
pub const WORKERS: usize = 2;
pub const ATTEMPTS: u8 = 16;
pub const EMPTY_PROBES: u8 = 8;
pub const ALL_TASKS: u32 = 127;
pub const MAX_ROW_SUM: u32 = 128 * 1024;
pub const DEPENDENCIES: [u32; TASKS] = [0, 1, 1, 6, 8, 8, 48];

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum Phase {
    Ready,
    Claim(u8),
    Execute(u8),
    Complete(u8),
    Enqueue { task: u8, old_done: u32 },
    Retired,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Model {
    epoch: u32,
    expected_epoch: u32,
    row_sums: [u32; TASKS],
    ready: u32,
    done: u32,
    claimed: u32,
    owners: u32,
    errors: u32,
    payload: [Option<u32>; TASKS],
    executions: [u8; TASKS],
    attempts: [u8; WORKERS],
    empty_probes: [u8; WORKERS],
    empty_limit: u8,
    witnessed: [u32; WORKERS],
    phases: [Phase; WORKERS],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelError {
    InvalidInput,
    InvalidWorker,
    DuplicateTask,
    MissingDependency,
    MissingPayload,
    RepeatedCandidate,
    ArithmeticOverflow,
    Incomplete,
}

impl Model {
    /// Start a quiescent epoch. Zero/mismatched epochs deliberately reach the
    /// device-equivalent stale path instead of granting work to the caller.
    ///
    /// # Errors
    /// Rejects row sums outside the fixed numerical envelope.
    pub fn new(
        epoch: u32,
        expected_epoch: u32,
        row_sums: [u32; TASKS],
    ) -> Result<Self, ModelError> {
        if row_sums.iter().any(|&value| value > MAX_ROW_SUM) {
            return Err(ModelError::InvalidInput);
        }
        Ok(Self {
            epoch,
            expected_epoch,
            row_sums,
            ready: 1,
            done: 0,
            claimed: 0,
            owners: 0,
            errors: 0,
            payload: [None; TASKS],
            executions: [0; TASKS],
            attempts: [0; WORKERS],
            empty_probes: [0; WORKERS],
            empty_limit: EMPTY_PROBES,
            witnessed: [0; WORKERS],
            phases: [Phase::Ready; WORKERS],
        })
    }

    #[must_use]
    pub fn is_retired(&self, worker: usize) -> bool {
        self.phases.get(worker) == Some(&Phase::Retired)
    }

    #[must_use]
    pub fn is_terminal(&self) -> bool {
        self.phases.iter().all(|phase| *phase == Phase::Retired)
    }

    /// Advance exactly one worker's next atomic/logical publication point.
    ///
    /// # Errors
    /// Detects violated scheduler, visibility, numerical, or ownership invariants.
    pub fn step(&mut self, worker: usize) -> Result<(), ModelError> {
        self.advance(worker, false)
    }

    #[allow(clippy::too_many_lines)]
    fn advance(&mut self, worker: usize, use_late_done_read: bool) -> Result<(), ModelError> {
        let phase = *self.phases.get(worker).ok_or(ModelError::InvalidWorker)?;
        match phase {
            Phase::Ready => {
                if self.attempts[worker] == ATTEMPTS {
                    self.phases[worker] = Phase::Retired;
                    return Ok(());
                }
                self.attempts[worker] += 1;
                if self.expected_epoch == 0 || self.epoch != self.expected_epoch {
                    self.errors |= 1;
                    self.phases[worker] = Phase::Retired;
                } else if self.ready & !ALL_TASKS != 0 {
                    self.errors |= 4;
                    self.phases[worker] = Phase::Retired;
                } else if self.ready == 0 {
                    self.empty_probes[worker] += 1;
                    if self.empty_probes[worker] == self.empty_limit {
                        self.phases[worker] = Phase::Retired;
                    }
                } else {
                    let task = u8::try_from(self.ready.trailing_zeros())
                        .map_err(|_| ModelError::InvalidInput)?;
                    if usize::from(task) >= TASKS {
                        return Err(ModelError::InvalidInput);
                    }
                    let bit = 1 << task;
                    if self.witnessed[worker] & bit != 0 {
                        return Err(ModelError::RepeatedCandidate);
                    }
                    self.witnessed[worker] |= bit;
                    self.phases[worker] = Phase::Claim(task);
                }
            }
            Phase::Claim(task) => {
                let bit = 1 << task;
                let old_ready = self.ready;
                self.ready &= !bit;
                if old_ready & bit == 0 {
                    self.phases[worker] = Phase::Ready;
                } else {
                    if self.claimed & bit != 0 {
                        return Err(ModelError::DuplicateTask);
                    }
                    if self.done & DEPENDENCIES[usize::from(task)]
                        != DEPENDENCIES[usize::from(task)]
                    {
                        return Err(ModelError::MissingDependency);
                    }
                    self.claimed |= bit;
                    let owner = u32::try_from(worker + 1).map_err(|_| ModelError::InvalidWorker)?;
                    self.owners |= owner << (2 * task);
                    self.phases[worker] = Phase::Execute(task);
                }
            }
            Phase::Execute(task) => {
                let index = usize::from(task);
                let mut value = self.row_sums[index];
                for dependency in 0..TASKS {
                    if DEPENDENCIES[index] & (1 << dependency) != 0 {
                        value = value
                            .checked_add(
                                self.payload[dependency].ok_or(ModelError::MissingPayload)?,
                            )
                            .ok_or(ModelError::ArithmeticOverflow)?;
                    }
                }
                if self.executions[index] != 0 {
                    return Err(ModelError::DuplicateTask);
                }
                self.executions[index] += 1;
                self.payload[index] = Some(value);
                self.phases[worker] = Phase::Complete(task);
            }
            Phase::Complete(task) => {
                if self.payload[usize::from(task)].is_none() {
                    return Err(ModelError::MissingPayload);
                }
                let old_done = self.done;
                self.done |= 1 << task;
                self.phases[worker] = Phase::Enqueue { task, old_done };
            }
            Phase::Enqueue { task, old_done } => {
                // The fault mode is solely for a regression test proving that
                // replacing the RMW old value with a late read is unsound.
                let completion_snapshot = if use_late_done_read {
                    self.done
                } else {
                    old_done | (1 << task)
                };
                let children = match task {
                    0 => 6,
                    1 | 2 if completion_snapshot & 6 == 6 => 8,
                    3 => 48,
                    4 | 5 if completion_snapshot & 0b11_0000 == 0b11_0000 => 64,
                    _ => 0,
                };
                self.ready |= children;
                self.phases[worker] = Phase::Ready;
            }
            Phase::Retired => {}
        }
        Ok(())
    }

    /// Validate exact completion, values, packing, and bounded worker attempts.
    ///
    /// # Errors
    /// Rejects incomplete or stale executions and any numerical/ownership mismatch.
    pub fn validate_complete(&self) -> Result<[u32; TASKS], ModelError> {
        if !self.is_terminal()
            || self.ready != 0
            || self.done != ALL_TASKS
            || self.claimed != ALL_TASKS
            || self.errors != 0
            || self.executions != [1; TASKS]
            || self.attempts.iter().any(|&attempts| attempts > ATTEMPTS)
            || self.owners >> (2 * TASKS) != 0
        {
            return Err(ModelError::Incomplete);
        }
        let expected = reference(self.row_sums)?;
        for (task, &value) in expected.iter().enumerate() {
            if self.payload[task] != Some(value)
                || !matches!((self.owners >> (2 * task)) & 3, 1 | 2)
            {
                return Err(ModelError::Incomplete);
            }
        }
        Ok(expected)
    }
}

/// Independent scalar expression for both diamonds, with checked arithmetic.
///
/// # Errors
/// Rejects row sums outside the fixture envelope or arithmetic overflow.
pub fn reference(rows: [u32; TASKS]) -> Result<[u32; TASKS], ModelError> {
    if rows.iter().any(|&value| value > MAX_ROW_SUM) {
        return Err(ModelError::InvalidInput);
    }
    let add = |left: u32, right: u32| {
        left.checked_add(right)
            .ok_or(ModelError::ArithmeticOverflow)
    };
    let root = rows[0];
    let left = add(rows[1], root)?;
    let right = add(rows[2], root)?;
    let first_join = add(add(rows[3], left)?, right)?;
    let second_left = add(rows[4], first_join)?;
    let second_right = add(rows[5], first_join)?;
    let final_join = add(add(rows[6], second_left)?, second_right)?;
    Ok([
        root,
        left,
        right,
        first_join,
        second_left,
        second_right,
        final_join,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn step_n(model: &mut Model, worker: usize, count: usize) {
        for _ in 0..count {
            model.step(worker).unwrap();
        }
    }

    #[test]
    fn exhaustive_two_worker_interleavings_complete_both_diamonds() {
        let mut initial = Model::new(1, 1, [1, 2, 3, 4, 5, 6, 7]).unwrap();
        // Nonretiring empty probes change no shared state. This explores the
        // protocol quotient that removes those local scheduling stutters;
        // the concrete eight-probe accounting is tested separately below.
        // This is not a count of all concrete sixteen-round GPU schedules.
        initial.empty_limit = 1;
        let mut seen = HashSet::new();
        let mut pending = vec![initial];
        let mut terminals = 0;
        while let Some(state) = pending.pop() {
            if !seen.insert(state.clone()) {
                continue;
            }
            if state.is_terminal() {
                assert_eq!(
                    state.validate_complete().unwrap(),
                    [1, 3, 4, 11, 16, 17, 40]
                );
                terminals += 1;
                continue;
            }
            assert!(
                seen.len() < 1_000_000,
                "bounded state exploration unexpectedly expanded"
            );
            for worker in 0..WORKERS {
                if !state.is_retired(worker) {
                    let mut next = state.clone();
                    next.step(worker).unwrap();
                    pending.push(next);
                }
            }
        }
        assert!(terminals > 1);
        println!(
            "explored {} states and {terminals} terminal states",
            seen.len()
        );
    }

    #[test]
    fn one_resident_workgroup_never_waits_for_the_other() {
        for resident in 0..WORKERS {
            let mut model = Model::new(9, 9, [MAX_ROW_SUM; TASKS]).unwrap();
            while !model.is_retired(resident) {
                model.step(resident).unwrap();
            }
            assert_eq!(model.attempts[resident], 15);
            assert_eq!(model.done, ALL_TASKS);
            while !model.is_retired(1 - resident) {
                model.step(1 - resident).unwrap();
            }
            assert_eq!(model.validate_complete().unwrap()[6], 1_703_936);
        }
    }

    #[test]
    fn losing_a_claim_does_not_retire_an_active_worker() {
        let mut model = Model::new(1, 1, [0; TASKS]).unwrap();
        step_n(&mut model, 0, 1);
        step_n(&mut model, 1, 1);
        step_n(&mut model, 0, 1);
        step_n(&mut model, 1, 1);
        assert_eq!(model.phases[1], Phase::Ready);
        assert_eq!(model.witnessed[1], 1);
        while !model.is_terminal() {
            model.step(0).unwrap();
            model.step(1).unwrap();
        }
        model.validate_complete().unwrap();
    }

    #[test]
    fn stale_or_zero_epoch_never_mutates_queue_or_payload() {
        for (epoch, expected) in [(4, 5), (0, 0)] {
            let mut model = Model::new(epoch, expected, [0; TASKS]).unwrap();
            model.step(0).unwrap();
            model.step(1).unwrap();
            assert!(model.is_terminal());
            assert_eq!(
                (
                    model.ready,
                    model.done,
                    model.claimed,
                    model.owners,
                    model.errors
                ),
                (1, 0, 0, 0, 1)
            );
            assert_eq!(model.payload, [None; TASKS]);
            assert_eq!(model.validate_complete(), Err(ModelError::Incomplete));
        }
    }

    #[test]
    fn late_done_load_would_republish_a_claimed_join() {
        let mut model = Model::new(1, 1, [0; TASKS]).unwrap();
        step_n(&mut model, 0, 5); // Complete and enqueue root successors.
        step_n(&mut model, 0, 2); // Claim task1.
        step_n(&mut model, 1, 2); // Claim task2.
        step_n(&mut model, 0, 2); // Publish task1 payload and completion.
        step_n(&mut model, 1, 2); // Publish task2 payload and completion.
        let mut correct = model.clone();
        correct.step(0).unwrap();
        assert_eq!(correct.ready, 0); // Task1 was not the last predecessor.
        correct.step(1).unwrap();
        assert_eq!(correct.ready, 8);
        model.advance(0, true).unwrap(); // Incorrect late read publishes task3.
        step_n(&mut model, 0, 2); // Claim task3 before other predecessor enqueues.
        model.advance(1, true).unwrap(); // Incorrectly publishes task3 again.
        assert_eq!(model.ready & model.claimed, 8);
        model.step(1).unwrap();
        assert_eq!(model.step(1), Err(ModelError::DuplicateTask));
    }

    #[test]
    fn payload_before_done_is_required() {
        let mut model = Model::new(1, 1, [0; TASKS]).unwrap();
        model.phases[0] = Phase::Complete(0);
        assert_eq!(model.step(0), Err(ModelError::MissingPayload));
    }

    #[test]
    fn overflowing_atomic_payload_cannot_publish_completion() {
        let mut model = Model::new(1, 1, [1; TASKS]).unwrap();
        model.payload[0] = Some(u32::MAX);
        model.done = 1;
        model.claimed = 3;
        model.phases[0] = Phase::Execute(1);
        assert_eq!(model.step(0), Err(ModelError::ArithmeticOverflow));
        assert_eq!(model.done, 1);
        assert_eq!(model.payload[1], None);
        assert_eq!(model.executions[1], 0);
        assert_ne!(model.ready, 8);
    }

    #[test]
    fn malformed_inputs_and_premature_ready_are_rejected() {
        for ready in [128, 129] {
            let mut invalid_mask = Model::new(1, 1, [0; TASKS]).unwrap();
            invalid_mask.ready = ready;
            invalid_mask.step(0).unwrap();
            invalid_mask.step(1).unwrap();
            assert!(invalid_mask.is_terminal());
            assert_eq!((invalid_mask.ready, invalid_mask.errors), (ready, 4));
            assert_eq!(invalid_mask.claimed, 0);
            assert_eq!(invalid_mask.payload, [None; TASKS]);
        }
        assert_eq!(
            Model::new(1, 1, [MAX_ROW_SUM + 1; TASKS]),
            Err(ModelError::InvalidInput)
        );
        let mut model = Model::new(1, 1, [0; TASKS]).unwrap();
        model.ready = 8;
        model.step(0).unwrap();
        assert_eq!(model.step(0), Err(ModelError::MissingDependency));
        assert_eq!(model.step(WORKERS), Err(ModelError::InvalidWorker));
    }
}

#[test]
fn concrete_eight_probe_schedules_remain_bounded_and_complete() {
    let mut random = 0x1234_5678u64;
    for _ in 0..10_000 {
        let mut model = Model::new(2, 2, [MAX_ROW_SUM; TASKS]).unwrap();
        let mut steps = 0;
        while !model.is_terminal() {
            random = random
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            let worker = usize::from((random >> 63) != 0);
            if !model.is_retired(worker) {
                model.step(worker).unwrap();
                steps += 1;
            }
            assert!(steps <= 2 * (16 + 4 * TASKS));
        }
        model.validate_complete().unwrap();
        assert!(
            model
                .empty_probes
                .iter()
                .all(|&probes| probes <= EMPTY_PROBES)
        );
        assert!(model.attempts.iter().all(|&attempts| attempts <= 15));
        assert!(model.witnessed.iter().all(|mask| mask.count_ones() <= 7));
    }
}
