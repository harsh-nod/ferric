//! Structural draft maintenance. No authenticated program capability is issued here.

use super::*;
use crate::m1_serving_physical_operations::M1StructuralDraftCatchupPendingV1;

#[derive(Debug)]
pub(crate) struct StructuralDraftCatchupScratchV1 {
    parent: Qwen3PlanSelection,
    draft_inputs:
        Option<crate::authenticated_resident_session::M1AuthenticatedResidentRoleInputStorageV1>,
    completion_inputs:
        Option<crate::authenticated_resident_session::M1AuthenticatedResidentRoleInputStorageV1>,
    write: crate::device_cache::M1DraftCatchupKvWriteHostStorageV1,
    page_leases: Vec<crate::DeviceKvPageLease>,
    reservations: Vec<crate::PendingDeviceKvStepWrite>,
    draft_table: Option<crate::kv_workspace_authority::M1KvWorkspaceTableHostStorageV1>,
    completion_page_indices: Option<Box<[u32]>>,
    workspace_images: Option<crate::m1_prepublication::M1FullStepWorkspaceImageHostStorageV1>,
    readback:
        Option<crate::physical_queue_lifecycle::M1StructuralDraftCatchupReadbackHostStorageV1>,
}

impl StructuralDraftCatchupScratchV1 {
    pub(crate) fn try_new(
        parent: Qwen3PlanSelection,
        plans: &M1FullStepWorkspacePlans,
    ) -> Option<Self> {
        let M1FullStepWorkspacePlans::DraftCatchup {
            parent: declared,
            draft_decode,
            completion,
        } = plans
        else {
            return None;
        };
        let decode = crate::M1StepDispatchIntent::DraftCatchup(parent).completion_selection();
        if *declared != parent
            || completion.selection() != decode
            || draft_decode.selection()
                != (Qwen3PlanSelection {
                    role: ferric_spec::Qwen3ModelRole::Draft06B,
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
            draft_inputs: Some(crate::authenticated_resident_session::M1AuthenticatedResidentRoleInputStorageV1::try_new(1)?),
            completion_inputs: Some(crate::authenticated_resident_session::M1AuthenticatedResidentRoleInputStorageV1::try_new(1)?),
            write: crate::device_cache::M1DraftCatchupKvWriteHostStorageV1::try_new()?,
            page_leases,
            reservations,
            draft_table: Some(crate::kv_workspace_authority::M1KvWorkspaceTableHostStorageV1::try_new(1)?),
            completion_page_indices: Some(completion_page_indices.into_boxed_slice()),
            workspace_images: Some(crate::m1_prepublication::M1FullStepWorkspaceImageHostStorageV1::try_new(plans)?),
            readback: Some(crate::physical_queue_lifecycle::M1StructuralDraftCatchupReadbackHostStorageV1::try_new(parent)?),
        })
    }

    pub(crate) fn take_readback(
        &mut self,
    ) -> Option<crate::physical_queue_lifecycle::M1StructuralDraftCatchupReadbackHostStorageV1>
    {
        self.readback.take()
    }
}

#[derive(Debug)]
pub(crate) struct StructuralDraftCatchupFailureV1 {
    retained: Box<dyn fmt::Debug>,
}

impl StructuralDraftCatchupFailureV1 {
    fn new(retained: impl fmt::Debug + 'static) -> Self {
        Self {
            retained: Box::new(retained),
        }
    }
}

impl fmt::Display for StructuralDraftCatchupFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "structural maintenance retains failed custody: {:?}",
            self.retained
        )
    }
}

impl std::error::Error for StructuralDraftCatchupFailureV1 {}

#[derive(Debug)]
pub(crate) struct StructuralSavedBindingsV1 {
    parent: Qwen3PlanSelection,
    request: RequestId,
    coordinator: crate::speculative_generation_loop::M1SpeculativeCoordinatorIdentityV1,
    source_rows: Box<[M1PhysicalBufferRecipeRowV1]>,
    bound_rows: Box<[M1BoundPhysicalBufferRowV1]>,
}

#[derive(Debug)]
pub(crate) struct StructuralDraftCatchupRestoreCustodyV1 {
    pub(super) completed:
        crate::m1_serving_physical_operations::M1StructuralDraftCatchupCompletedV1,
    initialized: crate::device_cache::InertInitializedDeviceKvStepWrite,
    saved: StructuralSavedBindingsV1,
    entered: M1QueueRolloverObservationV1,
}

struct StructuralTransitionPartsV1 {
    lower: ServiceQueueUnboundSessionV1,
    custody: M1PhysicalQueueBatchRearmPartsV1,
    step: M1PrepublicationStepCustodyV1,
    images: Box<[crate::M1PhysicalKernargImageV1]>,
    saved: StructuralSavedBindingsV1,
}

#[allow(clippy::too_many_arguments)]
fn prepare_structural_queue_transition(
    lower: ServiceQueueUnboundSessionV1,
    custody: M1PhysicalQueueBatchCustodyV1,
    prepared: M1PreparedScheduledWorkspaceImagesV1,
    recipe: AddresslessM1PhysicalBufferRecipeV1,
    authorized: &M1StructuralDraftCatchupPendingV1,
    saved: Option<StructuralSavedBindingsV1>,
    storage: &mut StructuralTransitionStorageV1,
) -> Result<StructuralTransitionPartsV1, StructuralDraftCatchupFailureV1> {
    let parent = authorized.parent();
    let intent = if storage.entering {
        crate::M1StepDispatchIntent::DraftCatchup(parent)
    } else {
        crate::M1StepDispatchIntent::SpeculativeRound(parent)
    };
    let expected_epoch = if storage.entering {
        Some(authorized.epoch().value())
    } else {
        authorized.epoch().value().checked_add(1)
    };
    let expected_generation = if storage.entering {
        Some(authorized.prior_dispatch_generation())
    } else {
        authorized.prior_dispatch_generation().checked_add(1)
    };
    let saved_matches = match &saved {
        None => storage.entering,
        Some(saved) => {
            !storage.entering
                && saved.parent == parent
                && saved.request == authorized.request()
                && saved.coordinator == authorized.coordinator_identity()
        }
    };
    if storage.parent != parent
        || custody.selection() != parent
        || !saved_matches
        || expected_generation != Some(lower.detached_dispatch_generation())
        || expected_epoch != Some(prepared.step().scheduled_dispatch().epoch().value())
        || prepared.step().scheduled_dispatch().member_count() != 1
        || prepared.step().scheduled_dispatch().member(0) != Some(authorized.request())
        || recipe.workspace_composition().dispatch_plan().intent() != intent
        || recipe.workspace_composition().workspace_plans() != prepared.plans()
        || recipe.requires_future_materialization()
        || !storage
            .rows
            .as_ref()
            .is_some_and(|rows| rows.has_capacity_for(recipe.rows()))
        || storage.draft_owner.is_none()
        || (storage.entering && storage.completion_owner.is_none())
        || (!storage.entering && storage.speculative_owner.is_none())
        || (!storage.entering && !storage.has_restore_phase())
        || !storage.workspace_ranges.is_empty()
        || storage.workspace_ranges.capacity()
            < crate::M1_DRAFT_STEP_WORKSPACE_SUBLEASE_COUNT_V1
                + crate::M1_TARGET_SPECULATIVE_STEP_WORKSPACE_SUBLEASE_COUNT_V1
        || (storage.entering
            && !custody
                .completion_output()
                .can_retarget_exact_s1_speculative_to_draft_catchup(parent))
        || (!storage.entering
            && custody.completion_output().draft_catchup_parent_selection() != Some(parent))
        || (!storage.entering
            && !storage
                .diagnostic_reset
                .as_ref()
                .is_some_and(|reset| reset.accepts_parent(parent)))
    {
        return Err(StructuralDraftCatchupFailureV1::new((
            "structural queue transition preflight",
            lower,
            custody,
            prepared,
            recipe,
            saved,
        )));
    }
    let custody = custody.into_rearm_parts();
    let (plans, images, step) = prepared.into_rearm_parts();
    let (lower, mut custody) =
        match replace_structural_workspaces(lower, custody, plans, images, storage) {
            Ok(replaced) => replaced,
            Err(error) => {
                return Err(StructuralDraftCatchupFailureV1::new((
                    error, step, recipe, saved,
                )))
            }
        };
    let output = if storage.entering {
        custody
            .completion_output
            .retarget_exact_s1_speculative_to_draft_catchup(parent)
    } else {
        custody
            .completion_output
            .restore_exact_s1_speculative_after_draft_catchup(parent)
    };
    let output = match output {
        Ok(output) => output,
        Err(output) => {
            return Err(StructuralDraftCatchupFailureV1::new((
                (
                    "completion phase retarget",
                    lower,
                    output,
                    custody.catalog_id,
                    custody.selection,
                    custody.physical_recipe,
                    custody.workspace_composition,
                ),
                (
                    custody.workspace_owners,
                    custody.partitioned_memory,
                    custody.source_rows,
                    custody.bound_rows,
                    custody.retired_rollover_custody,
                    step,
                    recipe,
                    saved,
                ),
            )))
        }
    };
    let (lower, output) = if storage.entering {
        (lower, output)
    } else {
        match reset_retained_diagnostic_capture_core(lower, output, storage.diagnostic_reset.take())
        {
            Ok(reset) => reset,
            Err(error) => {
                return Err(StructuralDraftCatchupFailureV1::new((
                    (
                        error,
                        custody.catalog_id,
                        custody.selection,
                        custody.physical_recipe,
                        custody.workspace_composition,
                        custody.workspace_owners,
                    ),
                    (
                        custody.partitioned_memory,
                        custody.source_rows,
                        custody.bound_rows,
                        custody.retired_rollover_custody,
                        step,
                        recipe,
                        saved,
                    ),
                )))
            }
        }
    };
    custody.completion_output = output;
    let retained = match retained_host_capture_ranges(&custody.completion_output) {
        Ok(retained) => retained,
        Err(()) => {
            return Err(StructuralDraftCatchupFailureV1::new((
                "retained capture ranges",
                lower,
                custody,
                step,
                recipe,
                saved,
            )))
        }
    };
    let (old_source, old_bound) = match &saved {
        Some(saved) => (saved.source_rows.as_ref(), saved.bound_rows.as_ref()),
        None => (custody.source_rows.as_ref(), custody.bound_rows.as_ref()),
    };
    let bound_rows = match build_rollover_bound_rows_with_storage(
        recipe.rows(),
        old_source,
        old_bound,
        recipe.workspace_composition(),
        &storage.workspace_ranges,
        &retained,
        storage.rows.take().expect("preflight bound-row storage"),
    ) {
        Ok(rows) => rows,
        Err(()) => {
            return Err(StructuralDraftCatchupFailureV1::new((
                "structural bound rows",
                lower,
                custody,
                step,
                recipe,
                saved,
            )))
        }
    };
    let (kernargs, workspace_composition, source_rows) = recipe.into_parts();
    let (physical_recipe, images) = kernargs.into_parts();
    let prior_source = core::mem::replace(&mut custody.source_rows, source_rows);
    let prior_bound = core::mem::replace(&mut custody.bound_rows, bound_rows);
    custody.physical_recipe = physical_recipe;
    custody.workspace_composition = workspace_composition;
    let saved = saved.unwrap_or_else(|| StructuralSavedBindingsV1 {
        parent,
        request: authorized.request(),
        coordinator: authorized.coordinator_identity(),
        source_rows: prior_source,
        bound_rows: prior_bound,
    });
    Ok(StructuralTransitionPartsV1 {
        lower,
        custody,
        step,
        images,
        saved,
    })
}

type DraftOwner =
    BoundM1StepWorkspaceSubleases<{ crate::M1_DRAFT_STEP_WORKSPACE_SUBLEASE_COUNT_V1 }>;
type CompletionOwner =
    BoundM1StepWorkspaceSubleases<{ crate::M1_TARGET_STEP_WORKSPACE_SUBLEASE_COUNT_V1 }>;
type SpeculativeOwner = BoundM1StepWorkspaceSubleases<
    { crate::M1_TARGET_SPECULATIVE_STEP_WORKSPACE_SUBLEASE_COUNT_V1 },
>;
type RestorePhase<const N: usize> =
    crate::physical_queue_lifecycle::M1PhysicalQueuePhaseCaseV1<ServiceQueueSessionV1<N>>;

pub(crate) struct StructuralTransitionStorageV1 {
    parent: Qwen3PlanSelection,
    entering: bool,
    workspace_ranges: Vec<FreshWorkspaceRangeV1>,
    rows: Option<M1RolloverBoundRowsHostStorageV1>,
    draft_owner: Option<Box<core::mem::MaybeUninit<DraftOwner>>>,
    completion_owner: Option<Box<core::mem::MaybeUninit<CompletionOwner>>>,
    speculative_owner: Option<Box<core::mem::MaybeUninit<SpeculativeOwner>>>,
    diagnostic_reset: Option<crate::completion_output::M1AuthenticatedDiagnosticResetHostStorageV1>,
    packet_inputs: Vec<LowerBatchInputV1>,
    packet_buffers: Vec<Vec<ServiceFixedDispatchBufferV1>>,
    phase_k4: Option<Box<core::mem::MaybeUninit<RestorePhase<2242>>>>,
    phase_k8: Option<Box<core::mem::MaybeUninit<RestorePhase<3938>>>>,
    phase_k16: Option<Box<core::mem::MaybeUninit<RestorePhase<7330>>>>,
}

impl fmt::Debug for StructuralTransitionStorageV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StructuralTransitionStorageV1")
            .field("parent", &self.parent)
            .field("entering", &self.entering)
            .field("packet_capacity", &self.packet_inputs.capacity())
            .finish_non_exhaustive()
    }
}

impl StructuralTransitionStorageV1 {
    fn has_restore_phase(&self) -> bool {
        match self.parent.bucket {
            ferric_spec::Qwen3PlanBucket::SpeculativeS1K4C8192 => self.phase_k4.is_some(),
            ferric_spec::Qwen3PlanBucket::SpeculativeS1K8C8192 => self.phase_k8.is_some(),
            ferric_spec::Qwen3PlanBucket::SpeculativeS1K16C8192 => self.phase_k16.is_some(),
            _ => false,
        }
    }

    pub(crate) fn try_new(
        parent: Qwen3PlanSelection,
        entering: bool,
        recipe: &AddresslessM1PhysicalBufferRecipeV1,
    ) -> Option<Self> {
        let intent = if entering {
            crate::M1StepDispatchIntent::DraftCatchup(parent)
        } else {
            crate::M1StepDispatchIntent::SpeculativeRound(parent)
        };
        if recipe.workspace_composition().dispatch_plan().intent() != intent {
            return None;
        }
        let mut workspace_ranges = Vec::new();
        workspace_ranges
            .try_reserve_exact(
                crate::M1_DRAFT_STEP_WORKSPACE_SUBLEASE_COUNT_V1
                    + crate::M1_TARGET_SPECULATIVE_STEP_WORKSPACE_SUBLEASE_COUNT_V1,
            )
            .ok()?;
        let mut packet_inputs = Vec::new();
        packet_inputs.try_reserve_exact(recipe.rows().len()).ok()?;
        let mut packet_buffers = Vec::new();
        packet_buffers.try_reserve_exact(recipe.rows().len()).ok()?;
        for row in recipe.rows() {
            let mut buffers = Vec::new();
            buffers.try_reserve_exact(row.buffers().len()).ok()?;
            packet_buffers.push(buffers);
        }
        Some(Self {
            parent,
            entering,
            workspace_ranges,
            rows: Some(M1RolloverBoundRowsHostStorageV1::try_new(
                recipe.rows().len(),
                16,
            )?),
            draft_owner: Some(Box::new_uninit()),
            completion_owner: entering.then(Box::new_uninit),
            speculative_owner: (!entering).then(Box::new_uninit),
            diagnostic_reset: if entering {
                None
            } else {
                Some(
                    crate::completion_output::M1AuthenticatedDiagnosticResetHostStorageV1::try_new(
                        parent,
                    )?,
                )
            },
            packet_inputs,
            packet_buffers,
            phase_k4: (!entering
                && parent.bucket == ferric_spec::Qwen3PlanBucket::SpeculativeS1K4C8192)
                .then(Box::new_uninit),
            phase_k8: (!entering
                && parent.bucket == ferric_spec::Qwen3PlanBucket::SpeculativeS1K8C8192)
                .then(Box::new_uninit),
            phase_k16: (!entering
                && parent.bucket == ferric_spec::Qwen3PlanBucket::SpeculativeS1K16C8192)
                .then(Box::new_uninit),
        })
    }
}

fn replace_structural_workspaces(
    lower: ServiceQueueUnboundSessionV1,
    mut custody: M1PhysicalQueueBatchRearmPartsV1,
    plans: M1FullStepWorkspacePlans,
    images: M1FullStepWorkspaceImagesV1,
    storage: &mut StructuralTransitionStorageV1,
) -> Result<
    (
        ServiceQueueUnboundSessionV1,
        M1PhysicalQueueBatchRearmPartsV1,
    ),
    StructuralDraftCatchupFailureV1,
> {
    let pair = match (plans, images) {
        (
            M1FullStepWorkspacePlans::DraftCatchup {
                parent,
                draft_decode,
                completion,
            },
            M1FullStepWorkspaceImagesV1::DraftCatchup {
                draft_decode: draft_bytes,
                completion: target_bytes,
            },
        ) if storage.entering && parent == storage.parent => (
            draft_decode,
            completion,
            draft_bytes,
            target_bytes,
            M1InitializedWorkspaceSlotV1::TargetOnlyTarget,
        ),
        (
            M1FullStepWorkspacePlans::SpeculativeRound {
                draft_decode,
                target_speculative,
            },
            M1FullStepWorkspaceImagesV1::SpeculativeRound {
                draft_decode: draft_bytes,
                target_speculative: target_bytes,
            },
        ) if !storage.entering && target_speculative.selection() == storage.parent => (
            draft_decode,
            target_speculative,
            draft_bytes,
            target_bytes,
            M1InitializedWorkspaceSlotV1::SpeculativeTarget,
        ),
        (plans, images) => {
            return Err(StructuralDraftCatchupFailureV1::new((
                "workspace transition kind",
                lower,
                custody,
                plans,
                images,
            )))
        }
    };
    let (draft_plan, target_plan, draft_bytes, target_bytes, target_slot) = pair;
    let draft_descriptor = match crate::m1_step_workspace_content_descriptor_v1(
        M1InitializedWorkspaceSlotV1::SpeculativeDraftDecode,
        &draft_bytes,
    ) {
        Ok(descriptor) => descriptor,
        Err(error) => {
            return Err(StructuralDraftCatchupFailureV1::new((
                error,
                lower,
                custody,
                draft_plan,
                target_plan,
                draft_bytes,
                target_bytes,
            )))
        }
    };
    let target_descriptor =
        match crate::m1_step_workspace_content_descriptor_v1(target_slot, &target_bytes) {
            Ok(descriptor) => descriptor,
            Err(error) => {
                return Err(StructuralDraftCatchupFailureV1::new((
                    error,
                    lower,
                    custody,
                    draft_plan,
                    target_plan,
                    draft_bytes,
                    target_bytes,
                )))
            }
        };
    let old_draft = match &custody.workspace_owners {
        M1FullStepWorkspaceSubleaseOwners::SpeculativeRound { draft_decode, .. }
            if storage.entering =>
        {
            draft_decode
        }
        M1FullStepWorkspaceSubleaseOwners::DraftCatchup { draft_decode, .. }
            if !storage.entering =>
        {
            draft_decode
        }
        _ => {
            return Err(StructuralDraftCatchupFailureV1::new((
                "workspace predecessor",
                lower,
                custody,
                draft_plan,
                target_plan,
                draft_bytes,
                target_bytes,
            )))
        }
    };
    let (lower, draft, draft_ranges): (_, DraftOwner, _) = match replace_rollover_workspace(
        lower,
        old_draft,
        *draft_plan,
        draft_bytes,
        draft_descriptor,
    ) {
        Ok(replaced) => replaced,
        Err(error) => {
            return Err(StructuralDraftCatchupFailureV1::new((
                error,
                custody,
                target_plan,
                target_bytes,
            )))
        }
    };
    let owners = match &custody.workspace_owners {
        M1FullStepWorkspaceSubleaseOwners::SpeculativeRound {
            target_speculative, ..
        } if storage.entering => {
            let (lower, target, target_ranges): (_, CompletionOwner, _) =
                match replace_rollover_workspace(
                    lower,
                    target_speculative,
                    *target_plan,
                    target_bytes,
                    target_descriptor,
                ) {
                    Ok(replaced) => replaced,
                    Err(error) => {
                        return Err(StructuralDraftCatchupFailureV1::new((
                            error,
                            custody,
                            draft,
                            draft_ranges,
                        )))
                    }
                };
            append_workspace_ranges(
                &mut storage.workspace_ranges,
                M1FullStepWorkspaceRole::Draft,
                &draft,
                draft_ranges,
            );
            append_workspace_ranges(
                &mut storage.workspace_ranges,
                M1FullStepWorkspaceRole::Target,
                &target,
                target_ranges,
            );
            (
                lower,
                M1FullStepWorkspaceSubleaseOwners::DraftCatchup {
                    draft_decode: Box::write(
                        storage
                            .draft_owner
                            .take()
                            .expect("preflight draft owner slot"),
                        draft,
                    ),
                    completion: Box::write(
                        storage
                            .completion_owner
                            .take()
                            .expect("preflight completion owner slot"),
                        target,
                    ),
                },
            )
        }
        M1FullStepWorkspaceSubleaseOwners::DraftCatchup { completion, .. } if !storage.entering => {
            let (lower, target, target_ranges): (_, SpeculativeOwner, _) =
                match replace_rollover_workspace(
                    lower,
                    completion,
                    *target_plan,
                    target_bytes,
                    target_descriptor,
                ) {
                    Ok(replaced) => replaced,
                    Err(error) => {
                        return Err(StructuralDraftCatchupFailureV1::new((
                            error,
                            custody,
                            draft,
                            draft_ranges,
                        )))
                    }
                };
            append_workspace_ranges(
                &mut storage.workspace_ranges,
                M1FullStepWorkspaceRole::Draft,
                &draft,
                draft_ranges,
            );
            append_workspace_ranges(
                &mut storage.workspace_ranges,
                M1FullStepWorkspaceRole::Target,
                &target,
                target_ranges,
            );
            (
                lower,
                M1FullStepWorkspaceSubleaseOwners::SpeculativeRound {
                    draft_decode: Box::write(
                        storage
                            .draft_owner
                            .take()
                            .expect("preflight draft owner slot"),
                        draft,
                    ),
                    target_speculative: Box::write(
                        storage
                            .speculative_owner
                            .take()
                            .expect("preflight speculative owner slot"),
                        target,
                    ),
                },
            )
        }
        _ => {
            return Err(StructuralDraftCatchupFailureV1::new((
                "workspace successor",
                lower,
                custody,
                draft,
                draft_ranges,
                target_plan,
                target_bytes,
            )))
        }
    };
    custody.workspace_owners = owners.1;
    Ok((owners.0, custody))
}

#[inline(never)]
fn lower_structural_batch<'a, const N: usize>(
    catalog: ContentBoundM1ProgramCatalogV1<'a>,
    physical: &crate::AddresslessM1PhysicalDispatchRecipeV1,
    images: Box<[crate::M1PhysicalKernargImageV1]>,
    bound: &[M1BoundPhysicalBufferRowV1],
    storage: &mut StructuralTransitionStorageV1,
) -> Result<ServiceFixedBatchV1<'a, N>, Box<LowerBatchFailureV1<'a>>> {
    if physical.rows().len() != N
        || images.len() != N
        || bound.len() != N
        || !storage.packet_inputs.is_empty()
        || storage.packet_inputs.capacity() < N
        || storage.packet_buffers.len() != N
        || !storage
            .packet_buffers
            .iter()
            .zip(bound)
            .all(|(buffers, row)| buffers.capacity() >= row.buffers().len())
    {
        return Err(Box::new(LowerBatchFailureV1 { catalog, images }));
    }
    let mut buffers = core::mem::take(&mut storage.packet_buffers).into_iter();
    for ((image, physical), bound) in images
        .into_vec()
        .into_iter()
        .zip(physical.rows().iter().copied())
        .zip(bound)
    {
        let mut row = buffers.next().expect("checked packet buffer count");
        row.clear();
        row.extend_from_slice(bound.buffers());
        storage.packet_inputs.push(LowerBatchInputV1 {
            physical,
            image,
            buffers: row.into_boxed_slice(),
        });
    }
    let inputs: [LowerBatchInputV1; N] =
        match core::mem::take(&mut storage.packet_inputs).try_into() {
            Ok(inputs) => inputs,
            Err(inputs) => {
                return Err(Box::new(LowerBatchFailureV1 {
                    catalog,
                    images: inputs
                        .into_iter()
                        .map(|input| input.image)
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                }))
            }
        };
    let packets = inputs.map(|input| {
        ServiceFixedDispatchPacketV1::new(
            input.physical.program_index(),
            input.physical.geometry(),
            input.physical.dynamic_group_segment_bytes(),
            input.image.into_bytes(),
            input.buffers,
        )
    });
    Ok(ServiceFixedBatchV1::new(catalog.into_programs(), packets))
}

fn rollover_structural_batch<'a, const N: usize>(
    lower: ServiceQueueUnboundSessionV1,
    batch: ServiceFixedBatchV1<'a, N>,
    ring_bytes: u32,
    prior_generation: u64,
) -> Result<(ServiceQueueSessionV1<N>, M1QueueRolloverObservationV1), Box<dyn fmt::Debug + 'a>> {
    let rollover = match lower.rollover(ring_bytes, batch) {
        Ok(rollover) => rollover,
        Err(error) => return Err(Box::new(error)),
    };
    let observation = M1QueueRolloverObservationV1::new(
        rollover.previous_queue_destroyed(),
        rollover.previous_dispatch_generation(),
        rollover.replacement_queue_observation(),
        rollover.replacement_dispatch_generation(),
    );
    if prior_generation == 0
        || observation.previous_dispatch_generation() != prior_generation
        || prior_generation.checked_add(1) != Some(observation.replacement_dispatch_generation())
    {
        return Err(Box::new((
            "structural rollover generation",
            rollover,
            observation,
        )));
    }
    Ok((rollover.into_queue(), observation))
}

#[derive(Debug)]
pub(crate) struct StructuralDraftCatchupPublishedV1 {
    published: crate::physical_queue_lifecycle::M1StructuralDraftCatchupPublishedV1,
    carry: M1RearmContinuationCustodyV1,
    saved: StructuralSavedBindingsV1,
    entered: M1QueueRolloverObservationV1,
}

impl StructuralDraftCatchupPublishedV1 {
    pub(crate) const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        self.published.scheduled_dispatch()
    }

    pub(crate) fn read_and_settle<const C: usize>(
        self,
        engine: &mut Engine<C>,
        runner: &LogicalRunnerDeclaration,
        pending: M1StructuralDraftCatchupPendingV1,
        timeout_ms: u32,
        storage: crate::physical_queue_lifecycle::M1StructuralDraftCatchupReadbackHostStorageV1,
    ) -> Result<StructuralDraftCatchupReleasedV1, StructuralDraftCatchupFailureV1> {
        if self.carry.selected.len() != 1
            || self.carry.selected[0].projection().request != pending.request()
            || self.saved.parent != pending.parent()
            || self.saved.request != pending.request()
            || self.saved.coordinator != pending.coordinator_identity()
            || self.entered.previous_dispatch_generation() != pending.prior_dispatch_generation()
            || pending.prior_dispatch_generation().checked_add(1)
                != Some(self.entered.replacement_dispatch_generation())
        {
            engine.quarantine_m1_queue_rearm_failure();
            return Err(StructuralDraftCatchupFailureV1::new((
                "maintenance settlement custody",
                self,
                pending,
                storage,
            )));
        }
        let Self {
            published,
            mut carry,
            saved,
            entered,
        } = self;
        let readback = match published.read_and_check(
            entered.replacement_dispatch_generation(),
            timeout_ms,
            storage,
        ) {
            Ok(readback) => readback,
            Err(error) => {
                engine.quarantine_m1_queue_rearm_failure();
                return Err(StructuralDraftCatchupFailureV1::new((
                    error, carry, saved, entered, pending,
                )));
            }
        };
        let cache = carry.selected.pop().expect("checked singleton cache");
        let settled =
            match crate::m1_serving_physical_operations::settle_structural_draft_catchup_v1(
                engine, runner, pending, readback, cache,
            ) {
                Ok(settled) => settled,
                Err(error) => {
                    return Err(StructuralDraftCatchupFailureV1::new((
                        error, carry, saved, entered,
                    )))
                }
            };
        carry.selected.push(settled.cache);
        carry.previous_epoch = settled.completed.completion_epoch();
        Ok(StructuralDraftCatchupReleasedV1 {
            lower: settled.lower,
            custody: settled.custody,
            carry,
            restore: StructuralDraftCatchupRestoreCustodyV1 {
                completed: settled.completed,
                initialized: settled.initialized,
                saved,
                entered,
            },
        })
    }
}

#[derive(Debug)]
pub(crate) struct StructuralDraftCatchupReleasedV1 {
    lower: fe2o3_service_host::ServiceRecycledQueueSessionV1<425>,
    custody: M1PhysicalQueueBatchCustodyV1,
    carry: M1RearmContinuationCustodyV1,
    restore: StructuralDraftCatchupRestoreCustodyV1,
}

impl StructuralDraftCatchupReleasedV1 {
    pub(crate) const fn completed(
        &self,
    ) -> &crate::m1_serving_physical_operations::M1StructuralDraftCatchupCompletedV1 {
        &self.restore.completed
    }

    pub(crate) fn close<const C: usize>(
        self,
        engine: &mut Engine<C>,
    ) -> StructuralDraftCatchupFailureV1 {
        engine.quarantine_m1_queue_rearm_failure();
        StructuralDraftCatchupFailureV1::new((
            self.lower.destroy_and_release(),
            self.custody,
            self.carry,
            self.restore,
        ))
    }
}

#[derive(Debug)]
pub(crate) struct StructuralRestoreScratchV1 {
    parent: Qwen3PlanSelection,
    draft_inputs:
        Option<crate::authenticated_resident_session::M1AuthenticatedResidentRoleInputStorageV1>,
    target_inputs:
        Option<crate::authenticated_resident_session::M1AuthenticatedResidentRoleInputStorageV1>,
    draft_pages: Vec<crate::DeviceKvPageLease>,
    target_pages: Vec<crate::DeviceKvPageLease>,
    draft_reservations: Vec<crate::PendingDeviceKvStepWrite>,
    target_reservations: Vec<crate::PendingDeviceKvStepWrite>,
    draft_table: Option<crate::kv_workspace_authority::M1KvWorkspaceTableHostStorageV1>,
    target_table: Option<crate::kv_workspace_authority::M1KvWorkspaceTableHostStorageV1>,
    images: Option<crate::m1_prepublication::M1FullStepWorkspaceImageHostStorageV1>,
}

impl StructuralRestoreScratchV1 {
    pub(crate) fn try_new(
        parent: Qwen3PlanSelection,
        plans: &M1FullStepWorkspacePlans,
    ) -> Option<Self> {
        let M1FullStepWorkspacePlans::SpeculativeRound {
            draft_decode,
            target_speculative,
        } = plans
        else {
            return None;
        };
        let width = parent
            .bucket
            .dimensions(parent.role, parent.mode)?
            .active_tokens;
        let draft_selection = Qwen3PlanSelection {
            role: ferric_spec::Qwen3ModelRole::Draft06B,
            mode: Qwen3ExecutionMode::Decode,
            bucket: ferric_spec::Qwen3PlanBucket::DecodeS1C8192,
        };
        if target_speculative.selection() != parent
            || draft_decode.selection() != draft_selection
            || !matches!(width, 5 | 9 | 17)
        {
            return None;
        }
        let mut draft_pages = Vec::new();
        draft_pages.try_reserve_exact(1).ok()?;
        let mut target_pages = Vec::new();
        target_pages
            .try_reserve_exact(width.div_ceil(M1_KV_PAGE_TOKENS) as usize)
            .ok()?;
        let mut draft_reservations = Vec::new();
        draft_reservations.try_reserve_exact(1).ok()?;
        let mut target_reservations = Vec::new();
        target_reservations.try_reserve_exact(1).ok()?;
        Some(Self {
            parent,
            draft_inputs: Some(crate::authenticated_resident_session::M1AuthenticatedResidentRoleInputStorageV1::try_new(1)?),
            target_inputs: Some(crate::authenticated_resident_session::M1AuthenticatedResidentRoleInputStorageV1::try_new(width as usize)?),
            draft_pages, target_pages, draft_reservations, target_reservations,
            draft_table: Some(crate::kv_workspace_authority::M1KvWorkspaceTableHostStorageV1::try_new(1)?),
            target_table: Some(crate::kv_workspace_authority::M1KvWorkspaceTableHostStorageV1::try_new(1)?),
            images: Some(crate::m1_prepublication::M1FullStepWorkspaceImageHostStorageV1::try_new(plans)?),
        })
    }
}

#[derive(Debug)]
pub(crate) struct StructuralRestorePreparedV1 {
    lower: ServiceQueueUnboundSessionV1,
    custody: M1PhysicalQueueBatchCustodyV1,
    prepared: M1PreparedScheduledWorkspaceImagesV1,
    carry: M1RearmContinuationCustodyV1,
    restore: StructuralDraftCatchupRestoreCustodyV1,
}

impl StructuralDraftCatchupReleasedV1 {
    pub(crate) fn prepare_restore<const C: usize>(
        self,
        engine: &mut Engine<C>,
        runner: &LogicalRunnerDeclaration,
        plans: M1FullStepWorkspacePlans,
        scratch: &mut StructuralRestoreScratchV1,
        deadline_expired: &mut impl FnMut() -> bool,
    ) -> Result<StructuralRestorePreparedV1, StructuralDraftCatchupFailureV1> {
        let pending = self.completed().pending();
        let parent = pending.parent();
        let request = pending.request();
        let committed = pending.target_committed();
        let anchor = pending.next_anchor();
        let width = parent
            .bucket
            .dimensions(parent.role, parent.mode)
            .map_or(0, |d| d.active_tokens);
        let epoch = pending.epoch().value().checked_add(1);
        let exact_cache = self.carry.selected.len() == 1
            && self.carry.selected[0].projection().request == request
            && self.carry.selected[0].projection().target.committed_tokens == committed
            && self.carry.selected[0].projection().target.resident_tokens == committed
            && self.carry.selected[0].projection().draft.committed_tokens == committed
            && self.carry.selected[0].projection().draft.resident_tokens == committed
            && !self.carry.selected[0].projection().target_write_pending
            && !self.carry.selected[0].projection().draft_write_pending;
        if deadline_expired()
            || engine.is_faulted()
            || !exact_cache
            || epoch.is_none()
            || engine.state(request) != Some(RequestState::Ready)
            || !matches!(width, 5 | 9 | 17)
            || !committed
                .checked_add(width)
                .is_some_and(|end| end <= ferric_spec::M1_MAX_CONTEXT_TOKENS)
            || scratch.parent != parent
            || plans.target().selection() != parent
            || scratch.draft_inputs.is_none()
            || scratch.target_inputs.is_none()
            || scratch.draft_table.is_none()
            || scratch.target_table.is_none()
            || scratch.images.is_none()
            || !scratch.draft_pages.is_empty()
            || scratch.draft_pages.capacity() < 1
            || !scratch.target_pages.is_empty()
            || scratch.target_pages.capacity() < width.div_ceil(M1_KV_PAGE_TOKENS) as usize
            || !scratch.draft_reservations.is_empty()
            || scratch.draft_reservations.capacity() < 1
            || !scratch.target_reservations.is_empty()
            || scratch.target_reservations.capacity() < 1
        {
            return Err(StructuralDraftCatchupFailureV1::new((
                "restore preparation preflight",
                self,
                plans,
            )));
        }
        let epoch = CompletionEpoch::new(epoch.expect("checked successor epoch"));
        let draft_selection = Qwen3PlanSelection {
            role: ferric_spec::Qwen3ModelRole::Draft06B,
            mode: Qwen3ExecutionMode::Decode,
            bucket: ferric_spec::Qwen3PlanBucket::DecodeS1C8192,
        };
        let draft_inputs = scratch
            .draft_inputs
            .take()
            .expect("checked draft input storage")
            .fill(runner, draft_selection, request, epoch, anchor, committed);
        let target_inputs = scratch
            .target_inputs
            .take()
            .expect("checked target input storage")
            .fill(runner, parent, request, epoch, anchor, committed);
        let (Some(draft_inputs), Some(target_inputs)) = (draft_inputs, target_inputs) else {
            return Err(StructuralDraftCatchupFailureV1::new((
                "restore input binding",
                self,
                plans,
            )));
        };
        if deadline_expired() {
            return Err(StructuralDraftCatchupFailureV1::new((
                "restore deadline before mutation",
                self,
                plans,
                draft_inputs,
                target_inputs,
            )));
        }
        if let Err(error) = engine.append_tentative(request, width) {
            engine.quarantine_m1_queue_rearm_failure();
            return Err(StructuralDraftCatchupFailureV1::new((
                error,
                self,
                plans,
                draft_inputs,
                target_inputs,
            )));
        }
        let Self {
            lower,
            mut custody,
            mut carry,
            restore,
        } = self;
        let lower = match lower.detach() {
            Ok(lower) => lower,
            Err(error) => {
                engine.quarantine_m1_queue_rearm_failure();
                return Err(StructuralDraftCatchupFailureV1::new((
                    error,
                    custody,
                    carry,
                    restore,
                    plans,
                    draft_inputs,
                    target_inputs,
                )));
            }
        };
        let scheduled = match engine.dispatch_m1_exact_ready(epoch, &[request]) {
            Ok(scheduled) => scheduled,
            Err(error) => {
                engine.quarantine_m1_queue_rearm_failure();
                return Err(StructuralDraftCatchupFailureV1::new((
                    error,
                    lower,
                    custody,
                    carry,
                    restore,
                    plans,
                    draft_inputs,
                    target_inputs,
                )));
            }
        };
        for (role, end, pages) in [
            (
                ferric_spec::Qwen3ModelRole::Draft06B,
                committed + width - 1,
                &mut scratch.draft_pages,
            ),
            (
                ferric_spec::Qwen3ModelRole::Target8B,
                committed + width,
                &mut scratch.target_pages,
            ),
        ] {
            for index in committed.div_ceil(M1_KV_PAGE_TOKENS)..end.div_ceil(M1_KV_PAGE_TOKENS) {
                let page = match custody
                    .partitioned_memory_mut()
                    .lease_page_for_detached_queue(&lower, request, role, index)
                {
                    Ok(page) => page,
                    Err(error) => {
                        engine.quarantine_m1_queue_rearm_failure();
                        return Err(StructuralDraftCatchupFailureV1::new((
                            error,
                            lower,
                            custody,
                            carry,
                            restore,
                            plans,
                            scheduled,
                            draft_inputs,
                            target_inputs,
                        )));
                    }
                };
                pages.push(page);
            }
        }
        let draft = match carry.selected[0].reserve_speculative_draft_round_write(
            request,
            parent,
            draft_selection,
            committed,
            epoch,
            core::mem::take(&mut scratch.draft_pages),
        ) {
            Ok(draft) => draft,
            Err(error) => {
                engine.quarantine_m1_queue_rearm_failure();
                return Err(StructuralDraftCatchupFailureV1::new((
                    error,
                    lower,
                    custody,
                    carry,
                    restore,
                    plans,
                    scheduled,
                    draft_inputs,
                    target_inputs,
                )));
            }
        };
        scratch.draft_reservations.push(draft);
        let target = match carry.selected[0].reserve_step_write(
            request,
            ferric_spec::Qwen3ModelRole::Target8B,
            committed,
            width,
            epoch,
            core::mem::take(&mut scratch.target_pages),
        ) {
            Ok(target) => target,
            Err(error) => {
                engine.quarantine_m1_queue_rearm_failure();
                return Err(StructuralDraftCatchupFailureV1::new((
                    error,
                    lower,
                    custody,
                    carry,
                    restore,
                    plans,
                    scheduled,
                    draft_inputs,
                    target_inputs,
                )));
            }
        };
        scratch.target_reservations.push(target);
        let target = match crate::kv_workspace_authority::bind_m1_kv_workspace_table_with_storage_v1(
            target_inputs,
            core::mem::take(&mut scratch.target_reservations),
            scratch
                .target_table
                .take()
                .expect("checked target table storage"),
        ) {
            Ok(target) => target,
            Err(error) => {
                engine.quarantine_m1_queue_rearm_failure();
                return Err(StructuralDraftCatchupFailureV1::new((
                    error,
                    lower,
                    custody,
                    carry,
                    restore,
                    plans,
                    scheduled,
                    draft_inputs,
                )));
            }
        };
        let draft = match crate::kv_workspace_authority::bind_m1_speculative_draft_kv_round_workspace_table_with_storage_v1(parent, draft_inputs, core::mem::take(&mut scratch.draft_reservations), scratch.draft_table.take().expect("checked draft table storage")) {
            Ok(draft) => draft,
            Err(error) => {
                engine.quarantine_m1_queue_rearm_failure();
                return Err(StructuralDraftCatchupFailureV1::new((error, lower, custody, carry, restore, plans, scheduled, target)));
            }
        };
        let tables = M1FullStepKvWorkspaceTablesV1::SpeculativeRound {
            draft_decode: draft,
            target_speculative: target,
        };
        match crate::m1_prepublication::prepare_m1_scheduled_workspace_images_with_storage_v1(
            scheduled,
            runner,
            plans,
            tables,
            scratch
                .images
                .take()
                .expect("checked workspace image storage"),
        ) {
            Ok(prepared) => Ok(StructuralRestorePreparedV1 {
                lower,
                custody,
                prepared,
                carry,
                restore,
            }),
            Err(error) => {
                engine.quarantine_m1_queue_rearm_failure();
                Err(StructuralDraftCatchupFailureV1::new((
                    error, lower, custody, carry, restore,
                )))
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn submit_structural_restore_case<'a, const N: usize>(
    transition: StructuralTransitionPartsV1,
    catalog: ContentBoundM1ProgramCatalogV1<'a>,
    ring_bytes: u32,
    predecessor_generation: u64,
    slot: Box<core::mem::MaybeUninit<RestorePhase<N>>>,
    storage: &mut StructuralTransitionStorageV1,
    wrap: impl FnOnce(Box<RestorePhase<N>>) -> M1PhysicalQueueSessionV1,
) -> Result<
    (
        M1PhysicalPublishedQueueSessionV1,
        StructuralSavedBindingsV1,
        M1QueueRolloverObservationV1,
    ),
    M1LongLivedQueueRearmSubmissionFailureV1<'a>,
> {
    let StructuralTransitionPartsV1 {
        lower,
        custody,
        step,
        images,
        saved,
    } = transition;
    let batch = match lower_structural_batch::<N>(
        catalog,
        &custody.physical_recipe,
        images,
        &custody.bound_rows,
        storage,
    ) {
        Ok(batch) => batch,
        Err(error) => {
            return Err(submission_failure(
                M1LongLivedQueueRearmSubmissionPhaseV1::FixedBatchRebuild,
                (
                    error.catalog,
                    error.images,
                    lower,
                    custody,
                    step,
                    saved,
                    slot,
                ),
            ))
        }
    };
    let (lower, observation) =
        match rollover_structural_batch(lower, batch, ring_bytes, predecessor_generation) {
            Ok(rolled) => rolled,
            Err(error) => {
                return Err(submission_failure(
                    M1LongLivedQueueRearmSubmissionPhaseV1::QueueRollover,
                    (error, custody, step, saved, slot),
                ))
            }
        };
    let case = Box::write(
        slot,
        RestorePhase::from_queue_rearm(
            lower,
            M1PhysicalQueueBatchCustodyV1::from_rearm_parts(custody),
            step,
        ),
    );
    match wrap(case).submit() {
        Ok(queue) => Ok((queue, saved, observation)),
        Err(error) => Err(submission_failure(
            M1LongLivedQueueRearmSubmissionPhaseV1::QueueSubmit,
            (error, saved, observation),
        )),
    }
}

pub(crate) fn submit_structural_restore_v1<'a>(
    prepared: StructuralRestorePreparedV1,
    recipe: AddresslessM1PhysicalBufferRecipeV1,
    catalog: ContentBoundM1ProgramCatalogV1<'a>,
    ring_bytes: u32,
    storage: &mut StructuralTransitionStorageV1,
) -> Result<M1RearmedPublishedQueueV1, M1LongLivedQueueRearmSubmissionFailureV1<'a>> {
    if storage.entering
        || !storage.has_restore_phase()
        || prepared.custody.catalog_id() != catalog.catalog_id()
        || prepared.carry.structural_maintenance.is_some()
        || prepared.carry.previous_epoch != prepared.restore.completed.completion_epoch()
    {
        return Err(submission_failure(
            M1LongLivedQueueRearmSubmissionPhaseV1::Preflight,
            (prepared, recipe, catalog),
        ));
    }
    let StructuralRestorePreparedV1 {
        lower,
        custody,
        prepared,
        mut carry,
        restore,
    } = prepared;
    let queue_observation = lower.observation();
    let device = custody.device();
    let StructuralDraftCatchupRestoreCustodyV1 {
        completed,
        initialized,
        saved,
        entered,
    } = restore;
    let parent = completed.pending().parent();
    let transition = match prepare_structural_queue_transition(
        lower,
        custody,
        prepared,
        recipe,
        completed.pending(),
        Some(saved),
        storage,
    ) {
        Ok(transition) => transition,
        Err(error) => {
            return Err(submission_failure(
                M1LongLivedQueueRearmSubmissionPhaseV1::WorkspaceRangeRebinding,
                (error, carry, completed, initialized, entered, catalog),
            ))
        }
    };
    let generation = completed.dispatch_generation();
    let result = match parent.bucket {
        ferric_spec::Qwen3PlanBucket::SpeculativeS1K4C8192 => {
            let phase = storage.phase_k4.take().expect("preflight restore K4 phase");
            submit_structural_restore_case(
                transition,
                catalog,
                ring_bytes,
                generation,
                phase,
                storage,
                M1PhysicalQueueSessionV1::SpeculativeK4,
            )
        }
        ferric_spec::Qwen3PlanBucket::SpeculativeS1K8C8192 => {
            let phase = storage.phase_k8.take().expect("preflight restore K8 phase");
            submit_structural_restore_case(
                transition,
                catalog,
                ring_bytes,
                generation,
                phase,
                storage,
                M1PhysicalQueueSessionV1::SpeculativeK8,
            )
        }
        ferric_spec::Qwen3PlanBucket::SpeculativeS1K16C8192 => {
            let phase = storage
                .phase_k16
                .take()
                .expect("preflight restore K16 phase");
            submit_structural_restore_case(
                transition,
                catalog,
                ring_bytes,
                generation,
                phase,
                storage,
                M1PhysicalQueueSessionV1::SpeculativeK16,
            )
        }
        _ => unreachable!("preflight exact singleton speculative parent"),
    };
    let (queue, saved, rollover) = match result {
        Ok(result) => result,
        Err(error) => {
            return Err(submission_failure(
                M1LongLivedQueueRearmSubmissionPhaseV1::QueueSubmit,
                (error, carry, completed, initialized, entered),
            ))
        }
    };
    carry.rollover = Some(rollover);
    carry.structural_maintenance = Some(StructuralDraftCatchupRestoreCustodyV1 {
        completed,
        initialized,
        saved,
        entered,
    });
    Ok(M1RearmedPublishedQueueV1 {
        queue,
        carry,
        queue_observation,
        device,
    })
}

pub(crate) fn submit_structural_draft_catchup_v1<'a>(
    prepared: M1PreparedLongLivedQueueRearmV1,
    recipe: AddresslessM1PhysicalBufferRecipeV1,
    catalog: ContentBoundM1ProgramCatalogV1<'a>,
    authorized: &M1StructuralDraftCatchupPendingV1,
    ring_bytes: u32,
    storage: &mut StructuralTransitionStorageV1,
) -> Result<StructuralDraftCatchupPublishedV1, M1LongLivedQueueRearmSubmissionFailureV1<'a>> {
    let M1PreparedLongLivedQueueRearmV1 {
        prepared,
        remainder,
    } = prepared;
    if !storage.entering
        || remainder.queue.custody().catalog_id() != catalog.catalog_id()
        || remainder.queue.custody().selection() != authorized.parent()
        || remainder.selected.len() != 1
        || remainder.selected[0].projection().request != authorized.request()
        || remainder.prior_checked.epoch() != authorized.prior_epoch()
        || remainder.prior_checked.dispatch_generation() != authorized.prior_dispatch_generation()
    {
        return Err(submission_failure(
            M1LongLivedQueueRearmSubmissionPhaseV1::Preflight,
            (prepared, remainder, recipe, catalog),
        ));
    }
    let ScheduledRemainderV1 {
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
    let (_, lower, custody) = queue.into_rearm_parts();
    let carry = M1RearmContinuationCustodyV1 {
        selected,
        parked,
        terminal,
        previous_epoch: authorized.prior_epoch(),
        prior_checked,
        logical_accepted_counts,
        externally_published_counts,
        release_counts,
        completed_members,
        total_released,
        history,
        rollover: None,
        structural_maintenance: None,
    };
    let transition = match prepare_structural_queue_transition(
        lower, custody, prepared, recipe, authorized, None, storage,
    ) {
        Ok(transition) => transition,
        Err(error) => {
            return Err(submission_failure(
                M1LongLivedQueueRearmSubmissionPhaseV1::WorkspaceRangeRebinding,
                (error, carry, catalog),
            ))
        }
    };
    let StructuralTransitionPartsV1 {
        lower,
        custody,
        step,
        images,
        saved,
    } = transition;
    let batch = match lower_structural_batch::<425>(
        catalog,
        &custody.physical_recipe,
        images,
        &custody.bound_rows,
        storage,
    ) {
        Ok(batch) => batch,
        Err(error) => {
            return Err(submission_failure(
                M1LongLivedQueueRearmSubmissionPhaseV1::FixedBatchRebuild,
                (
                    error.catalog,
                    error.images,
                    lower,
                    custody,
                    step,
                    carry,
                    saved,
                ),
            ))
        }
    };
    let (lower, entered) = match rollover_structural_batch(
        lower,
        batch,
        ring_bytes,
        authorized.prior_dispatch_generation(),
    ) {
        Ok(rolled) => rolled,
        Err(error) => {
            return Err(submission_failure(
                M1LongLivedQueueRearmSubmissionPhaseV1::QueueRollover,
                (error, custody, step, carry, saved),
            ))
        }
    };
    let custody = M1PhysicalQueueBatchCustodyV1::from_rearm_parts(custody);
    let published = match crate::physical_queue_lifecycle::publish_m1_structural_draft_catchup_v1(
        lower, custody, step,
    ) {
        Ok(published) => published,
        Err(error) => {
            return Err(submission_failure(
                M1LongLivedQueueRearmSubmissionPhaseV1::QueueSubmit,
                (error, carry, saved, entered),
            ))
        }
    };
    Ok(StructuralDraftCatchupPublishedV1 {
        published,
        carry,
        saved,
        entered,
    })
}

pub(crate) fn prepare_structural_draft_catchup_v1(
    mut scheduled: M1ScheduledLongLivedQueueRearmV1,
    authorized: &M1StructuralDraftCatchupPendingV1,
    runner: &LogicalRunnerDeclaration,
    plans: M1FullStepWorkspacePlans,
    scratch: &mut StructuralDraftCatchupScratchV1,
) -> Result<M1PreparedLongLivedQueueRearmV1, StructuralDraftCatchupFailureV1> {
    if scratch.parent != authorized.parent()
        || scheduled.selected.len() != 1
        || !scheduled.parked.is_empty()
        || scheduled.selected[0].projection().request != authorized.request()
        || scheduled.scheduled.member_count() != 1
        || scheduled.scheduled.member(0) != Some(authorized.request())
        || scheduled.scheduled.epoch() != authorized.epoch()
        || scheduled.prior_checked.selection() != authorized.parent()
        || scheduled.prior_checked.epoch() != authorized.prior_epoch()
        || scheduled.prior_checked.dispatch_generation() != authorized.prior_dispatch_generation()
        || plans.kind() != M1FullStepWorkspaceInputKind::DraftCatchup
        || scratch.page_leases.capacity() < 1
        || !scratch.page_leases.is_empty()
        || scratch.reservations.capacity() < 1
        || !scratch.reservations.is_empty()
        || scratch.draft_inputs.is_none()
        || scratch.completion_inputs.is_none()
        || scratch.draft_table.is_none()
        || scratch.completion_page_indices.is_none()
        || scratch.workspace_images.is_none()
    {
        return Err(StructuralDraftCatchupFailureV1::new((
            "maintenance preparation",
            scheduled,
            plans,
        )));
    }
    let completion_selection =
        crate::M1StepDispatchIntent::DraftCatchup(authorized.parent()).completion_selection();
    let draft_selection = Qwen3PlanSelection {
        role: ferric_spec::Qwen3ModelRole::Draft06B,
        ..completion_selection
    };
    let draft_inputs = scratch
        .draft_inputs
        .take()
        .expect("checked draft input storage")
        .fill(
            runner,
            draft_selection,
            authorized.request(),
            authorized.epoch(),
            authorized.token(),
            authorized.draft_committed(),
        );
    let completion_inputs = scratch
        .completion_inputs
        .take()
        .expect("checked completion input storage")
        .fill(
            runner,
            completion_selection,
            authorized.request(),
            authorized.epoch(),
            authorized.token(),
            authorized.draft_committed(),
        );
    let (Some(draft_inputs), Some(completion_inputs)) = (draft_inputs, completion_inputs) else {
        return Err(StructuralDraftCatchupFailureV1::new((
            "maintenance input binding",
            scheduled,
            plans,
        )));
    };
    if authorized
        .draft_committed()
        .is_multiple_of(M1_KV_PAGE_TOKENS)
    {
        let page = match scheduled
            .queue
            .lease_structural_draft_catchup_page(authorized)
        {
            Ok(page) => page,
            Err(error) => {
                return Err(StructuralDraftCatchupFailureV1::new((
                    error,
                    scheduled,
                    plans,
                    draft_inputs,
                    completion_inputs,
                )))
            }
        };
        scratch.page_leases.push(page);
    }
    let reservation = match scheduled.selected[0].reserve_structural_draft_catchup_write(
        authorized,
        core::mem::take(&mut scratch.page_leases),
        &mut scratch.write,
    ) {
        Ok(reservation) => reservation,
        Err(error) => {
            return Err(StructuralDraftCatchupFailureV1::new((
                error,
                scheduled,
                plans,
                draft_inputs,
                completion_inputs,
            )))
        }
    };
    scratch.reservations.push(reservation);
    let draft = match crate::kv_workspace_authority::bind_m1_structural_draft_catchup_kv_workspace_table_with_storage_v1(
        draft_inputs, core::mem::take(&mut scratch.reservations),
        scratch.draft_table.take().expect("checked draft table storage"), authorized,
    ) {
        Ok(draft) => draft,
        Err(error) => return Err(StructuralDraftCatchupFailureV1::new((error, scheduled, plans, completion_inputs))),
    };
    let tables = M1FullStepKvWorkspaceTablesV1::DraftCatchup {
        draft,
        completion: completion_inputs,
        completion_page_indices: scratch
            .completion_page_indices
            .take()
            .expect("checked completion page indices"),
        target_allocation_id: scheduled
            .queue
            .custody()
            .partitioned_memory()
            .allocation_id(ferric_spec::Qwen3ModelRole::Target8B),
    };
    let M1ScheduledLongLivedQueueRearmV1 {
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
    let remainder = ScheduledRemainderV1 {
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
    match crate::m1_prepublication::prepare_m1_scheduled_workspace_images_with_storage_v1(
        scheduled,
        runner,
        plans,
        tables,
        scratch
            .workspace_images
            .take()
            .expect("checked workspace image storage"),
    ) {
        Ok(prepared) => Ok(M1PreparedLongLivedQueueRearmV1 {
            prepared,
            remainder,
        }),
        Err(error) => Err(StructuralDraftCatchupFailureV1::new((error, remainder))),
    }
}

pub(crate) fn prepare_structural_speculative_rearm_v1<const C: usize>(
    engine: &mut Engine<C>,
    mut scheduled: M1ScheduledLongLivedQueueRearmV1,
    runner: &LogicalRunnerDeclaration,
    plans: M1FullStepWorkspacePlans,
    anchor: ferric_spec::TokenId,
    committed: u32,
    scratch: &mut StructuralRestoreScratchV1,
) -> Result<M1PreparedLongLivedQueueRearmV1, StructuralDraftCatchupFailureV1> {
    let parent = scratch.parent;
    let width = parent
        .bucket
        .dimensions(parent.role, parent.mode)
        .map_or(0, |d| d.active_tokens);
    let Some(request) = scheduled.scheduled.member(0) else {
        return Err(StructuralDraftCatchupFailureV1::new((
            "empty structural rearm",
            scheduled,
            plans,
        )));
    };
    if scheduled.selected.len() != 1
        || !scheduled.parked.is_empty()
        || scheduled.scheduled.member_count() != 1
        || scheduled.selected[0].projection().request != request
        || scheduled.selected[0].projection().draft.committed_tokens != committed
        || scheduled.selected[0].projection().target.committed_tokens != committed
        || plans.target().selection() != parent
        || scheduled.prior_checked.selection() != parent
        || !matches!(width, 5 | 9 | 17)
        || !committed
            .checked_add(width)
            .is_some_and(|end| end <= ferric_spec::M1_MAX_CONTEXT_TOKENS)
        || scratch.draft_inputs.is_none()
        || scratch.target_inputs.is_none()
        || !scratch.draft_pages.is_empty()
        || scratch.draft_pages.capacity() < 1
        || !scratch.target_pages.is_empty()
        || scratch.target_pages.capacity() < width.div_ceil(M1_KV_PAGE_TOKENS) as usize
    {
        engine.quarantine_m1_queue_rearm_failure();
        return Err(StructuralDraftCatchupFailureV1::new((
            "aligned structural rearm preflight",
            scheduled,
            plans,
        )));
    }
    let epoch = scheduled.scheduled.epoch();
    let draft_selection = Qwen3PlanSelection {
        role: ferric_spec::Qwen3ModelRole::Draft06B,
        mode: Qwen3ExecutionMode::Decode,
        bucket: ferric_spec::Qwen3PlanBucket::DecodeS1C8192,
    };
    let draft_inputs = scratch
        .draft_inputs
        .take()
        .expect("checked draft input storage")
        .fill(runner, draft_selection, request, epoch, anchor, committed);
    let target_inputs = scratch
        .target_inputs
        .take()
        .expect("checked target input storage")
        .fill(runner, parent, request, epoch, anchor, committed);
    let (Some(draft_inputs), Some(target_inputs)) = (draft_inputs, target_inputs) else {
        engine.quarantine_m1_queue_rearm_failure();
        return Err(StructuralDraftCatchupFailureV1::new((
            "structural rearm input binding",
            scheduled,
            plans,
        )));
    };
    for (role, end, pages) in [
        (
            ferric_spec::Qwen3ModelRole::Draft06B,
            committed + width - 1,
            &mut scratch.draft_pages,
        ),
        (
            ferric_spec::Qwen3ModelRole::Target8B,
            committed + width,
            &mut scratch.target_pages,
        ),
    ] {
        for index in committed.div_ceil(M1_KV_PAGE_TOKENS)..end.div_ceil(M1_KV_PAGE_TOKENS) {
            match scheduled
                .queue
                .lease_structural_speculative_page(parent, request, role, index)
            {
                Ok(page) => pages.push(page),
                Err(error) => {
                    engine.quarantine_m1_queue_rearm_failure();
                    return Err(StructuralDraftCatchupFailureV1::new((
                        error,
                        scheduled,
                        plans,
                        draft_inputs,
                        target_inputs,
                    )));
                }
            }
        }
    }
    let inputs = M1LongLivedQueueRearmKvInputsV1::speculative_round(
        draft_inputs,
        target_inputs,
        vec![core::mem::take(&mut scratch.draft_pages)],
        vec![core::mem::take(&mut scratch.target_pages)],
    );
    let reserved = match reserve_m1_long_lived_queue_rearm_kv_v1(engine, scheduled, inputs) {
        Ok(reserved) => reserved,
        Err(error) => return Err(StructuralDraftCatchupFailureV1::new((error, plans))),
    };
    prepare_m1_long_lived_queue_rearm_v1(engine, reserved, runner, plans)
        .map_err(StructuralDraftCatchupFailureV1::new)
}
