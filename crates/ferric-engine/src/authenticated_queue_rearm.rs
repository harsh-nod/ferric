//! Authenticated same-native-queue rebind after semantic completion.
//!
//! This is the effectful core used by the released-step rearm lifecycle. It
//! accepts only a detached post-readback authenticated queue, exact fresh
//! workspace images, and an unchanged-structure physical recipe. Program
//! indices remain private to the retained authenticated witness.

#![expect(
    dead_code,
    reason = "the staged private rebind core is consumed by the authenticated reserve/prepare/submit bridge"
)]

use core::fmt;

use arrayvec::ArrayVec;
use fe2o3_host::{
    AuthenticatedServiceQueueDataUpdateFailureV1, AuthenticatedServiceQueueReleaseV1,
    AuthenticatedServiceQueueRetainedBindFailureV1, AuthenticatedServiceQueueSessionV1,
    AuthenticatedServiceQueueUnboundSessionV1,
};
use fe2o3_kfd::{
    ComputeAqlQueueObservationV1, Gfx942DeviceContentDescriptorV1,
    Gfx942TimeoutExecutionObservationV1,
};
use fe2o3_service_host::{DeviceWorkspaceRoleV1, HostDownloadRoleV1, ServiceDeviceDispatchRangeV1};
use ferric_build::AddresslessM1StepWorkspacePlan;
use ferric_spec::{
    completion::CompletionEpoch, scheduling::RequestState, Qwen3ExecutionMode, Qwen3ModelRole,
    Qwen3PlanSelection, RequestId, StepPlan, ValidatedM1StepInputs, M1_KV_PAGE_TOKENS,
    M1_MAX_ACTIVE_SEQUENCES,
};

use crate::m1_queue_rearm::{
    append_workspace_ranges, member_layout, preflight_all_terminal_rearm_shutdown,
    rebuild_bound_rows, rebuild_bound_rows_with_storage, retained_host_capture_ranges,
    M1NonEmptyRearmRoundHistoryV1, M1RearmRoundHistoryV1,
};
use crate::physical_fixed_batch::{
    build_m1_authenticated_queue_packet_batch_v1,
    build_m1_authenticated_rollover_packet_batch_with_storage_v1,
    validate_authenticated_operation_plan_v1, M1AuthenticatedQueuePacketBatchCaseV1,
    M1AuthenticatedQueuePacketBatchV1,
};
use crate::step_workspace_subleases::{
    bind_authenticated_queue_replaced_m1_step_workspace,
    M1AuthenticatedQueueReplacedWorkspaceBindingFailureV1,
};
use crate::{
    ActiveDeviceKvCache, AddresslessM1PhysicalBufferRecipeV1, BoundM1StepWorkspaceSubleases,
    DeclaredOperationKernelPlan, Engine, EngineError, ExactCompletion, Gfx942DeviceBinding,
    LogicalRunnerError, M1AuthenticatedCompletedReadbackJoinFailureV1,
    M1AuthenticatedCompletedStepOutcomeV1, M1AuthenticatedCompletionObservationFailureV1,
    M1AuthenticatedObservedCompletionOutputV1, M1AuthenticatedPhysicalCompletedQueueSessionV1,
    M1AuthenticatedPhysicalCompletedReadbackV1, M1AuthenticatedPhysicalPublishedQueueSessionV1,
    M1AuthenticatedPhysicalQueueOperationFailureV1, M1AuthenticatedPhysicalQueuePhaseOwnerV1,
    M1AuthenticatedPhysicalQueuePhaseSlotV1, M1AuthenticatedPhysicalQueueSessionV1,
    M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1,
    M1AuthenticatedPhysicalReadbackQueueSessionV1, M1AuthenticatedPhysicalRecycledQueueSessionV1,
    M1AuthenticatedReleasedCompletedStepV1, M1CheckedCompletionOutputV1,
    M1CompletedKvPageReleaseCountsV1, M1DeviceKvCompletionDispositionV1,
    M1DeviceKvCompletionMemberV1, M1DeviceKvCompletionRosterV1, M1ExactDispatchErrorV1,
    M1FullStepKvReservationCustodyV1, M1FullStepKvWorkspaceTablesV1, M1FullStepWorkspaceImagesV1,
    M1FullStepWorkspaceInputKind, M1FullStepWorkspacePlans, M1FullStepWorkspaceRole,
    M1FullStepWorkspaceSubleaseOwners, M1InitializedWorkspaceSlotV1,
    M1LongLivedQueueRearmKvInputsV1, M1LongLivedQueueRearmKvReservationPhaseV1,
    M1LongLivedQueueRearmProgressPhaseV1, M1LongLivedQueueRearmScheduleErrorV1,
    M1LongLivedQueueRearmSchedulePhaseV1, M1ObservedCompletionImageV1,
    M1PhysicalFixedBatchBuildErrorV1, M1PhysicalFixedBatchShapeV1, M1PhysicalQueueBatchCustodyV1,
    M1PhysicalRunnerRecipeOutcomeV1, M1PrepareFailureV1, M1PreparedScheduledWorkspaceImagesV1,
    M1PrepublicationStepCustodyV1, M1ReleasedDeviceKvMemberV1, M1ReleasedTerminalDeviceKvMemberV1,
    M1ScheduledDispatchV1, M1StepDispatchIntent, M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1,
    M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1, M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1,
    M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1, M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1,
};

pub(crate) type M1AuthenticatedRearmedDispositionsV1 =
    ArrayVec<M1DeviceKvCompletionDispositionV1, { M1_MAX_ACTIVE_SEQUENCES as usize }>;

#[derive(Debug)]
pub(crate) struct M1AuthenticatedResidentCompletionScratchV1 {
    dispositions: Vec<M1DeviceKvCompletionDispositionV1>,
    members: Vec<M1DeviceKvCompletionMemberV1>,
    completion: crate::m1_completed_step::M1CompletedStepScratchV1,
    release: crate::m1_completed_step_release::M1CompletedStepKvReleaseScratchV1,
    selected_slots: Vec<Option<ActiveDeviceKvCache>>,
    selected: Vec<ActiveDeviceKvCache>,
    parked: Vec<ActiveDeviceKvCache>,
    terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
    draft_reservations: Vec<crate::PendingSpeculativeDraftKvRoundWrite>,
    target_reservations: Vec<crate::PendingDeviceKvStepWrite>,
    draft_page_leases: Vec<Vec<crate::DeviceKvPageLease>>,
    target_page_leases: Vec<Vec<crate::DeviceKvPageLease>>,
    draft_table: Option<crate::kv_workspace_authority::M1KvWorkspaceTableHostStorageV1>,
    target_table: Option<crate::kv_workspace_authority::M1KvWorkspaceTableHostStorageV1>,
    workspace_images: Option<crate::m1_prepublication::M1FullStepWorkspaceImageHostStorageV1>,
    publication: Option<M1AuthenticatedRearmPublicationHostStorageV1>,
    readback: Option<
        crate::authenticated_physical_readback::M1AuthenticatedResidentReadbackHostStorageV1,
    >,
}

#[derive(Debug)]
pub(crate) struct M1AuthenticatedRearmPublicationHostStorageV1 {
    selection: Qwen3PlanSelection,
    shape: M1PhysicalFixedBatchShapeV1,
    workspace_ranges: Vec<crate::m1_queue_rearm::FreshWorkspaceRangeV1>,
    bound_rows: Option<crate::m1_queue_rearm::M1RolloverBoundRowsHostStorageV1>,
    packet_batch: crate::physical_fixed_batch::M1AuthenticatedRolloverPacketBatchHostStorageV1,
    phase: Option<
        Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1>>,
    >,
    phase_k8: Option<
        Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1>>,
    >,
    phase_k16: Option<
        Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1>>,
    >,
    phase_catchup: Option<
        Box<
            M1AuthenticatedPhysicalQueuePhaseSlotV1<
                { crate::M1_DRAFT_CATCHUP_FIXED_BATCH_PACKETS_V1 },
            >,
        >,
    >,
    draft_owner: Option<
        Box<
            core::mem::MaybeUninit<
                BoundM1StepWorkspaceSubleases<{ crate::M1_DRAFT_STEP_WORKSPACE_SUBLEASE_COUNT_V1 }>,
            >,
        >,
    >,
    completion_owner: Option<
        Box<
            core::mem::MaybeUninit<
                BoundM1StepWorkspaceSubleases<
                    { crate::M1_TARGET_STEP_WORKSPACE_SUBLEASE_COUNT_V1 },
                >,
            >,
        >,
    >,
    speculative_owner: Option<
        Box<
            core::mem::MaybeUninit<
                BoundM1StepWorkspaceSubleases<
                    { crate::M1_TARGET_SPECULATIVE_STEP_WORKSPACE_SUBLEASE_COUNT_V1 },
                >,
            >,
        >,
    >,
}

impl M1AuthenticatedRearmPublicationHostStorageV1 {
    fn try_new(selection: Qwen3PlanSelection) -> Option<Self> {
        Self::try_new_inner(selection, false)
    }

    fn try_new_inner(selection: Qwen3PlanSelection, catchup: bool) -> Option<Self> {
        let plan =
            crate::authenticated_prefill_bootstrap::admitted_s1_t128_speculative_successor_v1(
                selection,
            )?;
        let speculative_shape = plan.shape();
        let shape = if catchup {
            M1PhysicalFixedBatchShapeV1::DraftCatchup
        } else {
            speculative_shape
        };
        let mut workspace_ranges = Vec::new();
        workspace_ranges
            .try_reserve_exact(
                crate::M1_DRAFT_STEP_WORKSPACE_SUBLEASE_COUNT_V1
                    + crate::M1_TARGET_SPECULATIVE_STEP_WORKSPACE_SUBLEASE_COUNT_V1,
            )
            .ok()?;
        Some(Self {
            selection,
            shape,
            workspace_ranges,
            bound_rows: Some(
                crate::m1_queue_rearm::M1RolloverBoundRowsHostStorageV1::try_new(
                    shape.packet_count(),
                    16,
                )?,
            ),
            packet_batch: if speculative_shape == M1PhysicalFixedBatchShapeV1::SpeculativeK4 {
                crate::physical_fixed_batch::M1AuthenticatedRolloverPacketBatchHostStorageV1::new()
            } else {
                crate::physical_fixed_batch::M1AuthenticatedRolloverPacketBatchHostStorageV1::new_for_speculative_shape(speculative_shape)?
            },
            phase: (shape == M1PhysicalFixedBatchShapeV1::SpeculativeK4)
                .then(|| Box::new(M1AuthenticatedPhysicalQueuePhaseSlotV1::empty())),
            phase_k8: (shape == M1PhysicalFixedBatchShapeV1::SpeculativeK8)
                .then(|| Box::new(M1AuthenticatedPhysicalQueuePhaseSlotV1::empty())),
            phase_k16: (shape == M1PhysicalFixedBatchShapeV1::SpeculativeK16)
                .then(|| Box::new(M1AuthenticatedPhysicalQueuePhaseSlotV1::empty())),
            phase_catchup: catchup
                .then(|| Box::new(M1AuthenticatedPhysicalQueuePhaseSlotV1::empty())),
            draft_owner: Some(Box::new_uninit()),
            completion_owner: catchup.then(Box::new_uninit),
            speculative_owner: (!catchup).then(Box::new_uninit),
        })
    }

    fn prepare_recipe(&mut self, recipe: &AddresslessM1PhysicalBufferRecipeV1) -> bool {
        let intent = if self.shape == M1PhysicalFixedBatchShapeV1::DraftCatchup {
            M1StepDispatchIntent::DraftCatchup(self.selection)
        } else {
            M1StepDispatchIntent::SpeculativeRound(self.selection)
        };
        recipe.workspace_composition().dispatch_plan().intent() == intent
            && recipe.rows().len() == self.shape.packet_count()
            && self.packet_batch.prepare_recipe(recipe)
    }
}

#[derive(Debug)]
pub(crate) struct M1AuthenticatedDraftCatchupScratchV1 {
    parent: Qwen3PlanSelection,
    scheduling: M1AuthenticatedResidentCompletionScratchV1,
    draft_inputs:
        Option<crate::authenticated_resident_session::M1AuthenticatedResidentRoleInputStorageV1>,
    completion_inputs:
        Option<crate::authenticated_resident_session::M1AuthenticatedResidentRoleInputStorageV1>,
    page_admission: Option<crate::device_cache::M1DraftCatchupPageAdmissionHostStorageV1>,
    write: crate::device_cache::M1DraftCatchupKvWriteHostStorageV1,
    page_leases: Vec<crate::DeviceKvPageLease>,
    reservations: Vec<crate::PendingDeviceKvStepWrite>,
    draft_table: Option<crate::kv_workspace_authority::M1KvWorkspaceTableHostStorageV1>,
    completion_page_indices: Option<Box<[u32]>>,
    workspace_images: Option<crate::m1_prepublication::M1FullStepWorkspaceImageHostStorageV1>,
    publication: Option<M1AuthenticatedRearmPublicationHostStorageV1>,
    readback: Option<
        crate::authenticated_physical_readback::M1AuthenticatedDraftCatchupReadbackHostStorageV1,
    >,
}

impl M1AuthenticatedDraftCatchupScratchV1 {
    pub(crate) fn try_new(
        parent: Qwen3PlanSelection,
        plans: &M1FullStepWorkspacePlans,
    ) -> Option<Self> {
        crate::authenticated_prefill_bootstrap::admitted_s1_t128_speculative_successor_v1(parent)?;
        let M1FullStepWorkspacePlans::DraftCatchup {
            parent: declared_parent,
            draft_decode,
            completion,
        } = plans
        else {
            return None;
        };
        let decode = M1StepDispatchIntent::DraftCatchup(parent).completion_selection();
        if *declared_parent != parent
            || completion.selection() != decode
            || draft_decode.selection()
                != (Qwen3PlanSelection {
                    role: Qwen3ModelRole::Draft06B,
                    ..decode
                })
            || draft_decode.allocation().allocation_id() == completion.allocation().allocation_id()
        {
            return None;
        }
        let mut page_leases = Vec::new();
        page_leases.try_reserve_exact(1).ok()?;
        let mut reservations = Vec::new();
        reservations.try_reserve_exact(1).ok()?;
        let mut completion_page_indices = Vec::new();
        completion_page_indices.try_reserve_exact(512).ok()?;
        completion_page_indices.resize(512, 0);
        Some(Self {
            parent,
            scheduling: M1AuthenticatedResidentCompletionScratchV1::try_new(1)?,
            draft_inputs: Some(crate::authenticated_resident_session::M1AuthenticatedResidentRoleInputStorageV1::try_new(1)?),
            completion_inputs: Some(crate::authenticated_resident_session::M1AuthenticatedResidentRoleInputStorageV1::try_new(1)?),
            page_admission: Some(crate::device_cache::M1DraftCatchupPageAdmissionHostStorageV1::try_new()?),
            write: crate::device_cache::M1DraftCatchupKvWriteHostStorageV1::try_new()?,
            page_leases,
            reservations,
            draft_table: Some(crate::kv_workspace_authority::M1KvWorkspaceTableHostStorageV1::try_new(1)?),
            completion_page_indices: Some(completion_page_indices.into_boxed_slice()),
            workspace_images: Some(crate::m1_prepublication::M1FullStepWorkspaceImageHostStorageV1::try_new(plans)?),
            publication: Some(M1AuthenticatedRearmPublicationHostStorageV1::try_new_inner(parent, true)?),
            readback: Some(crate::authenticated_physical_readback::M1AuthenticatedDraftCatchupReadbackHostStorageV1::try_new(parent)?),
        })
    }

    pub(crate) fn prepare_recipe(&mut self, recipe: &AddresslessM1PhysicalBufferRecipeV1) -> bool {
        recipe.workspace_composition().dispatch_plan().intent()
            == M1StepDispatchIntent::DraftCatchup(self.parent)
            && self
                .publication
                .as_mut()
                .is_some_and(|storage| storage.prepare_recipe(recipe))
    }
}

impl M1AuthenticatedResidentCompletionScratchV1 {
    pub(crate) fn try_new(member_capacity: usize) -> Option<Self> {
        Self::try_new_inner(member_capacity, None)
    }

    pub(crate) fn try_new_for_plans(
        member_capacity: usize,
        plans: &M1FullStepWorkspacePlans,
    ) -> Option<Self> {
        Self::try_new_inner(member_capacity, Some(plans))
    }

    fn try_new_inner(
        member_capacity: usize,
        plans: Option<&M1FullStepWorkspacePlans>,
    ) -> Option<Self> {
        let target_page_capacity = match plans {
            Some(plans) => {
                let plan = crate::authenticated_prefill_bootstrap::admitted_s1_t128_speculative_successor_v1(plans.target().selection())?;
                if plans.draft().map(AddresslessM1StepWorkspacePlan::selection)
                    != Some(plan.draft())
                {
                    return None;
                }
                let shape =
                    crate::M1SpeculativePhysicalShapeV1::from_selection(plan.target()).ok()?;
                (usize::from(shape.draft_tokens()) + 1).div_ceil(M1_KV_PAGE_TOKENS as usize)
            }
            None => 1,
        };
        let mut members = Vec::new();
        let mut dispositions = Vec::new();
        let mut selected_slots = Vec::new();
        let mut selected = Vec::new();
        let mut parked = Vec::new();
        let mut terminal = Vec::new();
        let mut draft_reservations = Vec::new();
        let mut target_reservations = Vec::new();
        let mut draft_page_leases = Vec::new();
        let mut target_page_leases = Vec::new();
        dispositions.try_reserve_exact(member_capacity).ok()?;
        members.try_reserve_exact(member_capacity).ok()?;
        selected_slots.try_reserve_exact(member_capacity).ok()?;
        selected.try_reserve_exact(member_capacity).ok()?;
        parked.try_reserve_exact(member_capacity).ok()?;
        terminal.try_reserve_exact(member_capacity).ok()?;
        draft_reservations.try_reserve_exact(member_capacity).ok()?;
        target_reservations
            .try_reserve_exact(member_capacity)
            .ok()?;
        draft_page_leases.try_reserve_exact(member_capacity).ok()?;
        target_page_leases.try_reserve_exact(member_capacity).ok()?;
        for _ in 0..member_capacity {
            let mut draft = Vec::new();
            let mut target = Vec::new();
            draft.try_reserve_exact(1).ok()?;
            target.try_reserve_exact(target_page_capacity).ok()?;
            draft_page_leases.push(draft);
            target_page_leases.push(target);
        }
        let draft_table = match plans {
            Some(_) => Some(
                crate::kv_workspace_authority::M1KvWorkspaceTableHostStorageV1::try_new(
                    member_capacity,
                )?,
            ),
            None => None,
        };
        let target_table = match plans {
            Some(_) => Some(
                crate::kv_workspace_authority::M1KvWorkspaceTableHostStorageV1::try_new(
                    member_capacity,
                )?,
            ),
            None => None,
        };
        Some(Self {
            dispositions,
            members,
            completion: crate::m1_completed_step::M1CompletedStepScratchV1::try_new(
                member_capacity,
            )?,
            release: crate::m1_completed_step_release::M1CompletedStepKvReleaseScratchV1::try_new(
                member_capacity,
            )?,
            selected_slots,
            selected,
            parked,
            terminal,
            draft_reservations,
            target_reservations,
            draft_page_leases,
            target_page_leases,
            draft_table,
            target_table,
            workspace_images: match plans {
                Some(plans) => Some(
                    crate::m1_prepublication::M1FullStepWorkspaceImageHostStorageV1::try_new(
                        plans,
                    )?,
                ),
                None => None,
            },
            publication: match plans {
                Some(plans) => Some(M1AuthenticatedRearmPublicationHostStorageV1::try_new(plans.target().selection())?),
                None => None,
            },
            readback: match plans {
                Some(plans) => Some(
                    crate::authenticated_physical_readback::M1AuthenticatedResidentReadbackHostStorageV1::try_new(
                        plans.target().selection(),
                    )?,
                ),
                None => None,
            },
        })
    }

    pub(crate) fn prepare_recipe(&mut self, recipe: &AddresslessM1PhysicalBufferRecipeV1) -> bool {
        self.publication
            .as_mut()
            .is_some_and(|storage| storage.prepare_recipe(recipe))
    }

    pub(crate) fn take_publication(
        &mut self,
    ) -> Option<M1AuthenticatedRearmPublicationHostStorageV1> {
        self.publication.take()
    }

    pub(crate) fn take_readback(
        &mut self,
    ) -> Option<crate::authenticated_physical_readback::M1AuthenticatedResidentReadbackHostStorageV1>
    {
        self.readback.take()
    }
}

/// Authenticated scheduling rejection before or after physical queue detachment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedLongLivedQueueRearmScheduleErrorV1 {
    /// The Engine was already permanently faulted before any input was consumed.
    EngineFaulted,
    /// A paired-prefill new-window bridge must use its dedicated successor join.
    PendingNewWindowBridge,
    /// Two available active cache owners claim the same request slot.
    DuplicateAvailableRequest { first_member: usize, member: usize },
    /// Existing shared same-shape scheduling diagnostic.
    Shared(M1LongLivedQueueRearmScheduleErrorV1),
}

/// Authenticated released round with parked, terminal, and history lineage.
#[must_use = "released authenticated round must schedule again or be torn down"]
#[derive(Debug)]
pub struct M1AuthenticatedLongLivedQueueReleasedRoundV1 {
    released: M1AuthenticatedReleasedCompletedStepV1,
    parked: Vec<ActiveDeviceKvCache>,
    terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
    history: M1RearmRoundHistoryV1,
    new_window_bridge:
        Option<crate::authenticated_queue_rollover::M1AuthenticatedSpeculativeNewWindowBridgeV1>,
}

/// Crate-private custody extracted only for the authenticated all-terminal
/// new-window transition. Every field remains linear through detachment.
#[derive(Debug)]
pub(crate) struct M1AuthenticatedNewWindowReleasedCustodyV1 {
    pub(crate) queue: M1AuthenticatedPhysicalReadbackQueueSessionV1,
    pub(crate) checked: M1CheckedCompletionOutputV1,
    pub(crate) members: Vec<M1ReleasedDeviceKvMemberV1>,
    pub(crate) terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
    pub(crate) logical_accepted_counts: Box<[u32]>,
    pub(crate) externally_published_counts: Box<[u32]>,
    pub(crate) release_counts: Box<[M1CompletedKvPageReleaseCountsV1]>,
    pub(crate) completed_members: usize,
    pub(crate) total_released: usize,
    pub(crate) history: M1RearmRoundHistoryV1,
    pub(crate) new_window_bridge:
        Option<crate::authenticated_queue_rollover::M1AuthenticatedSpeculativeNewWindowBridgeV1>,
}

/// Confirmed healthy shutdown of an all-terminal authenticated queue.
#[must_use = "authenticated queue release and complete round lineage remain retained"]
#[derive(Debug)]
pub struct M1AuthenticatedLongLivedQueueAllTerminalShutdownSuccessV1 {
    released: crate::M1AuthenticatedReleasedAllTerminalQueueShutdownSuccessV1,
    terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
    history: M1RearmRoundHistoryV1,
    new_window_bridge:
        Option<crate::authenticated_queue_rollover::M1AuthenticatedSpeculativeNewWindowBridgeV1>,
}

impl M1AuthenticatedLongLivedQueueAllTerminalShutdownSuccessV1 {
    #[must_use = "authenticated destruction and current-step custody remain retained"]
    pub const fn released(
        &self,
    ) -> &crate::M1AuthenticatedReleasedAllTerminalQueueShutdownSuccessV1 {
        &self.released
    }

    #[must_use]
    pub const fn parked_count(&self) -> usize {
        0
    }

    #[must_use]
    pub const fn terminal_lineage_count(&self) -> usize {
        self.terminal.len()
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&crate::M1RearmRoundHistoryEntryV1> {
        self.history.get(index)
    }
}

/// Retry-safe authenticated all-terminal shutdown rejection.
#[must_use = "the unchanged authenticated released round remains retry-capable"]
#[derive(Debug)]
pub struct M1AuthenticatedLongLivedQueueAllTerminalShutdownRejectionV1 {
    error: crate::M1AllTerminalQueueShutdownErrorV1,
    released: Box<M1AuthenticatedLongLivedQueueReleasedRoundV1>,
}

impl M1AuthenticatedLongLivedQueueAllTerminalShutdownRejectionV1 {
    #[must_use]
    pub const fn error(&self) -> crate::M1AllTerminalQueueShutdownErrorV1 {
        self.error
    }

    #[must_use = "the complete unchanged authenticated round remains retained"]
    pub const fn released(&self) -> &M1AuthenticatedLongLivedQueueReleasedRoundV1 {
        &self.released
    }

    /// Returns the rejection and unchanged authenticated round exactly once.
    #[must_use = "the rejection and authenticated round remain retained"]
    pub fn into_parts(
        self,
    ) -> (
        crate::M1AllTerminalQueueShutdownErrorV1,
        M1AuthenticatedLongLivedQueueReleasedRoundV1,
    ) {
        (self.error, *self.released)
    }
}

/// Exhaustive authenticated all-terminal rejection or terminal quarantine.
#[must_use = "authenticated all-terminal shutdown failure retains complete custody"]
#[derive(Debug)]
pub enum M1AuthenticatedLongLivedQueueAllTerminalShutdownFailureV1 {
    /// Pure preflight rejection retaining the unchanged round.
    Rejected(Box<M1AuthenticatedLongLivedQueueAllTerminalShutdownRejectionV1>),
    /// Native release failed and the Engine was permanently quarantined.
    Quarantined(Box<M1AuthenticatedLongLivedQueueRearmTeardownFailureV1>),
}

impl M1AuthenticatedLongLivedQueueReleasedRoundV1 {
    pub(crate) const fn initial(released: M1AuthenticatedReleasedCompletedStepV1) -> Self {
        Self {
            released,
            parked: Vec::new(),
            terminal: Vec::new(),
            history: M1RearmRoundHistoryV1::Empty,
            new_window_bridge: None,
        }
    }

    pub(crate) fn try_reserve_round_history(
        &mut self,
        additional: usize,
    ) -> Result<(), crate::M1RearmedCompletionPreflightErrorV1> {
        self.history.try_reserve_additional(additional)
    }

    pub(crate) fn has_reserved_round_history_capacity(&self, additional: usize) -> bool {
        self.history.has_reserved_append_capacity(additional)
    }

    /// Current released authenticated step retained by this round.
    #[must_use = "current released-step custody remains linear"]
    pub const fn current_released(&self) -> &M1AuthenticatedReleasedCompletedStepV1 {
        &self.released
    }

    pub(crate) const fn retained_logical_runner(&self) -> &crate::LogicalRunnerDeclaration {
        self.released.queue().logical_runner()
    }

    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.parked.len()
    }

    #[must_use]
    pub const fn terminal_lineage_count(&self) -> usize {
        self.terminal.len()
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&crate::M1RearmRoundHistoryEntryV1> {
        self.history.get(index)
    }

    pub(crate) fn speculative_lineage_witness(
        &self,
    ) -> Result<
        &crate::authenticated_speculative_executor::M1AuthenticatedSpeculativePhysicalLineageWitnessV1,
        (),
    >{
        let mut found = self.released.checked().speculative_lineage();
        for index in 0..self.history.len() {
            let witness = self
                .history
                .get(index)
                .and_then(|entry| entry.checked().speculative_lineage());
            if witness.is_some() {
                if found.is_some() {
                    return Err(());
                }
                found = witness;
            }
        }
        found.ok_or(())
    }

    pub(crate) fn speculative_history_count(&self, selection: Qwen3PlanSelection) -> usize {
        (0..self.history.len())
            .filter(|index| {
                self.history
                    .get(*index)
                    .is_some_and(|entry| entry.checked().selection() == selection)
            })
            .count()
    }

    pub(crate) fn terminal_lineage_requests(
        &self,
    ) -> impl ExactSizeIterator<Item = RequestId> + Clone + '_ {
        self.terminal.iter().map(|member| member.request())
    }

    pub(crate) const fn has_pending_new_window_bridge(&self) -> bool {
        self.new_window_bridge.is_some()
    }

    pub(crate) fn pending_new_window_speculative_successor(
        &self,
    ) -> Option<crate::M1ServingPlanV1> {
        self.parked
            .is_empty()
            .then(|| {
                self.new_window_bridge
                    .as_ref()
                    .map(|bridge| bridge.speculative_successor)
            })
            .flatten()
    }

    pub(crate) fn into_authenticated_new_window_successor_custody(
        self,
    ) -> Result<
        (
            M1AuthenticatedReleasedCompletedStepV1,
            Vec<M1ReleasedTerminalDeviceKvMemberV1>,
            M1RearmRoundHistoryV1,
            crate::authenticated_queue_rollover::M1AuthenticatedSpeculativeNewWindowBridgeV1,
        ),
        Box<Self>,
    > {
        if !self.parked.is_empty() || self.new_window_bridge.is_none() {
            return Err(Box::new(self));
        }
        let Self {
            released,
            parked: _,
            terminal,
            history,
            new_window_bridge,
        } = self;
        Ok((
            released,
            terminal,
            history,
            new_window_bridge.expect("bridge presence was checked"),
        ))
    }

    pub(crate) fn try_reserve_new_window_terminal_lineage(
        &mut self,
    ) -> Result<(), std::collections::TryReserveError> {
        self.terminal
            .try_reserve_exact(self.released.members().len())
    }

    pub(crate) fn has_new_window_terminal_lineage_capacity(&self) -> bool {
        self.terminal.capacity().saturating_sub(self.terminal.len())
            >= self.released.members().len()
    }

    pub(crate) fn try_reserve_resident_window_terminal_lineage(
        &mut self,
        additional: usize,
    ) -> Result<(), std::collections::TryReserveError> {
        self.terminal.try_reserve_exact(additional)
    }

    pub(crate) fn into_authenticated_new_window_custody(
        self,
    ) -> M1AuthenticatedNewWindowReleasedCustodyV1 {
        let Self {
            released,
            parked,
            terminal,
            history,
            new_window_bridge,
        } = self;
        debug_assert!(parked.is_empty());
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
        M1AuthenticatedNewWindowReleasedCustodyV1 {
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
        }
    }

    /// Shuts down an all-terminal authenticated queue without faulting its Engine.
    ///
    /// Parked ownership is rejected before any queue or released-step owner is
    /// consumed. The lower transition then requires every current member and
    /// the Engine itself to be quiescent. Only confirmed authenticated native
    /// destruction returns success with a still-healthy Engine.
    ///
    /// # Errors
    ///
    /// Returns the unchanged round on preflight rejection, or terminal
    /// authenticated release quarantine joined to every round owner.
    pub fn shutdown_all_terminal_queue<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1AuthenticatedLongLivedQueueAllTerminalShutdownSuccessV1,
        M1AuthenticatedLongLivedQueueAllTerminalShutdownFailureV1,
    > {
        if let Err(error) = preflight_all_terminal_rearm_shutdown(self.parked.len()) {
            return Err(
                M1AuthenticatedLongLivedQueueAllTerminalShutdownFailureV1::Rejected(Box::new(
                    M1AuthenticatedLongLivedQueueAllTerminalShutdownRejectionV1 {
                        error,
                        released: Box::new(self),
                    },
                )),
            );
        }
        let Self {
            released,
            parked,
            terminal,
            history,
            new_window_bridge,
        } = self;
        match released.shutdown_all_terminal_queue(engine) {
            Ok(released) => Ok(M1AuthenticatedLongLivedQueueAllTerminalShutdownSuccessV1 {
                released,
                terminal,
                history,
                new_window_bridge,
            }),
            Err(crate::M1AuthenticatedReleasedAllTerminalQueueShutdownFailureV1::Rejected(
                rejection,
            )) => {
                let (error, released) = rejection.into_parts();
                Err(
                    M1AuthenticatedLongLivedQueueAllTerminalShutdownFailureV1::Rejected(Box::new(
                        M1AuthenticatedLongLivedQueueAllTerminalShutdownRejectionV1 {
                            error,
                            released: Box::new(Self {
                                released,
                                parked,
                                terminal,
                                history,
                                new_window_bridge,
                            }),
                        },
                    )),
                )
            }
            Err(crate::M1AuthenticatedReleasedAllTerminalQueueShutdownFailureV1::Quarantined(
                released,
            )) => Err(
                M1AuthenticatedLongLivedQueueAllTerminalShutdownFailureV1::Quarantined(Box::new(
                    M1AuthenticatedLongLivedQueueRearmTeardownFailureV1 {
                        released,
                        parked,
                        terminal,
                        history,
                        new_window_bridge,
                    },
                )),
            ),
        }
    }

    /// Destroys the authenticated queue while retaining current and prior lineage.
    ///
    /// # Errors
    ///
    /// Returns terminal authenticated queue-release quarantine together with all
    /// current, parked, terminal, and prior-round custody.
    pub fn destroy_queue_and_retain_round<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1AuthenticatedLongLivedQueueRearmTeardownSuccessV1,
        Box<M1AuthenticatedLongLivedQueueRearmTeardownFailureV1>,
    > {
        let Self {
            released,
            parked,
            terminal,
            history,
            new_window_bridge,
        } = self;
        match released.destroy_queue_and_retain_step(engine) {
            Ok(released) => Ok(M1AuthenticatedLongLivedQueueRearmTeardownSuccessV1 {
                released,
                parked,
                terminal,
                history,
                new_window_bridge,
            }),
            Err(released) => Err(Box::new(
                M1AuthenticatedLongLivedQueueRearmTeardownFailureV1 {
                    released,
                    parked,
                    terminal,
                    history,
                    new_window_bridge,
                },
            )),
        }
    }

    /// Schedules the next automatic batch while preserving all prior lineage.
    ///
    /// # Errors
    ///
    /// Pure preflight failures retain this exact round for retry. Invariant,
    /// detach, scheduler, and later failures fault the Engine and retain custody.
    pub fn schedule_next<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1AuthenticatedScheduledLongLivedQueueRearmV1,
        M1AuthenticatedLongLivedQueueRearmScheduleFailureV1,
    > {
        schedule_m1_authenticated_long_lived_queue_rearm_inner_v1(
            engine,
            self,
            M1AuthenticatedRearmDispatchV1::Automatic,
            None,
        )
    }

    /// Schedules exactly the named Ready subset at the exact next epoch.
    ///
    /// # Errors
    ///
    /// Pure roster and epoch failures retain this exact round for retry.
    /// Invariant, detach, scheduler, and later failures fault the Engine.
    pub fn schedule_next_exact<const C: usize>(
        self,
        engine: &mut Engine<C>,
        expected_epoch: CompletionEpoch,
        requests: &[RequestId],
    ) -> Result<
        M1AuthenticatedScheduledLongLivedQueueRearmV1,
        M1AuthenticatedLongLivedQueueRearmScheduleFailureV1,
    > {
        schedule_m1_authenticated_long_lived_queue_rearm_inner_v1(
            engine,
            self,
            M1AuthenticatedRearmDispatchV1::Exact {
                expected_epoch,
                requests,
            },
            None,
        )
    }

    pub(crate) fn schedule_next_exact_resident<const C: usize>(
        self,
        engine: &mut Engine<C>,
        expected_epoch: CompletionEpoch,
        requests: &[RequestId],
        scratch: &mut M1AuthenticatedResidentCompletionScratchV1,
    ) -> Result<
        M1AuthenticatedScheduledLongLivedQueueRearmV1,
        M1AuthenticatedLongLivedQueueRearmScheduleFailureV1,
    > {
        schedule_m1_authenticated_long_lived_queue_rearm_inner_v1(
            engine,
            self,
            M1AuthenticatedRearmDispatchV1::Exact {
                expected_epoch,
                requests,
            },
            Some(scratch),
        )
    }
}

/// Clean authenticated queue teardown retaining complete released-round lineage.
#[must_use = "authenticated released-round teardown custody remains retained"]
#[derive(Debug)]
pub struct M1AuthenticatedLongLivedQueueRearmTeardownSuccessV1 {
    released: crate::M1AuthenticatedReleasedQueueTeardownSuccessV1,
    parked: Vec<ActiveDeviceKvCache>,
    terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
    history: M1RearmRoundHistoryV1,
    new_window_bridge:
        Option<crate::authenticated_queue_rollover::M1AuthenticatedSpeculativeNewWindowBridgeV1>,
}

/// Terminal authenticated queue-release quarantine retaining released-round lineage.
#[must_use = "authenticated released-round quarantine custody remains retained"]
#[derive(Debug)]
pub struct M1AuthenticatedLongLivedQueueRearmTeardownFailureV1 {
    released: Box<crate::M1AuthenticatedReleasedQueueTeardownFailureV1>,
    parked: Vec<ActiveDeviceKvCache>,
    terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
    history: M1RearmRoundHistoryV1,
    new_window_bridge:
        Option<crate::authenticated_queue_rollover::M1AuthenticatedSpeculativeNewWindowBridgeV1>,
}

impl M1AuthenticatedLongLivedQueueRearmTeardownSuccessV1 {
    pub const fn released(&self) -> &crate::M1AuthenticatedReleasedQueueTeardownSuccessV1 {
        &self.released
    }

    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.parked.len()
    }

    #[must_use]
    pub const fn terminal_lineage_count(&self) -> usize {
        self.terminal.len()
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&crate::M1RearmRoundHistoryEntryV1> {
        self.history.get(index)
    }
}

impl M1AuthenticatedLongLivedQueueRearmTeardownFailureV1 {
    pub const fn released(&self) -> &crate::M1AuthenticatedReleasedQueueTeardownFailureV1 {
        &self.released
    }

    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.parked.len()
    }

    #[must_use]
    pub const fn terminal_lineage_count(&self) -> usize {
        self.terminal.len()
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&crate::M1RearmRoundHistoryEntryV1> {
        self.history.get(index)
    }
}

/// Unchanged released-step rejection before queue detachment.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedLongLivedQueueRearmScheduleRejectionV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1AuthenticatedLongLivedQueueRearmScheduleRejectionV1>();
/// ```
#[must_use = "the intact authenticated released step remains retry-capable"]
#[derive(Debug)]
pub struct M1AuthenticatedLongLivedQueueRearmScheduleRejectionV1 {
    error: M1AuthenticatedLongLivedQueueRearmScheduleErrorV1,
    released: Box<M1AuthenticatedLongLivedQueueReleasedRoundV1>,
}

impl M1AuthenticatedLongLivedQueueRearmScheduleRejectionV1 {
    #[must_use]
    pub const fn error(&self) -> M1AuthenticatedLongLivedQueueRearmScheduleErrorV1 {
        self.error
    }

    /// Recovers the diagnostic and exact unchanged released-round owner.
    #[must_use = "the diagnostic and released-round owner remain retained"]
    pub fn into_parts(
        self,
    ) -> (
        M1AuthenticatedLongLivedQueueRearmScheduleErrorV1,
        M1AuthenticatedLongLivedQueueReleasedRoundV1,
    ) {
        (self.error, *self.released)
    }
}

#[derive(Debug)]
struct AuthenticatedScheduleOpaqueCustodyV1(Box<dyn fmt::Debug>);

/// Terminal authenticated scheduling custody after queue detachment begins.
#[must_use = "terminal authenticated scheduling custody must remain retained"]
#[derive(Debug)]
pub struct M1AuthenticatedLongLivedQueueRearmScheduleTerminalV1 {
    error: M1AuthenticatedLongLivedQueueRearmScheduleErrorV1,
    phase: M1LongLivedQueueRearmSchedulePhaseV1,
    retained: AuthenticatedScheduleOpaqueCustodyV1,
}

impl M1AuthenticatedLongLivedQueueRearmScheduleTerminalV1 {
    #[must_use]
    pub const fn error(&self) -> M1AuthenticatedLongLivedQueueRearmScheduleErrorV1 {
        self.error
    }

    #[must_use]
    pub const fn phase(&self) -> M1LongLivedQueueRearmSchedulePhaseV1 {
        self.phase
    }

    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        true
    }

    #[must_use]
    pub fn retains_custody(&self) -> bool {
        let _ = &self.retained.0;
        true
    }
}

/// Intact retry rejection or terminal post-detach authenticated custody.
#[must_use = "authenticated scheduling failure must be retained or handled"]
#[derive(Debug)]
pub enum M1AuthenticatedLongLivedQueueRearmScheduleFailureV1 {
    /// Pure rejection retaining the exact released-step owner.
    Rejected(Box<M1AuthenticatedLongLivedQueueRearmScheduleRejectionV1>),
    /// Detach or later failure after permanently faulting the Engine.
    Terminal(Box<M1AuthenticatedLongLivedQueueRearmScheduleTerminalV1>),
}

#[derive(Debug)]
struct M1AuthenticatedReleasedStepResidueV1 {
    checked: M1CheckedCompletionOutputV1,
    members: Vec<M1ReleasedDeviceKvMemberV1>,
    logical_accepted_counts: Box<[u32]>,
    externally_published_counts: Box<[u32]>,
    release_counts: Box<[M1CompletedKvPageReleaseCountsV1]>,
    completed_members: usize,
    total_released: usize,
    history: M1RearmRoundHistoryV1,
}

/// One detached authenticated queue paired with exactly one next scheduler batch.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedScheduledLongLivedQueueRearmV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1AuthenticatedScheduledLongLivedQueueRearmV1>();
/// ```
#[must_use = "scheduled authenticated rearm custody must proceed or remain retained"]
#[derive(Debug)]
pub struct M1AuthenticatedScheduledLongLivedQueueRearmV1 {
    queue: M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1,
    scheduled: M1ScheduledDispatchV1,
    selected: Vec<ActiveDeviceKvCache>,
    parked: Vec<ActiveDeviceKvCache>,
    terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
    prior_checked: M1CheckedCompletionOutputV1,
    logical_accepted_counts: Box<[u32]>,
    externally_published_counts: Box<[u32]>,
    release_counts: Box<[M1CompletedKvPageReleaseCountsV1]>,
    completed_members: usize,
    total_released: usize,
    history: M1RearmRoundHistoryV1,
}

/// Exact failure to bind a selected request through retained queue custody.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedRetainedStepPlanErrorV1 {
    /// The request is not a member of the scheduler-issued next batch.
    RequestNotSelected(RequestId),
    /// The retained generated runner declaration rejected its exact selection.
    Declaration(LogicalRunnerError),
}

impl M1AuthenticatedScheduledLongLivedQueueRearmV1 {
    /// Exact once-issued next scheduler batch.
    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        &self.scheduled
    }

    #[must_use]
    pub fn selected_requests(&self) -> impl ExactSizeIterator<Item = RequestId> + '_ {
        self.selected.iter().map(|cache| cache.projection().request)
    }

    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.parked.len()
    }

    #[must_use]
    pub const fn terminal_count(&self) -> usize {
        self.terminal.len()
    }

    #[must_use]
    pub fn prior_logical_accepted_counts(&self) -> &[u32] {
        &self.logical_accepted_counts
    }

    #[must_use]
    pub fn prior_externally_published_counts(&self) -> &[u32] {
        &self.externally_published_counts
    }

    /// Binds one selected request through the exact declaration retained by the queue.
    ///
    /// # Errors
    ///
    /// Rejects a request outside the scheduler-issued batch or any drift in the retained
    /// generated declaration. Neither selection nor epoch is accepted from the caller.
    pub fn bind_selected_step_plan(
        &self,
        request: RequestId,
    ) -> Result<StepPlan, M1AuthenticatedRetainedStepPlanErrorV1> {
        if !self.selected_requests().any(|selected| selected == request) {
            return Err(M1AuthenticatedRetainedStepPlanErrorV1::RequestNotSelected(
                request,
            ));
        }
        self.queue
            .operations()
            .runner()
            .bind_step_plan(
                request,
                self.scheduled.epoch(),
                self.queue.custody().selection(),
            )
            .map_err(M1AuthenticatedRetainedStepPlanErrorV1::Declaration)
    }

    /// Derives the next addressless physical recipe from retained queue shape and selection.
    ///
    /// The caller supplies only workspace plans. The operation plan, target selection, and
    /// step shape all remain private and are derived from authenticated queue custody.
    pub fn derive_retained_step_recipe(
        &self,
        workspace_plans: M1FullStepWorkspacePlans,
    ) -> M1PhysicalRunnerRecipeOutcomeV1 {
        let selection = self.queue.custody().selection();
        let intent = match self.queue.shape() {
            M1PhysicalFixedBatchShapeV1::DraftCatchup => {
                M1StepDispatchIntent::DraftCatchup(selection)
            }
            M1PhysicalFixedBatchShapeV1::TargetOnly => M1StepDispatchIntent::TargetOnly(selection),
            M1PhysicalFixedBatchShapeV1::PairedPrefill => {
                M1StepDispatchIntent::PairedPrefill(selection)
            }
            M1PhysicalFixedBatchShapeV1::SpeculativeK4
            | M1PhysicalFixedBatchShapeV1::SpeculativeK8
            | M1PhysicalFixedBatchShapeV1::SpeculativeK16 => {
                M1StepDispatchIntent::SpeculativeRound(selection)
            }
        };
        crate::runner::derive_physical_step_recipe(self.queue.operations(), intent, workspace_plans)
    }

    pub(crate) fn derive_retained_step_recipe_input(
        &self,
        recipe: crate::runner::M1PhysicalRunnerRecipeInputV1,
    ) -> M1PhysicalRunnerRecipeOutcomeV1 {
        let selection = self.queue.custody().selection();
        let intent = match self.queue.shape() {
            M1PhysicalFixedBatchShapeV1::DraftCatchup => {
                M1StepDispatchIntent::DraftCatchup(selection)
            }
            M1PhysicalFixedBatchShapeV1::TargetOnly => M1StepDispatchIntent::TargetOnly(selection),
            M1PhysicalFixedBatchShapeV1::PairedPrefill => {
                M1StepDispatchIntent::PairedPrefill(selection)
            }
            M1PhysicalFixedBatchShapeV1::SpeculativeK4
            | M1PhysicalFixedBatchShapeV1::SpeculativeK8
            | M1PhysicalFixedBatchShapeV1::SpeculativeK16 => {
                M1StepDispatchIntent::SpeculativeRound(selection)
            }
        };
        recipe.derive(self.queue.operations(), intent)
    }
}

/// Materializes only exact missing speculative KV tail pages after the
/// authenticated queue has detached. The caller supplies no page authority.
pub(crate) fn materialize_m1_authenticated_speculative_round_tail_pages_v1(
    scheduled: M1AuthenticatedScheduledLongLivedQueueRearmV1,
    draft_decode: ValidatedM1StepInputs,
    target_speculative: ValidatedM1StepInputs,
    draft_round_tokens: u32,
) -> Result<
    (
        M1AuthenticatedScheduledLongLivedQueueRearmV1,
        M1LongLivedQueueRearmKvInputsV1,
    ),
    M1AuthenticatedLongLivedQueueRearmKvReservationFailureV1,
> {
    materialize_m1_authenticated_speculative_round_tail_pages_inner_v1(
        scheduled,
        draft_decode,
        target_speculative,
        draft_round_tokens,
        None,
    )
}

pub(crate) fn materialize_m1_authenticated_speculative_round_tail_pages_resident_v1(
    scheduled: M1AuthenticatedScheduledLongLivedQueueRearmV1,
    draft_decode: ValidatedM1StepInputs,
    target_speculative: ValidatedM1StepInputs,
    draft_round_tokens: u32,
    scratch: &mut M1AuthenticatedResidentCompletionScratchV1,
) -> Result<
    (
        M1AuthenticatedScheduledLongLivedQueueRearmV1,
        M1LongLivedQueueRearmKvInputsV1,
    ),
    M1AuthenticatedLongLivedQueueRearmKvReservationFailureV1,
> {
    let rosters = (
        core::mem::take(&mut scratch.draft_page_leases),
        core::mem::take(&mut scratch.target_page_leases),
    );
    materialize_m1_authenticated_speculative_round_tail_pages_inner_v1(
        scheduled,
        draft_decode,
        target_speculative,
        draft_round_tokens,
        Some(rosters),
    )
}

fn materialize_m1_authenticated_speculative_round_tail_pages_inner_v1(
    mut scheduled: M1AuthenticatedScheduledLongLivedQueueRearmV1,
    draft_decode: ValidatedM1StepInputs,
    target_speculative: ValidatedM1StepInputs,
    draft_round_tokens: u32,
    resident_rosters: Option<
        crate::authenticated_queue_rollover::M1AuthenticatedSpeculativeTailPageRostersV1,
    >,
) -> Result<
    (
        M1AuthenticatedScheduledLongLivedQueueRearmV1,
        M1LongLivedQueueRearmKvInputsV1,
    ),
    M1AuthenticatedLongLivedQueueRearmKvReservationFailureV1,
> {
    let selected_count = scheduled.selected.len();
    let draft_live = usize::try_from(draft_decode.live_lane_count()).ok();
    let target_live = usize::try_from(target_speculative.live_lane_count()).ok();
    if draft_live != Some(selected_count)
        || target_live != Some(selected_count)
        || !authenticated_input_roster_matches(&scheduled, &target_speculative)
        || draft_decode
            .lanes()
            .iter()
            .take(selected_count)
            .zip(&scheduled.selected)
            .any(|(plan, cache)| {
                plan.is_none_or(|plan| {
                    plan.request() != cache.projection().request
                        || plan.completion_epoch() != scheduled.scheduled.epoch()
                })
            })
    {
        return Err(authenticated_kv_reservation_failure(
            M1LongLivedQueueRearmKvReservationPhaseV1::Preflight,
            (scheduled, draft_decode, target_speculative),
        ));
    }
    let mut projections =
        ArrayVec::<crate::DeviceKvCacheProjection, { M1_MAX_ACTIVE_SEQUENCES as usize }>::new();
    for projection in scheduled
        .selected
        .iter()
        .map(ActiveDeviceKvCache::projection)
    {
        if projections.try_push(projection).is_err() {
            return Err(authenticated_kv_reservation_failure(
                M1LongLivedQueueRearmKvReservationPhaseV1::Preflight,
                (scheduled, draft_decode, target_speculative),
            ));
        }
    }
    let (draft_page_leases, target_page_leases) =
        match crate::authenticated_queue_rollover::materialize_m1_authenticated_speculative_tail_pages_with_rosters_v1(
            &mut scheduled.queue,
            &projections,
            &draft_decode,
            &target_speculative,
            draft_round_tokens,
            resident_rosters,
        ) {
            Ok(page_leases) => page_leases,
            Err(source) => {
                return Err(authenticated_kv_reservation_failure(
                    M1LongLivedQueueRearmKvReservationPhaseV1::Preflight,
                    (scheduled, draft_decode, target_speculative, source),
                ));
            }
        };
    Ok((
        scheduled,
        M1LongLivedQueueRearmKvInputsV1::SpeculativeRound {
            draft_decode,
            target_speculative,
            draft_page_leases,
            target_page_leases,
        },
    ))
}

#[derive(Clone, Copy)]
enum M1AuthenticatedRearmDispatchV1<'a> {
    Automatic,
    Exact {
        expected_epoch: CompletionEpoch,
        requests: &'a [RequestId],
    },
}

#[derive(Debug)]
enum M1AuthenticatedRearmDispatchFailureV1 {
    EmptyAutomatic,
    Automatic(EngineError),
    Exact(M1ExactDispatchErrorV1),
}

fn exact_next_epoch(previous: CompletionEpoch) -> Option<CompletionEpoch> {
    previous.value().checked_add(1).map(CompletionEpoch::new)
}

fn validate_authenticated_rearm_eligibility(
    shape: M1PhysicalFixedBatchShapeV1,
    selection: Qwen3PlanSelection,
    qualification_logits_enabled: bool,
) -> Result<(), M1LongLivedQueueRearmScheduleErrorV1> {
    let supported = match shape {
        M1PhysicalFixedBatchShapeV1::TargetOnly => selection.mode == Qwen3ExecutionMode::Decode,
        M1PhysicalFixedBatchShapeV1::SpeculativeK4
        | M1PhysicalFixedBatchShapeV1::SpeculativeK8
        | M1PhysicalFixedBatchShapeV1::SpeculativeK16 => {
            selection.mode == Qwen3ExecutionMode::Speculative
        }
        M1PhysicalFixedBatchShapeV1::PairedPrefill | M1PhysicalFixedBatchShapeV1::DraftCatchup => {
            false
        }
    };
    let qualification_supported = !qualification_logits_enabled
        || (shape == M1PhysicalFixedBatchShapeV1::TargetOnly
            && selection.mode == Qwen3ExecutionMode::Decode);
    if supported && qualification_supported {
        Ok(())
    } else {
        Err(M1LongLivedQueueRearmScheduleErrorV1::UnsupportedPriorShape)
    }
}

fn validate_authenticated_request_partition(
    available: &[RequestId],
    scheduled: &[RequestId],
) -> Result<(), M1LongLivedQueueRearmScheduleErrorV1> {
    for (lane, request) in scheduled.iter().copied().enumerate() {
        if let Some(first_lane) = scheduled[..lane]
            .iter()
            .position(|prior| prior.slot() == request.slot())
        {
            return Err(
                M1LongLivedQueueRearmScheduleErrorV1::DuplicateScheduledRequest {
                    first_lane,
                    lane,
                },
            );
        }
        if !available.contains(&request) {
            return Err(M1LongLivedQueueRearmScheduleErrorV1::UnownedScheduledRequest { lane });
        }
    }
    Ok(())
}

fn validate_authenticated_available_roster(
    released: &M1AuthenticatedReleasedCompletedStepV1,
    parked: &[ActiveDeviceKvCache],
) -> Result<(), M1AuthenticatedLongLivedQueueRearmScheduleErrorV1> {
    for (member, current) in released.members().iter().enumerate() {
        let M1ReleasedDeviceKvMemberV1::Active(current) = current else {
            continue;
        };
        if let Some(first_member) = released.members()[..member].iter().position(|prior| {
            matches!(
                prior,
                M1ReleasedDeviceKvMemberV1::Active(prior)
                    if prior.projection().request.slot() == current.projection().request.slot()
            )
        }) {
            return Err(
                M1AuthenticatedLongLivedQueueRearmScheduleErrorV1::DuplicateAvailableRequest {
                    first_member,
                    member,
                },
            );
        }
    }
    for (parked_member, current) in parked.iter().enumerate() {
        let member = released.members().len() + parked_member;
        if let Some(first_member) = released.members().iter().position(|prior| {
            matches!(
                prior,
                M1ReleasedDeviceKvMemberV1::Active(prior)
                    if prior.projection().request.slot() == current.projection().request.slot()
            )
        }) {
            return Err(
                M1AuthenticatedLongLivedQueueRearmScheduleErrorV1::DuplicateAvailableRequest {
                    first_member,
                    member,
                },
            );
        }
        if let Some(first_parked) = parked[..parked_member].iter().position(|prior| {
            prior.projection().request.slot() == current.projection().request.slot()
        }) {
            return Err(
                M1AuthenticatedLongLivedQueueRearmScheduleErrorV1::DuplicateAvailableRequest {
                    first_member: released.members().len() + first_parked,
                    member,
                },
            );
        }
    }
    Ok(())
}

fn validate_authenticated_exact_rearm_preflight<const C: usize>(
    engine: &Engine<C>,
    released: &M1AuthenticatedReleasedCompletedStepV1,
    parked: &[ActiveDeviceKvCache],
    expected_epoch: CompletionEpoch,
    requests: &[RequestId],
) -> Result<(), M1LongLivedQueueRearmScheduleErrorV1> {
    if requests.is_empty() {
        return Err(M1LongLivedQueueRearmScheduleErrorV1::ExactScheduler(
            M1ExactDispatchErrorV1::EmptyRoster,
        ));
    }
    if requests.len() > M1_MAX_ACTIVE_SEQUENCES as usize {
        return Err(M1LongLivedQueueRearmScheduleErrorV1::ExactScheduler(
            M1ExactDispatchErrorV1::RosterTooLarge {
                maximum: M1_MAX_ACTIVE_SEQUENCES as usize,
                actual: requests.len(),
            },
        ));
    }
    let Some(next_epoch) = exact_next_epoch(released.checked().epoch()) else {
        return Err(M1LongLivedQueueRearmScheduleErrorV1::EpochExhausted);
    };
    if expected_epoch != next_epoch {
        return Err(M1LongLivedQueueRearmScheduleErrorV1::EpochNotExactNext {
            expected: next_epoch,
            actual: expected_epoch,
        });
    }
    let mut available = ArrayVec::<RequestId, { M1_MAX_ACTIVE_SEQUENCES as usize }>::new();
    for request in released
        .members()
        .iter()
        .filter_map(|member| match member {
            M1ReleasedDeviceKvMemberV1::Active(cache) => Some(cache.projection().request),
            M1ReleasedDeviceKvMemberV1::Terminal(_) => None,
        })
        .chain(parked.iter().map(|cache| cache.projection().request))
    {
        available
            .try_push(request)
            .map_err(|_| M1LongLivedQueueRearmScheduleErrorV1::HostAllocation)?;
    }
    validate_authenticated_request_partition(&available, requests)?;
    for (lane, request) in requests.iter().copied().enumerate() {
        match engine.state(request) {
            Some(RequestState::Ready) => {}
            None => {
                return Err(M1LongLivedQueueRearmScheduleErrorV1::ExactScheduler(
                    M1ExactDispatchErrorV1::MissingRequest { lane, request },
                ));
            }
            Some(state) => {
                return Err(M1LongLivedQueueRearmScheduleErrorV1::ExactScheduler(
                    M1ExactDispatchErrorV1::RequestNotReady {
                        lane,
                        request,
                        state,
                    },
                ));
            }
        }
    }
    Ok(())
}

fn dispatch_authenticated_rearm<const C: usize>(
    engine: &mut Engine<C>,
    dispatch: M1AuthenticatedRearmDispatchV1<'_>,
) -> Result<M1ScheduledDispatchV1, M1AuthenticatedRearmDispatchFailureV1> {
    match dispatch {
        M1AuthenticatedRearmDispatchV1::Automatic => engine
            .dispatch_m1_ready()
            .map_err(M1AuthenticatedRearmDispatchFailureV1::Automatic)?
            .ok_or(M1AuthenticatedRearmDispatchFailureV1::EmptyAutomatic),
        M1AuthenticatedRearmDispatchV1::Exact {
            expected_epoch,
            requests,
        } => engine
            .dispatch_m1_exact_ready(expected_epoch, requests)
            .map_err(M1AuthenticatedRearmDispatchFailureV1::Exact),
    }
}

fn authenticated_schedule_rejection(
    error: M1AuthenticatedLongLivedQueueRearmScheduleErrorV1,
    released: M1AuthenticatedLongLivedQueueReleasedRoundV1,
) -> M1AuthenticatedLongLivedQueueRearmScheduleFailureV1 {
    M1AuthenticatedLongLivedQueueRearmScheduleFailureV1::Rejected(Box::new(
        M1AuthenticatedLongLivedQueueRearmScheduleRejectionV1 {
            error,
            released: Box::new(released),
        },
    ))
}

fn authenticated_schedule_terminal<const C: usize>(
    engine: &mut Engine<C>,
    phase: M1LongLivedQueueRearmSchedulePhaseV1,
    error: M1AuthenticatedLongLivedQueueRearmScheduleErrorV1,
    retained: impl fmt::Debug + 'static,
) -> M1AuthenticatedLongLivedQueueRearmScheduleFailureV1 {
    engine.quarantine_m1_queue_rearm_failure();
    M1AuthenticatedLongLivedQueueRearmScheduleFailureV1::Terminal(Box::new(
        M1AuthenticatedLongLivedQueueRearmScheduleTerminalV1 {
            error,
            phase,
            retained: AuthenticatedScheduleOpaqueCustodyV1(Box::new(retained)),
        },
    ))
}

fn schedule_m1_authenticated_long_lived_queue_rearm_inner_v1<const C: usize>(
    engine: &mut Engine<C>,
    released_round: M1AuthenticatedLongLivedQueueReleasedRoundV1,
    dispatch: M1AuthenticatedRearmDispatchV1<'_>,
    resident_scratch: Option<&mut M1AuthenticatedResidentCompletionScratchV1>,
) -> Result<
    M1AuthenticatedScheduledLongLivedQueueRearmV1,
    M1AuthenticatedLongLivedQueueRearmScheduleFailureV1,
> {
    if engine.is_faulted() {
        return Err(authenticated_schedule_rejection(
            M1AuthenticatedLongLivedQueueRearmScheduleErrorV1::EngineFaulted,
            released_round,
        ));
    }
    if released_round.has_pending_new_window_bridge() {
        return Err(authenticated_schedule_rejection(
            M1AuthenticatedLongLivedQueueRearmScheduleErrorV1::PendingNewWindowBridge,
            released_round,
        ));
    }
    if released_round.history.len() >= crate::M1_MAX_REARM_ROUND_HISTORY_V1 {
        return Err(authenticated_schedule_rejection(
            M1AuthenticatedLongLivedQueueRearmScheduleErrorV1::Shared(
                M1LongLivedQueueRearmScheduleErrorV1::RoundHistoryCapacity {
                    maximum: crate::M1_MAX_REARM_ROUND_HISTORY_V1,
                },
            ),
            released_round,
        ));
    }
    let released = &released_round.released;
    let shape = released.queue().shape();
    let selection = released.queue().custody().selection();
    if let Err(error) = validate_authenticated_rearm_eligibility(
        shape,
        selection,
        released
            .queue()
            .custody()
            .completion_output()
            .qualification_logits()
            .is_some(),
    ) {
        return Err(authenticated_schedule_rejection(
            M1AuthenticatedLongLivedQueueRearmScheduleErrorV1::Shared(error),
            released_round,
        ));
    }
    if !released
        .members()
        .iter()
        .any(|member| matches!(member, M1ReleasedDeviceKvMemberV1::Active(_)))
        && released_round.parked.is_empty()
    {
        return Err(authenticated_schedule_rejection(
            M1AuthenticatedLongLivedQueueRearmScheduleErrorV1::Shared(
                M1LongLivedQueueRearmScheduleErrorV1::NoContinuingRequests,
            ),
            released_round,
        ));
    }
    if let Err(error) = validate_authenticated_available_roster(released, &released_round.parked) {
        return Err(authenticated_schedule_terminal(
            engine,
            M1LongLivedQueueRearmSchedulePhaseV1::Released,
            error,
            released_round,
        ));
    }
    if let M1AuthenticatedRearmDispatchV1::Exact {
        expected_epoch,
        requests,
    } = dispatch
    {
        if let Err(error) = validate_authenticated_exact_rearm_preflight(
            engine,
            released,
            &released_round.parked,
            expected_epoch,
            requests,
        ) {
            return Err(authenticated_schedule_rejection(
                M1AuthenticatedLongLivedQueueRearmScheduleErrorV1::Shared(error),
                released_round,
            ));
        }
    }

    let M1AuthenticatedLongLivedQueueReleasedRoundV1 {
        mut released,
        parked,
        terminal,
        history,
        new_window_bridge,
    } = released_round;
    debug_assert!(new_window_bridge.is_none());
    let additional_members = parked.len() + terminal.len();
    if released
        .try_reserve_rearm_members(additional_members)
        .is_err()
    {
        return Err(authenticated_schedule_rejection(
            M1AuthenticatedLongLivedQueueRearmScheduleErrorV1::Shared(
                M1LongLivedQueueRearmScheduleErrorV1::HostAllocation,
            ),
            M1AuthenticatedLongLivedQueueReleasedRoundV1 {
                released,
                parked,
                terminal,
                history,
                new_window_bridge,
            },
        ));
    }
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
    members.extend(parked.into_iter().map(M1ReleasedDeviceKvMemberV1::Active));
    members.extend(
        terminal
            .into_iter()
            .map(M1ReleasedDeviceKvMemberV1::Terminal),
    );
    let residue = M1AuthenticatedReleasedStepResidueV1 {
        checked,
        members,
        logical_accepted_counts,
        externally_published_counts,
        release_counts,
        completed_members,
        total_released,
        history,
    };
    let queue = match queue.detach() {
        Ok(queue) => queue,
        Err(failure) => {
            return Err(authenticated_schedule_terminal(
                engine,
                M1LongLivedQueueRearmSchedulePhaseV1::QueueDetach,
                M1AuthenticatedLongLivedQueueRearmScheduleErrorV1::Shared(
                    M1LongLivedQueueRearmScheduleErrorV1::Detach,
                ),
                (failure, residue),
            ));
        }
    };
    let scheduled = match dispatch_authenticated_rearm(engine, dispatch) {
        Ok(scheduled) => scheduled,
        Err(failure) => {
            let error = match &failure {
                M1AuthenticatedRearmDispatchFailureV1::EmptyAutomatic => {
                    M1LongLivedQueueRearmScheduleErrorV1::EmptySchedulerBatch
                }
                M1AuthenticatedRearmDispatchFailureV1::Automatic(error) => {
                    let _ = error;
                    M1LongLivedQueueRearmScheduleErrorV1::Scheduler
                }
                M1AuthenticatedRearmDispatchFailureV1::Exact(error) => {
                    M1LongLivedQueueRearmScheduleErrorV1::ExactScheduler(*error)
                }
            };
            return Err(authenticated_schedule_terminal(
                engine,
                M1LongLivedQueueRearmSchedulePhaseV1::Detached,
                M1AuthenticatedLongLivedQueueRearmScheduleErrorV1::Shared(error),
                (queue, residue, failure),
            ));
        }
    };
    let Some(expected_epoch) = exact_next_epoch(residue.checked.epoch()) else {
        return Err(authenticated_schedule_terminal(
            engine,
            M1LongLivedQueueRearmSchedulePhaseV1::PostDispatch,
            M1AuthenticatedLongLivedQueueRearmScheduleErrorV1::Shared(
                M1LongLivedQueueRearmScheduleErrorV1::EpochExhausted,
            ),
            (queue, residue, scheduled),
        ));
    };
    if scheduled.epoch() != expected_epoch {
        let actual = scheduled.epoch();
        return Err(authenticated_schedule_terminal(
            engine,
            M1LongLivedQueueRearmSchedulePhaseV1::PostDispatch,
            M1AuthenticatedLongLivedQueueRearmScheduleErrorV1::Shared(
                M1LongLivedQueueRearmScheduleErrorV1::EpochNotExactNext {
                    expected: expected_epoch,
                    actual,
                },
            ),
            (queue, residue, scheduled),
        ));
    }

    let mut scheduled_requests = ArrayVec::<RequestId, { M1_MAX_ACTIVE_SEQUENCES as usize }>::new();
    for lane in 0..scheduled.member_count() {
        let Some(request) = scheduled.member(lane) else {
            return Err(authenticated_schedule_terminal(
                engine,
                M1LongLivedQueueRearmSchedulePhaseV1::PostDispatch,
                M1AuthenticatedLongLivedQueueRearmScheduleErrorV1::Shared(
                    M1LongLivedQueueRearmScheduleErrorV1::MalformedSchedulerBatch { lane },
                ),
                (queue, residue, scheduled),
            ));
        };
        if scheduled_requests.try_push(request).is_err() {
            return Err(authenticated_schedule_terminal(
                engine,
                M1LongLivedQueueRearmSchedulePhaseV1::PostDispatch,
                M1AuthenticatedLongLivedQueueRearmScheduleErrorV1::Shared(
                    M1LongLivedQueueRearmScheduleErrorV1::HostAllocation,
                ),
                (queue, residue, scheduled),
            ));
        }
    }
    let mut available_requests = ArrayVec::<RequestId, { M1_MAX_ACTIVE_SEQUENCES as usize }>::new();
    for request in residue.members.iter().filter_map(|member| match member {
        M1ReleasedDeviceKvMemberV1::Active(cache) => Some(cache.projection().request),
        M1ReleasedDeviceKvMemberV1::Terminal(_) => None,
    }) {
        if available_requests.try_push(request).is_err() {
            return Err(authenticated_schedule_terminal(
                engine,
                M1LongLivedQueueRearmSchedulePhaseV1::PostDispatch,
                M1AuthenticatedLongLivedQueueRearmScheduleErrorV1::Shared(
                    M1LongLivedQueueRearmScheduleErrorV1::HostAllocation,
                ),
                (queue, residue, scheduled),
            ));
        }
    }
    if let Err(error) =
        validate_authenticated_request_partition(&available_requests, &scheduled_requests)
    {
        return Err(authenticated_schedule_terminal(
            engine,
            M1LongLivedQueueRearmSchedulePhaseV1::PostDispatch,
            M1AuthenticatedLongLivedQueueRearmScheduleErrorV1::Shared(error),
            (queue, residue, scheduled),
        ));
    }

    let (mut selected_slots, mut selected, mut parked, mut terminal_members) =
        match resident_scratch {
            Some(scratch) => (
                core::mem::take(&mut scratch.selected_slots),
                core::mem::take(&mut scratch.selected),
                core::mem::take(&mut scratch.parked),
                core::mem::take(&mut scratch.terminal),
            ),
            None => (Vec::new(), Vec::new(), Vec::new(), Vec::new()),
        };
    selected_slots.clear();
    selected.clear();
    parked.clear();
    terminal_members.clear();
    if selected_slots.capacity() < scheduled.member_count()
        || selected.capacity() < scheduled.member_count()
        || parked.capacity() < residue.members.len()
        || terminal_members.capacity() < residue.members.len()
    {
        let reserve_failed = selected_slots
            .try_reserve_exact(scheduled.member_count())
            .is_err()
            || selected
                .try_reserve_exact(scheduled.member_count())
                .is_err()
            || parked.try_reserve_exact(residue.members.len()).is_err()
            || terminal_members
                .try_reserve_exact(residue.members.len())
                .is_err();
        if reserve_failed {
            return Err(authenticated_schedule_terminal(
                engine,
                M1LongLivedQueueRearmSchedulePhaseV1::PostDispatch,
                M1AuthenticatedLongLivedQueueRearmScheduleErrorV1::Shared(
                    M1LongLivedQueueRearmScheduleErrorV1::HostAllocation,
                ),
                (queue, residue, scheduled),
            ));
        }
    }
    selected_slots.resize_with(scheduled.member_count(), || None);
    for member in residue.members {
        match member {
            M1ReleasedDeviceKvMemberV1::Active(cache) => {
                let request = cache.projection().request;
                if let Some(lane) = scheduled_requests
                    .iter()
                    .position(|scheduled_request| *scheduled_request == request)
                {
                    selected_slots[lane] = Some(cache);
                } else {
                    parked.push(cache);
                }
            }
            M1ReleasedDeviceKvMemberV1::Terminal(observation) => {
                terminal_members.push(observation);
            }
        }
    }
    selected.extend(selected_slots.into_iter().flatten());
    Ok(M1AuthenticatedScheduledLongLivedQueueRearmV1 {
        queue,
        scheduled,
        selected,
        parked,
        terminal: terminal_members,
        prior_checked: residue.checked,
        logical_accepted_counts: residue.logical_accepted_counts,
        externally_published_counts: residue.externally_published_counts,
        release_counts: residue.release_counts,
        completed_members: residue.completed_members,
        total_released: residue.total_released,
        history: residue.history,
    })
}

/// Detaches an authenticated released step and captures one automatic ready batch.
///
/// # Errors
///
/// Returns the unchanged released owner for pure preflight rejection. Any
/// failure after detachment permanently faults `engine` and returns opaque
/// custody retaining every owner available at that phase.
pub fn schedule_m1_authenticated_long_lived_queue_rearm_v1<const C: usize>(
    engine: &mut Engine<C>,
    released: M1AuthenticatedReleasedCompletedStepV1,
) -> Result<
    M1AuthenticatedScheduledLongLivedQueueRearmV1,
    M1AuthenticatedLongLivedQueueRearmScheduleFailureV1,
> {
    schedule_m1_authenticated_long_lived_queue_rearm_inner_v1(
        engine,
        M1AuthenticatedLongLivedQueueReleasedRoundV1::initial(released),
        M1AuthenticatedRearmDispatchV1::Automatic,
        None,
    )
}

/// Detaches an authenticated released step and captures the exact named ready roster.
///
/// # Errors
///
/// Returns the unchanged released owner for pure roster or epoch rejection.
/// Any failure after detachment permanently faults `engine` and returns opaque
/// custody retaining every owner available at that phase.
pub fn schedule_m1_authenticated_long_lived_queue_rearm_exact_v1<const C: usize>(
    engine: &mut Engine<C>,
    released: M1AuthenticatedReleasedCompletedStepV1,
    expected_epoch: CompletionEpoch,
    requests: &[RequestId],
) -> Result<
    M1AuthenticatedScheduledLongLivedQueueRearmV1,
    M1AuthenticatedLongLivedQueueRearmScheduleFailureV1,
> {
    schedule_m1_authenticated_long_lived_queue_rearm_inner_v1(
        engine,
        M1AuthenticatedLongLivedQueueReleasedRoundV1::initial(released),
        M1AuthenticatedRearmDispatchV1::Exact {
            expected_epoch,
            requests,
        },
        None,
    )
}

/// Authenticated detached queue and exact next-round KV table custody.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedReservedLongLivedQueueRearmV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1AuthenticatedReservedLongLivedQueueRearmV1>();
/// ```
#[must_use = "authenticated KV reservations must proceed with detached queue custody"]
#[derive(Debug)]
pub struct M1AuthenticatedReservedLongLivedQueueRearmV1 {
    scheduled: M1AuthenticatedScheduledLongLivedQueueRearmV1,
    tables: M1FullStepKvWorkspaceTablesV1,
}

impl M1AuthenticatedReservedLongLivedQueueRearmV1 {
    /// Exact scheduler authority retained with the reserved tables.
    #[must_use = "scheduler authority remains retained"]
    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        self.scheduled.scheduled_dispatch()
    }

    #[must_use]
    pub fn selected_requests(&self) -> impl ExactSizeIterator<Item = RequestId> + '_ {
        self.scheduled.selected_requests()
    }
}

#[derive(Debug)]
struct AuthenticatedKvReservationOpaqueCustodyV1(Box<dyn fmt::Debug>);

/// Terminal authenticated KV reservation or workspace-table binding custody.
#[must_use = "terminal authenticated KV reservation custody must remain retained"]
#[derive(Debug)]
pub struct M1AuthenticatedLongLivedQueueRearmKvReservationFailureV1 {
    phase: M1LongLivedQueueRearmKvReservationPhaseV1,
    retained: AuthenticatedKvReservationOpaqueCustodyV1,
}

impl M1AuthenticatedLongLivedQueueRearmKvReservationFailureV1 {
    #[must_use]
    pub const fn phase(&self) -> M1LongLivedQueueRearmKvReservationPhaseV1 {
        self.phase
    }

    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        true
    }

    #[must_use]
    pub fn retains_custody(&self) -> bool {
        let _ = &self.retained.0;
        true
    }
}

fn authenticated_kv_reservation_failure(
    phase: M1LongLivedQueueRearmKvReservationPhaseV1,
    retained: impl fmt::Debug + 'static,
) -> M1AuthenticatedLongLivedQueueRearmKvReservationFailureV1 {
    M1AuthenticatedLongLivedQueueRearmKvReservationFailureV1 {
        phase,
        retained: AuthenticatedKvReservationOpaqueCustodyV1(Box::new(retained)),
    }
}

fn authenticated_input_roster_matches(
    scheduled: &M1AuthenticatedScheduledLongLivedQueueRearmV1,
    inputs: &ferric_spec::ValidatedM1StepInputs,
) -> bool {
    let Ok(live) = usize::try_from(inputs.live_lane_count()) else {
        return false;
    };
    live == scheduled.selected.len()
        && inputs.selection() == scheduled.queue.custody().selection()
        && inputs
            .lanes()
            .iter()
            .take(live)
            .zip(&scheduled.selected)
            .all(|(plan, cache)| {
                plan.is_some_and(|plan| {
                    plan.request() == cache.projection().request
                        && plan.completion_epoch() == scheduled.scheduled.epoch()
                })
            })
}

fn authenticated_qualification_logits_preflight(
    selection: Qwen3PlanSelection,
    logits: Option<&crate::BoundM1QualificationLogitsV1>,
) -> bool {
    let Some(logits) = logits else {
        return false;
    };
    let Ok(expected) = crate::m1_qualification_logits_shape_v1(selection) else {
        return false;
    };
    logits.shape() == expected
        && logits.retained_host_dispatch_range().extent_bytes() == expected.extent_bytes()
}

fn reserve_m1_authenticated_long_lived_queue_rearm_kv_inner_v1(
    mut scheduled: M1AuthenticatedScheduledLongLivedQueueRearmV1,
    inputs: M1LongLivedQueueRearmKvInputsV1,
    mut resident_scratch: Option<&mut M1AuthenticatedResidentCompletionScratchV1>,
) -> Result<
    M1AuthenticatedReservedLongLivedQueueRearmV1,
    M1AuthenticatedLongLivedQueueRearmKvReservationFailureV1,
> {
    match inputs {
        M1LongLivedQueueRearmKvInputsV1::TargetOnly {
            target,
            mut target_page_leases,
        } => {
            if scheduled.queue.shape() != M1PhysicalFixedBatchShapeV1::TargetOnly
                || !authenticated_input_roster_matches(&scheduled, &target)
                || target_page_leases.len() != scheduled.selected.len()
            {
                return Err(authenticated_kv_reservation_failure(
                    M1LongLivedQueueRearmKvReservationPhaseV1::Preflight,
                    (scheduled, target, target_page_leases),
                ));
            }
            let mut reservations = resident_scratch
                .as_deref_mut()
                .map_or_else(Vec::new, |scratch| {
                    core::mem::take(&mut scratch.target_reservations)
                });
            reservations.clear();
            if reservations.capacity() < scheduled.selected.len()
                && reservations
                    .try_reserve_exact(scheduled.selected.len())
                    .is_err()
            {
                return Err(authenticated_kv_reservation_failure(
                    M1LongLivedQueueRearmKvReservationPhaseV1::Preflight,
                    (scheduled, target, target_page_leases),
                ));
            }
            for lane in 0..scheduled.selected.len() {
                let request = scheduled.selected[lane].projection().request;
                let leases = core::mem::take(&mut target_page_leases[lane]);
                match scheduled.selected[lane].reserve_step_write(
                    request,
                    Qwen3ModelRole::Target8B,
                    target.context_lengths()[lane],
                    target.active_lengths()[lane],
                    scheduled.scheduled.epoch(),
                    leases,
                ) {
                    Ok(reservation) => reservations.push(reservation),
                    Err(failure) => {
                        let (error, leases) = (*failure).into_parts();
                        target_page_leases[lane] = leases;
                        return Err(authenticated_kv_reservation_failure(
                            M1LongLivedQueueRearmKvReservationPhaseV1::TargetReservation,
                            (
                                scheduled,
                                target,
                                target_page_leases,
                                reservations,
                                lane,
                                error,
                            ),
                        ));
                    }
                }
            }
            let target_storage = resident_scratch
                .as_deref_mut()
                .and_then(|scratch| scratch.target_table.take());
            let target_result = match target_storage {
                Some(storage) => {
                    crate::kv_workspace_authority::bind_m1_kv_workspace_table_with_storage_v1(
                        target,
                        reservations,
                        storage,
                    )
                }
                None => crate::bind_m1_kv_workspace_table_v1(target, reservations),
            };
            let target = match target_result {
                Ok(table) => table,
                Err(failure) => {
                    return Err(authenticated_kv_reservation_failure(
                        M1LongLivedQueueRearmKvReservationPhaseV1::TargetTableBinding,
                        (scheduled, target_page_leases, failure),
                    ));
                }
            };
            Ok(M1AuthenticatedReservedLongLivedQueueRearmV1 {
                scheduled,
                tables: M1FullStepKvWorkspaceTablesV1::TargetOnly { target },
            })
        }
        M1LongLivedQueueRearmKvInputsV1::QualificationTargetOnly { target, contexts } => {
            let custody = scheduled.queue.custody();
            if scheduled.queue.shape() != M1PhysicalFixedBatchShapeV1::TargetOnly
                || !authenticated_input_roster_matches(&scheduled, &target)
                || contexts.len() != scheduled.selected.len()
                || !authenticated_qualification_logits_preflight(
                    custody.selection(),
                    custody.completion_output().qualification_logits(),
                )
            {
                return Err(authenticated_kv_reservation_failure(
                    M1LongLivedQueueRearmKvReservationPhaseV1::Preflight,
                    (scheduled, target, contexts),
                ));
            }
            let mut reservations = resident_scratch
                .as_deref_mut()
                .map_or_else(Vec::new, |scratch| {
                    core::mem::take(&mut scratch.target_reservations)
                });
            reservations.clear();
            if reservations.capacity() < scheduled.selected.len()
                && reservations
                    .try_reserve_exact(scheduled.selected.len())
                    .is_err()
            {
                return Err(authenticated_kv_reservation_failure(
                    M1LongLivedQueueRearmKvReservationPhaseV1::Preflight,
                    (scheduled, target, contexts),
                ));
            }
            for (lane, (cache, context)) in scheduled
                .selected
                .iter_mut()
                .zip(contexts.iter().copied())
                .enumerate()
            {
                let request = cache.projection().request;
                match cache.reserve_m1_qualification_context_step_write_v1(
                    request,
                    u32::try_from(lane).unwrap_or(u32::MAX),
                    context,
                    scheduled.scheduled.epoch(),
                ) {
                    Ok(reservation) => reservations.push(reservation.into_pending_step_write()),
                    Err(failure) => {
                        return Err(authenticated_kv_reservation_failure(
                            M1LongLivedQueueRearmKvReservationPhaseV1::TargetReservation,
                            (scheduled, target, contexts, reservations, lane, failure),
                        ));
                    }
                }
            }
            let target_storage = resident_scratch
                .as_deref_mut()
                .and_then(|scratch| scratch.target_table.take());
            let target_result = match target_storage {
                Some(storage) => {
                    crate::kv_workspace_authority::bind_m1_kv_workspace_table_with_storage_v1(
                        target,
                        reservations,
                        storage,
                    )
                }
                None => crate::bind_m1_kv_workspace_table_v1(target, reservations),
            };
            let target = match target_result {
                Ok(table) => table,
                Err(failure) => {
                    return Err(authenticated_kv_reservation_failure(
                        M1LongLivedQueueRearmKvReservationPhaseV1::TargetTableBinding,
                        (scheduled, contexts, failure),
                    ));
                }
            };
            Ok(M1AuthenticatedReservedLongLivedQueueRearmV1 {
                scheduled,
                tables: M1FullStepKvWorkspaceTablesV1::TargetOnly { target },
            })
        }
        M1LongLivedQueueRearmKvInputsV1::SpeculativeRound {
            draft_decode,
            target_speculative,
            mut draft_page_leases,
            mut target_page_leases,
        } => {
            let speculative_shape = matches!(
                scheduled.queue.shape(),
                M1PhysicalFixedBatchShapeV1::SpeculativeK4
                    | M1PhysicalFixedBatchShapeV1::SpeculativeK8
                    | M1PhysicalFixedBatchShapeV1::SpeculativeK16
            );
            let draft_live = usize::try_from(draft_decode.live_lane_count()).ok();
            if !speculative_shape
                || !authenticated_input_roster_matches(&scheduled, &target_speculative)
                || draft_live != Some(scheduled.selected.len())
                || draft_page_leases.len() != scheduled.selected.len()
                || target_page_leases.len() != scheduled.selected.len()
            {
                return Err(authenticated_kv_reservation_failure(
                    M1LongLivedQueueRearmKvReservationPhaseV1::Preflight,
                    (
                        scheduled,
                        draft_decode,
                        target_speculative,
                        draft_page_leases,
                        target_page_leases,
                    ),
                ));
            }
            let draft_selection = draft_decode.selection();
            let target_selection = target_speculative.selection();
            let draft_roster_matches = draft_decode
                .lanes()
                .iter()
                .take(scheduled.selected.len())
                .zip(&scheduled.selected)
                .all(|(plan, cache)| {
                    plan.is_some_and(|plan| {
                        plan.request() == cache.projection().request
                            && plan.completion_epoch() == scheduled.scheduled.epoch()
                    })
                });
            if !draft_roster_matches {
                return Err(authenticated_kv_reservation_failure(
                    M1LongLivedQueueRearmKvReservationPhaseV1::Preflight,
                    (
                        scheduled,
                        draft_decode,
                        target_speculative,
                        draft_page_leases,
                        target_page_leases,
                    ),
                ));
            }
            let (mut draft_reservations, mut target_reservations) =
                resident_scratch.as_deref_mut().map_or_else(
                    || (Vec::new(), Vec::new()),
                    |scratch| {
                        (
                            core::mem::take(&mut scratch.draft_reservations),
                            core::mem::take(&mut scratch.target_reservations),
                        )
                    },
                );
            draft_reservations.clear();
            target_reservations.clear();
            if (draft_reservations.capacity() < scheduled.selected.len()
                && draft_reservations
                    .try_reserve_exact(scheduled.selected.len())
                    .is_err())
                || (target_reservations.capacity() < scheduled.selected.len()
                    && target_reservations
                        .try_reserve_exact(scheduled.selected.len())
                        .is_err())
            {
                return Err(authenticated_kv_reservation_failure(
                    M1LongLivedQueueRearmKvReservationPhaseV1::Preflight,
                    (
                        scheduled,
                        draft_decode,
                        target_speculative,
                        draft_page_leases,
                        target_page_leases,
                    ),
                ));
            }
            for lane in 0..scheduled.selected.len() {
                let request = scheduled.selected[lane].projection().request;
                let leases = core::mem::take(&mut draft_page_leases[lane]);
                match scheduled.selected[lane].reserve_speculative_draft_round_write(
                    request,
                    target_selection,
                    draft_selection,
                    draft_decode.context_lengths()[lane],
                    scheduled.scheduled.epoch(),
                    leases,
                ) {
                    Ok(reservation) => draft_reservations.push(reservation),
                    Err(failure) => {
                        let (error, leases) = (*failure).into_parts();
                        draft_page_leases[lane] = leases;
                        return Err(authenticated_kv_reservation_failure(
                            M1LongLivedQueueRearmKvReservationPhaseV1::DraftReservation,
                            (
                                scheduled,
                                draft_decode,
                                target_speculative,
                                draft_page_leases,
                                target_page_leases,
                                draft_reservations,
                                lane,
                                error,
                            ),
                        ));
                    }
                }
            }
            for lane in 0..scheduled.selected.len() {
                let request = scheduled.selected[lane].projection().request;
                let leases = core::mem::take(&mut target_page_leases[lane]);
                match scheduled.selected[lane].reserve_step_write(
                    request,
                    Qwen3ModelRole::Target8B,
                    target_speculative.context_lengths()[lane],
                    target_speculative.active_lengths()[lane],
                    scheduled.scheduled.epoch(),
                    leases,
                ) {
                    Ok(reservation) => target_reservations.push(reservation),
                    Err(failure) => {
                        let (error, leases) = (*failure).into_parts();
                        target_page_leases[lane] = leases;
                        return Err(authenticated_kv_reservation_failure(
                            M1LongLivedQueueRearmKvReservationPhaseV1::TargetReservation,
                            (
                                scheduled,
                                draft_decode,
                                target_speculative,
                                draft_page_leases,
                                target_page_leases,
                                draft_reservations,
                                target_reservations,
                                lane,
                                error,
                            ),
                        ));
                    }
                }
            }
            let target_storage = resident_scratch
                .as_deref_mut()
                .and_then(|scratch| scratch.target_table.take());
            let target_result = match target_storage {
                Some(storage) => {
                    crate::kv_workspace_authority::bind_m1_kv_workspace_table_with_storage_v1(
                        target_speculative,
                        target_reservations,
                        storage,
                    )
                }
                None => {
                    crate::bind_m1_kv_workspace_table_v1(target_speculative, target_reservations)
                }
            };
            let target = match target_result {
                Ok(table) => table,
                Err(failure) => {
                    return Err(authenticated_kv_reservation_failure(
                        M1LongLivedQueueRearmKvReservationPhaseV1::TargetTableBinding,
                        (
                            scheduled,
                            draft_decode,
                            draft_page_leases,
                            target_page_leases,
                            draft_reservations,
                            failure,
                        ),
                    ));
                }
            };
            let draft_storage = resident_scratch
                .as_mut()
                .and_then(|scratch| scratch.draft_table.take());
            let draft_result = match draft_storage {
                Some(storage) => crate::kv_workspace_authority::bind_m1_speculative_draft_kv_round_workspace_table_with_storage_v1(
                    target_selection,
                    draft_decode,
                    draft_reservations,
                    storage,
                ),
                None => crate::bind_m1_speculative_draft_kv_round_workspace_table_v1(
                    target_selection,
                    draft_decode,
                    draft_reservations,
                ),
            };
            let draft_decode = match draft_result {
                Ok(table) => table,
                Err(failure) => {
                    return Err(authenticated_kv_reservation_failure(
                        M1LongLivedQueueRearmKvReservationPhaseV1::DraftTableBinding,
                        (
                            scheduled,
                            target,
                            draft_page_leases,
                            target_page_leases,
                            failure,
                        ),
                    ));
                }
            };
            Ok(M1AuthenticatedReservedLongLivedQueueRearmV1 {
                scheduled,
                tables: M1FullStepKvWorkspaceTablesV1::SpeculativeRound {
                    draft_decode,
                    target_speculative: target,
                },
            })
        }
    }
}

/// Installs exact authenticated next-round reservations and KV workspace tables.
///
/// # Errors
///
/// Returns opaque terminal custody for any preflight, reservation, or table
/// binding failure after permanently faulting `engine`.
pub fn reserve_m1_authenticated_long_lived_queue_rearm_kv_v1<const C: usize>(
    engine: &mut Engine<C>,
    scheduled: M1AuthenticatedScheduledLongLivedQueueRearmV1,
    inputs: M1LongLivedQueueRearmKvInputsV1,
) -> Result<
    M1AuthenticatedReservedLongLivedQueueRearmV1,
    M1AuthenticatedLongLivedQueueRearmKvReservationFailureV1,
> {
    if engine.is_faulted() {
        return Err(authenticated_kv_reservation_failure(
            M1LongLivedQueueRearmKvReservationPhaseV1::Preflight,
            (scheduled, inputs),
        ));
    }
    match reserve_m1_authenticated_long_lived_queue_rearm_kv_inner_v1(scheduled, inputs, None) {
        Ok(reserved) => Ok(reserved),
        Err(failure) => {
            engine.quarantine_m1_queue_rearm_failure();
            Err(failure)
        }
    }
}

pub(crate) fn reserve_m1_authenticated_long_lived_queue_rearm_kv_resident_v1<const C: usize>(
    engine: &mut Engine<C>,
    scheduled: M1AuthenticatedScheduledLongLivedQueueRearmV1,
    inputs: M1LongLivedQueueRearmKvInputsV1,
    scratch: &mut M1AuthenticatedResidentCompletionScratchV1,
) -> Result<
    M1AuthenticatedReservedLongLivedQueueRearmV1,
    M1AuthenticatedLongLivedQueueRearmKvReservationFailureV1,
> {
    if engine.is_faulted() {
        return Err(authenticated_kv_reservation_failure(
            M1LongLivedQueueRearmKvReservationPhaseV1::Preflight,
            (scheduled, inputs),
        ));
    }
    match reserve_m1_authenticated_long_lived_queue_rearm_kv_inner_v1(
        scheduled,
        inputs,
        Some(scratch),
    ) {
        Ok(reserved) => Ok(reserved),
        Err(failure) => {
            engine.quarantine_m1_queue_rearm_failure();
            Err(failure)
        }
    }
}

#[derive(Debug)]
struct M1AuthenticatedScheduledRemainderV1 {
    queue: M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1,
    selected: Vec<ActiveDeviceKvCache>,
    parked: Vec<ActiveDeviceKvCache>,
    terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
    prior_checked: M1CheckedCompletionOutputV1,
    logical_accepted_counts: Box<[u32]>,
    externally_published_counts: Box<[u32]>,
    release_counts: Box<[M1CompletedKvPageReleaseCountsV1]>,
    completed_members: usize,
    total_released: usize,
    history: M1RearmRoundHistoryV1,
}

impl M1AuthenticatedScheduledRemainderV1 {
    fn from_scheduled(
        scheduled: M1AuthenticatedScheduledLongLivedQueueRearmV1,
    ) -> (M1ScheduledDispatchV1, Self) {
        let M1AuthenticatedScheduledLongLivedQueueRearmV1 {
            queue,
            scheduled,
            selected,
            parked,
            terminal,
            prior_checked,
            logical_accepted_counts,
            externally_published_counts,
            release_counts,
            completed_members,
            total_released,
            history,
        } = scheduled;
        (
            scheduled,
            Self {
                queue,
                selected,
                parked,
                terminal,
                prior_checked,
                logical_accepted_counts,
                externally_published_counts,
                release_counts,
                completed_members,
                total_released,
                history,
            },
        )
    }

    fn close(self, retained: impl fmt::Debug + 'static) -> AuthenticatedSubmissionOpaqueCustodyV1 {
        let Self {
            queue,
            selected,
            parked,
            terminal,
            prior_checked,
            logical_accepted_counts,
            externally_published_counts,
            release_counts,
            completed_members,
            total_released,
            history,
        } = self;
        let (shape, lower, witness, operations, custody) = queue.into_rearm_parts();
        close_prepared_rearm_submission_core(
            lower,
            (
                (
                    shape, witness, operations, custody, selected, parked, terminal,
                ),
                (
                    prior_checked,
                    logical_accepted_counts,
                    externally_published_counts,
                    release_counts,
                    completed_members,
                    total_released,
                    history,
                    retained,
                ),
            ),
        )
    }
}

#[derive(Debug)]
pub(crate) struct M1AuthenticatedDraftCatchupPreparationFailureV1 {
    custody: AuthenticatedSubmissionOpaqueCustodyV1,
}

impl M1AuthenticatedDraftCatchupPreparationFailureV1 {
    pub(crate) fn into_disposition(
        self,
    ) -> crate::authenticated_speculative_executor::M1AuthenticatedSpeculativeFailureDispositionV1
    {
        match self.custody {
            AuthenticatedSubmissionOpaqueCustodyV1::Released(retained) => {
                crate::authenticated_speculative_executor::released_disposition(retained)
            }
            AuthenticatedSubmissionOpaqueCustodyV1::Quarantined(retained) => {
                crate::authenticated_speculative_executor::quarantined_disposition(retained)
            }
        }
    }
}

fn close_draft_catchup_scheduled<const C: usize>(
    engine: &mut Engine<C>,
    scheduled: M1AuthenticatedScheduledLongLivedQueueRearmV1,
    retained: impl fmt::Debug + 'static,
) -> Box<M1AuthenticatedDraftCatchupPreparationFailureV1> {
    engine.quarantine_m1_queue_rearm_failure();
    let (dispatch, remainder) = M1AuthenticatedScheduledRemainderV1::from_scheduled(scheduled);
    Box::new(M1AuthenticatedDraftCatchupPreparationFailureV1 {
        custody: remainder.close((dispatch, retained)),
    })
}

pub(crate) fn prepare_authenticated_draft_catchup_v1<const C: usize>(
    engine: &mut Engine<C>,
    mut scheduled: M1AuthenticatedScheduledLongLivedQueueRearmV1,
    pending: &crate::authenticated_speculative_executor::M1AuthenticatedDraftCatchupPendingV1,
    plans: M1FullStepWorkspacePlans,
    recipe_input: crate::runner::M1PhysicalRunnerRecipeInputV1,
    scratch: &mut M1AuthenticatedDraftCatchupScratchV1,
) -> Result<
    (
        M1AuthenticatedPreparedLongLivedQueueRearmV1,
        AddresslessM1PhysicalBufferRecipeV1,
    ),
    Box<M1AuthenticatedDraftCatchupPreparationFailureV1>,
> {
    let shape = crate::authenticated_prefill_bootstrap::admitted_s1_t128_speculative_successor_v1(
        pending.parent(),
    );
    if engine.is_faulted()
        || scratch.parent != pending.parent()
        || scheduled.selected.len() != 1
        || scheduled.selected[0].projection().request != pending.request()
        || scheduled.scheduled.member_count() != 1
        || scheduled.scheduled.member(0) != Some(pending.request())
        || scheduled.scheduled.epoch() != pending.epoch()
        || scheduled.prior_checked.epoch() != pending.prior_epoch()
        || scheduled.prior_checked.selection() != pending.parent()
        || scheduled.prior_checked.dispatch_generation() != pending.prior_dispatch_generation()
        || shape.is_none_or(|shape| shape.shape() != scheduled.queue.shape())
        || scheduled.queue.detached_dispatch_generation() != pending.prior_dispatch_generation()
        || plans.kind() != M1FullStepWorkspaceInputKind::DraftCatchup
        || scratch.reservations.capacity() < 1
        || !scratch.reservations.is_empty()
        || scratch.draft_inputs.is_none()
        || scratch.completion_inputs.is_none()
        || scratch.page_admission.is_none()
        || scratch.draft_table.is_none()
        || scratch.workspace_images.is_none()
        || scratch.completion_page_indices.is_none()
    {
        return Err(close_draft_catchup_scheduled(
            engine,
            scheduled,
            ("maintenance preparation", plans, recipe_input),
        ));
    }
    let completion_selection =
        M1StepDispatchIntent::DraftCatchup(pending.parent()).completion_selection();
    let draft_selection = Qwen3PlanSelection {
        role: Qwen3ModelRole::Draft06B,
        ..completion_selection
    };
    let draft_inputs = scratch
        .draft_inputs
        .take()
        .expect("checked draft input storage")
        .fill(
            scheduled.queue.operations().runner(),
            draft_selection,
            pending.request(),
            pending.epoch(),
            pending.token(),
            pending.draft_committed(),
        );
    let completion_inputs = scratch
        .completion_inputs
        .take()
        .expect("checked receipt input storage")
        .fill(
            scheduled.queue.operations().runner(),
            completion_selection,
            pending.request(),
            pending.epoch(),
            pending.token(),
            pending.draft_committed(),
        );
    let (Some(draft_inputs), Some(completion_inputs)) = (draft_inputs, completion_inputs) else {
        return Err(close_draft_catchup_scheduled(
            engine,
            scheduled,
            ("maintenance input binding", plans, recipe_input),
        ));
    };
    let recipe = match recipe_input.derive(
        scheduled.queue.operations(),
        M1StepDispatchIntent::DraftCatchup(pending.parent()),
    ) {
        M1PhysicalRunnerRecipeOutcomeV1::Prepared(recipe) => recipe,
        source => {
            return Err(close_draft_catchup_scheduled(
                engine,
                scheduled,
                (source, plans, draft_inputs, completion_inputs),
            ))
        }
    };
    let admission = match scheduled.queue.admit_authenticated_draft_catchup_page_set(
        pending,
        scratch
            .page_admission
            .take()
            .expect("checked page admission storage"),
    ) {
        Ok(admission) => admission,
        source => {
            return Err(close_draft_catchup_scheduled(
                engine,
                scheduled,
                (source, plans, recipe, draft_inputs, completion_inputs),
            ))
        }
    };
    let leases = match scheduled
        .queue
        .commit_authenticated_successor_page_set(admission)
    {
        Ok(leases) => leases,
        source => {
            return Err(close_draft_catchup_scheduled(
                engine,
                scheduled,
                (source, plans, recipe, draft_inputs, completion_inputs),
            ))
        }
    };
    let reservation = match scheduled.selected[0].reserve_authenticated_draft_catchup_write(
        pending,
        leases,
        &mut scratch.write,
    ) {
        Ok(reservation) => reservation,
        source => {
            return Err(close_draft_catchup_scheduled(
                engine,
                scheduled,
                (source, plans, recipe, draft_inputs, completion_inputs),
            ))
        }
    };
    scratch.reservations.push(reservation);
    let table = match crate::kv_workspace_authority::bind_m1_draft_catchup_kv_workspace_table_with_storage_v1(
        draft_inputs, core::mem::take(&mut scratch.reservations), scratch.draft_table.take().expect("checked draft table storage"), pending,
    ) {
        Ok(table) => table,
        source => return Err(close_draft_catchup_scheduled(engine, scheduled, (source, plans, recipe, completion_inputs))),
    };
    let tables = M1FullStepKvWorkspaceTablesV1::DraftCatchup {
        draft: table,
        completion: completion_inputs,
        completion_page_indices: scratch
            .completion_page_indices
            .take()
            .expect("checked inert completion page table"),
        target_allocation_id: scheduled
            .queue
            .custody()
            .partitioned_memory()
            .allocation_id(Qwen3ModelRole::Target8B),
    };
    let (dispatch, remainder) = M1AuthenticatedScheduledRemainderV1::from_scheduled(scheduled);
    let prepared =
        match crate::m1_prepublication::prepare_m1_scheduled_workspace_images_with_storage_v1(
            dispatch,
            remainder.queue.operations().runner(),
            plans,
            tables,
            scratch
                .workspace_images
                .take()
                .expect("checked workspace image storage"),
        ) {
            Ok(prepared) => prepared,
            source => {
                engine.quarantine_m1_queue_rearm_failure();
                return Err(Box::new(M1AuthenticatedDraftCatchupPreparationFailureV1 {
                    custody: remainder.close((source, recipe)),
                }));
            }
        };
    Ok((
        M1AuthenticatedPreparedLongLivedQueueRearmV1 {
            prepared,
            remainder,
        },
        recipe,
    ))
}

/// Stable authenticated workspace-preparation failure class.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedLongLivedQueueRearmPrepareErrorV1 {
    /// The Engine was already permanently faulted before preparation began.
    EngineFaulted,
    /// Workspace-plan, scheduler, KV-table, or image composition rejected.
    Preparation,
}

#[derive(Debug)]
struct AuthenticatedPrepareOpaqueCustodyV1(Box<dyn fmt::Debug>);

/// Terminal authenticated workspace-image preparation custody.
#[must_use = "terminal authenticated preparation custody must remain retained"]
#[derive(Debug)]
pub struct M1AuthenticatedLongLivedQueueRearmPrepareFailureV1 {
    error: M1AuthenticatedLongLivedQueueRearmPrepareErrorV1,
    source: Option<Box<M1PrepareFailureV1>>,
    retained: AuthenticatedPrepareOpaqueCustodyV1,
}

impl M1AuthenticatedLongLivedQueueRearmPrepareFailureV1 {
    #[must_use]
    pub const fn error(&self) -> M1AuthenticatedLongLivedQueueRearmPrepareErrorV1 {
        self.error
    }

    /// Exact shared preparation diagnostic when image preparation was attempted.
    #[must_use]
    pub fn source(&self) -> Option<&M1PrepareFailureV1> {
        self.source.as_deref()
    }

    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        true
    }

    #[must_use]
    pub fn retains_custody(&self) -> bool {
        let _ = &self.retained.0;
        true
    }
}

/// Fresh authenticated scheduler-bound workspace bytes beside detached custody.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedPreparedLongLivedQueueRearmV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1AuthenticatedPreparedLongLivedQueueRearmV1>();
/// ```
#[must_use = "prepared authenticated rearm custody must be rebound and submitted"]
#[derive(Debug)]
pub struct M1AuthenticatedPreparedLongLivedQueueRearmV1 {
    prepared: M1PreparedScheduledWorkspaceImagesV1,
    remainder: M1AuthenticatedScheduledRemainderV1,
}

impl M1AuthenticatedPreparedLongLivedQueueRearmV1 {
    #[must_use]
    pub const fn kind(&self) -> M1FullStepWorkspaceInputKind {
        self.prepared.kind()
    }

    #[must_use]
    pub const fn next_epoch(&self) -> CompletionEpoch {
        self.prepared.step().scheduled_dispatch().epoch()
    }

    #[must_use]
    pub fn selected_requests(&self) -> impl ExactSizeIterator<Item = RequestId> + '_ {
        self.remainder
            .selected
            .iter()
            .map(|cache| cache.projection().request)
    }
}

fn prepare_m1_authenticated_long_lived_queue_rearm_inner_v1(
    reserved: M1AuthenticatedReservedLongLivedQueueRearmV1,
    plans: M1FullStepWorkspacePlans,
    storage: Option<crate::m1_prepublication::M1FullStepWorkspaceImageHostStorageV1>,
) -> Result<
    M1AuthenticatedPreparedLongLivedQueueRearmV1,
    Box<M1AuthenticatedLongLivedQueueRearmPrepareFailureV1>,
> {
    let M1AuthenticatedReservedLongLivedQueueRearmV1 { scheduled, tables } = reserved;
    let M1AuthenticatedScheduledLongLivedQueueRearmV1 {
        queue,
        scheduled,
        selected,
        parked,
        terminal,
        prior_checked,
        logical_accepted_counts,
        externally_published_counts,
        release_counts,
        completed_members,
        total_released,
        history,
    } = scheduled;
    let preparation = match storage {
        Some(storage) => {
            crate::m1_prepublication::prepare_m1_scheduled_workspace_images_with_storage_v1(
                scheduled,
                queue.operations().runner(),
                plans,
                tables,
                storage,
            )
        }
        None => crate::prepare_m1_scheduled_workspace_images_v1(
            scheduled,
            queue.operations().runner(),
            plans,
            tables,
        ),
    };
    let remainder = M1AuthenticatedScheduledRemainderV1 {
        queue,
        selected,
        parked,
        terminal,
        prior_checked,
        logical_accepted_counts,
        externally_published_counts,
        release_counts,
        completed_members,
        total_released,
        history,
    };
    match preparation {
        Ok(prepared) => Ok(M1AuthenticatedPreparedLongLivedQueueRearmV1 {
            prepared,
            remainder,
        }),
        Err(source) => Err(Box::new(
            M1AuthenticatedLongLivedQueueRearmPrepareFailureV1 {
                error: M1AuthenticatedLongLivedQueueRearmPrepareErrorV1::Preparation,
                source: Some(Box::new(source)),
                retained: AuthenticatedPrepareOpaqueCustodyV1(Box::new(remainder)),
            },
        )),
    }
}

/// Prepares authenticated next-round workspace images from exact reserved tables.
///
/// # Errors
///
/// Returns opaque terminal custody and permanently faults `engine` when the
/// Engine was already faulted or shared workspace preparation rejects.
pub fn prepare_m1_authenticated_long_lived_queue_rearm_v1<const C: usize>(
    engine: &mut Engine<C>,
    reserved: M1AuthenticatedReservedLongLivedQueueRearmV1,
    plans: M1FullStepWorkspacePlans,
) -> Result<
    M1AuthenticatedPreparedLongLivedQueueRearmV1,
    Box<M1AuthenticatedLongLivedQueueRearmPrepareFailureV1>,
> {
    if engine.is_faulted() {
        return Err(Box::new(
            M1AuthenticatedLongLivedQueueRearmPrepareFailureV1 {
                error: M1AuthenticatedLongLivedQueueRearmPrepareErrorV1::EngineFaulted,
                source: None,
                retained: AuthenticatedPrepareOpaqueCustodyV1(Box::new((reserved, plans))),
            },
        ));
    }
    match prepare_m1_authenticated_long_lived_queue_rearm_inner_v1(reserved, plans, None) {
        Ok(prepared) => Ok(prepared),
        Err(failure) => {
            engine.quarantine_m1_queue_rearm_failure();
            Err(failure)
        }
    }
}

pub(crate) fn prepare_m1_authenticated_long_lived_queue_rearm_resident_v1<const C: usize>(
    engine: &mut Engine<C>,
    reserved: M1AuthenticatedReservedLongLivedQueueRearmV1,
    plans: M1FullStepWorkspacePlans,
    scratch: &mut M1AuthenticatedResidentCompletionScratchV1,
) -> Result<
    M1AuthenticatedPreparedLongLivedQueueRearmV1,
    Box<M1AuthenticatedLongLivedQueueRearmPrepareFailureV1>,
> {
    if engine.is_faulted() {
        return Err(Box::new(
            M1AuthenticatedLongLivedQueueRearmPrepareFailureV1 {
                error: M1AuthenticatedLongLivedQueueRearmPrepareErrorV1::EngineFaulted,
                source: None,
                retained: AuthenticatedPrepareOpaqueCustodyV1(Box::new((reserved, plans))),
            },
        ));
    }
    let storage = scratch.workspace_images.take();
    match prepare_m1_authenticated_long_lived_queue_rearm_inner_v1(reserved, plans, storage) {
        Ok(prepared) => Ok(prepared),
        Err(failure) => {
            engine.quarantine_m1_queue_rearm_failure();
            Err(failure)
        }
    }
}

/// Stable terminal phase for authenticated same-native-queue publication.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedLongLivedQueueRearmSubmissionPhaseV1 {
    EngineFaulted,
    RebindPreflight,
    WorkspaceContent,
    DraftWorkspaceReplacement,
    TargetWorkspaceReplacement,
    DirectDiagnosticChoiceReplacement,
    SpeculativeDraftChoiceReplacement,
    SpeculativeTargetChoiceReplacement,
    WorkspaceRangeRebinding,
    BoundRowRebuild,
    PacketLowering,
    ShapeJoin,
    QueueBind,
    QueueObservation,
    QueueSubmit,
}

enum AuthenticatedSubmissionOpaqueCustodyV1 {
    Released(Box<dyn fmt::Debug>),
    Quarantined(Box<dyn fmt::Debug>),
}

impl AuthenticatedSubmissionOpaqueCustodyV1 {
    fn retain(self, extra: impl fmt::Debug + 'static) -> Self {
        match self {
            Self::Released(retained) => Self::Released(Box::new((retained, extra))),
            Self::Quarantined(retained) => Self::Quarantined(Box::new((retained, extra))),
        }
    }
}

/// Terminal authenticated rebind or publication custody.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedLongLivedQueueRearmSubmissionFailureV1;
/// fn resubmit(failure: M1AuthenticatedLongLivedQueueRearmSubmissionFailureV1) {
///     let _queue = failure.into_queue();
/// }
/// ```
#[must_use = "terminal authenticated submission custody must remain retained"]
pub struct M1AuthenticatedLongLivedQueueRearmSubmissionFailureV1 {
    phase: M1AuthenticatedLongLivedQueueRearmSubmissionPhaseV1,
    retained: AuthenticatedSubmissionOpaqueCustodyV1,
}

impl M1AuthenticatedLongLivedQueueRearmSubmissionFailureV1 {
    #[must_use]
    pub const fn phase(&self) -> M1AuthenticatedLongLivedQueueRearmSubmissionPhaseV1 {
        self.phase
    }

    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        true
    }

    /// Whether every unpublished queue owner was cleanly destroyed.
    #[must_use]
    pub const fn queue_released(&self) -> bool {
        matches!(
            &self.retained,
            AuthenticatedSubmissionOpaqueCustodyV1::Released(_)
        )
    }

    /// Every returned submission failure permanently faults its Engine.
    #[must_use]
    pub const fn engine_quarantined(&self) -> bool {
        true
    }

    #[must_use]
    pub fn retains_custody(&self) -> bool {
        let _ = match &self.retained {
            AuthenticatedSubmissionOpaqueCustodyV1::Released(retained)
            | AuthenticatedSubmissionOpaqueCustodyV1::Quarantined(retained) => retained,
        };
        true
    }
}

impl fmt::Debug for M1AuthenticatedLongLivedQueueRearmSubmissionFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedLongLivedQueueRearmSubmissionFailureV1")
            .field("phase", &self.phase)
            .field("queue_released", &self.queue_released())
            .field("engine_quarantined", &true)
            .finish_non_exhaustive()
    }
}

fn authenticated_submission_failure(
    phase: M1AuthenticatedLongLivedQueueRearmSubmissionPhaseV1,
    retained: impl fmt::Debug + 'static,
) -> M1AuthenticatedLongLivedQueueRearmSubmissionFailureV1 {
    M1AuthenticatedLongLivedQueueRearmSubmissionFailureV1 {
        phase,
        retained: AuthenticatedSubmissionOpaqueCustodyV1::Quarantined(Box::new(retained)),
    }
}

const fn authenticated_classified_submission_failure(
    phase: M1AuthenticatedLongLivedQueueRearmSubmissionPhaseV1,
    retained: AuthenticatedSubmissionOpaqueCustodyV1,
) -> M1AuthenticatedLongLivedQueueRearmSubmissionFailureV1 {
    M1AuthenticatedLongLivedQueueRearmSubmissionFailureV1 { phase, retained }
}

trait M1PreparedRearmCloseEffectV1: fmt::Debug {
    fn destroy_and_release_effect(self) -> Result<Box<dyn fmt::Debug>, Box<dyn fmt::Debug>>;
}

impl M1PreparedRearmCloseEffectV1 for AuthenticatedServiceQueueUnboundSessionV1 {
    fn destroy_and_release_effect(self) -> Result<Box<dyn fmt::Debug>, Box<dyn fmt::Debug>> {
        self.destroy_and_release()
            .map(|released| Box::new(released) as Box<dyn fmt::Debug>)
            .map_err(|quarantined| Box::new(quarantined) as Box<dyn fmt::Debug>)
    }
}

fn close_prepared_rearm_submission_core<Q, L>(
    queue: Q,
    retained: L,
) -> AuthenticatedSubmissionOpaqueCustodyV1
where
    Q: M1PreparedRearmCloseEffectV1,
    L: fmt::Debug + 'static,
{
    match queue.destroy_and_release_effect() {
        Ok(release) => {
            AuthenticatedSubmissionOpaqueCustodyV1::Released(Box::new((release, retained)))
        }
        Err(quarantined) => {
            AuthenticatedSubmissionOpaqueCustodyV1::Quarantined(Box::new((quarantined, retained)))
        }
    }
}

fn close_prepared_rearm_submission(
    prepared: M1AuthenticatedPreparedLongLivedQueueRearmV1,
    recipe: AddresslessM1PhysicalBufferRecipeV1,
) -> AuthenticatedSubmissionOpaqueCustodyV1 {
    let M1AuthenticatedPreparedLongLivedQueueRearmV1 {
        prepared,
        remainder,
    } = prepared;
    let M1AuthenticatedScheduledRemainderV1 {
        queue,
        selected,
        parked,
        terminal,
        prior_checked,
        logical_accepted_counts,
        externally_published_counts,
        release_counts,
        completed_members,
        total_released,
        history,
    } = remainder;
    let (shape, lower, witness, operations, custody) = queue.into_rearm_parts();
    let retained = (
        (
            shape, witness, operations, custody, prepared, recipe, selected, parked,
        ),
        (
            terminal,
            prior_checked,
            logical_accepted_counts,
            externally_published_counts,
            release_counts,
            completed_members,
            total_released,
            history,
        ),
    );
    close_prepared_rearm_submission_core(lower, retained)
}

pub(crate) fn close_m1_authenticated_prepared_long_lived_queue_rearm_v1<const C: usize>(
    engine: &mut Engine<C>,
    prepared: M1AuthenticatedPreparedLongLivedQueueRearmV1,
    recipe: AddresslessM1PhysicalBufferRecipeV1,
) -> crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1 {
    engine.quarantine_m1_queue_rearm_failure();
    match close_prepared_rearm_submission(prepared, recipe) {
        AuthenticatedSubmissionOpaqueCustodyV1::Released(released) => {
            crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1::Released(
                released,
            )
        }
        AuthenticatedSubmissionOpaqueCustodyV1::Quarantined(quarantined) => {
            crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1::Quarantined(
                quarantined,
            )
        }
    }
}

const fn authenticated_submission_phase(
    phase: M1AuthenticatedQueueRearmTerminalPhaseV1,
) -> M1AuthenticatedLongLivedQueueRearmSubmissionPhaseV1 {
    match phase {
        M1AuthenticatedQueueRearmTerminalPhaseV1::WorkspaceContent => {
            M1AuthenticatedLongLivedQueueRearmSubmissionPhaseV1::WorkspaceContent
        }
        M1AuthenticatedQueueRearmTerminalPhaseV1::DraftWorkspaceReplacement => {
            M1AuthenticatedLongLivedQueueRearmSubmissionPhaseV1::DraftWorkspaceReplacement
        }
        M1AuthenticatedQueueRearmTerminalPhaseV1::TargetWorkspaceReplacement => {
            M1AuthenticatedLongLivedQueueRearmSubmissionPhaseV1::TargetWorkspaceReplacement
        }
        M1AuthenticatedQueueRearmTerminalPhaseV1::DirectDiagnosticChoiceReplacement => {
            M1AuthenticatedLongLivedQueueRearmSubmissionPhaseV1::DirectDiagnosticChoiceReplacement
        }
        M1AuthenticatedQueueRearmTerminalPhaseV1::SpeculativeDraftChoiceReplacement => {
            M1AuthenticatedLongLivedQueueRearmSubmissionPhaseV1::SpeculativeDraftChoiceReplacement
        }
        M1AuthenticatedQueueRearmTerminalPhaseV1::SpeculativeTargetChoiceReplacement => {
            M1AuthenticatedLongLivedQueueRearmSubmissionPhaseV1::SpeculativeTargetChoiceReplacement
        }
        M1AuthenticatedQueueRearmTerminalPhaseV1::WorkspaceRangeRebinding => {
            M1AuthenticatedLongLivedQueueRearmSubmissionPhaseV1::WorkspaceRangeRebinding
        }
        M1AuthenticatedQueueRearmTerminalPhaseV1::BoundRowRebuild => {
            M1AuthenticatedLongLivedQueueRearmSubmissionPhaseV1::BoundRowRebuild
        }
        M1AuthenticatedQueueRearmTerminalPhaseV1::PacketLowering => {
            M1AuthenticatedLongLivedQueueRearmSubmissionPhaseV1::PacketLowering
        }
        M1AuthenticatedQueueRearmTerminalPhaseV1::ShapeJoin => {
            M1AuthenticatedLongLivedQueueRearmSubmissionPhaseV1::ShapeJoin
        }
        M1AuthenticatedQueueRearmTerminalPhaseV1::QueueBind => {
            M1AuthenticatedLongLivedQueueRearmSubmissionPhaseV1::QueueBind
        }
        M1AuthenticatedQueueRearmTerminalPhaseV1::QueueObservation => {
            M1AuthenticatedLongLivedQueueRearmSubmissionPhaseV1::QueueObservation
        }
    }
}

#[derive(Debug)]
struct M1AuthenticatedRearmContinuationCustodyV1 {
    selected: Vec<ActiveDeviceKvCache>,
    parked: Vec<ActiveDeviceKvCache>,
    terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
    previous_epoch: CompletionEpoch,
    prior_checked: M1CheckedCompletionOutputV1,
    logical_accepted_counts: Box<[u32]>,
    externally_published_counts: Box<[u32]>,
    release_counts: Box<[M1CompletedKvPageReleaseCountsV1]>,
    completed_members: usize,
    total_released: usize,
    history: M1RearmRoundHistoryV1,
    rollover: Option<crate::M1QueueRolloverObservationV1>,
    new_window_bridge:
        Option<crate::authenticated_queue_rollover::M1AuthenticatedSpeculativeNewWindowBridgeV1>,
}

/// Published authenticated next generation on the same native queue.
///
/// ```compile_fail
/// use ferric_engine::M1AuthenticatedRearmedPublishedQueueV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1AuthenticatedRearmedPublishedQueueV1>();
/// ```
#[must_use = "published authenticated rearm custody must enter the completion pipeline"]
#[derive(Debug)]
pub struct M1AuthenticatedRearmedPublishedQueueV1 {
    queue: M1AuthenticatedPhysicalPublishedQueueSessionV1,
    carry: M1AuthenticatedRearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

impl M1AuthenticatedRearmedPublishedQueueV1 {
    pub(crate) fn close_in_flight<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1 {
        engine.quarantine_m1_queue_rearm_failure();
        let Self {
            queue,
            carry,
            queue_observation,
            device,
        } = self;
        crate::authenticated_physical_queue::retain_in_queue_closure(
            queue.close_in_flight(),
            (carry, queue_observation, device),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) const fn from_authenticated_rollover(
        queue: M1AuthenticatedPhysicalPublishedQueueSessionV1,
        selected: Vec<ActiveDeviceKvCache>,
        terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
        history: M1RearmRoundHistoryV1,
        previous_epoch: CompletionEpoch,
        prior_checked: M1CheckedCompletionOutputV1,
        logical_accepted_counts: Box<[u32]>,
        externally_published_counts: Box<[u32]>,
        release_counts: Box<[M1CompletedKvPageReleaseCountsV1]>,
        completed_members: usize,
        total_released: usize,
        queue_observation: ComputeAqlQueueObservationV1,
        device: Gfx942DeviceBinding,
        rollover: crate::M1QueueRolloverObservationV1,
    ) -> Self {
        Self {
            queue,
            carry: M1AuthenticatedRearmContinuationCustodyV1 {
                selected,
                parked: Vec::new(),
                terminal,
                previous_epoch,
                prior_checked,
                logical_accepted_counts,
                externally_published_counts,
                release_counts,
                completed_members,
                total_released,
                history,
                rollover: Some(rollover),
                new_window_bridge: None,
            },
            queue_observation,
            device,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) const fn from_authenticated_new_window(
        queue: M1AuthenticatedPhysicalPublishedQueueSessionV1,
        selected: Vec<ActiveDeviceKvCache>,
        terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
        previous_epoch: CompletionEpoch,
        prior_checked: M1CheckedCompletionOutputV1,
        logical_accepted_counts: Box<[u32]>,
        externally_published_counts: Box<[u32]>,
        release_counts: Box<[M1CompletedKvPageReleaseCountsV1]>,
        completed_members: usize,
        total_released: usize,
        history: M1RearmRoundHistoryV1,
        queue_observation: ComputeAqlQueueObservationV1,
        device: Gfx942DeviceBinding,
        rollover: crate::M1QueueRolloverObservationV1,
        new_window_bridge:
            crate::authenticated_queue_rollover::M1AuthenticatedSpeculativeNewWindowBridgeV1,
    ) -> Self {
        Self {
            queue,
            carry: M1AuthenticatedRearmContinuationCustodyV1 {
                selected,
                parked: Vec::new(),
                terminal,
                previous_epoch,
                prior_checked,
                logical_accepted_counts,
                externally_published_counts,
                release_counts,
                completed_members,
                total_released,
                history,
                rollover: Some(rollover),
                new_window_bridge: Some(new_window_bridge),
            },
            queue_observation,
            device,
        }
    }

    /// Exact scheduler authority retained through authenticated publication.
    #[must_use = "scheduler authority remains retained"]
    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        self.queue.scheduled_dispatch()
    }

    #[must_use]
    pub const fn shape(&self) -> M1PhysicalFixedBatchShapeV1 {
        self.queue.shape()
    }

    #[must_use]
    pub const fn queue_observation(&self) -> ComputeAqlQueueObservationV1 {
        self.queue_observation
    }

    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.device
    }

    #[must_use]
    pub fn selected_requests(&self) -> impl ExactSizeIterator<Item = RequestId> + '_ {
        self.carry
            .selected
            .iter()
            .map(|cache| cache.projection().request)
    }

    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.carry.parked.len()
    }

    #[must_use]
    pub const fn terminal_count(&self) -> usize {
        self.carry.terminal.len()
    }

    #[must_use]
    pub const fn previous_epoch(&self) -> CompletionEpoch {
        self.carry.previous_epoch
    }

    pub const fn prior_checked(&self) -> &M1CheckedCompletionOutputV1 {
        &self.carry.prior_checked
    }

    #[must_use]
    pub fn prior_logical_accepted_counts(&self) -> &[u32] {
        &self.carry.logical_accepted_counts
    }

    #[must_use]
    pub fn prior_externally_published_counts(&self) -> &[u32] {
        &self.carry.externally_published_counts
    }

    #[must_use]
    pub fn prior_release_counts(&self) -> &[M1CompletedKvPageReleaseCountsV1] {
        &self.carry.release_counts
    }

    #[must_use]
    pub const fn prior_completed_members(&self) -> usize {
        self.carry.completed_members
    }

    #[must_use]
    pub const fn prior_total_released(&self) -> usize {
        self.carry.total_released
    }

    /// Waits for the exact authenticated rearmed generation.
    ///
    /// # Errors
    ///
    /// Returns terminal authenticated queue-operation custody paired with all
    /// continuation owners after permanently faulting `engine`.
    pub fn wait<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1AuthenticatedRearmedCompletedQueueV1,
        Box<M1AuthenticatedRearmedQueueProgressFailureV1>,
    > {
        let Self {
            queue,
            carry,
            queue_observation,
            device,
        } = self;
        match queue.wait() {
            Ok(queue) => Ok(M1AuthenticatedRearmedCompletedQueueV1 {
                queue,
                carry,
                queue_observation,
                device,
            }),
            Err(source) => {
                engine.quarantine_m1_queue_rearm_failure();
                Err(Box::new(M1AuthenticatedRearmedQueueProgressFailureV1 {
                    phase: M1LongLivedQueueRearmProgressPhaseV1::QueueWait,
                    source,
                    carry,
                    queue_observation,
                    device,
                }))
            }
        }
    }

    /// Waits for this exact authenticated generation until fe2o3 KFD's
    /// monotonic relative millisecond deadline.
    ///
    /// # Errors
    ///
    /// Returns terminal authenticated queue-operation custody paired with all
    /// continuation owners after permanently faulting `engine`.
    pub fn wait_for<const C: usize>(
        self,
        timeout_ms: u32,
        engine: &mut Engine<C>,
    ) -> Result<
        M1AuthenticatedRearmedCompletedQueueV1,
        Box<M1AuthenticatedRearmedQueueProgressFailureV1>,
    > {
        let Self {
            queue,
            carry,
            queue_observation,
            device,
        } = self;
        match queue.wait_for(timeout_ms) {
            Ok(queue) => Ok(M1AuthenticatedRearmedCompletedQueueV1 {
                queue,
                carry,
                queue_observation,
                device,
            }),
            Err(source) => {
                engine.quarantine_m1_queue_rearm_failure();
                Err(Box::new(M1AuthenticatedRearmedQueueProgressFailureV1 {
                    phase: M1LongLivedQueueRearmProgressPhaseV1::QueueWait,
                    source,
                    carry,
                    queue_observation,
                    device,
                }))
            }
        }
    }
}

/// Terminal authenticated queue-operation failure retaining continuation custody.
#[must_use = "terminal authenticated queue progress custody must remain retained"]
#[derive(Debug)]
pub struct M1AuthenticatedRearmedQueueProgressFailureV1 {
    phase: M1LongLivedQueueRearmProgressPhaseV1,
    source: Box<M1AuthenticatedPhysicalQueueOperationFailureV1>,
    carry: M1AuthenticatedRearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

impl M1AuthenticatedRearmedQueueProgressFailureV1 {
    #[must_use]
    pub const fn phase(&self) -> M1LongLivedQueueRearmProgressPhaseV1 {
        self.phase
    }

    pub const fn source(&self) -> &M1AuthenticatedPhysicalQueueOperationFailureV1 {
        &self.source
    }

    /// Addressless terminal state captured by an upstream deadline timeout.
    #[must_use]
    pub fn timeout_observation(&self) -> Option<&Gfx942TimeoutExecutionObservationV1> {
        self.source.timeout_observation()
    }

    #[must_use]
    pub const fn retained_cache_count(&self) -> usize {
        self.carry.selected.len() + self.carry.parked.len()
    }

    #[must_use]
    pub const fn queue_observation(&self) -> ComputeAqlQueueObservationV1 {
        self.queue_observation
    }

    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.device
    }
}

/// Completed authenticated rearmed generation before exact signal recycle.
#[must_use = "completed authenticated rearm custody must recycle or remain retained"]
#[derive(Debug)]
pub struct M1AuthenticatedRearmedCompletedQueueV1 {
    queue: M1AuthenticatedPhysicalCompletedQueueSessionV1,
    carry: M1AuthenticatedRearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

impl M1AuthenticatedRearmedCompletedQueueV1 {
    pub(crate) fn close_completed<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1 {
        engine.quarantine_m1_queue_rearm_failure();
        let Self {
            queue,
            carry,
            queue_observation,
            device,
        } = self;
        crate::authenticated_physical_queue::retain_in_queue_closure(
            queue.close_completed(),
            (carry, queue_observation, device),
        )
    }

    /// Recycles exact completion signals while retaining authenticated custody.
    ///
    /// # Errors
    ///
    /// Returns terminal queue-operation custody and permanently faults `engine`.
    pub fn recycle<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1AuthenticatedRearmedRecycledQueueV1,
        Box<M1AuthenticatedRearmedQueueProgressFailureV1>,
    > {
        let Self {
            queue,
            carry,
            queue_observation,
            device,
        } = self;
        match queue.recycle() {
            Ok(queue) => Ok(M1AuthenticatedRearmedRecycledQueueV1 {
                queue,
                carry,
                queue_observation,
                device,
            }),
            Err(source) => {
                engine.quarantine_m1_queue_rearm_failure();
                Err(Box::new(M1AuthenticatedRearmedQueueProgressFailureV1 {
                    phase: M1LongLivedQueueRearmProgressPhaseV1::SignalRecycle,
                    source,
                    carry,
                    queue_observation,
                    device,
                }))
            }
        }
    }
}

/// Recycled authenticated rearmed queue ready for one exact completion read.
#[must_use = "recycled authenticated rearm custody must observe completion"]
#[derive(Debug)]
pub struct M1AuthenticatedRearmedRecycledQueueV1 {
    queue: M1AuthenticatedPhysicalRecycledQueueSessionV1,
    carry: M1AuthenticatedRearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

/// Physical observation or semantic-join rejection with complete rearm custody.
#[derive(Debug)]
pub enum M1AuthenticatedRearmedReadbackFailureSourceV1 {
    Observation(M1AuthenticatedCompletionObservationFailureV1),
    Join(M1AuthenticatedCompletedReadbackJoinFailureV1),
}

/// Authenticated rearmed readback failure retaining every continuation owner.
#[must_use = "authenticated readback failure custody must be retried or retained"]
#[derive(Debug)]
pub struct M1AuthenticatedRearmedReadbackFailureV1 {
    source: M1AuthenticatedRearmedReadbackFailureSourceV1,
    carry: M1AuthenticatedRearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

impl M1AuthenticatedRearmedReadbackFailureV1 {
    #[must_use]
    pub const fn source(&self) -> &M1AuthenticatedRearmedReadbackFailureSourceV1 {
        &self.source
    }

    #[must_use]
    pub const fn retained_cache_count(&self) -> usize {
        self.carry.selected.len() + self.carry.parked.len()
    }

    /// Retries only a lower observation rejection that still retains recycled custody.
    ///
    /// # Errors
    ///
    /// Returns unchanged phase-accurate failure custody when no retry is admitted
    /// or when the retried observation rejects again.
    pub fn retry_observation(
        self: Box<Self>,
    ) -> Result<M1AuthenticatedRearmedObservedCompletionOutputV1, Box<Self>> {
        let Self {
            source,
            carry,
            queue_observation,
            device,
        } = *self;
        match source {
            M1AuthenticatedRearmedReadbackFailureSourceV1::Observation(source) => {
                match source.retry() {
                    Ok(observed) => Ok(M1AuthenticatedRearmedObservedCompletionOutputV1 {
                        observed,
                        carry,
                        queue_observation,
                        device,
                    }),
                    Err(source) => Err(Box::new(Self {
                        source: M1AuthenticatedRearmedReadbackFailureSourceV1::Observation(*source),
                        carry,
                        queue_observation,
                        device,
                    })),
                }
            }
            source @ M1AuthenticatedRearmedReadbackFailureSourceV1::Join(_) => {
                Err(Box::new(Self {
                    source,
                    carry,
                    queue_observation,
                    device,
                }))
            }
        }
    }

    /// Recovers an already-copied observation after semantic join rejection.
    ///
    /// # Errors
    ///
    /// Returns unchanged failure custody unless the source is a semantic join.
    pub fn recover_observed_after_semantic_rejection(
        self: Box<Self>,
    ) -> Result<M1AuthenticatedRearmedObservedCompletionOutputV1, Box<Self>> {
        let Self {
            source,
            carry,
            queue_observation,
            device,
        } = *self;
        match source {
            M1AuthenticatedRearmedReadbackFailureSourceV1::Join(source) => {
                let (_error, observed) = source.into_parts();
                Ok(M1AuthenticatedRearmedObservedCompletionOutputV1 {
                    observed,
                    carry,
                    queue_observation,
                    device,
                })
            }
            source @ M1AuthenticatedRearmedReadbackFailureSourceV1::Observation(_) => {
                Err(Box::new(Self {
                    source,
                    carry,
                    queue_observation,
                    device,
                }))
            }
        }
    }

    /// Faults the Engine, destroys the authenticated queue, and retains all
    /// copied evidence and rearm continuation custody.
    ///
    /// # Errors
    ///
    /// Returns authenticated lower release quarantine joined to the same
    /// selected, parked, terminal, and prior-round owners.
    pub fn destroy_queue_and_retain_custody<const C: usize>(
        self: Box<Self>,
        engine: &mut Engine<C>,
    ) -> Result<
        M1AuthenticatedRearmedReadbackTeardownSuccessV1,
        Box<M1AuthenticatedRearmedReadbackTeardownFailureV1>,
    > {
        let Self {
            source,
            carry,
            queue_observation,
            device,
        } = *self;
        let teardown = match source {
            M1AuthenticatedRearmedReadbackFailureSourceV1::Observation(source) => {
                source.destroy_queue_and_retain_evidence(engine)
            }
            M1AuthenticatedRearmedReadbackFailureSourceV1::Join(source) => {
                source.destroy_queue_and_retain_evidence(engine)
            }
        };
        match teardown {
            Ok(source) => Ok(M1AuthenticatedRearmedReadbackTeardownSuccessV1 {
                source,
                carry,
                queue_observation,
                device,
            }),
            Err(source) => Err(Box::new(M1AuthenticatedRearmedReadbackTeardownFailureV1 {
                source,
                carry,
                queue_observation,
                device,
            })),
        }
    }
}

/// Clean authenticated readback teardown retaining complete rearm custody.
#[must_use = "authenticated readback teardown custody remains retained"]
#[derive(Debug)]
pub struct M1AuthenticatedRearmedReadbackTeardownSuccessV1 {
    source: crate::M1AuthenticatedReadbackTeardownSuccessV1,
    carry: M1AuthenticatedRearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

/// Terminal authenticated readback release quarantine with complete rearm custody.
#[must_use = "authenticated readback quarantine custody remains retained"]
#[derive(Debug)]
pub struct M1AuthenticatedRearmedReadbackTeardownFailureV1 {
    source: Box<crate::M1AuthenticatedReadbackTeardownFailureV1>,
    carry: M1AuthenticatedRearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

impl M1AuthenticatedRearmedReadbackTeardownSuccessV1 {
    pub const fn source(&self) -> &crate::M1AuthenticatedReadbackTeardownSuccessV1 {
        &self.source
    }

    #[must_use]
    pub const fn retained_cache_count(&self) -> usize {
        self.carry.selected.len() + self.carry.parked.len()
    }

    #[must_use]
    pub const fn terminal_lineage_count(&self) -> usize {
        self.carry.terminal.len()
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.carry.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&crate::M1RearmRoundHistoryEntryV1> {
        self.carry.history.get(index)
    }

    #[must_use]
    pub const fn queue_observation(&self) -> ComputeAqlQueueObservationV1 {
        self.queue_observation
    }

    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.device
    }
}

impl M1AuthenticatedRearmedReadbackTeardownFailureV1 {
    pub const fn source(&self) -> &crate::M1AuthenticatedReadbackTeardownFailureV1 {
        &self.source
    }

    #[must_use]
    pub const fn retained_cache_count(&self) -> usize {
        self.carry.selected.len() + self.carry.parked.len()
    }

    #[must_use]
    pub const fn terminal_lineage_count(&self) -> usize {
        self.carry.terminal.len()
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.carry.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&crate::M1RearmRoundHistoryEntryV1> {
        self.carry.history.get(index)
    }

    #[must_use]
    pub const fn queue_observation(&self) -> ComputeAqlQueueObservationV1 {
        self.queue_observation
    }

    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.device
    }
}

/// Phase-local authenticated diagnostic readback rejection.
#[must_use = "diagnostic readback failure retains queue, cache, and copied evidence custody"]
#[derive(Debug)]
pub enum M1AuthenticatedRearmedSpeculativeDiagnosticReadbackFailureSourceV1 {
    Observation(M1AuthenticatedCompletionObservationFailureV1),
    Choices(Box<crate::M1AuthenticatedSpeculativeDiagnosticObservationFailureV1>),
    Join(Box<crate::M1AuthenticatedSpeculativeDiagnosticCompletedReadbackJoinFailureV1>),
}

/// Authenticated diagnostic failure with complete continuation lineage.
#[must_use = "diagnostic failure custody must be torn down or retained"]
#[derive(Debug)]
pub struct M1AuthenticatedRearmedSpeculativeDiagnosticReadbackFailureV1 {
    source: M1AuthenticatedRearmedSpeculativeDiagnosticReadbackFailureSourceV1,
    carry: M1AuthenticatedRearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

impl M1AuthenticatedRearmedSpeculativeDiagnosticReadbackFailureV1 {
    pub const fn source(
        &self,
    ) -> &M1AuthenticatedRearmedSpeculativeDiagnosticReadbackFailureSourceV1 {
        &self.source
    }

    #[must_use]
    pub const fn retained_cache_count(&self) -> usize {
        self.carry.selected.len() + self.carry.parked.len()
    }

    /// Faults the Engine, destroys the authenticated queue, and retains every
    /// compact or choice copy already made before the rejection.
    ///
    /// # Errors
    ///
    /// Returns the exact lower release quarantine with all continuation owners.
    pub fn destroy_queue_and_retain_custody<const C: usize>(
        self: Box<Self>,
        engine: &mut Engine<C>,
    ) -> Result<
        M1AuthenticatedRearmedSpeculativeDiagnosticReadbackTeardownSuccessV1,
        Box<M1AuthenticatedRearmedSpeculativeDiagnosticReadbackTeardownFailureV1>,
    > {
        let Self {
            source,
            carry,
            queue_observation,
            device,
        } = *self;
        let teardown = match source {
            M1AuthenticatedRearmedSpeculativeDiagnosticReadbackFailureSourceV1::Observation(
                source,
            ) => source
                .destroy_queue_and_retain_evidence(engine)
                .map(
                    M1AuthenticatedRearmedSpeculativeDiagnosticReadbackTeardownSuccessSourceV1::Observation,
                )
                .map_err(
                    M1AuthenticatedRearmedSpeculativeDiagnosticReadbackTeardownFailureSourceV1::Observation,
                ),
            M1AuthenticatedRearmedSpeculativeDiagnosticReadbackFailureSourceV1::Choices(source) => {
                source
                    .destroy_queue_and_retain_evidence(engine)
                    .map(
                        M1AuthenticatedRearmedSpeculativeDiagnosticReadbackTeardownSuccessSourceV1::Choices,
                    )
                    .map_err(
                        M1AuthenticatedRearmedSpeculativeDiagnosticReadbackTeardownFailureSourceV1::Choices,
                    )
            }
            M1AuthenticatedRearmedSpeculativeDiagnosticReadbackFailureSourceV1::Join(source) => {
                source
                    .destroy_queue_and_retain_evidence(engine)
                    .map(|source| {
                        M1AuthenticatedRearmedSpeculativeDiagnosticReadbackTeardownSuccessSourceV1::Join(
                            Box::new(source),
                        )
                    })
                    .map_err(
                        M1AuthenticatedRearmedSpeculativeDiagnosticReadbackTeardownFailureSourceV1::Join,
                    )
            }
        };
        let retained = M1AuthenticatedRearmedSpeculativeDiagnosticRetainedCustodyV1 {
            carry,
            queue_observation,
            device,
        };
        match teardown {
            Ok(source) => Ok(
                M1AuthenticatedRearmedSpeculativeDiagnosticReadbackTeardownSuccessV1 {
                    source,
                    retained,
                },
            ),
            Err(source) => Err(Box::new(
                M1AuthenticatedRearmedSpeculativeDiagnosticReadbackTeardownFailureV1 {
                    source,
                    retained,
                },
            )),
        }
    }
}

/// Lower diagnostic teardown result before rearm lineage is reattached.
#[must_use = "diagnostic teardown evidence remains retained"]
#[derive(Debug)]
pub enum M1AuthenticatedRearmedSpeculativeDiagnosticReadbackTeardownSuccessSourceV1 {
    Observation(crate::M1AuthenticatedReadbackTeardownSuccessV1),
    Choices(crate::M1AuthenticatedSpeculativeK4DiagnosticObservationTeardownSuccessV1),
    Join(Box<crate::M1AuthenticatedSpeculativeK4DiagnosticSemanticTeardownSuccessV1>),
}

/// Lower diagnostic release quarantine before rearm lineage is reattached.
#[must_use = "diagnostic teardown quarantine remains retained"]
#[derive(Debug)]
pub enum M1AuthenticatedRearmedSpeculativeDiagnosticReadbackTeardownFailureSourceV1 {
    Observation(Box<crate::M1AuthenticatedReadbackTeardownFailureV1>),
    Choices(Box<crate::M1AuthenticatedSpeculativeK4DiagnosticObservationTeardownFailureV1>),
    Join(Box<crate::M1AuthenticatedSpeculativeK4DiagnosticSemanticTeardownFailureV1>),
}

/// Continuation custody retained beside a terminal diagnostic teardown.
#[must_use = "diagnostic continuation custody remains retained"]
#[derive(Debug)]
pub struct M1AuthenticatedRearmedSpeculativeDiagnosticRetainedCustodyV1 {
    carry: M1AuthenticatedRearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

impl M1AuthenticatedRearmedSpeculativeDiagnosticRetainedCustodyV1 {
    #[must_use]
    pub const fn retained_cache_count(&self) -> usize {
        self.carry.selected.len() + self.carry.parked.len()
    }

    #[must_use]
    pub const fn queue_observation(&self) -> ComputeAqlQueueObservationV1 {
        self.queue_observation
    }

    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.device
    }
}

/// Clean diagnostic queue release paired with full rearm lineage.
#[must_use = "diagnostic evidence and rearm lineage remain retained"]
#[derive(Debug)]
pub struct M1AuthenticatedRearmedSpeculativeDiagnosticReadbackTeardownSuccessV1 {
    source: M1AuthenticatedRearmedSpeculativeDiagnosticReadbackTeardownSuccessSourceV1,
    retained: M1AuthenticatedRearmedSpeculativeDiagnosticRetainedCustodyV1,
}

/// Diagnostic release quarantine paired with full rearm lineage.
#[must_use = "diagnostic release quarantine and rearm lineage remain retained"]
#[derive(Debug)]
pub struct M1AuthenticatedRearmedSpeculativeDiagnosticReadbackTeardownFailureV1 {
    source: M1AuthenticatedRearmedSpeculativeDiagnosticReadbackTeardownFailureSourceV1,
    retained: M1AuthenticatedRearmedSpeculativeDiagnosticRetainedCustodyV1,
}

impl M1AuthenticatedRearmedSpeculativeDiagnosticReadbackTeardownSuccessV1 {
    pub const fn source(
        &self,
    ) -> &M1AuthenticatedRearmedSpeculativeDiagnosticReadbackTeardownSuccessSourceV1 {
        &self.source
    }

    pub const fn retained(&self) -> &M1AuthenticatedRearmedSpeculativeDiagnosticRetainedCustodyV1 {
        &self.retained
    }
}

impl M1AuthenticatedRearmedSpeculativeDiagnosticReadbackTeardownFailureV1 {
    pub const fn source(
        &self,
    ) -> &M1AuthenticatedRearmedSpeculativeDiagnosticReadbackTeardownFailureSourceV1 {
        &self.source
    }

    pub const fn retained(&self) -> &M1AuthenticatedRearmedSpeculativeDiagnosticRetainedCustodyV1 {
        &self.retained
    }
}

/// Joined authenticated rearm completion with freshly copied choice evidence.
#[must_use = "diagnostic readback and choice evidence must remain retained"]
#[derive(Debug)]
pub struct M1AuthenticatedRearmedSpeculativeDiagnosticCompletedReadbackV1 {
    readback: M1AuthenticatedRearmedCompletedReadbackV1,
    choices: crate::M1ObservedSpeculativeDiagnosticChoicesV1,
}

impl M1AuthenticatedRearmedSpeculativeDiagnosticCompletedReadbackV1 {
    pub const fn checked(&self) -> &M1CheckedCompletionOutputV1 {
        self.readback.checked()
    }

    pub const fn choices(&self) -> &crate::M1ObservedSpeculativeDiagnosticChoicesV1 {
        &self.choices
    }

    /// Separates completion authority from inert diagnostic evidence once.
    #[must_use = "both completion and choice evidence remain retained"]
    pub fn into_parts(
        self,
    ) -> (
        M1AuthenticatedRearmedCompletedReadbackV1,
        crate::M1ObservedSpeculativeDiagnosticChoicesV1,
    ) {
        (self.readback, self.choices)
    }
}

#[derive(Debug)]
enum M1AuthenticatedRearmedResidentReadbackFailureSourceV1 {
    DispatchGeneration {
        queue: M1AuthenticatedPhysicalRecycledQueueSessionV1,
        storage: Box<dyn fmt::Debug>,
    },
    Physical(
        Box<
            crate::authenticated_physical_readback::M1AuthenticatedResidentPhysicalReadbackFailureV1,
        >,
    ),
}

#[derive(Debug)]
pub(crate) struct M1AuthenticatedRearmedResidentPrefillReadbackV1 {
    readback: M1AuthenticatedRearmedCompletedReadbackV1,
    request: RequestId,
    emitted_token: ferric_spec::TokenId,
    selection: Qwen3PlanSelection,
    epoch: CompletionEpoch,
}

impl M1AuthenticatedRearmedResidentPrefillReadbackV1 {
    pub(crate) fn into_parts(
        self,
    ) -> (
        M1AuthenticatedRearmedCompletedReadbackV1,
        RequestId,
        ferric_spec::TokenId,
        Qwen3PlanSelection,
        CompletionEpoch,
    ) {
        (
            self.readback,
            self.request,
            self.emitted_token,
            self.selection,
            self.epoch,
        )
    }
}

#[derive(Debug)]
pub(crate) struct M1AuthenticatedRearmedResidentReadbackFailureV1 {
    source: M1AuthenticatedRearmedResidentReadbackFailureSourceV1,
    carry: M1AuthenticatedRearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

impl M1AuthenticatedRearmedResidentReadbackFailureV1 {
    pub(crate) fn close<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1 {
        let Self {
            source,
            carry,
            queue_observation,
            device,
        } = self;
        let closure = match source {
            M1AuthenticatedRearmedResidentReadbackFailureSourceV1::DispatchGeneration {
                queue,
                storage,
            } => {
                engine.quarantine_m1_queue_rearm_failure();
                crate::authenticated_physical_queue::retain_in_queue_closure(
                    queue.close_recycled(),
                    storage,
                )
            }
            M1AuthenticatedRearmedResidentReadbackFailureSourceV1::Physical(failure) => {
                failure.close(engine)
            }
        };
        crate::authenticated_physical_queue::retain_in_queue_closure(
            closure,
            (carry, queue_observation, device),
        )
    }
}

impl M1AuthenticatedRearmedRecycledQueueV1 {
    pub(crate) fn read_and_check_paired_prefill_completion_with_storage(
        self,
        storage: crate::authenticated_physical_readback::M1AuthenticatedResidentPrefillReadbackHostStorageV1,
    ) -> Result<
        M1AuthenticatedRearmedResidentPrefillReadbackV1,
        Box<M1AuthenticatedRearmedResidentReadbackFailureV1>,
    > {
        let Self {
            queue,
            carry,
            queue_observation,
            device,
        } = self;
        let expected_dispatch_generation = carry.rollover.as_ref().map_or_else(
            || carry.prior_checked.dispatch_generation().checked_add(1),
            |rollover| Some(rollover.replacement_dispatch_generation()),
        );
        let Some(expected_dispatch_generation) = expected_dispatch_generation else {
            return Err(Box::new(M1AuthenticatedRearmedResidentReadbackFailureV1 {
                source: M1AuthenticatedRearmedResidentReadbackFailureSourceV1::DispatchGeneration {
                    queue,
                    storage: Box::new(storage),
                },
                carry,
                queue_observation,
                device,
            }));
        };
        let readback = match queue.read_and_check_paired_prefill_completion_with_storage(
            expected_dispatch_generation,
            storage,
        ) {
            Ok(readback) => readback,
            Err(source) => {
                return Err(Box::new(M1AuthenticatedRearmedResidentReadbackFailureV1 {
                    source: M1AuthenticatedRearmedResidentReadbackFailureSourceV1::Physical(source),
                    carry,
                    queue_observation,
                    device,
                }));
            }
        };
        let (readback, request, emitted_token, selection, epoch) = readback.into_parts();
        Ok(M1AuthenticatedRearmedResidentPrefillReadbackV1 {
            readback: M1AuthenticatedRearmedCompletedReadbackV1 {
                readback,
                carry,
                queue_observation,
                device,
            },
            request,
            emitted_token,
            selection,
            epoch,
        })
    }

    pub(crate) fn read_and_check_speculative_diagnostic_completion_with_storage(
        self,
        storage:
            crate::authenticated_physical_readback::M1AuthenticatedResidentReadbackHostStorageV1,
    ) -> Result<
        M1AuthenticatedRearmedSpeculativeDiagnosticCompletedReadbackV1,
        Box<M1AuthenticatedRearmedResidentReadbackFailureV1>,
    > {
        let Self {
            queue,
            carry,
            queue_observation,
            device,
        } = self;
        let expected_dispatch_generation = carry.rollover.as_ref().map_or_else(
            || carry.prior_checked.dispatch_generation().checked_add(1),
            |rollover| Some(rollover.replacement_dispatch_generation()),
        );
        let Some(expected_dispatch_generation) = expected_dispatch_generation else {
            return Err(Box::new(M1AuthenticatedRearmedResidentReadbackFailureV1 {
                source: M1AuthenticatedRearmedResidentReadbackFailureSourceV1::DispatchGeneration {
                    queue,
                    storage: Box::new(storage),
                },
                carry,
                queue_observation,
                device,
            }));
        };
        let joined = match queue.read_and_check_speculative_diagnostic_completion_with_storage(
            expected_dispatch_generation,
            storage,
        ) {
            Ok(joined) => joined,
            Err(source) => {
                return Err(Box::new(M1AuthenticatedRearmedResidentReadbackFailureV1 {
                    source: M1AuthenticatedRearmedResidentReadbackFailureSourceV1::Physical(source),
                    carry,
                    queue_observation,
                    device,
                }));
            }
        };
        let (readback, choices) = joined.into_parts();
        Ok(
            M1AuthenticatedRearmedSpeculativeDiagnosticCompletedReadbackV1 {
                readback: M1AuthenticatedRearmedCompletedReadbackV1 {
                    readback,
                    carry,
                    queue_observation,
                    device,
                },
                choices,
            },
        )
    }

    pub(crate) fn close_recycled<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1 {
        engine.quarantine_m1_queue_rearm_failure();
        let Self {
            queue,
            carry,
            queue_observation,
            device,
        } = self;
        crate::authenticated_physical_queue::retain_in_queue_closure(
            queue.close_recycled(),
            (carry, queue_observation, device),
        )
    }

    /// Copies and structurally observes the exact authenticated completion once.
    ///
    /// # Errors
    ///
    /// Returns the phase-accurate authenticated observation failure paired with
    /// every selected, parked, terminal, and prior-step owner.
    pub fn observe_completion(
        self,
    ) -> Result<
        M1AuthenticatedRearmedObservedCompletionOutputV1,
        Box<M1AuthenticatedRearmedReadbackFailureV1>,
    > {
        let Self {
            queue,
            carry,
            queue_observation,
            device,
        } = self;
        match queue.observe_completion() {
            Ok(observed) => Ok(M1AuthenticatedRearmedObservedCompletionOutputV1 {
                observed,
                carry,
                queue_observation,
                device,
            }),
            Err(source) => Err(Box::new(M1AuthenticatedRearmedReadbackFailureV1 {
                source: M1AuthenticatedRearmedReadbackFailureSourceV1::Observation(source),
                carry,
                queue_observation,
                device,
            })),
        }
    }

    /// Observes and semantically checks the authenticated completion once.
    ///
    /// # Errors
    ///
    /// Returns retryable pre-copy or recoverable post-copy custody without ever
    /// issuing a second completed read after a successful copy.
    pub fn read_and_check_completion(
        self,
        expectations: &[crate::CompletionWireSemanticExpectation<'_>],
    ) -> Result<
        M1AuthenticatedRearmedCompletedReadbackV1,
        Box<M1AuthenticatedRearmedReadbackFailureV1>,
    > {
        self.observe_completion()?.check_completion(expectations)
    }

    /// Copies, independently observes, and semantically joins one authenticated
    /// finite speculative rearm generation.
    ///
    /// This route accepts no caller-supplied token semantics. The compact K7
    /// output and all `K + 1` choice ranges must come from the same authenticated
    /// queue generation.
    ///
    /// # Errors
    ///
    /// Returns exact continuation custody beside compact-copy, choice-copy, or
    /// semantic-join failure.
    pub fn read_and_check_speculative_diagnostic_completion(
        self,
    ) -> Result<
        M1AuthenticatedRearmedSpeculativeDiagnosticCompletedReadbackV1,
        Box<M1AuthenticatedRearmedSpeculativeDiagnosticReadbackFailureV1>,
    > {
        let Self {
            queue,
            carry,
            queue_observation,
            device,
        } = self;
        let observed = match queue.observe_completion() {
            Ok(observed) => observed,
            Err(source) => {
                return Err(Box::new(
                    M1AuthenticatedRearmedSpeculativeDiagnosticReadbackFailureV1 {
                        source: M1AuthenticatedRearmedSpeculativeDiagnosticReadbackFailureSourceV1::Observation(source),
                        carry,
                        queue_observation,
                        device,
                    },
                ));
            }
        };
        let diagnostic = match observed.observe_speculative_diagnostic_choices() {
            Ok(diagnostic) => diagnostic,
            Err(source) => {
                return Err(Box::new(
                    M1AuthenticatedRearmedSpeculativeDiagnosticReadbackFailureV1 {
                        source: M1AuthenticatedRearmedSpeculativeDiagnosticReadbackFailureSourceV1::Choices(source),
                        carry,
                        queue_observation,
                        device,
                    },
                ));
            }
        };
        let joined = match diagnostic.check_completion() {
            Ok(joined) => joined,
            Err(source) => {
                return Err(Box::new(
                    M1AuthenticatedRearmedSpeculativeDiagnosticReadbackFailureV1 {
                        source:
                            M1AuthenticatedRearmedSpeculativeDiagnosticReadbackFailureSourceV1::Join(
                                source,
                            ),
                        carry,
                        queue_observation,
                        device,
                    },
                ));
            }
        };
        let (readback, choices) = joined.into_parts();
        Ok(
            M1AuthenticatedRearmedSpeculativeDiagnosticCompletedReadbackV1 {
                readback: M1AuthenticatedRearmedCompletedReadbackV1 {
                    readback,
                    carry,
                    queue_observation,
                    device,
                },
                choices,
            },
        )
    }
}

/// Move-only authenticated structural observation with complete rearm custody.
#[must_use = "observed authenticated completion must be checked or retained"]
#[derive(Debug)]
pub struct M1AuthenticatedRearmedObservedCompletionOutputV1 {
    observed: M1AuthenticatedObservedCompletionOutputV1,
    carry: M1AuthenticatedRearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

impl M1AuthenticatedRearmedObservedCompletionOutputV1 {
    #[must_use = "the inert observed image remains paired with authenticated custody"]
    pub const fn image(&self) -> &M1ObservedCompletionImageV1 {
        self.observed.image()
    }

    #[must_use]
    pub fn selected_requests(&self) -> impl ExactSizeIterator<Item = RequestId> + '_ {
        self.carry
            .selected
            .iter()
            .map(|cache| cache.projection().request)
    }

    /// Joins exact semantic expectations without another physical read.
    ///
    /// # Errors
    ///
    /// Returns the unchanged copied observation and continuation custody when
    /// roster, plan, epoch, wire, or token semantics reject.
    pub fn check_completion(
        self,
        expectations: &[crate::CompletionWireSemanticExpectation<'_>],
    ) -> Result<
        M1AuthenticatedRearmedCompletedReadbackV1,
        Box<M1AuthenticatedRearmedReadbackFailureV1>,
    > {
        let Self {
            observed,
            carry,
            queue_observation,
            device,
        } = self;
        match observed.check_completion(expectations) {
            Ok(readback) => Ok(M1AuthenticatedRearmedCompletedReadbackV1 {
                readback,
                carry,
                queue_observation,
                device,
            }),
            Err(source) => Err(Box::new(M1AuthenticatedRearmedReadbackFailureV1 {
                source: M1AuthenticatedRearmedReadbackFailureSourceV1::Join(source),
                carry,
                queue_observation,
                device,
            })),
        }
    }
}

/// Joined authenticated readback paired with all rearm continuation custody.
#[must_use = "joined authenticated rearm readback must settle KV custody"]
#[derive(Debug)]
pub struct M1AuthenticatedRearmedCompletedReadbackV1 {
    readback: M1AuthenticatedPhysicalCompletedReadbackV1,
    carry: M1AuthenticatedRearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

impl M1AuthenticatedRearmedCompletedReadbackV1 {
    pub const fn checked(&self) -> &M1CheckedCompletionOutputV1 {
        self.readback.checked()
    }

    #[must_use]
    pub fn selected_requests(&self) -> impl ExactSizeIterator<Item = RequestId> + '_ {
        self.carry
            .selected
            .iter()
            .map(|cache| cache.projection().request)
    }

    /// Destroys an authenticated queue that cannot proceed to physical
    /// settlement while retaining the checked output, exact completion, KV,
    /// and complete rearm lineage.
    ///
    /// # Errors
    ///
    /// Returns exact queue-release quarantine with the same checked readback,
    /// KV reservations, and continuation lineage.
    pub fn destroy_queue_and_retain_custody<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1AuthenticatedRearmedCompletedReadbackTeardownSuccessV1,
        Box<M1AuthenticatedRearmedCompletedReadbackTeardownFailureV1>,
    > {
        engine.quarantine_m1_queue_rearm_failure();
        let Self {
            readback,
            carry,
            queue_observation,
            device,
        } = self;
        let (queue, checked, completion, kv) = readback.into_parts();
        let custody = M1AuthenticatedRearmedCompletedReadbackTeardownCustodyV1 {
            checked,
            completion,
            kv,
            carry,
            queue_observation,
            device,
        };
        match queue.destroy_and_release() {
            Ok(queue_release) => Ok(M1AuthenticatedRearmedCompletedReadbackTeardownSuccessV1 {
                queue_release,
                custody,
            }),
            Err(source) => Err(Box::new(
                M1AuthenticatedRearmedCompletedReadbackTeardownFailureV1 { source, custody },
            )),
        }
    }

    /// Settles the authenticated generation using selected caches in scheduler order.
    ///
    /// # Errors
    ///
    /// Returns unchanged readback/cache custody and all supplied dispositions
    /// for a count mismatch or host reservation failure.
    pub fn complete<const C: usize>(
        self,
        engine: &mut Engine<C>,
        dispositions: Vec<M1DeviceKvCompletionDispositionV1>,
    ) -> Result<
        M1AuthenticatedRearmedCompletionOutcomeV1,
        M1AuthenticatedRearmedCompletionPreflightFailureV1,
    > {
        self.complete_inner(engine, dispositions, None)
    }

    pub(crate) fn complete_resident<const C: usize>(
        self,
        engine: &mut Engine<C>,
        dispositions: M1AuthenticatedRearmedDispositionsV1,
        mut scratch: M1AuthenticatedResidentCompletionScratchV1,
    ) -> Result<
        M1AuthenticatedRearmedCompletionOutcomeV1,
        M1AuthenticatedRearmedCompletionPreflightFailureV1,
    > {
        scratch.dispositions.clear();
        if scratch.dispositions.capacity() < dispositions.len() {
            let retained = dispositions.into_iter().collect();
            return Err(M1AuthenticatedRearmedCompletionPreflightFailureV1 {
                error: M1AuthenticatedRearmedCompletionPreflightErrorV1::HostAllocation,
                readback: Box::new(self),
                dispositions: retained,
            });
        }
        scratch.dispositions.extend(dispositions);
        let dispositions = core::mem::take(&mut scratch.dispositions);
        self.complete_inner(engine, dispositions, Some(scratch))
    }

    fn complete_inner<const C: usize>(
        mut self,
        engine: &mut Engine<C>,
        dispositions: Vec<M1DeviceKvCompletionDispositionV1>,
        scratch: Option<M1AuthenticatedResidentCompletionScratchV1>,
    ) -> Result<
        M1AuthenticatedRearmedCompletionOutcomeV1,
        M1AuthenticatedRearmedCompletionPreflightFailureV1,
    > {
        if dispositions.len() != self.carry.selected.len() {
            return Err(M1AuthenticatedRearmedCompletionPreflightFailureV1 {
                error: M1AuthenticatedRearmedCompletionPreflightErrorV1::DispositionCount {
                    expected: self.carry.selected.len(),
                    actual: dispositions.len(),
                },
                readback: Box::new(self),
                dispositions,
            });
        }
        if self.carry.history.len() >= crate::M1_MAX_REARM_ROUND_HISTORY_V1 {
            return Err(M1AuthenticatedRearmedCompletionPreflightFailureV1 {
                error: M1AuthenticatedRearmedCompletionPreflightErrorV1::RoundHistoryCapacity {
                    maximum: crate::M1_MAX_REARM_ROUND_HISTORY_V1,
                },
                readback: Box::new(self),
                dispositions,
            });
        }
        if self.carry.history.try_reserve_append().is_err() {
            return Err(M1AuthenticatedRearmedCompletionPreflightFailureV1 {
                error: M1AuthenticatedRearmedCompletionPreflightErrorV1::HostAllocation,
                readback: Box::new(self),
                dispositions,
            });
        }
        let (mut members, completion_scratch, release_scratch) = match scratch {
            Some(scratch) => (
                scratch.members,
                Some(scratch.completion),
                Some(scratch.release),
            ),
            None => (Vec::new(), None, None),
        };
        if members.capacity() < self.carry.selected.len()
            && members
                .try_reserve_exact(self.carry.selected.len())
                .is_err()
        {
            return Err(M1AuthenticatedRearmedCompletionPreflightFailureV1 {
                error: M1AuthenticatedRearmedCompletionPreflightErrorV1::HostAllocation,
                readback: Box::new(self),
                dispositions,
            });
        }
        let Self {
            readback,
            carry,
            queue_observation,
            device,
        } = self;
        for (cache, disposition) in carry.selected.into_iter().zip(dispositions) {
            members.push(match disposition {
                M1DeviceKvCompletionDispositionV1::Continue => {
                    M1DeviceKvCompletionMemberV1::continuing(cache)
                }
                M1DeviceKvCompletionDispositionV1::Retire => {
                    M1DeviceKvCompletionMemberV1::retiring(cache)
                }
            });
        }
        let roster = M1DeviceKvCompletionRosterV1::new(members);
        let outcome = match completion_scratch {
            Some(scratch) => {
                crate::m1_completed_step::complete_m1_authenticated_physical_step_with_scratch_v1(
                    engine, readback, roster, scratch,
                )
            }
            None => crate::complete_m1_authenticated_physical_step_v1(engine, readback, roster),
        };
        let history_entry = match carry.rollover {
            Some(rollover) => crate::M1RearmRoundHistoryEntryV1::from_queue_transition(
                carry.prior_checked,
                carry.logical_accepted_counts,
                carry.externally_published_counts,
                carry.release_counts,
                carry.completed_members,
                carry.total_released,
                queue_observation,
                device,
                Some(rollover),
            ),
            None => crate::M1RearmRoundHistoryEntryV1::from_same_native_queue(
                carry.prior_checked,
                carry.logical_accepted_counts,
                carry.externally_published_counts,
                carry.release_counts,
                carry.completed_members,
                carry.total_released,
                queue_observation,
                device,
            ),
        };
        let history = carry.history.append(history_entry);
        Ok(M1AuthenticatedRearmedCompletionOutcomeV1 {
            outcome,
            release_scratch,
            lineage: M1AuthenticatedRearmPriorRoundCustodyV1 {
                parked: carry.parked,
                terminal: carry.terminal,
                previous_epoch: carry.previous_epoch,
                queue_observation,
                device,
                history,
                new_window_bridge: carry.new_window_bridge,
            },
        })
    }
}

#[derive(Debug)]
struct M1AuthenticatedRearmedCompletedReadbackTeardownCustodyV1 {
    checked: M1CheckedCompletionOutputV1,
    completion: ExactCompletion,
    kv: M1FullStepKvReservationCustodyV1,
    carry: M1AuthenticatedRearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

/// Clean queue release after a joined readback could not be settled.
#[must_use = "checked readback and queue release remain retained"]
#[derive(Debug)]
pub struct M1AuthenticatedRearmedCompletedReadbackTeardownSuccessV1 {
    queue_release: AuthenticatedServiceQueueReleaseV1,
    custody: M1AuthenticatedRearmedCompletedReadbackTeardownCustodyV1,
}

/// Release quarantine after a joined readback could not be settled.
#[must_use = "checked readback and release quarantine remain retained"]
#[derive(Debug)]
pub struct M1AuthenticatedRearmedCompletedReadbackTeardownFailureV1 {
    source: Box<crate::M1AuthenticatedPhysicalPostReadbackQueueReleaseFailureV1>,
    custody: M1AuthenticatedRearmedCompletedReadbackTeardownCustodyV1,
}

impl M1AuthenticatedRearmedCompletedReadbackTeardownSuccessV1 {
    pub const fn queue_release(&self) -> &AuthenticatedServiceQueueReleaseV1 {
        &self.queue_release
    }

    pub const fn checked(&self) -> &M1CheckedCompletionOutputV1 {
        &self.custody.checked
    }

    #[must_use]
    pub const fn retained_cache_count(&self) -> usize {
        self.custody.carry.selected.len() + self.custody.carry.parked.len()
    }
}

impl M1AuthenticatedRearmedCompletedReadbackTeardownFailureV1 {
    pub const fn source(&self) -> &crate::M1AuthenticatedPhysicalPostReadbackQueueReleaseFailureV1 {
        &self.source
    }

    pub const fn checked(&self) -> &M1CheckedCompletionOutputV1 {
        &self.custody.checked
    }

    #[must_use]
    pub const fn retained_cache_count(&self) -> usize {
        self.custody.carry.selected.len() + self.custody.carry.parked.len()
    }
}

/// Pure authenticated completion preflight rejection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedRearmedCompletionPreflightErrorV1 {
    DispositionCount { expected: usize, actual: usize },
    RoundHistoryCapacity { maximum: usize },
    HostAllocation,
}

/// Retry-capable completion preflight failure retaining every exact input.
#[must_use = "authenticated completion preflight inputs remain retry-capable"]
#[derive(Debug)]
pub struct M1AuthenticatedRearmedCompletionPreflightFailureV1 {
    error: M1AuthenticatedRearmedCompletionPreflightErrorV1,
    readback: Box<M1AuthenticatedRearmedCompletedReadbackV1>,
    dispositions: Vec<M1DeviceKvCompletionDispositionV1>,
}

impl M1AuthenticatedRearmedCompletionPreflightFailureV1 {
    #[must_use]
    pub const fn error(&self) -> M1AuthenticatedRearmedCompletionPreflightErrorV1 {
        self.error
    }

    #[must_use]
    pub fn dispositions(&self) -> &[M1DeviceKvCompletionDispositionV1] {
        &self.dispositions
    }

    /// Recovers unchanged readback and disposition custody.
    #[must_use = "readback and dispositions remain the sole retry inputs"]
    pub fn into_parts(
        self,
    ) -> (
        M1AuthenticatedRearmedCompletionPreflightErrorV1,
        M1AuthenticatedRearmedCompletedReadbackV1,
        Vec<M1DeviceKvCompletionDispositionV1>,
    ) {
        (self.error, *self.readback, self.dispositions)
    }

    /// Retries the same completion preflight and settlement transition.
    ///
    /// # Errors
    ///
    /// Returns fresh unchanged preflight custody if validation still rejects.
    pub fn retry<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1AuthenticatedRearmedCompletionOutcomeV1,
        M1AuthenticatedRearmedCompletionPreflightFailureV1,
    > {
        (*self.readback).complete(engine, self.dispositions)
    }

    /// Faults the Engine, destroys the authenticated queue, and retains the
    /// rejected preflight diagnostic, dispositions, checked readback, KV, and
    /// complete rearm lineage.
    ///
    /// # Errors
    ///
    /// Returns terminal authenticated lower release quarantine joined to the
    /// same completion-preflight custody.
    pub fn destroy_queue_and_retain_custody<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1AuthenticatedRearmedCompletionPreflightTeardownSuccessV1,
        Box<M1AuthenticatedRearmedCompletionPreflightTeardownFailureV1>,
    > {
        engine.quarantine_m1_queue_rearm_failure();
        let Self {
            error,
            readback,
            dispositions,
        } = self;
        let M1AuthenticatedRearmedCompletedReadbackV1 {
            readback,
            carry,
            queue_observation,
            device,
        } = *readback;
        let (queue, checked, completion, kv) = readback.into_parts();
        let custody = M1AuthenticatedRearmedCompletionPreflightTeardownCustodyV1 {
            error,
            checked,
            completion,
            kv,
            dispositions,
            carry,
            queue_observation,
            device,
        };
        match queue.destroy_and_release() {
            Ok(queue_release) => Ok(M1AuthenticatedRearmedCompletionPreflightTeardownSuccessV1 {
                queue_release,
                custody,
            }),
            Err(source) => Err(Box::new(
                M1AuthenticatedRearmedCompletionPreflightTeardownFailureV1 { source, custody },
            )),
        }
    }
}

#[derive(Debug)]
struct M1AuthenticatedRearmedCompletionPreflightTeardownCustodyV1 {
    error: M1AuthenticatedRearmedCompletionPreflightErrorV1,
    checked: M1CheckedCompletionOutputV1,
    completion: ExactCompletion,
    kv: M1FullStepKvReservationCustodyV1,
    dispositions: Vec<M1DeviceKvCompletionDispositionV1>,
    carry: M1AuthenticatedRearmContinuationCustodyV1,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
}

/// Clean authenticated queue teardown retaining completion-preflight custody.
#[must_use = "authenticated completion-preflight teardown remains retained"]
#[derive(Debug)]
pub struct M1AuthenticatedRearmedCompletionPreflightTeardownSuccessV1 {
    queue_release: AuthenticatedServiceQueueReleaseV1,
    custody: M1AuthenticatedRearmedCompletionPreflightTeardownCustodyV1,
}

/// Terminal release quarantine retaining authenticated completion-preflight custody.
#[must_use = "authenticated completion-preflight quarantine remains retained"]
#[derive(Debug)]
pub struct M1AuthenticatedRearmedCompletionPreflightTeardownFailureV1 {
    source: Box<crate::M1AuthenticatedPhysicalPostReadbackQueueReleaseFailureV1>,
    custody: M1AuthenticatedRearmedCompletionPreflightTeardownCustodyV1,
}

impl M1AuthenticatedRearmedCompletionPreflightTeardownSuccessV1 {
    #[must_use]
    pub const fn error(&self) -> M1AuthenticatedRearmedCompletionPreflightErrorV1 {
        self.custody.error
    }

    pub const fn checked(&self) -> &M1CheckedCompletionOutputV1 {
        &self.custody.checked
    }

    #[must_use]
    pub fn dispositions(&self) -> &[M1DeviceKvCompletionDispositionV1] {
        &self.custody.dispositions
    }

    #[must_use]
    pub const fn retained_cache_count(&self) -> usize {
        self.custody.carry.selected.len() + self.custody.carry.parked.len()
    }

    #[must_use]
    pub const fn terminal_lineage_count(&self) -> usize {
        self.custody.carry.terminal.len()
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.custody.carry.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&crate::M1RearmRoundHistoryEntryV1> {
        self.custody.carry.history.get(index)
    }

    #[must_use]
    pub const fn queue_observation(&self) -> ComputeAqlQueueObservationV1 {
        self.custody.queue_observation
    }

    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.custody.device
    }

    #[must_use]
    pub const fn completion_epoch(&self) -> CompletionEpoch {
        self.custody.completion.epoch()
    }

    pub const fn kv_reservations(&self) -> &M1FullStepKvReservationCustodyV1 {
        &self.custody.kv
    }
}

impl M1AuthenticatedRearmedCompletionPreflightTeardownFailureV1 {
    #[must_use]
    pub const fn error(&self) -> M1AuthenticatedRearmedCompletionPreflightErrorV1 {
        self.custody.error
    }

    pub const fn checked(&self) -> &M1CheckedCompletionOutputV1 {
        &self.custody.checked
    }

    #[must_use]
    pub fn dispositions(&self) -> &[M1DeviceKvCompletionDispositionV1] {
        &self.custody.dispositions
    }

    #[must_use]
    pub const fn retained_cache_count(&self) -> usize {
        self.custody.carry.selected.len() + self.custody.carry.parked.len()
    }

    #[must_use]
    pub const fn terminal_lineage_count(&self) -> usize {
        self.custody.carry.terminal.len()
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.custody.carry.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&crate::M1RearmRoundHistoryEntryV1> {
        self.custody.carry.history.get(index)
    }

    #[must_use]
    pub const fn queue_observation(&self) -> ComputeAqlQueueObservationV1 {
        self.custody.queue_observation
    }

    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.custody.device
    }

    #[must_use]
    pub const fn completion_epoch(&self) -> CompletionEpoch {
        self.custody.completion.epoch()
    }

    pub const fn kv_reservations(&self) -> &M1FullStepKvReservationCustodyV1 {
        &self.custody.kv
    }
}

impl M1AuthenticatedRearmedCompletionPreflightTeardownSuccessV1 {
    pub const fn queue_release(&self) -> &AuthenticatedServiceQueueReleaseV1 {
        &self.queue_release
    }
}

impl M1AuthenticatedRearmedCompletionPreflightTeardownFailureV1 {
    pub const fn source(&self) -> &crate::M1AuthenticatedPhysicalPostReadbackQueueReleaseFailureV1 {
        &self.source
    }
}

#[derive(Debug)]
struct M1AuthenticatedRearmPriorRoundCustodyV1 {
    parked: Vec<ActiveDeviceKvCache>,
    terminal: Vec<M1ReleasedTerminalDeviceKvMemberV1>,
    previous_epoch: CompletionEpoch,
    queue_observation: ComputeAqlQueueObservationV1,
    device: Gfx942DeviceBinding,
    history: M1NonEmptyRearmRoundHistoryV1,
    new_window_bridge:
        Option<crate::authenticated_queue_rollover::M1AuthenticatedSpeculativeNewWindowBridgeV1>,
}

/// Authenticated completion outcome paired with all predecessor lineage.
#[must_use = "authenticated completion outcome and lineage must be handled together"]
#[derive(Debug)]
pub struct M1AuthenticatedRearmedCompletionOutcomeV1 {
    outcome: M1AuthenticatedCompletedStepOutcomeV1,
    release_scratch: Option<crate::m1_completed_step_release::M1CompletedStepKvReleaseScratchV1>,
    lineage: M1AuthenticatedRearmPriorRoundCustodyV1,
}

impl M1AuthenticatedRearmedCompletionOutcomeV1 {
    #[must_use = "authenticated completion outcome remains linear"]
    pub const fn outcome(&self) -> &M1AuthenticatedCompletedStepOutcomeV1 {
        &self.outcome
    }

    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.lineage.parked.len()
    }

    #[must_use]
    pub const fn terminal_lineage_count(&self) -> usize {
        self.lineage.terminal.len()
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.lineage.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&crate::M1RearmRoundHistoryEntryV1> {
        self.lineage.history.get(index)
    }

    /// Retries only an unchanged preflight-rejected physical completion.
    ///
    /// # Errors
    ///
    /// Returns this owner unchanged when completion already succeeded or became
    /// terminal poison.
    pub fn retry_rejected<const C: usize>(self, engine: &mut Engine<C>) -> Result<Self, Box<Self>> {
        let Self {
            outcome,
            release_scratch,
            lineage,
        } = self;
        let M1AuthenticatedCompletedStepOutcomeV1::Rejected(rejected) = outcome else {
            return Err(Box::new(Self {
                outcome,
                release_scratch,
                lineage,
            }));
        };
        let (_error, readback, roster) = rejected.into_parts();
        Ok(Self {
            outcome: crate::complete_m1_authenticated_physical_step_v1(engine, readback, roster),
            release_scratch,
            lineage,
        })
    }

    /// Destroys an unchanged rejected completion while retaining all lineage.
    ///
    /// # Errors
    ///
    /// Returns this owner unchanged unless the physical outcome is retryably
    /// rejected. The inner result distinguishes clean teardown from terminal
    /// authenticated queue-release quarantine.
    pub fn destroy_queue_and_retain_rejected<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        Result<
            M1AuthenticatedRearmedRejectedCompletionTeardownSuccessV1,
            Box<M1AuthenticatedRearmedRejectedCompletionTeardownFailureV1>,
        >,
        Box<Self>,
    > {
        let Self {
            outcome,
            release_scratch,
            lineage,
        } = self;
        let M1AuthenticatedCompletedStepOutcomeV1::Rejected(rejected) = outcome else {
            return Err(Box::new(Self {
                outcome,
                release_scratch,
                lineage,
            }));
        };
        Ok(match rejected.destroy_queue_and_retain_rejection(engine) {
            Ok(source) => {
                Ok(M1AuthenticatedRearmedRejectedCompletionTeardownSuccessV1 { source, lineage })
            }
            Err(source) => Err(Box::new(
                M1AuthenticatedRearmedRejectedCompletionTeardownFailureV1 { source, lineage },
            )),
        })
    }

    /// Extracts terminal physical completion poison with complete lineage.
    ///
    /// # Errors
    ///
    /// Returns this owner unchanged unless its physical outcome is poisoned.
    pub fn into_terminal_poison(
        self,
    ) -> Result<M1AuthenticatedRearmedPoisonedCompletionV1, Box<Self>> {
        let Self {
            outcome,
            release_scratch,
            lineage,
        } = self;
        let M1AuthenticatedCompletedStepOutcomeV1::Poisoned(poison) = outcome else {
            return Err(Box::new(Self {
                outcome,
                release_scratch,
                lineage,
            }));
        };
        Ok(M1AuthenticatedRearmedPoisonedCompletionV1 {
            poison: *poison,
            lineage,
        })
    }

    /// Releases retired KV pages only after successful authenticated completion.
    #[must_use = "release outcome retains every authenticated queue and cache owner"]
    pub fn release_completed(self) -> M1AuthenticatedRearmedRoundReleaseOutcomeV1 {
        let Self {
            outcome,
            release_scratch,
            lineage,
        } = self;
        let M1AuthenticatedCompletedStepOutcomeV1::Completed(completed) = outcome else {
            return M1AuthenticatedRearmedRoundReleaseOutcomeV1::NotCompleted(Self {
                outcome,
                release_scratch,
                lineage,
            });
        };
        release_authenticated_rearmed_round(completed, lineage, release_scratch)
    }

    pub(crate) fn destroy_queue_and_retain_any<const C: usize>(
        self,
        engine: &mut Engine<C>,
        retained: impl fmt::Debug + 'static,
    ) -> crate::M1AuthenticatedSpeculativeFailureDispositionV1 {
        use crate::authenticated_speculative_executor::{
            quarantined_disposition, released_disposition,
        };
        let Self {
            outcome,
            release_scratch: _,
            lineage,
        } = self;
        match crate::m1_completed_step::close_m1_authenticated_completed_step_outcome_v1(
            engine, outcome,
        ) {
            crate::m1_completed_step::M1AuthenticatedCompletedStepClosureV1::Released(released) => {
                released_disposition((released, lineage, retained))
            }
            crate::m1_completed_step::M1AuthenticatedCompletedStepClosureV1::Quarantined(
                quarantined,
            ) => quarantined_disposition((quarantined, lineage, retained)),
        }
    }
}

/// Terminal authenticated completion poison retaining all rearm lineage.
#[must_use = "authenticated completion poison and lineage remain retained"]
#[derive(Debug)]
pub struct M1AuthenticatedRearmedPoisonedCompletionV1 {
    poison: crate::M1AuthenticatedCompletedStepPoisonV1,
    lineage: M1AuthenticatedRearmPriorRoundCustodyV1,
}

impl M1AuthenticatedRearmedPoisonedCompletionV1 {
    pub const fn poison(&self) -> &crate::M1AuthenticatedCompletedStepPoisonV1 {
        &self.poison
    }

    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.lineage.parked.len()
    }

    #[must_use]
    pub const fn terminal_lineage_count(&self) -> usize {
        self.lineage.terminal.len()
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.lineage.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&crate::M1RearmRoundHistoryEntryV1> {
        self.lineage.history.get(index)
    }
}

/// Clean teardown retaining a rejected authenticated completion and lineage.
#[must_use = "authenticated rejected-completion teardown remains retained"]
#[derive(Debug)]
pub struct M1AuthenticatedRearmedRejectedCompletionTeardownSuccessV1 {
    source: crate::M1AuthenticatedCompletedStepRejectionTeardownSuccessV1,
    lineage: M1AuthenticatedRearmPriorRoundCustodyV1,
}

/// Terminal release quarantine retaining rejected authenticated completion lineage.
#[must_use = "authenticated rejected-completion quarantine remains retained"]
#[derive(Debug)]
pub struct M1AuthenticatedRearmedRejectedCompletionTeardownFailureV1 {
    source: Box<crate::M1AuthenticatedCompletedStepRejectionTeardownFailureV1>,
    lineage: M1AuthenticatedRearmPriorRoundCustodyV1,
}

impl M1AuthenticatedRearmedRejectedCompletionTeardownSuccessV1 {
    pub const fn source(&self) -> &crate::M1AuthenticatedCompletedStepRejectionTeardownSuccessV1 {
        &self.source
    }

    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.lineage.parked.len()
    }

    #[must_use]
    pub const fn terminal_lineage_count(&self) -> usize {
        self.lineage.terminal.len()
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.lineage.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&crate::M1RearmRoundHistoryEntryV1> {
        self.lineage.history.get(index)
    }
}

impl M1AuthenticatedRearmedRejectedCompletionTeardownFailureV1 {
    pub const fn source(&self) -> &crate::M1AuthenticatedCompletedStepRejectionTeardownFailureV1 {
        &self.source
    }

    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.lineage.parked.len()
    }

    #[must_use]
    pub const fn terminal_lineage_count(&self) -> usize {
        self.lineage.terminal.len()
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.lineage.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&crate::M1RearmRoundHistoryEntryV1> {
        self.lineage.history.get(index)
    }
}

/// Retry-safe authenticated page-release rejection with complete prior lineage.
#[must_use = "authenticated page-release rejection remains the sole retry owner"]
#[derive(Debug)]
pub struct M1AuthenticatedRearmedRoundPageReleaseFailureV1 {
    source: Box<crate::M1AuthenticatedCompletedStepKvReleaseFailureV1>,
    lineage: M1AuthenticatedRearmPriorRoundCustodyV1,
}

impl M1AuthenticatedRearmedRoundPageReleaseFailureV1 {
    pub const fn source(&self) -> &crate::M1AuthenticatedCompletedStepKvReleaseFailureV1 {
        &self.source
    }

    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.lineage.parked.len()
    }

    #[must_use]
    pub const fn terminal_lineage_count(&self) -> usize {
        self.lineage.terminal.len()
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.lineage.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&crate::M1RearmRoundHistoryEntryV1> {
        self.lineage.history.get(index)
    }

    /// Retries page release with the exact unchanged completed owner.
    #[must_use = "retry outcome retains every authenticated round owner"]
    pub fn retry(self) -> M1AuthenticatedRearmedRoundReleaseOutcomeV1 {
        let (_error, completed) = (*self.source).into_parts();
        release_authenticated_rearmed_round(completed, self.lineage, None)
    }

    /// Destroys the queue after page release cannot make progress.
    ///
    /// # Errors
    ///
    /// Returns terminal authenticated queue-release quarantine joined to the
    /// same page-release diagnostic and prior lineage.
    pub fn destroy_queue_and_retain_round<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> Result<
        M1AuthenticatedRearmedRoundPageReleaseTeardownSuccessV1,
        Box<M1AuthenticatedRearmedRoundPageReleaseTeardownFailureV1>,
    > {
        let (error, completed) = (*self.source).into_parts();
        match completed.destroy_queue_and_retain_completion(engine) {
            Ok(completed) => Ok(M1AuthenticatedRearmedRoundPageReleaseTeardownSuccessV1 {
                error,
                completed,
                lineage: self.lineage,
            }),
            Err(completed) => Err(Box::new(
                M1AuthenticatedRearmedRoundPageReleaseTeardownFailureV1 {
                    error,
                    completed,
                    lineage: self.lineage,
                },
            )),
        }
    }
}

/// Clean teardown retaining an authenticated page-release rejection and lineage.
#[must_use = "authenticated page-release teardown remains retained"]
#[derive(Debug)]
pub struct M1AuthenticatedRearmedRoundPageReleaseTeardownSuccessV1 {
    error: crate::M1CompletedStepKvReleaseErrorV1,
    completed: crate::M1AuthenticatedCompletedStepTeardownSuccessV1,
    lineage: M1AuthenticatedRearmPriorRoundCustodyV1,
}

/// Terminal release quarantine retaining authenticated page-release lineage.
#[must_use = "authenticated page-release quarantine remains retained"]
#[derive(Debug)]
pub struct M1AuthenticatedRearmedRoundPageReleaseTeardownFailureV1 {
    error: crate::M1CompletedStepKvReleaseErrorV1,
    completed: Box<crate::M1AuthenticatedCompletedStepTeardownFailureV1>,
    lineage: M1AuthenticatedRearmPriorRoundCustodyV1,
}

impl M1AuthenticatedRearmedRoundPageReleaseTeardownSuccessV1 {
    #[must_use]
    pub const fn error(&self) -> &crate::M1CompletedStepKvReleaseErrorV1 {
        &self.error
    }

    pub const fn completed(&self) -> &crate::M1AuthenticatedCompletedStepTeardownSuccessV1 {
        &self.completed
    }

    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.lineage.parked.len()
    }

    #[must_use]
    pub const fn terminal_lineage_count(&self) -> usize {
        self.lineage.terminal.len()
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.lineage.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&crate::M1RearmRoundHistoryEntryV1> {
        self.lineage.history.get(index)
    }
}

impl M1AuthenticatedRearmedRoundPageReleaseTeardownFailureV1 {
    #[must_use]
    pub const fn error(&self) -> &crate::M1CompletedStepKvReleaseErrorV1 {
        &self.error
    }

    pub const fn completed(&self) -> &crate::M1AuthenticatedCompletedStepTeardownFailureV1 {
        &self.completed
    }

    #[must_use]
    pub const fn parked_count(&self) -> usize {
        self.lineage.parked.len()
    }

    #[must_use]
    pub const fn terminal_lineage_count(&self) -> usize {
        self.lineage.terminal.len()
    }

    #[must_use]
    pub const fn round_history_len(&self) -> usize {
        self.lineage.history.len()
    }

    #[must_use]
    pub fn round_history(&self, index: usize) -> Option<&crate::M1RearmRoundHistoryEntryV1> {
        self.lineage.history.get(index)
    }
}

/// Exhaustive transition from authenticated completion into another schedulable round.
#[must_use = "every authenticated release outcome retains exact linear custody"]
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum M1AuthenticatedRearmedRoundReleaseOutcomeV1 {
    Released(M1AuthenticatedLongLivedQueueReleasedRoundV1),
    Rejected(Box<M1AuthenticatedRearmedRoundPageReleaseFailureV1>),
    NotCompleted(M1AuthenticatedRearmedCompletionOutcomeV1),
}

fn release_authenticated_rearmed_round(
    completed: crate::M1AuthenticatedCompletedStepSuccessV1,
    lineage: M1AuthenticatedRearmPriorRoundCustodyV1,
    scratch: Option<crate::m1_completed_step_release::M1CompletedStepKvReleaseScratchV1>,
) -> M1AuthenticatedRearmedRoundReleaseOutcomeV1 {
    let released = match scratch {
        Some(scratch) => crate::m1_completed_step_release::release_m1_authenticated_completed_step_kv_pages_with_scratch_v1(completed, scratch),
        None => crate::release_m1_authenticated_completed_step_kv_pages_v1(completed),
    };
    match released {
        Ok(released) => M1AuthenticatedRearmedRoundReleaseOutcomeV1::Released(
            M1AuthenticatedLongLivedQueueReleasedRoundV1 {
                released,
                parked: lineage.parked,
                terminal: lineage.terminal,
                history: M1RearmRoundHistoryV1::NonEmpty(lineage.history),
                new_window_bridge: lineage.new_window_bridge,
            },
        ),
        Err(source) => M1AuthenticatedRearmedRoundReleaseOutcomeV1::Rejected(Box::new(
            M1AuthenticatedRearmedRoundPageReleaseFailureV1 { source, lineage },
        )),
    }
}

/// Rebinds fresh authenticated workspaces and submits the next queue generation.
///
/// # Errors
///
/// Returns opaque terminal custody and permanently faults `engine` for any
/// rebind preflight/effect failure or authenticated queue submission rejection.
pub fn submit_m1_authenticated_long_lived_queue_rearm_v1<const C: usize>(
    engine: &mut Engine<C>,
    prepared: M1AuthenticatedPreparedLongLivedQueueRearmV1,
    recipe: AddresslessM1PhysicalBufferRecipeV1,
) -> Result<
    M1AuthenticatedRearmedPublishedQueueV1,
    M1AuthenticatedLongLivedQueueRearmSubmissionFailureV1,
> {
    submit_m1_authenticated_long_lived_queue_rearm_core_v1(engine, prepared, recipe, None)
}

pub(crate) fn submit_m1_authenticated_long_lived_queue_rearm_resident_v1<const C: usize>(
    engine: &mut Engine<C>,
    prepared: M1AuthenticatedPreparedLongLivedQueueRearmV1,
    recipe: AddresslessM1PhysicalBufferRecipeV1,
    storage: M1AuthenticatedRearmPublicationHostStorageV1,
) -> Result<
    M1AuthenticatedRearmedPublishedQueueV1,
    M1AuthenticatedLongLivedQueueRearmSubmissionFailureV1,
> {
    submit_m1_authenticated_long_lived_queue_rearm_core_v1(engine, prepared, recipe, Some(storage))
}

fn submit_m1_authenticated_long_lived_queue_rearm_core_v1<const C: usize>(
    engine: &mut Engine<C>,
    prepared: M1AuthenticatedPreparedLongLivedQueueRearmV1,
    recipe: AddresslessM1PhysicalBufferRecipeV1,
    mut storage: Option<M1AuthenticatedRearmPublicationHostStorageV1>,
) -> Result<
    M1AuthenticatedRearmedPublishedQueueV1,
    M1AuthenticatedLongLivedQueueRearmSubmissionFailureV1,
> {
    if engine.is_faulted() {
        let retained = close_prepared_rearm_submission(prepared, recipe);
        return Err(authenticated_classified_submission_failure(
            M1AuthenticatedLongLivedQueueRearmSubmissionPhaseV1::EngineFaulted,
            retained,
        ));
    }
    let M1AuthenticatedPreparedLongLivedQueueRearmV1 {
        prepared,
        remainder,
    } = prepared;
    let M1AuthenticatedScheduledRemainderV1 {
        queue,
        selected,
        parked,
        terminal: terminal_members,
        prior_checked,
        logical_accepted_counts,
        externally_published_counts,
        release_counts,
        completed_members,
        total_released,
        history,
    } = remainder;
    let queue_observation = queue.observation();
    let device = queue.device();
    let previous_epoch = prior_checked.epoch();
    let carry = M1AuthenticatedRearmContinuationCustodyV1 {
        selected,
        parked,
        terminal: terminal_members,
        previous_epoch,
        prior_checked,
        logical_accepted_counts,
        externally_published_counts,
        release_counts,
        completed_members,
        total_released,
        history,
        rollover: None,
        new_window_bridge: None,
    };
    let queue = match rearm_m1_authenticated_detached_queue_core_v1(
        queue,
        prepared,
        recipe,
        queue_observation,
        storage.as_mut(),
    ) {
        Ok(queue) => queue,
        Err(M1AuthenticatedQueueRearmFailureV1::Rejected(rejection)) => {
            let error = rejection.error();
            let retained = rejection
                .close_without_authority(engine)
                .retain((error, carry));
            return Err(authenticated_classified_submission_failure(
                M1AuthenticatedLongLivedQueueRearmSubmissionPhaseV1::RebindPreflight,
                retained,
            ));
        }
        Err(M1AuthenticatedQueueRearmFailureV1::Terminal(terminal)) => {
            let phase = authenticated_submission_phase(terminal.phase());
            engine.quarantine_m1_queue_rearm_failure();
            return Err(authenticated_classified_submission_failure(
                phase,
                terminal.into_custody().retain(carry),
            ));
        }
    };
    let queue = match queue.submit() {
        Ok(queue) => queue,
        Err(failure) => {
            use crate::authenticated_physical_queue::M1AuthenticatedPhysicalQueueClosureV1;
            let retained = match failure.close_without_authority(engine) {
                M1AuthenticatedPhysicalQueueClosureV1::Released(released) => {
                    AuthenticatedSubmissionOpaqueCustodyV1::Released(Box::new((released, carry)))
                }
                M1AuthenticatedPhysicalQueueClosureV1::Quarantined(quarantined) => {
                    AuthenticatedSubmissionOpaqueCustodyV1::Quarantined(Box::new((
                        quarantined,
                        carry,
                    )))
                }
            };
            return Err(authenticated_classified_submission_failure(
                M1AuthenticatedLongLivedQueueRearmSubmissionPhaseV1::QueueSubmit,
                retained,
            ));
        }
    };
    Ok(M1AuthenticatedRearmedPublishedQueueV1 {
        queue,
        carry,
        queue_observation,
        device,
    })
}

/// Stable pure rejection before an authenticated queue allocation is replaced.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum M1AuthenticatedQueueRearmPreflightErrorV1 {
    ProgramCatalogIdentity,
    ProgramFamilyArtifacts,
    OperationPlan(M1PhysicalFixedBatchBuildErrorV1),
    Selection,
    TargetKvArena,
    DraftKvArena,
    KvDevice,
    PhysicalRecipe,
    WorkspaceComposition,
    SourceRows,
    WorkspaceKind,
    RetainedIntentShape,
    FutureMaterialization,
    PacketCount,
    ImageCount,
    WorkspacePlan,
    ShapeKind,
    WorkspaceContent,
    DiagnosticCapture,
}

/// Effectful phase at which authenticated same-queue rebind became terminal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum M1AuthenticatedQueueRearmTerminalPhaseV1 {
    WorkspaceContent,
    DraftWorkspaceReplacement,
    TargetWorkspaceReplacement,
    DirectDiagnosticChoiceReplacement,
    SpeculativeDraftChoiceReplacement,
    SpeculativeTargetChoiceReplacement,
    WorkspaceRangeRebinding,
    BoundRowRebuild,
    PacketLowering,
    ShapeJoin,
    QueueBind,
    QueueObservation,
}

#[must_use = "unchanged authenticated rearm inputs remain retry-capable"]
pub(crate) struct M1AuthenticatedQueueRearmRejectionV1 {
    error: M1AuthenticatedQueueRearmPreflightErrorV1,
    detached: M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1,
    prepared: M1PreparedScheduledWorkspaceImagesV1,
    recipe: AddresslessM1PhysicalBufferRecipeV1,
}

impl fmt::Debug for M1AuthenticatedQueueRearmRejectionV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedQueueRearmRejectionV1")
            .field("error", &self.error)
            .field("shape", &self.detached.shape())
            .field("prepared_kind", &self.prepared.kind())
            .finish_non_exhaustive()
    }
}

impl M1AuthenticatedQueueRearmRejectionV1 {
    pub(crate) const fn error(&self) -> M1AuthenticatedQueueRearmPreflightErrorV1 {
        self.error
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1,
        M1PreparedScheduledWorkspaceImagesV1,
        AddresslessM1PhysicalBufferRecipeV1,
    ) {
        (self.detached, self.prepared, self.recipe)
    }

    fn close_without_authority<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> AuthenticatedSubmissionOpaqueCustodyV1 {
        engine.quarantine_m1_queue_rearm_failure();
        let Self {
            error,
            detached,
            prepared,
            recipe,
        } = self;
        let (shape, lower, witness, operations, custody) = detached.into_rearm_parts();
        match lower.destroy_and_release() {
            Ok(release) => AuthenticatedSubmissionOpaqueCustodyV1::Released(Box::new((
                release, error, shape, witness, operations, custody, prepared, recipe,
            ))),
            Err(quarantined) => AuthenticatedSubmissionOpaqueCustodyV1::Quarantined(Box::new((
                quarantined,
                error,
                shape,
                witness,
                operations,
                custody,
                prepared,
                recipe,
            ))),
        }
    }
}

#[must_use = "terminal authenticated rearm custody must remain retained"]
pub(crate) struct M1AuthenticatedQueueRearmTerminalV1 {
    phase: M1AuthenticatedQueueRearmTerminalPhaseV1,
    custody: AuthenticatedSubmissionOpaqueCustodyV1,
}

impl fmt::Debug for M1AuthenticatedQueueRearmTerminalV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedQueueRearmTerminalV1")
            .field("phase", &self.phase)
            .field(
                "queue_released",
                &matches!(
                    &self.custody,
                    AuthenticatedSubmissionOpaqueCustodyV1::Released(_)
                ),
            )
            .finish()
    }
}

impl M1AuthenticatedQueueRearmTerminalV1 {
    pub(crate) const fn phase(&self) -> M1AuthenticatedQueueRearmTerminalPhaseV1 {
        self.phase
    }

    fn into_custody(self) -> AuthenticatedSubmissionOpaqueCustodyV1 {
        self.custody
    }
}

#[must_use = "authenticated rearm failure retains every available owner"]
#[derive(Debug)]
pub(crate) enum M1AuthenticatedQueueRearmFailureV1 {
    Rejected(Box<M1AuthenticatedQueueRearmRejectionV1>),
    Terminal(Box<M1AuthenticatedQueueRearmTerminalV1>),
}

fn terminal<T: fmt::Debug + 'static>(
    phase: M1AuthenticatedQueueRearmTerminalPhaseV1,
    retained: T,
) -> M1AuthenticatedQueueRearmFailureV1 {
    M1AuthenticatedQueueRearmFailureV1::Terminal(Box::new(M1AuthenticatedQueueRearmTerminalV1 {
        phase,
        custody: AuthenticatedSubmissionOpaqueCustodyV1::Quarantined(Box::new(retained)),
    }))
}

fn terminal_custody(
    phase: M1AuthenticatedQueueRearmTerminalPhaseV1,
    custody: AuthenticatedSubmissionOpaqueCustodyV1,
) -> M1AuthenticatedQueueRearmFailureV1 {
    M1AuthenticatedQueueRearmFailureV1::Terminal(Box::new(M1AuthenticatedQueueRearmTerminalV1 {
        phase,
        custody,
    }))
}

fn terminal_unbound(
    phase: M1AuthenticatedQueueRearmTerminalPhaseV1,
    lower: AuthenticatedServiceQueueUnboundSessionV1,
    retained: impl fmt::Debug + 'static,
) -> M1AuthenticatedQueueRearmFailureV1 {
    let custody = match lower.destroy_and_release() {
        Ok(release) => {
            AuthenticatedSubmissionOpaqueCustodyV1::Released(Box::new((release, retained)))
        }
        Err(quarantined) => {
            AuthenticatedSubmissionOpaqueCustodyV1::Quarantined(Box::new((quarantined, retained)))
        }
    };
    M1AuthenticatedQueueRearmFailureV1::Terminal(Box::new(M1AuthenticatedQueueRearmTerminalV1 {
        phase,
        custody,
    }))
}

fn terminal_bound<const N: usize>(
    phase: M1AuthenticatedQueueRearmTerminalPhaseV1,
    lower: AuthenticatedServiceQueueSessionV1<N>,
    retained: impl fmt::Debug + 'static,
) -> M1AuthenticatedQueueRearmFailureV1 {
    let custody = match lower.destroy_and_release() {
        Ok(release) => {
            AuthenticatedSubmissionOpaqueCustodyV1::Released(Box::new((release, retained)))
        }
        Err(quarantined) => {
            AuthenticatedSubmissionOpaqueCustodyV1::Quarantined(Box::new((quarantined, retained)))
        }
    };
    M1AuthenticatedQueueRearmFailureV1::Terminal(Box::new(M1AuthenticatedQueueRearmTerminalV1 {
        phase,
        custody,
    }))
}

fn terminal_data_update_failure(
    phase: M1AuthenticatedQueueRearmTerminalPhaseV1,
    failure: AuthenticatedServiceQueueDataUpdateFailureV1,
    retained: impl fmt::Debug + 'static,
) -> M1AuthenticatedQueueRearmFailureV1 {
    match failure {
        AuthenticatedServiceQueueDataUpdateFailureV1::Rejected { error, queue } => {
            terminal_unbound(phase, *queue, (error, retained))
        }
        AuthenticatedServiceQueueDataUpdateFailureV1::Quarantined {
            error,
            retained: queue,
        } => terminal(phase, (error, queue, retained)),
    }
}

#[allow(clippy::boxed_local)]
fn terminal_workspace_failure<const N: usize>(
    phase: M1AuthenticatedQueueRearmTerminalPhaseV1,
    failure: Box<AuthenticatedWorkspaceReplacementFailureV1<N>>,
    retained: impl fmt::Debug + 'static,
) -> M1AuthenticatedQueueRearmFailureV1 {
    match *failure {
        AuthenticatedWorkspaceReplacementFailureV1::Update { failure, plan } => {
            terminal_data_update_failure(phase, failure, (plan, retained))
        }
        AuthenticatedWorkspaceReplacementFailureV1::Binding(failure) => match *failure {
            M1AuthenticatedQueueReplacedWorkspaceBindingFailureV1::Plan { failure, update } => {
                let (lower, subleases, ranges) = update.into_parts();
                terminal_unbound(phase, lower, (failure, subleases, ranges, retained))
            }
            M1AuthenticatedQueueReplacedWorkspaceBindingFailureV1::ReturnedRange {
                plan,
                queue,
                subleases,
                ranges,
            } => terminal_unbound(phase, *queue, (plan, subleases, ranges, retained)),
        },
    }
}

fn terminal_bind_failure<const N: usize>(
    phase: M1AuthenticatedQueueRearmTerminalPhaseV1,
    failure: AuthenticatedServiceQueueRetainedBindFailureV1<N>,
    retained: impl fmt::Debug + 'static,
) -> M1AuthenticatedQueueRearmFailureV1 {
    match failure {
        AuthenticatedServiceQueueRetainedBindFailureV1::Program {
            error,
            queue,
            packets,
        } => terminal_unbound(phase, *queue, (error, packets, retained)),
        AuthenticatedServiceQueueRetainedBindFailureV1::QueueRejected {
            error,
            queue,
            packets,
        } => terminal_unbound(phase, *queue, (error, packets, retained)),
        AuthenticatedServiceQueueRetainedBindFailureV1::Quarantined {
            error,
            retained: queue,
        } => terminal(phase, (error, queue, retained)),
    }
}

fn shape_kind_matches(
    shape: M1PhysicalFixedBatchShapeV1,
    kind: M1FullStepWorkspaceInputKind,
) -> bool {
    matches!(
        (shape, kind),
        (
            M1PhysicalFixedBatchShapeV1::TargetOnly,
            M1FullStepWorkspaceInputKind::TargetOnly
        ) | (
            M1PhysicalFixedBatchShapeV1::PairedPrefill,
            M1FullStepWorkspaceInputKind::PairedPrefill
        ) | (
            M1PhysicalFixedBatchShapeV1::SpeculativeK4
                | M1PhysicalFixedBatchShapeV1::SpeculativeK8
                | M1PhysicalFixedBatchShapeV1::SpeculativeK16,
            M1FullStepWorkspaceInputKind::SpeculativeRound
        )
    )
}

fn workspace_plans_match(
    old: &M1FullStepWorkspaceSubleaseOwners,
    fresh: &M1FullStepWorkspacePlans,
) -> bool {
    match (old, fresh) {
        (
            M1FullStepWorkspaceSubleaseOwners::TargetOnly { target: old },
            M1FullStepWorkspacePlans::TargetOnly { target: fresh },
        ) => old.plan() == &**fresh,
        (
            M1FullStepWorkspaceSubleaseOwners::PairedPrefill {
                draft: old_draft,
                target: old_target,
            },
            M1FullStepWorkspacePlans::PairedPrefill {
                draft: fresh_draft,
                target: fresh_target,
            },
        ) => old_draft.plan() == &**fresh_draft && old_target.plan() == &**fresh_target,
        (
            M1FullStepWorkspaceSubleaseOwners::SpeculativeRound {
                draft_decode: old_draft,
                target_speculative: old_target,
            },
            M1FullStepWorkspacePlans::SpeculativeRound {
                draft_decode: fresh_draft,
                target_speculative: fresh_target,
            },
        ) => old_draft.plan() == &**fresh_draft && old_target.plan() == &**fresh_target,
        _ => false,
    }
}

fn workspace_content_is_valid(
    plans: &M1FullStepWorkspacePlans,
    images: &M1FullStepWorkspaceImagesV1,
) -> bool {
    match (plans, images) {
        (
            M1FullStepWorkspacePlans::TargetOnly { .. },
            M1FullStepWorkspaceImagesV1::TargetOnly { target },
        ) => crate::m1_step_workspace_content_descriptor_v1(
            M1InitializedWorkspaceSlotV1::TargetOnlyTarget,
            target,
        )
        .is_ok(),
        (
            M1FullStepWorkspacePlans::PairedPrefill { .. },
            M1FullStepWorkspaceImagesV1::PairedPrefill { draft, target },
        ) => {
            crate::m1_step_workspace_content_descriptor_v1(
                M1InitializedWorkspaceSlotV1::PairedPrefillDraft,
                draft,
            )
            .is_ok()
                && crate::m1_step_workspace_content_descriptor_v1(
                    M1InitializedWorkspaceSlotV1::PairedPrefillTarget,
                    target,
                )
                .is_ok()
        }
        (
            M1FullStepWorkspacePlans::SpeculativeRound { .. },
            M1FullStepWorkspaceImagesV1::SpeculativeRound {
                draft_decode,
                target_speculative,
            },
        ) => {
            crate::m1_step_workspace_content_descriptor_v1(
                M1InitializedWorkspaceSlotV1::SpeculativeDraftDecode,
                draft_decode,
            )
            .is_ok()
                && crate::m1_step_workspace_content_descriptor_v1(
                    M1InitializedWorkspaceSlotV1::SpeculativeTarget,
                    target_speculative,
                )
                .is_ok()
        }
        _ => false,
    }
}

fn validate_kv_arena_ids(
    kind: M1FullStepWorkspaceInputKind,
    fresh_target: ferric_spec::Identity,
    fresh_draft: Option<ferric_spec::Identity>,
    retained_target: ferric_spec::Identity,
    retained_draft: ferric_spec::Identity,
) -> Result<(), M1AuthenticatedQueueRearmPreflightErrorV1> {
    if fresh_target != retained_target {
        return Err(M1AuthenticatedQueueRearmPreflightErrorV1::TargetKvArena);
    }
    let expected_draft = match kind {
        M1FullStepWorkspaceInputKind::TargetOnly => None,
        M1FullStepWorkspaceInputKind::PairedPrefill
        | M1FullStepWorkspaceInputKind::SpeculativeRound
        | M1FullStepWorkspaceInputKind::DraftCatchup => Some(retained_draft),
    };
    if fresh_draft != expected_draft {
        return Err(M1AuthenticatedQueueRearmPreflightErrorV1::DraftKvArena);
    }
    Ok(())
}

const fn diagnostic_capture_is_supported(direct: bool, _speculative: bool) -> bool {
    !direct
}

struct AuthenticatedDiagnosticCaptureResetFailureV1 {
    phase: M1AuthenticatedQueueRearmTerminalPhaseV1,
    custody: AuthenticatedSubmissionOpaqueCustodyV1,
}

fn authenticated_diagnostic_capture_reset_unbound_failure(
    phase: M1AuthenticatedQueueRearmTerminalPhaseV1,
    lower: AuthenticatedServiceQueueUnboundSessionV1,
    retained: impl fmt::Debug + 'static,
) -> AuthenticatedDiagnosticCaptureResetFailureV1 {
    let terminal = terminal_unbound(phase, lower, retained);
    let M1AuthenticatedQueueRearmFailureV1::Terminal(terminal) = terminal else {
        unreachable!("unbound teardown always produces terminal custody")
    };
    AuthenticatedDiagnosticCaptureResetFailureV1 {
        phase,
        custody: terminal.into_custody(),
    }
}

fn authenticated_diagnostic_capture_reset_update_failure(
    phase: M1AuthenticatedQueueRearmTerminalPhaseV1,
    failure: AuthenticatedServiceQueueDataUpdateFailureV1,
    retained: impl fmt::Debug + 'static,
) -> AuthenticatedDiagnosticCaptureResetFailureV1 {
    let terminal = terminal_data_update_failure(phase, failure, retained);
    let M1AuthenticatedQueueRearmFailureV1::Terminal(terminal) = terminal else {
        unreachable!("data-update teardown always produces terminal custody")
    };
    AuthenticatedDiagnosticCaptureResetFailureV1 {
        phase,
        custody: terminal.into_custody(),
    }
}

fn reset_retained_authenticated_diagnostic_capture(
    lower: AuthenticatedServiceQueueUnboundSessionV1,
    mut completion: crate::BoundM1CompletionOutputV1,
) -> Result<
    (
        AuthenticatedServiceQueueUnboundSessionV1,
        crate::BoundM1CompletionOutputV1,
    ),
    AuthenticatedDiagnosticCaptureResetFailureV1,
> {
    if completion.direct_diagnostic_choices().is_some() {
        let (old, image) = {
            let choices = completion
                .direct_diagnostic_choices()
                .expect("presence checked above");
            let image = match choices.replacement_image() {
                Ok(image) => image,
                Err(error) => {
                    return Err(authenticated_diagnostic_capture_reset_unbound_failure(
                        M1AuthenticatedQueueRearmTerminalPhaseV1::DirectDiagnosticChoiceReplacement,
                        lower,
                        (completion, error),
                    ));
                }
            };
            (choices.retained_range(), image)
        };
        let update = match lower.replace_initialized_host_visible::<HostDownloadRoleV1>(old, image)
        {
            Ok(update) => update,
            Err(failure) => {
                return Err(authenticated_diagnostic_capture_reset_update_failure(
                    M1AuthenticatedQueueRearmTerminalPhaseV1::DirectDiagnosticChoiceReplacement,
                    failure,
                    completion,
                ));
            }
        };
        let (lower, range, _snapshot) = update.into_parts();
        if let Err(error) = completion
            .direct_diagnostic_choices_mut()
            .expect("presence checked above")
            .replace_retained_range(range)
        {
            return Err(authenticated_diagnostic_capture_reset_unbound_failure(
                M1AuthenticatedQueueRearmTerminalPhaseV1::DirectDiagnosticChoiceReplacement,
                lower,
                (completion, error),
            ));
        }
        return Ok((lower, completion));
    }

    if completion.speculative_diagnostic_choices().is_some() {
        let (old_draft, draft_image, old_target, target_image) = {
            let choices = completion
                .speculative_diagnostic_choices()
                .expect("presence checked above");
            let draft_image = match choices.replacement_draft_image() {
                Ok(image) => image,
                Err(error) => {
                    return Err(authenticated_diagnostic_capture_reset_unbound_failure(
                        M1AuthenticatedQueueRearmTerminalPhaseV1::SpeculativeDraftChoiceReplacement,
                        lower,
                        (completion, error),
                    ));
                }
            };
            let target_image = match choices.replacement_target_image() {
                Ok(image) => image,
                Err(error) => {
                    return Err(authenticated_diagnostic_capture_reset_unbound_failure(
                        M1AuthenticatedQueueRearmTerminalPhaseV1::SpeculativeTargetChoiceReplacement,
                        lower,
                        (completion, draft_image, error),
                    ));
                }
            };
            (
                choices.retained_draft_range(),
                draft_image,
                choices.retained_target_range(),
                target_image,
            )
        };
        let draft_update = match lower
            .replace_initialized_host_visible::<HostDownloadRoleV1>(old_draft, draft_image)
        {
            Ok(update) => update,
            Err(failure) => {
                return Err(authenticated_diagnostic_capture_reset_update_failure(
                    M1AuthenticatedQueueRearmTerminalPhaseV1::SpeculativeDraftChoiceReplacement,
                    failure,
                    (completion, target_image),
                ));
            }
        };
        let (lower, draft_range, _draft_snapshot) = draft_update.into_parts();
        if let Err(error) = completion
            .speculative_diagnostic_choices_mut()
            .expect("presence checked above")
            .replace_retained_draft_range(draft_range)
        {
            return Err(authenticated_diagnostic_capture_reset_unbound_failure(
                M1AuthenticatedQueueRearmTerminalPhaseV1::SpeculativeDraftChoiceReplacement,
                lower,
                (completion, target_image, error),
            ));
        }
        let target_update = match lower
            .replace_initialized_host_visible::<HostDownloadRoleV1>(old_target, target_image)
        {
            Ok(update) => update,
            Err(failure) => {
                return Err(authenticated_diagnostic_capture_reset_update_failure(
                    M1AuthenticatedQueueRearmTerminalPhaseV1::SpeculativeTargetChoiceReplacement,
                    failure,
                    completion,
                ));
            }
        };
        let (lower, target_range, _target_snapshot) = target_update.into_parts();
        if let Err(error) = completion
            .speculative_diagnostic_choices_mut()
            .expect("presence checked above")
            .replace_retained_target_range(target_range)
        {
            return Err(authenticated_diagnostic_capture_reset_unbound_failure(
                M1AuthenticatedQueueRearmTerminalPhaseV1::SpeculativeTargetChoiceReplacement,
                lower,
                (completion, error),
            ));
        }
        return Ok((lower, completion));
    }

    Ok((lower, completion))
}

fn preflight_authenticated_queue_rearm(
    detached: &M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1,
    prepared: &M1PreparedScheduledWorkspaceImagesV1,
    recipe: &AddresslessM1PhysicalBufferRecipeV1,
) -> Result<(), M1AuthenticatedQueueRearmPreflightErrorV1> {
    let old = detached.custody();
    if detached.program_catalog_id() != old.catalog_id() {
        return Err(M1AuthenticatedQueueRearmPreflightErrorV1::ProgramCatalogIdentity);
    }
    if !detached.program_families_match() {
        return Err(M1AuthenticatedQueueRearmPreflightErrorV1::ProgramFamilyArtifacts);
    }
    if let Err(error) = validate_authenticated_operation_plan_v1(
        detached.operations(),
        recipe.workspace_composition().dispatch_plan(),
    ) {
        return Err(M1AuthenticatedQueueRearmPreflightErrorV1::OperationPlan(
            error,
        ));
    }
    let reservations = prepared.step().kv_reservations();
    if old.selection() != reservations.target_selection() {
        return Err(M1AuthenticatedQueueRearmPreflightErrorV1::Selection);
    }
    validate_kv_arena_ids(
        prepared.kind(),
        reservations.target_allocation_id(),
        reservations.draft_allocation_id(),
        old.partitioned_memory()
            .allocation_id(Qwen3ModelRole::Target8B),
        old.partitioned_memory()
            .allocation_id(Qwen3ModelRole::Draft06B),
    )?;
    if !reservations.all_devices_match(old.device()) {
        return Err(M1AuthenticatedQueueRearmPreflightErrorV1::KvDevice);
    }
    if recipe.kernarg_recipe().source_recipe() != old.physical_recipe() {
        return Err(M1AuthenticatedQueueRearmPreflightErrorV1::PhysicalRecipe);
    }
    if recipe.workspace_composition() != old.workspace_composition() {
        return Err(M1AuthenticatedQueueRearmPreflightErrorV1::WorkspaceComposition);
    }
    if recipe.rows() != old.source_rows() {
        return Err(M1AuthenticatedQueueRearmPreflightErrorV1::SourceRows);
    }
    if prepared.kind() != old.workspace_owners().kind() {
        return Err(M1AuthenticatedQueueRearmPreflightErrorV1::WorkspaceKind);
    }
    if !diagnostic_capture_is_supported(
        old.completion_output()
            .direct_diagnostic_choices()
            .is_some(),
        old.completion_output()
            .speculative_diagnostic_choices()
            .is_some(),
    ) {
        return Err(M1AuthenticatedQueueRearmPreflightErrorV1::DiagnosticCapture);
    }
    if old.retained_intent_shape() != Some(detached.shape()) {
        return Err(M1AuthenticatedQueueRearmPreflightErrorV1::RetainedIntentShape);
    }
    if recipe.requires_future_materialization() {
        return Err(M1AuthenticatedQueueRearmPreflightErrorV1::FutureMaterialization);
    }
    if recipe.rows().len() != detached.shape().packet_count() {
        return Err(M1AuthenticatedQueueRearmPreflightErrorV1::PacketCount);
    }
    if recipe.kernarg_recipe().images().len() != detached.shape().packet_count() {
        return Err(M1AuthenticatedQueueRearmPreflightErrorV1::ImageCount);
    }
    if !workspace_plans_match(old.workspace_owners(), prepared.plans()) {
        return Err(M1AuthenticatedQueueRearmPreflightErrorV1::WorkspacePlan);
    }
    if !shape_kind_matches(detached.shape(), prepared.kind()) {
        return Err(M1AuthenticatedQueueRearmPreflightErrorV1::ShapeKind);
    }
    if !workspace_content_is_valid(prepared.plans(), prepared.images()) {
        return Err(M1AuthenticatedQueueRearmPreflightErrorV1::WorkspaceContent);
    }
    Ok(())
}

#[derive(Debug)]
pub(crate) enum AuthenticatedWorkspaceReplacementFailureV1<const N: usize> {
    Update {
        failure: AuthenticatedServiceQueueDataUpdateFailureV1,
        plan: AddresslessM1StepWorkspacePlan,
    },
    Binding(Box<M1AuthenticatedQueueReplacedWorkspaceBindingFailureV1<N>>),
}

impl<const N: usize> AuthenticatedWorkspaceReplacementFailureV1<N> {
    fn retained_owner_count(&self) -> usize {
        match self {
            Self::Update { failure, plan } => {
                let _ = failure.error();
                let _ = plan.selection();
                2
            }
            Self::Binding(failure) => failure.retained_owner_count(),
        }
    }
}

pub(crate) fn replace_authenticated_workspace<const N: usize>(
    queue: AuthenticatedServiceQueueUnboundSessionV1,
    old: &BoundM1StepWorkspaceSubleases<N>,
    plan: AddresslessM1StepWorkspacePlan,
    bytes: Box<[u8]>,
    descriptor: Gfx942DeviceContentDescriptorV1,
) -> Result<
    (
        AuthenticatedServiceQueueUnboundSessionV1,
        BoundM1StepWorkspaceSubleases<N>,
        [ServiceDeviceDispatchRangeV1; N],
    ),
    Box<AuthenticatedWorkspaceReplacementFailureV1<N>>,
> {
    let allocation = plan.allocation();
    let update = match queue
        .replace_initialized_partitioned_device_local::<DeviceWorkspaceRoleV1, N, N>(
            old.replacement_subleases(),
            bytes,
            allocation.alignment(),
            descriptor,
            member_layout(&plan),
        ) {
        Ok(update) => update,
        Err(failure) => {
            return Err(Box::new(
                AuthenticatedWorkspaceReplacementFailureV1::Update { failure, plan },
            ));
        }
    };
    bind_authenticated_queue_replaced_m1_step_workspace(plan, update)
        .map_err(|failure| Box::new(AuthenticatedWorkspaceReplacementFailureV1::Binding(failure)))
}

pub(crate) fn replace_authenticated_rollover_workspace<const OLD_N: usize, const NEW_N: usize>(
    queue: AuthenticatedServiceQueueUnboundSessionV1,
    old: &BoundM1StepWorkspaceSubleases<OLD_N>,
    plan: AddresslessM1StepWorkspacePlan,
    bytes: Box<[u8]>,
    descriptor: Gfx942DeviceContentDescriptorV1,
) -> Result<
    (
        AuthenticatedServiceQueueUnboundSessionV1,
        BoundM1StepWorkspaceSubleases<NEW_N>,
        [ServiceDeviceDispatchRangeV1; NEW_N],
    ),
    Box<AuthenticatedWorkspaceReplacementFailureV1<NEW_N>>,
> {
    let allocation = plan.allocation();
    let update = match queue
        .replace_initialized_partitioned_device_local::<DeviceWorkspaceRoleV1, OLD_N, NEW_N>(
            old.replacement_subleases(),
            bytes,
            allocation.alignment(),
            descriptor,
            member_layout(&plan),
        ) {
        Ok(update) => update,
        Err(failure) => {
            return Err(Box::new(
                AuthenticatedWorkspaceReplacementFailureV1::Update { failure, plan },
            ));
        }
    };
    bind_authenticated_queue_replaced_m1_step_workspace(plan, update)
        .map_err(|failure| Box::new(AuthenticatedWorkspaceReplacementFailureV1::Binding(failure)))
}

pub(crate) fn descriptor(
    slot: M1InitializedWorkspaceSlotV1,
    bytes: &[u8],
) -> Result<Gfx942DeviceContentDescriptorV1, ()> {
    crate::m1_step_workspace_content_descriptor_v1(slot, bytes).map_err(|_| ())
}

#[allow(clippy::too_many_arguments)]
fn bind_authenticated_case<const N: usize, F>(
    lower: AuthenticatedServiceQueueUnboundSessionV1,
    batch: M1AuthenticatedQueuePacketBatchCaseV1<N>,
    witness: crate::authenticated_kernel_programs::M1AuthenticatedProgramCatalogWitnessV1,
    operations: DeclaredOperationKernelPlan,
    step: M1PrepublicationStepCustodyV1,
    expected_observation: ComputeAqlQueueObservationV1,
    phase: Option<Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<N>>>,
    wrap: F,
) -> Result<M1AuthenticatedPhysicalQueueSessionV1, M1AuthenticatedQueueRearmFailureV1>
where
    F: FnOnce(
        Box<M1AuthenticatedPhysicalQueuePhaseSlotV1<N>>,
    ) -> M1AuthenticatedPhysicalQueueSessionV1,
{
    let (packets, custody) = batch.into_parts();
    let lower = match lower.bind_retained(packets) {
        Ok(lower) => lower,
        Err(failure) => {
            return Err(terminal_bind_failure(
                M1AuthenticatedQueueRearmTerminalPhaseV1::QueueBind,
                failure,
                (witness, operations, custody, step),
            ));
        }
    };
    if lower.observation() != expected_observation {
        return Err(terminal_bound(
            M1AuthenticatedQueueRearmTerminalPhaseV1::QueueObservation,
            lower,
            (witness, operations, custody, step),
        ));
    }
    let owner = M1AuthenticatedPhysicalQueuePhaseOwnerV1::from_queue_rearm(
        lower, witness, operations, custody, step,
    );
    let phase = match phase {
        Some(mut phase) => {
            phase.install_prepared_from_owner(owner);
            phase
        }
        None => Box::new(M1AuthenticatedPhysicalQueuePhaseSlotV1::occupied(owner)),
    };
    Ok(wrap(phase))
}

#[derive(Debug)]
pub(crate) struct M1AuthenticatedDraftCatchupRetainedBindingsV1 {
    parent: Qwen3PlanSelection,
    request: RequestId,
    coordinator_identity: crate::speculative_generation_loop::M1SpeculativeCoordinatorIdentityV1,
    catalog_id: ferric_spec::Identity,
    prior_epoch: CompletionEpoch,
    prior_dispatch_generation: u64,
    source_rows: Box<[crate::M1PhysicalBufferRecipeRowV1]>,
    bound_rows: Box<[crate::M1BoundPhysicalBufferRowV1]>,
}

enum M1AuthenticatedDraftCatchupQueueTransitionV1<'a> {
    Enter(&'a crate::authenticated_speculative_executor::M1AuthenticatedDraftCatchupPendingV1),
    Restore(&'a crate::authenticated_speculative_executor::M1AuthenticatedDraftCatchupCompletedV1),
}

impl M1AuthenticatedDraftCatchupQueueTransitionV1<'_> {
    fn pending(
        &self,
    ) -> &crate::authenticated_speculative_executor::M1AuthenticatedDraftCatchupPendingV1 {
        match self {
            Self::Enter(pending) => pending,
            Self::Restore(completed) => completed.pending(),
        }
    }

    fn intent(&self) -> M1StepDispatchIntent {
        match self {
            Self::Enter(pending) => M1StepDispatchIntent::DraftCatchup(pending.parent()),
            Self::Restore(completed) => {
                M1StepDispatchIntent::SpeculativeRound(completed.pending().parent())
            }
        }
    }
}

fn retain_queue_rearm_failure(
    failure: M1AuthenticatedQueueRearmFailureV1,
    retained: impl fmt::Debug + 'static,
) -> M1AuthenticatedQueueRearmFailureV1 {
    match failure {
        M1AuthenticatedQueueRearmFailureV1::Terminal(mut failure) => {
            failure.custody = failure.custody.retain(retained);
            M1AuthenticatedQueueRearmFailureV1::Terminal(failure)
        }
        failure => terminal(
            M1AuthenticatedQueueRearmTerminalPhaseV1::ShapeJoin,
            (failure, retained),
        ),
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn replace_authenticated_catchup_workspace_pair<
    const OLD_TARGET: usize,
    const NEW_TARGET: usize,
>(
    lower: AuthenticatedServiceQueueUnboundSessionV1,
    old_draft: &BoundM1StepWorkspaceSubleases<{ crate::M1_DRAFT_STEP_WORKSPACE_SUBLEASE_COUNT_V1 }>,
    old_target: &BoundM1StepWorkspaceSubleases<OLD_TARGET>,
    draft_plan: Box<AddresslessM1StepWorkspacePlan>,
    target_plan: Box<AddresslessM1StepWorkspacePlan>,
    draft_bytes: Box<[u8]>,
    target_bytes: Box<[u8]>,
    target_slot: M1InitializedWorkspaceSlotV1,
    mut workspace_ranges: Vec<crate::m1_queue_rearm::FreshWorkspaceRangeV1>,
    wrap: impl FnOnce(
        BoundM1StepWorkspaceSubleases<{ crate::M1_DRAFT_STEP_WORKSPACE_SUBLEASE_COUNT_V1 }>,
        BoundM1StepWorkspaceSubleases<NEW_TARGET>,
    ) -> M1FullStepWorkspaceSubleaseOwners,
) -> Result<
    (
        AuthenticatedServiceQueueUnboundSessionV1,
        M1FullStepWorkspaceSubleaseOwners,
        Vec<crate::m1_queue_rearm::FreshWorkspaceRangeV1>,
    ),
    M1AuthenticatedQueueRearmFailureV1,
> {
    let descriptors = descriptor(
        M1InitializedWorkspaceSlotV1::SpeculativeDraftDecode,
        &draft_bytes,
    )
    .and_then(|draft| descriptor(target_slot, &target_bytes).map(|target| (draft, target)));
    let (draft_descriptor, target_descriptor) = match descriptors {
        Ok(value) => value,
        Err(()) => {
            return Err(terminal_unbound(
                M1AuthenticatedQueueRearmTerminalPhaseV1::WorkspaceContent,
                lower,
                (
                    draft_plan,
                    target_plan,
                    draft_bytes,
                    target_bytes,
                    workspace_ranges,
                ),
            ))
        }
    };
    let (lower, draft, draft_ranges) = match replace_authenticated_workspace(
        lower,
        old_draft,
        *draft_plan,
        draft_bytes,
        draft_descriptor,
    ) {
        Ok(value) => value,
        Err(failure) => {
            return Err(terminal_workspace_failure(
                M1AuthenticatedQueueRearmTerminalPhaseV1::DraftWorkspaceReplacement,
                failure,
                (target_plan, target_bytes, workspace_ranges),
            ))
        }
    };
    let (lower, target, target_ranges) = match replace_authenticated_rollover_workspace(
        lower,
        old_target,
        *target_plan,
        target_bytes,
        target_descriptor,
    ) {
        Ok(value) => value,
        Err(failure) => {
            return Err(terminal_workspace_failure(
                M1AuthenticatedQueueRearmTerminalPhaseV1::TargetWorkspaceReplacement,
                failure,
                (draft, draft_ranges, workspace_ranges),
            ))
        }
    };
    workspace_ranges.clear();
    if workspace_ranges.capacity() < draft_ranges.len() + target_ranges.len() {
        return Err(terminal_unbound(
            M1AuthenticatedQueueRearmTerminalPhaseV1::WorkspaceRangeRebinding,
            lower,
            (draft, target, draft_ranges, target_ranges, workspace_ranges),
        ));
    }
    append_workspace_ranges(
        &mut workspace_ranges,
        M1FullStepWorkspaceRole::Draft,
        &draft,
        draft_ranges,
    );
    append_workspace_ranges(
        &mut workspace_ranges,
        M1FullStepWorkspaceRole::Target,
        &target,
        target_ranges,
    );
    Ok((lower, wrap(draft, target), workspace_ranges))
}

fn preflight_authenticated_draft_catchup_queue_transition(
    detached: &M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1,
    prepared: &M1PreparedScheduledWorkspaceImagesV1,
    recipe: &AddresslessM1PhysicalBufferRecipeV1,
    transition: &M1AuthenticatedDraftCatchupQueueTransitionV1<'_>,
    saved: Option<&M1AuthenticatedDraftCatchupRetainedBindingsV1>,
    storage: &M1AuthenticatedRearmPublicationHostStorageV1,
) -> bool {
    let pending = transition.pending();
    let Some(parent) =
        crate::authenticated_prefill_bootstrap::admitted_s1_t128_speculative_successor_v1(
            pending.parent(),
        )
    else {
        return false;
    };
    let old = detached.custody();
    let scheduled = prepared.step().scheduled_dispatch();
    let (old_draft_plan, old_target_plan) = match old.workspace_owners() {
        M1FullStepWorkspaceSubleaseOwners::SpeculativeRound {
            draft_decode,
            target_speculative,
        } => (draft_decode.plan(), target_speculative.plan()),
        M1FullStepWorkspaceSubleaseOwners::DraftCatchup {
            draft_decode,
            completion,
        } => (draft_decode.plan(), completion.plan()),
        _ => return false,
    };
    let phase_matches = match transition {
        M1AuthenticatedDraftCatchupQueueTransitionV1::Enter(_) => {
            detached.shape() == parent.shape()
                && saved.is_none()
                && detached.detached_dispatch_generation() == pending.prior_dispatch_generation()
                && scheduled.epoch() == pending.epoch()
                && prepared.kind() == M1FullStepWorkspaceInputKind::DraftCatchup
                && storage.shape == M1PhysicalFixedBatchShapeV1::DraftCatchup
                && storage.phase_catchup.is_some()
                && storage.completion_owner.is_some()
                && old
                    .completion_output()
                    .can_retarget_exact_s1_speculative_to_draft_catchup(pending.parent())
                && matches!(prepared.step().kv_reservations(), M1FullStepKvReservationCustodyV1::DraftCatchup { parent, draft, .. }
                    if *parent == pending.parent() && draft.reservations().len() == 1
                        && draft.reservations()[0].matches_authenticated_draft_catchup(pending))
        }
        M1AuthenticatedDraftCatchupQueueTransitionV1::Restore(completed) => {
            detached.shape() == M1PhysicalFixedBatchShapeV1::DraftCatchup
                && detached.detached_dispatch_generation() == completed.dispatch_generation()
                && completed.completion_epoch() == pending.epoch()
                && exact_next_epoch(pending.epoch()) == Some(scheduled.epoch())
                && prepared.kind() == M1FullStepWorkspaceInputKind::SpeculativeRound
                && storage.shape == parent.shape()
                && match parent.shape() {
                    M1PhysicalFixedBatchShapeV1::SpeculativeK4 => storage.phase.is_some(),
                    M1PhysicalFixedBatchShapeV1::SpeculativeK8 => storage.phase_k8.is_some(),
                    M1PhysicalFixedBatchShapeV1::SpeculativeK16 => storage.phase_k16.is_some(),
                    _ => false,
                }
                && storage.speculative_owner.is_some()
                && old.completion_output().draft_catchup_parent_selection()
                    == Some(pending.parent())
                && saved.is_some_and(|saved| {
                    saved.parent == pending.parent()
                        && saved.request == pending.request()
                        && saved.coordinator_identity == pending.coordinator_identity()
                        && saved.catalog_id == old.catalog_id()
                        && saved.prior_epoch == pending.prior_epoch()
                        && saved.prior_dispatch_generation == pending.prior_dispatch_generation()
                        && saved.source_rows.len() == parent.shape().packet_count()
                        && saved.bound_rows.len() == saved.source_rows.len()
                })
        }
    };
    phase_matches
        && scheduled.member_count() == 1
        && scheduled.member(0) == Some(pending.request())
        && old.selection() == pending.parent()
        && storage.selection == pending.parent()
        && storage
            .bound_rows
            .as_ref()
            .is_some_and(|rows| rows.has_capacity_for(recipe.rows()))
        && storage.draft_owner.is_some()
        && recipe.workspace_composition().dispatch_plan().intent() == transition.intent()
        && recipe.workspace_composition().workspace_plans() == prepared.plans()
        && recipe.rows().len() == storage.shape.packet_count()
        && recipe.kernarg_recipe().images().len() == storage.shape.packet_count()
        && !recipe.requires_future_materialization()
        && prepared.step().kv_reservations().target_selection() == pending.parent()
        && prepared
            .step()
            .kv_reservations()
            .all_devices_match(old.device())
        && validate_kv_arena_ids(
            prepared.kind(),
            prepared.step().kv_reservations().target_allocation_id(),
            prepared.step().kv_reservations().draft_allocation_id(),
            old.partitioned_memory()
                .allocation_id(Qwen3ModelRole::Target8B),
            old.partitioned_memory()
                .allocation_id(Qwen3ModelRole::Draft06B),
        )
        .is_ok()
        && prepared.plans().draft().is_some_and(|draft| {
            old_draft_plan.allocation().allocation_id() == draft.allocation().allocation_id()
        })
        && old_target_plan.allocation().allocation_id()
            == prepared.plans().target().allocation().allocation_id()
}

#[allow(clippy::type_complexity)]
fn rearm_authenticated_draft_catchup_queue_transition(
    detached: M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1,
    prepared: M1PreparedScheduledWorkspaceImagesV1,
    recipe: AddresslessM1PhysicalBufferRecipeV1,
    transition: M1AuthenticatedDraftCatchupQueueTransitionV1<'_>,
    mut saved: Option<M1AuthenticatedDraftCatchupRetainedBindingsV1>,
    storage: &mut M1AuthenticatedRearmPublicationHostStorageV1,
) -> Result<
    (
        M1AuthenticatedPhysicalQueueSessionV1,
        M1AuthenticatedDraftCatchupRetainedBindingsV1,
    ),
    (
        M1AuthenticatedQueueRearmFailureV1,
        Option<M1AuthenticatedDraftCatchupRetainedBindingsV1>,
    ),
> {
    if !preflight_authenticated_draft_catchup_queue_transition(
        &detached,
        &prepared,
        &recipe,
        &transition,
        saved.as_ref(),
        storage,
    ) {
        return Err((
            M1AuthenticatedQueueRearmFailureV1::Rejected(Box::new(
                M1AuthenticatedQueueRearmRejectionV1 {
                    error: M1AuthenticatedQueueRearmPreflightErrorV1::ShapeKind,
                    detached,
                    prepared,
                    recipe,
                },
            )),
            saved,
        ));
    }
    macro_rules! fail_transition {
        ($source:expr, $retained:expr) => {
            return Err((retain_queue_rearm_failure($source, $retained), saved))
        };
    }
    let expected_observation = detached.observation();
    let (_, lower, witness, operations, custody) = detached.into_rearm_parts();
    let mut custody = custody.into_rearm_parts();
    let (plans, images, step) = prepared.into_rearm_parts();
    let workspace_ranges = core::mem::take(&mut storage.workspace_ranges);
    let draft_slot = storage
        .draft_owner
        .take()
        .expect("preflight retained draft slot");
    let replacement = match (&custody.workspace_owners, plans, images) {
        (
            M1FullStepWorkspaceSubleaseOwners::SpeculativeRound {
                draft_decode: old_draft,
                target_speculative: old_target,
            },
            M1FullStepWorkspacePlans::DraftCatchup {
                draft_decode,
                completion,
                ..
            },
            M1FullStepWorkspaceImagesV1::DraftCatchup {
                draft_decode: draft_bytes,
                completion: target_bytes,
            },
        ) if matches!(
            transition,
            M1AuthenticatedDraftCatchupQueueTransitionV1::Enter(_)
        ) =>
        {
            let target_slot = storage
                .completion_owner
                .take()
                .expect("preflight retained completion slot");
            replace_authenticated_catchup_workspace_pair(
                lower,
                old_draft,
                old_target,
                draft_decode,
                completion,
                draft_bytes,
                target_bytes,
                M1InitializedWorkspaceSlotV1::TargetOnlyTarget,
                workspace_ranges,
                |draft, target| M1FullStepWorkspaceSubleaseOwners::DraftCatchup {
                    draft_decode: Box::write(draft_slot, draft),
                    completion: Box::write(target_slot, target),
                },
            )
        }
        (
            M1FullStepWorkspaceSubleaseOwners::DraftCatchup {
                draft_decode: old_draft,
                completion: old_target,
            },
            M1FullStepWorkspacePlans::SpeculativeRound {
                draft_decode,
                target_speculative,
            },
            M1FullStepWorkspaceImagesV1::SpeculativeRound {
                draft_decode: draft_bytes,
                target_speculative: target_bytes,
            },
        ) if matches!(
            transition,
            M1AuthenticatedDraftCatchupQueueTransitionV1::Restore(_)
        ) =>
        {
            let target_slot = storage
                .speculative_owner
                .take()
                .expect("preflight retained speculative slot");
            replace_authenticated_catchup_workspace_pair(
                lower,
                old_draft,
                old_target,
                draft_decode,
                target_speculative,
                draft_bytes,
                target_bytes,
                M1InitializedWorkspaceSlotV1::SpeculativeTarget,
                workspace_ranges,
                |draft, target| M1FullStepWorkspaceSubleaseOwners::SpeculativeRound {
                    draft_decode: Box::write(draft_slot, draft),
                    target_speculative: Box::write(target_slot, target),
                },
            )
        }
        (_, plans, images) => {
            fail_transition!(
                terminal_unbound(
                    M1AuthenticatedQueueRearmTerminalPhaseV1::ShapeJoin,
                    lower,
                    (
                        witness,
                        operations,
                        custody,
                        plans,
                        images,
                        step,
                        recipe,
                        workspace_ranges,
                        draft_slot
                    )
                ),
                ()
            );
        }
    };
    let (lower, workspace_owners, workspace_ranges) = match replacement {
        Ok(value) => value,
        Err(source) => fail_transition!(source, (witness, operations, custody, step, recipe)),
    };
    custody.workspace_owners = workspace_owners;
    let output = match &transition {
        M1AuthenticatedDraftCatchupQueueTransitionV1::Enter(pending) => custody
            .completion_output
            .retarget_exact_s1_speculative_to_draft_catchup(pending.parent()),
        M1AuthenticatedDraftCatchupQueueTransitionV1::Restore(completed) => custody
            .completion_output
            .restore_exact_s1_speculative_after_draft_catchup(completed.pending().parent()),
    };
    custody.completion_output = match output {
        Ok(output) => output,
        Err(output) => {
            custody.completion_output = *output;
            fail_transition!(
                terminal_unbound(
                    M1AuthenticatedQueueRearmTerminalPhaseV1::ShapeJoin,
                    lower,
                    (witness, operations, custody, step, recipe, workspace_ranges)
                ),
                ()
            );
        }
    };
    let (lower, output) =
        match reset_retained_authenticated_diagnostic_capture(lower, custody.completion_output) {
            Ok(value) => value,
            Err(failure) => {
                fail_transition!(
                    terminal_custody(failure.phase, failure.custody),
                    (
                        witness,
                        operations,
                        custody.catalog_id,
                        custody.selection,
                        custody.physical_recipe,
                        custody.workspace_composition,
                        custody.workspace_owners,
                        custody.partitioned_memory,
                        custody.source_rows,
                        custody.bound_rows,
                        custody.retired_rollover_custody,
                        step,
                        recipe,
                        workspace_ranges
                    )
                );
            }
        };
    custody.completion_output = output;
    let capture = match retained_host_capture_ranges(&custody.completion_output) {
        Ok(value) => value,
        Err(()) => fail_transition!(
            terminal_unbound(
                M1AuthenticatedQueueRearmTerminalPhaseV1::WorkspaceRangeRebinding,
                lower,
                (witness, operations, custody, step, recipe, workspace_ranges)
            ),
            ()
        ),
    };
    let (source_rows, bound_rows) = match &transition {
        M1AuthenticatedDraftCatchupQueueTransitionV1::Enter(_) => {
            (&*custody.source_rows, &*custody.bound_rows)
        }
        M1AuthenticatedDraftCatchupQueueTransitionV1::Restore(_) => {
            let saved = saved
                .as_ref()
                .expect("restoration preflight retained predecessor bindings");
            (&*saved.source_rows, &*saved.bound_rows)
        }
    };
    let bound_rows = match crate::m1_queue_rearm::build_rollover_bound_rows_with_storage(
        recipe.rows(),
        source_rows,
        bound_rows,
        recipe.workspace_composition(),
        &workspace_ranges,
        &capture,
        storage
            .bound_rows
            .take()
            .expect("preflight retained bound-row storage"),
    ) {
        Ok(rows) => rows,
        Err(()) => fail_transition!(
            terminal_unbound(
                M1AuthenticatedQueueRearmTerminalPhaseV1::BoundRowRebuild,
                lower,
                (witness, operations, custody, step, recipe, workspace_ranges)
            ),
            ()
        ),
    };
    if let M1AuthenticatedDraftCatchupQueueTransitionV1::Enter(pending) = &transition {
        saved = Some(M1AuthenticatedDraftCatchupRetainedBindingsV1 {
            parent: pending.parent(),
            request: pending.request(),
            coordinator_identity: pending.coordinator_identity(),
            catalog_id: custody.catalog_id,
            prior_epoch: pending.prior_epoch(),
            prior_dispatch_generation: pending.prior_dispatch_generation(),
            source_rows: core::mem::take(&mut custody.source_rows),
            bound_rows: core::mem::take(&mut custody.bound_rows),
        });
    }
    let batch = match build_m1_authenticated_rollover_packet_batch_with_storage_v1(
        &witness,
        &operations,
        recipe,
        bound_rows,
        M1PhysicalQueueBatchCustodyV1::from_rearm_parts(custody),
        &mut storage.packet_batch,
    ) {
        Ok(batch) => batch,
        Err(source) => fail_transition!(
            terminal_unbound(
                M1AuthenticatedQueueRearmTerminalPhaseV1::PacketLowering,
                lower,
                (witness, operations, source, step, workspace_ranges)
            ),
            ()
        ),
    };
    let bound = match (storage.shape, batch) {
        (
            M1PhysicalFixedBatchShapeV1::DraftCatchup,
            M1AuthenticatedQueuePacketBatchV1::DraftCatchup(batch),
        ) => bind_authenticated_case(
            lower,
            *batch,
            witness,
            operations,
            step,
            expected_observation,
            storage.phase_catchup.take(),
            M1AuthenticatedPhysicalQueueSessionV1::DraftCatchup,
        ),
        (
            M1PhysicalFixedBatchShapeV1::SpeculativeK4,
            M1AuthenticatedQueuePacketBatchV1::SpeculativeK4(batch),
        ) => bind_authenticated_case(
            lower,
            *batch,
            witness,
            operations,
            step,
            expected_observation,
            storage.phase.take(),
            M1AuthenticatedPhysicalQueueSessionV1::SpeculativeK4,
        ),
        (
            M1PhysicalFixedBatchShapeV1::SpeculativeK8,
            M1AuthenticatedQueuePacketBatchV1::SpeculativeK8(batch),
        ) => bind_authenticated_case(
            lower,
            *batch,
            witness,
            operations,
            step,
            expected_observation,
            storage.phase_k8.take(),
            M1AuthenticatedPhysicalQueueSessionV1::SpeculativeK8,
        ),
        (
            M1PhysicalFixedBatchShapeV1::SpeculativeK16,
            M1AuthenticatedQueuePacketBatchV1::SpeculativeK16(batch),
        ) => bind_authenticated_case(
            lower,
            *batch,
            witness,
            operations,
            step,
            expected_observation,
            storage.phase_k16.take(),
            M1AuthenticatedPhysicalQueueSessionV1::SpeculativeK16,
        ),
        (_, batch) => Err(terminal_unbound(
            M1AuthenticatedQueueRearmTerminalPhaseV1::ShapeJoin,
            lower,
            (witness, operations, batch, step),
        )),
    };
    match bound {
        Ok(queue) => Ok((
            queue,
            saved.expect("entry or restoration retains predecessor bindings"),
        )),
        Err(source) => Err((source, saved)),
    }
}

pub(crate) fn rearm_m1_authenticated_detached_queue_v1(
    detached: M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1,
    prepared: M1PreparedScheduledWorkspaceImagesV1,
    recipe: AddresslessM1PhysicalBufferRecipeV1,
    expected_observation: ComputeAqlQueueObservationV1,
) -> Result<M1AuthenticatedPhysicalQueueSessionV1, M1AuthenticatedQueueRearmFailureV1> {
    rearm_m1_authenticated_detached_queue_core_v1(
        detached,
        prepared,
        recipe,
        expected_observation,
        None,
    )
}

fn rearm_m1_authenticated_detached_queue_core_v1(
    detached: M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1,
    prepared: M1PreparedScheduledWorkspaceImagesV1,
    recipe: AddresslessM1PhysicalBufferRecipeV1,
    expected_observation: ComputeAqlQueueObservationV1,
    mut storage: Option<&mut M1AuthenticatedRearmPublicationHostStorageV1>,
) -> Result<M1AuthenticatedPhysicalQueueSessionV1, M1AuthenticatedQueueRearmFailureV1> {
    if let Err(error) = preflight_authenticated_queue_rearm(&detached, &prepared, &recipe) {
        return Err(M1AuthenticatedQueueRearmFailureV1::Rejected(Box::new(
            M1AuthenticatedQueueRearmRejectionV1 {
                error,
                detached,
                prepared,
                recipe,
            },
        )));
    }

    let (shape, lower, witness, operations, custody) = detached.into_rearm_parts();
    let mut custody = custody.into_rearm_parts();
    let (plans, images, step) = prepared.into_rearm_parts();

    let (lower, workspace_owners, workspace_ranges) =
        match (shape, &custody.workspace_owners, plans, images) {
            (
                M1PhysicalFixedBatchShapeV1::TargetOnly,
                M1FullStepWorkspaceSubleaseOwners::TargetOnly { target: old_target },
                M1FullStepWorkspacePlans::TargetOnly { target: plan },
                M1FullStepWorkspaceImagesV1::TargetOnly { target: bytes },
            ) => {
                let descriptor =
                    match descriptor(M1InitializedWorkspaceSlotV1::TargetOnlyTarget, &bytes) {
                        Ok(descriptor) => descriptor,
                        Err(()) => {
                            return Err(terminal_unbound(
                                M1AuthenticatedQueueRearmTerminalPhaseV1::WorkspaceContent,
                                lower,
                                (witness, operations, custody, plan, bytes, recipe, step),
                            ));
                        }
                    };
                let (lower, target, ranges) = match replace_authenticated_workspace(
                    lower, old_target, *plan, bytes, descriptor,
                ) {
                    Ok(replaced) => replaced,
                    Err(failure) => {
                        let _ = failure.retained_owner_count();
                        return Err(terminal_workspace_failure(
                            M1AuthenticatedQueueRearmTerminalPhaseV1::TargetWorkspaceReplacement,
                            failure,
                            (witness, operations, custody, recipe, step),
                        ));
                    }
                };
                let mut workspace_ranges =
                    storage.as_deref_mut().map_or_else(Vec::new, |storage| {
                        core::mem::take(&mut storage.workspace_ranges)
                    });
                workspace_ranges.clear();
                if workspace_ranges.try_reserve_exact(ranges.len()).is_err() {
                    return Err(terminal_unbound(
                        M1AuthenticatedQueueRearmTerminalPhaseV1::WorkspaceRangeRebinding,
                        lower,
                        (witness, operations, custody, target, ranges, recipe, step),
                    ));
                }
                append_workspace_ranges(
                    &mut workspace_ranges,
                    M1FullStepWorkspaceRole::Target,
                    &target,
                    ranges,
                );
                (
                    lower,
                    M1FullStepWorkspaceSubleaseOwners::target_only(target),
                    workspace_ranges,
                )
            }
            (
                M1PhysicalFixedBatchShapeV1::PairedPrefill,
                M1FullStepWorkspaceSubleaseOwners::PairedPrefill {
                    draft: old_draft,
                    target: old_target,
                },
                M1FullStepWorkspacePlans::PairedPrefill {
                    draft: draft_plan,
                    target: target_plan,
                },
                M1FullStepWorkspaceImagesV1::PairedPrefill {
                    draft: draft_bytes,
                    target: target_bytes,
                },
            ) => {
                let draft_descriptor = match descriptor(
                    M1InitializedWorkspaceSlotV1::PairedPrefillDraft,
                    &draft_bytes,
                ) {
                    Ok(descriptor) => descriptor,
                    Err(()) => {
                        return Err(terminal_unbound(
                            M1AuthenticatedQueueRearmTerminalPhaseV1::WorkspaceContent,
                            lower,
                            (
                                witness,
                                operations,
                                custody,
                                draft_plan,
                                target_plan,
                                draft_bytes,
                                target_bytes,
                                recipe,
                                step,
                            ),
                        ));
                    }
                };
                let target_descriptor = match descriptor(
                    M1InitializedWorkspaceSlotV1::PairedPrefillTarget,
                    &target_bytes,
                ) {
                    Ok(descriptor) => descriptor,
                    Err(()) => {
                        return Err(terminal_unbound(
                            M1AuthenticatedQueueRearmTerminalPhaseV1::WorkspaceContent,
                            lower,
                            (
                                witness,
                                operations,
                                custody,
                                draft_plan,
                                target_plan,
                                draft_bytes,
                                target_bytes,
                                recipe,
                                step,
                            ),
                        ));
                    }
                };
                let (lower, draft, draft_ranges) = match replace_authenticated_workspace(
                    lower,
                    old_draft,
                    *draft_plan,
                    draft_bytes,
                    draft_descriptor,
                ) {
                    Ok(replaced) => replaced,
                    Err(failure) => {
                        let _ = failure.retained_owner_count();
                        return Err(terminal_workspace_failure(
                            M1AuthenticatedQueueRearmTerminalPhaseV1::DraftWorkspaceReplacement,
                            failure,
                            (
                                witness,
                                operations,
                                custody,
                                target_plan,
                                target_bytes,
                                recipe,
                                step,
                            ),
                        ));
                    }
                };
                let (lower, target, target_ranges) = match replace_authenticated_workspace(
                    lower,
                    old_target,
                    *target_plan,
                    target_bytes,
                    target_descriptor,
                ) {
                    Ok(replaced) => replaced,
                    Err(failure) => {
                        let _ = failure.retained_owner_count();
                        return Err(terminal_workspace_failure(
                            M1AuthenticatedQueueRearmTerminalPhaseV1::TargetWorkspaceReplacement,
                            failure,
                            (
                                witness,
                                operations,
                                custody,
                                draft,
                                draft_ranges,
                                recipe,
                                step,
                            ),
                        ));
                    }
                };
                let mut workspace_ranges =
                    storage.as_deref_mut().map_or_else(Vec::new, |storage| {
                        core::mem::take(&mut storage.workspace_ranges)
                    });
                workspace_ranges.clear();
                if workspace_ranges
                    .try_reserve_exact(draft_ranges.len() + target_ranges.len())
                    .is_err()
                {
                    return Err(terminal_unbound(
                        M1AuthenticatedQueueRearmTerminalPhaseV1::WorkspaceRangeRebinding,
                        lower,
                        (
                            witness,
                            operations,
                            custody,
                            draft,
                            target,
                            draft_ranges,
                            target_ranges,
                            recipe,
                            step,
                        ),
                    ));
                }
                append_workspace_ranges(
                    &mut workspace_ranges,
                    M1FullStepWorkspaceRole::Draft,
                    &draft,
                    draft_ranges,
                );
                append_workspace_ranges(
                    &mut workspace_ranges,
                    M1FullStepWorkspaceRole::Target,
                    &target,
                    target_ranges,
                );
                (
                    lower,
                    M1FullStepWorkspaceSubleaseOwners::paired_prefill(draft, target),
                    workspace_ranges,
                )
            }
            (
                M1PhysicalFixedBatchShapeV1::SpeculativeK4
                | M1PhysicalFixedBatchShapeV1::SpeculativeK8
                | M1PhysicalFixedBatchShapeV1::SpeculativeK16,
                M1FullStepWorkspaceSubleaseOwners::SpeculativeRound {
                    draft_decode: old_draft,
                    target_speculative: old_target,
                },
                M1FullStepWorkspacePlans::SpeculativeRound {
                    draft_decode: draft_plan,
                    target_speculative: target_plan,
                },
                M1FullStepWorkspaceImagesV1::SpeculativeRound {
                    draft_decode: draft_bytes,
                    target_speculative: target_bytes,
                },
            ) => {
                let draft_descriptor = match descriptor(
                    M1InitializedWorkspaceSlotV1::SpeculativeDraftDecode,
                    &draft_bytes,
                ) {
                    Ok(descriptor) => descriptor,
                    Err(()) => {
                        return Err(terminal_unbound(
                            M1AuthenticatedQueueRearmTerminalPhaseV1::WorkspaceContent,
                            lower,
                            (
                                witness,
                                operations,
                                custody,
                                draft_plan,
                                target_plan,
                                draft_bytes,
                                target_bytes,
                                recipe,
                                step,
                            ),
                        ));
                    }
                };
                let target_descriptor = match descriptor(
                    M1InitializedWorkspaceSlotV1::SpeculativeTarget,
                    &target_bytes,
                ) {
                    Ok(descriptor) => descriptor,
                    Err(()) => {
                        return Err(terminal_unbound(
                            M1AuthenticatedQueueRearmTerminalPhaseV1::WorkspaceContent,
                            lower,
                            (
                                witness,
                                operations,
                                custody,
                                draft_plan,
                                target_plan,
                                draft_bytes,
                                target_bytes,
                                recipe,
                                step,
                            ),
                        ));
                    }
                };
                let (lower, draft, draft_ranges) = match replace_authenticated_workspace(
                    lower,
                    old_draft,
                    *draft_plan,
                    draft_bytes,
                    draft_descriptor,
                ) {
                    Ok(replaced) => replaced,
                    Err(failure) => {
                        let _ = failure.retained_owner_count();
                        return Err(terminal_workspace_failure(
                            M1AuthenticatedQueueRearmTerminalPhaseV1::DraftWorkspaceReplacement,
                            failure,
                            (
                                witness,
                                operations,
                                custody,
                                target_plan,
                                target_bytes,
                                recipe,
                                step,
                            ),
                        ));
                    }
                };
                let (lower, target, target_ranges) = match replace_authenticated_workspace(
                    lower,
                    old_target,
                    *target_plan,
                    target_bytes,
                    target_descriptor,
                ) {
                    Ok(replaced) => replaced,
                    Err(failure) => {
                        let _ = failure.retained_owner_count();
                        return Err(terminal_workspace_failure(
                            M1AuthenticatedQueueRearmTerminalPhaseV1::TargetWorkspaceReplacement,
                            failure,
                            (
                                witness,
                                operations,
                                custody,
                                draft,
                                draft_ranges,
                                recipe,
                                step,
                            ),
                        ));
                    }
                };
                let mut workspace_ranges =
                    storage.as_deref_mut().map_or_else(Vec::new, |storage| {
                        core::mem::take(&mut storage.workspace_ranges)
                    });
                workspace_ranges.clear();
                if workspace_ranges
                    .try_reserve_exact(draft_ranges.len() + target_ranges.len())
                    .is_err()
                {
                    return Err(terminal_unbound(
                        M1AuthenticatedQueueRearmTerminalPhaseV1::WorkspaceRangeRebinding,
                        lower,
                        (
                            witness,
                            operations,
                            custody,
                            draft,
                            target,
                            draft_ranges,
                            target_ranges,
                            recipe,
                            step,
                        ),
                    ));
                }
                append_workspace_ranges(
                    &mut workspace_ranges,
                    M1FullStepWorkspaceRole::Draft,
                    &draft,
                    draft_ranges,
                );
                append_workspace_ranges(
                    &mut workspace_ranges,
                    M1FullStepWorkspaceRole::Target,
                    &target,
                    target_ranges,
                );
                (
                    lower,
                    M1FullStepWorkspaceSubleaseOwners::speculative_round(draft, target),
                    workspace_ranges,
                )
            }
            (_, _, plans, images) => {
                return Err(terminal_unbound(
                    M1AuthenticatedQueueRearmTerminalPhaseV1::ShapeJoin,
                    lower,
                    (witness, operations, custody, plans, images, recipe, step),
                ));
            }
        };

    custody.workspace_owners = workspace_owners;
    let previous_capture = match retained_host_capture_ranges(&custody.completion_output) {
        Ok(capture) => capture,
        Err(()) => {
            return Err(terminal_unbound(
                M1AuthenticatedQueueRearmTerminalPhaseV1::WorkspaceRangeRebinding,
                lower,
                (witness, operations, custody, workspace_ranges, recipe, step),
            ));
        }
    };
    let (lower, completion_output) =
        match reset_retained_authenticated_diagnostic_capture(lower, custody.completion_output) {
            Ok(reset) => reset,
            Err(failure) => {
                let phase = failure.phase;
                return Err(terminal_custody(
                    phase,
                    failure.custody.retain((
                        witness,
                        operations,
                        (
                            custody.catalog_id,
                            custody.selection,
                            custody.physical_recipe,
                            custody.workspace_composition,
                            custody.workspace_owners,
                            custody.partitioned_memory,
                            custody.source_rows,
                            custody.bound_rows,
                            workspace_ranges,
                            recipe,
                            step,
                        ),
                    )),
                ));
            }
        };
    custody.completion_output = completion_output;
    let retained_capture = match retained_host_capture_ranges(&custody.completion_output) {
        Ok(capture) => capture,
        Err(()) => {
            return Err(terminal_unbound(
                M1AuthenticatedQueueRearmTerminalPhaseV1::WorkspaceRangeRebinding,
                lower,
                (witness, operations, custody, workspace_ranges, recipe, step),
            ));
        }
    };
    let bound_storage = storage
        .as_deref_mut()
        .and_then(|storage| storage.bound_rows.take());
    let bound_result = match bound_storage {
        Some(storage) => rebuild_bound_rows_with_storage(
            recipe.rows(),
            &custody.bound_rows,
            recipe.workspace_composition(),
            &workspace_ranges,
            &previous_capture,
            &retained_capture,
            storage,
        ),
        None => rebuild_bound_rows(
            recipe.rows(),
            &custody.bound_rows,
            recipe.workspace_composition(),
            &workspace_ranges,
            &previous_capture,
            &retained_capture,
        ),
    };
    let bound_rows = match bound_result {
        Ok(rows) => rows,
        Err(()) => {
            return Err(terminal_unbound(
                M1AuthenticatedQueueRearmTerminalPhaseV1::BoundRowRebuild,
                lower,
                (witness, operations, custody, workspace_ranges, recipe, step),
            ));
        }
    };
    let custody = M1PhysicalQueueBatchCustodyV1::from_rearm_parts(custody);
    let batch_result = match storage.as_deref_mut() {
        Some(storage) => build_m1_authenticated_rollover_packet_batch_with_storage_v1(
            &witness,
            &operations,
            recipe,
            bound_rows,
            custody,
            &mut storage.packet_batch,
        ),
        None => build_m1_authenticated_queue_packet_batch_v1(
            &witness,
            &operations,
            recipe,
            bound_rows,
            custody,
        ),
    };
    let batch = match batch_result {
        Ok(batch) => batch,
        Err(failure) => {
            let error = failure.error();
            let parts = failure.into_parts();
            return Err(terminal_unbound(
                M1AuthenticatedQueueRearmTerminalPhaseV1::PacketLowering,
                lower,
                (witness, operations, error, parts, step),
            ));
        }
    };
    if batch.shape() != shape {
        return Err(terminal_unbound(
            M1AuthenticatedQueueRearmTerminalPhaseV1::ShapeJoin,
            lower,
            (witness, operations, batch, step),
        ));
    }

    match (shape, batch) {
        (
            M1PhysicalFixedBatchShapeV1::TargetOnly,
            M1AuthenticatedQueuePacketBatchV1::TargetOnly(batch),
        ) => bind_authenticated_case::<M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1, _>(
            lower,
            *batch,
            witness,
            operations,
            step,
            expected_observation,
            None,
            M1AuthenticatedPhysicalQueueSessionV1::TargetOnly,
        ),
        (
            M1PhysicalFixedBatchShapeV1::PairedPrefill,
            M1AuthenticatedQueuePacketBatchV1::PairedPrefill(batch),
        ) => bind_authenticated_case::<M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1, _>(
            lower,
            *batch,
            witness,
            operations,
            step,
            expected_observation,
            None,
            M1AuthenticatedPhysicalQueueSessionV1::PairedPrefill,
        ),
        (
            M1PhysicalFixedBatchShapeV1::SpeculativeK4,
            M1AuthenticatedQueuePacketBatchV1::SpeculativeK4(batch),
        ) => bind_authenticated_case::<M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1, _>(
            lower,
            *batch,
            witness,
            operations,
            step,
            expected_observation,
            storage.as_mut().and_then(|storage| storage.phase.take()),
            M1AuthenticatedPhysicalQueueSessionV1::SpeculativeK4,
        ),
        (
            M1PhysicalFixedBatchShapeV1::SpeculativeK8,
            M1AuthenticatedQueuePacketBatchV1::SpeculativeK8(batch),
        ) => bind_authenticated_case::<M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1, _>(
            lower,
            *batch,
            witness,
            operations,
            step,
            expected_observation,
            storage.as_mut().and_then(|storage| storage.phase_k8.take()),
            M1AuthenticatedPhysicalQueueSessionV1::SpeculativeK8,
        ),
        (
            M1PhysicalFixedBatchShapeV1::SpeculativeK16,
            M1AuthenticatedQueuePacketBatchV1::SpeculativeK16(batch),
        ) => bind_authenticated_case::<M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1, _>(
            lower,
            *batch,
            witness,
            operations,
            step,
            expected_observation,
            storage
                .as_mut()
                .and_then(|storage| storage.phase_k16.take()),
            M1AuthenticatedPhysicalQueueSessionV1::SpeculativeK16,
        ),
        (_, batch) => Err(terminal_unbound(
            M1AuthenticatedQueueRearmTerminalPhaseV1::ShapeJoin,
            lower,
            (witness, operations, batch, step),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::authenticated_test_runtime::{ModelPreparedQueueV1, ModelQueueV1};
    use ferric_spec::Identity;

    #[test]
    fn finite_resident_scratch_binds_typed_phase_and_worst_case_tail_pages() {
        use ferric_build::{
            m1_step_workspace_requirements, plan_addressless_m1_step_workspace,
            AvailableM1StepWorkspace, DeclaredM1StepWorkspaceAllocation,
            M1StepWorkspaceDeclaration, M1StepWorkspacePlanOutcome,
        };
        use ferric_spec::Qwen3PlanBucket;
        let workspace = |selection, byte| {
            let requirements = m1_step_workspace_requirements(selection).unwrap();
            let available = AvailableM1StepWorkspace::new(M1StepWorkspaceDeclaration::new(
                selection,
                DeclaredM1StepWorkspaceAllocation::new(
                    Identity::new([byte; 32]),
                    requirements.allocation_byte_len(),
                    requirements.allocation_alignment(),
                ),
                requirements.ranges().to_vec().into_boxed_slice(),
            ));
            match plan_addressless_m1_step_workspace(selection, available) {
                M1StepWorkspacePlanOutcome::Planned(plan) => plan,
                M1StepWorkspacePlanOutcome::Rejected(_) => panic!("test workspace rejected"),
            }
        };
        for (bucket, width) in [
            (Qwen3PlanBucket::SpeculativeS1K4C8192, 4_usize),
            (Qwen3PlanBucket::SpeculativeS1K8C8192, 8),
            (Qwen3PlanBucket::SpeculativeS1K16C8192, 16),
        ] {
            let target = Qwen3PlanSelection {
                role: Qwen3ModelRole::Target8B,
                mode: Qwen3ExecutionMode::Speculative,
                bucket,
            };
            let plan =
                crate::authenticated_prefill_bootstrap::admitted_s1_t128_speculative_successor_v1(
                    target,
                )
                .unwrap();
            let plans = M1FullStepWorkspacePlans::speculative_round(
                workspace(plan.draft(), 1),
                workspace(target, 2),
            );
            let scratch =
                M1AuthenticatedResidentCompletionScratchV1::try_new_for_plans(1, &plans).unwrap();
            let publication = scratch.publication.as_ref().unwrap();
            assert_eq!(publication.selection, target);
            assert_eq!(publication.shape, plan.shape());
            assert_eq!(publication.phase.is_some(), width == 4);
            assert_eq!(publication.phase_k8.is_some(), width == 8);
            assert_eq!(publication.phase_k16.is_some(), width == 16);
            let target_capacity = scratch.target_page_leases[0].capacity();
            assert!(target_capacity >= (width + 1).div_ceil(16));
            for cursor in [127_usize, 128, 143, 144, 255, 256, 8_192 - width - 1] {
                let missing = (cursor + width + 1).div_ceil(16) - cursor.div_ceil(16);
                assert!(missing <= target_capacity, "cursor {cursor}, K{width}");
            }
        }
        let unsupported = Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Speculative,
            bucket: Qwen3PlanBucket::SpeculativeS8K4C8192,
        };
        assert!(M1AuthenticatedRearmPublicationHostStorageV1::try_new(unsupported).is_none());
    }

    #[derive(Debug)]
    struct ModelPreparedRearmV1 {
        prepared: ModelPreparedQueueV1,
        clean: bool,
    }

    impl M1PreparedRearmCloseEffectV1 for ModelPreparedRearmV1 {
        fn destroy_and_release_effect(self) -> Result<Box<dyn fmt::Debug>, Box<dyn fmt::Debug>> {
            self.prepared.destroy(self.clean);
            if self.clean {
                Ok(Box::new("released"))
            } else {
                Err(Box::new("quarantined"))
            }
        }
    }

    #[test]
    fn prepared_rearm_closure_executes_one_destroy_and_preserves_classification() {
        for clean in [true, false] {
            let queue = ModelQueueV1::new([], []);
            let custody = close_prepared_rearm_submission_core(
                ModelPreparedRearmV1 {
                    prepared: ModelPreparedQueueV1::new(queue.clone()),
                    clean,
                },
                "retained recipe and lineage",
            );
            assert_eq!(queue.snapshot().submits, 0);
            assert_eq!(queue.snapshot().destroys, 1);
            assert_eq!(
                matches!(custody, AuthenticatedSubmissionOpaqueCustodyV1::Released(_)),
                clean
            );
        }
    }

    #[test]
    fn authenticated_rearm_shape_kind_join_closes_all_five_shapes() {
        assert!(shape_kind_matches(
            M1PhysicalFixedBatchShapeV1::TargetOnly,
            M1FullStepWorkspaceInputKind::TargetOnly,
        ));
        assert!(shape_kind_matches(
            M1PhysicalFixedBatchShapeV1::PairedPrefill,
            M1FullStepWorkspaceInputKind::PairedPrefill,
        ));
        for shape in [
            M1PhysicalFixedBatchShapeV1::SpeculativeK4,
            M1PhysicalFixedBatchShapeV1::SpeculativeK8,
            M1PhysicalFixedBatchShapeV1::SpeculativeK16,
        ] {
            assert!(shape_kind_matches(
                shape,
                M1FullStepWorkspaceInputKind::SpeculativeRound,
            ));
        }
        assert!(!shape_kind_matches(
            M1PhysicalFixedBatchShapeV1::TargetOnly,
            M1FullStepWorkspaceInputKind::SpeculativeRound,
        ));
        assert!(!shape_kind_matches(
            M1PhysicalFixedBatchShapeV1::SpeculativeK4,
            M1FullStepWorkspaceInputKind::TargetOnly,
        ));
    }

    #[test]
    fn authenticated_rearm_terminal_phases_cover_every_effectful_boundary() {
        let phases = [
            M1AuthenticatedQueueRearmTerminalPhaseV1::WorkspaceContent,
            M1AuthenticatedQueueRearmTerminalPhaseV1::DraftWorkspaceReplacement,
            M1AuthenticatedQueueRearmTerminalPhaseV1::TargetWorkspaceReplacement,
            M1AuthenticatedQueueRearmTerminalPhaseV1::DirectDiagnosticChoiceReplacement,
            M1AuthenticatedQueueRearmTerminalPhaseV1::SpeculativeDraftChoiceReplacement,
            M1AuthenticatedQueueRearmTerminalPhaseV1::SpeculativeTargetChoiceReplacement,
            M1AuthenticatedQueueRearmTerminalPhaseV1::WorkspaceRangeRebinding,
            M1AuthenticatedQueueRearmTerminalPhaseV1::BoundRowRebuild,
            M1AuthenticatedQueueRearmTerminalPhaseV1::PacketLowering,
            M1AuthenticatedQueueRearmTerminalPhaseV1::ShapeJoin,
            M1AuthenticatedQueueRearmTerminalPhaseV1::QueueBind,
            M1AuthenticatedQueueRearmTerminalPhaseV1::QueueObservation,
        ];
        assert_eq!(phases.len(), 12);
    }

    #[test]
    fn authenticated_rearm_kv_arena_join_rejects_role_and_presence_drift() {
        let target = Identity::new([1; 32]);
        let draft = Identity::new([2; 32]);
        for kind in [
            M1FullStepWorkspaceInputKind::PairedPrefill,
            M1FullStepWorkspaceInputKind::SpeculativeRound,
        ] {
            assert_eq!(
                validate_kv_arena_ids(kind, target, Some(draft), target, draft),
                Ok(()),
            );
            assert_eq!(
                validate_kv_arena_ids(kind, Identity::new([3; 32]), Some(draft), target, draft,),
                Err(M1AuthenticatedQueueRearmPreflightErrorV1::TargetKvArena),
            );
            assert_eq!(
                validate_kv_arena_ids(kind, target, None, target, draft),
                Err(M1AuthenticatedQueueRearmPreflightErrorV1::DraftKvArena),
            );
            assert_eq!(
                validate_kv_arena_ids(kind, target, Some(Identity::new([4; 32])), target, draft,),
                Err(M1AuthenticatedQueueRearmPreflightErrorV1::DraftKvArena),
            );
        }
        assert_eq!(
            validate_kv_arena_ids(
                M1FullStepWorkspaceInputKind::TargetOnly,
                target,
                None,
                target,
                draft,
            ),
            Ok(()),
        );
        assert_eq!(
            validate_kv_arena_ids(
                M1FullStepWorkspaceInputKind::TargetOnly,
                target,
                Some(draft),
                target,
                draft,
            ),
            Err(M1AuthenticatedQueueRearmPreflightErrorV1::DraftKvArena),
        );
    }

    #[test]
    fn authenticated_rearm_rejects_direct_and_preserves_speculative_capture() {
        assert!(diagnostic_capture_is_supported(false, false));
        assert!(!diagnostic_capture_is_supported(true, false));
        assert!(diagnostic_capture_is_supported(false, true));
        assert!(!diagnostic_capture_is_supported(true, true));
    }

    #[test]
    fn public_submission_failure_truthfully_reports_release_and_quarantine() {
        for (retained, released) in [
            (
                AuthenticatedSubmissionOpaqueCustodyV1::Released(Box::new("clean release")),
                true,
            ),
            (
                AuthenticatedSubmissionOpaqueCustodyV1::Quarantined(Box::new("terminal custody")),
                false,
            ),
        ] {
            let failure = authenticated_classified_submission_failure(
                M1AuthenticatedLongLivedQueueRearmSubmissionPhaseV1::QueueSubmit,
                retained,
            );
            assert_eq!(failure.queue_released(), released);
            assert!(failure.engine_quarantined());
            assert!(failure.retains_custody());
            let debug = format!("{failure:?}");
            assert!(!debug.contains("clean release"));
            assert!(!debug.contains("terminal custody"));
        }
    }

    #[test]
    fn bounded_rearm_wait_surface_requires_the_scheduler_engine() {
        type WaitFor = fn(
            M1AuthenticatedRearmedPublishedQueueV1,
            u32,
            &mut Engine<1>,
        ) -> Result<
            M1AuthenticatedRearmedCompletedQueueV1,
            Box<M1AuthenticatedRearmedQueueProgressFailureV1>,
        >;
        type TimeoutObservation = for<'a> fn(
            &'a M1AuthenticatedRearmedQueueProgressFailureV1,
        )
            -> Option<&'a Gfx942TimeoutExecutionObservationV1>;

        let _: WaitFor = M1AuthenticatedRearmedPublishedQueueV1::wait_for::<1>;
        let _: TimeoutObservation =
            M1AuthenticatedRearmedQueueProgressFailureV1::timeout_observation;
    }
}
