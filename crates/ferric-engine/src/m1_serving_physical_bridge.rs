//! Move-only physical custody routing for dynamic M1 serving actions.
//!
//! Concrete first-publication, same-shape rearm, and quiescent rollover code
//! has very different input owners. This module keeps their common serving
//! decision boundary small: it validates the registry action against retained
//! queue custody, consumes the selected operation exactly once, and returns the
//! exact batch plan only after physical publication succeeds.

use core::fmt;

use ferric_spec::completion::CompletionEpoch;

use crate::m1_serving_registry::M1ServingRegistryIdentityV1;
use crate::{
    M1CheckedCompletionOutputV1, M1DeviceKvCompletionDispositionV1, M1ScheduledDispatchV1,
    M1ServingBatchPlanV1, M1ServingCompletionDispositionV1, M1ServingPlanV1,
    M1ServingPublicationFailureV1, M1ServingPublicationReservationV1, M1ServingQueueActionV1,
    M1ServingRegistryErrorV1, M1ServingRegistryV1, M1ServingRolloverReasonV1,
    M1SpeculativeGenerationLoopErrorV1, M1SpeculativeGenerationLoopV1,
    M1SpeculativePreflightedRoundV1, M1SpeculativeRoundOutcomeV1,
};

/// Concrete physical operations supplied by the Ferric lifecycle owner.
///
/// `Quiescent` must contain the complete quiescent native queue/allocation session,
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
/// path, and return the complete successor `Published` owner. Physical
/// completion must then consume that published typestate and recover the next
/// complete `Quiescent` owner before registry or speculative state advances.
///
/// A rejection before scheduler or queue-generation progress is retryable and
/// must return the logically unchanged `Q`. Once a scheduler dispatch, native
/// queue generation, allocation release, or KV transfer cannot be rolled back,
/// failure is terminal and must return `TerminalCustody` containing every
/// residual owner. Successful rollover advances the scheduler epoch exactly to
/// `batch.epoch()` and publishes exactly one native queue generation; retryable
/// failure advances neither. This is the capability required from the lower
/// lifecycle and must not be emulated through the current quarantining teardown.
pub trait M1ServingPhysicalOperationsV1 {
    type Quiescent;
    type Published;
    type Readback;
    type Error;
    type TerminalCustody;

    /// Returns the exact scheduler authority retained by a successful physical
    /// publication. The bridge validates it before advancing the registry.
    fn scheduled_dispatch<'a>(&self, custody: &'a Self::Published) -> &'a M1ScheduledDispatchV1;

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
    ) -> M1ServingPhysicalOperationResultV1<Self::Published, (), Self::TerminalCustody, Self::Error>;

    /// Runs the existing unchanged-plan schedule/reserve/prepare/submit path.
    ///
    /// # Errors
    ///
    /// Returns the unchanged queue owner when retryable, or terminal quarantine.
    fn same_shape_rearm(
        &mut self,
        custody: Self::Quiescent,
        batch: &M1ServingBatchPlanV1,
    ) -> M1ServingPhysicalOperationResultV1<
        Self::Published,
        Self::Quiescent,
        Self::TerminalCustody,
        Self::Error,
    >;

    /// Detaches the quiescent generation, rebuilds exact shape-dependent
    /// owners, and rebinds before publishing the next unlike plan.
    ///
    /// # Errors
    ///
    /// Returns the unchanged prior queue before irreversible progress, or
    /// terminal quarantine retaining every residual owner afterward.
    fn quiescent_rollover(
        &mut self,
        custody: Self::Quiescent,
        prior: M1ServingPlanV1,
        next: M1ServingPlanV1,
        reason: M1ServingRolloverReasonV1,
        batch: &M1ServingBatchPlanV1,
    ) -> M1ServingPhysicalOperationResultV1<
        Self::Published,
        Self::Quiescent,
        Self::TerminalCustody,
        Self::Error,
    >;

    /// Waits for exact physical completion, recycles the published queue, and
    /// checks its readback without settling device KV or Engine state.
    ///
    /// A retryable failure must return the unchanged published owner. Once
    /// completion, recycle, or readback has made non-rollbackable progress,
    /// failure is terminal and must return exhaustive lower-layer custody.
    ///
    /// # Errors
    ///
    /// Returns unchanged published custody before irreversible progress, or
    /// exhaustive terminal custody after it.
    fn read_published(
        &mut self,
        custody: Self::Published,
        epoch: CompletionEpoch,
        batch: &M1ServingBatchPlanV1,
    ) -> M1ServingPhysicalOperationResultV1<
        Self::Readback,
        Self::Published,
        Self::TerminalCustody,
        Self::Error,
    >;

    /// Borrows the inert, semantically checked completion used to derive a
    /// speculative permit before any device-KV or Engine completion mutation.
    fn checked_completion<'a>(
        &self,
        custody: &'a Self::Readback,
    ) -> &'a M1CheckedCompletionOutputV1;

    /// Settles the checked readback with the exact caller-provided physical
    /// Continue/Retire roster and releases retired pages.
    ///
    /// A retryable rejection must return the unchanged checked readback. Any
    /// failure after device-KV or Engine mutation is terminal and retains all
    /// residual custody.
    ///
    /// # Errors
    ///
    /// Returns retryable unchanged readback custody or terminal lower custody.
    fn settle_readback(
        &mut self,
        custody: Self::Readback,
        dispositions: Vec<M1DeviceKvCompletionDispositionV1>,
    ) -> M1ServingPhysicalOperationResultV1<
        Self::Quiescent,
        Self::Readback,
        Self::TerminalCustody,
        Self::Error,
    >;
}

/// Exhaustive lower physical-operation failure custody.
#[must_use = "a physical operation failure retains retryable or terminal custody"]
#[derive(Debug)]
pub enum M1ServingPhysicalOperationFailureV1<Q, T, E> {
    Retryable { source: E, custody: Q },
    Terminal { source: E, custody: T },
}

/// Lower physical-operation result with exhaustive retryable or terminal custody.
pub type M1ServingPhysicalOperationResultV1<S, Q, T, E> =
    Result<S, M1ServingPhysicalOperationFailureV1<Q, T, E>>;

/// Result of routing one registry action through its physical operation.
pub type M1ServingPhysicalPublishResultV1<Q, P, T, E> =
    Result<M1ServingPhysicalPublishedV1<P>, Box<M1ServingPhysicalBridgeFailureV1<Q, P, T, E>>>;

type M1ServingPhysicalRouteResultV1<Q, P, T, E> =
    Result<M1ServingPhysicalRawPublishedV1<P>, M1ServingPhysicalRouteFailureV1<Q, T, E>>;

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

    /// Consumes one live registry reservation through one physical operation
    /// and immediately records successful publication in the registry.
    ///
    /// # Errors
    ///
    /// Action/custody drift is rejected without invoking the executor. A
    /// retryable operation failure retains the prior queue and abortable
    /// reservation. Scheduler-authority drift and registry-record rejection
    /// retain the already-published owner without exposing completion.
    pub fn publish<const C: usize, O>(
        self,
        reservation: M1ServingPublicationReservationV1,
        registry: &mut M1ServingRegistryV1<C>,
        operations: &mut O,
    ) -> M1ServingPhysicalPublishResultV1<Q, O::Published, O::TerminalCustody, O::Error>
    where
        O: M1ServingPhysicalOperationsV1<Quiescent = Q>,
    {
        if let Err(error) = registry.preflight_publication(&reservation) {
            return Err(Box::new(M1ServingPhysicalBridgeFailureV1 {
                error: M1ServingPhysicalBridgeErrorV1::Registry(error),
                custody: M1ServingPhysicalFailureCustodyV1::Retryable(
                    M1ServingPhysicalRetryablePublicationV1 {
                        custody: self,
                        reservation,
                    },
                ),
            }));
        }
        let registry_identity = reservation.registry_identity();
        let batch = reservation.physical_batch();
        let raw = match self.route_batch(batch, registry_identity, operations) {
            Ok(raw) => raw,
            Err(failure) => {
                let (error, custody, batch) = failure.into_parts();
                let custody = match custody {
                    M1ServingPhysicalRouteFailureCustodyV1::Retryable(custody) => {
                        M1ServingPhysicalFailureCustodyV1::Retryable(
                            M1ServingPhysicalRetryablePublicationV1 {
                                custody,
                                reservation,
                            },
                        )
                    }
                    M1ServingPhysicalRouteFailureCustodyV1::Terminal(custody) => {
                        M1ServingPhysicalFailureCustodyV1::Terminal(
                            M1ServingPhysicalTerminalPublicationV1 {
                                reservation,
                                batch,
                                custody,
                            },
                        )
                    }
                };
                return Err(Box::new(M1ServingPhysicalBridgeFailureV1 {
                    error,
                    custody,
                }));
            }
        };

        if let Err(error) =
            validate_scheduled_dispatch(operations.scheduled_dispatch(&raw.custody), &reservation)
        {
            return Err(Box::new(M1ServingPhysicalBridgeFailureV1 {
                error,
                custody: M1ServingPhysicalFailureCustodyV1::UnmatchedPublished(
                    M1ServingPhysicalUnmatchedPublishedV1 { raw, reservation },
                ),
            }));
        }

        match registry.record_publication(reservation) {
            Ok(()) => Ok(raw.activate()),
            Err(failure) => {
                let error = failure.error();
                Err(Box::new(M1ServingPhysicalBridgeFailureV1 {
                    error: M1ServingPhysicalBridgeErrorV1::Registry(error),
                    custody: M1ServingPhysicalFailureCustodyV1::UnrecordedPublished(
                        M1ServingPhysicalUnrecordedPublishedV1 {
                            raw,
                            reservation: failure.into_reservation(),
                        },
                    ),
                }))
            }
        }
    }

    fn route_batch<O>(
        self,
        batch: M1ServingBatchPlanV1,
        registry_identity: M1ServingRegistryIdentityV1,
        operations: &mut O,
    ) -> M1ServingPhysicalRouteResultV1<Q, O::Published, O::TerminalCustody, O::Error>
    where
        O: M1ServingPhysicalOperationsV1<Quiescent = Q>,
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
                return Err(M1ServingPhysicalRouteFailureV1 {
                    error: M1ServingPhysicalBridgeErrorV1::ActionCustodyDrift,
                    custody: M1ServingPhysicalRouteFailureCustodyV1::Retryable(custody),
                    batch,
                });
            }
        };
        match result {
            Ok(custody) => Ok(M1ServingPhysicalRawPublishedV1 {
                registry_identity,
                plan: next,
                epoch: batch.epoch(),
                custody,
                batch,
            }),
            Err(M1ServingPhysicalOperationFailureV1::Retryable { source, custody }) => {
                Err(M1ServingPhysicalRouteFailureV1 {
                    error: M1ServingPhysicalBridgeErrorV1::Operation(source),
                    custody: M1ServingPhysicalRouteFailureCustodyV1::Retryable(custody),
                    batch,
                })
            }
            Err(M1ServingPhysicalOperationFailureV1::Terminal { source, custody }) => {
                Err(M1ServingPhysicalRouteFailureV1 {
                    error: M1ServingPhysicalBridgeErrorV1::Operation(source),
                    custody: M1ServingPhysicalRouteFailureCustodyV1::Terminal(custody),
                    batch,
                })
            }
        }
    }
}

fn validate_scheduled_dispatch<E>(
    scheduled: &M1ScheduledDispatchV1,
    reservation: &M1ServingPublicationReservationV1,
) -> Result<(), M1ServingPhysicalBridgeErrorV1<E>> {
    if scheduled.epoch() != reservation.epoch() {
        return Err(M1ServingPhysicalBridgeErrorV1::ScheduledEpoch {
            expected: reservation.epoch(),
            actual: scheduled.epoch(),
        });
    }
    if scheduled.member_count() != reservation.requests().len() {
        return Err(M1ServingPhysicalBridgeErrorV1::ScheduledMemberCount {
            expected: reservation.requests().len(),
            actual: scheduled.member_count(),
        });
    }
    for (lane, expected) in reservation.requests().iter().copied().enumerate() {
        let actual = scheduled.member(lane);
        if actual != Some(expected) {
            return Err(M1ServingPhysicalBridgeErrorV1::ScheduledMember {
                lane,
                expected,
                actual,
            });
        }
    }
    Ok(())
}

#[derive(Debug)]
struct M1ServingPhysicalRawPublishedV1<P> {
    registry_identity: M1ServingRegistryIdentityV1,
    plan: M1ServingPlanV1,
    epoch: CompletionEpoch,
    custody: P,
    batch: M1ServingBatchPlanV1,
}

impl<P> M1ServingPhysicalRawPublishedV1<P> {
    fn activate(self) -> M1ServingPhysicalPublishedV1<P> {
        M1ServingPhysicalPublishedV1 {
            registry_identity: self.registry_identity,
            plan: self.plan,
            epoch: self.epoch,
            custody: self.custody,
            batch: self.batch,
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

/// Physically published generation whose exact roster is already active in the registry.
#[must_use = "published physical custody must complete or remain retained"]
#[derive(Debug)]
pub struct M1ServingPhysicalPublishedV1<P> {
    registry_identity: M1ServingRegistryIdentityV1,
    plan: M1ServingPlanV1,
    epoch: CompletionEpoch,
    custody: P,
    batch: M1ServingBatchPlanV1,
}

impl<P> M1ServingPhysicalPublishedV1<P> {
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

    /// Performs wait, recycle, observation, and semantic checking while leaving
    /// device-KV, Engine, registry, and speculative state unchanged.
    ///
    /// The registry is already `InFlight` before this operation is available;
    /// Epoch drift is rejected before the lower operation is invoked. Every
    /// failure retains either the unchanged published owner or exhaustive
    /// terminal lower-layer custody.
    ///
    /// # Errors
    ///
    /// Returns the unchanged active published owner on retryable rejection, or
    /// exhaustive terminal lower custody after irreversible progress.
    pub fn read_physical<O>(
        self,
        epoch: CompletionEpoch,
        operations: &mut O,
    ) -> M1ServingPhysicalReadbackResultV1<O::Readback, P, O::TerminalCustody, O::Error>
    where
        O: M1ServingPhysicalOperationsV1<Published = P>,
    {
        if epoch != self.epoch {
            return Err(Box::new(M1ServingPhysicalCompletionFailureV1 {
                error: M1ServingPhysicalCompletionErrorV1::Epoch {
                    expected: self.epoch,
                    actual: epoch,
                },
                custody: M1ServingPhysicalCompletionFailureCustodyV1::Retryable(self),
            }));
        }
        let Self {
            registry_identity,
            plan,
            epoch,
            custody,
            batch,
        } = self;
        match operations.read_published(custody, epoch, &batch) {
            Ok(custody) => Ok(M1ServingPhysicalReadbackV1 {
                registry_identity,
                plan,
                epoch,
                custody,
                batch,
            }),
            Err(M1ServingPhysicalOperationFailureV1::Retryable { source, custody }) => {
                Err(Box::new(M1ServingPhysicalCompletionFailureV1 {
                    error: M1ServingPhysicalCompletionErrorV1::Operation(source),
                    custody: M1ServingPhysicalCompletionFailureCustodyV1::Retryable(
                        M1ServingPhysicalPublishedV1 {
                            registry_identity,
                            plan,
                            epoch,
                            custody,
                            batch,
                        },
                    ),
                }))
            }
            Err(M1ServingPhysicalOperationFailureV1::Terminal { source, custody }) => {
                Err(Box::new(M1ServingPhysicalCompletionFailureV1 {
                    error: M1ServingPhysicalCompletionErrorV1::Operation(source),
                    custody: M1ServingPhysicalCompletionFailureCustodyV1::Terminal {
                        registry_identity,
                        plan,
                        epoch,
                        batch,
                        custody,
                    },
                }))
            }
        }
    }
}

/// Checked physical readback awaiting exact permit-bound KV settlement.
#[must_use = "checked readback custody must settle or remain retained"]
#[derive(Debug)]
pub struct M1ServingPhysicalReadbackV1<R> {
    registry_identity: M1ServingRegistryIdentityV1,
    plan: M1ServingPlanV1,
    epoch: CompletionEpoch,
    custody: R,
    batch: M1ServingBatchPlanV1,
}

impl<R> M1ServingPhysicalReadbackV1<R> {
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

    /// Borrows the checked output without granting settlement authority.
    pub fn checked<'a, O>(&'a self, operations: &'a O) -> &'a M1CheckedCompletionOutputV1
    where
        O: M1ServingPhysicalOperationsV1<Readback = R>,
    {
        operations.checked_completion(&self.custody)
    }

    /// Preflights the exact logical decision, settles physical KV with its exact
    /// Continue/Retire projection, and then applies the registry transition.
    ///
    /// # Errors
    ///
    /// Returns unchanged checked readback before physical mutation, or explicit
    /// terminal lower custody after irreversible settlement progress.
    pub fn complete_exact<const C: usize, O>(
        self,
        registry: &mut M1ServingRegistryV1<C>,
        dispositions: &[M1ServingCompletionDispositionV1],
        operations: &mut O,
    ) -> M1ServingRegistryCompletionResultV1<R, O::Quiescent, O::TerminalCustody, O::Error>
    where
        O: M1ServingPhysicalOperationsV1<Readback = R>,
    {
        if self.plan.mode() == ferric_spec::Qwen3ExecutionMode::Speculative {
            return Err(registry_completion_retryable(
                M1ServingRegistryCompletionErrorV1::SpeculativeCoordinatorRequired,
                self,
            ));
        }
        if let Err(error) = registry.preflight_completion_exact_for(
            self.registry_identity,
            self.epoch,
            dispositions,
        ) {
            return Err(registry_completion_retryable(
                M1ServingRegistryCompletionErrorV1::Registry(error),
                self,
            ));
        }
        let physical = match physical_dispositions_from_registry(dispositions) {
            Ok(physical) => physical,
            Err(()) => {
                return Err(registry_completion_retryable(
                    M1ServingRegistryCompletionErrorV1::HostAllocation,
                    self,
                ));
            }
        };
        let Self {
            registry_identity,
            plan,
            epoch,
            custody,
            batch,
        } = self;
        match operations.settle_readback(custody, physical) {
            Ok(custody) => {
                registry.apply_preflighted_completion(epoch, dispositions);
                Ok((
                    batch,
                    M1ServingPhysicalQueueCustodyV1::Quiescent { plan, custody },
                ))
            }
            Err(M1ServingPhysicalOperationFailureV1::Retryable { source, custody }) => {
                Err(Box::new(M1ServingRegistryCompletionFailureV1 {
                    error: M1ServingRegistryCompletionErrorV1::Operation(source),
                    custody: M1ServingRegistryCompletionFailureCustodyV1::Retryable(
                        M1ServingPhysicalReadbackV1 {
                            registry_identity,
                            plan,
                            epoch,
                            custody,
                            batch,
                        },
                    ),
                }))
            }
            Err(M1ServingPhysicalOperationFailureV1::Terminal { source, custody }) => {
                Err(Box::new(M1ServingRegistryCompletionFailureV1 {
                    error: M1ServingRegistryCompletionErrorV1::Operation(source),
                    custody: M1ServingRegistryCompletionFailureCustodyV1::Terminal {
                        registry_identity,
                        plan,
                        epoch,
                        batch,
                        custody,
                    },
                }))
            }
        }
    }

    /// Atomically joins checked speculative policy, physical KV settlement,
    /// coordinator commit, and exact registry completion.
    ///
    /// Selection, epoch, roster, registry, and coordinator drift are rejected
    /// before either logical state mutates. Registry dispositions are derived
    /// from checked member statuses rather than accepted from the caller.
    ///
    /// # Errors
    ///
    /// Returns checked readback plus the unchanged permit before settlement,
    /// exhaustive lower terminal custody, or sealed post-settlement custody if
    /// an internal coordinator invariant unexpectedly fails.
    pub fn commit_speculative<const C: usize, O>(
        self,
        registry: &mut M1ServingRegistryV1<C>,
        coordinator: &mut M1SpeculativeGenerationLoopV1,
        permit: M1SpeculativePreflightedRoundV1,
        operations: &mut O,
    ) -> M1ServingSpeculativeCompletionResultV1<R, O::Quiescent, O::TerminalCustody, O::Error>
    where
        O: M1ServingPhysicalOperationsV1<Readback = R>,
    {
        if permit.selection() != self.plan.target() {
            return Err(speculative_before_commit_failure(
                M1ServingSpeculativeCompletionErrorV1::Selection {
                    expected: self.plan.target(),
                    actual: permit.selection(),
                },
                self,
                permit,
            ));
        }
        if permit.epoch() != self.epoch {
            return Err(speculative_before_commit_failure(
                M1ServingSpeculativeCompletionErrorV1::PermitEpoch {
                    expected: self.epoch,
                    actual: permit.epoch(),
                },
                self,
                permit,
            ));
        }
        if permit.members().len() != self.batch.requests().len() {
            return Err(speculative_before_commit_failure(
                M1ServingSpeculativeCompletionErrorV1::PermitRosterCount {
                    expected: self.batch.requests().len(),
                    actual: permit.members().len(),
                },
                self,
                permit,
            ));
        }
        for (lane, (expected, member)) in self
            .batch
            .requests()
            .iter()
            .copied()
            .zip(permit.members())
            .enumerate()
        {
            let actual = member.request();
            if actual != expected {
                return Err(speculative_before_commit_failure(
                    M1ServingSpeculativeCompletionErrorV1::PermitRoster {
                        lane,
                        expected,
                        actual,
                    },
                    self,
                    permit,
                ));
            }
        }

        let mut registry_dispositions = Vec::new();
        if registry_dispositions
            .try_reserve_exact(permit.members().len())
            .is_err()
        {
            return Err(speculative_before_commit_failure(
                M1ServingSpeculativeCompletionErrorV1::HostAllocation,
                self,
                permit,
            ));
        }
        let mut physical_dispositions = Vec::new();
        if physical_dispositions
            .try_reserve_exact(permit.members().len())
            .is_err()
        {
            return Err(speculative_before_commit_failure(
                M1ServingSpeculativeCompletionErrorV1::HostAllocation,
                self,
                permit,
            ));
        }
        let mut next_active_lane = 0;
        for member in permit.members() {
            let expected_physical = match member.status() {
                crate::M1SpeculativeMemberStatusV1::Active => {
                    let actual = permit.next_active_roster().get(next_active_lane).copied();
                    if actual != Some(member.request()) {
                        return Err(speculative_before_commit_failure(
                            M1ServingSpeculativeCompletionErrorV1::PermitNextRoster {
                                lane: next_active_lane,
                                expected: member.request(),
                                actual,
                            },
                            self,
                            permit,
                        ));
                    }
                    next_active_lane += 1;
                    registry_dispositions
                        .push(M1ServingCompletionDispositionV1::Continue(self.plan));
                    M1DeviceKvCompletionDispositionV1::Continue
                }
                crate::M1SpeculativeMemberStatusV1::Completed(_)
                | crate::M1SpeculativeMemberStatusV1::Cancelled(_) => {
                    registry_dispositions.push(M1ServingCompletionDispositionV1::Retire);
                    M1DeviceKvCompletionDispositionV1::Retire
                }
            };
            let actual_physical = member.physical_disposition();
            if actual_physical != expected_physical {
                return Err(speculative_before_commit_failure(
                    M1ServingSpeculativeCompletionErrorV1::PhysicalDisposition {
                        request: member.request(),
                        expected: expected_physical,
                        actual: actual_physical,
                    },
                    self,
                    permit,
                ));
            }
            physical_dispositions.push(actual_physical);
        }
        if next_active_lane != permit.next_active_roster().len() {
            return Err(speculative_before_commit_failure(
                M1ServingSpeculativeCompletionErrorV1::PermitNextRosterCount {
                    expected: next_active_lane,
                    actual: permit.next_active_roster().len(),
                },
                self,
                permit,
            ));
        }

        if let Err(error) = registry.preflight_completion_exact_for(
            self.registry_identity,
            self.epoch,
            &registry_dispositions,
        ) {
            return Err(speculative_before_commit_failure(
                M1ServingSpeculativeCompletionErrorV1::Registry(error),
                self,
                permit,
            ));
        }
        if let Err(error) = coordinator.preflight_prepared_round_commit(&permit) {
            return Err(speculative_before_commit_failure(
                M1ServingSpeculativeCompletionErrorV1::Coordinator(error),
                self,
                permit,
            ));
        }
        let Self {
            registry_identity,
            plan,
            epoch,
            custody,
            batch,
        } = self;
        let custody = match operations.settle_readback(custody, physical_dispositions) {
            Ok(custody) => custody,
            Err(M1ServingPhysicalOperationFailureV1::Retryable { source, custody }) => {
                return Err(Box::new(M1ServingSpeculativeCompletionFailureV1 {
                    error: M1ServingSpeculativeCompletionErrorV1::Operation(source),
                    custody: M1ServingSpeculativeCompletionFailureCustodyV1::BeforeCommit {
                        readback: M1ServingPhysicalReadbackV1 {
                            registry_identity,
                            plan,
                            epoch,
                            custody,
                            batch,
                        },
                        permit,
                    },
                }));
            }
            Err(M1ServingPhysicalOperationFailureV1::Terminal { source, custody }) => {
                return Err(Box::new(M1ServingSpeculativeCompletionFailureV1 {
                    error: M1ServingSpeculativeCompletionErrorV1::Operation(source),
                    custody: M1ServingSpeculativeCompletionFailureCustodyV1::Terminal {
                        registry_identity,
                        plan,
                        epoch,
                        batch,
                        custody,
                        permit,
                    },
                }));
            }
        };
        let outcome = match coordinator.commit_preflighted_round(permit) {
            Ok(outcome) => outcome,
            Err(failure) => {
                let (error, permit) = failure.into_parts();
                return Err(Box::new(M1ServingSpeculativeCompletionFailureV1 {
                    error: M1ServingSpeculativeCompletionErrorV1::Coordinator(error),
                    custody: M1ServingSpeculativeCompletionFailureCustodyV1::AfterPhysical {
                        plan,
                        epoch,
                        custody,
                        batch,
                        permit,
                    },
                }));
            }
        };
        registry.apply_preflighted_completion(epoch, &registry_dispositions);
        Ok((
            batch,
            M1ServingPhysicalQueueCustodyV1::Quiescent { plan, custody },
            outcome,
        ))
    }
}

fn physical_dispositions_from_registry(
    dispositions: &[M1ServingCompletionDispositionV1],
) -> Result<Vec<M1DeviceKvCompletionDispositionV1>, ()> {
    let mut physical = Vec::new();
    physical
        .try_reserve_exact(dispositions.len())
        .map_err(|_| ())?;
    physical.extend(dispositions.iter().map(|disposition| match disposition {
        M1ServingCompletionDispositionV1::Continue(_) => {
            M1DeviceKvCompletionDispositionV1::Continue
        }
        M1ServingCompletionDispositionV1::Retire => M1DeviceKvCompletionDispositionV1::Retire,
    }));
    Ok(physical)
}

#[allow(clippy::unnecessary_box_returns)]
fn registry_completion_retryable<R, T, E>(
    error: M1ServingRegistryCompletionErrorV1<E>,
    readback: M1ServingPhysicalReadbackV1<R>,
) -> Box<M1ServingRegistryCompletionFailureV1<R, T, E>> {
    Box::new(M1ServingRegistryCompletionFailureV1 {
        error,
        custody: M1ServingRegistryCompletionFailureCustodyV1::Retryable(readback),
    })
}

/// Exact completion failure retaining checked readback or terminal lower custody.
#[must_use = "completion rejection retains every physical owner"]
#[derive(Debug)]
pub struct M1ServingRegistryCompletionFailureV1<R, T, E> {
    error: M1ServingRegistryCompletionErrorV1<E>,
    custody: M1ServingRegistryCompletionFailureCustodyV1<R, T>,
}

/// Retryable checked readback or terminal lower completion custody.
#[must_use = "completion failure custody must remain retained"]
#[derive(Debug)]
pub enum M1ServingRegistryCompletionFailureCustodyV1<R, T> {
    Retryable(M1ServingPhysicalReadbackV1<R>),
    Terminal {
        registry_identity: M1ServingRegistryIdentityV1,
        plan: M1ServingPlanV1,
        epoch: CompletionEpoch,
        batch: M1ServingBatchPlanV1,
        custody: T,
    },
}

/// Stable rejection while joining physical and registry completion.
#[derive(Debug, Eq, PartialEq)]
pub enum M1ServingRegistryCompletionErrorV1<E> {
    SpeculativeCoordinatorRequired,
    Registry(M1ServingRegistryErrorV1),
    HostAllocation,
    Operation(E),
}

impl<E: fmt::Debug> fmt::Display for M1ServingRegistryCompletionErrorV1<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "M1 serving registry completion rejected: {self:?}"
        )
    }
}

impl<E: fmt::Debug> std::error::Error for M1ServingRegistryCompletionErrorV1<E> {}

/// Registry completion result that releases quiescent custody only on success.
pub type M1ServingRegistryCompletionResultV1<R, Q, T, E> = Result<
    (M1ServingBatchPlanV1, M1ServingPhysicalQueueCustodyV1<Q>),
    Box<M1ServingRegistryCompletionFailureV1<R, T, E>>,
>;

impl<R, T, E> M1ServingRegistryCompletionFailureV1<R, T, E> {
    #[must_use]
    pub const fn error(&self) -> &M1ServingRegistryCompletionErrorV1<E> {
        &self.error
    }

    pub fn into_parts(
        self,
    ) -> (
        M1ServingRegistryCompletionErrorV1<E>,
        M1ServingRegistryCompletionFailureCustodyV1<R, T>,
    ) {
        (self.error, self.custody)
    }
}

/// Stable rejection while joining physical and speculative completion.
#[derive(Debug, Eq, PartialEq)]
pub enum M1ServingSpeculativeCompletionErrorV1<E> {
    Selection {
        expected: ferric_spec::Qwen3PlanSelection,
        actual: ferric_spec::Qwen3PlanSelection,
    },
    PermitEpoch {
        expected: CompletionEpoch,
        actual: CompletionEpoch,
    },
    PermitRosterCount {
        expected: usize,
        actual: usize,
    },
    PermitRoster {
        lane: usize,
        expected: ferric_spec::RequestId,
        actual: ferric_spec::RequestId,
    },
    PermitNextRosterCount {
        expected: usize,
        actual: usize,
    },
    PermitNextRoster {
        lane: usize,
        expected: ferric_spec::RequestId,
        actual: Option<ferric_spec::RequestId>,
    },
    PhysicalDisposition {
        request: ferric_spec::RequestId,
        expected: M1DeviceKvCompletionDispositionV1,
        actual: M1DeviceKvCompletionDispositionV1,
    },
    Registry(M1ServingRegistryErrorV1),
    Coordinator(M1SpeculativeGenerationLoopErrorV1),
    HostAllocation,
    Operation(E),
}

impl<E: fmt::Debug> fmt::Display for M1ServingSpeculativeCompletionErrorV1<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "M1 serving speculative completion rejected: {self:?}"
        )
    }
}

impl<E: fmt::Debug> std::error::Error for M1ServingSpeculativeCompletionErrorV1<E> {}

/// Speculative completion rejection retaining every physical owner and permit.
#[must_use = "failed speculative completion retains physical custody and permit"]
#[derive(Debug)]
pub struct M1ServingSpeculativeCompletionFailureV1<R, Q, T, E> {
    error: M1ServingSpeculativeCompletionErrorV1<E>,
    custody: M1ServingSpeculativeCompletionFailureCustodyV1<R, Q, T>,
}

/// Pre-settlement retry, lower terminal custody, or post-settlement quarantine.
#[must_use = "speculative completion failure custody must remain retained"]
#[derive(Debug)]
pub enum M1ServingSpeculativeCompletionFailureCustodyV1<R, Q, T> {
    BeforeCommit {
        readback: M1ServingPhysicalReadbackV1<R>,
        permit: M1SpeculativePreflightedRoundV1,
    },
    Terminal {
        registry_identity: M1ServingRegistryIdentityV1,
        plan: M1ServingPlanV1,
        epoch: CompletionEpoch,
        batch: M1ServingBatchPlanV1,
        custody: T,
        permit: M1SpeculativePreflightedRoundV1,
    },
    AfterPhysical {
        plan: M1ServingPlanV1,
        epoch: CompletionEpoch,
        custody: Q,
        batch: M1ServingBatchPlanV1,
        permit: M1SpeculativePreflightedRoundV1,
    },
}

/// Atomic speculative coordinator/registry completion result.
pub type M1ServingSpeculativeCompletionResultV1<R, Q, T, E> = Result<
    (
        M1ServingBatchPlanV1,
        M1ServingPhysicalQueueCustodyV1<Q>,
        M1SpeculativeRoundOutcomeV1,
    ),
    Box<M1ServingSpeculativeCompletionFailureV1<R, Q, T, E>>,
>;

fn speculative_before_commit_failure<R, Q, T, E>(
    error: M1ServingSpeculativeCompletionErrorV1<E>,
    readback: M1ServingPhysicalReadbackV1<R>,
    permit: M1SpeculativePreflightedRoundV1,
) -> Box<M1ServingSpeculativeCompletionFailureV1<R, Q, T, E>> {
    Box::new(M1ServingSpeculativeCompletionFailureV1 {
        error,
        custody: M1ServingSpeculativeCompletionFailureCustodyV1::BeforeCommit { readback, permit },
    })
}

impl<R, Q, T, E> M1ServingSpeculativeCompletionFailureV1<R, Q, T, E> {
    #[must_use]
    pub const fn error(&self) -> &M1ServingSpeculativeCompletionErrorV1<E> {
        &self.error
    }

    pub fn into_parts(
        self,
    ) -> (
        M1ServingSpeculativeCompletionErrorV1<E>,
        M1ServingSpeculativeCompletionFailureCustodyV1<R, Q, T>,
    ) {
        (self.error, self.custody)
    }
}

/// Stable physical completion rejection.
#[derive(Debug)]
pub enum M1ServingPhysicalCompletionErrorV1<E> {
    Epoch {
        expected: CompletionEpoch,
        actual: CompletionEpoch,
    },
    Operation(E),
}

impl<E: fmt::Debug> fmt::Display for M1ServingPhysicalCompletionErrorV1<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "M1 serving physical completion rejected: {self:?}"
        )
    }
}

impl<E: fmt::Debug> std::error::Error for M1ServingPhysicalCompletionErrorV1<E> {}

/// Retryable published custody or terminal lower-layer completion quarantine.
#[must_use = "physical completion failure custody must remain retained"]
#[derive(Debug)]
pub enum M1ServingPhysicalCompletionFailureCustodyV1<P, T> {
    Retryable(M1ServingPhysicalPublishedV1<P>),
    Terminal {
        registry_identity: M1ServingRegistryIdentityV1,
        plan: M1ServingPlanV1,
        epoch: CompletionEpoch,
        batch: M1ServingBatchPlanV1,
        custody: T,
    },
}

/// Physical completion failure retaining every owner and the registry plan.
#[must_use = "failed physical completion retains published or terminal custody"]
#[derive(Debug)]
pub struct M1ServingPhysicalCompletionFailureV1<P, T, E> {
    error: M1ServingPhysicalCompletionErrorV1<E>,
    custody: M1ServingPhysicalCompletionFailureCustodyV1<P, T>,
}

/// Checked-readback result retaining active or terminal custody on failure.
pub type M1ServingPhysicalReadbackResultV1<R, P, T, E> =
    Result<M1ServingPhysicalReadbackV1<R>, Box<M1ServingPhysicalCompletionFailureV1<P, T, E>>>;

impl<P, T, E> M1ServingPhysicalCompletionFailureV1<P, T, E> {
    #[must_use]
    pub const fn error(&self) -> &M1ServingPhysicalCompletionErrorV1<E> {
        &self.error
    }

    pub fn into_parts(
        self,
    ) -> (
        M1ServingPhysicalCompletionErrorV1<E>,
        M1ServingPhysicalCompletionFailureCustodyV1<P, T>,
    ) {
        (self.error, self.custody)
    }
}

/// Stable bridge rejection.
#[derive(Debug)]
pub enum M1ServingPhysicalBridgeErrorV1<E> {
    ActionCustodyDrift,
    ScheduledEpoch {
        expected: CompletionEpoch,
        actual: CompletionEpoch,
    },
    ScheduledMemberCount {
        expected: usize,
        actual: usize,
    },
    ScheduledMember {
        lane: usize,
        expected: ferric_spec::RequestId,
        actual: Option<ferric_spec::RequestId>,
    },
    Registry(M1ServingRegistryErrorV1),
    Operation(E),
}

impl<E: fmt::Debug> fmt::Display for M1ServingPhysicalBridgeErrorV1<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "M1 serving physical bridge rejected: {self:?}")
    }
}

impl<E: fmt::Debug> std::error::Error for M1ServingPhysicalBridgeErrorV1<E> {}

/// Failed physical publication retaining the reservation and all lower custody.
#[must_use = "failed physical publication retains registry and physical custody"]
#[derive(Debug)]
pub struct M1ServingPhysicalBridgeFailureV1<Q, P, T, E> {
    error: M1ServingPhysicalBridgeErrorV1<E>,
    custody: M1ServingPhysicalFailureCustodyV1<Q, P, T>,
}

/// Exhaustive publication-join failure custody.
#[must_use = "physical failure custody must remain retained"]
#[derive(Debug)]
pub enum M1ServingPhysicalFailureCustodyV1<Q, P, T> {
    Retryable(M1ServingPhysicalRetryablePublicationV1<Q>),
    UnmatchedPublished(M1ServingPhysicalUnmatchedPublishedV1<P>),
    UnrecordedPublished(M1ServingPhysicalUnrecordedPublishedV1<P>),
    Terminal(M1ServingPhysicalTerminalPublicationV1<T>),
}

impl<Q, P, T, E> M1ServingPhysicalBridgeFailureV1<Q, P, T, E> {
    #[must_use]
    pub const fn error(&self) -> &M1ServingPhysicalBridgeErrorV1<E> {
        &self.error
    }

    pub fn into_parts(
        self,
    ) -> (
        M1ServingPhysicalBridgeErrorV1<E>,
        M1ServingPhysicalFailureCustodyV1<Q, P, T>,
    ) {
        (self.error, self.custody)
    }
}

/// Pre-publication retry owner. Its reservation may safely be aborted because
/// no physical queue generation was published.
#[must_use = "retryable publication custody must be retried or aborted"]
#[derive(Debug)]
pub struct M1ServingPhysicalRetryablePublicationV1<Q> {
    custody: M1ServingPhysicalQueueCustodyV1<Q>,
    reservation: M1ServingPublicationReservationV1,
}

impl<Q> M1ServingPhysicalRetryablePublicationV1<Q> {
    pub fn into_parts(
        self,
    ) -> (
        M1ServingPhysicalQueueCustodyV1<Q>,
        M1ServingPublicationReservationV1,
    ) {
        (self.custody, self.reservation)
    }

    /// Aborts the still-pre-publication registry reservation and returns the
    /// unchanged quiescent queue owner.
    ///
    /// # Errors
    ///
    /// Returns the unchanged queue owner and reservation failure when the
    /// registry no longer recognizes the exact live reservation.
    #[allow(clippy::result_large_err)]
    pub fn abort<const C: usize>(
        self,
        registry: &mut M1ServingRegistryV1<C>,
    ) -> Result<M1ServingPhysicalQueueCustodyV1<Q>, M1ServingPhysicalAbortFailureV1<Q>> {
        match registry.abort_publication(self.reservation) {
            Ok(()) => Ok(self.custody),
            Err(failure) => Err(M1ServingPhysicalAbortFailureV1 {
                custody: self.custody,
                failure,
            }),
        }
    }
}

/// Failed retryable abort retaining the queue and reservation failure owner.
#[must_use]
#[derive(Debug)]
pub struct M1ServingPhysicalAbortFailureV1<Q> {
    custody: M1ServingPhysicalQueueCustodyV1<Q>,
    failure: M1ServingPublicationFailureV1,
}

impl<Q> M1ServingPhysicalAbortFailureV1<Q> {
    pub fn into_parts(
        self,
    ) -> (
        M1ServingPhysicalQueueCustodyV1<Q>,
        M1ServingPublicationFailureV1,
    ) {
        (self.custody, self.failure)
    }
}

/// A physically published owner whose scheduler authority did not match the
/// registry reservation. It intentionally exposes neither completion nor
/// registry-record operations.
#[must_use = "mismatched published custody must remain quarantined"]
#[derive(Debug)]
pub struct M1ServingPhysicalUnmatchedPublishedV1<P> {
    raw: M1ServingPhysicalRawPublishedV1<P>,
    reservation: M1ServingPublicationReservationV1,
}

impl<P> M1ServingPhysicalUnmatchedPublishedV1<P> {
    #[must_use]
    pub const fn epoch(&self) -> CompletionEpoch {
        self.raw.epoch
    }

    pub const fn reservation(&self) -> &M1ServingPublicationReservationV1 {
        &self.reservation
    }
}

/// Physically published custody whose exact scheduler authority was validated,
/// but whose defensive registry record failed. It can only retry that record;
/// redispatch and lower completion are unavailable.
#[must_use = "unrecorded published custody must retry the registry record"]
#[derive(Debug)]
pub struct M1ServingPhysicalUnrecordedPublishedV1<P> {
    raw: M1ServingPhysicalRawPublishedV1<P>,
    reservation: M1ServingPublicationReservationV1,
}

impl<P> M1ServingPhysicalUnrecordedPublishedV1<P> {
    #[must_use]
    pub const fn epoch(&self) -> CompletionEpoch {
        self.raw.epoch
    }

    /// Retries only the defensive registry record after physical publication.
    ///
    /// # Errors
    ///
    /// Returns the unchanged published owner when the registry still rejects
    /// the exact reservation; no physical operation is repeated.
    pub fn retry_record<const C: usize>(
        self,
        registry: &mut M1ServingRegistryV1<C>,
    ) -> Result<M1ServingPhysicalPublishedV1<P>, Box<M1ServingPhysicalRecordRetryFailureV1<P>>>
    {
        match registry.record_publication(self.reservation) {
            Ok(()) => Ok(self.raw.activate()),
            Err(failure) => Err(Box::new(M1ServingPhysicalRecordRetryFailureV1 {
                error: failure.error(),
                published: Self {
                    raw: self.raw,
                    reservation: failure.into_reservation(),
                },
            })),
        }
    }
}

/// Failed defensive registry-record retry retaining the published owner.
#[must_use]
#[derive(Debug)]
pub struct M1ServingPhysicalRecordRetryFailureV1<P> {
    error: M1ServingRegistryErrorV1,
    published: M1ServingPhysicalUnrecordedPublishedV1<P>,
}

impl<P> M1ServingPhysicalRecordRetryFailureV1<P> {
    #[must_use]
    pub const fn error(&self) -> M1ServingRegistryErrorV1 {
        self.error
    }

    pub fn into_published(self) -> M1ServingPhysicalUnrecordedPublishedV1<P> {
        self.published
    }
}

/// Terminal lower-layer quarantine with its reservation deliberately sealed.
/// There is no abort-to-Ready path after irreversible physical progress.
#[must_use = "terminal publication custody and reservation must remain retained"]
#[derive(Debug)]
pub struct M1ServingPhysicalTerminalPublicationV1<T> {
    reservation: M1ServingPublicationReservationV1,
    batch: M1ServingBatchPlanV1,
    custody: T,
}

impl<T> M1ServingPhysicalTerminalPublicationV1<T> {
    pub const fn reservation(&self) -> &M1ServingPublicationReservationV1 {
        &self.reservation
    }

    pub const fn batch(&self) -> &M1ServingBatchPlanV1 {
        &self.batch
    }

    #[must_use]
    pub const fn lower_custody(&self) -> &T {
        &self.custody
    }
}

struct M1ServingPhysicalRouteFailureV1<Q, T, E> {
    error: M1ServingPhysicalBridgeErrorV1<E>,
    custody: M1ServingPhysicalRouteFailureCustodyV1<Q, T>,
    batch: M1ServingBatchPlanV1,
}

enum M1ServingPhysicalRouteFailureCustodyV1<Q, T> {
    Retryable(M1ServingPhysicalQueueCustodyV1<Q>),
    Terminal(T),
}

impl<Q, T, E> M1ServingPhysicalRouteFailureV1<Q, T, E> {
    fn into_parts(
        self,
    ) -> (
        M1ServingPhysicalBridgeErrorV1<E>,
        M1ServingPhysicalRouteFailureCustodyV1<Q, T>,
        M1ServingBatchPlanV1,
    ) {
        (self.error, self.custody, self.batch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        completed_readback_join::check_m1_completed_output_v1, m1_completion_output_shape_v1,
        CompletionWireExpectation, CompletionWireSemanticExpectation, M1ObservedCompletionImageV1,
        M1ServingRequestPhaseV1,
    };
    use ferric_qwen_kernels::logits::Qwen3LogitsCompactRecordLayoutV1 as CompletionLayout;
    use ferric_spec::{
        Identity, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection,
        RequestId, StepPlan, TokenId,
    };

    #[derive(Debug, Eq, PartialEq)]
    struct Custody(u64);

    #[derive(Debug, Eq, PartialEq)]
    struct PublishedCustody {
        value: u64,
        scheduled: M1ScheduledDispatchV1,
    }

    #[derive(Clone, Copy, Default)]
    enum ScheduleFault {
        #[default]
        None,
        Epoch,
        Count,
        Order,
    }

    #[derive(Default)]
    struct Operations {
        calls: Vec<&'static str>,
        settled: Vec<Vec<M1DeviceKvCompletionDispositionV1>>,
        fail: bool,
        terminal: bool,
        schedule_fault: ScheduleFault,
    }

    impl Operations {
        fn published(&self, value: u64, batch: &M1ServingBatchPlanV1) -> PublishedCustody {
            let mut epoch = batch.epoch();
            let mut members = batch.requests().to_vec();
            match self.schedule_fault {
                ScheduleFault::None => {}
                ScheduleFault::Epoch => {
                    epoch = CompletionEpoch::new(epoch.value() + 1);
                }
                ScheduleFault::Count => members.push(RequestId::new(7, 77)),
                ScheduleFault::Order => members.swap(0, 1),
            }
            PublishedCustody {
                value,
                scheduled: M1ScheduledDispatchV1::for_test(epoch, &members),
            }
        }
    }

    impl M1ServingPhysicalOperationsV1 for Operations {
        type Quiescent = Custody;
        type Published = PublishedCustody;
        type Readback = PublishedCustody;
        type Error = &'static str;
        type TerminalCustody = Custody;

        fn scheduled_dispatch<'a>(
            &self,
            custody: &'a Self::Published,
        ) -> &'a M1ScheduledDispatchV1 {
            &custody.scheduled
        }

        fn fresh_launch(
            &mut self,
            batch: &M1ServingBatchPlanV1,
        ) -> Result<
            PublishedCustody,
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
                Ok(self.published(1, batch))
            }
        }

        fn same_shape_rearm(
            &mut self,
            custody: Custody,
            batch: &M1ServingBatchPlanV1,
        ) -> Result<
            PublishedCustody,
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
                Ok(self.published(custody.0 + 1, batch))
            }
        }

        fn quiescent_rollover(
            &mut self,
            custody: Custody,
            _: M1ServingPlanV1,
            _: M1ServingPlanV1,
            _: M1ServingRolloverReasonV1,
            batch: &M1ServingBatchPlanV1,
        ) -> Result<
            PublishedCustody,
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
                Ok(self.published(custody.0 + 10, batch))
            }
        }

        fn read_published(
            &mut self,
            custody: PublishedCustody,
            _: CompletionEpoch,
            _: &M1ServingBatchPlanV1,
        ) -> Result<
            PublishedCustody,
            M1ServingPhysicalOperationFailureV1<
                PublishedCustody,
                Self::TerminalCustody,
                Self::Error,
            >,
        > {
            self.calls.push("read");
            if self.terminal {
                Err(M1ServingPhysicalOperationFailureV1::Terminal {
                    source: "read",
                    custody: Custody(custody.value),
                })
            } else if self.fail {
                Err(M1ServingPhysicalOperationFailureV1::Retryable {
                    source: "read",
                    custody,
                })
            } else {
                Ok(custody)
            }
        }

        fn checked_completion<'a>(&self, _: &'a Self::Readback) -> &'a M1CheckedCompletionOutputV1 {
            panic!("bridge-only operation fake has no checked output")
        }

        fn settle_readback(
            &mut self,
            custody: Self::Readback,
            dispositions: Vec<M1DeviceKvCompletionDispositionV1>,
        ) -> M1ServingPhysicalOperationResultV1<
            Self::Quiescent,
            Self::Readback,
            Self::TerminalCustody,
            Self::Error,
        > {
            self.calls.push("settle");
            self.settled.push(dispositions);
            if self.terminal {
                Err(M1ServingPhysicalOperationFailureV1::Terminal {
                    source: "settle",
                    custody: Custody(custody.value),
                })
            } else if self.fail {
                Err(M1ServingPhysicalOperationFailureV1::Retryable {
                    source: "settle",
                    custody,
                })
            } else {
                Ok(Custody(custody.value))
            }
        }
    }

    fn pair(mode: Qwen3ExecutionMode, bucket: Qwen3PlanBucket) -> M1ServingPlanV1 {
        let (draft_mode, draft_bucket) = match (mode, bucket) {
            (
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K4C8192
                | Qwen3PlanBucket::SpeculativeS1K8C8192
                | Qwen3PlanBucket::SpeculativeS1K16C8192,
            ) => (Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS1C8192),
            (Qwen3ExecutionMode::Speculative, Qwen3PlanBucket::SpeculativeS8K4C8192) => {
                (Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS8C8192)
            }
            _ => (mode, bucket),
        };
        M1ServingPlanV1::new(
            Qwen3PlanSelection {
                role: Qwen3ModelRole::Target8B,
                mode,
                bucket,
            },
            Qwen3PlanSelection {
                role: Qwen3ModelRole::Draft06B,
                mode: draft_mode,
                bucket: draft_bucket,
            },
        )
        .unwrap()
    }

    fn fresh_context(
        plan: M1ServingPlanV1,
        request_count: usize,
    ) -> (
        M1ServingRegistryV1<8>,
        M1ServingPublicationReservationV1,
        Vec<RequestId>,
    ) {
        let mut registry = M1ServingRegistryV1::<8>::new().unwrap();
        let requests = (0..request_count)
            .map(|slot| RequestId::new(u32::try_from(slot).unwrap(), 1))
            .collect::<Vec<_>>();
        for request in requests.iter().copied() {
            registry.admit(request, plan).unwrap();
        }
        let batch = registry.plan_next().unwrap().unwrap();
        let reservation = registry.reserve_publication(batch).unwrap();
        (registry, reservation, requests)
    }

    fn encode_speculative_completion(
        request: RequestId,
        epoch: CompletionEpoch,
        plan_id: Identity,
        accepted: u8,
        emitted: &[TokenId],
    ) -> Box<[u8]> {
        let mut bytes = vec![0; CompletionLayout::RECORD_BYTES_USIZE];
        bytes[CompletionLayout::REQUEST_SLOT_OFFSET..CompletionLayout::REQUEST_SLOT_OFFSET + 4]
            .copy_from_slice(&request.slot().to_le_bytes());
        bytes[CompletionLayout::REQUEST_GENERATION_OFFSET
            ..CompletionLayout::REQUEST_GENERATION_OFFSET + 4]
            .copy_from_slice(&request.generation().to_le_bytes());
        bytes[CompletionLayout::COMPLETION_EPOCH_OFFSET
            ..CompletionLayout::COMPLETION_EPOCH_OFFSET + 8]
            .copy_from_slice(&epoch.value().to_le_bytes());
        bytes[CompletionLayout::PLAN_IDENTITY_OFFSET
            ..CompletionLayout::PLAN_IDENTITY_OFFSET + CompletionLayout::PLAN_IDENTITY_BYTES]
            .copy_from_slice(plan_id.as_bytes());
        bytes[CompletionLayout::ACCEPTED_DRAFT_TOKENS_OFFSET] = accepted;
        bytes[CompletionLayout::EMITTED_TOKEN_COUNT_OFFSET] = u8::try_from(emitted.len()).unwrap();
        for (index, token) in emitted.iter().enumerate() {
            let offset = CompletionLayout::token_offset(index).unwrap();
            bytes[offset..offset + 4].copy_from_slice(&token.to_le_bytes());
        }
        bytes.into_boxed_slice()
    }

    fn speculative_permit(
        coordinator: &M1SpeculativeGenerationLoopV1,
        selection: Qwen3PlanSelection,
        request: RequestId,
        epoch: CompletionEpoch,
    ) -> M1SpeculativePreflightedRoundV1 {
        let plan_id = Identity::new([71; 32]);
        let scheduled = M1ScheduledDispatchV1::for_test(epoch, &[request]);
        let plan = StepPlan::new(request, epoch, plan_id, selection);
        let expectations = [CompletionWireExpectation::new(
            &plan,
            CompletionWireSemanticExpectation::Speculative {
                draft_tokens: &[3, 4, 5, 6],
                target_choices: &[3, 4, 9, 7, 8],
            },
        )];
        let observed = M1ObservedCompletionImageV1::from_bytes_for_test(
            m1_completion_output_shape_v1(selection).unwrap(),
            selection,
            &scheduled,
            19,
            5,
            384,
            encode_speculative_completion(request, epoch, plan_id, 2, &[3, 4, 9]),
        )
        .unwrap();
        let checked =
            check_m1_completed_output_v1(&observed, selection, &scheduled, &expectations).unwrap();
        let binding = coordinator.bind_round(0, epoch, &[request]).unwrap();
        coordinator
            .preflight_checked_round(
                binding,
                &checked,
                &[crate::M1SpeculativeMemberControlV1::continuing(request)],
            )
            .unwrap()
    }

    #[test]
    fn scheduler_authority_epoch_count_and_order_must_match() {
        let plan = pair(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS8T128);
        for (fault, expected_kind) in [
            (ScheduleFault::Epoch, "epoch"),
            (ScheduleFault::Count, "count"),
            (ScheduleFault::Order, "order"),
        ] {
            let (mut registry, reservation, requests) = fresh_context(plan, 2);
            let mut operations = Operations {
                schedule_fault: fault,
                ..Operations::default()
            };
            let failure = M1ServingPhysicalQueueCustodyV1::Vacant
                .publish(reservation, &mut registry, &mut operations)
                .unwrap_err();
            let (error, custody) = failure.into_parts();
            match (expected_kind, error) {
                ("epoch", M1ServingPhysicalBridgeErrorV1::ScheduledEpoch { .. })
                | ("count", M1ServingPhysicalBridgeErrorV1::ScheduledMemberCount { .. })
                | ("order", M1ServingPhysicalBridgeErrorV1::ScheduledMember { lane: 0, .. }) => {}
                (_, other) => panic!("unexpected schedule rejection: {other:?}"),
            }
            assert!(matches!(
                custody,
                M1ServingPhysicalFailureCustodyV1::UnmatchedPublished(_)
            ));
            for request in requests {
                assert_eq!(
                    registry.phase(request),
                    Some(M1ServingRequestPhaseV1::Ready)
                );
            }
            assert_eq!(operations.calls, ["fresh"]);
        }
    }

    #[test]
    fn retryable_operation_failure_retains_abortable_reservation_and_queue() {
        let plan = pair(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128);
        let (mut registry, reservation, requests) = fresh_context(plan, 1);
        let mut operations = Operations {
            fail: true,
            ..Operations::default()
        };
        let failure = M1ServingPhysicalQueueCustodyV1::Vacant
            .publish(reservation, &mut registry, &mut operations)
            .unwrap_err();
        let (error, custody) = failure.into_parts();
        assert!(matches!(
            error,
            M1ServingPhysicalBridgeErrorV1::Operation("fresh")
        ));
        let M1ServingPhysicalFailureCustodyV1::Retryable(retryable) = custody else {
            panic!("retryable physical rejection lost retry custody");
        };
        assert!(matches!(
            retryable.abort(&mut registry).unwrap(),
            M1ServingPhysicalQueueCustodyV1::Vacant
        ));
        assert_eq!(
            registry.phase(requests[0]),
            Some(M1ServingRequestPhaseV1::Ready)
        );
        assert!(registry.plan_next().unwrap().is_some());
    }

    #[test]
    fn successful_publication_immediately_moves_registry_ready_to_in_flight() {
        let plan = pair(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128);
        let (mut registry, reservation, requests) = fresh_context(plan, 1);
        let epoch = reservation.epoch();
        let mut operations = Operations::default();
        let active = M1ServingPhysicalQueueCustodyV1::Vacant
            .publish(reservation, &mut registry, &mut operations)
            .unwrap();
        assert_eq!(active.epoch(), epoch);
        assert_eq!(
            registry.phase(requests[0]),
            Some(M1ServingRequestPhaseV1::InFlight { epoch })
        );
        assert_eq!(operations.calls, ["fresh"]);
    }

    #[test]
    fn wrong_registry_is_rejected_before_physical_publication() {
        let plan = pair(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128);
        let (mut owning_registry, reservation, _) = fresh_context(plan, 1);
        let mut wrong_registry = M1ServingRegistryV1::<8>::new().unwrap();
        let epoch = reservation.epoch();
        let mut operations = Operations::default();
        let failure = M1ServingPhysicalQueueCustodyV1::Vacant
            .publish(reservation, &mut wrong_registry, &mut operations)
            .unwrap_err();
        let (error, custody) = failure.into_parts();
        assert!(matches!(
            error,
            M1ServingPhysicalBridgeErrorV1::Registry(
                M1ServingRegistryErrorV1::RegistryIdentityMismatch
            )
        ));
        let M1ServingPhysicalFailureCustodyV1::Retryable(retryable) = custody else {
            panic!("registry preflight lost retryable publication custody");
        };
        assert!(operations.calls.is_empty());
        let (queue, reservation) = retryable.into_parts();
        let active = queue
            .publish(reservation, &mut owning_registry, &mut operations)
            .unwrap();
        assert_eq!(active.epoch(), epoch);
        assert_eq!(operations.calls, ["fresh"]);
    }

    #[test]
    fn completed_owner_cannot_advance_an_identical_wrong_registry() {
        let plan = pair(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128);
        let (mut owning_registry, reservation, _) = fresh_context(plan, 1);
        let (mut wrong_registry, _wrong_reservation, _) = fresh_context(plan, 1);
        let epoch = reservation.epoch();
        let mut operations = Operations::default();
        let readback = M1ServingPhysicalQueueCustodyV1::Vacant
            .publish(reservation, &mut owning_registry, &mut operations)
            .unwrap()
            .read_physical(epoch, &mut operations)
            .unwrap();
        let failure = readback
            .complete_exact(
                &mut wrong_registry,
                &[M1ServingCompletionDispositionV1::Retire],
                &mut operations,
            )
            .unwrap_err();
        assert_eq!(
            failure.error(),
            &M1ServingRegistryCompletionErrorV1::Registry(
                M1ServingRegistryErrorV1::RegistryIdentityMismatch
            )
        );
        let (_, custody) = failure.into_parts();
        let M1ServingRegistryCompletionFailureCustodyV1::Retryable(readback) = custody else {
            panic!("wrong registry lost retryable checked readback");
        };
        let _ = readback
            .complete_exact(
                &mut owning_registry,
                &[M1ServingCompletionDispositionV1::Retire],
                &mut operations,
            )
            .unwrap();
        assert!(wrong_registry.has_publication_reservation());
        assert_eq!(operations.calls, ["fresh", "read", "settle"]);
    }

    #[test]
    fn registry_completion_rejection_retains_completed_physical_custody_for_retry() {
        let plan = pair(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128);
        let (mut registry, reservation, requests) = fresh_context(plan, 1);
        let epoch = reservation.epoch();
        let mut operations = Operations::default();
        let readback = M1ServingPhysicalQueueCustodyV1::Vacant
            .publish(reservation, &mut registry, &mut operations)
            .unwrap()
            .read_physical(epoch, &mut operations)
            .unwrap();

        let failure = readback
            .complete_exact(&mut registry, &[], &mut operations)
            .unwrap_err();
        assert_eq!(
            failure.error(),
            &M1ServingRegistryCompletionErrorV1::Registry(
                M1ServingRegistryErrorV1::CompletionDispositionCount
            )
        );
        assert_eq!(
            registry.phase(requests[0]),
            Some(M1ServingRequestPhaseV1::InFlight { epoch })
        );
        assert!(registry.has_in_flight_batch());
        assert_eq!(operations.calls, ["fresh", "read"]);

        let (_, failure_custody) = failure.into_parts();
        let M1ServingRegistryCompletionFailureCustodyV1::Retryable(readback) = failure_custody
        else {
            panic!("registry preflight lost retryable checked readback");
        };
        let (batch, custody) = readback
            .complete_exact(
                &mut registry,
                &[M1ServingCompletionDispositionV1::Retire],
                &mut operations,
            )
            .unwrap();
        assert_eq!(batch.epoch(), epoch);
        assert!(matches!(
            custody,
            M1ServingPhysicalQueueCustodyV1::Quiescent {
                plan: released_plan,
                custody: Custody(1),
            } if released_plan == plan
        ));
        assert_eq!(
            registry.phase(requests[0]),
            Some(M1ServingRequestPhaseV1::Retired {
                quiescence: crate::M1ServingQuiescenceV1::Completed(epoch),
            })
        );
        assert!(!registry.has_in_flight_batch());
        assert_eq!(operations.calls, ["fresh", "read", "settle"]);
    }

    #[test]
    fn successful_registry_completion_transitions_before_quiescent_release() {
        let plan = pair(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128);
        let (mut registry, reservation, requests) = fresh_context(plan, 1);
        let epoch = reservation.epoch();
        let mut operations = Operations::default();
        let readback = M1ServingPhysicalQueueCustodyV1::Vacant
            .publish(reservation, &mut registry, &mut operations)
            .unwrap()
            .read_physical(epoch, &mut operations)
            .unwrap();

        let (_, custody) = readback
            .complete_exact(
                &mut registry,
                &[M1ServingCompletionDispositionV1::Retire],
                &mut operations,
            )
            .unwrap();
        assert_eq!(
            registry.phase(requests[0]),
            Some(M1ServingRequestPhaseV1::Retired {
                quiescence: crate::M1ServingQuiescenceV1::Completed(epoch),
            })
        );
        assert!(!registry.has_in_flight_batch());
        assert!(matches!(
            custody,
            M1ServingPhysicalQueueCustodyV1::Quiescent {
                plan: released_plan,
                custody: Custody(1),
            } if released_plan == plan
        ));
    }

    #[test]
    fn speculative_physical_completion_cannot_bypass_coordinator_commit() {
        let prefill = pair(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128);
        let speculative = pair(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
        );
        let (mut registry, reservation, requests) = fresh_context(prefill, 1);
        let mut operations = Operations::default();
        let prefill_epoch = reservation.epoch();
        let prefill_readback = M1ServingPhysicalQueueCustodyV1::Vacant
            .publish(reservation, &mut registry, &mut operations)
            .unwrap()
            .read_physical(prefill_epoch, &mut operations)
            .unwrap();
        let (_, queue) = prefill_readback
            .complete_exact(
                &mut registry,
                &[M1ServingCompletionDispositionV1::Continue(speculative)],
                &mut operations,
            )
            .unwrap();

        let batch = registry.plan_next().unwrap().unwrap();
        let reservation = registry.reserve_publication(batch).unwrap();
        let epoch = reservation.epoch();
        let speculative_readback = queue
            .publish(reservation, &mut registry, &mut operations)
            .unwrap()
            .read_physical(epoch, &mut operations)
            .unwrap();
        let failure = speculative_readback
            .complete_exact(
                &mut registry,
                &[M1ServingCompletionDispositionV1::Retire],
                &mut operations,
            )
            .unwrap_err();

        assert_eq!(
            failure.error(),
            &M1ServingRegistryCompletionErrorV1::SpeculativeCoordinatorRequired
        );
        assert_eq!(
            registry.phase(requests[0]),
            Some(M1ServingRequestPhaseV1::InFlight { epoch })
        );
        assert!(registry.has_in_flight_batch());
        assert_eq!(
            operations.calls,
            ["fresh", "read", "settle", "rollover", "read"]
        );
        let (_, failure_custody) = failure.into_parts();
        let M1ServingRegistryCompletionFailureCustodyV1::Retryable(readback) = failure_custody
        else {
            panic!("speculative bypass rejection lost checked readback");
        };
        assert_eq!(readback.epoch(), epoch);
        assert_eq!(readback.plan(), speculative);
    }

    #[test]
    fn terminal_readback_quarantines_lower_custody_and_leaves_registry_in_flight() {
        let plan = pair(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128);
        let (mut registry, reservation, requests) = fresh_context(plan, 1);
        let epoch = reservation.epoch();
        let mut operations = Operations::default();
        let published = M1ServingPhysicalQueueCustodyV1::Vacant
            .publish(reservation, &mut registry, &mut operations)
            .unwrap();
        operations.terminal = true;

        let failure = published.read_physical(epoch, &mut operations).unwrap_err();
        let (error, custody) = failure.into_parts();
        assert!(matches!(
            error,
            M1ServingPhysicalCompletionErrorV1::Operation("read")
        ));
        let M1ServingPhysicalCompletionFailureCustodyV1::Terminal {
            plan: failed_plan,
            epoch: failed_epoch,
            batch,
            custody,
            ..
        } = custody
        else {
            panic!("terminal read failure exposed retryable published custody");
        };
        assert_eq!(failed_plan, plan);
        assert_eq!(failed_epoch, epoch);
        assert_eq!(batch.epoch(), epoch);
        assert_eq!(batch.requests(), requests);
        assert_eq!(custody, Custody(1));
        assert_eq!(operations.calls, ["fresh", "read"]);
        assert!(operations.settled.is_empty());
        assert_eq!(
            registry.phase(requests[0]),
            Some(M1ServingRequestPhaseV1::InFlight { epoch })
        );
        assert!(registry.has_in_flight_batch());
    }

    #[test]
    fn retryable_ordinary_settlement_retains_readback_until_exact_retry_succeeds() {
        let plan = pair(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128);
        let (mut registry, reservation, requests) = fresh_context(plan, 1);
        let epoch = reservation.epoch();
        let mut operations = Operations::default();
        let readback = M1ServingPhysicalQueueCustodyV1::Vacant
            .publish(reservation, &mut registry, &mut operations)
            .unwrap()
            .read_physical(epoch, &mut operations)
            .unwrap();
        operations.fail = true;

        let failure = readback
            .complete_exact(
                &mut registry,
                &[M1ServingCompletionDispositionV1::Retire],
                &mut operations,
            )
            .unwrap_err();
        assert_eq!(
            failure.error(),
            &M1ServingRegistryCompletionErrorV1::Operation("settle")
        );
        assert_eq!(
            registry.phase(requests[0]),
            Some(M1ServingRequestPhaseV1::InFlight { epoch })
        );
        assert!(registry.has_in_flight_batch());
        assert_eq!(
            operations.settled,
            [vec![M1DeviceKvCompletionDispositionV1::Retire]]
        );

        let (_, custody) = failure.into_parts();
        let M1ServingRegistryCompletionFailureCustodyV1::Retryable(readback) = custody else {
            panic!("retryable settlement lost checked readback custody");
        };
        operations.fail = false;
        let (_, quiescent) = readback
            .complete_exact(
                &mut registry,
                &[M1ServingCompletionDispositionV1::Retire],
                &mut operations,
            )
            .unwrap();
        assert!(matches!(
            quiescent,
            M1ServingPhysicalQueueCustodyV1::Quiescent {
                plan: released_plan,
                custody: Custody(1),
            } if released_plan == plan
        ));
        assert_eq!(
            operations.settled,
            [
                vec![M1DeviceKvCompletionDispositionV1::Retire],
                vec![M1DeviceKvCompletionDispositionV1::Retire],
            ]
        );
        assert_eq!(
            registry.phase(requests[0]),
            Some(M1ServingRequestPhaseV1::Retired {
                quiescence: crate::M1ServingQuiescenceV1::Completed(epoch),
            })
        );
        assert!(!registry.has_in_flight_batch());
    }

    #[test]
    fn terminal_ordinary_settlement_quarantines_lower_custody_before_registry_mutation() {
        let plan = pair(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128);
        let (mut registry, reservation, requests) = fresh_context(plan, 1);
        let epoch = reservation.epoch();
        let mut operations = Operations::default();
        let readback = M1ServingPhysicalQueueCustodyV1::Vacant
            .publish(reservation, &mut registry, &mut operations)
            .unwrap()
            .read_physical(epoch, &mut operations)
            .unwrap();
        operations.terminal = true;

        let failure = readback
            .complete_exact(
                &mut registry,
                &[M1ServingCompletionDispositionV1::Retire],
                &mut operations,
            )
            .unwrap_err();
        let (error, custody) = failure.into_parts();
        assert_eq!(
            error,
            M1ServingRegistryCompletionErrorV1::Operation("settle")
        );
        let M1ServingRegistryCompletionFailureCustodyV1::Terminal {
            plan: failed_plan,
            epoch: failed_epoch,
            batch,
            custody,
            ..
        } = custody
        else {
            panic!("terminal settlement exposed retryable checked readback");
        };
        assert_eq!(failed_plan, plan);
        assert_eq!(failed_epoch, epoch);
        assert_eq!(batch.requests(), requests);
        assert_eq!(custody, Custody(1));
        assert_eq!(operations.calls, ["fresh", "read", "settle"]);
        assert_eq!(
            operations.settled,
            [vec![M1DeviceKvCompletionDispositionV1::Retire]]
        );
        assert_eq!(
            registry.phase(requests[0]),
            Some(M1ServingRequestPhaseV1::InFlight { epoch })
        );
        assert!(registry.has_in_flight_batch());
    }

    #[test]
    fn terminal_speculative_settlement_quarantines_permit_without_logical_commit() {
        let prefill = pair(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128);
        let plan = pair(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
        );
        let (mut registry, reservation, requests) = fresh_context(prefill, 1);
        let request = requests[0];
        let prefill_epoch = reservation.epoch();
        let mut operations = Operations::default();
        let prefill_readback = M1ServingPhysicalQueueCustodyV1::Vacant
            .publish(reservation, &mut registry, &mut operations)
            .unwrap()
            .read_physical(prefill_epoch, &mut operations)
            .unwrap();
        let (_, queue) = prefill_readback
            .complete_exact(
                &mut registry,
                &[M1ServingCompletionDispositionV1::Continue(plan)],
                &mut operations,
            )
            .unwrap();
        let batch = registry.plan_next().unwrap().unwrap();
        let reservation = registry.reserve_publication(batch).unwrap();
        let epoch = reservation.epoch();
        let target = plan.target();
        let seed = crate::M1SpeculativeMemberSeedV1::new(
            request,
            70,
            10,
            10,
            crate::M1SpeculativeGenerationPolicyV1::new(32, &[999]).unwrap(),
        );
        let mut coordinator = M1SpeculativeGenerationLoopV1::new(target, &[seed]).unwrap();
        let permit = speculative_permit(&coordinator, target, request, epoch);
        let readback = queue
            .publish(reservation, &mut registry, &mut operations)
            .unwrap()
            .read_physical(epoch, &mut operations)
            .unwrap();
        operations.terminal = true;

        let failure = readback
            .commit_speculative(&mut registry, &mut coordinator, permit, &mut operations)
            .unwrap_err();
        let (error, custody) = failure.into_parts();
        assert_eq!(
            error,
            M1ServingSpeculativeCompletionErrorV1::Operation("settle")
        );
        let M1ServingSpeculativeCompletionFailureCustodyV1::Terminal {
            plan: failed_plan,
            epoch: failed_epoch,
            batch,
            custody,
            permit,
            ..
        } = custody
        else {
            panic!("terminal speculative settlement lost terminal quarantine");
        };
        assert_eq!(failed_plan, plan);
        assert_eq!(failed_epoch, epoch);
        assert_eq!(batch.requests(), requests);
        assert_eq!(custody, Custody(11));
        assert_eq!(permit.selection(), target);
        assert_eq!(permit.epoch(), epoch);
        assert_eq!(permit.members().len(), 1);
        assert_eq!(permit.members()[0].request(), request);
        assert_eq!(coordinator.next_round(), 0);
        assert_eq!(coordinator.last_epoch(), None);
        assert_eq!(
            operations.settled,
            [
                vec![M1DeviceKvCompletionDispositionV1::Continue],
                vec![M1DeviceKvCompletionDispositionV1::Continue],
            ]
        );
        assert_eq!(
            registry.phase(request),
            Some(M1ServingRequestPhaseV1::InFlight { epoch })
        );
        assert!(registry.has_in_flight_batch());
    }

    #[test]
    fn terminal_physical_failure_seals_the_live_reservation() {
        let plan = pair(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128);
        let (mut registry, reservation, _) = fresh_context(plan, 1);
        let epoch = reservation.epoch();
        let mut operations = Operations {
            terminal: true,
            ..Operations::default()
        };
        let failure = M1ServingPhysicalQueueCustodyV1::Vacant
            .publish(reservation, &mut registry, &mut operations)
            .unwrap_err();
        let (_, custody) = failure.into_parts();
        let M1ServingPhysicalFailureCustodyV1::Terminal(terminal) = custody else {
            panic!("terminal failure lost sealed custody");
        };
        assert_eq!(terminal.reservation().epoch(), epoch);
        assert_eq!(terminal.lower_custody(), &Custody(0));
        assert!(matches!(
            registry.plan_next(),
            Err(M1ServingRegistryErrorV1::PublicationReservationActive)
        ));
    }
}
