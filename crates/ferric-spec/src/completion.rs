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

    const ENGINE_SYSTEM_SOURCE: &str = include_str!("../../ferric-engine/src/system.rs");

    fn unique_offset(source: &str, needle: &str) -> usize {
        let mut matches = source.match_indices(needle);
        let Some((offset, _)) = matches.next() else {
            panic!("source-policy anchor is absent: {needle}");
        };
        assert!(
            matches.next().is_none(),
            "source-policy anchor is not unique: {needle}"
        );
        offset
    }

    /// Syntactic join evidence only; this does not prove engine state effects.
    #[test]
    fn source_policy_pins_exact_successor_check_in_immutable_preflight() {
        let preflight_start = unique_offset(
            ENGINE_SYSTEM_SOURCE,
            "pub(crate) fn preflight_complete_exact(",
        );
        let preflight_end =
            unique_offset(ENGINE_SYSTEM_SOURCE, "pub fn reject_reordered_completion(");
        let preflight = &ENGINE_SYSTEM_SOURCE[preflight_start..preflight_end];
        let count_guard = unique_offset(preflight, "if accepted_tokens.len() != member_count");
        let completed_epoch = unique_offset(
            preflight,
            "let completed_epoch = self.scheduler.completed_epoch();",
        );
        let exact_successor = unique_offset(
            preflight,
            "ferric_spec::completion::check_exact_next(completed_epoch, observed_epoch)",
        );
        let rejection = unique_offset(preflight, "Err(_) => {");
        let request_preflight = unique_offset(preflight, "let mut index = 0;");

        assert!(count_guard < completed_epoch);
        assert!(completed_epoch < exact_successor);
        assert!(exact_successor < rejection);
        assert!(rejection < request_preflight);
    }

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
