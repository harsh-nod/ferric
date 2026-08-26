//! Move-only physical custody routing for dynamic M1 serving actions.
//!
//! Concrete first-publication, same-shape rearm, and quiescent rollover code
//! has very different input owners. This module keeps their common serving
//! decision boundary small: it validates the registry action against retained
//! queue custody, consumes the selected operation exactly once, and returns the
//! exact batch plan only after physical publication succeeds.

use core::fmt;

use ferric_spec::completion::CompletionEpoch;

use crate::{
    M1ServingBatchPlanV1, M1ServingPlanV1, M1ServingQueueActionV1, M1ServingRolloverReasonV1,
    M1SpeculativeGenerationLoopErrorV1, M1SpeculativeGenerationLoopV1,
    M1SpeculativePreflightedRoundV1, M1SpeculativeRoundOutcomeV1,
};

/// Concrete physical operations supplied by the Ferric lifecycle owner.
///
/// `Q` must contain the complete quiescent native queue/allocation session,
/// model allocations, partitioned active-KV pool and ledger, active/parked/
/// terminal member owners, scheduler dispatch and completed epoch, round
/// history, checked output, workspaces, and completion-output allocation.
///
/// The lower rollover capability consumes that owner plus the exact successor
/// batch. Before consuming either it must validate prior/next plan, quiescence,
/// roster, and successor epoch. On success it must cleanly detach and reclaim
/// the old queue allocation without quarantining `Engine`, preserve model and
/// live KV custody by request/role, rebuild exact next-shape packet/workspace/
/// completion owners, run the normal schedule/reserve/prepare/create/publish
/// path, and return the complete successor `Q`.
///
/// A rejection before scheduler or queue-generation progress is retryable and
/// must return the logically unchanged `Q`. Once a scheduler dispatch, native
/// queue generation, allocation release, or KV transfer cannot be rolled back,
/// failure is terminal and must return `TerminalCustody` containing every
/// residual owner. Successful rollover advances the scheduler epoch exactly to
/// `batch.epoch()` and publishes exactly one native queue generation; retryable
/// failure advances neither. This is the capability required from the lower
/// lifecycle and must not be emulated through the current quarantining teardown.
pub trait M1ServingPhysicalOperationsV1<Q> {
    type Error;
    type TerminalCustody;

    /// Builds, binds, and publishes a first physical queue generation.
    /// Retryable failure leaves first-publication inputs retained by the
    /// operations owner; terminal failure returns their quarantine explicitly.
    ///
    /// # Errors
    ///
    /// Returns exhaustive retryable or terminal lower-layer custody.
    fn fresh_launch(
        &mut self,
        batch: &M1ServingBatchPlanV1,
    ) -> Result<Q, M1ServingPhysicalOperationFailureV1<(), Self::TerminalCustody, Self::Error>>;

    /// Runs the existing unchanged-plan schedule/reserve/prepare/submit path.
    ///
    /// # Errors
    ///
    /// Returns the unchanged queue owner when retryable, or terminal quarantine.
    fn same_shape_rearm(
        &mut self,
        custody: Q,
        batch: &M1ServingBatchPlanV1,
    ) -> Result<Q, M1ServingPhysicalOperationFailureV1<Q, Self::TerminalCustody, Self::Error>>;

    /// Detaches the quiescent generation, rebuilds exact shape-dependent
    /// owners, and rebinds before publishing the next unlike plan.
    ///
    /// # Errors
    ///
    /// Returns the unchanged prior queue before irreversible progress, or
    /// terminal quarantine retaining every residual owner afterward.
    fn quiescent_rollover(
        &mut self,
        custody: Q,
        prior: M1ServingPlanV1,
        next: M1ServingPlanV1,
        reason: M1ServingRolloverReasonV1,
        batch: &M1ServingBatchPlanV1,
    ) -> Result<Q, M1ServingPhysicalOperationFailureV1<Q, Self::TerminalCustody, Self::Error>>;
}

/// Exhaustive lower physical-operation failure custody.
#[must_use = "a physical operation failure retains retryable or terminal custody"]
#[derive(Debug)]
pub enum M1ServingPhysicalOperationFailureV1<Q, T, E> {
    Retryable { source: E, custody: Q },
    Terminal { source: E, custody: T },
}

/// Result of routing one registry action through its physical operation.
pub type M1ServingPhysicalPublishResultV1<Q, T, E> =
    Result<M1ServingPhysicalPublishedV1<Q>, Box<M1ServingPhysicalBridgeFailureV1<Q, T, E>>>;

/// Physical queue state at a registry planning boundary.
#[must_use = "physical queue and model/KV custody must remain retained"]
#[derive(Debug)]
pub enum M1ServingPhysicalQueueCustodyV1<Q> {
    Vacant,
    Quiescent { plan: M1ServingPlanV1, custody: Q },
}

impl<Q> M1ServingPhysicalQueueCustodyV1<Q> {
    #[must_use]
    pub const fn bound_plan(&self) -> Option<M1ServingPlanV1> {
        match self {
            Self::Vacant => None,
            Self::Quiescent { plan, .. } => Some(*plan),
        }
    }

    /// Consumes the exact registry plan through one physical operation.
    ///
    /// # Errors
    ///
    /// Action/custody drift is rejected without invoking the executor. A
    /// retryable operation failure retains the prior queue custody and batch.
    pub fn publish<O>(
        self,
        batch: M1ServingBatchPlanV1,
        operations: &mut O,
    ) -> M1ServingPhysicalPublishResultV1<Q, O::TerminalCustody, O::Error>
    where
        O: M1ServingPhysicalOperationsV1<Q>,
    {
        let action = batch.action();
        let next = batch.plan();
        let result = match (self, action) {
            (Self::Vacant, M1ServingQueueActionV1::FreshLaunch) => operations
                .fresh_launch(&batch)
                .map_err(|failure| map_operation_failure(failure, |()| Self::Vacant)),
            (Self::Quiescent { plan, custody }, M1ServingQueueActionV1::SameShapeRearm)
                if plan == next =>
            {
                operations
                    .same_shape_rearm(custody, &batch)
                    .map_err(|failure| {
                        map_operation_failure(failure, |custody| Self::Quiescent { plan, custody })
                    })
            }
            (
                Self::Quiescent { plan, custody },
                M1ServingQueueActionV1::QuiescentRollover {
                    prior,
                    next: declared_next,
                    reason,
                },
            ) if plan == prior && declared_next == next => operations
                .quiescent_rollover(custody, prior, declared_next, reason, &batch)
                .map_err(|failure| {
                    map_operation_failure(failure, |custody| Self::Quiescent { plan, custody })
                }),
            (custody, _) => {
                return Err(Box::new(M1ServingPhysicalBridgeFailureV1 {
                    error: M1ServingPhysicalBridgeErrorV1::ActionCustodyDrift,
                    custody: M1ServingPhysicalFailureCustodyV1::Retryable(custody),
                    batch,
                }));
            }
        };
        match result {
            Ok(custody) => Ok(M1ServingPhysicalPublishedV1 {
                plan: next,
                epoch: batch.epoch(),
                custody,
                batch,
            }),
            Err(M1ServingPhysicalOperationFailureV1::Retryable { source, custody }) => {
                Err(Box::new(M1ServingPhysicalBridgeFailureV1 {
                    error: M1ServingPhysicalBridgeErrorV1::Operation(source),
                    custody: M1ServingPhysicalFailureCustodyV1::Retryable(custody),
                    batch,
                }))
            }
            Err(M1ServingPhysicalOperationFailureV1::Terminal { source, custody }) => {
                Err(Box::new(M1ServingPhysicalBridgeFailureV1 {
                    error: M1ServingPhysicalBridgeErrorV1::Operation(source),
                    custody: M1ServingPhysicalFailureCustodyV1::Terminal(custody),
                    batch,
                }))
            }
        }
    }
}

fn map_operation_failure<Q, T, E, R>(
    failure: M1ServingPhysicalOperationFailureV1<Q, T, E>,
    recover: impl FnOnce(Q) -> R,
) -> M1ServingPhysicalOperationFailureV1<R, T, E> {
    match failure {
        M1ServingPhysicalOperationFailureV1::Retryable { source, custody } => {
            M1ServingPhysicalOperationFailureV1::Retryable {
                source,
                custody: recover(custody),
            }
        }
        M1ServingPhysicalOperationFailureV1::Terminal { source, custody } => {
            M1ServingPhysicalOperationFailureV1::Terminal { source, custody }
        }
    }
}

/// Published physical generation retaining the uncommitted registry plan.
#[must_use = "published physical custody must complete or remain retained"]
#[derive(Debug)]
pub struct M1ServingPhysicalPublishedV1<Q> {
    plan: M1ServingPlanV1,
    epoch: CompletionEpoch,
    custody: Q,
    batch: M1ServingBatchPlanV1,
}

impl<Q> M1ServingPhysicalPublishedV1<Q> {
    #[must_use]
    pub const fn plan(&self) -> M1ServingPlanV1 {
        self.plan
    }

    #[must_use]
    pub const fn epoch(&self) -> CompletionEpoch {
        self.epoch
    }

    pub const fn batch(&self) -> &M1ServingBatchPlanV1 {
        &self.batch
    }

    /// Marks exact physical completion and returns the registry publication
    /// plan beside quiescent custody. Speculative coordinator permits must be
    /// committed only after this check succeeds.
    ///
    /// # Errors
    ///
    /// Returns the complete published owner on epoch drift.
    pub fn complete_physical(
        self,
        epoch: CompletionEpoch,
    ) -> Result<
        (M1ServingBatchPlanV1, M1ServingPhysicalQueueCustodyV1<Q>),
        Box<M1ServingPhysicalCompletionFailureV1<Q>>,
    > {
        if epoch != self.epoch {
            return Err(Box::new(M1ServingPhysicalCompletionFailureV1 {
                expected: self.epoch,
                actual: epoch,
                published: self,
            }));
        }
        Ok((
            self.batch,
            M1ServingPhysicalQueueCustodyV1::Quiescent {
                plan: self.plan,
                custody: self.custody,
            },
        ))
    }

    /// Commits a speculative coordinator permit after exact physical
    /// completion of the published queue generation.
    ///
    /// The published serving plan, physical epoch, and permit selection/epoch
    /// are checked before the coordinator is mutated. Every failure retains
    /// both the published physical owner and the unchanged permit.
    ///
    /// # Errors
    ///
    /// Returns move-only recovery on physical epoch, selection, permit epoch,
    /// or coordinator-state drift.
    pub fn complete_speculative_physical(
        self,
        epoch: CompletionEpoch,
        coordinator: &mut M1SpeculativeGenerationLoopV1,
        permit: M1SpeculativePreflightedRoundV1,
    ) -> Result<
        (
            M1ServingBatchPlanV1,
            M1ServingPhysicalQueueCustodyV1<Q>,
            M1SpeculativeRoundOutcomeV1,
        ),
        Box<M1ServingSpeculativeCompletionFailureV1<Q>>,
    > {
        if epoch != self.epoch {
            return Err(Box::new(M1ServingSpeculativeCompletionFailureV1 {
                error: M1ServingSpeculativeCompletionErrorV1::PhysicalEpoch {
                    expected: self.epoch,
                    actual: epoch,
                },
                published: self,
                permit,
            }));
        }
        if permit.selection() != self.plan.target() {
            return Err(Box::new(M1ServingSpeculativeCompletionFailureV1 {
                error: M1ServingSpeculativeCompletionErrorV1::Selection {
                    expected: self.plan.target(),
                    actual: permit.selection(),
                },
                published: self,
                permit,
            }));
        }
        if permit.epoch() != self.epoch {
            return Err(Box::new(M1ServingSpeculativeCompletionFailureV1 {
                error: M1ServingSpeculativeCompletionErrorV1::PermitEpoch {
                    expected: self.epoch,
                    actual: permit.epoch(),
                },
                published: self,
                permit,
            }));
        }
        let outcome = match coordinator.commit_preflighted_round(permit) {
            Ok(outcome) => outcome,
            Err(failure) => {
                let (source, permit) = failure.into_parts();
                return Err(Box::new(M1ServingSpeculativeCompletionFailureV1 {
                    error: M1ServingSpeculativeCompletionErrorV1::Coordinator(source),
                    published: self,
                    permit,
                }));
            }
        };
        Ok((
            self.batch,
            M1ServingPhysicalQueueCustodyV1::Quiescent {
                plan: self.plan,
                custody: self.custody,
            },
            outcome,
        ))
    }
}

/// Stable rejection while joining physical and speculative completion.
#[derive(Debug)]
pub enum M1ServingSpeculativeCompletionErrorV1 {
    PhysicalEpoch {
        expected: CompletionEpoch,
        actual: CompletionEpoch,
    },
    Selection {
        expected: ferric_spec::Qwen3PlanSelection,
        actual: ferric_spec::Qwen3PlanSelection,
    },
    PermitEpoch {
        expected: CompletionEpoch,
        actual: CompletionEpoch,
    },
    Coordinator(M1SpeculativeGenerationLoopErrorV1),
}

impl fmt::Display for M1ServingSpeculativeCompletionErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "M1 serving speculative completion rejected: {self:?}"
        )
    }
}

impl std::error::Error for M1ServingSpeculativeCompletionErrorV1 {}

/// Retry-safe speculative completion rejection retaining all physical custody.
#[must_use = "failed speculative completion retains published custody and permit"]
#[derive(Debug)]
pub struct M1ServingSpeculativeCompletionFailureV1<Q> {
    error: M1ServingSpeculativeCompletionErrorV1,
    published: M1ServingPhysicalPublishedV1<Q>,
    permit: M1SpeculativePreflightedRoundV1,
}

impl<Q> M1ServingSpeculativeCompletionFailureV1<Q> {
    #[must_use]
    pub const fn error(&self) -> &M1ServingSpeculativeCompletionErrorV1 {
        &self.error
    }

    pub fn into_parts(
        self,
    ) -> (
        M1ServingSpeculativeCompletionErrorV1,
        M1ServingPhysicalPublishedV1<Q>,
        M1SpeculativePreflightedRoundV1,
    ) {
        (self.error, self.published, self.permit)
    }
}

/// Physical completion drift retaining the exact published owner.
#[must_use = "failed physical completion retains published custody"]
#[derive(Debug)]
pub struct M1ServingPhysicalCompletionFailureV1<Q> {
    expected: CompletionEpoch,
    actual: CompletionEpoch,
    published: M1ServingPhysicalPublishedV1<Q>,
}

impl<Q> M1ServingPhysicalCompletionFailureV1<Q> {
    #[must_use]
    pub const fn expected(&self) -> CompletionEpoch {
        self.expected
    }

    #[must_use]
    pub const fn actual(&self) -> CompletionEpoch {
        self.actual
    }

    pub fn into_published(self) -> M1ServingPhysicalPublishedV1<Q> {
        self.published
    }
}

/// Stable bridge rejection.
#[derive(Debug)]
pub enum M1ServingPhysicalBridgeErrorV1<E> {
    ActionCustodyDrift,
    Operation(E),
}

impl<E: fmt::Debug> fmt::Display for M1ServingPhysicalBridgeErrorV1<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "M1 serving physical bridge rejected: {self:?}")
    }
}

impl<E: fmt::Debug> std::error::Error for M1ServingPhysicalBridgeErrorV1<E> {}

/// Retry-safe physical action rejection.
#[must_use = "failed physical action retains retryable or terminal custody and batch plan"]
#[derive(Debug)]
pub struct M1ServingPhysicalBridgeFailureV1<Q, T, E> {
    error: M1ServingPhysicalBridgeErrorV1<E>,
    custody: M1ServingPhysicalFailureCustodyV1<Q, T>,
    batch: M1ServingBatchPlanV1,
}

/// Retryable prior queue custody or terminal lower-layer quarantine.
#[must_use = "physical failure custody must remain retained"]
#[derive(Debug)]
pub enum M1ServingPhysicalFailureCustodyV1<Q, T> {
    Retryable(M1ServingPhysicalQueueCustodyV1<Q>),
    Terminal(T),
}

impl<Q, T, E> M1ServingPhysicalBridgeFailureV1<Q, T, E> {
    #[must_use]
    pub const fn error(&self) -> &M1ServingPhysicalBridgeErrorV1<E> {
        &self.error
    }

    pub fn into_parts(
        self,
    ) -> (
        M1ServingPhysicalBridgeErrorV1<E>,
        M1ServingPhysicalFailureCustodyV1<Q, T>,
        M1ServingBatchPlanV1,
    ) {
        (self.error, self.custody, self.batch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{M1ServingPlanV1, M1ServingRegistryV1};
    use ferric_spec::{
        Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection, RequestId,
    };

    #[derive(Debug, Eq, PartialEq)]
    struct Custody(u64);

    #[derive(Default)]
    struct Operations {
        calls: Vec<&'static str>,
        fail: bool,
        terminal: bool,
    }

    impl M1ServingPhysicalOperationsV1<Custody> for Operations {
        type Error = &'static str;
        type TerminalCustody = Custody;

        fn fresh_launch(
            &mut self,
            _: &M1ServingBatchPlanV1,
        ) -> Result<
            Custody,
            M1ServingPhysicalOperationFailureV1<(), Self::TerminalCustody, Self::Error>,
        > {
            self.calls.push("fresh");
            if self.terminal {
                Err(M1ServingPhysicalOperationFailureV1::Terminal {
                    source: "fresh",
                    custody: Custody(0),
                })
            } else if self.fail {
                Err(M1ServingPhysicalOperationFailureV1::Retryable {
                    source: "fresh",
                    custody: (),
                })
            } else {
                Ok(Custody(1))
            }
        }

        fn same_shape_rearm(
            &mut self,
            custody: Custody,
            _: &M1ServingBatchPlanV1,
        ) -> Result<
            Custody,
            M1ServingPhysicalOperationFailureV1<Custody, Self::TerminalCustody, Self::Error>,
        > {
            self.calls.push("rearm");
            if self.terminal {
                Err(M1ServingPhysicalOperationFailureV1::Terminal {
                    source: "rearm",
                    custody,
                })
            } else if self.fail {
                Err(M1ServingPhysicalOperationFailureV1::Retryable {
                    source: "rearm",
                    custody,
                })
            } else {
                Ok(Custody(custody.0 + 1))
            }
        }

        fn quiescent_rollover(
            &mut self,
            custody: Custody,
            _: M1ServingPlanV1,
            _: M1ServingPlanV1,
            _: M1ServingRolloverReasonV1,
            _: &M1ServingBatchPlanV1,
        ) -> Result<
            Custody,
            M1ServingPhysicalOperationFailureV1<Custody, Self::TerminalCustody, Self::Error>,
        > {
            self.calls.push("rollover");
            if self.terminal {
                Err(M1ServingPhysicalOperationFailureV1::Terminal {
                    source: "rollover",
                    custody,
                })
            } else if self.fail {
                Err(M1ServingPhysicalOperationFailureV1::Retryable {
                    source: "rollover",
                    custody,
                })
            } else {
                Ok(Custody(custody.0 + 10))
            }
        }
    }

    fn pair(mode: Qwen3ExecutionMode, bucket: Qwen3PlanBucket) -> M1ServingPlanV1 {
        M1ServingPlanV1::new(
            Qwen3PlanSelection {
                role: Qwen3ModelRole::Target8B,
                mode,
                bucket,
            },
            Qwen3PlanSelection {
                role: Qwen3ModelRole::Draft06B,
                mode,
                bucket,
            },
        )
        .unwrap()
    }

    fn fresh_plan(plan: M1ServingPlanV1) -> M1ServingBatchPlanV1 {
        let mut registry = M1ServingRegistryV1::<8>::new().unwrap();
        registry.admit(RequestId::new(0, 1), plan).unwrap();
        registry.plan_next().unwrap().unwrap()
    }

    fn bound_next_plan(plan: M1ServingPlanV1) -> M1ServingBatchPlanV1 {
        let prefill = pair(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128);
        let mut registry = M1ServingRegistryV1::<8>::new().unwrap();
        let request = RequestId::new(0, 1);
        registry.admit(request, prefill).unwrap();
        let first = registry.plan_next().unwrap().unwrap();
        let epoch = first.epoch();
        registry.record_publication(first).unwrap();
        registry
            .complete_exact(
                epoch,
                &[crate::M1ServingCompletionDispositionV1::Continue(plan)],
            )
            .unwrap();
        registry.plan_next().unwrap().unwrap()
    }

    fn rebound_next_plan(plan: M1ServingPlanV1) -> M1ServingBatchPlanV1 {
        let prefill = pair(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128);
        let mut registry = M1ServingRegistryV1::<8>::new().unwrap();
        let request = RequestId::new(0, 1);
        registry.admit(request, prefill).unwrap();
        for next in [plan, plan] {
            let batch = registry.plan_next().unwrap().unwrap();
            let epoch = batch.epoch();
            registry.record_publication(batch).unwrap();
            registry
                .complete_exact(
                    epoch,
                    &[crate::M1ServingCompletionDispositionV1::Continue(next)],
                )
                .unwrap();
        }
        registry.plan_next().unwrap().unwrap()
    }

    fn rollover_next_plan(prior: M1ServingPlanV1, next: M1ServingPlanV1) -> M1ServingBatchPlanV1 {
        let prefill = pair(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128);
        let mut registry = M1ServingRegistryV1::<8>::new().unwrap();
        let request = RequestId::new(0, 1);
        registry.admit(request, prefill).unwrap();
        for next_plan in [prior, next] {
            let batch = registry.plan_next().unwrap().unwrap();
            let epoch = batch.epoch();
            registry.record_publication(batch).unwrap();
            registry
                .complete_exact(
                    epoch,
                    &[crate::M1ServingCompletionDispositionV1::Continue(next_plan)],
                )
                .unwrap();
        }
        registry.plan_next().unwrap().unwrap()
    }

    #[test]
    fn fresh_rearm_and_rollover_route_exactly_once() {
        let prefill = pair(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128);
        let mut operations = Operations::default();
        let published = M1ServingPhysicalQueueCustodyV1::Vacant
            .publish(fresh_plan(prefill), &mut operations)
            .unwrap();
        let epoch = published.epoch();
        let (_, queue) = published.complete_physical(epoch).unwrap();
        assert_eq!(operations.calls, ["fresh"]);

        let decode = pair(Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS1C8192);
        let rollover = queue
            .publish(bound_next_plan(decode), &mut operations)
            .unwrap();
        let epoch = rollover.epoch();
        let (_, queue) = rollover.complete_physical(epoch).unwrap();
        assert_eq!(operations.calls, ["fresh", "rollover"]);

        let same = queue
            .publish(rebound_next_plan(decode), &mut operations)
            .unwrap();
        let epoch = same.epoch();
        let (_, queue) = same.complete_physical(epoch).unwrap();
        assert_eq!(operations.calls, ["fresh", "rollover", "rearm"]);

        let speculative = pair(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K8C8192,
        );
        let rollover = queue
            .publish(rollover_next_plan(decode, speculative), &mut operations)
            .unwrap();
        assert_eq!(rollover.plan(), speculative);
        assert_eq!(operations.calls, ["fresh", "rollover", "rearm", "rollover"]);
    }

    #[test]
    fn action_drift_and_operation_failure_retain_exact_custody() {
        let prefill = pair(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128);
        let batch = fresh_plan(prefill);
        let mut operations = Operations {
            calls: Vec::new(),
            fail: true,
            terminal: false,
        };
        let failure = M1ServingPhysicalQueueCustodyV1::Vacant
            .publish(batch, &mut operations)
            .unwrap_err();
        let (_, custody, _) = failure.into_parts();
        assert!(matches!(
            custody,
            M1ServingPhysicalFailureCustodyV1::Retryable(M1ServingPhysicalQueueCustodyV1::Vacant)
        ));
        assert_eq!(operations.calls, ["fresh"]);

        let decode = pair(Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS1C8192);
        let failure = M1ServingPhysicalQueueCustodyV1::Quiescent {
            plan: decode,
            custody: Custody(70),
        }
        .publish(fresh_plan(prefill), &mut Operations::default())
        .unwrap_err();
        let (_, custody, _) = failure.into_parts();
        assert!(matches!(
            custody,
            M1ServingPhysicalFailureCustodyV1::Retryable(
                M1ServingPhysicalQueueCustodyV1::Quiescent {
                    plan,
                    custody: Custody(70)
                }
            ) if plan == decode
        ));
    }

    #[test]
    fn every_admitted_speculative_shape_rolls_over_and_failures_are_exhaustive() {
        let decode = pair(Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS1C8192);
        for bucket in [
            Qwen3PlanBucket::SpeculativeS1K4C8192,
            Qwen3PlanBucket::SpeculativeS8K4C8192,
            Qwen3PlanBucket::SpeculativeS1K8C8192,
            Qwen3PlanBucket::SpeculativeS1K16C8192,
        ] {
            let next = pair(Qwen3ExecutionMode::Speculative, bucket);
            let mut operations = Operations::default();
            let published = M1ServingPhysicalQueueCustodyV1::Quiescent {
                plan: decode,
                custody: Custody(5),
            }
            .publish(rollover_next_plan(decode, next), &mut operations)
            .unwrap();
            assert_eq!(published.plan(), next);
            assert_eq!(operations.calls, ["rollover"]);
        }

        let next = pair(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K16C8192,
        );
        let batch = rollover_next_plan(decode, next);
        let mut retryable = Operations {
            calls: Vec::new(),
            fail: true,
            terminal: false,
        };
        let failure = M1ServingPhysicalQueueCustodyV1::Quiescent {
            plan: decode,
            custody: Custody(80),
        }
        .publish(batch, &mut retryable)
        .unwrap_err();
        let (_, custody, _) = failure.into_parts();
        assert!(matches!(
            custody,
            M1ServingPhysicalFailureCustodyV1::Retryable(
                M1ServingPhysicalQueueCustodyV1::Quiescent {
                    custody: Custody(80),
                    ..
                }
            )
        ));

        let batch = rollover_next_plan(decode, next);
        let mut terminal = Operations {
            calls: Vec::new(),
            fail: false,
            terminal: true,
        };
        let failure = M1ServingPhysicalQueueCustodyV1::Quiescent {
            plan: decode,
            custody: Custody(90),
        }
        .publish(batch, &mut terminal)
        .unwrap_err();
        let (_, custody, _) = failure.into_parts();
        assert!(matches!(
            custody,
            M1ServingPhysicalFailureCustodyV1::Terminal(Custody(90))
        ));
    }

    #[test]
    fn physical_completion_epoch_drift_retains_the_published_owner() {
        let prefill = pair(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128);
        let mut operations = Operations::default();
        let published = M1ServingPhysicalQueueCustodyV1::Vacant
            .publish(fresh_plan(prefill), &mut operations)
            .unwrap();
        let expected = published.epoch();
        let failure = published
            .complete_physical(CompletionEpoch::new(expected.value() + 1))
            .unwrap_err();
        assert_eq!(failure.expected(), expected);
        let published = failure.into_published();
        assert_eq!(published.epoch(), expected);
        assert_eq!(published.plan(), prefill);
    }
}
