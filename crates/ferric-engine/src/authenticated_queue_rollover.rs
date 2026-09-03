//! Authenticated paired-prefill to finite-speculative queue rollover.
//!
//! The transition consumes the released authenticated prefill owner. It does
//! not accept raw queues, catalog indices, currentness claims, or checked
//! output supplied independently by the normal completion pipeline.

use core::fmt;

use fe2o3_host::{
    AuthenticatedQuarantinedServiceQueueV1, AuthenticatedServiceQueueReleaseFailureV1,
    AuthenticatedServiceQueueReleaseV1, AuthenticatedServiceQueueRetainedRolloverFailureV1,
    AuthenticatedServiceQueueUnboundSessionV1,
};
use ferric_spec::{completion::CompletionEpoch, scheduling::RequestState};

use crate::authenticated_speculative_executor::{
    speculative_validated_pair_matches_binding, upgrade_m1_authenticated_speculative_lineage_v1,
    M1AuthenticatedSpeculativeLogicalLineageWitnessV1,
    M1AuthenticatedSpeculativePhysicalLineageWitnessV1,
};
use crate::m1_serving_registry::admit_m1_production_rollover_transition_v1;
use crate::{
    ActiveDeviceKvCache, AddresslessM1PhysicalBufferRecipeV1, DeviceKvCacheProjection, Engine,
    LogicalRunnerDeclaration, M1AuthenticatedPhysicalQueuePhaseCaseV1,
    M1AuthenticatedPhysicalQueueSessionV1, M1AuthenticatedPhysicalQueueSubmitFailureV1,
    M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1,
    M1AuthenticatedPhysicalReadbackQueueOperationFailureV1, M1AuthenticatedRearmedPublishedQueueV1,
    M1AuthenticatedReleasedCompletedStepV1, M1FiniteSpeculativeQueueRolloverKvInputsV1,
    M1FullStepKvWorkspaceTablesV1, M1FullStepWorkspaceImagesV1, M1FullStepWorkspaceInputKind,
    M1FullStepWorkspacePlans, M1FullStepWorkspaceRole, M1FullStepWorkspaceSubleaseOwners,
    M1InitializedWorkspaceSlotV1, M1PhysicalFixedBatchShapeV1, M1PhysicalQueueBatchCustodyV1,
    M1PhysicalRunnerRecipeOutcomeV1, M1PreparedScheduledWorkspaceImagesV1,
    M1QueueRolloverObservationV1, M1ReleasedDeviceKvMemberV1, M1ScheduledDispatchV1,
    M1ServingBatchPlanV1, M1ServingPlanV1, M1ServingQueueActionV1, M1ServingRolloverReasonV1,
    M1SpeculativeGenerationLoopV1, M1SpeculativeGenerationPolicyV1,
    M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1, M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1,
    M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1,
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
    Lineage,
    Detach,
    ExactDispatch,
    CacheReselection { lane: usize },
}

#[derive(Debug)]
pub(crate) struct M1AuthenticatedSpeculativeRolloverResidueV1 {
    pub(crate) checked: crate::M1CheckedCompletionOutputV1,
    pub(crate) logical_accepted_counts: Box<[u32]>,
    pub(crate) externally_published_counts: Box<[u32]>,
    pub(crate) release_counts: Box<[crate::M1CompletedKvPageReleaseCountsV1]>,
    pub(crate) completed_members: usize,
    pub(crate) total_released: usize,
}

#[derive(Debug)]
pub(crate) struct M1AuthenticatedSpeculativeRolloverLogicalV1 {
    pub(crate) coordinator: M1SpeculativeGenerationLoopV1,
    pub(crate) epoch: CompletionEpoch,
    pub(crate) lineage: M1AuthenticatedSpeculativeLogicalLineageWitnessV1,
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
    inputs: Box<M1FiniteSpeculativeQueueRolloverKvInputsV1>,
    recipe_plans: M1FullStepWorkspacePlans,
    preparation_plans: M1FullStepWorkspacePlans,
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
        schedule_m1_authenticated_speculative_rollover_v1(
            engine,
            *self.released,
            batch,
            *self.intent,
            *self.coordinator,
            *self.inputs,
            self.recipe_plans,
            self.preparation_plans,
        )
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
        } = self;
        let retained = (intent, coordinator, inputs, recipe_plans, preparation_plans);
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
        inputs: Box<M1FiniteSpeculativeQueueRolloverKvInputsV1>,
        recipe_plans: M1FullStepWorkspacePlans,
        preparation_plans: M1FullStepWorkspacePlans,
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
        } => {
            if error == M1AuthenticatedSpeculativeRolloverScheduleErrorV1::EngineFaulted {
                engine.quarantine_m1_queue_rearm_failure();
                let logical = (intent, coordinator, inputs, recipe_plans, preparation_plans);
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

/// Constructs the logical serving transition used only by the ignored native
/// authenticated rollover fixture.
#[cfg(feature = "native-rollover-fixture")]
#[doc(hidden)]
pub fn build_m1_native_rollover_fixture_batch_v1(
    prior: M1ServingPlanV1,
    next: M1ServingPlanV1,
    requests: &[ferric_spec::RequestId],
) -> M1ServingBatchPlanV1 {
    let mut registry = crate::M1ServingRegistryV1::<8>::new()
        .expect("native rollover fixture registry construction must succeed");
    for request in requests {
        registry
            .admit(*request, prior)
            .expect("native rollover fixture admission must succeed");
    }
    let prefill = registry
        .plan_next()
        .expect("native rollover fixture prefill planning must succeed")
        .expect("native rollover fixture prefill must be ready");
    let epoch = prefill.epoch();
    let reservation = registry
        .reserve_publication(prefill)
        .expect("native rollover fixture publication reservation must succeed");
    let identity = reservation.registry_identity();
    registry
        .record_publication(reservation)
        .expect("native rollover fixture publication recording must succeed");
    let dispositions =
        vec![crate::M1ServingCompletionDispositionV1::Continue(next); requests.len()];
    registry
        .preflight_completion_exact_for(identity, epoch, &dispositions)
        .expect("native rollover fixture transition preflight must succeed");
    registry.apply_preflighted_completion(epoch, &dispositions);
    registry
        .plan_next()
        .expect("native rollover fixture rollover planning must succeed")
        .expect("native rollover fixture rollover must be ready")
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
    inputs: &M1FiniteSpeculativeQueueRolloverKvInputsV1,
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
        inputs,
        recipe_plans,
        preparation_plans,
    )
    .map_err(|failure| close_pending_schedule_failure(engine, failure))
}

#[allow(clippy::too_many_arguments)]
fn schedule_m1_authenticated_speculative_rollover_pending_v1<const C: usize>(
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
    };
    let queue = match queue.detach() {
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
    prepare_m1_authenticated_speculative_rollover_pending_v1(engine, scheduled, runner)
        .map_err(|failure| close_pending_preparation_failure(engine, failure))
}

fn prepare_m1_authenticated_speculative_rollover_pending_v1<const C: usize>(
    engine: &mut Engine<C>,
    scheduled: M1AuthenticatedScheduledSpeculativeRolloverV1,
    runner: &LogicalRunnerDeclaration,
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
            Self::NativeK4(_) | Self::NativeK8(_) | Self::NativeK16(_) => {
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
) -> Result<
    crate::M1AuthenticatedSpeculativeRolloverPublishedV1,
    M1AuthenticatedSpeculativeRolloverSubmissionFailureV1,
> {
    submit_m1_authenticated_speculative_rollover_pending_v1(engine, prepared, ring_bytes)
        .map_err(|failure| close_pending_submission_failure(engine, failure))
}

fn submit_m1_authenticated_speculative_rollover_pending_v1<const C: usize>(
    engine: &mut Engine<C>,
    prepared: M1AuthenticatedPreparedSpeculativeRolloverV1,
    ring_bytes: u32,
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
    } = residue;
    let previous_epoch = checked.epoch();
    let published = M1AuthenticatedRearmedPublishedQueueV1::from_authenticated_rollover(
        queue,
        selected,
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
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        M1ServingCompletionDispositionV1, M1ServingRegistryV1, M1SpeculativeGenerationPolicyV1,
    };
    use ferric_spec::{
        Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection, RequestId,
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
        wrong_identity.identity = identity;
        wrong_identity.speculative_selection = serving_plan(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K8C8192,
        )
        .target();
        assert!(!association(&wrong_identity, &[exact_seed]));
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
    fn terminal_teardown_debug_is_opaque() {
        let failure = M1AuthenticatedSpeculativeRolloverTeardownFailureV1 {
            retained: Box::new("secret detached queue quarantine"),
        };
        assert!(failure.engine_quarantined());
        assert!(failure.retains_ferric_custody());
        assert!(!format!("{failure:?}").contains("secret detached queue quarantine"));
    }
}
