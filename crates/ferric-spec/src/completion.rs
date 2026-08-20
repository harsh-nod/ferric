//! Sequential completion and retirement semantics.

use vstd::prelude::*;

verus! {

/// Monotonic identity assigned to one submitted batch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CompletionEpoch {
    pub value: u64,
}

impl CompletionEpoch {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self { value }
    }

    #[must_use]
    pub const fn value(self) -> u64 {
        self.value
    }
}

/// Rejection from the sequential ordered-completion oracle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionOrderError {
    Exhausted,
    NotExactNext,
}

/// Checks the single-authority, exact-next completion rule.
///
/// # Errors
///
/// Returns [`CompletionOrderError::Exhausted`] when no successor epoch exists,
/// or [`CompletionOrderError::NotExactNext`] when `observed` is not that successor.
pub fn check_exact_next(
    completed: CompletionEpoch,
    observed: CompletionEpoch,
) -> (result: Result<CompletionEpoch, CompletionOrderError>)
    ensures
        result == exact_next(completed, observed),
{
    match completed.value.checked_add(1) {
        Some(next) if next == observed.value => Ok(observed),
        Some(_) => Err(CompletionOrderError::NotExactNext),
        None => Err(CompletionOrderError::Exhausted),
    }
}

/// Mathematical form of [`check_exact_next`].
pub open spec fn exact_next(
    completed: CompletionEpoch,
    observed: CompletionEpoch,
) -> Result<CompletionEpoch, CompletionOrderError> {
    if completed.value == u64::MAX {
        Err(CompletionOrderError::Exhausted)
    } else if observed.value == completed.value + 1 {
        Ok(observed)
    } else {
        Err(CompletionOrderError::NotExactNext)
    }
}

pub proof fn exact_next_is_unique(
    completed: CompletionEpoch,
    first: CompletionEpoch,
    second: CompletionEpoch,
)
    requires
        exact_next(completed, first).is_ok(),
        exact_next(completed, second).is_ok(),
    ensures
        first == second,
{
}

}

#[cfg(test)]
mod tests {
    use super::{check_exact_next, CompletionEpoch, CompletionOrderError};

    #[test]
    fn rejects_skipped_replayed_and_exhausted_epochs() {
        let zero = CompletionEpoch::new(0);
        assert_eq!(
            check_exact_next(zero, CompletionEpoch::new(1)),
            Ok(CompletionEpoch::new(1))
        );
        assert_eq!(
            check_exact_next(zero, CompletionEpoch::new(2)),
            Err(CompletionOrderError::NotExactNext)
        );
        assert_eq!(
            check_exact_next(
                CompletionEpoch::new(u64::MAX),
                CompletionEpoch::new(u64::MAX)
            ),
            Err(CompletionOrderError::Exhausted)
        );
    }
}
