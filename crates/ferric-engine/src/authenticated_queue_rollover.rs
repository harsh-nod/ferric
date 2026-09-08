//! Authenticated paired-prefill to finite-speculative queue rollover.
//!
//! The transition consumes the released authenticated prefill owner. It does
//! not accept raw queues, catalog indices, currentness claims, or checked
//! output supplied independently by the normal completion pipeline.

use core::fmt;

use arrayvec::ArrayVec;
use fe2o3_host::{
    AuthenticatedQuarantinedServiceQueueV1, AuthenticatedServiceQueueReleaseFailureV1,
    AuthenticatedServiceQueueReleaseV1, AuthenticatedServiceQueueRetainedRolloverFailureV1,
    AuthenticatedServiceQueueUnboundSessionV1,
};
use fe2o3_service_host::{DeviceWorkspaceRoleV1, ServiceDeviceDispatchRangeV1};
use ferric_spec::{
    completion::CompletionEpoch, scheduling::RequestState, PhysicalKvLifecycle, Qwen3ExecutionMode,
    Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection, ValidatedM1StepInputs, M1_KV_PAGE_TOKENS,
    M1_KV_PHYSICAL_PAGE_SLOTS,
};

use crate::authenticated_speculative_executor::{
    speculative_validated_pair_matches_binding, upgrade_m1_authenticated_speculative_lineage_v1,
    M1AuthenticatedSpeculativeLogicalLineageWitnessV1,
    M1AuthenticatedSpeculativePhysicalLineageWitnessV1,
};
use crate::authenticated_target_rollover_phase_custody::{
    advance_m1_authenticated_target_rollover_prepared_custody_v1,
    advance_m1_authenticated_target_rollover_reselected_custody_v1,
    begin_m1_authenticated_target_rollover_scheduled_custody_v1,
    establish_m1_authenticated_target_rollover_submit_entry_custody_v1,
    M1AuthenticatedTargetRolloverPreparedCustodyV1,
    M1AuthenticatedTargetRolloverReselectedCustodyV1,
};
use crate::m1_serving_registry::{
    admit_m1_production_rollover_transition_v1, admit_m1_target_decode_rollover_transition_v1,
};
use crate::{
    ActiveDeviceKvCache, AddresslessM1PhysicalBufferRecipeV1, BoundM1StepWorkspaceSubleases,
    DeviceKvCacheProjection, DeviceKvPageLease, Engine, LogicalRunnerDeclaration,
    M1AuthenticatedPhysicalQueuePhaseCaseV1, M1AuthenticatedPhysicalQueueSessionV1,
    M1AuthenticatedPhysicalQueueSubmitFailureV1,
    M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1,
    M1AuthenticatedPhysicalReadbackQueueOperationFailureV1, M1AuthenticatedRearmedPublishedQueueV1,
    M1AuthenticatedReleasedCompletedStepV1, M1FiniteSpeculativeQueueRolloverKvInputsV1,
    M1FullStepKvWorkspaceTablesV1, M1FullStepWorkspaceImagesV1, M1FullStepWorkspaceInputKind,
    M1FullStepWorkspacePlans, M1FullStepWorkspaceRole, M1FullStepWorkspaceSubleaseOwners,
    M1InitializedWorkspaceSlotV1, M1PhysicalFixedBatchShapeV1, M1PhysicalQueueBatchCustodyV1,
    M1PhysicalRunnerRecipeOutcomeV1, M1PreparedScheduledWorkspaceImagesV1,
    M1QueueRolloverObservationV1, M1ReleasedDeviceKvMemberV1, M1ScheduledDispatchV1,
    M1ServingBatchPlanV1, M1ServingPlanV1, M1ServingQueueActionV1,
    M1ServingQueuedPairedPrefillNewWindowV1, M1ServingRolloverReasonV1,
    M1SpeculativeGenerationLoopV1, M1SpeculativeGenerationPolicyV1, M1StepDispatchIntent,
    M1_DRAFT_STEP_WORKSPACE_SUBLEASE_COUNT_V1, M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1,
    M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1, M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1,
    M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1, M1_TARGET_STEP_WORKSPACE_SUBLEASE_COUNT_V1,
};

/// One request and immutable generation policy fixed before paired-prefill
/// publication.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1AuthenticatedSpeculativeRolloverMemberIntentV1 {
    request: ferric_spec::RequestId,
    policy: M1SpeculativeGenerationPolicyV1,
}

impl M1AuthenticatedSpeculativeRolloverMemberIntentV1 {
    #[must_use]
    pub const fn new(
        request: ferric_spec::RequestId,
        policy: M1SpeculativeGenerationPolicyV1,
    ) -> Self {
        Self { request, policy }
    }

    #[must_use]
    pub const fn request(self) -> ferric_spec::RequestId {
        self.request
    }

    #[must_use]
    pub const fn policy(self) -> M1SpeculativeGenerationPolicyV1 {
        self.policy
    }
}

/// Private physical half of the prefill-to-speculative causal join.
#[derive(Debug)]
pub(crate) struct M1AuthenticatedSpeculativeRolloverPhysicalIntentV1 {
    identity:
        crate::authenticated_speculative_executor::M1AuthenticatedSpeculativeLineageIdentityV1,
    prefill_selection: ferric_spec::Qwen3PlanSelection,
    speculative_selection: ferric_spec::Qwen3PlanSelection,
    prefill_epoch: CompletionEpoch,
    members: Box<[M1AuthenticatedSpeculativeRolloverMemberIntentV1]>,
}

/// Caller-held logical half of an intent carried through authenticated prefill.
#[must_use = "rollover intent must be joined to its authenticated prefill release"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativeRolloverIntentV1 {
    identity:
        crate::authenticated_speculative_executor::M1AuthenticatedSpeculativeLineageIdentityV1,
    prefill_selection: ferric_spec::Qwen3PlanSelection,
    speculative_selection: ferric_spec::Qwen3PlanSelection,
    prefill_epoch: CompletionEpoch,
    members: Box<[M1AuthenticatedSpeculativeRolloverMemberIntentV1]>,
}

/// Copy-only facts from joining the logical and physical halves of the exact
/// paired-prefill rollover intent. This carries no queue or cache authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct M1AuthenticatedPrefillRegistryIntentFactsV1 {
    pub(crate) request: ferric_spec::RequestId,
    pub(crate) prefill_selection: ferric_spec::Qwen3PlanSelection,
    pub(crate) speculative_selection: ferric_spec::Qwen3PlanSelection,
    pub(crate) prefill_epoch: CompletionEpoch,
}

pub(crate) fn join_m1_authenticated_prefill_registry_intent_v1(
    physical: &M1AuthenticatedSpeculativeRolloverPhysicalIntentV1,
    logical: &M1AuthenticatedSpeculativeRolloverIntentV1,
) -> Option<M1AuthenticatedPrefillRegistryIntentFactsV1> {
    let [physical_member] = physical.members.as_ref() else {
        return None;
    };
    let [logical_member] = logical.members.as_ref() else {
        return None;
    };
    if physical.identity != logical.identity
        || physical.prefill_selection != logical.prefill_selection
        || physical.speculative_selection != logical.speculative_selection
        || physical.prefill_epoch != logical.prefill_epoch
        || physical_member != logical_member
    {
        return None;
    }
    Some(M1AuthenticatedPrefillRegistryIntentFactsV1 {
        request: physical_member.request(),
        prefill_selection: physical.prefill_selection,
        speculative_selection: physical.speculative_selection,
        prefill_epoch: physical.prefill_epoch,
    })
}

/// Validated logical inputs for the authenticated successor of a fresh window.
///
/// This owner deliberately contains no KV page lease. Its exact missing tail
/// pages are minted only after the authenticated predecessor queue is detached,
/// from the page-generation ledger retained by that queue.
#[must_use = "successor inputs must enter their authenticated new-window join"]
#[derive(Debug, Eq, PartialEq)]
pub struct M1AuthenticatedSpeculativeNewWindowSuccessorInputsV1 {
    draft_decode: ValidatedM1StepInputs,
    target_speculative: ValidatedM1StepInputs,
}

impl M1AuthenticatedSpeculativeNewWindowSuccessorInputsV1 {
    pub const fn new(
        draft_decode: ValidatedM1StepInputs,
        target_speculative: ValidatedM1StepInputs,
    ) -> Self {
        Self {
            draft_decode,
            target_speculative,
        }
    }

    const fn validated_inputs(&self) -> (&ValidatedM1StepInputs, &ValidatedM1StepInputs) {
        (&self.draft_decode, &self.target_speculative)
    }

    fn into_raw(
        self,
        draft_page_leases: Vec<Vec<DeviceKvPageLease>>,
        target_page_leases: Vec<Vec<DeviceKvPageLease>>,
    ) -> M1FiniteSpeculativeQueueRolloverKvInputsV1 {
        M1FiniteSpeculativeQueueRolloverKvInputsV1::from_lane_leases(
            self.draft_decode,
            self.target_speculative,
            draft_page_leases,
            target_page_leases,
        )
    }
}

#[derive(Debug)]
enum M1AuthenticatedSpeculativeRolloverInputsV1 {
    Preleased(M1FiniteSpeculativeQueueRolloverKvInputsV1),
    NewWindow(M1AuthenticatedSpeculativeNewWindowSuccessorInputsV1),
}

impl M1AuthenticatedSpeculativeRolloverInputsV1 {
    const fn validated_inputs(&self) -> (&ValidatedM1StepInputs, &ValidatedM1StepInputs) {
        match self {
            Self::Preleased(inputs) => inputs.validated_inputs(),
            Self::NewWindow(inputs) => inputs.validated_inputs(),
        }
    }
}

/// Prepared paired-prefill work plus its unique logical rollover intent.
#[must_use = "prepared prefill and logical rollover intent remain linear"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativeRolloverIntentPreparedV1 {
    prepared: M1PreparedScheduledWorkspaceImagesV1,
    intent: M1AuthenticatedSpeculativeRolloverIntentV1,
}

impl M1AuthenticatedSpeculativeRolloverIntentPreparedV1 {
    #[must_use = "both prefill owners remain linear"]
    pub fn into_parts(
        self,
    ) -> (
        M1PreparedScheduledWorkspaceImagesV1,
        M1AuthenticatedSpeculativeRolloverIntentV1,
    ) {
        (self.prepared, self.intent)
    }
}

/// Stable pure rejection while binding a speculative intent to paired prefill.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedSpeculativeRolloverIntentErrorV1 {
    PairedPrefill,
    Successor,
    Roster,
    IdentityExhausted,
    Attachment,
}

/// Intent-binding failure with every caller owner unchanged.
#[must_use = "rejected prepared prefill, successor, and policies remain retained"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativeRolloverIntentFailureV1 {
    error: M1AuthenticatedSpeculativeRolloverIntentErrorV1,
    prepared: M1PreparedScheduledWorkspaceImagesV1,
    successor: M1ServingPlanV1,
    members: Vec<M1AuthenticatedSpeculativeRolloverMemberIntentV1>,
}

impl M1AuthenticatedSpeculativeRolloverIntentFailureV1 {
    #[must_use]
    pub const fn error(&self) -> M1AuthenticatedSpeculativeRolloverIntentErrorV1 {
        self.error
    }

    #[must_use = "all rejected intent inputs remain linear"]
    pub fn into_parts(
        self,
    ) -> (
        M1PreparedScheduledWorkspaceImagesV1,
        M1ServingPlanV1,
        Vec<M1AuthenticatedSpeculativeRolloverMemberIntentV1>,
    ) {
        (self.prepared, self.successor, self.members)
    }
}

/// Stable pre-detachment rejection for authenticated prefill rollover.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedSpeculativeRolloverScheduleErrorV1 {
    Action,
    Transition,
    EngineFaulted,
    Epoch,
    Roster,
    QueueShape,
    QueueSelection,
    OutputReserve,
    MemberCustody { lane: usize },
    RequestNotReady { lane: usize },
    Coordinator,
    CoordinatorSeed { lane: usize },
    CommittedHistory { lane: usize },
    Inputs,
    SuccessorPages,
    Lineage,
    Detach,
    ExactDispatch,
    CacheReselection { lane: usize },
}

#[allow(dead_code)] // Every field is intentionally retained behind opaque terminal custody.
#[derive(Debug)]
enum M1AuthenticatedSpeculativeSuccessorPageFailureV1 {
    Materialization {
        source: Box<M1AuthenticatedSpeculativeTailPageMaterializationFailureV1>,
        inputs: M1AuthenticatedSpeculativeNewWindowSuccessorInputsV1,
    },
    MemberRoster {
        inputs: M1AuthenticatedSpeculativeNewWindowSuccessorInputsV1,
    },
}

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) enum M1AuthenticatedSpeculativeTailPageMaterializationFailureV1 {
    Span,
    Admission {
        source: crate::M1DeviceKvArenaLeaseErrorV1,
    },
    Commit {
        source: Box<crate::device_cache::M1AuthenticatedNewWindowPageSetCommitFailureV1>,
    },
    CommittedRoster {
        spans: Vec<M1AuthenticatedSpeculativeSuccessorLanePageSpansV1>,
        draft_page_leases: Vec<Vec<DeviceKvPageLease>>,
        target_page_leases: Vec<Vec<DeviceKvPageLease>>,
        remaining: std::vec::IntoIter<DeviceKvPageLease>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct M1AuthenticatedSpeculativeSuccessorPageSpanV1 {
    first_page: u32,
    page_count: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct M1AuthenticatedSpeculativeSuccessorLanePageSpansV1 {
    request: ferric_spec::RequestId,
    draft: M1AuthenticatedSpeculativeSuccessorPageSpanV1,
    target: M1AuthenticatedSpeculativeSuccessorPageSpanV1,
}

#[derive(Debug)]
pub(crate) struct M1AuthenticatedSpeculativeRolloverResidueV1 {
    pub(crate) checked: crate::M1CheckedCompletionOutputV1,
    pub(crate) logical_accepted_counts: Box<[u32]>,
    pub(crate) externally_published_counts: Box<[u32]>,
    pub(crate) release_counts: Box<[crate::M1CompletedKvPageReleaseCountsV1]>,
    pub(crate) completed_members: usize,
    pub(crate) total_released: usize,
    pub(crate) terminal: Vec<crate::M1ReleasedTerminalDeviceKvMemberV1>,
    pub(crate) history: crate::m1_queue_rearm::M1RearmRoundHistoryV1,
}

#[derive(Debug)]
pub(crate) struct M1AuthenticatedSpeculativeRolloverLogicalV1 {
    pub(crate) coordinator: M1SpeculativeGenerationLoopV1,
    pub(crate) epoch: CompletionEpoch,
    pub(crate) lineage: M1AuthenticatedSpeculativeLogicalLineageWitnessV1,
    pub(crate) prior_windows: Vec<crate::authenticated_speculative_executor::M1AuthenticatedSpeculativeCompletedWindowHistoryV1>,
    pub(crate) frozen_queue_wait_timeout: Option<crate::M1QueueWaitTimeoutV1>,
}

#[derive(Debug)]
struct M1AuthenticatedSpeculativePriorWindowContinuationV1 {
    terminal: Vec<crate::M1ReleasedTerminalDeviceKvMemberV1>,
    history: crate::m1_queue_rearm::M1RearmRoundHistoryV1,
    prior_windows: Vec<crate::authenticated_speculative_executor::M1AuthenticatedSpeculativeCompletedWindowHistoryV1>,
    frozen_queue_wait_timeout: Option<crate::M1QueueWaitTimeoutV1>,
}

impl M1AuthenticatedSpeculativePriorWindowContinuationV1 {
    const fn initial() -> Self {
        Self {
            terminal: Vec::new(),
            history: crate::m1_queue_rearm::M1RearmRoundHistoryV1::Empty,
            prior_windows: Vec::new(),
            frozen_queue_wait_timeout: None,
        }
    }
}

/// Rollover scheduling rejection before detachment or terminal failure after
/// detachment began.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedSpeculativeRolloverScheduleFailureV1;
/// fn recover_round(failure: M1AuthenticatedSpeculativeRolloverScheduleFailureV1) {
///     let _round = failure.into_released_round();
/// }
/// ```
#[must_use = "rollover scheduling retry or terminal custody remains retained"]
pub enum M1AuthenticatedSpeculativeRolloverScheduleFailureV1 {
    /// Exact unchanged owners retained after a pure pre-detach rejection.
    PreDetach {
        error: M1AuthenticatedSpeculativeRolloverScheduleErrorV1,
        retry: Box<M1AuthenticatedSpeculativeRolloverSchedulePreDetachRetryV1>,
    },
    /// Detach-or-later failure, or an already-faulted Engine closed explicitly.
    Terminal {
        error: M1AuthenticatedSpeculativeRolloverScheduleErrorV1,
        disposition: crate::M1AuthenticatedSpeculativeFailureDispositionV1,
    },
}

impl M1AuthenticatedSpeculativeRolloverScheduleFailureV1 {
    #[must_use]
    pub const fn error(&self) -> M1AuthenticatedSpeculativeRolloverScheduleErrorV1 {
        match self {
            Self::PreDetach { error, .. } | Self::Terminal { error, .. } => *error,
        }
    }

    #[must_use]
    pub const fn is_pre_detach_retry(&self) -> bool {
        matches!(self, Self::PreDetach { .. })
    }

    #[must_use = "the terminal disposition must remain observed when present"]
    pub const fn disposition(
        &self,
    ) -> Option<&crate::M1AuthenticatedSpeculativeFailureDispositionV1> {
        match self {
            Self::PreDetach { .. } => None,
            Self::Terminal { disposition, .. } => Some(disposition),
        }
    }
}

impl fmt::Debug for M1AuthenticatedSpeculativeRolloverScheduleFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedSpeculativeRolloverScheduleFailureV1")
            .field("error", &self.error())
            .field("pre_detach_retry", &self.is_pre_detach_retry())
            .field("terminal_disposition", &self.disposition())
            .finish()
    }
}

/// Opaque exact owners retained by a pure rollover scheduling preflight
/// rejection. The released queue and coordinator cannot be separated.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedSpeculativeRolloverSchedulePreDetachRetryV1;
/// fn extract(retry: M1AuthenticatedSpeculativeRolloverSchedulePreDetachRetryV1) {
///     let _released_queue_or_coordinator = retry.into_parts();
/// }
/// ```
#[must_use = "pre-detach rollover retry custody remains linear"]
pub struct M1AuthenticatedSpeculativeRolloverSchedulePreDetachRetryV1 {
    released: Box<M1AuthenticatedReleasedCompletedStepV1>,
    intent: Box<M1AuthenticatedSpeculativeRolloverIntentV1>,
    coordinator: Box<M1SpeculativeGenerationLoopV1>,
    inputs: Box<M1AuthenticatedSpeculativeRolloverInputsV1>,
    recipe_plans: M1FullStepWorkspacePlans,
    preparation_plans: M1FullStepWorkspacePlans,
    prior_window_continuation: Box<M1AuthenticatedSpeculativePriorWindowContinuationV1>,
}

impl fmt::Debug for M1AuthenticatedSpeculativeRolloverSchedulePreDetachRetryV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedSpeculativeRolloverSchedulePreDetachRetryV1")
            .field("retains_exact_inputs", &true)
            .finish()
    }
}

impl M1AuthenticatedSpeculativeRolloverSchedulePreDetachRetryV1 {
    /// Retries the exact unchanged released owner and causal inputs against a
    /// caller-retained serving batch.
    ///
    /// # Errors
    ///
    /// Returns renewed pre-detach custody or a terminal detach-or-later failure.
    pub fn retry<const C: usize>(
        self,
        engine: &mut Engine<C>,
        batch: &M1ServingBatchPlanV1,
    ) -> Result<
        M1AuthenticatedScheduledSpeculativeRolloverV1,
        M1AuthenticatedSpeculativeRolloverScheduleFailureV1,
    > {
        schedule_m1_authenticated_speculative_rollover_pending_v1(
            engine,
            *self.released,
            batch,
            *self.intent,
            *self.coordinator,
            *self.inputs,
            self.recipe_plans,
            self.preparation_plans,
            *self.prior_window_continuation,
        )
        .map_err(|failure| close_pending_schedule_failure(engine, failure))
    }

    #[must_use]
    pub const fn retains_exact_inputs(&self) -> bool {
        true
    }

    /// Cancels this retry owner, faults the scheduler, and destroys its queue.
    ///
    /// The result exposes only clean release evidence or opaque quarantine;
    /// none of the exact retry inputs can be recovered or resubmitted.
    #[must_use = "cancelled rollover custody remains retained"]
    pub fn cancel_and_close<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> crate::M1AuthenticatedSpeculativeFailureDispositionV1 {
        use crate::authenticated_speculative_executor::{
            quarantined_disposition, released_disposition,
        };

        let Self {
            released,
            intent,
            coordinator,
            inputs,
            recipe_plans,
            preparation_plans,
            prior_window_continuation,
        } = self;
        let retained = (
            intent,
            coordinator,
            inputs,
            recipe_plans,
            preparation_plans,
            prior_window_continuation,
        );
        match released.destroy_queue_and_retain_step(engine) {
            Ok(released) => released_disposition((released, retained)),
            Err(quarantined) => quarantined_disposition((quarantined, retained)),
        }
    }
}

/// Internal scheduling custody pending mandatory queue closure.
#[derive(Debug)]
enum PendingM1AuthenticatedSpeculativeRolloverScheduleFailureV1 {
    Rejected {
        error: M1AuthenticatedSpeculativeRolloverScheduleErrorV1,
        released: Box<M1AuthenticatedReleasedCompletedStepV1>,
        intent: Box<M1AuthenticatedSpeculativeRolloverIntentV1>,
        coordinator: Box<M1SpeculativeGenerationLoopV1>,
        inputs: Box<M1AuthenticatedSpeculativeRolloverInputsV1>,
        recipe_plans: M1FullStepWorkspacePlans,
        preparation_plans: M1FullStepWorkspacePlans,
        prior_window_continuation: Box<M1AuthenticatedSpeculativePriorWindowContinuationV1>,
    },
    Detach {
        error: M1AuthenticatedSpeculativeRolloverScheduleErrorV1,
        source: Box<M1AuthenticatedPhysicalReadbackQueueOperationFailureV1>,
        retained: Box<dyn fmt::Debug>,
    },
    Detached(Box<M1AuthenticatedSpeculativeRolloverDetachedFailureV1>),
}

/// Authenticated detached queue plus all non-queue scheduling custody.
#[must_use = "detached rollover failure must destroy the queue or remain quarantined"]
#[derive(Debug)]
struct M1AuthenticatedSpeculativeRolloverDetachedFailureV1 {
    error: M1AuthenticatedSpeculativeRolloverScheduleErrorV1,
    queue: M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1,
    retained: Box<dyn fmt::Debug>,
}

/// Clean teardown of a detached authenticated rollover failure.
#[must_use = "released program sets and Ferric custody remain retained"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativeRolloverTeardownSuccessV1 {
    release: AuthenticatedServiceQueueReleaseV1,
    retained: Box<dyn fmt::Debug>,
}

/// Opaque terminal teardown quarantine retaining all Ferric custody.
#[must_use = "lower quarantine and Ferric custody remain retained"]
pub struct M1AuthenticatedSpeculativeRolloverTeardownFailureV1 {
    retained: Box<dyn fmt::Debug>,
}

#[allow(clippy::missing_fields_in_debug)]
impl fmt::Debug for M1AuthenticatedSpeculativeRolloverTeardownFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedSpeculativeRolloverTeardownFailureV1")
            .field("engine_quarantined", &true)
            .field("custody_sealed", &true)
            .finish()
    }
}

impl M1AuthenticatedSpeculativeRolloverTeardownSuccessV1 {
    pub const fn release(&self) -> &AuthenticatedServiceQueueReleaseV1 {
        &self.release
    }

    #[must_use]
    pub fn retains_ferric_custody(&self) -> bool {
        let _ = &self.retained;
        true
    }
}

impl M1AuthenticatedSpeculativeRolloverTeardownFailureV1 {
    #[must_use]
    pub fn retains_ferric_custody(&self) -> bool {
        let _ = &self.retained;
        true
    }

    #[must_use]
    pub const fn engine_quarantined(&self) -> bool {
        true
    }
}

impl M1AuthenticatedSpeculativeRolloverDetachedFailureV1 {
    /// Faults the scheduler and destroys the detached authenticated queue.
    ///
    /// # Errors
    ///
    /// Returns the lower terminal release quarantine with all Ferric custody.
    fn destroy_queue_and_retain_custody<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1AuthenticatedSpeculativeRolloverTeardownSuccessV1,
        Box<M1AuthenticatedSpeculativeRolloverTeardownFailureV1>,
    > {
        engine.quarantine_m1_queue_rearm_failure();
        let (shape, lower, witness, operations, custody) = self.queue.into_rearm_parts();
        let retained = Box::new((
            self.error,
            shape,
            witness,
            operations,
            custody,
            self.retained,
        ));
        match lower.destroy_and_release() {
            Ok(release) => {
                Ok(M1AuthenticatedSpeculativeRolloverTeardownSuccessV1 { release, retained })
            }
            Err(source) => Err(Box::new(
                M1AuthenticatedSpeculativeRolloverTeardownFailureV1 {
                    retained: Box::new((source, retained)),
                },
            )),
        }
    }
}

fn close_pending_schedule_failure<const C: usize>(
    engine: &mut Engine<C>,
    pending: PendingM1AuthenticatedSpeculativeRolloverScheduleFailureV1,
) -> M1AuthenticatedSpeculativeRolloverScheduleFailureV1 {
    use crate::authenticated_speculative_executor::{
        quarantined_disposition, released_disposition,
    };

    match pending {
        PendingM1AuthenticatedSpeculativeRolloverScheduleFailureV1::Rejected {
            error,
            released,
            intent,
            coordinator,
            inputs,
            recipe_plans,
            preparation_plans,
            prior_window_continuation,
        } => {
            if error == M1AuthenticatedSpeculativeRolloverScheduleErrorV1::EngineFaulted {
                engine.quarantine_m1_queue_rearm_failure();
                let logical = (
                    intent,
                    coordinator,
                    inputs,
                    recipe_plans,
                    preparation_plans,
                    prior_window_continuation,
                );
                let disposition = match released.destroy_queue_and_retain_step(engine) {
                    Ok(released) => released_disposition((released, logical)),
                    Err(quarantined) => quarantined_disposition((quarantined, logical)),
                };
                M1AuthenticatedSpeculativeRolloverScheduleFailureV1::Terminal { error, disposition }
            } else {
                M1AuthenticatedSpeculativeRolloverScheduleFailureV1::PreDetach {
                    error,
                    retry: Box::new(M1AuthenticatedSpeculativeRolloverSchedulePreDetachRetryV1 {
                        released,
                        intent,
                        coordinator,
                        inputs,
                        recipe_plans,
                        preparation_plans,
                        prior_window_continuation,
                    }),
                }
            }
        }
        PendingM1AuthenticatedSpeculativeRolloverScheduleFailureV1::Detach {
            error,
            source,
            retained,
        } => {
            engine.quarantine_m1_queue_rearm_failure();
            M1AuthenticatedSpeculativeRolloverScheduleFailureV1::Terminal {
                error,
                disposition: quarantined_disposition((source, retained)),
            }
        }
        PendingM1AuthenticatedSpeculativeRolloverScheduleFailureV1::Detached(detached) => {
            let error = detached.error;
            engine.quarantine_m1_queue_rearm_failure();
            let disposition = match detached.destroy_queue_and_retain_custody(engine) {
                Ok(released) => released_disposition(released),
                Err(quarantined) => quarantined_disposition(quarantined),
            };
            M1AuthenticatedSpeculativeRolloverScheduleFailureV1::Terminal { error, disposition }
        }
    }
}

/// Detached, exactly scheduled rollover with a private split causal witness.
#[must_use = "scheduled rollover must be prepared or torn down"]
#[derive(Debug)]
pub struct M1AuthenticatedScheduledSpeculativeRolloverV1 {
    prior: M1ServingPlanV1,
    next: M1ServingPlanV1,
    reason: M1ServingRolloverReasonV1,
    queue: M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1,
    scheduled: M1ScheduledDispatchV1,
    selected: Vec<ActiveDeviceKvCache>,
    residue: M1AuthenticatedSpeculativeRolloverResidueV1,
    inputs: M1FiniteSpeculativeQueueRolloverKvInputsV1,
    recipe_plans: M1FullStepWorkspacePlans,
    preparation_plans: M1FullStepWorkspacePlans,
    physical_lineage: M1AuthenticatedSpeculativePhysicalLineageWitnessV1,
    logical: M1AuthenticatedSpeculativeRolloverLogicalV1,
}

impl M1AuthenticatedScheduledSpeculativeRolloverV1 {
    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        &self.scheduled
    }

    #[must_use]
    pub const fn next_plan(&self) -> M1ServingPlanV1 {
        self.next
    }

    #[must_use]
    pub fn selected_cache_at(&self, lane: usize) -> Option<DeviceKvCacheProjection> {
        self.selected.get(lane).map(ActiveDeviceKvCache::projection)
    }

    /// Destroys the detached queue and preserves every scheduled owner.
    ///
    /// # Errors
    ///
    /// Returns the lower terminal release quarantine with all scheduled custody.
    pub fn destroy_queue_and_retain_custody<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1AuthenticatedSpeculativeRolloverTeardownSuccessV1,
        Box<M1AuthenticatedSpeculativeRolloverTeardownFailureV1>,
    > {
        M1AuthenticatedSpeculativeRolloverDetachedFailureV1 {
            error: M1AuthenticatedSpeculativeRolloverScheduleErrorV1::Coordinator,
            queue: self.queue,
            retained: Box::new((
                self.prior,
                self.next,
                self.reason,
                self.scheduled,
                self.selected,
                self.residue,
                self.inputs,
                self.recipe_plans,
                self.preparation_plans,
                self.physical_lineage,
                self.logical,
            )),
        }
        .destroy_queue_and_retain_custody(engine)
    }
}

fn next_epoch(epoch: CompletionEpoch) -> Option<CompletionEpoch> {
    epoch.value().checked_add(1).map(CompletionEpoch::new)
}

fn transition(
    batch: &M1ServingBatchPlanV1,
) -> Result<
    (M1ServingPlanV1, M1ServingPlanV1, M1ServingRolloverReasonV1),
    M1AuthenticatedSpeculativeRolloverScheduleErrorV1,
> {
    let M1ServingQueueActionV1::QuiescentRollover {
        prior,
        next,
        reason,
    } = batch.action()
    else {
        return Err(M1AuthenticatedSpeculativeRolloverScheduleErrorV1::Action);
    };
    if next != batch.plan()
        || prior.shape() != M1PhysicalFixedBatchShapeV1::PairedPrefill
        || !matches!(
            next.shape(),
            M1PhysicalFixedBatchShapeV1::SpeculativeK4
                | M1PhysicalFixedBatchShapeV1::SpeculativeK8
                | M1PhysicalFixedBatchShapeV1::SpeculativeK16
        )
        || admit_m1_production_rollover_transition_v1(prior, next)
            .is_none_or(|admitted| admitted.reason() != reason)
    {
        return Err(M1AuthenticatedSpeculativeRolloverScheduleErrorV1::Transition);
    }
    Ok((prior, next, reason))
}

fn paired_prefill_plan(target: ferric_spec::Qwen3PlanSelection) -> Option<M1ServingPlanV1> {
    M1ServingPlanV1::new(
        target,
        ferric_spec::Qwen3PlanSelection {
            role: ferric_spec::Qwen3ModelRole::Draft06B,
            mode: target.mode,
            bucket: target.bucket,
        },
    )
    .ok()
    .filter(|plan| plan.shape() == M1PhysicalFixedBatchShapeV1::PairedPrefill)
}

/// Fixes the exact speculative successor and ordered request policies into a
/// paired-prefill step before that step enters physical publication.
///
/// This creates no completion or KFD authority. The private physical half must
/// return through normal authenticated prefill readback and release before the
/// logical half can authorize rollover scheduling.
///
/// # Errors
///
/// Returns every input unchanged if the prepared step is not the exact paired
/// prefill predecessor, the successor/profile or roster differs, identity
/// allocation fails, or lineage is already attached.
pub fn bind_m1_authenticated_speculative_rollover_intent_v1(
    prepared: M1PreparedScheduledWorkspaceImagesV1,
    successor: M1ServingPlanV1,
    members: Vec<M1AuthenticatedSpeculativeRolloverMemberIntentV1>,
) -> Result<
    M1AuthenticatedSpeculativeRolloverIntentPreparedV1,
    Box<M1AuthenticatedSpeculativeRolloverIntentFailureV1>,
> {
    let reject = |error, prepared, members| {
        Box::new(M1AuthenticatedSpeculativeRolloverIntentFailureV1 {
            error,
            prepared,
            successor,
            members,
        })
    };
    if prepared.kind() != M1FullStepWorkspaceInputKind::PairedPrefill {
        return Err(reject(
            M1AuthenticatedSpeculativeRolloverIntentErrorV1::PairedPrefill,
            prepared,
            members,
        ));
    }
    let prefill_selection = prepared.step().kv_reservations().target_selection();
    let Some(prefill) = paired_prefill_plan(prefill_selection) else {
        return Err(reject(
            M1AuthenticatedSpeculativeRolloverIntentErrorV1::PairedPrefill,
            prepared,
            members,
        ));
    };
    if admit_m1_production_rollover_transition_v1(prefill, successor).is_none() {
        return Err(reject(
            M1AuthenticatedSpeculativeRolloverIntentErrorV1::Successor,
            prepared,
            members,
        ));
    }
    let scheduled = prepared.step().scheduled_dispatch();
    if members.is_empty()
        || members.len() != scheduled.member_count()
        || members
            .iter()
            .enumerate()
            .any(|(lane, member)| scheduled.member(lane) != Some(member.request()))
    {
        return Err(reject(
            M1AuthenticatedSpeculativeRolloverIntentErrorV1::Roster,
            prepared,
            members,
        ));
    }
    let Some(identity) =
        crate::authenticated_speculative_executor::M1AuthenticatedSpeculativeLineageIdentityV1::fresh()
    else {
        return Err(reject(
            M1AuthenticatedSpeculativeRolloverIntentErrorV1::IdentityExhausted,
            prepared,
            members,
        ));
    };
    let prefill_epoch = scheduled.epoch();
    let physical = M1AuthenticatedSpeculativeRolloverPhysicalIntentV1 {
        identity,
        prefill_selection,
        speculative_selection: successor.target(),
        prefill_epoch,
        members: members.clone().into_boxed_slice(),
    };
    let prepared = match prepared.retain_speculative_rollover_intent(physical) {
        Ok(prepared) => prepared,
        Err(prepared) => {
            return Err(reject(
                M1AuthenticatedSpeculativeRolloverIntentErrorV1::Attachment,
                prepared,
                members,
            ));
        }
    };
    Ok(M1AuthenticatedSpeculativeRolloverIntentPreparedV1 {
        prepared,
        intent: M1AuthenticatedSpeculativeRolloverIntentV1 {
            identity,
            prefill_selection,
            speculative_selection: successor.target(),
            prefill_epoch,
            members: members.into_boxed_slice(),
        },
    })
}

fn exact_member_association(
    seed: crate::M1SpeculativeMemberSeedV1,
    member: &M1ReleasedDeviceKvMemberV1,
    record: &crate::InertCheckedCompletionRecord,
    logical_accepted: u32,
    externally_published: u32,
) -> bool {
    let M1ReleasedDeviceKvMemberV1::Active(cache) = member else {
        return false;
    };
    let projection = cache.projection();
    let wire = record.record();
    let Some(emitted) = wire
        .emitted_tokens
        .get(..usize::from(wire.emitted_token_count))
    else {
        return false;
    };
    exact_member_association_values(
        seed,
        projection.request,
        wire.request,
        emitted,
        projection.target.committed_tokens,
        projection.draft.committed_tokens,
        logical_accepted,
        externally_published,
    )
}

#[allow(clippy::too_many_arguments)]
fn exact_member_association_values(
    seed: crate::M1SpeculativeMemberSeedV1,
    released_request: ferric_spec::RequestId,
    checked_request: ferric_spec::RequestId,
    emitted: &[ferric_spec::TokenId],
    target_committed: u32,
    draft_committed: u32,
    logical_accepted: u32,
    externally_published: u32,
) -> bool {
    seed.request() == checked_request
        && seed.request() == released_request
        && emitted == [seed.round_anchor()]
        && seed.target_committed_tokens() == target_committed
        && seed.draft_committed_tokens() == draft_committed
        && seed.policy().permits_fresh_anchor(seed.round_anchor())
        && logical_accepted == 1
        && externally_published == 1
}

#[allow(clippy::too_many_arguments)]
fn exact_rollover_intent_association(
    physical: &M1AuthenticatedSpeculativeRolloverPhysicalIntentV1,
    logical: &M1AuthenticatedSpeculativeRolloverIntentV1,
    prior: M1ServingPlanV1,
    next: M1ServingPlanV1,
    released_selection: ferric_spec::Qwen3PlanSelection,
    released_epoch: CompletionEpoch,
    successor_epoch: CompletionEpoch,
    seeds: &[crate::M1SpeculativeMemberSeedV1],
) -> bool {
    physical.identity == logical.identity
        && physical.prefill_selection == logical.prefill_selection
        && physical.speculative_selection == logical.speculative_selection
        && physical.prefill_epoch == logical.prefill_epoch
        && physical.members == logical.members
        && physical.prefill_selection == prior.target()
        && physical.speculative_selection == next.target()
        && physical.prefill_selection == released_selection
        && physical.prefill_epoch == released_epoch
        && next_epoch(physical.prefill_epoch) == Some(successor_epoch)
        && logical.members.len() == seeds.len()
        && seeds
            .iter()
            .zip(logical.members.iter())
            .all(|(seed, member)| {
                seed.request() == member.request() && seed.policy() == member.policy()
            })
}

#[allow(clippy::too_many_arguments)]
fn preflight<const C: usize>(
    engine: &Engine<C>,
    released: &M1AuthenticatedReleasedCompletedStepV1,
    batch: &M1ServingBatchPlanV1,
    intent: &M1AuthenticatedSpeculativeRolloverIntentV1,
    coordinator: &M1SpeculativeGenerationLoopV1,
    inputs: &M1AuthenticatedSpeculativeRolloverInputsV1,
) -> Result<
    (
        M1ServingPlanV1,
        M1ServingPlanV1,
        M1ServingRolloverReasonV1,
        crate::M1SpeculativeRoundBindingV1,
    ),
    M1AuthenticatedSpeculativeRolloverScheduleErrorV1,
> {
    let (prior, next, reason) = transition(batch)?;
    if engine.is_faulted() {
        return Err(M1AuthenticatedSpeculativeRolloverScheduleErrorV1::EngineFaulted);
    }
    if next_epoch(released.checked().epoch()) != Some(batch.epoch()) {
        return Err(M1AuthenticatedSpeculativeRolloverScheduleErrorV1::Epoch);
    }
    if batch.requests().is_empty()
        || batch.requests().len() > next.sequence_capacity()
        || released.members().len() != batch.requests().len()
        || released.checked().records().len() != batch.requests().len()
        || released.logical_accepted_counts().len() != batch.requests().len()
        || released.externally_published_counts().len() != batch.requests().len()
    {
        return Err(M1AuthenticatedSpeculativeRolloverScheduleErrorV1::Roster);
    }
    if released.queue().shape() != M1PhysicalFixedBatchShapeV1::PairedPrefill {
        return Err(M1AuthenticatedSpeculativeRolloverScheduleErrorV1::QueueShape);
    }
    let queue = released.queue().custody();
    if queue.selection() != prior.target() || released.checked().selection() != prior.target() {
        return Err(M1AuthenticatedSpeculativeRolloverScheduleErrorV1::QueueSelection);
    }
    if queue
        .partitioned_memory()
        .finite_speculative_rollover_output_state()
        != crate::M1FiniteSpeculativeRolloverOutputPortfolioStateV1::Reserved
    {
        return Err(M1AuthenticatedSpeculativeRolloverScheduleErrorV1::OutputReserve);
    }
    let seeds = coordinator
        .bootstrap_seed_snapshot()
        .map_err(|_| M1AuthenticatedSpeculativeRolloverScheduleErrorV1::Coordinator)?;
    if coordinator.shape().selection() != next.target() || seeds.len() != batch.requests().len() {
        return Err(M1AuthenticatedSpeculativeRolloverScheduleErrorV1::Coordinator);
    }
    let Some(physical_intent) = released.checked().speculative_rollover_intent() else {
        return Err(M1AuthenticatedSpeculativeRolloverScheduleErrorV1::Lineage);
    };
    if !exact_rollover_intent_association(
        physical_intent,
        intent,
        prior,
        next,
        released.checked().selection(),
        released.checked().epoch(),
        batch.epoch(),
        &seeds,
    ) {
        return Err(M1AuthenticatedSpeculativeRolloverScheduleErrorV1::Lineage);
    }
    let binding = coordinator
        .bind_round(0, batch.epoch(), batch.requests())
        .map_err(|_| M1AuthenticatedSpeculativeRolloverScheduleErrorV1::Coordinator)?;
    let (draft, target) = inputs.validated_inputs();
    if !speculative_validated_pair_matches_binding(draft, target, &binding) {
        return Err(M1AuthenticatedSpeculativeRolloverScheduleErrorV1::Inputs);
    }
    for lane in 0..batch.requests().len() {
        let member = &released.members()[lane];
        let M1ReleasedDeviceKvMemberV1::Active(cache) = member else {
            return Err(M1AuthenticatedSpeculativeRolloverScheduleErrorV1::MemberCustody { lane });
        };
        if cache.projection().request != batch.requests()[lane] {
            return Err(M1AuthenticatedSpeculativeRolloverScheduleErrorV1::MemberCustody { lane });
        }
        if !exact_member_association(
            seeds[lane],
            member,
            &released.checked().records()[lane],
            released.logical_accepted_counts()[lane],
            released.externally_published_counts()[lane],
        ) {
            return Err(
                M1AuthenticatedSpeculativeRolloverScheduleErrorV1::CoordinatorSeed { lane },
            );
        }
        cache
            .preflight_quiescent_reselection(next.target(), next.draft_cache_selection())
            .map_err(
                |_| M1AuthenticatedSpeculativeRolloverScheduleErrorV1::MemberCustody { lane },
            )?;
        if engine.state(batch.requests()[lane]) != Some(RequestState::Ready) {
            return Err(
                M1AuthenticatedSpeculativeRolloverScheduleErrorV1::RequestNotReady { lane },
            );
        }
    }
    Ok((prior, next, reason, binding))
}

/// Validates and detaches one exact authenticated paired-prefill generation.
///
/// The prefill intent identity is upgraded only after its released physical
/// half, logical half, fresh coordinator seed, successor inputs, and serving
/// transition all agree.
///
/// # Errors
///
/// Pure preflight rejection returns an opaque retry owner without faulting the
/// Engine. Once detachment starts, every failure closes or quarantines queue
/// custody and permanently faults the Engine before returning.
#[allow(clippy::too_many_arguments)]
pub fn schedule_m1_authenticated_speculative_rollover_v1<const C: usize>(
    engine: &mut Engine<C>,
    released: M1AuthenticatedReleasedCompletedStepV1,
    batch: &M1ServingBatchPlanV1,
    intent: M1AuthenticatedSpeculativeRolloverIntentV1,
    coordinator: M1SpeculativeGenerationLoopV1,
    inputs: M1FiniteSpeculativeQueueRolloverKvInputsV1,
    recipe_plans: M1FullStepWorkspacePlans,
    preparation_plans: M1FullStepWorkspacePlans,
) -> Result<
    M1AuthenticatedScheduledSpeculativeRolloverV1,
    M1AuthenticatedSpeculativeRolloverScheduleFailureV1,
> {
    schedule_m1_authenticated_speculative_rollover_pending_v1(
        engine,
        released,
        batch,
        intent,
        coordinator,
        M1AuthenticatedSpeculativeRolloverInputsV1::Preleased(inputs),
        recipe_plans,
        preparation_plans,
        M1AuthenticatedSpeculativePriorWindowContinuationV1::initial(),
    )
    .map_err(|failure| close_pending_schedule_failure(engine, failure))
}

/// Pure failure before the released paired-prefill bridge can enter rollover.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedSpeculativeNewWindowSuccessorJoinErrorV1 {
    MissingBridge,
    Successor,
}

#[must_use = "the unchanged released round and successor inputs remain retryable"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativeNewWindowSuccessorJoinRetryV1 {
    released: crate::M1AuthenticatedLongLivedQueueReleasedRoundV1,
    coordinator: M1SpeculativeGenerationLoopV1,
    inputs: M1AuthenticatedSpeculativeNewWindowSuccessorInputsV1,
    recipe_plans: M1FullStepWorkspacePlans,
    preparation_plans: M1FullStepWorkspacePlans,
}

/// Exact bridge admission rejection or existing authenticated rollover failure.
#[must_use = "successor join failure retains every authenticated owner"]
#[derive(Debug)]
pub enum M1AuthenticatedSpeculativeNewWindowSuccessorJoinFailureV1 {
    PreJoin {
        error: M1AuthenticatedSpeculativeNewWindowSuccessorJoinErrorV1,
        retry: Box<M1AuthenticatedSpeculativeNewWindowSuccessorJoinRetryV1>,
    },
    Rollover(M1AuthenticatedSpeculativeRolloverScheduleFailureV1),
}

impl M1AuthenticatedSpeculativeNewWindowSuccessorJoinRetryV1 {
    /// Retries without exposing the released queue, bridge, or logical owners.
    ///
    /// # Errors
    ///
    /// Returns renewed opaque join custody when the retained bridge still does
    /// not match the batch, or an authenticated rollover failure after joining.
    pub fn retry<const C: usize>(
        self,
        engine: &mut Engine<C>,
        batch: &M1ServingBatchPlanV1,
    ) -> Result<
        M1AuthenticatedScheduledSpeculativeRolloverV1,
        M1AuthenticatedSpeculativeNewWindowSuccessorJoinFailureV1,
    > {
        schedule_m1_authenticated_speculative_new_window_successor_v1(
            engine,
            self.released,
            batch,
            self.coordinator,
            self.inputs,
            self.recipe_plans,
            self.preparation_plans,
        )
    }

    /// Abandons the join without exposing reusable queue or bridge authority.
    #[must_use = "clean release or terminal quarantine remains retained"]
    pub fn cancel_and_close<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> crate::M1AuthenticatedSpeculativeFailureDispositionV1 {
        use crate::authenticated_speculative_executor::{
            quarantined_disposition, released_disposition,
        };

        engine.quarantine_m1_queue_rearm_failure();
        let retained = (
            self.coordinator,
            self.inputs,
            self.recipe_plans,
            self.preparation_plans,
        );
        match self.released.destroy_queue_and_retain_round(engine) {
            Ok(released) => released_disposition((released, retained)),
            Err(quarantined) => quarantined_disposition((quarantined, retained)),
        }
    }
}

/// Consumes the released authenticated paired-prefill round into its exact
/// fresh speculative successor. The successor coordinator is supplied only
/// now, after its anchors and committed cursors can be derived from readback.
/// The timeout frozen before paired-prefill publication cannot be replaced.
#[allow(clippy::too_many_arguments)]
pub(crate) fn schedule_m1_authenticated_speculative_new_window_successor_v1<const C: usize>(
    engine: &mut Engine<C>,
    released: crate::M1AuthenticatedLongLivedQueueReleasedRoundV1,
    batch: &M1ServingBatchPlanV1,
    coordinator: M1SpeculativeGenerationLoopV1,
    inputs: M1AuthenticatedSpeculativeNewWindowSuccessorInputsV1,
    recipe_plans: M1FullStepWorkspacePlans,
    preparation_plans: M1FullStepWorkspacePlans,
) -> Result<
    M1AuthenticatedScheduledSpeculativeRolloverV1,
    M1AuthenticatedSpeculativeNewWindowSuccessorJoinFailureV1,
> {
    let successor = released.pending_new_window_speculative_successor();
    if successor.is_none() || successor != Some(batch.plan()) {
        return Err(
            M1AuthenticatedSpeculativeNewWindowSuccessorJoinFailureV1::PreJoin {
                error: if successor.is_none() {
                    M1AuthenticatedSpeculativeNewWindowSuccessorJoinErrorV1::MissingBridge
                } else {
                    M1AuthenticatedSpeculativeNewWindowSuccessorJoinErrorV1::Successor
                },
                retry: Box::new(M1AuthenticatedSpeculativeNewWindowSuccessorJoinRetryV1 {
                    released,
                    coordinator,
                    inputs,
                    recipe_plans,
                    preparation_plans,
                }),
            },
        );
    }
    let (released, terminal, history, bridge) = released
        .into_authenticated_new_window_successor_custody()
        .expect("bridge and empty parked custody were checked");
    let M1AuthenticatedSpeculativeNewWindowBridgeV1 {
        speculative_successor: _,
        intent,
        mut prior_windows,
        next_queue_wait_timeout,
    } = bridge;
    prior_windows
        .last_mut()
        .expect("new-window bridge always archives its predecessor")
        .attach_physical_history(history)
        .expect("predecessor physical history is attached exactly once");
    schedule_m1_authenticated_speculative_rollover_pending_v1(
        engine,
        released,
        batch,
        intent,
        coordinator,
        M1AuthenticatedSpeculativeRolloverInputsV1::NewWindow(inputs),
        recipe_plans,
        preparation_plans,
        M1AuthenticatedSpeculativePriorWindowContinuationV1 {
            terminal,
            history: crate::m1_queue_rearm::M1RearmRoundHistoryV1::Empty,
            prior_windows,
            frozen_queue_wait_timeout: Some(next_queue_wait_timeout),
        },
    )
    .map_err(|failure| {
        M1AuthenticatedSpeculativeNewWindowSuccessorJoinFailureV1::Rollover(
            close_pending_schedule_failure(engine, failure),
        )
    })
}

pub(crate) fn authenticated_speculative_successor_page_span(
    logical: ferric_spec::LogicalKvState,
    active_pages: usize,
    request: ferric_spec::RequestId,
    role: Qwen3ModelRole,
    context_tokens: u32,
    active_tokens: u32,
) -> Option<M1AuthenticatedSpeculativeSuccessorPageSpanV1> {
    if logical.request != request
        || logical.role != role
        || logical.lifecycle != PhysicalKvLifecycle::Active
        || logical.committed_tokens != context_tokens
        || logical.resident_tokens != context_tokens
        || active_tokens == 0
    {
        return None;
    }
    let first_page = logical.resident_tokens.div_ceil(M1_KV_PAGE_TOKENS);
    if usize::try_from(first_page).ok()? != active_pages {
        return None;
    }
    let required_pages = context_tokens
        .checked_add(active_tokens)?
        .div_ceil(M1_KV_PAGE_TOKENS);
    if usize::try_from(required_pages).ok()? > M1_KV_PHYSICAL_PAGE_SLOTS {
        return None;
    }
    Some(M1AuthenticatedSpeculativeSuccessorPageSpanV1 {
        first_page,
        page_count: required_pages.checked_sub(first_page)?,
    })
}

pub(crate) fn authenticated_speculative_successor_lease_roster_matches(
    leases: &[DeviceKvPageLease],
    spans: &[M1AuthenticatedSpeculativeSuccessorLanePageSpansV1],
) -> bool {
    let mut cursor = 0usize;
    for lane in spans {
        for (role, span) in [
            (Qwen3ModelRole::Draft06B, lane.draft),
            (Qwen3ModelRole::Target8B, lane.target),
        ] {
            let Some(end_page) = span.first_page.checked_add(span.page_count) else {
                return false;
            };
            for physical_index in span.first_page..end_page {
                let Some(lease) = leases.get(cursor) else {
                    return false;
                };
                if lease.request() != lane.request
                    || lease.page().role() != role
                    || lease.page().index() != physical_index
                    || lease.page().generation() == 0
                {
                    return false;
                }
                let Some(next_cursor) = cursor.checked_add(1) else {
                    return false;
                };
                cursor = next_cursor;
            }
        }
    }
    cursor == leases.len()
}

fn authenticated_speculative_tail_binding_width_matches(
    selected_shape: Option<crate::M1SpeculativePhysicalShapeV1>,
    draft_round_tokens: u32,
) -> bool {
    match selected_shape {
        Some(shape) => u32::from(shape.draft_tokens()) == draft_round_tokens,
        None => false,
    }
}

type M1AuthenticatedSpeculativeTailPageRostersV1 =
    (Vec<Vec<DeviceKvPageLease>>, Vec<Vec<DeviceKvPageLease>>);

pub(crate) fn materialize_m1_authenticated_speculative_tail_pages_v1(
    queue: &mut M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1,
    projections: &[crate::DeviceKvCacheProjection],
    draft_inputs: &ValidatedM1StepInputs,
    target_inputs: &ValidatedM1StepInputs,
    draft_round_tokens: u32,
) -> Result<
    M1AuthenticatedSpeculativeTailPageRostersV1,
    Box<M1AuthenticatedSpeculativeTailPageMaterializationFailureV1>,
> {
    let selected_shape =
        crate::M1SpeculativePhysicalShapeV1::from_selection(target_inputs.selection()).ok();
    if projections.is_empty()
        || draft_inputs.live_lane_count() as usize != projections.len()
        || target_inputs.live_lane_count() as usize != projections.len()
        || draft_round_tokens == 0
        || !authenticated_speculative_tail_binding_width_matches(selected_shape, draft_round_tokens)
    {
        return Err(Box::new(
            M1AuthenticatedSpeculativeTailPageMaterializationFailureV1::Span,
        ));
    }

    let mut lanes = Vec::new();
    let mut spans = Vec::new();
    let mut draft_page_leases = Vec::new();
    let mut target_page_leases = Vec::new();
    if lanes.try_reserve_exact(projections.len()).is_err()
        || spans.try_reserve_exact(projections.len()).is_err()
        || draft_page_leases
            .try_reserve_exact(projections.len())
            .is_err()
        || target_page_leases
            .try_reserve_exact(projections.len())
            .is_err()
    {
        return Err(Box::new(
            M1AuthenticatedSpeculativeTailPageMaterializationFailureV1::Span,
        ));
    }
    for (lane, projection) in projections.iter().copied().enumerate() {
        let Some(draft_plan) = draft_inputs.lanes().get(lane).and_then(Option::as_ref) else {
            return Err(Box::new(
                M1AuthenticatedSpeculativeTailPageMaterializationFailureV1::Span,
            ));
        };
        let Some(target_plan) = target_inputs.lanes().get(lane).and_then(Option::as_ref) else {
            return Err(Box::new(
                M1AuthenticatedSpeculativeTailPageMaterializationFailureV1::Span,
            ));
        };
        let Some(draft_context) = draft_inputs.context_lengths().get(lane).copied() else {
            return Err(Box::new(
                M1AuthenticatedSpeculativeTailPageMaterializationFailureV1::Span,
            ));
        };
        let Some(target_context) = target_inputs.context_lengths().get(lane).copied() else {
            return Err(Box::new(
                M1AuthenticatedSpeculativeTailPageMaterializationFailureV1::Span,
            ));
        };
        let Some(target_active) = target_inputs.active_lengths().get(lane).copied() else {
            return Err(Box::new(
                M1AuthenticatedSpeculativeTailPageMaterializationFailureV1::Span,
            ));
        };
        if projection.request != draft_plan.request() || projection.request != target_plan.request()
        {
            return Err(Box::new(
                M1AuthenticatedSpeculativeTailPageMaterializationFailureV1::Span,
            ));
        }
        let Some(draft) = authenticated_speculative_successor_page_span(
            projection.draft,
            projection.draft_active_pages,
            projection.request,
            Qwen3ModelRole::Draft06B,
            draft_context,
            draft_round_tokens,
        ) else {
            return Err(Box::new(
                M1AuthenticatedSpeculativeTailPageMaterializationFailureV1::Span,
            ));
        };
        let Some(target) = authenticated_speculative_successor_page_span(
            projection.target,
            projection.target_active_pages,
            projection.request,
            Qwen3ModelRole::Target8B,
            target_context,
            target_active,
        ) else {
            return Err(Box::new(
                M1AuthenticatedSpeculativeTailPageMaterializationFailureV1::Span,
            ));
        };
        let draft_count = match usize::try_from(draft.page_count) {
            Ok(count) => count,
            Err(_) => {
                return Err(Box::new(
                    M1AuthenticatedSpeculativeTailPageMaterializationFailureV1::Span,
                ));
            }
        };
        let target_count = match usize::try_from(target.page_count) {
            Ok(count) => count,
            Err(_) => {
                return Err(Box::new(
                    M1AuthenticatedSpeculativeTailPageMaterializationFailureV1::Span,
                ));
            }
        };
        let mut draft_leases = Vec::new();
        let mut target_leases = Vec::new();
        if draft_leases.try_reserve_exact(draft_count).is_err()
            || target_leases.try_reserve_exact(target_count).is_err()
        {
            return Err(Box::new(
                M1AuthenticatedSpeculativeTailPageMaterializationFailureV1::Span,
            ));
        }
        let lane_spans = M1AuthenticatedSpeculativeSuccessorLanePageSpansV1 {
            request: projection.request,
            draft,
            target,
        };
        let admission =
            match crate::device_cache::M1AuthenticatedNewWindowLaneAdmissionV1::successor(
                projection.request,
                draft.first_page,
                draft.page_count,
                target.first_page,
                target.page_count,
            ) {
                Ok(admission) => admission,
                Err(_) => {
                    return Err(Box::new(
                        M1AuthenticatedSpeculativeTailPageMaterializationFailureV1::Span,
                    ));
                }
            };
        spans.push(lane_spans);
        lanes.push(admission);
        draft_page_leases.push(draft_leases);
        target_page_leases.push(target_leases);
    }

    let admission = queue
        .admit_authenticated_successor_page_set(lanes)
        .map_err(|source| {
            Box::new(
                M1AuthenticatedSpeculativeTailPageMaterializationFailureV1::Admission { source },
            )
        })?;
    let page_leases = queue
        .commit_authenticated_successor_page_set(admission)
        .map_err(|source| {
            Box::new(M1AuthenticatedSpeculativeTailPageMaterializationFailureV1::Commit { source })
        })?;
    if !authenticated_speculative_successor_lease_roster_matches(&page_leases, &spans) {
        return Err(Box::new(
            M1AuthenticatedSpeculativeTailPageMaterializationFailureV1::CommittedRoster {
                spans,
                draft_page_leases,
                target_page_leases,
                remaining: page_leases.into_iter(),
            },
        ));
    }
    let mut page_leases = page_leases.into_iter();
    for ((lane_spans, draft), target) in spans
        .iter()
        .zip(draft_page_leases.iter_mut())
        .zip(target_page_leases.iter_mut())
    {
        draft.extend(
            page_leases
                .by_ref()
                .take(lane_spans.draft.page_count as usize),
        );
        target.extend(
            page_leases
                .by_ref()
                .take(lane_spans.target.page_count as usize),
        );
    }
    if !page_leases.as_slice().is_empty() {
        return Err(Box::new(
            M1AuthenticatedSpeculativeTailPageMaterializationFailureV1::CommittedRoster {
                spans,
                draft_page_leases,
                target_page_leases,
                remaining: page_leases,
            },
        ));
    }
    Ok((draft_page_leases, target_page_leases))
}

fn successor_page_failure<T>(
    failure: M1AuthenticatedSpeculativeSuccessorPageFailureV1,
) -> Result<T, Box<M1AuthenticatedSpeculativeSuccessorPageFailureV1>> {
    Err(Box::new(failure))
}

fn materialize_authenticated_speculative_rollover_inputs(
    queue: &mut M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1,
    members: &[M1ReleasedDeviceKvMemberV1],
    binding: &crate::M1SpeculativeRoundBindingV1,
    inputs: M1AuthenticatedSpeculativeRolloverInputsV1,
) -> Result<
    M1FiniteSpeculativeQueueRolloverKvInputsV1,
    Box<M1AuthenticatedSpeculativeSuccessorPageFailureV1>,
> {
    let inputs = match inputs {
        M1AuthenticatedSpeculativeRolloverInputsV1::Preleased(inputs) => return Ok(inputs),
        M1AuthenticatedSpeculativeRolloverInputsV1::NewWindow(inputs) => inputs,
    };
    let (draft_inputs, target_inputs) = inputs.validated_inputs();
    let draft_round_tokens = u32::from(binding.shape().draft_tokens());
    if members.is_empty()
        || draft_inputs.live_lane_count() as usize != members.len()
        || target_inputs.live_lane_count() as usize != members.len()
        || binding.members().len() != members.len()
        || draft_round_tokens == 0
    {
        return successor_page_failure(
            M1AuthenticatedSpeculativeSuccessorPageFailureV1::MemberRoster { inputs },
        );
    }
    let mut projections = Vec::new();
    if projections.try_reserve_exact(members.len()).is_err() {
        return successor_page_failure(
            M1AuthenticatedSpeculativeSuccessorPageFailureV1::MemberRoster { inputs },
        );
    }
    for member in members {
        let M1ReleasedDeviceKvMemberV1::Active(cache) = member else {
            return successor_page_failure(
                M1AuthenticatedSpeculativeSuccessorPageFailureV1::MemberRoster { inputs },
            );
        };
        projections.push(cache.projection());
    }
    let (draft_page_leases, target_page_leases) =
        match materialize_m1_authenticated_speculative_tail_pages_v1(
            queue,
            &projections,
            draft_inputs,
            target_inputs,
            draft_round_tokens,
        ) {
            Ok(page_leases) => page_leases,
            Err(source) => {
                return successor_page_failure(
                    M1AuthenticatedSpeculativeSuccessorPageFailureV1::Materialization {
                        source,
                        inputs,
                    },
                );
            }
        };
    Ok(inputs.into_raw(draft_page_leases, target_page_leases))
}

#[allow(clippy::too_many_arguments)]
fn schedule_m1_authenticated_speculative_rollover_pending_v1<const C: usize>(
    engine: &mut Engine<C>,
    released: M1AuthenticatedReleasedCompletedStepV1,
    batch: &M1ServingBatchPlanV1,
    intent: M1AuthenticatedSpeculativeRolloverIntentV1,
    coordinator: M1SpeculativeGenerationLoopV1,
    inputs: M1AuthenticatedSpeculativeRolloverInputsV1,
    recipe_plans: M1FullStepWorkspacePlans,
    preparation_plans: M1FullStepWorkspacePlans,
    prior_window_continuation: M1AuthenticatedSpeculativePriorWindowContinuationV1,
) -> Result<
    M1AuthenticatedScheduledSpeculativeRolloverV1,
    PendingM1AuthenticatedSpeculativeRolloverScheduleFailureV1,
> {
    let (prior, next, reason, binding) =
        match preflight(engine, &released, batch, &intent, &coordinator, &inputs) {
            Ok(value) => value,
            Err(error) => {
                return Err(
                    PendingM1AuthenticatedSpeculativeRolloverScheduleFailureV1::Rejected {
                        error,
                        released: Box::new(released),
                        intent: Box::new(intent),
                        coordinator: Box::new(coordinator),
                        inputs: Box::new(inputs),
                        recipe_plans,
                        preparation_plans,
                        prior_window_continuation: Box::new(prior_window_continuation),
                    },
                );
            }
        };
    let (physical_lineage, logical_lineage) = match upgrade_m1_authenticated_speculative_lineage_v1(
        &coordinator,
        &binding,
        intent.identity,
    ) {
        Ok(value) => value,
        Err(_) => {
            return Err(
                PendingM1AuthenticatedSpeculativeRolloverScheduleFailureV1::Rejected {
                    error: M1AuthenticatedSpeculativeRolloverScheduleErrorV1::Lineage,
                    released: Box::new(released),
                    intent: Box::new(intent),
                    coordinator: Box::new(coordinator),
                    inputs: Box::new(inputs),
                    recipe_plans,
                    preparation_plans,
                    prior_window_continuation: Box::new(prior_window_continuation),
                },
            );
        }
    };
    let (
        queue,
        checked,
        members,
        logical_accepted_counts,
        externally_published_counts,
        release_counts,
        completed_members,
        total_released,
    ) = released.into_rearm_parts();
    let residue = M1AuthenticatedSpeculativeRolloverResidueV1 {
        checked,
        logical_accepted_counts,
        externally_published_counts,
        release_counts,
        completed_members,
        total_released,
        terminal: prior_window_continuation.terminal,
        history: prior_window_continuation.history,
    };
    let mut queue = match queue.detach() {
        Ok(queue) => queue,
        Err(source) => {
            engine.quarantine_m1_queue_rearm_failure();
            return Err(
                PendingM1AuthenticatedSpeculativeRolloverScheduleFailureV1::Detach {
                    error: M1AuthenticatedSpeculativeRolloverScheduleErrorV1::Detach,
                    source,
                    retained: Box::new((
                        members,
                        residue,
                        inputs,
                        recipe_plans,
                        preparation_plans,
                        physical_lineage,
                        coordinator,
                        logical_lineage,
                    )),
                },
            );
        }
    };
    let inputs = match materialize_authenticated_speculative_rollover_inputs(
        &mut queue, &members, &binding, inputs,
    ) {
        Ok(inputs) => inputs,
        Err(failure) => {
            engine.quarantine_m1_queue_rearm_failure();
            return Err(
                PendingM1AuthenticatedSpeculativeRolloverScheduleFailureV1::Detached(Box::new(
                    M1AuthenticatedSpeculativeRolloverDetachedFailureV1 {
                        error: M1AuthenticatedSpeculativeRolloverScheduleErrorV1::SuccessorPages,
                        queue,
                        retained: Box::new((
                            failure,
                            members,
                            residue,
                            recipe_plans,
                            preparation_plans,
                            physical_lineage,
                            coordinator,
                            logical_lineage,
                        )),
                    },
                )),
            );
        }
    };
    let scheduled = match engine.dispatch_m1_exact_ready(batch.epoch(), batch.requests()) {
        Ok(scheduled) => scheduled,
        Err(source) => {
            engine.quarantine_m1_queue_rearm_failure();
            return Err(
                PendingM1AuthenticatedSpeculativeRolloverScheduleFailureV1::Detached(Box::new(
                    M1AuthenticatedSpeculativeRolloverDetachedFailureV1 {
                        error: M1AuthenticatedSpeculativeRolloverScheduleErrorV1::ExactDispatch,
                        queue,
                        retained: Box::new((
                            source,
                            members,
                            residue,
                            inputs,
                            recipe_plans,
                            preparation_plans,
                            physical_lineage,
                            coordinator,
                            logical_lineage,
                        )),
                    },
                )),
            );
        }
    };
    let mut selected = Vec::new();
    if selected.try_reserve_exact(members.len()).is_err() {
        engine.quarantine_m1_queue_rearm_failure();
        return Err(
            PendingM1AuthenticatedSpeculativeRolloverScheduleFailureV1::Detached(Box::new(
                M1AuthenticatedSpeculativeRolloverDetachedFailureV1 {
                    error: M1AuthenticatedSpeculativeRolloverScheduleErrorV1::Roster,
                    queue,
                    retained: Box::new((
                        scheduled,
                        members,
                        residue,
                        inputs,
                        recipe_plans,
                        preparation_plans,
                        physical_lineage,
                        coordinator,
                        logical_lineage,
                    )),
                },
            )),
        );
    }
    for (lane, member) in members.into_iter().enumerate() {
        let M1ReleasedDeviceKvMemberV1::Active(mut cache) = member else {
            unreachable!("all-active rollover roster was checked before detachment")
        };
        if let Err(source) = cache.reselect_quiescent(next.target(), next.draft_cache_selection()) {
            engine.quarantine_m1_queue_rearm_failure();
            return Err(
                PendingM1AuthenticatedSpeculativeRolloverScheduleFailureV1::Detached(Box::new(
                    M1AuthenticatedSpeculativeRolloverDetachedFailureV1 {
                        error:
                            M1AuthenticatedSpeculativeRolloverScheduleErrorV1::CacheReselection {
                                lane,
                            },
                        queue,
                        retained: Box::new((
                            source,
                            cache,
                            selected,
                            scheduled,
                            residue,
                            inputs,
                            recipe_plans,
                            preparation_plans,
                            physical_lineage,
                            coordinator,
                            logical_lineage,
                        )),
                    },
                )),
            );
        }
        selected.push(cache);
    }
    Ok(M1AuthenticatedScheduledSpeculativeRolloverV1 {
        prior,
        next,
        reason,
        queue,
        scheduled,
        selected,
        residue,
        inputs,
        recipe_plans,
        preparation_plans,
        physical_lineage,
        logical: M1AuthenticatedSpeculativeRolloverLogicalV1 {
            coordinator,
            epoch: binding.epoch(),
            lineage: logical_lineage,
            prior_windows: prior_window_continuation.prior_windows,
            frozen_queue_wait_timeout: prior_window_continuation.frozen_queue_wait_timeout,
        },
    })
}

/// Stage reached by a failed detached rollover preparation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedSpeculativeRolloverPrepareStageV1 {
    Recipe,
    Inputs,
    DraftReservation,
    TargetReservation,
    TargetTable,
    DraftTable,
    Workspace,
    LineageAttachment,
}

/// Terminal detached preparation failure without queue authority.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedSpeculativeRolloverPrepareFailureV1;
/// fn recover_queue(failure: M1AuthenticatedSpeculativeRolloverPrepareFailureV1) {
///     let _queue = failure.into_detached_queue();
/// }
/// ```
#[must_use = "terminal rollover preparation custody remains retained"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativeRolloverPrepareFailureV1 {
    stage: M1AuthenticatedSpeculativeRolloverPrepareStageV1,
    disposition: crate::M1AuthenticatedSpeculativeFailureDispositionV1,
}

impl M1AuthenticatedSpeculativeRolloverPrepareFailureV1 {
    #[must_use]
    pub const fn stage(&self) -> M1AuthenticatedSpeculativeRolloverPrepareStageV1 {
        self.stage
    }

    #[must_use = "the terminal disposition must remain observed"]
    pub const fn disposition(&self) -> &crate::M1AuthenticatedSpeculativeFailureDispositionV1 {
        &self.disposition
    }

    #[allow(clippy::boxed_local)]
    pub(crate) fn into_disposition(
        self: Box<Self>,
    ) -> crate::M1AuthenticatedSpeculativeFailureDispositionV1 {
        self.disposition.retain(self.stage)
    }
}

/// Internal detached preparation custody pending mandatory closure.
#[derive(Debug)]
struct PendingM1AuthenticatedSpeculativeRolloverPrepareFailureV1 {
    stage: M1AuthenticatedSpeculativeRolloverPrepareStageV1,
    queue: M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1,
    retained: Box<dyn fmt::Debug>,
}

impl PendingM1AuthenticatedSpeculativeRolloverPrepareFailureV1 {
    /// Destroys the detached queue while retaining failed preparation custody.
    ///
    /// # Errors
    ///
    /// Returns the lower terminal release quarantine with all Ferric custody.
    fn destroy_queue_and_retain_custody<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1AuthenticatedSpeculativeRolloverTeardownSuccessV1,
        Box<M1AuthenticatedSpeculativeRolloverTeardownFailureV1>,
    > {
        M1AuthenticatedSpeculativeRolloverDetachedFailureV1 {
            error: M1AuthenticatedSpeculativeRolloverScheduleErrorV1::Coordinator,
            queue: self.queue,
            retained: Box::new((self.stage, self.retained)),
        }
        .destroy_queue_and_retain_custody(engine)
    }
}

#[allow(clippy::boxed_local, clippy::unnecessary_box_returns)]
fn close_pending_preparation_failure<const C: usize>(
    engine: &mut Engine<C>,
    pending: Box<PendingM1AuthenticatedSpeculativeRolloverPrepareFailureV1>,
) -> Box<M1AuthenticatedSpeculativeRolloverPrepareFailureV1> {
    use crate::authenticated_speculative_executor::{
        quarantined_disposition, released_disposition,
    };

    engine.quarantine_m1_queue_rearm_failure();
    let stage = pending.stage;
    let disposition = match pending.destroy_queue_and_retain_custody(engine) {
        Ok(released) => released_disposition(released),
        Err(quarantined) => quarantined_disposition(quarantined),
    };
    Box::new(M1AuthenticatedSpeculativeRolloverPrepareFailureV1 { stage, disposition })
}

#[allow(clippy::unnecessary_box_returns)]
fn preparation_failure(
    stage: M1AuthenticatedSpeculativeRolloverPrepareStageV1,
    queue: M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1,
    retained: impl fmt::Debug + 'static,
) -> Box<PendingM1AuthenticatedSpeculativeRolloverPrepareFailureV1> {
    Box::new(PendingM1AuthenticatedSpeculativeRolloverPrepareFailureV1 {
        stage,
        queue,
        retained: Box::new(retained),
    })
}

/// Fully prepared authenticated cross-shape rollover.
#[must_use = "prepared rollover must be submitted or torn down"]
#[derive(Debug)]
pub struct M1AuthenticatedPreparedSpeculativeRolloverV1 {
    pub(crate) prior: M1ServingPlanV1,
    pub(crate) next: M1ServingPlanV1,
    pub(crate) reason: M1ServingRolloverReasonV1,
    pub(crate) queue: M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1,
    pub(crate) selected: Vec<ActiveDeviceKvCache>,
    pub(crate) residue: M1AuthenticatedSpeculativeRolloverResidueV1,
    pub(crate) prepared: M1PreparedScheduledWorkspaceImagesV1,
    pub(crate) recipe: AddresslessM1PhysicalBufferRecipeV1,
    pub(crate) logical: M1AuthenticatedSpeculativeRolloverLogicalV1,
}

impl M1AuthenticatedPreparedSpeculativeRolloverV1 {
    #[must_use]
    pub const fn next_plan(&self) -> M1ServingPlanV1 {
        self.next
    }

    #[must_use]
    pub const fn next_epoch(&self) -> CompletionEpoch {
        self.prepared.step().scheduled_dispatch().epoch()
    }

    pub(crate) fn cancel_and_close<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> crate::M1AuthenticatedSpeculativeFailureDispositionV1 {
        let Self {
            prior,
            next,
            reason,
            queue,
            selected,
            residue,
            prepared,
            recipe,
            logical,
        } = self;
        let pending = PendingM1AuthenticatedSpeculativeRolloverPrepareFailureV1 {
            stage: M1AuthenticatedSpeculativeRolloverPrepareStageV1::LineageAttachment,
            queue,
            retained: Box::new((
                prior, next, reason, selected, residue, prepared, recipe, logical,
            )),
        };
        let closed = close_pending_preparation_failure(engine, Box::new(pending));
        closed.into_disposition()
    }
}

/// Reserves exact successor KV writes and prepares authenticated rollover images.
///
/// # Errors
///
/// Every rejection quarantines the Engine and consumes the detached queue. The
/// returned failure contains only clean release evidence or opaque quarantine.
///
/// # Panics
///
/// Internal exact-size iterators are indexed only after their roster lengths
/// have been validated.
pub fn prepare_m1_authenticated_speculative_rollover_v1<const C: usize>(
    engine: &mut Engine<C>,
    scheduled: M1AuthenticatedScheduledSpeculativeRolloverV1,
    runner: &LogicalRunnerDeclaration,
) -> Result<
    M1AuthenticatedPreparedSpeculativeRolloverV1,
    Box<M1AuthenticatedSpeculativeRolloverPrepareFailureV1>,
> {
    prepare_m1_authenticated_speculative_rollover_pending_v1(engine, scheduled, Some(runner))
        .map_err(|failure| close_pending_preparation_failure(engine, failure))
}

/// Prepares rollover from the logical declaration already sealed into the
/// authenticated predecessor queue.
pub(crate) fn prepare_m1_authenticated_speculative_rollover_retained_v1<const C: usize>(
    engine: &mut Engine<C>,
    scheduled: M1AuthenticatedScheduledSpeculativeRolloverV1,
) -> Result<
    M1AuthenticatedPreparedSpeculativeRolloverV1,
    Box<M1AuthenticatedSpeculativeRolloverPrepareFailureV1>,
> {
    prepare_m1_authenticated_speculative_rollover_pending_v1(engine, scheduled, None)
        .map_err(|failure| close_pending_preparation_failure(engine, failure))
}

fn prepare_m1_authenticated_speculative_rollover_pending_v1<const C: usize>(
    engine: &mut Engine<C>,
    scheduled: M1AuthenticatedScheduledSpeculativeRolloverV1,
    runner: Option<&LogicalRunnerDeclaration>,
) -> Result<
    M1AuthenticatedPreparedSpeculativeRolloverV1,
    Box<PendingM1AuthenticatedSpeculativeRolloverPrepareFailureV1>,
> {
    let M1AuthenticatedScheduledSpeculativeRolloverV1 {
        prior,
        next,
        reason,
        queue,
        scheduled,
        mut selected,
        residue,
        inputs,
        recipe_plans,
        preparation_plans,
        physical_lineage,
        logical,
    } = scheduled;
    let runner = runner.unwrap_or_else(|| queue.operations().runner());
    let recipe = match crate::runner::derive_physical_step_recipe(
        queue.operations(),
        crate::M1StepDispatchIntent::SpeculativeRound(next.target()),
        recipe_plans,
    ) {
        M1PhysicalRunnerRecipeOutcomeV1::Prepared(recipe) => recipe,
        M1PhysicalRunnerRecipeOutcomeV1::Rejected(source) => {
            engine.quarantine_m1_queue_rearm_failure();
            return Err(preparation_failure(
                M1AuthenticatedSpeculativeRolloverPrepareStageV1::Recipe,
                queue,
                (
                    source,
                    prior,
                    next,
                    reason,
                    scheduled,
                    selected,
                    residue,
                    inputs,
                    preparation_plans,
                    physical_lineage,
                    logical,
                ),
            ));
        }
    };
    let (draft_inputs, target_inputs, draft_pages, target_pages) = inputs.into_parts();
    if selected.len() != scheduled.member_count()
        || draft_pages.len() != selected.len()
        || target_pages.len() != selected.len()
    {
        engine.quarantine_m1_queue_rearm_failure();
        return Err(preparation_failure(
            M1AuthenticatedSpeculativeRolloverPrepareStageV1::Inputs,
            queue,
            (
                (
                    prior,
                    next,
                    reason,
                    scheduled,
                    selected,
                    residue,
                    draft_inputs,
                ),
                (
                    target_inputs,
                    draft_pages,
                    target_pages,
                    recipe,
                    preparation_plans,
                    physical_lineage,
                    logical,
                ),
            ),
        ));
    }
    let mut draft_reservations = Vec::new();
    let mut target_reservations = Vec::new();
    if draft_reservations
        .try_reserve_exact(selected.len())
        .is_err()
        || target_reservations
            .try_reserve_exact(selected.len())
            .is_err()
    {
        engine.quarantine_m1_queue_rearm_failure();
        return Err(preparation_failure(
            M1AuthenticatedSpeculativeRolloverPrepareStageV1::Inputs,
            queue,
            (
                (
                    prior,
                    next,
                    reason,
                    scheduled,
                    selected,
                    residue,
                    draft_inputs,
                ),
                (
                    target_inputs,
                    draft_pages,
                    target_pages,
                    recipe,
                    preparation_plans,
                    physical_lineage,
                    logical,
                ),
            ),
        ));
    }
    let mut draft_pages = draft_pages.into_iter();
    for (lane, cache) in selected.iter_mut().enumerate() {
        let pages = draft_pages.next().expect("draft page roster was checked");
        match cache.reserve_speculative_draft_round_write(
            cache.projection().request,
            next.target(),
            next.draft(),
            draft_inputs.context_lengths()[lane],
            scheduled.epoch(),
            pages,
        ) {
            Ok(reservation) => draft_reservations.push(reservation),
            Err(source) => {
                engine.quarantine_m1_queue_rearm_failure();
                return Err(preparation_failure(
                    M1AuthenticatedSpeculativeRolloverPrepareStageV1::DraftReservation,
                    queue,
                    (
                        (
                            source,
                            prior,
                            next,
                            reason,
                            scheduled,
                            selected,
                            residue,
                            draft_inputs,
                        ),
                        (
                            target_inputs,
                            draft_reservations,
                            draft_pages,
                            target_pages,
                            recipe,
                            preparation_plans,
                            physical_lineage,
                            logical,
                        ),
                    ),
                ));
            }
        }
    }
    let mut target_pages = target_pages.into_iter();
    for (lane, cache) in selected.iter_mut().enumerate() {
        let pages = target_pages.next().expect("target page roster was checked");
        match cache.reserve_step_write(
            cache.projection().request,
            ferric_spec::Qwen3ModelRole::Target8B,
            target_inputs.context_lengths()[lane],
            target_inputs.active_lengths()[lane],
            scheduled.epoch(),
            pages,
        ) {
            Ok(reservation) => target_reservations.push(reservation),
            Err(source) => {
                engine.quarantine_m1_queue_rearm_failure();
                return Err(preparation_failure(
                    M1AuthenticatedSpeculativeRolloverPrepareStageV1::TargetReservation,
                    queue,
                    (
                        (
                            source,
                            prior,
                            next,
                            reason,
                            scheduled,
                            selected,
                            residue,
                            draft_inputs,
                        ),
                        (
                            target_inputs,
                            draft_reservations,
                            target_reservations,
                            target_pages,
                            recipe,
                            preparation_plans,
                            physical_lineage,
                            logical,
                        ),
                    ),
                ));
            }
        }
    }
    let target = match crate::bind_m1_kv_workspace_table_v1(target_inputs, target_reservations) {
        Ok(table) => table,
        Err(source) => {
            engine.quarantine_m1_queue_rearm_failure();
            return Err(preparation_failure(
                M1AuthenticatedSpeculativeRolloverPrepareStageV1::TargetTable,
                queue,
                (
                    (source, prior, next, reason, scheduled, selected, residue),
                    (
                        draft_inputs,
                        draft_reservations,
                        recipe,
                        preparation_plans,
                        physical_lineage,
                        logical,
                    ),
                ),
            ));
        }
    };
    let draft = match crate::bind_m1_speculative_draft_kv_round_workspace_table_v1(
        next.target(),
        draft_inputs,
        draft_reservations,
    ) {
        Ok(table) => table,
        Err(source) => {
            engine.quarantine_m1_queue_rearm_failure();
            return Err(preparation_failure(
                M1AuthenticatedSpeculativeRolloverPrepareStageV1::DraftTable,
                queue,
                (
                    source,
                    prior,
                    next,
                    reason,
                    scheduled,
                    selected,
                    residue,
                    target,
                    recipe,
                    preparation_plans,
                    physical_lineage,
                    logical,
                ),
            ));
        }
    };
    let tables = M1FullStepKvWorkspaceTablesV1::SpeculativeRound {
        draft_decode: draft,
        target_speculative: target,
    };
    let prepared = match crate::prepare_m1_scheduled_workspace_images_v1(
        scheduled,
        runner,
        preparation_plans,
        tables,
    ) {
        Ok(prepared) => prepared,
        Err(source) => {
            engine.quarantine_m1_queue_rearm_failure();
            return Err(preparation_failure(
                M1AuthenticatedSpeculativeRolloverPrepareStageV1::Workspace,
                queue,
                (
                    source,
                    prior,
                    next,
                    reason,
                    selected,
                    residue,
                    recipe,
                    physical_lineage,
                    logical,
                ),
            ));
        }
    };
    let prepared = match prepared.retain_speculative_lineage(physical_lineage) {
        Ok(prepared) => prepared,
        Err(prepared) => {
            engine.quarantine_m1_queue_rearm_failure();
            return Err(preparation_failure(
                M1AuthenticatedSpeculativeRolloverPrepareStageV1::LineageAttachment,
                queue,
                (
                    prior, next, reason, selected, residue, recipe, prepared, logical,
                ),
            ));
        }
    };
    Ok(M1AuthenticatedPreparedSpeculativeRolloverV1 {
        prior,
        next,
        reason,
        queue,
        selected,
        residue,
        prepared,
        recipe,
        logical,
    })
}

/// Exact authenticated rollover submission stage.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedSpeculativeRolloverSubmissionStageV1 {
    Preflight,
    DraftWorkspace,
    TargetWorkspace,
    OutputActivation,
    BoundRows,
    PacketLowering,
    NativeRollover,
    QueueObservation,
    QueueSubmit,
}

/// A queue that was cleanly destroyed while closing a pre-rollover failure.
#[must_use = "authenticated release and retained Ferric inputs remain owned"]
#[derive(Debug)]
struct M1AuthenticatedSpeculativeRolloverClosedFailureV1 {
    release: Result<AuthenticatedServiceQueueReleaseV1, AuthenticatedServiceQueueReleaseFailureV1>,
    retained: Box<dyn fmt::Debug>,
}

/// Exact lower native rollover rejection or terminal program quarantine.
#[must_use = "native rollover failure must be torn down or retained"]
struct M1AuthenticatedSpeculativeNativeRolloverFailureV1<const N: usize> {
    source: AuthenticatedServiceQueueRetainedRolloverFailureV1<N>,
    retained: Box<dyn fmt::Debug>,
}

impl<const N: usize> fmt::Debug for M1AuthenticatedSpeculativeNativeRolloverFailureV1<N> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedSpeculativeNativeRolloverFailureV1")
            .field("source", &self.source)
            .field("retained", &self.retained)
            .finish()
    }
}

impl<const N: usize> M1AuthenticatedSpeculativeNativeRolloverFailureV1<N> {
    /// Destroys a retryable predecessor queue or retains an already terminal quarantine.
    fn close(self) -> M1AuthenticatedSpeculativeNativeRolloverClosureV1<N> {
        match self.source {
            AuthenticatedServiceQueueRetainedRolloverFailureV1::Program {
                error,
                queue,
                packets,
            } => M1AuthenticatedSpeculativeNativeRolloverClosureV1::Released(Box::new(
                M1AuthenticatedSpeculativeRolloverClosedFailureV1 {
                    release: queue.destroy_and_release(),
                    retained: Box::new((error, packets, self.retained)),
                },
            )),
            AuthenticatedServiceQueueRetainedRolloverFailureV1::QueueRejected {
                error,
                queue,
                packets,
            } => M1AuthenticatedSpeculativeNativeRolloverClosureV1::Released(Box::new(
                M1AuthenticatedSpeculativeRolloverClosedFailureV1 {
                    release: queue.destroy_and_release(),
                    retained: Box::new((error, packets, self.retained)),
                },
            )),
            source @ AuthenticatedServiceQueueRetainedRolloverFailureV1::Terminal { .. } => {
                M1AuthenticatedSpeculativeNativeRolloverClosureV1::Quarantined(Box::new((
                    source,
                    self.retained,
                )))
            }
        }
    }
}

/// Exhaustive close outcome for one native rollover failure.
#[must_use = "released or quarantined native custody remains retained"]
#[derive(Debug)]
enum M1AuthenticatedSpeculativeNativeRolloverClosureV1<const N: usize> {
    Released(Box<M1AuthenticatedSpeculativeRolloverClosedFailureV1>),
    Quarantined(
        Box<(
            AuthenticatedServiceQueueRetainedRolloverFailureV1<N>,
            Box<dyn fmt::Debug>,
        )>,
    ),
}

/// Terminal rollover submission failure without queue or retry authority.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedSpeculativeRolloverSubmissionFailureV1;
/// fn resubmit(failure: M1AuthenticatedSpeculativeRolloverSubmissionFailureV1) {
///     let _queue = failure.into_queue();
/// }
/// ```
#[must_use = "terminal rollover submission custody remains retained"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativeRolloverSubmissionFailureV1 {
    stage: M1AuthenticatedSpeculativeRolloverSubmissionStageV1,
    disposition: crate::M1AuthenticatedSpeculativeFailureDispositionV1,
}

impl M1AuthenticatedSpeculativeRolloverSubmissionFailureV1 {
    #[must_use]
    pub const fn stage(&self) -> M1AuthenticatedSpeculativeRolloverSubmissionStageV1 {
        self.stage
    }

    #[must_use = "the terminal disposition must remain observed"]
    pub const fn disposition(&self) -> &crate::M1AuthenticatedSpeculativeFailureDispositionV1 {
        &self.disposition
    }

    pub(crate) fn into_disposition(self) -> crate::M1AuthenticatedSpeculativeFailureDispositionV1 {
        self.disposition.retain(self.stage)
    }
}

/// Internal rollover submission custody pending mandatory terminal closure.
#[derive(Debug)]
enum PendingM1AuthenticatedSpeculativeRolloverSubmissionFailureV1 {
    Closed {
        stage: M1AuthenticatedSpeculativeRolloverSubmissionStageV1,
        source: Box<M1AuthenticatedSpeculativeRolloverClosedFailureV1>,
    },
    Quarantined {
        stage: M1AuthenticatedSpeculativeRolloverSubmissionStageV1,
        source: Box<AuthenticatedQuarantinedServiceQueueV1>,
        retained: Box<dyn fmt::Debug>,
    },
    NativeK4(
        Box<
            M1AuthenticatedSpeculativeNativeRolloverFailureV1<
                M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1,
            >,
        >,
    ),
    NativeK8(
        Box<
            M1AuthenticatedSpeculativeNativeRolloverFailureV1<
                M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1,
            >,
        >,
    ),
    NativeK16(
        Box<
            M1AuthenticatedSpeculativeNativeRolloverFailureV1<
                M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1,
            >,
        >,
    ),
    NativePaired(
        Box<
            M1AuthenticatedSpeculativeNativeRolloverFailureV1<
                { crate::M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1 },
            >,
        >,
    ),
    NativeTarget(
        Box<
            M1AuthenticatedSpeculativeNativeRolloverFailureV1<
                M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1,
            >,
        >,
    ),
    Observation {
        queue: Box<M1AuthenticatedPhysicalQueueSessionV1>,
        retained: Box<dyn fmt::Debug>,
    },
    Submit {
        source: Box<M1AuthenticatedPhysicalQueueSubmitFailureV1>,
        retained: Box<dyn fmt::Debug>,
    },
}

impl PendingM1AuthenticatedSpeculativeRolloverSubmissionFailureV1 {
    #[must_use]
    const fn stage(&self) -> M1AuthenticatedSpeculativeRolloverSubmissionStageV1 {
        match self {
            Self::Closed { stage, .. } | Self::Quarantined { stage, .. } => *stage,
            Self::NativeK4(_)
            | Self::NativeK8(_)
            | Self::NativeK16(_)
            | Self::NativePaired(_)
            | Self::NativeTarget(_) => {
                M1AuthenticatedSpeculativeRolloverSubmissionStageV1::NativeRollover
            }
            Self::Observation { .. } => {
                M1AuthenticatedSpeculativeRolloverSubmissionStageV1::QueueObservation
            }
            Self::Submit { .. } => M1AuthenticatedSpeculativeRolloverSubmissionStageV1::QueueSubmit,
        }
    }
}

fn close_pending_submission_failure<const C: usize>(
    engine: &mut Engine<C>,
    pending: PendingM1AuthenticatedSpeculativeRolloverSubmissionFailureV1,
) -> M1AuthenticatedSpeculativeRolloverSubmissionFailureV1 {
    use crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1;
    use crate::authenticated_speculative_executor::{
        quarantined_disposition, released_disposition,
    };

    engine.quarantine_m1_queue_rearm_failure();
    let stage = pending.stage();
    let disposition = match pending {
        PendingM1AuthenticatedSpeculativeRolloverSubmissionFailureV1::Closed { source, .. } => {
            let M1AuthenticatedSpeculativeRolloverClosedFailureV1 { release, retained } = *source;
            if release.is_ok() {
                released_disposition((release, retained))
            } else {
                quarantined_disposition((release, retained))
            }
        }
        PendingM1AuthenticatedSpeculativeRolloverSubmissionFailureV1::Quarantined {
            source,
            retained,
            ..
        } => quarantined_disposition((source, retained)),
        PendingM1AuthenticatedSpeculativeRolloverSubmissionFailureV1::NativeK4(source) => {
            disposition_from_native_closure(source.close())
        }
        PendingM1AuthenticatedSpeculativeRolloverSubmissionFailureV1::NativeK8(source) => {
            disposition_from_native_closure(source.close())
        }
        PendingM1AuthenticatedSpeculativeRolloverSubmissionFailureV1::NativeK16(source) => {
            disposition_from_native_closure(source.close())
        }
        PendingM1AuthenticatedSpeculativeRolloverSubmissionFailureV1::NativePaired(source) => {
            disposition_from_native_closure(source.close())
        }
        PendingM1AuthenticatedSpeculativeRolloverSubmissionFailureV1::NativeTarget(source) => {
            disposition_from_native_closure(source.close())
        }
        PendingM1AuthenticatedSpeculativeRolloverSubmissionFailureV1::Observation {
            queue,
            retained,
        } => match queue.close_unpublished() {
            M1AuthenticatedPhysicalQueueClosureV1::Released(released) => {
                released_disposition((released, retained))
            }
            M1AuthenticatedPhysicalQueueClosureV1::Quarantined(quarantined) => {
                quarantined_disposition((quarantined, retained))
            }
        },
        PendingM1AuthenticatedSpeculativeRolloverSubmissionFailureV1::Submit {
            source,
            retained,
        } => match source.close_without_authority(engine) {
            M1AuthenticatedPhysicalQueueClosureV1::Released(released) => {
                released_disposition((released, retained))
            }
            M1AuthenticatedPhysicalQueueClosureV1::Quarantined(quarantined) => {
                quarantined_disposition((quarantined, retained))
            }
        },
    };
    M1AuthenticatedSpeculativeRolloverSubmissionFailureV1 { stage, disposition }
}

fn disposition_from_native_closure<const N: usize>(
    closure: M1AuthenticatedSpeculativeNativeRolloverClosureV1<N>,
) -> crate::M1AuthenticatedSpeculativeFailureDispositionV1 {
    use crate::authenticated_speculative_executor::{
        quarantined_disposition, released_disposition,
    };

    match closure {
        M1AuthenticatedSpeculativeNativeRolloverClosureV1::Released(source) => {
            let M1AuthenticatedSpeculativeRolloverClosedFailureV1 { release, retained } = *source;
            if release.is_ok() {
                released_disposition((release, retained))
            } else {
                quarantined_disposition((release, retained))
            }
        }
        M1AuthenticatedSpeculativeNativeRolloverClosureV1::Quarantined(source) => {
            quarantined_disposition(source)
        }
    }
}

fn close_unbound<const C: usize>(
    engine: &mut Engine<C>,
    stage: M1AuthenticatedSpeculativeRolloverSubmissionStageV1,
    lower: AuthenticatedServiceQueueUnboundSessionV1,
    retained: impl fmt::Debug + 'static,
) -> PendingM1AuthenticatedSpeculativeRolloverSubmissionFailureV1 {
    engine.quarantine_m1_queue_rearm_failure();
    PendingM1AuthenticatedSpeculativeRolloverSubmissionFailureV1::Closed {
        stage,
        source: Box::new(M1AuthenticatedSpeculativeRolloverClosedFailureV1 {
            release: lower.destroy_and_release(),
            retained: Box::new(retained),
        }),
    }
}

#[allow(clippy::boxed_local)]
fn close_workspace_failure<const C: usize, const N: usize>(
    engine: &mut Engine<C>,
    stage: M1AuthenticatedSpeculativeRolloverSubmissionStageV1,
    failure: Box<crate::authenticated_queue_rearm::AuthenticatedWorkspaceReplacementFailureV1<N>>,
    retained: impl fmt::Debug + 'static,
) -> PendingM1AuthenticatedSpeculativeRolloverSubmissionFailureV1 {
    use crate::authenticated_queue_rearm::AuthenticatedWorkspaceReplacementFailureV1;
    use crate::step_workspace_subleases::M1AuthenticatedQueueReplacedWorkspaceBindingFailureV1;

    match *failure {
        AuthenticatedWorkspaceReplacementFailureV1::Update { failure, plan } => match failure {
            fe2o3_host::AuthenticatedServiceQueueDataUpdateFailureV1::Rejected { error, queue } => {
                close_unbound(engine, stage, *queue, (error, plan, retained))
            }
            fe2o3_host::AuthenticatedServiceQueueDataUpdateFailureV1::Quarantined {
                error,
                retained: queue,
            } => {
                engine.quarantine_m1_queue_rearm_failure();
                PendingM1AuthenticatedSpeculativeRolloverSubmissionFailureV1::Quarantined {
                    stage,
                    source: queue,
                    retained: Box::new((error, plan, retained)),
                }
            }
        },
        AuthenticatedWorkspaceReplacementFailureV1::Binding(failure) => match *failure {
            M1AuthenticatedQueueReplacedWorkspaceBindingFailureV1::Plan { failure, update } => {
                let (lower, subleases, ranges) = update.into_parts();
                close_unbound(engine, stage, lower, (failure, subleases, ranges, retained))
            }
            M1AuthenticatedQueueReplacedWorkspaceBindingFailureV1::ReturnedRange {
                plan,
                queue,
                subleases,
                ranges,
            } => close_unbound(engine, stage, *queue, (plan, subleases, ranges, retained)),
        },
    }
}

type M1AuthenticatedPrefillDraftWorkspaceV1 =
    BoundM1StepWorkspaceSubleases<M1_DRAFT_STEP_WORKSPACE_SUBLEASE_COUNT_V1>;
type M1AuthenticatedOrdinaryTargetWorkspaceV1 =
    BoundM1StepWorkspaceSubleases<M1_TARGET_STEP_WORKSPACE_SUBLEASE_COUNT_V1>;
type M1AuthenticatedTargetDecodeWorkspaceTransitionPartsV1 = (
    AuthenticatedServiceQueueUnboundSessionV1,
    Box<M1AuthenticatedPrefillDraftWorkspaceV1>,
    Box<M1AuthenticatedOrdinaryTargetWorkspaceV1>,
    Box<ferric_build::AddresslessM1StepWorkspacePlan>,
    Box<[u8]>,
);

/// Exact detached workspace inputs for the dormant authenticated
/// S1/T128-prefill to S1/C8192 target-decode transition.
#[must_use = "target-decode transition inputs retain the detached queue and both old workspaces"]
#[allow(dead_code)]
#[derive(Debug)]
pub(crate) struct M1AuthenticatedTargetDecodeWorkspaceTransitionInputsV1 {
    lower: AuthenticatedServiceQueueUnboundSessionV1,
    old_draft: Box<M1AuthenticatedPrefillDraftWorkspaceV1>,
    old_target: Box<M1AuthenticatedOrdinaryTargetWorkspaceV1>,
    target_plan: Box<ferric_build::AddresslessM1StepWorkspacePlan>,
    target_bytes: Box<[u8]>,
}

#[allow(dead_code)]
impl M1AuthenticatedTargetDecodeWorkspaceTransitionInputsV1 {
    pub(crate) fn new(
        lower: AuthenticatedServiceQueueUnboundSessionV1,
        old_draft: M1AuthenticatedPrefillDraftWorkspaceV1,
        old_target: M1AuthenticatedOrdinaryTargetWorkspaceV1,
        target_plan: ferric_build::AddresslessM1StepWorkspacePlan,
        target_bytes: Box<[u8]>,
    ) -> Self {
        Self {
            lower,
            old_draft: Box::new(old_draft),
            old_target: Box::new(old_target),
            target_plan: Box::new(target_plan),
            target_bytes,
        }
    }

    #[must_use = "all unmodified transition inputs remain linear"]
    pub(crate) fn into_parts(self) -> M1AuthenticatedTargetDecodeWorkspaceTransitionPartsV1 {
        (
            self.lower,
            self.old_draft,
            self.old_target,
            self.target_plan,
            self.target_bytes,
        )
    }
}

/// Detached queue after removing the obsolete draft workspace and replacing
/// the ordinary target workspace for exact target decode.
///
/// The retired draft witness remains attached so a later packet-publication
/// owner cannot silently discard the allocation generation that was removed.
#[must_use = "converted target-decode workspace custody must publish or close"]
#[allow(dead_code)]
#[derive(Debug)]
pub(crate) struct M1AuthenticatedTargetDecodeWorkspaceTransitionV1 {
    lower: AuthenticatedServiceQueueUnboundSessionV1,
    retired_draft: Box<M1AuthenticatedPrefillDraftWorkspaceV1>,
    target: Box<M1AuthenticatedOrdinaryTargetWorkspaceV1>,
    target_ranges: Box<[ServiceDeviceDispatchRangeV1; M1_TARGET_STEP_WORKSPACE_SUBLEASE_COUNT_V1]>,
}

#[allow(dead_code)]
impl M1AuthenticatedTargetDecodeWorkspaceTransitionV1 {
    #[must_use = "all converted workspace owners remain linear"]
    pub(crate) fn into_parts(
        self,
    ) -> (
        AuthenticatedServiceQueueUnboundSessionV1,
        Box<M1AuthenticatedPrefillDraftWorkspaceV1>,
        Box<M1AuthenticatedOrdinaryTargetWorkspaceV1>,
        Box<[ServiceDeviceDispatchRangeV1; M1_TARGET_STEP_WORKSPACE_SUBLEASE_COUNT_V1]>,
    ) {
        (
            self.lower,
            self.retired_draft,
            self.target,
            self.target_ranges,
        )
    }
}

/// Stable stage for exact target-decode workspace conversion.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum M1AuthenticatedTargetDecodeWorkspaceTransitionStageV1 {
    Preflight,
    WorkspaceContent,
    DraftWorkspaceRemoval,
    TargetWorkspaceReplacement,
}

/// Opaque post-mutation custody. No variant permits retry as the original
/// paired-prefill allocation set after draft removal.
#[must_use = "terminal target-decode workspace custody must remain retained"]
#[allow(dead_code)]
#[derive(Debug)]
pub(crate) enum M1AuthenticatedTargetDecodeWorkspaceTerminalCustodyV1 {
    DraftRemovalQuarantined {
        source: Box<fe2o3_service_host::ServiceQueueErrorV1>,
        lower: Box<AuthenticatedQuarantinedServiceQueueV1>,
        old_draft: Box<M1AuthenticatedPrefillDraftWorkspaceV1>,
        old_target: Box<M1AuthenticatedOrdinaryTargetWorkspaceV1>,
        target_plan: Box<ferric_build::AddresslessM1StepWorkspacePlan>,
        target_bytes: Box<[u8]>,
    },
    TargetReplacement {
        failure: Box<
            crate::authenticated_queue_rearm::AuthenticatedWorkspaceReplacementFailureV1<
                M1_TARGET_STEP_WORKSPACE_SUBLEASE_COUNT_V1,
            >,
        >,
        retired_draft: Box<M1AuthenticatedPrefillDraftWorkspaceV1>,
        old_target: Box<M1AuthenticatedOrdinaryTargetWorkspaceV1>,
    },
}

/// Pre-mutation retry or terminal post-mutation target workspace conversion failure.
#[must_use = "failed target-decode workspace conversion retains exact custody"]
#[allow(dead_code)]
#[derive(Debug)]
pub(crate) enum M1AuthenticatedTargetDecodeWorkspaceTransitionFailureV1 {
    Rejected {
        stage: M1AuthenticatedTargetDecodeWorkspaceTransitionStageV1,
        source: Option<Box<fe2o3_service_host::ServiceQueueErrorV1>>,
        retry: Box<M1AuthenticatedTargetDecodeWorkspaceTransitionInputsV1>,
    },
    Terminal {
        stage: M1AuthenticatedTargetDecodeWorkspaceTransitionStageV1,
        custody: Box<M1AuthenticatedTargetDecodeWorkspaceTerminalCustodyV1>,
    },
}

#[allow(dead_code)]
impl M1AuthenticatedTargetDecodeWorkspaceTransitionFailureV1 {
    #[must_use]
    pub(crate) const fn stage(&self) -> M1AuthenticatedTargetDecodeWorkspaceTransitionStageV1 {
        match self {
            Self::Rejected { stage, .. } | Self::Terminal { stage, .. } => *stage,
        }
    }

    #[must_use]
    pub(crate) const fn is_pre_mutation_retry(&self) -> bool {
        matches!(self, Self::Rejected { .. })
    }
}

enum M1AuthenticatedTargetDecodeRemovalRouteV1<Q, E, T> {
    Success(Q),
    Rejected { source: E, queue: Q },
    Quarantined { source: E, retained: T },
}

enum M1AuthenticatedTargetDecodeRemovalCustodyV1<Q, E, T, I> {
    Success { queue: Q, inputs: I },
    Rejected { source: E, queue: Q, inputs: I },
    Quarantined { source: E, retained: T, inputs: I },
}

fn retain_target_decode_removal_inputs<Q, E, T, I>(
    route: M1AuthenticatedTargetDecodeRemovalRouteV1<Q, E, T>,
    inputs: I,
) -> M1AuthenticatedTargetDecodeRemovalCustodyV1<Q, E, T, I> {
    match route {
        M1AuthenticatedTargetDecodeRemovalRouteV1::Success(queue) => {
            M1AuthenticatedTargetDecodeRemovalCustodyV1::Success { queue, inputs }
        }
        M1AuthenticatedTargetDecodeRemovalRouteV1::Rejected { source, queue } => {
            M1AuthenticatedTargetDecodeRemovalCustodyV1::Rejected {
                source,
                queue,
                inputs,
            }
        }
        M1AuthenticatedTargetDecodeRemovalRouteV1::Quarantined { source, retained } => {
            M1AuthenticatedTargetDecodeRemovalCustodyV1::Quarantined {
                source,
                retained,
                inputs,
            }
        }
    }
}

fn exact_target_decode_workspace_selections(
    old_draft: Qwen3PlanSelection,
    old_target: Qwen3PlanSelection,
    target: Qwen3PlanSelection,
) -> bool {
    old_draft
        == (Qwen3PlanSelection {
            role: Qwen3ModelRole::Draft06B,
            mode: Qwen3ExecutionMode::Prefill,
            bucket: Qwen3PlanBucket::PrefillS1T128,
        })
        && old_target
            == (Qwen3PlanSelection {
                role: Qwen3ModelRole::Target8B,
                mode: Qwen3ExecutionMode::Prefill,
                bucket: Qwen3PlanBucket::PrefillS1T128,
            })
        && target
            == (Qwen3PlanSelection {
                role: Qwen3ModelRole::Target8B,
                mode: Qwen3ExecutionMode::Decode,
                bucket: Qwen3PlanBucket::DecodeS1C8192,
            })
}

/// Converts only the device-workspace allocation set for the exact dormant
/// authenticated target-decode rollover.
///
/// All selection and image checks run before queue mutation. A generic removal
/// rejection returns the original queue and all inputs for retry. Once removal
/// succeeds, every later failure is terminal because the paired-prefill draft
/// allocation is no longer present.
#[allow(dead_code)]
pub(crate) fn transition_m1_authenticated_target_decode_workspaces_v1(
    inputs: M1AuthenticatedTargetDecodeWorkspaceTransitionInputsV1,
) -> Result<
    M1AuthenticatedTargetDecodeWorkspaceTransitionV1,
    Box<M1AuthenticatedTargetDecodeWorkspaceTransitionFailureV1>,
> {
    if !exact_target_decode_workspace_selections(
        inputs.old_draft.selection(),
        inputs.old_target.selection(),
        inputs.target_plan.selection(),
    ) {
        return Err(Box::new(
            M1AuthenticatedTargetDecodeWorkspaceTransitionFailureV1::Rejected {
                stage: M1AuthenticatedTargetDecodeWorkspaceTransitionStageV1::Preflight,
                source: None,
                retry: Box::new(inputs),
            },
        ));
    }
    let descriptor = match crate::authenticated_queue_rearm::descriptor(
        M1InitializedWorkspaceSlotV1::TargetOnlyTarget,
        &inputs.target_bytes,
    ) {
        Ok(descriptor) => descriptor,
        Err(()) => {
            return Err(Box::new(
                M1AuthenticatedTargetDecodeWorkspaceTransitionFailureV1::Rejected {
                    stage: M1AuthenticatedTargetDecodeWorkspaceTransitionStageV1::WorkspaceContent,
                    source: None,
                    retry: Box::new(inputs),
                },
            ));
        }
    };
    let M1AuthenticatedTargetDecodeWorkspaceTransitionInputsV1 {
        lower,
        old_draft,
        old_target,
        target_plan,
        target_bytes,
    } = inputs;
    let route = match lower.remove_partitioned_device_local::<
        DeviceWorkspaceRoleV1,
        M1_DRAFT_STEP_WORKSPACE_SUBLEASE_COUNT_V1,
    >(old_draft.replacement_subleases()) {
        Ok(lower) => M1AuthenticatedTargetDecodeRemovalRouteV1::Success(lower),
        Err(fe2o3_host::AuthenticatedServiceQueueDataUpdateFailureV1::Rejected {
            error,
            queue,
        }) => M1AuthenticatedTargetDecodeRemovalRouteV1::Rejected {
            source: error,
            queue: *queue,
        },
        Err(fe2o3_host::AuthenticatedServiceQueueDataUpdateFailureV1::Quarantined {
            error,
            retained,
        }) => M1AuthenticatedTargetDecodeRemovalRouteV1::Quarantined {
            source: error,
            retained,
        },
    };
    let (lower, old_draft, old_target, target_plan, target_bytes) =
        match retain_target_decode_removal_inputs(
            route,
            (old_draft, old_target, target_plan, target_bytes),
        ) {
            M1AuthenticatedTargetDecodeRemovalCustodyV1::Success {
                queue,
                inputs: (old_draft, old_target, target_plan, target_bytes),
            } => (queue, old_draft, old_target, target_plan, target_bytes),
            M1AuthenticatedTargetDecodeRemovalCustodyV1::Rejected {
                source,
                queue,
                inputs: (old_draft, old_target, target_plan, target_bytes),
            } => {
                return Err(Box::new(
                    M1AuthenticatedTargetDecodeWorkspaceTransitionFailureV1::Rejected {
                        stage: M1AuthenticatedTargetDecodeWorkspaceTransitionStageV1::DraftWorkspaceRemoval,
                        source: Some(source),
                        retry: Box::new(
                            M1AuthenticatedTargetDecodeWorkspaceTransitionInputsV1 {
                                lower: queue,
                                old_draft,
                                old_target,
                                target_plan,
                                target_bytes,
                            },
                        ),
                    },
                ));
            }
            M1AuthenticatedTargetDecodeRemovalCustodyV1::Quarantined {
                source,
                retained,
                inputs: (old_draft, old_target, target_plan, target_bytes),
            } => {
                return Err(Box::new(
                    M1AuthenticatedTargetDecodeWorkspaceTransitionFailureV1::Terminal {
                        stage: M1AuthenticatedTargetDecodeWorkspaceTransitionStageV1::DraftWorkspaceRemoval,
                        custody: Box::new(
                            M1AuthenticatedTargetDecodeWorkspaceTerminalCustodyV1::DraftRemovalQuarantined {
                                source,
                                lower: retained,
                                old_draft,
                                old_target,
                                target_plan,
                                target_bytes,
                            },
                        ),
                    },
                ));
            }
        };
    let (lower, target, target_ranges) =
        match crate::authenticated_queue_rearm::replace_authenticated_rollover_workspace(
            lower,
            &old_target,
            *target_plan,
            target_bytes,
            descriptor,
        ) {
            Ok(replaced) => replaced,
            Err(failure) => {
                return Err(Box::new(
                    M1AuthenticatedTargetDecodeWorkspaceTransitionFailureV1::Terminal {
                        stage: M1AuthenticatedTargetDecodeWorkspaceTransitionStageV1::TargetWorkspaceReplacement,
                        custody: Box::new(
                            M1AuthenticatedTargetDecodeWorkspaceTerminalCustodyV1::TargetReplacement {
                                failure,
                                retired_draft: old_draft,
                                old_target,
                            },
                        ),
                    },
                ));
            }
        };
    Ok(M1AuthenticatedTargetDecodeWorkspaceTransitionV1 {
        lower,
        retired_draft: old_draft,
        target: Box::new(target),
        target_ranges: Box::new(target_ranges),
    })
}

#[allow(clippy::result_large_err, clippy::too_many_arguments)]
fn rollover_case<const N: usize, F>(
    lower: AuthenticatedServiceQueueUnboundSessionV1,
    ring_bytes: u32,
    batch: crate::physical_fixed_batch::M1AuthenticatedQueuePacketBatchCaseV1<N>,
    witness: crate::authenticated_kernel_programs::M1AuthenticatedProgramCatalogWitnessV1,
    operations: crate::DeclaredOperationKernelPlan,
    step: crate::M1PrepublicationStepCustodyV1,
    _predecessor_generation: u64,
    wrap: F,
) -> Result<
    (
        M1AuthenticatedPhysicalQueueSessionV1,
        M1QueueRolloverObservationV1,
    ),
    M1AuthenticatedSpeculativeNativeRolloverFailureV1<N>,
>
where
    F: FnOnce(
        M1AuthenticatedPhysicalQueuePhaseCaseV1<fe2o3_host::AuthenticatedServiceQueueSessionV1<N>>,
    ) -> M1AuthenticatedPhysicalQueueSessionV1,
{
    let (packets, custody) = batch.into_parts();
    let rollover = match lower.rollover_retained(ring_bytes, packets) {
        Ok(rollover) => rollover,
        Err(source) => {
            return Err(M1AuthenticatedSpeculativeNativeRolloverFailureV1 {
                source,
                retained: Box::new((witness, operations, custody, step)),
            });
        }
    };
    let observation = M1QueueRolloverObservationV1::new(
        rollover.previous_queue_destroyed(),
        rollover.previous_dispatch_generation(),
        rollover.replacement_queue_observation(),
        rollover.replacement_dispatch_generation(),
    );
    Ok((
        wrap(M1AuthenticatedPhysicalQueuePhaseCaseV1::from_queue_rearm(
            rollover.into_queue(),
            witness,
            operations,
            custody,
            step,
        )),
        observation,
    ))
}

/// Structural target-decode inputs retained until authenticated scheduling.
///
/// These values carry no selection, currentness, dispatch, KV, workspace, or
/// queue authority. The authenticated bridge joins them to the live registry
/// batch, released completion, Engine-issued dispatch, and reselected cache.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedTargetDecodeServingInputsV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1AuthenticatedTargetDecodeServingInputsV1>();
/// ```
#[must_use = "target-decode serving inputs own linear page leases and workspace plans"]
#[derive(Debug)]
pub struct M1AuthenticatedTargetDecodeServingInputsV1 {
    target: ValidatedM1StepInputs,
    target_page_leases: Vec<DeviceKvPageLease>,
    preparation_plans: M1FullStepWorkspacePlans,
    recipe_plans: M1FullStepWorkspacePlans,
}

impl M1AuthenticatedTargetDecodeServingInputsV1 {
    #[must_use = "target-decode serving inputs remain linear"]
    pub const fn new(
        target: ValidatedM1StepInputs,
        target_page_leases: Vec<DeviceKvPageLease>,
        preparation_plans: M1FullStepWorkspacePlans,
        recipe_plans: M1FullStepWorkspacePlans,
    ) -> Self {
        Self {
            target,
            target_page_leases,
            preparation_plans,
            recipe_plans,
        }
    }
}

/// Stable authenticated target-decode scheduling rejection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedTargetDecodeScheduleErrorV1 {
    Action,
    Transition,
    EngineFaulted,
    Epoch,
    Roster,
    QueueShape,
    QueueSelection,
    Workspace,
    Output,
    MemberCustody,
    RequestNotReady,
    Inputs,
    Detach,
    ExactDispatch,
    CacheReselection,
}

/// Exact pre-detach target-decode owners retained for a safe retry.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedTargetDecodeSchedulePreDetachRetryV1;
/// fn extract(retry: M1AuthenticatedTargetDecodeSchedulePreDetachRetryV1) {
///     let _released = retry.into_released();
/// }
/// ```
#[must_use = "pre-detach target-decode retry custody remains linear"]
pub struct M1AuthenticatedTargetDecodeSchedulePreDetachRetryV1 {
    released: Box<M1AuthenticatedReleasedCompletedStepV1>,
    inputs: Box<M1AuthenticatedTargetDecodeServingInputsV1>,
}

impl fmt::Debug for M1AuthenticatedTargetDecodeSchedulePreDetachRetryV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedTargetDecodeSchedulePreDetachRetryV1")
            .field("retains_exact_inputs", &true)
            .finish()
    }
}

impl M1AuthenticatedTargetDecodeSchedulePreDetachRetryV1 {
    /// Retries the unchanged released queue and structural inputs against a
    /// caller-retained live registry batch.
    ///
    /// # Errors
    ///
    /// Returns renewed pre-detach retry custody or terminal closure custody.
    pub fn retry<const C: usize>(
        self,
        engine: &mut Engine<C>,
        batch: &M1ServingBatchPlanV1,
    ) -> Result<
        M1AuthenticatedScheduledTargetDecodeRolloverV1,
        M1AuthenticatedTargetDecodeScheduleFailureV1,
    > {
        schedule_m1_authenticated_target_decode_rollover_v1(
            engine,
            *self.released,
            batch,
            *self.inputs,
        )
    }

    #[must_use]
    pub const fn retains_exact_inputs(&self) -> bool {
        true
    }

    /// Cancels retry authority, faults the Engine, and closes queue custody.
    #[must_use = "cancelled target-decode custody remains retained"]
    pub fn cancel_and_close<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> crate::M1AuthenticatedSpeculativeFailureDispositionV1 {
        use crate::authenticated_speculative_executor::{
            quarantined_disposition, released_disposition,
        };

        match self.released.destroy_queue_and_retain_step(engine) {
            Ok(released) => released_disposition((released, self.inputs)),
            Err(quarantined) => quarantined_disposition((quarantined, self.inputs)),
        }
    }
}

/// Pre-detach retry or terminal target-decode scheduling custody.
#[must_use = "target-decode scheduling custody remains retained"]
pub enum M1AuthenticatedTargetDecodeScheduleFailureV1 {
    PreDetach {
        error: M1AuthenticatedTargetDecodeScheduleErrorV1,
        retry: Box<M1AuthenticatedTargetDecodeSchedulePreDetachRetryV1>,
    },
    Terminal {
        error: M1AuthenticatedTargetDecodeScheduleErrorV1,
        disposition: crate::M1AuthenticatedSpeculativeFailureDispositionV1,
    },
}

impl M1AuthenticatedTargetDecodeScheduleFailureV1 {
    #[must_use]
    pub const fn error(&self) -> M1AuthenticatedTargetDecodeScheduleErrorV1 {
        match self {
            Self::PreDetach { error, .. } | Self::Terminal { error, .. } => *error,
        }
    }

    #[must_use]
    pub const fn is_pre_detach_retry(&self) -> bool {
        matches!(self, Self::PreDetach { .. })
    }

    #[must_use = "terminal disposition remains observed when present"]
    pub const fn disposition(
        &self,
    ) -> Option<&crate::M1AuthenticatedSpeculativeFailureDispositionV1> {
        match self {
            Self::PreDetach { .. } => None,
            Self::Terminal { disposition, .. } => Some(disposition),
        }
    }
}

impl fmt::Debug for M1AuthenticatedTargetDecodeScheduleFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedTargetDecodeScheduleFailureV1")
            .field("error", &self.error())
            .field("pre_detach_retry", &self.is_pre_detach_retry())
            .field("terminal_disposition", &self.disposition())
            .finish()
    }
}

/// Detached authenticated predecessor, Engine dispatch, and reselected cache.
#[must_use = "scheduled target-decode rollover must be prepared or torn down"]
#[derive(Debug)]
pub struct M1AuthenticatedScheduledTargetDecodeRolloverV1 {
    prior: M1ServingPlanV1,
    next: M1ServingPlanV1,
    reason: M1ServingRolloverReasonV1,
    queue: M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1,
    scheduled: M1ScheduledDispatchV1,
    selected: Vec<ActiveDeviceKvCache>,
    residue: M1AuthenticatedSpeculativeRolloverResidueV1,
    inputs: M1AuthenticatedTargetDecodeServingInputsV1,
    phase_custody: M1AuthenticatedTargetRolloverReselectedCustodyV1,
}

impl M1AuthenticatedScheduledTargetDecodeRolloverV1 {
    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        &self.scheduled
    }

    #[must_use]
    pub const fn next_plan(&self) -> M1ServingPlanV1 {
        self.next
    }

    #[must_use]
    pub fn selected_cache(&self) -> DeviceKvCacheProjection {
        self.selected[0].projection()
    }

    /// Destroys the detached queue and preserves every scheduled owner.
    ///
    /// # Errors
    ///
    /// Returns authenticated lower quarantine with all Ferric custody.
    pub fn destroy_queue_and_retain_custody<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1AuthenticatedSpeculativeRolloverTeardownSuccessV1,
        Box<M1AuthenticatedSpeculativeRolloverTeardownFailureV1>,
    > {
        M1AuthenticatedSpeculativeRolloverDetachedFailureV1 {
            error: M1AuthenticatedSpeculativeRolloverScheduleErrorV1::Coordinator,
            queue: self.queue,
            retained: Box::new((
                self.prior,
                self.next,
                self.reason,
                self.scheduled,
                self.selected,
                self.residue,
                self.inputs,
            )),
        }
        .destroy_queue_and_retain_custody(engine)
    }
}

fn target_decode_serving_transition(
    batch: &M1ServingBatchPlanV1,
) -> Result<
    (M1ServingPlanV1, M1ServingPlanV1, M1ServingRolloverReasonV1),
    M1AuthenticatedTargetDecodeScheduleErrorV1,
> {
    let M1ServingQueueActionV1::QuiescentRollover {
        prior,
        next,
        reason,
    } = batch.action()
    else {
        return Err(M1AuthenticatedTargetDecodeScheduleErrorV1::Action);
    };
    if batch.plan() != next
        || admit_m1_target_decode_rollover_transition_v1(prior, next)
            .is_none_or(|admitted| admitted.reason() != reason)
    {
        return Err(M1AuthenticatedTargetDecodeScheduleErrorV1::Transition);
    }
    Ok((prior, next, reason))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct M1AuthenticatedTargetDecodeInputAssociationV1 {
    selection: Qwen3PlanSelection,
    live_lanes: u32,
    lane_selection: Qwen3PlanSelection,
    request: ferric_spec::RequestId,
    epoch: CompletionEpoch,
    anchor: ferric_spec::TokenId,
    position: u32,
    active: u32,
    context: u32,
    preparation_kind: M1FullStepWorkspaceInputKind,
    preparation_selection: Qwen3PlanSelection,
    recipe_kind: M1FullStepWorkspaceInputKind,
    recipe_selection: Qwen3PlanSelection,
}

fn target_decode_input_association(
    target: &ValidatedM1StepInputs,
    preparation_plans: &M1FullStepWorkspacePlans,
    recipe_plans: &M1FullStepWorkspacePlans,
) -> Option<M1AuthenticatedTargetDecodeInputAssociationV1> {
    let plan = target.lanes().first().and_then(Option::as_ref)?;
    Some(M1AuthenticatedTargetDecodeInputAssociationV1 {
        selection: target.selection(),
        live_lanes: target.live_lane_count(),
        lane_selection: plan.selection(),
        request: plan.request(),
        epoch: plan.completion_epoch(),
        anchor: *target.token_ids().first()?,
        position: *target.position_ids().first()?,
        active: *target.active_lengths().first()?,
        context: *target.context_lengths().first()?,
        preparation_kind: preparation_plans.kind(),
        preparation_selection: preparation_plans.target().selection(),
        recipe_kind: recipe_plans.kind(),
        recipe_selection: recipe_plans.target().selection(),
    })
}

fn expected_target_decode_input_association(
    next: M1ServingPlanV1,
    request: ferric_spec::RequestId,
    epoch: CompletionEpoch,
    anchor: ferric_spec::TokenId,
    committed: u32,
) -> M1AuthenticatedTargetDecodeInputAssociationV1 {
    M1AuthenticatedTargetDecodeInputAssociationV1 {
        selection: next.target(),
        live_lanes: 1,
        lane_selection: next.target(),
        request,
        epoch,
        anchor,
        position: committed,
        active: 1,
        context: committed,
        preparation_kind: M1FullStepWorkspaceInputKind::TargetOnly,
        preparation_selection: next.target(),
        recipe_kind: M1FullStepWorkspaceInputKind::TargetOnly,
        recipe_selection: next.target(),
    }
}

fn target_decode_input_association_matches(
    actual: Option<M1AuthenticatedTargetDecodeInputAssociationV1>,
    expected: M1AuthenticatedTargetDecodeInputAssociationV1,
) -> bool {
    actual == Some(expected)
}

#[allow(clippy::too_many_arguments)]
fn target_decode_inputs_match(
    target: &ValidatedM1StepInputs,
    preparation_plans: &M1FullStepWorkspacePlans,
    recipe_plans: &M1FullStepWorkspacePlans,
    next: M1ServingPlanV1,
    request: ferric_spec::RequestId,
    epoch: CompletionEpoch,
    anchor: ferric_spec::TokenId,
    committed: u32,
) -> bool {
    target_decode_input_association_matches(
        target_decode_input_association(target, preparation_plans, recipe_plans),
        expected_target_decode_input_association(next, request, epoch, anchor, committed),
    )
}

fn target_decode_schedule_preflight<const C: usize>(
    engine: &Engine<C>,
    released: &M1AuthenticatedReleasedCompletedStepV1,
    batch: &M1ServingBatchPlanV1,
    inputs: &M1AuthenticatedTargetDecodeServingInputsV1,
) -> Result<
    (M1ServingPlanV1, M1ServingPlanV1, M1ServingRolloverReasonV1),
    M1AuthenticatedTargetDecodeScheduleErrorV1,
> {
    let (prior, next, reason) = target_decode_serving_transition(batch)?;
    if engine.is_faulted() {
        return Err(M1AuthenticatedTargetDecodeScheduleErrorV1::EngineFaulted);
    }
    if released.checked().epoch().value().checked_add(1) != Some(batch.epoch().value()) {
        return Err(M1AuthenticatedTargetDecodeScheduleErrorV1::Epoch);
    }
    let [request] = batch.requests() else {
        return Err(M1AuthenticatedTargetDecodeScheduleErrorV1::Roster);
    };
    let [record] = released.checked().records() else {
        return Err(M1AuthenticatedTargetDecodeScheduleErrorV1::Roster);
    };
    let [M1ReleasedDeviceKvMemberV1::Active(cache)] = released.members() else {
        return Err(M1AuthenticatedTargetDecodeScheduleErrorV1::MemberCustody);
    };
    if released.logical_accepted_counts() != [1]
        || released.externally_published_counts() != [1]
        || released.queue().shape() != M1PhysicalFixedBatchShapeV1::PairedPrefill
    {
        return Err(M1AuthenticatedTargetDecodeScheduleErrorV1::QueueShape);
    }
    let old = released.queue().custody();
    if old.selection() != prior.target() || released.checked().selection() != prior.target() {
        return Err(M1AuthenticatedTargetDecodeScheduleErrorV1::QueueSelection);
    }
    if old.workspace_owners().kind() != M1FullStepWorkspaceInputKind::PairedPrefill
        || old.retains_retired_rollover_custody()
    {
        return Err(M1AuthenticatedTargetDecodeScheduleErrorV1::Workspace);
    }
    if !old
        .completion_output()
        .can_retarget_exact_s1_prefill_to_decode(next.target())
    {
        return Err(M1AuthenticatedTargetDecodeScheduleErrorV1::Output);
    }
    let projection = cache.projection();
    if record.record().request != *request
        || record.record().emitted_token_count != 1
        || projection.request != *request
        || projection.target.committed_tokens == 0
        || projection.target.resident_tokens != projection.target.committed_tokens
    {
        return Err(M1AuthenticatedTargetDecodeScheduleErrorV1::MemberCustody);
    }
    cache
        .preflight_quiescent_reselection(next.target(), next.draft_cache_selection())
        .map_err(|_| M1AuthenticatedTargetDecodeScheduleErrorV1::CacheReselection)?;
    if engine.state(*request) != Some(RequestState::Ready) {
        return Err(M1AuthenticatedTargetDecodeScheduleErrorV1::RequestNotReady);
    }
    if !target_decode_inputs_match(
        &inputs.target,
        &inputs.preparation_plans,
        &inputs.recipe_plans,
        next,
        *request,
        batch.epoch(),
        record.record().emitted_tokens[0],
        projection.target.committed_tokens,
    ) {
        return Err(M1AuthenticatedTargetDecodeScheduleErrorV1::Inputs);
    }
    Ok((prior, next, reason))
}

fn close_target_decode_schedule_detached<const C: usize>(
    engine: &mut Engine<C>,
    error: M1AuthenticatedTargetDecodeScheduleErrorV1,
    queue: M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1,
    retained: impl fmt::Debug + 'static,
) -> M1AuthenticatedTargetDecodeScheduleFailureV1 {
    use crate::authenticated_speculative_executor::{
        quarantined_disposition, released_disposition,
    };

    let detached = M1AuthenticatedSpeculativeRolloverDetachedFailureV1 {
        error: M1AuthenticatedSpeculativeRolloverScheduleErrorV1::Coordinator,
        queue,
        retained: Box::new((error, retained)),
    };
    let disposition = match detached.destroy_queue_and_retain_custody(engine) {
        Ok(released) => released_disposition(released),
        Err(quarantined) => quarantined_disposition(quarantined),
    };
    M1AuthenticatedTargetDecodeScheduleFailureV1::Terminal { error, disposition }
}

/// Authenticates, detaches, dispatches, and reselects one registry-selected
/// S1/T128 prefill to S1/C8192 target-decode transition.
///
/// Every pure rejection returns an opaque retry owner. Detachment or later
/// failure permanently faults the Engine and closes or quarantines the queue.
///
/// # Errors
///
/// Returns an opaque retry before detachment or terminal release/quarantine
/// custody after detachment begins.
pub fn schedule_m1_authenticated_target_decode_rollover_v1<const C: usize>(
    engine: &mut Engine<C>,
    released: M1AuthenticatedReleasedCompletedStepV1,
    batch: &M1ServingBatchPlanV1,
    inputs: M1AuthenticatedTargetDecodeServingInputsV1,
) -> Result<
    M1AuthenticatedScheduledTargetDecodeRolloverV1,
    M1AuthenticatedTargetDecodeScheduleFailureV1,
> {
    let (prior, next, reason) =
        match target_decode_schedule_preflight(engine, &released, batch, &inputs) {
            Ok(transition) => transition,
            Err(error) if error == M1AuthenticatedTargetDecodeScheduleErrorV1::EngineFaulted => {
                use crate::authenticated_speculative_executor::{
                    quarantined_disposition, released_disposition,
                };
                let disposition = match released.destroy_queue_and_retain_step(engine) {
                    Ok(released) => released_disposition((released, inputs)),
                    Err(quarantined) => quarantined_disposition((quarantined, inputs)),
                };
                return Err(M1AuthenticatedTargetDecodeScheduleFailureV1::Terminal {
                    error,
                    disposition,
                });
            }
            Err(error) => {
                return Err(M1AuthenticatedTargetDecodeScheduleFailureV1::PreDetach {
                    error,
                    retry: Box::new(M1AuthenticatedTargetDecodeSchedulePreDetachRetryV1 {
                        released: Box::new(released),
                        inputs: Box::new(inputs),
                    }),
                });
            }
        };
    let (
        queue,
        checked,
        mut members,
        logical_accepted_counts,
        externally_published_counts,
        release_counts,
        completed_members,
        total_released,
    ) = released.into_rearm_parts();
    let residue = M1AuthenticatedSpeculativeRolloverResidueV1 {
        checked,
        logical_accepted_counts,
        externally_published_counts,
        release_counts,
        completed_members,
        total_released,
        terminal: Vec::new(),
        history: crate::m1_queue_rearm::M1RearmRoundHistoryV1::Empty,
    };
    let queue = match queue.detach() {
        Ok(queue) => queue,
        Err(source) => {
            use crate::authenticated_speculative_executor::quarantined_disposition;
            engine.quarantine_m1_queue_rearm_failure();
            return Err(M1AuthenticatedTargetDecodeScheduleFailureV1::Terminal {
                error: M1AuthenticatedTargetDecodeScheduleErrorV1::Detach,
                disposition: quarantined_disposition((source, members, residue, inputs)),
            });
        }
    };
    let scheduled = match engine.dispatch_m1_exact_ready(batch.epoch(), batch.requests()) {
        Ok(scheduled) => scheduled,
        Err(source) => {
            return Err(close_target_decode_schedule_detached(
                engine,
                M1AuthenticatedTargetDecodeScheduleErrorV1::ExactDispatch,
                queue,
                (source, members, residue, inputs),
            ));
        }
    };
    let phase_custody = begin_m1_authenticated_target_rollover_scheduled_custody_v1();
    let mut selected = match members.pop() {
        Some(M1ReleasedDeviceKvMemberV1::Active(selected)) => selected,
        member => {
            return Err(close_target_decode_schedule_detached(
                engine,
                M1AuthenticatedTargetDecodeScheduleErrorV1::MemberCustody,
                queue,
                (member, members, scheduled, residue, inputs, phase_custody),
            ));
        }
    };
    if let Err(source) = selected.reselect_quiescent(next.target(), next.draft_cache_selection()) {
        return Err(close_target_decode_schedule_detached(
            engine,
            M1AuthenticatedTargetDecodeScheduleErrorV1::CacheReselection,
            queue,
            (source, selected, scheduled, residue, inputs, phase_custody),
        ));
    }
    let phase_custody =
        advance_m1_authenticated_target_rollover_reselected_custody_v1(phase_custody);
    Ok(M1AuthenticatedScheduledTargetDecodeRolloverV1 {
        prior,
        next,
        reason,
        queue,
        scheduled,
        selected: vec![selected],
        residue,
        inputs,
        phase_custody,
    })
}

/// Target-decode preparation failure shares the authenticated terminal custody
/// representation with speculative rollover preparation.
pub type M1AuthenticatedTargetDecodePrepareFailureV1 =
    M1AuthenticatedSpeculativeRolloverPrepareFailureV1;
/// Target-decode preparation uses the common authenticated preparation stages.
pub type M1AuthenticatedTargetDecodePrepareStageV1 =
    M1AuthenticatedSpeculativeRolloverPrepareStageV1;

/// Fully prepared authenticated target-decode rollover.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedPreparedTargetDecodeRolloverV1;
/// fn extract_queue(prepared: M1AuthenticatedPreparedTargetDecodeRolloverV1) {
///     let _queue = prepared.into_queue();
/// }
/// ```
#[must_use = "prepared target-decode rollover must be submitted"]
#[derive(Debug)]
pub struct M1AuthenticatedPreparedTargetDecodeRolloverV1 {
    prior: M1ServingPlanV1,
    next: M1ServingPlanV1,
    reason: M1ServingRolloverReasonV1,
    queue: M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1,
    selected: Vec<ActiveDeviceKvCache>,
    residue: M1AuthenticatedSpeculativeRolloverResidueV1,
    prepared: M1PreparedScheduledWorkspaceImagesV1,
    recipe: AddresslessM1PhysicalBufferRecipeV1,
    phase_custody: M1AuthenticatedTargetRolloverPreparedCustodyV1,
}

impl M1AuthenticatedPreparedTargetDecodeRolloverV1 {
    #[must_use]
    pub const fn next_plan(&self) -> M1ServingPlanV1 {
        self.next
    }

    #[must_use]
    pub const fn next_epoch(&self) -> CompletionEpoch {
        self.prepared.step().scheduled_dispatch().epoch()
    }
}

/// Reserves target KV writes and prepares target-only workspace images after
/// the authenticated queue is detached and the cache has been reselected.
///
/// # Errors
///
/// Returns only terminal clean-release or opaque quarantine custody after a
/// recipe, KV reservation, table binding, or workspace preparation failure.
pub fn prepare_m1_authenticated_target_decode_rollover_v1<const C: usize>(
    engine: &mut Engine<C>,
    scheduled: M1AuthenticatedScheduledTargetDecodeRolloverV1,
) -> Result<
    M1AuthenticatedPreparedTargetDecodeRolloverV1,
    Box<M1AuthenticatedTargetDecodePrepareFailureV1>,
> {
    let M1AuthenticatedScheduledTargetDecodeRolloverV1 {
        prior,
        next,
        reason,
        queue,
        scheduled,
        mut selected,
        residue,
        inputs,
        phase_custody,
    } = scheduled;
    let M1AuthenticatedTargetDecodeServingInputsV1 {
        target,
        target_page_leases,
        preparation_plans,
        recipe_plans,
    } = inputs;
    let recipe = match crate::runner::derive_physical_step_recipe(
        queue.operations(),
        M1StepDispatchIntent::TargetOnly(next.target()),
        recipe_plans,
    ) {
        M1PhysicalRunnerRecipeOutcomeV1::Prepared(recipe) => recipe,
        M1PhysicalRunnerRecipeOutcomeV1::Rejected(source) => {
            return Err(close_pending_preparation_failure(
                engine,
                preparation_failure(
                    M1AuthenticatedSpeculativeRolloverPrepareStageV1::Recipe,
                    queue,
                    (
                        source,
                        prior,
                        next,
                        reason,
                        scheduled,
                        selected,
                        residue,
                        target,
                        preparation_plans,
                        phase_custody,
                    ),
                ),
            ));
        }
    };
    let Some(cache) = selected.first_mut() else {
        return Err(close_pending_preparation_failure(
            engine,
            preparation_failure(
                M1AuthenticatedSpeculativeRolloverPrepareStageV1::Inputs,
                queue,
                (
                    prior,
                    next,
                    reason,
                    scheduled,
                    selected,
                    residue,
                    target,
                    target_page_leases,
                    recipe,
                    preparation_plans,
                    phase_custody,
                ),
            ),
        ));
    };
    let reservation = match cache.reserve_step_write(
        cache.projection().request,
        Qwen3ModelRole::Target8B,
        target.context_lengths()[0],
        target.active_lengths()[0],
        scheduled.epoch(),
        target_page_leases,
    ) {
        Ok(reservation) => reservation,
        Err(source) => {
            return Err(close_pending_preparation_failure(
                engine,
                preparation_failure(
                    M1AuthenticatedSpeculativeRolloverPrepareStageV1::TargetReservation,
                    queue,
                    (
                        source,
                        prior,
                        next,
                        reason,
                        scheduled,
                        selected,
                        residue,
                        target,
                        recipe,
                        preparation_plans,
                        phase_custody,
                    ),
                ),
            ));
        }
    };
    let target = match crate::bind_m1_kv_workspace_table_v1(target, vec![reservation]) {
        Ok(target) => target,
        Err(source) => {
            return Err(close_pending_preparation_failure(
                engine,
                preparation_failure(
                    M1AuthenticatedSpeculativeRolloverPrepareStageV1::TargetTable,
                    queue,
                    (
                        source,
                        prior,
                        next,
                        reason,
                        scheduled,
                        selected,
                        residue,
                        recipe,
                        preparation_plans,
                        phase_custody,
                    ),
                ),
            ));
        }
    };
    let prepared = match crate::prepare_m1_scheduled_workspace_images_v1(
        scheduled,
        queue.operations().runner(),
        preparation_plans,
        M1FullStepKvWorkspaceTablesV1::TargetOnly { target },
    ) {
        Ok(prepared) => prepared,
        Err(source) => {
            return Err(close_pending_preparation_failure(
                engine,
                preparation_failure(
                    M1AuthenticatedSpeculativeRolloverPrepareStageV1::Workspace,
                    queue,
                    (
                        source,
                        prior,
                        next,
                        reason,
                        selected,
                        residue,
                        recipe,
                        phase_custody,
                    ),
                ),
            ));
        }
    };
    let phase_custody = advance_m1_authenticated_target_rollover_prepared_custody_v1(phase_custody);
    Ok(M1AuthenticatedPreparedTargetDecodeRolloverV1 {
        prior,
        next,
        reason,
        queue,
        selected,
        residue,
        prepared,
        recipe,
        phase_custody,
    })
}

/// Terminal failure for the exact authenticated target-decode rollover.
///
/// The underlying custody representation is shared with authenticated
/// speculative rollover because both paths close the same generic queue states.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedTargetDecodeRolloverSubmissionFailureV1;
/// fn resubmit(failure: M1AuthenticatedTargetDecodeRolloverSubmissionFailureV1) {
///     let _queue = failure.into_queue();
/// }
/// ```
pub type M1AuthenticatedTargetDecodeRolloverSubmissionFailureV1 =
    M1AuthenticatedSpeculativeRolloverSubmissionFailureV1;

fn close_target_decode_unbound<const C: usize>(
    engine: &mut Engine<C>,
    stage: M1AuthenticatedSpeculativeRolloverSubmissionStageV1,
    lower: AuthenticatedServiceQueueUnboundSessionV1,
    retained: impl fmt::Debug + 'static,
) -> M1AuthenticatedTargetDecodeRolloverSubmissionFailureV1 {
    let pending = close_unbound(engine, stage, lower, retained);
    close_pending_submission_failure(engine, pending)
}

fn close_target_decode_workspace_failure<const C: usize, const N: usize>(
    engine: &mut Engine<C>,
    stage: M1AuthenticatedSpeculativeRolloverSubmissionStageV1,
    failure: Box<crate::authenticated_queue_rearm::AuthenticatedWorkspaceReplacementFailureV1<N>>,
    retained: impl fmt::Debug + 'static,
) -> M1AuthenticatedTargetDecodeRolloverSubmissionFailureV1 {
    let pending = close_workspace_failure(engine, stage, failure, retained);
    close_pending_submission_failure(engine, pending)
}

fn target_decode_submission_preflight<const C: usize>(
    engine: &Engine<C>,
    target: &M1AuthenticatedPreparedTargetDecodeRolloverV1,
) -> bool {
    let M1AuthenticatedPreparedTargetDecodeRolloverV1 {
        prior,
        next,
        reason,
        queue,
        selected,
        residue,
        prepared,
        recipe,
        phase_custody: _,
    } = target;
    let Some(transition) = admit_m1_target_decode_rollover_transition_v1(*prior, *next) else {
        return false;
    };
    let [cache] = selected.as_slice() else {
        return false;
    };
    let [record] = residue.checked.records() else {
        return false;
    };
    let scheduled = prepared.step().scheduled_dispatch();
    let old = queue.custody();
    let projection = cache.projection();
    transition.reason() == *reason
        && !engine.is_faulted()
        && queue.shape() == M1PhysicalFixedBatchShapeV1::PairedPrefill
        && old.selection() == prior.target()
        && residue.checked.selection() == prior.target()
        && old.workspace_owners().kind() == M1FullStepWorkspaceInputKind::PairedPrefill
        && !old.retains_retired_rollover_custody()
        && old
            .completion_output()
            .can_retarget_exact_s1_prefill_to_decode(next.target())
        && record.record().emitted_token_count == 1
        && residue.logical_accepted_counts.as_ref() == [1]
        && residue.externally_published_counts.as_ref() == [1]
        && residue.checked.epoch().value().checked_add(1) == Some(scheduled.epoch().value())
        && scheduled.member_count() == 1
        && scheduled.member(0) == Some(record.record().request)
        && projection.request == record.record().request
        && projection.target.committed_tokens != 0
        && projection.target.resident_tokens == projection.target.committed_tokens
        && projection.target_write_pending
        && !projection.draft_write_pending
        && prepared.kind() == M1FullStepWorkspaceInputKind::TargetOnly
        && prepared.step().kv_reservations().target_selection() == next.target()
        && prepared
            .step()
            .kv_reservations()
            .draft_allocation_id()
            .is_none()
        && prepared.step().kv_reservations().target_allocation_id()
            == old
                .partitioned_memory()
                .allocation_id(Qwen3ModelRole::Target8B)
        && prepared
            .step()
            .kv_reservations()
            .all_devices_match(old.device())
        && recipe.workspace_composition().workspace_plans() == prepared.plans()
        && recipe
            .workspace_composition()
            .dispatch_plan()
            .intent()
            .target_selection()
            == next.target()
        && !recipe.requires_future_materialization()
        && recipe.rows().len() == M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1
        && recipe.kernarg_recipe().images().len() == M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1
}

fn close_target_decode_detached_submission<const C: usize>(
    engine: &mut Engine<C>,
    queue: M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1,
    retained: impl fmt::Debug + 'static,
) -> M1AuthenticatedTargetDecodeRolloverSubmissionFailureV1 {
    use crate::authenticated_speculative_executor::{
        quarantined_disposition, released_disposition,
    };

    let detached = M1AuthenticatedSpeculativeRolloverDetachedFailureV1 {
        error: M1AuthenticatedSpeculativeRolloverScheduleErrorV1::Coordinator,
        queue,
        retained: Box::new(retained),
    };
    let disposition = match detached.destroy_queue_and_retain_custody(engine) {
        Ok(released) => released_disposition(released),
        Err(quarantined) => quarantined_disposition(quarantined),
    };
    M1AuthenticatedSpeculativeRolloverSubmissionFailureV1 {
        stage: M1AuthenticatedSpeculativeRolloverSubmissionStageV1::Preflight,
        disposition,
    }
}

/// Publishes the exact authenticated S1/T128 paired-prefill to S1/C8192
/// target-decode rollover selected and prepared by the authenticated bridge.
///
/// The caller supplies no selection or currentness authority. The opaque input
/// already joins the live registry transition, Engine-issued dispatch,
/// authenticated predecessor, reselected KV cache, and prepared target work.
///
/// # Errors
///
/// Returns terminal clean-release or quarantine custody when the registry,
/// scheduler, KV, workspace, packet, or native queue join fails.
///
/// # Panics
///
/// Panics only if a move-only owner changes its already-preflighted enum shape
/// while being consumed in the same call.
pub fn submit_m1_authenticated_target_decode_rollover_v1<const C: usize>(
    engine: &mut Engine<C>,
    target: M1AuthenticatedPreparedTargetDecodeRolloverV1,
    ring_bytes: u32,
) -> Result<
    M1AuthenticatedRearmedPublishedQueueV1,
    M1AuthenticatedTargetDecodeRolloverSubmissionFailureV1,
> {
    let preflight = target_decode_submission_preflight(engine, &target);
    let M1AuthenticatedPreparedTargetDecodeRolloverV1 {
        prior,
        next,
        reason,
        queue,
        selected,
        residue,
        prepared,
        recipe,
        phase_custody,
    } = target;
    if !preflight {
        return Err(close_target_decode_detached_submission(
            engine,
            queue,
            (
                prior,
                next,
                reason,
                selected,
                residue,
                prepared,
                recipe,
                phase_custody,
            ),
        ));
    }
    let _submit_entry_custody =
        establish_m1_authenticated_target_rollover_submit_entry_custody_v1(phase_custody);
    let (old_shape, lower, witness, operations, custody) = queue.into_rearm_parts();
    let predecessor_observation = lower.observation();
    let predecessor_generation = lower.detached_dispatch_generation();
    let device = custody.device();
    let crate::physical_fixed_batch::M1PhysicalQueueBatchRearmPartsV1 {
        catalog_id,
        selection: old_selection,
        physical_recipe: old_physical_recipe,
        workspace_composition: old_workspace_composition,
        workspace_owners,
        partitioned_memory,
        completion_output: prior_output,
        source_rows: old_source_rows,
        bound_rows: old_bound_rows,
        retired_rollover_custody,
    } = custody.into_rearm_parts();
    let (plans, images, step) = prepared.into_rearm_parts();
    let (old_draft, old_target, target_plan, target_bytes) = match (workspace_owners, plans, images)
    {
        (
            M1FullStepWorkspaceSubleaseOwners::PairedPrefill { draft, target },
            M1FullStepWorkspacePlans::TargetOnly {
                target: target_plan,
            },
            M1FullStepWorkspaceImagesV1::TargetOnly {
                target: target_bytes,
            },
        ) => (draft, target, target_plan, target_bytes),
        (workspace_owners, plans, images) => {
            return Err(close_target_decode_unbound(
                engine,
                M1AuthenticatedSpeculativeRolloverSubmissionStageV1::Preflight,
                lower,
                (
                    (
                        old_shape,
                        witness,
                        operations,
                        catalog_id,
                        old_selection,
                        old_physical_recipe,
                        old_workspace_composition,
                        workspace_owners,
                        partitioned_memory,
                        prior_output,
                    ),
                    (
                        old_source_rows,
                        old_bound_rows,
                        retired_rollover_custody,
                        plans,
                        images,
                        step,
                        selected,
                        residue,
                        recipe,
                    ),
                ),
            ));
        }
    };
    let transition = transition_m1_authenticated_target_decode_workspaces_v1(
        M1AuthenticatedTargetDecodeWorkspaceTransitionInputsV1::new(
            lower,
            *old_draft,
            *old_target,
            *target_plan,
            target_bytes,
        ),
    );
    let (lower, retired_draft, target, target_ranges) = match transition {
        Ok(transition) => transition.into_parts(),
        Err(failure) => match *failure {
            M1AuthenticatedTargetDecodeWorkspaceTransitionFailureV1::Rejected {
                stage,
                source,
                retry,
            } => {
                let (lower, old_draft, old_target, target_plan, target_bytes) =
                    retry.into_parts();
                return Err(close_target_decode_unbound(
                    engine,
                    M1AuthenticatedSpeculativeRolloverSubmissionStageV1::TargetWorkspace,
                    lower,
                    (
                        (
                            stage,
                            source,
                            old_draft,
                            old_target,
                            target_plan,
                            target_bytes,
                            witness,
                        ),
                        (
                            operations,
                            partitioned_memory,
                            prior_output,
                            step,
                            selected,
                            residue,
                            recipe,
                        ),
                    ),
                ));
            }
            M1AuthenticatedTargetDecodeWorkspaceTransitionFailureV1::Terminal {
                stage,
                custody,
            } => match *custody {
                M1AuthenticatedTargetDecodeWorkspaceTerminalCustodyV1::DraftRemovalQuarantined {
                    source,
                    lower,
                    old_draft,
                    old_target,
                    target_plan,
                    target_bytes,
                } => {
                    return Err(close_pending_submission_failure(
                        engine,
                        PendingM1AuthenticatedSpeculativeRolloverSubmissionFailureV1::Quarantined {
                            stage: M1AuthenticatedSpeculativeRolloverSubmissionStageV1::DraftWorkspace,
                            source: lower,
                            retained: Box::new((
                                (
                                    stage,
                                    source,
                                    old_draft,
                                    old_target,
                                    target_plan,
                                    target_bytes,
                                    witness,
                                ),
                                (
                                    operations,
                                    partitioned_memory,
                                    prior_output,
                                    step,
                                    selected,
                                    residue,
                                    recipe,
                                ),
                            )),
                        },
                    ));
                }
                M1AuthenticatedTargetDecodeWorkspaceTerminalCustodyV1::TargetReplacement {
                    failure,
                    retired_draft,
                    old_target,
                } => {
                    return Err(close_target_decode_workspace_failure(
                        engine,
                        M1AuthenticatedSpeculativeRolloverSubmissionStageV1::TargetWorkspace,
                        failure,
                        (
                            stage,
                            retired_draft,
                            old_target,
                            witness,
                            operations,
                            partitioned_memory,
                            prior_output,
                            step,
                            selected,
                            residue,
                            recipe,
                        ),
                    ));
                }
            },
        },
    };
    let completion_output = match prior_output.retarget_exact_s1_prefill_to_decode(next.target()) {
        Ok(output) => output,
        Err(prior_output) => {
            return Err(close_target_decode_unbound(
                engine,
                M1AuthenticatedSpeculativeRolloverSubmissionStageV1::OutputActivation,
                lower,
                (
                    retired_draft,
                    target,
                    target_ranges,
                    witness,
                    operations,
                    partitioned_memory,
                    prior_output,
                    step,
                    selected,
                    residue,
                    recipe,
                ),
            ));
        }
    };
    let mut workspace_ranges = Vec::new();
    if workspace_ranges
        .try_reserve_exact(target_ranges.len())
        .is_err()
    {
        return Err(close_target_decode_unbound(
            engine,
            M1AuthenticatedSpeculativeRolloverSubmissionStageV1::BoundRows,
            lower,
            (
                retired_draft,
                target,
                target_ranges,
                witness,
                operations,
                partitioned_memory,
                completion_output,
                step,
                selected,
                residue,
                recipe,
            ),
        ));
    }
    crate::m1_queue_rearm::append_workspace_ranges(
        &mut workspace_ranges,
        M1FullStepWorkspaceRole::Target,
        &target,
        *target_ranges,
    );
    let capture = crate::m1_queue_rearm::retained_host_capture_ranges(&completion_output)
        .expect("bare target rollover output was preflighted");
    let bound_rows = match crate::m1_queue_rearm::build_rollover_bound_rows(
        recipe.rows(),
        &old_source_rows,
        &old_bound_rows,
        recipe.workspace_composition(),
        &workspace_ranges,
        &capture,
    ) {
        Ok(rows) => rows,
        Err(()) => {
            return Err(close_target_decode_unbound(
                engine,
                M1AuthenticatedSpeculativeRolloverSubmissionStageV1::BoundRows,
                lower,
                (
                    retired_draft,
                    target,
                    workspace_ranges,
                    witness,
                    operations,
                    partitioned_memory,
                    completion_output,
                    step,
                    selected,
                    residue,
                    recipe,
                ),
            ));
        }
    };
    let custody = M1PhysicalQueueBatchCustodyV1::from_rearm_parts(
        crate::physical_fixed_batch::M1PhysicalQueueBatchRearmPartsV1 {
            catalog_id,
            selection: next.target(),
            physical_recipe: old_physical_recipe,
            workspace_composition: old_workspace_composition,
            workspace_owners: M1FullStepWorkspaceSubleaseOwners::target_only(*target),
            partitioned_memory,
            completion_output,
            source_rows: old_source_rows,
            bound_rows: old_bound_rows,
            retired_rollover_custody: Some(Box::new((retired_rollover_custody, retired_draft))),
        },
    );
    let batch = match crate::physical_fixed_batch::build_m1_authenticated_rollover_packet_batch_v1(
        &witness,
        &operations,
        recipe,
        bound_rows,
        custody,
    ) {
        Ok(crate::physical_fixed_batch::M1AuthenticatedQueuePacketBatchV1::TargetOnly(batch)) => {
            batch
        }
        Ok(batch) => {
            return Err(close_target_decode_unbound(
                engine,
                M1AuthenticatedSpeculativeRolloverSubmissionStageV1::PacketLowering,
                lower,
                (batch, witness, operations, step, selected, residue),
            ));
        }
        Err(source) => {
            return Err(close_target_decode_unbound(
                engine,
                M1AuthenticatedSpeculativeRolloverSubmissionStageV1::PacketLowering,
                lower,
                (source, witness, operations, step, selected, residue),
            ));
        }
    };
    let (queue, rollover) = match rollover_case(
        lower,
        ring_bytes,
        *batch,
        witness,
        operations,
        step,
        predecessor_generation,
        |case| M1AuthenticatedPhysicalQueueSessionV1::TargetOnly(Box::new(case)),
    ) {
        Ok(value) => value,
        Err(mut source) => {
            source.retained = Box::new((source.retained, selected, residue));
            return Err(close_pending_submission_failure(
                engine,
                PendingM1AuthenticatedSpeculativeRolloverSubmissionFailureV1::NativeTarget(
                    Box::new(source),
                ),
            ));
        }
    };
    if rollover.previous_dispatch_generation() != predecessor_generation
        || predecessor_generation
            .checked_add(1)
            .is_none_or(|next_generation| {
                rollover.replacement_dispatch_generation() != next_generation
            })
    {
        return Err(close_pending_submission_failure(
            engine,
            PendingM1AuthenticatedSpeculativeRolloverSubmissionFailureV1::Observation {
                queue: Box::new(queue),
                retained: Box::new((selected, residue, rollover)),
            },
        ));
    }
    let queue = match queue.submit() {
        Ok(queue) => queue,
        Err(source) => {
            return Err(close_pending_submission_failure(
                engine,
                PendingM1AuthenticatedSpeculativeRolloverSubmissionFailureV1::Submit {
                    source: Box::new(source),
                    retained: Box::new((selected, residue, rollover)),
                },
            ));
        }
    };
    let M1AuthenticatedSpeculativeRolloverResidueV1 {
        checked,
        logical_accepted_counts,
        externally_published_counts,
        release_counts,
        completed_members,
        total_released,
        terminal,
        history,
    } = residue;
    Ok(
        M1AuthenticatedRearmedPublishedQueueV1::from_authenticated_rollover(
            queue,
            selected,
            terminal,
            history,
            checked.epoch(),
            checked,
            logical_accepted_counts,
            externally_published_counts,
            release_counts,
            completed_members,
            total_released,
            predecessor_observation,
            device,
            rollover,
        ),
    )
}

/// Failure phase for the authenticated target-decode serving bridge.
#[must_use = "authenticated target-decode bridge custody remains retained"]
#[derive(Debug)]
pub enum M1AuthenticatedTargetDecodeServingFailureV1 {
    Schedule(M1AuthenticatedTargetDecodeScheduleFailureV1),
    Prepare(Box<M1AuthenticatedTargetDecodePrepareFailureV1>),
    Submit(M1AuthenticatedTargetDecodeRolloverSubmissionFailureV1),
}

/// Runs the live registry-selected authenticated target-decode transition from
/// released prefill custody through physical queue publication.
///
/// The registry batch is consumed only as a borrowed scheduler decision. It
/// cannot supply selection or currentness authority; those are joined from the
/// authenticated predecessor, Engine dispatch, and protected queue program.
///
/// # Errors
///
/// Returns exact pre-detach retry custody for pure scheduling rejection and
/// terminal release/quarantine custody for every later failure.
pub fn run_m1_authenticated_target_decode_serving_v1<const C: usize>(
    engine: &mut Engine<C>,
    released: M1AuthenticatedReleasedCompletedStepV1,
    batch: &M1ServingBatchPlanV1,
    inputs: M1AuthenticatedTargetDecodeServingInputsV1,
    ring_bytes: u32,
) -> Result<M1AuthenticatedRearmedPublishedQueueV1, M1AuthenticatedTargetDecodeServingFailureV1> {
    let scheduled =
        schedule_m1_authenticated_target_decode_rollover_v1(engine, released, batch, inputs)
            .map_err(M1AuthenticatedTargetDecodeServingFailureV1::Schedule)?;
    let prepared = prepare_m1_authenticated_target_decode_rollover_v1(engine, scheduled)
        .map_err(M1AuthenticatedTargetDecodeServingFailureV1::Prepare)?;
    submit_m1_authenticated_target_decode_rollover_v1(engine, prepared, ring_bytes)
        .map_err(M1AuthenticatedTargetDecodeServingFailureV1::Submit)
}

/// Replaces the authenticated paired-prefill queue with a prepared finite-
/// speculative generation, then attempts its exact publication.
///
/// # Errors
///
/// Every failure quarantines the Engine and consumes any unpublished queue.
/// The caller receives only clean release evidence or opaque quarantine.
pub fn submit_m1_authenticated_speculative_rollover_v1<const C: usize>(
    engine: &mut Engine<C>,
    prepared: M1AuthenticatedPreparedSpeculativeRolloverV1,
    ring_bytes: u32,
    queue_wait_timeout: crate::M1QueueWaitTimeoutV1,
) -> Result<
    crate::M1AuthenticatedSpeculativeRolloverPublishedV1,
    M1AuthenticatedSpeculativeRolloverSubmissionFailureV1,
> {
    submit_m1_authenticated_speculative_rollover_pending_v1(
        engine,
        prepared,
        ring_bytes,
        queue_wait_timeout,
    )
    .map_err(|failure| close_pending_submission_failure(engine, failure))
}

/// Maximum total speculative windows admitted by one bounded M1 service run.
///
/// The current completed window counts toward this bound, so a history of 19
/// predecessors rejects another transition rather than creating window 21.
pub const M1_MAX_AUTHENTICATED_SPECULATIVE_WINDOWS_V1: usize = 20;

/// Stable admission stage for authenticated all-terminal new-window work.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedSpeculativeNewWindowScheduleErrorV1 {
    ExecutorActive,
    EngineFaulted,
    Action,
    Transition,
    Epoch,
    Queue,
    TerminalRoster,
    ReplacementRoster,
    Input,
    RolloverIntent,
    WindowHistoryCapacity,
    HostAllocation,
    Detach,
    RequestReplacement,
    ExactDispatch,
}

#[derive(Debug)]
enum M1AuthenticatedSpeculativeNewWindowRetryStateV1 {
    Executor(crate::M1AuthenticatedSpeculativePhysicalExecutorV1),
    Completed(crate::authenticated_speculative_executor::M1AuthenticatedCompletedSpeculativeWindowHandoffV1),
}

/// Opaque exact owners returned by pure new-window admission rejection.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedSpeculativeNewWindowSchedulePreDetachRetryV1;
/// fn decompose(value: M1AuthenticatedSpeculativeNewWindowSchedulePreDetachRetryV1) {
///     let _ = value.into_parts();
/// }
/// ```
#[must_use = "the unchanged completed executor and successor inputs remain retryable"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativeNewWindowSchedulePreDetachRetryV1 {
    state: M1AuthenticatedSpeculativeNewWindowRetryStateV1,
    input: M1ServingQueuedPairedPrefillNewWindowV1,
    speculative_successor: M1ServingPlanV1,
    member_intents: Vec<M1AuthenticatedSpeculativeRolloverMemberIntentV1>,
    ring_bytes: u32,
    next_queue_wait_timeout: crate::M1QueueWaitTimeoutV1,
}

/// Pure retry rejection or terminal post-detachment closure.
#[must_use = "new-window failure retains all authenticated custody"]
#[derive(Debug)]
pub enum M1AuthenticatedSpeculativeNewWindowScheduleFailureV1 {
    PreDetach {
        error: M1AuthenticatedSpeculativeNewWindowScheduleErrorV1,
        retry: Box<M1AuthenticatedSpeculativeNewWindowSchedulePreDetachRetryV1>,
    },
    Terminal {
        error: M1AuthenticatedSpeculativeNewWindowScheduleErrorV1,
        disposition: crate::M1AuthenticatedSpeculativeFailureDispositionV1,
    },
}

impl M1AuthenticatedSpeculativeNewWindowScheduleFailureV1 {
    #[must_use]
    pub const fn error(&self) -> M1AuthenticatedSpeculativeNewWindowScheduleErrorV1 {
        match self {
            Self::PreDetach { error, .. } | Self::Terminal { error, .. } => *error,
        }
    }

    #[must_use]
    pub const fn is_pre_detach_retry(&self) -> bool {
        matches!(self, Self::PreDetach { .. })
    }

    #[must_use = "terminal release or quarantine remains retained when present"]
    pub const fn disposition(
        &self,
    ) -> Option<&crate::M1AuthenticatedSpeculativeFailureDispositionV1> {
        match self {
            Self::PreDetach { .. } => None,
            Self::Terminal { disposition, .. } => Some(disposition),
        }
    }
}

#[derive(Debug)]
struct M1AuthenticatedSpeculativeNewWindowResidueV1 {
    checked: crate::M1CheckedCompletionOutputV1,
    members: Vec<M1ReleasedDeviceKvMemberV1>,
    terminal: Vec<crate::M1ReleasedTerminalDeviceKvMemberV1>,
    logical_accepted_counts: Box<[u32]>,
    externally_published_counts: Box<[u32]>,
    release_counts: Box<[crate::M1CompletedKvPageReleaseCountsV1]>,
    completed_members: usize,
    total_released: usize,
    history: crate::m1_queue_rearm::M1RearmRoundHistoryV1,
}

/// Private continuation carried through normal authenticated paired-prefill
/// completion into the only admitted speculative-successor join.
#[derive(Debug)]
pub(crate) struct M1AuthenticatedSpeculativeNewWindowBridgeV1 {
    pub(crate) speculative_successor: M1ServingPlanV1,
    pub(crate) intent: M1AuthenticatedSpeculativeRolloverIntentV1,
    pub(crate) prior_windows: Vec<crate::authenticated_speculative_executor::M1AuthenticatedSpeculativeCompletedWindowHistoryV1>,
    pub(crate) next_queue_wait_timeout: crate::M1QueueWaitTimeoutV1,
}

/// Detached authenticated predecessor and exact fresh paired-prefill request.
#[must_use = "scheduled new-window custody must be prepared or explicitly closed"]
#[derive(Debug)]
pub struct M1AuthenticatedScheduledSpeculativeNewWindowV1 {
    prior: M1ServingPlanV1,
    next: M1ServingPlanV1,
    queue: M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1,
    scheduled: M1ScheduledDispatchV1,
    residue: M1AuthenticatedSpeculativeNewWindowResidueV1,
    binding: crate::M1ServingQueuedGenerationBindingV1,
    draft_prefill: ValidatedM1StepInputs,
    target_prefill: ValidatedM1StepInputs,
    preparation_plans: M1FullStepWorkspacePlans,
    recipe_plans: M1FullStepWorkspacePlans,
    speculative_successor: M1ServingPlanV1,
    member_intents: Vec<M1AuthenticatedSpeculativeRolloverMemberIntentV1>,
    prior_windows: Vec<crate::authenticated_speculative_executor::M1AuthenticatedSpeculativeCompletedWindowHistoryV1>,
    ring_bytes: u32,
    next_queue_wait_timeout: crate::M1QueueWaitTimeoutV1,
}

impl M1AuthenticatedScheduledSpeculativeNewWindowV1 {
    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        &self.scheduled
    }

    #[must_use]
    pub const fn next_plan(&self) -> M1ServingPlanV1 {
        self.next
    }

    #[must_use]
    pub const fn queue_wait_timeout(&self) -> crate::M1QueueWaitTimeoutV1 {
        self.next_queue_wait_timeout
    }

    #[must_use]
    pub fn prior_window_count(&self) -> usize {
        self.prior_windows.len()
    }
}

fn new_window_transition(
    batch: &M1ServingBatchPlanV1,
) -> Result<(M1ServingPlanV1, M1ServingPlanV1), M1AuthenticatedSpeculativeNewWindowScheduleErrorV1>
{
    let M1ServingQueueActionV1::QuiescentNewWindow { prior, next } = batch.action() else {
        return Err(M1AuthenticatedSpeculativeNewWindowScheduleErrorV1::Action);
    };
    if next != batch.plan()
        || !matches!(
            prior.shape(),
            M1PhysicalFixedBatchShapeV1::SpeculativeK4
                | M1PhysicalFixedBatchShapeV1::SpeculativeK8
                | M1PhysicalFixedBatchShapeV1::SpeculativeK16
        )
        || next.shape() != M1PhysicalFixedBatchShapeV1::PairedPrefill
    {
        return Err(M1AuthenticatedSpeculativeNewWindowScheduleErrorV1::Transition);
    }
    Ok((prior, next))
}

fn new_window_request_is_exact_successor(
    predecessor: ferric_spec::RequestId,
    replacement: ferric_spec::RequestId,
) -> bool {
    predecessor.slot() == replacement.slot()
        && predecessor.generation().checked_add(1) == Some(replacement.generation())
}

fn authenticated_new_window_terminal_predecessors_match<I, J>(
    current: I,
    lineage: J,
    replacements: &[ferric_spec::RequestId],
) -> bool
where
    I: Clone + ExactSizeIterator<Item = ferric_spec::RequestId>,
    J: Clone + ExactSizeIterator<Item = ferric_spec::RequestId>,
{
    if replacements.is_empty()
        || !current.clone().all(|predecessor| {
            replacements
                .iter()
                .copied()
                .any(|replacement| new_window_request_is_exact_successor(predecessor, replacement))
        })
    {
        return false;
    }
    replacements
        .iter()
        .copied()
        .enumerate()
        .all(|(lane, replacement)| {
            let Some(predecessor_generation) = replacement.generation().checked_sub(1) else {
                return false;
            };
            if predecessor_generation == 0
                || replacements[..lane]
                    .iter()
                    .any(|prior| prior.slot() == replacement.slot())
            {
                return false;
            }
            let predecessor =
                ferric_spec::RequestId::new(replacement.slot(), predecessor_generation);
            current
                .clone()
                .filter(|member| *member == predecessor)
                .count()
                + lineage
                    .clone()
                    .filter(|member| *member == predecessor)
                    .count()
                == 1
        })
}

fn new_window_preflight<const C: usize>(
    engine: &Engine<C>,
    executor: &crate::M1AuthenticatedSpeculativePhysicalExecutorV1,
    batch: &M1ServingBatchPlanV1,
    input: &M1ServingQueuedPairedPrefillNewWindowV1,
    speculative_successor: M1ServingPlanV1,
    member_intents: &[M1AuthenticatedSpeculativeRolloverMemberIntentV1],
) -> Result<(M1ServingPlanV1, M1ServingPlanV1), M1AuthenticatedSpeculativeNewWindowScheduleErrorV1>
{
    let (prior, next) = new_window_transition(batch)?;
    if !executor.is_complete() {
        return Err(M1AuthenticatedSpeculativeNewWindowScheduleErrorV1::ExecutorActive);
    }
    if engine.is_faulted() {
        return Err(M1AuthenticatedSpeculativeNewWindowScheduleErrorV1::EngineFaulted);
    }
    if !authenticated_new_window_transition_within_limit(executor.prior_window_count()) {
        return Err(M1AuthenticatedSpeculativeNewWindowScheduleErrorV1::WindowHistoryCapacity);
    }
    let released = executor.new_window_released_round();
    let current = released.current_released();
    if current.queue().shape() != prior.shape()
        || current.queue().custody().selection() != prior.target()
        || executor.selection() != prior.target()
        || released.parked_count() != 0
        || released.has_pending_new_window_bridge()
        || released.round_history_len() >= crate::M1_MAX_REARM_ROUND_HISTORY_V1
        || released.speculative_lineage_witness().is_err()
        || current
            .queue()
            .custody()
            .partitioned_memory()
            .finite_speculative_rollover_output_state()
            != crate::M1FiniteSpeculativeRolloverOutputPortfolioStateV1::Activated
    {
        return Err(M1AuthenticatedSpeculativeNewWindowScheduleErrorV1::Queue);
    }
    if current
        .checked()
        .epoch()
        .value()
        .checked_add(1)
        .map(CompletionEpoch::new)
        != Some(batch.epoch())
    {
        return Err(M1AuthenticatedSpeculativeNewWindowScheduleErrorV1::Epoch);
    }
    if current.members().is_empty()
        || current
            .members()
            .iter()
            .any(|member| !matches!(member, M1ReleasedDeviceKvMemberV1::Terminal(_)))
        || !authenticated_new_window_terminal_predecessors_match(
            current
                .members()
                .iter()
                .map(M1ReleasedDeviceKvMemberV1::request),
            released.terminal_lineage_requests(),
            batch.requests(),
        )
    {
        return Err(M1AuthenticatedSpeculativeNewWindowScheduleErrorV1::TerminalRoster);
    }
    if engine.live_count() != batch.requests().len()
        || batch
            .requests()
            .iter()
            .copied()
            .enumerate()
            .any(|(lane, replacement)| {
                batch.requests()[..lane]
                    .iter()
                    .any(|prior| prior.slot() == replacement.slot())
                    || replacement
                        .generation()
                        .checked_sub(1)
                        .is_none_or(|generation| {
                            generation == 0
                                || engine.state(ferric_spec::RequestId::new(
                                    replacement.slot(),
                                    generation,
                                )) != Some(RequestState::Retiring)
                        })
            })
    {
        return Err(M1AuthenticatedSpeculativeNewWindowScheduleErrorV1::ReplacementRoster);
    }
    if speculative_successor != prior
        || admit_m1_production_rollover_transition_v1(next, speculative_successor).is_none()
        || member_intents.is_empty()
        || member_intents.len() != batch.requests().len()
        || member_intents
            .iter()
            .zip(batch.requests())
            .any(|(member, request)| member.request() != *request)
    {
        return Err(M1AuthenticatedSpeculativeNewWindowScheduleErrorV1::RolloverIntent);
    }
    if !input.physical_inputs_match(batch)
        || !input.logical_runner_plan_identities_match(current.queue().operations().runner())
        || current
            .queue()
            .custody()
            .partitioned_memory()
            .preflight_new_window_pages(
                batch.requests(),
                Qwen3ModelRole::Draft06B,
                input.draft_prefill().active_lengths(),
            )
            .is_err()
        || current
            .queue()
            .custody()
            .partitioned_memory()
            .preflight_new_window_pages(
                batch.requests(),
                Qwen3ModelRole::Target8B,
                input.target_prefill().active_lengths(),
            )
            .is_err()
    {
        return Err(M1AuthenticatedSpeculativeNewWindowScheduleErrorV1::Input);
    }
    Ok((prior, next))
}

fn authenticated_new_window_transition_within_limit(prior_window_count: usize) -> bool {
    prior_window_count
        .checked_add(1)
        .is_some_and(|archived_windows| {
            archived_windows < M1_MAX_AUTHENTICATED_SPECULATIVE_WINDOWS_V1
        })
}

fn new_window_schedule_rejection(
    error: M1AuthenticatedSpeculativeNewWindowScheduleErrorV1,
    state: M1AuthenticatedSpeculativeNewWindowRetryStateV1,
    input: M1ServingQueuedPairedPrefillNewWindowV1,
    speculative_successor: M1ServingPlanV1,
    member_intents: Vec<M1AuthenticatedSpeculativeRolloverMemberIntentV1>,
    ring_bytes: u32,
    next_queue_wait_timeout: crate::M1QueueWaitTimeoutV1,
) -> M1AuthenticatedSpeculativeNewWindowScheduleFailureV1 {
    M1AuthenticatedSpeculativeNewWindowScheduleFailureV1::PreDetach {
        error,
        retry: Box::new(
            M1AuthenticatedSpeculativeNewWindowSchedulePreDetachRetryV1 {
                state,
                input,
                speculative_successor,
                member_intents,
                ring_bytes,
                next_queue_wait_timeout,
            },
        ),
    }
}

fn close_new_window_detached<const C: usize>(
    engine: &mut Engine<C>,
    error: M1AuthenticatedSpeculativeNewWindowScheduleErrorV1,
    queue: M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1,
    retained: impl fmt::Debug + 'static,
) -> M1AuthenticatedSpeculativeNewWindowScheduleFailureV1 {
    use crate::authenticated_speculative_executor::{
        quarantined_disposition, released_disposition,
    };
    engine.quarantine_m1_queue_rearm_failure();
    let detached = M1AuthenticatedSpeculativeRolloverDetachedFailureV1 {
        error: M1AuthenticatedSpeculativeRolloverScheduleErrorV1::ExactDispatch,
        queue,
        retained: Box::new(retained),
    };
    let disposition = match detached.destroy_queue_and_retain_custody(engine) {
        Ok(released) => released_disposition(released),
        Err(quarantined) => quarantined_disposition(quarantined),
    };
    M1AuthenticatedSpeculativeNewWindowScheduleFailureV1::Terminal { error, disposition }
}

/// Consumes one completed authenticated speculative executor into an exact
/// fresh paired-prefill schedule.
///
/// The next queue timeout is accepted here, before any publication-capable
/// owner exists, and remains frozen through retry and eventual publication.
///
/// # Errors
///
/// Pure admission rejection retains every unchanged owner. Detach-or-later
/// rejection destroys the queue or returns terminal quarantine and faults the
/// Engine.
///
/// # Panics
///
/// Panics only if the completed executor changes state after the preceding
/// pure preflight while it is held by this function.
#[allow(clippy::too_many_arguments)]
pub fn schedule_m1_authenticated_speculative_new_window_v1<const C: usize>(
    engine: &mut Engine<C>,
    executor: crate::M1AuthenticatedSpeculativePhysicalExecutorV1,
    batch: &M1ServingBatchPlanV1,
    input: M1ServingQueuedPairedPrefillNewWindowV1,
    speculative_successor: M1ServingPlanV1,
    member_intents: Vec<M1AuthenticatedSpeculativeRolloverMemberIntentV1>,
    ring_bytes: u32,
    next_queue_wait_timeout: crate::M1QueueWaitTimeoutV1,
) -> Result<
    M1AuthenticatedScheduledSpeculativeNewWindowV1,
    M1AuthenticatedSpeculativeNewWindowScheduleFailureV1,
> {
    let (prior, next) = match new_window_preflight(
        engine,
        &executor,
        batch,
        &input,
        speculative_successor,
        &member_intents,
    ) {
        Ok(transition) => transition,
        Err(error) => {
            return Err(new_window_schedule_rejection(
                error,
                M1AuthenticatedSpeculativeNewWindowRetryStateV1::Executor(executor),
                input,
                speculative_successor,
                member_intents,
                ring_bytes,
                next_queue_wait_timeout,
            ));
        }
    };
    let mut handoff = executor
        .into_completed_new_window_handoff()
        .expect("completed executor was checked in pure preflight");
    let mut replaced_lanes = Vec::new();
    if handoff.lineage.prior_windows.try_reserve_exact(1).is_err()
        || handoff
            .released
            .try_reserve_new_window_terminal_lineage()
            .is_err()
        || replaced_lanes
            .try_reserve_exact(batch.requests().len())
            .is_err()
    {
        return Err(new_window_schedule_rejection(
            M1AuthenticatedSpeculativeNewWindowScheduleErrorV1::HostAllocation,
            M1AuthenticatedSpeculativeNewWindowRetryStateV1::Completed(handoff),
            input,
            speculative_successor,
            member_intents,
            ring_bytes,
            next_queue_wait_timeout,
        ));
    }
    replaced_lanes.resize(batch.requests().len(), false);
    let (mut prior_windows, archived) =
        crate::authenticated_speculative_executor::M1AuthenticatedSpeculativeCompletedWindowHistoryV1::archive(
            handoff.coordinator,
            handoff.lineage,
            handoff.queue_wait_timeout,
        );
    prior_windows.push(archived);
    let released = handoff.released.into_authenticated_new_window_custody();
    let crate::authenticated_queue_rearm::M1AuthenticatedNewWindowReleasedCustodyV1 {
        queue,
        checked,
        members,
        terminal,
        logical_accepted_counts,
        externally_published_counts,
        release_counts,
        completed_members,
        total_released,
        history,
        new_window_bridge,
    } = released;
    debug_assert!(new_window_bridge.is_none());
    let residue = M1AuthenticatedSpeculativeNewWindowResidueV1 {
        checked,
        members,
        terminal,
        logical_accepted_counts,
        externally_published_counts,
        release_counts,
        completed_members,
        total_released,
        history,
    };
    let queue = match queue.detach() {
        Ok(queue) => queue,
        Err(source) => {
            engine.quarantine_m1_queue_rearm_failure();
            return Err(
                M1AuthenticatedSpeculativeNewWindowScheduleFailureV1::Terminal {
                    error: M1AuthenticatedSpeculativeNewWindowScheduleErrorV1::Detach,
                    disposition: crate::authenticated_speculative_executor::quarantined_disposition(
                        (
                            source,
                            residue,
                            input,
                            speculative_successor,
                            member_intents,
                            prior_windows,
                            ring_bytes,
                            next_queue_wait_timeout,
                        ),
                    ),
                },
            );
        }
    };
    let (binding, draft_prefill, target_prefill, preparation_plans, recipe_plans) =
        input.into_parts();
    for _ in 0..batch.requests().len() {
        let successor = match engine.reincarnate_next_retiring() {
            Ok(successor) => successor,
            result => {
                return Err(close_new_window_detached(
                    engine,
                    M1AuthenticatedSpeculativeNewWindowScheduleErrorV1::RequestReplacement,
                    queue,
                    (
                        result,
                        residue,
                        binding,
                        draft_prefill,
                        target_prefill,
                        preparation_plans,
                        recipe_plans,
                        speculative_successor,
                        member_intents,
                        prior_windows,
                        ring_bytes,
                        next_queue_wait_timeout,
                    ),
                ));
            }
        };
        let Some(lane) = batch
            .requests()
            .iter()
            .position(|replacement| *replacement == successor)
        else {
            return Err(close_new_window_detached(
                engine,
                M1AuthenticatedSpeculativeNewWindowScheduleErrorV1::RequestReplacement,
                queue,
                (
                    residue,
                    binding,
                    draft_prefill,
                    target_prefill,
                    preparation_plans,
                    recipe_plans,
                    speculative_successor,
                    member_intents,
                    prior_windows,
                    ring_bytes,
                    next_queue_wait_timeout,
                ),
            ));
        };
        if replaced_lanes[lane] || engine.append_tentative(successor, 1).is_err() {
            return Err(close_new_window_detached(
                engine,
                M1AuthenticatedSpeculativeNewWindowScheduleErrorV1::RequestReplacement,
                queue,
                (
                    residue,
                    binding,
                    draft_prefill,
                    target_prefill,
                    preparation_plans,
                    recipe_plans,
                    speculative_successor,
                    member_intents,
                    prior_windows,
                    ring_bytes,
                    next_queue_wait_timeout,
                ),
            ));
        }
        replaced_lanes[lane] = true;
    }
    let scheduled = match engine.dispatch_m1_exact_ready(batch.epoch(), batch.requests()) {
        Ok(scheduled) => scheduled,
        Err(source) => {
            return Err(close_new_window_detached(
                engine,
                M1AuthenticatedSpeculativeNewWindowScheduleErrorV1::ExactDispatch,
                queue,
                (
                    source,
                    residue,
                    binding,
                    draft_prefill,
                    target_prefill,
                    preparation_plans,
                    recipe_plans,
                    speculative_successor,
                    member_intents,
                    prior_windows,
                    ring_bytes,
                    next_queue_wait_timeout,
                ),
            ));
        }
    };
    Ok(M1AuthenticatedScheduledSpeculativeNewWindowV1 {
        prior,
        next,
        queue,
        scheduled,
        residue,
        binding,
        draft_prefill,
        target_prefill,
        preparation_plans,
        recipe_plans,
        speculative_successor,
        member_intents,
        prior_windows,
        ring_bytes,
        next_queue_wait_timeout,
    })
}

impl M1AuthenticatedSpeculativeNewWindowSchedulePreDetachRetryV1 {
    /// Repeats admission without exposing the completed executor or inputs.
    ///
    /// # Errors
    ///
    /// Returns renewed pre-detach custody on pure rejection, or a terminal
    /// clean-release/quarantine disposition once detachment begins.
    pub fn retry<const C: usize>(
        self,
        engine: &mut Engine<C>,
        batch: &M1ServingBatchPlanV1,
    ) -> Result<
        M1AuthenticatedScheduledSpeculativeNewWindowV1,
        M1AuthenticatedSpeculativeNewWindowScheduleFailureV1,
    > {
        match self.state {
            M1AuthenticatedSpeculativeNewWindowRetryStateV1::Executor(executor) => {
                schedule_m1_authenticated_speculative_new_window_v1(
                    engine,
                    executor,
                    batch,
                    self.input,
                    self.speculative_successor,
                    self.member_intents,
                    self.ring_bytes,
                    self.next_queue_wait_timeout,
                )
            }
            M1AuthenticatedSpeculativeNewWindowRetryStateV1::Completed(mut handoff) => {
                if handoff.lineage.prior_windows.try_reserve_exact(1).is_err()
                    || handoff
                        .released
                        .try_reserve_new_window_terminal_lineage()
                        .is_err()
                {
                    return Err(new_window_schedule_rejection(
                        M1AuthenticatedSpeculativeNewWindowScheduleErrorV1::HostAllocation,
                        M1AuthenticatedSpeculativeNewWindowRetryStateV1::Completed(handoff),
                        self.input,
                        self.speculative_successor,
                        self.member_intents,
                        self.ring_bytes,
                        self.next_queue_wait_timeout,
                    ));
                }
                // Allocation retry is the only state that reaches this arm;
                // rebuild the private executor solely to re-run pure admission.
                let executor = crate::M1AuthenticatedSpeculativePhysicalExecutorV1::from_completed_new_window_handoff(handoff);
                schedule_m1_authenticated_speculative_new_window_v1(
                    engine,
                    executor,
                    batch,
                    self.input,
                    self.speculative_successor,
                    self.member_intents,
                    self.ring_bytes,
                    self.next_queue_wait_timeout,
                )
            }
        }
    }

    /// Abandons this retry without exposing the completed executor or queued
    /// new-window inputs as independently reusable authority.
    #[must_use = "clean release or terminal quarantine remains retained"]
    pub fn cancel_and_close<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> crate::M1AuthenticatedSpeculativeFailureDispositionV1 {
        use crate::authenticated_speculative_executor::{
            quarantined_disposition, released_disposition,
        };

        engine.quarantine_m1_queue_rearm_failure();
        let executor = match self.state {
            M1AuthenticatedSpeculativeNewWindowRetryStateV1::Executor(executor) => executor,
            M1AuthenticatedSpeculativeNewWindowRetryStateV1::Completed(handoff) => {
                crate::M1AuthenticatedSpeculativePhysicalExecutorV1::from_completed_new_window_handoff(
                    handoff,
                )
            }
        };
        let retained = (
            self.input,
            self.speculative_successor,
            self.member_intents,
            self.ring_bytes,
            self.next_queue_wait_timeout,
        );
        match executor.destroy_queue_and_retain_state(engine) {
            Ok(released) => released_disposition((released, retained)),
            Err(quarantined) => quarantined_disposition((quarantined, retained)),
        }
    }
}

/// Recipe-checked authenticated paired-prefill new-window custody.
#[must_use = "prepared new-window custody must be submitted or closed"]
#[derive(Debug)]
pub struct M1AuthenticatedPreparedSpeculativeNewWindowV1 {
    prior: M1ServingPlanV1,
    next: M1ServingPlanV1,
    queue: M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1,
    scheduled: M1ScheduledDispatchV1,
    residue: M1AuthenticatedSpeculativeNewWindowResidueV1,
    binding: crate::M1ServingQueuedGenerationBindingV1,
    draft_prefill: ValidatedM1StepInputs,
    target_prefill: ValidatedM1StepInputs,
    preparation_plans: M1FullStepWorkspacePlans,
    recipe: AddresslessM1PhysicalBufferRecipeV1,
    speculative_successor: M1ServingPlanV1,
    member_intents: Vec<M1AuthenticatedSpeculativeRolloverMemberIntentV1>,
    prior_windows: Vec<crate::authenticated_speculative_executor::M1AuthenticatedSpeculativeCompletedWindowHistoryV1>,
    ring_bytes: u32,
    next_queue_wait_timeout: crate::M1QueueWaitTimeoutV1,
}

impl M1AuthenticatedPreparedSpeculativeNewWindowV1 {
    pub(crate) fn cancel_and_close<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> crate::M1AuthenticatedSpeculativeFailureDispositionV1 {
        let Self {
            prior,
            next,
            queue,
            scheduled,
            residue,
            binding,
            draft_prefill,
            target_prefill,
            preparation_plans,
            recipe,
            speculative_successor,
            member_intents,
            prior_windows,
            ring_bytes,
            next_queue_wait_timeout,
        } = self;
        close_new_window_prepared_detached(
            engine,
            queue,
            (
                prior,
                next,
                scheduled,
                residue,
                binding,
                draft_prefill,
                target_prefill,
                preparation_plans,
                recipe,
                speculative_successor,
                member_intents,
                prior_windows,
                ring_bytes,
                next_queue_wait_timeout,
            ),
        )
        .into_disposition()
    }
}

/// Opaque paired-prefill publication carrying its exact frozen wait budget.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedSpeculativeNewWindowPublishedV1;
/// fn separate(value: M1AuthenticatedSpeculativeNewWindowPublishedV1) {
///     let _ = value.into_parts();
/// }
/// ```
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedSpeculativeNewWindowPublishedV1;
/// fn bypass_frozen_wait(value: M1AuthenticatedSpeculativeNewWindowPublishedV1) {
///     let _ = value.wait();
/// }
/// ```
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedSpeculativeNewWindowPublishedV1;
/// fn replace_frozen_timeout(value: M1AuthenticatedSpeculativeNewWindowPublishedV1) {
///     let _ = value.wait_for(1);
/// }
/// ```
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedSpeculativeNewWindowPublishedV1;
/// fn bypass_exact_settlement(value: M1AuthenticatedSpeculativeNewWindowPublishedV1) {
///     let _ = value.complete(Vec::new());
/// }
/// ```
#[must_use = "published new-window custody must complete with its frozen deadline"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativeNewWindowPublishedV1 {
    published: M1AuthenticatedRearmedPublishedQueueV1,
    queue_wait_timeout: crate::M1QueueWaitTimeoutV1,
}

/// Exact paired-prefill member observation exposed without copied wire bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1AuthenticatedSpeculativeNewWindowObservedMemberV1 {
    request: ferric_spec::RequestId,
    emitted_token: ferric_spec::TokenId,
}

impl M1AuthenticatedSpeculativeNewWindowObservedMemberV1 {
    #[must_use]
    pub const fn request(self) -> ferric_spec::RequestId {
        self.request
    }

    #[must_use]
    pub const fn emitted_token(self) -> ferric_spec::TokenId {
        self.emitted_token
    }
}

/// Ordered post-prefill disposition accepted by the exact new-window join.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedSpeculativeNewWindowMemberDispositionV1 {
    Continue,
    Retire,
}

/// Stable stage-local observation failure class.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedSpeculativeNewWindowObservationErrorV1 {
    Deadline,
    Wait,
    Recycle,
    Readback,
    Selection,
    MemberCount,
    Member { lane: usize },
}

#[derive(Debug)]
enum M1AuthenticatedSpeculativeNewWindowObservationFailureStateV1 {
    Deadline(crate::M1AuthenticatedSpeculativeFailureDispositionV1),
    Progress(Box<crate::M1AuthenticatedRearmedQueueProgressFailureV1>),
    Readback(Box<crate::M1AuthenticatedRearmedReadbackFailureV1>),
    Observed(Box<crate::M1AuthenticatedRearmedObservedCompletionOutputV1>),
}

/// Opaque observation failure retaining all authenticated queue custody.
#[must_use = "new-window observation failure must be retried or closed"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativeNewWindowObservationFailureV1 {
    error: M1AuthenticatedSpeculativeNewWindowObservationErrorV1,
    state: M1AuthenticatedSpeculativeNewWindowObservationFailureStateV1,
}

impl M1AuthenticatedSpeculativeNewWindowObservationFailureV1 {
    #[must_use]
    pub const fn error(&self) -> M1AuthenticatedSpeculativeNewWindowObservationErrorV1 {
        self.error
    }

    /// Retries only the unchanged completed-copy observation rejection.
    ///
    /// # Errors
    ///
    /// Returns the same opaque failure when this phase is not retryable or the
    /// exact retained readback fails observation again.
    pub fn retry(
        self,
    ) -> Result<
        M1AuthenticatedSpeculativeNewWindowObservedV1,
        M1AuthenticatedSpeculativeNewWindowObservationFailureV1,
    > {
        match self.state {
            M1AuthenticatedSpeculativeNewWindowObservationFailureStateV1::Readback(source) => {
                match source.retry_observation() {
                    Ok(observed) => authenticated_new_window_observed(observed),
                    Err(source) => Err(Self {
                        error: M1AuthenticatedSpeculativeNewWindowObservationErrorV1::Readback,
                        state:
                            M1AuthenticatedSpeculativeNewWindowObservationFailureStateV1::Readback(
                                source,
                            ),
                    }),
                }
            }
            state => Err(Self {
                error: self.error,
                state,
            }),
        }
    }

    /// Abandons observation without releasing generic phase owners.
    #[must_use = "clean release or terminal quarantine remains retained"]
    pub fn cancel_and_close<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> crate::M1AuthenticatedSpeculativeFailureDispositionV1 {
        use crate::authenticated_speculative_executor::{
            quarantined_disposition, released_disposition,
        };
        engine.quarantine_m1_queue_rearm_failure();
        match self.state {
            M1AuthenticatedSpeculativeNewWindowObservationFailureStateV1::Deadline(retained) => {
                retained.retain(self.error)
            }
            M1AuthenticatedSpeculativeNewWindowObservationFailureStateV1::Progress(source) => {
                quarantined_disposition((self.error, source))
            }
            M1AuthenticatedSpeculativeNewWindowObservationFailureStateV1::Readback(source) => {
                match source.destroy_queue_and_retain_custody(engine) {
                    Ok(released) => released_disposition((self.error, released)),
                    Err(quarantined) => quarantined_disposition((self.error, quarantined)),
                }
            }
            M1AuthenticatedSpeculativeNewWindowObservationFailureStateV1::Observed(observed) => {
                match (*observed).check_completion(&[]) {
                    Ok(readback) => match readback.destroy_queue_and_retain_custody(engine) {
                        Ok(released) => released_disposition((self.error, released)),
                        Err(quarantined) => quarantined_disposition((self.error, quarantined)),
                    },
                    Err(source) => match source.destroy_queue_and_retain_custody(engine) {
                        Ok(released) => released_disposition((self.error, released)),
                        Err(quarantined) => quarantined_disposition((self.error, quarantined)),
                    },
                }
            }
        }
    }
}

/// Structurally observed paired-prefill output with no public wire-image API.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedSpeculativeNewWindowObservedV1;
/// fn expose_wire_image(value: &M1AuthenticatedSpeculativeNewWindowObservedV1) {
///     let _ = value.image();
/// }
/// ```
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedSpeculativeNewWindowObservedV1;
/// fn expose_raw_bytes(value: &M1AuthenticatedSpeculativeNewWindowObservedV1) {
///     let _ = value.raw_bytes();
/// }
/// ```
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedSpeculativeNewWindowObservedV1;
/// fn separate(value: M1AuthenticatedSpeculativeNewWindowObservedV1) {
///     let _ = value.into_parts();
/// }
/// ```
#[must_use = "observed new-window output must be settled or closed"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativeNewWindowObservedV1 {
    observed: crate::M1AuthenticatedRearmedObservedCompletionOutputV1,
    members: Box<
        [Option<M1AuthenticatedSpeculativeNewWindowObservedMemberV1>;
            ferric_spec::M1_MAX_ACTIVE_SEQUENCES as usize],
    >,
    member_count: usize,
    selection: Qwen3PlanSelection,
    epoch: CompletionEpoch,
}

impl M1AuthenticatedSpeculativeNewWindowObservedV1 {
    pub(crate) fn cancel_and_close<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> crate::M1AuthenticatedSpeculativeFailureDispositionV1 {
        use crate::authenticated_speculative_executor::{
            quarantined_disposition, released_disposition,
        };

        engine.quarantine_m1_queue_rearm_failure();
        let retained = (self.members, self.member_count, self.selection, self.epoch);
        match self.observed.check_completion(&[]) {
            Ok(readback) => match readback.destroy_queue_and_retain_custody(engine) {
                Ok(released) => released_disposition((released, retained)),
                Err(quarantined) => quarantined_disposition((quarantined, retained)),
            },
            Err(source) => match source.destroy_queue_and_retain_custody(engine) {
                Ok(released) => released_disposition((released, retained)),
                Err(quarantined) => quarantined_disposition((quarantined, retained)),
            },
        }
    }

    #[must_use]
    pub const fn member_count(&self) -> usize {
        self.member_count
    }

    #[must_use]
    pub const fn selection(&self) -> Qwen3PlanSelection {
        self.selection
    }

    #[must_use]
    pub const fn epoch(&self) -> CompletionEpoch {
        self.epoch
    }

    #[must_use]
    pub fn member(
        &self,
        lane: usize,
    ) -> Option<M1AuthenticatedSpeculativeNewWindowObservedMemberV1> {
        self.members.get(lane).copied().flatten()
    }

    /// Derives exact direct-final-row semantics from the retained observation,
    /// settles ordered KV custody, and releases the completed round.
    ///
    /// # Errors
    ///
    /// Returns opaque settlement custody on disposition, semantic, completion,
    /// page-release, host-allocation, or released-roster rejection.
    pub fn settle<const C: usize>(
        self,
        engine: &mut Engine<C>,
        dispositions: Vec<M1AuthenticatedSpeculativeNewWindowMemberDispositionV1>,
    ) -> Result<
        M1AuthenticatedSpeculativeNewWindowReleasedV1,
        M1AuthenticatedSpeculativeNewWindowSettlementFailureV1,
    > {
        self.settle_with_deadline(engine, dispositions, |_| false)
    }

    pub(crate) fn settle_with_deadline<const C: usize>(
        self,
        engine: &mut Engine<C>,
        dispositions: Vec<M1AuthenticatedSpeculativeNewWindowMemberDispositionV1>,
        deadline_expired: impl FnMut(
            crate::authenticated_resident_session::M1AuthenticatedResidentDeadlineBoundaryV1,
        ) -> bool,
    ) -> Result<
        M1AuthenticatedSpeculativeNewWindowReleasedV1,
        M1AuthenticatedSpeculativeNewWindowSettlementFailureV1,
    > {
        settle_authenticated_speculative_new_window_with_deadline(
            engine,
            self,
            dispositions,
            deadline_expired,
        )
    }
}

/// Stable settlement failure class without generic completion custody.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedSpeculativeNewWindowSettlementErrorV1 {
    Deadline,
    DispositionCount { expected: usize, actual: usize },
    HostAllocation,
    SemanticJoin,
    CompletionPreflight,
    CompletionRejected,
    PageRelease,
    ReleasedRoster,
}

#[derive(Debug)]
enum M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1 {
    DeadlineObserved(
        Box<(
            crate::M1AuthenticatedRearmedObservedCompletionOutputV1,
            crate::authenticated_resident_session::M1AuthenticatedResidentDeadlineBoundaryV1,
        )>,
    ),
    DeadlineReadback(
        Box<(
            crate::M1AuthenticatedRearmedCompletedReadbackV1,
            ArrayVec<
                crate::M1DeviceKvCompletionDispositionV1,
                { ferric_spec::M1_MAX_ACTIVE_SEQUENCES as usize },
            >,
            crate::authenticated_resident_session::M1AuthenticatedResidentDeadlineBoundaryV1,
        )>,
    ),
    DeadlineCompletion(
        Box<(
            crate::M1AuthenticatedRearmedCompletionOutcomeV1,
            crate::authenticated_resident_session::M1AuthenticatedResidentDeadlineBoundaryV1,
        )>,
    ),
    DeadlineRelease(
        Box<(
            crate::M1AuthenticatedRearmedRoundReleaseOutcomeV1,
            crate::authenticated_resident_session::M1AuthenticatedResidentDeadlineBoundaryV1,
        )>,
    ),
    Observed(Box<crate::M1AuthenticatedRearmedObservedCompletionOutputV1>),
    Join(Box<crate::M1AuthenticatedRearmedReadbackFailureV1>),
    CompletionPreflight(Box<crate::M1AuthenticatedRearmedCompletionPreflightFailureV1>),
    Completion(Box<crate::M1AuthenticatedRearmedCompletionOutcomeV1>),
    PageRelease(Box<crate::M1AuthenticatedRearmedRoundPageReleaseFailureV1>),
    Released(Box<crate::M1AuthenticatedLongLivedQueueReleasedRoundV1>),
}

/// Opaque settlement failure retaining the exact observation and bridge.
#[must_use = "new-window settlement failure must be retried or closed"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativeNewWindowSettlementFailureV1 {
    error: M1AuthenticatedSpeculativeNewWindowSettlementErrorV1,
    state: M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1,
    members: Box<
        [Option<M1AuthenticatedSpeculativeNewWindowObservedMemberV1>;
            ferric_spec::M1_MAX_ACTIVE_SEQUENCES as usize],
    >,
    member_count: usize,
    selection: Qwen3PlanSelection,
    epoch: CompletionEpoch,
    dispositions: Vec<M1AuthenticatedSpeculativeNewWindowMemberDispositionV1>,
}

impl M1AuthenticatedSpeculativeNewWindowSettlementFailureV1 {
    #[must_use]
    pub const fn error(&self) -> M1AuthenticatedSpeculativeNewWindowSettlementErrorV1 {
        self.error
    }

    #[must_use]
    pub const fn retained_member_count(&self) -> usize {
        self.member_count
    }

    /// Repeats only a phase that still owns unchanged retry authority.
    ///
    /// # Errors
    ///
    /// Returns renewed opaque settlement custody if the retained phase rejects
    /// again or has no retry transition.
    pub fn retry<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<M1AuthenticatedSpeculativeNewWindowReleasedV1, Self> {
        retry_authenticated_speculative_new_window_settlement(engine, self)
    }

    /// Destroys or terminally retains every phase owner without exposing it.
    #[must_use = "clean release or terminal quarantine remains retained"]
    pub fn cancel_and_close<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> crate::M1AuthenticatedSpeculativeFailureDispositionV1 {
        close_authenticated_speculative_new_window_settlement(engine, self)
    }
}

/// Immutable post-release member status used to construct a fresh coordinator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedSpeculativeNewWindowReleasedMemberStatusV1 {
    Continuing,
    Retired,
}

/// Exact post-prefill seed coordinates, with all live custody still opaque.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1AuthenticatedSpeculativeNewWindowReleasedMemberV1 {
    request: ferric_spec::RequestId,
    emitted_token: ferric_spec::TokenId,
    target_committed_tokens: u32,
    draft_committed_tokens: u32,
    status: M1AuthenticatedSpeculativeNewWindowReleasedMemberStatusV1,
}

impl M1AuthenticatedSpeculativeNewWindowReleasedMemberV1 {
    #[must_use]
    pub const fn request(self) -> ferric_spec::RequestId {
        self.request
    }

    #[must_use]
    pub const fn emitted_token(self) -> ferric_spec::TokenId {
        self.emitted_token
    }

    #[must_use]
    pub const fn target_committed_tokens(self) -> u32 {
        self.target_committed_tokens
    }

    #[must_use]
    pub const fn draft_committed_tokens(self) -> u32 {
        self.draft_committed_tokens
    }

    #[must_use]
    pub const fn status(self) -> M1AuthenticatedSpeculativeNewWindowReleasedMemberStatusV1 {
        self.status
    }
}

/// Released paired-prefill round and sole public new-window successor owner.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedSpeculativeNewWindowReleasedV1;
/// fn expose_generic_queue(value: M1AuthenticatedSpeculativeNewWindowReleasedV1) {
///     let _ = value.into_parts();
/// }
/// ```
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedSpeculativeNewWindowReleasedV1;
/// fn expose_generic_completion(value: &M1AuthenticatedSpeculativeNewWindowReleasedV1) {
///     let _ = value.current_released();
/// }
/// ```
#[must_use = "released new-window custody must enter its exact successor or close"]
#[derive(Debug)]
pub struct M1AuthenticatedSpeculativeNewWindowReleasedV1 {
    released: crate::M1AuthenticatedLongLivedQueueReleasedRoundV1,
    members: Box<
        [Option<M1AuthenticatedSpeculativeNewWindowReleasedMemberV1>;
            ferric_spec::M1_MAX_ACTIVE_SEQUENCES as usize],
    >,
    member_count: usize,
    selection: Qwen3PlanSelection,
    epoch: CompletionEpoch,
}

impl M1AuthenticatedSpeculativeNewWindowReleasedV1 {
    pub(crate) const fn retained_logical_runner(&self) -> &crate::LogicalRunnerDeclaration {
        self.released.retained_logical_runner()
    }

    #[must_use]
    pub const fn member_count(&self) -> usize {
        self.member_count
    }

    #[must_use]
    pub const fn selection(&self) -> Qwen3PlanSelection {
        self.selection
    }

    #[must_use]
    pub const fn epoch(&self) -> CompletionEpoch {
        self.epoch
    }

    #[must_use]
    pub fn member(
        &self,
        lane: usize,
    ) -> Option<M1AuthenticatedSpeculativeNewWindowReleasedMemberV1> {
        self.members.get(lane).copied().flatten()
    }

    /// Consumes this exact released bridge into its fresh speculative successor.
    ///
    /// # Errors
    ///
    /// Returns opaque join retry custody when the batch does not match the
    /// bridge, or an authenticated rollover failure after joining.
    #[allow(clippy::too_many_arguments)]
    pub fn schedule_successor<const C: usize>(
        self,
        engine: &mut Engine<C>,
        batch: &M1ServingBatchPlanV1,
        coordinator: M1SpeculativeGenerationLoopV1,
        inputs: M1AuthenticatedSpeculativeNewWindowSuccessorInputsV1,
        recipe_plans: M1FullStepWorkspacePlans,
        preparation_plans: M1FullStepWorkspacePlans,
    ) -> Result<
        M1AuthenticatedScheduledSpeculativeRolloverV1,
        M1AuthenticatedSpeculativeNewWindowSuccessorJoinFailureV1,
    > {
        schedule_m1_authenticated_speculative_new_window_successor_v1(
            engine,
            self.released,
            batch,
            coordinator,
            inputs,
            recipe_plans,
            preparation_plans,
        )
    }

    pub(crate) fn cancel_and_close<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> crate::M1AuthenticatedSpeculativeFailureDispositionV1 {
        use crate::authenticated_speculative_executor::{
            quarantined_disposition, released_disposition,
        };

        engine.quarantine_m1_queue_rearm_failure();
        let retained = (self.members, self.member_count, self.selection, self.epoch);
        match self.released.destroy_queue_and_retain_round(engine) {
            Ok(released) => released_disposition((released, retained)),
            Err(quarantined) => quarantined_disposition((quarantined, retained)),
        }
    }
}

impl M1AuthenticatedSpeculativeNewWindowPublishedV1 {
    #[must_use]
    pub const fn queue_wait_timeout(&self) -> crate::M1QueueWaitTimeoutV1 {
        self.queue_wait_timeout
    }

    pub(crate) fn cancel_and_close<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> crate::M1AuthenticatedSpeculativeFailureDispositionV1 {
        engine.quarantine_m1_queue_rearm_failure();
        crate::authenticated_speculative_executor::disposition_with_logical(
            self.published.close_in_flight(engine),
            self.queue_wait_timeout,
        )
    }

    /// Waits with the frozen deadline, recycles, and copies exactly once.
    ///
    /// # Errors
    ///
    /// Returns opaque observation custody on wait, recycle, copy, selection,
    /// member-count, or lane-association rejection.
    pub fn observe<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1AuthenticatedSpeculativeNewWindowObservedV1,
        M1AuthenticatedSpeculativeNewWindowObservationFailureV1,
    > {
        self.observe_with_deadline(engine, |_, timeout| Some(timeout))
    }

    pub(crate) fn observe_with_deadline<const C: usize>(
        self,
        engine: &mut Engine<C>,
        mut deadline_expired: impl FnMut(
            crate::authenticated_resident_session::M1AuthenticatedResidentDeadlineBoundaryV1,
            crate::M1QueueWaitTimeoutV1,
        ) -> Option<crate::M1QueueWaitTimeoutV1>,
    ) -> Result<
        M1AuthenticatedSpeculativeNewWindowObservedV1,
        M1AuthenticatedSpeculativeNewWindowObservationFailureV1,
    > {
        use crate::authenticated_resident_session::M1AuthenticatedResidentDeadlineBoundaryV1 as Boundary;

        let queue_wait_timeout = self.queue_wait_timeout;
        let Some(wait_timeout) =
            deadline_expired(Boundary::BeforeCompletionWait, queue_wait_timeout)
        else {
            let disposition = self.cancel_and_close(engine);
            return Err(M1AuthenticatedSpeculativeNewWindowObservationFailureV1 {
                error: M1AuthenticatedSpeculativeNewWindowObservationErrorV1::Deadline,
                state: M1AuthenticatedSpeculativeNewWindowObservationFailureStateV1::Deadline(
                    disposition.retain(Boundary::BeforeCompletionWait),
                ),
            });
        };
        let completed = match self.published.wait_for(wait_timeout.milliseconds(), engine) {
            Ok(completed) => completed,
            Err(source) => {
                return Err(M1AuthenticatedSpeculativeNewWindowObservationFailureV1 {
                    error: M1AuthenticatedSpeculativeNewWindowObservationErrorV1::Wait,
                    state: M1AuthenticatedSpeculativeNewWindowObservationFailureStateV1::Progress(
                        source,
                    ),
                });
            }
        };
        if deadline_expired(Boundary::AfterCompletionWait, queue_wait_timeout).is_none() {
            let disposition = crate::authenticated_speculative_executor::disposition_with_logical(
                completed.close_completed(engine),
                Boundary::AfterCompletionWait,
            );
            return Err(M1AuthenticatedSpeculativeNewWindowObservationFailureV1 {
                error: M1AuthenticatedSpeculativeNewWindowObservationErrorV1::Deadline,
                state: M1AuthenticatedSpeculativeNewWindowObservationFailureStateV1::Deadline(
                    disposition,
                ),
            });
        }
        let recycled = match completed.recycle(engine) {
            Ok(recycled) => recycled,
            Err(source) => {
                return Err(M1AuthenticatedSpeculativeNewWindowObservationFailureV1 {
                    error: M1AuthenticatedSpeculativeNewWindowObservationErrorV1::Recycle,
                    state: M1AuthenticatedSpeculativeNewWindowObservationFailureStateV1::Progress(
                        source,
                    ),
                });
            }
        };
        if deadline_expired(Boundary::BeforeReadback, queue_wait_timeout).is_none() {
            let disposition = crate::authenticated_speculative_executor::disposition_with_logical(
                recycled.close_recycled(engine),
                Boundary::BeforeReadback,
            );
            return Err(M1AuthenticatedSpeculativeNewWindowObservationFailureV1 {
                error: M1AuthenticatedSpeculativeNewWindowObservationErrorV1::Deadline,
                state: M1AuthenticatedSpeculativeNewWindowObservationFailureStateV1::Deadline(
                    disposition,
                ),
            });
        }
        match recycled.observe_completion() {
            Ok(observed)
                if deadline_expired(Boundary::AfterReadback, queue_wait_timeout).is_none() =>
            {
                Err(M1AuthenticatedSpeculativeNewWindowObservationFailureV1 {
                    error: M1AuthenticatedSpeculativeNewWindowObservationErrorV1::Deadline,
                    state: M1AuthenticatedSpeculativeNewWindowObservationFailureStateV1::Observed(
                        Box::new(observed),
                    ),
                })
            }
            Ok(observed) => authenticated_new_window_observed(observed),
            Err(source) => Err(M1AuthenticatedSpeculativeNewWindowObservationFailureV1 {
                error: M1AuthenticatedSpeculativeNewWindowObservationErrorV1::Readback,
                state: M1AuthenticatedSpeculativeNewWindowObservationFailureStateV1::Readback(
                    source,
                ),
            }),
        }
    }
}

fn authenticated_new_window_observed(
    observed: crate::M1AuthenticatedRearmedObservedCompletionOutputV1,
) -> Result<
    M1AuthenticatedSpeculativeNewWindowObservedV1,
    M1AuthenticatedSpeculativeNewWindowObservationFailureV1,
> {
    let member_count = observed.selected_requests().len();
    let selection = observed.image().selection();
    let epoch = observed.image().epoch();
    let mut members = [None; ferric_spec::M1_MAX_ACTIVE_SEQUENCES as usize];
    let error = if selection.role != Qwen3ModelRole::Target8B
        || selection.mode != Qwen3ExecutionMode::Prefill
    {
        Some(M1AuthenticatedSpeculativeNewWindowObservationErrorV1::Selection)
    } else if member_count == 0
        || member_count > members.len()
        || observed.image().records().len() != member_count
    {
        Some(M1AuthenticatedSpeculativeNewWindowObservationErrorV1::MemberCount)
    } else {
        let mut error = None;
        for (lane, (request, record)) in observed
            .selected_requests()
            .zip(observed.image().records())
            .enumerate()
        {
            let raw = record.record();
            let emitted = record.emitted_tokens();
            if raw.request != request
                || raw.epoch != epoch
                || record.accepted_draft_tokens() != 0
                || emitted.len() != 1
            {
                error =
                    Some(M1AuthenticatedSpeculativeNewWindowObservationErrorV1::Member { lane });
                break;
            }
            members[lane] = Some(M1AuthenticatedSpeculativeNewWindowObservedMemberV1 {
                request,
                emitted_token: emitted[0],
            });
        }
        error
    };
    if let Some(error) = error {
        return Err(M1AuthenticatedSpeculativeNewWindowObservationFailureV1 {
            error,
            state: M1AuthenticatedSpeculativeNewWindowObservationFailureStateV1::Observed(
                Box::new(observed),
            ),
        });
    }
    Ok(M1AuthenticatedSpeculativeNewWindowObservedV1 {
        observed,
        members: Box::new(members),
        member_count,
        selection,
        epoch,
    })
}

fn authenticated_new_window_expectations(
    members: &[Option<M1AuthenticatedSpeculativeNewWindowObservedMemberV1>;
         ferric_spec::M1_MAX_ACTIVE_SEQUENCES as usize],
    member_count: usize,
) -> [crate::CompletionWireSemanticExpectation<'static>;
       ferric_spec::M1_MAX_ACTIVE_SEQUENCES as usize] {
    let mut expectations = [crate::CompletionWireSemanticExpectation::DirectFinalRow { choice: 0 };
        ferric_spec::M1_MAX_ACTIVE_SEQUENCES as usize];
    for (lane, expectation) in expectations.iter_mut().take(member_count).enumerate() {
        *expectation = crate::CompletionWireSemanticExpectation::DirectFinalRow {
            choice: members[lane]
                .expect("validated new-window observation is complete")
                .emitted_token,
        };
    }
    expectations
}

fn new_window_settlement_failure(
    error: M1AuthenticatedSpeculativeNewWindowSettlementErrorV1,
    state: M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1,
    members: Box<
        [Option<M1AuthenticatedSpeculativeNewWindowObservedMemberV1>;
            ferric_spec::M1_MAX_ACTIVE_SEQUENCES as usize],
    >,
    member_count: usize,
    selection: Qwen3PlanSelection,
    epoch: CompletionEpoch,
    dispositions: Vec<M1AuthenticatedSpeculativeNewWindowMemberDispositionV1>,
) -> M1AuthenticatedSpeculativeNewWindowSettlementFailureV1 {
    M1AuthenticatedSpeculativeNewWindowSettlementFailureV1 {
        error,
        state,
        members,
        member_count,
        selection,
        epoch,
        dispositions,
    }
}

fn settle_authenticated_speculative_new_window<const C: usize>(
    engine: &mut Engine<C>,
    observed: M1AuthenticatedSpeculativeNewWindowObservedV1,
    dispositions: Vec<M1AuthenticatedSpeculativeNewWindowMemberDispositionV1>,
) -> Result<
    M1AuthenticatedSpeculativeNewWindowReleasedV1,
    M1AuthenticatedSpeculativeNewWindowSettlementFailureV1,
> {
    settle_authenticated_speculative_new_window_with_deadline(
        engine,
        observed,
        dispositions,
        |_| false,
    )
}

fn settle_authenticated_speculative_new_window_with_deadline<const C: usize>(
    engine: &mut Engine<C>,
    observed: M1AuthenticatedSpeculativeNewWindowObservedV1,
    dispositions: Vec<M1AuthenticatedSpeculativeNewWindowMemberDispositionV1>,
    mut deadline_expired: impl FnMut(
        crate::authenticated_resident_session::M1AuthenticatedResidentDeadlineBoundaryV1,
    ) -> bool,
) -> Result<
    M1AuthenticatedSpeculativeNewWindowReleasedV1,
    M1AuthenticatedSpeculativeNewWindowSettlementFailureV1,
> {
    use crate::authenticated_resident_session::M1AuthenticatedResidentDeadlineBoundaryV1 as Boundary;

    let M1AuthenticatedSpeculativeNewWindowObservedV1 {
        observed,
        members,
        member_count,
        selection,
        epoch,
    } = observed;
    if dispositions.len() != member_count {
        let actual = dispositions.len();
        return Err(new_window_settlement_failure(
            M1AuthenticatedSpeculativeNewWindowSettlementErrorV1::DispositionCount {
                expected: member_count,
                actual,
            },
            M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::Observed(Box::new(
                observed,
            )),
            members,
            member_count,
            selection,
            epoch,
            dispositions,
        ));
    }
    let physical_dispositions = dispositions
        .iter()
        .map(|disposition| match disposition {
            M1AuthenticatedSpeculativeNewWindowMemberDispositionV1::Continue => {
                crate::M1DeviceKvCompletionDispositionV1::Continue
            }
            M1AuthenticatedSpeculativeNewWindowMemberDispositionV1::Retire => {
                crate::M1DeviceKvCompletionDispositionV1::Retire
            }
        })
        .collect();
    let expectations = authenticated_new_window_expectations(&members, member_count);
    if deadline_expired(Boundary::BeforeReadback) {
        return Err(new_window_settlement_failure(
            M1AuthenticatedSpeculativeNewWindowSettlementErrorV1::Deadline,
            M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::DeadlineObserved(
                Box::new((observed, Boundary::BeforeReadback)),
            ),
            members,
            member_count,
            selection,
            epoch,
            dispositions,
        ));
    }
    let readback = match observed.check_completion(&expectations[..member_count]) {
        Ok(readback) => readback,
        Err(source) => {
            return Err(new_window_settlement_failure(
                M1AuthenticatedSpeculativeNewWindowSettlementErrorV1::SemanticJoin,
                M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::Join(source),
                members,
                member_count,
                selection,
                epoch,
                dispositions,
            ));
        }
    };
    if deadline_expired(Boundary::AfterReadback) {
        return Err(new_window_settlement_failure(
            M1AuthenticatedSpeculativeNewWindowSettlementErrorV1::Deadline,
            M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::DeadlineReadback(
                Box::new((readback, physical_dispositions, Boundary::AfterReadback)),
            ),
            members,
            member_count,
            selection,
            epoch,
            dispositions,
        ));
    }
    if deadline_expired(Boundary::BeforeSettlement) {
        return Err(new_window_settlement_failure(
            M1AuthenticatedSpeculativeNewWindowSettlementErrorV1::Deadline,
            M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::DeadlineReadback(
                Box::new((readback, physical_dispositions, Boundary::BeforeSettlement)),
            ),
            members,
            member_count,
            selection,
            epoch,
            dispositions,
        ));
    }
    let outcome = match readback.complete(engine, physical_dispositions) {
        Ok(outcome) => outcome,
        Err(source) => {
            return Err(new_window_settlement_failure(
                M1AuthenticatedSpeculativeNewWindowSettlementErrorV1::CompletionPreflight,
                M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::CompletionPreflight(
                    Box::new(source),
                ),
                members,
                member_count,
                selection,
                epoch,
                dispositions,
            ));
        }
    };
    if deadline_expired(Boundary::AfterSettlement) {
        return Err(new_window_settlement_failure(
            M1AuthenticatedSpeculativeNewWindowSettlementErrorV1::Deadline,
            M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::DeadlineCompletion(
                Box::new((outcome, Boundary::AfterSettlement)),
            ),
            members,
            member_count,
            selection,
            epoch,
            dispositions,
        ));
    }
    finish_authenticated_speculative_new_window_completion_with_deadline(
        outcome,
        members,
        member_count,
        selection,
        epoch,
        dispositions,
        deadline_expired,
    )
}

fn finish_authenticated_speculative_new_window_completion(
    outcome: crate::M1AuthenticatedRearmedCompletionOutcomeV1,
    members: Box<
        [Option<M1AuthenticatedSpeculativeNewWindowObservedMemberV1>;
            ferric_spec::M1_MAX_ACTIVE_SEQUENCES as usize],
    >,
    member_count: usize,
    selection: Qwen3PlanSelection,
    epoch: CompletionEpoch,
    dispositions: Vec<M1AuthenticatedSpeculativeNewWindowMemberDispositionV1>,
) -> Result<
    M1AuthenticatedSpeculativeNewWindowReleasedV1,
    M1AuthenticatedSpeculativeNewWindowSettlementFailureV1,
> {
    finish_authenticated_speculative_new_window_completion_with_deadline(
        outcome,
        members,
        member_count,
        selection,
        epoch,
        dispositions,
        |_| false,
    )
}

fn finish_authenticated_speculative_new_window_completion_with_deadline(
    outcome: crate::M1AuthenticatedRearmedCompletionOutcomeV1,
    members: Box<
        [Option<M1AuthenticatedSpeculativeNewWindowObservedMemberV1>;
            ferric_spec::M1_MAX_ACTIVE_SEQUENCES as usize],
    >,
    member_count: usize,
    selection: Qwen3PlanSelection,
    epoch: CompletionEpoch,
    dispositions: Vec<M1AuthenticatedSpeculativeNewWindowMemberDispositionV1>,
    mut deadline_expired: impl FnMut(
        crate::authenticated_resident_session::M1AuthenticatedResidentDeadlineBoundaryV1,
    ) -> bool,
) -> Result<
    M1AuthenticatedSpeculativeNewWindowReleasedV1,
    M1AuthenticatedSpeculativeNewWindowSettlementFailureV1,
> {
    use crate::authenticated_resident_session::M1AuthenticatedResidentDeadlineBoundaryV1 as Boundary;

    if deadline_expired(Boundary::BeforeSettlement) {
        return Err(new_window_settlement_failure(
            M1AuthenticatedSpeculativeNewWindowSettlementErrorV1::Deadline,
            M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::DeadlineCompletion(
                Box::new((outcome, Boundary::BeforeSettlement)),
            ),
            members,
            member_count,
            selection,
            epoch,
            dispositions,
        ));
    }
    let released = outcome.release_completed();
    if deadline_expired(Boundary::AfterSettlement) {
        return Err(new_window_settlement_failure(
            M1AuthenticatedSpeculativeNewWindowSettlementErrorV1::Deadline,
            M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::DeadlineRelease(Box::new(
                (released, Boundary::AfterSettlement),
            )),
            members,
            member_count,
            selection,
            epoch,
            dispositions,
        ));
    }
    match released {
        crate::M1AuthenticatedRearmedRoundReleaseOutcomeV1::Released(released) => {
            finish_authenticated_speculative_new_window_release(
                released,
                members,
                member_count,
                selection,
                epoch,
                dispositions,
            )
        }
        crate::M1AuthenticatedRearmedRoundReleaseOutcomeV1::Rejected(source) => {
            Err(new_window_settlement_failure(
                M1AuthenticatedSpeculativeNewWindowSettlementErrorV1::PageRelease,
                M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::PageRelease(source),
                members,
                member_count,
                selection,
                epoch,
                dispositions,
            ))
        }
        crate::M1AuthenticatedRearmedRoundReleaseOutcomeV1::NotCompleted(outcome) => {
            Err(new_window_settlement_failure(
                M1AuthenticatedSpeculativeNewWindowSettlementErrorV1::CompletionRejected,
                M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::Completion(Box::new(
                    outcome,
                )),
                members,
                member_count,
                selection,
                epoch,
                dispositions,
            ))
        }
    }
}

fn finish_authenticated_speculative_new_window_release(
    released: crate::M1AuthenticatedLongLivedQueueReleasedRoundV1,
    observed_members: Box<
        [Option<M1AuthenticatedSpeculativeNewWindowObservedMemberV1>;
            ferric_spec::M1_MAX_ACTIVE_SEQUENCES as usize],
    >,
    member_count: usize,
    selection: Qwen3PlanSelection,
    epoch: CompletionEpoch,
    dispositions: Vec<M1AuthenticatedSpeculativeNewWindowMemberDispositionV1>,
) -> Result<
    M1AuthenticatedSpeculativeNewWindowReleasedV1,
    M1AuthenticatedSpeculativeNewWindowSettlementFailureV1,
> {
    let mut members = [None; ferric_spec::M1_MAX_ACTIVE_SEQUENCES as usize];
    let current = released.current_released().members();
    let mut valid = current.len() == member_count;
    for lane in 0..member_count {
        let Some(observed) = observed_members[lane] else {
            valid = false;
            break;
        };
        let Some(current) = current.get(lane) else {
            valid = false;
            break;
        };
        if current.request() != observed.request {
            valid = false;
            break;
        }
        let (target, draft, status) = match (current, dispositions[lane]) {
            (
                crate::M1ReleasedDeviceKvMemberV1::Active(cache),
                M1AuthenticatedSpeculativeNewWindowMemberDispositionV1::Continue,
            ) => {
                let projection = cache.projection();
                (
                    projection.target.committed_tokens,
                    projection.draft.committed_tokens,
                    M1AuthenticatedSpeculativeNewWindowReleasedMemberStatusV1::Continuing,
                )
            }
            (
                crate::M1ReleasedDeviceKvMemberV1::Terminal(terminal),
                M1AuthenticatedSpeculativeNewWindowMemberDispositionV1::Retire,
            ) => (
                terminal.target().committed_tokens,
                terminal.draft().committed_tokens,
                M1AuthenticatedSpeculativeNewWindowReleasedMemberStatusV1::Retired,
            ),
            _ => {
                valid = false;
                break;
            }
        };
        members[lane] = Some(M1AuthenticatedSpeculativeNewWindowReleasedMemberV1 {
            request: observed.request,
            emitted_token: observed.emitted_token,
            target_committed_tokens: target,
            draft_committed_tokens: draft,
            status,
        });
    }
    if !valid {
        return Err(new_window_settlement_failure(
            M1AuthenticatedSpeculativeNewWindowSettlementErrorV1::ReleasedRoster,
            M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::Released(Box::new(
                released,
            )),
            observed_members,
            member_count,
            selection,
            epoch,
            dispositions,
        ));
    }
    Ok(M1AuthenticatedSpeculativeNewWindowReleasedV1 {
        released,
        members: Box::new(members),
        member_count,
        selection,
        epoch,
    })
}

fn retry_authenticated_speculative_new_window_settlement<const C: usize>(
    engine: &mut Engine<C>,
    failure: M1AuthenticatedSpeculativeNewWindowSettlementFailureV1,
) -> Result<
    M1AuthenticatedSpeculativeNewWindowReleasedV1,
    M1AuthenticatedSpeculativeNewWindowSettlementFailureV1,
> {
    let M1AuthenticatedSpeculativeNewWindowSettlementFailureV1 {
        error,
        state,
        members,
        member_count,
        selection,
        epoch,
        dispositions,
    } = failure;
    match state {
        state @ (M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::DeadlineObserved(_)
        | M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::DeadlineReadback(_)
        | M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::DeadlineCompletion(_)
        | M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::DeadlineRelease(_)) => {
            Err(new_window_settlement_failure(
                error,
                state,
                members,
                member_count,
                selection,
                epoch,
                dispositions,
            ))
        }
        M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::Observed(observed) => {
            settle_authenticated_speculative_new_window(
                engine,
                M1AuthenticatedSpeculativeNewWindowObservedV1 {
                    observed: *observed,
                    members,
                    member_count,
                    selection,
                    epoch,
                },
                dispositions,
            )
        }
        M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::Join(source) => {
            match source.recover_observed_after_semantic_rejection() {
                Ok(observed) => settle_authenticated_speculative_new_window(
                    engine,
                    M1AuthenticatedSpeculativeNewWindowObservedV1 {
                        observed,
                        members,
                        member_count,
                        selection,
                        epoch,
                    },
                    dispositions,
                ),
                Err(source) => Err(new_window_settlement_failure(
                    error,
                    M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::Join(source),
                    members,
                    member_count,
                    selection,
                    epoch,
                    dispositions,
                )),
            }
        }
        M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::CompletionPreflight(
            source,
        ) => match (*source).retry(engine) {
            Ok(outcome) => finish_authenticated_speculative_new_window_completion(
                outcome,
                members,
                member_count,
                selection,
                epoch,
                dispositions,
            ),
            Err(source) => Err(new_window_settlement_failure(
                M1AuthenticatedSpeculativeNewWindowSettlementErrorV1::CompletionPreflight,
                M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::CompletionPreflight(
                    Box::new(source),
                ),
                members,
                member_count,
                selection,
                epoch,
                dispositions,
            )),
        },
        M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::Completion(outcome) => {
            match (*outcome).retry_rejected(engine) {
                Ok(outcome) => finish_authenticated_speculative_new_window_completion(
                    outcome,
                    members,
                    member_count,
                    selection,
                    epoch,
                    dispositions,
                ),
                Err(outcome) => Err(new_window_settlement_failure(
                    error,
                    M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::Completion(
                        outcome,
                    ),
                    members,
                    member_count,
                    selection,
                    epoch,
                    dispositions,
                )),
            }
        }
        M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::PageRelease(source) => {
            match source.retry() {
                crate::M1AuthenticatedRearmedRoundReleaseOutcomeV1::Released(released) => {
                    finish_authenticated_speculative_new_window_release(
                        released,
                        members,
                        member_count,
                        selection,
                        epoch,
                        dispositions,
                    )
                }
                crate::M1AuthenticatedRearmedRoundReleaseOutcomeV1::Rejected(source) => {
                    Err(new_window_settlement_failure(
                        M1AuthenticatedSpeculativeNewWindowSettlementErrorV1::PageRelease,
                        M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::PageRelease(
                            source,
                        ),
                        members,
                        member_count,
                        selection,
                        epoch,
                        dispositions,
                    ))
                }
                crate::M1AuthenticatedRearmedRoundReleaseOutcomeV1::NotCompleted(outcome) => {
                    Err(new_window_settlement_failure(
                        M1AuthenticatedSpeculativeNewWindowSettlementErrorV1::CompletionRejected,
                        M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::Completion(
                            Box::new(outcome),
                        ),
                        members,
                        member_count,
                        selection,
                        epoch,
                        dispositions,
                    ))
                }
            }
        }
        M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::Released(released) => {
            Err(new_window_settlement_failure(
                error,
                M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::Released(released),
                members,
                member_count,
                selection,
                epoch,
                dispositions,
            ))
        }
    }
}

fn close_authenticated_speculative_new_window_settlement<const C: usize>(
    engine: &mut Engine<C>,
    failure: M1AuthenticatedSpeculativeNewWindowSettlementFailureV1,
) -> crate::M1AuthenticatedSpeculativeFailureDispositionV1 {
    use crate::authenticated_speculative_executor::{
        close_deadline_phase, quarantined_disposition, released_disposition,
    };
    engine.quarantine_m1_queue_rearm_failure();
    let M1AuthenticatedSpeculativeNewWindowSettlementFailureV1 {
        error,
        state,
        members,
        member_count,
        selection,
        epoch,
        dispositions,
    } = failure;
    let observed_expectations = matches!(
        &state,
        M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::Observed(_)
            | M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::DeadlineObserved(_)
    )
    .then(|| authenticated_new_window_expectations(&members, member_count));
    let metadata = (error, members, member_count, selection, epoch, dispositions);
    match state {
        M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::DeadlineObserved(retained) => {
            close_deadline_phase(*retained, |(observed, boundary)| {
                let expectations = observed_expectations
                    .expect("deadline-observed settlement state prepared exact expectations");
                match observed.check_completion(&expectations[..member_count]) {
                    Ok(readback) => match readback.destroy_queue_and_retain_custody(engine) {
                        Ok(released) => released_disposition((metadata, boundary, released)),
                        Err(quarantined) => {
                            quarantined_disposition((metadata, boundary, quarantined))
                        }
                    },
                    Err(source) => match source.destroy_queue_and_retain_custody(engine) {
                        Ok(released) => released_disposition((metadata, boundary, released)),
                        Err(quarantined) => {
                            quarantined_disposition((metadata, boundary, quarantined))
                        }
                    },
                }
            })
        }
        M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::DeadlineReadback(retained) => {
            close_deadline_phase(*retained, |(readback, physical_dispositions, boundary)| {
                match readback.destroy_queue_and_retain_custody(engine) {
                    Ok(released) => {
                        released_disposition((metadata, physical_dispositions, boundary, released))
                    }
                    Err(quarantined) => quarantined_disposition((
                        metadata,
                        physical_dispositions,
                        boundary,
                        quarantined,
                    )),
                }
            })
        }
        M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::DeadlineCompletion(
            retained,
        ) => close_deadline_phase(*retained, |(outcome, boundary)| {
            outcome.destroy_queue_and_retain_any(engine, (metadata, boundary))
        }),
        M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::DeadlineRelease(retained) => {
            close_deadline_phase(*retained, |(released, boundary)| match released {
                crate::M1AuthenticatedRearmedRoundReleaseOutcomeV1::Released(released) => {
                    match released.destroy_queue_and_retain_round(engine) {
                        Ok(released) => released_disposition((metadata, boundary, released)),
                        Err(quarantined) => {
                            quarantined_disposition((metadata, boundary, quarantined))
                        }
                    }
                }
                crate::M1AuthenticatedRearmedRoundReleaseOutcomeV1::Rejected(source) => {
                    match source.destroy_queue_and_retain_round(engine) {
                        Ok(released) => released_disposition((metadata, boundary, released)),
                        Err(quarantined) => {
                            quarantined_disposition((metadata, boundary, quarantined))
                        }
                    }
                }
                crate::M1AuthenticatedRearmedRoundReleaseOutcomeV1::NotCompleted(outcome) => {
                    outcome.destroy_queue_and_retain_any(engine, (metadata, boundary))
                }
            })
        }
        M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::Observed(observed) => {
            let expectations = observed_expectations
                .expect("observed settlement state prepared exact expectations");
            match (*observed).check_completion(&expectations[..member_count]) {
                Ok(readback) => match readback.destroy_queue_and_retain_custody(engine) {
                    Ok(released) => released_disposition((metadata, released)),
                    Err(quarantined) => quarantined_disposition((metadata, quarantined)),
                },
                Err(source) => match source.destroy_queue_and_retain_custody(engine) {
                    Ok(released) => released_disposition((metadata, released)),
                    Err(quarantined) => quarantined_disposition((metadata, quarantined)),
                },
            }
        }
        M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::Join(source) => {
            match source.destroy_queue_and_retain_custody(engine) {
                Ok(released) => released_disposition((metadata, released)),
                Err(quarantined) => quarantined_disposition((metadata, quarantined)),
            }
        }
        M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::CompletionPreflight(
            source,
        ) => match (*source).destroy_queue_and_retain_custody(engine) {
            Ok(released) => released_disposition((metadata, released)),
            Err(quarantined) => quarantined_disposition((metadata, quarantined)),
        },
        M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::Completion(outcome) => {
            (*outcome).destroy_queue_and_retain_any(engine, metadata)
        }
        M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::PageRelease(source) => {
            match source.destroy_queue_and_retain_round(engine) {
                Ok(released) => released_disposition((metadata, released)),
                Err(quarantined) => quarantined_disposition((metadata, quarantined)),
            }
        }
        M1AuthenticatedSpeculativeNewWindowSettlementFailureStateV1::Released(released) => {
            match (*released).destroy_queue_and_retain_round(engine) {
                Ok(released) => released_disposition((metadata, released)),
                Err(quarantined) => quarantined_disposition((metadata, quarantined)),
            }
        }
    }
}

impl M1AuthenticatedPreparedSpeculativeNewWindowV1 {
    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        &self.scheduled
    }

    #[must_use]
    pub const fn queue_wait_timeout(&self) -> crate::M1QueueWaitTimeoutV1 {
        self.next_queue_wait_timeout
    }
}

/// Derives the exact paired-prefill recipe from retained authenticated
/// operations. This stage accepts no structural runner or catalog.
///
/// # Errors
///
/// Recipe rejection or mismatch closes or quarantines the already detached
/// queue, faults the Engine, and retains every other owner in the disposition.
pub fn prepare_m1_authenticated_speculative_new_window_v1<const C: usize>(
    engine: &mut Engine<C>,
    scheduled: M1AuthenticatedScheduledSpeculativeNewWindowV1,
) -> Result<
    M1AuthenticatedPreparedSpeculativeNewWindowV1,
    M1AuthenticatedSpeculativeNewWindowScheduleFailureV1,
> {
    let M1AuthenticatedScheduledSpeculativeNewWindowV1 {
        prior,
        next,
        queue,
        scheduled,
        residue,
        binding,
        draft_prefill,
        target_prefill,
        preparation_plans,
        recipe_plans,
        speculative_successor,
        member_intents,
        prior_windows,
        ring_bytes,
        next_queue_wait_timeout,
    } = scheduled;
    let recipe = match crate::runner::derive_physical_step_recipe(
        queue.operations(),
        M1StepDispatchIntent::PairedPrefill(next.target()),
        recipe_plans,
    ) {
        M1PhysicalRunnerRecipeOutcomeV1::Prepared(recipe) => recipe,
        M1PhysicalRunnerRecipeOutcomeV1::Rejected(source) => {
            return Err(close_new_window_detached(
                engine,
                M1AuthenticatedSpeculativeNewWindowScheduleErrorV1::Input,
                queue,
                (
                    (
                        source,
                        prior,
                        next,
                        scheduled,
                        residue,
                        binding,
                        draft_prefill,
                    ),
                    (
                        target_prefill,
                        preparation_plans,
                        speculative_successor,
                        member_intents,
                        prior_windows,
                        ring_bytes,
                        next_queue_wait_timeout,
                    ),
                ),
            ));
        }
    };
    if engine.is_faulted()
        || recipe.workspace_composition().workspace_plans() != &preparation_plans
        || recipe.requires_future_materialization()
        || recipe.rows().len() != crate::M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1
        || recipe.kernarg_recipe().images().len() != crate::M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1
    {
        return Err(close_new_window_detached(
            engine,
            M1AuthenticatedSpeculativeNewWindowScheduleErrorV1::Input,
            queue,
            (
                (
                    prior,
                    next,
                    scheduled,
                    residue,
                    binding,
                    draft_prefill,
                    target_prefill,
                ),
                (
                    preparation_plans,
                    recipe,
                    speculative_successor,
                    member_intents,
                    prior_windows,
                    ring_bytes,
                    next_queue_wait_timeout,
                ),
            ),
        ));
    }
    Ok(M1AuthenticatedPreparedSpeculativeNewWindowV1 {
        prior,
        next,
        queue,
        scheduled,
        residue,
        binding,
        draft_prefill,
        target_prefill,
        preparation_plans,
        recipe,
        speculative_successor,
        member_intents,
        prior_windows,
        ring_bytes,
        next_queue_wait_timeout,
    })
}

struct M1AuthenticatedNewWindowOpaqueCustodyV1<T>(T);

impl<T> fmt::Debug for M1AuthenticatedNewWindowOpaqueCustodyV1<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedNewWindowOpaqueCustodyV1")
            .finish_non_exhaustive()
    }
}

fn close_new_window_prepared_detached<const C: usize, T: 'static>(
    engine: &mut Engine<C>,
    queue: M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1,
    retained: T,
) -> M1AuthenticatedSpeculativeRolloverSubmissionFailureV1 {
    let failure = close_new_window_detached(
        engine,
        M1AuthenticatedSpeculativeNewWindowScheduleErrorV1::Input,
        queue,
        M1AuthenticatedNewWindowOpaqueCustodyV1(retained),
    );
    let M1AuthenticatedSpeculativeNewWindowScheduleFailureV1::Terminal { disposition, .. } =
        failure
    else {
        unreachable!("detached closure is terminal")
    };
    M1AuthenticatedSpeculativeRolloverSubmissionFailureV1 {
        stage: M1AuthenticatedSpeculativeRolloverSubmissionStageV1::Preflight,
        disposition,
    }
}

fn close_new_window_unbound<const C: usize, T: 'static>(
    engine: &mut Engine<C>,
    stage: M1AuthenticatedSpeculativeRolloverSubmissionStageV1,
    lower: AuthenticatedServiceQueueUnboundSessionV1,
    retained: T,
) -> M1AuthenticatedSpeculativeRolloverSubmissionFailureV1 {
    let pending = close_unbound(
        engine,
        stage,
        lower,
        M1AuthenticatedNewWindowOpaqueCustodyV1(retained),
    );
    close_pending_submission_failure(engine, pending)
}

/// Publishes the authenticated speculative-to-paired-prefill new window.
///
/// The exact speculative successor intent and queue timeout were frozen before
/// this publication-capable stage. Every failure closes the detached/unbound
/// queue or returns terminal quarantine and permanently faults the Engine.
///
/// # Errors
///
/// Returns a terminal authenticated rollover submission failure if any exact
/// bind, reservation, admission, packet, rollover, or publication step fails.
///
/// # Panics
///
/// Panics only if the scheduled member roster changes after its exact
/// preparation preflight while it is held by this function.
#[allow(clippy::too_many_lines)]
pub fn submit_m1_authenticated_speculative_new_window_v1<const C: usize>(
    engine: &mut Engine<C>,
    prepared: M1AuthenticatedPreparedSpeculativeNewWindowV1,
) -> Result<
    M1AuthenticatedSpeculativeNewWindowPublishedV1,
    M1AuthenticatedSpeculativeRolloverSubmissionFailureV1,
> {
    let M1AuthenticatedPreparedSpeculativeNewWindowV1 {
        prior,
        next,
        queue,
        scheduled,
        residue,
        binding,
        draft_prefill,
        target_prefill,
        preparation_plans,
        recipe,
        speculative_successor,
        member_intents,
        prior_windows,
        ring_bytes,
        next_queue_wait_timeout,
    } = prepared;
    let old = queue.custody();
    if engine.is_faulted()
        || !matches!(
            queue.shape(),
            M1PhysicalFixedBatchShapeV1::SpeculativeK4
                | M1PhysicalFixedBatchShapeV1::SpeculativeK8
                | M1PhysicalFixedBatchShapeV1::SpeculativeK16
        )
        || queue.shape() != prior.shape()
        || next.shape() != M1PhysicalFixedBatchShapeV1::PairedPrefill
        || old.selection() != prior.target()
        || old
            .partitioned_memory()
            .finite_speculative_rollover_output_state()
            != crate::M1FiniteSpeculativeRolloverOutputPortfolioStateV1::Activated
        || scheduled.member_count() == 0
        || scheduled.member_count() != member_intents.len()
        || draft_prefill.active_lengths().len() != scheduled.member_count()
        || target_prefill.active_lengths().len() != scheduled.member_count()
        || recipe.workspace_composition().workspace_plans() != &preparation_plans
        || recipe.requires_future_materialization()
        || recipe.rows().len() != crate::M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1
        || recipe.kernarg_recipe().images().len() != crate::M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1
    {
        return Err(close_new_window_prepared_detached(
            engine,
            queue,
            (
                (
                    prior,
                    next,
                    scheduled,
                    residue,
                    binding,
                    draft_prefill,
                    target_prefill,
                ),
                (
                    preparation_plans,
                    recipe,
                    speculative_successor,
                    member_intents,
                    prior_windows,
                    ring_bytes,
                    next_queue_wait_timeout,
                ),
            ),
        ));
    }

    let (old_shape, lower, witness, operations, custody) = queue.into_rearm_parts();
    let predecessor_observation = lower.observation();
    let predecessor_generation = lower.detached_dispatch_generation();
    let device = custody.device();
    let crate::physical_fixed_batch::M1PhysicalQueueBatchRearmPartsV1 {
        catalog_id,
        selection: old_selection,
        physical_recipe: old_physical_recipe,
        workspace_composition: old_workspace_composition,
        workspace_owners,
        mut partitioned_memory,
        completion_output: prior_output,
        source_rows: old_source_rows,
        bound_rows: old_bound_rows,
        retired_rollover_custody,
    } = custody.into_rearm_parts();
    let retired_physical_metadata = (
        old_shape,
        old_selection,
        old_physical_recipe,
        old_workspace_composition,
    );

    let members = scheduled.member_count();
    let mut selected = Vec::new();
    let mut draft_reservations = Vec::new();
    let mut target_reservations = Vec::new();
    let mut draft_page_leases: Vec<Vec<DeviceKvPageLease>> = Vec::new();
    let mut target_page_leases: Vec<Vec<DeviceKvPageLease>> = Vec::new();
    if selected.try_reserve_exact(members).is_err()
        || draft_reservations.try_reserve_exact(members).is_err()
        || target_reservations.try_reserve_exact(members).is_err()
        || draft_page_leases.try_reserve_exact(members).is_err()
        || target_page_leases.try_reserve_exact(members).is_err()
    {
        return Err(close_new_window_unbound(
            engine,
            M1AuthenticatedSpeculativeRolloverSubmissionStageV1::Preflight,
            lower,
            (
                retired_physical_metadata,
                witness,
                operations,
                catalog_id,
                workspace_owners,
                partitioned_memory,
                prior_output,
                old_source_rows,
                old_bound_rows,
                retired_rollover_custody,
                residue,
                binding,
                draft_prefill,
                target_prefill,
                preparation_plans,
                recipe,
                speculative_successor,
                member_intents,
                prior_windows,
                ring_bytes,
                next_queue_wait_timeout,
            ),
        ));
    }
    for lane in 0..members {
        let draft_pages = draft_prefill.active_lengths()[lane].div_ceil(M1_KV_PAGE_TOKENS);
        let target_pages = target_prefill.active_lengths()[lane].div_ceil(M1_KV_PAGE_TOKENS);
        let mut draft = Vec::new();
        let mut target = Vec::new();
        if draft.try_reserve_exact(draft_pages as usize).is_err()
            || target.try_reserve_exact(target_pages as usize).is_err()
        {
            return Err(close_new_window_unbound(
                engine,
                M1AuthenticatedSpeculativeRolloverSubmissionStageV1::Preflight,
                lower,
                (
                    retired_physical_metadata,
                    witness,
                    operations,
                    catalog_id,
                    workspace_owners,
                    partitioned_memory,
                    prior_output,
                    old_source_rows,
                    old_bound_rows,
                    retired_rollover_custody,
                    residue,
                    binding,
                    draft_prefill,
                    target_prefill,
                    preparation_plans,
                    recipe,
                    speculative_successor,
                    member_intents,
                    prior_windows,
                    ring_bytes,
                    next_queue_wait_timeout,
                    selected,
                    draft_reservations,
                    target_reservations,
                    draft_page_leases,
                    target_page_leases,
                    draft,
                    target,
                ),
            ));
        }
        draft_page_leases.push(draft);
        target_page_leases.push(target);
    }

    let mut page_requests =
        [ferric_spec::RequestId::new(u32::MAX, 0); ferric_spec::M1_MAX_ACTIVE_SEQUENCES as usize];
    for (lane, request) in page_requests.iter_mut().take(members).enumerate() {
        *request = scheduled
            .member(lane)
            .expect("the checked scheduled roster is complete");
    }
    let page_admission = match partitioned_memory.admit_authenticated_new_window_page_set(
        &lower,
        &page_requests[..members],
        draft_prefill.active_lengths(),
        target_prefill.active_lengths(),
    ) {
        Ok(admission) => admission,
        Err(source) => {
            return Err(close_new_window_unbound(
                engine,
                M1AuthenticatedSpeculativeRolloverSubmissionStageV1::Preflight,
                lower,
                (
                    source,
                    retired_physical_metadata,
                    witness,
                    operations,
                    catalog_id,
                    workspace_owners,
                    partitioned_memory,
                    prior_output,
                    old_source_rows,
                    old_bound_rows,
                    retired_rollover_custody,
                    residue,
                    binding,
                    draft_prefill,
                    target_prefill,
                    preparation_plans,
                    recipe,
                    speculative_successor,
                    member_intents,
                    prior_windows,
                    ring_bytes,
                    next_queue_wait_timeout,
                    selected,
                    draft_reservations,
                    target_reservations,
                    draft_page_leases,
                    target_page_leases,
                ),
            ));
        }
    };
    let page_leases =
        match partitioned_memory.commit_authenticated_new_window_page_set(&lower, page_admission) {
            Ok(leases) => leases,
            Err(failure) => {
                debug_assert!(failure.retains_exact_admission());
                return Err(close_new_window_unbound(
                    engine,
                    M1AuthenticatedSpeculativeRolloverSubmissionStageV1::Preflight,
                    lower,
                    (
                        failure,
                        retired_physical_metadata,
                        witness,
                        operations,
                        catalog_id,
                        workspace_owners,
                        partitioned_memory,
                        prior_output,
                        old_source_rows,
                        old_bound_rows,
                        retired_rollover_custody,
                        residue,
                        binding,
                        draft_prefill,
                        target_prefill,
                        preparation_plans,
                        recipe,
                        speculative_successor,
                        member_intents,
                        prior_windows,
                        ring_bytes,
                        next_queue_wait_timeout,
                        selected,
                        draft_reservations,
                        target_reservations,
                        draft_page_leases,
                        target_page_leases,
                    ),
                ));
            }
        };
    let mut page_leases = page_leases.into_iter();

    for lane in 0..members {
        let request = scheduled
            .member(lane)
            .expect("scheduled roster length was checked");
        let mut cache = match partitioned_memory.new_window_device_cache(
            request,
            next.target(),
            next.draft(),
        ) {
            Ok(cache) => cache,
            Err(source) => {
                return Err(close_new_window_unbound(
                    engine,
                    M1AuthenticatedSpeculativeRolloverSubmissionStageV1::Preflight,
                    lower,
                    (
                        source,
                        retired_physical_metadata,
                        witness,
                        operations,
                        catalog_id,
                        workspace_owners,
                        partitioned_memory,
                        prior_output,
                        old_source_rows,
                        old_bound_rows,
                        retired_rollover_custody,
                        residue,
                        binding,
                        draft_prefill,
                        target_prefill,
                        preparation_plans,
                        recipe,
                        speculative_successor,
                        member_intents,
                        prior_windows,
                        ring_bytes,
                        next_queue_wait_timeout,
                        selected,
                        draft_reservations,
                        target_reservations,
                        draft_page_leases,
                        target_page_leases,
                        page_leases,
                    ),
                ));
            }
        };
        let mut draft_leases = core::mem::take(&mut draft_page_leases[lane]);
        let mut target_leases = core::mem::take(&mut target_page_leases[lane]);
        for _ in 0..draft_prefill.active_lengths()[lane].div_ceil(M1_KV_PAGE_TOKENS) {
            draft_leases.push(
                page_leases
                    .next()
                    .expect("the exact admitted draft-page roster is complete"),
            );
        }
        for _ in 0..target_prefill.active_lengths()[lane].div_ceil(M1_KV_PAGE_TOKENS) {
            target_leases.push(
                page_leases
                    .next()
                    .expect("the exact admitted target-page roster is complete"),
            );
        }
        let draft = match cache.reserve_step_write(
            request,
            Qwen3ModelRole::Draft06B,
            0,
            draft_prefill.active_lengths()[lane],
            scheduled.epoch(),
            draft_leases,
        ) {
            Ok(reservation) => reservation,
            Err(source) => {
                return Err(close_new_window_unbound(
                    engine,
                    M1AuthenticatedSpeculativeRolloverSubmissionStageV1::Preflight,
                    lower,
                    (
                        source,
                        cache,
                        target_leases,
                        retired_physical_metadata,
                        witness,
                        operations,
                        workspace_owners,
                        partitioned_memory,
                        prior_output,
                        old_source_rows,
                        old_bound_rows,
                        retired_rollover_custody,
                        residue,
                        binding,
                        draft_prefill,
                        target_prefill,
                        preparation_plans,
                        recipe,
                        speculative_successor,
                        member_intents,
                        prior_windows,
                        ring_bytes,
                        next_queue_wait_timeout,
                        selected,
                        draft_reservations,
                        target_reservations,
                        draft_page_leases,
                        target_page_leases,
                        catalog_id,
                        page_leases,
                    ),
                ));
            }
        };
        let target = match cache.reserve_step_write(
            request,
            Qwen3ModelRole::Target8B,
            0,
            target_prefill.active_lengths()[lane],
            scheduled.epoch(),
            target_leases,
        ) {
            Ok(reservation) => reservation,
            Err(source) => {
                return Err(close_new_window_unbound(
                    engine,
                    M1AuthenticatedSpeculativeRolloverSubmissionStageV1::Preflight,
                    lower,
                    (
                        source,
                        cache,
                        draft,
                        retired_physical_metadata,
                        witness,
                        operations,
                        workspace_owners,
                        partitioned_memory,
                        prior_output,
                        old_source_rows,
                        old_bound_rows,
                        retired_rollover_custody,
                        residue,
                        binding,
                        draft_prefill,
                        target_prefill,
                        preparation_plans,
                        recipe,
                        speculative_successor,
                        member_intents,
                        prior_windows,
                        ring_bytes,
                        next_queue_wait_timeout,
                        selected,
                        draft_reservations,
                        target_reservations,
                        draft_page_leases,
                        target_page_leases,
                        catalog_id,
                        page_leases,
                    ),
                ));
            }
        };
        selected.push(cache);
        draft_reservations.push(draft);
        target_reservations.push(target);
    }
    debug_assert!(page_leases.next().is_none());

    let target = match crate::bind_m1_kv_workspace_table_v1(target_prefill, target_reservations) {
        Ok(table) => table,
        Err(source) => {
            return Err(close_new_window_unbound(
                engine,
                M1AuthenticatedSpeculativeRolloverSubmissionStageV1::Preflight,
                lower,
                (
                    source,
                    retired_physical_metadata,
                    witness,
                    operations,
                    catalog_id,
                    workspace_owners,
                    partitioned_memory,
                    prior_output,
                    old_source_rows,
                    old_bound_rows,
                    retired_rollover_custody,
                    residue,
                    binding,
                    draft_prefill,
                    preparation_plans,
                    recipe,
                    speculative_successor,
                    member_intents,
                    prior_windows,
                    ring_bytes,
                    next_queue_wait_timeout,
                    selected,
                    draft_reservations,
                    draft_page_leases,
                    target_page_leases,
                ),
            ));
        }
    };
    let draft = match crate::bind_m1_kv_workspace_table_v1(draft_prefill, draft_reservations) {
        Ok(table) => table,
        Err(source) => {
            return Err(close_new_window_unbound(
                engine,
                M1AuthenticatedSpeculativeRolloverSubmissionStageV1::Preflight,
                lower,
                (
                    source,
                    retired_physical_metadata,
                    witness,
                    operations,
                    catalog_id,
                    workspace_owners,
                    partitioned_memory,
                    prior_output,
                    old_source_rows,
                    old_bound_rows,
                    retired_rollover_custody,
                    residue,
                    binding,
                    target,
                    preparation_plans,
                    recipe,
                    speculative_successor,
                    member_intents,
                    prior_windows,
                    ring_bytes,
                    next_queue_wait_timeout,
                    selected,
                    draft_page_leases,
                    target_page_leases,
                ),
            ));
        }
    };
    let prepared = match crate::prepare_m1_scheduled_workspace_images_v1(
        scheduled,
        operations.runner(),
        preparation_plans,
        M1FullStepKvWorkspaceTablesV1::PairedPrefill { draft, target },
    ) {
        Ok(prepared) => prepared,
        Err(source) => {
            return Err(close_new_window_unbound(
                engine,
                M1AuthenticatedSpeculativeRolloverSubmissionStageV1::Preflight,
                lower,
                (
                    source,
                    retired_physical_metadata,
                    witness,
                    operations,
                    workspace_owners,
                    partitioned_memory,
                    prior_output,
                    old_source_rows,
                    old_bound_rows,
                    retired_rollover_custody,
                    residue,
                    binding,
                    recipe,
                    speculative_successor,
                    member_intents,
                    prior_windows,
                    ring_bytes,
                    next_queue_wait_timeout,
                    selected,
                    draft_page_leases,
                    target_page_leases,
                    catalog_id,
                ),
            ));
        }
    };
    let prepared = match bind_m1_authenticated_speculative_rollover_intent_v1(
        prepared,
        speculative_successor,
        member_intents,
    ) {
        Ok(prepared) => prepared,
        Err(source) => {
            return Err(close_new_window_unbound(
                engine,
                M1AuthenticatedSpeculativeRolloverSubmissionStageV1::Preflight,
                lower,
                (
                    source,
                    retired_physical_metadata,
                    witness,
                    operations,
                    workspace_owners,
                    partitioned_memory,
                    prior_output,
                    old_source_rows,
                    old_bound_rows,
                    retired_rollover_custody,
                    residue,
                    binding,
                    recipe,
                    prior_windows,
                    ring_bytes,
                    next_queue_wait_timeout,
                    selected,
                    draft_page_leases,
                    target_page_leases,
                    catalog_id,
                ),
            ));
        }
    };
    let (prepared, intent) = prepared.into_parts();
    let (plans, images, step) = prepared.into_rearm_parts();
    let (old_draft, old_target, draft_plan, target_plan, draft_bytes, target_bytes) =
        match (workspace_owners, plans, images) {
            (
                M1FullStepWorkspaceSubleaseOwners::SpeculativeRound {
                    draft_decode,
                    target_speculative,
                },
                M1FullStepWorkspacePlans::PairedPrefill { draft, target },
                M1FullStepWorkspaceImagesV1::PairedPrefill {
                    draft: draft_bytes,
                    target: target_bytes,
                },
            ) => (
                draft_decode,
                target_speculative,
                draft,
                target,
                draft_bytes,
                target_bytes,
            ),
            (workspace_owners, plans, images) => {
                return Err(close_new_window_unbound(
                    engine,
                    M1AuthenticatedSpeculativeRolloverSubmissionStageV1::Preflight,
                    lower,
                    (
                        retired_physical_metadata,
                        witness,
                        operations,
                        workspace_owners,
                        partitioned_memory,
                        prior_output,
                        old_source_rows,
                        old_bound_rows,
                        retired_rollover_custody,
                        residue,
                        binding,
                        recipe,
                        prior_windows,
                        ring_bytes,
                        next_queue_wait_timeout,
                        selected,
                        draft_page_leases,
                        target_page_leases,
                        plans,
                        images,
                        step,
                        intent,
                        catalog_id,
                    ),
                ));
            }
        };
    let draft_descriptor = match crate::authenticated_queue_rearm::descriptor(
        M1InitializedWorkspaceSlotV1::PairedPrefillDraft,
        &draft_bytes,
    ) {
        Ok(descriptor) => descriptor,
        Err(()) => {
            return Err(close_new_window_unbound(
                engine,
                M1AuthenticatedSpeculativeRolloverSubmissionStageV1::DraftWorkspace,
                lower,
                (
                    retired_physical_metadata,
                    witness,
                    operations,
                    old_draft,
                    old_target,
                    draft_plan,
                    target_plan,
                    draft_bytes,
                    target_bytes,
                    partitioned_memory,
                    prior_output,
                    old_source_rows,
                    old_bound_rows,
                    retired_rollover_custody,
                    residue,
                    binding,
                    recipe,
                    prior_windows,
                    ring_bytes,
                    next_queue_wait_timeout,
                    selected,
                    step,
                    intent,
                    catalog_id,
                ),
            ));
        }
    };
    let target_descriptor = match crate::authenticated_queue_rearm::descriptor(
        M1InitializedWorkspaceSlotV1::PairedPrefillTarget,
        &target_bytes,
    ) {
        Ok(descriptor) => descriptor,
        Err(()) => {
            return Err(close_new_window_unbound(
                engine,
                M1AuthenticatedSpeculativeRolloverSubmissionStageV1::TargetWorkspace,
                lower,
                (
                    retired_physical_metadata,
                    witness,
                    operations,
                    old_draft,
                    old_target,
                    draft_plan,
                    target_plan,
                    draft_bytes,
                    target_bytes,
                    draft_descriptor,
                    partitioned_memory,
                    prior_output,
                    old_source_rows,
                    old_bound_rows,
                    retired_rollover_custody,
                    residue,
                    binding,
                    recipe,
                    prior_windows,
                    ring_bytes,
                    next_queue_wait_timeout,
                    selected,
                    step,
                    intent,
                    catalog_id,
                ),
            ));
        }
    };
    let (lower, draft, draft_ranges) =
        match crate::authenticated_queue_rearm::replace_authenticated_rollover_workspace(
            lower,
            &old_draft,
            *draft_plan,
            draft_bytes,
            draft_descriptor,
        ) {
            Ok(value) => value,
            Err(failure) => {
                let pending = close_workspace_failure(
                    engine,
                    M1AuthenticatedSpeculativeRolloverSubmissionStageV1::DraftWorkspace,
                    failure,
                    M1AuthenticatedNewWindowOpaqueCustodyV1((
                        witness,
                        operations,
                        old_target,
                        target_plan,
                        target_bytes,
                        target_descriptor,
                        partitioned_memory,
                        prior_output,
                        old_source_rows,
                        old_bound_rows,
                        retired_rollover_custody,
                        residue,
                        binding,
                        recipe,
                        prior_windows,
                        ring_bytes,
                        next_queue_wait_timeout,
                        selected,
                        step,
                        intent,
                        retired_physical_metadata,
                        catalog_id,
                    )),
                );
                return Err(close_pending_submission_failure(engine, pending));
            }
        };
    let (lower, target, target_ranges) =
        match crate::authenticated_queue_rearm::replace_authenticated_rollover_workspace(
            lower,
            &old_target,
            *target_plan,
            target_bytes,
            target_descriptor,
        ) {
            Ok(value) => value,
            Err(failure) => {
                let pending = close_workspace_failure(
                    engine,
                    M1AuthenticatedSpeculativeRolloverSubmissionStageV1::TargetWorkspace,
                    failure,
                    M1AuthenticatedNewWindowOpaqueCustodyV1((
                        witness,
                        operations,
                        draft,
                        draft_ranges,
                        partitioned_memory,
                        prior_output,
                        old_source_rows,
                        old_bound_rows,
                        retired_rollover_custody,
                        residue,
                        binding,
                        recipe,
                        prior_windows,
                        ring_bytes,
                        next_queue_wait_timeout,
                        selected,
                        step,
                        intent,
                        retired_physical_metadata,
                        catalog_id,
                    )),
                );
                return Err(close_pending_submission_failure(engine, pending));
            }
        };
    let mut workspace_ranges = Vec::new();
    if workspace_ranges
        .try_reserve_exact(draft_ranges.len() + target_ranges.len())
        .is_err()
    {
        return Err(close_new_window_unbound(
            engine,
            M1AuthenticatedSpeculativeRolloverSubmissionStageV1::BoundRows,
            lower,
            (
                witness,
                operations,
                draft,
                target,
                draft_ranges,
                target_ranges,
                partitioned_memory,
                prior_output,
                old_source_rows,
                old_bound_rows,
                retired_rollover_custody,
                residue,
                binding,
                recipe,
                prior_windows,
                ring_bytes,
                next_queue_wait_timeout,
                selected,
                step,
                intent,
                retired_physical_metadata,
                catalog_id,
            ),
        ));
    }
    crate::m1_queue_rearm::append_workspace_ranges(
        &mut workspace_ranges,
        M1FullStepWorkspaceRole::Draft,
        &draft,
        draft_ranges,
    );
    crate::m1_queue_rearm::append_workspace_ranges(
        &mut workspace_ranges,
        M1FullStepWorkspaceRole::Target,
        &target,
        target_ranges,
    );
    let completion_output = match partitioned_memory
        .rotate_finite_speculative_output_for_new_window(
            next.target(),
            prior.target(),
            prior_output,
        ) {
        Ok(output) => output,
        Err(source) => {
            return Err(close_new_window_unbound(
                engine,
                M1AuthenticatedSpeculativeRolloverSubmissionStageV1::OutputActivation,
                lower,
                (
                    source,
                    witness,
                    operations,
                    draft,
                    target,
                    workspace_ranges,
                    partitioned_memory,
                    old_source_rows,
                    old_bound_rows,
                    retired_rollover_custody,
                    residue,
                    binding,
                    recipe,
                    prior_windows,
                    ring_bytes,
                    next_queue_wait_timeout,
                    selected,
                    step,
                    intent,
                    retired_physical_metadata,
                    catalog_id,
                ),
            ));
        }
    };
    let capture = match crate::m1_queue_rearm::retained_host_capture_ranges(&completion_output) {
        Ok(capture) => capture,
        Err(()) => {
            return Err(close_new_window_unbound(
                engine,
                M1AuthenticatedSpeculativeRolloverSubmissionStageV1::BoundRows,
                lower,
                (
                    witness,
                    operations,
                    draft,
                    target,
                    workspace_ranges,
                    partitioned_memory,
                    completion_output,
                    old_source_rows,
                    old_bound_rows,
                    retired_rollover_custody,
                    residue,
                    binding,
                    recipe,
                    prior_windows,
                    ring_bytes,
                    next_queue_wait_timeout,
                    selected,
                    step,
                    intent,
                    retired_physical_metadata,
                    catalog_id,
                ),
            ));
        }
    };
    let bound_rows = match crate::m1_queue_rearm::build_rollover_bound_rows(
        recipe.rows(),
        &old_source_rows,
        &old_bound_rows,
        recipe.workspace_composition(),
        &workspace_ranges,
        &capture,
    ) {
        Ok(rows) => rows,
        Err(()) => {
            return Err(close_new_window_unbound(
                engine,
                M1AuthenticatedSpeculativeRolloverSubmissionStageV1::BoundRows,
                lower,
                (
                    witness,
                    operations,
                    draft,
                    target,
                    workspace_ranges,
                    partitioned_memory,
                    completion_output,
                    old_source_rows,
                    old_bound_rows,
                    retired_rollover_custody,
                    residue,
                    binding,
                    recipe,
                    prior_windows,
                    ring_bytes,
                    next_queue_wait_timeout,
                    selected,
                    step,
                    intent,
                    retired_physical_metadata,
                    catalog_id,
                ),
            ));
        }
    };
    let custody = M1PhysicalQueueBatchCustodyV1::from_rearm_parts(
        crate::physical_fixed_batch::M1PhysicalQueueBatchRearmPartsV1 {
            catalog_id,
            selection: next.target(),
            physical_recipe: retired_physical_metadata.2,
            workspace_composition: retired_physical_metadata.3,
            workspace_owners: M1FullStepWorkspaceSubleaseOwners::paired_prefill(draft, target),
            partitioned_memory,
            completion_output,
            source_rows: old_source_rows,
            bound_rows: old_bound_rows,
            retired_rollover_custody,
        },
    );
    let batch = match crate::physical_fixed_batch::build_m1_authenticated_rollover_packet_batch_v1(
        &witness,
        &operations,
        recipe,
        bound_rows,
        custody,
    ) {
        Ok(crate::physical_fixed_batch::M1AuthenticatedQueuePacketBatchV1::PairedPrefill(
            batch,
        )) => batch,
        Ok(batch) => {
            return Err(close_new_window_unbound(
                engine,
                M1AuthenticatedSpeculativeRolloverSubmissionStageV1::PacketLowering,
                lower,
                (
                    batch,
                    witness,
                    operations,
                    step,
                    selected,
                    residue,
                    binding,
                    intent,
                    prior_windows,
                    next_queue_wait_timeout,
                    retired_physical_metadata.0,
                    retired_physical_metadata.1,
                ),
            ));
        }
        Err(source) => {
            return Err(close_new_window_unbound(
                engine,
                M1AuthenticatedSpeculativeRolloverSubmissionStageV1::PacketLowering,
                lower,
                (
                    source,
                    witness,
                    operations,
                    step,
                    selected,
                    residue,
                    binding,
                    intent,
                    prior_windows,
                    next_queue_wait_timeout,
                    retired_physical_metadata.0,
                    retired_physical_metadata.1,
                ),
            ));
        }
    };
    let (queue, rollover) = match rollover_case(
        lower,
        ring_bytes,
        *batch,
        witness,
        operations,
        step,
        predecessor_generation,
        |case| M1AuthenticatedPhysicalQueueSessionV1::PairedPrefill(Box::new(case)),
    ) {
        Ok(value) => value,
        Err(mut source) => {
            source.retained = Box::new((
                source.retained,
                selected,
                residue,
                binding,
                intent,
                prior_windows,
                next_queue_wait_timeout,
            ));
            return Err(close_pending_submission_failure(
                engine,
                PendingM1AuthenticatedSpeculativeRolloverSubmissionFailureV1::NativePaired(
                    Box::new(source),
                ),
            ));
        }
    };
    if rollover.previous_dispatch_generation() != predecessor_generation
        || predecessor_generation
            .checked_add(1)
            .is_none_or(|generation| rollover.replacement_dispatch_generation() != generation)
    {
        return Err(close_pending_submission_failure(
            engine,
            PendingM1AuthenticatedSpeculativeRolloverSubmissionFailureV1::Observation {
                queue: Box::new(queue),
                retained: Box::new((
                    selected,
                    residue,
                    binding,
                    intent,
                    prior_windows,
                    next_queue_wait_timeout,
                    rollover,
                )),
            },
        ));
    }
    let queue = match queue.submit() {
        Ok(queue) => queue,
        Err(source) => {
            return Err(close_pending_submission_failure(
                engine,
                PendingM1AuthenticatedSpeculativeRolloverSubmissionFailureV1::Submit {
                    source: Box::new(source),
                    retained: Box::new((
                        selected,
                        residue,
                        binding,
                        intent,
                        prior_windows,
                        next_queue_wait_timeout,
                        rollover,
                    )),
                },
            ));
        }
    };
    let M1AuthenticatedSpeculativeNewWindowResidueV1 {
        checked,
        members,
        mut terminal,
        logical_accepted_counts,
        externally_published_counts,
        release_counts,
        completed_members,
        total_released,
        history,
    } = residue;
    debug_assert!(terminal.capacity() >= terminal.len() + members.len());
    for member in members {
        terminal.push(match member {
            M1ReleasedDeviceKvMemberV1::Terminal(terminal) => terminal,
            M1ReleasedDeviceKvMemberV1::Active(_) => {
                unreachable!("all-terminal new-window preflight rejected active custody")
            }
        });
    }
    let previous_epoch = checked.epoch();
    let bridge = M1AuthenticatedSpeculativeNewWindowBridgeV1 {
        speculative_successor,
        intent,
        prior_windows,
        next_queue_wait_timeout,
    };
    Ok(M1AuthenticatedSpeculativeNewWindowPublishedV1 {
        published: M1AuthenticatedRearmedPublishedQueueV1::from_authenticated_new_window(
            queue,
            selected,
            terminal,
            previous_epoch,
            checked,
            logical_accepted_counts,
            externally_published_counts,
            release_counts,
            completed_members,
            total_released,
            history,
            predecessor_observation,
            device,
            rollover,
            bridge,
        ),
        queue_wait_timeout: next_queue_wait_timeout,
    })
}

fn submit_m1_authenticated_speculative_rollover_pending_v1<const C: usize>(
    engine: &mut Engine<C>,
    prepared: M1AuthenticatedPreparedSpeculativeRolloverV1,
    ring_bytes: u32,
    queue_wait_timeout: crate::M1QueueWaitTimeoutV1,
) -> Result<
    crate::M1AuthenticatedSpeculativeRolloverPublishedV1,
    PendingM1AuthenticatedSpeculativeRolloverSubmissionFailureV1,
> {
    let M1AuthenticatedPreparedSpeculativeRolloverV1 {
        prior,
        next,
        reason,
        queue,
        selected,
        residue,
        prepared,
        recipe,
        logical,
    } = prepared;
    let old = queue.custody();
    if engine.is_faulted()
        || queue.shape() != M1PhysicalFixedBatchShapeV1::PairedPrefill
        || prior.shape() != M1PhysicalFixedBatchShapeV1::PairedPrefill
        || !matches!(
            next.shape(),
            M1PhysicalFixedBatchShapeV1::SpeculativeK4
                | M1PhysicalFixedBatchShapeV1::SpeculativeK8
                | M1PhysicalFixedBatchShapeV1::SpeculativeK16
        )
        || reason != M1ServingRolloverReasonV1::Mode
        || old.selection() != prior.target()
        || prepared.kind() != M1FullStepWorkspaceInputKind::SpeculativeRound
        || prepared.step().kv_reservations().target_selection() != next.target()
        || recipe.workspace_composition().workspace_plans() != prepared.plans()
        || recipe.requires_future_materialization()
        || recipe.rows().len() != next.shape().packet_count()
        || recipe.kernarg_recipe().images().len() != next.shape().packet_count()
        || selected.is_empty()
        || selected
            .iter()
            .any(|cache| cache.projection().device != old.device())
    {
        let (shape, lower, witness, operations, custody) = queue.into_rearm_parts();
        return Err(close_unbound(
            engine,
            M1AuthenticatedSpeculativeRolloverSubmissionStageV1::Preflight,
            lower,
            (
                shape, witness, operations, custody, prior, next, reason, selected, residue,
                prepared, recipe, logical,
            ),
        ));
    }

    let (old_shape, lower, witness, operations, custody) = queue.into_rearm_parts();
    let predecessor_observation = lower.observation();
    let predecessor_generation = lower.detached_dispatch_generation();
    let device = custody.device();
    let crate::physical_fixed_batch::M1PhysicalQueueBatchRearmPartsV1 {
        catalog_id,
        selection: old_selection,
        physical_recipe: old_physical_recipe,
        workspace_composition: old_workspace_composition,
        workspace_owners,
        mut partitioned_memory,
        completion_output: prior_output,
        source_rows: old_source_rows,
        bound_rows: old_bound_rows,
        retired_rollover_custody,
    } = custody.into_rearm_parts();
    let (plans, images, step) = prepared.into_rearm_parts();
    let (old_draft, old_target, draft_plan, target_plan, draft_bytes, target_bytes) =
        match (workspace_owners, plans, images) {
            (
                M1FullStepWorkspaceSubleaseOwners::PairedPrefill { draft, target },
                M1FullStepWorkspacePlans::SpeculativeRound {
                    draft_decode,
                    target_speculative,
                },
                M1FullStepWorkspaceImagesV1::SpeculativeRound {
                    draft_decode: draft_bytes,
                    target_speculative: target_bytes,
                },
            ) => (
                draft,
                target,
                draft_decode,
                target_speculative,
                draft_bytes,
                target_bytes,
            ),
            (workspace_owners, plans, images) => {
                return Err(close_unbound(
                    engine,
                    M1AuthenticatedSpeculativeRolloverSubmissionStageV1::Preflight,
                    lower,
                    (
                        (
                            old_shape,
                            witness,
                            operations,
                            catalog_id,
                            old_selection,
                            old_physical_recipe,
                            old_workspace_composition,
                            workspace_owners,
                            partitioned_memory,
                            prior_output,
                        ),
                        (
                            old_source_rows,
                            old_bound_rows,
                            plans,
                            images,
                            step,
                            selected,
                            residue,
                            recipe,
                            logical,
                        ),
                    ),
                ));
            }
        };
    let draft_descriptor = match crate::authenticated_queue_rearm::descriptor(
        M1InitializedWorkspaceSlotV1::SpeculativeDraftDecode,
        &draft_bytes,
    ) {
        Ok(descriptor) => descriptor,
        Err(()) => {
            return Err(close_unbound(
                engine,
                M1AuthenticatedSpeculativeRolloverSubmissionStageV1::DraftWorkspace,
                lower,
                (
                    (
                        witness,
                        operations,
                        old_shape,
                        catalog_id,
                        old_selection,
                        old_physical_recipe,
                        old_workspace_composition,
                        old_draft,
                        old_target,
                        draft_plan,
                        target_plan,
                    ),
                    (
                        draft_bytes,
                        target_bytes,
                        partitioned_memory,
                        prior_output,
                        old_source_rows,
                        old_bound_rows,
                        step,
                        selected,
                        residue,
                        recipe,
                        logical,
                    ),
                ),
            ));
        }
    };
    let target_descriptor = match crate::authenticated_queue_rearm::descriptor(
        M1InitializedWorkspaceSlotV1::SpeculativeTarget,
        &target_bytes,
    ) {
        Ok(descriptor) => descriptor,
        Err(()) => {
            return Err(close_unbound(
                engine,
                M1AuthenticatedSpeculativeRolloverSubmissionStageV1::TargetWorkspace,
                lower,
                (
                    (
                        witness,
                        operations,
                        old_shape,
                        catalog_id,
                        old_selection,
                        old_physical_recipe,
                        old_workspace_composition,
                        old_draft,
                        old_target,
                        draft_plan,
                        target_plan,
                    ),
                    (
                        draft_bytes,
                        target_bytes,
                        partitioned_memory,
                        prior_output,
                        old_source_rows,
                        old_bound_rows,
                        step,
                        selected,
                        residue,
                        recipe,
                        logical,
                    ),
                ),
            ));
        }
    };
    let (lower, draft, draft_ranges) =
        match crate::authenticated_queue_rearm::replace_authenticated_rollover_workspace(
            lower,
            &old_draft,
            *draft_plan,
            draft_bytes,
            draft_descriptor,
        ) {
            Ok(value) => value,
            Err(failure) => {
                return Err(close_workspace_failure(
                    engine,
                    M1AuthenticatedSpeculativeRolloverSubmissionStageV1::DraftWorkspace,
                    failure,
                    (
                        (
                            witness,
                            operations,
                            old_shape,
                            catalog_id,
                            old_selection,
                            old_physical_recipe,
                            old_workspace_composition,
                            old_target,
                            target_plan,
                            target_bytes,
                        ),
                        (
                            partitioned_memory,
                            prior_output,
                            old_source_rows,
                            old_bound_rows,
                            step,
                            selected,
                            residue,
                            recipe,
                            logical,
                        ),
                    ),
                ));
            }
        };
    let (lower, target, target_ranges) =
        match crate::authenticated_queue_rearm::replace_authenticated_rollover_workspace(
            lower,
            &old_target,
            *target_plan,
            target_bytes,
            target_descriptor,
        ) {
            Ok(value) => value,
            Err(failure) => {
                return Err(close_workspace_failure(
                    engine,
                    M1AuthenticatedSpeculativeRolloverSubmissionStageV1::TargetWorkspace,
                    failure,
                    (
                        (
                            witness,
                            operations,
                            old_shape,
                            catalog_id,
                            old_selection,
                            old_physical_recipe,
                            old_workspace_composition,
                            draft,
                            draft_ranges,
                        ),
                        (
                            partitioned_memory,
                            prior_output,
                            old_source_rows,
                            old_bound_rows,
                            step,
                            selected,
                            residue,
                            recipe,
                            logical,
                        ),
                    ),
                ));
            }
        };
    let mut workspace_ranges = Vec::new();
    if workspace_ranges
        .try_reserve_exact(draft_ranges.len() + target_ranges.len())
        .is_err()
    {
        return Err(close_unbound(
            engine,
            M1AuthenticatedSpeculativeRolloverSubmissionStageV1::BoundRows,
            lower,
            (
                (
                    witness,
                    operations,
                    old_shape,
                    catalog_id,
                    old_selection,
                    old_physical_recipe,
                    old_workspace_composition,
                    draft,
                    target,
                    draft_ranges,
                ),
                (
                    target_ranges,
                    partitioned_memory,
                    prior_output,
                    old_source_rows,
                    old_bound_rows,
                    step,
                    selected,
                    residue,
                    recipe,
                    logical,
                ),
            ),
        ));
    }
    crate::m1_queue_rearm::append_workspace_ranges(
        &mut workspace_ranges,
        M1FullStepWorkspaceRole::Draft,
        &draft,
        draft_ranges,
    );
    crate::m1_queue_rearm::append_workspace_ranges(
        &mut workspace_ranges,
        M1FullStepWorkspaceRole::Target,
        &target,
        target_ranges,
    );
    let completion_output = match partitioned_memory
        .activate_finite_speculative_rollover_output(next.target(), prior_output)
    {
        Ok(output) => output,
        Err(source) => {
            return Err(close_unbound(
                engine,
                M1AuthenticatedSpeculativeRolloverSubmissionStageV1::OutputActivation,
                lower,
                (
                    (
                        source,
                        witness,
                        operations,
                        old_shape,
                        catalog_id,
                        old_selection,
                        old_physical_recipe,
                        old_workspace_composition,
                        draft,
                        target,
                    ),
                    (
                        workspace_ranges,
                        partitioned_memory,
                        old_source_rows,
                        old_bound_rows,
                        step,
                        selected,
                        residue,
                        recipe,
                        logical,
                    ),
                ),
            ));
        }
    };
    let capture = match crate::m1_queue_rearm::retained_host_capture_ranges(&completion_output) {
        Ok(capture) => capture,
        Err(()) => {
            return Err(close_unbound(
                engine,
                M1AuthenticatedSpeculativeRolloverSubmissionStageV1::BoundRows,
                lower,
                (
                    (
                        witness,
                        operations,
                        draft,
                        target,
                        workspace_ranges,
                        partitioned_memory,
                        completion_output,
                    ),
                    (
                        old_source_rows,
                        old_bound_rows,
                        step,
                        selected,
                        residue,
                        recipe,
                        logical,
                    ),
                ),
            ));
        }
    };
    let bound_rows = match crate::m1_queue_rearm::build_rollover_bound_rows(
        recipe.rows(),
        &old_source_rows,
        &old_bound_rows,
        recipe.workspace_composition(),
        &workspace_ranges,
        &capture,
    ) {
        Ok(rows) => rows,
        Err(()) => {
            return Err(close_unbound(
                engine,
                M1AuthenticatedSpeculativeRolloverSubmissionStageV1::BoundRows,
                lower,
                (
                    (
                        witness,
                        operations,
                        draft,
                        target,
                        workspace_ranges,
                        partitioned_memory,
                        completion_output,
                    ),
                    (
                        old_source_rows,
                        old_bound_rows,
                        step,
                        selected,
                        residue,
                        recipe,
                        logical,
                    ),
                ),
            ));
        }
    };
    let custody = M1PhysicalQueueBatchCustodyV1::from_rearm_parts(
        crate::physical_fixed_batch::M1PhysicalQueueBatchRearmPartsV1 {
            catalog_id,
            selection: next.target(),
            physical_recipe: old_physical_recipe,
            workspace_composition: old_workspace_composition,
            workspace_owners: M1FullStepWorkspaceSubleaseOwners::speculative_round(draft, target),
            partitioned_memory,
            completion_output,
            source_rows: old_source_rows,
            bound_rows: old_bound_rows,
            retired_rollover_custody,
        },
    );
    let batch = match crate::physical_fixed_batch::build_m1_authenticated_rollover_packet_batch_v1(
        &witness,
        &operations,
        recipe,
        bound_rows,
        custody,
    ) {
        Ok(batch) => batch,
        Err(source) => {
            return Err(close_unbound(
                engine,
                M1AuthenticatedSpeculativeRolloverSubmissionStageV1::PacketLowering,
                lower,
                (
                    source, witness, operations, step, selected, residue, logical,
                ),
            ));
        }
    };
    let (queue, rollover) = match batch {
        crate::physical_fixed_batch::M1AuthenticatedQueuePacketBatchV1::SpeculativeK4(batch) => {
            match rollover_case(
                lower,
                ring_bytes,
                *batch,
                witness,
                operations,
                step,
                predecessor_generation,
                |case| M1AuthenticatedPhysicalQueueSessionV1::SpeculativeK4(Box::new(case)),
            ) {
                Ok(value) => value,
                Err(mut source) => {
                    engine.quarantine_m1_queue_rearm_failure();
                    source.retained = Box::new((source.retained, selected, residue, logical));
                    return Err(
                        PendingM1AuthenticatedSpeculativeRolloverSubmissionFailureV1::NativeK4(
                            Box::new(source),
                        ),
                    );
                }
            }
        }
        crate::physical_fixed_batch::M1AuthenticatedQueuePacketBatchV1::SpeculativeK8(batch) => {
            match rollover_case(
                lower,
                ring_bytes,
                *batch,
                witness,
                operations,
                step,
                predecessor_generation,
                |case| M1AuthenticatedPhysicalQueueSessionV1::SpeculativeK8(Box::new(case)),
            ) {
                Ok(value) => value,
                Err(mut source) => {
                    engine.quarantine_m1_queue_rearm_failure();
                    source.retained = Box::new((source.retained, selected, residue, logical));
                    return Err(
                        PendingM1AuthenticatedSpeculativeRolloverSubmissionFailureV1::NativeK8(
                            Box::new(source),
                        ),
                    );
                }
            }
        }
        crate::physical_fixed_batch::M1AuthenticatedQueuePacketBatchV1::SpeculativeK16(batch) => {
            match rollover_case(
                lower,
                ring_bytes,
                *batch,
                witness,
                operations,
                step,
                predecessor_generation,
                |case| M1AuthenticatedPhysicalQueueSessionV1::SpeculativeK16(Box::new(case)),
            ) {
                Ok(value) => value,
                Err(mut source) => {
                    engine.quarantine_m1_queue_rearm_failure();
                    source.retained = Box::new((source.retained, selected, residue, logical));
                    return Err(
                        PendingM1AuthenticatedSpeculativeRolloverSubmissionFailureV1::NativeK16(
                            Box::new(source),
                        ),
                    );
                }
            }
        }
        batch => {
            return Err(close_unbound(
                engine,
                M1AuthenticatedSpeculativeRolloverSubmissionStageV1::PacketLowering,
                lower,
                (batch, witness, operations, step, selected, residue, logical),
            ));
        }
    };
    if rollover.previous_dispatch_generation() != predecessor_generation
        || predecessor_generation
            .checked_add(1)
            .is_none_or(|next| rollover.replacement_dispatch_generation() != next)
    {
        engine.quarantine_m1_queue_rearm_failure();
        return Err(
            PendingM1AuthenticatedSpeculativeRolloverSubmissionFailureV1::Observation {
                queue: Box::new(queue),
                retained: Box::new((selected, residue, logical, rollover)),
            },
        );
    }
    let queue = match queue.submit() {
        Ok(queue) => queue,
        Err(source) => {
            engine.quarantine_m1_queue_rearm_failure();
            return Err(
                PendingM1AuthenticatedSpeculativeRolloverSubmissionFailureV1::Submit {
                    source: Box::new(source),
                    retained: Box::new((selected, residue, logical, rollover)),
                },
            );
        }
    };
    let M1AuthenticatedSpeculativeRolloverResidueV1 {
        checked,
        logical_accepted_counts,
        externally_published_counts,
        release_counts,
        completed_members,
        total_released,
        terminal,
        history,
    } = residue;
    let previous_epoch = checked.epoch();
    let published = M1AuthenticatedRearmedPublishedQueueV1::from_authenticated_rollover(
        queue,
        selected,
        terminal,
        history,
        previous_epoch,
        checked,
        logical_accepted_counts,
        externally_published_counts,
        release_counts,
        completed_members,
        total_released,
        predecessor_observation,
        device,
        rollover,
    );
    Ok(crate::M1AuthenticatedSpeculativeRolloverPublishedV1::new(
        published,
        logical.coordinator,
        logical.epoch,
        logical.lineage,
        logical.prior_windows,
        logical
            .frozen_queue_wait_timeout
            .unwrap_or(queue_wait_timeout),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        device_cache::test_support::bind_gfx942_device, M1ServingCompletionDispositionV1,
        M1ServingRegistryV1, M1SpeculativeGenerationPolicyV1, GFX942_PROCESSOR,
        GFX942_TARGET_FEATURES,
    };
    use ferric_spec::{
        Identity, LogicalKvState, PhysicalPageId, Qwen3ExecutionMode, Qwen3ModelRole,
        Qwen3PlanBucket, Qwen3PlanSelection, RequestId,
    };

    fn serving_plan(mode: Qwen3ExecutionMode, bucket: Qwen3PlanBucket) -> M1ServingPlanV1 {
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

    fn active_logical(request: RequestId, role: Qwen3ModelRole, tokens: u32) -> LogicalKvState {
        LogicalKvState {
            request,
            role,
            lifecycle: PhysicalKvLifecycle::Active,
            resident_tokens: tokens,
            committed_tokens: tokens,
        }
    }

    #[test]
    fn authenticated_successor_span_accepts_owned_prefix_and_cross_page_growth() {
        let request = RequestId::new(3, 7);
        assert_eq!(
            authenticated_speculative_successor_page_span(
                active_logical(request, Qwen3ModelRole::Draft06B, 127),
                8,
                request,
                Qwen3ModelRole::Draft06B,
                127,
                1,
            ),
            Some(M1AuthenticatedSpeculativeSuccessorPageSpanV1 {
                first_page: 8,
                page_count: 0,
            })
        );
        assert_eq!(
            authenticated_speculative_successor_page_span(
                active_logical(request, Qwen3ModelRole::Draft06B, 128),
                8,
                request,
                Qwen3ModelRole::Draft06B,
                128,
                16,
            ),
            Some(M1AuthenticatedSpeculativeSuccessorPageSpanV1 {
                first_page: 8,
                page_count: 1,
            })
        );
        assert_eq!(
            authenticated_speculative_successor_page_span(
                active_logical(request, Qwen3ModelRole::Target8B, 128),
                8,
                request,
                Qwen3ModelRole::Target8B,
                128,
                17,
            ),
            Some(M1AuthenticatedSpeculativeSuccessorPageSpanV1 {
                first_page: 8,
                page_count: 2,
            })
        );
        assert_eq!(
            authenticated_speculative_successor_page_span(
                active_logical(request, Qwen3ModelRole::Target8B, 127),
                7,
                request,
                Qwen3ModelRole::Target8B,
                127,
                1,
            ),
            None
        );
    }

    #[test]
    fn authenticated_successor_span_rejects_stale_generation_and_role_swap() {
        let request = RequestId::new(3, 7);
        let stale = RequestId::new(request.slot(), request.generation() + 1);
        let logical = active_logical(request, Qwen3ModelRole::Draft06B, 128);
        assert_eq!(
            authenticated_speculative_successor_page_span(
                logical,
                8,
                stale,
                Qwen3ModelRole::Draft06B,
                128,
                1,
            ),
            None
        );
        assert_eq!(
            authenticated_speculative_successor_page_span(
                logical,
                8,
                request,
                Qwen3ModelRole::Target8B,
                128,
                1,
            ),
            None
        );
    }

    #[test]
    fn authenticated_tail_materialization_rejects_cross_k_binding_drift() {
        let k4 = crate::M1SpeculativePhysicalShapeV1::from_selection(Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Speculative,
            bucket: Qwen3PlanBucket::SpeculativeS1K4C8192,
        })
        .ok();
        let k8 = crate::M1SpeculativePhysicalShapeV1::from_selection(Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Speculative,
            bucket: Qwen3PlanBucket::SpeculativeS1K8C8192,
        })
        .ok();
        assert!(authenticated_speculative_tail_binding_width_matches(k4, 4));
        assert!(!authenticated_speculative_tail_binding_width_matches(k4, 8));
        assert!(authenticated_speculative_tail_binding_width_matches(k8, 8));
        assert!(!authenticated_speculative_tail_binding_width_matches(k8, 4));
        assert!(!authenticated_speculative_tail_binding_width_matches(
            None, 4
        ));
    }

    #[test]
    fn authenticated_successor_committed_roster_binds_role_request_and_page_order() {
        let request = RequestId::new(3, 7);
        let device = bind_gfx942_device(
            Identity::new([1; 32]),
            7,
            GFX942_PROCESSOR,
            GFX942_TARGET_FEATURES,
        )
        .unwrap();
        let allocation = Identity::new([2; 32]);
        let lease = |request, role, index, generation| {
            DeviceKvPageLease::from_contracted_workspace_bridge_test_allocation(
                device,
                allocation,
                request,
                PhysicalPageId::new(role, index, generation),
            )
        };
        let spans = [M1AuthenticatedSpeculativeSuccessorLanePageSpansV1 {
            request,
            draft: M1AuthenticatedSpeculativeSuccessorPageSpanV1 {
                first_page: 8,
                page_count: 1,
            },
            target: M1AuthenticatedSpeculativeSuccessorPageSpanV1 {
                first_page: 8,
                page_count: 2,
            },
        }];
        let exact = vec![
            lease(request, Qwen3ModelRole::Draft06B, 8, 3),
            lease(request, Qwen3ModelRole::Target8B, 8, 5),
            lease(request, Qwen3ModelRole::Target8B, 9, 5),
        ];
        assert!(authenticated_speculative_successor_lease_roster_matches(
            &exact, &spans
        ));
        let role_swap = vec![
            lease(request, Qwen3ModelRole::Target8B, 8, 3),
            lease(request, Qwen3ModelRole::Draft06B, 8, 5),
            lease(request, Qwen3ModelRole::Target8B, 9, 5),
        ];
        assert!(!authenticated_speculative_successor_lease_roster_matches(
            &role_swap, &spans,
        ));
        let stale = vec![
            lease(
                RequestId::new(request.slot(), request.generation() + 1),
                Qwen3ModelRole::Draft06B,
                8,
                3,
            ),
            lease(request, Qwen3ModelRole::Target8B, 8, 5),
            lease(request, Qwen3ModelRole::Target8B, 9, 5),
        ];
        assert!(!authenticated_speculative_successor_lease_roster_matches(
            &stale, &spans,
        ));
        let duplicate = vec![
            lease(request, Qwen3ModelRole::Draft06B, 8, 3),
            lease(request, Qwen3ModelRole::Target8B, 8, 5),
            lease(request, Qwen3ModelRole::Target8B, 8, 5),
        ];
        assert!(!authenticated_speculative_successor_lease_roster_matches(
            &duplicate, &spans,
        ));
    }

    #[test]
    fn authenticated_successor_materialization_is_post_detach_and_fail_closed() {
        let source = include_str!("authenticated_queue_rollover.rs");
        let production = source.split("#[cfg(test)]").next().unwrap();
        let pending_start = production
            .find("fn schedule_m1_authenticated_speculative_rollover_pending_v1")
            .unwrap();
        let pending = &production[pending_start..];
        let detach = pending
            .find("let mut queue = match queue.detach()")
            .unwrap();
        let materialize = pending
            .find("materialize_authenticated_speculative_rollover_inputs")
            .unwrap();
        let dispatch = pending.find("engine.dispatch_m1_exact_ready").unwrap();
        assert!(detach < materialize && materialize < dispatch);

        let wrapper_start = production
            .find("fn materialize_authenticated_speculative_rollover_inputs")
            .unwrap();
        let wrapper_end = production[wrapper_start..]
            .find("\n#[allow(clippy::too_many_arguments)]\nfn schedule_m1_authenticated")
            .map(|offset| wrapper_start + offset)
            .unwrap();
        let wrapper = &production[wrapper_start..wrapper_end];
        assert!(
            wrapper.contains("let draft_round_tokens = u32::from(binding.shape().draft_tokens())")
        );
        assert!(wrapper.contains("materialize_m1_authenticated_speculative_tail_pages_v1"));
        assert!(!wrapper.contains(".unwrap()"));
        assert!(!wrapper.contains(".expect("));

        let shared_start = production
            .find("pub(crate) fn materialize_m1_authenticated_speculative_tail_pages_v1")
            .unwrap();
        let shared_end = production[shared_start..]
            .find("\nfn successor_page_failure")
            .map(|offset| shared_start + offset)
            .unwrap();
        let shared = &production[shared_start..shared_end];
        let admit = shared
            .find("admit_authenticated_successor_page_set(lanes)")
            .unwrap();
        let commit = shared
            .find("commit_authenticated_successor_page_set(admission)")
            .unwrap();
        let validate = shared
            .find("authenticated_speculative_successor_lease_roster_matches")
            .unwrap();
        let split = shared.find("draft.extend(").unwrap();
        assert!(admit < commit && commit < validate && validate < split);
        assert!(
            shared.contains("M1AuthenticatedSpeculativeTailPageMaterializationFailureV1::Commit")
        );
        assert!(shared.contains(
            "M1AuthenticatedSpeculativeTailPageMaterializationFailureV1::CommittedRoster"
        ));
        assert!(!shared.contains(".unwrap()"));
        assert!(!shared.contains(".expect("));

        let released_start = production
            .find("pub fn schedule_successor<const C: usize>(")
            .unwrap();
        let released_end = production[released_start..]
            .find("\n    }")
            .map(|offset| released_start + offset)
            .unwrap();
        let released = &production[released_start..released_end];
        assert!(released.contains("M1AuthenticatedSpeculativeNewWindowSuccessorInputsV1"));
        assert!(!released.contains("M1FiniteSpeculativeQueueRolloverKvInputsV1"));
    }

    #[test]
    fn authenticated_new_window_limit_counts_the_current_window() {
        assert_eq!(M1_MAX_AUTHENTICATED_SPECULATIVE_WINDOWS_V1, 20);
        assert!(authenticated_new_window_transition_within_limit(18));
        assert!(!authenticated_new_window_transition_within_limit(19));
        assert!(!authenticated_new_window_transition_within_limit(
            usize::MAX
        ));
    }

    #[test]
    fn hostile_new_window_expiry_is_checked_immediately_around_wait() {
        let source = include_str!("authenticated_queue_rollover.rs");
        let observe = source
            .split("pub(crate) fn observe_with_deadline")
            .nth(1)
            .and_then(|tail| tail.split("fn cancel_and_close").next())
            .expect("deadline-bound new-window observation is present");
        let mut tail = observe;
        for needle in [
            "let Some(wait_timeout)",
            "deadline_expired(Boundary::BeforeCompletionWait",
            "self.published.wait_for(wait_timeout.milliseconds(), engine)",
            "deadline_expired(Boundary::AfterCompletionWait",
        ] {
            let position = tail
                .find(needle)
                .unwrap_or_else(|| panic!("missing deadline-bound new-window wait step {needle}"));
            tail = &tail[position + needle.len()..];
        }
    }

    #[test]
    fn hostile_new_window_settlement_deadlines_destroy_exactly_once() {
        use crate::authenticated_resident_session::M1AuthenticatedResidentDeadlineBoundaryV1 as Boundary;
        use crate::authenticated_speculative_executor::{
            close_deadline_phase, quarantined_disposition, released_disposition,
        };

        let phases = [
            ("observed", Boundary::BeforeReadback),
            ("readback-after", Boundary::AfterReadback),
            ("readback-before-settlement", Boundary::BeforeSettlement),
            ("completion-after", Boundary::AfterSettlement),
            ("completion-before-release", Boundary::BeforeSettlement),
            ("released-after", Boundary::AfterSettlement),
        ];
        for (phase, boundary) in phases {
            let mut destroys = 0;
            let disposition = close_deadline_phase((phase, boundary), |retained| {
                assert_eq!(retained, (phase, boundary));
                destroys += 1;
                released_disposition((retained, "model native release"))
            });
            assert_eq!(destroys, 1);
            assert!(disposition.queue_released());
        }

        let mut destroys = 0;
        let disposition = close_deadline_phase("released-after", |retained| {
            destroys += 1;
            quarantined_disposition((retained, "model native destruction failure"))
        });
        assert_eq!(destroys, 1);
        assert!(!disposition.queue_released());

        let production = include_str!("authenticated_queue_rollover.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();
        assert!(!production.contains("Deadline(Box<dyn fmt::Debug>)"));
        for typed in [
            "DeadlineObserved",
            "DeadlineReadback",
            "DeadlineCompletion",
            "DeadlineRelease",
        ] {
            assert!(production.contains(typed));
        }
        for destroy in [
            "destroy_queue_and_retain_custody(engine)",
            "destroy_queue_and_retain_any(engine",
            "destroy_queue_and_retain_round(engine)",
        ] {
            assert!(production.contains(destroy));
        }
    }

    #[test]
    fn authenticated_new_window_surface_keeps_generic_queue_authority_private() {
        let source = include_str!("authenticated_queue_rollover.rs");
        let production = source.split("#[cfg(test)]").next().unwrap();

        let published_start = production
            .find("impl M1AuthenticatedSpeculativeNewWindowPublishedV1 {")
            .unwrap();
        let published_end = production[published_start..]
            .find("\nfn authenticated_new_window_observed(")
            .map(|offset| published_start + offset)
            .unwrap();
        let published = &production[published_start..published_end];
        assert!(published.contains("pub fn observe<const C: usize>("));
        assert!(!published.contains("pub fn wait("));
        assert!(!published.contains("pub fn wait_for("));
        assert!(!published.contains("pub fn complete("));
        assert!(!published.contains("into_parts"));

        let observed_start = production
            .find("impl M1AuthenticatedSpeculativeNewWindowObservedV1 {")
            .unwrap();
        let observed_end = production[observed_start..]
            .find("\n/// Stable settlement failure class")
            .map(|offset| observed_start + offset)
            .unwrap();
        let observed = &production[observed_start..observed_end];
        assert!(observed.contains("pub fn settle<const C: usize>("));
        assert!(!observed.contains("CompletionWireSemanticExpectation"));
        assert!(!observed.contains("pub fn image("));
        assert!(!observed.contains("pub fn raw_bytes("));
        assert!(!observed.contains("into_parts"));

        let released_start = production
            .find("impl M1AuthenticatedSpeculativeNewWindowReleasedV1 {")
            .unwrap();
        let released_end = production[released_start..]
            .find("\nimpl M1AuthenticatedSpeculativeNewWindowPublishedV1 {")
            .map(|offset| released_start + offset)
            .unwrap();
        let released = &production[released_start..released_end];
        assert!(released.contains("pub fn schedule_successor<const C: usize>("));
        assert!(!released.contains("into_parts"));
        assert!(!released.contains("current_released"));

        assert!(production.contains(
            "pub(crate) fn schedule_m1_authenticated_speculative_new_window_successor_v1"
        ));
        assert!(!production
            .contains("pub fn schedule_m1_authenticated_speculative_new_window_successor_v1"));
    }

    #[test]
    fn authenticated_new_window_detach_retains_complete_residue() {
        let source = include_str!("authenticated_queue_rollover.rs");
        let schedule_start = source
            .find("pub fn schedule_m1_authenticated_speculative_new_window_v1")
            .unwrap();
        let schedule_end = source[schedule_start..]
            .find("\nimpl M1AuthenticatedSpeculativeNewWindowSchedulePreDetachRetryV1")
            .map(|offset| schedule_start + offset)
            .unwrap();
        let schedule = &source[schedule_start..schedule_end];
        let residue = schedule
            .find("let residue = M1AuthenticatedSpeculativeNewWindowResidueV1 {")
            .unwrap();
        let detach = schedule.find("let queue = match queue.detach() {").unwrap();
        assert!(residue < detach);
        let detach_failure = &schedule[detach..schedule.find("let (binding,").unwrap()];
        let source_owner = detach_failure.find("source,").unwrap();
        let residue_owner = detach_failure[source_owner..].find("residue,").unwrap();
        assert!(residue_owner > 0);
    }

    #[test]
    fn authenticated_new_window_archives_then_resets_active_history() {
        let source = include_str!("authenticated_queue_rollover.rs");
        let join_start = source
            .find("pub(crate) fn schedule_m1_authenticated_speculative_new_window_successor_v1")
            .unwrap();
        let join_end = source[join_start..]
            .find("\nfn schedule_m1_authenticated_speculative_rollover_pending_v1")
            .map(|offset| join_start + offset)
            .unwrap();
        let join = &source[join_start..join_end];
        let archive = join.find(".attach_physical_history(history)").unwrap();
        let reset = join
            .find("history: crate::m1_queue_rearm::M1RearmRoundHistoryV1::Empty")
            .unwrap();
        assert!(archive < reset);
    }

    fn planned_rollover(
        prior: M1ServingPlanV1,
        next: M1ServingPlanV1,
        members: usize,
    ) -> M1ServingBatchPlanV1 {
        let mut registry = M1ServingRegistryV1::<8>::new().unwrap();
        for lane in 0..members {
            registry
                .admit(RequestId::new(u32::try_from(lane).unwrap(), 1), prior)
                .unwrap();
        }
        let prefill = registry.plan_next().unwrap().unwrap();
        let epoch = prefill.epoch();
        let reservation = registry.reserve_publication(prefill).unwrap();
        let identity = reservation.registry_identity();
        registry.record_publication(reservation).unwrap();
        let dispositions = vec![M1ServingCompletionDispositionV1::Continue(next); members];
        registry
            .preflight_completion_exact_for(identity, epoch, &dispositions)
            .unwrap();
        registry.apply_preflighted_completion(epoch, &dispositions);
        registry.plan_next().unwrap().unwrap()
    }

    #[test]
    fn rollover_seed_association_rejects_anchor_cursor_and_history_splices() {
        let request = RequestId::new(0, 7);
        let policy = M1SpeculativeGenerationPolicyV1::new(8, &[99]).unwrap();
        let seed = crate::M1SpeculativeMemberSeedV1::new(request, 42, 17, 17, policy);
        assert!(exact_member_association_values(
            seed,
            request,
            request,
            &[42],
            17,
            17,
            1,
            1,
        ));
        assert!(!exact_member_association_values(
            seed,
            request,
            request,
            &[41],
            17,
            17,
            1,
            1,
        ));
        assert!(!exact_member_association_values(
            seed,
            request,
            request,
            &[42],
            18,
            17,
            1,
            1,
        ));
        assert!(!exact_member_association_values(
            seed,
            request,
            request,
            &[42],
            17,
            18,
            1,
            1,
        ));
        assert!(!exact_member_association_values(
            seed,
            RequestId::new(1, 7),
            request,
            &[42],
            17,
            17,
            1,
            1,
        ));
        assert!(!exact_member_association_values(
            seed,
            request,
            request,
            &[42],
            17,
            17,
            2,
            1,
        ));
        assert!(!exact_member_association_values(
            seed,
            request,
            request,
            &[42],
            17,
            17,
            1,
            2,
        ));
    }

    #[test]
    fn rollover_intent_rejects_untriggered_policy_identity_and_selection_substitution() {
        let engine = Engine::<1>::new(8, 4, 32).unwrap();
        let request = RequestId::new(0, 7);
        let policy = M1SpeculativeGenerationPolicyV1::new(8, &[99]).unwrap();
        let changed_policy = M1SpeculativeGenerationPolicyV1::new(9, &[98]).unwrap();
        assert!(policy.permits_fresh_anchor(42));
        assert!(changed_policy.permits_fresh_anchor(42));
        let prior = serving_plan(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128);
        let next = serving_plan(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
        );
        let epoch = CompletionEpoch::new(7);
        let identity = crate::authenticated_speculative_executor::M1AuthenticatedSpeculativeLineageIdentityV1::fresh().unwrap();
        let members = [M1AuthenticatedSpeculativeRolloverMemberIntentV1::new(
            request, policy,
        )]
        .into();
        let physical = M1AuthenticatedSpeculativeRolloverPhysicalIntentV1 {
            identity,
            prefill_selection: prior.target(),
            speculative_selection: next.target(),
            prefill_epoch: epoch,
            members,
        };
        let logical = M1AuthenticatedSpeculativeRolloverIntentV1 {
            identity,
            prefill_selection: prior.target(),
            speculative_selection: next.target(),
            prefill_epoch: epoch,
            members: physical.members.clone(),
        };
        let exact_seed = crate::M1SpeculativeMemberSeedV1::new(request, 42, 17, 17, policy);
        let association = |logical: &M1AuthenticatedSpeculativeRolloverIntentV1,
                           seeds: &[crate::M1SpeculativeMemberSeedV1]| {
            exact_rollover_intent_association(
                &physical,
                logical,
                prior,
                next,
                prior.target(),
                epoch,
                CompletionEpoch::new(8),
                seeds,
            )
        };
        assert!(association(&logical, &[exact_seed]));
        assert_eq!(
            join_m1_authenticated_prefill_registry_intent_v1(&physical, &logical),
            Some(M1AuthenticatedPrefillRegistryIntentFactsV1 {
                request,
                prefill_selection: prior.target(),
                speculative_selection: next.target(),
                prefill_epoch: epoch,
            })
        );
        assert!(!association(
            &logical,
            &[crate::M1SpeculativeMemberSeedV1::new(
                request,
                42,
                17,
                17,
                changed_policy,
            )],
        ));

        let mut wrong_identity = M1AuthenticatedSpeculativeRolloverIntentV1 {
            identity: crate::authenticated_speculative_executor::M1AuthenticatedSpeculativeLineageIdentityV1::fresh().unwrap(),
            prefill_selection: logical.prefill_selection,
            speculative_selection: logical.speculative_selection,
            prefill_epoch: logical.prefill_epoch,
            members: logical.members.clone(),
        };
        assert!(!association(&wrong_identity, &[exact_seed]));
        assert_eq!(
            join_m1_authenticated_prefill_registry_intent_v1(&physical, &wrong_identity),
            None
        );
        wrong_identity.identity = identity;
        wrong_identity.speculative_selection = serving_plan(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K8C8192,
        )
        .target();
        assert!(!association(&wrong_identity, &[exact_seed]));
        assert_eq!(
            join_m1_authenticated_prefill_registry_intent_v1(&physical, &wrong_identity),
            None
        );
        wrong_identity.speculative_selection = logical.speculative_selection;
        wrong_identity.prefill_epoch = CompletionEpoch::new(epoch.value() + 1);
        assert_eq!(
            join_m1_authenticated_prefill_registry_intent_v1(&physical, &wrong_identity),
            None
        );
        wrong_identity.prefill_epoch = epoch;
        wrong_identity.members = [M1AuthenticatedSpeculativeRolloverMemberIntentV1::new(
            RequestId::new(request.slot(), request.generation() + 1),
            policy,
        )]
        .into();
        assert_eq!(
            join_m1_authenticated_prefill_registry_intent_v1(&physical, &wrong_identity),
            None
        );
        // This association gate runs before queue detachment or Engine dispatch.
        assert!(!engine.is_faulted());
    }

    #[test]
    fn rollover_intent_rejects_epoch_and_s8_roster_reordering() {
        let prior = serving_plan(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS8T128);
        let next = serving_plan(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS8K4C8192,
        );
        let epoch = CompletionEpoch::new(11);
        let identity = crate::authenticated_speculative_executor::M1AuthenticatedSpeculativeLineageIdentityV1::fresh().unwrap();
        let policy = M1SpeculativeGenerationPolicyV1::new(8, &[99]).unwrap();
        let members: Box<[_]> = [
            M1AuthenticatedSpeculativeRolloverMemberIntentV1::new(RequestId::new(0, 1), policy),
            M1AuthenticatedSpeculativeRolloverMemberIntentV1::new(RequestId::new(1, 1), policy),
        ]
        .into();
        let physical = M1AuthenticatedSpeculativeRolloverPhysicalIntentV1 {
            identity,
            prefill_selection: prior.target(),
            speculative_selection: next.target(),
            prefill_epoch: epoch,
            members: members.clone(),
        };
        let mut logical = M1AuthenticatedSpeculativeRolloverIntentV1 {
            identity,
            prefill_selection: prior.target(),
            speculative_selection: next.target(),
            prefill_epoch: epoch,
            members,
        };
        let seeds = [
            crate::M1SpeculativeMemberSeedV1::new(RequestId::new(0, 1), 42, 17, 17, policy),
            crate::M1SpeculativeMemberSeedV1::new(RequestId::new(1, 1), 43, 18, 18, policy),
        ];
        let matches = |logical: &M1AuthenticatedSpeculativeRolloverIntentV1,
                       successor_epoch: CompletionEpoch| {
            exact_rollover_intent_association(
                &physical,
                logical,
                prior,
                next,
                prior.target(),
                epoch,
                successor_epoch,
                &seeds,
            )
        };
        assert!(matches(&logical, CompletionEpoch::new(12)));
        logical.members.swap(0, 1);
        assert!(!matches(&logical, CompletionEpoch::new(12)));
        logical.members.swap(0, 1);
        logical.prefill_epoch = CompletionEpoch::new(10);
        assert!(!matches(&logical, CompletionEpoch::new(12)));
        logical.prefill_epoch = epoch;
        assert!(!matches(&logical, CompletionEpoch::new(13)));
    }

    #[test]
    fn authenticated_rollover_transition_gate_covers_all_four_profiles() {
        let cases = [
            (
                Qwen3PlanBucket::PrefillS1T128,
                Qwen3PlanBucket::SpeculativeS1K4C8192,
                1,
            ),
            (
                Qwen3PlanBucket::PrefillS8T128,
                Qwen3PlanBucket::SpeculativeS8K4C8192,
                8,
            ),
            (
                Qwen3PlanBucket::PrefillS1T128,
                Qwen3PlanBucket::SpeculativeS1K8C8192,
                1,
            ),
            (
                Qwen3PlanBucket::PrefillS1T128,
                Qwen3PlanBucket::SpeculativeS1K16C8192,
                1,
            ),
        ];
        for (prefill_bucket, speculative_bucket, members) in cases {
            let prior = serving_plan(Qwen3ExecutionMode::Prefill, prefill_bucket);
            let next = serving_plan(Qwen3ExecutionMode::Speculative, speculative_bucket);
            let batch = planned_rollover(prior, next, members);
            assert_eq!(
                transition(&batch),
                Ok((prior, next, M1ServingRolloverReasonV1::Mode))
            );
        }
    }

    #[test]
    fn target_decode_workspace_gate_is_exact_and_fail_closed() {
        let draft_prefill = Qwen3PlanSelection {
            role: Qwen3ModelRole::Draft06B,
            mode: Qwen3ExecutionMode::Prefill,
            bucket: Qwen3PlanBucket::PrefillS1T128,
        };
        let target_prefill = Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Prefill,
            bucket: Qwen3PlanBucket::PrefillS1T128,
        };
        let target_decode = Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Decode,
            bucket: Qwen3PlanBucket::DecodeS1C8192,
        };
        assert!(exact_target_decode_workspace_selections(
            draft_prefill,
            target_prefill,
            target_decode,
        ));

        for hostile_draft in [
            target_prefill,
            Qwen3PlanSelection {
                mode: Qwen3ExecutionMode::Decode,
                ..draft_prefill
            },
            Qwen3PlanSelection {
                bucket: Qwen3PlanBucket::PrefillS1T512,
                ..draft_prefill
            },
        ] {
            assert!(!exact_target_decode_workspace_selections(
                hostile_draft,
                target_prefill,
                target_decode,
            ));
        }
        for hostile_target in [
            draft_prefill,
            Qwen3PlanSelection {
                mode: Qwen3ExecutionMode::Decode,
                ..target_prefill
            },
            Qwen3PlanSelection {
                bucket: Qwen3PlanBucket::PrefillS8T128,
                ..target_prefill
            },
        ] {
            assert!(!exact_target_decode_workspace_selections(
                draft_prefill,
                hostile_target,
                target_decode,
            ));
        }
        for hostile_decode in [
            Qwen3PlanSelection {
                role: Qwen3ModelRole::Draft06B,
                ..target_decode
            },
            Qwen3PlanSelection {
                mode: Qwen3ExecutionMode::Prefill,
                ..target_decode
            },
            Qwen3PlanSelection {
                bucket: Qwen3PlanBucket::DecodeS8C8192,
                ..target_decode
            },
        ] {
            assert!(!exact_target_decode_workspace_selections(
                draft_prefill,
                target_prefill,
                hostile_decode,
            ));
        }
    }

    #[test]
    fn target_decode_removal_router_retains_inputs_on_every_lower_outcome() {
        let inputs = [3_u8, 5, 8];
        match retain_target_decode_removal_inputs(
            M1AuthenticatedTargetDecodeRemovalRouteV1::<u16, u32, u64>::Success(13),
            inputs,
        ) {
            M1AuthenticatedTargetDecodeRemovalCustodyV1::Success {
                queue,
                inputs: retained,
            } => {
                assert_eq!(queue, 13);
                assert_eq!(retained, inputs);
            }
            _ => panic!("success route must retain conversion inputs"),
        }
        match retain_target_decode_removal_inputs(
            M1AuthenticatedTargetDecodeRemovalRouteV1::<u16, u32, u64>::Rejected {
                source: 21,
                queue: 34,
            },
            inputs,
        ) {
            M1AuthenticatedTargetDecodeRemovalCustodyV1::Rejected {
                source,
                queue,
                inputs: retained,
            } => {
                assert_eq!(source, 21);
                assert_eq!(queue, 34);
                assert_eq!(retained, inputs);
            }
            _ => panic!("rejection route must retain the unchanged queue and inputs"),
        }
        match retain_target_decode_removal_inputs(
            M1AuthenticatedTargetDecodeRemovalRouteV1::<u16, u32, u64>::Quarantined {
                source: 55,
                retained: 89,
            },
            inputs,
        ) {
            M1AuthenticatedTargetDecodeRemovalCustodyV1::Quarantined {
                source,
                retained,
                inputs: retained_inputs,
            } => {
                assert_eq!(source, 55);
                assert_eq!(retained, 89);
                assert_eq!(retained_inputs, inputs);
            }
            _ => panic!("terminal route must retain quarantine and inputs"),
        }
    }

    #[test]
    fn target_decode_serving_association_rejects_every_authority_splice() {
        let prior = serving_plan(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128);
        let next = serving_plan(Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS1C8192);
        let batch = planned_rollover(prior, next, 1);
        assert_eq!(
            target_decode_serving_transition(&batch),
            Ok((prior, next, M1ServingRolloverReasonV1::Mode))
        );

        let request = batch.requests()[0];
        let exact = expected_target_decode_input_association(next, request, batch.epoch(), 42, 128);
        let matches = |observed| target_decode_input_association_matches(Some(observed), exact);
        assert!(matches(exact));
        let mut hostile = exact;
        let wrong_selection = Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Decode,
            bucket: Qwen3PlanBucket::DecodeS8C8192,
        };

        hostile.selection = wrong_selection;
        assert!(!matches(hostile));
        hostile = exact;
        hostile.live_lanes = 2;
        assert!(!matches(hostile));
        hostile = exact;
        hostile.lane_selection = wrong_selection;
        assert!(!matches(hostile));
        hostile = exact;
        hostile.request = RequestId::new(0, 2);
        assert!(!matches(hostile));
        hostile = exact;
        hostile.epoch = CompletionEpoch::new(batch.epoch().value() + 1);
        assert!(!matches(hostile));
        hostile = exact;
        hostile.anchor = 41;
        assert!(!matches(hostile));
        hostile = exact;
        hostile.position = 127;
        assert!(!matches(hostile));
        hostile = exact;
        hostile.active = 2;
        assert!(!matches(hostile));
        hostile = exact;
        hostile.context = 127;
        assert!(!matches(hostile));
        hostile = exact;
        hostile.preparation_kind = M1FullStepWorkspaceInputKind::PairedPrefill;
        assert!(!matches(hostile));
        hostile = exact;
        hostile.preparation_selection = wrong_selection;
        assert!(!matches(hostile));
        hostile = exact;
        hostile.recipe_kind = M1FullStepWorkspaceInputKind::SpeculativeRound;
        assert!(!matches(hostile));
        hostile = exact;
        hostile.recipe_selection = wrong_selection;
        assert!(!matches(hostile));
    }

    #[test]
    fn terminal_teardown_debug_is_opaque() {
        let failure = M1AuthenticatedSpeculativeRolloverTeardownFailureV1 {
            retained: Box::new("secret detached queue quarantine"),
        };
        assert!(failure.engine_quarantined());
        assert!(failure.retains_ferric_custody());
        assert!(!format!("{failure:?}").contains("secret detached queue quarantine"));
    }
}
