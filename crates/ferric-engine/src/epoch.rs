//! Completion epochs and deferred resource retirement.

use ferric_spec::completion::CompletionEpoch;
use vstd::prelude::*;

verus! {

/// Linear evidence that one ordered GPU submission is quiescent.
///
/// This capability is intentionally neither `Copy` nor `Clone` and its epoch
/// field is private. Numeric progress alone is not quiescence evidence.
#[derive(Debug, PartialEq, Eq)]
pub struct ExactCompletion {
    epoch: CompletionEpoch,
}

impl ExactCompletion {
    /// Constructs evidence at the current external HSA trust boundary.
    ///
    /// # Contracted boundary
    ///
    /// The caller must have observed successful completion of the exact HSA
    /// signal associated with `epoch`, on the one ordered completion authority,
    /// and the fe2o3 runtime contract must establish that the observation makes
    /// every resource retained by that submission quiescent. Verus proves all
    /// subsequent lifecycle transitions but does not prove this HSA premise.
    #[must_use]
    pub(super) fn from_contracted_hsa_quiescence(epoch: CompletionEpoch) -> (completion: Self)
        ensures completion.epoch_spec() == epoch,
    {
        Self { epoch }
    }

    #[must_use]
    pub const fn epoch(&self) -> (epoch: CompletionEpoch)
        ensures epoch == self.epoch_spec(),
    {
        self.epoch
    }

    pub closed spec fn epoch_spec(&self) -> CompletionEpoch {
        self.epoch
    }
}

}

#[cfg(test)]
mod tests {
    use super::ExactCompletion;
    use ferric_spec::completion::CompletionEpoch;

    #[test]
    fn capability_preserves_the_contracted_epoch() {
        let epoch = CompletionEpoch::new(7);
        let completion = ExactCompletion::from_contracted_hsa_quiescence(epoch);
        assert_eq!(completion.epoch(), epoch);
    }
}
