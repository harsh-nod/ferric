//! Production authenticated repeated speculative execution.
//!
//! This bridge can start only from an authenticated released queue produced by
//! the normal completion path. It never constructs program, queue-currentness,
//! completion, or verifier authority.
//! Unit tests cover the public production entry shape and custody transitions
//! without manufacturing that authority; hardware-backed execution still has
//! to enter through the normal verified-launch path.

use core::fmt;

use ferric_spec::{completion::CompletionEpoch, Qwen3PlanSelection, RequestId};

use crate::{
    prepare_m1_authenticated_long_lived_queue_rearm_v1,
    reserve_m1_authenticated_long_lived_queue_rearm_kv_v1,
    submit_m1_authenticated_long_lived_queue_rearm_v1, Engine,
    M1AuthenticatedLongLivedQueueRearmScheduleFailureV1,
    M1AuthenticatedLongLivedQueueReleasedRoundV1, M1AuthenticatedRearmedRoundPageReleaseFailureV1,
    M1AuthenticatedRearmedRoundReleaseOutcomeV1, M1FullStepWorkspacePlans,
    M1LongLivedQueueRearmKvInputsV1, M1ObservedSpeculativeDiagnosticChoicesV1,
    M1PhysicalFixedBatchShapeV1, M1PhysicalRunnerRecipeOutcomeV1, M1ReleasedDeviceKvMemberV1,
    M1SpeculativeGenerationLoopV1, M1SpeculativeMemberControlV1, M1SpeculativeMemberStatusV1,
    M1SpeculativeRoundOutcomeV1,
};

/// Stable association rejection before any executor exists.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedSpeculativeExecutorInitErrorV1 {
    CoordinatorIdentity,
    Selection,
    QueueShape,
    PriorEpoch,
    PriorRound,
    PriorRoster,
    PriorMember { lane: usize },
    ActiveKv { lane: usize },
    TerminalKv { lane: usize },
}

/// Failed constructor retaining all three association witnesses unchanged.
#[must_use = "constructor rejection retains coordinator, outcome, and authenticated queue"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativeExecutorInitFailureV1 {
    error: M1AuthenticatedSpeculativeExecutorInitErrorV1,
    coordinator: M1SpeculativeGenerationLoopV1,
    prior: M1SpeculativeRoundOutcomeV1,
    released: M1AuthenticatedLongLivedQueueReleasedRoundV1,
}

impl M1AuthenticatedSpeculativeExecutorInitFailureV1 {
    #[must_use]
    pub const fn error(&self) -> M1AuthenticatedSpeculativeExecutorInitErrorV1 {
        self.error
    }

    #[must_use = "all rejected constructor inputs remain linear"]
    pub fn into_parts(
        self,
    ) -> (
        M1AuthenticatedSpeculativeExecutorInitErrorV1,
        M1SpeculativeGenerationLoopV1,
        M1SpeculativeRoundOutcomeV1,
        M1AuthenticatedLongLivedQueueReleasedRoundV1,
    ) {
        (self.error, self.coordinator, self.prior, self.released)
    }
}

/// Linear production owner of a verified authenticated queue and its logical coordinator.
#[must_use = "the authenticated executor must execute another round or be retained"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativePhysicalExecutorV1 {
    coordinator: M1SpeculativeGenerationLoopV1,
    released: M1AuthenticatedLongLivedQueueReleasedRoundV1,
}

/// Clean queue teardown retaining the final logical coordinator state.
#[must_use = "final coordinator state and authenticated release remain retained"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativeExecutorTeardownSuccessV1 {
    coordinator: M1SpeculativeGenerationLoopV1,
    released: crate::M1AuthenticatedLongLivedQueueRearmTeardownSuccessV1,
}

/// Terminal queue-release quarantine retaining the final logical state.
#[must_use = "final coordinator state and release quarantine remain retained"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativeExecutorTeardownFailureV1 {
    coordinator: M1SpeculativeGenerationLoopV1,
    released: Box<crate::M1AuthenticatedLongLivedQueueRearmTeardownFailureV1>,
}

impl M1AuthenticatedSpeculativeExecutorTeardownSuccessV1 {
    pub const fn released(&self) -> &crate::M1AuthenticatedLongLivedQueueRearmTeardownSuccessV1 {
        &self.released
    }

    #[must_use]
    pub const fn coordinator(&self) -> &M1SpeculativeGenerationLoopV1 {
        &self.coordinator
    }
}

impl M1AuthenticatedSpeculativeExecutorTeardownFailureV1 {
    pub const fn released(&self) -> &crate::M1AuthenticatedLongLivedQueueRearmTeardownFailureV1 {
        &self.released
    }

    #[must_use]
    pub const fn coordinator(&self) -> &M1SpeculativeGenerationLoopV1 {
        &self.coordinator
    }
}

/// Exact linear inputs for one next generation.
#[must_use = "round inputs contain linear page leases and workspace plans"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativePhysicalRoundInputsV1 {
    kv: M1LongLivedQueueRearmKvInputsV1,
    recipe_workspace_plans: M1FullStepWorkspacePlans,
    preparation_workspace_plans: M1FullStepWorkspacePlans,
    controls: Vec<M1SpeculativeMemberControlV1>,
}

impl M1AuthenticatedSpeculativePhysicalRoundInputsV1 {
    /// The recipe and preparation plans must describe equal addressless ranges.
    /// They are separate because both downstream owners intentionally consume
    /// their plan custody.
    pub const fn new(
        kv: M1LongLivedQueueRearmKvInputsV1,
        recipe_workspace_plans: M1FullStepWorkspacePlans,
        preparation_workspace_plans: M1FullStepWorkspacePlans,
        controls: Vec<M1SpeculativeMemberControlV1>,
    ) -> Self {
        Self {
            kv,
            recipe_workspace_plans,
            preparation_workspace_plans,
            controls,
        }
    }
}

/// Effectful stage retained by a failed production round.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedSpeculativePhysicalRoundStageV1 {
    Complete,
    Profile,
    Inputs,
    Epoch,
    Bind,
    Schedule,
    Recipe,
    KvReservation,
    WorkspacePreparation,
    Submit,
    Wait,
    Recycle,
    DiagnosticReadback,
    CoordinatorPreflight,
    PhysicalCompletion,
    CoordinatorCommit,
    PageRelease,
}

#[derive(Debug)]
#[allow(clippy::type_complexity)]
enum M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1 {
    Complete(
        Box<(
            M1AuthenticatedSpeculativePhysicalExecutorV1,
            M1AuthenticatedSpeculativePhysicalRoundInputsV1,
        )>,
    ),
    Retryable(
        Box<(
            M1AuthenticatedSpeculativePhysicalExecutorV1,
            M1AuthenticatedSpeculativePhysicalRoundInputsV1,
        )>,
    ),
    DiagnosticReadback(
        Box<(
            M1SpeculativeGenerationLoopV1,
            crate::M1SpeculativeRoundBindingV1,
            Vec<M1SpeculativeMemberControlV1>,
            Box<crate::M1AuthenticatedRearmedSpeculativeDiagnosticReadbackFailureV1>,
        )>,
    ),
    CoordinatorPreflight(
        Box<(
            M1SpeculativeGenerationLoopV1,
            crate::M1AuthenticatedRearmedSpeculativeDiagnosticCompletedReadbackV1,
            Vec<M1SpeculativeMemberControlV1>,
            crate::M1SpeculativeGenerationLoopErrorV1,
        )>,
    ),
    PhysicalCompletionPreflight(
        Box<(
            M1SpeculativeGenerationLoopV1,
            crate::M1SpeculativePreflightedRoundV1,
            Vec<M1SpeculativeMemberControlV1>,
            M1ObservedSpeculativeDiagnosticChoicesV1,
            crate::M1AuthenticatedRearmedCompletionPreflightFailureV1,
        )>,
    ),
    PageRelease(Box<M1AuthenticatedSpeculativePhysicalPageReleaseCustodyV1>),
    Schedule(
        Box<(
            M1SpeculativeGenerationLoopV1,
            crate::M1SpeculativeRoundBindingV1,
            M1LongLivedQueueRearmKvInputsV1,
            M1FullStepWorkspacePlans,
            M1FullStepWorkspacePlans,
            Vec<M1SpeculativeMemberControlV1>,
            Box<crate::M1AuthenticatedLongLivedQueueRearmScheduleTerminalV1>,
        )>,
    ),
    Recipe(
        Box<(
            M1SpeculativeGenerationLoopV1,
            crate::M1SpeculativeRoundBindingV1,
            crate::M1AuthenticatedScheduledLongLivedQueueRearmV1,
            M1LongLivedQueueRearmKvInputsV1,
            M1FullStepWorkspacePlans,
            Vec<M1SpeculativeMemberControlV1>,
            crate::M1PhysicalRunnerRecipeFailureV1,
        )>,
    ),
    KvReservation(
        Box<(
            M1SpeculativeGenerationLoopV1,
            crate::M1SpeculativeRoundBindingV1,
            crate::AddresslessM1PhysicalBufferRecipeV1,
            M1FullStepWorkspacePlans,
            Vec<M1SpeculativeMemberControlV1>,
            crate::M1AuthenticatedLongLivedQueueRearmKvReservationFailureV1,
        )>,
    ),
    WorkspacePreparation(
        Box<(
            M1SpeculativeGenerationLoopV1,
            crate::M1SpeculativeRoundBindingV1,
            crate::AddresslessM1PhysicalBufferRecipeV1,
            Vec<M1SpeculativeMemberControlV1>,
            Box<crate::M1AuthenticatedLongLivedQueueRearmPrepareFailureV1>,
        )>,
    ),
    Submit(
        Box<(
            M1SpeculativeGenerationLoopV1,
            crate::M1SpeculativeRoundBindingV1,
            Vec<M1SpeculativeMemberControlV1>,
            crate::M1AuthenticatedLongLivedQueueRearmSubmissionFailureV1,
        )>,
    ),
    QueueProgress(
        Box<(
            M1SpeculativeGenerationLoopV1,
            crate::M1SpeculativeRoundBindingV1,
            Vec<M1SpeculativeMemberControlV1>,
            Box<crate::M1AuthenticatedRearmedQueueProgressFailureV1>,
        )>,
    ),
    CoordinatorCommitPreflight(
        Box<(
            M1SpeculativeGenerationLoopV1,
            crate::M1AuthenticatedRearmedSpeculativeDiagnosticCompletedReadbackV1,
            Vec<M1SpeculativeMemberControlV1>,
            crate::M1SpeculativePreflightedRoundV1,
            crate::M1SpeculativeGenerationLoopErrorV1,
        )>,
    ),
    HostAllocation(
        Box<(
            M1SpeculativeGenerationLoopV1,
            crate::M1AuthenticatedRearmedSpeculativeDiagnosticCompletedReadbackV1,
            Vec<M1SpeculativeMemberControlV1>,
            crate::M1SpeculativePreflightedRoundV1,
        )>,
    ),
    PhysicalOutcome(
        Box<(
            M1SpeculativeGenerationLoopV1,
            crate::M1SpeculativePreflightedRoundV1,
            Vec<M1SpeculativeMemberControlV1>,
            M1ObservedSpeculativeDiagnosticChoicesV1,
            crate::M1AuthenticatedRearmedCompletionOutcomeV1,
        )>,
    ),
    CoordinatorCommit(
        Box<(
            M1SpeculativeGenerationLoopV1,
            Vec<M1SpeculativeMemberControlV1>,
            M1ObservedSpeculativeDiagnosticChoicesV1,
            crate::M1AuthenticatedRearmedCompletionOutcomeV1,
            Box<crate::M1SpeculativePreparedRoundCommitFailureV1>,
        )>,
    ),
    ReleasedPhysicalOutcome(
        Box<(
            M1SpeculativeGenerationLoopV1,
            M1SpeculativeRoundOutcomeV1,
            M1ObservedSpeculativeDiagnosticChoicesV1,
            crate::M1AuthenticatedRearmedCompletionOutcomeV1,
        )>,
    ),
}

/// Exact stage-specific terminal source; no lower failure is type-erased.
#[derive(Debug)]
pub enum M1AuthenticatedSpeculativePhysicalRoundTerminalSourceV1 {
    Schedule(Box<crate::M1AuthenticatedLongLivedQueueRearmScheduleTerminalV1>),
    Recipe(Box<crate::M1PhysicalRunnerRecipeFailureV1>),
    KvReservation(Box<crate::M1AuthenticatedLongLivedQueueRearmKvReservationFailureV1>),
    WorkspacePreparation(Box<crate::M1AuthenticatedLongLivedQueueRearmPrepareFailureV1>),
    Submit(Box<crate::M1AuthenticatedLongLivedQueueRearmSubmissionFailureV1>),
    QueueProgress(Box<crate::M1AuthenticatedRearmedQueueProgressFailureV1>),
    Coordinator(crate::M1SpeculativeGenerationLoopErrorV1),
    CoordinatorCommit(Box<crate::M1SpeculativePreparedRoundCommitFailureV1>),
    PhysicalOutcome(Box<crate::M1AuthenticatedRearmedCompletionOutcomeV1>),
    HostAllocation,
}

/// Failure retaining all available authenticated, KV, logical, and input custody.
#[must_use = "failed production round custody must be retried, torn down, or retained"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativePhysicalRoundFailureV1 {
    stage: M1AuthenticatedSpeculativePhysicalRoundStageV1,
    custody: M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1,
}

impl M1AuthenticatedSpeculativePhysicalRoundFailureV1 {
    #[must_use]
    pub const fn stage(&self) -> M1AuthenticatedSpeculativePhysicalRoundStageV1 {
        self.stage
    }

    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        !matches!(
            self.custody,
            M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::Complete(_)
                | M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::Retryable(_)
                | M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::PageRelease(_)
        )
    }

    #[must_use]
    pub fn retains_custody(&self) -> bool {
        let _ = &self.custody;
        true
    }

    /// Recovers the unchanged executor only for pre-detachment rejection.
    ///
    /// # Errors
    ///
    /// Returns the unchanged failure when its custody is not retryable.
    pub fn into_retryable_round(
        self,
    ) -> Result<
        (
            M1AuthenticatedSpeculativePhysicalExecutorV1,
            M1AuthenticatedSpeculativePhysicalRoundInputsV1,
        ),
        Self,
    > {
        let Self { stage, custody } = self;
        match custody {
            M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::Retryable(retained) => {
                Ok(*retained)
            }
            custody => Err(Self { stage, custody }),
        }
    }

    /// Recovers an already-complete executor without permitting another round.
    ///
    /// # Errors
    ///
    /// Returns the unchanged failure when it does not represent completion.
    pub fn into_complete_round(
        self,
    ) -> Result<
        (
            M1AuthenticatedSpeculativePhysicalExecutorV1,
            M1AuthenticatedSpeculativePhysicalRoundInputsV1,
        ),
        Self,
    > {
        let Self { stage, custody } = self;
        match custody {
            M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::Complete(retained) => {
                Ok(*retained)
            }
            custody => Err(Self { stage, custody }),
        }
    }

    /// Destroys a recoverable queue or consumes a concrete lower terminal owner
    /// into explicit quarantine. No effectful-stage failure is type-erased.
    ///
    /// # Errors
    ///
    /// The outer error returns unchanged pure-rejection custody that has no
    /// queue teardown to perform. The inner error is exact release quarantine.
    pub fn destroy_queue_and_retain_custody<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        Result<
            M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessV1,
            Box<M1AuthenticatedSpeculativePhysicalRoundTeardownFailureV1>,
        >,
        Self,
    > {
        let Self { stage, custody } = self;
        match custody {
            M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::DiagnosticReadback(
                retained,
            ) => {
                let (coordinator, binding, controls, failure) = *retained;
                let logical = Box::new((coordinator, binding, controls)) as Box<dyn fmt::Debug>;
                Ok(match failure.destroy_queue_and_retain_custody(engine) {
                    Ok(source) => Ok(M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessV1 {
                        stage,
                        source:
                            M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessSourceV1::Diagnostic(
                                source,
                            ),
                        logical,
                    }),
                    Err(source) => Err(Box::new(
                        M1AuthenticatedSpeculativePhysicalRoundTeardownFailureV1 {
                            stage,
                            source:
                                M1AuthenticatedSpeculativePhysicalRoundTeardownFailureSourceV1::Diagnostic(
                                    source,
                                ),
                            logical,
                        },
                    )),
                })
            }
            M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::CoordinatorPreflight(
                retained,
            ) => {
                let (coordinator, diagnostic, controls, error) = *retained;
                let (readback, choices) = diagnostic.into_parts();
                let logical =
                    Box::new((coordinator, controls, error, choices)) as Box<dyn fmt::Debug>;
                Ok(match readback.destroy_queue_and_retain_custody(engine) {
                    Ok(source) => Ok(M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessV1 {
                        stage,
                        source:
                            M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessSourceV1::Joined(
                                source,
                            ),
                        logical,
                    }),
                    Err(source) => Err(Box::new(
                        M1AuthenticatedSpeculativePhysicalRoundTeardownFailureV1 {
                            stage,
                            source:
                                M1AuthenticatedSpeculativePhysicalRoundTeardownFailureSourceV1::Joined(
                                    source,
                                ),
                            logical,
                        },
                    )),
                })
            }
            M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::CoordinatorCommitPreflight(
                retained,
            ) => {
                let (coordinator, diagnostic, controls, permit, error) = *retained;
                let (readback, choices) = diagnostic.into_parts();
                let logical = Box::new((coordinator, controls, permit, error, choices))
                    as Box<dyn fmt::Debug>;
                Ok(match readback.destroy_queue_and_retain_custody(engine) {
                    Ok(source) => Ok(M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessV1 {
                        stage,
                        source:
                            M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessSourceV1::Joined(
                                source,
                            ),
                        logical,
                    }),
                    Err(source) => Err(Box::new(
                        M1AuthenticatedSpeculativePhysicalRoundTeardownFailureV1 {
                            stage,
                            source:
                                M1AuthenticatedSpeculativePhysicalRoundTeardownFailureSourceV1::Joined(
                                    source,
                                ),
                            logical,
                        },
                    )),
                })
            }
            M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::HostAllocation(retained) => {
                let (coordinator, diagnostic, controls, permit) = *retained;
                let (readback, choices) = diagnostic.into_parts();
                let logical =
                    Box::new((coordinator, controls, permit, choices)) as Box<dyn fmt::Debug>;
                Ok(match readback.destroy_queue_and_retain_custody(engine) {
                    Ok(source) => Ok(M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessV1 {
                        stage,
                        source:
                            M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessSourceV1::Joined(
                                source,
                            ),
                        logical,
                    }),
                    Err(source) => Err(Box::new(
                        M1AuthenticatedSpeculativePhysicalRoundTeardownFailureV1 {
                            stage,
                            source:
                                M1AuthenticatedSpeculativePhysicalRoundTeardownFailureSourceV1::Joined(
                                    source,
                                ),
                            logical,
                        },
                    )),
                })
            }
            M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::PhysicalCompletionPreflight(
                retained,
            ) => {
                let (coordinator, permit, controls, choices, failure) = *retained;
                let logical =
                    Box::new((coordinator, permit, controls, choices)) as Box<dyn fmt::Debug>;
                Ok(match failure.destroy_queue_and_retain_custody(engine) {
                    Ok(source) => Ok(M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessV1 {
                        stage,
                        source:
                            M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessSourceV1::CompletionPreflight(
                                source,
                            ),
                        logical,
                    }),
                    Err(source) => Err(Box::new(
                        M1AuthenticatedSpeculativePhysicalRoundTeardownFailureV1 {
                            stage,
                            source:
                                M1AuthenticatedSpeculativePhysicalRoundTeardownFailureSourceV1::CompletionPreflight(
                                    source,
                                ),
                            logical,
                        },
                    )),
                })
            }
            M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::PageRelease(retained) => {
                let M1AuthenticatedSpeculativePhysicalPageReleaseCustodyV1 {
                    coordinator,
                    outcome,
                    choices,
                    failure,
                } = *retained;
                let logical = Box::new((coordinator, outcome, choices)) as Box<dyn fmt::Debug>;
                Ok(match failure.destroy_queue_and_retain_round(engine) {
                    Ok(source) => Ok(M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessV1 {
                        stage,
                        source:
                            M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessSourceV1::PageRelease(
                                source,
                            ),
                        logical,
                    }),
                    Err(source) => Err(Box::new(
                        M1AuthenticatedSpeculativePhysicalRoundTeardownFailureV1 {
                            stage,
                            source:
                                M1AuthenticatedSpeculativePhysicalRoundTeardownFailureSourceV1::PageRelease(
                                    source,
                                ),
                            logical,
                        },
                    )),
                })
            }
            M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::PhysicalOutcome(retained) => {
                let (coordinator, permit, controls, choices, physical) = *retained;
                let logical =
                    Box::new((coordinator, permit, controls, choices)) as Box<dyn fmt::Debug>;
                Ok(match physical.destroy_queue_and_retain_rejected(engine) {
                    Ok(Ok(source)) => Ok(M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessV1 {
                        stage,
                        source:
                            M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessSourceV1::RejectedCompletion(
                                source,
                            ),
                        logical,
                    }),
                    Ok(Err(source)) => Err(Box::new(
                        M1AuthenticatedSpeculativePhysicalRoundTeardownFailureV1 {
                            stage,
                            source:
                                M1AuthenticatedSpeculativePhysicalRoundTeardownFailureSourceV1::RejectedCompletion(
                                    source,
                                ),
                            logical,
                        },
                    )),
                    Err(physical) => {
                        engine.quarantine_m1_queue_rearm_failure();
                        Ok(M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessV1 {
                            stage,
                            source:
                                M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessSourceV1::TerminalQuarantine(
                                    M1AuthenticatedSpeculativePhysicalRoundTerminalSourceV1::PhysicalOutcome(
                                        physical,
                                    ),
                                ),
                            logical,
                        })
                    }
                })
            }
            M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::CoordinatorCommit(retained) => {
                let (coordinator, controls, choices, physical, failure) = *retained;
                engine.quarantine_m1_queue_rearm_failure();
                let logical = Box::new((coordinator, controls, choices)) as Box<dyn fmt::Debug>;
                Ok(match physical.release_completed() {
                    M1AuthenticatedRearmedRoundReleaseOutcomeV1::Released(released) => {
                        match released.destroy_queue_and_retain_round(engine) {
                            Ok(source) => Ok(M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessV1 {
                                stage,
                                source:
                                    M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessSourceV1::Released(
                                        source,
                                    ),
                                logical: Box::new((logical, failure)),
                            }),
                            Err(source) => Err(Box::new(
                                M1AuthenticatedSpeculativePhysicalRoundTeardownFailureV1 {
                                    stage,
                                    source:
                                        M1AuthenticatedSpeculativePhysicalRoundTeardownFailureSourceV1::Released(
                                            source,
                                        ),
                                    logical: Box::new((logical, failure)),
                                },
                            )),
                        }
                    }
                    M1AuthenticatedRearmedRoundReleaseOutcomeV1::Rejected(source) => {
                        match source.destroy_queue_and_retain_round(engine) {
                            Ok(source) => Ok(M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessV1 {
                                stage,
                                source:
                                    M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessSourceV1::PageRelease(
                                        source,
                                    ),
                                logical: Box::new((logical, failure)),
                            }),
                            Err(source) => Err(Box::new(
                                M1AuthenticatedSpeculativePhysicalRoundTeardownFailureV1 {
                                    stage,
                                    source:
                                        M1AuthenticatedSpeculativePhysicalRoundTeardownFailureSourceV1::PageRelease(
                                            source,
                                        ),
                                    logical: Box::new((logical, failure)),
                                },
                            )),
                        }
                    }
                    M1AuthenticatedRearmedRoundReleaseOutcomeV1::NotCompleted(physical) => {
                        Ok(M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessV1 {
                            stage,
                            source:
                                M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessSourceV1::TerminalQuarantine(
                                    M1AuthenticatedSpeculativePhysicalRoundTerminalSourceV1::PhysicalOutcome(
                                        Box::new(physical),
                                    ),
                                ),
                            logical: Box::new((logical, failure)),
                        })
                    }
                })
            }
            M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::Schedule(retained) => {
                let (coordinator, binding, kv, recipe, preparation, controls, source) = *retained;
                terminal_quarantine(
                    engine,
                    stage,
                    M1AuthenticatedSpeculativePhysicalRoundTerminalSourceV1::Schedule(source),
                    (coordinator, binding, kv, recipe, preparation, controls),
                )
            }
            M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::Recipe(retained) => {
                let (coordinator, binding, scheduled, kv, plans, controls, source) = *retained;
                terminal_quarantine(
                    engine,
                    stage,
                    M1AuthenticatedSpeculativePhysicalRoundTerminalSourceV1::Recipe(Box::new(
                        source,
                    )),
                    (coordinator, binding, scheduled, kv, plans, controls),
                )
            }
            M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::KvReservation(retained) => {
                let (coordinator, binding, recipe, plans, controls, source) = *retained;
                terminal_quarantine(
                    engine,
                    stage,
                    M1AuthenticatedSpeculativePhysicalRoundTerminalSourceV1::KvReservation(
                        Box::new(source),
                    ),
                    (coordinator, binding, recipe, plans, controls),
                )
            }
            M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::WorkspacePreparation(retained) => {
                let (coordinator, binding, recipe, controls, source) = *retained;
                terminal_quarantine(
                    engine,
                    stage,
                    M1AuthenticatedSpeculativePhysicalRoundTerminalSourceV1::WorkspacePreparation(source),
                    (coordinator, binding, recipe, controls),
                )
            }
            M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::Submit(retained) => {
                let (coordinator, binding, controls, source) = *retained;
                terminal_quarantine(
                    engine,
                    stage,
                    M1AuthenticatedSpeculativePhysicalRoundTerminalSourceV1::Submit(Box::new(
                        source,
                    )),
                    (coordinator, binding, controls),
                )
            }
            M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::QueueProgress(retained) => {
                let (coordinator, binding, controls, source) = *retained;
                terminal_quarantine(
                    engine,
                    stage,
                    M1AuthenticatedSpeculativePhysicalRoundTerminalSourceV1::QueueProgress(source),
                    (coordinator, binding, controls),
                )
            }
            M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::ReleasedPhysicalOutcome(retained) => {
                let (coordinator, outcome, choices, source) = *retained;
                terminal_quarantine(
                    engine,
                    stage,
                    M1AuthenticatedSpeculativePhysicalRoundTerminalSourceV1::PhysicalOutcome(
                        Box::new(source),
                    ),
                    (coordinator, outcome, choices),
                )
            }
            M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::Retryable(retained) => {
                Err(Self {
                    stage,
                    custody: M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::Retryable(
                        retained,
                    ),
                })
            }
            M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::Complete(retained) => {
                Err(Self {
                    stage,
                    custody: M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::Complete(
                        retained,
                    ),
                })
            }
        }
    }

    /// Retries only a post-commit KV page release and returns the next executor.
    ///
    /// # Errors
    ///
    /// Returns the unchanged failure for every other stage or when page release
    /// remains rejected.
    pub fn retry_page_release(
        self,
    ) -> Result<M1AuthenticatedSpeculativePhysicalRoundSuccessV1, Self> {
        let Self { stage, custody } = self;
        let M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::PageRelease(retained) =
            custody
        else {
            return Err(Self { stage, custody });
        };
        let M1AuthenticatedSpeculativePhysicalPageReleaseCustodyV1 {
            coordinator,
            outcome,
            choices,
            failure,
        } = *retained;
        match failure.retry() {
            M1AuthenticatedRearmedRoundReleaseOutcomeV1::Released(released) => {
                Ok(M1AuthenticatedSpeculativePhysicalRoundSuccessV1 {
                    executor: M1AuthenticatedSpeculativePhysicalExecutorV1 {
                        coordinator,
                        released,
                    },
                    outcome,
                    choices,
                })
            }
            M1AuthenticatedRearmedRoundReleaseOutcomeV1::Rejected(failure) => Err(Self {
                stage: M1AuthenticatedSpeculativePhysicalRoundStageV1::PageRelease,
                custody: M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::PageRelease(
                    Box::new(M1AuthenticatedSpeculativePhysicalPageReleaseCustodyV1 {
                        coordinator,
                        outcome,
                        choices,
                        failure: *failure,
                    }),
                ),
            }),
            M1AuthenticatedRearmedRoundReleaseOutcomeV1::NotCompleted(not_completed) => Err(Self {
                stage: M1AuthenticatedSpeculativePhysicalRoundStageV1::PageRelease,
                custody:
                    M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::ReleasedPhysicalOutcome(
                        Box::new((coordinator, outcome, choices, not_completed)),
                    ),
            }),
        }
    }
}

/// Exact lower success from a typed production-round teardown.
#[derive(Debug)]
pub enum M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessSourceV1 {
    Diagnostic(crate::M1AuthenticatedRearmedSpeculativeDiagnosticReadbackTeardownSuccessV1),
    Joined(crate::M1AuthenticatedRearmedCompletedReadbackTeardownSuccessV1),
    CompletionPreflight(crate::M1AuthenticatedRearmedCompletionPreflightTeardownSuccessV1),
    RejectedCompletion(crate::M1AuthenticatedRearmedRejectedCompletionTeardownSuccessV1),
    PageRelease(crate::M1AuthenticatedRearmedRoundPageReleaseTeardownSuccessV1),
    Released(crate::M1AuthenticatedLongLivedQueueRearmTeardownSuccessV1),
    TerminalQuarantine(M1AuthenticatedSpeculativePhysicalRoundTerminalSourceV1),
}

/// Exact lower quarantine from a typed production-round teardown.
#[derive(Debug)]
pub enum M1AuthenticatedSpeculativePhysicalRoundTeardownFailureSourceV1 {
    Diagnostic(Box<crate::M1AuthenticatedRearmedSpeculativeDiagnosticReadbackTeardownFailureV1>),
    Joined(Box<crate::M1AuthenticatedRearmedCompletedReadbackTeardownFailureV1>),
    CompletionPreflight(Box<crate::M1AuthenticatedRearmedCompletionPreflightTeardownFailureV1>),
    RejectedCompletion(Box<crate::M1AuthenticatedRearmedRejectedCompletionTeardownFailureV1>),
    PageRelease(Box<crate::M1AuthenticatedRearmedRoundPageReleaseTeardownFailureV1>),
    Released(Box<crate::M1AuthenticatedLongLivedQueueRearmTeardownFailureV1>),
}

/// Completed queue teardown or explicit process-level terminal quarantine.
#[must_use = "teardown disposition retains lower and logical custody"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessV1 {
    stage: M1AuthenticatedSpeculativePhysicalRoundStageV1,
    source: M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessSourceV1,
    logical: Box<dyn fmt::Debug>,
}

/// Queue release quarantine with all logical failure context retained.
#[must_use = "teardown failure retains lower and logical custody"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativePhysicalRoundTeardownFailureV1 {
    stage: M1AuthenticatedSpeculativePhysicalRoundStageV1,
    source: M1AuthenticatedSpeculativePhysicalRoundTeardownFailureSourceV1,
    logical: Box<dyn fmt::Debug>,
}

impl M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessV1 {
    #[must_use]
    pub const fn stage(&self) -> M1AuthenticatedSpeculativePhysicalRoundStageV1 {
        self.stage
    }

    #[must_use]
    pub const fn source(&self) -> &M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessSourceV1 {
        &self.source
    }

    #[must_use]
    pub fn retains_logical_custody(&self) -> bool {
        let _ = &self.logical;
        true
    }
}

impl M1AuthenticatedSpeculativePhysicalRoundTeardownFailureV1 {
    #[must_use]
    pub const fn stage(&self) -> M1AuthenticatedSpeculativePhysicalRoundStageV1 {
        self.stage
    }

    #[must_use]
    pub const fn source(&self) -> &M1AuthenticatedSpeculativePhysicalRoundTeardownFailureSourceV1 {
        &self.source
    }

    #[must_use]
    pub fn retains_logical_custody(&self) -> bool {
        let _ = &self.logical;
        true
    }
}

#[derive(Debug)]
struct M1AuthenticatedSpeculativePhysicalPageReleaseCustodyV1 {
    coordinator: M1SpeculativeGenerationLoopV1,
    outcome: M1SpeculativeRoundOutcomeV1,
    choices: M1ObservedSpeculativeDiagnosticChoicesV1,
    failure: M1AuthenticatedRearmedRoundPageReleaseFailureV1,
}

/// One committed round and the next reusable authenticated executor.
#[must_use = "the next executor and inert choices remain linear"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativePhysicalRoundSuccessV1 {
    executor: M1AuthenticatedSpeculativePhysicalExecutorV1,
    outcome: M1SpeculativeRoundOutcomeV1,
    choices: M1ObservedSpeculativeDiagnosticChoicesV1,
}

impl M1AuthenticatedSpeculativePhysicalRoundSuccessV1 {
    pub const fn executor(&self) -> &M1AuthenticatedSpeculativePhysicalExecutorV1 {
        &self.executor
    }

    pub const fn outcome(&self) -> &M1SpeculativeRoundOutcomeV1 {
        &self.outcome
    }

    /// Independent device copies used for semantic checking; never M1 evidence.
    pub const fn diagnostic_choices(&self) -> &M1ObservedSpeculativeDiagnosticChoicesV1 {
        &self.choices
    }

    #[must_use = "all success owners remain linear"]
    pub fn into_parts(
        self,
    ) -> (
        M1AuthenticatedSpeculativePhysicalExecutorV1,
        M1SpeculativeRoundOutcomeV1,
        M1ObservedSpeculativeDiagnosticChoicesV1,
    ) {
        (self.executor, self.outcome, self.choices)
    }
}

fn expected_queue_shape(selection: Qwen3PlanSelection) -> Option<M1PhysicalFixedBatchShapeV1> {
    crate::M1SpeculativePhysicalShapeV1::from_selection(selection)
        .ok()
        .and_then(|shape| match shape.draft_tokens() {
            4 => Some(M1PhysicalFixedBatchShapeV1::SpeculativeK4),
            8 => Some(M1PhysicalFixedBatchShapeV1::SpeculativeK8),
            16 => Some(M1PhysicalFixedBatchShapeV1::SpeculativeK16),
            _ => None,
        })
}

fn production_entry_profile_matches(
    selection: Qwen3PlanSelection,
    queue_shape: M1PhysicalFixedBatchShapeV1,
) -> bool {
    expected_queue_shape(selection) == Some(queue_shape)
}

const fn production_entry_has_active_members(active_count: usize) -> bool {
    active_count != 0
}

struct M1AuthenticatedSpeculativeAssociationHeaderV1<'a> {
    coordinator_identity: crate::speculative_generation_loop::M1SpeculativeCoordinatorIdentityV1,
    prior_identity: crate::speculative_generation_loop::M1SpeculativeCoordinatorIdentityV1,
    selection: Qwen3PlanSelection,
    prior_selection: Qwen3PlanSelection,
    checked_selection: Qwen3PlanSelection,
    queue_selection: Qwen3PlanSelection,
    queue_shape: M1PhysicalFixedBatchShapeV1,
    coordinator_last_epoch: Option<CompletionEpoch>,
    prior_epoch: CompletionEpoch,
    checked_epoch: CompletionEpoch,
    coordinator_next_round: u64,
    prior_round: u64,
    active: &'a [RequestId],
    prior_active: &'a [RequestId],
    released_active: &'a [RequestId],
}

fn validate_prior_association_header(
    header: &M1AuthenticatedSpeculativeAssociationHeaderV1<'_>,
) -> Result<(), M1AuthenticatedSpeculativeExecutorInitErrorV1> {
    if header.prior_identity != header.coordinator_identity {
        return Err(M1AuthenticatedSpeculativeExecutorInitErrorV1::CoordinatorIdentity);
    }
    if header.prior_selection != header.selection
        || header.checked_selection != header.selection
        || header.queue_selection != header.selection
    {
        return Err(M1AuthenticatedSpeculativeExecutorInitErrorV1::Selection);
    }
    if !production_entry_profile_matches(header.selection, header.queue_shape) {
        return Err(M1AuthenticatedSpeculativeExecutorInitErrorV1::QueueShape);
    }
    if header.coordinator_last_epoch != Some(header.prior_epoch)
        || header.checked_epoch != header.prior_epoch
    {
        return Err(M1AuthenticatedSpeculativeExecutorInitErrorV1::PriorEpoch);
    }
    if header
        .prior_round
        .checked_add(1)
        .is_none_or(|next| next != header.coordinator_next_round)
    {
        return Err(M1AuthenticatedSpeculativeExecutorInitErrorV1::PriorRound);
    }
    if header.prior_active != header.active || header.released_active != header.active {
        return Err(M1AuthenticatedSpeculativeExecutorInitErrorV1::PriorRoster);
    }
    Ok(())
}

fn validate_prior_association(
    coordinator: &M1SpeculativeGenerationLoopV1,
    prior: &M1SpeculativeRoundOutcomeV1,
    released: &M1AuthenticatedLongLivedQueueReleasedRoundV1,
) -> Result<(), M1AuthenticatedSpeculativeExecutorInitErrorV1> {
    let current = released.current_released();
    let checked = current.checked();
    let selection = coordinator.shape().selection();
    let active = coordinator.active_roster();
    let released_active: Vec<RequestId> = current
        .members()
        .iter()
        .filter_map(|member| match member {
            M1ReleasedDeviceKvMemberV1::Active(cache) => Some(cache.projection().request),
            M1ReleasedDeviceKvMemberV1::Terminal(_) => None,
        })
        .collect();
    validate_prior_association_header(&M1AuthenticatedSpeculativeAssociationHeaderV1 {
        coordinator_identity: coordinator.identity(),
        prior_identity: prior.coordinator_identity(),
        selection,
        prior_selection: prior.selection(),
        checked_selection: checked.selection(),
        queue_selection: current.queue().custody().selection(),
        queue_shape: current.queue().shape(),
        coordinator_last_epoch: coordinator.last_epoch(),
        prior_epoch: prior.completed_epoch(),
        checked_epoch: checked.epoch(),
        coordinator_next_round: coordinator.next_round(),
        prior_round: prior.completed_round(),
        active: &active,
        prior_active: prior.next_active_roster(),
        released_active: &released_active,
    })?;
    if checked.records().len() != prior.members().len()
        || current.members().len() != prior.members().len()
    {
        return Err(M1AuthenticatedSpeculativeExecutorInitErrorV1::PriorRoster);
    }
    for (lane, ((record, outcome), released_member)) in checked
        .records()
        .iter()
        .zip(prior.members())
        .zip(current.members())
        .enumerate()
    {
        let wire = record.record();
        if wire.request != outcome.request()
            || released_member.request() != outcome.request()
            || usize::from(wire.emitted_token_count) != outcome.raw_emitted().tokens().len()
            || wire.emitted_tokens[..usize::from(wire.emitted_token_count)]
                != *outcome.raw_emitted().tokens()
        {
            return Err(M1AuthenticatedSpeculativeExecutorInitErrorV1::PriorMember { lane });
        }
        if let M1ReleasedDeviceKvMemberV1::Active(cache) = released_member {
            let projection = cache.projection();
            let Some(member) = coordinator.member(outcome.request()) else {
                return Err(M1AuthenticatedSpeculativeExecutorInitErrorV1::ActiveKv { lane });
            };
            if outcome.status() != M1SpeculativeMemberStatusV1::Active
                || projection.target.committed_tokens != member.target_committed_tokens()
                || projection.draft.committed_tokens != member.draft_committed_tokens()
            {
                return Err(M1AuthenticatedSpeculativeExecutorInitErrorV1::ActiveKv { lane });
            }
        } else if let M1ReleasedDeviceKvMemberV1::Terminal(terminal) = released_member {
            let Some(member) = coordinator.member(outcome.request()) else {
                return Err(M1AuthenticatedSpeculativeExecutorInitErrorV1::TerminalKv { lane });
            };
            if outcome.status() == M1SpeculativeMemberStatusV1::Active
                || member.status() != outcome.status()
                || active.contains(&outcome.request())
                || terminal.target().committed_tokens != outcome.target_settlement().commit_end()
                || terminal.draft().committed_tokens != outcome.draft_settlement().commit_end()
            {
                return Err(M1AuthenticatedSpeculativeExecutorInitErrorV1::TerminalKv { lane });
            }
        }
    }
    Ok(())
}

#[allow(clippy::unnecessary_box_returns)]
fn retryable_failure(
    stage: M1AuthenticatedSpeculativePhysicalRoundStageV1,
    executor: M1AuthenticatedSpeculativePhysicalExecutorV1,
    inputs: M1AuthenticatedSpeculativePhysicalRoundInputsV1,
) -> Box<M1AuthenticatedSpeculativePhysicalRoundFailureV1> {
    Box::new(M1AuthenticatedSpeculativePhysicalRoundFailureV1 {
        stage,
        custody: M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::Retryable(Box::new((
            executor, inputs,
        ))),
    })
}

#[allow(clippy::unnecessary_wraps)]
fn terminal_quarantine<const C: usize>(
    engine: &mut Engine<C>,
    stage: M1AuthenticatedSpeculativePhysicalRoundStageV1,
    source: M1AuthenticatedSpeculativePhysicalRoundTerminalSourceV1,
    logical: impl fmt::Debug + 'static,
) -> Result<
    Result<
        M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessV1,
        Box<M1AuthenticatedSpeculativePhysicalRoundTeardownFailureV1>,
    >,
    M1AuthenticatedSpeculativePhysicalRoundFailureV1,
> {
    engine.quarantine_m1_queue_rearm_failure();
    Ok(Ok(
        M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessV1 {
            stage,
            source:
                M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessSourceV1::TerminalQuarantine(
                    source,
                ),
            logical: Box::new(logical),
        },
    ))
}

impl M1AuthenticatedSpeculativePhysicalExecutorV1 {
    /// Associates one coordinator-committed prior result with the exact released
    /// authenticated queue that produced it.
    ///
    /// # Errors
    ///
    /// Returns all three unchanged owners when coordinator identity, selection,
    /// queue shape, epoch, round, roster, record, or KV lineage differs.
    pub fn new(
        coordinator: M1SpeculativeGenerationLoopV1,
        prior: M1SpeculativeRoundOutcomeV1,
        released: M1AuthenticatedLongLivedQueueReleasedRoundV1,
    ) -> Result<Self, Box<M1AuthenticatedSpeculativeExecutorInitFailureV1>> {
        if let Err(error) = validate_prior_association(&coordinator, &prior, &released) {
            return Err(Box::new(M1AuthenticatedSpeculativeExecutorInitFailureV1 {
                error,
                coordinator,
                prior,
                released,
            }));
        }
        Ok(Self {
            coordinator,
            released,
        })
    }

    #[must_use]
    pub const fn selection(&self) -> Qwen3PlanSelection {
        self.coordinator.shape().selection()
    }

    #[must_use]
    pub const fn next_round(&self) -> u64 {
        self.coordinator.next_round()
    }

    #[must_use]
    pub fn active_count(&self) -> usize {
        self.coordinator.active_roster().len()
    }

    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.active_count() == 0
    }

    /// Destroys the authenticated queue while retaining the final coordinator,
    /// checked completion, and complete queue history on either lower outcome.
    ///
    /// # Errors
    ///
    /// Returns exact lower queue-release quarantine with final logical state.
    pub fn destroy_queue_and_retain_state<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1AuthenticatedSpeculativeExecutorTeardownSuccessV1,
        Box<M1AuthenticatedSpeculativeExecutorTeardownFailureV1>,
    > {
        let Self {
            coordinator,
            released,
        } = self;
        match released.destroy_queue_and_retain_round(engine) {
            Ok(released) => Ok(M1AuthenticatedSpeculativeExecutorTeardownSuccessV1 {
                coordinator,
                released,
            }),
            Err(released) => Err(Box::new(
                M1AuthenticatedSpeculativeExecutorTeardownFailureV1 {
                    coordinator,
                    released,
                },
            )),
        }
    }

    /// Executes one real authenticated KFD queue generation and returns a new
    /// executor only after physical settlement and logical commit both succeed.
    ///
    /// # Errors
    ///
    /// Returns stage-specific linear custody for retry, teardown, or permanent
    /// quarantine. No failed transition returns a reusable stale executor.
    pub fn execute_round<const C: usize>(
        self,
        engine: &mut Engine<C>,
        inputs: M1AuthenticatedSpeculativePhysicalRoundInputsV1,
    ) -> Result<
        M1AuthenticatedSpeculativePhysicalRoundSuccessV1,
        Box<M1AuthenticatedSpeculativePhysicalRoundFailureV1>,
    > {
        if !production_entry_profile_matches(
            self.selection(),
            self.released.current_released().queue().shape(),
        ) {
            return Err(retryable_failure(
                M1AuthenticatedSpeculativePhysicalRoundStageV1::Profile,
                self,
                inputs,
            ));
        }
        if !production_entry_has_active_members(self.active_count()) {
            return Err(Box::new(M1AuthenticatedSpeculativePhysicalRoundFailureV1 {
                stage: M1AuthenticatedSpeculativePhysicalRoundStageV1::Complete,
                custody: M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::Complete(
                    Box::new((self, inputs)),
                ),
            }));
        }
        if inputs.recipe_workspace_plans != inputs.preparation_workspace_plans
            || inputs.recipe_workspace_plans.kind()
                != crate::M1FullStepWorkspaceInputKind::SpeculativeRound
            || !matches!(
                &inputs.kv,
                M1LongLivedQueueRearmKvInputsV1::SpeculativeRound { .. }
            )
        {
            return Err(retryable_failure(
                M1AuthenticatedSpeculativePhysicalRoundStageV1::Inputs,
                self,
                inputs,
            ));
        }
        let Self {
            coordinator,
            released,
        } = self;
        let epoch = match released
            .current_released()
            .checked()
            .epoch()
            .value()
            .checked_add(1)
            .map(CompletionEpoch::new)
        {
            Some(epoch) => epoch,
            None => {
                return Err(retryable_failure(
                    M1AuthenticatedSpeculativePhysicalRoundStageV1::Epoch,
                    Self {
                        coordinator,
                        released,
                    },
                    inputs,
                ));
            }
        };
        let roster = coordinator.active_roster();
        let binding = match coordinator.bind_round(coordinator.next_round(), epoch, &roster) {
            Ok(binding) => binding,
            Err(_) => {
                return Err(retryable_failure(
                    M1AuthenticatedSpeculativePhysicalRoundStageV1::Bind,
                    Self {
                        coordinator,
                        released,
                    },
                    inputs,
                ));
            }
        };
        let M1AuthenticatedSpeculativePhysicalRoundInputsV1 {
            kv,
            recipe_workspace_plans,
            preparation_workspace_plans,
            controls,
        } = inputs;
        let scheduled = match released.schedule_next_exact(engine, epoch, &roster) {
            Ok(scheduled) => scheduled,
            Err(M1AuthenticatedLongLivedQueueRearmScheduleFailureV1::Rejected(rejected)) => {
                let (_error, released) = rejected.into_parts();
                return Err(retryable_failure(
                    M1AuthenticatedSpeculativePhysicalRoundStageV1::Schedule,
                    Self {
                        coordinator,
                        released,
                    },
                    M1AuthenticatedSpeculativePhysicalRoundInputsV1 {
                        kv,
                        recipe_workspace_plans,
                        preparation_workspace_plans,
                        controls,
                    },
                ));
            }
            Err(M1AuthenticatedLongLivedQueueRearmScheduleFailureV1::Terminal(terminal)) => {
                return Err(Box::new(M1AuthenticatedSpeculativePhysicalRoundFailureV1 {
                    stage: M1AuthenticatedSpeculativePhysicalRoundStageV1::Schedule,
                    custody: M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::Schedule(
                        Box::new((
                            coordinator,
                            binding,
                            kv,
                            recipe_workspace_plans,
                            preparation_workspace_plans,
                            controls,
                            terminal,
                        )),
                    ),
                }));
            }
        };
        let recipe = match scheduled.derive_retained_step_recipe(recipe_workspace_plans) {
            M1PhysicalRunnerRecipeOutcomeV1::Prepared(recipe) => recipe,
            M1PhysicalRunnerRecipeOutcomeV1::Rejected(failure) => {
                engine.quarantine_m1_queue_rearm_failure();
                return Err(Box::new(M1AuthenticatedSpeculativePhysicalRoundFailureV1 {
                    stage: M1AuthenticatedSpeculativePhysicalRoundStageV1::Recipe,
                    custody: M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::Recipe(
                        Box::new((
                            coordinator,
                            binding,
                            scheduled,
                            kv,
                            preparation_workspace_plans,
                            controls,
                            failure,
                        )),
                    ),
                }));
            }
        };
        let reserved =
            match reserve_m1_authenticated_long_lived_queue_rearm_kv_v1(engine, scheduled, kv) {
                Ok(reserved) => reserved,
                Err(failure) => {
                    return Err(Box::new(M1AuthenticatedSpeculativePhysicalRoundFailureV1 {
                        stage: M1AuthenticatedSpeculativePhysicalRoundStageV1::KvReservation,
                        custody:
                            M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::KvReservation(
                                Box::new((
                                    coordinator,
                                    binding,
                                    recipe,
                                    preparation_workspace_plans,
                                    controls,
                                    failure,
                                )),
                            ),
                    }));
                }
            };
        let prepared = match prepare_m1_authenticated_long_lived_queue_rearm_v1(
            engine,
            reserved,
            preparation_workspace_plans,
        ) {
            Ok(prepared) => prepared,
            Err(failure) => {
                return Err(Box::new(M1AuthenticatedSpeculativePhysicalRoundFailureV1 {
                    stage: M1AuthenticatedSpeculativePhysicalRoundStageV1::WorkspacePreparation,
                    custody:
                        M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::WorkspacePreparation(
                            Box::new((coordinator, binding, recipe, controls, failure)),
                        ),
                }));
            }
        };
        let published =
            match submit_m1_authenticated_long_lived_queue_rearm_v1(engine, prepared, recipe) {
                Ok(published) => published,
                Err(failure) => {
                    return Err(Box::new(M1AuthenticatedSpeculativePhysicalRoundFailureV1 {
                        stage: M1AuthenticatedSpeculativePhysicalRoundStageV1::Submit,
                        custody: M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::Submit(
                            Box::new((coordinator, binding, controls, failure)),
                        ),
                    }));
                }
            };
        let completed = match published.wait(engine) {
            Ok(completed) => completed,
            Err(failure) => {
                return Err(Box::new(M1AuthenticatedSpeculativePhysicalRoundFailureV1 {
                    stage: M1AuthenticatedSpeculativePhysicalRoundStageV1::Wait,
                    custody: M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::QueueProgress(
                        Box::new((coordinator, binding, controls, failure)),
                    ),
                }));
            }
        };
        let recycled = match completed.recycle(engine) {
            Ok(recycled) => recycled,
            Err(failure) => {
                return Err(Box::new(M1AuthenticatedSpeculativePhysicalRoundFailureV1 {
                    stage: M1AuthenticatedSpeculativePhysicalRoundStageV1::Recycle,
                    custody: M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::QueueProgress(
                        Box::new((coordinator, binding, controls, failure)),
                    ),
                }));
            }
        };
        let diagnostic = match recycled.read_and_check_speculative_diagnostic_completion() {
            Ok(diagnostic) => diagnostic,
            Err(failure) => {
                return Err(Box::new(M1AuthenticatedSpeculativePhysicalRoundFailureV1 {
                    stage: M1AuthenticatedSpeculativePhysicalRoundStageV1::DiagnosticReadback,
                    custody:
                        M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::DiagnosticReadback(
                            Box::new((coordinator, binding, controls, failure)),
                        ),
                }));
            }
        };
        let preflighted = match coordinator.preflight_checked_round(
            binding,
            diagnostic.checked(),
            &controls,
        ) {
            Ok(preflighted) => preflighted,
            Err(error) => {
                engine.quarantine_m1_queue_rearm_failure();
                return Err(Box::new(M1AuthenticatedSpeculativePhysicalRoundFailureV1 {
                    stage: M1AuthenticatedSpeculativePhysicalRoundStageV1::CoordinatorPreflight,
                    custody:
                        M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::CoordinatorPreflight(
                            Box::new((coordinator, diagnostic, controls, error)),
                        ),
                }));
            }
        };
        if let Err(error) = coordinator.preflight_prepared_round_commit(&preflighted) {
            engine.quarantine_m1_queue_rearm_failure();
            return Err(Box::new(M1AuthenticatedSpeculativePhysicalRoundFailureV1 {
                stage: M1AuthenticatedSpeculativePhysicalRoundStageV1::CoordinatorPreflight,
                custody:
                    M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::CoordinatorCommitPreflight(
                        Box::new((coordinator, diagnostic, controls, preflighted, error)),
                    ),
            }));
        }
        let mut dispositions = Vec::new();
        if dispositions
            .try_reserve_exact(preflighted.members().len())
            .is_err()
        {
            engine.quarantine_m1_queue_rearm_failure();
            return Err(Box::new(M1AuthenticatedSpeculativePhysicalRoundFailureV1 {
                stage: M1AuthenticatedSpeculativePhysicalRoundStageV1::CoordinatorPreflight,
                custody: M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::HostAllocation(
                    Box::new((coordinator, diagnostic, controls, preflighted)),
                ),
            }));
        }
        dispositions.extend(
            preflighted
                .members()
                .iter()
                .copied()
                .map(crate::M1SpeculativeMemberRoundOutcomeV1::physical_disposition),
        );
        let (readback, choices) = diagnostic.into_parts();
        let physical = match readback.complete(engine, dispositions) {
            Ok(physical) => physical,
            Err(failure) => {
                return Err(Box::new(M1AuthenticatedSpeculativePhysicalRoundFailureV1 {
                    stage: M1AuthenticatedSpeculativePhysicalRoundStageV1::PhysicalCompletion,
                    custody:
                        M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::PhysicalCompletionPreflight(
                            Box::new((coordinator, preflighted, controls, choices, failure)),
                        ),
                }));
            }
        };
        if !matches!(
            physical.outcome(),
            crate::M1AuthenticatedCompletedStepOutcomeV1::Completed(_)
        ) {
            engine.quarantine_m1_queue_rearm_failure();
            return Err(Box::new(M1AuthenticatedSpeculativePhysicalRoundFailureV1 {
                stage: M1AuthenticatedSpeculativePhysicalRoundStageV1::PhysicalCompletion,
                custody: M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::PhysicalOutcome(
                    Box::new((coordinator, preflighted, controls, choices, physical)),
                ),
            }));
        }
        let mut coordinator = coordinator;
        let outcome = match coordinator.commit_preflighted_round(preflighted) {
            Ok(outcome) => outcome,
            Err(failure) => {
                engine.quarantine_m1_queue_rearm_failure();
                return Err(Box::new(M1AuthenticatedSpeculativePhysicalRoundFailureV1 {
                    stage: M1AuthenticatedSpeculativePhysicalRoundStageV1::CoordinatorCommit,
                    custody:
                        M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::CoordinatorCommit(
                            Box::new((coordinator, controls, choices, physical, failure)),
                        ),
                }));
            }
        };
        match physical.release_completed() {
            M1AuthenticatedRearmedRoundReleaseOutcomeV1::Released(released) => {
                Ok(M1AuthenticatedSpeculativePhysicalRoundSuccessV1 {
                    executor: Self {
                        coordinator,
                        released,
                    },
                    outcome,
                    choices,
                })
            }
            M1AuthenticatedRearmedRoundReleaseOutcomeV1::Rejected(failure) => {
                Err(Box::new(M1AuthenticatedSpeculativePhysicalRoundFailureV1 {
                    stage: M1AuthenticatedSpeculativePhysicalRoundStageV1::PageRelease,
                    custody: M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::PageRelease(
                        Box::new(M1AuthenticatedSpeculativePhysicalPageReleaseCustodyV1 {
                            coordinator,
                            outcome,
                            choices,
                            failure: *failure,
                        }),
                    ),
                }))
            }
            M1AuthenticatedRearmedRoundReleaseOutcomeV1::NotCompleted(not_completed) => {
                engine.quarantine_m1_queue_rearm_failure();
                Err(Box::new(M1AuthenticatedSpeculativePhysicalRoundFailureV1 {
                    stage: M1AuthenticatedSpeculativePhysicalRoundStageV1::PageRelease,
                    custody:
                        M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::ReleasedPhysicalOutcome(
                            Box::new((coordinator, outcome, choices, not_completed)),
                        ),
                }))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferric_spec::{Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket};

    fn selection(bucket: Qwen3PlanBucket) -> Qwen3PlanSelection {
        Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Speculative,
            bucket,
        }
    }

    fn coordinator(selection: Qwen3PlanSelection) -> crate::M1SpeculativeGenerationLoopV1 {
        crate::M1SpeculativeGenerationLoopV1::new(
            selection,
            &[crate::M1SpeculativeMemberSeedV1::new(
                RequestId::new(0, 1),
                70,
                10,
                10,
                crate::M1SpeculativeGenerationPolicyV1::new(32, &[999]).unwrap(),
            )],
        )
        .unwrap()
    }

    #[test]
    fn production_entry_profile_gate_covers_all_four_declared_shapes() {
        let cases = [
            (
                Qwen3PlanBucket::SpeculativeS1K4C8192,
                M1PhysicalFixedBatchShapeV1::SpeculativeK4,
            ),
            (
                Qwen3PlanBucket::SpeculativeS8K4C8192,
                M1PhysicalFixedBatchShapeV1::SpeculativeK4,
            ),
            (
                Qwen3PlanBucket::SpeculativeS1K8C8192,
                M1PhysicalFixedBatchShapeV1::SpeculativeK8,
            ),
            (
                Qwen3PlanBucket::SpeculativeS1K16C8192,
                M1PhysicalFixedBatchShapeV1::SpeculativeK16,
            ),
        ];
        for (bucket, expected) in cases {
            assert!(production_entry_profile_matches(
                selection(bucket),
                expected
            ));
        }
        assert!(!production_entry_profile_matches(
            Qwen3PlanSelection {
                role: Qwen3ModelRole::Target8B,
                mode: Qwen3ExecutionMode::Decode,
                bucket: Qwen3PlanBucket::DecodeS1C8192,
            },
            M1PhysicalFixedBatchShapeV1::SpeculativeK4,
        ));
    }

    #[test]
    fn constructor_header_rejects_wrong_coordinator_queue_epoch_and_round() {
        let selected = selection(Qwen3PlanBucket::SpeculativeS1K4C8192);
        let first = coordinator(selected);
        let other = coordinator(selected);
        let request = RequestId::new(0, 1);
        let active = [request];
        let coherent = || M1AuthenticatedSpeculativeAssociationHeaderV1 {
            coordinator_identity: first.identity(),
            prior_identity: first.identity(),
            selection: selected,
            prior_selection: selected,
            checked_selection: selected,
            queue_selection: selected,
            queue_shape: M1PhysicalFixedBatchShapeV1::SpeculativeK4,
            coordinator_last_epoch: Some(CompletionEpoch::new(7)),
            prior_epoch: CompletionEpoch::new(7),
            checked_epoch: CompletionEpoch::new(7),
            coordinator_next_round: 2,
            prior_round: 1,
            active: &active,
            prior_active: &active,
            released_active: &active,
        };
        assert_eq!(validate_prior_association_header(&coherent()), Ok(()));

        let mut hostile = coherent();
        hostile.prior_identity = other.identity();
        assert_eq!(
            validate_prior_association_header(&hostile),
            Err(M1AuthenticatedSpeculativeExecutorInitErrorV1::CoordinatorIdentity),
        );
        let mut hostile = coherent();
        hostile.queue_shape = M1PhysicalFixedBatchShapeV1::SpeculativeK8;
        assert_eq!(
            validate_prior_association_header(&hostile),
            Err(M1AuthenticatedSpeculativeExecutorInitErrorV1::QueueShape),
        );
        let mut hostile = coherent();
        hostile.checked_epoch = CompletionEpoch::new(8);
        assert_eq!(
            validate_prior_association_header(&hostile),
            Err(M1AuthenticatedSpeculativeExecutorInitErrorV1::PriorEpoch),
        );
        let mut hostile = coherent();
        hostile.coordinator_next_round = 3;
        assert_eq!(
            validate_prior_association_header(&hostile),
            Err(M1AuthenticatedSpeculativeExecutorInitErrorV1::PriorRound),
        );
        let mut hostile = coherent();
        hostile.queue_selection = selection(Qwen3PlanBucket::SpeculativeS1K8C8192);
        assert_eq!(
            validate_prior_association_header(&hostile),
            Err(M1AuthenticatedSpeculativeExecutorInitErrorV1::Selection),
        );
        let wrong_roster = [RequestId::new(1, 1)];
        let mut hostile = coherent();
        hostile.released_active = &wrong_roster;
        assert_eq!(
            validate_prior_association_header(&hostile),
            Err(M1AuthenticatedSpeculativeExecutorInitErrorV1::PriorRoster),
        );
    }

    #[test]
    fn production_round_complete_guard_and_cleanup_are_caller_consumable() {
        type Execute = fn(
            M1AuthenticatedSpeculativePhysicalExecutorV1,
            &mut Engine<32>,
            M1AuthenticatedSpeculativePhysicalRoundInputsV1,
        ) -> Result<
            M1AuthenticatedSpeculativePhysicalRoundSuccessV1,
            Box<M1AuthenticatedSpeculativePhysicalRoundFailureV1>,
        >;
        type FailureCleanup = fn(
            M1AuthenticatedSpeculativePhysicalRoundFailureV1,
            &mut Engine<32>,
        ) -> Result<
            Result<
                M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessV1,
                Box<M1AuthenticatedSpeculativePhysicalRoundTeardownFailureV1>,
            >,
            M1AuthenticatedSpeculativePhysicalRoundFailureV1,
        >;
        type FinishedCleanup = fn(
            M1AuthenticatedSpeculativePhysicalExecutorV1,
            &mut Engine<32>,
        ) -> Result<
            M1AuthenticatedSpeculativeExecutorTeardownSuccessV1,
            Box<M1AuthenticatedSpeculativeExecutorTeardownFailureV1>,
        >;

        let _: Execute = M1AuthenticatedSpeculativePhysicalExecutorV1::execute_round::<32>;
        let _: FailureCleanup =
            M1AuthenticatedSpeculativePhysicalRoundFailureV1::destroy_queue_and_retain_custody::<32>;
        let _: FinishedCleanup =
            M1AuthenticatedSpeculativePhysicalExecutorV1::destroy_queue_and_retain_state::<32>;
        assert!(production_entry_has_active_members(1));
        assert!(!production_entry_has_active_members(0));
    }

    #[test]
    fn terminal_quarantine_cleanup_faults_engine_and_preserves_concrete_source() {
        let mut engine = Engine::<1>::new(8, 4, 32).unwrap();
        assert!(!engine.is_faulted());
        let cleanup = terminal_quarantine(
            &mut engine,
            M1AuthenticatedSpeculativePhysicalRoundStageV1::CoordinatorPreflight,
            M1AuthenticatedSpeculativePhysicalRoundTerminalSourceV1::Coordinator(
                crate::M1SpeculativeGenerationLoopErrorV1::NoActiveMembers,
            ),
            ("retained logical lineage", 2_u64),
        )
        .unwrap()
        .unwrap();
        assert!(engine.is_faulted());
        assert_eq!(
            cleanup.stage(),
            M1AuthenticatedSpeculativePhysicalRoundStageV1::CoordinatorPreflight,
        );
        assert!(cleanup.retains_logical_custody());
        assert!(matches!(
            cleanup.source(),
            M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessSourceV1::TerminalQuarantine(
                M1AuthenticatedSpeculativePhysicalRoundTerminalSourceV1::Coordinator(
                    crate::M1SpeculativeGenerationLoopErrorV1::NoActiveMembers,
                ),
            ),
        ));
    }
}
