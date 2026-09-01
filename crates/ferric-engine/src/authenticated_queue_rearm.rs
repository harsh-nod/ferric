//! Authenticated same-native-queue rebind after semantic completion.
//!
//! This is the effectful core used by the released-step rearm lifecycle. It
//! accepts only a detached post-readback authenticated queue, exact fresh
//! workspace images, and an unchanged-structure physical recipe. Program
//! indices remain private to the retained authenticated witness.

#![expect(
    dead_code,
    reason = "the released-step authenticated lifecycle is landing immediately after this core"
)]

use core::fmt;

use fe2o3_host::{
    AuthenticatedServiceQueueDataUpdateFailureV1, AuthenticatedServiceQueueSessionV1,
    AuthenticatedServiceQueueUnboundSessionV1,
};
use fe2o3_kfd::{ComputeAqlQueueObservationV1, Gfx942DeviceContentDescriptorV1};
use fe2o3_service_host::{DeviceWorkspaceRoleV1, ServiceDeviceDispatchRangeV1};
use ferric_build::AddresslessM1StepWorkspacePlan;
use ferric_spec::Qwen3ModelRole;

use crate::m1_queue_rearm::{
    append_workspace_ranges, member_layout, rebuild_unchanged_capture_bound_rows,
};
use crate::physical_fixed_batch::{
    build_m1_authenticated_queue_packet_batch_v1, validate_authenticated_operation_plan_v1,
    M1AuthenticatedQueuePacketBatchCaseV1, M1AuthenticatedQueuePacketBatchV1,
};
use crate::step_workspace_subleases::{
    bind_authenticated_queue_replaced_m1_step_workspace,
    M1AuthenticatedQueueReplacedWorkspaceBindingFailureV1,
};
use crate::{
    AddresslessM1PhysicalBufferRecipeV1, BoundM1StepWorkspaceSubleases,
    DeclaredOperationKernelPlan, M1AuthenticatedPhysicalQueuePhaseCaseV1,
    M1AuthenticatedPhysicalQueueSessionV1, M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1,
    M1FullStepWorkspaceImagesV1, M1FullStepWorkspaceInputKind, M1FullStepWorkspacePlans,
    M1FullStepWorkspaceRole, M1FullStepWorkspaceSubleaseOwners, M1InitializedWorkspaceSlotV1,
    M1PhysicalFixedBatchBuildErrorV1, M1PhysicalFixedBatchShapeV1, M1PhysicalQueueBatchCustodyV1,
    M1PreparedScheduledWorkspaceImagesV1, M1PrepublicationStepCustodyV1,
    M1_PAIRED_PREFILL_FIXED_BATCH_PACKETS_V1, M1_SPECULATIVE_K16_FIXED_BATCH_PACKETS_V1,
    M1_SPECULATIVE_K4_FIXED_BATCH_PACKETS_V1, M1_SPECULATIVE_K8_FIXED_BATCH_PACKETS_V1,
    M1_TARGET_ONLY_FIXED_BATCH_PACKETS_V1,
};

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
}

#[must_use = "terminal authenticated rearm custody must remain retained"]
pub(crate) struct M1AuthenticatedQueueRearmTerminalV1 {
    phase: M1AuthenticatedQueueRearmTerminalPhaseV1,
    retained: Box<dyn fmt::Debug>,
}

impl fmt::Debug for M1AuthenticatedQueueRearmTerminalV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedQueueRearmTerminalV1")
            .field("phase", &self.phase)
            .field("retained", &self.retained)
            .finish()
    }
}

impl M1AuthenticatedQueueRearmTerminalV1 {
    pub(crate) const fn phase(&self) -> M1AuthenticatedQueueRearmTerminalPhaseV1 {
        self.phase
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
        retained: Box::new(retained),
    }))
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
        | M1FullStepWorkspaceInputKind::SpeculativeRound => Some(retained_draft),
    };
    if fresh_draft != expected_draft {
        return Err(M1AuthenticatedQueueRearmPreflightErrorV1::DraftKvArena);
    }
    Ok(())
}

const fn diagnostic_capture_is_supported(direct: bool, speculative: bool) -> bool {
    !direct && !speculative
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
enum AuthenticatedWorkspaceReplacementFailureV1<const N: usize> {
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

fn replace_authenticated_workspace<const N: usize>(
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

fn descriptor(
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
    wrap: F,
) -> Result<M1AuthenticatedPhysicalQueueSessionV1, M1AuthenticatedQueueRearmFailureV1>
where
    F: FnOnce(
        M1AuthenticatedPhysicalQueuePhaseCaseV1<AuthenticatedServiceQueueSessionV1<N>>,
    ) -> M1AuthenticatedPhysicalQueueSessionV1,
{
    let (packets, custody) = batch.into_parts();
    let lower = match lower.bind_retained(packets) {
        Ok(lower) => lower,
        Err(failure) => {
            return Err(terminal(
                M1AuthenticatedQueueRearmTerminalPhaseV1::QueueBind,
                (failure, witness, operations, custody, step),
            ));
        }
    };
    if lower.observation() != expected_observation {
        return Err(terminal(
            M1AuthenticatedQueueRearmTerminalPhaseV1::QueueObservation,
            (lower, witness, operations, custody, step),
        ));
    }
    Ok(wrap(
        M1AuthenticatedPhysicalQueuePhaseCaseV1::from_queue_rearm(
            lower, witness, operations, custody, step,
        ),
    ))
}

pub(crate) fn rearm_m1_authenticated_detached_queue_v1(
    detached: M1AuthenticatedPhysicalReadbackDetachedQueueSessionV1,
    prepared: M1PreparedScheduledWorkspaceImagesV1,
    recipe: AddresslessM1PhysicalBufferRecipeV1,
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
    let expected_observation = lower.observation();
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
                            return Err(terminal(
                                M1AuthenticatedQueueRearmTerminalPhaseV1::WorkspaceContent,
                                (
                                    lower, witness, operations, custody, plan, bytes, recipe, step,
                                ),
                            ));
                        }
                    };
                let (lower, target, ranges) = match replace_authenticated_workspace(
                    lower, old_target, *plan, bytes, descriptor,
                ) {
                    Ok(replaced) => replaced,
                    Err(failure) => {
                        let _ = failure.retained_owner_count();
                        return Err(terminal(
                            M1AuthenticatedQueueRearmTerminalPhaseV1::TargetWorkspaceReplacement,
                            (failure, witness, operations, custody, recipe, step),
                        ));
                    }
                };
                let mut workspace_ranges = Vec::new();
                if workspace_ranges.try_reserve_exact(ranges.len()).is_err() {
                    return Err(terminal(
                        M1AuthenticatedQueueRearmTerminalPhaseV1::WorkspaceRangeRebinding,
                        (
                            lower, witness, operations, custody, target, ranges, recipe, step,
                        ),
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
                        return Err(terminal(
                            M1AuthenticatedQueueRearmTerminalPhaseV1::WorkspaceContent,
                            (
                                lower,
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
                        return Err(terminal(
                            M1AuthenticatedQueueRearmTerminalPhaseV1::WorkspaceContent,
                            (
                                lower,
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
                        return Err(terminal(
                            M1AuthenticatedQueueRearmTerminalPhaseV1::DraftWorkspaceReplacement,
                            (
                                failure,
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
                        return Err(terminal(
                            M1AuthenticatedQueueRearmTerminalPhaseV1::TargetWorkspaceReplacement,
                            (
                                failure,
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
                let mut workspace_ranges = Vec::new();
                if workspace_ranges
                    .try_reserve_exact(draft_ranges.len() + target_ranges.len())
                    .is_err()
                {
                    return Err(terminal(
                        M1AuthenticatedQueueRearmTerminalPhaseV1::WorkspaceRangeRebinding,
                        (
                            lower,
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
                        return Err(terminal(
                            M1AuthenticatedQueueRearmTerminalPhaseV1::WorkspaceContent,
                            (
                                lower,
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
                        return Err(terminal(
                            M1AuthenticatedQueueRearmTerminalPhaseV1::WorkspaceContent,
                            (
                                lower,
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
                        return Err(terminal(
                            M1AuthenticatedQueueRearmTerminalPhaseV1::DraftWorkspaceReplacement,
                            (
                                failure,
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
                        return Err(terminal(
                            M1AuthenticatedQueueRearmTerminalPhaseV1::TargetWorkspaceReplacement,
                            (
                                failure,
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
                let mut workspace_ranges = Vec::new();
                if workspace_ranges
                    .try_reserve_exact(draft_ranges.len() + target_ranges.len())
                    .is_err()
                {
                    return Err(terminal(
                        M1AuthenticatedQueueRearmTerminalPhaseV1::WorkspaceRangeRebinding,
                        (
                            lower,
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
                return Err(terminal(
                    M1AuthenticatedQueueRearmTerminalPhaseV1::ShapeJoin,
                    (
                        lower, witness, operations, custody, plans, images, recipe, step,
                    ),
                ));
            }
        };

    custody.workspace_owners = workspace_owners;
    let bound_rows = match rebuild_unchanged_capture_bound_rows(
        recipe.rows(),
        &custody.bound_rows,
        recipe.workspace_composition(),
        &workspace_ranges,
        &custody.completion_output,
    ) {
        Ok(rows) => rows,
        Err(()) => {
            return Err(terminal(
                M1AuthenticatedQueueRearmTerminalPhaseV1::BoundRowRebuild,
                (
                    lower,
                    witness,
                    operations,
                    custody,
                    workspace_ranges,
                    recipe,
                    step,
                ),
            ));
        }
    };
    let custody = M1PhysicalQueueBatchCustodyV1::from_rearm_parts(custody);
    let batch = match build_m1_authenticated_queue_packet_batch_v1(
        &witness,
        &operations,
        recipe,
        bound_rows,
        custody,
    ) {
        Ok(batch) => batch,
        Err(failure) => {
            let error = failure.error();
            let parts = failure.into_parts();
            return Err(terminal(
                M1AuthenticatedQueueRearmTerminalPhaseV1::PacketLowering,
                (lower, witness, operations, error, parts, step),
            ));
        }
    };
    if batch.shape() != shape {
        return Err(terminal(
            M1AuthenticatedQueueRearmTerminalPhaseV1::ShapeJoin,
            (lower, witness, operations, batch, step),
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
            |case| M1AuthenticatedPhysicalQueueSessionV1::TargetOnly(Box::new(case)),
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
            |case| M1AuthenticatedPhysicalQueueSessionV1::PairedPrefill(Box::new(case)),
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
            |case| M1AuthenticatedPhysicalQueueSessionV1::SpeculativeK4(Box::new(case)),
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
            |case| M1AuthenticatedPhysicalQueueSessionV1::SpeculativeK8(Box::new(case)),
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
            |case| M1AuthenticatedPhysicalQueueSessionV1::SpeculativeK16(Box::new(case)),
        ),
        (_, batch) => Err(terminal(
            M1AuthenticatedQueueRearmTerminalPhaseV1::ShapeJoin,
            (lower, witness, operations, batch, step),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferric_spec::Identity;

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
            M1AuthenticatedQueueRearmTerminalPhaseV1::WorkspaceRangeRebinding,
            M1AuthenticatedQueueRearmTerminalPhaseV1::BoundRowRebuild,
            M1AuthenticatedQueueRearmTerminalPhaseV1::PacketLowering,
            M1AuthenticatedQueueRearmTerminalPhaseV1::ShapeJoin,
            M1AuthenticatedQueueRearmTerminalPhaseV1::QueueBind,
            M1AuthenticatedQueueRearmTerminalPhaseV1::QueueObservation,
        ];
        assert_eq!(phases.len(), 9);
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
    fn generic_authenticated_rearm_rejects_diagnostic_capture_until_reset_exists() {
        assert!(diagnostic_capture_is_supported(false, false));
        assert!(!diagnostic_capture_is_supported(true, false));
        assert!(!diagnostic_capture_is_supported(false, true));
        assert!(!diagnostic_capture_is_supported(true, true));
    }
}
