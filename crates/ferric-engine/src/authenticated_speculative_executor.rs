//! Production authenticated repeated speculative execution.
//!
//! This bridge can start only from an authenticated released queue produced by
//! the normal completion path. It never constructs program, queue-currentness,
//! completion, or verifier authority.
//! Unit tests cover the public production entry shape and custody transitions
//! without manufacturing that authority; hardware-backed execution still has
//! to enter through the normal verified-launch path.

use core::fmt;
use core::sync::atomic::{AtomicU64, Ordering};

use ferric_spec::{
    completion::CompletionEpoch, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket,
    Qwen3PlanSelection, RequestId, ValidatedM1StepInputs,
};

use crate::{
    complete_m1_authenticated_physical_step_v1, prepare_m1_authenticated_long_lived_queue_rearm_v1,
    release_m1_authenticated_completed_step_kv_pages_v1,
    reserve_m1_authenticated_long_lived_queue_rearm_kv_v1,
    submit_m1_authenticated_long_lived_queue_rearm_v1, ActiveDeviceKvCache,
    AddresslessM1PhysicalBufferRecipeV1, Engine, LogicalRunnerDeclaration,
    M1AllocatedScheduledStepV1, M1AuthenticatedCompletedStepOutcomeV1,
    M1AuthenticatedLongLivedQueueRearmScheduleFailureV1,
    M1AuthenticatedLongLivedQueueReleasedRoundV1, M1AuthenticatedPhysicalQueueCreateFailureV1,
    M1AuthenticatedPhysicalQueueSessionV1, M1AuthenticatedPhysicalRunnerV1,
    M1AuthenticatedPrepublicationBatchV1, M1AuthenticatedRearmedRoundPageReleaseFailureV1,
    M1AuthenticatedRearmedRoundReleaseOutcomeV1,
    M1AuthenticatedSpeculativeDiagnosticCompletedReadbackV1, M1DeviceKvCompletionMemberV1,
    M1DeviceKvCompletionRosterV1, M1FullStepKvWorkspaceTablesV1, M1FullStepWorkspacePlans,
    M1LongLivedQueueRearmKvInputsV1, M1ObservedSpeculativeDiagnosticChoicesV1,
    M1PartitionedModelMemoryKvPoolV1, M1PhysicalFixedBatchShapeV1, M1PhysicalRunnerRecipeOutcomeV1,
    M1PrepareFailureV1, M1PreparedScheduledWorkspaceImagesV1, M1ReleasedDeviceKvMemberV1,
    M1ScheduledDispatchV1, M1SpeculativeGenerationLoopV1, M1SpeculativeMemberControlV1,
    M1SpeculativeMemberSeedV1, M1SpeculativeMemberStatusV1, M1SpeculativeRoundOutcomeV1,
};

static NEXT_AUTHENTICATED_SPECULATIVE_LINEAGE_ID_V1: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct M1AuthenticatedSpeculativeLineageIdentityV1(u64);

impl M1AuthenticatedSpeculativeLineageIdentityV1 {
    pub(crate) fn fresh() -> Option<Self> {
        let identity = NEXT_AUTHENTICATED_SPECULATIVE_LINEAGE_ID_V1.fetch_update(
            Ordering::Relaxed,
            Ordering::Relaxed,
            |current| current.checked_add(1),
        );
        identity.ok().filter(|identity| *identity != 0).map(Self)
    }
}

/// Private move-only proof that a round-zero coordinator binding entered one
/// exact authenticated physical queue before publication.
#[derive(Debug)]
pub(crate) struct M1AuthenticatedSpeculativePhysicalLineageWitnessV1 {
    identity: M1AuthenticatedSpeculativeLineageIdentityV1,
    coordinator_identity: crate::speculative_generation_loop::M1SpeculativeCoordinatorIdentityV1,
    selection: Qwen3PlanSelection,
    round: u64,
    epoch: CompletionEpoch,
    initial_seeds: Box<[M1SpeculativeMemberSeedV1]>,
}

#[derive(Debug)]
pub(crate) struct M1AuthenticatedSpeculativeLogicalLineageWitnessV1 {
    identity: M1AuthenticatedSpeculativeLineageIdentityV1,
}

#[derive(Debug)]
pub struct M1AuthenticatedSpeculativeCausalLineageV1 {
    logical: M1AuthenticatedSpeculativeLogicalLineageWitnessV1,
    coordinator_identity: crate::speculative_generation_loop::M1SpeculativeCoordinatorIdentityV1,
    selection: Qwen3PlanSelection,
    initial_seeds: Box<[M1SpeculativeMemberSeedV1]>,
    generated: Box<[(RequestId, u32)]>,
    completed_rounds: u64,
    last_epoch: CompletionEpoch,
}

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

/// Linear production owner of a verified authenticated queue and its logical coordinator.
#[must_use = "the authenticated executor must execute another round or be retained"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativePhysicalExecutorV1 {
    coordinator: M1SpeculativeGenerationLoopV1,
    released: M1AuthenticatedLongLivedQueueReleasedRoundV1,
    lineage: M1AuthenticatedSpeculativeCausalLineageV1,
}

/// Clean queue teardown retaining the final logical coordinator state.
#[must_use = "final coordinator state and authenticated release remain retained"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativeExecutorTeardownSuccessV1 {
    coordinator: M1SpeculativeGenerationLoopV1,
    released: crate::M1AuthenticatedLongLivedQueueRearmTeardownSuccessV1,
    lineage: M1AuthenticatedSpeculativeCausalLineageV1,
}

/// Terminal queue-release quarantine retaining the final logical state.
#[must_use = "final coordinator state and release quarantine remain retained"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativeExecutorTeardownFailureV1 {
    coordinator: M1SpeculativeGenerationLoopV1,
    released: Box<crate::M1AuthenticatedLongLivedQueueRearmTeardownFailureV1>,
    lineage: M1AuthenticatedSpeculativeCausalLineageV1,
}

impl M1AuthenticatedSpeculativeExecutorTeardownSuccessV1 {
    pub const fn released(&self) -> &crate::M1AuthenticatedLongLivedQueueRearmTeardownSuccessV1 {
        &self.released
    }

    #[must_use]
    pub const fn coordinator(&self) -> &M1SpeculativeGenerationLoopV1 {
        &self.coordinator
    }

    #[must_use]
    pub const fn retains_causal_lineage(&self) -> bool {
        let _ = &self.lineage;
        true
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

    #[must_use]
    pub const fn retains_causal_lineage(&self) -> bool {
        let _ = &self.lineage;
        true
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

/// Pure authenticated round-zero bootstrap rejection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedSpeculativeBootstrapErrorV1 {
    Coordinator,
    Inputs,
    CacheRoster,
    LineageIdentityExhausted,
    Preparation,
    LineageAttachment,
}

#[derive(Debug)]
pub enum M1AuthenticatedSpeculativeBootstrapFailureCustodyV1 {
    Unprepared(
        Box<(
            M1SpeculativeGenerationLoopV1,
            Vec<ActiveDeviceKvCache>,
            M1ScheduledDispatchV1,
            M1FullStepWorkspacePlans,
            M1FullStepKvWorkspaceTablesV1,
        )>,
    ),
    Preparation(
        Box<(
            M1SpeculativeGenerationLoopV1,
            crate::M1SpeculativeRoundBindingV1,
            Vec<ActiveDeviceKvCache>,
            M1PrepareFailureV1,
        )>,
    ),
    Attachment(
        Box<(
            M1SpeculativeGenerationLoopV1,
            crate::M1SpeculativeRoundBindingV1,
            Vec<ActiveDeviceKvCache>,
            M1PreparedScheduledWorkspaceImagesV1,
        )>,
    ),
}

/// Bootstrap failure retaining every scheduler, KV, cache, and coordinator owner.
#[must_use = "bootstrap rejection retains all linear inputs"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativeBootstrapFailureV1 {
    error: M1AuthenticatedSpeculativeBootstrapErrorV1,
    custody: M1AuthenticatedSpeculativeBootstrapFailureCustodyV1,
}

impl M1AuthenticatedSpeculativeBootstrapFailureV1 {
    #[must_use]
    pub const fn error(&self) -> M1AuthenticatedSpeculativeBootstrapErrorV1 {
        self.error
    }

    #[must_use]
    pub const fn retains_all_custody(&self) -> bool {
        match &self.custody {
            M1AuthenticatedSpeculativeBootstrapFailureCustodyV1::Unprepared(_)
            | M1AuthenticatedSpeculativeBootstrapFailureCustodyV1::Preparation(_)
            | M1AuthenticatedSpeculativeBootstrapFailureCustodyV1::Attachment(_) => true,
        }
    }

    #[must_use = "all rejected bootstrap custody remains linear"]
    pub fn into_custody(self) -> M1AuthenticatedSpeculativeBootstrapFailureCustodyV1 {
        self.custody
    }
}

/// Logical half of a round-zero provenance join.
///
/// It is useful only when joined later to the physical half carried through
/// the exact authenticated queue publication and readback.
#[must_use = "bootstrap continuation must rejoin its authenticated physical lineage"]
#[derive(Debug)]
struct M1AuthenticatedSpeculativeBootstrapContinuationV1 {
    coordinator: M1SpeculativeGenerationLoopV1,
    epoch: CompletionEpoch,
    selected: Vec<ActiveDeviceKvCache>,
    lineage: M1AuthenticatedSpeculativeLogicalLineageWitnessV1,
}

/// Prepared round-zero images plus the only logical continuation able to join them.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedSpeculativeBootstrapPreparedV1;
/// fn separate(value: M1AuthenticatedSpeculativeBootstrapPreparedV1) {
///     let _ = value.into_parts();
/// }
/// ```
#[must_use = "prepared physical custody and logical bootstrap continuation remain linear"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativeBootstrapPreparedV1 {
    prepared: M1PreparedScheduledWorkspaceImagesV1,
    continuation: M1AuthenticatedSpeculativeBootstrapContinuationV1,
}

/// Logical half of an authenticated paired-prefill rollover lineage join.
#[must_use = "rollover continuation must rejoin the first speculative readback"]
#[derive(Debug)]
struct M1AuthenticatedSpeculativeRolloverContinuationV1 {
    coordinator: M1SpeculativeGenerationLoopV1,
    epoch: CompletionEpoch,
    lineage: M1AuthenticatedSpeculativeLogicalLineageWitnessV1,
}

/// Published first speculative generation plus its only logical continuation.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedSpeculativeRolloverPublishedV1;
/// fn separate(value: M1AuthenticatedSpeculativeRolloverPublishedV1) {
///     let _ = value.into_parts();
/// }
/// ```
#[must_use = "published rollover queue and continuation must remain paired"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativeRolloverPublishedV1 {
    published: crate::M1AuthenticatedRearmedPublishedQueueV1,
    continuation: M1AuthenticatedSpeculativeRolloverContinuationV1,
}

/// Terminal first-round rollover failure without reusable queue authority.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedSpeculativeRolloverRoundFailureV1;
/// fn recover_completed_owner(failure: M1AuthenticatedSpeculativeRolloverRoundFailureV1) {
///     let _completed = failure.into_completed_step();
/// }
/// ```
#[must_use = "terminal rollover failure custody remains retained"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativeRolloverRoundFailureV1 {
    stage: M1AuthenticatedSpeculativePhysicalRoundStageV1,
    disposition: M1AuthenticatedSpeculativeFailureDispositionV1,
}

impl M1AuthenticatedSpeculativeRolloverRoundFailureV1 {
    #[must_use]
    pub const fn stage(&self) -> M1AuthenticatedSpeculativePhysicalRoundStageV1 {
        self.stage
    }

    #[must_use = "the terminal disposition must remain observed"]
    pub const fn disposition(&self) -> &M1AuthenticatedSpeculativeFailureDispositionV1 {
        &self.disposition
    }
}

/// Internal first-round rollover custody pending mandatory terminal closure.
#[derive(Debug)]
enum PendingM1AuthenticatedSpeculativeRolloverRoundFailureV1 {
    Unsettled {
        stage: M1AuthenticatedSpeculativePhysicalRoundStageV1,
        retained: Box<(
            M1AuthenticatedSpeculativeRolloverContinuationV1,
            crate::M1AuthenticatedRearmedSpeculativeDiagnosticCompletedReadbackV1,
            Vec<M1SpeculativeMemberControlV1>,
        )>,
    },
    Round(Box<PendingM1AuthenticatedSpeculativePhysicalRoundFailureV1>),
}

impl M1AuthenticatedSpeculativeRolloverPublishedV1 {
    pub(crate) const fn new(
        published: crate::M1AuthenticatedRearmedPublishedQueueV1,
        coordinator: M1SpeculativeGenerationLoopV1,
        epoch: CompletionEpoch,
        lineage: M1AuthenticatedSpeculativeLogicalLineageWitnessV1,
    ) -> Self {
        Self {
            published,
            continuation: M1AuthenticatedSpeculativeRolloverContinuationV1 {
                coordinator,
                epoch,
                lineage,
            },
        }
    }

    /// Drives wait, recycle, diagnostic readback, and coordinator completion
    /// without ever exposing the generic published queue independently.
    ///
    /// # Errors
    ///
    /// Every failure closes available queue custody, permanently faults
    /// `engine`, and returns only a clean release or opaque quarantine.
    pub fn complete_round<const C: usize>(
        self,
        engine: &mut Engine<C>,
        controls: Vec<M1SpeculativeMemberControlV1>,
    ) -> Result<
        M1AuthenticatedSpeculativePhysicalRoundSuccessV1,
        M1AuthenticatedSpeculativeRolloverRoundFailureV1,
    > {
        let Self {
            published,
            continuation,
        } = self;
        let (diagnostic, (continuation, controls)) =
            complete_round_core::<M1NativeRearmedQueueEffectsV1, _, C>(
                engine,
                published,
                (continuation, controls),
            )
            .map_err(|(stage, disposition)| M1AuthenticatedSpeculativeRolloverRoundFailureV1 {
                stage,
                disposition,
            })?;
        continuation.complete_rollover_round(engine, diagnostic, controls)
    }
}

/// Round-zero completion failure stage after authenticated bootstrap.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedSpeculativeBootstrapRoundStageV1 {
    EngineFaulted,
    WorkspaceAllocation,
    CompletionOutput,
    DiagnosticCapture,
    Prepublication,
    QueueCreate,
    QueueSubmit,
    QueueWait,
    SignalRecycle,
    CompletionObservation,
    DiagnosticObservation,
    SemanticJoin,
    Lineage,
    CoordinatorPreflight,
    HostAllocation,
    PhysicalCompletion,
    CoordinatorCommit,
    PageRelease,
}

/// Terminal round-zero failure with no recoverable queue or readback authority.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedSpeculativeBootstrapRoundFailureV1;
/// fn recover_readback(failure: M1AuthenticatedSpeculativeBootstrapRoundFailureV1) {
///     let _readback = failure.into_custody();
/// }
/// ```
#[must_use = "bootstrap retry or terminal custody remains retained"]
pub enum M1AuthenticatedSpeculativeBootstrapRoundFailureV1 {
    /// Pure rejection before native queue creation with exact retry custody.
    PreDetach {
        stage: M1AuthenticatedSpeculativeBootstrapRoundStageV1,
        retry: Box<M1AuthenticatedSpeculativeBootstrapPreDetachRetryV1>,
    },
    /// Failure after native queue creation began, or an intentionally terminal
    /// earlier failure after the Engine was explicitly quarantined.
    Terminal {
        stage: M1AuthenticatedSpeculativeBootstrapRoundStageV1,
        disposition: M1AuthenticatedSpeculativeFailureDispositionV1,
    },
}

impl M1AuthenticatedSpeculativeBootstrapRoundFailureV1 {
    #[must_use]
    pub const fn stage(&self) -> M1AuthenticatedSpeculativeBootstrapRoundStageV1 {
        match self {
            Self::PreDetach { stage, .. } | Self::Terminal { stage, .. } => *stage,
        }
    }

    #[must_use]
    pub const fn is_pre_detach_retry(&self) -> bool {
        matches!(self, Self::PreDetach { .. })
    }

    #[must_use = "the terminal disposition must remain observed when present"]
    pub const fn disposition(&self) -> Option<&M1AuthenticatedSpeculativeFailureDispositionV1> {
        match self {
            Self::PreDetach { .. } => None,
            Self::Terminal { disposition, .. } => Some(disposition),
        }
    }
}

impl fmt::Debug for M1AuthenticatedSpeculativeBootstrapRoundFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedSpeculativeBootstrapRoundFailureV1")
            .field("stage", &self.stage())
            .field("pre_detach_retry", &self.is_pre_detach_retry())
            .field("terminal_disposition", &self.disposition())
            .finish()
    }
}

/// Opaque exact inputs for retrying a bootstrap rejection that occurred before
/// native queue creation began.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedSpeculativeBootstrapPreDetachRetryV1;
/// fn extract(retry: M1AuthenticatedSpeculativeBootstrapPreDetachRetryV1) {
///     let _generic_queue_or_inputs = retry.into_parts();
/// }
/// ```
#[must_use = "pre-detach bootstrap retry custody remains linear"]
pub struct M1AuthenticatedSpeculativeBootstrapPreDetachRetryV1 {
    state: M1AuthenticatedSpeculativeBootstrapPreDetachRetryStateV1,
}

impl fmt::Debug for M1AuthenticatedSpeculativeBootstrapPreDetachRetryV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedSpeculativeBootstrapPreDetachRetryV1")
            .field("retains_exact_inputs", &true)
            .finish()
    }
}

#[allow(clippy::large_enum_variant)]
enum M1AuthenticatedSpeculativeBootstrapPreDetachRetryStateV1 {
    Allocation {
        diagnostic: Box<dyn fmt::Debug>,
        prepared: M1AuthenticatedSpeculativeBootstrapPreparedV1,
        partitioned_memory: M1PartitionedModelMemoryKvPoolV1,
        runner: M1AuthenticatedPhysicalRunnerV1,
        recipe: AddresslessM1PhysicalBufferRecipeV1,
        ring_bytes: u32,
        controls: Vec<M1SpeculativeMemberControlV1>,
    },
    Prepublication {
        diagnostic: Box<dyn fmt::Debug>,
        continuation: M1AuthenticatedSpeculativeBootstrapContinuationV1,
        runner: M1AuthenticatedPhysicalRunnerV1,
        allocated: M1AllocatedScheduledStepV1,
        recipe: AddresslessM1PhysicalBufferRecipeV1,
        completion: crate::BoundM1CompletionOutputV1,
        ring_bytes: u32,
        controls: Vec<M1SpeculativeMemberControlV1>,
    },
    QueueCreate {
        diagnostic: Box<dyn fmt::Debug>,
        continuation: M1AuthenticatedSpeculativeBootstrapContinuationV1,
        prepublication: M1AuthenticatedPrepublicationBatchV1,
        ring_bytes: u32,
        controls: Vec<M1SpeculativeMemberControlV1>,
    },
}

/// Internal round-zero custody pending mandatory terminal closure.
#[allow(clippy::type_complexity)]
#[derive(Debug)]
enum PendingM1AuthenticatedSpeculativeBootstrapRoundFailureCustodyV1 {
    Retryable(
        Box<(
            M1AuthenticatedSpeculativeBootstrapContinuationV1,
            M1AuthenticatedSpeculativeDiagnosticCompletedReadbackV1,
            Vec<M1SpeculativeMemberControlV1>,
        )>,
    ),
    PhysicalOutcome(
        Box<(
            M1SpeculativeGenerationLoopV1,
            M1AuthenticatedSpeculativeCausalLineageV1,
            Vec<M1SpeculativeMemberControlV1>,
            M1ObservedSpeculativeDiagnosticChoicesV1,
            crate::M1SpeculativePreflightedRoundV1,
            M1AuthenticatedCompletedStepOutcomeV1,
        )>,
    ),
    CoordinatorCommit(
        Box<(
            M1SpeculativeGenerationLoopV1,
            M1AuthenticatedSpeculativeCausalLineageV1,
            Vec<M1SpeculativeMemberControlV1>,
            M1ObservedSpeculativeDiagnosticChoicesV1,
            crate::M1AuthenticatedCompletedStepSuccessV1,
            Box<crate::M1SpeculativePreparedRoundCommitFailureV1>,
        )>,
    ),
    LineageAfterCommit(
        Box<(
            M1SpeculativeGenerationLoopV1,
            M1AuthenticatedSpeculativeCausalLineageV1,
            Vec<M1SpeculativeMemberControlV1>,
            M1ObservedSpeculativeDiagnosticChoicesV1,
            crate::M1AuthenticatedCompletedStepSuccessV1,
            M1SpeculativeRoundOutcomeV1,
        )>,
    ),
    PageRelease(
        Box<(
            M1SpeculativeGenerationLoopV1,
            M1AuthenticatedSpeculativeCausalLineageV1,
            M1SpeculativeRoundOutcomeV1,
            M1ObservedSpeculativeDiagnosticChoicesV1,
            Box<crate::M1AuthenticatedCompletedStepKvReleaseFailureV1>,
        )>,
    ),
    ReleasedLineage(
        Box<(
            M1SpeculativeGenerationLoopV1,
            M1AuthenticatedSpeculativeCausalLineageV1,
            M1SpeculativeRoundOutcomeV1,
            M1ObservedSpeculativeDiagnosticChoicesV1,
            M1AuthenticatedLongLivedQueueReleasedRoundV1,
        )>,
    ),
}

/// Internal failed round-zero completion before terminal closure.
#[derive(Debug)]
struct PendingM1AuthenticatedSpeculativeBootstrapRoundFailureV1 {
    stage: M1AuthenticatedSpeculativeBootstrapRoundStageV1,
    custody: PendingM1AuthenticatedSpeculativeBootstrapRoundFailureCustodyV1,
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
    CausalLineage,
    PageRelease,
}

/// Proof that a failed speculative transition destroyed its queue cleanly.
///
/// The lower queue and all logical/KV owners remain sealed inside this object;
/// callers receive no schedule, completion, readback, or resubmission authority.
#[must_use = "clean queue-release evidence retains all terminal custody"]
pub struct M1AuthenticatedSpeculativeCleanReleaseV1 {
    retained: Box<dyn fmt::Debug>,
}

impl fmt::Debug for M1AuthenticatedSpeculativeCleanReleaseV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedSpeculativeCleanReleaseV1")
            .field("queue_released", &true)
            .field("engine_quarantined", &true)
            .field("custody_sealed", &true)
            .finish_non_exhaustive()
    }
}

impl M1AuthenticatedSpeculativeCleanReleaseV1 {
    #[must_use]
    pub const fn queue_released(&self) -> bool {
        true
    }

    #[must_use]
    pub const fn engine_quarantined(&self) -> bool {
        true
    }

    #[must_use]
    pub fn retains_custody(&self) -> bool {
        let _ = &self.retained;
        true
    }
}

/// Opaque terminal custody after a failed speculative transition.
///
/// Construction always follows Engine quarantine. No contained queue or
/// readback owner is accessible through the public API.
#[must_use = "terminal quarantine retains all unavailable custody"]
pub struct M1AuthenticatedSpeculativeTerminalQuarantineV1 {
    retained: Box<dyn fmt::Debug>,
}

impl fmt::Debug for M1AuthenticatedSpeculativeTerminalQuarantineV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedSpeculativeTerminalQuarantineV1")
            .field("engine_quarantined", &true)
            .field("custody_sealed", &true)
            .finish_non_exhaustive()
    }
}

impl M1AuthenticatedSpeculativeTerminalQuarantineV1 {
    #[must_use]
    pub const fn engine_quarantined(&self) -> bool {
        true
    }

    #[must_use]
    pub fn retains_custody(&self) -> bool {
        let _ = &self.retained;
        true
    }
}

/// Exhaustive terminal disposition of a failed speculative queue transition.
#[must_use = "released or quarantined custody remains retained"]
#[derive(Debug)]
pub enum M1AuthenticatedSpeculativeFailureDispositionV1 {
    Released(M1AuthenticatedSpeculativeCleanReleaseV1),
    Quarantined(M1AuthenticatedSpeculativeTerminalQuarantineV1),
}

fn disposition_with_logical(
    closure: crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1,
    logical: impl fmt::Debug + 'static,
) -> M1AuthenticatedSpeculativeFailureDispositionV1 {
    use crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1;
    match closure {
        M1AuthenticatedPhysicalQueueClosureV1::Released(released) => {
            released_disposition((released, logical))
        }
        M1AuthenticatedPhysicalQueueClosureV1::Quarantined(quarantined) => {
            quarantined_disposition((quarantined, logical))
        }
    }
}

trait M1InitialQueueEffectsV1 {
    type Prepared;
    type Published;
    type Completed;
    type Recycled;
    type Observed;
    type DiagnosticObserved;
    type Diagnostic;
    type SubmitFailure: fmt::Debug + 'static;
    type WaitFailure: fmt::Debug + 'static;
    type RecycleFailure: fmt::Debug + 'static;
    type ObservationFailure: fmt::Debug + 'static;
    type DiagnosticObservationFailure: fmt::Debug + 'static;
    type JoinFailure: fmt::Debug + 'static;

    fn submit(
        prepared: Self::Prepared,
    ) -> Result<Self::Published, Self::SubmitFailure>;
    fn close_submit_failure<const C: usize>(
        engine: &mut Engine<C>,
        failure: Self::SubmitFailure,
    ) -> crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1;
    fn wait(published: Self::Published) -> Result<Self::Completed, Self::WaitFailure>;
    fn recycle(completed: Self::Completed) -> Result<Self::Recycled, Self::RecycleFailure>;
    fn observe(recycled: Self::Recycled) -> Result<Self::Observed, Self::ObservationFailure>;
    fn close_observation_failure<const C: usize>(
        engine: &mut Engine<C>,
        failure: Self::ObservationFailure,
    ) -> crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1;
    fn observe_diagnostic(
        observed: Self::Observed,
    ) -> Result<Self::DiagnosticObserved, Self::DiagnosticObservationFailure>;
    fn close_diagnostic_observation_failure<const C: usize>(
        engine: &mut Engine<C>,
        failure: Self::DiagnosticObservationFailure,
    ) -> crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1;
    fn check(
        observed: Self::DiagnosticObserved,
    ) -> Result<Self::Diagnostic, Self::JoinFailure>;
    fn close_join_failure<const C: usize>(
        engine: &mut Engine<C>,
        failure: Self::JoinFailure,
    ) -> crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1;
}

struct M1NativeInitialQueueEffectsV1;

impl M1InitialQueueEffectsV1 for M1NativeInitialQueueEffectsV1 {
    type Prepared = M1AuthenticatedPhysicalQueueSessionV1;
    type Published = crate::M1AuthenticatedPhysicalPublishedQueueSessionV1;
    type Completed = crate::M1AuthenticatedPhysicalCompletedQueueSessionV1;
    type Recycled = crate::M1AuthenticatedPhysicalRecycledQueueSessionV1;
    type Observed = crate::M1AuthenticatedObservedCompletionOutputV1;
    type DiagnosticObserved = crate::M1AuthenticatedObservedSpeculativeDiagnosticOutputV1;
    type Diagnostic = M1AuthenticatedSpeculativeDiagnosticCompletedReadbackV1;
    type SubmitFailure = crate::M1AuthenticatedPhysicalQueueSubmitFailureV1;
    type WaitFailure = Box<crate::M1AuthenticatedPhysicalQueueOperationFailureV1>;
    type RecycleFailure = Box<crate::M1AuthenticatedPhysicalQueueOperationFailureV1>;
    type ObservationFailure = crate::M1AuthenticatedCompletionObservationFailureV1;
    type DiagnosticObservationFailure =
        Box<crate::M1AuthenticatedSpeculativeDiagnosticObservationFailureV1>;
    type JoinFailure = Box<crate::M1AuthenticatedSpeculativeDiagnosticCompletedReadbackJoinFailureV1>;

    fn submit(
        prepared: Self::Prepared,
    ) -> Result<Self::Published, Self::SubmitFailure> {
        prepared.submit()
    }

    fn close_submit_failure<const C: usize>(
        engine: &mut Engine<C>,
        failure: Self::SubmitFailure,
    ) -> crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1 {
        failure.close_without_authority(engine)
    }

    fn wait(published: Self::Published) -> Result<Self::Completed, Self::WaitFailure> {
        published.wait()
    }

    fn recycle(completed: Self::Completed) -> Result<Self::Recycled, Self::RecycleFailure> {
        completed.recycle()
    }

    fn observe(recycled: Self::Recycled) -> Result<Self::Observed, Self::ObservationFailure> {
        recycled.observe_completion()
    }

    fn close_observation_failure<const C: usize>(
        engine: &mut Engine<C>,
        failure: Self::ObservationFailure,
    ) -> crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1 {
        match failure.destroy_queue_and_retain_evidence(engine) {
            Ok(released) => crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1::Released(Box::new(released)),
            Err(quarantined) => crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1::Quarantined(Box::new(quarantined)),
        }
    }

    fn observe_diagnostic(
        observed: Self::Observed,
    ) -> Result<Self::DiagnosticObserved, Self::DiagnosticObservationFailure> {
        observed.observe_speculative_diagnostic_choices()
    }

    fn close_diagnostic_observation_failure<const C: usize>(
        engine: &mut Engine<C>,
        failure: Self::DiagnosticObservationFailure,
    ) -> crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1 {
        match (*failure).destroy_queue_and_retain_evidence(engine) {
            Ok(released) => crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1::Released(Box::new(released)),
            Err(quarantined) => crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1::Quarantined(Box::new(quarantined)),
        }
    }

    fn check(
        observed: Self::DiagnosticObserved,
    ) -> Result<Self::Diagnostic, Self::JoinFailure> {
        observed.check_completion()
    }

    fn close_join_failure<const C: usize>(
        engine: &mut Engine<C>,
        failure: Self::JoinFailure,
    ) -> crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1 {
        match (*failure).destroy_queue_and_retain_evidence(engine) {
            Ok(released) => crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1::Released(Box::new(released)),
            Err(quarantined) => crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1::Quarantined(Box::new(quarantined)),
        }
    }
}

fn execute_initial_round_core<A, L, const C: usize>(
    engine: &mut Engine<C>,
    prepared: A::Prepared,
    logical: L,
) -> Result<
    (A::Diagnostic, L),
    (
        M1AuthenticatedSpeculativeBootstrapRoundStageV1,
        M1AuthenticatedSpeculativeFailureDispositionV1,
    ),
>
where
    A: M1InitialQueueEffectsV1,
    L: fmt::Debug + 'static,
{
    let published = match A::submit(prepared) {
        Ok(published) => published,
        Err(failure) => {
            let closure = A::close_submit_failure(engine, failure);
            return Err((
                M1AuthenticatedSpeculativeBootstrapRoundStageV1::QueueSubmit,
                disposition_with_logical(closure, logical),
            ));
        }
    };
    let completed = match A::wait(published) {
        Ok(completed) => completed,
        Err(failure) => {
            engine.quarantine_m1_queue_rearm_failure();
            return Err((
                M1AuthenticatedSpeculativeBootstrapRoundStageV1::QueueWait,
                quarantined_disposition((failure, logical)),
            ));
        }
    };
    let recycled = match A::recycle(completed) {
        Ok(recycled) => recycled,
        Err(failure) => {
            engine.quarantine_m1_queue_rearm_failure();
            return Err((
                M1AuthenticatedSpeculativeBootstrapRoundStageV1::SignalRecycle,
                quarantined_disposition((failure, logical)),
            ));
        }
    };
    let observed = match A::observe(recycled) {
        Ok(observed) => observed,
        Err(failure) => {
            let closure = A::close_observation_failure(engine, failure);
            return Err((
                M1AuthenticatedSpeculativeBootstrapRoundStageV1::CompletionObservation,
                disposition_with_logical(closure, logical),
            ));
        }
    };
    let observed = match A::observe_diagnostic(observed) {
        Ok(observed) => observed,
        Err(failure) => {
            let closure = A::close_diagnostic_observation_failure(engine, failure);
            return Err((
                M1AuthenticatedSpeculativeBootstrapRoundStageV1::DiagnosticObservation,
                disposition_with_logical(closure, logical),
            ));
        }
    };
    match A::check(observed) {
        Ok(diagnostic) => Ok((diagnostic, logical)),
        Err(failure) => {
            let closure = A::close_join_failure(engine, failure);
            Err((
                M1AuthenticatedSpeculativeBootstrapRoundStageV1::SemanticJoin,
                disposition_with_logical(closure, logical),
            ))
        }
    }
}

trait M1RearmedQueueEffectsV1 {
    type Prepared;
    type Published;
    type Completed;
    type Recycled;
    type Diagnostic;
    type SubmitFailure: fmt::Debug + 'static;
    type ProgressFailure: fmt::Debug + 'static;
    type ReadbackFailure: fmt::Debug + 'static;

    fn submit<const C: usize>(
        engine: &mut Engine<C>,
        prepared: Self::Prepared,
    ) -> Result<Self::Published, Self::SubmitFailure>;
    fn classify_submit_failure(
        failure: Self::SubmitFailure,
    ) -> crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1;
    fn wait<const C: usize>(
        engine: &mut Engine<C>,
        published: Self::Published,
    ) -> Result<Self::Completed, Self::ProgressFailure>;
    fn recycle<const C: usize>(
        engine: &mut Engine<C>,
        completed: Self::Completed,
    ) -> Result<Self::Recycled, Self::ProgressFailure>;
    fn readback(
        recycled: Self::Recycled,
    ) -> Result<Self::Diagnostic, Self::ReadbackFailure>;
    fn close_readback_failure<const C: usize>(
        engine: &mut Engine<C>,
        failure: Self::ReadbackFailure,
    ) -> crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1;
}

struct M1NativeRearmedQueueEffectsV1;

impl M1RearmedQueueEffectsV1 for M1NativeRearmedQueueEffectsV1 {
    type Prepared = (
        crate::M1AuthenticatedPreparedLongLivedQueueRearmV1,
        AddresslessM1PhysicalBufferRecipeV1,
    );
    type Published = crate::M1AuthenticatedRearmedPublishedQueueV1;
    type Completed = crate::M1AuthenticatedRearmedCompletedQueueV1;
    type Recycled = crate::M1AuthenticatedRearmedRecycledQueueV1;
    type Diagnostic = crate::M1AuthenticatedRearmedSpeculativeDiagnosticCompletedReadbackV1;
    type SubmitFailure = crate::M1AuthenticatedLongLivedQueueRearmSubmissionFailureV1;
    type ProgressFailure = Box<crate::M1AuthenticatedRearmedQueueProgressFailureV1>;
    type ReadbackFailure = Box<crate::M1AuthenticatedRearmedSpeculativeDiagnosticReadbackFailureV1>;

    fn submit<const C: usize>(
        engine: &mut Engine<C>,
        (prepared, recipe): Self::Prepared,
    ) -> Result<Self::Published, Self::SubmitFailure> {
        submit_m1_authenticated_long_lived_queue_rearm_v1(engine, prepared, recipe)
    }

    fn classify_submit_failure(
        failure: Self::SubmitFailure,
    ) -> crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1 {
        if failure.queue_released() {
            crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1::Released(
                Box::new(failure),
            )
        } else {
            crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1::Quarantined(
                Box::new(failure),
            )
        }
    }

    fn wait<const C: usize>(
        engine: &mut Engine<C>,
        published: Self::Published,
    ) -> Result<Self::Completed, Self::ProgressFailure> {
        published.wait(engine)
    }

    fn recycle<const C: usize>(
        engine: &mut Engine<C>,
        completed: Self::Completed,
    ) -> Result<Self::Recycled, Self::ProgressFailure> {
        completed.recycle(engine)
    }

    fn readback(
        recycled: Self::Recycled,
    ) -> Result<Self::Diagnostic, Self::ReadbackFailure> {
        recycled.read_and_check_speculative_diagnostic_completion()
    }

    fn close_readback_failure<const C: usize>(
        engine: &mut Engine<C>,
        failure: Self::ReadbackFailure,
    ) -> crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1 {
        match failure.destroy_queue_and_retain_custody(engine) {
            Ok(released) => crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1::Released(Box::new(released)),
            Err(quarantined) => crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1::Quarantined(Box::new(quarantined)),
        }
    }
}

fn complete_round_core<A, L, const C: usize>(
    engine: &mut Engine<C>,
    published: A::Published,
    logical: L,
) -> Result<
    (A::Diagnostic, L),
    (
        M1AuthenticatedSpeculativePhysicalRoundStageV1,
        M1AuthenticatedSpeculativeFailureDispositionV1,
    ),
>
where
    A: M1RearmedQueueEffectsV1,
    L: fmt::Debug + 'static,
{
    let completed = match A::wait(engine, published) {
        Ok(completed) => completed,
        Err(failure) => {
            engine.quarantine_m1_queue_rearm_failure();
            return Err((
                M1AuthenticatedSpeculativePhysicalRoundStageV1::Wait,
                quarantined_disposition((failure, logical)),
            ));
        }
    };
    let recycled = match A::recycle(engine, completed) {
        Ok(recycled) => recycled,
        Err(failure) => {
            engine.quarantine_m1_queue_rearm_failure();
            return Err((
                M1AuthenticatedSpeculativePhysicalRoundStageV1::Recycle,
                quarantined_disposition((failure, logical)),
            ));
        }
    };
    match A::readback(recycled) {
        Ok(diagnostic) => Ok((diagnostic, logical)),
        Err(failure) => {
            engine.quarantine_m1_queue_rearm_failure();
            let closure = A::close_readback_failure(engine, failure);
            Err((
                M1AuthenticatedSpeculativePhysicalRoundStageV1::DiagnosticReadback,
                disposition_with_logical(closure, logical),
            ))
        }
    }
}

fn execute_round_core<A, L, const C: usize>(
    engine: &mut Engine<C>,
    prepared: A::Prepared,
    logical: L,
) -> Result<
    (A::Diagnostic, L),
    (
        M1AuthenticatedSpeculativePhysicalRoundStageV1,
        M1AuthenticatedSpeculativeFailureDispositionV1,
    ),
>
where
    A: M1RearmedQueueEffectsV1,
    L: fmt::Debug + 'static,
{
    let published = match A::submit(engine, prepared) {
        Ok(published) => published,
        Err(failure) => {
            engine.quarantine_m1_queue_rearm_failure();
            let closure = A::classify_submit_failure(failure);
            return Err((
                M1AuthenticatedSpeculativePhysicalRoundStageV1::Submit,
                disposition_with_logical(closure, logical),
            ));
        }
    };
    complete_round_core::<A, _, C>(engine, published, logical)
}

trait M1SpeculativeRoundObservationV1: fmt::Debug {
    fn preflight(
        &self,
        coordinator: &M1SpeculativeGenerationLoopV1,
        binding: crate::M1SpeculativeRoundBindingV1,
        controls: &[M1SpeculativeMemberControlV1],
    ) -> Result<crate::M1SpeculativePreflightedRoundV1, crate::M1SpeculativeGenerationLoopErrorV1>;
}

impl M1SpeculativeRoundObservationV1
    for crate::M1AuthenticatedRearmedSpeculativeDiagnosticCompletedReadbackV1
{
    fn preflight(
        &self,
        coordinator: &M1SpeculativeGenerationLoopV1,
        binding: crate::M1SpeculativeRoundBindingV1,
        controls: &[M1SpeculativeMemberControlV1],
    ) -> Result<crate::M1SpeculativePreflightedRoundV1, crate::M1SpeculativeGenerationLoopErrorV1>
    {
        coordinator.preflight_checked_round(binding, self.checked(), controls)
    }
}

impl M1SpeculativeRoundObservationV1 for M1AuthenticatedSpeculativeDiagnosticCompletedReadbackV1 {
    fn preflight(
        &self,
        coordinator: &M1SpeculativeGenerationLoopV1,
        binding: crate::M1SpeculativeRoundBindingV1,
        controls: &[M1SpeculativeMemberControlV1],
    ) -> Result<crate::M1SpeculativePreflightedRoundV1, crate::M1SpeculativeGenerationLoopErrorV1>
    {
        coordinator.preflight_checked_round(binding, self.completed().checked(), controls)
    }
}

#[derive(Debug)]
struct M1PreparedCoordinatorRoundCoreV1<D> {
    coordinator: M1SpeculativeGenerationLoopV1,
    diagnostic: D,
    controls: Vec<M1SpeculativeMemberControlV1>,
    preflighted: crate::M1SpeculativePreflightedRoundV1,
    dispositions: Vec<crate::M1DeviceKvCompletionDispositionV1>,
    lineage: M1AuthenticatedSpeculativeCausalLineageV1,
}

#[derive(Debug)]
enum M1PrepareCoordinatorRoundCoreFailureV1<D> {
    Preflight {
        coordinator: M1SpeculativeGenerationLoopV1,
        diagnostic: D,
        controls: Vec<M1SpeculativeMemberControlV1>,
        error: crate::M1SpeculativeGenerationLoopErrorV1,
        lineage: M1AuthenticatedSpeculativeCausalLineageV1,
    },
    CommitPreflight {
        coordinator: M1SpeculativeGenerationLoopV1,
        diagnostic: D,
        controls: Vec<M1SpeculativeMemberControlV1>,
        preflighted: crate::M1SpeculativePreflightedRoundV1,
        error: crate::M1SpeculativeGenerationLoopErrorV1,
        lineage: M1AuthenticatedSpeculativeCausalLineageV1,
    },
    HostAllocation {
        coordinator: M1SpeculativeGenerationLoopV1,
        diagnostic: D,
        controls: Vec<M1SpeculativeMemberControlV1>,
        preflighted: crate::M1SpeculativePreflightedRoundV1,
        lineage: M1AuthenticatedSpeculativeCausalLineageV1,
    },
}

fn prepare_coordinator_round_core<D, const C: usize>(
    engine: &mut Engine<C>,
    coordinator: M1SpeculativeGenerationLoopV1,
    binding: crate::M1SpeculativeRoundBindingV1,
    diagnostic: D,
    controls: Vec<M1SpeculativeMemberControlV1>,
    lineage: M1AuthenticatedSpeculativeCausalLineageV1,
) -> Result<M1PreparedCoordinatorRoundCoreV1<D>, M1PrepareCoordinatorRoundCoreFailureV1<D>>
where
    D: M1SpeculativeRoundObservationV1,
{
    let preflighted = match diagnostic.preflight(&coordinator, binding, &controls) {
        Ok(preflighted) => preflighted,
        Err(error) => {
            engine.quarantine_m1_queue_rearm_failure();
            return Err(M1PrepareCoordinatorRoundCoreFailureV1::Preflight {
                coordinator,
                diagnostic,
                controls,
                error,
                lineage,
            });
        }
    };
    if let Err(error) = coordinator.preflight_prepared_round_commit(&preflighted) {
        engine.quarantine_m1_queue_rearm_failure();
        return Err(M1PrepareCoordinatorRoundCoreFailureV1::CommitPreflight {
            coordinator,
            diagnostic,
            controls,
            preflighted,
            error,
            lineage,
        });
    }
    let mut dispositions = Vec::new();
    if dispositions
        .try_reserve_exact(preflighted.members().len())
        .is_err()
    {
        engine.quarantine_m1_queue_rearm_failure();
        return Err(M1PrepareCoordinatorRoundCoreFailureV1::HostAllocation {
            coordinator,
            diagnostic,
            controls,
            preflighted,
            lineage,
        });
    }
    dispositions.extend(
        preflighted
            .members()
            .iter()
            .copied()
            .map(crate::M1SpeculativeMemberRoundOutcomeV1::physical_disposition),
    );
    Ok(M1PreparedCoordinatorRoundCoreV1 {
        coordinator,
        diagnostic,
        controls,
        preflighted,
        dispositions,
        lineage,
    })
}

#[derive(Debug)]
struct M1CommittedCoordinatorRoundCoreV1<P> {
    coordinator: M1SpeculativeGenerationLoopV1,
    outcome: M1SpeculativeRoundOutcomeV1,
    physical: P,
    lineage: M1AuthenticatedSpeculativeCausalLineageV1,
}

#[derive(Debug)]
enum M1CommitCoordinatorRoundCoreFailureV1<P> {
    Coordinator {
        coordinator: M1SpeculativeGenerationLoopV1,
        controls: Vec<M1SpeculativeMemberControlV1>,
        physical: P,
        failure: Box<crate::M1SpeculativePreparedRoundCommitFailureV1>,
        lineage: M1AuthenticatedSpeculativeCausalLineageV1,
    },
    CausalLineage {
        coordinator: M1SpeculativeGenerationLoopV1,
        outcome: M1SpeculativeRoundOutcomeV1,
        physical: P,
        lineage: M1AuthenticatedSpeculativeCausalLineageV1,
    },
}

fn commit_coordinator_round_core<P, const C: usize>(
    engine: &mut Engine<C>,
    mut coordinator: M1SpeculativeGenerationLoopV1,
    preflighted: crate::M1SpeculativePreflightedRoundV1,
    controls: Vec<M1SpeculativeMemberControlV1>,
    physical: P,
    mut lineage: M1AuthenticatedSpeculativeCausalLineageV1,
) -> Result<M1CommittedCoordinatorRoundCoreV1<P>, M1CommitCoordinatorRoundCoreFailureV1<P>> {
    let outcome = match coordinator.commit_preflighted_round(preflighted) {
        Ok(outcome) => outcome,
        Err(failure) => {
            engine.quarantine_m1_queue_rearm_failure();
            return Err(M1CommitCoordinatorRoundCoreFailureV1::Coordinator {
                coordinator,
                controls,
                physical,
                failure,
                lineage,
            });
        }
    };
    if !refresh_causal_generated_counts(&mut lineage, &coordinator, outcome.completed_epoch()) {
        engine.quarantine_m1_queue_rearm_failure();
        return Err(M1CommitCoordinatorRoundCoreFailureV1::CausalLineage {
            coordinator,
            outcome,
            physical,
            lineage,
        });
    }
    Ok(M1CommittedCoordinatorRoundCoreV1 {
        coordinator,
        outcome,
        physical,
        lineage,
    })
}

/// Public rejection or terminal failure for one production speculative round.
///
/// A pure pre-detach rejection yields only an opaque consuming retry owner.
/// Terminal failures can never yield an executor, completed readback,
/// completed-step owner, or released round.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedSpeculativePhysicalRoundFailureV1;
/// fn resubmit(failure: M1AuthenticatedSpeculativePhysicalRoundFailureV1) {
///     let _executor = failure.into_retryable_round();
/// }
/// ```
///
/// ```compile_fail
/// use ferric_engine::{Engine, M1AuthenticatedSpeculativePhysicalRoundFailureV1};
/// fn retry_page_release(
///     failure: M1AuthenticatedSpeculativePhysicalRoundFailureV1,
///     engine: &mut Engine<32>,
/// ) {
///     let _ = failure.retry_page_release(engine);
/// }
/// ```
#[must_use = "pre-detach retry or terminal speculative custody remains retained"]
pub enum M1AuthenticatedSpeculativePhysicalRoundFailureV1 {
    PreDetach {
        stage: M1AuthenticatedSpeculativePhysicalRoundStageV1,
        retry: Box<M1AuthenticatedSpeculativePhysicalRoundPreDetachRetryV1>,
    },
    Terminal {
        stage: M1AuthenticatedSpeculativePhysicalRoundStageV1,
        disposition: M1AuthenticatedSpeculativeFailureDispositionV1,
    },
}

impl M1AuthenticatedSpeculativePhysicalRoundFailureV1 {
    #[must_use]
    pub const fn stage(&self) -> M1AuthenticatedSpeculativePhysicalRoundStageV1 {
        match self {
            Self::PreDetach { stage, .. } | Self::Terminal { stage, .. } => *stage,
        }
    }

    #[must_use = "the terminal disposition must remain observed when present"]
    pub const fn disposition(&self) -> Option<&M1AuthenticatedSpeculativeFailureDispositionV1> {
        match self {
            Self::PreDetach { .. } => None,
            Self::Terminal { disposition, .. } => Some(disposition),
        }
    }

    #[must_use]
    pub const fn is_pre_detach_retry(&self) -> bool {
        matches!(self, Self::PreDetach { .. })
    }

    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(self, Self::Terminal { .. })
    }
}

impl fmt::Debug for M1AuthenticatedSpeculativePhysicalRoundFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedSpeculativePhysicalRoundFailureV1")
            .field("stage", &self.stage())
            .field("pre_detach_retry", &self.is_pre_detach_retry())
            .field("terminal_disposition", &self.disposition())
            .finish()
    }
}

/// Opaque exact executor and inputs retained before queue detachment.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedSpeculativePhysicalRoundPreDetachRetryV1;
/// fn extract(retry: M1AuthenticatedSpeculativePhysicalRoundPreDetachRetryV1) {
///     let _executor_or_inputs = retry.into_parts();
/// }
/// ```
#[must_use = "pre-detach speculative retry custody remains linear"]
pub struct M1AuthenticatedSpeculativePhysicalRoundPreDetachRetryV1 {
    executor: M1AuthenticatedSpeculativePhysicalExecutorV1,
    inputs: M1AuthenticatedSpeculativePhysicalRoundInputsV1,
}

impl fmt::Debug for M1AuthenticatedSpeculativePhysicalRoundPreDetachRetryV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedSpeculativePhysicalRoundPreDetachRetryV1")
            .field("retains_exact_inputs", &true)
            .finish()
    }
}

impl M1AuthenticatedSpeculativePhysicalRoundPreDetachRetryV1 {
    /// Retries the exact unchanged executor and inputs.
    ///
    /// # Errors
    ///
    /// Returns renewed pre-detach custody or a terminal post-detach failure.
    pub fn retry<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1AuthenticatedSpeculativePhysicalRoundSuccessV1,
        Box<M1AuthenticatedSpeculativePhysicalRoundFailureV1>,
    > {
        self.executor.execute_round(engine, self.inputs)
    }

    #[must_use]
    pub const fn retains_exact_inputs(&self) -> bool {
        true
    }

    fn close_without_authority<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> M1AuthenticatedSpeculativeFailureDispositionV1 {
        engine.quarantine_m1_queue_rearm_failure();
        match self.executor.destroy_queue_and_retain_state(engine) {
            Ok(released) => released_disposition((released, self.inputs)),
            Err(quarantined) => quarantined_disposition((quarantined, self.inputs)),
        }
    }
}

#[derive(Debug)]
#[allow(clippy::type_complexity)]
enum M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1 {
    Closed(M1AuthenticatedSpeculativeFailureDispositionV1),
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
    LineageAfterCommit(
        Box<(
            M1SpeculativeGenerationLoopV1,
            M1SpeculativeRoundOutcomeV1,
            M1ObservedSpeculativeDiagnosticChoicesV1,
            crate::M1AuthenticatedRearmedCompletionOutcomeV1,
        )>,
    ),
    ReleasedLineage(
        Box<(
            M1SpeculativeGenerationLoopV1,
            M1SpeculativeRoundOutcomeV1,
            M1ObservedSpeculativeDiagnosticChoicesV1,
            M1AuthenticatedLongLivedQueueReleasedRoundV1,
        )>,
    ),
}

/// Exact stage-specific terminal source retained only until opaque closure.
#[allow(dead_code)]
#[derive(Debug)]
enum M1AuthenticatedSpeculativePhysicalRoundTerminalSourceV1 {
    Schedule(Box<crate::M1AuthenticatedLongLivedQueueRearmScheduleTerminalV1>),
    Recipe(Box<crate::M1PhysicalRunnerRecipeFailureV1>),
    KvReservation(Box<crate::M1AuthenticatedLongLivedQueueRearmKvReservationFailureV1>),
    WorkspacePreparation(Box<crate::M1AuthenticatedLongLivedQueueRearmPrepareFailureV1>),
    Submit(Box<crate::M1AuthenticatedLongLivedQueueRearmSubmissionFailureV1>),
    QueueProgress(Box<crate::M1AuthenticatedRearmedQueueProgressFailureV1>),
    Coordinator(crate::M1SpeculativeGenerationLoopErrorV1),
    CoordinatorCommit(Box<crate::M1SpeculativePreparedRoundCommitFailureV1>),
    PhysicalOutcome(Box<crate::M1AuthenticatedRearmedCompletionOutcomeV1>),
    CausalLineage,
    HostAllocation,
}

/// Failure retaining all available authenticated, KV, logical, and input custody.
#[must_use = "failed production round custody must be retried, torn down, or retained"]
#[derive(Debug)]
struct PendingM1AuthenticatedSpeculativePhysicalRoundFailureV1 {
    stage: M1AuthenticatedSpeculativePhysicalRoundStageV1,
    custody: M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1,
    lineage: Option<M1AuthenticatedSpeculativeCausalLineageV1>,
}

impl PendingM1AuthenticatedSpeculativePhysicalRoundFailureV1 {
    /// Destroys a recoverable queue or consumes a concrete lower terminal owner
    /// into explicit quarantine. No effectful-stage failure is type-erased.
    ///
    /// # Errors
    ///
    /// The outer error returns unchanged pure-rejection custody that has no
    /// queue teardown to perform. The inner error is exact release quarantine.
    fn destroy_queue_and_retain_custody<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        Result<
            M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessV1,
            Box<M1AuthenticatedSpeculativePhysicalRoundTeardownFailureV1>,
        >,
        Self,
    > {
        let Self {
            stage,
            custody,
            lineage,
        } = self;
        match custody {
            M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::CoordinatorPreflight(
                retained,
            ) => {
                let (coordinator, diagnostic, controls, error) = *retained;
                let (readback, choices) = diagnostic.into_parts();
                let logical = retain_logical_lineage(
                    lineage,
                    (coordinator, controls, error, choices),
                );
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
                let logical = retain_logical_lineage(
                    lineage,
                    (coordinator, controls, permit, error, choices),
                );
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
                let logical = retain_logical_lineage(
                    lineage,
                    (coordinator, controls, permit, choices),
                );
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
                let logical = retain_logical_lineage(
                    lineage,
                    (coordinator, permit, controls, choices),
                );
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
                let logical = retain_logical_lineage(lineage, (coordinator, outcome, choices));
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
                let logical = retain_logical_lineage(
                    lineage,
                    (coordinator, permit, controls, choices),
                );
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
                        Err(Box::new(M1AuthenticatedSpeculativePhysicalRoundTeardownFailureV1 {
                            stage,
                            source:
                                M1AuthenticatedSpeculativePhysicalRoundTeardownFailureSourceV1::TerminalQuarantine(
                                    M1AuthenticatedSpeculativePhysicalRoundTerminalSourceV1::PhysicalOutcome(
                                        physical,
                                    ),
                                ),
                            logical,
                        }))
                    }
                })
            }
            M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::CoordinatorCommit(retained) => {
                let (coordinator, controls, choices, physical, failure) = *retained;
                engine.quarantine_m1_queue_rearm_failure();
                let logical = retain_logical_lineage(lineage, (coordinator, controls, choices));
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
                        engine.quarantine_m1_queue_rearm_failure();
                        Err(Box::new(M1AuthenticatedSpeculativePhysicalRoundTeardownFailureV1 {
                            stage,
                            source:
                                M1AuthenticatedSpeculativePhysicalRoundTeardownFailureSourceV1::TerminalQuarantine(
                                    M1AuthenticatedSpeculativePhysicalRoundTerminalSourceV1::PhysicalOutcome(
                                        Box::new(physical),
                                    ),
                                ),
                            logical: Box::new((logical, failure)),
                        }))
                    }
                })
            }
            M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::LineageAfterCommit(
                retained,
            ) => {
                let (coordinator, outcome, choices, physical) = *retained;
                engine.quarantine_m1_queue_rearm_failure();
                let logical = retain_logical_lineage(lineage, (coordinator, outcome, choices));
                Ok(match physical.release_completed() {
                    M1AuthenticatedRearmedRoundReleaseOutcomeV1::Released(released) => {
                        match released.destroy_queue_and_retain_round(engine) {
                            Ok(source) => Ok(M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessV1 {
                                stage,
                                source:
                                    M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessSourceV1::Released(
                                        source,
                                    ),
                                logical,
                            }),
                            Err(source) => Err(Box::new(
                                M1AuthenticatedSpeculativePhysicalRoundTeardownFailureV1 {
                                    stage,
                                    source:
                                        M1AuthenticatedSpeculativePhysicalRoundTeardownFailureSourceV1::Released(
                                            source,
                                        ),
                                    logical,
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
                        }
                    }
                    M1AuthenticatedRearmedRoundReleaseOutcomeV1::NotCompleted(physical) => {
                        engine.quarantine_m1_queue_rearm_failure();
                        Err(Box::new(M1AuthenticatedSpeculativePhysicalRoundTeardownFailureV1 {
                            stage,
                            source:
                                M1AuthenticatedSpeculativePhysicalRoundTeardownFailureSourceV1::TerminalQuarantine(
                                    M1AuthenticatedSpeculativePhysicalRoundTerminalSourceV1::PhysicalOutcome(
                                        Box::new(physical),
                                    ),
                                ),
                            logical,
                        }))
                    }
                })
            }
            M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::ReleasedLineage(retained) => {
                let (coordinator, outcome, choices, released) = *retained;
                engine.quarantine_m1_queue_rearm_failure();
                let logical = retain_logical_lineage(lineage, (coordinator, outcome, choices));
                Ok(match released.destroy_queue_and_retain_round(engine) {
                    Ok(source) => Ok(M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessV1 {
                        stage,
                        source:
                            M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessSourceV1::Released(
                                source,
                            ),
                        logical,
                    }),
                    Err(source) => Err(Box::new(
                        M1AuthenticatedSpeculativePhysicalRoundTeardownFailureV1 {
                            stage,
                            source:
                                M1AuthenticatedSpeculativePhysicalRoundTeardownFailureSourceV1::Released(
                                    source,
                                ),
                            logical,
                        },
                    )),
                })
            }
            M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::Schedule(retained) => {
                let (coordinator, binding, kv, recipe, preparation, controls, source) = *retained;
                terminal_quarantine(
                    engine,
                    stage,
                    M1AuthenticatedSpeculativePhysicalRoundTerminalSourceV1::Schedule(source),
                    (lineage, coordinator, binding, kv, recipe, preparation, controls),
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
                    (lineage, coordinator, binding, scheduled, kv, plans, controls),
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
                    (lineage, coordinator, binding, recipe, plans, controls),
                )
            }
            M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::WorkspacePreparation(retained) => {
                let (coordinator, binding, recipe, controls, source) = *retained;
                terminal_quarantine(
                    engine,
                    stage,
                    M1AuthenticatedSpeculativePhysicalRoundTerminalSourceV1::WorkspacePreparation(source),
                    (lineage, coordinator, binding, recipe, controls),
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
                    (lineage, coordinator, outcome, choices),
                )
            }
            M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::Retryable(retained) => {
                Err(Self {
                    stage,
                    custody: M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::Retryable(
                        retained,
                    ),
                    lineage,
                })
            }
            M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::Complete(retained) => {
                Err(Self {
                    stage,
                    custody: M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::Complete(
                        retained,
                    ),
                    lineage,
                })
            }
            M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::Closed(disposition) => {
                Err(Self {
                    stage,
                    custody: M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::Closed(
                        disposition,
                    ),
                    lineage,
                })
            }
        }
    }
}

pub(crate) fn released_disposition(
    retained: impl fmt::Debug + 'static,
) -> M1AuthenticatedSpeculativeFailureDispositionV1 {
    M1AuthenticatedSpeculativeFailureDispositionV1::Released(
        M1AuthenticatedSpeculativeCleanReleaseV1 {
            retained: Box::new(retained),
        },
    )
}

pub(crate) fn quarantined_disposition(
    retained: impl fmt::Debug + 'static,
) -> M1AuthenticatedSpeculativeFailureDispositionV1 {
    M1AuthenticatedSpeculativeFailureDispositionV1::Quarantined(
        M1AuthenticatedSpeculativeTerminalQuarantineV1 {
            retained: Box::new(retained),
        },
    )
}

#[allow(clippy::unnecessary_box_returns)]
fn released_failure(
    stage: M1AuthenticatedSpeculativePhysicalRoundStageV1,
    retained: impl fmt::Debug + 'static,
) -> Box<M1AuthenticatedSpeculativePhysicalRoundFailureV1> {
    Box::new(M1AuthenticatedSpeculativePhysicalRoundFailureV1::Terminal {
        stage,
        disposition: released_disposition(retained),
    })
}

#[allow(clippy::unnecessary_box_returns)]
fn quarantined_failure(
    stage: M1AuthenticatedSpeculativePhysicalRoundStageV1,
    retained: impl fmt::Debug + 'static,
) -> Box<M1AuthenticatedSpeculativePhysicalRoundFailureV1> {
    Box::new(M1AuthenticatedSpeculativePhysicalRoundFailureV1::Terminal {
        stage,
        disposition: quarantined_disposition(retained),
    })
}

/// Consumes internal stage custody into a non-retryable public outcome. This
/// is the only high-level failure exit after queue detachment.
#[allow(clippy::boxed_local, clippy::unnecessary_box_returns)]
fn close_pending_round_failure<const C: usize>(
    engine: &mut Engine<C>,
    pending: Box<PendingM1AuthenticatedSpeculativePhysicalRoundFailureV1>,
) -> Box<M1AuthenticatedSpeculativePhysicalRoundFailureV1> {
    let PendingM1AuthenticatedSpeculativePhysicalRoundFailureV1 {
        stage,
        custody,
        lineage,
    } = *pending;
    let custody = match custody {
        M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::Closed(disposition) => {
            engine.quarantine_m1_queue_rearm_failure();
            return Box::new(M1AuthenticatedSpeculativePhysicalRoundFailureV1::Terminal {
                stage,
                disposition,
            });
        }
        custody => custody,
    };
    if let M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::Retryable(retained) = custody {
        let (executor, inputs) = *retained;
        if !engine.is_faulted() {
            return Box::new(
                M1AuthenticatedSpeculativePhysicalRoundFailureV1::PreDetach {
                    stage,
                    retry: Box::new(M1AuthenticatedSpeculativePhysicalRoundPreDetachRetryV1 {
                        executor,
                        inputs,
                    }),
                },
            );
        }
        engine.quarantine_m1_queue_rearm_failure();
        return match executor.destroy_queue_and_retain_state(engine) {
            Ok(released) => released_failure(stage, (released, inputs, lineage)),
            Err(quarantined) => quarantined_failure(stage, (quarantined, inputs, lineage)),
        };
    }
    let pending = Box::new(PendingM1AuthenticatedSpeculativePhysicalRoundFailureV1 {
        stage,
        custody,
        lineage,
    });
    engine.quarantine_m1_queue_rearm_failure();
    match pending.destroy_queue_and_retain_custody(engine) {
        Ok(Ok(released)) => released_failure(stage, released),
        Ok(Err(quarantined)) => {
            engine.quarantine_m1_queue_rearm_failure();
            quarantined_failure(stage, quarantined)
        }
        Err(pending) => {
            let PendingM1AuthenticatedSpeculativePhysicalRoundFailureV1 {
                custody, lineage, ..
            } = pending;
            let (executor, inputs) = match custody {
                M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::Complete(retained)
                | M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::Retryable(retained) => {
                    *retained
                }
                custody => {
                    engine.quarantine_m1_queue_rearm_failure();
                    return quarantined_failure(stage, (custody, lineage));
                }
            };
            match executor.destroy_queue_and_retain_state(engine) {
                Ok(released) => released_failure(stage, (released, inputs, lineage)),
                Err(quarantined) => {
                    engine.quarantine_m1_queue_rearm_failure();
                    quarantined_failure(stage, (quarantined, inputs, lineage))
                }
            }
        }
    }
}

#[allow(clippy::boxed_local, clippy::unnecessary_box_returns)]
fn close_pending_bootstrap_failure<const C: usize>(
    engine: &mut Engine<C>,
    pending: Box<PendingM1AuthenticatedSpeculativeBootstrapRoundFailureV1>,
) -> Box<M1AuthenticatedSpeculativeBootstrapRoundFailureV1> {
    engine.quarantine_m1_queue_rearm_failure();
    let PendingM1AuthenticatedSpeculativeBootstrapRoundFailureV1 { stage, custody } = *pending;
    let disposition = match custody {
        PendingM1AuthenticatedSpeculativeBootstrapRoundFailureCustodyV1::Retryable(retained) => {
            let (continuation, diagnostic, controls) = *retained;
            match diagnostic.destroy_queue_and_retain_evidence(engine) {
                Ok(released) => released_disposition((released, continuation, controls)),
                Err(quarantined) => quarantined_disposition((quarantined, continuation, controls)),
            }
        }
        PendingM1AuthenticatedSpeculativeBootstrapRoundFailureCustodyV1::PhysicalOutcome(
            retained,
        ) => {
            let (coordinator, lineage, controls, choices, preflighted, physical) = *retained;
            let logical = (coordinator, lineage, controls, choices, preflighted);
            match crate::m1_completed_step::close_m1_authenticated_completed_step_outcome_v1(
                engine, physical,
            ) {
                crate::m1_completed_step::M1AuthenticatedCompletedStepClosureV1::Released(
                    released,
                ) => released_disposition((released, logical)),
                crate::m1_completed_step::M1AuthenticatedCompletedStepClosureV1::Quarantined(
                    quarantined,
                ) => quarantined_disposition((quarantined, logical)),
            }
        }
        PendingM1AuthenticatedSpeculativeBootstrapRoundFailureCustodyV1::CoordinatorCommit(
            retained,
        ) => {
            let (coordinator, lineage, controls, choices, completed, failure) = *retained;
            match completed.destroy_queue_and_retain_completion(engine) {
                Ok(released) => released_disposition((
                    released,
                    coordinator,
                    lineage,
                    controls,
                    choices,
                    failure,
                )),
                Err(quarantined) => quarantined_disposition((
                    quarantined,
                    coordinator,
                    lineage,
                    controls,
                    choices,
                    failure,
                )),
            }
        }
        PendingM1AuthenticatedSpeculativeBootstrapRoundFailureCustodyV1::LineageAfterCommit(
            retained,
        ) => {
            let (coordinator, lineage, controls, choices, completed, outcome) = *retained;
            match completed.destroy_queue_and_retain_completion(engine) {
                Ok(released) => released_disposition((
                    released,
                    coordinator,
                    lineage,
                    controls,
                    choices,
                    outcome,
                )),
                Err(quarantined) => quarantined_disposition((
                    quarantined,
                    coordinator,
                    lineage,
                    controls,
                    choices,
                    outcome,
                )),
            }
        }
        PendingM1AuthenticatedSpeculativeBootstrapRoundFailureCustodyV1::PageRelease(retained) => {
            let (coordinator, lineage, outcome, choices, failure) = *retained;
            let (error, completed) = failure.into_parts();
            match completed.destroy_queue_and_retain_completion(engine) {
                Ok(released) => {
                    released_disposition((released, coordinator, lineage, outcome, choices, error))
                }
                Err(quarantined) => quarantined_disposition((
                    quarantined,
                    coordinator,
                    lineage,
                    outcome,
                    choices,
                    error,
                )),
            }
        }
        PendingM1AuthenticatedSpeculativeBootstrapRoundFailureCustodyV1::ReleasedLineage(
            retained,
        ) => {
            let (coordinator, lineage, outcome, choices, released) = *retained;
            match released.destroy_queue_and_retain_round(engine) {
                Ok(release) => {
                    released_disposition((release, coordinator, lineage, outcome, choices))
                }
                Err(quarantined) => {
                    quarantined_disposition((quarantined, coordinator, lineage, outcome, choices))
                }
            }
        }
    };
    Box::new(M1AuthenticatedSpeculativeBootstrapRoundFailureV1::Terminal { stage, disposition })
}

fn close_pending_rollover_failure<const C: usize>(
    engine: &mut Engine<C>,
    pending: PendingM1AuthenticatedSpeculativeRolloverRoundFailureV1,
) -> M1AuthenticatedSpeculativeRolloverRoundFailureV1 {
    engine.quarantine_m1_queue_rearm_failure();
    match pending {
        PendingM1AuthenticatedSpeculativeRolloverRoundFailureV1::Unsettled { stage, retained } => {
            let (continuation, diagnostic, controls) = *retained;
            let (readback, choices) = diagnostic.into_parts();
            let disposition = match readback.destroy_queue_and_retain_custody(engine) {
                Ok(released) => released_disposition((released, choices, continuation, controls)),
                Err(quarantined) => {
                    quarantined_disposition((quarantined, choices, continuation, controls))
                }
            };
            M1AuthenticatedSpeculativeRolloverRoundFailureV1 { stage, disposition }
        }
        PendingM1AuthenticatedSpeculativeRolloverRoundFailureV1::Round(pending) => {
            let failure = close_pending_round_failure(engine, pending);
            match *failure {
                M1AuthenticatedSpeculativePhysicalRoundFailureV1::Terminal {
                    stage,
                    disposition,
                } => M1AuthenticatedSpeculativeRolloverRoundFailureV1 { stage, disposition },
                M1AuthenticatedSpeculativePhysicalRoundFailureV1::PreDetach { stage, retry } => {
                    M1AuthenticatedSpeculativeRolloverRoundFailureV1 {
                        stage,
                        disposition: retry.close_without_authority(engine),
                    }
                }
            }
        }
    }
}

/// Exact lower success retained only until opaque closure.
#[allow(dead_code)]
#[derive(Debug)]
enum M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessSourceV1 {
    Diagnostic(crate::M1AuthenticatedRearmedSpeculativeDiagnosticReadbackTeardownSuccessV1),
    Joined(crate::M1AuthenticatedRearmedCompletedReadbackTeardownSuccessV1),
    CompletionPreflight(crate::M1AuthenticatedRearmedCompletionPreflightTeardownSuccessV1),
    RejectedCompletion(crate::M1AuthenticatedRearmedRejectedCompletionTeardownSuccessV1),
    PageRelease(crate::M1AuthenticatedRearmedRoundPageReleaseTeardownSuccessV1),
    Released(crate::M1AuthenticatedLongLivedQueueRearmTeardownSuccessV1),
}

/// Exact lower quarantine retained only until opaque closure.
#[allow(dead_code)]
#[derive(Debug)]
enum M1AuthenticatedSpeculativePhysicalRoundTeardownFailureSourceV1 {
    Diagnostic(Box<crate::M1AuthenticatedRearmedSpeculativeDiagnosticReadbackTeardownFailureV1>),
    Joined(Box<crate::M1AuthenticatedRearmedCompletedReadbackTeardownFailureV1>),
    CompletionPreflight(Box<crate::M1AuthenticatedRearmedCompletionPreflightTeardownFailureV1>),
    RejectedCompletion(Box<crate::M1AuthenticatedRearmedRejectedCompletionTeardownFailureV1>),
    PageRelease(Box<crate::M1AuthenticatedRearmedRoundPageReleaseTeardownFailureV1>),
    Released(Box<crate::M1AuthenticatedLongLivedQueueRearmTeardownFailureV1>),
    TerminalQuarantine(M1AuthenticatedSpeculativePhysicalRoundTerminalSourceV1),
}

/// Completed queue teardown or explicit process-level terminal quarantine.
#[must_use = "teardown disposition retains lower and logical custody"]
#[allow(dead_code)]
#[derive(Debug)]
struct M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessV1 {
    stage: M1AuthenticatedSpeculativePhysicalRoundStageV1,
    source: M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessSourceV1,
    logical: Box<dyn fmt::Debug>,
}

/// Queue release quarantine with all logical failure context retained.
#[must_use = "teardown failure retains lower and logical custody"]
#[allow(dead_code)]
#[derive(Debug)]
struct M1AuthenticatedSpeculativePhysicalRoundTeardownFailureV1 {
    stage: M1AuthenticatedSpeculativePhysicalRoundStageV1,
    source: M1AuthenticatedSpeculativePhysicalRoundTeardownFailureSourceV1,
    logical: Box<dyn fmt::Debug>,
}

#[allow(dead_code)]
impl M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessV1 {
    #[must_use]
    const fn stage(&self) -> M1AuthenticatedSpeculativePhysicalRoundStageV1 {
        self.stage
    }

    #[must_use]
    const fn source(&self) -> &M1AuthenticatedSpeculativePhysicalRoundTeardownSuccessSourceV1 {
        &self.source
    }

    #[must_use]
    fn retains_logical_custody(&self) -> bool {
        let _ = &self.logical;
        true
    }
}

#[allow(dead_code)]
impl M1AuthenticatedSpeculativePhysicalRoundTeardownFailureV1 {
    #[must_use]
    const fn stage(&self) -> M1AuthenticatedSpeculativePhysicalRoundStageV1 {
        self.stage
    }

    #[must_use]
    const fn source(&self) -> &M1AuthenticatedSpeculativePhysicalRoundTeardownFailureSourceV1 {
        &self.source
    }

    #[must_use]
    fn retains_logical_custody(&self) -> bool {
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

fn speculative_draft_selection(target: Qwen3PlanSelection) -> Option<Qwen3PlanSelection> {
    let bucket = match target.bucket {
        Qwen3PlanBucket::SpeculativeS1K4C8192
        | Qwen3PlanBucket::SpeculativeS1K8C8192
        | Qwen3PlanBucket::SpeculativeS1K16C8192 => Qwen3PlanBucket::DecodeS1C8192,
        Qwen3PlanBucket::SpeculativeS8K4C8192 => Qwen3PlanBucket::DecodeS8C8192,
        _ => return None,
    };
    (target.role == Qwen3ModelRole::Target8B && target.mode == Qwen3ExecutionMode::Speculative)
        .then_some(Qwen3PlanSelection {
            role: Qwen3ModelRole::Draft06B,
            mode: Qwen3ExecutionMode::Decode,
            bucket,
        })
}

fn speculative_role_inputs_match_binding(
    input: &ValidatedM1StepInputs,
    expected_selection: Qwen3PlanSelection,
    binding: &crate::M1SpeculativeRoundBindingV1,
    committed: impl Fn(crate::M1SpeculativeRoundMemberInputV1) -> u32,
) -> bool {
    let members = binding.members();
    let Ok(live_lanes) = usize::try_from(input.live_lane_count()) else {
        return false;
    };
    let width = input.dimensions().active_tokens as usize;
    input.selection() == expected_selection
        && live_lanes == members.len()
        && members.iter().copied().enumerate().all(|(lane, member)| {
            let Some(plan) = input.lanes().get(lane).and_then(Option::as_ref) else {
                return false;
            };
            let Some(row_start) = lane.checked_mul(width) else {
                return false;
            };
            plan.selection() == expected_selection
                && plan.request() == member.request()
                && plan.completion_epoch() == binding.epoch()
                && input.token_ids().get(row_start) == Some(&member.round_anchor())
                && input.position_ids().get(row_start) == Some(&committed(member))
                && input.context_lengths().get(lane) == Some(&committed(member))
                && input.active_lengths().get(lane) == Some(&input.dimensions().active_tokens)
        })
}

fn speculative_round_inputs_match_binding(
    inputs: &M1LongLivedQueueRearmKvInputsV1,
    binding: &crate::M1SpeculativeRoundBindingV1,
) -> bool {
    let M1LongLivedQueueRearmKvInputsV1::SpeculativeRound {
        draft_decode,
        target_speculative,
        ..
    } = inputs
    else {
        return false;
    };
    speculative_validated_pair_matches_binding(draft_decode, target_speculative, binding)
}

pub(crate) fn speculative_validated_pair_matches_binding(
    draft_decode: &ValidatedM1StepInputs,
    target_speculative: &ValidatedM1StepInputs,
    binding: &crate::M1SpeculativeRoundBindingV1,
) -> bool {
    let target_selection = binding.shape().selection();
    let Some(draft_selection) = speculative_draft_selection(target_selection) else {
        return false;
    };
    let target_width = target_speculative.dimensions().active_tokens as usize;
    let expected_target_width = usize::from(binding.shape().draft_tokens()) + 1;
    speculative_role_inputs_match_binding(
        draft_decode,
        draft_selection,
        binding,
        crate::M1SpeculativeRoundMemberInputV1::draft_pre_committed,
    ) && draft_decode.dimensions().active_tokens == 1
        && speculative_role_inputs_match_binding(
            target_speculative,
            target_selection,
            binding,
            crate::M1SpeculativeRoundMemberInputV1::target_pre_committed,
        )
        && target_width == expected_target_width
        && binding.members().iter().enumerate().all(|(lane, _)| {
            let Some(row_start) = lane.checked_mul(target_width) else {
                return false;
            };
            let Some(future_start) = row_start.checked_add(1) else {
                return false;
            };
            let Some(row_end) = row_start.checked_add(target_width) else {
                return false;
            };
            target_speculative
                .token_ids()
                .get(future_start..row_end)
                .is_some_and(|future| future.iter().all(|token| *token == 0))
        })
}

pub(crate) fn upgrade_m1_authenticated_speculative_lineage_v1(
    coordinator: &M1SpeculativeGenerationLoopV1,
    binding: &crate::M1SpeculativeRoundBindingV1,
    identity: M1AuthenticatedSpeculativeLineageIdentityV1,
) -> Result<
    (
        M1AuthenticatedSpeculativePhysicalLineageWitnessV1,
        M1AuthenticatedSpeculativeLogicalLineageWitnessV1,
    ),
    M1AuthenticatedSpeculativeBootstrapErrorV1,
> {
    let initial_seeds = coordinator
        .bootstrap_seed_snapshot()
        .map_err(|_| M1AuthenticatedSpeculativeBootstrapErrorV1::Coordinator)?;
    Ok((
        M1AuthenticatedSpeculativePhysicalLineageWitnessV1 {
            identity,
            coordinator_identity: coordinator.identity(),
            selection: coordinator.shape().selection(),
            round: binding.round(),
            epoch: binding.epoch(),
            initial_seeds,
        },
        M1AuthenticatedSpeculativeLogicalLineageWitnessV1 { identity },
    ))
}

#[allow(clippy::unnecessary_box_returns)]
fn bootstrap_unprepared_failure(
    error: M1AuthenticatedSpeculativeBootstrapErrorV1,
    coordinator: M1SpeculativeGenerationLoopV1,
    selected: Vec<ActiveDeviceKvCache>,
    scheduled: M1ScheduledDispatchV1,
    plans: M1FullStepWorkspacePlans,
    tables: M1FullStepKvWorkspaceTablesV1,
) -> Box<M1AuthenticatedSpeculativeBootstrapFailureV1> {
    Box::new(M1AuthenticatedSpeculativeBootstrapFailureV1 {
        error,
        custody: M1AuthenticatedSpeculativeBootstrapFailureCustodyV1::Unprepared(Box::new((
            coordinator,
            selected,
            scheduled,
            plans,
            tables,
        ))),
    })
}

/// Binds a fresh coordinator to the exact authenticated round-zero physical
/// inputs before any queue publication can consume them.
///
/// The returned prepared step contains one private move-only lineage half.
/// Its companion continuation cannot become an executor until that same half
/// returns through authenticated completion and page release.
///
/// # Errors
///
/// Every rejection retains all coordinator, cache, scheduler, plan, and KV
/// custody at the exact stage reached.
pub fn prepare_m1_authenticated_speculative_bootstrap_v1(
    coordinator: M1SpeculativeGenerationLoopV1,
    selected: Vec<ActiveDeviceKvCache>,
    scheduled: M1ScheduledDispatchV1,
    runner: &LogicalRunnerDeclaration,
    plans: M1FullStepWorkspacePlans,
    tables: M1FullStepKvWorkspaceTablesV1,
) -> Result<
    M1AuthenticatedSpeculativeBootstrapPreparedV1,
    Box<M1AuthenticatedSpeculativeBootstrapFailureV1>,
> {
    let mut roster = Vec::new();
    if roster.try_reserve_exact(scheduled.member_count()).is_err() {
        return Err(bootstrap_unprepared_failure(
            M1AuthenticatedSpeculativeBootstrapErrorV1::Coordinator,
            coordinator,
            selected,
            scheduled,
            plans,
            tables,
        ));
    }
    for lane in 0..scheduled.member_count() {
        let Some(request) = scheduled.member(lane) else {
            return Err(bootstrap_unprepared_failure(
                M1AuthenticatedSpeculativeBootstrapErrorV1::Inputs,
                coordinator,
                selected,
                scheduled,
                plans,
                tables,
            ));
        };
        roster.push(request);
    }
    let binding = match coordinator.bind_round(0, scheduled.epoch(), &roster) {
        Ok(binding) => binding,
        Err(_) => {
            return Err(bootstrap_unprepared_failure(
                M1AuthenticatedSpeculativeBootstrapErrorV1::Coordinator,
                coordinator,
                selected,
                scheduled,
                plans,
                tables,
            ));
        }
    };
    let inputs_match = match &tables {
        M1FullStepKvWorkspaceTablesV1::SpeculativeRound {
            draft_decode,
            target_speculative,
        } => speculative_validated_pair_matches_binding(
            draft_decode.inputs(),
            target_speculative.inputs(),
            &binding,
        ),
        _ => false,
    };
    if !inputs_match {
        return Err(bootstrap_unprepared_failure(
            M1AuthenticatedSpeculativeBootstrapErrorV1::Inputs,
            coordinator,
            selected,
            scheduled,
            plans,
            tables,
        ));
    }
    let caches_match = selected.len() == binding.members().len()
        && selected
            .iter()
            .zip(binding.members())
            .all(|(cache, member)| {
                let projection = cache.projection();
                projection.request == member.request()
                    && projection.target.committed_tokens == member.target_pre_committed()
                    && projection.draft.committed_tokens == member.draft_pre_committed()
            });
    if !caches_match {
        return Err(bootstrap_unprepared_failure(
            M1AuthenticatedSpeculativeBootstrapErrorV1::CacheRoster,
            coordinator,
            selected,
            scheduled,
            plans,
            tables,
        ));
    }
    let initial_seeds = match coordinator.bootstrap_seed_snapshot() {
        Ok(seeds) => seeds,
        Err(_) => {
            return Err(bootstrap_unprepared_failure(
                M1AuthenticatedSpeculativeBootstrapErrorV1::Coordinator,
                coordinator,
                selected,
                scheduled,
                plans,
                tables,
            ));
        }
    };
    let Some(lineage_identity) = M1AuthenticatedSpeculativeLineageIdentityV1::fresh() else {
        return Err(bootstrap_unprepared_failure(
            M1AuthenticatedSpeculativeBootstrapErrorV1::LineageIdentityExhausted,
            coordinator,
            selected,
            scheduled,
            plans,
            tables,
        ));
    };
    let physical = M1AuthenticatedSpeculativePhysicalLineageWitnessV1 {
        identity: lineage_identity,
        coordinator_identity: coordinator.identity(),
        selection: coordinator.shape().selection(),
        round: binding.round(),
        epoch: binding.epoch(),
        initial_seeds,
    };
    let prepared =
        match crate::prepare_m1_scheduled_workspace_images_v1(scheduled, runner, plans, tables) {
            Ok(prepared) => prepared,
            Err(source) => {
                return Err(Box::new(M1AuthenticatedSpeculativeBootstrapFailureV1 {
                    error: M1AuthenticatedSpeculativeBootstrapErrorV1::Preparation,
                    custody: M1AuthenticatedSpeculativeBootstrapFailureCustodyV1::Preparation(
                        Box::new((coordinator, binding, selected, source)),
                    ),
                }));
            }
        };
    let prepared = match prepared.retain_speculative_lineage(physical) {
        Ok(prepared) => prepared,
        Err(prepared) => {
            return Err(Box::new(M1AuthenticatedSpeculativeBootstrapFailureV1 {
                error: M1AuthenticatedSpeculativeBootstrapErrorV1::LineageAttachment,
                custody: M1AuthenticatedSpeculativeBootstrapFailureCustodyV1::Attachment(Box::new(
                    (coordinator, binding, selected, prepared),
                )),
            }));
        }
    };
    Ok(M1AuthenticatedSpeculativeBootstrapPreparedV1 {
        prepared,
        continuation: M1AuthenticatedSpeculativeBootstrapContinuationV1 {
            coordinator,
            epoch: binding.epoch(),
            selected,
            lineage: M1AuthenticatedSpeculativeLogicalLineageWitnessV1 {
                identity: lineage_identity,
            },
        },
    })
}

#[allow(clippy::unnecessary_box_returns)]
fn bootstrap_pre_detach_failure(
    stage: M1AuthenticatedSpeculativeBootstrapRoundStageV1,
    retry_state: M1AuthenticatedSpeculativeBootstrapPreDetachRetryStateV1,
) -> Box<M1AuthenticatedSpeculativeBootstrapRoundFailureV1> {
    Box::new(
        M1AuthenticatedSpeculativeBootstrapRoundFailureV1::PreDetach {
            stage,
            retry: Box::new(M1AuthenticatedSpeculativeBootstrapPreDetachRetryV1 {
                state: retry_state,
            }),
        },
    )
}

#[allow(clippy::unnecessary_box_returns)]
fn bootstrap_terminal_failure<const C: usize>(
    engine: &mut Engine<C>,
    stage: M1AuthenticatedSpeculativeBootstrapRoundStageV1,
    retained: impl fmt::Debug + 'static,
) -> Box<M1AuthenticatedSpeculativeBootstrapRoundFailureV1> {
    engine.quarantine_m1_queue_rearm_failure();
    Box::new(
        M1AuthenticatedSpeculativeBootstrapRoundFailureV1::Terminal {
            stage,
            disposition: quarantined_disposition(retained),
        },
    )
}

#[allow(clippy::unnecessary_box_returns)]
fn bootstrap_terminal_disposition(
    stage: M1AuthenticatedSpeculativeBootstrapRoundStageV1,
    disposition: M1AuthenticatedSpeculativeFailureDispositionV1,
) -> Box<M1AuthenticatedSpeculativeBootstrapRoundFailureV1> {
    Box::new(M1AuthenticatedSpeculativeBootstrapRoundFailureV1::Terminal { stage, disposition })
}

impl M1AuthenticatedSpeculativeBootstrapPreparedV1 {
    /// Runs the complete authenticated round-zero physical lifecycle while
    /// keeping the coordinator continuation joined to every queue phase.
    ///
    /// Pure allocation, packet-preparation, and queue-creation rejections
    /// return opaque exact retry custody with a healthy Engine. Every other
    /// failure faults `engine`, closes any available queue, and returns only a
    /// clean release or opaque terminal quarantine.
    ///
    /// # Errors
    ///
    /// Returns pre-detach retry custody or terminal failure custody as
    /// described above.
    #[allow(clippy::too_many_arguments)]
    pub fn execute_initial_round<const C: usize>(
        self,
        engine: &mut Engine<C>,
        partitioned_memory: M1PartitionedModelMemoryKvPoolV1,
        runner: M1AuthenticatedPhysicalRunnerV1,
        recipe: AddresslessM1PhysicalBufferRecipeV1,
        ring_bytes: u32,
        controls: Vec<M1SpeculativeMemberControlV1>,
    ) -> Result<
        M1AuthenticatedSpeculativePhysicalRoundSuccessV1,
        Box<M1AuthenticatedSpeculativeBootstrapRoundFailureV1>,
    > {
        execute_bootstrap_from_allocation(
            engine,
            self,
            partitioned_memory,
            runner,
            recipe,
            ring_bytes,
            controls,
        )
    }
}

impl M1AuthenticatedSpeculativeBootstrapPreDetachRetryV1 {
    /// Retries the exact unchanged inputs retained by a pre-detach rejection.
    ///
    /// # Errors
    ///
    /// Returns renewed pre-detach retry custody or a terminal failure if the
    /// retry advances beyond the pure rejection boundary.
    pub fn retry<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1AuthenticatedSpeculativePhysicalRoundSuccessV1,
        Box<M1AuthenticatedSpeculativeBootstrapRoundFailureV1>,
    > {
        match self.state {
            M1AuthenticatedSpeculativeBootstrapPreDetachRetryStateV1::Allocation {
                diagnostic,
                prepared,
                partitioned_memory,
                runner,
                recipe,
                ring_bytes,
                controls,
            } => {
                drop(diagnostic);
                execute_bootstrap_from_allocation(
                    engine,
                    prepared,
                    partitioned_memory,
                    runner,
                    recipe,
                    ring_bytes,
                    controls,
                )
            }
            M1AuthenticatedSpeculativeBootstrapPreDetachRetryStateV1::Prepublication {
                diagnostic,
                continuation,
                runner,
                allocated,
                recipe,
                completion,
                ring_bytes,
                controls,
            } => {
                drop(diagnostic);
                execute_bootstrap_from_prepublication(
                    engine,
                    continuation,
                    runner,
                    allocated,
                    recipe,
                    completion,
                    ring_bytes,
                    controls,
                )
            }
            M1AuthenticatedSpeculativeBootstrapPreDetachRetryStateV1::QueueCreate {
                diagnostic,
                continuation,
                prepublication,
                ring_bytes,
                controls,
            } => {
                drop(diagnostic);
                execute_bootstrap_from_queue_create(
                    engine,
                    continuation,
                    prepublication,
                    ring_bytes,
                    controls,
                )
            }
        }
    }

    /// Confirms this opaque owner retains every exact input needed for retry.
    #[must_use]
    pub const fn retains_exact_inputs(&self) -> bool {
        true
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_bootstrap_from_allocation<const C: usize>(
    engine: &mut Engine<C>,
    bootstrap: M1AuthenticatedSpeculativeBootstrapPreparedV1,
    partitioned_memory: M1PartitionedModelMemoryKvPoolV1,
    runner: M1AuthenticatedPhysicalRunnerV1,
    recipe: AddresslessM1PhysicalBufferRecipeV1,
    ring_bytes: u32,
    controls: Vec<M1SpeculativeMemberControlV1>,
) -> Result<
    M1AuthenticatedSpeculativePhysicalRoundSuccessV1,
    Box<M1AuthenticatedSpeculativeBootstrapRoundFailureV1>,
> {
    if engine.is_faulted() {
        return Err(bootstrap_terminal_failure(
            engine,
            M1AuthenticatedSpeculativeBootstrapRoundStageV1::EngineFaulted,
            (
                bootstrap,
                partitioned_memory,
                runner,
                recipe,
                ring_bytes,
                controls,
            ),
        ));
    }
    let M1AuthenticatedSpeculativeBootstrapPreparedV1 {
        prepared,
        continuation,
    } = bootstrap;
    let mut allocated =
        match crate::allocate_m1_prepublication_workspaces_v1(partitioned_memory, prepared) {
            Ok(allocated) => allocated,
            Err(failure) => match failure.into_preflight_prepared() {
                Ok((diagnostic, partitioned_memory, prepared)) => {
                    return Err(bootstrap_pre_detach_failure(
                        M1AuthenticatedSpeculativeBootstrapRoundStageV1::WorkspaceAllocation,
                        M1AuthenticatedSpeculativeBootstrapPreDetachRetryStateV1::Allocation {
                            diagnostic: Box::new(diagnostic),
                            prepared: M1AuthenticatedSpeculativeBootstrapPreparedV1 {
                                prepared,
                                continuation,
                            },
                            partitioned_memory,
                            runner,
                            recipe,
                            ring_bytes,
                            controls,
                        },
                    ));
                }
                Err(failure) => {
                    return Err(bootstrap_terminal_failure(
                        engine,
                        M1AuthenticatedSpeculativeBootstrapRoundStageV1::WorkspaceAllocation,
                        (failure, continuation, runner, recipe, ring_bytes, controls),
                    ));
                }
            },
        };
    let selection = continuation.coordinator.shape().selection();
    let completion = match allocated.allocate_completion_output(selection) {
        Ok(completion) => completion,
        Err(error) => {
            return Err(bootstrap_terminal_failure(
                engine,
                M1AuthenticatedSpeculativeBootstrapRoundStageV1::CompletionOutput,
                (
                    error,
                    allocated,
                    continuation,
                    runner,
                    recipe,
                    ring_bytes,
                    controls,
                ),
            ));
        }
    };
    let completion = match allocated.enable_speculative_diagnostic_choices_capture(completion) {
        Ok(completion) => completion,
        Err(failure) => {
            return Err(bootstrap_terminal_failure(
                engine,
                M1AuthenticatedSpeculativeBootstrapRoundStageV1::DiagnosticCapture,
                (
                    failure,
                    allocated,
                    continuation,
                    runner,
                    recipe,
                    ring_bytes,
                    controls,
                ),
            ));
        }
    };
    execute_bootstrap_from_prepublication(
        engine,
        continuation,
        runner,
        allocated,
        recipe,
        completion,
        ring_bytes,
        controls,
    )
}

#[allow(clippy::too_many_arguments)]
fn execute_bootstrap_from_prepublication<const C: usize>(
    engine: &mut Engine<C>,
    continuation: M1AuthenticatedSpeculativeBootstrapContinuationV1,
    runner: M1AuthenticatedPhysicalRunnerV1,
    allocated: M1AllocatedScheduledStepV1,
    recipe: AddresslessM1PhysicalBufferRecipeV1,
    completion: crate::BoundM1CompletionOutputV1,
    ring_bytes: u32,
    controls: Vec<M1SpeculativeMemberControlV1>,
) -> Result<
    M1AuthenticatedSpeculativePhysicalRoundSuccessV1,
    Box<M1AuthenticatedSpeculativeBootstrapRoundFailureV1>,
> {
    if engine.is_faulted() {
        return Err(bootstrap_terminal_failure(
            engine,
            M1AuthenticatedSpeculativeBootstrapRoundStageV1::EngineFaulted,
            (
                continuation,
                runner,
                allocated,
                recipe,
                completion,
                ring_bytes,
                controls,
            ),
        ));
    }
    let prepublication = match runner.prepare_first_step(allocated, recipe, completion) {
        Ok(prepublication) => prepublication,
        Err(failure) => {
            let (diagnostic, runner, allocated, recipe, completion) = failure.into_retry_inputs();
            return Err(bootstrap_pre_detach_failure(
                M1AuthenticatedSpeculativeBootstrapRoundStageV1::Prepublication,
                M1AuthenticatedSpeculativeBootstrapPreDetachRetryStateV1::Prepublication {
                    diagnostic: Box::new(diagnostic),
                    continuation,
                    runner,
                    allocated,
                    recipe,
                    completion,
                    ring_bytes,
                    controls,
                },
            ));
        }
    };
    execute_bootstrap_from_queue_create(engine, continuation, prepublication, ring_bytes, controls)
}

fn execute_bootstrap_from_queue_create<const C: usize>(
    engine: &mut Engine<C>,
    continuation: M1AuthenticatedSpeculativeBootstrapContinuationV1,
    prepublication: M1AuthenticatedPrepublicationBatchV1,
    ring_bytes: u32,
    controls: Vec<M1SpeculativeMemberControlV1>,
) -> Result<
    M1AuthenticatedSpeculativePhysicalRoundSuccessV1,
    Box<M1AuthenticatedSpeculativeBootstrapRoundFailureV1>,
> {
    if engine.is_faulted() {
        return Err(bootstrap_terminal_failure(
            engine,
            M1AuthenticatedSpeculativeBootstrapRoundStageV1::EngineFaulted,
            (continuation, prepublication, ring_bytes, controls),
        ));
    }
    let queue = match M1AuthenticatedPhysicalQueueSessionV1::create(ring_bytes, prepublication) {
        Ok(queue) => queue,
        Err(M1AuthenticatedPhysicalQueueCreateFailureV1::Rejected {
            diagnostic,
            prepublication,
        }) => {
            return Err(bootstrap_pre_detach_failure(
                M1AuthenticatedSpeculativeBootstrapRoundStageV1::QueueCreate,
                M1AuthenticatedSpeculativeBootstrapPreDetachRetryStateV1::QueueCreate {
                    diagnostic: Box::new(diagnostic),
                    continuation,
                    prepublication: *prepublication,
                    ring_bytes,
                    controls,
                },
            ));
        }
        Err(M1AuthenticatedPhysicalQueueCreateFailureV1::Terminal(terminal)) => {
            return Err(bootstrap_terminal_failure(
                engine,
                M1AuthenticatedSpeculativeBootstrapRoundStageV1::QueueCreate,
                (terminal, continuation, controls),
            ));
        }
    };
    let (diagnostic, (continuation, controls)) =
        execute_initial_round_core::<M1NativeInitialQueueEffectsV1, _, C>(
            engine,
            queue,
            (continuation, controls),
        )
        .map_err(|(stage, disposition)| bootstrap_terminal_disposition(stage, disposition))?;
    continuation.complete_initial_round(engine, diagnostic, controls)
}

#[allow(clippy::unnecessary_box_returns)]
fn bootstrap_round_retryable(
    stage: M1AuthenticatedSpeculativeBootstrapRoundStageV1,
    continuation: M1AuthenticatedSpeculativeBootstrapContinuationV1,
    diagnostic: M1AuthenticatedSpeculativeDiagnosticCompletedReadbackV1,
    controls: Vec<M1SpeculativeMemberControlV1>,
) -> Box<PendingM1AuthenticatedSpeculativeBootstrapRoundFailureV1> {
    Box::new(PendingM1AuthenticatedSpeculativeBootstrapRoundFailureV1 {
        stage,
        custody: PendingM1AuthenticatedSpeculativeBootstrapRoundFailureCustodyV1::Retryable(
            Box::new((continuation, diagnostic, controls)),
        ),
    })
}

impl M1AuthenticatedSpeculativeBootstrapContinuationV1 {
    /// Joins the private physical lineage half after round-zero authenticated
    /// readback, settles physical KV, commits the coordinator, and only then
    /// releases retired pages into the repeated executor.
    ///
    /// # Errors
    ///
    /// Every failure quarantines the Engine, consumes the completed queue, and
    /// returns only clean release evidence or opaque terminal quarantine.
    fn complete_initial_round<const C: usize>(
        self,
        engine: &mut Engine<C>,
        diagnostic: M1AuthenticatedSpeculativeDiagnosticCompletedReadbackV1,
        controls: Vec<M1SpeculativeMemberControlV1>,
    ) -> Result<
        M1AuthenticatedSpeculativePhysicalRoundSuccessV1,
        Box<M1AuthenticatedSpeculativeBootstrapRoundFailureV1>,
    > {
        self.complete_initial_round_pending(engine, diagnostic, controls)
            .map_err(|failure| close_pending_bootstrap_failure(engine, failure))
    }

    fn complete_initial_round_pending<const C: usize>(
        self,
        engine: &mut Engine<C>,
        diagnostic: M1AuthenticatedSpeculativeDiagnosticCompletedReadbackV1,
        controls: Vec<M1SpeculativeMemberControlV1>,
    ) -> Result<
        M1AuthenticatedSpeculativePhysicalRoundSuccessV1,
        Box<PendingM1AuthenticatedSpeculativeBootstrapRoundFailureV1>,
    > {
        let checked = diagnostic.completed().checked();
        let Some(physical) = checked.speculative_lineage() else {
            return Err(bootstrap_round_retryable(
                M1AuthenticatedSpeculativeBootstrapRoundStageV1::Lineage,
                self,
                diagnostic,
                controls,
            ));
        };
        if physical.identity != self.lineage.identity
            || physical.coordinator_identity != self.coordinator.identity()
            || physical.selection != self.coordinator.shape().selection()
            || physical.round != 0
            || physical.epoch != self.epoch
            || checked.selection() != physical.selection
            || checked.epoch() != physical.epoch
            || !self
                .coordinator
                .matches_bootstrap_seeds(physical.selection, &physical.initial_seeds)
        {
            return Err(bootstrap_round_retryable(
                M1AuthenticatedSpeculativeBootstrapRoundStageV1::Lineage,
                self,
                diagnostic,
                controls,
            ));
        }
        let physical_coordinator_identity = physical.coordinator_identity;
        let physical_selection = physical.selection;

        let mut initial_seeds = Vec::new();
        let mut generated = Vec::new();
        let mut roster = Vec::new();
        let mut completion_members = Vec::new();
        let member_count = physical.initial_seeds.len();
        if initial_seeds.try_reserve_exact(member_count).is_err()
            || generated.try_reserve_exact(member_count).is_err()
            || roster.try_reserve_exact(member_count).is_err()
            || completion_members.try_reserve_exact(member_count).is_err()
        {
            return Err(bootstrap_round_retryable(
                M1AuthenticatedSpeculativeBootstrapRoundStageV1::HostAllocation,
                self,
                diagnostic,
                controls,
            ));
        }
        initial_seeds.extend_from_slice(&physical.initial_seeds);
        roster.extend(physical.initial_seeds.iter().map(|seed| seed.request()));
        generated.extend(roster.iter().copied().map(|request| (request, 0)));
        let binding = match self.coordinator.bind_round(0, self.epoch, &roster) {
            Ok(binding) => binding,
            Err(_) => {
                return Err(bootstrap_round_retryable(
                    M1AuthenticatedSpeculativeBootstrapRoundStageV1::CoordinatorPreflight,
                    self,
                    diagnostic,
                    controls,
                ));
            }
        };
        let preflighted = match self
            .coordinator
            .preflight_checked_round(binding, checked, &controls)
        {
            Ok(preflighted) => preflighted,
            Err(_) => {
                return Err(bootstrap_round_retryable(
                    M1AuthenticatedSpeculativeBootstrapRoundStageV1::CoordinatorPreflight,
                    self,
                    diagnostic,
                    controls,
                ));
            }
        };
        if self.selected.len() != preflighted.members().len() {
            return Err(bootstrap_round_retryable(
                M1AuthenticatedSpeculativeBootstrapRoundStageV1::CoordinatorPreflight,
                self,
                diagnostic,
                controls,
            ));
        }
        let Self {
            coordinator,
            epoch,
            selected,
            lineage: logical,
        } = self;
        for (cache, outcome) in selected.into_iter().zip(preflighted.members()) {
            debug_assert_eq!(cache.projection().request, outcome.request());
            completion_members.push(match outcome.physical_disposition() {
                crate::M1DeviceKvCompletionDispositionV1::Continue => {
                    M1DeviceKvCompletionMemberV1::continuing(cache)
                }
                crate::M1DeviceKvCompletionDispositionV1::Retire => {
                    M1DeviceKvCompletionMemberV1::retiring(cache)
                }
            });
        }
        let mut lineage = M1AuthenticatedSpeculativeCausalLineageV1 {
            logical,
            coordinator_identity: physical_coordinator_identity,
            selection: physical_selection,
            initial_seeds: initial_seeds.into_boxed_slice(),
            generated: generated.into_boxed_slice(),
            completed_rounds: 0,
            last_epoch: epoch,
        };
        let (readback, choices) = diagnostic.into_parts();
        let physical = complete_m1_authenticated_physical_step_v1(
            engine,
            readback,
            M1DeviceKvCompletionRosterV1::new(completion_members),
        );
        let physical = match physical {
            M1AuthenticatedCompletedStepOutcomeV1::Completed(physical) => physical,
            physical => {
                return Err(Box::new(PendingM1AuthenticatedSpeculativeBootstrapRoundFailureV1 {
                    stage: M1AuthenticatedSpeculativeBootstrapRoundStageV1::PhysicalCompletion,
                    custody:
                        PendingM1AuthenticatedSpeculativeBootstrapRoundFailureCustodyV1::PhysicalOutcome(
                            Box::new((
                                coordinator,
                                lineage,
                                controls,
                                choices,
                                preflighted,
                                physical,
                            )),
                        ),
                }));
            }
        };
        let mut coordinator = coordinator;
        let outcome = match coordinator.commit_preflighted_round(preflighted) {
            Ok(outcome) => outcome,
            Err(failure) => {
                engine.quarantine_m1_queue_rearm_failure();
                return Err(Box::new(PendingM1AuthenticatedSpeculativeBootstrapRoundFailureV1 {
                    stage: M1AuthenticatedSpeculativeBootstrapRoundStageV1::CoordinatorCommit,
                    custody:
                        PendingM1AuthenticatedSpeculativeBootstrapRoundFailureCustodyV1::CoordinatorCommit(
                            Box::new((
                                coordinator,
                                lineage,
                                controls,
                                choices,
                                physical,
                                failure,
                            )),
                        ),
                }));
            }
        };
        if !refresh_causal_generated_counts(&mut lineage, &coordinator, outcome.completed_epoch()) {
            engine.quarantine_m1_queue_rearm_failure();
            return Err(Box::new(
                PendingM1AuthenticatedSpeculativeBootstrapRoundFailureV1 {
                    stage: M1AuthenticatedSpeculativeBootstrapRoundStageV1::Lineage,
                    custody:
                        PendingM1AuthenticatedSpeculativeBootstrapRoundFailureCustodyV1::LineageAfterCommit(
                            Box::new((coordinator, lineage, controls, choices, physical, outcome)),
                        ),
                },
            ));
        }
        let released = match release_m1_authenticated_completed_step_kv_pages_v1(physical) {
            Ok(released) => M1AuthenticatedLongLivedQueueReleasedRoundV1::initial(released),
            Err(failure) => {
                return Err(Box::new(PendingM1AuthenticatedSpeculativeBootstrapRoundFailureV1 {
                    stage: M1AuthenticatedSpeculativeBootstrapRoundStageV1::PageRelease,
                    custody:
                        PendingM1AuthenticatedSpeculativeBootstrapRoundFailureCustodyV1::PageRelease(
                            Box::new((coordinator, lineage, outcome, choices, failure)),
                        ),
                }));
            }
        };
        if validate_prior_association(&coordinator, &outcome, &released).is_err()
            || !validate_causal_lineage(&coordinator, &released, &lineage)
        {
            engine.quarantine_m1_queue_rearm_failure();
            return Err(Box::new(
                PendingM1AuthenticatedSpeculativeBootstrapRoundFailureV1 {
                    stage: M1AuthenticatedSpeculativeBootstrapRoundStageV1::Lineage,
                    custody:
                        PendingM1AuthenticatedSpeculativeBootstrapRoundFailureCustodyV1::ReleasedLineage(
                            Box::new((coordinator, lineage, outcome, choices, released)),
                        ),
                },
            ));
        }
        Ok(M1AuthenticatedSpeculativePhysicalRoundSuccessV1 {
            executor: M1AuthenticatedSpeculativePhysicalExecutorV1 {
                coordinator,
                released,
                lineage,
            },
            outcome,
            choices,
        })
    }
}

fn complete_authenticated_rearmed_speculative_round<const C: usize>(
    engine: &mut Engine<C>,
    coordinator: M1SpeculativeGenerationLoopV1,
    binding: crate::M1SpeculativeRoundBindingV1,
    diagnostic: crate::M1AuthenticatedRearmedSpeculativeDiagnosticCompletedReadbackV1,
    controls: Vec<M1SpeculativeMemberControlV1>,
    lineage: M1AuthenticatedSpeculativeCausalLineageV1,
) -> Result<
    M1AuthenticatedSpeculativePhysicalRoundSuccessV1,
    Box<PendingM1AuthenticatedSpeculativePhysicalRoundFailureV1>,
> {
    let prepared = prepare_coordinator_round_core(
        engine,
        coordinator,
        binding,
        diagnostic,
        controls,
        lineage,
    )
    .map_err(|failure| {
        let (custody, lineage) = match failure {
            M1PrepareCoordinatorRoundCoreFailureV1::Preflight {
                coordinator,
                diagnostic,
                controls,
                error,
                lineage,
            } => (
                M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::CoordinatorPreflight(
                    Box::new((coordinator, diagnostic, controls, error)),
                ),
                lineage,
            ),
            M1PrepareCoordinatorRoundCoreFailureV1::CommitPreflight {
                coordinator,
                diagnostic,
                controls,
                preflighted,
                error,
                lineage,
            } => (
                M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::CoordinatorCommitPreflight(
                    Box::new((coordinator, diagnostic, controls, preflighted, error)),
                ),
                lineage,
            ),
            M1PrepareCoordinatorRoundCoreFailureV1::HostAllocation {
                coordinator,
                diagnostic,
                controls,
                preflighted,
                lineage,
            } => (
                M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::HostAllocation(Box::new((
                    coordinator,
                    diagnostic,
                    controls,
                    preflighted,
                ))),
                lineage,
            ),
        };
        Box::new(PendingM1AuthenticatedSpeculativePhysicalRoundFailureV1 {
            stage: M1AuthenticatedSpeculativePhysicalRoundStageV1::CoordinatorPreflight,
            custody,
            lineage: Some(lineage),
        })
    })?;
    let M1PreparedCoordinatorRoundCoreV1 {
        coordinator,
        diagnostic,
        controls,
        preflighted,
        dispositions,
        lineage,
    } = prepared;
    let (readback, choices) = diagnostic.into_parts();
    let physical = match readback.complete(engine, dispositions) {
        Ok(physical) => physical,
        Err(failure) => {
            return Err(Box::new(PendingM1AuthenticatedSpeculativePhysicalRoundFailureV1 {
                stage: M1AuthenticatedSpeculativePhysicalRoundStageV1::PhysicalCompletion,
                custody:
                    M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::PhysicalCompletionPreflight(
                        Box::new((coordinator, preflighted, controls, choices, failure)),
                    ),
                lineage: Some(lineage),
            }));
        }
    };
    if !matches!(
        physical.outcome(),
        crate::M1AuthenticatedCompletedStepOutcomeV1::Completed(_)
    ) {
        engine.quarantine_m1_queue_rearm_failure();
        return Err(Box::new(
            PendingM1AuthenticatedSpeculativePhysicalRoundFailureV1 {
                stage: M1AuthenticatedSpeculativePhysicalRoundStageV1::PhysicalCompletion,
                custody: M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::PhysicalOutcome(
                    Box::new((coordinator, preflighted, controls, choices, physical)),
                ),
                lineage: Some(lineage),
            },
        ));
    }
    let committed = commit_coordinator_round_core(
        engine,
        coordinator,
        preflighted,
        controls,
        (choices, physical),
        lineage,
    )
    .map_err(|failure| match failure {
        M1CommitCoordinatorRoundCoreFailureV1::Coordinator {
            coordinator,
            controls,
            physical: (choices, physical),
            failure,
            lineage,
        } => Box::new(PendingM1AuthenticatedSpeculativePhysicalRoundFailureV1 {
            stage: M1AuthenticatedSpeculativePhysicalRoundStageV1::CoordinatorCommit,
            custody: M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::CoordinatorCommit(
                Box::new((coordinator, controls, choices, physical, failure)),
            ),
            lineage: Some(lineage),
        }),
        M1CommitCoordinatorRoundCoreFailureV1::CausalLineage {
            coordinator,
            outcome,
            physical: (choices, physical),
            lineage,
        } => Box::new(PendingM1AuthenticatedSpeculativePhysicalRoundFailureV1 {
            stage: M1AuthenticatedSpeculativePhysicalRoundStageV1::CausalLineage,
            custody:
                M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::LineageAfterCommit(
                    Box::new((coordinator, outcome, choices, physical)),
                ),
            lineage: Some(lineage),
        }),
    })?;
    let M1CommittedCoordinatorRoundCoreV1 {
        coordinator,
        outcome,
        physical: (choices, physical),
        lineage,
    } = committed;
    match physical.release_completed() {
        M1AuthenticatedRearmedRoundReleaseOutcomeV1::Released(released) => {
            if validate_prior_association(&coordinator, &outcome, &released).is_err()
                || !validate_causal_lineage(&coordinator, &released, &lineage)
            {
                engine.quarantine_m1_queue_rearm_failure();
                return Err(Box::new(
                    PendingM1AuthenticatedSpeculativePhysicalRoundFailureV1 {
                        stage: M1AuthenticatedSpeculativePhysicalRoundStageV1::CausalLineage,
                        custody:
                            M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::ReleasedLineage(
                                Box::new((coordinator, outcome, choices, released)),
                            ),
                        lineage: Some(lineage),
                    },
                ));
            }
            Ok(M1AuthenticatedSpeculativePhysicalRoundSuccessV1 {
                executor: M1AuthenticatedSpeculativePhysicalExecutorV1 {
                    coordinator,
                    released,
                    lineage,
                },
                outcome,
                choices,
            })
        }
        M1AuthenticatedRearmedRoundReleaseOutcomeV1::Rejected(failure) => Err(Box::new(
            PendingM1AuthenticatedSpeculativePhysicalRoundFailureV1 {
                stage: M1AuthenticatedSpeculativePhysicalRoundStageV1::PageRelease,
                custody: M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::PageRelease(
                    Box::new(M1AuthenticatedSpeculativePhysicalPageReleaseCustodyV1 {
                        coordinator,
                        outcome,
                        choices,
                        failure: *failure,
                    }),
                ),
                lineage: Some(lineage),
            },
        )),
        M1AuthenticatedRearmedRoundReleaseOutcomeV1::NotCompleted(not_completed) => {
            engine.quarantine_m1_queue_rearm_failure();
            Err(Box::new(PendingM1AuthenticatedSpeculativePhysicalRoundFailureV1 {
                stage: M1AuthenticatedSpeculativePhysicalRoundStageV1::PhysicalCompletion,
                custody:
                    M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::ReleasedPhysicalOutcome(
                        Box::new((coordinator, outcome, choices, not_completed)),
                    ),
                lineage: Some(lineage),
            }))
        }
    }
}

impl M1AuthenticatedSpeculativeRolloverContinuationV1 {
    /// Joins the first speculative readback to the causal witness created from
    /// its exact authenticated paired-prefill predecessor.
    ///
    /// # Errors
    ///
    /// Every failure quarantines the Engine, consumes the detached queue when
    /// possible, and returns no retry or scheduling authority.
    fn complete_rollover_round<const C: usize>(
        self,
        engine: &mut Engine<C>,
        diagnostic: crate::M1AuthenticatedRearmedSpeculativeDiagnosticCompletedReadbackV1,
        controls: Vec<M1SpeculativeMemberControlV1>,
    ) -> Result<
        M1AuthenticatedSpeculativePhysicalRoundSuccessV1,
        M1AuthenticatedSpeculativeRolloverRoundFailureV1,
    > {
        self.complete_rollover_round_pending(engine, diagnostic, controls)
            .map_err(|failure| close_pending_rollover_failure(engine, failure))
    }

    fn complete_rollover_round_pending<const C: usize>(
        self,
        engine: &mut Engine<C>,
        diagnostic: crate::M1AuthenticatedRearmedSpeculativeDiagnosticCompletedReadbackV1,
        controls: Vec<M1SpeculativeMemberControlV1>,
    ) -> Result<
        M1AuthenticatedSpeculativePhysicalRoundSuccessV1,
        PendingM1AuthenticatedSpeculativeRolloverRoundFailureV1,
    > {
        let checked = diagnostic.checked();
        let Some(physical) = checked.speculative_lineage() else {
            return Err(
                PendingM1AuthenticatedSpeculativeRolloverRoundFailureV1::Unsettled {
                    stage: M1AuthenticatedSpeculativePhysicalRoundStageV1::CausalLineage,
                    retained: Box::new((self, diagnostic, controls)),
                },
            );
        };
        if physical.identity != self.lineage.identity
            || physical.coordinator_identity != self.coordinator.identity()
            || physical.selection != self.coordinator.shape().selection()
            || physical.round != 0
            || physical.epoch != self.epoch
            || checked.selection() != physical.selection
            || checked.epoch() != physical.epoch
            || !self
                .coordinator
                .matches_bootstrap_seeds(physical.selection, &physical.initial_seeds)
        {
            return Err(
                PendingM1AuthenticatedSpeculativeRolloverRoundFailureV1::Unsettled {
                    stage: M1AuthenticatedSpeculativePhysicalRoundStageV1::CausalLineage,
                    retained: Box::new((self, diagnostic, controls)),
                },
            );
        }
        let mut initial_seeds = Vec::new();
        let mut generated = Vec::new();
        let mut roster = Vec::new();
        if initial_seeds
            .try_reserve_exact(physical.initial_seeds.len())
            .is_err()
            || generated
                .try_reserve_exact(physical.initial_seeds.len())
                .is_err()
            || roster
                .try_reserve_exact(physical.initial_seeds.len())
                .is_err()
        {
            return Err(
                PendingM1AuthenticatedSpeculativeRolloverRoundFailureV1::Unsettled {
                    stage: M1AuthenticatedSpeculativePhysicalRoundStageV1::CoordinatorPreflight,
                    retained: Box::new((self, diagnostic, controls)),
                },
            );
        }
        initial_seeds.extend_from_slice(&physical.initial_seeds);
        roster.extend(physical.initial_seeds.iter().map(|seed| seed.request()));
        generated.extend(roster.iter().copied().map(|request| (request, 0)));
        let binding = match self.coordinator.bind_round(0, self.epoch, &roster) {
            Ok(binding) => binding,
            Err(_) => {
                return Err(
                    PendingM1AuthenticatedSpeculativeRolloverRoundFailureV1::Unsettled {
                        stage: M1AuthenticatedSpeculativePhysicalRoundStageV1::Bind,
                        retained: Box::new((self, diagnostic, controls)),
                    },
                );
            }
        };
        let lineage = M1AuthenticatedSpeculativeCausalLineageV1 {
            logical: self.lineage,
            coordinator_identity: physical.coordinator_identity,
            selection: physical.selection,
            initial_seeds: initial_seeds.into_boxed_slice(),
            generated: generated.into_boxed_slice(),
            completed_rounds: 0,
            last_epoch: self.epoch,
        };
        complete_authenticated_rearmed_speculative_round(
            engine,
            self.coordinator,
            binding,
            diagnostic,
            controls,
            lineage,
        )
        .map_err(PendingM1AuthenticatedSpeculativeRolloverRoundFailureV1::Round)
    }
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

fn validate_causal_lineage(
    coordinator: &M1SpeculativeGenerationLoopV1,
    released: &M1AuthenticatedLongLivedQueueReleasedRoundV1,
    lineage: &M1AuthenticatedSpeculativeCausalLineageV1,
) -> bool {
    let Ok(physical) = released.speculative_lineage_witness() else {
        return false;
    };
    validate_causal_lineage_join(
        coordinator,
        physical,
        lineage,
        released.current_released().checked().epoch(),
        released.speculative_history_count(lineage.selection),
    )
}

fn validate_causal_lineage_join(
    coordinator: &M1SpeculativeGenerationLoopV1,
    physical: &M1AuthenticatedSpeculativePhysicalLineageWitnessV1,
    lineage: &M1AuthenticatedSpeculativeCausalLineageV1,
    current_epoch: CompletionEpoch,
    history_count: usize,
) -> bool {
    physical.identity == lineage.logical.identity
        && physical.coordinator_identity == lineage.coordinator_identity
        && physical.coordinator_identity == coordinator.identity()
        && physical.selection == lineage.selection
        && physical.round == 0
        && physical.epoch.value() != 0
        && physical.initial_seeds == lineage.initial_seeds
        && lineage.completed_rounds == coordinator.next_round()
        && coordinator.last_epoch() == Some(lineage.last_epoch)
        && current_epoch == lineage.last_epoch
        && history_count
            .checked_add(1)
            .and_then(|count| u64::try_from(count).ok())
            == Some(lineage.completed_rounds)
        && coordinator.matches_causal_lineage(
            lineage.selection,
            &lineage.initial_seeds,
            &lineage.generated,
            lineage.completed_rounds,
            Some(lineage.last_epoch),
        )
}

fn refresh_causal_generated_counts(
    lineage: &mut M1AuthenticatedSpeculativeCausalLineageV1,
    coordinator: &M1SpeculativeGenerationLoopV1,
    epoch: CompletionEpoch,
) -> bool {
    if lineage.generated.len() != lineage.initial_seeds.len() {
        return false;
    }
    for ((request, generated), seed) in lineage.generated.iter_mut().zip(&lineage.initial_seeds) {
        let Some(member) = coordinator.member(seed.request()) else {
            return false;
        };
        if *request != seed.request() || member.request() != seed.request() {
            return false;
        }
        *generated = member.generated_tokens();
    }
    lineage.completed_rounds = coordinator.next_round();
    lineage.last_epoch = epoch;
    coordinator.matches_causal_lineage(
        lineage.selection,
        &lineage.initial_seeds,
        &lineage.generated,
        lineage.completed_rounds,
        Some(lineage.last_epoch),
    )
}

#[allow(clippy::unnecessary_box_returns)]
fn retryable_failure(
    stage: M1AuthenticatedSpeculativePhysicalRoundStageV1,
    executor: M1AuthenticatedSpeculativePhysicalExecutorV1,
    inputs: M1AuthenticatedSpeculativePhysicalRoundInputsV1,
) -> Box<PendingM1AuthenticatedSpeculativePhysicalRoundFailureV1> {
    Box::new(PendingM1AuthenticatedSpeculativePhysicalRoundFailureV1 {
        stage,
        custody: M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::Retryable(Box::new((
            executor, inputs,
        ))),
        lineage: None,
    })
}

fn retain_logical_lineage(
    lineage: Option<M1AuthenticatedSpeculativeCausalLineageV1>,
    logical: impl fmt::Debug + 'static,
) -> Box<dyn fmt::Debug> {
    Box::new((lineage, logical))
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
    PendingM1AuthenticatedSpeculativePhysicalRoundFailureV1,
> {
    engine.quarantine_m1_queue_rearm_failure();
    Ok(Err(Box::new(
        M1AuthenticatedSpeculativePhysicalRoundTeardownFailureV1 {
            stage,
            source:
                M1AuthenticatedSpeculativePhysicalRoundTeardownFailureSourceV1::TerminalQuarantine(
                    source,
                ),
            logical: Box::new(logical),
        },
    )))
}

impl M1AuthenticatedSpeculativePhysicalExecutorV1 {
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
            lineage,
        } = self;
        match released.destroy_queue_and_retain_round(engine) {
            Ok(released) => Ok(M1AuthenticatedSpeculativeExecutorTeardownSuccessV1 {
                coordinator,
                released,
                lineage,
            }),
            Err(released) => Err(Box::new(
                M1AuthenticatedSpeculativeExecutorTeardownFailureV1 {
                    coordinator,
                    released,
                    lineage,
                },
            )),
        }
    }

    /// Executes one authenticated KFD queue generation.
    ///
    /// The executor is returned only after physical settlement, logical commit,
    /// page release, and causal validation all succeed. Pure validation and
    /// scheduling rejection before detachment returns one opaque exact retry
    /// owner. Once scheduling detaches the queue, the Engine is quarantined even
    /// when queue destruction itself succeeds.
    ///
    /// # Errors
    ///
    /// Returns opaque pre-detach retry custody, clean queue-release evidence, or
    /// opaque terminal quarantine. No failure exposes raw readback, completion,
    /// or scheduling authority.
    pub fn execute_round<const C: usize>(
        self,
        engine: &mut Engine<C>,
        inputs: M1AuthenticatedSpeculativePhysicalRoundInputsV1,
    ) -> Result<
        M1AuthenticatedSpeculativePhysicalRoundSuccessV1,
        Box<M1AuthenticatedSpeculativePhysicalRoundFailureV1>,
    > {
        self.execute_round_pending(engine, inputs)
            .map_err(|failure| close_pending_round_failure(engine, failure))
    }

    fn execute_round_pending<const C: usize>(
        self,
        engine: &mut Engine<C>,
        inputs: M1AuthenticatedSpeculativePhysicalRoundInputsV1,
    ) -> Result<
        M1AuthenticatedSpeculativePhysicalRoundSuccessV1,
        Box<PendingM1AuthenticatedSpeculativePhysicalRoundFailureV1>,
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
            return Err(Box::new(
                PendingM1AuthenticatedSpeculativePhysicalRoundFailureV1 {
                    stage: M1AuthenticatedSpeculativePhysicalRoundStageV1::Complete,
                    custody: M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::Complete(
                        Box::new((self, inputs)),
                    ),
                    lineage: None,
                },
            ));
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
            lineage,
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
                        lineage,
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
                        lineage,
                    },
                    inputs,
                ));
            }
        };
        if !speculative_round_inputs_match_binding(&inputs.kv, &binding) {
            return Err(retryable_failure(
                M1AuthenticatedSpeculativePhysicalRoundStageV1::Inputs,
                Self {
                    coordinator,
                    released,
                    lineage,
                },
                inputs,
            ));
        }
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
                        lineage,
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
                return Err(Box::new(
                    PendingM1AuthenticatedSpeculativePhysicalRoundFailureV1 {
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
                        lineage: Some(lineage),
                    },
                ));
            }
        };
        let recipe = match scheduled.derive_retained_step_recipe(recipe_workspace_plans) {
            M1PhysicalRunnerRecipeOutcomeV1::Prepared(recipe) => recipe,
            M1PhysicalRunnerRecipeOutcomeV1::Rejected(failure) => {
                engine.quarantine_m1_queue_rearm_failure();
                return Err(Box::new(
                    PendingM1AuthenticatedSpeculativePhysicalRoundFailureV1 {
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
                        lineage: Some(lineage),
                    },
                ));
            }
        };
        let reserved =
            match reserve_m1_authenticated_long_lived_queue_rearm_kv_v1(engine, scheduled, kv) {
                Ok(reserved) => reserved,
                Err(failure) => {
                    return Err(Box::new(PendingM1AuthenticatedSpeculativePhysicalRoundFailureV1 {
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
                        lineage: Some(lineage),
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
                return Err(Box::new(PendingM1AuthenticatedSpeculativePhysicalRoundFailureV1 {
                    stage: M1AuthenticatedSpeculativePhysicalRoundStageV1::WorkspacePreparation,
                    custody:
                        M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::WorkspacePreparation(
                            Box::new((coordinator, binding, recipe, controls, failure)),
                        ),
                    lineage: Some(lineage),
                }));
            }
        };
        let (diagnostic, (coordinator, binding, controls, lineage)) =
            execute_round_core::<M1NativeRearmedQueueEffectsV1, _, C>(
                engine,
                (prepared, recipe),
                (coordinator, binding, controls, lineage),
            )
            .map_err(|(stage, disposition)| {
                Box::new(PendingM1AuthenticatedSpeculativePhysicalRoundFailureV1 {
                    stage,
                    custody: M1AuthenticatedSpeculativePhysicalRoundFailureCustodyV1::Closed(
                        disposition,
                    ),
                    lineage: None,
                })
            })?;
        complete_authenticated_rearmed_speculative_round(
            engine,
            coordinator,
            binding,
            diagnostic,
            controls,
            lineage,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::convert::Infallible;
    use ferric_spec::{
        validate_m1_step_inputs, Identity, M1StepInputCandidate, M1StepInputValidationOutcome,
        Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, StepPlan,
    };

    use crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1;
    use crate::authenticated_test_runtime::{
        ModelCompletedQueueV1, ModelDiagnosticV1, ModelMemberReadbackV1, ModelPreparedQueueV1,
        ModelPublishedQueueV1, ModelQueueFailureV1, ModelQueueV1, ModelReadbackFailureV1,
        ModelRecycledQueueV1, ModelSubmitFailureV1, ModelWaitFailureV1,
    };
    use crate::speculative_generation_loop::CheckedMemberObservationV1;
    use crate::{CheckedCompletionSemantics, M1SpeculativeTokenBlockV1};

    struct ModelInitialQueueEffectsV1;

    fn close_model_submit_failure(
        failure: ModelSubmitFailureV1,
    ) -> M1AuthenticatedPhysicalQueueClosureV1 {
        let clean = failure.releases_cleanly;
        failure.prepared.destroy(clean);
        if clean {
            M1AuthenticatedPhysicalQueueClosureV1::Released(Box::new("model prepared destroyed"))
        } else {
            M1AuthenticatedPhysicalQueueClosureV1::Quarantined(Box::new(
                "model prepared quarantined",
            ))
        }
    }

    fn close_model_readback_failure(
        failure: ModelReadbackFailureV1,
    ) -> M1AuthenticatedPhysicalQueueClosureV1 {
        let clean = failure.releases_cleanly;
        failure.recycled.destroy(clean);
        if clean {
            M1AuthenticatedPhysicalQueueClosureV1::Released(Box::new("model recycled destroyed"))
        } else {
            M1AuthenticatedPhysicalQueueClosureV1::Quarantined(Box::new(
                "model recycled quarantined",
            ))
        }
    }

    impl M1InitialQueueEffectsV1 for ModelInitialQueueEffectsV1 {
        type Prepared = ModelPreparedQueueV1;
        type Published = ModelPublishedQueueV1;
        type Completed = ModelCompletedQueueV1;
        type Recycled = ModelRecycledQueueV1;
        type Observed = ModelDiagnosticV1;
        type DiagnosticObserved = ModelDiagnosticV1;
        type Diagnostic = ModelDiagnosticV1;
        type SubmitFailure = ModelSubmitFailureV1;
        type WaitFailure = ModelWaitFailureV1;
        type RecycleFailure = Infallible;
        type ObservationFailure = ModelReadbackFailureV1;
        type DiagnosticObservationFailure = Infallible;
        type JoinFailure = Infallible;

        fn submit(
            prepared: Self::Prepared,
        ) -> Result<Self::Published, Self::SubmitFailure> {
            prepared.submit()
        }

        fn close_submit_failure<const C: usize>(
            engine: &mut Engine<C>,
            failure: Self::SubmitFailure,
        ) -> M1AuthenticatedPhysicalQueueClosureV1 {
            engine.quarantine_m1_queue_rearm_failure();
            close_model_submit_failure(failure)
        }

        fn wait(published: Self::Published) -> Result<Self::Completed, Self::WaitFailure> {
            published.wait()
        }

        fn recycle(completed: Self::Completed) -> Result<Self::Recycled, Self::RecycleFailure> {
            Ok(completed.recycle())
        }

        fn observe(recycled: Self::Recycled) -> Result<Self::Observed, Self::ObservationFailure> {
            recycled.readback()
        }

        fn close_observation_failure<const C: usize>(
            engine: &mut Engine<C>,
            failure: Self::ObservationFailure,
        ) -> M1AuthenticatedPhysicalQueueClosureV1 {
            engine.quarantine_m1_queue_rearm_failure();
            close_model_readback_failure(failure)
        }

        fn observe_diagnostic(
            observed: Self::Observed,
        ) -> Result<Self::DiagnosticObserved, Self::DiagnosticObservationFailure> {
            Ok(observed)
        }

        fn close_diagnostic_observation_failure<const C: usize>(
            _engine: &mut Engine<C>,
            failure: Self::DiagnosticObservationFailure,
        ) -> M1AuthenticatedPhysicalQueueClosureV1 {
            match failure {}
        }

        fn check(
            observed: Self::DiagnosticObserved,
        ) -> Result<Self::Diagnostic, Self::JoinFailure> {
            Ok(observed)
        }

        fn close_join_failure<const C: usize>(
            _engine: &mut Engine<C>,
            failure: Self::JoinFailure,
        ) -> M1AuthenticatedPhysicalQueueClosureV1 {
            match failure {}
        }
    }

    struct ModelRearmedQueueEffectsV1;

    impl M1RearmedQueueEffectsV1 for ModelRearmedQueueEffectsV1 {
        type Prepared = ModelPreparedQueueV1;
        type Published = ModelPublishedQueueV1;
        type Completed = ModelCompletedQueueV1;
        type Recycled = ModelRecycledQueueV1;
        type Diagnostic = ModelDiagnosticV1;
        type SubmitFailure = ModelSubmitFailureV1;
        type ProgressFailure = ModelWaitFailureV1;
        type ReadbackFailure = ModelReadbackFailureV1;

        fn submit<const C: usize>(
            _engine: &mut Engine<C>,
            prepared: Self::Prepared,
        ) -> Result<Self::Published, Self::SubmitFailure> {
            prepared.submit()
        }

        fn classify_submit_failure(
            failure: Self::SubmitFailure,
        ) -> M1AuthenticatedPhysicalQueueClosureV1 {
            close_model_submit_failure(failure)
        }

        fn wait<const C: usize>(
            _engine: &mut Engine<C>,
            published: Self::Published,
        ) -> Result<Self::Completed, Self::ProgressFailure> {
            published.wait()
        }

        fn recycle<const C: usize>(
            _engine: &mut Engine<C>,
            completed: Self::Completed,
        ) -> Result<Self::Recycled, Self::ProgressFailure> {
            Ok(completed.recycle())
        }

        fn readback(
            recycled: Self::Recycled,
        ) -> Result<Self::Diagnostic, Self::ReadbackFailure> {
            recycled.readback()
        }

        fn close_readback_failure<const C: usize>(
            _engine: &mut Engine<C>,
            failure: Self::ReadbackFailure,
        ) -> M1AuthenticatedPhysicalQueueClosureV1 {
            close_model_readback_failure(failure)
        }
    }

    impl M1SpeculativeRoundObservationV1 for ModelDiagnosticV1 {
        fn preflight(
            &self,
            coordinator: &M1SpeculativeGenerationLoopV1,
            binding: crate::M1SpeculativeRoundBindingV1,
            controls: &[M1SpeculativeMemberControlV1],
        ) -> Result<crate::M1SpeculativePreflightedRoundV1, crate::M1SpeculativeGenerationLoopErrorV1>
        {
            let epoch = binding.epoch();
            let observations: Vec<_> = binding
                .members()
                .iter()
                .zip(&self.members)
                .map(|(input, member)| CheckedMemberObservationV1 {
                    request: input.request(),
                    semantics: CheckedCompletionSemantics::Speculative {
                        accepted_draft_tokens: member.accepted,
                        correction_or_bonus: *member
                            .emitted
                            .last()
                            .expect("model completion has a correction or bonus"),
                    },
                    emitted: M1SpeculativeTokenBlockV1::from_slice(&member.emitted)
                        .expect("model completion respects the selected K bound"),
                })
                .collect();
            coordinator.preflight_observed_round(
                binding,
                coordinator.shape().selection(),
                epoch,
                &observations,
                controls,
            )
        }
    }

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

    fn model_lineage(
        coordinator: &M1SpeculativeGenerationLoopV1,
    ) -> M1AuthenticatedSpeculativeCausalLineageV1 {
        let initial_seeds = coordinator.bootstrap_seed_snapshot().unwrap();
        let generated = initial_seeds
            .iter()
            .map(|seed| (seed.request(), 0))
            .collect::<Vec<_>>()
            .into_boxed_slice();
        M1AuthenticatedSpeculativeCausalLineageV1 {
            logical: M1AuthenticatedSpeculativeLogicalLineageWitnessV1 {
                identity: M1AuthenticatedSpeculativeLineageIdentityV1::fresh().unwrap(),
            },
            coordinator_identity: coordinator.identity(),
            selection: coordinator.shape().selection(),
            initial_seeds,
            generated,
            completed_rounds: 0,
            last_epoch: CompletionEpoch::new(39),
        }
    }

    fn model_members(
        member_count: usize,
        accepted: u8,
        stop_lane: Option<usize>,
    ) -> Vec<ModelMemberReadbackV1> {
        (0..member_count)
            .map(|lane| {
                let mut emitted = (0..accepted)
                    .map(|token| 500 + u32::from(token) + u32::try_from(lane).unwrap() * 16)
                    .collect::<Vec<_>>();
                emitted.push(if stop_lane == Some(lane) {
                    999
                } else {
                    700 + u32::try_from(lane).unwrap()
                });
                ModelMemberReadbackV1 { accepted, emitted }
            })
            .collect()
    }

    fn model_coordinator(
        selected: Qwen3PlanSelection,
        member_count: usize,
    ) -> M1SpeculativeGenerationLoopV1 {
        let policy = crate::M1SpeculativeGenerationPolicyV1::new(32, &[999]).unwrap();
        let seeds = (0..member_count)
            .map(|lane| {
                M1SpeculativeMemberSeedV1::new(
                    RequestId::new(u32::try_from(lane).unwrap(), 1),
                    70 + u32::try_from(lane).unwrap(),
                    10,
                    10,
                    policy,
                )
            })
            .collect::<Vec<_>>();
        M1SpeculativeGenerationLoopV1::new(selected, &seeds).unwrap()
    }

    fn finish_model_round<const C: usize>(
        engine: &mut Engine<C>,
        prepared: M1PreparedCoordinatorRoundCoreV1<ModelDiagnosticV1>,
    ) -> M1CommittedCoordinatorRoundCoreV1<ModelDiagnosticV1> {
        assert_eq!(
            prepared.dispositions.len(),
            prepared.preflighted.members().len()
        );
        let committed = commit_coordinator_round_core(
            engine,
            prepared.coordinator,
            prepared.preflighted,
            prepared.controls,
            prepared.diagnostic,
            prepared.lineage,
        )
        .unwrap();
        committed.physical.queue.release_generation();
        committed
    }

    #[test]
    fn model_queue_cores_drive_initial_repeated_and_rollover_lifecycles() {
        let profiles = [
            (Qwen3PlanBucket::SpeculativeS1K4C8192, 1_usize, 3_u8),
            (Qwen3PlanBucket::SpeculativeS1K8C8192, 1, 5),
            (Qwen3PlanBucket::SpeculativeS1K16C8192, 1, 9),
            (Qwen3PlanBucket::SpeculativeS8K4C8192, 8, 2),
        ];
        for (bucket, member_count, accepted) in profiles {
            let selected = selection(bucket);
            let queue = ModelQueueV1::new(
                [None, None],
                [
                    model_members(member_count, accepted, None),
                    model_members(member_count, 1, None),
                ],
            );
            let coordinator = model_coordinator(selected, member_count);
            let lineage = model_lineage(&coordinator);
            let roster = coordinator.active_roster();
            let binding = coordinator
                .bind_round(0, CompletionEpoch::new(40), &roster)
                .unwrap();
            let controls = roster
                .iter()
                .copied()
                .map(M1SpeculativeMemberControlV1::continuing)
                .collect::<Vec<_>>();
            let mut engine = Engine::<1>::new(8, 4, 32).unwrap();
            let (diagnostic, (coordinator, binding, controls, lineage)) =
                execute_initial_round_core::<ModelInitialQueueEffectsV1, _, 1>(
                    &mut engine,
                    ModelPreparedQueueV1::new(queue.clone()),
                    (coordinator, binding, controls, lineage),
                )
                .unwrap();
            let prepared = prepare_coordinator_round_core(
                &mut engine,
                coordinator,
                binding,
                diagnostic,
                controls,
                lineage,
            )
            .unwrap();
            let first = finish_model_round(&mut engine, prepared);
            assert_eq!(first.outcome.members().len(), member_count);
            assert!(first
                .outcome
                .members()
                .iter()
                .all(|member| member.accepted_draft_tokens() == accepted));

            let coordinator = first.coordinator;
            let lineage = first.lineage;
            let roster = coordinator.active_roster();
            let binding = coordinator
                .bind_round(1, CompletionEpoch::new(41), &roster)
                .unwrap();
            let controls = roster
                .iter()
                .copied()
                .map(M1SpeculativeMemberControlV1::continuing)
                .collect::<Vec<_>>();
            let (diagnostic, (coordinator, binding, controls, lineage)) =
                execute_round_core::<ModelRearmedQueueEffectsV1, _, 1>(
                    &mut engine,
                    ModelPreparedQueueV1::new(queue.clone()),
                    (coordinator, binding, controls, lineage),
                )
                .unwrap();
            let prepared = prepare_coordinator_round_core(
                &mut engine,
                coordinator,
                binding,
                diagnostic,
                controls,
                lineage,
            )
            .unwrap();
            let second = finish_model_round(&mut engine, prepared);
            assert_eq!(second.outcome.completed_round(), 1);
            assert_eq!(second.lineage.completed_rounds, 2);
            assert!(!engine.is_faulted());
            let snapshot = queue.snapshot();
            assert_eq!(snapshot.submits, 2);
            assert_eq!(snapshot.waits, 2);
            assert_eq!(snapshot.recycles, 2);
            assert_eq!(snapshot.readbacks, 2);
            assert_eq!(snapshot.releases, 2);

            let rollover = ModelQueueV1::new(
                [None],
                [model_members(member_count, 0, None)],
            );
            rollover.publish_for_rollover();
            let (_diagnostic, logical) =
                complete_round_core::<ModelRearmedQueueEffectsV1, _, 1>(
                    &mut engine,
                    ModelPublishedQueueV1::from_published(rollover.clone()),
                    "rollover continuation",
                )
                .unwrap();
            assert_eq!(logical, "rollover continuation");
            let snapshot = rollover.snapshot();
            assert_eq!((snapshot.submits, snapshot.waits, snapshot.recycles), (1, 1, 1));
            assert_eq!(snapshot.readbacks, 1);
        }
    }

    #[test]
    fn model_currentness_and_readback_failures_close_once_without_resubmit() {
        for failure in [
            ModelQueueFailureV1::CurrentnessReleased,
            ModelQueueFailureV1::CurrentnessQuarantined,
        ] {
            let queue = ModelQueueV1::new([Some(failure)], []);
            let mut engine = Engine::<1>::new(8, 4, 32).unwrap();
            let (_stage, disposition) =
                execute_round_core::<ModelRearmedQueueEffectsV1, _, 1>(
                    &mut engine,
                    ModelPreparedQueueV1::new(queue.clone()),
                    "retained logical",
                )
                .unwrap_err();
            assert!(engine.is_faulted());
            assert_eq!(queue.snapshot().submits, 0);
            assert_eq!(queue.snapshot().destroys, 1);
            assert_eq!(
                matches!(
                    disposition,
                    M1AuthenticatedSpeculativeFailureDispositionV1::Released(_)
                ),
                failure == ModelQueueFailureV1::CurrentnessReleased
            );
        }

        for failure in [
            ModelQueueFailureV1::ReadbackReleased,
            ModelQueueFailureV1::ReadbackQuarantined,
        ] {
            let queue = ModelQueueV1::new([Some(failure)], [model_members(1, 0, None)]);
            let mut engine = Engine::<1>::new(8, 4, 32).unwrap();
            let (stage, disposition) =
                execute_initial_round_core::<ModelInitialQueueEffectsV1, _, 1>(
                    &mut engine,
                    ModelPreparedQueueV1::new(queue.clone()),
                    "retained bootstrap",
                )
                .unwrap_err();
            assert_eq!(
                stage,
                M1AuthenticatedSpeculativeBootstrapRoundStageV1::CompletionObservation
            );
            assert!(engine.is_faulted());
            assert_eq!(queue.snapshot().submits, 1);
            assert_eq!(queue.snapshot().destroys, 1);
            assert_eq!(
                matches!(
                    disposition,
                    M1AuthenticatedSpeculativeFailureDispositionV1::Released(_)
                ),
                failure == ModelQueueFailureV1::ReadbackReleased
            );
        }
    }

    #[test]
    fn model_s8_stops_and_cancels_every_live_lane_after_a_repeat() {
        use crate::{
            M1SpeculativeCancellationReasonV1, M1SpeculativeMemberStatusV1,
            M1SpeculativeTerminalReasonV1,
        };

        for stop_lane in 0..8 {
            let cancel_lane = (stop_lane + 1) % 8;
            let selected = selection(Qwen3PlanBucket::SpeculativeS8K4C8192);
            let queue = ModelQueueV1::new(
                [None, None],
                [model_members(8, 1, None), model_members(8, 2, Some(stop_lane))],
            );
            let coordinator = model_coordinator(selected, 8);
            let lineage = model_lineage(&coordinator);
            let roster = coordinator.active_roster();
            let binding = coordinator
                .bind_round(0, CompletionEpoch::new(40), &roster)
                .unwrap();
            let controls = roster
                .iter()
                .copied()
                .map(M1SpeculativeMemberControlV1::continuing)
                .collect::<Vec<_>>();
            let mut engine = Engine::<1>::new(8, 4, 32).unwrap();
            let (diagnostic, (coordinator, binding, controls, lineage)) =
                execute_initial_round_core::<ModelInitialQueueEffectsV1, _, 1>(
                    &mut engine,
                    ModelPreparedQueueV1::new(queue.clone()),
                    (coordinator, binding, controls, lineage),
                )
                .unwrap();
            let prepared = prepare_coordinator_round_core(
                &mut engine,
                coordinator,
                binding,
                diagnostic,
                controls,
                lineage,
            )
            .unwrap();
            let first = finish_model_round(&mut engine, prepared);
            let coordinator = first.coordinator;
            let lineage = first.lineage;
            let roster = coordinator.active_roster();
            let binding = coordinator
                .bind_round(1, CompletionEpoch::new(41), &roster)
                .unwrap();
            let controls = roster
                .iter()
                .enumerate()
                .map(|(lane, request)| {
                    if lane == cancel_lane {
                        M1SpeculativeMemberControlV1::cancelling(
                            *request,
                            M1SpeculativeCancellationReasonV1::Deadline,
                        )
                    } else {
                        M1SpeculativeMemberControlV1::continuing(*request)
                    }
                })
                .collect::<Vec<_>>();
            let (diagnostic, (coordinator, binding, controls, lineage)) =
                execute_round_core::<ModelRearmedQueueEffectsV1, _, 1>(
                    &mut engine,
                    ModelPreparedQueueV1::new(queue.clone()),
                    (coordinator, binding, controls, lineage),
                )
                .unwrap();
            let prepared = prepare_coordinator_round_core(
                &mut engine,
                coordinator,
                binding,
                diagnostic,
                controls,
                lineage,
            )
            .unwrap();
            let second = finish_model_round(&mut engine, prepared);
            assert_eq!(
                second.outcome.members()[stop_lane].status(),
                M1SpeculativeMemberStatusV1::Completed(
                    M1SpeculativeTerminalReasonV1::StopToken { token: 999 }
                )
            );
            assert_eq!(
                second.outcome.members()[cancel_lane].status(),
                M1SpeculativeMemberStatusV1::Cancelled(
                    M1SpeculativeCancellationReasonV1::Deadline
                )
            );
            assert_eq!(second.outcome.next_active_roster().len(), 6);
            assert_eq!(queue.snapshot().submits, 2);
            assert!(!engine.is_faulted());
        }
    }

    fn validated_role_inputs(
        selection: Qwen3PlanSelection,
        requests: &[RequestId],
        epoch: CompletionEpoch,
        anchors: &[ferric_spec::TokenId],
        committed: &[u32],
        future_override: Option<(usize, usize, ferric_spec::TokenId)>,
    ) -> ValidatedM1StepInputs {
        assert_eq!(requests.len(), anchors.len());
        assert_eq!(requests.len(), committed.len());
        let dimensions = selection
            .bucket
            .dimensions(selection.role, selection.mode)
            .expect("test selection must be finite");
        let capacity = dimensions.sequences as usize;
        let width = dimensions.active_tokens as usize;
        let mut lanes = Vec::with_capacity(capacity);
        let mut tokens = vec![0; capacity * width];
        let mut positions = vec![0; capacity * width];
        let mut active_lengths = vec![0; capacity];
        let mut context_lengths = vec![0; capacity];
        for lane in 0..capacity {
            lanes.push(
                requests.get(lane).copied().map(|request| {
                    StepPlan::new(request, epoch, Identity::new([73; 32]), selection)
                }),
            );
        }
        for lane in 0..requests.len() {
            let row_start = lane * width;
            tokens[row_start] = anchors[lane];
            for column in 0..width {
                positions[row_start + column] =
                    committed[lane] + u32::try_from(column).expect("finite row width fits u32");
            }
            active_lengths[lane] = dimensions.active_tokens;
            context_lengths[lane] = committed[lane];
        }
        if let Some((lane, column, token)) = future_override {
            tokens[lane * width + column] = token;
        }
        match validate_m1_step_inputs(M1StepInputCandidate::new(
            selection,
            lanes,
            tokens,
            positions,
            active_lengths,
            context_lengths,
        )) {
            M1StepInputValidationOutcome::Validated(inputs) => inputs,
            M1StepInputValidationOutcome::Rejected(failure) => {
                panic!("structural test input rejected: {:?}", failure.error())
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn speculative_kv_inputs(
        target_selection: Qwen3PlanSelection,
        draft_selection: Qwen3PlanSelection,
        requests: &[RequestId],
        epoch: CompletionEpoch,
        draft_anchors: &[ferric_spec::TokenId],
        target_anchors: &[ferric_spec::TokenId],
        draft_committed: &[u32],
        target_committed: &[u32],
        future_override: Option<(usize, usize, ferric_spec::TokenId)>,
    ) -> M1LongLivedQueueRearmKvInputsV1 {
        M1LongLivedQueueRearmKvInputsV1::speculative_round(
            validated_role_inputs(
                draft_selection,
                requests,
                epoch,
                draft_anchors,
                draft_committed,
                None,
            ),
            validated_role_inputs(
                target_selection,
                requests,
                epoch,
                target_anchors,
                target_committed,
                future_override,
            ),
            (0..requests.len()).map(|_| Vec::new()).collect(),
            (0..requests.len()).map(|_| Vec::new()).collect(),
        )
    }

    fn coordinator_for_members(
        selection: Qwen3PlanSelection,
        requests: &[RequestId],
        anchors: &[ferric_spec::TokenId],
        target_committed: &[u32],
        draft_committed: &[u32],
    ) -> crate::M1SpeculativeGenerationLoopV1 {
        let policy = crate::M1SpeculativeGenerationPolicyV1::new(32, &[999]).unwrap();
        let seeds: Vec<_> = requests
            .iter()
            .copied()
            .zip(anchors.iter().copied())
            .zip(target_committed.iter().copied())
            .zip(draft_committed.iter().copied())
            .map(|(((request, anchor), target), draft)| {
                crate::M1SpeculativeMemberSeedV1::new(request, anchor, target, draft, policy)
            })
            .collect();
        crate::M1SpeculativeGenerationLoopV1::new(selection, &seeds).unwrap()
    }

    fn causal_round_zero_fixture(
        selected: Qwen3PlanSelection,
        seeds: &[M1SpeculativeMemberSeedV1],
    ) -> (
        M1SpeculativeGenerationLoopV1,
        M1AuthenticatedSpeculativePhysicalLineageWitnessV1,
        M1AuthenticatedSpeculativeCausalLineageV1,
    ) {
        let mut coordinator = M1SpeculativeGenerationLoopV1::new(selected, seeds).unwrap();
        let initial_seeds = coordinator.bootstrap_seed_snapshot().unwrap();
        let identity = M1AuthenticatedSpeculativeLineageIdentityV1::fresh().unwrap();
        let physical = M1AuthenticatedSpeculativePhysicalLineageWitnessV1 {
            identity,
            coordinator_identity: coordinator.identity(),
            selection: selected,
            round: 0,
            epoch: CompletionEpoch::new(7),
            initial_seeds: initial_seeds.clone(),
        };
        let generated: Vec<_> = initial_seeds
            .iter()
            .map(|seed| (seed.request(), 0))
            .collect();
        let mut lineage = M1AuthenticatedSpeculativeCausalLineageV1 {
            logical: M1AuthenticatedSpeculativeLogicalLineageWitnessV1 { identity },
            coordinator_identity: coordinator.identity(),
            selection: selected,
            initial_seeds,
            generated: generated.into_boxed_slice(),
            completed_rounds: 0,
            last_epoch: CompletionEpoch::new(7),
        };
        let _ = coordinator
            .commit_test_causal_round(CompletionEpoch::new(7))
            .unwrap();
        assert!(refresh_causal_generated_counts(
            &mut lineage,
            &coordinator,
            CompletionEpoch::new(7),
        ));
        (coordinator, physical, lineage)
    }

    #[test]
    fn production_input_binding_rejects_hostile_role_rows_before_detach() {
        let engine = Engine::<1>::new(8, 4, 32).unwrap();
        let target = selection(Qwen3PlanBucket::SpeculativeS1K4C8192);
        let draft = speculative_draft_selection(target).unwrap();
        let epoch = CompletionEpoch::new(7);
        let requests = [RequestId::new(0, 1)];
        let anchors = [70];
        let target_committed = [10];
        let draft_committed = [11];
        let coordinator = coordinator_for_members(
            target,
            &requests,
            &anchors,
            &target_committed,
            &draft_committed,
        );
        let binding = coordinator.bind_round(0, epoch, &requests).unwrap();
        let exact = speculative_kv_inputs(
            target,
            draft,
            &requests,
            epoch,
            &anchors,
            &anchors,
            &draft_committed,
            &target_committed,
            None,
        );
        assert!(speculative_round_inputs_match_binding(&exact, &binding));

        let independent_target_anchor = speculative_kv_inputs(
            target,
            draft,
            &requests,
            epoch,
            &anchors,
            &[71],
            &draft_committed,
            &target_committed,
            None,
        );
        assert!(!speculative_round_inputs_match_binding(
            &independent_target_anchor,
            &binding,
        ));
        let nonzero_future = speculative_kv_inputs(
            target,
            draft,
            &requests,
            epoch,
            &anchors,
            &anchors,
            &draft_committed,
            &target_committed,
            Some((0, 1, 72)),
        );
        assert!(!speculative_round_inputs_match_binding(
            &nonzero_future,
            &binding,
        ));
        let wrong_cursor_and_position = speculative_kv_inputs(
            target,
            draft,
            &requests,
            epoch,
            &anchors,
            &anchors,
            &draft_committed,
            &[12],
            None,
        );
        assert!(!speculative_round_inputs_match_binding(
            &wrong_cursor_and_position,
            &binding,
        ));
        let wrong_epoch = speculative_kv_inputs(
            target,
            draft,
            &requests,
            CompletionEpoch::new(8),
            &anchors,
            &anchors,
            &draft_committed,
            &target_committed,
            None,
        );
        assert!(!speculative_round_inputs_match_binding(
            &wrong_epoch,
            &binding,
        ));
        let wrong_target = selection(Qwen3PlanBucket::SpeculativeS1K8C8192);
        let wrong_selection = speculative_kv_inputs(
            wrong_target,
            speculative_draft_selection(wrong_target).unwrap(),
            &requests,
            epoch,
            &anchors,
            &anchors,
            &draft_committed,
            &target_committed,
            None,
        );
        assert!(!speculative_round_inputs_match_binding(
            &wrong_selection,
            &binding,
        ));
        // These predicates execute before queue detachment or any engine call.
        assert!(!engine.is_faulted());
    }

    #[test]
    fn production_input_binding_rejects_s8_lane_swap_before_detach() {
        let engine = Engine::<1>::new(8, 4, 32).unwrap();
        let target = selection(Qwen3PlanBucket::SpeculativeS8K4C8192);
        let draft = speculative_draft_selection(target).unwrap();
        let epoch = CompletionEpoch::new(9);
        let requests = [RequestId::new(0, 1), RequestId::new(1, 1)];
        let anchors = [70, 80];
        let target_committed = [10, 20];
        let draft_committed = [11, 21];
        let coordinator = coordinator_for_members(
            target,
            &requests,
            &anchors,
            &target_committed,
            &draft_committed,
        );
        let binding = coordinator.bind_round(0, epoch, &requests).unwrap();
        let swapped = speculative_kv_inputs(
            target,
            draft,
            &[requests[1], requests[0]],
            epoch,
            &[anchors[1], anchors[0]],
            &[anchors[1], anchors[0]],
            &[draft_committed[1], draft_committed[0]],
            &[target_committed[1], target_committed[0]],
            None,
        );
        assert!(!speculative_round_inputs_match_binding(&swapped, &binding,));
        assert!(!engine.is_faulted());
    }

    #[test]
    fn causal_witness_joins_fresh_spec_zero_and_repeated_spec_one() {
        let seeds = [M1SpeculativeMemberSeedV1::new(
            RequestId::new(0, 1),
            70,
            10,
            10,
            crate::M1SpeculativeGenerationPolicyV1::new(32, &[999]).unwrap(),
        )];
        let (mut coordinator, physical, mut lineage) =
            causal_round_zero_fixture(selection(Qwen3PlanBucket::SpeculativeS1K4C8192), &seeds);
        assert!(validate_causal_lineage_join(
            &coordinator,
            &physical,
            &lineage,
            CompletionEpoch::new(7),
            0,
        ));

        let _ = coordinator
            .commit_test_causal_round(CompletionEpoch::new(8))
            .unwrap();
        assert!(refresh_causal_generated_counts(
            &mut lineage,
            &coordinator,
            CompletionEpoch::new(8),
        ));
        assert!(validate_causal_lineage_join(
            &coordinator,
            &physical,
            &lineage,
            CompletionEpoch::new(8),
            1,
        ));
        assert!(!validate_causal_lineage_join(
            &coordinator,
            &physical,
            &lineage,
            CompletionEpoch::new(7),
            1,
        ));
    }

    #[test]
    fn causal_witness_rejects_fresh_coordinator_policy_and_generated_splices() {
        let seeds = [M1SpeculativeMemberSeedV1::new(
            RequestId::new(0, 1),
            70,
            10,
            10,
            crate::M1SpeculativeGenerationPolicyV1::new(32, &[999]).unwrap(),
        )];
        let (coordinator, physical, mut lineage) =
            causal_round_zero_fixture(selection(Qwen3PlanBucket::SpeculativeS1K4C8192), &seeds);
        let mut fresh = M1SpeculativeGenerationLoopV1::new(lineage.selection, &seeds).unwrap();
        let _ = fresh
            .commit_test_causal_round(CompletionEpoch::new(7))
            .unwrap();
        assert!(!validate_causal_lineage_join(
            &fresh,
            &physical,
            &lineage,
            CompletionEpoch::new(7),
            0,
        ));

        lineage.initial_seeds[0] = M1SpeculativeMemberSeedV1::new(
            RequestId::new(0, 1),
            70,
            10,
            10,
            crate::M1SpeculativeGenerationPolicyV1::new(32, &[998]).unwrap(),
        );
        assert!(!validate_causal_lineage_join(
            &coordinator,
            &physical,
            &lineage,
            CompletionEpoch::new(7),
            0,
        ));
        lineage.initial_seeds[0] = seeds[0];
        lineage.generated[0].1 += 1;
        assert!(!validate_causal_lineage_join(
            &coordinator,
            &physical,
            &lineage,
            CompletionEpoch::new(7),
            0,
        ));
    }

    #[test]
    fn causal_witness_rejects_reorder_epoch_and_history_splices() {
        let policy = crate::M1SpeculativeGenerationPolicyV1::new(32, &[999]).unwrap();
        let seeds = [
            M1SpeculativeMemberSeedV1::new(RequestId::new(0, 1), 70, 10, 10, policy),
            M1SpeculativeMemberSeedV1::new(RequestId::new(1, 1), 80, 20, 20, policy),
        ];
        let (coordinator, physical, mut lineage) =
            causal_round_zero_fixture(selection(Qwen3PlanBucket::SpeculativeS8K4C8192), &seeds);
        lineage.generated.swap(0, 1);
        assert!(!validate_causal_lineage_join(
            &coordinator,
            &physical,
            &lineage,
            CompletionEpoch::new(7),
            0,
        ));
        lineage.generated.swap(0, 1);
        assert!(!validate_causal_lineage_join(
            &coordinator,
            &physical,
            &lineage,
            CompletionEpoch::new(8),
            0,
        ));
        assert!(!validate_causal_lineage_join(
            &coordinator,
            &physical,
            &lineage,
            CompletionEpoch::new(7),
            1,
        ));
    }

    #[test]
    fn production_entry_profile_gate_covers_all_four_declared_shapes() {
        let cases = [
            (
                Qwen3PlanBucket::SpeculativeS1K4C8192,
                M1PhysicalFixedBatchShapeV1::SpeculativeK4,
                1_usize,
            ),
            (
                Qwen3PlanBucket::SpeculativeS8K4C8192,
                M1PhysicalFixedBatchShapeV1::SpeculativeK4,
                8,
            ),
            (
                Qwen3PlanBucket::SpeculativeS1K8C8192,
                M1PhysicalFixedBatchShapeV1::SpeculativeK8,
                1,
            ),
            (
                Qwen3PlanBucket::SpeculativeS1K16C8192,
                M1PhysicalFixedBatchShapeV1::SpeculativeK16,
                1,
            ),
        ];
        for (bucket, expected, member_count) in cases {
            let target = selection(bucket);
            assert!(production_entry_profile_matches(target, expected));
            let draft = speculative_draft_selection(target).unwrap();
            let requests: Vec<_> = (0..member_count)
                .map(|lane| RequestId::new(u32::try_from(lane).unwrap(), 1))
                .collect();
            let anchors: Vec<_> = (0..member_count)
                .map(|lane| 70 + u32::try_from(lane).unwrap())
                .collect();
            let target_committed = vec![10; member_count];
            let draft_committed = vec![10; member_count];
            let coordinator = coordinator_for_members(
                target,
                &requests,
                &anchors,
                &target_committed,
                &draft_committed,
            );
            let binding = coordinator
                .bind_round(0, CompletionEpoch::new(7), &requests)
                .unwrap();
            let inputs = speculative_kv_inputs(
                target,
                draft,
                &requests,
                CompletionEpoch::new(7),
                &anchors,
                &anchors,
                &draft_committed,
                &target_committed,
                None,
            );
            assert!(speculative_round_inputs_match_binding(&inputs, &binding));
            assert_eq!(binding.members().len(), member_count);
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
    fn public_coordinator_profiles_repeat_then_stop_and_cancel_all_live_lanes() {
        use crate::speculative_generation_loop::CheckedMemberObservationV1;
        use crate::{
            CheckedCompletionSemantics, M1SpeculativeCancellationReasonV1,
            M1SpeculativeMemberStatusV1, M1SpeculativeTerminalReasonV1, M1SpeculativeTokenBlockV1,
        };

        let cases = [
            (Qwen3PlanBucket::SpeculativeS1K4C8192, 1_usize),
            (Qwen3PlanBucket::SpeculativeS8K4C8192, 8),
            (Qwen3PlanBucket::SpeculativeS1K8C8192, 1),
            (Qwen3PlanBucket::SpeculativeS1K16C8192, 1),
        ];
        for (bucket, member_count) in cases {
            let target = selection(bucket);
            let policy = crate::M1SpeculativeGenerationPolicyV1::new(32, &[999]).unwrap();
            let seeds: Vec<_> = (0..member_count)
                .map(|lane| {
                    crate::M1SpeculativeMemberSeedV1::new(
                        RequestId::new(u32::try_from(lane).unwrap(), 1),
                        70 + u32::try_from(lane).unwrap(),
                        10,
                        10,
                        policy,
                    )
                })
                .collect();
            let mut coordinator = M1SpeculativeGenerationLoopV1::new(target, &seeds).unwrap();
            let roster = coordinator.active_roster();
            assert_eq!(roster.len(), member_count);

            let first = coordinator
                .bind_round(0, CompletionEpoch::new(40), &roster)
                .unwrap();
            let first_observations: Vec<_> = roster
                .iter()
                .copied()
                .map(|request| CheckedMemberObservationV1 {
                    request,
                    semantics: CheckedCompletionSemantics::Speculative {
                        accepted_draft_tokens: 0,
                        correction_or_bonus: 777,
                    },
                    emitted: M1SpeculativeTokenBlockV1::from_slice(&[777]).unwrap(),
                })
                .collect();
            let first_controls: Vec<_> = roster
                .iter()
                .copied()
                .map(M1SpeculativeMemberControlV1::continuing)
                .collect();
            let first = coordinator
                .preflight_observed_round(
                    first,
                    target,
                    CompletionEpoch::new(40),
                    &first_observations,
                    &first_controls,
                )
                .unwrap();
            let first = coordinator.commit_preflighted_round(first).unwrap();
            assert_eq!(first.next_active_roster(), roster.as_slice());

            let second_roster = coordinator.active_roster();
            let second = coordinator
                .bind_round(1, CompletionEpoch::new(41), &second_roster)
                .unwrap();
            let second_observations: Vec<_> = second_roster
                .iter()
                .enumerate()
                .map(|(lane, request)| {
                    let token = if lane == 0 { 999 } else { 778 };
                    CheckedMemberObservationV1 {
                        request: *request,
                        semantics: CheckedCompletionSemantics::Speculative {
                            accepted_draft_tokens: 0,
                            correction_or_bonus: token,
                        },
                        emitted: M1SpeculativeTokenBlockV1::from_slice(&[token]).unwrap(),
                    }
                })
                .collect();
            let second_controls: Vec<_> = second_roster
                .iter()
                .enumerate()
                .map(|(lane, request)| {
                    if member_count == 8 && lane == 1 {
                        M1SpeculativeMemberControlV1::cancelling(
                            *request,
                            M1SpeculativeCancellationReasonV1::Deadline,
                        )
                    } else {
                        M1SpeculativeMemberControlV1::continuing(*request)
                    }
                })
                .collect();
            let second = coordinator
                .preflight_observed_round(
                    second,
                    target,
                    CompletionEpoch::new(41),
                    &second_observations,
                    &second_controls,
                )
                .unwrap();
            let second = coordinator.commit_preflighted_round(second).unwrap();
            assert_eq!(
                second.members()[0].status(),
                M1SpeculativeMemberStatusV1::Completed(M1SpeculativeTerminalReasonV1::StopToken {
                    token: 999
                })
            );
            if member_count == 8 {
                assert_eq!(
                    second.members()[1].status(),
                    M1SpeculativeMemberStatusV1::Cancelled(
                        M1SpeculativeCancellationReasonV1::Deadline
                    )
                );
                assert_eq!(second.next_active_roster().len(), 6);
            } else {
                assert!(second.next_active_roster().is_empty());
            }
        }
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
        .unwrap_err();
        assert!(engine.is_faulted());
        assert_eq!(
            cleanup.stage(),
            M1AuthenticatedSpeculativePhysicalRoundStageV1::CoordinatorPreflight,
        );
        assert!(cleanup.retains_logical_custody());
        assert!(matches!(
            cleanup.source(),
            M1AuthenticatedSpeculativePhysicalRoundTeardownFailureSourceV1::TerminalQuarantine(
                M1AuthenticatedSpeculativePhysicalRoundTerminalSourceV1::Coordinator(
                    crate::M1SpeculativeGenerationLoopErrorV1::NoActiveMembers,
                ),
            ),
        ));
    }

    #[test]
    fn public_failure_dispositions_redact_all_retained_authority() {
        let released = M1AuthenticatedSpeculativeCleanReleaseV1 {
            retained: Box::new("secret released authority"),
        };
        assert!(released.queue_released());
        assert!(released.engine_quarantined());
        assert!(released.retains_custody());
        let released_debug = format!("{released:?}");
        assert!(!released_debug.contains("secret released authority"));

        let quarantined = M1AuthenticatedSpeculativeTerminalQuarantineV1 {
            retained: Box::new("secret quarantined authority"),
        };
        assert!(quarantined.engine_quarantined());
        assert!(quarantined.retains_custody());
        let quarantined_debug = format!("{quarantined:?}");
        assert!(!quarantined_debug.contains("secret quarantined authority"));
    }
}
