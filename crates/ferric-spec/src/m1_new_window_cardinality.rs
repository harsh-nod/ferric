//! Pure cardinality planning for an all-terminal serving-window transition.
//!
//! This module decides only how many retiring request slots are reincarnated,
//! reclaimed, or newly admitted. It owns no scheduler, KV, queue, or device
//! authority and performs no runtime mutation.

use vstd::prelude::*;

verus! {

/// Exact immutable operation counts for one variable-cardinality new window.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1NewWindowCardinalityPlanV1 {
    old: u32,
    new: u32,
    reuse: u32,
    reclaim: u32,
    admit: u32,
}

impl M1NewWindowCardinalityPlanV1 {
    pub closed spec fn old_count_spec(self) -> u32 {
        self.old
    }

    pub closed spec fn new_count_spec(self) -> u32 {
        self.new
    }

    pub closed spec fn reuse_count_spec(self) -> u32 {
        self.reuse
    }

    pub closed spec fn reclaim_count_spec(self) -> u32 {
        self.reclaim
    }

    pub closed spec fn admit_count_spec(self) -> u32 {
        self.admit
    }

    /// Number of terminal predecessor rows entering the transition.
    #[must_use]
    pub const fn old_count(self) -> (count: u32)
        ensures count == self.old_count_spec(),
    {
        self.old
    }

    /// Number of requested successor rows leaving the transition.
    #[must_use]
    pub const fn new_count(self) -> (count: u32)
        ensures count == self.new_count_spec(),
    {
        self.new
    }

    /// Number of predecessor slots reincarnated in place.
    #[must_use]
    pub const fn reuse_count(self) -> (count: u32)
        ensures count == self.reuse_count_spec(),
    {
        self.reuse
    }

    /// Number of unmatched predecessor slots returned to the free ring.
    #[must_use]
    pub const fn reclaim_count(self) -> (count: u32)
        ensures count == self.reclaim_count_spec(),
    {
        self.reclaim
    }

    /// Number of successor rows admitted from the free ring.
    #[must_use]
    pub const fn admit_count(self) -> (count: u32)
        ensures count == self.admit_count_spec(),
    {
        self.admit
    }
}

/// Read-only rejection from variable-cardinality new-window planning.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1NewWindowCardinalityPlanErrorV1 {
    EmptyPredecessor,
    EmptySuccessor,
    PredecessorExceedsCapacity,
    SuccessorExceedsCapacity,
    ArithmeticOverflow,
}

/// Mathematical result of the variable-cardinality planner.
pub closed spec fn m1_new_window_cardinality_plan_spec_v1(
    old_count: u32,
    new_count: u32,
    capacity: u32,
) -> Result<M1NewWindowCardinalityPlanV1, M1NewWindowCardinalityPlanErrorV1> {
    if old_count == 0 {
        Err(M1NewWindowCardinalityPlanErrorV1::EmptyPredecessor)
    } else if new_count == 0 {
        Err(M1NewWindowCardinalityPlanErrorV1::EmptySuccessor)
    } else if old_count > capacity {
        Err(M1NewWindowCardinalityPlanErrorV1::PredecessorExceedsCapacity)
    } else if new_count > capacity {
        Err(M1NewWindowCardinalityPlanErrorV1::SuccessorExceedsCapacity)
    } else {
        let reuse_count = if old_count <= new_count { old_count } else { new_count };
        Ok(M1NewWindowCardinalityPlanV1 {
            old: old_count,
            new: new_count,
            reuse: reuse_count,
            reclaim: (old_count - reuse_count) as u32,
            admit: (new_count - reuse_count) as u32,
        })
    }
}

/// Computes exact reuse, reclaim, and admission counts before runtime commit.
///
/// # Errors
///
/// Rejects empty windows, either cardinality above `capacity`, or any checked
/// arithmetic inconsistency. Rejection performs no mutation and retains no
/// external authority.
pub fn plan_m1_new_window_cardinality_v1(
    old_count: u32,
    new_count: u32,
    capacity: u32,
) -> (result: Result<M1NewWindowCardinalityPlanV1, M1NewWindowCardinalityPlanErrorV1>)
    ensures
        result == m1_new_window_cardinality_plan_spec_v1(old_count, new_count, capacity),
{
    if old_count == 0 {
        return Err(M1NewWindowCardinalityPlanErrorV1::EmptyPredecessor);
    }
    if new_count == 0 {
        return Err(M1NewWindowCardinalityPlanErrorV1::EmptySuccessor);
    }
    if old_count > capacity {
        return Err(M1NewWindowCardinalityPlanErrorV1::PredecessorExceedsCapacity);
    }
    if new_count > capacity {
        return Err(M1NewWindowCardinalityPlanErrorV1::SuccessorExceedsCapacity);
    }

    let reuse_count = if old_count <= new_count { old_count } else { new_count };
    let reclaim_count = match old_count.checked_sub(reuse_count) {
        Some(count) => count,
        None => return Err(M1NewWindowCardinalityPlanErrorV1::ArithmeticOverflow),
    };
    let admit_count = match new_count.checked_sub(reuse_count) {
        Some(count) => count,
        None => return Err(M1NewWindowCardinalityPlanErrorV1::ArithmeticOverflow),
    };
    if reuse_count.checked_add(reclaim_count) != Some(old_count)
        || reuse_count.checked_add(admit_count) != Some(new_count)
    {
        return Err(M1NewWindowCardinalityPlanErrorV1::ArithmeticOverflow);
    }
    let retained_count = match old_count.checked_sub(reclaim_count) {
        Some(count) => count,
        None => return Err(M1NewWindowCardinalityPlanErrorV1::ArithmeticOverflow),
    };
    if retained_count.checked_add(admit_count) != Some(new_count) {
        return Err(M1NewWindowCardinalityPlanErrorV1::ArithmeticOverflow);
    }

    Ok(M1NewWindowCardinalityPlanV1 {
        old: old_count,
        new: new_count,
        reuse: reuse_count,
        reclaim: reclaim_count,
        admit: admit_count,
    })
}

}

#[cfg(test)]
mod tests {
    use super::{plan_m1_new_window_cardinality_v1, M1NewWindowCardinalityPlanErrorV1};
    use crate::M1_MAX_ACTIVE_SEQUENCES;

    fn assert_plan(
        old_count: u32,
        new_count: u32,
        reuse_count: u32,
        reclaim_count: u32,
        admit_count: u32,
    ) {
        let plan = plan_m1_new_window_cardinality_v1(old_count, new_count, M1_MAX_ACTIVE_SEQUENCES)
            .expect("bounded nonempty window cardinalities must plan");
        assert_eq!(plan.old_count(), old_count);
        assert_eq!(plan.new_count(), new_count);
        assert_eq!(plan.reuse_count(), reuse_count);
        assert_eq!(plan.reclaim_count(), reclaim_count);
        assert_eq!(plan.admit_count(), admit_count);
        assert_eq!(plan.reuse_count() + plan.reclaim_count(), old_count);
        assert_eq!(plan.reuse_count() + plan.admit_count(), new_count);
        assert_eq!(
            old_count - plan.reclaim_count() + plan.admit_count(),
            new_count
        );
    }

    #[test]
    fn plans_maximum_shrink() {
        assert_plan(32, 1, 1, 31, 0);
    }

    #[test]
    fn plans_maximum_growth() {
        assert_plan(1, 32, 1, 0, 31);
    }

    #[test]
    fn plans_equal_odd_cardinality() {
        assert_plan(17, 17, 17, 0, 0);
    }

    #[test]
    fn plans_twenty_alternating_windows() {
        let mut old_count = 32;
        for window in 0..20 {
            let new_count = if window % 2 == 0 { 1 } else { 32 };
            let reuse_count = old_count.min(new_count);
            assert_plan(
                old_count,
                new_count,
                reuse_count,
                old_count - reuse_count,
                new_count - reuse_count,
            );
            old_count = new_count;
        }
        assert_eq!(old_count, 32);
    }

    #[test]
    fn rejects_empty_and_over_capacity_windows() {
        assert_eq!(
            plan_m1_new_window_cardinality_v1(0, 1, M1_MAX_ACTIVE_SEQUENCES),
            Err(M1NewWindowCardinalityPlanErrorV1::EmptyPredecessor),
        );
        assert_eq!(
            plan_m1_new_window_cardinality_v1(1, 0, M1_MAX_ACTIVE_SEQUENCES),
            Err(M1NewWindowCardinalityPlanErrorV1::EmptySuccessor),
        );
        assert_eq!(
            plan_m1_new_window_cardinality_v1(33, 1, M1_MAX_ACTIVE_SEQUENCES),
            Err(M1NewWindowCardinalityPlanErrorV1::PredecessorExceedsCapacity),
        );
        assert_eq!(
            plan_m1_new_window_cardinality_v1(1, 33, M1_MAX_ACTIVE_SEQUENCES),
            Err(M1NewWindowCardinalityPlanErrorV1::SuccessorExceedsCapacity),
        );
    }
}
