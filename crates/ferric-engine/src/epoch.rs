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
    pub(crate) fn from_completed_m1_queue_readback(
        scheduled: crate::M1ScheduledDispatchV1,
    ) -> Self {
        Self {
            epoch: scheduled.epoch(),
        }
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

    impl ExactCompletion {
        /// Test-only stand-in for the future HSA quiescence boundary.
        #[must_use]
        pub(crate) fn from_contracted_hsa_quiescence(epoch: CompletionEpoch) -> Self {
            Self { epoch }
        }
    }

    #[test]
    fn capability_preserves_the_contracted_epoch() {
        let epoch = CompletionEpoch::new(7);
        let completion = ExactCompletion::from_contracted_hsa_quiescence(epoch);
        assert_eq!(completion.epoch(), epoch);
    }
}
